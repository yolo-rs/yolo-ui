// SPDX-License-Identifier: Apache-2.0
//! eframe/egui backend: viewport state, toolkit-drawn menu bar, about window
//! and theme.

use eframe::egui;
use egui::{Pos2, Vec2, ViewportBuilder, ViewportCommand};

use crate::menu::{Menu, MenuEvent, MenuItem, Role};
use crate::theme::{ColorScheme, SystemTheme};
use crate::{AppInfo, WindowState, WindowStateStore};

/// Applies a stored state to a viewport builder.
pub fn viewport_with_state(mut builder: ViewportBuilder, state: &WindowState) -> ViewportBuilder {
    builder = builder.with_inner_size(Vec2::new(state.width, state.height));
    if let (Some(x), Some(y)) = (state.x, state.y) {
        builder = builder.with_position(Pos2::new(x, y));
    }
    builder.with_maximized(state.maximized).with_fullscreen(state.fullscreen)
}

/// Reads the current viewport geometry, in points.
pub fn capture_state(ctx: &egui::Context) -> Option<WindowState> {
    ctx.input(|input| {
        let info = input.viewport();
        let inner = info.inner_rect?;
        let outer = info.outer_rect.unwrap_or(inner);
        Some(WindowState {
            x: Some(outer.min.x),
            y: Some(outer.min.y),
            width: inner.width(),
            height: inner.height(),
            maximized: info.maximized.unwrap_or(false),
            fullscreen: info.fullscreen.unwrap_or(false),
        })
    })
}

/// Tracks one viewport and saves its state when it settles.
pub struct WindowStateTracker {
    id: String,
    store: WindowStateStore,
    last_change: Option<f64>,
}

impl WindowStateTracker {
    /// Seconds a geometry must stay unchanged before it is written.
    pub const SETTLE_SECONDS: f64 = 0.75;

    pub fn new(id: impl Into<String>, store: WindowStateStore) -> Self {
        Self { id: id.into(), store, last_change: None }
    }

    /// Stored state, or `default`.
    pub fn initial(&self, default: WindowState) -> WindowState {
        self.store.get_or(&self.id, default)
    }

    /// Call every frame (from `App::logic`).
    pub fn update(&mut self, ctx: &egui::Context) {
        let now = ctx.input(|i| i.time);
        if let Some(state) = capture_state(ctx)
            && self.store.observe(&self.id, state)
        {
            self.last_change = Some(now);
        }
        if let Some(changed) = self.last_change {
            if now - changed >= Self::SETTLE_SECONDS {
                let _ = self.store.save();
                self.last_change = None;
            } else {
                ctx.request_repaint_after(std::time::Duration::from_secs_f64(Self::SETTLE_SECONDS));
            }
        }
    }

    /// Writes pending changes (call from `App::on_exit`).
    pub fn flush(&mut self) {
        let _ = self.store.save();
    }
}

/// Draws `menu` as an egui menu bar. Roles that map to viewport commands
/// (quit, close, minimize, maximize, fullscreen) are executed; every
/// activation is returned for the app.
pub fn menu_bar(ui: &mut egui::Ui, menu: &mut Menu) -> Vec<MenuEvent> {
    let mut activated = Vec::new();
    egui::MenuBar::new().ui(ui, |ui| {
        draw_items(ui, &menu.items, &mut activated);
    });
    let ctx = ui.ctx().clone();
    activated.into_iter().map(|id| handle_activation(&ctx, menu, &id)).collect()
}

/// Draws `menu` as a context menu on `response`.
pub fn context_menu(response: &egui::Response, menu: &mut Menu) -> Vec<MenuEvent> {
    let mut activated = Vec::new();
    response.context_menu(|ui| draw_items(ui, &menu.items, &mut activated));
    activated.into_iter().map(|id| handle_activation(&response.ctx, menu, &id)).collect()
}

