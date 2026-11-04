// SPDX-License-Identifier: Apache-2.0
//! The egui adapter for the canonical Yolo UI 5-preset theme engine (`Neutral`, `M3`, `Fluent`,
//! `Apple`, `Aphrody`) and Material Design 3 [`DynamicScheme`].
//!
//! Colors are derived from [`yolo_tokens::presets`] and [`yolo_contracts::theme::Theme`] — no
//! duplicate hex tables live here.

use egui::{Color32, CornerRadius, Stroke, Visuals};
pub use yolo_shared::YoloThemePreset;
use yolo_tokens::presets::scheme_for_preset;
pub use yolo_tokens::role::Role;
use yolo_tokens::scheme::DynamicScheme;

use super::{shape, WidgetPalette};

/// Resolved M3 / Yolo color palette for an egui frame.
#[derive(Clone, Copy, Debug)]
pub struct Palette {
    scheme: DynamicScheme,
    preset: YoloThemePreset,
}

impl Palette {
    /// Resolve the palette for `dark` mode from the canonical Aphrody / Byron Love request.
    #[must_use]
    pub fn new(dark: bool) -> Self {
        Self::for_preset(YoloThemePreset::Aphrody, dark)
    }

    /// Resolve the palette for `dark` mode from any of the 5 [`YoloThemePreset`]s.
    #[must_use]
    pub fn for_preset(preset: YoloThemePreset, dark: bool) -> Self {
        Self {
            scheme: scheme_for_preset(preset, dark),
            preset,
        }
    }

    /// Resolve the light and dark palettes from the canonical request in one validation pass.
    #[must_use]
    pub fn pair() -> (Self, Self) {
        Self::pair_for_preset(YoloThemePreset::Aphrody)
    }

    /// Resolve the light and dark palettes for any [`YoloThemePreset`] in one validation pass.
    #[must_use]
    pub fn pair_for_preset(preset: YoloThemePreset) -> (Self, Self) {
        (
            Self::for_preset(preset, false),
            Self::for_preset(preset, true),
        )
    }

    /// Returns the [`YoloThemePreset`] this palette was resolved from.
    #[must_use]
    pub const fn preset(&self) -> YoloThemePreset {
        self.preset
    }

    /// Look up any of the 49 M3 semantic roles as an egui [`Color32`].
    #[must_use]
    pub fn role(&self, role: Role) -> Color32 {
        argb_to_color32(self.scheme.argb(role))
    }

    /// Whether this palette was resolved in dark mode.
    #[must_use]
    pub const fn is_dark(&self) -> bool {
        self.scheme.is_dark
    }

    /// Primary accent role (`Role::Primary`).
    #[must_use]
    pub fn accent(&self) -> Color32 {
        self.role(Role::Primary)
    }

    /// Window and central canvas background (`Role::Surface`).
    #[must_use]
    pub fn surface(&self) -> Color32 {
        self.role(Role::Surface)
    }

    /// Elevated bar, dock and panel container (`Role::SurfaceContainer`).
    #[must_use]
    pub fn panel(&self) -> Color32 {
        self.role(Role::SurfaceContainer)
    }

    /// Primary foreground text and icon ink (`Role::OnSurface`).
    #[must_use]
    pub fn ink(&self) -> Color32 {
        self.role(Role::OnSurface)
    }

    /// Secondary foreground ink (`Role::OnSurfaceVariant`).
    #[must_use]
    pub fn muted_ink(&self) -> Color32 {
        self.role(Role::OnSurfaceVariant)
    }

    /// Subtle structural divider (`Role::OutlineVariant`).
    #[must_use]
    pub fn border(&self) -> Color32 {
        self.role(Role::OutlineVariant)
    }

    /// Five-state widget fills derived from M3 container and state-layer roles.
    #[must_use]
    pub fn widget_palette(&self) -> WidgetPalette {
        WidgetPalette {
            noninteractive: self.role(Role::SurfaceContainerLow),
            inactive: self.role(Role::SurfaceContainerHigh),
            hovered: self.role(Role::SurfaceContainerHighest),
            active: self.role(Role::SecondaryContainer),
            open: self.role(Role::SecondaryContainer),
        }
    }

