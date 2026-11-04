#![doc = include_str!("../README.md")]

use serde::{Deserialize, Serialize};
use std::fmt;

/// Unified normalized RGBA color (`r`, `g`, `b`, `a` in `0.0..=1.0`) shared across
/// Dioxus CSS, Zed/Longbridge `gpui::Rgba`, `iced_core::Color`, and Tauri `window-vibrancy`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RgbaColor {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl RgbaColor {
    pub const TRANSPARENT: Self = Self::new(0.0, 0.0, 0.0, 0.0);
    pub const BLACK: Self = Self::new(0.0, 0.0, 0.0, 1.0);
    pub const WHITE: Self = Self::new(1.0, 1.0, 1.0, 1.0);

    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub const fn from_rgb(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b, a: 1.0 }
    }

    pub const fn from_rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub const fn from_u8(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self {
            r: (r as f32) / 255.0,
            g: (g as f32) / 255.0,
            b: (b as f32) / 255.0,
            a: (a as f32) / 255.0,
        }
    }

    pub const fn from_rgb8(r: u8, g: u8, b: u8) -> Self {
        Self::from_u8(r, g, b, 255)
    }

    pub const fn from_rgba8(r: u8, g: u8, b: u8, a: f32) -> Self {
        Self {
            r: (r as f32) / 255.0,
            g: (g as f32) / 255.0,
            b: (b as f32) / 255.0,
            a,
        }
    }

    /// Parses `#RRGGBB` or `#RRGGBBAA` hex color strings.
    pub fn from_hex(hex: &str) -> Option<Self> {
        let cleaned = hex.trim().strip_prefix('#').unwrap_or(hex.trim());
        match cleaned.len() {
            6 => {
                let r = u8::from_str_radix(&cleaned[0..2], 16).ok()?;
                let g = u8::from_str_radix(&cleaned[2..4], 16).ok()?;
                let b = u8::from_str_radix(&cleaned[4..6], 16).ok()?;
                Some(Self::from_u8(r, g, b, 255))
            }
            8 => {
                let r = u8::from_str_radix(&cleaned[0..2], 16).ok()?;
                let g = u8::from_str_radix(&cleaned[2..4], 16).ok()?;
                let b = u8::from_str_radix(&cleaned[4..6], 16).ok()?;
                let a = u8::from_str_radix(&cleaned[6..8], 16).ok()?;
                Some(Self::from_u8(r, g, b, a))
            }
            _ => None,
        }
    }

    /// Parses `#RRGGBB`, `#RRGGBBAA`, or `rgba(r, g, b, a)` CSS strings.
    pub fn parse_css(input: &str) -> Option<Self> {
        let s = input.trim();
        if s.starts_with('#') {
            Self::from_hex(s)
        } else if let Some(inner) = s.strip_prefix("rgba(").and_then(|v| v.strip_suffix(')')) {
            let parts: Vec<&str> = inner.split(',').map(str::trim).collect();
            if parts.len() != 4 {
                return None;
            }
            let r = parts[0].parse::<u8>().ok()?;
            let g = parts[1].parse::<u8>().ok()?;
            let b = parts[2].parse::<u8>().ok()?;
            let a = parts[3].parse::<f32>().ok()?;
            Some(Self::from_rgba8(r, g, b, a.clamp(0.0, 1.0)))
        } else {
            None
        }
    }

    /// Packs the color into a `0xRRGGBBAA` u32 integer used by `gpui::rgba()`.
    pub fn to_u32_rgba(self) -> u32 {
        let r = (self.r.clamp(0.0, 1.0) * 255.0).round() as u32;
        let g = (self.g.clamp(0.0, 1.0) * 255.0).round() as u32;
        let b = (self.b.clamp(0.0, 1.0) * 255.0).round() as u32;
        let a = (self.a.clamp(0.0, 1.0) * 255.0).round() as u32;
        (r << 24) | (g << 16) | (b << 8) | a
    }

    /// Formats the color as a `#rrggbbaa` 8-digit CSS hex string.
    pub fn to_css_hex(self) -> String {
        format!("#{:08x}", self.to_u32_rgba())
    }

    /// Formats the color as `#rrggbb` when opaque or `rgba(r, g, b, a)` when translucent.
    pub fn to_css_string(self) -> String {
        let r = (self.r.clamp(0.0, 1.0) * 255.0).round() as u8;
        let g = (self.g.clamp(0.0, 1.0) * 255.0).round() as u8;
        let b = (self.b.clamp(0.0, 1.0) * 255.0).round() as u8;
        if (self.a - 1.0).abs() < f32::EPSILON {
            format!("#{r:02x}{g:02x}{b:02x}")
        } else {
            format!("rgba({r}, {g}, {b}, {:.2})", self.a.clamp(0.0, 1.0))
        }
    }

    /// Linearly interpolates between `self` and `other` by factor `t` (`0.0..=1.0`).
    pub fn mix(self, other: Self, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        Self {
            r: self.r + (other.r - self.r) * t,
            g: self.g + (other.g - self.g) * t,
            b: self.b + (other.b - self.b) * t,
            a: self.a + (other.a - self.a) * t,
        }
    }
}

