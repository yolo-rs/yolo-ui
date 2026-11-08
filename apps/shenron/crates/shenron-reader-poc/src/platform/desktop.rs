//! Desktop: dioxus-desktop owns a tao window hosting a transparent webview.
//! We create a wgpu surface on that same window (tao implements
//! raw-window-handle 0.6) and paint under the webview; the canvas area of the
//! chrome is transparent so the GPU output shows through.

use std::sync::Arc;

use dioxus::prelude::*;

use super::FrameGeometry;
use crate::gpu::Painter;
use crate::layout::Rect;

pub struct SurfaceTarget {
    window: Arc<tao::window::Window>,
}

pub fn surface_target(_mounted: Option<Event<MountedData>>) -> Option<SurfaceTarget> {
    let ctx = dioxus::desktop::window();
    Some(SurfaceTarget { window: ctx.window.clone() })
}

pub async fn init_painter(target: SurfaceTarget) -> Result<Painter, String> {
    let instance =
        wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
    let size = target.window.inner_size();
    let surface = instance
        .create_surface(wgpu::SurfaceTarget::Window(Box::new(target.window.clone())))
        .map_err(|e| format!("create_surface: {e}"))?;
    Painter::new(&instance, surface, size.width, size.height).await
}

/// Surface = whole window; canvas rect is scaled by the window scale factor.
pub fn frame_geometry(canvas_css: Rect) -> FrameGeometry {
    let ctx = dioxus::desktop::window();
    let size = ctx.window.inner_size();
    let dpr = ctx.window.scale_factor() as f32;
    FrameGeometry {
        surface_w: size.width,
        surface_h: size.height,
        rect_px: Rect {
            x: (canvas_css.x * dpr).round(),
            y: (canvas_css.y * dpr).round(),
            w: (canvas_css.w * dpr).round(),
            h: (canvas_css.h * dpr).round(),
        },
        dpr,
    }
}

/// Initial viewport in CSS px (before the first `onresize`).
pub fn initial_viewport() -> (f32, f32) {
    let ctx = dioxus::desktop::window();
    let s = ctx
        .window
        .inner_size()
        .to_logical::<f32>(ctx.window.scale_factor());
    (s.width, s.height)
}
