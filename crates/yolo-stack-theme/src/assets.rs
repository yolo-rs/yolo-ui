//! Asset resolution: every file is addressed by content hash on the CDN.
//!
//! `https://cdn.example/<sha256 first 16 hex>.<ext>` never changes meaning, so it is served with
//! `Cache-Control: public, max-age=31536000, immutable`. Replacing art changes the hash, hence
//! the url, hence no purge is ever needed.

use serde::Serialize;
use yolo_theme_pack::manifest::{IconStyle, Mode, Titles};

use crate::bundle::{PetExtras, ThemeBundle, WallpaperVariant};

/// Where assets are served from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CdnConfig {
    /// Base url without trailing slash (`https://cdn.dragonballfr.com/t`, or `/theme-assets`).
    pub base: String,
}

impl Default for CdnConfig {
    fn default() -> Self {
        Self { base: "/theme-assets".to_owned() }
    }
}

/// Header to send with every asset url produced here.
pub const IMMUTABLE_CACHE_CONTROL: &str = "public, max-age=31536000, immutable";

/// A resolved file with its provenance.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct AssetRef {
    /// Path inside the bundle.
    pub path: String,
    /// Content-addressed url.
    pub url: String,
    /// Lowercase hex sha256.
    pub sha256: String,
    /// Size in bytes.
    pub bytes: u64,
    /// Media type.
    pub mime: String,
    /// SPDX id or `LicenseRef-*`.
    pub license: String,
    /// `original` or a URL.
    pub source: String,
    /// Credit line, if any.
    pub attribution: Option<String>,
    /// Generated stand-in art awaiting a commission.
    pub placeholder: bool,
}

/// Logo with its variants.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ResolvedLogo {
    /// Coloured mark.
    pub mark: Option<AssetRef>,
    /// Single colour mark using `currentColor`, so it follows the scheme.
    pub monochrome: Option<AssetRef>,
    /// Dedicated dark-surface variant.
    pub dark: Option<AssetRef>,
    /// Monochrome mask for CSS `mask-image`.
    pub mask: Option<AssetRef>,
    /// Wordmark.
    pub wordmark: Option<AssetRef>,
}

/// One wallpaper file with its metadata.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ResolvedWallpaper {
    /// The file.
    pub asset: AssetRef,
    /// Scheme it suits (`None`: both).
    pub scheme: Option<Mode>,
    /// Smallest viewport width it targets.
    pub min_width: u32,
    /// Pixel width.
    pub width: u32,
    /// Pixel height.
    pub height: u32,
    /// Dominant colour shown while loading.
    pub dominant: String,
}

/// A wallpaper set.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ResolvedWallpaperSet {
    /// Id.
    pub id: String,
    /// Picker label.
    pub label: String,
    /// Files.
    pub variants: Vec<ResolvedWallpaper>,
}

impl ResolvedWallpaperSet {
    /// Best file for a viewport: the widest `min_width` not above the viewport, among files that
    /// suit the scheme.
    #[must_use]
    pub fn pick(&self, dark: bool, viewport_width: u32) -> Option<&ResolvedWallpaper> {
        self.variants
            .iter()
            .filter(|v| {
                v.scheme
                    .is_none_or(|m| m == Mode::System || (m == Mode::Dark) == dark)
            })
            .filter(|v| v.min_width <= viewport_width)
            .max_by_key(|v| (v.min_width, v.scheme.is_some()))
    }
}

/// Icon set selection, passed through to the icon catalogue.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct IconSelection {
    /// Catalogue id (`phosphor`, ...).
    pub set: String,
    /// Glyph style.
    pub style: IconStyle,
    /// Weight.
    pub weight: u16,
}

/// Companion pet.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ResolvedPet {
    /// Pet id in its provider.
    pub id: String,
    /// `pack` or `openpets`.
    pub provider: String,
    /// Per-theme display name.
    pub name: String,
    /// Where it sits.
    pub anchor: String,
    /// Sprite edge in CSS px.
    pub size: u16,
    /// Animation states the pet provides.
    pub states: Vec<String>,
    /// Idle animation ids.
    pub idle: Vec<String>,
    /// Behaviour ids.
    pub behaviors: Vec<String>,
    /// Vector sprite.
    pub sprite: Option<AssetRef>,
}

