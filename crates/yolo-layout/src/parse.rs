// SPDX-License-Identifier: Apache-2.0
//! CSS subset -> [`LayoutStyle`], through Lightning CSS. Fails closed: every unsupported
//! property, value, unit, selector or at-rule is a [`Diagnostic`] with its line and column.

use core::fmt;

use lightningcss::error::Error as CssError;
use lightningcss::properties::align::{
    AlignContent, AlignItems, AlignSelf, BaselinePosition, ContentDistribution, ContentPosition,
    GapValue, JustifyContent, JustifyItems, JustifySelf, OverflowPosition, SelfPosition,
};
use lightningcss::properties::display::{
    Display as CssDisplay, DisplayInside, DisplayKeyword, DisplayOutside,
};
use lightningcss::properties::flex::{FlexDirection as CssFlexDirection, FlexWrap as CssFlexWrap};
use lightningcss::properties::grid::{
    GridAutoFlow as CssGridAutoFlow, GridLine as CssGridLine,
    GridTemplateAreas as CssGridTemplateAreas, RepeatCount as CssRepeatCount,
    TrackBreadth as CssTrackBreadth, TrackListItem, TrackSize, TrackSizing,
};
use lightningcss::properties::overflow::OverflowKeyword;
use lightningcss::properties::position::Position as CssPosition;
use lightningcss::properties::size::{BoxSizing as CssBoxSizing, MaxSize, Size};
use lightningcss::properties::{Property, PropertyId};
use lightningcss::stylesheet::{ParserOptions, StyleAttribute, StyleSheet as CssStyleSheet};
use lightningcss::values::length::{LengthPercentage, LengthPercentageOrAuto};

use crate::style::*;
use crate::tokens::TokenTable;

/// One parse or conversion error, located in the source (1-based line and column).
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct Diagnostic {
    /// Line, from 1.
    pub line: u32,
    /// Column, from 1.
    pub column: u32,
    /// The property concerned, when there is one.
    pub property: Option<String>,
    /// What is wrong and, when possible, what is supported instead.
    pub message: String,
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.property {
            Some(p) => write!(f, "{}:{}: `{p}`: {}", self.line, self.column, self.message),
            None => write!(f, "{}:{}: {}", self.line, self.column, self.message),
        }
    }
}

/// Every diagnostic of one parse; the parse produced nothing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostics(pub Vec<Diagnostic>);

impl fmt::Display for Diagnostics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, d) in self.0.iter().enumerate() {
            if i > 0 {
                f.write_str("\n")?;
            }
            write!(f, "{d}")?;
        }
        Ok(())
    }
}

impl std::error::Error for Diagnostics {}

/// A validated declaration with `var()` already resolved.
#[derive(Clone, Debug, PartialEq)]
struct Declaration {
    name: String,
    value: String,
}

/// A rule: simple selectors and their validated declarations, in source order.
#[derive(Clone, Debug, PartialEq)]
pub struct Rule {
    /// `.class`, `#id` or `tag` selectors.
    pub selectors: Vec<String>,
    declarations: Vec<Declaration>,
}

/// A parsed layout stylesheet.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct StyleSheet {
    /// Rules in source order.
    pub rules: Vec<Rule>,
    /// The context the sheet was parsed with.
    pub context: LengthContext,
}

impl StyleSheet {
    /// The cascaded style of a node matched by any of `selectors` (`".row"`, `"#title"`,
    /// `"section"`), applying matching rules in source order.
    #[must_use]
    pub fn style(&self, selectors: &[&str]) -> LayoutStyle {
        self.style_in(selectors, &self.context)
    }

    /// Like [`Self::style`], but resolves `rem`, `em`, `vw` and `vh` against `cx` instead of the
    /// context the sheet was parsed with (a per-node parent font size, a resized viewport).
    /// A viewport unit with no viewport in `cx` resolves to nothing: that declaration is skipped.
    #[must_use]
    pub fn style_in(&self, selectors: &[&str], cx: &LengthContext) -> LayoutStyle {
        let mut style = LayoutStyle::default();
        for rule in &self.rules {
            if rule
                .selectors
                .iter()
                .any(|s| selectors.contains(&s.as_str()))
            {
                for d in &rule.declarations {
                    // Validated at parse time; re-applying cannot fail.
                    let _ = apply(&mut style, &d.name, &d.value, cx);
                }
            }
        }
        style
    }

    /// Whether any rule names `selector`.
    #[must_use]
    pub fn has(&self, selector: &str) -> bool {
        self.rules
            .iter()
            .any(|r| r.selectors.iter().any(|s| s == selector))
    }
}

/// Parses a declaration list (the body of a `style` attribute) into one style.
pub fn parse_style(css: &str, tokens: &TokenTable) -> Result<LayoutStyle, Diagnostics> {
    parse_style_in(css, tokens, &LengthContext::default())
}

