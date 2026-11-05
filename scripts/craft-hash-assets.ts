#!/usr/bin/env bun
// SPDX-License-Identifier: Apache-2.0
// Content-hash the Craft studio assets after the build has assembled them and before deployment.
//
//   bun scripts/craft-hash-assets.ts <assembled-dir> [--no-precompress]
//
// <assembled-dir> holds one directory per studio (apps/webos/out/craft from build-craft.ts). In each studio, the
// wasm-bindgen module (<bin>_bg.wasm), its glue (<bin>.js) and theme.css become <stem>-<8 hex>.<ext>, the
// first 8 hex digits of the sha256 of the final bytes. The glue's reference to the wasm is rewritten before
// the glue is hashed. The pages, workers and service-worker lists that name these files are rewritten in
// turn. Those names match the immutable rule of craft.aphrody.app.conf (`-[0-9a-f]{8}\.(js|wasm|css)$`),
// while documents keep `no-cache`. Unless --no-precompress is given, each hashed file also gets .br (q11)
// and .zst (-19) siblings for brotli_static/zstd_static. Running it twice is refused because the plain names
// are gone after the first run.
import { readdirSync, renameSync, statSync } from "node:fs";
import { join, relative } from "node:path";

const [dir, ...flags] = Bun.argv.slice(2);
if (!dir || flags.some(flag => flag !== "--no-precompress"))
  throw new Error("usage: bun scripts/craft-hash-assets.ts <assembled-dir> [--no-precompress]");
const precompress = !flags.includes("--no-precompress");

const hash8 = (bytes: Uint8Array) => new Bun.CryptoHasher("sha256").update(bytes).digest("hex").slice(0, 8);
const escape = (text: string) => text.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
/** Replace whole file names only: "theme.css" must not match inside "craft-theme.css". */
function rewrite(text: string, names: Map<string, string>): string {
  for (const [from, to] of names)
    text = text.replace(new RegExp(`(?<![\\w.-])${escape(from)}(?![\\w-])`, "g"), to);
  return text;
}
function hashedName(name: string, bytes: Uint8Array): string {
  const dot = name.lastIndexOf(".");
  return `${name.slice(0, dot)}-${hash8(bytes)}${name.slice(dot)}`;
}
function textFiles(root: string): string[] {
  const out: string[] = [];
  for (const entry of readdirSync(root, { withFileTypes: true })) {
    const path = join(root, entry.name);
    if (entry.isDirectory()) {
      if (entry.name !== "licenses" && entry.name !== "fonts") out.push(...textFiles(path));
    } else if (/\.(?:html|js|mjs|json|webmanifest)$/.test(entry.name)) out.push(path);
  }
  return out;
}

const hashed: string[] = [];
const studios = readdirSync(dir, { withFileTypes: true }).filter(entry => entry.isDirectory());
if (!studios.length) throw new Error(`${dir}: no studio directories`);
for (const studio of studios) {
  const root = join(dir, studio.name);
  const files = readdirSync(root);
  const wasm = files.filter(name => /^[a-z0-9_]+_bg\.wasm$/.test(name));
  if (wasm.length !== 1) throw new Error(`${studio.name}: expected one plain <bin>_bg.wasm, found ${wasm.length}`);
  const bin = wasm[0]!.slice(0, -"_bg.wasm".length);
  const names = new Map<string, string>();

  // Leaves first: the wasm module and the stylesheet reference nothing that gets renamed.
  for (const name of [`${bin}_bg.wasm`, "theme.css"]) {
    if (!files.includes(name)) continue;
    const bytes = await Bun.file(join(root, name)).bytes();
    const next = hashedName(name, bytes);
    renameSync(join(root, name), join(root, next));
    names.set(name, next);
  }
  // The glue names the wasm (wasm-bindgen's default `new URL('<bin>_bg.wasm', import.meta.url)`).
  const glue = `${bin}.js`;
  if (!files.includes(glue)) throw new Error(`${studio.name}: ${glue} is missing`);
  const glueText = rewrite(await Bun.file(join(root, glue)).text(), names);
  const glueNext = hashedName(glue, new TextEncoder().encode(glueText));
  await Bun.write(join(root, glueNext), glueText);
  await Bun.file(join(root, glue)).delete();
  names.set(glue, glueNext);

  // Everything else that names them: pages, workers, the service-worker precache list, manifests.
  const finals = new Set(names.values());
  let references = 0;
  for (const path of textFiles(root)) {
    if (finals.has(relative(root, path))) continue;
    const before = await Bun.file(path).text();
    const after = rewrite(before, names);
    if (after !== before) {
      await Bun.write(path, after);
      references++;
    }
  }
  // Refuse a studio whose page no longer reaches the glue, or any file still naming a plain asset.
  const texts = new Map<string, string>();
  for (const path of textFiles(root))
    if (!finals.has(relative(root, path))) texts.set(path, await Bun.file(path).text());
  if (![...texts.values()].some(text => text.includes(glueNext)))
    throw new Error(`${studio.name}: nothing references ${glueNext}`);
  for (const old of names.keys())
    for (const [path, text] of texts)
      if (rewrite(text, new Map([[old, "\0"]])).includes("\0")) throw new Error(`${relative(dir, path)} still names ${old}`);
  console.log(`${studio.name}: ${[...names.values()].join(" ")} (${references} referencing files)`);
  hashed.push(...[...finals].map(name => join(root, name)));
}

if (precompress) {
  // Static siblings compressed once at the highest useful level; zstd -19 keeps an 8 MB window (RFC 9659).
  const jobs = hashed.flatMap(path => [
    ["brotli", "--force", "--quality=11", "--keep", `--output=${path}.br`, path],
    ["zstd", "--force", "-19", "--quiet", "--keep", "-o", `${path}.zst`, path],
  ]);
  const width = Math.max(1, Number(Bun.env.CRAFT_COMPRESS_JOBS ?? 4));
  let next = 0;
  await Promise.all(
    Array.from({ length: width }, async () => {
      while (next < jobs.length) {
        const argv = jobs[next++]!;
        const proc = Bun.spawn(argv, { stdout: "inherit", stderr: "inherit" });
        if ((await proc.exited) !== 0) throw new Error(`${argv[0]} failed on ${argv.at(-1)}`);
      }
    }),
  );
  for (const path of hashed.filter(p => p.endsWith(".wasm")))
    console.log(
      `${relative(dir, path)}: ${statSync(path).size} B, br ${statSync(`${path}.br`).size} B, zst ${statSync(`${path}.zst`).size} B`,
    );
}
