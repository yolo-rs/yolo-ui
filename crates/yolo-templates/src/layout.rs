// SPDX-License-Identifier: Apache-2.0
//! Taffy layout of the templates through `yolo-layout`.
//!
//! The box rules of [`crate::style::LAYOUT_CSS`] are parsed by `yolo-layout` and laid out by
//! Taffy 0.15 over a tree that mirrors the rendered markup. Native hosts (Blitz, Bevy, egui)
//! place their widgets at these rects; the tests use them to check that the search is centred,
//! targets keep their size and the compact layout turns the rail into a bottom bar.

use yolo_layout::{Diagnostics, LayoutRect, LayoutTree, NodeKey, StyleSheet, TokenTable, monospace_measure, parse_stylesheet};

use crate::content::AppChrome;
use crate::style::{COMPACT_MAX_WIDTH, LAYOUT_COMPACT_CSS, LAYOUT_CSS};

/// The layout stylesheet for a viewport `width` px wide (compact rules appended at or below
/// [`COMPACT_MAX_WIDTH`]).
///
/// # Errors
/// Returns the `yolo-layout` diagnostics if a rule leaves the supported subset.
pub fn sheet_for(width: f32) -> Result<StyleSheet, Diagnostics> {
    let tokens = TokenTable::material();
    if width <= COMPACT_MAX_WIDTH as f32 {
        parse_stylesheet(&format!("{LAYOUT_CSS}\n{LAYOUT_COMPACT_CSS}"), &tokens)
    } else {
        parse_stylesheet(LAYOUT_CSS, &tokens)
    }
}

/// Computed boxes of the app chrome.
#[derive(Clone, Debug, PartialEq)]
pub struct AppChromeRects {
    /// The whole chrome.
    pub app: LayoutRect,
    /// App bar.
    pub bar: LayoutRect,
    /// Search form.
    pub search: LayoutRect,
    /// Voice button.
    pub voice: LayoutRect,
    /// Rail.
    pub rail: LayoutRect,
    /// Rail destinations.
    pub rail_items: Vec<LayoutRect>,
    /// Content area.
    pub main: LayoutRect,
    /// Media tiles.
    pub tiles: Vec<LayoutRect>,
}

const CHAR_W: f32 = 8.0;
const LINE_H: f32 = 20.0;

/// Lays out the app chrome for `app` in a `width` x `height` viewport.
///
/// # Errors
/// Fails on stylesheet diagnostics or a Taffy error (rendered as text).
pub fn app_chrome(app: &AppChrome, width: f32, height: f32) -> Result<AppChromeRects, String> {
    let sheet = sheet_for(width).map_err(|d| d.to_string())?;
    let st = |s: &str| sheet.style(&[s]);
    let mut t = LayoutTree::new();
    let e = |x: yolo_layout::TreeError| x.to_string();

    let toggle = t.node("toggle", &st(".yt-icon-btn"), &[]).map_err(e)?;
    let name = t.text("app-name", &st(".yt-app-name"), app.app_name).map_err(e)?;
    let start = t.node("start", &st(".yt-app-start"), &[toggle, name]).map_err(e)?;
    let input = t.text("input", &st(".yt-search-input"), app.search_label).map_err(e)?;
    let field = t.node("field", &st(".yt-search-field"), &[input]).map_err(e)?;
    let submit = t.node("submit", &st(".yt-icon-btn"), &[]).map_err(e)?;
    let voice = t.node("voice", &st(".yt-icon-btn"), &[]).map_err(e)?;
    let search = t.node("search", &st(".yt-search"), &[field, submit, voice]).map_err(e)?;
    let end = t.node("end", &st(".yt-app-end"), &[]).map_err(e)?;
    let bar = t.node("bar", &st(".yt-app-bar"), &[start, search, end]).map_err(e)?;

    let mut items = Vec::new();
    for (i, item) in app.rail.iter().enumerate() {
        let icon = t.node(&format!("rail-{i}-icon"), &st(".yt-icon"), &[]).map_err(e)?;
        let label = t.text(&format!("rail-{i}-label"), &st(".yt-rail-label"), item.label).map_err(e)?;
        items.push(t.node(&format!("rail-{i}"), &st(".yt-rail-item"), &[icon, label]).map_err(e)?);
    }
    let list = t.node("rail-list", &st(".yt-rail-list"), &items).map_err(e)?;
    let rail = t.node("rail", &st(".yt-rail"), &[list]).map_err(e)?;

    let heading = t.text("grid-heading", &st(".yt-media-heading"), app.grid_heading).map_err(e)?;
    let mut tiles = Vec::new();
    for (i, m) in app.media.iter().enumerate() {
        let thumb = t.node(&format!("tile-{i}-thumb"), &st(".yt-thumb"), &[]).map_err(e)?;
        let avatar = t.node(&format!("tile-{i}-avatar"), &st(".yt-avatar"), &[]).map_err(e)?;
        let title = t.text(&format!("tile-{i}-title"), &st(".yt-media-text"), m.title).map_err(e)?;
        let meta = t.node(&format!("tile-{i}-meta"), &st(".yt-media-meta"), &[avatar, title]).map_err(e)?;
        tiles.push(t.node(&format!("tile-{i}"), &st(".yt-media-item"), &[thumb, meta]).map_err(e)?);
    }
    let grid = t.node("grid", &st(".yt-media"), &tiles).map_err(e)?;
    let main = t.node("main", &st(".yt-app-main"), &[heading, grid]).map_err(e)?;
    let root = t.node("app", &st(".yt-app"), &[bar, rail, main]).map_err(e)?;

    t.compute(root, Some(width), Some(height), monospace_measure(CHAR_W, LINE_H)).map_err(e)?;
    let r = |k: NodeKey| t.rect(k).map_err(e);
    Ok(AppChromeRects {
        app: r(root)?,
        bar: r(bar)?,
        search: r(search)?,
        voice: r(voice)?,
        rail: r(rail)?,
        rail_items: items.iter().map(|k| r(*k)).collect::<Result<_, _>>()?,
        main: r(main)?,
        tiles: tiles.iter().map(|k| r(*k)).collect::<Result<_, _>>()?,
    })
}

