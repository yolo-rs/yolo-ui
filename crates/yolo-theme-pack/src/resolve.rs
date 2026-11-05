//! Pack plus user preferences into a resolved theme, its CSS and its asset map.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use serde::Serialize;
use yolo_tokens::palette::TonalPalette;
use yolo_tokens::role::Role;
use yolo_tokens::scheme::{DynamicScheme, Variant};

use crate::manifest::{
    ColorSource, IconStyle, LogoSpec, Mode, MotionProfile, PetSpec, ShellLayout, ThemePack, Titles,
    Typography, WallpaperFit, parse_hex,
};

/// Version of the resolved JSON layout; bumped on any breaking change.
pub const SCHEMA_VERSION: u32 = 1;
use crate::wallpaper::generated_wallpaper_svg;

/// What the user changed on top of the pack. Every field is optional; `None` keeps the pack value.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct UserPrefs {
    /// Custom seed `#RRGGBB`.
    pub seed: Option<String>,
    /// Seed source override.
    pub color_source: Option<ColorSource>,
    /// Dynamic scheme variant name.
    pub variant: Option<String>,
    /// Contrast level, clamped to -1.0..=1.0.
    pub contrast: Option<f64>,
    /// Mode override.
    pub mode: Option<Mode>,
    /// Index into the pack wallpapers.
    pub wallpaper: Option<usize>,
    /// Seed extracted by the caller from the active wallpaper image, when the manifest has none.
    pub wallpaper_seed: Option<u32>,
    /// Icon style override.
    pub icon_style: Option<IconStyle>,
    /// Shell layout override.
    pub layout: Option<ShellLayout>,
    /// Motion override; set to `Still` when the OS asks for reduced motion.
    pub motion: Option<MotionProfile>,
    /// Per-role colour overrides, applied after the pack overrides.
    pub overrides: BTreeMap<String, String>,
    /// Adjust `on-*` text roles that fall below AA instead of only reporting them.
    pub clamp_text: bool,
}

/// The active wallpaper.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum ActiveWallpaper {
    /// A file shipped with the pack, relative to the pack directory.
    File {
        /// Path inside the pack.
        path: String,
        /// Fill mode.
        fit: WallpaperFit,
        /// Focal point in percent.
        focal: [u8; 2],
        /// Expected SHA-256 of the file.
        sha256: Option<String>,
    },
    /// The SVG generated from the scheme.
    Generated {
        /// SVG document.
        svg: String,
        /// Seed the composition was derived from (`#RRGGBB`).
        seed: String,
    },
}

/// A fully resolved theme: what every surface needs to render it.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ResolvedTheme {
    /// Pack id.
    pub pack: String,
    /// Product and window strings.
    pub title: Titles,
    /// Logo files inside the pack.
    pub logo: Option<LogoSpec>,
    /// Whether the dark scheme is active.
    pub dark: bool,
    /// Seed actually used, `#RRGGBB`.
    pub seed: String,
    /// Variant actually used.
    pub variant: String,
    /// Contrast level actually used.
    pub contrast: f64,
    /// The 49 M3 system colour roles plus `focus-ring`, as (`role-name`, `#RRGGBB`).
    pub roles: Vec<(String, String)>,
    /// Active wallpaper.
    pub wallpaper: ActiveWallpaper,
    /// Icon set id.
    pub icon_set: String,
    /// Icon style.
    pub icon_style: IconStyle,
    /// Icon weight.
    pub icon_weight: u16,
    /// Companion pet.
    pub pet: Option<PetSpec>,
    /// Typography.
    pub typography: Typography,
    /// Shell layout.
    pub layout: ShellLayout,
    /// Base spacing unit in px.
    pub spacing: u16,
    /// Radii in px.
    pub radius: [u16; 3],
    /// Glass blur in px.
    pub glass_blur: u16,
    /// Motion duration in ms and easing.
    pub motion: (u16, &'static str),
    /// Role pairs still below WCAG AA (4.5:1), as (`on-role`, `role`, ratio).
    pub contrast_warnings: Vec<(String, String, f64)>,
    /// `on-*` roles adjusted to reach AA because `clamp_text` was set.
    pub clamped_roles: Vec<String>,
}

