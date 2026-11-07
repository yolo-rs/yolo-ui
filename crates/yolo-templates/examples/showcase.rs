// SPDX-License-Identifier: Apache-2.0
//! Renders every template for the eight presets in light and dark to static HTML.
//!
//! ```text
//! cargo run -p yolo-templates --example showcase -- [out-dir]
//! ```
//!
//! Writes `index.html` plus one page per preset and mode (`yolo-aphrody-dark.html`, ...). Each
//! page uses one of the six sites in turn so every site's copy appears; `sites/<id>.html`
//! renders each site in its home preset. Open the files in any browser, or screenshot them
//! headless (see `docs/replica/design-references/test-plan.md` in yolo-web).

use std::fmt::Write as _;
use std::path::PathBuf;

use yolo_shared::YoloThemePreset;
use yolo_templates::{Mode, SITES, Template, render_document};

fn home_preset(site_id: &str) -> YoloThemePreset {
    match site_id {
        "yolo" => YoloThemePreset::Hyprland,
        "shenron" => YoloThemePreset::M3,
        "rosegriffon" => YoloThemePreset::Fluent,
        "auto" => YoloThemePreset::Neutral,
        "craft" => YoloThemePreset::Omarchy,
        _ => YoloThemePreset::Aphrody,
    }
}

fn main() -> std::io::Result<()> {
    let out = std::env::args().nth(1).map_or_else(|| PathBuf::from("target/yolo-templates-showcase"), PathBuf::from);
    std::fs::create_dir_all(out.join("sites"))?;
    let mut index = String::from(
        "<!DOCTYPE html><html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"><title>yolo-templates showcase</title></head><body><h1>yolo-templates showcase</h1><h2>Presets</h2><ul>",
    );
    for (i, preset) in YoloThemePreset::ALL.into_iter().enumerate() {
        let site = &SITES[i % SITES.len()];
        for mode in Mode::ALL {
            let file = format!("{}-{}.html", preset.id(), mode.id());
            std::fs::write(out.join(&file), render_document(site, preset, mode, &Template::ALL))?;
            let _ = write!(index, "<li><a href=\"{file}\">{} {} ({})</a></li>", preset.display_name(), mode.id(), site.name);
        }
    }
    index.push_str("</ul><h2>Sites</h2><ul>");
    for site in &SITES {
        let preset = home_preset(site.id);
        for mode in Mode::ALL {
            let file = format!("sites/{}-{}.html", site.id, mode.id());
            std::fs::write(out.join(&file), render_document(site, preset, mode, &Template::ALL))?;
            let _ = write!(index, "<li><a href=\"{file}\">{} ({}, {})</a></li>", site.name, preset.display_name(), mode.id());
        }
    }
    index.push_str("</ul></body></html>");
    std::fs::write(out.join("index.html"), index)?;
    println!("{}", out.display());
    Ok(())
}
