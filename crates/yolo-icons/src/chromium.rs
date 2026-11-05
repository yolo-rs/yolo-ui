// SPDX-License-Identifier: Apache-2.0
//! Chromium vector icon (`.icon`) importer.
//!
//! The format is a comma-separated list of commands and numeric arguments, read by
//! `components/vector_icons/aggregate_vector_icons.py` and painted by `ui/gfx/paint_vector_icon.cc`. Semantics
//! reproduced here:
//!
//! - The canvas defaults to 48 dip; `CANVAS_DIMENSIONS, n` starts a new representation when it follows path
//!   commands (files may carry a 32 dip and a 16 dip drawing, separated by a blank line).
//! - Each path defaults to the even-odd fill rule; `FILL_RULE_NONZERO` switches the current path to non-zero.
//! - `NEW_PATH` starts a new path with fresh paint. `PATH_COLOR_ARGB`, `PATH_COLOR_ALPHA`, `STROKE` and `CAP_SQUARE`
//!   apply to the current path. Strokes default to round caps.
//! - `CUBIC_TO_SHORTHAND` and `QUADRATIC_TO_SHORTHAND` reflect the last control point of the previous command when
//!   that command was any Bezier (Chromium's `IsCommandTypeCurve`), otherwise use the current point.
//! - `CIRCLE`, `OVAL` and `ROUND_RECT` append closed sub-paths.
//! - `FLIPS_IN_RTL` is recorded on the icon. `CLIP`, `DISABLE_AA` and `PATH_MODE_CLEAR` are rendering hints that a
//!   static SVG cannot express exactly; they are reported as warnings.
//!
//! Each representation is lowered to an SVG document with absolute commands and handed to `usvg`, which converts
//! the elliptical arcs to cubics with the same math as the SVG importer.

use crate::Error;
use crate::ir::{Icon, format_number};
use crate::svg_import::{ImportNotes, import_svg};
use std::fmt::Write as _;

/// Chromium `kReferenceSizeDip`.
pub const DEFAULT_CANVAS: u32 = 48;

/// Commands understood by `ui/gfx/vector_icon_types.h` (Chromium 2026).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    NewPath,
    FillRuleNonzero,
    PathColorAlpha,
    PathColorArgb,
    PathModeClear,
    Stroke,
    CapSquare,
    MoveTo,
    RMoveTo,
    ArcTo,
    RArcTo,
    LineTo,
    RLineTo,
    HLineTo,
    RHLineTo,
    VLineTo,
    RVLineTo,
    CubicTo,
    RCubicTo,
    CubicToShorthand,
    QuadraticTo,
    RQuadraticTo,
    QuadraticToShorthand,
    RQuadraticToShorthand,
    Circle,
    Oval,
    RoundRect,
    Close,
    CanvasDimensions,
    Clip,
    DisableAa,
    FlipsInRtl,
}

impl Command {
    fn parse(token: &str) -> Option<Self> {
        Some(match token {
            "NEW_PATH" => Self::NewPath,
            "FILL_RULE_NONZERO" => Self::FillRuleNonzero,
            "PATH_COLOR_ALPHA" => Self::PathColorAlpha,
            "PATH_COLOR_ARGB" => Self::PathColorArgb,
            "PATH_MODE_CLEAR" => Self::PathModeClear,
            "STROKE" => Self::Stroke,
            "CAP_SQUARE" => Self::CapSquare,
            "MOVE_TO" => Self::MoveTo,
            "R_MOVE_TO" => Self::RMoveTo,
            "ARC_TO" => Self::ArcTo,
            "R_ARC_TO" => Self::RArcTo,
            "LINE_TO" => Self::LineTo,
            "R_LINE_TO" => Self::RLineTo,
            "H_LINE_TO" => Self::HLineTo,
            "R_H_LINE_TO" => Self::RHLineTo,
            "V_LINE_TO" => Self::VLineTo,
            "R_V_LINE_TO" => Self::RVLineTo,
            "CUBIC_TO" => Self::CubicTo,
            "R_CUBIC_TO" => Self::RCubicTo,
            "CUBIC_TO_SHORTHAND" => Self::CubicToShorthand,
            "QUADRATIC_TO" => Self::QuadraticTo,
            "R_QUADRATIC_TO" => Self::RQuadraticTo,
            "QUADRATIC_TO_SHORTHAND" => Self::QuadraticToShorthand,
            "R_QUADRATIC_TO_SHORTHAND" => Self::RQuadraticToShorthand,
            "CIRCLE" => Self::Circle,
            "OVAL" => Self::Oval,
            "ROUND_RECT" => Self::RoundRect,
            "CLOSE" => Self::Close,
            "CANVAS_DIMENSIONS" => Self::CanvasDimensions,
            "CLIP" => Self::Clip,
            "DISABLE_AA" => Self::DisableAa,
            "FLIPS_IN_RTL" => Self::FlipsInRtl,
            _ => return None,
        })
    }

