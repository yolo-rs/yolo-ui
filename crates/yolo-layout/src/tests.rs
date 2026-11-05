// SPDX-License-Identifier: Apache-2.0
//! Unit tests for relative units, `calc()`, the container-resolving Taffy conversion, grid areas
//! and named lines.

use crate::*;

fn cx() -> LengthContext {
    LengthContext::default().with_viewport(800.0, 600.0)
}

fn parse(css: &str) -> LayoutStyle {
    parse_style_in(css, &TokenTable::empty(), &cx()).unwrap_or_else(|e| panic!("{css}: {e}"))
}

fn parse_err(css: &str) -> String {
    parse_style_in(css, &TokenTable::empty(), &cx())
        .expect_err(css)
        .to_string()
}

// ---- Length helpers ------------------------------------------------------------------------

#[test]
fn from_linear_collapses() {
    assert_eq!(Length::from_linear(10.0, 0.0), Length::Px(10.0));
    assert_eq!(Length::from_linear(0.0, 0.5), Length::Percent(0.5));
    assert_eq!(Length::from_linear(0.0, 0.0), Length::Px(0.0));
    assert_eq!(
        Length::from_linear(-20.0, 1.0),
        Length::Calc {
            px: -20.0,
            percent: 1.0
        }
    );
}

#[test]
fn resolve_against_basis() {
    assert_eq!(Length::Auto.resolve(Some(100.0)), None);
    assert_eq!(Length::Px(8.0).resolve(None), Some(8.0));
    assert_eq!(Length::Percent(0.25).resolve(Some(200.0)), Some(50.0));
    assert_eq!(Length::Percent(0.25).resolve(None), None);
    assert_eq!(
        Length::Calc {
            px: -20.0,
            percent: 1.0
        }
        .resolve(Some(200.0)),
        Some(180.0)
    );
    assert_eq!(
        Length::Calc {
            px: -20.0,
            percent: 1.0
        }
        .resolve(None),
        None
    );
}

// ---- rem, em, vw, vh -----------------------------------------------------------------------

#[test]
fn rem_uses_root_font_size() {
    assert_eq!(parse("width: 2rem").width, Length::Px(32.0));
    let s = parse_style_in(
        "width: 2rem",
        &TokenTable::empty(),
        &cx().with_root_font_size(10.0),
    )
    .unwrap();
    assert_eq!(s.width, Length::Px(20.0));
}

#[test]
fn em_uses_parent_font_size_not_root() {
    let c = cx().with_root_font_size(10.0).with_font_size(20.0);
    let s = parse_style_in(
        "padding-left: 1.5em; margin-top: 1rem",
        &TokenTable::empty(),
        &c,
    )
    .unwrap();
    assert_eq!(s.padding.left, Length::Px(30.0));
    assert_eq!(s.margin.top, Length::Px(10.0));
}

#[test]
fn viewport_units_resolve_against_viewport() {
    let s = parse("width: 50vw; height: 25vh; min-width: 10vmin; max-width: 10vmax");
    assert_eq!(s.width, Length::Px(400.0));
    assert_eq!(s.height, Length::Px(150.0));
    assert_eq!(s.min_width, Length::Px(60.0));
    assert_eq!(s.max_width, Length::Px(80.0));
}

#[test]
fn viewport_unit_without_viewport_fails_closed() {
    let e = parse_style("width: 50vw", &TokenTable::empty())
        .unwrap_err()
        .to_string();
    assert!(e.contains("viewport"), "{e}");
}

#[test]
fn negative_and_zero_relative_values() {
    let s = parse("margin-left: -2rem; margin-top: 0rem; margin-right: -10vw");
    assert_eq!(s.margin.left, Length::Px(-32.0));
    assert_eq!(s.margin.top, Length::Px(0.0));
    assert_eq!(s.margin.right, Length::Px(-80.0));
}

#[test]
fn unsupported_unit_is_still_rejected() {
    let e = parse_err("width: 3ch");
    assert!(e.contains("not supported"), "{e}");
}

#[test]
fn absolute_units_still_work() {
    assert_eq!(parse("width: 1in").width, Length::Px(96.0));
}

#[test]
fn stylesheet_style_in_rebinds_context() {
    let sheet = parse_stylesheet_in(
        ".a { width: 2em; height: 10vh }",
        &TokenTable::empty(),
        &cx().with_font_size(10.0),
    )
    .unwrap();
    let s = sheet.style(&[".a"]);
    assert_eq!((s.width, s.height), (Length::Px(20.0), Length::Px(60.0)));
    let s = sheet.style_in(
        &[".a"],
        &cx().with_font_size(30.0).with_viewport(100.0, 1000.0),
    );
    assert_eq!((s.width, s.height), (Length::Px(60.0), Length::Px(100.0)));
}

