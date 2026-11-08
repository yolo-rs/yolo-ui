// SPDX-License-Identifier: Apache-2.0
//! Dioxus 0.8 lane emitter: `Cargo.toml`, `Dioxus.toml`, `src/main.rs` (`Routable`), server fns, assets.
//!
//! The generated crate targets `dioxus = "0.8.0-alpha.3"` with the `fullstack` and `router`
//! features: `#[derive(Routable)]` with nested `#[nest]` / `#[layout]` blocks, `document::*`
//! head tags from the static metadata, `asset!()` for local `next/image` sources, and
//! `#[get]` / `#[post]` server functions for route handlers and Server Actions.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use crate::metadata;
use crate::types::{
    EmittedFile, ImageUsage, Metadata, ProjectAnalysis, RouteEntry, RouteKind, RouteManifest,
    SegmentKind, Viewport,
};
use crate::{images, proxy, routes};

// ============================================================================
// Shared naming helpers (also used by the Leptos lane)
// ============================================================================

/// Rust keywords that cannot be used as identifiers.
const RUST_KEYWORDS: [&str; 52] = [
    "as", "break", "const", "continue", "crate", "else", "enum", "extern", "false", "fn", "for",
    "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref", "return",
    "self", "Self", "static", "struct", "super", "trait", "true", "type", "unsafe", "use", "where",
    "while", "async", "await", "dyn", "abstract", "become", "box", "do", "final", "macro",
    "override", "priv", "typeof", "unsized", "virtual", "yield", "try", "gen",
];

/// `snake_case` identifier (`postId` -> `post_id`, `my-slug` -> `my_slug`, `type` -> `type_`).
pub(crate) fn rust_ident(name: &str) -> String {
    let mut out = String::new();
    let mut prev_lower = false;
    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            if c.is_ascii_uppercase() {
                if prev_lower {
                    out.push('_');
                }
                out.extend(c.to_lowercase());
            } else {
                out.push(c);
            }
            prev_lower = c.is_ascii_lowercase() || c.is_ascii_digit();
        } else if !out.ends_with('_') && !out.is_empty() {
            out.push('_');
            prev_lower = false;
        }
    }
    let mut out = out.trim_end_matches('_').to_owned();
    if out.is_empty() {
        out.push_str("param");
    }
    if out.starts_with(|c: char| c.is_ascii_digit()) {
        out.insert_str(0, "p_");
    }
    if RUST_KEYWORDS.contains(&out.as_str()) {
        out.push('_');
    }
    out
}

/// Cargo package name in `snake_case` (`next-fixture` -> `next_fixture`).
pub(crate) fn crate_name(name: &str) -> String {
    let scoped = name.rsplit('/').next().unwrap_or(name);
    let ident = rust_ident(scoped);
    if ident == "param" {
        String::from("yolo_app")
    } else {
        ident
    }
}

/// `PascalCase` type name from any text.
pub(crate) fn pascal_ident(text: &str) -> String {
    let mut out = String::new();
    let mut upper = true;
    for c in text.chars() {
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
    if out.is_empty() {
        out.push_str("Item");
    }
    if out.starts_with(|c: char| c.is_ascii_digit()) {
        out.insert_str(0, "Route");
    }
    out
}

/// Quoted TOML basic string.
pub(crate) fn toml_string(text: &str) -> String {
    let mut out = String::from("\"");
    for c in text.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            _ => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Escapes text for a Rust string literal; RSX literals also need `{{` / `}}`.
pub(crate) fn rust_string(text: &str, rsx: bool) -> String {
    let mut out = String::new();
    for c in text.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '{' if rsx => out.push_str("{{"),
            '}' if rsx => out.push_str("}}"),
            _ => out.push(c),
        }
    }
    out
}

/// Unique identifier registry.
#[derive(Default)]
pub(crate) struct UniqueNames {
    used: BTreeSet<String>,
}

impl UniqueNames {
    /// Registry pre-seeded with reserved names.
    pub(crate) fn with_reserved(reserved: &[&str]) -> Self {
        Self { used: reserved.iter().map(|s| (*s).to_owned()).collect() }
    }

    /// Returns `base`, or `base2`, `base3`, ... when already taken.
    pub(crate) fn unique(&mut self, base: &str) -> String {
        let mut candidate = base.to_owned();
        let mut n = 2;
        while !self.used.insert(candidate.clone()) {
            candidate = format!("{base}{n}");
            n += 1;
        }
        candidate
    }
}

