// SPDX-License-Identifier: Apache-2.0
//! Applies Material Design 3 [`DynamicScheme`]s and all 5 [`YoloThemePreset`]s (`Neutral`, `M3`,
//! `Fluent`, `Apple`, `Aphrody`) to Iced 0.13 / 0.14 and System76 `libcosmic`:
//! - [`iced_palette`] and [`theme`] build an Iced `Palette` / `Theme` from any [`DynamicScheme`];
//! - [`theme_for_preset`] and [`Palette::from_preset`] build palettes for all 5 [`YoloThemePreset`]s;
//! - [`Palette`] caches all 49 M3 roles as [`Color`]s for `O(1)` lookup inside widget
//!   style closures;
//! - [`style`] provides M3 appearance structs for `button` (5 variants), `container` (cards,
//!   sheets, dialogs, sidebars), `text_input`, and `toggler`;
//! - [`cosmic`] produces [`cosmic::CosmicThemeBuilder`] values and `cosmic-config` RON key-value
//!   fragments so a `libcosmic` application or the COSMIC desktop reads the exact M3 / Yolo tokens.

use serde::{Deserialize, Serialize};
pub use yolo_shared::{
    BYRON_LOVE_PALETTE, BYRON_LOVE_PET_SPEC, ByronLoveColorTeinte, ByronLovePetSpec, RgbaColor,
    YoloManifest, YoloStyle, YoloThemePreset, YoloThemeTokens,
};
use yolo_tokens::elevation;
use yolo_tokens::presets::scheme_for_preset;
pub use yolo_tokens::role::Role;
pub use yolo_tokens::scheme::{DynamicScheme, Variant};
use yolo_tokens::shape::SCALE as SHAPE_SCALE;
use yolo_tokens::typography::{self, TypeToken};

pub mod cosmic;
pub mod style;

/// RGBA color in linear/sRGB `0.0..=1.0` floats matching `iced_core::Color`.
pub type Color = RgbaColor;

/// 2D vector matching `iced_core::Vector<f32>`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Vector {
    /// Horizontal offset.
    pub x: f32,
    /// Vertical offset.
    pub y: f32,
}

impl Vector {
    /// Zero vector.
    pub const ZERO: Self = Self { x: 0.0, y: 0.0 };

    /// Creates a new vector `(x, y)`.
    #[must_use]
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

/// Corner radii matching `iced_core::border::Radius`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Radius {
    /// Top-left corner radius in dp.
    pub top_left: f32,
    /// Top-right corner radius in dp.
    pub top_right: f32,
    /// Bottom-right corner radius in dp.
    pub bottom_right: f32,
    /// Bottom-left corner radius in dp.
    pub bottom_left: f32,
}

impl Radius {
    /// Uniform corner radius across all 4 corners.
    #[must_use]
    pub const fn new(r: f32) -> Self {
        Self {
            top_left: r,
            top_right: r,
            bottom_right: r,
            bottom_left: r,
        }
    }
}

impl From<f32> for Radius {
    fn from(r: f32) -> Self {
        Self::new(r)
    }
}

/// Widget border matching `iced_core::Border`.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Border {
    /// Border stroke color.
    pub color: Color,
    /// Border stroke width in dp.
    pub width: f32,
    /// Corner radii.
    pub radius: Radius,
}

impl Default for Border {
    fn default() -> Self {
        Self {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: Radius::new(0.0),
        }
    }
}

/// Drop shadow matching `iced_core::Shadow`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Shadow {
    /// Shadow color.
    pub color: Color,
    /// Shadow offset.
    pub offset: Vector,
    /// Shadow blur radius.
    pub blur_radius: f32,
}

/// Surface background fill matching `iced_core::Background`.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Background {
    /// Solid color fill.
    Color(Color),
}

/// Core 6-slot palette matching `iced_core::theme::Palette`.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct IcedPalette {
    /// Background color (`Role::Surface`).
    pub background: Color,
    /// Text color (`Role::OnSurface`).
    pub text: Color,
    /// Primary accent (`Role::Primary`).
    pub primary: Color,
    /// Success indicator (`Role::Secondary`).
    pub success: Color,
    /// Warning indicator (`Role::Tertiary`).
    pub warning: Color,
    /// Danger / error indicator (`Role::Error`).
    pub danger: Color,
}

