// SPDX-License-Identifier: Apache-2.0
//! Multi-preset theme emitters for every Yolo UI host.
//!
//! `yolo-tokens` resolves the scheme (HCT, dynamic color, contrast) and `yolo-shared` provides
//! the 7 first-class `YoloThemePreset`s (`Neutral`, `M3`, `Fluent`, `Apple`, `Omarchy`, `Hyprland`, `Aphrody`).
//! This crate maps the resolved roles and preset tokens to each host's native theme format,
//! following `catalog/conversion.toml`.

use std::fmt::Write;
pub use yolo_shared::{
    apple_hig, ByronLovePetSpec, HyprlandDecorationSpec, OmarchySubTheme, OmarchyThemePalette,
    YoloManifest, YoloStyle, YoloThemePreset, YoloThemeTokens, BYRON_LOVE_PALETTE,
    BYRON_LOVE_PET_SPEC, HYPRLAND_DECORATION_SPEC, OMARCHY_THEMES, YOLO_HYPRLAND_DARK_THEME,
};
use yolo_tokens::presets::variant_for_preset;
use yolo_tokens::role::Role;
use yolo_tokens::scheme::{DynamicScheme, Variant};

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
    /// Constructs a [`Theme`] from any of the 7 built-in [`YoloThemePreset`]s.
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

/// Output formats across Web, Linux (GTK4, KDE, COSMIC, Omarchy, Hyprland), Qt6, Windows (WinUI 3 / Fluent 2),
/// macOS/iOS (Apple HIG Liquid Glass), and W3C Design Tokens (DTCG).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    /// `--md-sys-color-*` custom properties with `prefers-color-scheme`.
    Css,
    /// GTK 4 / libadwaita 1.6+ stylesheet (`gtk.css`).
    Gtk,
    /// KDE Plasma color scheme (`.colors`).
    Kde,
    /// cosmic-config builder keys, one `key=value` RON line per field.
    Cosmic,
    /// QML singleton for Qt Quick (`M3Theme.qml`).
    Qml,
    /// WinUI 3 `ResourceDictionary` with Light and Dark theme dictionaries.
    Winui,
    /// Microsoft Fluent UI 2 / WinUI 3 (`FluentTheme.xaml` + Mica/Acrylic/Reveal tokens).
    Fluent,
    /// Apple macOS (Sequoia / Tahoe Liquid Glass `NSGlassEffectView`) & iOS (Cupertino / SwiftUI HIG) (`AppleLiquidGlass.css`).
    Apple,
    /// Omacom Omarchy theme bundle (`omarchy-theme.bundle`: `colors.toml` + Waybar + Walker + Hyprland + Ghostty + `omarchy-site` CSS).
    Omarchy,
    /// Hyprland (`hyprwm`) `source`-able config fragment + `hyprtoolkit` + `hyprqt6engine` tokens.
    Hyprland,
    /// W3C Design Tokens (DTCG 2025.10) interchange JSON.
    Dtcg,
}

