// SPDX-License-Identifier: Apache-2.0
//! Global navigation with per-item flyouts and the mega menu, plus the sticky section tabs.
//!
//! Flyouts are native popovers (`popover="auto"` opened by `popovertarget`), the WAI
//! disclosure-navigation pattern: Tab reaches each trigger, Enter or Space opens its panel,
//! focus moves into the panel in DOM order, Escape or a click outside closes it and returns
//! focus to the trigger, and opening one panel closes the other. The browser exposes the
//! expanded state of the trigger. No script is needed, so server-rendered pages work before
//! hydration.

use dioxus::prelude::*;

use crate::content::{NavItem, Section, Site};
use crate::icons::{Icon, IconName};

/// How a navigation item renders.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NavKind {
    /// A plain link.
    Link,
    /// A single-group flyout.
    Flyout,
    /// A multi-column mega menu.
    Mega,
}

impl NavKind {
    /// The kind of `item`.
    #[must_use]
    pub const fn of(item: &NavItem) -> Self {
        match item.groups.len() {
            0 => Self::Link,
            1 => Self::Flyout,
            _ => Self::Mega,
        }
    }
}

/// Element id of the panel of navigation item `index` on site `site_id`.
#[must_use]
pub fn panel_id(site_id: &str, index: usize) -> String {
    format!("yt-{site_id}-nav-{index}")
}

/// The site header: skip link, wordmark, items with flyouts or mega menus.
#[component]
pub fn GlobalNav(site: &'static Site) -> Element {
    let s = site.strings;
    rsx! {
        header { class: "yt-globalnav",
            a { class: "yt-skip", href: "#yt-main", {s.skip} }
            nav { class: "yt-nav", "aria-label": s.primary_nav,
                a { class: "yt-wordmark", href: "/", {site.name} }
                ul { class: "yt-navlist",
                    for (i , item) in site.nav.iter().enumerate() {
                        li { key: "{i}", class: "yt-navitem",
                            NavEntry { site_id: site.id, index: i, item: *item, overview: s.overview }
                        }
                    }
                }
            }
        }
    }
}

/// One navigation entry: a link, or a trigger and its panel.
#[component]
fn NavEntry(site_id: &'static str, index: usize, item: NavItem, overview: &'static str) -> Element {
    let kind = NavKind::of(&item);
    let id = panel_id(site_id, index);
    if kind == NavKind::Link {
        return rsx! {
            a { class: "yt-navlink", href: item.href, {item.label} }
        };
    }
    let groups_class = if kind == NavKind::Mega { "yt-flyout-groups yt-mega" } else { "yt-flyout-groups" };
    rsx! {
        button {
            class: "yt-navlink yt-navtrigger",
            r#type: "button",
            popovertarget: "{id}",
            "aria-controls": "{id}",
            {item.label}
            Icon { name: IconName::ChevronDown }
        }
        div { class: "yt-flyout", id: "{id}", popover: "auto", "aria-label": item.label,
            div { class: "yt-flyout-inner",
                div { class: "yt-flyout-lead",
                    p { class: "yt-flyout-summary", {item.summary} }
                    a { class: "yt-flyout-overview", href: item.href,
                        "{overview}: {item.label}"
                        Icon { name: IconName::ArrowRight }
                    }
                }
                div { class: groups_class,
                    for (g , group) in item.groups.iter().enumerate() {
                        section { key: "{g}", class: "yt-flyout-group", "aria-labelledby": "{id}-g{g}",
                            h3 { class: "yt-flyout-heading", id: "{id}-g{g}", {group.heading} }
                            ul {
                                for (k , link) in group.links.iter().enumerate() {
                                    li { key: "{k}",
                                        a { class: "yt-flyout-link", href: link.href, {link.label} }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Sticky in-page section tabs. They are links (in-page navigation, not an ARIA tablist): Tab
/// moves between them, Enter jumps to the section, and the current one carries
/// `aria-current`. Sections set `scroll-margin-top` so the sticky bar never hides the heading
/// or the focused element (WCAG 2.4.11).
#[component]
pub fn SectionTabs(label: &'static str, sections: &'static [Section]) -> Element {
    let mut active = use_signal(|| 0usize);
    rsx! {
        nav { class: "yt-sectiontabs", "aria-label": label,
            ul { class: "yt-tabs-list",
                for (i , section) in sections.iter().enumerate() {
                    li { key: "{i}",
                        a {
                            class: "yt-tab",
                            href: "#{section.id}",
                            "aria-current": if active() == i { "true" } else { "false" },
                            onclick: move |_| active.set(i),
                            {section.tab}
                        }
                    }
                }
            }
        }
    }
}
