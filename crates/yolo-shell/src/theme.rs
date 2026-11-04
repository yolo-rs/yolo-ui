// SPDX-License-Identifier: MIT OR Apache-2.0
//! Light/dark and accent color detection across Windows, macOS, and Linux/BSD,
//! with preset seed mapping for all 5 [`YoloThemePreset`]s.

use serde::{Deserialize, Serialize};
use yolo_shared::YoloThemePreset;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ColorScheme {
    Light,
    Dark,
    #[default]
    Unknown,
}

impl ColorScheme {
    pub fn is_dark(self) -> bool {
        self == ColorScheme::Dark
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// Constructs an [`Rgb`] from a `0xAARRGGBB` integer.
    pub const fn from_argb(argb: u32) -> Self {
        Self {
            r: ((argb >> 16) & 0xFF) as u8,
            g: ((argb >> 8) & 0xFF) as u8,
            b: (argb & 0xFF) as u8,
        }
    }

    /// Opaque `0xAARRGGBB`, the seed format of Material color utilities.
    pub const fn argb(self) -> u32 {
        0xff00_0000 | (self.r as u32) << 16 | (self.g as u32) << 8 | (self.b as u32)
    }

    pub fn hex(self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }

    /// Parses `#rrggbb`, `rrggbb` or `r,g,b`.
    pub fn parse(value: &str) -> Option<Self> {
        let value = value.trim();
        if let Some((r, rest)) = value.split_once(',') {
            let mut it = rest.split(',');
            let g = it.next()?.trim().parse().ok()?;
            let b = it.next()?.trim().parse().ok()?;
            return Some(Self::new(r.trim().parse().ok()?, g, b));
        }
        let hex = value.trim_start_matches('#');
        if hex.len() != 6 {
            return None;
        }
        let n = u32::from_str_radix(hex, 16).ok()?;
        Some(Self::new((n >> 16) as u8, (n >> 8) as u8, n as u8))
    }
}

/// Detected system appearance.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemTheme {
    pub scheme: ColorScheme,
    pub accent: Option<Rgb>,
}

/// Default neutral indigo seed (`#6366F1`) when the platform reports no accent.
pub const DEFAULT_ACCENT: Rgb = Rgb::from_argb(YoloThemePreset::Neutral.seed_argb());

impl SystemTheme {
    /// Creates a [`SystemTheme`] seeded by one of the 5 [`YoloThemePreset`]s.
    pub const fn for_preset(preset: YoloThemePreset, dark: bool) -> Self {
        Self {
            scheme: if dark {
                ColorScheme::Dark
            } else {
                ColorScheme::Light
            },
            accent: Some(Rgb::from_argb(preset.seed_argb())),
        }
    }

    pub fn accent_or_default(&self) -> Rgb {
        self.accent.unwrap_or(DEFAULT_ACCENT)
    }

    pub fn accent_or_preset(&self, preset: YoloThemePreset) -> Rgb {
        self.accent
            .unwrap_or_else(|| Rgb::from_argb(preset.seed_argb()))
    }
}

/// Detects the current system scheme and accent.
pub fn detect() -> SystemTheme {
    platform::detect()
}

