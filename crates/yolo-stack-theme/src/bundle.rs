//! Bundle loading and validation.
//!
//! A bundle directory holds three files:
//! - `theme.toml`: the yolo-theme-pack manifest (colour rules, icons, typography, shell shape).
//! - `bundle.toml`: what the stack adds (brand variants, wallpaper sets per scheme and
//!   breakpoint, pet names and behaviours, optional sound and cursor, per-asset licences).
//! - `assets.lock.json`: sha256, size and media type of every file, written by the asset script.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use yolo_theme_pack::ThemePack;
use yolo_theme_pack::manifest::{Mode, parse_hex};
use yolo_tokens::{role::Role, scheme::Variant};

use crate::error::ThemeError;
use crate::prefs::is_hex;

/// Version of `bundle.toml` and `assets.lock.json`.
pub const SCHEMA: u32 = 1;

/// `bundle.toml`.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Extras {
    /// Must equal [`SCHEMA`].
    pub schema: u32,
    /// Brand variants.
    #[serde(default)]
    pub brand: Brand,
    /// Wallpaper sets.
    #[serde(default)]
    pub wallpaper: Vec<WallpaperSet>,
    /// Pet presentation (the pet itself is `pet` in `theme.toml`).
    #[serde(default)]
    pub pet: Option<PetExtras>,
    /// Optional sound pack (gap in yolo-theme-pack).
    #[serde(default)]
    pub sound: Option<SoundSpec>,
    /// Optional cursor image.
    #[serde(default)]
    pub cursor: Option<String>,
    /// Header font family (gap in yolo-theme-pack, which has display, body and mono).
    #[serde(default)]
    pub header_font: Option<String>,
    /// Licence record of every shipped file.
    #[serde(default)]
    pub asset: Vec<AssetMeta>,
}

/// Logo variants. The primary logo is `logo` in `theme.toml`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Brand {
    /// Logo variant for dark surfaces, when `currentColor` is not enough.
    #[serde(default)]
    pub logo_dark: Option<String>,
    /// Single colour mask of the logo, for CSS `mask-image`.
    #[serde(default)]
    pub logo_mask: Option<String>,
}

/// A wallpaper with one file per scheme and breakpoint.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WallpaperSet {
    /// Stable id used by [`crate::prefs::WallpaperChoice::Id`].
    pub id: String,
    /// Label in the picker.
    pub label: String,
    /// Files of this wallpaper.
    pub variant: Vec<WallpaperVariant>,
}

/// One wallpaper file.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WallpaperVariant {
    /// Path inside the bundle.
    pub file: String,
    /// Scheme it suits; `None` suits both.
    #[serde(default)]
    pub scheme: Option<Mode>,
    /// Smallest viewport width in CSS px it is meant for.
    #[serde(default)]
    pub min_width: u32,
    /// Pixel width.
    pub width: u32,
    /// Pixel height.
    pub height: u32,
    /// Dominant colour `#rrggbb`, shown while the image loads.
    pub dominant: String,
}

/// Pet presentation.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PetExtras {
    /// Per-theme display name.
    pub name: String,
    /// Vector sprite inside the bundle.
    pub sprite: String,
    /// Idle animation ids, played in turn.
    #[serde(default)]
    pub idle: Vec<String>,
    /// Behaviour ids (`follow-cursor`, `greet`, `sleep`, ...).
    #[serde(default)]
    pub behaviors: Vec<String>,
}

/// Optional sound pack.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SoundSpec {
    /// Event id (`click`, `success`, `error`) to file.
    pub events: BTreeMap<String, String>,
}

/// Licence record of one file.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AssetMeta {
    /// Path inside the bundle.
    pub file: String,
    /// SPDX id or `LicenseRef-*`.
    pub license: String,
    /// `original` or a URL.
    pub source: String,
    /// Credit line, if the licence needs one.
    #[serde(default)]
    pub attribution: Option<String>,
    /// True while the file is generated stand-in art that must be replaced.
    #[serde(default)]
    pub placeholder: bool,
}

