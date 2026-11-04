// SPDX-License-Identifier: MIT OR Apache-2.0
// `next.config.{js,mjs,ts,mts,cjs}` of a project, loaded the way Next.js does (object, function or async
// function of the phase) and normalized into the keys the framework honours. Keys the framework ignores are
// reported, not silently dropped, so a migration knows what it loses.
import { existsSync } from "node:fs";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

export const PHASE_DEVELOPMENT_SERVER = "phase-development-server";
export const PHASE_PRODUCTION_BUILD = "phase-production-build";
export const PHASE_PRODUCTION_SERVER = "phase-production-server";
export const PHASE_EXPORT = "phase-export";
export const PHASE_TEST = "phase-test";

export type Phase =
  | typeof PHASE_DEVELOPMENT_SERVER
  | typeof PHASE_PRODUCTION_BUILD
  | typeof PHASE_PRODUCTION_SERVER
  | typeof PHASE_EXPORT
  | typeof PHASE_TEST;

/** Order Next.js probes config files in. */
export const CONFIG_FILES = ["next.config.js", "next.config.mjs", "next.config.ts", "next.config.mts", "next.config.cjs"];

export interface RouteHas {
  type: "header" | "cookie" | "host" | "query";
  key: string;
  value?: string;
}

export interface Rewrite {
  source: string;
  destination: string;
  basePath?: false;
  locale?: false;
  has?: RouteHas[];
  missing?: RouteHas[];
}

export interface Redirect extends Rewrite {
  permanent?: boolean;
  statusCode?: number;
}

export interface HeaderRule {
  source: string;
  headers: { key: string; value: string }[];
  basePath?: false;
  locale?: false;
  has?: RouteHas[];
  missing?: RouteHas[];
}

export interface RewriteGroups {
  beforeFiles: Rewrite[];
  afterFiles: Rewrite[];
  fallback: Rewrite[];
}

export interface RemotePattern {
  protocol?: "http" | "https";
  hostname: string;
  port?: string;
  pathname?: string;
  search?: string;
}

export interface LocalPattern {
  pathname?: string;
  search?: string;
}

export type ImageLoader = "default" | "imgix" | "cloudinary" | "akamai" | "cloudflare" | "fastly" | "vercel" | "custom";

export interface ImagesConfig {
  loader?: ImageLoader;
  loaderFile?: string;
  path?: string;
  deviceSizes?: number[];
  imageSizes?: number[];
  qualities?: number[];
  formats?: ("image/avif" | "image/webp")[];
  minimumCacheTTL?: number;
  remotePatterns?: (RemotePattern | URL)[];
  localPatterns?: LocalPattern[];
  /** Deprecated in Next 16; converted to `remotePatterns` (https). */
  domains?: string[];
  dangerouslyAllowSVG?: boolean;
  contentSecurityPolicy?: string;
  contentDispositionType?: "inline" | "attachment";
  unoptimized?: boolean;
  disableStaticImages?: boolean;
}

export type AliasTarget = string | string[] | Record<string, string | string[]>;

export interface TurbopackRule {
  loaders: (string | { loader: string; options?: Record<string, unknown> })[];
  as?: string;
  condition?: unknown;
}

export interface TurbopackConfig {
  root?: string;
  resolveAlias?: Record<string, AliasTarget>;
  resolveExtensions?: string[];
  rules?: Record<string, TurbopackRule | TurbopackRule[]>;
  conditions?: Record<string, unknown>;
  moduleIds?: "named" | "deterministic";
  treeShaking?: boolean;
  memoryLimit?: number;
  debugIds?: boolean;
  sourceMaps?: boolean;
}

