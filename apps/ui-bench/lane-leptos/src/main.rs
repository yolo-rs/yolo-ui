//! Lane 2: Leptos 0.8 static SSR on Axum (ADR 0003). Every page is rendered once at startup
//! with `RenderHtml::to_html` and served as a fixed string; the only other responses are the
//! two Tailwind-compiled stylesheets built from the kits' CSS (`bun scripts/build-leptos-css.ts`).
mod content;
mod rosegriffon;
mod shenron;

use std::{collections::HashMap, sync::Arc};

use axum::{
    Router,
    extract::State,
    http::{StatusCode, Uri, header},
    response::{Html, IntoResponse, Response},
    routing::get,
};
use leptos::prelude::*;
use tower_http::services::ServeDir;

fn document(
    title: &'static str,
    html_class: &'static str,
    css: &'static str,
    body: AnyView,
) -> String {
    let owner = Owner::new();
    let html = owner.with(|| {
        view! {
            <html lang="fr" class=html_class>
                <head>
                    <meta charset="utf-8" />
                    <meta name="viewport" content="width=device-width, initial-scale=1" />
                    <title>{title}</title>
                    <link rel="stylesheet" href=css />
                </head>
                <body>{body}</body>
            </html>
        }
        .to_html()
    });
    format!("<!DOCTYPE html>{html}")
}

type Pages = Arc<HashMap<&'static str, String>>;

async fn page(State(pages): State<Pages>, uri: Uri) -> Response {
    match pages.get(uri.path()) {
        Some(html) => ([(header::CACHE_CONTROL, "no-store")], Html(html.clone())).into_response(),
        None => (StatusCode::NOT_FOUND, "Not found").into_response(),
    }
}

#[tokio::main]
async fn main() {
    let site: &'static content::Site = Box::leak(Box::new(content::Site::load()));
    let rg = "/assets/rosegriffon.css";
    let sh = "/assets/shenron.css";
    let mut pages = HashMap::new();
    pages.insert(
        "/rosegriffon/",
        document(
            "Rose Griffon UI - Accueil",
            "theme-roy",
            rg,
            rosegriffon::home(site),
        ),
    );
    pages.insert(
        "/rosegriffon/gallery",
        document(
            "Rose Griffon UI - Galerie",
            "theme-roy",
            rg,
            rosegriffon::gallery(site),
        ),
    );
    pages.insert(
        "/rosegriffon/legal",
        document(
            "Rose Griffon UI - Mentions légales",
            "theme-roy",
            rg,
            rosegriffon::legal(site),
        ),
    );
    pages.insert(
        "/shenron/",
        document("Shenron UI - Accueil", "dark", sh, shenron::home(site)),
    );
    pages.insert(
        "/shenron/gallery",
        document("Shenron UI - Galerie", "dark", sh, shenron::gallery(site)),
    );
    pages.insert(
        "/shenron/legal",
        document(
            "Shenron UI - Mentions légales",
            "dark",
            sh,
            shenron::legal(site),
        ),
    );
    for (path, html) in &pages {
        println!("{path}: {} bytes", html.len());
    }
    let pages: Pages = Arc::new(pages);

    let css_dir = std::env::var("CSS_DIR").unwrap_or_else(|_| "dist/leptos-css".into());
    let mut app = Router::new().route("/health", get(|| async { "ok" }));
    for path in pages.keys() {
        app = app.route(path, get(page));
    }
    let app = app
        .nest_service("/assets", ServeDir::new(css_dir))
        .with_state(pages);

    let host = std::env::var("HOST").unwrap_or_else(|_| "127.0.0.1".into());
    let port = std::env::var("PORT").unwrap_or_else(|_| "4200".into());
    let listener = tokio::net::TcpListener::bind(format!("{host}:{port}"))
        .await
        .expect("bind");
    println!("leptos lane listening on http://{host}:{port}");
    axum::serve(listener, app).await.expect("serve");
}
