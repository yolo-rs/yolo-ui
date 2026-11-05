// SPDX-License-Identifier: Apache-2.0
//! Every preset x target output is parsed by a real parser for its format and checked for
//! structure, and the main on-color pairs of every preset meet WCAG 2.2 AA in both modes.

use std::collections::{BTreeMap, BTreeSet};
use yolo_emit::contrast::{AA_TEXT, failures, ratio};
use yolo_emit::{
    EmittedFile, Target, Theme, ThemeSource, YoloThemePreset, chrome, compose, emit, emit_files,
    fluent, omarchy, winui,
};

fn sources() -> Vec<ThemeSource> {
    let mut out: Vec<ThemeSource> = YoloThemePreset::ALL
        .into_iter()
        .map(ThemeSource::from_preset)
        .collect();
    out.push(ThemeSource::from_theme(Theme {
        seed: 0xFF8C_6C88,
        variant: yolo_tokens::scheme::Variant::Expressive,
        contrast: 0.5,
    }));
    out
}

fn files(src: &ThemeSource, target: Target) -> Vec<EmittedFile> {
    let files = emit_files(src, target);
    assert!(
        !files.is_empty(),
        "{} {}: no files",
        src.slug,
        target.name()
    );
    let paths: BTreeSet<&str> = files.iter().map(|f| f.path.as_str()).collect();
    assert_eq!(
        paths.len(),
        files.len(),
        "{} {}: duplicate paths",
        src.slug,
        target.name()
    );
    for f in &files {
        assert!(!f.contents.trim().is_empty(), "{}: empty", f.path);
        assert!(
            !f.path.starts_with('/') && !f.path.contains(".."),
            "{}: unsafe path",
            f.path
        );
    }
    files
}

fn is_hex6(s: &str) -> bool {
    s.len() == 7 && s.starts_with('#') && s[1..].bytes().all(|b| b.is_ascii_hexdigit())
}

fn parse_hex(s: &str) -> u32 {
    assert!(is_hex6(s), "not #rrggbb: {s}");
    0xFF00_0000 | u32::from_str_radix(&s[1..], 16).unwrap()
}

// ---------------------------------------------------------------------------------------------
// CSS (lightningcss)
// ---------------------------------------------------------------------------------------------

fn parse_css(name: &str, code: &str) {
    use lightningcss::stylesheet::{ParserOptions, StyleSheet};
    let sheet = StyleSheet::parse(code, ParserOptions::default());
    let sheet = sheet.unwrap_or_else(|e| panic!("{name}: CSS parse error: {e}"));
    assert!(!sheet.rules.0.is_empty(), "{name}: no rules");
}

/// Custom properties defined in `code` (`--name:`).
fn defined_vars(code: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for line in code.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("--") {
            if let Some((name, _)) = rest.split_once(':') {
                out.insert(name.trim().to_owned());
            }
        }
    }
    out
}

/// Every `var(--x)` reference resolves to a property defined in the same file.
fn vars_resolve(name: &str, code: &str) {
    let defined = defined_vars(code);
    let mut rest = code;
    while let Some(i) = rest.find("var(--") {
        let tail = &rest[i + 6..];
        let end = tail.find([')', ',']).unwrap();
        let var = tail[..end].trim();
        assert!(defined.contains(var), "{name}: var(--{var}) is undefined");
        rest = &tail[end..];
    }
}

#[test]
fn css_targets_parse() {
    for src in sources() {
        for target in [Target::Css, Target::Tailwind, Target::Gtk, Target::Apple] {
            for f in files(&src, target) {
                let label = format!("{} {} {}", src.slug, target.name(), f.path);
                parse_css(&label, &f.contents);
                vars_resolve(&label, &f.contents);
            }
        }
    }
}

#[test]
fn tailwind_theme_maps_every_role() {
    for src in sources() {
        let css = emit_files(&src, Target::Tailwind).remove(0).contents;
        assert!(css.contains("@theme inline {"));
        assert!(css.contains("@custom-variant dark"));
        for role in yolo_emit::ROLES {
            assert!(css.contains(&format!(
                "--color-{0}: var(--md-sys-color-{0});",
                role.name()
            )));
        }
        assert!(css.contains("[data-theme=\"dark\"] {"));
    }
}

