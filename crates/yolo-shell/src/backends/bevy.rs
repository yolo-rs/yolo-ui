// SPDX-License-Identifier: Apache-2.0
//! Bevy backend: apply/capture `bevy_window::Window` state and a plugin that
//! restores and persists the primary window.

use std::time::{Duration, Instant};

use bevy_app::{App, Plugin, PreStartup, Update};
use bevy_ecs::prelude::*;
use bevy_math::IVec2;
use bevy_window::{MonitorSelection, PrimaryWindow, Window, WindowMode, WindowPosition};

use crate::{WindowState, WindowStateStore};

/// Applies a stored state to a Bevy window. Positions are treated as
/// physical pixels (Bevy's `WindowPosition::At`).
pub fn apply_state(window: &mut Window, state: &WindowState) {
    window.resolution.set(state.width, state.height);
    if let (Some(x), Some(y)) = (state.x, state.y) {
        window.position = WindowPosition::At(IVec2::new(x.round() as i32, y.round() as i32));
    }
    window.set_maximized(state.maximized);
    window.mode = if state.fullscreen { WindowMode::BorderlessFullscreen(MonitorSelection::Current) } else { WindowMode::Windowed };
}

/// Reads a Bevy window's state.
pub fn capture_state(window: &Window) -> WindowState {
    let (x, y) = match window.position {
        WindowPosition::At(p) => (Some(p.x as f32), Some(p.y as f32)),
        _ => (None, None),
    };
    WindowState {
        x,
        y,
        width: window.width(),
        height: window.height(),
        // Bevy exposes no maximized query; keep the last requested value.
        maximized: false,
        fullscreen: !matches!(window.mode, WindowMode::Windowed),
    }
}

/// Restores the primary window at startup and saves it when it changes.
pub struct WindowStatePlugin {
    pub id: String,
    pub store: WindowStateStore,
}

impl WindowStatePlugin {
    pub fn new(id: impl Into<String>, store: WindowStateStore) -> Self {
        Self { id: id.into(), store }
    }
}

#[derive(Resource)]
struct StateResource {
    id: String,
    store: WindowStateStore,
    pending_since: Option<Instant>,
}

const SETTLE: Duration = Duration::from_millis(750);

impl Plugin for WindowStatePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(StateResource { id: self.id.clone(), store: self.store.clone(), pending_since: None })
            .add_systems(PreStartup, restore)
            .add_systems(Update, persist);
    }
}

fn restore(state: Res<StateResource>, mut windows: Query<&mut Window, With<PrimaryWindow>>) {
    if let Some(saved) = state.store.get(&state.id) {
        for mut window in &mut windows {
            apply_state(&mut window, &saved);
        }
    }
}

fn persist(mut state: ResMut<StateResource>, windows: Query<&Window, (With<PrimaryWindow>, Changed<Window>)>) {
    let state = &mut *state;
    for window in &windows {
        if state.store.observe(&state.id, capture_state(window)) {
            state.pending_since = Some(Instant::now());
        }
    }
    if state.pending_since.is_some_and(|since| since.elapsed() >= SETTLE) {
        let _ = state.store.save();
        state.pending_since = None;
    }
}
