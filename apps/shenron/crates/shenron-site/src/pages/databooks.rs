use leptos::prelude::*;
use leptos_router::hooks::use_params_map;

use crate::{
    content::humanize,
    i18n::{Lang, tf},
    pages::with_lang,
    seo::SeoHead,
};

#[component]
pub fn DatabookPage() -> impl IntoView {
    let params = use_params_map();
    with_lang(move |lang: Lang| {
        let slug = params.read().get("slug").unwrap_or_default();
        let name = humanize(&slug);
        let args = [("name", name.as_str())];
        view! {
            <SeoHead
                lang=lang
                path=format!("/databooks/{slug}")
                title=tf(lang, "databooks.title", &args)
                description=tf(lang, "databooks.intro", &args)
                og_type="book"
            />
            <article>
                <h1>{tf(lang, "databooks.heading", &args)}</h1>
                <p>{tf(lang, "databooks.intro", &args)}</p>
            </article>
        }
    })
}
