// SPDX-License-Identifier: Apache-2.0
// @yolo-rs/next/types: TypeScript twin of crates/yolo-next/src/types.rs (camelCase JSON shapes).
// Keep names aligned with the Rust model; the Rust CLI emits these JSON documents under .yolo/.

export type Severity = "info" | "warning" | "error";

export interface Diagnostic {
  readonly severity: Severity;
  readonly code: string;
  readonly message: string;
  readonly file?: string;
  readonly hint?: string;
}

export type Target = "bun" | "dioxus" | "leptos";
export type Support = "mapped" | "partial" | "unsupported";

export interface FeatureStatus {
  readonly feature: string;
  readonly support: Support;
  readonly note: string;
  readonly target?: Target;
}

export interface EmittedFile {
  readonly path: string;
  readonly contents: string;
}

// ---------------------------------------------------------------------------
// next.config.* (Next.js 16.4)
// ---------------------------------------------------------------------------

export interface TurbopackLoader {
  readonly loader: string;
  readonly options?: Record<string, unknown>;
}

export interface TurbopackRule {
  readonly loaders?: ReadonlyArray<string | TurbopackLoader>;
  readonly as?: string;
  readonly type?:
    | "asset"
    | "ecmascript"
    | "typescript"
    | "css"
    | "css-module"
    | "json"
    | "wasm"
    | "node"
    | "raw"
    | "text"
    | "bytes";
  readonly condition?: unknown;
}

export interface TurbopackConfig {
  readonly root?: string;
  readonly rules?: Record<string, TurbopackRule | TurbopackRule[]>;
  readonly resolveAlias?: Record<
    string,
    false | string | string[] | Record<string, string | false>
  >;
  readonly resolveExtensions?: string[];
  readonly debugIds?: boolean;
  readonly ignoreIssue?: unknown[];
  readonly chunkLoadingGlobal?: string;
}

export interface RemotePattern {
  readonly protocol?: "http" | "https";
  readonly hostname: string;
  readonly port?: string;
  readonly pathname?: string;
  readonly search?: string;
}

export interface LocalPattern {
  readonly pathname?: string;
  readonly search?: string;
}

export interface ImagesConfig {
  readonly remotePatterns?: ReadonlyArray<RemotePattern | URL>;
  readonly localPatterns?: ReadonlyArray<LocalPattern>;
  readonly loader?: "default" | "imgix" | "cloudinary" | "akamai" | "custom";
  readonly loaderFile?: string;
  readonly path?: string;
  readonly formats?: ReadonlyArray<"image/avif" | "image/webp">;
  readonly deviceSizes?: ReadonlyArray<number>;
  readonly imageSizes?: ReadonlyArray<number>;
  readonly qualities?: ReadonlyArray<number>;
  readonly minimumCacheTTL?: number;
  readonly dangerouslyAllowSVG?: boolean;
  readonly dangerouslyAllowLocalIP?: boolean;
  readonly contentSecurityPolicy?: string;
  readonly contentDispositionType?: "attachment" | "inline";
  readonly unoptimized?: boolean;
  readonly maximumRedirects?: number;
  readonly maximumResponseBody?: number;
  readonly domains?: ReadonlyArray<string>;
}

export const IMAGES_DEFAULTS = {
  loader: "default",
  loaderFile: "",
  path: "/_next/image",
  formats: ["image/webp"],
  deviceSizes: [640, 750, 828, 1080, 1200, 1920, 2048, 3840],
  imageSizes: [32, 48, 64, 96, 128, 256, 384],
  qualities: [75],
  minimumCacheTTL: 14_400,
  dangerouslyAllowSVG: false,
  dangerouslyAllowLocalIP: false,
  contentSecurityPolicy: "script-src 'none'; frame-src 'none'; sandbox;",
  contentDispositionType: "attachment",
  unoptimized: false,
  maximumRedirects: 3,
  maximumResponseBody: 50_000_000,
} as const;

export interface RouteCondition {
  readonly type: "header" | "cookie" | "host" | "query";
  readonly key?: string;
  readonly value?: string;
}

export interface RouteRule {
  readonly source: string;
  readonly destination?: string;
  readonly permanent?: boolean;
  readonly statusCode?: 301 | 302 | 303 | 307 | 308;
  readonly basePath?: false;
  readonly locale?: false;
  readonly has?: ReadonlyArray<RouteCondition>;
  readonly missing?: ReadonlyArray<RouteCondition>;
}

