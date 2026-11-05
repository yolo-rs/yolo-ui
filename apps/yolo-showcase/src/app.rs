//! The interactive showcase: a Dioxus 0.8 app with a router and one signal for the active theme.
//!
//! Routes: `/` the component matrix with a live column, `/themes` the pack picker and colour
//! controls, `/components/:name` one component across the live theme and all eight presets.
//!
//! State: [`ThemeChoice`] lives in a signal. A memo turns it into one [`AppliedTheme`], and the
//! shell renders the stylesheet, the `data-*` attributes, the window title and the wallpaper from
//! that single value, so changing the pack (or the seed, variant, contrast, mode) repaints all of
//! them in the same render. The logic itself is in [`crate::theme_state`] and tested there.
//!
//! Build for the browser with `dx serve --platform web -p yolo-showcase --features web`, or with
//! `cargo build --target wasm32-unknown-unknown --features web` plus `wasm-bindgen`.

use dioxus::prelude::*;
use yolo_theme_pack::Mode;
use yolo_tokens::scheme::Variant;

use crate::gallery::{self, Matrix};
use crate::theme_state::{AppliedTheme, ThemeChoice, catalog};

/// Favicon, bundled and hashed by the asset pipeline.
static FAVICON: Asset = asset!("/assets/yolo-mark.svg");

/// Roles shown as swatches on the themes page.
const SWATCH_ROLES: [&str; 8] = [
    "primary",
    "on-primary",
    "secondary",
    "tertiary",
    "surface",
    "on-surface",
    "error",
    "outline",
];

#[derive(Routable, Clone, PartialEq)]
#[rustfmt::skip]
pub enum Route {
    #[layout(Shell)]
        #[route("/")]
        Home {},
        #[route("/themes")]
        Themes {},
        #[route("/components/:name")]
        ComponentPage { name: String },
    #[end_layout]
    #[route("/:..segments")]
    NotFound { segments: Vec<String> },
}

impl Route {
    /// Page name used in the window title.
    fn page(&self) -> String {
        match self {
            Self::Home {} => "Components".to_owned(),
            Self::Themes {} => "Themes".to_owned(),
            Self::ComponentPage { name } => gallery::row_by_slug(name)
                .map_or_else(|| "Component".to_owned(), |row| row.name.to_owned()),
            Self::NotFound { .. } => "Not found".to_owned(),
        }
    }
}

/// The shared theme state, provided by [`App`].
#[derive(Clone, Copy)]
struct ThemeCtx {
    choice: Signal<ThemeChoice>,
    system_dark: Signal<bool>,
    applied: Memo<AppliedTheme>,
}

fn use_theme() -> ThemeCtx {
    use_context::<ThemeCtx>()
}

/// Follows the OS colour scheme so that mode `System` repaints when it changes.
async fn watch_system_scheme(mut dark: Signal<bool>) {
    let mut eval = document::eval(
        "const q = window.matchMedia('(prefers-color-scheme: dark)');\
         dioxus.send(q.matches);\
         q.addEventListener('change', (e) => dioxus.send(e.matches));",
    );
    while let Ok(value) = eval.recv::<bool>().await {
        dark.set(value);
    }
}

/// Root component.
#[component]
pub fn App() -> Element {
    let choice = use_signal(ThemeChoice::default);
    let system_dark = use_signal(|| false);
    let applied = use_memo(move || AppliedTheme::new(&choice.read(), *system_dark.read()));
    use_context_provider(|| ThemeCtx { choice, system_dark, applied });
    use_future(move || watch_system_scheme(system_dark));
    rsx! { Router::<Route> {} }
}

/// Starts the app in the browser. Routes use the URL hash, so a static file host is enough.
#[cfg(target_arch = "wasm32")]
pub fn launch() {
    use std::rc::Rc;
    dioxus::LaunchBuilder::new()
        .with_cfg(dioxus::web::Config::new().history(Rc::new(dioxus::web::HashHistory::new(false))))
        .launch(App);
}

