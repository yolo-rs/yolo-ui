// SPDX-License-Identifier: Apache-2.0
//! One CSS + Tailwind layout engine for every Yolo UI host.
//!
//! A CSS subset is parsed by Lightning CSS, or translated from Tailwind CSS v3/v4 utility classes
//! via [`tailwind`], resolved against a [`TokenTable`] (`var(--md-sys-spacing-*)`) into a
//! host-neutral [`LayoutStyle`], and laid out by Taffy 0.15 (pinned) in a [`LayoutTree`] whose
//! text leaves the host measures. The computed rects export to JSON and feed every host:
//!
//! - web / Dioxus / Blitz: the same CSS runs in the browser or native DOM;
//! - Bevy (`bevy` feature): [`LayoutStyle::to_bevy_node`] produces a `bevy::ui::Node`;
//! - egui (`egui` feature): [`LayoutTree::put`] places widgets at the computed rects.

mod calc;
mod parse;
mod snap;
mod style;
pub mod tailwind;
mod to_taffy;
mod tokens;
mod tree;

#[cfg(feature = "bevy")]
mod to_bevy;
#[cfg(feature = "egui")]
mod to_egui;

pub use parse::{
    Diagnostic, Diagnostics, Rule, SUPPORTED, StyleSheet, parse_style, parse_style_in,
    parse_stylesheet, parse_stylesheet_in,
};
pub use snap::{GRID_4, GRID_8, TapTarget, ensure_tap_target, min_tap_target, snap_to_grid};
pub use style::*;
pub use tailwind::{
    TailwindResolution, apply_tailwind_token, resolve_tailwind, tailwind_to_css,
    tailwind_to_layout_style,
};
pub use tokens::{SPACING_PREFIX, TokenTable};
pub use tree::{Available, LayoutRect, LayoutTree, MeasureRequest, NodeKey, NodeLayout, TreeError};

#[cfg(feature = "bevy")]
pub use to_bevy::css_to_bevy_node;

/// Re-export of the pinned Taffy, for hosts that extend the tree.
pub use taffy;

/// A fixed-advance text measurer: `char_width` px per character, `line_height` px per line,
/// wrapping at word boundaries to the available width. Good enough for previews and tests;
/// real hosts measure with Parley, cosmic-text, egui or the browser.
pub fn monospace_measure(
    char_width: f32,
    line_height: f32,
) -> impl FnMut(&MeasureRequest<'_>) -> (f32, f32) {
    move |req| {
        let full = req.text.chars().count() as f32 * char_width;
        let longest_word = req
            .text
            .split_whitespace()
            .map(|w| w.chars().count())
            .max()
            .unwrap_or(0) as f32
            * char_width;
        let max_width = match (req.known_width, req.available_width) {
            (Some(w), _) => w,
            (None, Available::Definite(w)) => w.max(longest_word),
            (None, Available::MinContent) => longest_word,
            (None, Available::MaxContent) => full,
        };
        if full <= max_width || max_width <= 0.0 {
            return (full, line_height);
        }
        // Greedy word wrap.
        let mut lines = 1u32;
        let mut line = 0.0f32;
        let mut widest = 0.0f32;
        for word in req.text.split_whitespace() {
            let w = word.chars().count() as f32 * char_width;
            let next = if line == 0.0 {
                w
            } else {
                line + char_width + w
            };
            if next > max_width && line > 0.0 {
                widest = widest.max(line);
                lines += 1;
                line = w;
            } else {
                line = next;
            }
        }
        widest = widest.max(line);
        (widest, lines as f32 * line_height)
    }
}

#[cfg(test)]
mod tests;