export interface Rewrites {
  readonly beforeFiles?: ReadonlyArray<RouteRule>;
  readonly afterFiles?: ReadonlyArray<RouteRule>;
  readonly fallback?: ReadonlyArray<RouteRule>;
}

export interface HeaderKv {
  readonly key: string;
  readonly value: string;
}

export interface HeaderRule {
  readonly source: string;
  readonly headers: ReadonlyArray<HeaderKv>;
  readonly basePath?: false;
  readonly locale?: false;
  readonly has?: ReadonlyArray<RouteCondition>;
  readonly missing?: ReadonlyArray<RouteCondition>;
}

export interface CacheLifeProfile {
  readonly stale?: number;
  readonly revalidate?: number;
  readonly expire?: number;
}

/** Built-in `cacheLife` profiles of Next.js 16 (seconds). */
export const BUILTIN_CACHE_LIFE: Readonly<Record<string, Required<CacheLifeProfile>>> = {
  default: { stale: 300, revalidate: 900, expire: 0xfffffffe },
  seconds: { stale: 30, revalidate: 1, expire: 60 },
  minutes: { stale: 300, revalidate: 60, expire: 3600 },
  hours: { stale: 300, revalidate: 3600, expire: 86_400 },
  days: { stale: 300, revalidate: 86_400, expire: 604_800 },
  weeks: { stale: 300, revalidate: 604_800, expire: 2_592_000 },
  max: { stale: 300, revalidate: 2_592_000, expire: 31_536_000 },
};

export interface I18nConfig {
  readonly locales: ReadonlyArray<string>;
  readonly defaultLocale: string;
  readonly domains?: ReadonlyArray<unknown>;
  readonly localeDetection?: false;
}

export interface CompilerConfig {
  readonly removeConsole?: boolean | { exclude?: string[] };
  readonly reactRemoveProperties?: boolean | { properties: string[] };
  readonly styledComponents?: unknown;
  readonly emotion?: unknown;
  readonly relay?: unknown;
  readonly define?: Record<string, string>;
  readonly defineServer?: Record<string, string>;
}

/**
 * Subset of the Next.js 16.4 `NextConfig` type accepted verbatim by `defineYoloConfig`.
 * Function-valued keys (`redirects`, `rewrites`, `headers`) are awaited at startup.
 */
export interface NextCompatConfig {
  readonly reactStrictMode?: boolean | null;
  readonly output?: "standalone" | "export";
  readonly distDir?: string;
  readonly basePath?: string;
  readonly assetPrefix?: string;
  readonly trailingSlash?: boolean;
  readonly skipTrailingSlashRedirect?: boolean;
  readonly skipProxyUrlNormalize?: boolean;
  readonly skipMiddlewareUrlNormalize?: boolean;
  readonly compress?: boolean;
  readonly poweredByHeader?: boolean;
  readonly generateEtags?: boolean;
  readonly reactCompiler?: boolean | { compilationMode?: "infer" | "annotation" | "all"; panicThreshold?: string };
  readonly reactProductionProfiling?: boolean;
  readonly reactMaxHeadersLength?: number;
  readonly cacheComponents?: boolean;
  readonly partialPrefetching?: boolean;
  readonly typedRoutes?: boolean;
  readonly transpilePackages?: ReadonlyArray<string>;
  readonly serverExternalPackages?: ReadonlyArray<string>;
  readonly bundlePagesRouterDependencies?: boolean;
  readonly pageExtensions?: ReadonlyArray<string>;
  readonly env?: Record<string, string | undefined>;
  readonly deploymentId?: string;
  readonly supportsImmutableAssets?: boolean;
  readonly expireTime?: number;
  readonly crossOrigin?: "anonymous" | "use-credentials";
  readonly productionBrowserSourceMaps?: boolean;
  readonly allowedDevOrigins?: ReadonlyArray<string>;
  readonly staticPageGenerationTimeout?: number;
  readonly turbopack?: TurbopackConfig;
  readonly images?: ImagesConfig;
  readonly redirects?: () => Promise<ReadonlyArray<RouteRule>> | ReadonlyArray<RouteRule>;
  readonly rewrites?: () =>
    | Promise<ReadonlyArray<RouteRule> | Rewrites>
    | ReadonlyArray<RouteRule>
    | Rewrites;
  readonly headers?: () => Promise<ReadonlyArray<HeaderRule>> | ReadonlyArray<HeaderRule>;
  readonly compiler?: CompilerConfig;
  readonly typescript?: { ignoreBuildErrors?: boolean; tsconfigPath?: string };
  readonly experimental?: Record<string, unknown>;
  readonly cacheLife?: Record<string, CacheLifeProfile>;
  readonly cacheHandlers?: Record<string, string>;
  readonly cacheHandler?: string;
  readonly cacheMaxMemorySize?: number;
  readonly i18n?: I18nConfig | null;
  readonly logging?: unknown;
  readonly devIndicators?: unknown;
  readonly outputFileTracingRoot?: string;
  readonly outputFileTracingIncludes?: Record<string, string[]>;
  readonly outputFileTracingExcludes?: Record<string, string[]>;
  readonly modularizeImports?: Record<string, unknown>;
  readonly sassOptions?: unknown;
  readonly htmlLimitedBots?: RegExp;
  readonly adapterPath?: string;
  readonly generateBuildId?: () => string | null | Promise<string | null>;
  readonly webpack?: unknown;
  readonly exportPathMap?: unknown;
  // Removed in Next 16, accepted for migration diagnostics only.
  readonly eslint?: unknown;
  readonly amp?: unknown;
  readonly serverRuntimeConfig?: Record<string, unknown>;
  readonly publicRuntimeConfig?: Record<string, unknown>;
}

