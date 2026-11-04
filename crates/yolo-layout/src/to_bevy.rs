// SPDX-License-Identifier: Apache-2.0
//! [`LayoutStyle`] -> `bevy::ui::Node` (Bevy 0.20). Bevy lays the node out with its own Taffy.

use bevy::ui::{
    AlignContent, AlignItems, AlignSelf, BoxSizing as BBoxSizing, Display as BDisplay,
    FlexDirection as BFlexDirection, FlexWrap as BFlexWrap, GridAutoFlow as BGridAutoFlow,
    GridPlacement as BGridPlacement, GridTrack, GridTrackRepetition, JustifyContent, JustifyItems,
    JustifySelf, MaxTrackSizingFunction, MinTrackSizingFunction, Node, Overflow as BOverflow,
    OverflowAxis, PositionType, RepeatedGridTrack, UiRect, Val,
};

use crate::style::*;

fn val(l: Length) -> Val {
    match l {
        Length::Auto => Val::Auto,
        Length::Px(v) => Val::Px(v),
        Length::Percent(v) => Val::Percent(v * 100.0),
    }
}

fn ui_rect(s: Sides<Length>) -> UiRect {
    UiRect { left: val(s.left), right: val(s.right), top: val(s.top), bottom: val(s.bottom) }
}

fn axis(o: Overflow) -> OverflowAxis {
    match o {
        Overflow::Visible => OverflowAxis::Visible,
        Overflow::Clip => OverflowAxis::Clip,
        Overflow::Hidden => OverflowAxis::Hidden,
        Overflow::Scroll => OverflowAxis::Scroll,
    }
}

macro_rules! item_align {
    ($fn:ident, $ty:ident, $normal:ident, $what:literal, flex) => {
        fn $fn(a: ItemAlign) -> Result<$ty, String> {
            let safe = a.safety == Safety::Safe;
            Ok(match a.keyword {
                ItemKeyword::Normal => $ty::$normal,
                ItemKeyword::Start if safe => $ty::StartSafe,
                ItemKeyword::Start => $ty::Start,
                ItemKeyword::End if safe => $ty::EndSafe,
                ItemKeyword::End => $ty::End,
                ItemKeyword::FlexStart if safe => $ty::FlexStartSafe,
                ItemKeyword::FlexStart => $ty::FlexStart,
                ItemKeyword::FlexEnd if safe => $ty::FlexEndSafe,
                ItemKeyword::FlexEnd => $ty::FlexEnd,
                ItemKeyword::Center if safe => $ty::CenterSafe,
                ItemKeyword::Center => $ty::Center,
                ItemKeyword::Baseline => $ty::Baseline,
                ItemKeyword::Stretch => $ty::Stretch,
                ItemKeyword::SelfStart | ItemKeyword::SelfEnd => {
                    return Err(concat!("`self-start`/`self-end` have no Bevy equivalent in ", $what).into());
                }
            })
        }
    };
    ($fn:ident, $ty:ident, $normal:ident, $what:literal) => {
        fn $fn(a: ItemAlign) -> Result<$ty, String> {
            let safe = a.safety == Safety::Safe;
            Ok(match a.keyword {
                ItemKeyword::Normal => $ty::$normal,
                ItemKeyword::Start | ItemKeyword::FlexStart if safe => $ty::StartSafe,
                ItemKeyword::Start | ItemKeyword::FlexStart => $ty::Start,
                ItemKeyword::End | ItemKeyword::FlexEnd if safe => $ty::EndSafe,
                ItemKeyword::End | ItemKeyword::FlexEnd => $ty::End,
                ItemKeyword::Center if safe => $ty::CenterSafe,
                ItemKeyword::Center => $ty::Center,
                ItemKeyword::Baseline => $ty::Baseline,
                ItemKeyword::Stretch => $ty::Stretch,
                ItemKeyword::SelfStart | ItemKeyword::SelfEnd => {
                    return Err(concat!("`self-start`/`self-end` have no Bevy equivalent in ", $what).into());
                }
            })
        }
    };
}

item_align!(align_items, AlignItems, Default, "align-items", flex);
item_align!(align_self_kw, AlignSelf, Auto, "align-self", flex);
item_align!(justify_items, JustifyItems, Default, "justify-items");
item_align!(justify_self_kw, JustifySelf, Auto, "justify-self");

