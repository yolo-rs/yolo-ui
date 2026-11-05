// SPDX-License-Identifier: Apache-2.0
//! GPUI Kit (`gpui-component`) theme-set JSON.
//!
//! Format source, verified: `submodules/gpui-kit/.theme-schema.json` (JSON Schema draft-07, title
//! `ThemeSet`) and the bundled examples in `submodules/gpui-kit/themes/*.json`. A set is
//! `{ name, author, url, themes: [ { name, mode: "light"|"dark", radius, radius.lg, shadow, colors,
//! highlight } ] }`; `colors` uses flat dotted keys (`primary.background`, `list.active.background`,
//! `base.red`, ...). This module emits one set per preset with a Light and a Dark theme, every
//! colour derived from the preset `DynamicScheme`; hover/active steps are blends toward the
//! foreground. Only keys listed in the schema are written. The `highlight.syntax` table is omitted
//! (optional in the examples); `highlight` carries the editor keys only. Not verified: that the
//! pinned `gpui-component` build accepts every key at runtime (nothing was compiled against it).

use std::fmt::Write as _;

use crate::Theme;
use yolo_shared::YoloThemePreset;
use yolo_tokens::palette::TonalPalette;
use yolo_tokens::role::Role;
use yolo_tokens::scheme::DynamicScheme;

/// Schema URL written to `$schema`.
pub const GPUI_THEME_SCHEMA: &str =
    "https://github.com/longbridge/gpui-kit/raw/refs/heads/main/.theme-schema.json";

fn hex(argb: u32) -> String {
    format!("#{:06x}", argb & 0x00FF_FFFF)
}

fn hex_a(argb: u32, alpha: u8) -> String {
    format!("#{:06x}{alpha:02x}", argb & 0x00FF_FFFF)
}

/// Channel-wise sRGB blend of `base` toward `toward`, `t` in `0..=1`.
fn mix(base: u32, toward: u32, t: f64) -> u32 {
    let ch = |shift: u32| -> u32 {
        let a = f64::from((base >> shift) & 0xFF);
        let b = f64::from((toward >> shift) & 0xFF);
        let v = (a + (b - a) * t).round().clamp(0.0, 255.0);
        // Clamped to 0..=255 above, so the cast is exact.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let v = v as u32;
        v << shift
    };
    0xFF00_0000 | ch(16) | ch(8) | ch(0)
}

/// Fixed hues so semantic and `base.*` colours keep their meaning in every preset.
fn semantic(hue: f64, dark: bool) -> (u32, u32) {
    let p = TonalPalette::from_hue_and_chroma(hue, 60.0);
    if dark {
        (p.tone(70.0), p.tone(85.0))
    } else {
        (p.tone(45.0), p.tone(60.0))
    }
}