// ---------------------------------------------------------------------------
// Bundler plan (Turbopack -> Bun)
// ---------------------------------------------------------------------------

export type BunLoader =
  | "js"
  | "jsx"
  | "ts"
  | "tsx"
  | "json"
  | "jsonc"
  | "toml"
  | "yaml"
  | "text"
  | "file"
  | "napi"
  | "wasm"
  | "css"
  | "html";

export type BunPluginKind =
  | "alias-resolve"
  | "loader-override"
  | "svg-react"
  | "markdown"
  | "text"
  | "unsupported";

export interface BunPluginSpec {
  readonly name: string;
  readonly kind: BunPluginKind;
  readonly filter: string;
  readonly originLoader?: string;
  readonly loader?: BunLoader;
  readonly options?: Record<string, unknown>;
}

export interface BunBundlerPlan {
  readonly plugins: ReadonlyArray<BunPluginSpec>;
  readonly loaders: Record<string, BunLoader>;
  readonly aliases: Record<string, string>;
  readonly browserAliases: Record<string, string>;
  readonly emptyModules: ReadonlyArray<string>;
  readonly extensions: ReadonlyArray<string>;
  readonly root?: string;
  readonly define: Record<string, string>;
  readonly external: ReadonlyArray<string>;
  readonly unsupported: ReadonlyArray<FeatureStatus>;
}

// ---------------------------------------------------------------------------
// Routes
// ---------------------------------------------------------------------------

export type SegmentKind =
  | "root"
  | "static"
  | "dynamic"
  | "catch-all"
  | "optional-catch-all"
  | "group"
  | "parallel-slot"
  | "intercept"
  | "private";

export type MetadataFileKind =
  | "favicon"
  | "icon"
  | "apple-icon"
  | "opengraph-image"
  | "twitter-image"
  | "sitemap"
  | "robots"
  | "manifest";

export interface MetadataFile {
  readonly kind: MetadataFileKind;
  readonly path: string;
  readonly generated: boolean;
}

export interface SegmentFiles {
  readonly page?: string;
  readonly layout?: string;
  readonly template?: string;
  readonly loading?: string;
  readonly error?: string;
  readonly globalError?: string;
  readonly notFound?: string;
  readonly forbidden?: string;
  readonly unauthorized?: string;
  readonly default?: string;
  readonly route?: string;
  readonly metadata: ReadonlyArray<MetadataFile>;
}

export interface RouteSegment {
  readonly name: string;
  readonly kind: SegmentKind;
  readonly param?: string;
  readonly interceptLevel?: number;
  readonly interceptTarget?: string;
  readonly files: SegmentFiles;
  readonly children: ReadonlyArray<RouteSegment>;
}

export interface RouteParam {
  readonly name: string;
  readonly catchAll: boolean;
  readonly optional: boolean;
}

export type RouteKind = "page" | "route-handler" | "metadata";
export type ModuleDirective = "none" | "use-client" | "use-server" | "use-cache";

