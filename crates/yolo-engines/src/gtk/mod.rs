// SPDX-License-Identifier: Apache-2.0
//! GTK 4.16 + libadwaita 1.6 + Relm4 adapter for Yolo UI and Material Design 3:
//! - [`theme::stylesheet`] and [`theme::preset_stylesheet`] generate GTK 4 / libadwaita 1.6+ CSS
//!   for any `(seed, variant, contrast)` or any of the 5 [`YoloThemePreset`]s (`Neutral`, `M3`,
//!   `Fluent`, `Apple`, `Aphrody`);
//! - [`widgets::GtkWidgetDescriptor`] and [`widgets::gtk_builder_xml`] generate GtkBuilder `.ui`
//!   XML definitions and M3/Fluent/Apple/Aphrody widget descriptors without requiring native C
//!   `pkg-config` GTK4 libraries;
//! - [`rail::NavigationRailDescriptor`] and [`icons::MaterialSymbol`] describe M3 navigation rails
//!   and variable-font Material Symbols;
//! - When the optional `system-gtk4` feature is enabled, `theme::M3Gtk`, `widgets::button`,
//!   `widgets::card`, `widgets::fab`, and `rail::navigation_rail` provide live native `gtk4` /
//!   `libadwaita` bindings.

#[cfg(feature = "system-gtk4")]
pub use adw;
#[cfg(feature = "system-gtk4")]
pub use gtk;
pub use yolo_shared::{
    ByronLoveColorTeinte, ByronLovePetSpec, YoloManifest, YoloStyle, YoloThemePreset,
    YoloThemeTokens, BYRON_LOVE_PALETTE, BYRON_LOVE_PET_SPEC,
};
pub use yolo_tokens::role::Role;
pub use yolo_tokens::scheme::{DynamicScheme, Variant};

pub mod icons;
pub mod rail;
pub mod theme;
pub mod widgets;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stylesheet_contains_adw_variables_and_m3_component_rules() {
        let css = theme::stylesheet(0xFF67_50A4, Variant::TonalSpot, 0.0);
        assert!(css.contains("--accent-bg-color:"));
        assert!(css.contains("@media (prefers-color-scheme: dark)"));
        assert!(css.contains("button.m3-filled"));
        assert!(css.contains("box.m3-nav-rail"));
        assert!(css.contains(".m3-card-elevated"));
    }

    #[test]
    fn all_five_presets_generate_gtk_stylesheets_and_builder_xml() {
        for preset in YoloThemePreset::ALL {
            let css = theme::preset_stylesheet(preset);
            assert!(css.contains(preset.id()));
            assert!(css.contains("--accent-bg-color:"));

            let btn = widgets::GtkWidgetDescriptor::button(
                "save_btn",
                widgets::ButtonKind::Filled,
                "Save",
                preset,
            );
            let card = widgets::GtkWidgetDescriptor::card(
                "main_card",
                widgets::CardKind::Elevated,
                preset,
            );
            let fab = widgets::GtkWidgetDescriptor::fab(
                "create_fab",
                widgets::FabSize::Regular,
                "list-add-symbolic",
                "Create Item",
                preset,
            );
            let xml = widgets::gtk_builder_xml("main_win", "Yolo GTK Window", preset, &[btn, card, fab]);
            assert!(xml.contains("<object class=\"AdwApplicationWindow\" id=\"main_win\">"));
            assert!(xml.contains("m3-filled"));
            assert!(xml.contains("m3-card-elevated"));
            assert!(xml.contains("m3-fab"));
        }
    }

    #[test]
    fn navigation_rail_and_material_symbols_descriptors_work() {
        let items = [
            rail::RailItem::new("home", "Home", icons::codepoints::HOME),
            rail::RailItem::new("settings", "Settings", icons::codepoints::SETTINGS),
        ];
        let nav = rail::NavigationRailDescriptor::new(&items);
        assert_eq!(nav.width_px, 80);
        assert_eq!(nav.selected, "home");
        assert_eq!(nav.items.len(), 2);

        let sym = icons::MaterialSymbol::new(icons::codepoints::PALETTE, true);
        assert_eq!(sym.css_class(), "m3-symbol-filled");
    }
}
