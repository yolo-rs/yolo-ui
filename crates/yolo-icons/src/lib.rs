// SPDX-License-Identifier: Apache-2.0
//! # yolo-icons
//!
//! Icon and favicon pipeline for Yolo UI.
//!
//! - [`svg_import`] and [`chromium`] lower SVG and Chromium `.icon` sources to the normalized path IR in [`ir`].
//! - [`optimize`] runs SVGO-equivalent passes (degenerate segment removal, precision rounding, `currentColor`).
//! - [`bundle`] imports every set recorded in `catalog/assets.lock.toml` ([`lock`]), merges them into the
//!   "Aphrody Symbols" namespace by priority, writes zstd bundles and typed Rust / TypeScript modules.
//! - [`favicon`] renders preset-colored favicons, an ICO, PWA icons (including maskable) and a web manifest.
//! - [`search`] finds icons across sets.

pub mod bundle;
pub mod chromium;
pub mod color;
pub mod favicon;
pub mod ir;
pub mod lock;
pub mod optimize;
pub mod search;
pub mod svg_import;

/// The Aphrody mark on the Material Symbols 24 dp grid.
pub const APHRODY_MARK_24: &str = include_str!("../assets/aphrody/aphrody_mark_24.svg");
/// The Aphrody mark on the Material Symbols 48 dp grid (optical size 48).
pub const APHRODY_MARK_48: &str = include_str!("../assets/aphrody/aphrody_mark_48.svg");

/// Errors from every stage of the pipeline.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("svg: {0}")]
    Svg(String),
    #[error("chromium .icon: {0}")]
    ChromiumIcon(String),
    #[error("asset lock: {0}")]
    Lock(String),
    #[error("preset: {0}")]
    Preset(String),
    #[error("render: {0}")]
    Render(String),
    #[error("verify: {0}")]
    Verify(String),
    #[error("io: {0}")]
    Io(String),
}

/// Imports and optimizes the built-in Aphrody mark at the given optical size (24 or 48).
///
/// # Errors
/// Only if the embedded asset were corrupt.
pub fn aphrody_mark(optical_size: u32) -> Result<ir::Icon, Error> {
    let (source, grid) = if optical_size >= 48 {
        (APHRODY_MARK_48, 48.0)
    } else {
        (APHRODY_MARK_24, 24.0)
    };
    let (mut icon, _) = svg_import::import_svg("aphrody_mark", "aphrody", source.as_bytes(), grid)?;
    optimize::optimize(
        &mut icon,
        &optimize::OptimizeOptions {
            grid,
            ..optimize::OptimizeOptions::default()
        },
    );
    Ok(icon)
}
