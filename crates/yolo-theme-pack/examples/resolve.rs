//! Resolves a theme pack directory and prints the JSON document consumers load.
//!
//! `cargo run -p yolo-theme-pack --example resolve -- themes/aurora-terminal [light|dark]`

#![allow(clippy::print_stdout, clippy::print_stderr)]

use std::{env, fs, process::ExitCode};

use yolo_theme_pack::{ThemePack, UserPrefs, resolve};

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let Some(dir) = args.first() else {
        eprintln!("usage: resolve <pack-dir> [light|dark]");
        return ExitCode::from(2);
    };
    let dark = args.get(1).is_none_or(|m| m != "light");
    let text = match fs::read_to_string(format!("{dir}/theme.toml")) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("cannot read {dir}/theme.toml: {e}");
            return ExitCode::FAILURE;
        }
    };
    let pack = match ThemePack::from_toml(&text) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("invalid pack: {e}");
            return ExitCode::FAILURE;
        }
    };
    let theme = resolve(&pack, &UserPrefs { clamp_text: true, ..UserPrefs::default() }, dark);
    match theme.to_json() {
        Ok(json) => {
            println!("{json}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("serialization failed: {e}");
            ExitCode::FAILURE
        }
    }
}
