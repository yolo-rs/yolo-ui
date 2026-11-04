import { resolve } from "node:path";

const root = resolve(import.meta.dir, "..");
const rootManifest = resolve(root, "Cargo.toml");
const appPrefix = `${resolve(root, "apps/spacedrive").replaceAll("\\", "/")}/`;
const metadata = Bun.spawnSync(
  [
    "cargo",
    "metadata",
    "--no-deps",
    "--format-version",
    "1",
    "--manifest-path",
    rootManifest,
  ],
  { cwd: root, stdout: "pipe", stderr: "pipe" },
);
if (metadata.exitCode !== 0) {
  throw new Error(new TextDecoder().decode(metadata.stderr));
}

const cargo = JSON.parse(new TextDecoder().decode(metadata.stdout)) as {
  packages: { id: string; manifest_path: string; name: string }[];
  workspace_members: string[];
};
const workspaceMembers = new Set(cargo.workspace_members);
const packages = cargo.packages
  .filter(
    (pkg) =>
      workspaceMembers.has(pkg.id) &&
      resolve(pkg.manifest_path).replaceAll("\\", "/").startsWith(appPrefix),
  )
  .map((pkg) => pkg.name);
const uniquePackages = [...new Set(packages)];
if (uniquePackages.length === 0)
  throw new Error("No Spacedrive packages found in the root Cargo workspace");

const result = Bun.spawnSync(
  [
    "cargo",
    "test",
    "--manifest-path",
    rootManifest,
    ...uniquePackages.flatMap((name) => ["--package", name]),
  ],
  { cwd: root, stdout: "inherit", stderr: "inherit" },
);
process.exit(result.exitCode ?? 1);
