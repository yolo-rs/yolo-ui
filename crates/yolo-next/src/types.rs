// SPDX-License-Identifier: Apache-2.0
//! Shared data model of the `yolo-next` migration compiler.
//!
//! Every module (`config`, `turbopack`, `routes`, `segment`, `metadata`, `fonts`, `images`,
//! `rsc`, `rules`, `proxy`, `cdn`, `report`, `emit_*`) consumes and produces these types.
//! The TypeScript twin lives in `packages/yolo-next/src/types.ts`; keep names aligned.
//!
//! Facts encoded here were verified against Next.js `v16.4.5` (2026-11-03) sources:
//! `packages/next/src/server/config-shared.ts`, `shared/lib/image-config.ts`,
//! `packages/font/src/**`, `crates/next-custom-transforms/src/transforms/server_actions.rs`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

// ============================================================================
// Diagnostics, targets, emitted files
// ============================================================================

/// Severity of a migration diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Severity {
    /// Informational note (feature mapped with a remark).
    Info,
    /// Feature mapped partially or with behavioural differences.
    Warning,
    /// Feature cannot be migrated automatically.
    Error,
}

/// One migration diagnostic (machine readable code + human message).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    /// Severity.
    pub severity: Severity,
    /// Stable kebab-case code, e.g. `next-config-webpack-unsupported`.
    pub code: String,
    /// Human readable message.
    pub message: String,
    /// Relative file path the diagnostic refers to, when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    /// Suggested remediation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
}

impl Diagnostic {
    /// Creates a diagnostic with the given severity.
    pub fn new(severity: Severity, code: impl Into<String>, message: impl Into<String>) -> Self {
        Self { severity, code: code.into(), message: message.into(), file: None, hint: None }
    }

    /// Informational diagnostic.
    pub fn info(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(Severity::Info, code, message)
    }

    /// Warning diagnostic.
    pub fn warning(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(Severity::Warning, code, message)
    }

    /// Error diagnostic.
    pub fn error(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(Severity::Error, code, message)
    }

    /// Attaches a file path.
    #[must_use]
    pub fn with_file(mut self, file: impl Into<String>) -> Self {
        self.file = Some(file.into());
        self
    }

    /// Attaches a remediation hint.
    #[must_use]
    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }
}

/// Output framework lane.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Target {
    /// Bun 1.4.4 `Bun.serve` + TanStack Router/Query on React 19 (`@yolo-rs/next`).
    Bun,
    /// Dioxus 0.8 fullstack (`#[derive(Routable)]`, `#[server]`, `asset!`).
    Dioxus,
    /// Leptos 0.8 (`leptos_router`, `#[server]`, `leptos_meta`).
    Leptos,
}

impl Target {
    /// All lanes in canonical order.
    pub const ALL: [Self; 3] = [Self::Bun, Self::Dioxus, Self::Leptos];

    /// Stable identifier (`bun`, `dioxus`, `leptos`).
    pub const fn id(self) -> &'static str {
        match self {
            Self::Bun => "bun",
            Self::Dioxus => "dioxus",
            Self::Leptos => "leptos",
        }
    }

    /// Parses an identifier (case-insensitive, accepts `tanstack` for Bun).
    pub fn from_id(id: &str) -> Option<Self> {
        match id.trim().to_ascii_lowercase().as_str() {
            "bun" | "tanstack" | "react" | "ts" => Some(Self::Bun),
            "dioxus" | "dx" | "rsx" => Some(Self::Dioxus),
            "leptos" => Some(Self::Leptos),
            _ => None,
        }
    }
}

/// Migration support level for one Next.js feature.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Support {
    /// Fully mapped with equivalent semantics.
    Mapped,
    /// Mapped with caveats listed in the note.
    Partial,
    /// Not migrated automatically.
    Unsupported,
}

/// Feature parity entry of the migration report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeatureStatus {
    /// Feature name, e.g. `next/font/google`, `turbopack.rules`, `app/[slug]/page.tsx`.
    pub feature: String,
    /// Support level.
    pub support: Support,
    /// Explanation of the mapping.
    pub note: String,
    /// Target the status applies to (`None` = all targets).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<Target>,
}

/// A file produced by an emitter (path relative to the output directory).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmittedFile {
    /// Relative path using `/` separators.
    pub path: String,
    /// UTF-8 contents.
    pub contents: String,
}

// ============================================================================
// next.config.* (Next.js 16.4)
// ============================================================================

/// Turbopack `rules` loader entry (`string | { loader, options }`).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TurbopackLoader {
    /// Loader package name, e.g. `@svgr/webpack`, `raw-loader`.
    pub loader: String,
    /// JSON-only loader options.
    #[serde(default)]
    pub options: Map<String, Value>,
}

/// One Turbopack rule (`turbopack.rules[glob]`).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TurbopackRule {
    /// Loaders applied in order.
    #[serde(default)]
    pub loaders: Vec<TurbopackLoader>,
    /// Output extension the loader result is treated as (`*.js`).
    #[serde(default, rename = "as", skip_serializing_if = "Option::is_none")]
    pub as_: Option<String>,
    /// Module type override (`asset`, `ecmascript`, `css`, `json`, `raw`, `text`, `bytes`, ...).
    #[serde(default, rename = "type", skip_serializing_if = "Option::is_none")]
    pub type_: Option<String>,
    /// Condition (`browser`, `foreign`, `development`, `{ all: [] }`, `{ path }`...), kept raw.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub condition: Option<Value>,
}

/// `turbopack` options (`TurbopackOptions` in `config-shared.ts`).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TurbopackConfig {
    /// Absolute project root override.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub root: Option<String>,
    /// Glob -> rule(s). A glob with no `/` matches the basename.
    #[serde(default)]
    pub rules: BTreeMap<String, Vec<TurbopackRule>>,
    /// Alias -> `false | string | string[] | { browser?, [condition]: ... }` kept raw.
    #[serde(default)]
    pub resolve_alias: BTreeMap<String, Value>,
    /// Replaces (does not extend) the default extension list.
    #[serde(default)]
    pub resolve_extensions: Vec<String>,
    /// TC39 debug ids.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub debug_ids: Option<bool>,
    /// Issues to ignore (`{ path, title?, description? }`), kept raw.
    #[serde(default)]
    pub ignore_issue: Vec<Value>,
    /// Chunk loading global name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chunk_loading_global: Option<String>,
}

/// `images.remotePatterns[]` entry.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemotePattern {
    /// `http` or `https`; `None` matches both.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub protocol: Option<String>,
    /// Hostname with `*` (one label) and `**` (any number of labels) wildcards.
    pub hostname: String,
    /// Port; `None` matches any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port: Option<String>,
    /// Pathname glob (`/account123/**`); `None` matches any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pathname: Option<String>,
    /// Exact query string (`?v=1`); `None` matches any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub search: Option<String>,
}

/// `images.localPatterns[]` entry.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalPattern {
    /// Pathname glob.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pathname: Option<String>,
    /// Exact query string.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub search: Option<String>,
}

