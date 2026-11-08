// SPDX-License-Identifier: Apache-2.0
//! `next/image`: usage extraction, `remotePatterns` matching, loaders, `srcset` algorithm,
//! optimizer policy and Manganis `asset!()` equivalents.
//!
//! Verified against Next.js 16.4.5 `shared/lib/get-img-props.ts` (`getWidths`,
//! `generateImgAttrs`, unoptimized rules), `shared/lib/image-loader.ts` (default loader URL),
//! `shared/lib/find-closest-quality.ts`, `shared/lib/match-remote-pattern.ts` /
//! `match-local-pattern.ts` (picomatch globs) and `server/image-optimizer.ts` (`Accept`
//! negotiation, `Cache-Control`).
//!
//! Hostname wildcards follow the documented semantics (`*` = one label, `**` = one or more
//! leading labels); upstream picomatch is slightly more permissive (`*` also spans dots), so a
//! pattern rejected here may still be accepted by Next.js, never the reverse for `**`.

use std::collections::BTreeSet;
use std::fmt::Write as _;

use oxc_allocator::Allocator;
#[allow(clippy::wildcard_imports)]
use oxc_ast::ast::*;
use oxc_parser::Parser;
use serde_json::{Value, json};

use crate::rsc::{
    Visit, literal_text, module_export_name, source_type_for, static_bool, static_number,
    static_string, unwrap_ts, walk_program,
};
use crate::types::{
    Diagnostic, ImageLoaderKind, ImageOptimizerPolicy, ImageUsage, ImageUsageReport, ImagesConfig,
    LocalPattern, RemotePattern, SrcSetCandidate,
};

/// Import sources that provide the `<Image>` component.
const IMAGE_IMPORT_SOURCES: [&str; 3] = ["next/image", "next/legacy/image", "next/future/image"];

/// Extensions treated as static image imports.
const IMAGE_EXTENSIONS: [&str; 10] = [
    "png", "jpg", "jpeg", "gif", "webp", "avif", "svg", "ico", "bmp", "tiff",
];

// ============================================================================
// Extraction
// ============================================================================

fn is_remote_src(src: &str) -> bool {
    let lower = src.to_ascii_lowercase();
    lower.starts_with("http://") || lower.starts_with("https://") || lower.starts_with("//")
}

fn has_image_extension(specifier: &str) -> bool {
    let path = specifier.split(['?', '#']).next().unwrap_or(specifier);
    path.rsplit_once('.')
        .is_some_and(|(_, ext)| IMAGE_EXTENSIONS.contains(&ext.to_ascii_lowercase().as_str()))
}

fn attr_expression<'a, 'b>(value: Option<&'b JSXAttributeValue<'a>>) -> Option<&'b Expression<'a>> {
    match value? {
        JSXAttributeValue::ExpressionContainer(container) => container.expression.as_expression(),
        _ => None,
    }
}

fn attr_string(value: Option<&JSXAttributeValue<'_>>) -> Option<String> {
    match value? {
        JSXAttributeValue::StringLiteral(lit) => Some(literal_text(lit)),
        JSXAttributeValue::ExpressionContainer(container) => {
            container.expression.as_expression().and_then(static_string)
        }
        _ => None,
    }
}

/// `<Image fill />`, `fill={true}`, `fill="true"`; `fill={false}` / `fill="false"` are false.
fn attr_bool(value: Option<&JSXAttributeValue<'_>>) -> bool {
    match value {
        None | Some(JSXAttributeValue::Element(_) | JSXAttributeValue::Fragment(_)) => true,
        Some(JSXAttributeValue::StringLiteral(lit)) => literal_text(lit) != "false",
        Some(JSXAttributeValue::ExpressionContainer(container)) => container
            .expression
            .as_expression()
            .is_none_or(|expr| static_bool(expr).unwrap_or(true)),
    }
}

fn attr_u32(value: Option<&JSXAttributeValue<'_>>) -> Option<u32> {
    let number = match value? {
        JSXAttributeValue::StringLiteral(lit) => literal_text(lit).trim().parse::<f64>().ok()?,
        JSXAttributeValue::ExpressionContainer(container) => {
            static_number(container.expression.as_expression()?)?
        }
        _ => return None,
    };
    (number >= 0.0 && number.fract() == 0.0 && number <= f64::from(u32::MAX))
        .then_some(number as u32)
}

struct ImageScan<'f> {
    file: &'f str,
    /// (local binding, import source)
    components: Vec<(String, String)>,
    /// static image import binding -> module specifier
    static_imports: Vec<(String, String)>,
    usages: Vec<ImageUsage>,
}

impl ImageScan<'_> {
    fn apply_prop(
        &self,
        usage: &mut ImageUsage,
        name: &str,
        value: Option<&JSXAttributeValue<'_>>,
    ) {
        match name {
            "src" => {
                if let Some(text) = attr_string(value) {
                    usage.src = Some(text);
                } else if let Some(Expression::Identifier(id)) =
                    attr_expression(value).map(unwrap_ts)
                {
                    let binding = id.name.as_str();
                    usage.src_import = Some(binding.to_owned());
                    usage.src = self
                        .static_imports
                        .iter()
                        .find(|(local, _)| local == binding)
                        .map(|(_, source)| source.clone());
                }
            }
            "alt" => usage.alt = attr_string(value),
            "width" => usage.width = attr_u32(value),
            "height" => usage.height = attr_u32(value),
            "fill" => usage.fill = attr_bool(value),
            "sizes" => usage.sizes = attr_string(value),
            "quality" => {
                usage.quality = attr_u32(value).and_then(|q| u8::try_from(q).ok());
            }
            "priority" | "preload" => usage.priority |= attr_bool(value),
            "placeholder" => usage.placeholder = attr_string(value),
            "loader" => usage.custom_loader = true,
            "unoptimized" => usage.unoptimized = attr_bool(value),
            "loading" => usage.loading = attr_string(value),
            "fetchPriority" => usage.fetch_priority = attr_string(value),
            _ => {}
        }
    }
}

impl Visit for ImageScan<'_> {
    fn jsx_element(&mut self, element: &JSXElement<'_>) {
        let name = match &element.opening_element.name {
            JSXElementName::Identifier(id) => id.name.as_str(),
            JSXElementName::IdentifierReference(id) => id.name.as_str(),
            _ => return,
        };
        let Some(import_source) = self
            .components
            .iter()
            .find(|(local, _)| local == name)
            .map(|(_, source)| source.clone())
        else {
            return;
        };
        let mut usage =
            ImageUsage { file: self.file.to_owned(), import_source, ..ImageUsage::default() };
        for attr in &element.opening_element.attributes {
            let JSXAttributeItem::Attribute(attribute) = attr else {
                continue;
            };
            let JSXAttributeName::Identifier(attr_name) = &attribute.name else {
                continue;
            };
            self.apply_prop(
                &mut usage,
                attr_name.name.as_str(),
                attribute.value.as_ref(),
            );
        }
        usage.remote = usage.src.as_deref().is_some_and(is_remote_src);
        self.usages.push(usage);
    }
}