#[test]
fn gtk_define_color_sheets_cover_libadwaita_names() {
    const REQUIRED: [&str; 12] = [
        "accent_bg_color",
        "accent_fg_color",
        "window_bg_color",
        "window_fg_color",
        "view_bg_color",
        "headerbar_bg_color",
        "card_bg_color",
        "dialog_bg_color",
        "popover_bg_color",
        "destructive_bg_color",
        "success_bg_color",
        "warning_bg_color",
    ];
    for src in sources() {
        let files = files(&src, Target::Gtk);
        assert_eq!(files[0].path, "gtk.css");
        assert!(files[0].contents.contains("--accent-bg-color:"));
        for f in &files[1..] {
            let mut defined = BTreeMap::new();
            for line in f.contents.lines() {
                if let Some(rest) = line.strip_prefix("@define-color ") {
                    let (name, value) = rest.split_once(' ').unwrap();
                    let value = value.trim_end_matches(';');
                    assert!(
                        is_hex6(value) || value.starts_with("rgba("),
                        "{}: {name} {value}",
                        f.path
                    );
                    assert!(defined.insert(name.to_owned(), value.to_owned()).is_none());
                }
            }
            for name in REQUIRED {
                assert!(defined.contains_key(name), "{}: missing {name}", f.path);
            }
            for line in f.contents.lines() {
                let mut rest = line;
                while let Some(i) = rest.find('@') {
                    let tail = &rest[i + 1..];
                    let end = tail
                        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
                        .unwrap_or(tail.len());
                    let name = &tail[..end];
                    if name != "define" {
                        assert!(defined.contains_key(name), "{}: @{name} undefined", f.path);
                    }
                    rest = &tail[end..];
                }
            }
            let fg = parse_hex(&defined["window_fg_color"]);
            let bg = parse_hex(&defined["window_bg_color"]);
            assert!(ratio(fg, bg) >= AA_TEXT, "{}: window text contrast", f.path);
            let fg = parse_hex(&defined["accent_fg_color"]);
            let bg = parse_hex(&defined["accent_bg_color"]);
            assert!(ratio(fg, bg) >= AA_TEXT, "{}: accent text contrast", f.path);
        }
    }
}

// ---------------------------------------------------------------------------------------------
// JSON (serde_json): Chrome, Fluent, DTCG, asset catalogs
// ---------------------------------------------------------------------------------------------

fn json(label: &str, text: &str) -> serde_json::Value {
    serde_json::from_str(text).unwrap_or_else(|e| panic!("{label}: JSON error: {e}"))
}

fn rgb_array(v: &serde_json::Value) -> u32 {
    let a = v.as_array().expect("color is an array");
    assert_eq!(a.len(), 3);
    let c: Vec<u32> = a
        .iter()
        .map(|x| {
            let n = x.as_u64().expect("integer channel");
            assert!(n <= 255);
            n as u32
        })
        .collect();
    0xFF00_0000 | (c[0] << 16) | (c[1] << 8) | c[2]
}

