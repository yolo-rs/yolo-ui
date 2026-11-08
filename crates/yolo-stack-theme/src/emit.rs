//! Emitters: CSS custom properties, DTCG JSON and a typed struct for native consumers.

use std::fmt::Write as _;

use serde_json::{Map, Value, json};
use yolo_theme_pack::manifest::parse_hex;

use crate::compute::ResolvedBundle;
use crate::gate::Roles;

/// Roles remapped to system colours under `forced-colors: active`.
const FORCED: &[(&str, &str)] = &[
    ("background", "Canvas"),
    ("surface", "Canvas"),
    ("surface-dim", "Canvas"),
    ("surface-bright", "Canvas"),
    ("surface-container-lowest", "Canvas"),
    ("surface-container-low", "Canvas"),
    ("surface-container", "Canvas"),
    ("surface-container-high", "Canvas"),
    ("surface-container-highest", "Canvas"),
    ("on-background", "CanvasText"),
    ("on-surface", "CanvasText"),
    ("on-surface-variant", "CanvasText"),
    ("outline", "CanvasText"),
    ("outline-variant", "GrayText"),
    ("primary", "Highlight"),
    ("on-primary", "HighlightText"),
    ("primary-container", "Canvas"),
    ("on-primary-container", "CanvasText"),
    ("secondary", "LinkText"),
    ("on-secondary", "Canvas"),
    ("secondary-container", "Canvas"),
    ("on-secondary-container", "CanvasText"),
    ("tertiary", "LinkText"),
    ("on-tertiary", "Canvas"),
    ("error", "Mark"),
    ("on-error", "MarkText"),
];

fn vars(out: &mut String, roles: &Roles, indent: &str) {
    for (name, value) in roles {
        let _ = writeln!(out, "{indent}--yolo-{name}: {value};");
    }
}

fn wallpaper_tiers(theme: &ResolvedBundle) -> Vec<u32> {
    let mut tiers: Vec<u32> = theme
        .assets
        .wallpapers
        .iter()
        .filter(|s| Some(&s.id) == theme.wallpaper_set.as_ref())
        .flat_map(|s| s.variants.iter().map(|v| v.min_width))
        .collect();
    tiers.push(0);
    tiers.sort_unstable();
    tiers.dedup();
    tiers
}

fn wallpaper_vars(theme: &ResolvedBundle, tier: u32) -> String {
    let mut out = String::new();
    let set = theme
        .assets
        .wallpapers
        .iter()
        .find(|s| Some(&s.id) == theme.wallpaper_set.as_ref());
    for (suffix, dark) in [("light", false), ("dark", true)] {
        let picked = set.and_then(|s| s.pick(dark, tier));
        let url = picked.map_or_else(
            || "none".to_owned(),
            |v| format!("url(\"{}\")", v.asset.url),
        );
        let dominant = picked.map_or("transparent", |v| v.dominant.as_str());
        let _ = writeln!(out, "  --yolo-wallpaper-{suffix}: {url};");
        let _ = writeln!(out, "  --yolo-wallpaper-dominant-{suffix}: {dominant};");
    }
    out
}

