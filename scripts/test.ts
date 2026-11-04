import { join, resolve } from "node:path";

const ROOT_DIR = resolve(import.meta.dir, "..");

export const UMBRELLA_TEST_CRATES = [
  "yolo-shared",
  "yolo-ui",
  "yolo-gpui",
  "yolo-iced",
  "yolo-tauri",
  "yolo-showcase",
] as const;

const TEST_CRATE_REL_PATHS: Record<
  (typeof UMBRELLA_TEST_CRATES)[number],
  string
> = {
  "yolo-shared": "crates/yolo-shared",
  "yolo-ui": "packages/yolo-ui",
  "yolo-gpui": "packages/yolo-gpui",
  "yolo-iced": "packages/yolo-iced",
  "yolo-tauri": "packages/yolo-tauri",
  "yolo-showcase": "apps/yolo-showcase",
};

export async function getTestableUmbrellaCrates(): Promise<string[]> {
  const cargoTomlText = await Bun.file(join(ROOT_DIR, "Cargo.toml")).text();
  const parsed = Bun.TOML.parse(cargoTomlText) as {
    workspace?: { members?: string[] };
  };
  const members = new Set(parsed.workspace?.members ?? []);

  const testable: string[] = [];
  for (const crateName of UMBRELLA_TEST_CRATES) {
    const pkgPath = TEST_CRATE_REL_PATHS[crateName];
    const manifestExists = await Bun.file(
      join(ROOT_DIR, pkgPath, "Cargo.toml"),
    ).exists();
    if (manifestExists && members.has(pkgPath)) {
      testable.push(crateName);
    }
  }
  return testable;
}

export async function runRustUmbrellaTests(): Promise<number> {
  const crates = await getTestableUmbrellaCrates();
  if (crates.length === 0) {
    console.error("No umbrella crates found in workspace.");
    return 1;
  }

  const pkgArgs: string[] = [];
  for (const c of crates) {
    pkgArgs.push("-p", c);
  }

  const cmd = [
    "cargo",
    "test",
    ...pkgArgs,
    "--no-default-features",
    "--features",
    "yolo-ui/gpui,yolo-ui/iced,yolo-ui/tauri,yolo-gpui/default,yolo-iced/default,yolo-tauri/default",
  ];
  console.log(`Running umbrella Rust unit tests: ${cmd.join(" ")}`);
  const proc = Bun.spawn(cmd, {
    cwd: ROOT_DIR,
    stdout: "inherit",
    stderr: "inherit",
  });
  return await proc.exited;
}

if (import.meta.main) {
  const code = await runRustUmbrellaTests();
  process.exit(code);
}