fn hex(argb: u32) -> String {
    format!("#{:06x}", argb & 0x00FF_FFFF)
}

/// Roles painted from the neutral palette.
const NEUTRAL_ROLES: [&str; 16] = [
    "background", "on-background", "surface", "on-surface", "surface-dim", "surface-bright",
    "surface-container-lowest", "surface-container-low", "surface-container",
    "surface-container-high", "surface-container-highest", "inverse-surface", "inverse-on-surface",
    "surface-variant", "on-surface-variant", "outline",
];

/// HCT tonal palette for an OKLCH hue and chroma, taken through the sRGB colour at L = 0.65.
fn oklch_palette(hue: f64, chroma: f64) -> TonalPalette {
    let argb = parse_hex(&format!("oklch(0.65 {chroma} {hue})")).unwrap_or(0xFF80_8080);
    TonalPalette::from_argb(argb)
}

fn motion_tokens(profile: MotionProfile) -> (u16, &'static str) {
    match profile {
        MotionProfile::Calm => (200, "cubic-bezier(0.2, 0, 0, 1)"),
        MotionProfile::Lively => (350, "cubic-bezier(0.34, 1.56, 0.64, 1)"),
        MotionProfile::Still => (0, "linear"),
    }
}

/// Resolves `pack` with the user's `prefs`. `system_dark` is the OS preference, used when the mode
/// is `System`. Unknown variant names fall back to `tonal-spot`; an invalid seed falls back to the
/// pack seed, then to the M3 baseline seed.
#[must_use]
pub fn resolve(pack: &ThemePack, prefs: &UserPrefs, system_dark: bool) -> ResolvedTheme {
    let mode = prefs.mode.unwrap_or(pack.color.mode);
    let dark = match mode {
        Mode::System => system_dark,
        Mode::Light => false,
        Mode::Dark => true,
    };

    let wallpapers: Vec<_> = pack
        .wallpapers
        .iter()
        .filter(|w| {
            w.mode
                .is_none_or(|m| m == Mode::System || (m == Mode::Dark) == dark)
        })
        .collect();
    let chosen = prefs
        .wallpaper
        .and_then(|i| wallpapers.get(i).copied())
        .or_else(|| wallpapers.first().copied());

    let pack_seed = parse_hex(&pack.color.seed).unwrap_or(0xFF67_50A4);
    // An explicit user seed wins unless the user explicitly chose the wallpaper as the source.
    let source = prefs.color_source.unwrap_or(if prefs.seed.is_some() {
        ColorSource::Seed
    } else {
        pack.color.source
    });
    let seed = match source {
        ColorSource::Wallpaper => chosen
            .and_then(|w| w.seed.as_deref().and_then(parse_hex))
            .or(prefs.wallpaper_seed)
            .unwrap_or(pack_seed),
        ColorSource::Seed => prefs
            .seed
            .as_deref()
            .and_then(parse_hex)
            .unwrap_or(pack_seed),
    };

    let variant_name = prefs.variant.as_deref().unwrap_or(&pack.color.variant);
    let (variant, variant_name) = Variant::from_name(variant_name)
        .map_or((Variant::TonalSpot, "tonal-spot"), |v| (v, variant_name));
    let contrast = prefs
        .contrast
        .unwrap_or(pack.color.contrast)
        .clamp(-1.0, 1.0);

    let scheme = DynamicScheme::from_argb(seed, variant, dark, contrast);
    let mut roles: Vec<(String, String)> = Role::ALL
        .iter()
        .map(|r| (r.name().to_owned(), hex(scheme.argb(*r))))
        .collect();
    // Independent secondary hue and neutral chroma: same tones as the scheme, other palettes.
    if let Some(sec) = pack.color.secondary {
        let palette = oklch_palette(sec.hue, sec.chroma);
        for (role, slot) in Role::ALL.iter().zip(roles.iter_mut()) {
            if slot.0.contains("secondary") {
                slot.1 = hex(palette.tone(scheme.tone(*role)));
            }
        }
    }
    if let Some(chroma) = pack.color.neutral_chroma {
        let hue = yolo_tokens::hct::Hct::from_argb(seed).hue;
        let palette = if chroma <= 0.0 {
            TonalPalette::from_hue_and_chroma(hue, 0.0)
        } else {
            oklch_palette(hue, chroma)
        };
        for (role, slot) in Role::ALL.iter().zip(roles.iter_mut()) {
            if NEUTRAL_ROLES.contains(&slot.0.as_str()) {
                slot.1 = hex(palette.tone(scheme.tone(*role)));
            }
        }
    }
    let primary = roles.iter().find(|(n, _)| n == "primary").map_or_else(String::new, |(_, v)| v.clone());
    roles.push(("focus-ring".to_owned(), primary));
    for (name, value) in pack.color.overrides.iter().chain(prefs.overrides.iter()) {
        if let (Some(slot), Some(argb)) =
            (roles.iter_mut().find(|(n, _)| n == name), parse_hex(value))
        {
            slot.1 = hex(argb);
        }
    }
    let clamped_roles = if prefs.clamp_text {
        clamp_text_roles(&mut roles)
    } else {
        Vec::new()
    };
    let contrast_warnings = contrast_warnings(&roles);

    let wallpaper = match chosen {
        Some(w) => ActiveWallpaper::File {
            path: w.file.clone(),
            fit: w.fit,
            focal: w.focal,
            sha256: w.sha256.clone(),
        },
        None => ActiveWallpaper::Generated {
            svg: generated_wallpaper_svg(&roles, seed),
            seed: hex(seed),
        },
    };

    let density = i16::from(pack.shell.density.clamp(-2, 2));
    ResolvedTheme {
        pack: pack.id.clone(),
        title: pack.title.clone(),
        logo: pack.logo.clone(),
        dark,
        seed: hex(seed),
        variant: variant_name.to_owned(),
        contrast,
        roles,
        wallpaper,
        icon_set: pack.icons.set.clone(),
        icon_style: prefs.icon_style.unwrap_or(pack.icons.style),
        icon_weight: pack.icons.weight,
        pet: pack.pet.clone(),
        typography: pack.typography.clone(),
        layout: prefs.layout.unwrap_or(pack.shell.layout),
        spacing: u16::try_from(8 + 2 * density).unwrap_or(8),
        radius: pack.shell.radius,
        glass_blur: pack.shell.glass_blur,
        motion: motion_tokens(prefs.motion.unwrap_or(pack.shell.motion)),
        contrast_warnings,
        clamped_roles,
    }
}

