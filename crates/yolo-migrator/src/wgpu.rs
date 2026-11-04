// SPDX-License-Identifier: Apache-2.0
//! Pure Rust WebGPU / WGPU + Taffy (`yolo-layout`) layout & render descriptor emitter (`MigrationTarget::Wgpu`).
//!
//! Emits a self-contained Rust module constructing a `yolo_layout::UiTree` (Taffy 0.15 flexbox/grid)
//! paired with GPU draw primitives (`WgpuNodeDescriptor`, surface/accent colors from `yolo-shared`).

use std::fmt::Write;

use yolo_shared::YoloThemePreset;

use crate::ir::{Align, Comp, Dir, Document, Justify, Kind, Node, shape_radius, slug, typescale_metrics};

struct WgpuEmitter {
    out: String,
    counter: usize,
    theme: YoloThemePreset,
}

fn lit(s: &str) -> String {
    format!("{s:?}")
}

fn f32_lit(n: f64) -> String {
    let s = format!("{n:?}");
    if s.contains('.') || s.contains('e') { s } else { format!("{s}.0") }
}

fn pascal(s: &str) -> String {
    let mut out = String::new();
    let mut up = true;
    for ch in s.chars() {
        if ch.is_ascii_alphanumeric() {
            if up {
                out.push(ch.to_ascii_uppercase());
            } else {
                out.push(ch);
            }
            up = false;
        } else {
            up = true;
        }
    }
    if out.is_empty() {
        "Scene".to_string()
    } else {
        out
    }
}

fn role_hex(role: Option<&str>, theme: YoloThemePreset, is_bg: bool) -> &'static str {
    let t = theme.tokens(true);
    match role {
        Some("primary" | "surface-tint") => t.accent_primary,
        Some("secondary" | "tertiary") => t.accent_secondary,
        Some("background") => t.bg_primary,
        Some("surface" | "surface-container" | "surface-container-low") => t.bg_surface,
        Some("surface-container-high" | "surface-container-highest" | "primary-container") => t.bg_elevated,
        Some("on-surface" | "on-primary" | "on-background") => t.text_primary,
        Some("on-surface-variant") => t.text_secondary,
        Some("outline" | "outline-variant") => t.border_subtle,
        _ => {
            if is_bg {
                t.bg_surface
            } else {
                t.text_primary
            }
        }
    }
}

impl WgpuEmitter {
    fn line(&mut self, ind: usize, s: &str) {
        let _ = writeln!(self.out, "{:ind$}{s}", "", ind = ind * 4);
    }

