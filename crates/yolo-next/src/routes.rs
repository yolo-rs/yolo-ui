// SPDX-License-Identifier: Apache-2.0
//! App Router (`app/`) and Pages Router (`pages/`) file-system scanner and route matcher.
//!
//! Conventions verified against Next.js 16.4.5 (`packages/next/src/build/entries.ts`,
//! `shared/lib/router/utils/{sorted-routes,route-regex,app-paths}.ts`,
//! `lib/metadata/is-metadata-route.ts`):
//!
//! - `[slug]` dynamic, `[...slug]` catch-all, `[[...slug]]` optional catch-all;
//! - `(group)` and `@slot` contribute nothing to the URL, `_private` is skipped;
//! - `(.)x`, `(..)x`, `(..)(..)x`, `(...)x` intercept `x` 0, 1, 2 route segments above or at the root;
//! - `page`, `layout`, `template`, `loading`, `error`, `global-error`, `not-found`, `forbidden`,
//!   `unauthorized`, `default` and `route` special files plus the metadata file conventions.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use serde_json::{Map, Value, json};
use walkdir::WalkDir;

use crate::segment;
use crate::types::{
    Diagnostic, InterceptSpec, MetadataFile, MetadataFileKind, NextConfig, RouteEntry, RouteKind,
    RouteManifest, RouteParam, RouteSegment, SegmentFiles, SegmentKind,
};

/// A matched route with captured params.
#[derive(Debug, Clone, PartialEq)]
pub struct RouteMatch<'a> {
    /// Matched entry.
    pub entry: &'a RouteEntry,
    /// Captured params (`slug` -> value; catch-all values joined with `/`).
    pub params: BTreeMap<String, String>,
}

/// Extensions of route handlers and non-page special files.
const ROUTE_EXTS: [&str; 4] = ["tsx", "ts", "jsx", "js"];
/// Extensions of `page.*` and `layout.*` (MDX allowed when `@next/mdx` is configured).
const PAGE_EXTS: [&str; 6] = ["tsx", "ts", "jsx", "js", "mdx", "md"];
/// HTTP methods served by a Pages Router API route (one handler for every method).
const PAGES_API_METHODS: [&str; 7] = ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"];

/// Normalizes a path to `/` separators.
fn norm(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// Path of `path` relative to `root` with `/` separators.
fn rel_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root).map_or_else(|_| norm(path), norm)
}

// ============================================================================
// Segment classification
// ============================================================================

/// Classification of one directory name.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Classified {
    kind: SegmentKind,
    param: Option<String>,
    intercept_level: Option<u8>,
    intercept_target: Option<String>,
}

impl Classified {
    fn of(name: &str) -> Self {
        let (kind, param, intercept_level, intercept_target) = classify_segment(name);
        Self { kind, param, intercept_level, intercept_target }
    }
}

/// Classifies a directory name: kind, param, intercept level, intercept target.
///
/// `(.)photos` -> intercept level 0 of `photos`; `(..)[id]` -> level 1 of the dynamic segment
/// `[id]` (param `id`); `[...slug]` -> catch-all; `[[...slug]]` -> optional catch-all;
/// `(marketing)` -> group; `@modal` -> parallel slot; `_lib` -> private; anything else static.
pub fn classify_segment(name: &str) -> (SegmentKind, Option<String>, Option<u8>, Option<String>) {
    for (prefix, level) in [("(...)", 3u8), ("(..)(..)", 2), ("(..)", 1), ("(.)", 0)] {
        if let Some(rest) = name.strip_prefix(prefix)
            && !rest.is_empty()
        {
            let (_, param, _, _) = classify_segment(rest);
            return (
                SegmentKind::Intercept,
                param,
                Some(level),
                Some(rest.to_owned()),
            );
        }
    }
    if let Some(inner) = name
        .strip_prefix("[[...")
        .and_then(|s| s.strip_suffix("]]"))
    {
        return (
            SegmentKind::OptionalCatchAll,
            Some(inner.to_owned()),
            None,
            None,
        );
    }
    if let Some(inner) = name.strip_prefix("[...").and_then(|s| s.strip_suffix(']')) {
        return (SegmentKind::CatchAll, Some(inner.to_owned()), None, None);
    }
    if let Some(inner) = name.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
        return (SegmentKind::Dynamic, Some(inner.to_owned()), None, None);
    }
    if name.len() > 1 && name.starts_with('(') && name.ends_with(')') {
        return (SegmentKind::Group, None, None, None);
    }
    if name.len() > 1 && name.starts_with('@') {
        return (SegmentKind::ParallelSlot, None, None, None);
    }
    if name.starts_with('_') {
        return (SegmentKind::Private, None, None, None);
    }
    (SegmentKind::Static, None, None, None)
}

/// Applies one directory name to a URL segment list (Next.js bracket form) and returns its kind.
///
/// Groups and slots add nothing; intercepting segments drop `level` route segments
/// (all of them for `(...)`) and append the intercepted segment.
fn apply_segment_name(url: &mut Vec<String>, name: &str) -> SegmentKind {
    let cls = Classified::of(name);
    match cls.kind {
        SegmentKind::Static
        | SegmentKind::Dynamic
        | SegmentKind::CatchAll
        | SegmentKind::OptionalCatchAll => url.push(name.to_owned()),
        SegmentKind::Intercept => {
            let keep = match cls.intercept_level.unwrap_or(0) {
                0 => url.len(),
                1 => url.len().saturating_sub(1),
                2 => url.len().saturating_sub(2),
                _ => 0,
            };
            url.truncate(keep);
            if let Some(target) = cls.intercept_target {
                url.push(target);
            }
        }
        SegmentKind::Root
        | SegmentKind::Group
        | SegmentKind::ParallelSlot
        | SegmentKind::Private => {}
    }
    cls.kind
}

/// Joins URL segments into a pattern (`/` for the root).
fn join_url(parts: &[String]) -> String {
    if parts.is_empty() {
        String::from("/")
    } else {
        format!("/{}", parts.join("/"))
    }
}

/// Non-empty segments of a URL pattern.
fn url_segments(url_pattern: &str) -> Vec<&str> {
    url_pattern.split('/').filter(|s| !s.is_empty()).collect()
}

/// Dynamic params of a URL pattern, in order.
fn params_of(url_pattern: &str) -> Vec<RouteParam> {
    url_segments(url_pattern)
        .into_iter()
        .filter_map(|segment| {
            let (kind, param, _, _) = classify_segment(segment);
            param.map(|name| RouteParam {
                name,
                catch_all: matches!(kind, SegmentKind::CatchAll | SegmentKind::OptionalCatchAll),
                optional: kind == SegmentKind::OptionalCatchAll,
            })
        })
        .collect()
}

/// URL pattern (bracket form) of the directory containing `file` (`""` for the app root).
///
/// `app/(marketing)/layout.tsx` -> `""`; `app/blog/layout.tsx` -> `/blog`;
/// `app/blog/[slug]/layout.tsx` -> `/blog/[slug]`.
pub(crate) fn file_url_prefix(app_dir: &str, file: &str) -> String {
    let mut url = Vec::new();
    for part in app_relative_dir_parts(app_dir, file) {
        apply_segment_name(&mut url, &part);
    }
    if url.is_empty() {
        String::new()
    } else {
        format!("/{}", url.join("/"))
    }
}

/// Raw directory names of `file` below the app directory (`app/(marketing)/pricing/page.tsx`
/// -> `["(marketing)", "pricing"]`).
pub(crate) fn app_relative_dir_parts(app_dir: &str, file: &str) -> Vec<String> {
    let rel = file
        .strip_prefix(app_dir)
        .unwrap_or(file)
        .trim_start_matches('/');
    let parts: Vec<&str> = rel.split('/').filter(|s| !s.is_empty()).collect();
    parts[..parts.len().saturating_sub(1)]
        .iter()
        .map(|s| (*s).to_owned())
        .collect()
}

// ============================================================================
// Pattern conversions
// ============================================================================

/// Maps every segment of a URL pattern through `f` (static segments are kept verbatim).
fn convert_pattern(url_pattern: &str, f: impl Fn(SegmentKind, &str, &str) -> String) -> String {
    let segments = url_segments(url_pattern);
    if segments.is_empty() {
        return String::from("/");
    }
    let converted: Vec<String> = segments
        .into_iter()
        .map(|segment| {
            let (kind, param, _, _) = classify_segment(segment);
            match kind {
                SegmentKind::Dynamic | SegmentKind::CatchAll | SegmentKind::OptionalCatchAll => {
                    f(kind, segment, param.as_deref().unwrap_or(""))
                }
                _ => segment.to_owned(),
            }
        })
        .collect();
    format!("/{}", converted.join("/"))
}

