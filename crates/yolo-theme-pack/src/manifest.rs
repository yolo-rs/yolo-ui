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
    /// Product and window strings while this pack is active.
    pub title: Titles,
    /// Logo files inside the pack directory.
    #[serde(default)]
    pub logo: Option<LogoSpec>,
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

/// Product and window strings.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Titles {
    /// Product name (about box, version strings, extension action title).
    pub product_name: String,
    /// Window title template; `{page}` is replaced by the current page title.
    pub window_title_template: String,
    /// Heading of the new tab page.
    pub ntp_heading: String,
}

impl Titles {
    /// Window title for `page`; the product name alone when `page` is empty.
    #[must_use]
    pub fn window_title(&self, page: &str) -> String {
        if page.trim().is_empty() {
            self.product_name.clone()
        } else {
            self.window_title_template.replace("{page}", page)
        }
    }
}

/// Logo files, relative to the pack directory. Raster sizes come from yolo-icons' favicon
/// generator so every surface (extension manifest, Windows launcher, browser tab) has one.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LogoSpec {
    /// Coloured mark (SVG).
    pub mark: String,
    /// Single-colour mark using `currentColor` (SVG), for tinting by the scheme.
    pub monochrome: String,
    /// Wordmark (SVG), if the pack has one.
    #[serde(default)]
    pub wordmark: Option<String>,
    /// PNG renders by pixel size (16, 32, 48, 128, ...).
    #[serde(default)]
    pub png: BTreeMap<u16, String>,
    /// Multi-size `.ico` for Windows launchers.
    #[serde(default)]
    pub ico: Option<String>,
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
    /// How the image fills the viewport.
    #[serde(default)]
    pub fit: WallpaperFit,
    /// Focal point in percent of width and height (0..=100), kept visible when cropping.
    #[serde(default = "default_focal")]
    pub focal: [u8; 2],
    /// SHA-256 of the file (lowercase hex), checked before use so offline packs stay intact.
    #[serde(default)]
    pub sha256: Option<String>,
}

fn default_focal() -> [u8; 2] {
    [50, 50]
}

/// How a wallpaper fills the viewport.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WallpaperFit {
    /// Fill and crop around the focal point.
    #[default]
    Cover,
    /// Fit entirely, letterboxed on the surface colour.
    Contain,
    /// Repeat at natural size.
    Tile,
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
    /// Animation states the pet provides; shells map app activity onto them.
    #[serde(default = "default_pet_states")]
    pub states: Vec<String>,
    /// Display size hint in CSS px (the longest side).
    #[serde(default = "default_pet_size")]
    pub size: u16,
}

fn default_pet_states() -> Vec<String> {
    ["idle", "active", "sleep"].map(str::to_owned).to_vec()
}

fn default_pet_size() -> u16 {
    96
}

impl PetSpec {
    /// Asset directory of the pet, relative to the bundle root: `pets/<provider>/<id>/`, holding
    /// one file per state (`<state>.webp` animated, or `<state>.json` for a sprite sheet spec).
    #[must_use]
    pub fn asset_dir(&self) -> String {
        format!("pets/{}/{}/", self.provider, self.id)
    }
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
    let text = hex.trim();
    if let Some(inner) = text
        .strip_prefix("oklch(")
        .and_then(|t| t.strip_suffix(')'))
    {
        return parse_oklch(inner);
    }
    let digits = text.trim_start_matches('#');
    if digits.len() != 6 {
        return None;
    }
    u32::from_str_radix(digits, 16)
        .ok()
        .map(|rgb| 0xFF00_0000 | rgb)
}

/// `L C H` from CSS `oklch()` (L as 0..1 or a percentage, H in degrees) to opaque sRGB ARGB,
/// using the OKLab matrices from Bjorn Ottosson's reference. Out-of-gamut channels are clipped.
fn parse_oklch(inner: &str) -> Option<u32> {
    let mut parts = inner
        .split(|c: char| c.is_whitespace() || c == '/')
        .filter(|p| !p.is_empty());
    let l_text = parts.next()?;
    let l = match l_text.strip_suffix('%') {
        Some(p) => p.parse::<f64>().ok()? / 100.0,
        None => l_text.parse::<f64>().ok()?,
    };
    let c = parts.next()?.parse::<f64>().ok()?;
    let h = parts
        .next()?
        .trim_end_matches("deg")
        .parse::<f64>()
        .ok()?
        .to_radians();
    let (a, b) = (c * h.cos(), c * h.sin());
    let l_ = (l + 0.396_337_777_4 * a + 0.215_803_757_3 * b).powi(3);
    let m_ = (l - 0.105_561_345_8 * a - 0.063_854_172_8 * b).powi(3);
    let s_ = (l - 0.089_484_177_5 * a - 1.291_485_548 * b).powi(3);
    let linear = [
        4.076_741_662_1 * l_ - 3.307_711_591_3 * m_ + 0.230_969_929_2 * s_,
        -1.268_438_004_6 * l_ + 2.609_757_401_1 * m_ - 0.341_319_396_5 * s_,
        -0.004_196_086_3 * l_ - 0.703_418_614_7 * m_ + 1.707_614_701 * s_,
    ];
    let encode = |x: f64| {
        let x = x.clamp(0.0, 1.0);
        let v = if x <= 0.003_130_8 {
            12.92 * x
        } else {
            1.055 * x.powf(1.0 / 2.4) - 0.055
        };
        // Clamped to 0..=1 above, so the rounded value fits in 0..=255.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let v = (v * 255.0).round() as u32;
        v
    };
    Some(0xFF00_0000 | encode(linear[0]) << 16 | encode(linear[1]) << 8 | encode(linear[2]))
}

#[cfg(test)]
mod tests {
    use super::parse_hex;

    #[test]
    fn parses_hex_and_oklch() {
        assert_eq!(parse_hex("#e78220"), Some(0xFFE7_8220));
        assert_eq!(parse_hex("oklch(1 0 0)"), Some(0xFFFF_FFFF));
        assert_eq!(parse_hex("oklch(0% 0 0)"), Some(0xFF00_0000));
        // CSS Color 4 reference: oklch(62.8% 0.2577 29.23) is sRGB red.
        let red = parse_hex("oklch(62.8% 0.2577 29.23)").expect("valid oklch");
        assert!(
            (red >> 16 & 0xFF) >= 0xFD && (red >> 8 & 0xFF) <= 2 && (red & 0xFF) <= 2,
            "{red:08x}"
        );
        assert_eq!(parse_hex("oklch(nope)"), None);
    }
}
