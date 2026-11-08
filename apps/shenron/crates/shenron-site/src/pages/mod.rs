//! Route views. Each page reads `:lang` (and its own params) from the router.

pub mod characters;
pub mod databooks;
pub mod home;
pub mod manga;
pub mod not_found;
pub mod theme;
pub mod wiki;

pub use characters::CharactersPage;
pub use databooks::DatabookPage;
pub use home::HomePage;
pub use manga::MangaPage;
pub use not_found::NotFound;
pub use theme::ThemePage;
pub use wiki::WikiPage;

use leptos::{either::Either, prelude::*};
use leptos_router::hooks::use_params_map;

use crate::i18n::Lang;

/// Current `:lang` param, if it names a supported locale.
pub fn use_lang() -> Memo<Option<Lang>> {
    let params = use_params_map();
    Memo::new(move |_| params.read().get("lang").as_deref().and_then(Lang::parse))
}

/// Renders `page(lang)` or the not-found view when `:lang` is not supported.
pub fn with_lang<V: IntoView + 'static>(
    page: impl Fn(Lang) -> V + Send + Sync + 'static,
) -> impl IntoView {
    let lang = use_lang();
    move || match lang.get() {
        Some(lang) => Either::Left(page(lang)),
        None => Either::Right(view! { <NotFound /> }),
    }
}
