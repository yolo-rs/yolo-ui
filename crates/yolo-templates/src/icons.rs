// SPDX-License-Identifier: Apache-2.0
//! The handful of line icons the templates need, drawn for this crate on a 24 px grid with a
//! 1.75 px stroke in `currentColor`. Original geometry (see `ASSETS.md`); the product icon
//! library lives in `yolo-icons` and can replace these through [`IconName`].

use dioxus::prelude::*;

/// Icons used by the templates.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum IconName {
    /// Magnifier.
    Search,
    /// Microphone.
    Mic,
    /// Downward chevron (disclosure).
    ChevronDown,
    /// Arrow to the right (onward link).
    ArrowRight,
    /// Three bars (navigation).
    Menu,
    /// House.
    Home,
    /// Compass (explore).
    Compass,
    /// Shelf of books (library).
    Library,
    /// Clock (history).
    Clock,
    /// Four squares (collections).
    Grid,
    /// Arrow into a tray (download).
    Download,
}

impl IconName {
    /// SVG path data on the 24 x 24 grid.
    #[must_use]
    pub const fn path(self) -> &'static str {
        match self {
            Self::Search => "M17 10.5a6.5 6.5 0 1 1-13 0a6.5 6.5 0 1 1 13 0Z M15.3 15.3 20 20",
            Self::Mic => "M12 3.5a3 3 0 0 0-3 3v4.5a3 3 0 0 0 6 0V6.5a3 3 0 0 0-3-3Z M5.75 11a6.25 6.25 0 0 0 12.5 0 M12 17.25V20.5 M8.75 20.5h6.5",
            Self::ChevronDown => "M6.5 9.5 12 15l5.5-5.5",
            Self::ArrowRight => "M5 12h13.5 M13 6.5 18.5 12 13 17.5",
            Self::Menu => "M4 7h16 M4 12h16 M4 17h16",
            Self::Home => "M4 11 12 4.5l8 6.5 M6.25 9.5V19.5h11.5V9.5 M10 19.5v-5h4v5",
            Self::Compass => "M20.5 12a8.5 8.5 0 1 1-17 0a8.5 8.5 0 1 1 17 0Z M15.25 8.75l-1.9 4.6-4.6 1.9 1.9-4.6Z",
            Self::Library => "M4.5 5h3.75v14.5H4.5Z M10.25 5H14v14.5h-3.75Z M16 6.1l3.6-.95 3.55 13.6-3.6.95Z",
            Self::Clock => "M20.5 12a8.5 8.5 0 1 1-17 0a8.5 8.5 0 1 1 17 0Z M12 7.25V12l3.25 2",
            Self::Grid => "M4.5 4.5h6v6h-6Z M13.5 4.5h6v6h-6Z M4.5 13.5h6v6h-6Z M13.5 13.5h6v6h-6Z",
            Self::Download => "M12 4v10.5 M7.5 10.5 12 15l4.5-4.5 M5 19.5h14",
        }
    }
}

/// A decorative icon. It is hidden from assistive technology: the control or text beside it
/// carries the accessible name.
#[component]
pub fn Icon(name: IconName) -> Element {
    rsx! {
        svg {
            class: "yt-icon",
            view_box: "0 0 24 24",
            width: "24",
            height: "24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "1.75",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            "aria-hidden": "true",
            "focusable": "false",
            path { d: name.path() }
        }
    }
}
