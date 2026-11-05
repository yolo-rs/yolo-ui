//! Theme packs: one switch changes the whole interface, not only its colours.
//!
//! A pack (`themes/<id>/theme.toml`) declares the app title and logo, the icon set and its style,
//! the wallpapers with their licence, the companion pet, typography, the shell layout (sidebar,
//! top bar, dock, floating), density, shape, glass and motion. Colours are never hard-coded: they
//! come from a seed or from the active wallpaper through the HCT dynamic schemes of `yolo-tokens`,
//! and the user can override seed, variant, contrast and mode on top of the pack.
//!
//! - [`manifest`]: the pack format.
//! - [`resolve`]: pack plus user preferences into a [`ResolvedTheme`], with CSS and an asset map.
//! - [`wallpaper`]: a generated SVG wallpaper derived from the scheme, used when a pack ships none.
//! - [`omarchy`]: import an Omarchy theme directory as a pack.

pub mod manifest;
pub mod omarchy;
pub mod resolve;
pub mod wallpaper;

pub use manifest::{
    ColorSource, IconStyle, Mode, MotionProfile, PetSpec, ShellLayout, ThemePack, Wallpaper,
};
pub use resolve::{ResolvedTheme, UserPrefs, resolve};
