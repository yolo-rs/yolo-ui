// SPDX-License-Identifier: Apache-2.0
//! CSS input through Lightning CSS: flat `.class` rules and inline `style` objects, resolved
//! into the closed IR style subset. Anything outside the subset is an error.

use std::collections::HashMap;

use lightningcss::rules::CssRule;
use lightningcss::stylesheet::{ParserOptions, PrinterOptions, StyleAttribute, StyleSheet};
use lightningcss::traits::ToCss;

use crate::diag::Diag;
use crate::ir::{Align, COLOR_ROLES, Dir, Justify, Style};

#[derive(Clone, Debug)]
pub struct Decl {
    pub name: String,
    pub value: String,
}

#[derive(Clone, Debug)]
pub struct ClassRule {
    pub decls: Vec<Decl>,
    pub file: String,
    pub line: u32,
    pub col: u32,
}

pub type ClassMap = HashMap<String, Vec<ClassRule>>;

fn decls_of(block: &lightningcss::declaration::DeclarationBlock<'_>) -> Result<Vec<Decl>, String> {
    let mut out = Vec::new();
    for prop in block.declarations.iter().chain(block.important_declarations.iter()) {
        let name = prop.property_id().name().to_string();
        let value = prop.value_to_css_string(PrinterOptions::default()).map_err(|e| e.to_string())?;
        out.push(Decl { name, value });
    }
    Ok(out)
}

/// Parses a stylesheet into class rules. Only `.class` (and `.a, .b`) selectors are accepted.
pub fn parse_stylesheet(src: &str, file: &str, diags: &mut Vec<Diag>, into: &mut ClassMap) {
    let options = ParserOptions { filename: file.to_string(), ..ParserOptions::default() };
    let sheet = match StyleSheet::parse(src, options) {
        Ok(s) => s,
        Err(e) => {
            let (line, col) = e.loc.as_ref().map_or((1, 1), |l| (l.line + 1, l.column));
            diags.push(Diag { file: file.into(), line, col, msg: format!("CSS parse error: {}", e.kind) });
            return;
        }
    };
    for rule in &sheet.rules.0 {
        match rule {
            CssRule::Style(r) => {
                let line = r.loc.line + 1;
                let col = r.loc.column;
                let err = |diags: &mut Vec<Diag>, msg: String| {
                    diags.push(Diag { file: file.into(), line, col, msg });
                };
                if !r.rules.0.is_empty() {
                    err(diags, "nested CSS rules are not supported; flatten them into `.class` rules".into());
                    continue;
                }
                let selectors = match r.selectors.to_css_string(PrinterOptions::default()) {
                    Ok(s) => s,
                    Err(e) => {
                        err(diags, format!("cannot read selector: {e}"));
                        continue;
                    }
                };
                let decls = match decls_of(&r.declarations) {
                    Ok(d) => d,
                    Err(e) => {
                        err(diags, format!("cannot read declarations: {e}"));
                        continue;
                    }
                };
                for sel in selectors.split(',').map(str::trim) {
                    let ok = sel.strip_prefix('.').is_some_and(|n| {
                        !n.is_empty()
                            && n.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
                    });
                    if !ok {
                        err(
                            diags,
                            format!(
                                "unsupported selector `{sel}`: only single class selectors (`.name`) are converted"
                            ),
                        );
                        continue;
                    }
                    into.entry(sel[1..].to_string()).or_default().push(ClassRule {
                        decls: decls.clone(),
                        file: file.into(),
                        line,
                        col,
                    });
                }
            }
            other => {
                let what = match other {
                    CssRule::Media(_) => "@media",
                    CssRule::Import(_) => "@import",
                    CssRule::Keyframes(_) => "@keyframes",
                    CssRule::FontFace(_) => "@font-face",
                    _ => "at-rule",
                };
                diags.push(Diag {
                    file: file.into(),
                    line: 1,
                    col: 1,
                    msg: format!("unsupported CSS {what}: only flat `.class` rules are converted"),
                });
            }
        }
    }
}