    fn emit_node(&mut self, ind: usize, n: &Node) -> String {
        self.counter += 1;
        let id = self.counter;
        let var = format!("node_{id}");
        let style_var = format!("style_{id}");

        self.line(ind, &format!("let mut {style_var} = LayoutStyle::default();"));
        if n.style.display_grid {
            self.line(ind, &format!("{style_var}.display = Display::Grid;"));
        } else {
            self.line(ind, &format!("{style_var}.display = Display::Flex;"));
            let dir = match n.dir() {
                Dir::Row => "FlexDirection::Row",
                Dir::Column => "FlexDirection::Column",
            };
            self.line(ind, &format!("{style_var}.flex_direction = {dir};"));
        }
        if let Some(g) = n.style.gap {
            self.line(ind, &format!("{style_var}.row_gap = Length::Px({});", f32_lit(g)));
            self.line(ind, &format!("{style_var}.column_gap = Length::Px({});", f32_lit(g)));
        }
        if let Some(p) = n.style.padding {
            self.line(
                ind,
                &format!(
                    "{style_var}.padding = Edges {{ top: Length::Px({}), right: Length::Px({}), bottom: Length::Px({}), left: Length::Px({}) }};",
                    f32_lit(p[0]),
                    f32_lit(p[1]),
                    f32_lit(p[2]),
                    f32_lit(p[3]),
                ),
            );
        } else if n.is(Comp::Card) {
            self.line(ind, &format!("{style_var}.padding = Edges::all(Length::Px(16.0));"));
        }
        if let Some(a) = n.style.align {
            let kw = match a {
                Align::Start => "ItemKeyword::FlexStart",
                Align::Center => "ItemKeyword::Center",
                Align::End => "ItemKeyword::FlexEnd",
                Align::Stretch => "ItemKeyword::Stretch",
            };
            self.line(ind, &format!("{style_var}.align_items.keyword = {kw};"));
        }
        if let Some(j) = n.style.justify {
            let kw = match j {
                Justify::Start => "ContentKeyword::FlexStart",
                Justify::Center => "ContentKeyword::Center",
                Justify::End => "ContentKeyword::FlexEnd",
                Justify::SpaceBetween => "ContentKeyword::SpaceBetween",
            };
            self.line(ind, &format!("{style_var}.justify_content.keyword = {kw};"));
        }
        if n.style.grow {
            self.line(ind, &format!("{style_var}.flex_grow = 1.0;"));
        }
        if n.style.fill_width {
            self.line(ind, &format!("{style_var}.width = Length::Percent(1.0);"));
        } else if let Some(w) = n.style.width {
            self.line(ind, &format!("{style_var}.width = Length::Px({});", f32_lit(w)));
        }
        if n.style.fill_height {
            self.line(ind, &format!("{style_var}.height = Length::Percent(1.0);"));
        } else if let Some(h) = n.style.height {
            self.line(ind, &format!("{style_var}.height = Length::Px({});", f32_lit(h)));
        }

        let mut child_vars = Vec::new();
        for c in &n.children {
            child_vars.push(self.emit_node(ind, c));
        }

        if child_vars.is_empty() {
            self.line(ind, &format!("let {var} = tree.new_leaf(&{style_var});"));
        } else {
            self.line(
                ind,
                &format!("let {var} = tree.new_with_children(&{style_var}, &[{}]);", child_vars.join(", ")),
            );
        }

        let kind_str = match n.kind {
            Kind::Box => "Box",
            Kind::Text(_) => "Text",
            Kind::Image => "Image",
            Kind::Comp(c) => match c {
                Comp::Button => "Button",
                Comp::IconButton => "IconButton",
                Comp::Fab => "Fab",
                Comp::Switch => "Switch",
                Comp::Checkbox => "Checkbox",
                Comp::Radio => "Radio",
                Comp::Slider => "Slider",
                Comp::TextField => "TextField",
                Comp::Card => "Card",
                Comp::Dialog => "Dialog",
                Comp::List => "List",
                Comp::ListItem => "ListItem",
                Comp::Divider => "Divider",
                Comp::Tabs => "Tabs",
                Comp::Tab => "Tab",
                Comp::TopAppBar => "TopAppBar",
                Comp::NavigationRail => "NavigationRail",
                Comp::NavigationRailItem => "NavigationRailItem",
                Comp::Chip => "Chip",
                Comp::Progress => "Progress",
            },
        };
        let label = n.label(&["label", "title", "headline", "placeholder"]).unwrap_or("");
        let radius = n
            .style
            .shape
            .as_deref()
            .and_then(shape_radius)
            .unwrap_or(if n.is(Comp::Card) { 12.0 } else { 0.0 });
        let font_px = n
            .style
            .typescale
            .as_deref()
            .and_then(typescale_metrics)
            .map(|(sz, _)| sz)
            .unwrap_or(14.0);
        let bg_hex = role_hex(n.style.background.as_deref(), self.theme, true);
        let fg_hex = role_hex(n.style.color.as_deref(), self.theme, false);

        self.line(
            ind,
            &format!(
                "primitives.push(WgpuPrimitive {{ node: {var}, kind: {}, label: {}, bg_hex: {}, fg_hex: {}, corner_radius_px: {}, font_size_px: {} }});",
                lit(kind_str),
                lit(label),
                lit(bg_hex),
                lit(fg_hex),
                f32_lit(radius),
                f32_lit(font_px),
            ),
        );

        var
    }
}

pub fn emit(doc: &Document, theme: YoloThemePreset) -> String {
    let mut e = WgpuEmitter {
        out: String::new(),
        counter: 0,
        theme,
    };
    let root_var = e.emit_node(1, &doc.root);
    let comp = pascal(&doc.component);
    let snake = slug(&doc.component);

    let mut out = format!(
        "// Generated by yolo-migrator from {} (component {}, theme {}). Do not hand-edit.\n\
         // Target: WebGPU / WGPU + Taffy (`yolo-layout`) descriptor.\n\n\
         use yolo_layout::{{ContentKeyword, Display, Edges, FlexDirection, ItemKeyword, LayoutStyle, Length, NodeId, UiTree}};\n\
         use yolo_shared::YoloThemePreset;\n\n\
         #[derive(Debug, Clone)]\n\
         pub struct WgpuPrimitive {{\n\
             pub node: NodeId,\n\
             pub kind: &'static str,\n\
             pub label: &'static str,\n\
             pub bg_hex: &'static str,\n\
             pub fg_hex: &'static str,\n\
             pub corner_radius_px: f32,\n\
             pub font_size_px: f32,\n\
         }}\n\n\
         #[derive(Debug)]\n\
         pub struct {comp}WgpuScene {{\n\
             pub theme: YoloThemePreset,\n\
             pub tree: UiTree,\n\
             pub root: NodeId,\n\
             pub primitives: Vec<WgpuPrimitive>,\n\
         }}\n\n\
         pub fn build_{snake}_wgpu_scene(viewport_width: f32, viewport_height: f32) -> {comp}WgpuScene {{\n\
             let mut tree = UiTree::new();\n\
             let mut primitives = Vec::new();\n",
        doc.source,
        doc.component,
        theme.id()
    );
    out.push_str(&e.out);
    let _ = writeln!(out, "    tree.compute({root_var}, viewport_width, viewport_height);");
    let _ = writeln!(
        out,
        "    {comp}WgpuScene {{ theme: YoloThemePreset::{:?}, tree, root: {root_var}, primitives }}",
        theme
    );
    let _ = writeln!(out, "}}");
    out
}