fn handle_activation(ctx: &egui::Context, menu: &mut Menu, id: &str) -> MenuEvent {
    let event = menu.activate(id);
    match event.role {
        Some(Role::Quit) | Some(Role::CloseWindow) => ctx.send_viewport_cmd(ViewportCommand::Close),
        Some(Role::Minimize) => ctx.send_viewport_cmd(ViewportCommand::Minimized(true)),
        Some(Role::Maximize) => {
            let maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
            ctx.send_viewport_cmd(ViewportCommand::Maximized(!maximized));
        }
        Some(Role::Fullscreen) => {
            let fullscreen = ctx.input(|i| i.viewport().fullscreen.unwrap_or(false));
            ctx.send_viewport_cmd(ViewportCommand::Fullscreen(!fullscreen));
        }
        _ => {}
    }
    event
}

fn draw_items(ui: &mut egui::Ui, items: &[MenuItem], activated: &mut Vec<String>) {
    for item in items {
        match item {
            MenuItem::Action { id, label, accelerator, enabled } => {
                let button = egui::Button::new(label.as_str()).shortcut_text(accelerator_text(accelerator.as_deref()));
                if ui.add_enabled(*enabled, button).clicked() {
                    activated.push(id.clone());
                    ui.close();
                }
            }
            MenuItem::Check { id, label, checked, enabled, .. } => {
                let mut value = *checked;
                if ui.add_enabled(*enabled, egui::Checkbox::new(&mut value, label.as_str())).clicked() {
                    activated.push(id.clone());
                    ui.close();
                }
            }
            MenuItem::Radio { id, label, checked, enabled, .. } => {
                if ui.add_enabled(*enabled, egui::RadioButton::new(*checked, label.as_str())).clicked() {
                    activated.push(id.clone());
                    ui.close();
                }
            }
            MenuItem::Submenu { label, items, enabled } => {
                ui.add_enabled_ui(*enabled, |ui| {
                    ui.menu_button(label.as_str(), |ui| draw_items(ui, items, activated));
                });
            }
            MenuItem::Separator => {
                ui.separator();
            }
            MenuItem::Role { role } => {
                if role.is_macos_only() {
                    continue;
                }
                let button = egui::Button::new(role.label()).shortcut_text(accelerator_text(role.accelerator()));
                if ui.add(button).clicked() {
                    activated.push(role.id().to_owned());
                    ui.close();
                }
            }
        }
    }
}

fn accelerator_text(accelerator: Option<&str>) -> String {
    accelerator
        .map(|a| if cfg!(target_os = "macos") { a.replace("CmdOrCtrl", "Cmd") } else { a.replace("CmdOrCtrl", "Ctrl") })
        .unwrap_or_default()
}

/// About window drawn with egui; `open` is cleared when closed.
pub fn about_window(ctx: &egui::Context, app: &AppInfo, open: &mut bool) {
    egui::Window::new(format!("About {}", app.name))
        .open(open)
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            ui.heading(&app.name);
            ui.label(format!("Version {}", app.version));
            if let Some(description) = &app.description {
                ui.add_space(4.0);
                ui.label(description);
            }
            if !app.authors.is_empty() {
                ui.add_space(4.0);
                ui.label(format!("Authors: {}", app.authors.join(", ")));
            }
            if let Some(copyright) = &app.copyright {
                ui.label(copyright);
            }
            if let Some(license) = &app.license {
                ui.label(format!("License: {license}"));
            }
            if let Some(website) = &app.website {
                ui.hyperlink(website);
            }
        });
}

/// Applies the detected scheme to egui (follows the system when unknown).
pub fn apply_theme(ctx: &egui::Context, theme: &SystemTheme) {
    ctx.set_theme(match theme.scheme {
        ColorScheme::Dark => egui::ThemePreference::Dark,
        ColorScheme::Light => egui::ThemePreference::Light,
        ColorScheme::Unknown => egui::ThemePreference::System,
    });
}
