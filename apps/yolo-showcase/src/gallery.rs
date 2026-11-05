//! Component gallery: every Yolo UI component in every [`YoloThemePreset`], rendered with Dioxus SSR.
//!
//! The page is a matrix. Each column is one preset painted with its own tokens, each row one component
//! with its states shown side by side (static SSR has no hover). The gallery chrome uses the
//! `yolo-neutral` tokens, so the reference page runs on the system it documents.
//!
//! `yolo-showcase --emit-gallery <dir>` writes `index.html` (light) and `dark.html` (dark).

use std::fmt::Write as _;

use yolo_contracts::accessibility::MIN_TARGET_DP;
use yolo_contracts::components::{Button, ButtonVariant};
use dioxus::prelude::*;
use yolo_ui::shared::{RgbaColor, YoloThemePreset};

/// Material 3 baseline error role (light, dark); the shared tokens carry no error role yet.
const ERROR_ROLE: (&str, &str) = ("#B3261E", "#F2B8B5");

/// WCAG 2.2 AA threshold for normal text.
pub const AA_TEXT: f64 = 4.5;

/// Short column label for a preset (the long `display_name` is shown as the column description).
#[must_use]
pub const fn short_name(preset: YoloThemePreset) -> &'static str {
    match preset {
        YoloThemePreset::Neutral => "Neutral",
        YoloThemePreset::M3 => "Material 3",
        YoloThemePreset::Fluent => "Fluent 2",
        YoloThemePreset::Apple => "Apple",
        YoloThemePreset::Omarchy => "Omarchy",
        YoloThemePreset::Hyprland => "Hyprland",
        YoloThemePreset::Aphrody => "Aphrody",
        YoloThemePreset::Chromium => "Chromium",
    }
}

fn channel(c: f32) -> f64 {
    let c = f64::from(c);
    if c <= 0.040_45 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
}

/// WCAG relative luminance.
#[must_use]
pub fn luminance(color: RgbaColor) -> f64 {
    0.2126 * channel(color.r) + 0.7152 * channel(color.g) + 0.0722 * channel(color.b)
}

/// WCAG contrast ratio between two opaque colors.
#[must_use]
pub fn contrast(a: RgbaColor, b: RgbaColor) -> f64 {
    let (la, lb) = (luminance(a), luminance(b));
    let (hi, lo) = if la >= lb { (la, lb) } else { (lb, la) };
    (hi + 0.05) / (lo + 0.05)
}

fn parse(css: &str) -> RgbaColor {
    RgbaColor::parse_css(css).unwrap_or(RgbaColor::BLACK)
}

/// Text color placed on the accent: black or white, whichever contrasts more.
#[must_use]
pub fn on_accent(accent: RgbaColor) -> RgbaColor {
    if contrast(accent, RgbaColor::WHITE) >= contrast(accent, RgbaColor::BLACK) {
        RgbaColor::WHITE
    } else {
        RgbaColor::BLACK
    }
}

/// Measured contrast facts for one preset and scheme.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PresetContrast {
    /// Body text on the page background.
    pub text_on_background: f64,
    /// Secondary text on the surface.
    pub secondary_on_surface: f64,
    /// Label text on the accent fill.
    pub label_on_accent: f64,
}

/// Contrast facts for a preset.
#[must_use]
pub fn preset_contrast(preset: YoloThemePreset, dark: bool) -> PresetContrast {
    let t = preset.tokens(dark);
    let accent = parse(t.accent_primary);
    PresetContrast {
        text_on_background: contrast(parse(t.text_primary), parse(t.bg_primary)),
        secondary_on_surface: contrast(parse(t.text_secondary), parse(t.bg_surface)),
        label_on_accent: contrast(on_accent(accent), accent),
    }
}

