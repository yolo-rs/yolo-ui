// SPDX-License-Identifier: Apache-2.0
//! winit backend: window attributes from stored state, state capture, theme
//! mapping and native menu attachment.

use winit::dpi::{LogicalPosition, LogicalSize};
use winit::window::{Fullscreen, Theme, Window, WindowAttributes};

use crate::theme::ColorScheme;
use crate::WindowState;

/// Applies a stored state to window attributes.
pub fn attributes_with_state(mut attributes: WindowAttributes, state: &WindowState) -> WindowAttributes {
    attributes = attributes.with_inner_size(LogicalSize::new(state.width as f64, state.height as f64));
    if let (Some(x), Some(y)) = (state.x, state.y) {
        attributes = attributes.with_position(LogicalPosition::new(x as f64, y as f64));
    }
    attributes = attributes.with_maximized(state.maximized);
    if state.fullscreen {
        attributes = attributes.with_fullscreen(Some(Fullscreen::Borderless(None)));
    }
    attributes
}

/// Reads the window geometry in logical pixels.
pub fn capture_state(window: &Window) -> WindowState {
    let scale = window.scale_factor();
    let size: LogicalSize<f64> = window.inner_size().to_logical(scale);
    let position: Option<LogicalPosition<f64>> = window.outer_position().ok().map(|p| p.to_logical(scale));
    WindowState {
        x: position.map(|p| p.x as f32),
        y: position.map(|p| p.y as f32),
        width: size.width as f32,
        height: size.height as f32,
        maximized: window.is_maximized(),
        fullscreen: window.fullscreen().is_some(),
    }
}

/// Maps a winit theme to the shell scheme.
pub fn color_scheme(theme: Option<Theme>) -> ColorScheme {
    match theme {
        Some(Theme::Dark) => ColorScheme::Dark,
        Some(Theme::Light) => ColorScheme::Light,
        None => ColorScheme::Unknown,
    }
}

/// Installs a native menu bar on a winit window (Windows/macOS).
#[cfg(feature = "menu")]
pub fn attach_menu(menu: &crate::native_menu::NativeMenu, window: &Window) -> crate::Result<()> {
    menu.attach(window)
}
