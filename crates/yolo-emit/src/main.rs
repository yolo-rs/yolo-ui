// SPDX-License-Identifier: Apache-2.0
//! `yolo-emit [--preset neutral|m3|fluent|apple|omarchy|hyprland|aphrody|chromium] [--seed '#6750a4'] [--variant tonal-spot] [--contrast 0] [--target gtk|fluent|apple|omarchy|hyprland|tailwind|gpui|all] [--out DIR]`

use std::path::PathBuf;
use std::process::ExitCode;
use yolo_emit::{emit, emit_gpui_theme, emit_preset, emit_tailwind_theme, Target, Theme, YoloThemePreset};
use yolo_tokens::scheme::Variant;

fn parse_seed(text: &str) -> Option<u32> {
    let hex = text.trim_start_matches('#');
    (hex.len() == 6).then(|| u32::from_str_radix(hex, 16).ok().map(|rgb| 0xff00_0000 | rgb))?
}

fn main() -> ExitCode {
    let mut preset: Option<YoloThemePreset> = None;
    let mut seed = YoloThemePreset::Neutral.seed_argb();
    let mut variant = Variant::Neutral;
    let mut contrast = 0.0;
    let mut target = None;
    // Preset-only targets outside `Target`: "tailwind" or "gpui".
    let mut extra: Option<&'static str> = None;
    let mut out: Option<PathBuf> = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
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
                extra = None;
                true
            }
            ("--target", Some(v @ ("tailwind" | "gpui"))) => {
                target = None;
                extra = Some(if v == "tailwind" { "tailwind" } else { "gpui" });
                true
            }
            ("--target", Some(v)) => Target::from_name(v)
                .map(|t| {
                    target = Some(t);
                    extra = None;
                })
                .is_some(),
            ("--out", Some(v)) => {
                out = Some(PathBuf::from(v));
                true
            }
            _ => false,
        };
        if !ok {
            eprintln!("usage: yolo-emit [--preset neutral|m3|fluent|apple|omarchy|hyprland|aphrody|chromium] [--seed '#rrggbb'] [--variant NAME] [--contrast N] [--target NAME|tailwind|gpui|all] [--out DIR]");
            return ExitCode::from(2);
        }
    }
    let theme = Theme {
        seed,
        variant,
        contrast,
    };
    if let Some(kind) = extra {
        // These emitters are preset-driven; without --preset the Neutral preset is used.
        let p = preset.unwrap_or(YoloThemePreset::Neutral);
        let (text, file) = if kind == "tailwind" {
            (emit_tailwind_theme(p), "globals.css")
        } else {
            (emit_gpui_theme(p), "gpui-theme.json")
        };
        return match &out {
            Some(dir) => match std::fs::create_dir_all(dir)
                .and_then(|()| std::fs::write(dir.join(file), text))
            {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) => {
                    eprintln!("yolo-emit: {kind}: {e}");
                    ExitCode::FAILURE
                }
            },
            None => {
                print!("{text}");
                ExitCode::SUCCESS
            }
        };
    }
    let targets: Vec<Target> = target.map_or_else(|| Target::ALL.to_vec(), |t| vec![t]);
    for t in targets {
        let text = match preset {
            Some(p) => emit_preset(p, t),
            None => emit(&theme, t),
        };
        match &out {
            Some(dir) => {
                if let Err(e) = std::fs::create_dir_all(dir)
                    .and_then(|()| std::fs::write(dir.join(t.file_name()), text))
                {
                    eprintln!("yolo-emit: {}: {e}", t.name());
                    return ExitCode::FAILURE;
                }
            }
            None => print!("{text}"),
        }
    }
    ExitCode::SUCCESS
}
