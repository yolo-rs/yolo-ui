//! Golden, gate and property tests over the real Shenron bundles.
//!
//! Regenerate goldens with `UPDATE_GOLDEN=1 cargo test -p yolo-stack-theme`.

use std::collections::BTreeMap;
use std::path::PathBuf;

use proptest::prelude::*;
use yolo_stack_theme::emit;
use yolo_stack_theme::{
    SchemePref, ThemeBundle, UserThemePrefs, WallpaperChoice, compute, gate::audit,
};
use yolo_tokens::{role::Role, scheme::Variant};

macro_rules! bundle {
    ($id:literal) => {
        ThemeBundle::from_strs(
            include_str!(concat!("../../../apps/shenron/themes/", $id, "/theme.toml")),
            include_str!(concat!(
                "../../../apps/shenron/themes/",
                $id,
                "/bundle.toml"
            )),
            include_str!(concat!(
                "../../../apps/shenron/themes/",
                $id,
                "/assets.lock.json"
            )),
        )
        .unwrap_or_else(|e| panic!("{}: {e}", $id))
    };
}

fn all() -> Vec<ThemeBundle> {
    vec![
        bundle!("shenron"),
        bundle!("kame"),
        bundle!("capsule"),
        bundle!("namek"),
    ]
}

fn golden(name: &str, actual: &str) {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden")
        .join(name);
    if std::env::var_os("UPDATE_GOLDEN").is_some() {
        std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
        std::fs::write(&path, actual).expect("write golden");
        return;
    }
    let expected = std::fs::read_to_string(&path)
        .unwrap_or_else(|_| panic!("missing golden {name}; run with UPDATE_GOLDEN=1"));
    assert_eq!(
        expected.replace("\r\n", "\n"),
        actual,
        "golden {name} differs"
    );
}

#[test]
fn every_shenron_bundle_loads_and_has_distinct_identity() {
    let bundles = all();
    let mut seeds = std::collections::BTreeSet::new();
    let mut titles = std::collections::BTreeSet::new();
    for b in &bundles {
        assert!(seeds.insert(b.pack.color.seed.clone()));
        assert!(titles.insert(b.pack.title.product_name.clone()));
        let t = compute(b, &UserThemePrefs::default());
        assert!(t.assets.logo.mark.is_some() && t.assets.logo.mask.is_some());
        assert!(t.assets.pet.is_some() && !t.assets.wallpapers.is_empty());
        assert!(
            t.assets.placeholders().len() >= 12,
            "placeholder art must be flagged"
        );
    }
}

#[test]
fn default_palettes_pass_the_gates_and_match_golden() {
    let mut snapshot = String::new();
    for b in all() {
        let t = compute(&b, &UserThemePrefs::default());
        assert!(t.gate.passes(), "{}: {:?}", t.id, t.gate.failures);
        snapshot.push_str(&format!(
            "{} seed {} adjustments {}\n",
            t.id,
            t.seed,
            t.gate.adjustments.len()
        ));
        for dark in [false, true] {
            for role in [
                "primary",
                "on-primary",
                "secondary",
                "tertiary",
                "surface",
                "on-surface",
                "outline",
            ] {
                snapshot.push_str(&format!(
                    "  {} {role} {}\n",
                    if dark { "dark" } else { "light" },
                    t.role(dark, role).expect("role")
                ));
            }
        }
    }
    golden("palettes.txt", &snapshot);
}

#[test]
fn contrast_audit_report() {
    let mut report = String::from("bundle scheme pair ratio min\n");
    for b in all() {
        let t = compute(&b, &UserThemePrefs::default());
        for (scheme, roles) in [("light", &t.light), ("dark", &t.dark)] {
            for r in audit(roles) {
                assert!(
                    r.pass,
                    "{} {scheme} {} on {} = {}",
                    t.id, r.fg, r.bg, r.ratio
                );
                report.push_str(&format!(
                    "{} {scheme} {}/{} {:.2} {:.1}\n",
                    t.id, r.fg, r.bg, r.ratio, r.min
                ));
            }
        }
    }
    golden("contrast-audit.txt", &report);
}

#[test]
fn css_has_scheme_forced_colors_and_motion_blocks_and_matches_golden() {
    let t = compute(&bundle!("shenron"), &UserThemePrefs::default());
    let css = emit::css(&t);
    assert!(css.contains(":root[data-theme=\"shenron\"] {"));
    assert!(css.contains("@media (prefers-color-scheme: dark)"));
    assert!(css.contains("[data-scheme=\"dark\"]"));
    assert!(css.contains("@media (forced-colors: active)"));
    assert!(css.contains("@media (prefers-reduced-motion: reduce)"));
    assert!(css.contains("--yolo-wallpaper: var(--yolo-wallpaper-light);"));
    assert!(css.contains("@media (min-width: 1280px)"));
    golden("shenron.css", &css);
}

#[test]
fn dtcg_and_native_agree_with_roles() {
    let t = compute(&bundle!("kame"), &UserThemePrefs::default());
    let json = emit::dtcg(&t);
    assert_eq!(json["color"]["dark"]["primary"]["$type"], "color");
    assert_eq!(
        json["color"]["light"]["primary"]["$value"],
        t.role(false, "primary").expect("role")
    );
    assert_eq!(json["$extensions"]["yolo.stack.theme"]["gatePasses"], true);
    let native = t.native(true);
    let [r, g, b] = native.color("primary").expect("primary");
    assert_eq!(
        format!("#{r:02x}{g:02x}{b:02x}"),
        t.role(true, "primary").expect("role")
    );
    assert_eq!(native.colors.len(), 50);
}

