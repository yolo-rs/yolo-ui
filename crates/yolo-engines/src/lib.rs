#![doc = include_str!("../README.md")]

#[cfg(feature = "cosmic")]
pub mod cosmic;
#[cfg(feature = "egui")]
pub mod egui;
#[cfg(feature = "gpui")]
pub mod gpui;
#[cfg(feature = "gtk")]
pub mod gtk;
#[cfg(feature = "iced")]
pub mod iced;
#[cfg(feature = "qt")]
pub mod qt;
#[cfg(feature = "tauri")]
pub mod tauri;
