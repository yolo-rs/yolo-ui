// SPDX-License-Identifier: Apache-2.0
//! Leptos 0.8 lane emitter: `Cargo.toml`, `src/app.rs` (`leptos_router`), `src/main.rs` (axum).
//!
//! Shaped after `apps/shenron/crates/shenron-site`, the only Leptos consumer: every page route
//! carries a `:lang` locale prefix and is prerendered with `SsrMode::Static` when Next.js
//! would have rendered it statically (`generateStaticParams`, `dynamic = "force-static"` or a
//! numeric `revalidate`); visible strings go through `i18n::t`; head tags go through
//! `seo::SeoHead`; `next.config` redirects become an axum 308 middleware; no CSS is emitted
//! (tokens come from `yolo_tokens`).

use std::collections::BTreeSet;
use std::fmt::Write as _;

use serde_json::{Map, Value, json};

use crate::emit_dioxus::{
    NestItem, NestNode, RouteField, UniqueNames, action_fn_names, build_nest_tree, crate_name,
    handler_fn_name, layout_name, page_names, page_title, route_fields, routed_pages, rust_ident,
    rust_string, toml_string,
};
use crate::types::{EmittedFile, Metadata, ProjectAnalysis, RouteEntry, RouteKind, Viewport};
use crate::{metadata, proxy, rules};

/// Legacy French first-level segments of dragonballfr.com and their canonical replacement.
const LEGACY_SEGMENTS: [(&str, &str); 7] = [
    ("personnages", "characters"),
    ("encyclopedie", "wiki"),
    ("fiches", "wiki"),
    ("mangas", "manga"),
    ("tomes", "manga"),
    ("livres", "databooks"),
    ("guides", "databooks"),
];

/// Locale set of the generated site.
struct Locales {
    codes: Vec<String>,
    default: String,
}

impl Locales {
    fn of(analysis: &ProjectAnalysis) -> Self {
        match &analysis.config.i18n {
            Some(i18n) if !i18n.locales.is_empty() => {
                let default = if i18n.locales.contains(&i18n.default_locale) {
                    i18n.default_locale.clone()
                } else {
                    i18n.locales[0].clone()
                };
                Self { codes: i18n.locales.clone(), default }
            }
            _ => Self {
                codes: vec![String::from("fr"), String::from("en"), String::from("ja")],
                default: String::from("fr"),
            },
        }
    }
}

/// Everything the Leptos generators share.
struct Ctx<'a> {
    analysis: &'a ProjectAnalysis,
    package: String,
    app_dir: &'a str,
    locales: Locales,
    root_meta: Metadata,
    root_viewport: Option<Viewport>,
    pages: Vec<&'a RouteEntry>,
    names: std::collections::BTreeMap<String, String>,
    origin: String,
}

impl Ctx<'_> {
    fn page_name(&self, page: &RouteEntry) -> String {
        self.names.get(&page.file).cloned().unwrap_or_default()
    }

    /// `routes.<variant_snake>` message key prefix of a page.
    fn message_key(&self, page: &RouteEntry) -> String {
        format!("routes.{}", rust_ident(&self.page_name(page)))
    }

    fn handlers(&self) -> Vec<&RouteEntry> {
        self.analysis
            .routes
            .entries
            .iter()
            .filter(|e| e.kind == RouteKind::RouteHandler)
            .collect()
    }
}

/// Emits the Leptos project.
pub fn emit_leptos_project(analysis: &ProjectAnalysis) -> Vec<EmittedFile> {
    let (root_meta, root_viewport) = metadata::root_metadata(analysis);
    let app_dir = analysis.routes.app_dir.as_deref().unwrap_or("app");
    let pages = routed_pages(&analysis.routes);
    let names = page_names(
        &pages,
        &[
            "App",
            "NotFound",
            "SeoHead",
            "Route",
            "Routes",
            "Router",
            "ParentRoute",
            "Outlet",
            "Title",
            "Meta",
            "Link",
            "MetaTags",
        ],
    );
    let origin = root_meta.metadata_base.clone().map_or_else(
        || String::from("https://example.com"),
        |b| b.trim_end_matches('/').to_owned(),
    );
    let ctx = Ctx {
        analysis,
        package: crate_name(&analysis.name),
        app_dir,
        locales: Locales::of(analysis),
        root_meta,
        root_viewport,
        pages,
        names,
        origin,
    };
    let server_fns = server_fns_rs(&ctx);
    let has_proxy = analysis.proxy.is_some();
    let mut files = vec![
        file("Cargo.toml", cargo_toml(&ctx)),
        file("src/lib.rs", lib_rs(&ctx, server_fns.is_some(), has_proxy)),
        file("src/main.rs", main_rs(&ctx)),
        file("src/app.rs", app_rs(&ctx)),
        file("src/content.rs", content_rs(&ctx)),
        file("src/i18n.rs", i18n_rs(&ctx)),
        file("src/seo.rs", seo_rs(&ctx)),
        file("src/tokens.rs", tokens_rs(&ctx)),
        file("src/server/mod.rs", server_mod_rs()),
        file("src/server/redirects.rs", redirects_rs(&ctx)),
    ];
    if let Some(contents) = server_fns {
        files.push(file("src/server_fns.rs", contents));
    }
    if let Some(spec) = &analysis.proxy {
        files.push(file("src/proxy.rs", proxy::emit_axum_proxy(spec)));
    }
    let catalog = messages_json(&ctx);
    for code in &ctx.locales.codes {
        files.push(file(&format!("messages/{code}.json"), catalog.clone()));
    }
    files.push(file("README.md", readme(&ctx)));
    files
}

fn file(path: &str, contents: String) -> EmittedFile {
    EmittedFile { path: path.to_owned(), contents }
}

// ============================================================================
// Cargo.toml, lib.rs, main.rs
// ============================================================================

fn cargo_toml(ctx: &Ctx<'_>) -> String {
    format!(
        "# Generated by yolo-next. Leptos 0.8 SSR lane (axum + static route tree).\n[package]\nname = {name}\nversion = \"0.1.0\"\nedition = \"2024\"\npublish = false\n\n[lib]\ncrate-type = [\"cdylib\", \"rlib\"]\n\n[[bin]]\nname = {name}\npath = \"src/main.rs\"\nrequired-features = [\"ssr\"]\n\n[dependencies]\nleptos = \"0.8.23\"\nleptos_router = \"0.8.23\"\nleptos_meta = \"0.8.23\"\nserde = {{ version = \"1\", features = [\"derive\"] }}\nserde_json = \"1\"\n\n# server (ssr)\nleptos_axum = {{ version = \"0.8.23\", optional = true }}\naxum = {{ version = \"0.8\", optional = true }}\ntokio = {{ version = \"1\", features = [\"rt-multi-thread\", \"macros\"], optional = true }}\n\n# client (hydrate, wasm32)\nwasm-bindgen = {{ version = \"0.2\", optional = true }}\nconsole_error_panic_hook = {{ version = \"0.1\", optional = true }}\n\n[features]\ndefault = []\nhydrate = [\"leptos/hydrate\", \"dep:wasm-bindgen\", \"dep:console_error_panic_hook\"]\nssr = [\n    \"dep:leptos_axum\",\n    \"dep:axum\",\n    \"dep:tokio\",\n    \"leptos/ssr\",\n    \"leptos_meta/ssr\",\n    \"leptos_router/ssr\",\n]\n\n[package.metadata.leptos]\noutput-name = {name}\nsite-root = \"target/site\"\nsite-pkg-dir = \"pkg\"\nsite-addr = \"127.0.0.1:3000\"\nreload-port = 3001\nbin-features = [\"ssr\"]\nbin-default-features = false\nlib-features = [\"hydrate\"]\nlib-default-features = false\n",
        name = toml_string(&ctx.package)
    )
}