/// Everything a surface needs besides colours.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ResolvedAssets {
    /// Display title.
    pub title: Titles,
    /// Logo variants.
    pub logo: ResolvedLogo,
    /// Wallpaper sets.
    pub wallpapers: Vec<ResolvedWallpaperSet>,
    /// Icon selection.
    pub icons: IconSelection,
    /// Pet, `None` when the bundle has none.
    pub pet: Option<ResolvedPet>,
    /// Sound events to files.
    pub sounds: Vec<(String, AssetRef)>,
    /// Cursor image.
    pub cursor: Option<AssetRef>,
    /// Header font family, when the bundle sets one.
    pub header_font: Option<String>,
}

impl ResolvedAssets {
    /// The wallpaper to show: `choice` by set id (first set when `None`), then the file for the
    /// scheme and viewport.
    #[must_use]
    pub fn wallpaper_for(
        &self,
        set_id: Option<&str>,
        dark: bool,
        viewport_width: u32,
    ) -> Option<&ResolvedWallpaper> {
        let set = match set_id {
            Some(id) => self.wallpapers.iter().find(|s| s.id == id)?,
            None => self.wallpapers.first()?,
        };
        set.pick(dark, viewport_width)
    }

    /// Every url that must be reachable, for preload lists and CDN uploads.
    #[must_use]
    pub fn all_urls(&self) -> Vec<&str> {
        let mut urls: Vec<&str> = Vec::new();
        let l = &self.logo;
        urls.extend(
            [&l.mark, &l.monochrome, &l.dark, &l.mask, &l.wordmark]
                .into_iter()
                .flatten()
                .map(|a| a.url.as_str()),
        );
        urls.extend(
            self.wallpapers
                .iter()
                .flat_map(|s| s.variants.iter().map(|v| v.asset.url.as_str())),
        );
        urls.extend(
            self.pet
                .iter()
                .filter_map(|p| p.sprite.as_ref())
                .map(|a| a.url.as_str()),
        );
        urls.extend(self.sounds.iter().map(|(_, a)| a.url.as_str()));
        urls.extend(self.cursor.iter().map(|a| a.url.as_str()));
        urls
    }

    /// Assets still marked as placeholders.
    #[must_use]
    pub fn placeholders(&self) -> Vec<&AssetRef> {
        let mut all: Vec<&AssetRef> = Vec::new();
        let l = &self.logo;
        all.extend(
            [&l.mark, &l.monochrome, &l.dark, &l.mask, &l.wordmark]
                .into_iter()
                .flatten(),
        );
        all.extend(
            self.wallpapers
                .iter()
                .flat_map(|s| s.variants.iter().map(|v| &v.asset)),
        );
        all.extend(self.pet.iter().filter_map(|p| p.sprite.as_ref()));
        all.extend(self.sounds.iter().map(|(_, a)| a));
        all.extend(self.cursor.iter());
        all.into_iter().filter(|a| a.placeholder).collect()
    }
}

fn extension(path: &str) -> &str {
    path.rsplit_once('.').map_or("bin", |(_, ext)| ext)
}

fn asset(bundle: &ThemeBundle, cdn: &CdnConfig, path: &str) -> Option<AssetRef> {
    let locked = bundle.lock.files.get(path)?;
    let meta = bundle.extras.asset.iter().find(|a| a.file == path);
    Some(AssetRef {
        path: path.to_owned(),
        url: format!("{}/{}.{}", cdn.base, &locked.sha256[..16], extension(path)),
        sha256: locked.sha256.clone(),
        bytes: locked.bytes,
        mime: locked.mime.clone(),
        license: meta.map_or_else(String::new, |m| m.license.clone()),
        source: meta.map_or_else(String::new, |m| m.source.clone()),
        attribution: meta.and_then(|m| m.attribution.clone()),
        placeholder: meta.is_some_and(|m| m.placeholder),
    })
}

