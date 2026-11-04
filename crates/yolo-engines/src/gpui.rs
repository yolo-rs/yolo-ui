#![doc = include_str!("../README.md")]

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
pub use yolo_shared::{self, APHRODY_DARK_THEME, AphrodyThemeTokens, RgbaColor as GpuiColorRgba};

/// Component-level elevation, border, and corner geometry for `gpui-kit` widgets.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GpuiComponentStyle {
    pub radius_sm_px: f32,
    pub radius_md_px: f32,
    pub radius_lg_px: f32,
    pub border_width_px: f32,
    pub font_family_ui: String,
    pub font_family_mono: String,
    pub base_font_size_px: f32,
}

impl Default for GpuiComponentStyle {
    fn default() -> Self {
        Self {
            radius_sm_px: 6.0,
            radius_md_px: 10.0,
            radius_lg_px: 14.0,
            border_width_px: 1.0,
            font_family_ui: "Zed Plex Sans, Inter, system-ui".to_string(),
            font_family_mono: "Zed Plex Mono, JetBrains Mono, monospace".to_string(),
            base_font_size_px: 14.0,
        }
    }
}

/// Unified GPUI Theme mapped from Aphrody Labs Material 3 Dark-Glass design tokens.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GpuiTheme {
    pub name: String,
    pub bg_primary: GpuiColorRgba,
    pub bg_surface: GpuiColorRgba,
    pub bg_elevated: GpuiColorRgba,
    pub text_primary: GpuiColorRgba,
    pub text_secondary: GpuiColorRgba,
    pub accent_primary: GpuiColorRgba,
    pub accent_secondary: GpuiColorRgba,
    pub border_subtle: GpuiColorRgba,
    pub style: GpuiComponentStyle,
}

impl GpuiTheme {
    /// Returns the canonical Aphrody M3 Dark-Glass GPUI theme matching `yolo_ui::theme::APHRODY_DARK_THEME`.
    pub fn aphrody_dark() -> Self {
        Self {
            name: "aphrody-m3-dark".to_string(),
            bg_primary: GpuiColorRgba::from_u8(0x09, 0x0b, 0x10, 0xff),
            bg_surface: GpuiColorRgba::from_u8(0x11, 0x15, 0x20, 0xff),
            bg_elevated: GpuiColorRgba::from_u8(0x18, 0x1e, 0x2e, 0xff),
            text_primary: GpuiColorRgba::from_u8(0xf3, 0xf6, 0xfc, 0xff),
            text_secondary: GpuiColorRgba::from_u8(0x9b, 0xa6, 0xbf, 0xff),
            accent_primary: GpuiColorRgba::from_u8(0x7c, 0x5c, 0xff, 0xff),
            accent_secondary: GpuiColorRgba::from_u8(0x00, 0xd4, 0xff, 0xff),
            border_subtle: GpuiColorRgba::from_u8(0xff, 0xff, 0xff, 20),
            style: GpuiComponentStyle::default(),
        }
    }
}

impl Default for GpuiTheme {
    fn default() -> Self {
        Self::aphrody_dark()
    }
}

/// Window creation options for GPUI desktop (`gpui-fast`), CEF (`wef`), and mobile (`gpui-mobile`) hosts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GpuiWindowOptions {
    pub title: String,
    pub width: f32,
    pub height: f32,
    pub min_width: Option<f32>,
    pub min_height: Option<f32>,
    pub transparent: bool,
    pub client_side_decorations: bool,
    pub vsync: bool,
}

impl Default for GpuiWindowOptions {
    fn default() -> Self {
        Self {
            title: "Aphrody GPUI Studio".to_string(),
            width: 1440.0,
            height: 900.0,
            min_width: Some(640.0),
            min_height: Some(480.0),
            transparent: false,
            client_side_decorations: true,
            vsync: true,
        }
    }
}