fn lib_rs(ctx: &Ctx<'_>, has_server_fns: bool, has_proxy: bool) -> String {
    let mut out = format!(
        "//! Generated by yolo-next from `{}` (Next.js {}): Leptos 0.8 SSR lane.\n//!\n//! Locale-prefixed static route tree (`/{{lang}}/...`), build-time i18n catalogs,\n//! `leptos_meta` SEO tags and an axum 308 redirect middleware.\n\n#![forbid(unsafe_code)]\n#![allow(\n    clippy::must_use_candidate,\n    clippy::module_name_repetitions,\n    clippy::unused_async\n)]\n\npub mod app;\npub mod content;\npub mod i18n;\npub mod seo;\npub mod tokens;\n",
        ctx.analysis.name,
        ctx.analysis.next_version.as_deref().unwrap_or("16")
    );
    if has_server_fns {
        out.push_str("pub mod server_fns;\n");
    }
    out.push_str("\n#[cfg(feature = \"ssr\")]\npub mod server;\n");
    if has_proxy {
        out.push_str("#[cfg(feature = \"ssr\")]\npub mod proxy;\n");
    }
    out.push_str("\n/// Client entry point: hydrates the server-rendered `<body>`.\n#[cfg(feature = \"hydrate\")]\n#[wasm_bindgen::prelude::wasm_bindgen]\npub fn hydrate() {\n    console_error_panic_hook::set_once();\n    leptos::mount::hydrate_body(app::App);\n}\n");
    out
}

fn main_rs(ctx: &Ctx<'_>) -> String {
    format!(
        "//! Axum server binary (feature `ssr`): prerenders the static routes, then serves.\n\n#[cfg(feature = \"ssr\")]\n#[tokio::main]\nasync fn main() {{\n    use axum::{{Router, routing::get}};\n    use leptos::prelude::*;\n    use leptos_axum::{{LeptosRoutes, generate_route_list_with_ssg}};\n    use {krate}::app::{{App, shell}};\n    use {krate}::server::{{legacy_redirects, root_redirect}};\n\n    let conf = get_configuration(None).expect(\"Cargo.toml [package.metadata.leptos]\");\n    let addr = conf.leptos_options.site_addr;\n    let leptos_options = conf.leptos_options;\n\n    // Walks the <Routes> tree once, then prerenders every SsrMode::Static route into site-root.\n    let (routes, static_routes) = generate_route_list_with_ssg(App);\n    static_routes.generate(&leptos_options).await;\n\n    let app = Router::new()\n        .route(\"/\", get(root_redirect))\n        .leptos_routes(&leptos_options, routes, {{\n            let leptos_options = leptos_options.clone();\n            move || shell(leptos_options.clone())\n        }})\n        .fallback(leptos_axum::file_and_error_handler(shell))\n        .layer(axum::middleware::from_fn(legacy_redirects))\n        .with_state(leptos_options);\n\n    let listener = tokio::net::TcpListener::bind(&addr)\n        .await\n        .expect(\"bind site-addr\");\n    axum::serve(listener, app.into_make_service())\n        .await\n        .expect(\"axum serve\");\n}}\n\n#[cfg(not(feature = \"ssr\"))]\nfn main() {{}}\n",
        krate = ctx.package
    )
}

// ============================================================================
// app.rs
// ============================================================================

/// `true` when Next.js would prerender the page (static generation).
fn is_static_page(page: &RouteEntry) -> bool {
    let config = &page.segment_config;
    config.has_generate_static_params
        || config.dynamic.as_deref() == Some("force-static")
        || config.revalidate.as_ref().is_some_and(Value::is_number)
}

/// `ssr=` attribute of a page route.
fn ssr_mode(page: &RouteEntry) -> String {
    if is_static_page(page) {
        format!(
            "SsrMode::Static(StaticRoute::new().prerender_params(|| async move {{ content::static_params_for(\"{}\").await }}))",
            rust_string(&page.url_pattern, false)
        )
    } else {
        String::from("SsrMode::OutOfOrder")
    }
}

/// Leptos path of a page with the `:lang` prefix, relative to `parent` (`""` for top level).
fn leptos_route_path(page: &RouteEntry, fields: &[RouteField], parent: &str) -> String {
    let full = leptos_full_path(&page.url_pattern, fields);
    relative_leptos(&full, parent)
}

/// `/:lang/blog/:slug` built with sanitized param identifiers.
fn leptos_full_path(url_pattern: &str, fields: &[RouteField]) -> String {
    let mut fields = fields.iter();
    let mut out = String::from("/:lang");
    for segment in url_pattern.split('/').filter(|s| !s.is_empty()) {
        let (kind, _, _, _) = crate::routes::classify_segment(segment);
        out.push('/');
        match kind {
            crate::types::SegmentKind::Dynamic => match fields.next() {
                Some(f) => {
                    out.push(':');
                    out.push_str(&f.ident);
                }
                None => out.push_str(segment),
            },
            crate::types::SegmentKind::CatchAll | crate::types::SegmentKind::OptionalCatchAll => {
                match fields.next() {
                    Some(f) => {
                        out.push('*');
                        out.push_str(&f.ident);
                    }
                    None => out.push_str(segment),
                }
            }
            _ => out.push_str(segment),
        }
    }
    out
}

/// Child path relative to a `ParentRoute` (`""` for the index child).
fn relative_leptos(full: &str, parent: &str) -> String {
    if parent.is_empty() {
        return full.to_owned();
    }
    full.strip_prefix(parent)
        .unwrap_or(full)
        .trim_start_matches('/')
        .to_owned()
}

