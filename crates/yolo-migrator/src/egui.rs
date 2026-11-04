// SPDX-License-Identifier: Apache-2.0
//! egui 0.32+ Rust snippet backend (catalog column `egui`). Immediate mode: the generator writes
//! a `State` struct with defaults from the TSX props, an `Actions` trait with one default method
//! per handler, and a `fn ui(...)`. Best effort: layout follows egui's own flow, not CSS.

use std::collections::HashSet;
use std::fmt::Write;

use yolo_shared::YoloThemePreset;

use crate::ir::{Align, Comp, Dir, Document, Kind, Node, slug, snake_handler, typescale_metrics};

struct E {
    out: String,
    fields: Vec<(String, String, String)>,
    names: HashSet<String>,
    ids: usize,
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
    out
}

fn margin(p: [f64; 4]) -> String {
    let c = |v: f64| v.clamp(-128.0, 127.0) as i64;
    format!("egui::Margin {{ left: {}, right: {}, top: {}, bottom: {} }}", c(p[3]), c(p[1]), c(p[0]), c(p[2]))
}

impl E {
    fn line(&mut self, ind: usize, s: &str) {
        let _ = writeln!(self.out, "{:ind$}{s}", "", ind = ind * 4);
    }

    fn field(&mut self, n: &Node, base: &str, ty: &str, default: String) -> String {
        let raw = n.id.as_deref().or(n.str("name")).map(slug).unwrap_or_else(|| slug(base));
        let mut name = raw.clone();
        let mut i = 2;
        while self.names.contains(&name) {
            name = format!("{raw}_{i}");
            i += 1;
        }
        self.names.insert(name.clone());
        self.fields.push((name.clone(), ty.into(), default));
        name
    }

    fn call(&self, h: &str) -> String {
        format!("actions.{}();", snake_handler(h))
    }

    fn rich(&self, text: &str, n: &Node) -> String {
        let mut s = format!("egui::RichText::new({})", lit(text));
        if let Some((size, weight)) = n.style.typescale.as_deref().and_then(typescale_metrics) {
            let _ = write!(s, ".size({})", f32_lit(size));
            if weight == 500 {
                s.push_str(".strong()");
            }
        }
        match n.style.color.as_deref() {
            Some("on-surface-variant" | "outline") => s.push_str(".weak()"),
            Some("primary") => s.push_str(".color(ui.visuals().selection.bg_fill)"),
            Some("error") => s.push_str(".color(ui.visuals().error_fg_color)"),
            _ => {}
        }
        s
    }

    fn container(&mut self, ind: usize, n: &Node, kids: &[&Node], card: bool) {
        let dir = n.dir();
        let mut depth = ind;
        let mut closers = Vec::new();
        if n.style.scroll {
            self.line(depth, "egui::ScrollArea::vertical().show(ui, |ui| {");
            closers.push("});");
            depth += 1;
        }
        if card || n.style.padding.is_some() || n.style.background.is_some() {
            let mut frame = if card { "egui::Frame::group(ui.style())".to_string() } else { "egui::Frame::new()".to_string() };
            let p = n.style.padding.unwrap_or(if card { [16.0; 4] } else { [0.0; 4] });
            let _ = write!(frame, ".inner_margin({})", margin(p));
            if n.style.background.is_some() {
                frame.push_str(".fill(ui.visuals().faint_bg_color)");
            }
            self.line(depth, &format!("{frame}.show(ui, |ui| {{"));
            closers.push("});");
            depth += 1;
        }
        let opener = match (dir, n.style.align) {
            (Dir::Row, _) => "ui.horizontal(|ui| {",
            (Dir::Column, Some(Align::Center)) => "ui.vertical_centered(|ui| {",
            (Dir::Column, _) => "ui.vertical(|ui| {",
        };
        self.line(depth, opener);
        if let Some(g) = n.style.gap {
            let axis = if dir == Dir::Row { "x" } else { "y" };
            self.line(depth + 1, &format!("ui.spacing_mut().item_spacing.{axis} = {};", f32_lit(g)));
        }
        for k in kids {
            self.node(depth + 1, k, false);
        }
        self.line(depth, "});");
        for c in closers.iter().rev() {
            depth -= 1;
            self.line(depth, c);
        }
    }

