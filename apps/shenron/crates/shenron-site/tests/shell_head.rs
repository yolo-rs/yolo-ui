#![cfg(feature = "ssr")]
use futures::StreamExt;
use leptos::prelude::*;
use leptos::tachys::view::RenderHtml;

#[tokio::test]
async fn first_stream_chunk_contains_the_closing_head() {
    let owner = Owner::new();
    owner.set();
    provide_context(leptos_router::location::RequestUrl::new("/fr"));
    let options = LeptosOptions::builder().output_name("shenron-site").build();
    let mut stream = Box::pin(shenron_site::app::shell(options).to_html_stream_in_order());
    let first = stream.next().await.unwrap_or_default();
    println!(
        "FIRST CHUNK ({} bytes) TAIL: {}",
        first.len(),
        &first[first.len().saturating_sub(300)..]
    );
    assert!(first.contains("</head>"));
}
