#![cfg(feature = "ssr")]
//! Text pages must ship no wasm and no hydration script; reader and theme pages must.
use futures::StreamExt;
use leptos::prelude::*;
use leptos::tachys::view::RenderHtml;
use leptos_router::location::RequestUrl;

async fn html(path: &str) -> String {
    let owner = Owner::new();
    owner.set();
    provide_context(RequestUrl::new(path));
    let options = LeptosOptions::builder().output_name("shenron-site").build();
    let stream = shenron_site::app::shell(options).to_html_stream_in_order();
    Box::pin(stream).collect::<Vec<_>>().await.concat()
}

#[tokio::test]
async fn text_pages_reference_no_wasm() {
    for path in [
        "/fr",
        "/en/characters",
        "/ja/wiki/bulma",
        "/fr/databooks/daizenshuu-7",
    ] {
        let html = html(path).await;
        assert!(!html.contains(".wasm"), "{path} references wasm");
        assert!(!html.contains("modulepreload"), "{path} preloads a module");
        assert!(!html.contains("shenron-site.js"), "{path} loads the bundle");
        assert!(!html.contains("<canvas"), "{path}");
    }
}

#[tokio::test]
async fn reader_and_theme_pages_reference_the_wasm_bundle() {
    for path in ["/fr/manga/1/3", "/en/theme"] {
        let html = html(path).await;
        assert!(html.contains("shenron-site_bg.wasm"), "{path} lacks wasm");
    }
    assert!(html("/fr/manga/1/3").await.contains(r#"<canvas"#));
}