#[test]
fn chrome_manifests_are_valid_themes() {
    let allowed: BTreeSet<&str> = chrome::COLOR_KEYS.into_iter().collect();
    for src in sources() {
        let files = files(&src, Target::Chrome);
        assert_eq!(files.len(), 2);
        for (f, dark) in files.iter().zip([true, false]) {
            assert_eq!(
                f.path,
                format!("{}/manifest.json", chrome::dir_name(&src, dark))
            );
            let m = json(&f.path, &f.contents);
            let obj = m.as_object().unwrap();
            let keys: BTreeSet<&str> = obj.keys().map(String::as_str).collect();
            assert_eq!(
                keys,
                [
                    "description",
                    "manifest_version",
                    "name",
                    "theme",
                    "version"
                ]
                .into_iter()
                .collect()
            );
            assert_eq!(m["manifest_version"], 3);
            let name = m["name"].as_str().unwrap();
            assert!(!name.is_empty() && name.chars().count() <= 75, "{name}");
            assert!(m["description"].as_str().unwrap().chars().count() <= 132);
            let version = m["version"].as_str().unwrap();
            let parts: Vec<&str> = version.split('.').collect();
            assert!((1..=4).contains(&parts.len()));
            assert!(parts.iter().all(|p| p.parse::<u16>().is_ok()), "{version}");
            let theme = m["theme"].as_object().unwrap();
            for section in theme.keys() {
                assert!(["colors", "properties", "tints", "images"].contains(&section.as_str()));
            }
            let colors = theme["colors"].as_object().unwrap();
            let mut values = BTreeMap::new();
            for (k, v) in colors {
                assert!(
                    allowed.contains(k.as_str()),
                    "{}: unknown color {k}",
                    f.path
                );
                values.insert(k.as_str(), rgb_array(v));
            }
            assert_eq!(values.len(), chrome::COLOR_KEYS.len());
            for p in theme["properties"].as_object().unwrap().keys() {
                assert!(chrome::PROPERTY_KEYS.contains(&p.as_str()));
            }
            for (fg, bg) in chrome::TEXT_PAIRS {
                let r = ratio(values[fg], values[bg]);
                assert!(r >= AA_TEXT, "{}: {fg} on {bg} = {r:.2}", f.path);
            }
        }
    }
}

#[test]
fn fluent_json_has_brand_ramp_and_modes() {
    for src in sources() {
        let f = &files(&src, Target::Fluent)[0];
        let v = json(&f.path, &f.contents);
        let brand = v["brand"].as_object().unwrap();
        let steps: Vec<u32> = (1..=16).map(|i| i * 10).collect();
        assert_eq!(brand.len(), 16);
        let mut last = -1.0;
        for s in steps {
            let hex = brand[&s.to_string()].as_str().unwrap();
            let l = yolo_emit::contrast::relative_luminance(parse_hex(hex));
            assert!(l >= last, "brand ramp must lighten monotonically");
            last = l;
        }
        for mode in ["light", "dark"] {
            let m = v[mode].as_object().unwrap();
            assert_eq!(m.len(), fluent::token_names().len());
            for name in fluent::token_names() {
                assert!(is_hex6(m[name].as_str().unwrap()));
            }
            let fg = parse_hex(m["colorNeutralForeground1"].as_str().unwrap());
            let bg = parse_hex(m["colorNeutralBackground1"].as_str().unwrap());
            assert!(ratio(fg, bg) >= AA_TEXT);
            let fg = parse_hex(m["colorNeutralForegroundOnBrand"].as_str().unwrap());
            let bg = parse_hex(m["colorBrandBackground"].as_str().unwrap());
            assert!(ratio(fg, bg) >= AA_TEXT);
        }
    }
}

#[test]
fn dtcg_parses() {
    for src in sources() {
        let f = &files(&src, Target::Dtcg)[0];
        let v = json(&f.path, &f.contents);
        for mode in ["light", "dark"] {
            assert_eq!(
                v["color"][mode].as_object().unwrap().len(),
                yolo_emit::ROLES.len()
            );
        }
    }
}