/// `/blog/[slug]` -> `/blog/:slug`; `[...slug]`/`[[...slug]]` -> `/*`.
///
/// Optional catch-alls (`/shop/[[...filters]]` -> `/shop/*`) must also be registered as the
/// bare parent path (`/shop`) by the Bun emitter.
pub fn to_bun_pattern(url_pattern: &str) -> String {
    convert_pattern(url_pattern, |kind, _, param| match kind {
        SegmentKind::Dynamic => format!(":{param}"),
        _ => String::from("*"),
    })
}

/// `/blog/[slug]` -> `/blog/$slug`; catch-all -> `/docs/$`.
pub fn to_tanstack_path(url_pattern: &str) -> String {
    convert_pattern(url_pattern, |kind, _, param| match kind {
        SegmentKind::Dynamic => format!("${param}"),
        _ => String::from("$"),
    })
}

/// `/blog/[slug]` -> `/blog/:slug`; catch-all -> `/docs/:..slug`.
pub fn to_dioxus_route(url_pattern: &str) -> String {
    convert_pattern(url_pattern, |kind, _, param| match kind {
        SegmentKind::Dynamic => format!(":{param}"),
        _ => format!(":..{param}"),
    })
}

/// `PascalCase` of a static segment or param name (`opengraph-image` -> `OpengraphImage`).
fn pascal(word: &str) -> String {
    let mut out = String::new();
    let mut upper = true;
    for c in word.chars() {
        if c.is_ascii_alphanumeric() {
            if upper {
                out.extend(c.to_uppercase());
            } else {
                out.push(c);
            }
            upper = false;
        } else {
            upper = true;
        }
    }
    out
}

/// `/blog/[slug]` -> `BlogSlug`; `/docs/[...slug]` -> `DocsSlugCatchAll`;
/// `/shop/[[...filters]]` -> `ShopFiltersOptional`; `/` -> `Home`.
pub fn to_dioxus_variant(url_pattern: &str) -> String {
    let segments = url_segments(url_pattern);
    if segments.is_empty() {
        return String::from("Home");
    }
    let mut name = String::new();
    for segment in segments {
        let (kind, param, _, _) = classify_segment(segment);
        match kind {
            SegmentKind::Dynamic => name.push_str(&pascal(param.as_deref().unwrap_or(""))),
            SegmentKind::CatchAll => {
                name.push_str(&pascal(param.as_deref().unwrap_or("")));
                name.push_str("CatchAll");
            }
            SegmentKind::OptionalCatchAll => {
                name.push_str(&pascal(param.as_deref().unwrap_or("")));
                name.push_str("Optional");
            }
            _ => name.push_str(&pascal(segment)),
        }
    }
    if name.is_empty() {
        name.push_str("Route");
    }
    if name.starts_with(|c: char| c.is_ascii_digit()) {
        name.insert_str(0, "Route");
    }
    name
}

/// `/blog/[slug]` -> `/blog/:slug`; catch-all -> `/docs/*slug`.
pub fn to_leptos_path(url_pattern: &str) -> String {
    convert_pattern(url_pattern, |kind, _, param| match kind {
        SegmentKind::Dynamic => format!(":{param}"),
        _ => format!("*{param}"),
    })
}

// ============================================================================
// File classification
// ============================================================================

/// Role of a file inside a segment directory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FileRole {
    Page,
    Layout,
    Template,
    Loading,
    Error,
    GlobalError,
    NotFound,
    Forbidden,
    Unauthorized,
    Default,
    Route,
    Metadata(MetadataFileKind, bool),
}

/// `icon`, `icon1`, `icon42` ...
fn numbered(stem: &str, base: &str) -> bool {
    stem.strip_prefix(base)
        .is_some_and(|rest| rest.chars().all(|c| c.is_ascii_digit()))
}

fn classify_metadata_file(stem: &str, ext: &str, is_root: bool) -> Option<FileRole> {
    let generated = ROUTE_EXTS.contains(&ext);
    let image = matches!(ext, "jpg" | "jpeg" | "png" | "gif");
    let kind = match stem {
        "sitemap" if generated || ext == "xml" => MetadataFileKind::Sitemap,
        "robots" if generated || ext == "txt" => MetadataFileKind::Robots,
        "manifest" if generated || matches!(ext, "json" | "webmanifest") => {
            MetadataFileKind::Manifest
        }
        "favicon" if is_root && ext == "ico" => MetadataFileKind::Favicon,
        _ if numbered(stem, "apple-icon")
            && (generated || matches!(ext, "jpg" | "jpeg" | "png")) =>
        {
            MetadataFileKind::AppleIcon
        }
        _ if numbered(stem, "icon")
            && (generated || matches!(ext, "ico" | "jpg" | "jpeg" | "png" | "svg")) =>
        {
            MetadataFileKind::Icon
        }
        _ if numbered(stem, "opengraph-image") && (generated || image) => {
            MetadataFileKind::OpengraphImage
        }
        _ if numbered(stem, "twitter-image") && (generated || image) => {
            MetadataFileKind::TwitterImage
        }
        _ => return None,
    };
    Some(FileRole::Metadata(kind, generated))
}

/// Classifies a file name inside a segment directory.
fn classify_file(file_name: &str, is_root: bool) -> Option<FileRole> {
    let (stem, ext) = file_name.rsplit_once('.')?;
    let ext = ext.to_ascii_lowercase();
    let ext = ext.as_str();
    let role = match stem {
        "page" if PAGE_EXTS.contains(&ext) => FileRole::Page,
        "layout" if PAGE_EXTS.contains(&ext) => FileRole::Layout,
        "template" if ROUTE_EXTS.contains(&ext) => FileRole::Template,
        "loading" if ROUTE_EXTS.contains(&ext) => FileRole::Loading,
        "error" if ROUTE_EXTS.contains(&ext) => FileRole::Error,
        "global-error" if ROUTE_EXTS.contains(&ext) => FileRole::GlobalError,
        "not-found" if ROUTE_EXTS.contains(&ext) => FileRole::NotFound,
        "forbidden" if ROUTE_EXTS.contains(&ext) => FileRole::Forbidden,
        "unauthorized" if ROUTE_EXTS.contains(&ext) => FileRole::Unauthorized,
        "default" if ROUTE_EXTS.contains(&ext) => FileRole::Default,
        "route" if ROUTE_EXTS.contains(&ext) => FileRole::Route,
        _ => return classify_metadata_file(stem, ext, is_root),
    };
    Some(role)
}

/// Stores a special file, warning when the slot is already taken (`page.tsx` + `page.js`).
fn assign_file(files: &mut SegmentFiles, role: FileRole, rel: String, diags: &mut Vec<Diagnostic>) {
    let slot = match role {
        FileRole::Page => &mut files.page,
        FileRole::Layout => &mut files.layout,
        FileRole::Template => &mut files.template,
        FileRole::Loading => &mut files.loading,
        FileRole::Error => &mut files.error,
        FileRole::GlobalError => &mut files.global_error,
        FileRole::NotFound => &mut files.not_found,
        FileRole::Forbidden => &mut files.forbidden,
        FileRole::Unauthorized => &mut files.unauthorized,
        FileRole::Default => &mut files.default,
        FileRole::Route => &mut files.route,
        FileRole::Metadata(kind, generated) => {
            files
                .metadata
                .push(MetadataFile { kind, path: rel, generated });
            return;
        }
    };
    if let Some(existing) = slot {
        diags.push(
            Diagnostic::warning(
                "route-file-duplicate",
                format!("`{rel}` ignored: `{existing}` already provides this special file"),
            )
            .with_file(rel),
        );
    } else {
        *slot = Some(rel);
    }
}

/// Direct children of `dir` sorted by file name.
fn list_dir(dir: &Path) -> Vec<walkdir::DirEntry> {
    WalkDir::new(dir)
        .min_depth(1)
        .max_depth(1)
        .sort_by_file_name()
        .into_iter()
        .filter_map(Result::ok)
        .collect()
}

