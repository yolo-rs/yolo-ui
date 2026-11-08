// SPDX-License-Identifier: Apache-2.0
//! CDN and static asset plan (`assetPrefix`, `basePath`, `deploymentId`, `output`).
//!
//! Hashed build output is served from `{basePath}/_yolo/static` (with `/_next/static` kept as a
//! compatibility route), fonts from `{basePath}/_yolo/fonts`, `public/` from `/`. Skew
//! protection mirrors Next.js: `?dpl=<deploymentId>` on asset URLs and the `x-deployment-id`
//! request header.

use serde_json::Value;

use crate::types::{CdnAssetManifestEntry, CdnPlan, CdnUploadTarget, NextConfig, OutputMode};

fn trim_trailing_slash(value: &str) -> &str {
    value.trim_end_matches('/')
}

/// Reads an optional `yoloCdn` object from `extra` (`provider`, `bucket`, `prefix`,
/// `publicBaseUrl`, optional `endpoint`, `region`).
fn upload_target(extra: &serde_json::Map<String, Value>) -> Option<CdnUploadTarget> {
    let object = extra.get("yoloCdn")?.as_object()?;
    let text = |key: &str| object.get(key).and_then(Value::as_str).map(str::to_owned);
    Some(CdnUploadTarget {
        provider: text("provider").unwrap_or_else(|| String::from("s3")),
        bucket: text("bucket")?,
        prefix: text("prefix").unwrap_or_else(|| String::from("_yolo/static/")),
        endpoint: text("endpoint"),
        region: text("region"),
        public_base_url: text("publicBaseUrl").unwrap_or_default(),
    })
}

/// Builds the CDN plan from a parsed `next.config`.
pub fn cdn_plan(cfg: &NextConfig) -> CdnPlan {
    let base_path = trim_trailing_slash(&cfg.base_path).to_owned();
    let output = match cfg.output.as_deref() {
        Some("standalone") => OutputMode::Standalone,
        Some("export") => OutputMode::Export,
        _ => OutputMode::Server,
    };
    CdnPlan {
        asset_prefix: trim_trailing_slash(&cfg.asset_prefix).to_owned(),
        static_route: format!("{base_path}/_yolo/static"),
        compat_static_routes: vec![format!("{base_path}/_next/static")],
        fonts_route: format!("{base_path}/_yolo/fonts"),
        public_dir: String::from("public"),
        immutable_cache_control: String::from("public, max-age=31536000, immutable"),
        public_cache_control: String::from("public, max-age=0"),
        dynamic_html_cache_control: String::from(
            "private, no-cache, no-store, max-age=0, must-revalidate",
        ),
        deployment_id: cfg.deployment_id.clone().filter(|id| !id.is_empty()),
        skew_query: String::from("dpl"),
        skew_header: String::from("x-deployment-id"),
        output,
        expire_time: cfg.expire_time.unwrap_or(31_536_000),
        compress: cfg.compress.unwrap_or(true),
        powered_by_header: cfg.powered_by_header.unwrap_or(true),
        generate_etags: cfg.generate_etags.unwrap_or(true),
        trailing_slash: cfg.trailing_slash,
        upload: upload_target(&cfg.extra),
        base_path,
    }
}

/// Returns the public URL of an output file (asset prefix + static route + skew query).
///
/// `path` is relative to the output directory (`static/chunks/app-abc123.js`).
pub fn public_url(plan: &CdnPlan, path: &str) -> String {
    let prefix = trim_trailing_slash(&plan.asset_prefix);
    let route = plan.static_route.trim_end_matches('/');
    let route = if route.starts_with('/') || route.is_empty() {
        route.to_owned()
    } else {
        format!("/{route}")
    };
    let relative = path.trim_start_matches('/');
    let url = if relative.is_empty() {
        format!("{prefix}{route}")
    } else {
        format!("{prefix}{route}/{relative}")
    };
    with_skew(plan, &url)
}

/// Appends the skew protection query (`?dpl=<deploymentId>`) when configured, never twice.
pub fn with_skew(plan: &CdnPlan, url: &str) -> String {
    let Some(id) = plan.deployment_id.as_deref().filter(|id| !id.is_empty()) else {
        return url.to_owned();
    };
    let (base, fragment) = url
        .split_once('#')
        .map_or((url, None), |(base, fragment)| (base, Some(fragment)));
    let key = plan.skew_query.as_str();
    let already = base.split_once('?').is_some_and(|(_, query)| {
        query
            .split('&')
            .any(|pair| pair.split('=').next() == Some(key))
    });
    if already {
        return url.to_owned();
    }
    let separator = if base.contains('?') { '&' } else { '?' };
    let mut out = format!("{base}{separator}{key}={id}");
    if let Some(fragment) = fragment {
        out.push('#');
        out.push_str(fragment);
    }
    out
}

