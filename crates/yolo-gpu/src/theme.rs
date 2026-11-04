// SPDX-License-Identifier: Apache-2.0

//! Theme-aware SVG recoloring: `currentColor` on the root, M3 / Yolo semantic role names
//! (`fill="primary"`, `stroke="var(--m3-on-surface)"`, `stop-color:var(--m3-tertiary)`) resolved
//! to `#rrggbb` against any [`ColorRoles`] palette or [`YoloThemePreset`], and an optional palette
//! map for authored source colors.

use std::collections::BTreeMap;

use roxmltree::Node;
use yolo_shared::YoloThemePreset;
use yolo_tokens::color::{BASELINE, BASELINE_DARK, ColorRoles};

const fn argb_to_rgb(argb: u32) -> (u8, u8, u8) {
    (
        ((argb >> 16) & 0xFF) as u8,
        ((argb >> 8) & 0xFF) as u8,
        (argb & 0xFF) as u8,
    )
}

use crate::{
    Result,
    xml::{self, Rules},
};

const PAINT_ATTRIBUTES: &[&str] = &["fill", "stroke", "stop-color", "flood-color", "lighting-color"];

/// Recoloring rules.
#[derive(Debug, Clone)]
pub struct ThemeOptions {
    /// Set `fill="currentColor"` on `<svg>` when the root has no `fill` yet.
    pub set_root_current_color: bool,
    /// Resolve role names and `var(--m3-<role>)` to hex colors from this scheme.
    pub scheme: Option<ColorRoles>,
    /// Case-insensitive `from -> to` color replacements (`#000` -> `currentColor` or `#6750a4`).
    pub color_map: BTreeMap<String, String>,
}

impl Default for ThemeOptions {
    fn default() -> Self {
        Self {
            set_root_current_color: true,
            scheme: None,
            color_map: BTreeMap::new(),
        }
    }
}

impl ThemeOptions {
    /// Baseline light scheme (`#6750A4` seed).
    #[must_use]
    pub fn m3_light() -> Self {
        Self {
            scheme: Some(BASELINE),
            ..Self::default()
        }
    }

    /// Baseline dark scheme (`#6750A4` seed).
    #[must_use]
    pub fn m3_dark() -> Self {
        Self {
            scheme: Some(BASELINE_DARK),
            ..Self::default()
        }
    }

    /// Creates [`ThemeOptions`] for any of the 7 [`YoloThemePreset`]s (`Neutral`, `M3`, `Fluent`, `Apple`, `Omarchy`, `Hyprland`, `Aphrody`).
    #[must_use]
    pub fn for_preset(preset: YoloThemePreset, dark: bool) -> Self {
        use yolo_tokens::scheme::Role;
        let s = yolo_tokens::presets::scheme_for_preset(preset, dark);
        let roles = ColorRoles {
            primary: s.argb(Role::Primary),
            on_primary: s.argb(Role::OnPrimary),
            primary_container: s.argb(Role::PrimaryContainer),
            on_primary_container: s.argb(Role::OnPrimaryContainer),
            secondary: s.argb(Role::Secondary),
            on_secondary: s.argb(Role::OnSecondary),
            secondary_container: s.argb(Role::SecondaryContainer),
            on_secondary_container: s.argb(Role::OnSecondaryContainer),
            tertiary: s.argb(Role::Tertiary),
            on_tertiary: s.argb(Role::OnTertiary),
            tertiary_container: s.argb(Role::TertiaryContainer),
            on_tertiary_container: s.argb(Role::OnTertiaryContainer),
            error: s.argb(Role::Error),
            on_error: s.argb(Role::OnError),
            error_container: s.argb(Role::ErrorContainer),
            on_error_container: s.argb(Role::OnErrorContainer),
            background: s.argb(Role::Background),
            on_background: s.argb(Role::OnBackground),
            surface: s.argb(Role::Surface),
            on_surface: s.argb(Role::OnSurface),
            surface_variant: s.argb(Role::SurfaceVariant),
            on_surface_variant: s.argb(Role::OnSurfaceVariant),
            outline: s.argb(Role::Outline),
            outline_variant: s.argb(Role::OutlineVariant),
            shadow: s.argb(Role::Shadow),
            scrim: s.argb(Role::Scrim),
            inverse_surface: s.argb(Role::InverseSurface),
            inverse_on_surface: s.argb(Role::InverseOnSurface),
            inverse_primary: s.argb(Role::InversePrimary),
            surface_dim: s.argb(Role::SurfaceDim),
            surface_bright: s.argb(Role::SurfaceBright),
            surface_container_lowest: s.argb(Role::SurfaceContainerLowest),
            surface_container_low: s.argb(Role::SurfaceContainerLow),
            surface_container: s.argb(Role::SurfaceContainer),
            surface_container_high: s.argb(Role::SurfaceContainerHigh),
            surface_container_highest: s.argb(Role::SurfaceContainerHighest),
        };
        Self {
            scheme: Some(roles),
            ..Self::default()
        }
    }
}

