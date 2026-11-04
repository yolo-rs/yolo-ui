// SPDX-License-Identifier: MIT OR Apache-2.0
//! Unified native application shell lifecycle API for Yolo UI.
//!
//! This crate owns the cross-platform desktop lifecycle concerns shared across
//! all Yolo UI toolkits: application identity and about box ([`AppInfo`]) integrated
//! with [`YoloManifest`] (`yolo-neutral`, `yolo-m3`, `yolo-fluent`, `yolo-apple`,
//! `yolo-aphrody`), window state persistence ([`window_state`]), single instance
//! with argv forwarding ([`single_instance`]), deep link registration ([`deep_link`]),
//! a declarative menu model ([`menu`]) rendered natively with muda or by a toolkit
//! backend, the system tray, file dialogs, notifications, clipboard, light/dark and
//! accent detection ([`theme`]), and a signed update feed client ([`updater`]).
//!
//! Heavy toolkit backends live in [`backends`] behind the `eframe`, `winit`, `bevy`
//! and `tauri` features. Platform services are gated by the `menu`, `tray`,
//! `dialogs`, `notifications`, `clipboard` and `updater` features so `yolo-shell`
//! compiles fast and warning-free by default across all platforms.

pub mod app;
pub mod backends;
pub mod deep_link;
pub mod error;
pub mod menu;
pub mod single_instance;
pub mod theme;
pub mod window_state;

#[cfg(feature = "clipboard")]
pub mod clipboard;
#[cfg(feature = "dialogs")]
pub mod dialogs;
#[cfg(feature = "menu")]
pub mod native_menu;
#[cfg(feature = "notifications")]
pub mod notifications;
#[cfg(feature = "tray")]
pub mod tray;
#[cfg(feature = "updater")]
pub mod updater;

pub use app::AppInfo;
pub use error::{Error, Result};
pub use menu::{Menu, MenuEvent, MenuItem, Role};
pub use single_instance::{Instance, PrimaryInstance};
pub use theme::{ColorScheme, Rgb, SystemTheme};
pub use window_state::{WindowState, WindowStateStore};
pub use yolo_shared::{YoloManifest, YoloThemePreset};

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn app_info_roundtrips_with_all_five_yolo_manifests() {
        for preset in YoloThemePreset::ALL {
            let manifest = YoloManifest::new("io.yolo.workbench", "Yolo Workbench")
                .with_preset(preset);
            let info = AppInfo::from_manifest(&manifest)
                .with_description("Multi-engine native workbench")
                .with_website("https://github.com/yolo-rs/yolo-ui");

            assert_eq!(info.id, "io.yolo.workbench");
            assert_eq!(info.name, "Yolo Workbench");
            assert_eq!(info.theme_preset, preset);
            assert_eq!(info.qualifier_organization(), ("io", "yolo"));
            assert_eq!(info.slug(), "io.yolo.workbench");

            let roundtrip = info.to_manifest();
            assert_eq!(roundtrip.app_id, manifest.app_id);
            assert_eq!(roundtrip.title, manifest.title);
            assert_eq!(roundtrip.theme_preset, preset);

            let sys_theme = SystemTheme::for_preset(preset, true);
            assert_eq!(sys_theme.scheme, ColorScheme::Dark);
            assert_eq!(sys_theme.accent_or_default().argb(), preset.seed_argb());
        }
    }

    #[test]
    fn menu_activation_toggles_checks_and_radio_groups() {
        let mut menu = Menu::standard("Yolo Studio").submenu(
            "Theme",
            vec![
                MenuItem::check("grid", "Show Grid", false).with_accelerator("CmdOrCtrl+G"),
                MenuItem::radio("preset_neutral", "Neutral", "theme", true),
                MenuItem::radio("preset_m3", "Material 3", "theme", false),
                MenuItem::radio("preset_aphrody", "Aphrody Byron Love", "theme", false),
            ],
        );

        let ev = menu.activate("grid");
        assert_eq!(ev.checked, Some(true));
        assert_eq!(menu.is_checked("grid"), Some(true));

        let ev2 = menu.activate("preset_aphrody");
        assert_eq!(ev2.checked, Some(true));
        assert_eq!(menu.is_checked("preset_neutral"), Some(false));
        assert_eq!(menu.is_checked("preset_aphrody"), Some(true));

        let quit_ev = menu.activate(Role::Quit.id());
        assert_eq!(quit_ev.role, Some(Role::Quit));
    }

    #[test]
    fn window_state_sanitizes_and_merges_maximized_geometry() {
        let normal = WindowState {
            x: Some(120.0),
            y: Some(80.0),
            width: 1440.0,
            height: 900.0,
            maximized: false,
            fullscreen: false,
        };
        let maximized = WindowState {
            x: Some(0.0),
            y: Some(0.0),
            width: 2560.0,
            height: 1440.0,
            maximized: true,
            fullscreen: false,
        };
        let merged = normal.merge_observed(maximized);
        assert!(merged.maximized);
        assert_eq!(merged.width, 1440.0);
        assert_eq!(merged.height, 900.0);

        let corrupt = WindowState::with_size(10.0, f32::NAN).sanitized();
        assert_eq!(corrupt.width, window_state::MIN_SIZE);
        assert_eq!(corrupt.height, 800.0);
    }

    #[test]
    fn deep_link_validation_and_manifest_fragments() {
        assert!(deep_link::validate_scheme("yolo-ui").is_ok());
        assert!(deep_link::validate_scheme("123bad").is_err());

        let args = vec![
            String::from("yolo-showcase"),
            String::from("--flag"),
            String::from("yolo-ui://open/project?id=42"),
        ];
        assert_eq!(
            deep_link::find_url(&args, "yolo-ui"),
            Some("yolo-ui://open/project?id=42")
        );

        let app = AppInfo::new("org.yolo.showcase", "Yolo Showcase", "0.8.0");
        let desktop = deep_link::linux_desktop_entry(&app, Path::new("/usr/bin/yolo"), &["yolo"]);
        assert!(desktop.contains("MimeType=x-scheme-handler/yolo;"));
        let plist = deep_link::macos_info_plist_fragment(&app, &["yolo"]);
        assert!(plist.contains("<string>yolo</string>"));
    }
}
