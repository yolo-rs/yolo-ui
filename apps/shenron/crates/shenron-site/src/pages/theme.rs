use leptos::prelude::*;

use crate::{
    i18n::{Lang, t},
    pages::with_lang,
    seo::SeoHead,
    theme_ui::ThemePanel,
};

/// Theme settings: bundle choice, computed colours, wallpaper, pet and overrides.
#[component]
pub fn ThemePage() -> impl IntoView {
    with_lang(|lang: Lang| {
        view! {
            <SeoHead
                lang=lang
                path="/theme"
                title=t(lang, "theme.title")
                description=t(lang, "theme.intro")
            />
            <h1>{t(lang, "theme.title")}</h1>
            <p>{t(lang, "theme.intro")}</p>
            <ThemePanel lang=lang.code().to_owned() />
        }
    })
}
