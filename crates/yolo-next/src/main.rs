// SPDX-License-Identifier: Apache-2.0
//! `yolo-next` CLI: analyze and migrate Next.js 16 projects to Bun + TanStack, Dioxus, or Leptos.

#![allow(clippy::print_stdout, clippy::print_stderr)]
// `run` is a flat subcommand dispatch table; prose docs name products (TanStack) not items.
#![allow(clippy::too_many_lines, clippy::doc_markdown)]

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use yolo_next::types::Target;
use yolo_next::{config, fonts, images, project, routes, rsc, rules};

const USAGE: &str = "usage: yolo-next <command> [options]

Commands:
  analyze <project> [--json]                   full analysis (config, routes, fonts, images, RSC graph)
  migrate <project> --out <dir> [--target bun|dioxus|leptos|all] [--dry-run]
  config <next.config.ts> [--toml|--ts|--json] evaluate next.config and print the Yolo config
  routes <project> [--format json|bun|tanstack|dioxus|leptos]
  match <project> <pathname>                    match a pathname against the App Router manifest
  fonts <file.tsx> [--json]                     next/font requests, CSS2 URLs and fallback metrics
  metrics <font.ttf|otf> [--fallback arial|times]  SFNT metrics and size-adjust overrides
  image-url --src <src> --width <w> [--quality <q>] [--loader default|cloudflare|cloudinary|imgix|akamai|fastly|supabase]
  rsc <project> [--json]                        Server Components module graph and Flight manifests
  rule <source> <pathname>                      test a redirects()/rewrites() source against a path
  version

Exit status: 0 success, 1 analysis errors, 2 usage or I/O error.";

fn flag<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .map(String::as_str)
}

fn has(args: &[String], name: &str) -> bool {
    args.iter().any(|a| a == name)
}

fn positional(args: &[String]) -> Vec<&str> {
    let mut out = Vec::new();
    let mut skip = false;
    for (i, a) in args.iter().enumerate() {
        if skip {
            skip = false;
            continue;
        }
        if a.starts_with("--") {
            let takes_value = matches!(
                a.as_str(),
                "--out"
                    | "--target"
                    | "--format"
                    | "--fallback"
                    | "--src"
                    | "--width"
                    | "--quality"
                    | "--loader"
            );
            if takes_value && args.get(i + 1).is_some() {
                skip = true;
            }
            continue;
        }
        out.push(a.as_str());
    }
    out
}

fn to_json<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|e| format!("{{\"error\":\"{e}\"}}"))
}