/// `assets.lock.json`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
pub struct AssetLock {
    /// Must equal [`SCHEMA`].
    pub schema: u32,
    /// Files by bundle-relative path.
    pub files: BTreeMap<String, LockedFile>,
}

/// One locked file.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct LockedFile {
    /// Lowercase hex sha256 of the bytes.
    pub sha256: String,
    /// Size in bytes.
    pub bytes: u64,
    /// Media type.
    pub mime: String,
}

/// A loaded, validated bundle.
#[derive(Clone, Debug, PartialEq)]
pub struct ThemeBundle {
    /// The yolo-theme-pack manifest.
    pub pack: ThemePack,
    /// Stack extras.
    pub extras: Extras,
    /// Content hashes.
    pub lock: AssetLock,
}

impl ThemeBundle {
    /// Parses and validates the three files.
    ///
    /// # Errors
    /// A parse error with the file name, or [`ThemeError::Invalid`] listing every broken rule.
    pub fn from_strs(
        theme_toml: &str,
        bundle_toml: &str,
        lock_json: &str,
    ) -> Result<Self, ThemeError> {
        let pack = ThemePack::from_toml(theme_toml).map_err(|e| ThemeError::Pack(e.to_string()))?;
        let extras: Extras =
            toml::from_str(bundle_toml).map_err(|e| ThemeError::Extras(e.to_string()))?;
        let lock: AssetLock =
            serde_json::from_str(lock_json).map_err(|e| ThemeError::Lock(e.to_string()))?;
        let bundle = Self { pack, extras, lock };
        bundle.validate()?;
        Ok(bundle)
    }

    /// The built-in fallback: neutral seed, generated wallpaper, no external asset. Used when a
    /// bundle is missing or invalid, so the interface never renders unthemed.
    ///
    /// # Panics
    /// Never: the embedded manifests are covered by a unit test.
    #[must_use]
    pub fn fallback() -> Self {
        let pack = ThemePack::from_toml(FALLBACK_THEME).expect("fallback theme.toml is valid");
        let extras: Extras =
            toml::from_str(FALLBACK_BUNDLE).expect("fallback bundle.toml is valid");
        Self { pack, extras, lock: AssetLock { schema: SCHEMA, files: BTreeMap::new() } }
    }