/// Scoped custom properties for one preset column (`.p-<preset-id>`).
fn preset_scope(preset: YoloThemePreset, dark: bool) -> String {
    let t = preset.tokens(dark);
    let s = preset.style();
    let accent = parse(t.accent_primary);
    format!(
        ".p-{id}{{--bg:{bg};--surface:{surface};--elevated:{elevated};--text:{text};--muted:{muted};\
         --accent:{accent};--accent-2:{accent2};--on-accent:{on};--rule:{rule};--r-sm:{rsm}px;--r-md:{rmd}px;\
         --r-lg:{rlg}px;--r-pill:{rpill}px;--grid:{grid}px;--bw:{bw}px;--font:{font};\
         background:var(--bg);color:var(--text);font-family:var(--font)}}",
        id = preset.id(),
        bg = t.bg_primary,
        surface = t.bg_surface,
        elevated = t.bg_elevated,
        text = t.text_primary,
        muted = t.text_secondary,
        accent = t.accent_primary,
        accent2 = t.accent_secondary,
        on = on_accent(accent).to_css_hex(),
        rule = t.border_subtle,
        rsm = s.radius_sm_px,
        rmd = s.radius_md_px,
        rlg = s.radius_lg_px,
        rpill = s.radius_pill_px,
        grid = s.spacing_grid_px,
        bw = s.border_width_px.max(1.0),
        font = s.font_family_ui,
    )
}