macro_rules! content_align {
    ($fn:ident, $ty:ident) => {
        fn $fn(a: ContentAlign) -> $ty {
            let safe = a.safety == Safety::Safe;
            match a.keyword {
                ContentKeyword::Normal => $ty::Default,
                ContentKeyword::Start if safe => $ty::StartSafe,
                ContentKeyword::Start => $ty::Start,
                ContentKeyword::End if safe => $ty::EndSafe,
                ContentKeyword::End => $ty::End,
                ContentKeyword::FlexStart if safe => $ty::FlexStartSafe,
                ContentKeyword::FlexStart => $ty::FlexStart,
                ContentKeyword::FlexEnd if safe => $ty::FlexEndSafe,
                ContentKeyword::FlexEnd => $ty::FlexEnd,
                ContentKeyword::Center if safe => $ty::CenterSafe,
                ContentKeyword::Center => $ty::Center,
                ContentKeyword::Stretch => $ty::Stretch,
                ContentKeyword::SpaceBetween => $ty::SpaceBetween,
                ContentKeyword::SpaceEvenly => $ty::SpaceEvenly,
                ContentKeyword::SpaceAround => $ty::SpaceAround,
            }
        }
    };
}

content_align!(align_content, AlignContent);
content_align!(justify_content, JustifyContent);

fn min_track(b: TrackBreadth) -> MinTrackSizingFunction {
    match b {
        TrackBreadth::Px(v) => MinTrackSizingFunction::Px(v),
        TrackBreadth::Percent(v) => MinTrackSizingFunction::Percent(v * 100.0),
        TrackBreadth::Fr(_) | TrackBreadth::Auto => MinTrackSizingFunction::Auto,
        TrackBreadth::MinContent => MinTrackSizingFunction::MinContent,
        TrackBreadth::MaxContent => MinTrackSizingFunction::MaxContent,
    }
}

fn max_track(b: TrackBreadth) -> MaxTrackSizingFunction {
    match b {
        TrackBreadth::Px(v) => MaxTrackSizingFunction::Px(v),
        TrackBreadth::Percent(v) => MaxTrackSizingFunction::Percent(v * 100.0),
        TrackBreadth::Fr(v) => MaxTrackSizingFunction::Fraction(v),
        TrackBreadth::Auto => MaxTrackSizingFunction::Auto,
        TrackBreadth::MinContent => MaxTrackSizingFunction::MinContent,
        TrackBreadth::MaxContent => MaxTrackSizingFunction::MaxContent,
    }
}

fn track(t: Track) -> GridTrack {
    match t {
        Track::Single(b) => GridTrack::minmax(min_track(b), max_track(b)),
        Track::MinMax(min, max) => GridTrack::minmax(min_track(min), max_track(max)),
        Track::FitContentPx(v) => GridTrack::fit_content_px(v),
        Track::FitContentPercent(v) => GridTrack::fit_content_percent(v * 100.0),
    }
}

fn template(items: &[TemplateItem]) -> Vec<RepeatedGridTrack> {
    items
        .iter()
        .map(|item| match item {
            TemplateItem::Track(t) => RepeatedGridTrack::from(track(*t)),
            TemplateItem::Repeat(count, tracks) => RepeatedGridTrack::repeat_many(
                match count {
                    RepeatCount::Count(n) => GridTrackRepetition::Count(*n),
                    RepeatCount::AutoFill => GridTrackRepetition::AutoFill,
                    RepeatCount::AutoFit => GridTrackRepetition::AutoFit,
                },
                tracks.iter().copied().map(track).collect::<Vec<_>>(),
            ),
        })
        .collect()
}

fn placement(p: GridPlacement) -> BGridPlacement {
    match (p.start, p.end) {
        (GridLine::Auto, GridLine::Auto) => BGridPlacement::auto(),
        (GridLine::Line(s), GridLine::Auto) => BGridPlacement::start(s),
        (GridLine::Auto, GridLine::Line(e)) => BGridPlacement::end(e),
        (GridLine::Line(s), GridLine::Line(e)) => BGridPlacement::start_end(s, e),
        (GridLine::Line(s), GridLine::Span(n)) => BGridPlacement::start_span(s, n),
        (GridLine::Span(n), GridLine::Line(e)) => BGridPlacement::end_span(e, n),
        (GridLine::Span(n), GridLine::Auto | GridLine::Span(_)) | (GridLine::Auto, GridLine::Span(n)) => {
            BGridPlacement::span(n)
        }
    }
}