    /// Argument count, as in `GetCommandArgumentCount`.
    #[must_use]
    pub const fn arg_count(self) -> usize {
        match self {
            Self::NewPath
            | Self::FillRuleNonzero
            | Self::PathModeClear
            | Self::CapSquare
            | Self::Close
            | Self::DisableAa
            | Self::FlipsInRtl => 0,
            Self::PathColorAlpha
            | Self::Stroke
            | Self::HLineTo
            | Self::RHLineTo
            | Self::VLineTo
            | Self::RVLineTo
            | Self::CanvasDimensions => 1,
            Self::MoveTo
            | Self::RMoveTo
            | Self::LineTo
            | Self::RLineTo
            | Self::QuadraticToShorthand
            | Self::RQuadraticToShorthand => 2,
            Self::Circle => 3,
            Self::PathColorArgb
            | Self::QuadraticTo
            | Self::RQuadraticTo
            | Self::CubicToShorthand
            | Self::Oval
            | Self::Clip => 4,
            Self::RoundRect => 5,
            Self::CubicTo | Self::RCubicTo => 6,
            Self::ArcTo | Self::RArcTo => 7,
        }
    }

    const fn is_curve(self) -> bool {
        matches!(
            self,
            Self::CubicTo
                | Self::RCubicTo
                | Self::CubicToShorthand
                | Self::QuadraticTo
                | Self::RQuadraticTo
                | Self::QuadraticToShorthand
                | Self::RQuadraticToShorthand
        )
    }

    const fn is_geometry(self) -> bool {
        !matches!(
            self,
            Self::NewPath
                | Self::FillRuleNonzero
                | Self::PathColorAlpha
                | Self::PathColorArgb
                | Self::PathModeClear
                | Self::Stroke
                | Self::CapSquare
                | Self::CanvasDimensions
                | Self::Clip
                | Self::DisableAa
                | Self::FlipsInRtl
        )
    }
}

/// One parsed command with its arguments.
#[derive(Debug, Clone, PartialEq)]
pub struct Element {
    pub command: Command,
    pub args: Vec<f64>,
}

/// One drawing of an icon at a given canvas size.
#[derive(Debug, Clone, PartialEq)]
pub struct Representation {
    pub canvas: u32,
    pub elements: Vec<Element>,
}

fn parse_number(token: &str) -> Option<f64> {
    let t = token.trim();
    let t = t
        .strip_suffix('f')
        .or_else(|| t.strip_suffix('F'))
        .unwrap_or(t);
    if let Some(hex) = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")) {
        return u32::from_str_radix(hex, 16).ok().map(f64::from);
    }
    t.parse::<f64>().ok()
}

fn strip_comments(source: &str) -> String {
    let mut out = String::with_capacity(source.len());
    for line in source.lines() {
        let code = line.find("//").map_or(line, |i| &line[..i]);
        out.push_str(code);
        out.push('\n');
    }
    out
}

