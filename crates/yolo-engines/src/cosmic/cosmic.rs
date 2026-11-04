// SPDX-License-Identifier: Apache-2.0
//! Bridge to `libcosmic` / `cosmic-theme` (`ThemeBuilder`, `CornerRadii`, `Spacing`) without
//! linking `libcosmic` or its Wayland/sctk dependency tree.
//!
//! `libcosmic`'s theme engine (`cosmic-theme`) derives every surface, container and component
//! state from 9 builder seeds plus corner radii and an 8-step spacing scale, stored either in
//! memory via `cosmic::theme::ThemeBuilder` or on disk under
//! `~/.config/cosmic/com.system76.CosmicTheme.{Light,Dark}.Builder/v2/` as RON files.
//!
//! This module gives both:
//! - [`CosmicThemeBuilder`], [`CosmicCornerRadii`] and [`CosmicSpacing`] with the exact field
//!   layout of `cosmic_theme::ThemeBuilder` so a `libcosmic` app can copy the fields straight into
//!   `cosmic::theme::ThemeBuilder::dark()` / `::light()`;
//! - [`CosmicThemeBuilder::to_ron_entries`] to write a complete cosmic-config directory for
//!   any [`DynamicScheme`] or any of the 5 [`YoloThemePreset`]s (`Neutral`, `M3`, `Fluent`, `Apple`, `Aphrody`).

use serde::{Deserialize, Serialize};
use yolo_shared::YoloThemePreset;
use yolo_tokens::presets::scheme_for_preset;
use yolo_tokens::role::Role;
use yolo_tokens::scheme::DynamicScheme;
use yolo_tokens::shape::SCALE as SHAPE;

use super::{color, Color};

/// `cosmic_theme::CornerRadii` filled from the M3 shape scale or a [`YoloThemePreset`].
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct CosmicCornerRadii {
    /// `[0.0; 4]`.
    pub radius_0: [f32; 4],
    /// Extra-small (`4 dp`).
    pub radius_xs: [f32; 4],
    /// Small (`8 dp`).
    pub radius_s: [f32; 4],
    /// Medium (`12 dp`).
    pub radius_m: [f32; 4],
    /// Large (`16 dp`).
    pub radius_l: [f32; 4],
    /// Extra-large (`28 dp`).
    pub radius_xl: [f32; 4],
}

impl CosmicCornerRadii {
    /// Canonical M3 corner radii.
    pub const M3: Self = Self {
        radius_0: [SHAPE.none; 4],
        radius_xs: [SHAPE.extra_small; 4],
        radius_s: [SHAPE.small; 4],
        radius_m: [SHAPE.medium; 4],
        radius_l: [SHAPE.large; 4],
        radius_xl: [SHAPE.extra_large; 4],
    };

    /// Returns the corner radii profile for one of the 7 [`YoloThemePreset`]s.
    #[must_use]
    pub const fn for_preset(preset: YoloThemePreset) -> Self {
        match preset {
            YoloThemePreset::Omarchy => Self {
                radius_0: [0.0; 4],
                radius_xs: [0.0; 4],
                radius_s: [0.0; 4],
                radius_m: [2.0; 4],
                radius_l: [2.0; 4],
                radius_xl: [4.0; 4],
            },
            YoloThemePreset::Fluent => Self {
                radius_0: [0.0; 4],
                radius_xs: [2.0; 4],
                radius_s: [4.0; 4],
                radius_m: [6.0; 4],
                radius_l: [8.0; 4],
                radius_xl: [12.0; 4],
            },
            YoloThemePreset::Apple => Self {
                radius_0: [0.0; 4],
                radius_xs: [6.0; 4],
                radius_s: [10.0; 4],
                radius_m: [14.0; 4],
                radius_l: [18.0; 4],
                radius_xl: [24.0; 4],
            },
            YoloThemePreset::Hyprland => Self {
                radius_0: [0.0; 4],
                radius_xs: [6.0; 4],
                radius_s: [10.0; 4],
                radius_m: [12.0; 4],
                radius_l: [16.0; 4],
                radius_xl: [20.0; 4],
            },
            YoloThemePreset::Neutral | YoloThemePreset::M3 | YoloThemePreset::Aphrody | YoloThemePreset::Chromium => Self::M3,
        }
    }
}

impl Default for CosmicCornerRadii {
    fn default() -> Self {
        Self::M3
    }
}

/// `cosmic_theme::Spacing` on the M3 4 dp grid.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CosmicSpacing {
    /// 0 px.
    pub space_none: u16,
    /// 4 px.
    pub space_xxxs: u16,
    /// 8 px.
    pub space_xxs: u16,
    /// 12 px.
    pub space_xs: u16,
    /// 16 px.
    pub space_s: u16,
    /// 24 px.
    pub space_m: u16,
    /// 32 px.
    pub space_l: u16,
    /// 48 px.
    pub space_xl: u16,
    /// 64 px.
    pub space_xxl: u16,
    /// 128 px.
    pub space_xxxl: u16,
}

impl CosmicSpacing {
    /// Canonical M3 4 dp spacing grid.
    pub const M3: Self = Self {
        space_none: 0,
        space_xxxs: 4,
        space_xxs: 8,
        space_xs: 12,
        space_s: 16,
        space_m: 24,
        space_l: 32,
        space_xl: 48,
        space_xxl: 64,
        space_xxxl: 128,
    };
}

impl Default for CosmicSpacing {
    fn default() -> Self {
        Self::M3
    }
}