#[test]
fn assets_are_content_addressed_and_follow_viewport_and_scheme() {
    let t = compute(&bundle!("namek"), &UserThemePrefs::default());
    let small = t.assets.wallpaper_for(None, true, 800).expect("wallpaper");
    let large = t.assets.wallpaper_for(None, true, 1920).expect("wallpaper");
    assert_eq!((small.width, large.width), (1280, 2560));
    let light = t.assets.wallpaper_for(None, false, 800).expect("wallpaper");
    assert_ne!(small.asset.sha256, light.asset.sha256);
    assert!(small.asset.url.starts_with("/theme-assets/") && small.asset.url.ends_with(".svg"));
    assert_eq!(
        small.asset.url.rsplit('/').next().expect("name").len(),
        16 + ".svg".len()
    );
    assert_eq!(
        t.assets
            .wallpaper_for(Some("grid"), true, 800)
            .expect("grid")
            .asset
            .path,
        "wallpapers/grid-dark-1280.svg"
    );
}

#[test]
fn prefs_drive_pet_wallpaper_motion_and_scheme() {
    let b = bundle!("shenron");
    assert!(compute(&b, &UserThemePrefs::default()).pet_enabled);
    let off = UserThemePrefs { pet: false, ..UserThemePrefs::default() };
    assert!(!compute(&b, &off).pet_enabled);
    let reduced = UserThemePrefs { reduced_motion: true, ..UserThemePrefs::default() };
    let t = compute(&b, &reduced);
    assert!(!t.pet_enabled);
    assert_eq!(t.shape.motion_ms, 0);
    let none = UserThemePrefs { wallpaper: WallpaperChoice::Off, ..UserThemePrefs::default() };
    assert!(compute(&b, &none).wallpaper_set.is_none());
    let dark = UserThemePrefs { scheme: SchemePref::Dark, ..UserThemePrefs::default() };
    assert!(compute(&b, &dark).is_dark(false));
}

#[test]
fn hue_and_variant_change_colours_but_not_identity() {
    let b = bundle!("shenron");
    let base = compute(&b, &UserThemePrefs::default());
    let moved = compute(
        &b,
        &UserThemePrefs { hue: Some(200.0), ..UserThemePrefs::default() },
    );
    assert_ne!(base.role(false, "primary"), moved.role(false, "primary"));
    assert_eq!(base.assets.logo.mark, moved.assets.logo.mark);
    assert_eq!(base.title, moved.title);
}

#[test]
fn bad_override_is_adjusted_not_shipped() {
    let b = bundle!("capsule");
    let mut prefs = UserThemePrefs::default();
    // Light grey text on the surface: fails 4.5:1 and must be moved.
    prefs
        .overrides
        .insert("on-surface".into(), "#d0d0d0".into());
    let t = compute(&b, &prefs);
    assert!(t.gate.passes());
    assert!(t.overrides[0].adjusted);
    assert_ne!(t.role(false, "on-surface"), Some("#d0d0d0"));
}

#[test]
fn invalid_bundle_reports_all_problems_and_fallback_is_usable() {
    let theme = include_str!("../../../apps/shenron/themes/kame/theme.toml")
        .replace("seed = \"#1f6fb8\"", "seed = \"blue\"");
    let err = ThemeBundle::from_strs(
        &theme,
        include_str!("../../../apps/shenron/themes/kame/bundle.toml"),
        r#"{"schema":1,"files":{}}"#,
    )
    .expect_err("invalid");
    let text = err.to_string();
    assert!(
        text.contains("color.seed") && text.contains("missing from assets.lock.json"),
        "{text}"
    );
    let fallback = compute(&ThemeBundle::fallback(), &UserThemePrefs::default());
    assert!(fallback.gate.passes());
    assert!(fallback.generated_wallpaper.is_some());
}

fn variant_strategy() -> impl Strategy<Value = &'static str> {
    prop::sample::select(Variant::ALL.iter().map(|v| v.name()).collect::<Vec<_>>())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(96))]

    #[test]
    fn every_computed_pair_passes_the_gates(
        which in 0usize..4,
        hue in 0.0f64..360.0,
        chroma in 0.0f64..150.0,
        contrast in -1.0f64..=1.0,
        variant in variant_strategy(),
        scheme in prop::sample::select(vec![SchemePref::System, SchemePref::Light, SchemePref::Dark]),
        override_role in prop::sample::select(Role::ALL.iter().map(|r| r.name()).collect::<Vec<_>>()),
        override_rgb in 0u32..0x0100_0000,
        use_override in any::<bool>(),
    ) {
        let bundle = &all()[which];
        let mut overrides = BTreeMap::new();
        if use_override {
            overrides.insert(override_role.to_owned(), format!("#{override_rgb:06x}"));
        }
        let prefs = UserThemePrefs {
            hue: Some(hue),
            chroma: Some(chroma),
            contrast: Some(contrast),
            variant: Some(variant.to_owned()),
            scheme,
            overrides,
            ..UserThemePrefs::default()
        };
        let t = compute(bundle, &prefs);
        prop_assert!(t.gate.passes(), "{:?}", t.gate.failures);
        for roles in [&t.light, &t.dark] {
            for r in audit(roles) {
                prop_assert!(r.pass, "{} on {} = {} (min {})", r.fg, r.bg, r.ratio, r.min);
            }
        }
        prop_assert_eq!(t.light.len(), 50);
    }
}