/// Like [`parse_style`], resolving `rem`, `em`, `vw` and `vh` against `cx`.
pub fn parse_style_in(
    css: &str,
    tokens: &TokenTable,
    cx: &LengthContext,
) -> Result<LayoutStyle, Diagnostics> {
    let src = blank_comments(css);
    let lines = LineIndex::new(&src);
    let mut diags = Vec::new();
    if let Err(e) = StyleAttribute::parse(&src, ParserOptions::default()) {
        diags.push(css_error(&e));
        return Err(Diagnostics(diags));
    }
    let mut style = LayoutStyle::default();
    for d in declarations(&src, 0, src.len(), &lines, tokens, cx, &mut diags) {
        let _ = apply(&mut style, &d.name, &d.value, cx);
    }
    if diags.is_empty() {
        Ok(style)
    } else {
        Err(Diagnostics(diags))
    }
}

/// Parses a stylesheet of simple-selector rules (`.page { display: grid; }`).
pub fn parse_stylesheet(css: &str, tokens: &TokenTable) -> Result<StyleSheet, Diagnostics> {
    parse_stylesheet_in(css, tokens, &LengthContext::default())
}

/// Like [`parse_stylesheet`], resolving `rem`, `em`, `vw` and `vh` against `cx`.
pub fn parse_stylesheet_in(
    css: &str,
    tokens: &TokenTable,
    cx: &LengthContext,
) -> Result<StyleSheet, Diagnostics> {
    let src = blank_comments(css);
    let lines = LineIndex::new(&src);
    let mut diags = Vec::new();
    if let Err(e) = CssStyleSheet::parse(&src, ParserOptions::default()) {
        diags.push(css_error(&e));
        return Err(Diagnostics(diags));
    }
    let bytes = src.as_bytes();
    let mut rules = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }
        let sel_start = i;
        let Some(open) = find_top_level(bytes, i, b'{') else {
            let (line, column) = lines.at(sel_start);
            diags.push(Diagnostic {
                line,
                column,
                property: None,
                message: "expected `{` after the selector".into(),
            });
            break;
        };
        let close = matching_brace(bytes, open).unwrap_or(bytes.len());
        let prelude = src[sel_start..open].trim();
        let (line, column) = lines.at(sel_start);
        if prelude.starts_with('@') {
            diags.push(Diagnostic {
                line,
                column,
                property: None,
                message: format!(
                    "at-rule `{}` is not supported; layout CSS is flat rules only",
                    prelude.split_whitespace().next().unwrap_or(prelude)
                ),
            });
            i = close + 1;
            continue;
        }
        let mut selectors = Vec::new();
        for sel in prelude.split(',') {
            let sel = sel.trim();
            if is_simple_selector(sel) {
                selectors.push(sel.to_owned());
            } else {
                diags.push(Diagnostic {
                    line,
                    column,
                    property: None,
                    message: format!(
                        "selector `{sel}` is not supported; use one `.class`, `#id` or `tag` per selector"
                    ),
                });
            }
        }
        if src[open + 1..close].contains('{') {
            let (l, c) = lines.at(open + 1 + src[open + 1..close].find('{').unwrap_or(0));
            diags.push(Diagnostic {
                line: l,
                column: c,
                property: None,
                message: "nested rules are not supported".into(),
            });
        }
        let declarations = declarations(&src, open + 1, close, &lines, tokens, cx, &mut diags);
        rules.push(Rule {
            selectors,
            declarations,
        });
        i = close + 1;
    }
    if diags.is_empty() {
        Ok(StyleSheet {
            rules,
            context: *cx,
        })
    } else {
        Err(Diagnostics(diags))
    }
}

fn css_error(e: &CssError<lightningcss::error::ParserError<'_>>) -> Diagnostic {
    let (line, column) = e.loc.as_ref().map_or((1, 1), |l| (l.line + 1, l.column));
    Diagnostic {
        line,
        column,
        property: None,
        message: format!("invalid CSS: {}", e.kind),
    }
}

fn is_simple_selector(sel: &str) -> bool {
    let body = sel.strip_prefix(['.', '#']).unwrap_or(sel);
    let mut chars = body.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_alphabetic() || c == '_' || c == '-')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// Replaces comments with spaces so offsets, lines and columns stay those of the source.
fn blank_comments(css: &str) -> String {
    let mut bytes = css.as_bytes().to_vec();
    let mut i = 0;
    while i + 1 < bytes.len() {
        if bytes[i] == b'/' && bytes[i + 1] == b'*' {
            let end = css[i + 2..]
                .find("*/")
                .map_or(bytes.len(), |e| i + 2 + e + 2);
            for b in &mut bytes[i..end] {
                if *b != b'\n' {
                    *b = b' ';
                }
            }
            i = end;
        } else {
            i += 1;
        }
    }
    // Only whole comments (ASCII-delimited) were blanked, so the bytes stay UTF-8.
    String::from_utf8(bytes).unwrap_or_else(|_| css.to_owned())
}

struct LineIndex(Vec<usize>);

impl LineIndex {
    fn new(src: &str) -> Self {
        let mut starts = vec![0];
        starts.extend(src.match_indices('\n').map(|(i, _)| i + 1));
        Self(starts)
    }

    fn at(&self, offset: usize) -> (u32, u32) {
        let line = self.0.partition_point(|&s| s <= offset).saturating_sub(1);
        let col = offset - self.0[line] + 1;
        (
            u32::try_from(line + 1).unwrap_or(u32::MAX),
            u32::try_from(col).unwrap_or(u32::MAX),
        )
    }
}

