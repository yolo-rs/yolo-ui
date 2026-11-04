// SPDX-License-Identifier: Apache-2.0
//! Pure-Rust GTK 4.16 + libadwaita 1.6 + Relm4 widget descriptors, GtkBuilder `.ui` XML builders,
//! and optional native `system-gtk4` widget constructors.

use serde::{Deserialize, Serialize};
use std::fmt::Write;
use yolo_shared::YoloThemePreset;

/// M3 common button variants (`button.m3-*`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ButtonKind {
    /// `button.m3-filled`.
    #[default]
    Filled,
    /// `button.m3-tonal`.
    Tonal,
    /// `button.m3-elevated`.
    Elevated,
    /// `button.m3-outlined`.
    Outlined,
    /// `button.m3-text`.
    Text,
}

impl ButtonKind {
    /// CSS class applied to the `GtkButton`.
    #[must_use]
    pub const fn css_class(self) -> &'static str {
        match self {
            Self::Filled => "m3-filled",
            Self::Tonal => "m3-tonal",
            Self::Elevated => "m3-elevated",
            Self::Outlined => "m3-outlined",
            Self::Text => "m3-text",
        }
    }
}

/// M3 card variants (`.m3-card-*`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum CardKind {
    /// `.m3-card-filled`.
    #[default]
    Filled,
    /// `.m3-card-elevated`.
    Elevated,
    /// `.m3-card-outlined`.
    Outlined,
}

impl CardKind {
    /// CSS class applied to the `GtkBox`.
    #[must_use]
    pub const fn css_class(self) -> &'static str {
        match self {
            Self::Filled => "m3-card-filled",
            Self::Elevated => "m3-card-elevated",
            Self::Outlined => "m3-card-outlined",
        }
    }
}

/// M3 FAB sizes (`button.m3-fab*`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum FabSize {
    /// 40 px.
    Small,
    /// 56 px.
    #[default]
    Regular,
    /// 96 px.
    Large,
}

impl FabSize {
    /// Optional modifier CSS class in addition to `m3-fab`.
    #[must_use]
    pub const fn modifier_class(self) -> Option<&'static str> {
        match self {
            Self::Small => Some("m3-fab-sm"),
            Self::Regular => None,
            Self::Large => Some("m3-fab-lg"),
        }
    }
}

/// Pure-Rust descriptor of a GTK4 / libadwaita widget node that can be serialized to GtkBuilder `.ui` XML.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GtkWidgetDescriptor {
    /// GTK / Adw class name (e.g. `"GtkButton"`, `"AdwHeaderBar"`, `"GtkBox"`).
    pub class: String,
    /// Widget ID inside the GtkBuilder `.ui` tree.
    pub id: String,
    /// CSS classes attached to `<style><class name="..."/></style>`.
    pub css_classes: Vec<String>,
    /// Accessible label (`accessible-role` / `label` property).
    pub label: Option<String>,
    /// Icon name or Material Symbol glyph.
    pub icon_name: Option<String>,
    /// Preset theme tag.
    pub preset: YoloThemePreset,
}

impl GtkWidgetDescriptor {
    /// Creates a new `GtkButton` descriptor with the given M3 [`ButtonKind`].
    #[must_use]
    pub fn button(
        id: impl Into<String>,
        kind: ButtonKind,
        label: impl Into<String>,
        preset: YoloThemePreset,
    ) -> Self {
        Self {
            class: "GtkButton".to_owned(),
            id: id.into(),
            css_classes: vec![
                kind.css_class().to_owned(),
                format!("yolo-preset-{}", preset.id()),
            ],
            label: Some(label.into()),
            icon_name: None,
            preset,
        }
    }

    /// Creates a new `GtkBox` card descriptor with the given [`CardKind`].
    #[must_use]
    pub fn card(id: impl Into<String>, kind: CardKind, preset: YoloThemePreset) -> Self {
        Self {
            class: "GtkBox".to_owned(),
            id: id.into(),
            css_classes: vec![
                kind.css_class().to_owned(),
                format!("yolo-preset-{}", preset.id()),
            ],
            label: None,
            icon_name: None,
            preset,
        }
    }

    /// Creates a new `GtkButton` Floating Action Button descriptor.
    #[must_use]
    pub fn fab(
        id: impl Into<String>,
        size: FabSize,
        icon_name: impl Into<String>,
        accessible_label: impl Into<String>,
        preset: YoloThemePreset,
    ) -> Self {
        let mut css_classes = vec!["m3-fab".to_owned()];
        if let Some(extra) = size.modifier_class() {
            css_classes.push(extra.to_owned());
        }
        Self {
            class: "GtkButton".to_owned(),
            id: id.into(),
            css_classes,
            label: Some(accessible_label.into()),
            icon_name: Some(icon_name.into()),
            preset,
        }
    }

