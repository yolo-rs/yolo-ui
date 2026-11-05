// SPDX-License-Identifier: Apache-2.0
//! App chrome: collapsible rail, centered search with voice, and the dense media grid.

use dioxus::prelude::*;

use crate::content::{AppChrome, MediaItem, Strings};
use crate::icons::{Icon, IconName};

/// The rows of a virtualized grid to mount for a scroll position.
///
/// `scroll` and `viewport` are in px, rows are `row_height` px tall with `columns` items each,
/// and `overscan` extra rows are kept on both sides. Returns the item index range.
#[must_use]
pub fn visible_window(
    scroll: f32,
    viewport: f32,
    row_height: f32,
    columns: usize,
    total: usize,
    overscan: usize,
) -> std::ops::Range<usize> {
    if total == 0 || columns == 0 || row_height <= 0.0 || !scroll.is_finite() || !viewport.is_finite() {
        return 0..0;
    }
    let rows = total.div_ceil(columns);
    let first = (scroll.max(0.0) / row_height).floor() as usize;
    let last = ((scroll.max(0.0) + viewport.max(0.0)) / row_height).ceil() as usize;
    let start_row = first.saturating_sub(overscan).min(rows);
    let end_row = (last + overscan).min(rows);
    (start_row * columns).min(total)..(end_row * columns).min(total)
}

/// Columns a `repeat(auto-fill, minmax(min, 1fr))` grid gets in `width` px with `gap` px gaps.
#[must_use]
pub fn auto_fill_columns(width: f32, min: f32, gap: f32) -> usize {
    if min <= 0.0 || width < min {
        return 1;
    }
    (((width + gap) / (min + gap)).floor() as usize).max(1)
}

/// One tile: generated artwork (hue from the data, no image file), badge, avatar initial,
/// title link covering the tile, source and metadata.
#[component]
pub fn MediaTile(item: MediaItem) -> Element {
    let initial = item.source.chars().next().unwrap_or('A');
    rsx! {
        li { class: "yt-media-item",
            div { class: "yt-thumb", style: "--yt-hue:{item.hue}", "aria-hidden": "true",
                span { class: "yt-thumb-badge", {item.badge} }
            }
            div { class: "yt-media-meta",
                span { class: "yt-avatar", style: "--yt-hue:{item.hue}", "aria-hidden": "true", "{initial}" }
                div { class: "yt-media-text",
                    h3 { class: "yt-media-title",
                        a { href: item.href, {item.title} }
                    }
                    p { class: "yt-media-source", {item.source} }
                    p { class: "yt-media-line", {item.meta} }
                }
            }
        }
    }
}

/// Dense media grid with metadata rows. Tiles use `content-visibility: auto`, and native hosts
/// window long lists with [`visible_window`].
#[component]
pub fn MediaGrid(heading: &'static str, items: &'static [MediaItem]) -> Element {
    rsx! {
        section { class: "yt-media-section", "aria-labelledby": "yt-media-h",
            h2 { class: "yt-media-heading", id: "yt-media-h", {heading} }
            ul { class: "yt-media",
                for (i , item) in items.iter().enumerate() {
                    MediaTile { key: "{i}", item: *item }
                }
            }
        }
    }
}

/// App chrome: an app bar with the rail switch and a search form centered in the bar, a rail
/// of destinations (icon plus label, labels beside icons when expanded, a bottom bar on
/// narrow screens) and the media grid as content.
#[component]
pub fn RailAppChrome(app: AppChrome, strings: &'static Strings) -> Element {
    let mut expanded = use_signal(|| false);
    let mut query = use_signal(String::new);
    rsx! {
        div { class: "yt-app",
            div { class: "yt-app-bar",
                div { class: "yt-app-start",
                    label { class: "yt-icon-btn yt-rail-toggle",
                        input {
                            class: "yt-rail-switch yt-sr",
                            r#type: "checkbox",
                            role: "switch",
                            "aria-label": strings.expand_rail,
                            checked: expanded(),
                            onchange: move |_| expanded.set(!expanded()),
                        }
                        Icon { name: IconName::Menu }
                    }
                    span { class: "yt-app-name", {app.app_name} }
                }
                form { class: "yt-search", role: "search", action: "/search", method: "get",
                    label { class: "yt-search-field",
                        span { class: "yt-sr", {app.search_label} }
                        input {
                            class: "yt-search-input",
                            r#type: "search",
                            name: "q",
                            placeholder: app.search_label,
                            autocomplete: "off",
                            value: "{query}",
                            oninput: move |evt| query.set(evt.value()),
                        }
                    }
                    button { class: "yt-icon-btn yt-search-submit", r#type: "submit", "aria-label": strings.search,
                        Icon { name: IconName::Search }
                    }
                    button { class: "yt-icon-btn yt-voice", r#type: "button", "aria-label": strings.voice_search,
                        Icon { name: IconName::Mic }
                    }
                }
                div { class: "yt-app-end" }
            }
            nav { class: "yt-rail", "aria-label": strings.app_nav,
                ul { class: "yt-rail-list",
                    for (i , item) in app.rail.iter().enumerate() {
                        li { key: "{i}",
                            a {
                                class: "yt-rail-item",
                                href: item.href,
                                "aria-current": if i == 0 { "page" } else { "false" },
                                Icon { name: item.icon }
                                span { class: "yt-rail-label", {item.label} }
                            }
                        }
                    }
                }
            }
            div { class: "yt-app-main",
                MediaGrid { heading: app.grid_heading, items: app.media }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn window_covers_the_viewport_plus_overscan() {
        // 1000 items, 4 per row, 300 px rows, 900 px viewport scrolled to row 10.
        let w = visible_window(3000.0, 900.0, 300.0, 4, 1000, 2);
        assert_eq!(w, (8 * 4)..(15 * 4));
        assert_eq!(visible_window(0.0, 900.0, 300.0, 4, 1000, 2), 0..(5 * 4));
        // Near the end the range stops at `total`.
        assert_eq!(visible_window(1e9, 900.0, 300.0, 4, 10, 1).end, 10);
        assert_eq!(visible_window(0.0, 900.0, 300.0, 0, 10, 1), 0..0);
        assert_eq!(visible_window(f32::NAN, 900.0, 300.0, 4, 10, 1), 0..0);
    }

    #[test]
    fn auto_fill_matches_css_track_count() {
        // 1440 wide minus 72 px rail and 48 px padding leaves 1320 px.
        assert_eq!(auto_fill_columns(1320.0, 300.0, 16.0), 4);
        assert_eq!(auto_fill_columns(366.0, 300.0, 16.0), 1);
        assert_eq!(auto_fill_columns(100.0, 300.0, 16.0), 1);
    }
}
