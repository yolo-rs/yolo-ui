// SPDX-License-Identifier: Apache-2.0
//! Applies Yolo UI 5-preset themes (`Neutral`, `M3`, `Fluent`, `Apple`, `Aphrody`) and
//! Material Design 3 [`DynamicScheme`]s to an [`egui::Context`]: [`egui::Visuals`] for light and
//! dark, shape radii, spacing, the 15-step type scale, the 49 colour roles stored in [`egui::Memory`]
//! for custom painting, and the M3 widget set in [`widgets`].

use std::sync::Arc;

use egui::epaint::Shadow;
use egui::style::{Selection, TextCursorStyle, WidgetVisuals, Widgets};
use egui::{Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Id, Stroke, Style, Visuals};
pub use yolo_shared::{
    ByronLoveColorTeinte, ByronLovePetSpec, YoloManifest, YoloStyle, YoloThemePreset,
    YoloThemeTokens, BYRON_LOVE_PALETTE, BYRON_LOVE_PET_SPEC,
};
use yolo_tokens::elevation;
use yolo_tokens::presets::scheme_for_preset;
pub use yolo_tokens::role::Role;
pub use yolo_tokens::scheme::{DynamicScheme, Variant};
use yolo_tokens::shape::SCALE as SHAPE_SCALE;
use yolo_tokens::state;
use yolo_tokens::typography::{self, TypeToken};

pub mod widgets;

/// Converts `0xAARRGGBB` to [`Color32`].
#[must_use]
pub fn color32(argb: u32) -> Color32 {
    Color32::from_rgba_unmultiplied(
        (argb >> 16) as u8,
        (argb >> 8) as u8,
        argb as u8,
        (argb >> 24) as u8,
    )
}

/// Replaces the alpha of `c` with `alpha` in `0.0..=1.0`.
#[must_use]
pub fn with_alpha(c: Color32, alpha: f32) -> Color32 {
    let [r, g, b, _] = c.to_srgba_unmultiplied();
    let a = (alpha.clamp(0.0, 1.0) * 255.0).round() as u8;
    Color32::from_rgba_unmultiplied(r, g, b, a)
}

/// Composites `state` over `container` at `opacity` (the M3 state-layer formula).
#[must_use]
pub fn state_layer(container: Color32, state: Color32, opacity: f32) -> Color32 {
    let t = opacity.clamp(0.0, 1.0);
    let [cr, cg, cb, ca] = container.to_srgba_unmultiplied();
    let [sr, sg, sb, _] = state.to_srgba_unmultiplied();
    let mix = |c: u8, s: u8| ((1.0 - t) * f32::from(c) + t * f32::from(s)).round() as u8;
    Color32::from_rgba_unmultiplied(mix(cr, sr), mix(cg, sg), mix(cb, sb), ca)
}

/// M3 shape tokens as [`CornerRadius`].
pub mod shapes {
    use super::{CornerRadius, SHAPE_SCALE};
    /// 0 dp.
    pub const NONE: CornerRadius = CornerRadius::same(SHAPE_SCALE.none as u8);
    /// 4 dp.
    pub const EXTRA_SMALL: CornerRadius = CornerRadius::same(SHAPE_SCALE.extra_small as u8);
    /// 8 dp.
    pub const SMALL: CornerRadius = CornerRadius::same(SHAPE_SCALE.small as u8);
    /// 12 dp.
    pub const MEDIUM: CornerRadius = CornerRadius::same(SHAPE_SCALE.medium as u8);
    /// 16 dp.
    pub const LARGE: CornerRadius = CornerRadius::same(SHAPE_SCALE.large as u8);
    /// 28 dp.
    pub const EXTRA_LARGE: CornerRadius = CornerRadius::same(SHAPE_SCALE.extra_large as u8);
    /// Pill (`255` saturates to half-height in egui).
    pub const FULL: CornerRadius = CornerRadius::same(u8::MAX);
}

/// M3 elevation level `0..=5` as an [`egui::Shadow`].
#[must_use]
pub fn elevation_shadow(level: u8, shadow_color: Color32) -> Shadow {
    let dp = match level {
        0 => elevation::LEVEL_0.dp,
        1 => elevation::LEVEL_1.dp,
        2 => elevation::LEVEL_2.dp,
        3 => elevation::LEVEL_3.dp,
        4 => elevation::LEVEL_4.dp,
        _ => elevation::LEVEL_5.dp,
    };
    if dp == 0.0 {
        return Shadow::NONE;
    }
    Shadow {
        offset: [0, (dp * 0.5).round() as i8],
        blur: (dp * 2.0).round() as u8,
        spread: 0,
        color: with_alpha(shadow_color, 0.28),
    }
}

