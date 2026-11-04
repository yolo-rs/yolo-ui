//! Unified multi-engine showcase binary (`apps/yolo-showcase`) verifying that
//! `yolo-shared`, `yolo-ui` (Dioxus 0.8), `yolo-gpui` (Zed + Longbridge GPUI),
//! `yolo-iced` (Iced Elm MVU + Audio DSP + Comet), and `yolo-tauri` (Tauri v3 Alpha)
//! share a single Material 3 Dark-Glass token foundation and compile together cleanly.

use yolo_ui::gpui::{DockArea, DockPosition, GpuiTheme, I18nContext, PanelDescriptor, TabItem};
use yolo_ui::iced::prelude::{FaderConfig, FreqRange, IcedTheme};
use yolo_ui::shared::{css_variables, APHRODY_DARK_THEME};
use yolo_ui::tauri::{CapabilityManifest, TauriV3Config, WebviewEngine};

fn main() {
    let css = css_variables(&APHRODY_DARK_THEME);
    let gpui_theme = GpuiTheme::aphrody_dark();
    let iced_theme = IcedTheme::aphrody_dark();
    let tauri_cfg = TauriV3Config::default();
    let cap = CapabilityManifest::new("main-shell", "Aphrody Default Capability");

    let mut dock = DockArea::new("aphrody-workspace");
    dock.add_panel(
        PanelDescriptor::new("explorer", "Fleet Explorer", DockPosition::Left)
            .with_tab(TabItem::new("fleet", "Submodules (70)")),
    );

    let mut i18n = I18nContext::new("fr", "en");
    i18n.catalog.insert("en", "app.title", "Yolo UI Unified Monorepo");
    i18n.catalog.insert("fr", "app.title", "Monorepo Unifié Yolo UI");

    let freq = FreqRange::audible_spectrum();
    let fader = FaderConfig::channel_strip("Master Bus", 0.85);

    println!("=== {} ===", i18n.t("app.title"));
    println!("Theme Token      : {} ({} bytes CSS)", APHRODY_DARK_THEME.name, css.len());
    println!(
        "GPUI Accent      : 0x{:08x} | Dock Panels: {}",
        gpui_theme.accent_primary.to_u32_rgba(),
        dock.panels.len()
    );
    println!(
        "Iced Primary     : {} | Audio 1kHz Norm: {:.3} | Fader: {:.1} dB",
        iced_theme.palette.primary,
        freq.map_to_normal(1000.0).value(),
        fader.current_db()
    );
    println!(
        "Tauri v3 Engine  : {:?} | Capability: {} ({} permissions)",
        WebviewEngine::default(),
        cap.identifier,
        cap.permissions.len()
    );
    println!("Tauri Identifier : {}", tauri_cfg.identifier);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unified_four_engine_token_parity() {
        let gpui = GpuiTheme::aphrody_dark();
        let iced = IcedTheme::aphrody_dark();
        assert_eq!(gpui.accent_primary, iced.palette.primary);
        assert_eq!(gpui.bg_primary, iced.palette.background);
        assert_eq!(gpui.accent_primary.to_css_string(), APHRODY_DARK_THEME.accent_primary);
    }
}
