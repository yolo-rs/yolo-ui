// SPDX-License-Identifier: Apache-2.0
//! `next/font/google` and `next/font/local`: request extraction, Google CSS2 URLs, self-hosting
//! plan, SFNT metrics and size-adjusted fallbacks.
//!
//! Verified against Next.js 16.4.5 `packages/font/src/google/*` (`validate-google-font-function-call`,
//! `get-font-axes`, `get-google-fonts-url`, `sort-fonts-variant-values`, `find-font-files-in-css`,
//! `fetch-resource` user agent) and `packages/font/src/local/*` (`validate-local-font-function-call`,
//! `loader`, `get-fallback-metrics-from-font-file`).
//!
//! Offline limitations, documented per function: Google `font-data.json` (variable axis ranges,
//! subsets, categories) and the capsize metrics used for Google fallbacks are not embedded, so
//! variable-weight URLs omit the family `wght@min..max` range, extra `axes` are left out and Google
//! fallback overrides are deferred to the build-time runtime (which fetches the font files and can
//! call [`parse_sfnt_metrics`] + [`fallback_overrides`] on them).

use std::cmp::Ordering;
use std::fmt::Write as _;

use oxc_allocator::Allocator;
#[allow(clippy::wildcard_imports)]
use oxc_ast::ast::*;
use oxc_parser::Parser;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use yolo_shared::YoloThemePreset;

use crate::rsc::{
    array_items, literal_text, module_export_name, object_entries, source_type_for, static_bool,
    static_json, static_string, string_list, unwrap_ts,
};
use crate::types::{
    FallbackOverrides, FontFaceAsset, FontMetrics, FontPlan, FontRequest, FontSource, LocalFontSrc,
};

/// User agent that makes Google Fonts return `woff2` + `unicode-range` subsets (next/font).
pub const GOOGLE_FONTS_USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/104.0.0.0 Safari/537.36";

/// Sample string next/font averages advance widths over (`calcAverageWidth`).
const AVG_SAMPLE: &str = "aaabcdeeeefghiijklmnnoopqrrssttuvwxyz      ";

/// Family-name fragments that select the Times New Roman fallback.
const SERIF_INDICATORS: [&str; 20] = [
    "serif",
    "garamond",
    "playfair",
    "merriweather",
    "lora",
    "crimson",
    "libre baskerville",
    "times",
    "georgia",
    "cormorant",
    "spectral",
    "literata",
    "source serif",
    "noto serif",
    "pt serif",
    "ibm plex serif",
    "roboto serif",
    "fraunces",
    "newsreader",
    "eb garamond",
];

/// Family / binding fragments that map to the `display` preset role.
const DISPLAY_INDICATORS: [&str; 9] = [
    "display",
    "cal sans",
    "calsans",
    "bebas",
    "anton",
    "oswald",
    "lobster",
    "pacifico",
    "abril fatface",
];

// ============================================================================
// Extraction
// ============================================================================

