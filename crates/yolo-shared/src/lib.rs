#![doc = include_str!("../README.md")]

use serde::{Deserialize, Serialize};
use std::fmt;

/// Unified normalized RGBA color (`r`, `g`, `b`, `a` in `0.0..=1.0`) shared across
/// Dioxus CSS, Zed/Longbridge `gpui::Rgba`, `iced_core::Color`, Egui `Color32`, Cosmic, GTK4, Qt6, and Tauri `window-vibrancy`.
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

    /// Creates an [`RgbaColor`] from a `0xAARRGGBB` 32-bit integer.
    pub const fn from_argb32(argb: u32) -> Self {
        let a = ((argb >> 24) & 0xFF) as u8;
        let r = ((argb >> 16) & 0xFF) as u8;
        let g = ((argb >> 8) & 0xFF) as u8;
        let b = (argb & 0xFF) as u8;
        Self::from_u8(r, g, b, a)
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

    /// Packs the color into a `0xAARRGGBB` u32 integer used by M3 HCT and WinUI/Qt palettes.
    pub fn to_u32_argb(self) -> u32 {
        let r = (self.r.clamp(0.0, 1.0) * 255.0).round() as u32;
        let g = (self.g.clamp(0.0, 1.0) * 255.0).round() as u32;
        let b = (self.b.clamp(0.0, 1.0) * 255.0).round() as u32;
        let a = (self.a.clamp(0.0, 1.0) * 255.0).round() as u32;
        (a << 24) | (r << 16) | (g << 8) | b
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

impl Default for RgbaColor {
    fn default() -> Self {
        Self::TRANSPARENT
    }
}

impl fmt::Display for RgbaColor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_css_string())
    }
}

// ============================================================================
// 7 First-Class Built-In Theme Presets (`YoloThemePreset`)
// ============================================================================

/// The 7 first-class built-in theme presets of `yolo-ui`.
///
/// By default, `yolo-ui` uses [`YoloThemePreset::Neutral`] (`"yolo-neutral"`), ensuring
/// zero vendor lock-in while allowing instant switching to M3, Fluent 2, Apple HIG Liquid Glass,
/// Omacom Omarchy (`Tokyo Night` + 23 official subthemes), Hyprland (`#33ccff` -> `#00ff99` active border + kawase blur),
/// or Aphrody (Byron Love).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum YoloThemePreset {
    /// `"yolo-neutral"` — Clean, neutral customizable default theme (zinc/slate surfaces, `#6366f1` indigo accent).
    #[default]
    Neutral,
    /// `"yolo-m3"` — Google Material Design 3 / Jetpack Compose upstream theme (seed `#6750A4`, HCT tonal spot, pill buttons `9999px`).
    M3,
    /// `"yolo-fluent"` — Microsoft Fluent UI 2 / WinUI 3 Design system (brand blue `#0078D4`, Mica/Acrylic vibrancy, `4px`/`8px` radii).
    Fluent,
    /// `"yolo-apple"` — Apple macOS (Sequoia / Tahoe Liquid Glass `NSGlassEffectView`) & iOS (Cupertino / SwiftUI HIG) theme (`#007AFF` system blue).
    Apple,
    /// `"yolo-omarchy"` — Omacom Omarchy Linux & `omarchy-site` TUI-inspired terminal-modern design system (`Tokyo Night` `#1a1b26`/`#7aa2f7`/`#bb9af7` + 23 official subthemes).
    Omarchy,
    /// `"yolo-hyprland"` — Hyprwm / Hyprland / `hyprtoolkit` / `hyprqt6engine` Wayland compositor design system (`#33ccff` -> `#00ff99` 45deg active border, kawase blur).
    Hyprland,
    /// `"yolo-aphrody"` — Signature Byron Love (Afuro Terumi / Aphrody) pet & brand theme (10 sprite-atlas colors, `#17335C` azur seed, `#FFD700` gold, `#7C5CFF` violet).
    Aphrody,
}

impl YoloThemePreset {
    /// All 7 built-in theme presets in canonical order.
    pub const ALL: [Self; 7] = [
        Self::Neutral,
        Self::M3,
        Self::Fluent,
        Self::Apple,
        Self::Omarchy,
        Self::Hyprland,
        Self::Aphrody,
    ];

    /// Canonical kebab-case preset identifier (`"yolo-neutral"`, `"yolo-m3"`, `"yolo-fluent"`, `"yolo-apple"`, `"yolo-omarchy"`, `"yolo-hyprland"`, `"yolo-aphrody"`).
    pub const fn id(self) -> &'static str {
        match self {
            Self::Neutral => "yolo-neutral",
            Self::M3 => "yolo-m3",
            Self::Fluent => "yolo-fluent",
            Self::Apple => "yolo-apple",
            Self::Omarchy => "yolo-omarchy",
            Self::Hyprland => "yolo-hyprland",
            Self::Aphrody => "yolo-aphrody",
        }
    }

    /// Human-readable display label for the theme preset.
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::Neutral => "Yolo Neutral (Customizable Default)",
            Self::M3 => "Google Material Design 3 (Jetpack Compose)",
            Self::Fluent => "Microsoft Fluent UI 2 (WinUI 3 Mica/Acrylic)",
            Self::Apple => "Apple HIG Liquid Glass (macOS Tahoe & iOS Cupertino)",
            Self::Omarchy => "Omacom Omarchy (Tokyo Night & 23 TUI-Modern Subthemes)",
            Self::Hyprland => "Hyprland Wayland Compositor (Cyan-Mint 45deg Border & Kawase Blur)",
            Self::Aphrody => "Aphrody Byron Love (10-Teinte Sprite Atlas & Celestial Gold)",
        }
    }

    /// Canonical opaque `0xAARRGGBB` seed color for dynamic HCT / palette generation.
    pub const fn seed_argb(self) -> u32 {
        match self {
            Self::Neutral => 0xFF63_66F1,
            Self::M3 => 0xFF67_50A4,
            Self::Fluent => 0xFF00_78D4,
            Self::Apple => 0xFF00_7AFF,
            Self::Omarchy => 0xFF7A_A2F7,
            Self::Hyprland => 0xFF33_CCFF,
            Self::Aphrody => 0xFF17_335C,
        }
    }

    /// Canonical `#RRGGBB` hex string for the preset's primary seed color.
    pub const fn seed_hex(self) -> &'static str {
        match self {
            Self::Neutral => "#6366F1",
            Self::M3 => "#6750A4",
            Self::Fluent => "#0078D4",
            Self::Apple => "#007AFF",
            Self::Omarchy => "#7AA2F7",
            Self::Hyprland => "#33CCFF",
            Self::Aphrody => "#17335C",
        }
    }

    /// Parses a preset from its identifier or shorthand (`"neutral"`, `"m3"`, `"fluent"`, `"apple"`, `"omarchy"`, `"hyprland"`, `"aphrody"`).
    pub fn from_id(id: &str) -> Option<Self> {
        match id.trim().to_ascii_lowercase().as_str() {
            "yolo-neutral" | "neutral" | "default" => Some(Self::Neutral),
            "yolo-m3" | "m3" | "material" | "material3" => Some(Self::M3),
            "yolo-fluent" | "fluent" | "fluent2" | "winui" | "winui3" => Some(Self::Fluent),
            "yolo-apple" | "apple" | "macos" | "ios" | "cupertino" | "liquid-glass" => {
                Some(Self::Apple)
            }
            "yolo-omarchy" | "omarchy" | "omacom" | "tokyo-night" => Some(Self::Omarchy),
            "yolo-hyprland" | "hyprland" | "hyprwm" | "hypr" => Some(Self::Hyprland),
            "yolo-aphrody" | "aphrody" | "byron-love" | "byron" => Some(Self::Aphrody),
            _ => None,
        }
    }

    /// Returns the surface & accent token table for this preset in dark or light mode.
    pub const fn tokens(self, dark: bool) -> YoloThemeTokens {
        match (self, dark) {
            (Self::Neutral, true) => YOLO_NEUTRAL_DARK_THEME,
            (Self::Neutral, false) => YOLO_NEUTRAL_LIGHT_THEME,
            (Self::M3, true) => YOLO_M3_DARK_THEME,
            (Self::M3, false) => YOLO_M3_LIGHT_THEME,
            (Self::Fluent, true) => YOLO_FLUENT_DARK_THEME,
            (Self::Fluent, false) => YOLO_FLUENT_LIGHT_THEME,
            (Self::Apple, true) => YOLO_APPLE_DARK_THEME,
            (Self::Apple, false) => YOLO_APPLE_LIGHT_THEME,
            (Self::Omarchy, true) => YOLO_OMARCHY_DARK_THEME,
            (Self::Omarchy, false) => YOLO_OMARCHY_LIGHT_THEME,
            (Self::Hyprland, true) => YOLO_HYPRLAND_DARK_THEME,
            (Self::Hyprland, false) => YOLO_HYPRLAND_LIGHT_THEME,
            (Self::Aphrody, true) => YOLO_APHRODY_DARK_THEME,
            (Self::Aphrody, false) => YOLO_APHRODY_LIGHT_THEME,
        }
    }

    /// Returns the structural geometry, typography, and vibrancy style tokens for this preset.
    pub const fn style(self) -> YoloStyle {
        YoloStyle::for_preset(self)
    }

    /// Alias for [`Self::id`].
    pub const fn as_str(self) -> &'static str {
        self.id()
    }

    /// Alias for [`Self::seed_argb`].
    pub const fn default_seed_argb(self) -> u32 {
        self.seed_argb()
    }
}

