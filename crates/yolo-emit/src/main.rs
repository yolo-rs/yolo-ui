// SPDX-License-Identifier: Apache-2.0
//! `yolo-emit [--preset neutral|m3|fluent|apple|omarchy|hyprland|aphrody|chromium] [--seed '#6750a4'] [--variant tonal-spot] [--contrast 0] [--target NAME|all] [--out DIR] [--list-targets] [--audit]`
//!
//! Without `--out`, prints the primary file of each target. With `--out`, writes every file of
//! the target under `DIR` (or `DIR/<target>/` with `--target all`) and prints the written paths.
//! `--audit` prints the WCAG contrast of the main role pairs in both modes and fails below AA.

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use yolo_emit::contrast::{AA_TEXT, audit};
use yolo_emit::{Target, Theme, ThemeSource, YoloThemePreset, emit_files};
use yolo_tokens::scheme::Variant;

const USAGE: &str = "usage: yolo-emit [--preset neutral|m3|fluent|apple|omarchy|hyprland|aphrody|chromium] [--seed '#rrggbb'] [--variant NAME] [--contrast N] [--target NAME|all] [--out DIR] [--list-targets] [--audit]";

fn parse_seed(text: &str) -> Option<u32> {
    let hex = text.trim_start_matches('#');
    (hex.len() == 6).then(|| {
        u32::from_str_radix(hex, 16)
            .ok()
            .map(|rgb| 0xff00_0000 | rgb)
    })?
}

fn write_files(dir: &Path, source: &ThemeSource, target: Target) -> std::io::Result<()> {
    for file in emit_files(source, target) {
        let path = file
            .path
            .split('/')
            .fold(dir.to_path_buf(), |p, part| p.join(part));
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, file.contents)?;
        println!("{}", path.display());
    }
    Ok(())
}

fn main() -> ExitCode {
    let mut preset: Option<YoloThemePreset> = None;
    let mut seed = YoloThemePreset::Neutral.seed_argb();
    let mut variant = Variant::Neutral;
    let mut contrast = 0.0;
    let mut target = None;
    let mut out: Option<PathBuf> = None;
    let mut run_audit = false;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--list-targets" => {
                for t in Target::ALL {
                    println!("{}", t.name());
                }
                return ExitCode::SUCCESS;
            }
            "--audit" => {
                run_audit = true;
                continue;
            }
            "-h" | "--help" => {
                println!("{USAGE}");
                return ExitCode::SUCCESS;
            }
            _ => {}
        }
        let value = args.next();
        let ok = match (arg.as_str(), value.as_deref()) {
            ("--preset", Some(v)) => YoloThemePreset::from_id(v)
                .map(|p| {
                    preset = Some(p);
                    seed = p.seed_argb();
                })
                .is_some(),
            ("--seed", Some(v)) => parse_seed(v)
                .map(|s| {
                    seed = s;
                    preset = None;
                })
                .is_some(),
            ("--variant", Some(v)) => Variant::from_name(v)
                .map(|x| {
                    variant = x;
                    preset = None;
                })
                .is_some(),
            ("--contrast", Some(v)) => v.parse().map(|x| contrast = x).is_ok(),
            ("--target", Some("all")) => {
                target = None;
                true
            }
            ("--target", Some(v)) => Target::from_name(v).map(|t| target = Some(t)).is_some(),
            ("--out", Some(v)) => {
                out = Some(PathBuf::from(v));
                true
            }
            _ => false,
        };
        if !ok {
            eprintln!("{USAGE}");
            return ExitCode::from(2);
        }
    }
    let source = match preset {
        Some(p) => ThemeSource::from_preset(p),
        None => ThemeSource::from_theme(Theme {
            seed,
            variant,
            contrast,
        }),
    };
    if run_audit {
        let mut failed = false;
        for dark in [false, true] {
            for pair in audit(source.scheme(dark)) {
                let pass = pair.ratio >= AA_TEXT;
                failed |= !pass;
                println!(
                    "{} {} {} on {} {:.2} {}",
                    source.slug,
                    if dark { "dark" } else { "light" },
                    pair.foreground,
                    pair.background,
                    pair.ratio,
                    if pass { "AA" } else { "FAIL" }
                );
            }
        }
        return if failed {
            ExitCode::FAILURE
        } else {
            ExitCode::SUCCESS
        };
    }
    let all = target.is_none();
    let targets: Vec<Target> = target.map_or_else(|| Target::ALL.to_vec(), |t| vec![t]);
    for t in targets {
        match &out {
            Some(dir) => {
                let dir = if all { dir.join(t.name()) } else { dir.clone() };
                if let Err(e) = write_files(&dir, &source, t) {
                    eprintln!("yolo-emit: {}: {e}", t.name());
                    return ExitCode::FAILURE;
                }
            }
            None => {
                if let Some(file) = emit_files(&source, t).into_iter().next() {
                    print!("{}", file.contents);
                }
            }
        }
    }
    ExitCode::SUCCESS
}
