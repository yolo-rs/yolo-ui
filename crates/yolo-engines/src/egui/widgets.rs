// SPDX-License-Identifier: Apache-2.0
//! M3 & multi-preset components egui does not ship: common buttons, switch, chips, FAB and snackbar toasts.
//!
//! Every widget reads its colours from the [`Palette`] installed by [`super::apply`] /
//! [`super::apply_preset`] / [`super::ThemeController`], applies the M3 state layers (hover 8 %,
//! focus/pressed 10 %, disabled 38 %/12 %) and reports AccessKit info through
//! [`egui::Response::widget_info`].

use std::time::Duration;

use egui::{
    Align2, Color32, CornerRadius, FontId, Id, Order, Pos2, Rect, Response, Sense, Stroke,
    StrokeKind, Ui, Vec2, Widget, WidgetInfo, WidgetType,
};
use yolo_tokens::{state, typography};

use super::{Palette, Role, elevation_shadow, shapes, state_layer, type_scale, with_alpha};

/// Interaction state of one frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Interaction {
    /// Idle.
    Enabled,
    /// Pointer over.
    Hovered,
    /// Keyboard focus.
    Focused,
    /// Pointer down.
    Pressed,
    /// Disabled.
    Disabled,
}

impl Interaction {
    /// Reads the state from a response.
    #[must_use]
    pub fn of(response: &Response) -> Self {
        if !response.enabled() {
            Self::Disabled
        } else if response.is_pointer_button_down_on() {
            Self::Pressed
        } else if response.has_focus() {
            Self::Focused
        } else if response.hovered() {
            Self::Hovered
        } else {
            Self::Enabled
        }
    }

    /// State-layer opacity (`0` when idle or disabled).
    #[must_use]
    pub fn layer_opacity(self) -> f32 {
        match self {
            Self::Hovered => state::HOVER.opacity,
            Self::Focused => state::FOCUS.opacity,
            Self::Pressed => state::PRESSED.opacity,
            Self::Enabled | Self::Disabled => 0.0,
        }
    }
}

fn label_font() -> FontId {
    type_scale::font(typography::LABEL_LARGE)
}

fn paint_label(ui: &Ui, rect: Rect, text: &str, font: FontId, color: Color32) {
    let galley = ui.painter().layout_no_wrap(text.to_owned(), font, color);
    let pos = rect.center() - galley.size() / 2.0;
    ui.painter().galley(pos, galley, color);
}

fn text_width(ui: &Ui, text: &str, font: FontId) -> f32 {
    ui.painter()
        .layout_no_wrap(text.to_owned(), font, Color32::WHITE)
        .size()
        .x
}

fn focus_ring(ui: &Ui, response: &Response, rect: Rect, radius: CornerRadius, p: &Palette) {
    if response.has_focus() {
        ui.painter().rect_stroke(
            rect.expand(2.0),
            radius,
            Stroke::new(2.0, p.get(Role::Secondary)),
            StrokeKind::Outside,
        );
    }
}

/// M3 common button variants.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ButtonKind {
    /// High emphasis: `primary` container.
    #[default]
    Filled,
    /// Medium emphasis: `secondary-container`.
    Tonal,
    /// Medium emphasis: `outline` stroke.
    Outlined,
    /// Low emphasis: text only.
    Text,
    /// `surface-container-low` with elevation 1.
    Elevated,
}

/// An M3 common button (40 dp, full shape, label-large).
#[must_use = "add it with `ui.add(...)`"]
pub struct Button<'a> {
    kind: ButtonKind,
    label: &'a str,
    icon: Option<&'a str>,
}

