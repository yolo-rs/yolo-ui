// SPDX-License-Identifier: Apache-2.0
// @yolo-rs/next/migrate: project analysis (feature counts over the sources, read once) and the
// Next.js -> @yolo-rs/next migration (package.json, lockfiles, proxy rename, yolo.config.ts, server
// and build entries, bunfig, bundler plugins, route manifest, tsconfig, .gitignore, report).

import { cpSync, existsSync, mkdirSync, readdirSync, readFileSync, rmSync, statSync, writeFileSync } from "node:fs";
import { basename, dirname, join, relative, resolve } from "node:path";

import { bunfigToml, bundlerPluginsTs } from "./turbopack.ts";
import { defineYoloConfig, loadNextConfig, resolveYoloConfig, serializeYoloConfig } from "./config.ts";
import { parseJsonc, shimSpecifierFor } from "./plugin.ts";
import { scanAppDir } from "./router.ts";
import { detectDirective } from "./segments.ts";
import type { Diagnostic, ResolvedYoloConfig, RouteManifest } from "./types.ts";

const toPosix = (path: string): string => path.replace(/\\/g, "/");
const SKIP_DIRS = new Set(["node_modules", ".git", ".next", ".yolo", ".turbo", ".vercel", "dist", "build", "out", "coverage", ".cache", "storybook-static"]);
const SOURCE_EXT = /\.(?:tsx|ts|jsx|js|mjs|mts|cjs)$/;
const I18N_DIRS = new Set(["messages", "locales", "i18n", "lang", "translations", "dictionaries"]);

/** Feature counts of a project (used by the report and `yolo-next analyze`). */
export interface ProjectCounts {
  readonly routes: number;
  readonly pages: number;
  readonly routeHandlers: number;
  readonly metadataRoutes: number;
  readonly legacyPages: number;
  readonly legacyApiRoutes: number;
  readonly dynamicRoutes: number;
  readonly parallelSlots: number;
  readonly interceptingRoutes: number;
  readonly serverActionFiles: number;
  readonly inlineServerActions: number;
  readonly useClientFiles: number;
  readonly useCacheFiles: number;
  readonly forceDynamic: number;
  readonly isrPages: number;
  readonly generateStaticParams: number;
  readonly nextOgUsage: number;
  readonly nextThirdPartiesUsage: number;
  readonly nextFontUsage: number;
  readonly nextImageUsage: number;
  readonly i18nMessageDirs: number;
  readonly sourceFiles: number;
  readonly sourceLines: number;
}

/** Result of `analyzeProject`. */
export interface ProjectAnalysis {
  readonly root: string;
  readonly name: string;
  readonly nextVersion?: string;
  readonly configFile?: string;
  readonly manifest: RouteManifest;
  readonly counts: ProjectCounts;
  readonly i18nDirs: string[];
  readonly diagnostics: Diagnostic[];
}

interface SourceFacts {
  readonly file: string;
  readonly source: string;
}

function* sourceFiles(root: string): Generator<SourceFacts> {
  const visit = function* (dir: string, depth: number): Generator<SourceFacts> {
    let dirents;
    try {
      dirents = readdirSync(dir, { withFileTypes: true });
    } catch {
      return;
    }
    for (const dirent of dirents) {
      if (dirent.isDirectory()) {
        if (SKIP_DIRS.has(dirent.name) || (dirent.name.startsWith(".") && dirent.name !== ".")) continue;
        if (depth < 16) yield* visit(join(dir, dirent.name), depth + 1);
        continue;
      }
      if (!dirent.isFile() || !SOURCE_EXT.test(dirent.name) || dirent.name.endsWith(".d.ts")) continue;
      const abs = join(dir, dirent.name);
      try {
        yield { file: toPosix(relative(root, abs)), source: readFileSync(abs, "utf8") };
      } catch {
        // unreadable file
      }
    }
  };
  yield* visit(root, 0);
}

