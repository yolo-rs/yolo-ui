// SPDX-License-Identifier: Apache-2.0
//! `shadcn_registry`: Shadcn UI (`components.json` + Registry JSON), Base UI (`@base-ui-components/react`
//! / `@base-ui/react`), Radix UI (`@radix-ui/react-*`), `cva()` (`class-variance-authority`), and
//! Tailwind CSS v4 `@theme` parser & Rust SDK generator.
//!
//! Makes Shadcn UI, Base UI, Radix UI, and Tailwind CSS v3/v4 100% Rust-friendly by:
//! 1. Parsing `components.json` and Shadcn Registry item definitions (`ShadcnConfig`, `ShadcnRegistryItem`).
//! 2. Mapping Base UI & Radix UI compound primitives (`Dialog.Root`, `NumberField.ScrubArea`, `Popover.Popup`,
//!    `Tabs.Indicator`, `Switch.Thumb`, etc.) to strongly typed Rust contracts.
//! 3. Extracting `cva("base-classes", { variants: { ... }, defaultVariants: { ... } })` into Rust enums
//!    whose variants resolve directly to `yolo_layout::tailwind::TailwindResolution` (`LayoutStyle` + semantic tokens).
//! 4. Parsing Tailwind CSS v4 `@theme { ... }` custom properties (`--color-*`, `--radius-*`, `--spacing-*`,
//!    `--font-*`) and lowering them to `yolo-layout` Taffy styles and `yolo-shared` theme constants.

use std::collections::BTreeMap;
use std::fmt::Write;

use serde::{Deserialize, Serialize};
use yolo_layout::tailwind::{TailwindResolution, resolve_tailwind};
use yolo_layout::{LayoutStyle, Length};
use yolo_shared::YoloThemePreset;

/// Parsed Shadcn `components.json` project manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShadcnConfig {
    #[serde(rename = "$schema", default)]
    pub schema: Option<String>,
    /// Shadcn style (`"new-york"`, `"default"`, `"yolo-neutral"`, etc.).
    #[serde(default = "default_style")]
    pub style: String,
    #[serde(default)]
    pub rsc: bool,
    #[serde(default = "default_true")]
    pub tsx: bool,
    pub tailwind: ShadcnTailwindConfig,
    pub aliases: ShadcnAliases,
    #[serde(rename = "iconLibrary", default)]
    pub icon_library: Option<String>,
}

fn default_style() -> String {
    "new-york".to_string()
}

const fn default_true() -> bool {
    true
}

/// `tailwind` section of `components.json`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ShadcnTailwindConfig {
    #[serde(default)]
    pub config: String,
    pub css: String,
    #[serde(rename = "baseColor", default = "default_base_color")]
    pub base_color: String,
    #[serde(rename = "cssVariables", default = "default_true")]
    pub css_variables: bool,
    #[serde(default)]
    pub prefix: String,
}

fn default_base_color() -> String {
    "zinc".to_string()
}

/// `aliases` section of `components.json`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ShadcnAliases {
    pub components: String,
    pub utils: String,
    #[serde(default)]
    pub ui: Option<String>,
    #[serde(default)]
    pub lib: Option<String>,
    #[serde(default)]
    pub hooks: Option<String>,
}

impl ShadcnConfig {
    /// Parses a Shadcn `components.json` string.
    pub fn parse_json(json: &str) -> Result<Self, String> {
        serde_json::from_str(json).map_err(|e| format!("invalid components.json: {e}"))
    }

    /// Maps the Shadcn `baseColor` / `style` to the closest [`YoloThemePreset`].
    #[must_use]
    pub fn recommended_yolo_preset(&self) -> YoloThemePreset {
        match self.style.to_ascii_lowercase().as_str() {
            s if s.contains("m3") || s.contains("material") => YoloThemePreset::M3,
            s if s.contains("fluent") || s.contains("winui") => YoloThemePreset::Fluent,
            s if s.contains("apple") || s.contains("cupertino") || s.contains("glass") => {
                YoloThemePreset::Apple
            }
            s if s.contains("omarchy") || s.contains("tokyo") => YoloThemePreset::Omarchy,
            s if s.contains("hypr") => YoloThemePreset::Hyprland,
            s if s.contains("aphrody") || s.contains("byron") => YoloThemePreset::Aphrody,
            _ => YoloThemePreset::Neutral,
        }
    }
}

