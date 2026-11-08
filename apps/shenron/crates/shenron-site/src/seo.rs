//! SEO head tags: title, description, canonical, hreflang alternates, OpenGraph.

use leptos::prelude::*;
use leptos_meta::{Link, Meta, Title};

use crate::{
    content::{OG_IMAGE_PATH, SITE_ORIGIN, url},
    i18n::{Lang, t},
};

/// Emits the `<head>` tags for one page.
///
/// `path` is the locale-independent path (`""`, `/wiki/son-goku`, ...): the
/// alternates are derived from it for every locale plus `x-default`.
#[component]
pub fn SeoHead(
    lang: Lang,
    #[prop(into)] path: String,
    #[prop(into)] title: String,
    #[prop(into)] description: String,
    #[prop(default = "website")] og_type: &'static str,
) -> impl IntoView {
    let canonical = url(lang, &path);
    let image = format!("{SITE_ORIGIN}{OG_IMAGE_PATH}");
    let alternates = Lang::ALL
        .iter()
        .map(|l| view! { <Link rel="alternate" hreflang=l.code() href=url(*l, &path) /> })
        .collect_view();
    let og_alternates = Lang::ALL
        .iter()
        .filter(|l| **l != lang)
        .map(|l| view! { <Meta property="og:locale:alternate" content=l.og_locale() /> })
        .collect_view();

    view! {
        <Title text=title.clone() />
        <Meta name="description" content=description.clone() />
        <Link rel="canonical" href=canonical.clone() />
        {alternates}
        <Link rel="alternate" hreflang="x-default" href=url(Lang::DEFAULT, &path) />
        <Meta property="og:type" content=og_type />
        <Meta property="og:site_name" content=t(lang, "site.name") />
        <Meta property="og:title" content=title />
        <Meta property="og:description" content=description />
        <Meta property="og:url" content=canonical />
        <Meta property="og:image" content=image />
        <Meta property="og:locale" content=lang.og_locale() />
        {og_alternates}
        <Meta name="twitter:card" content="summary_large_image" />
    }
}
