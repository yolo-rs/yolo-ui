use leptos::prelude::*;
use leptos_router::hooks::use_params_map;

use crate::{
    content::humanize,
    i18n::{Lang, tf},
    pages::with_lang,
    seo::SeoHead,
};

#[component]
pub fn WikiPage() -> impl IntoView {
    let params = use_params_map();
    with_lang(move |lang: Lang| {
        let slug = params.read().get("slug").unwrap_or_default();
        let name = humanize(&slug);
        let args = [("name", name.as_str())];
        view! {
            <SeoHead
                lang=lang
                path=format!("/wiki/{slug}")
                title=tf(lang, "wiki.title", &args)
                description=tf(lang, "wiki.intro", &args)
                og_type="article"
            />
            <article>
                <h1>{tf(lang, "wiki.heading", &args)}</h1>
                <p>{tf(lang, "wiki.intro", &args)}</p>
            </article>
        }
    })
}
