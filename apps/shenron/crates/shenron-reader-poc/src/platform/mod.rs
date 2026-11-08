//! Platform glue: where the wgpu surface comes from and how CSS px map to
//! surface px. Everything else (layout, painting, components) is shared.

use std::cell::RefCell;
use std::rc::Rc;

use dioxus::prelude::*;

use crate::gpu::Painter;
use crate::layout::Rect;

#[cfg(feature = "desktop")]
mod desktop;
#[cfg(feature = "desktop")]
pub use desktop::*;

#[cfg(all(feature = "web", not(feature = "desktop")))]
mod web;
#[cfg(all(feature = "web", not(feature = "desktop")))]
pub use web::*;

#[cfg(not(any(feature = "desktop", feature = "web")))]
mod headless;
#[cfg(not(any(feature = "desktop", feature = "web")))]
pub use headless::*;

pub type SharedPainter = Rc<RefCell<Option<Painter>>>;

/// Geometry for one frame: surface size to configure, the canvas rect in
/// surface pixels, and the device pixel ratio.
#[derive(Clone, Copy, Debug)]
pub struct FrameGeometry {
    pub surface_w: u32,
    pub surface_h: u32,
    pub rect_px: Rect,
    pub dpr: f32,
}

/// Starts the asynchronous GPU bring-up. `mounted` is the `<canvas>` mount
/// event on web; it is ignored on desktop (the surface is the window itself).
pub fn attach(
    mounted: Option<Event<MountedData>>,
    painter: SharedPainter,
    mut gpu_ready: Signal<u32>,
    mut status: Signal<String>,
) {
    let Some(target) = surface_target(mounted) else {
        return;
    };
    spawn(async move {
        status.set("GPU: initialisation".to_string());
        match init_painter(target).await {
            Ok(p) => {
                *painter.borrow_mut() = Some(p);
                status.set("GPU: pret".to_string());
                gpu_ready += 1;
            }
            Err(e) => {
                log::error!("gpu init failed: {e}");
                status.set(format!("GPU: erreur ({e})"));
            }
        }
    });
}