/// A Shadcn Registry item (`registry:ui`, `registry:block`, `registry:hook`, `registry:theme`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShadcnRegistryItem {
    pub name: String,
    #[serde(rename = "type")]
    pub item_type: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub dependencies: Vec<String>,
    #[serde(rename = "registryDependencies", default)]
    pub registry_dependencies: Vec<String>,
    #[serde(default)]
    pub files: Vec<ShadcnRegistryFile>,
}

/// A file entry inside a [`ShadcnRegistryItem`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShadcnRegistryFile {
    pub path: String,
    #[serde(rename = "type", default)]
    pub file_type: String,
    #[serde(default)]
    pub content: Option<String>,
}

impl ShadcnRegistryItem {
    /// Parses a Shadcn registry item JSON document.
    pub fn parse_json(json: &str) -> Result<Self, String> {
        serde_json::from_str(json).map_err(|e| format!("invalid registry item JSON: {e}"))
    }
}

/// Supported headless primitive ecosystems mapped into `yolo-migrator`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HeadlessEcosystem {
    /// `@radix-ui/react-*` or `radix-ui` unified package.
    RadixUi,
    /// `@base-ui-components/react` or `@base-ui/react`.
    BaseUi,
    /// `@aphrody/m3-baseui` or `@material/web`.
    Material3,
    /// Shadcn UI (`@/components/ui/*`).
    ShadcnUi,
}

/// Descriptor for a Base UI or Radix UI primitive import (`@base-ui-components/react/dialog`, `@radix-ui/react-tabs`, etc.).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PrimitiveDescriptor {
    pub ecosystem: HeadlessEcosystem,
    pub package: String,
    pub component: String,
    pub rust_widget: &'static str,
    pub parts: &'static [&'static str],
}

/// Resolves a module specifier (`@base-ui-components/react/dialog`, `@base-ui/react/number-field`,
/// `@radix-ui/react-popover`, `@/components/ui/button`) into a [`PrimitiveDescriptor`].
#[must_use]
pub fn resolve_primitive_import(specifier: &str) -> Option<PrimitiveDescriptor> {
    let spec = specifier.trim();
    let (ecosystem, raw_name) = if let Some(rest) = spec
        .strip_prefix("@base-ui-components/react/")
        .or_else(|| spec.strip_prefix("@base-ui/react/"))
    {
        (HeadlessEcosystem::BaseUi, rest)
    } else if let Some(rest) = spec.strip_prefix("@radix-ui/react-") {
        (HeadlessEcosystem::RadixUi, rest)
    } else if let Some(rest) = spec
        .strip_prefix("@/components/ui/")
        .or_else(|| spec.strip_prefix("~/components/ui/"))
    {
        (HeadlessEcosystem::ShadcnUi, rest)
    } else if spec.starts_with("@aphrody/m3-baseui") || spec.starts_with("@material/web") {
        (HeadlessEcosystem::Material3, "m3")
    } else {
        return None;
    };

    let (rust_widget, parts): (&'static str, &'static [&'static str]) = match raw_name {
        "button" => ("Button", &["Root"]),
        "dialog" | "alert-dialog" | "sheet" | "drawer" => (
            "Dialog",
            &[
                "Root",
                "Trigger",
                "Portal",
                "Backdrop",
                "Overlay",
                "Popup",
                "Content",
                "Title",
                "Description",
                "Close",
                "Header",
                "Footer",
            ],
        ),
        "tabs" => ("Tabs", &["Root", "List", "Tab", "Trigger", "Indicator", "Panel", "Content"]),
        "switch" => ("Switch", &["Root", "Thumb"]),
        "checkbox" | "checkbox-group" => ("Checkbox", &["Root", "Indicator"]),
        "radio" | "radio-group" => ("Radio", &["Root", "Item", "Indicator"]),
        "slider" => ("Slider", &["Root", "Control", "Track", "Indicator", "Range", "Thumb"]),
        "progress" => ("Progress", &["Root", "Track", "Indicator"]),
        "input" | "textarea" | "field" | "number-field" => (
            "TextField",
            &["Root", "Label", "Control", "Input", "ScrubArea", "Decrement", "Increment", "Description", "Error"],
        ),
        "select" | "combobox" | "dropdown-menu" | "menu" => (
            "List",
            &["Root", "Trigger", "Value", "Portal", "Positioner", "Popup", "Content", "List", "Item", "ItemText", "ItemIndicator"],
        ),
        "popover" | "tooltip" | "preview-card" | "hover-card" => (
            "Card",
            &["Root", "Trigger", "Portal", "Positioner", "Popup", "Content", "Arrow"],
        ),
        "accordion" | "collapsible" => (
            "Card",
            &["Root", "Item", "Header", "Trigger", "Panel", "Content"],
        ),
        "separator" | "divider" => ("Divider", &["Root"]),
        "avatar" => ("Avatar", &["Root", "Image", "Fallback"]),
        "scroll-area" => ("ScrollArea", &["Root", "Viewport", "Scrollbar", "Thumb", "Corner"]),
        "card" | "alert" => ("Card", &["Root", "Header", "Title", "Description", "Content", "Footer"]),
        "badge" | "chip" | "toggle" | "toggle-group" => ("Chip", &["Root", "Item"]),
        _ => ("Box", &["Root"]),
    };

    Some(PrimitiveDescriptor {
        ecosystem,
        package: spec.to_string(),
        component: pascal_ident(raw_name),
        rust_widget,
        parts,
    })
}

/// Parsed `cva("base classes", { variants: { ... }, defaultVariants: { ... } })` definition.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct CvaSpec {
    /// Variable name in TS/TSX (e.g. `buttonVariants`).
    pub name: String,
    /// Base Tailwind utility classes applied to every variant.
    pub base_classes: String,
    /// Variant axes (e.g. `"variant" -> {"default": "bg-primary ...", "outline": "border ..."}`).
    pub variants: BTreeMap<String, BTreeMap<String, String>>,
    /// Default variant value per axis (e.g. `"variant" -> "default"`, `"size" -> "default"`).
    pub default_variants: BTreeMap<String, String>,
}

