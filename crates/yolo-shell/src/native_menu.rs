// SPDX-License-Identifier: Apache-2.0
//! Native rendering of the [`Menu`] model with muda.
//!
//! - Windows: [`NativeMenu::attach`] installs the menu bar on the window HWND.
//! - macOS: [`NativeMenu::attach`] installs it as the NSApp main menu.
//! - Linux: muda needs a GTK 3 window; winit/eframe windows are not GTK, so
//!   `attach` returns [`Error::Unsupported`]. Draw the model with the toolkit
//!   instead (`backends::eframe::menu_bar`), or use [`NativeMenu::menu`] with
//!   `muda::Menu::init_for_gtk_window` in GTK hosts. Context and tray menus
//!   work everywhere.

use std::collections::HashMap;

use muda::accelerator::Accelerator;
use muda::{CheckMenuItem, IsMenuItem, MenuId, MenuItem as MudaItem, PredefinedMenuItem, Submenu};

use crate::menu::{Menu, MenuEvent, MenuItem, Role};
use crate::{AppInfo, Error, Result};

/// A [`Menu`] realised as muda items.
pub struct NativeMenu {
    menu: muda::Menu,
    checks: HashMap<String, CheckMenuItem>,
}

enum Built {
    Item(MudaItem),
    Check(CheckMenuItem),
    Submenu(Submenu),
    Predefined(PredefinedMenuItem),
}

impl Built {
    fn as_dyn(&self) -> &dyn IsMenuItem {
        match self {
            Built::Item(i) => i,
            Built::Check(i) => i,
            Built::Submenu(i) => i,
            Built::Predefined(i) => i,
        }
    }
}

fn accelerator(value: &Option<String>) -> Result<Option<Accelerator>> {
    value
        .as_deref()
        .map(|s| s.parse::<Accelerator>().map_err(|e| Error::Accelerator(s.to_owned(), e.to_string())))
        .transpose()
}

fn predefined(role: Role, app: Option<&AppInfo>) -> Option<PredefinedMenuItem> {
    Some(match role {
        Role::About => PredefinedMenuItem::about(None, app.map(AppInfo::about_metadata)),
        Role::Quit => PredefinedMenuItem::quit(None),
        Role::Undo => PredefinedMenuItem::undo(None),
        Role::Redo => PredefinedMenuItem::redo(None),
        Role::Cut => PredefinedMenuItem::cut(None),
        Role::Copy => PredefinedMenuItem::copy(None),
        Role::Paste => PredefinedMenuItem::paste(None),
        Role::SelectAll => PredefinedMenuItem::select_all(None),
        Role::Minimize => PredefinedMenuItem::minimize(None),
        Role::Maximize => PredefinedMenuItem::maximize(None),
        Role::Fullscreen => PredefinedMenuItem::fullscreen(None),
        Role::CloseWindow => PredefinedMenuItem::close_window(None),
        Role::Hide if cfg!(target_os = "macos") => PredefinedMenuItem::hide(None),
        Role::HideOthers if cfg!(target_os = "macos") => PredefinedMenuItem::hide_others(None),
        Role::ShowAll if cfg!(target_os = "macos") => PredefinedMenuItem::show_all(None),
        Role::Services if cfg!(target_os = "macos") => PredefinedMenuItem::services(None),
        _ => return None,
    })
}

impl NativeMenu {
    /// Builds muda items for `model`. `app` feeds the About role.
    pub fn build(model: &Menu, app: Option<&AppInfo>) -> Result<Self> {
        let menu = muda::Menu::new();
        let mut checks = HashMap::new();
        for item in &model.items {
            if let Some(built) = Self::build_item(item, app, &mut checks)? {
                menu.append(built.as_dyn())?;
            }
        }
        Ok(Self { menu, checks })
    }

