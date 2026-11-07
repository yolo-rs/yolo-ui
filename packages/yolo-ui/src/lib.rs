#![doc = include_str!("../README.md")]

pub use dioxus;
pub use dioxus::*;
pub use yolo_shared as shared;

#[cfg(feature = "tokens")]
pub use yolo_tokens as tokens;

#[cfg(feature = "contracts")]
pub use yolo_contracts as contracts;

#[cfg(feature = "emit")]
pub use yolo_emit as emit;

#[cfg(feature = "layout")]
pub use yolo_layout as layout;

#[cfg(feature = "migrator")]
pub use yolo_migrator as migrator;

#[cfg(feature = "shell-lifecycle")]
pub use yolo_shell as shell_lifecycle;

#[cfg(feature = "craft")]
pub use yolo_craft as craft;

#[cfg(feature = "storyteller")]
pub use yolo_storyteller as storyteller;

#[cfg(feature = "react")]
pub use yolo_react as react;

#[cfg(feature = "gpui")]
pub use yolo_engines::gpui;

#[cfg(feature = "iced")]
pub use yolo_engines::iced;

#[cfg(feature = "tauri")]
pub use yolo_engines::tauri;

#[cfg(feature = "egui")]
pub use yolo_engines::egui;

#[cfg(feature = "cosmic")]
pub use yolo_engines::cosmic;

#[cfg(feature = "gtk")]
pub use yolo_engines::gtk;

#[cfg(feature = "qt")]
pub use yolo_engines::qt;

#[cfg(any(
    feature = "gpui",
    feature = "iced",
    feature = "tauri",
    feature = "egui",
    feature = "cosmic",
    feature = "gtk",
    feature = "qt"
))]
pub use yolo_engines as engines;

#[cfg(feature = "three3d")]
pub use yolo_3d as three3d;

pub use dioxus_core as core;
pub use dioxus_document as document;
pub use dioxus_history as history;
pub use dioxus_hooks as hooks;
pub use dioxus_html as html;
pub use dioxus_signals as signals;
pub use dioxus_stores as stores;
pub use subsecond;
pub use warnings;

#[cfg(feature = "router")]
pub use dioxus_router as router;

#[cfg(feature = "fullstack")]
pub use dioxus_fullstack as fullstack;

#[cfg(feature = "server")]
pub use dioxus_server as server;

#[cfg(feature = "web")]
pub use dioxus_web as web;

#[cfg(any(feature = "desktop", feature = "mobile"))]
pub use dioxus_desktop as desktop;

#[cfg(feature = "native")]
pub use dioxus_native as native;

#[cfg(feature = "native")]
pub use dioxus_native_dom as native_dom;

#[cfg(feature = "liveview")]
pub use dioxus_liveview as liveview;

#[cfg(feature = "ssr")]
pub use dioxus_ssr as ssr;

#[cfg(feature = "primitives")]
pub use dioxus_primitives as primitives;

#[cfg(feature = "primitives")]
pub use dioxus_attributes as attributes;

#[cfg(feature = "sdk")]
pub use dioxus_sdk as sdk;

#[cfg(feature = "icons")]
pub use dioxus_icons as icons;

#[cfg(feature = "code")]
pub use dioxus_code as code;

#[cfg(feature = "html-macro")]
pub use dioxus_html_macro as html_macro;

#[cfg(feature = "rpc")]
pub use rsrpc as rpc;

#[cfg(feature = "native")]
pub mod engine {
    pub use anyrender;
    pub use blitz_dom;
    pub use blitz_html;
    pub use blitz_paint;
    pub use blitz_traits;
    pub use taffy;
}

/// Yolo UI Neutral & Multi-Preset Design System Tokens (`yolo-neutral`, `yolo-m3`,
/// `yolo-fluent`, `yolo-apple`, `yolo-omarchy`, `yolo-hyprland`, `yolo-aphrody`, `yolo-chromium`).
/// Re-exported from the shared foundation crate [`yolo_shared`].
pub mod theme {
    pub use yolo_shared::{
        APHRODY_DARK_THEME, AphrodyThemeTokens, BYRON_LOVE_PALETTE, BYRON_LOVE_PET_SPEC,
        ByronLovePetSpec, HYPRLAND_DECORATION_SPEC, HyprlandDecorationSpec, OMARCHY_THEMES,
        OmarchySubTheme, OmarchyThemePalette, RgbaColor, YOLO_APHRODY_DARK_THEME,
        YOLO_APHRODY_LIGHT_THEME, YOLO_APPLE_DARK_THEME, YOLO_APPLE_LIGHT_THEME,
        YOLO_CHROMIUM_DARK_THEME, YOLO_CHROMIUM_LIGHT_THEME, YOLO_DEFAULT_THEME,
        YOLO_FLUENT_DARK_THEME, YOLO_FLUENT_LIGHT_THEME, YOLO_HYPRLAND_DARK_THEME,
        YOLO_HYPRLAND_LIGHT_THEME, YOLO_M3_DARK_THEME, YOLO_M3_LIGHT_THEME,
        YOLO_NEUTRAL_DARK_THEME, YOLO_NEUTRAL_LIGHT_THEME, YOLO_OMARCHY_DARK_THEME,
        YOLO_OMARCHY_LIGHT_THEME, YoloManifest, YoloStyle, YoloThemePreset, YoloThemeTokens,
        css_preset_variables, css_variables,
    };
}

