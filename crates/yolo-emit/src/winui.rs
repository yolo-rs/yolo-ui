// SPDX-License-Identifier: Apache-2.0
//! WinUI 3 `ResourceDictionary` with Fluent 2 theme resources.
//!
//! Merge the file into `Application.Resources`. It overrides the WinUI 3 theme resource keys
//! (`TextFillColorPrimary`, `ControlFillColorDefault`, `SolidBackgroundFillColorBase`, ...) in
//! the `Default` (dark), `Light` and `HighContrast` theme dictionaries, defines the matching
//! `*Brush` resources, Mica-alt and Acrylic brushes, the accent ramp (`SystemAccentColor`,
//! `SystemAccentColorLight1..3`, `SystemAccentColorDark1..3`) and the Fluent 2 corner radii.
//! High contrast maps every brush to a Windows system color so the user's contrast theme wins.

use crate::extended::{Status, primary, status};
use crate::{EmittedFile, ThemeSource, hex_argb};
use std::fmt::Write;
use yolo_tokens::role::Role;
use yolo_tokens::scheme::DynamicScheme;

/// Default XAML namespace.
pub const XMLNS: &str = "http://schemas.microsoft.com/winfx/2006/xaml/presentation";
/// XAML language namespace (`x:`).
pub const XMLNS_X: &str = "http://schemas.microsoft.com/winfx/2006/xaml";

#[derive(Clone, Copy)]
enum Src {
    Role(Role),
    Success,
    SuccessBackground,
    Caution,
    CautionBackground,
}

/// Windows high-contrast system color used for a key in the `HighContrast` dictionary.
#[derive(Clone, Copy)]
enum Hc {
    WindowText,
    Window,
    Highlight,
    HighlightText,
    GrayText,
    ButtonFace,
    ButtonText,
}

impl Hc {
    const fn key(self) -> &'static str {
        match self {
            Hc::WindowText => "SystemColorWindowTextColor",
            Hc::Window => "SystemColorWindowColor",
            Hc::Highlight => "SystemColorHighlightColor",
            Hc::HighlightText => "SystemColorHighlightTextColor",
            Hc::GrayText => "SystemColorGrayTextColor",
            Hc::ButtonFace => "SystemColorButtonFaceColor",
            Hc::ButtonText => "SystemColorButtonTextColor",
        }
    }
}