impl CvaSpec {
    /// Resolves the combined Tailwind utility string for a given set of variant choices.
    #[must_use]
    pub fn resolve_classes(&self, choices: &[(&str, &str)]) -> String {
        let mut classes = Vec::new();
        if !self.base_classes.trim().is_empty() {
            classes.push(self.base_classes.trim().to_string());
        }
        for (axis, map) in &self.variants {
            let chosen = choices
                .iter()
                .find(|(k, _)| *k == axis.as_str())
                .map(|(_, v)| *v)
                .or_else(|| self.default_variants.get(axis).map(String::as_str));
            if let Some(val) = chosen
                && let Some(cls) = map.get(val)
                && !cls.trim().is_empty()
            {
                classes.push(cls.trim().to_string());
            }
        }
        classes.join(" ")
    }

    /// Resolves the [`TailwindResolution`] (`yolo_layout::LayoutStyle` + semantic tokens) for `choices`.
    #[must_use]
    pub fn resolve_layout(&self, choices: &[(&str, &str)]) -> TailwindResolution {
        resolve_tailwind(&self.resolve_classes(choices))
    }

    /// Emits strongly typed Rust enums (`#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]`)
    /// and a `resolve_<name>_style` helper backed by `yolo_layout::tailwind::resolve_tailwind`.
    #[must_use]
    pub fn emit_rust_enums(&self) -> String {
        let prefix = pascal_ident(self.name.strip_suffix("Variants").unwrap_or(&self.name));
        let mut out = String::new();
        let _ = writeln!(out, "// Generated by yolo-migrator shadcn_registry from `{}`", self.name);
        let _ = writeln!(out, "pub const {}_BASE_CLASSES: &str = {:?};\n", snake_ident(&prefix).to_ascii_uppercase(), self.base_classes);

        for (axis, options) in &self.variants {
            let enum_name = format!("{prefix}{}", pascal_ident(axis));
            let default_opt = self
                .default_variants
                .get(axis)
                .or_else(|| options.keys().next())
                .cloned()
                .unwrap_or_default();

            let _ = writeln!(out, "#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]");
            let _ = writeln!(out, "pub enum {enum_name} {{");
            for key in options.keys() {
                let vname = pascal_ident(key);
                if *key == default_opt {
                    let _ = writeln!(out, "    #[default]");
                }
                let _ = writeln!(out, "    {vname},");
            }
            let _ = writeln!(out, "}}\n");

            let _ = writeln!(out, "impl {enum_name} {{");
            let _ = writeln!(out, "    #[must_use]");
            let _ = writeln!(out, "    pub const fn classes(self) -> &'static str {{");
            let _ = writeln!(out, "        match self {{");
            for (key, cls) in options {
                let vname = pascal_ident(key);
                let _ = writeln!(out, "            Self::{vname} => {cls:?},");
            }
            let _ = writeln!(out, "        }}");
            let _ = writeln!(out, "    }}");
            let _ = writeln!(out, "}}\n");
        }
        out
    }
}

