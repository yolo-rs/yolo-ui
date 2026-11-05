//! Theme state of the interactive showcase: which pack is active, what the user changed on top of
//! it, and everything the page needs to repaint from that one value.
//!
//! The renderer never reads a pack, a role or a wallpaper on its own. It reads one
//! [`AppliedTheme`], computed from one [`ThemeChoice`], so a switch rewrites the stylesheet, the
//! `data-*` attributes, the window title and the wallpaper together, or not at all.
//! This module has no UI dependency and is tested without a browser.

use std::sync::OnceLock;

use base64::Engine as _;
use yolo_shared::YoloThemePreset;
use yolo_theme_pack::manifest::{ColorSpec, IconSpec, ShellSpec, Typography, parse_hex};
use yolo_theme_pack::wallpaper::generated_wallpaper_svg;
use yolo_theme_pack::{
    ActiveWallpaper, ColorSource, IconStyle, Mode, MotionProfile, ResolvedTheme, ShellLayout,
    ThemePack, Titles, UserPrefs, resolve,
};
use yolo_tokens::scheme::Variant;

use crate::gallery::short_name;

/// Packs shipped in the repository `themes/` directory, embedded at compile time.
const REPO_PACKS: [&str; 2] = [
    include_str!("../../../themes/aurora-terminal/theme.toml"),
    include_str!("../../../themes/paper-studio/theme.toml"),
];

/// First family of a CSS `font-family` list, without quotes.
fn first_family(list: &str) -> String {
    list.split(',')
        .next()
        .unwrap_or("system-ui")
        .trim()
        .trim_matches(['"', '\''])
        .to_owned()
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn px(value: f32) -> u16 {
    // Clamped to the u16 range first, so the cast cannot wrap.
    value.round().clamp(0.0, f32::from(u16::MAX)) as u16
}

/// A built-in preset expressed as a pack: its seed, radii, blur and font, and a shell layout that
/// matches its platform. Presets ship no wallpaper, so each gets the generated one.
#[must_use]
pub fn preset_pack(preset: YoloThemePreset) -> ThemePack {
    let style = preset.style();
    let name = short_name(preset);
    let font = first_family(style.font_family_ui);
    let layout = match preset {
        YoloThemePreset::Neutral | YoloThemePreset::M3 | YoloThemePreset::Fluent => {
            ShellLayout::Sidebar
        }
        YoloThemePreset::Apple => ShellLayout::Dock,
        YoloThemePreset::Aphrody => ShellLayout::Floating,
        YoloThemePreset::Omarchy | YoloThemePreset::Hyprland | YoloThemePreset::Chromium => {
            ShellLayout::TopBar
        }
    };
    ThemePack {
        id: preset.id().to_owned(),
        name: name.to_owned(),
        title: Titles {
            product_name: format!("Yolo UI {name}"),
            window_title_template: format!("{{page}} - Yolo UI {name}"),
            ntp_heading: format!("Yolo UI {name}"),
        },
        logo: None,
        color: ColorSpec {
            seed: preset.seed_hex().to_owned(),
            source: ColorSource::Seed,
            variant: Variant::TonalSpot.name().to_owned(),
            contrast: 0.0,
            mode: Mode::System,
            overrides: std::collections::BTreeMap::new(),
            secondary: None,
            neutral_chroma: None,
        },
        icons: IconSpec {
            set: "phosphor".to_owned(),
            style: IconStyle::Outlined,
            weight: 400,
        },
        wallpapers: Vec::new(),
        pet: None,
        typography: Typography {
            display: font.clone(),
            body: font,
            mono: first_family(style.font_family_mono),
        },
        shell: ShellSpec {
            layout,
            density: 0,
            radius: [
                px(style.radius_sm_px),
                px(style.radius_md_px),
                px(style.radius_lg_px),
            ],
            glass_blur: px(style.backdrop_blur_px),
            motion: MotionProfile::Calm,
        },
        assets: std::collections::BTreeMap::new(),
    }
}

/// Every pack the picker offers: the repository packs first, then the presets.
#[must_use]
pub fn catalog() -> &'static [ThemePack] {
    static CATALOG: OnceLock<Vec<ThemePack>> = OnceLock::new();
    CATALOG.get_or_init(|| {
        let mut packs: Vec<ThemePack> = REPO_PACKS
            .iter()
            .map(|text| ThemePack::from_toml(text).expect("embedded theme pack parses"))
            .collect();
        packs.extend(YoloThemePreset::ALL.map(preset_pack));
        packs
    })
}

