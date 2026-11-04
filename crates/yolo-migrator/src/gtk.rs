// SPDX-License-Identifier: Apache-2.0
//! GtkBuilder XML backend for GTK 4.16+ and libadwaita 1.6+ (catalog column `gtk`).
//! Colors and typescale map to libadwaita style classes; semantic roles without a native class map to
//! `m3-*` classes written to a sidecar CSS.

use std::collections::{BTreeSet, HashMap};
use std::fmt::Write;

use yolo_shared::YoloThemePreset;

use crate::ir::{Align, Comp, Dir, Document, Justify, Kind, Node, TextTag, snake_handler};

#[derive(Clone, Debug)]
pub struct Output {
    pub ui: String,
    pub css: Option<String>,
}

#[derive(Clone, Copy)]
struct Ctx {
    parent: Dir,
    in_list: bool,
    align: Option<Align>,
    expand_main: bool,
    end: bool,
}

impl Ctx {
    fn root() -> Self {
        Ctx { parent: Dir::Column, in_list: false, align: None, expand_main: false, end: false }
    }
}

struct G {
    out: String,
    next: HashMap<&'static str, usize>,
    groups: HashMap<String, String>,
    css: BTreeSet<String>,
}

pub fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn fmt_num(n: f64) -> String {
    if n.fract() == 0.0 { format!("{}", n as i64) } else { format!("{n}") }
}

/// M3 typescale -> libadwaita typography class.
fn adw_type_class(ts: &str) -> &'static str {
    match ts {
        "display-large" | "display-medium" | "display-small" | "headline-large" => "title-1",
        "headline-medium" => "title-2",
        "headline-small" => "title-3",
        "title-large" => "title-4",
        "title-medium" | "title-small" => "heading",
        "label-large" | "label-medium" => "caption-heading",
        "label-small" | "body-small" => "caption",
        _ => "body",
    }
}

impl G {
    fn id(&mut self, n: &Node, base: &'static str) -> String {
        if let Some(id) = &n.id {
            return crate::ir::slug(id);
        }
        let c = self.next.entry(base).or_insert(0);
        *c += 1;
        format!("{base}{c}")
    }

    fn line(&mut self, ind: usize, s: &str) {
        let _ = writeln!(self.out, "{:ind$}{s}", "", ind = ind * 2);
    }

    fn prop(&mut self, ind: usize, name: &str, value: &str) {
        self.line(ind, &format!("<property name=\"{name}\">{}</property>", esc(value)));
    }

    fn tprop(&mut self, ind: usize, name: &str, value: &str) {
        self.line(ind, &format!("<property name=\"{name}\" translatable=\"yes\">{}</property>", esc(value)));
    }

    fn classes(&mut self, ind: usize, classes: &[String]) {
        if classes.is_empty() {
            return;
        }
        self.line(ind, "<style>");
        for c in classes {
            self.line(ind + 1, &format!("<class name=\"{c}\"/>"));
        }
        self.line(ind, "</style>");
    }

    fn signal(&mut self, ind: usize, signal: &str, handler: &str) {
        self.line(ind, &format!("<signal name=\"{signal}\" handler=\"{}\"/>", snake_handler(handler)));
    }

    fn a11y_label(&mut self, ind: usize, label: &str) {
        self.line(ind, "<accessibility>");
        self.line(ind + 1, &format!("<property name=\"label\" translatable=\"yes\">{}</property>", esc(label)));
        self.line(ind, "</accessibility>");
    }

    fn open(&mut self, ind: usize, class: &str, id: Option<&str>, n: Option<&Node>) {
        if let Some(key) = n.and_then(Node::catalog_key) {
            self.line(ind, &format!("<!-- catalog: components.{key} -->"));
        }
        match id {
            Some(id) => self.line(ind, &format!("<object class=\"{class}\" id=\"{id}\">")),
            None => self.line(ind, &format!("<object class=\"{class}\">")),
        }
    }

    fn close(&mut self, ind: usize) {
        self.line(ind, "</object>");
    }

    fn color_classes(&mut self, n: &Node) -> Vec<String> {
        let mut out = Vec::new();
        if let Some(c) = &n.style.color {
            if c == "on-surface-variant" {
                out.push("dim-label".into());
            } else {
                out.push(format!("m3-color-{c}"));
                self.css.insert(format!(".m3-color-{c} {{ color: var(--m3-{c}); }}"));
            }
        }
        if let Some(b) = &n.style.background {
            out.push(format!("m3-bg-{b}"));
            self.css.insert(format!(".m3-bg-{b} {{ background-color: var(--m3-{b}); }}"));
        }
        if let Some(s) = &n.style.shape {
            let r = crate::ir::shape_radius(s).unwrap_or(0.0);
            out.push(format!("m3-shape-{s}"));
            self.css.insert(format!(".m3-shape-{s} {{ border-radius: {}px; }}", fmt_num(r)));
        }
        out
    }

    fn margins(&mut self, ind: usize, m: [f64; 4]) {
        for (i, side) in ["top", "end", "bottom", "start"].iter().enumerate() {
            if m[i] != 0.0 {
                self.prop(ind, &format!("margin-{side}"), &fmt_num(m[i]));
            }
        }
    }