/// Extracts `<Image ... />` usages from a module.
///
/// The component is the default import of `next/image` / `next/legacy/image` under any local
/// name (also `import { default as Img }`); static image imports (`import hero from
/// "@/public/hero.png"`) are resolved so `src={hero}` yields `src_import = "hero"` and `src` =
/// the import specifier. Every JSX element in the module is visited (returns, arrow bodies,
/// conditionals, `.map()` callbacks, nested children).
pub fn extract_image_usages(source: &str, file: &str) -> Vec<ImageUsage> {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, source_type_for(file)).parse();
    let program = &parsed.program;

    let mut scan =
        ImageScan { file, components: Vec::new(), static_imports: Vec::new(), usages: Vec::new() };
    for stmt in &program.body {
        let Statement::ImportDeclaration(import) = stmt else {
            continue;
        };
        let Some(specifiers) = &import.specifiers else {
            continue;
        };
        let source_text = literal_text(&import.source);
        if IMAGE_IMPORT_SOURCES.contains(&source_text.as_str()) {
            for spec in specifiers {
                match spec {
                    ImportDeclarationSpecifier::ImportDefaultSpecifier(default) => {
                        scan.components
                            .push((default.local.name.as_str().to_owned(), source_text.clone()));
                    }
                    ImportDeclarationSpecifier::ImportSpecifier(named)
                        if module_export_name(&named.imported) == "default" =>
                    {
                        scan.components
                            .push((named.local.name.as_str().to_owned(), source_text.clone()));
                    }
                    _ => {}
                }
            }
        } else if has_image_extension(&source_text) {
            for spec in specifiers {
                if let ImportDeclarationSpecifier::ImportDefaultSpecifier(default) = spec {
                    scan.static_imports
                        .push((default.local.name.as_str().to_owned(), source_text.clone()));
                }
            }
        }
    }
    if scan.components.is_empty() {
        return Vec::new();
    }
    walk_program(program, &mut scan);
    scan.usages
}

// ============================================================================
// Pattern matching
// ============================================================================

struct ParsedUrl {
    scheme: String,
    host: String,
    port: String,
    path: String,
    search: String,
}

/// Minimal WHATWG-like parse: scheme, lowercase host, port normalised like `URL.port` (empty for
/// the default port), pathname (`/` when absent) and search (`""` for a bare or missing `?`).
fn parse_url(url: &str) -> Option<ParsedUrl> {
    let (scheme, rest) = if let Some(index) = url.find("://") {
        (url[..index].to_ascii_lowercase(), &url[index + 3..])
    } else {
        (String::new(), url.strip_prefix("//")?)
    };
    let rest = rest.split('#').next().unwrap_or("");
    let authority_end = rest.find(['/', '?']).unwrap_or(rest.len());
    let authority = &rest[..authority_end];
    let path_and_query = &rest[authority_end..];
    let (path, search) = match path_and_query.find('?') {
        Some(index) => (&path_and_query[..index], &path_and_query[index..]),
        None => (path_and_query, ""),
    };
    let authority = authority.rsplit('@').next().unwrap_or(authority);
    let (host, port) = if authority.starts_with('[') {
        match authority.find(']') {
            Some(end) => (
                &authority[..=end],
                authority[end + 1..].strip_prefix(':').unwrap_or(""),
            ),
            None => (authority, ""),
        }
    } else if let Some(index) = authority.rfind(':') {
        (&authority[..index], &authority[index + 1..])
    } else {
        (authority, "")
    };
    if host.is_empty() {
        return None;
    }
    let default_port = match scheme.as_str() {
        "http" | "ws" => "80",
        "https" | "wss" => "443",
        _ => "",
    };
    let port = if port == default_port { "" } else { port };
    Some(ParsedUrl {
        scheme,
        host: host.to_ascii_lowercase(),
        port: port.to_owned(),
        path: if path.is_empty() {
            String::from("/")
        } else {
            path.to_owned()
        },
        search: if search == "?" {
            String::new()
        } else {
            search.to_owned()
        },
    })
}

/// Wildcard match within one segment: `*` = any run of characters, `?` = one character.
fn segment_matches(pattern: &str, text: &str) -> bool {
    fn go(p: &[char], t: &[char]) -> bool {
        match p.split_first() {
            None => t.is_empty(),
            Some(('*', rest)) => (0..=t.len()).any(|n| go(rest, &t[n..])),
            Some(('?', rest)) => t.split_first().is_some_and(|(_, tr)| go(rest, tr)),
            Some((c, rest)) => t
                .split_first()
                .is_some_and(|(tc, tr)| tc.eq_ignore_ascii_case(c) && go(rest, tr)),
        }
    }
    let p: Vec<char> = pattern.chars().collect();
    let t: Vec<char> = text.chars().collect();
    go(&p, &t)
}

/// Segment-list glob: `**` consumes `min_star_star..` segments, `*` exactly one.
fn segments_match(pattern: &[&str], text: &[&str], min_star_star: usize) -> bool {
    match pattern.split_first() {
        None => text.is_empty(),
        Some((&"**", rest)) => {
            (min_star_star..=text.len()).any(|n| segments_match(rest, &text[n..], min_star_star))
        }
        Some((first, rest)) => text.split_first().is_some_and(|(head, tail)| {
            segment_matches(first, head) && segments_match(rest, tail, min_star_star)
        }),
    }
}

fn hostname_matches(pattern: &str, host: &str) -> bool {
    let pattern = pattern.to_ascii_lowercase();
    let host = host.to_ascii_lowercase();
    let p: Vec<&str> = pattern.split('.').collect();
    let h: Vec<&str> = host.split('.').collect();
    segments_match(&p, &h, 1)
}

fn path_segments(path: &str) -> Vec<&str> {
    let trimmed = path.trim_matches('/');
    if trimmed.is_empty() {
        Vec::new()
    } else {
        trimmed.split('/').collect()
    }
}

/// picomatch-like pathname glob (`{ dot: true }`): `**` any number of segments, `*` one segment.
fn pathname_matches(pattern: &str, path: &str) -> bool {
    let p = path_segments(pattern);
    let h = path_segments(path);
    segments_match(&p, &h, 0)
}

fn normalize_search(search: &str) -> String {
    if search.is_empty() || search.starts_with('?') {
        search.to_owned()
    } else {
        format!("?{search}")
    }
}

/// Matches an absolute URL against a remote pattern (`*` one label, `**` any labels).
///
/// `protocol` (when set) must equal the URL scheme, `port` (when set) must equal the normalised
/// URL port (`""` = default), the hostname glob is case-insensitive (`**.example.com` needs at
/// least one leading label), `search` (when set) must equal the query exactly (`""` = none) and
/// `pathname` (default `**`) is a picomatch-style glob.
pub fn remote_pattern_matches(pattern: &RemotePattern, url: &str) -> bool {
    let Some(parsed) = parse_url(url) else {
        return false;
    };
    if let Some(protocol) = &pattern.protocol
        && !protocol
            .trim_end_matches(':')
            .eq_ignore_ascii_case(&parsed.scheme)
    {
        return false;
    }
    if let Some(port) = &pattern.port
        && port.trim() != parsed.port
    {
        return false;
    }
    if !hostname_matches(&pattern.hostname, &parsed.host) {
        return false;
    }
    if let Some(search) = &pattern.search
        && normalize_search(search) != parsed.search
    {
        return false;
    }
    pathname_matches(pattern.pathname.as_deref().unwrap_or("**"), &parsed.path)
}