/// Extended Iced theme wrapper carrying the name, 6-slot [`IcedPalette`], and full 49-role [`Palette`].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Theme {
    /// Theme name.
    pub name: String,
    /// Core 6-slot Iced palette.
    pub palette: IcedPalette,
    /// Full 49-role M3 / Yolo palette.
    pub roles: Palette,
}

impl Theme {
    /// Returns the 6-slot [`IcedPalette`].
    #[must_use]
    pub const fn palette(&self) -> IcedPalette {
        self.palette
    }
}

/// Converts `0xAARRGGBB` to [`Color`].
#[must_use]
pub const fn color(argb: u32) -> Color {
    Color::from_argb32(argb)
}

/// Returns `c` with its alpha replaced by `alpha` (`0.0..=1.0`).
#[must_use]
pub const fn with_alpha(c: Color, alpha: f32) -> Color {
    let a = alpha.clamp(0.0, 1.0);
    Color { a, ..c }
}

/// Composites `state` over `container` at `opacity` (the M3 state-layer formula).
#[must_use]
pub fn state_layer(container: Color, state: Color, opacity: f32) -> Color {
    container.mix(state, opacity.clamp(0.0, 1.0))
}

/// M3 shape tokens as [`Radius`].
pub mod shapes {
    use super::{Radius, SHAPE_SCALE};
    /// 0 dp.
    pub const NONE: Radius = Radius::new(SHAPE_SCALE.none);
    /// 4 dp.
    pub const EXTRA_SMALL: Radius = Radius::new(SHAPE_SCALE.extra_small);
    /// 8 dp.
    pub const SMALL: Radius = Radius::new(SHAPE_SCALE.small);
    /// 12 dp.
    pub const MEDIUM: Radius = Radius::new(SHAPE_SCALE.medium);
    /// 16 dp.
    pub const LARGE: Radius = Radius::new(SHAPE_SCALE.large);
    /// 28 dp.
    pub const EXTRA_LARGE: Radius = Radius::new(SHAPE_SCALE.extra_large);
    /// Pill (`9999 dp`).
    pub const FULL: Radius = Radius::new(SHAPE_SCALE.full);
}

/// M3 elevation level `0..=5` as a [`Shadow`].
#[must_use]
pub const fn elevation_shadow(level: u8, shadow_color: Color) -> Shadow {
    let dp = match level {
        0 => elevation::LEVEL_0.dp,
        1 => elevation::LEVEL_1.dp,
        2 => elevation::LEVEL_2.dp,
        3 => elevation::LEVEL_3.dp,
        4 => elevation::LEVEL_4.dp,
        _ => elevation::LEVEL_5.dp,
    };
    if dp == 0.0 {
        return Shadow {
            color: Color::TRANSPARENT,
            offset: Vector::ZERO,
            blur_radius: 0.0,
        };
    }
    Shadow {
        color: with_alpha(shadow_color, 0.28),
        offset: Vector::new(0.0, dp * 0.5),
        blur_radius: dp * 2.0,
    }
}

