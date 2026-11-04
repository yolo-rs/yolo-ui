// SPDX-License-Identifier: MIT OR Apache-2.0
// The `turbopack` key of next.config, applied to Bun.build. Turbopack itself is not linked (the bundler is
// Bun's); its configuration is honoured: `resolveAlias` (plain, list and conditional targets, `pkg/*`
// subpaths), `resolveExtensions`, and `rules` whose loaders map to Bun loaders. The `--turbopack` / `--turbo`
// CLI flags are accepted and reported as no-ops.
import type { BunPlugin } from "bun";
import { existsSync } from "node:fs";
import { dirname, extname, isAbsolute, resolve } from "node:path";
import type { AliasTarget, TurbopackConfig, TurbopackRule, Unsupported } from "./next-config";

export type Layer = "rsc" | "ssr" | "client";

/** Bun loaders a Turbopack rule can map to. */
export type RuleLoader = "text" | "file" | "dataurl" | "json" | "toml" | "svgr" | "wasm";

/** Known webpack/Turbopack loader names and the Bun loader replacing them. */
export const LOADER_MAP: Record<string, RuleLoader> = {
  "raw-loader": "text",
  "@next/raw-loader": "text",
  "text-loader": "text",
  "file-loader": "file",
  "url-loader": "dataurl",
  "json-loader": "json",
  "toml-loader": "toml",
  "wasm-loader": "wasm",
  "@svgr/webpack": "svgr",
  "svgr-loader": "svgr",
};

export interface TurbopackMapping {
  plugin: BunPlugin;
  /** Rules or targets the plugin could not honour. */
  unsupported: Unsupported[];
}

const escapeRegex = (text: string) => text.replace(/[.+*?^${}()|[\]\\]/g, c => `\\${c}`);

/** Conditions tried, in order, for a conditional alias target in one layer. */
export function aliasConditions(layer: Layer): string[] {
  return layer === "client"
    ? ["browser", "import", "module", "default"]
    : layer === "rsc"
      ? ["react-server", "node", "import", "module", "default"]
      : ["node", "import", "module", "default"];
}

/** Pick the target string of an alias for one layer; `undefined` when the target excludes the layer. */
export function pickAliasTarget(target: AliasTarget, layer: Layer): string | string[] | undefined {
  if (typeof target === "string" || Array.isArray(target)) return target;
  for (const condition of aliasConditions(layer)) {
    const value = target[condition];
    if (value !== undefined) return value;
  }
  return undefined;
}

// Convert a glob rule key ("*.svg", "**" + "/*.md", "./src/**" + "/*.yaml") to a RegExp over posix paths.
export function ruleGlobToRegExp(pattern: string): RegExp {
  const glob = pattern.startsWith("./") ? pattern.slice(2) : pattern;
  let source = "";
  for (let i = 0; i < glob.length; i++) {
    const c = glob[i]!;
    if (c === "*") {
      if (glob[i + 1] === "*") {
        source += ".*";
        i++;
        if (glob[i + 1] === "/") i++;
      } else source += "[^/]*";
    } else if (c === "?") source += "[^/]";
    else if (c === "{") {
      const end = glob.indexOf("}", i);
      source += `(?:${glob
        .slice(i + 1, end)
        .split(",")
        .map(escapeRegex)
        .join("|")})`;
      i = end;
    } else source += escapeRegex(c);
  }
  return new RegExp(`(^|/)${source}$`);
}

/** The Bun loader of a Turbopack rule, or `undefined` with a reason when it cannot be mapped. */
export function ruleLoader(rule: TurbopackRule): { loader?: RuleLoader; reason?: string } {
  const first = rule.loaders[0];
  if (!first) return { reason: "no loader" };
  const name = typeof first === "string" ? first : first.loader;
  const loader = LOADER_MAP[name];
  if (!loader) return { reason: `loader ${name} has no Bun equivalent` };
  if (rule.loaders.length > 1) return { loader, reason: `only the first loader (${name}) is applied` };
  return { loader };
}

/** A React component module for an SVG file (what `@svgr/webpack` produces). */
export function svgrModule(svg: string): string {
  const open = /<svg\b([^>]*)>/i.exec(svg);
  const inner = open ? svg.slice(open.index + open[0].length).replace(/<\/svg>\s*$/i, "") : svg;
  const attrs: Record<string, string> = {};
  for (const m of (open?.[1] ?? "").matchAll(/([\w:-]+)\s*=\s*"([^"]*)"/g)) attrs[m[1]!] = m[2]!;
  delete attrs.xmlns;
  return [
    `import { createElement } from "react";`,
    `const attrs = ${JSON.stringify(attrs)};`,
    `const html = ${JSON.stringify(inner.trim())};`,
    `export default function SvgComponent(props) { return createElement("svg", { ...attrs, ...props, dangerouslySetInnerHTML: { __html: html } }); }`,
    `export const ReactComponent = SvgComponent;`,
  ].join("\n");
}

