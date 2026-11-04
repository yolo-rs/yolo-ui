//! Unified multi-engine & 7-theme showcase binary (`apps/yolo-showcase`) verifying that
//! `yolo-shared` (Neutral default + 7 presets: Neutral, M3, Fluent, Apple, Omarchy + 23 subthemes,
//! Hyprland, and Aphrody Byron Love), `yolo-ui` (Dioxus 0.8), `yolo-gpui` (Zed + Longbridge GPUI),
//! `yolo-iced` (Iced Elm MVU + Audio DSP + Comet), `yolo-cosmic` (System76 libcosmic),
//! `yolo-egui` (Egui 0.36), `yolo-gtk` (GTK 4.16 + libadwaita 1.6 + Relm4),
//! `yolo-qt` (Qt 6 + cxx-qt + QML), `yolo-tauri` (Tauri v3 Alpha),
//! `yolo-craft` (13 Creative Workbenches), `yolo-storyteller` (10 Storyteller Modules),
//! `yolo-migrator` (Shadcn/Tailwind/Radix/BaseUI TSX -> 6 Rust targets), and
//! `yolo-react` (Full-Bun + Rust React 19 RSC & React Native Taffy Layout) compile and work together!

pub mod benchmarks;
pub mod site_hub;

use std::fs;
use std::path::Path;
use site_hub::{
    AWESOME_HUB_CATEGORIES, MASTER_MIGRATION_ENTRIES, MigrationPillar, YOLO_STARTER_TEMPLATES,
    render_yolo_aphrody_com_html,
};
use yolo_ui::craft::workbenches::all_workbenches;
use yolo_ui::emit::{Target as EmitTarget, emit_preset};
use yolo_ui::gpui::{DockArea, DockPosition, GpuiTheme, I18nContext, PanelDescriptor, TabItem};
use yolo_ui::iced::prelude::{FaderConfig, FreqRange, IcedTheme};
use yolo_ui::migrator::{MigrationTarget, migrate_tsx};
use yolo_ui::react::theme_bridge::build_react_native_theme;
use yolo_ui::shared::{
    BYRON_LOVE_PALETTE, BYRON_LOVE_PET_SPEC, HYPRLAND_DECORATION_SPEC, OMARCHY_THEMES,
    OmarchySubTheme, YoloManifest, YoloStyle, YoloThemePreset, css_preset_variables,
};
use yolo_ui::storyteller::StorytellerProject;
use yolo_ui::tauri::{CapabilityManifest, TauriV3Config, WebviewEngine};