/// CSS for one bundle, scoped by `:root[data-theme="<id>"]`.
///
/// Light is the base; dark applies under `prefers-color-scheme: dark` unless the user forced
/// light, or under `data-scheme="dark"`. A bundle whose default mode is dark inverts the base.
/// `forced-colors: active` maps the roles to system colours, and reduced motion zeroes the
/// durations, either from the OS or from `data-motion="reduce"`.
#[must_use]
pub fn css(theme: &ResolvedBundle) -> String {
    let root = format!(":root[data-theme=\"{}\"]", theme.id);
    let default_dark = matches!(theme.scheme, crate::SchemePref::Dark);
    let (base, other) = if default_dark {
        (&theme.dark, &theme.light)
    } else {
        (&theme.light, &theme.dark)
    };
    let (base_name, other_name) = if default_dark {
        ("dark", "light")
    } else {
        ("light", "dark")
    };
    let mut out = String::with_capacity(8192);
    let _ = writeln!(
        out,
        "/* {} (seed {}, {}, contrast {}) */",
        theme.id, theme.seed, theme.variant, theme.contrast
    );

    let _ = writeln!(out, "{root} {{");
    let _ = writeln!(
        out,
        "  color-scheme: {};",
        match theme.scheme {
            crate::SchemePref::System => "light dark",
            crate::SchemePref::Light => "light",
            crate::SchemePref::Dark => "dark",
        }
    );
    vars(&mut out, base, "  ");
    let s = &theme.shape;
    let [sm, md, lg] = s.radius;
    let _ = writeln!(out, "  --yolo-space: {}px;", s.spacing);
    let _ = writeln!(
        out,
        "  --yolo-radius-sm: {sm}px;\n  --yolo-radius-md: {md}px;\n  --yolo-radius-lg: {lg}px;"
    );
    let _ = writeln!(out, "  --yolo-glass-blur: {}px;", s.glass_blur);
    let _ = writeln!(
        out,
        "  --yolo-motion-duration: {}ms;\n  --yolo-motion-easing: {};",
        s.motion_ms, s.motion_easing
    );
    let f = &theme.fonts;
    let _ = writeln!(
        out,
        "  --yolo-font-display: \"{}\", system-ui, sans-serif;\n  --yolo-font-header: \"{}\", system-ui, sans-serif;\n  --yolo-font-body: \"{}\", system-ui, sans-serif;\n  --yolo-font-mono: \"{}\", ui-monospace, monospace;",
        f.display, f.header, f.body, f.mono
    );
    let _ = writeln!(
        out,
        "  --yolo-wallpaper: var(--yolo-wallpaper-{base_name});"
    );
    let _ = writeln!(
        out,
        "  --yolo-wallpaper-dominant: var(--yolo-wallpaper-dominant-{base_name});"
    );
    out.push_str(&wallpaper_vars(theme, 0));
    out.push_str("}\n");

    for tier in wallpaper_tiers(theme).into_iter().filter(|t| *t > 0) {
        let _ = writeln!(
            out,
            "@media (min-width: {tier}px) {{\n{root} {{\n{}}}\n}}",
            wallpaper_vars(theme, tier)
        );
    }

    if theme.scheme == crate::SchemePref::System {
        let _ = writeln!(
            out,
            "@media (prefers-color-scheme: dark) {{\n{root}:not([data-scheme=\"light\"]) {{"
        );
        vars(&mut out, other, "    ");
        let _ = writeln!(
            out,
            "    --yolo-wallpaper: var(--yolo-wallpaper-{other_name});"
        );
        let _ = writeln!(
            out,
            "    --yolo-wallpaper-dominant: var(--yolo-wallpaper-dominant-{other_name});"
        );
        out.push_str("  }\n}\n");
    }
    for (scheme, roles) in [("light", &theme.light), ("dark", &theme.dark)] {
        let _ = writeln!(
            out,
            "{root}[data-scheme=\"{scheme}\"] {{\n  color-scheme: {scheme};"
        );
        vars(&mut out, roles, "  ");
        let _ = writeln!(out, "  --yolo-wallpaper: var(--yolo-wallpaper-{scheme});");
        let _ = writeln!(
            out,
            "  --yolo-wallpaper-dominant: var(--yolo-wallpaper-dominant-{scheme});"
        );
        out.push_str("}\n");
    }

    let _ = writeln!(
        out,
        "@media (forced-colors: active) {{\n{root}, {root}[data-scheme] {{"
    );
    for (role, system) in FORCED {
        let _ = writeln!(out, "  --yolo-{role}: {system};");
    }
    out.push_str("  --yolo-wallpaper: none;\n  --yolo-glass-blur: 0px;\n}\n}\n");

    let _ = writeln!(
        out,
        "@media (prefers-reduced-motion: reduce) {{\n{root} {{ --yolo-motion-duration: 0ms; }}\n}}\n{root}[data-motion=\"reduce\"] {{ --yolo-motion-duration: 0ms; }}"
    );
    out
}

