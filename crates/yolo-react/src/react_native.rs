// SPDX-License-Identifier: Apache-2.0
//! React Native `StyleSheet` -> Rust Taffy 0.15 (`yolo-layout`) Layout Engine
//! & C++ JSI / TurboModule / UniFFI Bridge Code Generator.
//!
//! Replaces C++ Yoga on the server, desktop, and WASM with `yolo-layout` (Taffy 0.15),
//! computing exact `(x, y, width, height)` boxes from React Native `ViewStyle` / `TextStyle` trees,
//! and emits C++ JSI `TurboModule` + TypeScript spec bindings for Rust structs.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use yolo_layout::{
    ContentAlign, ContentKeyword, Display, FlexDirection, FlexWrap, ItemAlign, ItemKeyword,
    LayoutRect, LayoutStyle, LayoutTree, Length, NodeKey, NodeLayout, Position, Safety, Sides,
    TokenTable, TreeError, monospace_measure, parse_style,
};

/// React Native style properties (`ViewStyle` / `TextStyle` / `FlexStyle`) mapped to `yolo-layout` (Taffy 0.15).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RnStyle {
    /// `"flex"` (default in React Native) or `"none"`.
    pub display: Option<String>,
    /// `"relative"` (default) or `"absolute"`.
    pub position: Option<String>,
    /// `"column"` (React Native default!), `"row"`, `"column-reverse"`, `"row-reverse"`.
    pub flex_direction: Option<String>,
    /// `"nowrap"`, `"wrap"`, `"wrap-reverse"`.
    pub flex_wrap: Option<String>,
    /// `"flex-start"`, `"flex-end"`, `"center"`, `"space-between"`, `"space-around"`, `"space-evenly"`.
    pub justify_content: Option<String>,
    /// `"stretch"` (default), `"flex-start"`, `"flex-end"`, `"center"`, `"baseline"`.
    pub align_items: Option<String>,
    /// Shorthand `flex` weight (`flexGrow`).
    pub flex: Option<f32>,
    /// `flexGrow` factor.
    pub flex_grow: Option<f32>,
    /// `flexShrink` factor.
    pub flex_shrink: Option<f32>,
    /// `width` in logical dp/px.
    pub width: Option<f32>,
    /// `height` in logical dp/px.
    pub height: Option<f32>,
    /// `minWidth` in logical dp/px.
    pub min_width: Option<f32>,
    /// `minHeight` in logical dp/px.
    pub min_height: Option<f32>,
    /// `maxWidth` in logical dp/px.
    pub max_width: Option<f32>,
    /// `maxHeight` in logical dp/px.
    pub max_height: Option<f32>,
    /// All-sides padding in dp/px.
    pub padding: Option<f32>,
    /// Horizontal (`left` + `right`) padding in dp/px.
    pub padding_horizontal: Option<f32>,
    /// Vertical (`top` + `bottom`) padding in dp/px.
    pub padding_vertical: Option<f32>,
    /// All-sides margin in dp/px.
    pub margin: Option<f32>,
    /// Unified gap (`rowGap` + `columnGap`) in dp/px.
    pub gap: Option<f32>,
    /// `rowGap` in dp/px.
    pub row_gap: Option<f32>,
    /// `columnGap` in dp/px.
    pub column_gap: Option<f32>,
    /// Background color token or hex string.
    pub background_color: Option<String>,
    /// Border radius in dp/px.
    pub border_radius: Option<f32>,
}