/// Page frame: window title, favicon, stylesheets, root element attributes, navigation, the
/// routed page and the contrast report. Everything theme-dependent is read from one value.
#[component]
fn Shell() -> Element {
    let ctx = use_theme();
    let route = use_route::<Route>();
    let applied = ctx.applied.read().clone();
    let base_css = use_memo(move || gallery::gallery_css(ctx.applied.read().theme.dark));
    let title = applied.title(&route.page());
    let attr = |name: &str| applied.attribute(name).unwrap_or_default().to_owned();
    let (pack, shell, icons, anchor) = (
        attr("data-theme-pack"),
        attr("data-shell"),
        attr("data-icon-style"),
        attr("data-pet-anchor"),
    );
    let product = applied.theme.title.product_name.clone();
    let css = applied.css.clone();
    rsx! {
        document::Title { "{title}" }
        document::Link { rel: "icon", r#type: "image/svg+xml", href: FAVICON }
        div {
            class: "app",
            "data-theme-pack": "{pack}",
            "data-shell": "{shell}",
            "data-icon-style": "{icons}",
            "data-pet-anchor": "{anchor}",
            style { dangerous_inner_html: "{base_css}" }
            style { id: "yolo-theme", dangerous_inner_html: "{css}" }
            nav { class: "primary", "aria-label": "Primary",
                span { class: "brand", "{product}" }
                Link { to: Route::Home {}, "Components" }
                Link { to: Route::Themes {}, "Themes" }
            }
            div { class: "page",
                Outlet::<Route> {}
                ContrastReport {}
            }
        }
    }
}

/// AA report of the active theme. It stays visible on every page.
#[component]
fn ContrastReport() -> Element {
    let ctx = use_theme();
    let applied = ctx.applied.read().clone();
    let warnings = applied.theme.contrast_warnings.clone();
    let clamped = applied.theme.clamped_roles.clone();
    let clamped_text = clamped.join(", ");
    rsx! {
        section { class: "warnings", role: "status", "aria-live": "polite", "aria-labelledby": "aa-title",
            h2 { id: "aa-title", "Contrast (WCAG 2.2 AA, 4.5:1)" }
            if warnings.is_empty() {
                p { class: "ok", "No text pair is below AA in {applied.theme.title.product_name}." }
            } else {
                p { class: "ok", "{warnings.len()} text pairs are below AA." }
                ul {
                    for (on, base, ratio) in warnings {
                        li { key: "{on}", "{on} on {base}: {ratio}:1" }
                    }
                }
            }
            if !clamped.is_empty() {
                p { class: "ok", "Adjusted by the text clamp to reach AA: {clamped_text}." }
            }
        }
    }
}

#[component]
fn Home() -> Element {
    let ctx = use_theme();
    let applied = ctx.applied.read().clone();
    let link = Callback::new(|name: String| {
        rsx! {
            Link { to: Route::ComponentPage { name: gallery::slug(&name) }, "{name}" }
        }
    });
    rsx! {
        header { class: "panel",
            h1 { "Yolo UI components" }
            p { "The first column is painted by the active theme and follows every change on the Themes page. The other eight are the built-in presets." }
        }
        Matrix { dark: applied.theme.dark, active: applied.summary(), link }
    }
}

#[component]
fn ComponentPage(name: String) -> Element {
    let ctx = use_theme();
    let applied = ctx.applied.read().clone();
    let Some(row) = gallery::row_by_slug(&name) else {
        return rsx! { NotFound { segments: vec!["components".to_owned(), name] } };
    };
    rsx! {
        header { class: "panel",
            h1 { "{row.name}" }
            p { "{row.note}" }
            Link { to: Route::Home {}, "All components" }
        }
        Matrix { dark: applied.theme.dark, active: applied.summary(), only: name }
    }
}

#[component]
fn NotFound(segments: Vec<String>) -> Element {
    let path = segments.join("/");
    rsx! {
        header { class: "panel",
            h1 { "Page not found" }
            p { "There is no page at /{path}." }
            Link { to: Route::Home {}, "Back to the components" }
        }
    }
}

#[component]
fn Themes() -> Element {
    let ctx = use_theme();
    let applied = ctx.applied.read().clone();
    let choice = ctx.choice.read().clone();
    let pack = choice.pack();
    let theme = &applied.theme;
    let mode = choice.mode.unwrap_or(pack.color.mode);
    let swatches: Vec<(&str, String)> = SWATCH_ROLES
        .iter()
        .filter_map(|role| theme.role(role).map(|value| (*role, value.to_owned())))
        .collect();
    let contrast_text = format!("{:+.2}", theme.contrast);
    let seed = theme.seed.clone();
    let variant = theme.variant.clone();
    let contrast = theme.contrast;
    let clamp = choice.clamp_text;
    let modes = [(Mode::Light, "Light"), (Mode::Dark, "Dark"), (Mode::System, "System")];
    rsx! {
        header { class: "panel",
            h1 { "Themes" }
            p { "One choice changes the colours, wallpaper, shell layout, shape, title and attributes together." }
        }
        form { class: "panel controls", onsubmit: move |event| event.prevent_default(),
            fieldset {
                legend { "Theme pack" }
                for entry in catalog() {
                    label { class: "opt", key: "{entry.id}",
                        input {
                            r#type: "radio",
                            name: "pack",
                            value: "{entry.id}",
                            checked: entry.id == choice.pack_id,
                            onchange: move |event| ctx.choice.write().select_pack(&event.value()),
                        }
                        "{entry.name}"
                    }
                }
            }
            div { class: "control",
                label { class: "caption", r#for: "seed", "Seed colour" }
                div { class: "row",
                    input {
                        id: "seed",
                        r#type: "color",
                        value: "{seed}",
                        oninput: move |event| ctx.choice.write().seed = Some(event.value()),
                    }
                    span { class: "value", "{seed}" }
                    button {
                        class: "reset",
                        r#type: "button",
                        onclick: move |_| ctx.choice.write().seed = None,
                        "Use pack seed"
                    }
                }
                div { class: "row",
                    label { class: "caption", r#for: "variant", "Variant" }
                    select {
                        id: "variant",
                        onchange: move |event| ctx.choice.write().variant = Some(event.value()),
                        for v in Variant::ALL {
                            option { value: "{v.name()}", selected: v.name() == variant, "{v.name()}" }
                        }
                    }
                }
                div { class: "row",
                    label { class: "caption", r#for: "contrast", "Contrast" }
                    input {
                        id: "contrast",
                        r#type: "range",
                        min: "-1",
                        max: "1",
                        step: "0.25",
                        value: "{contrast}",
                        "aria-valuetext": "{contrast_text}",
                        oninput: move |event| {
                            if let Ok(level) = event.value().parse::<f64>() {
                                ctx.choice.write().contrast = Some(level.clamp(-1.0, 1.0));
                            }
                        },
                    }
                    span { class: "value", "{contrast_text}" }
                }
            }
            fieldset {
                legend { "Mode" }
                for (value, label) in modes {
                    label { class: "opt", key: "{label}",
                        input {
                            r#type: "radio",
                            name: "mode",
                            value: "{label}",
                            checked: mode == value,
                            onchange: move |_| ctx.choice.write().mode = Some(value),
                        }
                        "{label}"
                    }
                }
                label { class: "opt",
                    input {
                        r#type: "checkbox",
                        checked: clamp,
                        onchange: move |event| ctx.choice.write().clamp_text = event.checked(),
                    }
                    "Raise text that falls below AA"
                }
                p { class: "value", "System follows the operating system, now {system_label(ctx)}." }
            }
        }
        section { class: "panel", "aria-labelledby": "preview-title",
            h2 { id: "preview-title", "Active theme" }
            div {
                class: "wallpaper-chip",
                role: "img",
                "aria-label": "Generated wallpaper for {theme.title.product_name}",
            }
            ul { class: "swatches",
                for (role, value) in swatches {
                    li { key: "{role}",
                        span { class: "chip-color", style: "background:{value}" }
                        "{role} {value}"
                    }
                }
            }
        }
    }
}

fn system_label(ctx: ThemeCtx) -> &'static str {
    if *ctx.system_dark.read() { "dark" } else { "light" }
}
