// SPDX-License-Identifier: Apache-2.0
//! Multi-preset theme emitters for every Yolo UI host.
//!
//! `yolo-tokens` resolves the scheme (HCT, dynamic color, contrast) and `yolo-shared` provides
//! the 8 first-class `YoloThemePreset`s (`Neutral`, `M3`, `Fluent`, `Apple`, `Omarchy`, `Hyprland`, `Aphrody`, `Chromium`).
//! This crate maps the resolved roles and preset tokens to each host's native theme format,
//! following `catalog/conversion.toml`. Every color in every output is a role resolved by
//! `yolo-tokens` (or a color derived from the scheme's palettes in [`extended`]).
//!
//! Single-file consumers call [`emit`] / [`emit_preset`]; multi-file targets (Chrome theme
//! directories, Xcode asset catalogs, Omarchy theme directories, GTK per-mode files) are
//! produced by [`emit_files`] / [`emit_preset_files`].

use std::fmt::Write;

pub mod dioxus_components;
pub use dioxus_components::emit_dioxus_components_theme;
pub mod gpui_theme;
pub mod tailwind;
pub use gpui_theme::emit_gpui_theme;
pub use tailwind::emit_tailwind_theme;
pub use yolo_shared::{
    BYRON_LOVE_PALETTE, BYRON_LOVE_PET_SPEC, ByronLovePetSpec, HYPRLAND_DECORATION_SPEC,
    HyprlandDecorationSpec, OMARCHY_THEMES, OmarchySubTheme, OmarchyThemePalette,
    YOLO_HYPRLAND_DARK_THEME, YoloManifest, YoloStyle, YoloThemePreset, YoloThemeTokens, apple_hig,
};
use yolo_tokens::presets::variant_for_preset;
use yolo_tokens::role::Role;
use yolo_tokens::scheme::{DynamicScheme, Variant};

pub mod chrome;
pub mod compose;
pub mod contrast;
pub mod cosmic;
pub mod extended;
pub mod fluent;
pub mod gtk;
mod legacy;
pub mod omarchy;
pub mod swiftui;
pub mod web;
pub mod winui;

/// Every role written by the CSS and DTCG emitters.
pub const ROLES: &[Role] = &[
    Role::Primary,
    Role::OnPrimary,
    Role::PrimaryContainer,
    Role::OnPrimaryContainer,
    Role::InversePrimary,
    Role::Secondary,
    Role::OnSecondary,
    Role::SecondaryContainer,
    Role::OnSecondaryContainer,
    Role::Tertiary,
    Role::OnTertiary,
    Role::TertiaryContainer,
    Role::OnTertiaryContainer,
    Role::Error,
    Role::OnError,
    Role::ErrorContainer,
    Role::OnErrorContainer,
    Role::Background,
    Role::OnBackground,
    Role::Surface,
    Role::OnSurface,
    Role::SurfaceVariant,
    Role::OnSurfaceVariant,
    Role::SurfaceDim,
    Role::SurfaceBright,
    Role::SurfaceContainerLowest,
    Role::SurfaceContainerLow,
    Role::SurfaceContainer,
    Role::SurfaceContainerHigh,
    Role::SurfaceContainerHighest,
    Role::SurfaceTint,
    Role::Outline,
    Role::OutlineVariant,
    Role::InverseSurface,
    Role::InverseOnSurface,
    Role::Shadow,
    Role::Scrim,
];

/// Theme request: seed, variant and contrast; both modes are resolved.
#[derive(Clone, Copy, Debug)]
pub struct Theme {
    /// Source color as `0xAARRGGBB`.
    pub seed: u32,
    /// Scheme variant.
    pub variant: Variant,
    /// Contrast level in `-1.0..=1.0` (0 standard, 0.5 medium, 1 high).
    pub contrast: f64,
}

impl Default for Theme {
    fn default() -> Self {
        Self::from_preset(YoloThemePreset::Neutral)
    }
}

impl Theme {
    /// Constructs a [`Theme`] from any built-in [`YoloThemePreset`].
    #[must_use]
    pub const fn from_preset(preset: YoloThemePreset) -> Self {
        Self {
            seed: preset.seed_argb(),
            variant: variant_for_preset(preset),
            contrast: 0.0,
        }
    }

    /// Resolves the scheme for one mode.
    #[must_use]
    pub fn scheme(&self, dark: bool) -> DynamicScheme {
        DynamicScheme::from_argb(self.seed, self.variant, dark, self.contrast)
    }
}