impl<'a> Button<'a> {
    /// A button of `kind` labelled `label`.
    pub const fn new(kind: ButtonKind, label: &'a str) -> Self {
        Self {
            kind,
            label,
            icon: None,
        }
    }
    /// Filled button.
    pub const fn filled(label: &'a str) -> Self {
        Self::new(ButtonKind::Filled, label)
    }
    /// Filled tonal button.
    pub const fn tonal(label: &'a str) -> Self {
        Self::new(ButtonKind::Tonal, label)
    }
    /// Outlined button.
    pub const fn outlined(label: &'a str) -> Self {
        Self::new(ButtonKind::Outlined, label)
    }
    /// Text button.
    pub const fn text(label: &'a str) -> Self {
        Self::new(ButtonKind::Text, label)
    }
    /// Elevated button.
    pub const fn elevated(label: &'a str) -> Self {
        Self::new(ButtonKind::Elevated, label)
    }
    /// Leading icon glyph (an icon-font codepoint or emoji), 18 dp.
    pub const fn icon(mut self, glyph: &'a str) -> Self {
        self.icon = Some(glyph);
        self
    }
}

impl Widget for Button<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let p = Palette::of(ui.ctx());
        let font = label_font();
        let icon_w = if self.icon.is_some() { 18.0 + 8.0 } else { 0.0 };
        let pad = match (self.kind, self.icon.is_some()) {
            (ButtonKind::Text, false) => 12.0,
            (ButtonKind::Text, true) => 12.0,
            (_, true) => 16.0,
            _ => 24.0,
        };
        let width = text_width(ui, self.label, font.clone()) + icon_w + pad * 2.0;
        let (rect, response) =
            ui.allocate_exact_size(Vec2::new(width.max(48.0), 40.0), Sense::click());
        response
            .widget_info(|| WidgetInfo::labeled(WidgetType::Button, ui.is_enabled(), self.label));
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let st = Interaction::of(&response);
        let (container, content) = match self.kind {
            ButtonKind::Filled => (Some(p.get(Role::Primary)), p.get(Role::OnPrimary)),
            ButtonKind::Tonal => (
                Some(p.get(Role::SecondaryContainer)),
                p.get(Role::OnSecondaryContainer),
            ),
            ButtonKind::Elevated => (Some(p.get(Role::SurfaceContainerLow)), p.get(Role::Primary)),
            ButtonKind::Outlined | ButtonKind::Text => (None, p.get(Role::Primary)),
        };
        let radius = shapes::FULL;
        let painter = ui.painter();
        let on_surface = p.get(Role::OnSurface);
        let (fill, fg) = if st == Interaction::Disabled {
            (
                container.map(|_| with_alpha(on_surface, state::DISABLED_CONTAINER.opacity)),
                with_alpha(on_surface, state::DISABLED_CONTENT.opacity),
            )
        } else {
            let base = container.unwrap_or(Color32::TRANSPARENT);
            let fill = if container.is_some() {
                Some(state_layer(base, content, st.layer_opacity()))
            } else if st.layer_opacity() > 0.0 {
                Some(with_alpha(content, st.layer_opacity()))
            } else {
                None
            };
            (fill, content)
        };
        if self.kind == ButtonKind::Elevated && st != Interaction::Disabled {
            let level = if st == Interaction::Hovered { 2 } else { 1 };
            painter.add(elevation_shadow(level, p.get(Role::Shadow)).as_shape(rect, radius));
        }
        if let Some(fill) = fill {
            painter.rect_filled(rect, radius, fill);
        }
        if self.kind == ButtonKind::Outlined {
            let stroke = if st == Interaction::Disabled {
                with_alpha(on_surface, state::DISABLED_CONTAINER.opacity)
            } else if st == Interaction::Focused {
                p.get(Role::Primary)
            } else {
                p.get(Role::OutlineVariant)
            };
            painter.rect_stroke(rect, radius, Stroke::new(1.0, stroke), StrokeKind::Inside);
        }
        let mut content_rect = rect.shrink2(Vec2::new(pad, 0.0));
        if let Some(icon) = self.icon {
            let icon_rect = Rect::from_min_size(
                Pos2::new(content_rect.left(), rect.center().y - 9.0),
                Vec2::splat(18.0),
            );
            paint_label(ui, icon_rect, icon, FontId::proportional(18.0), fg);
            content_rect.min.x += icon_w;
        }
        paint_label(ui, content_rect, self.label, font, fg);
        focus_ring(ui, &response, rect, radius, &p);
        response
    }
}

