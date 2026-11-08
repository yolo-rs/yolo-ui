// SPDX-License-Identifier: Apache-2.0
// @yolo-rs/next/plugin: the Bun plugin of a Yolo Next project: `next/*` -> shims, `"use client"` /
// `"use server"` boundaries per environment (rsc / server / client) with react-server-dom-webpack
// references, and tsconfig `paths` aliases.

import type { BunPlugin, PluginBuilder } from "bun";
import { existsSync, readFileSync, statSync } from "node:fs";
import { dirname, isAbsolute, join, relative, resolve } from "node:path";

import { detectDirective, scanModuleExports } from "./segments.ts";
import type { ModuleDirective, ResolvedYoloConfig } from "./types.ts";

/** Bundling environment of the plugin. */
export type PluginEnv = "client" | "server" | "rsc";

/** Options of `yoloNextPlugin`. */
export interface YoloNextPluginOptions {
  readonly root: string;
  readonly config?: ResolvedYoloConfig;
  readonly env: PluginEnv;
  /** Called with the module id (root-relative, `/` separators) of every `"use client"` module seen by the rsc pass. */
  readonly onClientModule?: (id: string) => void;
  /** Called for every exported server action (`id` root-relative, plus the export name). */
  readonly onServerAction?: (id: string, exportName: string) => void;
  /** Specifier of the actions client module (default `@yolo-rs/next/actions`). */
  readonly actionsModule?: string;
}

const SRC_DIR = import.meta.dir;
/** Directory of the `next/*` shim modules. */
export const SHIMS_DIR = join(SRC_DIR, "shims");

/** `next/*` specifier -> shim file name. */
export const SHIM_MAP: Readonly<Record<string, string>> = {
  "next/font/google": "font-google.ts",
  "next/font/local": "font-local.ts",
  "next/image": "image.ts",
  "next/legacy/image": "legacy-image.ts",
  "next/link": "link.ts",
  "next/navigation": "navigation.ts",
  "next/headers": "headers.ts",
  "next/server": "server.ts",
  "next/cache": "cache.ts",
  "next/script": "script.ts",
  "next/dynamic": "dynamic.ts",
  "next/form": "form.ts",
  "next/og": "og.ts",
  "next/router": "router.ts",
  "next/head": "head.ts",
  "next/root-params": "root-params.ts",
  "next/error": "error.ts",
  "server-only": "server-only.ts",
  "client-only": "client-only.ts",
};

const EMPTY_NAMESPACE = "yolo-empty";
const toPosix = (path: string): string => path.replace(/\\/g, "/");

function canonicalSpecifier(specifier: string): string {
  return specifier.replace(/\.(?:js|mjs|cjs)$/, "").replace(/\/index$/, "");
}

/** Absolute shim path of a `next/*`, `server-only` or `client-only` specifier; undefined when unknown. */
export function resolveNextSpecifier(specifier: string): string | undefined {
  const file = SHIM_MAP[canonicalSpecifier(specifier)];
  return file ? join(SHIMS_DIR, file) : undefined;
}

/** `@yolo-rs/next/shims/<name>` specifier of a `next/*` import (used by `migrate --rewrite-imports`). */
export function shimSpecifierFor(specifier: string): string | undefined {
  const file = SHIM_MAP[canonicalSpecifier(specifier)];
  return file ? `@yolo-rs/next/shims/${file.replace(/\.ts$/, "")}` : undefined;
}

/** Resolves `@yolo-rs/next[/subpath]` to the local source files when they exist (robust when the package is linked). */
export function resolveSelfSpecifier(specifier: string): string | undefined {
  if (specifier === "@yolo-rs/next") return join(SRC_DIR, "index.ts");
  if (!specifier.startsWith("@yolo-rs/next/")) return undefined;
  const sub = specifier.slice("@yolo-rs/next/".length).replace(/\.(?:ts|js)$/, "");
  const candidate = join(SRC_DIR, `${sub}.ts`);
  return existsSync(candidate) ? candidate : undefined;
}

/**
 * 40-hex server reference id: `"00"` + first 38 hex chars of `sha256(salt + id + ":" + exportName)`
 * (same shape as Next.js action ids).
 */
export function serverReferenceId(id: string, exportName: string, salt = ""): string {
  const hasher = new Bun.CryptoHasher("sha256");
  hasher.update(`${salt}${id}:${exportName}`);
  return `00${hasher.digest("hex").slice(0, 38)}`;
}