/// `images` config (`imageConfigDefault` in `image-config.ts`, Next 16 defaults).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ImagesConfig {
    /// Allowed remote sources.
    pub remote_patterns: Vec<RemotePattern>,
    /// Allowed local sources (`None` = allow all).
    pub local_patterns: Option<Vec<LocalPattern>>,
    /// `default | imgix | cloudinary | akamai | custom`.
    pub loader: String,
    /// Path of a `'use client'` module default-exporting the loader.
    pub loader_file: String,
    /// Optimizer route (`/_next/image`).
    pub path: String,
    /// Preferred formats in order (`image/avif`, `image/webp`).
    pub formats: Vec<String>,
    /// Breakpoints for `sizes`/`fill` srcsets.
    pub device_sizes: Vec<u32>,
    /// Fixed widths merged with `device_sizes`.
    pub image_sizes: Vec<u32>,
    /// Allowed `q` values; the `quality` prop is coerced to the nearest.
    pub qualities: Vec<u8>,
    /// Minimum `max-age` of optimized images, seconds.
    pub minimum_cache_ttl: u64,
    /// Allow `.svg` through the optimizer.
    pub dangerously_allow_svg: bool,
    /// Allow private IP upstreams.
    pub dangerously_allow_local_ip: bool,
    /// CSP header of optimizer responses.
    pub content_security_policy: String,
    /// `attachment | inline`.
    pub content_disposition_type: String,
    /// Disable the optimizer globally.
    pub unoptimized: bool,
    /// Upstream redirect limit.
    pub maximum_redirects: u32,
    /// Upstream body limit, bytes.
    pub maximum_response_body: u64,
    /// Deprecated hostname allow-list.
    pub domains: Vec<String>,
}

impl Default for ImagesConfig {
    fn default() -> Self {
        Self {
            remote_patterns: Vec::new(),
            local_patterns: None,
            loader: String::from("default"),
            loader_file: String::new(),
            path: String::from("/_next/image"),
            formats: vec![String::from("image/webp")],
            device_sizes: vec![640, 750, 828, 1080, 1200, 1920, 2048, 3840],
            image_sizes: vec![32, 48, 64, 96, 128, 256, 384],
            qualities: vec![75],
            minimum_cache_ttl: 14_400,
            dangerously_allow_svg: false,
            dangerously_allow_local_ip: false,
            content_security_policy: String::from("script-src 'none'; frame-src 'none'; sandbox;"),
            content_disposition_type: String::from("attachment"),
            unoptimized: false,
            maximum_redirects: 3,
            maximum_response_body: 50_000_000,
            domains: Vec::new(),
        }
    }
}

/// `has` / `missing` condition of redirects, rewrites, headers and proxy matchers.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteCondition {
    /// `header | cookie | host | query`.
    #[serde(rename = "type")]
    pub kind: String,
    /// Header/cookie/query name (ignored for `host`).
    #[serde(default)]
    pub key: String,
    /// Optional value (string or regex in the Next.js grammar).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

/// One `redirects()` / `rewrites()` entry.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteRule {
    /// path-to-regexp source (`/old/:slug*`).
    pub source: String,
    /// Destination with `:param` substitutions (may be absolute).
    #[serde(default)]
    pub destination: String,
    /// Redirects: `308` when true, `307` when false.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub permanent: Option<bool>,
    /// Explicit status code (`301 | 302 | 303 | 307 | 308`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status_code: Option<u16>,
    /// `false` to skip `basePath` prefixing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_path: Option<bool>,
    /// `false` to skip locale prefixing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub locale: Option<bool>,
    /// Conditions that must match.
    #[serde(default)]
    pub has: Vec<RouteCondition>,
    /// Conditions that must not match.
    #[serde(default)]
    pub missing: Vec<RouteCondition>,
}

/// `rewrites()` phases.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Rewrites {
    /// Checked before filesystem routes.
    #[serde(default)]
    pub before_files: Vec<RouteRule>,
    /// Checked after filesystem routes, before dynamic routes.
    #[serde(default)]
    pub after_files: Vec<RouteRule>,
    /// Checked last (404 fallback).
    #[serde(default)]
    pub fallback: Vec<RouteRule>,
}

/// One response header of a `headers()` rule.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct HeaderKv {
    /// Header name.
    pub key: String,
    /// Header value.
    pub value: String,
}

/// One `headers()` entry.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HeaderRule {
    /// path-to-regexp source.
    pub source: String,
    /// Headers to set.
    #[serde(default)]
    pub headers: Vec<HeaderKv>,
    /// `false` to skip `basePath` prefixing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_path: Option<bool>,
    /// `false` to skip locale prefixing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub locale: Option<bool>,
    /// Conditions that must match.
    #[serde(default)]
    pub has: Vec<RouteCondition>,
    /// Conditions that must not match.
    #[serde(default)]
    pub missing: Vec<RouteCondition>,
}

/// `cacheLife` profile (seconds). Built-ins: `default` 300/900/INFINITE, `seconds` 30/1/60,
/// `minutes` 300/60/3600, `hours` 300/3600/86400, `days` 300/86400/604800,
/// `weeks` 300/604800/2592000, `max` 300/2592000/31536000.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheLifeProfile {
    /// Client-side stale window.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stale: Option<u64>,
    /// Server revalidation period.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revalidate: Option<u64>,
    /// Hard expiry.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expire: Option<u64>,
}

/// Pages Router `i18n` config.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct I18nConfig {
    /// Supported locales.
    #[serde(default)]
    pub locales: Vec<String>,
    /// Default locale.
    #[serde(default)]
    pub default_locale: String,
    /// Domain routing entries, kept raw.
    #[serde(default)]
    pub domains: Vec<Value>,
    /// `false` disables Accept-Language detection.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub locale_detection: Option<bool>,
}

/// `compiler` options (SWC transforms).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompilerConfig {
    /// `boolean | { exclude: string[] }`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remove_console: Option<Value>,
    /// `boolean | { properties: string[] }`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub react_remove_properties: Option<Value>,
    /// styled-components options.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub styled_components: Option<Value>,
    /// emotion options.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub emotion: Option<Value>,
    /// relay options.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relay: Option<Value>,
    /// Compile-time constants (client + server).
    #[serde(default)]
    pub define: BTreeMap<String, String>,
    /// Compile-time constants (server only).
    #[serde(default)]
    pub define_server: BTreeMap<String, String>,
}

/// `typescript` options.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TypescriptConfig {
    /// Skip type errors during build.
    #[serde(default)]
    pub ignore_build_errors: bool,
    /// Custom tsconfig path.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tsconfig_path: Option<String>,
}