/// Emits `<Route/>` / `<ParentRoute>` elements for one nesting level.
fn emit_routes(ctx: &Ctx<'_>, node: &NestNode<'_>, parent: &str, indent: usize, out: &mut String) {
    let pad = " ".repeat(indent);
    for item in &node.items {
        match item {
            NestItem::Page(page) => {
                let fields = route_fields(page);
                let path = leptos_route_path(page, &fields, parent);
                if let Some(Value::Number(seconds)) = &page.segment_config.revalidate {
                    let _ = writeln!(
                        out,
                        "{pad}// ISR: Next.js revalidated every {seconds}s; chain `.regenerate(|params| async move {{ /* refresh */ }})` on the StaticRoute."
                    );
                }
                let _ = writeln!(
                    out,
                    "{pad}<Route\n{pad}    path=path!(\"{path}\")\n{pad}    view={}\n{pad}    ssr={}\n{pad}/>",
                    ctx.page_name(page),
                    ssr_mode(page)
                );
            }
            NestItem::Nest(child) => {
                let full = leptos_full_path(&child.url_prefix, &[]);
                let path = relative_leptos(&full, parent);
                let layout = child
                    .layout
                    .map(|l| layout_name(ctx.app_dir, l))
                    .unwrap_or_default();
                let _ = writeln!(
                    out,
                    "{pad}<ParentRoute path=path!(\"{path}\") view={layout}>"
                );
                emit_routes(ctx, child, &full, indent + 4, out);
                let _ = writeln!(out, "{pad}</ParentRoute>");
            }
        }
    }
}

/// Canonical (locale-independent) path expression of a page.
fn canonical_path_expr(page: &RouteEntry, fields: &[RouteField]) -> String {
    if fields.is_empty() {
        return if page.url_pattern == "/" {
            String::from("String::new()")
        } else {
            format!(
                "String::from(\"{}\")",
                rust_string(&page.url_pattern, false)
            )
        };
    }
    let mut fields = fields.iter();
    let mut template = String::new();
    let mut args = Vec::new();
    for segment in page.url_pattern.split('/').filter(|s| !s.is_empty()) {
        template.push('/');
        let (_, param, _, _) = crate::routes::classify_segment(segment);
        if param.is_some()
            && let Some(field) = fields.next()
        {
            {
                template.push_str("{}");
                args.push(format!("{}.get()", field.ident));
                continue;
            }
        }
        template.push_str(
            &rust_string(segment, false)
                .replace('{', "{{")
                .replace('}', "}}"),
        );
    }
    format!("format!(\"{template}\", {})", args.join(", "))
}

fn page_component(ctx: &Ctx<'_>, page: &RouteEntry) -> String {
    let name = ctx.page_name(page);
    let key = ctx.message_key(page);
    let fields = route_fields(page);
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
        out.push_str("/// TODO(yolo-next): port `generateMetadata` into a `#[server]` resource feeding `SeoHead`.\n");
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
        "#[component]\nfn {name}() -> impl IntoView {{\n    let lang = current_lang();"
    );
    for field in &fields {
        let _ = writeln!(out, "    let {} = param(\"{}\");", field.ident, field.ident);
    }
    let _ = writeln!(
        out,
        "    let path = {};\n    view! {{\n        <SeoHead\n            lang=lang.get()\n            title=t(&lang.get(), \"{key}.title\")\n            description=t(&lang.get(), \"{key}.description\")\n            canonical=content::url(&lang.get(), &path)\n        />\n        <main>\n            <h1>{{move || t(&lang.get(), \"{key}.title\")}}</h1>",
        canonical_path_expr(page, &fields)
    );
    for field in &fields {
        let _ = writeln!(out, "            <p>{{move || {}.get()}}</p>", field.ident);
    }
    out.push_str("        </main>\n    }\n}\n\n");
    out
}

fn app_rs(ctx: &Ctx<'_>) -> String {
    let manifest = &ctx.analysis.routes;
    let tree = build_nest_tree(manifest, ctx.app_dir, false);
    let mut routes = String::new();
    emit_routes(ctx, &tree, "", 16, &mut routes);

    let mut layouts: Vec<(String, String)> = Vec::new();
    let mut seen = BTreeSet::new();
    collect_layouts(ctx, &tree, &mut seen, &mut layouts);

    let mut out = format!(
        "//! Application shell, locale-prefixed route tree and page components.\n//!\n//! Generated by yolo-next from `{}`; page bodies are placeholders to port by hand.\n#![allow(clippy::needless_pass_by_value)]\n\nuse leptos::prelude::*;\nuse leptos_meta::{{MetaTags, provide_meta_context}};\nuse leptos_router::{{\n    SsrMode,\n    components::{{Outlet, ParentRoute, Route, Router, Routes}},\n    hooks::{{use_location, use_params_map}},\n    path,\n    static_routes::StaticRoute,\n}};\n\nuse crate::{{\n    content,\n    i18n::{{DEFAULT_LANG, lang_from_path, t}},\n    seo::SeoHead,\n}};\n\n/// HTML document shell rendered by the server for every route.\npub fn shell(options: LeptosOptions) -> impl IntoView {{\n    view! {{\n        <!DOCTYPE html>\n        <html lang=DEFAULT_LANG>\n            <head>\n                <meta charset=\"utf-8\"/>\n                <meta name=\"viewport\" content=\"{}\"/>\n                <AutoReload options=options.clone()/>\n                <HydrationScripts options/>\n                <MetaTags/>\n            </head>\n            <body>\n                <App/>\n            </body>\n        </html>\n    }}\n}}\n\n/// Locale of the current route (`:lang` segment, default locale otherwise).\nfn current_lang() -> Memo<String> {{\n    let params = use_params_map();\n    Memo::new(move |_| {{\n        params\n            .read()\n            .get(\"lang\")\n            .unwrap_or_else(|| DEFAULT_LANG.to_owned())\n    }})\n}}\n\n/// One route param as a memo (empty when absent).\nfn param(name: &'static str) -> Memo<String> {{\n    let params = use_params_map();\n    Memo::new(move |_| params.read().get(name).unwrap_or_default())\n}}\n\n/// Router host: static route tree with the `:lang` prefix on every page.\n#[component]\npub fn App() -> impl IntoView {{\n    provide_meta_context();\n    view! {{\n        <Router>\n            <Routes fallback=|| view! {{ <NotFound/> }}>\n{routes}            </Routes>\n        </Router>\n    }}\n}}\n\n",
        ctx.analysis.name,
        viewport_content(ctx),
    );
    for (file, name) in &layouts {
        let _ = write!(
            out,
            "/// Layout `{file}`.\n#[component]\nfn {name}() -> impl IntoView {{\n    view! {{ <Outlet/> }}\n}}\n\n"
        );
    }
    for page in &ctx.pages {
        out.push_str(&page_component(ctx, page));
    }
    let _ = writeln!(
        out,
        "/// Fallback route{}.\n#[component]\nfn NotFound() -> impl IntoView {{\n    let location = use_location();\n    let lang = Memo::new(move |_| lang_from_path(&location.pathname.get()).to_owned());\n    view! {{\n        <main>\n            <h1>{{move || t(&lang.get(), \"routes.not_found.title\")}}</h1>\n            <p>{{move || t(&lang.get(), \"routes.not_found.description\")}}</p>\n        </main>\n    }}\n}}",
        manifest
            .root_not_found
            .as_deref()
            .map_or(String::new(), |f| format!(" (port `{f}`)"))
    );
    out
}

