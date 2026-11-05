//! The theme pack format (`theme.toml`).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// A complete theme: identity, assets, colour rules and interface shape.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThemePack {
    /// Stable kebab-case id, also the directory name.
    pub id: String,
    /// Display name in the theme picker.
    pub name: String,
    /// Application title shown in the window and the shell while this pack is active.
    pub title: String,
    /// Logo file inside the pack directory (SVG with `currentColor` so it follows the scheme).
    #[serde(default)]
    pub logo: Option<String>,
    /// How colours are produced.
    pub color: ColorSpec,
    /// Icon set and style.
    pub icons: IconSpec,
    /// Wallpapers, in rotation order. Empty means the generated wallpaper is used.
    #[serde(default)]
    pub wallpapers: Vec<Wallpaper>,
    /// Companion pet shown by the shell.
    #[serde(default)]
    pub pet: Option<PetSpec>,
    /// Typography (open-licensed families only).
    pub typography: Typography,
    /// Interface shape: layout, density, radii, glass and motion.
    pub shell: ShellSpec,
}

/// Colour rules of a pack.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ColorSpec {
    /// Seed colour as `#RRGGBB`, used when the source is [`ColorSource::Seed`].
    pub seed: String,
    /// Where the scheme seed comes from by default.
    #[serde(default)]
    pub source: ColorSource,
    /// Dynamic scheme variant name (`tonal-spot`, `vibrant`, `expressive`, `fidelity`, ...).
    #[serde(default = "default_variant")]
    pub variant: String,
    /// Contrast level from -1.0 (reduced) to 1.0 (high); 0.0 is the M3 standard.
    #[serde(default)]
    pub contrast: f64,
    /// Default mode.
    #[serde(default)]
    pub mode: Mode,
    /// Per-role colour overrides (`role-name` to `#RRGGBB`), applied after the scheme.
    #[serde(default)]
    pub overrides: BTreeMap<String, String>,
}

fn default_variant() -> String {
    "tonal-spot".to_owned()
}

/// Seed source.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ColorSource {
    /// The pack or user seed.
    #[default]
    Seed,
    /// The dominant colour of the active wallpaper (its `seed` field, or extracted from the image).
    Wallpaper,
}

/// Light or dark preference.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Mode {
    /// Follow the operating system.
    #[default]
    System,
    /// Always light.
    Light,
    /// Always dark.
    Dark,
}

/// Icon set selection.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IconSpec {
    /// Icon set id in the yolo-icons catalogue (`material-symbols`, `fluent`, `phosphor`, ...).
    pub set: String,
    /// Glyph style.
    #[serde(default)]
    pub style: IconStyle,
    /// Stroke or variable-font weight.
    #[serde(default = "default_weight")]
    pub weight: u16,
}

fn default_weight() -> u16 {
    400
}

/// Glyph style.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum IconStyle {
    /// Outlined glyphs.
    #[default]
    Outlined,
    /// Filled glyphs.
    Filled,
    /// Rounded terminals.
    Rounded,
    /// Sharp terminals.
    Sharp,
}

impl IconStyle {
    /// Kebab-case name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Outlined => "outlined",
            Self::Filled => "filled",
            Self::Rounded => "rounded",
            Self::Sharp => "sharp",
        }
    }
}

/// One wallpaper and where it comes from.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Wallpaper {
    /// File inside the pack directory.
    pub file: String,
    /// Which mode the image suits; `None` means both.
    #[serde(default)]
    pub mode: Option<Mode>,
    /// Precomputed dominant colour `#RRGGBB`, so the seed needs no image decode at runtime.
    #[serde(default)]
    pub seed: Option<String>,
    /// SPDX licence id or `LicenseRef-*`; `unverified` keeps the file out of distribution.
    pub license: String,
    /// Where the file came from (URL, or `original`).
    pub source: String,
    /// Credit line the licence requires, if any.
    #[serde(default)]
    pub attribution: Option<String>,
}

impl Wallpaper {
    /// Whether this file may ship with the product.
    #[must_use]
    pub fn distributable(&self) -> bool {
        let license = self.license.trim();
        !license.is_empty() && !license.eq_ignore_ascii_case("unverified")
    }
}

/// Companion pet from the OpenPets catalogue or from the pack itself.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PetSpec {
    /// Pet id in its provider.
    pub id: String,
    /// Provider: `openpets` or `pack` (files inside the pack directory).
    pub provider: String,
    /// Where the pet sits by default (`bottom-right`, `bottom-left`, `dock`).
    #[serde(default = "default_anchor")]
    pub anchor: String,
}

fn default_anchor() -> String {
    "bottom-right".to_owned()
}

/// Typography; families must be open-licensed (OFL or similar).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Typography {
    /// Display and title family.
    pub display: String,
    /// Body family.
    pub body: String,
    /// Monospace family for code.
    pub mono: String,
}

/// Interface shape.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShellSpec {
    /// Where navigation lives; changing it rearranges the whole shell.
    pub layout: ShellLayout,
    /// Spacing density from -2 (compact) to 2 (spacious).
    #[serde(default)]
    pub density: i8,
    /// Corner radii in px for small, medium and large surfaces.
    pub radius: [u16; 3],
    /// Backdrop blur in px for glass surfaces; 0 disables glass.
    #[serde(default)]
    pub glass_blur: u16,
    /// Motion character.
    #[serde(default)]
    pub motion: MotionProfile,
}

/// Shell layout.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ShellLayout {
    /// Navigation rail or sidebar on the leading edge.
    Sidebar,
    /// Navigation in a top bar.
    TopBar,
    /// Launcher dock at the bottom.
    Dock,
    /// Floating panels over the wallpaper.
    Floating,
}

impl ShellLayout {
    /// Kebab-case name, used as the `data-shell` attribute.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Sidebar => "sidebar",
            Self::TopBar => "top-bar",
            Self::Dock => "dock",
            Self::Floating => "floating",
        }
    }
}

/// Motion character, mapped to duration and easing tokens.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MotionProfile {
    /// Short, eased transitions.
    #[default]
    Calm,
    /// Spring motion with overshoot.
    Lively,
    /// Instant state changes.
    Still,
}

impl ThemePack {
    /// Parses a `theme.toml`.
    ///
    /// # Errors
    /// Returns the TOML error when the text is not a valid pack.
    pub fn from_toml(text: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(text)
    }

    /// Wallpapers that may ship with the product.
    pub fn distributable_wallpapers(&self) -> impl Iterator<Item = &Wallpaper> {
        self.wallpapers.iter().filter(|w| w.distributable())
    }
}

/// Parses `#RRGGBB` (or `RRGGBB`) into opaque ARGB.
#[must_use]
pub fn parse_hex(hex: &str) -> Option<u32> {
    let digits = hex.trim().trim_start_matches('#');
    if digits.len() != 6 {
        return None;
    }
    u32::from_str_radix(digits, 16).ok().map(|rgb| 0xFF00_0000 | rgb)
}
