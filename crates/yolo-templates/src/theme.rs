// SPDX-License-Identifier: Apache-2.0
//! Template palette: the eight `YoloThemePreset`s resolved into the roles the templates paint
//! with, each one solved to WCAG 2.2 AA against the surfaces it sits on.
//!
//! `yolo-shared` owns the raw preset tokens and is read here, never modified. Some raw pairs do
//! not reach AA (for example the Apple light secondary text on its canvas), so every text and
//! UI role goes through [`ensure_contrast`], which moves the colour toward the readable end
//! until the ratio holds against every background it is used on.

use std::fmt::Write as _;

use yolo_shared::{RgbaColor, YoloStyle, YoloThemePreset};

/// Light or dark rendering of a preset.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Mode {
    /// Light surfaces, dark text.
    Light,
    /// Dark surfaces, light text.
    Dark,
}

impl Mode {
    /// Both modes, light first.
    pub const ALL: [Self; 2] = [Self::Light, Self::Dark];

    /// `"light"` or `"dark"`, used in `data-yt-mode` and file names.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }

    /// Whether this is the dark mode.
    #[must_use]
    pub const fn is_dark(self) -> bool {
        matches!(self, Self::Dark)
    }
}

/// WCAG 2.x minimum ratio for body text (1.4.3).
pub const AA_TEXT: f64 = 4.5;
/// WCAG 2.x minimum ratio for focus indicators and component boundaries (1.4.11).
pub const AA_UI: f64 = 3.0;

fn channel(c: f32) -> f64 {
    let c = f64::from(c.clamp(0.0, 1.0));
    if c <= 0.040_45 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
}

/// WCAG relative luminance of an opaque colour.
#[must_use]
pub fn luminance(c: RgbaColor) -> f64 {
    0.2126 * channel(c.r) + 0.7152 * channel(c.g) + 0.0722 * channel(c.b)
}

/// WCAG contrast ratio between two opaque colours, `1.0..=21.0`.
#[must_use]
pub fn contrast(a: RgbaColor, b: RgbaColor) -> f64 {
    let (la, lb) = (luminance(a), luminance(b));
    let (hi, lo) = if la >= lb { (la, lb) } else { (lb, la) };
    (hi + 0.05) / (lo + 0.05)
}

/// `fg` alpha-composited over the opaque `bg`.
#[must_use]
pub fn composite(fg: RgbaColor, bg: RgbaColor) -> RgbaColor {
    let a = fg.a.clamp(0.0, 1.0);
    RgbaColor::from_rgb(
        fg.r * a + bg.r * (1.0 - a),
        fg.g * a + bg.g * (1.0 - a),
        fg.b * a + bg.b * (1.0 - a),
    )
}

/// The first colour on the line from `fg` to `toward` (in 2 % steps) that reaches `min` against
/// every colour in `backgrounds`. Returns `toward` if no step does, so callers pass a `toward`
/// that is itself readable (pure black or white).
#[must_use]
pub fn ensure_contrast(fg: RgbaColor, backgrounds: &[RgbaColor], min: f64, toward: RgbaColor) -> RgbaColor {
    let fg = backgrounds.first().map_or(fg, |bg| composite(fg, *bg));
    for step in 0..=50u8 {
        let c = fg.mix(toward, f32::from(step) / 50.0);
        if backgrounds.iter().all(|bg| contrast(c, *bg) >= min) {
            return c;
        }
    }
    toward
}

fn parse(css: &str, fallback: RgbaColor) -> RgbaColor {
    RgbaColor::parse_css(css).unwrap_or(fallback)
}

/// Near-black used for text on light accents; warmer than pure black.
pub const INK: RgbaColor = RgbaColor::from_rgb(0.043, 0.047, 0.063);

/// Font stacks of one preset. Faces are open (SIL OFL 1.1) variable fonts referenced by family
/// name; see `ASSETS.md`. Hosts that ship them bundle the font file with its `OFL.txt`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TypeStacks {
    /// Running text and controls.
    pub body: &'static str,
    /// Headlines and the wordmark.
    pub display: &'static str,
    /// Editorial headlines.
    pub serif: &'static str,
    /// Code, tokens and the terminal identity.
    pub mono: &'static str,
}

