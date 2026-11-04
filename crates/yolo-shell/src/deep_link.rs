// SPDX-License-Identifier: MIT OR Apache-2.0
//! Deep link (custom URL scheme) registration helpers.
//!
//! - Windows: `HKCU\Software\Classes\<scheme>` with `URL Protocol` and a
//!   `shell\open\command` pointing at the executable (`"exe" "%1"`).
//! - Linux/BSD: a desktop entry with `MimeType=x-scheme-handler/<scheme>;`
//!   in `~/.local/share/applications`, then `xdg-mime default` (best effort).
//! - macOS: schemes are declared in the bundle's `Info.plist`
//!   (`CFBundleURLTypes`); LaunchServices registers them when the bundle is
//!   first launched or copied. [`macos_info_plist_fragment`] renders that block.

use std::path::{Path, PathBuf};

use crate::{AppInfo, Error, Result};

/// Returns the first argument that starts with `<scheme>:`.
pub fn find_url<'a>(args: &'a [String], scheme: &str) -> Option<&'a str> {
    args.iter().map(String::as_str).find(|arg| {
        arg.len() > scheme.len()
            && arg.as_bytes()[scheme.len()] == b':'
            && arg[..scheme.len()].eq_ignore_ascii_case(scheme)
    })
}

/// Validates a URL scheme per RFC 3986 (`ALPHA *( ALPHA / DIGIT / "+" / "-" / "." )`).
pub fn validate_scheme(scheme: &str) -> Result<()> {
    let mut chars = scheme.chars();
    let valid = chars.next().is_some_and(|c| c.is_ascii_alphabetic())
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'));
    if valid {
        Ok(())
    } else {
        Err(Error::Invalid(format!("invalid URL scheme {scheme:?}")))
    }
}

/// What [`register`] did on the current OS.
#[derive(Debug, Clone)]
pub enum Registration {
    /// Registry keys written under `HKCU\Software\Classes`.
    WindowsRegistry { schemes: Vec<String> },
    /// Desktop entry written; `xdg_mime` tells whether `xdg-mime` succeeded.
    DesktopEntry { path: PathBuf, xdg_mime: bool },
    /// Nothing written: declare this fragment in the bundle `Info.plist`.
    InfoPlist { fragment: String },
}

/// Registers `schemes` for the running executable on the current OS.
pub fn register(app: &AppInfo, schemes: &[&str]) -> Result<Registration> {
    for scheme in schemes {
        validate_scheme(scheme)?;
    }
    #[cfg(not(target_os = "macos"))]
    let exe = std::env::current_exe()?;
    #[cfg(windows)]
    {
        for scheme in schemes {
            register_windows(app, scheme, &exe)?;
        }
        Ok(Registration::WindowsRegistry {
            schemes: schemes.iter().map(|s| s.to_string()).collect(),
        })
    }
    #[cfg(target_os = "macos")]
    {
        Ok(Registration::InfoPlist {
            fragment: macos_info_plist_fragment(app, schemes),
        })
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        register_linux(app, &exe, schemes)
    }
}

/// Writes `HKCU\Software\Classes\<scheme>` for `exe`.
#[cfg(windows)]
pub fn register_windows(app: &AppInfo, scheme: &str, exe: &Path) -> Result<()> {
    validate_scheme(scheme)?;
    let base_key = format!("HKCU\\Software\\Classes\\{scheme}");
    let run_reg = |args: &[&str]| -> Result<()> {
        let status = std::process::Command::new("reg").args(args).status()?;
        if status.success() {
            Ok(())
        } else {
            Err(Error::Invalid(format!(
                "reg command failed with status {status}"
            )))
        }
    };
    let desc = format!("URL:{} protocol", app.name);
    let icon_val = format!("\"{}\",0", exe.display());
    let cmd_val = format!("\"{}\" \"%1\"", exe.display());
    run_reg(&["add", &base_key, "/ve", "/d", &desc, "/f"])?;
    run_reg(&["add", &base_key, "/v", "URL Protocol", "/d", "", "/f"])?;
    run_reg(&[
        "add",
        &format!("{base_key}\\DefaultIcon"),
        "/ve",
        "/d",
        &icon_val,
        "/f",
    ])?;
    run_reg(&[
        "add",
        &format!("{base_key}\\shell\\open\\command"),
        "/ve",
        "/d",
        &cmd_val,
        "/f",
    ])?;
    Ok(())
}

/// Removes `HKCU\Software\Classes\<scheme>`.
#[cfg(windows)]
pub fn unregister_windows(scheme: &str) -> Result<()> {
    validate_scheme(scheme)?;
    let base_key = format!("HKCU\\Software\\Classes\\{scheme}");
    let status = std::process::Command::new("reg")
        .args(["delete", &base_key, "/f"])
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(Error::Invalid(format!(
            "failed to delete registry key {base_key}"
        )))
    }
}

/// Desktop entry advertising `schemes` through `x-scheme-handler` MIME types.
pub fn linux_desktop_entry(app: &AppInfo, exe: &Path, schemes: &[&str]) -> String {
    let mime: String = schemes
        .iter()
        .map(|s| format!("x-scheme-handler/{s};"))
        .collect();
    let exec = exe
        .display()
        .to_string()
        .replace('\\', "\\\\")
        .replace(' ', "\\s");
    let mut entry = format!(
        "[Desktop Entry]\nType=Application\nName={}\nExec={} %u\nTerminal=false\nMimeType={}\nStartupWMClass={}\n",
        app.name, exec, mime, app.id
    );
    if let Some(description) = &app.description {
        entry.push_str(&format!("Comment={}\n", description.replace('\n', " ")));
    }
    entry.push_str(&format!("Icon={}\n", app.id));
    entry
}

/// Writes `<data>/applications/<id>.desktop` and asks `xdg-mime` to make it the
/// default handler for each scheme.
#[cfg(not(any(windows, target_os = "macos")))]
pub fn register_linux(app: &AppInfo, exe: &Path, schemes: &[&str]) -> Result<Registration> {
    let base = directories::BaseDirs::new().ok_or(Error::NoHomeDir)?;
    let dir = base.data_dir().join("applications");
    std::fs::create_dir_all(&dir)?;
    let file_name = format!("{}.desktop", app.slug());
    let path = dir.join(&file_name);
    std::fs::write(&path, linux_desktop_entry(app, exe, schemes))?;
    let mut xdg_mime = !schemes.is_empty();
    for scheme in schemes {
        let ok = std::process::Command::new("xdg-mime")
            .args([
                "default",
                &file_name,
                &format!("x-scheme-handler/{scheme}"),
            ])
            .status()
            .is_ok_and(|s| s.success());
        xdg_mime &= ok;
    }
    let _ = std::process::Command::new("update-desktop-database")
        .arg(&dir)
        .status();
    Ok(Registration::DesktopEntry { path, xdg_mime })
}

/// `CFBundleURLTypes` block to paste in (or merge into) the bundle `Info.plist`.
pub fn macos_info_plist_fragment(app: &AppInfo, schemes: &[&str]) -> String {
    let schemes: String = schemes
        .iter()
        .map(|s| format!("        <string>{s}</string>\n"))
        .collect();
    format!(
        "<key>CFBundleIdentifier</key>\n<string>{id}</string>\n<key>CFBundleURLTypes</key>\n<array>\n  <dict>\n    <key>CFBundleURLName</key>\n    <string>{id}</string>\n    <key>CFBundleURLSchemes</key>\n    <array>\n{schemes}    </array>\n  </dict>\n</array>\n",
        id = app.id
    )
}