/// `(key, source, alpha, high-contrast system color)` for every overridden Fluent 2 color.
const KEYS: &[(&str, Src, u8, Hc)] = &[
    (
        "TextFillColorPrimary",
        Src::Role(Role::OnSurface),
        0xFF,
        Hc::WindowText,
    ),
    (
        "TextFillColorSecondary",
        Src::Role(Role::OnSurfaceVariant),
        0xFF,
        Hc::WindowText,
    ),
    (
        "TextFillColorTertiary",
        Src::Role(Role::OnSurfaceVariant),
        0xC8,
        Hc::WindowText,
    ),
    (
        "TextFillColorDisabled",
        Src::Role(Role::OnSurface),
        0x5C,
        Hc::GrayText,
    ),
    (
        "AccentTextFillColorPrimary",
        Src::Role(Role::Primary),
        0xFF,
        Hc::Highlight,
    ),
    (
        "AccentTextFillColorSecondary",
        Src::Role(Role::Primary),
        0xE6,
        Hc::Highlight,
    ),
    (
        "AccentTextFillColorTertiary",
        Src::Role(Role::Primary),
        0xCC,
        Hc::Highlight,
    ),
    (
        "AccentTextFillColorDisabled",
        Src::Role(Role::OnSurface),
        0x5C,
        Hc::GrayText,
    ),
    (
        "TextOnAccentFillColorPrimary",
        Src::Role(Role::OnPrimary),
        0xFF,
        Hc::HighlightText,
    ),
    (
        "TextOnAccentFillColorSecondary",
        Src::Role(Role::OnPrimary),
        0xB3,
        Hc::HighlightText,
    ),
    (
        "TextOnAccentFillColorDisabled",
        Src::Role(Role::OnSurface),
        0x87,
        Hc::GrayText,
    ),
    (
        "TextOnAccentFillColorSelectedText",
        Src::Role(Role::OnPrimary),
        0xFF,
        Hc::HighlightText,
    ),
    (
        "AccentFillColorDefault",
        Src::Role(Role::Primary),
        0xFF,
        Hc::Highlight,
    ),
    (
        "AccentFillColorSecondary",
        Src::Role(Role::Primary),
        0xE6,
        Hc::Highlight,
    ),
    (
        "AccentFillColorTertiary",
        Src::Role(Role::Primary),
        0xCC,
        Hc::Highlight,
    ),
    (
        "AccentFillColorDisabled",
        Src::Role(Role::OnSurface),
        0x29,
        Hc::GrayText,
    ),
    (
        "AccentFillColorSelectedTextBackground",
        Src::Role(Role::Primary),
        0xFF,
        Hc::Highlight,
    ),
    (
        "ControlFillColorDefault",
        Src::Role(Role::SurfaceContainerHigh),
        0xFF,
        Hc::ButtonFace,
    ),
    (
        "ControlFillColorSecondary",
        Src::Role(Role::SurfaceContainerHighest),
        0xFF,
        Hc::ButtonFace,
    ),
    (
        "ControlFillColorTertiary",
        Src::Role(Role::SurfaceContainer),
        0xFF,
        Hc::ButtonFace,
    ),
    (
        "ControlFillColorDisabled",
        Src::Role(Role::SurfaceContainerLow),
        0xFF,
        Hc::ButtonFace,
    ),
    (
        "ControlFillColorTransparent",
        Src::Role(Role::Surface),
        0x00,
        Hc::ButtonFace,
    ),
    (
        "ControlFillColorInputActive",
        Src::Role(Role::SurfaceContainerLowest),
        0xFF,
        Hc::Window,
    ),
    (
        "ControlStrongFillColorDefault",
        Src::Role(Role::OnSurfaceVariant),
        0xFF,
        Hc::ButtonText,
    ),
    (
        "ControlStrongFillColorDisabled",
        Src::Role(Role::Outline),
        0x51,
        Hc::GrayText,
    ),
    (
        "ControlSolidFillColorDefault",
        Src::Role(Role::SurfaceContainerHighest),
        0xFF,
        Hc::ButtonFace,
    ),
    (
        "SubtleFillColorTransparent",
        Src::Role(Role::Surface),
        0x00,
        Hc::Window,
    ),
    (
        "SubtleFillColorSecondary",
        Src::Role(Role::OnSurface),
        0x0F,
        Hc::Highlight,
    ),
    (
        "SubtleFillColorTertiary",
        Src::Role(Role::OnSurface),
        0x0A,
        Hc::Highlight,
    ),
    (
        "SubtleFillColorDisabled",
        Src::Role(Role::Surface),
        0x00,
        Hc::Window,
    ),
    (
        "ControlAltFillColorTransparent",
        Src::Role(Role::Surface),
        0x00,
        Hc::Window,
    ),
    (
        "ControlAltFillColorSecondary",
        Src::Role(Role::SurfaceContainerLow),
        0xFF,
        Hc::ButtonFace,
    ),
    (
        "ControlAltFillColorTertiary",
        Src::Role(Role::SurfaceContainer),
        0xFF,
        Hc::ButtonFace,
    ),
    (
        "ControlAltFillColorQuarternary",
        Src::Role(Role::SurfaceContainerHigh),
        0xFF,
        Hc::ButtonFace,
    ),
    (
        "ControlAltFillColorDisabled",
        Src::Role(Role::Surface),
        0x00,
        Hc::Window,
    ),
    (
        "ControlOnImageFillColorDefault",
        Src::Role(Role::SurfaceContainer),
        0xB3,
        Hc::Window,
    ),
    (
        "ControlStrokeColorDefault",
        Src::Role(Role::OutlineVariant),
        0xFF,
        Hc::ButtonText,
    ),
    (
        "ControlStrokeColorSecondary",
        Src::Role(Role::Outline),
        0x66,
        Hc::ButtonText,
    ),
    (
        "ControlStrokeColorOnAccentDefault",
        Src::Role(Role::OnPrimary),
        0x14,
        Hc::HighlightText,
    ),
    (
        "ControlStrokeColorOnAccentSecondary",
        Src::Role(Role::OnPrimary),
        0x66,
        Hc::HighlightText,
    ),
    (
        "ControlStrongStrokeColorDefault",
        Src::Role(Role::Outline),
        0xFF,
        Hc::ButtonText,
    ),
    (
        "ControlStrongStrokeColorDisabled",
        Src::Role(Role::OutlineVariant),
        0xFF,
        Hc::GrayText,
    ),
    (
        "CardStrokeColorDefault",
        Src::Role(Role::OutlineVariant),
        0x80,
        Hc::WindowText,
    ),
    (
        "CardStrokeColorDefaultSolid",
        Src::Role(Role::OutlineVariant),
        0xFF,
        Hc::WindowText,
    ),
    (
        "CardBackgroundFillColorDefault",
        Src::Role(Role::SurfaceContainerLow),
        0xFF,
        Hc::Window,
    ),
    (
        "CardBackgroundFillColorSecondary",
        Src::Role(Role::SurfaceContainer),
        0xFF,
        Hc::Window,
    ),
    (
        "SurfaceStrokeColorDefault",
        Src::Role(Role::OutlineVariant),
        0xFF,
        Hc::WindowText,
    ),
    (
        "SurfaceStrokeColorFlyout",
        Src::Role(Role::OutlineVariant),
        0xFF,
        Hc::WindowText,
    ),
    (
        "DividerStrokeColorDefault",
        Src::Role(Role::OutlineVariant),
        0xFF,
        Hc::WindowText,
    ),
    (
        "FocusStrokeColorOuter",
        Src::Role(Role::OnSurface),
        0xFF,
        Hc::WindowText,
    ),
    (
        "FocusStrokeColorInner",
        Src::Role(Role::Surface),
        0xFF,
        Hc::Window,
    ),
    (
        "SolidBackgroundFillColorBase",
        Src::Role(Role::Surface),
        0xFF,
        Hc::Window,
    ),
    (
        "SolidBackgroundFillColorBaseAlt",
        Src::Role(Role::SurfaceDim),
        0xFF,
        Hc::Window,
    ),
    (
        "SolidBackgroundFillColorSecondary",
        Src::Role(Role::SurfaceContainerLow),
        0xFF,
        Hc::Window,
    ),
    (
        "SolidBackgroundFillColorTertiary",
        Src::Role(Role::SurfaceContainer),
        0xFF,
        Hc::Window,
    ),
    (
        "SolidBackgroundFillColorQuarternary",
        Src::Role(Role::SurfaceContainerHigh),
        0xFF,
        Hc::Window,
    ),
    (
        "LayerFillColorDefault",
        Src::Role(Role::SurfaceContainerLow),
        0x80,
        Hc::Window,
    ),
    (
        "LayerFillColorAlt",
        Src::Role(Role::SurfaceContainer),
        0xFF,
        Hc::Window,
    ),
    (
        "LayerOnMicaBaseAltFillColorDefault",
        Src::Role(Role::SurfaceContainer),
        0xB3,
        Hc::Window,
    ),
    (
        "SmokeFillColorDefault",
        Src::Role(Role::Scrim),
        0x4D,
        Hc::Window,
    ),
    (
        "SystemFillColorCritical",
        Src::Role(Role::Error),
        0xFF,
        Hc::WindowText,
    ),
    (
        "SystemFillColorCriticalBackground",
        Src::Role(Role::ErrorContainer),
        0xFF,
        Hc::Window,
    ),
    ("SystemFillColorSuccess", Src::Success, 0xFF, Hc::WindowText),
    (
        "SystemFillColorSuccessBackground",
        Src::SuccessBackground,
        0xFF,
        Hc::Window,
    ),
    ("SystemFillColorCaution", Src::Caution, 0xFF, Hc::WindowText),
    (
        "SystemFillColorCautionBackground",
        Src::CautionBackground,
        0xFF,
        Hc::Window,
    ),
    (
        "SystemFillColorAttention",
        Src::Role(Role::Primary),
        0xFF,
        Hc::WindowText,
    ),
    (
        "SystemFillColorAttentionBackground",
        Src::Role(Role::SurfaceContainerHigh),
        0xFF,
        Hc::Window,
    ),
    (
        "SystemFillColorNeutral",
        Src::Role(Role::Outline),
        0xFF,
        Hc::WindowText,
    ),
    (
        "SystemFillColorNeutralBackground",
        Src::Role(Role::SurfaceContainerHighest),
        0xFF,
        Hc::Window,
    ),
    (
        "SystemFillColorSolidNeutral",
        Src::Role(Role::Outline),
        0xFF,
        Hc::WindowText,
    ),
    (
        "SystemFillColorSolidNeutralBackground",
        Src::Role(Role::SurfaceContainerHigh),
        0xFF,
        Hc::Window,
    ),
];

