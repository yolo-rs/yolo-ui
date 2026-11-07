// SPDX-License-Identifier: Apache-2.0
//! Renders every template for every site, preset and mode on the server and checks the
//! markup for the accessibility and content rules of the template contract.

use std::collections::{BTreeMap, BTreeSet};

use yolo_shared::YoloThemePreset;
use yolo_templates::content::{Strings, SITES};
use yolo_templates::{Mode, Template, render_document};

/// Every value of attribute `name` in `html`, in order.
fn attr_values<'a>(html: &'a str, name: &str) -> Vec<&'a str> {
    let mut out = Vec::new();
    for quote in ['"', '\''] {
        let needle = format!(" {name}={quote}");
        let mut rest = html;
        while let Some(i) = rest.find(&needle) {
            let tail = &rest[i + needle.len()..];
            let end = tail.find(quote).unwrap_or(tail.len());
            out.push(&tail[..end]);
            rest = &tail[end..];
        }
    }
    out
}

/// The opening tags of `tag` (`<button ...>`) with their inner content up to the matching close.
fn elements<'a>(html: &'a str, tag: &str) -> Vec<(&'a str, &'a str)> {
    let open = format!("<{tag}");
    let close = format!("</{tag}>");
    let mut out = Vec::new();
    let mut rest = html;
    while let Some(i) = rest.find(&open) {
        let after = &rest[i..];
        let next = after.as_bytes().get(open.len()).copied().unwrap_or(b'>');
        if !(next == b' ' || next == b'>') {
            rest = &after[open.len()..];
            continue;
        }
        let head_end = after.find('>').map_or(after.len(), |e| e + 1);
        let head = &after[..head_end];
        let body_end = after[head_end..].find(&close).map_or(after.len(), |e| head_end + e);
        out.push((head, &after[head_end..body_end]));
        rest = &after[head_end..];
    }
    out
}

fn strip_tags(html: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            c if !in_tag => out.push(c),
            _ => {}
        }
    }
    out
}

fn body(doc: &str) -> &str {
    let start = doc.find("<body").expect("body");
    &doc[start..]
}

fn all_documents() -> Vec<(String, String)> {
    let mut docs = Vec::new();
    for (i, preset) in YoloThemePreset::ALL.into_iter().enumerate() {
        for mode in Mode::ALL {
            // Each preset and mode with a rotating site, plus every site in its own preset.
            let site = &SITES[i % SITES.len()];
            docs.push((format!("{}-{}-{}", site.id, preset.id(), mode.id()), render_document(site, preset, mode, &Template::ALL)));
        }
    }
    for site in &SITES {
        docs.push((format!("{}-aphrody-dark", site.id), render_document(site, YoloThemePreset::Aphrody, Mode::Dark, &Template::ALL)));
    }
    docs
}

#[test]
fn every_page_renders_every_template() {
    for (name, doc) in all_documents() {
        let b = body(&doc);
        for class in [
            "yt-globalnav", "yt-flyout", "yt-mega", "yt-stage", "yt-pill", "yt-starfield", "yt-sectiontabs", "yt-product",
            "yt-faq", "yt-channels", "yt-reveal", "yt-editorial", "yt-app", "yt-rail", "yt-search", "yt-media", "yt-docs",
            "yt-docs-tree", "yt-segmented", "yt-dark-switch", "yt-tokens",
        ] {
            assert!(b.contains(&format!("class=\"{class}")) || b.contains(&format!(" {class}\"")) || b.contains(&format!(" {class} ")), "{name}: missing {class}");
        }
    }
}

#[test]
fn one_h1_and_unique_ids_and_resolvable_references() {
    for (name, doc) in all_documents() {
        let b = body(&doc);
        assert_eq!(b.matches("<h1").count(), 1, "{name}: exactly one h1");
        let ids = attr_values(b, "id");
        let unique: BTreeSet<_> = ids.iter().collect();
        assert_eq!(unique.len(), ids.len(), "{name}: duplicate ids");
        for attr in ["popovertarget", "aria-controls", "aria-labelledby"] {
            for target in attr_values(b, attr) {
                assert!(ids.contains(&target), "{name}: {attr}={target} has no element");
            }
        }
        for href in attr_values(b, "href") {
            if let Some(fragment) = href.strip_prefix('#') {
                assert!(ids.contains(&fragment), "{name}: link to missing #{fragment}");
            }
        }
    }
}

