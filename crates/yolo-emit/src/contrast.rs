// SPDX-License-Identifier: Apache-2.0
//! WCAG 2.2 contrast audit of the foreground/background role pairs every emitter writes.
//!
//! The ratio is computed from the 8-bit sRGB values the emitters actually write (WCAG 2.2
//! relative luminance), not from HCT tones, so it measures the shipped files.

use crate::extended::status;
use yolo_tokens::role::Role;
use yolo_tokens::scheme::DynamicScheme;

/// WCAG 2.2 AA minimum for normal text (SC 1.4.3).
pub const AA_TEXT: f64 = 4.5;

/// WCAG 2.2 relative luminance of an `0xAARRGGBB` color (alpha ignored).
#[must_use]
pub fn relative_luminance(argb: u32) -> f64 {
    let channel = |shift: u32| {
        let v = f64::from((argb >> shift) & 0xff) / 255.0;
        if v <= 0.040_45 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * channel(16) + 0.7152 * channel(8) + 0.0722 * channel(0)
}

/// WCAG contrast ratio (1..=21) between two colors.
#[must_use]
pub fn ratio(a: u32, b: u32) -> f64 {
    let (la, lb) = (relative_luminance(a), relative_luminance(b));
    let (hi, lo) = if la >= lb { (la, lb) } else { (lb, la) };
    (hi + 0.05) / (lo + 0.05)
}

/// Main content-on-fill role pairs (`foreground`, `background`) that must meet [`AA_TEXT`].
pub const ROLE_PAIRS: &[(Role, Role)] = &[
    (Role::OnPrimary, Role::Primary),
    (Role::OnPrimaryContainer, Role::PrimaryContainer),
    (Role::OnSecondary, Role::Secondary),
    (Role::OnSecondaryContainer, Role::SecondaryContainer),
    (Role::OnTertiary, Role::Tertiary),
    (Role::OnTertiaryContainer, Role::TertiaryContainer),
    (Role::OnError, Role::Error),
    (Role::OnErrorContainer, Role::ErrorContainer),
    (Role::OnBackground, Role::Background),
    (Role::OnSurface, Role::Surface),
    (Role::OnSurface, Role::SurfaceDim),
    (Role::OnSurface, Role::SurfaceBright),
    (Role::OnSurface, Role::SurfaceContainerLowest),
    (Role::OnSurface, Role::SurfaceContainerLow),
    (Role::OnSurface, Role::SurfaceContainer),
    (Role::OnSurface, Role::SurfaceContainerHigh),
    (Role::OnSurface, Role::SurfaceContainerHighest),
    (Role::OnSurfaceVariant, Role::Surface),
    (Role::OnSurfaceVariant, Role::SurfaceVariant),
    (Role::OnSurfaceVariant, Role::SurfaceContainerHighest),
    (Role::InverseOnSurface, Role::InverseSurface),
    (Role::InversePrimary, Role::InverseSurface),
    (Role::OnPrimaryFixed, Role::PrimaryFixed),
    (Role::OnPrimaryFixedVariant, Role::PrimaryFixed),
    (Role::OnSecondaryFixed, Role::SecondaryFixed),
    (Role::OnSecondaryFixedVariant, Role::SecondaryFixed),
    (Role::OnTertiaryFixed, Role::TertiaryFixed),
    (Role::OnTertiaryFixedVariant, Role::TertiaryFixed),
];

/// One audited pair.
#[derive(Clone, Debug, PartialEq)]
pub struct PairResult {
    /// Foreground label (`on-primary`).
    pub foreground: String,
    /// Background label (`primary`).
    pub background: String,
    /// Measured ratio.
    pub ratio: f64,
}

/// Measures every [`ROLE_PAIRS`] entry plus the status groups of `scheme`.
#[must_use]
pub fn audit(scheme: &DynamicScheme) -> Vec<PairResult> {
    let mut out: Vec<PairResult> = ROLE_PAIRS
        .iter()
        .map(|&(fg, bg)| PairResult {
            foreground: fg.name().to_owned(),
            background: bg.name().to_owned(),
            ratio: ratio(scheme.argb(fg), scheme.argb(bg)),
        })
        .collect();
    let st = status(scheme);
    for (name, group) in [("success", st.success), ("warning", st.warning)] {
        out.push(PairResult {
            foreground: format!("on-{name}"),
            background: name.to_owned(),
            ratio: ratio(group.on_color, group.color),
        });
        out.push(PairResult {
            foreground: format!("on-{name}-container"),
            background: format!("{name}-container"),
            ratio: ratio(group.on_container, group.container),
        });
        out.push(PairResult {
            foreground: name.to_owned(),
            background: "surface".to_owned(),
            ratio: ratio(group.color, scheme.argb(Role::Surface)),
        });
    }
    out
}

/// Pairs of [`audit`] below [`AA_TEXT`].
#[must_use]
pub fn failures(scheme: &DynamicScheme) -> Vec<PairResult> {
    audit(scheme)
        .into_iter()
        .filter(|p| p.ratio < AA_TEXT)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn black_on_white_is_21() {
        assert!((ratio(0xff00_0000, 0xffff_ffff) - 21.0).abs() < 1e-9);
        assert!((ratio(0xff77_7777, 0xff77_7777) - 1.0).abs() < 1e-12);
    }
}