fn argb(s: &DynamicScheme, st: &Status, src: Src) -> u32 {
    match src {
        Src::Role(role) => s.argb(role),
        Src::Success => st.success.color,
        Src::SuccessBackground => st.success.container,
        Src::Caution => st.warning.color,
        Src::CautionBackground => st.warning.container,
    }
}

/// Every `x:Key` the `Default` and `Light` dictionaries define as `Color`.
#[must_use]
pub fn color_keys() -> Vec<&'static str> {
    KEYS.iter().map(|k| k.0).collect()
}

fn mode_dictionary(out: &mut String, key: &str, s: &DynamicScheme) {
    let st = status(s);
    let _ = writeln!(out, "    <ResourceDictionary x:Key=\"{key}\">");
    for (name, src, alpha, _) in KEYS {
        let _ = writeln!(
            out,
            "      <Color x:Key=\"{name}\">{}</Color>",
            hex_argb(argb(s, &st, *src), *alpha)
        );
    }
    for (name, ..) in KEYS {
        let _ = writeln!(
            out,
            "      <SolidColorBrush x:Key=\"{name}Brush\" Color=\"{{StaticResource {name}}}\" />"
        );
    }
    let _ = writeln!(
        out,
        "      <AcrylicBrush x:Key=\"AcrylicBackgroundFillColorDefaultBrush\" TintColor=\"{0}\" TintOpacity=\"0.15\" TintLuminosityOpacity=\"0.96\" FallbackColor=\"{0}\" />\n      <AcrylicBrush x:Key=\"AcrylicInAppFillColorDefaultBrush\" TintColor=\"{1}\" TintOpacity=\"0.15\" TintLuminosityOpacity=\"0.96\" FallbackColor=\"{1}\" />\n      <SolidColorBrush x:Key=\"MicaBackgroundFillColorBaseAltBrush\" Color=\"{{StaticResource SolidBackgroundFillColorBaseAlt}}\" />",
        hex_argb(s.argb(Role::SurfaceContainer), 0xFF),
        hex_argb(s.argb(Role::SurfaceContainerHigh), 0xFF),
    );
    out.push_str("    </ResourceDictionary>\n");
}

