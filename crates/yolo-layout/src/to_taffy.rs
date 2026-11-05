// SPDX-License-Identifier: Apache-2.0
//! [`LayoutStyle`] -> `taffy::Style` (Taffy 0.15).

use taffy::style_helpers::{TaffyGridLine, TaffyGridSpan};
use taffy::style::{
    AlignContentKeyword, AlignItemsKeyword, AlignmentSafety, GridTemplateComponent,
    MaxTrackSizingFunction, MinTrackSizingFunction, RepetitionCount, TrackSizingFunction,
};
use taffy::{Dimension, LengthPercentage, LengthPercentageAuto, MinMax, Point, Rect, Size};

use crate::style::*;

/// Axis of the container a percentage resolves against.
#[derive(Clone, Copy)]
enum Axis {
    X,
    Y,
}

/// Converts lengths, resolving [`Length::Calc`] against `container` (width, height) when given.
#[derive(Clone, Copy)]
struct Conv {
    container: Option<(f32, f32)>,
}

impl Conv {
    /// Collapses a `Calc`; every other variant passes through.
    fn flat(self, l: Length, axis: Axis) -> Length {
        match l {
            Length::Calc { px, percent } => match self.container {
                Some((w, h)) => Length::Px(px + percent * if matches!(axis, Axis::X) { w } else { h }),
                // No container: TaffyTree cannot resolve a mixed calc(), so keep the px term.
                None => Length::Px(px),
            },
            other => other,
        }
    }

    fn dim(self, l: Length, axis: Axis) -> Dimension {
        match self.flat(l, axis) {
            Length::Px(v) => Dimension::length(v),
            Length::Percent(v) => Dimension::percent(v),
            _ => Dimension::auto(),
        }
    }

    fn lpa(self, l: Length, axis: Axis) -> LengthPercentageAuto {
        match self.flat(l, axis) {
            Length::Px(v) => LengthPercentageAuto::length(v),
            Length::Percent(v) => LengthPercentageAuto::percent(v),
            _ => LengthPercentageAuto::auto(),
        }
    }

    fn lp(self, l: Length, axis: Axis) -> LengthPercentage {
        match self.flat(l, axis) {
            Length::Px(v) => LengthPercentage::length(v),
            Length::Percent(v) => LengthPercentage::percent(v),
            _ => LengthPercentage::length(0.0),
        }
    }

    /// Margin percentages refer to the container width on every side (CSS).
    fn margin(self, s: Sides<Length>) -> Rect<LengthPercentageAuto> {
        Rect {
            left: self.lpa(s.left, Axis::X),
            right: self.lpa(s.right, Axis::X),
            top: self.lpa(s.top, Axis::X),
            bottom: self.lpa(s.bottom, Axis::X),
        }
    }

    /// Padding percentages refer to the container width on every side (CSS).
    fn padding(self, s: Sides<Length>) -> Rect<LengthPercentage> {
        Rect {
            left: self.lp(s.left, Axis::X),
            right: self.lp(s.right, Axis::X),
            top: self.lp(s.top, Axis::X),
            bottom: self.lp(s.bottom, Axis::X),
        }
    }

    fn inset(self, s: Sides<Length>) -> Rect<LengthPercentageAuto> {
        Rect {
            left: self.lpa(s.left, Axis::X),
            right: self.lpa(s.right, Axis::X),
            top: self.lpa(s.top, Axis::Y),
            bottom: self.lpa(s.bottom, Axis::Y),
        }
    }
}

fn safety(s: Safety) -> AlignmentSafety {
    match s {
        Safety::Default => AlignmentSafety::Default,
        Safety::Safe => AlignmentSafety::Safe,
        Safety::Unsafe => AlignmentSafety::Unsafe,
    }
}

fn items(a: ItemAlign) -> taffy::AlignItems {
    let keyword = match a.keyword {
        ItemKeyword::Normal => AlignItemsKeyword::Normal,
        ItemKeyword::Start => AlignItemsKeyword::Start,
        ItemKeyword::End => AlignItemsKeyword::End,
        ItemKeyword::FlexStart => AlignItemsKeyword::FlexStart,
        ItemKeyword::FlexEnd => AlignItemsKeyword::FlexEnd,
        ItemKeyword::SelfStart => AlignItemsKeyword::SelfStart,
        ItemKeyword::SelfEnd => AlignItemsKeyword::SelfEnd,
        ItemKeyword::Center => AlignItemsKeyword::Center,
        ItemKeyword::Baseline => AlignItemsKeyword::Baseline,
        ItemKeyword::Stretch => AlignItemsKeyword::Stretch,
    };
    taffy::AlignItems { keyword, safety: safety(a.safety) }
}