/// All seeds `cosmic_theme::ThemeBuilder` needs to build a `cosmic::Theme`.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct CosmicThemeBuilder {
    /// Dark mode flag.
    pub is_dark: bool,
    /// Optional [`YoloThemePreset`] this builder was created from.
    pub preset: Option<YoloThemePreset>,
    /// `accent` (`Role::Primary`).
    pub accent: Color,
    /// `bg_color` (`Role::Surface`).
    pub bg_color: Color,
    /// `primary_container_bg` (`Role::SurfaceContainer`).
    pub primary_container_bg: Color,
    /// `secondary_container_bg` (`Role::SurfaceContainerHigh`).
    pub secondary_container_bg: Color,
    /// `text_tint` (`Role::OnSurface`).
    pub text_tint: Color,
    /// `neutral_tint` (`Role::SurfaceTint`).
    pub neutral_tint: Color,
    /// `destructive` (`Role::Error`).
    pub destructive: Color,
    /// `success` (`Role::Secondary`).
    pub success: Color,
    /// `warning` (`Role::Tertiary`).
    pub warning: Color,
    /// Shape scale.
    pub corner_radii: CosmicCornerRadii,
    /// Spacing grid.
    pub spacing: CosmicSpacing,
    /// Window gap (`active_hint` in cosmic-theme), in dp.
    pub active_hint: u32,
}

impl CosmicThemeBuilder {
    /// Populates every `ThemeBuilder` seed from `scheme`.
    #[must_use]
    pub fn from_scheme(s: &DynamicScheme) -> Self {
        let c = |r| color(s.argb(r));
        Self {
            is_dark: s.is_dark,
            preset: None,
            accent: c(Role::Primary),
            bg_color: c(Role::Surface),
            primary_container_bg: c(Role::SurfaceContainer),
            secondary_container_bg: c(Role::SurfaceContainerHigh),
            text_tint: c(Role::OnSurface),
            neutral_tint: c(Role::SurfaceTint),
            destructive: c(Role::Error),
            success: c(Role::Secondary),
            warning: c(Role::Tertiary),
            corner_radii: CosmicCornerRadii::M3,
            spacing: CosmicSpacing::M3,
            active_hint: 2,
        }
    }

    /// Populates a `CosmicThemeBuilder` for any of the 7 [`YoloThemePreset`]s.
    #[must_use]
    pub fn for_preset(preset: YoloThemePreset, dark: bool) -> Self {
        let scheme = scheme_for_preset(preset, dark);
        let tokens = preset.tokens(dark);
        let mut builder = Self::from_scheme(&scheme);
        builder.preset = Some(preset);
        builder.accent = tokens.primary_rgba();
        builder.bg_color = tokens.surface_rgba();
        builder.primary_container_bg = tokens.surface_container_rgba();
        builder.secondary_container_bg = tokens.surface_elevated_rgba();
        builder.text_tint = tokens.on_surface_rgba();
        builder.corner_radii = CosmicCornerRadii::for_preset(preset);
        builder
    }

    /// Returns `(key, ron_value)` pairs for
    /// `com.system76.CosmicTheme.{Light,Dark}.Builder/v2/<key>`.
    #[must_use]
    pub fn to_ron_entries(&self) -> Vec<(&'static str, String)> {
        let rgb = |c: Color| format!("Some((red: {:.6}, green: {:.6}, blue: {:.6}))", c.r, c.g, c.b);
        let rgba = |c: Color| {
            format!(
                "Some((red: {:.6}, green: {:.6}, blue: {:.6}, alpha: {:.4}))",
                c.r, c.g, c.b, c.a
            )
        };
        let r = &self.corner_radii;
        let sp = &self.spacing;
        vec![
            ("is_dark", self.is_dark.to_string()),
            ("accent", rgb(self.accent)),
            ("bg_color", rgba(self.bg_color)),
            ("primary_container_bg", rgba(self.primary_container_bg)),
            ("secondary_container_bg", rgba(self.secondary_container_bg)),
            ("text_tint", rgb(self.text_tint)),
            ("neutral_tint", rgb(self.neutral_tint)),
            ("destructive", rgb(self.destructive)),
            ("success", rgb(self.success)),
            ("warning", rgb(self.warning)),
            ("active_hint", self.active_hint.to_string()),
            (
                "corner_radii",
                format!(
                    "(radius_0: {:?}, radius_xs: {:?}, radius_s: {:?}, radius_m: {:?}, radius_l: {:?}, radius_xl: {:?})",
                    r.radius_0, r.radius_xs, r.radius_s, r.radius_m, r.radius_l, r.radius_xl
                ),
            ),
            (
                "spacing",
                format!(
                    "(space_none: {}, space_xxxs: {}, space_xxs: {}, space_xs: {}, space_s: {}, space_m: {}, space_l: {}, space_xl: {}, space_xxl: {}, space_xxxl: {})",
                    sp.space_none,
                    sp.space_xxxs,
                    sp.space_xxs,
                    sp.space_xs,
                    sp.space_s,
                    sp.space_m,
                    sp.space_l,
                    sp.space_xl,
                    sp.space_xxl,
                    sp.space_xxxl,
                ),
            ),
        ]
    }

    /// Formats a full multi-line `cosmic-config` RON file for this builder.
    #[must_use]
    pub fn to_ron_string(&self) -> String {
        let mode = if self.is_dark { "Dark" } else { "Light" };
        let mut out = String::new();
        for (key, value) in self.to_ron_entries() {
            out.push_str(&format!(
                "com.system76.CosmicTheme.{mode}.Builder/v2/{key} = {value}\n"
            ));
        }
        out
    }
}