/// Parses a `.icon` file into its representations (largest canvas first, as Chromium stores them).
///
/// # Errors
/// Returns [`Error::ChromiumIcon`] on unknown commands, malformed numbers or missing arguments.
pub fn parse(source: &str) -> Result<Vec<Representation>, Error> {
    let code = strip_comments(source);
    // Commas and whitespace both separate tokens: the last command of a representation often has no trailing
    // comma before the blank line that precedes the next `CANVAS_DIMENSIONS`.
    let tokens: Vec<&str> = code
        .split(|c: char| c == ',' || c.is_whitespace())
        .filter(|t| !t.is_empty())
        .collect();
    let mut reps: Vec<Representation> = Vec::new();
    let mut current = Representation {
        canvas: DEFAULT_CANVAS,
        elements: Vec::new(),
    };
    let mut has_geometry = false;
    let mut i = 0;
    while i < tokens.len() {
        let token = tokens[i];
        let command = Command::parse(token).ok_or_else(|| {
            Error::ChromiumIcon(format!("unknown command `{token}` at token {i}"))
        })?;
        let n = command.arg_count();
        let mut args = Vec::with_capacity(n);
        for k in 0..n {
            let raw = tokens
                .get(i + 1 + k)
                .ok_or_else(|| Error::ChromiumIcon(format!("`{token}` expects {n} arguments")))?;
            args.push(
                parse_number(raw).ok_or_else(|| {
                    Error::ChromiumIcon(format!("bad number `{raw}` for `{token}`"))
                })?,
            );
        }
        i += 1 + n;
        if command == Command::CanvasDimensions {
            if has_geometry {
                reps.push(std::mem::replace(
                    &mut current,
                    Representation {
                        canvas: DEFAULT_CANVAS,
                        elements: Vec::new(),
                    },
                ));
                has_geometry = false;
            }
            current.canvas = args[0] as u32;
            continue;
        }
        has_geometry |= command.is_geometry();
        current.elements.push(Element { command, args });
    }
    if has_geometry || reps.is_empty() {
        reps.push(current);
    }
    Ok(reps)
}

#[derive(Clone)]
struct PathState {
    d: String,
    evenodd: bool,
    argb: Option<[u8; 4]>,
    alpha: Option<u8>,
    stroke: Option<f64>,
    square_cap: bool,
    clear: bool,
}

impl PathState {
    fn new() -> Self {
        Self {
            d: String::new(),
            evenodd: true,
            argb: None,
            alpha: None,
            stroke: None,
            square_cap: false,
            clear: false,
        }
    }
}

fn n(v: f64) -> String {
    format_number(v, 4)
}

