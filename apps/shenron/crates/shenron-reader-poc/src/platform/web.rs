//! Web (wasm32): the Dioxus `<canvas>` element is the wgpu surface. WebGPU is
//! used when the browser exposes it, WebGL2 otherwise.

use dioxus::prelude::*;
use wasm_bindgen::JsCast;

use super::FrameGeometry;
use crate::gpu::Painter;
use crate::layout::Rect;

pub struct SurfaceTarget {
    canvas: web_sys::HtmlCanvasElement,
}

pub fn surface_target(mounted: Option<Event<MountedData>>) -> Option<SurfaceTarget> {
    let ev = mounted?;
    let el = ev.data().downcast::<web_sys::Element>()?.clone();
    let canvas = el.dyn_into::<web_sys::HtmlCanvasElement>().ok()?;
    Some(SurfaceTarget { canvas })
}

pub async fn init_painter(target: SurfaceTarget) -> Result<Painter, String> {
    let instance = wgpu::util::new_instance_with_webgpu_detection(
        wgpu::InstanceDescriptor::new_without_display_handle(),
    )
    .await;
    let (w, h) = (target.canvas.width().max(1), target.canvas.height().max(1));
    let surface = instance
        .create_surface(wgpu::SurfaceTarget::Canvas(target.canvas))
        .map_err(|e| format!("create_surface: {e}"))?;
    Painter::new(&instance, surface, w, h).await
}

pub fn dpr() -> f32 {
    web_sys::window()
        .map(|w| w.device_pixel_ratio() as f32)
        .unwrap_or(1.0)
}

/// Surface = the canvas itself, sized to the Taffy rect times DPR.
pub fn frame_geometry(canvas_css: Rect) -> FrameGeometry {
    let dpr = dpr();
    let w = (canvas_css.w * dpr).round().max(1.0);
    let h = (canvas_css.h * dpr).round().max(1.0);
    FrameGeometry {
        surface_w: w as u32,
        surface_h: h as u32,
        rect_px: Rect { x: 0.0, y: 0.0, w, h },
        dpr,
    }
}

pub fn initial_viewport() -> (f32, f32) {
    let w = web_sys::window();
    let width = w
        .as_ref()
        .and_then(|w| w.inner_width().ok())
        .and_then(|v| v.as_f64())
        .unwrap_or(1280.0);
    let height = w
        .as_ref()
        .and_then(|w| w.inner_height().ok())
        .and_then(|v| v.as_f64())
        .unwrap_or(800.0);
    (width as f32, height as f32)
}