export interface SegmentConfig {
  readonly dynamic?: "auto" | "force-dynamic" | "error" | "force-static";
  readonly dynamicParams?: boolean;
  readonly revalidate?: false | number;
  readonly fetchCache?: string;
  readonly runtime?: "nodejs" | "edge";
  readonly preferredRegion?: unknown;
  readonly maxDuration?: number;
  readonly ensureStatic?: unknown;
  readonly prefetch?: "auto" | "partial" | "force-disabled";
  readonly instant?: boolean;
  readonly experimentalPpr?: boolean;
  readonly hasGenerateStaticParams: boolean;
  readonly hasGenerateMetadata: boolean;
  readonly hasGenerateViewport: boolean;
  readonly hasGenerateImageMetadata: boolean;
  readonly hasGenerateSitemaps: boolean;
  readonly staticMetadata?: Metadata;
  readonly staticViewport?: Viewport;
  readonly directive: ModuleDirective;
  readonly defaultExport?: string;
  readonly namedExports: ReadonlyArray<string>;
  readonly defaultExportAsync: boolean;
  readonly hooksUsed: ReadonlyArray<string>;
  readonly nextImports: ReadonlyArray<string>;
  readonly diagnostics: ReadonlyArray<Diagnostic>;
}

export interface InterceptSpec {
  readonly level: number;
  readonly target: string;
}

export interface RouteEntry {
  readonly appPath: string;
  readonly urlPattern: string;
  readonly bunPattern: string;
  readonly tanstackPath: string;
  readonly dioxusRoute: string;
  readonly dioxusVariant: string;
  readonly leptosPath: string;
  readonly params: ReadonlyArray<RouteParam>;
  readonly kind: RouteKind;
  readonly file: string;
  readonly layouts: ReadonlyArray<string>;
  readonly templates: ReadonlyArray<string>;
  readonly loading?: string;
  readonly error?: string;
  readonly notFound?: string;
  readonly parallelSlots: ReadonlyArray<string>;
  readonly slot?: string;
  readonly intercepting?: InterceptSpec;
  readonly segmentConfig: SegmentConfig;
  readonly methods: ReadonlyArray<string>;
  readonly metadataKind?: MetadataFileKind;
  readonly legacyPages: boolean;
  readonly legacyDataFunctions: ReadonlyArray<string>;
}

export interface RouteManifest {
  readonly root: string;
  readonly appDir?: string;
  readonly pagesDir?: string;
  readonly tree?: RouteSegment;
  readonly entries: ReadonlyArray<RouteEntry>;
  readonly rootLayout?: string;
  readonly rootNotFound?: string;
  readonly globalError?: string;
  readonly proxyFile?: string;
  readonly instrumentation?: string;
  readonly instrumentationClient?: string;
  readonly mdxComponents?: string;
  readonly staticMetadata: ReadonlyArray<MetadataFile>;
  readonly diagnostics: ReadonlyArray<Diagnostic>;
}

export interface RouteMatch {
  readonly entry: RouteEntry;
  readonly params: Record<string, string>;
}

// ---------------------------------------------------------------------------
// Metadata API
// ---------------------------------------------------------------------------

export interface TitleSpec {
  readonly default?: string;
  readonly template?: string;
  readonly absolute?: string;
}

export interface Metadata {
  readonly title?: string | TitleSpec;
  readonly description?: string;
  readonly metadataBase?: string | URL;
  readonly applicationName?: string;
  readonly authors?: ReadonlyArray<{ name?: string; url?: string }>;
  readonly generator?: string;
  readonly keywords?: string | ReadonlyArray<string>;
  readonly referrer?: string;
  readonly creator?: string;
  readonly publisher?: string;
  readonly robots?: string | Record<string, unknown>;
  readonly alternates?: Record<string, unknown>;
  readonly icons?: string | Record<string, unknown> | ReadonlyArray<unknown>;
  readonly manifest?: string;
  readonly openGraph?: Record<string, unknown>;
  readonly twitter?: Record<string, unknown>;
  readonly verification?: Record<string, unknown>;
  readonly appleWebApp?: Record<string, unknown>;
  readonly formatDetection?: Record<string, unknown>;
  readonly category?: string;
  readonly themeColor?: unknown;
  readonly viewport?: unknown;
  readonly colorScheme?: string;
  readonly other?: Record<string, string | number | ReadonlyArray<string | number>>;
}

export interface ThemeColor {
  readonly media?: string;
  readonly color: string;
}

