//! Pack plus user preferences into a resolved theme, its CSS and its asset map.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use serde::Serialize;
use yolo_tokens::role::Role;
use yolo_tokens::scheme::{DynamicScheme, Variant};

use crate::manifest::{
    ColorSource, IconStyle, Mode, MotionProfile, PetSpec, ShellLayout, ThemePack, Typography,
    parse_hex,
};
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
}

/// The active wallpaper.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum ActiveWallpaper {
    /// A file shipped with the pack, relative to the pack directory.
    File {
        /// Path inside the pack.
        path: String,
    },
    /// The SVG generated from the scheme.
    Generated {
        /// SVG document.
        svg: String,
    },
}

/// A fully resolved theme: what every surface needs to render it.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ResolvedTheme {
    /// Pack id.
    pub pack: String,
    /// Window and shell title.
    pub title: String,
    /// Logo path inside the pack.
    pub logo: Option<String>,
    /// Whether the dark scheme is active.
    pub dark: bool,
    /// Seed actually used, `#RRGGBB`.
    pub seed: String,
    /// Variant actually used.
    pub variant: String,
    /// Contrast level actually used.
    pub contrast: f64,
    /// The 49 system colour roles as (`role-name`, `#RRGGBB`).
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
    /// Role pairs below WCAG AA (4.5:1) after overrides, as (`on-role`, `role`, ratio).
    pub contrast_warnings: Vec<(String, String, f64)>,
}

