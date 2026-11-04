// SPDX-License-Identifier: Apache-2.0
//! Error type shared by every shell service.

use std::io;

/// Result alias for shell operations.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Errors raised by shell services.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("no home directory to resolve platform directories")]
    NoHomeDir,
    #[error("operation not supported on this platform or window handle: {0}")]
    Unsupported(&'static str),
    #[error("invalid argument: {0}")]
    Invalid(String),
    #[cfg(feature = "menu")]
    #[error("menu error: {0}")]
    Menu(#[from] muda::Error),
    #[cfg(feature = "menu")]
    #[error("invalid accelerator {0:?}: {1}")]
    Accelerator(String, String),
    #[cfg(feature = "tray")]
    #[error("tray error: {0}")]
    Tray(#[from] tray_icon::Error),
    #[cfg(feature = "tray")]
    #[error("bad tray icon: {0}")]
    TrayIcon(#[from] tray_icon::BadIcon),
    #[cfg(feature = "notifications")]
    #[error("notification error: {0}")]
    Notification(#[from] notify_rust::error::Error),
    #[cfg(feature = "clipboard")]
    #[error("clipboard error: {0}")]
    Clipboard(#[from] arboard::Error),
    #[cfg(feature = "updater")]
    #[error("update feed error: {0}")]
    Update(String),
    #[cfg(feature = "updater")]
    #[error("HTTP error: {0}")]
    Http(#[from] ureq::Error),
    #[cfg(feature = "updater")]
    #[error("update signature rejected: {0}")]
    Signature(#[from] ed25519_dalek::SignatureError),
}
