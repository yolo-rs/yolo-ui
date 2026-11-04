import { join, resolve } from "node:path";
import { discoverSubmodules } from "./fleet.ts";

const ROOT_DIR = resolve(import.meta.dir, "..");
const CARGO_TOML_PATH = join(ROOT_DIR, "Cargo.toml");

export interface WorkspaceOptimizationReport {
  workspaceMembersCount: number;
  excludedMembersCount: number;
  workspaceDependenciesCount: number;
  submodulesCount: number;
  profiles: string[];
  independentCargoWorkspaces: string[];
  duplicateWorkspaceDeps: string[];
  inlineDependencyVersions: string[];
  repeatedInlineDependencies: string[];
  compilerOptimizations: {
    devLineTablesOnly: boolean;
    releaseIncremental: boolean;
    releasePanicUnwind: boolean;
    releaseLto: boolean;
    releaseSizeOptimization: boolean;
    releaseStrip: boolean;
    releaseSingleCodegenUnit: boolean;
    wasmSplitLto: boolean;
    cliReleaseOptLevel3: boolean;
  };
}

export async function analyzeWorkspaceOptimization(): Promise<WorkspaceOptimizationReport> {
  const rawToml = await Bun.file(CARGO_TOML_PATH).text();
  const parsed = Bun.TOML.parse(rawToml) as {
    workspace?: {
      members?: string[];
      exclude?: string[];
      dependencies?: Record<string, unknown>;
    };
    profile?: Record<string, Record<string, unknown>>;
  };

  const exclude = parsed.workspace?.exclude ?? [];
  const deps = parsed.workspace?.dependencies ?? {};
  const profiles = Object.keys(parsed.profile ?? {});

  // Check for duplicate keys in [workspace.dependencies] section of raw TOML
  const depSectionMatch = rawToml.split("[workspace.dependencies]")[1];
  const depBody = depSectionMatch
    ? (depSectionMatch.split(/\n\[/)[0] ?? "")
    : "";
  const seenKeys = new Set<string>();
  const duplicates = new Set<string>();

  for (const line of depBody.split(/\r?\n/)) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith("#")) continue;
    const match = /^([a-zA-Z0-9_-]+)\s*=/.exec(trimmed);
    if (match?.[1]) {
      const key = match[1];
      if (seenKeys.has(key)) {
        duplicates.add(key);
      }
      seenKeys.add(key);
    }
  }

  const submodules = await discoverSubmodules();
  const metadata = Bun.spawnSync(
    ["cargo", "metadata", "--no-deps", "--format-version", "1"],
    { cwd: ROOT_DIR, stdout: "pipe", stderr: "pipe" },
  );
  if (metadata.exitCode !== 0) {
    throw new Error(
      `cargo metadata failed: ${new TextDecoder().decode(metadata.stderr)}`,
    );
  }
  const cargoMetadata = JSON.parse(
    new TextDecoder().decode(metadata.stdout),
  ) as {
    workspace_members: string[];
    packages: Array<{ id: string; manifest_path: string }>;
  };
  const workspaceMembers = new Set(cargoMetadata.workspace_members);
  const workspaceManifests = new Set(
    cargoMetadata.packages
      .filter((pkg) => workspaceMembers.has(pkg.id))
      .map((pkg) => resolve(pkg.manifest_path)),
  );
  const independentCargoWorkspaces = new Set<string>();

  // Product crates and applications should inherit third-party versions from the workspace.
  // Keep this inventory separate from upstream submodule manifests, which are
  // tracked as third-party sources and need their own migration plan.
  const inlineVersions = new Set<string>();
  const inlineDependencyOwners = new Map<string, Set<string>>();
  for (const base of ["crates", "packages", "apps"]) {
    const glob = new Bun.Glob("**/Cargo.toml");
    for await (const relativePath of glob.scan({ cwd: join(ROOT_DIR, base) })) {
      const manifestText = await Bun.file(
        join(ROOT_DIR, base, relativePath),
      ).text();
      const manifest = Bun.TOML.parse(manifestText) as Record<string, unknown>;
      const manifestPath = resolve(join(ROOT_DIR, base, relativePath));
      if (!workspaceManifests.has(manifestPath)) {
        const separateWorkspace = manifest.workspace;
        if (separateWorkspace && typeof separateWorkspace === "object") {
          independentCargoWorkspaces.add(`${base}/${relativePath}`);
        }
        continue;
      }
      const tables: unknown[] = [
        manifest.dependencies,
        manifest["dev-dependencies"],
        manifest["build-dependencies"],
      ];
      const targets = manifest.target;
      if (targets && typeof targets === "object") {
        for (const target of Object.values(targets)) {
          if (!target || typeof target !== "object") continue;
          const targetTables = target as Record<string, unknown>;
          tables.push(
            targetTables.dependencies,
            targetTables["dev-dependencies"],
            targetTables["build-dependencies"],
          );
        }
      }

      for (const table of tables) {
        if (!table || typeof table !== "object") continue;
        for (const [name, value] of Object.entries(table)) {
          if (typeof value === "string") {
            inlineVersions.add(
              `${name} = "${value}" (${base}/${relativePath})`,
            );
            const key = `${name} = "${value}"`;
            const owners = inlineDependencyOwners.get(key) ?? new Set<string>();
            owners.add(`${base}/${relativePath}`);
            inlineDependencyOwners.set(key, owners);
            continue;
          }
          if (!value || typeof value !== "object") continue;
          const dependency = value as Record<string, unknown>;
          if (
            typeof dependency.version === "string" &&
            dependency.workspace !== true &&
            typeof dependency.path !== "string" &&
            typeof dependency.git !== "string"
          ) {
            inlineVersions.add(
              `${name} = "${dependency.version}" (${base}/${relativePath})`,
            );
            const key = `${name} = "${dependency.version}"`;
            const owners = inlineDependencyOwners.get(key) ?? new Set<string>();
            owners.add(`${base}/${relativePath}`);
            inlineDependencyOwners.set(key, owners);
          }
        }
      }
    }
  }

  const devProfile = parsed.profile?.["dev"] ?? {};
  const releaseProfile = parsed.profile?.["release"] ?? {};
  const wasmSplitProfile = parsed.profile?.["wasm-split-release"] ?? {};
  const cliReleaseProfile = parsed.profile?.["cli-release-dev"] ?? {};

  return {
    workspaceMembersCount: workspaceMembers.size,
    excludedMembersCount: exclude.length,
    workspaceDependenciesCount: Object.keys(deps).length,
    submodulesCount: submodules.length,
    profiles,
    independentCargoWorkspaces: Array.from(independentCargoWorkspaces).sort(),
    duplicateWorkspaceDeps: Array.from(duplicates),
    inlineDependencyVersions: Array.from(inlineVersions).sort(),
    repeatedInlineDependencies: Array.from(inlineDependencyOwners)
      .filter(([, owners]) => owners.size > 1)
      .map(([dependency, owners]) => `${dependency} (${owners.size} manifests)`)
      .sort((a, b) => {
        const count = (value: string) =>
          Number(/\((\d+) manifests\)$/.exec(value)?.[1] ?? 0);
        return count(b) - count(a) || a.localeCompare(b);
      }),
    compilerOptimizations: {
      devLineTablesOnly: devProfile["debug"] === "line-tables-only",
      releaseIncremental: releaseProfile["incremental"] === true,
      releasePanicUnwind: releaseProfile["panic"] === "unwind",
      releaseLto: releaseProfile["lto"] === true,
      releaseSizeOptimization: releaseProfile["opt-level"] === "s",
      releaseStrip: releaseProfile["strip"] === true,
      releaseSingleCodegenUnit: releaseProfile["codegen-units"] === 1,
      wasmSplitLto: wasmSplitProfile["lto"] === true,
      cliReleaseOptLevel3: cliReleaseProfile["opt-level"] === 3,
    },
  };
}

