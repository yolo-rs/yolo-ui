#![doc = include_str!("../README.md")]

pub use dioxus;
pub use dioxus::*;

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

/// Aphrody Labs Design System Tokens (Material 3 Dark Glass & Surface Palette)
pub mod theme {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    pub struct AphrodyThemeTokens {
        pub name: &'static str,
        pub bg_primary: &'static str,
        pub bg_surface: &'static str,
        pub bg_elevated: &'static str,
        pub text_primary: &'static str,
        pub text_secondary: &'static str,
        pub accent_primary: &'static str,
        pub accent_secondary: &'static str,
        pub border_subtle: &'static str,
    }

    pub const APHRODY_DARK_THEME: AphrodyThemeTokens = AphrodyThemeTokens {
        name: "aphrody-m3-dark",
        bg_primary: "#090b10",
        bg_surface: "#111520",
        bg_elevated: "#181e2e",
        text_primary: "#f3f6fc",
        text_secondary: "#9ba6bf",
        accent_primary: "#7c5cff",
        accent_secondary: "#00d4ff",
        border_subtle: "rgba(255, 255, 255, 0.08)",
    };

    /// Returns the baseline CSS variables for Aphrody Labs sites and apps.
    pub fn css_variables(tokens: &AphrodyThemeTokens) -> String {
        format!(
            ":root {{ \
                --yolo-bg-primary: {}; \
                --yolo-bg-surface: {}; \
                --yolo-bg-elevated: {}; \
                --yolo-text-primary: {}; \
                --yolo-text-secondary: {}; \
                --yolo-accent-primary: {}; \
                --yolo-accent-secondary: {}; \
                --yolo-border-subtle: {}; \
            }}",
            tokens.bg_primary,
            tokens.bg_surface,
            tokens.bg_elevated,
            tokens.text_primary,
            tokens.text_secondary,
            tokens.accent_primary,
            tokens.accent_secondary,
            tokens.border_subtle,
        )
    }
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
                document::Meta { property: "og:url", href: "{url}" }
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
