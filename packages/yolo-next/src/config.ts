// SPDX-License-Identifier: Apache-2.0
// @yolo-rs/next/config: next.config.* loading (ESM/CJS/function forms), Next.js 16 renames and removed
// keys, and normalization into the Yolo config (CDN plan, images policy, bundler plan, server/react).

import { existsSync, readFileSync, statSync } from "node:fs";
import { basename, isAbsolute, join, resolve } from "node:path";

import type {
  BunBundlerPlan,
  CacheLifeProfile,
  CdnPlan,
  Diagnostic,
  HeaderRule,
  ImageOptimizerPolicy,
  ImagesConfig,
  NextCompatConfig,
  OutputMode,
  RemotePattern,
  ResolvedYoloConfig,
  Rewrites,
  RouteRule,
  YoloWebConfig,
} from "./types.ts";
import { BUILTIN_CACHE_LIFE, CDN_DEFAULTS, IMAGES_DEFAULTS } from "./types.ts";
import { translateTurbopack } from "./turbopack.ts";

export type { ResolvedYoloConfig } from "./types.ts";

/**
 * Input of `defineYoloConfig`: a Next.js config plus Yolo-only keys at the top level (`server`,
 * `react`, `theme`, `cdn`, `publicEnvPrefixes`, `appDir`, ...) or grouped under `yolo`. Keys that
 * exist in both models (`output`, `distDir`, `images`, `redirects`, ...) keep their Next.js meaning
 * at the top level and must go under `yolo` to be given in the Yolo form.
 */
export type YoloConfigInput = NextCompatConfig &
  Partial<Omit<YoloWebConfig, keyof NextCompatConfig>> & {
    readonly yolo?: Partial<YoloWebConfig>;
  };

const YOLO_ONLY_KEYS = ["appDir", "pagesDir", "proxyFile", "server", "react", "theme", "cdn", "bundler", "publicEnvPrefixes"] as const;

type Mutable<T> = { -readonly [K in keyof T]: T[K] };

// ---------------------------------------------------------------------------
// Normalization of next.config keys (Next.js 16 renames + removed keys)
// ---------------------------------------------------------------------------

/** Result of `normalizeNextConfig`. */
export interface NormalizedNextConfig {
  readonly config: NextCompatConfig;
  readonly diagnostics: Diagnostic[];
  readonly renamed: ReadonlyArray<readonly [string, string]>;
  readonly removed: ReadonlyArray<string>;
}

const REMOVED_KEYS: ReadonlyArray<readonly [string, string, string]> = [
  ["eslint", "`eslint` was removed in Next.js 16 (`next lint` is gone)", "Run `bun lint` (oxlint/biome) in CI instead"],
  ["amp", "`amp` was removed in Next.js 16", "Delete the AMP pages"],
  ["serverRuntimeConfig", "`serverRuntimeConfig` was removed in Next.js 16", "Read `Bun.env` on the server"],
  ["publicRuntimeConfig", "`publicRuntimeConfig` was removed in Next.js 16", "Use `NEXT_PUBLIC_*` / `YOLO_PUBLIC_*` env vars inlined at build time"],
];

const isObject = (value: unknown): value is Record<string, unknown> => typeof value === "object" && value !== null && !Array.isArray(value);