// ============================================================================
// Omacom Omarchy (`submodules/omarchy/themes/*/colors.toml` & `omarchy-site`) 23 Official Subthemes
// ============================================================================

/// All 23 official Omarchy subthemes extracted from `submodules/omarchy/themes/*/colors.toml`
/// and `submodules/omarchy-site/src/lib/site-themes.ts`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OmarchySubTheme {
    /// `"tokyo-night"` — Default Omarchy dark theme (`#1a1b26` bg, `#7aa2f7` blue accent, `#bb9af7` bright magenta).
    #[default]
    TokyoNight,
    /// `"catppuccin"` — Catppuccin Mocha dark theme (`#1e1e2e` bg, `#89b4fa` sapphire blue accent, `#f5c2e7` pink).
    Catppuccin,
    /// `"catppuccin-latte"` — Catppuccin Latte light theme (`#eff1f5` bg, `#1e66f5` blue accent, `#ea76cb` pink).
    CatppuccinLatte,
    /// `"ethereal"` — Ethereal deep indigo dark theme (`#060B1E` bg, `#7d82d9` periwinkle accent, `#c89dc1` orchid).
    Ethereal,
    /// `"everforest"` — Everforest dark theme (`#2d353b` bg, `#7fbbb3` aqua accent, `#d699b6` purple).
    Everforest,
    /// `"flexoki-light"` — Flexoki warm paper light theme (`#FFFCF0` bg, `#205EA6` cobalt accent, `#CE5D97` magenta).
    FlexokiLight,
    /// `"gruvbox"` — Gruvbox Material dark theme (`#282828` bg, `#7daea3` blue-aqua accent, `#d3869b` purple).
    Gruvbox,
    /// `"hackerman"` — Hackerman cyber-terminal dark theme (`#0B0C16` bg, `#82FB9C` neon matrix green, `#7cf8f7` cyan).
    Hackerman,
    /// `"jade-horse"` (`"osaka-jade"`) — Osaka Jade / Jade Horse dark theme (`#142620` deep jade bg, `#8bcbb1` jade accent, `#86c7c5` cyan).
    OsakaJade,
    /// `"kanagawa"` — Kanagawa wave dark theme (`#1f1f28` bg, `#dcd7ba` fuji gold accent, `#957fb8` oni violet).
    Kanagawa,
    /// `"last-horizon"` — Last Horizon obsidian dark theme (`#0c0b0c` bg, `#b59790` warm rose-taupe accent, `#c4d8e2` ice).
    LastHorizon,
    /// `"lumon"` — Lumon macrodata refinement dark theme (`#16242d` bg, `#8bc9eb` terminal ice-blue accent, `#b4e4f6` cyan).
    Lumon,
    /// `"lupine"` — Lupine crisp light theme (`#fafafa` bg, `#3264eb` royal blue accent, `#8a4ad7` violet).
    Lupine,
    /// `"matte-black"` — Matte Black minimal dark theme (`#121212` bg, `#e68e0d` amber accent, `#D35F5F` crimson).
    MatteBlack,
    /// `"miasma"` — Miasma earthy olive dark theme (`#222222` bg, `#78824b` moss accent, `#c9a554` ochre).
    Miasma,
    /// `"nord"` — Nord arctic dark theme (`#2e3440` bg, `#81a1c1` frost blue accent, `#88c0d0` frost cyan).
    Nord,
    /// `"retro-82"` — Retro 82 synth-navy dark theme (`#05182e` bg, `#faa968` sunset orange accent, `#8cbfb8` teal).
    Retro82,
    /// `"ristretto"` — Monokai Ristretto espresso dark theme (`#2c2525` bg, `#f38d70` coral accent, `#85dacc` mint).
    Ristretto,
    /// `"rose-pine"` — Rosé Pine Dawn light theme (`#faf4ed` bg, `#56949f` pine teal accent, `#907aa9` iris).
    RosePine,
    /// `"solitude"` — Solitude slate-monochrome dark theme (`#101315` bg, `#798186` steel accent, `#9fa5a9` mist).
    Solitude,
    /// `"starship"` — Starship orbital dark theme (`#1c1c1e` bg, `#ff9f0a` solar amber accent, `#a6dcef` sky cyan).
    Starship,
    /// `"vantablack"` — Vantablack pure `#000000` OLED dark theme (`#000000` bg, `#8d8d8d` titanium accent, `#ffffff` fg).
    Vantablack,
    /// `"white"` — Pure White high-key light theme (`#ffffff` bg, `#6e6e6e` graphite accent, `#000000` ink).
    White,
}

/// Complete palette specification for an official [`OmarchySubTheme`] from `submodules/omarchy/themes/*/colors.toml`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct OmarchySubThemeSpec {
    pub subtheme: OmarchySubTheme,
    pub id: &'static str,
    pub display_name: &'static str,
    pub is_dark: bool,
    pub seed_argb: u32,
    pub background: &'static str,
    pub lighter_background: &'static str,
    pub selection_background: &'static str,
    pub foreground: &'static str,
    pub bright_foreground: &'static str,
    pub accent: &'static str,
    pub accent_secondary: &'static str,
    pub muted_border: &'static str,
    pub red: &'static str,
    pub green: &'static str,
    pub yellow: &'static str,
    pub cyan: &'static str,
}

/// Alias for [`OmarchySubThemeSpec`].
pub type OmarchyThemePalette = OmarchySubThemeSpec;

