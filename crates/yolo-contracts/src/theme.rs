// SPDX-License-Identifier: Apache-2.0
//! Neutral-by-default and 7-preset theme selection over the canonical `yolo-tokens` engine.

use serde::{Deserialize, Serialize};

pub use crate::facade::platform::ThemeMode;
pub use yolo_shared::{
    HyprlandDecorationSpec, OmarchySubTheme, OmarchySubThemeSpec, YoloManifest, YoloStyle,
    YoloThemePreset, YoloThemeTokens, HYPRLAND_DECORATION_SPEC, OMARCHY_THEMES,
};
use yolo_tokens::presets;
use yolo_tokens::scheme::DynamicScheme;
pub use yolo_tokens::theme::{ThemeExport, ThemeRequest};

/// Theme context shared by components and rendering adapters.
///
/// Defaults to [`YoloThemePreset::Neutral`] and [`ThemeMode::System`].
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct Theme {
    /// Active theme preset (`Neutral` by default).
    pub preset: YoloThemePreset,
    /// User or system theme mode preference.
    pub mode: ThemeMode,
}

impl Theme {
    /// Creates a theme context for a specific [`YoloThemePreset`].
    #[must_use]
    pub const fn with_preset(preset: YoloThemePreset) -> Self {
        Self {
            preset,
            mode: ThemeMode::System,
        }
    }

    /// Returns the neutral customizable default [`ThemeRequest`] (`YoloThemePreset::Neutral`, `#6366F1`).
    #[must_use]
    pub fn default_request() -> ThemeRequest {
        Self::preset_request(YoloThemePreset::Neutral)
    }

    /// Returns the validated [`ThemeRequest`] for any built-in [`YoloThemePreset`]:
    /// - `YoloThemePreset::Neutral` (`#6366F1`)
    /// - `YoloThemePreset::M3` (`#6750A4`)
    /// - `YoloThemePreset::Fluent` (`#0078D4`)
    /// - `YoloThemePreset::Apple` (`#007AFF`)
    /// - `YoloThemePreset::Omarchy` (`#7AA2F7` Tokyo Night + 23 subthemes)
    /// - `YoloThemePreset::Hyprland` (`#33CCFF` -> `#00FF99` 45deg border + kawase blur)
    /// - `YoloThemePreset::Aphrody` (`#17335C` Byron Love seed)
    #[must_use]
    pub fn preset_request(preset: YoloThemePreset) -> ThemeRequest {
        presets::request_for_preset(preset)
    }

    /// Convenience helper for `YoloThemePreset::Neutral` (`#6366F1`).
    #[must_use]
    pub fn neutral_request() -> ThemeRequest {
        Self::preset_request(YoloThemePreset::Neutral)
    }

    /// Convenience helper for `YoloThemePreset::M3` (`#6750A4`).
    #[must_use]
    pub fn m3_request() -> ThemeRequest {
        Self::preset_request(YoloThemePreset::M3)
    }

    /// Convenience helper for `YoloThemePreset::Fluent` (`#0078D4`).
    #[must_use]
    pub fn fluent_request() -> ThemeRequest {
        Self::preset_request(YoloThemePreset::Fluent)
    }

    /// Convenience helper for `YoloThemePreset::Apple` (`#007AFF`).
    #[must_use]
    pub fn apple_request() -> ThemeRequest {
        Self::preset_request(YoloThemePreset::Apple)
    }

    /// Convenience helper for `YoloThemePreset::Omarchy` (`#7AA2F7` Tokyo Night).
    #[must_use]
    pub fn omarchy_request() -> ThemeRequest {
        Self::preset_request(YoloThemePreset::Omarchy)
    }

    /// Convenience helper for `YoloThemePreset::Hyprland` (`#33CCFF` electric cyan).
    #[must_use]
    pub fn hyprland_request() -> ThemeRequest {
        Self::preset_request(YoloThemePreset::Hyprland)
    }

    /// Convenience helper for `YoloThemePreset::Aphrody` (`#17335C` Byron Love seed).
    #[must_use]
    pub fn aphrody_request() -> ThemeRequest {
        Self::preset_request(YoloThemePreset::Aphrody)
    }

    /// Apply the user's contrast preference to an already validated request.
    #[must_use]
    pub const fn request(self, request: ThemeRequest) -> ThemeRequest {
        match self.mode {
            ThemeMode::HighContrast => request.high_contrast(),
            _ => request,
        }
    }

    /// Resolve host darkness and user preference without introducing another
    /// role table. High contrast follows host darkness at MCU contrast level 1.
    #[must_use]
    pub fn scheme(self, request: ThemeRequest, system_dark: bool) -> DynamicScheme {
        let dark = match self.mode {
            ThemeMode::Light => false,
            ThemeMode::Dark => true,
            ThemeMode::System | ThemeMode::HighContrast => system_dark,
        };
        self.request(request).scheme(dark)
    }

    /// Resolves the [`DynamicScheme`] for `self.preset` directly.
    #[must_use]
    pub fn preset_scheme(self, system_dark: bool) -> DynamicScheme {
        self.scheme(Self::preset_request(self.preset), system_dark)
    }

    /// Export both modes for Bun, browser, native and OS consumers, applying
    /// the user's high-contrast preference when selected.
    #[must_use]
    pub fn export(self, request: ThemeRequest) -> ThemeExport {
        self.request(request).export()
    }

    /// Produce the baseline M3 CSS variables from the shared token crate.
    #[must_use]
    pub fn baseline_css() -> String {
        yolo_tokens::color::export_css(&yolo_tokens::color::BASELINE)
    }
}

#[cfg(test)]
mod tests {
    use super::{Theme, ThemeMode, YoloThemePreset};

    #[test]
    fn theme_mode_uses_the_canonical_token_facade() {
        let mode: crate::facade::platform::ThemeMode = ThemeMode::Dark;
        assert_eq!(mode, crate::facade::platform::ThemeMode::Dark);
    }

    #[test]
    fn default_theme_is_neutral_and_all_seven_presets_resolve() {
        let default_theme = Theme::default();
        assert_eq!(default_theme.preset, YoloThemePreset::Neutral);

        let default_req = Theme::default_request();
        assert_eq!(default_req.seed(), 0xFF63_66F1);

        assert_eq!(Theme::m3_request().seed(), 0xFF67_50A4);
        assert_eq!(Theme::fluent_request().seed(), 0xFF00_78D4);
        assert_eq!(Theme::apple_request().seed(), 0xFF00_7AFF);
        assert_eq!(Theme::omarchy_request().seed(), 0xFF7A_A2F7);
        assert_eq!(Theme::hyprland_request().seed(), 0xFF33_CCFF);
        assert_eq!(Theme::aphrody_request().seed(), 0xFF17_335C);
    }

    #[test]
    fn baseline_css_comes_from_canonical_tokens() {
        assert!(Theme::baseline_css().contains("--md-sys-color-primary:"));
    }
}
