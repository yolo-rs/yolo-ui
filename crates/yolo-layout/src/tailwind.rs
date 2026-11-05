// SPDX-License-Identifier: Apache-2.0
//! Tailwind CSS v3/v4 utility class parser and translator into [`LayoutStyle`],
//! CSS declarations, and semantic design tokens (`bg_token`, `text_token`, `border_token`,
//! `shape_token`, `shadow`, `typescale`).

use serde::Serialize;

use crate::style::{
    ContentAlign, ContentKeyword, Display, FlexDirection, FlexWrap, GridLine, GridPlacement,
    ItemAlign, ItemKeyword, LayoutStyle, Length, Overflow, Position, RepeatCount, Safety, Sides,
    TemplateItem, Track, TrackBreadth,
};

/// Resolved Tailwind utility bundle containing both Taffy-ready [`LayoutStyle`]
/// and semantic visual tokens + CSS declarations.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct TailwindResolution {
    /// Computed Taffy-compatible layout style.
    pub layout: LayoutStyle,
    /// Equivalent CSS declaration pairs `(property, value)`.
    pub declarations: Vec<(String, String)>,
    /// Semantic background color token (e.g. `"primary"`, `"surface-container"`, `"background"`).
    pub bg_token: Option<String>,
    /// Semantic foreground / text color token (e.g. `"on-primary"`, `"on-surface"`, `"on-surface-variant"`).
    pub text_token: Option<String>,
    /// Semantic border color token (e.g. `"outline-variant"`, `"primary"`).
    pub border_token: Option<String>,
    /// Corner radius in logical pixels (`rounded-lg` -> `12.0`).
    pub border_radius_px: Option<f32>,
    /// Semantic shape token (`"extra-small"`, `"small"`, `"medium"`, `"large"`, `"extra-large"`, `"full"`, `"none"`).
    pub shape_token: Option<String>,
    /// Border width in logical pixels (`border` -> `1.0`).
    pub border_width_px: Option<f32>,
    /// Shadow elevation token (`"sm"`, `"md"`, `"lg"`, `"xl"`, `"2xl"`, `"none"`).
    pub shadow: Option<String>,
    /// Elevation level (`0..=5`).
    pub elevation: Option<u8>,
    /// Semantic typography scale token (`"headline-small"`, `"title-large"`, `"body-medium"`, `"label-large"`, etc.).
    pub typescale: Option<String>,
    /// Unrecognized utility classes, if any.
    pub unknown_classes: Vec<String>,
}

impl TailwindResolution {
    /// Formats all translated CSS declarations into an inline `style` string (`prop: val; ...`).
    #[must_use]
    pub fn to_css_declarations(&self) -> String {
        let mut out = String::new();
        for (k, v) in &self.declarations {
            if !out.is_empty() {
                out.push(' ');
            }
            out.push_str(k);
            out.push_str(": ");
            out.push_str(v);
            out.push(';');
        }
        out
    }
}

/// Parses a space-separated string of Tailwind CSS v3/v4 utility classes into a [`TailwindResolution`].
#[must_use]
pub fn resolve_tailwind(classes: &str) -> TailwindResolution {
    let mut res = TailwindResolution::default();
    for raw_token in classes.split_whitespace() {
        // Strip responsive/state variant prefixes such as `sm:`, `md:`, `lg:`, `hover:`, `dark:`, `focus:`
        let token = raw_token.rsplit(':').next().unwrap_or(raw_token);
        if token.is_empty() {
            continue;
        }
        if !apply_tailwind_token(&mut res, token) {
            res.unknown_classes.push(raw_token.to_owned());
        }
    }
    res
}

/// Translates a space-separated Tailwind utility string directly into a [`LayoutStyle`].
#[must_use]
pub fn tailwind_to_layout_style(classes: &str) -> LayoutStyle {
    resolve_tailwind(classes).layout
}

/// Translates a space-separated Tailwind utility string into a CSS declaration string.
#[must_use]
pub fn tailwind_to_css(classes: &str) -> String {
    resolve_tailwind(classes).to_css_declarations()
}

