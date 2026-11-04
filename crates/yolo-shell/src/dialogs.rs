// SPDX-License-Identifier: Apache-2.0
//! File and message dialogs through rfd (XDG portal on Linux, COM on Windows,
//! NSOpenPanel on macOS).

use std::path::PathBuf;

use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
pub use rfd::{MessageButtons, MessageDialogResult, MessageLevel};

/// One filter row (`"Images"`, `["png", "jpg"]`).
#[derive(Clone, Debug)]
pub struct FileFilter {
    pub name: String,
    pub extensions: Vec<String>,
}

impl FileFilter {
    pub fn new(name: impl Into<String>, extensions: &[&str]) -> Self {
        Self { name: name.into(), extensions: extensions.iter().map(|e| e.to_string()).collect() }
    }
}

/// Builder over `rfd::FileDialog`.
#[derive(Clone, Debug, Default)]
pub struct FileDialog {
    inner: rfd::FileDialog,
}

impl FileDialog {
    pub fn new() -> Self {
        Self { inner: rfd::FileDialog::new() }
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.inner = self.inner.set_title(title);
        self
    }

    pub fn filter(mut self, filter: &FileFilter) -> Self {
        self.inner = self.inner.add_filter(filter.name.clone(), &filter.extensions);
        self
    }

    pub fn directory(mut self, dir: impl Into<PathBuf>) -> Self {
        self.inner = self.inner.set_directory(dir.into());
        self
    }

    pub fn file_name(mut self, name: impl Into<String>) -> Self {
        self.inner = self.inner.set_file_name(name);
        self
    }

    /// Makes the dialog modal to `window`.
    pub fn parent<W: HasWindowHandle + HasDisplayHandle + ?Sized>(mut self, window: &W) -> Self {
        self.inner = self.inner.set_parent(window);
        self
    }

    pub fn open_file(self) -> Option<PathBuf> {
        self.inner.pick_file()
    }

    pub fn open_files(self) -> Option<Vec<PathBuf>> {
        self.inner.pick_files()
    }

    pub fn pick_folder(self) -> Option<PathBuf> {
        self.inner.pick_folder()
    }

    pub fn save_file(self) -> Option<PathBuf> {
        self.inner.save_file()
    }
}

/// Shows a blocking message box; returns the pressed button.
pub fn message(level: MessageLevel, title: &str, text: &str, buttons: MessageButtons) -> MessageDialogResult {
    rfd::MessageDialog::new().set_level(level).set_title(title).set_description(text).set_buttons(buttons).show()
}

/// Yes/No confirmation; `true` on Yes or Ok.
pub fn confirm(title: &str, text: &str) -> bool {
    matches!(
        message(MessageLevel::Info, title, text, MessageButtons::YesNo),
        MessageDialogResult::Yes | MessageDialogResult::Ok
    )
}

/// Native about box built from [`crate::AppInfo::about_text`].
pub fn about(app: &crate::AppInfo) {
    message(MessageLevel::Info, &format!("About {}", app.name), &app.about_text(), MessageButtons::Ok);
}