/// Statically evaluated `next.config.(js|mjs|ts)` (Next.js 16.4 `NextConfig`).
///
/// Function-valued keys (`redirects()`, `rewrites()`, `headers()`, `generateBuildId()`,
/// `webpack()`, `exportPathMap()`) are evaluated only when their body is a literal
/// `return [...]`; otherwise the corresponding `*_dynamic` flag is set.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct NextConfig {
    /// `reactStrictMode`.
    pub react_strict_mode: Option<bool>,
    /// `output`: `standalone | export`.
    pub output: Option<String>,
    /// `distDir` (default `.next`).
    pub dist_dir: Option<String>,
    /// `basePath`.
    pub base_path: String,
    /// `assetPrefix` (may be a full CDN origin).
    pub asset_prefix: String,
    /// `trailingSlash`.
    pub trailing_slash: bool,
    /// `skipTrailingSlashRedirect`.
    pub skip_trailing_slash_redirect: bool,
    /// `skipProxyUrlNormalize` (alias `skipMiddlewareUrlNormalize`).
    pub skip_proxy_url_normalize: bool,
    /// `compress` (default true).
    pub compress: Option<bool>,
    /// `poweredByHeader` (default true).
    pub powered_by_header: Option<bool>,
    /// `generateEtags` (default true).
    pub generate_etags: Option<bool>,
    /// `reactCompiler`: `bool | { compilationMode, panicThreshold }`.
    pub react_compiler: Option<Value>,
    /// `reactProductionProfiling`.
    pub react_production_profiling: Option<bool>,
    /// `reactMaxHeadersLength` (default 6000).
    pub react_max_headers_length: Option<u64>,
    /// `cacheComponents` (replaces `experimental.{ppr,dynamicIO,useCache}`).
    pub cache_components: bool,
    /// `partialPrefetching`.
    pub partial_prefetching: Option<bool>,
    /// `typedRoutes` (top-level since 15.5).
    pub typed_routes: bool,
    /// `transpilePackages`.
    pub transpile_packages: Vec<String>,
    /// `serverExternalPackages`.
    pub server_external_packages: Vec<String>,
    /// `bundlePagesRouterDependencies`.
    pub bundle_pages_router_dependencies: bool,
    /// `pageExtensions` (default `tsx, ts, jsx, js`).
    pub page_extensions: Vec<String>,
    /// `env` (build-time inlined).
    pub env: BTreeMap<String, String>,
    /// `deploymentId` (skew protection `?dpl=`).
    pub deployment_id: Option<String>,
    /// `supportsImmutableAssets`.
    pub supports_immutable_assets: Option<bool>,
    /// `expireTime` (default 31536000).
    pub expire_time: Option<u64>,
    /// `crossOrigin`: `anonymous | use-credentials`.
    pub cross_origin: Option<String>,
    /// `productionBrowserSourceMaps`.
    pub production_browser_source_maps: bool,
    /// `allowedDevOrigins`.
    pub allowed_dev_origins: Vec<String>,
    /// `staticPageGenerationTimeout` (default 60).
    pub static_page_generation_timeout: Option<u64>,
    /// `turbopack`.
    pub turbopack: TurbopackConfig,
    /// `images`.
    pub images: ImagesConfig,
    /// Statically evaluated `redirects()`.
    pub redirects: Vec<RouteRule>,
    /// Statically evaluated `rewrites()`.
    pub rewrites: Rewrites,
    /// Statically evaluated `headers()`.
    pub headers: Vec<HeaderRule>,
    /// `redirects()` body could not be evaluated statically.
    pub redirects_dynamic: bool,
    /// `rewrites()` body could not be evaluated statically.
    pub rewrites_dynamic: bool,
    /// `headers()` body could not be evaluated statically.
    pub headers_dynamic: bool,
    /// `compiler`.
    pub compiler: CompilerConfig,
    /// `typescript`.
    pub typescript: TypescriptConfig,
    /// `experimental.*` kept raw.
    pub experimental: Map<String, Value>,
    /// `cacheLife` custom profiles.
    pub cache_life: BTreeMap<String, CacheLifeProfile>,
    /// `cacheHandlers` (`default`, `remote`, `static`, custom).
    pub cache_handlers: BTreeMap<String, String>,
    /// Legacy ISR `cacheHandler` path.
    pub cache_handler: Option<String>,
    /// `cacheMaxMemorySize` bytes.
    pub cache_max_memory_size: Option<u64>,
    /// Pages Router `i18n`.
    pub i18n: Option<I18nConfig>,
    /// `logging` kept raw.
    pub logging: Option<Value>,
    /// `devIndicators` kept raw.
    pub dev_indicators: Option<Value>,
    /// `outputFileTracingRoot`.
    pub output_file_tracing_root: Option<String>,
    /// `outputFileTracingIncludes` kept raw.
    pub output_file_tracing_includes: Option<Value>,
    /// `outputFileTracingExcludes` kept raw.
    pub output_file_tracing_excludes: Option<Value>,
    /// `modularizeImports` kept raw.
    pub modularize_imports: Map<String, Value>,
    /// `sassOptions` kept raw.
    pub sass_options: Option<Value>,
    /// `htmlLimitedBots` regex source.
    pub html_limited_bots: Option<String>,
    /// `adapterPath`.
    pub adapter_path: Option<String>,
    /// `generateBuildId` present.
    pub generate_build_id_present: bool,
    /// `webpack()` present (build fails under Turbopack unless `--webpack`).
    pub webpack_present: bool,
    /// `exportPathMap` present.
    pub export_path_map_present: bool,
    /// Removed-in-16 keys that were present (`eslint`, `amp`, `serverRuntimeConfig`, ...).
    pub removed_keys: Vec<String>,
    /// Keys renamed by the parser (`from` -> `to`).
    pub renamed_keys: Vec<(String, String)>,
    /// Config wrapped by plugins (`withMDX`, `withSentryConfig`, ...), outermost first.
    pub wrappers: Vec<String>,
    /// Unknown top-level keys kept raw.
    pub extra: Map<String, Value>,
}

impl Default for NextConfig {
    fn default() -> Self {
        Self {
            react_strict_mode: None,
            output: None,
            dist_dir: None,
            base_path: String::new(),
            asset_prefix: String::new(),
            trailing_slash: false,
            skip_trailing_slash_redirect: false,
            skip_proxy_url_normalize: false,
            compress: None,
            powered_by_header: None,
            generate_etags: None,
            react_compiler: None,
            react_production_profiling: None,
            react_max_headers_length: None,
            cache_components: false,
            partial_prefetching: None,
            typed_routes: false,
            transpile_packages: Vec::new(),
            server_external_packages: Vec::new(),
            bundle_pages_router_dependencies: false,
            page_extensions: vec![
                String::from("tsx"),
                String::from("ts"),
                String::from("jsx"),
                String::from("js"),
            ],
            env: BTreeMap::new(),
            deployment_id: None,
            supports_immutable_assets: None,
            expire_time: None,
            cross_origin: None,
            production_browser_source_maps: false,
            allowed_dev_origins: Vec::new(),
            static_page_generation_timeout: None,
            turbopack: TurbopackConfig::default(),
            images: ImagesConfig::default(),
            redirects: Vec::new(),
            rewrites: Rewrites::default(),
            headers: Vec::new(),
            redirects_dynamic: false,
            rewrites_dynamic: false,
            headers_dynamic: false,
            compiler: CompilerConfig::default(),
            typescript: TypescriptConfig::default(),
            experimental: Map::new(),
            cache_life: BTreeMap::new(),
            cache_handlers: BTreeMap::new(),
            cache_handler: None,
            cache_max_memory_size: None,
            i18n: None,
            logging: None,
            dev_indicators: None,
            output_file_tracing_root: None,
            output_file_tracing_includes: None,
            output_file_tracing_excludes: None,
            modularize_imports: Map::new(),
            sass_options: None,
            html_limited_bots: None,
            adapter_path: None,
            generate_build_id_present: false,
            webpack_present: false,
            export_path_map_present: false,
            removed_keys: Vec::new(),
            renamed_keys: Vec::new(),
            wrappers: Vec::new(),
            extra: Map::new(),
        }
    }
}

/// Report produced while converting a [`NextConfig`] into a [`YoloWebConfig`].
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ConfigMigrationReport {
    /// Per-key mapping status.
    pub features: Vec<FeatureStatus>,
    /// Diagnostics (removed keys, dynamic functions, unsupported loaders...).
    pub diagnostics: Vec<Diagnostic>,
}

// ============================================================================
// Bundler plan (Turbopack -> Bun.build / bunfig.toml)
// ============================================================================

/// Bun bundler loader names (`Loader` union of `bun-types@1.4.4`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BunLoader {
    /// JavaScript.
    Js,
    /// JSX.
    Jsx,
    /// TypeScript.
    Ts,
    /// TSX.
    Tsx,
    /// JSON.
    Json,
    /// JSONC.
    Jsonc,
    /// TOML.
    Toml,
    /// YAML.
    Yaml,
    /// Plain text string export.
    Text,
    /// Copied asset, import yields its URL.
    File,
    /// Native addon.
    Napi,
    /// WebAssembly.
    Wasm,
    /// CSS.
    Css,
    /// HTML bundle.
    Html,
}

