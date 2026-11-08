// SPDX-License-Identifier: Apache-2.0
//! Ready-to-use `DynamicScheme` and `ThemeRequest` builders for the 8 built-in `YoloThemePreset`s:
//!
//! 1. [`yolo_neutral`] (`0xFF63_66F1` — neutral customizable default)
//! 2. [`yolo_m3`] (`0xFF67_50A4` — Google Material Design 3 / Jetpack Compose)
//! 3. [`yolo_fluent`] (`0xFF00_78D4` — Microsoft Fluent UI 2 / WinUI 3)
//! 4. [`yolo_apple`] (`0xFF00_7AFF` — Apple macOS Tahoe Liquid Glass & iOS Cupertino HIG)
//! 5. [`yolo_omarchy`] (`0xFF7A_A2F7` — Omacom Omarchy `Tokyo Night` + 23 official `OmarchySubTheme`s)
//! 6. [`yolo_hyprland`] (`0xFF33_CCFF` — Hyprwm / Hyprland Wayland Compositor `#33ccff` -> `#00ff99` 45deg border + kawase blur)
//! 7. [`yolo_aphrody`] (`0xFF17_335C` — Byron Love / Aphrody 10-teinte sprite atlas & celestial gold)

use crate::scheme::{DynamicScheme, Variant};
use crate::theme::ThemeRequest;
pub use yolo_shared::{
    ByronLoveColorTeinte, ByronLovePetSpec, HyprlandDecorationSpec, OmarchySubTheme,
    OmarchySubThemeSpec, YoloManifest, YoloStyle, YoloThemePreset, YoloThemeTokens,
    BYRON_LOVE_PALETTE, BYRON_LOVE_PET_SPEC, HYPRLAND_DECORATION_SPEC, OMARCHY_THEMES,
};

/// Canonical opaque ARGB seed for `YoloThemePreset::Neutral` (`#6366F1`).
pub const NEUTRAL_SEED: u32 = 0xFF63_66F1;

/// Canonical opaque ARGB seed for `YoloThemePreset::M3` (`#6750A4`).
pub const M3_SEED: u32 = 0xFF67_50A4;

/// Canonical opaque ARGB seed for `YoloThemePreset::Fluent` (`#0078D4`).
pub const FLUENT_SEED: u32 = 0xFF00_78D4;

/// Canonical opaque ARGB seed for `YoloThemePreset::Apple` (`#007AFF`).
pub const APPLE_SEED: u32 = 0xFF00_7AFF;

/// Canonical opaque ARGB seed for `YoloThemePreset::Omarchy` (`#7AA2F7` Tokyo Night blue).
pub const OMARCHY_SEED: u32 = 0xFF7A_A2F7;

/// Canonical opaque ARGB seed for `YoloThemePreset::Hyprland` (`#33CCFF` Hyprland electric cyan).
pub const HYPRLAND_SEED: u32 = 0xFF33_CCFF;

/// Canonical opaque ARGB seed for `YoloThemePreset::Aphrody` (`#17335C` Byron Love Royal Azur).
pub const APHRODY_SEED: u32 = 0xFF17_335C;

/// Canonical opaque ARGB seed for `YoloThemePreset::Chromium` (`#1A73E8` Chrome blue).
pub const CHROMIUM_SEED: u32 = 0xFF1A_73E8;

/// Builds the neutral default [`DynamicScheme`] (`0xFF63_66F1`, `Variant::Neutral`).
#[must_use]
pub fn yolo_neutral(dark: bool) -> DynamicScheme {
    DynamicScheme::from_argb(NEUTRAL_SEED, Variant::Neutral, dark, 0.0)
}

/// Builds the Google Material Design 3 [`DynamicScheme`] (`0xFF67_50A4`, `Variant::TonalSpot`).
#[must_use]
pub fn yolo_m3(dark: bool) -> DynamicScheme {
    DynamicScheme::from_argb(M3_SEED, Variant::TonalSpot, dark, 0.0)
}

/// Builds the Microsoft Fluent UI 2 / WinUI 3 [`DynamicScheme`] (`0xFF00_78D4`, `Variant::Content`).
#[must_use]
pub fn yolo_fluent(dark: bool) -> DynamicScheme {
    DynamicScheme::from_argb(FLUENT_SEED, Variant::Content, dark, 0.0)
}

/// Builds the Apple HIG Liquid Glass / Cupertino [`DynamicScheme`] (`0xFF00_7AFF`, `Variant::Fidelity`).
#[must_use]
pub fn yolo_apple(dark: bool) -> DynamicScheme {
    DynamicScheme::from_argb(APPLE_SEED, Variant::Fidelity, dark, 0.0)
}