    /// Serializes this widget descriptor into a GtkBuilder `.ui` `<object>` XML fragment.
    #[must_use]
    pub fn to_builder_xml(&self) -> String {
        let mut xml = String::new();
        let _ = writeln!(
            xml,
            "  <object class=\"{}\" id=\"{}\">",
            self.class, self.id
        );
        if let Some(label) = &self.label {
            let _ = writeln!(
                xml,
                "    <property name=\"label\">{}</property>",
                escape_xml(label)
            );
        }
        if let Some(icon) = &self.icon_name {
            let _ = writeln!(
                xml,
                "    <property name=\"icon-name\">{}</property>",
                escape_xml(icon)
            );
        }
        if !self.css_classes.is_empty() {
            xml.push_str("    <style>\n");
            for cls in &self.css_classes {
                let _ = writeln!(xml, "      <class name=\"{}\"/>", escape_xml(cls));
            }
            xml.push_str("    </style>\n");
        }
        xml.push_str("  </object>\n");
        xml
    }
}

/// Builds a complete GtkBuilder `.ui` XML document wrapping `AdwApplicationWindow` + child widgets.
#[must_use]
pub fn gtk_builder_xml(
    window_id: &str,
    title: &str,
    preset: YoloThemePreset,
    children: &[GtkWidgetDescriptor],
) -> String {
    let mut xml = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<interface>\n  <requires lib=\"gtk\" version=\"4.16\"/>\n  <requires lib=\"libadwaita\" version=\"1.6\"/>\n",
    );
    let _ = writeln!(
        xml,
        "  <object class=\"AdwApplicationWindow\" id=\"{}\">\n    <property name=\"title\">{}</property>\n    <property name=\"default-width\">1280</property>\n    <property name=\"default-height\">800</property>\n    <style>\n      <class name=\"yolo-preset-{}\"/>\n    </style>",
        escape_xml(window_id),
        escape_xml(title),
        preset.id()
    );
    for child in children {
        let _ = writeln!(xml, "    <child>\n{}    </child>", child.to_builder_xml());
    }
    xml.push_str("  </object>\n</interface>\n");
    xml
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

// ── Native GTK4 widget constructors (when `system-gtk4` is enabled) ──────────

#[cfg(feature = "system-gtk4")]
use gtk::prelude::*;

/// Builds an M3 common button (`40 px` height, pill shape).
#[cfg(feature = "system-gtk4")]
#[must_use]
pub fn button(kind: ButtonKind, label: &str) -> gtk::Button {
    let b = gtk::Button::with_label(label);
    b.add_css_class(kind.css_class());
    b
}

/// Builds an M3 card (`GtkBox`, vertical, `12 px` spacing, `16 px` padding).
#[cfg(feature = "system-gtk4")]
#[must_use]
pub fn card(kind: CardKind) -> gtk::Box {
    let bx = gtk::Box::new(gtk::Orientation::Vertical, 12);
    bx.add_css_class(kind.css_class());
    bx
}

/// Builds an M3 Floating Action Button showing a Material Symbols `codepoint` (`\u{e145}` = `add`).
#[cfg(feature = "system-gtk4")]
#[must_use]
pub fn fab(size: FabSize, codepoint: char, accessible_label: &str) -> gtk::Button {
    let icon = super::icons::symbol(codepoint, false);
    let b = gtk::Button::new();
    b.set_child(Some(&icon));
    b.add_css_class("m3-fab");
    if let Some(extra) = size.modifier_class() {
        b.add_css_class(extra);
    }
    b.set_tooltip_text(Some(accessible_label));
    b.update_property(&[gtk::accessible::Property::Label(accessible_label)]);
    b
}

/// Builds an M3 filter chip (`GtkToggleButton`, `32 px`, `small` shape).
#[cfg(feature = "system-gtk4")]
#[must_use]
pub fn filter_chip(label: &str, active: bool) -> gtk::ToggleButton {
    let t = gtk::ToggleButton::with_label(label);
    t.add_css_class("m3-chip");
    t.set_active(active);
    t
}

/// Shows an M3 snackbar on `overlay` (`AdwToastOverlay`).
#[cfg(feature = "system-gtk4")]
pub fn show_snackbar(
    overlay: &adw::ToastOverlay,
    message: &str,
    action_label: Option<&str>,
) -> adw::Toast {
    let toast = adw::Toast::new(message);
    toast.set_timeout(4);
    if let Some(act) = action_label {
        toast.set_button_label(Some(act));
        toast.set_timeout(10);
    }
    overlay.add_toast(toast.clone());
    toast
}
