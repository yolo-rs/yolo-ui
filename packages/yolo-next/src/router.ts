// SPDX-License-Identifier: Apache-2.0
// @yolo-rs/next/router: App Router (`app/`) and Pages Router (`pages/`) file-system scanner, pattern
// conversions (Bun / TanStack / Dioxus / Leptos), matcher, Next.js-shaped manifests and the TanStack
// code-based route tree generator. TypeScript twin of crates/yolo-next/src/routes.rs.

import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { join, relative, resolve } from "node:path";

import { analyzeSegmentModule, legacyDataFunctions, routeHandlerMethods } from "./segments.ts";
import type {
  Diagnostic,
  HeaderRule,
  InterceptSpec,
  MetadataFile,
  MetadataFileKind,
  RouteEntry,
  RouteKind,
  RouteManifest,
  RouteMatch,
  RouteParam,
  RouteSegment,
  Rewrites,
  RouteRule,
  SegmentConfig,
  SegmentFiles,
  SegmentKind,
} from "./types.ts";

/** Options of `scanAppDir`. */
export interface ScanOptions {
  readonly appDir?: string;
  readonly pagesDir?: string;
  /** Page extensions (default `tsx ts jsx js`; `mdx`/`md` are accepted for pages and layouts when listed). */
  readonly pageExtensions?: ReadonlyArray<string>;
}

const DEFAULT_PAGE_EXTENSIONS = ["tsx", "ts", "jsx", "js"];
const ROUTE_EXTENSIONS = ["ts", "js", "tsx", "jsx"];
const SPECIAL_FILES: ReadonlyArray<readonly [string, keyof Omit<SegmentFiles, "metadata">]> = [
  ["page", "page"],
  ["layout", "layout"],
  ["template", "template"],
  ["loading", "loading"],
  ["error", "error"],
  ["global-error", "globalError"],
  ["not-found", "notFound"],
  ["forbidden", "forbidden"],
  ["unauthorized", "unauthorized"],
  ["default", "default"],
  ["route", "route"],
];
const METADATA_GENERATED = new Set(["tsx", "ts", "js", "jsx"]);
const IMAGE_EXTENSIONS = new Set(["ico", "jpg", "jpeg", "png", "svg", "gif", "webp", "avif"]);
const PAGES_METHODS = ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"];

const toPosix = (path: string): string => path.replace(/\\/g, "/");

function isDir(path: string): boolean {
  try {
    return statSync(path).isDirectory();
  } catch {
    return false;
  }
}

function splitName(name: string): { base: string; ext: string } {
  const dot = name.lastIndexOf(".");
  if (dot <= 0) return { base: name, ext: "" };
  return { base: name.slice(0, dot), ext: name.slice(dot + 1).toLowerCase() };
}

function readSource(path: string): string {
  try {
    return readFileSync(path, "utf8");
  } catch {
    return "";
  }
}

// ---------------------------------------------------------------------------
// Segment classification
// ---------------------------------------------------------------------------

/** Classification of one `app/` directory name. */
export interface SegmentClass {
  readonly kind: SegmentKind;
  readonly param?: string;
  readonly interceptLevel?: number;
  /** Intercepted segment name (`photos` for `(.)photos`). */
  readonly interceptTarget?: string;
  /** Name of the segment once the intercept marker is removed. */
  readonly inner?: string;
}

/** Classifies a directory name: dynamic, catch-all, optional catch-all, group, parallel slot, intercept, private, static. */
export function classifySegment(name: string): SegmentClass {
  if (name.startsWith("_")) return { kind: "private" };
  const intercept = /^(\(\.\.\.\)|(?:\(\.\.\))+|\(\.\))(.+)$/.exec(name);
  if (intercept) {
    const marker = intercept[1] ?? "";
    const inner = intercept[2] ?? "";
    const level = marker === "(.)" ? 0 : marker === "(...)" ? 3 : marker.length / 4;
    return { kind: "intercept", interceptLevel: level, interceptTarget: inner, inner };
  }
  if (/^\(.+\)$/.test(name)) return { kind: "group" };
  if (name.startsWith("@")) return { kind: "parallel-slot" };
  const optional = /^\[\[\.\.\.(.+)\]\]$/.exec(name);
  if (optional) return { kind: "optional-catch-all", param: optional[1] };
  const catchAll = /^\[\.\.\.(.+)\]$/.exec(name);
  if (catchAll) return { kind: "catch-all", param: catchAll[1] };
  const dynamic = /^\[(.+)\]$/.exec(name);
  if (dynamic) return { kind: "dynamic", param: dynamic[1] };
  return { kind: "static" };
}

/** Parses the params of a URL pattern (`/blog/[slug]/[...rest]`). */
export function paramsOf(urlPattern: string): RouteParam[] {
  const out: RouteParam[] = [];
  for (const segment of urlPattern.split("/")) {
    const cls = classifySegment(segment);
    if (cls.kind === "dynamic" || cls.kind === "catch-all" || cls.kind === "optional-catch-all") {
      out.push({ name: cls.param ?? "", catchAll: cls.kind !== "dynamic", optional: cls.kind === "optional-catch-all" });
    }
  }
  return out;
}

// ---------------------------------------------------------------------------
// Pattern conversions
// ---------------------------------------------------------------------------

