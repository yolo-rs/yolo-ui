// SPDX-License-Identifier: Apache-2.0
//! Preset colors for icons and favicons, resolved through the `yolo-tokens` HCT/CAM16 dynamic scheme.

use crate::Error;
use serde::Serialize;
use yolo_shared::YoloThemePreset;
use yolo_tokens::presets::scheme_for_preset;
use yolo_tokens::role::Role;

/// The roles a favicon needs, as `#rrggbb`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IconPalette {
    pub preset: String,
    pub dark: bool,
    /// Badge background (`primary`).
    pub background: String,
    /// Mark color on the badge (`on-primary`).
    pub foreground: String,
    /// Mark accent (`tertiary-container`) for two-tone marks.
    pub accent: String,
    /// `theme_color` in the web manifest (`primary`).
    pub theme_color: String,
    /// `background_color` in the web manifest (`surface`).
    pub surface: String,
}

/// `0xAARRGGBB` to `#rrggbb`.
#[must_use]
pub fn argb_to_hex(argb: u32) -> String {
    format!("#{:06x}", argb & 0x00FF_FFFF)
}

/// Resolves a preset id (`aphrody`, `yolo-m3`, `fluent`, `chromium`, ...).
///
/// # Errors
/// [`Error::Preset`] for unknown ids.
pub fn preset_from_id(id: &str) -> Result<YoloThemePreset, Error> {
    YoloThemePreset::from_id(id).ok_or_else(|| {
        let known: Vec<&str> = YoloThemePreset::ALL.iter().map(|p| p.id()).collect();
        Error::Preset(format!(
            "unknown preset `{id}` (known: {})",
            known.join(", ")
        ))
    })
}

/// Builds the icon palette for `preset` from its dynamic scheme.
#[must_use]
pub fn palette(preset: YoloThemePreset, dark: bool) -> IconPalette {
    let scheme = scheme_for_preset(preset, dark);
    IconPalette {
        preset: preset.id().to_owned(),
        dark,
        background: argb_to_hex(scheme.argb(Role::Primary)),
        foreground: argb_to_hex(scheme.argb(Role::OnPrimary)),
        accent: argb_to_hex(scheme.argb(Role::TertiaryContainer)),
        theme_color: argb_to_hex(scheme.argb(Role::Primary)),
        surface: argb_to_hex(scheme.argb(Role::Surface)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_preset_resolves_distinct_badge_colors() {
        for preset in YoloThemePreset::ALL {
            let p = palette(preset, false);
            assert_eq!(p.background.len(), 7);
            assert_ne!(p.background, p.foreground, "{}", p.preset);
        }
        assert_eq!(preset_from_id("aphrody").unwrap(), YoloThemePreset::Aphrody);
        assert!(preset_from_id("sf").is_err());
    }
}