export interface Viewport {
  readonly width?: string | number;
  readonly height?: string | number;
  readonly initialScale?: number;
  readonly minimumScale?: number;
  readonly maximumScale?: number;
  readonly userScalable?: boolean;
  readonly viewportFit?: "auto" | "cover" | "contain";
  readonly interactiveWidget?: "resizes-visual" | "resizes-content" | "overlays-content";
  readonly themeColor?: string | ReadonlyArray<ThemeColor>;
  readonly colorScheme?: string;
}

// ---------------------------------------------------------------------------
// next/font
// ---------------------------------------------------------------------------

export type FontDisplay = "auto" | "block" | "swap" | "fallback" | "optional";

export interface GoogleFontOptions {
  readonly weight?: string | ReadonlyArray<string>;
  readonly style?: string | ReadonlyArray<string>;
  readonly subsets?: ReadonlyArray<string>;
  readonly axes?: ReadonlyArray<string>;
  readonly display?: FontDisplay;
  readonly preload?: boolean;
  readonly fallback?: ReadonlyArray<string>;
  readonly adjustFontFallback?: boolean;
  readonly variable?: `--${string}`;
}

export interface LocalFontSrc {
  readonly path: string;
  readonly weight?: string;
  readonly style?: string;
}

export interface LocalFontOptions {
  readonly src: string | ReadonlyArray<LocalFontSrc>;
  readonly display?: FontDisplay;
  readonly weight?: string;
  readonly style?: string;
  readonly fallback?: ReadonlyArray<string>;
  readonly preload?: boolean;
  readonly variable?: `--${string}`;
  readonly adjustFontFallback?: "Arial" | "Times New Roman" | false;
  readonly declarations?: ReadonlyArray<{ prop: string; value: string }>;
}

/** Object returned by `next/font` factories. */
export interface NextFont {
  readonly className: string;
  readonly style: { fontFamily: string; fontWeight?: number; fontStyle?: string };
}

export interface NextFontWithVariable extends NextFont {
  readonly variable: string;
}

export interface FontFaceAsset {
  readonly source: string;
  readonly localPath: string;
  readonly family: string;
  readonly weight: string;
  readonly style: string;
  readonly unicodeRange?: string;
  readonly subset?: string;
  readonly format: string;
  readonly preload: boolean;
}

export interface FontMetrics {
  readonly family: string;
  readonly unitsPerEm: number;
  readonly ascent: number;
  readonly descent: number;
  readonly lineGap: number;
  readonly capHeight?: number;
  readonly xAvgCharWidth: number;
  readonly azAvgWidth?: number;
}

export interface FallbackOverrides {
  readonly fallbackFamily: string;
  readonly ascentOverride: string;
  readonly descentOverride: string;
  readonly lineGapOverride: string;
  readonly sizeAdjust: string;
}

/** Arial (sans-serif fallback) as used by next/font. */
export const ARIAL_METRICS: FontMetrics = {
  family: "Arial",
  unitsPerEm: 2048,
  ascent: 1854,
  descent: -434,
  lineGap: 67,
  capHeight: 1467,
  xAvgCharWidth: 904,
  azAvgWidth: 934.5116279069767,
};

/** Times New Roman (serif fallback) as used by next/font. */
export const TIMES_NEW_ROMAN_METRICS: FontMetrics = {
  family: "Times New Roman",
  unitsPerEm: 2048,
  ascent: 1825,
  descent: -443,
  lineGap: 87,
  capHeight: 1341,
  xAvgCharWidth: 821,
  azAvgWidth: 854.3953488372093,
};

// ---------------------------------------------------------------------------
// next/image + CDN
// ---------------------------------------------------------------------------

export type ImageLoaderKind =
  | "default"
  | "custom"
  | "cloudflare"
  | "cloudinary"
  | "imgix"
  | "akamai"
  | "fastly"
  | "supabase"
  | "cloudfront"
  | "unoptimized";

export interface ImageLoaderProps {
  readonly src: string;
  readonly width: number;
  readonly quality?: number;
}

export type ImageLoader = (props: ImageLoaderProps) => string;

export interface SrcSetCandidate {
  readonly url: string;
  readonly descriptor: string;
}

