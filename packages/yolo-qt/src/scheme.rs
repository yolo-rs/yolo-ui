// SPDX-License-Identifier: Apache-2.0
//! Pure-Rust CxxQt bridge descriptor (`YoloQtScheme`, `CxxQtBridgeDescriptor`) and optional
//! `#[cxx_qt::bridge]` `M3Scheme` `QObject` (when `system-qt6` is enabled).

use serde::{Deserialize, Serialize};
use yolo_emit::hex;
use yolo_shared::YoloThemePreset;
use yolo_tokens::presets::variant_for_preset;
use yolo_tokens::role::Role;
use yolo_tokens::scheme::{DynamicScheme, Variant};
use yolo_tokens::shape::SCALE as SHAPE;

use crate::palette::palette_for_preset;

/// Pure-Rust representation of the `M3Scheme` / `YoloQtScheme` QML `QObject` singleton properties
/// and invokables, usable on any host without requiring system `qmake`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct YoloQtScheme {
    /// Active [`YoloThemePreset`] (if any).
    pub preset: Option<YoloThemePreset>,
    /// Source seed (`0xAARRGGBB`).
    pub seed: u32,
    /// M3 scheme variant index (`0..=8`).
    pub variant: i32,
    /// Dark mode flag.
    pub dark: bool,
    /// Contrast level (`-1.0..=1.0`).
    pub contrast: f64,
    /// Primary role (`#rrggbb`).
    pub primary: String,
    /// On-primary role (`#rrggbb`).
    pub on_primary: String,
    /// Primary container (`#rrggbb`).
    pub primary_container: String,
    /// On-primary container (`#rrggbb`).
    pub on_primary_container: String,
    /// Secondary role (`#rrggbb`).
    pub secondary: String,
    /// On-secondary role (`#rrggbb`).
    pub on_secondary: String,
    /// Secondary container (`#rrggbb`).
    pub secondary_container: String,
    /// On-secondary container (`#rrggbb`).
    pub on_secondary_container: String,
    /// Tertiary role (`#rrggbb`).
    pub tertiary: String,
    /// On-tertiary role (`#rrggbb`).
    pub on_tertiary: String,
    /// Tertiary container (`#rrggbb`).
    pub tertiary_container: String,
    /// On-tertiary container (`#rrggbb`).
    pub on_tertiary_container: String,
    /// Error role (`#rrggbb`).
    pub error: String,
    /// On-error role (`#rrggbb`).
    pub on_error: String,
    /// Error container (`#rrggbb`).
    pub error_container: String,
    /// On-error container (`#rrggbb`).
    pub on_error_container: String,
    /// Surface role (`#rrggbb`).
    pub surface: String,
    /// On-surface role (`#rrggbb`).
    pub on_surface: String,
    /// On-surface variant (`#rrggbb`).
    pub on_surface_variant: String,
    /// Surface container lowest (`#rrggbb`).
    pub surface_container_lowest: String,
    /// Surface container low (`#rrggbb`).
    pub surface_container_low: String,
    /// Surface container (`#rrggbb`).
    pub surface_container: String,
    /// Surface container high (`#rrggbb`).
    pub surface_container_high: String,
    /// Surface container highest (`#rrggbb`).
    pub surface_container_highest: String,
    /// Outline (`#rrggbb`).
    pub outline: String,
    /// Outline variant (`#rrggbb`).
    pub outline_variant: String,
    /// Inverse surface (`#rrggbb`).
    pub inverse_surface: String,
    /// Inverse on-surface (`#rrggbb`).
    pub inverse_on_surface: String,
    /// Inverse primary (`#rrggbb`).
    pub inverse_primary: String,
    /// Shadow (`#rrggbb`).
    pub shadow: String,
    /// Scrim (`#rrggbb`).
    pub scrim: String,
    /// Extra-small shape radius (`4 dp`).
    pub radius_extra_small: i32,
    /// Small shape radius (`8 dp`).
    pub radius_small: i32,
    /// Medium shape radius (`12 dp`).
    pub radius_medium: i32,
    /// Large shape radius (`16 dp`).
    pub radius_large: i32,
    /// Extra-large shape radius (`28 dp`).
    pub radius_extra_large: i32,
}