impl Target {
    /// Every supported emitter target (11 targets).
    pub const ALL: &'static [Target] = &[
        Target::Css,
        Target::Gtk,
        Target::Kde,
        Target::Cosmic,
        Target::Qml,
        Target::Winui,
        Target::Fluent,
        Target::Apple,
        Target::Omarchy,
        Target::Hyprland,
        Target::Dtcg,
    ];

    /// CLI name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Target::Css => "css",
            Target::Gtk => "gtk",
            Target::Kde => "kde",
            Target::Cosmic => "cosmic",
            Target::Qml => "qml",
            Target::Winui => "winui",
            Target::Fluent => "fluent",
            Target::Apple => "apple",
            Target::Omarchy => "omarchy",
            Target::Hyprland => "hyprland",
            Target::Dtcg => "dtcg",
        }
    }

    /// Conventional file name of the output.
    #[must_use]
    pub const fn file_name(self) -> &'static str {
        match self {
            Target::Css => "m3-colors.css",
            Target::Gtk => "gtk.css",
            Target::Kde => "YoloUiM3.colors",
            Target::Cosmic => "cosmic-theme.ron",
            Target::Qml => "M3Theme.qml",
            Target::Winui => "M3Theme.xaml",
            Target::Fluent => "FluentTheme.xaml",
            Target::Apple => "AppleLiquidGlass.css",
            Target::Omarchy => "omarchy-theme.bundle",
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

/// Writes `theme` in the `target` format.
#[must_use]
pub fn emit(theme: &Theme, target: Target) -> String {
    let light = theme.scheme(false);
    let dark = theme.scheme(true);
    match target {
        Target::Css => css(&light, &dark),
        Target::Gtk => gtk(&light, &dark),
        Target::Kde => kde(&dark),
        Target::Cosmic => cosmic(&light, &dark),
        Target::Qml => qml(&light, &dark),
        Target::Winui => winui(&light, &dark),
        Target::Fluent => fluent(&light, &dark),
        Target::Apple => apple(&light, &dark),
        Target::Omarchy => omarchy(&dark, OmarchySubTheme::TokyoNight),
        Target::Hyprland => hyprland(&dark),
        Target::Dtcg => dtcg(&light, &dark),
    }
}

/// Emits any of the 23 official [`OmarchySubTheme`] palettes as a complete Omarchy bundle
/// (`colors.toml` + Waybar + Walker + Hyprland + Ghostty + `omarchy-site` CSS tokens).
#[must_use]
pub fn emit_omarchy_subtheme(subtheme: OmarchySubTheme) -> String {
    let p = subtheme.spec();
    let dark = Theme {
        seed: p.seed_argb,
        variant: Variant::Vibrant,
        contrast: 0.0,
    }
    .scheme(p.is_dark);
    omarchy(&dark, subtheme)
}

/// Emits any of the 7 built-in [`YoloThemePreset`]s (`Neutral`, `M3`, `Fluent`, `Apple`, `Omarchy`, `Hyprland`, `Aphrody`)
/// in the requested [`Target`] format, including preset-specific CSS/XAML/SwiftUI/Omarchy/Hyprland/Byron-Love metadata.
#[must_use]
pub fn emit_preset(preset: YoloThemePreset, target: Target) -> String {
    let theme = Theme::from_preset(preset);
    let mut out = emit(&theme, target);
    if matches!(target, Target::Css | Target::Apple | Target::Omarchy) {
        let dark_tokens = preset.tokens(true);
        let style = preset.style();
        let _ = writeln!(
            out,
            "\n/* YoloThemePreset: {} ({}) */\n:root {{\n  --yolo-theme-preset: \"{}\";\n  --yolo-theme-seed: {};\n  --yolo-bg-primary: {};\n  --yolo-bg-surface: {};\n  --yolo-bg-elevated: {};\n  --yolo-text-primary: {};\n  --yolo-text-secondary: {};\n  --yolo-accent-primary: {};\n  --yolo-accent-secondary: {};\n  --yolo-border-subtle: {};\n  --yolo-radius-sm: {}px;\n  --yolo-radius-md: {}px;\n  --yolo-radius-lg: {}px;\n  --yolo-backdrop-blur: {}px;\n  --yolo-window-vibrancy: {};",
            preset.id(),
            preset.display_name(),
            preset.id(),
            preset.seed_hex(),
            dark_tokens.bg_primary,
            dark_tokens.bg_surface,
            dark_tokens.bg_elevated,
            dark_tokens.text_primary,
            dark_tokens.text_secondary,
            dark_tokens.accent_primary,
            dark_tokens.accent_secondary,
            dark_tokens.border_subtle,
            style.radius_sm_px,
            style.radius_md_px,
            style.radius_lg_px,
            style.backdrop_blur_px,
            style.window_vibrancy,
        );
        if preset == YoloThemePreset::Omarchy {
            let p = OmarchySubTheme::TokyoNight.spec();
            let _ = writeln!(
                out,
                "  --omarchy-subtheme: \"{}\";\n  --omarchy-subtheme-count: {};\n  --t-bg: {};\n  --t-surface: {};\n  --t-brand: {};\n  --t-accent-cyan: {};\n  --t-accent-purple: {};\n  --t-border-subtle: {};",
                p.id,
                OmarchySubTheme::ALL.len(),
                p.background,
                p.lighter_background,
                p.accent,
                p.cyan,
                p.accent_secondary,
                p.muted_border,
            );
        }
        if preset == YoloThemePreset::Hyprland {
            let h = HYPRLAND_DECORATION_SPEC;
            let _ = writeln!(
                out,
                "  --hypr-active-border-start: {};\n  --hypr-active-border-end: {};\n  --hypr-active-border-angle: {}deg;\n  --hypr-inactive-border: {};\n  --hypr-rounding: {}px;\n  --hypr-blur-size: {};\n  --hypr-blur-passes: {};",
                h.active_border_start_hex,
                h.active_border_end_hex,
                h.active_border_angle_deg,
                h.inactive_border_rgba,
                h.rounding_px,
                h.kawase_blur_size,
                h.kawase_blur_passes,
            );
        }
        if preset == YoloThemePreset::Aphrody {
            for teinte in BYRON_LOVE_PALETTE {
                let _ = writeln!(out, "  --byron-love-{}: {};", teinte.id, teinte.hex);
            }
            let _ = writeln!(
                out,
                "  --aphrody-celestial-gold: {};\n  --aphrody-divine-violet: {};\n  --aphrody-divine-cyan: {};\n  --aphrody-pet-id: \"{}\";",
                BYRON_LOVE_PET_SPEC.celestial_gold_hex,
                BYRON_LOVE_PET_SPEC.divine_violet_hex,
                BYRON_LOVE_PET_SPEC.divine_cyan_hex,
                BYRON_LOVE_PET_SPEC.pet_id,
            );
        }
        out.push_str("}\n");
    }
    out
}

fn rgb(argb: u32) -> (u8, u8, u8) {
    ((argb >> 16) as u8, (argb >> 8) as u8, argb as u8)
}

/// `#rrggbb`.
#[must_use]
pub fn hex(argb: u32) -> String {
    let (r, g, b) = rgb(argb);
    format!("#{r:02x}{g:02x}{b:02x}")
}

fn c(s: &DynamicScheme, role: Role) -> String {
    hex(s.argb(role))
}

fn css_block(out: &mut String, s: &DynamicScheme, indent: &str) {
    for &role in ROLES {
        let _ = writeln!(out, "{indent}--md-sys-color-{}: {};", role.name(), c(s, role));
    }
}

fn css(light: &DynamicScheme, dark: &DynamicScheme) -> String {
    let mut out = String::from(
        "/* Generated by yolo-emit. Do not hand-edit. */\n:root {\n  color-scheme: light dark;\n",
    );
    css_block(&mut out, light, "  ");
    out.push_str("}\n@media (prefers-color-scheme: dark) {\n  :root {\n");
    css_block(&mut out, dark, "    ");
    out.push_str("  }\n}\n");
    out
}

fn gtk_vars(out: &mut String, s: &DynamicScheme, indent: &str) {
    // libadwaita 1.6+ CSS variables; the legacy @define-color names alias them.
    let pairs: &[(&str, Role)] = &[
        ("accent-bg-color", Role::Primary),
        ("accent-fg-color", Role::OnPrimary),
        ("accent-color", Role::Primary),
        ("destructive-bg-color", Role::Error),
        ("destructive-fg-color", Role::OnError),
        ("destructive-color", Role::Error),
        ("error-bg-color", Role::Error),
        ("error-fg-color", Role::OnError),
        ("error-color", Role::Error),
        ("window-bg-color", Role::Surface),
        ("window-fg-color", Role::OnSurface),
        ("view-bg-color", Role::SurfaceContainerLowest),
        ("view-fg-color", Role::OnSurface),
        ("headerbar-bg-color", Role::SurfaceContainer),
        ("headerbar-fg-color", Role::OnSurface),
        ("headerbar-backdrop-color", Role::Surface),
        ("sidebar-bg-color", Role::SurfaceContainerLow),
        ("sidebar-fg-color", Role::OnSurface),
        ("sidebar-backdrop-color", Role::Surface),
        ("secondary-sidebar-bg-color", Role::SurfaceContainer),
        ("secondary-sidebar-fg-color", Role::OnSurface),
        ("card-bg-color", Role::SurfaceContainer),
        ("card-fg-color", Role::OnSurface),
        ("dialog-bg-color", Role::SurfaceContainerHigh),
        ("dialog-fg-color", Role::OnSurface),
        ("popover-bg-color", Role::SurfaceContainerHigh),
        ("popover-fg-color", Role::OnSurface),
        ("thumbnail-bg-color", Role::SurfaceContainerHighest),
        ("thumbnail-fg-color", Role::OnSurface),
        ("border-color", Role::OutlineVariant),
        ("m3-primary-container", Role::PrimaryContainer),
        ("m3-on-primary-container", Role::OnPrimaryContainer),
        ("m3-secondary-container", Role::SecondaryContainer),
        ("m3-on-secondary-container", Role::OnSecondaryContainer),
        ("m3-tertiary", Role::Tertiary),
        ("m3-tertiary-container", Role::TertiaryContainer),
        ("m3-outline", Role::Outline),
        ("m3-inverse-surface", Role::InverseSurface),
        ("m3-inverse-on-surface", Role::InverseOnSurface),
    ];
    for (name, role) in pairs {
        let _ = writeln!(out, "{indent}--{name}: {};", c(s, *role));
    }
}

fn gtk(light: &DynamicScheme, dark: &DynamicScheme) -> String {
    let mut out = String::from(
        "/* Generated by yolo-emit for GTK 4 / libadwaita 1.6+. Do not hand-edit. */\n:root {\n",
    );
    gtk_vars(&mut out, light, "  ");
    out.push_str("}\n@media (prefers-color-scheme: dark) {\n  :root {\n");
    gtk_vars(&mut out, dark, "    ");
    out.push_str("  }\n}\n");
    out.push_str(concat!(
        "button.m3-tonal { background-color: var(--m3-secondary-container); color: var(--m3-on-secondary-container); }\n",
        "button.m3-outlined { background: none; box-shadow: inset 0 0 0 1px var(--m3-outline); }\n",
        "button.m3-filled, button.suggested-action { border-radius: 9999px; }\n",
        ".toast { background-color: var(--m3-inverse-surface); color: var(--m3-inverse-on-surface); }\n",
    ));
    out
}

fn kde_rgb(argb: u32) -> String {
    let (r, g, b) = rgb(argb);
    format!("{r},{g},{b}")
}

fn kde(s: &DynamicScheme) -> String {
    let k = |role| kde_rgb(s.argb(role));
    let mut out = String::from("# Generated by yolo-emit. Do not hand-edit.\n");
    let groups: &[(&str, Role, Role)] = &[
        ("Window", Role::Surface, Role::OnSurface),
        ("View", Role::SurfaceContainerLowest, Role::OnSurface),
        ("Button", Role::SurfaceContainer, Role::OnSurface),
        ("Header", Role::SurfaceContainerHighest, Role::OnSurface),
        ("Tooltip", Role::InverseSurface, Role::InverseOnSurface),
        ("Complementary", Role::SurfaceContainerHigh, Role::OnSurface),
        ("Selection", Role::Primary, Role::OnPrimary),
    ];
    for (group, bg, fg) in groups {
        let _ = write!(
            out,
            "\n[Colors:{group}]\nBackgroundNormal={}\nBackgroundAlternate={}\nForegroundNormal={}\nForegroundInactive={}\nForegroundActive={}\nForegroundLink={}\nForegroundVisited={}\nForegroundNegative={}\nForegroundNeutral={}\nForegroundPositive={}\nDecorationFocus={}\nDecorationHover={}\n",
            k(*bg),
            k(Role::SurfaceContainerLow),
            k(*fg),
            k(Role::OnSurfaceVariant),
            k(Role::Primary),
            k(Role::Primary),
            k(Role::Tertiary),
            k(Role::Error),
            k(Role::Tertiary),
            k(Role::Secondary),
            k(Role::Primary),
            k(Role::Outline),
        );
    }
    let _ = write!(
        out,
        "\n[General]\nColorScheme=YoloUiM3\nName=Yolo UI M3\nAccentColor={}\n\n[KDE]\ncontrast=4\n",
        k(Role::Primary)
    );
    out
}

fn ron_srgb(argb: u32) -> String {
    let (r, g, b) = rgb(argb);
    format!(
        "(red: {:.6}, green: {:.6}, blue: {:.6})",
        f32::from(r) / 255.0,
        f32::from(g) / 255.0,
        f32::from(b) / 255.0
    )
}

fn ron_srgba(argb: u32) -> String {
    let (r, g, b) = rgb(argb);
    format!(
        "(red: {:.6}, green: {:.6}, blue: {:.6}, alpha: 1.0)",
        f32::from(r) / 255.0,
        f32::from(g) / 255.0,
        f32::from(b) / 255.0
    )
}

fn cosmic(light: &DynamicScheme, dark: &DynamicScheme) -> String {
    let mut out = String::from(
        "// Generated by yolo-emit. One `<mode>/<field> = <RON>` per cosmic-config key.\n",
    );
    for (mode, s) in [("Light", light), ("Dark", dark)] {
        let fields: &[(&str, String)] = &[
            ("accent", format!("Some({})", ron_srgb(s.argb(Role::Primary)))),
            ("bg_color", format!("Some({})", ron_srgba(s.argb(Role::Surface)))),
            (
                "primary_container_bg",
                format!("Some({})", ron_srgba(s.argb(Role::SurfaceContainer))),
            ),
            (
                "secondary_container_bg",
                format!("Some({})", ron_srgba(s.argb(Role::SurfaceContainerHigh))),
            ),
            ("text_tint", format!("Some({})", ron_srgb(s.argb(Role::OnSurface)))),
            ("neutral_tint", format!("Some({})", ron_srgb(s.argb(Role::SurfaceTint)))),
            ("destructive", format!("Some({})", ron_srgb(s.argb(Role::Error)))),
            ("success", format!("Some({})", ron_srgb(s.argb(Role::Secondary)))),
            ("warning", format!("Some({})", ron_srgb(s.argb(Role::Tertiary)))),
        ];
        for (field, value) in fields {
            let _ = writeln!(out, "com.system76.CosmicTheme.{mode}.Builder/v2/{field} = {value}");
        }
    }
    out
}

fn qml(light: &DynamicScheme, dark: &DynamicScheme) -> String {
    let mut out = String::from(
        "// Generated by yolo-emit. Do not hand-edit.\npragma Singleton\nimport QtQuick\n\nQtObject {\n    property bool dark: Application.styleHints.colorScheme === Qt.ColorScheme.Dark\n",
    );
    for &role in ROLES {
        let _ = writeln!(
            out,
            "    readonly property color {}: dark ? \"{}\" : \"{}\"",
            role.camel_name(),
            c(dark, role),
            c(light, role)
        );
    }
    out.push_str("}\n");
    out
}

fn winui(light: &DynamicScheme, dark: &DynamicScheme) -> String {
    let pairs: &[(&str, Role)] = &[
        ("SystemAccentColor", Role::Primary),
        ("M3OnPrimary", Role::OnPrimary),
        ("M3PrimaryContainer", Role::PrimaryContainer),
        ("M3SecondaryContainer", Role::SecondaryContainer),
        ("M3Surface", Role::Surface),
        ("M3OnSurface", Role::OnSurface),
        ("M3OnSurfaceVariant", Role::OnSurfaceVariant),
        ("M3SurfaceContainer", Role::SurfaceContainer),
        ("M3SurfaceContainerHigh", Role::SurfaceContainerHigh),
        ("M3Outline", Role::Outline),
        ("M3OutlineVariant", Role::OutlineVariant),
        ("M3Error", Role::Error),
    ];
    let mut out = String::from(
        "<!-- Generated by yolo-emit. Do not hand-edit. -->\n<ResourceDictionary xmlns=\"http://schemas.microsoft.com/winfx/2006/xaml/presentation\" xmlns:x=\"http://schemas.microsoft.com/winfx/2006/xaml\">\n  <ResourceDictionary.ThemeDictionaries>\n",
    );
    for (key, s) in [("Light", light), ("Dark", dark)] {
        let _ = writeln!(out, "    <ResourceDictionary x:Key=\"{key}\">");
        for (name, role) in pairs {
            let (r, g, b) = rgb(s.argb(*role));
            let _ = writeln!(out, "      <Color x:Key=\"{name}\">#FF{r:02X}{g:02X}{b:02X}</Color>");
        }
        out.push_str(concat!(
            "      <SolidColorBrush x:Key=\"AccentFillColorDefaultBrush\" Color=\"{StaticResource SystemAccentColor}\" />\n",
            "      <SolidColorBrush x:Key=\"TextFillColorPrimaryBrush\" Color=\"{StaticResource M3OnSurface}\" />\n",
            "      <SolidColorBrush x:Key=\"TextFillColorSecondaryBrush\" Color=\"{StaticResource M3OnSurfaceVariant}\" />\n",
            "      <SolidColorBrush x:Key=\"SolidBackgroundFillColorBaseBrush\" Color=\"{StaticResource M3Surface}\" />\n",
            "      <SolidColorBrush x:Key=\"CardBackgroundFillColorDefaultBrush\" Color=\"{StaticResource M3SurfaceContainer}\" />\n",
            "      <SolidColorBrush x:Key=\"ControlStrokeColorDefaultBrush\" Color=\"{StaticResource M3OutlineVariant}\" />\n",
            "      <SolidColorBrush x:Key=\"SystemFillColorCriticalBrush\" Color=\"{StaticResource M3Error}\" />\n",
            "    </ResourceDictionary>\n",
        ));
    }
    out.push_str("  </ResourceDictionary.ThemeDictionaries>\n</ResourceDictionary>\n");
    out
}

fn fluent(light: &DynamicScheme, dark: &DynamicScheme) -> String {
    let mut out = winui(light, dark);
    let _ = write!(
        out,
        "\n<!-- Microsoft Fluent UI 2 / WinUI 3 Design Tokens (Mica / Acrylic / Smoke & Corner Radii) -->\n\
         <!-- ControlCornerRadius: 4,4,4,4 | OverlayCornerRadius: 8,8,8,8 | RevealBorderThickness: 1 -->\n\
         /*\n\
         :root {{\n\
           --fluent-brand-primary: #0078D4;\n\
           --fluent-accent-light: {};\n\
           --fluent-accent-dark: {};\n\
           --fluent-surface-mica-dark: #202020;\n\
           --fluent-surface-acrylic-dark: rgba(44, 44, 44, 0.78);\n\
           --fluent-backdrop-smoke: rgba(0, 0, 0, 0.30);\n\
           --fluent-radius-control: 4px;\n\
           --fluent-radius-overlay: 8px;\n\
           --fluent-stroke-reveal: 1px;\n\
         }}\n\
         */\n",
        c(light, Role::Primary),
        c(dark, Role::Primary),
    );
    out
}

fn apple(light: &DynamicScheme, dark: &DynamicScheme) -> String {
    format!(
        "/* Generated by yolo-emit for Apple macOS (Sequoia / Tahoe Liquid Glass NSGlassEffectView) & iOS (Cupertino / SwiftUI HIG). */\n\
         :root {{\n\
           color-scheme: light dark;\n\
           --apple-system-blue: {blue};\n\
           --apple-system-green: {green};\n\
           --apple-system-orange: {orange};\n\
           --apple-system-red: {red};\n\
           --apple-traffic-close: {close};\n\
           --apple-traffic-minimize: {minimize};\n\
           --apple-traffic-zoom: {zoom};\n\
           --apple-liquid-glass-blur: {blur:.0}px;\n\
           --apple-liquid-glass-backdrop: blur({blur:.0}px) saturate(180%);\n\
           --apple-liquid-glass-specular: inset 0 1px 0 rgba(255, 255, 255, 0.35);\n\
           --apple-radius-squircle-md: 10px;\n\
           --apple-radius-squircle-lg: 14px;\n\
           --apple-font-sans: 'SF Pro', -apple-system, BlinkMacSystemFont, 'Helvetica Neue', sans-serif;\n\
           --apple-font-mono: 'SF Mono', ui-monospace, Menlo, monospace;\n\
           --apple-scheme-primary-light: {pl};\n\
           --apple-scheme-surface-light: {sl};\n\
           --apple-scheme-primary-dark: {pd};\n\
           --apple-scheme-surface-dark: {sd};\n\
         }}\n\
         /* SwiftUI HIG Token Equivalence:\n\
          * Color.accentColor -> Color(hex: \"{pd}\")\n\
          * .glassEffect(.regular.interactive(), in: .rect(cornerRadius: 14, style: .continuous))\n\
          */\n",
        blue = apple_hig::SYSTEM_BLUE,
        green = apple_hig::SYSTEM_GREEN,
        orange = apple_hig::SYSTEM_ORANGE,
        red = apple_hig::SYSTEM_RED,
        close = apple_hig::TRAFFIC_LIGHT_CLOSE,
        minimize = apple_hig::TRAFFIC_LIGHT_MINIMIZE,
        zoom = apple_hig::TRAFFIC_LIGHT_ZOOM,
        blur = apple_hig::LIQUID_GLASS_BLUR_PX,
        pl = c(light, Role::Primary),
        sl = c(light, Role::Surface),
        pd = c(dark, Role::Primary),
        sd = c(dark, Role::Surface),
    )
}

fn omarchy(s: &DynamicScheme, subtheme: OmarchySubTheme) -> String {
    let p = subtheme.spec();
    let mut out = format!(
        "# Generated by yolo-emit for Omacom Omarchy ({label} / `{id}`).\n\
         # Supports all 23 official Omarchy themes + Waybar + Walker + Hyprland + Ghostty + omarchy-site.\n\
         [omarchy.colors]\n\
         accent = \"{accent}\"\n\
         cursor = \"{cursor}\"\n\
         foreground = \"{fg}\"\n\
         background = \"{bg}\"\n\
         selection_foreground = \"{sel_fg}\"\n\
         selection_background = \"{sel_bg}\"\n\
         color0 = \"{c0}\"\n\
         color1 = \"{c1}\"\n\
         color2 = \"{c2}\"\n\
         color3 = \"{c3}\"\n\
         color4 = \"{c4}\"\n\
         color5 = \"{c5}\"\n\
         color6 = \"{c6}\"\n\
         color7 = \"{c7}\"\n\
         color8 = \"{c8}\"\n\
         color9 = \"{c9}\"\n\
         color10 = \"{c10}\"\n\
         color11 = \"{c11}\"\n\
         color12 = \"{c12}\"\n\
         color13 = \"{c13}\"\n\
         color14 = \"{c14}\"\n\
         color15 = \"{c15}\"\n\n\
         /* Waybar + Walker + omarchy-site CSS variables */\n\
         @define-color foreground {fg};\n\
         @define-color background {bg};\n\
         @define-color accent {accent};\n\
         :root {{\n\
           --font-sans: 'Geist Variable', -apple-system, BlinkMacSystemFont, sans-serif;\n\
           --font-mono: 'JetBrains Mono Variable', 'CaskaydiaMono Nerd Font', monospace;\n\
           --radius-xs: 0px;\n\
           --radius-sm: 0px;\n\
           --t-bg: {bg};\n\
           --t-surface: {c0};\n\
           --t-surface-2: {c8};\n\
           --t-text: {fg};\n\
           --t-text-muted: {c7};\n\
           --t-brand: {accent};\n\
           --t-accent-cyan: {c6};\n\
           --t-accent-purple: {c5};\n\
           --t-accent-green: {c2};\n\
           --t-accent-amber: {c3};\n\
           --t-accent-red: {c1};\n\
           --t-m3-primary: {m3_primary};\n\
         }}\n",
        label = p.display_name,
        id = p.id,
        accent = p.accent,
        cursor = p.bright_foreground,
        fg = p.foreground,
        bg = p.background,
        sel_fg = p.bright_foreground,
        sel_bg = p.selection_background,
        c0 = p.lighter_background,
        c1 = p.red,
        c2 = p.green,
        c3 = p.yellow,
        c4 = p.accent,
        c5 = p.accent_secondary,
        c6 = p.cyan,
        c7 = p.foreground,
        c8 = p.muted_border,
        c9 = p.red,
        c10 = p.green,
        c11 = p.yellow,
        c12 = p.accent,
        c13 = p.accent_secondary,
        c14 = p.cyan,
        c15 = p.bright_foreground,
        m3_primary = c(s, Role::Primary),
    );
    let _ = writeln!(
        out,
        "/* Catalog of all {} embedded Omarchy subthemes: */",
        OMARCHY_THEMES.len()
    );
    for t in OMARCHY_THEMES {
        let _ = writeln!(out, "/* - {}: bg={} fg={} accent={} */", t.id, t.background, t.foreground, t.accent);
    }
    out
}

fn hypr_rgb(argb: u32) -> String {
    let (r, g, b) = rgb(argb);
    format!("rgb({r:02x}{g:02x}{b:02x})")
}

fn hyprland(s: &DynamicScheme) -> String {
    let h = HYPRLAND_DECORATION_SPEC;
    format!(
        "# Generated by yolo-emit for Hyprland (`hyprwm`), `hyprtoolkit`, and `hyprqt6engine`.\n\
         # `source = m3-hyprland.conf` from hyprland.conf.\n\
         $hypr_active_start = rgba({start_hex}ee)\n\
         $hypr_active_end = rgba({end_hex}ee)\n\
         $hypr_inactive = rgba(595959aa)\n\
         $m3_primary = {p}\n\
         $m3_on_surface = {on}\n\
         $m3_surface = {sf}\n\
         $m3_outline = {ol}\n\
         general {{\n\
             gaps_in = {gaps_in}\n\
             gaps_out = {gaps_out}\n\
             border_size = {border_size}\n\
             col.active_border = $hypr_active_start $hypr_active_end {angle}deg\n\
             col.inactive_border = $hypr_inactive\n\
         }}\n\
         decoration {{\n\
             rounding = {rounding}\n\
             shadow {{\n\
                 enabled = true\n\
                 range = 4\n\
                 render_power = 3\n\
                 color = rgba({sh}ee)\n\
             }}\n\
             blur {{\n\
                 enabled = true\n\
                 size = {blur_size}\n\
                 passes = {blur_passes}\n\
                 vibrancy = {vibrancy:.4}\n\
             }}\n\
         }}\n\
         animations {{\n\
             enabled = yes\n\
             bezier = {bezier_name}, {b0:.2}, {b1:.2}, {b2:.2}, {b3:.2}\n\
         }}\n\
         # hyprtoolkit / hyprqt6engine palette variables\n\
         # background = {bg_abyss} | surface = {bg_surface} | elevated = {bg_elevated}\n",
        start_hex = &h.active_border_start_hex[1..],
        end_hex = &h.active_border_end_hex[1..],
        angle = h.active_border_angle_deg,
        gaps_in = h.gaps_in_px,
        gaps_out = h.gaps_out_px,
        border_size = h.border_size_px,
        rounding = h.rounding_px,
        blur_size = h.kawase_blur_size,
        blur_passes = h.kawase_blur_passes,
        vibrancy = h.kawase_blur_vibrancy,
        bezier_name = h.bezier_name,
        b0 = h.bezier_points[0],
        b1 = h.bezier_points[1],
        b2 = h.bezier_points[2],
        b3 = h.bezier_points[3],
        bg_abyss = YOLO_HYPRLAND_DARK_THEME.bg_primary,
        bg_surface = YOLO_HYPRLAND_DARK_THEME.bg_surface,
        bg_elevated = YOLO_HYPRLAND_DARK_THEME.bg_elevated,
        p = hypr_rgb(s.argb(Role::Primary)),
        on = hypr_rgb(s.argb(Role::OnSurface)),
        sf = hypr_rgb(s.argb(Role::Surface)),
        ol = hypr_rgb(s.argb(Role::OutlineVariant)),
        sh = &hex(s.argb(Role::Shadow))[1..],
    )
}

fn dtcg(light: &DynamicScheme, dark: &DynamicScheme) -> String {
    let mut out = String::from(
        "{\n  \"$description\": \"Generated by yolo-emit (DTCG 2025.10)\",\n  \"color\": {\n",
    );
    for (mi, (mode, s)) in [("light", light), ("dark", dark)].into_iter().enumerate() {
        let _ = writeln!(out, "    \"{mode}\": {{");
        for (i, &role) in ROLES.iter().enumerate() {
            let (r, g, b) = rgb(s.argb(role));
            let comma = if i + 1 == ROLES.len() { "" } else { "," };
            let _ = writeln!(
                out,
                "      \"{}\": {{ \"$type\": \"color\", \"$value\": {{ \"colorSpace\": \"srgb\", \"components\": [{:.4}, {:.4}, {:.4}], \"hex\": \"{}\" }} }}{comma}",
                role.name(),
                f32::from(r) / 255.0,
                f32::from(g) / 255.0,
                f32::from(b) / 255.0,
                hex(s.argb(role)),
            );
        }
        let _ = writeln!(out, "    }}{}", if mi == 0 { "," } else { "" });
    }
    out.push_str("  }\n}\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_targets_and_presets_emit_non_empty_output() {
        assert_eq!(Target::ALL.len(), 11);
        assert_eq!(YoloThemePreset::ALL.len(), 7);
        for preset in YoloThemePreset::ALL {
            for &target in Target::ALL {
                let output = emit_preset(preset, target);
                assert!(!output.is_empty());
                assert_eq!(Target::from_name(target.name()), Some(target));
            }
        }
    }

    #[test]
    fn fluent_apple_omarchy_hyprland_and_aphrody_targets_contain_signature_tokens() {
        let fluent_out = emit_preset(YoloThemePreset::Fluent, Target::Fluent);
        assert!(fluent_out.contains("ResourceDictionary"));
        assert!(fluent_out.contains("--fluent-brand-primary: #0078D4;"));
        assert!(fluent_out.contains("--fluent-radius-control: 4px;"));

        let apple_out = emit_preset(YoloThemePreset::Apple, Target::Apple);
        assert!(apple_out.contains("--apple-system-blue: #007AFF;"));
        assert!(apple_out.contains("--apple-traffic-close: #FF5F56;"));
        assert!(apple_out.contains("blur(24px)"));

        let omarchy_out = emit_preset(YoloThemePreset::Omarchy, Target::Omarchy);
        assert!(omarchy_out.contains("[omarchy.colors]"));
        assert!(omarchy_out.contains("--omarchy-subtheme-count: 23;"));
        assert!(omarchy_out.contains("#7aa2f7"));

        let hypr_out = emit_preset(YoloThemePreset::Hyprland, Target::Hyprland);
        assert!(hypr_out.contains("rgba(33ccffee)"));
        assert!(hypr_out.contains("rgba(00ff99ee)"));
        assert!(hypr_out.contains("vibrancy = 0.1696"));
        assert!(hypr_out.contains("bezier = myBezier, 0.05, 0.90, 0.10, 1.05"));

        let aphrody_css = emit_preset(YoloThemePreset::Aphrody, Target::Css);
        assert!(aphrody_css.contains("--byron-love-azur: #17335C;"));
        assert!(aphrody_css.contains("--aphrody-celestial-gold: #FFD700;"));
    }
}
