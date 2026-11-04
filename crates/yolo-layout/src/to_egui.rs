// SPDX-License-Identifier: Apache-2.0
//! Computed rects as `egui::Rect`: lay out with [`LayoutTree`], then place widgets with
//! `Ui::put`. egui keeps its own immediate-mode layout inside each placed widget.

use crate::tree::{LayoutRect, LayoutTree, NodeKey, TreeError};

impl From<LayoutRect> for egui::Rect {
    fn from(r: LayoutRect) -> Self {
        egui::Rect::from_min_size(egui::pos2(r.x, r.y), egui::vec2(r.width, r.height))
    }
}

impl LayoutTree {
    /// The box of `key` in egui coordinates, offset by `origin` (usually `ui.max_rect().min`).
    pub fn egui_rect(&self, key: NodeKey, origin: egui::Pos2) -> Result<egui::Rect, TreeError> {
        Ok(egui::Rect::from(self.rect(key)?).translate(origin.to_vec2()))
    }

    /// Places `widget` in the box of `key`, relative to the top-left of `ui`.
    pub fn put(&self, ui: &mut egui::Ui, key: NodeKey, widget: impl egui::Widget) -> Result<egui::Response, TreeError> {
        let rect = self.egui_rect(key, ui.max_rect().min)?;
        Ok(ui.put(rect, widget))
    }
}