/// Moves each failing `on-*` role toward black or white, whichever direction its base calls for,
/// until it reaches 4.5:1, keeping as much of the original colour as possible. Returns the names.
fn clamp_text_roles(roles: &mut [(String, String)]) -> Vec<String> {
    let snapshot: Vec<(String, String)> = roles.to_vec();
    let get = |n: &str| {
        snapshot
            .iter()
            .find(|(r, _)| r == n)
            .and_then(|(_, v)| parse_hex(v))
    };
    let mut clamped = Vec::new();
    for (name, value) in roles.iter_mut() {
        let Some(base) = name.strip_prefix("on-").and_then(get) else {
            continue;
        };
        let Some(fg) = parse_hex(value) else { continue };
        if contrast_ratio(fg, base) >= 4.5 {
            continue;
        }
        let toward = if luminance(base) > 0.18 {
            0xFF00_0000
        } else {
            0xFFFF_FFFF
        };
        let fixed = (1..=20)
            .map(|step| mix(fg, toward, f64::from(step) / 20.0))
            .find(|c| contrast_ratio(*c, base) >= 4.5)
            .unwrap_or(toward);
        *value = hex(fixed);
        clamped.push(name.clone());
    }
    clamped
}

fn mix(a: u32, b: u32, t: f64) -> u32 {
    let ch = |c: u32, shift: u32| f64::from((c >> shift) & 0xFF);
    let lerp = |shift: u32| {
        let v = ch(a, shift) + (ch(b, shift) - ch(a, shift)) * t;
        // Rounded and clamped to 0..=255 first, so the cast cannot truncate or wrap.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let v = v.round().clamp(0.0, 255.0) as u32;
        v << shift
    };
    0xFF00_0000 | lerp(16) | lerp(8) | lerp(0)
}