/// Applies a single Tailwind utility token onto a [`TailwindResolution`].
/// Returns `true` if the token was recognized.
pub fn apply_tailwind_token(res: &mut TailwindResolution, token: &str) -> bool {
    let s = &mut res.layout;
    let mut decl = |k: &str, v: &str| {
        res.declarations.push((k.to_owned(), v.to_owned()));
    };

    match token {
        // Display
        "block" => {
            s.display = Display::Block;
            decl("display", "block");
        }
        "flex" | "inline-flex" => {
            s.display = Display::Flex;
            decl("display", "flex");
        }
        "grid" | "inline-grid" => {
            s.display = Display::Grid;
            decl("display", "grid");
        }
        "hidden" => {
            s.display = Display::None;
            decl("display", "none");
        }

        // Position
        "static" => {
            s.position = Position::Static;
            decl("position", "static");
        }
        "relative" => {
            s.position = Position::Relative;
            decl("position", "relative");
        }
        "absolute" => {
            s.position = Position::Absolute;
            decl("position", "absolute");
        }
        "fixed" => {
            s.position = Position::Fixed;
            decl("position", "fixed");
        }
        "inset-0" => {
            s.inset = Sides::all(Length::ZERO);
            decl("inset", "0px");
        }

        // Flex direction & wrap & grow/shrink
        "flex-row" => {
            s.display = Display::Flex;
            s.flex_direction = FlexDirection::Row;
            decl("flex-direction", "row");
        }
        "flex-row-reverse" => {
            s.display = Display::Flex;
            s.flex_direction = FlexDirection::RowReverse;
            decl("flex-direction", "row-reverse");
        }
        "flex-col" => {
            s.display = Display::Flex;
            s.flex_direction = FlexDirection::Column;
            decl("flex-direction", "column");
        }
        "flex-col-reverse" => {
            s.display = Display::Flex;
            s.flex_direction = FlexDirection::ColumnReverse;
            decl("flex-direction", "column-reverse");
        }
        "flex-wrap" => {
            s.flex_wrap = FlexWrap::Wrap;
            decl("flex-wrap", "wrap");
        }
        "flex-nowrap" => {
            s.flex_wrap = FlexWrap::NoWrap;
            decl("flex-wrap", "nowrap");
        }
        "flex-1" => {
            s.flex_grow = 1.0;
            s.flex_shrink = 1.0;
            s.flex_basis = Length::ZERO;
            decl("flex", "1 1 0px");
        }
        "flex-auto" => {
            s.flex_grow = 1.0;
            s.flex_shrink = 1.0;
            s.flex_basis = Length::Auto;
            decl("flex", "1 1 auto");
        }
        "flex-none" => {
            s.flex_grow = 0.0;
            s.flex_shrink = 0.0;
            s.flex_basis = Length::Auto;
            decl("flex", "0 0 auto");
        }
        "grow" | "flex-grow" => {
            s.flex_grow = 1.0;
            decl("flex-grow", "1");
        }
        "grow-0" | "flex-grow-0" => {
            s.flex_grow = 0.0;
            decl("flex-grow", "0");
        }
        "shrink" | "flex-shrink" => {
            s.flex_shrink = 1.0;
            decl("flex-shrink", "1");
        }
        "shrink-0" | "flex-shrink-0" => {
            s.flex_shrink = 0.0;
            decl("flex-shrink", "0");
        }

        // Align items
        "items-start" => {
            s.align_items = ItemAlign {
                keyword: ItemKeyword::FlexStart,
                safety: Safety::Default,
            };
            decl("align-items", "flex-start");
        }
        "items-end" => {
            s.align_items = ItemAlign {
                keyword: ItemKeyword::FlexEnd,
                safety: Safety::Default,
            };
            decl("align-items", "flex-end");
        }
        "items-center" => {
            s.align_items = ItemAlign {
                keyword: ItemKeyword::Center,
                safety: Safety::Default,
            };
            decl("align-items", "center");
        }
        "items-baseline" => {
            s.align_items = ItemAlign {
                keyword: ItemKeyword::Baseline,
                safety: Safety::Default,
            };
            decl("align-items", "baseline");
        }
        "items-stretch" => {
            s.align_items = ItemAlign {
                keyword: ItemKeyword::Stretch,
                safety: Safety::Default,
            };
            decl("align-items", "stretch");
        }

        // Justify content
        "justify-start" => {
            s.justify_content = ContentAlign {
                keyword: ContentKeyword::FlexStart,
                safety: Safety::Default,
            };
            decl("justify-content", "flex-start");
        }
        "justify-end" => {
            s.justify_content = ContentAlign {
                keyword: ContentKeyword::FlexEnd,
                safety: Safety::Default,
            };
            decl("justify-content", "flex-end");
        }
        "justify-center" => {
            s.justify_content = ContentAlign {
                keyword: ContentKeyword::Center,
                safety: Safety::Default,
            };
            decl("justify-content", "center");
        }
        "justify-between" => {
            s.justify_content = ContentAlign {
                keyword: ContentKeyword::SpaceBetween,
                safety: Safety::Default,
            };
            decl("justify-content", "space-between");
        }
        "justify-around" => {
            s.justify_content = ContentAlign {
                keyword: ContentKeyword::SpaceAround,
                safety: Safety::Default,
            };
            decl("justify-content", "space-around");
        }
        "justify-evenly" => {
            s.justify_content = ContentAlign {
                keyword: ContentKeyword::SpaceEvenly,
                safety: Safety::Default,
            };
            decl("justify-content", "space-evenly");
        }

        // Self alignment
        "self-auto" => {
            s.align_self = None;
            decl("align-self", "auto");
        }
        "self-start" => {
            s.align_self = Some(ItemAlign {
                keyword: ItemKeyword::FlexStart,
                safety: Safety::Default,
            });
            decl("align-self", "flex-start");
        }
        "self-end" => {
            s.align_self = Some(ItemAlign {
                keyword: ItemKeyword::FlexEnd,
                safety: Safety::Default,
            });
            decl("align-self", "flex-end");
        }
        "self-center" => {
            s.align_self = Some(ItemAlign {
                keyword: ItemKeyword::Center,
                safety: Safety::Default,
            });
            decl("align-self", "center");
        }
        "self-stretch" => {
            s.align_self = Some(ItemAlign {
                keyword: ItemKeyword::Stretch,
                safety: Safety::Default,
            });
            decl("align-self", "stretch");
        }

        // Width & Height keywords
        "w-full" => {
            s.width = Length::Percent(1.0);
            decl("width", "100%");
        }
        "w-auto" => {
            s.width = Length::Auto;
            decl("width", "auto");
        }
        "w-screen" => {
            s.width = Length::Percent(1.0);
            decl("width", "100%");
        }
        "h-full" => {
            s.height = Length::Percent(1.0);
            decl("height", "100%");
        }
        "h-auto" => {
            s.height = Length::Auto;
            decl("height", "auto");
        }
        "h-screen" => {
            s.height = Length::Px(900.0);
            decl("height", "900px");
        }
        "min-h-screen" => {
            s.min_height = Length::Px(900.0);
            decl("min-height", "900px");
        }
        "min-h-full" => {
            s.min_height = Length::Percent(1.0);
            decl("min-height", "100%");
        }
        "min-w-full" => {
            s.min_width = Length::Percent(1.0);
            decl("min-width", "100%");
        }
        "min-w-0" => {
            s.min_width = Length::ZERO;
            decl("min-width", "0px");
        }
        "max-w-sm" => {
            s.max_width = Length::Px(384.0);
            decl("max-width", "384px");
        }
        "max-w-md" => {
            s.max_width = Length::Px(448.0);
            decl("max-width", "448px");
        }
        "max-w-lg" => {
            s.max_width = Length::Px(512.0);
            decl("max-width", "512px");
        }
        "max-w-xl" => {
            s.max_width = Length::Px(576.0);
            decl("max-width", "576px");
        }
        "max-w-2xl" => {
            s.max_width = Length::Px(672.0);
            decl("max-width", "672px");
        }
        "max-w-3xl" => {
            s.max_width = Length::Px(768.0);
            decl("max-width", "768px");
        }
        "max-w-4xl" => {
            s.max_width = Length::Px(896.0);
            decl("max-width", "896px");
        }
        "max-w-5xl" => {
            s.max_width = Length::Px(1024.0);
            decl("max-width", "1024px");
        }
        "max-w-full" => {
            s.max_width = Length::Percent(1.0);
            decl("max-width", "100%");
        }
        "mx-auto" => {
            s.margin.left = Length::Auto;
            s.margin.right = Length::Auto;
            decl("margin-left", "auto");
            decl("margin-right", "auto");
        }

        // Overflow
        "overflow-hidden" => {
            s.overflow_x = Overflow::Hidden;
            s.overflow_y = Overflow::Hidden;
            decl("overflow", "hidden");
        }
        "overflow-clip" => {
            s.overflow_x = Overflow::Clip;
            s.overflow_y = Overflow::Clip;
            decl("overflow", "clip");
        }
        "overflow-visible" => {
            s.overflow_x = Overflow::Visible;
            s.overflow_y = Overflow::Visible;
            decl("overflow", "visible");
        }
        "overflow-scroll" | "overflow-auto" | "overflow-y-auto" | "overflow-y-scroll" => {
            s.overflow_y = Overflow::Scroll;
            decl("overflow-y", "scroll");
        }
        "overflow-x-auto" | "overflow-x-scroll" => {
            s.overflow_x = Overflow::Scroll;
            decl("overflow-x", "scroll");
        }

        // Rounded / Corner Radius
        "rounded-none" => {
            res.border_radius_px = Some(0.0);
            res.shape_token = Some("none".into());
            decl("border-radius", "var(--md-sys-shape-corner-none)");
        }
        "rounded-sm" => {
            res.border_radius_px = Some(4.0);
            res.shape_token = Some("extra-small".into());
            decl("border-radius", "var(--md-sys-shape-corner-extra-small)");
        }
        "rounded" | "rounded-md" => {
            res.border_radius_px = Some(8.0);
            res.shape_token = Some("small".into());
            decl("border-radius", "var(--md-sys-shape-corner-small)");
        }
        "rounded-lg" => {
            res.border_radius_px = Some(12.0);
            res.shape_token = Some("medium".into());
            decl("border-radius", "var(--md-sys-shape-corner-medium)");
        }
        "rounded-xl" => {
            res.border_radius_px = Some(16.0);
            res.shape_token = Some("large".into());
            decl("border-radius", "var(--md-sys-shape-corner-large)");
        }
        "rounded-2xl" | "rounded-3xl" => {
            res.border_radius_px = Some(28.0);
            res.shape_token = Some("extra-large".into());
            decl("border-radius", "var(--md-sys-shape-corner-extra-large)");
        }
        "rounded-full" => {
            res.border_radius_px = Some(9999.0);
            res.shape_token = Some("full".into());
            decl("border-radius", "var(--md-sys-shape-corner-full)");
        }

        // Shadows & Elevation
        "shadow-none" => {
            res.shadow = Some("none".into());
            res.elevation = Some(0);
        }
        "shadow-xs" | "shadow-sm" => {
            res.shadow = Some("sm".into());
            res.elevation = Some(1);
        }
        "shadow" | "shadow-md" => {
            res.shadow = Some("md".into());
            res.elevation = Some(2);
        }
        "shadow-lg" => {
            res.shadow = Some("lg".into());
            res.elevation = Some(3);
        }
        "shadow-xl" => {
            res.shadow = Some("xl".into());
            res.elevation = Some(4);
        }
        "shadow-2xl" => {
            res.shadow = Some("2xl".into());
            res.elevation = Some(5);
        }

        // Borders
        "border" => {
            res.border_width_px = Some(1.0);
            if res.border_token.is_none() {
                res.border_token = Some("outline-variant".into());
            }
        }
        "border-0" => {
            res.border_width_px = Some(0.0);
        }
        "border-2" => {
            res.border_width_px = Some(2.0);
            if res.border_token.is_none() {
                res.border_token = Some("outline-variant".into());
            }
        }
        "border-4" => {
            res.border_width_px = Some(4.0);
            if res.border_token.is_none() {
                res.border_token = Some("outline-variant".into());
            }
        }
        "border-border" | "border-input" | "border-muted" => {
            res.border_token = Some("outline-variant".into());
        }
        "border-primary" => {
            res.border_token = Some("primary".into());
        }
        "border-destructive" => {
            res.border_token = Some("error".into());
        }

        // Semantic Background Tokens (Shadcn / Radix / M3 / Tailwind)
        "bg-primary" => {
            res.bg_token = Some("primary".into());
            decl("background-color", "var(--md-sys-color-primary)");
        }
        "bg-secondary" => {
            res.bg_token = Some("secondary-container".into());
            decl(
                "background-color",
                "var(--md-sys-color-secondary-container)",
            );
        }
        "bg-card" => {
            res.bg_token = Some("surface-container".into());
            decl("background-color", "var(--md-sys-color-surface-container)");
        }
        "bg-popover" => {
            res.bg_token = Some("surface-container-high".into());
            decl(
                "background-color",
                "var(--md-sys-color-surface-container-high)",
            );
        }
        "bg-background" | "bg-surface" => {
            res.bg_token = Some("surface".into());
            decl("background-color", "var(--md-sys-color-surface)");
        }
        "bg-muted" => {
            res.bg_token = Some("surface-variant".into());
            decl("background-color", "var(--md-sys-color-surface-variant)");
        }
        "bg-accent" => {
            res.bg_token = Some("tertiary-container".into());
            decl("background-color", "var(--md-sys-color-tertiary-container)");
        }
        "bg-destructive" => {
            res.bg_token = Some("error".into());
            decl("background-color", "var(--md-sys-color-error)");
        }
        "bg-transparent" => {
            res.bg_token = None;
        }

        // Semantic Text Color Tokens
        "text-primary" => {
            res.text_token = Some("primary".into());
            decl("color", "var(--md-sys-color-primary)");
        }
        "text-primary-foreground" => {
            res.text_token = Some("on-primary".into());
            decl("color", "var(--md-sys-color-on-primary)");
        }
        "text-secondary" => {
            res.text_token = Some("secondary".into());
            decl("color", "var(--md-sys-color-secondary)");
        }
        "text-secondary-foreground" => {
            res.text_token = Some("on-secondary-container".into());
            decl("color", "var(--md-sys-color-on-secondary-container)");
        }
        "text-card-foreground" | "text-foreground" | "text-popover-foreground" => {
            res.text_token = Some("on-surface".into());
            decl("color", "var(--md-sys-color-on-surface)");
        }
        "text-muted-foreground" => {
            res.text_token = Some("on-surface-variant".into());
            decl("color", "var(--md-sys-color-on-surface-variant)");
        }
        "text-accent-foreground" => {
            res.text_token = Some("on-tertiary-container".into());
            decl("color", "var(--md-sys-color-on-tertiary-container)");
        }
        "text-destructive" => {
            res.text_token = Some("error".into());
            decl("color", "var(--md-sys-color-error)");
        }
        "text-destructive-foreground" => {
            res.text_token = Some("on-error".into());
            decl("color", "var(--md-sys-color-on-error)");
        }

        // Text alignment & typography scale
        "text-left" => {
            decl("text-align", "left");
        }
        "text-center" => {
            decl("text-align", "center");
        }
        "text-right" => {
            decl("text-align", "right");
        }
        "text-xs" => {
            res.typescale = Some("label-small".into());
        }
        "text-sm" => {
            res.typescale = Some("body-small".into());
        }
        "text-base" => {
            res.typescale = Some("body-medium".into());
        }
        "text-lg" => {
            res.typescale = Some("title-medium".into());
        }
        "text-xl" => {
            res.typescale = Some("title-large".into());
        }
        "text-2xl" => {
            res.typescale = Some("headline-small".into());
        }
        "text-3xl" => {
            res.typescale = Some("headline-medium".into());
        }
        "text-4xl" | "text-5xl" => {
            res.typescale = Some("headline-large".into());
        }
        "font-normal" | "font-medium" | "font-semibold" | "font-bold" | "tracking-tight"
        | "tracking-wide" | "leading-none" | "leading-tight" | "leading-normal"
        | "leading-relaxed" | "antialiased" | "select-none" | "cursor-pointer" | "transition"
        | "transition-colors" | "transition-all" => {
            // Accepted cosmetic/typographic utilities
        }

        _ => return apply_parametric_token(res, token),
    }

    true
}

