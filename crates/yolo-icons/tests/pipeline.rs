// SPDX-License-Identifier: Apache-2.0
//! End-to-end tests over the committed source subsets, the asset lock and the favicon outputs.

use resvg::tiny_skia::{Pixmap, Transform};
use resvg::usvg;
use std::path::{Path, PathBuf};
use yolo_icons::bundle::{build, decode_bundle, import_source};
use yolo_icons::chromium;
use yolo_icons::color::palette;
use yolo_icons::favicon::{FaviconOptions, ICO_SIZES, generate, verify_dir};
use yolo_icons::lock::{AssetLock, SourceFormat};
use yolo_icons::optimize::{OptimizeOptions, optimize};
use yolo_shared::YoloThemePreset;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repo root")
}

fn lock() -> AssetLock {
    AssetLock::load(&repo_root().join("catalog/assets.lock.toml")).expect("asset lock validates")
}

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("yolo-icons-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn render_alpha(svg: &[u8], side: u32) -> Vec<u8> {
    let tree = usvg::Tree::from_data(svg, &usvg::Options::default()).expect("svg parses");
    let mut pixmap = Pixmap::new(side, side).unwrap();
    // Same normalization as the importer: a non-square canvas is centered on the square grid.
    let (w, h) = (tree.size().width(), tree.size().height());
    let k = side as f32 / w.max(h);
    let ts = Transform::from_scale(k, k).pre_translate((w.max(h) - w) / 2.0, (w.max(h) - h) / 2.0);
    resvg::render(&tree, ts, &mut pixmap.as_mut());
    pixmap.data().chunks(4).map(|px| px[3]).collect()
}

/// Source document as the renderer sees it (Chromium `.icon` lowered with the importer's own lowering).
fn source_svg(format: SourceFormat, bytes: &[u8], grid: f64) -> Vec<u8> {
    match format {
        SourceFormat::Svg => bytes.to_vec(),
        SourceFormat::ChromiumIcon => {
            let reps = chromium::parse(std::str::from_utf8(bytes).unwrap()).unwrap();
            let rep = reps
                .iter()
                .min_by(|a, b| {
                    (f64::from(a.canvas) - grid)
                        .abs()
                        .total_cmp(&(f64::from(b.canvas) - grid).abs())
                        .then(b.canvas.cmp(&a.canvas))
                })
                .unwrap();
            chromium::to_svg(rep).0.into_bytes()
        }
    }
}

#[test]
fn lock_lists_only_allowed_sources_and_every_file_is_materialized() {
    let lock = lock();
    let root = repo_root();
    let ids: Vec<&str> = lock.by_priority().iter().map(|s| s.id.as_str()).collect();
    assert_eq!(
        ids,
        [
            "aphrody",
            "material-symbols",
            "fluent",
            "chromium",
            "cosmic"
        ]
    );
    for set in &lock.sets {
        if set.fetch != "local" {
            assert!(
                root.join(&set.dir).join("LICENSE").is_file(),
                "{}: LICENSE missing",
                set.id
            );
        }
        for name in set.icons.keys() {
            assert!(
                set.local_file(&root, name).is_file(),
                "{}:{name} not materialized",
                set.id
            );
        }
    }
}