const BODY: &str = "'Instrument Sans', 'Atkinson Hyperlegible Next', system-ui, sans-serif";
const DISPLAY: &str = "'Bricolage Grotesque', 'Instrument Sans', system-ui, sans-serif";
const SERIF: &str = "'Fraunces', 'Source Serif 4', Georgia, serif";
const MONO: &str = "'JetBrains Mono', 'Iosevka', ui-monospace, monospace";
const GROTESK: &str = "'Space Grotesk', 'Instrument Sans', system-ui, sans-serif";

impl TypeStacks {
    /// Stacks for `preset`: Omarchy is the terminal identity (monospace everywhere), Hyprland the
    /// geometric one; every other preset shares the Aphrody pairing.
    #[must_use]
    pub const fn for_preset(preset: YoloThemePreset) -> Self {
        match preset {
            YoloThemePreset::Omarchy => Self { body: MONO, display: MONO, serif: MONO, mono: MONO },
            YoloThemePreset::Hyprland => Self { body: BODY, display: GROTESK, serif: SERIF, mono: MONO },
            _ => Self { body: BODY, display: DISPLAY, serif: SERIF, mono: MONO },
        }
    }
}

/// Every role a template paints with, resolved and AA-checked for one preset and mode.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Palette {
    /// Source preset.
    pub preset: YoloThemePreset,
    /// Light or dark.
    pub mode: Mode,
    /// Page background.
    pub canvas: RgbaColor,
    /// Cards, panels, nav bars.
    pub surface: RgbaColor,
    /// Flyouts, menus, raised cards.
    pub raised: RgbaColor,
    /// Primary text.
    pub text: RgbaColor,
    /// Secondary text, AA on canvas, surface and raised.
    pub muted: RgbaColor,
    /// Accent used as text (links, eyebrows), AA on canvas, surface and raised.
    pub accent: RgbaColor,
    /// Filled button background.
    pub accent_fill: RgbaColor,
    /// Text on `accent_fill`, AA.
    pub on_accent: RgbaColor,
    /// Decorative dividers (no contrast requirement).
    pub border: RgbaColor,
    /// Input and control boundaries, 3:1 on canvas and surface.
    pub control_border: RgbaColor,
    /// Focus ring, 3:1 on canvas, surface and raised.
    pub focus: RgbaColor,
    /// Stage (spotlight) background: always the dark preset canvas.
    pub stage: RgbaColor,
    /// Text on the stage.
    pub stage_text: RgbaColor,
    /// Secondary text on the stage.
    pub stage_muted: RgbaColor,
    /// Accent text on the stage.
    pub stage_accent: RgbaColor,
    /// Filled button on the stage.
    pub stage_fill: RgbaColor,
    /// Text on `stage_fill`.
    pub stage_on_fill: RgbaColor,
    /// Focus ring on the stage.
    pub stage_focus: RgbaColor,
    /// Geometry of the preset.
    pub style: YoloStyle,
    /// Font stacks of the preset.
    pub type_stacks: TypeStacks,
}

/// `(on, fill)`: the readable text colour for an accent and the accent adjusted until it holds AA.
fn fill_pair(accent: RgbaColor) -> (RgbaColor, RgbaColor) {
    let on = if contrast(RgbaColor::WHITE, accent) >= contrast(INK, accent) { RgbaColor::WHITE } else { INK };
    let away = if on == RgbaColor::WHITE { RgbaColor::BLACK } else { RgbaColor::WHITE };
    (on, ensure_contrast(accent, &[on], AA_TEXT, away))
}