impl LayoutStyle {
    /// The equivalent Bevy UI node.
    ///
    /// Fails for what Bevy cannot express: `position: fixed`, `self-start`/`self-end`.
    /// `position: static` maps to `Relative` with no insets.
    pub fn to_bevy_node(&self) -> Result<Node, String> {
        let (position_type, inset) = match self.position {
            Position::Static => (PositionType::Relative, Sides::all(Length::Auto)),
            Position::Relative => (PositionType::Relative, self.inset),
            Position::Absolute => (PositionType::Absolute, self.inset),
            Position::Fixed => return Err("`position: fixed` has no Bevy UI equivalent".into()),
        };
        Ok(Node {
            display: match self.display {
                Display::Block => BDisplay::Block,
                Display::Flex => BDisplay::Flex,
                Display::Grid => BDisplay::Grid,
                Display::None => BDisplay::None,
            },
            box_sizing: match self.box_sizing {
                BoxSizing::BorderBox => BBoxSizing::BorderBox,
                BoxSizing::ContentBox => BBoxSizing::ContentBox,
            },
            position_type,
            overflow: BOverflow { x: axis(self.overflow_x), y: axis(self.overflow_y) },
            left: val(inset.left),
            right: val(inset.right),
            top: val(inset.top),
            bottom: val(inset.bottom),
            width: val(self.width),
            height: val(self.height),
            min_width: val(self.min_width),
            min_height: val(self.min_height),
            max_width: val(self.max_width),
            max_height: val(self.max_height),
            aspect_ratio: self.aspect_ratio,
            align_items: align_items(self.align_items)?,
            justify_items: justify_items(self.justify_items)?,
            align_self: self.align_self.map_or(Ok(AlignSelf::Auto), align_self_kw)?,
            justify_self: self.justify_self.map_or(Ok(JustifySelf::Auto), justify_self_kw)?,
            align_content: align_content(self.align_content),
            justify_content: justify_content(self.justify_content),
            margin: ui_rect(self.margin),
            padding: ui_rect(self.padding),
            flex_direction: match self.flex_direction {
                FlexDirection::Row => BFlexDirection::Row,
                FlexDirection::RowReverse => BFlexDirection::RowReverse,
                FlexDirection::Column => BFlexDirection::Column,
                FlexDirection::ColumnReverse => BFlexDirection::ColumnReverse,
            },
            flex_wrap: match self.flex_wrap {
                FlexWrap::NoWrap => BFlexWrap::NoWrap,
                FlexWrap::Wrap => BFlexWrap::Wrap,
                FlexWrap::WrapReverse => BFlexWrap::WrapReverse,
            },
            flex_grow: self.flex_grow,
            flex_shrink: self.flex_shrink,
            flex_basis: val(self.flex_basis),
            row_gap: val(self.row_gap),
            column_gap: val(self.column_gap),
            grid_auto_flow: match self.grid_auto_flow {
                GridAutoFlow::Row => BGridAutoFlow::Row,
                GridAutoFlow::Column => BGridAutoFlow::Column,
                GridAutoFlow::RowDense => BGridAutoFlow::RowDense,
                GridAutoFlow::ColumnDense => BGridAutoFlow::ColumnDense,
            },
            grid_template_rows: template(&self.grid_template_rows),
            grid_template_columns: template(&self.grid_template_columns),
            grid_auto_rows: self.grid_auto_rows.iter().copied().map(track).collect(),
            grid_auto_columns: self.grid_auto_columns.iter().copied().map(track).collect(),
            grid_row: placement(self.grid_row),
            grid_column: placement(self.grid_column),
            ..Node::default()
        })
    }
}

/// Parses a declaration list straight to a Bevy node.
pub fn css_to_bevy_node(css: &str, tokens: &crate::TokenTable) -> Result<Node, crate::Diagnostics> {
    let style = crate::parse_style(css, tokens)?;
    style.to_bevy_node().map_err(|message| {
        crate::Diagnostics(vec![crate::Diagnostic { line: 1, column: 1, property: None, message }])
    })
}