export interface TurbopackPluginOptions {
  root: string;
  layer: Layer;
}

/** Build the plugin applying `turbopack` for one layer. */
export function turbopackPlugin(
  config: TurbopackConfig | undefined,
  options: TurbopackPluginOptions,
): TurbopackMapping {
  const unsupported: Unsupported[] = [];
  const base = config?.root ? resolve(options.root, config.root) : options.root;
  const aliases: { regex: RegExp; name: string; subpath: boolean; target: string | string[] }[] = [];
  for (const [name, raw] of Object.entries(config?.resolveAlias ?? {})) {
    const target = pickAliasTarget(raw, options.layer);
    if (target === undefined) continue;
    const subpath = name.endsWith("/*");
    const bare = subpath ? name.slice(0, -2) : name;
    aliases.push({
      regex: subpath ? new RegExp(`^${escapeRegex(bare)}/(.+)$`) : new RegExp(`^${escapeRegex(name)}$`),
      name: bare,
      subpath,
      target,
    });
  }
  const rules: { regex: RegExp; loader: RuleLoader; key: string }[] = [];
  for (const [key, raw] of Object.entries(config?.rules ?? {})) {
    const rule = Array.isArray(raw) ? raw[0] : raw;
    if (!rule) continue;
    const { loader, reason } = ruleLoader(rule);
    if (reason) unsupported.push({ key: `turbopack.rules[${key}]`, reason });
    if (loader) rules.push({ regex: ruleGlobToRegExp(key), loader, key });
  }
  for (const key of ["moduleIds", "treeShaking", "memoryLimit", "debugIds", "sourceMaps", "conditions"] as const)
    if (config?.[key] !== undefined)
      unsupported.push({ key: `turbopack.${key}`, reason: "Bun.build has its own setting; ignored" });
  const extensions = config?.resolveExtensions;

  const resolveTarget = (target: string, importer: string): string | undefined => {
    const candidates = target.startsWith(".") ? [resolve(base, target)] : isAbsolute(target) ? [target] : [target];
    for (const candidate of candidates) {
      try {
        return Bun.resolveSync(candidate, isAbsolute(candidate) ? dirname(candidate) : dirname(importer) || base);
      } catch {
        if (extensions && !extname(candidate))
          for (const ext of extensions) if (existsSync(candidate + ext)) return candidate + ext;
      }
    }
    return undefined;
  };

  const plugin: BunPlugin = {
    name: `@yolo-rs/fullstack:turbopack:${options.layer}`,
    setup(build) {
      if (aliases.length) {
        build.onResolve({ filter: /.*/ }, args => {
          if (args.namespace !== "file" && args.namespace !== "") return undefined;
          for (const alias of aliases) {
            const m = alias.regex.exec(args.path);
            if (!m) continue;
            const targets = Array.isArray(alias.target) ? alias.target : [alias.target];
            for (const target of targets) {
              const spec = alias.subpath ? `${target.replace(/\/\*$/, "")}/${m[1]}` : target;
              const path = resolveTarget(spec, args.importer);
              if (path) return { path };
            }
            unsupported.push({
              key: `turbopack.resolveAlias[${alias.name}]`,
              reason: `could not resolve ${JSON.stringify(alias.target)}`,
            });
          }
          return undefined;
        });
      }
      if (extensions?.length) {
        build.onResolve({ filter: /^\.{1,2}\// }, args => {
          if (extname(args.path)) return undefined;
          const abs = resolve(dirname(args.importer), args.path);
          for (const ext of extensions) if (existsSync(abs + ext)) return { path: abs + ext };
          return undefined;
        });
      }
      for (const rule of rules) {
        if (rule.loader === "svgr") {
          build.onLoad({ filter: rule.regex }, async args => ({
            contents: svgrModule(await Bun.file(args.path).text()),
            loader: "js",
            resolveDir: dirname(args.path),
          }));
        } else {
          const loader = rule.loader;
          build.onLoad({ filter: rule.regex }, async args => ({
            contents: new Uint8Array(await Bun.file(args.path).arrayBuffer()),
            loader: loader as Bun.Loader,
          }));
        }
      }
    },
  };
  return { plugin, unsupported };
}

/** Turbopack CLI flags (`--turbopack`, `--turbo`, `--webpack`) accepted for compatibility. */
export const BUNDLER_FLAGS = ["--turbopack", "--turbo", "--webpack", "--experimental-turbo"] as const;

/** Strip the bundler flags from argv and report whether any was given. */
export function stripBundlerFlags(argv: string[]): { argv: string[]; turbopack: boolean; webpack: boolean } {
  const kept: string[] = [];
  let turbopack = false;
  let webpack = false;
  for (const arg of argv) {
    if (arg === "--turbopack" || arg === "--turbo" || arg === "--experimental-turbo") turbopack = true;
    else if (arg === "--webpack") webpack = true;
    else kept.push(arg);
  }
  return { argv: kept, turbopack, webpack };
}