impl RnStyle {
    /// Converts a React Native `RnStyle` into a `yolo-layout` [`LayoutStyle`] for Taffy 0.15.
    /// Note that React Native defaults `display` to `flex` and `flexDirection` to `column` (like Yoga).
    pub fn to_layout_style(&self) -> LayoutStyle {
        let mut style = parse_style(
            "display: flex; flex-direction: column;",
            &TokenTable::default(),
        )
        .expect("valid default React Native flex style");

        if let Some(d) = &self.display {
            style.display = match d.as_str() {
                "none" => Display::None,
                _ => Display::Flex,
            };
        }
        if let Some(pos) = &self.position {
            style.position = match pos.as_str() {
                "absolute" => Position::Absolute,
                _ => Position::Relative,
            };
        }
        if let Some(dir) = &self.flex_direction {
            style.flex_direction = match dir.as_str() {
                "row" => FlexDirection::Row,
                "row-reverse" => FlexDirection::RowReverse,
                "column-reverse" => FlexDirection::ColumnReverse,
                _ => FlexDirection::Column,
            };
        }
        if let Some(wrap) = &self.flex_wrap {
            style.flex_wrap = match wrap.as_str() {
                "wrap" => FlexWrap::Wrap,
                "wrap-reverse" => FlexWrap::WrapReverse,
                _ => FlexWrap::NoWrap,
            };
        }
        if let Some(jc) = &self.justify_content {
            let keyword = match jc.as_str() {
                "flex-end" | "end" => ContentKeyword::FlexEnd,
                "center" => ContentKeyword::Center,
                "space-between" => ContentKeyword::SpaceBetween,
                "space-around" => ContentKeyword::SpaceAround,
                "space-evenly" => ContentKeyword::SpaceEvenly,
                _ => ContentKeyword::FlexStart,
            };
            style.justify_content = ContentAlign {
                keyword,
                safety: Safety::Default,
            };
        }
        if let Some(ai) = &self.align_items {
            let keyword = match ai.as_str() {
                "flex-start" | "start" => ItemKeyword::FlexStart,
                "flex-end" | "end" => ItemKeyword::FlexEnd,
                "center" => ItemKeyword::Center,
                "baseline" => ItemKeyword::Baseline,
                _ => ItemKeyword::Stretch,
            };
            style.align_items = ItemAlign {
                keyword,
                safety: Safety::Default,
            };
        }

        if let Some(flex) = self.flex {
            style.flex_grow = flex;
            if flex > 0.0 {
                style.flex_shrink = 1.0;
            }
        }
        if let Some(fg) = self.flex_grow {
            style.flex_grow = fg;
        }
        if let Some(fs) = self.flex_shrink {
            style.flex_shrink = fs;
        }

        if let Some(w) = self.width {
            style.width = Length::Px(w);
        }
        if let Some(h) = self.height {
            style.height = Length::Px(h);
        }
        if let Some(mw) = self.min_width {
            style.min_width = Length::Px(mw);
        }
        if let Some(mh) = self.min_height {
            style.min_height = Length::Px(mh);
        }
        if let Some(xw) = self.max_width {
            style.max_width = Length::Px(xw);
        }
        if let Some(xh) = self.max_height {
            style.max_height = Length::Px(xh);
        }

        let pad_base = self.padding.unwrap_or(0.0);
        let pad_h = self.padding_horizontal.unwrap_or(pad_base);
        let pad_v = self.padding_vertical.unwrap_or(pad_base);
        style.padding = Sides {
            top: Length::Px(pad_v),
            right: Length::Px(pad_h),
            bottom: Length::Px(pad_v),
            left: Length::Px(pad_h),
        };

        if let Some(m) = self.margin {
            style.margin = Sides::all(Length::Px(m));
        }

        if let Some(g) = self.gap {
            style.row_gap = Length::Px(g);
            style.column_gap = Length::Px(g);
        }
        if let Some(rg) = self.row_gap {
            style.row_gap = Length::Px(rg);
        }
        if let Some(cg) = self.column_gap {
            style.column_gap = Length::Px(cg);
        }

        style
    }
}

/// Named table of React Native styles (`StyleSheet.create({ ... })`).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RnStyleSheet {
    /// Style rules indexed by class/slot name.
    pub styles: HashMap<String, RnStyle>,
}

impl RnStyleSheet {
    /// Creates an empty [`RnStyleSheet`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Inserts a named [`RnStyle`] rule into the sheet.
    pub fn insert(&mut self, name: impl Into<String>, style: RnStyle) -> &mut Self {
        self.styles.insert(name.into(), style);
        self
    }

    /// Retrieves a style by name.
    pub fn get(&self, name: &str) -> Option<&RnStyle> {
        self.styles.get(name)
    }
}

/// A React Native view/text node hierarchy for Taffy 0.15 layout computation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RnViewNode {
    /// Node identifier (e.g. `"root"`, `"header"`, `"cta-button"`).
    pub name: String,
    /// React Native layout style.
    pub style: RnStyle,
    /// Optional text content for `<Text>` leaf nodes.
    pub text: Option<String>,
    /// Child `<View>` / `<Text>` nodes.
    #[serde(default)]
    pub children: Vec<RnViewNode>,
}

fn insert_rn_node(tree: &mut LayoutTree, node: &RnViewNode) -> Result<NodeKey, TreeError> {
    let layout_style = node.style.to_layout_style();
    if let Some(text) = &node.text {
        tree.text(&node.name, &layout_style, text)
    } else {
        let mut child_keys = Vec::with_capacity(node.children.len());
        for child in &node.children {
            child_keys.push(insert_rn_node(tree, child)?);
        }
        tree.node(&node.name, &layout_style, &child_keys)
    }
}

/// Computes the exact layout tree of a React Native [`RnViewNode`] hierarchy using Rust Taffy 0.15
/// (`yolo-layout`) instead of C++ Yoga.
pub fn compute_rn_layout(
    root: &RnViewNode,
    viewport_width: f32,
    viewport_height: f32,
) -> Result<NodeLayout, TreeError> {
    let mut tree = LayoutTree::new();
    let mut normalized_root = root.clone();
    if normalized_root.style.flex.unwrap_or(0.0) > 0.0 {
        if normalized_root.style.width.is_none() {
            normalized_root.style.width = Some(viewport_width);
        }
        if normalized_root.style.height.is_none() {
            normalized_root.style.height = Some(viewport_height);
        }
    }
    let root_key = insert_rn_node(&mut tree, &normalized_root)?;
    tree.compute(
        root_key,
        Some(viewport_width),
        Some(viewport_height),
        monospace_measure(8.0, 20.0),
    )?;
    tree.export(root_key)
}

