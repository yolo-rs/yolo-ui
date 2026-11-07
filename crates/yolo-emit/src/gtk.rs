// SPDX-License-Identifier: Apache-2.0
//! GTK 4 / libadwaita targets.
//!
//! * `gtk.css`: libadwaita 1.6+ CSS variables (`--accent-bg-color`, `--window-bg-color`, ...)
//!   for both modes, switched by `prefers-color-scheme`, plus M3 component classes. This is
//!   the file to install as `~/.config/gtk-4.0/gtk.css`.
//! * `gtk-light.css` / `gtk-dark.css`: the same palette as libadwaita named colors
//!   (`@define-color accent_bg_color ...`), for GTK 4 applications without libadwaita and for
//!   GTK 3 (`~/.config/gtk-3.0/gtk.css` and `gtk-dark.css`).

use crate::extended::status;
use crate::{EmittedFile, ThemeSource, c, hex, rgb};
use std::fmt::Write;
use yolo_tokens::role::Role;
use yolo_tokens::scheme::DynamicScheme;

/// A color source for one libadwaita name.
#[derive(Clone, Copy)]
enum Src {
    Role(Role),
    Success,
    OnSuccess,
    Warning,
    OnWarning,
    /// A role at a fixed opacity (shades and borders).
    Alpha(Role, f32),
}

/// libadwaita 1.6 named colors (underscore form; CSS variables use dashes).
const NAMED: &[(&str, Src)] = &[
    ("accent_color", Src::Role(Role::Primary)),
    ("accent_bg_color", Src::Role(Role::Primary)),
    ("accent_fg_color", Src::Role(Role::OnPrimary)),
    ("destructive_color", Src::Role(Role::Error)),
    ("destructive_bg_color", Src::Role(Role::Error)),
    ("destructive_fg_color", Src::Role(Role::OnError)),
    ("success_color", Src::Success),
    ("success_bg_color", Src::Success),
    ("success_fg_color", Src::OnSuccess),
    ("warning_color", Src::Warning),
    ("warning_bg_color", Src::Warning),
    ("warning_fg_color", Src::OnWarning),
    ("error_color", Src::Role(Role::Error)),
    ("error_bg_color", Src::Role(Role::Error)),
    ("error_fg_color", Src::Role(Role::OnError)),
    ("window_bg_color", Src::Role(Role::Surface)),
    ("window_fg_color", Src::Role(Role::OnSurface)),
    ("view_bg_color", Src::Role(Role::SurfaceContainerLowest)),
    ("view_fg_color", Src::Role(Role::OnSurface)),
    ("headerbar_bg_color", Src::Role(Role::SurfaceContainer)),
    ("headerbar_fg_color", Src::Role(Role::OnSurface)),
    ("headerbar_border_color", Src::Role(Role::OnSurface)),
    ("headerbar_backdrop_color", Src::Role(Role::Surface)),
    ("headerbar_shade_color", Src::Alpha(Role::Shadow, 0.12)),
    (
        "headerbar_darker_shade_color",
        Src::Alpha(Role::Shadow, 0.24),
    ),
    ("sidebar_bg_color", Src::Role(Role::SurfaceContainerLow)),
    ("sidebar_fg_color", Src::Role(Role::OnSurface)),
    ("sidebar_backdrop_color", Src::Role(Role::Surface)),
    ("sidebar_shade_color", Src::Alpha(Role::Shadow, 0.08)),
    ("sidebar_border_color", Src::Role(Role::OutlineVariant)),
    (
        "secondary_sidebar_bg_color",
        Src::Role(Role::SurfaceContainer),
    ),
    ("secondary_sidebar_fg_color", Src::Role(Role::OnSurface)),
    ("secondary_sidebar_backdrop_color", Src::Role(Role::Surface)),
    (
        "secondary_sidebar_shade_color",
        Src::Alpha(Role::Shadow, 0.08),
    ),
    (
        "secondary_sidebar_border_color",
        Src::Role(Role::OutlineVariant),
    ),
    ("card_bg_color", Src::Role(Role::SurfaceContainer)),
    ("card_fg_color", Src::Role(Role::OnSurface)),
    ("card_shade_color", Src::Alpha(Role::Shadow, 0.08)),
    ("dialog_bg_color", Src::Role(Role::SurfaceContainerHigh)),
    ("dialog_fg_color", Src::Role(Role::OnSurface)),
    ("popover_bg_color", Src::Role(Role::SurfaceContainerHigh)),
    ("popover_fg_color", Src::Role(Role::OnSurface)),
    ("popover_shade_color", Src::Alpha(Role::Shadow, 0.14)),
    (
        "thumbnail_bg_color",
        Src::Role(Role::SurfaceContainerHighest),
    ),
    ("thumbnail_fg_color", Src::Role(Role::OnSurface)),
    ("shade_color", Src::Alpha(Role::Shadow, 0.20)),
    ("scrollbar_outline_color", Src::Role(Role::Outline)),
    ("border_color", Src::Role(Role::OutlineVariant)),
];