impl Palette {
    /// Resolves `preset` in `mode`.
    #[must_use]
    pub fn resolve(preset: YoloThemePreset, mode: Mode) -> Self {
        let t = preset.tokens(mode.is_dark());
        let readable = if mode.is_dark() { RgbaColor::WHITE } else { RgbaColor::BLACK };
        let canvas = composite(parse(t.bg_primary, readable.mix(RgbaColor::WHITE, 0.5)), RgbaColor::BLACK);
        let surface = composite(parse(t.bg_surface, canvas), canvas);
        let raised = composite(parse(t.bg_elevated, surface), canvas);
        let grounds = [canvas, surface, raised];
        let text = ensure_contrast(parse(t.text_primary, readable), &grounds, 7.0, readable);
        let muted = ensure_contrast(parse(t.text_secondary, text), &grounds, AA_TEXT, text);
        let raw_accent = parse(t.accent_primary, text);
        let accent = ensure_contrast(raw_accent, &grounds, AA_TEXT, text);
        let (on_accent, accent_fill) = fill_pair(composite(raw_accent, canvas));
        let border = composite(parse(t.border_subtle, muted), canvas);
        let control_border = ensure_contrast(border, &[canvas, surface], AA_UI, text);
        let focus = ensure_contrast(raw_accent, &grounds, AA_UI, text);

        let d = preset.tokens(true);
        let stage = composite(parse(d.bg_primary, INK), RgbaColor::BLACK);
        let stage_text = ensure_contrast(parse(d.text_primary, RgbaColor::WHITE), &[stage], 7.0, RgbaColor::WHITE);
        let stage_muted = ensure_contrast(parse(d.text_secondary, stage_text), &[stage], AA_TEXT, stage_text);
        let stage_raw = composite(parse(d.accent_primary, stage_text), stage);
        let stage_accent = ensure_contrast(stage_raw, &[stage], AA_TEXT, stage_text);
        let (stage_on_fill, stage_fill) = fill_pair(stage_raw);
        let stage_focus = ensure_contrast(stage_raw, &[stage], AA_UI, stage_text);

        Self {
            preset,
            mode,
            canvas,
            surface,
            raised,
            text,
            muted,
            accent,
            accent_fill,
            on_accent,
            border,
            control_border,
            focus,
            stage,
            stage_text,
            stage_muted,
            stage_accent,
            stage_fill,
            stage_on_fill,
            stage_focus,
            style: preset.style(),
            type_stacks: TypeStacks::for_preset(preset),
        }
    }