if (import.meta.main) {
  const report = await analyzeWorkspaceOptimization();
  console.log("=== Aphrody Labs Yolo UI Cargo & Fleet Optimization Report ===");
  console.log(`Workspace Members      : ${report.workspaceMembersCount}`);
  console.log(`Excluded Paths         : ${report.excludedMembersCount}`);
  console.log(`Workspace Dependencies : ${report.workspaceDependenciesCount}`);
  console.log(
    `Independent Cargo Roots : ${report.independentCargoWorkspaces.length}`,
  );
  for (const workspace of report.independentCargoWorkspaces) {
    console.log(`  - ${workspace}`);
  }
  console.log(`Submodules Discovered  : ${report.submodulesCount}`);
  console.log(
    `Inline Product Versions: ${report.inlineDependencyVersions.length}`,
  );
  console.log(
    `Repeated Inline Versions: ${report.repeatedInlineDependencies.length}`,
  );
  console.log(
    `Defined Profiles (${report.profiles.length})  : ${report.profiles.join(", ")}`,
  );
  console.log("\nCompiler Optimization Flags:");
  console.log(
    `  - [profile.dev] debug = "line-tables-only" : ${report.compilerOptimizations.devLineTablesOnly}`,
  );
  console.log(
    `  - [profile.release] incremental = true     : ${report.compilerOptimizations.releaseIncremental}`,
  );
  console.log(
    `  - [profile.release] panic = "unwind"      : ${report.compilerOptimizations.releasePanicUnwind}`,
  );
  console.log(
    `  - [profile.release] lto = true             : ${report.compilerOptimizations.releaseLto}`,
  );
  console.log(
    `  - [profile.release] opt-level = "s"        : ${report.compilerOptimizations.releaseSizeOptimization}`,
  );
  console.log(
    `  - [profile.release] strip = true           : ${report.compilerOptimizations.releaseStrip}`,
  );
  console.log(
    `  - [profile.release] codegen-units = 1      : ${report.compilerOptimizations.releaseSingleCodegenUnit}`,
  );
  console.log(
    `  - [profile.wasm-split-release] lto = true  : ${report.compilerOptimizations.wasmSplitLto}`,
  );
  console.log(
    `  - [profile.cli-release-dev] opt-level = 3  : ${report.compilerOptimizations.cliReleaseOptLevel3}`,
  );
  if (report.duplicateWorkspaceDeps.length > 0) {
    console.warn(
      `\nDuplicate [workspace.dependencies] keys found: ${report.duplicateWorkspaceDeps.join(", ")}`,
    );
    process.exit(1);
  } else {
    console.log("\nZero duplicate [workspace.dependencies] entries detected.");
  }
  if (report.inlineDependencyVersions.length > 0) {
    console.warn(
      "\nDependency versions still declared inside product manifests:",
    );
    for (const dependency of report.inlineDependencyVersions) {
      console.warn(`  - ${dependency}`);
    }
  } else {
    console.log(
      "Zero inline dependency versions remain in crates/ and packages/.",
    );
  }
  if (report.repeatedInlineDependencies.length > 0) {
    console.log("\nRepeated versions to review for workspace centralization:");
    for (const dependency of report.repeatedInlineDependencies.slice(0, 40)) {
      console.log(`  - ${dependency}`);
    }
  }
}
