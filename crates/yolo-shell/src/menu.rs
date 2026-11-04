// SPDX-License-Identifier: Apache-2.0
//! Declarative menu model shared by every backend.
//!
//! One [`Menu`] describes actions, submenus, accelerators, check and radio
//! items and platform roles. It is rendered natively through muda
//! ([`crate::native_menu`]), as `tauri::menu` (tauri backend) or drawn by the
//! toolkit (egui backend). Check and radio state lives in the model:
//! backends report activations through [`Menu::activate`], which updates the
//! state and returns the [`MenuEvent`] for the app.
//!
//! Accelerators use the cross-platform string form understood by muda and
//! Tauri: `CmdOrCtrl+Shift+S`, `Alt+F4`, `F11`.

use serde::{Deserialize, Serialize};

/// A menu bar or context menu.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Menu {
    pub items: Vec<MenuItem>,
}

/// One entry of a menu.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MenuItem {
    Action {
        id: String,
        label: String,
        #[serde(default)]
        accelerator: Option<String>,
        #[serde(default = "yes")]
        enabled: bool,
    },
    Check {
        id: String,
        label: String,
        checked: bool,
        #[serde(default)]
        accelerator: Option<String>,
        #[serde(default = "yes")]
        enabled: bool,
    },
    /// Mutually exclusive within `group` across the whole menu.
    Radio {
        id: String,
        label: String,
        group: String,
        checked: bool,
        #[serde(default)]
        accelerator: Option<String>,
        #[serde(default = "yes")]
        enabled: bool,
    },
    Submenu {
        label: String,
        items: Vec<MenuItem>,
        #[serde(default = "yes")]
        enabled: bool,
    },
    Separator,
    /// Platform-provided item (native label, accelerator and behaviour).
    Role { role: Role },
}

fn yes() -> bool {
    true
}

/// Platform menu roles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    About,
    Quit,
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    SelectAll,
    Minimize,
    Maximize,
    Fullscreen,
    CloseWindow,
    Hide,
    HideOthers,
    ShowAll,
    Services,
}

impl Role {
    /// Stable id reported in [`MenuEvent::id`] when a toolkit draws the role.
    pub fn id(self) -> &'static str {
        match self {
            Role::About => "role:about",
            Role::Quit => "role:quit",
            Role::Undo => "role:undo",
            Role::Redo => "role:redo",
            Role::Cut => "role:cut",
            Role::Copy => "role:copy",
            Role::Paste => "role:paste",
            Role::SelectAll => "role:select_all",
            Role::Minimize => "role:minimize",
            Role::Maximize => "role:maximize",
            Role::Fullscreen => "role:fullscreen",
            Role::CloseWindow => "role:close_window",
            Role::Hide => "role:hide",
            Role::HideOthers => "role:hide_others",
            Role::ShowAll => "role:show_all",
            Role::Services => "role:services",
        }
    }

    /// English fallback label for toolkit-drawn menus.
    pub fn label(self) -> &'static str {
        match self {
            Role::About => "About",
            Role::Quit => "Quit",
            Role::Undo => "Undo",
            Role::Redo => "Redo",
            Role::Cut => "Cut",
            Role::Copy => "Copy",
            Role::Paste => "Paste",
            Role::SelectAll => "Select All",
            Role::Minimize => "Minimize",
            Role::Maximize => "Zoom",
            Role::Fullscreen => "Toggle Full Screen",
            Role::CloseWindow => "Close Window",
            Role::Hide => "Hide",
            Role::HideOthers => "Hide Others",
            Role::ShowAll => "Show All",
            Role::Services => "Services",
        }
    }

    /// Default accelerator for toolkit-drawn menus.
    pub fn accelerator(self) -> Option<&'static str> {
        match self {
            Role::Quit => Some("CmdOrCtrl+Q"),
            Role::Undo => Some("CmdOrCtrl+Z"),
            Role::Redo => Some("CmdOrCtrl+Shift+Z"),
            Role::Cut => Some("CmdOrCtrl+X"),
            Role::Copy => Some("CmdOrCtrl+C"),
            Role::Paste => Some("CmdOrCtrl+V"),
            Role::SelectAll => Some("CmdOrCtrl+A"),
            Role::Minimize => Some("CmdOrCtrl+M"),
            Role::Fullscreen => Some("F11"),
            Role::CloseWindow => Some("CmdOrCtrl+W"),
            Role::Hide => Some("CmdOrCtrl+H"),
            _ => None,
        }
    }

    /// Roles that only exist in the macOS application menu.
    pub fn is_macos_only(self) -> bool {
        matches!(self, Role::Hide | Role::HideOthers | Role::ShowAll | Role::Services)
    }
}