type SegmentShape = { kind: "static"; text: string } | { kind: "dynamic" | "catch-all" | "optional-catch-all"; name: string };

function shapes(urlPattern: string): SegmentShape[] {
  if (urlPattern === "/" || urlPattern === "") return [];
  return urlPattern
    .replace(/^\//, "")
    .split("/")
    .map((seg) => {
      const cls = classifySegment(seg);
      if (cls.kind === "dynamic" || cls.kind === "catch-all" || cls.kind === "optional-catch-all") return { kind: cls.kind, name: cls.param ?? "" };
      return { kind: "static", text: seg };
    });
}

function convert(urlPattern: string, dynamic: (name: string) => string, catchAll: (name: string, optional: boolean) => string): string {
  const parts = shapes(urlPattern).map((s) => (s.kind === "static" ? s.text : s.kind === "dynamic" ? dynamic(s.name) : catchAll(s.name, s.kind === "optional-catch-all")));
  return parts.length ? `/${parts.join("/")}` : "/";
}

/** `/blog/[slug]` -> `/blog/:slug`; `[...slug]` and `[[...slug]]` -> `*`. */
export function toBunPattern(urlPattern: string): string {
  return convert(urlPattern, (name) => `:${name}`, () => "*");
}

/** `/blog/[slug]` -> `/blog/$slug`; catch-all -> `/docs/$`. */
export function toTanStackPath(urlPattern: string): string {
  return convert(urlPattern, (name) => `$${name}`, () => "$");
}

/** `/blog/[slug]` -> `/blog/:slug`; catch-all -> `/docs/:..slug`. */
export function toDioxusRoute(urlPattern: string): string {
  return convert(urlPattern, (name) => `:${name}`, (name) => `:..${name}`);
}

/** `/blog/[slug]` -> `/blog/:slug`; catch-all -> `/docs/*slug`. */
export function toLeptosPath(urlPattern: string): string {
  return convert(urlPattern, (name) => `:${name}`, (name) => `*${name}`);
}

function pascal(text: string): string {
  return text
    .split(/[^A-Za-z0-9]+/)
    .filter(Boolean)
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join("");
}

/** `/` -> `Home`; `/blog/[slug]` -> `BlogSlug`; `/docs/[...slug]` -> `DocsSlugCatchAll`; `/shop/[[...f]]` -> `ShopFOptional`. */
export function toDioxusVariant(urlPattern: string): string {
  const parts = shapes(urlPattern);
  if (!parts.length) return "Home";
  const name = parts
    .map((s) => (s.kind === "static" ? pascal(s.text) : s.kind === "dynamic" ? pascal(s.name) : s.kind === "catch-all" ? `${pascal(s.name)}CatchAll` : `${pascal(s.name)}Optional`))
    .join("");
  return /^\d/.test(name) ? `Route${name}` : name || "Home";
}

// ---------------------------------------------------------------------------
// Sorting and matching
// ---------------------------------------------------------------------------

function segmentScore(segment: string): number {
  const cls = classifySegment(segment);
  switch (cls.kind) {
    case "dynamic":
      return 1;
    case "catch-all":
      return 2;
    case "optional-catch-all":
      return 3;
    default:
      return 0;
  }
}

function scores(urlPattern: string): number[] {
  if (urlPattern === "/") return [];
  return urlPattern.replace(/^\//, "").split("/").map(segmentScore);
}

/** Compares two URL patterns by matching precedence (static < dynamic < catch-all < optional, shorter first, then alphabetical). */
export function compareUrlPatterns(a: string, b: string): number {
  const sa = scores(a);
  const sb = scores(b);
  const n = Math.min(sa.length, sb.length);
  for (let i = 0; i < n; i += 1) {
    const diff = (sa[i] ?? 0) - (sb[i] ?? 0);
    if (diff !== 0) return diff;
  }
  if (sa.length !== sb.length) return sa.length - sb.length;
  return a < b ? -1 : a > b ? 1 : 0;
}

/** Sorts entries by matching precedence; page/handler/metadata kinds tie on pattern, slots after their page. */
export function sortEntries(entries: ReadonlyArray<RouteEntry>): RouteEntry[] {
  return [...entries].sort((a, b) => {
    const byPattern = compareUrlPatterns(a.urlPattern, b.urlPattern);
    if (byPattern !== 0) return byPattern;
    const slotA = a.slot ? 1 : 0;
    const slotB = b.slot ? 1 : 0;
    if (slotA !== slotB) return slotA - slotB;
    return a.appPath < b.appPath ? -1 : a.appPath > b.appPath ? 1 : 0;
  });
}

function safeDecode(value: string): string {
  try {
    return decodeURIComponent(value);
  } catch {
    return value;
  }
}

/** Matches URL segments against a bracket pattern; returns the params or undefined. */
export function matchPattern(urlPattern: string, segments: ReadonlyArray<string>): Record<string, string> | undefined {
  const parts = shapes(urlPattern);
  const params: Record<string, string> = {};
  let i = 0;
  for (let p = 0; p < parts.length; p += 1) {
    const part = parts[p];
    if (!part) continue;
    if (part.kind === "static") {
      if (segments[i] !== part.text) return undefined;
      i += 1;
      continue;
    }
    if (part.kind === "dynamic") {
      const value = segments[i];
      if (value === undefined || value === "") return undefined;
      params[part.name] = safeDecode(value);
      i += 1;
      continue;
    }
    const rest = segments.slice(i);
    if (part.kind === "catch-all" && rest.length === 0) return undefined;
    if (rest.length) params[part.name] = rest.map(safeDecode).join("/");
    i = segments.length;
  }
  return i === segments.length ? params : undefined;
}

const sortedCache = new WeakMap<RouteManifest, RouteEntry[]>();

function sortedEntries(manifest: RouteManifest): RouteEntry[] {
  let sorted = sortedCache.get(manifest);
  if (!sorted) {
    sorted = sortEntries(manifest.entries);
    sortedCache.set(manifest, sorted);
  }
  return sorted;
}

/** Splits a pathname into decoded-later segments, tolerating a trailing slash and ignoring query/hash. */
export function pathSegments(pathname: string): string[] {
  let path = pathname.split(/[?#]/)[0] ?? "";
  if (!path.startsWith("/")) path = `/${path}`;
  if (path.length > 1 && path.endsWith("/")) path = path.slice(0, -1);
  return path === "/" ? [] : path.slice(1).split("/");
}

/** Matches a pathname against the manifest (slot entries are skipped; catch-all params are joined with `/`). */
export function matchRoute(manifest: RouteManifest, pathname: string): RouteMatch | undefined {
  const segments = pathSegments(pathname);
  for (const entry of sortedEntries(manifest)) {
    if (entry.slot) continue;
    const params = matchPattern(entry.urlPattern, segments);
    if (params) return { entry, params };
  }
  return undefined;
}

// ---------------------------------------------------------------------------
// Scanner
// ---------------------------------------------------------------------------

interface Frame {
  readonly dirAbs: string;
  readonly appPath: ReadonlyArray<string>;
  readonly urlSegments: ReadonlyArray<string>;
  readonly layouts: ReadonlyArray<string>;
  readonly templates: ReadonlyArray<string>;
  readonly loading?: string;
  readonly error?: string;
  readonly notFound?: string;
  readonly parallelSlots: ReadonlyArray<string>;
  readonly slot?: string;
  readonly intercept?: { level: number };
}

interface ScanState {
  readonly root: string;
  readonly appDir: string;
  readonly exts: ReadonlyArray<string>;
  readonly entries: RouteEntry[];
  readonly diagnostics: Diagnostic[];
  readonly staticMetadata: MetadataFile[];
  rootLayout?: string;
  rootNotFound?: string;
  globalError?: string;
}

function metadataKindOf(base: string, ext: string): { kind: MetadataFileKind; generated: boolean } | undefined {
  const image = /^(icon|apple-icon|opengraph-image|twitter-image)\d*$/.exec(base);
  if (image) {
    const kind = image[1] as MetadataFileKind;
    if (METADATA_GENERATED.has(ext)) return { kind, generated: true };
    if (IMAGE_EXTENSIONS.has(ext)) return { kind, generated: false };
    return undefined;
  }
  if (base === "favicon" && ext === "ico") return { kind: "favicon", generated: false };
  if (base === "sitemap" && (ext === "ts" || ext === "js")) return { kind: "sitemap", generated: true };
  if (base === "sitemap" && ext === "xml") return { kind: "sitemap", generated: false };
  if (base === "robots" && (ext === "ts" || ext === "js")) return { kind: "robots", generated: true };
  if (base === "robots" && ext === "txt") return { kind: "robots", generated: false };
  if (base === "manifest" && (ext === "ts" || ext === "js")) return { kind: "manifest", generated: true };
  if (base === "manifest" && (ext === "json" || ext === "webmanifest")) return { kind: "manifest", generated: false };
  return undefined;
}

function metadataUrl(kind: MetadataFileKind, prefix: string, fileName: string, generated: boolean): string {
  const base = prefix === "/" ? "" : prefix;
  if (!generated) return `${base}/${fileName}`;
  switch (kind) {
    case "sitemap":
      return `${base}/sitemap.xml`;
    case "robots":
      return `${base}/robots.txt`;
    case "manifest":
      return `${base}/manifest.webmanifest`;
    default:
      return `${base}/${kind}`;
  }
}

function makeEntry(state: ScanState, frame: Frame, kind: RouteKind, file: string, urlPattern: string, extra: Partial<RouteEntry> = {}): RouteEntry {
  const relFile = toPosix(relative(state.root, file));
  const source = readSource(file);
  const segmentConfig: SegmentConfig = kind === "metadata" && !extra.segmentConfig ? analyzeSegmentModule(source, relFile) : (extra.segmentConfig ?? analyzeSegmentModule(source, relFile));
  const entry: RouteEntry = {
    appPath: frame.appPath.length ? `/${frame.appPath.join("/")}` : "/",
    urlPattern,
    bunPattern: toBunPattern(urlPattern),
    tanstackPath: toTanStackPath(urlPattern),
    dioxusRoute: toDioxusRoute(urlPattern),
    dioxusVariant: toDioxusVariant(urlPattern),
    leptosPath: toLeptosPath(urlPattern),
    params: paramsOf(urlPattern),
    kind,
    file: relFile,
    layouts: [...frame.layouts],
    templates: [...frame.templates],
    loading: frame.loading,
    error: frame.error,
    notFound: frame.notFound,
    parallelSlots: [...frame.parallelSlots],
    slot: frame.slot,
    intercepting: frame.intercept ? { level: frame.intercept.level, target: urlPattern } : undefined,
    segmentConfig,
    methods: kind === "route-handler" ? routeHandlerMethods(source, relFile) : [],
    metadataKind: undefined,
    legacyPages: false,
    legacyDataFunctions: [],
    ...extra,
  };
  return stripUndefinedEntry(entry);
}

function stripUndefinedEntry(entry: RouteEntry): RouteEntry {
  const out: Record<string, unknown> = {};
  for (const [key, value] of Object.entries(entry)) if (value !== undefined) out[key] = value;
  return out as unknown as RouteEntry;
}

function walkSegment(state: ScanState, frame: Frame, name: string, cls: SegmentClass): RouteSegment {
  const dirents = readdirSync(frame.dirAbs, { withFileTypes: true }).sort((a, b) => (a.name < b.name ? -1 : a.name > b.name ? 1 : 0));
  const files: Record<string, string> = {};
  const metadata: MetadataFile[] = [];
  const generatedMetadata: Array<{ kind: MetadataFileKind; file: string; fileName: string }> = [];
  const dirs: string[] = [];
  for (const dirent of dirents) {
    if (dirent.isDirectory()) {
      dirs.push(dirent.name);
      continue;
    }
    if (!dirent.isFile()) continue;
    const { base, ext } = splitName(dirent.name);
    const abs = join(frame.dirAbs, dirent.name);
    const special = SPECIAL_FILES.find(([special]) => special === base);
    if (special) {
      const [, key] = special;
      const allowed = key === "route" ? ROUTE_EXTENSIONS : state.exts;
      if (allowed.includes(ext) && files[key] === undefined) files[key] = abs;
      continue;
    }
    const meta = metadataKindOf(base, ext);
    if (meta) {
      const rel = toPosix(relative(state.root, abs));
      metadata.push({ kind: meta.kind, path: rel, generated: meta.generated });
      if (meta.generated) generatedMetadata.push({ kind: meta.kind, file: abs, fileName: dirent.name });
      else if (frame.appPath.length === 0) state.staticMetadata.push({ kind: meta.kind, path: rel, generated: false });
    }
  }
  const rel = (abs: string | undefined): string | undefined => (abs ? toPosix(relative(state.root, abs)) : undefined);
  const segmentFiles: SegmentFiles = stripUndefinedFiles({
    page: rel(files.page),
    layout: rel(files.layout),
    template: rel(files.template),
    loading: rel(files.loading),
    error: rel(files.error),
    globalError: rel(files.globalError),
    notFound: rel(files.notFound),
    forbidden: rel(files.forbidden),
    unauthorized: rel(files.unauthorized),
    default: rel(files.default),
    route: rel(files.route),
    metadata,
  });
  const isRoot = frame.appPath.length === 0;
  if (isRoot) {
    state.rootLayout = segmentFiles.layout;
    state.rootNotFound = segmentFiles.notFound;
    state.globalError = segmentFiles.globalError;
  }
  const slotChildren = dirs.filter((d) => d.startsWith("@")).map((d) => d.slice(1));
  const layouts = segmentFiles.layout ? [...frame.layouts, segmentFiles.layout] : frame.layouts;
  const templates = segmentFiles.template ? [...frame.templates, segmentFiles.template] : frame.templates;
  const current: Frame = {
    ...frame,
    layouts,
    templates,
    loading: segmentFiles.loading ?? frame.loading,
    error: segmentFiles.error ?? frame.error,
    notFound: segmentFiles.notFound ?? frame.notFound,
    parallelSlots: segmentFiles.layout ? slotChildren : frame.parallelSlots,
  };
  const urlPattern = frame.urlSegments.length ? `/${frame.urlSegments.join("/")}` : "/";
  if (segmentFiles.page && segmentFiles.route) {
    state.diagnostics.push({
      severity: "error",
      code: "route-page-conflict",
      message: `${urlPattern} has both page and route files; a segment resolves to one or the other`,
      file: segmentFiles.route,
      hint: "Move the route handler to a sibling segment (for example `api/`)",
    });
  }
  if (files.page) state.entries.push(makeEntry(state, current, "page", files.page, urlPattern));
  else if (files.route) state.entries.push(makeEntry(state, current, "route-handler", files.route, urlPattern));
  for (const generated of generatedMetadata) {
    const url = metadataUrl(generated.kind, urlPattern, generated.fileName, true);
    state.entries.push(makeEntry(state, { ...current, slot: undefined, intercept: undefined }, "metadata", generated.file, url, { metadataKind: generated.kind, params: paramsOf(url) }));
  }
  const children: RouteSegment[] = [];
  for (const dir of dirs) {
    if (dir === "node_modules") continue;
    const childClass = classifySegment(dir);
    if (childClass.kind === "private") continue;
    const child = childFrame(current, dir, childClass);
    children.push(walkSegment(state, child, dir, childClass));
  }
  return stripUndefinedSegment({
    name,
    kind: cls.kind,
    param: cls.param,
    interceptLevel: cls.interceptLevel,
    interceptTarget: cls.interceptTarget,
    files: segmentFiles,
    children,
  });
}

function childFrame(parent: Frame, dir: string, cls: SegmentClass): Frame {
  const dirAbs = join(parent.dirAbs, dir);
  const appPath = [...parent.appPath, dir];
  switch (cls.kind) {
    case "group":
      return { ...parent, dirAbs, appPath };
    case "parallel-slot":
      return { ...parent, dirAbs, appPath, slot: dir.slice(1) };
    case "intercept": {
      const level = cls.interceptLevel ?? 0;
      const base = level === 3 ? [] : level === 0 ? [...parent.urlSegments] : parent.urlSegments.slice(0, Math.max(0, parent.urlSegments.length - level));
      const inner = cls.inner ?? dir;
      return { ...parent, dirAbs, appPath, urlSegments: [...base, inner], intercept: { level } };
    }
    default:
      return { ...parent, dirAbs, appPath, urlSegments: [...parent.urlSegments, dir] };
  }
}

function stripUndefinedFiles(files: SegmentFiles): SegmentFiles {
  const out: Record<string, unknown> = {};
  for (const [key, value] of Object.entries(files)) if (value !== undefined) out[key] = value;
  return out as unknown as SegmentFiles;
}

function stripUndefinedSegment(segment: RouteSegment): RouteSegment {
  const out: Record<string, unknown> = {};
  for (const [key, value] of Object.entries(segment)) if (value !== undefined) out[key] = value;
  return out as unknown as RouteSegment;
}

function scanPages(state: ScanState, pagesDir: string): void {
  const base = join(state.root, pagesDir);
  const walk = (dir: string, urlParts: string[]): void => {
    const dirents = readdirSync(dir, { withFileTypes: true }).sort((a, b) => (a.name < b.name ? -1 : a.name > b.name ? 1 : 0));
    for (const dirent of dirents) {
      if (dirent.isDirectory()) {
        if (dirent.name === "node_modules" || dirent.name.startsWith("_")) continue;
        walk(join(dir, dirent.name), [...urlParts, dirent.name]);
        continue;
      }
      if (!dirent.isFile()) continue;
      const { base: name, ext } = splitName(dirent.name);
      if (!state.exts.includes(ext) || ext === "css") continue;
      if (name.startsWith("_")) continue;
      if (name.endsWith(".test") || name.endsWith(".spec") || name.endsWith(".d")) continue;
      const parts = name === "index" ? urlParts : [...urlParts, name];
      const urlPattern = parts.length ? `/${parts.join("/")}` : "/";
      const abs = join(dir, dirent.name);
      const relFile = toPosix(relative(state.root, abs));
      const source = readSource(abs);
      const isApi = urlParts[0] === "api";
      const frame: Frame = { dirAbs: dir, appPath: parts, urlSegments: parts, layouts: [], templates: [], parallelSlots: [] };
      const entry = makeEntry(state, frame, isApi ? "route-handler" : "page", abs, urlPattern, {
        appPath: urlPattern,
        legacyPages: true,
        legacyDataFunctions: isApi ? [] : legacyDataFunctions(source, relFile),
        methods: isApi ? [...PAGES_METHODS] : [],
      });
      state.entries.push(entry);
    }
  };
  walk(base, []);
}

const INSTRUMENTATION = ["instrumentation.ts", "instrumentation.js", "src/instrumentation.ts", "src/instrumentation.js"];
const INSTRUMENTATION_CLIENT = ["instrumentation-client.ts", "instrumentation-client.js", "src/instrumentation-client.ts", "src/instrumentation-client.js"];
const MDX_COMPONENTS = ["mdx-components.tsx", "mdx-components.ts", "mdx-components.js", "src/mdx-components.tsx", "src/mdx-components.ts"];
const PROXY_FILES = ["proxy.ts", "proxy.js", "proxy.mts", "proxy.mjs", "src/proxy.ts", "src/proxy.js", "middleware.ts", "middleware.js", "src/middleware.ts", "src/middleware.js"];

function firstExisting(root: string, names: ReadonlyArray<string>): string | undefined {
  return names.find((name) => existsSync(join(root, name)));
}

/** Scans `app/` (and `pages/` when present) under `root` into a RouteManifest with sorted entries. */
export function scanAppDir(root: string, opts: ScanOptions = {}): RouteManifest {
  const absRoot = resolve(root);
  const appDir = opts.appDir ?? ["app", "src/app"].find((dir) => isDir(join(absRoot, dir)));
  const pagesDir = opts.pagesDir ?? ["pages", "src/pages"].find((dir) => isDir(join(absRoot, dir)));
  const exts = opts.pageExtensions ? [...opts.pageExtensions] : [...DEFAULT_PAGE_EXTENSIONS];
  const state: ScanState = { root: absRoot, appDir: appDir ?? "app", exts, entries: [], diagnostics: [], staticMetadata: [] };
  let tree: RouteSegment | undefined;
  if (appDir && isDir(join(absRoot, appDir))) {
    const frame: Frame = { dirAbs: join(absRoot, appDir), appPath: [], urlSegments: [], layouts: [], templates: [], parallelSlots: [] };
    tree = walkSegment(state, frame, "app", { kind: "root" });
    if (!state.rootLayout) {
      state.diagnostics.push({ severity: "error", code: "root-layout-missing", message: `${appDir}/layout.tsx is required by the App Router`, hint: "Add a root layout rendering <html> and <body>" });
    }
  }
  if (pagesDir && isDir(join(absRoot, pagesDir))) scanPages(state, pagesDir);
  const manifest: RouteManifest = {
    root: toPosix(absRoot),
    appDir: appDir && isDir(join(absRoot, appDir)) ? appDir : undefined,
    pagesDir: pagesDir && isDir(join(absRoot, pagesDir)) ? pagesDir : undefined,
    tree,
    entries: sortEntries(state.entries),
    rootLayout: state.rootLayout,
    rootNotFound: state.rootNotFound,
    globalError: state.globalError,
    proxyFile: firstExisting(absRoot, PROXY_FILES),
    instrumentation: firstExisting(absRoot, INSTRUMENTATION),
    instrumentationClient: firstExisting(absRoot, INSTRUMENTATION_CLIENT),
    mdxComponents: firstExisting(absRoot, MDX_COMPONENTS),
    staticMetadata: state.staticMetadata,
    diagnostics: state.diagnostics,
  };
  const out: Record<string, unknown> = {};
  for (const [key, value] of Object.entries(manifest)) if (value !== undefined) out[key] = value;
  return out as unknown as RouteManifest;
}

/** Reads `<root>/.yolo/route-manifest.json` when present, else scans the project. */
export function loadRouteManifest(root: string, opts: ScanOptions & { distDir?: string } = {}): RouteManifest {
  const file = join(resolve(root), opts.distDir ?? ".yolo", "route-manifest.json");
  if (existsSync(file)) {
    try {
      return JSON.parse(readFileSync(file, "utf8")) as RouteManifest;
    } catch {
      // fall through to a fresh scan
    }
  }
  return scanAppDir(root, opts);
}

// ---------------------------------------------------------------------------
// Next.js-shaped manifests
// ---------------------------------------------------------------------------

const escapeRegex = (text: string): string => text.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");

/** `routes-manifest.json` regexes of a URL pattern (`regex`, `namedRegex`, `routeKeys`). */
export function routeRegexes(urlPattern: string): { regex: string; namedRegex: string; routeKeys: Record<string, string> } {
  const routeKeys: Record<string, string> = {};
  let regex = "^";
  let named = "^";
  for (const part of shapes(urlPattern)) {
    if (part.kind === "static") {
      regex += `/${escapeRegex(part.text)}`;
      named += `/${escapeRegex(part.text)}`;
      continue;
    }
    const key = `nxtP${part.name.replace(/[^\w]/g, "")}`;
    routeKeys[key] = key;
    if (part.kind === "dynamic") {
      regex += "/([^/]+?)";
      named += `/(?<${key}>[^/]+?)`;
    } else if (part.kind === "catch-all") {
      regex += "/(.+?)";
      named += `/(?<${key}>.+?)`;
    } else {
      regex += "(?:/(.+?))?";
      named += `(?:/(?<${key}>.+?))?`;
    }
  }
  if (regex === "^") {
    regex += "/";
    named += "/";
  }
  return { regex: `${regex}(?:/)?$`, namedRegex: `${named}(?:/)?$`, routeKeys };
}

/** Optional rules embedded in `routes-manifest.json`. */
export interface RoutesManifestRules {
  readonly redirects?: ReadonlyArray<RouteRule>;
  readonly rewrites?: Rewrites;
  readonly headers?: ReadonlyArray<HeaderRule>;
}

/** `routes-manifest.json` (version 3) with staticRoutes/dynamicRoutes regexes and the RSC header block. */
export function routesManifestJson(manifest: RouteManifest, basePath = "", rules: RoutesManifestRules = {}): Record<string, unknown> {
  const staticRoutes: Record<string, unknown>[] = [];
  const dynamicRoutes: Record<string, unknown>[] = [];
  for (const entry of sortEntries(manifest.entries)) {
    if (entry.slot) continue;
    const { regex, namedRegex, routeKeys } = routeRegexes(entry.urlPattern);
    const item = { page: entry.urlPattern, regex, routeKeys, namedRegex };
    (entry.params.length ? dynamicRoutes : staticRoutes).push(item);
  }
  return {
    version: 3,
    pages404: true,
    caseSensitive: false,
    basePath,
    redirects: [...(rules.redirects ?? [])],
    rewrites: {
      beforeFiles: [...(rules.rewrites?.beforeFiles ?? [])],
      afterFiles: [...(rules.rewrites?.afterFiles ?? [])],
      fallback: [...(rules.rewrites?.fallback ?? [])],
    },
    headers: [...(rules.headers ?? [])],
    staticRoutes,
    dynamicRoutes,
    dataRoutes: [],
    rsc: {
      header: "RSC",
      varyHeader: "RSC, Next-Router-State-Tree, Next-Router-Prefetch, Next-Router-Segment-Prefetch",
      prefetchHeader: "Next-Router-Prefetch",
      didPostponeHeader: "x-nextjs-postponed",
      contentTypeHeader: "text/x-component",
      suffix: ".rsc",
      prefetchSuffix: ".prefetch.rsc",
      prefetchSegmentHeader: "Next-Router-Segment-Prefetch",
      prefetchSegmentSuffix: ".segment.rsc",
      prefetchSegmentDirSuffix: ".segments",
    },
    rewriteHeaders: { pathHeader: "x-nextjs-rewritten-path", queryHeader: "x-nextjs-rewritten-query" },
  };
}

/** `app-paths-manifest.json`: `/(group)/x/page` -> module path (App Router entries only). */
export function appPathsManifestJson(manifest: RouteManifest): Record<string, string> {
  const out: Record<string, string> = {};
  for (const entry of sortEntries(manifest.entries)) {
    if (entry.legacyPages) continue;
    const suffix = entry.kind === "page" ? "page" : "route";
    const key = entry.kind === "metadata" ? `${entry.urlPattern}/route` : `${entry.appPath === "/" ? "" : entry.appPath}/${suffix}`;
    out[key] = entry.file;
  }
  return out;
}

/** `pages-manifest.json`: legacy Pages Router entries. */
export function pagesManifestJson(manifest: RouteManifest): Record<string, string> {
  const out: Record<string, string> = {};
  for (const entry of manifest.entries) if (entry.legacyPages) out[entry.urlPattern] = entry.file;
  return out;
}

// ---------------------------------------------------------------------------
// TanStack Router code-based tree
// ---------------------------------------------------------------------------

/** Options of `generateRouteTree`. */
export interface RouteTreeOptions {
  /** Import prefix of the app directory as seen from the generated file (default `../app`). */
  readonly appDirImportPrefix?: string;
  /** Import prefix of the pages directory (default derived from `appDirImportPrefix`). */
  readonly pagesDirImportPrefix?: string;
  /** Include legacy `pages/` entries (default true). */
  readonly includePages?: boolean;
}

function identifier(text: string, used: Set<string>): string {
  let base = text.replace(/[^A-Za-z0-9_$]+/g, "_").replace(/^_+|_+$/g, "") || "root";
  if (/^\d/.test(base)) base = `_${base}`;
  let name = base;
  let n = 2;
  while (used.has(name)) {
    name = `${base}_${n}`;
    n += 1;
  }
  used.add(name);
  return name;
}

/**
 * Generates a TanStack Router code-based tree from the manifest: `createRootRoute` (root layout +
 * template + loading/error/not-found), pathless layout routes for nested layouts, one `createRoute`
 * per page (Next page props built from `useParams`/`useSearch`), `createRouter` and the `Register` module augmentation.
 */
export function generateRouteTree(manifest: RouteManifest, options: RouteTreeOptions = {}): string {
  const appPrefix = (options.appDirImportPrefix ?? "../app").replace(/\/$/, "");
  const pagesPrefix = (options.pagesDirImportPrefix ?? appPrefix.replace(/app$/, "pages")).replace(/\/$/, "");
  const appDir = manifest.appDir ?? "app";
  const pagesDir = manifest.pagesDir ?? "pages";
  const importPath = (file: string): string => {
    if (file.startsWith(`${appDir}/`)) return `${appPrefix}/${file.slice(appDir.length + 1)}`;
    if (file.startsWith(`${pagesDir}/`)) return `${pagesPrefix}/${file.slice(pagesDir.length + 1)}`;
    return `${appPrefix}/../${file}`;
  };
  const used = new Set<string>(["rootRoute", "routeTree", "router", "withLayout", "withPageProps", "PageProps", "Outlet", "createElement", "createRootRoute", "createRoute", "createRouter"]);
  const imports: string[] = [];
  const moduleVars = new Map<string, string>();
  const moduleVar = (file: string): string => {
    let name = moduleVars.get(file);
    if (!name) {
      name = identifier(`Mod_${file.replace(/\.(tsx|ts|jsx|js|mdx|md)$/, "")}`, used);
      moduleVars.set(file, name);
      imports.push(`import ${name} from ${JSON.stringify(importPath(file))};`);
    }
    return name;
  };
  const rootLayout = manifest.rootLayout;
  const rootSegmentFiles = manifest.tree?.files;
  const rootTemplate = rootSegmentFiles?.template;
  const rootLoading = rootSegmentFiles?.loading;
  const rootError = rootSegmentFiles?.error;
  const rootNotFound = manifest.rootNotFound;
  const lines: string[] = [];
  const rootOptions: string[] = [];
  if (rootLayout) rootOptions.push(`  component: withLayout(${moduleVar(rootLayout)}${rootTemplate ? `, ${moduleVar(rootTemplate)}` : ""}),`);
  if (rootLoading) rootOptions.push(`  pendingComponent: ${moduleVar(rootLoading)},`);
  if (rootError) rootOptions.push(`  errorComponent: ${moduleVar(rootError)} as never,`);
  if (rootNotFound) rootOptions.push(`  notFoundComponent: ${moduleVar(rootNotFound)},`);
  lines.push(`const rootRoute = createRootRoute({`, ...rootOptions, `});`, "");

  // Pathless layout routes for every nested layout, keyed by the layout chain prefix.
  const layoutVars = new Map<string, string>();
  const layoutChildren = new Map<string, string[]>();
  layoutChildren.set("", []);
  const layoutVarFor = (chain: ReadonlyArray<string>): string => {
    const nested = rootLayout && chain[0] === rootLayout ? chain.slice(1) : chain;
    let parent = "rootRoute";
    let key = "";
    nested.forEach((layout) => {
      key = key ? `${key}|${layout}` : layout;
      let name = layoutVars.get(key);
      if (!name) {
        name = identifier(`layout_${layout.replace(/\/layout\.\w+$/, "").replace(/^app\//, "")}`, used);
        layoutVars.set(key, name);
        layoutChildren.set(key, []);
        const parentKey = key.includes("|") ? key.slice(0, key.lastIndexOf("|")) : "";
        layoutChildren.get(parentKey)?.push(name);
        lines.push(`const ${name} = createRoute({`, `  getParentRoute: () => ${parent},`, `  id: ${JSON.stringify(`/_${name}`)},`, `  component: withLayout(${moduleVar(layout)}),`, `});`, "");
      }
      parent = name;
    });
    return key;
  };
  const entries = sortEntries(manifest.entries).filter((e) => e.kind === "page" && !e.slot && !e.intercepting && (options.includePages !== false || !e.legacyPages));
  const pathsSeen = new Set<string>();
  for (const entry of entries) {
    if (pathsSeen.has(entry.tanstackPath)) continue;
    pathsSeen.add(entry.tanstackPath);
    const key = layoutVarFor(entry.layouts);
    const parent = key ? (layoutVars.get(key) ?? "rootRoute") : "rootRoute";
    const name = identifier(`route_${entry.urlPattern === "/" ? "index" : entry.urlPattern}`, used);
    layoutChildren.get(key)?.push(name);
    const catchAll = entry.params.find((p) => p.catchAll)?.name;
    const page = moduleVar(entry.file);
    const extras: string[] = [];
    if (entry.loading && entry.loading !== rootLoading) extras.push(`  pendingComponent: ${moduleVar(entry.loading)},`);
    if (entry.error && entry.error !== rootError) extras.push(`  errorComponent: ${moduleVar(entry.error)} as never,`);
    if (entry.notFound && entry.notFound !== rootNotFound) extras.push(`  notFoundComponent: ${moduleVar(entry.notFound)},`);
    lines.push(
      `const ${name} = createRoute({`,
      `  getParentRoute: () => ${parent},`,
      `  path: ${JSON.stringify(entry.tanstackPath)},`,
      `  component: withPageProps(${page} as never, () => ${name}${catchAll ? `, ${JSON.stringify(catchAll)}` : ""}),`,
      ...extras,
      `});`,
      "",
    );
  }
  const childrenExpr = (key: string): string => {
    const names = layoutChildren.get(key) ?? [];
    const parts = names.map((name) => {
      const childKey = [...layoutVars.entries()].find(([, v]) => v === name)?.[0];
      return childKey !== undefined ? `${name}.addChildren([${childrenExpr(childKey)}])` : name;
    });
    return parts.join(", ");
  };
  const header = [
    "// Generated by @yolo-rs/next from the App Router manifest. Do not edit.",
    'import { createElement, type ComponentType, type ReactNode } from "react";',
    'import { Outlet, createRootRoute, createRoute, createRouter } from "@tanstack/react-router";',
    ...imports,
    "",
    "type PageParams = Record<string, string | string[] | undefined>;",
    "type PageProps = { params: Promise<PageParams>; searchParams: Promise<PageParams> };",
    "type LayoutComponent = ComponentType<{ children?: ReactNode }>;",
    "",
    "function withLayout(Layout: LayoutComponent, Template?: LayoutComponent) {",
    "  return function YoloLayout() {",
    "    const outlet = createElement(Outlet);",
    "    return createElement(Layout, null, Template ? createElement(Template, null, outlet) : outlet);",
    "  };",
    "}",
    "",
    "function withPageProps(Page: ComponentType<PageProps>, getRoute: () => { useParams: () => unknown; useSearch: () => unknown }, catchAll?: string) {",
    "  return function YoloPage() {",
    "    const route = getRoute();",
    "    const raw = { ...(route.useParams() as Record<string, string | undefined>) };",
    "    const params: PageParams = { ...raw };",
    "    if (catchAll) {",
    '      const splat = raw._splat ?? "";',
    '      params[catchAll] = splat === "" ? undefined : splat.split("/");',
    "      delete params._splat;",
    "    }",
    "    return createElement(Page, { params: Promise.resolve(params), searchParams: Promise.resolve(route.useSearch() as PageParams) });",
    "  };",
    "}",
    "",
  ];
  const footer = [
    `export const routeTree = rootRoute.addChildren([${childrenExpr("")}]);`,
    "",
    'export const router = createRouter({ routeTree, defaultPreload: "intent" });',
    "",
    'declare module "@tanstack/react-router" {',
    "  interface Register {",
    "    router: typeof router;",
    "  }",
    "}",
    "",
  ];
  return [...header, ...lines, ...footer].join("\n");
}

/** Convenience: the intercept spec of an entry when it intercepts another route. */
export function interceptOf(entry: RouteEntry): InterceptSpec | undefined {
  return entry.intercepting;
}
