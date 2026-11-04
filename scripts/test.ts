import { resolve } from "node:path";

const ROOT_DIR = resolve(import.meta.dir, "..");
const args = ["cargo", "test", "--workspace", ...Bun.argv.slice(2)];
console.log(`Executing: ${args.join(" ")}`);

const proc = Bun.spawn(args, {
  cwd: ROOT_DIR,
  stdout: "inherit",
  stderr: "inherit",
});
process.exit(await proc.exited);
