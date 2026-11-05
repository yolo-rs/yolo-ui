// SPDX-License-Identifier: Apache-2.0
//! Set import, merge ("Aphrody Symbols"), zstd bundles and typed Rust / TypeScript codegen.

use crate::Error;
use crate::chromium::import_icon;
use crate::ir::Icon;
use crate::lock::{AssetLock, AssetSet, SourceFormat};
use crate::optimize::{OptimizeOptions, optimize};
use crate::svg_import::import_svg;
use serde::Serialize;
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;

/// Name of the merged namespace.
pub const MERGED_SET: &str = "aphrody-symbols";

/// Imports one source file according to its format.
///
/// # Errors
/// Parse errors from the SVG or `.icon` importer.
pub fn import_source(
    name: &str,
    set: &str,
    format: SourceFormat,
    bytes: &[u8],
    grid: f64,
) -> Result<(Icon, Vec<String>), Error> {
    let (icon, notes) = match format {
        SourceFormat::Svg => import_svg(name, set, bytes, grid)?,
        SourceFormat::ChromiumIcon => {
            let text = std::str::from_utf8(bytes)
                .map_err(|e| Error::ChromiumIcon(format!("{name}: {e}")))?;
            import_icon(name, set, text, grid)?
        }
    };
    Ok((icon, notes.warnings))
}

/// One optimized icon inside a bundle.
#[derive(Debug, Clone, Serialize)]
pub struct BundledIcon {
    pub name: String,
    pub set: String,
    pub svg: String,
    pub monochrome: bool,
    pub flips_in_rtl: bool,
    pub source_bytes: u64,
}

/// A set (or the merged namespace) ready to serialize.
#[derive(Debug, Clone, Serialize)]
pub struct Bundle {
    pub id: String,
    pub license: String,
    pub attribution: String,
    pub grid: f64,
    pub icons: Vec<BundledIcon>,
}

/// Per-set size accounting.
#[derive(Debug, Clone, Serialize, Default)]
pub struct SetReport {
    pub id: String,
    pub icons: usize,
    pub missing: Vec<String>,
    pub warnings: Vec<String>,
    pub source_bytes: u64,
    pub optimized_bytes: u64,
    pub bundle_json_bytes: u64,
    pub bundle_zstd_bytes: u64,
}

/// Build report.
#[derive(Debug, Clone, Serialize)]
pub struct BuildReport {
    pub grid: f64,
    pub decimals: u32,
    pub sets: Vec<SetReport>,
    pub merged: SetReport,
    pub merged_sources: BTreeMap<String, String>,
}

/// Imports and optimizes every materialized icon of `set`.
///
/// Missing files are reported, not fatal: a set that was never fetched simply contributes nothing.
///
/// # Errors
/// Import errors for files that exist but fail to parse.
pub fn build_set(
    root: &Path,
    set: &AssetSet,
    options: &OptimizeOptions,
) -> Result<(Bundle, SetReport), Error> {
    let mut report = SetReport {
        id: set.id.clone(),
        ..SetReport::default()
    };
    let mut icons = Vec::new();
    for name in set.icons.keys() {
        let path = set.local_file(root, name);
        let Ok(bytes) = std::fs::read(&path) else {
            report.missing.push(name.clone());
            continue;
        };
        let (mut icon, warnings) = import_source(name, &set.id, set.format, &bytes, options.grid)?;
        report
            .warnings
            .extend(warnings.into_iter().map(|w| format!("{name}: {w}")));
        optimize(&mut icon, options);
        let svg = icon.to_svg(options.decimals);
        report.source_bytes += bytes.len() as u64;
        report.optimized_bytes += svg.len() as u64;
        icons.push(BundledIcon {
            name: name.clone(),
            set: set.id.clone(),
            monochrome: icon.is_monochrome(),
            flips_in_rtl: icon.flips_in_rtl,
            svg,
            source_bytes: bytes.len() as u64,
        });
    }
    report.icons = icons.len();
    let bundle = Bundle {
        id: set.id.clone(),
        license: set.license.clone(),
        attribution: set.attribution.clone(),
        grid: options.grid,
        icons,
    };
    Ok((bundle, report))
}