/// Matches a local path + query against a local pattern.
pub fn local_pattern_matches(pattern: &LocalPattern, path_and_query: &str) -> bool {
    let (path, search) = match path_and_query.find('?') {
        Some(index) => (&path_and_query[..index], &path_and_query[index..]),
        None => (path_and_query, ""),
    };
    let search = if search == "?" { "" } else { search };
    if let Some(wanted) = &pattern.search
        && normalize_search(wanted) != search
    {
        return false;
    }
    pathname_matches(pattern.pathname.as_deref().unwrap_or("**"), path)
}

// ============================================================================
// Loaders and policy
// ============================================================================

/// Loader kind from config (`loader` + `loaderFile`).
pub fn loader_kind(cfg: &ImagesConfig) -> ImageLoaderKind {
    if !cfg.loader_file.trim().is_empty() {
        return ImageLoaderKind::Custom;
    }
    match cfg.loader.trim().to_ascii_lowercase().as_str() {
        "imgix" => ImageLoaderKind::Imgix,
        "cloudinary" => ImageLoaderKind::Cloudinary,
        "akamai" => ImageLoaderKind::Akamai,
        "custom" => ImageLoaderKind::Custom,
        "cloudflare" => ImageLoaderKind::Cloudflare,
        "fastly" => ImageLoaderKind::Fastly,
        "supabase" => ImageLoaderKind::Supabase,
        "cloudfront" => ImageLoaderKind::Cloudfront,
        _ if cfg.unoptimized => ImageLoaderKind::Unoptimized,
        _ => ImageLoaderKind::Default,
    }
}

/// Builds the optimizer policy (`/_yolo/image` + `/_next/image` compat).
pub fn optimizer_policy(cfg: &ImagesConfig, base_path: &str) -> ImageOptimizerPolicy {
    let base = base_path.trim_end_matches('/');
    let route = format!("{base}/_yolo/image");
    let mut compat_routes = vec![format!("{base}/_next/image")];
    let configured = cfg.path.trim();
    if !configured.is_empty() && configured != "/_next/image" {
        let custom = format!("{base}/{}", configured.trim_start_matches('/'));
        if !compat_routes.contains(&custom) && custom != route {
            compat_routes.push(custom);
        }
    }
    let loader = loader_kind(cfg);
    ImageOptimizerPolicy {
        route,
        compat_routes,
        loader,
        loader_file: (!cfg.loader_file.trim().is_empty()).then(|| cfg.loader_file.clone()),
        formats: cfg.formats.clone(),
        device_sizes: cfg.device_sizes.clone(),
        image_sizes: cfg.image_sizes.clone(),
        qualities: cfg.qualities.clone(),
        minimum_cache_ttl: cfg.minimum_cache_ttl,
        cache_control: String::from("public, max-age={max_age}, must-revalidate"),
        allow_svg: cfg.dangerously_allow_svg,
        allow_local_ip: cfg.dangerously_allow_local_ip,
        content_security_policy: cfg.content_security_policy.clone(),
        content_disposition_type: cfg.content_disposition_type.clone(),
        remote_patterns: cfg.remote_patterns.clone(),
        local_patterns: cfg.local_patterns.clone(),
        maximum_redirects: cfg.maximum_redirects,
        maximum_response_body: cfg.maximum_response_body,
        unoptimized: cfg.unoptimized,
    }
}

/// `encodeURIComponent`: keeps `A-Z a-z 0-9 - _ . ! ~ * ' ( )`, percent-encodes everything else.
fn encode_uri_component(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z'
            | b'a'..=b'z'
            | b'0'..=b'9'
            | b'-'
            | b'_'
            | b'.'
            | b'!'
            | b'~'
            | b'*'
            | b'\''
            | b'('
            | b')' => out.push(char::from(byte)),
            _ => {
                let _ = write!(out, "%{byte:02X}");
            }
        }
    }
    out
}

/// Appends `query` with `?` or `&` depending on whether `src` already has a query string.
fn with_query(src: &str, query: &str) -> String {
    let separator = if src.contains('?') { '&' } else { '?' };
    format!("{src}{separator}{query}")
}

/// Splits an absolute URL into `(origin, path-and-query)`; relative sources yield `("", src)`.
fn split_origin(src: &str) -> (&str, &str) {
    if let Some(scheme_end) = src.find("://") {
        let after = &src[scheme_end + 3..];
        match after.find('/') {
            Some(slash) => src.split_at(scheme_end + 3 + slash),
            None => (src, ""),
        }
    } else {
        ("", src)
    }
}

/// Builds the URL of one candidate for a loader kind.
///
/// - `Default` / `Custom`: `{route}?url={encodeURIComponent(src)}&w={width}&q={quality}` (the
///   custom `loaderFile` is applied by the runtime).
/// - `Cloudflare`: `/cdn-cgi/image/width=,quality=,format=auto/{src without leading slash}`.
/// - `Cloudinary`: `f_auto,c_limit,w_,q_` inserted after the origin (or as the first path segment).
/// - `Imgix`: `?auto=format&fit=max&w=&q=`; `Akamai`: `?imwidth=`; `Fastly`:
///   `?auto=webp&width=&quality=`; `Supabase`: `?width=&quality=`; `Cloudfront`:
///   `?format=auto&width=&quality=`; `Unoptimized`: `src` unchanged.
pub fn loader_url(
    kind: ImageLoaderKind,
    policy: &ImageOptimizerPolicy,
    src: &str,
    width: u32,
    quality: u8,
) -> String {
    match kind {
        ImageLoaderKind::Default | ImageLoaderKind::Custom => format!(
            "{}?url={}&w={width}&q={quality}",
            policy.route,
            encode_uri_component(src)
        ),
        ImageLoaderKind::Cloudflare => format!(
            "/cdn-cgi/image/width={width},quality={quality},format=auto/{}",
            if is_remote_src(src) {
                src
            } else {
                src.trim_start_matches('/')
            }
        ),
        ImageLoaderKind::Cloudinary => {
            let params = format!("f_auto,c_limit,w_{width},q_{quality}");
            let (origin, path) = split_origin(src);
            format!("{origin}/{params}/{}", path.trim_start_matches('/'))
        }
        ImageLoaderKind::Imgix => {
            with_query(src, &format!("auto=format&fit=max&w={width}&q={quality}"))
        }
        ImageLoaderKind::Akamai => with_query(src, &format!("imwidth={width}")),
        ImageLoaderKind::Fastly => {
            with_query(src, &format!("auto=webp&width={width}&quality={quality}"))
        }
        ImageLoaderKind::Supabase => with_query(src, &format!("width={width}&quality={quality}")),
        ImageLoaderKind::Cloudfront => {
            with_query(src, &format!("format=auto&width={width}&quality={quality}"))
        }
        ImageLoaderKind::Unoptimized => src.to_owned(),
    }
}

/// Coerces a requested quality to the nearest allowed value (default 75).
///
/// Mirrors `findClosestQuality`: a missing or zero quality becomes 75, then the nearest entry
/// of `qualities` wins (ties keep the earlier entry, i.e. the lower one for a sorted list); an
/// empty `qualities` list returns the request unchanged.
pub fn coerce_quality(policy: &ImageOptimizerPolicy, quality: Option<u8>) -> u8 {
    let wanted = quality.filter(|q| *q > 0).unwrap_or(75);
    let Some(&first) = policy.qualities.first() else {
        return wanted;
    };
    policy.qualities.iter().fold(first, |best, &candidate| {
        let distance = |q: u8| (i16::from(q) - i16::from(wanted)).abs();
        if distance(candidate) < distance(best) {
            candidate
        } else {
            best
        }
    })
}