/// Split orientation for `gpui-kit` resizable pane groups.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SplitAxis {
    Horizontal,
    Vertical,
}

/// Dock placement position within a `DockArea` workspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DockPosition {
    Left,
    Right,
    Bottom,
    Center,
}

/// Individual tab descriptor inside a `gpui-kit` panel or center dock.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TabItem {
    pub id: String,
    pub title: String,
    pub icon: Option<String>,
    pub closable: bool,
    pub dirty: bool,
}

impl TabItem {
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            icon: None,
            closable: true,
            dirty: false,
        }
    }
}

/// Dockable panel descriptor modeled after `longbridge/gpui-kit` and `zed` workspace panels.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PanelDescriptor {
    pub id: String,
    pub title: String,
    pub position: DockPosition,
    pub initial_size_px: f32,
    pub collapsible: bool,
    pub collapsed: bool,
    pub tabs: Vec<TabItem>,
    pub active_tab_id: Option<String>,
}

impl PanelDescriptor {
    pub fn new(id: impl Into<String>, title: impl Into<String>, position: DockPosition) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            position,
            initial_size_px: 280.0,
            collapsible: true,
            collapsed: false,
            tabs: Vec::new(),
            active_tab_id: None,
        }
    }

    pub fn with_tab(mut self, tab: TabItem) -> Self {
        if self.active_tab_id.is_none() {
            self.active_tab_id = Some(tab.id.clone());
        }
        self.tabs.push(tab);
        self
    }
}

/// Status bar configuration for the bottom edge of a `DockArea`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatusBarConfig {
    pub visible: bool,
    pub height_px: u32,
    pub left_items: Vec<String>,
    pub right_items: Vec<String>,
}

impl Default for StatusBarConfig {
    fn default() -> Self {
        Self {
            visible: true,
            height_px: 26,
            left_items: vec![
                "workspace.branch".to_string(),
                "diagnostics.summary".to_string(),
            ],
            right_items: vec!["locale.active".to_string(), "gpu.renderer".to_string()],
        }
    }
}

/// Multi-dock workspace layout container modeled after `longbridge/gpui-kit`'s `DockArea`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DockArea {
    pub id: String,
    pub center_split_axis: SplitAxis,
    pub panels: Vec<PanelDescriptor>,
    pub status_bar: StatusBarConfig,
}

impl DockArea {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            center_split_axis: SplitAxis::Horizontal,
            panels: Vec::new(),
            status_bar: StatusBarConfig::default(),
        }
    }

    pub fn add_panel(&mut self, panel: PanelDescriptor) {
        self.panels.push(panel);
    }

    pub fn panels_at(&self, position: DockPosition) -> Vec<&PanelDescriptor> {
        self.panels
            .iter()
            .filter(|panel| panel.position == position)
            .collect()
    }
}

/// In-memory localization catalog inspired by `longbridge/rust-i18n`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocaleCatalog {
    /// Map of `locale -> (key -> template)`
    translations: BTreeMap<String, BTreeMap<String, String>>,
}

impl LocaleCatalog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(
        &mut self,
        locale: impl Into<String>,
        key: impl Into<String>,
        template: impl Into<String>,
    ) {
        self.translations
            .entry(locale.into())
            .or_default()
            .insert(key.into(), template.into());
    }

    pub fn available_locales(&self) -> Vec<&str> {
        self.translations.keys().map(String::as_str).collect()
    }

    pub fn get(&self, locale: &str, key: &str) -> Option<&str> {
        self.translations
            .get(locale)
            .and_then(|map| map.get(key))
            .map(String::as_str)
    }
}

/// Runtime i18n context with fallback locale resolution and `{var}` template interpolation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct I18nContext {
    pub active_locale: String,
    pub fallback_locale: String,
    pub catalog: LocaleCatalog,
}