/// The pack with this id.
#[must_use]
pub fn find_pack(id: &str) -> Option<&'static ThemePack> {
    catalog().iter().find(|pack| pack.id == id)
}

/// What the user picked. `None` keeps the value of the active pack.
#[derive(Clone, Debug, PartialEq)]
pub struct ThemeChoice {
    /// Id of the active pack.
    pub pack_id: String,
    /// Custom seed `#RRGGBB`.
    pub seed: Option<String>,
    /// Dynamic scheme variant name.
    pub variant: Option<String>,
    /// Contrast level, -1.0..=1.0.
    pub contrast: Option<f64>,
    /// Light, dark or follow the system.
    pub mode: Option<Mode>,
    /// Repair `on-*` roles below AA instead of only reporting them.
    pub clamp_text: bool,
}

impl Default for ThemeChoice {
    fn default() -> Self {
        Self {
            pack_id: catalog()[0].id.clone(),
            seed: None,
            variant: None,
            contrast: None,
            mode: None,
            clamp_text: true,
        }
    }
}

impl ThemeChoice {
    /// Activates another pack. Seed, variant and contrast go back to the pack's own, because they
    /// were tuned for the previous pack; mode and the text clamp are the user's and stay.
    pub fn select_pack(&mut self, id: &str) {
        if find_pack(id).is_some() {
            id.clone_into(&mut self.pack_id);
            self.seed = None;
            self.variant = None;
            self.contrast = None;
        }
    }

    /// The active pack, or the first one when the id is unknown.
    #[must_use]
    pub fn pack(&self) -> &'static ThemePack {
        find_pack(&self.pack_id).unwrap_or(&catalog()[0])
    }

    /// The preferences handed to the resolver.
    #[must_use]
    pub fn prefs(&self) -> UserPrefs {
        UserPrefs {
            seed: self.seed.clone().filter(|seed| parse_hex(seed).is_some()),
            variant: self.variant.clone(),
            contrast: self.contrast,
            mode: self.mode,
            clamp_text: self.clamp_text,
            ..UserPrefs::default()
        }
    }
}