/// Route param with its Rust identifier.
pub(crate) struct RouteField {
    /// Rust identifier.
    pub ident: String,
    /// Original Next.js param name.
    pub param: String,
    /// `[...x]` / `[[...x]]`.
    pub catch_all: bool,
}

/// Rust identifiers of the params of a page (unique within the route).
pub(crate) fn route_fields(entry: &RouteEntry) -> Vec<RouteField> {
    let mut names = UniqueNames::with_reserved(&["lang"]);
    entry
        .params
        .iter()
        .map(|p| RouteField {
            ident: names.unique(&rust_ident(&p.name)),
            param: p.name.clone(),
            catch_all: p.catch_all,
        })
        .collect()
}

/// Dioxus route string using the sanitized param identifiers (`/blog/:slug`, `/docs/:..slug`).
pub(crate) fn dioxus_route_for(entry: &RouteEntry, fields: &[RouteField]) -> String {
    let mut fields = fields.iter();
    let segments: Vec<String> = entry
        .url_pattern
        .split('/')
        .filter(|s| !s.is_empty())
        .map(|segment| {
            let (kind, _, _, _) = routes::classify_segment(segment);
            match kind {
                SegmentKind::Dynamic => fields
                    .next()
                    .map_or_else(|| segment.to_owned(), |f| format!(":{}", f.ident)),
                SegmentKind::CatchAll | SegmentKind::OptionalCatchAll => fields
                    .next()
                    .map_or_else(|| segment.to_owned(), |f| format!(":..{}", f.ident)),
                _ => segment.to_owned(),
            }
        })
        .collect();
    if segments.is_empty() {
        String::from("/")
    } else {
        format!("/{}", segments.join("/"))
    }
}

/// Pages that take part in client routing (no slot / intercept entries).
pub(crate) fn routed_pages(manifest: &RouteManifest) -> Vec<&RouteEntry> {
    manifest
        .entries
        .iter()
        .filter(|e| e.kind == RouteKind::Page && e.slot.is_none())
        .collect()
}

/// Component name per page file (`dioxus_variant` made unique).
pub(crate) fn page_names(pages: &[&RouteEntry], reserved: &[&str]) -> BTreeMap<String, String> {
    let mut names = UniqueNames::with_reserved(reserved);
    pages
        .iter()
        .map(|p| {
            (
                p.file.clone(),
                names.unique(&pascal_ident(&p.dioxus_variant)),
            )
        })
        .collect()
}

/// Layout component name (`app/blog/layout.tsx` -> `BlogLayout`).
pub(crate) fn layout_name(app_dir: &str, file: &str) -> String {
    let parts = routes::app_relative_dir_parts(app_dir, file);
    let base: String = parts.iter().map(|p| pascal_ident(p)).collect();
    if base.is_empty() {
        String::from("RootLayout")
    } else {
        format!("{base}Layout")
    }
}

/// `api_posts_id_get` from `/api/posts/[id]` + `GET`.
pub(crate) fn handler_fn_name(entry: &RouteEntry, method: &str) -> String {
    let mut parts: Vec<String> = entry
        .url_pattern
        .split('/')
        .filter(|s| !s.is_empty())
        .map(|segment| {
            let (_, param, _, _) = routes::classify_segment(segment);
            rust_ident(&param.unwrap_or_else(|| segment.to_owned()))
        })
        .collect();
    if parts.is_empty() {
        parts.push(String::from("index"));
    }
    parts.push(method.to_ascii_lowercase());
    parts.join("_")
}

/// Axum-style path of a route handler (`/api/posts/{id}`, `/docs/{*slug}`).
pub(crate) fn axum_path(entry: &RouteEntry) -> String {
    let segments: Vec<String> = entry
        .url_pattern
        .split('/')
        .filter(|s| !s.is_empty())
        .map(|segment| {
            let (kind, param, _, _) = routes::classify_segment(segment);
            let name = rust_ident(&param.unwrap_or_default());
            match kind {
                SegmentKind::Dynamic => format!("{{{name}}}"),
                SegmentKind::CatchAll | SegmentKind::OptionalCatchAll => format!("{{*{name}}}"),
                _ => segment.to_owned(),
            }
        })
        .collect();
    if segments.is_empty() {
        String::from("/")
    } else {
        format!("/{}", segments.join("/"))
    }
}