fn apply_parametric_token(res: &mut TailwindResolution, token: &str) -> bool {
    let s = &mut res.layout;
    let mut decl = |k: &str, v: String| {
        res.declarations.push((k.to_owned(), v));
    };

    // Grid columns: `grid-cols-1`..`grid-cols-12`
    if let Some(cols) = token.strip_prefix("grid-cols-")
        && let Ok(n) = cols.parse::<u16>()
        && n > 0
    {
        s.display = Display::Grid;
        s.grid_template_columns = vec![TemplateItem::Repeat(
            RepeatCount::Count(n),
            vec![Track::Single(TrackBreadth::Fr(1.0))],
        )];
        decl("grid-template-columns", format!("repeat({n}, 1fr)"));
        return true;
    }

    // Grid rows: `grid-rows-1`..`grid-rows-12`
    if let Some(rows) = token.strip_prefix("grid-rows-")
        && let Ok(n) = rows.parse::<u16>()
        && n > 0
    {
        s.display = Display::Grid;
        s.grid_template_rows = vec![TemplateItem::Repeat(
            RepeatCount::Count(n),
            vec![Track::Single(TrackBreadth::Fr(1.0))],
        )];
        decl("grid-template-rows", format!("repeat({n}, 1fr)"));
        return true;
    }

    // Grid spans: `col-span-N`, `row-span-N`
    if let Some(span) = token.strip_prefix("col-span-")
        && let Ok(n) = span.parse::<u16>()
        && n > 0
    {
        s.grid_column = GridPlacement {
            start: GridLine::Span(n),
            end: GridLine::Auto,
        };
        decl("grid-column", format!("span {n}"));
        return true;
    }
    if let Some(span) = token.strip_prefix("row-span-")
        && let Ok(n) = span.parse::<u16>()
        && n > 0
    {
        s.grid_row = GridPlacement {
            start: GridLine::Span(n),
            end: GridLine::Auto,
        };
        decl("grid-row", format!("span {n}"));
        return true;
    }

    // Gap: `gap-*`, `gap-x-*`, `gap-y-*`, `space-y-*`, `space-x-*`
    if let Some(rest) = token
        .strip_prefix("gap-x-")
        .or_else(|| token.strip_prefix("space-x-"))
        && let Some(px) = parse_spacing_scale(rest)
    {
        s.column_gap = Length::Px(px);
        decl("column-gap", format!("{px}px"));
        return true;
    }
    if let Some(rest) = token
        .strip_prefix("gap-y-")
        .or_else(|| token.strip_prefix("space-y-"))
        && let Some(px) = parse_spacing_scale(rest)
    {
        s.row_gap = Length::Px(px);
        decl("row-gap", format!("{px}px"));
        return true;
    }
    if let Some(rest) = token.strip_prefix("gap-")
        && let Some(px) = parse_spacing_scale(rest)
    {
        s.row_gap = Length::Px(px);
        s.column_gap = Length::Px(px);
        decl("gap", format!("{px}px"));
        return true;
    }

    // Padding: `p-*`, `px-*`, `py-*`, `pt-*`, `pr-*`, `pb-*`, `pl-*`
    if let Some(rest) = token.strip_prefix("px-")
        && let Some(px) = parse_spacing_scale(rest)
    {
        s.padding.left = Length::Px(px);
        s.padding.right = Length::Px(px);
        decl("padding-left", format!("{px}px"));
        decl("padding-right", format!("{px}px"));
        return true;
    }
    if let Some(rest) = token.strip_prefix("py-")
        && let Some(px) = parse_spacing_scale(rest)
    {
        s.padding.top = Length::Px(px);
        s.padding.bottom = Length::Px(px);
        decl("padding-top", format!("{px}px"));
        decl("padding-bottom", format!("{px}px"));
        return true;
    }
    if let Some(rest) = token.strip_prefix("pt-")
        && let Some(px) = parse_spacing_scale(rest)
    {
        s.padding.top = Length::Px(px);
        decl("padding-top", format!("{px}px"));
        return true;
    }
    if let Some(rest) = token.strip_prefix("pr-")
        && let Some(px) = parse_spacing_scale(rest)
    {
        s.padding.right = Length::Px(px);
        decl("padding-right", format!("{px}px"));
        return true;
    }
    if let Some(rest) = token.strip_prefix("pb-")
        && let Some(px) = parse_spacing_scale(rest)
    {
        s.padding.bottom = Length::Px(px);
        decl("padding-bottom", format!("{px}px"));
        return true;
    }
    if let Some(rest) = token.strip_prefix("pl-")
        && let Some(px) = parse_spacing_scale(rest)
    {
        s.padding.left = Length::Px(px);
        decl("padding-left", format!("{px}px"));
        return true;
    }
    if let Some(rest) = token.strip_prefix("p-")
        && let Some(px) = parse_spacing_scale(rest)
    {
        s.padding = Sides::all(Length::Px(px));
        decl("padding", format!("{px}px"));
        return true;
    }

    // Margin: `m-*`, `mx-*`, `my-*`, `mt-*`, `mr-*`, `mb-*`, `ml-*`
    if let Some(rest) = token.strip_prefix("mx-")
        && let Some(px) = parse_spacing_scale(rest)
    {
        s.margin.left = Length::Px(px);
        s.margin.right = Length::Px(px);
        decl("margin-left", format!("{px}px"));
        decl("margin-right", format!("{px}px"));
        return true;
    }
    if let Some(rest) = token.strip_prefix("my-")
        && let Some(px) = parse_spacing_scale(rest)
    {
        s.margin.top = Length::Px(px);
        s.margin.bottom = Length::Px(px);
        decl("margin-top", format!("{px}px"));
        decl("margin-bottom", format!("{px}px"));
        return true;
    }
    if let Some(rest) = token.strip_prefix("mt-")
        && let Some(px) = parse_spacing_scale(rest)
    {
        s.margin.top = Length::Px(px);
        decl("margin-top", format!("{px}px"));
        return true;
    }
    if let Some(rest) = token.strip_prefix("mr-")
        && let Some(px) = parse_spacing_scale(rest)
    {
        s.margin.right = Length::Px(px);
        decl("margin-right", format!("{px}px"));
        return true;
    }
    if let Some(rest) = token.strip_prefix("mb-")
        && let Some(px) = parse_spacing_scale(rest)
    {
        s.margin.bottom = Length::Px(px);
        decl("margin-bottom", format!("{px}px"));
        return true;
    }
    if let Some(rest) = token.strip_prefix("ml-")
        && let Some(px) = parse_spacing_scale(rest)
    {
        s.margin.left = Length::Px(px);
        decl("margin-left", format!("{px}px"));
        return true;
    }
    if let Some(rest) = token.strip_prefix("m-")
        && let Some(px) = parse_spacing_scale(rest)
    {
        s.margin = Sides::all(Length::Px(px));
        decl("margin", format!("{px}px"));
        return true;
    }

    // Width & Height numeric / percentage / arbitrary: `w-64`, `w-1/2`, `w-[320px]`, `h-10`
    if let Some(rest) = token.strip_prefix("w-")
        && let Some((len, css)) = parse_dimension_token(rest)
    {
        s.width = len;
        decl("width", css);
        return true;
    }
    if let Some(rest) = token.strip_prefix("h-")
        && let Some((len, css)) = parse_dimension_token(rest)
    {
        s.height = len;
        decl("height", css);
        return true;
    }
    if let Some(rest) = token.strip_prefix("min-w-")
        && let Some((len, css)) = parse_dimension_token(rest)
    {
        s.min_width = len;
        decl("min-width", css);
        return true;
    }
    if let Some(rest) = token.strip_prefix("min-h-")
        && let Some((len, css)) = parse_dimension_token(rest)
    {
        s.min_height = len;
        decl("min-height", css);
        return true;
    }
    if let Some(rest) = token.strip_prefix("max-w-")
        && let Some((len, css)) = parse_dimension_token(rest)
    {
        s.max_width = len;
        decl("max-width", css);
        return true;
    }
    if let Some(rest) = token.strip_prefix("max-h-")
        && let Some((len, css)) = parse_dimension_token(rest)
    {
        s.max_height = len;
        decl("max-height", css);
        return true;
    }

    // Palette color fallback (`bg-slate-900`, `text-zinc-400`, `border-slate-800`, etc.)
    if let Some(color_rest) = token.strip_prefix("bg-") {
        if let Some(role) = map_palette_to_role(color_rest, true) {
            res.bg_token = Some(role.to_owned());
            decl("background-color", format!("var(--md-sys-color-{role})"));
            return true;
        }
    }
    if let Some(color_rest) = token.strip_prefix("text-") {
        if let Some(role) = map_palette_to_role(color_rest, false) {
            res.text_token = Some(role.to_owned());
            decl("color", format!("var(--md-sys-color-{role})"));
            return true;
        }
    }
    if let Some(color_rest) = token.strip_prefix("border-") {
        if let Some(role) = map_palette_to_role(color_rest, false) {
            res.border_token = Some(role.to_owned());
            return true;
        }
    }

    false
}

