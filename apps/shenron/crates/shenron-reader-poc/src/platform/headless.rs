//! No renderer selected: keeps `cargo check`/`cargo test` of the shared code
//! meaningful without a window system.

use dioxus::prelude::*;

use super::FrameGeometry;
use crate::gpu::Painter;
use crate::layout::Rect;

pub struct SurfaceTarget;

pub fn surface_target(_mounted: Option<Event<MountedData>>) -> Option<SurfaceTarget> {
    None
}

pub async fn init_painter(_target: SurfaceTarget) -> Result<Painter, String> {
    Err("no renderer feature enabled".to_string())
}

pub fn frame_geometry(canvas_css: Rect) -> FrameGeometry {
    FrameGeometry {
        surface_w: canvas_css.w.max(1.0) as u32,
        surface_h: canvas_css.h.max(1.0) as u32,
        rect_px: Rect { x: 0.0, y: 0.0, w: canvas_css.w, h: canvas_css.h },
        dpr: 1.0,
    }
}

pub fn initial_viewport() -> (f32, f32) {
    (1280.0, 800.0)
}