/// Server Action function names (`action_create_post`), unique per `(module, export)`.
pub(crate) fn action_fn_names(analysis: &ProjectAnalysis) -> Vec<(String, String, String)> {
    let mut names = UniqueNames::default();
    let mut seen = BTreeSet::new();
    analysis
        .graph
        .server_manifest
        .iter()
        .filter(|r| !r.cache && seen.insert((r.module.clone(), r.export_name.clone())))
        .map(|r| {
            (
                names.unique(&format!("action_{}", rust_ident(&r.export_name))),
                r.module.clone(),
                r.export_name.clone(),
            )
        })
        .collect()
}

// ============================================================================
// Shared nesting tree (layouts with static URL prefixes)
// ============================================================================

/// One nesting level: a layout and the pages / nested layouts below it.
pub(crate) struct NestNode<'a> {
    /// Layout file (`None` for the synthetic root).
    pub layout: Option<&'a str>,
    /// URL prefix in Next.js bracket form (`""` for the root, `/blog`).
    pub url_prefix: String,
    /// Items in route order.
    pub items: Vec<NestItem<'a>>,
}

/// Page or nested layout.
pub(crate) enum NestItem<'a> {
    /// A page entry.
    Page(&'a RouteEntry),
    /// A nested layout block.
    Nest(NestNode<'a>),
}

impl<'a> NestNode<'a> {
    fn child(&mut self, layout: &'a str, url_prefix: String) -> &mut NestNode<'a> {
        let index = self
            .items
            .iter()
            .position(|item| matches!(item, NestItem::Nest(node) if node.layout == Some(layout)));
        let index = index.unwrap_or_else(|| {
            self.items.push(NestItem::Nest(NestNode {
                layout: Some(layout),
                url_prefix,
                items: Vec::new(),
            }));
            self.items.len() - 1
        });
        match &mut self.items[index] {
            NestItem::Nest(node) => node,
            NestItem::Page(_) => unreachable!("index points at a nest node"),
        }
    }
}

/// Builds the nesting tree. Layouts under a dynamic prefix are flattened; group layouts
/// (same prefix as their parent) are kept only when `allow_pathless` is set.
pub(crate) fn build_nest_tree<'a>(
    manifest: &'a RouteManifest,
    app_dir: &str,
    allow_pathless: bool,
) -> NestNode<'a> {
    let mut root = NestNode {
        layout: manifest.root_layout.as_deref(),
        url_prefix: String::new(),
        items: Vec::new(),
    };
    for page in routed_pages(manifest) {
        let mut node = &mut root;
        for layout in page
            .layouts
            .iter()
            .filter(|l| manifest.root_layout.as_ref() != Some(l))
        {
            let prefix = routes::file_url_prefix(app_dir, layout);
            let pathless = prefix == node.url_prefix;
            if prefix.contains('[') || (pathless && !allow_pathless) {
                continue;
            }
            node = node.child(layout, prefix);
        }
        node.items.push(NestItem::Page(page));
    }
    root
}

/// Static metadata title of a page resolved through the root template.
pub(crate) fn page_title(root: &Metadata, page: &RouteEntry) -> Option<String> {
    let own = page
        .segment_config
        .static_metadata
        .clone()
        .unwrap_or_default();
    metadata::resolve_title(Some(root), &own)
}

/// Local (non-remote) image usages with a literal `src`, deduplicated by `src`.
pub(crate) fn local_images(analysis: &ProjectAnalysis) -> Vec<&ImageUsage> {
    let mut seen = BTreeSet::new();
    analysis
        .images
        .usages
        .iter()
        .filter(|u| !u.remote)
        .filter(|u| u.src.as_deref().is_some_and(|s| seen.insert(s.to_owned())))
        .collect()
}

// ============================================================================
// Emitter
// ============================================================================

