// SPDX-License-Identifier: MIT OR Apache-2.0
// `yolo-fullstack migrate`: audit a Next.js project for the move to @yolo-rs/fullstack. The report lists
// what the framework does not implement (parallel and intercepting routes, `next/og`, `use cache`, the
// Pages Router, unsupported `next.config` keys, ...), what it maps (`next/font`, `next/dynamic`,
// `next/script`, `images.loaderFile`, `middleware` matchers), and the package.json / tsconfig / .gitignore
// rewrites. `apply: true` writes those rewrites; source files are never deleted or edited.
import { existsSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { basename, join, relative } from "node:path";
import { loadNextConfig, type ResolvedConfig } from "../config/next-config";
import pkg from "../../package.json";

export type RouterKind = "app" | "pages" | "both" | "none";
export type FindingLevel = "unsupported" | "limited";

export interface Finding {
  /** Short identifier, e.g. `parallel-route`, `next/og`, `next.config:webpack`. */
  kind: string;
  /** Project-relative file or directory (posix separators). */
  file?: string;
  detail: string;
  level?: FindingLevel;
}

export interface ScriptChange {
  name: string;
  from: string;
  to: string;
}

export interface DependencyChanges {
  remove: string[];
  add: Record<string, string>;
  warnings: string[];
}

export interface TsconfigChanges {
  file?: string;
  removeNextPlugin: boolean;
  removeInclude: string[];
}

export interface GitignoreChanges {
  file: string;
  add: string[];
}

export interface MigrationReport {
  root: string;
  router: RouterKind;
  unsupported: Finding[];
  supported: Finding[];
  scripts: ScriptChange[];
  dependencies: DependencyChanges;
  tsconfig: TsconfigChanges;
  gitignore: GitignoreChanges;
  /** Problems met while scanning (e.g. `next.config` failing to load). */
  errors: string[];
  /** Files written by `apply: true`. */
  applied: string[];
}

export interface MigrateOptions {
  apply?: boolean;
  config?: ResolvedConfig;
}

const toPosix = (path: string) => path.replaceAll("\\", "/");
const SOURCE_GLOB = new Bun.Glob("**/*.{ts,tsx,js,jsx,mjs}");
const SKIP_DIRS = new Set(["node_modules", ".next", "dist", ".git", ".yolo", "out", "build"]);
const PAGE_FILE = /^page\.(tsx|ts|jsx|js|mdx)$/;
const SOURCE_EXT = /\.(tsx|ts|jsx|js|mjs|mdx)$/;

function readJson(file: string): Record<string, unknown> | undefined {
  try {
    const value = Bun.JSONC.parse(readFileSync(file, "utf8"));
    return value && typeof value === "object" && !Array.isArray(value) ? (value as Record<string, unknown>) : undefined;
  } catch {
    return undefined;
  }
}

/** Project-relative source files, skipping build and dependency directories. */
function listSources(root: string): string[] {
  const out: string[] = [];
  for (const path of SOURCE_GLOB.scanSync({
    cwd: root,
    onlyFiles: true,
    dot: false,
  })) {
    const posix = toPosix(path);
    if (posix.split("/").some(segment => SKIP_DIRS.has(segment))) continue;
    out.push(posix);
  }
  return out.sort();
}

function dirOf(root: string, name: string): string | undefined {
  for (const dir of [name, `src/${name}`]) {
    if (statSync(join(root, dir), { throwIfNoEntry: false })?.isDirectory()) return dir;
  }
  return undefined;
}

function detectRouter(root: string, sources: string[]): RouterKind {
  const appDir = dirOf(root, "app");
  const pagesDir = dirOf(root, "pages");
  const app = !!appDir && sources.some(f => f.startsWith(`${appDir}/`) && PAGE_FILE.test(basename(f)));
  const pages =
    !!pagesDir && sources.some(f => f.startsWith(`${pagesDir}/`) && SOURCE_EXT.test(f) && !f.endsWith(".d.ts"));
  if (app && pages) return "both";
  if (app) return "app";
  if (pages) return "pages";
  return "none";
}

const importOf = (specifier: string) =>
  new RegExp(
    `(?:from\\s*|import\\s*\\(?\\s*|require\\s*\\(\\s*)["']${specifier.replaceAll("/", "\\/")}(?:\\/[^"']*)?["']`,
  );

const IMPORT_NEXT_OG = importOf("next/og");
const IMPORT_NEXT_FONT = importOf("next/font");
const IMPORT_NEXT_DYNAMIC = importOf("next/dynamic");
const IMPORT_NEXT_SCRIPT = importOf("next/script");
const IMPORT_NEXT_CONFIG = importOf("next/config");
const IMPORT_NEXT_HEADERS = importOf("next/headers");
const IMPORT_NEXT_ROUTER = importOf("next/router");
const USE_CACHE = /^\s*["']use cache["']\s*;?\s*$/m;
const UNSTABLE_CACHE = /\bunstable_cache\b/;
const EDGE_RUNTIME = /export\s+const\s+runtime\s*=\s*["']edge["']/;
const DRAFT_MODE = /\bdraftMode\b/;
const PAGES_DATA = /\b(getServerSideProps|getStaticProps|getStaticPaths|getInitialProps)\b/;
const MATCHER = /\bmatcher\s*:/;

function scanSources(root: string, sources: string[], unsupported: Finding[], supported: Finding[]): void {
  const parallel = new Set<string>();
  const intercepting = new Set<string>();
  for (const file of sources) {
    const segments = file.split("/");
    for (let i = 0; i < segments.length - 1; i++) {
      const name = segments[i]!;
      const dir = segments.slice(0, i + 1).join("/");
      if (name.startsWith("@")) parallel.add(dir);
      else if (/^\(\.{1,3}\)/.test(name)) intercepting.add(dir);
    }
  }
  for (const dir of [...parallel].sort())
    unsupported.push({
      kind: "parallel-route",
      file: dir,
      detail: "parallel routes (`@slot`) are skipped by the router",
    });
  for (const dir of [...intercepting].sort())
    unsupported.push({
      kind: "intercepting-route",
      file: dir,
      detail: "intercepting routes (`(.)`, `(..)`, `(...)`) are skipped by the router",
    });

  for (const file of sources) {
    const base = basename(file);
    const depth = file.split("/").length;
    if (/^instrumentation\.(ts|js|mjs)$/.test(base) && (depth === 1 || (file.startsWith("src/") && depth === 2)))
      unsupported.push({
        kind: "instrumentation",
        file,
        detail: "`instrumentation.ts` hooks are not called; run startup code from the server entry",
      });
    let text: string;
    try {
      text = readFileSync(join(root, file), "utf8");
    } catch {
      continue;
    }
    if (IMPORT_NEXT_OG.test(text))
      unsupported.push({
        kind: "next/og",
        file,
        detail: "`ImageResponse` is not implemented; render Open Graph images with a route handler and `sharp`",
      });
    if (IMPORT_NEXT_FONT.test(text))
      supported.push({
        kind: "next/font",
        file,
        detail: "mapped to `@yolo-rs/fullstack/font/google` and `font/local`",
      });
    if (IMPORT_NEXT_DYNAMIC.test(text))
      supported.push({
        kind: "next/dynamic",
        file,
        detail: "mapped to `@yolo-rs/fullstack/next/dynamic` (React.lazy + Suspense)",
      });
    if (IMPORT_NEXT_SCRIPT.test(text))
      supported.push({
        kind: "next/script",
        file,
        detail: "mapped to `@yolo-rs/fullstack/next/script`",
      });
    if (IMPORT_NEXT_CONFIG.test(text))
      supported.push({
        kind: "next/config",
        file,
        detail: "mapped to `@yolo-rs/fullstack/next/config` (runtime config via `define`)",
      });
    if (IMPORT_NEXT_HEADERS.test(text) && DRAFT_MODE.test(text))
      unsupported.push({
        kind: "draftMode",
        file,
        level: "limited",
        detail: "`draftMode()` reports disabled; there is no preview cookie",
      });
    if (USE_CACHE.test(text))
      unsupported.push({
        kind: "use-cache",
        file,
        detail: '`"use cache"` is not implemented; pages are prerendered or dynamic',
      });
    if (UNSTABLE_CACHE.test(text))
      unsupported.push({
        kind: "unstable_cache",
        file,
        detail: "`unstable_cache` has no data cache; memoize in the module or use `revalidatePath`",
      });
    if (EDGE_RUNTIME.test(text))
      unsupported.push({
        kind: "edge-runtime",
        file,
        detail: '`runtime = "edge"` is ignored; every route runs on Bun',
      });
    if (PAGES_DATA.test(text))
      unsupported.push({
        kind: "pages-data-fetching",
        file,
        detail:
          "`getServerSideProps` / `getStaticProps` / `getInitialProps` belong to the Pages Router; move to async server components",
      });
    if (IMPORT_NEXT_ROUTER.test(text))
      unsupported.push({
        kind: "next/router",
        file,
        detail: "`next/router` is the Pages Router API; use `next/navigation`",
      });
    if (/^(src\/)?(middleware|proxy)\.(ts|js|mjs)$/.test(file) && MATCHER.test(text))
      supported.push({
        kind: "middleware.matcher",
        file,
        detail: "`config.matcher` (strings, params and regex) is honoured",
      });
  }
}

async function scanConfig(root: string, options: MigrateOptions, report: MigrationReport): Promise<void> {
  let config = options.config;
  if (!config) {
    try {
      config = await loadNextConfig(root);
    } catch (error) {
      report.errors.push(`next.config: ${error instanceof Error ? error.message : String(error)}`);
      return;
    }
  }
  const file = config.file ? toPosix(relative(root, config.file)) : undefined;
  for (const { key, reason } of config.unsupported)
    report.unsupported.push({
      kind: `next.config:${key}`,
      file,
      detail: reason,
    });
  if (config.output === "export")
    report.supported.push({
      kind: "next.config:output",
      file,
      detail: '`output: "export"` becomes `yolo-fullstack build --prerender` (static output in dist/)',
    });
  if (config.images.loaderFile)
    report.supported.push({
      kind: "next.config:images.loaderFile",
      file,
      detail: `custom image loader ${config.images.loaderFile} is used by next/image`,
    });
}

const DROP_FLAGS = new Set(["--turbopack", "--turbo", "--webpack", "--no-turbopack"]);
const FLAG_MAP: Record<string, string> = {
  "-p": "--port",
  "--port": "--port",
  "-H": "--hostname",
  "--hostname": "--hostname",
};

function rewriteArgs(args: string[]): string[] {
  const out: string[] = [];
  for (let i = 0; i < args.length; i++) {
    const arg = args[i]!;
    if (DROP_FLAGS.has(arg)) continue;
    const eq = arg.indexOf("=");
    const name = eq > 0 ? arg.slice(0, eq) : arg;
    const mapped = FLAG_MAP[name];
    if (mapped) {
      if (eq > 0) out.push(`${mapped}=${arg.slice(eq + 1)}`);
      else {
        out.push(mapped);
        if (i + 1 < args.length) out.push(args[++i]!);
      }
      continue;
    }
    out.push(arg);
  }
  return out;
}

/** Rewrite the `next ...` commands of one package.json script (each `&&`, `||`, `;`, `|` segment separately). */
export function rewriteScript(value: string): string {
  return value
    .split(/(\s*(?:&&|\|\||;|\|)\s*)/)
    .map((segment, i) => {
      if (i % 2) return segment;
      const m = /^(\s*)next\s+(dev|build|start|lint)(?:\s+(.*))?$/.exec(segment);
      if (!m) return segment;
      const [, lead = "", command, flags = ""] = m;
      if (command === "lint") return `${lead}bun lint`;
      const args = rewriteArgs(flags.trim().split(/\s+/).filter(Boolean));
      return `${lead}yolo-fullstack ${command}${args.length ? ` ${args.join(" ")}` : ""}`;
    })
    .join("");
}

function planScripts(packageJson: Record<string, unknown> | undefined): ScriptChange[] {
  const scripts = packageJson?.scripts;
  if (!scripts || typeof scripts !== "object") return [];
  const out: ScriptChange[] = [];
  for (const [name, from] of Object.entries(scripts as Record<string, unknown>)) {
    if (typeof from !== "string") continue;
    const to = rewriteScript(from);
    if (to !== from) out.push({ name, from, to });
  }
  return out;
}

const lowestMajor = (range: string): number | undefined => {
  const m = /(\d+)\.\d+/.exec(range);
  return m ? Number(m[1]) : undefined;
};

function planDependencies(packageJson: Record<string, unknown> | undefined): DependencyChanges {
  const out: DependencyChanges = { remove: [], add: {}, warnings: [] };
  const deps = (packageJson?.dependencies ?? {}) as Record<string, string>;
  const dev = (packageJson?.devDependencies ?? {}) as Record<string, string>;
  const all = { ...dev, ...deps };
  if ("next" in all) out.remove.push("next");
  for (const name of ["eslint-config-next", "@next/eslint-plugin-next"]) if (name in all) out.remove.push(name);
  if (!("@yolo-rs/fullstack" in all)) out.add["@yolo-rs/fullstack"] = `^${pkg.version}`;
  if (!("react-server-dom-parcel" in all)) out.add["react-server-dom-parcel"] = ">=19.0.0";
  for (const name of ["react", "react-dom"]) {
    const range = all[name];
    if (!range) {
      out.add[name] = "^19.0.0";
      continue;
    }
    const major = lowestMajor(range);
    if (major !== undefined && major < 19)
      out.warnings.push(`${name}@${range}: @yolo-rs/fullstack needs React 19 (server components)`);
  }
  for (const name of Object.keys(all)) {
    if (name.startsWith("@next/") && !out.remove.includes(name))
      out.warnings.push(`${name}: Next.js package without equivalent; review or remove`);
  }
  return out;
}

const isNextPlugin = (plugin: unknown) =>
  !!plugin && typeof plugin === "object" && (plugin as { name?: unknown }).name === "next";

function planTsconfig(root: string): {
  changes: TsconfigChanges;
  json?: Record<string, unknown>;
} {
  const file = join(root, "tsconfig.json");
  if (!existsSync(file)) return { changes: { removeNextPlugin: false, removeInclude: [] } };
  const json = readJson(file);
  const changes: TsconfigChanges = {
    file: "tsconfig.json",
    removeNextPlugin: false,
    removeInclude: [],
  };
  if (!json) return { changes };
  const compiler = json.compilerOptions as Record<string, unknown> | undefined;
  const plugins = compiler?.plugins;
  if (Array.isArray(plugins) && plugins.some(isNextPlugin)) changes.removeNextPlugin = true;
  const include = json.include;
  if (Array.isArray(include))
    for (const entry of include)
      if (typeof entry === "string" && /(^|\/)next-env\.d\.ts$/.test(entry)) changes.removeInclude.push(entry);
  return { changes, json };
}

function applyTsconfig(json: Record<string, unknown>, changes: TsconfigChanges): Record<string, unknown> {
  const out = structuredClone(json);
  const compiler = out.compilerOptions as Record<string, unknown> | undefined;
  if (changes.removeNextPlugin && compiler && Array.isArray(compiler.plugins)) {
    const plugins = compiler.plugins.filter(plugin => !isNextPlugin(plugin));
    if (plugins.length) compiler.plugins = plugins;
    else delete compiler.plugins;
  }
  if (changes.removeInclude.length && Array.isArray(out.include))
    out.include = out.include.filter(entry => !changes.removeInclude.includes(entry as string));
  return out;
}

const GITIGNORE_LINES = [".yolo/", "dist/"];

function planGitignore(root: string): {
  changes: GitignoreChanges;
  text: string;
} {
  const file = join(root, ".gitignore");
  const text = existsSync(file) ? readFileSync(file, "utf8") : "";
  const present = new Set(text.split(/\r?\n/).map(line => line.trim().replace(/^\/+/, "")));
  const add = GITIGNORE_LINES.filter(line => !present.has(line) && !present.has(line.replace(/\/$/, "")));
  return { changes: { file: ".gitignore", add }, text };
}

function applyPackageJson(json: Record<string, unknown>, report: MigrationReport): Record<string, unknown> {
  const out = structuredClone(json);
  const scripts = (out.scripts ??= {}) as Record<string, string>;
  for (const change of report.scripts) scripts[change.name] = change.to;
  const deps = (out.dependencies ??= {}) as Record<string, string>;
  const dev = (out.devDependencies ?? {}) as Record<string, string>;
  for (const name of report.dependencies.remove) {
    delete deps[name];
    delete dev[name];
  }
  for (const [name, range] of Object.entries(report.dependencies.add)) deps[name] = range;
  if (Object.keys(dev).length === 0) delete out.devDependencies;
  return out;
}

const sortFindings = (a: Finding, b: Finding) =>
  (a.file ?? "").localeCompare(b.file ?? "") || a.kind.localeCompare(b.kind);

/** Audit `root` and, with `apply`, rewrite package.json, tsconfig.json and .gitignore. */
export async function migrateNextProject(root: string, options: MigrateOptions = {}): Promise<MigrationReport> {
  const sources = listSources(root);
  const packageFile = join(root, "package.json");
  const packageJson = existsSync(packageFile) ? readJson(packageFile) : undefined;
  const tsconfig = planTsconfig(root);
  const gitignore = planGitignore(root);
  const report: MigrationReport = {
    root,
    router: detectRouter(root, sources),
    unsupported: [],
    supported: [],
    scripts: planScripts(packageJson),
    dependencies: planDependencies(packageJson),
    tsconfig: tsconfig.changes,
    gitignore: gitignore.changes,
    errors: [],
    applied: [],
  };
  if (!packageJson) report.errors.push("package.json: missing or unreadable");
  if (report.router === "pages" || report.router === "both")
    report.unsupported.push({
      kind: "pages-router",
      file: dirOf(root, "pages"),
      detail: "the Pages Router is not supported; move routes to app/",
    });
  scanSources(root, sources, report.unsupported, report.supported);
  await scanConfig(root, options, report);
  report.unsupported.sort(sortFindings);
  report.supported.sort(sortFindings);

  if (options.apply) {
    if (
      packageJson &&
      (report.scripts.length || report.dependencies.remove.length || Object.keys(report.dependencies.add).length)
    ) {
      writeFileSync(packageFile, `${JSON.stringify(applyPackageJson(packageJson, report), null, 2)}\n`);
      report.applied.push("package.json");
    }
    if (tsconfig.json && (tsconfig.changes.removeNextPlugin || tsconfig.changes.removeInclude.length)) {
      writeFileSync(
        join(root, "tsconfig.json"),
        `${JSON.stringify(applyTsconfig(tsconfig.json, tsconfig.changes), null, 2)}\n`,
      );
      report.applied.push("tsconfig.json");
    }
    if (gitignore.changes.add.length) {
      const base = gitignore.text.length && !gitignore.text.endsWith("\n") ? `${gitignore.text}\n` : gitignore.text;
      writeFileSync(join(root, ".gitignore"), `${base}${gitignore.changes.add.join("\n")}\n`);
      report.applied.push(".gitignore");
    }
    writeFileSync(join(root, "yolo-migration-report.json"), `${JSON.stringify(report, null, 2)}\n`);
    report.applied.push("yolo-migration-report.json");
  }
  return report;
}

function table(rows: string[][]): string[] {
  const widths: number[] = [];
  for (const row of rows) row.forEach((cell, i) => (widths[i] = Math.max(widths[i] ?? 0, cell.length)));
  return rows.map(row =>
    row
      .map((cell, i) => (i === row.length - 1 ? cell : cell.padEnd(widths[i]!)))
      .join("  ")
      .trimEnd(),
  );
}

/** Plain-text rendering of a report for the CLI. */
export function formatMigrationReport(report: MigrationReport): string {
  const lines: string[] = [`Next.js migration report: ${report.root}`, `router: ${report.router}`, ""];
  const section = (title: string, body: string[]) => {
    lines.push(title);
    lines.push(...(body.length ? body.map(line => `  ${line}`) : ["  (none)"]));
    lines.push("");
  };
  section(
    `unsupported (${report.unsupported.length})`,
    table(
      report.unsupported.map(f => [f.level === "limited" ? "limited" : "blocked", f.kind, f.file ?? "-", f.detail]),
    ),
  );
  section(
    `supported (${report.supported.length})`,
    table(report.supported.map(f => [f.kind, f.file ?? "-", f.detail])),
  );
  section("scripts", table(report.scripts.map(s => [s.name, s.from, "->", s.to])));
  section("dependencies", [
    ...report.dependencies.remove.map(name => `remove ${name}`),
    ...Object.entries(report.dependencies.add).map(([name, range]) => `add    ${name}@${range}`),
    ...report.dependencies.warnings.map(warning => `warn   ${warning}`),
  ]);
  const ts: string[] = [];
  if (report.tsconfig.removeNextPlugin) ts.push('remove compilerOptions.plugins { name: "next" }');
  for (const entry of report.tsconfig.removeInclude) ts.push(`remove include ${entry}`);
  section("tsconfig.json", ts);
  section(
    ".gitignore",
    report.gitignore.add.map(line => `add ${line}`),
  );
  if (report.errors.length) section("errors", report.errors);
  if (report.applied.length) section("written", report.applied);
  return lines.join("\n").trimEnd() + "\n";
}
