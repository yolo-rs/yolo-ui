// SPDX-License-Identifier: Apache-2.0
//! Host-neutral layout style: the CSS subset after parsing and token resolution.
//!
//! Lengths are logical px; percentages are fractions (`50%` is `0.5`).

use serde::Serialize;

/// `display`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Display {
    /// Block container (CSS initial value for the elements this engine models).
    #[default]
    Block,
    /// Flex container.
    Flex,
    /// Grid container.
    Grid,
    /// Removed from layout.
    None,
}

/// `position`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Position {
    /// Normal flow, insets ignored.
    #[default]
    Static,
    /// Normal flow, offset by insets.
    Relative,
    /// Out of flow, positioned against the nearest positioned ancestor.
    Absolute,
    /// Out of flow, positioned against the root.
    Fixed,
}

/// `box-sizing`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum BoxSizing {
    /// Sizes include padding and border (the Yolo UI default, as in Bevy and most CSS resets).
    #[default]
    BorderBox,
    /// Sizes exclude padding and border.
    ContentBox,
}

/// `overflow-x` / `overflow-y`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Overflow {
    /// Content may overflow.
    #[default]
    Visible,
    /// Clipped, not scrollable.
    Clip,
    /// Clipped, scrollable by script.
    Hidden,
    /// Clipped, scrollable, scrollbar space reserved.
    Scroll,
}

/// A length that may be `auto`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Length {
    /// `auto`.
    #[default]
    Auto,
    /// Logical px.
    Px(f32),
    /// Fraction of the containing block (`0.5` = `50%`).
    Percent(f32),
}

impl Length {
    /// Zero px.
    pub const ZERO: Self = Self::Px(0.0);
}

/// Four sides, CSS order.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize)]
#[allow(missing_docs)]
pub struct Sides<T> {
    pub top: T,
    pub right: T,
    pub bottom: T,
    pub left: T,
}

impl<T: Copy> Sides<T> {
    /// The same value on every side.
    pub const fn all(v: T) -> Self {
        Self { top: v, right: v, bottom: v, left: v }
    }
}

/// `flex-direction`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
#[allow(missing_docs)]
pub enum FlexDirection {
    #[default]
    Row,
    RowReverse,
    Column,
    ColumnReverse,
}

/// `flex-wrap`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
#[allow(missing_docs)]
pub enum FlexWrap {
    #[default]
    NoWrap,
    Wrap,
    WrapReverse,
}

/// Item-alignment keyword (`align-items`, `align-self`, `justify-items`, `justify-self`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
#[allow(missing_docs)]
pub enum ItemKeyword {
    #[default]
    Normal,
    Start,
    End,
    FlexStart,
    FlexEnd,
    SelfStart,
    SelfEnd,
    Center,
    Baseline,
    Stretch,
}

/// Content-distribution keyword (`justify-content`, `align-content`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
#[allow(missing_docs)]
pub enum ContentKeyword {
    #[default]
    Normal,
    Start,
    End,
    FlexStart,
    FlexEnd,
    Center,
    Stretch,
    SpaceBetween,
    SpaceEvenly,
    SpaceAround,
}

/// `safe` / `unsafe` overflow alignment.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
#[allow(missing_docs)]
pub enum Safety {
    #[default]
    Default,
    Safe,
    Unsafe,
}

/// An item alignment value.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[allow(missing_docs)]
pub struct ItemAlign {
    pub keyword: ItemKeyword,
    pub safety: Safety,
}

/// A content alignment value.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[allow(missing_docs)]
pub struct ContentAlign {
    pub keyword: ContentKeyword,
    pub safety: Safety,
}

/// One grid track breadth.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
#[allow(missing_docs)]
pub enum TrackBreadth {
    Px(f32),
    Percent(f32),
    Fr(f32),
    Auto,
    MinContent,
    MaxContent,
}

/// One grid track size.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Track {
    /// A single breadth.
    Single(TrackBreadth),
    /// `minmax(min, max)`; `min` is never `Fr`.
    MinMax(TrackBreadth, TrackBreadth),
    /// `fit-content(<px>)`.
    FitContentPx(f32),
    /// `fit-content(<percent>)`.
    FitContentPercent(f32),
}

