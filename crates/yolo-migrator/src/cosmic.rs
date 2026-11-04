// SPDX-License-Identifier: Apache-2.0
//! libcosmic / Iced Elm MVU Rust view emitter (`MigrationTarget::Cosmic`).
//!
//! Emits a `Message` enum, a `State` struct, and `pub fn view<'a>(state: &'a State) -> Element<'a, Message>`
//! compatible with Iced 0.14 / libcosmic and `yolo-iced` themed via `yolo-shared`.

use std::fmt::Write;

use yolo_shared::YoloThemePreset;

use crate::ir::{Align, Comp, Dir, Document, Kind, Node, slug, snake_handler, typescale_metrics};

struct CosmicEmitter {
    out: String,
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
        "Action".to_string()
    } else {
        out
    }
}

impl CosmicEmitter {
    fn line(&mut self, ind: usize, s: &str) {
        let _ = writeln!(self.out, "{:ind$}{s}", "", ind = ind * 4);
    }

    fn emit_expr(&mut self, ind: usize, n: &Node, trailing_comma: bool) {
        let suffix = if trailing_comma { "," } else { "" };
        match n.kind {
            Kind::Box | Kind::Comp(Comp::Card | Comp::Dialog | Comp::Tabs | Comp::Tab | Comp::List | Comp::TopAppBar | Comp::NavigationRail) => {
                let dir = n.dir();
                let macro_name = if dir == Dir::Row { "row!" } else { "column!" };
                self.line(ind, &format!("container({macro_name}["));
                for child in &n.children {
                    self.emit_expr(ind + 1, child, true);
                }
                let mut chain = String::from("]");
                if let Some(g) = n.style.gap {
                    let _ = write!(chain, ".spacing({})", f32_lit(g));
                }
                if let Some(a) = n.style.align {
                    let al = match a {
                        Align::Start => "Alignment::Start",
                        Align::Center => "Alignment::Center",
                        Align::End => "Alignment::End",
                        Align::Stretch => "Alignment::Start",
                    };
                    let _ = write!(chain, ".align_y({al})");
                }
                chain.push(')');
                if let Some(p) = n.style.padding {
                    let _ = write!(chain, ".padding([{}, {}, {}, {}])", f32_lit(p[0]), f32_lit(p[1]), f32_lit(p[2]), f32_lit(p[3]));
                } else if n.is(Comp::Card) {
                    chain.push_str(".padding(16.0)");
                }
                if n.style.fill_width || n.style.grow {
                    chain.push_str(".width(Length::Fill)");
                } else if let Some(w) = n.style.width {
                    let _ = write!(chain, ".width(Length::Fixed({}))", f32_lit(w));
                }
                if n.style.fill_height {
                    chain.push_str(".height(Length::Fill)");
                } else if let Some(h) = n.style.height {
                    let _ = write!(chain, ".height(Length::Fixed({}))", f32_lit(h));
                }
                let _ = write!(chain, ".into(){suffix}");
                self.line(ind, &chain);
            }
            Kind::Text(_) => {
                let txt = n.text.as_deref().unwrap_or("");
                let mut expr = format!("text({})", lit(txt));
                if let Some((sz, _)) = n.style.typescale.as_deref().and_then(typescale_metrics) {
                    let _ = write!(expr, ".size({})", f32_lit(sz));
                }
                let _ = write!(expr, ".into(){suffix}");
                self.line(ind, &expr);
            }
            Kind::Image => {
                let src = n.str("src").unwrap_or("");
                self.line(ind, &format!("text({}).into(){suffix}", lit(src)));
            }
            Kind::Comp(c) => {
                let _ = self.theme;
                match c {
                    Comp::Button | Comp::Fab | Comp::IconButton | Comp::Chip | Comp::ListItem | Comp::NavigationRailItem => {
                        let lbl = n.label(&["label", "headline"]).or(n.str("icon")).unwrap_or("");
                        let mut expr = format!("button(text({}))", lit(lbl));
                        if let Some(p) = n.style.padding {
                            let _ = write!(expr, ".padding([{}, {}, {}, {}])", f32_lit(p[0]), f32_lit(p[1]), f32_lit(p[2]), f32_lit(p[3]));
                        }
                        if let Some(ev) = n.event(&["onClick"]) {
                            let variant = pascal(& snake_handler(&ev.handler));
                            let _ = write!(expr, ".on_press(Message::{variant})");
                        }
                        let _ = write!(expr, ".into(){suffix}");
                        self.line(ind, &expr);
                    }
                    Comp::TextField => {
                        let placeholder = n.str("placeholder").or(n.str("label")).unwrap_or("");
                        let val = n.str("value").unwrap_or("");
                        let mut expr = format!("text_input({}, {})", lit(placeholder), lit(val));
                        if let Some(w) = n.style.width {
                            let _ = write!(expr, ".width(Length::Fixed({}))", f32_lit(w));
                        } else {
                            expr.push_str(".width(Length::Fill)");
                        }
                        let _ = write!(expr, ".into(){suffix}");
                        self.line(ind, &expr);
                    }
                    Comp::Switch | Comp::Checkbox => {
                        let lbl = n.label(&["label"]).unwrap_or("");
                        let checked = n.flag("checked") || n.flag("selected");
                        self.line(ind, &format!("checkbox({}, {checked}).into(){suffix}", lit(lbl)));
                    }
                    Comp::Radio => {
                        let lbl = n.label(&["label"]).unwrap_or("");
                        self.line(ind, &format!("text({}).into(){suffix}", lit(lbl)));
                    }
                    Comp::Slider => {
                        let min = f32_lit(n.num("min").unwrap_or(0.0));
                        let max = f32_lit(n.num("max").unwrap_or(100.0));
                        let val = f32_lit(n.num("value").unwrap_or(0.0));
                        self.line(ind, &format!("slider({min}..={max}, {val}, |_| Message::Noop).into(){suffix}"));
                    }
                    Comp::Progress => {
                        let frac = n.progress_fraction().unwrap_or(0.5) * 100.0;
                        self.line(ind, &format!("progress_bar(0.0..=100.0, {}).into(){suffix}", f32_lit(frac)));
                    }
                    Comp::Divider => {
                        self.line(ind, &format!("horizontal_rule(1).into(){suffix}"));
                    }
                    _ => {}
                }
            }
        }
    }
}