/// Kind of Bun plugin the Turbopack translator emits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BunPluginKind {
    /// `onResolve` alias rewrite (`resolveAlias`).
    AliasResolve,
    /// Override the loader for a glob (`rules[glob].type` or trivial loaders).
    LoaderOverride,
    /// `@svgr/webpack` replacement: SVG -> React component module.
    SvgReact,
    /// Markdown/MDX -> module (`Bun.markdown`).
    Markdown,
    /// Raw text import (`raw-loader`, `asset/source`, `graphql-tag/loader`).
    Text,
    /// Loader kept as a runtime-less stub with a diagnostic.
    Unsupported,
}

/// One Bun plugin to generate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BunPluginSpec {
    /// Plugin name (`yolo:svg-react`).
    pub name: String,
    /// Kind.
    pub kind: BunPluginKind,
    /// Source glob or regex (JavaScript regex source) the plugin filters on.
    pub filter: String,
    /// Original Turbopack loader name, when derived from one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origin_loader: Option<String>,
    /// Loader to apply after transformation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub loader: Option<BunLoader>,
    /// JSON options forwarded to the plugin.
    #[serde(default)]
    pub options: Map<String, Value>,
}

/// Complete bundler plan for a project.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BunBundlerPlan {
    /// Plugins in application order.
    pub plugins: Vec<BunPluginSpec>,
    /// `Bun.build({ loader })` map (`.svg` -> `file`, ...).
    pub loaders: BTreeMap<String, BunLoader>,
    /// Universal aliases (`resolveAlias` string targets).
    pub aliases: BTreeMap<String, String>,
    /// Browser-only aliases (`resolveAlias[name].browser`).
    pub browser_aliases: BTreeMap<String, String>,
    /// Aliases resolved to `false` (empty module).
    pub empty_modules: Vec<String>,
    /// Resolve extensions (Bun order).
    pub extensions: Vec<String>,
    /// Project root.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub root: Option<String>,
    /// `Bun.build({ define })` entries (`process.env.NEXT_PUBLIC_*`, `compiler.define`).
    pub define: BTreeMap<String, String>,
    /// Packages kept external (`serverExternalPackages`).
    pub external: Vec<String>,
    /// Features that could not be translated.
    pub unsupported: Vec<FeatureStatus>,
}

// ============================================================================
// App Router / Pages Router route manifest
// ============================================================================

/// Kind of an `app/` directory segment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SegmentKind {
    /// `app/` itself.
    Root,
    /// Literal segment (`blog`).
    Static,
    /// `[slug]`.
    Dynamic,
    /// `[...slug]`.
    CatchAll,
    /// `[[...slug]]`.
    OptionalCatchAll,
    /// `(group)`: omitted from the URL.
    Group,
    /// `@slot`: parallel route.
    ParallelSlot,
    /// `(.)x`, `(..)x`, `(..)(..)x`, `(...)x`: intercepting route.
    Intercept,
    /// `_private`: excluded from routing.
    Private,
}

/// Kind of an `app/` metadata file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MetadataFileKind {
    /// `favicon.ico` (root only).
    Favicon,
    /// `icon.*` / `icon[0-9]*.*`.
    Icon,
    /// `apple-icon.*`.
    AppleIcon,
    /// `opengraph-image.*`.
    OpengraphImage,
    /// `twitter-image.*`.
    TwitterImage,
    /// `sitemap.xml` / `sitemap.(js|ts)`.
    Sitemap,
    /// `robots.txt` / `robots.(js|ts)`.
    Robots,
    /// `manifest.(json|webmanifest|js|ts)`.
    Manifest,
}

/// One metadata file found in a segment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetadataFile {
    /// Kind.
    pub kind: MetadataFileKind,
    /// Path relative to the project root.
    pub path: String,
    /// `true` for `.js/.ts/.tsx` generators, `false` for static assets.
    pub generated: bool,
}

/// Special files of one segment (paths relative to the project root).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SegmentFiles {
    /// `page.*`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page: Option<String>,
    /// `layout.*`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layout: Option<String>,
    /// `template.*`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub template: Option<String>,
    /// `loading.*`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub loading: Option<String>,
    /// `error.*`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// `global-error.*`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub global_error: Option<String>,
    /// `not-found.*`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub not_found: Option<String>,
    /// `forbidden.*`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub forbidden: Option<String>,
    /// `unauthorized.*`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unauthorized: Option<String>,
    /// `default.*` (parallel slot fallback).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
    /// `route.*` (route handler).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub route: Option<String>,
    /// Metadata files.
    #[serde(default)]
    pub metadata: Vec<MetadataFile>,
}

/// One directory node of the `app/` tree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteSegment {
    /// Raw directory name (`[slug]`, `(marketing)`, `@modal`, `(.)photos`).
    pub name: String,
    /// Kind.
    pub kind: SegmentKind,
    /// Dynamic parameter name (`slug`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub param: Option<String>,
    /// Intercept level: 0 `(.)`, 1 `(..)`, 2 `(..)(..)`, 3 `(...)` root.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub intercept_level: Option<u8>,
    /// Intercepted segment name (`photos` for `(.)photos`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub intercept_target: Option<String>,
    /// Special files.
    pub files: SegmentFiles,
    /// Child segments in directory order.
    #[serde(default)]
    pub children: Vec<RouteSegment>,
}

/// Dynamic parameter of a route.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteParam {
    /// Parameter name.
    pub name: String,
    /// `[...slug]` / `[[...slug]]`.
    pub catch_all: bool,
    /// `[[...slug]]`.
    pub optional: bool,
}

/// Kind of a route entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RouteKind {
    /// `page.*` (App Router) or `pages/**/*.tsx`.
    Page,
    /// `route.*` (App Router) or `pages/api/**`.
    RouteHandler,
    /// Generated metadata route (`sitemap.ts`, `opengraph-image.tsx`, ...).
    Metadata,
}

/// Module directive at the top of a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ModuleDirective {
    /// No directive (Server Component by default in `app/`).
    #[default]
    None,
    /// `"use client"`.
    UseClient,
    /// `"use server"`.
    UseServer,
    /// `"use cache"` (including `: private`, `: remote`, `: <handler>`).
    UseCache,
}

/// Route segment config exports and static analysis of a page/layout/route module.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SegmentConfig {
    /// `export const dynamic`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dynamic: Option<String>,
    /// `export const dynamicParams`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dynamic_params: Option<bool>,
    /// `export const revalidate` (`false | 0 | n`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revalidate: Option<Value>,
    /// `export const fetchCache`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fetch_cache: Option<String>,
    /// `export const runtime` (`nodejs | edge`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime: Option<String>,
    /// `export const preferredRegion`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preferred_region: Option<Value>,
    /// `export const maxDuration`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_duration: Option<u64>,
    /// `export const ensureStatic` (cacheComponents, 16.4).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ensure_static: Option<Value>,
    /// `export const prefetch` (cacheComponents, 16.3).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prefetch: Option<String>,
    /// `export const instant`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instant: Option<bool>,
    /// `export const experimental_ppr` (removed in 16, reported).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub experimental_ppr: Option<bool>,
    /// `generateStaticParams` exported.
    #[serde(default)]
    pub has_generate_static_params: bool,
    /// `generateMetadata` exported.
    #[serde(default)]
    pub has_generate_metadata: bool,
    /// `generateViewport` exported.
    #[serde(default)]
    pub has_generate_viewport: bool,
    /// `generateImageMetadata` exported.
    #[serde(default)]
    pub has_generate_image_metadata: bool,
    /// `generateSitemaps` exported.
    #[serde(default)]
    pub has_generate_sitemaps: bool,
    /// Static `export const metadata = {...}`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub static_metadata: Option<Metadata>,
    /// Static `export const viewport = {...}`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub static_viewport: Option<Viewport>,
    /// Module directive.
    #[serde(default)]
    pub directive: ModuleDirective,
    /// Default export identifier (component or handler name).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_export: Option<String>,
    /// Named exports.
    #[serde(default)]
    pub named_exports: Vec<String>,
    /// Default export is `async`.
    #[serde(default)]
    pub default_export_async: bool,
    /// React hooks used (client components).
    #[serde(default)]
    pub hooks_used: Vec<String>,
    /// `next/*` specifiers imported by the module.
    #[serde(default)]
    pub next_imports: Vec<String>,
    /// Diagnostics (parse errors, removed segment configs).
    #[serde(default)]
    pub diagnostics: Vec<Diagnostic>,
}