/// M3 15-step type scale sizes in sp/dp.
pub mod type_scale {
    use super::{TypeToken, typography};
    /// Font size in pixels/dp of a [`TypeToken`].
    #[must_use]
    pub const fn size(t: TypeToken) -> f32 {
        t.size_dp
    }
    /// Display Large (57 sp).
    pub const DISPLAY_LARGE: f32 = typography::DISPLAY_LARGE.size_dp;
    /// Display Medium (45 sp).
    pub const DISPLAY_MEDIUM: f32 = typography::DISPLAY_MEDIUM.size_dp;
    /// Display Small (36 sp).
    pub const DISPLAY_SMALL: f32 = typography::DISPLAY_SMALL.size_dp;
    /// Headline Large (32 sp).
    pub const HEADLINE_LARGE: f32 = typography::HEADLINE_LARGE.size_dp;
    /// Headline Medium (28 sp).
    pub const HEADLINE_MEDIUM: f32 = typography::HEADLINE_MEDIUM.size_dp;
    /// Headline Small (24 sp).
    pub const HEADLINE_SMALL: f32 = typography::HEADLINE_SMALL.size_dp;
    /// Title Large (22 sp).
    pub const TITLE_LARGE: f32 = typography::TITLE_LARGE.size_dp;
    /// Title Medium (16 sp).
    pub const TITLE_MEDIUM: f32 = typography::TITLE_MEDIUM.size_dp;
    /// Title Small (14 sp).
    pub const TITLE_SMALL: f32 = typography::TITLE_SMALL.size_dp;
    /// Body Large (16 sp).
    pub const BODY_LARGE: f32 = typography::BODY_LARGE.size_dp;
    /// Body Medium (14 sp).
    pub const BODY_MEDIUM: f32 = typography::BODY_MEDIUM.size_dp;
    /// Body Small (12 sp).
    pub const BODY_SMALL: f32 = typography::BODY_SMALL.size_dp;
    /// Label Large (14 sp).
    pub const LABEL_LARGE: f32 = typography::LABEL_LARGE.size_dp;
    /// Label Medium (12 sp).
    pub const LABEL_MEDIUM: f32 = typography::LABEL_MEDIUM.size_dp;
    /// Label Small (11 sp).
    pub const LABEL_SMALL: f32 = typography::LABEL_SMALL.size_dp;
}

/// All 49 resolved [`Role`] colours cached as [`Color`]s.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Palette {
    colors: Vec<Color>,
    /// True when the palette was built from a dark scheme.
    pub is_dark: bool,
    /// Optional [`YoloThemePreset`] this palette was built from.
    pub preset: Option<YoloThemePreset>,
}

impl Palette {
    /// Resolves every role from `scheme`.
    #[must_use]
    pub fn from_scheme(s: &DynamicScheme) -> Self {
        let mut colors = Vec::with_capacity(Role::ALL.len());
        for role in Role::ALL {
            colors.push(color(s.argb(role)));
        }
        Self {
            colors,
            is_dark: s.is_dark,
            preset: None,
        }
    }

    /// Resolves a palette for one of the 7 [`YoloThemePreset`]s.
    #[must_use]
    pub fn from_preset(preset: YoloThemePreset, dark: bool) -> Self {
        let scheme = scheme_for_preset(preset, dark);
        let mut pal = Self::from_scheme(&scheme);
        let tokens = preset.tokens(dark);
        pal.set(Role::Primary, tokens.primary_rgba());
        pal.set(Role::Secondary, tokens.secondary_rgba());
        pal.set(Role::Surface, tokens.surface_rgba());
        pal.set(Role::Background, tokens.surface_rgba());
        pal.set(Role::SurfaceContainer, tokens.surface_container_rgba());
        pal.set(Role::SurfaceContainerHigh, tokens.surface_elevated_rgba());
        pal.set(Role::OnSurface, tokens.on_surface_rgba());
        pal.set(Role::OnSurfaceVariant, tokens.on_surface_variant_rgba());
        pal.set(Role::OutlineVariant, tokens.outline_rgba());
        pal.preset = Some(preset);
        pal
    }

    /// Colour of `role`.
    #[must_use]
    pub fn get(&self, role: Role) -> Color {
        self.colors[role as usize]
    }

    /// Overrides the colour of `role`.
    pub fn set(&mut self, role: Role, c: Color) {
        self.colors[role as usize] = c;
    }

    /// Returns the button corner radius appropriate for this palette's preset.
    #[must_use]
    pub const fn button_radius(&self) -> Radius {
        match self.preset {
            Some(YoloThemePreset::Omarchy) => Radius::new(0.0),
            Some(YoloThemePreset::Fluent) => Radius::new(4.0),
            Some(YoloThemePreset::Apple | YoloThemePreset::Hyprland) => Radius::new(10.0),
            Some(YoloThemePreset::Neutral) => Radius::new(8.0),
            Some(YoloThemePreset::M3 | YoloThemePreset::Aphrody) | None => shapes::FULL,
            Some(YoloThemePreset::Chromium) => Radius::new(8.0),
        }
    }
}