function findI18nDirs(root: string): string[] {
  const out: string[] = [];
  const visit = (dir: string, depth: number): void => {
    let dirents;
    try {
      dirents = readdirSync(dir, { withFileTypes: true });
    } catch {
      return;
    }
    for (const dirent of dirents) {
      if (!dirent.isDirectory() || SKIP_DIRS.has(dirent.name) || dirent.name.startsWith(".")) continue;
      const abs = join(dir, dirent.name);
      if (I18N_DIRS.has(dirent.name.toLowerCase())) {
        let hasJson = false;
        try {
          hasJson = readdirSync(abs).some((f) => f.endsWith(".json") || f.endsWith(".yaml") || f.endsWith(".yml"));
        } catch {
          hasJson = false;
        }
        if (hasJson) out.push(toPosix(relative(root, abs)));
      }
      if (depth < 4) visit(abs, depth + 1);
    }
  };
  visit(root, 0);
  return out;
}

function packageFacts(root: string): { name?: string; nextVersion?: string } {
  try {
    const pkg = JSON.parse(readFileSync(join(root, "package.json"), "utf8")) as { name?: string; dependencies?: Record<string, string>; devDependencies?: Record<string, string> };
    return { name: pkg.name, nextVersion: pkg.dependencies?.next ?? pkg.devDependencies?.next };
  } catch {
    return {};
  }
}

