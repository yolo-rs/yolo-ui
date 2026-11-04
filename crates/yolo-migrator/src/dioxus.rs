// SPDX-License-Identifier: Apache-2.0
//! Dioxus 0.8 `rsx!` fullstack / WebGPU / WASM emitter (`MigrationTarget::Dioxus`).
//!
//! Emits an idiomatic `#[component] pub fn ComponentName(...) -> Element` function using
//! `dioxus::prelude::*` and `yolo_shared::{YoloThemePreset, YoloThemeTokens}` (`yolo-neutral`,
//! `yolo-m3`, `yolo-fluent`, `yolo-apple`, `yolo-aphrody`).

use std::fmt::Write;

use yolo_shared::YoloThemePreset;

use crate::ir::{Align, Comp, Dir, Document, Justify, Kind, Node, TextTag, shape_radius, snake_handler, typescale_metrics};

struct DioxusEmitter {
    out: String,
    theme: YoloThemePreset,
}

fn lit(s: &str) -> String {
    format!("{s:?}")
}

fn fmt_px(n: f64) -> String {
    if n.fract() == 0.0 {
        format!("{}px", n as i64)
    } else {
        format!("{n}px")
    }
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
        "MigratedComponent".to_string()
    } else {
        out
    }
}

fn preset_variant(theme: YoloThemePreset) -> &'static str {
    match theme {
        YoloThemePreset::Neutral => "YoloThemePreset::Neutral",
        YoloThemePreset::M3 => "YoloThemePreset::M3",
        YoloThemePreset::Fluent => "YoloThemePreset::Fluent",
        YoloThemePreset::Apple => "YoloThemePreset::Apple",
        YoloThemePreset::Omarchy => "YoloThemePreset::Omarchy",
        YoloThemePreset::Hyprland => "YoloThemePreset::Hyprland",
        YoloThemePreset::Aphrody => "YoloThemePreset::Aphrody",
    }
}

fn role_to_css_var(role: &str, theme: YoloThemePreset) -> String {
    let tokens = theme.tokens(true);
    match role {
        "primary" | "surface-tint" => tokens.accent_primary.to_string(),
        "secondary" | "tertiary" => tokens.accent_secondary.to_string(),
        "background" => tokens.bg_primary.to_string(),
        "surface" | "surface-container-low" | "surface-container" => tokens.bg_surface.to_string(),
        "surface-container-high" | "surface-container-highest" | "primary-container" | "secondary-container" => {
            tokens.bg_elevated.to_string()
        }
        "on-surface" | "on-background" | "on-primary" | "on-primary-container" => tokens.text_primary.to_string(),
        "on-surface-variant" => tokens.text_secondary.to_string(),
        "outline" | "outline-variant" => tokens.border_subtle.to_string(),
        "error" => "#ef4444".to_string(),
        other => format!("var(--m3-{other})"),
    }
}

impl DioxusEmitter {
    fn line(&mut self, ind: usize, s: &str) {
        let _ = writeln!(self.out, "{:ind$}{s}", "", ind = ind * 4);
    }