/// Everything a render needs, derived from one [`ThemeChoice`].
#[derive(Clone, Debug, PartialEq)]
pub struct AppliedTheme {
    /// The resolved colours, shape and contrast report.
    pub theme: ResolvedTheme,
    /// Stylesheet: custom properties, wallpaper and the rules that read them.
    pub css: String,
    /// `data-*` attributes for the app root.
    pub attributes: [(&'static str, String); 4],
    /// Wallpaper as a `data:` URI, usable in a CSS `url()`.
    pub wallpaper_uri: String,
}

/// SVG wallpaper as a base64 `data:` URI. A pack file wallpaper cannot be fetched from a static
/// page, so it is replaced by the wallpaper generated from the same scheme.
fn wallpaper_uri(theme: &ResolvedTheme) -> String {
    let svg = match &theme.wallpaper {
        ActiveWallpaper::Generated { svg, .. } => svg.clone(),
        ActiveWallpaper::File { .. } => {
            generated_wallpaper_svg(&theme.roles, parse_hex(&theme.seed).unwrap_or(0xFF67_50A4))
        }
    };
    format!(
        "data:image/svg+xml;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(svg)
    )
}

impl AppliedTheme {
    /// Resolves `choice`. `system_dark` is the OS colour scheme, used when the mode is `System`.
    #[must_use]
    pub fn new(choice: &ThemeChoice, system_dark: bool) -> Self {
        let theme = resolve(choice.pack(), &choice.prefs(), system_dark);
        let uri = wallpaper_uri(&theme);
        let css = format!("{}{}", theme.css(Some(&uri)), shell_css());
        let attributes = theme.attributes();
        Self {
            theme,
            css,
            attributes,
            wallpaper_uri: uri,
        }
    }

    /// Window title for `page`.
    #[must_use]
    pub fn title(&self, page: &str) -> String {
        self.theme.title.window_title(page)
    }

    /// Value of a `data-*` attribute.
    #[must_use]
    pub fn attribute(&self, name: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find(|(key, _)| *key == name)
            .map(|(_, value)| value.as_str())
    }

    /// One line describing the contrast state, shown with the active column.
    #[must_use]
    pub fn summary(&self) -> String {
        format!(
            "{}, {}, {} text pairs below AA",
            self.theme.title.product_name,
            if self.theme.dark { "dark" } else { "light" },
            self.theme.contrast_warnings.len(),
        )
    }
}

/// Stylesheet rules that read the `--yolo-*` custom properties and the `data-shell` attribute.
/// The component rules themselves come from the gallery stylesheet; this layer repaints its page
/// chrome and maps the `p-active` scope onto the resolved roles.
#[must_use]
pub fn shell_css() -> &'static str {
    concat!(
        ".p-active{--bg:var(--yolo-surface);--surface:var(--yolo-surface-container-low);",
        "--elevated:var(--yolo-surface-container-high);--text:var(--yolo-on-surface);",
        "--muted:var(--yolo-on-surface-variant);--accent:var(--yolo-primary);--accent-2:var(--yolo-secondary);",
        "--on-accent:var(--yolo-on-primary);--rule:var(--yolo-outline-variant);--r-sm:var(--yolo-radius-sm);",
        "--r-md:var(--yolo-radius-md);--r-lg:var(--yolo-radius-lg);--r-pill:9999px;--grid:var(--yolo-space);",
        "--bw:1px;--font:var(--yolo-font-body),system-ui,sans-serif;",
        "background:var(--bg);color:var(--text);font-family:var(--font)}\n",
        "body{background:var(--yolo-background);color:var(--yolo-on-background);font-family:var(--yolo-font-body),system-ui,sans-serif}\n",
        ":focus-visible{outline-color:var(--yolo-focus-ring)}\n",
        ".app{min-height:100vh;display:grid;background:var(--yolo-background) var(--yolo-wallpaper) center/cover fixed;",
        "color:var(--yolo-on-background);gap:var(--yolo-space)}\n",
        ".app nav.primary{background:var(--yolo-surface-container);color:var(--yolo-on-surface);",
        "border:1px solid var(--yolo-outline-variant);border-radius:var(--yolo-radius-md);",
        "padding:var(--yolo-space);display:flex;gap:var(--yolo-space);align-items:center;flex-wrap:wrap}\n",
        ".app nav.primary a{color:inherit;padding:8px 12px;min-height:44px;display:inline-flex;align-items:center;",
        "border-radius:var(--yolo-radius-sm)}\n",
        ".app nav.primary a[aria-current=page]{background:var(--yolo-secondary-container);color:var(--yolo-on-secondary-container)}\n",
        ".app nav.primary .brand{font-weight:600;margin-inline-end:auto}\n",
        ".app .page{background:var(--yolo-surface);color:var(--yolo-on-surface);",
        "border:1px solid var(--yolo-outline-variant);border-radius:var(--yolo-radius-lg);min-width:0;overflow:auto}\n",
        ".app .panel{padding:calc(var(--yolo-space) * 2)}\n",
        ".app .panel h1{margin:0 0 8px;font-size:26px;font-family:var(--yolo-font-display),system-ui,sans-serif}\n",
        ".app .panel p{color:var(--yolo-on-surface-variant)}\n",
        ".app[data-shell=sidebar]{grid-template-columns:240px minmax(0,1fr);grid-template-rows:1fr;padding:var(--yolo-space)}\n",
        ".app[data-shell=sidebar] nav.primary{flex-direction:column;align-items:stretch;align-self:start}\n",
        ".app[data-shell=top-bar]{grid-template-rows:auto 1fr;padding:0 var(--yolo-space) var(--yolo-space)}\n",
        ".app[data-shell=top-bar] nav.primary{border-radius:0 0 var(--yolo-radius-md) var(--yolo-radius-md)}\n",
        ".app[data-shell=dock]{grid-template-rows:1fr auto;padding:var(--yolo-space)}\n",
        ".app[data-shell=dock] nav.primary{order:2;justify-self:center;border-radius:var(--yolo-radius-lg);",
        "backdrop-filter:blur(var(--yolo-glass-blur))}\n",
        ".app[data-shell=floating]{grid-template-rows:auto 1fr;padding:calc(var(--yolo-space) * 2)}\n",
        ".app[data-shell=floating] nav.primary{justify-self:center;border-radius:9999px;",
        "backdrop-filter:blur(var(--yolo-glass-blur))}\n",
        ".controls{display:grid;grid-template-columns:repeat(auto-fit,minmax(260px,1fr));gap:calc(var(--yolo-space) * 2)}\n",
        ".controls fieldset,.controls .control{border:1px solid var(--yolo-outline-variant);",
        "border-radius:var(--yolo-radius-md);padding:var(--yolo-space) calc(var(--yolo-space) * 2);margin:0;",
        "background:var(--yolo-surface-container-low)}\n",
        ".controls legend,.controls label.caption{font-weight:600;color:var(--yolo-on-surface)}\n",
        ".controls .opt,.controls .row{display:flex;align-items:center;gap:8px;min-height:44px}\n",
        ".controls select,.controls input[type=color]{min-height:44px;min-width:44px;font:inherit;",
        "background:var(--yolo-surface-container-high);color:var(--yolo-on-surface);",
        "border:1px solid var(--yolo-outline);border-radius:var(--yolo-radius-sm);padding:0 8px}\n",
        ".controls .reset{min-height:44px;padding:0 16px;font:inherit;cursor:pointer;",
        "background:transparent;color:var(--yolo-primary);border:1px solid var(--yolo-outline);",
        "border-radius:var(--yolo-radius-sm)}\n",
        ".controls .value{font-variant-numeric:tabular-nums;color:var(--yolo-on-surface-variant)}\n",
        ".warnings{margin:var(--yolo-space);padding:var(--yolo-space) calc(var(--yolo-space) * 2);",
        "border:1px solid var(--yolo-outline-variant);border-radius:var(--yolo-radius-md);",
        "background:var(--yolo-surface-container);color:var(--yolo-on-surface)}\n",
        ".warnings h2{font-size:16px;margin:0 0 4px}\n",
        ".warnings ul{margin:4px 0 0;padding-inline-start:20px}\n",
        ".warnings .ok{margin:0}\n",
        ".wallpaper-chip{display:block;width:100%;height:96px;border-radius:var(--yolo-radius-md);",
        "border:1px solid var(--yolo-outline-variant);background:var(--yolo-wallpaper) center/cover}\n",
        ".swatches{display:grid;grid-template-columns:repeat(auto-fill,minmax(150px,1fr));gap:8px;list-style:none;padding:0;margin:0}\n",
        ".swatches li{padding:8px;border:1px solid var(--yolo-outline-variant);border-radius:var(--yolo-radius-sm);",
        "background:var(--yolo-surface-container-low);font-size:13px}\n",
        ".swatches .chip-color{display:block;height:24px;border-radius:var(--yolo-radius-sm);margin-bottom:4px;",
        "border:1px solid var(--yolo-outline-variant)}\n",
        "@media (min-width:900px){.matrix.has-active{grid-template-columns:minmax(200px,240px) repeat(9,minmax(0,1fr))}}\n",
        "@media (max-width:899px){.app[data-shell=sidebar]{grid-template-columns:1fr}}\n",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn choice(pack: &str) -> ThemeChoice {
        let mut choice = ThemeChoice::default();
        choice.select_pack(pack);
        choice
    }

    #[test]
    fn catalog_has_repo_packs_and_all_presets_with_unique_ids() {
        let ids: Vec<&str> = catalog().iter().map(|p| p.id.as_str()).collect();
        assert_eq!(ids.len(), 2 + YoloThemePreset::ALL.len());
        assert!(ids.contains(&"aurora-terminal") && ids.contains(&"paper-studio"));
        for preset in YoloThemePreset::ALL {
            assert!(ids.contains(&preset.id()), "{}", preset.id());
        }
        let mut sorted = ids.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), ids.len());
    }