fn collect_layouts(
    ctx: &Ctx<'_>,
    node: &NestNode<'_>,
    seen: &mut BTreeSet<String>,
    out: &mut Vec<(String, String)>,
) {
    for item in &node.items {
        if let NestItem::Nest(child) = item {
            if let Some(layout) = child.layout
                && seen.insert(layout.to_owned())
            {
                out.push((layout.to_owned(), layout_name(ctx.app_dir, layout)));
            }
            collect_layouts(ctx, child, seen, out);
        }
    }
}

fn viewport_content(ctx: &Ctx<'_>) -> String {
    let viewport = ctx
        .root_viewport
        .clone()
        .unwrap_or_else(metadata::default_viewport);
    let html = metadata::render_head_html(&Metadata::default(), Some(&viewport));
    html.lines()
        .find_map(|line| {
            line.strip_prefix("<meta name=\"viewport\" content=\"")
                .and_then(|rest| rest.strip_suffix("\">"))
        })
        .unwrap_or("width=device-width, initial-scale=1")
        .to_owned()
}

// ============================================================================
// content.rs, i18n.rs, seo.rs, tokens.rs
// ============================================================================

fn content_rs(ctx: &Ctx<'_>) -> String {
    let static_pages: Vec<&RouteEntry> = ctx
        .pages
        .iter()
        .copied()
        .filter(|p| p.params.is_empty())
        .collect();
    let dynamic_pages: Vec<&RouteEntry> = ctx
        .pages
        .iter()
        .copied()
        .filter(|p| !p.params.is_empty())
        .collect();
    let mut out = format!(
        "//! Content catalog: locale-independent paths and the static params used to prerender the\n//! `SsrMode::Static` routes (`leptos_axum::generate_route_list_with_ssg`).\n\nuse leptos_router::static_routes::StaticParamsMap;\n\nuse crate::i18n::{{DEFAULT_LANG, LANGS}};\n\n/// Public origin (Next.js `metadataBase`), used for canonical URLs, `og:url` and hreflang.\npub const SITE_ORIGIN: &str = \"{}\";\n\n/// Static routes: `(Next.js URL pattern, locale-independent path)`.\npub const STATIC_ROUTES: &[(&str, &str)] = &[\n",
        rust_string(&ctx.origin, false)
    );
    for page in &static_pages {
        let path = if page.url_pattern == "/" {
            ""
        } else {
            page.url_pattern.as_str()
        };
        let _ = writeln!(
            out,
            "    (\"{}\", \"{}\"),",
            rust_string(&page.url_pattern, false),
            rust_string(path, false)
        );
    }
    out.push_str("];\n\n/// Dynamic routes whose params come from data: `(Next.js URL pattern, source of `generateStaticParams`)`.\npub const DYNAMIC_ROUTES: &[(&str, &str)] = &[\n");
    for page in &dynamic_pages {
        let _ = writeln!(
            out,
            "    (\"{}\", \"{}\"),",
            rust_string(&page.url_pattern, false),
            rust_string(&page.file, false)
        );
    }
    out.push_str("];\n\n/// Locale codes as owned strings (static params need `Vec<String>`).\npub fn langs() -> Vec<String> {\n    LANGS.iter().map(|l| (*l).to_owned()).collect()\n}\n\n/// Absolute URL of a locale-independent path (`\"\"` or `/blog/hello`) in `lang`.\npub fn url(lang: &str, path: &str) -> String {\n    format!(\"{SITE_ORIGIN}/{lang}{path}\")\n}\n\n/// Locale-independent path of an absolute URL or of a locale-prefixed path.\npub fn strip_locale(url_or_path: &str) -> String {\n    let path = url_or_path.strip_prefix(SITE_ORIGIN).unwrap_or(url_or_path);\n    let trimmed = path.trim_start_matches('/');\n    match trimmed.split_once('/') {\n        Some((first, rest)) if LANGS.contains(&first) => format!(\"/{rest}\"),\n        None if LANGS.contains(&trimmed) => String::new(),\n        _ if path.is_empty() => String::new(),\n        _ => format!(\"/{trimmed}\"),\n    }\n}\n\n/// Every prerendered path: static routes expanded per locale, dynamic routes once enumerated.\npub async fn all_paths() -> Vec<String> {\n    let mut paths = Vec::new();\n    for lang in LANGS {\n        for (_, path) in STATIC_ROUTES {\n            paths.push(format!(\"/{lang}{path}\"));\n        }\n    }\n");
    for page in &dynamic_pages {
        let _ = writeln!(
            out,
            "    // TODO: enumerate from data: `{}` (Next.js `generateStaticParams` in \"{}\").",
            page.url_pattern, page.file
        );
    }
    out.push_str("    paths\n}\n\n/// Static params of one route (`lang` plus the route's own params) for `SsrMode::Static`.\npub async fn static_params_for(route: &str) -> StaticParamsMap {\n    let mut map = StaticParamsMap::new();\n    map.insert(\"lang\", langs());\n    match route {\n");
    for page in &dynamic_pages {
        let _ = writeln!(
            out,
            "        \"{}\" => {{\n            // TODO: enumerate from data (Next.js `generateStaticParams` in \"{}\").",
            rust_string(&page.url_pattern, false),
            page.file
        );
        for field in route_fields(page) {
            let _ = writeln!(
                out,
                "            map.insert(\"{}\", Vec::new());",
                field.ident
            );
        }
        out.push_str("        }\n");
    }
    out.push_str("        _ => {}\n    }\n    map\n}\n\n/// Default locale home path (`/fr`).\npub fn default_home() -> String {\n    format!(\"/{DEFAULT_LANG}\")\n}\n");
    out
}

/// `og:locale` of a locale code (`fr` -> `fr_FR`).
fn og_locale(code: &str) -> String {
    if let Some((lang, region)) = code.split_once('-') {
        return format!("{lang}_{}", region.to_ascii_uppercase());
    }
    let region = match code {
        "en" => "US",
        "ja" => "JP",
        "zh" => "CN",
        "ko" => "KR",
        "sv" => "SE",
        "da" => "DK",
        "cs" => "CZ",
        "el" => "GR",
        "uk" => "UA",
        "hi" => "IN",
        "ar" => "SA",
        other => return format!("{other}_{}", other.to_ascii_uppercase()),
    };
    format!("{code}_{region}")
}