/** Export names of a module as `{ names, hasDefault, defaultName }` (light scanner or oxc). */
export function moduleExportNames(source: string, file: string): { names: string[]; hasDefault: boolean; defaultName?: string } {
  const exports = scanModuleExports(source, file);
  return { names: exports.names, hasDefault: exports.hasDefault, defaultName: exports.defaultName };
}

/** Stub of a `"use client"` module for the rsc environment (`registerClientReference` per export). */
export function clientReferenceStub(id: string, exports: { names: ReadonlyArray<string>; hasDefault: boolean }): string {
  const lines = ['import { registerClientReference } from "react-server-dom-webpack/server.edge";', ""];
  const ref = (name: string): string =>
    `registerClientReference(function() { throw new Error(${JSON.stringify(`Attempted to call ${name}() from the server but ${name} is on the client. It's not possible to invoke a client function from the server, it can only be rendered as a Component or passed to props of a Client Component.`)}); }, ${JSON.stringify(id)}, ${JSON.stringify(name)})`;
  for (const name of exports.names) lines.push(`export const ${name} = ${ref(name)};`);
  if (exports.hasDefault) lines.push(`export default ${ref("default")};`);
  lines.push("");
  return lines.join("\n");
}

/** Stub of a `"use server"` module for the client/ssr environments (`createServerReference` per export). */
export function serverReferenceStub(id: string, exports: { names: ReadonlyArray<string>; hasDefault: boolean }, actionsModule = "@yolo-rs/next/actions", salt = ""): string {
  const lines = ['import { createServerReference } from "react-server-dom-webpack/client";', `import { callServer } from ${JSON.stringify(actionsModule)};`, ""];
  for (const name of exports.names) {
    lines.push(`export const ${name} = createServerReference(${JSON.stringify(serverReferenceId(id, name, salt))}, callServer, undefined, undefined, ${JSON.stringify(name)});`);
  }
  if (exports.hasDefault) lines.push(`export default createServerReference(${JSON.stringify(serverReferenceId(id, "default", salt))}, callServer, undefined, undefined, "default");`);
  lines.push("");
  return lines.join("\n");
}

/** Trailer appended to a `"use server"` module in the rsc environment (`registerServerReference` per export). */
export function serverReferenceTrailer(id: string, exports: { names: ReadonlyArray<string>; hasDefault: boolean; defaultName?: string }, salt = ""): string {
  const lines = ["", 'import { registerServerReference as __yoloRegisterServerReference } from "react-server-dom-webpack/server.edge";'];
  for (const name of exports.names) lines.push(`if (typeof ${name} === "function") __yoloRegisterServerReference(${name}, ${JSON.stringify(serverReferenceId(id, name, salt))}, ${JSON.stringify(name)});`);
  if (exports.hasDefault && exports.defaultName && exports.defaultName !== "default") {
    lines.push(`if (typeof ${exports.defaultName} === "function") __yoloRegisterServerReference(${exports.defaultName}, ${JSON.stringify(serverReferenceId(id, "default", salt))}, "default");`);
  }
  lines.push("");
  return lines.join("\n");
}

function loaderFor(path: string): "tsx" | "ts" | "jsx" | "js" {
  if (path.endsWith(".tsx")) return "tsx";
  if (path.endsWith(".ts") || path.endsWith(".mts")) return "ts";
  if (path.endsWith(".jsx")) return "jsx";
  return "js";
}

/** Parses JSON with comments and trailing commas (tsconfig.json). */
export function parseJsonc(text: string): unknown {
  let out = "";
  let i = 0;
  while (i < text.length) {
    const ch = text.charAt(i);
    if (ch === '"') {
      const start = i;
      i += 1;
      while (i < text.length && text.charAt(i) !== '"') {
        if (text.charAt(i) === "\\") i += 1;
        i += 1;
      }
      i += 1;
      out += text.slice(start, i);
      continue;
    }
    if (ch === "/" && text.charAt(i + 1) === "/") {
      const nl = text.indexOf("\n", i);
      i = nl === -1 ? text.length : nl;
      continue;
    }
    if (ch === "/" && text.charAt(i + 1) === "*") {
      const end = text.indexOf("*/", i + 2);
      i = end === -1 ? text.length : end + 2;
      continue;
    }
    out += ch;
    i += 1;
  }
  return JSON.parse(out.replace(/,(\s*[}\]])/g, "$1"));
}