/// `(^|\s)(1?\d?\d)vw` tokens of a `sizes` attribute.
fn vw_percentages(sizes: &str) -> Vec<u32> {
    sizes
        .split_whitespace()
        .filter_map(|token| {
            let digits: String = token.chars().take_while(char::is_ascii_digit).collect();
            if digits.is_empty()
                || digits.len() > 3
                || (digits.len() == 3 && !digits.starts_with('1'))
            {
                return None;
            }
            token[digits.len()..]
                .starts_with("vw")
                .then(|| digits.parse().ok())
                .flatten()
        })
        .collect()
}

/// `getWidths`: candidate widths and descriptor kind (`'w'` or `'x'`).
fn candidate_widths(
    policy: &ImageOptimizerPolicy,
    width: Option<u32>,
    sizes: Option<&str>,
) -> (Vec<u32>, char) {
    let mut device_sizes = policy.device_sizes.clone();
    device_sizes.sort_unstable();
    device_sizes.dedup();
    let mut all_sizes = device_sizes.clone();
    all_sizes.extend(policy.image_sizes.iter().copied());
    all_sizes.sort_unstable();
    all_sizes.dedup();

    if let Some(sizes) = sizes {
        let percentages = vw_percentages(sizes);
        if let Some(smallest) = percentages.iter().min() {
            let smallest_device = device_sizes.first().copied().unwrap_or(0);
            let threshold = f64::from(smallest_device) * f64::from(*smallest) / 100.0;
            return (
                all_sizes
                    .into_iter()
                    .filter(|&s| f64::from(s) >= threshold)
                    .collect(),
                'w',
            );
        }
        return (all_sizes, 'w');
    }
    let Some(width) = width else {
        return (device_sizes, 'w');
    };
    let pick = |target: u32| {
        all_sizes
            .iter()
            .copied()
            .find(|&p| p >= target)
            .or_else(|| all_sizes.last().copied())
    };
    let mut widths = Vec::new();
    for target in [width, width.saturating_mul(2)] {
        if let Some(picked) = pick(target)
            && !widths.contains(&picked)
        {
            widths.push(picked);
        }
    }
    (widths, 'x')
}

fn is_svg_src(src: &str) -> bool {
    let path = src.split('?').next().unwrap_or(src);
    path.rsplit_once('.')
        .is_some_and(|(_, ext)| ext.eq_ignore_ascii_case("svg"))
}

fn is_unoptimized(
    usage: &ImageUsage,
    policy: &ImageOptimizerPolicy,
    kind: ImageLoaderKind,
    src: &str,
) -> bool {
    let lower = src.to_ascii_lowercase();
    usage.unoptimized
        || policy.unoptimized
        || kind == ImageLoaderKind::Unoptimized
        || lower.starts_with("data:")
        || lower.starts_with("blob:")
        || (kind == ImageLoaderKind::Default && !policy.allow_svg && is_svg_src(src))
}

/// next/image `srcset` algorithm (`w` descriptors with `sizes`/`fill`, else `1x`/`2x`).
///
/// `generateImgAttrs` + `getWidths`: with `sizes`, every `all_sizes` entry >= `device_sizes[0] *
/// smallest vw / 100` (all of them without a `vw` token) as `{w}w`; with `fill` (or no width)
/// and no `sizes`, `device_sizes` as `{w}w`; with a fixed `width`, the smallest `all_sizes`
/// entry >= `width` and >= `2 * width` as `1x` / `2x`. Unoptimized usages (prop, config,
/// `Unoptimized` loader, `data:` / `blob:` sources, `.svg` without `dangerouslyAllowSVG`) return
/// the bare `src` with an empty descriptor; a dynamic `src` returns no candidates.
pub fn srcset(
    usage: &ImageUsage,
    policy: &ImageOptimizerPolicy,
    kind: ImageLoaderKind,
) -> Vec<SrcSetCandidate> {
    let Some(src) = usage.src.as_deref() else {
        return Vec::new();
    };
    if is_unoptimized(usage, policy, kind, src) {
        return vec![SrcSetCandidate { url: src.to_owned(), descriptor: String::new() }];
    }
    let quality = coerce_quality(policy, usage.quality);
    let width = if usage.fill { None } else { usage.width };
    let (widths, descriptor_kind) = candidate_widths(policy, width, usage.sizes.as_deref());
    widths
        .iter()
        .enumerate()
        .map(|(index, &w)| SrcSetCandidate {
            url: loader_url(kind, policy, src, w, quality),
            descriptor: if descriptor_kind == 'w' {
                format!("{w}w")
            } else {
                format!("{}x", index + 1)
            },
        })
        .collect()
}

/// Negotiates the output format from an `Accept` header.
///
/// Returns the first entry of `formats` (policy order) whose MIME type is listed explicitly in
/// `Accept`; `*/*` does not count, like `getSupportedMimeType`.
pub fn negotiate_format<'a>(accept: &str, formats: &'a [String]) -> Option<&'a str> {
    let accepted: Vec<String> = accept
        .split(',')
        .map(|part| {
            part.split(';')
                .next()
                .unwrap_or("")
                .trim()
                .to_ascii_lowercase()
        })
        .filter(|part| !part.is_empty())
        .collect();
    formats
        .iter()
        .find(|format| accepted.iter().any(|a| a.eq_ignore_ascii_case(format)))
        .map(String::as_str)
}

fn manganis_format(mime: &str) -> Option<&'static str> {
    match mime.to_ascii_lowercase().as_str() {
        "image/avif" => Some("Avif"),
        "image/webp" => Some("Webp"),
        "image/jpeg" | "image/jpg" => Some("Jpg"),
        "image/png" => Some("Png"),
        _ => None,
    }
}

/// Manganis asset paths are crate-root relative: `@/public/hero.png` -> `/public/hero.png`.
fn manganis_path(src: &str) -> String {
    if let Some(rest) = src.strip_prefix("@/").or_else(|| src.strip_prefix("~/")) {
        return format!("/{rest}");
    }
    src.to_owned()
}

/// Manganis `asset!()` expression equivalent of a usage.
///
/// Static / local sources become `asset!("<path>", AssetOptions::image().with_format(...)
/// .with_size(ImageSize::Manual { width, height }).with_preload(true))` (format = first policy
/// format, size only when both dimensions are known, preload for `priority` / `preload`);
/// remote sources stay plain strings served through the image proxy.
pub fn manganis_asset_expr(usage: &ImageUsage, policy: &ImageOptimizerPolicy) -> String {
    let Some(src) = usage.src.as_deref() else {
        return String::from("String::new() /* dynamic src: resolve at runtime */");
    };
    if usage.remote {
        return format!("{src:?}.to_string() /* remote: served through the image proxy */");
    }
    let mut options = String::from("AssetOptions::image()");
    if let Some(format) = policy.formats.iter().find_map(|f| manganis_format(f)) {
        let _ = write!(options, ".with_format(ImageFormat::{format})");
    }
    if let (Some(width), Some(height)) = (usage.width, usage.height) {
        let _ = write!(
            options,
            ".with_size(ImageSize::Manual {{ width: {width}, height: {height} }})"
        );
    }
    if usage.priority {
        options.push_str(".with_preload(true)");
    }
    format!("asset!({:?}, {options})", manganis_path(src))
}