/// A resolved theme: the request, both schemes, and the identity used in file names and labels.
#[derive(Clone, Debug)]
pub struct ThemeSource {
    /// The request both schemes were resolved from.
    pub theme: Theme,
    /// The built-in preset, when the theme is one.
    pub preset: Option<YoloThemePreset>,
    /// Kebab-case identifier used for directory and file names (`yolo-aphrody`).
    pub slug: String,
    /// Short human-readable name (`Yolo Aphrody`).
    pub name: String,
    /// Light scheme.
    pub light: DynamicScheme,
    /// Dark scheme.
    pub dark: DynamicScheme,
}

impl ThemeSource {
    /// Resolves an arbitrary seed/variant/contrast request.
    #[must_use]
    pub fn from_theme(theme: Theme) -> Self {
        let seed = hex(theme.seed);
        Self {
            theme,
            preset: None,
            slug: format!("yolo-custom-{}", &seed[1..]),
            name: format!("Yolo Custom {seed}"),
            light: theme.scheme(false),
            dark: theme.scheme(true),
        }
    }

    /// Resolves a built-in preset.
    #[must_use]
    pub fn from_preset(preset: YoloThemePreset) -> Self {
        let theme = Theme::from_preset(preset);
        Self {
            theme,
            preset: Some(preset),
            slug: preset.id().to_owned(),
            name: format!("Yolo {}", preset_short_name(preset)),
            light: theme.scheme(false),
            dark: theme.scheme(true),
        }
    }

    /// The scheme of one mode.
    #[must_use]
    pub const fn scheme(&self, dark: bool) -> &DynamicScheme {
        if dark { &self.dark } else { &self.light }
    }

    /// The same request at high contrast (`contrast = 1.0`), for accessibility appearances.
    #[must_use]
    pub fn high_contrast(&self, dark: bool) -> DynamicScheme {
        DynamicScheme::from_argb(self.theme.seed, self.theme.variant, dark, 1.0)
    }

    /// `PascalCase` form of [`Self::slug`] (`YoloAphrody`), for type and file names.
    #[must_use]
    pub fn pascal(&self) -> String {
        self.slug
            .split('-')
            .filter(|part| !part.is_empty())
            .map(|part| {
                let mut chars = part.chars();
                chars.next().map_or_else(String::new, |first| {
                    first.to_ascii_uppercase().to_string() + chars.as_str()
                })
            })
            .collect()
    }

    /// Structural style (radii, spacing, blur) of the preset, when the theme is one.
    #[must_use]
    pub fn style(&self) -> Option<YoloStyle> {
        self.preset.map(YoloThemePreset::style)
    }
}

/// Short label of a preset (`Aphrody`), used in theme names that hosts display.
#[must_use]
pub const fn preset_short_name(preset: YoloThemePreset) -> &'static str {
    match preset {
        YoloThemePreset::Neutral => "Neutral",
        YoloThemePreset::M3 => "M3",
        YoloThemePreset::Fluent => "Fluent",
        YoloThemePreset::Apple => "Apple",
        YoloThemePreset::Omarchy => "Omarchy",
        YoloThemePreset::Hyprland => "Hyprland",
        YoloThemePreset::Aphrody => "Aphrody",
        YoloThemePreset::Chromium => "Chromium",
    }
}

/// One generated file, with a `/`-separated path relative to the target's output directory.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EmittedFile {
    /// Relative path (`yolo-aphrody-dark/manifest.json`).
    pub path: String,
    /// File contents.
    pub contents: String,
}

impl EmittedFile {
    fn new(path: impl Into<String>, contents: String) -> Self {
        Self {
            path: path.into(),
            contents,
        }
    }
}

