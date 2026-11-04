// SPDX-License-Identifier: Apache-2.0
//! Tauri 2 backend: the [`Menu`] model as `tauri::menu`, and window state for
//! `tauri::WebviewWindow`.
//!
//! Forward `RunEvent::MenuEvent` / `App::on_menu_event` ids to
//! [`Menu::activate`]; call [`sync_checks`] afterwards to mirror radio groups.

use tauri::menu::{
    AboutMetadata, CheckMenuItem, IsMenuItem, Menu as TauriMenu, MenuItem as TauriItem, MenuItemKind, PredefinedMenuItem, Submenu,
};
use tauri::{LogicalPosition, LogicalSize, Manager, Runtime, WebviewWindow};

use crate::menu::{Menu, MenuItem, Role};
use crate::{AppInfo, WindowState};

fn about_metadata(app: &AppInfo) -> AboutMetadata<'static> {
    AboutMetadata {
        name: Some(app.name.clone()),
        version: Some(app.version.clone()),
        authors: (!app.authors.is_empty()).then(|| app.authors.clone()),
        comments: app.description.clone(),
        copyright: app.copyright.clone(),
        license: app.license.clone(),
        website: app.website.clone(),
        ..Default::default()
    }
}

fn predefined<R: Runtime, M: Manager<R>>(manager: &M, role: Role, app: Option<&AppInfo>) -> tauri::Result<Option<PredefinedMenuItem<R>>> {
    Ok(Some(match role {
        Role::About => PredefinedMenuItem::about(manager, None, app.map(about_metadata))?,
        Role::Quit => PredefinedMenuItem::quit(manager, None)?,
        Role::Undo => PredefinedMenuItem::undo(manager, None)?,
        Role::Redo => PredefinedMenuItem::redo(manager, None)?,
        Role::Cut => PredefinedMenuItem::cut(manager, None)?,
        Role::Copy => PredefinedMenuItem::copy(manager, None)?,
        Role::Paste => PredefinedMenuItem::paste(manager, None)?,
        Role::SelectAll => PredefinedMenuItem::select_all(manager, None)?,
        Role::Minimize => PredefinedMenuItem::minimize(manager, None)?,
        Role::Maximize => PredefinedMenuItem::maximize(manager, None)?,
        Role::Fullscreen => PredefinedMenuItem::fullscreen(manager, None)?,
        Role::CloseWindow => PredefinedMenuItem::close_window(manager, None)?,
        Role::Hide if cfg!(target_os = "macos") => PredefinedMenuItem::hide(manager, None)?,
        Role::HideOthers if cfg!(target_os = "macos") => PredefinedMenuItem::hide_others(manager, None)?,
        Role::ShowAll if cfg!(target_os = "macos") => PredefinedMenuItem::show_all(manager, None)?,
        Role::Services if cfg!(target_os = "macos") => PredefinedMenuItem::services(manager, None)?,
        _ => return Ok(None),
    }))
}

fn build_item<R: Runtime, M: Manager<R>>(
    manager: &M,
    item: &MenuItem,
    app: Option<&AppInfo>,
) -> tauri::Result<Option<Box<dyn IsMenuItem<R>>>> {
    Ok(Some(match item {
        MenuItem::Action { id, label, accelerator, enabled } => {
            Box::new(TauriItem::with_id(manager, id.as_str(), label, *enabled, accelerator.as_deref())?)
        }
        MenuItem::Check { id, label, checked, accelerator, enabled } | MenuItem::Radio { id, label, checked, accelerator, enabled, .. } => {
            Box::new(CheckMenuItem::with_id(manager, id.as_str(), label, *enabled, *checked, accelerator.as_deref())?)
        }
        MenuItem::Submenu { label, items, enabled } => {
            let submenu = Submenu::new(manager, label, *enabled)?;
            for child in items {
                if let Some(built) = build_item(manager, child, app)? {
                    submenu.append(built.as_ref())?;
                }
            }
            Box::new(submenu)
        }
        MenuItem::Separator => Box::new(PredefinedMenuItem::separator(manager)?),
        MenuItem::Role { role } => match predefined(manager, *role, app)? {
            Some(item) => Box::new(item),
            None => return Ok(None),
        },
    }))
}

/// Builds a `tauri::menu::Menu` from the model.
pub fn build_menu<R: Runtime, M: Manager<R>>(manager: &M, model: &Menu, app: Option<&AppInfo>) -> tauri::Result<TauriMenu<R>> {
    let menu = TauriMenu::new(manager)?;
    for item in &model.items {
        if let Some(built) = build_item(manager, item, app)? {
            menu.append(built.as_ref())?;
        }
    }
    Ok(menu)
}

/// Mirrors the model's check/radio state into a built Tauri menu.
pub fn sync_checks<R: Runtime>(menu: &TauriMenu<R>, model: &Menu) -> tauri::Result<()> {
    fn walk<R: Runtime>(items: Vec<MenuItemKind<R>>, model: &Menu) -> tauri::Result<()> {
        for item in items {
            match item {
                MenuItemKind::Check(check) => {
                    if let Some(checked) = model.is_checked(&check.id().0) {
                        check.set_checked(checked)?;
                    }
                }
                MenuItemKind::Submenu(submenu) => walk(submenu.items()?, model)?,
                _ => {}
            }
        }
        Ok(())
    }
    walk(menu.items()?, model)
}

/// Applies a stored state to a Tauri window.
pub fn apply_state<R: Runtime>(window: &WebviewWindow<R>, state: &WindowState) -> tauri::Result<()> {
    window.set_size(LogicalSize::new(state.width as f64, state.height as f64))?;
    if let (Some(x), Some(y)) = (state.x, state.y) {
        window.set_position(LogicalPosition::new(x as f64, y as f64))?;
    }
    if state.maximized {
        window.maximize()?;
    }
    if state.fullscreen {
        window.set_fullscreen(true)?;
    }
    Ok(())
}

/// Reads a Tauri window's state in logical pixels.
pub fn capture_state<R: Runtime>(window: &WebviewWindow<R>) -> tauri::Result<WindowState> {
    let scale = window.scale_factor()?;
    let size: LogicalSize<f64> = window.inner_size()?.to_logical(scale);
    let position: LogicalPosition<f64> = window.outer_position()?.to_logical(scale);
    Ok(WindowState {
        x: Some(position.x as f32),
        y: Some(position.y as f32),
        width: size.width as f32,
        height: size.height as f32,
        maximized: window.is_maximized()?,
        fullscreen: window.is_fullscreen()?,
    })
}