/// `#rrggbb` for a semantic role name (`primary`, `on-surface-variant`, `surface_container_high`,
/// or `var(--m3-primary)`).
#[must_use]
pub fn role_hex(roles: &ColorRoles, name: &str) -> Option<String> {
    let trimmed = name.trim();
    let role = trimmed
        .strip_prefix("var(")
        .and_then(|s| s.strip_suffix(')'))
        .map(str::trim)
        .and_then(|s| s.strip_prefix("--m3-").or_else(|| s.strip_prefix("--md-sys-color-")))
        .unwrap_or(trimmed);
    let key = role.to_ascii_lowercase().replace('_', "-");
    let argb = match key.as_str() {
        "primary" => roles.primary,
        "on-primary" => roles.on_primary,
        "primary-container" => roles.primary_container,
        "on-primary-container" => roles.on_primary_container,
        "secondary" => roles.secondary,
        "on-secondary" => roles.on_secondary,
        "secondary-container" => roles.secondary_container,
        "on-secondary-container" => roles.on_secondary_container,
        "tertiary" => roles.tertiary,
        "on-tertiary" => roles.on_tertiary,
        "tertiary-container" => roles.tertiary_container,
        "on-tertiary-container" => roles.on_tertiary_container,
        "error" => roles.error,
        "on-error" => roles.on_error,
        "error-container" => roles.error_container,
        "on-error-container" => roles.on_error_container,
        "background" => roles.background,
        "on-background" => roles.on_background,
        "surface" => roles.surface,
        "on-surface" => roles.on_surface,
        "surface-variant" => roles.surface_variant,
        "on-surface-variant" => roles.on_surface_variant,
        "outline" => roles.outline,
        "outline-variant" => roles.outline_variant,
        "shadow" => roles.shadow,
        "scrim" => roles.scrim,
        "inverse-surface" => roles.inverse_surface,
        "inverse-on-surface" => roles.inverse_on_surface,
        "inverse-primary" => roles.inverse_primary,
        "surface-dim" => roles.surface_dim,
        "surface-bright" => roles.surface_bright,
        "surface-container-lowest" => roles.surface_container_lowest,
        "surface-container-low" => roles.surface_container_low,
        "surface-container" => roles.surface_container,
        "surface-container-high" => roles.surface_container_high,
        "surface-container-highest" => roles.surface_container_highest,
        "surface-tint" => roles.primary,
        _ => return None,
    };
    let (r, g, b) = argb_to_rgb(argb);
    Some(format!("#{r:02x}{g:02x}{b:02x}"))
}

fn rewrite_paint(value: &str, options: &ThemeOptions) -> String {
    let trimmed = value.trim();
    if let Some((from, to)) = options
        .color_map
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(trimmed))
    {
        let _ = from;
        return to.clone();
    }
    if let Some(roles) = &options.scheme
        && let Some(hex) = role_hex(roles, trimmed)
    {
        return hex;
    }
    value.to_owned()
}

fn rewrite_style(style: &str, options: &ThemeOptions) -> String {
    let mut out = Vec::new();
    for decl in style.split(';') {
        let Some((prop, val)) = decl.split_once(':') else {
            if !decl.trim().is_empty() {
                out.push(decl.to_owned());
            }
            continue;
        };
        let prop_trim = prop.trim();
        if PAINT_ATTRIBUTES
            .iter()
            .any(|p| p.eq_ignore_ascii_case(prop_trim))
        {
            out.push(format!("{prop_trim}:{}", rewrite_paint(val, options)));
        } else {
            out.push(decl.to_owned());
        }
    }
    out.join(";")
}

struct Themer<'o>(&'o ThemeOptions);

impl Rules for Themer<'_> {
    fn attribute(
        &self,
        _node: Node<'_, '_>,
        local: &str,
        ns: Option<&str>,
        value: &str,
    ) -> Option<String> {
        if ns.is_some() {
            return Some(value.to_owned());
        }
        Some(if PAINT_ATTRIBUTES.contains(&local) {
            rewrite_paint(value, self.0)
        } else if local == "style" {
            rewrite_style(value, self.0)
        } else {
            value.to_owned()
        })
    }

    fn extra_attributes(&self, node: Node<'_, '_>) -> Vec<(String, String)> {
        if node.parent_element().is_none()
            && self.0.set_root_current_color
            && node.attribute("fill").is_none()
        {
            vec![("fill".to_owned(), "currentColor".to_owned())]
        } else {
            Vec::new()
        }
    }

    fn keep_comments(&self) -> bool {
        true
    }
}

/// Applies `options` to `svg`.
///
/// # Errors
/// Malformed XML or non-SVG root.
pub fn apply_theme(svg: &str, options: &ThemeOptions) -> Result<String> {
    let doc = xml::parse(svg)?;
    Ok(xml::write_document(&doc, &Themer(options)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_m3_roles_and_sets_current_color() {
        let mut opts = ThemeOptions::m3_light();
        opts.color_map
            .insert("#000000".into(), "currentColor".into());
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="primary" stroke="var(--m3-on-surface-variant)" style="stop-color:var(--m3-tertiary);opacity:1" d="M0 0"/><circle fill="#000000" r="1"/></svg>"##;
        let out = apply_theme(svg, &opts).expect("ok");
        assert!(out.contains(r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor">"#));
        assert!(out.contains(r##"fill="#6750a4""##), "{out}");
        assert!(out.contains(r##"stroke="#49454f""##), "{out}");
        assert!(out.contains("stop-color:#7d5260;opacity:1"), "{out}");
        assert!(out.contains(r#"<circle fill="currentColor" r="1"/>"#));
    }

    #[test]
    fn supports_all_five_yolo_theme_presets() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="primary" d="M0 0"/></svg>"#;
        for preset in YoloThemePreset::ALL {
            let opts = ThemeOptions::for_preset(preset, true);
            let out = apply_theme(svg, &opts).expect("theme preset");
            assert!(out.contains("fill=\"#"));
        }
    }
}
