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
  duplicateWorkspaceDeps: string[];
  compilerOptimizations: {
    devLineTablesOnly: boolean;
    releaseIncremental: boolean;
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

  const members = parsed.workspace?.members ?? [];
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

  const devProfile = parsed.profile?.["dev"] ?? {};
  const releaseProfile = parsed.profile?.["release"] ?? {};
  const wasmSplitProfile = parsed.profile?.["wasm-split-release"] ?? {};
  const cliReleaseProfile = parsed.profile?.["cli-release-dev"] ?? {};

  return {
    workspaceMembersCount: members.length,
    excludedMembersCount: exclude.length,
    workspaceDependenciesCount: Object.keys(deps).length,
    submodulesCount: submodules.length,
    profiles,
    duplicateWorkspaceDeps: Array.from(duplicates),
    compilerOptimizations: {
      devLineTablesOnly: devProfile["debug"] === "line-tables-only",
      releaseIncremental: releaseProfile["incremental"] === true,
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
  console.log(`Submodules Discovered  : ${report.submodulesCount}`);
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
}
