// SPDX-License-Identifier: Apache-2.0
// @yolo-rs/next/turbopack: `turbopack.{rules,resolveAlias,resolveExtensions}` + compiler.define/env ->
// Bun bundler plan, real Bun plugins for the translated loaders, bunfig.toml and yolo.bundler.ts emitters.

import type { BunPlugin, PluginBuilder } from "bun";
import { existsSync } from "node:fs";
import { dirname, isAbsolute, resolve } from "node:path";

import type { BunBundlerPlan, BunLoader, BunPluginKind, BunPluginSpec, FeatureStatus, NextCompatConfig, ResolvedYoloConfig, Support, TurbopackLoader, TurbopackRule } from "./types.ts";

/** Default resolve extensions of Turbopack (Bun order). */
export const DEFAULT_EXTENSIONS: ReadonlyArray<string> = [".tsx", ".ts", ".jsx", ".js", ".mjs", ".json"];

/** Loader mapping of one webpack/Turbopack loader package. */
export interface LoaderMapping {
  readonly kind: BunPluginKind;
  readonly loader?: BunLoader;
  readonly support: Support;
  readonly note: string;
}

const LOADER_MAP: ReadonlyArray<readonly [RegExp, LoaderMapping]> = [
  [/^@svgr\/webpack$/, { kind: "svg-react", loader: "tsx", support: "mapped", note: "SVG files become React components (default export) with `src` data URL" }],
  [/^(?:svg-inline-loader|raw-loader|asset\/source|graphql-tag\/loader|glslify-loader)$/, { kind: "text", loader: "text", support: "mapped", note: "imported as a string" }],
  [/^yaml-loader$/, { kind: "loader-override", loader: "yaml", support: "mapped", note: "Bun parses YAML natively" }],
  [/^(?:@mdx-js\/loader|@next\/mdx)$/, { kind: "markdown", loader: "tsx", support: "partial", note: "Markdown rendered with Bun.markdown; MDX expressions and imports are not evaluated" }],
  [/^(?:file-loader|url-loader|asset\/resource|asset\/inline|asset)$/, { kind: "loader-override", loader: "file", support: "mapped", note: "copied to the output, import yields its URL" }],
  [/^toml-loader$/, { kind: "loader-override", loader: "toml", support: "mapped", note: "Bun parses TOML natively" }],
  [/^json5-loader$/, { kind: "loader-override", loader: "jsonc", support: "partial", note: "parsed as JSONC (comments and trailing commas)" }],
  [/^sass-loader$/, { kind: "unsupported", support: "unsupported", note: "compile Sass ahead of time (sass CLI or bun-plugin-sass) and import the CSS" }],
  [/^string-replace-loader$/, { kind: "unsupported", support: "unsupported", note: "use `define` or a custom Bun plugin for source replacements" }],
  [/^babel-loader$/, { kind: "unsupported", support: "unsupported", note: "Bun transpiles TS/JSX natively; port Babel plugins to OXC or Bun macros" }],
];

/** Maps a webpack/Turbopack loader package to a Bun plugin kind. */
export function mapTurbopackLoader(loader: string): LoaderMapping {
  for (const [pattern, mapping] of LOADER_MAP) if (pattern.test(loader)) return mapping;
  return { kind: "unsupported", support: "unsupported", note: `loader ${loader} has no Bun equivalent; write a Bun plugin` };
}

const TYPE_LOADERS: Record<string, BunLoader> = { raw: "text", text: "text", bytes: "file", json: "json", css: "css", wasm: "wasm", asset: "file", ecmascript: "js", typescript: "ts" };

/** Default Bun loader for a file extension (without dot). */
export function loaderForExtension(ext: string): BunLoader | undefined {
  const table: Record<string, BunLoader> = {
    js: "js", mjs: "js", cjs: "js", jsx: "jsx", ts: "ts", mts: "ts", cts: "ts", tsx: "tsx",
    json: "json", jsonc: "jsonc", toml: "toml", yaml: "yaml", yml: "yaml", txt: "text", md: "text",
    css: "css", html: "html", wasm: "wasm", node: "napi",
    svg: "file", png: "file", jpg: "file", jpeg: "file", gif: "file", webp: "file", avif: "file", ico: "file",
    woff: "file", woff2: "file", ttf: "file", otf: "file", mp4: "file", webm: "file", mp3: "file",
  };
  return table[ext.toLowerCase()];
}