interface PathAlias {
  readonly prefix: string;
  readonly suffix: string;
  readonly targets: ReadonlyArray<string>;
  readonly wildcard: boolean;
}

/** Reads `compilerOptions.paths` (and `baseUrl`) of `<root>/tsconfig.json`; empty when absent. */
export function tsconfigPaths(root: string): { baseUrl: string; aliases: PathAlias[] } {
  const file = join(root, "tsconfig.json");
  if (!existsSync(file)) return { baseUrl: root, aliases: [] };
  try {
    const parsed = parseJsonc(readFileSync(file, "utf8")) as { compilerOptions?: { baseUrl?: string; paths?: Record<string, string[]> } };
    const options = parsed.compilerOptions ?? {};
    const baseUrl = resolve(root, options.baseUrl ?? ".");
    const aliases: PathAlias[] = [];
    for (const [pattern, targets] of Object.entries(options.paths ?? {})) {
      const star = pattern.indexOf("*");
      aliases.push({
        prefix: star === -1 ? pattern : pattern.slice(0, star),
        suffix: star === -1 ? "" : pattern.slice(star + 1),
        targets: Array.isArray(targets) ? targets : [],
        wildcard: star !== -1,
      });
    }
    aliases.sort((a, b) => b.prefix.length - a.prefix.length);
    return { baseUrl, aliases };
  } catch {
    return { baseUrl: root, aliases: [] };
  }
}

const RESOLVE_EXTENSIONS = [".tsx", ".ts", ".jsx", ".js", ".mjs", ".json", ".css"];

function resolveFile(candidate: string): string | undefined {
  try {
    if (statSync(candidate).isFile()) return candidate;
  } catch {
    // continue with extensions
  }
  for (const ext of RESOLVE_EXTENSIONS) if (existsSync(candidate + ext)) return candidate + ext;
  for (const ext of RESOLVE_EXTENSIONS) {
    const index = join(candidate, `index${ext}`);
    if (existsSync(index)) return index;
  }
  return undefined;
}

/** Resolves a specifier through tsconfig `paths`; undefined when no alias matches or no file exists. */
export function resolveTsconfigPath(specifier: string, paths: { baseUrl: string; aliases: ReadonlyArray<PathAlias> }): string | undefined {
  for (const alias of paths.aliases) {
    if (alias.wildcard) {
      if (!specifier.startsWith(alias.prefix) || !specifier.endsWith(alias.suffix) || specifier.length < alias.prefix.length + alias.suffix.length) continue;
      const captured = specifier.slice(alias.prefix.length, specifier.length - alias.suffix.length);
      for (const target of alias.targets) {
        const resolved = resolveFile(resolve(paths.baseUrl, target.replace("*", captured)));
        if (resolved) return resolved;
      }
    } else if (specifier === alias.prefix) {
      for (const target of alias.targets) {
        const resolved = resolveFile(resolve(paths.baseUrl, target));
        if (resolved) return resolved;
      }
    }
  }
  return undefined;
}

// node_modules never needs the directive transform; excluding it here keeps Bun's own loader for dependencies.
const SOURCE_FILTER = /^(?!.*[\\/]node_modules[\\/]).*\.(?:tsx|ts|jsx|js|mjs|mts)$/;

/** Bun loader for a path matched by `SOURCE_FILTER`. */
function sourceLoader(path: string): "tsx" | "ts" | "jsx" | "js" {
  if (path.endsWith(".tsx")) return "tsx";
  if (path.endsWith(".ts") || path.endsWith(".mts")) return "ts";
  if (path.endsWith(".jsx")) return "jsx";
  return "js";
}
const NEXT_FILTER = /^(?:next(?:\/.*)?|server-only|client-only)$/;
const SELF_FILTER = /^@yolo-rs\/next(?:\/.*)?$/;

/** Result of `transformModule`: the replacement source for a module in a given environment, or undefined to keep it. */
export interface DirectiveTransform {
  readonly directive: ModuleDirective;
  readonly contents?: string;
  readonly loader: "tsx" | "ts" | "jsx" | "js";
  readonly clientModule: boolean;
  readonly serverActions: ReadonlyArray<string>;
}

/**
 * Applies the directive rules of one environment to a module: rsc + `"use client"` -> client
 * reference stub; client/server + `"use server"` -> server reference stub; rsc + `"use server"` ->
 * source plus `registerServerReference` trailer. Function-level `"use server"` directives are left
 * to the runtime (`@yolo-rs/next/actions` registers them when the module is evaluated).
 */