fn i18n_rs(ctx: &Ctx<'_>) -> String {
    let codes: Vec<String> = ctx
        .locales
        .codes
        .iter()
        .map(|c| format!("\"{}\"", rust_string(c, false)))
        .collect();
    let sources: Vec<String> = ctx
        .locales
        .codes
        .iter()
        .map(|c| {
            format!(
                "    (\"{c}\", include_str!(\"../messages/{c}.json\")),",
                c = rust_string(c, false)
            )
        })
        .collect();
    let og_arms: Vec<String> = ctx
        .locales
        .codes
        .iter()
        .map(|c| {
            format!(
                "        \"{}\" => \"{}\",",
                rust_string(c, false),
                og_locale(c)
            )
        })
        .collect();
    format!(
        "//! Build-time i18n: `messages/{{lang}}.json` catalogs behind [`t`] / [`tf`].\n//!\n//! Every visible string goes through these helpers; the catalogs are embedded with\n//! `include_str!` and parsed once (`OnceLock`). Nested JSON objects flatten to dotted keys\n//! (`routes.home.title`).\n\nuse std::collections::HashMap;\nuse std::sync::OnceLock;\n\nuse serde_json::Value;\n\n/// Supported locale codes in canonical order (URL prefix, `hreflang`, sitemap order).\npub const LANGS: [&str; {n}] = [{codes}];\n/// Locale used when the URL carries none (`x-default`, root redirect).\npub const DEFAULT_LANG: &str = \"{default}\";\n\nconst CATALOG_SOURCES: [(&str, &str); {n}] = [\n{sources}\n];\n\ntype Catalog = HashMap<String, String>;\n\nfn catalogs() -> &'static HashMap<&'static str, Catalog> {{\n    static CATALOGS: OnceLock<HashMap<&'static str, Catalog>> = OnceLock::new();\n    CATALOGS.get_or_init(|| {{\n        CATALOG_SOURCES\n            .iter()\n            .map(|(lang, json)| (*lang, parse(json)))\n            .collect()\n    }})\n}}\n\nfn parse(json: &str) -> Catalog {{\n    let value: Value = serde_json::from_str(json).unwrap_or(Value::Null);\n    let mut out = Catalog::new();\n    flatten(&value, String::new(), &mut out);\n    out\n}}\n\nfn flatten(value: &Value, prefix: String, out: &mut Catalog) {{\n    match value {{\n        Value::Object(map) => {{\n            for (key, child) in map {{\n                let full = if prefix.is_empty() {{\n                    key.clone()\n                }} else {{\n                    format!(\"{{prefix}}.{{key}}\")\n                }};\n                flatten(child, full, out);\n            }}\n        }}\n        Value::String(text) => {{\n            out.insert(prefix, text.clone());\n        }}\n        Value::Null => {{}}\n        other => {{\n            out.insert(prefix, other.to_string());\n        }}\n    }}\n}}\n\n/// Looks up `key` in the `lang` catalog, falling back to the default locale, then to the key\n/// itself (a missing translation is visible, never a panic).\npub fn t(lang: &str, key: &str) -> String {{\n    let catalogs = catalogs();\n    catalogs\n        .get(lang)\n        .and_then(|catalog| catalog.get(key))\n        .or_else(|| catalogs.get(DEFAULT_LANG).and_then(|catalog| catalog.get(key)))\n        .cloned()\n        .unwrap_or_else(|| key.to_owned())\n}}\n\n/// Like [`t`] but substitutes `{{name}}` placeholders.\npub fn tf(lang: &str, key: &str, args: &[(&str, &str)]) -> String {{\n    let mut out = t(lang, key);\n    for (name, value) in args {{\n        out = out.replace(&format!(\"{{{{{{name}}}}}}\"), value);\n    }}\n    out\n}}\n\n/// `true` when `code` is a supported locale.\npub fn is_lang(code: &str) -> bool {{\n    LANGS.contains(&code)\n}}\n\n/// Locale read from the first segment of an absolute path (default locale otherwise).\npub fn lang_from_path(path: &str) -> &'static str {{\n    let first = path.trim_start_matches('/').split('/').next().unwrap_or_default();\n    LANGS\n        .iter()\n        .copied()\n        .find(|lang| *lang == first)\n        .unwrap_or(DEFAULT_LANG)\n}}\n\n/// `og:locale` value of a locale code.\npub fn og_locale(lang: &str) -> &'static str {{\n    match lang {{\n{og_arms}\n        _ => \"{default_og}\",\n    }}\n}}\n\n/// Keys of the default catalog (tests and tooling).\npub fn keys() -> Vec<&'static str> {{\n    catalogs()\n        .get(DEFAULT_LANG)\n        .map(|catalog| catalog.keys().map(String::as_str).collect())\n        .unwrap_or_default()\n}}\n",
        n = ctx.locales.codes.len(),
        codes = codes.join(", "),
        default = rust_string(&ctx.locales.default, false),
        sources = sources.join("\n"),
        og_arms = og_arms.join("\n"),
        default_og = og_locale(&ctx.locales.default),
    )
}

/// Extra root head tags (icons, manifest, `og:site_name`, `twitter:site`, verification...)
/// rendered through `metadata::render_leptos_head`, minus the tags `SeoHead` owns.
fn extra_head_lines(ctx: &Ctx<'_>) -> Vec<String> {
    let owned = [
        "<Title",
        "<Meta charset",
        "<Meta name=\"viewport\"",
        "<Meta name=\"description\"",
        "<Link rel=\"canonical\"",
        "<Link rel=\"alternate\"",
        "<Meta property=\"og:title\"",
        "<Meta property=\"og:description\"",
        "<Meta property=\"og:url\"",
        "<Meta property=\"og:type\"",
        "<Meta property=\"og:locale\"",
        "<Meta name=\"twitter:card\"",
        "<Meta name=\"twitter:title\"",
        "<Meta name=\"twitter:description\"",
    ];
    metadata::render_leptos_head(&ctx.root_meta, ctx.root_viewport.as_ref())
        .lines()
        .filter(|line| !owned.iter().any(|prefix| line.starts_with(prefix)))
        .map(str::to_owned)
        .collect()
}