    fn style_attr(&self, n: &Node, extra_css: &str) -> Option<String> {
        let mut parts = Vec::new();
        let s = &n.style;
        if s.display_grid {
            parts.push("display: grid".to_string());
            if let Some(cols) = s.grid_cols {
                parts.push(format!("grid-template-columns: repeat({cols}, minmax(0, 1fr))"));
            }
        } else if s.display_flex || s.direction.is_some() || matches!(n.kind, Kind::Box | Kind::Comp(Comp::Card)) {
            parts.push("display: flex".to_string());
            let dir = match n.dir() {
                Dir::Row => "row",
                Dir::Column => "column",
            };
            parts.push(format!("flex-direction: {dir}"));
        }
        if let Some(a) = s.align {
            let v = match a {
                Align::Start => "flex-start",
                Align::Center => "center",
                Align::End => "flex-end",
                Align::Stretch => "stretch",
            };
            parts.push(format!("align-items: {v}"));
        }
        if let Some(j) = s.justify {
            let v = match j {
                Justify::Start => "flex-start",
                Justify::Center => "center",
                Justify::End => "flex-end",
                Justify::SpaceBetween => "space-between",
            };
            parts.push(format!("justify-content: {v}"));
        }
        if let Some(g) = s.gap {
            parts.push(format!("gap: {}", fmt_px(g)));
        }
        if let Some(p) = s.padding {
            if p.iter().all(|v| *v == p[0]) {
                parts.push(format!("padding: {}", fmt_px(p[0])));
            } else {
                parts.push(format!(
                    "padding: {} {} {} {}",
                    fmt_px(p[0]),
                    fmt_px(p[1]),
                    fmt_px(p[2]),
                    fmt_px(p[3])
                ));
            }
        }
        if let Some(m) = s.margin {
            if m.iter().all(|v| *v == m[0]) {
                parts.push(format!("margin: {}", fmt_px(m[0])));
            } else {
                parts.push(format!(
                    "margin: {} {} {} {}",
                    fmt_px(m[0]),
                    fmt_px(m[1]),
                    fmt_px(m[2]),
                    fmt_px(m[3])
                ));
            }
        }
        if s.grow {
            parts.push("flex-grow: 1".to_string());
        }
        if s.fill_width {
            parts.push("width: 100%".to_string());
        } else if let Some(w) = s.width {
            parts.push(format!("width: {}", fmt_px(w)));
        }
        if s.fill_height {
            parts.push("height: 100%".to_string());
        } else if let Some(h) = s.height {
            parts.push(format!("height: {}", fmt_px(h)));
        }
        if let Some(mw) = s.max_width {
            parts.push(format!("max-width: {}", fmt_px(mw)));
        }
        if s.scroll {
            parts.push("overflow-y: auto".to_string());
        }
        if let Some(bg) = &s.background {
            parts.push(format!("background: {}", role_to_css_var(bg, self.theme)));
        }
        if let Some(fg) = &s.color {
            parts.push(format!("color: {}", role_to_css_var(fg, self.theme)));
        }
        if let Some(bw) = s.border_width {
            let bc = s
                .border_color
                .as_deref()
                .map(|c| role_to_css_var(c, self.theme))
                .unwrap_or_else(|| self.theme.tokens(true).border_subtle.to_string());
            parts.push(format!("border: {} solid {bc}", fmt_px(bw)));
        }
        if let Some(sh) = &s.shape
            && let Some(r) = shape_radius(sh)
        {
            parts.push(format!("border-radius: {}", fmt_px(r)));
        }
        if let Some(ts) = &s.typescale
            && let Some((sz, wt)) = typescale_metrics(ts)
        {
            parts.push(format!("font-size: {}", fmt_px(sz)));
            parts.push(format!("font-weight: {wt}"));
        }
        if !extra_css.is_empty() {
            parts.push(extra_css.to_string());
        }
        if parts.is_empty() {
            None
        } else {
            Some(parts.join("; "))
        }
    }

    fn emit_common_attrs(&mut self, ind: usize, n: &Node, extra_css: &str) {
        if let Some(id) = &n.id {
            self.line(ind, &format!("id: {},", lit(id)));
        }
        if !n.classes.is_empty() {
            self.line(ind, &format!("class: {},", lit(&n.classes.join(" "))));
        }
        if let Some(a) = &n.aria_label {
            self.line(ind, &format!("aria_label: {},", lit(a)));
        }
        if let Some(st) = self.style_attr(n, extra_css) {
            self.line(ind, &format!("style: {},", lit(&st)));
        }
    }

    fn node(&mut self, ind: usize, n: &Node) {
        match n.kind {
            Kind::Box => {
                let tag = match n.tag_name.as_str() {
                    "section" | "main" | "header" | "footer" | "nav" | "aside" | "article" | "form" => n.tag_name.as_str(),
                    _ => "div",
                };
                self.line(ind, &format!("{tag} {{"));
                self.emit_common_attrs(ind + 1, n, "");
                if let Some(t) = &n.text {
                    self.line(ind + 1, &lit(t));
                }
                for child in &n.children {
                    self.node(ind + 1, child);
                }
                self.line(ind, "}");
            }
            Kind::Text(tag) => {
                let el = match tag {
                    TextTag::Heading(1) => "h1",
                    TextTag::Heading(2) => "h2",
                    TextTag::Heading(3) => "h3",
                    TextTag::Heading(4) => "h4",
                    TextTag::Heading(5) => "h5",
                    TextTag::Heading(_) => "h6",
                    TextTag::Paragraph => "p",
                    TextTag::Span => "span",
                    TextTag::Label => "label",
                };
                self.line(ind, &format!("{el} {{"));
                self.emit_common_attrs(ind + 1, n, "");
                if let Some(t) = &n.text {
                    self.line(ind + 1, &lit(t));
                }
                for child in &n.children {
                    self.node(ind + 1, child);
                }
                self.line(ind, "}");
            }
            Kind::Image => {
                self.line(ind, "img {");
                self.emit_common_attrs(ind + 1, n, "");
                if let Some(src) = n.str("src") {
                    self.line(ind + 1, &format!("src: {},", lit(src)));
                }
                if let Some(alt) = n.str("alt") {
                    self.line(ind + 1, &format!("alt: {},", lit(alt)));
                }
                self.line(ind, "}");
            }
            Kind::Comp(c) => self.comp(ind, n, c),
        }
    }