/// Emits the Dioxus fullstack project.
pub fn emit_dioxus_project(analysis: &ProjectAnalysis) -> Vec<EmittedFile> {
    let package = crate_name(&analysis.name);
    let (root_meta, root_viewport) = metadata::root_metadata(analysis);
    let title = metadata::resolve_title(None, &root_meta).unwrap_or_else(|| analysis.name.clone());
    let server_fns = server_fns_rs(analysis);
    let has_fonts = !analysis.font_plans.is_empty();
    let mut files = vec![
        EmittedFile {
            path: String::from("Cargo.toml"),
            contents: cargo_toml(&package, analysis.proxy.is_some()),
        },
        EmittedFile {
            path: String::from("Dioxus.toml"),
            contents: dioxus_toml(&package, &title, &analysis.config.base_path),
        },
        EmittedFile {
            path: String::from("src/main.rs"),
            contents: main_rs(
                analysis,
                &root_meta,
                root_viewport.as_ref(),
                server_fns.is_some(),
                has_fonts,
            ),
        },
    ];
    if let Some(contents) = server_fns {
        files.push(EmittedFile { path: String::from("src/server_fns.rs"), contents });
    }
    if let Some(spec) = &analysis.proxy {
        files.push(EmittedFile {
            path: String::from("src/proxy.rs"),
            contents: proxy::emit_axum_proxy(spec),
        });
    }
    if has_fonts {
        let mut css =
            String::from("/* Generated by yolo-next: self-hosted next/font replacements. */\n");
        for plan in &analysis.font_plans {
            css.push_str(&plan.css);
            if !plan.css.ends_with('\n') {
                css.push('\n');
            }
        }
        files.push(EmittedFile { path: String::from("assets/fonts.css"), contents: css });
    }
    files.push(EmittedFile {
        path: String::from("README.md"),
        contents: readme(analysis, &package),
    });
    files
}

fn cargo_toml(package: &str, has_proxy: bool) -> String {
    let mut out = format!(
        "# Generated by yolo-next. Dioxus 0.8 fullstack lane.\n[package]\nname = {}\nversion = \"0.1.0\"\nedition = \"2024\"\npublish = false\n\n[dependencies]\ndioxus = {{ version = \"0.8.0-alpha.3\", features = [\"fullstack\", \"router\"] }}\nserde = {{ version = \"1\", features = [\"derive\"] }}\nserde_json = \"1\"\n",
        toml_string(package)
    );
    if has_proxy {
        out.push_str("axum = { version = \"0.8\", optional = true }\ntower = { version = \"0.5\", optional = true }\n");
    }
    out.push_str("\n[features]\ndefault = [\"web\"]\nweb = [\"dioxus/web\"]\n");
    if has_proxy {
        out.push_str("server = [\"dioxus/server\", \"dep:axum\", \"dep:tower\"]\n");
    } else {
        out.push_str("server = [\"dioxus/server\"]\n");
    }
    out
}

fn dioxus_toml(package: &str, title: &str, base_path: &str) -> String {
    let mut out = format!(
        "# Generated by yolo-next.\n[application]\nname = {}\nasset_dir = \"assets\"\nout_dir = \"dist\"\n\n[web.app]\ntitle = {}\n",
        toml_string(package),
        toml_string(title)
    );
    let base = base_path.trim_matches('/');
    if !base.is_empty() {
        let _ = writeln!(out, "base_path = {}", toml_string(base));
    }
    out.push_str("\n[web.watcher]\nindex_on_404 = true\n");
    out
}

/// Variant fields of a page (`{ slug: String }`).
fn variant_fields(fields: &[RouteField]) -> String {
    if fields.is_empty() {
        return String::from("{}");
    }
    let list: Vec<String> = fields
        .iter()
        .map(|f| {
            if f.catch_all {
                format!("{}: Vec<String>", f.ident)
            } else {
                format!("{}: String", f.ident)
            }
        })
        .collect();
    format!("{{ {} }}", list.join(", "))
}

/// Route string relative to the enclosing `#[nest]` prefix (`/` when equal).
fn relative_route(full: &str, nest_prefix: &str) -> String {
    let rest = full.strip_prefix(nest_prefix).unwrap_or(full);
    if rest.is_empty() {
        String::from("/")
    } else if rest.starts_with('/') {
        rest.to_owned()
    } else {
        format!("/{rest}")
    }
}

/// Code generation state of `src/main.rs`.
struct MainGen {
    page_names: BTreeMap<String, String>,
    layout_names: BTreeMap<String, String>,
}

