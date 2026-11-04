// SPDX-License-Identifier: Apache-2.0
//! GTK 4.16 + libadwaita 1.6 + Relm4 stylesheet generation and live `CssProvider` registration.
//!
//! [`stylesheet`] and [`preset_stylesheet`] combine the `--accent-bg-color` / `--window-bg-color` /
//! `--m3-*` custom-property block from `yolo_emit` with the component rules in [`components_css`]
//! for buttons, cards, chips, FABs, navigation rails, Relm4 views, and preset-specific corner radii.

use std::fmt::Write;
#[cfg(feature = "system-gtk4")]
use std::rc::Rc;

use yolo_emit::{Target, Theme, emit};
use yolo_shared::YoloThemePreset;
use yolo_tokens::presets::variant_for_preset;
use yolo_tokens::role::Role;
use yolo_tokens::scheme::Variant;
use yolo_tokens::shape::SCALE as SHAPE;
use yolo_tokens::state;
use yolo_tokens::typography;

/// Component and layout rules appended after `yolo-emit`'s variable block.
#[must_use]
pub fn components_css() -> String {
    let mut s = String::with_capacity(4096);
    let _ = writeln!(
        s,
        ":root {{\n  \
           --m3-shape-none: {}px;\n  \
           --m3-shape-xs: {}px;\n  \
           --m3-shape-sm: {}px;\n  \
           --m3-shape-md: {}px;\n  \
           --m3-shape-lg: {}px;\n  \
           --m3-shape-xl: {}px;\n  \
           --m3-shape-full: 9999px;\n  \
           --m3-state-hover: {:.2};\n  \
           --m3-state-focus: {:.2};\n  \
           --m3-state-pressed: {:.2};\n  \
           --m3-font-display-sm: {}pt;\n  \
           --m3-font-headline-sm: {}pt;\n  \
           --m3-font-title-lg: {}pt;\n  \
           --m3-font-title-md: {}pt;\n  \
           --m3-font-body-lg: {}pt;\n  \
           --m3-font-body-md: {}pt;\n  \
           --m3-font-label-lg: {}pt;\n\
         }}",
        SHAPE.none as u32,
        SHAPE.extra_small as u32,
        SHAPE.small as u32,
        SHAPE.medium as u32,
        SHAPE.large as u32,
        SHAPE.extra_large as u32,
        state::HOVER.opacity,
        state::FOCUS.opacity,
        state::PRESSED.opacity,
        typography::DISPLAY_SMALL.size_dp as u32,
        typography::HEADLINE_SMALL.size_dp as u32,
        typography::TITLE_LARGE.size_dp as u32,
        typography::TITLE_MEDIUM.size_dp as u32,
        typography::BODY_LARGE.size_dp as u32,
        typography::BODY_MEDIUM.size_dp as u32,
        typography::LABEL_LARGE.size_dp as u32,
    );
    s.push_str(
        r#"
/* M3 common buttons (40px height, pill shape) */
button.m3-filled,
button.m3-tonal,
button.m3-elevated,
button.m3-outlined,
button.m3-text {
  min-height: 40px;
  padding: 0 24px;
  border-radius: var(--m3-shape-full);
  font-weight: 500;
}
button.m3-filled {
  background-color: var(--accent-bg-color);
  color: var(--accent-fg-color);
}
button.m3-tonal {
  background-color: var(--m3-secondary-container);
  color: var(--m3-on-secondary-container);
}
button.m3-elevated {
  background-color: var(--sidebar-bg-color);
  color: var(--accent-color);
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.28);
}
button.m3-outlined {
  background: transparent;
  color: var(--accent-color);
  box-shadow: inset 0 0 0 1px var(--m3-outline);
}
button.m3-text {
  background: transparent;
  color: var(--accent-color);
  padding: 0 12px;
}

/* M3 FAB (56px, large shape; .m3-fab-sm 40px; .m3-fab-lg 96px) */
button.m3-fab {
  min-width: 56px;
  min-height: 56px;
  padding: 16px;
  border-radius: var(--m3-shape-lg);
  background-color: var(--m3-primary-container);
  color: var(--m3-on-primary-container);
  box-shadow: 0 4px 10px rgba(0, 0, 0, 0.30);
}
button.m3-fab-sm { min-width: 40px; min-height: 40px; padding: 8px; border-radius: var(--m3-shape-md); }
button.m3-fab-lg { min-width: 96px; min-height: 96px; padding: 30px; border-radius: var(--m3-shape-xl); }

/* M3 chips (32px, small shape) */
button.m3-chip, togglebutton.m3-chip {
  min-height: 32px;
  padding: 0 16px;
  border-radius: var(--m3-shape-sm);
  background: transparent;
  box-shadow: inset 0 0 0 1px var(--border-color);
}
togglebutton.m3-chip:checked {
  background-color: var(--m3-secondary-container);
  color: var(--m3-on-secondary-container);
  box-shadow: none;
}

/* M3 cards (12px medium shape) */
.m3-card-filled {
  background-color: var(--thumbnail-bg-color);
  color: var(--window-fg-color);
  border-radius: var(--m3-shape-md);
  padding: 16px;
}
.m3-card-elevated {
  background-color: var(--sidebar-bg-color);
  color: var(--window-fg-color);
  border-radius: var(--m3-shape-md);
  padding: 16px;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.26);
}
.m3-card-outlined {
  background-color: var(--window-bg-color);
  color: var(--window-fg-color);
  border-radius: var(--m3-shape-md);
  padding: 16px;
  box-shadow: inset 0 0 0 1px var(--border-color);
}