/// M3 15-step type scale as [`FontId`]s.
pub mod type_scale {
    use super::{typography, FontFamily, FontId, TypeToken};

    /// Converts a [`TypeToken`] to a proportional [`FontId`].
    #[must_use]
    pub const fn font(t: TypeToken) -> FontId {
        FontId::new(t.size_dp, FontFamily::Proportional)
    }

    /// Display Large (57 sp).
    pub const DISPLAY_LARGE: FontId = font(typography::DISPLAY_LARGE);
    /// Display Medium (45 sp).
    pub const DISPLAY_MEDIUM: FontId = font(typography::DISPLAY_MEDIUM);
    /// Display Small (36 sp).
    pub const DISPLAY_SMALL: FontId = font(typography::DISPLAY_SMALL);
    /// Headline Large (32 sp).
    pub const HEADLINE_LARGE: FontId = font(typography::HEADLINE_LARGE);
    /// Headline Medium (28 sp).
    pub const HEADLINE_MEDIUM: FontId = font(typography::HEADLINE_MEDIUM);
    /// Headline Small (24 sp).
    pub const HEADLINE_SMALL: FontId = font(typography::HEADLINE_SMALL);
    /// Title Large (22 sp).
    pub const TITLE_LARGE: FontId = font(typography::TITLE_LARGE);
    /// Title Medium (16 sp).
    pub const TITLE_MEDIUM: FontId = font(typography::TITLE_MEDIUM);
    /// Title Small (14 sp).
    pub const TITLE_SMALL: FontId = font(typography::TITLE_SMALL);
    /// Body Large (16 sp).
    pub const BODY_LARGE: FontId = font(typography::BODY_LARGE);
    /// Body Medium (14 sp).
    pub const BODY_MEDIUM: FontId = font(typography::BODY_MEDIUM);
    /// Body Small (12 sp).
    pub const BODY_SMALL: FontId = font(typography::BODY_SMALL);
    /// Label Large (14 sp).
    pub const LABEL_LARGE: FontId = font(typography::LABEL_LARGE);
    /// Label Medium (12 sp).
    pub const LABEL_MEDIUM: FontId = font(typography::LABEL_MEDIUM);
    /// Label Small (11 sp).
    pub const LABEL_SMALL: FontId = font(typography::LABEL_SMALL);
}

/// All 49 resolved [`Role`] colours cached as [`Color32`]s; stored in [`egui::Context`] data so
/// custom widgets can read any role in `O(1)`.
#[derive(Clone, Debug)]
pub struct Palette {
    colors: [Color32; Role::ALL.len()],
    /// True when the palette came from a dark scheme.
    pub is_dark: bool,
    /// Optional theme preset this palette was built from.
    pub preset: Option<YoloThemePreset>,
}

impl Palette {
    /// Resolves every role from `scheme`.
    #[must_use]
    pub fn from_scheme(scheme: &DynamicScheme) -> Self {
        let mut colors = [Color32::TRANSPARENT; Role::ALL.len()];
        for (slot, &role) in colors.iter_mut().zip(Role::ALL.iter()) {
            *slot = color32(scheme.argb(role));
        }
        Self {
            colors,
            is_dark: scheme.is_dark,
            preset: None,
        }
    }

    /// Resolves a palette for one of the 7 [`YoloThemePreset`]s.
    #[must_use]
    pub fn from_preset(preset: YoloThemePreset, dark: bool) -> Self {
        let scheme = scheme_for_preset(preset, dark);
        let mut pal = Self::from_scheme(&scheme);
        let tokens = preset.tokens(dark);
        pal.set(Role::Primary, color32(tokens.primary_rgba().to_u32_argb()));
        pal.set(Role::Secondary, color32(tokens.secondary_rgba().to_u32_argb()));
        pal.set(Role::Surface, color32(tokens.surface_rgba().to_u32_argb()));
        pal.set(Role::Background, color32(tokens.surface_rgba().to_u32_argb()));
        pal.set(
            Role::SurfaceContainer,
            color32(tokens.surface_container_rgba().to_u32_argb()),
        );
        pal.set(
            Role::SurfaceContainerHigh,
            color32(tokens.surface_elevated_rgba().to_u32_argb()),
        );
        pal.set(Role::OnSurface, color32(tokens.on_surface_rgba().to_u32_argb()));
        pal.set(
            Role::OnSurfaceVariant,
            color32(tokens.on_surface_variant_rgba().to_u32_argb()),
        );
        pal.set(
            Role::OutlineVariant,
            color32(tokens.outline_rgba().to_u32_argb()),
        );
        pal.preset = Some(preset);
        pal
    }

