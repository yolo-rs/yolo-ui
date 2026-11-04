// SPDX-License-Identifier: Apache-2.0
//! 7-Preset Universal Theme Bridge for React 19 Web CSS + React Native `StyleSheet` / `PlatformColor`.
//!
//! Bridges `yolo-shared` (`YoloThemePreset`, `OmarchySubTheme`, `HYPRLAND_DECORATION_SPEC`,
//! `BYRON_LOVE_PALETTE`) and `yolo-tokens` (`scheme_for_preset`, `Role`) into:
//! 1. Web CSS `:root` custom property sheets (`--yolo-*` + `--md-sys-color-*`).
//! 2. React Native / Expo `StyleSheet` JSON payloads for zero-drift cross-platform styling.

use serde::{Deserialize, Serialize};
use yolo_shared::{
    BYRON_LOVE_PALETTE, HYPRLAND_DECORATION_SPEC, OmarchySubTheme, YoloStyle, YoloThemePreset,
    YoloThemeTokens, css_preset_variables,
};
use yolo_tokens::presets::scheme_for_preset;
use yolo_tokens::role::Role;

/// Typed React Native / Expo theme payload serialized for JSI / Metro / Server-Driven UI.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReactNativeThemePayload {
    /// Canonical preset id (`"yolo-neutral"`, `"yolo-m3"`, `"yolo-fluent"`, `"yolo-apple"`, `"yolo-omarchy"`, `"yolo-hyprland"`, `"yolo-aphrody"`).
    pub preset_id: String,
    /// Human-readable display name.
    pub display_name: String,
    /// Dark mode flag.
    pub dark: bool,
    /// Primary hex seed (`"#RRGGBB"`).
    pub seed_hex: String,
    /// Resolved surface, text, accent, and border tokens.
    pub tokens: ReactNativeColorTokens,
    /// Dynamic HCT Material 3 color roles from `yolo-tokens`.
    pub hct_roles: ReactNativeHctRoles,
    /// Structural radii, spacing grid, typography, and window vibrancy.
    pub geometry: ReactNativeGeometry,
    /// Optional Omarchy subtheme identifier when using `YoloThemePreset::Omarchy`.
    pub omarchy_subtheme: Option<String>,
    /// Optional Hyprland active border gradient spec when using `YoloThemePreset::Hyprland`.
    pub hyprland_border_gradient: Option<String>,
    /// Optional Byron Love 10-teinte hex palette when using `YoloThemePreset::Aphrody`.
    pub byron_love_palette: Option<Vec<String>>,
}

/// Core color roles formatted for React Native `StyleSheet.create`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReactNativeColorTokens {
    pub bg_primary: String,
    pub bg_surface: String,
    pub bg_elevated: String,
    pub text_primary: String,
    pub text_secondary: String,
    pub accent_primary: String,
    pub accent_secondary: String,
    pub border_subtle: String,
}

/// HCT dynamic scheme roles computed by `yolo-tokens`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReactNativeHctRoles {
    pub primary: String,
    pub on_primary: String,
    pub primary_container: String,
    pub secondary: String,
    pub surface: String,
    pub on_surface: String,
    pub outline_variant: String,
}

/// Structural geometry & typography values for React Native `StyleSheet`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReactNativeGeometry {
    pub radius_sm: f32,
    pub radius_md: f32,
    pub radius_lg: f32,
    pub radius_pill: f32,
    pub spacing_grid: f32,
    pub border_width: f32,
    pub backdrop_blur: f32,
    pub font_family_ui: String,
    pub font_family_mono: String,
    pub window_vibrancy: String,
}

fn argb_to_hex(argb: u32) -> String {
    let r = (argb >> 16) & 0xFF;
    let g = (argb >> 8) & 0xFF;
    let b = argb & 0xFF;
    format!("#{r:02x}{g:02x}{b:02x}")
}

