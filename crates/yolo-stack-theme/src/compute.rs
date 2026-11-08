//! `compute`: bundle plus user preferences into everything a surface renders.
//!
//! Colours come from yolo-theme-pack (HCT dynamic schemes of yolo-tokens). This module maps the
//! stack preferences onto it, enforces the contrast gates and resolves the assets.

use std::collections::BTreeMap;

use serde::Serialize;
use yolo_theme_pack::manifest::{ColorSource, Mode, MotionProfile, parse_hex};
use yolo_theme_pack::resolve::{self, UserPrefs as PackPrefs};
use yolo_theme_pack::wallpaper::generated_wallpaper_svg;
use yolo_tokens::hct::Hct;

use crate::assets::{CdnConfig, ResolvedAssets, resolve_assets};
use crate::bundle::ThemeBundle;
use crate::gate::{self, GateReport, Roles};
use crate::prefs::{SchemePref, UserThemePrefs, WallpaperChoice};

/// Corner radii, spacing, glass and motion.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Shape {
    /// Small, medium and large radii in px.
    pub radius: [u16; 3],
    /// Base spacing unit in px.
    pub spacing: u16,
    /// Backdrop blur in px (0: no glass).
    pub glass_blur: u16,
    /// Motion duration in ms (0 when reduced).
    pub motion_ms: u16,
    /// Motion easing.
    pub motion_easing: String,
    /// Shell layout name (`sidebar`, `top-bar`, `dock`, `floating`).
    pub layout: String,
}

/// Font families.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Fonts {
    /// Display and titles.
    pub display: String,
    /// Header and navigation; equals `display` unless the bundle sets one.
    pub header: String,
    /// Body.
    pub body: String,
    /// Monospace.
    pub mono: String,
}

/// A per-role override and what became of it after the gates.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct OverrideOutcome {
    /// Role name.
    pub role: String,
    /// Value the user asked for, light scheme.
    pub requested: String,
    /// Value in the light scheme after the gates.
    pub light: String,
    /// Value in the dark scheme after the gates.
    pub dark: String,
    /// True when a gate had to move it (or its partner).
    pub adjusted: bool,
    /// True when no tone adjustment could reconcile it, so the computed value was kept.
    pub rejected: bool,
}

/// The computed theme.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ResolvedBundle {
    /// Bundle id.
    pub id: String,
    /// Display title.
    pub title: yolo_theme_pack::manifest::Titles,
    /// Seed actually used, `#rrggbb`.
    pub seed: String,
    /// Variant actually used.
    pub variant: String,
    /// Contrast level actually used.
    pub contrast: f64,
    /// Effective scheme preference after merging user choice and bundle default.
    pub scheme: SchemePref,
    /// The 50 roles in the light scheme, gated.
    pub light: Roles,
    /// The 50 roles in the dark scheme, gated.
    pub dark: Roles,
    /// Shape tokens.
    pub shape: Shape,
    /// Fonts.
    pub fonts: Fonts,
    /// Text roles the pack itself moved to reach AA.
    pub pack_clamped: Vec<String>,
    /// Assets by content hash.
    pub assets: ResolvedAssets,
    /// Set id of the chosen wallpaper; `None` means no wallpaper.
    pub wallpaper_set: Option<String>,
    /// Generated wallpapers (light, dark) when the bundle ships none.
    pub generated_wallpaper: Option<(String, String)>,
    /// Whether the pet is shown (user pref and not reduced motion).
    pub pet_enabled: bool,
    /// Reduced motion requested by the user.
    pub reduced_motion: bool,
    /// Gate outcome.
    pub gate: GateReport,
    /// Overrides and their outcome.
    pub overrides: Vec<OverrideOutcome>,
}

impl ResolvedBundle {
    /// Hue and chroma (HCT) of the seed actually used, for sliders.
    #[must_use]
    pub fn seed_hue_chroma(&self) -> (f64, f64) {
        parse_hex(&self.seed).map_or((0.0, 0.0), |argb| {
            let hct = Hct::from_argb(argb);
            (hct.hue, hct.chroma)
        })
    }

    /// Roles of a scheme.
    #[must_use]
    pub fn roles(&self, dark: bool) -> &Roles {
        if dark { &self.dark } else { &self.light }
    }

    /// Role value in a scheme.
    #[must_use]
    pub fn role(&self, dark: bool, name: &str) -> Option<&str> {
        self.roles(dark)
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.as_str())
    }

    /// Whether the dark scheme is active given the OS preference.
    #[must_use]
    pub fn is_dark(&self, system_dark: bool) -> bool {
        match self.scheme {
            SchemePref::System => system_dark,
            SchemePref::Light => false,
            SchemePref::Dark => true,
        }
    }
}

/// Computes a bundle with the default CDN base.
#[must_use]
pub fn compute(bundle: &ThemeBundle, prefs: &UserThemePrefs) -> ResolvedBundle {
    compute_with(bundle, prefs, &CdnConfig::default())
}

