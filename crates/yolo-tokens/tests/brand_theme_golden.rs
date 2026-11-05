// SPDX-License-Identifier: Apache-2.0
//! Golden check of the Aphrody brand theme exports.
//!
//! `tests/fixtures/brand-theme{,-high-contrast}.json` are the
//! [`ThemeExport`](yolo_tokens::theme::ThemeExport) of the identity pet seed at contrast 0
//! and 1. Consumers (the former `@aphrody/app-ui` brand CSS) copy them from here. Rewrite them
//! on the vps with `APHRODY_UPDATE_TOKENS=1 cargo test -p yolo-tokens --test brand_theme_golden`.

#![cfg(feature = "std")]

use std::{fs, path::PathBuf};

use serde_json::{Value, json};
use yolo_tokens::theme::ThemeRequest;

fn pet_seed() -> String {
    let fixture: Value = serde_json::from_str(include_str!("fixtures/mcu-pet.json")).unwrap();
    fixture["seed"].as_str().unwrap().to_owned()
}

fn export_json(contrast: f64) -> String {
    let export = ThemeRequest::parse(Some(&pet_seed()), Some("tonal-spot"), Some(contrast))
        .unwrap()
        .export();
    let value = json!({
        "schema_version": export.schema_version,
        "spec_version": export.spec_version,
        "seed": export.seed,
        "variant": export.variant,
        "contrast_level": export.contrast_level,
        "light": export.light,
        "dark": export.dark,
    });
    let mut text = serde_json::to_string_pretty(&value).unwrap();
    text.push('\n');
    text
}

fn check(relative: &str, expected: &str) {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(relative);
    if std::env::var_os("APHRODY_UPDATE_TOKENS").is_some() {
        fs::write(&path, expected).unwrap_or_else(|e| panic!("write {}: {e}", path.display()));
        return;
    }
    let actual =
        fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    assert!(
        actual == expected,
        "{relative} is stale; regenerate with APHRODY_UPDATE_TOKENS=1 (see this file's docs)"
    );
}

#[test]
fn brand_theme_exports_are_current() {
    check("brand-theme.json", &export_json(0.0));
    check("brand-theme-high-contrast.json", &export_json(1.0));
}