impl Default for YoloQtScheme {
    fn default() -> Self {
        Self::for_preset(YoloThemePreset::M3, true)
    }
}

impl YoloQtScheme {
    /// Creates a [`YoloQtScheme`] from `(seed, variant_index, dark, contrast)`.
    #[must_use]
    pub fn new(seed: u32, variant: i32, dark: bool, contrast: f64) -> Self {
        let s = DynamicScheme::from_argb(seed, variant_from_index(variant), dark, contrast);
        let c = |r: Role| hex(s.argb(r));
        Self {
            preset: None,
            seed,
            variant,
            dark,
            contrast,
            primary: c(Role::Primary),
            on_primary: c(Role::OnPrimary),
            primary_container: c(Role::PrimaryContainer),
            on_primary_container: c(Role::OnPrimaryContainer),
            secondary: c(Role::Secondary),
            on_secondary: c(Role::OnSecondary),
            secondary_container: c(Role::SecondaryContainer),
            on_secondary_container: c(Role::OnSecondaryContainer),
            tertiary: c(Role::Tertiary),
            on_tertiary: c(Role::OnTertiary),
            tertiary_container: c(Role::TertiaryContainer),
            on_tertiary_container: c(Role::OnTertiaryContainer),
            error: c(Role::Error),
            on_error: c(Role::OnError),
            error_container: c(Role::ErrorContainer),
            on_error_container: c(Role::OnErrorContainer),
            surface: c(Role::Surface),
            on_surface: c(Role::OnSurface),
            on_surface_variant: c(Role::OnSurfaceVariant),
            surface_container_lowest: c(Role::SurfaceContainerLowest),
            surface_container_low: c(Role::SurfaceContainerLow),
            surface_container: c(Role::SurfaceContainer),
            surface_container_high: c(Role::SurfaceContainerHigh),
            surface_container_highest: c(Role::SurfaceContainerHighest),
            outline: c(Role::Outline),
            outline_variant: c(Role::OutlineVariant),
            inverse_surface: c(Role::InverseSurface),
            inverse_on_surface: c(Role::InverseOnSurface),
            inverse_primary: c(Role::InversePrimary),
            shadow: c(Role::Shadow),
            scrim: c(Role::Scrim),
            radius_extra_small: SHAPE.extra_small as i32,
            radius_small: SHAPE.small as i32,
            radius_medium: SHAPE.medium as i32,
            radius_large: SHAPE.large as i32,
            radius_extra_large: SHAPE.extra_large as i32,
        }
    }

    /// Creates a [`YoloQtScheme`] for one of the 7 [`YoloThemePreset`]s.
    #[must_use]
    pub fn for_preset(preset: YoloThemePreset, dark: bool) -> Self {
        let v_idx = variant_to_index(variant_for_preset(preset));
        let mut scheme = Self::new(preset.seed_argb(), v_idx, dark, 0.0);
        let tokens = preset.tokens(dark);
        scheme.preset = Some(preset);
        scheme.primary = tokens.accent_primary.to_owned();
        scheme.secondary = tokens.accent_secondary.to_owned();
        scheme.surface = tokens.bg_primary.to_owned();
        scheme.surface_container = tokens.bg_surface.to_owned();
        scheme.surface_container_high = tokens.bg_elevated.to_owned();
        scheme.on_surface = tokens.text_primary.to_owned();
        scheme.on_surface_variant = tokens.text_secondary.to_owned();
        scheme.outline_variant = tokens.border_subtle.to_owned();
        match preset {
            YoloThemePreset::Omarchy => {
                scheme.radius_extra_small = 0;
                scheme.radius_small = 0;
                scheme.radius_medium = 2;
                scheme.radius_large = 4;
            }
            YoloThemePreset::Fluent => {
                scheme.radius_small = 4;
                scheme.radius_medium = 8;
            }
            YoloThemePreset::Apple => {
                scheme.radius_small = 10;
                scheme.radius_medium = 14;
            }
            YoloThemePreset::Hyprland => {
                scheme.radius_small = 10;
                scheme.radius_medium = 12;
            }
            _ => {}
        }
        scheme
    }