/// Exports a complete React 19 Web CSS `:root` sheet combining `yolo-shared` preset tokens
/// (`--yolo-*`) and `yolo-tokens` HCT dynamic roles (`--md-sys-color-*`).
pub fn export_react_web_css(preset: YoloThemePreset, dark: bool) -> String {
    let base = css_preset_variables(preset, dark);
    let scheme = scheme_for_preset(preset, dark);

    let hct_block = format!(
        ":root {{\n  \
         --md-sys-color-primary: {};\n  \
         --md-sys-color-on-primary: {};\n  \
         --md-sys-color-primary-container: {};\n  \
         --md-sys-color-secondary: {};\n  \
         --md-sys-color-surface: {};\n  \
         --md-sys-color-on-surface: {};\n  \
         --md-sys-color-outline-variant: {};\n\
         }}",
        argb_to_hex(scheme.argb(Role::Primary)),
        argb_to_hex(scheme.argb(Role::OnPrimary)),
        argb_to_hex(scheme.argb(Role::PrimaryContainer)),
        argb_to_hex(scheme.argb(Role::Secondary)),
        argb_to_hex(scheme.argb(Role::Surface)),
        argb_to_hex(scheme.argb(Role::OnSurface)),
        argb_to_hex(scheme.argb(Role::OutlineVariant)),
    );

    format!("{base}\n{hct_block}\n")
}

/// Builds a [`ReactNativeThemePayload`] for any of the 7 [`YoloThemePreset`]s (and optional [`OmarchySubTheme`]).
pub fn build_react_native_theme(
    preset: YoloThemePreset,
    dark: bool,
    omarchy_subtheme: Option<OmarchySubTheme>,
) -> ReactNativeThemePayload {
    let tokens: YoloThemeTokens = if let Some(sub) = omarchy_subtheme {
        sub.tokens()
    } else {
        preset.tokens(dark)
    };
    let style: YoloStyle = preset.style();
    let scheme = scheme_for_preset(preset, dark);

    let hyprland_border_gradient = if preset == YoloThemePreset::Hyprland {
        Some(format!(
            "linear-gradient({}deg, {}, {})",
            HYPRLAND_DECORATION_SPEC.active_border_angle_deg,
            HYPRLAND_DECORATION_SPEC.active_border_start_hex,
            HYPRLAND_DECORATION_SPEC.active_border_end_hex
        ))
    } else {
        None
    };

    let byron_love_palette = if preset == YoloThemePreset::Aphrody {
        Some(
            BYRON_LOVE_PALETTE
                .iter()
                .map(|c| c.hex.to_string())
                .collect(),
        )
    } else {
        None
    };

    ReactNativeThemePayload {
        preset_id: preset.id().to_string(),
        display_name: preset.display_name().to_string(),
        dark,
        seed_hex: preset.seed_hex().to_string(),
        tokens: ReactNativeColorTokens {
            bg_primary: tokens.bg_primary.to_string(),
            bg_surface: tokens.bg_surface.to_string(),
            bg_elevated: tokens.bg_elevated.to_string(),
            text_primary: tokens.text_primary.to_string(),
            text_secondary: tokens.text_secondary.to_string(),
            accent_primary: tokens.accent_primary.to_string(),
            accent_secondary: tokens.accent_secondary.to_string(),
            border_subtle: tokens.border_subtle.to_string(),
        },
        hct_roles: ReactNativeHctRoles {
            primary: argb_to_hex(scheme.argb(Role::Primary)),
            on_primary: argb_to_hex(scheme.argb(Role::OnPrimary)),
            primary_container: argb_to_hex(scheme.argb(Role::PrimaryContainer)),
            secondary: argb_to_hex(scheme.argb(Role::Secondary)),
            surface: argb_to_hex(scheme.argb(Role::Surface)),
            on_surface: argb_to_hex(scheme.argb(Role::OnSurface)),
            outline_variant: argb_to_hex(scheme.argb(Role::OutlineVariant)),
        },
        geometry: ReactNativeGeometry {
            radius_sm: style.radius_sm_px,
            radius_md: style.radius_md_px,
            radius_lg: style.radius_lg_px,
            radius_pill: style.radius_pill_px,
            spacing_grid: style.spacing_grid_px,
            border_width: style.border_width_px,
            backdrop_blur: style.backdrop_blur_px,
            font_family_ui: style.font_family_ui.to_string(),
            font_family_mono: style.font_family_mono.to_string(),
            window_vibrancy: style.window_vibrancy.to_string(),
        },
        omarchy_subtheme: omarchy_subtheme.map(|s| s.spec().id.to_string()),
        hyprland_border_gradient,
        byron_love_palette,
    }
}

/// Serializes the [`ReactNativeThemePayload`] for `preset` as pretty-printed JSON.
pub fn export_react_native_theme_json(
    preset: YoloThemePreset,
    dark: bool,
    omarchy_subtheme: Option<OmarchySubTheme>,
) -> String {
    let payload = build_react_native_theme(preset, dark, omarchy_subtheme);
    serde_json::to_string_pretty(&payload).unwrap_or_default()
}