/// Merges set bundles by priority (Aphrody > Material > Fluent > Chromium > COSMIC by default).
#[must_use]
pub fn merge(bundles: &[(u32, Bundle)]) -> (Bundle, BTreeMap<String, String>) {
    let mut ordered: Vec<&(u32, Bundle)> = bundles.iter().collect();
    ordered.sort_by_key(|(p, b)| (*p, b.id.clone()));
    let mut chosen: BTreeMap<String, BundledIcon> = BTreeMap::new();
    for (_, bundle) in ordered {
        for icon in &bundle.icons {
            chosen
                .entry(icon.name.clone())
                .or_insert_with(|| icon.clone());
        }
    }
    let sources = chosen
        .iter()
        .map(|(k, v)| (k.clone(), v.set.clone()))
        .collect();
    let licenses: std::collections::BTreeSet<&str> = bundles
        .iter()
        .filter(|(_, b)| chosen.values().any(|i| i.set == b.id))
        .map(|(_, b)| b.license.as_str())
        .collect();
    let merged = Bundle {
        id: MERGED_SET.to_owned(),
        license: licenses.into_iter().collect::<Vec<_>>().join(" AND "),
        attribution: "Per-icon source set recorded in `set`; see catalog/assets.lock.toml."
            .to_owned(),
        grid: bundles.first().map_or(24.0, |(_, b)| b.grid),
        icons: chosen.into_values().collect(),
    };
    (merged, sources)
}

/// Serializes a bundle to JSON and zstd (level 19).
///
/// # Errors
/// [`Error::Io`] if compression fails.
pub fn encode_bundle(bundle: &Bundle) -> Result<(Vec<u8>, Vec<u8>), Error> {
    let json = serde_json::to_vec(bundle).map_err(|e| Error::Io(e.to_string()))?;
    let zst = zstd::bulk::compress(&json, 19).map_err(|e| Error::Io(format!("zstd: {e}")))?;
    Ok((json, zst))
}

/// Decodes a `.icons.json.zst` bundle back to JSON.
///
/// # Errors
/// [`Error::Io`] on corrupt input.
pub fn decode_bundle(zst: &[u8]) -> Result<serde_json::Value, Error> {
    let json = zstd::stream::decode_all(zst).map_err(|e| Error::Io(format!("zstd: {e}")))?;
    serde_json::from_slice(&json).map_err(|e| Error::Io(e.to_string()))
}

fn pascal(name: &str) -> String {
    let mut out = String::new();
    for part in name.split('_').filter(|p| !p.is_empty()) {
        let mut chars = part.chars();
        if let Some(first) = chars.next() {
            out.extend(first.to_uppercase());
            out.push_str(chars.as_str());
        }
    }
    if out.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        out.insert(0, 'I');
    }
    out
}

/// Generates a typed Rust module for the merged namespace.
#[must_use]
pub fn codegen_rust(bundle: &Bundle) -> String {
    let mut s = String::new();
    let _ = writeln!(s, "// @generated by `yolo-icons build`. Do not edit.");
    let _ = writeln!(
        s,
        "// Sources and licenses: catalog/assets.lock.toml ({}).",
        bundle.license
    );
    let _ = writeln!(s);
    let _ = writeln!(
        s,
        "/// Icons of the `{}` namespace on a {} unit grid.",
        bundle.id, bundle.grid
    );
    let _ = writeln!(
        s,
        "#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]"
    );
    let _ = writeln!(s, "pub enum AphrodySymbol {{");
    for icon in &bundle.icons {
        let _ = writeln!(s, "    {},", pascal(&icon.name));
    }
    let _ = writeln!(s, "}}");
    let _ = writeln!(s);
    let _ = writeln!(s, "impl AphrodySymbol {{");
    let _ = writeln!(s, "    pub const ALL: [Self; {}] = [", bundle.icons.len());
    for icon in &bundle.icons {
        let _ = writeln!(s, "        Self::{},", pascal(&icon.name));
    }
    let _ = writeln!(s, "    ];");
    for (fname, field) in [("name", 0), ("source_set", 1), ("svg", 2)] {
        let _ = writeln!(s);
        let _ = writeln!(s, "    #[must_use]");
        let _ = writeln!(s, "    pub const fn {fname}(self) -> &'static str {{");
        let _ = writeln!(s, "        match self {{");
        for icon in &bundle.icons {
            let value = match field {
                0 => icon.name.as_str(),
                1 => icon.set.as_str(),
                _ => icon.svg.as_str(),
            };
            let _ = writeln!(
                s,
                "            Self::{} => {:?},",
                pascal(&icon.name),
                value
            );
        }
        let _ = writeln!(s, "        }}");
        let _ = writeln!(s, "    }}");
    }
    let _ = writeln!(s, "}}");
    s
}

