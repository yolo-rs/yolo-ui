// SPDX-License-Identifier: Apache-2.0
//! Host-neutral symbol path resolution and vector glyph constants for native and web renderers.

/// Material Icons `check` (24x24 view box).
pub const CHECK_PATH: &str = "M9 16.17 4.83 12l-1.42 1.41L9 19 21 7l-1.41-1.41z";
/// Material Icons `remove` (24x24 view box), the indeterminate checkbox bar.
pub const INDETERMINATE_PATH: &str = "M19 13H5v-2h14v2z";

/// Resolves a symbol icon name to SVG path data in a 24x24 view box.
pub trait SymbolPaths {
    /// Path data for `name`, or `None` to skip the glyph.
    fn path(&self, name: &str) -> Option<String>;
}

/// No symbol font available: only the built-in check and indeterminate glyphs are drawn.
pub struct NoSymbols;

impl SymbolPaths for NoSymbols {
    fn path(&self, _name: &str) -> Option<String> {
        None
    }
}

impl<F: Fn(&str) -> Option<String>> SymbolPaths for F {
    fn path(&self, name: &str) -> Option<String> {
        self(name)
    }
}

/// Resolves the canonical 0..15 index in `yolo_tokens::typography::NAMES` for a type-scale name.
#[must_use]
pub fn type_style_index(name: &str) -> Option<usize> {
    yolo_tokens::typography::NAMES
        .iter()
        .position(|candidate| *candidate == name)
}

#[cfg(test)]
mod tests {
    use super::{NoSymbols, SymbolPaths, type_style_index};

    #[test]
    fn type_names_map_to_canonical_indices() {
        assert!(type_style_index("label-large").is_some());
        assert!(type_style_index("label-huge").is_none());
        assert_eq!(NoSymbols.path("check"), None);
    }
}
