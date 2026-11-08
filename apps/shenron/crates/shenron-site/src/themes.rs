//! Theme registry embedded in the binary and the wasm bundle.
//!
//! The bundles live in `apps/shenron/themes/<id>/`; `build.rs` validates them and emits the
//! stylesheet, this module makes the same data available at runtime so the settings panel can
//! recompute a palette in the browser without a round trip.

use std::sync::OnceLock;

use yolo_stack_theme::{
    assets::CdnConfig,
    bundle::{Registry, ThemeBundle},
    compute::{ResolvedBundle, compute_with},
    prefs::{UserThemePrefs, WallpaperChoice},
};

/// Bundle used when the visitor has no saved choice.
pub const DEFAULT_ID: &str = "shenron";

macro_rules! bundle {
    ($id:literal) => {
        ThemeBundle::from_strs(
            include_str!(concat!("../../../themes/", $id, "/theme.toml")),
            include_str!(concat!("../../../themes/", $id, "/bundle.toml")),
            include_str!(concat!("../../../themes/", $id, "/assets.lock.json")),
        )
        .unwrap_or_else(|e| panic!("{e}"))
    };
}

/// All bundles, default first.
pub fn registry() -> &'static Registry {
    static REGISTRY: OnceLock<Registry> = OnceLock::new();
    REGISTRY.get_or_init(|| {
        Registry::new(
            vec![
                bundle!("shenron"),
                bundle!("kame"),
                bundle!("capsule"),
                bundle!("namek"),
            ],
            DEFAULT_ID,
        )
        .unwrap_or_else(|e| panic!("{e}"))
    })
}

/// CDN base for assets (`SHENRON_THEME_CDN` at build time, `/theme-assets` otherwise).
pub fn cdn() -> CdnConfig {
    CdnConfig {
        base: option_env!("SHENRON_THEME_CDN")
            .unwrap_or("/theme-assets")
            .to_owned(),
    }
}

/// Computes the theme for some preferences.
pub fn resolved(prefs: &UserThemePrefs) -> ResolvedBundle {
    compute_with(
        registry().get_or_default(prefs.theme.as_deref()),
        prefs,
        &cdn(),
    )
}

/// True when the preferences change the stylesheet beyond the shipped bundle CSS.
pub fn is_customized(prefs: &UserThemePrefs) -> bool {
    prefs.hue.is_some()
        || prefs.chroma.is_some()
        || prefs.contrast.is_some()
        || prefs.variant.is_some()
        || !prefs.overrides.is_empty()
        || prefs.wallpaper != WallpaperChoice::Auto
}

/// CSS for customised preferences. The selectors gain `[data-user]` so they outrank the shipped
/// stylesheet whatever the order of the `<style>` and `<link>` elements.
pub fn user_css(prefs: &UserThemePrefs) -> String {
    yolo_stack_theme::emit::css(&resolved(prefs))
        .replace(":root[data-theme=", ":root[data-user][data-theme=")
}