    /// Bundle id.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.pack.id
    }

    /// Checks every rule and lists all problems at once.
    ///
    /// # Errors
    /// [`ThemeError::Invalid`] when at least one rule is broken.
    pub fn validate(&self) -> Result<(), ThemeError> {
        let mut issues = Vec::new();
        let p = &self.pack;
        if !is_kebab(&p.id) {
            issues.push(format!("id `{}` must be kebab-case ascii", p.id));
        }
        if p.title.product_name.trim().is_empty()
            || !p.title.window_title_template.contains("{page}")
        {
            issues.push(
                "title.product_name is empty or title.window_title_template lacks {page}".into(),
            );
        }
        if parse_hex(&p.color.seed).is_none() {
            issues.push(format!("color.seed `{}` is not #rrggbb", p.color.seed));
        }
        if Variant::from_name(&p.color.variant).is_none() {
            issues.push(format!("color.variant `{}` is unknown", p.color.variant));
        }
        if !(-1.0..=1.0).contains(&p.color.contrast) {
            issues.push(format!(
                "color.contrast {} outside -1..=1",
                p.color.contrast
            ));
        }
        for (role, value) in &p.color.overrides {
            if Role::from_name(role).is_none() {
                issues.push(format!("color.overrides: unknown role `{role}`"));
            }
            if !is_hex(value) {
                issues.push(format!("color.overrides.{role}: `{value}` is not #rrggbb"));
            }
        }
        if self.extras.schema != SCHEMA {
            issues.push(format!(
                "bundle.toml schema {} (expected {SCHEMA})",
                self.extras.schema
            ));
        }
        if self.lock.schema != SCHEMA {
            issues.push(format!(
                "assets.lock.json schema {} (expected {SCHEMA})",
                self.lock.schema
            ));
        }
        if let Some(pet) = &p.pet
            && !matches!(pet.provider.as_str(), "pack" | "openpets")
        {
            issues.push(format!(
                "pet.provider `{}` must be `pack` or `openpets`",
                pet.provider
            ));
        }
        if p.pet.is_some() != self.extras.pet.is_some() {
            issues.push("pet in theme.toml and [pet] in bundle.toml must both be present".into());
        }
        self.validate_files(&mut issues);
        if issues.is_empty() {
            Ok(())
        } else {
            Err(ThemeError::Invalid { id: p.id.clone(), issues })
        }
    }

    fn referenced_files(&self) -> Vec<&str> {
        let e = &self.extras;
        let mut files: Vec<&str> = Vec::new();
        if let Some(logo) = &self.pack.logo {
            files.push(logo.mark.as_str());
            files.push(logo.monochrome.as_str());
            files.extend(logo.wordmark.as_deref());
            files.extend(logo.png.values().map(String::as_str));
            files.extend(logo.ico.as_deref());
        }
        files.extend(e.brand.logo_dark.as_deref());
        files.extend(e.brand.logo_mask.as_deref());
        files.extend(e.cursor.as_deref());
        files.extend(e.pet.iter().map(|p| p.sprite.as_str()));
        files.extend(
            e.sound
                .iter()
                .flat_map(|s| s.events.values().map(String::as_str)),
        );
        files.extend(
            e.wallpaper
                .iter()
                .flat_map(|w| w.variant.iter().map(|v| v.file.as_str())),
        );
        files
    }

    fn validate_files(&self, issues: &mut Vec<String>) {
        let licensed: BTreeSet<&str> = self.extras.asset.iter().map(|a| a.file.as_str()).collect();
        for file in self.referenced_files() {
            if file.starts_with('/') || file.contains("..") || file.contains("://") {
                issues.push(format!(
                    "`{file}` must be a relative path inside the bundle"
                ));
            }
            match self.lock.files.get(file) {
                None => issues.push(format!("`{file}` is missing from assets.lock.json")),
                Some(lock)
                    if lock.sha256.len() != 64
                        || !lock.sha256.bytes().all(|b| b.is_ascii_hexdigit()) =>
                {
                    issues.push(format!("`{file}` has a malformed sha256"));
                }
                Some(_) => {}
            }
            if !licensed.contains(file) {
                issues.push(format!("`{file}` has no [[asset]] licence record"));
            }
        }
        for meta in &self.extras.asset {
            let license = meta.license.trim();
            if !yolo_theme_pack::manifest::is_distributable_license(license) {
                issues.push(format!(
                    "asset `{}` has no distributable licence (empty, unverified or pending)",
                    meta.file
                ));
            }
        }
        let mut seen = BTreeSet::new();
        for set in &self.extras.wallpaper {
            if !seen.insert(set.id.as_str()) {
                issues.push(format!("wallpaper id `{}` is duplicated", set.id));
            }
            if set.variant.is_empty() {
                issues.push(format!("wallpaper `{}` has no variant", set.id));
            }
            for v in &set.variant {
                if v.width == 0 || v.height == 0 {
                    issues.push(format!("wallpaper file `{}` has zero dimensions", v.file));
                }
                if !is_hex(&v.dominant) {
                    issues.push(format!(
                        "wallpaper file `{}` dominant `{}` is not #rrggbb",
                        v.file, v.dominant
                    ));
                }
            }
            // A set must serve a viewport of any width in both schemes.
            for dark in [false, true] {
                let covered = set.variant.iter().any(|v| {
                    v.min_width == 0
                        && v.scheme
                            .is_none_or(|m| m == Mode::System || (m == Mode::Dark) == dark)
                });
                if !covered {
                    issues.push(format!(
                        "wallpaper `{}` has no min_width = 0 variant for the {} scheme",
                        set.id,
                        if dark { "dark" } else { "light" }
                    ));
                }
            }
        }
    }
}

