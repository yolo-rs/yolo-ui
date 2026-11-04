// SPDX-License-Identifier: Apache-2.0
//! Iced 0.13 / 0.14 + `libcosmic` widget style structs (`button`, `container`, `text_input`,
//! `toggler`, `checkbox`, `radio`, `slider`, `pick_list`, `menu`, `scrollable`) built directly
//! from [`super::Palette`] and the 5 [`yolo_shared::YoloThemePreset`]s.
//!
//! All style functions are pure functions of `(&Palette, Status)` so they work with custom
//! `Theme` types, `iced::Theme::Custom`, or a `libcosmic` app holding a `Palette` in its model.

use serde::{Deserialize, Serialize};
use yolo_tokens::state;

use super::{elevation_shadow, shapes, state_layer, with_alpha, Background, Border, Color, Palette, Role, Shadow};

/// Widget interaction status matching `iced::widget::button::Status` / `toggler::Status`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Status {
    /// Idle.
    Active,
    /// Pointer over.
    Hovered,
    /// Pointer down or keyboard activation.
    Pressed,
    /// Keyboard focus.
    Focused,
    /// Disabled.
    Disabled,
}

impl Status {
    /// M3 state-layer opacity (`0.08` hover, `0.10` focus/press).
    #[must_use]
    pub fn layer_opacity(self) -> f32 {
        match self {
            Self::Hovered => state::HOVER.opacity,
            Self::Focused => state::FOCUS.opacity,
            Self::Pressed => state::PRESSED.opacity,
            Self::Active | Self::Disabled => 0.0,
        }
    }
}

/// Appearance of a button (`iced::widget::button::Style`).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ButtonStyle {
    /// Fill.
    pub background: Option<Background>,
    /// Label / icon colour.
    pub text_color: Color,
    /// Border.
    pub border: Border,
    /// Drop shadow.
    pub shadow: Shadow,
}

/// M3 common button variants.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ButtonKind {
    /// `primary` fill, pill shape.
    #[default]
    Filled,
    /// `secondary-container` fill, pill shape.
    Tonal,
    /// `surface-container-low` fill + elevation 1, pill shape.
    Elevated,
    /// `outline` 1 dp border, pill shape.
    Outlined,
    /// Text-only, pill shape.
    Text,
    /// `error` fill for destructive actions.
    Destructive,
}

/// Computes the M3 [`ButtonStyle`] for `(kind, status)`.
#[must_use]
pub fn button(p: &Palette, kind: ButtonKind, status: Status) -> ButtonStyle {
    let on_surface = p.get(Role::OnSurface);
    let radius = p.button_radius();
    if status == Status::Disabled {
        let bg = matches!(
            kind,
            ButtonKind::Filled | ButtonKind::Tonal | ButtonKind::Elevated | ButtonKind::Destructive
        )
        .then(|| Background::Color(with_alpha(on_surface, state::DISABLED_CONTAINER.opacity)));
        let stroke = if kind == ButtonKind::Outlined {
            Border {
                color: with_alpha(on_surface, state::DISABLED_CONTAINER.opacity),
                width: 1.0,
                radius,
            }
        } else {
            Border {
                radius,
                ..Border::default()
            }
        };
        return ButtonStyle {
            background: bg,
            text_color: with_alpha(on_surface, state::DISABLED_CONTENT.opacity),
            border: stroke,
            shadow: Shadow::default(),
        };
    }

    let (container, content) = match kind {
        ButtonKind::Filled => (Some(p.get(Role::Primary)), p.get(Role::OnPrimary)),
        ButtonKind::Tonal => (
            Some(p.get(Role::SecondaryContainer)),
            p.get(Role::OnSecondaryContainer),
        ),
        ButtonKind::Elevated => (Some(p.get(Role::SurfaceContainerLow)), p.get(Role::Primary)),
        ButtonKind::Destructive => (Some(p.get(Role::Error)), p.get(Role::OnError)),
        ButtonKind::Outlined | ButtonKind::Text => (None, p.get(Role::Primary)),
    };
    let op = status.layer_opacity();
    let background = match container {
        Some(base) => Some(Background::Color(state_layer(base, content, op))),
        None if op > 0.0 => Some(Background::Color(with_alpha(content, op))),
        None => None,
    };
    let border = match kind {
        ButtonKind::Outlined => Border {
            color: if status == Status::Focused {
                p.get(Role::Primary)
            } else {
                p.get(Role::OutlineVariant)
            },
            width: 1.0,
            radius,
        },
        _ => Border {
            radius,
            ..Border::default()
        },
    };
    let shadow = if kind == ButtonKind::Elevated {
        elevation_shadow(if status == Status::Hovered { 2 } else { 1 }, p.get(Role::Shadow))
    } else {
        Shadow::default()
    };
    ButtonStyle {
        background,
        text_color: content,
        border,
        shadow,
    }
}

/// Appearance of a container (`iced::widget::container::Style`).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ContainerStyle {
    /// Text override.
    pub text_color: Option<Color>,
    /// Fill.
    pub background: Option<Background>,
    /// Border.
    pub border: Border,
    /// Shadow.
    pub shadow: Shadow,
}

/// M3 card / surface container variants.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SurfaceKind {
    /// Window / page background (`surface`).
    #[default]
    Window,
    /// Navigation rail / drawer (`surface-container-low`).
    Sidebar,
    /// Filled card (`surface-container-highest`, `medium` shape).
    FilledCard,
    /// Elevated card (`surface-container-low`, `medium` shape, elevation 1).
    ElevatedCard,
    /// Outlined card (`surface`, `outline-variant` 1 dp border, `medium` shape).
    OutlinedCard,
    /// Modal dialog (`surface-container-high`, `extra-large` shape, elevation 3).
    Dialog,
    /// Snackbar (`inverse-surface`, `extra-small` shape, elevation 3).
    Snackbar,
}

