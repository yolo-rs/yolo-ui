import { resolve } from "node:path";

const ROOT_DIR = resolve(import.meta.dir, "..");
type BuildMode = "check" | "release" | "metadata" | "workspace";

export async function runCargoBuild(
  mode: BuildMode = "check",
): Promise<number> {
  const args =
    mode === "metadata"
      ? ["cargo", "metadata", "--no-deps", "--format-version", "1"]
      : mode === "workspace"
        ? ["cargo", "check", "--workspace"]
        : mode === "release"
          ? ["cargo", "build", "--release"]
          : ["cargo", "check"];

  console.log(`Executing: ${args.join(" ")}`);
  const proc = Bun.spawn(args, {
    cwd: ROOT_DIR,
    stdout: mode === "metadata" ? "ignore" : "inherit",
    stderr: "inherit",
  });
  return await proc.exited;
}

if (import.meta.main) {
  const modeArg = (Bun.argv[2] ?? "check") as BuildMode;
  const metaCode = await runCargoBuild("metadata");
  if (metaCode !== 0) process.exit(metaCode);
  if (modeArg !== "metadata") process.exit(await runCargoBuild(modeArg));
}