    fn layout(&mut self, ind: usize, n: &Node, ctx: Ctx, extra_margin: Option<[f64; 4]>) {
        let mut m = n.style.margin.unwrap_or([0.0; 4]);
        if let Some(e) = extra_margin {
            for i in 0..4 {
                m[i] += e[i];
            }
        }
        self.margins(ind, m);
        let grow = n.style.grow || ctx.expand_main;
        let main_expand = if ctx.parent == Dir::Row { "hexpand" } else { "vexpand" };
        if grow {
            self.prop(ind, main_expand, "True");
        }
        if n.style.scroll && ctx.parent == Dir::Column && !grow {
            self.prop(ind, "vexpand", "True");
        }
        let cross = if ctx.parent == Dir::Row { "valign" } else { "halign" };
        let main = if ctx.parent == Dir::Row { "halign" } else { "valign" };
        let gtk_align = |a: Align| match a {
            Align::Start => "start",
            Align::Center => "center",
            Align::End => "end",
            Align::Stretch => "fill",
        };
        let clamped = n.style.max_width.is_some();
        if let Some(a) = ctx.align.filter(|a| *a != Align::Stretch && !(clamped && *a == Align::Center)) {
            self.prop(ind, cross, gtk_align(a));
        } else if ctx.parent == Dir::Row && matches!(n.kind, Kind::Comp(Comp::Switch | Comp::IconButton)) {
            self.prop(ind, "valign", "center");
        }
        if ctx.end {
            self.prop(ind, main, "end");
        } else if ctx.expand_main {
            self.prop(ind, main, "start");
        }
        if let Some(w) = n.style.width {
            self.prop(ind, "width-request", &fmt_num(w));
        }
        if let Some(h) = n.style.height {
            self.prop(ind, "height-request", &fmt_num(h));
        }
    }

    fn child(&mut self, ind: usize, n: &Node, ctx: Ctx, ty: Option<&str>) {
        match ty {
            Some(t) => self.line(ind, &format!("<child type=\"{t}\">")),
            None => self.line(ind, "<child>"),
        }
        self.object(ind + 1, n, ctx);
        self.line(ind, "</child>");
    }

    fn object(&mut self, ind: usize, n: &Node, ctx: Ctx) {
        let scroll = n.style.scroll;
        let clamp = n.style.max_width;
        if !scroll && clamp.is_none() {
            self.core(ind, n, ctx, true);
            return;
        }
        let mut depth = ind;
        let mut first = true;
        if scroll {
            self.open(depth, "GtkScrolledWindow", None, None);
            self.layout(depth + 1, n, ctx, None);
            first = false;
            self.prop(depth + 1, "hscrollbar-policy", "never");
            self.prop(depth + 1, "propagate-natural-height", "True");
            self.line(depth + 1, "<property name=\"child\">");
            depth += 2;
        }
        if let Some(w) = clamp {
            self.open(depth, "AdwClamp", None, None);
            if first {
                self.layout(depth + 1, n, ctx, None);
            }
            first = false;
            self.prop(depth + 1, "maximum-size", &fmt_num(w));
            self.line(depth + 1, "<property name=\"child\">");
            depth += 2;
        }
        let _ = first;
        let inner = Ctx { expand_main: false, end: false, align: None, ..ctx };
        let mut plain = n.clone();
        plain.style.margin = None;
        plain.style.grow = false;
        plain.style.width = None;
        plain.style.height = None;
        self.core(depth, &plain, inner, true);
        if clamp.is_some() {
            depth -= 2;
            self.line(depth + 1, "</property>");
            self.close(depth);
        }
        if scroll {
            depth -= 2;
            self.line(depth + 1, "</property>");
            self.close(depth);
        }
    }

    fn label_obj(&mut self, ind: usize, text: &str, classes: &[&str], xalign: Option<&str>) {
        self.open(ind, "GtkLabel", None, None);
        self.tprop(ind + 1, "label", text);
        self.prop(ind + 1, "xalign", xalign.unwrap_or("0"));
        self.prop(ind + 1, "wrap", "True");
        let cls: Vec<String> = classes.iter().map(|s| s.to_string()).collect();
        self.classes(ind + 1, &cls);
        self.close(ind);
    }

    fn box_children(&mut self, ind: usize, n: &Node, kids: &[&Node], dir: Dir) {
        let count = kids.len();
        for (i, k) in kids.iter().enumerate() {
            let spread = n.style.justify == Some(Justify::SpaceBetween) && count > 1;
            let ctx = Ctx {
                parent: dir,
                in_list: false,
                align: n.style.align,
                expand_main: spread && i == 0,
                end: spread && i == count - 1,
            };
            self.child(ind, k, ctx, None);
        }
    }