/// Output formats across Web, Linux (GTK4, KDE, COSMIC, Omarchy, Hyprland), Qt6, Windows (WinUI 3 / Fluent 2),
/// macOS/iOS (asset catalogs, SwiftUI), Android (Compose), Chrome, and W3C Design Tokens (DTCG).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    /// `--md-sys-color-*` custom properties with `prefers-color-scheme` and `data-theme` switches.
    Css,
    /// Tailwind CSS v4 stylesheet: mode-switched custom properties plus an `@theme inline` block.
    Tailwind,
    /// GTK 4 / libadwaita 1.6+ stylesheet (`gtk.css`) plus per-mode `@define-color` sheets.
    Gtk,
    /// KDE Plasma color scheme (`.colors`).
    Kde,
    /// COSMIC `ThemeBuilder` RON (the format COSMIC Settings imports), one file per mode.
    Cosmic,
    /// QML singleton for Qt Quick (`M3Theme.qml`).
    Qml,
    /// WinUI 3 `ResourceDictionary` with Fluent 2 theme resources for Default (dark), Light and HighContrast.
    Winui,
    /// Fluent 2 web theme JSON: a 16-step brand ramp plus light and dark alias tokens.
    Fluent,
    /// Apple HIG CSS custom properties (`AppleLiquidGlass.css`).
    Apple,
    /// Xcode asset catalog (`.colorset` per role, light/dark/high-contrast) plus a SwiftUI accessor file.
    Swiftui,
    /// Jetpack Compose Material 3 `ColorScheme` Kotlin with light, dark and dynamic color.
    Compose,
    /// Chrome theme extensions (`manifest.json`, manifest version 3), one directory per mode.
    Chrome,
    /// Omarchy theme directories (`colors.toml`, `icons.theme`), one per mode.
    Omarchy,
    /// Hyprland (`hyprwm`) `source`-able config fragment.
    Hyprland,
    /// W3C Design Tokens (DTCG 2025.10) interchange JSON.
    Dtcg,
}

impl Target {
    /// Every supported emitter target (15 targets).
    pub const ALL: &'static [Target] = &[
        Target::Css,
        Target::Tailwind,
        Target::Gtk,
        Target::Kde,
        Target::Cosmic,
        Target::Qml,
        Target::Winui,
        Target::Fluent,
        Target::Apple,
        Target::Swiftui,
        Target::Compose,
        Target::Chrome,
        Target::Omarchy,
        Target::Hyprland,
        Target::Dtcg,
    ];

    /// CLI name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Target::Css => "css",
            Target::Tailwind => "tailwind",
            Target::Gtk => "gtk",
            Target::Kde => "kde",
            Target::Cosmic => "cosmic",
            Target::Qml => "qml",
            Target::Winui => "winui",
            Target::Fluent => "fluent",
            Target::Apple => "apple",
            Target::Swiftui => "swiftui",
            Target::Compose => "compose",
            Target::Chrome => "chrome",
            Target::Omarchy => "omarchy",
            Target::Hyprland => "hyprland",
            Target::Dtcg => "dtcg",
        }
    }

    /// Conventional file name of the primary output (the first file of [`emit_files`]).
    #[must_use]
    pub const fn file_name(self) -> &'static str {
        match self {
            Target::Css => "m3-colors.css",
            Target::Tailwind => "tailwind.css",
            Target::Gtk => "gtk.css",
            Target::Kde => "YoloUiM3.colors",
            Target::Cosmic => "cosmic-theme.ron",
            Target::Qml => "M3Theme.qml",
            Target::Winui => "M3Theme.xaml",
            Target::Fluent => "fluent-theme.json",
            Target::Apple => "AppleLiquidGlass.css",
            Target::Swiftui => "Colors.swift",
            Target::Compose => "Theme.kt",
            Target::Chrome => "manifest.json",
            Target::Omarchy => "colors.toml",
            Target::Hyprland => "m3-hyprland.conf",
            Target::Dtcg => "m3.tokens.json",
        }
    }

    /// Parses a CLI name.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|t| t.name() == name)
    }
}

/// Every file of `target` for an already-resolved theme. The first file is the primary output.
#[must_use]
pub fn emit_files(source: &ThemeSource, target: Target) -> Vec<EmittedFile> {
    let single = |contents: String| vec![EmittedFile::new(target.file_name(), contents)];
    match target {
        Target::Css => single(web::css(source)),
        Target::Tailwind => single(web::tailwind(source)),
        Target::Gtk => gtk::files(source),
        Target::Kde => single(legacy::kde(&source.dark)),
        Target::Cosmic => cosmic::files(source),
        Target::Qml => single(legacy::qml(&source.light, &source.dark)),
        Target::Winui => single(winui::resource_dictionary(source)),
        Target::Fluent => single(fluent::theme_json(source)),
        Target::Apple => single(legacy::apple(source)),
        Target::Swiftui => swiftui::files(source),
        Target::Compose => compose::files(source),
        Target::Chrome => chrome::files(source),
        Target::Omarchy => omarchy::files(source),
        Target::Hyprland => single(legacy::hyprland(source)),
        Target::Dtcg => single(legacy::dtcg(&source.light, &source.dark)),
    }
}