export interface ImageOptimizerPolicy {
  readonly route: string;
  readonly compatRoutes: ReadonlyArray<string>;
  readonly loader: ImageLoaderKind;
  readonly loaderFile?: string;
  readonly formats: ReadonlyArray<string>;
  readonly deviceSizes: ReadonlyArray<number>;
  readonly imageSizes: ReadonlyArray<number>;
  readonly qualities: ReadonlyArray<number>;
  readonly minimumCacheTtl: number;
  readonly cacheControl: string;
  readonly allowSvg: boolean;
  readonly allowLocalIp: boolean;
  readonly contentSecurityPolicy: string;
  readonly contentDispositionType: string;
  readonly remotePatterns: ReadonlyArray<RemotePattern>;
  readonly localPatterns?: ReadonlyArray<LocalPattern>;
  readonly maximumRedirects: number;
  readonly maximumResponseBody: number;
  readonly unoptimized: boolean;
}

export type OutputMode = "server" | "standalone" | "export";

export interface CdnUploadTarget {
  readonly provider: "s3" | "r2" | "gcs" | "minio";
  readonly bucket: string;
  readonly prefix: string;
  readonly endpoint?: string;
  readonly region?: string;
  readonly publicBaseUrl: string;
}

export interface CdnPlan {
  readonly assetPrefix: string;
  readonly basePath: string;
  readonly staticRoute: string;
  readonly compatStaticRoutes: ReadonlyArray<string>;
  readonly fontsRoute: string;
  readonly publicDir: string;
  readonly immutableCacheControl: string;
  readonly publicCacheControl: string;
  readonly dynamicHtmlCacheControl: string;
  readonly deploymentId?: string;
  readonly skewQuery: string;
  readonly skewHeader: string;
  readonly output: OutputMode;
  readonly expireTime: number;
  readonly compress: boolean;
  readonly poweredByHeader: boolean;
  readonly generateEtags: boolean;
  readonly trailingSlash: boolean;
  readonly upload?: CdnUploadTarget;
}

export const CDN_DEFAULTS: CdnPlan = {
  assetPrefix: "",
  basePath: "",
  staticRoute: "/_yolo/static",
  compatStaticRoutes: ["/_next/static"],
  fontsRoute: "/_yolo/fonts",
  publicDir: "public",
  immutableCacheControl: "public, max-age=31536000, immutable",
  publicCacheControl: "public, max-age=0",
  dynamicHtmlCacheControl: "private, no-cache, no-store, max-age=0, must-revalidate",
  skewQuery: "dpl",
  skewHeader: "x-deployment-id",
  output: "server",
  expireTime: 31_536_000,
  compress: true,
  poweredByHeader: true,
  generateEtags: true,
  trailingSlash: false,
};

export interface CdnAssetManifestEntry {
  readonly path: string;
  readonly publicUrl: string;
  readonly contentType: string;
  readonly cacheControl: string;
  readonly hashed: boolean;
  readonly size: number;
}

// ---------------------------------------------------------------------------
// RSC module graph
// ---------------------------------------------------------------------------

export type ModuleKind = "server" | "client" | "server-action" | "cache" | "shared";

export interface CacheFunction {
  readonly name: string;
  readonly life?: string;
  readonly tags: ReadonlyArray<string>;
  readonly variant: string;
}

export interface ModuleInfo {
  readonly id: string;
  readonly kind: ModuleKind;
  readonly directive: ModuleDirective;
  readonly exports: ReadonlyArray<string>;
  readonly imports: ReadonlyArray<string>;
  readonly inlineServerActions: ReadonlyArray<string>;
  readonly cacheFunctions: ReadonlyArray<CacheFunction>;
  readonly nextImports: ReadonlyArray<string>;
  readonly hooks: ReadonlyArray<string>;
  readonly diagnostics: ReadonlyArray<Diagnostic>;
}

export interface ClientReference {
  readonly id: string;
  readonly module: string;
  readonly exportName: string;
  readonly chunks: ReadonlyArray<string>;
  readonly async: boolean;
}

export interface ServerReference {
  readonly id: string;
  readonly module: string;
  readonly exportName: string;
  readonly inline: boolean;
  readonly cache: boolean;
}

export interface ModuleGraph {
  readonly root: string;
  readonly modules: ReadonlyArray<ModuleInfo>;
  readonly clientManifest: ReadonlyArray<ClientReference>;
  readonly serverManifest: ReadonlyArray<ServerReference>;
  readonly diagnostics: ReadonlyArray<Diagnostic>;
}

// ---------------------------------------------------------------------------
// Proxy
// ---------------------------------------------------------------------------