/// Generated stylesheet: neutral chrome, one scope per preset, shared component rules on the scope variables.
#[must_use]
pub fn gallery_css(dark: bool) -> String {
    let chrome = YoloThemePreset::Neutral.tokens(dark);
    let chrome_style = YoloThemePreset::Neutral.style();
    let target = MIN_TARGET_DP;
    let mut css = format!(
        "*{{box-sizing:border-box}}\
         body{{margin:0;background:{bg};color:{text};font:16px/1.5 {font}}}\
         a{{color:inherit}}\
         :focus-visible{{outline:3px solid {focus};outline-offset:2px}}\
         .top{{padding:40px 32px 28px;max-width:72ch}}\
         .top h1{{font-size:28px;line-height:1.2;font-weight:600;margin:0 0 8px;letter-spacing:-0.01em}}\
         .top p{{margin:0 0 12px;color:{muted}}}\
         .matrix{{display:grid;grid-template-columns:minmax(200px,240px) repeat(8,minmax(0,1fr));border-top:1px solid {rule}}}\
         .head{{position:sticky;top:0;z-index:2;padding:14px 12px;border-bottom:1px solid var(--rule)}}\
         .head h2{{font-size:16px;font-weight:600;margin:0;line-height:1.3}}\
         .head p{{font-size:12px;margin:4px 0 0;color:var(--muted);line-height:1.35}}\
         .head dl{{display:grid;grid-template-columns:auto auto;gap:2px 8px;margin:8px 0 0;font-size:12px}}\
         .head dt{{color:var(--muted)}} .head dd{{margin:0;font-variant-numeric:tabular-nums;text-align:right}}\
         .corner{{position:sticky;top:0;z-index:2;background:{bg};border-bottom:1px solid {rule};padding:14px 16px;font-size:14px;color:{muted}}}\
         .label{{padding:20px 16px;border-bottom:1px solid {rule};background:{bg}}}\
         .label h3{{font-size:16px;font-weight:600;margin:0}}\
         .label p{{font-size:13px;margin:4px 0 0;color:{muted};line-height:1.4}}\
         .cell{{padding:20px 12px;border-bottom:1px solid var(--rule);display:flex;flex-wrap:wrap;gap:var(--grid);align-items:center;align-content:flex-start;min-width:0}}\
         .cell .stack{{display:flex;flex-direction:column;gap:6px;width:100%}}\
         .btn{{min-height:{target}px;min-width:{target}px;padding:0 16px;border-radius:var(--r-pill);font:500 14px/1 var(--font);border:var(--bw) solid transparent;cursor:pointer;display:inline-flex;align-items:center;gap:8px;touch-action:manipulation}}\
         .btn.filled{{background:var(--accent);color:var(--on-accent)}}\
         .btn.tonal{{background:var(--elevated);color:var(--text)}}\
         .btn.outlined{{background:transparent;color:var(--accent);border-color:var(--rule)}}\
         .btn.text{{background:transparent;color:var(--accent);padding:0 12px}}\
         .btn:disabled{{opacity:.38;cursor:not-allowed}}\
         .btn.focus{{outline:3px solid var(--accent);outline-offset:2px}}\
         .icon-btn{{width:{target}px;height:{target}px;padding:0;justify-content:center}}\
         .fab{{width:56px;height:56px;border-radius:var(--r-lg);justify-content:center;padding:0}}\
         svg.i{{width:20px;height:20px;fill:none;stroke:currentColor;stroke-width:2;stroke-linecap:round;stroke-linejoin:round}}\
         .chip{{min-height:32px;padding:0 12px;border-radius:var(--r-sm);border:var(--bw) solid var(--rule);background:transparent;color:var(--text);font:500 13px/1 var(--font);display:inline-flex;align-items:center;gap:6px}}\
         .chip[aria-pressed=true]{{background:var(--elevated);border-color:var(--accent)}}\
         .opt{{display:flex;align-items:center;gap:8px;min-height:{target}px;font-size:14px}}\
         .opt input{{width:20px;height:20px;margin:0;accent-color:var(--accent)}}\
         .switch input{{appearance:none;width:40px;height:24px;border-radius:12px;background:var(--rule);position:relative;margin:0;cursor:pointer}}\
         .switch input::after{{content:\"\";position:absolute;top:3px;left:3px;width:18px;height:18px;border-radius:50%;background:var(--bg)}}\
         .switch input:checked{{background:var(--accent)}} .switch input:checked::after{{left:19px;background:var(--on-accent)}}\
         input[type=range]{{width:100%;accent-color:var(--accent);min-height:{target}px;margin:0}}\
         progress{{width:100%;height:6px;accent-color:var(--accent)}}\
         .field label{{display:block;font-size:13px;font-weight:500;margin-bottom:4px}}\
         .field input{{width:100%;min-height:{target}px;padding:0 12px;font:15px var(--font);color:var(--text);background:var(--surface);border:var(--bw) solid var(--rule);border-radius:var(--r-sm)}}\
         .field .help{{font-size:12px;color:var(--muted);margin:4px 0 0}}\
         .field.error input{{border-color:{error};border-width:2px}} .field.error .help{{color:inherit}}\
         .tabs{{display:flex;width:100%;border-bottom:1px solid var(--rule)}}\
         .tabs button{{flex:1;min-height:{target}px;background:none;border:0;border-bottom:3px solid transparent;color:var(--muted);font:500 14px var(--font);cursor:pointer}}\
         .tabs button[aria-selected=true]{{color:var(--text);border-bottom-color:var(--accent)}}\
         .nav{{display:flex;width:100%;background:var(--surface);border-radius:var(--r-md)}}\
         .nav a{{flex:1;display:flex;flex-direction:column;align-items:center;gap:2px;min-height:56px;justify-content:center;font-size:12px;text-decoration:none;color:var(--muted)}}\
         .nav a[aria-current=page]{{color:var(--text);font-weight:600}}\
         .nav a[aria-current=page] svg{{color:var(--accent)}}\
         .card{{width:100%;padding:16px;border-radius:var(--r-lg)}}\
         .card h4{{margin:0 0 4px;font-size:15px}} .card p{{margin:0;font-size:13px;color:var(--muted)}}\
         .card.filled{{background:var(--surface)}} .card.elevated{{background:var(--elevated)}} .card.outlined{{border:var(--bw) solid var(--rule)}}\
         .menu{{list-style:none;margin:0;padding:6px 0;width:100%;background:var(--elevated);border-radius:var(--r-md);border:var(--bw) solid var(--rule)}}\
         .menu li{{min-height:{target}px;display:flex;align-items:center;padding:0 14px;font-size:14px}}\
         .menu li[aria-disabled=true]{{opacity:.38}}\
         .dialog{{width:100%;padding:20px;border-radius:var(--r-lg);background:var(--elevated);border:var(--bw) solid var(--rule)}}\
         .dialog h4{{margin:0 0 8px;font-size:17px}} .dialog p{{margin:0 0 16px;font-size:14px;color:var(--muted)}}\
         .dialog .actions{{display:flex;justify-content:flex-end;gap:8px;flex-wrap:wrap}}\
         .scheme{{font-size:14px}}\
         @media (max-width:899px){{.matrix{{grid-template-columns:repeat(2,minmax(0,1fr))}}\
           .corner{{display:none}} .label{{grid-column:1/-1}} .head{{position:static}} .top{{padding:28px 20px 20px}}}}\
         @media (prefers-reduced-motion:reduce){{*{{transition:none!important;animation:none!important}}}}",
        bg = chrome.bg_primary,
        text = chrome.text_primary,
        muted = chrome.text_secondary,
        rule = chrome.border_subtle,
        focus = chrome.accent_primary,
        font = chrome_style.font_family_ui,
        error = if dark { ERROR_ROLE.1 } else { ERROR_ROLE.0 },
    );
    for preset in YoloThemePreset::ALL {
        let _ = write!(css, "{}", preset_scope(preset, dark));
    }
    css
}