/// Extracts `cva(...)` specifications from a TypeScript / TSX source file (such as Shadcn's `button.tsx` or `badge.tsx`).
#[must_use]
pub fn extract_cva_specs(source: &str) -> Vec<CvaSpec> {
    let mut specs = Vec::new();
    let mut search_from = 0;
    while let Some(rel_idx) = source[search_from..].find("cva(") {
        let call_start = search_from + rel_idx;
        // Look backwards on the same statement for `const <name> = cva(`
        let prefix_slice = &source[..call_start];
        let name = prefix_slice
            .rsplit_once("const ")
            .and_then(|(_, after)| after.split('=').next())
            .map(|s| s.split(':').next().unwrap_or(s).trim().to_string())
            .filter(|s| !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'))
            .unwrap_or_else(|| "componentVariants".to_string());

        let args_start = call_start + 4;
        let Some(call_body) = balanced_slice(&source[args_start..], '(', ')') else {
            search_from = args_start;
            continue;
        };
        search_from = args_start + call_body.len();

        let (base_classes, rest) = extract_first_string_literal(call_body).unwrap_or_default();
        let mut variants = BTreeMap::new();
        let mut default_variants = BTreeMap::new();

        if let Some(v_idx) = rest.find("variants")
            && let Some(brace_off) = rest[v_idx..].find('{')
            && let Some(v_block) = balanced_slice(&rest[v_idx + brace_off + 1..], '{', '}')
        {
            variants = parse_nested_string_map(v_block);
        }
        if let Some(dv_idx) = rest.find("defaultVariants")
            && let Some(brace_off) = rest[dv_idx..].find('{')
            && let Some(dv_block) = balanced_slice(&rest[dv_idx + brace_off + 1..], '{', '}')
        {
            default_variants = parse_flat_string_map(dv_block);
        }

        specs.push(CvaSpec {
            name,
            base_classes,
            variants,
            default_variants,
        });
    }
    specs
}

fn balanced_slice(s: &str, open: char, close: char) -> Option<&str> {
    let mut depth = 1usize;
    let mut in_str: Option<char> = None;
    let mut escaped = false;
    for (i, ch) in s.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' && in_str.is_some() {
            escaped = true;
            continue;
        }
        if let Some(q) = in_str {
            if ch == q {
                in_str = None;
            }
            continue;
        }
        if matches!(ch, '"' | '\'' | '`') {
            in_str = Some(ch);
            continue;
        }
        if ch == open {
            depth += 1;
        } else if ch == close {
            depth -= 1;
            if depth == 0 {
                return Some(&s[..i]);
            }
        }
    }
    None
}

fn extract_first_string_literal(s: &str) -> Option<(String, &str)> {
    let trimmed = s.trim_start();
    let quote = trimmed.chars().next()?;
    if !matches!(quote, '"' | '\'' | '`') {
        return None;
    }
    let after = &trimmed[quote.len_utf8()..];
    let end = after.find(quote)?;
    Some((after[..end].to_string(), &after[end + quote.len_utf8()..]))
}

fn parse_flat_string_map(block: &str) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    let mut rest = block;
    while let Some(colon) = rest.find(':') {
        let key_part = rest[..colon].trim().trim_start_matches(',').trim();
        let key = key_part.trim_matches(|c| c == '"' || c == '\'').to_string();
        let after_colon = rest[colon + 1..].trim_start();
        if let Some((val, tail)) = extract_first_string_literal(after_colon) {
            if !key.is_empty() {
                map.insert(key, val);
            }
            rest = tail;
        } else {
            break;
        }
    }
    map
}

fn parse_nested_string_map(block: &str) -> BTreeMap<String, BTreeMap<String, String>> {
    let mut out = BTreeMap::new();
    let mut rest = block;
    while let Some(colon) = rest.find(':') {
        let key_part = rest[..colon].trim().trim_start_matches(',').trim();
        let key = key_part.trim_matches(|c| c == '"' || c == '\'').to_string();
        let after_colon = &rest[colon + 1..];
        let Some(open_brace) = after_colon.find('{') else {
            break;
        };
        let inner_start = &after_colon[open_brace + 1..];
        let Some(inner) = balanced_slice(inner_start, '{', '}') else {
            break;
        };
        if !key.is_empty() {
            out.insert(key, parse_flat_string_map(inner));
        }
        rest = &inner_start[inner.len() + 1..];
    }
    out
}