fn seo_rs(ctx: &Ctx<'_>) -> String {
    let extras = extra_head_lines(ctx);
    let mut out = String::from(
        "//! SEO head tags (`leptos_meta`): title, description, canonical, hreflang alternates,\n//! Open Graph and Twitter cards. Site-wide tags come from the Next.js root `metadata` export.\n\nuse leptos::prelude::*;\nuse leptos_meta::{Link, Meta, Title};\n\nuse crate::{\n    content::{strip_locale, url},\n    i18n::{DEFAULT_LANG, LANGS, og_locale},\n};\n\n/// Emits the `<head>` tags of one page.\n///\n/// `canonical` is the absolute URL of the page in `lang`; the hreflang alternates swap the\n/// locale prefix for every supported locale plus `x-default`.\n#[component]\npub fn SeoHead(lang: String, title: String, description: String, canonical: String) -> impl IntoView {\n    let path = strip_locale(&canonical);\n    let alternates = LANGS\n        .iter()\n        .map(|l| view! { <Link rel=\"alternate\" hreflang=*l href=url(l, &path)/> })\n        .collect_view();\n    let x_default = url(DEFAULT_LANG, &path);\n    view! {\n        <Title text=title.clone()/>\n        <Meta name=\"description\" content=description.clone()/>\n        <Link rel=\"canonical\" href=canonical.clone()/>\n        {alternates}\n        <Link rel=\"alternate\" hreflang=\"x-default\" href=x_default/>\n        <Meta property=\"og:title\" content=title/>\n        <Meta property=\"og:description\" content=description/>\n        <Meta property=\"og:url\" content=canonical/>\n        <Meta property=\"og:type\" content=\"website\"/>\n        <Meta property=\"og:locale\" content=og_locale(&lang)/>\n        <Meta name=\"twitter:card\" content=\"summary_large_image\"/>\n",
    );
    for line in &extras {
        out.push_str("        ");
        out.push_str(line);
        out.push('\n');
    }
    out.push_str("    }\n}\n");
    out
}

fn tokens_rs(ctx: &Ctx<'_>) -> String {
    let theme = &ctx.analysis.yolo_config.theme;
    format!(
        "//! Design tokens: the palette, shapes and type scale come from `yolo_tokens` (HCT / CAM16\n//! engine of yolo-ui) for the preset below; no stylesheet is generated by the migration.\n//!\n//! Add `yolo-tokens` to `Cargo.toml` and emit the CSS variables with\n//! `yolo_tokens::ThemeTokens::from_preset(PRESET)` (see `yolo-ui/crates/yolo-tokens`).\n\n/// Theme preset id selected by the migration (`yolo.config` `theme.preset`).\npub const PRESET: &str = \"{}\";\n/// Dark scheme by default.\npub const DARK: bool = {};\n/// Custom seed color, when the Next.js project configured one.\npub const SEED_HEX: Option<&str> = {};\n",
        rust_string(&theme.preset, false),
        theme.dark,
        theme
            .seed_hex
            .as_deref()
            .map_or(String::from("None"), |hex| format!(
                "Some(\"{}\")",
                rust_string(hex, false)
            ))
    )
}

// ============================================================================
// server/redirects.rs
// ============================================================================

fn server_mod_rs() -> String {
    String::from(
        "//! Axum-side handlers (`ssr` feature only): 308 redirects.\n\npub mod redirects;\n\npub use redirects::{legacy_redirects, root_redirect};\n",
    )
}

fn redirects_rs(ctx: &Ctx<'_>) -> String {
    let config = &ctx.analysis.config;
    let mut table = Vec::new();
    let mut notes = Vec::new();
    let mut uses_legacy = false;
    for rule in &config.redirects {
        if LEGACY_SEGMENTS
            .iter()
            .any(|(from, _)| rule.source.contains(from))
        {
            uses_legacy = true;
        }
        match rules::compile_source(&rule.source) {
            Ok(compiled) => {
                let status = rules::redirect_status(rule);
                let conditions = if rule.has.is_empty() && rule.missing.is_empty() {
                    String::new()
                } else {
                    String::from(" // has/missing conditions are not evaluated by this table")
                };
                table.push(format!(
                    "    (\"{}\", \"{}\", {status}),{conditions}",
                    rust_string(&compiled.axum_pattern(), false),
                    rust_string(&rule.destination, false)
                ));
            }
            Err(diagnostic) => notes.push(format!(
                "// Skipped `{}` -> `{}`: {}",
                rule.source, rule.destination, diagnostic.message
            )),
        }
    }
    if config.redirects_dynamic {
        notes.push(String::from(
            "// `redirects()` in next.config could not be evaluated statically; port it by hand.",
        ));
    }
    let legacy_table: Vec<String> = if uses_legacy {
        LEGACY_SEGMENTS
            .iter()
            .map(|(from, to)| format!("    (\"{from}\", \"{to}\"),"))
            .collect()
    } else {
        Vec::new()
    };
    format!(
        "//! 308 redirects: `/` -> `/{{default locale}}`, the `next.config` `redirects()` table and the legacy\n//! French first-level segments, as an axum middleware.\n\nuse axum::{{\n    extract::Request,\n    middleware::Next,\n    response::{{IntoResponse, Redirect, Response}},\n}};\n\nuse crate::i18n::{{DEFAULT_LANG, is_lang}};\n\n/// Permanent redirect status used by the tables.\npub const PERMANENT: u16 = 308;\n\n/// `(axum pattern, destination with `:param` placeholders, status)` from `next.config` `redirects()`.\npub const REDIRECTS: &[(&str, &str, u16)] = &[\n{table}\n];\n{notes}\n/// Legacy first-level segments and their canonical replacement (empty when the project has none).\npub const LEGACY_SEGMENTS: &[(&str, &str)] = &[\n{legacy}\n];\n\n/// Matches an axum-style pattern (`/old/{{slug}}`, `/legacy/{{*path}}`) against a path and\n/// returns the captured params.\nfn match_pattern(pattern: &str, path: &str) -> Option<Vec<(String, String)>> {{\n    let mut params = Vec::new();\n    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();\n    let mut index = 0;\n    for part in pattern.split('/').filter(|s| !s.is_empty()) {{\n        if let Some(name) = part.strip_prefix(\"{{*\").and_then(|p| p.strip_suffix('}}')) {{\n            params.push((name.to_owned(), segments[index..].join(\"/\")));\n            return Some(params);\n        }}\n        if let Some(name) = part.strip_prefix('{{').and_then(|p| p.strip_suffix('}}')) {{\n            params.push((name.to_owned(), (*segments.get(index)?).to_owned()));\n        }} else if segments.get(index).copied() != Some(part) {{\n            return None;\n        }}\n        index += 1;\n    }}\n    (index == segments.len()).then_some(params)\n}}\n\n/// Destination of a configured redirect for `path`, with `:param` placeholders substituted.\npub fn redirect_for(path: &str) -> Option<(String, u16)> {{\n    REDIRECTS.iter().find_map(|(pattern, destination, status)| {{\n        let params = match_pattern(pattern, path)?;\n        let mut target = (*destination).to_owned();\n        for (name, value) in params {{\n            target = target\n                .replace(&format!(\":{{name}}*\"), &value)\n                .replace(&format!(\":{{name}}\"), &value);\n        }}\n        Some((target, *status))\n    }})\n}}\n\n/// Canonical path for a legacy path, or `None` when the path is already canonical. A missing\n/// locale prefix is completed with the default locale.\npub fn rewrite_legacy_path(path: &str) -> Option<String> {{\n    let mut segments = path.trim_start_matches('/').split('/').filter(|s| !s.is_empty());\n    let first = segments.next()?;\n    let (lang, legacy) = if is_lang(first) {{\n        (first, segments.next()?)\n    }} else {{\n        (DEFAULT_LANG, first)\n    }};\n    let canonical = LEGACY_SEGMENTS\n        .iter()\n        .find(|(from, _)| *from == legacy)\n        .map(|(_, to)| *to)?;\n    let mut out = format!(\"/{{lang}}/{{canonical}}\");\n    for rest in segments {{\n        out.push('/');\n        out.push_str(rest);\n    }}\n    Some(out)\n}}\n\n/// Axum middleware: answers 308 (or the configured status) for redirected paths, otherwise\n/// passes the request through. The query string is preserved.\npub async fn legacy_redirects(req: Request, next: Next) -> Response {{\n    let path = req.uri().path();\n    let hit = redirect_for(path).or_else(|| rewrite_legacy_path(path).map(|t| (t, PERMANENT)));\n    if let Some((mut target, status)) = hit {{\n        if let Some(query) = req.uri().query() {{\n            target.push('?');\n            target.push_str(query);\n        }}\n        let redirect = if status == PERMANENT {{\n            Redirect::permanent(&target)\n        }} else {{\n            Redirect::temporary(&target)\n        }};\n        return redirect.into_response();\n    }}\n    next.run(req).await\n}}\n\n/// `GET /` -> 308 to the default locale home.\npub async fn root_redirect() -> Redirect {{\n    Redirect::permanent(&format!(\"/{{DEFAULT_LANG}}\"))\n}}\n",
        table = table.join("\n"),
        notes = if notes.is_empty() {
            String::new()
        } else {
            format!("{}\n", notes.join("\n"))
        },
        legacy = legacy_table.join("\n"),
    )
}