fn main() {
    let manifest = YoloManifest::default();
    let gpui_theme = GpuiTheme::aphrody_dark();
    let iced_theme = IcedTheme::aphrody_dark();
    let tauri_cfg = TauriV3Config::default();
    let cap = CapabilityManifest::new("main-shell", "Yolo Default Capability");

    let mut dock = DockArea::new("yolo-workspace");
    dock.add_panel(
        PanelDescriptor::new("explorer", "Fleet Explorer", DockPosition::Left)
            .with_tab(TabItem::new("fleet", "Submodules (124)")),
    );

    let mut i18n = I18nContext::new("fr", "en");
    i18n.catalog.insert("en", "app.title", "Yolo UI Unified Multi-Engine & 7-Theme Monorepo");
    i18n.catalog.insert("fr", "app.title", "Monorepo Unifié Multi-Moteur & 7 Thèmes Yolo UI");

    let freq = FreqRange::audible_spectrum();
    let fader = FaderConfig::channel_strip("Master Bus", 0.85);
    let workbenches = all_workbenches();
    let storyteller = StorytellerProject::new("yolo-demo", "Yolo Storyteller Suite", YoloThemePreset::Neutral);

    let sample_shadcn_tsx = r#"
        export function HeroCard() {
            return (
                <Card className="flex flex-col gap-4 p-6 rounded-xl bg-card">
                    <CardHeader>
                        <CardTitle>Yolo UI Universal</CardTitle>
                    </CardHeader>
                    <CardContent>
                        <Button className="px-4 py-2 rounded-lg bg-primary">Launch</Button>
                    </CardContent>
                </Card>
            );
        }
    "#;
    let migrated_dioxus =
        migrate_tsx(sample_shadcn_tsx, MigrationTarget::Dioxus, YoloThemePreset::Neutral)
            .expect("Shadcn TSX should migrate to Dioxus RSX");
    let rn_theme = build_react_native_theme(
        YoloThemePreset::Omarchy,
        true,
        Some(OmarchySubTheme::OsakaJade),
    );

    println!("=== {} ===", i18n.t("app.title"));
    println!(
        "Default Manifest : {} ({}) | Neutral Default Preset: {}",
        manifest.title,
        manifest.app_id,
        manifest.theme_preset.as_str()
    );
    println!("7 Theme Presets  :");
    for preset in YoloThemePreset::ALL {
        let style = YoloStyle::for_preset(preset);
        let css = css_preset_variables(preset, true);
        let emitted_gtk = emit_preset(preset, EmitTarget::Gtk);
        println!(
            "  - {:<14} | seed=0x{:08X} | radius={}px | blur={}px | css={}B | gtk={}B",
            preset.id(),
            preset.seed_argb(),
            style.radius_md_px,
            style.backdrop_blur_px,
            css.len(),
            emitted_gtk.len()
        );
    }
    println!(
        "Omarchy Themes   : {} official palettes (default: {})",
        OMARCHY_THEMES.len(),
        OMARCHY_THEMES[0].display_name
    );
    println!(
        "Hyprland Spec    : active_border=#33CCFF->#00FF99 ({}deg), rounding={}px, kawase_blur={}x{}",
        HYPRLAND_DECORATION_SPEC.active_border_angle_deg,
        HYPRLAND_DECORATION_SPEC.rounding_px,
        HYPRLAND_DECORATION_SPEC.kawase_blur_size,
        HYPRLAND_DECORATION_SPEC.kawase_blur_passes
    );
    println!(
        "Byron Love Pet   : {} ({}x{} grid, {}x{} cells, {} measured teintes)",
        BYRON_LOVE_PET_SPEC.character_name,
        BYRON_LOVE_PET_SPEC.grid_cols,
        BYRON_LOVE_PET_SPEC.grid_rows,
        BYRON_LOVE_PET_SPEC.cell_width_px,
        BYRON_LOVE_PET_SPEC.cell_height_px,
        BYRON_LOVE_PALETTE.len()
    );
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
        "Craft & Story    : {} Craft workbenches | {} Storyteller modules",
        workbenches.len(),
        storyteller.modules.len()
    );
    println!(
        "Shadcn Migrator  : TSX -> Dioxus RSX ({}B) | React Native Theme ({})",
        migrated_dioxus.code.len(),
        rn_theme.preset_id
    );
    println!(
        "Tauri v3 Engine  : {:?} | Capability: {} ({} permissions) | ID: {}",
        WebviewEngine::default(),
        cap.identifier,
        cap.permissions.len(),
        tauri_cfg.identifier
    );

    let site_html = render_yolo_aphrody_com_html();
    println!(
        "yolo.aphrody.com : {}B HTML | {} Migration Pillars ({} targets) | {} Templates | {} Awesome Hubs",
        site_html.len(),
        MigrationPillar::ALL.len(),
        MASTER_MIGRATION_ENTRIES.len(),
        YOLO_STARTER_TEMPLATES.len(),
        AWESOME_HUB_CATEGORIES.len()
    );

    let args: Vec<String> = std::env::args().collect();
    if let Some(idx) = args.iter().position(|a| a == "--emit-site") {
        let out_dir = args
            .get(idx + 1)
            .map(String::as_str)
            .unwrap_or("dist/yolo-aphrody-com");
        let dir_path = Path::new(out_dir);
        fs::create_dir_all(dir_path).expect("failed to create yolo.aphrody.com output directory");
        let index_path = dir_path.join("index.html");
        fs::write(&index_path, &site_html).expect("failed to write yolo.aphrody.com index.html");
        println!("Emitted Site     : {}", index_path.display());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use yolo_ui::shared::APHRODY_DARK_THEME;

    #[test]
    fn test_unified_multi_engine_and_seven_theme_parity() {
        assert_eq!(YoloManifest::default().theme_preset, YoloThemePreset::Neutral);
        assert_eq!(YoloThemePreset::ALL.len(), 8);
        assert_eq!(OMARCHY_THEMES.len(), 23);
        assert_eq!(BYRON_LOVE_PALETTE.len(), 10);
        assert_eq!(all_workbenches().len(), 13);

        let gpui = GpuiTheme::aphrody_dark();
        let iced = IcedTheme::aphrody_dark();
        assert_eq!(gpui.accent_primary, iced.palette.primary);
        assert_eq!(gpui.bg_primary, iced.palette.background);
        assert_eq!(gpui.accent_primary.to_css_string(), APHRODY_DARK_THEME.accent_primary);
    }

    #[test]
    fn test_yolo_aphrody_com_site_and_six_pillar_migration_plan() {
        assert_eq!(MigrationPillar::ALL.len(), 7);
        assert!(MASTER_MIGRATION_ENTRIES.len() >= 20);
        assert_eq!(YOLO_STARTER_TEMPLATES.len(), 8);
        assert_eq!(AWESOME_HUB_CATEGORIES.len(), 9);

        let html = render_yolo_aphrody_com_html();
        assert!(html.contains("yolo.aphrody.com"));
        assert!(html.contains("Dioxus 0.8"));
        assert!(html.contains("Iced 0.14"));
        assert!(html.contains("Egui 0.36"));
        assert!(html.contains("Taffy 0.15"));
        assert!(html.contains("WGPUI 30/29"));
        assert!(html.contains("Zed GPUI"));
        assert!(html.contains("Benchmarks &amp; Comparatifs"));
        assert!(html.contains("Cold Startup"));
        assert!(html.contains("Electron (Legacy)"));
    }
}