    #[test]
    fn every_pack_resolves_in_both_modes() {
        for pack in catalog() {
            for mode in [Mode::Light, Mode::Dark] {
                let mut c = choice(&pack.id);
                c.mode = Some(mode);
                let applied = AppliedTheme::new(&c, false);
                assert_eq!(applied.theme.roles.len(), 50, "{}", pack.id);
                assert_eq!(applied.theme.dark, mode == Mode::Dark);
                assert!(applied.css.contains("--yolo-primary: #"));
            }
        }
    }

    #[test]
    fn switching_a_pack_changes_roles_wallpaper_attributes_and_title_together() {
        let mut c = choice("aurora-terminal");
        c.mode = Some(Mode::Light);
        let before = AppliedTheme::new(&c, false);
        c.select_pack("paper-studio");
        let after = AppliedTheme::new(&c, false);

        assert_ne!(before.theme.roles, after.theme.roles);
        assert_ne!(before.theme.role("primary"), after.theme.role("primary"));
        assert_ne!(before.wallpaper_uri, after.wallpaper_uri);
        assert_ne!(before.css, after.css);
        assert_eq!(before.attribute("data-theme-pack"), Some("aurora-terminal"));
        assert_eq!(after.attribute("data-theme-pack"), Some("paper-studio"));
        assert_eq!(before.attribute("data-shell"), Some("dock"));
        assert_eq!(after.attribute("data-shell"), Some("sidebar"));
        assert_ne!(before.title("Themes"), after.title("Themes"));
        assert_eq!(after.title("Themes"), "Themes - Yolo Studio");
        // The stylesheet carries the new roles, the new wallpaper and the new pack together.
        let primary = after.theme.role("primary").expect("primary role");
        assert!(after.css.contains(&format!("--yolo-primary: {primary};")));
        assert!(after.css.contains(&after.wallpaper_uri));
        assert!(!after.css.contains(&before.wallpaper_uri));
        assert!(after.css.contains("paper-studio"));
    }

