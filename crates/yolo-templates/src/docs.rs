// SPDX-License-Identifier: Apache-2.0
//! Design-system docs shell: navigation tree, platform switcher, theme switch and a token page
//! generated from the resolved palette.

use dioxus::prelude::*;
use yolo_shared::YoloThemePreset;

use crate::content::{DocNode, Docs, Strings};
use crate::theme::{Mode, Palette, contrast};

/// Formats a contrast ratio for the token table (`"4.62:1"`).
#[must_use]
pub fn ratio_label(ratio: f64) -> String {
    format!("{:.2}:1", (ratio * 100.0).floor() / 100.0)
}

/// One row of the token table.
#[derive(Clone, Debug, PartialEq)]
pub struct TokenRow {
    /// Role name.
    pub role: &'static str,
    /// Light value as CSS.
    pub light: String,
    /// Dark value as CSS.
    pub dark: String,
    /// Light contrast cell, or `None` for background and decorative roles.
    pub light_ratio: Option<String>,
    /// Dark contrast cell.
    pub dark_ratio: Option<String>,
}

/// Token rows of `preset` in both modes.
#[must_use]
pub fn token_rows(preset: YoloThemePreset) -> Vec<TokenRow> {
    let light = Palette::resolve(preset, Mode::Light);
    let dark = Palette::resolve(preset, Mode::Dark);
    let cell = |p: &Palette, role: &str, c: yolo_shared::RgbaColor| p.requirement(role).map(|(bg, _)| ratio_label(contrast(c, bg)));
    light
        .roles()
        .into_iter()
        .zip(dark.roles())
        .map(|((role, lc), (_, dc))| TokenRow {
            role,
            light: lc.to_css_string(),
            dark: dc.to_css_string(),
            light_ratio: cell(&light, role, lc),
            dark_ratio: cell(&dark, role, dc),
        })
        .collect()
}

#[component]
fn TreeNode(node: DocNode, current: &'static str) -> Element {
    if node.children.is_empty() {
        return rsx! {
            a {
                class: "yt-tree-link",
                href: node.href,
                "aria-current": if node.href == current { "page" } else { "false" },
                {node.label}
            }
        };
    }
    let open = node.href == current || node.children.iter().any(|c| c.href == current);
    rsx! {
        details { class: "yt-tree-group", open: open,
            summary { class: "yt-tree-summary", {node.label} }
            ul {
                for (i , child) in node.children.iter().enumerate() {
                    li { key: "{i}",
                        TreeNode { node: *child, current: current }
                    }
                }
            }
        }
    }
}

/// The docs shell. The platform radios switch the code sample, the theme switch flips the
/// whole page between light and dark (both by CSS `:has()`, so they work before hydration and
/// with scripting off), and the token table lists both modes with their measured ratios.
#[component]
pub fn DocsShell(docs: Docs, preset: YoloThemePreset, mode: Mode, strings: &'static Strings) -> Element {
    let mut dark = use_signal(|| mode.is_dark());
    let mut platform = use_signal(|| 0usize);
    let rows = token_rows(preset);
    let cols = strings.token_columns;
    rsx! {
        div { class: "yt-docs",
            div { class: "yt-docs-top",
                a { class: "yt-wordmark", href: "/docs", "{docs.product} docs" }
                div { class: "yt-docs-controls",
                    fieldset { class: "yt-segmented",
                        legend { class: "yt-segmented-legend", {strings.platform} }
                        for (i , p) in docs.platforms.iter().enumerate() {
                            label { key: "{p.id}", class: "yt-segment",
                                input {
                                    class: "yt-sr",
                                    r#type: "radio",
                                    name: "yt-platform",
                                    id: "yt-plat-{p.id}",
                                    value: p.id,
                                    checked: platform() == i,
                                    onchange: move |_| platform.set(i),
                                }
                                span { {p.label} }
                            }
                        }
                    }
                    label { class: "yt-switch",
                        input {
                            class: "yt-dark-switch",
                            r#type: "checkbox",
                            role: "switch",
                            checked: dark(),
                            onchange: move |_| dark.set(!dark()),
                        }
                        span { {strings.dark_theme} }
                    }
                }
            }
            nav { class: "yt-docs-tree", "aria-label": strings.docs_nav,
                ul {
                    for (i , node) in docs.tree.iter().enumerate() {
                        li { key: "{i}",
                            TreeNode { node: *node, current: docs.current }
                        }
                    }
                }
            }
            div { class: "yt-docs-main",
                h2 { class: "yt-docs-title", {docs.title} }
                p { class: "yt-lede", {docs.intro} }
                div { class: "yt-code-panels",
                    for p in docs.platforms.iter() {
                        pre { key: "{p.id}", class: "yt-code yt-code-{p.id}", tabindex: "0", "aria-label": p.label,
                            code { {p.snippet} }
                        }
                    }
                }
                div { class: "yt-table-wrap", tabindex: "0", role: "region", "aria-label": docs.title,
                    table { class: "yt-tokens",
                        caption { {strings.token_caption} }
                        thead {
                            tr {
                                for (i , c) in cols.iter().enumerate() {
                                    th { key: "{i}", scope: "col", {*c} }
                                }
                            }
                        }
                        tbody {
                            for row in rows.iter() {
                                tr { key: "{row.role}",
                                    th { scope: "row",
                                        code { {row.role} }
                                    }
                                    td {
                                        span { class: "yt-swatch", style: "background:{row.light}", "aria-hidden": "true" }
                                        code { "{row.light}" }
                                    }
                                    td {
                                        span { class: "yt-swatch", style: "background:{row.dark}", "aria-hidden": "true" }
                                        code { "{row.dark}" }
                                    }
                                    td {
                                        {ratio_cell(row.light_ratio.as_deref(), strings)}
                                    }
                                    td {
                                        {ratio_cell(row.dark_ratio.as_deref(), strings)}
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn ratio_cell(ratio: Option<&str>, strings: &'static Strings) -> String {
    match ratio {
        Some(r) => format!("{r} {}", strings.passes),
        None => strings.decorative.to_owned(),
    }
}

/// CSS rules that show the code sample of the checked platform.
#[must_use]
pub fn platform_css(docs: &Docs) -> String {
    let mut css = String::from(".yt-code{display:none}");
    for p in docs.platforms {
        css.push_str(&format!(".yt-docs:has(#yt-plat-{0}:checked) .yt-code-{0}{{display:block}}", p.id));
    }
    css
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ratio_label_rounds_down_so_it_never_overstates() {
        assert_eq!(ratio_label(4.499), "4.49:1");
        assert_eq!(ratio_label(21.0), "21.00:1");
    }

    #[test]
    fn token_rows_report_every_role_in_both_modes() {
        for preset in YoloThemePreset::ALL {
            let rows = token_rows(preset);
            assert_eq!(rows.len(), 18);
            let text = rows.iter().find(|r| r.role == "text").unwrap();
            assert!(text.light_ratio.is_some() && text.dark_ratio.is_some());
            let canvas = rows.iter().find(|r| r.role == "canvas").unwrap();
            assert!(canvas.light_ratio.is_none());
        }
    }

    #[test]
    fn platform_css_has_one_rule_per_platform() {
        let css = platform_css(&crate::content::DOCS);
        assert_eq!(css.matches(":checked").count(), crate::content::DOCS.platforms.len());
    }
}