/// SEO & Head Metadata helpers for Aphrody Labs fullstack sites
pub mod seo {
    use dioxus::prelude::*;

    #[derive(Props, Clone, PartialEq)]
    pub struct SiteMetaProps {
        pub title: String,
        #[props(default = String::from("Built with Aphrody Labs Yolo UI Fullstack Framework"))]
        pub description: String,
        #[props(optional)]
        pub canonical_url: Option<String>,
        #[props(optional)]
        pub og_image: Option<String>,
    }

    #[component]
    pub fn SiteMeta(props: SiteMetaProps) -> Element {
        rsx! {
            document::Title { "{props.title}" }
            document::Meta { name: "description", content: "{props.description}" }
            document::Meta { property: "og:title", content: "{props.title}" }
            document::Meta { property: "og:description", content: "{props.description}" }
            document::Meta { property: "og:type", content: "website" }
            if let Some(url) = &props.canonical_url {
                document::Link { rel: "canonical", href: "{url}" }
                document::Meta { property: "og:url", content: "{url}" }
            }
            if let Some(img) = &props.og_image {
                document::Meta { property: "og:image", content: "{img}" }
            }
        }
    }
}

/// Foundational Fullstack App Shell components for Aphrody sites and apps
pub mod shell {
    use crate::theme::{css_variables, APHRODY_DARK_THEME};
    use dioxus::prelude::*;

    #[derive(Props, Clone, PartialEq)]
    pub struct AppShellProps {
        #[props(default = String::from("Aphrody App"))]
        pub brand: String,
        #[props(optional)]
        pub subtitle: Option<String>,
        pub children: Element,
    }

    #[component]
    pub fn AppShell(props: AppShellProps) -> Element {
        let vars = css_variables(&APHRODY_DARK_THEME);
        rsx! {
            style { "{vars}" }
            div {
                style: "min-height: 100vh; background: var(--yolo-bg-primary); color: var(--yolo-text-primary); font-family: system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; display: flex; flex-direction: column;",
                header {
                    style: "display: flex; align-items: center; justify-content: space-between; padding: 0.875rem 1.5rem; background: var(--yolo-bg-surface); border-bottom: 1px solid var(--yolo-border-subtle);",
                    div {
                        style: "display: flex; align-items: baseline; gap: 0.75rem;",
                        span {
                            style: "font-weight: 700; font-size: 1.125rem; letter-spacing: -0.02em; color: var(--yolo-text-primary);",
                            "{props.brand}"
                        }
                        if let Some(sub) = &props.subtitle {
                            span {
                                style: "font-size: 0.85rem; color: var(--yolo-text-secondary);",
                                "{sub}"
                            }
                        }
                    }
                }
                main {
                    style: "flex: 1; width: 100%; max-width: 1280px; margin: 0 auto; padding: 1.5rem;",
                    {props.children}
                }
            }
        }
    }
}

pub mod prelude {
    pub use crate::seo::SiteMeta;
    pub use crate::shell::AppShell;
    pub use crate::theme::{APHRODY_DARK_THEME, AphrodyThemeTokens};
    pub use dioxus::prelude::*;

    #[cfg(feature = "primitives")]
    pub use dioxus_primitives;

    #[cfg(feature = "sdk")]
    pub use dioxus_sdk;

    #[cfg(feature = "icons")]
    pub use dioxus_icons;
}

#[cfg(test)]
mod tests {
    use crate::theme::{css_variables, APHRODY_DARK_THEME};

    #[test]
    fn test_yolo_ui_theme_reexport() {
        let css = css_variables(&APHRODY_DARK_THEME);
        assert!(css.contains("--yolo-bg-primary: #090b10;"));
        assert!(css.contains("--yolo-accent-secondary: #00d4ff;"));
    }
}