fn theme_object(
    out: &mut String,
    name: &str,
    dark: bool,
    s: &DynamicScheme,
    radius: f32,
    radius_lg: f32,
) {
    let r = |role| s.argb(role);
    let bg = r(Role::Background);
    let fg = r(Role::OnBackground);
    let primary = r(Role::Primary);
    let on_primary = r(Role::OnPrimary);
    let secondary = r(Role::SecondaryContainer);
    let on_secondary = r(Role::OnSecondaryContainer);
    let muted = r(Role::SurfaceContainerHigh);
    let panel = r(Role::SurfaceContainerLow);
    let danger = r(Role::Error);
    let (success, success_light) = semantic(145.0, dark);
    let (warning, warning_light) = semantic(75.0, dark);
    let (info, info_light) = semantic(250.0, dark);
    let (magenta, magenta_light) = semantic(320.0, dark);
    let (cyan, cyan_light) = semantic(200.0, dark);
    let (red, red_light) = semantic(25.0, dark);

    let mut c: Vec<(&str, String)> = vec![
        ("accent.background", hex(r(Role::TertiaryContainer))),
        ("accent.foreground", hex(r(Role::OnTertiaryContainer))),
        ("accordion.background", hex(panel)),
        ("accordion.hover.background", hex(muted)),
        ("background", hex(bg)),
        ("border", hex(r(Role::OutlineVariant))),
        ("caret", hex(primary)),
        ("chart.bullish", hex(success)),
        ("chart.bearish", hex(danger)),
        ("chart.grid", hex(r(Role::OutlineVariant))),
        ("danger.background", hex(danger)),
        ("danger.hover.background", hex(mix(danger, fg, 0.08))),
        ("danger.active.background", hex(mix(danger, fg, 0.16))),
        ("danger.foreground", hex(r(Role::OnError))),
        ("description_list.label.background", hex(panel)),
        ("description_list.label.foreground", hex(r(Role::OnSurfaceVariant))),
        ("drag.border", hex(primary)),
        ("drop_target.background", hex_a(primary, 0x22)),
        ("foreground", hex(fg)),
        ("group_box.background", hex(panel)),
        ("group_box.foreground", hex(r(Role::OnSurface))),
        ("group_box.title.foreground", hex(r(Role::OnSurfaceVariant))),
        ("info.background", hex(info)),
        ("info.hover.background", hex(mix(info, fg, 0.08))),
        ("info.active.background", hex(mix(info, fg, 0.16))),
        ("info.foreground", hex(on_primary)),
        ("input.border", hex(r(Role::Outline))),
        ("link", hex(primary)),
        ("link.hover", hex(mix(primary, fg, 0.15))),
        ("link.active", hex(mix(primary, fg, 0.3))),
        ("list.background", hex(bg)),
        ("list.active.background", hex_a(primary, 0x22)),
        ("list.active.border", hex(primary)),
        ("list.even.background", hex(panel)),
        ("list.head.background", hex(muted)),
        ("list.hover.background", hex(mix(bg, fg, 0.06))),
        ("muted.background", hex(muted)),
        ("muted.foreground", hex(r(Role::OnSurfaceVariant))),
        ("popover.background", hex(r(Role::SurfaceContainer))),
        ("popover.foreground", hex(r(Role::OnSurface))),
        ("primary.background", hex(primary)),
        ("primary.hover.background", hex(mix(primary, fg, 0.08))),
        ("primary.active.background", hex(mix(primary, fg, 0.16))),
        ("primary.foreground", hex(on_primary)),
        ("progress.bar.background", hex(primary)),
        ("ring", hex(primary)),
        ("scrollbar.background", hex_a(bg, 0x00)),
        ("scrollbar.thumb.background", hex_a(fg, 0x4c)),
        ("scrollbar.thumb.hover.background", hex_a(fg, 0x80)),
        ("secondary.background", hex(secondary)),
        ("secondary.hover.background", hex(mix(secondary, on_secondary, 0.08))),
        ("secondary.active.background", hex(mix(secondary, on_secondary, 0.16))),
        ("secondary.foreground", hex(on_secondary)),
        ("selection.background", hex_a(primary, 0x4d)),
        ("sidebar.background", hex(panel)),
        ("sidebar.accent.background", hex(secondary)),
        ("sidebar.accent.foreground", hex(on_secondary)),
        ("sidebar.border", hex(r(Role::OutlineVariant))),
        ("sidebar.foreground", hex(r(Role::OnSurface))),
        ("sidebar.primary.background", hex(primary)),
        ("sidebar.primary.foreground", hex(on_primary)),
        ("skeleton.background", hex(muted)),
        ("slider.background", hex(primary)),
        ("slider.thumb.background", hex(on_primary)),
        ("success.background", hex(success)),
        ("success.hover.background", hex(mix(success, fg, 0.08))),
        ("success.active.background", hex(mix(success, fg, 0.16))),
        ("success.foreground", hex(on_primary)),
        ("switch.background", hex(r(Role::Outline))),
        ("switch.thumb.background", hex(on_primary)),
        ("tab.background", hex_a(panel, 0x00)),
        ("tab.active.background", hex(bg)),
        ("tab.active.foreground", hex(fg)),
        ("tab.foreground", hex(r(Role::OnSurfaceVariant))),
        ("tab_bar.background", hex(panel)),
        ("tab_bar.segmented.background", hex(muted)),
        ("table.background", hex(bg)),
        ("table.active.background", hex_a(primary, 0x22)),
        ("table.active.border", hex(primary)),
        ("table.even.background", hex(panel)),
        ("table.head.background", hex(muted)),
        ("table.head.foreground", hex(r(Role::OnSurfaceVariant))),
        ("table.foot.background", hex(muted)),
        ("table.foot.foreground", hex(r(Role::OnSurfaceVariant))),
        ("table.hover.background", hex(mix(bg, fg, 0.06))),
        ("table.row.border", hex(r(Role::OutlineVariant))),
        ("title_bar.background", hex(r(Role::SurfaceContainer))),
        ("title_bar.border", hex(r(Role::OutlineVariant))),
        ("warning.background", hex(warning)),
        ("warning.hover.background", hex(mix(warning, fg, 0.08))),
        ("warning.active.background", hex(mix(warning, fg, 0.16))),
        ("warning.foreground", hex(r(Role::OnSurface))),
        ("overlay", hex_a(r(Role::Scrim), 0x66)),
        ("window.border", hex(r(Role::OutlineVariant))),
        ("base.blue", hex(info)),
        ("base.blue.light", hex(info_light)),
        ("base.cyan", hex(cyan)),
        ("base.cyan.light", hex(cyan_light)),
        ("base.green", hex(success)),
        ("base.green.light", hex(success_light)),
        ("base.magenta", hex(magenta)),
        ("base.magenta.light", hex(magenta_light)),
        ("base.red", hex(red)),
        ("base.red.light", hex(red_light)),
        ("base.yellow", hex(warning)),
        ("base.yellow.light", hex(warning_light)),
    ];
    c.sort_by(|a, b| a.0.cmp(b.0));

    let mode = if dark { "dark" } else { "light" };
    let _ = write!(
        out,
        "    {{\n      \"name\": \"{name}\",\n      \"mode\": \"{mode}\",\n      \"radius\": {radius},\n      \"radius.lg\": {radius_lg},\n      \"shadow\": true,\n      \"colors\": {{\n"
    );
    for (i, (k, v)) in c.iter().enumerate() {
        let comma = if i + 1 == c.len() { "" } else { "," };
        let _ = writeln!(out, "        \"{k}\": \"{v}\"{comma}");
    }
    let _ = write!(
        out,
        "      }},\n      \"highlight\": {{\n        \"editor.background\": \"{}\",\n        \"editor.foreground\": \"{}\",\n        \"editor.active_line.background\": \"{}\",\n        \"editor.line_number\": \"{}\",\n        \"editor.active_line_number\": \"{}\"\n      }}\n    }}",
        hex(bg),
        hex(fg),
        hex(panel),
        hex(r(Role::Outline)),
        hex(fg)
    );
}