    fn gtk_box(&mut self, ind: usize, n: &Node, ctx: Ctx, layout: bool, extra_classes: Vec<String>, kids: &[&Node]) {
        let dir = n.dir();
        let padded = n.style.padding.is_some();
        let decorated = !extra_classes.is_empty() || n.style.background.is_some();
        let id = n.id.as_ref().map(|i| crate::ir::slug(i));
        self.open(ind, "GtkBox", id.as_deref(), Some(n));
        if layout {
            self.layout(ind + 1, n, ctx, if decorated { None } else { n.style.padding });
        }
        let orient = |d: Dir| if d == Dir::Row { "horizontal" } else { "vertical" };
        let mut classes = extra_classes;
        classes.extend(self.color_classes(n));
        let justify = match n.style.justify {
            Some(Justify::Center) => Some("center"),
            Some(Justify::End) => Some("end"),
            _ => None,
        };
        if decorated && padded {
            self.prop(ind + 1, "orientation", "vertical");
            self.classes(ind + 1, &classes);
            self.line(ind + 1, "<child>");
            self.open(ind + 2, "GtkBox", None, None);
            self.margins(ind + 3, n.style.padding.unwrap_or([0.0; 4]));
            self.prop(ind + 3, "orientation", orient(dir));
            if let Some(g) = n.style.gap {
                self.prop(ind + 3, "spacing", &fmt_num(g));
            }
            if let Some(j) = justify {
                self.prop(ind + 3, if dir == Dir::Row { "halign" } else { "valign" }, j);
            }
            self.box_children(ind + 3, n, kids, dir);
            self.close(ind + 2);
            self.line(ind + 1, "</child>");
        } else {
            self.prop(ind + 1, "orientation", orient(dir));
            if let Some(g) = n.style.gap {
                self.prop(ind + 1, "spacing", &fmt_num(g));
            }
            if let Some(j) = justify {
                self.prop(ind + 1, if dir == Dir::Row { "halign" } else { "valign" }, j);
            }
            self.classes(ind + 1, &classes);
            self.box_children(ind + 1, n, kids, dir);
        }
        self.close(ind);
    }

    fn header_bar(&mut self, ind: usize, n: &Node, ctx: Ctx, layout: bool) {
        self.open(ind, "AdwHeaderBar", None, Some(n));
        if layout {
            self.layout(ind + 1, n, ctx, None);
        }
        if let Some(title) = n.label(&["title", "headline"]) {
            self.line(ind + 1, "<property name=\"title-widget\">");
            self.open(ind + 2, "AdwWindowTitle", None, None);
            self.tprop(ind + 3, "title", title);
            if let Some(sub) = n.str("subtitle") {
                self.tprop(ind + 3, "subtitle", sub);
            }
            self.close(ind + 2);
            self.line(ind + 1, "</property>");
        }
        for k in &n.children {
            let ty = match k.slot.as_deref() {
                Some("start" | "navigation" | "leading") => "start",
                _ => "end",
            };
            self.child(ind + 1, k, Ctx { parent: Dir::Row, ..Ctx::root() }, Some(ty));
        }
        self.close(ind);
    }

    fn core(&mut self, ind: usize, n: &Node, ctx: Ctx, layout: bool) {
        let lay = |g: &mut G, ind: usize| {
            if layout {
                g.layout(ind, n, ctx, None)
            }
        };
        match n.kind {
            Kind::Box => {
                let dir = n.dir();
                if dir == Dir::Column && n.children.first().is_some_and(|c| c.is(Comp::TopAppBar)) {
                    let id = n.id.as_ref().map(|i| crate::ir::slug(i));
                    self.open(ind, "AdwToolbarView", id.as_deref(), None);
                    lay(self, ind + 1);
                    self.line(ind + 1, "<child type=\"top\">");
                    self.header_bar(ind + 2, &n.children[0], Ctx::root(), false);
                    self.line(ind + 1, "</child>");
                    self.line(ind + 1, "<property name=\"content\">");
                    let mut rest = n.clone();
                    rest.children.remove(0);
                    rest.id = None;
                    rest.style.margin = None;
                    rest.style.grow = false;
                    let kids: Vec<&Node> = rest.children.iter().collect();
                    self.gtk_box(ind + 2, &rest, Ctx::root(), true, Vec::new(), &kids);
                    self.line(ind + 1, "</property>");
                    self.close(ind);
                } else {
                    let kids: Vec<&Node> = n.children.iter().collect();
                    self.gtk_box(ind, n, ctx, layout, Vec::new(), &kids);
                }
            }
            Kind::Text(tag) => {
                self.open(ind, "GtkLabel", None, None);
                lay(self, ind + 1);
                self.tprop(ind + 1, "label", n.text.as_deref().unwrap_or(""));
                let xalign = match n.style.text_align {
                    Some(Align::Center) => "0.5",
                    Some(Align::End) => "1",
                    _ => "0",
                };
                self.prop(ind + 1, "xalign", xalign);
                if xalign == "0.5" {
                    self.prop(ind + 1, "justify", "center");
                }
                self.prop(ind + 1, "wrap", "True");
                let mut classes = vec![adw_type_class(n.style.typescale.as_deref().unwrap_or("body-medium")).to_string()];
                classes.extend(self.color_classes(n));
                self.classes(ind + 1, &classes);
                if let TextTag::Heading(level) = tag {
                    self.line(ind + 1, "<accessibility>");
                    self.line(ind + 2, &format!("<property name=\"level\">{level}</property>"));
                    self.line(ind + 1, "</accessibility>");
                }
                self.close(ind);
            }
            Kind::Image => {
                self.open(ind, "GtkImage", None, None);
                lay(self, ind + 1);
                if let Some(src) = n.str("src") {
                    self.prop(ind + 1, "file", src);
                }
                if let Some(w) = n.num("width").or(n.style.width) {
                    self.prop(ind + 1, "pixel-size", &fmt_num(w));
                }
                if let Some(alt) = n.str("alt") {
                    self.a11y_label(ind + 1, alt);
                }
                self.close(ind);
            }
            Kind::Comp(c) => self.comp(ind, n, c, ctx, layout),
        }
    }