    /// Colour of `role`.
    #[must_use]
    pub const fn get(&self, role: Role) -> Color32 {
        self.colors[role as usize]
    }

    /// Overrides the colour of `role`.
    pub fn set(&mut self, role: Role, color: Color32) {
        self.colors[role as usize] = color;
    }

    fn data_id(dark: bool) -> Id {
        Id::new(if dark {
            "yolo_egui_palette_dark"
        } else {
            "yolo_egui_palette_light"
        })
    }

    /// Reads the palette matching `ctx`'s active theme from context memory, falling back to the
    /// baseline M3 scheme if [`apply`] was not called.
    #[must_use]
    pub fn of(ctx: &egui::Context) -> Arc<Self> {
        let dark = ctx.global_style().visuals.dark_mode;
        ctx.data_mut(|d| {
            d.get_temp::<Arc<Self>>(Self::data_id(dark)).unwrap_or_else(|| {
                let s = DynamicScheme::from_argb(0xFF67_50A4, Variant::TonalSpot, dark, 0.0);
                let pal = Arc::new(Self::from_scheme(&s));
                d.insert_temp(Self::data_id(dark), Arc::clone(&pal));
                pal
            })
        })
    }
}

/// Builds an [`egui::Visuals`] from a resolved [`DynamicScheme`].
#[must_use]
pub fn visuals(s: &DynamicScheme) -> Visuals {
    visuals_from_palette(&Palette::from_scheme(s), PresetShapeProfile::M3)
}

#[derive(Clone, Copy)]
enum PresetShapeProfile {
    M3,
    Neutral,
    Fluent,
    Apple,
    Omarchy,
    Hyprland,
    Aphrody,
}

impl PresetShapeProfile {
    const fn from_preset(preset: YoloThemePreset) -> Self {
        match preset {
            YoloThemePreset::Neutral => Self::Neutral,
            YoloThemePreset::M3 => Self::M3,
            YoloThemePreset::Fluent => Self::Fluent,
            YoloThemePreset::Apple => Self::Apple,
            YoloThemePreset::Omarchy => Self::Omarchy,
            YoloThemePreset::Hyprland => Self::Hyprland,
            YoloThemePreset::Aphrody => Self::Aphrody,
        }
    }

    const fn control_radius(self) -> CornerRadius {
        match self {
            Self::Omarchy => CornerRadius::same(0),
            Self::Fluent => CornerRadius::same(4),
            Self::Neutral => CornerRadius::same(8),
            Self::M3 => shapes::SMALL,
            Self::Apple | Self::Hyprland => CornerRadius::same(10),
            Self::Aphrody => CornerRadius::same(12),
        }
    }

    const fn window_radius(self) -> CornerRadius {
        match self {
            Self::Omarchy => CornerRadius::same(2),
            Self::Fluent => CornerRadius::same(8),
            Self::Neutral | Self::Hyprland => CornerRadius::same(12),
            Self::M3 => shapes::LARGE,
            Self::Apple => CornerRadius::same(14),
            Self::Aphrody => CornerRadius::same(16),
        }
    }

    const fn menu_radius(self) -> CornerRadius {
        match self {
            Self::Omarchy => CornerRadius::same(0),
            Self::Fluent => CornerRadius::same(6),
            Self::Neutral => CornerRadius::same(8),
            Self::M3 => shapes::EXTRA_SMALL,
            Self::Apple | Self::Hyprland => CornerRadius::same(10),
            Self::Aphrody => CornerRadius::same(12),
        }
    }
}