/// GPUI Kit theme set JSON for `preset`: one Light and one Dark theme.
#[must_use]
pub fn emit_gpui_theme(preset: YoloThemePreset) -> String {
    let theme = Theme::from_preset(preset);
    let style = preset.style();
    let name = preset.display_name();
    let mut out = format!(
        "{{\n  \"$schema\": \"{GPUI_THEME_SCHEMA}\",\n  \"name\": \"Yolo {name}\",\n  \"author\": \"Yolo UI (yolo-emit), seed {}\",\n  \"themes\": [\n",
        preset.seed_hex()
    );
    theme_object(
        &mut out,
        &format!("Yolo {name} Light"),
        false,
        &theme.scheme(false),
        style.radius_md_px,
        style.radius_lg_px,
    );
    out.push_str(",\n");
    theme_object(
        &mut out,
        &format!("Yolo {name} Dark"),
        true,
        &theme.scheme(true),
        style.radius_md_px,
        style.radius_lg_px,
    );
    out.push_str("\n  ]\n}\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Keys the GPUI Kit examples and schema treat as essential for a usable theme.
    const REQUIRED: [&str; 24] = [
        "background", "foreground", "border", "ring", "input.border", "accent.background",
        "accent.foreground", "muted.background", "muted.foreground", "primary.background",
        "primary.foreground", "primary.hover.background", "primary.active.background",
        "secondary.background", "secondary.foreground", "danger.background", "danger.foreground",
        "popover.background", "popover.foreground", "list.active.background", "sidebar.background",
        "tab.active.background", "title_bar.background", "base.red",
    ];

    #[test]
    fn every_preset_has_light_and_dark_with_required_keys() {
        for preset in YoloThemePreset::ALL {
            let json = emit_gpui_theme(preset);
            assert!(json.contains("\"mode\": \"light\"") && json.contains("\"mode\": \"dark\""));
            for key in REQUIRED {
                assert_eq!(
                    json.matches(&format!("        \"{key}\": \"#")).count(),
                    2,
                    "{} {key}",
                    preset.id()
                );
            }
            assert_eq!(json.matches('{').count(), json.matches('}').count(), "balanced braces");
            assert_eq!(json.matches('[').count(), json.matches(']').count(), "balanced brackets");
            assert!(!json.contains(",\n      }"), "no trailing commas");
        }
    }

    #[test]
    fn presets_differ() {
        assert_ne!(emit_gpui_theme(YoloThemePreset::M3), emit_gpui_theme(YoloThemePreset::Fluent));
    }

    #[test]
    fn mix_endpoints() {
        assert_eq!(mix(0xFF10_2030, 0xFFFF_FFFF, 0.0) & 0xFF_FFFF, 0x10_2030);
        assert_eq!(mix(0xFF10_2030, 0xFFFF_FFFF, 1.0) & 0xFF_FFFF, 0xFF_FFFF);
    }
}