/** Converts a Turbopack rule glob into a JavaScript regex source (`*.svg` -> `\.svg$`, `src/icons/*.svg` keeps the directory). */
export function globToRegexSource(glob: string): string {
  let pattern = glob.replace(/^\.\//, "").replace(/^\/+/, "");
  while (pattern.startsWith("**/")) pattern = pattern.slice(3);
  if (!pattern.includes("/")) {
    // Basename glob: a leading `*` already matches any prefix, otherwise anchor at a path boundary.
    if (pattern.startsWith("*")) {
      const body = globBody(pattern.replace(/^\*+/, ""));
      return body ? `${body}$` : ".*$";
    }
    return `(?:^|/)${globBody(pattern)}$`;
  }
  const parts = pattern.split("/");
  const body = parts
    .map((part, index) => (part === "**" ? (index === parts.length - 1 ? ".*" : "(?:.*/)?") : globBody(part)))
    .join("/")
    .replace(/\(\?:\.\*\/\)\?\//g, "(?:.*/)?");
  return `(?:^|/)${body}$`;
}

function globBody(part: string): string {
  let out = "";
  for (let i = 0; i < part.length; i += 1) {
    const ch = part.charAt(i);
    if (ch === "*") out += "[^/]*";
    else if (ch === "?") out += "[^/]";
    else if (ch === "{") {
      const end = part.indexOf("}", i);
      if (end === -1) out += "\\{";
      else {
        out += `(?:${part
          .slice(i + 1, end)
          .split(",")
          .map((alt) => alt.replace(/[.*+?^${}()|[\]\\]/g, "\\$&"))
          .join("|")})`;
        i = end;
      }
    } else out += ch.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  }
  return out;
}

function extensionOfGlob(glob: string): string | undefined {
  const match = /\.([A-Za-z0-9]+)$/.exec(glob);
  return match?.[1];
}

function loaderName(loader: string | TurbopackLoader): { name: string; options: Record<string, unknown> } {
  return typeof loader === "string" ? { name: loader, options: {} } : { name: loader.loader, options: loader.options ?? {} };
}

/** Translates the Turbopack section (plus `compiler.define`, `env`, `serverExternalPackages`) into a Bun bundler plan. */
export function translateTurbopack(config: NextCompatConfig): BunBundlerPlan {
  const turbopack = config.turbopack ?? {};
  const plugins: BunPluginSpec[] = [];
  const loaders: Record<string, BunLoader> = {};
  const unsupported: FeatureStatus[] = [];
  for (const [glob, ruleOrRules] of Object.entries(turbopack.rules ?? {})) {
    const rules: TurbopackRule[] = Array.isArray(ruleOrRules) ? ruleOrRules : [ruleOrRules];
    const filter = globToRegexSource(glob);
    const ext = extensionOfGlob(glob);
    for (const rule of rules) {
      let handled = false;
      for (const raw of rule.loaders ?? []) {
        const { name, options } = loaderName(raw);
        const mapping = mapTurbopackLoader(name);
        const spec: BunPluginSpec = {
          name: `yolo:${mapping.kind}:${ext ?? "any"}`,
          kind: mapping.kind,
          filter,
          originLoader: name,
          loader: mapping.loader,
          options: { ...options, ...(rule.as ? { as: rule.as } : {}), ...(rule.condition !== undefined ? { condition: rule.condition } : {}) },
        };
        plugins.push(spec);
        if (mapping.kind === "unsupported") {
          unsupported.push({ feature: `turbopack.rules["${glob}"] ${name}`, support: "unsupported", note: mapping.note, target: "bun" });
        } else {
          handled = true;
          if (ext && (mapping.kind === "text" || mapping.kind === "loader-override") && mapping.loader) loaders[`.${ext}`] = mapping.loader;
          if (mapping.support === "partial") unsupported.push({ feature: `turbopack.rules["${glob}"] ${name}`, support: "partial", note: mapping.note, target: "bun" });
        }
      }
      if (rule.type) {
        const loader = TYPE_LOADERS[rule.type];
        if (loader) {
          plugins.push({ name: `yolo:loader-override:${ext ?? "any"}`, kind: "loader-override", filter, loader, options: { type: rule.type } });
          if (ext) loaders[`.${ext}`] = loader;
          if (rule.type === "bytes") unsupported.push({ feature: `turbopack.rules["${glob}"].type = "bytes"`, support: "partial", note: "Bun has no bytes loader; the file is copied and imported by URL", target: "bun" });
        } else {
          unsupported.push({ feature: `turbopack.rules["${glob}"].type = "${rule.type}"`, support: "unsupported", note: "module type has no Bun loader", target: "bun" });
        }
      } else if (!handled && !(rule.loaders ?? []).length) {
        unsupported.push({ feature: `turbopack.rules["${glob}"]`, support: "unsupported", note: "rule without loaders or type", target: "bun" });
      }
    }
  }
  const aliases: Record<string, string> = {};
  const browserAliases: Record<string, string> = {};
  const emptyModules: string[] = [];
  for (const [name, target] of Object.entries(turbopack.resolveAlias ?? {})) {
    if (target === false) {
      emptyModules.push(name);
    } else if (typeof target === "string") {
      aliases[name] = target;
    } else if (Array.isArray(target)) {
      const first = target.find((t) => typeof t === "string");
      if (first) aliases[name] = first;
      unsupported.push({ feature: `turbopack.resolveAlias["${name}"]`, support: "partial", note: "array aliases resolve to the first entry only", target: "bun" });
    } else if (typeof target === "object" && target !== null) {
      for (const [condition, value] of Object.entries(target)) {
        if (condition === "browser") {
          if (value === false) emptyModules.push(name);
          else browserAliases[name] = value;
        } else if (condition === "default" || condition === "import" || condition === "require" || condition === "node") {
          if (value === false) emptyModules.push(name);
          else aliases[name] ??= value;
        } else {
          unsupported.push({ feature: `turbopack.resolveAlias["${name}"].${condition}`, support: "partial", note: "custom resolve condition ignored", target: "bun" });
        }
      }
    }
  }
  const define: Record<string, string> = {};
  for (const [key, value] of Object.entries(config.compiler?.define ?? {})) define[key] = value;
  for (const [key, value] of Object.entries(config.compiler?.defineServer ?? {})) define[key] = value;
  for (const [key, value] of Object.entries(config.env ?? {})) if (value !== undefined) define[`process.env.${key}`] = JSON.stringify(value);
  return {
    plugins,
    loaders,
    aliases,
    browserAliases,
    emptyModules: [...new Set(emptyModules)],
    extensions: turbopack.resolveExtensions && turbopack.resolveExtensions.length ? [...turbopack.resolveExtensions] : [...DEFAULT_EXTENSIONS],
    root: turbopack.root,
    define,
    external: [...(config.serverExternalPackages ?? [])],
    unsupported,
  };
}

// ---------------------------------------------------------------------------
// Bun plugin factories
// ---------------------------------------------------------------------------

const EMPTY_NAMESPACE = "yolo-empty";

function resolveTarget(target: string, root: string, importer: string | undefined): string {
  if (target.startsWith(".") || isAbsolute(target)) {
    const base = target.startsWith(".") ? root : "";
    const abs = base ? resolve(base, target) : target;
    try {
      return Bun.resolveSync(abs, dirname(abs));
    } catch {
      return abs;
    }
  }
  try {
    return Bun.resolveSync(target, importer ? dirname(importer) : root);
  } catch {
    return target;
  }
}

/**
 * `resolveAlias` plugin: exact and `name/` prefix aliases, browser-only aliases when the build target
 * is `browser`, and `false` aliases mapped to an empty virtual module.
 */
export function aliasPlugin(aliases: Record<string, string>, browserAliases: Record<string, string> = {}, emptyModules: ReadonlyArray<string> = [], root: string = process.cwd()): BunPlugin {
  return {
    name: "yolo:alias-resolve",
    setup(build: PluginBuilder) {
      const empties = new Set(emptyModules);
      const names = [...new Set([...Object.keys(aliases), ...Object.keys(browserAliases), ...emptyModules])];
      if (names.length === 0) return;
      const wildcard = names.filter((n) => n.endsWith("/*")).map((n) => n.slice(0, -2));
      const exact = names.filter((n) => !n.endsWith("/*"));
      const escape = (text: string): string => text.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
      const alternatives = [...exact.map((n) => `${escape(n)}(?:/.*)?`), ...wildcard.map((n) => `${escape(n)}/.+`)];
      const filter = new RegExp(`^(?:${alternatives.join("|")})$`);
      build.onResolve({ filter }, (args) => {
        const isBrowser = build.config.target === "browser";
        const table = isBrowser ? { ...aliases, ...browserAliases } : aliases;
        for (const name of exact) {
          if (args.path !== name && !args.path.startsWith(`${name}/`)) continue;
          if (empties.has(name) && !(isBrowser ? browserAliases[name] : aliases[name])) return { path: name, namespace: EMPTY_NAMESPACE };
          const target = table[name];
          if (!target) continue;
          const rest = args.path.slice(name.length);
          return { path: resolveTarget(target + rest, root, args.importer) };
        }
        for (const prefix of wildcard) {
          if (!args.path.startsWith(`${prefix}/`)) continue;
          const target = table[`${prefix}/*`];
          if (!target) continue;
          const rest = args.path.slice(prefix.length + 1);
          return { path: resolveTarget(target.replace("*", rest), root, args.importer) };
        }
        return undefined;
      });
      build.onLoad({ filter: /.*/, namespace: EMPTY_NAMESPACE }, () => ({ contents: "export default {};\n", loader: "js" }));
    },
  };
}

const SVG_ATTR_NAMES: Record<string, string> = { class: "className", for: "htmlFor", "xlink:href": "xlinkHref", "xml:space": "xmlSpace", "xmlns:xlink": "xmlnsXlink", tabindex: "tabIndex" };

/** Parses the attributes of the root `<svg ...>` tag with a small tokenizer (no backtracking regexes). */
export function parseSvgRoot(svg: string): { attrs: Record<string, string>; inner: string } {
  const start = svg.indexOf("<svg");
  if (start === -1) return { attrs: {}, inner: svg };
  let i = start + 4;
  const attrs: Record<string, string> = {};
  const isSpace = (ch: string): boolean => ch === " " || ch === "\n" || ch === "\t" || ch === "\r";
  while (i < svg.length) {
    while (i < svg.length && isSpace(svg.charAt(i))) i += 1;
    const ch = svg.charAt(i);
    if (ch === ">" || ch === "" || svg.startsWith("/>", i)) break;
    let name = "";
    while (i < svg.length && !isSpace(svg.charAt(i)) && svg.charAt(i) !== "=" && svg.charAt(i) !== ">" && !svg.startsWith("/>", i)) {
      name += svg.charAt(i);
      i += 1;
    }
    while (i < svg.length && isSpace(svg.charAt(i))) i += 1;
    let value = "";
    if (svg.charAt(i) === "=") {
      i += 1;
      while (i < svg.length && isSpace(svg.charAt(i))) i += 1;
      const quote = svg.charAt(i);
      if (quote === '"' || quote === "'") {
        const end = svg.indexOf(quote, i + 1);
        value = svg.slice(i + 1, end === -1 ? svg.length : end);
        i = end === -1 ? svg.length : end + 1;
      } else {
        while (i < svg.length && !isSpace(svg.charAt(i)) && svg.charAt(i) !== ">") {
          value += svg.charAt(i);
          i += 1;
        }
      }
    }
    if (name) attrs[reactAttrName(name)] = value;
  }
  const selfClosing = svg.startsWith("/>", i);
  const open = svg.indexOf(">", i);
  const close = svg.lastIndexOf("</svg>");
  const inner = selfClosing || open === -1 || close === -1 || close < open ? "" : svg.slice(open + 1, close);
  return { attrs, inner: inner.trim() };
}

function reactAttrName(name: string): string {
  const mapped = SVG_ATTR_NAMES[name.toLowerCase()] ?? SVG_ATTR_NAMES[name];
  if (mapped) return mapped;
  if (name.startsWith("data-") || name.startsWith("aria-")) return name;
  return name.replace(/[-:]([a-z])/g, (_m, c: string) => c.toUpperCase());
}

/** Module source of an SVG React component (default export) plus `src` data URL. */
export function svgComponentSource(svg: string): string {
  const { attrs, inner } = parseSvgRoot(svg);
  const dataUrl = `data:image/svg+xml,${encodeURIComponent(svg).replace(/%20/g, " ").replace(/%3D/g, "=").replace(/%3A/g, ":").replace(/%2F/g, "/")}`;
  return [
    'import { createElement } from "react";',
    `const attrs = ${JSON.stringify(attrs)};`,
    `const inner = ${JSON.stringify(inner)};`,
    `export const src = ${JSON.stringify(dataUrl)};`,
    "export default function SvgIcon(props) {",
    '  return createElement("svg", { ...attrs, ...props, dangerouslySetInnerHTML: { __html: inner } });',
    "}",
    "",
  ].join("\n");
}

/** `@svgr/webpack` replacement: SVG files export a React component (default) and `src`. */
export function svgReactPlugin(filter: RegExp): BunPlugin {
  return {
    name: "yolo:svg-react",
    setup(build: PluginBuilder) {
      build.onLoad({ filter }, async (args) => ({ contents: svgComponentSource(await Bun.file(args.path).text()), loader: "tsx" }));
    },
  };
}

/** `raw-loader` replacement: files matching `filter` are imported as strings. */
export function textPlugin(filter: RegExp): BunPlugin {
  return {
    name: "yolo:text",
    setup(build: PluginBuilder) {
      build.onLoad({ filter }, async (args) => ({ contents: await Bun.file(args.path).text(), loader: "text" }));
    },
  };
}

/** Renders Markdown to HTML with `Bun.markdown` when available, else a `<pre>` fallback. */
export function markdownToHtml(markdown: string): string {
  const api = (Bun as unknown as { markdown?: { html?: (s: string) => string } }).markdown;
  if (api && typeof api.html === "function") {
    try {
      return api.html(markdown);
    } catch {
      // fall through
    }
  }
  return `<pre>${markdown.replace(/[&<>]/g, (ch) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;" })[ch] ?? ch)}</pre>`;
}

/** Module source of a Markdown React component (default export) plus `html`. */
export function markdownComponentSource(markdown: string): string {
  return [
    'import { createElement } from "react";',
    `export const html = ${JSON.stringify(markdownToHtml(markdown))};`,
    `export const markdown = ${JSON.stringify(markdown)};`,
    "export default function Markdown(props) {",
    '  return createElement("div", { ...props, dangerouslySetInnerHTML: { __html: html } });',
    "}",
    "",
  ].join("\n");
}

/** `@mdx-js/loader` / `@next/mdx` replacement: Markdown files export a React component and `html`. */
export function markdownPlugin(filter: RegExp): BunPlugin {
  return {
    name: "yolo:markdown",
    setup(build: PluginBuilder) {
      build.onLoad({ filter }, async (args) => ({ contents: markdownComponentSource(await Bun.file(args.path).text()), loader: "tsx" }));
    },
  };
}

/** Forces a Bun loader for files matching `filter` (yaml, toml, jsonc, file, text, ...). */
export function loaderOverridePlugin(filter: RegExp, loader: BunLoader): BunPlugin {
  return {
    name: `yolo:loader-override:${loader}`,
    setup(build: PluginBuilder) {
      build.onLoad({ filter }, async (args) => {
        const file = Bun.file(args.path);
        if (loader === "file" || loader === "wasm" || loader === "napi") return { contents: await file.arrayBuffer(), loader };
        return { contents: await file.text(), loader };
      });
    },
  };
}

/** Instantiates the Bun plugins of a bundler plan (alias plugin first, then one plugin per rule). */
export function yoloBundlerPlugins(plan: BunBundlerPlan): BunPlugin[] {
  const plugins: BunPlugin[] = [];
  if (Object.keys(plan.aliases).length || Object.keys(plan.browserAliases).length || plan.emptyModules.length) {
    plugins.push(aliasPlugin(plan.aliases, plan.browserAliases, plan.emptyModules, plan.root ?? process.cwd()));
  }
  for (const spec of plan.plugins) {
    const filter = new RegExp(spec.filter);
    switch (spec.kind) {
      case "svg-react":
        plugins.push(svgReactPlugin(filter));
        break;
      case "text":
        plugins.push(textPlugin(filter));
        break;
      case "markdown":
        plugins.push(markdownPlugin(filter));
        break;
      case "loader-override":
        if (spec.loader) plugins.push(loaderOverridePlugin(filter, spec.loader));
        break;
      default:
        break;
    }
  }
  return plugins;
}

const tomlString = (value: string): string => JSON.stringify(value);

/** Emits `bunfig.toml` (`[serve.static]` plugins/env/sourcemap, `[install]`, `[test]`). */
export function bunfigToml(plan: BunBundlerPlan, config: Pick<ResolvedYoloConfig, "publicEnvPrefixes">): string {
  const prefix = config.publicEnvPrefixes[0] ?? "NEXT_PUBLIC_";
  const lines = [
    "# Generated by @yolo-rs/next from next.config.* (Turbopack -> Bun).",
    "",
    "[serve.static]",
    'plugins = ["./yolo.bundler.ts"]',
    `env = ${tomlString(`${prefix}*`)}`,
    'sourcemap = "linked"',
  ];
  const defines = Object.entries(plan.define);
  if (defines.length) {
    lines.push("", "[serve.static.define]");
    for (const [key, value] of defines) lines.push(`${tomlString(key)} = ${tomlString(value)}`);
  }
  const loaders = Object.entries(plan.loaders);
  if (loaders.length) {
    lines.push("", "[serve.static.loader]");
    for (const [ext, loader] of loaders) lines.push(`${tomlString(ext)} = ${tomlString(loader)}`);
  }
  lines.push("", "[install]", "exact = true", "", "[test]", 'root = "./tests"', "");
  return lines.join("\n");
}

const regexLiteral = (source: string): string => `/${source.replace(/\//g, "\\/")}/`;

/** Emits `yolo.bundler.ts`: the `BunPlugin[]` of the plan built from the `@yolo-rs/next/turbopack` factories. */
export function bundlerPluginsTs(plan: BunBundlerPlan): string {
  const calls: string[] = [];
  const used = new Set<string>();
  if (Object.keys(plan.aliases).length || Object.keys(plan.browserAliases).length || plan.emptyModules.length) {
    used.add("aliasPlugin");
    calls.push(`  aliasPlugin(${JSON.stringify(plan.aliases)}, ${JSON.stringify(plan.browserAliases)}, ${JSON.stringify(plan.emptyModules)}, import.meta.dir),`);
  }
  for (const spec of plan.plugins) {
    const filter = regexLiteral(spec.filter);
    switch (spec.kind) {
      case "svg-react":
        used.add("svgReactPlugin");
        calls.push(`  svgReactPlugin(${filter}),`);
        break;
      case "text":
        used.add("textPlugin");
        calls.push(`  textPlugin(${filter}),`);
        break;
      case "markdown":
        used.add("markdownPlugin");
        calls.push(`  markdownPlugin(${filter}),`);
        break;
      case "loader-override":
        if (spec.loader) {
          used.add("loaderOverridePlugin");
          calls.push(`  loaderOverridePlugin(${filter}, ${JSON.stringify(spec.loader)}),`);
        }
        break;
      default:
        calls.push(`  // unsupported: ${spec.originLoader ?? spec.name} (${spec.filter})`);
        break;
    }
  }
  const imports = [...used].sort().join(", ");
  return [
    "// Generated by @yolo-rs/next from next.config.* (turbopack.rules / resolveAlias). Do not edit.",
    'import type { BunPlugin } from "bun";',
    imports ? `import { ${imports} } from "@yolo-rs/next/turbopack";` : "",
    "",
    "export const plugins: BunPlugin[] = [",
    ...calls,
    "];",
    "",
    "/** Single plugin composing every translated rule (bunfig `[serve.static] plugins`). */",
    "const bundler: BunPlugin = {",
    '  name: "yolo-next-bundler",',
    "  setup(build) {",
    "    for (const plugin of plugins) plugin.setup(build);",
    "  },",
    "};",
    "",
    "export default bundler;",
    "",
  ]
    .filter((line, index, all) => !(line === "" && all[index - 1] === ""))
    .join("\n");
}

/** True when the plan's root exists on disk (used to decide whether alias targets can be resolved). */
export function planRootExists(plan: BunBundlerPlan): boolean {
  return plan.root !== undefined && existsSync(plan.root);
}
