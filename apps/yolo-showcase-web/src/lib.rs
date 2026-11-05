//! Browser build of the Yolo UI showcase.
//!
//! [`gallery`] renders every component in every preset, either as static HTML (used by
//! `yolo-showcase --emit-gallery`) or inside the app. [`theme_state`] turns a theme choice into
//! one applied theme. [`app`] (feature `web`) is the Dioxus 0.8 app with the router.
//!
//! This crate depends only on `yolo-shared`, `yolo-tokens`, `yolo-theme-pack`, `yolo-contracts`
//! and Dioxus, so it builds for `wasm32-unknown-unknown`.

#[cfg(feature = "web")]
pub mod app;
pub mod gallery;
pub mod theme_state;
