// SPDX-License-Identifier: Apache-2.0
//! Toolkit backends: map window state, menus, about box and theme onto each
//! host. Each module is gated by the feature of the same name.

#[cfg(feature = "bevy")]
pub mod bevy;
#[cfg(feature = "eframe")]
pub mod eframe;
#[cfg(feature = "tauri")]
pub mod tauri;
#[cfg(feature = "winit")]
pub mod winit;