    /// Build complete egui [`Visuals`] from the resolved M3 roles and shape tokens.
    #[must_use]
    pub fn visuals(&self) -> Visuals {
        let mut visuals = if self.is_dark() {
            Visuals::dark()
        } else {
            Visuals::light()
        };
        let ink = self.ink();
        let border = Stroke::new(1.0, self.border());
        let (ctrl_radius, menu_radius, win_radius) = match self.preset {
            YoloThemePreset::Omarchy => (
                CornerRadius::same(0),
                CornerRadius::same(0),
                CornerRadius::same(2),
            ),
            YoloThemePreset::Fluent => (
                CornerRadius::same(4),
                CornerRadius::same(6),
                CornerRadius::same(8),
            ),
            YoloThemePreset::Apple => (
                CornerRadius::same(10),
                CornerRadius::same(10),
                CornerRadius::same(14),
            ),
            YoloThemePreset::Hyprland => (
                CornerRadius::same(10),
                CornerRadius::same(10),
                CornerRadius::same(12),
            ),
            _ => (shape::CONTROL, shape::MENU, shape::WINDOW),
        };
        self.widget_palette()
            .apply_to(&mut visuals.widgets, ctrl_radius, Stroke::new(1.0, ink));
        visuals.widgets.noninteractive.bg_stroke = border;
        visuals.widgets.inactive.bg_stroke = border;
        visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, self.role(Role::Outline));
        visuals.widgets.active.bg_stroke = Stroke::new(1.0, self.accent());
        visuals.widgets.active.fg_stroke = Stroke::new(1.0, self.role(Role::OnSecondaryContainer));
        visuals.widgets.open.bg_stroke = Stroke::new(1.0, self.accent());
        visuals.widgets.open.fg_stroke = Stroke::new(1.0, self.role(Role::OnSecondaryContainer));
        visuals.selection.bg_fill = self.role(Role::PrimaryContainer);
        visuals.selection.stroke = Stroke::new(1.0, self.role(Role::OnPrimaryContainer));
        visuals.hyperlink_color = self.accent();
        visuals.faint_bg_color = self.role(Role::SurfaceContainerLow);
        visuals.extreme_bg_color = self.role(Role::SurfaceContainerLowest);
        visuals.code_bg_color = self.role(Role::SurfaceContainerHighest);
        visuals.warn_fg_color = self.role(Role::Tertiary);
        visuals.error_fg_color = self.role(Role::Error);
        visuals.window_fill = self.panel();
        visuals.window_stroke = border;
        visuals.window_corner_radius = win_radius;
        visuals.panel_fill = self.panel();
        visuals.menu_corner_radius = menu_radius;
        visuals.override_text_color = Some(ink);
        visuals
    }
}

/// Apply the canonical Aphrody M3 [`Visuals`] for `dark` mode to `ctx` and return the resolved [`Palette`].
pub fn apply(ctx: &egui::Context, dark: bool) -> Palette {
    apply_preset(ctx, YoloThemePreset::Aphrody, dark)
}

/// Apply the [`Visuals`] for any of the 5 [`YoloThemePreset`]s to `ctx` and return the resolved [`Palette`].
pub fn apply_preset(ctx: &egui::Context, preset: YoloThemePreset, dark: bool) -> Palette {
    let palette = Palette::for_preset(preset, dark);
    ctx.set_visuals(palette.visuals());
    palette
}

fn argb_to_color32(argb: u32) -> Color32 {
    let [a, r, g, b] = argb.to_be_bytes();
    Color32::from_rgba_unmultiplied(r, g, b, a)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dark_and_light_palettes_resolve_distinct_surfaces() {
        let (light, dark) = Palette::pair();
        assert!(!light.is_dark());
        assert!(dark.is_dark());
        assert_ne!(light.surface(), dark.surface());
        assert_ne!(light.ink(), dark.ink());
        assert_ne!(light.accent(), Color32::TRANSPARENT);
    }

    #[test]
    fn visuals_apply_m3_roles_and_shapes() {
        let visuals = Palette::new(true).visuals();
        assert!(visuals.dark_mode);
        assert_eq!(visuals.widgets.inactive.corner_radius, shape::CONTROL);
        assert_eq!(visuals.menu_corner_radius, shape::MENU);
        assert_eq!(visuals.window_corner_radius, shape::WINDOW);
    }

    #[test]
    fn all_five_presets_resolve_cleanly() {
        for preset in YoloThemePreset::ALL {
            let (light, dark) = Palette::pair_for_preset(preset);
            assert_eq!(light.preset(), preset);
            assert_eq!(dark.preset(), preset);
            assert_ne!(light.surface(), dark.surface());
        }
    }
}
