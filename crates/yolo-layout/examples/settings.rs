// SPDX-License-Identifier: Apache-2.0
//! Lays out a Material 3 settings page from CSS and prints the computed rects as JSON.
//!
//! `cargo run --example settings [width]`

use yolo_layout::{LayoutTree, NodeKey, TokenTable, monospace_measure, parse_stylesheet};

const CSS: &str = r"
.page {
  display: grid;
  grid-template-columns: minmax(200px, 280px) 1fr;
  grid-template-rows: 64px 1fr;
  width: 100%;
  height: 100%;
}
.top-bar {
  grid-column: 1 / span 2;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 var(--md-sys-spacing-large);
}
.nav {
  display: flex;
  flex-direction: column;
  gap: var(--md-sys-spacing-extra-small);
  padding: var(--md-sys-spacing-medium);
  overflow-y: scroll;
}
.nav-item { height: 56px; padding: 0 var(--md-sys-spacing-large); display: flex; align-items: center; }
.content {
  display: flex;
  flex-direction: column;
  gap: var(--md-sys-spacing-extra-large);
  padding: var(--md-sys-spacing-extra-large);
  max-width: 840px;
}
.section { display: flex; flex-direction: column; gap: var(--md-sys-spacing-small); }
.cards {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
  gap: var(--md-sys-spacing-large);
}
.card { aspect-ratio: 16 / 9; padding: var(--md-sys-spacing-large); }
.row {
  display: flex;
  align-items: center;
  gap: var(--md-sys-spacing-large);
  min-height: 56px;
}
.row-text { flex: 1 1 0px; display: flex; flex-direction: column; }
.switch { width: 52px; height: 32px; flex-shrink: 0; }
.fab { position: absolute; right: var(--md-sys-spacing-large); bottom: var(--md-sys-spacing-large); width: 56px; height: 56px; }
";

fn main() {
    let width = std::env::args()
        .nth(1)
        .and_then(|w| w.parse().ok())
        .unwrap_or(1024.0f32);
    let tokens = TokenTable::material();
    let sheet = match parse_stylesheet(CSS, &tokens) {
        Ok(sheet) => sheet,
        Err(diags) => {
            eprintln!("{diags}");
            std::process::exit(1);
        }
    };

    // A rejected declaration shows the fail-closed diagnostics.
    if let Err(diags) = parse_stylesheet(".x { float: left; overflow: auto; width: 2em }", &tokens)
    {
        eprintln!("expected diagnostics:\n{diags}\n");
    }

    let mut tree = LayoutTree::new();
    let style = |s: &str| sheet.style(&[s]);
    let text =
        |tree: &mut LayoutTree, name: &str, t: &str| tree.text(name, &style(".text"), t).unwrap();

    let title = text(&mut tree, "title", "Settings");
    let search = text(&mut tree, "search", "Search settings");
    let top_bar = tree
        .node("top-bar", &style(".top-bar"), &[title, search])
        .unwrap();

    let nav_items: Vec<NodeKey> = ["Network", "Display", "Sound", "Privacy", "About"]
        .iter()
        .map(|label| {
            let t = text(&mut tree, &format!("nav-{label}-label"), label);
            tree.node(&format!("nav-{label}"), &style(".nav-item"), &[t])
                .unwrap()
        })
        .collect();
    let nav = tree.node("nav", &style(".nav"), &nav_items).unwrap();

    let mut rows = Vec::new();
    for (i, (headline, supporting)) in [
        ("Wi-Fi", "Connected to yolo-5G"),
        ("Bluetooth", "Off"),
        (
            "Airplane mode",
            "Turns off every wireless connection on this device",
        ),
    ]
    .iter()
    .enumerate()
    {
        let h = text(&mut tree, &format!("row-{i}-headline"), headline);
        let s = text(&mut tree, &format!("row-{i}-supporting"), supporting);
        let body = tree
            .node(&format!("row-{i}-text"), &style(".row-text"), &[h, s])
            .unwrap();
        let switch = tree
            .node(&format!("row-{i}-switch"), &style(".switch"), &[])
            .unwrap();
        rows.push(
            tree.node(&format!("row-{i}"), &style(".row"), &[body, switch])
                .unwrap(),
        );
    }
    let heading = text(&mut tree, "section-heading", "Network & internet");
    rows.insert(0, heading);
    let section = tree.node("section", &style(".section"), &rows).unwrap();

    let cards: Vec<NodeKey> = (0..4)
        .map(|i| {
            tree.node(&format!("card-{i}"), &style(".card"), &[])
                .unwrap()
        })
        .collect();
    let card_grid = tree.node("cards", &style(".cards"), &cards).unwrap();
    let content = tree
        .node("content", &style(".content"), &[section, card_grid])
        .unwrap();
    let fab = tree.node("fab", &style(".fab"), &[]).unwrap();

    let page = tree
        .node("page", &style(".page"), &[top_bar, nav, content, fab])
        .unwrap();
    let mut page_style = style(".page");
    page_style.position = yolo_layout::Position::Relative;
    tree.set_style(page, &page_style).unwrap();

    tree.compute(page, Some(width), Some(720.0), monospace_measure(8.0, 20.0))
        .unwrap();
    println!("{}", tree.to_json(page).unwrap());
}