/// M3 switch (52x32 track, 16/24/28 dp handle).
#[must_use = "add it with `ui.add(...)`"]
pub struct Switch<'a> {
    on: &'a mut bool,
    label: Option<&'a str>,
}

impl<'a> Switch<'a> {
    /// A switch bound to `on`.
    pub fn new(on: &'a mut bool) -> Self {
        Self { on, label: None }
    }
    /// Accessible label drawn after the switch.
    pub fn label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }
}

impl Widget for Switch<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let p = Palette::of(ui.ctx());
        let font = type_scale::font(typography::BODY_LARGE);
        let label_w = self
            .label
            .map_or(0.0, |l| 16.0 + text_width(ui, l, font.clone()));
        let (rect, mut response) =
            ui.allocate_exact_size(Vec2::new(52.0 + label_w, 32.0), Sense::click());
        if response.clicked() {
            *self.on = !*self.on;
            response.mark_changed();
        }
        let on = *self.on;
        let label = self.label.unwrap_or("");
        response
            .widget_info(|| WidgetInfo::selected(WidgetType::Checkbox, ui.is_enabled(), on, label));
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let st = Interaction::of(&response);
        let t = ui.ctx().animate_bool_responsive(response.id, on);
        let track = Rect::from_min_size(rect.min, Vec2::new(52.0, 32.0));
        let painter = ui.painter();
        let disabled = st == Interaction::Disabled;
        let on_surface = p.get(Role::OnSurface);
        let (track_fill, track_stroke, handle) = if on {
            (p.get(Role::Primary), None, p.get(Role::OnPrimary))
        } else {
            (
                p.get(Role::SurfaceContainerHighest),
                Some(p.get(Role::Outline)),
                p.get(Role::Outline),
            )
        };
        let alpha = |c: Color32, a: f32| if disabled { with_alpha(c, a) } else { c };
        painter.rect_filled(track, shapes::FULL, alpha(track_fill, 0.12));
        if let Some(s) = track_stroke {
            painter.rect_stroke(
                track,
                shapes::FULL,
                Stroke::new(2.0, alpha(s, 0.12)),
                StrokeKind::Inside,
            );
        }
        let pressed = st == Interaction::Pressed;
        let diameter = if pressed { 28.0 } else { 16.0 + 8.0 * t };
        let cx = egui::lerp((track.left() + 16.0)..=(track.right() - 16.0), t);
        let center = Pos2::new(cx, track.center().y);
        if st.layer_opacity() > 0.0 {
            let layer = if on { p.get(Role::Primary) } else { on_surface };
            painter.circle_filled(center, 20.0, with_alpha(layer, st.layer_opacity()));
        }
        let handle = if on && !disabled && st != Interaction::Enabled {
            p.get(Role::PrimaryContainer)
        } else if !on && !disabled && st != Interaction::Enabled {
            p.get(Role::OnSurfaceVariant)
        } else {
            handle
        };
        painter.circle_filled(center, diameter / 2.0, alpha(handle, 0.38));
        if let Some(text) = self.label {
            let r = Rect::from_min_max(Pos2::new(track.right() + 16.0, rect.top()), rect.max);
            let galley = painter.layout_no_wrap(text.to_owned(), font, alpha(on_surface, 0.38));
            painter.galley(
                Pos2::new(r.left(), r.center().y - galley.size().y / 2.0),
                galley,
                on_surface,
            );
        }
        focus_ring(ui, &response, track, shapes::FULL, &p);
        response
    }
}

/// M3 chip variants.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ChipKind {
    /// Assist chip: an action.
    #[default]
    Assist,
    /// Filter chip: toggles, shows a check when selected.
    Filter,
    /// Suggestion chip.
    Suggestion,
    /// Input chip: user entry with a trailing remove mark.
    Input,
}