/// Extracts `Inter({...})` / `localFont({...})` calls from a module.
///
/// Recognises named imports from `next/font/google` / `@next/font/google` (family = export name
/// with `_` -> space, e.g. `Roboto_Mono` -> `Roboto Mono`) and the default import of
/// `next/font/local` / `@next/font/local` under any local name. Module-scope `const x = Loader({...})`
/// and `export const` declarators with a static options object are extracted; a string `src`
/// inherits the top-level `weight` / `style` like `validateLocalFontFunctionCall` does.
pub fn extract_font_requests(source: &str, file: &str) -> Vec<FontRequest> {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, source_type_for(file)).parse();
    let program = &parsed.program;

    // (local binding, imported export name)
    let mut google: Vec<(String, String)> = Vec::new();
    let mut local: Vec<String> = Vec::new();
    for stmt in &program.body {
        let Statement::ImportDeclaration(import) = stmt else {
            continue;
        };
        let Some(specifiers) = &import.specifiers else {
            continue;
        };
        match literal_text(&import.source).as_str() {
            "next/font/google" | "@next/font/google" => {
                for spec in specifiers {
                    if let ImportDeclarationSpecifier::ImportSpecifier(named) = spec {
                        google.push((
                            named.local.name.as_str().to_owned(),
                            module_export_name(&named.imported),
                        ));
                    }
                }
            }
            "next/font/local" | "@next/font/local" => {
                for spec in specifiers {
                    match spec {
                        ImportDeclarationSpecifier::ImportDefaultSpecifier(default) => {
                            local.push(default.local.name.as_str().to_owned());
                        }
                        ImportDeclarationSpecifier::ImportSpecifier(named)
                            if module_export_name(&named.imported) == "default" =>
                        {
                            local.push(named.local.name.as_str().to_owned());
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
    if google.is_empty() && local.is_empty() {
        return Vec::new();
    }

    let mut requests = Vec::new();
    for stmt in &program.body {
        let decl = match stmt {
            Statement::VariableDeclaration(decl) => decl,
            Statement::ExportDeclaration(export) => match &export.declaration {
                Declaration::VariableDeclaration(decl) => decl,
                _ => continue,
            },
            _ => continue,
        };
        for declarator in &decl.declarations {
            let BindingPattern::BindingIdentifier(id) = &declarator.id else {
                continue;
            };
            let Some(init) = &declarator.init else {
                continue;
            };
            let Expression::CallExpression(call) = unwrap_ts(init) else {
                continue;
            };
            let Expression::Identifier(callee) = &call.callee else {
                continue;
            };
            let callee_name = callee.name.as_str();
            let options = call.arguments.first().and_then(Argument::as_expression);
            if let Some((_, imported)) = google.iter().find(|(name, _)| name == callee_name) {
                let source = FontSource::Google { family: imported.replace('_', " ") };
                requests.push(build_request(
                    id.name.as_str(),
                    imported,
                    source,
                    options,
                    file,
                ));
            } else if local.iter().any(|name| name == callee_name) {
                let (src, string_form) = options
                    .and_then(object_entries)
                    .and_then(|entries| {
                        entries
                            .into_iter()
                            .find(|(key, _)| key == "src")
                            .map(|(_, value)| parse_local_src(value))
                    })
                    .unwrap_or_default();
                let mut request = build_request(
                    id.name.as_str(),
                    callee_name,
                    FontSource::Local { src },
                    options,
                    file,
                );
                if string_form {
                    inherit_single_src_defaults(&mut request);
                }
                requests.push(request);
            }
        }
    }
    requests
}

fn build_request(
    binding: &str,
    import_name: &str,
    source: FontSource,
    options: Option<&Expression<'_>>,
    file: &str,
) -> FontRequest {
    let mut request = FontRequest {
        binding: binding.to_owned(),
        import_name: import_name.to_owned(),
        source,
        subsets: Vec::new(),
        weights: Vec::new(),
        styles: Vec::new(),
        display: None,
        variable: None,
        preload: None,
        fallback: Vec::new(),
        adjust_font_fallback: None,
        axes: Vec::new(),
        declarations: Vec::new(),
        file: file.to_owned(),
    };
    if let Some(entries) = options.and_then(object_entries) {
        for (key, value) in entries {
            match key.as_str() {
                "subsets" => request.subsets = string_list(value),
                "weight" => request.weights = string_list(value),
                "style" => request.styles = string_list(value),
                "display" => request.display = static_string(value),
                "variable" => request.variable = static_string(value),
                "preload" => request.preload = static_bool(value),
                "fallback" => request.fallback = string_list(value),
                "adjustFontFallback" => request.adjust_font_fallback = static_json(value),
                "axes" => request.axes = string_list(value),
                "declarations" => request.declarations = parse_declarations(value),
                _ => {}
            }
        }
    }
    request
}

/// `src: "./font.woff2"` takes the top-level `weight` / `style` (`validateLocalFontFunctionCall`).
fn inherit_single_src_defaults(request: &mut FontRequest) {
    let weight = (request.weights.len() == 1)
        .then(|| request.weights.first().cloned())
        .flatten();
    let style = (request.styles.len() == 1)
        .then(|| request.styles.first().cloned())
        .flatten();
    if let FontSource::Local { src } = &mut request.source
        && let [single] = src.as_mut_slice()
    {
        if single.weight.is_none() {
            single.weight = weight;
        }
        if single.style.is_none() {
            single.style = style;
        }
    }
}

/// `(entries, src_was_a_string)`.
fn parse_local_src(value: &Expression<'_>) -> (Vec<LocalFontSrc>, bool) {
    if let Some(path) = static_string(value) {
        return (vec![LocalFontSrc { path, weight: None, style: None }], true);
    }
    let entries = array_items(value)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|item| {
            if let Some(path) = static_string(item) {
                return Some(LocalFontSrc { path, weight: None, style: None });
            }
            let mut src = LocalFontSrc::default();
            for (key, value) in object_entries(item)? {
                match key.as_str() {
                    "path" => src.path = static_string(value).unwrap_or_default(),
                    "weight" => src.weight = string_list(value).into_iter().next(),
                    "style" => src.style = static_string(value),
                    _ => {}
                }
            }
            (!src.path.is_empty()).then_some(src)
        })
        .collect();
    (entries, false)
}

fn parse_declarations(value: &Expression<'_>) -> Vec<(String, String)> {
    array_items(value)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|item| {
            let mut prop = None;
            let mut text = None;
            for (key, value) in object_entries(item)? {
                match key.as_str() {
                    "prop" => prop = static_string(value),
                    "value" => text = static_string(value),
                    _ => {}
                }
            }
            Some((prop?, text?))
        })
        .collect()
}

// ============================================================================
// Google CSS2 URL
// ============================================================================

fn leading_int(value: &str) -> i64 {
    let digits: String = value.chars().take_while(char::is_ascii_digit).collect();
    digits.parse().unwrap_or(0)
}

/// `sortFontsVariantValues`: compare `ital` first, then the leading integer of the weight.
fn compare_tuples(a: &[String], b: &[String]) -> Ordering {
    for (x, y) in a.iter().zip(b.iter()) {
        let order = leading_int(x).cmp(&leading_int(y));
        if order != Ordering::Equal {
            return order;
        }
    }
    a.len().cmp(&b.len())
}

/// Builds the `https://fonts.googleapis.com/css2?family=...` URL (`None` for local fonts).
///
/// Mirrors `getFontAxes` + `getGoogleFontsUrl`: axis tags `ital,wght`, tuples = styles x weights
/// sorted like `sortFontsVariantValues`, then `&display=<display>` (default `swap`). A
/// `"100 900"` weight becomes `wght@100..900`. Offline we have no `font-data.json`, so `weight:
/// "variable"` (or a missing weight) cannot be expanded to the family range: the URL is
/// `family=Inter&display=swap` (plus `ital@0;1` when italic is requested) and extra `axes` are
/// omitted; the build-time runtime re-derives both from the Google metadata.
pub fn google_css2_url(request: &FontRequest) -> Option<String> {
    let FontSource::Google { family } = &request.source else {
        return None;
    };
    let display = request.display.as_deref().unwrap_or("swap");
    let styles: Vec<&str> = if request.styles.is_empty() {
        vec!["normal"]
    } else {
        request.styles.iter().map(String::as_str).collect()
    };
    let has_italic = styles.contains(&"italic");
    let has_normal = styles.contains(&"normal");
    let ital: Option<Vec<&str>> = has_italic.then(|| {
        if has_normal {
            vec!["0", "1"]
        } else {
            vec!["1"]
        }
    });

    let mut weights: Vec<String> = Vec::new();
    for weight in &request.weights {
        let trimmed = weight.trim();
        if trimmed.is_empty() || trimmed == "variable" {
            continue;
        }
        let normalized = trimmed.split_whitespace().collect::<Vec<_>>().join("..");
        if !weights.contains(&normalized) {
            weights.push(normalized);
        }
    }

    let mut tags: Vec<&str> = Vec::new();
    let mut tuples: Vec<Vec<String>> = Vec::new();
    if weights.is_empty() {
        if let Some(itals) = &ital {
            tuples.extend(itals.iter().map(|i| vec![(*i).to_owned()]));
            tags.push("ital");
        }
    } else {
        for weight in &weights {
            match &ital {
                None => tuples.push(vec![weight.clone()]),
                Some(itals) => {
                    tuples.extend(itals.iter().map(|i| vec![(*i).to_owned(), weight.clone()]));
                }
            }
        }
        if ital.is_some() {
            tags.push("ital");
        }
        tags.push("wght");
    }
    tuples.sort_by(|a, b| compare_tuples(a, b));

    let mut url = format!(
        "https://fonts.googleapis.com/css2?family={}",
        family.replace(' ', "+")
    );
    if !tuples.is_empty() {
        let values: Vec<String> = tuples.iter().map(|tuple| tuple.join(",")).collect();
        let _ = write!(url, ":{}@{}", tags.join(","), values.join(";"));
    }
    let _ = write!(url, "&display={display}");
    Some(url)
}

// ============================================================================
// Plan
// ============================================================================

fn sha256_hex(input: &[u8]) -> String {
    let digest = Sha256::digest(input);
    let mut out = String::with_capacity(64);
    for byte in digest.as_slice() {
        let _ = write!(out, "{byte:02x}");
    }
    out
}

fn strip_quotes(value: &str) -> String {
    value
        .trim()
        .trim_matches(|c| c == '\'' || c == '"')
        .to_owned()
}

fn pascal_case(name: &str) -> String {
    let mut out = String::new();
    let mut upper = true;
    for ch in name.chars() {
        if matches!(ch, '_' | '-' | ' ' | '$') {
            upper = true;
            continue;
        }
        if upper {
            out.extend(ch.to_uppercase());
            upper = false;
        } else {
            out.push(ch);
        }
    }
    out
}

/// Lowercase slug used in self-hosted file names.
fn slug(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    out.trim_matches('-').to_owned()
}

/// Family used in CSS: the Google family, or for local fonts the `font-family` declaration
/// override, else the `PascalCase` binding (`calSans` -> `CalSans`).
fn plan_family(request: &FontRequest) -> String {
    match &request.source {
        FontSource::Google { family } => family.clone(),
        FontSource::Local { .. } => request
            .declarations
            .iter()
            .find(|(prop, _)| prop.eq_ignore_ascii_case("font-family"))
            .map_or_else(
                || pascal_case(&request.binding),
                |(_, value)| strip_quotes(value),
            ),
    }
}

/// `adjustFontFallback` defaults to `true` (Google) / `"Arial"` (local); only `false` disables it.
fn adjust_enabled(request: &FontRequest) -> bool {
    !matches!(request.adjust_font_fallback, Some(Value::Bool(false)))
}

fn single_weight(request: &FontRequest) -> Option<String> {
    let top_level = match request.weights.as_slice() {
        [weight] if weight != "variable" => Some(weight.clone()),
        _ => None,
    };
    match &request.source {
        FontSource::Google { .. } => top_level,
        FontSource::Local { src } => match src.as_slice() {
            [single] => single.weight.clone().or(top_level),
            _ => None,
        },
    }
}

fn single_style(request: &FontRequest) -> Option<String> {
    let top_level = match request.styles.as_slice() {
        [style] => Some(style.clone()),
        _ => None,
    };
    match &request.source {
        FontSource::Google { .. } => top_level,
        FontSource::Local { src } => match src.as_slice() {
            [single] => single.style.clone().or(top_level),
            _ => None,
        },
    }
}

fn preset_role(request: &FontRequest, family: &str) -> &'static str {
    let haystack = format!(
        "{family} {} {}",
        request.binding,
        request.variable.as_deref().unwrap_or("")
    )
    .to_ascii_lowercase();
    if haystack.contains("mono") || haystack.contains("code") {
        return "mono";
    }
    if DISPLAY_INDICATORS.iter().any(|d| haystack.contains(d)) {
        return "display";
    }
    "sans"
}

fn extension_to_format(ext: &str) -> Option<&'static str> {
    match ext {
        "woff2" => Some("woff2"),
        "woff" => Some("woff"),
        "ttf" => Some("truetype"),
        "otf" => Some("opentype"),
        "eot" => Some("embedded-opentype"),
        _ => None,
    }
}

fn format_to_extension(format: &str) -> Option<&'static str> {
    match format {
        "woff2" => Some("woff2"),
        "woff" => Some("woff"),
        "truetype" => Some("ttf"),
        "opentype" => Some("otf"),
        "embedded-opentype" => Some("eot"),
        _ => None,
    }
}