/// Canonical catalog of all 23 official Omarchy themes (`submodules/omarchy/themes/*/colors.toml`).
pub const OMARCHY_THEMES: [OmarchySubThemeSpec; 23] = [
    OmarchySubThemeSpec {
        subtheme: OmarchySubTheme::TokyoNight,
        id: "tokyo-night",
        display_name: "Tokyo Night",
        is_dark: true,
        seed_argb: 0xFF7A_A2F7,
        background: "#1a1b26",
        lighter_background: "#24283b",
        selection_background: "#292e42",
        foreground: "#a9b1d6",
        bright_foreground: "#c0caf5",
        accent: "#7aa2f7",
        accent_secondary: "#bb9af7",
        muted_border: "#414868",
        red: "#f7768e",
        green: "#9ece6a",
        yellow: "#e0af68",
        cyan: "#449dab",
    },
    OmarchySubThemeSpec {
        subtheme: OmarchySubTheme::Catppuccin,
        id: "catppuccin",
        display_name: "Catppuccin",
        is_dark: true,
        seed_argb: 0xFF89_B4FA,
        background: "#1e1e2e",
        lighter_background: "#313244",
        selection_background: "#45475a",
        foreground: "#cdd6f4",
        bright_foreground: "#cdd6f4",
        accent: "#89b4fa",
        accent_secondary: "#f5c2e7",
        muted_border: "#585b70",
        red: "#f38ba8",
        green: "#a6e3a1",
        yellow: "#f9e2af",
        cyan: "#94e2d5",
    },
    OmarchySubThemeSpec {
        subtheme: OmarchySubTheme::CatppuccinLatte,
        id: "catppuccin-latte",
        display_name: "Catppuccin Latte",
        is_dark: false,
        seed_argb: 0xFF1E_66F5,
        background: "#eff1f5",
        lighter_background: "#dce0e8",
        selection_background: "#ccd0da",
        foreground: "#4c4f69",
        bright_foreground: "#4c4f69",
        accent: "#1e66f5",
        accent_secondary: "#ea76cb",
        muted_border: "#acb0be",
        red: "#d20f39",
        green: "#40a02b",
        yellow: "#df8e1d",
        cyan: "#179299",
    },
    OmarchySubThemeSpec {
        subtheme: OmarchySubTheme::Ethereal,
        id: "ethereal",
        display_name: "Ethereal",
        is_dark: true,
        seed_argb: 0xFF7D_82D9,
        background: "#060B1E",
        lighter_background: "#131a3a",
        selection_background: "#252e56",
        foreground: "#ffcead",
        bright_foreground: "#ffcead",
        accent: "#7d82d9",
        accent_secondary: "#c89dc1",
        muted_border: "#6d7db6",
        red: "#ED5B5A",
        green: "#92a593",
        yellow: "#E9BB4F",
        cyan: "#a3bfd1",
    },
    OmarchySubThemeSpec {
        subtheme: OmarchySubTheme::Everforest,
        id: "everforest",
        display_name: "Everforest",
        is_dark: true,
        seed_argb: 0xFF7F_BBB3,
        background: "#2d353b",
        lighter_background: "#343f44",
        selection_background: "#3d484d",
        foreground: "#d3c6aa",
        bright_foreground: "#d3c6aa",
        accent: "#7fbbb3",
        accent_secondary: "#d699b6",
        muted_border: "#475258",
        red: "#e67e80",
        green: "#a7c080",
        yellow: "#dbbc7f",
        cyan: "#83c092",
    },
    OmarchySubThemeSpec {
        subtheme: OmarchySubTheme::FlexokiLight,
        id: "flexoki-light",
        display_name: "Flexoki Light",
        is_dark: false,
        seed_argb: 0xFF20_5EA6,
        background: "#FFFCF0",
        lighter_background: "#E6E4D9",
        selection_background: "#CECDC3",
        foreground: "#100F0F",
        bright_foreground: "#100F0F",
        accent: "#205EA6",
        accent_secondary: "#CE5D97",
        muted_border: "#B7B5AC",
        red: "#D14D41",
        green: "#879A39",
        yellow: "#D0A215",
        cyan: "#3AA99F",
    },
    OmarchySubThemeSpec {
        subtheme: OmarchySubTheme::Gruvbox,
        id: "gruvbox",
        display_name: "Gruvbox",
        is_dark: true,
        seed_argb: 0xFF7D_AEA3,
        background: "#282828",
        lighter_background: "#3c3836",
        selection_background: "#504945",
        foreground: "#d4be98",
        bright_foreground: "#d4be98",
        accent: "#7daea3",
        accent_secondary: "#d3869b",
        muted_border: "#665c54",
        red: "#ea6962",
        green: "#a9b665",
        yellow: "#d8a657",
        cyan: "#89b482",
    },
    OmarchySubThemeSpec {
        subtheme: OmarchySubTheme::Hackerman,
        id: "hackerman",
        display_name: "Hackerman",
        is_dark: true,
        seed_argb: 0xFF82_FB9C,
        background: "#0B0C16",
        lighter_background: "#151828",
        selection_background: "#1f253a",
        foreground: "#ddf7ff",
        bright_foreground: "#ddf7ff",
        accent: "#82FB9C",
        accent_secondary: "#7cf8f7",
        muted_border: "#2d3450",
        red: "#50f872",
        green: "#4fe88f",
        yellow: "#50f7d4",
        cyan: "#7cf8f7",
    },
    OmarchySubThemeSpec {
        subtheme: OmarchySubTheme::OsakaJade,
        id: "osaka-jade",
        display_name: "Osaka Jade",
        is_dark: true,
        seed_argb: 0xFF8B_CBB1,
        background: "#142620",
        lighter_background: "#203b31",
        selection_background: "#2b5147",
        foreground: "#dfede5",
        bright_foreground: "#f0faf4",
        accent: "#8bcbb1",
        accent_secondary: "#86c7c5",
        muted_border: "#64887b",
        red: "#d88d91",
        green: "#8bcbb1",
        yellow: "#c5cc91",
        cyan: "#86c7c5",
    },
    OmarchySubThemeSpec {
        subtheme: OmarchySubTheme::Kanagawa,
        id: "kanagawa",
        display_name: "Kanagawa",
        is_dark: true,
        seed_argb: 0xFF95_7FB8,
        background: "#1f1f28",
        lighter_background: "#223249",
        selection_background: "#363646",
        foreground: "#dcd7ba",
        bright_foreground: "#dcd7ba",
        accent: "#dcd7ba",
        accent_secondary: "#957fb8",
        muted_border: "#54546D",
        red: "#c34043",
        green: "#76946a",
        yellow: "#c0a36e",
        cyan: "#6a9589",
    },
    OmarchySubThemeSpec {
        subtheme: OmarchySubTheme::LastHorizon,
        id: "last-horizon",
        display_name: "Last Horizon",
        is_dark: true,
        seed_argb: 0xFFB5_9790,
        background: "#0c0b0c",
        lighter_background: "#181517",
        selection_background: "#584e51",
        foreground: "#FAFCFB",
        bright_foreground: "#e2dddc",
        accent: "#b59790",
        accent_secondary: "#c4d8e2",
        muted_border: "#584e51",
        red: "#c38b7b",
        green: "#87a9b0",
        yellow: "#6B5E73",
        cyan: "#a5a0b6",
    },
    OmarchySubThemeSpec {
        subtheme: OmarchySubTheme::Lumon,
        id: "lumon",
        display_name: "Lumon",
        is_dark: true,
        seed_argb: 0xFF8B_C9EB,
        background: "#16242d",
        lighter_background: "#1b2d40",
        selection_background: "#243d56",
        foreground: "#d6e2ee",
        bright_foreground: "#f2fcff",
        accent: "#8bc9eb",
        accent_secondary: "#b4e4f6",
        muted_border: "#304860",
        red: "#4d86b0",
        green: "#5e95bc",
        yellow: "#6fa4c9",
        cyan: "#b4e4f6",
    },
    OmarchySubThemeSpec {
        subtheme: OmarchySubTheme::Lupine,
        id: "lupine",
        display_name: "Lupine",
        is_dark: false,
        seed_argb: 0xFF32_64EB,
        background: "#fafafa",
        lighter_background: "#f5f5f5",
        selection_background: "#d0d0d0",
        foreground: "#212121",
        bright_foreground: "#000000",
        accent: "#3264eb",
        accent_secondary: "#8a4ad7",
        muted_border: "#9e9e9e",
        red: "#c900c4",
        green: "#4a2fd0",
        yellow: "#026fde",
        cyan: "#0c67de",
    },
    OmarchySubThemeSpec {
        subtheme: OmarchySubTheme::MatteBlack,
        id: "matte-black",
        display_name: "Matte Black",
        is_dark: true,
        seed_argb: 0xFFE6_8E0D,
        background: "#121212",
        lighter_background: "#1e1e1e",
        selection_background: "#2a2a2a",
        foreground: "#bebebe",
        bright_foreground: "#bebebe",
        accent: "#e68e0d",
        accent_secondary: "#D35F5F",
        muted_border: "#333333",
        red: "#D35F5F",
        green: "#FFC107",
        yellow: "#b91c1c",
        cyan: "#bebebe",
    },
    OmarchySubThemeSpec {
        subtheme: OmarchySubTheme::Miasma,
        id: "miasma",
        display_name: "Miasma",
        is_dark: true,
        seed_argb: 0xFF78_824B,
        background: "#222222",
        lighter_background: "#2c2c2c",
        selection_background: "#383838",
        foreground: "#c2c2b0",
        bright_foreground: "#c2c2b0",
        accent: "#78824b",
        accent_secondary: "#c9a554",
        muted_border: "#666666",
        red: "#685742",
        green: "#5f875f",
        yellow: "#b36d43",
        cyan: "#c9a554",
    },
    OmarchySubThemeSpec {
        subtheme: OmarchySubTheme::Nord,
        id: "nord",
        display_name: "Nord",
        is_dark: true,
        seed_argb: 0xFF81_A1C1,
        background: "#2e3440",
        lighter_background: "#3b4252",
        selection_background: "#434c5e",
        foreground: "#d8dee9",
        bright_foreground: "#d8dee9",
        accent: "#81a1c1",
        accent_secondary: "#88c0d0",
        muted_border: "#4c566a",
        red: "#bf616a",
        green: "#a3be8c",
        yellow: "#ebcb8b",
        cyan: "#88c0d0",
    },
    OmarchySubThemeSpec {
        subtheme: OmarchySubTheme::Retro82,
        id: "retro-82",
        display_name: "Retro 82",
        is_dark: true,
        seed_argb: 0xFFFA_A968,
        background: "#05182e",
        lighter_background: "#0a2540",
        selection_background: "#134e5a",
        foreground: "#f6dcac",
        bright_foreground: "#f6dcac",
        accent: "#faa968",
        accent_secondary: "#8cbfb8",
        muted_border: "#2a6b78",
        red: "#f85525",
        green: "#028391",
        yellow: "#e97b3c",
        cyan: "#8cbfb8",
    },
    OmarchySubThemeSpec {
        subtheme: OmarchySubTheme::Ristretto,
        id: "ristretto",
        display_name: "Ristretto",
        is_dark: true,
        seed_argb: 0xFFF3_8D70,
        background: "#2c2525",
        lighter_background: "#3d2f2a",
        selection_background: "#403e41",
        foreground: "#e6d9db",
        bright_foreground: "#e6d9db",
        accent: "#f38d70",
        accent_secondary: "#85dacc",
        muted_border: "#72696a",
        red: "#fd6883",
        green: "#adda78",
        yellow: "#f9cc6c",
        cyan: "#85dacc",
    },
    OmarchySubThemeSpec {
        subtheme: OmarchySubTheme::RosePine,
        id: "rose-pine",
        display_name: "Rosé Pine",
        is_dark: false,
        seed_argb: 0xFF56_949F,
        background: "#faf4ed",
        lighter_background: "#f2e9e1",
        selection_background: "#dfdad9",
        foreground: "#575279",
        bright_foreground: "#575279",
        accent: "#56949f",
        accent_secondary: "#907aa9",
        muted_border: "#cecacd",
        red: "#b4637a",
        green: "#286983",
        yellow: "#ea9d34",
        cyan: "#d7827e",
    },
    OmarchySubThemeSpec {
        subtheme: OmarchySubTheme::Solitude,
        id: "solitude",
        display_name: "Solitude",
        is_dark: true,
        seed_argb: 0xFF79_8186,
        background: "#101315",
        lighter_background: "#181c1f",
        selection_background: "#343d41",
        foreground: "#cacccc",
        bright_foreground: "#a5aeb4",
        accent: "#798186",
        accent_secondary: "#9fa5a9",
        muted_border: "#4b4e55",
        red: "#565d60",
        green: "#9fa5a9",
        yellow: "#d9dbdc",
        cyan: "#707070",
    },
    OmarchySubThemeSpec {
        subtheme: OmarchySubTheme::Starship,
        id: "starship",
        display_name: "Starship",
        is_dark: true,
        seed_argb: 0xFFFF_9F0A,
        background: "#1c1c1e",
        lighter_background: "#2c2c2e",
        selection_background: "#3a3a3c",
        foreground: "#f5f5f7",
        bright_foreground: "#ffffff",
        accent: "#ff9f0a",
        accent_secondary: "#a6dcef",
        muted_border: "#8e8e93",
        red: "#ff5f4a",
        green: "#ff9f0a",
        yellow: "#ffc27a",
        cyan: "#a6dcef",
    },
    OmarchySubThemeSpec {
        subtheme: OmarchySubTheme::Vantablack,
        id: "vantablack",
        display_name: "Vantablack",
        is_dark: true,
        seed_argb: 0xFF8D_8D8D,
        background: "#000000",
        lighter_background: "#1a1a1a",
        selection_background: "#262626",
        foreground: "#ffffff",
        bright_foreground: "#ffffff",
        accent: "#8d8d8d",
        accent_secondary: "#b0b0b0",
        muted_border: "#7a7a7a",
        red: "#a4a4a4",
        green: "#b6b6b6",
        yellow: "#cecece",
        cyan: "#b0b0b0",
    },
    OmarchySubThemeSpec {
        subtheme: OmarchySubTheme::White,
        id: "white",
        display_name: "White",
        is_dark: false,
        seed_argb: 0xFF6E_6E6E,
        background: "#ffffff",
        lighter_background: "#f0f0f0",
        selection_background: "#c0c0c0",
        foreground: "#000000",
        bright_foreground: "#000000",
        accent: "#6e6e6e",
        accent_secondary: "#3e3e3e",
        muted_border: "#808080",
        red: "#2a2a2a",
        green: "#3a3a3a",
        yellow: "#4a4a4a",
        cyan: "#3e3e3e",
    },
];