/// Intercepting route specification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InterceptSpec {
    /// Level (see [`RouteSegment::intercept_level`]).
    pub level: u8,
    /// Resolved intercepted URL pattern (`/photos/[id]`).
    pub target: String,
}

/// One routable entry (page, route handler, or metadata route).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteEntry {
    /// Next.js app path including groups and slots (`/(marketing)/pricing`, `/dashboard/@analytics`).
    pub app_path: String,
    /// URL pattern with Next.js brackets, groups/slots removed (`/blog/[slug]`, `/docs/[...slug]`).
    pub url_pattern: String,
    /// `Bun.serve` routes key (`/blog/:slug`, `/docs/*`).
    pub bun_pattern: String,
    /// TanStack Router path (`/blog/$slug`, `/docs/$`).
    pub tanstack_path: String,
    /// Dioxus `#[route("...")]` path (`/blog/:slug`, `/docs/:..slug`).
    pub dioxus_route: String,
    /// Dioxus enum variant (`BlogSlug`).
    pub dioxus_variant: String,
    /// Leptos `path!()` (`/blog/:slug`, `/docs/*slug`).
    pub leptos_path: String,
    /// Dynamic parameters in order.
    #[serde(default)]
    pub params: Vec<RouteParam>,
    /// Kind.
    pub kind: RouteKind,
    /// Module path relative to the project root.
    pub file: String,
    /// Layout chain root -> leaf (relative paths).
    #[serde(default)]
    pub layouts: Vec<String>,
    /// Templates in the chain.
    #[serde(default)]
    pub templates: Vec<String>,
    /// Nearest `loading.*`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub loading: Option<String>,
    /// Nearest `error.*`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Nearest `not-found.*`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub not_found: Option<String>,
    /// Parallel slot names available to the leaf layout (`analytics`, `modal`).
    #[serde(default)]
    pub parallel_slots: Vec<String>,
    /// Slot this entry renders into (`modal` for `app/@modal/...`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slot: Option<String>,
    /// Intercepting route information.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub intercepting: Option<InterceptSpec>,
    /// Segment config and module analysis.
    #[serde(default)]
    pub segment_config: SegmentConfig,
    /// HTTP methods exported by a route handler (`GET`, `POST`, ...).
    #[serde(default)]
    pub methods: Vec<String>,
    /// Metadata kind for generated metadata routes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata_kind: Option<MetadataFileKind>,
    /// Entry comes from the legacy `pages/` router.
    #[serde(default)]
    pub legacy_pages: bool,
    /// Legacy data functions detected (`getServerSideProps`, `getStaticProps`, `getStaticPaths`, `getInitialProps`).
    #[serde(default)]
    pub legacy_data_functions: Vec<String>,
}

/// Complete route manifest of a project.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteManifest {
    /// Project root (absolute, `/` separators).
    pub root: String,
    /// `app/` or `src/app/` relative path, when present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub app_dir: Option<String>,
    /// `pages/` or `src/pages/` relative path, when present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pages_dir: Option<String>,
    /// Segment tree (root node named `app`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tree: Option<RouteSegment>,
    /// Flattened entries sorted by matching precedence (static > dynamic > catch-all > optional).
    #[serde(default)]
    pub entries: Vec<RouteEntry>,
    /// Root layout path.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub root_layout: Option<String>,
    /// Root `not-found.*`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub root_not_found: Option<String>,
    /// `global-error.*`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub global_error: Option<String>,
    /// `proxy.(js|ts)` or legacy `middleware.(js|ts)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proxy_file: Option<String>,
    /// `instrumentation.(js|ts)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instrumentation: Option<String>,
    /// `instrumentation-client.(js|ts)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instrumentation_client: Option<String>,
    /// `mdx-components.(js|tsx)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mdx_components: Option<String>,
    /// Static metadata files at the app root (`favicon.ico`, `robots.txt`, ...).
    #[serde(default)]
    pub static_metadata: Vec<MetadataFile>,
    /// Diagnostics.
    #[serde(default)]
    pub diagnostics: Vec<Diagnostic>,
}

#[allow(clippy::derivable_impls)] // explicit so the empty manifest stays readable next to the struct
impl Default for RouteManifest {
    fn default() -> Self {
        Self {
            root: String::new(),
            app_dir: None,
            pages_dir: None,
            tree: None,
            entries: Vec::new(),
            root_layout: None,
            root_not_found: None,
            global_error: None,
            proxy_file: None,
            instrumentation: None,
            instrumentation_client: None,
            mdx_components: None,
            static_metadata: Vec::new(),
            diagnostics: Vec::new(),
        }
    }
}

// ============================================================================
// Metadata API
// ============================================================================

/// `metadata.title`: `string | { default, template, absolute }`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TitleSpec {
    /// Default title used by children without their own title.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
    /// Template applied to child titles (`%s | Site`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub template: Option<String>,
    /// Title ignoring parent templates.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub absolute: Option<String>,
}

/// Static `metadata` export (subset modelled explicitly, rest kept raw).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Metadata {
    /// Title.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<TitleSpec>,
    /// Description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// `metadataBase` URL.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata_base: Option<String>,
    /// Application name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub application_name: Option<String>,
    /// Authors (`{ name, url }` kept raw).
    #[serde(default)]
    pub authors: Vec<Value>,
    /// Generator.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generator: Option<String>,
    /// Keywords.
    #[serde(default)]
    pub keywords: Vec<String>,
    /// Referrer policy.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub referrer: Option<String>,
    /// Creator.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub creator: Option<String>,
    /// Publisher.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub publisher: Option<String>,
    /// Robots (`string | { index, follow, googleBot }`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub robots: Option<Value>,
    /// Alternates (`{ canonical, languages, media, types }`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alternates: Option<Value>,
    /// Icons (`{ icon, shortcut, apple, other }` or string).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icons: Option<Value>,
    /// Manifest URL.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manifest: Option<String>,
    /// Open Graph.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub open_graph: Option<Value>,
    /// Twitter card.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub twitter: Option<Value>,
    /// Verification (`google`, `yandex`, `other`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verification: Option<Value>,
    /// Apple web app.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub apple_web_app: Option<Value>,
    /// Format detection.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format_detection: Option<Value>,
    /// Category.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    /// Deprecated `themeColor` on metadata (moved to viewport).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theme_color_legacy: Option<Value>,
    /// Deprecated `viewport` on metadata.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub viewport_legacy: Option<Value>,
    /// Deprecated `colorScheme` on metadata.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color_scheme_legacy: Option<String>,
    /// `other` and unmodelled keys.
    #[serde(default)]
    pub other: BTreeMap<String, Value>,
}

/// `viewport.themeColor` entry.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ThemeColor {
    /// Media query (`(prefers-color-scheme: dark)`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub media: Option<String>,
    /// CSS color.
    pub color: String,
}

