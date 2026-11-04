import { join, resolve } from "node:path";

const ROOT_DIR = resolve(import.meta.dir, "..");

export const TARGET_UMBRELLA_CRATES = [
  "yolo-shared",
  "yolo-tokens",
  "yolo-contracts",
  "yolo-emit",
  "yolo-layout",
  "yolo-migrator",
  "yolo-shell",
  "yolo-craft",
  "yolo-storyteller",
  "yolo-react",
  "yolo-ui",
  "yolo-gpui",
  "yolo-iced",
  "yolo-tauri",
  "yolo-egui",
  "yolo-cosmic",
  "yolo-gtk",
  "yolo-qt",
  "yolo-showcase",
  "dioxus",
] as const;

const CRATE_REL_PATHS: Record<(typeof TARGET_UMBRELLA_CRATES)[number], string> =
  {
    "yolo-shared": "crates/yolo-shared",
    "yolo-tokens": "crates/yolo-tokens",
    "yolo-contracts": "crates/yolo-contracts",
    "yolo-emit": "crates/yolo-emit",
    "yolo-layout": "crates/yolo-layout",
    "yolo-migrator": "crates/yolo-migrator",
    "yolo-shell": "crates/yolo-shell",
    "yolo-craft": "crates/yolo-craft",
    "yolo-storyteller": "crates/yolo-storyteller",
    "yolo-react": "crates/yolo-react",
    "yolo-ui": "packages/yolo-ui",
    "yolo-gpui": "packages/yolo-gpui",
    "yolo-iced": "packages/yolo-iced",
    "yolo-tauri": "packages/yolo-tauri",
    "yolo-egui": "packages/yolo-egui",
    "yolo-cosmic": "packages/yolo-cosmic",
    "yolo-gtk": "packages/yolo-gtk",
    "yolo-qt": "packages/yolo-qt",
    "yolo-showcase": "apps/yolo-showcase",
    dioxus: "packages/dioxus",
  };

export async function getAvailableUmbrellaCrates(): Promise<string[]> {
  const cargoTomlText = await Bun.file(join(ROOT_DIR, "Cargo.toml")).text();
  const parsed = Bun.TOML.parse(cargoTomlText) as {
    workspace?: { members?: string[] };
  };
  const members = new Set(parsed.workspace?.members ?? []);

  const available: string[] = [];
  for (const crateName of TARGET_UMBRELLA_CRATES) {
    const pkgPath = CRATE_REL_PATHS[crateName];
    const manifestExists = await Bun.file(
      join(ROOT_DIR, pkgPath, "Cargo.toml"),
    ).exists();
    if (manifestExists && members.has(pkgPath)) {
      available.push(crateName);
    }
  }
  return available;
}

export async function runCargoBuild(
  mode: "check" | "release" | "metadata" | "workspace" = "check",
): Promise<number> {
  if (mode === "metadata") {
    console.log("Running cargo metadata --no-deps...");
    const proc = Bun.spawn(
      ["cargo", "metadata", "--no-deps", "--format-version", "1"],
      {
        cwd: ROOT_DIR,
        stdout: "ignore",
        stderr: "inherit",
      },
    );
    return await proc.exited;
  }

  const crates = await getAvailableUmbrellaCrates();
  const pkgArgs: string[] = [];
  if (mode !== "workspace") {
    for (const c of crates) {
      pkgArgs.push("-p", c);
    }
  } else {
    pkgArgs.push("--workspace");
  }

  const args =
    mode === "release"
      ? ["cargo", "build", "--release", ...pkgArgs]
      : ["cargo", "check", ...pkgArgs];

  console.log(`Executing: ${args.join(" ")}`);
  const proc = Bun.spawn(args, {
    cwd: ROOT_DIR,
    stdout: "inherit",
    stderr: "inherit",
  });
  return await proc.exited;
}

if (import.meta.main) {
  const modeArg = (Bun.argv[2] ?? "check") as
    "check" | "release" | "metadata" | "workspace";
  const metaCode = await runCargoBuild("metadata");
  if (metaCode !== 0) {
    process.exit(metaCode);
  }
  if (modeArg !== "metadata") {
    const buildCode = await runCargoBuild(modeArg);
    process.exit(buildCode);
  }
}