/// DTCG (Design Tokens Community Group format) JSON for both schemes.
#[must_use]
pub fn dtcg(theme: &ResolvedBundle) -> Value {
    let group = |roles: &Roles| {
        let mut map = Map::new();
        for (name, value) in roles {
            map.insert(name.clone(), json!({ "$type": "color", "$value": value }));
        }
        Value::Object(map)
    };
    let px = |v: u16| json!({ "$type": "dimension", "$value": format!("{v}px") });
    let s = &theme.shape;
    json!({
        "$description": format!("Theme bundle {} computed by yolo-stack-theme", theme.id),
        "color": { "light": group(&theme.light), "dark": group(&theme.dark) },
        "radius": { "sm": px(s.radius[0]), "md": px(s.radius[1]), "lg": px(s.radius[2]) },
        "space": { "unit": px(s.spacing) },
        "glass": { "blur": px(s.glass_blur) },
        "motion": {
            "duration": { "$type": "duration", "$value": format!("{}ms", s.motion_ms) },
            "easing": { "$type": "cubicBezier", "$value": s.motion_easing }
        },
        "font": {
            "display": { "$type": "fontFamily", "$value": theme.fonts.display },
            "header": { "$type": "fontFamily", "$value": theme.fonts.header },
            "body": { "$type": "fontFamily", "$value": theme.fonts.body },
            "mono": { "$type": "fontFamily", "$value": theme.fonts.mono }
        },
        "$extensions": {
            "yolo.stack.theme": {
                "id": theme.id,
                "seed": theme.seed,
                "variant": theme.variant,
                "contrast": theme.contrast,
                "gatePasses": theme.gate.passes(),
                "adjustments": theme.gate.adjustments.len()
            }
        }
    })
}

/// 8-bit sRGB colour.
pub type Rgb = [u8; 3];

/// Typed theme for native consumers (Dioxus, wgpu, vello): no strings to parse at paint time.
#[derive(Clone, Debug, PartialEq)]
pub struct NativeTheme {
    /// Bundle id.
    pub id: String,
    /// Display title.
    pub title: String,
    /// Dark scheme.
    pub dark: bool,
    /// Colour roles.
    pub colors: Vec<(String, Rgb)>,
    /// Small, medium and large radii in px.
    pub radius: [u16; 3],
    /// Spacing unit in px.
    pub spacing: u16,
    /// Glass blur in px.
    pub glass_blur: u16,
    /// Motion duration in ms.
    pub motion_ms: u16,
    /// Fonts: display, header, body, mono.
    pub fonts: [String; 4],
}

impl NativeTheme {
    /// Colour of a role.
    #[must_use]
    pub fn color(&self, role: &str) -> Option<Rgb> {
        self.colors.iter().find(|(n, _)| n == role).map(|(_, c)| *c)
    }
}

impl ResolvedBundle {
    /// Typed theme for one scheme.
    #[must_use]
    pub fn native(&self, dark: bool) -> NativeTheme {
        let colors = self
            .roles(dark)
            .iter()
            .filter_map(|(name, hex)| {
                let argb = parse_hex(hex)?;
                Some((
                    name.clone(),
                    [(argb >> 16) as u8, (argb >> 8) as u8, argb as u8],
                ))
            })
            .collect();
        NativeTheme {
            id: self.id.clone(),
            title: self.title.product_name.clone(),
            dark,
            colors,
            radius: self.shape.radius,
            spacing: self.shape.spacing,
            glass_blur: self.shape.glass_blur,
            motion_ms: self.shape.motion_ms,
            fonts: [
                self.fonts.display.clone(),
                self.fonts.header.clone(),
                self.fonts.body.clone(),
                self.fonts.mono.clone(),
            ],
        }
    }
}