/// Static `viewport` export.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Viewport {
    /// `width` (`device-width` or number).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<String>,
    /// `height`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<String>,
    /// `initialScale`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub initial_scale: Option<f64>,
    /// `minimumScale`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minimum_scale: Option<f64>,
    /// `maximumScale`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maximum_scale: Option<f64>,
    /// `userScalable`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_scalable: Option<bool>,
    /// `viewportFit`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub viewport_fit: Option<String>,
    /// `interactiveWidget`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interactive_widget: Option<String>,
    /// `themeColor` entries.
    #[serde(default)]
    pub theme_color: Vec<ThemeColor>,
    /// `colorScheme`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color_scheme: Option<String>,
}

// ============================================================================
// next/font
// ============================================================================

/// One `src` entry of `next/font/local`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalFontSrc {
    /// Path relative to the calling file.
    pub path: String,
    /// Weight (`400`, `100 900`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub weight: Option<String>,
    /// Style (`normal`, `italic`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
}

/// Origin of a font request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum FontSource {
    /// `next/font/google`: family from the export name (`Roboto_Mono` -> `Roboto Mono`).
    Google {
        /// Family display name.
        family: String,
    },
    /// `next/font/local`.
    Local {
        /// Source files.
        src: Vec<LocalFontSrc>,
    },
}

/// One `Inter({...})` / `localFont({...})` call.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FontRequest {
    /// Binding name (`inter`).
    pub binding: String,
    /// Imported identifier (`Inter`, `localFont`).
    pub import_name: String,
    /// Origin.
    pub source: FontSource,
    /// `subsets`.
    #[serde(default)]
    pub subsets: Vec<String>,
    /// `weight` values (`400`, `100 900`, `variable`).
    #[serde(default)]
    pub weights: Vec<String>,
    /// `style` values.
    #[serde(default)]
    pub styles: Vec<String>,
    /// `display` (default `swap`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display: Option<String>,
    /// `variable` CSS custom property (`--font-inter`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variable: Option<String>,
    /// `preload` (default true).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preload: Option<bool>,
    /// `fallback` families.
    #[serde(default)]
    pub fallback: Vec<String>,
    /// `adjustFontFallback`: `true | false | "Arial" | "Times New Roman"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adjust_font_fallback: Option<Value>,
    /// `axes` (variable fonts).
    #[serde(default)]
    pub axes: Vec<String>,
    /// `declarations` (`{ prop, value }`).
    #[serde(default)]
    pub declarations: Vec<(String, String)>,
    /// File the call was found in.
    pub file: String,
}

/// One `@font-face` source to self-host.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FontFaceAsset {
    /// Remote URL (`https://fonts.gstatic.com/s/...`) or local path.
    pub source: String,
    /// Served path (`/_yolo/fonts/<hash>.woff2`).
    pub local_path: String,
    /// Family.
    pub family: String,
    /// Weight.
    pub weight: String,
    /// Style.
    pub style: String,
    /// `unicode-range`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unicode_range: Option<String>,
    /// Subset comment (`latin`, `cyrillic-ext`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subset: Option<String>,
    /// Format (`woff2`, `truetype`).
    pub format: String,
    /// Preload this file.
    pub preload: bool,
}

/// SFNT metrics (`head`, `hhea`, `OS/2`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FontMetrics {
    /// Family name.
    pub family: String,
    /// `head.unitsPerEm`.
    pub units_per_em: u16,
    /// `hhea.ascender`.
    pub ascent: i32,
    /// `hhea.descender` (negative).
    pub descent: i32,
    /// `hhea.lineGap`.
    pub line_gap: i32,
    /// `OS/2.sCapHeight`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cap_height: Option<i32>,
    /// `OS/2.xAvgCharWidth` (capsize `xWidthAvg`).
    pub x_avg_char_width: i32,
    /// Mean advance width over `aaabcdeeeefghiijklmnnoopqrrssttuvwxyz      ` (`azAvgWidth`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub az_avg_width: Option<f64>,
}

/// `@font-face` descriptors of the size-adjusted fallback.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FallbackOverrides {
    /// `Arial` or `Times New Roman`.
    pub fallback_family: String,
    /// `ascent-override` (`90.49%`).
    pub ascent_override: String,
    /// `descent-override`.
    pub descent_override: String,
    /// `line-gap-override`.
    pub line_gap_override: String,
    /// `size-adjust`.
    pub size_adjust: String,
}

/// Resolved plan for one font request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FontPlan {
    /// Request.
    pub request: FontRequest,
    /// Google CSS2 URL to fetch (None for local fonts).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub css2_url: Option<String>,
    /// Family used in `font-family` (`'Inter', 'Inter Fallback'`).
    pub family_css: String,
    /// Generated class name (`yolo-font-<hash>`).
    pub class_name: String,
    /// Generated variable class (`yolo-font-var-<hash>`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variable_class: Option<String>,
    /// Fallback overrides when `adjustFontFallback` is enabled.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fallback: Option<FallbackOverrides>,
    /// Font files to self-host.
    #[serde(default)]
    pub assets: Vec<FontFaceAsset>,
    /// Complete CSS (`@font-face` rules + classes).
    pub css: String,
    /// Yolo preset font role the request maps to (`sans`, `mono`, `display`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preset_role: Option<String>,
}

// ============================================================================
// next/image + CDN
// ============================================================================

/// One `<Image>` usage found in a module.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageUsage {
    /// File.
    pub file: String,
    /// Literal `src` when static.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub src: Option<String>,
    /// Static import binding used as `src`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub src_import: Option<String>,
    /// `alt`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alt: Option<String>,
    /// `width`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    /// `height`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
    /// `fill`.
    #[serde(default)]
    pub fill: bool,
    /// `sizes`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sizes: Option<String>,
    /// `quality`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quality: Option<u8>,
    /// `priority` (deprecated) or `preload`.
    #[serde(default)]
    pub priority: bool,
    /// `placeholder`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub placeholder: Option<String>,
    /// `loader` prop present.
    #[serde(default)]
    pub custom_loader: bool,
    /// `unoptimized`.
    #[serde(default)]
    pub unoptimized: bool,
    /// `loading`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub loading: Option<String>,
    /// `fetchPriority`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fetch_priority: Option<String>,
    /// `src` is an absolute URL.
    #[serde(default)]
    pub remote: bool,
    /// Import source (`next/image`, `next/legacy/image`).
    #[serde(default)]
    pub import_source: String,
}

/// Aggregate image usage report.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageUsageReport {
    /// Usages.
    pub usages: Vec<ImageUsage>,
    /// Distinct remote hosts referenced.
    pub remote_hosts: Vec<String>,
    /// Remote `src` values not covered by `remotePatterns`.
    pub unmatched_remote: Vec<String>,
    /// Diagnostics.
    pub diagnostics: Vec<Diagnostic>,
}

/// Image loader families.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ImageLoaderKind {
    /// `/_yolo/image?url=&w=&q=` (Next default-compatible).
    Default,
    /// `images.loaderFile`.
    Custom,
    /// `/cdn-cgi/image/width=,quality=,format=auto/`.
    Cloudflare,
    /// `f_auto,c_limit,w_,q_`.
    Cloudinary,
    /// `?auto=format&fit=max&w=&q=`.
    Imgix,
    /// `?imwidth=`.
    Akamai,
    /// `?auto=webp&width=&quality=`.
    Fastly,
    /// `?width=&quality=`.
    Supabase,
    /// `?format=auto&width=&quality=` (CloudFront).
    Cloudfront,
    /// No transformation.
    Unoptimized,
}

/// One `srcset` candidate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SrcSetCandidate {
    /// URL.
    pub url: String,
    /// Descriptor (`640w`, `2x`).
    pub descriptor: String,
}

