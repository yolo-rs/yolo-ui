// SPDX-License-Identifier: Apache-2.0
//! Desktop notifications through notify-rust (freedesktop D-Bus on Linux/BSD,
//! WinRT toasts on Windows, NSUserNotification on macOS).

use std::time::Duration;

use crate::{AppInfo, Result};

/// A notification to show.
#[derive(Clone, Debug, Default)]
pub struct Notice {
    pub summary: String,
    pub body: Option<String>,
    /// Freedesktop icon name or file path (Linux/BSD).
    pub icon: Option<String>,
    pub timeout: Option<Duration>,
}

impl Notice {
    pub fn new(summary: impl Into<String>) -> Self {
        Self { summary: summary.into(), ..Self::default() }
    }

    pub fn body(mut self, body: impl Into<String>) -> Self {
        self.body = Some(body.into());
        self
    }

    pub fn icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }
}

/// Shows `notice` on behalf of `app`.
pub fn notify(app: &AppInfo, notice: &Notice) -> Result<()> {
    let mut n = notify_rust::Notification::new();
    n.appname(&app.name).summary(&notice.summary);
    if let Some(body) = &notice.body {
        n.body(body);
    }
    if let Some(icon) = &notice.icon {
        n.icon(icon);
    }
    if let Some(timeout) = notice.timeout {
        n.timeout(notify_rust::Timeout::Milliseconds(timeout.as_millis().min(u32::MAX as u128) as u32));
    }
    #[cfg(windows)]
    n.app_id(&app.id);
    n.show()?;
    Ok(())
}

/// Shorthand for a summary/body notification.
pub fn notify_text(app: &AppInfo, summary: &str, body: &str) -> Result<()> {
    notify(app, &Notice::new(summary).body(body))
}