fn run(args: &[String]) -> Result<ExitCode, String> {
    let Some(command) = args.first().map(String::as_str) else {
        return Err(String::new());
    };
    let rest = &args[1..];
    let pos = positional(rest);
    match command {
        "version" | "--version" | "-V" => {
            println!(
                "yolo-next {} (next {} / bun {})",
                yolo_next::VERSION,
                yolo_next::NEXT_VERSION_BASELINE,
                yolo_next::BUN_VERSION_BASELINE
            );
            Ok(ExitCode::SUCCESS)
        }
        "analyze" => {
            let root = pos
                .first()
                .map(Path::new)
                .ok_or("analyze needs <project>")?;
            let analysis = project::analyze_project(root);
            if has(rest, "--json") {
                println!("{}", to_json(&analysis));
            } else {
                let (pages, handlers, metadata) = project::route_counts(&analysis);
                println!("project      {}", analysis.name);
                println!(
                    "next         {}",
                    analysis.next_version.as_deref().unwrap_or("?")
                );
                println!(
                    "config       {}",
                    analysis.config_file.as_deref().unwrap_or("-")
                );
                println!("pages        {pages}");
                println!("handlers     {handlers}");
                println!("metadata     {metadata}");
                println!("fonts        {}", analysis.fonts.len());
                println!("images       {}", analysis.images.usages.len());
                println!("client refs  {}", analysis.graph.client_manifest.len());
                println!("server refs  {}", analysis.graph.server_manifest.len());
                println!("diagnostics  {}", analysis.diagnostics.len());
                for d in &analysis.diagnostics {
                    println!("  [{:?}] {}: {}", d.severity, d.code, d.message);
                }
            }
            let errors = analysis
                .diagnostics
                .iter()
                .any(|d| d.severity == yolo_next::types::Severity::Error);
            Ok(if errors {
                ExitCode::from(1)
            } else {
                ExitCode::SUCCESS
            })
        }
        "migrate" => {
            let root = pos
                .first()
                .map(Path::new)
                .ok_or("migrate needs <project>")?;
            let out = flag(rest, "--out")
                .map(PathBuf::from)
                .ok_or("migrate needs --out <dir>")?;
            let targets: Vec<Target> = match flag(rest, "--target").unwrap_or("bun") {
                "all" => Target::ALL.to_vec(),
                list => list
                    .split(',')
                    .map(|t| Target::from_id(t).ok_or_else(|| format!("unknown target `{t}`")))
                    .collect::<Result<_, _>>()?,
            };
            let (analysis, files, report) = project::migrate(root, &targets);
            if has(rest, "--dry-run") {
                for f in &files {
                    println!("{}", f.path);
                }
            } else {
                let n = project::write_emitted(&out, &files).map_err(|e| e.to_string())?;
                println!("wrote {n} files to {}", out.display());
            }
            println!(
                "parity {}% | routes {} | diagnostics {}",
                report.parity_score,
                analysis.routes.entries.len(),
                report.diagnostics.len()
            );
            Ok(ExitCode::SUCCESS)
        }
        "config" => {
            let file = pos
                .first()
                .map(Path::new)
                .ok_or("config needs <next.config.*>")?;
            let source = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
            let name = file
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            let (cfg, diags) = config::parse_next_config(&source, &name).map_err(|errs| {
                errs.iter()
                    .map(|d| d.message.clone())
                    .collect::<Vec<_>>()
                    .join("\n")
            })?;
            let (yolo, report) = config::next_config_to_yolo(&cfg, "app", None, None);
            if has(rest, "--json") {
                println!(
                    "{}",
                    to_json(
                        &serde_json::json!({ "next": cfg, "yolo": yolo, "report": report, "diagnostics": diags })
                    )
                );
            } else if has(rest, "--ts") {
                println!("{}", config::yolo_config_ts(&yolo));
            } else {
                println!("{}", config::yolo_config_toml(&yolo));
            }
            Ok(ExitCode::SUCCESS)
        }
        "routes" => {
            let root = pos.first().map(Path::new).ok_or("routes needs <project>")?;
            let manifest = routes::scan_project(root);
            match flag(rest, "--format").unwrap_or("json") {
                "json" => println!("{}", to_json(&manifest)),
                "bun" => manifest
                    .entries
                    .iter()
                    .for_each(|e| println!("{:<8} {}", format!("{:?}", e.kind), e.bun_pattern)),
                "tanstack" => manifest
                    .entries
                    .iter()
                    .for_each(|e| println!("{}", e.tanstack_path)),
                "dioxus" => manifest.entries.iter().for_each(|e| {
                    println!("#[route(\"{}\")] {} ", e.dioxus_route, e.dioxus_variant);
                }),
                "leptos" => manifest
                    .entries
                    .iter()
                    .for_each(|e| println!("{}", e.leptos_path)),
                other => return Err(format!("unknown format `{other}`")),
            }
            Ok(ExitCode::SUCCESS)
        }
        "match" => {
            let root = pos
                .first()
                .map(Path::new)
                .ok_or("match needs <project> <pathname>")?;
            let pathname = pos.get(1).ok_or("match needs <pathname>")?;
            let manifest = routes::scan_project(root);
            match routes::match_path(&manifest, pathname) {
                Some(m) => println!(
                    "{}",
                    to_json(
                        &serde_json::json!({ "route": m.entry.url_pattern, "file": m.entry.file, "params": m.params })
                    )
                ),
                None => println!("{{\"route\":null}}"),
            }
            Ok(ExitCode::SUCCESS)
        }
        "fonts" => {
            let file = pos.first().map(Path::new).ok_or("fonts needs <file.tsx>")?;
            let source = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
            let requests = fonts::extract_font_requests(&source, &file.to_string_lossy());
            let plans: Vec<_> = requests
                .iter()
                .map(|r| fonts::plan_font(r, "/_yolo/fonts"))
                .collect();
            if has(rest, "--json") {
                println!("{}", to_json(&plans));
            } else {
                for p in &plans {
                    println!(
                        "{} -> {} {}",
                        p.request.binding,
                        p.class_name,
                        p.css2_url.as_deref().unwrap_or("(local)")
                    );
                }
            }
            Ok(ExitCode::SUCCESS)
        }
        "metrics" => {
            let file = pos
                .first()
                .map(Path::new)
                .ok_or("metrics needs <font.ttf>")?;
            let bytes = std::fs::read(file).map_err(|e| e.to_string())?;
            let family = file
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default();
            let metrics = fonts::parse_sfnt_metrics(&bytes, &family).ok_or("not an SFNT font")?;
            let fallback = match flag(rest, "--fallback").unwrap_or("arial") {
                "times" | "serif" => fonts::times_new_roman_metrics(),
                _ => fonts::arial_metrics(),
            };
            let overrides = fonts::fallback_overrides(&metrics, &fallback);
            println!(
                "{}",
                to_json(&serde_json::json!({ "metrics": metrics, "fallback": overrides }))
            );
            Ok(ExitCode::SUCCESS)
        }
        "image-url" => {
            let src = flag(rest, "--src").ok_or("image-url needs --src")?;
            let width: u32 = flag(rest, "--width")
                .ok_or("image-url needs --width")?
                .parse()
                .map_err(|_| "bad --width")?;
            let quality: u8 = flag(rest, "--quality")
                .unwrap_or("75")
                .parse()
                .map_err(|_| "bad --quality")?;
            let kind = match flag(rest, "--loader").unwrap_or("default") {
                "cloudflare" => yolo_next::types::ImageLoaderKind::Cloudflare,
                "cloudinary" => yolo_next::types::ImageLoaderKind::Cloudinary,
                "imgix" => yolo_next::types::ImageLoaderKind::Imgix,
                "akamai" => yolo_next::types::ImageLoaderKind::Akamai,
                "fastly" => yolo_next::types::ImageLoaderKind::Fastly,
                "supabase" => yolo_next::types::ImageLoaderKind::Supabase,
                _ => yolo_next::types::ImageLoaderKind::Default,
            };
            let policy = images::optimizer_policy(&yolo_next::types::ImagesConfig::default(), "");
            println!("{}", images::loader_url(kind, &policy, src, width, quality));
            Ok(ExitCode::SUCCESS)
        }
        "rsc" => {
            let root = pos.first().map(Path::new).ok_or("rsc needs <project>")?;
            let graph = rsc::scan_module_graph(
                root,
                &[
                    "app",
                    "src/app",
                    "components",
                    "src/components",
                    "lib",
                    "src/lib",
                ],
            );
            if has(rest, "--json") {
                println!(
                    "{}",
                    to_json(
                        &serde_json::json!({ "graph": graph, "clientManifest": rsc::client_manifest_json(&graph), "serverManifest": rsc::server_manifest_json(&graph) })
                    )
                );
            } else {
                for m in &graph.modules {
                    println!("{:<14} {}", format!("{:?}", m.kind), m.id);
                }
                println!("client references: {}", graph.client_manifest.len());
                println!("server references: {}", graph.server_manifest.len());
            }
            Ok(ExitCode::SUCCESS)
        }
        "rule" => {
            let source = pos.first().ok_or("rule needs <source> <pathname>")?;
            let pathname = pos.get(1).ok_or("rule needs <pathname>")?;
            let compiled = rules::compile_source(source).map_err(|d| d.message)?;
            println!(
                "{}",
                to_json(
                    &serde_json::json!({ "bun": compiled.bun_pattern(), "axum": compiled.axum_pattern(), "regex": compiled.to_regex_source(), "params": compiled.matches(pathname) })
                )
            );
            Ok(ExitCode::SUCCESS)
        }
        "-h" | "--help" | "help" => Err(String::new()),
        other => Err(format!("unknown command `{other}`")),
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(code) => code,
        Err(message) => {
            if !message.is_empty() {
                eprintln!("error: {message}\n");
            }
            eprintln!("{USAGE}");
            ExitCode::from(2)
        }
    }
}