/// Extra M3 roles exposed for the component classes.
const M3_VARS: &[(&str, Role)] = &[
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

fn value(s: &DynamicScheme, src: Src) -> String {
    let st = status(s);
    match src {
        Src::Role(role) => c(s, role),
        Src::Success => hex(st.success.color),
        Src::OnSuccess => hex(st.success.on_color),
        Src::Warning => hex(st.warning.color),
        Src::OnWarning => hex(st.warning.on_color),
        Src::Alpha(role, a) => {
            let (r, g, b) = rgb(s.argb(role));
            format!("rgba({r}, {g}, {b}, {a:.2})")
        }
    }
}

fn vars(out: &mut String, s: &DynamicScheme, indent: &str) {
    for (name, src) in NAMED {
        let _ = writeln!(
            out,
            "{indent}--{}: {};",
            name.replace('_', "-"),
            value(s, *src)
        );
    }
    for (name, role) in M3_VARS {
        let _ = writeln!(out, "{indent}--{name}: {};", c(s, *role));
    }
}

const COMPONENTS: &str = concat!(
    "button.m3-tonal { background-color: var(--m3-secondary-container); color: var(--m3-on-secondary-container); }\n",
    "button.m3-outlined { background: none; box-shadow: inset 0 0 0 1px var(--m3-outline); }\n",
    "button.m3-filled, button.suggested-action { border-radius: 9999px; }\n",
    ".toast { background-color: var(--m3-inverse-surface); color: var(--m3-inverse-on-surface); }\n",
);

/// The adaptive libadwaita stylesheet (`gtk.css`).
#[must_use]
pub fn stylesheet(src: &ThemeSource) -> String {
    let mut out = format!(
        "/* Generated by yolo-emit for GTK 4 / libadwaita 1.6+ ({}) from yolo-tokens roles. Do not hand-edit. */\n:root {{\n",
        src.name
    );
    vars(&mut out, &src.light, "  ");
    out.push_str("}\n@media (prefers-color-scheme: dark) {\n  :root {\n");
    vars(&mut out, &src.dark, "    ");
    out.push_str("  }\n}\n");
    out.push_str(COMPONENTS);
    out
}

/// One mode as `@define-color` named colors.
#[must_use]
pub fn define_colors(src: &ThemeSource, dark: bool) -> String {
    let s = src.scheme(dark);
    let mut out = format!(
        "/* Generated by yolo-emit for GTK ({}, {}) from yolo-tokens roles. Do not hand-edit. */\n",
        src.name,
        if dark { "dark" } else { "light" }
    );
    for (name, source) in NAMED {
        let _ = writeln!(out, "@define-color {name} {};", value(s, *source));
    }
    for (name, role) in M3_VARS {
        let _ = writeln!(
            out,
            "@define-color {} {};",
            name.replace('-', "_"),
            c(s, *role)
        );
    }
    out.push_str(concat!(
        "button.suggested-action { background-color: @accent_bg_color; color: @accent_fg_color; }\n",
        "button.destructive-action { background-color: @destructive_bg_color; color: @destructive_fg_color; }\n",
        "button.m3-tonal { background-color: @m3_secondary_container; color: @m3_on_secondary_container; }\n",
        ".toast { background-color: @m3_inverse_surface; color: @m3_inverse_on_surface; }\n",
    ));
    out
}

/// `gtk.css`, `gtk-light.css`, `gtk-dark.css`.
#[must_use]
pub fn files(src: &ThemeSource) -> Vec<EmittedFile> {
    vec![
        EmittedFile::new("gtk.css", stylesheet(src)),
        EmittedFile::new("gtk-light.css", define_colors(src, false)),
        EmittedFile::new("gtk-dark.css", define_colors(src, true)),
    ]
}