/// Generates a Bun/TypeScript module for the merged namespace.
#[must_use]
pub fn codegen_ts(bundle: &Bundle) -> String {
    let mut s = String::new();
    let _ = writeln!(s, "// @generated by `yolo-icons build`. Do not edit.");
    let _ = writeln!(
        s,
        "// Sources and licenses: catalog/assets.lock.toml ({}).",
        bundle.license
    );
    let _ = writeln!(s);
    let _ = writeln!(s, "export const APHRODY_SYMBOLS = {{");
    for icon in &bundle.icons {
        let entry = serde_json::json!({ "set": icon.set, "svg": icon.svg });
        let _ = writeln!(
            s,
            "  {}: {},",
            serde_json::to_string(&icon.name).expect("str"),
            entry
        );
    }
    let _ = writeln!(s, "}} as const;");
    let _ = writeln!(s);
    let _ = writeln!(
        s,
        "export type AphrodySymbolName = keyof typeof APHRODY_SYMBOLS;"
    );
    let _ = writeln!(s, "export const APHRODY_SYMBOL_GRID = {};", bundle.grid);
    s
}

fn write(path: &Path, bytes: &[u8]) -> Result<(), Error> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| Error::Io(format!("{}: {e}", parent.display())))?;
    }
    std::fs::write(path, bytes).map_err(|e| Error::Io(format!("{}: {e}", path.display())))
}

/// Builds every set of `lock` from the materialized sources under `root` into `out`:
/// `out/<set>/<name>.svg`, `out/<set>.icons.json.zst`, the merged bundle, `aphrody_symbols.rs`,
/// `aphrody-symbols.ts` and `report.json`.
///
/// # Errors
/// Import or I/O errors.
pub fn build(
    root: &Path,
    lock: &AssetLock,
    out: &Path,
    options: &OptimizeOptions,
) -> Result<BuildReport, Error> {
    let mut bundles = Vec::new();
    let mut reports = Vec::new();
    for set in lock.by_priority() {
        let (bundle, mut report) = build_set(root, set, options)?;
        for icon in &bundle.icons {
            write(
                &out.join(&set.id).join(format!("{}.svg", icon.name)),
                icon.svg.as_bytes(),
            )?;
        }
        let (json, zst) = encode_bundle(&bundle)?;
        report.bundle_json_bytes = json.len() as u64;
        report.bundle_zstd_bytes = zst.len() as u64;
        write(&out.join(format!("{}.icons.json.zst", set.id)), &zst)?;
        reports.push(report);
        bundles.push((set.priority, bundle));
    }
    let (merged, sources) = merge(&bundles);
    let (json, zst) = encode_bundle(&merged)?;
    write(&out.join(format!("{MERGED_SET}.icons.json.zst")), &zst)?;
    write(
        &out.join("aphrody_symbols.rs"),
        codegen_rust(&merged).as_bytes(),
    )?;
    write(
        &out.join("aphrody-symbols.ts"),
        codegen_ts(&merged).as_bytes(),
    )?;
    let merged_report = SetReport {
        id: MERGED_SET.to_owned(),
        icons: merged.icons.len(),
        source_bytes: merged.icons.iter().map(|i| i.source_bytes).sum(),
        optimized_bytes: merged.icons.iter().map(|i| i.svg.len() as u64).sum(),
        bundle_json_bytes: json.len() as u64,
        bundle_zstd_bytes: zst.len() as u64,
        ..SetReport::default()
    };
    let report = BuildReport {
        grid: options.grid,
        decimals: options.decimals,
        sets: reports,
        merged: merged_report,
        merged_sources: sources,
    };
    write(
        &out.join("report.json"),
        serde_json::to_string_pretty(&report)
            .map_err(|e| Error::Io(e.to_string()))?
            .as_bytes(),
    )?;
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pascal_case_names() {
        assert_eq!(pascal("arrow_back"), "ArrowBack");
        assert_eq!(pascal("aphrody_mark_48"), "AphrodyMark48");
        assert_eq!(pascal("3d_rotation"), "I3dRotation");
    }
}
