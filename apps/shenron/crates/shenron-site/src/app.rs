//! Application shell, layout and static route tree.

use leptos::{either::Either, prelude::*};
use leptos_meta::{Html, MetaTags, Stylesheet, provide_meta_context};
use leptos_router::{
    SsrMode,
    components::{Route, Router, Routes},
    hooks::use_location,
    path,
    static_routes::{StaticParamsMap, StaticRoute},
};

use crate::{
    boot::BOOT_JS,
    content::{databook_slugs, langs, manga_vc_values, wiki_slugs},
    i18n::{Lang, t},
    pages::{CharactersPage, DatabookPage, HomePage, MangaPage, NotFound, ThemePage, WikiPage},
    theme_ui::{ThemeBrand, ThemePet, ThemeRuntime, ThemeSwitcher},
    themes::DEFAULT_ID,
};

/// HTML document shell rendered by the server for every route.
pub fn shell(options: LeptosOptions) -> impl IntoView {
    // Read by `Layout` to emit the hydration scripts on pages that contain islands.
    provide_context(options.clone());
    view! {
        <!DOCTYPE html>
        <html data-theme=DEFAULT_ID>
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1" />
                // Sets data-theme and the saved custom stylesheet before the first paint.
                <script inner_html=BOOT_JS></script>
                <AutoReload options=options.clone() />
                <MetaTags />
            </head>
            <body>
                <App />
            </body>
        </html>
    }
}

/// Static params for the locale-only routes (`/{lang}`, `/{lang}/characters`).
fn lang_params() -> StaticRoute {
    StaticRoute::new().prerender_params(|| async move {
        let mut map = StaticParamsMap::new();
        map.insert("lang", langs());
        map
    })
}

/// `/{lang}/wiki/{slug}`
fn wiki_params() -> StaticRoute {
    StaticRoute::new().prerender_params(|| async move {
        let mut map = StaticParamsMap::new();
        map.insert("lang", langs());
        map.insert("slug", wiki_slugs());
        map
    })
}

/// `/{lang}/databooks/{slug}`
fn databook_params() -> StaticRoute {
    StaticRoute::new().prerender_params(|| async move {
        let mut map = StaticParamsMap::new();
        map.insert("lang", langs());
        map.insert("slug", databook_slugs());
        map
    })
}
/// `/{lang}/manga/{volume}/{chapter}`: one splat value `"{volume}/{chapter}"` per real pair.
/// Two separate lists (volumes, chapters) would make the router build their cartesian product.
fn manga_params() -> StaticRoute {
    StaticRoute::new().prerender_params(|| async move {
        let mut map = StaticParamsMap::new();
        map.insert("lang", langs());
        map.insert("vc", manga_vc_values());
        map
    })
}

/// True for the pages that contain islands (reader, theme settings). Every other page is plain
/// server-rendered HTML and ships no wasm and no hydration script.
pub fn ships_wasm(path: &str) -> bool {
    let mut parts = path.trim_start_matches('/').split('/').skip(1);
    matches!(parts.next(), Some("manga" | "theme"))
}

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    view! {
        <Stylesheet id="leptos" href="/pkg/shenron-site.css" />
        <Router>
            <Layout>
                <Routes fallback=|| view! { <NotFound /> }>
                    <Route
                        path=path!("/:lang")
                        view=HomePage
                        ssr=SsrMode::Static(lang_params())
                    />
                    <Route
                        path=path!("/:lang/theme")
                        view=ThemePage
                        ssr=SsrMode::Static(lang_params())
                    />
                    <Route
                        path=path!("/:lang/characters")
                        view=CharactersPage
                        ssr=SsrMode::Static(lang_params())
                    />
                    <Route
                        path=path!("/:lang/wiki/:slug")
                        view=WikiPage
                        ssr=SsrMode::Static(wiki_params())
                    />
                    <Route
                        path=path!("/:lang/manga/*vc")
                        view=MangaPage
                        ssr=SsrMode::Static(manga_params())
                    />
                    <Route
                        path=path!("/:lang/databooks/:slug")
                        view=DatabookPage
                        ssr=SsrMode::Static(databook_params())
                    />
                </Routes>
            </Layout>
        </Router>
    }
}

/// Header (nav + locale switch), main and footer. The locale comes from the
/// URL prefix so the layout can live outside the `Routes` tree.
#[component]
fn Layout(children: Children) -> impl IntoView {
    let location = use_location();
    let pathname = location.pathname;
    // Islands (theme runtime, brand, switcher, pet) only exist on pages that ship wasm.
    let interactive = ships_wasm(&pathname.get_untracked());
    let lang = Memo::new(move |_| Lang::from_path(&pathname.get()).unwrap_or(Lang::DEFAULT));
    // Same page in another locale: swap the first segment.
    let switch_href = move |target: Lang| {
        let path = pathname.get();
        let rest = match Lang::from_path(&path) {
            Some(l) => path
                .trim_start_matches('/')
                .strip_prefix(l.code())
                .unwrap_or(""),
            None => "",
        };
        format!("/{}{rest}", target.code())
    };

    view! {
        <Html attr:lang=move || lang.get().code() />
        <div class="theme-wallpaper" aria-hidden="true"></div>
        {interactive
            .then(|| {
                let options = expect_context::<LeptosOptions>();
                view! { <HydrationScripts options islands=true /> }
            })}
        {interactive.then(|| view! { <ThemeRuntime /> })}
        {interactive.then(|| view! { <ThemePet /> })}
        <header class="site-header">
            <nav aria-label=move || t(lang.get(), "nav.home")>
                {if interactive {
                    Either::Left(view! { <ThemeBrand lang=lang.get_untracked().code().to_owned() /> })
                } else {
                    Either::Right(static_brand(lang.get_untracked()))
                }}
                <a href=move || format!("/{}/characters", lang.get().code())>
                    {move || t(lang.get(), "nav.characters")}
                </a>
                <a href=move || format!("/{}/manga/1/1", lang.get().code())>
                    {move || t(lang.get(), "nav.manga")}
                </a>
                <a href=move || format!("/{}/databooks/daizenshuu-7", lang.get().code())>
                    {move || t(lang.get(), "nav.databooks")}
                </a>
                <a href=move || format!("/{}/theme", lang.get().code())>
                    {move || t(lang.get(), "nav.theme")}
                </a>
                {interactive
                    .then(|| view! { <ThemeSwitcher lang=lang.get_untracked().code().to_owned() /> })}
                <div
                    class="lang-switch"
                    role="group"
                    aria-label=move || t(lang.get(), "lang.switch_label")
                >
                    {Lang::ALL
                        .iter()
                        .map(|target| {
                            let target = *target;
                            view! {
                                <a
                                    href=move || switch_href(target)
                                    hreflang=target.code()
                                    lang=target.code()
                                    aria-current=move || (lang.get() == target).then_some("true")
                                >
                                    {move || t(lang.get(), &target.name_key())}
                                </a>
                            }
                        })
                        .collect_view()}
                </div>
            </nav>
        </header>
        <main>{children()}</main>
        <footer class="site-footer">
            <p class="muted">{move || t(lang.get(), "footer.tagline")}</p>
        </footer>
    }
}

/// Brand link of the default theme for pages without islands (no wasm, no theme switching).
fn static_brand(lang: Lang) -> impl IntoView {
    let title = crate::themes::resolved(&Default::default())
        .title
        .product_name
        .clone();
    view! {
        <a class="brand" href=format!("/{}", lang.code())>
            <span class="brand-logo" aria-hidden="true"></span>
            <span>{title}</span>
        </a>
    }
}