impl MainGen {
    fn emit_enum(&self, node: &NestNode<'_>, nest_prefix: &str, indent: usize, out: &mut String) {
        let pad = " ".repeat(indent);
        for item in &node.items {
            match item {
                NestItem::Page(page) => {
                    let fields = route_fields(page);
                    let route = relative_route(&dioxus_route_for(page, &fields), nest_prefix);
                    let name = self.page_names.get(&page.file).cloned().unwrap_or_default();
                    let _ = writeln!(
                        out,
                        "{pad}#[route(\"{route}\")]\n{pad}{name} {},",
                        variant_fields(&fields)
                    );
                }
                NestItem::Nest(child) => {
                    let child_prefix = if child.url_prefix.is_empty() {
                        String::new()
                    } else {
                        routes::to_dioxus_route(&child.url_prefix)
                    };
                    let relative = relative_route(&child_prefix, nest_prefix);
                    let nested = relative != "/";
                    let layout = child
                        .layout
                        .and_then(|l| self.layout_names.get(l))
                        .cloned()
                        .unwrap_or_default();
                    let mut inner = indent;
                    if nested {
                        let _ = writeln!(out, "{pad}#[nest(\"{relative}\")]");
                        inner += 4;
                    }
                    let lpad = " ".repeat(inner);
                    let _ = writeln!(out, "{lpad}#[layout({layout})]");
                    self.emit_enum(child, &child_prefix, inner + 4, out);
                    let _ = writeln!(out, "{lpad}#[end_layout]");
                    if nested {
                        let _ = writeln!(out, "{pad}#[end_nest]");
                    }
                }
            }
        }
    }
}

/// Asset constants for local `next/image` sources.
fn asset_consts(analysis: &ProjectAnalysis) -> String {
    let policy = analysis.yolo_config.images.clone().unwrap_or_else(|| {
        images::optimizer_policy(&analysis.config.images, &analysis.config.base_path)
    });
    let mut out = String::new();
    for (index, usage) in local_images(analysis).iter().enumerate() {
        let src = usage.src.clone().unwrap_or_default();
        let expr = images::manganis_asset_expr(usage, &policy);
        let expr = if expr.contains("asset!") {
            expr
        } else {
            format!(
                "asset!(\"{}\", AssetOptions::image().with_format(ImageFormat::Avif).with_preload({}))",
                rust_string(&src, false),
                usage.priority
            )
        };
        let _ = writeln!(
            out,
            "/// `{}` from `{}`{}.\nconst ASSET_{index}: Asset = {expr};",
            src,
            usage.file,
            usage
                .alt
                .as_deref()
                .map_or(String::new(), |alt| format!(" (alt: {alt})"))
        );
    }
    out
}

fn main_rs(
    analysis: &ProjectAnalysis,
    root_meta: &Metadata,
    root_viewport: Option<&Viewport>,
    has_server_fns: bool,
    has_fonts: bool,
) -> String {
    let manifest = &analysis.routes;
    let app_dir = manifest.app_dir.as_deref().unwrap_or("app");
    let pages = routed_pages(manifest);
    let reserved = [
        "App",
        "Route",
        "RootLayout",
        "NotFound",
        "Element",
        "Outlet",
        "Router",
    ];
    let page_names = page_names(&pages, &reserved);
    let mut names = UniqueNames::with_reserved(&reserved);
    for name in page_names.values() {
        names.used.insert(name.clone());
    }
    let mut layout_names = BTreeMap::new();
    for page in &pages {
        for layout in page
            .layouts
            .iter()
            .filter(|l| manifest.root_layout.as_ref() != Some(l))
        {
            if !layout_names.contains_key(layout) {
                let name = names.unique(&layout_name(app_dir, layout));
                layout_names.insert(layout.clone(), name);
            }
        }
    }
    let generator = MainGen { page_names, layout_names };
    let tree = build_nest_tree(manifest, app_dir, true);

    let mut out = format!(
        "//! Generated by yolo-next from `{}` (Next.js {}). Dioxus 0.8 fullstack lane.\n//!\n//! Pages keep their Next.js URL structure; bodies are placeholders to port by hand.\n#![allow(non_snake_case, clippy::needless_pass_by_value)]\n\nuse dioxus::prelude::*;\n",
        analysis.name,
        analysis.next_version.as_deref().unwrap_or("16")
    );
    if has_server_fns {
        out.push_str("\nmod server_fns;\n");
    }
    if analysis.proxy.is_some() {
        out.push_str("#[cfg(feature = \"server\")]\nmod proxy;\n");
    }
    let assets = asset_consts(analysis);
    if !assets.is_empty() {
        out.push('\n');
        out.push_str(&assets);
    }

    // Route enum.
    out.push_str("\n/// Application routes (`app/` tree).\n#[derive(Routable, Clone, PartialEq, Debug)]\n#[rustfmt::skip]\nenum Route {\n    #[layout(RootLayout)]\n");
    generator.emit_enum(&tree, "", 8, &mut out);
    out.push_str("        #[route(\"/:..segments\")]\n        NotFound { segments: Vec<String> },\n    #[end_layout]\n}\n\n");

    // main + App.
    out.push_str("fn main() {\n    #[cfg(feature = \"server\")]\n    dioxus::serve(|| async move { Ok(dioxus::server::router(App)) });\n    #[cfg(not(feature = \"server\"))]\n    dioxus::launch(App);\n}\n\n");
    out.push_str("/// Router host.\nfn App() -> Element {\n    rsx! {\n        Router::<Route> {}\n    }\n}\n\n");
    out.push_str(&root_layout_component(
        manifest,
        root_meta,
        root_viewport,
        has_fonts,
    ));

    // Nested layouts.
    for (file, name) in &generator.layout_names {
        let _ = write!(
            out,
            "/// Layout `{file}`.\n#[component]\nfn {name}() -> Element {{\n    rsx! {{\n        Outlet::<Route> {{}}\n    }}\n}}\n\n"
        );
    }

    // Pages.
    for page in &pages {
        out.push_str(&page_component(&generator, page, root_meta));
    }

    let _ = writeln!(
        out,
        "/// Fallback route{}.\n#[component]\nfn NotFound(segments: Vec<String>) -> Element {{\n    rsx! {{\n        main {{\n            h1 {{ \"Not found\" }}\n            p {{ \"{{segments:?}}\" }}\n        }}\n    }}\n}}",
        manifest
            .root_not_found
            .as_deref()
            .map_or(String::new(), |f| format!(" (port `{f}`)"))
    );
    out
}

