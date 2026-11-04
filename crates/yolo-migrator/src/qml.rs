// SPDX-License-Identifier: Apache-2.0
//! QML backend for Qt 6.8+ Qt Quick Controls (Material style) and Kirigami (catalog column `qt`).

use std::collections::BTreeSet;
use std::fmt::Write;

use yolo_shared::YoloThemePreset;

use crate::ir::{Align, Comp, Dir, Document, Justify, Kind, Node, camel, qml_signal, typescale_metrics};

#[derive(Clone, Copy)]
struct Ctx {
    parent: Option<Dir>,
    align: Option<Align>,
    expand_main: bool,
    in_list: bool,
}

struct Q {
    out: String,
    material: bool,
    kirigami: bool,
    next_id: usize,
    groups: BTreeSet<String>,
}

pub fn js_str(s: &str) -> String {
    let mut out = String::from("\"");
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn tr(s: &str) -> String {
    format!("qsTr({})", js_str(s))
}

fn fmt_num(n: f64) -> String {
    if n.fract() == 0.0 { format!("{}", n as i64) } else { format!("{n}") }
}

impl Q {
    fn line(&mut self, ind: usize, s: &str) {
        let _ = writeln!(self.out, "{:ind$}{s}", "", ind = ind * 4);
    }

    fn fresh(&mut self, base: &str) -> String {
        self.next_id += 1;
        format!("{base}{}", self.next_id)
    }

    fn handler(&mut self, ind: usize, qml_prop: &str, h: &str) {
        self.line(ind, &format!("{qml_prop}: root.{}()", qml_signal(h)));
    }

    fn catalog(&mut self, ind: usize, n: &Node) {
        if let Some(k) = n.catalog_key() {
            self.line(ind, &format!("// catalog: components.{k}"));
        }
    }

    fn layout(&mut self, ind: usize, n: &Node, ctx: Ctx, fill_cross_default: bool) {
        let Some(parent) = ctx.parent else {
            if let Some(w) = n.style.width {
                self.line(ind, &format!("implicitWidth: {}", fmt_num(w)));
            }
            if let Some(h) = n.style.height {
                self.line(ind, &format!("implicitHeight: {}", fmt_num(h)));
            }
            return;
        };
        let grow = n.style.grow || ctx.expand_main;
        let stretch = ctx.align.is_none_or(|a| a == Align::Stretch);
        match parent {
            Dir::Column => {
                if (stretch && fill_cross_default) || n.style.max_width.is_some() || n.style.fill_width {
                    self.line(ind, "Layout.fillWidth: true");
                }
                if grow || n.style.scroll || n.style.fill_height {
                    self.line(ind, "Layout.fillHeight: true");
                }
            }
            Dir::Row => {
                if grow || n.style.fill_width {
                    self.line(ind, "Layout.fillWidth: true");
                }
                let container = matches!(n.kind, Kind::Box | Kind::Comp(Comp::Card | Comp::List | Comp::NavigationRail | Comp::Tabs));
                if (stretch && container) || n.style.fill_height {
                    self.line(ind, "Layout.fillHeight: true");
                }
            }
        }
        let align = match (parent, ctx.align) {
            (Dir::Column, Some(Align::Center)) => Some("Qt.AlignHCenter"),
            (Dir::Column, Some(Align::End)) => Some("Qt.AlignRight"),
            (Dir::Row, Some(Align::Center)) => Some("Qt.AlignVCenter"),
            (Dir::Row, Some(Align::End)) => Some("Qt.AlignBottom"),
            _ => None,
        };
        if let Some(a) = align {
            self.line(ind, &format!("Layout.alignment: {a}"));
        }
        if let Some(m) = n.style.margin {
            for (i, side) in ["topMargin", "rightMargin", "bottomMargin", "leftMargin"].iter().enumerate() {
                if m[i] != 0.0 {
                    self.line(ind, &format!("Layout.{side}: {}", fmt_num(m[i])));
                }
            }
        }
        if let Some(w) = n.style.width {
            self.line(ind, &format!("Layout.preferredWidth: {}", fmt_num(w)));
        }
        if let Some(h) = n.style.height {
            self.line(ind, &format!("Layout.preferredHeight: {}", fmt_num(h)));
        }
        if let Some(w) = n.style.max_width {
            self.line(ind, &format!("Layout.maximumWidth: {}", fmt_num(w)));
            if parent == Dir::Column && align.is_none() {
                self.line(ind, "Layout.alignment: Qt.AlignHCenter");
            }
        }
    }

    fn padding(&mut self, ind: usize, p: [f64; 4]) {
        if p.iter().all(|v| *v == p[0]) {
            self.line(ind, &format!("padding: {}", fmt_num(p[0])));
        } else {
            for (i, side) in ["topPadding", "rightPadding", "bottomPadding", "leftPadding"].iter().enumerate() {
                self.line(ind, &format!("{side}: {}", fmt_num(p[i])));
            }
        }
    }

    fn item(&mut self, ind: usize, n: &Node, ctx: Ctx, root: bool) {
        if n.style.scroll {
            let id = self.fresh("scroll");
            self.line(ind, "ScrollView {");
            if root {
                self.root_decls(ind + 1);
            }
            self.line(ind + 1, &format!("id: {id}"));
            self.layout(ind + 1, n, ctx, true);
            self.line(ind + 1, "clip: true");
            self.line(ind + 1, "contentWidth: availableWidth");
            let mut inner = n.clone();
            inner.style.scroll = false;
            inner.style.grow = false;
            inner.style.margin = None;
            inner.style.width = None;
            inner.style.height = None;
            inner.style.max_width = None;
            self.item_inner(ind + 1, &inner, Ctx { parent: None, ..ctx }, false, Some(&format!("width: {id}.availableWidth")));
            self.line(ind, "}");
        } else {
            self.item_inner(ind, n, ctx, root, None);
        }
    }

    fn root_decls(&mut self, ind: usize) {
        self.line(ind, "id: root");
        self.line(ind, "//@@ROOT_DECLS@@");
    }

    fn open(&mut self, ind: usize, ty: &str, n: &Node, root: bool) {
        self.catalog(ind, n);
        self.line(ind, &format!("{ty} {{"));
        if root {
            self.root_decls(ind + 1);
        }
    }

    fn color_line(&mut self, ind: usize, n: &Node) {
        if let Some(c) = &n.style.color {
            self.line(ind, &format!("color: M3Theme.{}", camel(c)));
        }
    }

    fn layout_box(&mut self, ind: usize, n: &Node, ctx: Ctx, root: bool, extra: Option<&str>, kids: &[&Node]) {
        let dir = n.dir();
        let ty = if dir == Dir::Row { "RowLayout" } else { "ColumnLayout" };
        let decorated = n.style.padding.is_some() || n.style.background.is_some() || n.is(Comp::Card);
        let mut ind2 = ind;
        if decorated {
            self.open(ind, "Pane", n, root);
            self.layout(ind + 1, n, ctx, true);
            if let Some(l) = extra {
                self.line(ind + 1, l);
            }
            if let Some(p) = n.style.padding {
                self.padding(ind + 1, p);
            } else if n.is(Comp::Card) {
                self.line(ind + 1, "padding: 16");
            }
            if n.is(Comp::Card) {
                self.material = true;
                let elevation = if n.variant() == "elevated" { 1 } else { 0 };
                self.line(ind + 1, &format!("Material.elevation: {elevation}"));
            }
            let radius = n.style.shape.as_deref().and_then(crate::ir::shape_radius).unwrap_or(if n.is(Comp::Card) { 12.0 } else { 0.0 });
            let bg = n.style.background.clone().or_else(|| n.is(Comp::Card).then(|| {
                if n.variant() == "outlined" { "surface".to_string() } else { "surface-container-highest".to_string() }
            }));
            match bg {
                Some(b) => {
                    let border = if n.variant() == "outlined" { "; border.color: M3Theme.outlineVariant; border.width: 1" } else { "" };
                    self.line(ind + 1, &format!("background: Rectangle {{ color: M3Theme.{}; radius: {}{border} }}", camel(&b), fmt_num(radius)));
                }
                None => self.line(ind + 1, "background: null"),
            }
            ind2 = ind + 1;
            self.line(ind2, &format!("{ty} {{"));
            self.line(ind2 + 1, "anchors.fill: parent");
        } else {
            self.open(ind, ty, n, root);
            self.layout(ind + 1, n, ctx, true);
            if let Some(l) = extra {
                self.line(ind + 1, l);
            }
        }
        self.line(ind2 + 1, &format!("spacing: {}", fmt_num(n.style.gap.unwrap_or(0.0))));
        let count = kids.len();
        for (i, k) in kids.iter().enumerate() {
            let spread = n.style.justify == Some(Justify::SpaceBetween) && count > 1;
            let c = Ctx { parent: Some(dir), align: n.style.align, expand_main: false, in_list: false };
            if n.style.justify == Some(Justify::Center) && i == 0 || n.style.justify == Some(Justify::End) && i == 0 {
                self.line(ind2 + 1, &format!("Item {{ {}: true }}", if dir == Dir::Row { "Layout.fillWidth" } else { "Layout.fillHeight" }));
            }
            self.item(ind2 + 1, k, c, false);
            if spread && i + 1 < count {
                self.line(ind2 + 1, &format!("Item {{ {}: true }}", if dir == Dir::Row { "Layout.fillWidth" } else { "Layout.fillHeight" }));
            }
            if n.style.justify == Some(Justify::Center) && i == count - 1 {
                self.line(ind2 + 1, &format!("Item {{ {}: true }}", if dir == Dir::Row { "Layout.fillWidth" } else { "Layout.fillHeight" }));
            }
        }
        if decorated {
            self.line(ind2, "}");
        }
        self.line(ind, "}");
    }

    fn tool_bar(&mut self, ind: usize, n: &Node, ctx: Ctx, as_header: bool) {
        if as_header {
            self.line(ind, "header: ToolBar {");
            self.catalog(ind + 1, n);
        } else {
            self.open(ind, "ToolBar", n, false);
            self.layout(ind + 1, n, ctx, true);
        }
        self.line(ind + 1, "RowLayout {");
        self.line(ind + 2, "anchors.fill: parent");
        let row = Ctx { parent: Some(Dir::Row), align: Some(Align::Center), expand_main: false, in_list: false };
        for k in n.children.iter().filter(|k| matches!(k.slot.as_deref(), Some("start" | "navigation" | "leading"))) {
            self.item(ind + 2, k, row, false);
        }
        self.line(ind + 2, "Label {");
        self.line(ind + 3, &format!("text: {}", tr(n.label(&["title", "headline"]).unwrap_or(""))));
        self.line(ind + 3, "font.pixelSize: 22");
        self.line(ind + 3, "elide: Label.ElideRight");
        self.line(ind + 3, "Layout.fillWidth: true");
        if n.variant() == "center" {
            self.line(ind + 3, "horizontalAlignment: Qt.AlignHCenter");
        }
        self.line(ind + 2, "}");
        for k in n.children.iter().filter(|k| !matches!(k.slot.as_deref(), Some("start" | "navigation" | "leading"))) {
            self.item(ind + 2, k, row, false);
        }
        self.line(ind + 1, "}");
        self.line(ind, "}");
    }

    fn label(&mut self, ind: usize, text: &str, typescale: &str, n: Option<&Node>, ctx: Ctx) {
        self.line(ind, "Label {");
        self.line(ind + 1, &format!("text: {}", tr(text)));
        if let Some((size, weight)) = typescale_metrics(typescale) {
            self.line(ind + 1, &format!("font.pixelSize: {}", fmt_num(size)));
            if weight == 500 {
                self.line(ind + 1, "font.weight: Font.Medium");
            }
        }
        self.line(ind + 1, "wrapMode: Text.WordWrap");
        if let Some(n) = n {
            self.layout(ind + 1, n, ctx, true);
            self.color_line(ind + 1, n);
            match n.style.text_align {
                Some(Align::Center) => self.line(ind + 1, "horizontalAlignment: Text.AlignHCenter"),
                Some(Align::End) => self.line(ind + 1, "horizontalAlignment: Text.AlignRight"),
                _ => {}
            }
        } else if ctx.parent == Some(Dir::Column) {
            self.line(ind + 1, "Layout.fillWidth: true");
        }
        self.line(ind, "}");
    }

    fn item_inner(&mut self, ind: usize, n: &Node, ctx: Ctx, root: bool, extra: Option<&str>) {
        match n.kind {
            Kind::Box => {
                if n.dir() == Dir::Column && n.children.first().is_some_and(|c| c.is(Comp::TopAppBar)) {
                    self.line(ind, "Page {");
                    if root {
                        self.root_decls(ind + 1);
                    }
                    self.layout(ind + 1, n, ctx, true);
                    self.tool_bar(ind + 1, &n.children[0], ctx, true);
                    let mut rest = n.clone();
                    rest.children.remove(0);
                    rest.style.grow = false;
                    rest.style.margin = None;
                    rest.style.width = None;
                    rest.style.height = None;
                    rest.id = None;
                    let kids: Vec<&Node> = rest.children.iter().collect();
                    self.layout_box(ind + 1, &rest, Ctx { parent: None, ..ctx }, false, Some("anchors.fill: parent"), &kids);
                    self.line(ind, "}");
                } else {
                    let kids: Vec<&Node> = n.children.iter().collect();
                    self.layout_box(ind, n, ctx, root, extra, &kids);
                }
            }
            Kind::Text(_) => {
                if root {
                    self.line(ind, "Label {");
                    self.root_decls(ind + 1);
                    self.line(ind + 1, &format!("text: {}", tr(n.text.as_deref().unwrap_or(""))));
                    self.line(ind, "}");
                } else {
                    let ts = n.style.typescale.clone().unwrap_or_else(|| "body-medium".into());
                    self.label(ind, n.text.as_deref().unwrap_or(""), &ts, Some(n), ctx);
                }
            }
            Kind::Image => {
                self.open(ind, "Image", n, root);
                self.layout(ind + 1, n, ctx, false);
                if let Some(src) = n.str("src") {
                    self.line(ind + 1, &format!("source: {}", js_str(src)));
                }
                self.line(ind + 1, "fillMode: Image.PreserveAspectFit");
                if let Some(w) = n.num("width") {
                    self.line(ind + 1, &format!("sourceSize.width: {}", fmt_num(w)));
                }
                if let Some(alt) = n.str("alt") {
                    self.line(ind + 1, &format!("Accessible.name: {}", tr(alt)));
                }
                self.line(ind, "}");
            }
            Kind::Comp(c) => self.comp(ind, n, c, ctx, root, extra),
        }
    }

    fn common(&mut self, ind: usize, n: &Node) {
        if n.flag("disabled") {
            self.line(ind, "enabled: false");
        }
        if let Some(a) = &n.aria_label {
            self.line(ind, &format!("Accessible.name: {}", tr(a)));
            self.line(ind, "ToolTip.visible: hovered");
            self.line(ind, &format!("ToolTip.text: {}", tr(a)));
        }
    }

    fn comp(&mut self, ind: usize, n: &Node, c: Comp, ctx: Ctx, root: bool, extra: Option<&str>) {
        let fill = |c: Comp| matches!(c, Comp::TextField | Comp::Slider | Comp::Progress | Comp::List | Comp::Divider | Comp::Tabs | Comp::Card);
        match c {
            Comp::Button => {
                self.open(ind, "Button", n, root);
                self.layout(ind + 1, n, ctx, false);
                if let Some(l) = n.label(&["label"]) {
                    self.line(ind + 1, &format!("text: {}", tr(l)));
                }
                if let Some(i) = n.str("icon") {
                    self.line(ind + 1, &format!("icon.name: {}", js_str(i)));
                }
                match n.variant() {
                    "tonal" => {
                        self.material = true;
                        self.line(ind + 1, "Material.background: M3Theme.secondaryContainer");
                        self.line(ind + 1, "Material.foreground: M3Theme.onSecondaryContainer");
                    }
                    "outlined" => {
                        self.line(ind + 1, "flat: true");
                    }
                    "text" => self.line(ind + 1, "flat: true"),
                    "elevated" => {
                        self.material = true;
                        self.line(ind + 1, "Material.elevation: 1");
                    }
                    _ => self.line(ind + 1, "highlighted: true"),
                }
                self.common(ind + 1, n);
                if let Some(e) = n.event(&["onClick"]) {
                    self.handler(ind + 1, "onClicked", &e.handler);
                }
                self.line(ind, "}");
            }
            Comp::IconButton | Comp::Fab => {
                let round = c == Comp::Fab || matches!(n.variant(), "filled" | "tonal" | "outlined");
                self.open(ind, if round { "RoundButton" } else { "ToolButton" }, n, root);
                self.layout(ind + 1, n, ctx, false);
                self.line(ind + 1, &format!("icon.name: {}", js_str(n.str("icon").unwrap_or(""))));
                match n.label(&["label"]) {
                    Some(l) => self.line(ind + 1, &format!("text: {}", tr(l))),
                    None => {
                        if let Some(a) = &n.aria_label {
                            self.line(ind + 1, &format!("text: {}", tr(a)));
                        }
                        self.line(ind + 1, "display: AbstractButton.IconOnly");
                    }
                }
                if c == Comp::Fab || n.variant() == "filled" {
                    self.line(ind + 1, "highlighted: true");
                }
                if n.props.contains_key("selected") {
                    self.line(ind + 1, "checkable: true");
                    self.line(ind + 1, &format!("checked: {}", n.flag("selected")));
                }
                self.common(ind + 1, n);
                if let Some(e) = n.event(&["onClick"]) {
                    self.handler(ind + 1, "onClicked", &e.handler);
                }
                self.line(ind, "}");
            }
            Comp::Switch | Comp::Checkbox | Comp::Radio => {
                let ty = match (c, ctx.in_list) {
                    (Comp::Switch, true) => "SwitchDelegate",
                    (Comp::Switch, false) => "Switch",
                    (Comp::Checkbox, _) => "CheckBox",
                    _ => "RadioButton",
                };
                self.open(ind, ty, n, root);
                self.layout(ind + 1, n, ctx, ctx.in_list);
                if let Some(l) = n.label(&["label"]) {
                    self.line(ind + 1, &format!("text: {}", tr(l)));
                }
                if n.flag("checked") || n.flag("selected") {
                    self.line(ind + 1, "checked: true");
                }
                if c == Comp::Radio
                    && let Some(g) = n.str("name")
                {
                    let gid = format!("group_{}", crate::ir::slug(g));
                    self.groups.insert(gid.clone());
                    self.line(ind + 1, &format!("ButtonGroup.group: {gid}"));
                }
                self.common(ind + 1, n);
                if let Some(e) = n.event(&["onChange", "onCheckedChange", "onClick"]) {
                    self.handler(ind + 1, "onToggled", &e.handler);
                }
                self.line(ind, "}");
            }
            Comp::Slider => {
                let label = n.label(&["label"]);
                let inner = if let Some(l) = label {
                    self.catalog(ind, n);
                    self.line(ind, "ColumnLayout {");
                    if root {
                        self.root_decls(ind + 1);
                    }
                    self.layout(ind + 1, n, ctx, true);
                    self.line(ind + 1, "spacing: 4");
                    self.label(ind + 1, l, "label-large", None, Ctx { parent: Some(Dir::Column), ..ctx });
                    self.line(ind + 1, "Slider {");
                    self.line(ind + 2, "Layout.fillWidth: true");
                    ind + 1
                } else {
                    self.open(ind, "Slider", n, root);
                    self.layout(ind + 1, n, ctx, true);
                    ind
                };
                self.line(inner + 1, &format!("from: {}", fmt_num(n.num("min").unwrap_or(0.0))));
                self.line(inner + 1, &format!("to: {}", fmt_num(n.num("max").unwrap_or(100.0))));
                self.line(inner + 1, &format!("value: {}", fmt_num(n.num("value").unwrap_or(0.0))));
                self.line(inner + 1, &format!("stepSize: {}", fmt_num(n.num("step").unwrap_or(1.0))));
                if let Some(l) = label {
                    self.line(inner + 1, &format!("Accessible.name: {}", tr(l)));
                }
                if n.flag("disabled") {
                    self.line(inner + 1, "enabled: false");
                }
                if let Some(e) = n.event(&["onChange", "onInput", "onValueChange"]) {
                    self.handler(inner + 1, "onMoved", &e.handler);
                }
                self.line(inner, "}");
                if label.is_some() {
                    self.line(ind, "}");
                }
            }
            Comp::TextField => {
                self.open(ind, "TextField", n, root);
                self.layout(ind + 1, n, ctx, true);
                if let Some(p) = n.str("label").or(n.str("placeholder")) {
                    self.line(ind + 1, &format!("placeholderText: {}", tr(p)));
                }
                if let Some(v) = n.str("value") {
                    self.line(ind + 1, &format!("text: {}", js_str(v)));
                }
                match n.str("type") {
                    Some("password") => self.line(ind + 1, "echoMode: TextInput.Password"),
                    Some("email") => self.line(ind + 1, "inputMethodHints: Qt.ImhEmailCharactersOnly"),
                    Some("number") => self.line(ind + 1, "inputMethodHints: Qt.ImhFormattedNumbersOnly"),
                    Some("url") => self.line(ind + 1, "inputMethodHints: Qt.ImhUrlCharactersOnly"),
                    _ => {}
                }
                self.material = true;
                let style = if n.variant() == "outlined" { "Material.Outlined" } else { "Material.Filled" };
                self.line(ind + 1, &format!("Material.containerStyle: {style}"));
                if let Some(l) = n.str("label") {
                    self.line(ind + 1, &format!("Accessible.name: {}", tr(l)));
                }
                if n.flag("disabled") {
                    self.line(ind + 1, "enabled: false");
                }
                if let Some(e) = n.event(&["onChange", "onInput", "onValueChange"]) {
                    self.handler(ind + 1, "onTextEdited", &e.handler);
                }
                if let Some(e) = n.event(&["onSubmit", "onEnter"]) {
                    self.handler(ind + 1, "onAccepted", &e.handler);
                }
                self.line(ind, "}");
                if let Some(s) = n.str("supportingText") {
                    self.label(ind, s, "body-small", None, ctx);
                }
            }
            Comp::Card => {
                let mut card = n.clone();
                card.style.direction.get_or_insert(Dir::Column);
                let kids: Vec<&Node> = n.children.iter().collect();
                self.layout_box(ind, &card, ctx, root, extra, &kids);
            }
            Comp::Dialog => {
                self.open(ind, "Dialog", n, root);
                if let Some(t) = n.label(&["title", "headline"]) {
                    self.line(ind + 1, &format!("title: {}", tr(t)));
                }
                self.line(ind + 1, "modal: true");
                if let Some(w) = n.style.width {
                    self.line(ind + 1, &format!("width: {}", fmt_num(w)));
                }
                if let Some(e) = n.event(&["onClose", "onOpenChange"]) {
                    self.handler(ind + 1, "onRejected", &e.handler);
                }
                let (actions, content): (Vec<&Node>, Vec<&Node>) = n.children.iter().partition(|k| k.slot.as_deref() == Some("actions"));
                let mut body = n.clone();
                body.kind = Kind::Box;
                body.id = None;
                body.style.width = None;
                body.style.padding = None;
                body.style.background = None;
                body.style.direction = Some(Dir::Column);
                body.style.gap.get_or_insert(16.0);
                self.layout_box(ind + 1, &body, Ctx { parent: None, ..ctx }, false, Some("anchors.fill: parent"), &content);
                if !actions.is_empty() {
                    self.line(ind + 1, "footer: DialogButtonBox {");
                    for a in actions {
                        self.item(ind + 2, a, Ctx { parent: None, align: None, expand_main: false, in_list: false }, false);
                    }
                    self.line(ind + 1, "}");
                }
                self.line(ind, "}");
            }
            Comp::List => {
                self.open(ind, "ColumnLayout", n, root);
                self.layout(ind + 1, n, ctx, true);
                self.line(ind + 1, "spacing: 0");
                for k in &n.children {
                    self.item(ind + 1, k, Ctx { parent: Some(Dir::Column), align: None, expand_main: false, in_list: true }, false);
                }
                self.line(ind, "}");
            }
            Comp::ListItem => {
                let title = n.label(&["headline"]).unwrap_or("");
                if n.children.len() == 1 && n.children[0].is(Comp::Switch) {
                    let mut sw = n.children[0].clone();
                    sw.props.insert("label".into(), crate::ir::Value::Str(title.into()));
                    self.comp(ind, &sw, Comp::Switch, Ctx { in_list: true, ..ctx }, root, None);
                    return;
                }
                self.open(ind, "ItemDelegate", n, root);
                self.layout(ind + 1, n, ctx, true);
                self.line(ind + 1, &format!("Accessible.name: {}", tr(title)));
                if let Some(e) = n.event(&["onClick"]) {
                    self.handler(ind + 1, "onClicked", &e.handler);
                }
                self.line(ind + 1, "contentItem: RowLayout {");
                self.line(ind + 2, "spacing: 16");
                if let Some(icon) = n.str("icon").or(n.str("leadingIcon")) {
                    self.kirigami = true;
                    self.line(ind + 2, &format!("Kirigami.Icon {{ source: {}; Layout.preferredWidth: 24; Layout.preferredHeight: 24 }}", js_str(icon)));
                }
                self.line(ind + 2, "ColumnLayout {");
                self.line(ind + 3, "spacing: 0");
                self.line(ind + 3, "Layout.fillWidth: true");
                self.line(ind + 3, &format!("Label {{ text: {}; font.pixelSize: 16; elide: Label.ElideRight; Layout.fillWidth: true }}", tr(title)));
                if let Some(s) = n.str("supportingText") {
                    self.line(ind + 3, &format!("Label {{ text: {}; font.pixelSize: 14; color: M3Theme.onSurfaceVariant; elide: Label.ElideRight; Layout.fillWidth: true }}", tr(s)));
                }
                self.line(ind + 2, "}");
                if let Some(t) = n.str("trailingText") {
                    self.line(ind + 2, &format!("Label {{ text: {}; font.pixelSize: 11; color: M3Theme.onSurfaceVariant }}", tr(t)));
                }
                for k in &n.children {
                    self.item(ind + 2, k, Ctx { parent: Some(Dir::Row), align: Some(Align::Center), expand_main: false, in_list: false }, false);
                }
                self.line(ind + 1, "}");
                self.line(ind, "}");
            }
            Comp::Divider => {
                let ty = if ctx.parent == Some(Dir::Row) { "ToolSeparator" } else { "MenuSeparator" };
                self.open(ind, ty, n, root);
                self.layout(ind + 1, n, ctx, true);
                self.line(ind, "}");
            }
            Comp::Tabs => {
                let bar = self.fresh("tabs");
                self.open(ind, "ColumnLayout", n, root);
                self.layout(ind + 1, n, ctx, true);
                self.line(ind + 1, "spacing: 0");
                self.line(ind + 1, "TabBar {");
                self.line(ind + 2, &format!("id: {bar}"));
                self.line(ind + 2, "Layout.fillWidth: true");
                if let Some(i) = n.num("activeIndex") {
                    self.line(ind + 2, &format!("currentIndex: {}", fmt_num(i)));
                }
                if let Some(e) = n.event(&["onChange", "onValueChange"]) {
                    self.handler(ind + 2, "onCurrentIndexChanged", &e.handler);
                }
                for tab in &n.children {
                    let mut l = format!("TabButton {{ text: {}", tr(tab.label(&["label"]).unwrap_or("")));
                    if let Some(icon) = tab.str("icon") {
                        let _ = write!(l, "; icon.name: {}", js_str(icon));
                    }
                    l.push_str(" }");
                    self.line(ind + 2, &l);
                }
                self.line(ind + 1, "}");
                self.line(ind + 1, "StackLayout {");
                self.line(ind + 2, &format!("currentIndex: {bar}.currentIndex"));
                self.line(ind + 2, "Layout.fillWidth: true");
                self.line(ind + 2, "Layout.fillHeight: true");
                for tab in &n.children {
                    let mut page = tab.clone();
                    page.kind = Kind::Box;
                    page.style.direction.get_or_insert(Dir::Column);
                    page.style.gap.get_or_insert(12.0);
                    let kids: Vec<&Node> = tab.children.iter().collect();
                    self.layout_box(ind + 2, &page, Ctx { parent: None, ..ctx }, false, None, &kids);
                }
                self.line(ind + 1, "}");
                self.line(ind, "}");
            }
            Comp::Tab => {
                let kids: Vec<&Node> = n.children.iter().collect();
                self.layout_box(ind, n, ctx, root, extra, &kids);
            }
            Comp::TopAppBar => self.tool_bar(ind, n, ctx, false),
            Comp::NavigationRail => {
                self.line(ind, "ColumnLayout {");
                if root {
                    self.root_decls(ind + 1);
                }
                self.layout(ind + 1, n, ctx, false);
                self.line(ind + 1, "spacing: 12");
                for item in &n.children {
                    self.line(ind + 1, "ToolButton {");
                    self.line(ind + 2, &format!("icon.name: {}", js_str(item.str("icon").unwrap_or(""))));
                    self.line(ind + 2, &format!("text: {}", tr(item.label(&["label"]).unwrap_or(""))));
                    self.line(ind + 2, "display: AbstractButton.TextUnderIcon");
                    self.line(ind + 2, "checkable: true");
                    self.line(ind + 2, "autoExclusive: true");
                    self.line(ind + 2, "Layout.fillWidth: true");
                    if item.flag("selected") || item.flag("active") {
                        self.line(ind + 2, "checked: true");
                    }
                    if let Some(e) = item.event(&["onClick"]) {
                        self.handler(ind + 2, "onClicked", &e.handler);
                    }
                    self.line(ind + 1, "}");
                }
                self.line(ind + 1, "Item { Layout.fillHeight: true }");
                self.line(ind, "}");
            }
            Comp::NavigationRailItem => {}
            Comp::Chip => {
                self.kirigami = true;
                self.open(ind, "Kirigami.Chip", n, root);
                self.layout(ind + 1, n, ctx, false);
                self.line(ind + 1, &format!("text: {}", tr(n.label(&["label"]).unwrap_or(""))));
                if let Some(i) = n.str("icon") {
                    self.line(ind + 1, &format!("icon.name: {}", js_str(i)));
                }
                let filter = matches!(n.variant(), "filter" | "input") || n.props.contains_key("selected");
                self.line(ind + 1, &format!("checkable: {filter}"));
                if filter {
                    self.line(ind + 1, &format!("checked: {}", n.flag("selected")));
                }
                self.line(ind + 1, &format!("closable: {}", n.variant() == "input"));
                if let Some(e) = n.event(&["onClick"]) {
                    self.handler(ind + 1, "onClicked", &e.handler);
                }
                self.line(ind, "}");
            }
            Comp::Progress => {
                if n.is_circular_progress() {
                    self.open(ind, "BusyIndicator", n, root);
                    self.layout(ind + 1, n, ctx, false);
                    self.line(ind + 1, "running: true");
                } else {
                    self.open(ind, "ProgressBar", n, root);
                    self.layout(ind + 1, n, ctx, fill(c));
                    match n.progress_fraction() {
                        Some(f) => self.line(ind + 1, &format!("value: {f:.3}")),
                        None => self.line(ind + 1, "indeterminate: true"),
                    }
                    if let Some(a) = &n.aria_label {
                        self.line(ind + 1, &format!("Accessible.name: {}", tr(a)));
                    }
                }
                self.line(ind, "}");
            }
        }
    }
}

pub fn emit(doc: &Document) -> String {
    emit_with_theme(doc, YoloThemePreset::Neutral)
}

pub fn emit_with_theme(doc: &Document, theme: YoloThemePreset) -> String {
    let mut q = Q { out: String::new(), material: false, kirigami: false, next_id: 0, groups: BTreeSet::new() };
    q.item(0, &doc.root, Ctx { parent: None, align: None, expand_main: false, in_list: false }, true);
    let mut decls = String::new();
    for h in &doc.handlers {
        let _ = writeln!(decls, "    signal {}()", qml_signal(h));
    }
    for g in &q.groups {
        let _ = writeln!(decls, "    ButtonGroup {{ id: {g} }}");
    }
    let body = q.out.replacen("    //@@ROOT_DECLS@@\n", &decls, 1);
    let mut out = format!(
        "// Generated by yolo-migrator from {} (component {}, theme {}). Do not hand-edit.\n",
        doc.source, doc.component, theme.id()
    );
    out.push_str("import QtQuick\nimport QtQuick.Controls\nimport QtQuick.Layouts\n");
    if q.material {
        out.push_str("import QtQuick.Controls.Material\n");
    }
    if q.kirigami {
        out.push_str("import org.kde.kirigami as Kirigami\n");
    }
    out.push('\n');
    out.push_str(&body);
    out
}