/// Converts Tailwind's 4px spacing scale (`0`, `px`, `0.5`, `1`, `2`, `4`, `[16px]`, etc.) into logical pixels.
fn parse_spacing_scale(raw: &str) -> Option<f32> {
    if raw == "px" {
        return Some(1.0);
    }
    if let Some(inner) = raw.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
        if let Some(px) = inner.strip_suffix("px") {
            return px.parse::<f32>().ok();
        }
        if let Some(rem) = inner.strip_suffix("rem") {
            return rem.parse::<f32>().ok().map(|v| v * 16.0);
        }
        return inner.parse::<f32>().ok();
    }
    raw.parse::<f32>().ok().map(|n| n * 4.0)
}

fn parse_dimension_token(raw: &str) -> Option<(Length, String)> {
    if let Some((num, den)) = raw.split_once('/')
        && let (Ok(n), Ok(d)) = (num.parse::<f32>(), den.parse::<f32>())
        && d > 0.0
    {
        let frac = (n / d).clamp(0.0, 1.0);
        return Some((Length::Percent(frac), format!("{}%", frac * 100.0)));
    }
    if let Some(inner) = raw.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
        if let Some(pct) = inner.strip_suffix('%')
            && let Ok(v) = pct.parse::<f32>()
        {
            return Some((Length::Percent(v / 100.0), format!("{v}%")));
        }
    }
    let px = parse_spacing_scale(raw)?;
    Some((Length::Px(px), format!("{px}px")))
}

