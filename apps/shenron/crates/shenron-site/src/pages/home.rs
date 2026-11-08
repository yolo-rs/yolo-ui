use leptos::prelude::*;

use crate::{
    i18n::{Lang, t},
    pages::with_lang,
    seo::SeoHead,
};

#[component]
pub fn HomePage() -> impl IntoView {
    with_lang(|lang: Lang| {
        let code = lang.code();
        view! {
            <SeoHead
                lang=lang
                path=""
                title=t(lang, "home.title")
                description=t(lang, "site.description")
            />
            <h1>{t(lang, "site.name")}</h1>
            <p>{t(lang, "home.tagline")}</p>
            <h2>{t(lang, "home.sections_heading")}</h2>
            <ul class="card-grid">
                <li class="card">
                    <a href=format!("/{code}/characters")>{t(lang, "nav.characters")}</a>
                </li>
                <li class="card">
                    <a href=format!("/{code}/manga/1/1")>{t(lang, "nav.manga")}</a>
                </li>
                <li class="card">
                    <a href=format!("/{code}/databooks/daizenshuu-7")>{t(lang, "nav.databooks")}</a>
                </li>
            </ul>
        }
    })
}