#[allow(dead_code)]
pub(crate) fn command_output(program: &str, args: &[&str]) -> Option<String> {
    let output = std::process::Command::new(program).args(args).output().ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

#[cfg(windows)]
mod platform {
    use super::{ColorScheme, Rgb, SystemTheme, command_output};

    fn query_reg_dword(key: &str, value_name: &str) -> Option<u32> {
        let out = command_output("reg", &["query", key, "/v", value_name])?;
        for line in out.lines() {
            let line = line.trim();
            if line.starts_with(value_name)
                && let Some(hex_str) = line.split_whitespace().last()
            {
                let trimmed = hex_str.trim_start_matches("0x").trim_start_matches("0X");
                if let Ok(val) = u32::from_str_radix(trimmed, 16) {
                    return Some(val);
                }
            }
        }
        None
    }

    pub fn detect() -> SystemTheme {
        let personalize =
            "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize";
        let scheme = query_reg_dword(personalize, "AppsUseLightTheme")
            .map(|light| {
                if light == 0 {
                    ColorScheme::Dark
                } else {
                    ColorScheme::Light
                }
            })
            .unwrap_or(ColorScheme::Dark);

        // DWM stores AccentColor as 0xAABBGGRR.
        let accent = query_reg_dword("HKCU\\Software\\Microsoft\\Windows\\DWM", "AccentColor")
            .map(|abgr| Rgb::new(abgr as u8, (abgr >> 8) as u8, (abgr >> 16) as u8));

        SystemTheme { scheme, accent }
    }
}

#[cfg(target_os = "macos")]
mod platform {
    use super::{ColorScheme, Rgb, SystemTheme, command_output};

    pub fn detect() -> SystemTheme {
        let scheme = match command_output("defaults", &["read", "-g", "AppleInterfaceStyle"]) {
            Some(style) if style.eq_ignore_ascii_case("dark") => ColorScheme::Dark,
            _ => ColorScheme::Light,
        };
        let accent = match command_output("defaults", &["read", "-g", "AppleAccentColor"])
            .as_deref()
            .map(str::trim)
        {
            Some("-1") => Rgb::new(0x8c, 0x8c, 0x8c),
            Some("0") => Rgb::new(0xff, 0x52, 0x57),
            Some("1") => Rgb::new(0xf7, 0x82, 0x1b),
            Some("2") => Rgb::new(0xff, 0xc6, 0x00),
            Some("3") => Rgb::new(0x62, 0xba, 0x46),
            Some("5") => Rgb::new(0xa5, 0x50, 0xa7),
            Some("6") => Rgb::new(0xf7, 0x4f, 0x9e),
            _ => Rgb::new(0x00, 0x7a, 0xff),
        };
        SystemTheme {
            scheme,
            accent: Some(accent),
        }
    }
}

#[cfg(not(any(windows, target_os = "macos")))]
mod platform {
    use super::{ColorScheme, Rgb, SystemTheme, command_output};

    fn gsettings(key: &str) -> Option<String> {
        command_output("gsettings", &["get", "org.gnome.desktop.interface", key])
            .map(|v| v.trim_matches('\'').to_owned())
    }

    fn gnome_accent(name: &str) -> Option<Rgb> {
        Some(match name {
            "blue" => Rgb::new(0x35, 0x84, 0xe4),
            "teal" => Rgb::new(0x21, 0x90, 0xa4),
            "green" => Rgb::new(0x3a, 0x94, 0x4a),
            "yellow" => Rgb::new(0xc8, 0x88, 0x00),
            "orange" => Rgb::new(0xed, 0x5b, 0x00),
            "red" => Rgb::new(0xe6, 0x2d, 0x42),
            "pink" => Rgb::new(0xd5, 0x61, 0x99),
            "purple" => Rgb::new(0x91, 0x41, 0xac),
            "slate" => Rgb::new(0x6f, 0x83, 0x96),
            _ => return None,
        })
    }

    fn kdeglobals() -> Option<(Option<ColorScheme>, Option<Rgb>)> {
        let path = directories::BaseDirs::new()?.config_dir().join("kdeglobals");
        let text = std::fs::read_to_string(path).ok()?;
        let mut section = "";
        let mut scheme = None;
        let mut accent = None;
        for line in text.lines().map(str::trim) {
            if line.starts_with('[') {
                section = line;
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            match (section, key.trim()) {
                ("[General]", "ColorScheme") => {
                    scheme = Some(if value.to_ascii_lowercase().contains("dark") {
                        ColorScheme::Dark
                    } else {
                        ColorScheme::Light
                    });
                }
                ("[General]", "AccentColor") => accent = Rgb::parse(value),
                _ => {}
            }
        }
        Some((scheme, accent))
    }

    pub fn detect() -> SystemTheme {
        let mut theme = SystemTheme::default();
        if let Ok(gtk_theme) = std::env::var("GTK_THEME") {
            theme.scheme = if gtk_theme.to_ascii_lowercase().contains("dark") {
                ColorScheme::Dark
            } else {
                ColorScheme::Light
            };
        }
        let desktop = std::env::var("XDG_CURRENT_DESKTOP")
            .unwrap_or_default()
            .to_ascii_uppercase();
        if desktop.contains("KDE")
            && let Some((scheme, accent)) = kdeglobals()
        {
            if theme.scheme == ColorScheme::Unknown {
                theme.scheme = scheme.unwrap_or_default();
            }
            theme.accent = accent;
            return theme;
        }
        if theme.scheme == ColorScheme::Unknown {
            theme.scheme = match gsettings("color-scheme").as_deref() {
                Some("prefer-dark") => ColorScheme::Dark,
                Some("prefer-light") | Some("default") => ColorScheme::Light,
                _ => match gsettings("gtk-theme") {
                    Some(name) if name.to_ascii_lowercase().contains("dark") => ColorScheme::Dark,
                    Some(_) => ColorScheme::Light,
                    None => ColorScheme::Unknown,
                },
            };
        }
        theme.accent = gsettings("accent-color")
            .as_deref()
            .and_then(gnome_accent);
        let incomplete = theme.scheme == ColorScheme::Unknown || theme.accent.is_none();
        if incomplete
            && let Some((scheme, accent)) = kdeglobals()
        {
            if theme.scheme == ColorScheme::Unknown {
                theme.scheme = scheme.unwrap_or_default();
            }
            theme.accent = theme.accent.or(accent);
        }
        theme
    }
}
