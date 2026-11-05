// SPDX-License-Identifier: Apache-2.0
//! Normalized icon path IR.
//!
//! Every importer (SVG through `usvg`, Chromium `.icon`) lowers to this representation: absolute coordinates on a
//! square grid with the origin at the top-left corner, all transforms already applied, arcs converted to cubic
//! Beziers, and paint reduced to either `currentColor` or a fixed sRGB color. The serializer writes the shortest
//! SVG path data it can (relative or absolute per command, `H`/`V`/`S`/`T` shorthands, implicit separators).

use serde::{Deserialize, Serialize};
use std::fmt::Write as _;

/// A point on the icon grid.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    #[must_use]
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    #[must_use]
    pub fn distance(self, other: Self) -> f64 {
        (self.x - other.x).hypot(self.y - other.y)
    }
}

/// One absolute path segment.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Segment {
    MoveTo(Point),
    LineTo(Point),
    QuadTo(Point, Point),
    CubicTo(Point, Point, Point),
    Close,
}

impl Segment {
    /// End point of a drawing segment (`None` for `Close`).
    #[must_use]
    pub const fn end(&self) -> Option<Point> {
        match *self {
            Self::MoveTo(p) | Self::LineTo(p) | Self::QuadTo(_, p) | Self::CubicTo(_, _, p) => {
                Some(p)
            }
            Self::Close => None,
        }
    }

    fn map_points(&mut self, mut f: impl FnMut(Point) -> Point) {
        match self {
            Self::MoveTo(p) | Self::LineTo(p) => *p = f(*p),
            Self::QuadTo(c, p) => {
                *c = f(*c);
                *p = f(*p);
            }
            Self::CubicTo(c1, c2, p) => {
                *c1 = f(*c1);
                *c2 = f(*c2);
                *p = f(*p);
            }
            Self::Close => {}
        }
    }
}

/// Fill rule of one layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FillRule {
    #[default]
    NonZero,
    EvenOdd,
}

/// Paint of one layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Paint {
    /// Inherits the CSS `color` of the host element, so every token color applies.
    CurrentColor,
    /// Fixed opaque sRGB color as `0xRRGGBB` (multi-color icons such as badges).
    Rgb(u32),
}

/// Stroke parameters for stroked layers (Chromium `STROKE`, SVG `stroke`).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Stroke {
    pub width: f64,
    pub square_cap: bool,
}

/// One painted path.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Layer {
    pub segments: Vec<Segment>,
    pub fill_rule: FillRule,
    pub paint: Paint,
    pub opacity: f64,
    pub stroke: Option<Stroke>,
}

impl Layer {
    #[must_use]
    pub fn filled(segments: Vec<Segment>, fill_rule: FillRule, paint: Paint) -> Self {
        Self {
            segments,
            fill_rule,
            paint,
            opacity: 1.0,
            stroke: None,
        }
    }

    /// Number of drawing segments (everything except `MoveTo` and `Close`).
    #[must_use]
    pub fn drawing_segments(&self) -> usize {
        self.segments
            .iter()
            .filter(|s| !matches!(s, Segment::MoveTo(_) | Segment::Close))
            .count()
    }
}

/// A normalized icon.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Icon {
    /// Canonical name (`home`, `arrow_back`, `aphrody_mark`).
    pub name: String,
    /// Source set id from `catalog/assets.lock.toml`.
    pub set: String,
    /// Side of the square grid the coordinates live on (24 for Material Symbols).
    pub grid: f64,
    pub layers: Vec<Layer>,
    /// Chromium `FLIPS_IN_RTL`: mirror horizontally in right-to-left locales.
    pub flips_in_rtl: bool,
}

impl Icon {
    /// Uniformly rescales the icon onto a grid of side `grid`.
    pub fn rescale(&mut self, grid: f64) {
        if (self.grid - grid).abs() < f64::EPSILON || self.grid <= 0.0 {
            self.grid = grid;
            return;
        }
        let k = grid / self.grid;
        for layer in &mut self.layers {
            for seg in &mut layer.segments {
                seg.map_points(|p| Point::new(p.x * k, p.y * k));
            }
            if let Some(stroke) = &mut layer.stroke {
                stroke.width *= k;
            }
        }
        self.grid = grid;
    }

    /// Applies `f` to every point of every layer.
    pub fn map_points(&mut self, mut f: impl FnMut(Point) -> Point) {
        for layer in &mut self.layers {
            for seg in &mut layer.segments {
                seg.map_points(&mut f);
            }
        }
    }