/// Lowers one representation to an SVG document whose user space equals the canvas.
///
/// Returns the document plus the `FLIPS_IN_RTL` flag and warnings.
#[must_use]
pub fn to_svg(rep: &Representation) -> (String, bool, Vec<String>) {
    let mut warnings = Vec::new();
    let mut paths: Vec<PathState> = vec![PathState::new()];
    let mut flips = false;
    let (mut cx, mut cy) = (0.0f64, 0.0f64);
    let (mut sx, mut sy) = (0.0f64, 0.0f64);
    // Last control point of the previous curve, for shorthand reflection.
    let mut last_ctrl: Option<(f64, f64)> = None;
    let mut prev = Command::NewPath;

    for el in &rep.elements {
        let a = &el.args;
        if el.command == Command::NewPath {
            // Chromium starts a fresh path and fresh paint flags; an untouched first path is reused.
            if paths.last().is_some_and(|p| !p.d.is_empty()) {
                paths.push(PathState::new());
            } else if let Some(last) = paths.last_mut() {
                *last = PathState::new();
            }
            prev = Command::NewPath;
            last_ctrl = None;
            continue;
        }
        let path = paths.last_mut().expect("at least one path");
        let mut ctrl: Option<(f64, f64)> = None;
        match el.command {
            Command::NewPath => unreachable!("handled above"),
            Command::FillRuleNonzero => path.evenodd = false,
            Command::PathColorAlpha => path.alpha = Some(a[0].floor().clamp(0.0, 255.0) as u8),
            Command::PathColorArgb => {
                path.argb = Some([a[0] as u8, a[1] as u8, a[2] as u8, a[3] as u8]);
            }
            Command::PathModeClear => {
                path.clear = true;
                warnings
                    .push("PATH_MODE_CLEAR path dropped (no SVG equivalent for kClear)".to_owned());
            }
            Command::Stroke => path.stroke = Some(a[0]),
            Command::CapSquare => path.square_cap = true,
            Command::MoveTo | Command::RMoveTo => {
                let (x, y) = if el.command == Command::MoveTo {
                    (a[0], a[1])
                } else {
                    (cx + a[0], cy + a[1])
                };
                let _ = write!(path.d, "M{} {}", n(x), n(y));
                (cx, cy, sx, sy) = (x, y, x, y);
            }
            Command::LineTo | Command::RLineTo => {
                let (x, y) = if el.command == Command::LineTo {
                    (a[0], a[1])
                } else {
                    (cx + a[0], cy + a[1])
                };
                let _ = write!(path.d, "L{} {}", n(x), n(y));
                (cx, cy) = (x, y);
            }
            Command::HLineTo | Command::RHLineTo => {
                let x = if el.command == Command::HLineTo {
                    a[0]
                } else {
                    cx + a[0]
                };
                let _ = write!(path.d, "L{} {}", n(x), n(cy));
                cx = x;
            }
            Command::VLineTo | Command::RVLineTo => {
                let y = if el.command == Command::VLineTo {
                    a[0]
                } else {
                    cy + a[0]
                };
                let _ = write!(path.d, "L{} {}", n(cx), n(y));
                cy = y;
            }
            Command::ArcTo | Command::RArcTo => {
                let (x, y) = if el.command == Command::ArcTo {
                    (a[5], a[6])
                } else {
                    (cx + a[5], cy + a[6])
                };
                let large = u8::from(a[3] != 0.0);
                let sweep = u8::from(a[4] != 0.0);
                let _ = write!(
                    path.d,
                    "A{} {} {} {large} {sweep} {} {}",
                    n(a[0]),
                    n(a[1]),
                    n(a[2]),
                    n(x),
                    n(y)
                );
                (cx, cy) = (x, y);
            }
            Command::CubicTo | Command::RCubicTo => {
                let (ox, oy) = if el.command == Command::CubicTo {
                    (0.0, 0.0)
                } else {
                    (cx, cy)
                };
                let (x1, y1, x2, y2, x, y) = (
                    ox + a[0],
                    oy + a[1],
                    ox + a[2],
                    oy + a[3],
                    ox + a[4],
                    oy + a[5],
                );
                let _ = write!(
                    path.d,
                    "C{} {} {} {} {} {}",
                    n(x1),
                    n(y1),
                    n(x2),
                    n(y2),
                    n(x),
                    n(y)
                );
                ctrl = Some((x2, y2));
                (cx, cy) = (x, y);
            }
            Command::CubicToShorthand => {
                let (x1, y1) = reflect(prev, last_ctrl, cx, cy);
                let (x2, y2, x, y) = (a[0], a[1], a[2], a[3]);
                let _ = write!(
                    path.d,
                    "C{} {} {} {} {} {}",
                    n(x1),
                    n(y1),
                    n(x2),
                    n(y2),
                    n(x),
                    n(y)
                );
                ctrl = Some((x2, y2));
                (cx, cy) = (x, y);
            }
            Command::QuadraticTo | Command::RQuadraticTo => {
                let (ox, oy) = if el.command == Command::QuadraticTo {
                    (0.0, 0.0)
                } else {
                    (cx, cy)
                };
                let (x1, y1, x, y) = (ox + a[0], oy + a[1], ox + a[2], oy + a[3]);
                let _ = write!(path.d, "Q{} {} {} {}", n(x1), n(y1), n(x), n(y));
                ctrl = Some((x1, y1));
                (cx, cy) = (x, y);
            }
            Command::QuadraticToShorthand | Command::RQuadraticToShorthand => {
                let (x1, y1) = reflect(prev, last_ctrl, cx, cy);
                let (x, y) = if el.command == Command::QuadraticToShorthand {
                    (a[0], a[1])
                } else {
                    (cx + a[0], cy + a[1])
                };
                let _ = write!(path.d, "Q{} {} {} {}", n(x1), n(y1), n(x), n(y));
                ctrl = Some((x1, y1));
                (cx, cy) = (x, y);
            }
            Command::Circle => {
                let (x, y, r) = (a[0], a[1], a[2]);
                // Skia `addCircle`: clockwise contour starting at the rightmost point.
                let _ = write!(
                    path.d,
                    "M{} {}A{r} {r} 0 1 1 {} {}A{r} {r} 0 1 1 {} {}Z",
                    n(x + r),
                    n(y),
                    n(x - r),
                    n(y),
                    n(x + r),
                    n(y),
                    r = n(r)
                );
                (cx, cy, sx, sy) = (x + r, y, x + r, y);
            }
            Command::Oval => {
                let (x, y, rx, ry) = (a[0], a[1], a[2], a[3]);
                let _ = write!(
                    path.d,
                    "M{} {}A{rx} {ry} 0 1 1 {} {}A{rx} {ry} 0 1 1 {} {}Z",
                    n(x + rx),
                    n(y),
                    n(x - rx),
                    n(y),
                    n(x + rx),
                    n(y),
                    rx = n(rx),
                    ry = n(ry)
                );
                (cx, cy, sx, sy) = (x + rx, y, x + rx, y);
            }
            Command::RoundRect => {
                let (x, y, w, h) = (a[0], a[1], a[2], a[3]);
                let r = a[4].min(w / 2.0).min(h / 2.0).max(0.0);
                let rr = n(r);
                let _ = write!(
                    path.d,
                    "M{} {}L{} {}A{rr} {rr} 0 0 1 {} {}L{} {}A{rr} {rr} 0 0 1 {} {}L{} {}A{rr} {rr} 0 0 1 {} {}L{} {}A{rr} {rr} 0 0 1 {} {}Z",
                    n(x + r),
                    n(y),
                    n(x + w - r),
                    n(y),
                    n(x + w),
                    n(y + r),
                    n(x + w),
                    n(y + h - r),
                    n(x + w - r),
                    n(y + h),
                    n(x + r),
                    n(y + h),
                    n(x),
                    n(y + h - r),
                    n(x),
                    n(y + r),
                    n(x + r),
                    n(y),
                );
                (cx, cy, sx, sy) = (x + r, y, x + r, y);
            }
            Command::Close => {
                path.d.push('Z');
                (cx, cy) = (sx, sy);
            }
            Command::Clip => warnings.push("CLIP rect ignored".to_owned()),
            Command::DisableAa => {}
            Command::FlipsInRtl => flips = true,
            Command::CanvasDimensions => {}
        }
        // Chromium tracks `previous_command_type` across every command, including paint commands.
        last_ctrl = ctrl;
        prev = el.command;
    }

    let side = rep.canvas;
    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{side}\" height=\"{side}\" viewBox=\"0 0 {side} {side}\">"
    );
    for p in paths.iter().filter(|p| !p.d.is_empty() && !p.clear) {
        let (rgb, mut alpha) = match p.argb {
            Some([a, r, g, b]) => (format!("#{r:02x}{g:02x}{b:02x}"), f64::from(a) / 255.0),
            None => ("#000000".to_owned(), 1.0),
        };
        if let Some(al) = p.alpha {
            alpha = f64::from(al) / 255.0;
        }
        let _ = write!(svg, "<path d=\"{}\"", p.d);
        match p.stroke {
            Some(width) => {
                let cap = if p.square_cap { "square" } else { "round" };
                let _ = write!(
                    svg,
                    " fill=\"none\" stroke=\"{rgb}\" stroke-width=\"{}\" stroke-linecap=\"{cap}\"",
                    n(width)
                );
                if alpha < 1.0 {
                    let _ = write!(svg, " stroke-opacity=\"{}\"", n(alpha));
                }
            }
            None => {
                let _ = write!(svg, " fill=\"{rgb}\"");
                if p.evenodd {
                    svg.push_str(" fill-rule=\"evenodd\"");
                }
                if alpha < 1.0 {
                    let _ = write!(svg, " fill-opacity=\"{}\"", n(alpha));
                }
            }
        }
        svg.push_str("/>");
    }
    svg.push_str("</svg>");
    (svg, flips, warnings)
}