export interface ProxySpec {
  readonly file: string;
  readonly legacyMiddlewareName: boolean;
  readonly exportName: string;
  readonly matchers: ReadonlyArray<RouteRule>;
  readonly runtimeExport?: string;
  readonly usesRewrite: boolean;
  readonly usesRedirect: boolean;
  readonly usesNextHeaders: boolean;
  readonly usesCookies: boolean;
  readonly usesWaitUntil: boolean;
  readonly usesRemovedGeoIp: boolean;
  readonly diagnostics: ReadonlyArray<Diagnostic>;
}

// ---------------------------------------------------------------------------
// Yolo normalized config
// ---------------------------------------------------------------------------

export interface YoloServerConfig {
  readonly port: number;
  readonly hostname: string;
  readonly hmr: boolean;
  readonly browserConsole: boolean;
  readonly compress: boolean;
  readonly poweredByHeader: boolean;
  readonly generateEtags: boolean;
  readonly actionBodySizeLimit: number;
  readonly actionAllowedOrigins: ReadonlyArray<string>;
}

export interface YoloReactConfig {
  readonly strictMode: boolean;
  readonly compiler: boolean;
  readonly fastRefresh: boolean;
  readonly cacheComponents: boolean;
  readonly typedRoutes: boolean;
}

export type YoloPresetId =
  | "yolo-neutral"
  | "yolo-m3"
  | "yolo-fluent"
  | "yolo-apple"
  | "yolo-omarchy"
  | "yolo-hyprland"
  | "yolo-aphrody"
  | "yolo-chromium";

export interface YoloThemeConfig {
  readonly preset: YoloPresetId;
  readonly dark: boolean;
  readonly seedHex?: string;
}

export interface YoloWebConfig {
  readonly appDir: string;
  readonly pagesDir?: string;
  readonly distDir: string;
  readonly output: OutputMode;
  readonly server: YoloServerConfig;
  readonly react: YoloReactConfig;
  readonly theme: YoloThemeConfig;
  readonly cdn: CdnPlan;
  readonly images?: ImageOptimizerPolicy;
  readonly bundler: BunBundlerPlan;
  readonly redirects: ReadonlyArray<RouteRule>;
  readonly rewrites: Rewrites;
  readonly headers: ReadonlyArray<HeaderRule>;
  readonly publicEnvPrefixes: ReadonlyArray<string>;
  readonly env: Record<string, string>;
  readonly cacheLife: Record<string, CacheLifeProfile>;
  readonly i18n?: I18nConfig;
  readonly transpilePackages: ReadonlyArray<string>;
  readonly serverExternalPackages: ReadonlyArray<string>;
  readonly pageExtensions: ReadonlyArray<string>;
  readonly trailingSlash: boolean;
  readonly proxyFile?: string;
}

export interface MigrationReport {
  readonly project: string;
  readonly generatedAt: string;
  readonly targets: ReadonlyArray<Target>;
  readonly features: ReadonlyArray<FeatureStatus>;
  readonly parityScore: number;
  readonly diagnostics: ReadonlyArray<Diagnostic>;
  readonly filesEmitted: ReadonlyArray<string>;
  readonly counts: Record<string, number>;
}

/** Request headers of the Server Actions / Flight protocol (Next.js compatible). */
export const RSC_HEADERS = {
  rsc: "rsc",
  action: "next-action",
  routerStateTree: "next-router-state-tree",
  routerPrefetch: "next-router-prefetch",
  url: "next-url",
  deploymentId: "x-deployment-id",
  actionRedirect: "x-action-redirect",
  actionRevalidated: "x-action-revalidated",
  contentType: "text/x-component",
  staleTime: "x-nextjs-stale-time",
} as const;

/** Form-encoded action field prefixes (`$ACTION_ID_<id>`, `$ACTION_REF_<n>`, `$ACTION_<n>:<m>`). */
export const ACTION_FIELD = {
  id: "$ACTION_ID_",
  ref: "$ACTION_REF_",
  payload: "$ACTION_",
} as const;

// ---------------------------------------------------------------------------
// Resolved Yolo config (output of defineYoloConfig / resolveYoloConfig)
// ---------------------------------------------------------------------------

/** Normalized Yolo config plus the original Next.js config it was derived from and the migration diagnostics. */
export type ResolvedYoloConfig = YoloWebConfig & {
  readonly next: NextCompatConfig;
  readonly diagnostics: Diagnostic[];
};