fn high_contrast_dictionary(out: &mut String) {
    out.push_str("    <ResourceDictionary x:Key=\"HighContrast\">\n");
    for (name, _, _, hc) in KEYS {
        let _ = writeln!(
            out,
            "      <SolidColorBrush x:Key=\"{name}Brush\" Color=\"{{ThemeResource {}}}\" />",
            hc.key()
        );
    }
    out.push_str(concat!(
        "      <SolidColorBrush x:Key=\"AcrylicBackgroundFillColorDefaultBrush\" Color=\"{ThemeResource SystemColorWindowColor}\" />\n",
        "      <SolidColorBrush x:Key=\"AcrylicInAppFillColorDefaultBrush\" Color=\"{ThemeResource SystemColorWindowColor}\" />\n",
        "      <SolidColorBrush x:Key=\"MicaBackgroundFillColorBaseAltBrush\" Color=\"{ThemeResource SystemColorWindowColor}\" />\n",
        "    </ResourceDictionary>\n",
    ));
}

/// Accent ramp tones of the primary palette (`SystemAccentColorDark3` .. `Light3`).
const ACCENT_RAMP: [(&str, f64); 7] = [
    ("SystemAccentColorDark3", 20.0),
    ("SystemAccentColorDark2", 30.0),
    ("SystemAccentColorDark1", 40.0),
    ("SystemAccentColor", 50.0),
    ("SystemAccentColorLight1", 60.0),
    ("SystemAccentColorLight2", 70.0),
    ("SystemAccentColorLight3", 80.0),
];

/// The complete `ResourceDictionary`.
#[must_use]
pub fn resource_dictionary(src: &ThemeSource) -> String {
    let mut out = format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<!-- Generated by yolo-emit for {} (WinUI 3, Fluent 2 theme resources) from yolo-tokens roles. Do not hand-edit. -->\n<ResourceDictionary xmlns=\"{XMLNS}\" xmlns:x=\"{XMLNS_X}\">\n",
        src.name
    );
    for (name, tone) in ACCENT_RAMP {
        let _ = writeln!(
            out,
            "  <Color x:Key=\"{name}\">{}</Color>",
            hex_argb(primary(&src.light, tone), 0xFF)
        );
    }
    let style = src.style();
    let (control, overlay) = style.map_or((4.0, 8.0), |s| (s.radius_sm_px, s.radius_lg_px));
    let _ = writeln!(
        out,
        "  <CornerRadius x:Key=\"ControlCornerRadius\">{control}</CornerRadius>\n  <CornerRadius x:Key=\"OverlayCornerRadius\">{overlay}</CornerRadius>"
    );
    out.push_str("  <ResourceDictionary.ThemeDictionaries>\n");
    mode_dictionary(&mut out, "Default", &src.dark);
    mode_dictionary(&mut out, "Light", &src.light);
    high_contrast_dictionary(&mut out);
    out.push_str("  </ResourceDictionary.ThemeDictionaries>\n</ResourceDictionary>\n");
    out
}

/// The `ResourceDictionary` as a single file.
#[must_use]
pub fn files(src: &ThemeSource) -> Vec<EmittedFile> {
    vec![EmittedFile::new("M3Theme.xaml", resource_dictionary(src))]
}
