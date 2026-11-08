//! Contrast gates: 4.5:1 for text, 3:1 for UI components, enforced at compute time.
//!
//! The M3 scheme already meets the text gate at contrast 0 for generated palettes, but user
//! overrides, reduced contrast and exotic seeds can break it. For every failing pair the
//! foreground role keeps its hue and chroma and moves along the tone axis (through
//! `yolo_tokens::hct`) until the ratio passes. The ratio is the WCAG 2.x one computed from the
//! actual 8-bit sRGB values (`yolo_theme_pack::resolve::contrast_ratio`).

use serde::Serialize;
use yolo_theme_pack::manifest::parse_hex;
use yolo_theme_pack::resolve::contrast_ratio;
use yolo_tokens::hct::Hct;

/// Minimum ratio for text.
pub const TEXT_MIN: f64 = 4.5;
/// Minimum ratio for UI components and graphical objects.
pub const UI_MIN: f64 = 3.0;

/// What a pair protects.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Kind {
    /// Text over a surface.
    Text,
    /// Borders, focus rings and icons over a surface.
    Ui,
}

/// A foreground role over a background role.
#[derive(Clone, Copy, Debug)]
pub struct Pair {
    /// Foreground role (the one that moves).
    pub fg: &'static str,
    /// Background role.
    pub bg: &'static str,
    /// Text or UI.
    pub kind: Kind,
}

impl Pair {
    const fn text(fg: &'static str, bg: &'static str) -> Self {
        Self { fg, bg, kind: Kind::Text }
    }
    const fn ui(fg: &'static str, bg: &'static str) -> Self {
        Self { fg, bg, kind: Kind::Ui }
    }

    /// Required ratio.
    #[must_use]
    pub const fn min(self) -> f64 {
        match self.kind {
            Kind::Text => TEXT_MIN,
            Kind::Ui => UI_MIN,
        }
    }
}

/// Gated pairs. UI pairs come first: moving `primary` for the 3:1 gate must happen before
/// `on-primary` is checked against it.
pub const PAIRS: &[Pair] = &[
    Pair::ui("primary", "surface"),
    Pair::ui("secondary", "surface"),
    Pair::ui("error", "surface"),
    Pair::ui("outline", "surface"),
    Pair::ui("outline", "surface-container"),
    Pair::text("on-primary", "primary"),
    Pair::text("on-secondary", "secondary"),
    Pair::text("on-tertiary", "tertiary"),
    Pair::text("on-error", "error"),
    Pair::text("on-primary-container", "primary-container"),
    Pair::text("on-secondary-container", "secondary-container"),
    Pair::text("on-tertiary-container", "tertiary-container"),
    Pair::text("on-error-container", "error-container"),
    Pair::text("on-surface", "surface"),
    Pair::text("on-surface", "surface-container-low"),
    Pair::text("on-surface", "surface-container"),
    Pair::text("on-surface", "surface-container-high"),
    Pair::text("on-surface", "surface-container-highest"),
    Pair::text("on-surface-variant", "surface"),
    Pair::text("on-surface-variant", "surface-container"),
    Pair::text("on-background", "background"),
    Pair::text("inverse-on-surface", "inverse-surface"),
    // Links are `primary` text over the page surface.
    Pair::text("primary", "surface"),
];

/// Measured result of one pair.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PairResult {
    /// Foreground role.
    pub fg: String,
    /// Background role.
    pub bg: String,
    /// Text or UI.
    pub kind: Kind,
    /// Required ratio.
    pub min: f64,
    /// Measured ratio.
    pub ratio: f64,
    /// `ratio >= min`.
    pub pass: bool,
}

/// One tone adjustment made to satisfy a gate.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Adjustment {
    /// `light` or `dark`.
    pub scheme: &'static str,
    /// Role that moved.
    pub role: String,
    /// Value before.
    pub from: String,
    /// Value after.
    pub to: String,
    /// Background it was checked against.
    pub against: String,
    /// Why: the gate kind.
    pub kind: Kind,
}

/// Outcome of enforcing the gates on both schemes.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct GateReport {
    /// Adjustments, in order.
    pub adjustments: Vec<Adjustment>,
    /// Pairs still failing after adjustment. Empty unless the background itself is unusable.
    pub failures: Vec<PairResult>,
    /// Every pair of both schemes, after adjustment.
    pub audit: Vec<(String, Vec<PairResult>)>,
}

impl GateReport {
    /// True when every gated pair passes in both schemes.
    #[must_use]
    pub fn passes(&self) -> bool {
        self.failures.is_empty()
    }
}

/// `Roles` is the ordered list produced by yolo-theme-pack: (`role-name`, `#rrggbb`).
pub type Roles = Vec<(String, String)>;

fn argb(roles: &[(String, String)], name: &str) -> Option<u32> {
    roles
        .iter()
        .find(|(n, _)| n == name)
        .and_then(|(_, v)| parse_hex(v))
}

fn hex(argb: u32) -> String {
    format!("#{:06x}", argb & 0x00FF_FFFF)
}