fn find_top_level(bytes: &[u8], from: usize, target: u8) -> Option<usize> {
    let mut depth = 0i32;
    let mut quote = None;
    for (i, &b) in bytes.iter().enumerate().skip(from) {
        match (quote, b) {
            (Some(q), _) if b == q => quote = None,
            (Some(_), _) => {}
            (None, b'"' | b'\'') => quote = Some(b),
            (None, b'(') => depth += 1,
            (None, b')') => depth -= 1,
            (None, _) if b == target && depth == 0 => return Some(i),
            _ => {}
        }
    }
    None
}

fn matching_brace(bytes: &[u8], open: usize) -> Option<usize> {
    let mut depth = 0i32;
    let mut quote = None;
    for (i, &b) in bytes.iter().enumerate().skip(open) {
        match (quote, b) {
            (Some(q), _) if b == q => quote = None,
            (Some(_), _) => {}
            (None, b'"' | b'\'') => quote = Some(b),
            (None, b'{') => depth += 1,
            (None, b'}') => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}

/// Splits `src[start..end]` into declarations, resolves `var()` and validates each one.
fn declarations(
    src: &str,
    start: usize,
    end: usize,
    lines: &LineIndex,
    tokens: &TokenTable,
    cx: &LengthContext,
    diags: &mut Vec<Diagnostic>,
) -> Vec<Declaration> {
    let bytes = &src.as_bytes()[..end];
    let mut out = Vec::new();
    let mut i = start;
    while i < end {
        let stop = find_top_level(bytes, i, b';').unwrap_or(end);
        let raw = &src[i..stop];
        let lead = raw.len() - raw.trim_start().len();
        let decl = raw.trim();
        let at = i + lead;
        i = stop + 1;
        if decl.is_empty() {
            continue;
        }
        let (line, column) = lines.at(at);
        let Some(colon) = decl.find(':') else {
            diags.push(Diagnostic {
                line,
                column,
                property: None,
                message: format!("expected `name: value`, found `{decl}`"),
            });
            continue;
        };
        let name = decl[..colon].trim().to_ascii_lowercase();
        let value_offset = at + colon + 1;
        let mut value = decl[colon + 1..].trim().to_owned();
        let err = |message: String| Diagnostic {
            line,
            column,
            property: Some(name.clone()),
            message,
        };
        if value.to_ascii_lowercase().ends_with("!important") {
            diags.push(err(
                "`!important` is not supported; layout CSS has no importance levels".into(),
            ));
            continue;
        }
        if !SUPPORTED.contains(&name.as_str()) {
            diags.push(err(if name.starts_with("--") {
                "custom property declarations are not supported; declare tokens in the TokenTable"
                    .into()
            } else {
                format!(
                    "property is not in the Yolo UI layout subset (supported: {})",
                    SUPPORTED.join(", ")
                )
            }));
            continue;
        }
        match substitute_vars(&value, tokens) {
            Ok(v) => value = v,
            Err((rel, message)) => {
                let lead = src[value_offset..].len() - src[value_offset..].trim_start().len();
                let (l, c) = lines.at(value_offset + lead + rel);
                diags.push(Diagnostic {
                    line: l,
                    column: c,
                    property: Some(name.clone()),
                    message,
                });
                continue;
            }
        }
        let mut scratch = LayoutStyle::default();
        match apply(&mut scratch, &name, &value, cx) {
            Ok(()) => out.push(Declaration { name, value }),
            Err(message) => diags.push(err(message)),
        }
    }
    out
}

/// Replaces every `var(--name[, fallback])` by the token value in px.
fn substitute_vars(value: &str, tokens: &TokenTable) -> Result<String, (usize, String)> {
    let mut out = String::with_capacity(value.len());
    let mut rest = value;
    let mut base = 0;
    while let Some(pos) = rest.find("var(") {
        out.push_str(&rest[..pos]);
        let args_start = pos + 4;
        let bytes = rest.as_bytes();
        let mut depth = 1;
        let mut j = args_start;
        while j < bytes.len() && depth > 0 {
            match bytes[j] {
                b'(' => depth += 1,
                b')' => depth -= 1,
                _ => {}
            }
            j += 1;
        }
        if depth != 0 {
            return Err((base + pos, "unterminated `var(`".into()));
        }
        let args = &rest[args_start..j - 1];
        let (name, fallback) = match args.find(',') {
            Some(c) => (args[..c].trim(), Some(args[c + 1..].trim())),
            None => (args.trim(), None),
        };
        if !name.starts_with("--") {
            return Err((base + pos, format!("`var({args})` must name a `--token`")));
        }
        match (tokens.get(name), fallback) {
            (Some(px), _) => out.push_str(&format!("{px}px")),
            (None, Some(fb)) => {
                out.push_str(&substitute_vars(fb, tokens).map_err(|(r, m)| (base + pos + r, m))?)
            }
            (None, None) => {
                let known: Vec<&str> = tokens
                    .names()
                    .filter(|n| n.starts_with(crate::tokens::SPACING_PREFIX))
                    .collect();
                return Err((
                    base + pos,
                    format!(
                        "unknown token `{name}` and no fallback (known spacing tokens: {})",
                        known.join(", ")
                    ),
                ));
            }
        }
        base += j;
        rest = &rest[j..];
    }
    out.push_str(rest);
    Ok(out)
}

/// The properties of the Yolo UI layout subset.
pub const SUPPORTED: &[&str] = &[
    "display",
    "position",
    "box-sizing",
    "overflow",
    "overflow-x",
    "overflow-y",
    "inset",
    "top",
    "right",
    "bottom",
    "left",
    "width",
    "height",
    "min-width",
    "min-height",
    "max-width",
    "max-height",
    "aspect-ratio",
    "margin",
    "margin-top",
    "margin-right",
    "margin-bottom",
    "margin-left",
    "padding",
    "padding-top",
    "padding-right",
    "padding-bottom",
    "padding-left",
    "flex",
    "flex-direction",
    "flex-wrap",
    "flex-flow",
    "flex-grow",
    "flex-shrink",
    "flex-basis",
    "justify-content",
    "align-content",
    "align-items",
    "align-self",
    "justify-items",
    "justify-self",
    "place-content",
    "place-items",
    "place-self",
    "gap",
    "row-gap",
    "column-gap",
    "grid-template",
    "grid-template-areas",
    "grid-template-columns",
    "grid-template-rows",
    "grid-auto-columns",
    "grid-auto-rows",
    "grid-auto-flow",
    "grid-row",
    "grid-column",
    "grid-area",
    "grid-row-start",
    "grid-row-end",
    "grid-column-start",
    "grid-column-end",
];

type R<T> = Result<T, String>;

fn lp(v: &LengthPercentage, cx: &LengthContext) -> R<Length> {
    crate::calc::length_percentage(v, cx)
}

fn lpa(v: &LengthPercentageOrAuto, cx: &LengthContext) -> R<Length> {
    match v {
        LengthPercentageOrAuto::Auto => Ok(Length::Auto),
        LengthPercentageOrAuto::LengthPercentage(l) => lp(l, cx),
    }
}

fn no_auto(v: &LengthPercentageOrAuto, what: &str, cx: &LengthContext) -> R<Length> {
    match lpa(v, cx)? {
        Length::Auto => Err(format!("`auto` is not valid for {what}")),
        l => Ok(l),
    }
}

fn size(v: &Size, cx: &LengthContext) -> R<Length> {
    match v {
        Size::Auto => Ok(Length::Auto),
        Size::LengthPercentage(l) => lp(l, cx),
        other => Err(format!(
            "sizing keyword `{other:?}` is not supported; use px, % or auto"
        )),
    }
}

fn max_size(v: &MaxSize, cx: &LengthContext) -> R<Length> {
    match v {
        MaxSize::None => Ok(Length::Auto),
        MaxSize::LengthPercentage(l) => lp(l, cx),
        other => Err(format!(
            "sizing keyword `{other:?}` is not supported; use px, % or none"
        )),
    }
}

fn safety(o: Option<OverflowPosition>) -> Safety {
    match o {
        None => Safety::Default,
        Some(OverflowPosition::Safe) => Safety::Safe,
        Some(OverflowPosition::Unsafe) => Safety::Unsafe,
    }
}

fn self_position(v: &SelfPosition) -> ItemKeyword {
    match v {
        SelfPosition::Center => ItemKeyword::Center,
        SelfPosition::Start => ItemKeyword::Start,
        SelfPosition::End => ItemKeyword::End,
        SelfPosition::SelfStart => ItemKeyword::SelfStart,
        SelfPosition::SelfEnd => ItemKeyword::SelfEnd,
        SelfPosition::FlexStart => ItemKeyword::FlexStart,
        SelfPosition::FlexEnd => ItemKeyword::FlexEnd,
    }
}

fn baseline(b: &BaselinePosition) -> R<ItemAlign> {
    match b {
        BaselinePosition::First => Ok(ItemAlign {
            keyword: ItemKeyword::Baseline,
            safety: Safety::Default,
        }),
        BaselinePosition::Last => Err("`last baseline` is not supported".into()),
    }
}

fn kw(keyword: ItemKeyword) -> ItemAlign {
    ItemAlign {
        keyword,
        safety: Safety::Default,
    }
}

fn align_items(v: &AlignItems) -> R<ItemAlign> {
    match v {
        AlignItems::Normal => Ok(kw(ItemKeyword::Normal)),
        AlignItems::Stretch => Ok(kw(ItemKeyword::Stretch)),
        AlignItems::BaselinePosition(b) => baseline(b),
        AlignItems::SelfPosition { overflow, value } => Ok(ItemAlign {
            keyword: self_position(value),
            safety: safety(*overflow),
        }),
    }
}

fn align_self(v: &AlignSelf) -> R<Option<ItemAlign>> {
    match v {
        AlignSelf::Auto => Ok(None),
        AlignSelf::Normal => Ok(Some(kw(ItemKeyword::Normal))),
        AlignSelf::Stretch => Ok(Some(kw(ItemKeyword::Stretch))),
        AlignSelf::BaselinePosition(b) => baseline(b).map(Some),
        AlignSelf::SelfPosition { overflow, value } => Ok(Some(ItemAlign {
            keyword: self_position(value),
            safety: safety(*overflow),
        })),
    }
}

fn justify_items(v: &JustifyItems) -> R<ItemAlign> {
    match v {
        JustifyItems::Normal => Ok(kw(ItemKeyword::Normal)),
        JustifyItems::Stretch => Ok(kw(ItemKeyword::Stretch)),
        JustifyItems::BaselinePosition(b) => baseline(b),
        JustifyItems::SelfPosition { value, overflow } => Ok(ItemAlign {
            keyword: self_position(value),
            safety: safety(*overflow),
        }),
        _ => Err("`left`, `right` and `legacy` are not supported; use start, end or center".into()),
    }
}

fn justify_self(v: &JustifySelf) -> R<Option<ItemAlign>> {
    match v {
        JustifySelf::Auto => Ok(None),
        JustifySelf::Normal => Ok(Some(kw(ItemKeyword::Normal))),
        JustifySelf::Stretch => Ok(Some(kw(ItemKeyword::Stretch))),
        JustifySelf::BaselinePosition(b) => baseline(b).map(Some),
        JustifySelf::SelfPosition { value, overflow } => Ok(Some(ItemAlign {
            keyword: self_position(value),
            safety: safety(*overflow),
        })),
        _ => Err("`left` and `right` are not supported; use start or end".into()),
    }
}

fn distribution(d: &ContentDistribution) -> ContentKeyword {
    match d {
        ContentDistribution::SpaceBetween => ContentKeyword::SpaceBetween,
        ContentDistribution::SpaceAround => ContentKeyword::SpaceAround,
        ContentDistribution::SpaceEvenly => ContentKeyword::SpaceEvenly,
        ContentDistribution::Stretch => ContentKeyword::Stretch,
    }
}

fn content_position(p: &ContentPosition) -> ContentKeyword {
    match p {
        ContentPosition::Center => ContentKeyword::Center,
        ContentPosition::Start => ContentKeyword::Start,
        ContentPosition::End => ContentKeyword::End,
        ContentPosition::FlexStart => ContentKeyword::FlexStart,
        ContentPosition::FlexEnd => ContentKeyword::FlexEnd,
    }
}

fn ckw(keyword: ContentKeyword) -> ContentAlign {
    ContentAlign {
        keyword,
        safety: Safety::Default,
    }
}

fn align_content(v: &AlignContent) -> R<ContentAlign> {
    match v {
        AlignContent::Normal => Ok(ckw(ContentKeyword::Normal)),
        AlignContent::BaselinePosition(_) => {
            Err("baseline content alignment is not supported".into())
        }
        AlignContent::ContentDistribution(d) => Ok(ckw(distribution(d))),
        AlignContent::ContentPosition { overflow, value } => Ok(ContentAlign {
            keyword: content_position(value),
            safety: safety(*overflow),
        }),
    }
}

fn justify_content(v: &JustifyContent) -> R<ContentAlign> {
    match v {
        JustifyContent::Normal => Ok(ckw(ContentKeyword::Normal)),
        JustifyContent::ContentDistribution(d) => Ok(ckw(distribution(d))),
        JustifyContent::ContentPosition { value, overflow } => Ok(ContentAlign {
            keyword: content_position(value),
            safety: safety(*overflow),
        }),
        _ => Err("`left` and `right` are not supported; use start or end".into()),
    }
}

fn overflow(v: OverflowKeyword) -> R<Overflow> {
    match v {
        OverflowKeyword::Visible => Ok(Overflow::Visible),
        OverflowKeyword::Hidden => Ok(Overflow::Hidden),
        OverflowKeyword::Clip => Ok(Overflow::Clip),
        OverflowKeyword::Scroll => Ok(Overflow::Scroll),
        OverflowKeyword::Auto => {
            Err("`auto` is not supported by Taffy; use scroll, hidden or clip".into())
        }
    }
}

fn gap(v: &GapValue, cx: &LengthContext) -> R<Length> {
    match v {
        GapValue::Normal => Ok(Length::ZERO),
        GapValue::LengthPercentage(l) => lp(l, cx),
    }
}

fn breadth(b: &CssTrackBreadth, cx: &LengthContext) -> R<TrackBreadth> {
    Ok(match b {
        CssTrackBreadth::Length(l) => match lp(l, cx)? {
            Length::Px(v) => TrackBreadth::Px(v),
            Length::Percent(v) => TrackBreadth::Percent(v),
            Length::Auto => TrackBreadth::Auto,
            Length::Calc { .. } => {
                return Err(
                    "calc() mixing px and % is not supported in grid tracks; use one unit".into(),
                );
            }
        },
        CssTrackBreadth::Flex(f) => TrackBreadth::Fr(*f),
        CssTrackBreadth::MinContent => TrackBreadth::MinContent,
        CssTrackBreadth::MaxContent => TrackBreadth::MaxContent,
        CssTrackBreadth::Auto => TrackBreadth::Auto,
    })
}

fn track(t: &TrackSize, cx: &LengthContext) -> R<Track> {
    match t {
        TrackSize::TrackBreadth(b) => Ok(Track::Single(breadth(b, cx)?)),
        TrackSize::MinMax { min, max } => {
            let min = breadth(min, cx)?;
            if matches!(min, TrackBreadth::Fr(_)) {
                return Err("an `fr` value is not valid as the minimum of minmax()".into());
            }
            Ok(Track::MinMax(min, breadth(max, cx)?))
        }
        TrackSize::FitContent(l) => match lp(l, cx)? {
            Length::Px(v) => Ok(Track::FitContentPx(v)),
            Length::Percent(v) => Ok(Track::FitContentPercent(v)),
            Length::Auto => Err("fit-content() needs a length".into()),
            Length::Calc { .. } => {
                Err("calc() mixing px and % is not supported in fit-content(); use one unit".into())
            }
        },
    }
}

type Template = (Vec<TemplateItem>, Vec<Vec<String>>);

/// Tracks plus the names of the lines around them. Names are positional (one set before each
/// item and one after the last) and are dropped when no line is named. Names inside `repeat()`
/// are not supported.
fn template(t: &TrackSizing<'_>, cx: &LengthContext) -> R<Template> {
    let list = match t {
        TrackSizing::None => return Ok((Vec::new(), Vec::new())),
        TrackSizing::TrackList(list) => list,
    };
    let items = list
        .items
        .iter()
        .map(|item| match item {
            TrackListItem::TrackSize(t) => Ok(TemplateItem::Track(track(t, cx)?)),
            TrackListItem::TrackRepeat(r) => {
                if r.line_names.iter().any(|n| !n.is_empty()) {
                    return Err("named grid lines inside repeat() are not supported".into());
                }
                let count = match r.count {
                    CssRepeatCount::Number(n) => RepeatCount::Count(
                        u16::try_from(n)
                            .ok()
                            .filter(|n| *n > 0)
                            .ok_or_else(|| format!("repeat count {n} is out of range"))?,
                    ),
                    CssRepeatCount::AutoFill => RepeatCount::AutoFill,
                    CssRepeatCount::AutoFit => RepeatCount::AutoFit,
                };
                let tracks = r
                    .track_sizes
                    .iter()
                    .map(|t| track(t, cx))
                    .collect::<R<Vec<_>>>()?;
                Ok(TemplateItem::Repeat(count, tracks))
            }
        })
        .collect::<R<Vec<_>>>()?;
    let mut names: Vec<Vec<String>> = list
        .line_names
        .iter()
        .map(|set| set.iter().map(|n| String::from(&*n.0)).collect())
        .collect();
    if names.iter().all(Vec::is_empty) {
        names.clear();
    } else {
        names.resize(items.len() + 1, Vec::new());
    }
    Ok((items, names))
}

/// `grid-template-areas` rows to named rectangles. Every named area must be one filled
/// rectangle (CSS Grid §7.3); an `.` cell is unnamed.
fn areas(a: &CssGridTemplateAreas) -> R<Option<GridAreas>> {
    let (columns, cells) = match a {
        CssGridTemplateAreas::None => return Ok(None),
        CssGridTemplateAreas::Areas { columns, areas } => (*columns as usize, areas),
    };
    if columns == 0 || cells.len() % columns != 0 {
        return Err("grid-template-areas rows must all have the same number of cells".into());
    }
    let rows = cells.len() / columns;
    let (Ok(rows16), Ok(columns16)) = (u16::try_from(rows), u16::try_from(columns)) else {
        return Err("grid-template-areas is too large".into());
    };
    let mut out: Vec<GridArea> = Vec::new();
    for (i, cell) in cells.iter().enumerate() {
        let Some(name) = cell else { continue };
        let (r, c) = (i / columns + 1, i % columns + 1);
        // `r`, `c` are at most `rows` and `columns`, which fit u16.
        let (r, c) = (r as u16, c as u16);
        match out.iter_mut().find(|x| x.name == *name) {
            Some(x) => {
                x.row_start = x.row_start.min(r);
                x.row_end = x.row_end.max(r + 1);
                x.column_start = x.column_start.min(c);
                x.column_end = x.column_end.max(c + 1);
            }
            None => out.push(GridArea {
                name: name.clone(),
                row_start: r,
                row_end: r + 1,
                column_start: c,
                column_end: c + 1,
            }),
        }
    }
    for area in &out {
        for r in area.row_start..area.row_end {
            for c in area.column_start..area.column_end {
                let idx = (usize::from(r) - 1) * columns + usize::from(c) - 1;
                if cells[idx].as_deref() != Some(area.name.as_str()) {
                    return Err(format!(
                        "grid area `{}` is not a filled rectangle",
                        area.name
                    ));
                }
            }
        }
    }
    Ok(Some(GridAreas {
        rows: rows16,
        columns: columns16,
        areas: out,
    }))
}

fn grid_line(l: &CssGridLine<'_>) -> R<GridLine> {
    match l {
        CssGridLine::Auto => Ok(GridLine::Auto),
        CssGridLine::Area { name } => Ok(GridLine::Named(String::from(&*name.0), 0)),
        CssGridLine::Line {
            index,
            name: Some(name),
        } => i16::try_from(*index)
            .ok()
            .filter(|i| *i != 0)
            .map(|i| GridLine::Named(String::from(&*name.0), i))
            .ok_or_else(|| format!("grid line {index} is out of range")),
        CssGridLine::Span {
            index,
            name: Some(name),
        } => u16::try_from(*index)
            .ok()
            .filter(|i| *i > 0)
            .map(|i| GridLine::NamedSpan(String::from(&*name.0), i))
            .ok_or_else(|| format!("span {index} is out of range")),
        CssGridLine::Line { index, name: None } => i16::try_from(*index)
            .ok()
            .filter(|i| *i != 0)
            .map(GridLine::Line)
            .ok_or_else(|| format!("grid line {index} is out of range")),
        CssGridLine::Span { index, name: None } => u16::try_from(*index)
            .ok()
            .filter(|i| *i > 0)
            .map(GridLine::Span)
            .ok_or_else(|| format!("span {index} is out of range")),
    }
}

fn sides(top: Length, right: Length, bottom: Length, left: Length) -> Sides<Length> {
    Sides {
        top,
        right,
        bottom,
        left,
    }
}

/// Applies one declaration (value already free of `var()`) to `style`.
pub(crate) fn apply(style: &mut LayoutStyle, name: &str, value: &str, cx: &LengthContext) -> R<()> {
    let prop = Property::parse_string(PropertyId::from(name), value, ParserOptions::default())
        .map_err(|e| {
            format!(
                "invalid value `{value}`: {}",
                CssError::from(e, String::new()).kind
            )
        })?;
    let s = style;
    match &prop {
        Property::Display(d) => {
            s.display = match d {
                CssDisplay::Keyword(DisplayKeyword::None) => Display::None,
                CssDisplay::Pair(p)
                    if !p.is_list_item && matches!(p.outside, DisplayOutside::Block) =>
                {
                    match p.inside {
                        DisplayInside::Flow => Display::Block,
                        DisplayInside::Flex(_) => Display::Flex,
                        DisplayInside::Grid => Display::Grid,
                        _ => {
                            return Err(format!(
                                "`display: {value}` is not supported; use block, flex, grid or none"
                            ));
                        }
                    }
                }
                _ => {
                    return Err(format!(
                        "`display: {value}` is not supported; use block, flex, grid or none"
                    ));
                }
            }
        }
        Property::Position(p) => {
            s.position = match p {
                CssPosition::Static => Position::Static,
                CssPosition::Relative => Position::Relative,
                CssPosition::Absolute => Position::Absolute,
                CssPosition::Fixed => Position::Fixed,
                CssPosition::Sticky(_) => return Err("`sticky` is not supported".into()),
            }
        }
        Property::BoxSizing(b, _) => {
            s.box_sizing = match b {
                CssBoxSizing::BorderBox => BoxSizing::BorderBox,
                CssBoxSizing::ContentBox => BoxSizing::ContentBox,
            }
        }
        Property::Overflow(o) => {
            s.overflow_x = overflow(o.x)?;
            s.overflow_y = overflow(o.y)?;
        }
        Property::OverflowX(o) => s.overflow_x = overflow(*o)?,
        Property::OverflowY(o) => s.overflow_y = overflow(*o)?,
        Property::Inset(i) => {
            s.inset = sides(
                lpa(&i.top, cx)?,
                lpa(&i.right, cx)?,
                lpa(&i.bottom, cx)?,
                lpa(&i.left, cx)?,
            )
        }
        Property::Top(v) => s.inset.top = lpa(v, cx)?,
        Property::Right(v) => s.inset.right = lpa(v, cx)?,
        Property::Bottom(v) => s.inset.bottom = lpa(v, cx)?,
        Property::Left(v) => s.inset.left = lpa(v, cx)?,
        Property::Width(v) => s.width = size(v, cx)?,
        Property::Height(v) => s.height = size(v, cx)?,
        Property::MinWidth(v) => s.min_width = size(v, cx)?,
        Property::MinHeight(v) => s.min_height = size(v, cx)?,
        Property::MaxWidth(v) => s.max_width = max_size(v, cx)?,
        Property::MaxHeight(v) => s.max_height = max_size(v, cx)?,
        Property::AspectRatio(a) => {
            s.aspect_ratio = match &a.ratio {
                Some(r) if r.1 == 0.0 || r.0 <= 0.0 => {
                    return Err("aspect ratio must be positive".into());
                }
                Some(r) => Some(r.0 / r.1),
                None => None,
            }
        }
        Property::Margin(m) => {
            s.margin = sides(
                lpa(&m.top, cx)?,
                lpa(&m.right, cx)?,
                lpa(&m.bottom, cx)?,
                lpa(&m.left, cx)?,
            )
        }
        Property::MarginTop(v) => s.margin.top = lpa(v, cx)?,
        Property::MarginRight(v) => s.margin.right = lpa(v, cx)?,
        Property::MarginBottom(v) => s.margin.bottom = lpa(v, cx)?,
        Property::MarginLeft(v) => s.margin.left = lpa(v, cx)?,
        Property::Padding(p) => {
            s.padding = sides(
                no_auto(&p.top, "padding", cx)?,
                no_auto(&p.right, "padding", cx)?,
                no_auto(&p.bottom, "padding", cx)?,
                no_auto(&p.left, "padding", cx)?,
            )
        }
        Property::PaddingTop(v) => s.padding.top = no_auto(v, "padding", cx)?,
        Property::PaddingRight(v) => s.padding.right = no_auto(v, "padding", cx)?,
        Property::PaddingBottom(v) => s.padding.bottom = no_auto(v, "padding", cx)?,
        Property::PaddingLeft(v) => s.padding.left = no_auto(v, "padding", cx)?,
        Property::FlexDirection(d, _) => s.flex_direction = flex_direction(d),
        Property::FlexWrap(w, _) => s.flex_wrap = flex_wrap(w),
        Property::FlexFlow(f, _) => {
            s.flex_direction = flex_direction(&f.direction);
            s.flex_wrap = flex_wrap(&f.wrap);
        }
        Property::Flex(f, _) => {
            s.flex_grow = f.grow;
            s.flex_shrink = f.shrink;
            s.flex_basis = lpa(&f.basis, cx)?;
        }
        Property::FlexGrow(v, _) => s.flex_grow = *v,
        Property::FlexShrink(v, _) => s.flex_shrink = *v,
        Property::FlexBasis(v, _) => s.flex_basis = lpa(v, cx)?,
        Property::JustifyContent(v, _) => s.justify_content = justify_content(v)?,
        Property::AlignContent(v, _) => s.align_content = align_content(v)?,
        Property::AlignItems(v, _) => s.align_items = align_items(v)?,
        Property::AlignSelf(v, _) => s.align_self = align_self(v)?,
        Property::JustifyItems(v) => s.justify_items = justify_items(v)?,
        Property::JustifySelf(v) => s.justify_self = justify_self(v)?,
        Property::PlaceContent(p) => {
            s.align_content = align_content(&p.align)?;
            s.justify_content = justify_content(&p.justify)?;
        }
        Property::PlaceItems(p) => {
            s.align_items = align_items(&p.align)?;
            s.justify_items = justify_items(&p.justify)?;
        }
        Property::PlaceSelf(p) => {
            s.align_self = align_self(&p.align)?;
            s.justify_self = justify_self(&p.justify)?;
        }
        Property::Gap(g) => {
            s.row_gap = gap(&g.row, cx)?;
            s.column_gap = gap(&g.column, cx)?;
        }
        Property::RowGap(g) => s.row_gap = gap(g, cx)?,
        Property::ColumnGap(g) => s.column_gap = gap(g, cx)?,
        Property::GridTemplateColumns(t) => {
            (s.grid_template_columns, s.grid_template_column_names) = template(t, cx)?;
        }
        Property::GridTemplateAreas(a) => s.grid_template_areas = areas(a)?,
        Property::GridTemplate(t) => {
            (s.grid_template_rows, s.grid_template_row_names) = template(&t.rows, cx)?;
            (s.grid_template_columns, s.grid_template_column_names) = template(&t.columns, cx)?;
            s.grid_template_areas = areas(&t.areas)?;
        }
        Property::GridTemplateRows(t) => {
            (s.grid_template_rows, s.grid_template_row_names) = template(t, cx)?;
        }
        Property::GridAutoColumns(l) => {
            s.grid_auto_columns = l.0.iter().map(|t| track(t, cx)).collect::<R<_>>()?
        }
        Property::GridAutoRows(l) => {
            s.grid_auto_rows = l.0.iter().map(|t| track(t, cx)).collect::<R<_>>()?
        }
        Property::GridAutoFlow(f) => {
            s.grid_auto_flow = match (
                f.contains(CssGridAutoFlow::Column),
                f.contains(CssGridAutoFlow::Dense),
            ) {
                (false, false) => GridAutoFlow::Row,
                (true, false) => GridAutoFlow::Column,
                (false, true) => GridAutoFlow::RowDense,
                (true, true) => GridAutoFlow::ColumnDense,
            }
        }
        Property::GridRow(r) => {
            s.grid_row = GridPlacement {
                start: grid_line(&r.start)?,
                end: grid_line(&r.end)?,
            }
        }
        Property::GridColumn(c) => {
            s.grid_column = GridPlacement {
                start: grid_line(&c.start)?,
                end: grid_line(&c.end)?,
            }
        }
        Property::GridArea(a) => {
            s.grid_row = GridPlacement {
                start: grid_line(&a.row_start)?,
                end: grid_line(&a.row_end)?,
            };
            s.grid_column = GridPlacement {
                start: grid_line(&a.column_start)?,
                end: grid_line(&a.column_end)?,
            };
        }
        Property::GridRowStart(l) => s.grid_row.start = grid_line(l)?,
        Property::GridRowEnd(l) => s.grid_row.end = grid_line(l)?,
        Property::GridColumnStart(l) => s.grid_column.start = grid_line(l)?,
        Property::GridColumnEnd(l) => s.grid_column.end = grid_line(l)?,
        Property::Unparsed(_) => {
            return Err(format!("invalid or unsupported value `{value}`"));
        }
        _ => return Err("property is not in the Yolo UI layout subset".into()),
    }
    Ok(())
}

fn flex_direction(d: &CssFlexDirection) -> FlexDirection {
    match d {
        CssFlexDirection::Row => FlexDirection::Row,
        CssFlexDirection::RowReverse => FlexDirection::RowReverse,
        CssFlexDirection::Column => FlexDirection::Column,
        CssFlexDirection::ColumnReverse => FlexDirection::ColumnReverse,
    }
}

fn flex_wrap(w: &CssFlexWrap) -> FlexWrap {
    match w {
        CssFlexWrap::NoWrap => FlexWrap::NoWrap,
        CssFlexWrap::Wrap => FlexWrap::Wrap,
        CssFlexWrap::WrapReverse => FlexWrap::WrapReverse,
    }
}
