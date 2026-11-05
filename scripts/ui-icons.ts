// SPDX-License-Identifier: Apache-2.0
// `bun ui icons <fetch|build|search|favicon|verify|optimize>`.
//
// `fetch` materializes exactly the subset listed in catalog/assets.lock.toml (Bun only, no package manager).
// Every other subcommand runs the Rust `yolo-icons` binary: $YOLO_ICONS_BIN, then target/{release,debug}, then
// `cargo run --release -p yolo-icons` as a last resort.

import { existsSync } from "node:fs";
import { mkdir } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";

export const ICONS_REPO_ROOT = resolve(import.meta.dir, "..");
export const ASSET_LOCK_PATH = join(
  ICONS_REPO_ROOT,
  "catalog/assets.lock.toml",
);

export interface AssetSet {
  id: string;
  name: string;
  format: "svg" | "chromium-icon";
  license: string;
  upstream: string;
  commit?: string;
  version?: string;
  attribution: string;
  fetch: "local" | "github-raw" | "submodule" | "chromium";
  base_url?: string;
  license_path?: string;
  submodule?: string;
  dir: string;
  priority: number;
  icons: Record<string, string>;
}

export interface AssetLock {
  schema: number;
  set: AssetSet[];
}

export async function readAssetLock(
  path = ASSET_LOCK_PATH,
): Promise<AssetLock> {
  const lock = Bun.TOML.parse(
    await Bun.file(path).text(),
  ) as unknown as AssetLock;
  if (lock.schema !== 1 || !Array.isArray(lock.set)) {
    throw new Error(`${path}: unsupported asset lock`);
  }
  return lock;
}

function extensionOf(set: AssetSet): string {
  return set.format === "chromium-icon" ? "icon" : "svg";
}

/** Repository-relative destination of one icon (mirrors `AssetSet::local_file` in Rust). */
export function localFile(set: AssetSet, name: string): string {
  return set.fetch === "local"
    ? join(set.dir, set.icons[name]!)
    : join(set.dir, `${name}.${extensionOf(set)}`);
}

function chromiumRoot(): string | undefined {
  const root = process.env.CHROMIUM_ROOT;
  if (!root) return undefined;
  return existsSync(join(root, "src", "LICENSE")) ? join(root, "src") : root;
}

async function download(url: string, base64: boolean): Promise<Uint8Array> {
  const res = await fetch(url);
  if (!res.ok) throw new Error(`GET ${url}: HTTP ${res.status}`);
  if (base64) {
    const binary = atob((await res.text()).replace(/\s+/g, ""));
    return Uint8Array.from(binary, (c) => c.charCodeAt(0));
  }
  return new Uint8Array(await res.arrayBuffer());
}

/** Reads one upstream file: local checkout first (submodule or CHROMIUM_ROOT), then the pinned URL. */
async function readUpstream(
  set: AssetSet,
  upstreamPath: string,
): Promise<{ bytes: Uint8Array; from: string }> {
  const localRoot =
    set.fetch === "submodule" && set.submodule
      ? join(ICONS_REPO_ROOT, set.submodule)
      : set.fetch === "chromium"
        ? chromiumRoot()
        : undefined;
  if (localRoot) {
    const candidate = Bun.file(join(localRoot, upstreamPath));
    if (await candidate.exists()) {
      return {
        bytes: await candidate.bytes(),
        from: "checkout",
      };
    }
  }
  if (!set.base_url)
    throw new Error(`${set.id}: no base_url to fetch ${upstreamPath}`);
  const encoded = upstreamPath.split("/").map(encodeURIComponent).join("/");
  if (set.fetch === "chromium") {
    return {
      bytes: await download(`${set.base_url}${encoded}?format=TEXT`, true),
      from: "googlesource",
    };
  }
  return {
    bytes: await download(`${set.base_url}${encoded}`, false),
    from: "github",
  };
}

async function writeFile(
  rel: string,
  bytes: Uint8Array | string,
): Promise<void> {
  const abs = join(ICONS_REPO_ROOT, rel);
  await mkdir(dirname(abs), { recursive: true });
  await Bun.write(abs, bytes);
}

export interface FetchSummary {
  set: string;
  fetched: number;
  skipped: number;
  sources: Record<string, number>;
}