/* M3 navigation rail (80px wide, 56x32 pill indicator on the selected row) */
box.m3-nav-rail {
  min-width: 80px;
  padding: 12px 0;
  background-color: var(--sidebar-bg-color);
}
listbox.m3-nav-rail-list {
  background: transparent;
}
listbox.m3-nav-rail-list > row {
  padding: 6px 12px;
  background: transparent;
}
listbox.m3-nav-rail-list > row .m3-nav-pill {
  min-width: 56px;
  min-height: 32px;
  border-radius: var(--m3-shape-full);
  transition: background-color 200ms cubic-bezier(0.2, 0, 0, 1);
}
listbox.m3-nav-rail-list > row:selected .m3-nav-pill {
  background-color: var(--m3-secondary-container);
  color: var(--m3-on-secondary-container);
}
listbox.m3-nav-rail-list > row .m3-nav-label {
  font-size: 9pt;
  font-weight: 500;
}

/* Relm4 & Yolo preset overrides */
.yolo-preset-omarchy button.m3-filled,
.yolo-preset-omarchy button.m3-tonal,
.yolo-preset-omarchy button.m3-elevated,
.yolo-preset-omarchy button.m3-outlined {
  border-radius: 0px;
}
.yolo-preset-fluent button.m3-filled,
.yolo-preset-fluent button.m3-tonal,
.yolo-preset-fluent button.m3-elevated,
.yolo-preset-fluent button.m3-outlined {
  border-radius: 4px;
}
.yolo-preset-apple button.m3-filled,
.yolo-preset-apple button.m3-tonal,
.yolo-preset-apple button.m3-elevated,
.yolo-preset-apple button.m3-outlined,
.yolo-preset-hyprland button.m3-filled,
.yolo-preset-hyprland button.m3-tonal,
.yolo-preset-hyprland button.m3-elevated,
.yolo-preset-hyprland button.m3-outlined {
  border-radius: 10px;
}

/* Material Symbols icon font */
label.m3-symbol {
  font-family: "Material Symbols Outlined", "Material Symbols Rounded";
  font-size: 18pt;
  font-variation-settings: "FILL" 0, "wght" 400, "GRAD" 0, "opsz" 24;
}
label.m3-symbol-filled {
  font-variation-settings: "FILL" 1, "wght" 400, "GRAD" 0, "opsz" 24;
}
"#,
    );
    s
}

/// Complete GTK 4 / libadwaita 1.6+ stylesheet for `(seed, variant, contrast)`.
#[must_use]
pub fn stylesheet(seed: u32, variant: Variant, contrast: f64) -> String {
    let mut css = emit(
        &Theme {
            seed,
            variant,
            contrast,
        },
        Target::Gtk,
    );
    css.push('\n');
    css.push_str(&components_css());
    css
}