/// An M3 chip (32 dp, `small` shape).
#[must_use = "add it with `ui.add(...)`"]
pub struct Chip<'a> {
    kind: ChipKind,
    label: &'a str,
    selected: Option<&'a mut bool>,
}

impl<'a> Chip<'a> {
    /// A chip of `kind`.
    pub const fn new(kind: ChipKind, label: &'a str) -> Self {
        Self {
            kind,
            label,
            selected: None,
        }
    }
    /// Assist chip.
    pub const fn assist(label: &'a str) -> Self {
        Self::new(ChipKind::Assist, label)
    }
    /// Filter chip bound to `selected`.
    pub fn filter(label: &'a str, selected: &'a mut bool) -> Self {
        Self {
            kind: ChipKind::Filter,
            label,
            selected: Some(selected),
        }
    }
    /// Suggestion chip.
    pub const fn suggestion(label: &'a str) -> Self {
        Self::new(ChipKind::Suggestion, label)
    }
    /// Input chip; a click reports removal.
    pub const fn input(label: &'a str) -> Self {
        Self::new(ChipKind::Input, label)
    }
}

impl Widget for Chip<'_> {
    fn ui(mut self, ui: &mut Ui) -> Response {
        let p = Palette::of(ui.ctx());
        let font = type_scale::font(typography::LABEL_LARGE);
        let selected = self.selected.as_deref().copied().unwrap_or(false);
        let lead = if self.kind == ChipKind::Filter && selected {
            18.0 + 8.0
        } else {
            0.0
        };
        let trail = if self.kind == ChipKind::Input {
            18.0 + 8.0
        } else {
            0.0
        };
        let w = text_width(ui, self.label, font.clone()) + 32.0 + lead + trail;
        let (rect, mut response) = ui.allocate_exact_size(Vec2::new(w, 32.0), Sense::click());
        if response.clicked() {
            if let Some(sel) = self.selected.as_deref_mut() {
                *sel = !*sel;
                response.mark_changed();
            }
        }
        let now = self.selected.as_deref().copied().unwrap_or(false);
        if self.kind == ChipKind::Filter {
            response.widget_info(|| {
                WidgetInfo::selected(WidgetType::Checkbox, ui.is_enabled(), now, self.label)
            });
        } else {
            response.widget_info(|| {
                WidgetInfo::labeled(WidgetType::Button, ui.is_enabled(), self.label)
            });
        }
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let st = Interaction::of(&response);
        let painter = ui.painter();
        let on_surface = p.get(Role::OnSurface);
        let (base, fg, stroke) = if now {
            (
                Some(p.get(Role::SecondaryContainer)),
                p.get(Role::OnSecondaryContainer),
                None,
            )
        } else {
            let fg = if self.kind == ChipKind::Assist {
                on_surface
            } else {
                p.get(Role::OnSurfaceVariant)
            };
            (None, fg, Some(p.get(Role::OutlineVariant)))
        };
        let fg = if st == Interaction::Disabled {
            with_alpha(on_surface, 0.38)
        } else {
            fg
        };
        if let Some(b) = base {
            painter.rect_filled(rect, shapes::SMALL, state_layer(b, fg, st.layer_opacity()));
        } else if st.layer_opacity() > 0.0 {
            painter.rect_filled(rect, shapes::SMALL, with_alpha(fg, st.layer_opacity()));
        }
        if let Some(s) = stroke {
            painter.rect_stroke(rect, shapes::SMALL, Stroke::new(1.0, s), StrokeKind::Inside);
        }
        let mut inner = rect.shrink2(Vec2::new(16.0, 0.0));
        if lead > 0.0 {
            let r = Rect::from_min_size(
                Pos2::new(rect.left() + 8.0, rect.center().y - 9.0),
                Vec2::splat(18.0),
            );
            paint_label(ui, r, "\u{2714}", FontId::proportional(14.0), fg);
            inner.min.x = r.right();
        }
        if trail > 0.0 {
            let r = Rect::from_min_size(
                Pos2::new(rect.right() - 26.0, rect.center().y - 9.0),
                Vec2::splat(18.0),
            );
            paint_label(ui, r, "\u{2715}", FontId::proportional(14.0), fg);
            inner.max.x = r.left();
        }
        paint_label(ui, inner, self.label, font, fg);
        focus_ring(ui, &response, rect, shapes::SMALL, &p);
        response
    }
}