impl I18nContext {
    pub fn new(active_locale: impl Into<String>, fallback_locale: impl Into<String>) -> Self {
        let mut catalog = LocaleCatalog::new();
        catalog.insert("en", "app.ready", "Ready");
        catalog.insert("zh-CN", "app.ready", "就绪");
        catalog.insert("ja", "app.ready", "準備完了");
        Self {
            active_locale: active_locale.into(),
            fallback_locale: fallback_locale.into(),
            catalog,
        }
    }

    pub fn set_locale(&mut self, locale: impl Into<String>) {
        self.active_locale = locale.into();
    }

    /// Resolves `key` using `active_locale`, falling back to `fallback_locale`, or returning `key`.
    pub fn t<'a>(&'a self, key: &'a str) -> &'a str {
        self.catalog
            .get(&self.active_locale, key)
            .or_else(|| self.catalog.get(&self.fallback_locale, key))
            .unwrap_or(key)
    }

    /// Resolves `key` and interpolates `%{name}` placeholders using `vars`.
    pub fn t_vars(&self, key: &str, vars: &[(&str, &str)]) -> String {
        let mut rendered = self.t(key).to_string();
        for (k, v) in vars {
            let token = format!("%{{{k}}}");
            rendered = rendered.replace(&token, v);
        }
        rendered
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gpui_color_rgba_hex_roundtrip() {
        let color = GpuiColorRgba::from_hex("#7c5cff").expect("valid hex");
        assert_eq!(color.to_u32_rgba(), 0x7c5cffff);
        assert_eq!(color.to_css_hex(), "#7c5cffff");

        let alpha_color = GpuiColorRgba::from_hex("#00d4ff80").expect("valid 8-digit hex");
        assert_eq!(alpha_color.to_u32_rgba(), 0x00d4ff80);
    }

    #[test]
    fn test_aphrody_dark_theme_tokens() {
        let theme = GpuiTheme::aphrody_dark();
        assert_eq!(theme.name, "aphrody-m3-dark");
        assert_eq!(theme.bg_primary.to_u32_rgba(), 0x090b10ff);
        assert_eq!(theme.accent_primary.to_u32_rgba(), 0x7c5cffff);
        assert_eq!(theme.accent_secondary.to_u32_rgba(), 0x00d4ffff);
    }

    #[test]
    fn test_dock_area_panel_filtering() {
        let mut dock = DockArea::new("main-workspace");
        dock.add_panel(
            PanelDescriptor::new("explorer", "Explorer", DockPosition::Left)
                .with_tab(TabItem::new("files", "Files")),
        );
        dock.add_panel(
            PanelDescriptor::new("editor", "Editor", DockPosition::Center)
                .with_tab(TabItem::new("lib.rs", "src/lib.rs")),
        );
        dock.add_panel(
            PanelDescriptor::new("terminal", "Terminal (nsh)", DockPosition::Bottom)
                .with_tab(TabItem::new("nsh-1", "nsh")),
        );

        assert_eq!(dock.panels_at(DockPosition::Left).len(), 1);
        assert_eq!(
            dock.panels_at(DockPosition::Center)[0]
                .active_tab_id
                .as_deref(),
            Some("lib.rs")
        );
        assert_eq!(
            dock.panels_at(DockPosition::Bottom)[0].title,
            "Terminal (nsh)"
        );
    }

    #[test]
    fn test_i18n_context_lookup_and_interpolation() {
        let mut i18n = I18nContext::new("zh-CN", "en");
        i18n.catalog
            .insert("en", "greeting.user", "Hello, %{name}!");
        i18n.catalog
            .insert("zh-CN", "greeting.user", "你好，%{name}！");
        i18n.catalog.insert("en", "only.english", "Fallback works");

        assert_eq!(i18n.t("app.ready"), "就绪");
        assert_eq!(
            i18n.t_vars("greeting.user", &[("name", "Aphrody")]),
            "你好，Aphrody！"
        );
        assert_eq!(i18n.t("only.english"), "Fallback works");
    }
}
