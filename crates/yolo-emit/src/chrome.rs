// SPDX-License-Identifier: Apache-2.0
//! Chrome theme extensions (`manifest.json`, manifest version 3).
//!
//! Chrome themes carry one appearance, so each theme source yields a dark and a light theme
//! directory, each loadable with `--load-extension=<dir>` or "Load unpacked". The accepted keys
//! are those of Chromium's `chrome/browser/themes/browser_theme_pack.cc` tables
//! ([`COLOR_KEYS`], [`TINT_KEYS`], [`PROPERTY_KEYS`]); any other key is ignored by Chrome.
//! Documentation: <https://developer.chrome.com/docs/extensions/develop/ui/themes>.

use crate::{EmittedFile, ThemeSource, quoted, release_version, rgb};
use std::fmt::Write;
use yolo_tokens::role::Role;
use yolo_tokens::scheme::DynamicScheme;

/// Chromium revision the key tables were read from.
pub const CHROMIUM_REVISION: &str = "4c7244a405d326e284710231ff609168dceb9044";

/// `kOverwritableColorTable` of `browser_theme_pack.cc`.
pub const COLOR_KEYS: [&str; 24] = [
    "background_tab",
    "background_tab_inactive",
    "background_tab_incognito",
    "background_tab_incognito_inactive",
    "bookmark_text",
    "button_background",
    "frame",
    "frame_inactive",
    "frame_incognito",
    "frame_incognito_inactive",
    "ntp_background",
    "ntp_header",
    "ntp_link",
    "ntp_text",
    "omnibox_background",
    "omnibox_text",
    "tab_background_text",
    "tab_background_text_inactive",
    "tab_background_text_incognito",
    "tab_background_text_incognito_inactive",
    "tab_text",
    "toolbar",
    "toolbar_button_icon",
    "toolbar_text",
];

/// `kTintTable` of `browser_theme_pack.cc`.
pub const TINT_KEYS: [&str; 6] = [
    "background_tab",
    "buttons",
    "frame",
    "frame_inactive",
    "frame_incognito",
    "frame_incognito_inactive",
];

/// `kDisplayProperties` of `browser_theme_pack.cc`.
pub const PROPERTY_KEYS: [&str; 3] = [
    "ntp_background_alignment",
    "ntp_background_repeat",
    "ntp_logo_alternate",
];

/// Which scheme a Chrome color is read from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    /// The theme's own appearance.
    Own,
    /// Always the dark scheme (incognito surfaces stay dark in both appearances).
    Incognito,
}

/// The role behind every Chrome color in a dark (`.1`) and light (`.2`) theme.
fn role_for(key: &str, dark: bool) -> (Mode, Role) {
    use Role as R;
    let own = |d: Role, l: Role| (Mode::Own, if dark { d } else { l });
    match key {
        "frame" | "button_background" => own(R::SurfaceContainerLowest, R::SurfaceContainerHighest),
        "frame_inactive" | "background_tab_inactive" => own(R::Surface, R::SurfaceContainerHigh),
        "background_tab" => own(R::SurfaceContainerLow, R::SurfaceContainerHigh),
        "frame_incognito" => (Mode::Incognito, R::SurfaceContainerLowest),
        "frame_incognito_inactive" | "background_tab_incognito_inactive" => {
            (Mode::Incognito, R::Surface)
        }
        "background_tab_incognito" => (Mode::Incognito, R::SurfaceContainerLow),
        "tab_background_text_incognito" | "tab_background_text_incognito_inactive" => {
            (Mode::Incognito, R::OnSurfaceVariant)
        }
        "toolbar" => own(R::SurfaceContainerHigh, R::SurfaceContainerLowest),
        "omnibox_background" => own(R::SurfaceContainerLowest, R::SurfaceContainerHigh),
        "ntp_background" => (Mode::Own, R::Surface),
        "ntp_link" | "ntp_header" => (Mode::Own, R::Primary),
        "tab_background_text" | "tab_background_text_inactive" | "toolbar_button_icon" => {
            (Mode::Own, R::OnSurfaceVariant)
        }
        // tab_text, toolbar_text, bookmark_text, ntp_text, omnibox_text
        _ => (Mode::Own, R::OnSurface),
    }
}

/// `0xAARRGGBB` of one Chrome color key.
#[must_use]
pub fn color(src: &ThemeSource, dark: bool, key: &str) -> u32 {
    let (mode, role) = role_for(key, dark);
    let scheme: &DynamicScheme = match mode {
        Mode::Own => src.scheme(dark),
        Mode::Incognito => &src.dark,
    };
    scheme.argb(role)
}

/// Text/background key pairs Chrome draws together, checked for WCAG AA.
pub const TEXT_PAIRS: [(&str, &str); 8] = [
    ("tab_text", "toolbar"),
    ("toolbar_text", "toolbar"),
    ("bookmark_text", "toolbar"),
    ("tab_background_text", "background_tab"),
    ("tab_background_text_inactive", "background_tab_inactive"),
    ("tab_background_text_incognito", "background_tab_incognito"),
    ("ntp_text", "ntp_background"),
    ("omnibox_text", "omnibox_background"),
];

/// Directory name of one appearance (`yolo-aphrody-dark`).
#[must_use]
pub fn dir_name(src: &ThemeSource, dark: bool) -> String {
    format!("{}-{}", src.slug, if dark { "dark" } else { "light" })
}

/// The `manifest.json` of one appearance.
#[must_use]
pub fn manifest(src: &ThemeSource, dark: bool) -> String {
    let mode = if dark { "Dark" } else { "Light" };
    let mut out = String::from("{\n  \"manifest_version\": 3,\n");
    let _ = writeln!(
        out,
        "  \"name\": {},",
        quoted(&format!("{} {mode}", src.name))
    );
    let _ = writeln!(out, "  \"version\": {},", quoted(release_version()));
    let _ = writeln!(
        out,
        "  \"description\": {},",
        quoted(&format!(
            "{} {} browser theme generated by yolo-emit from yolo-tokens HCT roles.",
            src.name,
            mode.to_ascii_lowercase()
        ))
    );
    out.push_str("  \"theme\": {\n    \"colors\": {\n");
    for (i, key) in COLOR_KEYS.iter().enumerate() {
        let (r, g, b) = rgb(color(src, dark, key));
        let comma = if i + 1 == COLOR_KEYS.len() { "" } else { "," };
        let _ = writeln!(out, "      \"{key}\": [{r}, {g}, {b}]{comma}");
    }
    out.push_str("    },\n    \"properties\": {\n");
    let _ = writeln!(out, "      \"ntp_logo_alternate\": {}", u8::from(dark));
    out.push_str("    }\n  }\n}\n");
    out
}

/// Dark then light theme directories.
#[must_use]
pub fn files(src: &ThemeSource) -> Vec<EmittedFile> {
    [true, false]
        .into_iter()
        .map(|dark| {
            EmittedFile::new(
                format!("{}/manifest.json", dir_name(src, dark)),
                manifest(src, dark),
            )
        })
        .collect()
}