export function transformModule(source: string, id: string, env: PluginEnv, options: { actionsModule?: string; file?: string } = {}): DirectiveTransform {
  const directive = detectDirective(source);
  const loader = loaderFor(options.file ?? id);
  const base: DirectiveTransform = { directive, loader, clientModule: false, serverActions: [] };
  if (directive === "use-client" && env === "rsc") {
    const exports = moduleExportNames(source, options.file ?? id);
    return { ...base, contents: clientReferenceStub(id, exports), loader: "ts", clientModule: true };
  }
  if (directive === "use-server") {
    const exports = moduleExportNames(source, options.file ?? id);
    const actions = [...exports.names, ...(exports.hasDefault ? ["default"] : [])];
    if (env === "rsc") return { ...base, contents: source + serverReferenceTrailer(id, exports), serverActions: actions };
    return { ...base, contents: serverReferenceStub(id, exports, options.actionsModule), loader: "ts", serverActions: actions };
  }
  return base;
}

/**
 * The Bun plugin of a Yolo Next project for one environment: resolves `next/*`, `server-only` and
 * `client-only` to the shims, applies the `"use client"` / `"use server"` boundary transforms, and
 * honours tsconfig `paths` aliases (`@/*`).
 */
export function yoloNextPlugin(options: YoloNextPluginOptions): BunPlugin {
  const root = resolve(options.root);
  const paths = tsconfigPaths(root);
  const distDir = options.config?.distDir ?? ".yolo";
  return {
    name: `yolo-next:${options.env}`,
    setup(build: PluginBuilder) {
      build.onResolve({ filter: NEXT_FILTER }, (args) => {
        const canonical = canonicalSpecifier(args.path);
        if (canonical === "next") return { path: "next", namespace: EMPTY_NAMESPACE };
        const shim = resolveNextSpecifier(canonical);
        return shim ? { path: shim } : undefined;
      });
      build.onResolve({ filter: SELF_FILTER }, (args) => {
        const self = resolveSelfSpecifier(args.path);
        return self ? { path: self } : undefined;
      });
      build.onLoad({ filter: /.*/, namespace: EMPTY_NAMESPACE }, () => ({ contents: "export default {};\n", loader: "js" }));
      if (paths.aliases.length) {
        const alternatives = paths.aliases.map((alias) => `${alias.prefix.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}${alias.wildcard ? ".*" : ""}${alias.suffix.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}`);
        const filter = new RegExp(`^(?:${alternatives.join("|")})$`);
        build.onResolve({ filter }, (args) => {
          if (args.path.startsWith(".") || isAbsolute(args.path)) return undefined;
          const resolved = resolveTsconfigPath(args.path, paths);
          return resolved ? { path: resolved } : undefined;
        });
      }
      build.onLoad({ filter: SOURCE_FILTER }, (args) => {
        const path = args.path;
        if (args.namespace && args.namespace !== "file") return undefined;
        if (!isAbsolute(path)) return undefined;
        let source: string;
        try {
          source = readFileSync(path, "utf8");
        } catch {
          return undefined;
        }
        // Bun 1.4.6 runtime plugins reject an `undefined` onLoad result for a real file,
        // so files left untouched are passed through with their own loader.
        const passthrough = { contents: source, loader: sourceLoader(path) };
        const posix = toPosix(path);
        if (posix.includes("/node_modules/")) return passthrough;
        const rel = relative(root, path);
        if (rel.startsWith("..") || isAbsolute(rel)) return passthrough;
        const id = toPosix(rel);
        if (id.startsWith(`${distDir.replace(/^\.\//, "")}/`)) return passthrough;
        if (!source.includes("use client") && !source.includes("use server")) return passthrough;
        const result = transformModule(source, id, options.env, { actionsModule: options.actionsModule, file: path });
        if (result.clientModule) options.onClientModule?.(id);
        for (const name of result.serverActions) options.onServerAction?.(id, name);
        if (result.contents === undefined) return passthrough;
        return { contents: result.contents, loader: result.loader };
      });
    },
  };
}

/** Module id of an absolute path relative to a root (`/` separators). */
export function moduleId(root: string, file: string): string {
  return toPosix(relative(resolve(root), resolve(file)));
}

/** Directory of a module id. */
export function moduleDir(root: string, id: string): string {
  return dirname(join(root, id));
}
