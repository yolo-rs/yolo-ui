// SPDX-License-Identifier: Apache-2.0
//! Theme packs: a preset switches identity as a whole (title, logo, icons, wallpapers,
//! companion, assets). Colors stay computed from an HCT seed and are user-tunable.
//!
//! Assets are logical keys (`<preset-id>/wallpaper/dark`); the host maps keys to files,
//! so a pack never embeds paths or binaries and works on native, WASM and Android alike.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use yolo_shared::YoloThemePreset;
use yolo_tokens::theme::ThemeRequest;

/// Logical asset key resolved by the host (file, embedded blob, URL).
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AssetKey(pub String);

impl AssetKey {
    fn of(preset: YoloThemePreset, slot: &str) -> Self {
        Self(format!("{}/{slot}", preset.id()))
    }
}

/// Logo variants for light, dark and single-color surfaces.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LogoSet {
    pub mark: AssetKey,
    pub wordmark: AssetKey,
    pub monochrome: AssetKey,
}

/// Wallpaper per resolved color mode, with an optional animated variant.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Wallpapers {
    pub light: AssetKey,
    pub dark: AssetKey,
    pub animated: Option<AssetKey>,
}

/// On-screen companion (pet). Sprite atlas plus named animation clips.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Companion {
    pub name: String,
    pub atlas: AssetKey,
    pub clips: Vec<String>,
}

/// Everything a theme changes besides computed colors.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ThemePack {
    pub preset: YoloThemePreset,
    /// Window or product title shown in chrome.
    pub title: String,
    pub logo: LogoSet,
    /// Icon set key (`phosphor`, `fluent`, `sf-symbols`, ...), resolved by the icon engine.
    pub icon_set: String,
    pub wallpapers: Wallpapers,
    pub companion: Option<Companion>,
    /// Extra named assets (sounds, cursors, boot splash, lock screen).
    pub assets: BTreeMap<String, AssetKey>,
}

impl ThemePack {
    /// Built-in pack for a preset.
    #[must_use]
    pub fn builtin(preset: YoloThemePreset) -> Self {
        let (title, icon_set) = match preset {
            YoloThemePreset::Neutral => ("YOLO", "phosphor"),
            YoloThemePreset::M3 => ("YOLO Material", "material-symbols"),
            YoloThemePreset::Fluent => ("YOLO Fluent", "fluent"),
            YoloThemePreset::Apple => ("YOLO Apple", "sf-symbols"),
            YoloThemePreset::Omarchy => ("YOLO Omarchy", "phosphor"),
            YoloThemePreset::Hyprland => ("YOLO Hyprland", "phosphor"),
            YoloThemePreset::Aphrody => ("Aphrody", "phosphor"),
            YoloThemePreset::Chromium => ("YOLO Chromium", "material-symbols"),
        };
        let companion = (preset == YoloThemePreset::Aphrody).then(|| Companion {
            name: "Byron Love".to_owned(),
            atlas: AssetKey::of(preset, "companion/atlas"),
            clips: ["idle", "walk", "sleep", "wave"].map(str::to_owned).to_vec(),
        });
        let mut assets = BTreeMap::new();
        for slot in ["boot-splash", "lock-screen", "cursor", "sound/startup"] {
            assets.insert(slot.to_owned(), AssetKey::of(preset, slot));
        }
        Self {
            preset,
            title: title.to_owned(),
            logo: LogoSet {
                mark: AssetKey::of(preset, "logo/mark"),
                wordmark: AssetKey::of(preset, "logo/wordmark"),
                monochrome: AssetKey::of(preset, "logo/mono"),
            },
            icon_set: icon_set.to_owned(),
            wallpapers: Wallpapers {
                light: AssetKey::of(preset, "wallpaper/light"),
                dark: AssetKey::of(preset, "wallpaper/dark"),
                animated: None,
            },
            companion,
            assets,
        }
    }

    /// Wallpaper for the resolved mode.
    #[must_use]
    pub const fn wallpaper(&self, dark: bool) -> &AssetKey {
        if dark { &self.wallpapers.dark } else { &self.wallpapers.light }
    }
}

/// User-adjustable color inputs. `None` keeps the preset value; nothing is hard-coded here.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ColorTuning {
    /// `#RRGGBB` source color.
    pub seed: Option<String>,
    /// HCT variant name (`tonal-spot`, `vibrant`, ...), same names as CSS generation.
    pub variant: Option<String>,
    /// MCU contrast level in -1..=1.
    pub contrast: Option<f64>,
}

/// A pack plus the user's color tuning: the complete, serializable theme state.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ThemeBundle {
    pub pack: ThemePack,
    pub tuning: ColorTuning,
}

impl ThemeBundle {
    /// Preset pack with untouched colors.
    #[must_use]
    pub fn new(preset: YoloThemePreset) -> Self {
        Self { pack: ThemePack::builtin(preset), tuning: ColorTuning::default() }
    }

    /// Switching preset swaps the whole pack and keeps the user's tuning.
    pub fn switch_preset(&mut self, preset: YoloThemePreset) {
        self.pack = ThemePack::builtin(preset);
    }

    /// Computes the token request: preset defaults overridden by validated user tuning.
    ///
    /// # Errors
    /// Invalid hex seed, unknown variant, or contrast outside -1..=1.
    pub fn request(&self) -> Result<ThemeRequest, String> {
        let base = yolo_tokens::presets::request_for_preset(self.pack.preset);
        let seed = match &self.tuning.seed {
            Some(hex) => ThemeRequest::parse(Some(hex), None, None)?.seed(),
            None => base.seed(),
        };
        let variant = match &self.tuning.variant {
            Some(name) => ThemeRequest::parse(None, Some(name), None)?.variant(),
            None => base.variant(),
        };
        ThemeRequest::new(seed, variant, self.tuning.contrast.unwrap_or(base.contrast_level()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_preset_has_a_distinct_pack() {
        let titles: std::collections::BTreeSet<_> =
            YoloThemePreset::ALL.iter().map(|p| ThemePack::builtin(*p).title).collect();
        assert_eq!(titles.len(), YoloThemePreset::ALL.len());
    }

    #[test]
    fn untouched_bundle_matches_preset_request() {
        let b = ThemeBundle::new(YoloThemePreset::Aphrody);
        let base = yolo_tokens::presets::request_for_preset(YoloThemePreset::Aphrody);
        assert_eq!(b.request().unwrap().seed(), base.seed());
    }

    #[test]
    fn tuning_survives_preset_switch_and_assets_follow() {
        let mut b = ThemeBundle::new(YoloThemePreset::Neutral);
        b.tuning.seed = Some("#FF0066".to_owned());
        b.switch_preset(YoloThemePreset::Aphrody);
        assert_eq!(b.request().unwrap().seed(), 0xFFFF_0066);
        assert!(b.pack.companion.is_some());
        assert!(b.pack.wallpaper(true).0.starts_with("yolo-aphrody/"));
    }

    #[test]
    fn invalid_tuning_is_rejected() {
        let mut b = ThemeBundle::new(YoloThemePreset::M3);
        b.tuning.contrast = Some(2.0);
        assert!(b.request().is_err());
    }
}
