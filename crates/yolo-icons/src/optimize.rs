// SPDX-License-Identifier: Apache-2.0
//! SVGO-equivalent optimizer passes on the IR.
//!
//! Transform flattening happens during import. The passes here are, in order:
//!
//! 1. `drop_invisible`: layers with zero opacity or no drawing segments.
//! 2. `simplify`: degenerate cubics and quadratics whose control points lie on the chord become lines,
//!    zero-length lines are removed from filled layers, consecutive movetos collapse, a final line back to the
//!    sub-path start before `Z` is dropped (`Z` draws it), and trailing movetos are removed.
//! 3. `round`: coordinates snap to `decimals` fractional digits on the target grid.
//! 4. `simplify` again, since rounding can create new degenerate segments.
//! 5. `current_color`: single-color icons paint with `currentColor`.
//!
//! The serializer in [`crate::ir`] then chooses relative/absolute commands and shorthands per segment.

use crate::ir::{Icon, Layer, Point, Segment, round_to};
use crate::svg_import::normalize_monochrome;

/// Optimizer configuration.
#[derive(Debug, Clone, Copy)]
pub struct OptimizeOptions {
    /// Target grid side (24 for the Material Symbols base grid).
    pub grid: f64,
    /// Fractional digits kept on that grid.
    pub decimals: u32,
    /// Tolerance (grid units) for "control point lies on the chord".
    pub flatness: f64,
}

impl Default for OptimizeOptions {
    fn default() -> Self {
        Self {
            grid: 24.0,
            decimals: 2,
            flatness: 0.005,
        }
    }
}

/// Size accounting for one optimized icon.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OptimizeStats {
    pub segments_before: usize,
    pub segments_after: usize,
}

/// Runs every pass on `icon` in place.
pub fn optimize(icon: &mut Icon, options: &OptimizeOptions) -> OptimizeStats {
    let before = icon.layers.iter().map(|l| l.segments.len()).sum();
    icon.rescale(options.grid);
    drop_invisible(icon);
    for layer in &mut icon.layers {
        simplify(layer, options.flatness);
    }
    round(icon, options.decimals);
    for layer in &mut icon.layers {
        simplify(layer, options.flatness);
    }
    drop_invisible(icon);
    normalize_monochrome(icon);
    let after = icon.layers.iter().map(|l| l.segments.len()).sum();
    OptimizeStats {
        segments_before: before,
        segments_after: after,
    }
}

fn drop_invisible(icon: &mut Icon) {
    icon.layers
        .retain(|l| l.opacity > 0.0 && l.drawing_segments() > 0);
}

fn round(icon: &mut Icon, decimals: u32) {
    icon.map_points(|p| Point::new(round_to(p.x, decimals), round_to(p.y, decimals)));
    for layer in &mut icon.layers {
        if let Some(stroke) = &mut layer.stroke {
            stroke.width = round_to(stroke.width, decimals);
        }
    }
}

/// Distance from `p` to the segment `a`-`b`.
fn distance_to_segment(p: Point, a: Point, b: Point) -> f64 {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let len2 = dx * dx + dy * dy;
    if len2 == 0.0 {
        return p.distance(a);
    }
    let t = (((p.x - a.x) * dx + (p.y - a.y) * dy) / len2).clamp(0.0, 1.0);
    p.distance(Point::new(a.x + t * dx, a.y + t * dy))
}

/// Simplifies one layer's segment list.
pub fn simplify(layer: &mut Layer, flatness: f64) {
    let filled = layer.stroke.is_none();
    let mut out: Vec<Segment> = Vec::with_capacity(layer.segments.len());
    let mut cur = Point::new(0.0, 0.0);
    let mut start = cur;
    for &seg in &layer.segments {
        let seg = match seg {
            Segment::CubicTo(c1, c2, p)
                if distance_to_segment(c1, cur, p) <= flatness
                    && distance_to_segment(c2, cur, p) <= flatness =>
            {
                Segment::LineTo(p)
            }
            Segment::QuadTo(c, p) if distance_to_segment(c, cur, p) <= flatness => {
                Segment::LineTo(p)
            }
            other => other,
        };
        match seg {
            Segment::MoveTo(p) => {
                if let Some(Segment::MoveTo(_)) = out.last() {
                    out.pop();
                }
                out.push(seg);
                cur = p;
                start = p;
            }
            Segment::LineTo(p) => {
                if filled && p == cur {
                    continue;
                }
                out.push(seg);
                cur = p;
            }
            Segment::QuadTo(_, p) | Segment::CubicTo(_, _, p) => {
                out.push(seg);
                cur = p;
            }
            Segment::Close => {
                // `Z` already draws the closing line.
                if let Some(Segment::LineTo(p)) = out.last()
                    && *p == start
                {
                    out.pop();
                }
                match out.last() {
                    // An empty sub-path (`M x y Z`) paints nothing in a filled layer.
                    Some(Segment::MoveTo(_)) if filled => {
                        out.pop();
                    }
                    Some(Segment::Close) | None => {}
                    _ => out.push(Segment::Close),
                }
                cur = start;
            }
        }
    }
    while matches!(out.last(), Some(Segment::MoveTo(_))) {
        out.pop();
    }
    layer.segments = out;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{FillRule, Paint};

    fn layer(segments: Vec<Segment>) -> Layer {
        Layer::filled(segments, FillRule::NonZero, Paint::CurrentColor)
    }

    #[test]
    fn flat_cubic_becomes_line_and_closing_line_is_dropped() {
        let mut l = layer(vec![
            Segment::MoveTo(Point::new(0.0, 0.0)),
            Segment::CubicTo(
                Point::new(1.0, 0.0),
                Point::new(2.0, 0.0),
                Point::new(3.0, 0.0),
            ),
            Segment::LineTo(Point::new(3.0, 3.0)),
            Segment::LineTo(Point::new(3.0, 3.0)),
            Segment::LineTo(Point::new(0.0, 0.0)),
            Segment::Close,
            Segment::MoveTo(Point::new(9.0, 9.0)),
        ]);
        simplify(&mut l, 0.005);
        assert_eq!(
            l.segments,
            vec![
                Segment::MoveTo(Point::new(0.0, 0.0)),
                Segment::LineTo(Point::new(3.0, 0.0)),
                Segment::LineTo(Point::new(3.0, 3.0)),
                Segment::Close,
            ]
        );
    }

    #[test]
    fn optimize_rescales_and_rounds() {
        let mut icon = Icon {
            name: "t".into(),
            set: "t".into(),
            grid: 48.0,
            layers: vec![Layer::filled(
                vec![
                    Segment::MoveTo(Point::new(4.123_456, 4.0)),
                    Segment::LineTo(Point::new(44.0, 4.0)),
                    Segment::LineTo(Point::new(24.0, 44.0)),
                    Segment::Close,
                ],
                FillRule::NonZero,
                Paint::Rgb(0x21_2121),
            )],
            flips_in_rtl: false,
        };
        optimize(&mut icon, &OptimizeOptions::default());
        assert_eq!(icon.grid, 24.0);
        assert_eq!(
            icon.layers[0].segments[0],
            Segment::MoveTo(Point::new(2.06, 2.0))
        );
        assert!(icon.is_monochrome());
    }
}