/// Builds the segment node of `dir` recursively.
fn build_segment(
    root: &Path,
    dir: &Path,
    name: &str,
    cls: Classified,
    diags: &mut Vec<Diagnostic>,
) -> RouteSegment {
    let is_root = cls.kind == SegmentKind::Root;
    let mut files = SegmentFiles::default();
    let mut children = Vec::new();
    for entry in list_dir(dir) {
        let file_name = entry.file_name().to_string_lossy().into_owned();
        if entry.file_type().is_dir() {
            if file_name == "node_modules" || file_name.starts_with('.') {
                continue;
            }
            let child = Classified::of(&file_name);
            if child.kind == SegmentKind::Private {
                continue;
            }
            children.push(build_segment(root, entry.path(), &file_name, child, diags));
        } else if entry.file_type().is_file()
            && let Some(role) = classify_file(&file_name, is_root)
        {
            assign_file(&mut files, role, rel_path(root, entry.path()), diags);
        }
    }
    RouteSegment {
        name: name.to_owned(),
        kind: cls.kind,
        param: cls.param,
        intercept_level: cls.intercept_level,
        intercept_target: cls.intercept_target,
        files,
        children,
    }
}

// ============================================================================
// Entry collection
// ============================================================================

/// Inherited state while walking the segment tree.
#[derive(Debug, Clone, Default)]
struct Chain {
    url: Vec<String>,
    app_path: Vec<String>,
    layouts: Vec<String>,
    templates: Vec<String>,
    loading: Option<String>,
    error: Option<String>,
    not_found: Option<String>,
    slot: Option<String>,
    intercept_level: Option<u8>,
    layout_slots: Vec<String>,
}

/// What to build an entry from.
struct EntrySpec<'a> {
    kind: RouteKind,
    file: String,
    url_pattern: String,
    app_path: String,
    chain: &'a Chain,
    metadata_kind: Option<MetadataFileKind>,
}

/// Builds one entry (reads and analyzes the module).
fn build_entry(root: &Path, spec: EntrySpec<'_>) -> RouteEntry {
    let source = fs::read_to_string(root.join(&spec.file)).unwrap_or_default();
    let segment_config = segment::analyze_segment_module(&source, &spec.file);
    let methods = if spec.kind == RouteKind::RouteHandler {
        segment::route_handler_methods(&source)
    } else {
        Vec::new()
    };
    let intercepting = spec
        .chain
        .intercept_level
        .map(|level| InterceptSpec { level, target: spec.url_pattern.clone() });
    RouteEntry {
        bun_pattern: to_bun_pattern(&spec.url_pattern),
        tanstack_path: to_tanstack_path(&spec.url_pattern),
        dioxus_route: to_dioxus_route(&spec.url_pattern),
        dioxus_variant: to_dioxus_variant(&spec.url_pattern),
        leptos_path: to_leptos_path(&spec.url_pattern),
        params: params_of(&spec.url_pattern),
        app_path: spec.app_path,
        url_pattern: spec.url_pattern,
        kind: spec.kind,
        file: spec.file,
        layouts: spec.chain.layouts.clone(),
        templates: spec.chain.templates.clone(),
        loading: spec.chain.loading.clone(),
        error: spec.chain.error.clone(),
        not_found: spec.chain.not_found.clone(),
        parallel_slots: spec.chain.layout_slots.clone(),
        slot: spec.chain.slot.clone(),
        intercepting,
        segment_config,
        methods,
        metadata_kind: spec.metadata_kind,
        legacy_pages: false,
        legacy_data_functions: Vec::new(),
    }
}

/// Public path name of a generated metadata route.
fn metadata_route_name(kind: MetadataFileKind, path: &str) -> String {
    match kind {
        MetadataFileKind::Sitemap => String::from("sitemap.xml"),
        MetadataFileKind::Robots => String::from("robots.txt"),
        MetadataFileKind::Manifest => String::from("manifest.webmanifest"),
        MetadataFileKind::Favicon => String::from("favicon.ico"),
        MetadataFileKind::Icon
        | MetadataFileKind::AppleIcon
        | MetadataFileKind::OpengraphImage
        | MetadataFileKind::TwitterImage => Path::new(path)
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default(),
    }
}

/// Appends `name` to a URL or app path prefix.
fn child_path(prefix: &str, name: &str) -> String {
    if prefix == "/" {
        format!("/{name}")
    } else {
        format!("{prefix}/{name}")
    }
}

/// Walks the tree and produces entries.
fn collect_entries(
    segment: &RouteSegment,
    mut chain: Chain,
    root: &Path,
    out: &mut Vec<RouteEntry>,
    diags: &mut Vec<Diagnostic>,
) {
    if segment.kind != SegmentKind::Root {
        let kind = apply_segment_name(&mut chain.url, &segment.name);
        match kind {
            SegmentKind::Private => return,
            SegmentKind::ParallelSlot => {
                if chain.slot.is_none() {
                    chain.slot = Some(segment.name.trim_start_matches('@').to_owned());
                }
            }
            SegmentKind::Intercept => {
                chain.intercept_level = segment.intercept_level.or(Some(0));
            }
            _ => {}
        }
        chain.app_path.push(segment.name.clone());
    }
    let files = &segment.files;
    if let Some(layout) = &files.layout {
        chain.layouts.push(layout.clone());
        chain.layout_slots = segment
            .children
            .iter()
            .filter(|c| c.kind == SegmentKind::ParallelSlot)
            .map(|c| c.name.trim_start_matches('@').to_owned())
            .collect();
    }
    if let Some(template) = &files.template {
        chain.templates.push(template.clone());
    }
    if files.loading.is_some() {
        chain.loading.clone_from(&files.loading);
    }
    if files.error.is_some() {
        chain.error.clone_from(&files.error);
    }
    if files.not_found.is_some() {
        chain.not_found.clone_from(&files.not_found);
    }

    let url_pattern = join_url(&chain.url);
    let app_path = join_url(&chain.app_path);
    if let (Some(page), Some(route)) = (&files.page, &files.route) {
        diags.push(
            Diagnostic::error(
                "route-page-conflict",
                format!("`{page}` and `{route}` resolve to the same path `{url_pattern}`"),
            )
            .with_file(route.clone())
            .with_hint("keep either the page or the route handler in this segment"),
        );
    }
    if let Some(page) = &files.page {
        out.push(build_entry(
            root,
            EntrySpec {
                kind: RouteKind::Page,
                file: page.clone(),
                url_pattern: url_pattern.clone(),
                app_path: app_path.clone(),
                chain: &chain,
                metadata_kind: None,
            },
        ));
    }
    if let Some(route) = &files.route {
        out.push(build_entry(
            root,
            EntrySpec {
                kind: RouteKind::RouteHandler,
                file: route.clone(),
                url_pattern: url_pattern.clone(),
                app_path: app_path.clone(),
                chain: &chain,
                metadata_kind: None,
            },
        ));
    }
    for meta in files.metadata.iter().filter(|m| m.generated) {
        let name = metadata_route_name(meta.kind, &meta.path);
        out.push(build_entry(
            root,
            EntrySpec {
                kind: RouteKind::Metadata,
                file: meta.path.clone(),
                url_pattern: child_path(&url_pattern, &name),
                app_path: child_path(&app_path, &name),
                chain: &chain,
                metadata_kind: Some(meta.kind),
            },
        ));
    }
    for child in &segment.children {
        collect_entries(child, chain.clone(), root, out, diags);
    }
}

// ============================================================================
// Scanners
// ============================================================================

/// Scans an `app/` directory (relative to `root`).
pub fn scan_app_dir(root: &Path, app_dir: &str) -> RouteManifest {
    let mut manifest = RouteManifest {
        root: norm(root),
        app_dir: Some(app_dir.to_owned()),
        ..RouteManifest::default()
    };
    let dir = root.join(app_dir);
    if !dir.is_dir() {
        manifest.diagnostics.push(Diagnostic::error(
            "app-dir-missing",
            format!("`{app_dir}` is not a directory"),
        ));
        return manifest;
    }
    let mut diags = Vec::new();
    let tree = build_segment(
        root,
        &dir,
        "app",
        Classified {
            kind: SegmentKind::Root,
            param: None,
            intercept_level: None,
            intercept_target: None,
        },
        &mut diags,
    );
    manifest.root_layout.clone_from(&tree.files.layout);
    manifest.root_not_found.clone_from(&tree.files.not_found);
    manifest.global_error.clone_from(&tree.files.global_error);
    manifest.static_metadata = tree
        .files
        .metadata
        .iter()
        .filter(|m| !m.generated)
        .cloned()
        .collect();
    let mut entries = Vec::new();
    collect_entries(&tree, Chain::default(), root, &mut entries, &mut diags);
    if manifest.root_layout.is_none() && entries.iter().any(|e| e.kind == RouteKind::Page) {
        diags.push(
            Diagnostic::warning(
                "app-root-layout-missing",
                format!("`{app_dir}/layout.*` is missing; the App Router requires a root layout"),
            )
            .with_hint("add app/layout.tsx exporting <html> and <body>"),
        );
    }
    sort_entries(&mut entries);
    manifest.tree = Some(tree);
    manifest.entries = entries;
    manifest.diagnostics = diags;
    manifest
}

