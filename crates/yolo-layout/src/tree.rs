// SPDX-License-Identifier: Apache-2.0
//! Layout tree: [`LayoutStyle`] nodes in a `TaffyTree`, text leaves measured by the host.

use serde::Serialize;
use taffy::{AvailableSpace, NodeId, TaffyTree};

use crate::style::LayoutStyle;

/// A node of a [`LayoutTree`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NodeKey(NodeId);

/// Available space along one axis while measuring.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Available {
    /// A definite size in px.
    Definite(f32),
    /// Lay out at the smallest size without overflow (narrowest line breaking).
    MinContent,
    /// Lay out without wrapping.
    MaxContent,
}

impl From<AvailableSpace> for Available {
    fn from(a: AvailableSpace) -> Self {
        match a {
            AvailableSpace::Definite(v) => Self::Definite(v),
            AvailableSpace::MinContent => Self::MinContent,
            AvailableSpace::MaxContent => Self::MaxContent,
        }
    }
}

/// The question asked to the host text measurer for one text leaf.
#[derive(Clone, Copy, Debug)]
pub struct MeasureRequest<'a> {
    /// The node.
    pub key: NodeKey,
    /// The node's text.
    pub text: &'a str,
    /// Width already fixed by the layout, if any.
    pub known_width: Option<f32>,
    /// Height already fixed by the layout, if any.
    pub known_height: Option<f32>,
    /// Space available for the width.
    pub available_width: Available,
    /// Space available for the height.
    pub available_height: Available,
}

/// A computed box, relative to its parent's border box or absolute to the root.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize)]
pub struct LayoutRect {
    /// Left edge.
    pub x: f32,
    /// Top edge.
    pub y: f32,
    /// Border-box width.
    pub width: f32,
    /// Border-box height.
    pub height: f32,
}

/// One node of the JSON export.
#[derive(Clone, Debug, Serialize)]
pub struct NodeLayout {
    /// The node's name.
    pub name: String,
    /// Position relative to the parent.
    pub local: LayoutRect,
    /// Position relative to the root.
    pub absolute: LayoutRect,
    /// Children in document order.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<NodeLayout>,
}

#[derive(Clone, Debug)]
struct NodeData {
    name: String,
    text: Option<String>,
}

/// A Taffy tree of named nodes with text leaves.
pub struct LayoutTree {
    taffy: TaffyTree<NodeData>,
}

impl Default for LayoutTree {
    fn default() -> Self {
        Self::new()
    }
}

/// A Taffy error (invalid node or child index).
pub type TreeError = taffy::TaffyError;

impl LayoutTree {
    /// An empty tree with pixel rounding enabled.
    #[must_use]
    pub fn new() -> Self {
        let mut taffy = TaffyTree::new();
        taffy.enable_rounding();
        Self { taffy }
    }

    /// Adds a container node.
    pub fn node(&mut self, name: &str, style: &LayoutStyle, children: &[NodeKey]) -> Result<NodeKey, TreeError> {
        let id = self.taffy.new_with_children(style.to_taffy(), &children.iter().map(|k| k.0).collect::<Vec<_>>())?;
        self.taffy.set_node_context(id, Some(NodeData { name: name.to_owned(), text: None }))?;
        Ok(NodeKey(id))
    }

    /// Adds a text leaf measured by the callback given to [`LayoutTree::compute`].
    pub fn text(&mut self, name: &str, style: &LayoutStyle, text: &str) -> Result<NodeKey, TreeError> {
        let id = self.taffy.new_leaf_with_context(
            style.to_taffy(),
            NodeData { name: name.to_owned(), text: Some(text.to_owned()) },
        )?;
        Ok(NodeKey(id))
    }

    /// Appends `child` to `parent`.
    pub fn add_child(&mut self, parent: NodeKey, child: NodeKey) -> Result<(), TreeError> {
        self.taffy.add_child(parent.0, child.0)
    }

