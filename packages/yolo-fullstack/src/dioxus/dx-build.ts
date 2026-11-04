// SPDX-License-Identifier: MIT OR Apache-2.0
// Run `dx build --platform web` and locate the generated public directory.

import { existsSync, statSync } from "node:fs";
import { join, resolve } from "node:path";

export interface DxBuildOptions {
  /** Crate directory containing Cargo.toml and Dioxus.toml. */
  cwd: string;
  /** Dioxus platform; only "web" produces a static public directory. */
  platform?: "web";
  release?: boolean;
  /** Path or name of the dx binary; defaults to "dx". */
  dx?: string;
  env?: Record<string, string | undefined>;
  /** Extra arguments appended after the generated ones. */
  args?: string[];
  /** Receives stdout/stderr chunks; defaults to writing to process stdout/stderr. */
  onOutput?: (stream: "stdout" | "stderr", chunk: string) => void;
}

export interface DxBuildResult {
  exitCode: number;
  /** Detected `target/dx/<crate>/{release|debug}/web/public`, or null when none exists. */
  publicDir: string | null;
  command: string[];
}

/** Find the dx web public directory for a crate, preferring the requested profile. */
export function findDxPublicDir(cwd: string, release = false): string | null {
  const root = resolve(cwd, "target", "dx");
  if (!existsSync(root) || !statSync(root).isDirectory()) return null;
  const profile = release ? "release" : "debug";
  const glob = new Bun.Glob(`*/${profile}/web/public`);
  let best: string | null = null;
  let bestMtime = -1;
  for (const match of glob.scanSync({ cwd: root, onlyFiles: false })) {
    const candidate = join(root, match);
    if (!existsSync(join(candidate, "index.html"))) continue;
    const mtime = statSync(candidate).mtimeMs;
    if (mtime > bestMtime) {
      best = candidate;
      bestMtime = mtime;
    }
  }
  return best;
}

async function pump(
  stream: ReadableStream<Uint8Array> | null,
  name: "stdout" | "stderr",
  onOutput: (stream: "stdout" | "stderr", chunk: string) => void,
): Promise<void> {
  if (stream === null) return;
  const decoder = new TextDecoder();
  for await (const chunk of stream) {
    onOutput(name, decoder.decode(chunk, { stream: true }));
  }
  const tail = decoder.decode();
  if (tail.length > 0) onOutput(name, tail);
}

const defaultOutput = (stream: "stdout" | "stderr", chunk: string): void => {
  if (stream === "stdout") process.stdout.write(chunk);
  else process.stderr.write(chunk);
};

/** Spawn `dx build` with streamed output and return the exit code plus the public directory. */
export async function dxBuild(options: DxBuildOptions): Promise<DxBuildResult> {
  const platform = options.platform ?? "web";
  const command = [options.dx ?? "dx", "build", "--platform", platform];
  if (options.release) command.push("--release");
  if (options.args) command.push(...options.args);
  const onOutput = options.onOutput ?? defaultOutput;

  const proc = Bun.spawn(command, {
    cwd: options.cwd,
    env: { ...process.env, ...options.env },
    stdin: "ignore",
    stdout: "pipe",
    stderr: "pipe",
  });
  await Promise.all([pump(proc.stdout, "stdout", onOutput), pump(proc.stderr, "stderr", onOutput)]);
  const exitCode = await proc.exited;
  return {
    exitCode,
    publicDir: findDxPublicDir(options.cwd, options.release ?? false),
    command,
  };
}