#[test]
fn asset_catalog_colorsets_are_valid() {
    for src in sources() {
        let files = files(&src, Target::Swiftui);
        let swift = &files[0];
        assert!(swift.path.ends_with("Colors.swift"));
        let catalog = format!("{}.xcassets", src.pascal());
        let root = json(&files[1].path, &files[1].contents);
        assert_eq!(files[1].path, format!("{catalog}/Contents.json"));
        assert_eq!(root["info"]["version"], 1);
        let mut names = Vec::new();
        for f in &files[2..] {
            let name = f
                .path
                .strip_prefix(&format!("{catalog}/"))
                .and_then(|p| p.strip_suffix(".colorset/Contents.json"))
                .unwrap_or_else(|| panic!("bad colorset path {}", f.path));
            names.push(name.to_owned());
            let v = json(&f.path, &f.contents);
            assert_eq!(v["info"]["author"], "xcode");
            let colors = v["colors"].as_array().unwrap();
            assert_eq!(colors.len(), 4);
            let mut seen = BTreeSet::new();
            for c in colors {
                assert_eq!(c["idiom"], "universal");
                let comps = &c["color"]["components"];
                assert_eq!(c["color"]["color-space"], "srgb");
                for ch in ["red", "green", "blue"] {
                    let s = comps[ch].as_str().unwrap();
                    assert!(s.starts_with("0x") && u8::from_str_radix(&s[2..], 16).is_ok());
                }
                let key: Vec<String> = c
                    .get("appearances")
                    .and_then(|a| a.as_array())
                    .map(|a| {
                        a.iter()
                            .map(|x| {
                                format!(
                                    "{}={}",
                                    x["appearance"].as_str().unwrap(),
                                    x["value"].as_str().unwrap()
                                )
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                assert!(seen.insert(key));
            }
            assert!(swift.contents.contains(&format!("Color(\"{name}\")")));
        }
        assert_eq!(names.len(), yolo_tokens::role::Role::ALL.len() + 4);
        assert!(swift.contents.contains("NSAppearance.Name.darkAqua"));
        assert_eq!(
            swift.contents.matches('{').count(),
            swift.contents.matches('}').count()
        );
    }
}

// ---------------------------------------------------------------------------------------------
// XAML (quick-xml)
// ---------------------------------------------------------------------------------------------

#[derive(Default, Debug)]
struct Dict {
    colors: BTreeMap<String, String>,
    brushes: BTreeMap<String, String>,
    others: BTreeSet<String>,
}

fn parse_xaml(label: &str, text: &str) -> (Dict, BTreeMap<String, Dict>) {
    use quick_xml::Reader;
    use quick_xml::events::Event;
    let mut reader = Reader::from_str(text);
    let mut root = Dict::default();
    let mut themes: BTreeMap<String, Dict> = BTreeMap::new();
    let mut stack: Vec<String> = Vec::new();
    let mut current_theme: Option<String> = None;
    let mut pending_color: Option<String> = None;
    let mut root_seen = false;
    loop {
        let event = reader
            .read_event()
            .unwrap_or_else(|e| panic!("{label}: XML error: {e}"));
        match event {
            Event::Start(_) | Event::Empty(_) => {
                let (e, empty) = match &event {
                    Event::Start(e) => (e.clone(), false),
                    Event::Empty(e) => (e.clone(), true),
                    _ => unreachable!(),
                };
                let name = String::from_utf8(e.name().as_ref().to_vec()).unwrap();
                let mut attrs = BTreeMap::new();
                for a in e.attributes() {
                    let a = a.unwrap_or_else(|err| panic!("{label}: attribute error: {err}"));
                    attrs.insert(
                        String::from_utf8(a.key.as_ref().to_vec()).unwrap(),
                        a.unescape_value().unwrap().into_owned(),
                    );
                }
                if !root_seen {
                    assert_eq!(name, "ResourceDictionary", "{label}: root");
                    assert_eq!(attrs["xmlns"], winui::XMLNS);
                    assert_eq!(attrs["xmlns:x"], winui::XMLNS_X);
                    root_seen = true;
                }
                if name == "ResourceDictionary"
                    && stack.last().map(String::as_str)
                        == Some("ResourceDictionary.ThemeDictionaries")
                {
                    let key = attrs["x:Key"].clone();
                    assert!(
                        themes.insert(key.clone(), Dict::default()).is_none(),
                        "{label}: dup theme {key}"
                    );
                    current_theme = Some(key);
                }
                let dict = match &current_theme {
                    Some(k) => themes.get_mut(k).unwrap(),
                    None => &mut root,
                };
                if let Some(key) = attrs.get("x:Key") {
                    match name.as_str() {
                        "Color" => pending_color = Some(key.clone()),
                        "SolidColorBrush" => {
                            assert!(
                                dict.brushes
                                    .insert(key.clone(), attrs["Color"].clone())
                                    .is_none(),
                                "{label}: dup {key}"
                            );
                        }
                        "ResourceDictionary" => {}
                        _ => {
                            assert!(dict.others.insert(key.clone()), "{label}: dup {key}");
                        }
                    }
                }
                if !empty {
                    stack.push(name);
                }
            }
            Event::Text(t) => {
                if let Some(key) = pending_color.take() {
                    let value = t.unescape().unwrap().trim().to_owned();
                    let dict = match &current_theme {
                        Some(k) => themes.get_mut(k).unwrap(),
                        None => &mut root,
                    };
                    assert!(
                        value.len() == 9
                            && value.starts_with('#')
                            && value[1..].bytes().all(|b| b.is_ascii_hexdigit()),
                        "{label}: {key} = {value}"
                    );
                    assert!(
                        dict.colors.insert(key.clone(), value).is_none(),
                        "{label}: dup {key}"
                    );
                }
            }
            Event::End(e) => {
                let name = String::from_utf8(e.name().as_ref().to_vec()).unwrap();
                let open = stack.pop();
                assert_eq!(
                    open.as_deref(),
                    Some(name.as_str()),
                    "{label}: mismatched </{name}>"
                );
                if name == "ResourceDictionary"
                    && stack.last().map(String::as_str)
                        == Some("ResourceDictionary.ThemeDictionaries")
                {
                    current_theme = None;
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    assert!(stack.is_empty(), "{label}: unclosed elements {stack:?}");
    (root, themes)
}

fn argb_of(xaml: &str) -> u32 {
    u32::from_str_radix(&xaml[1..], 16).unwrap() | 0xFF00_0000
}

#[test]
fn winui_xaml_parses_and_resolves() {
    for src in sources() {
        let f = &files(&src, Target::Winui)[0];
        let (root, themes) = parse_xaml(&f.path, &f.contents);
        assert_eq!(
            themes.keys().map(String::as_str).collect::<Vec<_>>(),
            ["Default", "HighContrast", "Light"]
        );
        assert!(root.colors.contains_key("SystemAccentColor"));
        assert!(root.others.contains("ControlCornerRadius"));
        let keys = winui::color_keys();
        for mode in ["Default", "Light"] {
            let d = &themes[mode];
            assert_eq!(d.colors.len(), keys.len(), "{mode}");
            for k in &keys {
                assert!(d.colors.contains_key(*k), "{mode}: {k}");
                let brush = &d.brushes[&format!("{k}Brush")];
                let target = brush
                    .strip_prefix("{StaticResource ")
                    .and_then(|b| b.strip_suffix('}'))
                    .unwrap();
                assert!(
                    d.colors.contains_key(target),
                    "{mode}: {k}Brush -> {target}"
                );
            }
            let fg = argb_of(&d.colors["TextFillColorPrimary"]);
            let bg = argb_of(&d.colors["SolidBackgroundFillColorBase"]);
            assert!(ratio(fg, bg) >= AA_TEXT, "{mode} text");
            let fg = argb_of(&d.colors["TextOnAccentFillColorPrimary"]);
            let bg = argb_of(&d.colors["AccentFillColorDefault"]);
            assert!(ratio(fg, bg) >= AA_TEXT, "{mode} on accent");
        }
        let hc = &themes["HighContrast"];
        let light = &themes["Light"];
        let light_brushes: BTreeSet<&String> = light
            .brushes
            .keys()
            .chain(light.others.iter().filter(|k| k.ends_with("Brush")))
            .collect();
        assert_eq!(hc.brushes.keys().collect::<BTreeSet<_>>(), light_brushes);
        for v in hc.brushes.values() {
            assert!(v.starts_with("{ThemeResource SystemColor"), "{v}");
        }
    }
}

// ---------------------------------------------------------------------------------------------
// RON (ron + serde): COSMIC ThemeBuilder mirror of cosmic-theme's schema
// ---------------------------------------------------------------------------------------------

mod cosmic_schema {
    use serde::Deserialize;

    #[derive(Deserialize, Debug, Clone, Copy)]
    #[serde(deny_unknown_fields)]
    pub struct Srgba {
        pub red: f32,
        pub green: f32,
        pub blue: f32,
        pub alpha: f32,
    }

    #[derive(Deserialize, Debug, Clone, Copy)]
    #[serde(deny_unknown_fields)]
    pub struct Srgb {
        pub red: f32,
        pub green: f32,
        pub blue: f32,
    }

    #[derive(Deserialize, Debug)]
    #[serde(deny_unknown_fields)]
    pub struct PaletteInner {
        pub name: String,
        pub bright_red: Srgba,
        pub bright_green: Srgba,
        pub bright_orange: Srgba,
        pub gray_1: Srgba,
        pub gray_2: Srgba,
        pub neutral_0: Srgba,
        pub neutral_1: Srgba,
        pub neutral_2: Srgba,
        pub neutral_3: Srgba,
        pub neutral_4: Srgba,
        pub neutral_5: Srgba,
        pub neutral_6: Srgba,
        pub neutral_7: Srgba,
        pub neutral_8: Srgba,
        pub neutral_9: Srgba,
        pub neutral_10: Srgba,
        pub accent_blue: Srgba,
        pub accent_indigo: Srgba,
        pub accent_purple: Srgba,
        pub accent_pink: Srgba,
        pub accent_red: Srgba,
        pub accent_orange: Srgba,
        pub accent_yellow: Srgba,
        pub accent_green: Srgba,
        pub accent_warm_grey: Srgba,
        pub ext_warm_grey: Srgba,
        pub ext_orange: Srgba,
        pub ext_yellow: Srgba,
        pub ext_blue: Srgba,
        pub ext_purple: Srgba,
        pub ext_pink: Srgba,
        pub ext_indigo: Srgba,
    }

    #[derive(Deserialize, Debug)]
    pub enum Palette {
        Dark(PaletteInner),
        Light(PaletteInner),
        HighContrastLight(PaletteInner),
        HighContrastDark(PaletteInner),
    }

    #[derive(Deserialize, Debug)]
    #[serde(deny_unknown_fields)]
    pub struct Spacing {
        pub space_none: u16,
        pub space_xxxs: u16,
        pub space_xxs: u16,
        pub space_xs: u16,
        pub space_s: u16,
        pub space_m: u16,
        pub space_l: u16,
        pub space_xl: u16,
        pub space_xxl: u16,
        pub space_xxxl: u16,
    }

    #[derive(Deserialize, Debug)]
    #[serde(deny_unknown_fields)]
    pub struct CornerRadii {
        pub radius_0: [f32; 4],
        pub radius_xs: [f32; 4],
        pub radius_s: [f32; 4],
        pub radius_m: [f32; 4],
        pub radius_l: [f32; 4],
        pub radius_xl: [f32; 4],
    }

    /// `cosmic_theme::ThemeBuilder` (config version 2) field set.
    #[derive(Deserialize, Debug)]
    #[serde(deny_unknown_fields)]
    pub struct ThemeBuilder {
        pub palette: Palette,
        pub spacing: Spacing,
        pub corner_radii: CornerRadii,
        pub neutral_tint: Option<Srgb>,
        pub bg_color: Option<Srgba>,
        pub primary_container_bg: Option<Srgba>,
        pub secondary_container_bg: Option<Srgba>,
        pub text_tint: Option<Srgb>,
        pub accent: Option<Srgb>,
        pub success: Option<Srgb>,
        pub warning: Option<Srgb>,
        pub destructive: Option<Srgb>,
        pub is_frosted: bool,
        pub gaps: (u32, u32),
        pub active_hint: u32,
        pub window_hint: Option<Srgb>,
    }
}

fn srgb_to_argb(r: f32, g: f32, b: f32) -> u32 {
    let ch = |v: f32| {
        assert!((0.0..=1.0).contains(&v));
        (v * 255.0).round() as u32
    };
    0xFF00_0000 | (ch(r) << 16) | (ch(g) << 8) | ch(b)
}

#[test]
fn cosmic_ron_matches_theme_builder_schema() {
    use cosmic_schema::{Palette, ThemeBuilder};
    for src in sources() {
        let files = files(&src, Target::Cosmic);
        assert_eq!(files.len(), 2);
        for (f, dark) in files.iter().zip([true, false]) {
            let b: ThemeBuilder =
                ron::from_str(&f.contents).unwrap_or_else(|e| panic!("{}: RON error: {e}", f.path));
            let inner = match (&b.palette, dark) {
                (Palette::Dark(p), true) | (Palette::Light(p), false) => p,
                _ => panic!("{}: wrong palette variant", f.path),
            };
            assert!(inner.name.starts_with(&src.slug));
            let bg = b.bg_color.unwrap();
            let text = b.text_tint.unwrap();
            let r = ratio(
                srgb_to_argb(text.red, text.green, text.blue),
                srgb_to_argb(bg.red, bg.green, bg.blue),
            );
            assert!(r >= AA_TEXT, "{}: text on bg {r:.2}", f.path);
            assert!((bg.alpha - 1.0).abs() < f32::EPSILON);
            assert!(b.spacing.space_xxxl > b.spacing.space_xxl);
            assert_eq!(b.corner_radii.radius_0, [0.0; 4]);
            assert!(b.accent.is_some() && b.destructive.is_some() && b.window_hint.is_some());
            let _ = (
                b.neutral_tint,
                b.primary_container_bg,
                b.secondary_container_bg,
                b.success,
                b.warning,
                b.is_frosted,
                b.gaps,
                b.active_hint,
            );
        }
    }
}

// ---------------------------------------------------------------------------------------------
// TOML (toml): Omarchy
// ---------------------------------------------------------------------------------------------

#[test]
fn omarchy_directories_are_complete() {
    for src in sources() {
        let files = files(&src, Target::Omarchy);
        assert_eq!(files.len(), 4);
        for dark in [true, false] {
            let dir = omarchy::dir_name(&src, dark);
            let colors = files
                .iter()
                .find(|f| f.path == format!("{dir}/colors.toml"))
                .unwrap();
            let icons = files
                .iter()
                .find(|f| f.path == format!("{dir}/icons.theme"))
                .unwrap();
            assert!(icons.contents.trim().starts_with("Yaru"));
            let table: toml::Table = colors
                .contents
                .parse()
                .unwrap_or_else(|e| panic!("{}: TOML error: {e}", colors.path));
            let keys: BTreeSet<&str> = table.keys().map(String::as_str).collect();
            assert_eq!(keys, omarchy::KEYS.into_iter().collect(), "{}", colors.path);
            assert_eq!(
                table["mode"].as_str(),
                Some(if dark { "dark" } else { "light" })
            );
            let get = |k: &str| table[k].as_str().unwrap().to_owned();
            for k in omarchy::KEYS {
                let v = get(k);
                // Omarchy's parser accepts this character set only (bin/omarchy-theme-color).
                assert!(
                    v.chars()
                        .all(|c| c.is_ascii_alphanumeric() || "#(),._+/% -".contains(c)),
                    "{k} = {v}"
                );
                if !matches!(
                    k,
                    "mode" | "hyprland_active_border" | "hyprland_inactive_border"
                ) {
                    assert!(is_hex6(&v), "{k} = {v}");
                }
            }
            let bg = parse_hex(&get("background"));
            for k in [
                "foreground",
                "dark_foreground",
                "bright_foreground",
                "red",
                "yellow",
                "green",
                "cyan",
                "blue",
                "magenta",
            ] {
                let r = ratio(parse_hex(&get(k)), bg);
                assert!(r >= AA_TEXT, "{}: {k} on background {r:.2}", colors.path);
            }
            let r = ratio(
                parse_hex(&get("selection_foreground")),
                parse_hex(&get("selection_background")),
            );
            assert!(r >= AA_TEXT, "{}: selection {r:.2}", colors.path);
            assert!(get("hyprland_active_border").ends_with("deg"));
        }
    }
    let sub: toml::Table = yolo_emit::emit_omarchy_subtheme(yolo_emit::OmarchySubTheme::TokyoNight)
        .parse()
        .unwrap();
    assert_eq!(sub["accent"].as_str(), Some("#7aa2f7"));
}

// ---------------------------------------------------------------------------------------------
// Kotlin (structural): Compose
// ---------------------------------------------------------------------------------------------

fn balanced(label: &str, text: &str) {
    let mut stack = Vec::new();
    let mut in_str = false;
    for ch in text.chars() {
        match ch {
            '"' => in_str = !in_str,
            _ if in_str => {}
            '(' | '{' | '[' => stack.push(ch),
            ')' | '}' | ']' => {
                let open = stack
                    .pop()
                    .unwrap_or_else(|| panic!("{label}: unmatched {ch}"));
                let expected = match ch {
                    ')' => '(',
                    '}' => '{',
                    _ => '[',
                };
                assert_eq!(open, expected, "{label}");
            }
            _ => {}
        }
    }
    assert!(stack.is_empty() && !in_str, "{label}: unbalanced");
}

#[test]
fn compose_schemes_set_every_role_once() {
    for src in sources() {
        let f = &files(&src, Target::Compose)[0];
        balanced(&f.path, &f.contents);
        let p = src.pascal();
        assert!(f.contents.starts_with("// Generated"));
        assert!(
            f.contents
                .contains(&format!("package {}", compose::PACKAGE))
        );
        assert!(f.contents.contains(&format!("fun {p}Theme(")));
        for (val, builder) in [
            ("LightColorScheme", "lightColorScheme"),
            ("DarkColorScheme", "darkColorScheme"),
        ] {
            let start = f
                .contents
                .find(&format!("val {p}{val}: ColorScheme = {builder}("))
                .unwrap();
            let body = &f.contents[start..];
            let body = &body[..body.find("\n)\n").unwrap()];
            let mut values = BTreeMap::new();
            for line in body.lines().skip(1) {
                let (name, value) = line.trim().trim_end_matches(',').split_once(" = ").unwrap();
                let hex = value
                    .strip_prefix("Color(0x")
                    .and_then(|v| v.strip_suffix(')'))
                    .unwrap();
                assert_eq!(hex.len(), 8);
                assert!(
                    values
                        .insert(name.to_owned(), u32::from_str_radix(hex, 16).unwrap())
                        .is_none()
                );
            }
            assert_eq!(values.len(), compose::PARAMS.len());
            for role in compose::PARAMS {
                assert!(values.contains_key(role.camel_name()));
            }
            assert!(ratio(values["onSurface"], values["surface"]) >= AA_TEXT);
            assert!(ratio(values["onPrimary"], values["primary"]) >= AA_TEXT);
        }
    }
}

// ---------------------------------------------------------------------------------------------
// WCAG AA on the main role pairs, every preset, both modes
// ---------------------------------------------------------------------------------------------

#[test]
fn every_preset_meets_wcag_aa_in_both_modes() {
    let mut report = Vec::new();
    for src in sources() {
        for dark in [false, true] {
            for f in failures(src.scheme(dark)) {
                report.push(format!(
                    "{} {}: {} on {} = {:.2}",
                    src.slug,
                    if dark { "dark" } else { "light" },
                    f.foreground,
                    f.background,
                    f.ratio
                ));
            }
        }
    }
    assert!(
        report.is_empty(),
        "WCAG AA failures:\n{}",
        report.join("\n")
    );
}

#[test]
fn single_file_emit_matches_primary_file() {
    let theme = Theme::default();
    let src = ThemeSource::from_theme(theme);
    for &t in Target::ALL {
        assert_eq!(emit(&theme, t), emit_files(&src, t)[0].contents);
    }
}

#[test]
fn registry_entries_are_complete() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../catalog/themes.registry.toml"
    );
    let text = std::fs::read_to_string(path).expect("catalog/themes.registry.toml");
    let doc: toml::Table = text.parse().expect("registry TOML");
    let meta = doc["meta"].as_table().unwrap();
    assert!(meta["checked"].as_str().unwrap().len() == 10);
    let entries = doc["source"].as_array().unwrap();
    assert!(entries.len() >= 20);
    let mut urls = BTreeSet::new();
    let mut platforms = BTreeSet::new();
    for e in entries {
        let e = e.as_table().unwrap();
        for k in ["name", "url", "license", "platform", "kind", "status"] {
            assert!(
                e.get(k)
                    .and_then(|v| v.as_str())
                    .is_some_and(|s| !s.is_empty()),
                "{e:?}: {k}"
            );
        }
        let url = e["url"].as_str().unwrap();
        assert!(url.starts_with("https://"), "{url}");
        assert!(urls.insert(url.to_owned()), "duplicate {url}");
        let status = e["status"].as_str().unwrap();
        assert!(status.starts_with('2'), "{url}: {status}");
        platforms.insert(e["platform"].as_str().unwrap().to_owned());
    }
    for p in [
        "omarchy", "material", "chrome", "gtk", "cosmic", "android", "windows", "apple",
    ] {
        assert!(platforms.contains(p), "no {p} entries");
    }
}