/// Aggregates usages into a report (remote hosts, unmatched patterns).
///
/// Diagnostics: `next-image-remote-unmatched` (warning, per remote `src` covered by neither
/// `remotePatterns` nor `domains`), `next-image-missing-dimensions` (warning, no `fill`, no
/// `width`/`height`, not a static import), `next-image-priority-deprecated` (info, once per file
/// using `priority`/`preload`; Next 16 names the prop `preload`) and a `next-image-usage-summary`
/// info line.
pub fn image_usage_report(usages: Vec<ImageUsage>, cfg: &ImagesConfig) -> ImageUsageReport {
    let mut remote_hosts: BTreeSet<String> = BTreeSet::new();
    let mut unmatched_remote: Vec<String> = Vec::new();
    let mut diagnostics = Vec::new();
    let mut priority_files: BTreeSet<&str> = BTreeSet::new();
    let mut remote_count = 0usize;
    let mut static_count = 0usize;

    for usage in &usages {
        if let (true, Some(src)) = (usage.remote, usage.src.as_deref()) {
            remote_count += 1;
            if let Some(parsed) = parse_url(src) {
                remote_hosts.insert(parsed.host.clone());
                let allowed = cfg
                    .domains
                    .iter()
                    .any(|domain| domain.eq_ignore_ascii_case(&parsed.host))
                    || cfg
                        .remote_patterns
                        .iter()
                        .any(|pattern| remote_pattern_matches(pattern, src));
                if !allowed && !unmatched_remote.iter().any(|u| u == src) {
                    unmatched_remote.push(src.to_owned());
                    diagnostics.push(
                        Diagnostic::warning(
                            "next-image-remote-unmatched",
                            format!("remote image `{src}` matches no `images.remotePatterns` entry"),
                        )
                        .with_file(usage.file.clone())
                        .with_hint(format!(
                            "add {{ protocol: \"{}\", hostname: \"{}\" }} to images.remotePatterns or mark the image `unoptimized`",
                            if parsed.scheme.is_empty() { "https" } else { parsed.scheme.as_str() },
                            parsed.host
                        )),
                    );
                }
            }
        }
        if usage.src_import.is_some() {
            static_count += 1;
        }
        if !usage.fill
            && usage.width.is_none()
            && usage.height.is_none()
            && usage.src_import.is_none()
        {
            diagnostics.push(
                Diagnostic::warning(
                    "next-image-missing-dimensions",
                    format!(
                        "`<Image src={}>` has neither `width`/`height` nor `fill`",
                        usage.src.as_deref().unwrap_or("{dynamic}")
                    ),
                )
                .with_file(usage.file.clone())
                .with_hint("add `width` and `height`, use `fill` with a sized parent, or import the image statically"),
            );
        }
        if usage.priority {
            priority_files.insert(usage.file.as_str());
        }
    }
    for file in priority_files {
        diagnostics.push(
            Diagnostic::info(
                "next-image-priority-deprecated",
                format!("`{file}` sets `priority`/`preload` on <Image>; Next.js 16 names the prop `preload` and emits a `<link rel=preload>`"),
            )
            .with_file(file),
        );
    }
    diagnostics.insert(
        0,
        Diagnostic::info(
            "next-image-usage-summary",
            format!(
                "{} <Image> usages: {remote_count} remote across {} hosts, {static_count} static imports, {} unmatched remote",
                usages.len(),
                remote_hosts.len(),
                unmatched_remote.len()
            ),
        ),
    );

    ImageUsageReport {
        usages,
        remote_hosts: remote_hosts.into_iter().collect(),
        unmatched_remote,
        diagnostics,
    }
}

/// `.yolo/images.json` manifest.
///
/// Shaped like Next.js `images-manifest.json` (`version`, `images.{deviceSizes, imageSizes,
/// path, loader, loaderFile, domains, disableStaticImages, minimumCacheTTL, formats,
/// dangerouslyAllowSVG, contentSecurityPolicy, contentDispositionType, remotePatterns,
/// localPatterns, unoptimized, qualities}`) plus `compatRoutes`; patterns keep their globs.
pub fn images_manifest_json(policy: &ImageOptimizerPolicy) -> Value {
    let remote_patterns: Vec<Value> = policy
        .remote_patterns
        .iter()
        .map(|pattern| {
            json!({
                "protocol": pattern.protocol,
                "hostname": pattern.hostname,
                "port": pattern.port,
                "pathname": pattern.pathname.as_deref().unwrap_or("**"),
                "search": pattern.search,
            })
        })
        .collect();
    let local_patterns: Value = policy
        .local_patterns
        .as_ref()
        .map_or(Value::Null, |patterns| {
            Value::Array(
                patterns
                    .iter()
                    .map(|pattern| {
                        json!({
                            "pathname": pattern.pathname.as_deref().unwrap_or("**"),
                            "search": pattern.search,
                        })
                    })
                    .collect(),
            )
        });
    json!({
        "version": 1,
        "images": {
            "deviceSizes": policy.device_sizes,
            "imageSizes": policy.image_sizes,
            "path": policy.route,
            "loader": policy.loader,
            "loaderFile": policy.loader_file.clone().unwrap_or_default(),
            "domains": [],
            "disableStaticImages": false,
            "minimumCacheTTL": policy.minimum_cache_ttl,
            "formats": policy.formats,
            "dangerouslyAllowSVG": policy.allow_svg,
            "contentSecurityPolicy": policy.content_security_policy,
            "contentDispositionType": policy.content_disposition_type,
            "remotePatterns": remote_patterns,
            "localPatterns": local_patterns,
            "unoptimized": policy.unoptimized,
            "qualities": policy.qualities,
            "compatRoutes": policy.compat_routes,
        }
    })
}

#[cfg(test)]
#[allow(clippy::assert_is_empty)] // `assert!(x.is_empty())` reads better than typed empty arrays
mod tests {
    use std::path::Path;

    use super::*;

    const PAGE_TSX: &str = r#"import Image from "next/image";
import Link from "next/link";
import { getPosts } from "./lib/data";
import hero from "@/public/hero.png";

export default async function HomePage() {
  const posts = await getPosts();
  return (
    <main>
      <Image src={hero} alt="Hero" priority sizes="(max-width: 768px) 100vw, 50vw" placeholder="blur" />
      <Image src="https://images.unsplash.com/photo-1" alt="Remote" width={800} height={600} quality={90} />
      <Image src="https://untrusted.example.net/x.png" alt="Unmatched" width={100} height={100} />
      <ul>
        {posts.map((p) => (
          <li key={p.slug}>
            <Link href={`/blog/${p.slug}`}>{p.title}</Link>
          </li>
        ))}
      </ul>
    </main>
  );
}
"#;

    fn fixture_config() -> ImagesConfig {
        ImagesConfig {
            formats: vec![String::from("image/avif"), String::from("image/webp")],
            qualities: vec![50, 75, 90],
            minimum_cache_ttl: 86_400,
            remote_patterns: vec![
                RemotePattern {
                    protocol: Some(String::from("https")),
                    hostname: String::from("images.unsplash.com"),
                    port: None,
                    pathname: Some(String::from("/**")),
                    search: None,
                },
                RemotePattern {
                    protocol: Some(String::from("https")),
                    hostname: String::from("**.cloudfront.net"),
                    port: None,
                    pathname: None,
                    search: None,
                },
                RemotePattern {
                    protocol: Some(String::from("https")),
                    hostname: String::from("cdn.example.com"),
                    port: Some(String::new()),
                    pathname: Some(String::from("/media/**")),
                    search: Some(String::new()),
                },
            ],
            local_patterns: Some(vec![LocalPattern {
                pathname: Some(String::from("/assets/images/**")),
                search: Some(String::new()),
            }]),
            ..ImagesConfig::default()
        }
    }