fn visuals_from_palette(p: &Palette, profile: PresetShapeProfile) -> Visuals {
    let mut v = if p.is_dark {
        Visuals::dark()
    } else {
        Visuals::light()
    };

    let primary = p.get(Role::Primary);
    let on_primary = p.get(Role::OnPrimary);
    let primary_container = p.get(Role::PrimaryContainer);
    let on_primary_container = p.get(Role::OnPrimaryContainer);
    let secondary_container = p.get(Role::SecondaryContainer);
    let on_secondary_container = p.get(Role::OnSecondaryContainer);
    let surface = p.get(Role::Surface);
    let surface_low = p.get(Role::SurfaceContainerLow);
    let surface_c = p.get(Role::SurfaceContainer);
    let surface_high = p.get(Role::SurfaceContainerHigh);
    let surface_highest = p.get(Role::SurfaceContainerHighest);
    let on_surface = p.get(Role::OnSurface);
    let on_surface_var = p.get(Role::OnSurfaceVariant);
    let outline = p.get(Role::Outline);
    let outline_var = p.get(Role::OutlineVariant);
    let error = p.get(Role::Error);
    let shadow = p.get(Role::Shadow);

    let ctrl_radius = profile.control_radius();
    let win_radius = profile.window_radius();
    let menu_radius = profile.menu_radius();

    v.override_text_color = None;
    v.weak_text_alpha = 0.78;
    v.weak_text_color = Some(on_surface_var);
    v.hyperlink_color = primary;
    v.faint_bg_color = surface_low;
    v.extreme_bg_color = p.get(Role::SurfaceContainerLowest);
    v.code_bg_color = surface_high;
    v.warn_fg_color = p.get(Role::Tertiary);
    v.error_fg_color = error;

    v.window_corner_radius = win_radius;
    v.window_shadow = elevation_shadow(3, shadow);
    v.window_fill = surface_c;
    v.window_stroke = Stroke::new(1.0, outline_var);
    v.panel_fill = surface;

    v.menu_corner_radius = menu_radius;
    v.popup_shadow = elevation_shadow(2, shadow);

    v.selection = Selection {
        bg_fill: primary_container,
        stroke: Stroke::new(1.0, on_primary_container),
    };
    v.text_cursor = TextCursorStyle {
        stroke: Stroke::new(2.0, primary),
        ..v.text_cursor
    };

    let hovered_fill = state_layer(surface_high, on_surface, state::HOVER.opacity);
    let pressed_fill = state_layer(secondary_container, on_secondary_container, state::PRESSED.opacity);

    v.widgets = Widgets {
        noninteractive: WidgetVisuals {
            bg_fill: surface,
            weak_bg_fill: surface,
            bg_stroke: Stroke::new(1.0, outline_var),
            corner_radius: ctrl_radius,
            fg_stroke: Stroke::new(1.0, on_surface),
            expansion: 0.0,
        },
        inactive: WidgetVisuals {
            bg_fill: surface_high,
            weak_bg_fill: surface_c,
            bg_stroke: Stroke::new(1.0, outline_var),
            corner_radius: ctrl_radius,
            fg_stroke: Stroke::new(1.0, on_surface),
            expansion: 0.0,
        },
        hovered: WidgetVisuals {
            bg_fill: hovered_fill,
            weak_bg_fill: hovered_fill,
            bg_stroke: Stroke::new(1.0, primary),
            corner_radius: ctrl_radius,
            fg_stroke: Stroke::new(1.5, on_surface),
            expansion: 1.0,
        },
        active: WidgetVisuals {
            bg_fill: pressed_fill,
            weak_bg_fill: secondary_container,
            bg_stroke: Stroke::new(1.5, primary),
            corner_radius: ctrl_radius,
            fg_stroke: Stroke::new(2.0, on_secondary_container),
            expansion: 1.0,
        },
        open: WidgetVisuals {
            bg_fill: surface_highest,
            weak_bg_fill: surface_high,
            bg_stroke: Stroke::new(1.0, outline),
            corner_radius: ctrl_radius,
            fg_stroke: Stroke::new(1.0, on_surface),
            expansion: 0.0,
        },
    };
    let _ = on_primary;
    v
}

/// Builds an [`egui::Style`] (visuals, spacing, text styles, animation time) for `scheme`.
#[must_use]
pub fn style(scheme: &DynamicScheme) -> Style {
    let mut st = Style {
        visuals: visuals(scheme),
        ..Style::default()
    };
    configure_style_metrics(&mut st);
    st
}