/** Materializes the listed subset of every third-party set, plus its LICENSE and a NOTICE. */
export async function fetchIconSources(
  options: { force?: boolean; only?: string } = {},
): Promise<FetchSummary[]> {
  const lock = await readAssetLock();
  const out: FetchSummary[] = [];
  for (const set of lock.set) {
    if (set.fetch === "local" || (options.only && options.only !== set.id))
      continue;
    const summary: FetchSummary = {
      set: set.id,
      fetched: 0,
      skipped: 0,
      sources: {},
    };
    for (const [name, upstreamPath] of Object.entries(set.icons)) {
      const dest = localFile(set, name);
      if (!options.force && existsSync(join(ICONS_REPO_ROOT, dest))) {
        summary.skipped++;
        continue;
      }
      const { bytes, from } = await readUpstream(set, upstreamPath);
      await writeFile(dest, bytes);
      summary.fetched++;
      summary.sources[from] = (summary.sources[from] ?? 0) + 1;
    }
    if (set.license_path) {
      const { bytes } = await readUpstream(set, set.license_path);
      await writeFile(join(set.dir, "LICENSE"), bytes);
    }
    const pin = set.commit ? `commit ${set.commit}` : `version ${set.version}`;
    await writeFile(
      join(set.dir, "NOTICE"),
      `${set.name}\nUpstream: ${set.upstream}\nPinned: ${pin}\nLicense: ${set.license}\n${set.attribution}\n` +
        `Subset recorded in catalog/assets.lock.toml; canonical names map to upstream paths there.\n`,
    );
    out.push(summary);
  }
  return out;
}

/** Resolves the command that runs the `yolo-icons` binary. */
export function yoloIconsCommand(): string[] {
  const fromEnv = process.env.YOLO_ICONS_BIN;
  if (fromEnv) return [fromEnv];
  const targetDir =
    process.env.CARGO_TARGET_DIR ?? join(ICONS_REPO_ROOT, "target");
  for (const profile of ["release", "debug"]) {
    for (const exe of ["yolo-icons.exe", "yolo-icons"]) {
      const candidate = join(targetDir, profile, exe);
      if (existsSync(candidate)) return [candidate];
    }
  }
  return [
    "cargo",
    "run",
    "--quiet",
    "--release",
    "-p",
    "yolo-icons",
    "--bin",
    "yolo-icons",
    "--",
  ];
}

export async function runYoloIcons(args: string[]): Promise<number> {
  const cmd = [...yoloIconsCommand(), "--root", ICONS_REPO_ROOT, ...args];
  const proc = Bun.spawn(cmd, {
    cwd: ICONS_REPO_ROOT,
    stdout: "inherit",
    stderr: "inherit",
  });
  return await proc.exited;
}

const ICONS_HELP = `
bun ui icons fetch [--force] [--set <id>]          materialize the subset listed in catalog/assets.lock.toml
bun ui icons build [--out dist/icons] [--precision 2] [--grid 24]
bun ui icons search <query> [-n 20]
bun ui icons favicon [--preset aphrody] [--out dist/favicon] [--mark file.svg|file.icon] [--name N] [--two-tone] [--dark]
bun ui icons verify <dir>
bun ui icons optimize <file.svg|file.icon> [--precision 2]
`;

export async function runIconsCli(argv: string[]): Promise<number> {
  const [sub, ...rest] = argv;
  switch (sub) {
    case "fetch": {
      const setIdx = rest.indexOf("--set");
      const summary = await fetchIconSources({
        force: rest.includes("--force"),
        only: setIdx >= 0 ? rest[setIdx + 1] : undefined,
      });
      console.log(JSON.stringify(summary, null, 2));
      return 0;
    }
    case "build":
    case "search":
    case "favicon":
    case "verify":
    case "optimize":
      return await runYoloIcons([sub, ...rest]);
    case undefined:
    case "help":
    case "--help":
    case "-h":
      console.log(ICONS_HELP);
      return 0;
    default:
      console.error(`Unknown icons subcommand: ${sub}`);
      console.log(ICONS_HELP);
      return 1;
  }
}

/** Runs `yolo-icons` with piped output and parses its JSON report (used by the `ui_icons` MCP tool). */
export async function runYoloIconsJson(args: string[]): Promise<unknown> {
  const cmd = [...yoloIconsCommand(), "--root", ICONS_REPO_ROOT, ...args];
  const proc = Bun.spawn(cmd, {
    cwd: ICONS_REPO_ROOT,
    stdout: "pipe",
    stderr: "pipe",
  });
  const [stdout, stderr, code] = await Promise.all([
    new Response(proc.stdout).text(),
    new Response(proc.stderr).text(),
    proc.exited,
  ]);
  if (code !== 0) {
    throw new Error(`yolo-icons ${args[0]} exited ${code}: ${stderr.trim()}`);
  }
  return JSON.parse(stdout);
}
