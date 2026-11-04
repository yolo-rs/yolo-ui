// SPDX-License-Identifier: Apache-2.0
//! Design tokens usable through `var()` in layout CSS.

use std::collections::BTreeMap;

/// Prefix of the Material 3 spacing tokens.
pub const SPACING_PREFIX: &str = "--md-sys-spacing-";

/// Named lengths (in logical px) that `var(--name)` resolves to.
///
/// [`TokenTable::material`] holds the Yolo UI spacing scale on the M3 4 dp grid: named steps
/// (`none`, `extra-small` .. `extra-extra-large`) and numeric steps `--md-sys-spacing-N` = `N * 4px`
/// for `N` in `0..=16`. Hosts add their own tokens with [`TokenTable::insert`].
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TokenTable {
    values: BTreeMap<String, f32>,
}

impl TokenTable {
    /// An empty table: every `var()` without a fallback fails.
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }

    /// The Yolo UI Material 3 spacing scale.
    #[must_use]
    pub fn material() -> Self {
        let mut t = Self::default();
        for (name, px) in [
            ("none", 0.0),
            ("extra-small", 4.0),
            ("small", 8.0),
            ("medium", 12.0),
            ("large", 16.0),
            ("extra-large", 24.0),
            ("extra-extra-large", 32.0),
        ] {
            t.insert(format!("{SPACING_PREFIX}{name}"), px);
        }
        for n in 0..=16u8 {
            t.insert(format!("{SPACING_PREFIX}{n}"), f32::from(n) * 4.0);
        }
        t
    }

    /// Adds or replaces `name` (with its leading `--`).
    pub fn insert(&mut self, name: impl Into<String>, px: f32) -> &mut Self {
        self.values.insert(name.into(), px);
        self
    }

    /// The value of `name`, if declared.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<f32> {
        self.values.get(name).copied()
    }

    /// Declared names, sorted.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.values.keys().map(String::as_str)
    }
}