fn content(a: ContentAlign) -> taffy::AlignContent {
    let keyword = match a.keyword {
        ContentKeyword::Normal => AlignContentKeyword::Normal,
        ContentKeyword::Start => AlignContentKeyword::Start,
        ContentKeyword::End => AlignContentKeyword::End,
        ContentKeyword::FlexStart => AlignContentKeyword::FlexStart,
        ContentKeyword::FlexEnd => AlignContentKeyword::FlexEnd,
        ContentKeyword::Center => AlignContentKeyword::Center,
        ContentKeyword::Stretch => AlignContentKeyword::Stretch,
        ContentKeyword::SpaceBetween => AlignContentKeyword::SpaceBetween,
        ContentKeyword::SpaceEvenly => AlignContentKeyword::SpaceEvenly,
        ContentKeyword::SpaceAround => AlignContentKeyword::SpaceAround,
    };
    taffy::AlignContent { keyword, safety: safety(a.safety) }
}

fn min_track(b: TrackBreadth) -> MinTrackSizingFunction {
    match b {
        TrackBreadth::Px(v) => MinTrackSizingFunction::length(v),
        TrackBreadth::Percent(v) => MinTrackSizingFunction::percent(v),
        // A flexible track's minimum is `auto` (CSS Grid §7.2.4).
        TrackBreadth::Fr(_) | TrackBreadth::Auto => MinTrackSizingFunction::auto(),
        TrackBreadth::MinContent => MinTrackSizingFunction::min_content(),
        TrackBreadth::MaxContent => MinTrackSizingFunction::max_content(),
    }
}

fn max_track(b: TrackBreadth) -> MaxTrackSizingFunction {
    match b {
        TrackBreadth::Px(v) => MaxTrackSizingFunction::length(v),
        TrackBreadth::Percent(v) => MaxTrackSizingFunction::percent(v),
        TrackBreadth::Fr(v) => MaxTrackSizingFunction::fr(v),
        TrackBreadth::Auto => MaxTrackSizingFunction::auto(),
        TrackBreadth::MinContent => MaxTrackSizingFunction::min_content(),
        TrackBreadth::MaxContent => MaxTrackSizingFunction::max_content(),
    }
}

fn track(t: Track) -> TrackSizingFunction {
    match t {
        Track::Single(b) => MinMax { min: min_track(b), max: max_track(b) },
        Track::MinMax(min, max) => MinMax { min: min_track(min), max: max_track(max) },
        Track::FitContentPx(v) => {
            MinMax { min: MinTrackSizingFunction::auto(), max: MaxTrackSizingFunction::fit_content_px(v) }
        }
        Track::FitContentPercent(v) => MinMax {
            min: MinTrackSizingFunction::auto(),
            max: MaxTrackSizingFunction::fit_content_percent(v),
        },
    }
}

fn template(items: &[TemplateItem]) -> Vec<GridTemplateComponent<String>> {
    items
        .iter()
        .map(|item| match item {
            TemplateItem::Track(t) => GridTemplateComponent::Single(track(*t)),
            TemplateItem::Repeat(count, tracks) => {
                let count = match count {
                    RepeatCount::Count(n) => RepetitionCount::Count(*n),
                    RepeatCount::AutoFill => RepetitionCount::AutoFill,
                    RepeatCount::AutoFit => RepetitionCount::AutoFit,
                };
                taffy::style_helpers::repeat(count, tracks.iter().copied().map(track).collect())
            }
        })
        .collect()
}

fn placement(line: &GridLine) -> taffy::GridPlacement<String> {
    match line {
        GridLine::Auto => taffy::GridPlacement::Auto,
        GridLine::Line(i) => taffy::GridPlacement::from_line_index(*i),
        GridLine::Named(name, idx) => taffy::GridPlacement::NamedLine(name.clone(), *idx),
        GridLine::NamedSpan(name, n) => taffy::GridPlacement::NamedSpan(name.clone(), *n),
        GridLine::Span(n) => taffy::GridPlacement::from_span(*n),
    }
}

fn placements(p: &GridPlacement) -> taffy::Line<taffy::GridPlacement<String>> {
    taffy::Line { start: placement(&p.start), end: placement(&p.end) }
}

fn overflow(o: Overflow) -> taffy::Overflow {
    match o {
        Overflow::Visible => taffy::Overflow::Visible,
        Overflow::Clip => taffy::Overflow::Clip,
        Overflow::Hidden => taffy::Overflow::Hidden,
        Overflow::Scroll => taffy::Overflow::Scroll,
    }
}

impl LayoutStyle {
    /// The equivalent Taffy style. A mixed px + percent [`Length::Calc`] keeps only its px term
    /// here (Taffy cannot resolve it without a container); use [`Self::to_taffy_in`] to resolve it.
    #[must_use]
    pub fn to_taffy(&self) -> taffy::Style {
        self.convert(Conv { container: None })
    }