    fn comp(&mut self, ind: usize, n: &Node, c: Comp, ctx: Ctx, layout: bool) {
        let lay = |g: &mut G, ind: usize| {
            if layout {
                g.layout(ind, n, ctx, None)
            }
        };
        let sens = |g: &mut G, ind: usize| {
            if n.flag("disabled") {
                g.prop(ind, "sensitive", "False");
            }
        };
        match c {
            Comp::Button | Comp::Fab => {
                let id = n.id.as_ref().map(|i| crate::ir::slug(i));
                self.open(ind, "GtkButton", id.as_deref(), Some(n));
                lay(self, ind + 1);
                sens(self, ind + 1);
                let label = n.label(&["label"]);
                let icon = n.str("icon");
                match (icon, label) {
                    (Some(icon), Some(label)) => {
                        self.line(ind + 1, "<property name=\"child\">");
                        self.open(ind + 2, "AdwButtonContent", None, None);
                        self.prop(ind + 3, "icon-name", icon);
                        self.tprop(ind + 3, "label", label);
                        self.close(ind + 2);
                        self.line(ind + 1, "</property>");
                    }
                    (Some(icon), None) => self.prop(ind + 1, "icon-name", icon),
                    (None, Some(label)) => self.tprop(ind + 1, "label", label),
                    (None, None) => {}
                }
                let mut classes: Vec<String> = match (c, n.variant()) {
                    (Comp::Fab, _) => vec!["suggested-action".into(), if label.is_some() { "pill" } else { "circular" }.into()],
                    (_, "tonal") => vec!["m3-tonal".into()],
                    (_, "outlined") => vec!["m3-outlined".into()],
                    (_, "text") => vec!["flat".into()],
                    (_, "elevated") => vec!["card".into()],
                    _ => vec!["suggested-action".into()],
                };
                if n.str("size").is_some_and(|s| s != "sm") {
                    classes.push("pill".into());
                }
                classes.extend(self.color_classes(n));
                self.classes(ind + 1, &classes);
                if let Some(a) = &n.aria_label {
                    self.tprop(ind + 1, "tooltip-text", a);
                }
                if let Some(e) = n.event(&["onClick"]) {
                    self.signal(ind + 1, "clicked", &e.handler);
                }
                self.close(ind);
            }
            Comp::IconButton => {
                let toggle = n.flag("toggle") || n.props.contains_key("selected");
                self.open(ind, if toggle { "GtkToggleButton" } else { "GtkButton" }, None, Some(n));
                lay(self, ind + 1);
                sens(self, ind + 1);
                self.prop(ind + 1, "icon-name", n.str("icon").unwrap_or(""));
                if toggle && n.flag("selected") {
                    self.prop(ind + 1, "active", "True");
                }
                if let Some(a) = &n.aria_label {
                    self.tprop(ind + 1, "tooltip-text", a);
                }
                let classes: Vec<String> = match n.variant() {
                    "filled" => vec!["suggested-action".into(), "circular".into()],
                    "tonal" => vec!["m3-tonal".into(), "circular".into()],
                    "outlined" => vec!["m3-outlined".into(), "circular".into()],
                    _ => vec!["flat".into()],
                };
                self.classes(ind + 1, &classes);
                if let Some(a) = &n.aria_label {
                    self.a11y_label(ind + 1, a);
                }
                if let Some(e) = n.event(&["onClick"]) {
                    self.signal(ind + 1, "clicked", &e.handler);
                }
                self.close(ind);
            }
            Comp::Switch => {
                let label = n.label(&["label"]);
                if ctx.in_list {
                    self.open(ind, "AdwSwitchRow", None, Some(n));
                    self.tprop(ind + 1, "title", label.unwrap_or(""));
                    self.switch_body(ind + 1, n, "notify::active");
                    self.close(ind);
                } else if let Some(label) = label {
                    self.open(ind, "GtkBox", None, Some(n));
                    lay(self, ind + 1);
                    self.prop(ind + 1, "spacing", "12");
                    self.line(ind + 1, "<child>");
                    self.open(ind + 2, "GtkLabel", None, None);
                    self.tprop(ind + 3, "label", label);
                    self.prop(ind + 3, "xalign", "0");
                    self.prop(ind + 3, "hexpand", "True");
                    self.close(ind + 2);
                    self.line(ind + 1, "</child>");
                    self.line(ind + 1, "<child>");
                    self.open(ind + 2, "GtkSwitch", None, None);
                    self.prop(ind + 3, "valign", "center");
                    self.switch_body(ind + 3, n, "notify::active");
                    self.a11y_label(ind + 3, label);
                    self.close(ind + 2);
                    self.line(ind + 1, "</child>");
                    self.close(ind);
                } else {
                    self.open(ind, "GtkSwitch", None, Some(n));
                    lay(self, ind + 1);
                    self.switch_body(ind + 1, n, "notify::active");
                    if let Some(a) = &n.aria_label {
                        self.a11y_label(ind + 1, a);
                    }
                    self.close(ind);
                }
            }
            Comp::Checkbox | Comp::Radio => {
                let id = (c == Comp::Radio || n.id.is_some())
                    .then(|| self.id(n, if c == Comp::Radio { "radio" } else { "check" }));
                self.open(ind, "GtkCheckButton", id.as_deref(), Some(n));
                lay(self, ind + 1);
                sens(self, ind + 1);
                if let Some(l) = n.label(&["label"]) {
                    self.tprop(ind + 1, "label", l);
                }
                if n.flag("checked") {
                    self.prop(ind + 1, "active", "True");
                }
                if c == Comp::Radio {
                    let group = n.str("name").unwrap_or("default").to_string();
                    match self.groups.get(&group) {
                        Some(first) => {
                            let first = first.clone();
                            self.prop(ind + 1, "group", &first);
                        }
                        None => {
                            self.groups.insert(group, id.clone().unwrap_or_default());
                        }
                    }
                }
                if let Some(e) = n.event(&["onChange", "onCheckedChange", "onClick"]) {
                    self.signal(ind + 1, "toggled", &e.handler);
                }
                self.close(ind);
            }
            Comp::Slider => {
                let label = n.label(&["label"]);
                let inner = ind + if label.is_some() { 2 } else { 0 };
                if let Some(label) = label {
                    self.open(ind, "GtkBox", None, Some(n));
                    lay(self, ind + 1);
                    self.prop(ind + 1, "orientation", "vertical");
                    self.prop(ind + 1, "spacing", "4");
                    self.line(ind + 1, "<child>");
                    self.label_obj(ind + 2, label, &["caption-heading"], None);
                    self.line(ind + 1, "</child>");
                    self.line(ind + 1, "<child>");
                    self.open(inner, "GtkScale", None, None);
                } else {
                    self.open(inner, "GtkScale", None, Some(n));
                    lay(self, inner + 1);
                }
                sens(self, inner + 1);
                self.prop(inner + 1, "orientation", "horizontal");
                self.prop(inner + 1, "hexpand", "True");
                self.line(inner + 1, "<property name=\"adjustment\">");
                self.open(inner + 2, "GtkAdjustment", None, None);
                self.prop(inner + 3, "lower", &fmt_num(n.num("min").unwrap_or(0.0)));
                self.prop(inner + 3, "upper", &fmt_num(n.num("max").unwrap_or(100.0)));
                self.prop(inner + 3, "value", &fmt_num(n.num("value").unwrap_or(0.0)));
                self.prop(inner + 3, "step-increment", &fmt_num(n.num("step").unwrap_or(1.0)));
                self.close(inner + 2);
                self.line(inner + 1, "</property>");
                if let Some(l) = label.or(n.aria_label.as_deref()) {
                    self.a11y_label(inner + 1, l);
                }
                if let Some(e) = n.event(&["onChange", "onInput", "onValueChange"]) {
                    self.signal(inner + 1, "value-changed", &e.handler);
                }
                self.close(inner);
                if label.is_some() {
                    self.line(ind + 1, "</child>");
                    self.close(ind);
                }
            }
            Comp::TextField => {
                let password = n.str("type") == Some("password");
                let label = n.str("label");
                let id = n.id.as_ref().map(|i| crate::ir::slug(i));
                if ctx.in_list {
                    self.open(ind, if password { "AdwPasswordEntryRow" } else { "AdwEntryRow" }, id.as_deref(), Some(n));
                    self.tprop(ind + 1, "title", label.or(n.str("placeholder")).unwrap_or(""));
                    self.entry_body(ind + 1, n, password, true);
                    self.close(ind);
                    return;
                }
                let support = n.str("supportingText");
                let wrapped = label.is_some() || support.is_some();
                let inner = if wrapped { ind + 2 } else { ind };
                if wrapped {
                    self.open(ind, "GtkBox", None, Some(n));
                    lay(self, ind + 1);
                    self.prop(ind + 1, "orientation", "vertical");
                    self.prop(ind + 1, "spacing", "4");
                    if let Some(l) = label {
                        self.line(ind + 1, "<child>");
                        self.label_obj(ind + 2, l, &["caption-heading"], None);
                        self.line(ind + 1, "</child>");
                    }
                    self.line(ind + 1, "<child>");
                    self.open(inner, if password { "GtkPasswordEntry" } else { "GtkEntry" }, id.as_deref(), None);
                } else {
                    self.open(inner, if password { "GtkPasswordEntry" } else { "GtkEntry" }, id.as_deref(), Some(n));
                    lay(self, inner + 1);
                }
                if let Some(p) = n.str("placeholder").or(if wrapped { None } else { label }) {
                    self.tprop(inner + 1, "placeholder-text", p);
                }
                if password {
                    self.prop(inner + 1, "show-peek-icon", "True");
                }
                self.entry_body(inner + 1, n, password, false);
                if let Some(l) = label {
                    self.a11y_label(inner + 1, l);
                }
                self.close(inner);
                if wrapped {
                    self.line(ind + 1, "</child>");
                    if let Some(s) = support {
                        self.line(ind + 1, "<child>");
                        self.label_obj(ind + 2, s, &["caption", "dim-label"], None);
                        self.line(ind + 1, "</child>");
                    }
                    self.close(ind);
                }
            }
            Comp::Card => {
                let mut n2 = n.clone();
                if n2.style.direction.is_none() {
                    n2.style.direction = Some(Dir::Column);
                }
                let mut classes = vec!["card".to_string()];
                if n.variant() == "outlined" {
                    classes.push("m3-outlined".into());
                }
                let kids: Vec<&Node> = n.children.iter().collect();
                self.gtk_box(ind, &n2, ctx, layout, classes, &kids);
            }
            Comp::Dialog => {
                let id = n.id.as_ref().map(|i| crate::ir::slug(i));
                self.open(ind, "AdwDialog", id.as_deref(), Some(n));
                if let Some(t) = n.label(&["title", "headline"]) {
                    self.tprop(ind + 1, "title", t);
                }
                if let Some(w) = n.style.width {
                    self.prop(ind + 1, "content-width", &fmt_num(w));
                }
                if let Some(e) = n.event(&["onClose", "onOpenChange"]) {
                    self.signal(ind + 1, "closed", &e.handler);
                }
                self.line(ind + 1, "<property name=\"child\">");
                self.open(ind + 2, "AdwToolbarView", None, None);
                self.line(ind + 3, "<child type=\"top\">");
                self.open(ind + 4, "AdwHeaderBar", None, None);
                self.close(ind + 4);
                self.line(ind + 3, "</child>");
                self.line(ind + 3, "<property name=\"content\">");
                let mut body = n.clone();
                body.kind = Kind::Box;
                body.id = None;
                body.style.width = None;
                body.style.direction = Some(Dir::Column);
                body.style.padding.get_or_insert([0.0, 24.0, 24.0, 24.0]);
                body.style.gap.get_or_insert(16.0);
                let (actions, content): (Vec<&Node>, Vec<&Node>) =
                    n.children.iter().partition(|k| k.slot.as_deref() == Some("actions"));
                let mut kids: Vec<&Node> = content;
                let action_box;
                if !actions.is_empty() {
                    let mut b = Node::new(Kind::Box, n.line, n.col);
                    b.style.direction = Some(Dir::Row);
                    b.style.gap = Some(8.0);
                    b.style.justify = Some(Justify::End);
                    b.children = actions.into_iter().cloned().collect();
                    action_box = b;
                    kids.push(&action_box);
                }
                body.style.background = None;
                self.gtk_box(ind + 4, &body, Ctx::root(), true, Vec::new(), &kids);
                self.line(ind + 3, "</property>");
                self.close(ind + 2);
                self.line(ind + 1, "</property>");
                self.close(ind);
            }
            Comp::List => {
                self.open(ind, "GtkListBox", None, Some(n));
                lay(self, ind + 1);
                self.prop(ind + 1, "selection-mode", "none");
                let mut classes = vec!["boxed-list".to_string()];
                classes.extend(self.color_classes(n));
                self.classes(ind + 1, &classes);
                for k in &n.children {
                    self.child(ind + 1, k, Ctx { in_list: true, ..Ctx::root() }, None);
                }
                self.close(ind);
            }
            Comp::ListItem => {
                let title = n.label(&["headline"]).unwrap_or("");
                let markup_safe = !title.contains(['&', '<']);
                let sole_switch = n.children.len() == 1 && n.children[0].is(Comp::Switch);
                if sole_switch {
                    let sw = &n.children[0];
                    self.open(ind, "AdwSwitchRow", None, Some(sw));
                    self.tprop(ind + 1, "title", title);
                    if !markup_safe {
                        self.prop(ind + 1, "use-markup", "False");
                    }
                    if let Some(s) = n.str("supportingText") {
                        self.tprop(ind + 1, "subtitle", s);
                    }
                    self.switch_body(ind + 1, sw, "notify::active");
                    self.close(ind);
                    return;
                }
                self.open(ind, "AdwActionRow", None, Some(n));
                self.tprop(ind + 1, "title", title);
                if !markup_safe {
                    self.prop(ind + 1, "use-markup", "False");
                }
                if let Some(s) = n.str("supportingText") {
                    self.tprop(ind + 1, "subtitle", s);
                }
                if let Some(e) = n.event(&["onClick"]) {
                    self.prop(ind + 1, "activatable", "True");
                    self.signal(ind + 1, "activated", &e.handler);
                }
                if let Some(icon) = n.str("icon").or(n.str("leadingIcon")) {
                    self.line(ind + 1, "<child type=\"prefix\">");
                    self.open(ind + 2, "GtkImage", None, None);
                    self.prop(ind + 3, "icon-name", icon);
                    self.close(ind + 2);
                    self.line(ind + 1, "</child>");
                }
                if let Some(t) = n.str("trailingText") {
                    self.line(ind + 1, "<child type=\"suffix\">");
                    self.label_obj(ind + 2, t, &["dim-label"], None);
                    self.line(ind + 1, "</child>");
                }
                for k in &n.children {
                    self.child(ind + 1, k, Ctx { parent: Dir::Row, align: Some(Align::Center), ..Ctx::root() }, Some("suffix"));
                }
                self.close(ind);
            }
            Comp::Divider => {
                self.open(ind, "GtkSeparator", None, Some(n));
                lay(self, ind + 1);
                let o = if ctx.parent == Dir::Row { "vertical" } else { "horizontal" };
                self.prop(ind + 1, "orientation", o);
                self.close(ind);
            }
            Comp::Tabs => {
                let stack = self.id(n, "view_stack");
                self.open(ind, "GtkBox", None, Some(n));
                lay(self, ind + 1);
                self.prop(ind + 1, "orientation", "vertical");
                self.line(ind + 1, "<child>");
                self.open(ind + 2, "AdwViewSwitcher", None, None);
                self.prop(ind + 3, "stack", &stack);
                self.prop(ind + 3, "policy", "wide");
                self.close(ind + 2);
                self.line(ind + 1, "</child>");
                self.line(ind + 1, "<child>");
                self.open(ind + 2, "AdwViewStack", Some(&stack), None);
                self.prop(ind + 3, "vexpand", "True");
                if let Some(e) = n.event(&["onChange", "onValueChange"]) {
                    self.signal(ind + 3, "notify::visible-child-name", &e.handler);
                }
                for (i, tab) in n.children.iter().enumerate() {
                    let title = tab.label(&["label"]).unwrap_or("");
                    self.line(ind + 3, "<child>");
                    self.open(ind + 4, "AdwViewStackPage", None, None);
                    self.prop(ind + 5, "name", &format!("{}_{i}", crate::ir::slug(title)));
                    self.tprop(ind + 5, "title", title);
                    if let Some(icon) = tab.str("icon") {
                        self.prop(ind + 5, "icon-name", icon);
                    }
                    self.line(ind + 5, "<property name=\"child\">");
                    let mut page = tab.clone();
                    page.kind = Kind::Box;
                    page.style.direction.get_or_insert(Dir::Column);
                    page.style.gap.get_or_insert(12.0);
                    let kids: Vec<&Node> = tab.children.iter().collect();
                    self.gtk_box(ind + 6, &page, Ctx::root(), true, Vec::new(), &kids);
                    self.line(ind + 5, "</property>");
                    self.close(ind + 4);
                    self.line(ind + 3, "</child>");
                }
                self.close(ind + 2);
                self.line(ind + 1, "</child>");
                self.close(ind);
            }
            Comp::Tab => {
                let kids: Vec<&Node> = n.children.iter().collect();
                self.gtk_box(ind, n, ctx, layout, Vec::new(), &kids);
            }
            Comp::TopAppBar => self.header_bar(ind, n, ctx, layout),
            Comp::NavigationRail => {
                self.open(ind, "GtkBox", None, Some(n));
                lay(self, ind + 1);
                self.prop(ind + 1, "orientation", "vertical");
                self.prop(ind + 1, "spacing", "12");
                self.classes(ind + 1, &["m3-navigation-rail".to_string()]);
                self.css.insert(".m3-navigation-rail { padding: 12px; min-width: 80px; }".into());
                let mut first: Option<String> = None;
                for item in &n.children {
                    let id = self.id(item, "rail_item");
                    self.line(ind + 1, "<child>");
                    self.open(ind + 2, "GtkToggleButton", Some(&id), Some(item));
                    if let Some(f) = &first {
                        self.prop(ind + 3, "group", f);
                    } else {
                        first = Some(id.clone());
                    }
                    if item.flag("selected") || item.flag("active") {
                        self.prop(ind + 3, "active", "True");
                    }
                    self.classes(ind + 3, &["flat".to_string()]);
                    let label = item.label(&["label"]).unwrap_or("");
                    self.tprop(ind + 3, "tooltip-text", label);
                    if let Some(e) = item.event(&["onClick"]) {
                        self.signal(ind + 3, "clicked", &e.handler);
                    }
                    self.line(ind + 3, "<property name=\"child\">");
                    self.open(ind + 4, "GtkBox", None, None);
                    self.prop(ind + 5, "orientation", "vertical");
                    self.prop(ind + 5, "spacing", "4");
                    self.line(ind + 5, "<child>");
                    self.open(ind + 6, "GtkImage", None, None);
                    self.prop(ind + 7, "icon-name", item.str("icon").unwrap_or(""));
                    self.close(ind + 6);
                    self.line(ind + 5, "</child>");
                    self.line(ind + 5, "<child>");
                    self.label_obj(ind + 6, label, &["caption"], Some("0.5"));
                    self.line(ind + 5, "</child>");
                    self.close(ind + 4);
                    self.line(ind + 3, "</property>");
                    self.close(ind + 2);
                    self.line(ind + 1, "</child>");
                }
                self.close(ind);
            }
            Comp::NavigationRailItem => {}
            Comp::Chip => {
                let filter = matches!(n.variant(), "filter" | "input") || n.props.contains_key("selected");
                self.open(ind, if filter { "GtkToggleButton" } else { "GtkButton" }, None, Some(n));
                lay(self, ind + 1);
                sens(self, ind + 1);
                let label = n.label(&["label"]).unwrap_or("");
                if let Some(icon) = n.str("icon") {
                    self.line(ind + 1, "<property name=\"child\">");
                    self.open(ind + 2, "AdwButtonContent", None, None);
                    self.prop(ind + 3, "icon-name", icon);
                    self.tprop(ind + 3, "label", label);
                    self.close(ind + 2);
                    self.line(ind + 1, "</property>");
                } else {
                    self.tprop(ind + 1, "label", label);
                }
                if filter && n.flag("selected") {
                    self.prop(ind + 1, "active", "True");
                }
                self.classes(ind + 1, &["pill".to_string()]);
                if let Some(e) = n.event(&["onClick"]) {
                    self.signal(ind + 1, if filter { "toggled" } else { "clicked" }, &e.handler);
                }
                self.close(ind);
            }
            Comp::Progress => match (n.is_circular_progress(), n.progress_fraction()) {
                (false, Some(f)) => {
                    self.open(ind, "GtkProgressBar", None, Some(n));
                    lay(self, ind + 1);
                    self.prop(ind + 1, "fraction", &format!("{f:.3}"));
                    if let Some(a) = &n.aria_label {
                        self.a11y_label(ind + 1, a);
                    }
                    self.close(ind);
                }
                _ => {
                    self.open(ind, "AdwSpinner", None, Some(n));
                    lay(self, ind + 1);
                    self.close(ind);
                }
            },
        }
    }