/// Maps `scheme` to Iced's 6-slot [`IcedPalette`].
#[must_use]
pub fn iced_palette(s: &DynamicScheme) -> IcedPalette {
    IcedPalette {
        background: color(s.argb(Role::Surface)),
        text: color(s.argb(Role::OnSurface)),
        primary: color(s.argb(Role::Primary)),
        success: color(s.argb(Role::Secondary)),
        warning: color(s.argb(Role::Tertiary)),
        danger: color(s.argb(Role::Error)),
    }
}

/// Builds a custom [`Theme`] whose palette and extended palette come from `scheme`.
#[must_use]
pub fn theme(name: &str, s: &DynamicScheme) -> Theme {
    let p = Palette::from_scheme(s);
    Theme {
        name: name.to_owned(),
        palette: iced_palette(s),
        roles: p,
    }
}

/// Builds a [`Theme`] for one of the 7 [`YoloThemePreset`]s.
#[must_use]
pub fn theme_for_preset(preset: YoloThemePreset, dark: bool) -> Theme {
    let roles = Palette::from_preset(preset, dark);
    Theme {
        name: format!("{}-{}", preset.id(), if dark { "dark" } else { "light" }),
        palette: IcedPalette {
            background: roles.get(Role::Surface),
            text: roles.get(Role::OnSurface),
            primary: roles.get(Role::Primary),
            success: roles.get(Role::Secondary),
            warning: roles.get(Role::Tertiary),
            danger: roles.get(Role::Error),
        },
        roles,
    }
}

/// Generates the `cosmic-theme` RON string (Light + Dark) for one of the 5 [`YoloThemePreset`]s.
#[must_use]
pub fn cosmic_ron_for_preset(preset: YoloThemePreset) -> String {
    let light = cosmic::CosmicThemeBuilder::for_preset(preset, false);
    let dark = cosmic::CosmicThemeBuilder::for_preset(preset, true);
    format!("{}{}", light.to_ron_string(), dark.to_ron_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theme_and_styles_populate_from_scheme() {
        let s = DynamicScheme::from_argb(0xFF67_50A4, Variant::TonalSpot, true, 0.0);
        let p = Palette::from_scheme(&s);
        let t = theme("Yolo M3 Dark", &s);
        assert_eq!(t.palette().primary, p.get(Role::Primary));
        assert_eq!(t.palette().background, p.get(Role::Surface));

        let b = style::button(&p, style::ButtonKind::Filled, style::Status::Active);
        assert_eq!(b.text_color, p.get(Role::OnPrimary));
        let bh = style::button(&p, style::ButtonKind::Filled, style::Status::Hovered);
        assert_ne!(b.background, bh.background);

        let c = style::container(&p, style::SurfaceKind::Dialog);
        assert_eq!(c.border.radius, shapes::EXTRA_LARGE);

        let cb = cosmic::CosmicThemeBuilder::from_scheme(&s);
        assert!(cb.is_dark);
        assert_eq!(cb.accent, p.get(Role::Primary));
        assert_eq!(cb.to_ron_entries().len(), 13);
    }

    #[test]
    fn all_five_presets_produce_cosmic_themes_and_ron() {
        for preset in YoloThemePreset::ALL {
            let dark_theme = theme_for_preset(preset, true);
            let light_theme = theme_for_preset(preset, false);
            assert!(dark_theme.roles.is_dark);
            assert!(!light_theme.roles.is_dark);
            assert_eq!(dark_theme.roles.preset, Some(preset));

            let builder = cosmic::CosmicThemeBuilder::for_preset(preset, true);
            assert_eq!(builder.preset, Some(preset));
            let ron = cosmic_ron_for_preset(preset);
            assert!(ron.contains("com.system76.CosmicTheme.Light.Builder/v2/accent"));
            assert!(ron.contains("com.system76.CosmicTheme.Dark.Builder/v2/accent"));
        }

        // Verify Fluent 4px button radius and Apple 10px button radius
        let fluent_pal = Palette::from_preset(YoloThemePreset::Fluent, true);
        assert_eq!(fluent_pal.button_radius(), Radius::new(4.0));
        let apple_pal = Palette::from_preset(YoloThemePreset::Apple, true);
        assert_eq!(apple_pal.button_radius(), Radius::new(10.0));
    }
}