    fn node(&mut self, ind: usize, n: &Node, in_list: bool) {
        match n.kind {
            Kind::Box | Kind::Comp(Comp::Tab) => {
                let kids: Vec<&Node> = n.children.iter().collect();
                self.container(ind, n, &kids, false);
            }
            Kind::Text(_) => {
                let r = self.rich(n.text.as_deref().unwrap_or(""), n);
                self.line(ind, &format!("ui.label({r});"));
            }
            Kind::Image => {
                let src = n.str("src").unwrap_or("");
                let uri = if src.contains("://") { src.to_string() } else { format!("file://{src}") };
                let mut s = format!("ui.add(egui::Image::new({})", lit(&uri));
                if let Some(w) = n.num("width").or(n.style.width) {
                    let _ = write!(s, ".max_width({})", f32_lit(w));
                }
                s.push_str(");");
                self.line(ind, &s);
            }
            Kind::Comp(c) => self.comp(ind, n, c, in_list),
        }
    }

    fn clicked(&mut self, ind: usize, widget: String, n: &Node, events: &[&str], method: &str) {
        match n.event(events) {
            Some(e) => {
                let call = self.call(&e.handler);
                self.line(ind, &format!("if {widget}.{method}() {{"));
                self.line(ind + 1, &call);
                self.line(ind, "}");
            }
            None => self.line(ind, &format!("let _ = {widget};")),
        }
    }