    fn build_item(item: &MenuItem, app: Option<&AppInfo>, checks: &mut HashMap<String, CheckMenuItem>) -> Result<Option<Built>> {
        Ok(Some(match item {
            MenuItem::Action { id, label, accelerator: accel, enabled } => {
                Built::Item(MudaItem::with_id(MenuId::new(id), label, *enabled, accelerator(accel)?))
            }
            MenuItem::Check { id, label, checked, accelerator: accel, enabled }
            | MenuItem::Radio { id, label, checked, accelerator: accel, enabled, .. } => {
                let check = CheckMenuItem::with_id(MenuId::new(id), label, *enabled, *checked, accelerator(accel)?);
                checks.insert(id.clone(), check.clone());
                Built::Check(check)
            }
            MenuItem::Submenu { label, items, enabled } => {
                let submenu = Submenu::new(label, *enabled);
                for child in items {
                    if let Some(built) = Self::build_item(child, app, checks)? {
                        submenu.append(built.as_dyn())?;
                    }
                }
                Built::Submenu(submenu)
            }
            MenuItem::Separator => Built::Predefined(PredefinedMenuItem::separator()),
            MenuItem::Role { role } => match predefined(*role, app) {
                Some(item) => Built::Predefined(item),
                None => return Ok(None),
            },
        }))
    }

    /// The underlying muda menu (also usable as a tray/context menu).
    pub fn menu(&self) -> &muda::Menu {
        &self.menu
    }

    /// Pushes the model's check/radio state into the native items.
    pub fn sync(&self, model: &Menu) {
        for (id, item) in &self.checks {
            if let Some(checked) = model.is_checked(id) {
                item.set_checked(checked);
            }
        }
    }

    /// Converts one muda event: updates `model` (check/radio) and native state.
    pub fn handle(&self, model: &mut Menu, event: &muda::MenuEvent) -> MenuEvent {
        let out = model.activate(&event.id.0);
        if out.checked.is_some() {
            self.sync(model);
        }
        out
    }

    /// Drains muda's global event channel.
    pub fn poll(&self, model: &mut Menu) -> Vec<MenuEvent> {
        muda::MenuEvent::receiver().try_iter().map(|event| self.handle(model, &event)).collect()
    }

    /// Installs the menu bar on a native window (Windows HWND, macOS NSApp).
    pub fn attach<W: raw_window_handle::HasWindowHandle + ?Sized>(&self, window: &W) -> Result<()> {
        let handle = window.window_handle().map_err(|_| Error::Unsupported("window handle unavailable"))?;
        match handle.as_raw() {
            #[cfg(windows)]
            raw_window_handle::RawWindowHandle::Win32(h) => {
                #[allow(unsafe_code)]
                // SAFETY: the HWND comes from a live window handle borrowed for this call.
                unsafe {
                    self.menu.init_for_hwnd(h.hwnd.get())?
                };
                Ok(())
            }
            #[cfg(target_os = "macos")]
            raw_window_handle::RawWindowHandle::AppKit(_) => {
                self.menu.init_for_nsapp();
                Ok(())
            }
            _ => Err(Error::Unsupported("native menu bar needs a Win32 or AppKit window; draw the model with the toolkit")),
        }
    }

    /// Shows the menu as a context menu at the cursor (Windows/macOS).
    pub fn show_context_menu<W: raw_window_handle::HasWindowHandle + ?Sized>(&self, window: &W) -> Result<bool> {
        let handle = window.window_handle().map_err(|_| Error::Unsupported("window handle unavailable"))?;
        match handle.as_raw() {
            #[cfg(windows)]
            raw_window_handle::RawWindowHandle::Win32(h) => {
                #[allow(unsafe_code)]
                // SAFETY: the HWND comes from a live window handle borrowed for this call.
                let shown = unsafe { muda::ContextMenu::show_context_menu_for_hwnd(&self.menu, h.hwnd.get(), None) };
                Ok(shown)
            }
            #[cfg(target_os = "macos")]
            raw_window_handle::RawWindowHandle::AppKit(h) => {
                #[allow(unsafe_code)]
                // SAFETY: the NSView pointer comes from a live window handle borrowed for this call.
                let shown = unsafe { muda::ContextMenu::show_context_menu_for_nsview(&self.menu, h.ns_view.as_ptr() as _, None) };
                Ok(shown)
            }
            _ => Err(Error::Unsupported("context menu needs a Win32 or AppKit window")),
        }
    }
}

/// Updates `model` from muda's global channel without native sync (for menus
/// owned by another thread, such as a Linux GTK tray thread).
pub fn poll_events(model: &mut Menu) -> Vec<MenuEvent> {
    muda::MenuEvent::receiver().try_iter().map(|event| model.activate(&event.id.0)).collect()
}