impl OmarchySubTheme {
    /// All 23 official Omarchy subthemes in canonical order.
    pub const ALL: [Self; 23] = [
        Self::TokyoNight,
        Self::Catppuccin,
        Self::CatppuccinLatte,
        Self::Ethereal,
        Self::Everforest,
        Self::FlexokiLight,
        Self::Gruvbox,
        Self::Hackerman,
        Self::OsakaJade,
        Self::Kanagawa,
        Self::LastHorizon,
        Self::Lumon,
        Self::Lupine,
        Self::MatteBlack,
        Self::Miasma,
        Self::Nord,
        Self::Retro82,
        Self::Ristretto,
        Self::RosePine,
        Self::Solitude,
        Self::Starship,
        Self::Vantablack,
        Self::White,
    ];

    /// Returns the full [`OmarchySubThemeSpec`] for this subtheme.
    pub const fn spec(self) -> OmarchySubThemeSpec {
        match self {
            Self::TokyoNight => OMARCHY_THEMES[0],
            Self::Catppuccin => OMARCHY_THEMES[1],
            Self::CatppuccinLatte => OMARCHY_THEMES[2],
            Self::Ethereal => OMARCHY_THEMES[3],
            Self::Everforest => OMARCHY_THEMES[4],
            Self::FlexokiLight => OMARCHY_THEMES[5],
            Self::Gruvbox => OMARCHY_THEMES[6],
            Self::Hackerman => OMARCHY_THEMES[7],
            Self::OsakaJade => OMARCHY_THEMES[8],
            Self::Kanagawa => OMARCHY_THEMES[9],
            Self::LastHorizon => OMARCHY_THEMES[10],
            Self::Lumon => OMARCHY_THEMES[11],
            Self::Lupine => OMARCHY_THEMES[12],
            Self::MatteBlack => OMARCHY_THEMES[13],
            Self::Miasma => OMARCHY_THEMES[14],
            Self::Nord => OMARCHY_THEMES[15],
            Self::Retro82 => OMARCHY_THEMES[16],
            Self::Ristretto => OMARCHY_THEMES[17],
            Self::RosePine => OMARCHY_THEMES[18],
            Self::Solitude => OMARCHY_THEMES[19],
            Self::Starship => OMARCHY_THEMES[20],
            Self::Vantablack => OMARCHY_THEMES[21],
            Self::White => OMARCHY_THEMES[22],
        }
    }

    /// Alias for [`Self::spec`].
    pub const fn palette(self) -> OmarchySubThemeSpec {
        self.spec()
    }

    /// Converts this Omarchy subtheme into a [`YoloThemeTokens`] table.
    pub const fn tokens(self) -> YoloThemeTokens {
        let s = self.spec();
        YoloThemeTokens {
            name: s.id,
            bg_primary: s.background,
            bg_surface: s.lighter_background,
            bg_elevated: s.selection_background,
            text_primary: s.bright_foreground,
            text_secondary: s.foreground,
            accent_primary: s.accent,
            accent_secondary: s.accent_secondary,
            border_subtle: s.muted_border,
        }
    }

    /// Parses an [`OmarchySubTheme`] from its kebab-case id (e.g. `"tokyo-night"`, `"osaka-jade"`, `"jade-horse"`, `"lumon"`).
    pub fn from_id(id: &str) -> Option<Self> {
        match id.trim().to_ascii_lowercase().as_str() {
            "tokyo-night" | "tokyonight" => Some(Self::TokyoNight),
            "catppuccin" | "catppuccin-mocha" => Some(Self::Catppuccin),
            "catppuccin-latte" => Some(Self::CatppuccinLatte),
            "ethereal" => Some(Self::Ethereal),
            "everforest" => Some(Self::Everforest),
            "flexoki-light" | "flexoki" => Some(Self::FlexokiLight),
            "gruvbox" => Some(Self::Gruvbox),
            "hackerman" => Some(Self::Hackerman),
            "osaka-jade" | "jade-horse" => Some(Self::OsakaJade),
            "kanagawa" => Some(Self::Kanagawa),
            "last-horizon" => Some(Self::LastHorizon),
            "lumon" => Some(Self::Lumon),
            "lupine" => Some(Self::Lupine),
            "matte-black" => Some(Self::MatteBlack),
            "miasma" => Some(Self::Miasma),
            "nord" => Some(Self::Nord),
            "retro-82" => Some(Self::Retro82),
            "ristretto" => Some(Self::Ristretto),
            "rose-pine" => Some(Self::RosePine),
            "solitude" => Some(Self::Solitude),
            "starship" => Some(Self::Starship),
            "vantablack" => Some(Self::Vantablack),
            "white" => Some(Self::White),
            _ => None,
        }
    }
}

// ============================================================================
// Hyprland (`hyprwm/Hyprland`, `hyprtoolkit`, `hyprqt6engine`) Decoration Spec
// ============================================================================

/// Canonical Hyprland Wayland compositor decoration, border gradient, kawase blur, and animation specification.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct HyprlandDecorationSpec {
    pub active_border_start_hex: &'static str,
    pub active_border_start_argb: u32,
    pub active_border_end_hex: &'static str,
    pub active_border_end_argb: u32,
    pub active_border_angle_deg: u16,
    pub inactive_border_rgba: &'static str,
    pub rounding_px: u8,
    pub border_size_px: u8,
    pub gaps_in_px: u8,
    pub gaps_out_px: u8,
    pub kawase_blur_size: u8,
    pub kawase_blur_passes: u8,
    pub kawase_blur_vibrancy: f32,
    pub bezier_name: &'static str,
    pub bezier_points: [f32; 4],
}