    #[test]
    fn switching_between_presets_changes_every_derived_value() {
        let mut c = choice("yolo-m3");
        c.mode = Some(Mode::Dark);
        let a = AppliedTheme::new(&c, true);
        c.select_pack("yolo-hyprland");
        let b = AppliedTheme::new(&c, true);
        assert_ne!(a.theme.roles, b.theme.roles);
        assert_ne!(a.wallpaper_uri, b.wallpaper_uri);
        assert_ne!(a.attributes, b.attributes);
        assert_eq!(b.attribute("data-shell"), Some("top-bar"));
        assert_eq!(b.theme.seed, "#33ccff");
    }

    #[test]
    fn user_seed_recolours_and_a_pack_switch_resets_it() {
        let mut c = choice("paper-studio");
        let base = AppliedTheme::new(&c, false);
        c.seed = Some("#e0a526".to_owned());
        let custom = AppliedTheme::new(&c, false);
        assert_eq!(custom.theme.seed, "#e0a526");
        assert_ne!(base.theme.role("primary"), custom.theme.role("primary"));
        assert_ne!(base.wallpaper_uri, custom.wallpaper_uri);
        c.select_pack("aurora-terminal");
        assert_eq!(c.seed, None);
        assert_eq!(c.pack_id, "aurora-terminal");
    }

    #[test]
    fn invalid_seed_and_unknown_pack_are_ignored() {
        let mut c = choice("paper-studio");
        c.seed = Some("not a colour".to_owned());
        assert_eq!(AppliedTheme::new(&c, false).theme.seed, "#b4532a");
        c.select_pack("no-such-pack");
        assert_eq!(c.pack_id, "paper-studio");
    }

    #[test]
    fn mode_system_follows_the_os_and_explicit_mode_does_not() {
        let mut c = choice("yolo-neutral");
        c.mode = Some(Mode::System);
        assert!(AppliedTheme::new(&c, true).theme.dark);
        assert!(!AppliedTheme::new(&c, false).theme.dark);
        c.mode = Some(Mode::Light);
        assert!(!AppliedTheme::new(&c, true).theme.dark);
        c.mode = Some(Mode::Dark);
        assert!(AppliedTheme::new(&c, false).theme.dark);
    }

    #[test]
    fn clamp_text_leaves_no_contrast_warning() {
        for pack in catalog() {
            for dark in [false, true] {
                for contrast in [-1.0, 0.0, 1.0] {
                    let mut c = choice(&pack.id);
                    c.clamp_text = true;
                    c.contrast = Some(contrast);
                    c.mode = Some(if dark { Mode::Dark } else { Mode::Light });
                    let applied = AppliedTheme::new(&c, dark);
                    assert!(
                        applied.theme.contrast_warnings.is_empty(),
                        "{} dark={dark} contrast={contrast}: {:?}",
                        pack.id,
                        applied.theme.contrast_warnings
                    );
                    assert!(applied.summary().contains("below AA"));
                }
            }
        }
    }

    #[test]
    fn wallpaper_is_a_data_uri_of_the_generated_svg() {
        let applied = AppliedTheme::new(&choice("yolo-apple"), false);
        let payload = applied
            .wallpaper_uri
            .strip_prefix("data:image/svg+xml;base64,")
            .expect("data uri");
        let svg = base64::engine::general_purpose::STANDARD
            .decode(payload)
            .expect("base64");
        assert!(svg.starts_with(b"<svg"));
        assert!(
            applied
                .css
                .contains("--yolo-wallpaper: url(\"data:image/svg+xml;base64,")
        );
    }

    #[test]
    fn stylesheet_selectors_match_the_shell_attribute_values() {
        let css = AppliedTheme::new(&choice("yolo-fluent"), false).css;
        assert!(!css.contains("</style"));
        for layout in ["sidebar", "top-bar", "dock", "floating"] {
            assert!(css.contains(&format!("[data-shell={layout}]")), "{layout}");
        }
    }
}