    fn switch_body(&mut self, ind: usize, n: &Node, signal: &str) {
        if n.flag("checked") || n.flag("selected") {
            self.prop(ind, "active", "True");
        }
        if n.flag("disabled") {
            self.prop(ind, "sensitive", "False");
        }
        if let Some(e) = n.event(&["onChange", "onCheckedChange", "onClick"]) {
            self.signal(ind, signal, &e.handler);
        }
    }

    fn entry_body(&mut self, ind: usize, n: &Node, password: bool, row: bool) {
        if let Some(v) = n.str("value") {
            self.prop(ind, "text", v);
        }
        if !password {
            match n.str("type") {
                Some("email") => self.prop(ind, "input-purpose", "email"),
                Some("number") => self.prop(ind, "input-purpose", "number"),
                Some("tel") => self.prop(ind, "input-purpose", "phone"),
                Some("url") => self.prop(ind, "input-purpose", "url"),
                _ => {}
            }
        }
        if n.flag("disabled") {
            self.prop(ind, "sensitive", "False");
        }
        if let Some(e) = n.event(&["onChange", "onInput", "onValueChange"]) {
            self.signal(ind, "changed", &e.handler);
        }
        if let Some(e) = n.event(&["onSubmit", "onEnter"]) {
            self.signal(ind, if row { "entry-activated" } else { "activate" }, &e.handler);
        }
    }
}