    fn comp(&mut self, ind: usize, n: &Node, c: Comp, in_list: bool) {
        if let Some(k) = n.catalog_key() {
            self.line(ind, &format!("// catalog: components.{k}"));
        }
        let enabled = !n.flag("disabled");
        let wrap = |w: String| if enabled { w } else { format!("ui.add_enabled(false, {})", w.trim_start_matches("ui.add(").trim_end_matches(')')) };
        match c {
            Comp::Button | Comp::Fab => {
                let label = n.label(&["label"]).or(n.str("icon")).unwrap_or("");
                let w = match (c, n.variant()) {
                    (Comp::Fab, _) | (_, "filled" | "") => format!(
                        "ui.add(egui::Button::new(egui::RichText::new({}).color(ui.visuals().selection.stroke.color)).fill(ui.visuals().selection.bg_fill))",
                        lit(label)
                    ),
                    (_, "tonal") => format!("ui.add(egui::Button::new({}).fill(ui.visuals().widgets.inactive.weak_bg_fill))", lit(label)),
                    (_, "outlined") => format!(
                        "ui.add(egui::Button::new({}).fill(egui::Color32::TRANSPARENT).stroke(ui.visuals().widgets.noninteractive.bg_stroke))",
                        lit(label)
                    ),
                    (_, "text") => format!("ui.add(egui::Button::new({}).frame(false))", lit(label)),
                    _ => format!("ui.add(egui::Button::new({}))", lit(label)),
                };
                self.clicked(ind, wrap(w), n, &["onClick"], "clicked");
            }
            Comp::IconButton => {
                let icon = n.str("icon").unwrap_or("");
                let mut w = format!("ui.add(egui::Button::new({}).frame({}))", lit(icon), n.variant() != "" && n.variant() != "standard");
                if let Some(a) = &n.aria_label {
                    w = format!("{}.on_hover_text({})", wrap(w), lit(a));
                } else {
                    w = wrap(w);
                }
                self.clicked(ind, w, n, &["onClick"], "clicked");
            }
            Comp::Switch | Comp::Checkbox => {
                let label = n.label(&["label"]).unwrap_or("");
                let f = self.field(n, if label.is_empty() { "toggle" } else { label }, "bool", (n.flag("checked") || n.flag("selected")).to_string());
                let w = format!("ui.add(egui::Checkbox::new(&mut state.{f}, {}))", lit(label));
                self.clicked(ind, wrap(w), n, &["onChange", "onCheckedChange", "onClick"], "changed");
            }
            Comp::Radio => {
                let group = n.str("name").unwrap_or("choice").to_string();
                let value = n.str("value").or(n.label(&["label"])).unwrap_or("").to_string();
                let gname = slug(&group);
                let f = if self.names.contains(&gname) {
                    if n.flag("checked")
                        && let Some(fd) = self.fields.iter_mut().find(|x| x.0 == gname)
                    {
                        fd.2 = format!("{}.to_owned()", lit(&value));
                    }
                    gname
                } else {
                    let d = if n.flag("checked") { format!("{}.to_owned()", lit(&value)) } else { "String::new()".into() };
                    self.names.insert(gname.clone());
                    self.fields.push((gname.clone(), "String".into(), d));
                    gname
                };
                let label = n.label(&["label"]).unwrap_or(&value).to_string();
                let w = format!("ui.radio_value(&mut state.{f}, {}.to_owned(), {})", lit(&value), lit(&label));
                self.clicked(ind, w, n, &["onChange", "onCheckedChange", "onClick"], "changed");
            }
            Comp::Slider => {
                let label = n.label(&["label"]);
                let f = self.field(n, label.unwrap_or("slider"), "f32", f32_lit(n.num("value").unwrap_or(0.0)));
                let mut w = format!(
                    "ui.add(egui::Slider::new(&mut state.{f}, {}..={}).step_by({})",
                    f32_lit(n.num("min").unwrap_or(0.0)),
                    f32_lit(n.num("max").unwrap_or(100.0)),
                    f32_lit(n.num("step").unwrap_or(1.0))
                );
                if let Some(l) = label {
                    let _ = write!(w, ".text({})", lit(l));
                }
                w.push(')');
                self.clicked(ind, wrap(w), n, &["onChange", "onInput", "onValueChange"], "changed");
            }
            Comp::TextField => {
                let label = n.str("label");
                let f = self.field(n, label.or(n.str("placeholder")).unwrap_or("text"), "String", match n.str("value") {
                    Some(v) => format!("{}.to_owned()", lit(v)),
                    None => "String::new()".into(),
                });
                if let Some(l) = label
                    && !in_list
                {
                    self.line(ind, &format!("ui.label({});", lit(l)));
                }
                let mut w = format!("ui.add(egui::TextEdit::singleline(&mut state.{f})");
                if let Some(h) = n.str("placeholder").or(if in_list { label } else { None }) {
                    let _ = write!(w, ".hint_text({})", lit(h));
                }
                if n.str("type") == Some("password") {
                    w.push_str(".password(true)");
                }
                w.push(')');
                let change = n.event(&["onChange", "onInput", "onValueChange"]).map(|e| self.call(&e.handler));
                let submit = n.event(&["onSubmit", "onEnter"]).map(|e| self.call(&e.handler));
                if change.is_none() && submit.is_none() {
                    self.line(ind, &format!("let _ = {};", wrap(w)));
                } else {
                    self.line(ind, "{");
                    self.line(ind + 1, &format!("let response = {};", wrap(w)));
                    if let Some(c) = change {
                        self.line(ind + 1, &format!("if response.changed() {{ {c} }}"));
                    }
                    if let Some(c) = submit {
                        self.line(ind + 1, &format!("if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {{ {c} }}"));
                    }
                    self.line(ind, "}");
                }
                if let Some(s) = n.str("supportingText") {
                    self.line(ind, &format!("ui.weak({});", lit(s)));
                }
            }
            Comp::Card => {
                let kids: Vec<&Node> = n.children.iter().collect();
                let mut card = n.clone();
                card.style.direction.get_or_insert(Dir::Column);
                self.container(ind, &card, &kids, true);
            }
            Comp::Dialog => {
                self.ids += 1;
                let id = n.id.clone().unwrap_or_else(|| format!("dialog_{}", self.ids));
                self.line(ind, &format!("egui::Modal::new(egui::Id::new({})).show(ui.ctx(), |ui| {{", lit(&id)));
                if let Some(t) = n.label(&["title", "headline"]) {
                    self.line(ind + 1, &format!("ui.heading({});", lit(t)));
                }
                let (actions, content): (Vec<&Node>, Vec<&Node>) = n.children.iter().partition(|k| k.slot.as_deref() == Some("actions"));
                let mut body = n.clone();
                body.kind = Kind::Box;
                body.style.direction = Some(Dir::Column);
                body.style.padding = None;
                body.style.background = None;
                self.container(ind + 1, &body, &content, false);
                if !actions.is_empty() {
                    self.line(ind + 1, "ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {");
                    for a in actions.iter().rev() {
                        self.node(ind + 2, a, false);
                    }
                    self.line(ind + 1, "});");
                }
                self.line(ind, "});");
                if let Some(e) = n.event(&["onClose", "onOpenChange"]) {
                    self.line(ind, &format!("// {} runs when the host closes the modal (ModalResponse::should_close)", snake_handler(&e.handler)));
                }
            }
            Comp::List => {
                self.line(ind, "ui.vertical(|ui| {");
                for k in &n.children {
                    self.node(ind + 1, k, true);
                }
                self.line(ind, "});");
            }
            Comp::ListItem => {
                let title = n.label(&["headline"]).unwrap_or("");
                self.line(ind, "{");
                self.line(ind + 1, "let row = ui.horizontal(|ui| {");
                if let Some(icon) = n.str("icon").or(n.str("leadingIcon")) {
                    self.line(ind + 2, &format!("ui.weak({});", lit(icon)));
                }
                self.line(ind + 2, "ui.vertical(|ui| {");
                self.line(ind + 3, &format!("ui.label({});", lit(title)));
                if let Some(s) = n.str("supportingText") {
                    self.line(ind + 3, &format!("ui.weak({});", lit(s)));
                }
                self.line(ind + 2, "});");
                if n.str("trailingText").is_some() || !n.children.is_empty() {
                    self.line(ind + 2, "ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {");
                    for k in n.children.iter().rev() {
                        self.node(ind + 3, k, false);
                    }
                    if let Some(t) = n.str("trailingText") {
                        self.line(ind + 3, &format!("ui.weak({});", lit(t)));
                    }
                    self.line(ind + 2, "});");
                }
                self.line(ind + 1, "}).response;");
                match n.event(&["onClick"]) {
                    Some(e) => {
                        let call = self.call(&e.handler);
                        self.line(ind + 1, &format!("if row.interact(egui::Sense::click()).clicked() {{ {call} }}"));
                    }
                    None => self.line(ind + 1, "let _ = row;"),
                }
                self.line(ind, "}");
            }
            Comp::Divider => self.line(ind, "ui.separator();"),
            Comp::Tabs => {
                let f = self.field(n, "tab", "usize", format!("{}", n.num("activeIndex").unwrap_or(0.0) as usize));
                self.line(ind, "ui.horizontal(|ui| {");
                let call = n.event(&["onChange", "onValueChange"]).map(|e| self.call(&e.handler));
                for (i, t) in n.children.iter().enumerate() {
                    let w = format!("ui.selectable_value(&mut state.{f}, {i}, {})", lit(t.label(&["label"]).unwrap_or("")));
                    match &call {
                        Some(c) => self.line(ind + 1, &format!("if {w}.clicked() {{ {c} }}")),
                        None => self.line(ind + 1, &format!("let _ = {w};")),
                    }
                }
                self.line(ind, "});");
                self.line(ind, &format!("match state.{f} {{"));
                for (i, t) in n.children.iter().enumerate() {
                    self.line(ind + 1, &format!("{i} => {{"));
                    let mut page = t.clone();
                    page.kind = Kind::Box;
                    page.style.direction.get_or_insert(Dir::Column);
                    self.node(ind + 2, &page, false);
                    self.line(ind + 1, "}");
                }
                self.line(ind + 1, "_ => {}");
                self.line(ind, "}");
            }
            Comp::TopAppBar => {
                self.line(ind, "ui.horizontal(|ui| {");
                let is_start = |k: &&Node| matches!(k.slot.as_deref(), Some("start" | "navigation" | "leading"));
                for k in n.children.iter().filter(is_start) {
                    self.node(ind + 1, k, false);
                }
                self.line(ind + 1, &format!("ui.heading({});", lit(n.label(&["title", "headline"]).unwrap_or(""))));
                let end: Vec<&Node> = n.children.iter().filter(|k| !is_start(k)).collect();
                if !end.is_empty() {
                    self.line(ind + 1, "ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {");
                    for k in end.iter().rev() {
                        self.node(ind + 2, k, false);
                    }
                    self.line(ind + 1, "});");
                }
                self.line(ind, "});");
            }
            Comp::NavigationRail => {
                let sel = n.children.iter().position(|i| i.flag("selected") || i.flag("active")).unwrap_or(0);
                let f = self.field(n, "rail", "usize", sel.to_string());
                self.line(ind, "ui.vertical(|ui| {");
                for (i, item) in n.children.iter().enumerate() {
                    let w = format!("ui.selectable_value(&mut state.{f}, {i}, {})", lit(item.label(&["label"]).unwrap_or("")));
                    match item.event(&["onClick"]) {
                        Some(e) => {
                            let c = self.call(&e.handler);
                            self.line(ind + 1, &format!("if {w}.clicked() {{ {c} }}"));
                        }
                        None => self.line(ind + 1, &format!("let _ = {w};")),
                    }
                }
                self.line(ind, "});");
            }
            Comp::NavigationRailItem | Comp::Tab => {}
            Comp::Chip => {
                let label = n.label(&["label"]).unwrap_or("");
                let filter = matches!(n.variant(), "filter" | "input") || n.props.contains_key("selected");
                let w = if filter {
                    let f = self.field(n, label, "bool", n.flag("selected").to_string());
                    format!("ui.toggle_value(&mut state.{f}, {})", lit(label))
                } else {
                    format!("ui.button({})", lit(label))
                };
                self.clicked(ind, w, n, &["onClick"], "clicked");
            }
            Comp::Progress => match (n.is_circular_progress(), n.progress_fraction()) {
                (false, Some(f)) => self.line(ind, &format!("ui.add(egui::ProgressBar::new({}));", f32_lit((f * 1000.0).round() / 1000.0))),
                _ => self.line(ind, "ui.add(egui::Spinner::new());"),
            },
        }
    }
}

