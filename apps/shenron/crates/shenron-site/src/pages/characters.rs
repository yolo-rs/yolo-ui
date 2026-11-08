use leptos::prelude::*;

use crate::{
    content::{character_slugs, humanize},
    i18n::{Lang, t},
    pages::with_lang,
    seo::SeoHead,
};

#[component]
pub fn CharactersPage() -> impl IntoView {
    with_lang(|lang: Lang| {
        let code = lang.code();
        let items = character_slugs()
            .map(|slug| {
                view! {
                    <li class="card">
                        <a href=format!("/{code}/wiki/{slug}")>{humanize(slug)}</a>
                    </li>
                }
            })
            .collect_view();
        view! {
            <SeoHead
                lang=lang
                path="/characters"
                title=t(lang, "characters.title")
                description=t(lang, "characters.intro")
            />
            <h1>{t(lang, "characters.heading")}</h1>
            <p>{t(lang, "characters.intro")}</p>
            <ul class="card-grid">{items}</ul>
        }
    })
}
