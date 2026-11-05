// SPDX-License-Identifier: Apache-2.0
//! Taffy layout of the templates (through `yolo-layout`) at desktop and phone widths.

use yolo_templates::content::SITES;
use yolo_templates::layout::{app_chrome, docs_shell, editorial_grid, sheet_for};

const DESKTOP: (f32, f32) = (1440.0, 900.0);
const PHONE: (f32, f32) = (390.0, 844.0);

#[test]
fn layout_css_stays_inside_the_yolo_layout_subset() {
    sheet_for(DESKTOP.0).expect("desktop layout CSS parses");
    sheet_for(PHONE.0).expect("compact layout CSS parses");
}

#[test]
fn search_is_centred_in_the_app_bar_on_desktop() {
    for site in &SITES {
        let r = app_chrome(&site.app, DESKTOP.0, DESKTOP.1).unwrap();
        let bar_centre = r.bar.x + r.bar.width / 2.0;
        let search_centre = r.search.x + r.search.width / 2.0;
        assert!((bar_centre - search_centre).abs() <= 1.0, "{}: search off centre by {}", site.id, bar_centre - search_centre);
        assert!(r.search.width <= 640.0 + 0.5 && r.search.width >= 480.0, "{}: search width {}", site.id, r.search.width);
        assert!(r.voice.width >= 44.0 && r.voice.height >= 44.0);
    }
}

#[test]
fn rail_sits_left_and_keeps_touch_targets() {
    let r = app_chrome(&SITES[0].app, DESKTOP.0, DESKTOP.1).unwrap();
    assert_eq!(r.rail.x, 0.0);
    assert!((r.rail.width - 72.0).abs() < 0.5);
    assert!(r.main.x >= 72.0);
    for item in &r.rail_items {
        assert!(item.height >= 64.0 && item.width >= 44.0, "rail item {item:?}");
    }
    // Items stack vertically.
    for pair in r.rail_items.windows(2) {
        assert!(pair[1].y > pair[0].y);
    }
}

#[test]
fn media_grid_fills_columns_on_desktop_and_one_column_on_phones() {
    let d = app_chrome(&SITES[1].app, DESKTOP.0, DESKTOP.1).unwrap();
    let columns = d.tiles.iter().filter(|t| (t.y - d.tiles[0].y).abs() < 0.5).count();
    // 1440 - 72 rail - 48 padding = 1320 px: four 300 px minimum tracks with 16 px gaps.
    assert_eq!(columns, yolo_templates::app::auto_fill_columns(1320.0, 300.0, 16.0));
    for t in &d.tiles {
        assert!(t.width >= 300.0 - 0.5);
    }
    let p = app_chrome(&SITES[1].app, PHONE.0, PHONE.1).unwrap();
    for pair in p.tiles.windows(2) {
        assert!(pair[1].y > pair[0].y, "phone tiles stack");
    }
    assert!(p.tiles.iter().all(|t| t.width <= PHONE.0));
}

#[test]
fn phone_layout_turns_the_rail_into_a_bottom_bar() {
    let r = app_chrome(&SITES[2].app, PHONE.0, PHONE.1).unwrap();
    assert!(r.rail.y > r.main.y, "rail below content");
    assert!((r.rail.width - PHONE.0).abs() < 1.0);
    for pair in r.rail_items.windows(2) {
        assert!((pair[1].y - pair[0].y).abs() < 0.5 && pair[1].x > pair[0].x, "items side by side");
    }
    for item in &r.rail_items {
        assert!(item.width >= 64.0 && item.height >= 44.0);
    }
    // The search takes the whole bar.
    assert!(r.search.width >= PHONE.0 - 2.0 * 8.0 - 1.0);
}

#[test]
fn docs_shell_has_a_tree_column_on_desktop_and_stacks_on_phones() {
    let d = docs_shell(DESKTOP.0).unwrap();
    assert!((d.tree.width - 280.0).abs() < 0.5);
    assert!((d.main.x - 280.0).abs() < 0.5);
    assert!((d.top.width - DESKTOP.0).abs() < 0.5);
    let p = docs_shell(PHONE.0).unwrap();
    assert!(p.main.y > p.tree.y && (p.main.x).abs() < 0.5);
}

#[test]
fn editorial_lead_spans_two_columns_on_desktop() {
    let cards = editorial_grid(1152.0, 4).unwrap();
    assert!(cards[0].width > cards[1].width * 1.9, "lead {:?} vs {:?}", cards[0], cards[1]);
    assert!(cards[0].height > cards[1].height);
    let phone = editorial_grid(358.0, 4).unwrap();
    assert!(phone.iter().all(|c| (c.width - 358.0).abs() < 0.5));
}