/// `@font-face` rules + assets of a local font (one per `src` entry), served from `fonts_route`.
fn local_faces(
    request: &FontRequest,
    family: &str,
    src: &[LocalFontSrc],
    fonts_route: &str,
) -> (Vec<FontFaceAsset>, String) {
    let route = fonts_route.trim_end_matches('/');
    let display = request.display.as_deref().unwrap_or("swap");
    let default_weight = (request.weights.len() == 1)
        .then(|| request.weights.first().cloned())
        .flatten();
    let default_style = (request.styles.len() == 1)
        .then(|| request.styles.first().cloned())
        .flatten();
    let mut css = String::new();
    let mut assets = Vec::new();
    for entry in src {
        let file_name = entry
            .path
            .rsplit(['/', '\\'])
            .next()
            .unwrap_or(entry.path.as_str());
        let ext = file_name.rsplit_once('.').map_or_else(
            || String::from("woff2"),
            |(_, ext)| ext.to_ascii_lowercase(),
        );
        let format = extension_to_format(&ext).unwrap_or("woff2");
        let local_path = format!("{route}/{file_name}");
        let weight = entry.weight.clone().or_else(|| default_weight.clone());
        let style = entry.style.clone().or_else(|| default_style.clone());
        css.push_str("@font-face {\n");
        for (prop, value) in &request.declarations {
            if prop.eq_ignore_ascii_case("font-family") {
                continue;
            }
            let _ = writeln!(css, "  {prop}: {value};");
        }
        let _ = writeln!(css, "  font-family: '{family}';");
        let _ = writeln!(css, "  src: url('{local_path}') format('{format}');");
        let _ = writeln!(css, "  font-display: {display};");
        if let Some(weight) = &weight {
            let _ = writeln!(css, "  font-weight: {weight};");
        }
        if let Some(style) = &style {
            let _ = writeln!(css, "  font-style: {style};");
        }
        css.push_str("}\n");
        assets.push(FontFaceAsset {
            source: entry.path.clone(),
            local_path,
            family: family.to_owned(),
            weight: weight.unwrap_or_else(|| String::from("normal")),
            style: style.unwrap_or_else(|| String::from("normal")),
            unicode_range: None,
            subset: None,
            format: format.to_owned(),
            preload: request.preload.unwrap_or(true),
        });
    }
    (assets, css)
}

/// Builds the plan (class names, family CSS, fallback) for one request, without network access.
///
/// - `class_name` = `yolo-font-<sha256(file + binding + family)[..8]>`, `variable_class` =
///   `yolo-font-var-<same hash>` when `variable` is set.
/// - `family_css` = `'<Family>', '<Family> Fallback'` + user `fallback` list when
///   `adjustFontFallback` is enabled (default), else `'<Family>'` + fallbacks.
/// - `fallback` is `None` offline (no metrics yet); the runtime fills it with
///   [`fallback_overrides`] once it has the font bytes, and [`font_class_css`] then emits the
///   `<Family> Fallback` face.
/// - Google fonts: `assets` stay empty until [`rewrite_font_css`] runs on the fetched CSS.
///   Local fonts: one `@font-face` + asset per `src`, served as `<fonts_route>/<file name>`.
pub fn plan_font(request: &FontRequest, fonts_route: &str) -> FontPlan {
    let family = plan_family(request);
    let hash = sha256_hex(format!("{}{}{family}", request.file, request.binding).as_bytes());
    let short = hash.get(..8).unwrap_or(&hash);
    let class_name = format!("yolo-font-{short}");
    let variable_class = request
        .variable
        .as_ref()
        .map(|_| format!("yolo-font-var-{short}"));

    let mut families = vec![format!("'{family}'")];
    if adjust_enabled(request) {
        families.push(format!("'{family} Fallback'"));
    }
    families.extend(request.fallback.iter().cloned());
    let family_css = families.join(", ");

    let (assets, face_css) = match &request.source {
        FontSource::Local { src } => local_faces(request, &family, src, fonts_route),
        FontSource::Google { .. } => (Vec::new(), String::new()),
    };

    let mut plan = FontPlan {
        request: request.clone(),
        css2_url: google_css2_url(request),
        family_css,
        class_name,
        variable_class,
        fallback: None,
        assets,
        css: String::new(),
        preset_role: Some(preset_role(request, &family).to_owned()),
    };
    let class_css = font_class_css(&plan);
    plan.css = if face_css.is_empty() {
        class_css
    } else {
        format!("{face_css}\n{class_css}")
    };
    plan
}

// ============================================================================
// Google CSS rewrite
// ============================================================================

fn is_remote_url(url: &str) -> bool {
    let lower = url.to_ascii_lowercase();
    lower.starts_with("http://") || lower.starts_with("https://") || lower.starts_with("//")
}

/// Splits on `delimiter` outside parentheses.
fn split_top_level(text: &str, delimiter: char) -> Vec<&str> {
    let mut out = Vec::new();
    let mut depth = 0usize;
    let mut start = 0usize;
    for (index, ch) in text.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            c if c == delimiter && depth == 0 => {
                out.push(&text[start..index]);
                start = index + ch.len_utf8();
            }
            _ => {}
        }
    }
    out.push(&text[start..]);
    out
}

/// Last `/* ... */` comment of a CSS fragment (Google puts the subset above each face).
fn last_comment(text: &str) -> Option<String> {
    let start = text.rfind("/*")?;
    let body = &text[start + 2..];
    let end = body.find("*/")?;
    let comment = body[..end].trim();
    (!comment.is_empty()).then(|| comment.to_owned())
}

fn parse_descriptors(block: &str) -> Vec<(String, String)> {
    split_top_level(block, ';')
        .into_iter()
        .filter_map(|declaration| {
            let (name, value) = declaration.split_once(':')?;
            let name = name.trim().to_ascii_lowercase();
            (!name.is_empty()).then(|| (name, value.trim().to_owned()))
        })
        .collect()
}

/// `(url, format hint)` pairs of a `src:` descriptor.
fn parse_src_urls(src: &str) -> Vec<(String, Option<String>)> {
    let mut out = Vec::new();
    for part in split_top_level(src, ',') {
        let Some(url_start) = part.find("url(") else {
            continue;
        };
        let after = &part[url_start + 4..];
        let Some(url_end) = after.find(')') else {
            continue;
        };
        let url = after[..url_end]
            .trim()
            .trim_matches(|c| c == '"' || c == '\'')
            .to_owned();
        let tail = &after[url_end + 1..];
        let format = tail.find("format(").and_then(|pos| {
            let inner = &tail[pos + 7..];
            let end = inner.find(')')?;
            Some(
                inner[..end]
                    .trim()
                    .trim_matches(|c| c == '"' || c == '\'')
                    .to_ascii_lowercase(),
            )
        });
        out.push((url, format));
    }
    out
}

fn url_extension(url: &str) -> Option<&str> {
    let path = url.split(['?', '#']).next().unwrap_or(url);
    let file = path.rsplit('/').next().unwrap_or(path);
    let (_, ext) = file.rsplit_once('.')?;
    matches!(ext, "woff2" | "woff" | "ttf" | "otf" | "eot").then_some(ext)
}

/// Rewrites a Google CSS response: remote `url()` -> `<fonts_route>/<hash>.<ext>`, collecting assets.
///
/// Follows `findFontFilesInCss`: the `/* latin */` comment above each `@font-face` names its
/// subset; a file is preloaded when `preload` is not `false` and its subset is in
/// `request.subsets` (or no subsets were requested). The local name is
/// `<family-slug>-<sha256(url)[..12]>.<ext>` where the extension comes from the URL path, then
/// the `format()` hint, then `woff2` (what the Chrome user agent receives).
pub fn rewrite_font_css(
    css: &str,
    request: &FontRequest,
    fonts_route: &str,
) -> (String, Vec<FontFaceAsset>) {
    let default_family = plan_family(request);
    let preload_enabled = request.preload.unwrap_or(true);
    let route = fonts_route.trim_end_matches('/');
    let mut rewritten = css.to_owned();
    let mut assets: Vec<FontFaceAsset> = Vec::new();
    let mut subset: Option<String> = None;
    let mut rest = css;
    while let Some(start) = rest.find("@font-face") {
        if let Some(comment) = last_comment(&rest[..start]) {
            subset = Some(comment);
        }
        let after = &rest[start..];
        let Some(open) = after.find('{') else {
            break;
        };
        let Some(close) = after[open..].find('}') else {
            break;
        };
        let block = &after[open + 1..open + close];
        let descriptors = parse_descriptors(block);
        let get = |name: &str| {
            descriptors
                .iter()
                .find(|(key, _)| key == name)
                .map(|(_, value)| value.clone())
        };
        let family =
            get("font-family").map_or_else(|| default_family.clone(), |f| strip_quotes(&f));
        let weight = get("font-weight").unwrap_or_else(|| String::from("400"));
        let style = get("font-style").unwrap_or_else(|| String::from("normal"));
        let unicode_range = get("unicode-range");
        if let Some(src) = get("src") {
            for (url, format_hint) in parse_src_urls(&src) {
                if !is_remote_url(&url) {
                    continue;
                }
                let ext = url_extension(&url)
                    .or_else(|| format_hint.as_deref().and_then(format_to_extension))
                    .unwrap_or("woff2")
                    .to_owned();
                let format = format_hint
                    .clone()
                    .or_else(|| extension_to_format(&ext).map(str::to_owned))
                    .unwrap_or_else(|| String::from("woff2"));
                let hash = sha256_hex(url.as_bytes());
                let local_path = format!(
                    "{route}/{}-{}.{ext}",
                    slug(&family),
                    hash.get(..12).unwrap_or(&hash)
                );
                if !assets.iter().any(|asset| asset.source == url) {
                    let in_subsets = request.subsets.is_empty()
                        || subset
                            .as_deref()
                            .is_some_and(|s| request.subsets.iter().any(|wanted| wanted == s));
                    assets.push(FontFaceAsset {
                        source: url.clone(),
                        local_path: local_path.clone(),
                        family: family.clone(),
                        weight: weight.clone(),
                        style: style.clone(),
                        unicode_range: unicode_range.clone(),
                        subset: subset.clone(),
                        format,
                        preload: preload_enabled && in_subsets,
                    });
                }
                rewritten = rewritten.replace(&url, &local_path);
            }
        }
        rest = &after[open + close + 1..];
    }
    (rewritten, assets)
}