    fn policy() -> ImageOptimizerPolicy {
        optimizer_policy(&fixture_config(), "")
    }

    fn pattern(hostname: &str) -> RemotePattern {
        RemotePattern { hostname: hostname.to_owned(), ..RemotePattern::default() }
    }

    fn assert_fixture_usages(usages: &[ImageUsage]) {
        assert_eq!(usages.len(), 3, "{usages:?}");
        let hero = &usages[0];
        assert_eq!(hero.src_import.as_deref(), Some("hero"));
        assert_eq!(hero.src.as_deref(), Some("@/public/hero.png"));
        assert!(!hero.remote);
        assert!(hero.priority);
        assert_eq!(
            hero.sizes.as_deref(),
            Some("(max-width: 768px) 100vw, 50vw")
        );
        assert_eq!(hero.placeholder.as_deref(), Some("blur"));
        assert_eq!(hero.alt.as_deref(), Some("Hero"));
        assert_eq!(hero.import_source, "next/image");
        assert_eq!(hero.width, None);

        let remote = &usages[1];
        assert_eq!(
            remote.src.as_deref(),
            Some("https://images.unsplash.com/photo-1")
        );
        assert!(remote.remote);
        assert_eq!(remote.width, Some(800));
        assert_eq!(remote.height, Some(600));
        assert_eq!(remote.quality, Some(90));
        assert!(!remote.priority);
        assert_eq!(remote.src_import, None);

        let unmatched = &usages[2];
        assert_eq!(
            unmatched.src.as_deref(),
            Some("https://untrusted.example.net/x.png")
        );
        assert_eq!(unmatched.width, Some(100));
        assert!(unmatched.remote);
    }

    #[test]
    fn extracts_fixture_usages() {
        assert_fixture_usages(&extract_image_usages(PAGE_TSX, "app/page.tsx"));
    }

    #[test]
    fn extracts_aliased_imports_and_nested_elements() {
        let source = "import { default as Img } from \"next/legacy/image\";\nimport logo from \"../assets/logo.svg\";\nconst Card = ({ ok }) => ok ? <Img src=\"/a.png\" fill sizes=\"100vw\" loading=\"lazy\" fetchPriority=\"high\" unoptimized /> : null;\nexport function Grid({ items }) {\n  return <div>{items.map((i) => <Img key={i} src={logo} width=\"32\" height=\"32\" loader={() => \"\"} preload={false} />)}</div>;\n}\n";
        let usages = extract_image_usages(source, "components/Grid.tsx");
        assert_eq!(usages.len(), 2);
        let fill = &usages[0];
        assert_eq!(fill.import_source, "next/legacy/image");
        assert_eq!(fill.src.as_deref(), Some("/a.png"));
        assert!(fill.fill);
        assert!(fill.unoptimized);
        assert_eq!(fill.loading.as_deref(), Some("lazy"));
        assert_eq!(fill.fetch_priority.as_deref(), Some("high"));
        let logo = &usages[1];
        assert_eq!(logo.src_import.as_deref(), Some("logo"));
        assert_eq!(logo.src.as_deref(), Some("../assets/logo.svg"));
        assert_eq!(logo.width, Some(32));
        assert!(logo.custom_loader);
        assert!(!logo.priority);
        assert!(
            extract_image_usages("export const x = <img src=\"/a.png\" />;", "a.tsx").is_empty()
        );
    }

    #[test]
    fn remote_pattern_table() {
        let cloudfront = pattern("**.cloudfront.net");
        assert!(remote_pattern_matches(
            &cloudfront,
            "https://a.b.cloudfront.net/img.png"
        ));
        assert!(remote_pattern_matches(
            &cloudfront,
            "https://a.cloudfront.net/img.png"
        ));
        assert!(!remote_pattern_matches(
            &cloudfront,
            "https://cloudfront.net/img.png"
        ));
        let single = pattern("*.x.com");
        assert!(remote_pattern_matches(&single, "https://a.x.com/p"));
        assert!(remote_pattern_matches(&single, "https://A.X.COM/p"));
        assert!(!remote_pattern_matches(&single, "https://a.b.x.com/p"));
        assert!(!remote_pattern_matches(&single, "https://x.com/p"));
        let middle = pattern("avatars.*.example.com");
        assert!(remote_pattern_matches(
            &middle,
            "https://avatars.sfo1.example.com/u/1"
        ));
        assert!(!remote_pattern_matches(
            &middle,
            "https://avatars.example.com/u/1"
        ));

        let https_only = RemotePattern {
            protocol: Some(String::from("https")),
            ..pattern("images.unsplash.com")
        };
        assert!(remote_pattern_matches(
            &https_only,
            "https://images.unsplash.com/photo-1"
        ));
        assert!(!remote_pattern_matches(
            &https_only,
            "http://images.unsplash.com/photo-1"
        ));
        assert!(remote_pattern_matches(
            &https_only,
            "https://images.unsplash.com:443/photo-1"
        ));

        let account = RemotePattern {
            pathname: Some(String::from("/account123/**")),
            ..pattern("example.com")
        };
        assert!(remote_pattern_matches(
            &account,
            "https://example.com/account123/a/b.png"
        ));
        assert!(remote_pattern_matches(
            &account,
            "https://example.com/account123"
        ));
        assert!(!remote_pattern_matches(
            &account,
            "https://example.com/account456/a.png"
        ));
        let one_segment =
            RemotePattern { pathname: Some(String::from("/media/*")), ..pattern("example.com") };
        assert!(remote_pattern_matches(
            &one_segment,
            "https://example.com/media/a.png"
        ));
        assert!(!remote_pattern_matches(
            &one_segment,
            "https://example.com/media/a/b.png"
        ));

        let port = RemotePattern { port: Some(String::from("8080")), ..pattern("localhost") };
        assert!(remote_pattern_matches(&port, "http://localhost:8080/a.png"));
        assert!(!remote_pattern_matches(&port, "http://localhost/a.png"));
        let no_port = RemotePattern { port: Some(String::new()), ..pattern("cdn.example.com") };
        assert!(remote_pattern_matches(
            &no_port,
            "https://cdn.example.com/a.png"
        ));
        assert!(!remote_pattern_matches(
            &no_port,
            "https://cdn.example.com:8443/a.png"
        ));

        let search =
            RemotePattern { search: Some(String::from("?v=1")), ..pattern("cdn.example.com") };
        assert!(remote_pattern_matches(
            &search,
            "https://cdn.example.com/a.png?v=1"
        ));
        assert!(!remote_pattern_matches(
            &search,
            "https://cdn.example.com/a.png?v=2"
        ));
        assert!(!remote_pattern_matches(
            &search,
            "https://cdn.example.com/a.png"
        ));
        let no_search = RemotePattern { search: Some(String::new()), ..pattern("cdn.example.com") };
        assert!(remote_pattern_matches(
            &no_search,
            "https://cdn.example.com/a.png"
        ));
        assert!(!remote_pattern_matches(
            &no_search,
            "https://cdn.example.com/a.png?v=1"
        ));
        assert!(!remote_pattern_matches(
            &pattern("example.com"),
            "not a url"
        ));

        let local = LocalPattern {
            pathname: Some(String::from("/assets/images/**")),
            search: Some(String::new()),
        };
        assert!(local_pattern_matches(&local, "/assets/images/hero.png"));
        assert!(!local_pattern_matches(
            &local,
            "/assets/images/hero.png?v=1"
        ));
        assert!(!local_pattern_matches(&local, "/public/hero.png"));
        assert!(local_pattern_matches(
            &LocalPattern::default(),
            "/anything/at/all.png?x=1"
        ));
    }

