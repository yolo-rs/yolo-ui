// SPDX-License-Identifier: Apache-2.0
//! `yolo-icons`: command-line front end used by `bun ui icons`.

use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use yolo_icons::bundle::{build, import_source};
use yolo_icons::color::{palette, preset_from_id};
use yolo_icons::favicon::{FaviconOptions, generate, verify_dir};
use yolo_icons::lock::{AssetLock, SourceFormat};
use yolo_icons::optimize::{OptimizeOptions, optimize};
use yolo_icons::search::search;
use yolo_icons::{Error, aphrody_mark};

#[derive(Parser)]
#[command(
    name = "yolo-icons",
    version,
    about = "Yolo UI icon and favicon pipeline"
)]
struct Cli {
    /// Repository root (holds `catalog/assets.lock.toml`).
    #[arg(long, global = true, default_value = ".")]
    root: PathBuf,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Import, optimize and bundle every set in the asset lock.
    Build {
        #[arg(long, default_value = "dist/icons")]
        out: PathBuf,
        #[arg(long, default_value_t = 24.0)]
        grid: f64,
        #[arg(long, default_value_t = 2)]
        precision: u32,
    },
    /// Search icons across sets.
    Search {
        query: Vec<String>,
        #[arg(long, short = 'n', default_value_t = 20)]
        limit: usize,
    },
    /// Generate favicon.ico, PNGs, maskable icons, favicon.svg and manifest.webmanifest.
    Favicon {
        #[arg(long, default_value = "aphrody")]
        preset: String,
        #[arg(long, default_value = "dist/favicon")]
        out: PathBuf,
        /// SVG or `.icon` mark; defaults to the built-in Aphrody mark.
        #[arg(long)]
        mark: Option<PathBuf>,
        #[arg(long, default_value = "Aphrody")]
        name: String,
        #[arg(long)]
        short_name: Option<String>,
        #[arg(long, default_value = "/")]
        start_url: String,
        /// Paint the first mark layer with the preset's tertiary-container accent.
        #[arg(long)]
        two_tone: bool,
        /// Use the dark scheme for raster outputs.
        #[arg(long)]
        dark: bool,
    },
    /// Decode a favicon output directory back and check it.
    Verify { dir: PathBuf },
    /// Optimize one SVG or `.icon` file and print the result.
    Optimize {
        file: PathBuf,
        #[arg(long, default_value_t = 24.0)]
        grid: f64,
        #[arg(long, default_value_t = 2)]
        precision: u32,
    },
}

fn lock_path(root: &Path) -> PathBuf {
    root.join("catalog/assets.lock.toml")
}

fn format_of(path: &Path) -> SourceFormat {
    if path.extension().is_some_and(|e| e == "icon") {
        SourceFormat::ChromiumIcon
    } else {
        SourceFormat::Svg
    }
}

fn load_mark(path: Option<&Path>) -> Result<yolo_icons::ir::Icon, Error> {
    let Some(path) = path else {
        return aphrody_mark(24);
    };
    let bytes = std::fs::read(path).map_err(|e| Error::Io(format!("{}: {e}", path.display())))?;
    let name = path
        .file_stem()
        .map_or_else(|| "mark".to_owned(), |s| s.to_string_lossy().into_owned());
    let (mut icon, _) = import_source(&name, "custom", format_of(path), &bytes, 24.0)?;
    optimize(&mut icon, &OptimizeOptions::default());
    Ok(icon)
}

fn json<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_string_pretty(value).expect("report serializes")
}

fn run(cli: Cli) -> Result<(), Error> {
    match cli.command {
        Command::Build {
            out,
            grid,
            precision,
        } => {
            let lock = AssetLock::load(&lock_path(&cli.root))?;
            let out = if out.is_absolute() {
                out
            } else {
                cli.root.join(out)
            };
            let options = OptimizeOptions {
                grid,
                decimals: precision,
                ..OptimizeOptions::default()
            };
            let report = build(&cli.root, &lock, &out, &options)?;
            println!("{}", json(&report));
        }
        Command::Search { query, limit } => {
            let lock = AssetLock::load(&lock_path(&cli.root))?;
            println!(
                "{}",
                json(&search(&lock, &cli.root, &query.join(" "), limit))
            );
        }
        Command::Favicon {
            preset,
            out,
            mark,
            name,
            short_name,
            start_url,
            two_tone,
            dark,
        } => {
            let preset = preset_from_id(&preset)?;
            let (light, dark_palette) = (palette(preset, false), palette(preset, true));
            let (raster, other) = if dark {
                (dark_palette, light)
            } else {
                (light, dark_palette)
            };
            let options = FaviconOptions {
                short_name: short_name.unwrap_or_else(|| name.clone()),
                name,
                light: raster,
                dark: other,
                two_tone,
                start_url,
            };
            let mark = load_mark(mark.as_deref())?;
            let out = if out.is_absolute() {
                out
            } else {
                cli.root.join(out)
            };
            let report = generate(&mark, &options, &out)?;
            let verified = verify_dir(&out)?;
            println!(
                "{}",
                json(&serde_json::json!({ "out": out, "report": report, "verified": verified }))
            );
        }
        Command::Verify { dir } => println!("{}", json(&verify_dir(&dir)?)),
        Command::Optimize {
            file,
            grid,
            precision,
        } => {
            let bytes =
                std::fs::read(&file).map_err(|e| Error::Io(format!("{}: {e}", file.display())))?;
            let name = file
                .file_stem()
                .map_or_else(|| "icon".to_owned(), |s| s.to_string_lossy().into_owned());
            let (mut icon, warnings) = import_source(&name, "cli", format_of(&file), &bytes, grid)?;
            optimize(
                &mut icon,
                &OptimizeOptions {
                    grid,
                    decimals: precision,
                    ..OptimizeOptions::default()
                },
            );
            for w in warnings {
                eprintln!("warning: {w}");
            }
            println!("{}", icon.to_svg(precision));
        }
    }
    Ok(())
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("yolo-icons: {e}");
            ExitCode::FAILURE
        }
    }
}
