// SPDX-License-Identifier: Apache-2.0
//! Feature parity matrix and migration report (JSON + Markdown).
//!
//! The matrix merges the config migration features with one entry per App Router convention,
//! `next/font`, `next/image` and Server Components capability found in the project. The parity
//! score weighs mapped features as 1, partial as 0.5 and unsupported as 0.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::types::{
    EmittedFile, FeatureStatus, FontSource, MigrationReport, ModuleKind, ProjectAnalysis,
    RouteKind, RouteSegment, Severity, Support, Target,
};

/// Computes the parity score (mapped = 1, partial = 0.5, unsupported = 0) as 0-100.
pub fn parity_score(features: &[FeatureStatus]) -> u8 {
    if features.is_empty() {
        return 100;
    }
    let mapped = features
        .iter()
        .filter(|f| f.support == Support::Mapped)
        .count();
    let partial = features
        .iter()
        .filter(|f| f.support == Support::Partial)
        .count();
    let score = 100.0 * (mapped as f64 + 0.5 * partial as f64) / features.len() as f64;
    score.round().clamp(0.0, 100.0) as u8
}

/// Collects the special-file conventions present anywhere in the segment tree.
fn collect_conventions(segment: &RouteSegment, out: &mut BTreeSet<&'static str>) {
    let files = &segment.files;
    let present: [(&'static str, bool); 11] = [
        ("layout", files.layout.is_some()),
        ("page", files.page.is_some()),
        ("loading", files.loading.is_some()),
        ("error", files.error.is_some()),
        ("not-found", files.not_found.is_some()),
        ("global-error", files.global_error.is_some()),
        ("template", files.template.is_some()),
        ("default", files.default.is_some()),
        ("route", files.route.is_some()),
        (
            "forbidden",
            files.forbidden.is_some() || files.unauthorized.is_some(),
        ),
        ("metadata", !files.metadata.is_empty()),
    ];
    for (name, is_present) in present {
        if is_present {
            out.insert(name);
        }
    }
    for child in &segment.children {
        collect_conventions(child, out);
    }
}

struct Matrix {
    features: Vec<FeatureStatus>,
}

impl Matrix {
    fn push(&mut self, feature: &str, support: Support, note: &str, target: Option<Target>) {
        if self
            .features
            .iter()
            .any(|f| f.feature == feature && f.target == target)
        {
            return;
        }
        self.features.push(FeatureStatus {
            feature: feature.to_owned(),
            support,
            note: note.to_owned(),
            target,
        });
    }

    fn mapped(&mut self, feature: &str, present: bool, note: &str) {
        if present {
            self.push(feature, Support::Mapped, note, None);
        }
    }

    fn partial(&mut self, feature: &str, present: bool, note: &str) {
        if present {
            self.push(feature, Support::Partial, note, None);
        }
    }
}

fn route_features(analysis: &ProjectAnalysis, matrix: &mut Matrix) {
    let routes = &analysis.routes;
    let mut conventions = BTreeSet::new();
    if let Some(tree) = &routes.tree {
        collect_conventions(tree, &mut conventions);
    }
    if routes.root_layout.is_some() {
        conventions.insert("layout");
    }
    if routes.root_not_found.is_some() {
        conventions.insert("not-found");
    }
    if routes.global_error.is_some() {
        conventions.insert("global-error");
    }
    let entries = &routes.entries;
    if entries
        .iter()
        .any(|e| e.kind == RouteKind::Page && !e.legacy_pages)
    {
        conventions.insert("page");
    }
    if entries
        .iter()
        .any(|e| e.kind == RouteKind::RouteHandler && !e.legacy_pages)
    {
        conventions.insert("route");
    }
    let has = |name: &str| conventions.contains(name);
    matrix.mapped(
        "app/layout.tsx",
        has("layout"),
        "nested layouts -> router outlet chain",
    );
    matrix.mapped("app/page.tsx", has("page"), "route component");
    matrix.mapped(
        "app/loading.tsx",
        has("loading"),
        "Suspense fallback / pending component",
    );
    matrix.mapped("app/error.tsx", has("error"), "error boundary");
    matrix.mapped("app/not-found.tsx", has("not-found"), "404 boundary");
    matrix.mapped(
        "app/global-error.tsx",
        has("global-error"),
        "root error boundary",
    );
    matrix.mapped("app/template.tsx", has("template"), "remounting layout");
    matrix.mapped("app/default.tsx", has("default"), "parallel slot fallback");
    matrix.mapped(
        "route handlers",
        has("route"),
        "GET/POST/... exports -> Bun.serve routes",
    );
    matrix.partial(
        "forbidden / unauthorized",
        has("forbidden"),
        "403/401 boundaries (authInterrupts)",
    );

    let parallel = entries
        .iter()
        .any(|e| e.slot.is_some() || !e.parallel_slots.is_empty());
    matrix.mapped(
        "parallel routes (@slot)",
        parallel,
        "slots rendered as layout props",
    );
    if entries.iter().any(|e| e.intercepting.is_some()) {
        matrix.push(
            "intercepting routes",
            Support::Mapped,
            "soft navigation renders the interceptor, hard navigation the target",
            Some(Target::Bun),
        );
        matrix.push(
            "intercepting routes",
            Support::Partial,
            "rendered as the target page (no interception)",
            Some(Target::Dioxus),
        );
        matrix.push(
            "intercepting routes",
            Support::Partial,
            "rendered as the target page (no interception)",
            Some(Target::Leptos),
        );
    }
    matrix.mapped(
        "generateStaticParams",
        entries
            .iter()
            .any(|e| e.segment_config.has_generate_static_params),
        "prerender list at build time",
    );
    matrix.mapped(
        "generateMetadata",
        entries
            .iter()
            .any(|e| e.segment_config.has_generate_metadata),
        "async head tags",
    );
    let metadata_files = has("metadata")
        || !routes.static_metadata.is_empty()
        || entries.iter().any(|e| e.kind == RouteKind::Metadata);
    matrix.mapped(
        "metadata files",
        metadata_files,
        "icon/opengraph/sitemap/robots/manifest routes",
    );
    matrix.partial(
        "proxy.ts",
        analysis.proxy.is_some(),
        "wrapped for Bun.serve; Node runtime semantics",
    );
    matrix.partial(
        "instrumentation",
        routes.instrumentation.is_some() || routes.instrumentation_client.is_some(),
        "register() called at server start; onRequestError mapped to the error hook",
    );
    matrix.partial(
        "pages router",
        routes.pages_dir.is_some() || entries.iter().any(|e| e.legacy_pages),
        "legacy pages/ served as SSR pages; getServerSideProps mapped to loaders",
    );
}

fn asset_features(analysis: &ProjectAnalysis, matrix: &mut Matrix) {
    let google = analysis
        .fonts
        .iter()
        .any(|f| matches!(f.source, FontSource::Google { .. }));
    let local = analysis
        .fonts
        .iter()
        .any(|f| matches!(f.source, FontSource::Local { .. }));
    matrix.mapped(
        "next/font/google",
        google,
        "self-hosted CSS2 fetch + size-adjust fallback",
    );
    matrix.mapped("next/font/local", local, "@font-face + fallback metrics");
    matrix.mapped(
        "next/image",
        !analysis.images.usages.is_empty(),
        "/_yolo/image optimizer with srcset algorithm",
    );
    matrix.partial(
        "next/image remote hosts",
        !analysis.images.unmatched_remote.is_empty(),
        "remote src not covered by images.remotePatterns",
    );
}

fn rsc_features(analysis: &ProjectAnalysis, matrix: &mut Matrix) {
    let modules = &analysis.graph.modules;
    let client = modules.iter().any(|m| m.kind == ModuleKind::Client);
    let server_actions = modules.iter().any(|m| m.kind == ModuleKind::ServerAction)
        || !analysis.graph.server_manifest.is_empty();
    let inline = modules.iter().any(|m| !m.inline_server_actions.is_empty());
    let cache = modules.iter().any(|m| !m.cache_functions.is_empty());
    let remote_cache = modules
        .iter()
        .flat_map(|m| m.cache_functions.iter())
        .any(|c| c.variant == "remote");
    matrix.mapped(
        "\"use client\"",
        client,
        "client reference manifest + hydration boundary",
    );
    matrix.mapped(
        "\"use server\"",
        server_actions,
        "Server Actions RPC (Next-Action id)",
    );
    matrix.mapped(
        "inline server actions",
        inline,
        "hoisted and registered by id",
    );
    matrix.partial(
        "\"use cache\"",
        cache,
        "in-memory + bun:sqlite cache handler",
    );
    matrix.partial(
        "\"use cache: remote\"",
        remote_cache,
        "remote handler stubbed to the sqlite handler",
    );
    matrix.partial(
        "cacheComponents / PPR",
        analysis.config.cache_components,
        "static shell + streamed dynamic holes",
    );
}

/// Builds the feature matrix from an analysis.
pub fn feature_matrix(analysis: &ProjectAnalysis) -> Vec<FeatureStatus> {
    let mut matrix = Matrix { features: Vec::new() };
    for feature in &analysis.config_report.features {
        matrix.push(
            &feature.feature,
            feature.support,
            &feature.note,
            feature.target,
        );
    }
    route_features(analysis, &mut matrix);
    asset_features(analysis, &mut matrix);
    rsc_features(analysis, &mut matrix);
    matrix.features
}

/// Proleptic Gregorian civil date from days since 1970-01-01 (Howard Hinnant's algorithm).
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let year = i64::try_from(yoe).unwrap_or(i64::MAX) + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if month <= 2 { year + 1 } else { year }, month, day)
}