    /// The equivalent Taffy style with every [`Length::Calc`] resolved against the container
    /// size `(width, height)` in px: widths, horizontal insets, margins, padding and column gaps
    /// against the width, heights, vertical insets and row gaps against the height.
    #[must_use]
    pub fn to_taffy_in(&self, container: (f32, f32)) -> taffy::Style {
        self.convert(Conv { container: Some(container) })
    }

    fn convert(&self, c: Conv) -> taffy::Style {
        let basis_axis = if matches!(self.flex_direction, FlexDirection::Row | FlexDirection::RowReverse) {
            Axis::X
        } else {
            Axis::Y
        };
        taffy::Style {
            display: match self.display {
                Display::Block => taffy::Display::Block,
                Display::Flex => taffy::Display::Flex,
                Display::Grid => taffy::Display::Grid,
                Display::None => taffy::Display::None,
            },
            position: match self.position {
                Position::Static => taffy::Position::Static,
                Position::Relative => taffy::Position::Relative,
                Position::Absolute => taffy::Position::Absolute,
                Position::Fixed => taffy::Position::Fixed,
            },
            box_sizing: match self.box_sizing {
                BoxSizing::BorderBox => taffy::BoxSizing::BorderBox,
                BoxSizing::ContentBox => taffy::BoxSizing::ContentBox,
            },
            overflow: Point { x: overflow(self.overflow_x), y: overflow(self.overflow_y) },
            inset: c.inset(self.inset),
            size: Size { width: c.dim(self.width, Axis::X), height: c.dim(self.height, Axis::Y) },
            min_size: Size { width: c.lpa(self.min_width, Axis::X), height: c.lpa(self.min_height, Axis::Y) },
            max_size: Size { width: c.lpa(self.max_width, Axis::X), height: c.lpa(self.max_height, Axis::Y) },
            aspect_ratio: self.aspect_ratio,
            margin: c.margin(self.margin),
            padding: c.padding(self.padding),
            flex_direction: match self.flex_direction {
                FlexDirection::Row => taffy::FlexDirection::Row,
                FlexDirection::RowReverse => taffy::FlexDirection::RowReverse,
                FlexDirection::Column => taffy::FlexDirection::Column,
                FlexDirection::ColumnReverse => taffy::FlexDirection::ColumnReverse,
            },
            flex_wrap: match self.flex_wrap {
                FlexWrap::NoWrap => taffy::FlexWrap::NoWrap,
                FlexWrap::Wrap => taffy::FlexWrap::Wrap,
                FlexWrap::WrapReverse => taffy::FlexWrap::WrapReverse,
            },
            flex_grow: self.flex_grow,
            flex_shrink: self.flex_shrink,
            flex_basis: c.dim(self.flex_basis, basis_axis),
            align_items: items(self.align_items),
            justify_items: items(self.justify_items),
            align_self: self.align_self.map(items),
            justify_self: self.justify_self.map(items),
            align_content: content(self.align_content),
            justify_content: content(self.justify_content),
            gap: Size { width: c.lp(self.column_gap, Axis::X), height: c.lp(self.row_gap, Axis::Y) },
            grid_template_rows: template(&self.grid_template_rows),
            grid_template_columns: template(&self.grid_template_columns),
            grid_auto_rows: self.grid_auto_rows.iter().copied().map(track).collect(),
            grid_auto_columns: self.grid_auto_columns.iter().copied().map(track).collect(),
            grid_auto_flow: match self.grid_auto_flow {
                GridAutoFlow::Row => taffy::GridAutoFlow::Row,
                GridAutoFlow::Column => taffy::GridAutoFlow::Column,
                GridAutoFlow::RowDense => taffy::GridAutoFlow::RowDense,
                GridAutoFlow::ColumnDense => taffy::GridAutoFlow::ColumnDense,
            },
            grid_row: placements(&self.grid_row),
            grid_column: placements(&self.grid_column),
            grid_template_row_names: self.grid_template_row_names.clone(),
            grid_template_column_names: self.grid_template_column_names.clone(),
            grid_template_areas: self.grid_template_areas.as_ref().map(areas),
            ..taffy::Style::default()
        }
    }
}

fn areas(a: &GridAreas) -> taffy::style::GridTemplateAreas<String> {
    taffy::style::GridTemplateAreas {
        areas: a
            .areas
            .iter()
            .map(|r| taffy::style::GridTemplateArea {
                name: r.name.clone(),
                row_start: r.row_start,
                row_end: r.row_end,
                column_start: r.column_start,
                column_end: r.column_end,
            })
            .collect(),
        row_count: a.rows,
        column_count: a.columns,
    }
}