fn reflect(prev: Command, last_ctrl: Option<(f64, f64)>, cx: f64, cy: f64) -> (f64, f64) {
    match (prev.is_curve(), last_ctrl) {
        (true, Some((qx, qy))) => (2.0 * cx - qx, 2.0 * cy - qy),
        _ => (cx, cy),
    }
}

/// Imports a `.icon` source into the IR on a grid of side `grid`, using the representation whose canvas is
/// closest to `grid` (ties prefer the larger, more detailed drawing).
///
/// # Errors
/// Propagates parse and SVG lowering errors.
pub fn import_icon(
    name: &str,
    set: &str,
    source: &str,
    grid: f64,
) -> Result<(Icon, ImportNotes), Error> {
    let reps = parse(source)?;
    let rep = reps
        .iter()
        .min_by(|a, b| {
            let da = (f64::from(a.canvas) - grid).abs();
            let db = (f64::from(b.canvas) - grid).abs();
            da.total_cmp(&db).then(b.canvas.cmp(&a.canvas))
        })
        .ok_or_else(|| Error::ChromiumIcon(format!("{name}: no representation")))?;
    let (svg, flips, warnings) = to_svg(rep);
    let (mut icon, mut notes) = import_svg(name, set, svg.as_bytes(), grid)?;
    icon.flips_in_rtl = flips;
    notes.warnings.extend(warnings);
    Ok((icon, notes))
}