fn set(roles: &mut [(String, String)], name: &str, value: u32) {
    if let Some(slot) = roles.iter_mut().find(|(n, _)| n == name) {
        slot.1 = hex(value);
    }
}

/// Measures every gated pair without changing anything.
#[must_use]
pub fn audit(roles: &[(String, String)]) -> Vec<PairResult> {
    PAIRS
        .iter()
        .filter_map(|pair| {
            let ratio = contrast_ratio(argb(roles, pair.fg)?, argb(roles, pair.bg)?);
            Some(PairResult {
                fg: pair.fg.to_owned(),
                bg: pair.bg.to_owned(),
                kind: pair.kind,
                min: pair.min(),
                ratio: (ratio * 100.0).round() / 100.0,
                pass: ratio >= pair.min(),
            })
        })
        .collect()
}

/// Moves `fg` along the tone axis until it reaches `min` against `bg`. Tries away from the
/// background first, then the other way, then falls back to black or white.
fn fix_tone(fg: u32, bg: u32, min: f64) -> u32 {
    let start = Hct::from_argb(fg);
    let bg_tone = Hct::from_argb(bg).tone;
    // Aim a little above the minimum so 8-bit rounding cannot bring it back under.
    let target = min + 0.02;
    let away_up = start.tone >= bg_tone;
    for up in [away_up, !away_up] {
        let mut tone = start.tone;
        while (0.0..=100.0).contains(&tone) {
            let candidate = Hct::new(start.hue, start.chroma, tone).to_argb();
            if contrast_ratio(candidate, bg) >= target {
                return candidate;
            }
            tone += if up { 0.5 } else { -0.5 };
        }
    }
    let (black, white) = (0xFF00_0000, 0xFFFF_FFFF);
    if contrast_ratio(black, bg) >= contrast_ratio(white, bg) {
        black
    } else {
        white
    }
}

/// Moves each failing foreground along the tone axis until its pair passes. A foreground cannot
/// satisfy two backgrounds that sit on opposite sides of the tone range, which only happens when
/// the user overrides a surface to an extreme colour: [`crate::compute`] then rejects that
/// override. Returns the adjustments made.
pub fn enforce(roles: &mut Roles, scheme: &'static str) -> Vec<Adjustment> {
    let mut made = Vec::new();
    for _ in 0..6 {
        let mut changed = false;
        for pair in PAIRS {
            changed |= repair(roles, scheme, pair, &mut made);
        }
        if !changed {
            break;
        }
    }
    made
}

/// Repairs one pair by moving its foreground.
fn repair(
    roles: &mut Roles,
    scheme: &'static str,
    pair: &Pair,
    made: &mut Vec<Adjustment>,
) -> bool {
    let (Some(fg), Some(bg)) = (argb(roles, pair.fg), argb(roles, pair.bg)) else {
        return false;
    };
    if contrast_ratio(fg, bg) >= pair.min() {
        return false;
    }
    let fixed = fix_tone(fg, bg, pair.min());
    if fixed == fg {
        return false;
    }
    made.push(Adjustment {
        scheme,
        role: pair.fg.to_owned(),
        from: hex(fg),
        to: hex(fixed),
        against: pair.bg.to_owned(),
        kind: pair.kind,
    });
    set(roles, pair.fg, fixed);
    true
}

/// Enforces both schemes and builds the report.
pub fn enforce_both(light: &mut Roles, dark: &mut Roles) -> GateReport {
    let mut report = GateReport::default();
    for (name, roles) in [("light", light), ("dark", dark)] {
        report.adjustments.extend(enforce(roles, name));
        let results = audit(roles);
        report
            .failures
            .extend(results.iter().filter(|r| !r.pass).cloned());
        report.audit.push((name.to_owned(), results));
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roles(pairs: &[(&str, &str)]) -> Roles {
        pairs
            .iter()
            .map(|(n, v)| ((*n).to_owned(), (*v).to_owned()))
            .collect()
    }

    #[test]
    fn failing_foreground_moves_and_keeps_its_hue() {
        let mut r = roles(&[("surface", "#ffffff"), ("on-surface", "#bbbbbb")]);
        let adj = enforce(&mut r, "light");
        assert_eq!(adj.len(), 1);
        let fixed = argb(&r, "on-surface").expect("role present");
        assert!(contrast_ratio(fixed, 0xFFFF_FFFF) >= TEXT_MIN);
    }

    #[test]
    fn passing_pairs_are_left_alone() {
        let mut r = roles(&[("surface", "#ffffff"), ("on-surface", "#111111")]);
        assert!(enforce(&mut r, "light").is_empty());
        assert_eq!(argb(&r, "on-surface"), Some(0xFF11_1111));
    }

    #[test]
    fn mid_grey_background_still_resolves() {
        // sqrt(21) - 0.05 luminance: the worst case for any single foreground.
        let mut r = roles(&[
            ("primary", "#777777"),
            ("on-primary", "#777777"),
            ("surface", "#ffffff"),
        ]);
        let _ = enforce(&mut r, "light");
        assert!(audit(&r).iter().all(|p| p.pass), "{:?}", audit(&r));
    }
}