/** Applies the Next.js 16 renames and reports removed keys and `webpack()` on a raw config object. */
export function normalizeNextConfig(raw: NextCompatConfig | Record<string, unknown>): NormalizedNextConfig {
  const cfg: Record<string, unknown> = { ...(raw as Record<string, unknown>) };
  const diagnostics: Diagnostic[] = [];
  const renamed: Array<readonly [string, string]> = [];
  const removed: string[] = [];
  const experimental: Record<string, unknown> = isObject(cfg.experimental) ? { ...cfg.experimental } : {};
  const rename = (from: string, to: string, value: unknown): void => {
    if (value === undefined) return;
    if (cfg[to] === undefined) cfg[to] = value;
    renamed.push([from, to]);
    diagnostics.push({ severity: "info", code: "next-config-renamed-key", message: `\`${from}\` is \`${to}\` in Next.js 16; migrated`, hint: `Rename \`${from}\` to \`${to}\`` });
  };
  if (experimental.turbo !== undefined) {
    rename("experimental.turbo", "turbopack", experimental.turbo);
    delete experimental.turbo;
  }
  if (experimental.serverComponentsExternalPackages !== undefined) {
    rename("experimental.serverComponentsExternalPackages", "serverExternalPackages", experimental.serverComponentsExternalPackages);
    delete experimental.serverComponentsExternalPackages;
  }
  if (experimental.typedRoutes !== undefined) {
    rename("experimental.typedRoutes", "typedRoutes", experimental.typedRoutes);
    delete experimental.typedRoutes;
  }
  if (experimental.reactCompiler !== undefined) {
    rename("experimental.reactCompiler", "reactCompiler", experimental.reactCompiler);
    delete experimental.reactCompiler;
  }
  if (experimental.cacheLife !== undefined) {
    rename("experimental.cacheLife", "cacheLife", experimental.cacheLife);
    delete experimental.cacheLife;
  }
  for (const key of ["dynamicIO", "useCache", "ppr"]) {
    if (experimental[key] === undefined) continue;
    const value = experimental[key];
    if (value === true || value === "incremental") rename(`experimental.${key}`, "cacheComponents", true);
    else renamed.push([`experimental.${key}`, "cacheComponents"]);
    delete experimental[key];
  }
  if (cfg.skipMiddlewareUrlNormalize !== undefined) {
    rename("skipMiddlewareUrlNormalize", "skipProxyUrlNormalize", cfg.skipMiddlewareUrlNormalize);
    delete cfg.skipMiddlewareUrlNormalize;
  }
  if (isObject(cfg.images) && Array.isArray(cfg.images.domains) && cfg.images.domains.length) {
    const patterns: RemotePattern[] = [...((cfg.images.remotePatterns as RemotePattern[] | undefined) ?? [])];
    for (const host of cfg.images.domains) if (typeof host === "string") patterns.push({ hostname: host });
    cfg.images = { ...cfg.images, remotePatterns: patterns };
    delete (cfg.images as Record<string, unknown>).domains;
    renamed.push(["images.domains", "images.remotePatterns"]);
    diagnostics.push({ severity: "warning", code: "next-config-images-domains-deprecated", message: "`images.domains` is deprecated; converted to `images.remotePatterns`", hint: "Use `remotePatterns` with `protocol` and `pathname`" });
  }
  cfg.experimental = experimental;
  for (const [key, message, hint] of REMOVED_KEYS) {
    if (cfg[key] === undefined) continue;
    removed.push(key);
    diagnostics.push({ severity: "warning", code: `next-config-removed-${key.toLowerCase()}`, message, hint });
  }
  if (cfg.webpack !== undefined) {
    removed.push("webpack");
    diagnostics.push({ severity: "warning", code: "next-config-webpack-unsupported", message: "`webpack()` has no effect: Bun.build replaces webpack and Turbopack", hint: "Translate the loaders to `turbopack.rules` or Bun plugins (`yolo.bundler.ts`)" });
  }
  if (cfg.exportPathMap !== undefined) {
    diagnostics.push({ severity: "warning", code: "next-config-export-path-map-unsupported", message: "`exportPathMap` is ignored; use `generateStaticParams` and route files", hint: "Delete `exportPathMap`" });
  }
  if (cfg.i18n !== undefined && cfg.i18n !== null) {
    diagnostics.push({ severity: "info", code: "next-config-i18n-pages-router", message: "`i18n` only applies to the Pages Router; App Router apps route locales with a `[lang]` segment" });
  }
  return { config: cfg as NextCompatConfig, diagnostics, renamed, removed };
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Parses a Server Actions `bodySizeLimit` (`"2mb"`, `"500kb"`, `1048576`) into bytes; default 1 MiB. */
export function parseBodySizeLimit(value: unknown): number {
  if (typeof value === "number" && Number.isFinite(value) && value > 0) return Math.floor(value);
  if (typeof value !== "string") return 1_048_576;
  const match = /^\s*(\d+(?:\.\d+)?)\s*(b|kb|mb|gb|tb)?\s*$/i.exec(value);
  if (!match) return 1_048_576;
  const amount = Number(match[1]);
  const unit = (match[2] ?? "b").toLowerCase();
  const factor = unit === "kb" ? 1024 : unit === "mb" ? 1024 ** 2 : unit === "gb" ? 1024 ** 3 : unit === "tb" ? 1024 ** 4 : 1;
  return Math.floor(amount * factor);
}

function outputMode(output: NextCompatConfig["output"]): OutputMode {
  if (output === "standalone") return "standalone";
  if (output === "export") return "export";
  return "server";
}

function trimSlash(value: string): string {
  return value.replace(/\/+$/, "");
}

/** Builds the CDN plan of a Next.js config. */
export function cdnPlanFor(next: NextCompatConfig, overrides: Partial<CdnPlan> = {}): CdnPlan {
  const basePath = next.basePath ? trimSlash(next.basePath) : "";
  const assetPrefix = next.assetPrefix ? trimSlash(next.assetPrefix) : "";
  return {
    ...CDN_DEFAULTS,
    assetPrefix,
    basePath,
    staticRoute: `${basePath}/_yolo/static`,
    compatStaticRoutes: [`${basePath}/_next/static`],
    fontsRoute: `${basePath}/_yolo/fonts`,
    deploymentId: next.deploymentId || undefined,
    output: outputMode(next.output),
    expireTime: next.expireTime ?? CDN_DEFAULTS.expireTime,
    compress: next.compress ?? true,
    poweredByHeader: next.poweredByHeader ?? true,
    generateEtags: next.generateEtags ?? true,
    trailingSlash: next.trailingSlash ?? false,
    ...overrides,
  };
}

function remotePattern(value: RemotePattern | URL): RemotePattern {
  if (value instanceof URL) {
    const protocol = value.protocol.replace(":", "");
    return {
      protocol: protocol === "http" || protocol === "https" ? protocol : undefined,
      hostname: value.hostname,
      port: value.port || undefined,
      pathname: value.pathname && value.pathname !== "/" ? value.pathname : undefined,
      search: value.search || undefined,
    };
  }
  return value;
}

/** Builds the image optimizer policy of a Next.js `images` config. */
export function imagesPolicyFor(images: ImagesConfig | undefined, basePath = ""): ImageOptimizerPolicy {
  const cfg = images ?? {};
  const base = trimSlash(basePath);
  const compatRoutes = [`${base}/_next/image`];
  if (cfg.path && cfg.path !== IMAGES_DEFAULTS.path && cfg.path !== `${base}/_yolo/image`) compatRoutes.push(cfg.path);
  const loaderKind: ImageOptimizerPolicy["loader"] = cfg.loaderFile ? "custom" : (cfg.loader ?? "default");
  const minimumCacheTtl = cfg.minimumCacheTTL ?? IMAGES_DEFAULTS.minimumCacheTTL;
  return {
    route: `${base}/_yolo/image`,
    compatRoutes,
    loader: loaderKind,
    loaderFile: cfg.loaderFile || undefined,
    formats: [...(cfg.formats ?? IMAGES_DEFAULTS.formats)],
    deviceSizes: [...(cfg.deviceSizes ?? IMAGES_DEFAULTS.deviceSizes)],
    imageSizes: [...(cfg.imageSizes ?? IMAGES_DEFAULTS.imageSizes)],
    qualities: [...(cfg.qualities ?? IMAGES_DEFAULTS.qualities)],
    minimumCacheTtl,
    cacheControl: `public, max-age=${minimumCacheTtl}, must-revalidate`,
    allowSvg: cfg.dangerouslyAllowSVG ?? IMAGES_DEFAULTS.dangerouslyAllowSVG,
    allowLocalIp: cfg.dangerouslyAllowLocalIP ?? IMAGES_DEFAULTS.dangerouslyAllowLocalIP,
    contentSecurityPolicy: cfg.contentSecurityPolicy ?? IMAGES_DEFAULTS.contentSecurityPolicy,
    contentDispositionType: cfg.contentDispositionType ?? IMAGES_DEFAULTS.contentDispositionType,
    remotePatterns: (cfg.remotePatterns ?? []).map(remotePattern),
    localPatterns: cfg.localPatterns ? [...cfg.localPatterns] : undefined,
    maximumRedirects: cfg.maximumRedirects ?? IMAGES_DEFAULTS.maximumRedirects,
    maximumResponseBody: cfg.maximumResponseBody ?? IMAGES_DEFAULTS.maximumResponseBody,
    unoptimized: cfg.unoptimized ?? IMAGES_DEFAULTS.unoptimized,
  };
}

/** Merges custom `cacheLife` profiles with the built-ins; missing fields inherit from `default`. */
export function cacheLifeProfiles(custom: Record<string, CacheLifeProfile> | undefined): Record<string, CacheLifeProfile> {
  const out: Record<string, CacheLifeProfile> = {};
  for (const [name, profile] of Object.entries(BUILTIN_CACHE_LIFE)) out[name] = { ...profile };
  const base = BUILTIN_CACHE_LIFE.default ?? { stale: 300, revalidate: 900, expire: 0xfffffffe };
  for (const [name, profile] of Object.entries(custom ?? {})) {
    out[name] = {
      stale: profile.stale ?? base.stale,
      revalidate: profile.revalidate ?? base.revalidate,
      expire: profile.expire ?? base.expire,
    };
  }
  return out;
}

function normalizeRewrites(value: ReadonlyArray<RouteRule> | Rewrites | undefined): Rewrites {
  if (!value) return { beforeFiles: [], afterFiles: [], fallback: [] };
  if (Array.isArray(value)) return { beforeFiles: [], afterFiles: [...value], fallback: [] };
  const rewrites = value as Rewrites;
  return { beforeFiles: [...(rewrites.beforeFiles ?? [])], afterFiles: [...(rewrites.afterFiles ?? [])], fallback: [...(rewrites.fallback ?? [])] };
}

function envRecord(env: NextCompatConfig["env"]): Record<string, string> {
  const out: Record<string, string> = {};
  for (const [key, value] of Object.entries(env ?? {})) if (typeof value === "string") out[key] = value;
  return out;
}

interface Detected {
  readonly appDir?: string;
  readonly pagesDir?: string;
  readonly proxyFile?: string;
}

interface ResolvedRules {
  readonly redirects: ReadonlyArray<RouteRule>;
  readonly rewrites: Rewrites;
  readonly headers: ReadonlyArray<HeaderRule>;
}

function pickOverrides(input: Record<string, unknown>): Partial<YoloWebConfig> {
  const out: Record<string, unknown> = {};
  for (const key of YOLO_ONLY_KEYS) if (input[key] !== undefined) out[key] = input[key];
  const yolo = input.yolo;
  if (isObject(yolo)) for (const [key, value] of Object.entries(yolo)) if (value !== undefined) out[key] = value;
  return out as Partial<YoloWebConfig>;
}

function buildWebConfig(next: NextCompatConfig, overrides: Partial<YoloWebConfig>, detected: Detected, rules: ResolvedRules | undefined): YoloWebConfig {
  const experimental = next.experimental ?? {};
  const serverActions = isObject(experimental.serverActions) ? experimental.serverActions : {};
  const cdn = cdnPlanFor(next, overrides.cdn ?? {});
  const bundler: BunBundlerPlan = overrides.bundler ?? translateTurbopack(next);
  const defaultPrefixes = ["NEXT_PUBLIC_", "YOLO_PUBLIC_"];
  const web: Mutable<YoloWebConfig> = {
    appDir: overrides.appDir ?? detected.appDir ?? "app",
    pagesDir: overrides.pagesDir ?? detected.pagesDir,
    distDir: overrides.distDir ?? (next.distDir && next.distDir !== ".next" ? next.distDir : ".yolo"),
    output: overrides.output ?? outputMode(next.output),
    server: {
      port: Number(Bun.env.PORT ?? 3000),
      hostname: "0.0.0.0",
      hmr: true,
      browserConsole: true,
      compress: next.compress ?? true,
      poweredByHeader: next.poweredByHeader ?? true,
      generateEtags: next.generateEtags ?? true,
      actionBodySizeLimit: parseBodySizeLimit(serverActions.bodySizeLimit ?? "1mb"),
      actionAllowedOrigins: Array.isArray(serverActions.allowedOrigins) ? serverActions.allowedOrigins.filter((o): o is string => typeof o === "string") : [],
      ...(overrides.server ?? {}),
    },
    react: {
      strictMode: next.reactStrictMode !== false,
      compiler: Boolean(next.reactCompiler),
      fastRefresh: true,
      cacheComponents: next.cacheComponents === true,
      typedRoutes: next.typedRoutes === true,
      ...(overrides.react ?? {}),
    },
    theme: { preset: "yolo-neutral", dark: true, ...(overrides.theme ?? {}) },
    cdn,
    images: overrides.images ?? imagesPolicyFor(next.images, cdn.basePath),
    bundler,
    redirects: rules ? rules.redirects : (overrides.redirects ?? (Array.isArray(next.redirects) ? (next.redirects as ReadonlyArray<RouteRule>) : [])),
    rewrites: rules ? rules.rewrites : (overrides.rewrites ?? normalizeRewrites(typeof next.rewrites === "function" ? undefined : (next.rewrites as Rewrites | undefined))),
    headers: rules ? rules.headers : (overrides.headers ?? (Array.isArray(next.headers) ? (next.headers as ReadonlyArray<HeaderRule>) : [])),
    publicEnvPrefixes: overrides.publicEnvPrefixes ?? defaultPrefixes,
    env: { ...envRecord(next.env), ...(overrides.env ?? {}) },
    cacheLife: cacheLifeProfiles({ ...(next.cacheLife ?? {}), ...(overrides.cacheLife ?? {}) }),
    i18n: overrides.i18n ?? next.i18n ?? undefined,
    transpilePackages: overrides.transpilePackages ?? [...(next.transpilePackages ?? [])],
    serverExternalPackages: overrides.serverExternalPackages ?? [...(next.serverExternalPackages ?? [])],
    pageExtensions: overrides.pageExtensions ?? [...(next.pageExtensions ?? ["tsx", "ts", "jsx", "js"])],
    trailingSlash: overrides.trailingSlash ?? next.trailingSlash ?? false,
    proxyFile: overrides.proxyFile ?? detected.proxyFile,
  };
  if (web.pagesDir === undefined) delete web.pagesDir;
  if (web.proxyFile === undefined) delete web.proxyFile;
  if (web.i18n === undefined) delete web.i18n;
  return web;
}

function isResolved(config: YoloConfigInput | ResolvedYoloConfig): config is ResolvedYoloConfig {
  return isObject((config as { next?: unknown }).next) && Array.isArray((config as { diagnostics?: unknown }).diagnostics);
}

/**
 * Normalizes a Next.js + Yolo config synchronously: defaults, Next.js 16 renames and removed-key
 * diagnostics, CDN plan, images policy, bundler plan. Function-valued `redirects`/`rewrites`/`headers`
 * stay on `.next` for `resolveYoloConfig` to await.
 */
export function defineYoloConfig(config: YoloConfigInput | ResolvedYoloConfig): ResolvedYoloConfig {
  if (isResolved(config)) return config;
  const input = config as unknown as Record<string, unknown>;
  const rest: Record<string, unknown> = {};
  for (const [key, value] of Object.entries(input)) if (key !== "yolo" && !(YOLO_ONLY_KEYS as ReadonlyArray<string>).includes(key)) rest[key] = value;
  const normalized = normalizeNextConfig(rest);
  const web = buildWebConfig(normalized.config, pickOverrides(input), {}, undefined);
  return { ...web, next: normalized.config, diagnostics: normalized.diagnostics };
}

const APP_DIRS = ["app", "src/app"];
const PAGES_DIRS = ["pages", "src/pages"];
const PROXY_FILES = ["proxy.ts", "proxy.js", "proxy.mts", "proxy.mjs", "src/proxy.ts", "src/proxy.js", "middleware.ts", "middleware.js", "src/middleware.ts", "src/middleware.js"];

function isDir(path: string): boolean {
  try {
    return statSync(path).isDirectory();
  } catch {
    return false;
  }
}

/** Detects `appDir`, `pagesDir` and the proxy file of a project root. */
export function detectProjectLayout(root: string): Detected {
  const appDir = APP_DIRS.find((dir) => isDir(join(root, dir)));
  const pagesDir = PAGES_DIRS.find((dir) => isDir(join(root, dir)));
  const proxyFile = PROXY_FILES.find((file) => existsSync(join(root, file)));
  return { appDir, pagesDir, proxyFile };
}

/** Options of `resolveYoloConfig`. */
export interface ResolveYoloConfigOptions {
  readonly root: string;
  /** Phase passed to function-valued configs (default `phase-production-build`). */
  readonly phase?: string;
}

/**
 * Awaits `redirects()`, `rewrites()` and `headers()`, detects `app/`, `pages/` and the proxy file
 * under `root`, and returns the fully resolved Yolo config.
 */
export async function resolveYoloConfig(config: YoloConfigInput | ResolvedYoloConfig, options: ResolveYoloConfigOptions): Promise<ResolvedYoloConfig> {
  const base = isResolved(config) ? config : defineYoloConfig(config);
  const next = base.next;
  const diagnostics: Diagnostic[] = [...base.diagnostics];
  const root = resolve(options.root);
  const detected = detectProjectLayout(root);
  const redirects = typeof next.redirects === "function" ? [...(await next.redirects())] : (base.redirects as RouteRule[]);
  const rewrites = typeof next.rewrites === "function" ? normalizeRewrites(await next.rewrites()) : base.rewrites;
  const headers = typeof next.headers === "function" ? [...(await next.headers())] : (base.headers as HeaderRule[]);
  const overrides: Mutable<Partial<YoloWebConfig>> = {
    server: base.server,
    react: base.react,
    theme: base.theme,
    publicEnvPrefixes: base.publicEnvPrefixes,
    distDir: base.distDir,
    output: base.output,
    trailingSlash: base.trailingSlash,
  };
  if (base.appDir !== "app") overrides.appDir = base.appDir;
  if (base.pagesDir !== undefined) overrides.pagesDir = base.pagesDir;
  if (base.proxyFile !== undefined) overrides.proxyFile = base.proxyFile;
  const web = buildWebConfig(next, overrides, detected, { redirects, rewrites, headers });
  if (!isDir(join(root, web.appDir))) {
    diagnostics.push({ severity: web.pagesDir ? "info" : "warning", code: "app-dir-missing", message: `no \`${web.appDir}\` directory under ${root}`, hint: "Create `app/layout.tsx` and `app/page.tsx`" });
  }
  if (web.proxyFile && basename(web.proxyFile).startsWith("middleware.")) {
    diagnostics.push({ severity: "warning", code: "proxy-legacy-middleware-name", message: "`middleware.ts` is deprecated in Next.js 16; rename it to `proxy.ts` and export `proxy`", file: web.proxyFile });
  }
  return { ...web, next, diagnostics };
}

// ---------------------------------------------------------------------------
// next.config loading
// ---------------------------------------------------------------------------

const CONFIG_FILES = ["yolo.config.ts", "yolo.config.js", "next.config.ts", "next.config.mts", "next.config.mjs", "next.config.js", "next.config.cjs"];

/** Result of `loadNextConfig`. */
export interface LoadedNextConfig {
  readonly config: NextCompatConfig;
  /** Config file path relative to `root` (posix), when found. */
  readonly file?: string;
  /** Plugin wrappers detected in the source (`withMDX`, `withSentryConfig`, ...), outermost first. */
  readonly wrappers: string[];
}

/** Finds the config file of a project (`yolo.config.*` first, then `next.config.*`). */
export function findConfigFile(root: string): string | undefined {
  for (const name of CONFIG_FILES) if (existsSync(join(root, name))) return name;
  return undefined;
}

function detectWrappers(source: string): string[] {
  const out: string[] = [];
  for (const match of source.matchAll(/\b(with[A-Z][\w$]*)\s*\(/g)) {
    const name = match[1] ?? "";
    if (!out.includes(name)) out.push(name);
  }
  return out;
}

/**
 * Loads `next.config.{ts,mts,mjs,js,cjs}` or `yolo.config.{ts,js}` with a dynamic import (never a
 * shell): object, `(phase, { defaultConfig }) => config | Promise<config>` and CJS `module.exports`
 * default exports are all accepted; a `defineYoloConfig` result yields its `.next` config.
 */
export async function loadNextConfig(root: string, options: { phase?: string } = {}): Promise<LoadedNextConfig> {
  const absRoot = resolve(root);
  const file = findConfigFile(absRoot);
  if (!file) return { config: {}, wrappers: [] };
  const path = join(absRoot, file);
  const source = readFileSync(path, "utf8");
  const wrappers = detectWrappers(source);
  const imported = (await import(Bun.pathToFileURL(path).href)) as Record<string, unknown>;
  let value: unknown = imported.default ?? imported;
  if (isObject(value) && "default" in value && Object.keys(value).length === 1) value = (value as { default: unknown }).default;
  if (typeof value === "function") {
    value = await (value as (phase: string, ctx: { defaultConfig: Record<string, unknown> }) => unknown)(options.phase ?? "phase-production-build", { defaultConfig: {} });
  }
  if (isObject(value) && "__esModule" in value && isObject(value.default)) value = value.default;
  if (!isObject(value)) throw new Error(`${file} does not export a config object`);
  const config = isResolved(value as YoloConfigInput) ? (value as unknown as ResolvedYoloConfig).next : (value as NextCompatConfig);
  return { config, file, wrappers };
}

// ---------------------------------------------------------------------------
// Env files
// ---------------------------------------------------------------------------

/**
 * Env files in Next.js precedence order (highest first): `.env.<NODE_ENV>.local` and `.env.local`
 * (both skipped in `test`), `.env.<NODE_ENV>`, `.env`. Only existing files are returned (absolute paths).
 */
export function nextEnvFiles(root: string, nodeEnv: string = Bun.env.NODE_ENV ?? "development"): string[] {
  const env = nodeEnv || "development";
  const names = [`.env.${env}.local`, ".env.local", `.env.${env}`, ".env"];
  const list = env === "test" ? names.filter((n) => !n.endsWith(".local")) : names;
  return list.map((name) => join(resolve(root), name)).filter((path) => existsSync(path));
}

/** Parses a dotenv file (`KEY=value`, quotes, `export`, comments). */
export function parseEnvFile(text: string): Record<string, string> {
  const out: Record<string, string> = {};
  for (const rawLine of text.split(/\r?\n/)) {
    const line = rawLine.trim();
    if (!line || line.startsWith("#")) continue;
    const match = /^(?:export\s+)?([A-Za-z_][A-Za-z0-9_.-]*)\s*=\s*(.*)$/.exec(line);
    if (!match) continue;
    let value = match[2] ?? "";
    const quote = value.charAt(0);
    if ((quote === '"' || quote === "'" || quote === "`") && value.endsWith(quote) && value.length >= 2) {
      value = value.slice(1, -1);
      if (quote === '"') value = value.replace(/\\n/g, "\n").replace(/\\r/g, "\r").replace(/\\t/g, "\t").replace(/\\(["\\])/g, "$1");
    } else {
      const hash = value.indexOf(" #");
      if (hash !== -1) value = value.slice(0, hash);
      value = value.trim();
    }
    out[match[1] ?? ""] = value;
  }
  return out;
}

/** Loads the env files of a root in precedence order (first definition wins), without touching `Bun.env`. */
export function loadEnvFiles(root: string, nodeEnv?: string): Record<string, string> {
  const out: Record<string, string> = {};
  for (const file of nextEnvFiles(root, nodeEnv)) {
    for (const [key, value] of Object.entries(parseEnvFile(readFileSync(file, "utf8")))) if (!(key in out)) out[key] = value;
  }
  return out;
}

/** `{ "process.env.NEXT_PUBLIC_X": "\"value\"" }` for every env key matching one of the prefixes. */
export function publicEnvDefine(prefixes: ReadonlyArray<string>, env: Record<string, string | undefined> = Bun.env): Record<string, string> {
  const out: Record<string, string> = {};
  for (const key of Object.keys(env).sort()) {
    const value = env[key];
    if (value === undefined || !prefixes.some((prefix) => key.startsWith(prefix))) continue;
    out[`process.env.${key}`] = JSON.stringify(value);
  }
  return out;
}

// ---------------------------------------------------------------------------
// Serialization (yolo.config.ts)
// ---------------------------------------------------------------------------

function literal(value: unknown, indent: string): string {
  if (value === undefined) return "undefined";
  if (value === null || typeof value === "number" || typeof value === "boolean") return String(value);
  if (typeof value === "string") return JSON.stringify(value);
  if (value instanceof RegExp) return value.toString();
  if (value instanceof URL) return `new URL(${JSON.stringify(value.href)})`;
  if (typeof value === "function") return "undefined";
  if (Array.isArray(value)) {
    if (!value.length) return "[]";
    const inner = indent + "  ";
    return `[\n${value.map((item) => `${inner}${literal(item, inner)}`).join(",\n")},\n${indent}]`;
  }
  if (isObject(value)) {
    const entries = Object.entries(value).filter(([, v]) => v !== undefined && typeof v !== "function");
    if (!entries.length) return "{}";
    const inner = indent + "  ";
    return `{\n${entries.map(([k, v]) => `${inner}${/^[A-Za-z_$][\w$]*$/.test(k) ? k : JSON.stringify(k)}: ${literal(v, inner)}`).join(",\n")},\n${indent}}`;
  }
  return JSON.stringify(String(value));
}

/** Serializes a resolved config as `yolo.config.ts` (`defineYoloConfig({...})`), re-emitting resolved rule functions as `async () => <json>`. */
export function serializeYoloConfig(config: ResolvedYoloConfig, options: { root?: string } = {}): string {
  const next: Record<string, unknown> = { ...(config.next as Record<string, unknown>) };
  for (const key of ["redirects", "rewrites", "headers", "webpack", "generateBuildId", "exportPathMap", "eslint", "amp", "serverRuntimeConfig", "publicRuntimeConfig"]) delete next[key];
  // `turbopack.root` is an absolute, machine-specific path (`process.cwd()`); the project root is implied.
  if (isObject(next.turbopack) && typeof next.turbopack.root === "string" && (isAbsolute(next.turbopack.root) || (options.root && resolve(next.turbopack.root) === resolve(options.root)))) {
    next.turbopack = { ...next.turbopack };
    delete (next.turbopack as Record<string, unknown>).root;
  }
  if (isObject(next.experimental) && !Object.keys(next.experimental).length) delete next.experimental;
  const lines: string[] = [];
  const body = literal(next, "");
  const inner = body === "{}" ? [] : body.slice(2, -2).split("\n");
  lines.push(...inner);
  const emit = (name: string, value: unknown): void => {
    lines.push(`  ${name}: async () => ${literal(value, "  ")},`);
  };
  if (config.redirects.length) emit("redirects", config.redirects);
  if ((config.rewrites.beforeFiles?.length ?? 0) + (config.rewrites.afterFiles?.length ?? 0) + (config.rewrites.fallback?.length ?? 0) > 0) emit("rewrites", config.rewrites);
  if (config.headers.length) emit("headers", config.headers);
  const yolo: Record<string, unknown> = { theme: config.theme };
  if (config.appDir !== "app") yolo.appDir = config.appDir;
  if (config.publicEnvPrefixes.join(",") !== "NEXT_PUBLIC_,YOLO_PUBLIC_") yolo.publicEnvPrefixes = config.publicEnvPrefixes;
  lines.push(`  yolo: ${literal(yolo, "  ")},`);
  return [
    "// Generated by @yolo-rs/next migrate from next.config.*; edit freely.",
    'import { defineYoloConfig } from "@yolo-rs/next/config";',
    "",
    "export default defineYoloConfig({",
    ...lines,
    "});",
    "",
  ].join("\n");
}

/** Absolute path of a config-relative value. */
export function absoluteFromRoot(root: string, path: string): string {
  return isAbsolute(path) ? path : join(root, path);
}
