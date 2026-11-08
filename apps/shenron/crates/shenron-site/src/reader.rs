//! Reader island: the mount point of the wgpu manga painter.

use leptos::prelude::*;

use crate::i18n::{Lang, t};

/// DOM id of the reader surface. Keep in sync with the painter bootstrap.
pub const READER_CANVAS_ID: &str = "reader";

/// Placeholder island rendering `<canvas id="reader">`.
///
/// The sibling crate `shenron-reader-poc` (`src/gpu.rs`) owns the WebGPU
/// painter: `gpu::Painter::new(...)` takes a `wgpu::Surface` created from this
/// very element (`platform/web.rs` wraps the `HtmlCanvasElement` into
/// `wgpu::SurfaceTarget::Canvas`), then `Painter::set_image` uploads the
/// decoded page and `Painter::paint(rect, view, dpr)` draws it with the Taffy
/// layout rect. On this site the island is server-rendered as an empty canvas
/// and the painter is attached client-side after hydration, reading
/// `data-volume` / `data-chapter` to fetch the first plate. Until then the
/// canvas keeps the `--color-reader-bg` background from the generated CSS.
#[island]
pub fn ReaderIsland(lang: String, volume: u32, chapter: u32) -> impl IntoView {
    let lang = Lang::parse(&lang).unwrap_or(Lang::DEFAULT);
    view! {
        <section class="reader" aria-label=t(lang, "manga.reader_label")>
            <canvas
                id=READER_CANVAS_ID
                data-volume=volume.to_string()
                data-chapter=chapter.to_string()
                width="1600"
                height="2400"
                role="img"
                aria-label=t(lang, "manga.reader_label")
            ></canvas>
            <p class="muted">{t(lang, "manga.reader_fallback")}</p>
        </section>
    }
}