    /// Look up any of the 49 M3 roles by kebab-case (`"on-surface-variant"`) or camelCase
    /// (`"onSurfaceVariant"`) name.
    #[must_use]
    pub fn role_hex(&self, name: &str) -> String {
        let s = DynamicScheme::from_argb(
            self.seed,
            variant_from_index(self.variant),
            self.dark,
            self.contrast,
        );
        for r in Role::ALL {
            if r.name() == name || r.camel_name() == name {
                return hex(s.argb(r));
            }
        }
        String::new()
    }

    /// Look up any of the 21 `QPalette` role names (`"Window"`, `"Highlight"`, `"Accent"`, ...)
    /// and return `#rrggbb`.
    #[must_use]
    pub fn qpalette_hex(&self, qpalette_role: &str) -> String {
        if let Some(preset) = self.preset {
            for (name, _, argb) in palette_for_preset(preset, self.dark) {
                if name == qpalette_role {
                    return hex(argb);
                }
            }
        }
        let s = DynamicScheme::from_argb(
            self.seed,
            variant_from_index(self.variant),
            self.dark,
            self.contrast,
        );
        for &(name, _, role) in crate::palette::PALETTE_ROLES {
            if name == qpalette_role {
                return hex(s.argb(role));
            }
        }
        String::new()
    }
}

/// Metadata descriptor for the CxxQt QML module bridge (`com.yolo.ui` / `M3Scheme`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CxxQtBridgeDescriptor {
    /// QML module URI (`"com.yolo.ui"`).
    pub qml_uri: &'static str,
    /// QML major.minor version (`(1, 0)`).
    pub qml_version: (u8, u8),
    /// QML singleton type name (`"M3Scheme"`).
    pub qobject_name: &'static str,
    /// Exposed `#[qproperty]` names.
    pub properties: &'static [&'static str],
    /// Exposed `#[qinvokable]` method signatures.
    pub invokables: &'static [&'static str],
}

impl CxxQtBridgeDescriptor {
    /// Canonical `M3Scheme` bridge descriptor.
    pub const CANONICAL: Self = Self {
        qml_uri: "com.yolo.ui",
        qml_version: (1, 0),
        qobject_name: "M3Scheme",
        properties: &[
            "seed",
            "variant",
            "dark",
            "contrast",
            "primary",
            "on_primary",
            "primary_container",
            "on_primary_container",
            "secondary",
            "on_secondary",
            "secondary_container",
            "on_secondary_container",
            "tertiary",
            "on_tertiary",
            "tertiary_container",
            "on_tertiary_container",
            "error",
            "on_error",
            "error_container",
            "on_error_container",
            "surface",
            "on_surface",
            "on_surface_variant",
            "surface_container_lowest",
            "surface_container_low",
            "surface_container",
            "surface_container_high",
            "surface_container_highest",
            "outline",
            "outline_variant",
            "inverse_surface",
            "inverse_on_surface",
            "inverse_primary",
            "shadow",
            "scrim",
            "radius_extra_small",
            "radius_small",
            "radius_medium",
            "radius_large",
            "radius_extra_large",
        ],
        invokables: &[
            "rebuild(seed: i64, variant: i32, dark: bool, contrast: f64)",
            "role_color(name: &QString) -> QColor",
            "qpalette_color(qpalette_role: &QString) -> QColor",
        ],
    };
}

const fn variant_from_index(v: i32) -> Variant {
    match v {
        1 => Variant::Content,
        2 => Variant::Fidelity,
        3 => Variant::Vibrant,
        4 => Variant::Expressive,
        5 => Variant::Neutral,
        6 => Variant::Monochrome,
        7 => Variant::Rainbow,
        8 => Variant::FruitSalad,
        _ => Variant::TonalSpot,
    }
}

const fn variant_to_index(v: Variant) -> i32 {
    match v {
        Variant::TonalSpot => 0,
        Variant::Content => 1,
        Variant::Fidelity => 2,
        Variant::Vibrant => 3,
        Variant::Expressive => 4,
        Variant::Neutral => 5,
        Variant::Monochrome => 6,
        Variant::Rainbow => 7,
        Variant::FruitSalad => 8,
    }
}