/// Every file of `target` for a built-in preset.
#[must_use]
pub fn emit_preset_files(preset: YoloThemePreset, target: Target) -> Vec<EmittedFile> {
    emit_files(&ThemeSource::from_preset(preset), target)
}

fn primary(mut files: Vec<EmittedFile>) -> String {
    if files.is_empty() {
        String::new()
    } else {
        files.swap_remove(0).contents
    }
}

/// Writes `theme` in the `target` format (the primary file of [`emit_files`]).
#[must_use]
pub fn emit(theme: &Theme, target: Target) -> String {
    primary(emit_files(&ThemeSource::from_theme(*theme), target))
}

/// Emits any of the 23 official [`OmarchySubTheme`] palettes as an Omarchy `colors.toml`.
#[must_use]
pub fn emit_omarchy_subtheme(subtheme: OmarchySubTheme) -> String {
    omarchy::subtheme_colors_toml(subtheme)
}

/// Emits any of the 8 built-in [`YoloThemePreset`]s in the requested [`Target`] format
/// (the primary file of [`emit_preset_files`]), including preset-specific metadata.
#[must_use]
pub fn emit_preset(preset: YoloThemePreset, target: Target) -> String {
    primary(emit_preset_files(preset, target))
}

pub(crate) const fn rgb(argb: u32) -> (u8, u8, u8) {
    ((argb >> 16) as u8, (argb >> 8) as u8, argb as u8)
}

/// `#rrggbb`.
#[must_use]
pub fn hex(argb: u32) -> String {
    let (r, g, b) = rgb(argb);
    format!("#{r:02x}{g:02x}{b:02x}")
}

/// `#AARRGGBB` (XAML order) with an explicit alpha byte.
#[must_use]
pub fn hex_argb(argb: u32, alpha: u8) -> String {
    let (r, g, b) = rgb(argb);
    format!("#{alpha:02X}{r:02X}{g:02X}{b:02X}")
}

pub(crate) fn c(s: &DynamicScheme, role: Role) -> String {
    hex(s.argb(role))
}

/// Escapes a string for a JSON, Kotlin, Swift or TOML double-quoted literal.
pub(crate) fn quoted(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for ch in text.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            c if c.is_control() => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Version of this crate without the pre-release suffix (`0.8.0`), valid as an extension version.
pub(crate) fn release_version() -> &'static str {
    let full = env!("CARGO_PKG_VERSION");
    full.split(['-', '+']).next().unwrap_or(full)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_targets_and_presets_emit_non_empty_output() {
        assert_eq!(Target::ALL.len(), 15);
        assert_eq!(YoloThemePreset::ALL.len(), 8);
        for preset in YoloThemePreset::ALL {
            for &target in Target::ALL {
                let files = emit_preset_files(preset, target);
                assert!(!files.is_empty());
                assert!(files.iter().all(|f| !f.contents.is_empty()));
                assert_eq!(emit_preset(preset, target), files[0].contents);
                assert_eq!(Target::from_name(target.name()), Some(target));
            }
        }
    }

    #[test]
    fn preset_signature_tokens_survive() {
        let hypr_out = emit_preset(YoloThemePreset::Hyprland, Target::Hyprland);
        assert!(hypr_out.contains("rgba(33ccffee)"));
        assert!(hypr_out.contains("rgba(00ff99ee)"));
        assert!(hypr_out.contains("vibrancy = 0.1696"));
        assert!(hypr_out.contains("bezier = myBezier, 0.05, 0.90, 0.10, 1.05"));

        let aphrody_css = emit_preset(YoloThemePreset::Aphrody, Target::Css);
        assert!(aphrody_css.contains("--byron-love-azur: #17335C;"));
        assert!(aphrody_css.contains("--aphrody-celestial-gold: #FFD700;"));

        let omarchy = emit_omarchy_subtheme(OmarchySubTheme::TokyoNight);
        assert!(omarchy.contains("accent = \"#7aa2f7\""));
    }

    #[test]
    fn pascal_and_release_version() {
        let src = ThemeSource::from_preset(YoloThemePreset::Aphrody);
        assert_eq!(src.pascal(), "YoloAphrody");
        assert_eq!(src.slug, "yolo-aphrody");
        assert!(
            release_version()
                .split('.')
                .all(|p| p.parse::<u32>().is_ok())
        );
    }
}