/// Parsed Tailwind CSS v4 `@theme { ... }` block.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct TailwindV4Theme {
    /// `--color-*` variables (e.g. `"primary" -> "oklch(0.62 0.19 260)"` or `"#6366f1"`).
    pub colors: BTreeMap<String, String>,
    /// `--radius-*` variables in pixels (e.g. `"lg" -> 12.0`).
    pub radii_px: BTreeMap<String, f32>,
    /// `--spacing-*` variables in pixels (e.g. `"4" -> 16.0`).
    pub spacing_px: BTreeMap<String, f32>,
    /// `--font-*` font stacks (e.g. `"sans" -> "'Inter', sans-serif"`).
    pub fonts: BTreeMap<String, String>,
}

impl TailwindV4Theme {
    /// Extracts all `@theme { ... }` custom properties from a Tailwind CSS v4 stylesheet.
    #[must_use]
    pub fn parse_css(css: &str) -> Self {
        let mut theme = Self::default();
        let mut search = 0;
        while let Some(idx) = css[search..].find("@theme") {
            let after = &css[search + idx + 6..];
            let Some(brace) = after.find('{') else {
                break;
            };
            let body_start = &after[brace + 1..];
            let Some(body) = balanced_slice(body_start, '{', '}') else {
                break;
            };
            search += idx + 6 + brace + 1 + body.len();

            for decl in body.split(';') {
                let Some((raw_k, raw_v)) = decl.split_once(':') else {
                    continue;
                };
                let k = raw_k.trim();
                let v = raw_v.trim();
                if let Some(color_name) = k.strip_prefix("--color-") {
                    theme.colors.insert(color_name.to_string(), v.to_string());
                } else if let Some(radius_name) = k.strip_prefix("--radius-") {
                    if let Some(px) = parse_css_px(v) {
                        theme.radii_px.insert(radius_name.to_string(), px);
                    }
                } else if let Some(space_name) = k.strip_prefix("--spacing-") {
                    if let Some(px) = parse_css_px(v) {
                        theme.spacing_px.insert(space_name.to_string(), px);
                    }
                } else if let Some(font_name) = k.strip_prefix("--font-") {
                    theme.fonts.insert(font_name.to_string(), v.to_string());
                }
            }
        }
        theme
    }

    /// Converts a spacing key from this `@theme` into a `yolo_layout::LayoutStyle` padding or gap.
    #[must_use]
    pub fn to_layout_style(&self, spacing_key: Option<&str>) -> LayoutStyle {
        let mut style = LayoutStyle::default();
        if let Some(k) = spacing_key
            && let Some(&px) = self.spacing_px.get(k)
        {
            style.row_gap = Length::Px(px);
            style.column_gap = Length::Px(px);
        }
        style
    }

    /// Emits a Rust module with typed constants for all `@theme` tokens.
    #[must_use]
    pub fn emit_rust_tokens(&self) -> String {
        let mut out = String::from("// Generated by yolo-migrator from Tailwind CSS v4 `@theme`.\n\n");
        for (k, v) in &self.colors {
            let ident = snake_ident(k).to_ascii_uppercase();
            let _ = writeln!(out, "pub const COLOR_{ident}: &str = {v:?};");
        }
        for (k, v) in &self.radii_px {
            let ident = snake_ident(k).to_ascii_uppercase();
            let _ = writeln!(out, "pub const RADIUS_{ident}_PX: f32 = {v:.1};");
        }
        for (k, v) in &self.spacing_px {
            let ident = snake_ident(k).to_ascii_uppercase();
            let _ = writeln!(out, "pub const SPACING_{ident}_PX: f32 = {v:.1};");
        }
        for (k, v) in &self.fonts {
            let ident = snake_ident(k).to_ascii_uppercase();
            let _ = writeln!(out, "pub const FONT_{ident}: &str = {v:?};");
        }
        out
    }
}

fn parse_css_px(val: &str) -> Option<f32> {
    let v = val.trim();
    if let Some(px) = v.strip_suffix("px") {
        px.trim().parse::<f32>().ok()
    } else if let Some(rem) = v.strip_suffix("rem") {
        rem.trim().parse::<f32>().ok().map(|r| r * 16.0)
    } else {
        v.parse::<f32>().ok()
    }
}

fn pascal_ident(s: &str) -> String {
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
    if out.is_empty() || out.starts_with(|c: char| c.is_ascii_digit()) {
        format!("V{out}")
    } else {
        out
    }
}