// ============================================================================
// SFNT metrics
// ============================================================================

#[derive(Default)]
struct Tables {
    head: Option<(usize, usize)>,
    hhea: Option<(usize, usize)>,
    os2: Option<(usize, usize)>,
    cmap: Option<(usize, usize)>,
    hmtx: Option<(usize, usize)>,
}

fn read_tag(bytes: &[u8], offset: usize) -> Option<[u8; 4]> {
    bytes.get(offset..offset.checked_add(4)?)?.try_into().ok()
}

fn read_u16(bytes: &[u8], offset: usize) -> Option<u16> {
    let slice = bytes.get(offset..offset.checked_add(2)?)?;
    Some(u16::from_be_bytes(slice.try_into().ok()?))
}

fn read_i16(bytes: &[u8], offset: usize) -> Option<i16> {
    read_u16(bytes, offset).map(|v| i16::from_be_bytes(v.to_be_bytes()))
}

fn read_u32(bytes: &[u8], offset: usize) -> Option<u32> {
    let slice = bytes.get(offset..offset.checked_add(4)?)?;
    Some(u32::from_be_bytes(slice.try_into().ok()?))
}

/// Best Unicode `cmap` subtable (format 12 preferred over format 4).
fn unicode_cmap_subtable(bytes: &[u8], cmap: usize) -> Option<usize> {
    let count = usize::from(read_u16(bytes, cmap + 2)?);
    let mut best: Option<(u8, usize)> = None;
    for index in 0..count {
        let record = cmap + 4 + index * 8;
        let platform = read_u16(bytes, record)?;
        let encoding = read_u16(bytes, record + 2)?;
        let offset = cmap + usize::try_from(read_u32(bytes, record + 4)?).ok()?;
        let unicode = platform == 0 || (platform == 3 && (encoding == 1 || encoding == 10));
        if !unicode {
            continue;
        }
        let priority = match read_u16(bytes, offset)? {
            12 => 2,
            4 => 1,
            _ => continue,
        };
        if best.is_none_or(|(current, _)| priority > current) {
            best = Some((priority, offset));
        }
    }
    best.map(|(_, offset)| offset)
}

fn glyph_index_format4(bytes: &[u8], sub: usize, code: u32) -> Option<u16> {
    let code = u16::try_from(code).ok()?;
    let seg_count = usize::from(read_u16(bytes, sub + 6)?) / 2;
    let end_codes = sub + 14;
    let start_codes = end_codes + seg_count * 2 + 2;
    let id_deltas = start_codes + seg_count * 2;
    let id_range_offsets = id_deltas + seg_count * 2;
    for segment in 0..seg_count {
        let end = read_u16(bytes, end_codes + segment * 2)?;
        if end < code {
            continue;
        }
        let start = read_u16(bytes, start_codes + segment * 2)?;
        if start > code {
            return Some(0);
        }
        let delta = read_u16(bytes, id_deltas + segment * 2)?;
        let range_offset = read_u16(bytes, id_range_offsets + segment * 2)?;
        if range_offset == 0 {
            return Some(code.wrapping_add(delta));
        }
        let address = id_range_offsets
            + segment * 2
            + usize::from(range_offset)
            + usize::from(code - start) * 2;
        let glyph = read_u16(bytes, address)?;
        return Some(if glyph == 0 {
            0
        } else {
            glyph.wrapping_add(delta)
        });
    }
    Some(0)
}

fn glyph_index_format12(bytes: &[u8], sub: usize, code: u32) -> Option<u16> {
    let groups = usize::try_from(read_u32(bytes, sub + 12)?).ok()?;
    for group in 0..groups {
        let record = sub + 16 + group * 12;
        let start = read_u32(bytes, record)?;
        let end = read_u32(bytes, record + 4)?;
        let start_glyph = read_u32(bytes, record + 8)?;
        if start > code {
            break;
        }
        if code <= end {
            return u16::try_from(start_glyph.checked_add(code - start)?).ok();
        }
    }
    Some(0)
}

fn glyph_index(bytes: &[u8], subtable: usize, code: u32) -> Option<u16> {
    match read_u16(bytes, subtable)? {
        4 => glyph_index_format4(bytes, subtable, code),
        12 => glyph_index_format12(bytes, subtable, code),
        _ => None,
    }
}

/// Mean advance width over [`AVG_SAMPLE`] (`azAvgWidth`); `None` when a sample glyph is missing.
fn average_advance(bytes: &[u8], tables: &Tables) -> Option<f64> {
    let (hhea, _) = tables.hhea?;
    let (cmap, _) = tables.cmap?;
    let (hmtx, _) = tables.hmtx?;
    let num_h_metrics = usize::from(read_u16(bytes, hhea + 34)?);
    if num_h_metrics == 0 {
        return None;
    }
    let subtable = unicode_cmap_subtable(bytes, cmap)?;
    let mut total = 0.0;
    let mut count = 0.0;
    for ch in AVG_SAMPLE.chars() {
        let glyph = glyph_index(bytes, subtable, u32::from(ch))?;
        if glyph == 0 {
            return None;
        }
        let index = usize::from(glyph).min(num_h_metrics - 1);
        total += f64::from(read_u16(bytes, hmtx + index * 4)?);
        count += 1.0;
    }
    Some(total / count)
}

/// Parses `head`, `hhea`, `OS/2` tables of a `TrueType` / `OpenType` (SFNT) font.
///
/// Accepts the `0x00010000`, `OTTO` and `true` tags and `TrueType` collections (`ttcf`, first
/// font). WOFF (`wOFF`) / WOFF2 (`wOF2`) containers hold compressed tables and return `None`:
/// decode them first. `cap_height` needs `OS/2` version >= 2; `az_avg_width` is the mean advance
/// over the next/font sample string via `cmap` (format 4 / 12) + `hmtx` and is `None` when a
/// sample glyph is missing. Every read is bounds-checked; malformed input yields `None`.
pub fn parse_sfnt_metrics(bytes: &[u8], family: &str) -> Option<FontMetrics> {
    let mut base = 0usize;
    if &read_tag(bytes, 0)? == b"ttcf" {
        base = usize::try_from(read_u32(bytes, 12)?).ok()?;
    }
    let version = read_tag(bytes, base)?;
    if !(version == [0, 1, 0, 0] || &version == b"OTTO" || &version == b"true") {
        return None;
    }
    let num_tables = usize::from(read_u16(bytes, base + 4)?);
    let mut tables = Tables::default();
    for index in 0..num_tables {
        let record = base + 12 + index * 16;
        let tag = read_tag(bytes, record)?;
        let offset = usize::try_from(read_u32(bytes, record + 8)?).ok()?;
        let length = usize::try_from(read_u32(bytes, record + 12)?).ok()?;
        let slot = match &tag {
            b"head" => &mut tables.head,
            b"hhea" => &mut tables.hhea,
            b"OS/2" => &mut tables.os2,
            b"cmap" => &mut tables.cmap,
            b"hmtx" => &mut tables.hmtx,
            _ => continue,
        };
        *slot = Some((offset, length));
    }
    let (head, _) = tables.head?;
    let (hhea, _) = tables.hhea?;
    let (os2, os2_len) = tables.os2?;
    let units_per_em = read_u16(bytes, head + 18)?;
    if units_per_em == 0 {
        return None;
    }
    let ascent = i32::from(read_i16(bytes, hhea + 4)?);
    let descent = i32::from(read_i16(bytes, hhea + 6)?);
    let line_gap = i32::from(read_i16(bytes, hhea + 8)?);
    let os2_version = read_u16(bytes, os2)?;
    let x_avg_char_width = i32::from(read_i16(bytes, os2 + 2)?);
    let cap_height = if os2_version >= 2 && os2_len >= 90 {
        read_i16(bytes, os2 + 88).map(i32::from)
    } else {
        None
    };
    let az_avg_width = average_advance(bytes, &tables);
    Some(FontMetrics {
        family: family.to_owned(),
        units_per_em,
        ascent,
        descent,
        line_gap,
        cap_height,
        x_avg_char_width,
        az_avg_width,
    })
}