// ============================================================================
// server_fns.rs, messages, README
// ============================================================================

fn server_fns_rs(ctx: &Ctx<'_>) -> Option<String> {
    let handlers = ctx.handlers();
    let actions = action_fn_names(ctx.analysis);
    if handlers.is_empty() && actions.is_empty() {
        return None;
    }
    let mut names = UniqueNames::default();
    let mut out = String::from(
        "//! Route handlers and Server Actions mapped to Leptos server functions by yolo-next.\n//! Bodies are stubs: port the original handler logic, keep the signatures.\n#![allow(unused_variables)]\n\nuse leptos::prelude::*;\n",
    );
    for entry in handlers {
        let params: Vec<String> = route_fields(entry)
            .iter()
            .map(|f| format!("{}: String", f.ident))
            .collect();
        for method in &entry.methods {
            let name = names.unique(&handler_fn_name(entry, method));
            let mut args = params.clone();
            if matches!(method.as_str(), "POST" | "PUT" | "PATCH") {
                args.push(String::from("body: String"));
            }
            let _ = writeln!(
                out,
                "\n/// `{method} {}` from `{}`{}.\n#[server]\npub async fn {name}({}) -> Result<String, ServerFnError> {{\n    // TODO(yolo-next): port the handler body.\n    Ok(String::new())\n}}",
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
            "\n/// Server Action `{export}` from `{module}`.\n#[server]\npub async fn {name}(input: String) -> Result<String, ServerFnError> {{\n    // TODO(yolo-next): port the action body (receives the serialized form data).\n    Ok(input)\n}}"
        );
    }
    Some(out)
}

/// Human label of a page (`BlogSlug` -> `Blog Slug`).
fn humanize(name: &str) -> String {
    let mut out = String::new();
    for (i, c) in name.chars().enumerate() {
        if c.is_ascii_uppercase() && i > 0 {
            out.push(' ');
        }
        out.push(c);
    }
    out
}

/// Message catalog (same source strings for every locale, marked for translation).
fn messages_json(ctx: &Ctx<'_>) -> String {
    let site_name = ctx
        .root_meta
        .application_name
        .clone()
        .or_else(|| metadata::resolve_title(None, &ctx.root_meta))
        .unwrap_or_else(|| ctx.analysis.name.clone());
    let site_description = ctx.root_meta.description.clone().unwrap_or_default();
    let mut routes = Map::new();
    for page in &ctx.pages {
        let own = page
            .segment_config
            .static_metadata
            .clone()
            .unwrap_or_default();
        let title =
            page_title(&ctx.root_meta, page).unwrap_or_else(|| humanize(&ctx.page_name(page)));
        let description = own
            .description
            .clone()
            .or_else(|| ctx.root_meta.description.clone())
            .unwrap_or_default();
        routes.insert(
            rust_ident(&ctx.page_name(page)),
            json!({ "title": title, "description": description }),
        );
    }
    routes.insert(
        String::from("not_found"),
        json!({ "title": "Not found", "description": "The requested page does not exist." }),
    );
    let mut text = serde_json::to_string_pretty(&json!({
        "_todo": "Translate every value: copied from the Next.js static metadata (source language) by yolo-next.",
        "site": { "name": site_name, "description": site_description },
        "routes": routes,
    }))
    .unwrap_or_default();
    text.push('\n');
    text
}

fn readme(ctx: &Ctx<'_>) -> String {
    let (pages, handlers, metadata) = crate::project::route_counts(ctx.analysis);
    format!(
        "# {}\n\nMigrated from Next.js {} to Leptos 0.8 (SSR + axum, static route tree) by yolo-next ({pages} pages, {handlers} route handlers, {metadata} metadata routes).\n\n## Run\n\n```sh\ncargo install cargo-leptos\ncargo leptos watch      # dev server on 127.0.0.1:3000\ncargo leptos build --release\n```\n\n## Layout\n\n- `src/app.rs`: `<Routes>` with the `/:lang` prefix on every page; static pages use `SsrMode::Static` and are prerendered from `content::static_params_for`.\n- `src/content.rs`: locale-independent paths and static params (fill the `TODO: enumerate from data` arms).\n- `src/i18n.rs` + `messages/{{{locales}}}.json`: every visible string goes through `t` / `tf`; the catalogs hold the source strings and must be translated.\n- `src/seo.rs`: `SeoHead` (title, description, canonical, hreflang, Open Graph, Twitter).\n- `src/server/redirects.rs`: 308 table from `next.config` `redirects()` plus legacy segments.\n- `src/tokens.rs`: theme preset for `yolo_tokens` (no CSS is generated).\n\nFonts, images and metadata routes (`sitemap.xml`, `robots.txt`, `manifest.webmanifest`) are listed in `MIGRATION-REPORT.md`.\n",
        ctx.package,
        ctx.analysis.next_version.as_deref().unwrap_or("16"),
        locales = ctx.locales.codes.join(",")
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
    #[allow(clippy::too_many_lines)]
    fn emits_leptos_project_from_fixture() {
        let dir = temp_project("emit-leptos");
        let analysis = analyze_project(&dir);
        let files = emit_leptos_project(&analysis);
        for expected in [
            "Cargo.toml",
            "src/lib.rs",
            "src/main.rs",
            "src/app.rs",
            "src/content.rs",
            "src/i18n.rs",
            "src/seo.rs",
            "src/tokens.rs",
            "src/server/mod.rs",
            "src/server/redirects.rs",
            "src/server_fns.rs",
            "messages/fr.json",
            "messages/en.json",
            "messages/ja.json",
            "README.md",
        ] {
            assert!(
                files.iter().any(|f| f.path == expected),
                "missing {expected}"
            );
        }
        assert_eq!(
            files.iter().any(|f| f.path == "src/proxy.rs"),
            analysis.proxy.is_some()
        );
        assert!(
            files.iter().all(|f| !std::path::Path::new(&f.path)
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("css"))),
            "no CSS in the Leptos lane"
        );

        let app = file(&files, "src/app.rs");
        assert!(app.contains("path!(\"/:lang\")"), "{app}");
        assert!(app.contains("<ParentRoute path=path!(\"/:lang/blog\") view=BlogLayout>"));
        assert!(app.contains("path!(\":slug\")"));
        assert!(app.contains("path!(\"/:lang/docs/*slug\")"));
        assert!(
            app.contains("path!(\"/:lang/pricing\")"),
            "group layouts are flattened"
        );
        assert!(app.contains("SsrMode::Static(StaticRoute::new().prerender_params(|| async move { content::static_params_for(\"/blog/[slug]\").await }))"));
        assert!(app.contains("SsrMode::OutOfOrder"));
        assert!(app.contains("ISR: Next.js revalidated every 60s"));
        assert!(app.contains("fn BlogSlug() -> impl IntoView"));
        assert!(app.contains("t(&lang.get(), \"routes.blog_slug.title\")"));
        assert!(app.contains("<SeoHead"));
        assert!(app.contains("provide_meta_context()"));
        assert!(app.contains("pub fn shell(options: LeptosOptions)"));
        assert!(!app.contains("PhotoModal"));
        let per_lang_routes = app.matches("path!(\"/:lang").count();
        assert!(
            per_lang_routes >= 8,
            "every top-level route carries :lang ({per_lang_routes})"
        );

        let content = file(&files, "src/content.rs");
        assert!(content.contains("pub async fn all_paths() -> Vec<String>"));
        assert!(content.contains("pub async fn static_params_for(route: &str) -> StaticParamsMap"));
        assert!(content.contains("// TODO: enumerate from data: `/blog/[slug]` (Next.js `generateStaticParams` in \"app/blog/[slug]/page.tsx\")"));
        assert!(content.contains("\"/blog/[slug]\" => {"));
        assert!(content.contains("SITE_ORIGIN: &str = \"https://fixture.example.com\""));

        let redirects = file(&files, "src/server/redirects.rs");
        assert!(redirects.contains("308"));
        assert!(
            redirects
                .contains("pub async fn legacy_redirects(req: Request, next: Next) -> Response")
        );
        assert!(redirects.contains("pub const LEGACY_SEGMENTS"));

        let fr: Value =
            serde_json::from_str(file(&files, "messages/fr.json")).expect("valid fr.json");
        assert!(fr["routes"].is_object());
        assert_eq!(fr["routes"]["pricing"]["title"], "Pricing | Yolo Fixture");
        assert_eq!(fr["routes"]["home"]["title"], "Yolo Fixture");
        assert_eq!(
            file(&files, "messages/en.json"),
            file(&files, "messages/fr.json")
        );

        let i18n = file(&files, "src/i18n.rs");
        assert!(i18n.contains("pub fn t(lang: &str, key: &str) -> String"));
        assert!(i18n.contains("pub fn tf(lang: &str, key: &str, args: &[(&str, &str)]) -> String"));
        assert!(i18n.contains("OnceLock"));
        assert!(i18n.contains("include_str!(\"../messages/fr.json\")"));
        assert!(i18n.contains("pub const LANGS: [&str; 3] = [\"fr\", \"en\", \"ja\"];"));

        let seo = file(&files, "src/seo.rs");
        assert!(seo.contains("pub fn SeoHead(lang: String, title: String, description: String, canonical: String) -> impl IntoView"));
        assert!(seo.contains("hreflang=\"x-default\""));
        assert!(seo.contains("<Meta name=\"twitter:card\" content=\"summary_large_image\"/>"));
        assert!(seo.contains("<Meta property=\"og:site_name\" content=\"Yolo Fixture\"/>"));
        assert!(
            !seo.contains("<Title text=\"Yolo Fixture\"/>"),
            "SeoHead owns the title"
        );

        let tokens = file(&files, "src/tokens.rs");
        assert!(tokens.contains("pub const PRESET: &str = \""));
        assert!(tokens.contains("yolo_tokens"));

        let cargo = file(&files, "Cargo.toml");
        assert!(cargo.contains("leptos = \"0.8.23\""));
        assert!(cargo.contains("[package.metadata.leptos]"));
        assert!(cargo.contains("bin-features = [\"ssr\"]"));
        assert!(cargo.contains("serde_json"));
        assert!(!cargo.contains("style-file"));

        let main = file(&files, "src/main.rs");
        assert!(main.contains("generate_route_list_with_ssg"));
        assert!(main.contains("legacy_redirects"));
        assert!(file(&files, "src/lib.rs").contains("hydrate_body"));
        let server_fns = file(&files, "src/server_fns.rs");
        assert!(server_fns.contains(
            "#[server]\npub async fn api_posts_id_get(id: String) -> Result<String, ServerFnError>"
        ));
        cleanup(&dir);
    }

    #[test]
    fn helpers() {
        assert_eq!(og_locale("fr"), "fr_FR");
        assert_eq!(og_locale("en"), "en_US");
        assert_eq!(og_locale("pt-br"), "pt_BR");
        assert_eq!(humanize("BlogSlug"), "Blog Slug");
        assert_eq!(relative_leptos("/:lang/blog/:slug", "/:lang/blog"), ":slug");
        assert_eq!(relative_leptos("/:lang/blog", "/:lang/blog"), "");
        assert_eq!(
            relative_leptos("/:lang/docs/*slug", ""),
            "/:lang/docs/*slug"
        );
    }

    #[test]
    fn real_fixture_when_present() {
        let root = Path::new("C:/src/yolo-stack/apps/next-fixture");
        if !root.exists() {
            return;
        }
        let analysis = analyze_project(root);
        let files = emit_leptos_project(&analysis);
        let app = file(&files, "src/app.rs");
        assert!(app.contains("path!(\":slug\")") || app.contains("path!(\"/:lang/blog/:slug\")"));
        assert!(app.contains("SsrMode::Static"));
        let redirects = file(&files, "src/server/redirects.rs");
        assert!(redirects.contains("/old-blog/{slug}") || redirects.contains("Skipped"));
    }
}
