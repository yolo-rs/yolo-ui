import { join, resolve } from "node:path";

const ROOT_DIR = resolve(import.meta.dir, "..");

export interface BunMigrationAudit {
  workspacePackageJsons: string[];
  legacyLockfilesFound: string[];
  legacyConfigFilesFound: string[];
  migratedScriptsCount: number;
  bunWorkspacesConfigured: boolean;
}

const LEGACY_LOCKFILES = [
  "packages/extension/package-lock.json",
  "packages/playwright-tests/package-lock.json",
  "package-lock.json",
  "yarn.lock",
  "pnpm-lock.yaml",
];

const LEGACY_CONFIG_FILES = [
  "packages/extension/webpack.config.js",
  "packages/extension/.eslintrc.js",
];

export async function auditBunMigration(): Promise<BunMigrationAudit> {
  const pkgPaths = [
    "package.json",
    "packages/extension/package.json",
    "packages/playwright-tests/package.json",
  ];

  const workspacePackageJsons: string[] = [];
  let migratedScriptsCount = 0;

  for (const rel of pkgPaths) {
    const file = Bun.file(join(ROOT_DIR, rel));
    if (await file.exists()) {
      workspacePackageJsons.push(rel);
      const pkg = (await file.json()) as {
        scripts?: Record<string, string>;
      };
      for (const cmd of Object.values(pkg.scripts ?? {})) {
        if (cmd.includes("bun ") || cmd.includes("bunx ")) {
          migratedScriptsCount++;
        }
      }
    }
  }

  const legacyLockfilesFound: string[] = [];
  for (const rel of LEGACY_LOCKFILES) {
    if (await Bun.file(join(ROOT_DIR, rel)).exists()) {
      legacyLockfilesFound.push(rel);
    }
  }

  const legacyConfigFilesFound: string[] = [];
  for (const rel of LEGACY_CONFIG_FILES) {
    if (await Bun.file(join(ROOT_DIR, rel)).exists()) {
      legacyConfigFilesFound.push(rel);
    }
  }

  const rootPkg = (await Bun.file(join(ROOT_DIR, "package.json")).json()) as {
    workspaces?: string[];
    packageManager?: string;
  };
  const bunWorkspacesConfigured =
    Array.isArray(rootPkg.workspaces) &&
    rootPkg.workspaces.length > 0 &&
    Boolean(rootPkg.packageManager?.startsWith("bun@"));

  return {
    workspacePackageJsons,
    legacyLockfilesFound,
    legacyConfigFilesFound,
    migratedScriptsCount,
    bunWorkspacesConfigured,
  };
}

if (import.meta.main) {
  const audit = await auditBunMigration();
  console.log("=== Yolo UI Node -> Bun (n2b) Migration Audit ===");
  console.log(JSON.stringify(audit, null, 2));
  if (
    audit.legacyLockfilesFound.length > 0 ||
    audit.legacyConfigFilesFound.length > 0
  ) {
    process.exit(1);
  }
}