/// Runtime policy of the image optimizer route.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageOptimizerPolicy {
    /// Primary route (`/_yolo/image`).
    pub route: String,
    /// Compatibility routes (`/_next/image`).
    pub compat_routes: Vec<String>,
    /// Loader kind.
    pub loader: ImageLoaderKind,
    /// Custom loader file.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub loader_file: Option<String>,
    /// Negotiable formats in preference order.
    pub formats: Vec<String>,
    /// Device sizes.
    pub device_sizes: Vec<u32>,
    /// Image sizes.
    pub image_sizes: Vec<u32>,
    /// Allowed qualities.
    pub qualities: Vec<u8>,
    /// Minimum cache TTL seconds.
    pub minimum_cache_ttl: u64,
    /// `Cache-Control` template (`public, max-age={max_age}, must-revalidate`).
    pub cache_control: String,
    /// Allow SVG.
    pub allow_svg: bool,
    /// Allow private IP upstreams.
    pub allow_local_ip: bool,
    /// CSP header value.
    pub content_security_policy: String,
    /// `attachment | inline`.
    pub content_disposition_type: String,
    /// Remote patterns.
    pub remote_patterns: Vec<RemotePattern>,
    /// Local patterns.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub local_patterns: Option<Vec<LocalPattern>>,
    /// Max upstream redirects.
    pub maximum_redirects: u32,
    /// Max upstream body.
    pub maximum_response_body: u64,
    /// Global `unoptimized`.
    pub unoptimized: bool,
}

/// Output mode (`output` key).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OutputMode {
    /// `Bun.serve` server.
    #[default]
    Server,
    /// Single executable (`bun build --compile`).
    Standalone,
    /// Static HTML export.
    Export,
}

/// CDN upload target.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CdnUploadTarget {
    /// `s3 | r2 | gcs | minio`.
    pub provider: String,
    /// Bucket.
    pub bucket: String,
    /// Key prefix (`_yolo/static/`).
    pub prefix: String,
    /// Custom endpoint.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,
    /// Region.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    /// Public base URL serving the bucket.
    pub public_base_url: String,
}

/// Static asset & CDN plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CdnPlan {
    /// `assetPrefix` (CDN origin or path).
    pub asset_prefix: String,
    /// `basePath`.
    pub base_path: String,
    /// Hashed assets route (`/_yolo/static`).
    pub static_route: String,
    /// Compatibility routes (`/_next/static`).
    pub compat_static_routes: Vec<String>,
    /// Fonts route (`/_yolo/fonts`).
    pub fonts_route: String,
    /// `public/` served from `/`.
    pub public_dir: String,
    /// `Cache-Control` for hashed assets.
    pub immutable_cache_control: String,
    /// `Cache-Control` for `public/`.
    pub public_cache_control: String,
    /// `Cache-Control` for dynamic HTML.
    pub dynamic_html_cache_control: String,
    /// `deploymentId`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deployment_id: Option<String>,
    /// Skew protection query key (`dpl`).
    pub skew_query: String,
    /// Skew protection request header (`x-deployment-id`).
    pub skew_header: String,
    /// Output mode.
    pub output: OutputMode,
    /// `expireTime` for ISR `stale-while-revalidate`.
    pub expire_time: u64,
    /// `compress`.
    pub compress: bool,
    /// `poweredByHeader`.
    pub powered_by_header: bool,
    /// `generateEtags`.
    pub generate_etags: bool,
    /// `trailingSlash`.
    pub trailing_slash: bool,
    /// Upload target.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upload: Option<CdnUploadTarget>,
}

impl Default for CdnPlan {
    fn default() -> Self {
        Self {
            asset_prefix: String::new(),
            base_path: String::new(),
            static_route: String::from("/_yolo/static"),
            compat_static_routes: vec![String::from("/_next/static")],
            fonts_route: String::from("/_yolo/fonts"),
            public_dir: String::from("public"),
            immutable_cache_control: String::from("public, max-age=31536000, immutable"),
            public_cache_control: String::from("public, max-age=0"),
            dynamic_html_cache_control: String::from(
                "private, no-cache, no-store, max-age=0, must-revalidate",
            ),
            deployment_id: None,
            skew_query: String::from("dpl"),
            skew_header: String::from("x-deployment-id"),
            output: OutputMode::Server,
            expire_time: 31_536_000,
            compress: true,
            powered_by_header: true,
            generate_etags: true,
            trailing_slash: false,
            upload: None,
        }
    }
}

/// One entry of the CDN asset manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CdnAssetManifestEntry {
    /// Local path relative to the output dir.
    pub path: String,
    /// Public URL (with asset prefix and skew query).
    pub public_url: String,
    /// MIME type.
    pub content_type: String,
    /// `Cache-Control`.
    pub cache_control: String,
    /// Content-hashed name.
    pub hashed: bool,
    /// Size in bytes.
    pub size: u64,
}

// ============================================================================
// Server Components / Server Actions module graph
// ============================================================================

/// Classification of a module in the RSC graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ModuleKind {
    /// Server Component module (default in `app/`).
    #[default]
    Server,
    /// `"use client"` boundary.
    Client,
    /// `"use server"` file-level actions module.
    ServerAction,
    /// `"use cache"` file-level module.
    Cache,
    /// Non-component shared module (utility, types).
    Shared,
}

/// One `"use cache"` function.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheFunction {
    /// Function name.
    pub name: String,
    /// `cacheLife(...)` profile or inline object as JSON text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub life: Option<String>,
    /// `cacheTag(...)` literal tags.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Directive variant (`""`, `private`, `remote`, custom handler).
    #[serde(default)]
    pub variant: String,
}

/// Static analysis of one module.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModuleInfo {
    /// Module id: path relative to the project root with `/` separators.
    pub id: String,
    /// Kind.
    pub kind: ModuleKind,
    /// Directive.
    pub directive: ModuleDirective,
    /// Named + default exports (`default` literal for default export).
    #[serde(default)]
    pub exports: Vec<String>,
    /// Import specifiers.
    #[serde(default)]
    pub imports: Vec<String>,
    /// Functions carrying an inline `"use server"` directive.
    #[serde(default)]
    pub inline_server_actions: Vec<String>,
    /// `"use cache"` functions.
    #[serde(default)]
    pub cache_functions: Vec<CacheFunction>,
    /// `next/*` specifiers used.
    #[serde(default)]
    pub next_imports: Vec<String>,
    /// React hooks used.
    #[serde(default)]
    pub hooks: Vec<String>,
    /// Boundary violations and parse problems.
    #[serde(default)]
    pub diagnostics: Vec<Diagnostic>,
}

/// Flight client reference (`clientModules` entry).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientReference {
    /// Reference id (module id or hashed id).
    pub id: String,
    /// Module id.
    pub module: String,
    /// Export name (`*` for namespace).
    pub export_name: String,
    /// Chunks to load before rendering.
    #[serde(default)]
    pub chunks: Vec<String>,
    /// Async module.
    #[serde(default, rename = "async")]
    pub async_: bool,
}

/// Flight server reference (`serverActions` entry).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerReference {
    /// 40-hex id (`sha1`-shaped, see `rsc::server_reference_id`).
    pub id: String,
    /// Module id.
    pub module: String,
    /// Export (or hoisted inline action) name.
    pub export_name: String,
    /// Inline action hoisted from a Server Component.
    #[serde(default)]
    pub inline: bool,
    /// `"use cache"` function instead of an action.
    #[serde(default)]
    pub cache: bool,
}