/// FAB sizes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FabSize {
    /// 40 dp, `medium` shape.
    Small,
    /// 56 dp, `large` shape.
    #[default]
    Regular,
    /// 96 dp, `extra-large` shape.
    Large,
}

/// FAB colour roles.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FabColor {
    /// `primary-container` (default).
    #[default]
    Primary,
    /// `secondary-container`.
    Secondary,
    /// `tertiary-container`.
    Tertiary,
    /// `surface-container-high` with `primary` icon.
    Surface,
}

/// M3 floating action button, optionally extended with a label.
#[must_use = "add it with `ui.add(...)`"]
pub struct Fab<'a> {
    icon: &'a str,
    label: Option<&'a str>,
    size: FabSize,
    color: FabColor,
    accessible: &'a str,
}

impl<'a> Fab<'a> {
    /// A FAB showing `icon`; `accessible` names it for screen readers.
    pub const fn new(icon: &'a str, accessible: &'a str) -> Self {
        Self {
            icon,
            label: None,
            size: FabSize::Regular,
            color: FabColor::Primary,
            accessible,
        }
    }
    /// Extended FAB with a visible label.
    pub const fn extended(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }
    /// Size.
    pub const fn size(mut self, size: FabSize) -> Self {
        self.size = size;
        self
    }
    /// Colour role.
    pub const fn color(mut self, color: FabColor) -> Self {
        self.color = color;
        self
    }
}

impl Widget for Fab<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let p = Palette::of(ui.ctx());
        let (side, radius, icon_size) = match self.size {
            FabSize::Small => (40.0, shapes::MEDIUM, 24.0),
            FabSize::Regular => (56.0, shapes::LARGE, 24.0),
            FabSize::Large => (96.0, shapes::EXTRA_LARGE, 36.0),
        };
        let font = label_font();
        let w = self.label.map_or(side, |l| {
            16.0 + icon_size + 12.0 + text_width(ui, l, font.clone()) + 20.0
        });
        let (rect, response) = ui.allocate_exact_size(Vec2::new(w.max(side), side), Sense::click());
        let name = self.label.unwrap_or(self.accessible);
        response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, ui.is_enabled(), name));
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let st = Interaction::of(&response);
        let (bg, fg) = match self.color {
            FabColor::Primary => (
                p.get(Role::PrimaryContainer),
                p.get(Role::OnPrimaryContainer),
            ),
            FabColor::Secondary => (
                p.get(Role::SecondaryContainer),
                p.get(Role::OnSecondaryContainer),
            ),
            FabColor::Tertiary => (
                p.get(Role::TertiaryContainer),
                p.get(Role::OnTertiaryContainer),
            ),
            FabColor::Surface => (p.get(Role::SurfaceContainerHigh), p.get(Role::Primary)),
        };
        let painter = ui.painter();
        let level = if st == Interaction::Hovered { 4 } else { 3 };
        painter.add(elevation_shadow(level, p.get(Role::Shadow)).as_shape(rect, radius));
        painter.rect_filled(rect, radius, state_layer(bg, fg, st.layer_opacity()));
        let icon_rect = if self.label.is_some() {
            Rect::from_center_size(
                Pos2::new(rect.left() + 16.0 + icon_size / 2.0, rect.center().y),
                Vec2::splat(icon_size),
            )
        } else {
            Rect::from_center_size(rect.center(), Vec2::splat(icon_size))
        };
        paint_label(
            ui,
            icon_rect,
            self.icon,
            FontId::proportional(icon_size),
            fg,
        );
        if let Some(label) = self.label {
            let r = Rect::from_min_max(
                Pos2::new(icon_rect.right() + 12.0, rect.top()),
                Pos2::new(rect.right() - 20.0, rect.bottom()),
            );
            paint_label(ui, r, label, font, fg);
        }
        focus_ring(ui, &response, rect, radius, &p);
        response
    }
}