/// Builds an [`egui::Style`] for one of the 5 [`YoloThemePreset`]s (`Neutral`, `M3`, `Fluent`, `Apple`, `Aphrody`).
///
/// - `Neutral`: 8px control radii, `#6366F1` primary.
/// - `M3`: 8px small / 16px large M3 shape scale, `#6750A4` tonal spot.
/// - `Fluent`: 4px control radii, 8px window radii, Fluent blue `#0078D4`.
/// - `Apple`: 10px continuous control radii, 14px window radii, Cupertino blue `#007AFF`.
/// - `Aphrody`: 12px control radii, 16px window radii, Byron Love `#17335C` / `#FFD700` / `#7C5CFF`.
#[must_use]
pub fn style_for_preset(preset: YoloThemePreset, dark: bool) -> Style {
    let pal = Palette::from_preset(preset, dark);
    let profile = PresetShapeProfile::from_preset(preset);
    let mut st = Style {
        visuals: visuals_from_palette(&pal, profile),
        ..Style::default()
    };
    configure_style_metrics(&mut st);
    st
}

fn configure_style_metrics(st: &mut Style) {
    // 200 ms standard M3 transition.
    st.animation_time = 0.2;
    st.spacing.item_spacing = egui::vec2(8.0, 8.0);
    st.spacing.window_margin = egui::Margin::same(16);
    st.spacing.button_padding = egui::vec2(16.0, 8.0);
    st.spacing.interact_size = egui::vec2(48.0, 36.0);
    st.spacing.combo_width = 140.0;
    st.text_styles.insert(egui::TextStyle::Small, type_scale::BODY_SMALL);
    st.text_styles.insert(egui::TextStyle::Body, type_scale::BODY_MEDIUM);
    st.text_styles.insert(egui::TextStyle::Button, type_scale::LABEL_LARGE);
    st.text_styles.insert(egui::TextStyle::Heading, type_scale::HEADLINE_SMALL);
    st.text_styles.insert(
        egui::TextStyle::Monospace,
        FontId::new(typography::BODY_MEDIUM.size_dp, FontFamily::Monospace),
    );
}

/// Installs both light and dark styles and palettes on `ctx` from `(seed, variant, contrast)`.
pub fn apply(ctx: &egui::Context, seed: u32, variant: Variant, contrast: f64) {
    let light = DynamicScheme::from_argb(seed, variant, false, contrast);
    let dark = DynamicScheme::from_argb(seed, variant, true, contrast);
    ctx.set_style_of(egui::Theme::Light, style(&light));
    ctx.set_style_of(egui::Theme::Dark, style(&dark));
    ctx.data_mut(|d| {
        d.insert_temp(Palette::data_id(false), Arc::new(Palette::from_scheme(&light)));
        d.insert_temp(Palette::data_id(true), Arc::new(Palette::from_scheme(&dark)));
    });
}

/// Applies one of the 5 built-in [`YoloThemePreset`]s (`Neutral`, `M3`, `Fluent`, `Apple`, `Aphrody`)
/// to `ctx` for both light and dark modes, installing the preset's shape radii and color palette.
pub fn apply_preset(ctx: &egui::Context, preset: YoloThemePreset) {
    let light_style = style_for_preset(preset, false);
    let dark_style = style_for_preset(preset, true);
    let light_pal = Palette::from_preset(preset, false);
    let dark_pal = Palette::from_preset(preset, true);
    ctx.set_style_of(egui::Theme::Light, light_style);
    ctx.set_style_of(egui::Theme::Dark, dark_style);
    ctx.data_mut(|d| {
        d.insert_temp(Palette::data_id(false), Arc::new(light_pal));
        d.insert_temp(Palette::data_id(true), Arc::new(dark_pal));
    });
}

/// Prepends caller-supplied font bytes (`Google Sans` / `Google Sans Text` / `Google Sans Mono`)
/// to egui's proportional and monospace families.
pub fn install_fonts(ctx: &egui::Context, proportional_ttf: Option<&'static [u8]>, mono_ttf: Option<&'static [u8]>) {
    let mut defs = FontDefinitions::default();
    if let Some(bytes) = proportional_ttf {
        defs.font_data.insert(
            "google-sans".into(),
            Arc::new(FontData::from_static(bytes)),
        );
        if let Some(fam) = defs.families.get_mut(&FontFamily::Proportional) {
            fam.insert(0, "google-sans".into());
        }
    }
    if let Some(bytes) = mono_ttf {
        defs.font_data.insert(
            "google-sans-mono".into(),
            Arc::new(FontData::from_static(bytes)),
        );
        if let Some(fam) = defs.families.get_mut(&FontFamily::Monospace) {
            fam.insert(0, "google-sans-mono".into());
        }
    }
    ctx.set_fonts(defs);
}

