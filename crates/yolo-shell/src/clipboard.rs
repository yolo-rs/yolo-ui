// SPDX-License-Identifier: Apache-2.0
//! Clipboard through arboard (Win32, NSPasteboard, X11 and Wayland data control).

use std::borrow::Cow;

use crate::Result;

/// RGBA8 image on the clipboard.
#[derive(Clone, Debug)]
pub struct ClipboardImage {
    pub width: usize,
    pub height: usize,
    pub rgba: Vec<u8>,
}

/// Owned clipboard connection. Keep it alive while the app owns a selection
/// (X11/Wayland serve clipboard data from the owning process).
pub struct Clipboard {
    inner: arboard::Clipboard,
}

impl Clipboard {
    pub fn new() -> Result<Self> {
        Ok(Self { inner: arboard::Clipboard::new()? })
    }

    pub fn text(&mut self) -> Result<String> {
        Ok(self.inner.get_text()?)
    }

    pub fn set_text(&mut self, text: impl Into<String>) -> Result<()> {
        let text: String = text.into();
        self.inner.set_text(text)?;
        Ok(())
    }

    pub fn image(&mut self) -> Result<ClipboardImage> {
        let image = self.inner.get_image()?;
        Ok(ClipboardImage { width: image.width, height: image.height, rgba: image.bytes.into_owned() })
    }

    pub fn set_image(&mut self, image: &ClipboardImage) -> Result<()> {
        self.inner.set_image(arboard::ImageData {
            width: image.width,
            height: image.height,
            bytes: Cow::Borrowed(&image.rgba),
        })?;
        Ok(())
    }

    pub fn clear(&mut self) -> Result<()> {
        self.inner.clear()?;
        Ok(())
    }

    /// The arboard handle for formats this wrapper does not cover.
    pub fn inner(&mut self) -> &mut arboard::Clipboard {
        &mut self.inner
    }
}