/** The Next.js config surface. Unknown keys are kept and reported. */
export interface NextConfig {
  basePath?: string;
  assetPrefix?: string;
  deploymentId?: string;
  trailingSlash?: boolean;
  skipTrailingSlashRedirect?: boolean;
  skipProxyUrlNormalize?: boolean;
  poweredByHeader?: boolean;
  compress?: boolean;
  distDir?: string;
  output?: "standalone" | "export";
  env?: Record<string, string | undefined>;
  images?: ImagesConfig;
  turbopack?: TurbopackConfig;
  rewrites?: () => Promise<Rewrite[] | Partial<RewriteGroups>> | Rewrite[] | Partial<RewriteGroups>;
  redirects?: () => Promise<Redirect[]> | Redirect[];
  headers?: () => Promise<HeaderRule[]> | HeaderRule[];
  transpilePackages?: string[];
  serverExternalPackages?: string[];
  reactStrictMode?: boolean | null;
  reactCompiler?: boolean | { compilationMode?: string };
  typedRoutes?: boolean;
  cacheComponents?: boolean;
  pageExtensions?: string[];
  productionBrowserSourceMaps?: boolean;
  generateBuildId?: () => Promise<string | null> | string | null;
  generateEtags?: boolean;
  crossOrigin?: "anonymous" | "use-credentials";
  allowedDevOrigins?: string[];
  serverRuntimeConfig?: Record<string, unknown>;
  publicRuntimeConfig?: Record<string, unknown>;
  experimental?: Record<string, unknown>;
  webpack?: unknown;
  i18n?: unknown;
  eslint?: unknown;
  typescript?: unknown;
  outputFileTracingRoot?: string;
  [key: string]: unknown;
}

export interface Unsupported {
  key: string;
  reason: string;
}

export interface ResolvedImages extends Required<
  Pick<
    ImagesConfig,
    | "loader"
    | "path"
    | "deviceSizes"
    | "imageSizes"
    | "qualities"
    | "formats"
    | "minimumCacheTTL"
    | "dangerouslyAllowSVG"
    | "contentSecurityPolicy"
    | "contentDispositionType"
    | "unoptimized"
  >
> {
  loaderFile?: string;
  remotePatterns: RemotePattern[];
  localPatterns?: LocalPattern[];
}

export interface ResolvedConfig {
  /** Absolute path of the config file, or `undefined` when the project has none. */
  file?: string;
  phase: Phase;
  config: NextConfig;
  basePath: string;
  assetPrefix: string;
  trailingSlash: boolean;
  skipTrailingSlashRedirect: boolean;
  poweredByHeader: boolean;
  distDir: string;
  output?: "standalone" | "export";
  /** `env` plus `NEXT_PUBLIC_*` of the process, as `define` entries. */
  define: Record<string, string>;
  images: ResolvedImages;
  turbopack: TurbopackConfig;
  rewrites: RewriteGroups;
  redirects: Redirect[];
  headers: HeaderRule[];
  transpilePackages: string[];
  serverExternalPackages: string[];
  unsupported: Unsupported[];
}

export const DEFAULT_DEVICE_SIZES = [640, 750, 828, 1080, 1200, 1920, 2048, 3840];
export const DEFAULT_IMAGE_SIZES = [16, 32, 48, 64, 96, 128, 256, 384];
export const DEFAULT_QUALITIES = [75];
export const DEFAULT_IMAGE_PATH = "/_next/image";
/** Next's default `minimumCacheTTL` (seconds). */
export const DEFAULT_IMAGE_TTL = 14400;

/** The Next.js defaults the framework reproduces. */
export function defaultConfig(): NextConfig {
  return {
    basePath: "",
    assetPrefix: "",
    trailingSlash: false,
    skipTrailingSlashRedirect: false,
    poweredByHeader: true,
    compress: true,
    distDir: ".next",
    env: {},
    images: {
      loader: "default",
      path: DEFAULT_IMAGE_PATH,
      deviceSizes: DEFAULT_DEVICE_SIZES,
      imageSizes: DEFAULT_IMAGE_SIZES,
      qualities: DEFAULT_QUALITIES,
      formats: ["image/webp"],
      minimumCacheTTL: DEFAULT_IMAGE_TTL,
      remotePatterns: [],
      dangerouslyAllowSVG: false,
      contentSecurityPolicy: "script-src 'none'; frame-src 'none'; sandbox;",
      contentDispositionType: "attachment",
      unoptimized: false,
    },
    turbopack: {},
    transpilePackages: [],
    serverExternalPackages: [],
    reactStrictMode: null,
    pageExtensions: ["tsx", "ts", "jsx", "js"],
  };
}