/// ISO-8601 UTC timestamp (`2026-11-05T12:34:56Z`) for seconds since the Unix epoch.
fn iso8601_utc(secs: u64) -> String {
    let days = i64::try_from(secs / 86_400).unwrap_or(i64::MAX);
    let rem = secs % 86_400;
    let (year, month, day) = civil_from_days(days);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        rem / 3600,
        (rem % 3600) / 60,
        rem % 60
    )
}

fn now_iso8601() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    iso8601_utc(secs)
}

/// Builds the final report.
pub fn build_report(
    analysis: &ProjectAnalysis,
    targets: &[Target],
    files: &[EmittedFile],
) -> MigrationReport {
    let features = feature_matrix(analysis);
    let parity = parity_score(&features);
    let entries = &analysis.routes.entries;
    let count_kind = |kind: RouteKind| entries.iter().filter(|e| e.kind == kind).count() as u64;
    let mut counts = BTreeMap::new();
    counts.insert(String::from("routes"), entries.len() as u64);
    counts.insert(String::from("pages"), count_kind(RouteKind::Page));
    counts.insert(
        String::from("route_handlers"),
        count_kind(RouteKind::RouteHandler),
    );
    counts.insert(
        String::from("metadata_routes"),
        count_kind(RouteKind::Metadata),
    );
    counts.insert(
        String::from("client_components"),
        analysis
            .graph
            .modules
            .iter()
            .filter(|m| m.kind == ModuleKind::Client)
            .count() as u64,
    );
    counts.insert(
        String::from("server_actions"),
        analysis.graph.server_manifest.len() as u64,
    );
    counts.insert(
        String::from("cache_functions"),
        analysis
            .graph
            .modules
            .iter()
            .map(|m| m.cache_functions.len() as u64)
            .sum(),
    );
    counts.insert(String::from("fonts"), analysis.fonts.len() as u64);
    counts.insert(String::from("images"), analysis.images.usages.len() as u64);
    counts.insert(
        String::from("diagnostics_errors"),
        analysis
            .diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .count() as u64,
    );
    MigrationReport {
        project: analysis.name.clone(),
        generated_at: now_iso8601(),
        targets: targets.to_vec(),
        features,
        parity_score: parity,
        diagnostics: analysis.diagnostics.clone(),
        files_emitted: files.iter().map(|f| f.path.clone()).collect(),
        counts,
    }
}