    /// Axis-aligned bounds of all on- and off-curve points, or `None` for an empty icon.
    #[must_use]
    pub fn control_bounds(&self) -> Option<(Point, Point)> {
        let mut min = Point::new(f64::INFINITY, f64::INFINITY);
        let mut max = Point::new(f64::NEG_INFINITY, f64::NEG_INFINITY);
        let mut any = false;
        let mut visit = |p: Point| {
            any = true;
            min.x = min.x.min(p.x);
            min.y = min.y.min(p.y);
            max.x = max.x.max(p.x);
            max.y = max.y.max(p.y);
        };
        for layer in &self.layers {
            for seg in &layer.segments {
                match *seg {
                    Segment::MoveTo(p) | Segment::LineTo(p) => visit(p),
                    Segment::QuadTo(c, p) => {
                        visit(c);
                        visit(p);
                    }
                    Segment::CubicTo(c1, c2, p) => {
                        visit(c1);
                        visit(c2);
                        visit(p);
                    }
                    Segment::Close => {}
                }
            }
        }
        any.then_some((min, max))
    }

    /// True when every layer paints with `currentColor`.
    #[must_use]
    pub fn is_monochrome(&self) -> bool {
        self.layers.iter().all(|l| l.paint == Paint::CurrentColor)
    }

    /// Serializes the icon as a standalone SVG document. `fill="currentColor"` sits on the root so the icon takes
    /// the host's text color; fixed-color layers override it.
    #[must_use]
    pub fn to_svg(&self, decimals: u32) -> String {
        let grid = format_number(self.grid, decimals);
        let mut out = format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {grid} {grid}\" fill=\"currentColor\">"
        );
        out.push_str(&self.svg_body(decimals, None));
        out.push_str("</svg>");
        out
    }

    /// Serializes only the `<path>` elements. When `color_override` is set, `currentColor` layers use it instead.
    #[must_use]
    pub fn svg_body(&self, decimals: u32, color_override: Option<&str>) -> String {
        let mut out = String::new();
        for layer in &self.layers {
            let d = path_data(&layer.segments, decimals);
            if d.is_empty() {
                continue;
            }
            let color = match layer.paint {
                Paint::CurrentColor => color_override.map(str::to_owned),
                Paint::Rgb(rgb) => Some(format!("#{rgb:06x}")),
            };
            let _ = write!(out, "<path d=\"{d}\"");
            match layer.stroke {
                Some(stroke) => {
                    let paint = color.unwrap_or_else(|| "currentColor".to_owned());
                    let _ = write!(
                        out,
                        " fill=\"none\" stroke=\"{paint}\" stroke-width=\"{}\"",
                        format_number(stroke.width, decimals)
                    );
                    out.push_str(if stroke.square_cap {
                        " stroke-linecap=\"square\""
                    } else {
                        " stroke-linecap=\"round\" stroke-linejoin=\"round\""
                    });
                }
                None => {
                    if let Some(color) = color {
                        let _ = write!(out, " fill=\"{color}\"");
                    }
                    if layer.fill_rule == FillRule::EvenOdd {
                        out.push_str(" fill-rule=\"evenodd\"");
                    }
                }
            }
            if layer.opacity < 1.0 {
                let _ = write!(out, " opacity=\"{}\"", format_number(layer.opacity, 3));
            }
            out.push_str("/>");
        }
        out
    }
}

/// Rounds `v` to `decimals` fractional digits.
#[must_use]
pub fn round_to(v: f64, decimals: u32) -> f64 {
    let k = 10f64.powi(decimals as i32);
    let r = (v * k).round() / k;
    if r == 0.0 { 0.0 } else { r }
}

/// Formats a number with at most `decimals` fractional digits, no trailing zeros and no leading zero
/// (`0.5` becomes `.5`, `-0.5` becomes `-.5`).
#[must_use]
pub fn format_number(v: f64, decimals: u32) -> String {
    let r = round_to(v, decimals);
    let mut s = format!("{r:.*}", decimals as usize);
    if s.contains('.') {
        while s.ends_with('0') {
            s.pop();
        }
        if s.ends_with('.') {
            s.pop();
        }
    }
    if s == "-0" {
        s = "0".to_owned();
    }
    if let Some(rest) = s.strip_prefix("0.") {
        s = format!(".{rest}");
    } else if let Some(rest) = s.strip_prefix("-0.") {
        s = format!("-.{rest}");
    }
    s
}