/// Finds a specific node's computed [`LayoutRect`] by name inside a [`NodeLayout`] tree.
pub fn find_layout_rect(layout: &NodeLayout, target_name: &str) -> Option<LayoutRect> {
    if layout.name == target_name {
        return Some(layout.absolute);
    }
    for child in &layout.children {
        if let Some(rect) = find_layout_rect(child, target_name) {
            return Some(rect);
        }
    }
    None
}

// ============================================================================
// Rust -> React Native C++ JSI TurboModule + UniFFI / TypeScript Spec Emitter
// ============================================================================

/// A typed parameter or return type in a Rust TurboModule method.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TurboRustMethod {
    /// Method name in camelCase (`"computeRnLayout"`, `"resolveThemeTokens"`).
    pub name: String,
    /// `(param_name, ts_type, cpp_type)` tuples.
    pub params: Vec<(String, String, String)>,
    /// TypeScript return type (`"string"`, `"number"`, `"boolean"`, `"Object"`).
    pub return_ts: String,
    /// C++ JSI return type (`"jsi::String"`, `"double"`, `"bool"`, `"jsi::Value"`).
    pub return_cpp: String,
}

/// Specification for generating a React Native New Architecture C++ JSI `TurboModule` and TypeScript spec.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TurboRustModuleSpec {
    /// Registered TurboModule name (`"YoloRustBridge"`).
    pub module_name: String,
    /// Backing Rust crate name (`"yolo_react"`).
    pub rust_crate: String,
    /// Exported synchronous/asynchronous JSI methods.
    pub methods: Vec<TurboRustMethod>,
}

/// Output from [`emit_turborust_module`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TurboRustGeneratedFiles {
    /// TypeScript `TurboModuleRegistry.getEnforcing<Spec>(...)` declaration file (`Native<Module>.ts`).
    pub typescript_spec: String,
    /// C++ JSI `facebook::react::TurboModule` header & host function registration (`<Module>JSI.h`).
    pub cpp_jsi_header: String,
}

/// Generates both the TypeScript `TurboModule` spec and C++ JSI `TurboModule` binding header
/// for bridging a Rust crate directly into React Native / Expo New Architecture (`uniffi-bindgen-react-native` compatible).
pub fn emit_turborust_module(spec: &TurboRustModuleSpec) -> TurboRustGeneratedFiles {
    let mut ts_methods = String::new();
    let mut cpp_installs = String::new();

    for m in &spec.methods {
        let ts_params: Vec<String> = m
            .params
            .iter()
            .map(|(n, ts, _)| format!("{n}: {ts}"))
            .collect();
        ts_methods.push_str(&format!(
            "  {}({}): {};\n",
            m.name,
            ts_params.join(", "),
            m.return_ts
        ));

        cpp_installs.push_str(&format!(
            "    methodMap_[\"{name}\"] = MethodMetadata{{{arg_count}, __hostFunction_{module}_{name}}};\n",
            name = m.name,
            arg_count = m.params.len(),
            module = spec.module_name,
        ));
    }

    let typescript_spec = format!(
        "// AUTO-GENERATED by yolo_react::react_native::emit_turborust_module\n\
// Rust Crate: {rust_crate} -> React Native New Architecture TurboModule\n\
import type {{ TurboModule }} from \"react-native\";\n\
import {{ TurboModuleRegistry }} from \"react-native\";\n\n\
export interface Spec extends TurboModule {{\n\
{ts_methods}}}\n\n\
export default TurboModuleRegistry.getEnforcing<Spec>(\"{module}\");\n",
        rust_crate = spec.rust_crate,
        module = spec.module_name,
    );

    let cpp_jsi_header = format!(
        "// AUTO-GENERATED by yolo_react::react_native::emit_turborust_module\n\
#pragma once\n\
#include <ReactCommon/TurboModule.h>\n\
#include <jsi/jsi.h>\n\n\
namespace yolo::react {{\n\
class JSI_EXPORT {module}CxxSpecJSI : public facebook::react::TurboModule {{\n\
public:\n\
  {module}CxxSpecJSI(std::shared_ptr<facebook::react::CallInvoker> jsInvoker)\n\
      : facebook::react::TurboModule(\"{module}\", jsInvoker) {{\n\
{cpp_installs}  }}\n\
}};\n\
}} // namespace yolo::react\n",
        module = spec.module_name,
    );

    TurboRustGeneratedFiles {
        typescript_spec,
        cpp_jsi_header,
    }
}
