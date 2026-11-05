//! Scoped stylesheet for a site that ships several packs: one block per pack, so switching
//! `data-theme-pack` on the root element is enough, with no client script.
//!
//! Layout of the output, for each pack `<id>`:
//! - `[data-theme-pack="<id>"]`: the light scheme (or the only scheme when the pack is pinned to a mode);
//! - `@media (prefers-color-scheme: dark) { [data-theme-pack="<id>"]:not([data-mode]) ... }`: the dark
//!   scheme for visitors who prefer it, unless `data-mode` is set on the root;
//! - `[data-theme-pack="<id>"][data-mode="dark"]` and `[data-mode="light"]`: explicit user choice;
//! - `@media (forced-colors: active)`: system colours, so Windows high contrast stays usable.
//!
//! The first pack is also emitted on `:root` as the default before any attribute is set.

use std::fmt::Write as _;

use crate::manifest::{Mode, ThemePack};
use crate::resolve::{ResolvedTheme, UserPrefs, resolve};

fn declarations(theme: &ResolvedTheme) -> String {
    let css = theme.css(None);
    // The body of the `:root { ... }` block that `css()` writes.
    let start = css.find(":root {").map_or(0, |i| i + ":root {".len());
    let end = css.find("\n}\n").unwrap_or(css.len());
    css[start..end].trim_matches('\n').to_owned()
}

fn block(selector: &str, theme: &ResolvedTheme) -> String {
    format!("{selector} {{\n{}\n}}\n", declarations(theme))
}

/// Stylesheet for `packs` (id taken from each pack). `prefs` applies to every pack; its `mode` is
/// ignored because both schemes are emitted.
#[must_use]
pub fn css_scoped(packs: &[&ThemePack], prefs: &UserPrefs) -> String {
    let mut out = String::from("/* yolo theme packs, scoped by data-theme-pack */\n");
    let resolve_mode = |pack: &ThemePack, dark: bool| {
        let p = UserPrefs {
            mode: Some(if dark { Mode::Dark } else { Mode::Light }),
            ..prefs.clone()
        };
        resolve(pack, &p, dark)
    };
    for (i, pack) in packs.iter().enumerate() {
        let light = resolve_mode(pack, false);
        let dark = resolve_mode(pack, true);
        let id = &pack.id;
        let scope = format!("[data-theme-pack=\"{id}\"]");
        let pinned = match pack.color.mode {
            Mode::Light => Some(&light),
            Mode::Dark => Some(&dark),
            Mode::System => None,
        };
        if i == 0 {
            out.push_str(&block(":root", pinned.unwrap_or(&light)));
        }
        out.push_str(&block(&scope, pinned.unwrap_or(&light)));
        if pinned.is_none() {
            let _ = writeln!(
                out,
                "@media (prefers-color-scheme: dark) {{\n{}}}",
                block(&format!("{scope}:not([data-mode])"), &dark)
            );
        }
        out.push_str(&block(&format!("{scope}[data-mode=\"light\"]"), &light));
        out.push_str(&block(&format!("{scope}[data-mode=\"dark\"]"), &dark));
        let _ = writeln!(
            out,
            "@media (forced-colors: active) {{\n  {scope} {{\n    --yolo-surface: Canvas;\n    --yolo-on-surface: CanvasText;\n    --yolo-primary: LinkText;\n    --yolo-on-primary: Canvas;\n    --yolo-outline: CanvasText;\n    --yolo-focus-ring: Highlight;\n  }}\n}}"
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pack(text: &str) -> ThemePack {
        ThemePack::from_toml(text).expect("pack parses")
    }

    fn packs() -> Vec<ThemePack> {
        vec![
            pack(include_str!("../../../themes/aurora-terminal/theme.toml")),
            pack(include_str!("../../../themes/paper-studio/theme.toml")),
        ]
    }

    #[test]
    fn one_block_per_pack_with_modes_and_forced_colors() {
        let packs = packs();
        let refs: Vec<&ThemePack> = packs.iter().collect();
        let css = css_scoped(&refs, &UserPrefs::default());
        for p in &packs {
            assert!(css.contains(&format!("[data-theme-pack=\"{}\"] {{", p.id)));
            assert!(css.contains(&format!(
                "[data-theme-pack=\"{}\"][data-mode=\"dark\"]",
                p.id
            )));
        }
        assert_eq!(
            css.matches("@media (forced-colors: active)").count(),
            packs.len()
        );
        assert_eq!(
            css.matches('{').count(),
            css.matches('}').count(),
            "balanced braces"
        );
        assert!(css.contains(":root {"));
    }

    #[test]
    fn system_mode_packs_follow_the_os_and_pinned_packs_do_not() {
        let mut system = packs().remove(0);
        system.color.mode = Mode::System;
        let css = css_scoped(&[&system], &UserPrefs::default());
        assert!(css.contains("@media (prefers-color-scheme: dark)"));
        let mut pinned = system.clone();
        pinned.color.mode = Mode::Dark;
        assert!(!css_scoped(&[&pinned], &UserPrefs::default()).contains("prefers-color-scheme"));
    }

    #[test]
    fn schemes_differ_between_light_and_dark_blocks() {
        let packs = packs();
        let css = css_scoped(&[&packs[1]], &UserPrefs::default());
        let value = |selector: &str| {
            let start = css.find(selector).expect("selector");
            let tail = &css[start..];
            tail[tail.find("--yolo-surface:").expect("surface")..]
                .lines()
                .next()
                .map(str::to_owned)
        };
        assert_ne!(
            value("[data-mode=\"light\"]"),
            value("[data-mode=\"dark\"]")
        );
    }
}