fn cell(text: &str) -> String {
    text.replace('|', "\\|").replace('\n', " ")
}

fn support_label(support: Support) -> &'static str {
    match support {
        Support::Mapped => "mapped",
        Support::Partial => "partial",
        Support::Unsupported => "unsupported",
    }
}

/// Renders the report as Markdown.
pub fn report_markdown(report: &MigrationReport) -> String {
    let mut out = String::new();
    _ = write!(out, "# Migration report: {}\n\n", report.project);
    _ = write!(
        out,
        "Generated {} by yolo-next (Next.js {} baseline, Bun {}).\n\n",
        report.generated_at,
        crate::NEXT_VERSION_BASELINE,
        crate::BUN_VERSION_BASELINE
    );
    out.push_str("## Summary\n\n| Metric | Value |\n|---|---|\n");
    _ = writeln!(out, "| Parity score | {}% |", report.parity_score);
    let targets: Vec<&str> = report.targets.iter().map(|t| t.id()).collect();
    _ = writeln!(
        out,
        "| Targets | {} |",
        if targets.is_empty() {
            String::from("-")
        } else {
            targets.join(", ")
        }
    );
    for (name, value) in &report.counts {
        _ = writeln!(out, "| {} | {value} |", name.replace('_', " "));
    }
    let errors = report
        .diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .count();
    let warnings = report
        .diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Warning)
        .count();
    _ = writeln!(
        out,
        "| Diagnostics | {} ({errors} errors, {warnings} warnings) |",
        report.diagnostics.len()
    );
    _ = write!(
        out,
        "| Emitted files | {} |\n\n",
        report.files_emitted.len()
    );

    out.push_str("## Features\n\n| Feature | Support | Note | Target |\n|---|---|---|---|\n");
    for feature in &report.features {
        _ = writeln!(
            out,
            "| {} | {} | {} | {} |",
            cell(&feature.feature),
            support_label(feature.support),
            cell(&feature.note),
            feature.target.map_or("all", Target::id)
        );
    }
    if report.features.is_empty() {
        out.push_str("| (none) | - | - | - |\n");
    }

    out.push_str("\n## Diagnostics\n");
    for (severity, title) in [
        (Severity::Error, "Errors"),
        (Severity::Warning, "Warnings"),
        (Severity::Info, "Info"),
    ] {
        _ = write!(out, "\n### {title}\n\n");
        let mut any = false;
        for diagnostic in report.diagnostics.iter().filter(|d| d.severity == severity) {
            any = true;
            _ = write!(out, "- `{}` {}", diagnostic.code, diagnostic.message);
            if let Some(file) = &diagnostic.file {
                _ = write!(out, " ({file})");
            }
            if let Some(hint) = &diagnostic.hint {
                _ = write!(out, " -- {hint}");
            }
            out.push('\n');
        }
        if !any {
            out.push_str("- none\n");
        }
    }

    out.push_str("\n## Emitted files\n\n");
    for path in &report.files_emitted {
        _ = writeln!(out, "- `{path}`");
    }
    if report.files_emitted.is_empty() {
        out.push_str("- none\n");
    }
    out
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::types::{
        CacheFunction, CdnPlan, ConfigMigrationReport, Diagnostic, FontRequest, ImageUsageReport,
        InterceptSpec, ModuleGraph, ModuleInfo, NextConfig, ProxySpec, RouteEntry, RouteManifest,
        SegmentConfig, SegmentFiles, SegmentKind, YoloWebConfig,
    };

    fn feature(support: Support) -> FeatureStatus {
        FeatureStatus {
            feature: format!("{support:?}"),
            support,
            note: String::new(),
            target: None,
        }
    }

    fn entry(kind: RouteKind, file: &str) -> RouteEntry {
        RouteEntry {
            app_path: String::from("/"),
            url_pattern: String::from("/"),
            bun_pattern: String::from("/"),
            tanstack_path: String::from("/"),
            dioxus_route: String::from("/"),
            dioxus_variant: String::from("Index"),
            leptos_path: String::from("/"),
            params: Vec::new(),
            kind,
            file: file.to_owned(),
            layouts: Vec::new(),
            templates: Vec::new(),
            loading: None,
            error: None,
            not_found: None,
            parallel_slots: Vec::new(),
            slot: None,
            intercepting: None,
            segment_config: SegmentConfig::default(),
            methods: Vec::new(),
            metadata_kind: None,
            legacy_pages: false,
            legacy_data_functions: Vec::new(),
        }
    }

    #[allow(clippy::too_many_lines, reason = "test fixture literal")]
    fn analysis() -> ProjectAnalysis {
        let tree = RouteSegment {
            name: String::from("app"),
            kind: SegmentKind::Root,
            param: None,
            intercept_level: None,
            intercept_target: None,
            files: SegmentFiles {
                page: Some(String::from("app/page.tsx")),
                layout: Some(String::from("app/layout.tsx")),
                loading: Some(String::from("app/loading.tsx")),
                ..SegmentFiles::default()
            },
            children: vec![RouteSegment {
                name: String::from("api"),
                kind: SegmentKind::Static,
                param: None,
                intercept_level: None,
                intercept_target: None,
                files: SegmentFiles {
                    route: Some(String::from("app/api/route.ts")),
                    ..SegmentFiles::default()
                },
                children: Vec::new(),
            }],
        };
        let mut page = entry(RouteKind::Page, "app/page.tsx");
        page.segment_config.has_generate_metadata = true;
        let mut modal = entry(RouteKind::Page, "app/@modal/(.)photo/page.tsx");
        modal.slot = Some(String::from("modal"));
        modal.intercepting = Some(InterceptSpec { level: 0, target: String::from("/photo") });
        let config_report = ConfigMigrationReport {
            features: vec![FeatureStatus {
                feature: String::from("output"),
                support: Support::Mapped,
                note: String::from("Bun.serve"),
                target: None,
            }],
            diagnostics: Vec::new(),
        };
        ProjectAnalysis {
            root: String::from("/demo"),
            name: String::from("demo"),
            next_version: Some(String::from("16.4.5")),
            config_file: Some(String::from("next.config.ts")),
            config: NextConfig { cache_components: true, ..NextConfig::default() },
            yolo_config: YoloWebConfig::default(),
            config_report,
            routes: RouteManifest {
                tree: Some(tree),
                entries: vec![
                    page,
                    entry(RouteKind::RouteHandler, "app/api/route.ts"),
                    modal,
                ],
                proxy_file: Some(String::from("proxy.ts")),
                instrumentation: Some(String::from("instrumentation.ts")),
                ..RouteManifest::default()
            },
            fonts: vec![FontRequest {
                binding: String::from("inter"),
                import_name: String::from("Inter"),
                source: FontSource::Google { family: String::from("Inter") },
                subsets: vec![String::from("latin")],
                weights: Vec::new(),
                styles: Vec::new(),
                display: None,
                variable: None,
                preload: None,
                fallback: Vec::new(),
                adjust_font_fallback: None,
                axes: Vec::new(),
                declarations: Vec::new(),
                file: String::from("app/layout.tsx"),
            }],
            font_plans: Vec::new(),
            images: ImageUsageReport::default(),
            graph: ModuleGraph {
                modules: vec![
                    ModuleInfo {
                        id: String::from("app/counter.tsx"),
                        kind: ModuleKind::Client,
                        ..ModuleInfo::default()
                    },
                    ModuleInfo {
                        id: String::from("app/data.ts"),
                        kind: ModuleKind::Server,
                        cache_functions: vec![CacheFunction {
                            name: String::from("getPosts"),
                            life: None,
                            tags: Vec::new(),
                            variant: String::from("remote"),
                        }],
                        ..ModuleInfo::default()
                    },
                ],
                ..ModuleGraph::default()
            },
            proxy: Some(ProxySpec::default()),
            cdn: CdnPlan::default(),
            diagnostics: vec![
                Diagnostic::error("x-error", "bad | thing").with_file("a.ts"),
                Diagnostic::warning("x-warn", "careful").with_hint("do this"),
                Diagnostic::info("x-info", "fyi"),
            ],
        }
    }

    #[test]
    fn parity_math() {
        assert_eq!(parity_score(&[]), 100);
        assert_eq!(
            parity_score(&[
                feature(Support::Mapped),
                feature(Support::Partial),
                feature(Support::Unsupported)
            ]),
            50
        );
        assert_eq!(
            parity_score(&[
                feature(Support::Mapped),
                feature(Support::Mapped),
                feature(Support::Partial)
            ]),
            83
        );
        assert_eq!(parity_score(&[feature(Support::Unsupported)]), 0);
        assert_eq!(parity_score(&[feature(Support::Mapped)]), 100);
    }

    #[test]
    fn matrix_covers_conventions_assets_and_rsc() {
        let features = feature_matrix(&analysis());
        let find = |name: &str| features.iter().find(|f| f.feature == name);
        assert_eq!(find("output").unwrap().support, Support::Mapped);
        assert_eq!(find("app/layout.tsx").unwrap().support, Support::Mapped);
        assert_eq!(find("app/loading.tsx").unwrap().support, Support::Mapped);
        assert!(find("app/error.tsx").is_none());
        assert_eq!(find("route handlers").unwrap().support, Support::Mapped);
        assert_eq!(find("generateMetadata").unwrap().support, Support::Mapped);
        assert!(find("generateStaticParams").is_none());
        assert_eq!(
            find("parallel routes (@slot)").unwrap().support,
            Support::Mapped
        );
        let intercepting: Vec<&FeatureStatus> = features
            .iter()
            .filter(|f| f.feature == "intercepting routes")
            .collect();
        assert_eq!(intercepting.len(), 3);
        assert!(
            intercepting
                .iter()
                .any(|f| f.target == Some(Target::Dioxus) && f.support == Support::Partial)
        );
        assert!(
            intercepting
                .iter()
                .any(|f| f.target == Some(Target::Bun) && f.support == Support::Mapped)
        );
        assert_eq!(find("proxy.ts").unwrap().support, Support::Partial);
        assert_eq!(find("instrumentation").unwrap().support, Support::Partial);
        assert!(find("pages router").is_none());
        assert_eq!(find("next/font/google").unwrap().support, Support::Mapped);
        assert!(find("next/font/local").is_none());
        assert!(find("next/image").is_none());
        assert_eq!(find("\"use client\"").unwrap().support, Support::Mapped);
        assert!(find("\"use server\"").is_none());
        assert_eq!(find("\"use cache\"").unwrap().support, Support::Partial);
        assert_eq!(
            find("\"use cache: remote\"").unwrap().support,
            Support::Partial
        );
        assert_eq!(
            find("cacheComponents / PPR").unwrap().support,
            Support::Partial
        );
    }

    #[test]
    fn report_counts_and_markdown() {
        let a = analysis();
        let files =
            vec![EmittedFile { path: String::from("src/router.tsx"), contents: String::new() }];
        let report = build_report(&a, &[Target::Bun, Target::Leptos], &files);
        assert_eq!(report.project, "demo");
        assert_eq!(report.counts["routes"], 3);
        assert_eq!(report.counts["pages"], 2);
        assert_eq!(report.counts["route_handlers"], 1);
        assert_eq!(report.counts["metadata_routes"], 0);
        assert_eq!(report.counts["client_components"], 1);
        assert_eq!(report.counts["server_actions"], 0);
        assert_eq!(report.counts["cache_functions"], 1);
        assert_eq!(report.counts["fonts"], 1);
        assert_eq!(report.counts["images"], 0);
        assert_eq!(report.counts["diagnostics_errors"], 1);
        assert_eq!(report.files_emitted, vec![String::from("src/router.tsx")]);
        assert!(
            report.generated_at.ends_with('Z') && report.generated_at.len() == 20,
            "{}",
            report.generated_at
        );
        assert!(report.parity_score > 50);

        let markdown = report_markdown(&report);
        assert!(markdown.starts_with("# Migration report: demo\n"));
        assert!(markdown.contains("\n## Summary\n"));
        assert!(markdown.contains("| Parity score | "));
        assert!(markdown.contains("| Targets | bun, leptos |"));
        assert!(markdown.contains("\n## Features\n\n| Feature | Support | Note | Target |\n"));
        assert!(markdown.contains("| intercepting routes | partial | rendered as the target page (no interception) | dioxus |"));
        assert!(markdown.contains("\n## Diagnostics\n"));
        assert!(markdown.contains("\n### Errors\n\n- `x-error` bad | thing (a.ts)\n"));
        assert!(markdown.contains("\n### Warnings\n\n- `x-warn` careful -- do this\n"));
        assert!(markdown.contains("\n### Info\n\n- `x-info` fyi\n"));
        assert!(markdown.contains("\n## Emitted files\n\n- `src/router.tsx`\n"));
        assert!(!markdown.contains("\u{1F600}"));
    }

    #[test]
    fn civil_dates() {
        assert_eq!(iso8601_utc(0), "1970-01-01T00:00:00Z");
        assert_eq!(iso8601_utc(951_782_400), "2000-02-29T00:00:00Z");
        assert_eq!(iso8601_utc(1_762_300_800), "2025-11-05T00:00:00Z");
        assert_eq!(iso8601_utc(1_762_300_800 + 3_661), "2025-11-05T01:01:01Z");
        assert_eq!(civil_from_days(-1), (1969, 12, 31));
    }
}