/// Runtime theme state with a ready-made inspector UI (`seed`, `variant`, `contrast`, `mode`).
#[derive(Clone, Debug)]
pub struct ThemeController {
    /// Seed colour `0xAARRGGBB`.
    pub seed: u32,
    /// Scheme variant.
    pub variant: Variant,
    /// Contrast in `-1.0..=1.0`.
    pub contrast: f64,
    /// Theme preference.
    pub preference: egui::ThemePreference,
    /// Optional active [`YoloThemePreset`].
    pub preset: Option<YoloThemePreset>,
}

impl Default for ThemeController {
    fn default() -> Self {
        Self {
            seed: 0xFF67_50A4,
            variant: Variant::TonalSpot,
            contrast: 0.0,
            preference: egui::ThemePreference::System,
            preset: Some(YoloThemePreset::M3),
        }
    }
}

impl ThemeController {
    /// Creates a controller with `seed` and `TonalSpot`.
    #[must_use]
    pub const fn new(seed: u32) -> Self {
        Self {
            seed,
            variant: Variant::TonalSpot,
            contrast: 0.0,
            preference: egui::ThemePreference::System,
            preset: None,
        }
    }

    /// Creates a controller initialized to one of the 5 [`YoloThemePreset`]s.
    #[must_use]
    pub const fn from_preset(preset: YoloThemePreset) -> Self {
        Self {
            seed: preset.seed_argb(),
            variant: yolo_tokens::presets::variant_for_preset(preset),
            contrast: 0.0,
            preference: egui::ThemePreference::System,
            preset: Some(preset),
        }
    }

    /// Applies the current settings to `ctx`.
    pub fn apply(&self, ctx: &egui::Context) {
        ctx.set_theme(self.preference);
        if let Some(preset) = self.preset {
            apply_preset(ctx, preset);
        } else {
            apply(ctx, self.seed, self.variant, self.contrast);
        }
    }