fn channel(c: u32) -> f64 {
    let c = f64::from(c & 0xFF) / 255.0;
    if c <= 0.040_45 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn luminance(argb: u32) -> f64 {
    0.2126 * channel(argb >> 16) + 0.7152 * channel(argb >> 8) + 0.0722 * channel(argb)
}

/// WCAG contrast ratio between two opaque colours.
#[must_use]
pub fn contrast_ratio(a: u32, b: u32) -> f64 {
    let (la, lb) = (luminance(a), luminance(b));
    (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
}

/// Every `on-X` / `X` pair below 4.5:1.
fn contrast_warnings(roles: &[(String, String)]) -> Vec<(String, String, f64)> {
    let get = |n: &str| {
        roles
            .iter()
            .find(|(r, _)| r == n)
            .and_then(|(_, v)| parse_hex(v))
    };
    roles
        .iter()
        .filter_map(|(on, fg)| {
            let base = on.strip_prefix("on-")?;
            let ratio = contrast_ratio(parse_hex(fg)?, get(base)?);
            (ratio < 4.5).then(|| (on.clone(), base.to_owned(), (ratio * 100.0).round() / 100.0))
        })
        .collect()
}

impl ResolvedTheme {
    /// Value of a colour role, `#RRGGBB`.
    #[must_use]
    pub fn role(&self, name: &str) -> Option<&str> {
        self.roles
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.as_str())
    }

    /// CSS custom properties for the whole theme, scoped to `:root`. The shell reads layout,
    /// icon style and pet anchor from `data-*` attributes set from [`Self::attributes`].
    #[must_use]
    pub fn css(&self, wallpaper_url: Option<&str>) -> String {
        let mut out = format!(
            "/* yolo theme pack {} ({}, seed {}, {}) */\n:root {{\n  color-scheme: {};\n",
            self.pack,
            if self.dark { "dark" } else { "light" },
            self.seed,
            self.variant,
            if self.dark { "dark" } else { "light" }
        );
        for (name, value) in &self.roles {
            let _ = writeln!(out, "  --yolo-{name}: {value};");
        }
        let [sm, md, lg] = self.radius;
        let (duration, easing) = self.motion;
        let _ = writeln!(out, "  --yolo-space: {}px;", self.spacing);
        let _ = writeln!(
            out,
            "  --yolo-radius-sm: {sm}px;\n  --yolo-radius-md: {md}px;\n  --yolo-radius-lg: {lg}px;"
        );
        let _ = writeln!(out, "  --yolo-glass-blur: {}px;", self.glass_blur);
        let _ = writeln!(
            out,
            "  --yolo-motion-duration: {duration}ms;\n  --yolo-motion-easing: {easing};"
        );
        let _ = writeln!(
            out,
            "  --yolo-font-display: \"{}\";\n  --yolo-font-body: \"{}\";\n  --yolo-font-mono: \"{}\";",
            self.typography.display, self.typography.body, self.typography.mono
        );
        if let Some(url) = wallpaper_url {
            let _ = writeln!(
                out,
                "  --yolo-wallpaper: url(\"{}\");",
                url.replace('"', "%22")
            );
        }
        out.push_str("}\n@media (prefers-reduced-motion: reduce) {\n  :root { --yolo-motion-duration: 0ms; }\n}\n");
        out
    }

    /// `data-*` attributes the shell sets on its root element.
    #[must_use]
    pub fn attributes(&self) -> [(&'static str, String); 4] {
        [
            ("data-theme-pack", self.pack.clone()),
            ("data-shell", self.layout.name().to_owned()),
            ("data-icon-style", self.icon_style.name().to_owned()),
            (
                "data-pet-anchor",
                self.pet
                    .as_ref()
                    .map_or_else(String::new, |p| p.anchor.clone()),
            ),
        ]
    }

    /// The resolved theme as one JSON document for Bun and web surfaces: `schema_version`, a
    /// `content_hash` (SHA-256 of the theme payload) to detect drift, and the `theme` itself.
    /// Consumers read the whole document and apply it at once, so a switch is never partial.
    ///
    /// # Errors
    /// Fails only if serialization fails, which the types here do not allow.
    pub fn to_json(&self) -> serde_json::Result<String> {
        use sha2::Digest as _;
        let payload = serde_json::to_value(self)?;
        let digest = sha2::Sha256::digest(serde_json::to_vec(&payload)?);
        let content_hash: String = digest.iter().map(|b| format!("{b:02x}")).collect();
        serde_json::to_string_pretty(&serde_json::json!({
            "schema_version": SCHEMA_VERSION,
            "content_hash": content_hash,
            "theme": payload,
        }))
    }
}

/// Seed from wallpaper image bytes (dominant colour, as Android dynamic colour does).
///
/// # Errors
/// Returns the decode error when the bytes are not a supported image.
#[cfg(feature = "image")]
pub fn seed_from_image_bytes(bytes: &[u8]) -> Result<u32, ::yolo_tokens::image::ImageError> {
    yolo_tokens::image::source_color_from_image_bytes(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::{ColorSpec, IconSpec, ShellSpec, Wallpaper};

    fn pack() -> ThemePack {
        ThemePack::from_toml(include_str!("../../../themes/aurora-terminal/theme.toml"))
            .expect("aurora-terminal pack parses")
    }

    #[test]
    fn packs_parse_and_resolve_both_modes() {
        for text in [
            include_str!("../../../themes/aurora-terminal/theme.toml"),
            include_str!("../../../themes/paper-studio/theme.toml"),
        ] {
            let p = ThemePack::from_toml(text).expect("pack parses");
            for dark in [false, true] {
                let t = resolve(
                    &p,
                    &UserPrefs {
                        mode: Some(if dark { Mode::Dark } else { Mode::Light }),
                        ..UserPrefs::default()
                    },
                    false,
                );
                assert_eq!(t.roles.len(), 50);
                assert_eq!(t.dark, dark);
                assert!(t.css(None).contains("--yolo-primary: #"));
            }
        }
    }

    #[test]
    fn user_seed_changes_colours_without_touching_the_pack() {
        let p = pack();
        let base = resolve(&p, &UserPrefs::default(), true);
        let custom = resolve(
            &p,
            &UserPrefs {
                seed: Some("#e0a526".into()),
                ..UserPrefs::default()
            },
            true,
        );
        assert_ne!(base.role("primary"), custom.role("primary"));
        assert_eq!(custom.seed, "#e0a526");
        assert_eq!(p.color.seed, pack().color.seed);
    }

    #[test]
    fn wallpaper_source_uses_the_wallpaper_seed() {
        let mut p = pack();
        p.color.source = ColorSource::Wallpaper;
        p.wallpapers = vec![Wallpaper {
            file: "w.webp".into(),
            mode: None,
            seed: Some("#2f6f73".into()),
            license: "CC0-1.0".into(),
            source: "original".into(),
            attribution: None,
            fit: WallpaperFit::Cover,
            focal: [50, 50],
            sha256: None,
        }];
        let t = resolve(&p, &UserPrefs::default(), false);
        assert_eq!(t.seed, "#2f6f73");
        assert!(matches!(t.wallpaper, ActiveWallpaper::File { ref path, .. } if path == "w.webp"));
    }

    #[test]
    fn no_wallpaper_means_generated_svg() {
        let t = resolve(&pack(), &UserPrefs::default(), true);
        match t.wallpaper {
            ActiveWallpaper::Generated { svg, .. } => assert!(svg.starts_with("<svg")),
            ActiveWallpaper::File { .. } => panic!("expected generated wallpaper"),
        }
    }

    #[test]
    fn layout_and_motion_overrides_apply() {
        let t = resolve(
            &pack(),
            &UserPrefs {
                layout: Some(ShellLayout::Dock),
                motion: Some(MotionProfile::Still),
                ..UserPrefs::default()
            },
            false,
        );
        assert_eq!(t.layout, ShellLayout::Dock);
        assert_eq!(t.motion.0, 0);
        assert!(
            t.attributes()
                .iter()
                .any(|(k, v)| *k == "data-shell" && v == "dock")
        );
    }

    #[test]
    fn generated_schemes_meet_aa_and_bad_overrides_are_reported() {
        let clean = resolve(&pack(), &UserPrefs::default(), true);
        assert!(
            clean.contrast_warnings.is_empty(),
            "{:?}",
            clean.contrast_warnings
        );
        let mut overrides = BTreeMap::new();
        overrides.insert(
            "on-primary".to_owned(),
            clean.role("primary").unwrap_or("#000000").to_owned(),
        );
        overrides.insert("not-a-role".to_owned(), "#ffffff".to_owned());
        let bad = resolve(
            &pack(),
            &UserPrefs {
                overrides,
                ..UserPrefs::default()
            },
            true,
        );
        assert_eq!(bad.role("on-primary"), clean.role("primary"));
        assert!(
            bad.contrast_warnings
                .iter()
                .any(|(on, base, _)| on == "on-primary" && base == "primary")
        );
    }

    #[test]
    fn clamp_text_repairs_failing_text_roles() {
        let clean = resolve(&pack(), &UserPrefs::default(), true);
        let mut overrides = BTreeMap::new();
        overrides.insert(
            "on-primary".to_owned(),
            clean.role("primary").unwrap_or("#000000").to_owned(),
        );
        let fixed = resolve(
            &pack(),
            &UserPrefs {
                overrides,
                clamp_text: true,
                ..UserPrefs::default()
            },
            true,
        );
        assert_eq!(fixed.clamped_roles, vec!["on-primary".to_owned()]);
        assert!(
            fixed.contrast_warnings.is_empty(),
            "{:?}",
            fixed.contrast_warnings
        );
    }

    #[test]
    fn json_carries_schema_version_and_stable_hash() {
        let t = resolve(&pack(), &UserPrefs::default(), true);
        let a: serde_json::Value =
            serde_json::from_str(&t.to_json().expect("json")).expect("parse");
        let b: serde_json::Value =
            serde_json::from_str(&t.to_json().expect("json")).expect("parse");
        assert_eq!(a["schema_version"], SCHEMA_VERSION);
        assert_eq!(a["content_hash"], b["content_hash"]);
        assert_eq!(a["theme"]["roles"].as_array().map(Vec::len), Some(50));
        let other = resolve(
            &pack(),
            &UserPrefs {
                seed: Some("#e0a526".into()),
                ..UserPrefs::default()
            },
            true,
        );
        let c: serde_json::Value =
            serde_json::from_str(&other.to_json().expect("json")).expect("parse");
        assert_ne!(a["content_hash"], c["content_hash"]);
    }

    #[test]
    fn secondary_neutral_and_focus_ring() {
        let mut p = pack();
        p.color.secondary = Some(crate::manifest::OklchHueChroma { hue: 84.0, chroma: 0.164 });
        p.color.neutral_chroma = Some(0.0);
        p.color.overrides.insert("focus-ring".into(), "#ffdd00".into());
        let base = resolve(&pack(), &UserPrefs::default(), true);
        let t = resolve(&p, &UserPrefs::default(), true);
        assert_ne!(base.role("secondary"), t.role("secondary"));
        let grey = |v: &str| parse_hex(v).is_some_and(|c| (c >> 16 & 0xFF) == (c >> 8 & 0xFF) && (c >> 8 & 0xFF) == (c & 0xFF));
        assert!(t.role("surface").is_some_and(grey), "{:?}", t.role("surface"));
        assert_eq!(t.role("focus-ring"), Some("#ffdd00"));
        assert_eq!(base.role("focus-ring"), base.role("primary"));
        assert!(!crate::manifest::is_distributable_license("pending-owner-decision"));
        assert!(crate::manifest::is_distributable_license("CC0-1.0"));
    }

    #[test]
    fn invalid_inputs_fall_back() {
        let p = ThemePack {
            color: ColorSpec {
                seed: "nope".into(),
                variant: "unknown".into(),
                ..pack().color
            },
            icons: IconSpec {
                set: "phosphor".into(),
                style: IconStyle::Filled,
                weight: 400,
            },
            shell: ShellSpec {
                density: 9,
                ..pack().shell
            },
            ..pack()
        };
        let t = resolve(
            &p,
            &UserPrefs {
                contrast: Some(5.0),
                ..UserPrefs::default()
            },
            false,
        );
        assert_eq!(t.variant, "tonal-spot");
        assert_eq!(t.seed, "#6750a4");
        assert!((t.contrast - 1.0).abs() < f64::EPSILON);
        assert_eq!(t.spacing, 12);
    }
}