fn hex(argb: u32) -> String {
    format!("#{:06x}", argb & 0x00FF_FFFF)
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
        .filter(|w| w.mode.is_none_or(|m| m == Mode::System || (m == Mode::Dark) == dark))
        .collect();
    let chosen = prefs
        .wallpaper
        .and_then(|i| wallpapers.get(i).copied())
        .or_else(|| wallpapers.first().copied());

    let pack_seed = parse_hex(&pack.color.seed).unwrap_or(0xFF67_50A4);
    let seed = match prefs.color_source.unwrap_or(pack.color.source) {
        ColorSource::Wallpaper => chosen
            .and_then(|w| w.seed.as_deref().and_then(parse_hex))
            .or(prefs.wallpaper_seed)
            .unwrap_or(pack_seed),
        ColorSource::Seed => prefs.seed.as_deref().and_then(parse_hex).unwrap_or(pack_seed),
    };

    let variant_name = prefs.variant.as_deref().unwrap_or(&pack.color.variant);
    let (variant, variant_name) = Variant::from_name(variant_name)
        .map_or((Variant::TonalSpot, "tonal-spot"), |v| (v, variant_name));
    let contrast = prefs.contrast.unwrap_or(pack.color.contrast).clamp(-1.0, 1.0);

    let scheme = DynamicScheme::from_argb(seed, variant, dark, contrast);
    let mut roles: Vec<(String, String)> =
        Role::ALL.iter().map(|r| (r.name().to_owned(), hex(scheme.argb(*r)))).collect();
    for (name, value) in pack.color.overrides.iter().chain(prefs.overrides.iter()) {
        if let (Some(slot), Some(argb)) = (roles.iter_mut().find(|(n, _)| n == name), parse_hex(value)) {
            slot.1 = hex(argb);
        }
    }
    let contrast_warnings = contrast_warnings(&roles);

    let wallpaper = match chosen {
        Some(w) => ActiveWallpaper::File { path: w.file.clone() },
        None => ActiveWallpaper::Generated {
            svg: generated_wallpaper_svg(&roles, seed),
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
    }
}

fn channel(c: u32) -> f64 {
    let c = f64::from(c & 0xFF) / 255.0;
    if c <= 0.040_45 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
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
    let get = |n: &str| roles.iter().find(|(r, _)| r == n).and_then(|(_, v)| parse_hex(v));
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
        self.roles.iter().find(|(n, _)| n == name).map(|(_, v)| v.as_str())
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
        let _ = writeln!(out, "  --yolo-radius-sm: {sm}px;\n  --yolo-radius-md: {md}px;\n  --yolo-radius-lg: {lg}px;");
        let _ = writeln!(out, "  --yolo-glass-blur: {}px;", self.glass_blur);
        let _ = writeln!(out, "  --yolo-motion-duration: {duration}ms;\n  --yolo-motion-easing: {easing};");
        let _ = writeln!(
            out,
            "  --yolo-font-display: \"{}\";\n  --yolo-font-body: \"{}\";\n  --yolo-font-mono: \"{}\";",
            self.typography.display, self.typography.body, self.typography.mono
        );
        if let Some(url) = wallpaper_url {
            let _ = writeln!(out, "  --yolo-wallpaper: url(\"{}\");", url.replace('"', "%22"));
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
                self.pet.as_ref().map_or_else(String::new, |p| p.anchor.clone()),
            ),
        ]
    }

    /// The resolved theme as JSON, for Bun and web surfaces.
    ///
    /// # Errors
    /// Fails only if serialization fails, which the types here do not allow.
    pub fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string_pretty(self)
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
                let t = resolve(&p, &UserPrefs { mode: Some(if dark { Mode::Dark } else { Mode::Light }), ..UserPrefs::default() }, false);
                assert_eq!(t.roles.len(), 49);
                assert_eq!(t.dark, dark);
                assert!(t.css(None).contains("--yolo-primary: #"));
            }
        }
    }

    #[test]
    fn user_seed_changes_colours_without_touching_the_pack() {
        let p = pack();
        let base = resolve(&p, &UserPrefs::default(), true);
        let custom = resolve(&p, &UserPrefs { seed: Some("#e0a526".into()), ..UserPrefs::default() }, true);
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
        }];
        let t = resolve(&p, &UserPrefs::default(), false);
        assert_eq!(t.seed, "#2f6f73");
        assert_eq!(t.wallpaper, ActiveWallpaper::File { path: "w.webp".into() });
    }

    #[test]
    fn no_wallpaper_means_generated_svg() {
        let t = resolve(&pack(), &UserPrefs::default(), true);
        match t.wallpaper {
            ActiveWallpaper::Generated { svg } => assert!(svg.starts_with("<svg")),
            ActiveWallpaper::File { .. } => panic!("expected generated wallpaper"),
        }
    }

    #[test]
    fn layout_and_motion_overrides_apply() {
        let t = resolve(
            &pack(),
            &UserPrefs { layout: Some(ShellLayout::Dock), motion: Some(MotionProfile::Still), ..UserPrefs::default() },
            false,
        );
        assert_eq!(t.layout, ShellLayout::Dock);
        assert_eq!(t.motion.0, 0);
        assert!(t.attributes().iter().any(|(k, v)| *k == "data-shell" && v == "dock"));
    }

    #[test]
    fn generated_schemes_meet_aa_and_bad_overrides_are_reported() {
        let clean = resolve(&pack(), &UserPrefs::default(), true);
        assert!(clean.contrast_warnings.is_empty(), "{:?}", clean.contrast_warnings);
        let mut overrides = BTreeMap::new();
        overrides.insert("on-primary".to_owned(), clean.role("primary").unwrap_or("#000000").to_owned());
        overrides.insert("not-a-role".to_owned(), "#ffffff".to_owned());
        let bad = resolve(&pack(), &UserPrefs { overrides, ..UserPrefs::default() }, true);
        assert_eq!(bad.role("on-primary"), clean.role("primary"));
        assert!(bad.contrast_warnings.iter().any(|(on, base, _)| on == "on-primary" && base == "primary"));
    }

    #[test]
    fn invalid_inputs_fall_back() {
        let p = ThemePack {
            color: ColorSpec { seed: "nope".into(), variant: "unknown".into(), ..pack().color },
            icons: IconSpec { set: "phosphor".into(), style: IconStyle::Filled, weight: 400 },
            shell: ShellSpec { density: 9, ..pack().shell },
            ..pack()
        };
        let t = resolve(&p, &UserPrefs { contrast: Some(5.0), ..UserPrefs::default() }, false);
        assert_eq!(t.variant, "tonal-spot");
        assert_eq!(t.seed, "#6750a4");
        assert!((t.contrast - 1.0).abs() < f64::EPSILON);
        assert_eq!(t.spacing, 12);
    }
}