/** Analyzes a project without writing anything: route manifest plus feature counts over the sources (each file read once). */
export function analyzeProject(root: string): ProjectAnalysis {
  const absRoot = resolve(root);
  const manifest = scanAppDir(absRoot);
  const facts = packageFacts(absRoot);
  let serverActionFiles = 0;
  let inlineServerActions = 0;
  let useClientFiles = 0;
  let useCacheFiles = 0;
  let forceDynamic = 0;
  let isrPages = 0;
  let generateStaticParams = 0;
  let nextOgUsage = 0;
  let nextThirdPartiesUsage = 0;
  let nextFontUsage = 0;
  let nextImageUsage = 0;
  let sourceCount = 0;
  let sourceLines = 0;
  for (const { source } of sourceFiles(absRoot)) {
    sourceCount += 1;
    sourceLines += source.split("\n").length;
    const directive = detectDirective(source);
    if (directive === "use-server") serverActionFiles += 1;
    if (directive === "use-client") useClientFiles += 1;
    if (directive === "use-cache" || /^\s*["']use cache(?::[^"']*)?["']/m.test(source)) useCacheFiles += 1;
    if (directive !== "use-server") {
      const inline = source.match(/^\s+["']use server["'];?\s*$/gm);
      if (inline) inlineServerActions += inline.length;
    }
    if (/\bexport\s+const\s+dynamic\s*(?::[^=]*)?=\s*["']force-dynamic["']/.test(source)) forceDynamic += 1;
    const revalidate = /\bexport\s+const\s+revalidate\s*(?::[^=]*)?=\s*(\d+)/.exec(source);
    if (revalidate && Number(revalidate[1]) > 0) isrPages += 1;
    if (/\bexport\s+(?:async\s+)?function\s+generateStaticParams\b|\bexport\s+const\s+generateStaticParams\b/.test(source)) generateStaticParams += 1;
    if (/["']next\/og["']/.test(source)) nextOgUsage += 1;
    if (/["']@next\/third-parties(?:\/[^"']*)?["']/.test(source)) nextThirdPartiesUsage += 1;
    if (/["']next\/font\/(?:google|local)["']/.test(source)) nextFontUsage += 1;
    if (/["']next\/(?:legacy\/)?image["']/.test(source)) nextImageUsage += 1;
  }
  const i18nDirs = findI18nDirs(absRoot);
  const entries = manifest.entries;
  const counts: ProjectCounts = {
    routes: entries.length,
    pages: entries.filter((e) => e.kind === "page").length,
    routeHandlers: entries.filter((e) => e.kind === "route-handler").length,
    metadataRoutes: entries.filter((e) => e.kind === "metadata").length,
    legacyPages: entries.filter((e) => e.legacyPages && e.kind === "page").length,
    legacyApiRoutes: entries.filter((e) => e.legacyPages && e.kind === "route-handler").length,
    dynamicRoutes: entries.filter((e) => e.params.length > 0).length,
    parallelSlots: entries.filter((e) => e.slot).length,
    interceptingRoutes: entries.filter((e) => e.intercepting).length,
    serverActionFiles,
    inlineServerActions,
    useClientFiles,
    useCacheFiles,
    forceDynamic,
    isrPages,
    generateStaticParams,
    nextOgUsage,
    nextThirdPartiesUsage,
    nextFontUsage,
    nextImageUsage,
    i18nMessageDirs: i18nDirs.length,
    sourceFiles: sourceCount,
    sourceLines,
  };
  return {
    root: toPosix(absRoot),
    name: facts.name ?? basename(absRoot),
    nextVersion: facts.nextVersion,
    configFile: ["next.config.ts", "next.config.mts", "next.config.mjs", "next.config.js", "yolo.config.ts"].find((f) => existsSync(join(absRoot, f))),
    manifest,
    counts,
    i18nDirs,
    diagnostics: [...manifest.diagnostics],
  };
}

// ---------------------------------------------------------------------------
// Migration
// ---------------------------------------------------------------------------

/** One planned or applied change. */
export interface MigrationChange {
  readonly kind: "create" | "modify" | "delete" | "rename";
  readonly path: string;
  readonly from?: string;
  readonly contents?: string;
  readonly note?: string;
}

/** Options of `migrateProject`. */
export interface MigrateOptions {
  readonly root: string;
  /** Destination of `mode: "copy"` (default `<root>-yolo`). */
  readonly out?: string;
  readonly mode?: "in-place" | "copy";
  readonly dryRun?: boolean;
  /** Rewrite `next/*` imports to `@yolo-rs/next/shims/*` (default false: the plugin resolves them). */
  readonly rewriteImports?: boolean;
  readonly log?: (message: string) => void;
}

/** Result of `migrateProject`. */
export interface MigrateResult {
  readonly root: string;
  readonly out: string;
  readonly mode: "in-place" | "copy";
  readonly dryRun: boolean;
  readonly changes: MigrationChange[];
  readonly written: string[];
  readonly report: string;
  readonly counts: ProjectCounts;
  readonly manifest: RouteManifest;
  readonly config: ResolvedYoloConfig;
  readonly diagnostics: Diagnostic[];
}

const REMOVED_DEPENDENCIES: ReadonlyArray<readonly [string, string]> = [
  ["next", "replaced by @yolo-rs/next"],
  ["eslint-config-next", "next lint is gone; use oxlint/biome"],
  ["@next/font", "next/font is shimmed by @yolo-rs/next/shims/font-google and font-local"],
  ["@next/bundle-analyzer", "use `bun build --metafile`"],
  ["@next/third-parties", "shim the scripts you use with next/script (see MIGRATION-REPORT.md)"],
  ["@next/eslint-plugin-next", "next lint is gone"],
  ["@next/mdx", "Markdown is handled by the yolo:markdown Bun plugin"],
];

const LOCKFILES = ["package-lock.json", "yarn.lock", "pnpm-lock.yaml", "npm-shrinkwrap.json"];

/** Rewrites a `next ...` script into its Bun equivalent (other scripts are left untouched). */
export function migrateScript(script: string): string {
  // Each `next <cmd> [flags]` segment ends before `&&`, `||`, `|`, `;` or the end of the script.
  const segment = (cmd: string): RegExp => new RegExp(`\\bnext\\s+${cmd}\\b[^&|;]*?(?=\\s*(?:&&|\\|\\||\\||;|$))`, "g");
  return script
    .replace(segment("dev"), "bun --hot src/server.ts")
    .replace(segment("build"), "bun run src/build.ts")
    .replace(segment("start"), "NODE_ENV=production bun src/server.ts")
    .replace(segment("lint"), "bun lint")
    .replace(segment("export"), "bun run src/build.ts --export")
    .replace(segment("telemetry"), "true")
    .replace(/\s+$/, "");
}

function findWorkspaceRoot(root: string): boolean {
  let dir = dirname(root);
  for (let i = 0; i < 8; i += 1) {
    const pkg = join(dir, "package.json");
    if (existsSync(pkg)) {
      try {
        const parsed = JSON.parse(readFileSync(pkg, "utf8")) as { workspaces?: string[] | { packages?: string[] } };
        const globs = Array.isArray(parsed.workspaces) ? parsed.workspaces : (parsed.workspaces?.packages ?? []);
        const rel = toPosix(relative(dir, root));
        if (globs.some((glob) => new Bun.Glob(glob).match(rel))) return true;
      } catch {
        // continue upwards
      }
    }
    const parent = dirname(dir);
    if (parent === dir) break;
    dir = parent;
  }
  return false;
}

function bumpRange(current: string | undefined, wanted: string): string {
  if (!current) return wanted;
  const bare = current.replace(/^[\^~>=<\s]+/, "").split(" ")[0] ?? "";
  if (/^\d/.test(bare) && Bun.semver.order(bare, wanted.replace(/^\^/, "")) >= 0) return current;
  return wanted;
}

/** Applies the dependency/script/engine edits to a package.json object; returns the notes. */
export function migratePackageJson(pkg: Record<string, unknown>, options: { workspace: boolean }): string[] {
  const notes: string[] = [];
  const scripts = (pkg.scripts as Record<string, string> | undefined) ?? {};
  for (const [name, value] of Object.entries(scripts)) {
    const next = migrateScript(value);
    if (next !== value) {
      scripts[name] = next;
      notes.push(`script ${name}: ${value} -> ${next}`);
    }
  }
  scripts.dev ??= "bun --hot src/server.ts";
  scripts.build ??= "bun run src/build.ts";
  scripts.start ??= "NODE_ENV=production bun src/server.ts";
  pkg.scripts = scripts;
  for (const section of ["dependencies", "devDependencies"] as const) {
    const deps = pkg[section] as Record<string, string> | undefined;
    if (!deps) continue;
    for (const [name, note] of REMOVED_DEPENDENCIES) {
      if (name in deps) {
        delete deps[name];
        notes.push(`removed ${name} (${note})`);
      }
    }
  }
  const deps = ((pkg.dependencies as Record<string, string> | undefined) ??= {});
  deps["@yolo-rs/next"] = options.workspace ? "workspace:*" : "^0.1.0";
  deps.react = bumpRange(deps.react, "^19.3.1");
  deps["react-dom"] = bumpRange(deps["react-dom"], "^19.3.1");
  if (!deps["@tanstack/react-router"] && !((pkg.devDependencies as Record<string, string> | undefined)?.["@tanstack/react-router"])) deps["@tanstack/react-router"] = "^1.170.0";
  pkg.dependencies = Object.fromEntries(Object.entries(deps).sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0)));
  pkg.packageManager = "bun@1.4.4";
  const engines = ((pkg.engines as Record<string, string> | undefined) ??= {});
  engines.bun = ">=1.4.4";
  delete engines.node;
  pkg.engines = engines;
  return notes;
}

/** Renames the exported `middleware` function to `proxy` in a middleware module source. */
export function renameMiddlewareExport(source: string): string {
  return source
    .replace(/\bexport\s+(async\s+)?function\s+middleware\b/g, "export $1function proxy")
    .replace(/\bexport\s+default\s+(async\s+)?function\s+middleware\b/g, "export default $1function proxy")
    .replace(/\bexport\s+const\s+middleware\b/g, "export const proxy")
    .replace(/\bexport\s*\{\s*middleware\s*\}/g, "export { proxy }")
    .replace(/\bexport\s*\{\s*middleware\s+as\s+default\s*\}/g, "export { proxy as default }");
}

/** Rewrites `next/*`, `server-only` and `client-only` import specifiers to `@yolo-rs/next/shims/*`. */
export function rewriteNextImports(source: string): string {
  return source.replace(/((?:from|import|require\()\s*)(["'])((?:next\/[^"']+)|server-only|client-only)\2/g, (whole, prefix: string, quote: string, specifier: string) => {
    const shim = shimSpecifierFor(specifier);
    return shim ? `${prefix}${quote}${shim}${quote}` : whole;
  });
}

const SERVER_TS = `// Generated by @yolo-rs/next migrate: Bun.serve entry of the app.
import createYoloNextApp from "@yolo-rs/next/server";
import config from "../yolo.config";

const app = createYoloNextApp({ root: \`\${import.meta.dir}/..\`, config });

export default app;

if (import.meta.main) app.listen();
`;

const BUILD_TS = `// Generated by @yolo-rs/next migrate: production build (bun run src/build.ts [--export] [--standalone]).
import { buildYoloNext } from "@yolo-rs/next/build";
import config from "../yolo.config";

const result = await buildYoloNext({
  root: \`\${import.meta.dir}/..\`,
  config,
  export: Bun.argv.includes("--export"),
  standalone: Bun.argv.includes("--standalone"),
  log: (message) => console.log(message),
});

for (const warning of result.warnings) console.warn(\`[\${warning.severity}] \${warning.code}: \${warning.message}\`);
console.log(\`build \${result.success ? "ok" : "failed"} in \${result.durationMs}ms (BUILD_ID \${result.buildId}, \${result.outputs.length} outputs)\`);
if (!result.success) process.exit(1);
`;

function migrateTsconfig(text: string): string {
  const parsed = parseJsonc(text) as { compilerOptions?: Record<string, unknown>; include?: string[]; exclude?: string[] };
  const options = (parsed.compilerOptions ??= {});
  const plugins = Array.isArray(options.plugins) ? (options.plugins as Array<{ name?: string }>).filter((p) => p?.name !== "next") : undefined;
  if (plugins && plugins.length) options.plugins = plugins;
  else delete options.plugins;
  const types = new Set<string>(Array.isArray(options.types) ? (options.types as string[]) : []);
  for (const t of ["bun-types", "react", "react-dom"]) types.add(t);
  options.types = [...types];
  options.allowImportingTsExtensions = true;
  options.noEmit = true;
  if (Array.isArray(parsed.include)) parsed.include = parsed.include.filter((p) => p !== "next-env.d.ts" && !p.includes(".next/"));
  const exclude = new Set(parsed.exclude ?? ["node_modules"]);
  exclude.add(".yolo");
  parsed.exclude = [...exclude];
  return `${JSON.stringify(parsed, null, 2)}\n`;
}

function copyProject(from: string, to: string): void {
  mkdirSync(to, { recursive: true });
  for (const dirent of readdirSync(from, { withFileTypes: true })) {
    if (SKIP_DIRS.has(dirent.name)) continue;
    cpSync(join(from, dirent.name), join(to, dirent.name), { recursive: true, force: true });
  }
}

function reportMarkdown(analysis: ProjectAnalysis, config: ResolvedYoloConfig, changes: ReadonlyArray<MigrationChange>, diagnostics: ReadonlyArray<Diagnostic>): string {
  const counts = analysis.counts;
  const rows: Array<[string, number | string]> = [
    ["Routes (total)", counts.routes],
    ["Pages", counts.pages],
    ["Route handlers (app)", counts.routeHandlers - counts.legacyApiRoutes],
    ["Legacy pages/api routes", counts.legacyApiRoutes],
    ["Legacy pages", counts.legacyPages],
    ["Metadata routes", counts.metadataRoutes],
    ["Dynamic routes", counts.dynamicRoutes],
    ["Parallel slot pages", counts.parallelSlots],
    ["Intercepting routes", counts.interceptingRoutes],
    ["Server action files (`use server`)", counts.serverActionFiles],
    ["Inline server actions", counts.inlineServerActions],
    ["Client components (`use client`)", counts.useClientFiles],
    ["`use cache` files", counts.useCacheFiles],
    ["`force-dynamic` segments", counts.forceDynamic],
    ["ISR pages (`revalidate` > 0)", counts.isrPages],
    ["`generateStaticParams`", counts.generateStaticParams],
    ["`next/og` usage", counts.nextOgUsage],
    ["`@next/third-parties` usage", counts.nextThirdPartiesUsage],
    ["`next/font` usage", counts.nextFontUsage],
    ["`next/image` usage", counts.nextImageUsage],
    ["i18n message directories", counts.i18nMessageDirs],
    ["Source files / lines", `${counts.sourceFiles} / ${counts.sourceLines}`],
  ];
  const lines = [
    `# Migration report: ${analysis.name}`,
    "",
    `Generated ${new Date().toISOString()} by @yolo-rs/next (Next.js ${analysis.nextVersion ?? "?"} -> Bun 1.4.4).`,
    "",
    "## Features",
    "",
    "| Feature | Count |",
    "|---|---:|",
    ...rows.map(([label, value]) => `| ${label} | ${value} |`),
    "",
    "## Configuration",
    "",
    `- Output: \`${config.output}\`, dist \`${config.distDir}\`, app dir \`${config.appDir}\`${config.pagesDir ? `, pages dir \`${config.pagesDir}\`` : ""}`,
    `- Redirects ${config.redirects.length}, rewrites ${(config.rewrites.beforeFiles?.length ?? 0) + (config.rewrites.afterFiles?.length ?? 0) + (config.rewrites.fallback?.length ?? 0)}, headers ${config.headers.length}`,
    `- Turbopack rules -> ${config.bundler.plugins.length} Bun plugins, ${Object.keys(config.bundler.aliases).length} aliases, ${config.bundler.unsupported.length} unsupported`,
    `- Images: loader \`${config.images?.loader ?? "default"}\`, ${config.images?.remotePatterns.length ?? 0} remote patterns, qualities ${(config.images?.qualities ?? []).join("/")}`,
    config.proxyFile ? `- Proxy: \`${config.proxyFile}\`` : "- Proxy: none",
    "",
    "## Changes",
    "",
    ...changes.map((c) => `- ${c.kind} \`${c.path}\`${c.from ? ` (from \`${c.from}\`)` : ""}${c.note ? `: ${c.note}` : ""}`),
    "",
    "## Diagnostics",
    "",
    ...(diagnostics.length ? diagnostics.map((d) => `- [${d.severity}] ${d.code}: ${d.message}${d.file ? ` (${d.file})` : ""}${d.hint ? ` Hint: ${d.hint}` : ""}`) : ["- none"]),
    "",
    "## Next steps",
    "",
    "1. `bun install`",
    "2. `bun run dev` (Bun.serve with HMR) and compare pages against the Next.js build",
    "3. Replace `@next/third-parties` components with `next/script` tags; port `webpack()` customizations to `yolo.bundler.ts`",
    "4. Review every diagnostic above; delete `next.config.*` once `yolo.config.ts` is the source of truth",
    "",
  ];
  return lines.join("\n");
}

/**
 * Migrates a Next.js project to @yolo-rs/next: package.json scripts/dependencies, lockfiles,
 * `middleware.ts` -> `proxy.ts`, `yolo.config.ts`, `src/server.ts`, `src/build.ts`, `bunfig.toml`,
 * `yolo.bundler.ts`, `.yolo/route-manifest.json`, tsconfig, `.gitignore`, optional import rewrites
 * and `MIGRATION-REPORT.md`. `dryRun` plans every change without writing.
 */
export async function migrateProject(options: MigrateOptions): Promise<MigrateResult> {
  const sourceRoot = resolve(options.root);
  const mode = options.mode ?? "in-place";
  const dryRun = options.dryRun ?? false;
  const out = mode === "copy" ? resolve(options.out ?? `${sourceRoot}-yolo`) : sourceRoot;
  const log = options.log ?? (() => {});
  if (mode === "copy" && !dryRun) {
    log(`copying ${sourceRoot} -> ${out}`);
    copyProject(sourceRoot, out);
  }
  const root = mode === "copy" && !dryRun ? out : sourceRoot;
  const analysis = analyzeProject(root);
  const diagnostics: Diagnostic[] = [...analysis.diagnostics];
  const changes: MigrationChange[] = [];
  const written: string[] = [];
  const planned = new Map<string, string>();
  const write = (rel: string, contents: string, note?: string): void => {
    const path = join(out, rel);
    const exists = existsSync(path) || planned.has(rel);
    changes.push({ kind: exists ? "modify" : "create", path: rel, contents, note });
    planned.set(rel, contents);
    if (dryRun) return;
    mkdirSync(dirname(path), { recursive: true });
    writeFileSync(path, contents);
    written.push(path);
  };
  const remove = (rel: string, note?: string): void => {
    const path = join(root, rel);
    if (!existsSync(path)) return;
    changes.push({ kind: "delete", path: rel, note });
    if (!dryRun) rmSync(path, { force: true, recursive: true });
  };

  // next.config -> resolved config.
  let config: ResolvedYoloConfig;
  try {
    const loaded = await loadNextConfig(root);
    config = await resolveYoloConfig(loaded.config, { root });
    if (loaded.wrappers.length) {
      diagnostics.push({ severity: "warning", code: "next-config-wrappers", message: `next.config is wrapped by ${loaded.wrappers.join(", ")}; the wrappers' effects are not migrated automatically`, file: loaded.file });
    }
  } catch (error) {
    diagnostics.push({ severity: "error", code: "next-config-load-failed", message: `next.config could not be evaluated: ${error instanceof Error ? error.message : String(error)}`, hint: "Fix the config imports or write yolo.config.ts by hand" });
    config = await resolveYoloConfig(defineYoloConfig({}), { root });
  }
  diagnostics.push(...config.diagnostics);

  // package.json
  const pkgPath = join(root, "package.json");
  if (existsSync(pkgPath)) {
    const pkg = JSON.parse(readFileSync(pkgPath, "utf8")) as Record<string, unknown>;
    const notes = migratePackageJson(pkg, { workspace: findWorkspaceRoot(root) });
    write("package.json", `${JSON.stringify(pkg, null, 2)}\n`, notes.join("; "));
  }
  for (const lockfile of LOCKFILES) remove(lockfile, "Bun uses bun.lock");

  // middleware.ts -> proxy.ts
  const middleware = ["middleware.ts", "middleware.js", "src/middleware.ts", "src/middleware.js"].find((f) => existsSync(join(root, f)));
  const hasProxy = ["proxy.ts", "proxy.js", "src/proxy.ts", "src/proxy.js"].some((f) => existsSync(join(root, f)));
  if (middleware && !hasProxy) {
    const target = middleware.replace(/middleware(\.[jt]s)$/, "proxy$1");
    const source = renameMiddlewareExport(readFileSync(join(root, middleware), "utf8"));
    changes.push({ kind: "rename", path: target, from: middleware, note: "exported function renamed middleware -> proxy" });
    planned.set(target, source);
    if (!dryRun) {
      writeFileSync(join(out, target), source);
      rmSync(join(root, middleware), { force: true });
      written.push(join(out, target));
    }
    config = { ...config, proxyFile: target };
  }

  // yolo.config.ts, src/server.ts, src/build.ts, bunfig.toml, yolo.bundler.ts, route manifest.
  write("yolo.config.ts", serializeYoloConfig(config, { root }), "from next.config.*");
  write("src/server.ts", SERVER_TS);
  write("src/build.ts", BUILD_TS);
  write("bunfig.toml", bunfigToml(config.bundler, config));
  write("yolo.bundler.ts", bundlerPluginsTs(config.bundler));
  write(`${config.distDir}/route-manifest.json`, `${JSON.stringify(analysis.manifest, null, 2)}\n`);

  // tsconfig.json
  const tsconfigPath = join(root, "tsconfig.json");
  if (existsSync(tsconfigPath)) {
    try {
      write("tsconfig.json", migrateTsconfig(readFileSync(tsconfigPath, "utf8")), "removed the next plugin, added bun-types/react types");
    } catch (error) {
      diagnostics.push({ severity: "warning", code: "tsconfig-unparseable", message: `tsconfig.json could not be parsed: ${error instanceof Error ? error.message : String(error)}` });
    }
  }
  remove("next-env.d.ts", "generated by Next.js");

  // .gitignore
  const gitignorePath = join(root, ".gitignore");
  const gitignore = existsSync(gitignorePath) ? readFileSync(gitignorePath, "utf8") : "";
  if (!gitignore.split(/\r?\n/).some((line) => line.trim() === ".yolo/" || line.trim() === ".yolo")) {
    write(".gitignore", `${gitignore.replace(/\s*$/, "")}\n.yolo/\nout/\n`.replace(/^\n/, ""), "added .yolo/");
  }

  // Imports.
  if (options.rewriteImports) {
    let rewritten = 0;
    for (const { file, source } of sourceFiles(root)) {
      if (file.startsWith(`${config.distDir}/`) || file === "yolo.config.ts" || file === "yolo.bundler.ts" || file.startsWith("src/server.") || file.startsWith("src/build.")) continue;
      const next = rewriteNextImports(source);
      if (next !== source) {
        rewritten += 1;
        write(file, next, "next/* imports -> @yolo-rs/next/shims/*");
      }
    }
    log(`rewrote imports in ${rewritten} files`);
  }

  // Report.
  const report = reportMarkdown(analysis, config, changes, diagnostics);
  write("MIGRATION-REPORT.md", report);
  return { root: toPosix(sourceRoot), out: toPosix(out), mode, dryRun, changes, written, report, counts: analysis.counts, manifest: analysis.manifest, config, diagnostics };
}

/** Size in bytes of a file or 0. */
export function fileSize(path: string): number {
  try {
    return statSync(path).size;
  } catch {
    return 0;
  }
}