/// Module graph of an App Router project.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModuleGraph {
    /// Project root.
    pub root: String,
    /// Modules.
    pub modules: Vec<ModuleInfo>,
    /// Client references.
    pub client_manifest: Vec<ClientReference>,
    /// Server references.
    pub server_manifest: Vec<ServerReference>,
    /// Diagnostics.
    pub diagnostics: Vec<Diagnostic>,
}

// ============================================================================
// proxy.ts / middleware.ts
// ============================================================================

/// `proxy.ts` analysis.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProxySpec {
    /// File path relative to the project root.
    pub file: String,
    /// File is the deprecated `middleware.ts`.
    pub legacy_middleware_name: bool,
    /// Exported function name (`proxy`, `middleware`, `default`).
    #[serde(default)]
    pub export_name: String,
    /// `config.matcher` entries normalised to rules (source + has/missing/locale).
    #[serde(default)]
    pub matchers: Vec<RouteRule>,
    /// `export const runtime` present (invalid for proxy).
    #[serde(default)]
    pub runtime_export: Option<String>,
    /// Uses `NextResponse.rewrite`.
    #[serde(default)]
    pub uses_rewrite: bool,
    /// Uses `NextResponse.redirect`.
    #[serde(default)]
    pub uses_redirect: bool,
    /// Uses `NextResponse.next({ request: { headers } })`.
    #[serde(default)]
    pub uses_next_headers: bool,
    /// Uses `request.cookies` / `response.cookies`.
    #[serde(default)]
    pub uses_cookies: bool,
    /// Uses `event.waitUntil`.
    #[serde(default)]
    pub uses_wait_until: bool,
    /// Uses `request.geo` / `request.ip` (removed in 15).
    #[serde(default)]
    pub uses_removed_geo_ip: bool,
    /// Diagnostics.
    #[serde(default)]
    pub diagnostics: Vec<Diagnostic>,
}

// ============================================================================
// Yolo normalized config (output of config migration)
// ============================================================================

/// Server settings of a Yolo web app.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct YoloServerConfig {
    /// Port (default 3000, overridable by `PORT`).
    pub port: u16,
    /// Hostname.
    pub hostname: String,
    /// `development: { hmr, console }` in dev.
    pub hmr: bool,
    /// Stream browser console to terminal in dev.
    pub browser_console: bool,
    /// Compression (`Bun.gzipSync` / zstd negotiated).
    pub compress: bool,
    /// Emit `x-powered-by: Yolo`.
    pub powered_by_header: bool,
    /// Emit `ETag` on dynamic responses.
    pub generate_etags: bool,
    /// `Server Actions` body size limit in bytes (default 1 MiB).
    pub action_body_size_limit: u64,
    /// Allowed origins for Server Actions (CSRF).
    pub action_allowed_origins: Vec<String>,
}

impl Default for YoloServerConfig {
    fn default() -> Self {
        Self {
            port: 3000,
            hostname: String::from("0.0.0.0"),
            hmr: true,
            browser_console: true,
            compress: true,
            powered_by_header: true,
            generate_etags: true,
            action_body_size_limit: 1_048_576,
            action_allowed_origins: Vec::new(),
        }
    }
}

/// React settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct YoloReactConfig {
    /// `<StrictMode>` wrapper.
    pub strict_mode: bool,
    /// React Compiler (`Bun.build({ reactCompiler })`).
    pub compiler: bool,
    /// Fast Refresh in dev.
    pub fast_refresh: bool,
    /// Cache Components / PPR semantics.
    pub cache_components: bool,
    /// Typed routes (`routeTree.gen.ts` typing).
    pub typed_routes: bool,
}

impl Default for YoloReactConfig {
    fn default() -> Self {
        Self {
            strict_mode: true,
            compiler: false,
            fast_refresh: true,
            cache_components: false,
            typed_routes: true,
        }
    }
}

/// Theme settings (yolo-ui presets).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct YoloThemeConfig {
    /// Preset id (`yolo-neutral`, `yolo-m3`, ...).
    pub preset: String,
    /// Dark by default.
    pub dark: bool,
    /// Custom seed hex.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seed_hex: Option<String>,
}

impl Default for YoloThemeConfig {
    fn default() -> Self {
        Self { preset: String::from("yolo-neutral"), dark: true, seed_hex: None }
    }
}

/// Normalized Yolo web configuration (`yolo.config.ts` / `yolo.toml`).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct YoloWebConfig {
    /// `app/` directory relative to the project root.
    pub app_dir: String,
    /// Legacy `pages/` directory when present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pages_dir: Option<String>,
    /// Build output directory (`.yolo`).
    pub dist_dir: String,
    /// Output mode.
    pub output: OutputMode,
    /// Server.
    pub server: YoloServerConfig,
    /// React.
    pub react: YoloReactConfig,
    /// Theme.
    pub theme: YoloThemeConfig,
    /// CDN / static assets.
    pub cdn: CdnPlan,
    /// Image optimizer.
    pub images: Option<ImageOptimizerPolicy>,
    /// Bundler plan.
    pub bundler: BunBundlerPlan,
    /// Redirects.
    pub redirects: Vec<RouteRule>,
    /// Rewrites.
    pub rewrites: Rewrites,
    /// Headers.
    pub headers: Vec<HeaderRule>,
    /// Public env prefixes inlined into client bundles.
    pub public_env_prefixes: Vec<String>,
    /// Build-time env.
    pub env: BTreeMap<String, String>,
    /// Cache life profiles (built-ins merged with custom).
    pub cache_life: BTreeMap<String, CacheLifeProfile>,
    /// i18n.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub i18n: Option<I18nConfig>,
    /// Transpiled packages.
    pub transpile_packages: Vec<String>,
    /// Server external packages.
    pub server_external_packages: Vec<String>,
    /// Page extensions.
    pub page_extensions: Vec<String>,
    /// Trailing slash.
    pub trailing_slash: bool,
    /// Proxy file.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proxy_file: Option<String>,
}

// ============================================================================
// Project analysis & report
// ============================================================================

/// Complete analysis of a Next.js project.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectAnalysis {
    /// Absolute project root (`/` separators).
    pub root: String,
    /// Project name (`package.json` name or directory name).
    pub name: String,
    /// `next` version from `package.json`, when present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_version: Option<String>,
    /// `next.config.*` path relative to the root.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config_file: Option<String>,
    /// Parsed config.
    pub config: NextConfig,
    /// Normalized config.
    pub yolo_config: YoloWebConfig,
    /// Config migration report.
    pub config_report: ConfigMigrationReport,
    /// Routes.
    pub routes: RouteManifest,
    /// Font requests.
    pub fonts: Vec<FontRequest>,
    /// Font plans.
    pub font_plans: Vec<FontPlan>,
    /// Image usage.
    pub images: ImageUsageReport,
    /// Module graph.
    pub graph: ModuleGraph,
    /// Proxy.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proxy: Option<ProxySpec>,
    /// CDN plan.
    pub cdn: CdnPlan,
    /// Diagnostics aggregated from all stages.
    pub diagnostics: Vec<Diagnostic>,
}

/// Final migration report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrationReport {
    /// Project name.
    pub project: String,
    /// ISO-8601 generation timestamp.
    pub generated_at: String,
    /// Targets emitted.
    pub targets: Vec<Target>,
    /// Feature parity.
    pub features: Vec<FeatureStatus>,
    /// Parity score 0-100 (mapped = 1, partial = 0.5, unsupported = 0).
    pub parity_score: u8,
    /// Diagnostics.
    pub diagnostics: Vec<Diagnostic>,
    /// Emitted file paths.
    pub files_emitted: Vec<String>,
    /// Counts (`routes`, `client_components`, `server_actions`, `fonts`, `images`).
    pub counts: BTreeMap<String, u64>,
}
