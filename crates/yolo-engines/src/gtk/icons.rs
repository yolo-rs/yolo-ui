// SPDX-License-Identifier: Apache-2.0
//! Material Symbols variable-font icon descriptors and optional `system-gtk4` `GtkLabel` helpers.

use serde::{Deserialize, Serialize};

/// Common Material Symbols codepoints.
pub mod codepoints {
    /// `home` (`\u{e88a}`).
    pub const HOME: char = '\u{e88a}';
    /// `add` (`\u{e145}`).
    pub const ADD: char = '\u{e145}';
    /// `settings` (`\u{e8b8}`).
    pub const SETTINGS: char = '\u{e8b8}';
    /// `palette` (`\u{e40a}`).
    pub const PALETTE: char = '\u{e40a}';
    /// `search` (`\u{e8b6}`).
    pub const SEARCH: char = '\u{e8b6}';
    /// `check` (`\u{e5ca}`).
    pub const CHECK: char = '\u{e5ca}';
    /// `close` (`\u{e5cd}`).
    pub const CLOSE: char = '\u{e5cd}';
}

/// Pure-Rust Material Symbols variable-font glyph descriptor.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaterialSymbol {
    /// Unicode codepoint in the Material Symbols font.
    pub codepoint: char,
    /// Whether the `FILL` axis is `1` (filled) or `0` (outlined).
    pub filled: bool,
    /// Optical size in dp (`20`, `24`, `40`, `48`).
    pub optical_size_dp: u16,
}

impl MaterialSymbol {
    /// Creates a new [`MaterialSymbol`] with 24 dp optical size.
    #[must_use]
    pub const fn new(codepoint: char, filled: bool) -> Self {
        Self {
            codepoint,
            filled,
            optical_size_dp: 24,
        }
    }

    /// CSS class (`m3-symbol` or `m3-symbol-filled`).
    #[must_use]
    pub const fn css_class(self) -> &'static str {
        if self.filled {
            "m3-symbol-filled"
        } else {
            "m3-symbol"
        }
    }
}

/// Renders a Material Symbols codepoint inside a `GtkLabel` with `.m3-symbol` (and
/// `.m3-symbol-filled` when `filled` is true).
#[cfg(feature = "system-gtk4")]
#[must_use]
pub fn symbol(codepoint: char, filled: bool) -> gtk::Label {
    let mut buf = [0u8; 4];
    let lbl = gtk::Label::new(Some(codepoint.encode_utf8(&mut buf)));
    lbl.add_css_class("m3-symbol");
    if filled {
        lbl.add_css_class("m3-symbol-filled");
    }
    lbl
}