// ---- calc() --------------------------------------------------------------------------------

#[test]
fn calc_mixed_px_and_percent() {
    assert_eq!(
        parse("width: calc(100% - 20px)").width,
        Length::Calc {
            px: -20.0,
            percent: 1.0
        }
    );
    assert_eq!(
        parse("width: calc(20px + 50%)").width,
        Length::Calc {
            px: 20.0,
            percent: 0.5
        }
    );
}

#[test]
fn calc_pure_values_collapse() {
    assert_eq!(parse("width: calc(10px + 5px)").width, Length::Px(15.0));
    assert_eq!(parse("width: calc(50% + 25%)").width, Length::Percent(0.75));
    assert_eq!(parse("width: calc(1rem + 1rem)").width, Length::Px(32.0));
    assert_eq!(parse("width: calc(100% - 100%)").width, Length::Px(0.0));
}

#[test]
fn calc_multiplication_and_division() {
    assert_eq!(parse("width: calc(100% / 4)").width, Length::Percent(0.25));
    assert_eq!(parse("width: calc(3 * 10px)").width, Length::Px(30.0));
    assert_eq!(parse("width: calc(10px * 3)").width, Length::Px(30.0));
    assert_eq!(
        parse("width: calc(2 * (10px + 1rem))").width,
        Length::Px(52.0)
    );
    assert_eq!(
        parse("width: calc((100% - 30px) / 3)").width,
        Length::Calc {
            px: -10.0,
            percent: 1.0 / 3.0
        }
    );
}

#[test]
fn calc_nested() {
    assert_eq!(
        parse("width: calc(100% - calc(2rem + 4px))").width,
        Length::Calc {
            px: -36.0,
            percent: 1.0
        }
    );
    assert_eq!(
        parse("width: calc(calc(calc(1px + 1px) * 2) + 1px)").width,
        Length::Px(5.0)
    );
}

#[test]
fn calc_with_relative_units() {
    assert_eq!(parse("width: calc(100vw - 2rem)").width, Length::Px(768.0));
    assert_eq!(
        parse("height: calc(100% - 10vh)").height,
        Length::Calc {
            px: -60.0,
            percent: 1.0
        }
    );
}

#[test]
fn calc_negative_results() {
    assert_eq!(
        parse("margin-left: calc(10px - 20px)").margin.left,
        Length::Px(-10.0)
    );
    assert_eq!(
        parse("margin-left: calc(-1 * 10px)").margin.left,
        Length::Px(-10.0)
    );
    assert_eq!(
        parse("margin-left: calc(-50% + 4px)").margin.left,
        Length::Calc {
            px: 4.0,
            percent: -0.5
        }
    );
}

#[test]
fn calc_multiply_by_zero() {
    assert_eq!(parse("width: calc(0 * 100%)").width, Length::Px(0.0));
}

#[test]
fn calc_division_by_zero_is_an_error() {
    parse_err("width: calc(10px / 0)");
    parse_err("width: calc(100% / 0)");
    parse_err("width: calc((1px + 1%) / (1 - 1))");
}

#[test]
fn calc_type_errors() {
    parse_err("width: calc(10px + 2)");
    parse_err("width: calc(2)");
    parse_err("width: calc(10px * 10px)");
}

#[test]
fn calc_non_linear_functions_are_rejected() {
    parse_err("width: min(10px, 50%)");
    parse_err("width: clamp(10px, 50%, 90%)");
}

#[test]
fn calc_in_each_length_property() {
    let s = parse(
        "padding: calc(1rem + 2px); margin: calc(-1 * 4px) auto; top: calc(50% - 8px); gap: calc(2 * 4px) calc(100% / 10); flex-basis: calc(100% / 3)",
    );
    assert_eq!(s.padding, Sides::all(Length::Px(18.0)));
    assert_eq!(s.margin.top, Length::Px(-4.0));
    assert_eq!(s.margin.right, Length::Auto);
    assert_eq!(
        s.inset.top,
        Length::Calc {
            px: -8.0,
            percent: 0.5
        }
    );
    assert_eq!(s.row_gap, Length::Px(8.0));
    assert_eq!(s.column_gap, Length::Percent(0.1));
    assert_eq!(s.flex_basis, Length::Percent(1.0 / 3.0));
}