fn snake_ident(s: &str) -> String {
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() {
        if ch.is_ascii_uppercase() {
            if i > 0 && !out.ends_with('_') {
                out.push('_');
            }
            out.push(ch.to_ascii_lowercase());
        } else if ch.is_ascii_alphanumeric() {
            out.push(ch);
        } else if !out.ends_with('_') {
            out.push('_');
        }
    }
    out.trim_matches('_').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_shadcn_components_json_and_recommends_preset() {
        let raw = r#"{
            "$schema": "https://ui.shadcn.com/schema.json",
            "style": "new-york",
            "rsc": true,
            "tsx": true,
            "tailwind": {
                "config": "tailwind.config.ts",
                "css": "app/globals.css",
                "baseColor": "zinc",
                "cssVariables": true
            },
            "aliases": {
                "components": "@/components",
                "utils": "@/lib/utils",
                "ui": "@/components/ui"
            },
            "iconLibrary": "lucide"
        }"#;
        let cfg = ShadcnConfig::parse_json(raw).expect("valid components.json");
        assert_eq!(cfg.style, "new-york");
        assert_eq!(cfg.tailwind.base_color, "zinc");
        assert_eq!(cfg.recommended_yolo_preset(), YoloThemePreset::Neutral);
    }

    #[test]
    fn extracts_cva_variants_and_emits_rust_enums() {
        let button_tsx = r#"
const buttonVariants = cva(
  "inline-flex items-center justify-center gap-2 rounded-md text-sm font-medium",
  {
    variants: {
      variant: {
        default: "bg-primary text-primary-foreground shadow-sm",
        destructive: "bg-destructive text-destructive-foreground",
        outline: "border border-input bg-background",
        secondary: "bg-secondary text-secondary-foreground",
        ghost: "bg-transparent",
      },
      size: {
        default: "h-9 px-4 py-2",
        sm: "h-8 rounded-md px-3 text-xs",
        lg: "h-10 rounded-md px-8",
      },
    },
    defaultVariants: {
      variant: "default",
      size: "default",
    },
  }
)
"#;
        let specs = extract_cva_specs(button_tsx);
        assert_eq!(specs.len(), 1);
        let spec = &specs[0];
        assert_eq!(spec.name, "buttonVariants");
        assert_eq!(spec.variants.len(), 2);

        let resolved = spec.resolve_layout(&[("variant", "outline"), ("size", "lg")]);
        assert!(resolved.unknown_classes.is_empty(), "unknown: {:?}", resolved.unknown_classes);
        assert_eq!(resolved.border_width_px, Some(1.0));
        assert_eq!(resolved.layout.height, Length::Px(40.0));

        let rust_code = spec.emit_rust_enums();
        assert!(rust_code.contains("pub enum ButtonVariant"));
        assert!(rust_code.contains("pub enum ButtonSize"));
        assert!(rust_code.contains("Self::Outline =>"));
    }

    #[test]
    fn resolves_base_ui_and_radix_imports() {
        let base_dialog = resolve_primitive_import("@base-ui-components/react/dialog").expect("base-ui dialog");
        assert_eq!(base_dialog.ecosystem, HeadlessEcosystem::BaseUi);
        assert_eq!(base_dialog.rust_widget, "Dialog");
        assert!(base_dialog.parts.contains(&"Popup"));

        let radix_tabs = resolve_primitive_import("@radix-ui/react-tabs").expect("radix tabs");
        assert_eq!(radix_tabs.ecosystem, HeadlessEcosystem::RadixUi);
        assert_eq!(radix_tabs.rust_widget, "Tabs");
    }

    #[test]
    fn parses_tailwind_v4_at_theme_block() {
        let css = r#"
@import "tailwindcss";

@theme {
  --color-primary: #6366f1;
  --color-surface: #18181b;
  --radius-lg: 0.75rem;
  --spacing-4: 16px;
  --font-sans: 'Inter', system-ui, sans-serif;
}
"#;
        let theme = TailwindV4Theme::parse_css(css);
        assert_eq!(theme.colors.get("primary").map(String::as_str), Some("#6366f1"));
        assert_eq!(theme.radii_px.get("lg"), Some(&12.0));
        assert_eq!(theme.spacing_px.get("4"), Some(&16.0));
        let rust_tokens = theme.emit_rust_tokens();
        assert!(rust_tokens.contains("pub const COLOR_PRIMARY: &str = \"#6366f1\";"));
        assert!(rust_tokens.contains("pub const RADIUS_LG_PX: f32 = 12.0;"));
    }
}