/// Adds legacy `pages/` entries to a manifest.
pub fn scan_pages_dir(root: &Path, pages_dir: &str, manifest: &mut RouteManifest) {
    let base = root.join(pages_dir);
    if !base.is_dir() {
        return;
    }
    manifest.pages_dir = Some(pages_dir.to_owned());
    let walker = WalkDir::new(&base)
        .sort_by_file_name()
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            !(e.file_type().is_dir() && (name == "node_modules" || name.starts_with('.')))
        })
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file());
    for entry in walker {
        let path = entry.path();
        let ext = path
            .extension()
            .map(|e| e.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default();
        if !ROUTE_EXTS.contains(&ext.as_str()) {
            continue;
        }
        let file_name = entry.file_name().to_string_lossy().into_owned();
        if file_name.ends_with(".d.ts") || file_name.starts_with('.') {
            continue;
        }
        let rel_to_pages = path.strip_prefix(&base).map(norm).unwrap_or_default();
        let mut parts: Vec<String> = rel_to_pages
            .split('/')
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
            .collect();
        if let Some(last) = parts.last_mut()
            && let Some(stem) = last.rsplit_once('.').map(|(stem, _)| stem.to_owned())
        {
            *last = stem;
        }
        if parts.len() == 1 && matches!(parts[0].as_str(), "_app" | "_document" | "_error") {
            continue;
        }
        if parts.last().is_some_and(|last| last == "index") {
            parts.pop();
        }
        let url_pattern = join_url(&parts);
        let is_api = parts.first().is_some_and(|p| p == "api");
        let file = rel_path(root, path);
        let source = fs::read_to_string(path).unwrap_or_default();
        let segment_config = segment::analyze_segment_module(&source, &file);
        let legacy_data_functions = segment::legacy_data_functions(&source);
        let kind = if is_api {
            RouteKind::RouteHandler
        } else {
            RouteKind::Page
        };
        manifest.entries.push(RouteEntry {
            app_path: url_pattern.clone(),
            bun_pattern: to_bun_pattern(&url_pattern),
            tanstack_path: to_tanstack_path(&url_pattern),
            dioxus_route: to_dioxus_route(&url_pattern),
            dioxus_variant: to_dioxus_variant(&url_pattern),
            leptos_path: to_leptos_path(&url_pattern),
            params: params_of(&url_pattern),
            url_pattern,
            kind,
            file,
            layouts: Vec::new(),
            templates: Vec::new(),
            loading: None,
            error: None,
            not_found: None,
            parallel_slots: Vec::new(),
            slot: None,
            intercepting: None,
            segment_config,
            methods: if is_api {
                PAGES_API_METHODS.iter().map(|m| (*m).to_owned()).collect()
            } else {
                Vec::new()
            },
            metadata_kind: None,
            legacy_pages: true,
            legacy_data_functions,
        });
    }
    sort_entries(&mut manifest.entries);
}

/// First existing `<dir>/<stem>.<ext>` for `dir` in `.` and `src/`.
fn find_root_file(root: &Path, stems: &[&str], exts: &[&str]) -> Option<String> {
    for dir in ["", "src"] {
        for stem in stems {
            for ext in exts {
                let rel = if dir.is_empty() {
                    format!("{stem}.{ext}")
                } else {
                    format!("{dir}/{stem}.{ext}")
                };
                if root.join(&rel).is_file() {
                    return Some(rel);
                }
            }
        }
    }
    None
}

/// Scans a project root (detects `app/`, `src/app/`, `pages/`, `src/pages/`, proxy, instrumentation).
pub fn scan_project(root: &Path) -> RouteManifest {
    let app_dir = ["app", "src/app"]
        .into_iter()
        .find(|d| root.join(d).is_dir());
    let pages_dir = ["pages", "src/pages"]
        .into_iter()
        .find(|d| root.join(d).is_dir());
    let mut manifest = match app_dir {
        Some(dir) => scan_app_dir(root, dir),
        None => RouteManifest { root: norm(root), ..RouteManifest::default() },
    };
    if let Some(dir) = pages_dir {
        scan_pages_dir(root, dir, &mut manifest);
    }
    if app_dir.is_none() && pages_dir.is_none() {
        manifest.diagnostics.push(Diagnostic::warning(
            "router-dir-missing",
            "neither app/ nor pages/ (or their src/ variants) exists",
        ));
    }
    let proxy = find_root_file(root, &["proxy"], &["ts", "js", "mts", "mjs"]);
    let middleware = find_root_file(root, &["middleware"], &["ts", "js", "mts", "mjs"]);
    if let (Some(p), Some(m)) = (&proxy, &middleware) {
        manifest.diagnostics.push(
            Diagnostic::error(
                "proxy-middleware-conflict",
                format!("both `{p}` and `{m}` exist; Next.js 16 accepts only one"),
            )
            .with_hint("rename middleware.ts to proxy.ts and delete the old file"),
        );
    }
    manifest.proxy_file = proxy.or(middleware);
    manifest.instrumentation = find_root_file(root, &["instrumentation"], &["ts", "js"]);
    manifest.instrumentation_client =
        find_root_file(root, &["instrumentation-client"], &["ts", "js"]);
    manifest.mdx_components =
        find_root_file(root, &["mdx-components"], &["tsx", "ts", "jsx", "js"]);
    sort_entries(&mut manifest.entries);
    manifest
}

// ============================================================================
// Sorting and matching
// ============================================================================

/// Matching specificity of one URL segment (lower matches first).
fn specificity(segment: &str) -> u8 {
    match classify_segment(segment).0 {
        SegmentKind::OptionalCatchAll => 3,
        SegmentKind::CatchAll => 2,
        SegmentKind::Dynamic => 1,
        _ => 0,
    }
}

fn kind_rank(kind: RouteKind) -> u8 {
    match kind {
        RouteKind::Page => 0,
        RouteKind::RouteHandler => 1,
        RouteKind::Metadata => 2,
    }
}

type SortKey = (Vec<u8>, usize, String, u8, u8, String);

fn sort_key(entry: &RouteEntry) -> SortKey {
    let segments = url_segments(&entry.url_pattern);
    (
        segments.iter().map(|s| specificity(s)).collect(),
        segments.len(),
        entry.url_pattern.clone(),
        u8::from(entry.slot.is_some()),
        kind_rank(entry.kind),
        entry.app_path.clone(),
    )
}

/// Sorts entries by matching precedence (static > dynamic > catch-all > optional catch-all),
/// segment by segment, then shorter paths first, then alphabetically. Entries rendering into a
/// slot sort after the page sharing their URL.
pub fn sort_entries(entries: &mut [RouteEntry]) {
    entries.sort_by_cached_key(sort_key);
}

/// Decodes `%XX` escapes (invalid sequences are kept verbatim).
fn percent_decode(segment: &str) -> String {
    if !segment.contains('%') {
        return segment.to_owned();
    }
    let bytes = segment.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let (Some(h), Some(l)) = (hex_val(bytes[i + 1]), hex_val(bytes[i + 2]))
        {
            out.push((h << 4) | l);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

/// Matches concrete path segments against a bracket pattern.
fn match_pattern(pattern: &str, segments: &[&str]) -> Option<BTreeMap<String, String>> {
    let mut params = BTreeMap::new();
    let mut i = 0;
    for pat in url_segments(pattern) {
        let (kind, param, _, _) = classify_segment(pat);
        let name = param.unwrap_or_default();
        match kind {
            SegmentKind::OptionalCatchAll => {
                let rest: Vec<String> = segments[i..].iter().map(|s| percent_decode(s)).collect();
                if !rest.is_empty() {
                    params.insert(name, rest.join("/"));
                }
                return Some(params);
            }
            SegmentKind::CatchAll => {
                if i >= segments.len() {
                    return None;
                }
                let rest: Vec<String> = segments[i..].iter().map(|s| percent_decode(s)).collect();
                params.insert(name, rest.join("/"));
                return Some(params);
            }
            SegmentKind::Dynamic => {
                let value = segments.get(i)?;
                params.insert(name, percent_decode(value));
                i += 1;
            }
            _ => {
                if segments.get(i).is_none_or(|s| *s != pat) {
                    return None;
                }
                i += 1;
            }
        }
    }
    (i == segments.len()).then_some(params)
}

/// Matches a pathname against the manifest (first entry in precedence order; entries rendering
/// into a parallel slot or intercepting another route are skipped). Trailing slashes and
/// query strings are ignored.
pub fn match_path<'a>(manifest: &'a RouteManifest, path: &str) -> Option<RouteMatch<'a>> {
    let path = path.split(['?', '#']).next().unwrap_or("");
    let segments = url_segments(path);
    manifest
        .entries
        .iter()
        .filter(|e| e.slot.is_none())
        .find_map(|entry| {
            match_pattern(&entry.url_pattern, &segments).map(|params| RouteMatch { entry, params })
        })
}