    fn comp(&mut self, ind: usize, n: &Node, c: Comp) {
        let tokens = self.theme.tokens(true);
        let style_preset = self.theme.style();
        match c {
            Comp::Button | Comp::Fab | Comp::IconButton | Comp::Chip => {
                self.line(ind, "button {");
                self.line(ind + 1, &format!("r#type: {},", lit(n.str("type").unwrap_or("button"))));
                if n.flag("disabled") {
                    self.line(ind + 1, "disabled: true,");
                }
                let default_css = format!(
                    "background: {}; color: {}; border-radius: {}px; border: 1px solid {}; cursor: pointer",
                    tokens.accent_primary, tokens.text_primary, style_preset.radius_md_px, tokens.border_subtle
                );
                self.emit_common_attrs(ind + 1, n, &default_css);
                if let Some(ev) = n.event(&["onClick"]) {
                    let h = snake_handler(&ev.handler);
                    self.line(ind + 1, &format!("onclick: move |evt| if let Some(cb) = &props.{h} {{ cb.call(evt); }},"));
                }
                if let Some(lbl) = n.label(&["label"]).or(n.str("icon")) {
                    self.line(ind + 1, &lit(lbl));
                }
                for child in &n.children {
                    self.node(ind + 1, child);
                }
                self.line(ind, "}");
            }
            Comp::TextField => {
                let is_textarea = n.tag_name.eq_ignore_ascii_case("textarea");
                if let Some(l) = n.str("label") {
                    self.line(ind, &format!("label {{ {} }}", lit(l)));
                }
                let el = if is_textarea { "textarea" } else { "input" };
                self.line(ind, &format!("{el} {{"));
                if !is_textarea {
                    self.line(ind + 1, &format!("r#type: {},", lit(n.str("type").unwrap_or("text"))));
                }
                if let Some(p) = n.str("placeholder") {
                    self.line(ind + 1, &format!("placeholder: {},", lit(p)));
                }
                if let Some(v) = n.str("value") {
                    self.line(ind + 1, &format!("value: {},", lit(v)));
                }
                if n.flag("disabled") {
                    self.line(ind + 1, "disabled: true,");
                }
                let default_css = format!(
                    "background: {}; color: {}; border-radius: {}px; border: 1px solid {}; padding: 8px 12px",
                    tokens.bg_surface, tokens.text_primary, style_preset.radius_sm_px, tokens.border_subtle
                );
                self.emit_common_attrs(ind + 1, n, &default_css);
                if let Some(ev) = n.event(&["onChange", "onInput", "onValueChange"]) {
                    let h = snake_handler(&ev.handler);
                    self.line(ind + 1, &format!("oninput: move |evt| if let Some(cb) = &props.{h} {{ cb.call(evt); }},"));
                }
                self.line(ind, "}");
            }
            Comp::Switch | Comp::Checkbox | Comp::Radio => {
                let input_ty = match c {
                    Comp::Radio => "radio",
                    _ => "checkbox",
                };
                self.line(ind, "label {");
                self.emit_common_attrs(ind + 1, n, "display: inline-flex; align-items: center; gap: 8px");
                self.line(ind + 1, "input {");
                self.line(ind + 2, &format!("r#type: {},", lit(input_ty)));
                if n.flag("checked") || n.flag("selected") {
                    self.line(ind + 2, "checked: true,");
                }
                if n.flag("disabled") {
                    self.line(ind + 2, "disabled: true,");
                }
                self.line(ind + 1, "}");
                if let Some(l) = n.label(&["label"]) {
                    self.line(ind + 1, &format!("span {{ {} }}", lit(l)));
                }
                self.line(ind, "}");
            }
            Comp::Slider => {
                self.line(ind, "input {");
                self.line(ind + 1, "r#type: \"range\",");
                self.line(ind + 1, &format!("min: {},", lit(&n.num("min").unwrap_or(0.0).to_string())));
                self.line(ind + 1, &format!("max: {},", lit(&n.num("max").unwrap_or(100.0).to_string())));
                self.line(ind + 1, &format!("value: {},", lit(&n.num("value").unwrap_or(0.0).to_string())));
                self.emit_common_attrs(ind + 1, n, "");
                self.line(ind, "}");
            }
            Comp::Progress => {
                self.line(ind, "progress {");
                if let Some(f) = n.progress_fraction() {
                    self.line(ind + 1, &format!("value: \"{:.3}\",", f));
                    self.line(ind + 1, "max: \"1.0\",");
                }
                self.emit_common_attrs(ind + 1, n, "");
                self.line(ind, "}");
            }
            Comp::Divider => {
                self.line(ind, "hr {");
                self.emit_common_attrs(ind + 1, n, &format!("border-color: {}", tokens.border_subtle));
                self.line(ind, "}");
            }
            Comp::Card | Comp::Dialog | Comp::Tabs | Comp::Tab | Comp::List | Comp::ListItem | Comp::TopAppBar | Comp::NavigationRail | Comp::NavigationRailItem => {
                let tag = match c {
                    Comp::Dialog => "dialog",
                    Comp::List => "ul",
                    Comp::ListItem => "li",
                    Comp::TopAppBar => "header",
                    Comp::NavigationRail => "nav",
                    _ => "div",
                };
                self.line(ind, &format!("{tag} {{"));
                let extra = if c == Comp::Card {
                    format!(
                        "background: {}; color: {}; border-radius: {}px; border: 1px solid {}",
                        tokens.bg_surface, tokens.text_primary, style_preset.radius_lg_px, tokens.border_subtle
                    )
                } else {
                    String::new()
                };
                self.emit_common_attrs(ind + 1, n, &extra);
                if let Some(t) = n.label(&["title", "headline", "label"]) {
                    self.line(ind + 1, &lit(t));
                }
                for child in &n.children {
                    self.node(ind + 1, child);
                }
                self.line(ind, "}");
            }
        }
    }
}