/// Canonical upstream Hyprland decoration specification (`col.active_border = rgba(33ccffee) rgba(00ff99ee) 45deg`).
pub const HYPRLAND_DECORATION_SPEC: HyprlandDecorationSpec = HyprlandDecorationSpec {
    active_border_start_hex: "#33ccff",
    active_border_start_argb: 0xFF33_CCFF,
    active_border_end_hex: "#00ff99",
    active_border_end_argb: 0xFF00_FF99,
    active_border_angle_deg: 45,
    inactive_border_rgba: "rgba(89, 89, 89, 0.67)",
    rounding_px: 10,
    border_size_px: 2,
    gaps_in_px: 5,
    gaps_out_px: 20,
    kawase_blur_size: 8,
    kawase_blur_passes: 3,
    kawase_blur_vibrancy: 0.1696,
    bezier_name: "myBezier",
    bezier_points: [0.05, 0.9, 0.1, 1.05],
};

impl Default for HyprlandDecorationSpec {
    fn default() -> Self {
        HYPRLAND_DECORATION_SPEC
    }
}

// ============================================================================
// Byron Love (Afuro Terumi / Aphrody) 10-Teinte Sprite Atlas & Pet Spec
// ============================================================================

/// A single measured color teinte from the Byron Love (Afuro Terumi / Aphrody) 8×11 sprite atlas.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ByronLoveColorTeinte {
    pub index: u8,
    pub id: &'static str,
    pub hex: &'static str,
    pub argb: u32,
    pub oklch_l: f32,
    pub oklch_c: f32,
    pub oklch_h: f32,
    pub role: &'static str,
}

/// The 10 canonical Oklch/sRGB colors measured from the Byron Love 8×11 sprite atlas (`192×208` cells).
pub const BYRON_LOVE_PALETTE: [ByronLoveColorTeinte; 10] = [
    ByronLoveColorTeinte {
        index: 1,
        id: "creme",
        hex: "#F7F3F0",
        argb: 0xFFF7_F3F0,
        oklch_l: 0.965,
        oklch_c: 0.008,
        oklch_h: 55.0,
        role: "Highlight / Wing Feather Light / On-Dark Primary Text",
    },
    ByronLoveColorTeinte {
        index: 2,
        id: "blond",
        hex: "#EADBA6",
        argb: 0xFFEA_DBA6,
        oklch_l: 0.892,
        oklch_c: 0.078,
        oklch_h: 88.5,
        role: "Byron Love Signature Golden-Blond Hair & Halo Glow",
    },
    ByronLoveColorTeinte {
        index: 3,
        id: "sable",
        hex: "#C2B19E",
        argb: 0xFFC2_B19E,
        oklch_l: 0.768,
        oklch_c: 0.032,
        oklch_h: 68.0,
        role: "Warm Sand Shading / Secondary Hair & Tunic Fold",
    },
    ByronLoveColorTeinte {
        index: 4,
        id: "taupe",
        hex: "#AF9183",
        argb: 0xFFAF_9183,
        oklch_l: 0.678,
        oklch_c: 0.042,
        oklch_h: 42.0,
        role: "Skin Midtone / Warm Neutral Surface Variant",
    },
    ByronLoveColorTeinte {
        index: 5,
        id: "mauve",
        hex: "#8B7083",
        argb: 0xFF8B_7083,
        oklch_l: 0.568,
        oklch_c: 0.048,
        oklch_h: 342.0,
        role: "Divine Wing Shadow / Tertiary Mauve Accent",
    },
    ByronLoveColorTeinte {
        index: 6,
        id: "brun",
        hex: "#6C5454",
        argb: 0xFF6C_5454,
        oklch_l: 0.462,
        oklch_c: 0.034,
        oklch_h: 22.0,
        role: "Warm Umber Contour / Muted Outline",
    },
    ByronLoveColorTeinte {
        index: 7,
        id: "ocre",
        hex: "#91704C",
        argb: 0xFF91_704C,
        oklch_l: 0.562,
        oklch_c: 0.076,
        oklch_h: 66.0,
        role: "Golden Sash Trim / Warm Gold Container",
    },
    ByronLoveColorTeinte {
        index: 8,
        id: "cacao",
        hex: "#4E3738",
        argb: 0xFF4E_3738,
        oklch_l: 0.354,
        oklch_c: 0.031,
        oklch_h: 18.0,
        role: "Deep Sprite Lineart / Elevated Warm Shadow",
    },
    ByronLoveColorTeinte {
        index: 9,
        id: "azur",
        hex: "#17335C",
        argb: 0xFF17_335C,
        oklch_l: 0.318,
        oklch_c: 0.088,
        oklch_h: 256.0,
        role: "Canonical Byron Love Royal Azur Tunic & HCT Theme Seed",
    },
    ByronLoveColorTeinte {
        index: 10,
        id: "nuit",
        hex: "#131420",
        argb: 0xFF13_1420,
        oklch_l: 0.192,
        oklch_c: 0.028,
        oklch_h: 274.0,
        role: "Nuit Abyss Background Surface",
    },
];

/// Metadata specification for the Byron Love (Afuro Terumi / Aphrody) 8×11 interactive pet companion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ByronLovePetSpec {
    pub pet_id: &'static str,
    pub character_name: &'static str,
    pub grid_cols: u8,
    pub grid_rows: u8,
    pub cell_width_px: u16,
    pub cell_height_px: u16,
    pub seed_argb: u32,
    pub seed_hex: &'static str,
    pub celestial_gold_hex: &'static str,
    pub divine_violet_hex: &'static str,
    pub divine_cyan_hex: &'static str,
}

/// Canonical Byron Love pet companion specification (`8×11` grid, `192×208` cells, id `"aphrody-byron-love"`).
pub const BYRON_LOVE_PET_SPEC: ByronLovePetSpec = ByronLovePetSpec {
    pet_id: "aphrody-byron-love",
    character_name: "Byron Love (Afuro Terumi / Aphrody)",
    grid_cols: 8,
    grid_rows: 11,
    cell_width_px: 192,
    cell_height_px: 208,
    seed_argb: 0xFF17_335C,
    seed_hex: "#17335C",
    celestial_gold_hex: "#FFD700",
    divine_violet_hex: "#7C5CFF",
    divine_cyan_hex: "#00D4FF",
};

impl Default for ByronLovePetSpec {
    fn default() -> Self {
        BYRON_LOVE_PET_SPEC
    }
}

// ============================================================================
// Design Tokens (`YoloThemeTokens` & `YoloStyle`)
// ============================================================================

/// Universal surface, text, accent, and border design tokens shared across all UI engines.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct YoloThemeTokens {
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

/// Backwards-compatible alias for [`YoloThemeTokens`] so existing `yolo-ui` / `yolo-gpui` / `yolo-iced` / `yolo-tauri` callers compile unchanged.
pub type AphrodyThemeTokens = YoloThemeTokens;

impl YoloThemeTokens {
    /// Parses `accent_primary` as an [`RgbaColor`].
    #[must_use]
    pub fn primary_rgba(&self) -> RgbaColor {
        RgbaColor::parse_css(self.accent_primary).unwrap_or(RgbaColor::WHITE)
    }

    /// Parses `accent_secondary` as an [`RgbaColor`].
    #[must_use]
    pub fn secondary_rgba(&self) -> RgbaColor {
        RgbaColor::parse_css(self.accent_secondary).unwrap_or(RgbaColor::WHITE)
    }

    /// Parses `bg_primary` as an [`RgbaColor`].
    #[must_use]
    pub fn surface_rgba(&self) -> RgbaColor {
        RgbaColor::parse_css(self.bg_primary).unwrap_or(RgbaColor::BLACK)
    }

    /// Parses `bg_surface` as an [`RgbaColor`].
    #[must_use]
    pub fn surface_container_rgba(&self) -> RgbaColor {
        RgbaColor::parse_css(self.bg_surface).unwrap_or(RgbaColor::BLACK)
    }

    /// Parses `bg_elevated` as an [`RgbaColor`].
    #[must_use]
    pub fn surface_elevated_rgba(&self) -> RgbaColor {
        RgbaColor::parse_css(self.bg_elevated).unwrap_or(RgbaColor::BLACK)
    }

    /// Parses `text_primary` as an [`RgbaColor`].
    #[must_use]
    pub fn on_surface_rgba(&self) -> RgbaColor {
        RgbaColor::parse_css(self.text_primary).unwrap_or(RgbaColor::WHITE)
    }

    /// Parses `text_secondary` as an [`RgbaColor`].
    #[must_use]
    pub fn on_surface_variant_rgba(&self) -> RgbaColor {
        RgbaColor::parse_css(self.text_secondary).unwrap_or(RgbaColor::WHITE)
    }