    #[test]
    fn loader_kinds_and_policy() {
        let cfg = fixture_config();
        assert_eq!(loader_kind(&cfg), ImageLoaderKind::Default);
        let custom = ImagesConfig {
            loader: String::from("custom"),
            loader_file: String::from("./my-loader.ts"),
            ..ImagesConfig::default()
        };
        assert_eq!(loader_kind(&custom), ImageLoaderKind::Custom);
        for (name, kind) in [
            ("imgix", ImageLoaderKind::Imgix),
            ("cloudinary", ImageLoaderKind::Cloudinary),
            ("akamai", ImageLoaderKind::Akamai),
            ("custom", ImageLoaderKind::Custom),
        ] {
            let cfg = ImagesConfig { loader: String::from(name), ..ImagesConfig::default() };
            assert_eq!(loader_kind(&cfg), kind, "{name}");
        }
        let unoptimized = ImagesConfig { unoptimized: true, ..ImagesConfig::default() };
        assert_eq!(loader_kind(&unoptimized), ImageLoaderKind::Unoptimized);

        let policy = optimizer_policy(&cfg, "/docs");
        assert_eq!(policy.route, "/docs/_yolo/image");
        assert_eq!(policy.compat_routes, vec!["/docs/_next/image"]);
        assert_eq!(
            policy.cache_control,
            "public, max-age={max_age}, must-revalidate"
        );
        assert_eq!(policy.minimum_cache_ttl, 86_400);
        assert_eq!(policy.qualities, vec![50, 75, 90]);
        assert_eq!(policy.loader_file, None);
        let custom_path = ImagesConfig {
            path: String::from("/images/optimize"),
            loader_file: String::from("./loader.ts"),
            ..ImagesConfig::default()
        };
        let policy = optimizer_policy(&custom_path, "");
        assert_eq!(
            policy.compat_routes,
            vec!["/_next/image", "/images/optimize"]
        );
        assert_eq!(policy.loader, ImageLoaderKind::Custom);
        assert_eq!(policy.loader_file.as_deref(), Some("./loader.ts"));
    }

    #[test]
    fn loader_urls_per_kind() {
        let policy = policy();
        let src = "https://images.unsplash.com/photo-1";
        assert_eq!(
            loader_url(ImageLoaderKind::Default, &policy, src, 1080, 90),
            "/_yolo/image?url=https%3A%2F%2Fimages.unsplash.com%2Fphoto-1&w=1080&q=90"
        );
        assert_eq!(
            loader_url(ImageLoaderKind::Custom, &policy, "/hero.png", 640, 75),
            "/_yolo/image?url=%2Fhero.png&w=640&q=75"
        );
        assert_eq!(
            loader_url(ImageLoaderKind::Cloudflare, &policy, "/hero.png", 640, 75),
            "/cdn-cgi/image/width=640,quality=75,format=auto/hero.png"
        );
        assert_eq!(
            loader_url(
                ImageLoaderKind::Cloudinary,
                &policy,
                "https://res.cloudinary.com/demo/image/upload/sample.jpg",
                640,
                75
            ),
            "https://res.cloudinary.com/f_auto,c_limit,w_640,q_75/demo/image/upload/sample.jpg"
        );
        assert_eq!(
            loader_url(ImageLoaderKind::Cloudinary, &policy, "/sample.jpg", 640, 75),
            "/f_auto,c_limit,w_640,q_75/sample.jpg"
        );
        assert_eq!(
            loader_url(
                ImageLoaderKind::Imgix,
                &policy,
                "https://x.imgix.net/a.png?crop=faces",
                640,
                75
            ),
            "https://x.imgix.net/a.png?crop=faces&auto=format&fit=max&w=640&q=75"
        );
        assert_eq!(
            loader_url(ImageLoaderKind::Akamai, &policy, "/a.png", 640, 75),
            "/a.png?imwidth=640"
        );
        assert_eq!(
            loader_url(ImageLoaderKind::Fastly, &policy, "/a.png", 640, 75),
            "/a.png?auto=webp&width=640&quality=75"
        );
        assert_eq!(
            loader_url(ImageLoaderKind::Supabase, &policy, "/a.png", 640, 75),
            "/a.png?width=640&quality=75"
        );
        assert_eq!(
            loader_url(ImageLoaderKind::Cloudfront, &policy, "/a.png", 640, 75),
            "/a.png?format=auto&width=640&quality=75"
        );
        assert_eq!(
            loader_url(ImageLoaderKind::Unoptimized, &policy, "/a.png", 640, 75),
            "/a.png"
        );
        assert_eq!(
            encode_uri_component("a b/c?d=e&f=g!~*'()"),
            "a%20b%2Fc%3Fd%3De%26f%3Dg!~*'()"
        );
    }

    #[test]
    fn quality_coercion() {
        let policy = policy();
        assert_eq!(coerce_quality(&policy, None), 75);
        assert_eq!(coerce_quality(&policy, Some(80)), 75);
        assert_eq!(coerce_quality(&policy, Some(88)), 90);
        assert_eq!(coerce_quality(&policy, Some(10)), 50);
        assert_eq!(coerce_quality(&policy, Some(0)), 75);
        let mut two = policy.clone();
        two.qualities = vec![50, 100];
        assert_eq!(coerce_quality(&two, Some(75)), 50);
        two.qualities = vec![50, 90];
        assert_eq!(coerce_quality(&two, None), 90);
        two.qualities.clear();
        assert_eq!(coerce_quality(&two, Some(33)), 33);
    }

