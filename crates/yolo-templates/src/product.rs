// SPDX-License-Identifier: Apache-2.0
//! Product page template, download channel cards, editorial grid and the scroll reveal.

use dioxus::prelude::*;

use crate::content::{Downloads, Editorial, ProductPage, Reveal, Sketch, Strings};
use crate::hero::{Object3d, ObjectPose};
use crate::icons::{Icon, IconName};
use crate::nav::SectionTabs;

/// A simplified drawing of the product UI (window, sidebar, rows, floating card), described
/// to assistive tech by `sketch.alt`. Shapes only: no screenshot, no reference artwork.
#[component]
pub fn UiSketch(sketch: Sketch) -> Element {
    rsx! {
        div { class: "yt-sketch", role: "img", "aria-label": sketch.alt,
            div { class: "yt-sketch-window",
                div { class: "yt-sketch-bar",
                    span { class: "yt-sketch-dot" }
                    span { class: "yt-sketch-dot" }
                    span { class: "yt-sketch-dot" }
                    span { class: "yt-sketch-title", {sketch.window_title} }
                }
                div { class: "yt-sketch-body",
                    div { class: "yt-sketch-side",
                        for (i , item) in sketch.sidebar.iter().enumerate() {
                            span { key: "{i}", class: if i == 0 { "yt-sketch-side-item is-active" } else { "yt-sketch-side-item" }, {*item} }
                        }
                    }
                    div { class: "yt-sketch-rows",
                        for (i , row) in sketch.rows.iter().enumerate() {
                            span { key: "{i}", class: "yt-sketch-row", {*row} }
                        }
                    }
                }
            }
            div { class: "yt-sketch-card", {sketch.card} }
        }
    }
}

/// The product page: hero with UI drawing, sticky section tabs, sections and FAQ.
#[component]
pub fn ProductPageView(page: ProductPage, strings: &'static Strings) -> Element {
    let hero = page.hero;
    rsx! {
        article { class: "yt-product", "aria-labelledby": "yt-product-title",
            div { class: "yt-product-hero",
                div { class: "yt-product-copy",
                    p { class: "yt-eyebrow", {hero.eyebrow} }
                    h2 { class: "yt-product-title", id: "yt-product-title", {hero.title} }
                    p { class: "yt-lede", {hero.lede} }
                    div { class: "yt-ctas yt-ctas-start",
                        a { class: "yt-btn yt-btn-primary", href: hero.primary.href, {hero.primary.label} }
                        if let Some(s) = hero.secondary {
                            a { class: "yt-btn yt-btn-secondary", href: s.href, {s.label} }
                        }
                    }
                }
                UiSketch { sketch: page.sketch }
            }
            SectionTabs { label: page.tabs_label, sections: page.sections }
            div { class: "yt-sections",
                for sec in page.sections.iter() {
                    section {
                        key: "{sec.id}",
                        class: "yt-section",
                        id: sec.id,
                        "aria-labelledby": "{sec.id}-h",
                        h3 { id: "{sec.id}-h", {sec.heading} }
                        p { {sec.body} }
                        ul { class: "yt-points",
                            for (i , point) in sec.points.iter().enumerate() {
                                li { key: "{i}", {*point} }
                            }
                        }
                    }
                }
                section { class: "yt-faq", "aria-labelledby": "yt-faq-h",
                    h3 { id: "yt-faq-h", {strings.faq} }
                    div { class: "yt-faq-list",
                        for (i , faq) in page.faqs.iter().enumerate() {
                            details { key: "{i}", class: "yt-faq-item",
                                summary { {faq.question} }
                                p { {faq.answer} }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Download channel cards. The recommended channel gets the filled button and a text badge
/// (never colour alone).
#[component]
pub fn DownloadChannels(downloads: Downloads, strings: &'static Strings) -> Element {
    rsx! {
        section { class: "yt-downloads", id: "channels", "aria-labelledby": "yt-downloads-h",
            h2 { id: "yt-downloads-h", {downloads.heading} }
            p { class: "yt-lede", {downloads.intro} }
            ul { class: "yt-channels",
                for (i , c) in downloads.channels.iter().enumerate() {
                    li { key: "{i}", class: if c.recommended { "yt-channel is-recommended" } else { "yt-channel" },
                        div { class: "yt-channel-head",
                            h3 { {c.name} }
                            if c.recommended {
                                span { class: "yt-badge", {strings.recommended} }
                            }
                        }
                        p { class: "yt-cadence", {c.cadence} }
                        p { {c.description} }
                        p { class: "yt-platforms-label", "{strings.available_on}:" }
                        ul { class: "yt-platforms",
                            for (k , platform) in c.platforms.iter().enumerate() {
                                li { key: "{k}", {*platform} }
                            }
                        }
                        a {
                            class: if c.recommended { "yt-btn yt-btn-primary" } else { "yt-btn yt-btn-secondary" },
                            href: c.action.href,
                            Icon { name: IconName::Download }
                            {c.action.label}
                        }
                    }
                }
            }
        }
    }
}

/// Editorial grid: oversized headline, a lead story with an illustration panel, then a grid.
#[component]
pub fn EditorialGrid(editorial: Editorial) -> Element {
    rsx! {
        section { class: "yt-editorial", "aria-labelledby": "yt-editorial-h",
            header { class: "yt-editorial-head",
                p { class: "yt-kicker", {editorial.kicker} }
                h2 { class: "yt-display", id: "yt-editorial-h", {editorial.headline} }
            }
            ul { class: "yt-editorial-grid",
                for (i , story) in editorial.stories.iter().enumerate() {
                    li { key: "{i}", class: if i == 0 { "yt-story yt-story-lead" } else { "yt-story" },
                        if i == 0 {
                            div { class: "yt-illus",
                                Object3d { pose: ObjectPose::Still }
                            }
                        }
                        p { class: "yt-kicker", {story.kicker} }
                        h3 { class: "yt-story-title",
                            a { href: story.href, {story.title} }
                        }
                        p { class: "yt-dek", {story.dek} }
                    }
                }
            }
        }
    }
}

/// Scroll-driven 3D reveal: two title halves converge while the object turns to face the
/// reader, then the copy appears. Runs on CSS scroll timelines where supported and motion is
/// allowed; otherwise it shows the final frame ([`crate::motion::RevealFrame::FINAL`]).
#[component]
pub fn ScrollReveal(reveal: Reveal) -> Element {
    rsx! {
        section { class: "yt-reveal", "aria-labelledby": "yt-reveal-h",
            div { class: "yt-reveal-track",
                div { class: "yt-reveal-sticky",
                    h2 { class: "yt-split", id: "yt-reveal-h",
                        span { class: "yt-split-a", "{reveal.left} " }
                        span { class: "yt-split-b", {reveal.right} }
                    }
                    div { class: "yt-reveal-object",
                        Object3d { pose: ObjectPose::Scroll }
                    }
                    p { class: "yt-reveal-body", {reveal.body} }
                }
            }
        }
    }
}