pub fn emit(doc: &Document, theme: YoloThemePreset) -> String {
    let mut e = CosmicEmitter {
        out: String::new(),
        theme,
    };
    e.emit_expr(1, &doc.root, false);

    let comp = pascal(&doc.component);
    let snake = slug(&doc.component);
    let tokens = theme.tokens(true);
    let mut out = format!(
        "// Generated by yolo-migrator from {} (component {}, theme {}). Do not hand-edit.\n\
         // Target: libcosmic / Iced 0.14 Elm MVU (`yolo-iced`).\n\n\
         use iced::widget::{{button, checkbox, column, container, horizontal_rule, progress_bar, row, slider, text, text_input}};\n\
         use iced::{{Alignment, Element, Length}};\n\
         use yolo_shared::YoloThemePreset;\n\n\
         pub const THEME_PRESET: &str = {:?};\n\
         pub const ACCENT_PRIMARY_HEX: &str = {:?};\n\n",
        doc.source,
        doc.component,
        theme.id(),
        theme.id(),
        tokens.accent_primary,
    );

    let _ = writeln!(out, "#[derive(Debug, Clone, PartialEq)]");
    let _ = writeln!(out, "pub enum {comp}Message {{");
    let _ = writeln!(out, "    Noop,");
    for h in &doc.handlers {
        let v = pascal(&snake_handler(h));
        let _ = writeln!(out, "    {v},");
    }
    let _ = writeln!(out, "}}\n");
    let _ = writeln!(out, "pub type Message = {comp}Message;\n");

    let _ = writeln!(out, "#[derive(Debug, Clone, Default)]");
    let _ = writeln!(out, "pub struct {comp}State {{\n    pub theme: YoloThemePreset,\n}}\n");

    let _ = writeln!(out, "pub fn view<'a>(_state: &'a {comp}State) -> Element<'a, Message> {{");
    out.push_str(&e.out);
    let _ = writeln!(out, "}}\n");
    let _ = writeln!(out, "pub use view as {snake}_view;");
    out
}