/// Lower-case extension of a path (`js`, `woff2`), without the dot.
fn extension_of(path: &str) -> String {
    let name = path.rsplit(['/', '\\']).next().unwrap_or(path);
    let name = name.split(['?', '#']).next().unwrap_or(name);
    name.rsplit_once('.')
        .map(|(_, ext)| ext.to_ascii_lowercase())
        .unwrap_or_default()
}

/// `Cache-Control` value for a served path: hashed assets and anything under the static or
/// fonts routes are immutable, HTML is private/no-store, everything else is `public, max-age=0`.
pub fn cache_control_for(plan: &CdnPlan, path: &str) -> String {
    let pathname = path.split(['?', '#']).next().unwrap_or(path);
    let under_route = |route: &str| {
        !route.is_empty()
            && (pathname.starts_with(route) || pathname.contains(&format!("{route}/")))
    };
    if under_route(&plan.static_route)
        || under_route(&plan.fonts_route)
        || plan.compat_static_routes.iter().any(|r| under_route(r))
        || is_hashed_asset(pathname)
    {
        return plan.immutable_cache_control.clone();
    }
    if extension_of(pathname) == "html" {
        return plan.dynamic_html_cache_control.clone();
    }
    plan.public_cache_control.clone()
}

/// MIME type guessed from the extension.
pub fn content_type_for(path: &str) -> &'static str {
    match extension_of(path).as_str() {
        "js" | "mjs" | "cjs" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "html" | "htm" => "text/html; charset=utf-8",
        "json" => "application/json; charset=utf-8",
        "map" => "application/json",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "avif" => "image/avif",
        "ico" => "image/x-icon",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "ttf" => "font/ttf",
        "otf" => "font/otf",
        "wasm" => "application/wasm",
        "txt" => "text/plain; charset=utf-8",
        "xml" => "application/xml",
        "webmanifest" => "application/manifest+json",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        _ => "application/octet-stream",
    }
}

fn is_hex_hash(segment: &str) -> bool {
    (8..=16).contains(&segment.len()) && segment.chars().all(|c| c.is_ascii_hexdigit())
}

/// True when the file name carries a content hash (`name-<hash>.ext` or `name.<hash>.ext`,
/// 8 to 16 hex characters right before the extension).
pub fn is_hashed_asset(path: &str) -> bool {
    let pathname = path.split(['?', '#']).next().unwrap_or(path);
    let name = pathname.rsplit(['/', '\\']).next().unwrap_or(pathname);
    let Some((stem, _ext)) = name.rsplit_once('.') else {
        return false;
    };
    let candidate = stem.rsplit(['-', '.']).next().unwrap_or(stem);
    candidate.len() < stem.len() && is_hex_hash(candidate)
}

/// Builds the upload manifest for `(path, size)` pairs.
pub fn asset_manifest(plan: &CdnPlan, files: &[(String, u64)]) -> Vec<CdnAssetManifestEntry> {
    files
        .iter()
        .map(|(path, size)| CdnAssetManifestEntry {
            path: path.clone(),
            public_url: public_url(plan, path),
            content_type: content_type_for(path).to_owned(),
            cache_control: cache_control_for(plan, path),
            hashed: is_hashed_asset(path),
            size: *size,
        })
        .collect()
}