fn wallpaper(
    bundle: &ThemeBundle,
    cdn: &CdnConfig,
    v: &WallpaperVariant,
) -> Option<ResolvedWallpaper> {
    Some(ResolvedWallpaper {
        asset: asset(bundle, cdn, &v.file)?,
        scheme: v.scheme,
        min_width: v.min_width,
        width: v.width,
        height: v.height,
        dominant: v.dominant.clone(),
    })
}

fn pet(bundle: &ThemeBundle, cdn: &CdnConfig) -> Option<ResolvedPet> {
    let spec = bundle.pack.pet.as_ref()?;
    let extras: Option<&PetExtras> = bundle.extras.pet.as_ref();
    Some(ResolvedPet {
        id: spec.id.clone(),
        provider: spec.provider.clone(),
        name: extras.map_or_else(|| spec.id.clone(), |e| e.name.clone()),
        anchor: spec.anchor.clone(),
        size: spec.size,
        states: spec.states.clone(),
        idle: extras.map(|e| e.idle.clone()).unwrap_or_default(),
        behaviors: extras.map(|e| e.behaviors.clone()).unwrap_or_default(),
        sprite: extras.and_then(|e| asset(bundle, cdn, &e.sprite)),
    })
}

/// Resolves every asset of a bundle against a CDN base.
#[must_use]
pub fn resolve_assets(
    bundle: &ThemeBundle,
    cdn: &CdnConfig,
    icon_style: IconStyle,
) -> ResolvedAssets {
    let e = &bundle.extras;
    let logo = bundle.pack.logo.as_ref();
    let get = |p: &Option<String>| p.as_deref().and_then(|p| asset(bundle, cdn, p));
    ResolvedAssets {
        title: bundle.pack.title.clone(),
        logo: ResolvedLogo {
            mark: logo.and_then(|l| asset(bundle, cdn, &l.mark)),
            monochrome: logo.and_then(|l| asset(bundle, cdn, &l.monochrome)),
            dark: get(&e.brand.logo_dark),
            mask: get(&e.brand.logo_mask),
            wordmark: logo
                .and_then(|l| l.wordmark.as_deref())
                .and_then(|p| asset(bundle, cdn, p)),
        },
        wallpapers: e
            .wallpaper
            .iter()
            .map(|set| ResolvedWallpaperSet {
                id: set.id.clone(),
                label: set.label.clone(),
                variants: set
                    .variant
                    .iter()
                    .filter_map(|v| wallpaper(bundle, cdn, v))
                    .collect(),
            })
            .collect(),
        icons: IconSelection {
            set: bundle.pack.icons.set.clone(),
            style: icon_style,
            weight: bundle.pack.icons.weight,
        },
        pet: pet(bundle, cdn),
        sounds: e
            .sound
            .iter()
            .flat_map(|s| s.events.iter())
            .filter_map(|(event, path)| Some((event.clone(), asset(bundle, cdn, path)?)))
            .collect(),
        cursor: get(&e.cursor),
        header_font: e.header_font.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn variant(min_width: u32, scheme: Option<Mode>) -> ResolvedWallpaper {
        ResolvedWallpaper {
            asset: AssetRef {
                path: format!("w{min_width}.avif"),
                url: String::new(),
                sha256: String::new(),
                bytes: 0,
                mime: String::new(),
                license: "CC0-1.0".into(),
                source: "original".into(),
                attribution: None,
                placeholder: false,
            },
            scheme,
            min_width,
            width: 1,
            height: 1,
            dominant: "#000000".into(),
        }
    }

    #[test]
    fn viewport_and_scheme_pick_the_widest_fitting_file() {
        let set = ResolvedWallpaperSet {
            id: "a".into(),
            label: "A".into(),
            variants: vec![
                variant(0, Some(Mode::Light)),
                variant(0, Some(Mode::Dark)),
                variant(1024, Some(Mode::Dark)),
                variant(1920, None),
            ],
        };
        assert_eq!(set.pick(true, 500).map(|v| v.min_width), Some(0));
        assert_eq!(set.pick(true, 1200).map(|v| v.min_width), Some(1024));
        assert_eq!(set.pick(true, 2560).map(|v| v.min_width), Some(1920));
        assert_eq!(set.pick(false, 1200).map(|v| v.min_width), Some(0));
    }
}