/// Activation reported to the app.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MenuEvent {
    pub id: String,
    /// New state of a check or radio item.
    pub checked: Option<bool>,
    /// Set when the activated item is a role drawn by a toolkit.
    pub role: Option<Role>,
}

impl MenuItem {
    pub fn action(id: impl Into<String>, label: impl Into<String>) -> Self {
        MenuItem::Action { id: id.into(), label: label.into(), accelerator: None, enabled: true }
    }

    pub fn check(id: impl Into<String>, label: impl Into<String>, checked: bool) -> Self {
        MenuItem::Check { id: id.into(), label: label.into(), checked, accelerator: None, enabled: true }
    }

    pub fn radio(id: impl Into<String>, label: impl Into<String>, group: impl Into<String>, checked: bool) -> Self {
        MenuItem::Radio {
            id: id.into(),
            label: label.into(),
            group: group.into(),
            checked,
            accelerator: None,
            enabled: true,
        }
    }

    pub fn submenu(label: impl Into<String>, items: Vec<MenuItem>) -> Self {
        MenuItem::Submenu { label: label.into(), items, enabled: true }
    }

    pub fn role(role: Role) -> Self {
        MenuItem::Role { role }
    }

    /// Sets the accelerator of an action, check or radio item.
    pub fn with_accelerator(mut self, value: impl Into<String>) -> Self {
        if let MenuItem::Action { accelerator, .. }
        | MenuItem::Check { accelerator, .. }
        | MenuItem::Radio { accelerator, .. } = &mut self
        {
            *accelerator = Some(value.into());
        }
        self
    }

    pub fn enabled(mut self, value: bool) -> Self {
        if let MenuItem::Action { enabled, .. }
        | MenuItem::Check { enabled, .. }
        | MenuItem::Radio { enabled, .. }
        | MenuItem::Submenu { enabled, .. } = &mut self
        {
            *enabled = value;
        }
        self
    }

    /// Id of an action, check or radio item.
    pub fn id(&self) -> Option<&str> {
        match self {
            MenuItem::Action { id, .. } | MenuItem::Check { id, .. } | MenuItem::Radio { id, .. } => Some(id),
            _ => None,
        }
    }
}

impl Menu {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn item(mut self, item: MenuItem) -> Self {
        self.items.push(item);
        self
    }

    pub fn submenu(self, label: impl Into<String>, items: Vec<MenuItem>) -> Self {
        self.item(MenuItem::submenu(label, items))
    }

    /// Standard app menu set: platform app menu on macOS, File/Edit/View/Window/Help
    /// skeleton the app extends.
    pub fn standard(app_name: &str) -> Self {
        let mut menu = Menu::new();
        if cfg!(target_os = "macos") {
            menu = menu.submenu(
                app_name,
                vec![
                    MenuItem::role(Role::About),
                    MenuItem::Separator,
                    MenuItem::role(Role::Services),
                    MenuItem::Separator,
                    MenuItem::role(Role::Hide),
                    MenuItem::role(Role::HideOthers),
                    MenuItem::role(Role::ShowAll),
                    MenuItem::Separator,
                    MenuItem::role(Role::Quit),
                ],
            );
        }
        let mut file = vec![MenuItem::role(Role::CloseWindow)];
        if !cfg!(target_os = "macos") {
            file.push(MenuItem::Separator);
            file.push(MenuItem::role(Role::Quit));
        }
        menu = menu.submenu("File", file).submenu(
            "Edit",
            vec![
                MenuItem::role(Role::Undo),
                MenuItem::role(Role::Redo),
                MenuItem::Separator,
                MenuItem::role(Role::Cut),
                MenuItem::role(Role::Copy),
                MenuItem::role(Role::Paste),
                MenuItem::role(Role::SelectAll),
            ],
        );
        menu = menu.submenu("Window", vec![MenuItem::role(Role::Minimize), MenuItem::role(Role::Maximize), MenuItem::role(Role::Fullscreen)]);
        if !cfg!(target_os = "macos") {
            menu = menu.submenu("Help", vec![MenuItem::role(Role::About)]);
        }
        menu
    }