/// Computes a bundle.
#[must_use]
pub fn compute_with(
    bundle: &ThemeBundle,
    prefs: &UserThemePrefs,
    cdn: &CdnConfig,
) -> ResolvedBundle {
    let prefs = prefs.clone().sanitized();
    let pack = &bundle.pack;

    let base_seed = parse_hex(&pack.color.seed).unwrap_or(0xFF67_50A4);
    let seed = if prefs.hue.is_some() || prefs.chroma.is_some() {
        let base = Hct::from_argb(base_seed);
        let custom = Hct::new(
            prefs.hue.unwrap_or(base.hue),
            prefs.chroma.unwrap_or(base.chroma),
            50.0,
        );
        Some(format!("#{:06x}", custom.to_argb() & 0x00FF_FFFF))
    } else {
        None
    };

    let scheme = match prefs.scheme {
        SchemePref::System => match pack.color.mode {
            Mode::System => SchemePref::System,
            Mode::Light => SchemePref::Light,
            Mode::Dark => SchemePref::Dark,
        },
        explicit => explicit,
    };

    let build = |user_overrides: &BTreeMap<String, String>| {
        let pack_prefs = |mode: Mode| PackPrefs {
            seed: seed.clone(),
            color_source: seed.as_ref().map(|_| ColorSource::Seed),
            variant: prefs.variant.clone(),
            contrast: prefs.contrast,
            mode: Some(mode),
            clamp_text: true,
            motion: prefs.reduced_motion.then_some(MotionProfile::Still),
            overrides: user_overrides.clone(),
            ..PackPrefs::default()
        };
        let light_theme = resolve::resolve(pack, &pack_prefs(Mode::Light), false);
        let dark_theme = resolve::resolve(pack, &pack_prefs(Mode::Dark), true);
        let mut light = light_theme.roles.clone();
        let mut dark = dark_theme.roles.clone();
        let gate = gate::enforce_both(&mut light, &mut dark);
        (light_theme, dark_theme, light, dark, gate)
    };

    // Overrides are re-checked: a foreground moves along the tone axis, but an override that no
    // tone adjustment can reconcile (a surface forced to an extreme colour) is rejected.
    let mut active = prefs.overrides.clone();
    let mut rejected: Vec<String> = Vec::new();
    let (light_theme, dark_theme, light, dark, gate) = loop {
        let built = build(&active);
        if built.4.passes() || active.is_empty() {
            break built;
        }
        let implicated: Vec<String> = built
            .4
            .failures
            .iter()
            .flat_map(|f| [f.fg.clone(), f.bg.clone()])
            .filter(|role| active.contains_key(role))
            .collect();
        let drop: Vec<String> = if implicated.is_empty() {
            active.keys().cloned().collect()
        } else {
            implicated
        };
        for role in drop {
            active.remove(&role);
            rejected.push(role);
        }
    };
    let requested: BTreeMap<&String, &String> = pack
        .color
        .overrides
        .iter()
        .chain(prefs.overrides.iter())
        .collect();

    let overrides = requested
        .iter()
        .filter_map(|(role, value)| {
            let value_of = |roles: &Roles| {
                roles
                    .iter()
                    .find(|(n, _)| n == *role)
                    .map(|(_, v)| v.clone())
            };
            let (l, d) = (value_of(&light)?, value_of(&dark)?);
            Some(OverrideOutcome {
                role: (*role).clone(),
                requested: (*value).clone(),
                rejected: rejected.contains(*role),
                adjusted: !l.eq_ignore_ascii_case(value)
                    || gate.adjustments.iter().any(|a| &a.against == *role),
                light: l,
                dark: d,
            })
        })
        .collect();

    let assets = resolve_assets(bundle, cdn, light_theme.icon_style);
    let wallpaper_set = match &prefs.wallpaper {
        WallpaperChoice::Off => None,
        WallpaperChoice::Id(id) if assets.wallpapers.iter().any(|s| &s.id == id) => {
            Some(id.clone())
        }
        _ => assets.wallpapers.first().map(|s| s.id.clone()),
    };
    let seed_argb = parse_hex(&light_theme.seed).unwrap_or(base_seed);
    let generated_wallpaper =
        (assets.wallpapers.is_empty() && prefs.wallpaper != WallpaperChoice::Off).then(|| {
            (
                generated_wallpaper_svg(&light, seed_argb),
                generated_wallpaper_svg(&dark, seed_argb),
            )
        });

    let display = pack.typography.display.clone();
    ResolvedBundle {
        id: pack.id.clone(),
        title: pack.title.clone(),
        seed: light_theme.seed.clone(),
        variant: light_theme.variant.clone(),
        contrast: light_theme.contrast,
        scheme,
        light,
        dark,
        shape: Shape {
            radius: light_theme.radius,
            spacing: light_theme.spacing,
            glass_blur: light_theme.glass_blur,
            motion_ms: light_theme.motion.0,
            motion_easing: light_theme.motion.1.to_owned(),
            layout: light_theme.layout.name().to_owned(),
        },
        fonts: Fonts {
            header: bundle
                .extras
                .header_font
                .clone()
                .unwrap_or_else(|| display.clone()),
            display,
            body: pack.typography.body.clone(),
            mono: pack.typography.mono.clone(),
        },
        pet_enabled: prefs.pet && !prefs.reduced_motion && assets.pet.is_some(),
        reduced_motion: prefs.reduced_motion,
        pack_clamped: light_theme
            .clamped_roles
            .iter()
            .chain(dark_theme.clamped_roles.iter())
            .cloned()
            .collect(),
        assets,
        wallpaper_set,
        generated_wallpaper,
        gate,
        overrides,
    }
}