/// Arial metrics (`xAvgCharWidth` 904, `unitsPerEm` 2048).
pub fn arial_metrics() -> FontMetrics {
    FontMetrics {
        family: String::from("Arial"),
        units_per_em: 2048,
        ascent: 1854,
        descent: -434,
        line_gap: 67,
        cap_height: Some(1467),
        x_avg_char_width: 904,
        az_avg_width: Some(934.511_627_906_976_7),
    }
}

/// Times New Roman metrics (`xAvgCharWidth` 821, `unitsPerEm` 2048).
pub fn times_new_roman_metrics() -> FontMetrics {
    FontMetrics {
        family: String::from("Times New Roman"),
        units_per_em: 2048,
        ascent: 1825,
        descent: -443,
        line_gap: 87,
        cap_height: Some(1341),
        x_avg_char_width: 821,
        az_avg_width: Some(854.395_348_837_209_3),
    }
}

/// Computes `ascent-override`, `descent-override`, `line-gap-override`, `size-adjust` (next/font formula).
///
/// `size_adjust = (main_avg / main_upm) / (fallback_avg / fallback_upm)` where the average is
/// `az_avg_width` when both fonts have it (next/font/local) and `x_avg_char_width` otherwise
/// (capsize `xWidthAvg` path); each override is `|metric / (upm * size_adjust) * 100|` formatted
/// with two decimals like `formatOverrideValue`.
pub fn fallback_overrides(main: &FontMetrics, fallback: &FontMetrics) -> FallbackOverrides {
    let upm = if main.units_per_em == 0 {
        1000.0
    } else {
        f64::from(main.units_per_em)
    };
    let fallback_upm = if fallback.units_per_em == 0 {
        1000.0
    } else {
        f64::from(fallback.units_per_em)
    };
    let size_adjust = match (main.az_avg_width, fallback.az_avg_width) {
        (Some(main_avg), Some(fallback_avg)) if main_avg > 0.0 && fallback_avg > 0.0 => {
            (main_avg / upm) / (fallback_avg / fallback_upm)
        }
        _ if main.x_avg_char_width > 0 && fallback.x_avg_char_width > 0 => {
            (f64::from(main.x_avg_char_width) / upm)
                / (f64::from(fallback.x_avg_char_width) / fallback_upm)
        }
        _ => 1.0,
    };
    let percent = |value: f64| format!("{:.2}%", (value * 100.0).abs());
    FallbackOverrides {
        fallback_family: fallback.family.clone(),
        ascent_override: percent(f64::from(main.ascent) / (upm * size_adjust)),
        descent_override: percent(f64::from(main.descent) / (upm * size_adjust)),
        line_gap_override: percent(f64::from(main.line_gap) / (upm * size_adjust)),
        size_adjust: percent(size_adjust),
    }
}

fn looks_serif(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    if lower.contains("sans") || lower.contains("mono") || lower.contains("code") {
        return false;
    }
    SERIF_INDICATORS.iter().any(|needle| lower.contains(needle))
}

/// Chooses Arial or Times New Roman for a request (explicit option, else serif heuristic).
///
/// `adjustFontFallback: "Times New Roman"` selects Times, `"Arial"` selects Arial; `false` still
/// returns Arial (the caller checks the option before emitting a fallback face). Otherwise the
/// family name (or `metrics.family`) is matched against common serif families; Next.js uses the
/// Google category metadata here, which is not available offline.
pub fn fallback_font_for(request: &FontRequest, metrics: Option<&FontMetrics>) -> FontMetrics {
    if let Some(Value::String(choice)) = &request.adjust_font_fallback {
        if choice.eq_ignore_ascii_case("times new roman") || choice.eq_ignore_ascii_case("serif") {
            return times_new_roman_metrics();
        }
        if choice.eq_ignore_ascii_case("arial") || choice.eq_ignore_ascii_case("sans-serif") {
            return arial_metrics();
        }
    }
    let family = plan_family(request);
    let serif = looks_serif(&family) || metrics.is_some_and(|m| looks_serif(&m.family));
    if serif {
        times_new_roman_metrics()
    } else {
        arial_metrics()
    }
}

/// `.class { font-family }` + `.variable { --var }` CSS for a plan.
///
/// `font-weight` is emitted only for a single non-variable weight and `font-style` only for a
/// single style (like `next-font-loader`); the `<Family> Fallback` `@font-face` is appended when
/// `plan.fallback` is known.
pub fn font_class_css(plan: &FontPlan) -> String {
    let mut css = String::new();
    let mut declarations = vec![format!("font-family: {}", plan.family_css)];
    if let Some(weight) = single_weight(&plan.request) {
        declarations.push(format!("font-weight: {weight}"));
    }
    if let Some(style) = single_style(&plan.request) {
        declarations.push(format!("font-style: {style}"));
    }
    let _ = writeln!(
        css,
        ".{} {{ {}; }}",
        plan.class_name,
        declarations.join("; ")
    );
    if let (Some(variable_class), Some(variable)) = (&plan.variable_class, &plan.request.variable) {
        let _ = writeln!(
            css,
            ".{variable_class} {{ {variable}: {}; }}",
            plan.family_css
        );
    }
    if let Some(fallback) = &plan.fallback {
        let family = plan_family(&plan.request);
        let _ = writeln!(
            css,
            "@font-face {{ font-family: '{family} Fallback'; src: local('{}'); ascent-override: {}; descent-override: {}; line-gap-override: {}; size-adjust: {}; }}",
            fallback.fallback_family,
            fallback.ascent_override,
            fallback.descent_override,
            fallback.line_gap_override,
            fallback.size_adjust
        );
    }
    css.trim_end().to_owned()
}

/// `(sans, mono)` font stacks of a yolo-ui preset.
pub fn preset_font_stack(preset: YoloThemePreset) -> (String, String) {
    let style = preset.style();
    (
        style.font_family_ui.to_owned(),
        style.font_family_mono.to_owned(),
    )
}

/// `.yolo/fonts.json` manifest.
///
/// `{ "fonts": [ { binding, family, css2Url, userAgent, className, variableClass, variable,
/// familyCss, preload, subsets, weights, styles, display, assets, css } ] }`; `userAgent` is
/// [`GOOGLE_FONTS_USER_AGENT`] for Google fonts and `null` for local fonts.
pub fn fonts_manifest_json(plans: &[FontPlan]) -> Value {
    let fonts: Vec<Value> = plans
        .iter()
        .map(|plan| {
            let request = &plan.request;
            let user_agent = match request.source {
                FontSource::Google { .. } => Value::String(GOOGLE_FONTS_USER_AGENT.to_owned()),
                FontSource::Local { .. } => Value::Null,
            };
            json!({
                "binding": request.binding,
                "family": plan_family(request),
                "css2Url": plan.css2_url,
                "userAgent": user_agent,
                "className": plan.class_name,
                "variableClass": plan.variable_class,
                "variable": request.variable,
                "familyCss": plan.family_css,
                "preload": request.preload.unwrap_or(true),
                "subsets": request.subsets,
                "weights": request.weights,
                "styles": request.styles,
                "display": request.display.as_deref().unwrap_or("swap"),
                "assets": plan.assets,
                "css": plan.css,
            })
        })
        .collect();
    json!({ "fonts": fonts })
}