    /// Parses `border_subtle` as an [`RgbaColor`].
    #[must_use]
    pub fn outline_rgba(&self) -> RgbaColor {
        RgbaColor::parse_css(self.border_subtle).unwrap_or(RgbaColor::WHITE)
    }
}

/// 1. `YoloThemePreset::Neutral` Dark Tokens (Default neutral zinc/slate dark surface, `#6366f1` indigo accent).
pub const YOLO_NEUTRAL_DARK_THEME: YoloThemeTokens = YoloThemeTokens {
    name: "yolo-neutral-dark",
    bg_primary: "#09090b",
    bg_surface: "#18181b",
    bg_elevated: "#27272a",
    text_primary: "#fafafa",
    text_secondary: "#a1a1aa",
    accent_primary: "#6366f1",
    accent_secondary: "#818cf8",
    border_subtle: "rgba(255, 255, 255, 0.08)",
};

/// 1b. `YoloThemePreset::Neutral` Light Tokens.
pub const YOLO_NEUTRAL_LIGHT_THEME: YoloThemeTokens = YoloThemeTokens {
    name: "yolo-neutral-light",
    bg_primary: "#ffffff",
    bg_surface: "#f4f4f5",
    bg_elevated: "#e4e4e7",
    text_primary: "#09090b",
    text_secondary: "#52525b",
    accent_primary: "#4f46e5",
    accent_secondary: "#6366f1",
    border_subtle: "rgba(9, 9, 11, 0.10)",
};

/// Default neutral theme constant for `yolo-ui`.
pub const YOLO_DEFAULT_THEME: YoloThemeTokens = YOLO_NEUTRAL_DARK_THEME;

/// 2. `YoloThemePreset::M3` Dark Tokens (Google Material Design 3 seed `#6750A4`).
pub const YOLO_M3_DARK_THEME: YoloThemeTokens = YoloThemeTokens {
    name: "yolo-m3-dark",
    bg_primary: "#141218",
    bg_surface: "#1d1b20",
    bg_elevated: "#2b2930",
    text_primary: "#e6e0e9",
    text_secondary: "#cac4d0",
    accent_primary: "#d0bcff",
    accent_secondary: "#ccc2dc",
    border_subtle: "#49454f",
};

/// 2b. `YoloThemePreset::M3` Light Tokens (Google Material Design 3 seed `#6750A4`).
pub const YOLO_M3_LIGHT_THEME: YoloThemeTokens = YoloThemeTokens {
    name: "yolo-m3-light",
    bg_primary: "#fef7ff",
    bg_surface: "#f3edf7",
    bg_elevated: "#ece6f0",
    text_primary: "#1d1b20",
    text_secondary: "#49454f",
    accent_primary: "#6750a4",
    accent_secondary: "#625b71",
    border_subtle: "#cac4d0",
};

/// 3. `YoloThemePreset::Fluent` Dark Tokens (Microsoft Fluent UI 2 / WinUI 3 `#0078D4`, Mica dark surfaces).
pub const YOLO_FLUENT_DARK_THEME: YoloThemeTokens = YoloThemeTokens {
    name: "yolo-fluent-dark",
    bg_primary: "#202020",
    bg_surface: "#2c2c2c",
    bg_elevated: "#383838",
    text_primary: "#ffffff",
    text_secondary: "#c5c5c5",
    accent_primary: "#60cdff",
    accent_secondary: "#0078d4",
    border_subtle: "rgba(255, 255, 255, 0.08)",
};

/// 3b. `YoloThemePreset::Fluent` Light Tokens (Microsoft Fluent UI 2 / WinUI 3 `#0078D4`, Mica light surfaces).
pub const YOLO_FLUENT_LIGHT_THEME: YoloThemeTokens = YoloThemeTokens {
    name: "yolo-fluent-light",
    bg_primary: "#f3f3f3",
    bg_surface: "#fbfbfb",
    bg_elevated: "#ffffff",
    text_primary: "#1b1b1b",
    text_secondary: "#5d5d5d",
    accent_primary: "#0078d4",
    accent_secondary: "#005fb8",
    border_subtle: "rgba(0, 0, 0, 0.06)",
};

/// 4. `YoloThemePreset::Apple` Dark Tokens (Apple macOS Tahoe Liquid Glass & iOS Cupertino HIG `#007AFF`).
pub const YOLO_APPLE_DARK_THEME: YoloThemeTokens = YoloThemeTokens {
    name: "yolo-apple-dark",
    bg_primary: "#1e1e1e",
    bg_surface: "#28282a",
    bg_elevated: "#323236",
    text_primary: "#ffffff",
    text_secondary: "#98989d",
    accent_primary: "#0a84ff",
    accent_secondary: "#30d158",
    border_subtle: "rgba(255, 255, 255, 0.14)",
};

/// 4b. `YoloThemePreset::Apple` Light Tokens (Apple macOS Tahoe Liquid Glass & iOS Cupertino HIG `#007AFF`).
pub const YOLO_APPLE_LIGHT_THEME: YoloThemeTokens = YoloThemeTokens {
    name: "yolo-apple-light",
    bg_primary: "#f5f5f7",
    bg_surface: "#ffffff",
    bg_elevated: "#ebebf0",
    text_primary: "#1d1d1f",
    text_secondary: "#86868b",
    accent_primary: "#007aff",
    accent_secondary: "#34c759",
    border_subtle: "rgba(0, 0, 0, 0.10)",
};

/// Apple HIG semantic status & traffic-light window control colors.
pub mod apple_hig {
    pub const SYSTEM_BLUE: &str = "#007AFF";
    pub const SYSTEM_GREEN: &str = "#34C759";
    pub const SYSTEM_ORANGE: &str = "#FF9500";
    pub const SYSTEM_RED: &str = "#FF3B30";
    pub const TRAFFIC_LIGHT_CLOSE: &str = "#FF5F56";
    pub const TRAFFIC_LIGHT_MINIMIZE: &str = "#FFBD2E";
    pub const TRAFFIC_LIGHT_ZOOM: &str = "#27C93F";
    pub const LIQUID_GLASS_BLUR_PX: f32 = 24.0;
}

/// 5. `YoloThemePreset::Omarchy` Dark Tokens (Omacom Omarchy `Tokyo Night` default from `submodules/omarchy/themes/tokyo-night/colors.toml`).
pub const YOLO_OMARCHY_DARK_THEME: YoloThemeTokens = YoloThemeTokens {
    name: "yolo-omarchy-tokyo-night",
    bg_primary: "#1a1b26",
    bg_surface: "#24283b",
    bg_elevated: "#292e42",
    text_primary: "#c0caf5",
    text_secondary: "#a9b1d6",
    accent_primary: "#7aa2f7",
    accent_secondary: "#bb9af7",
    border_subtle: "#414868",
};

/// 5b. `YoloThemePreset::Omarchy` Light Tokens (Omacom Omarchy `Catppuccin Latte` from `submodules/omarchy/themes/catppuccin-latte/colors.toml`).
pub const YOLO_OMARCHY_LIGHT_THEME: YoloThemeTokens = YoloThemeTokens {
    name: "yolo-omarchy-catppuccin-latte",
    bg_primary: "#eff1f5",
    bg_surface: "#dce0e8",
    bg_elevated: "#ccd0da",
    text_primary: "#4c4f69",
    text_secondary: "#5c5f77",
    accent_primary: "#1e66f5",
    accent_secondary: "#ea76cb",
    border_subtle: "#acb0be",
};

/// 6. `YoloThemePreset::Hyprland` Dark Tokens (Hyprwm / Hyprland Wayland Compositor `#33ccff` cyan -> `#00ff99` mint emerald).
pub const YOLO_HYPRLAND_DARK_THEME: YoloThemeTokens = YoloThemeTokens {
    name: "yolo-hyprland-dark",
    bg_primary: "#0b0e14",
    bg_surface: "#111622",
    bg_elevated: "#192030",
    text_primary: "#eef4fc",
    text_secondary: "#94a3b8",
    accent_primary: "#33ccff",
    accent_secondary: "#00ff99",
    border_subtle: "rgba(89, 89, 89, 0.67)",
};

/// 6b. `YoloThemePreset::Hyprland` Light Tokens.
pub const YOLO_HYPRLAND_LIGHT_THEME: YoloThemeTokens = YoloThemeTokens {
    name: "yolo-hyprland-light",
    bg_primary: "#f0f6fc",
    bg_surface: "#ffffff",
    bg_elevated: "#e2ecf7",
    text_primary: "#0b0e14",
    text_secondary: "#475569",
    accent_primary: "#0099cc",
    accent_secondary: "#00b36b",
    border_subtle: "rgba(15, 23, 42, 0.14)",
};

