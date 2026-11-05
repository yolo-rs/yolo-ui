//! Import an Omarchy theme directory (`colors.toml` + `backgrounds/`) as a theme pack.
//!
//! Omarchy themes already switch colours and wallpapers together; importing one gives a pack
//! with the same seed and wallpaper order. The Omarchy repository is MIT, but individual
//! wallpapers can be third-party art, so imported wallpapers are marked `unverified` and stay
//! out of distribution until each file's licence is checked.

use std::fs;
use std::io;
use std::path::Path;

use serde::Deserialize;

use crate::manifest::{
    ColorSource, ColorSpec, IconSpec, IconStyle, Mode, MotionProfile, ShellLayout, ShellSpec,
    ThemePack, Titles, Typography, Wallpaper, WallpaperFit,
};

#[derive(Deserialize)]
struct OmarchyColors {
    #[serde(default)]
    mode: Option<String>,
    accent: String,
}

/// Builds a pack from an Omarchy `colors.toml` text and the file names in `backgrounds/`.
///
/// # Errors
/// Returns the TOML error when `colors.toml` has no `accent`.
pub fn pack_from_colors(
    id: &str,
    colors_toml: &str,
    backgrounds: &[String],
) -> Result<ThemePack, toml::de::Error> {
    let colors: OmarchyColors = toml::from_str(colors_toml)?;
    let mode = match colors.mode.as_deref() {
        Some("light") => Mode::Light,
        Some("dark") => Mode::Dark,
        _ => Mode::System,
    };
    let wallpapers = backgrounds
        .iter()
        .map(|file| Wallpaper {
            file: format!("backgrounds/{file}"),
            mode: None,
            seed: None,
            license: "unverified".to_owned(),
            source: format!("omarchy:themes/{id}/backgrounds/{file}"),
            attribution: None,
            fit: WallpaperFit::Cover,
            focal: [50, 50],
            sha256: None,
        })
        .collect();
    Ok(ThemePack {
        id: format!("omarchy-{id}"),
        name: format!("Omarchy {id}"),
        title: Titles {
            product_name: "Yolo".to_owned(),
            window_title_template: "{page} - Yolo".to_owned(),
            ntp_heading: "Yolo".to_owned(),
        },
        logo: None,
        color: ColorSpec {
            seed: colors.accent,
            source: ColorSource::Seed,
            // Fidelity keeps the accent itself as the primary container, close to the original look.
            variant: "fidelity".to_owned(),
            contrast: 0.0,
            mode,
            overrides: std::collections::BTreeMap::new(),
        },
        icons: IconSpec { set: "phosphor".to_owned(), style: IconStyle::Outlined, weight: 400 },
        wallpapers,
        pet: None,
        typography: Typography {
            display: "Google Sans Flex".to_owned(),
            body: "Google Sans Flex".to_owned(),
            mono: "Google Sans Code".to_owned(),
        },
        shell: ShellSpec {
            layout: ShellLayout::TopBar,
            density: -1,
            radius: [2, 4, 8],
            glass_blur: 0,
            motion: MotionProfile::Calm,
        },
    })
}

/// Imports `themes/<id>` from an Omarchy checkout.
///
/// # Errors
/// Fails when the directory or `colors.toml` cannot be read, or the colours are invalid.
pub fn import_dir(theme_dir: &Path) -> io::Result<ThemePack> {
    let id = theme_dir
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "theme directory has no name"))?;
    let colors = fs::read_to_string(theme_dir.join("colors.toml"))?;
    let mut backgrounds: Vec<String> = match fs::read_dir(theme_dir.join("backgrounds")) {
        Ok(entries) => entries
            .filter_map(Result::ok)
            .filter(|e| e.file_type().is_ok_and(|t| t.is_file()))
            .filter_map(|e| e.file_name().into_string().ok())
            .collect(),
        Err(_) => Vec::new(),
    };
    backgrounds.sort();
    pack_from_colors(id, &colors, &backgrounds)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resolve::{UserPrefs, resolve};

    const CATPPUCCIN: &str = "mode = \"dark\"\naccent = \"#89b4fa\"\nbackground = \"#1e1e2e\"\n";

    #[test]
    fn imports_seed_mode_and_unverified_wallpapers() {
        let pack = pack_from_colors("catppuccin", CATPPUCCIN, &["1-a.webp".into(), "2-b.webp".into()])
            .expect("valid colors");
        assert_eq!(pack.color.seed, "#89b4fa");
        assert_eq!(pack.color.mode, Mode::Dark);
        assert_eq!(pack.wallpapers.len(), 2);
        assert_eq!(pack.distributable_wallpapers().count(), 0);
        let resolved = resolve(&pack, &UserPrefs::default(), false);
        assert!(resolved.dark);
        assert_eq!(resolved.seed, "#89b4fa");
    }

    #[test]
    fn rejects_colors_without_accent() {
        assert!(pack_from_colors("x", "mode = \"dark\"\n", &[]).is_err());
    }
}
