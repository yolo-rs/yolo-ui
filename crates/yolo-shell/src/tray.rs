// SPDX-License-Identifier: Apache-2.0
//! System tray icon through tray-icon, with the menu built from the model.
//!
//! Threading rules come from tray-icon:
//! - Windows: create on a thread that runs a Win32 message loop (the winit /
//!   eframe event loop thread).
//! - macOS: create on the main thread after the event loop started.
//! - Linux: create on a thread running GTK. winit/eframe do not run GTK, so use
//!   [`Tray::spawn_gtk_thread`] (libayatana-appindicator3 + GTK 3 required).
//!
//! Tray and menu activations arrive on tray-icon / muda global channels; read
//! them with [`Tray::poll_events`] and [`crate::native_menu::poll_events`].

use tray_icon::{Icon, TrayIcon, TrayIconBuilder, TrayIconEvent};

use crate::menu::Menu;
use crate::native_menu::NativeMenu;
use crate::{AppInfo, Result};

/// Tray configuration.
#[derive(Clone, Debug)]
pub struct TrayOptions {
    pub id: String,
    pub tooltip: Option<String>,
    /// RGBA8 pixels, width, height.
    pub icon: Option<(Vec<u8>, u32, u32)>,
    pub menu: Option<Menu>,
    /// Show the menu on left click as well as right click.
    pub menu_on_left_click: bool,
    /// macOS: treat the icon as a template image.
    pub template: bool,
}

impl TrayOptions {
    pub fn new(app: &AppInfo) -> Self {
        Self {
            id: app.id.clone(),
            tooltip: Some(app.name.clone()),
            icon: None,
            menu: None,
            menu_on_left_click: false,
            template: false,
        }
    }
}

/// A live tray icon.
pub struct Tray {
    icon: TrayIcon,
    menu: Option<NativeMenu>,
}

impl Tray {
    /// Creates the tray icon on the current thread (see module threading rules).
    pub fn new(options: &TrayOptions, app: Option<&AppInfo>) -> Result<Self> {
        let mut builder = TrayIconBuilder::new()
            .with_id(options.id.clone())
            .with_menu_on_left_click(options.menu_on_left_click)
            .with_icon_as_template(options.template);
        if let Some(tooltip) = &options.tooltip {
            builder = builder.with_tooltip(tooltip);
        }
        if let Some((rgba, width, height)) = &options.icon {
            builder = builder.with_icon(Icon::from_rgba(rgba.clone(), *width, *height)?);
        }
        let menu = options.menu.as_ref().map(|model| NativeMenu::build(model, app)).transpose()?;
        if let Some(menu) = &menu {
            builder = builder.with_menu(Box::new(menu.menu().clone()));
        }
        Ok(Self { icon: builder.build()?, menu })
    }

    pub fn icon(&self) -> &TrayIcon {
        &self.icon
    }

    pub fn native_menu(&self) -> Option<&NativeMenu> {
        self.menu.as_ref()
    }

    pub fn set_tooltip(&self, tooltip: Option<&str>) -> Result<()> {
        self.icon.set_tooltip(tooltip)?;
        Ok(())
    }

    /// Drains tray-icon's global event channel (clicks, enter/leave).
    pub fn poll_events() -> Vec<TrayIconEvent> {
        TrayIconEvent::receiver().try_iter().collect()
    }

    /// Linux: runs GTK and the tray on a dedicated thread for the process
    /// lifetime. Events still arrive on the global channels.
    #[cfg(any(target_os = "linux", target_os = "dragonfly", target_os = "freebsd", target_os = "netbsd", target_os = "openbsd"))]
    pub fn spawn_gtk_thread(options: TrayOptions, app: Option<AppInfo>) -> Result<()> {
        let (ready_tx, ready_rx) = std::sync::mpsc::channel::<Result<()>>();
        std::thread::Builder::new().name("yolo-tray-gtk".into()).spawn(move || {
            if let Err(error) = gtk::init() {
                let _ = ready_tx.send(Err(crate::Error::Invalid(format!("GTK init failed: {error}"))));
                return;
            }
            match Tray::new(&options, app.as_ref()) {
                Ok(tray) => {
                    let _ = ready_tx.send(Ok(()));
                    gtk::main();
                    drop(tray);
                }
                Err(error) => {
                    let _ = ready_tx.send(Err(error));
                }
            }
        })?;
        ready_rx.recv().map_err(|_| crate::Error::Invalid("tray thread exited".into()))?
    }
}
