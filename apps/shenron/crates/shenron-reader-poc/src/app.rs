//! Dioxus component tree. Layout rectangles come from Taffy (`layout.rs`);
//! pixels come from wgpu (`gpu.rs`). Components only wire state and events.

use std::cell::RefCell;
use std::rc::Rc;

use dioxus::prelude::*;

use crate::gpu::{self, View};
use crate::layout::{self, ChromeLayout, Rect};
use crate::platform::{self, SharedPainter};
use crate::theme;

const PAGE_COUNT: usize = 8;
const ZOOM_MIN: f32 = 0.1;
const ZOOM_MAX: f32 = 8.0;

#[component]
pub fn App() -> Element {
    let mut viewport = use_signal(platform::initial_viewport);
    let mut view = use_signal(View::default);
    let mut page = use_signal(|| 1usize);
    let mut drag = use_signal(|| None::<(f64, f64)>);
    let status = use_signal(|| "GPU: en attente".to_string());
    let mut gpu_ready = use_signal(|| 0u32);
    let painter: SharedPainter = use_hook(|| Rc::new(RefCell::new(None)));

    // Taffy pass: viewport -> chrome rects (memoised, recomputed on resize).
    let chrome = use_memo(move || {
        let (w, h) = viewport();
        layout::compute(w, h)
    });

    // Desktop: the surface is the window, attach immediately.
    #[cfg(feature = "desktop")]
    {
        let painter = painter.clone();
        use_hook(move || platform::attach(None, painter, gpu_ready, status));
    }

    // Paint pass: any change of view/layout/GPU state repaints the canvas rect.
    {
        let painter = painter.clone();
        use_effect(move || {
            let _ = gpu_ready();
            let v = view();
            let c: ChromeLayout = chrome();
            if let Some(p) = painter.borrow_mut().as_mut() {
                let g = platform::frame_geometry(c.canvas);
                p.resize(g.surface_w, g.surface_h);
                let v_px =
                    View { pan_x: v.pan_x * g.dpr, pan_y: v.pan_y * g.dpr, zoom: v.zoom * g.dpr };
                p.paint(g.rect_px, v_px, g.dpr);
            }
        });
    }

    let mut fit = {
        let painter = painter.clone();
        move || {
            let c = chrome();
            let (iw, ih) = painter
                .borrow()
                .as_ref()
                .map(|p| p.image_size())
                .unwrap_or(gpu::PLACEHOLDER_PAGE);
            let z = ((c.canvas.w - 32.0) / iw as f32).min((c.canvas.h - 32.0) / ih as f32);
            view.set(View { pan_x: 0.0, pan_y: 0.0, zoom: z.clamp(ZOOM_MIN, ZOOM_MAX) });
        }
    };

    let mut zoom_by = move |factor: f32, anchor: Option<(f32, f32)>| {
        let c = chrome();
        let mut v = view();
        let nz = (v.zoom * factor).clamp(ZOOM_MIN, ZOOM_MAX);
        let f = nz / v.zoom;
        let (ax, ay) = anchor.unwrap_or((c.canvas.w * 0.5, c.canvas.h * 0.5));
        let (cx, cy) = (ax - c.canvas.w * 0.5, ay - c.canvas.h * 0.5);
        v.pan_x = cx + (v.pan_x - cx) * f;
        v.pan_y = cy + (v.pan_y - cy) * f;
        v.zoom = nz;
        view.set(v);
    };

    let load_test_page = {
        let painter = painter.clone();
        move |_| {
            let (w, h) = (560u32, 800u32);
            if let Some(p) = painter.borrow_mut().as_mut() {
                p.set_image(&gpu::test_page(w, h), w, h);
            }
            gpu_ready += 1;
        }
    };
    let clear_page = {
        let painter = painter.clone();
        move |_| {
            if let Some(p) = painter.borrow_mut().as_mut() {
                p.set_image(&[], 0, 0);
            }
            gpu_ready += 1;
        }
    };

    let c = chrome();
    let canvas_rect: Rect = c.canvas;
    let mut fit_click = fit.clone();

    rsx! {
        div {
            style: "{theme::root_style()}",
            onresize: move |e| {
                if let Ok(s) = e.data().get_border_box_size() {
                    viewport.set((s.width as f32, s.height as f32));
                }
            },

            // ---- toolbar ----
            div { style: "{theme::toolbar_style(c.toolbar)}",
                span { style: "{theme::title_style()}", "SHENRON" }
                button { style: "{theme::button_style(false)}", onclick: move |_| zoom_by(1.0 / 1.25, None), "Zoom -" }
                button { style: "{theme::button_style(false)}", onclick: move |_| zoom_by(1.25, None), "Zoom +" }
                button { style: "{theme::button_style(false)}", onclick: move |_| fit_click(), "Ajuster" }
                button { style: "{theme::button_style(false)}", onclick: move |_| view.set(View::default()), "100 %" }
                button { style: "{theme::button_style(true)}", onclick: load_test_page, "Page test" }
                button { style: "{theme::button_style(false)}", onclick: clear_page, "Vider" }
                span { style: "{theme::label_style()}",
                    "p.{page} | zoom {(view().zoom * 100.0).round()} % | {status}"
                }
            }

            // ---- canvas area (wgpu) ----
            div {
                style: "{theme::canvas_area_style(canvas_rect)}",
                onwheel: move |e| {
                    e.prevent_default();
                    let d = e.data().delta().strip_units();
                    let factor = (-(d.y as f32) * 0.0015).exp();
                    let p = e.data().element_coordinates();
                    zoom_by(factor, Some((p.x as f32, p.y as f32)));
                },
                onpointerdown: move |e| {
                    let p = e.data().client_coordinates();
                    drag.set(Some((p.x, p.y)));
                },
                onpointermove: move |e| {
                    if let Some((lx, ly)) = drag() {
                        let p = e.data().client_coordinates();
                        let mut v = view();
                        v.pan_x += (p.x - lx) as f32;
                        v.pan_y += (p.y - ly) as f32;
                        view.set(v);
                        drag.set(Some((p.x, p.y)));
                    }
                },
                onpointerup: move |_| drag.set(None),
                onpointerleave: move |_| drag.set(None),
                ondoubleclick: move |_| fit(),

                // Web only: the <canvas> is the wgpu surface. On desktop the
                // area stays transparent and the window surface shows through.
                if cfg!(all(feature = "web", not(feature = "desktop"))) {
                    canvas {
                        style: "{theme::canvas_el_style()}",
                        width: "{platform::frame_geometry(canvas_rect).surface_w}",
                        height: "{platform::frame_geometry(canvas_rect).surface_h}",
                        onmounted: {
                            let painter = painter.clone();
                            move |e| platform::attach(Some(e), painter.clone(), gpu_ready, status)
                        },
                    }
                }
            }

            // ---- page strip ----
            div { style: "{theme::strip_style(c.strip)}",
                for i in 1..=PAGE_COUNT {
                    div {
                        key: "{i}",
                        style: "{theme::thumb_style(page() == i)}",
                        onclick: move |_| {
                            page.set(i);
                            view.set(View::default());
                        },
                        "{i:02}"
                    }
                }
            }
        }
    }
}