/// Appends numbers to path data with the minimum separators: none before `-`, none before `.` when the previous
/// number already has a fractional part.
struct NumberWriter {
    out: String,
    prev_has_dot: bool,
    need_sep: bool,
}

impl NumberWriter {
    fn command(&mut self, c: char) {
        self.out.push(c);
        self.need_sep = false;
        self.prev_has_dot = false;
    }

    fn num(&mut self, s: &str) {
        if self.need_sep {
            let starts_neg = s.starts_with('-');
            let starts_dot = s.starts_with('.');
            if !(starts_neg || (starts_dot && self.prev_has_dot)) {
                self.out.push(' ');
            }
        }
        self.out.push_str(s);
        self.prev_has_dot = s.contains('.') || s.contains('e');
        self.need_sep = true;
    }
}

fn fmt_args(args: &[f64], decimals: u32) -> Vec<String> {
    args.iter().map(|&a| format_number(a, decimals)).collect()
}

fn encoded_len(parts: &[String]) -> usize {
    // Upper bound: one separator per number. Good enough to choose between absolute and relative forms.
    parts.iter().map(|p| p.len() + 1).sum()
}

/// Serializes segments to the shortest SVG path data the encoder supports.
#[must_use]
pub fn path_data(segments: &[Segment], decimals: u32) -> String {
    let mut w = NumberWriter {
        out: String::new(),
        prev_has_dot: false,
        need_sep: false,
    };
    let mut cur = Point::new(0.0, 0.0);
    let mut start = cur;
    let mut last_cmd: Option<char> = None;
    // Previous segment's last control point, for S/T reflection.
    let mut prev_cubic_ctrl: Option<Point> = None;
    let mut prev_quad_ctrl: Option<Point> = None;
    let r = |v: f64| round_to(v, decimals);
    // S/T are used when the reflected control point is within 1.5 units of the last kept digit: rounding both
    // neighbours can push an exactly smooth join off by one digit.
    let tol = 1.5 * 10f64.powi(-(decimals as i32)) + 1e-9;
    let same = |a: Point, b: Point| (a.x - b.x).abs() <= tol && (a.y - b.y).abs() <= tol;
    // Control point the decoder will use for the next reflection when the previous command was S/T.
    let mut implied_ctrl: Option<Point> = None;

    for original in segments {
        // Work on the rounded segment so absolute and relative forms describe the same points.
        let mut rounded = *original;
        rounded.map_points(|p| Point::new(r(p.x), r(p.y)));
        let seg = &rounded;
        // Choose the command letter and numbers; emit the shorter of absolute/relative.
        let (abs_cmd, abs_args, rel_args): (char, Vec<f64>, Vec<f64>) = match *seg {
            Segment::MoveTo(p) => ('M', vec![p.x, p.y], vec![p.x - cur.x, p.y - cur.y]),
            Segment::LineTo(p) => {
                if r(p.y) == r(cur.y) {
                    ('H', vec![p.x], vec![p.x - cur.x])
                } else if r(p.x) == r(cur.x) {
                    ('V', vec![p.y], vec![p.y - cur.y])
                } else {
                    ('L', vec![p.x, p.y], vec![p.x - cur.x, p.y - cur.y])
                }
            }
            Segment::QuadTo(c, p) => {
                let reflected = prev_quad_ctrl
                    .map_or(cur, |q| Point::new(2.0 * cur.x - q.x, 2.0 * cur.y - q.y));
                if same(reflected, c) {
                    implied_ctrl = Some(reflected);
                    ('T', vec![p.x, p.y], vec![p.x - cur.x, p.y - cur.y])
                } else {
                    (
                        'Q',
                        vec![c.x, c.y, p.x, p.y],
                        vec![c.x - cur.x, c.y - cur.y, p.x - cur.x, p.y - cur.y],
                    )
                }
            }
            Segment::CubicTo(c1, c2, p) => {
                let reflected = prev_cubic_ctrl
                    .map_or(cur, |q| Point::new(2.0 * cur.x - q.x, 2.0 * cur.y - q.y));
                if same(reflected, c1) {
                    (
                        'S',
                        vec![c2.x, c2.y, p.x, p.y],
                        vec![c2.x - cur.x, c2.y - cur.y, p.x - cur.x, p.y - cur.y],
                    )
                } else {
                    (
                        'C',
                        vec![c1.x, c1.y, c2.x, c2.y, p.x, p.y],
                        vec![
                            c1.x - cur.x,
                            c1.y - cur.y,
                            c2.x - cur.x,
                            c2.y - cur.y,
                            p.x - cur.x,
                            p.y - cur.y,
                        ],
                    )
                }
            }
            Segment::Close => ('Z', vec![], vec![]),
        };

        if abs_cmd == 'Z' {
            if last_cmd != Some('z') {
                w.command('z');
                last_cmd = Some('z');
            }
            cur = start;
            prev_cubic_ctrl = None;
            prev_quad_ctrl = None;
            continue;
        }

        let abs_parts = fmt_args(&abs_args, decimals);
        let rel_parts = fmt_args(
            &rel_args.iter().map(|&v| r(v)).collect::<Vec<_>>(),
            decimals,
        );
        // Cost includes the command letter, which is free when it repeats the previous one.
        let letter = |c: char| usize::from(!(last_cmd == Some(c) && !matches!(c, 'M' | 'm')));
        let abs_cost = encoded_len(&abs_parts) + letter(abs_cmd);
        let rel_cost = encoded_len(&rel_parts) + letter(abs_cmd.to_ascii_lowercase());
        let use_rel = rel_cost < abs_cost;
        let cmd = if use_rel {
            abs_cmd.to_ascii_lowercase()
        } else {
            abs_cmd
        };
        let parts = if use_rel { rel_parts } else { abs_parts };

        // A repeated command letter may be omitted, except after a moveto (implicit lineto) and for moveto itself.
        let implicit_ok = last_cmd == Some(cmd) && !matches!(cmd, 'M' | 'm');
        if !implicit_ok {
            w.command(cmd);
        }
        for p in &parts {
            w.num(p);
        }
        last_cmd = Some(cmd);

        // Track the exact rounded current point so relative output never drifts.
        let end = seg.end().expect("drawing segment has an end point");
        let end = Point::new(r(end.x), r(end.y));
        prev_cubic_ctrl = None;
        prev_quad_ctrl = None;
        match *seg {
            Segment::MoveTo(_) => start = end,
            Segment::CubicTo(_, c2, _) => prev_cubic_ctrl = Some(Point::new(r(c2.x), r(c2.y))),
            Segment::QuadTo(c, _) => {
                prev_quad_ctrl = Some(implied_ctrl.unwrap_or(Point::new(r(c.x), r(c.y))));
            }
            _ => {}
        }
        implied_ctrl = None;
        cur = end;
    }
    w.out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_are_minimal() {
        assert_eq!(format_number(0.5, 2), ".5");
        assert_eq!(format_number(-0.25, 2), "-.25");
        assert_eq!(format_number(12.0, 2), "12");
        assert_eq!(format_number(-0.001, 2), "0");
        assert_eq!(format_number(1.23456, 3), "1.235");
    }

    #[test]
    fn path_data_uses_shorthands_and_relative_commands() {
        let segs = vec![
            Segment::MoveTo(Point::new(2.0, 2.0)),
            Segment::LineTo(Point::new(22.0, 2.0)),
            Segment::LineTo(Point::new(22.0, 22.0)),
            Segment::LineTo(Point::new(2.0, 22.0)),
            Segment::Close,
        ];
        // Ties between absolute and relative keep the absolute form.
        assert_eq!(path_data(&segs, 2), "M2 2H22V22H2z");
    }

    #[test]
    fn separators_are_elided() {
        let segs = vec![
            Segment::MoveTo(Point::new(0.5, 0.25)),
            Segment::LineTo(Point::new(-1.5, 3.0)),
        ];
        // `.5.25` and `-2 2.75` style compaction.
        let d = path_data(&segs, 2);
        assert!(d.starts_with("M.5.25"), "{d}");
    }

    #[test]
    fn smooth_cubic_is_detected() {
        let segs = vec![
            Segment::MoveTo(Point::new(0.0, 0.0)),
            Segment::CubicTo(
                Point::new(0.0, 5.0),
                Point::new(5.0, 10.0),
                Point::new(10.0, 10.0),
            ),
            Segment::CubicTo(
                Point::new(15.0, 10.0),
                Point::new(20.0, 5.0),
                Point::new(20.0, 0.0),
            ),
        ];
        let d = path_data(&segs, 2);
        assert!(d.contains('s') || d.contains('S'), "{d}");
    }
}