/// Emits a Bun script uploading hashed assets with `Bun.S3Client`.
pub fn upload_script_ts(plan: &CdnPlan) -> String {
    let upload = plan.upload.clone().unwrap_or_default();
    let bucket = if upload.bucket.is_empty() {
        String::from("process.env.CDN_BUCKET")
    } else {
        serde_json::to_string(&upload.bucket).unwrap_or_default()
    };
    let endpoint = upload.endpoint.as_deref().map_or_else(
        || String::from("process.env.CDN_ENDPOINT"),
        |e| serde_json::to_string(e).unwrap_or_default(),
    );
    let region = upload.region.as_deref().map_or_else(
        || String::from("process.env.CDN_REGION ?? \"auto\""),
        |r| serde_json::to_string(r).unwrap_or_default(),
    );
    let prefix = serde_json::to_string(&upload.prefix).unwrap_or_else(|_| String::from("\"\""));
    let static_route = serde_json::to_string(&plan.static_route).unwrap_or_default();
    let provider = if upload.provider.is_empty() {
        String::from("s3")
    } else {
        upload.provider.clone()
    };
    format!(
        r#"// Generated by yolo-next: upload the hashed build output to a {provider} bucket with Bun.S3Client.
// Usage: bun run upload-cdn.ts [dist-dir]   (reads <dist-dir>/cdn-manifest.json)
import {{ S3Client }} from "bun";

interface ManifestEntry {{
  path: string;
  publicUrl: string;
  contentType: string;
  cacheControl: string;
  hashed: boolean;
  size: number;
}}

const distDir = process.argv[2] ?? ".yolo";
const manifest = (await Bun.file(`${{distDir}}/cdn-manifest.json`).json()) as ManifestEntry[];
const prefix = {prefix};
const staticRoute = {static_route};
const client = new S3Client({{
  bucket: {bucket},
  endpoint: {endpoint},
  region: {region},
  accessKeyId: process.env.S3_ACCESS_KEY_ID ?? process.env.AWS_ACCESS_KEY_ID,
  secretAccessKey: process.env.S3_SECRET_ACCESS_KEY ?? process.env.AWS_SECRET_ACCESS_KEY,
}});

let uploaded = 0;
for (const entry of manifest) {{
  const key = `${{prefix}}${{entry.path.replace(/^\/+/, "")}}`;
  const file = Bun.file(`${{distDir}}/${{entry.path}}`);
  if (!(await file.exists())) {{
    console.warn(`skip missing ${{entry.path}}`);
    continue;
  }}
  // S3Client.write forwards `type` as Content-Type and `acl`; Cache-Control travels in the
  // request headers when the Bun release supports `headers` on write options (1.3+), otherwise
  // configure the bucket's default Cache-Control / CDN rules for `${{staticRoute}}` (immutable).
  await client.write(key, file, {{
    type: entry.contentType,
    acl: "public-read",
    headers: {{ "Cache-Control": entry.cacheControl }},
  }} as Parameters<S3Client["write"]>[2]);
  uploaded += 1;
}}
console.log(`uploaded ${{uploaded}}/${{manifest.length}} files under ${{prefix}} (served from ${{staticRoute}})`);
"#
    )
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use serde_json::json;

    fn plan_with(prefix: &str, base: &str, dpl: Option<&str>) -> CdnPlan {
        let cfg = NextConfig {
            asset_prefix: prefix.to_owned(),
            base_path: base.to_owned(),
            deployment_id: dpl.map(str::to_owned),
            ..NextConfig::default()
        };
        cdn_plan(&cfg)
    }

    #[test]
    fn plan_routes_follow_base_path() {
        let plan = plan_with("", "/docs/", None);
        assert_eq!(plan.base_path, "/docs");
        assert_eq!(plan.static_route, "/docs/_yolo/static");
        assert_eq!(
            plan.compat_static_routes,
            vec![String::from("/docs/_next/static")]
        );
        assert_eq!(plan.fonts_route, "/docs/_yolo/fonts");
        assert_eq!(plan.expire_time, 31_536_000);
        assert!(plan.compress && plan.powered_by_header && plan.generate_etags);
        assert_eq!(plan.output, OutputMode::Server);
        assert!(plan.upload.is_none());
    }

    #[test]
    fn output_and_upload_target() {
        let mut cfg = NextConfig {
            output: Some(String::from("standalone")),
            compress: Some(false),
            ..NextConfig::default()
        };
        cfg.extra.insert(
            String::from("yoloCdn"),
            json!({ "provider": "r2", "bucket": "assets", "prefix": "app/", "publicBaseUrl": "https://cdn.example.com", "endpoint": "https://acct.r2.cloudflarestorage.com" }),
        );
        let plan = cdn_plan(&cfg);
        assert_eq!(plan.output, OutputMode::Standalone);
        assert!(!plan.compress);
        let upload = plan.upload.unwrap();
        assert_eq!(upload.provider, "r2");
        assert_eq!(upload.bucket, "assets");
        assert_eq!(
            upload.endpoint.as_deref(),
            Some("https://acct.r2.cloudflarestorage.com")
        );
        let export =
            cdn_plan(&NextConfig { output: Some(String::from("export")), ..NextConfig::default() });
        assert_eq!(export.output, OutputMode::Export);
    }

    #[test]
    fn public_url_with_prefix_and_skew() {
        let plan = plan_with("https://cdn.example.com/", "", Some("dpl_123"));
        assert_eq!(
            public_url(&plan, "static/chunks/app-abc12345.js"),
            "https://cdn.example.com/_yolo/static/static/chunks/app-abc12345.js?dpl=dpl_123"
        );
        assert_eq!(
            public_url(&plan, "/static/x.css"),
            "https://cdn.example.com/_yolo/static/static/x.css?dpl=dpl_123"
        );
        let no_dpl = plan_with("", "/app", None);
        assert_eq!(
            public_url(&no_dpl, "chunk.js"),
            "/app/_yolo/static/chunk.js"
        );
    }

    #[test]
    fn skew_is_idempotent() {
        let plan = plan_with("", "", Some("v9"));
        assert_eq!(with_skew(&plan, "/a.js"), "/a.js?dpl=v9");
        assert_eq!(with_skew(&plan, "/a.js?x=1"), "/a.js?x=1&dpl=v9");
        assert_eq!(with_skew(&plan, "/a.js?dpl=v9"), "/a.js?dpl=v9");
        assert_eq!(with_skew(&plan, "/a.js?x=1&dpl=v9"), "/a.js?x=1&dpl=v9");
        assert_eq!(with_skew(&plan, "/a.js#frag"), "/a.js?dpl=v9#frag");
        let none = plan_with("", "", None);
        assert_eq!(with_skew(&none, "/a.js"), "/a.js");
    }

    #[test]
    fn cache_control_classification() {
        let plan = plan_with("", "", None);
        assert_eq!(
            cache_control_for(&plan, "static/chunks/app-abc12345.js"),
            plan.immutable_cache_control
        );
        assert_eq!(
            cache_control_for(&plan, "/_yolo/static/main.js"),
            plan.immutable_cache_control
        );
        assert_eq!(
            cache_control_for(&plan, "/_next/static/css/x.css"),
            plan.immutable_cache_control
        );
        assert_eq!(
            cache_control_for(&plan, "/_yolo/fonts/inter.woff2"),
            plan.immutable_cache_control
        );
        assert_eq!(
            cache_control_for(&plan, "index.html"),
            plan.dynamic_html_cache_control
        );
        assert_eq!(
            cache_control_for(&plan, "robots.txt"),
            plan.public_cache_control
        );
        assert_eq!(
            cache_control_for(&plan, "logo.png"),
            plan.public_cache_control
        );
        assert_eq!(
            cache_control_for(&plan, "chunk.1a2b3c4d.js"),
            plan.immutable_cache_control
        );
    }

    #[test]
    fn hashed_asset_detection() {
        assert!(is_hashed_asset("app-abc12345.js"));
        assert!(is_hashed_asset("a/b/chunk.deadbeefcafe.css"));
        assert!(is_hashed_asset("main-0123456789ABCDEF.js"));
        assert!(!is_hashed_asset("main.js"));
        assert!(
            !is_hashed_asset("abc12345.js"),
            "a bare hash with no stem is a plain name"
        );
        assert!(
            !is_hashed_asset("app-abc1234.js"),
            "7 hex chars is below the threshold"
        );
        assert!(!is_hashed_asset("app-xyz12345.js"));
        assert!(!is_hashed_asset("README"));
    }

    #[test]
    fn content_types() {
        assert_eq!(content_type_for("a.js"), "text/javascript; charset=utf-8");
        assert_eq!(content_type_for("a.mjs"), "text/javascript; charset=utf-8");
        assert_eq!(content_type_for("a.css"), "text/css; charset=utf-8");
        assert_eq!(content_type_for("a.HTML"), "text/html; charset=utf-8");
        assert_eq!(
            content_type_for("a.json"),
            "application/json; charset=utf-8"
        );
        assert_eq!(content_type_for("a.js.map"), "application/json");
        assert_eq!(content_type_for("a.svg"), "image/svg+xml");
        assert_eq!(content_type_for("a.ico"), "image/x-icon");
        assert_eq!(content_type_for("a.woff2"), "font/woff2");
        assert_eq!(content_type_for("a.wasm"), "application/wasm");
        assert_eq!(
            content_type_for("site.webmanifest"),
            "application/manifest+json"
        );
        assert_eq!(content_type_for("a.webm"), "video/webm");
        assert_eq!(content_type_for("a.bin"), "application/octet-stream");
        assert_eq!(content_type_for("noext"), "application/octet-stream");
    }

    #[test]
    fn manifest_and_upload_script() {
        let plan = plan_with("https://cdn.example.com", "", Some("d1"));
        let entries = asset_manifest(
            &plan,
            &[
                (String::from("static/app-abc12345.js"), 1200),
                (String::from("index.html"), 300),
            ],
        );
        assert_eq!(entries.len(), 2);
        assert!(entries[0].hashed);
        assert_eq!(
            entries[0].public_url,
            "https://cdn.example.com/_yolo/static/static/app-abc12345.js?dpl=d1"
        );
        assert_eq!(entries[0].content_type, "text/javascript; charset=utf-8");
        assert_eq!(entries[0].cache_control, plan.immutable_cache_control);
        assert!(!entries[1].hashed);
        assert_eq!(entries[1].size, 300);
        let script = upload_script_ts(&plan);
        assert!(script.contains("new S3Client("));
        assert!(script.contains("cdn-manifest.json"));
        assert!(script.contains("client.write("));
        assert!(
            script.lines().count() < 80,
            "upload script must stay under 80 lines"
        );
    }
}