#[cfg(test)]
mod tests {
    use super::*;

    const LOCK: &str = "// Copyright\nCANVAS_DIMENSIONS, 24,\nFILL_RULE_NONZERO,\nMOVE_TO, 6, 22,\nR_CUBIC_TO, -0.55f, 0, -1.02f, -0.19f, -1.41f, -0.59f,\nARC_TO, 1.94f, 1.94f, 0, 0, 1, 4, 20,\nV_LINE_TO, 10,\nR_H_LINE_TO, 1,\nCLOSE";

    #[test]
    fn parses_commands_and_float_suffixes() {
        let reps = parse(LOCK).unwrap();
        assert_eq!(reps.len(), 1);
        assert_eq!(reps[0].canvas, 24);
        assert_eq!(reps[0].elements[1].command, Command::MoveTo);
        assert_eq!(
            reps[0].elements[2].args,
            vec![-0.55, 0.0, -1.02, -0.19, -1.41, -0.59]
        );
    }

    #[test]
    fn splits_representations_on_canvas_dimensions() {
        let src = "CANVAS_DIMENSIONS, 32,\nMOVE_TO, 1, 1,\nLINE_TO, 31, 1,\nLINE_TO, 16, 31,\nCLOSE\n\nCANVAS_DIMENSIONS, 16,\nMOVE_TO, 1, 1,\nLINE_TO, 15, 1,\nLINE_TO, 8, 15,\nCLOSE";
        let reps = parse(src).unwrap();
        assert_eq!(
            reps.iter().map(|r| r.canvas).collect::<Vec<_>>(),
            vec![32, 16]
        );
    }

    #[test]
    fn default_canvas_is_48_and_circle_is_supported() {
        let reps = parse("CIRCLE, 24, 24, 20,\nMOVE_TO, 20, 34,\nLINE_TO, 10, 24,\nCLOSE").unwrap();
        assert_eq!(reps[0].canvas, 48);
        let (svg, _, _) = to_svg(&reps[0]);
        assert!(svg.contains("viewBox=\"0 0 48 48\""));
        assert!(svg.contains("A20 20 0 1 1"));
    }

    #[test]
    fn rejects_unknown_commands() {
        assert!(parse("MOVE_TO, 1, 1,\nWIGGLE, 3").is_err());
    }

    #[test]
    fn imports_to_ir_and_rescales() {
        let (icon, _) = import_icon("lock", "chromium", LOCK, 24.0).unwrap();
        assert_eq!(icon.grid, 24.0);
        assert_eq!(icon.layers.len(), 1);
        assert!(icon.is_monochrome());
        assert_eq!(icon.layers[0].fill_rule, crate::ir::FillRule::NonZero);
        let (min, max) = icon.control_bounds().unwrap();
        assert!(min.x >= 3.9 && max.y <= 22.01, "{min:?} {max:?}");
    }
}