#[derive(Clone, Debug)]
struct Toast {
    id: u64,
    message: String,
    action: Option<String>,
    shown_at: Option<f64>,
    duration: Duration,
}

/// Queue of M3 snackbars, shown one at a time at the bottom centre of the screen.
#[derive(Clone, Debug, Default)]
pub struct Snackbars {
    queue: std::collections::VecDeque<Toast>,
    next_id: u64,
}

/// Outcome of a snackbar this frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SnackbarEvent {
    /// The action of snackbar `id` was clicked.
    Action(u64),
    /// Snackbar `id` was dismissed (timeout or close).
    Dismissed(u64),
}

impl Snackbars {
    /// An empty queue.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Queues a 4 s snackbar; returns its id.
    pub fn show(&mut self, message: impl Into<String>) -> u64 {
        self.push(message.into(), None, Duration::from_secs(4))
    }

    /// Queues a 10 s snackbar with an action button; returns its id.
    pub fn show_with_action(
        &mut self,
        message: impl Into<String>,
        action: impl Into<String>,
    ) -> u64 {
        self.push(message.into(), Some(action.into()), Duration::from_secs(10))
    }

    fn push(&mut self, message: String, action: Option<String>, duration: Duration) -> u64 {
        self.next_id += 1;
        self.queue.push_back(Toast {
            id: self.next_id,
            message,
            action,
            shown_at: None,
            duration,
        });
        self.next_id
    }

    /// Draws the current snackbar; call once per frame after the panels.
    pub fn ui(&mut self, ctx: &egui::Context) -> Option<SnackbarEvent> {
        let p = Palette::of(ctx);
        let now = ctx.input(|i| i.time);
        let toast = self.queue.front_mut()?;
        let shown = *toast.shown_at.get_or_insert(now);
        let elapsed = now - shown;
        if elapsed >= toast.duration.as_secs_f64() {
            let id = toast.id;
            self.queue.pop_front();
            ctx.request_repaint();
            return Some(SnackbarEvent::Dismissed(id));
        }
        ctx.request_repaint_after(Duration::from_millis(100));
        let fade = ctx.animate_bool_with_time(Id::new(("m3-snackbar", toast.id)), true, 0.15);
        let id = toast.id;
        let message = toast.message.clone();
        let action = toast.action.clone();
        let bg = p.get(Role::InverseSurface);
        let fg = p.get(Role::InverseOnSurface);
        let accent = p.get(Role::InversePrimary);
        let mut event = None;
        egui::Area::new(Id::new("m3-snackbar-area"))
            .order(Order::Foreground)
            .anchor(
                Align2::CENTER_BOTTOM,
                Vec2::new(0.0, -16.0 + 16.0 * (1.0 - fade)),
            )
            .interactable(true)
            .show(ctx, |ui| {
                let frame = egui::Frame::new()
                    .fill(bg)
                    .corner_radius(shapes::EXTRA_SMALL)
                    .shadow(elevation_shadow(3, p.get(Role::Shadow)))
                    .inner_margin(egui::Margin::symmetric(16, 4));
                frame.show(ui, |ui| {
                    ui.set_min_height(40.0);
                    ui.set_max_width(560.0);
                    ui.horizontal_centered(|ui| {
                        ui.label(
                            egui::RichText::new(&message)
                                .color(fg)
                                .font(type_scale::font(typography::BODY_MEDIUM)),
                        );
                        if let Some(action) = &action {
                            ui.add_space(8.0);
                            let label =
                                egui::RichText::new(action).color(accent).font(label_font());
                            if ui.add(egui::Button::new(label).frame(false)).clicked() {
                                event = Some(SnackbarEvent::Action(id));
                            }
                        }
                    });
                });
            });
        if event.is_some() {
            self.queue.pop_front();
        }
        event
    }
}