    /// Draws a compact theme picker (preset, seed colour, variant, contrast, light/dark/system)
    /// and re-applies the theme when anything changes.
    pub fn ui(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;
        ui.horizontal_wrapped(|ui| {
            for preset in YoloThemePreset::ALL {
                if ui
                    .selectable_label(self.preset == Some(preset), preset.display_name())
                    .clicked()
                {
                    self.preset = Some(preset);
                    self.seed = preset.seed_argb();
                    self.variant = yolo_tokens::presets::variant_for_preset(preset);
                    changed = true;
                }
            }
            let mut rgb = [
                (self.seed >> 16) as u8,
                (self.seed >> 8) as u8,
                self.seed as u8,
            ];
            ui.label("Seed");
            if ui.color_edit_button_srgb(&mut rgb).changed() {
                self.seed =
                    0xFF00_0000 | (u32::from(rgb[0]) << 16) | (u32::from(rgb[1]) << 8) | u32::from(rgb[2]);
                self.preset = None;
                changed = true;
            }
            egui::ComboBox::from_label("Variant")
                .selected_text(format!("{:?}", self.variant))
                .show_ui(ui, |ui| {
                    for v in [
                        Variant::TonalSpot,
                        Variant::Content,
                        Variant::Fidelity,
                        Variant::Vibrant,
                        Variant::Expressive,
                        Variant::Neutral,
                        Variant::Monochrome,
                        Variant::Rainbow,
                        Variant::FruitSalad,
                    ] {
                        if ui.selectable_value(&mut self.variant, v, format!("{v:?}")).changed() {
                            self.preset = None;
                            changed = true;
                        }
                    }
                });
            ui.label("Contrast");
            if ui
                .add(egui::Slider::new(&mut self.contrast, -1.0..=1.0).step_by(0.25))
                .changed()
            {
                self.preset = None;
                changed = true;
            }
            for (pref, label) in [
                (egui::ThemePreference::Light, "Light"),
                (egui::ThemePreference::Dark, "Dark"),
                (egui::ThemePreference::System, "System"),
            ] {
                changed |= ui.selectable_value(&mut self.preference, pref, label).changed();
            }
        });
        if changed {
            self.apply(ui.ctx());
        }
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_populates_palette_and_visuals() {
        let ctx = egui::Context::default();
        ctx.set_theme(egui::Theme::Dark);
        apply(&ctx, 0xFF67_50A4, Variant::TonalSpot, 0.0);
        let p = Palette::of(&ctx);
        assert!(p.is_dark);
        assert_ne!(p.get(Role::Primary), p.get(Role::Surface));
        assert_eq!(ctx.global_style().visuals.panel_fill, p.get(Role::Surface));
    }

    #[test]
    fn apply_preset_supports_all_five_presets() {
        let ctx = egui::Context::default();
        ctx.set_theme(egui::Theme::Dark);

        // Fluent: 4px radii, Fluent blue #0078D4
        apply_preset(&ctx, YoloThemePreset::Fluent);
        let fluent_style = ctx.global_style();
        assert_eq!(
            fluent_style.visuals.widgets.inactive.corner_radius,
            CornerRadius::same(4)
        );
        let fluent_pal = Palette::of(&ctx);
        assert_eq!(fluent_pal.preset, Some(YoloThemePreset::Fluent));
        assert_eq!(fluent_pal.get(Role::Primary), color32(0xFF60_CDFF));

        // Apple: 10px continuous radii, Cupertino dark blue #0A84FF
        apply_preset(&ctx, YoloThemePreset::Apple);
        let apple_style = ctx.global_style();
        assert_eq!(
            apple_style.visuals.widgets.inactive.corner_radius,
            CornerRadius::same(10)
        );
        let apple_pal = Palette::of(&ctx);
        assert_eq!(apple_pal.preset, Some(YoloThemePreset::Apple));
        assert_eq!(apple_pal.get(Role::Primary), color32(0xFF0A_84FF));

        // Aphrody: Byron Love #17335C / #91704C
        ctx.set_theme(egui::Theme::Light);
        apply_preset(&ctx, YoloThemePreset::Aphrody);
        let aphrody_light = Palette::of(&ctx);
        assert_eq!(aphrody_light.get(Role::Primary), color32(0xFF17_335C));
        assert_eq!(aphrody_light.get(Role::Secondary), color32(0xFF91_704C));

        // Neutral, M3, Omarchy & Hyprland
        apply_preset(&ctx, YoloThemePreset::Neutral);
        assert_eq!(Palette::of(&ctx).preset, Some(YoloThemePreset::Neutral));
        apply_preset(&ctx, YoloThemePreset::M3);
        assert_eq!(Palette::of(&ctx).preset, Some(YoloThemePreset::M3));
        apply_preset(&ctx, YoloThemePreset::Omarchy);
        assert_eq!(Palette::of(&ctx).preset, Some(YoloThemePreset::Omarchy));
        assert_eq!(
            ctx.global_style().visuals.widgets.inactive.corner_radius,
            CornerRadius::same(0)
        );
        apply_preset(&ctx, YoloThemePreset::Hyprland);
        assert_eq!(Palette::of(&ctx).preset, Some(YoloThemePreset::Hyprland));
        assert_eq!(
            ctx.global_style().visuals.widgets.inactive.corner_radius,
            CornerRadius::same(10)
        );
    }

    #[test]
    fn widgets_run_headless_frame() {
        let ctx = egui::Context::default();
        apply_preset(&ctx, YoloThemePreset::Aphrody);
        let mut sw = true;
        let mut filt = true;
        let mut snacks = widgets::Snackbars::new();
        snacks.show_with_action("Saved", "Undo");
        let mut out = ctx.run_ui(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let _ = ui.add(widgets::Button::filled("Save").icon("+"));
                let _ = ui.add(widgets::Button::tonal("Draft"));
                let _ = ui.add(widgets::Button::outlined("Cancel"));
                let _ = ui.add(widgets::Button::text("More"));
                let _ = ui.add(widgets::Button::elevated("Lift"));
                let _ = ui.add(widgets::Switch::new(&mut sw).label("Dark"));
                let _ = ui.add(widgets::Chip::assist("Help"));
                let _ = ui.add(widgets::Chip::filter("Active", &mut filt));
                let _ = ui.add(widgets::Chip::input("tag"));
                let _ = ui.add(widgets::Fab::new("+", "Create").extended("New"));
            });
            let _ = snacks.ui(ctx);
        });
        out.textures_delta.clear();
    }
}