/// Computes the M3 [`ContainerStyle`] for `kind`.
#[must_use]
pub fn container(p: &Palette, kind: SurfaceKind) -> ContainerStyle {
    let sh = p.get(Role::Shadow);
    let on = Some(p.get(Role::OnSurface));
    let none = Border::default();
    let r = |radius| Border { radius, ..none };
    match kind {
        SurfaceKind::Window => ContainerStyle {
            text_color: on,
            background: Some(Background::Color(p.get(Role::Surface))),
            border: none,
            shadow: Shadow::default(),
        },
        SurfaceKind::Sidebar => ContainerStyle {
            text_color: on,
            background: Some(Background::Color(p.get(Role::SurfaceContainerLow))),
            border: none,
            shadow: Shadow::default(),
        },
        SurfaceKind::FilledCard => ContainerStyle {
            text_color: on,
            background: Some(Background::Color(p.get(Role::SurfaceContainerHighest))),
            border: r(shapes::MEDIUM),
            shadow: Shadow::default(),
        },
        SurfaceKind::ElevatedCard => ContainerStyle {
            text_color: on,
            background: Some(Background::Color(p.get(Role::SurfaceContainerLow))),
            border: r(shapes::MEDIUM),
            shadow: elevation_shadow(1, sh),
        },
        SurfaceKind::OutlinedCard => ContainerStyle {
            text_color: on,
            background: Some(Background::Color(p.get(Role::Surface))),
            border: Border {
                color: p.get(Role::OutlineVariant),
                width: 1.0,
                radius: shapes::MEDIUM,
            },
            shadow: Shadow::default(),
        },
        SurfaceKind::Dialog => ContainerStyle {
            text_color: on,
            background: Some(Background::Color(p.get(Role::SurfaceContainerHigh))),
            border: r(shapes::EXTRA_LARGE),
            shadow: elevation_shadow(3, sh),
        },
        SurfaceKind::Snackbar => ContainerStyle {
            text_color: Some(p.get(Role::InverseOnSurface)),
            background: Some(Background::Color(p.get(Role::InverseSurface))),
            border: r(shapes::EXTRA_SMALL),
            shadow: elevation_shadow(3, sh),
        },
    }
}

/// Appearance of a text input (`iced::widget::text_input::Style`).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct TextInputStyle {
    /// Fill.
    pub background: Background,
    /// Border.
    pub border: Border,
    /// Leading/trailing icon colour.
    pub icon: Color,
    /// Placeholder text colour.
    pub placeholder: Color,
    /// Entered text colour.
    pub value: Color,
    /// Selection highlight colour.
    pub selection: Color,
}

/// Outlined M3 text field style.
#[must_use]
pub fn text_input(p: &Palette, status: Status) -> TextInputStyle {
    let on = p.get(Role::OnSurface);
    let var = p.get(Role::OnSurfaceVariant);
    let (border_color, width) = match status {
        Status::Focused | Status::Pressed => (p.get(Role::Primary), 2.0),
        Status::Hovered => (on, 1.0),
        Status::Disabled => (with_alpha(on, 0.12), 1.0),
        Status::Active => (p.get(Role::Outline), 1.0),
    };
    let fg = if status == Status::Disabled {
        with_alpha(on, 0.38)
    } else {
        on
    };
    TextInputStyle {
        background: Background::Color(p.get(Role::SurfaceContainerLowest)),
        border: Border {
            color: border_color,
            width,
            radius: shapes::EXTRA_SMALL,
        },
        icon: var,
        placeholder: var,
        value: fg,
        selection: p.get(Role::PrimaryContainer),
    }
}

/// Appearance of a switch (`iced::widget::toggler::Style`).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct TogglerStyle {
    /// Track fill.
    pub background: Color,
    /// Track border width.
    pub background_border_width: f32,
    /// Track border colour.
    pub background_border_color: Color,
    /// Handle fill.
    pub foreground: Color,
    /// Handle border width.
    pub foreground_border_width: f32,
    /// Handle border colour.
    pub foreground_border_color: Color,
}

/// M3 switch style for `(is_toggled, status)`.
#[must_use]
pub fn toggler(p: &Palette, is_toggled: bool, status: Status) -> TogglerStyle {
    let hovered = matches!(status, Status::Hovered | Status::Focused | Status::Pressed);
    if is_toggled {
        let fg = if hovered {
            p.get(Role::PrimaryContainer)
        } else {
            p.get(Role::OnPrimary)
        };
        TogglerStyle {
            background: p.get(Role::Primary),
            background_border_width: 0.0,
            background_border_color: Color::TRANSPARENT,
            foreground: fg,
            foreground_border_width: 0.0,
            foreground_border_color: Color::TRANSPARENT,
        }
    } else {
        let fg = if hovered {
            p.get(Role::OnSurfaceVariant)
        } else {
            p.get(Role::Outline)
        };
        TogglerStyle {
            background: p.get(Role::SurfaceContainerHighest),
            background_border_width: 2.0,
            background_border_color: p.get(Role::Outline),
            foreground: fg,
            foreground_border_width: 0.0,
            foreground_border_color: Color::TRANSPARENT,
        }
    }
}