pub fn emit(doc: &Document) -> Output {
    emit_with_theme(doc, YoloThemePreset::Neutral)
}

pub fn emit_with_theme(doc: &Document, theme: YoloThemePreset) -> Output {
    let mut g = G { out: String::new(), next: HashMap::new(), groups: HashMap::new(), css: BTreeSet::new() };
    g.line(0, "<?xml version=\"1.0\" encoding=\"UTF-8\"?>");
    g.line(
        0,
        &format!(
            "<!-- Generated by yolo-migrator from {} (component {}, theme {}). Do not hand-edit. -->",
            esc(&doc.source),
            esc(&doc.component),
            theme.id()
        ),
    );
    g.line(0, "<interface>");
    g.line(1, "<requires lib=\"gtk\" version=\"4.16\"/>");
    g.line(1, "<requires lib=\"libadwaita\" version=\"1.6\"/>");
    if !doc.handlers.is_empty() {
        let list: Vec<String> = doc.handlers.iter().map(|h| snake_handler(h)).collect();
        g.line(1, &format!("<!-- handlers: {} -->", list.join(", ")));
    }
    let mut root = doc.root.clone();
    if root.id.is_none() {
        root.id = Some("root".into());
    }
    g.object(1, &root, Ctx::root());
    g.line(0, "</interface>");
    let css = (!g.css.is_empty()).then(|| {
        let mut s = format!(
            "/* Generated by yolo-migrator from {} (theme {}). */\n",
            doc.source,
            theme.id()
        );
        for rule in &g.css {
            s.push_str(rule);
            s.push('\n');
        }
        s
    });
    Output { ui: g.out, css }
}