    /// Visits every item depth-first.
    pub fn walk(&self, f: &mut impl FnMut(&MenuItem)) {
        fn go(items: &[MenuItem], f: &mut impl FnMut(&MenuItem)) {
            for item in items {
                f(item);
                if let MenuItem::Submenu { items, .. } = item {
                    go(items, f);
                }
            }
        }
        go(&self.items, f);
    }

    fn walk_mut(items: &mut [MenuItem], f: &mut impl FnMut(&mut MenuItem)) {
        for item in items {
            f(item);
            if let MenuItem::Submenu { items, .. } = item {
                Self::walk_mut(items, f);
            }
        }
    }

    pub fn find(&self, id: &str) -> Option<&MenuItem> {
        Self::find_in(&self.items, id)
    }

    fn find_in<'a>(items: &'a [MenuItem], id: &str) -> Option<&'a MenuItem> {
        for item in items {
            if item.id() == Some(id) {
                return Some(item);
            }
            if let MenuItem::Submenu { items, .. } = item
                && let Some(found) = Self::find_in(items, id)
            {
                return Some(found);
            }
        }
        None
    }

    /// Current checked state of a check or radio item.
    pub fn is_checked(&self, id: &str) -> Option<bool> {
        match self.find(id)? {
            MenuItem::Check { checked, .. } | MenuItem::Radio { checked, .. } => Some(*checked),
            _ => None,
        }
    }

    /// Sets a check item, or selects a radio item (unchecking its group).
    pub fn set_checked(&mut self, id: &str, value: bool) {
        let group = match self.find(id) {
            Some(MenuItem::Radio { group, .. }) => Some(group.clone()),
            _ => None,
        };
        Self::walk_mut(&mut self.items, &mut |item| match item {
            MenuItem::Check { id: item_id, checked, .. } if item_id == id => *checked = value,
            MenuItem::Radio { id: item_id, group: g, checked, .. } if Some(&*g) == group.as_ref() => {
                *checked = if item_id == id { value } else { *checked && !value };
            }
            _ => {}
        });
    }

    /// Enables or disables an item by id.
    pub fn set_enabled(&mut self, id: &str, value: bool) {
        Self::walk_mut(&mut self.items, &mut |item| {
            if item.id() == Some(id)
                && let MenuItem::Action { enabled, .. } | MenuItem::Check { enabled, .. } | MenuItem::Radio { enabled, .. } = item
            {
                *enabled = value;
            }
        });
    }

    /// Records an activation coming from a backend and returns the app event:
    /// toggles check items and selects radio items.
    pub fn activate(&mut self, id: &str) -> MenuEvent {
        if let Some(role) = role_from_id(id) {
            return MenuEvent { id: id.to_owned(), checked: None, role: Some(role) };
        }
        let checked = match self.find(id) {
            Some(MenuItem::Check { checked, .. }) => Some(!*checked),
            Some(MenuItem::Radio { .. }) => Some(true),
            _ => None,
        };
        if let Some(value) = checked {
            self.set_checked(id, value);
        }
        MenuEvent { id: id.to_owned(), checked, role: None }
    }
}

/// Parses a [`Role::id`].
pub fn role_from_id(id: &str) -> Option<Role> {
    const ALL: [Role; 16] = [
        Role::About,
        Role::Quit,
        Role::Undo,
        Role::Redo,
        Role::Cut,
        Role::Copy,
        Role::Paste,
        Role::SelectAll,
        Role::Minimize,
        Role::Maximize,
        Role::Fullscreen,
        Role::CloseWindow,
        Role::Hide,
        Role::HideOthers,
        Role::ShowAll,
        Role::Services,
    ];
    ALL.into_iter().find(|role| role.id() == id)
}
