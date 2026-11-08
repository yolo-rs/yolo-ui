//! Theme bundles for yolo-stack.
//!
//! A theme is a bundle, not a palette. The manifest format, the HCT colour computation and the
//! generated wallpaper live in `yolo-theme-pack` (yolo-ui). This crate adds what the stack needs
//! on top, with no I/O so that it compiles for `wasm32-unknown-unknown`:
//!
//! - [`bundle`]: `theme.toml` (pack), `bundle.toml` (extras) and `assets.lock.json` (content
//!   hashes) loaded from strings, validated with readable errors, plus the fallback bundle.
//! - [`prefs`]: [`UserThemePrefs`] and their cookie encoding.
//! - [`gate`]: contrast gates (4.5:1 text, 3:1 UI) with automatic tone adjustment.
//! - [`compute`]: [`compute()`] turns a bundle and prefs into a [`ResolvedBundle`].
//! - [`assets`]: logo, wallpaper for a viewport, icon set and pet, as CDN urls by content hash.
//! - [`emit`]: CSS custom properties, DTCG JSON and a typed struct for native consumers.

#![forbid(unsafe_code)]

pub mod assets;
pub mod bundle;
pub mod compute;
pub mod emit;
pub mod error;
pub mod gate;
pub mod prefs;

pub use assets::{AssetRef, CdnConfig, ResolvedAssets};
pub use bundle::{Registry, ThemeBundle};
pub use compute::{ResolvedBundle, compute};
pub use error::ThemeError;
pub use gate::{GateReport, PairResult};
pub use prefs::{SchemePref, UserThemePrefs, WallpaperChoice};
