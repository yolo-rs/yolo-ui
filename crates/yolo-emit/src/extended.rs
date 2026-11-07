// SPDX-License-Identifier: Apache-2.0
//! Colors hosts need beyond the 49 Material roles: success and warning groups, and a 16-color
//! terminal palette. Every color is produced by `yolo-tokens`: a canonical hue is harmonized
//! toward the theme seed (`blend::harmonize`, at most 15 degrees) and read from an HCT tonal
//! palette at the Material custom-color tones, so the result keeps the scheme's contrast model.

use yolo_tokens::blend::harmonize;
use yolo_tokens::hct::Hct;
use yolo_tokens::palette::TonalPalette;
use yolo_tokens::scheme::DynamicScheme;

/// HCT hue of the canonical "success" color (green).
const SUCCESS_HUE: f64 = 145.0;
/// HCT hue of the canonical "warning" color (amber).
const WARNING_HUE: f64 = 75.0;
/// Chroma of status and terminal palettes.
const STATUS_CHROMA: f64 = 48.0;

/// A tonal palette for `hue`, harmonized toward `seed`.
#[must_use]
pub fn harmonized_palette(seed: u32, hue: f64, chroma: f64) -> TonalPalette {
    let design = Hct::new(hue, chroma, 50.0).to_argb();
    let hue = Hct::from_argb(harmonize(design, seed)).hue;
    TonalPalette::from_hue_and_chroma(hue, chroma)
}

/// A color role group in one mode (`color`, `on-color`, `color-container`, `on-color-container`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Group {
    /// Fill color.
    pub color: u32,
    /// Content on [`Self::color`].
    pub on_color: u32,
    /// Container fill.
    pub container: u32,
    /// Content on [`Self::container`].
    pub on_container: u32,
}

impl Group {
    fn of(palette: &TonalPalette, dark: bool) -> Self {
        if dark {
            Self {
                color: palette.tone(80.0),
                on_color: palette.tone(20.0),
                container: palette.tone(30.0),
                on_container: palette.tone(90.0),
            }
        } else {
            Self {
                color: palette.tone(40.0),
                on_color: palette.tone(100.0),
                container: palette.tone(90.0),
                on_container: palette.tone(10.0),
            }
        }
    }
}

/// Status colors of a scheme.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Status {
    /// Success (green) group.
    pub success: Group,
    /// Warning (amber) group.
    pub warning: Group,
}

/// Success and warning groups for `scheme`, harmonized toward its source color.
#[must_use]
pub fn status(scheme: &DynamicScheme) -> Status {
    let seed = scheme.source.to_argb();
    Status {
        success: Group::of(
            &harmonized_palette(seed, SUCCESS_HUE, STATUS_CHROMA),
            scheme.is_dark,
        ),
        warning: Group::of(
            &harmonized_palette(seed, WARNING_HUE, STATUS_CHROMA),
            scheme.is_dark,
        ),
    }
}

/// The named terminal colors Omarchy, COSMIC and terminal emulators consume.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Terminal {
    /// ANSI red.
    pub red: u32,
    /// ANSI yellow.
    pub yellow: u32,
    /// Orange (extended).
    pub orange: u32,
    /// ANSI green.
    pub green: u32,
    /// ANSI cyan.
    pub cyan: u32,
    /// ANSI blue.
    pub blue: u32,
    /// ANSI magenta.
    pub magenta: u32,
    /// Brown (extended).
    pub brown: u32,
    /// Bright red.
    pub bright_red: u32,
    /// Bright yellow.
    pub bright_yellow: u32,
    /// Bright green.
    pub bright_green: u32,
    /// Bright cyan.
    pub bright_cyan: u32,
    /// Bright blue.
    pub bright_blue: u32,
    /// Bright magenta.
    pub bright_magenta: u32,
}

/// Canonical HCT hues of the terminal colors.
pub const TERMINAL_HUES: [(&str, f64); 7] = [
    ("red", 27.0),
    ("orange", 55.0),
    ("yellow", 90.0),
    ("green", 142.0),
    ("cyan", 200.0),
    ("blue", 262.0),
    ("magenta", 325.0),
];

/// Terminal palette for `scheme`: base tones readable on the scheme surface, bright tones one
/// step further from it.
#[must_use]
pub fn terminal(scheme: &DynamicScheme) -> Terminal {
    let seed = scheme.source.to_argb();
    let (base, bright) = if scheme.is_dark {
        (72.0, 82.0)
    } else {
        (40.0, 32.0)
    };
    let tone = |hue: f64, tone: f64| harmonized_palette(seed, hue, STATUS_CHROMA + 8.0).tone(tone);
    let hue = |name: &str| {
        TERMINAL_HUES
            .iter()
            .find(|(n, _)| *n == name)
            .map_or(0.0, |(_, h)| *h)
    };
    Terminal {
        red: tone(hue("red"), base),
        yellow: tone(hue("yellow"), base),
        orange: tone(hue("orange"), base),
        green: tone(hue("green"), base),
        cyan: tone(hue("cyan"), base),
        blue: tone(hue("blue"), base),
        magenta: tone(hue("magenta"), base),
        brown: harmonized_palette(seed, hue("orange"), 24.0).tone(if scheme.is_dark {
            45.0
        } else {
            35.0
        }),
        bright_red: tone(hue("red"), bright),
        bright_yellow: tone(hue("yellow"), bright),
        bright_green: tone(hue("green"), bright),
        bright_cyan: tone(hue("cyan"), bright),
        bright_blue: tone(hue("blue"), bright),
        bright_magenta: tone(hue("magenta"), bright),
    }
}

/// A tone of the scheme's neutral palette.
#[must_use]
pub fn neutral(scheme: &DynamicScheme, tone: f64) -> u32 {
    scheme.palettes.neutral.tone(tone)
}

/// A tone of the scheme's neutral-variant palette.
#[must_use]
pub fn neutral_variant(scheme: &DynamicScheme, tone: f64) -> u32 {
    scheme.palettes.neutral_variant.tone(tone)
}

/// A tone of the scheme's primary palette.
#[must_use]
pub fn primary(scheme: &DynamicScheme, tone: f64) -> u32 {
    scheme.palettes.primary.tone(tone)
}