/// Computed boxes of the docs shell.
#[derive(Clone, Debug, PartialEq)]
pub struct DocsRects {
    /// Top bar.
    pub top: LayoutRect,
    /// Navigation tree.
    pub tree: LayoutRect,
    /// Content column.
    pub main: LayoutRect,
}

/// Lays out the docs shell skeleton in a `width` px viewport.
///
/// # Errors
/// Fails on stylesheet diagnostics or a Taffy error.
pub fn docs_shell(width: f32) -> Result<DocsRects, String> {
    let sheet = sheet_for(width).map_err(|d| d.to_string())?;
    let st = |s: &str| sheet.style(&[s]);
    let mut t = LayoutTree::new();
    let e = |x: yolo_layout::TreeError| x.to_string();
    let word = t.text("wordmark", &st(".yt-wordmark"), "yolo-ui docs").map_err(e)?;
    let top = t.node("top", &st(".yt-docs-top"), &[word]).map_err(e)?;
    let link = t.text("link", &st(".yt-tree-link"), "Colour tokens").map_err(e)?;
    let tree = t.node("tree", &st(".yt-docs-tree"), &[link]).map_err(e)?;
    let title = t.text("title", &st(".yt-docs-title"), "Colour tokens").map_err(e)?;
    let main = t.node("main", &st(".yt-docs-main"), &[title]).map_err(e)?;
    let root = t.node("docs", &st(".yt-docs"), &[top, tree, main]).map_err(e)?;
    t.compute(root, Some(width), Some(900.0), monospace_measure(CHAR_W, LINE_H)).map_err(e)?;
    Ok(DocsRects { top: t.rect(top).map_err(e)?, tree: t.rect(tree).map_err(e)?, main: t.rect(main).map_err(e)? })
}

/// Lays out the editorial grid with `stories` cards in a `width` px column; returns card rects.
///
/// # Errors
/// Fails on stylesheet diagnostics or a Taffy error.
pub fn editorial_grid(width: f32, stories: usize) -> Result<Vec<LayoutRect>, String> {
    let sheet = sheet_for(width).map_err(|d| d.to_string())?;
    let mut t = LayoutTree::new();
    let e = |x: yolo_layout::TreeError| x.to_string();
    let mut cards = Vec::new();
    for i in 0..stories {
        let style = if i == 0 { sheet.style(&[".yt-story", ".yt-story-lead"]) } else { sheet.style(&[".yt-story"]) };
        let text = t.text(&format!("story-{i}-title"), &sheet.style(&[".yt-story-title"]), "A headline of a few words").map_err(e)?;
        cards.push(t.node(&format!("story-{i}"), &style, &[text]).map_err(e)?);
    }
    let grid = t.node("grid", &sheet.style(&[".yt-editorial-grid"]), &cards).map_err(e)?;
    t.compute(grid, Some(width), None, monospace_measure(CHAR_W, LINE_H)).map_err(e)?;
    cards.iter().map(|k| t.rect(*k).map_err(e)).collect()
}
