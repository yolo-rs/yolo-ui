// SPDX-License-Identifier: Apache-2.0
//! Orchestrator: runs every analysis stage over a project root and drives the emitters.

use std::fs;
use std::path::Path;

use serde_json::Value;

use crate::types::{
    Diagnostic, EmittedFile, ImageUsage, MigrationReport, NextConfig, ProjectAnalysis, RouteKind,
    Target,
};
use crate::{
    cdn, config, emit_bun, emit_dioxus, emit_leptos, fonts, images, proxy, report, routes, rsc,
};

/// Normalizes a path to `/` separators.
pub fn norm(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// Reads `package.json` facts (`name`, `dependencies.next`).
fn package_facts(root: &Path) -> (Option<String>, Option<String>) {
    let Ok(text) = fs::read_to_string(root.join("package.json")) else {
        return (None, None);
    };
    let Ok(value) = serde_json::from_str::<Value>(&text) else {
        return (None, None);
    };
    let name = value.get("name").and_then(Value::as_str).map(str::to_owned);
    let next_version = ["dependencies", "devDependencies"]
        .iter()
        .find_map(|k| {
            value
                .get(k)
                .and_then(|d| d.get("next"))
                .and_then(Value::as_str)
        })
        .map(str::to_owned);
    (name, next_version)
}

/// Source files (`.ts/.tsx/.js/.jsx/.mjs`) under the routable directories.
fn source_files(root: &Path, dirs: &[&str]) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for dir in dirs {
        let base = root.join(dir);
        if !base.is_dir() {
            continue;
        }
        for entry in walkdir::WalkDir::new(&base)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| e.file_type().is_file())
        {
            let path = entry.path();
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if !matches!(ext, "ts" | "tsx" | "js" | "jsx" | "mjs") {
                continue;
            }
            if let Ok(source) = fs::read_to_string(path) {
                let rel = path.strip_prefix(root).map_or_else(|_| norm(path), norm);
                out.push((rel, source));
            }
        }
    }
    out.sort();
    out
}

/// Runs every analysis stage.
#[allow(clippy::too_many_lines)] // one linear pipeline; splitting it would only scatter the stage order
pub fn analyze_project(root: &Path) -> ProjectAnalysis {
    let root_str = norm(root);
    let (pkg_name, next_version) = package_facts(root);
    let name = pkg_name.unwrap_or_else(|| {
        root.file_name().map_or_else(
            || String::from("next-app"),
            |n| n.to_string_lossy().into_owned(),
        )
    });

    let mut diagnostics: Vec<Diagnostic> = Vec::new();

    // next.config.*
    let config_file = config::find_next_config(root);
    let (next_config, config_diags) = match &config_file {
        Some(path) => match fs::read_to_string(path) {
            Ok(source) => {
                let file_name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                match config::parse_next_config(&source, &file_name) {
                    Ok(ok) => ok,
                    Err(errors) => (NextConfig::default(), errors),
                }
            }
            Err(err) => (
                NextConfig::default(),
                vec![Diagnostic::error(
                    "next-config-unreadable",
                    format!("cannot read next.config: {err}"),
                )],
            ),
        },
        None => (
            NextConfig::default(),
            vec![Diagnostic::info(
                "next-config-missing",
                "no next.config.* found; Next.js defaults assumed",
            )],
        ),
    };
    diagnostics.extend(config_diags);

    // Routes
    let route_manifest = routes::scan_project(root);
    diagnostics.extend(route_manifest.diagnostics.iter().cloned());
    let app_dir = route_manifest
        .app_dir
        .clone()
        .unwrap_or_else(|| String::from("app"));

    // Proxy
    let proxy_spec = proxy::find_proxy(root).and_then(|path| {
        let rel = path.strip_prefix(root).map_or_else(|_| norm(&path), norm);
        fs::read_to_string(&path)
            .ok()
            .map(|source| proxy::analyze_proxy(&source, &rel))
    });
    if let Some(spec) = &proxy_spec {
        diagnostics.extend(spec.diagnostics.iter().cloned());
    }

    // Normalized config
    let (yolo_config, config_report) = config::next_config_to_yolo(
        &next_config,
        &app_dir,
        route_manifest.pages_dir.as_deref(),
        proxy_spec.as_ref().map(|p| p.file.as_str()),
    );
    diagnostics.extend(config_report.diagnostics.iter().cloned());

    // Fonts + images over all source files
    let mut scan_dirs: Vec<&str> = vec![app_dir.as_str()];
    if let Some(pages) = route_manifest.pages_dir.as_deref() {
        scan_dirs.push(pages);
    }
    for extra in ["components", "src/components", "lib", "src/lib"] {
        if root.join(extra).is_dir() && !scan_dirs.contains(&extra) {
            scan_dirs.push(extra);
        }
    }
    let files = source_files(root, &scan_dirs);
    let mut font_requests = Vec::new();
    let mut image_usages: Vec<ImageUsage> = Vec::new();
    for (rel, source) in &files {
        font_requests.extend(fonts::extract_font_requests(source, rel));
        image_usages.extend(images::extract_image_usages(source, rel));
    }
    let fonts_route = yolo_config.cdn.fonts_route.clone();
    let font_plans: Vec<_> = font_requests
        .iter()
        .map(|request| fonts::plan_font(request, &fonts_route))
        .collect();
    let image_report = images::image_usage_report(image_usages, &next_config.images);
    diagnostics.extend(image_report.diagnostics.iter().cloned());

    // Module graph
    let graph = rsc::scan_module_graph(root, &scan_dirs);
    diagnostics.extend(graph.diagnostics.iter().cloned());

    let cdn_plan = cdn::cdn_plan(&next_config);

    ProjectAnalysis {
        root: root_str,
        name,
        next_version,
        config_file: config_file
            .as_ref()
            .map(|p| p.strip_prefix(root).map_or_else(|_| norm(p), norm)),
        config: next_config,
        yolo_config,
        config_report,
        routes: route_manifest,
        fonts: font_requests,
        font_plans,
        images: image_report,
        graph,
        proxy: proxy_spec,
        cdn: cdn_plan,
        diagnostics,
    }
}