const PLUS: &str = "M12 5v14M5 12h14";
const HOME: &str = "M4 11l8-7 8 7v9h-5v-6H9v6H4z";
const SEARCH: &str = "M11 18a7 7 0 1 0 0-14 7 7 0 0 0 0 14zM20 20l-4-4";
const BELL: &str = "M6 16V11a6 6 0 1 1 12 0v5l2 2H4zM10 20h4";
const CLOSE: &str = "M6 6l12 12M18 6L6 18";

#[component]
fn Icon(d: &'static str) -> Element {
    rsx! {
        svg { class: "i", view_box: "0 0 24 24", "aria-hidden": "true", path { d: "{d}" } }
    }
}

fn variant_class(variant: ButtonVariant) -> &'static str {
    match variant {
        ButtonVariant::Filled => "filled",
        ButtonVariant::Tonal => "tonal",
        ButtonVariant::Outlined => "outlined",
        ButtonVariant::Text => "text",
    }
}

/// One gallery row: label column plus one cell per preset.
struct Row {
    name: &'static str,
    note: &'static str,
    render: fn(YoloThemePreset) -> Element,
}

fn buttons(_: YoloThemePreset) -> Element {
    let variants = [
        (ButtonVariant::Filled, "Save"),
        (ButtonVariant::Tonal, "Duplicate"),
        (ButtonVariant::Outlined, "Export"),
        (ButtonVariant::Text, "Cancel"),
    ];
    rsx! {
        for (variant, label) in variants {
            {
                let contract = Button { variant, ..Button::new(label) };
                rsx! {
                    button {
                        class: "btn {variant_class(contract.variant)}",
                        r#type: "button",
                        style: "min-height:{contract.min_target_dp}px",
                        "{contract.label}"
                    }
                }
            }
        }
        button { class: "btn filled focus", r#type: "button", "Focused" }
        button { class: "btn filled", r#type: "button", disabled: true, "Disabled" }
    }
}

fn icon_buttons(_: YoloThemePreset) -> Element {
    rsx! {
        button { class: "btn tonal icon-btn", r#type: "button", "aria-label": "Search", Icon { d: SEARCH } }
        button { class: "btn outlined icon-btn", r#type: "button", "aria-label": "Notifications", Icon { d: BELL } }
        button { class: "btn filled fab", r#type: "button", "aria-label": "Create", Icon { d: PLUS } }
    }
}

fn chips(_: YoloThemePreset) -> Element {
    rsx! {
        button { class: "chip", r#type: "button", "aria-pressed": "false", "Nearby" }
        button { class: "chip", r#type: "button", "aria-pressed": "true", "Open now" }
        span { class: "chip",
            "Paris"
            button { class: "btn text icon-btn", r#type: "button", "aria-label": "Remove Paris", style: "min-height:32px;min-width:32px;width:32px;height:32px", Icon { d: CLOSE } }
        }
    }
}

fn selection(preset: YoloThemePreset) -> Element {
    let id = preset.id();
    rsx! {
        div { class: "stack",
            label { class: "opt", input { r#type: "checkbox" } "Email updates" }
            label { class: "opt", input { r#type: "checkbox", checked: true } "Weekly digest" }
            label { class: "opt switch", input { r#type: "checkbox", role: "switch", checked: true } "Sync" }
            fieldset { style: "border:0;margin:0;padding:0",
                legend { style: "font-size:13px;font-weight:500;padding:0", "Density" }
                label { class: "opt", input { r#type: "radio", name: "density-{id}", checked: true } "Comfortable" }
                label { class: "opt", input { r#type: "radio", name: "density-{id}" } "Compact" }
            }
        }
    }
}

fn slider_progress(preset: YoloThemePreset) -> Element {
    let id = preset.id();
    rsx! {
        div { class: "stack",
            label { r#for: "vol-{id}", style: "font-size:13px;font-weight:500", "Volume" }
            input { id: "vol-{id}", r#type: "range", min: "0", max: "100", value: "40" }
            label { r#for: "up-{id}", style: "font-size:13px;font-weight:500", "Upload" }
            progress { id: "up-{id}", max: "100", value: "64", "64%" }
        }
    }
}

fn text_fields(preset: YoloThemePreset) -> Element {
    let id = preset.id();
    rsx! {
        div { class: "stack",
            div { class: "field",
                label { r#for: "name-{id}", "Display name" }
                input { id: "name-{id}", r#type: "text", value: "Byron Love", autocomplete: "nickname", "aria-describedby": "name-help-{id}" }
                p { class: "help", id: "name-help-{id}", "Shown on your profile." }
            }
            div { class: "field error",
                label { r#for: "mail-{id}", "Email" }
                input { id: "mail-{id}", r#type: "email", value: "byron@", autocomplete: "email", "aria-invalid": "true", "aria-describedby": "mail-err-{id}" }
                p { class: "help", id: "mail-err-{id}", "Add the domain after @, for example byron@aphrody.com." }
            }
        }
    }
}

fn navigation(_: YoloThemePreset) -> Element {
    rsx! {
        div { class: "tabs", role: "tablist", "aria-label": "Library",
            button { role: "tab", "aria-selected": "true", r#type: "button", "Songs" }
            button { role: "tab", "aria-selected": "false", r#type: "button", "Albums" }
            button { role: "tab", "aria-selected": "false", r#type: "button", "Artists" }
        }
        nav { class: "nav", "aria-label": "Main",
            a { href: "#home", "aria-current": "page", Icon { d: HOME } "Home" }
            a { href: "#search", Icon { d: SEARCH } "Search" }
            a { href: "#alerts", Icon { d: BELL } "Alerts" }
        }
    }
}

fn cards(_: YoloThemePreset) -> Element {
    rsx! {
        div { class: "stack",
            article { class: "card filled", h4 { "Filled" } p { "Groups related content on the surface." } }
            article { class: "card elevated", h4 { "Elevated" } p { "Lifts content above the surface." } }
            article { class: "card outlined", h4 { "Outlined" } p { "Separates content with a rule." } }
        }
    }
}

fn overlays(preset: YoloThemePreset) -> Element {
    let id = preset.id();
    rsx! {
        div { class: "stack",
            ul { class: "menu", role: "menu", "aria-label": "Track actions",
                li { role: "menuitem", "Add to queue" }
                li { role: "menuitem", "Share" }
                li { role: "menuitem", "aria-disabled": "true", "Download" }
            }
            section { class: "dialog", "aria-labelledby": "dlg-{id}",
                h4 { id: "dlg-{id}", "Delete playlist?" }
                p { "The 24 songs stay in your library." }
                div { class: "actions",
                    button { class: "btn text", r#type: "button", "Keep" }
                    button { class: "btn filled", r#type: "button", "Delete" }
                }
            }
        }
    }
}

const ROWS: [Row; 9] = [
    Row { name: "Buttons", note: "Filled, tonal, outlined and text, plus focused and disabled. 48 dp minimum target.", render: buttons },
    Row { name: "Icon buttons", note: "Icon-only controls carry an accessible name.", render: icon_buttons },
    Row { name: "Chips", note: "Filter chips expose aria-pressed; input chips have a remove control.", render: chips },
    Row { name: "Selection", note: "Native checkbox, switch and radio with the preset accent.", render: selection },
    Row { name: "Slider and progress", note: "Native range and progress elements with visible labels.", render: slider_progress },
    Row { name: "Text fields", note: "Visible label, helper text, and an error that says how to fix it.", render: text_fields },
    Row { name: "Navigation", note: "Tabs and a bottom bar; the current place is marked by weight and accent.", render: navigation },
    Row { name: "Cards", note: "Filled, elevated and outlined containers.", render: cards },
    Row { name: "Menu and dialog", note: "Static samples of the overlay surfaces.", render: overlays },
];

fn ratio(v: f64) -> String {
    let verdict = if v >= AA_TEXT { "pass" } else { "fail" };
    format!("{v:.1}:1 {verdict}")
}

/// Renders the gallery page body for one scheme.
#[component]
fn Gallery(dark: bool) -> Element {
    let (scheme, other_label, other_href) =
        if dark { ("Dark scheme.", "Show light scheme", "index.html") } else { ("Light scheme.", "Show dark scheme", "dark.html") };
    rsx! {
        header { class: "top",
            h1 { "Yolo UI components" }
            p { "Every component in all eight presets, rendered from the shared contracts. {scheme} Contrast is measured against WCAG 2.2 AA (4.5:1)." }
            a { class: "scheme", href: "{other_href}", "{other_label}" }
        }
        main { class: "matrix",
            div { class: "corner", "Component" }
            for preset in YoloThemePreset::ALL {
                {
                    let c = preset_contrast(preset, dark);
                    rsx! {
                        section { class: "head p-{preset.id()}", "aria-label": "{short_name(preset)} preset",
                            h2 { "{short_name(preset)}" }
                            p { "{preset.display_name()}" }
                            dl {
                                dt { "Text" } dd { "{ratio(c.text_on_background)}" }
                                dt { "Muted" } dd { "{ratio(c.secondary_on_surface)}" }
                                dt { "On accent" } dd { "{ratio(c.label_on_accent)}" }
                            }
                        }
                    }
                }
            }
            for row in ROWS {
                div { class: "label", h3 { "{row.name}" } p { "{row.note}" } }
                for preset in YoloThemePreset::ALL {
                    div { class: "cell p-{preset.id()}", {(row.render)(preset)} }
                }
            }
        }
    }
}

/// Full HTML document for one scheme.
#[must_use]
pub fn render_gallery_html(dark: bool) -> String {
    let css = gallery_css(dark);
    let title = if dark { "Yolo UI components, dark" } else { "Yolo UI components" };
    let body = dioxus_ssr::render_element(rsx! {
        html { lang: "en",
            head {
                meta { charset: "utf-8" }
                meta { name: "viewport", content: "width=device-width, initial-scale=1" }
                meta { name: "color-scheme", content: if dark { "dark" } else { "light" } }
                title { "{title}" }
                style { "{css}" }
            }
            body { Gallery { dark } }
        }
    });
    format!("<!doctype html>{body}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contrast_matches_wcag_reference_values() {
        assert!((contrast(RgbaColor::BLACK, RgbaColor::WHITE) - 21.0).abs() < 1e-9);
        assert!((contrast(RgbaColor::WHITE, RgbaColor::WHITE) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn on_accent_picks_the_stronger_label() {
        for preset in YoloThemePreset::ALL {
            for dark in [false, true] {
                let accent = parse(preset.tokens(dark).accent_primary);
                let chosen = contrast(on_accent(accent), accent);
                let other = if on_accent(accent) == RgbaColor::WHITE { RgbaColor::BLACK } else { RgbaColor::WHITE };
                assert!(chosen >= contrast(other, accent), "{} dark={dark}", preset.id());
            }
        }
    }

    #[test]
    fn gallery_renders_every_preset_and_row() {
        for dark in [false, true] {
            let html = render_gallery_html(dark);
            assert!(html.starts_with("<!doctype html><html lang=\"en\">"));
            for preset in YoloThemePreset::ALL {
                assert!(html.contains(&format!(".p-{}{{", preset.id())), "scope for {}", preset.id());
                assert_eq!(html.matches(&format!("cell p-{}", preset.id())).count(), ROWS.len());
            }
            for row in &ROWS {
                assert!(html.contains(row.name));
            }
            assert!(html.contains("aria-invalid=\"true\""));
            assert!(html.contains("aria-label=\"Search\""));
            assert!(!html.contains("undefined"));
        }
    }
}
