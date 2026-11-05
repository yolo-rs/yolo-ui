// SPDX-License-Identifier: Apache-2.0
//! SVG import through `usvg`.
//!
//! `usvg` resolves CSS, `<use>`, basic shapes, arcs and inherited attributes into plain paths. This module flattens
//! every absolute transform (including the `viewBox` mapping) into the coordinates, rescales the result onto the
//! requested square grid and lowers paint to [`Paint`].

use crate::Error;
use crate::ir::{FillRule, Icon, Layer, Paint, Point, Segment, Stroke};
use resvg::tiny_skia::{PathSegment, Transform};
use resvg::usvg;

/// Non-fatal findings while importing (unsupported constructs that were approximated or dropped).
#[derive(Debug, Default, Clone)]
pub struct ImportNotes {
    pub warnings: Vec<String>,
}

/// Parses an SVG document into the IR on a square grid of side `grid`.
///
/// # Errors
/// Returns [`Error::Svg`] when `usvg` rejects the document.
pub fn import_svg(
    name: &str,
    set: &str,
    data: &[u8],
    grid: f64,
) -> Result<(Icon, ImportNotes), Error> {
    let options = usvg::Options::default();
    let tree =
        usvg::Tree::from_data(data, &options).map_err(|e| Error::Svg(format!("{name}: {e}")))?;
    let size = tree.size();
    let source_side = f64::from(size.width().max(size.height()));
    let mut notes = ImportNotes::default();
    let mut layers = Vec::new();
    walk(tree.root(), 1.0, &mut layers, &mut notes);

    // Center a non-square canvas on the square grid, then scale to `grid`.
    let dx = (source_side - f64::from(size.width())) / 2.0;
    let dy = (source_side - f64::from(size.height())) / 2.0;
    let k = if source_side > 0.0 {
        grid / source_side
    } else {
        1.0
    };
    let mut icon = Icon {
        name: name.to_owned(),
        set: set.to_owned(),
        grid,
        layers,
        flips_in_rtl: false,
    };
    icon.map_points(|p| Point::new((p.x + dx) * k, (p.y + dy) * k));
    for layer in &mut icon.layers {
        if let Some(stroke) = &mut layer.stroke {
            stroke.width *= k;
        }
    }
    normalize_monochrome(&mut icon);
    Ok((icon, notes))
}

fn walk(
    group: &usvg::Group,
    parent_opacity: f64,
    layers: &mut Vec<Layer>,
    notes: &mut ImportNotes,
) {
    let opacity = parent_opacity * f64::from(group.opacity().get());
    if group.clip_path().is_some() || group.mask().is_some() {
        notes
            .warnings
            .push(format!("group `{}`: clip-path/mask ignored", group.id()));
    }
    if !group.filters().is_empty() {
        notes
            .warnings
            .push(format!("group `{}`: filters ignored", group.id()));
    }
    for node in group.children() {
        match node {
            usvg::Node::Group(g) => walk(g, opacity, layers, notes),
            usvg::Node::Path(path) => {
                if !path.is_visible() {
                    continue;
                }
                lower_path(path, opacity, layers, notes);
            }
            usvg::Node::Image(_) => notes.warnings.push("raster <image> dropped".to_owned()),
            usvg::Node::Text(_) => notes
                .warnings
                .push("<text> dropped (icons must be outlines)".to_owned()),
        }
    }
}

fn transform_scale(ts: Transform) -> f64 {
    let det = f64::from(ts.sx) * f64::from(ts.sy) - f64::from(ts.kx) * f64::from(ts.ky);
    det.abs().sqrt()
}

fn segments_of(path: &usvg::Path) -> Vec<Segment> {
    let ts = path.abs_transform();
    let data = path.data().clone();
    let data = if ts.is_identity() {
        Some(data)
    } else {
        data.transform(ts)
    };
    let Some(data) = data else {
        return Vec::new();
    };
    let p = |pt: resvg::tiny_skia::Point| Point::new(f64::from(pt.x), f64::from(pt.y));
    data.segments()
        .map(|seg| match seg {
            PathSegment::MoveTo(a) => Segment::MoveTo(p(a)),
            PathSegment::LineTo(a) => Segment::LineTo(p(a)),
            PathSegment::QuadTo(c, a) => Segment::QuadTo(p(c), p(a)),
            PathSegment::CubicTo(c1, c2, a) => Segment::CubicTo(p(c1), p(c2), p(a)),
            PathSegment::Close => Segment::Close,
        })
        .collect()
}

fn paint_of(paint: &usvg::Paint, notes: &mut ImportNotes) -> u32 {
    match paint {
        usvg::Paint::Color(c) => {
            (u32::from(c.red) << 16) | (u32::from(c.green) << 8) | u32::from(c.blue)
        }
        usvg::Paint::LinearGradient(g) => {
            notes
                .warnings
                .push("linear gradient flattened to its first stop".to_owned());
            g.stops().first().map_or(0, |s| {
                let c = s.color();
                (u32::from(c.red) << 16) | (u32::from(c.green) << 8) | u32::from(c.blue)
            })
        }
        usvg::Paint::RadialGradient(g) => {
            notes
                .warnings
                .push("radial gradient flattened to its first stop".to_owned());
            g.stops().first().map_or(0, |s| {
                let c = s.color();
                (u32::from(c.red) << 16) | (u32::from(c.green) << 8) | u32::from(c.blue)
            })
        }
        usvg::Paint::Pattern(_) => {
            notes
                .warnings
                .push("pattern paint flattened to black".to_owned());
            0
        }
    }
}

fn lower_path(path: &usvg::Path, opacity: f64, layers: &mut Vec<Layer>, notes: &mut ImportNotes) {
    let segments = segments_of(path);
    if segments.is_empty() {
        return;
    }
    if let Some(fill) = path.fill() {
        let rgb = paint_of(fill.paint(), notes);
        layers.push(Layer {
            segments: segments.clone(),
            fill_rule: match fill.rule() {
                usvg::FillRule::NonZero => FillRule::NonZero,
                usvg::FillRule::EvenOdd => FillRule::EvenOdd,
            },
            paint: Paint::Rgb(rgb),
            opacity: opacity * f64::from(fill.opacity().get()),
            stroke: None,
        });
    }
    if let Some(stroke) = path.stroke() {
        let rgb = paint_of(stroke.paint(), notes);
        let width = f64::from(stroke.width().get()) * transform_scale(path.abs_transform());
        if stroke.dasharray().is_some() {
            notes.warnings.push("stroke dash array ignored".to_owned());
        }
        layers.push(Layer {
            segments,
            fill_rule: FillRule::NonZero,
            paint: Paint::Rgb(rgb),
            opacity: opacity * f64::from(stroke.opacity().get()),
            stroke: Some(Stroke {
                width,
                square_cap: matches!(stroke.linecap(), usvg::LineCap::Square),
            }),
        });
    }
}

/// Single-color icons (Material `#000`, Fluent `#212121`, COSMIC `#2e3436`) become `currentColor`.
/// Multi-color icons keep their fixed colors.
pub fn normalize_monochrome(icon: &mut Icon) {
    let mut colors = icon.layers.iter().filter_map(|l| match l.paint {
        Paint::Rgb(c) => Some(c),
        Paint::CurrentColor => None,
    });
    let Some(first) = colors.next() else {
        return;
    };
    if colors.all(|c| c == first) {
        for layer in &mut icon.layers {
            layer.paint = Paint::CurrentColor;
        }
    }
}