// ============================================================================
// Manifest JSON
// ============================================================================

/// Escapes a literal for use in a JavaScript regular expression.
fn regex_escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if matches!(
            c,
            '.' | '*' | '+' | '?' | '^' | '$' | '{' | '}' | '(' | ')' | '|' | '[' | ']' | '\\'
        ) {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// `nxtP` + cleaned param name (Next.js `routeKeys` convention).
fn route_key(param: &str) -> String {
    let cleaned: String = param
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
        .collect();
    format!("nxtP{cleaned}")
}

/// `(regex, namedRegex, routeKeys)` of a URL pattern, Next.js `getRouteRegex` shape.
fn route_regex(url_pattern: &str) -> (String, String, Map<String, Value>) {
    let mut regex = String::from("^");
    let mut named = String::from("^");
    let mut keys = Map::new();
    let segments = url_segments(url_pattern);
    if segments.is_empty() {
        regex.push('/');
        named.push('/');
    }
    for segment in segments {
        let (kind, param, _, _) = classify_segment(segment);
        let param = param.unwrap_or_default();
        let key = route_key(&param);
        match kind {
            SegmentKind::Dynamic => {
                regex.push_str("/([^/]+?)");
                let _ = write!(named, "/(?<{key}>[^/]+?)");
                keys.insert(key, Value::String(param));
            }
            SegmentKind::CatchAll => {
                regex.push_str("/(.+?)");
                let _ = write!(named, "/(?<{key}>.+?)");
                keys.insert(key, Value::String(param));
            }
            SegmentKind::OptionalCatchAll => {
                regex.push_str("(?:/(.+?))?");
                let _ = write!(named, "(?:/(?<{key}>.+?))?");
                keys.insert(key, Value::String(param));
            }
            _ => {
                let escaped = regex_escape(segment);
                regex.push('/');
                regex.push_str(&escaped);
                named.push('/');
                named.push_str(&escaped);
            }
        }
    }
    regex.push_str("(?:/)?$");
    named.push_str("(?:/)?$");
    (regex, named, keys)
}

/// `routes-manifest.json`-shaped JSON (static/dynamic routes with regexes).
pub fn routes_manifest_json(manifest: &RouteManifest, cfg: &NextConfig) -> Value {
    let app_type = match (manifest.app_dir.is_some(), manifest.pages_dir.is_some()) {
        (true, true) => "hybrid",
        (false, true) => "pages",
        _ => "app",
    };
    let mut seen = BTreeSet::new();
    let mut static_routes = Vec::new();
    let mut dynamic_routes = Vec::new();
    for entry in manifest.entries.iter().filter(|e| e.slot.is_none()) {
        if !seen.insert(entry.url_pattern.clone()) {
            continue;
        }
        let (regex, named_regex, route_keys) = route_regex(&entry.url_pattern);
        let route = json!({
            "page": entry.url_pattern,
            "regex": regex,
            "namedRegex": named_regex,
            "routeKeys": route_keys,
        });
        if entry.params.is_empty() {
            static_routes.push(route);
        } else {
            dynamic_routes.push(route);
        }
    }
    json!({
        "version": 3,
        "basePath": cfg.base_path,
        "appType": app_type,
        "staticRoutes": static_routes,
        "dynamicRoutes": dynamic_routes,
        "dataRoutes": [],
        "rsc": {
            "header": "rsc",
            "contentTypeHeader": "text/x-component",
            "varyHeader": "rsc, next-router-state-tree, next-router-prefetch, next-router-segment-prefetch",
            "prefetchHeader": "next-router-prefetch",
            "suffix": ".rsc",
            "prefetchSegmentHeader": "next-router-segment-prefetch",
            "prefetchSegmentDirSuffix": ".segments",
            "prefetchSegmentSuffix": ".segment.rsc",
        },
    })
}

/// `app-paths-manifest.json`-shaped JSON (`/page` -> module path).
pub fn app_paths_manifest_json(manifest: &RouteManifest) -> Value {
    let mut map = Map::new();
    for entry in manifest.entries.iter().filter(|e| !e.legacy_pages) {
        let suffix = match entry.kind {
            RouteKind::Page => "page",
            RouteKind::RouteHandler | RouteKind::Metadata => "route",
        };
        map.insert(
            child_path(&entry.app_path, suffix),
            Value::String(entry.file.clone()),
        );
    }
    Value::Object(map)
}

// ============================================================================
// Tests
// ============================================================================

/// Shared fixture writer for the unit tests of this crate.
#[cfg(test)]
pub(crate) mod test_support {
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    const ROOT_LAYOUT: &str = r##"import type { Metadata, Viewport } from "next";
import { Inter } from "next/font/google";
import "./globals.css";

const inter = Inter({ subsets: ["latin"], display: "swap", variable: "--font-inter" });

export const metadata: Metadata = {
  metadataBase: new URL("https://fixture.example.com"),
  title: { default: "Yolo Fixture", template: "%s | Yolo Fixture" },
  description: "Next.js 16 migration fixture",
  applicationName: "Yolo Fixture",
  keywords: ["yolo", "next", "bun"],
  authors: [{ name: "Aphrody", url: "https://aphrody.com" }],
  openGraph: {
    title: "Yolo Fixture",
    description: "Migration fixture",
    url: "https://fixture.example.com",
    siteName: "Yolo Fixture",
    images: [{ url: "/opengraph-image", width: 1200, height: 630 }],
    locale: "en_US",
    type: "website",
  },
  twitter: { card: "summary_large_image", site: "@aphrody" },
  robots: { index: true, follow: true },
  alternates: { canonical: "/", languages: { "fr-FR": "/fr" } },
  icons: { icon: "/icon.svg", apple: "/apple-icon.png" },
  manifest: "/manifest.webmanifest",
};

export const viewport: Viewport = {
  width: "device-width",
  initialScale: 1,
  themeColor: [
    { media: "(prefers-color-scheme: dark)", color: "#09090b" },
    { media: "(prefers-color-scheme: light)", color: "#ffffff" },
  ],
  colorScheme: "dark light",
};

export default function RootLayout({ children, modal }: { children: React.ReactNode; modal: React.ReactNode }) {
  return (
    <html lang="en" className={inter.variable}>
      <body>{children}{modal}</body>
    </html>
  );
}
"##;

    const BLOG_PAGE: &str = r#"import type { Metadata } from "next";
import { notFound } from "next/navigation";

export const dynamicParams = true;
export const revalidate = 60;

export async function generateStaticParams() {
  return [{ slug: "hello" }];
}

export async function generateMetadata({ params }: { params: Promise<{ slug: string }> }): Promise<Metadata> {
  const { slug } = await params;
  return { title: slug };
}

export default async function BlogPost({ params }: { params: Promise<{ slug: string }> }) {
  const { slug } = await params;
  if (!slug) notFound();
  return <article><h1>{slug}</h1></article>;
}
"#;

    /// `(relative path, contents)` of the synthetic App Router + Pages Router project.
    pub(crate) const FIXTURE_FILES: &[(&str, &str)] = &[
        (
            "package.json",
            "{ \"name\": \"temp-fixture\", \"dependencies\": { \"next\": \"16.4.5\", \"react\": \"19.3.1\", \"zod\": \"^4.0.0\" }, \"devDependencies\": { \"eslint-config-next\": \"16.4.5\" } }\n",
        ),
        (
            "next.config.ts",
            "const nextConfig = { reactStrictMode: true };\nexport default nextConfig;\n",
        ),
        (
            "proxy.ts",
            "import { NextResponse } from \"next/server\";\nexport function proxy() { return NextResponse.next(); }\nexport const config = { matcher: [\"/dashboard/:path*\"] };\n",
        ),
        (
            "instrumentation.ts",
            "export async function register() {}\n",
        ),
        ("instrumentation-client.ts", "console.log(\"client\");\n"),
        ("app/globals.css", "body { margin: 0; }\n"),
        ("app/layout.tsx", ROOT_LAYOUT),
        (
            "app/page.tsx",
            "export const revalidate = 3600;\nexport default async function HomePage() { return <main>Home</main>; }\n",
        ),
        (
            "app/loading.tsx",
            "export default function Loading() { return <p>Loading</p>; }\n",
        ),
        (
            "app/error.tsx",
            "\"use client\";\nexport default function Error({ error, reset }: { error: Error; reset: () => void }) { return <button onClick={reset}>{error.message}</button>; }\n",
        ),
        (
            "app/global-error.tsx",
            "\"use client\";\nexport default function GlobalError() { return <html><body>Error</body></html>; }\n",
        ),
        (
            "app/not-found.tsx",
            "export default function NotFound() { return <h2>Not found</h2>; }\n",
        ),
        (
            "app/template.tsx",
            "export default function Template({ children }: { children: React.ReactNode }) { return <div>{children}</div>; }\n",
        ),
        (
            "app/icon.svg",
            "<svg xmlns=\"http://www.w3.org/2000/svg\"/>\n",
        ),
        (
            "app/sitemap.ts",
            "export default function sitemap() { return [{ url: \"https://fixture.example.com\" }]; }\n",
        ),
        (
            "app/robots.ts",
            "export default function robots() { return { rules: [{ userAgent: \"*\", allow: \"/\" }] }; }\n",
        ),
        (
            "app/manifest.ts",
            "export default function manifest() { return { name: \"Fixture\" }; }\n",
        ),
        (
            "app/opengraph-image.tsx",
            "import { ImageResponse } from \"next/og\";\nexport const alt = \"Fixture\";\nexport default async function Image() { return new ImageResponse(<div>Fixture</div>); }\n",
        ),
        (
            "app/(marketing)/layout.tsx",
            "export default function MarketingLayout({ children }: { children: React.ReactNode }) { return <div>{children}</div>; }\n",
        ),
        (
            "app/(marketing)/pricing/page.tsx",
            "export const metadata = { title: \"Pricing\" };\nexport const runtime = \"edge\";\nexport default function PricingPage() { return <h1>Pricing</h1>; }\n",
        ),
        (
            "app/@modal/default.tsx",
            "export default function ModalDefault() { return null; }\n",
        ),
        (
            "app/@modal/(.)photos/[id]/page.tsx",
            "\"use client\";\nimport { useRouter } from \"next/navigation\";\nexport default function PhotoModal() { const router = useRouter(); return <dialog open onClick={() => router.back()} />; }\n",
        ),
        ("app/_private/helper.ts", "export const helper = 1;\n"),
        (
            "app/api/health/route.ts",
            "export const dynamic = \"force-static\";\nexport async function GET() { return Response.json({ ok: true }); }\n",
        ),
        (
            "app/api/posts/[id]/route.ts",
            "export async function GET() { return Response.json({}); }\nexport async function POST() { return Response.json({}); }\nexport async function DELETE() { return new Response(null, { status: 204 }); }\n",
        ),
        (
            "app/blog/layout.tsx",
            "export default function BlogLayout({ children }: { children: React.ReactNode }) { return <section>{children}</section>; }\n",
        ),
        (
            "app/blog/[slug]/loading.tsx",
            "export default function Loading() { return <p>Loading post</p>; }\n",
        ),
        ("app/blog/[slug]/page.tsx", BLOG_PAGE),
        (
            "app/blog/new/page.tsx",
            "export const metadata = { title: \"New post\" };\nexport default function NewPost() { return <h1>New</h1>; }\n",
        ),
        (
            "app/dashboard/layout.tsx",
            "export default function DashboardLayout({ children, analytics }: { children: React.ReactNode; analytics: React.ReactNode }) { return <div>{children}{analytics}</div>; }\n",
        ),
        (
            "app/dashboard/page.tsx",
            "export const maxDuration = 30;\nexport default async function Dashboard() { return <form>Dashboard</form>; }\n",
        ),
        (
            "app/dashboard/@analytics/page.tsx",
            "export default function AnalyticsSlot() { return <p>Analytics</p>; }\n",
        ),
        (
            "app/dashboard/@analytics/default.tsx",
            "export default function Default() { return null; }\n",
        ),
        (
            "app/dashboard/@analytics/loading.tsx",
            "export default function Loading() { return null; }\n",
        ),
        (
            "app/docs/[...slug]/page.tsx",
            "export const dynamic = \"force-static\";\nexport default async function DocsPage() { return <article>Docs</article>; }\n",
        ),
        (
            "app/shop/[[...filters]]/page.tsx",
            "export const fetchCache = \"force-cache\";\nexport default async function ShopPage() { return <main>Shop</main>; }\n",
        ),
        (
            "app/photos/[id]/page.tsx",
            "import Image from \"next/image\";\nexport default async function PhotoPage() { return <Image src=\"/hero.png\" alt=\"Hero\" width={10} height={10} />; }\n",
        ),
        (
            "app/fr/page.tsx",
            "export const metadata = { title: \"Accueil\" };\nexport default function FrenchHomePage() { return <main lang=\"fr\">Bienvenue</main>; }\n",
        ),
        (
            "app/login/page.tsx",
            "export const metadata = { title: \"Login\", description: \"Sign in\" };\nexport default function LoginPage() { return <form />; }\n",
        ),
        (
            "app/components/Counter.tsx",
            "\"use client\";\nimport { useState } from \"react\";\nexport function Counter() { const [n, setN] = useState(0); return <button onClick={() => setN(n + 1)}>{n}</button>; }\n",
        ),
        (
            "app/actions.ts",
            "\"use server\";\nexport async function createPost(formData: FormData) { return { ok: true }; }\n",
        ),
        (
            "pages/_app.tsx",
            "export default function App({ Component, pageProps }: any) { return <Component {...pageProps} />; }\n",
        ),
        (
            "pages/legacy.tsx",
            "export async function getServerSideProps() { return { props: { now: 1 } }; }\nexport default function Legacy({ now }: { now: number }) { return <p>{now}</p>; }\n",
        ),
        (
            "pages/404.tsx",
            "export default function Custom404() { return <h1>404</h1>; }\n",
        ),
        (
            "pages/api/legacy.ts",
            "export default function handler(req: any, res: any) { res.status(200).json({ legacy: true }); }\n",
        ),
        ("public/hero.png", ""),
    ];

    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    /// Writes the synthetic project into a fresh temporary directory and returns its path.
    pub(crate) fn temp_project(tag: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or_default();
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "yolo-next-{tag}-{}-{nanos}-{n}",
            std::process::id()
        ));
        for (rel, contents) in FIXTURE_FILES {
            let path = dir.join(rel);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).expect("create fixture dir");
            }
            fs::write(&path, contents).expect("write fixture file");
        }
        dir
    }

    /// Removes a temporary project (errors ignored).
    pub(crate) fn cleanup(dir: &std::path::Path) {
        let _ = fs::remove_dir_all(dir);
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::test_support::{cleanup, temp_project};
    use super::*;

    fn urls(manifest: &RouteManifest) -> Vec<String> {
        manifest
            .entries
            .iter()
            .map(|e| e.url_pattern.clone())
            .collect()
    }

    fn find<'a>(manifest: &'a RouteManifest, url: &str) -> &'a RouteEntry {
        manifest
            .entries
            .iter()
            .find(|e| e.url_pattern == url && e.slot.is_none())
            .unwrap_or_else(|| panic!("entry {url} missing"))
    }

    #[test]
    fn classify_segment_forms() {
        assert_eq!(
            classify_segment("[...slug]"),
            (SegmentKind::CatchAll, Some("slug".into()), None, None)
        );
        assert_eq!(
            classify_segment("[[...slug]]"),
            (
                SegmentKind::OptionalCatchAll,
                Some("slug".into()),
                None,
                None
            )
        );
        assert_eq!(
            classify_segment("[slug]"),
            (SegmentKind::Dynamic, Some("slug".into()), None, None)
        );
        assert_eq!(classify_segment("(marketing)").0, SegmentKind::Group);
        assert_eq!(classify_segment("@modal").0, SegmentKind::ParallelSlot);
        assert_eq!(classify_segment("_lib").0, SegmentKind::Private);
        assert_eq!(classify_segment("blog").0, SegmentKind::Static);
        assert_eq!(
            classify_segment("(.)photos"),
            (SegmentKind::Intercept, None, Some(0), Some("photos".into()))
        );
        assert_eq!(
            classify_segment("(..)[id]"),
            (
                SegmentKind::Intercept,
                Some("id".into()),
                Some(1),
                Some("[id]".into())
            )
        );
        assert_eq!(classify_segment("(..)(..)photo").2, Some(2));
        assert_eq!(classify_segment("(...)photo").2, Some(3));
    }

    #[test]
    fn pattern_conversions() {
        assert_eq!(to_bun_pattern("/"), "/");
        assert_eq!(to_bun_pattern("/blog/[slug]"), "/blog/:slug");
        assert_eq!(to_bun_pattern("/docs/[...slug]"), "/docs/*");
        assert_eq!(to_bun_pattern("/shop/[[...filters]]"), "/shop/*");
        assert_eq!(to_tanstack_path("/"), "/");
        assert_eq!(to_tanstack_path("/blog/[slug]"), "/blog/$slug");
        assert_eq!(to_tanstack_path("/docs/[...slug]"), "/docs/$");
        assert_eq!(to_tanstack_path("/shop/[[...filters]]"), "/shop/$");
        assert_eq!(to_dioxus_route("/blog/[slug]"), "/blog/:slug");
        assert_eq!(to_dioxus_route("/docs/[...slug]"), "/docs/:..slug");
        assert_eq!(to_dioxus_route("/shop/[[...filters]]"), "/shop/:..filters");
        assert_eq!(to_dioxus_variant("/"), "Home");
        assert_eq!(to_dioxus_variant("/blog/[slug]"), "BlogSlug");
        assert_eq!(to_dioxus_variant("/docs/[...slug]"), "DocsSlugCatchAll");
        assert_eq!(
            to_dioxus_variant("/shop/[[...filters]]"),
            "ShopFiltersOptional"
        );
        assert_eq!(to_dioxus_variant("/api/posts/[id]"), "ApiPostsId");
        assert_eq!(to_dioxus_variant("/opengraph-image"), "OpengraphImage");
        assert_eq!(to_dioxus_variant("/sitemap.xml"), "SitemapXml");
        assert_eq!(to_leptos_path("/blog/[slug]"), "/blog/:slug");
        assert_eq!(to_leptos_path("/docs/[...slug]"), "/docs/*slug");
        assert_eq!(to_leptos_path("/shop/[[...filters]]"), "/shop/*filters");
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn scans_synthetic_fixture() {
        let dir = temp_project("routes");
        let manifest = scan_project(&dir);
        let urls = urls(&manifest);
        for expected in [
            "/",
            "/blog/[slug]",
            "/blog/new",
            "/docs/[...slug]",
            "/shop/[[...filters]]",
            "/pricing",
            "/dashboard",
            "/photos/[id]",
            "/api/health",
            "/api/posts/[id]",
            "/sitemap.xml",
            "/robots.txt",
            "/manifest.webmanifest",
            "/opengraph-image",
            "/fr",
            "/login",
            "/legacy",
            "/404",
            "/api/legacy",
        ] {
            assert!(
                urls.contains(&expected.to_owned()),
                "missing {expected} in {urls:?}"
            );
        }
        let pages = manifest
            .entries
            .iter()
            .filter(|e| e.kind == RouteKind::Page)
            .count();
        let handlers = manifest
            .entries
            .iter()
            .filter(|e| e.kind == RouteKind::RouteHandler)
            .count();
        let metadata = manifest
            .entries
            .iter()
            .filter(|e| e.kind == RouteKind::Metadata)
            .count();
        assert_eq!((pages, handlers, metadata), (14, 3, 4));
        assert_eq!(manifest.entries.len(), 21);
        assert!(!urls.iter().any(|u| u.contains("_private")));

        // Root files.
        assert_eq!(manifest.app_dir.as_deref(), Some("app"));
        assert_eq!(manifest.pages_dir.as_deref(), Some("pages"));
        assert_eq!(manifest.root_layout.as_deref(), Some("app/layout.tsx"));
        assert_eq!(
            manifest.root_not_found.as_deref(),
            Some("app/not-found.tsx")
        );
        assert_eq!(
            manifest.global_error.as_deref(),
            Some("app/global-error.tsx")
        );
        assert_eq!(manifest.proxy_file.as_deref(), Some("proxy.ts"));
        assert_eq!(
            manifest.instrumentation.as_deref(),
            Some("instrumentation.ts")
        );
        assert_eq!(
            manifest.instrumentation_client.as_deref(),
            Some("instrumentation-client.ts")
        );
        assert_eq!(manifest.static_metadata.len(), 1);
        assert_eq!(manifest.static_metadata[0].kind, MetadataFileKind::Icon);
        assert!(!manifest.static_metadata[0].generated);

        // Layout chain, loading, templates, slots.
        let blog = find(&manifest, "/blog/[slug]");
        assert_eq!(blog.layouts, vec!["app/layout.tsx", "app/blog/layout.tsx"]);
        assert_eq!(blog.templates, vec!["app/template.tsx"]);
        assert_eq!(blog.loading.as_deref(), Some("app/blog/[slug]/loading.tsx"));
        assert_eq!(blog.error.as_deref(), Some("app/error.tsx"));
        assert_eq!(blog.not_found.as_deref(), Some("app/not-found.tsx"));
        assert_eq!(blog.parallel_slots, Vec::<String>::new());
        assert_eq!(blog.params.len(), 1);
        assert_eq!(blog.params[0].name, "slug");
        assert!(blog.segment_config.has_generate_static_params);
        assert!(blog.segment_config.has_generate_metadata);
        assert_eq!(blog.segment_config.revalidate, Some(Value::from(60)));
        assert_eq!(blog.app_path, "/blog/[slug]");

        let dashboard = find(&manifest, "/dashboard");
        assert_eq!(dashboard.parallel_slots, vec!["analytics"]);
        assert_eq!(
            dashboard.layouts,
            vec!["app/layout.tsx", "app/dashboard/layout.tsx"]
        );
        assert_eq!(dashboard.segment_config.max_duration, Some(30));
        let analytics = manifest
            .entries
            .iter()
            .find(|e| e.slot.as_deref() == Some("analytics"))
            .expect("analytics slot entry");
        assert_eq!(analytics.url_pattern, "/dashboard");
        assert_eq!(analytics.app_path, "/dashboard/@analytics");
        assert_eq!(
            analytics.loading.as_deref(),
            Some("app/dashboard/@analytics/loading.tsx")
        );

        let home = find(&manifest, "/");
        assert_eq!(home.parallel_slots, vec!["modal"]);
        assert_eq!(home.app_path, "/");
        assert_eq!(home.loading.as_deref(), Some("app/loading.tsx"));

        let pricing = find(&manifest, "/pricing");
        assert_eq!(pricing.app_path, "/(marketing)/pricing");
        assert_eq!(
            pricing.layouts,
            vec!["app/layout.tsx", "app/(marketing)/layout.tsx"]
        );
        assert_eq!(pricing.segment_config.runtime.as_deref(), Some("edge"));
        assert!(
            pricing
                .segment_config
                .diagnostics
                .iter()
                .any(|d| d.code == "segment-config-edge-runtime-deprecated")
        );

        // Intercepting route.
        let modal = manifest
            .entries
            .iter()
            .find(|e| e.slot.as_deref() == Some("modal"))
            .expect("modal entry");
        assert_eq!(modal.url_pattern, "/photos/[id]");
        assert_eq!(modal.app_path, "/@modal/(.)photos/[id]");
        assert_eq!(
            modal.intercepting,
            Some(InterceptSpec { level: 0, target: "/photos/[id]".into() })
        );
        assert_eq!(modal.params[0].name, "id");
        assert_eq!(
            modal.segment_config.directive,
            crate::types::ModuleDirective::UseClient
        );
        let photo = find(&manifest, "/photos/[id]");
        assert!(photo.intercepting.is_none());

        // Route handlers and metadata.
        let health = find(&manifest, "/api/health");
        assert_eq!(health.kind, RouteKind::RouteHandler);
        assert_eq!(health.methods, vec!["GET"]);
        assert_eq!(
            health.segment_config.dynamic.as_deref(),
            Some("force-static")
        );
        let posts = find(&manifest, "/api/posts/[id]");
        assert_eq!(posts.methods, vec!["GET", "POST", "DELETE"]);
        let sitemap = find(&manifest, "/sitemap.xml");
        assert_eq!(sitemap.kind, RouteKind::Metadata);
        assert_eq!(sitemap.metadata_kind, Some(MetadataFileKind::Sitemap));
        assert_eq!(sitemap.file, "app/sitemap.ts");
        assert_eq!(sitemap.app_path, "/sitemap.xml");
        assert_eq!(
            find(&manifest, "/opengraph-image").file,
            "app/opengraph-image.tsx"
        );

        // Pages router.
        let legacy = find(&manifest, "/legacy");
        assert!(legacy.legacy_pages);
        assert_eq!(legacy.legacy_data_functions, vec!["getServerSideProps"]);
        assert_eq!(legacy.file, "pages/legacy.tsx");
        let api_legacy = find(&manifest, "/api/legacy");
        assert_eq!(api_legacy.kind, RouteKind::RouteHandler);
        assert_eq!(api_legacy.methods.len(), 7);
        assert!(api_legacy.legacy_pages);
        assert!(!urls.iter().any(|u| u.contains("_app")));

        // Sort precedence.
        let pos = |u: &str| urls.iter().position(|x| x == u).unwrap_or(usize::MAX);
        assert!(pos("/blog/new") < pos("/blog/[slug]"));
        assert!(pos("/") < pos("/blog/[slug]"));
        assert!(pos("/docs/[...slug]") > pos("/dashboard"));
        assert!(
            manifest
                .diagnostics
                .iter()
                .all(|d| d.code != "route-page-conflict")
        );
        cleanup(&dir);
    }

    #[test]
    fn matches_paths() {
        let dir = temp_project("match");
        let manifest = scan_project(&dir);
        let m = match_path(&manifest, "/blog/hello").expect("blog match");
        assert_eq!(m.entry.url_pattern, "/blog/[slug]");
        assert_eq!(m.params.get("slug").map(String::as_str), Some("hello"));
        let m = match_path(&manifest, "/blog/new/").expect("static wins");
        assert_eq!(m.entry.url_pattern, "/blog/new");
        let m = match_path(&manifest, "/docs/a/b").expect("docs");
        assert_eq!(m.params.get("slug").map(String::as_str), Some("a/b"));
        let m = match_path(&manifest, "/shop").expect("optional bare");
        assert_eq!(m.entry.url_pattern, "/shop/[[...filters]]");
        assert!(m.params.is_empty());
        let m = match_path(&manifest, "/shop/x/y?sort=asc").expect("optional rest");
        assert_eq!(m.params.get("filters").map(String::as_str), Some("x/y"));
        assert_eq!(
            match_path(&manifest, "/pricing").expect("group").entry.file,
            "app/(marketing)/pricing/page.tsx"
        );
        let m = match_path(&manifest, "/photos/9").expect("photo");
        assert_eq!(m.entry.file, "app/photos/[id]/page.tsx");
        assert!(m.entry.slot.is_none());
        let m = match_path(&manifest, "/api/posts/42").expect("api");
        assert_eq!(m.entry.kind, RouteKind::RouteHandler);
        assert_eq!(m.params.get("id").map(String::as_str), Some("42"));
        assert_eq!(
            match_path(&manifest, "/sitemap.xml")
                .expect("sitemap")
                .entry
                .kind,
            RouteKind::Metadata
        );
        assert_eq!(
            match_path(&manifest, "/").expect("root").entry.file,
            "app/page.tsx"
        );
        assert_eq!(
            match_path(&manifest, "/dashboard")
                .expect("dashboard")
                .entry
                .file,
            "app/dashboard/page.tsx"
        );
        assert!(match_path(&manifest, "/nope").is_none());
        assert!(match_path(&manifest, "/docs").is_none());
        assert_eq!(
            match_path(&manifest, "/blog/caf%C3%A9")
                .expect("decoded")
                .params["slug"],
            "café"
        );
        cleanup(&dir);
    }

    #[test]
    fn manifest_json_shapes() {
        let dir = temp_project("manifest");
        let manifest = scan_project(&dir);
        let json = routes_manifest_json(&manifest, &NextConfig::default());
        assert_eq!(json["version"], 3);
        assert_eq!(json["appType"], "hybrid");
        assert_eq!(json["rsc"]["contentTypeHeader"], "text/x-component");
        let statics = json["staticRoutes"].as_array().expect("staticRoutes");
        let dynamics = json["dynamicRoutes"].as_array().expect("dynamicRoutes");
        let regex_of = |list: &[Value], page: &str| -> (String, String, Value) {
            let r = list
                .iter()
                .find(|r| r["page"] == page)
                .unwrap_or_else(|| panic!("{page} missing"));
            (
                r["regex"].as_str().unwrap_or_default().to_owned(),
                r["namedRegex"].as_str().unwrap_or_default().to_owned(),
                r["routeKeys"].clone(),
            )
        };
        assert_eq!(regex_of(statics, "/").0, "^/(?:/)?$");
        assert_eq!(regex_of(statics, "/blog/new").0, "^/blog/new(?:/)?$");
        assert_eq!(
            regex_of(statics, "/sitemap.xml").0,
            "^/sitemap\\.xml(?:/)?$"
        );
        let (regex, named, keys) = regex_of(dynamics, "/blog/[slug]");
        assert_eq!(regex, "^/blog/([^/]+?)(?:/)?$");
        assert_eq!(named, "^/blog/(?<nxtPslug>[^/]+?)(?:/)?$");
        assert_eq!(keys, json!({ "nxtPslug": "slug" }));
        assert_eq!(
            regex_of(dynamics, "/docs/[...slug]").0,
            "^/docs/(.+?)(?:/)?$"
        );
        assert_eq!(
            regex_of(dynamics, "/shop/[[...filters]]").0,
            "^/shop(?:/(.+?))?(?:/)?$"
        );
        assert_eq!(
            statics.iter().filter(|r| r["page"] == "/dashboard").count(),
            1,
            "slot pages deduplicated"
        );

        let paths = app_paths_manifest_json(&manifest);
        assert_eq!(paths["/page"], "app/page.tsx");
        assert_eq!(paths["/blog/[slug]/page"], "app/blog/[slug]/page.tsx");
        assert_eq!(paths["/api/health/route"], "app/api/health/route.ts");
        assert_eq!(paths["/sitemap.xml/route"], "app/sitemap.ts");
        assert_eq!(
            paths["/(marketing)/pricing/page"],
            "app/(marketing)/pricing/page.tsx"
        );
        assert_eq!(
            paths["/@modal/(.)photos/[id]/page"],
            "app/@modal/(.)photos/[id]/page.tsx"
        );
        assert!(paths.get("/legacy/page").is_none());
        cleanup(&dir);
    }

    #[test]
    fn layout_prefix_helper() {
        assert_eq!(file_url_prefix("app", "app/layout.tsx"), "");
        assert_eq!(file_url_prefix("app", "app/(marketing)/layout.tsx"), "");
        assert_eq!(file_url_prefix("app", "app/blog/layout.tsx"), "/blog");
        assert_eq!(
            file_url_prefix("app", "app/blog/[slug]/layout.tsx"),
            "/blog/[slug]"
        );
        assert_eq!(
            file_url_prefix("src/app", "src/app/@modal/(.)photos/layout.tsx"),
            "/photos"
        );
        assert_eq!(
            app_relative_dir_parts("app", "app/(marketing)/pricing/page.tsx"),
            vec!["(marketing)", "pricing"]
        );
    }

    #[test]
    fn route_page_conflict_is_reported() {
        let dir = temp_project("conflict");
        std::fs::write(
            dir.join("app/login/route.ts"),
            "export async function POST() { return new Response(null); }\n",
        )
        .expect("write route");
        let manifest = scan_project(&dir);
        assert!(manifest.diagnostics.iter().any(
            |d| d.code == "route-page-conflict" && d.severity == crate::types::Severity::Error
        ));
        cleanup(&dir);
    }

    #[test]
    fn real_fixture_when_present() {
        let root = Path::new("C:/src/yolo-stack/apps/next-fixture");
        if !root.exists() {
            return;
        }
        let manifest = scan_project(root);
        let urls = urls(&manifest);
        for expected in [
            "/",
            "/blog/[slug]",
            "/docs/[...slug]",
            "/shop/[[...filters]]",
            "/pricing",
            "/dashboard",
            "/photos/[id]",
            "/users/[username]",
            "/api/health",
            "/api/posts/[id]",
            "/sitemap.xml",
            "/robots.txt",
            "/manifest.webmanifest",
            "/opengraph-image",
            "/legacy",
            "/api/legacy",
        ] {
            assert!(
                urls.contains(&expected.to_owned()),
                "missing {expected} in {urls:?}"
            );
        }
        assert_eq!(manifest.proxy_file.as_deref(), Some("proxy.ts"));
        assert_eq!(
            manifest.instrumentation.as_deref(),
            Some("instrumentation.ts")
        );
        assert_eq!(
            find(&manifest, "/api/posts/[id]").methods,
            vec!["GET", "POST", "DELETE"]
        );
        assert_eq!(
            find(&manifest, "/dashboard").parallel_slots,
            vec!["analytics"]
        );
        let m = match_path(&manifest, "/photos/9").expect("photo");
        assert_eq!(m.entry.file, "app/photos/[id]/page.tsx");
        assert!(manifest.tree.is_some());
    }
}
