// SPDX-License-Identifier: Apache-2.0
//! Theme root, the composed template page and the server-side document renderer.

use dioxus::prelude::*;
use yolo_shared::YoloThemePreset;

use crate::app::RailAppChrome;
use crate::content::{DOCS, Site};
use crate::docs::DocsShell;
use crate::hero::StageHero;
use crate::nav::GlobalNav;
use crate::product::{DownloadChannels, EditorialGrid, ProductPageView, ScrollReveal};
use crate::style::stylesheet;
use crate::theme::Mode;

/// The templates a page can include, in page order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Template {
    /// Global nav with flyouts and the mega menu.
    GlobalNav,
    /// Stage hero with release pill and starfield.
    StageHero,
    /// Product page with sticky section tabs and FAQ.
    ProductPage,
    /// Download channel cards.
    Downloads,
    /// Design-system docs shell.
    DocsShell,
    /// Rail app chrome with centred search and the dense media grid.
    AppChrome,
    /// Editorial grid.
    Editorial,
    /// Scroll-driven 3D reveal.
    ScrollReveal,
}

impl Template {
    /// Every template, in page order.
    pub const ALL: [Self; 8] = [
        Self::GlobalNav,
        Self::StageHero,
        Self::ProductPage,
        Self::Downloads,
        Self::ScrollReveal,
        Self::Editorial,
        Self::AppChrome,
        Self::DocsShell,
    ];

    /// Stable id.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::GlobalNav => "global-nav",
            Self::StageHero => "stage-hero",
            Self::ProductPage => "product-page",
            Self::Downloads => "downloads",
            Self::DocsShell => "docs-shell",
            Self::AppChrome => "app-chrome",
            Self::Editorial => "editorial",
            Self::ScrollReveal => "scroll-reveal",
        }
    }
}

/// Scopes the theme: every `--yt-*` token resolves under this element.
#[component]
pub fn ThemeRoot(preset: YoloThemePreset, mode: Mode, lang: &'static str, children: Element) -> Element {
    rsx! {
        div { class: "yt-root", "data-yt-preset": preset.id(), "data-yt-mode": mode.id(), lang: lang, {children} }
    }
}

/// One template of a page.
#[component]
fn TemplateBlock(site: &'static Site, preset: YoloThemePreset, mode: Mode, template: Template) -> Element {
    let s = site.strings;
    match template {
        Template::GlobalNav => rsx! {},
        Template::StageHero => rsx! {
            StageHero { hero: site.hero, release: site.release, strings: s, starfield: true }
        },
        Template::ProductPage => rsx! {
            ProductPageView { page: site.product, strings: s }
        },
        Template::Downloads => rsx! {
            DownloadChannels { downloads: site.downloads, strings: s }
        },
        Template::ScrollReveal => rsx! {
            ScrollReveal { reveal: site.reveal }
        },
        Template::Editorial => rsx! {
            EditorialGrid { editorial: site.editorial }
        },
        Template::AppChrome => rsx! {
            div { class: "yt-frame",
                RailAppChrome { app: site.app, strings: s }
            }
        },
        Template::DocsShell => rsx! {
            div { class: "yt-frame",
                DocsShell { docs: DOCS, preset: preset, mode: mode, strings: s }
            }
        },
    }
}

/// A page of `site` made of `templates`.
#[component]
pub fn TemplatePage(site: &'static Site, preset: YoloThemePreset, mode: Mode, templates: Vec<Template>) -> Element {
    let s = site.strings;
    let with_nav = templates.contains(&Template::GlobalNav);
    rsx! {
        ThemeRoot { preset: preset, mode: mode, lang: s.lang,
            if with_nav {
                GlobalNav { site: site }
            }
            main { id: "yt-main", tabindex: "-1",
                for (i , t) in templates.iter().copied().enumerate() {
                    TemplateBlock { key: "{i}", site: site, preset: preset, mode: mode, template: t }
                }
            }
            footer { class: "yt-footer",
                p { "{site.footer} {site.domain}" }
            }
        }
    }
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

/// Renders a complete HTML document: one page of `site` with `templates`, themed by `preset`
/// in `mode`, with the stylesheet inlined. Works with no script; hydrating it with
/// `dioxus-web` or rendering the same components with `dioxus-native` adds the signal-driven
/// state on top.
#[must_use]
pub fn render_document(site: &'static Site, preset: YoloThemePreset, mode: Mode, templates: &[Template]) -> String {
    let templates = templates.to_vec();
    let body = dioxus_ssr::render_element(rsx! {
        TemplatePage { site: site, preset: preset, mode: mode, templates: templates }
    });
    format!(
        "<!DOCTYPE html><html lang=\"{lang}\"><head><meta charset=\"utf-8\">\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\
         <meta name=\"color-scheme\" content=\"light dark\">\
         <title>{title}</title><style>{css}</style></head><body style=\"margin:0\">{body}</body></html>",
        lang = site.strings.lang,
        title = escape(&format!("{}: {}, {}", site.name, preset.display_name(), mode.id())),
        css = stylesheet(),
    )
}