/// Emits the selected targets.
pub fn emit(analysis: &ProjectAnalysis, targets: &[Target]) -> Vec<EmittedFile> {
    let mut files = Vec::new();
    for target in targets {
        let lane = match target {
            Target::Bun => emit_bun::emit_bun_project(analysis),
            Target::Dioxus => emit_dioxus::emit_dioxus_project(analysis),
            Target::Leptos => emit_leptos::emit_leptos_project(analysis),
        };
        let prefix = if targets.len() > 1 {
            format!("{}/", target.id())
        } else {
            String::new()
        };
        files.extend(
            lane.into_iter()
                .map(|f| EmittedFile { path: format!("{prefix}{}", f.path), contents: f.contents }),
        );
    }
    files
}

/// Analyzes, emits and reports in one call.
pub fn migrate(
    root: &Path,
    targets: &[Target],
) -> (ProjectAnalysis, Vec<EmittedFile>, MigrationReport) {
    let analysis = analyze_project(root);
    let mut files = emit(&analysis, targets);
    let report = report::build_report(&analysis, targets, &files);
    files.push(EmittedFile {
        path: String::from("MIGRATION-REPORT.md"),
        contents: report::report_markdown(&report),
    });
    files.push(EmittedFile {
        path: String::from("MIGRATION-REPORT.json"),
        contents: serde_json::to_string_pretty(&report).unwrap_or_default(),
    });
    (analysis, files, report)
}

/// Writes emitted files under `out_dir`, creating directories. Returns the number of files written.
pub fn write_emitted(out_dir: &Path, files: &[EmittedFile]) -> std::io::Result<usize> {
    for file in files {
        let target = out_dir.join(&file.path);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&target, &file.contents)?;
    }
    Ok(files.len())
}

/// Counts route entries by kind (used by the report and the CLI summary).
pub fn route_counts(analysis: &ProjectAnalysis) -> (usize, usize, usize) {
    let pages = analysis
        .routes
        .entries
        .iter()
        .filter(|e| e.kind == RouteKind::Page)
        .count();
    let handlers = analysis
        .routes
        .entries
        .iter()
        .filter(|e| e.kind == RouteKind::RouteHandler)
        .count();
    let metadata = analysis
        .routes
        .entries
        .iter()
        .filter(|e| e.kind == RouteKind::Metadata)
        .count();
    (pages, handlers, metadata)
}