#[test]
fn calc_mixed_in_grid_tracks_is_an_error() {
    let e = parse_err("grid-template-columns: calc(100% - 10px) 1fr");
    assert!(e.contains("grid"), "{e}");
    // A pure calc is fine.
    let s = parse("grid-template-columns: calc(10px * 2) 1fr");
    assert_eq!(s.grid_template_columns.len(), 2);
}

// ---- Taffy conversion ----------------------------------------------------------------------

#[test]
fn to_taffy_keeps_px_term_without_container() {
    let s = parse("width: calc(100% - 20px)");
    assert_eq!(s.to_taffy().size.width, taffy::Dimension::length(-20.0));
}

#[test]
fn to_taffy_in_resolves_against_the_matching_axis() {
    let s = parse(
        "width: calc(100% - 20px); height: calc(50% + 10px); padding-top: calc(10% + 2px); margin-left: calc(10% - 1px); row-gap: calc(10% + 1px); column-gap: calc(10% + 1px); top: calc(100% - 5px); left: calc(100% - 5px)",
    );
    let t = s.to_taffy_in((200.0, 100.0));
    assert_eq!(t.size.width, taffy::Dimension::length(180.0));
    assert_eq!(t.size.height, taffy::Dimension::length(60.0));
    // Padding and margin percentages use the container width on every side.
    assert_eq!(t.padding.top, taffy::LengthPercentage::length(22.0));
    assert_eq!(t.margin.left, taffy::LengthPercentageAuto::length(19.0));
    assert_eq!(t.gap.width, taffy::LengthPercentage::length(21.0));
    assert_eq!(t.gap.height, taffy::LengthPercentage::length(11.0));
    assert_eq!(t.inset.top, taffy::LengthPercentageAuto::length(95.0));
    assert_eq!(t.inset.left, taffy::LengthPercentageAuto::length(195.0));
}

#[test]
fn to_taffy_in_leaves_plain_lengths_alone() {
    let s = parse("width: 50%; height: 30px; margin-left: auto");
    let (a, b) = (s.to_taffy(), s.to_taffy_in((200.0, 100.0)));
    assert_eq!(a.size, b.size);
    assert_eq!(a.margin, b.margin);
    assert_eq!(b.size.width, taffy::Dimension::percent(0.5));
}

#[test]
fn to_taffy_in_lays_out_a_calc_width() {
    let mut root = parse("width: 200px; height: 100px");
    root.display = Display::Block;
    let child = parse("width: calc(100% - 20px); height: calc(50% + 10px)");
    let mut tree = taffy::TaffyTree::<()>::new();
    let c = tree.new_leaf(child.to_taffy_in((200.0, 100.0))).unwrap();
    let r = tree.new_with_children(root.to_taffy(), &[c]).unwrap();
    tree.compute_layout(
        r,
        taffy::Size {
            width: taffy::AvailableSpace::MaxContent,
            height: taffy::AvailableSpace::MaxContent,
        },
    )
    .unwrap();
    let l = tree.layout(c).unwrap();
    assert_eq!((l.size.width, l.size.height), (180.0, 60.0));
}

// ---- grid-template-areas, named lines ------------------------------------------------------

const PAGE: &str = "display: grid; width: 300px; height: 200px; \
    grid-template-columns: 100px 1fr; grid-template-rows: 40px 1fr; \
    grid-template-areas: \"header header\" \"sidebar main\"";

#[test]
fn parses_grid_template_areas() {
    let s = parse(PAGE);
    let a = s.grid_template_areas.expect("areas");
    assert_eq!((a.rows, a.columns), (2, 2));
    let get = |n: &str| a.areas.iter().find(|x| x.name == n).cloned().unwrap();
    assert_eq!(
        get("header"),
        GridArea {
            name: "header".into(),
            row_start: 1,
            row_end: 2,
            column_start: 1,
            column_end: 3
        }
    );
    assert_eq!(
        get("main"),
        GridArea {
            name: "main".into(),
            row_start: 2,
            row_end: 3,
            column_start: 2,
            column_end: 3
        }
    );
    assert_eq!(a.areas.len(), 3);
}

#[test]
fn grid_template_areas_none_and_dots() {
    assert_eq!(parse("grid-template-areas: none").grid_template_areas, None);
    let a = parse("grid-template-areas: \"a .\" \". b\"")
        .grid_template_areas
        .unwrap();
    assert_eq!((a.rows, a.columns, a.areas.len()), (2, 2, 2));
}

