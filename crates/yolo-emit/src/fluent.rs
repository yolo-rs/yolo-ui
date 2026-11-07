// SPDX-License-Identifier: Apache-2.0
//! Fluent 2 web theme JSON (`@fluentui/react-components` / `@fluentui/tokens`).
//!
//! `brand` is a `BrandVariants` ramp (keys `10`..`160`, darkest to lightest) read from the
//! primary tonal palette, so `createLightTheme(brand)` / `createDarkTheme(brand)` reproduce the
//! theme. `light` and `dark` hold the alias tokens overridden from the Material roles, to spread
//! over the created theme object.

use crate::extended::{primary, status};
use crate::{ThemeSource, hex, quoted};
use std::fmt::Write;
use yolo_tokens::role::Role;
use yolo_tokens::scheme::DynamicScheme;

#[derive(Clone, Copy)]
enum Src {
    Role(Role),
    Success,
    SuccessBackground,
    Warning,
    WarningBackground,
}

/// Fluent 2 alias tokens written for each mode.
const TOKENS: &[(&str, Src)] = &[
    ("colorNeutralForeground1", Src::Role(Role::OnSurface)),
    ("colorNeutralForeground2", Src::Role(Role::OnSurfaceVariant)),
    ("colorNeutralForeground3", Src::Role(Role::OnSurfaceVariant)),
    ("colorNeutralForegroundDisabled", Src::Role(Role::Outline)),
    (
        "colorNeutralForegroundInverted",
        Src::Role(Role::InverseOnSurface),
    ),
    ("colorNeutralForegroundOnBrand", Src::Role(Role::OnPrimary)),
    ("colorBrandForeground1", Src::Role(Role::Primary)),
    ("colorBrandForeground2", Src::Role(Role::OnPrimaryContainer)),
    ("colorBrandForegroundLink", Src::Role(Role::Primary)),
    ("colorNeutralBackground1", Src::Role(Role::Surface)),
    (
        "colorNeutralBackground2",
        Src::Role(Role::SurfaceContainerLow),
    ),
    ("colorNeutralBackground3", Src::Role(Role::SurfaceContainer)),
    (
        "colorNeutralBackground4",
        Src::Role(Role::SurfaceContainerHigh),
    ),
    (
        "colorNeutralBackground5",
        Src::Role(Role::SurfaceContainerHighest),
    ),
    ("colorNeutralBackground6", Src::Role(Role::SurfaceDim)),
    (
        "colorNeutralBackgroundInverted",
        Src::Role(Role::InverseSurface),
    ),
    (
        "colorNeutralBackgroundDisabled",
        Src::Role(Role::SurfaceContainerLow),
    ),
    (
        "colorSubtleBackgroundHover",
        Src::Role(Role::SurfaceContainerHigh),
    ),
    (
        "colorSubtleBackgroundPressed",
        Src::Role(Role::SurfaceContainerHighest),
    ),
    ("colorBrandBackground", Src::Role(Role::Primary)),
    ("colorBrandBackgroundHover", Src::Role(Role::SurfaceTint)),
    ("colorBrandBackgroundSelected", Src::Role(Role::Primary)),
    ("colorBrandBackground2", Src::Role(Role::PrimaryContainer)),
    ("colorCompoundBrandBackground", Src::Role(Role::Primary)),
    ("colorCompoundBrandForeground1", Src::Role(Role::Primary)),
    ("colorCompoundBrandStroke", Src::Role(Role::Primary)),
    ("colorNeutralStroke1", Src::Role(Role::Outline)),
    ("colorNeutralStroke2", Src::Role(Role::OutlineVariant)),
    ("colorNeutralStroke3", Src::Role(Role::OutlineVariant)),
    ("colorNeutralStrokeAccessible", Src::Role(Role::Outline)),
    (
        "colorNeutralStrokeDisabled",
        Src::Role(Role::OutlineVariant),
    ),
    ("colorStrokeFocus1", Src::Role(Role::Surface)),
    ("colorStrokeFocus2", Src::Role(Role::OnSurface)),
    ("colorNeutralShadowAmbient", Src::Role(Role::Shadow)),
    ("colorBackgroundOverlay", Src::Role(Role::Scrim)),
    ("colorStatusDangerForeground1", Src::Role(Role::Error)),
    (
        "colorStatusDangerBackground1",
        Src::Role(Role::ErrorContainer),
    ),
    ("colorStatusDangerBorder1", Src::Role(Role::Error)),
    ("colorStatusSuccessForeground1", Src::Success),
    ("colorStatusSuccessBackground1", Src::SuccessBackground),
    ("colorStatusSuccessBorder1", Src::Success),
    ("colorStatusWarningForeground1", Src::Warning),
    ("colorStatusWarningBackground1", Src::WarningBackground),
    ("colorStatusWarningBorder1", Src::Warning),
];

/// Fluent alias token names written per mode.
#[must_use]
pub fn token_names() -> Vec<&'static str> {
    TOKENS.iter().map(|t| t.0).collect()
}

fn mode(out: &mut String, s: &DynamicScheme) {
    let st = status(s);
    for (i, (name, src)) in TOKENS.iter().enumerate() {
        let argb = match src {
            Src::Role(role) => s.argb(*role),
            Src::Success => st.success.color,
            Src::SuccessBackground => st.success.container,
            Src::Warning => st.warning.color,
            Src::WarningBackground => st.warning.container,
        };
        let comma = if i + 1 == TOKENS.len() { "" } else { "," };
        let _ = writeln!(out, "    \"{name}\": \"{}\"{comma}", hex(argb));
    }
}

/// `fluent-theme.json`.
#[must_use]
pub fn theme_json(src: &ThemeSource) -> String {
    let mut out = String::from("{\n");
    let _ = writeln!(out, "  \"name\": {},", quoted(&src.name));
    out.push_str("  \"generator\": \"yolo-emit (yolo-tokens HCT)\",\n  \"brand\": {\n");
    for step in 1..=16u32 {
        // 16 evenly spaced tones from 4 (darkest) to 96 (lightest).
        let tone = 4.0 + f64::from(step - 1) * (92.0 / 15.0);
        let comma = if step == 16 { "" } else { "," };
        let _ = writeln!(
            out,
            "    \"{}\": \"{}\"{comma}",
            step * 10,
            hex(primary(&src.light, tone))
        );
    }
    out.push_str("  },\n  \"light\": {\n");
    mode(&mut out, &src.light);
    out.push_str("  },\n  \"dark\": {\n");
    mode(&mut out, &src.dark);
    out.push_str("  }\n}\n");
    out
}