/** Keys the framework does not implement, with the reason reported by `loadNextConfig`. */
const UNSUPPORTED: Record<string, string> = {
  webpack: "there is no webpack; Bun.build bundles the app (map aliases through `turbopack.resolveAlias`)",
  i18n: "Pages Router i18n routing is not implemented; use a `[locale]` segment and `proxy.ts`",
  eslint: "Next 16 removed `next lint`; run `bun lint`",
  typescript: "type checking is `bun check`, not part of the build",
  cacheComponents: "`use cache` / Cache Components are not implemented; pages are prerendered or dynamic",
  reactCompiler: "the React Compiler is not wired into Bun.build; add it as a plugin if needed",
  typedRoutes: "typed routes are not generated",
  productionBrowserSourceMaps: "source maps are always linked in production builds",
  generateEtags: "ETags are not generated",
  crossOrigin: "the `crossorigin` attribute is not added to bundle scripts",
  compress: "compression is left to the reverse proxy",
  onDemandEntries: "no equivalent in the Bun dev server",
  httpAgentOptions: "fetch uses Bun's own agent",
  outputFileTracingRoot: "no file tracing; `output: \"standalone\"` copies the build directory",
  serverRuntimeConfig: "exposed through `next/config` on the server only",
  publicRuntimeConfig: "exposed through `next/config`; prefer `NEXT_PUBLIC_*`",
  devIndicators: "no dev indicator overlay",
  allowedDevOrigins: "the dev server does not restrict origins",
  logging: "no fetch logging",
  staticPageGenerationTimeout: "prerendering has no timeout",
  excludeDefaultMomentLocales: "no webpack plugins",
  sassOptions: "add a Sass Bun plugin to `clientPlugins`",
  useFileSystemPublicRoutes: "file-system routes are always on",
  htmlLimitedBots: "no bot-specific rendering",
  expireTime: "no stale-while-revalidate headers for prerendered pages",
  bundlePagesRouterDependencies: "the Pages Router is not supported",
  cleanDistDir: "the output directory is always recreated",
};

function readExport(mod: Record<string, unknown>): unknown {
  const value = mod.default ?? mod;
  if (value && typeof value === "object" && "default" in (value as object) && Object.keys(value as object).length === 1)
    return (value as { default: unknown }).default;
  return value;
}

/** Find the config file of `root`. */
export function findConfigFile(root: string): string | undefined {
  for (const name of CONFIG_FILES) {
    const file = join(root, name);
    if (existsSync(file)) return file;
  }
  return undefined;
}

function normalizeRewrites(value: Rewrite[] | Partial<RewriteGroups> | undefined): RewriteGroups {
  if (!value) return { beforeFiles: [], afterFiles: [], fallback: [] };
  if (Array.isArray(value)) return { beforeFiles: [], afterFiles: value, fallback: [] };
  return { beforeFiles: value.beforeFiles ?? [], afterFiles: value.afterFiles ?? [], fallback: value.fallback ?? [] };
}

function normalizeImages(images: ImagesConfig | undefined, unsupported: Unsupported[]): ResolvedImages {
  const base = defaultConfig().images!;
  const remotePatterns: RemotePattern[] = [];
  for (const pattern of images?.remotePatterns ?? []) {
    if (pattern instanceof URL) {
      remotePatterns.push({
        protocol: pattern.protocol.replace(":", "") as "http" | "https",
        hostname: pattern.hostname,
        port: pattern.port || undefined,
        pathname: pattern.pathname === "/" ? undefined : pattern.pathname,
        search: pattern.search || undefined,
      });
    } else remotePatterns.push(pattern);
  }
  for (const host of images?.domains ?? []) remotePatterns.push({ protocol: "https", hostname: host });
  if (images?.domains?.length) unsupported.push({ key: "images.domains", reason: "deprecated; converted to remotePatterns (https)" });
  if (images?.disableStaticImages) unsupported.push({ key: "images.disableStaticImages", reason: "static image imports are always typed objects" });
  const loader = images?.loader ?? (images?.loaderFile ? "custom" : base.loader!);
  return {
    loader,
    loaderFile: images?.loaderFile,
    path: images?.path ?? base.path!,
    deviceSizes: images?.deviceSizes ?? base.deviceSizes!,
    imageSizes: images?.imageSizes ?? base.imageSizes!,
    qualities: images?.qualities ?? base.qualities!,
    formats: images?.formats ?? base.formats!,
    minimumCacheTTL: images?.minimumCacheTTL ?? base.minimumCacheTTL!,
    remotePatterns,
    localPatterns: images?.localPatterns,
    dangerouslyAllowSVG: images?.dangerouslyAllowSVG ?? false,
    contentSecurityPolicy: images?.contentSecurityPolicy ?? base.contentSecurityPolicy!,
    contentDispositionType: images?.contentDispositionType ?? base.contentDispositionType!,
    unoptimized: images?.unoptimized ?? false,
  };
}

