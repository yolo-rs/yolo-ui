//! Axum server binary (feature `ssr`).

#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use axum::{Router, routing::get};
    use leptos::prelude::*;
    use leptos_axum::{LeptosRoutes, generate_route_list_with_ssg};
    use shenron_site::{
        app::shell,
        server::{legacy_redirect, robots_txt, root_redirect, sitemap_xml},
    };

    let conf = get_configuration(None).expect("Cargo.toml [package.metadata.leptos]");
    let addr = conf.leptos_options.site_addr;
    let leptos_options = conf.leptos_options;

    // Walks the <Routes> tree once, then prerenders every SsrMode::Static route
    // (from `prerender_params`) into `site-root`.
    // The app function must be the whole document (shell), or leptos_meta finds no <head>.
    let (routes, static_routes) = generate_route_list_with_ssg({
        let leptos_options = leptos_options.clone();
        move || shell(leptos_options.clone())
    });
    static_routes.generate(&leptos_options).await;

    let app = Router::new()
        .route("/", get(root_redirect))
        .route("/sitemap.xml", get(sitemap_xml))
        .route("/robots.txt", get(robots_txt))
        .leptos_routes(&leptos_options, routes, {
            let leptos_options = leptos_options.clone();
            move || shell(leptos_options.clone())
        })
        .fallback(leptos_axum::file_and_error_handler(shell))
        .layer(axum::middleware::from_fn(legacy_redirect))
        .with_state(leptos_options);

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("bind site-addr");
    println!("listening on http://{addr}");
    axum::serve(listener, app.into_make_service())
        .await
        .expect("axum serve");
}

#[cfg(not(feature = "ssr"))]
fn main() {}