fn map_palette_to_role(color: &str, is_bg: bool) -> Option<&'static str> {
    let base = color.split('/').next().unwrap_or(color);
    if base == "white" || base == "black" {
        return Some(if is_bg { "surface" } else { "on-surface" });
    }
    let (hue, shade) = base.rsplit_once('-')?;
    let shade_num = shade.parse::<u16>().ok()?;
    match hue {
        "slate" | "gray" | "zinc" | "neutral" | "stone" => Some(if is_bg {
            if shade_num >= 800 {
                "surface-container"
            } else {
                "surface-variant"
            }
        } else if shade_num <= 300 || shade_num >= 800 {
            "on-surface"
        } else {
            "on-surface-variant"
        }),
        "red" | "rose" => Some(if is_bg { "error-container" } else { "error" }),
        "indigo" | "violet" | "purple" | "blue" => Some("primary"),
        "cyan" | "teal" | "emerald" | "green" => Some(if is_bg {
            "secondary-container"
        } else {
            "secondary"
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::monospace_measure;
    use crate::tree::LayoutTree;

    #[test]
    fn resolves_tailwind_flex_and_grid_utilities() {
        let res = resolve_tailwind(
            "flex flex-col items-center justify-between gap-4 p-4 px-6 py-2 m-2 w-full min-h-screen rounded-lg shadow-md border bg-primary text-primary-foreground",
        );
        assert!(
            res.unknown_classes.is_empty(),
            "unknown: {:?}",
            res.unknown_classes
        );
        assert_eq!(res.layout.display, Display::Flex);
        assert_eq!(res.layout.flex_direction, FlexDirection::Column);
        assert_eq!(res.layout.align_items.keyword, ItemKeyword::Center);
        assert_eq!(
            res.layout.justify_content.keyword,
            ContentKeyword::SpaceBetween
        );
        assert_eq!(res.layout.row_gap, Length::Px(16.0));
        assert_eq!(res.layout.column_gap, Length::Px(16.0));
        assert_eq!(res.layout.padding.left, Length::Px(24.0));
        assert_eq!(res.layout.padding.right, Length::Px(24.0));
        assert_eq!(res.layout.padding.top, Length::Px(8.0));
        assert_eq!(res.layout.padding.bottom, Length::Px(8.0));
        assert_eq!(res.layout.margin, Sides::all(Length::Px(8.0)));
        assert_eq!(res.layout.width, Length::Percent(1.0));
        assert_eq!(res.layout.min_height, Length::Px(900.0));
        assert_eq!(res.border_radius_px, Some(12.0));
        assert_eq!(res.shape_token.as_deref(), Some("medium"));
        assert_eq!(res.shadow.as_deref(), Some("md"));
        assert_eq!(res.elevation, Some(2));
        assert_eq!(res.border_width_px, Some(1.0));
        assert_eq!(res.bg_token.as_deref(), Some("primary"));
        assert_eq!(res.text_token.as_deref(), Some("on-primary"));
    }

    #[test]
    fn resolves_grid_cols_and_computes_taffy_layout() {
        let grid_style = tailwind_to_layout_style("grid grid-cols-3 gap-4 p-4 w-full");
        let card_style = tailwind_to_layout_style(
            "flex flex-col gap-2 p-4 h-24 rounded-xl bg-card text-muted-foreground",
        );
        let mut tree = LayoutTree::new();
        let c1 = tree.node("c1", &card_style, &[]).unwrap();
        let c2 = tree.node("c2", &card_style, &[]).unwrap();
        let c3 = tree.node("c3", &card_style, &[]).unwrap();
        let root = tree.node("grid", &grid_style, &[c1, c2, c3]).unwrap();
        tree.compute(root, Some(640.0), Some(480.0), monospace_measure(8.0, 20.0))
            .unwrap();

        let r1 = tree.rect(c1).unwrap();
        let r2 = tree.rect(c2).unwrap();
        let r3 = tree.rect(c3).unwrap();
        assert!(r1.width > 180.0 && r1.width < 200.0);
        assert!(r2.x > r1.x + r1.width);
        assert!(r3.x > r2.x + r2.width);
        assert_eq!(r1.height, 96.0);
    }
}