pub fn emit(doc: &Document) -> String {
    emit_with_theme(doc, YoloThemePreset::Neutral)
}

pub fn emit_with_theme(doc: &Document, theme: YoloThemePreset) -> String {
    let mut e = E { out: String::new(), fields: Vec::new(), names: HashSet::new(), ids: 0 };
    e.node(1, &doc.root, false);
    let name = pascal(&doc.component);
    let snake = slug(&doc.component.chars().flat_map(|c| if c.is_ascii_uppercase() { vec!['_', c] } else { vec![c] }).collect::<String>());
    let mut out = format!(
        "// Generated by yolo-migrator from {} (component {}, theme {}). Do not hand-edit.\n// Target: egui 0.32+.\n\n",
        doc.source, doc.component, theme.id()
    );
    let _ = writeln!(out, "/// Handlers from the TSX event props; override the ones the app needs.");
    let _ = writeln!(out, "pub trait {name}Actions {{");
    for h in &doc.handlers {
        let _ = writeln!(out, "    fn {}(&mut self) {{}}", snake_handler(h));
    }
    let _ = writeln!(out, "}}\n");
    let _ = writeln!(out, "/// Widget state generated from the TSX props (React keeps it in the DOM).");
    let _ = writeln!(out, "#[derive(Clone, Debug)]");
    let _ = writeln!(out, "pub struct {name}State {{");
    for (f, ty, _) in &e.fields {
        let _ = writeln!(out, "    pub {f}: {ty},");
    }
    let _ = writeln!(out, "}}\n");
    let _ = writeln!(out, "impl Default for {name}State {{");
    let _ = writeln!(out, "    fn default() -> Self {{");
    let _ = writeln!(out, "        Self {{");
    for (f, _, d) in &e.fields {
        let _ = writeln!(out, "            {f}: {d},");
    }
    let _ = writeln!(out, "        }}\n    }}\n}}\n");
    let unused = if doc.handlers.is_empty() { "_actions" } else { "actions" };
    let state_name = if e.fields.is_empty() { "_state" } else { "state" };
    let _ = writeln!(out, "pub fn {snake}_ui(ui: &mut egui::Ui, {state_name}: &mut {name}State, {unused}: &mut impl {name}Actions) {{");
    out.push_str(&e.out);
    out.push_str("}\n");
    out
}