#[cfg(test)]
#[allow(clippy::assert_is_empty)] // `assert!(x.is_empty())` reads better than typed empty arrays
mod tests {
    use std::path::Path;

    use super::*;

    const LAYOUT_TSX: &str = r#"import type { Metadata } from "next";
import { Inter, Roboto_Mono } from "next/font/google";
import localFont from "next/font/local";
import "./globals.css";

const inter = Inter({ subsets: ["latin"], display: "swap", variable: "--font-inter" });
const robotoMono = Roboto_Mono({ subsets: ["latin"], weight: ["400", "700"], variable: "--font-mono" });
const calSans = localFont({
  src: [{ path: "../public/fonts/CalSans-SemiBold.woff2", weight: "600", style: "normal" }],
  variable: "--font-display",
  display: "swap",
  adjustFontFallback: "Arial",
});

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en" className={`${inter.variable} ${robotoMono.variable} ${calSans.variable}`}>
      <body className={inter.className}>{children}</body>
    </html>
  );
}
"#;

    const GOOGLE_CSS: &str = "/* cyrillic-ext */
@font-face {
  font-family: 'Inter';
  font-style: normal;
  font-weight: 400;
  font-display: swap;
  src: url(https://fonts.gstatic.com/s/inter/v18/UcC73FwrK3iLTeHuS_nVMrMxCp50SjIa2JL7W0Q5n-wU.woff2) format('woff2');
  unicode-range: U+0460-052F, U+1C80-1C88, U+20B4, U+2DE0-2DFF, U+A640-A69F, U+FE2E-FE2F;
}
/* latin */
@font-face {
  font-family: 'Inter';
  font-style: normal;
  font-weight: 400;
  font-display: swap;
  src: url(https://fonts.gstatic.com/s/inter/v18/UcC73FwrK3iLTeHuS_nVMrMxCp50SjIa1ZL7W0Q5nw.woff2) format('woff2');
  unicode-range: U+0000-00FF, U+0131, U+0152-0153, U+02BB-02BC, U+02C6, U+02DA, U+02DC, U+0304, U+0308, U+0329, U+2000-206F, U+20AC, U+2122, U+2191, U+2193, U+2212, U+2215, U+FEFF, U+FFFD;
}
";

    fn fixture_requests() -> Vec<FontRequest> {
        extract_font_requests(LAYOUT_TSX, "app/layout.tsx")
    }

    fn google_request(family: &str, weights: &[&str], styles: &[&str]) -> FontRequest {
        FontRequest {
            binding: String::from("font"),
            import_name: family.replace(' ', "_"),
            source: FontSource::Google { family: family.to_owned() },
            subsets: vec![String::from("latin")],
            weights: weights.iter().map(|w| (*w).to_owned()).collect(),
            styles: styles.iter().map(|s| (*s).to_owned()).collect(),
            display: None,
            variable: None,
            preload: None,
            fallback: Vec::new(),
            adjust_font_fallback: None,
            axes: Vec::new(),
            declarations: Vec::new(),
            file: String::from("app/layout.tsx"),
        }
    }

    fn assert_fixture_requests(requests: &[FontRequest]) {
        assert_eq!(requests.len(), 3, "{requests:?}");
        let inter = &requests[0];
        assert_eq!(inter.binding, "inter");
        assert_eq!(inter.import_name, "Inter");
        assert_eq!(
            inter.source,
            FontSource::Google { family: String::from("Inter") }
        );
        assert_eq!(inter.subsets, vec!["latin"]);
        assert_eq!(inter.display.as_deref(), Some("swap"));
        assert_eq!(inter.variable.as_deref(), Some("--font-inter"));
        assert!(inter.weights.is_empty());

        let mono = &requests[1];
        assert_eq!(mono.binding, "robotoMono");
        assert_eq!(mono.import_name, "Roboto_Mono");
        assert_eq!(
            mono.source,
            FontSource::Google { family: String::from("Roboto Mono") }
        );
        assert_eq!(mono.weights, vec!["400", "700"]);
        assert_eq!(mono.variable.as_deref(), Some("--font-mono"));

        let cal = &requests[2];
        assert_eq!(cal.binding, "calSans");
        assert_eq!(cal.import_name, "localFont");
        let FontSource::Local { src } = &cal.source else {
            panic!("calSans must be a local font");
        };
        assert_eq!(src.len(), 1);
        assert_eq!(src[0].path, "../public/fonts/CalSans-SemiBold.woff2");
        assert_eq!(src[0].weight.as_deref(), Some("600"));
        assert_eq!(src[0].style.as_deref(), Some("normal"));
        assert_eq!(cal.variable.as_deref(), Some("--font-display"));
        assert_eq!(
            cal.adjust_font_fallback,
            Some(Value::String(String::from("Arial")))
        );
        assert_eq!(cal.file, "app/layout.tsx");
    }

    #[test]
    fn extracts_fixture_font_calls() {
        assert_fixture_requests(&fixture_requests());
    }

    #[test]
    fn extracts_exported_and_aliased_calls() {
        let source = "import { Playfair_Display as Playfair } from \"@next/font/google\";\nimport { default as loadLocal } from \"next/font/local\";\nexport const serif = Playfair({ weight: \"700\", style: [\"normal\", \"italic\"], preload: false, fallback: [\"Georgia\"], adjustFontFallback: false, axes: [\"opsz\"] });\nexport const body = loadLocal({ src: \"./Body.woff2\", weight: \"400\", style: \"italic\", declarations: [{ prop: \"ascent-override\", value: \"90%\" }] });\n";
        let requests = extract_font_requests(source, "src/fonts.ts");
        assert_eq!(requests.len(), 2);
        let serif = &requests[0];
        assert_eq!(serif.binding, "serif");
        assert_eq!(serif.import_name, "Playfair_Display");
        assert_eq!(
            serif.source,
            FontSource::Google { family: String::from("Playfair Display") }
        );
        assert_eq!(serif.weights, vec!["700"]);
        assert_eq!(serif.styles, vec!["normal", "italic"]);
        assert_eq!(serif.preload, Some(false));
        assert_eq!(serif.fallback, vec!["Georgia"]);
        assert_eq!(serif.adjust_font_fallback, Some(Value::Bool(false)));
        assert_eq!(serif.axes, vec!["opsz"]);
        let body = &requests[1];
        let FontSource::Local { src } = &body.source else {
            panic!("body must be local");
        };
        assert_eq!(src[0].path, "./Body.woff2");
        assert_eq!(src[0].weight.as_deref(), Some("400"));
        assert_eq!(src[0].style.as_deref(), Some("italic"));
        assert_eq!(
            body.declarations,
            vec![(String::from("ascent-override"), String::from("90%"))]
        );
    }

    #[test]
    fn css2_urls_follow_next_font() {
        let inter = google_request("Inter", &[], &[]);
        assert_eq!(
            google_css2_url(&inter).as_deref(),
            Some("https://fonts.googleapis.com/css2?family=Inter&display=swap")
        );
        let variable = google_request("Inter", &["variable"], &[]);
        assert_eq!(
            google_css2_url(&variable).as_deref(),
            Some("https://fonts.googleapis.com/css2?family=Inter&display=swap")
        );
        let mono = google_request("Roboto Mono", &["400", "700"], &[]);
        assert_eq!(
            google_css2_url(&mono).as_deref(),
            Some("https://fonts.googleapis.com/css2?family=Roboto+Mono:wght@400;700&display=swap")
        );
        let italic = google_request("Inter", &["700", "400"], &["normal", "italic"]);
        assert_eq!(
            google_css2_url(&italic).as_deref(),
            Some(
                "https://fonts.googleapis.com/css2?family=Inter:ital,wght@0,400;0,700;1,400;1,700&display=swap"
            )
        );
        let mut range = google_request("Inter", &["100 900"], &[]);
        range.display = Some(String::from("optional"));
        assert_eq!(
            google_css2_url(&range).as_deref(),
            Some("https://fonts.googleapis.com/css2?family=Inter:wght@100..900&display=optional")
        );
        let variable_italic = google_request("Inter", &["variable"], &["normal", "italic"]);
        assert_eq!(
            google_css2_url(&variable_italic).as_deref(),
            Some("https://fonts.googleapis.com/css2?family=Inter:ital@0;1&display=swap")
        );
        let italic_only = google_request("Lora", &["400"], &["italic"]);
        assert_eq!(
            google_css2_url(&italic_only).as_deref(),
            Some("https://fonts.googleapis.com/css2?family=Lora:ital,wght@1,400&display=swap")
        );
        let local = &fixture_requests()[2];
        assert_eq!(google_css2_url(local), None);
    }

    #[test]
    fn rewrites_google_css_and_collects_assets() {
        let inter = &fixture_requests()[0];
        let (css, assets) = rewrite_font_css(GOOGLE_CSS, inter, "/_yolo/fonts/");
        assert_eq!(assets.len(), 2);
        assert!(!css.contains("fonts.gstatic.com"));
        let cyrillic = &assets[0];
        assert_eq!(cyrillic.subset.as_deref(), Some("cyrillic-ext"));
        assert!(!cyrillic.preload);
        assert_eq!(cyrillic.family, "Inter");
        assert_eq!(cyrillic.weight, "400");
        assert_eq!(cyrillic.style, "normal");
        assert_eq!(cyrillic.format, "woff2");
        assert!(cyrillic.local_path.starts_with("/_yolo/fonts/inter-"));
        assert!(
            Path::new(&cyrillic.local_path)
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("woff2"))
        );
        assert_eq!(
            cyrillic.local_path.len(),
            "/_yolo/fonts/inter-".len() + 12 + ".woff2".len()
        );
        assert!(
            cyrillic
                .unicode_range
                .as_deref()
                .is_some_and(|r| r.starts_with("U+0460-052F"))
        );
        let latin = &assets[1];
        assert_eq!(latin.subset.as_deref(), Some("latin"));
        assert!(latin.preload);
        assert!(
            latin
                .unicode_range
                .as_deref()
                .is_some_and(|r| r.starts_with("U+0000-00FF"))
        );
        assert!(css.contains(&format!("src: url({}) format('woff2')", latin.local_path)));
        assert_ne!(cyrillic.local_path, latin.local_path);

        let mut no_preload = inter.clone();
        no_preload.preload = Some(false);
        let (_, assets) = rewrite_font_css(GOOGLE_CSS, &no_preload, "/_yolo/fonts");
        assert!(assets.iter().all(|a| !a.preload));
    }

    /// Builds a minimal TrueType font: head, hhea, OS/2, cmap (format 4), hmtx.
    fn synthetic_font() -> Vec<u8> {
        fn u16s(values: &[u16]) -> Vec<u8> {
            values.iter().flat_map(|v| v.to_be_bytes()).collect()
        }
        let mut head = vec![0u8; 54];
        head[18..20].copy_from_slice(&1000u16.to_be_bytes());
        let mut hhea = vec![0u8; 36];
        hhea[4..6].copy_from_slice(&900i16.to_be_bytes());
        hhea[6..8].copy_from_slice(&(-200i16).to_be_bytes());
        hhea[8..10].copy_from_slice(&0i16.to_be_bytes());
        hhea[34..36].copy_from_slice(&3u16.to_be_bytes());
        let mut os2 = vec![0u8; 96];
        os2[0..2].copy_from_slice(&4u16.to_be_bytes());
        os2[2..4].copy_from_slice(&500i16.to_be_bytes());
        os2[88..90].copy_from_slice(&700i16.to_be_bytes());
        // cmap: one (3,1) format 4 subtable; ' ' -> glyph 1, 'a'..'z' -> glyphs 2..27.
        let mut sub = u16s(&[4, 0, 0, 6, 0, 0, 0]);
        sub.extend(u16s(&[0x20, 0x7A, 0xFFFF]));
        sub.extend(u16s(&[0]));
        sub.extend(u16s(&[0x20, 0x61, 0xFFFF]));
        sub.extend(u16s(&[1u16.wrapping_sub(0x20), 2u16.wrapping_sub(0x61), 1]));
        sub.extend(u16s(&[0, 0, 0]));
        let length = u16::try_from(sub.len()).expect("subtable length");
        sub[2..4].copy_from_slice(&length.to_be_bytes());
        let mut cmap = u16s(&[0, 1, 3, 1]);
        cmap.extend(12u32.to_be_bytes());
        cmap.extend(sub);
        let hmtx = u16s(&[1000, 0, 250, 0, 500, 0]);

        let tables: [(&[u8; 4], Vec<u8>); 5] = [
            (b"OS/2", os2),
            (b"cmap", cmap),
            (b"head", head),
            (b"hhea", hhea),
            (b"hmtx", hmtx),
        ];
        let mut font = Vec::new();
        font.extend([0u8, 1, 0, 0]);
        font.extend(u16s(&[5, 64, 2, 16]));
        let mut offset = 12 + 16 * tables.len();
        let mut body = Vec::new();
        for (tag, data) in &tables {
            font.extend_from_slice(*tag);
            font.extend(0u32.to_be_bytes());
            font.extend(u32::try_from(offset).expect("offset").to_be_bytes());
            font.extend(u32::try_from(data.len()).expect("length").to_be_bytes());
            let padded = data.len().div_ceil(4) * 4;
            offset += padded;
            body.extend_from_slice(data);
            body.extend(std::iter::repeat_n(0u8, padded - data.len()));
        }
        font.extend(body);
        font
    }

    #[test]
    fn parses_synthetic_sfnt_metrics() {
        let bytes = synthetic_font();
        let metrics = parse_sfnt_metrics(&bytes, "Synthetic").expect("metrics");
        assert_eq!(metrics.family, "Synthetic");
        assert_eq!(metrics.units_per_em, 1000);
        assert_eq!(metrics.ascent, 900);
        assert_eq!(metrics.descent, -200);
        assert_eq!(metrics.line_gap, 0);
        assert_eq!(metrics.cap_height, Some(700));
        assert_eq!(metrics.x_avg_char_width, 500);
        // 37 letters at 500 + 6 spaces at 250 over 43 glyphs.
        let expected = 20_000.0 / 43.0;
        let az = metrics.az_avg_width.expect("az average");
        assert!((az - expected).abs() < 1e-9, "{az}");

        // Collections point at the first font; truncated input never panics.
        let mut ttc = Vec::new();
        ttc.extend_from_slice(b"ttcf");
        ttc.extend(0x0001_0000u32.to_be_bytes());
        ttc.extend(1u32.to_be_bytes());
        ttc.extend(16u32.to_be_bytes());
        // TTC table offsets are absolute: shift every record past the 16-byte header.
        let mut shifted = bytes.clone();
        for index in 0..5 {
            let at = 12 + index * 16 + 8;
            let offset = u32::from_be_bytes(shifted[at..at + 4].try_into().expect("offset")) + 16;
            shifted[at..at + 4].copy_from_slice(&offset.to_be_bytes());
        }
        ttc.extend(&shifted);
        assert_eq!(parse_sfnt_metrics(&ttc, "TTC").map(|m| m.ascent), Some(900));
        for cut in [0, 3, 12, 40, 100] {
            let _ = parse_sfnt_metrics(&bytes[..cut.min(bytes.len())], "cut");
        }
        assert_eq!(parse_sfnt_metrics(b"wOFF\0\0\0\0", "woff"), None);
        assert_eq!(parse_sfnt_metrics(b"not a font at all", "text"), None);
    }

    #[test]
    fn fallback_math_matches_next_font() {
        let main = FontMetrics {
            family: String::from("Main"),
            units_per_em: 1000,
            ascent: 900,
            descent: -200,
            line_gap: 0,
            cap_height: None,
            x_avg_char_width: 500,
            az_avg_width: None,
        };
        let overrides = fallback_overrides(&main, &arial_metrics());
        assert_eq!(overrides.fallback_family, "Arial");
        assert_eq!(overrides.size_adjust, "113.27%");
        assert_eq!(overrides.ascent_override, "79.45%");
        assert_eq!(overrides.descent_override, "17.66%");
        assert_eq!(overrides.line_gap_override, "0.00%");

        let same = fallback_overrides(&arial_metrics(), &arial_metrics());
        assert_eq!(same.size_adjust, "100.00%");
        assert_eq!(same.ascent_override, "90.53%");
        assert_eq!(same.descent_override, "21.19%");

        let az_main = FontMetrics {
            family: String::from("Az"),
            units_per_em: 2000,
            ascent: 1800,
            descent: -400,
            line_gap: 100,
            cap_height: None,
            x_avg_char_width: 1,
            az_avg_width: Some(1000.0),
        };
        let az = fallback_overrides(&az_main, &arial_metrics());
        assert_eq!(az.size_adjust, "109.58%");
        assert_eq!(az.ascent_override, "82.13%");

        let times = fallback_overrides(&main, &times_new_roman_metrics());
        assert_eq!(times.fallback_family, "Times New Roman");
        assert_eq!(times.size_adjust, "124.73%");
    }

    #[test]
    fn fallback_font_selection() {
        let mut inter = google_request("Inter", &[], &[]);
        assert_eq!(fallback_font_for(&inter, None).family, "Arial");
        inter.adjust_font_fallback = Some(Value::String(String::from("Times New Roman")));
        assert_eq!(fallback_font_for(&inter, None).family, "Times New Roman");
        inter.adjust_font_fallback = Some(Value::Bool(false));
        assert_eq!(fallback_font_for(&inter, None).family, "Arial");
        assert_eq!(
            fallback_font_for(&google_request("Playfair Display", &[], &[]), None).family,
            "Times New Roman"
        );
        assert_eq!(
            fallback_font_for(&google_request("Roboto Serif", &[], &[]), None).family,
            "Times New Roman"
        );
        assert_eq!(
            fallback_font_for(&google_request("Roboto Mono", &[], &[]), None).family,
            "Arial"
        );
        assert_eq!(
            fallback_font_for(&google_request("Noto Sans", &[], &[]), None).family,
            "Arial"
        );
        let metrics = FontMetrics { family: String::from("EB Garamond"), ..arial_metrics() };
        assert_eq!(
            fallback_font_for(&google_request("Custom", &[], &[]), Some(&metrics)).family,
            "Times New Roman"
        );
    }

    #[test]
    fn plans_google_fonts() {
        let requests = fixture_requests();
        let mono = plan_font(&requests[1], "/_yolo/fonts");
        assert!(mono.class_name.starts_with("yolo-font-"));
        assert_eq!(mono.class_name.len(), "yolo-font-".len() + 8);
        let suffix = mono.class_name.trim_start_matches("yolo-font-").to_owned();
        assert_eq!(
            mono.variable_class.as_deref(),
            Some(format!("yolo-font-var-{suffix}").as_str())
        );
        assert_eq!(mono.family_css, "'Roboto Mono', 'Roboto Mono Fallback'");
        assert_eq!(
            mono.css2_url.as_deref(),
            Some("https://fonts.googleapis.com/css2?family=Roboto+Mono:wght@400;700&display=swap")
        );
        assert!(mono.assets.is_empty());
        assert_eq!(mono.fallback, None);
        assert_eq!(mono.preset_role.as_deref(), Some("mono"));
        assert!(mono.css.contains(&format!(
            ".{} {{ font-family: 'Roboto Mono', 'Roboto Mono Fallback'; }}",
            mono.class_name
        )));
        assert!(mono.css.contains(&format!(
            ".yolo-font-var-{suffix} {{ --font-mono: 'Roboto Mono', 'Roboto Mono Fallback'; }}"
        )));
        assert!(!mono.css.contains("font-weight"));

        let inter = plan_font(&requests[0], "/_yolo/fonts");
        assert_eq!(inter.preset_role.as_deref(), Some("sans"));
        assert_ne!(inter.class_name, mono.class_name);
        assert_eq!(
            plan_font(&requests[0], "/_yolo/fonts").class_name,
            inter.class_name
        );

        let mut single = google_request("Inter", &["400"], &["normal"]);
        single.fallback = vec![String::from("system-ui"), String::from("arial")];
        single.adjust_font_fallback = Some(Value::Bool(false));
        let plan = plan_font(&single, "/_yolo/fonts");
        assert_eq!(plan.family_css, "'Inter', system-ui, arial");
        assert!(plan.css.contains(
            "font-family: 'Inter', system-ui, arial; font-weight: 400; font-style: normal;"
        ));
        assert_eq!(plan.variable_class, None);
    }

    #[test]
    fn plans_local_fonts_with_font_faces() {
        let requests = fixture_requests();
        let cal = plan_font(&requests[2], "/_yolo/fonts/");
        assert_eq!(cal.css2_url, None);
        assert_eq!(cal.family_css, "'CalSans', 'CalSans Fallback'");
        assert_eq!(cal.preset_role.as_deref(), Some("display"));
        assert_eq!(cal.assets.len(), 1);
        assert_eq!(
            cal.assets[0].source,
            "../public/fonts/CalSans-SemiBold.woff2"
        );
        assert_eq!(
            cal.assets[0].local_path,
            "/_yolo/fonts/CalSans-SemiBold.woff2"
        );
        assert_eq!(cal.assets[0].weight, "600");
        assert_eq!(cal.assets[0].format, "woff2");
        assert!(cal.assets[0].preload);
        assert!(cal.css.contains("@font-face {\n  font-family: 'CalSans';\n  src: url('/_yolo/fonts/CalSans-SemiBold.woff2') format('woff2');\n  font-display: swap;\n  font-weight: 600;\n  font-style: normal;\n}"));
        assert!(cal.css.contains(&format!(
            ".{} {{ font-family: 'CalSans', 'CalSans Fallback'; font-weight: 600; font-style: normal; }}",
            cal.class_name
        )));
        assert!(
            cal.css
                .contains("--font-display: 'CalSans', 'CalSans Fallback'")
        );

        let mut named = requests[2].clone();
        named.declarations = vec![(String::from("font-family"), String::from("'Cal Sans'"))];
        named.adjust_font_fallback = Some(Value::Bool(false));
        let plan = plan_font(&named, "/_yolo/fonts");
        assert_eq!(plan.family_css, "'Cal Sans'");
        assert!(plan.css.contains("font-family: 'Cal Sans';"));
    }

    #[test]
    fn fallback_face_is_emitted_when_known() {
        let request = google_request("Inter", &["400"], &[]);
        let mut plan = plan_font(&request, "/_yolo/fonts");
        let main = FontMetrics {
            family: String::from("Inter"),
            units_per_em: 2816,
            ascent: 2728,
            descent: -680,
            line_gap: 0,
            cap_height: Some(2048),
            x_avg_char_width: 1500,
            az_avg_width: None,
        };
        plan.fallback = Some(fallback_overrides(
            &main,
            &fallback_font_for(&request, Some(&main)),
        ));
        let css = font_class_css(&plan);
        assert!(css.contains(
            "@font-face { font-family: 'Inter Fallback'; src: local('Arial'); ascent-override: "
        ));
        assert!(css.contains("size-adjust: "));
        assert!(css.contains("; }"));
    }

    #[test]
    fn manifest_shape_and_preset_stacks() {
        let requests = fixture_requests();
        let plans: Vec<FontPlan> = requests
            .iter()
            .map(|r| plan_font(r, "/_yolo/fonts"))
            .collect();
        let manifest = fonts_manifest_json(&plans);
        let fonts = manifest["fonts"].as_array().expect("fonts array");
        assert_eq!(fonts.len(), 3);
        let inter = &fonts[0];
        for key in [
            "binding",
            "family",
            "css2Url",
            "userAgent",
            "className",
            "variableClass",
            "variable",
            "familyCss",
            "preload",
            "subsets",
            "weights",
            "styles",
            "display",
            "assets",
            "css",
        ] {
            assert!(inter.get(key).is_some(), "missing key {key}");
        }
        assert_eq!(inter["family"], "Inter");
        assert_eq!(inter["userAgent"], GOOGLE_FONTS_USER_AGENT);
        assert_eq!(inter["preload"], true);
        assert_eq!(inter["display"], "swap");
        assert_eq!(fonts[2]["userAgent"], Value::Null);
        assert_eq!(
            fonts[2]["assets"][0]["localPath"],
            "/_yolo/fonts/CalSans-SemiBold.woff2"
        );
        assert!(GOOGLE_FONTS_USER_AGENT.contains("Chrome/104.0.0.0"));

        for preset in YoloThemePreset::ALL {
            let (sans, mono) = preset_font_stack(preset);
            assert!(!sans.is_empty());
            assert!(!mono.is_empty());
        }
    }

    #[test]
    fn fixture_layout_when_present() {
        let path =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../apps/next-fixture/app/layout.tsx");
        let Ok(source) = std::fs::read_to_string(&path) else {
            return;
        };
        let requests = extract_font_requests(&source, "app/layout.tsx");
        assert_fixture_requests(&requests);
        let plans: Vec<FontPlan> = requests
            .iter()
            .map(|r| plan_font(r, "/_yolo/fonts"))
            .collect();
        assert_eq!(
            plans[0].css2_url.as_deref(),
            Some("https://fonts.googleapis.com/css2?family=Inter&display=swap")
        );
        assert_eq!(plans[2].assets.len(), 1);
    }
}
