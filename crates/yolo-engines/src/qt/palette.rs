// SPDX-License-Identifier: Apache-2.0
//! Mapping from [`DynamicScheme`] and the 5 [`YoloThemePreset`]s to Qt 6's 21 `QPalette::ColorRole`s
//! (`Window`, `WindowText`, `Base`, `AlternateBase`, `ToolTipBase`, `ToolTipText`, `PlaceholderText`,
//! `Text`, `Button`, `ButtonText`, `BrightText`, `Light`, `Midlight`, `Dark`, `Mid`, `Shadow`,
//! `Highlight`, `HighlightedText`, `Link`, `LinkVisited`, `Accent`).

use serde::{Deserialize, Serialize};
use yolo_shared::YoloThemePreset;
use yolo_tokens::presets::scheme_for_preset;
use yolo_tokens::role::Role;
use yolo_tokens::scheme::DynamicScheme;

/// `(qpalette_role_name, qpalette_enum_value, m3_role)` in `QPalette::ColorRole` order (`0..=20`).
pub const PALETTE_ROLES: &[(&str, i32, Role)] = &[
    ("WindowText", 0, Role::OnSurface),
    ("Button", 1, Role::SurfaceContainer),
    ("Light", 2, Role::SurfaceBright),
    ("Midlight", 3, Role::SurfaceContainerHigh),
    ("Dark", 4, Role::SurfaceContainerLowest),
    ("Mid", 5, Role::SurfaceDim),
    ("Text", 6, Role::OnSurface),
    ("BrightText", 7, Role::OnPrimary),
    ("ButtonText", 8, Role::OnSurface),
    ("Base", 9, Role::SurfaceContainerLowest),
    ("Window", 10, Role::Surface),
    ("Shadow", 11, Role::Shadow),
    ("Highlight", 12, Role::Primary),
    ("HighlightedText", 13, Role::OnPrimary),
    ("Link", 14, Role::Primary),
    ("LinkVisited", 15, Role::Tertiary),
    ("AlternateBase", 16, Role::SurfaceContainerLow),
    // 17 is NoRole
    ("ToolTipBase", 18, Role::InverseSurface),
    ("ToolTipText", 19, Role::InverseOnSurface),
    ("PlaceholderText", 20, Role::OnSurfaceVariant),
    ("Accent", 21, Role::Primary),
];

/// Resolves all 21 `QPalette` entries for `scheme` as `(qpalette_enum_value, 0xAARRGGBB)`.
///
/// Pass the pairs to `QPalette::setColor(static_cast<QPalette::ColorRole>(role), QColor::fromRgba(argb))`
/// on the C++ side or inspect them through [`QPaletteTable`].
#[must_use]
pub fn palette_argb(s: &DynamicScheme) -> [(&'static str, i32, u32); PALETTE_ROLES.len()] {
    let mut out = [("", 0, 0u32); PALETTE_ROLES.len()];
    for (slot, &(name, id, role)) in out.iter_mut().zip(PALETTE_ROLES.iter()) {
        *slot = (name, id, s.argb(role));
    }
    out
}

/// Resolves all 21 `QPalette` entries for one of the 5 [`YoloThemePreset`]s
/// (`Neutral`, `M3`, `Fluent`, `Apple`, `Aphrody`).
#[must_use]
pub fn palette_for_preset(
    preset: YoloThemePreset,
    dark: bool,
) -> [(&'static str, i32, u32); PALETTE_ROLES.len()] {
    let scheme = scheme_for_preset(preset, dark);
    let tokens = preset.tokens(dark);
    let mut table = palette_argb(&scheme);
    for (name, _, argb) in &mut table {
        match *name {
            "Highlight" | "Link" | "Accent" => *argb = tokens.primary_rgba().to_u32_argb(),
            "Window" => *argb = tokens.surface_rgba().to_u32_argb(),
            "Button" => *argb = tokens.surface_container_rgba().to_u32_argb(),
            "Midlight" => *argb = tokens.surface_elevated_rgba().to_u32_argb(),
            "WindowText" | "Text" | "ButtonText" => *argb = tokens.on_surface_rgba().to_u32_argb(),
            "PlaceholderText" => *argb = tokens.on_surface_variant_rgba().to_u32_argb(),
            "LinkVisited" => *argb = tokens.secondary_rgba().to_u32_argb(),
            _ => {}
        }
    }
    table
}

/// Serializable Qt 6 `QPalette` role table with C++ snippet generation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct QPaletteTable {
    /// Optional [`YoloThemePreset`] this table was resolved from.
    pub preset: Option<YoloThemePreset>,
    /// True if resolved for dark mode.
    pub is_dark: bool,
    /// `(role_name, qpalette_enum_id, argb)` entries.
    pub entries: Vec<(String, i32, u32)>,
}

impl QPaletteTable {
    /// Builds a [`QPaletteTable`] for a [`YoloThemePreset`] and mode.
    #[must_use]
    pub fn from_preset(preset: YoloThemePreset, dark: bool) -> Self {
        let raw = palette_for_preset(preset, dark);
        Self {
            preset: Some(preset),
            is_dark: dark,
            entries: raw
                .into_iter()
                .map(|(n, id, argb)| (n.to_owned(), id, argb))
                .collect(),
        }
    }

    /// Look up the `0xAARRGGBB` color for a `QPalette` role name (e.g. `"Accent"`, `"Window"`).
    #[must_use]
    pub fn role_argb(&self, role_name: &str) -> Option<u32> {
        self.entries
            .iter()
            .find(|(n, _, _)| n == role_name)
            .map(|(_, _, argb)| *argb)
    }

    /// Emits a ready-to-compile C++ helper function applying this `QPalette` to a `QGuiApplication`.
    #[must_use]
    pub fn to_cpp_function(&self, fn_name: &str) -> String {
        use std::fmt::Write;
        let mut cpp = format!("inline QPalette {fn_name}() {{\n    QPalette pal;\n");
        for (name, _, argb) in &self.entries {
            let _ = writeln!(
                cpp,
                "    pal.setColor(QPalette::{name}, QColor::fromRgba(0x{argb:08X}u));"
            );
        }
        cpp.push_str("    return pal;\n}\n");
        cpp
    }
}