/// Complete GTK 4.16 + libadwaita 1.6 + Relm4 stylesheet for one of the 5 [`YoloThemePreset`]s
/// (`Neutral`, `M3`, `Fluent`, `Apple`, `Aphrody`).
#[must_use]
pub fn preset_stylesheet(preset: YoloThemePreset) -> String {
    let seed = preset.seed_argb();
    let variant = variant_for_preset(preset);
    let mut css = stylesheet(seed, variant, 0.0);
    let light_tokens = preset.tokens(false);
    let dark_tokens = preset.tokens(true);
    let radius = match preset {
        YoloThemePreset::Omarchy => 0,
        YoloThemePreset::Fluent => 4,
        YoloThemePreset::Apple | YoloThemePreset::Hyprland => 10,
        YoloThemePreset::Neutral => 8,
        YoloThemePreset::M3 => 12,
        YoloThemePreset::Aphrody => 12,
        YoloThemePreset::Chromium => 8,
    };
    let _ = writeln!(
        css,
        "\n/* Preset: {} ({}) */\n:root {{\n  --yolo-preset-id: \"{}\";\n  --yolo-control-radius: {}px;\n  --yolo-primary-light: {};\n  --yolo-primary-dark: {};\n}}",
        preset.display_name(),
        preset.seed_hex(),
        preset.id(),
        radius,
        light_tokens.accent_primary,
        dark_tokens.accent_primary,
    );
    css
}

/// Resolves a single M3 [`Role`] hex string (`#rrggbb`) for a given [`YoloThemePreset`] and `dark` mode.
#[must_use]
pub fn preset_role_hex(preset: YoloThemePreset, role: Role, dark: bool) -> String {
    let scheme = yolo_tokens::presets::scheme_for_preset(preset, dark);
    yolo_emit::hex(scheme.argb(role))
}

/// Live GTK 4 `CssProvider` + `AdwStyleManager` handle (requires `system-gtk4` feature).
#[cfg(feature = "system-gtk4")]
#[derive(Clone)]
pub struct M3Gtk {
    provider: gtk::CssProvider,
    seed: std::cell::Cell<u32>,
    variant: std::cell::Cell<Variant>,
    contrast: std::cell::Cell<f64>,
}

#[cfg(feature = "system-gtk4")]
impl M3Gtk {
    /// Registers a `CssProvider` at `STYLE_PROVIDER_PRIORITY_APPLICATION` on the default
    /// `gdk::Display` and loads the M3 stylesheet for `(seed, variant, contrast)`.
    #[must_use]
    pub fn install(seed: u32, variant: Variant, contrast: f64) -> Rc<Self> {
        let provider = gtk::CssProvider::new();
        if let Some(display) = gtk::gdk::Display::default() {
            gtk::style_context_add_provider_for_display(
                &display,
                &provider,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }
        let handle = Rc::new(Self {
            provider,
            seed: std::cell::Cell::new(seed),
            variant: std::cell::Cell::new(variant),
            contrast: std::cell::Cell::new(contrast),
        });
        handle.reload();
        handle
    }

    /// Installs one of the 5 [`YoloThemePreset`]s onto the default `gdk::Display`.
    #[must_use]
    pub fn install_preset(preset: YoloThemePreset) -> Rc<Self> {
        Self::install(preset.seed_argb(), variant_for_preset(preset), 0.0)
    }

    /// Updates `(seed, variant, contrast)` and reloads the provider in place.
    pub fn update(&self, seed: u32, variant: Variant, contrast: f64) {
        self.seed.set(seed);
        self.variant.set(variant);
        self.contrast.set(contrast);
        self.reload();
    }

    /// Updates to a [`YoloThemePreset`] and reloads the provider in place.
    pub fn set_preset(&self, preset: YoloThemePreset) {
        self.update(preset.seed_argb(), variant_for_preset(preset), 0.0);
    }

    /// Sets libadwaita's color scheme preference (`Default`, `ForceLight`, `ForceDark`).
    pub fn set_color_scheme(&self, scheme: adw::ColorScheme) {
        adw::StyleManager::default().set_color_scheme(scheme);
    }

    fn reload(&self) {
        let css = stylesheet(self.seed.get(), self.variant.get(), self.contrast.get());
        self.provider.load_from_string(&css);
    }
}