fn is_kebab(id: &str) -> bool {
    !id.is_empty()
        && !id.starts_with('-')
        && !id.ends_with('-')
        && id
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

const FALLBACK_THEME: &str = r##"
id = "fallback"
name = "Fallback"

[title]
product_name = "Shenron"
window_title_template = "{page} - Shenron"
ntp_heading = "Shenron"

[color]
seed = "#6750a4"
variant = "tonal-spot"

[icons]
set = "phosphor"
style = "outlined"
weight = 400

[typography]
display = "system-ui"
body = "system-ui"
mono = "ui-monospace"

[shell]
layout = "top-bar"
radius = [4, 8, 16]
motion = "calm"
"##;

const FALLBACK_BUNDLE: &str = "schema = 1\n";

/// All bundles of an app plus the default one.
#[derive(Clone, Debug)]
pub struct Registry {
    bundles: Vec<ThemeBundle>,
    default: usize,
    fallback: ThemeBundle,
}

impl Registry {
    /// Builds a registry.
    ///
    /// # Errors
    /// [`ThemeError::UnknownTheme`] when `default_id` is not among the bundles, or
    /// [`ThemeError::Invalid`] on duplicate ids.
    pub fn new(bundles: Vec<ThemeBundle>, default_id: &str) -> Result<Self, ThemeError> {
        let mut ids = BTreeSet::new();
        for b in &bundles {
            if !ids.insert(b.id().to_owned()) {
                return Err(ThemeError::Invalid {
                    id: b.id().to_owned(),
                    issues: vec!["duplicate bundle id".into()],
                });
            }
        }
        let default = bundles
            .iter()
            .position(|b| b.id() == default_id)
            .ok_or_else(|| ThemeError::UnknownTheme(default_id.to_owned()))?;
        Ok(Self { bundles, default, fallback: ThemeBundle::fallback() })
    }

    /// Bundle by id, or the default when the id is unknown.
    #[must_use]
    pub fn get_or_default(&self, id: Option<&str>) -> &ThemeBundle {
        id.and_then(|id| self.bundles.iter().find(|b| b.id() == id))
            .unwrap_or(&self.bundles[self.default])
    }

    /// The default bundle.
    #[must_use]
    pub fn default_bundle(&self) -> &ThemeBundle {
        &self.bundles[self.default]
    }

    /// The built-in fallback bundle.
    #[must_use]
    pub fn fallback(&self) -> &ThemeBundle {
        &self.fallback
    }

    /// All bundles in registration order.
    #[must_use]
    pub fn all(&self) -> &[ThemeBundle] {
        &self.bundles
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fallback_is_valid_and_has_no_assets() {
        let f = ThemeBundle::fallback();
        f.validate().expect("fallback validates");
        assert!(f.referenced_files().is_empty());
    }

    #[test]
    fn errors_list_every_problem() {
        let theme = FALLBACK_THEME
            .replace("seed = \"#6750a4\"", "seed = \"nope\"")
            .replace("tonal-spot", "wavy");
        let err = ThemeBundle::from_strs(&theme, "schema = 2\n", r#"{"schema":1,"files":{}}"#)
            .unwrap_err();
        let text = err.to_string();
        assert!(text.contains("color.seed"), "{text}");
        assert!(text.contains("color.variant"), "{text}");
        assert!(text.contains("schema 2"), "{text}");
    }

    #[test]
    fn unlocked_and_unlicensed_files_are_rejected() {
        let theme = format!(
            "{FALLBACK_THEME}
[logo]
mark = \"logo.svg\"
monochrome = \"logo-mono.svg\"
"
        );
        let err = ThemeBundle::from_strs(&theme, "schema = 1\n", r#"{"schema":1,"files":{}}"#)
            .unwrap_err();
        let text = err.to_string();
        assert!(text.contains("missing from assets.lock.json"), "{text}");
        assert!(text.contains("no [[asset]] licence"), "{text}");
    }
}
