use leptos::prelude::*;
use leptos_router::hooks::use_location;

use crate::{
    i18n::{Lang, t},
    seo::SeoHead,
};

/// 404 view. The locale is inferred from the URL prefix, defaulting to `fr`.
#[component]
pub fn NotFound() -> impl IntoView {
    #[cfg(feature = "ssr")]
    if let Some(resp) = use_context::<leptos_axum::ResponseOptions>() {
        resp.set_status(axum::http::StatusCode::NOT_FOUND);
    }
    let location = use_location();
    let lang = Lang::from_path(&location.pathname.get_untracked()).unwrap_or(Lang::DEFAULT);
    view! {
        <SeoHead
            lang=lang
            path="/404"
            title=t(lang, "notfound.title")
            description=t(lang, "notfound.body")
        />
        <h1>{t(lang, "notfound.heading")}</h1>
        <p>{t(lang, "notfound.body")}</p>
        <p>
            <a href=format!("/{}", lang.code())>{t(lang, "notfound.back")}</a>
        </p>
    }
}