/// Parses inline declarations (`gap: 12px; flex-direction: row`).
pub fn parse_inline(text: &str) -> Result<Vec<Decl>, String> {
    let attr = StyleAttribute::parse(text, ParserOptions::default()).map_err(|e| e.kind.to_string())?;
    decls_of(&attr.declarations)
}

fn px(v: &str) -> Result<f64, String> {
    let v = v.trim();
    if v == "0" {
        return Ok(0.0);
    }
    if let Some(n) = v.strip_suffix("px") {
        return n.parse().map_err(|_| format!("invalid length `{v}`"));
    }
    if let Some(n) = v.strip_suffix("rem") {
        return n.parse::<f64>().map(|x| x * 16.0).map_err(|_| format!("invalid length `{v}`"));
    }
    Err(format!("unsupported length `{v}`: use px or rem"))
}

fn box4(v: &str) -> Result<[f64; 4], String> {
    let parts: Vec<f64> = v.split_whitespace().map(px).collect::<Result<_, _>>()?;
    Ok(match parts.as_slice() {
        [a] => [*a, *a, *a, *a],
        [a, b] => [*a, *b, *a, *b],
        [a, b, c] => [*a, *b, *c, *b],
        [a, b, c, d] => [*a, *b, *c, *d],
        _ => return Err(format!("invalid box value `{v}`")),
    })
}

fn pair(v: &str) -> Result<(f64, f64), String> {
    let parts: Vec<f64> = v.split_whitespace().map(px).collect::<Result<_, _>>()?;
    match parts.as_slice() {
        [a] => Ok((*a, *a)),
        [a, b] => Ok((*a, *b)),
        _ => Err(format!("invalid value `{v}`")),
    }
}

/// `var(--md-sys-color-on-surface)` -> `on-surface`.
fn color_role(v: &str) -> Result<String, String> {
    let inner = v
        .trim()
        .strip_prefix("var(")
        .and_then(|s| s.strip_suffix(')'))
        .map(|s| s.split(',').next().unwrap_or("").trim());
    let role = inner.and_then(|s| s.strip_prefix("--md-sys-color-"));
    match role {
        Some(r) if COLOR_ROLES.contains(&r) => Ok(r.to_string()),
        Some(r) => Err(format!("unknown M3 color role `{r}`")),
        None => Err(format!(
            "color `{v}` is not a Material 3 role: use var(--md-sys-color-<role>) so every host can theme it"
        )),
    }
}

fn shape(v: &str) -> Result<String, String> {
    v.trim()
        .strip_prefix("var(--md-sys-shape-corner-")
        .and_then(|s| s.strip_suffix(')'))
        .filter(|s| crate::ir::shape_radius(s).is_some())
        .map(str::to_string)
        .ok_or_else(|| format!("border-radius `{v}` must be an M3 shape token var(--md-sys-shape-corner-<size>)"))
}

fn align(v: &str) -> Result<Align, String> {
    Ok(match v.trim() {
        "start" | "flex-start" | "left" | "self-start" => Align::Start,
        "center" => Align::Center,
        "end" | "flex-end" | "right" | "self-end" => Align::End,
        "stretch" | "normal" => Align::Stretch,
        other => return Err(format!("unsupported alignment `{other}`")),
    })
}

