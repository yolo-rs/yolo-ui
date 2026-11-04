// SPDX-License-Identifier: Apache-2.0
//! `yolo-migrate`: CLI compiler for React + Shadcn UI + Radix UI + Base UI + M3 + Tailwind CSS
//! into Dioxus 0.8 `rsx!`, WebGPU/Taffy, Egui, Cosmic/Iced, GTK4 GtkBuilder XML, and Qt6 QML.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use yolo_migrator::{MigrationTarget, YoloThemePreset, emit_document, tsx};

const USAGE: &str = "usage: yolo-migrate [--target dioxus,wgpu,egui,cosmic,gtk,qml|all] [--theme neutral|m3|fluent|apple|omarchy|hyprland|aphrody] [--out-dir DIR] [--check] [--json] <input.tsx>...

Converts one stateless React / Shadcn UI / Radix UI / Base UI / Material 3 + Tailwind CSS component per file into:
  dioxus  <stem>_dioxus.rs (Dioxus 0.8 `rsx!` fullstack/WebGPU/WASM component)
  wgpu    <stem>_wgpu.rs   (WebGPU / WGPU + Taffy `yolo-layout` scene descriptor)
  egui    <stem>_egui.rs   (egui 0.32+ immediate-mode function, state struct and actions trait)
  cosmic  <stem>_cosmic.rs (libcosmic / Iced 0.14 Elm MVU view and message enum)
  gtk     <stem>.ui        (GtkBuilder, GTK 4.16 + libadwaita 1.6) and <stem>.gtk.css when needed
  qml     <Component>.qml  (Qt 6.8+ Qt Quick Controls Material + Kirigami)

  --theme     YoloThemePreset (`yolo-neutral` [default], `yolo-m3`, `yolo-fluent`, `yolo-apple`, `yolo-omarchy`, `yolo-hyprland`, `yolo-aphrody`)
  --check     validate only, write nothing
  --json      print diagnostics as JSON lines
Exit status: 0 success, 1 unsupported input (diagnostics on stderr), 2 usage or I/O error.";

struct Opts {
    targets: Vec<MigrationTarget>,
    theme: YoloThemePreset,
    out_dir: Option<PathBuf>,
    check: bool,
    json: bool,
    inputs: Vec<PathBuf>,
}

fn parse_args() -> Result<Opts, String> {
    let mut o = Opts {
        targets: MigrationTarget::ALL.to_vec(),
        theme: YoloThemePreset::Neutral,
        out_dir: None,
        check: false,
        json: false,
        inputs: Vec::new(),
    };
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "-h" | "--help" => return Err(String::new()),
            "-t" | "--target" => {
                let v = args.next().ok_or("--target needs a value")?;
                o.targets.clear();
                for t in v.split(',') {
                    let trimmed = t.trim();
                    if trimmed == "all" {
                        o.targets = MigrationTarget::ALL.to_vec();
                    } else if let Some(target) = MigrationTarget::from_id(trimmed) {
                        if !o.targets.contains(&target) {
                            o.targets.push(target);
                        }
                    } else {
                        return Err(format!("unknown target `{trimmed}`"));
                    }
                }
            }
            "--theme" => {
                let v = args.next().ok_or("--theme needs a value")?;
                o.theme = YoloThemePreset::from_id(&v)
                    .ok_or_else(|| format!("unknown theme preset `{v}` (expected neutral, m3, fluent, apple, omarchy, hyprland, aphrody)"))?;
            }
            "-o" | "--out-dir" => o.out_dir = Some(args.next().ok_or("--out-dir needs a value")?.into()),
            "--check" => o.check = true,
            "--json" => o.json = true,
            s if s.starts_with('-') => return Err(format!("unknown option `{s}`")),
            s => o.inputs.push(s.into()),
        }
    }
    if o.inputs.is_empty() {
        return Err("no input files".into());
    }
    Ok(o)
}

fn write_file(path: &Path, text: &str) -> Result<(), String> {
    std::fs::write(path, text).map_err(|e| format!("cannot write {}: {e}", path.display()))?;
    eprintln!("wrote {}", path.display());
    Ok(())
}

fn main() -> ExitCode {
    let opts = match parse_args() {
        Ok(o) => o,
        Err(e) => {
            if !e.is_empty() {
                eprintln!("yolo-migrate: {e}");
            }
            eprintln!("{USAGE}");
            return ExitCode::from(2);
        }
    };
    let mut failed = false;
    for input in &opts.inputs {
        let src = match std::fs::read_to_string(input) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("yolo-migrate: cannot read {}: {e}", input.display());
                return ExitCode::from(2);
            }
        };
        let doc = match tsx::lower(input, &src) {
            Ok(d) => d,
            Err(diags) => {
                for d in &diags {
                    if opts.json {
                        eprintln!("{}", d.to_json());
                    } else {
                        eprintln!("{d}");
                    }
                }
                eprintln!(
                    "yolo-migrate: {}: {} error(s); no output written",
                    input.display(),
                    diags.len()
                );
                failed = true;
                continue;
            }
        };
        if opts.check {
            eprintln!("ok {}", input.display());
            continue;
        }
        let dir = opts
            .out_dir
            .clone()
            .unwrap_or_else(|| input.parent().map(Path::to_path_buf).unwrap_or_default());
        let stem = input
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "out".into());

        for &target in &opts.targets {
            let out = emit_document(&doc, target, opts.theme);
            let res = match target {
                MigrationTarget::Dioxus => write_file(&dir.join(format!("{stem}_dioxus.rs")), &out.code),
                MigrationTarget::Wgpu => write_file(&dir.join(format!("{stem}_wgpu.rs")), &out.code),
                MigrationTarget::Egui => write_file(&dir.join(format!("{stem}_egui.rs")), &out.code),
                MigrationTarget::Cosmic => write_file(&dir.join(format!("{stem}_cosmic.rs")), &out.code),
                MigrationTarget::Gtk => {
                    if let Err(e) = write_file(&dir.join(format!("{stem}.ui")), &out.code) {
                        Err(e)
                    } else if let Some(css) = &out.sidecar_css {
                        write_file(&dir.join(format!("{stem}.gtk.css")), css)
                    } else {
                        Ok(())
                    }
                }
                MigrationTarget::Qml => write_file(&dir.join(format!("{}.qml", doc.component)), &out.code),
            };
            if let Err(e) = res {
                eprintln!("yolo-migrate: {e}");
                return ExitCode::from(2);
            }
        }
    }
    if failed {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}