/// 7. `YoloThemePreset::Aphrody` Dark Tokens (Byron Love / Aphrody Dark-Glass with `#7c5cff` divine violet, `#00d4ff` cyan, `#FFD700` gold).
pub const YOLO_APHRODY_DARK_THEME: YoloThemeTokens = YoloThemeTokens {
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

/// 7b. `YoloThemePreset::Aphrody` Light Tokens (Byron Love Creme `#F7F3F0`, Sable `#C2B19E`, Royal Azur `#17335C`, Ocre `#91704C`).
pub const YOLO_APHRODY_LIGHT_THEME: YoloThemeTokens = YoloThemeTokens {
    name: "yolo-aphrody-light",
    bg_primary: "#F7F3F0",
    bg_surface: "#EADBA6",
    bg_elevated: "#C2B19E",
    text_primary: "#131420",
    text_secondary: "#4E3738",
    accent_primary: "#17335C",
    accent_secondary: "#91704C",
    border_subtle: "rgba(23, 51, 92, 0.14)",
};

/// Backwards-compatible canonical Aphrody Material 3 Dark-Glass token constant.
pub const APHRODY_DARK_THEME: AphrodyThemeTokens = YOLO_APHRODY_DARK_THEME;

/// Structural geometry, spacing grid, typography stack, backdrop blur, and elevation tokens per preset.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct YoloStyle {
    pub preset: YoloThemePreset,
    pub radius_sm_px: f32,
    pub radius_md_px: f32,
    pub radius_lg_px: f32,
    pub radius_pill_px: f32,
    pub spacing_grid_px: f32,
    pub font_family_ui: &'static str,
    pub font_family_mono: &'static str,
    pub backdrop_blur_px: f32,
    pub elevation_level: u8,
    pub border_width_px: f32,
    pub window_vibrancy: &'static str,
}

impl YoloStyle {
    /// Returns the canonical [`YoloStyle`] for any of the 7 [`YoloThemePreset`]s.
    pub const fn for_preset(preset: YoloThemePreset) -> Self {
        match preset {
            YoloThemePreset::Neutral => Self {
                preset,
                radius_sm_px: 6.0,
                radius_md_px: 8.0,
                radius_lg_px: 12.0,
                radius_pill_px: 9999.0,
                spacing_grid_px: 4.0,
                font_family_ui: "Inter, system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif",
                font_family_mono: "'JetBrains Mono', ui-monospace, SFMono-Regular, Consolas, monospace",
                backdrop_blur_px: 12.0,
                elevation_level: 1,
                border_width_px: 1.0,
                window_vibrancy: "none",
            },
            YoloThemePreset::M3 => Self {
                preset,
                radius_sm_px: 8.0,
                radius_md_px: 12.0,
                radius_lg_px: 16.0,
                radius_pill_px: 9999.0,
                spacing_grid_px: 4.0,
                font_family_ui: "'Google Sans Flex', 'Google Sans', Roboto, system-ui, sans-serif",
                font_family_mono: "'Google Sans Code', 'JetBrains Mono', monospace",
                backdrop_blur_px: 0.0,
                elevation_level: 2,
                border_width_px: 1.0,
                window_vibrancy: "surface-tint",
            },
            YoloThemePreset::Fluent => Self {
                preset,
                radius_sm_px: 4.0,
                radius_md_px: 4.0,
                radius_lg_px: 8.0,
                radius_pill_px: 9999.0,
                spacing_grid_px: 4.0,
                font_family_ui: "'Segoe UI Variable', 'Segoe UI', system-ui, sans-serif",
                font_family_mono: "'Cascadia Code', Consolas, monospace",
                backdrop_blur_px: 20.0,
                elevation_level: 1,
                border_width_px: 1.0,
                window_vibrancy: "mica",
            },
            YoloThemePreset::Apple => Self {
                preset,
                radius_sm_px: 6.0,
                radius_md_px: 10.0,
                radius_lg_px: 14.0,
                radius_pill_px: 9999.0,
                spacing_grid_px: 4.0,
                font_family_ui: "'SF Pro', -apple-system, BlinkMacSystemFont, 'Helvetica Neue', sans-serif",
                font_family_mono: "'SF Mono', ui-monospace, Menlo, monospace",
                backdrop_blur_px: 24.0,
                elevation_level: 2,
                border_width_px: 0.5,
                window_vibrancy: "macos-liquid-glass",
            },
            YoloThemePreset::Omarchy => Self {
                preset,
                radius_sm_px: 0.0,
                radius_md_px: 0.0,
                radius_lg_px: 2.0,
                radius_pill_px: 2.0,
                spacing_grid_px: 8.0,
                font_family_ui: "'Geist Variable', 'JetBrains Mono Variable', 'CaskaydiaMono Nerd Font', monospace",
                font_family_mono: "'JetBrains Mono Variable', 'JetBrains Mono', 'CaskaydiaMono Nerd Font', monospace",
                backdrop_blur_px: 16.0,
                elevation_level: 0,
                border_width_px: 2.0,
                window_vibrancy: "hyprland-blur",
            },
            YoloThemePreset::Hyprland => Self {
                preset,
                radius_sm_px: 8.0,
                radius_md_px: 10.0,
                radius_lg_px: 12.0,
                radius_pill_px: 9999.0,
                spacing_grid_px: 5.0,
                font_family_ui: "'Inter', 'Noto Sans', 'JetBrains Mono', system-ui, sans-serif",
                font_family_mono: "'JetBrains Mono', 'CaskaydiaCove Nerd Font', monospace",
                backdrop_blur_px: 24.0,
                elevation_level: 2,
                border_width_px: 2.0,
                window_vibrancy: "kawase-blur",
            },
            YoloThemePreset::Aphrody => Self {
                preset,
                radius_sm_px: 8.0,
                radius_md_px: 12.0,
                radius_lg_px: 16.0,
                radius_pill_px: 9999.0,
                spacing_grid_px: 4.0,
                font_family_ui: "'Google Sans Flex', 'Zed Plex Sans', Inter, system-ui, sans-serif",
                font_family_mono: "'Google Sans Code', 'Zed Plex Mono', 'JetBrains Mono', monospace",
                backdrop_blur_px: 20.0,
                elevation_level: 3,
                border_width_px: 1.0,
                window_vibrancy: "acrylic-dark-glass",
            },
        }
    }
}

impl Default for YoloStyle {
    fn default() -> Self {
        Self::for_preset(YoloThemePreset::Neutral)
    }
}

// ============================================================================
// Neutral & Customizable Application Manifest (`YoloManifest`)
// ============================================================================

/// Vendor-neutral, customizable application manifest for any `yolo-ui` host.
///
/// Defaults to [`YoloThemePreset::Neutral`] with `pet_companion: None` so new projects
/// start 100% clean and unbranded, while allowing one-line opt-in to M3, Fluent 2,
/// Apple Liquid Glass, Omacom Omarchy (`OmarchySubTheme`), Hyprland (`HyprlandDecorationSpec`),
/// or Aphrody (Byron Love).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(bound(deserialize = "'de: 'static"))]
pub struct YoloManifest {
    pub app_id: String,
    pub title: String,
    pub version: String,
    pub organization: String,
    pub theme_preset: YoloThemePreset,
    pub omarchy_subtheme: Option<OmarchySubTheme>,
    pub hyprland_decoration: Option<HyprlandDecorationSpec>,
    pub custom_seed_argb: Option<u32>,
    pub dark_mode: bool,
    pub window_vibrancy: String,
    pub pet_companion: Option<ByronLovePetSpec>,
}

impl Default for YoloManifest {
    fn default() -> Self {
        Self {
            app_id: String::from("org.example.yolo-app"),
            title: String::from("Yolo UI Application"),
            version: String::from("0.8.0"),
            organization: String::from("Neutral Organization"),
            theme_preset: YoloThemePreset::Neutral,
            omarchy_subtheme: None,
            hyprland_decoration: None,
            custom_seed_argb: None,
            dark_mode: true,
            window_vibrancy: String::from("none"),
            pet_companion: None,
        }
    }
}