pub fn emit(doc: &Document, theme: YoloThemePreset) -> String {
    let mut e = DioxusEmitter {
        out: String::new(),
        theme,
    };
    e.node(2, &doc.root);
    let comp_name = pascal(&doc.component);
    let preset_expr = preset_variant(theme);
    let mut out = format!(
        "// Generated by yolo-migrator from {} (component {}, theme {}). Do not hand-edit.\n\
         // Target: Dioxus 0.8 `rsx!` fullstack / WebGPU / WASM.\n\n\
         use dioxus::prelude::*;\n\
         use yolo_shared::{{YoloThemePreset, YoloThemeTokens}};\n\n\
         pub const ACTIVE_THEME_PRESET: YoloThemePreset = {preset_expr};\n\
         pub const ACTIVE_THEME_TOKENS: YoloThemeTokens = {preset_expr}.tokens(true);\n\n",
        doc.source,
        doc.component,
        theme.id()
    );

    let _ = writeln!(out, "#[derive(Props, Clone, PartialEq, Default)]");
    let _ = writeln!(out, "pub struct {comp_name}Props {{");
    for h in &doc.handlers {
        let field = snake_handler(h);
        let _ = writeln!(out, "    #[props(optional)]");
        let _ = writeln!(out, "    pub {field}: Option<EventHandler<MouseEvent>>,");
    }
    let _ = writeln!(out, "}}\n");

    let props_arg = if doc.handlers.is_empty() { "_props" } else { "props" };
    let _ = writeln!(out, "#[component]");
    let _ = writeln!(out, "pub fn {comp_name}({props_arg}: {comp_name}Props) -> Element {{");
    let _ = writeln!(out, "    rsx! {{");
    out.push_str(&e.out);
    let _ = writeln!(out, "    }}");
    let _ = writeln!(out, "}}");
    out
}
