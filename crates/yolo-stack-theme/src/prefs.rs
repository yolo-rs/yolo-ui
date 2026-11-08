//! What the user changes on top of a bundle. Stored in a cookie and in localStorage.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use yolo_tokens::scheme::Variant;

/// Light, dark or follow the system.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SchemePref {
    /// Follow `prefers-color-scheme`.
    #[default]
    System,
    /// Always light.
    Light,
    /// Always dark.
    Dark,
}

/// Which wallpaper set to show.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WallpaperChoice {
    /// First set of the bundle.
    #[default]
    Auto,
    /// A set by id.
    Id(String),
    /// No wallpaper.
    Off,
}

/// User preferences. Every field is optional or has a neutral default, so an empty value
/// reproduces the bundle as shipped.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct UserThemePrefs {
    /// Selected bundle id.
    pub theme: Option<String>,
    /// Seed hue in degrees, 0..360. `None` keeps the bundle hue.
    pub hue: Option<f64>,
    /// Seed chroma, 0..150 (HCT). `None` keeps the bundle chroma.
    pub chroma: Option<f64>,
    /// Contrast level, -1..=1.
    pub contrast: Option<f64>,
    /// Scheme variant name (`tonal-spot`, `vibrant`, `expressive`, `fidelity`, `monochrome`, ...).
    pub variant: Option<String>,
    /// Light, dark or system.
    pub scheme: SchemePref,
    /// Reduce motion regardless of the OS setting.
    pub reduced_motion: bool,
    /// Show the companion pet.
    pub pet: bool,
    /// Wallpaper selection.
    pub wallpaper: WallpaperChoice,
    /// Per-role overrides (`role-name` to `#rrggbb`). Re-checked against the contrast gates.
    pub overrides: BTreeMap<String, String>,
}

impl Default for UserThemePrefs {
    fn default() -> Self {
        Self {
            theme: None,
            hue: None,
            chroma: None,
            contrast: None,
            variant: None,
            scheme: SchemePref::System,
            reduced_motion: false,
            pet: true,
            wallpaper: WallpaperChoice::Auto,
            overrides: BTreeMap::new(),
        }
    }
}

impl UserThemePrefs {
    /// Clamps numbers into their ranges and drops unusable entries, so any stored value is safe.
    #[must_use]
    pub fn sanitized(mut self) -> Self {
        self.hue = self
            .hue
            .filter(|h| h.is_finite())
            .map(|h| h.rem_euclid(360.0));
        self.chroma = self
            .chroma
            .filter(|c| c.is_finite())
            .map(|c| c.clamp(0.0, 150.0));
        self.contrast = self
            .contrast
            .filter(|c| c.is_finite())
            .map(|c| c.clamp(-1.0, 1.0));
        self.variant = self.variant.filter(|v| Variant::from_name(v).is_some());
        self.overrides.retain(|role, value| {
            yolo_tokens::role::Role::from_name(role).is_some() && is_hex(value)
        });
        self
    }

    /// Compact cookie value: percent-encoded JSON.
    #[must_use]
    pub fn to_cookie_value(&self) -> String {
        let json = serde_json::to_string(self).unwrap_or_else(|_| "{}".to_owned());
        percent_encode(&json)
    }

    /// Parses a cookie value written by [`Self::to_cookie_value`]. Garbage gives the defaults.
    #[must_use]
    pub fn from_cookie_value(value: &str) -> Self {
        percent_decode(value)
            .and_then(|json| serde_json::from_str::<Self>(&json).ok())
            .unwrap_or_default()
            .sanitized()
    }
}

/// `#rrggbb` check.
#[must_use]
pub fn is_hex(value: &str) -> bool {
    value.len() == 7 && value.starts_with('#') && value[1..].bytes().all(|b| b.is_ascii_hexdigit())
}

fn percent_encode(text: &str) -> String {
    let mut out = String::with_capacity(text.len() * 2);
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            out.push(char::from(byte));
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

fn percent_decode(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = text.get(i + 1..i + 3)?;
            out.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cookie_round_trip_and_garbage() {
        let mut prefs = UserThemePrefs {
            theme: Some("kame".into()),
            hue: Some(400.0),
            chroma: Some(900.0),
            contrast: Some(-3.0),
            variant: Some("vibrant".into()),
            ..UserThemePrefs::default()
        };
        prefs.overrides.insert("primary".into(), "#112233".into());
        prefs.overrides.insert("bogus".into(), "#112233".into());
        let back = UserThemePrefs::from_cookie_value(&prefs.to_cookie_value());
        assert_eq!(back.theme.as_deref(), Some("kame"));
        assert_eq!(back.hue, Some(40.0));
        assert_eq!(back.chroma, Some(150.0));
        assert_eq!(back.contrast, Some(-1.0));
        assert_eq!(back.overrides.len(), 1);
        assert_eq!(
            UserThemePrefs::from_cookie_value("%zz"),
            UserThemePrefs::default()
        );
    }
}