    /// Replaces the style of `key`.
    pub fn set_style(&mut self, key: NodeKey, style: &LayoutStyle) -> Result<(), TreeError> {
        self.taffy.set_style(key.0, style.to_taffy())
    }

    /// Lays out the subtree of `root` in `width` x `height` (`None` = max-content), asking
    /// `measure` for the intrinsic size `(width, height)` of each text leaf.
    pub fn compute(
        &mut self,
        root: NodeKey,
        width: Option<f32>,
        height: Option<f32>,
        mut measure: impl FnMut(&MeasureRequest<'_>) -> (f32, f32),
    ) -> Result<(), TreeError> {
        let space = |v: Option<f32>| v.map_or(AvailableSpace::MaxContent, AvailableSpace::Definite);
        let available = taffy::Size { width: space(width), height: space(height) };
        self.taffy.compute_layout_with_measure(root.0, available, |inputs, id, ctx, style| {
            let text = ctx.and_then(|c| c.text.as_deref());
            taffy::compute_leaf_layout(inputs, style, |_, _| 0.0, |known, avail| {
                let Some(text) = text else {
                    return taffy::Size { width: known.width.unwrap_or(0.0), height: known.height.unwrap_or(0.0) };
                };
                let (w, h) = measure(&MeasureRequest {
                    key: NodeKey(id),
                    text,
                    known_width: known.width,
                    known_height: known.height,
                    available_width: avail.width.into(),
                    available_height: avail.height.into(),
                });
                taffy::Size { width: known.width.unwrap_or(w), height: known.height.unwrap_or(h) }
            })
        })
    }

    /// The computed box of `key` relative to its parent.
    pub fn local_rect(&self, key: NodeKey) -> Result<LayoutRect, TreeError> {
        let l = self.taffy.layout(key.0)?;
        Ok(LayoutRect { x: l.location.x, y: l.location.y, width: l.size.width, height: l.size.height })
    }

    /// The computed box of `key` relative to the root.
    pub fn rect(&self, key: NodeKey) -> Result<LayoutRect, TreeError> {
        let mut r = self.local_rect(key)?;
        let mut node = key.0;
        while let Some(parent) = self.taffy.parent(node) {
            let l = self.taffy.layout(parent)?;
            r.x += l.location.x;
            r.y += l.location.y;
            node = parent;
        }
        Ok(r)
    }

    /// The name given to `key`.
    #[must_use]
    pub fn name(&self, key: NodeKey) -> &str {
        self.taffy.get_node_context(key.0).map_or("", |d| d.name.as_str())
    }

    /// Finds the first node named `name` in the subtree of `root` (depth first).
    #[must_use]
    pub fn find(&self, root: NodeKey, name: &str) -> Option<NodeKey> {
        if self.name(root) == name {
            return Some(root);
        }
        self.taffy.children(root.0).ok()?.into_iter().find_map(|c| self.find(NodeKey(c), name))
    }

    /// The computed layout of the subtree of `root`.
    pub fn export(&self, root: NodeKey) -> Result<NodeLayout, TreeError> {
        let origin = self.rect(root)?;
        let base = LayoutRect { x: origin.x - self.local_rect(root)?.x, y: origin.y - self.local_rect(root)?.y, ..origin };
        self.export_at(root, base.x, base.y)
    }

    fn export_at(&self, key: NodeKey, px: f32, py: f32) -> Result<NodeLayout, TreeError> {
        let local = self.local_rect(key)?;
        let absolute = LayoutRect { x: px + local.x, y: py + local.y, ..local };
        let children = self
            .taffy
            .children(key.0)?
            .into_iter()
            .map(|c| self.export_at(NodeKey(c), absolute.x, absolute.y))
            .collect::<Result<_, _>>()?;
        Ok(NodeLayout { name: self.name(key).to_owned(), local, absolute, children })
    }

    /// The computed layout of the subtree of `root` as pretty JSON.
    pub fn to_json(&self, root: NodeKey) -> Result<String, TreeError> {
        let layout = self.export(root)?;
        Ok(serde_json::to_string_pretty(&layout).unwrap_or_default())
    }
}