impl fmt::Display for RgbaColor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_css_string())
    }
}

/// Aphrody Labs Material 3 Dark-Glass Design Tokens shared across all UI engines.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AphrodyThemeTokens {
    pub name: &'static str,
    pub bg_primary: &'static str,
    pub bg_surface: &'static str,
    pub bg_elevated: &'static str,
    pub text_primary: &'static str,
    pub text_secondary: &'static str,
    pub accent_primary: &'static str,
    pub accent_secondary: &'static str,
    pub border_subtle: &'static str,
}

/// Canonical Aphrody Material 3 Dark-Glass token constant.
pub const APHRODY_DARK_THEME: AphrodyThemeTokens = AphrodyThemeTokens {
    name: "aphrody-m3-dark",
    bg_primary: "#090b10",
    bg_surface: "#111520",
    bg_elevated: "#181e2e",
    text_primary: "#f3f6fc",
    text_secondary: "#9ba6bf",
    accent_primary: "#7c5cff",
    accent_secondary: "#00d4ff",
    border_subtle: "rgba(255, 255, 255, 0.08)",
};

/// Generates the CSS `:root` custom properties block for any [`AphrodyThemeTokens`] palette.
pub fn css_variables(tokens: &AphrodyThemeTokens) -> String {
    format!(
        ":root {{ \
            --yolo-bg-primary: {}; \
            --yolo-bg-surface: {}; \
            --yolo-bg-elevated: {}; \
            --yolo-text-primary: {}; \
            --yolo-text-secondary: {}; \
            --yolo-accent-primary: {}; \
            --yolo-accent-secondary: {}; \
            --yolo-border-subtle: {}; \
        }}",
        tokens.bg_primary,
        tokens.bg_surface,
        tokens.bg_elevated,
        tokens.text_primary,
        tokens.text_secondary,
        tokens.accent_primary,
        tokens.accent_secondary,
        tokens.border_subtle,
    )
}

/// Upstream organization classification in the 70-submodule Yolo UI fleet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EcosystemOrg {
    DioxusLabs,
    Longbridge,
    ZedIndustries,
    IcedRs,
    TauriApps,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rgba_color_parsing_and_packing() {
        let c = RgbaColor::parse_css(APHRODY_DARK_THEME.accent_primary).unwrap();
        assert_eq!(c.to_u32_rgba(), 0x7c5cff_ff);
        assert_eq!(c.to_css_string(), "#7c5cff");

        let border = RgbaColor::parse_css(APHRODY_DARK_THEME.border_subtle).unwrap();
        assert_eq!(border.to_css_string(), "rgba(255, 255, 255, 0.08)");
    }

    #[test]
    fn test_css_variables_generation() {
        let css = css_variables(&APHRODY_DARK_THEME);
        assert!(css.contains("--yolo-bg-primary: #090b10;"));
        assert!(css.contains("--yolo-accent-primary: #7c5cff;"));
    }
}