/// `RootLayout` component: static metadata head tags, fonts stylesheet and the route outlet.
fn root_layout_component(
    manifest: &RouteManifest,
    root_meta: &Metadata,
    root_viewport: Option<&Viewport>,
    has_fonts: bool,
) -> String {
    let slots: BTreeSet<String> = manifest
        .entries
        .iter()
        .filter_map(|e| e.slot.clone())
        .collect();
    let mut out = format!(
        "/// Root layout (`{}`): static metadata head tags and the route outlet.\n",
        manifest.root_layout.as_deref().unwrap_or("none")
    );
    if !slots.is_empty() {
        let _ = writeln!(
            out,
            "/// Parallel slots to render manually: {}.",
            slots
                .iter()
                .map(|s| format!("`@{s}`"))
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    out.push_str("#[component]\nfn RootLayout() -> Element {\n    rsx! {\n");
    for line in metadata::render_dioxus_head_rsx(root_meta, root_viewport).lines() {
        out.push_str("    ");
        out.push_str(line);
        out.push('\n');
    }
    if has_fonts {
        out.push_str("        document::Stylesheet { href: asset!(\"/assets/fonts.css\") }\n");
    }
    out.push_str("        Outlet::<Route> {}\n    }\n}\n\n");
    out
}

fn page_component(generator: &MainGen, page: &RouteEntry, root_meta: &Metadata) -> String {
    let name = generator
        .page_names
        .get(&page.file)
        .cloned()
        .unwrap_or_default();
    let fields = route_fields(page);
    let signature: Vec<String> = fields
        .iter()
        .map(|f| {
            if f.catch_all {
                format!("{}: Vec<String>", f.ident)
            } else {
                format!("{}: String", f.ident)
            }
        })
        .collect();
    let mut out = format!(
        "/// Page `{}` -> `{}`{}.\n",
        page.file,
        page.url_pattern,
        if page.legacy_pages {
            " (Pages Router)"
        } else {
            ""
        }
    );
    if page.segment_config.has_generate_metadata {
        out.push_str("/// TODO(yolo-next): port `generateMetadata` into a server function feeding `document::Title`.\n");
    }
    if !page.legacy_data_functions.is_empty() {
        let _ = writeln!(
            out,
            "/// TODO(yolo-next): port {} into `#[server]` loaders.",
            page.legacy_data_functions.join(", ")
        );
    }
    let _ = writeln!(
        out,
        "#[component]\nfn {name}({}) -> Element {{\n    rsx! {{",
        signature.join(", ")
    );
    if let Some(title) = page_title(root_meta, page) {
        let _ = writeln!(
            out,
            "        document::Title {{ \"{}\" }}",
            rust_string(&title, true)
        );
    }
    let _ = writeln!(
        out,
        "        main {{\n            h1 {{ \"{}\" }}",
        rust_string(&page.url_pattern, true)
    );
    for field in &fields {
        if field.catch_all {
            let _ = writeln!(
                out,
                "            p {{ \"{}: {{{}:?}}\" }}",
                field.param, field.ident
            );
        } else {
            let _ = writeln!(
                out,
                "            p {{ \"{}: {{{}}}\" }}",
                field.param, field.ident
            );
        }
    }
    out.push_str("        }\n    }\n}\n\n");
    out
}

/// `src/server_fns.rs` (route handlers + Server Actions), `None` when there is nothing to emit.
fn server_fns_rs(analysis: &ProjectAnalysis) -> Option<String> {
    let handlers: Vec<&RouteEntry> = analysis
        .routes
        .entries
        .iter()
        .filter(|e| e.kind == RouteKind::RouteHandler)
        .collect();
    let actions = action_fn_names(analysis);
    if handlers.is_empty() && actions.is_empty() {
        return None;
    }
    let mut names = UniqueNames::default();
    let mut out = String::from(
        "//! Route handlers and Server Actions mapped to Dioxus server functions by yolo-next.\n//! Bodies are stubs: port the original handler logic, keep the signatures.\n#![allow(unused_variables, clippy::unused_async)]\n\nuse dioxus::prelude::*;\n",
    );
    for entry in handlers {
        let path = axum_path(entry);
        let params: Vec<String> = route_fields(entry)
            .iter()
            .map(|f| format!("{}: String", f.ident))
            .collect();
        for method in &entry.methods {
            let attr = match method.as_str() {
                "GET" => "get",
                "POST" => "post",
                "PUT" => "put",
                "PATCH" => "patch",
                "DELETE" => "delete",
                _ => {
                    let _ = writeln!(
                        out,
                        "\n// {method} {} (`{}`): handled by the framework, no server function emitted.",
                        entry.url_pattern, entry.file
                    );
                    continue;
                }
            };
            let name = names.unique(&handler_fn_name(entry, method));
            let mut args = params.clone();
            if matches!(attr, "post" | "put" | "patch") {
                args.push(String::from("body: String"));
            }
            let _ = writeln!(
                out,
                "\n/// `{method} {}` from `{}`{}.\n#[{attr}(\"{path}\")]\npub async fn {name}({}) -> Result<String> {{\n    // TODO(yolo-next): port the handler body.\n    Ok(String::new())\n}}",
                entry.url_pattern,
                entry.file,
                if entry.legacy_pages {
                    " (Pages Router API route)"
                } else {
                    ""
                },
                args.join(", ")
            );
        }
    }
    for (name, module, export) in actions {
        let _ = writeln!(
            out,
            "\n/// Server Action `{export}` from `{module}`.\n#[post(\"/_actions/{}\")]\npub async fn {name}(input: String) -> Result<String> {{\n    // TODO(yolo-next): port the action body (receives the serialized form data).\n    Ok(input)\n}}",
            rust_string(&export, false)
        );
    }
    Some(out)
}

fn readme(analysis: &ProjectAnalysis, package: &str) -> String {
    let (pages, handlers, metadata) = crate::project::route_counts(analysis);
    format!(
        "# {package}\n\nMigrated from Next.js {} to Dioxus 0.8 fullstack by yolo-next ({pages} pages, {handlers} route handlers, {metadata} metadata routes).\n\n## Run\n\n```sh\ndx serve --platform web                    # client + dev server\ndx serve --platform web --features server  # fullstack (axum) with server functions\ndx bundle --platform web --release\n```\n\n## Layout\n\n- `src/main.rs`: `#[derive(Routable)] enum Route` mirroring `app/`, layouts as `#[layout]` blocks, `document::*` head tags from the static metadata.\n- `src/server_fns.rs`: `#[get]` / `#[post]` stubs for route handlers and Server Actions.\n- `assets/`: self-hosted fonts and `asset!()` images.\n\nMetadata routes (`sitemap.xml`, `robots.txt`, `manifest.webmanifest`, `opengraph-image`) and the proxy must be wired into `dioxus::server::router(App)`; see `MIGRATION-REPORT.md`.\n",
        analysis.next_version.as_deref().unwrap_or("16")
    )
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::project::analyze_project;
    use crate::routes::test_support::{cleanup, temp_project};

    fn file<'a>(files: &'a [EmittedFile], path: &str) -> &'a str {
        files
            .iter()
            .find(|f| f.path == path)
            .map_or_else(|| panic!("{path} missing"), |f| f.contents.as_str())
    }

    #[test]
    fn naming_helpers() {
        assert_eq!(rust_ident("postId"), "post_id");
        assert_eq!(rust_ident("my-slug"), "my_slug");
        assert_eq!(rust_ident("type"), "type_");
        assert_eq!(rust_ident("404"), "p_404");
        assert_eq!(crate_name("next-fixture"), "next_fixture");
        assert_eq!(crate_name("@acme/Site App"), "site_app");
        assert_eq!(pascal_ident("(marketing)"), "Marketing");
        assert_eq!(toml_string("a\"b"), "\"a\\\"b\"");
        assert_eq!(rust_string("{x}", true), "{{x}}");
        assert_eq!(layout_name("app", "app/blog/layout.tsx"), "BlogLayout");
        assert_eq!(
            layout_name("app", "app/(marketing)/layout.tsx"),
            "MarketingLayout"
        );
        assert_eq!(layout_name("app", "app/layout.tsx"), "RootLayout");
        assert_eq!(relative_route("/blog/:slug", "/blog"), "/:slug");
        assert_eq!(relative_route("/blog", "/blog"), "/");
    }

    #[test]
    fn emits_dioxus_project_from_fixture() {
        let dir = temp_project("emit-dioxus");
        let analysis = analyze_project(&dir);
        let files = emit_dioxus_project(&analysis);
        for expected in [
            "Cargo.toml",
            "Dioxus.toml",
            "src/main.rs",
            "src/server_fns.rs",
            "README.md",
        ] {
            assert!(
                files.iter().any(|f| f.path == expected),
                "missing {expected}"
            );
        }
        let has_proxy = analysis.proxy.is_some();
        assert_eq!(files.iter().any(|f| f.path == "src/proxy.rs"), has_proxy);
        let main = file(&files, "src/main.rs");
        assert!(main.contains("#[derive(Routable"), "{main}");
        assert!(main.contains("#[nest(\"/blog\")]"));
        assert!(main.contains("#[layout(BlogLayout)]"));
        assert!(main.contains("#[route(\"/:slug\")]"));
        assert!(main.contains("BlogSlug { slug: String }"));
        assert!(main.contains(":..slug"));
        assert!(main.contains("DocsSlugCatchAll { slug: Vec<String> }"));
        assert!(main.contains("#[route(\"/shop/:..filters\")]"));
        assert!(main.contains("#[layout(MarketingLayout)]"));
        assert!(main.contains("#[route(\"/pricing\")]"));
        assert!(main.contains("NotFound { segments: Vec<String> }"));
        assert!(main.contains("fn RootLayout() -> Element"));
        assert!(main.contains("document::Title { \"Yolo Fixture\" }"));
        assert!(main.contains("document::Title { \"Pricing | Yolo Fixture\" }"));
        assert!(main.contains("Outlet::<Route> {}"));
        assert!(main.contains("dioxus::serve"));
        assert_eq!(
            main.contains("const ASSET_0: Asset = "),
            !local_images(&analysis).is_empty()
        );
        assert!(!main.contains("PhotoModal"), "slot entries are not routed");
        let cargo = file(&files, "Cargo.toml");
        assert!(cargo.contains("name = \"temp_fixture\""));
        assert!(cargo.contains(
            "dioxus = { version = \"0.8.0-alpha.3\", features = [\"fullstack\", \"router\"] }"
        ));
        if has_proxy {
            assert!(cargo.contains("server = [\"dioxus/server\", \"dep:axum\", \"dep:tower\"]"));
        } else {
            assert!(cargo.contains("server = [\"dioxus/server\"]"));
        }
        let dx = file(&files, "Dioxus.toml");
        assert!(dx.contains("title = \"Yolo Fixture\""));
        assert!(dx.contains("index_on_404 = true"));
        assert!(!dx.contains("base_path"));
        let server_fns = file(&files, "src/server_fns.rs");
        assert!(server_fns.contains("#[get(\"/api/health\")]"));
        assert!(server_fns.contains("pub async fn api_health_get() -> Result<String>"));
        assert!(server_fns.contains("#[get(\"/api/posts/{id}\")]"));
        assert!(server_fns.contains("pub async fn api_posts_id_post(id: String, body: String)"));
        assert!(server_fns.contains("#[delete(\"/api/posts/{id}\")]"));
        cleanup(&dir);
    }

    #[test]
    fn real_fixture_when_present() {
        let root = Path::new("C:/src/yolo-stack/apps/next-fixture");
        if !root.exists() {
            return;
        }
        let analysis = analyze_project(root);
        let files = emit_dioxus_project(&analysis);
        let main = file(&files, "src/main.rs");
        assert!(
            main.contains("#[route(\"/:slug\")]") || main.contains("#[route(\"/blog/:slug\")]")
        );
        assert!(main.contains(":..slug"));
    }
}