/** `define` entries of `process.env.X` for `config.env` and every `NEXT_PUBLIC_*` variable. */
export function envDefines(config: NextConfig, env: Record<string, string | undefined> = process.env): Record<string, string> {
  const out: Record<string, string> = {};
  for (const [key, value] of Object.entries(env)) if (key.startsWith("NEXT_PUBLIC_") && value !== undefined) out[`process.env.${key}`] = JSON.stringify(value);
  for (const [key, value] of Object.entries(config.env ?? {})) if (value !== undefined) out[`process.env.${key}`] = JSON.stringify(value);
  return out;
}

export interface LoadConfigOptions {
  phase?: Phase;
  env?: Record<string, string | undefined>;
  /** Use this object instead of reading a file (tests, programmatic use). */
  config?: NextConfig | ((phase: Phase, ctx: { defaultConfig: NextConfig }) => NextConfig | Promise<NextConfig>);
}

/** Normalize a loaded config object. */
export async function resolveNextConfig(raw: NextConfig, phase: Phase, file?: string, env?: Record<string, string | undefined>): Promise<ResolvedConfig> {
  const config: NextConfig = { ...defaultConfig(), ...raw };
  const unsupported: Unsupported[] = [];
  for (const [key, reason] of Object.entries(UNSUPPORTED)) {
    if (raw[key] === undefined || raw[key] === null) continue;
    if (key === "compress" && raw[key] === true) continue;
    unsupported.push({ key, reason });
  }
  for (const key of Object.keys(raw.experimental ?? {})) unsupported.push({ key: `experimental.${key}`, reason: "experimental flags are ignored" });
  if (raw.output === "standalone") unsupported.push({ key: "output", reason: "`standalone` copies the whole build; use `yolo-fullstack compile` for one executable" });
  const basePath = (config.basePath ?? "").replace(/\/$/, "");
  if (basePath && !basePath.startsWith("/")) throw new Error(`@yolo-rs/fullstack: basePath must start with "/" (got ${basePath})`);
  const [rewrites, redirects, headers] = await Promise.all([
    typeof config.rewrites === "function" ? config.rewrites() : config.rewrites,
    typeof config.redirects === "function" ? config.redirects() : config.redirects,
    typeof config.headers === "function" ? config.headers() : config.headers,
  ]);
  return {
    file,
    phase,
    config,
    basePath,
    assetPrefix: (config.assetPrefix ?? "").replace(/\/$/, ""),
    trailingSlash: config.trailingSlash ?? false,
    skipTrailingSlashRedirect: config.skipTrailingSlashRedirect ?? false,
    poweredByHeader: config.poweredByHeader ?? true,
    distDir: config.distDir ?? ".next",
    output: config.output,
    define: envDefines(config, env),
    images: normalizeImages(config.images, unsupported),
    turbopack: config.turbopack ?? {},
    rewrites: normalizeRewrites(rewrites as Rewrite[] | Partial<RewriteGroups> | undefined),
    redirects: (redirects as Redirect[] | undefined) ?? [],
    headers: (headers as HeaderRule[] | undefined) ?? [],
    transpilePackages: config.transpilePackages ?? [],
    serverExternalPackages: config.serverExternalPackages ?? [],
    unsupported,
  };
}

/** Load and normalize the config of `root` (defaults when there is no config file). */
export async function loadNextConfig(root: string, options: LoadConfigOptions = {}): Promise<ResolvedConfig> {
  const phase = options.phase ?? PHASE_PRODUCTION_BUILD;
  let raw: unknown = options.config;
  let file: string | undefined;
  if (raw === undefined) {
    file = findConfigFile(root);
    if (file) {
      // The query defeats the module cache so the dev server sees edits.
      const mod = (await import(`${pathToFileURL(file).href}?t=${Date.now()}`)) as Record<string, unknown>;
      raw = readExport(mod);
    } else raw = {};
  }
  if (typeof raw === "function") raw = await (raw as (p: Phase, c: { defaultConfig: NextConfig }) => unknown)(phase, { defaultConfig: defaultConfig() });
  if (!raw || typeof raw !== "object") throw new Error(`@yolo-rs/fullstack: ${file ?? "next.config"} must export an object or a function returning one`);
  return resolveNextConfig(raw as NextConfig, phase, file, options.env);
}
