// SPDX-License-Identifier: Apache-2.0
//! Window geometry persistence keyed by window id.
//!
//! States are stored as one JSON object (`window-state.json`) in the app's
//! platform configuration directory. Geometry is in logical pixels.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::{AppInfo, Result};

/// Smallest width/height restored, so a corrupt file never hides a window.
pub const MIN_SIZE: f32 = 160.0;

/// Persisted geometry of one window, in logical pixels.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct WindowState {
    /// Outer position; `None` lets the window manager place the window.
    #[serde(default)]
    pub x: Option<f32>,
    #[serde(default)]
    pub y: Option<f32>,
    /// Inner (client area) size.
    pub width: f32,
    pub height: f32,
    #[serde(default)]
    pub maximized: bool,
    #[serde(default)]
    pub fullscreen: bool,
}

impl Default for WindowState {
    fn default() -> Self {
        Self::with_size(1280.0, 800.0)
    }
}

impl WindowState {
    pub fn with_size(width: f32, height: f32) -> Self {
        Self { x: None, y: None, width, height, maximized: false, fullscreen: false }
    }

    /// Clamps the size to [`MIN_SIZE`] and drops non-finite values.
    pub fn sanitized(mut self) -> Self {
        let fix = |v: f32, fallback: f32| if v.is_finite() { v.max(MIN_SIZE) } else { fallback };
        self.width = fix(self.width, 1280.0);
        self.height = fix(self.height, 800.0);
        self.x = self.x.filter(|v| v.is_finite());
        self.y = self.y.filter(|v| v.is_finite());
        self
    }

    /// Merges a newly observed state: while maximized or fullscreen the
    /// restored (normal) geometry is kept, so un-maximizing restores it.
    pub fn merge_observed(self, observed: WindowState) -> WindowState {
        if observed.maximized || observed.fullscreen {
            WindowState { maximized: observed.maximized, fullscreen: observed.fullscreen, ..self }
        } else {
            observed
        }
    }
}

/// JSON-backed map of window id to [`WindowState`].
#[derive(Debug, Clone)]
pub struct WindowStateStore {
    path: PathBuf,
    states: BTreeMap<String, WindowState>,
    dirty: bool,
}

impl WindowStateStore {
    /// Opens `<config_dir>/window-state.json` for the app.
    pub fn open(app: &AppInfo) -> Result<Self> {
        Ok(Self::at(app.config_dir()?.join("window-state.json")))
    }

    /// Opens a store at an explicit path; a missing or corrupt file yields an
    /// empty store.
    pub fn at(path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        let states = std::fs::read(&path)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default();
        Self { path, states, dirty: false }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn get(&self, id: &str) -> Option<WindowState> {
        self.states.get(id).copied().map(WindowState::sanitized)
    }

    pub fn get_or(&self, id: &str, default: WindowState) -> WindowState {
        self.get(id).unwrap_or(default)
    }

    /// Records a state; returns `true` when it differs from the stored one.
    pub fn set(&mut self, id: &str, state: WindowState) -> bool {
        let state = state.sanitized();
        let changed = self.states.get(id) != Some(&state);
        if changed {
            self.states.insert(id.to_owned(), state);
            self.dirty = true;
        }
        changed
    }

    /// Records an observed state through [`WindowState::merge_observed`].
    pub fn observe(&mut self, id: &str, observed: WindowState) -> bool {
        let merged = match self.states.get(id) {
            Some(previous) => previous.merge_observed(observed),
            None => observed,
        };
        self.set(id, merged)
    }

    pub fn remove(&mut self, id: &str) {
        if self.states.remove(id).is_some() {
            self.dirty = true;
        }
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Writes the file atomically (temp file + rename) when dirty.
    pub fn save(&mut self) -> Result<()> {
        if !self.dirty {
            return Ok(());
        }
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let tmp = self.path.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_vec_pretty(&self.states)?)?;
        std::fs::rename(&tmp, &self.path)?;
        self.dirty = false;
        Ok(())
    }
}