/// Builds the Omacom Omarchy [`DynamicScheme`] (`0xFF7A_A2F7` Tokyo Night default, `Variant::Content`).
#[must_use]
pub fn yolo_omarchy(dark: bool) -> DynamicScheme {
    DynamicScheme::from_argb(OMARCHY_SEED, Variant::Content, dark, 0.0)
}

/// Builds an Omacom Omarchy [`DynamicScheme`] for any of the 23 official [`OmarchySubTheme`]s.
#[must_use]
pub fn yolo_omarchy_subtheme(subtheme: OmarchySubTheme) -> DynamicScheme {
    let spec = subtheme.spec();
    DynamicScheme::from_argb(spec.seed_argb, Variant::Content, spec.is_dark, 0.0)
}

/// Builds the Hyprwm / Hyprland [`DynamicScheme`] (`0xFF33_CCFF` electric cyan, `Variant::Vibrant`).
#[must_use]
pub fn yolo_hyprland(dark: bool) -> DynamicScheme {
    DynamicScheme::from_argb(HYPRLAND_SEED, Variant::Vibrant, dark, 0.0)
}

/// Builds the Byron Love / Aphrody [`DynamicScheme`] (`0xFF17_335C`, `Variant::TonalSpot`).
#[must_use]
pub fn yolo_aphrody(dark: bool) -> DynamicScheme {
    DynamicScheme::from_argb(APHRODY_SEED, Variant::TonalSpot, dark, 0.0)
}

/// Builds the Chromium Views/WebUI [`DynamicScheme`] from the customizable Chrome blue HCT seed.
#[must_use]
pub fn yolo_chromium(dark: bool) -> DynamicScheme {
    DynamicScheme::from_argb(CHROMIUM_SEED, Variant::Content, dark, 0.0)
}

/// Returns the canonical [`Variant`] associated with each [`YoloThemePreset`].
#[must_use]
pub const fn variant_for_preset(preset: YoloThemePreset) -> Variant {
    match preset {
        YoloThemePreset::Neutral => Variant::Neutral,
        YoloThemePreset::M3 => Variant::TonalSpot,
        YoloThemePreset::Fluent => Variant::Content,
        YoloThemePreset::Apple => Variant::Fidelity,
        YoloThemePreset::Omarchy => Variant::Content,
        YoloThemePreset::Hyprland => Variant::Vibrant,
        YoloThemePreset::Aphrody => Variant::TonalSpot,
        YoloThemePreset::Chromium => Variant::Content,
    }
}

/// Builds a validated [`ThemeRequest`] for any of the 7 [`YoloThemePreset`]s.
#[must_use]
pub fn request_for_preset(preset: YoloThemePreset) -> ThemeRequest {
    ThemeRequest::new(preset.seed_argb(), variant_for_preset(preset), 0.0)
        .expect("all built-in YoloThemePreset seeds are valid opaque ARGB colors")
}

/// Resolves the [`DynamicScheme`] for any [`YoloThemePreset`] in light or dark mode.
#[must_use]
pub fn scheme_for_preset(preset: YoloThemePreset, dark: bool) -> DynamicScheme {
    match preset {
        YoloThemePreset::Neutral => yolo_neutral(dark),
        YoloThemePreset::M3 => yolo_m3(dark),
        YoloThemePreset::Fluent => yolo_fluent(dark),
        YoloThemePreset::Apple => yolo_apple(dark),
        YoloThemePreset::Omarchy => yolo_omarchy(dark),
        YoloThemePreset::Hyprland => yolo_hyprland(dark),
        YoloThemePreset::Aphrody => yolo_aphrody(dark),
        YoloThemePreset::Chromium => yolo_chromium(dark),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::role::Role;

    #[test]
    fn all_eight_presets_and_twenty_three_omarchy_subthemes_resolve() {
        assert_eq!(YoloThemePreset::ALL.len(), 8);
        assert_eq!(OmarchySubTheme::ALL.len(), 23);

        for preset in YoloThemePreset::ALL {
            let req = request_for_preset(preset);
            assert_eq!(req.seed(), preset.seed_argb());
            let dark_scheme = scheme_for_preset(preset, true);
            assert_eq!(dark_scheme.argb(Role::Primary) >> 24, 0xFF);
        }

        for sub in OmarchySubTheme::ALL {
            let scheme = yolo_omarchy_subtheme(sub);
            assert_eq!(scheme.source.to_argb(), sub.spec().seed_argb);
        }
    }

    #[test]
    fn byron_love_palette_and_hyprland_decoration_match_seeds() {
        assert_eq!(BYRON_LOVE_PALETTE.len(), 10);
        assert_eq!(BYRON_LOVE_PALETTE[8].argb, APHRODY_SEED);
        assert_eq!(HYPRLAND_DECORATION_SPEC.active_border_start_argb, HYPRLAND_SEED);
    }
}
