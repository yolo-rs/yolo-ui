// SPDX-License-Identifier: Apache-2.0
//! Neutral, customizable brand identity and preset descriptors for every Craft host.
//!
//! Defaults to `"Yolo Craft"` with [`YoloThemePreset::Neutral`], while providing built-in
//! presets for `Neutral`, `M3`, `Fluent`, `Apple`, and `Aphrody` (Byron Love).

pub use yolo_shared::{
    ByronLoveColorTeinte, ByronLovePetSpec, YoloManifest, YoloStyle, YoloThemePreset,
    YoloThemeTokens, BYRON_LOVE_PALETTE, BYRON_LOVE_PET_SPEC,
};

/// Canonical default product family name.
pub const PRODUCT: &str = "Yolo Craft";

/// Neutral default seed color (`#6366F1`).
pub const BRAND_SEED_ARGB: u32 = YoloThemePreset::Neutral.seed_argb();

/// Aphrody / Byron Love Royal Azur seed color (`#17335C`).
pub const APHRODY_SEED_ARGB: u32 = YoloThemePreset::Aphrody.seed_argb();

/// Minimal neutral SVG mark for Yolo Craft hosts.
pub const LOGO_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64"><rect width="64" height="64" rx="14" fill="#6366F1"/><path d="M18 20L32 36L46 20M32 36V48" stroke="#FFFFFF" stroke-width="5" stroke-linecap="round" stroke-linejoin="round"/></svg>"##;

/// Customizable brand identity for any Craft application or workbench.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CraftBrand {
    /// Product suite name (e.g. `"Yolo Craft"`).
    pub product: String,
    /// Organization or vendor name (e.g. `"Yolo UI"`).
    pub vendor: String,
    /// Active theme preset (`Neutral`, `M3`, `Fluent`, `Apple`, or `Aphrody`).
    pub preset: YoloThemePreset,
    /// Primary brand seed color as `0xAARRGGBB`.
    pub seed_argb: u32,
    /// Inline SVG logo mark.
    pub logo_svg: String,
}

impl Default for CraftBrand {
    fn default() -> Self {
        Self::neutral()
    }
}

impl CraftBrand {
    /// Creates a new customizable [`CraftBrand`] for a given [`YoloThemePreset`].
    #[must_use]
    pub fn for_preset(preset: YoloThemePreset) -> Self {
        let (product, vendor) = match preset {
            YoloThemePreset::Neutral => ("Yolo Craft", "Yolo UI"),
            YoloThemePreset::M3 => ("Yolo Craft M3", "Yolo UI"),
            YoloThemePreset::Fluent => ("Yolo Craft Fluent", "Yolo UI"),
            YoloThemePreset::Apple => ("Yolo Craft HIG", "Yolo UI"),
            YoloThemePreset::Omarchy => ("Yolo Craft Omarchy", "Yolo UI"),
            YoloThemePreset::Hyprland => ("Yolo Craft Hyprland", "Yolo UI"),
            YoloThemePreset::Aphrody => ("Aphrody Craft", "Byron Love Studio"),
            YoloThemePreset::Chromium => ("Yolo Craft Chromium", "Chromium Project"),
        };
        Self {
            product: product.to_owned(),
            vendor: vendor.to_owned(),
            preset,
            seed_argb: preset.seed_argb(),
            logo_svg: LOGO_SVG.to_owned(),
        }
    }

    /// Neutral default Craft brand (`"Yolo Craft"`, `#6366F1`).
    #[must_use]
    pub fn neutral() -> Self {
        Self::for_preset(YoloThemePreset::Neutral)
    }

    /// Material Design 3 Craft brand (`#6750A4`).
    #[must_use]
    pub fn m3() -> Self {
        Self::for_preset(YoloThemePreset::M3)
    }

    /// Microsoft Fluent 2 Craft brand (`#0078D4`).
    #[must_use]
    pub fn fluent() -> Self {
        Self::for_preset(YoloThemePreset::Fluent)
    }

    /// Apple HIG / Cupertino Craft brand (`#007AFF`).
    #[must_use]
    pub fn apple() -> Self {
        Self::for_preset(YoloThemePreset::Apple)
    }

    /// Omarchy Linux TUI / Tokyo Night Craft brand (`#7AA2F7`, sharp `0px`/`2px` radii).
    #[must_use]
    pub fn omarchy() -> Self {
        Self::for_preset(YoloThemePreset::Omarchy)
    }

    /// Hyprland dynamic Wayland compositor Craft brand (`#33CCFF` -> `#00FF99`, `10px` radii).
    #[must_use]
    pub fn hyprland() -> Self {
        Self::for_preset(YoloThemePreset::Hyprland)
    }

    /// Aphrody / Byron Love Craft brand (`#17335C` Royal Azur + `#FFD700` Or Céleste).
    #[must_use]
    pub fn aphrody() -> Self {
        Self::for_preset(YoloThemePreset::Aphrody)
    }

    /// Formats a window title for a Craft capability (e.g. `"Yolo Craft — CAD"`).
    #[must_use]
    pub fn title_for(&self, capability: &str) -> String {
        format!("{} \u{2014} {capability}", self.product)
    }

    /// Converts this brand into a [`YoloManifest`] for the given workbench ID.
    #[must_use]
    pub fn to_manifest(&self, app_id: &str, capability: &str) -> YoloManifest {
        YoloManifest::new(app_id, self.title_for(capability)).with_preset(self.preset)
    }
}

/// Window title for a Craft capability using the default [`PRODUCT`] name.
#[must_use]
pub fn app_title(capability: &str) -> String {
    format!("{PRODUCT} \u{2014} {capability}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_brand_is_neutral_yolo_craft() {
        let brand = CraftBrand::default();
        assert_eq!(brand.product, "Yolo Craft");
        assert_eq!(brand.preset, YoloThemePreset::Neutral);
        assert_eq!(brand.seed_argb, 0xFF63_66F1);
        assert_eq!(app_title("CAD"), "Yolo Craft \u{2014} CAD");
    }

    #[test]
    fn all_five_presets_have_valid_brand_builders() {
        for preset in YoloThemePreset::ALL {
            let brand = CraftBrand::for_preset(preset);
            assert_eq!(brand.preset, preset);
            assert_eq!(brand.seed_argb, preset.seed_argb());
            assert!(brand.title_for("Vector").contains("Vector"));
        }
        assert_eq!(CraftBrand::aphrody().seed_argb, 0xFF17_335C);
    }
}
