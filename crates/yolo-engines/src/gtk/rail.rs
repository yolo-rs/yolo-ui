// SPDX-License-Identifier: Apache-2.0
//! M3 navigation rail (`80 px` wide, `56x32` pill indicator, optional top FAB) descriptor and
//! optional `system-gtk4` widget builder.

use serde::{Deserialize, Serialize};

/// One destination in an M3 navigation rail.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RailItem<'a> {
    /// `GtkStack` child name.
    pub name: &'a str,
    /// Short caption under the icon (`label-medium`).
    pub title: &'a str,
    /// Material Symbols codepoint (`\u{e88a}` = `home`).
    pub symbol: char,
}

impl<'a> RailItem<'a> {
    /// Creates a new [`RailItem`].
    #[must_use]
    pub const fn new(name: &'a str, title: &'a str, symbol: char) -> Self {
        Self {
            name,
            title,
            symbol,
        }
    }
}

/// Pure-Rust descriptor for an M3 navigation rail (`box.m3-nav-rail`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NavigationRailDescriptor {
    /// Width in logical pixels (`80` dp standard M3 rail).
    pub width_px: u32,
    /// Selected destination name.
    pub selected: String,
    /// Destination names, titles, and Material Symbol codepoints.
    pub items: Vec<(String, String, char)>,
}

impl NavigationRailDescriptor {
    /// Creates a new [`NavigationRailDescriptor`] from a slice of [`RailItem`]s.
    #[must_use]
    pub fn new(items: &[RailItem<'_>]) -> Self {
        let selected = items
            .first()
            .map(|item| item.name.to_owned())
            .unwrap_or_default();
        Self {
            width_px: 80,
            selected,
            items: items
                .iter()
                .map(|i| (i.name.to_owned(), i.title.to_owned(), i.symbol))
                .collect(),
        }
    }
}

#[cfg(feature = "system-gtk4")]
use gtk::prelude::*;

/// Builds an `80 px` M3 navigation rail bound to `stack`: selecting a row switches the visible
/// `GtkStack` child.
#[cfg(feature = "system-gtk4")]
#[must_use]
pub fn navigation_rail(
    stack: &gtk::Stack,
    items: &[RailItem<'_>],
    fab: Option<&gtk::Button>,
) -> gtk::Box {
    let rail = gtk::Box::new(gtk::Orientation::Vertical, 12);
    rail.add_css_class("m3-nav-rail");
    rail.set_vexpand(true);

    if let Some(fab) = fab {
        fab.set_halign(gtk::Align::Center);
        rail.append(fab);
    }

    let list = gtk::ListBox::new();
    list.add_css_class("m3-nav-rail-list");
    list.set_selection_mode(gtk::SelectionMode::Single);
    list.set_vexpand(true);

    let names: Vec<String> = items.iter().map(|i| i.name.to_owned()).collect();
    for item in items {
        let cell = gtk::Box::new(gtk::Orientation::Vertical, 4);
        cell.set_halign(gtk::Align::Center);

        let pill = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        pill.add_css_class("m3-nav-pill");
        pill.set_halign(gtk::Align::Center);
        let icon = super::icons::symbol(item.symbol, false);
        icon.set_halign(gtk::Align::Center);
        icon.set_valign(gtk::Align::Center);
        icon.set_hexpand(true);
        pill.append(&icon);

        let label = gtk::Label::new(Some(item.title));
        label.add_css_class("m3-nav-label");
        label.set_halign(gtk::Align::Center);

        cell.append(&pill);
        cell.append(&label);

        let row = gtk::ListBoxRow::new();
        row.set_child(Some(&cell));
        row.update_property(&[gtk::accessible::Property::Label(item.title)]);
        list.append(&row);
    }

    let stack_weak = stack.downgrade();
    list.connect_row_selected(move |_, row| {
        if let (Some(row), Some(stack)) = (row, stack_weak.upgrade()) {
            let idx = row.index();
            if let Some(name) = names.get(idx as usize) {
                stack.set_visible_child_name(name);
            }
        }
    });
    if let Some(first) = list.row_at_index(0) {
        list.select_row(Some(&first));
    }

    rail.append(&list);
    rail
}