#[test]
fn grid_template_areas_must_be_rectangles() {
    let e = parse_err("grid-template-areas: \"a b\" \"b a\"");
    assert!(e.contains("rectangle"), "{e}");
    parse_err("grid-template-areas: \"a .\" \". a\"");
    // Ragged rows are invalid CSS.
    parse_err("grid-template-areas: \"a b\" \"c\"");
}

#[test]
fn grid_area_places_items_through_taffy() {
    let page = parse(PAGE);
    let place = |area: &str| parse(&format!("grid-area: {area}"));
    let mut tree = LayoutTree::new();
    let header = tree.node("header", &place("header"), &[]).unwrap();
    let sidebar = tree.node("sidebar", &place("sidebar"), &[]).unwrap();
    let main = tree.node("main", &place("main"), &[]).unwrap();
    let root = tree.node("page", &page, &[header, sidebar, main]).unwrap();
    tree.compute(root, Some(300.0), Some(200.0), monospace_measure(8.0, 16.0))
        .unwrap();
    let r = |k| tree.local_rect(k).unwrap();
    assert_eq!(
        r(header),
        LayoutRect {
            x: 0.0,
            y: 0.0,
            width: 300.0,
            height: 40.0
        }
    );
    assert_eq!(
        r(sidebar),
        LayoutRect {
            x: 0.0,
            y: 40.0,
            width: 100.0,
            height: 160.0
        }
    );
    assert_eq!(
        r(main),
        LayoutRect {
            x: 100.0,
            y: 40.0,
            width: 200.0,
            height: 160.0
        }
    );
}

#[test]
fn grid_template_shorthand_sets_areas_and_tracks() {
    let s = parse("grid-template: \"a a\" 40px \"b c\" 1fr / 1fr 2fr");
    assert_eq!(s.grid_template_rows.len(), 2);
    assert_eq!(s.grid_template_columns.len(), 2);
    assert_eq!(s.grid_template_areas.unwrap().areas.len(), 3);
}

#[test]
fn parses_named_grid_lines() {
    let s = parse(
        "grid-template-columns: [full-start] 100px [content-start] 1fr [content-end full-end]",
    );
    assert_eq!(
        s.grid_template_column_names,
        vec![
            vec!["full-start".to_string()],
            vec!["content-start".to_string()],
            vec!["content-end".to_string(), "full-end".to_string()],
        ]
    );
    assert!(
        parse("grid-template-columns: 100px 1fr")
            .grid_template_column_names
            .is_empty()
    );
}

#[test]
fn parses_named_line_placements() {
    let s = parse("grid-column: content-start / content-end; grid-row: 2 band / span 3 band");
    assert_eq!(
        s.grid_column,
        GridPlacement {
            start: GridLine::Named("content-start".into(), 0),
            end: GridLine::Named("content-end".into(), 0)
        }
    );
    assert_eq!(
        s.grid_row,
        GridPlacement {
            start: GridLine::Named("band".into(), 2),
            end: GridLine::NamedSpan("band".into(), 3)
        }
    );
}

#[test]
fn named_lines_inside_repeat_are_rejected() {
    let e = parse_err("grid-template-columns: repeat(2, [a] 10px)");
    assert!(e.contains("repeat"), "{e}");
}

#[test]
fn named_lines_place_items_through_taffy() {
    let page = parse(
        "display: grid; width: 300px; height: 50px; \
         grid-template-columns: [full-start] 100px [content-start] 1fr [content-end full-end]; \
         grid-template-rows: 50px",
    );
    let item = parse("grid-column: content-start / content-end");
    let mut tree = LayoutTree::new();
    let k = tree.node("item", &item, &[]).unwrap();
    let root = tree.node("page", &page, &[k]).unwrap();
    tree.compute(root, Some(300.0), Some(50.0), monospace_measure(8.0, 16.0))
        .unwrap();
    assert_eq!(
        tree.local_rect(k).unwrap(),
        LayoutRect {
            x: 100.0,
            y: 0.0,
            width: 200.0,
            height: 50.0
        }
    );
}

#[test]
fn numeric_lines_are_unchanged() {
    let s = parse("grid-column: 1 / span 2; grid-row: -1");
    assert_eq!(
        s.grid_column,
        GridPlacement {
            start: GridLine::Line(1),
            end: GridLine::Span(2)
        }
    );
    assert_eq!(s.grid_row.start, GridLine::Line(-1));
}