    /// `(role, colour)` for every colour role, in documentation order.
    #[must_use]
    pub fn roles(&self) -> [(&'static str, RgbaColor); 18] {
        [
            ("canvas", self.canvas),
            ("surface", self.surface),
            ("raised", self.raised),
            ("text", self.text),
            ("muted", self.muted),
            ("accent", self.accent),
            ("accent-fill", self.accent_fill),
            ("on-accent", self.on_accent),
            ("border", self.border),
            ("control-border", self.control_border),
            ("focus", self.focus),
            ("stage", self.stage),
            ("stage-text", self.stage_text),
            ("stage-muted", self.stage_muted),
            ("stage-accent", self.stage_accent),
            ("stage-fill", self.stage_fill),
            ("stage-on-fill", self.stage_on_fill),
            ("stage-focus", self.stage_focus),
        ]
    }

    /// The background a role is checked against in the token docs, and the minimum ratio.
    /// `None` for background roles and decorative borders.
    #[must_use]
    pub fn requirement(&self, role: &str) -> Option<(RgbaColor, f64)> {
        Some(match role {
            "text" | "muted" | "accent" => (self.canvas, AA_TEXT),
            "on-accent" => (self.accent_fill, AA_TEXT),
            "control-border" | "focus" => (self.canvas, AA_UI),
            "stage-text" | "stage-muted" | "stage-accent" => (self.stage, AA_TEXT),
            "stage-on-fill" => (self.stage_fill, AA_TEXT),
            "stage-focus" => (self.stage, AA_UI),
            _ => return None,
        })
    }

    /// Custom-property declarations (`--yt-*`) for this palette, without a selector.
    #[must_use]
    pub fn declarations(&self) -> String {
        let mut out = String::new();
        let _ = write!(out, "color-scheme:{};", self.mode.id());
        for (role, c) in self.roles() {
            let _ = write!(out, "--yt-{role}:{};", c.to_css_string());
        }
        let s = &self.style;
        let t = &self.type_stacks;
        let _ = write!(
            out,
            "--yt-radius-sm:{}px;--yt-radius-md:{}px;--yt-radius-lg:{}px;--yt-blur:{}px;\
             --yt-font-body:{};--yt-font-display:{};--yt-font-serif:{};--yt-font-mono:{};",
            s.radius_sm_px, s.radius_md_px, s.radius_lg_px, s.backdrop_blur_px, t.body, t.display, t.serif, t.mono,
        );
        out
    }
}

/// Theme rules for every preset and mode.
///
/// `data-yt-mode` selects the server-rendered mode. When a page contains the theme switch
/// (`.yt-dark-switch`), its checked state wins, so the docs shell toggles the whole page with
/// no script.
#[must_use]
pub fn theme_css() -> String {
    let mut css = String::new();
    for preset in YoloThemePreset::ALL {
        let p = preset.id();
        let light = Palette::resolve(preset, Mode::Light).declarations();
        let dark = Palette::resolve(preset, Mode::Dark).declarations();
        let _ = write!(
            css,
            ".yt-root[data-yt-preset=\"{p}\"],.yt-root[data-yt-preset=\"{p}\"]:has(.yt-dark-switch:not(:checked)){{{light}}}\
             .yt-root[data-yt-preset=\"{p}\"][data-yt-mode=\"dark\"],.yt-root[data-yt-preset=\"{p}\"]:has(.yt-dark-switch:checked){{{dark}}}",
        );
    }
    css
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all() -> impl Iterator<Item = Palette> {
        YoloThemePreset::ALL
            .into_iter()
            .flat_map(|p| Mode::ALL.into_iter().map(move |m| Palette::resolve(p, m)))
    }

    #[test]
    fn contrast_math_matches_wcag_reference_points() {
        assert!((contrast(RgbaColor::BLACK, RgbaColor::WHITE) - 21.0).abs() < 1e-9);
        assert!((contrast(RgbaColor::WHITE, RgbaColor::WHITE) - 1.0).abs() < 1e-9);
        // #767676 on white is the classic 4.54:1 pair.
        let grey = RgbaColor::from_rgb8(0x76, 0x76, 0x76);
        assert!((contrast(grey, RgbaColor::WHITE) - 4.54).abs() < 0.01);
    }

    #[test]
    fn every_text_role_meets_aa_in_every_preset_and_mode() {
        for p in all() {
            for bg in [p.canvas, p.surface, p.raised] {
                assert!(contrast(p.text, bg) >= 7.0, "{:?} {:?} text", p.preset, p.mode);
                assert!(contrast(p.muted, bg) >= AA_TEXT, "{:?} {:?} muted", p.preset, p.mode);
                assert!(contrast(p.accent, bg) >= AA_TEXT, "{:?} {:?} accent", p.preset, p.mode);
                assert!(contrast(p.focus, bg) >= AA_UI, "{:?} {:?} focus", p.preset, p.mode);
            }
            assert!(contrast(p.on_accent, p.accent_fill) >= AA_TEXT, "{:?} {:?} on-accent", p.preset, p.mode);
            for bg in [p.canvas, p.surface] {
                assert!(contrast(p.control_border, bg) >= AA_UI, "{:?} {:?} control", p.preset, p.mode);
            }
            assert!(contrast(p.stage_text, p.stage) >= 7.0);
            assert!(contrast(p.stage_muted, p.stage) >= AA_TEXT);
            assert!(contrast(p.stage_accent, p.stage) >= AA_TEXT);
            assert!(contrast(p.stage_on_fill, p.stage_fill) >= AA_TEXT);
            assert!(contrast(p.stage_focus, p.stage) >= AA_UI);
        }
    }

    #[test]
    fn requirements_cover_every_foreground_role() {
        for p in all() {
            for (role, c) in p.roles() {
                if let Some((bg, min)) = p.requirement(role) {
                    assert!(contrast(c, bg) >= min, "{:?} {:?} {role}", p.preset, p.mode);
                }
            }
        }
    }

    #[test]
    fn solver_fixes_a_failing_raw_pair() {
        // The Apple light preset ships #86868b secondary text on #f5f5f7, about 3.3:1.
        let raw = RgbaColor::from_hex("#86868b").unwrap();
        let bg = RgbaColor::from_hex("#f5f5f7").unwrap();
        assert!(contrast(raw, bg) < AA_TEXT);
        let fixed = ensure_contrast(raw, &[bg], AA_TEXT, RgbaColor::BLACK);
        assert!(contrast(fixed, bg) >= AA_TEXT);
        // It moves no further than needed: one step back is below the floor.
        assert!(contrast(fixed, bg) < AA_TEXT + 0.5);
    }

    #[test]
    fn theme_css_has_light_and_dark_blocks_for_all_presets() {
        let css = theme_css();
        for preset in YoloThemePreset::ALL {
            assert!(css.contains(&format!("[data-yt-preset=\"{}\"][data-yt-mode=\"dark\"]", preset.id())));
        }
        assert_eq!(css.matches("color-scheme:light").count(), 8);
        assert_eq!(css.matches("color-scheme:dark").count(), 8);
        assert!(!css.contains("Inter,") && !css.contains("Roboto"));
    }
}
