#![cfg(feature = "ssr")]
use std::collections::BTreeSet;

use leptos::prelude::*;
use leptos_router::{RouteList, location::RequestUrl};
use shenron_site::{
    app::shell,
    content::{MANGA_VOLUMES, all_paths},
    i18n::Lang,
};

async fn prerendered_paths() -> Vec<String> {
    let owner = Owner::new_root(None);
    owner.set();
    provide_context(RequestUrl::new(""));
    let options = LeptosOptions::builder().output_name("shenron-site").build();
    let routes = RouteList::generate(move || shell(options.clone())).expect("route list");
    routes
        .into_static_paths()
        .await
        .into_iter()
        .map(|p| p.as_ref().to_owned())
        .collect()
}

#[tokio::test]
async fn prerender_lists_exactly_the_real_pages_once_each() {
    let paths = prerendered_paths().await;
    let unique: BTreeSet<&String> = paths.iter().collect();
    assert_eq!(unique.len(), paths.len(), "duplicate prerender paths");

    let expected: BTreeSet<String> = all_paths()
        .iter()
        .flat_map(|p| Lang::ALL.iter().map(move |l| format!("/{}{p}", l.code())))
        .collect();
    let got: BTreeSet<String> = paths.iter().cloned().collect();
    assert_eq!(got, expected);
}

#[tokio::test]
async fn prerendered_manga_pairs_are_the_real_pairs() {
    let paths = prerendered_paths().await;
    let real: usize = MANGA_VOLUMES.iter().map(|(_, n)| *n as usize).sum();
    let fr_manga: Vec<&String> = paths
        .iter()
        .filter(|p| p.starts_with("/fr/manga/"))
        .collect();
    assert_eq!(fr_manga.len(), real);
    assert!(!paths.iter().any(|p| p == "/fr/manga/1/12"));
    assert!(paths.iter().any(|p| p == "/fr/manga/2/12"));
}