impl YoloManifest {
    /// Creates a new neutral [`YoloManifest`] with the given `app_id` and `title`.
    pub fn new(app_id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            app_id: app_id.into(),
            title: title.into(),
            ..Self::default()
        }
    }

    /// Switches the manifest to one of the 7 built-in [`YoloThemePreset`]s and updates default vibrancy.
    pub fn with_preset(mut self, preset: YoloThemePreset) -> Self {
        self.theme_preset = preset;
        self.window_vibrancy = preset.style().window_vibrancy.to_string();
        if preset == YoloThemePreset::Omarchy && self.omarchy_subtheme.is_none() {
            self.omarchy_subtheme = Some(OmarchySubTheme::TokyoNight);
        }
        if preset == YoloThemePreset::Hyprland && self.hyprland_decoration.is_none() {
            self.hyprland_decoration = Some(HYPRLAND_DECORATION_SPEC);
        }
        if preset == YoloThemePreset::Aphrody && self.pet_companion.is_none() {
            self.pet_companion = Some(BYRON_LOVE_PET_SPEC);
        }
        self
    }

    /// Switches the manifest to [`YoloThemePreset::Omarchy`] with a specific [`OmarchySubTheme`].
    pub fn with_omarchy_subtheme(mut self, subtheme: OmarchySubTheme) -> Self {
        self.theme_preset = YoloThemePreset::Omarchy;
        self.omarchy_subtheme = Some(subtheme);
        self.dark_mode = subtheme.spec().is_dark;
        self.window_vibrancy = YoloThemePreset::Omarchy.style().window_vibrancy.to_string();
        self
    }

    /// Switches the manifest to [`YoloThemePreset::Hyprland`] with the given [`HyprlandDecorationSpec`].
    pub fn with_hyprland_decoration(mut self, spec: HyprlandDecorationSpec) -> Self {
        self.theme_preset = YoloThemePreset::Hyprland;
        self.hyprland_decoration = Some(spec);
        self.window_vibrancy = YoloThemePreset::Hyprland.style().window_vibrancy.to_string();
        self
    }

    /// Overrides the theme seed with a custom opaque `0xAARRGGBB` color.
    pub fn with_custom_seed(mut self, seed_argb: u32) -> Self {
        self.custom_seed_argb = Some(0xFF00_0000 | seed_argb);
        self
    }

    /// Sets light or dark mode preference.
    pub fn with_dark_mode(mut self, dark_mode: bool) -> Self {
        self.dark_mode = dark_mode;
        self
    }

    /// Enables the Byron Love (`"aphrody-byron-love"`) 8×11 sprite companion on this manifest.
    pub fn with_byron_love_pet(mut self) -> Self {
        self.pet_companion = Some(BYRON_LOVE_PET_SPEC);
        self
    }

    /// Resolves the active `0xAARRGGBB` seed (custom seed if set, Omarchy subtheme seed if set, otherwise preset seed).
    pub const fn resolved_seed_argb(&self) -> u32 {
        if let Some(seed) = self.custom_seed_argb {
            return seed;
        }
        if let Some(sub) = self.omarchy_subtheme {
            return sub.spec().seed_argb;
        }
        self.theme_preset.seed_argb()
    }

    /// Resolves the active [`YoloThemeTokens`] for the manifest's preset (or [`OmarchySubTheme`]) and `dark_mode`.
    pub const fn resolved_tokens(&self) -> YoloThemeTokens {
        if let Some(sub) = self.omarchy_subtheme {
            return sub.tokens();
        }
        self.theme_preset.tokens(self.dark_mode)
    }

    /// Resolves the active [`YoloStyle`] for the manifest's preset.
    pub const fn resolved_style(&self) -> YoloStyle {
        self.theme_preset.style()
    }
}

/// Generates the CSS `:root` custom properties block for any [`YoloThemeTokens`] palette.
pub fn css_variables(tokens: &YoloThemeTokens) -> String {
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

/// Generates a complete preset CSS `:root` block including both `--yolo-*` color variables and structural style variables.
pub fn css_preset_variables(preset: YoloThemePreset, dark: bool) -> String {
    let tokens = preset.tokens(dark);
    let style = preset.style();
    format!(
        ":root {{ \
            --yolo-theme-preset: \"{}\"; \
            --yolo-theme-seed: {}; \
            --yolo-bg-primary: {}; \
            --yolo-bg-surface: {}; \
            --yolo-bg-elevated: {}; \
            --yolo-text-primary: {}; \
            --yolo-text-secondary: {}; \
            --yolo-accent-primary: {}; \
            --yolo-accent-secondary: {}; \
            --yolo-border-subtle: {}; \
            --yolo-radius-sm: {}px; \
            --yolo-radius-md: {}px; \
            --yolo-radius-lg: {}px; \
            --yolo-radius-pill: {}px; \
            --yolo-spacing-grid: {}px; \
            --yolo-font-sans: {}; \
            --yolo-font-mono: {}; \
            --yolo-backdrop-blur: {}px; \
            --yolo-border-width: {}px; \
            --yolo-window-vibrancy: {}; \
        }}",
        preset.id(),
        preset.seed_hex(),
        tokens.bg_primary,
        tokens.bg_surface,
        tokens.bg_elevated,
        tokens.text_primary,
        tokens.text_secondary,
        tokens.accent_primary,
        tokens.accent_secondary,
        tokens.border_subtle,
        style.radius_sm_px,
        style.radius_md_px,
        style.radius_lg_px,
        style.radius_pill_px,
        style.spacing_grid_px,
        style.font_family_ui,
        style.font_family_mono,
        style.backdrop_blur_px,
        style.border_width_px,
        style.window_vibrancy,
    )
}

/// Upstream organization classification in the expanded Yolo UI multi-engine fleet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EcosystemOrg {
    DioxusLabs,
    Longbridge,
    ZedIndustries,
    IcedRs,
    TauriApps,
    Egui,
    Cosmic,
    GtkRs,
    CxxQt,
    FluentUi,
    MaterialDesign,
    AppleUi,
    Omacom,
    Hyprwm,
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

    #[test]
    fn test_neutral_by_default_and_seven_theme_presets() {
        assert_eq!(YoloThemePreset::default(), YoloThemePreset::Neutral);
        let manifest = YoloManifest::default();
        assert_eq!(manifest.theme_preset, YoloThemePreset::Neutral);
        assert!(manifest.pet_companion.is_none());
        assert_eq!(manifest.resolved_seed_argb(), 0xFF63_66F1);

        let all_ids: Vec<&str> = YoloThemePreset::ALL.iter().map(|p| p.id()).collect();
        assert_eq!(
            all_ids,
            vec![
                "yolo-neutral",
                "yolo-m3",
                "yolo-fluent",
                "yolo-apple",
                "yolo-omarchy",
                "yolo-hyprland",
                "yolo-aphrody"
            ]
        );

        for preset in YoloThemePreset::ALL {
            assert_eq!(YoloThemePreset::from_id(preset.id()), Some(preset));
            let css = css_preset_variables(preset, true);
            assert!(css.contains(preset.id()));
        }
    }

    #[test]
    fn test_omarchy_twenty_three_subthemes_and_hyprland_spec() {
        assert_eq!(OMARCHY_THEMES.len(), 23);
        assert_eq!(OmarchySubTheme::ALL.len(), 23);
        assert_eq!(OmarchySubTheme::default(), OmarchySubTheme::TokyoNight);
        let tokyo = OmarchySubTheme::TokyoNight.spec();
        assert_eq!(tokyo.background, "#1a1b26");
        assert_eq!(tokyo.accent, "#7aa2f7");
        assert_eq!(tokyo.accent_secondary, "#bb9af7");

        for sub in OmarchySubTheme::ALL {
            assert_eq!(OmarchySubTheme::from_id(sub.spec().id), Some(sub));
        }

        let omarchy_manifest = YoloManifest::new("org.omarchy.app", "Omarchy Walker")
            .with_omarchy_subtheme(OmarchySubTheme::OsakaJade);
        assert_eq!(omarchy_manifest.theme_preset, YoloThemePreset::Omarchy);
        assert_eq!(omarchy_manifest.resolved_tokens().bg_primary, "#142620");
        assert_eq!(omarchy_manifest.resolved_tokens().accent_primary, "#8bcbb1");

        let hypr_manifest = YoloManifest::new("org.hyprwm.shell", "Hyprland Shell")
            .with_preset(YoloThemePreset::Hyprland);
        let dec = hypr_manifest.hyprland_decoration.expect("Hyprland decoration attached");
        assert_eq!(dec.active_border_start_hex, "#33ccff");
        assert_eq!(dec.active_border_end_hex, "#00ff99");
        assert_eq!(dec.active_border_angle_deg, 45);
        assert_eq!(dec.kawase_blur_size, 8);
        assert_eq!(dec.kawase_blur_passes, 3);
    }

    #[test]
    fn test_byron_love_ten_teinte_palette_and_pet_spec() {
        assert_eq!(BYRON_LOVE_PALETTE.len(), 10);
        assert_eq!(BYRON_LOVE_PALETTE[0].hex, "#F7F3F0");
        assert_eq!(BYRON_LOVE_PALETTE[8].id, "azur");
        assert_eq!(BYRON_LOVE_PALETTE[8].argb, 0xFF17_335C);
        assert_eq!(BYRON_LOVE_PALETTE[9].hex, "#131420");

        let aphrody_manifest = YoloManifest::new("com.aphrody.studio", "Aphrody Studio")
            .with_preset(YoloThemePreset::Aphrody);
        let pet = aphrody_manifest.pet_companion.expect("Byron Love pet attached");
        assert_eq!(pet.pet_id, "aphrody-byron-love");
        assert_eq!(pet.grid_cols, 8);
        assert_eq!(pet.grid_rows, 11);
        assert_eq!(pet.cell_width_px, 192);
        assert_eq!(pet.cell_height_px, 208);
    }
}