    #[test]
    fn srcset_algorithm() {
        let policy = policy();
        let usages = extract_image_usages(PAGE_TSX, "app/page.tsx");

        let sized = srcset(&usages[0], &policy, ImageLoaderKind::Default);
        let widths: Vec<&str> = sized.iter().map(|c| c.descriptor.as_str()).collect();
        assert_eq!(
            widths,
            vec![
                "384w", "640w", "750w", "828w", "1080w", "1200w", "1920w", "2048w", "3840w"
            ]
        );
        assert_eq!(
            sized[0].url,
            "/_yolo/image?url=%40%2Fpublic%2Fhero.png&w=384&q=75"
        );

        let fixed = srcset(&usages[1], &policy, ImageLoaderKind::Default);
        assert_eq!(fixed.len(), 2);
        assert_eq!(fixed[0].descriptor, "1x");
        assert_eq!(
            fixed[0].url,
            "/_yolo/image?url=https%3A%2F%2Fimages.unsplash.com%2Fphoto-1&w=828&q=90"
        );
        assert_eq!(fixed[1].descriptor, "2x");
        assert!(fixed[1].url.ends_with("&w=1920&q=90"));

        let mut fill = ImageUsage {
            src: Some(String::from("/banner.jpg")),
            fill: true,
            ..ImageUsage::default()
        };
        let fill_set = srcset(&fill, &policy, ImageLoaderKind::Default);
        let widths: Vec<&str> = fill_set.iter().map(|c| c.descriptor.as_str()).collect();
        assert_eq!(
            widths,
            vec![
                "640w", "750w", "828w", "1080w", "1200w", "1920w", "2048w", "3840w"
            ]
        );

        fill.sizes = Some(String::from("calc(100vw - 2rem)"));
        let all = srcset(&fill, &policy, ImageLoaderKind::Default);
        assert_eq!(all.len(), 15);
        assert_eq!(all[0].descriptor, "32w");

        let huge = ImageUsage {
            src: Some(String::from("/big.png")),
            width: Some(3000),
            ..ImageUsage::default()
        };
        let huge_set = srcset(&huge, &policy, ImageLoaderKind::Default);
        assert_eq!(huge_set.len(), 1);
        assert_eq!(huge_set[0].descriptor, "1x");
        assert!(huge_set[0].url.contains("&w=3840&"));

        let unoptimized = ImageUsage {
            src: Some(String::from("/logo.svg")),
            width: Some(100),
            ..ImageUsage::default()
        };
        assert_eq!(
            srcset(&unoptimized, &policy, ImageLoaderKind::Default),
            vec![SrcSetCandidate { url: String::from("/logo.svg"), descriptor: String::new() }]
        );
        let passthrough = ImageUsage {
            src: Some(String::from("/a.png")),
            unoptimized: true,
            ..ImageUsage::default()
        };
        assert_eq!(
            srcset(&passthrough, &policy, ImageLoaderKind::Default)[0].descriptor,
            ""
        );
        assert_eq!(
            srcset(&huge, &policy, ImageLoaderKind::Unoptimized)[0].url,
            "/big.png"
        );
        assert!(srcset(&ImageUsage::default(), &policy, ImageLoaderKind::Default).is_empty());
    }

    #[test]
    fn format_negotiation() {
        let formats = vec![String::from("image/avif"), String::from("image/webp")];
        assert_eq!(
            negotiate_format("image/avif,image/webp,*/*", &formats),
            Some("image/avif")
        );
        assert_eq!(
            negotiate_format("image/webp,*/*;q=0.8", &formats),
            Some("image/webp")
        );
        assert_eq!(negotiate_format("image/png,*/*", &formats), None);
        assert_eq!(negotiate_format("", &formats), None);
        let webp_first = vec![String::from("image/webp"), String::from("image/avif")];
        assert_eq!(
            negotiate_format("image/avif,image/webp", &webp_first),
            Some("image/webp")
        );
    }

    #[test]
    fn manganis_expressions() {
        let policy = policy();
        let usages = extract_image_usages(PAGE_TSX, "app/page.tsx");
        assert_eq!(
            manganis_asset_expr(&usages[0], &policy),
            "asset!(\"/public/hero.png\", AssetOptions::image().with_format(ImageFormat::Avif).with_preload(true))"
        );
        assert_eq!(
            manganis_asset_expr(&usages[1], &policy),
            "\"https://images.unsplash.com/photo-1\".to_string() /* remote: served through the image proxy */"
        );
        let sized = ImageUsage {
            src: Some(String::from("/images/card.png")),
            width: Some(320),
            height: Some(200),
            ..ImageUsage::default()
        };
        let mut webp = policy.clone();
        webp.formats = vec![String::from("image/webp")];
        assert_eq!(
            manganis_asset_expr(&sized, &webp),
            "asset!(\"/images/card.png\", AssetOptions::image().with_format(ImageFormat::Webp).with_size(ImageSize::Manual { width: 320, height: 200 }))"
        );
        assert!(manganis_asset_expr(&ImageUsage::default(), &policy).starts_with("String::new()"));
    }

    #[test]
    fn usage_report_flags_unmatched_hosts() {
        let cfg = fixture_config();
        let usages = extract_image_usages(PAGE_TSX, "app/page.tsx");
        let report = image_usage_report(usages, &cfg);
        assert_eq!(
            report.remote_hosts,
            vec!["images.unsplash.com", "untrusted.example.net"]
        );
        assert_eq!(
            report.unmatched_remote,
            vec!["https://untrusted.example.net/x.png"]
        );
        let codes: Vec<&str> = report.diagnostics.iter().map(|d| d.code.as_str()).collect();
        assert_eq!(codes[0], "next-image-usage-summary");
        assert!(codes.contains(&"next-image-remote-unmatched"));
        assert!(codes.contains(&"next-image-priority-deprecated"));
        assert!(!codes.contains(&"next-image-missing-dimensions"));
        let unmatched = report
            .diagnostics
            .iter()
            .find(|d| d.code == "next-image-remote-unmatched")
            .expect("unmatched diagnostic");
        assert_eq!(unmatched.file.as_deref(), Some("app/page.tsx"));
        assert!(
            unmatched
                .hint
                .as_deref()
                .is_some_and(|h| h.contains("untrusted.example.net"))
        );

        let mut domains = cfg.clone();
        domains.domains = vec![String::from("untrusted.example.net")];
        let report = image_usage_report(extract_image_usages(PAGE_TSX, "app/page.tsx"), &domains);
        assert!(report.unmatched_remote.is_empty());

        let missing = image_usage_report(
            vec![ImageUsage {
                file: String::from("x.tsx"),
                src: Some(String::from("/a.png")),
                ..ImageUsage::default()
            }],
            &cfg,
        );
        assert!(
            missing
                .diagnostics
                .iter()
                .any(|d| d.code == "next-image-missing-dimensions")
        );
    }

    #[test]
    fn manifest_shape() {
        let manifest = images_manifest_json(&policy());
        assert_eq!(manifest["version"], 1);
        let images = &manifest["images"];
        assert_eq!(images["path"], "/_yolo/image");
        assert_eq!(images["loader"], "default");
        assert_eq!(images["loaderFile"], "");
        assert_eq!(images["deviceSizes"][0], 640);
        assert_eq!(images["imageSizes"][0], 32);
        assert_eq!(images["formats"][0], "image/avif");
        assert_eq!(images["qualities"], json!([50, 75, 90]));
        assert_eq!(images["minimumCacheTTL"], 86_400);
        assert_eq!(images["dangerouslyAllowSVG"], false);
        assert_eq!(images["contentDispositionType"], "attachment");
        assert_eq!(images["remotePatterns"][1]["hostname"], "**.cloudfront.net");
        assert_eq!(images["remotePatterns"][1]["pathname"], "**");
        assert_eq!(images["remotePatterns"][2]["search"], "");
        assert_eq!(images["localPatterns"][0]["pathname"], "/assets/images/**");
        assert_eq!(images["compatRoutes"][0], "/_next/image");
        assert_eq!(images["domains"], json!([]));
        assert_eq!(images["unoptimized"], false);
    }

    #[test]
    fn fixture_page_when_present() {
        let path =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../apps/next-fixture/app/page.tsx");
        let Ok(source) = std::fs::read_to_string(&path) else {
            return;
        };
        let usages = extract_image_usages(&source, "app/page.tsx");
        assert_fixture_usages(&usages);
        let report = image_usage_report(usages, &fixture_config());
        assert_eq!(
            report.unmatched_remote,
            vec!["https://untrusted.example.net/x.png"]
        );
    }
}