/// `repeat()` count.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
#[allow(missing_docs)]
pub enum RepeatCount {
    Count(u16),
    AutoFill,
    AutoFit,
}

/// One item of `grid-template-rows` / `grid-template-columns`.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum TemplateItem {
    /// A plain track.
    Track(Track),
    /// `repeat(count, tracks...)`.
    Repeat(RepeatCount, Vec<Track>),
}

/// `grid-auto-flow`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
#[allow(missing_docs)]
pub enum GridAutoFlow {
    #[default]
    Row,
    Column,
    RowDense,
    ColumnDense,
}

/// One grid line reference.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum GridLine {
    /// `auto`.
    #[default]
    Auto,
    /// A line number (negative counts from the end; never 0).
    Line(i16),
    /// `span N`.
    Span(u16),
}

/// Start and end grid lines of one axis.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[allow(missing_docs)]
pub struct GridPlacement {
    pub start: GridLine,
    pub end: GridLine,
}

/// The resolved layout style of one node.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[allow(missing_docs)]
pub struct LayoutStyle {
    pub display: Display,
    pub position: Position,
    pub box_sizing: BoxSizing,
    pub overflow_x: Overflow,
    pub overflow_y: Overflow,
    pub inset: Sides<Length>,
    pub width: Length,
    pub height: Length,
    pub min_width: Length,
    pub min_height: Length,
    pub max_width: Length,
    pub max_height: Length,
    pub aspect_ratio: Option<f32>,
    pub margin: Sides<Length>,
    /// Never `Auto`.
    pub padding: Sides<Length>,
    pub flex_direction: FlexDirection,
    pub flex_wrap: FlexWrap,
    pub flex_grow: f32,
    pub flex_shrink: f32,
    pub flex_basis: Length,
    pub align_items: ItemAlign,
    pub justify_items: ItemAlign,
    /// `None` is `auto`.
    pub align_self: Option<ItemAlign>,
    /// `None` is `auto`.
    pub justify_self: Option<ItemAlign>,
    pub align_content: ContentAlign,
    pub justify_content: ContentAlign,
    /// Never `Auto`.
    pub row_gap: Length,
    /// Never `Auto`.
    pub column_gap: Length,
    pub grid_template_rows: Vec<TemplateItem>,
    pub grid_template_columns: Vec<TemplateItem>,
    pub grid_auto_rows: Vec<Track>,
    pub grid_auto_columns: Vec<Track>,
    pub grid_auto_flow: GridAutoFlow,
    pub grid_row: GridPlacement,
    pub grid_column: GridPlacement,
}

impl Default for LayoutStyle {
    fn default() -> Self {
        Self {
            display: Display::Block,
            position: Position::Static,
            box_sizing: BoxSizing::BorderBox,
            overflow_x: Overflow::Visible,
            overflow_y: Overflow::Visible,
            inset: Sides::all(Length::Auto),
            width: Length::Auto,
            height: Length::Auto,
            min_width: Length::Auto,
            min_height: Length::Auto,
            max_width: Length::Auto,
            max_height: Length::Auto,
            aspect_ratio: None,
            margin: Sides::all(Length::ZERO),
            padding: Sides::all(Length::ZERO),
            flex_direction: FlexDirection::Row,
            flex_wrap: FlexWrap::NoWrap,
            flex_grow: 0.0,
            flex_shrink: 1.0,
            flex_basis: Length::Auto,
            align_items: ItemAlign::default(),
            justify_items: ItemAlign::default(),
            align_self: None,
            justify_self: None,
            align_content: ContentAlign::default(),
            justify_content: ContentAlign::default(),
            row_gap: Length::ZERO,
            column_gap: Length::ZERO,
            grid_template_rows: Vec::new(),
            grid_template_columns: Vec::new(),
            grid_auto_rows: Vec::new(),
            grid_auto_columns: Vec::new(),
            grid_auto_flow: GridAutoFlow::Row,
            grid_row: GridPlacement::default(),
            grid_column: GridPlacement::default(),
        }
    }
}