#[test]
fn every_source_imports_optimizes_and_renders_like_the_original() {
    let lock = lock();
    let root = repo_root();
    let options = OptimizeOptions::default();
    let mut checked = 0;
    let mut failures = Vec::new();
    for set in &lock.sets {
        for name in set.icons.keys() {
            let bytes = std::fs::read(set.local_file(&root, name)).unwrap();
            let (mut icon, _) = import_source(name, &set.id, set.format, &bytes, options.grid)
                .unwrap_or_else(|e| panic!("{}:{name}: {e}", set.id));
            optimize(&mut icon, &options);
            assert!(!icon.layers.is_empty(), "{}:{name} is empty", set.id);
            let (min, max) = icon.control_bounds().unwrap();
            assert!(
                min.x >= -1.0 && min.y >= -1.0 && max.x <= 25.0 && max.y <= 25.0,
                "{}:{name} off-grid",
                set.id
            );
            let optimized = icon.to_svg(options.decimals);
            assert!(optimized.contains("fill=\"currentColor\""));

            // Pixel fidelity at 48 px: mean alpha error below 1.5% and no catastrophic pixel.
            let a = render_alpha(&source_svg(set.format, &bytes, options.grid), 48);
            let b = render_alpha(optimized.as_bytes(), 48);
            let total: u64 = a
                .iter()
                .zip(&b)
                .map(|(x, y)| u64::from(x.abs_diff(*y)))
                .sum();
            let mean = total as f64 / a.len() as f64 / 255.0;
            if mean >= 0.015 {
                failures.push(format!("{}:{name}: mean alpha error {mean:.4}", set.id));
            }
            checked += 1;
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
    assert!(checked >= 90, "only {checked} icons checked");
}

#[test]
fn build_writes_smaller_bundles_and_merges_by_priority() {
    let lock = lock();
    let out = temp_dir("build");
    let report = build(&repo_root(), &lock, &out, &OptimizeOptions::default()).unwrap();
    for set in &report.sets {
        assert!(
            set.missing.is_empty(),
            "{}: missing {:?}",
            set.id,
            set.missing
        );
        // Material Symbols ships already-minified single-path SVGs, so the per-icon SVG may stay close to the
        // source size; the compressed bundle must always be smaller than the sources it replaces.
        assert!(
            set.bundle_zstd_bytes < set.source_bytes,
            "{}: bundle {} >= source {}",
            set.id,
            set.bundle_zstd_bytes,
            set.source_bytes
        );
        assert!(set.bundle_zstd_bytes < set.bundle_json_bytes);
    }
    let source: u64 = report.sets.iter().map(|s| s.source_bytes).sum();
    let optimized: u64 = report.sets.iter().map(|s| s.optimized_bytes).sum();
    assert!(
        optimized < source,
        "optimized SVGs {optimized} >= sources {source}"
    );
    assert!(report.merged.bundle_zstd_bytes < report.merged.source_bytes);
    assert_eq!(report.merged_sources["aphrody_mark"], "aphrody");
    assert_eq!(report.merged_sources["home"], "material-symbols");
    assert_eq!(report.merged_sources["more_vert"], "chromium");

    let merged =
        decode_bundle(&std::fs::read(out.join("aphrody-symbols.icons.json.zst")).unwrap()).unwrap();
    assert_eq!(merged["id"], "aphrody-symbols");
    assert_eq!(
        merged["icons"].as_array().unwrap().len(),
        report.merged.icons
    );
    let rs = std::fs::read_to_string(out.join("aphrody_symbols.rs")).unwrap();
    assert!(rs.contains("pub enum AphrodySymbol") && rs.contains("ArrowBack,"));
    let ts = std::fs::read_to_string(out.join("aphrody-symbols.ts")).unwrap();
    assert!(ts.contains("export type AphrodySymbolName"));
    let _ = std::fs::remove_dir_all(out);
}

fn pixel(p: &Pixmap, x: u32, y: u32) -> [u8; 4] {
    let c = p.pixel(x, y).unwrap().demultiply();
    [c.red(), c.green(), c.blue(), c.alpha()]
}

fn hex(s: &str) -> [u8; 3] {
    let v = u32::from_str_radix(s.trim_start_matches('#'), 16).unwrap();
    [(v >> 16) as u8, (v >> 8) as u8, v as u8]
}

fn close(a: [u8; 4], rgb: [u8; 3]) -> bool {
    a[3] == 255 && a.iter().zip(rgb).all(|(x, y)| x.abs_diff(y) <= 2)
}

#[test]
fn favicon_for_every_preset_decodes_back() {
    let mark = yolo_icons::aphrody_mark(24).unwrap();
    for preset in [
        YoloThemePreset::Aphrody,
        YoloThemePreset::Chromium,
        YoloThemePreset::M3,
        YoloThemePreset::Fluent,
    ] {
        let out = temp_dir(preset.id());
        let options = FaviconOptions {
            name: "Aphrody".into(),
            short_name: "Aphrody".into(),
            light: palette(preset, false),
            dark: palette(preset, true),
            two_tone: false,
            start_url: "/".into(),
        };
        generate(&mark, &options, &out).unwrap();
        let verified = verify_dir(&out).unwrap();
        assert_eq!(verified.ico_entries.len(), ICO_SIZES.len());

        // Independent decoder: the `ico` crate reads our container and every PNG entry.
        let dir =
            ico::IconDir::read(std::fs::File::open(out.join("favicon.ico")).unwrap()).unwrap();
        let sizes: Vec<u32> = dir.entries().iter().map(|e| e.width()).collect();
        assert_eq!(sizes, ICO_SIZES.to_vec());
        for entry in dir.entries() {
            assert!(entry.is_png());
            let image = entry.decode().unwrap();
            assert_eq!(image.width(), entry.width());
        }

        let bg = hex(&options.light.background);
        let p32 = Pixmap::decode_png(&std::fs::read(out.join("favicon-32.png")).unwrap()).unwrap();
        assert_eq!(
            pixel(&p32, 0, 0)[3],
            0,
            "rounded badge corner must be transparent"
        );
        assert!(
            close(pixel(&p32, 16, 1), bg),
            "{}: badge color {:?}",
            preset.id(),
            pixel(&p32, 16, 1)
        );
        let m512 =
            Pixmap::decode_png(&std::fs::read(out.join("icon-maskable-512.png")).unwrap()).unwrap();
        assert!(
            close(pixel(&m512, 0, 0), bg),
            "maskable icon must be full bleed"
        );
        // The mark is drawn: some pixel of the 512 icon carries the foreground color.
        let fg = hex(&options.light.foreground);
        let any = Pixmap::decode_png(&std::fs::read(out.join("icon-512.png")).unwrap()).unwrap();
        let has_fg = (0..512)
            .step_by(4)
            .any(|y| (0..512).step_by(4).any(|x| close(pixel(&any, x, y), fg)));
        assert!(has_fg, "{}: mark not rendered", preset.id());
        let _ = std::fs::remove_dir_all(out);
    }
}

#[test]
fn committed_example_favicon_decodes() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/out");
    let report = verify_dir(&dir).expect("examples/out holds a valid favicon set");
    assert_eq!(report.pngs.len(), 8);
    assert_eq!(report.manifest_icons, 5);
}