#[test]
fn every_control_has_an_accessible_name() {
    for (name, doc) in all_documents() {
        let b = body(&doc);
        for (head, inner) in elements(b, "button") {
            assert!(
                head.contains("aria-label=") || !strip_tags(inner).trim().is_empty(),
                "{name}: unnamed button {head}"
            );
            assert!(head.contains("type="), "{name}: button without type {head}");
        }
        for (head, inner) in elements(b, "a") {
            assert!(head.contains("href="), "{name}: link without href {head}");
            assert!(!strip_tags(inner).trim().is_empty() || head.contains("aria-label="), "{name}: empty link {head}");
        }
        // Inputs are named by aria-label or sit inside a <label> with text.
        for (head, inner) in elements(b, "label") {
            if inner.contains("<input") && !inner.contains("aria-label=") {
                assert!(!strip_tags(inner).trim().is_empty(), "{name}: unnamed input in {head}");
            }
        }
        assert_eq!(
            b.matches("<input").count(),
            elements(b, "label").iter().map(|(_, inner)| inner.matches("<input").count()).sum::<usize>(),
            "{name}: input outside a label"
        );
        for (head, _) in elements(b, "nav") {
            assert!(head.contains("aria-label="), "{name}: unlabelled nav {head}");
        }
        assert!(!b.contains("<img"), "{name}: image file in templates");
        assert!(!b.contains("target=\"_blank\""), "{name}: new-window link");
    }
}

#[test]
fn icons_are_hidden_and_decorative_layers_are_hidden() {
    for (name, doc) in all_documents() {
        let b = body(&doc);
        for (head, _) in elements(b, "svg") {
            assert!(head.contains("aria-hidden=\"true\""), "{name}: exposed icon {head}");
        }
        for class in ["yt-starfield", "yt-object", "yt-thumb"] {
            for (head, _) in elements(b, "div") {
                if head.contains(&format!("class=\"{class}")) {
                    assert!(head.contains("aria-hidden=\"true\""), "{name}: {class} exposed");
                }
            }
        }
        // The drawn UI is one image with a description.
        for (head, _) in elements(b, "div") {
            if head.contains("class=\"yt-sketch\"") {
                assert!(head.contains("role=\"img\"") && head.contains("aria-label="), "{name}: sketch without alt");
            }
        }
    }
}

#[test]
fn state_matches_mode_and_motion_has_a_pause_control() {
    for mode in Mode::ALL {
        let doc = render_document(&SITES[1], YoloThemePreset::Hyprland, mode, &Template::ALL);
        let b = body(&doc);
        assert!(b.contains(&format!("data-yt-mode=\"{}\"", mode.id())));
        let switch = elements(b, "input").into_iter().find(|(h, _)| h.contains("yt-dark-switch")).unwrap().0;
        assert_eq!(switch.contains("checked"), mode.is_dark(), "dark switch reflects the mode");
        let pause = elements(b, "input").into_iter().find(|(h, _)| h.contains("yt-pause-switch")).unwrap().0;
        assert!(pause.contains("role=\"switch\"") && !pause.contains("checked"));
    }
}

fn is_emoji(c: char) -> bool {
    matches!(c as u32, 0x1F000..=0x1FAFF | 0x2600..=0x27BF | 0xFE0F)
}

#[test]
fn copy_is_original_and_finished() {
    let banned = ["lorem", "ipsum", "placeholder text", "todo", "tbd", "xxx", "coming soon", "your text", "sample text"];
    let mut words: BTreeMap<&str, usize> = BTreeMap::new();
    for (name, doc) in all_documents() {
        let text = strip_tags(body(&doc)).to_lowercase();
        for b in banned {
            assert!(!text.contains(b), "{name}: banned copy `{b}`");
        }
        assert!(!text.chars().any(is_emoji), "{name}: emoji in copy");
        *words.entry("pages").or_default() += 1;
    }
    assert_eq!(words["pages"], 22);
}

#[test]
fn french_sites_render_french_interface() {
    let fr: Vec<_> = SITES.iter().filter(|s| s.strings.lang == "fr").map(|s| s.id).collect();
    assert_eq!(fr, ["shenron", "rosegriffon", "auto"]);
    let s: &Strings = SITES[2].strings;
    let doc = render_document(&SITES[2], YoloThemePreset::M3, Mode::Light, &Template::ALL);
    assert!(doc.starts_with("<!DOCTYPE html><html lang=\"fr\">"));
    assert!(doc.contains(s.skip) && doc.contains(s.pause_motion));
}

#[test]
fn every_site_fills_every_slot() {
    for site in &SITES {
        assert!(site.nav.iter().any(|n| n.groups.len() >= 2), "{}: needs a mega menu item", site.id);
        assert!(site.nav.iter().any(|n| n.groups.len() == 1), "{}: needs a flyout item", site.id);
        assert!(site.product.sections.len() >= 3 && site.product.faqs.len() >= 2, "{}", site.id);
        assert!(!site.downloads.channels.is_empty() && site.downloads.channels.iter().filter(|c| c.recommended).count() == 1);
        assert!(site.app.rail.len() >= 4 && site.app.media.len() >= 6, "{}", site.id);
        assert_eq!(site.editorial.stories.len(), 4, "{}", site.id);
        let ids: BTreeSet<_> = site.product.sections.iter().map(|s| s.id).collect();
        assert_eq!(ids.len(), site.product.sections.len(), "{}: section ids unique", site.id);
    }
}