/// Applies declarations to the style; returns one message per unsupported declaration.
pub fn apply(style: &mut Style, decls: &[Decl]) -> Vec<String> {
    let mut errors = Vec::new();
    for d in decls {
        let v = d.value.trim();
        let r: Result<(), String> = (|| {
            match d.name.as_str() {
                "display" => match v {
                    "flex" | "inline-flex" => style.display_flex = true,
                    "grid" | "inline-grid" => style.display_grid = true,
                    "block" => {
                        style.display_flex = false;
                        style.display_grid = false;
                    }
                    _ => return Err(format!("display `{v}` is not supported (flex, grid or block only)")),
                },
                "flex-direction" => {
                    style.display_flex = true;
                    style.direction = Some(match v {
                        "row" => Dir::Row,
                        "column" => Dir::Column,
                        _ => return Err(format!("flex-direction `{v}` is not supported")),
                    })
                }
                "gap" | "row-gap" | "column-gap" => style.gap = Some(pair(v)?.0),
                "padding" => style.padding = Some(box4(v)?),
                "margin" => style.margin = Some(box4(v)?),
                "padding-top" | "padding-right" | "padding-bottom" | "padding-left" | "padding-inline"
                | "padding-block" | "padding-inline-start" | "padding-inline-end" | "padding-block-start"
                | "padding-block-end" | "margin-top" | "margin-right" | "margin-bottom" | "margin-left"
                | "margin-inline" | "margin-block" | "margin-inline-start" | "margin-inline-end"
                | "margin-block-start" | "margin-block-end" => {
                    let (target, side) = d.name.split_once('-').unwrap();
                    let slot = if target == "padding" { &mut style.padding } else { &mut style.margin };
                    let b = slot.get_or_insert([0.0; 4]);
                    match side {
                        "top" | "block-start" => b[0] = px(v)?,
                        "right" | "inline-end" => b[1] = px(v)?,
                        "bottom" | "block-end" => b[2] = px(v)?,
                        "left" | "inline-start" => b[3] = px(v)?,
                        "inline" => {
                            let (s, e) = pair(v)?;
                            b[3] = s;
                            b[1] = e;
                        }
                        _ => {
                            let (s, e) = pair(v)?;
                            b[0] = s;
                            b[2] = e;
                        }
                    }
                }
                "align-items" => style.align = Some(align(v)?),
                "justify-content" => {
                    style.justify = Some(match v {
                        "start" | "flex-start" | "normal" => Justify::Start,
                        "center" => Justify::Center,
                        "end" | "flex-end" => Justify::End,
                        "space-between" => Justify::SpaceBetween,
                        _ => return Err(format!("justify-content `{v}` is not supported")),
                    })
                }
                "text-align" => style.text_align = Some(align(v)?),
                "flex" => {
                    let first = v.split_whitespace().next().unwrap_or("");
                    style.grow = match first {
                        "auto" => true,
                        "none" => false,
                        n => n.parse::<f64>().map_err(|_| format!("unsupported flex `{v}`"))? > 0.0,
                    }
                }
                "flex-grow" => style.grow = v.parse::<f64>().map_err(|_| format!("invalid flex-grow `{v}`"))? > 0.0,
                "color" => style.color = Some(color_role(v)?),
                "background-color" | "background" => style.background = Some(color_role(v)?),
                "border-radius" => style.shape = Some(shape(v)?),
                "width" | "min-width" => {
                    if v == "100%" {
                        style.fill_width = true;
                    } else {
                        style.width = Some(px(v)?);
                    }
                }
                "height" | "min-height" => {
                    if v == "100%" {
                        style.fill_height = true;
                    } else {
                        style.height = Some(px(v)?);
                    }
                }
                "max-width" => style.max_width = Some(px(v)?),
                "overflow" | "overflow-y" => match v {
                    "auto" | "scroll" | "visible auto" | "hidden auto" | "visible scroll" | "hidden scroll" => {
                        style.scroll = true
                    }
                    "visible" | "hidden" | "clip" => {}
                    _ => return Err(format!("overflow `{v}` is not supported")),
                },
                "font" | "font-size" | "font-weight" | "font-family" | "line-height" | "letter-spacing" => {
                    return Err(format!(
                        "`{}` is not converted: use an M3 typescale class (md-typescale-<role>-<size>) or Tailwind text-*",
                        d.name
                    ));
                }
                other => return Err(format!("CSS property `{other}` is outside the converter subset")),
            }
            Ok(())
        })();
        if let Err(e) = r {
            errors.push(e);
        }
    }
    errors
}
