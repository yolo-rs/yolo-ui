# yolo-stack-leptos (planned, optional)

Minimal-wasm islands adapter on Leptos. Not a workspace member until code lands.

Planned dependencies: `leptos`, `leptos_router`, `leptos_axum` pinned `=0.8.23` from crates.io
(Leptos is not vendored in any YOLO repository; `yolo-rs/*` has no Leptos fork). `islands` feature on,
`hydrate_islands()` entry, `HydrationScripts islands=true`.

Surface: identical to `yolo-stack-dioxus` (`render_island`, `/api/*`) so the TanStack shell does not
know which Rust framework rendered an island.

Entry condition: Leptos 0.9 stable (0.9.0-rc1 on 2026-10-31) and a `bench/` receipt showing the
`island-counter` route at a smaller gzip wasm than the Dioxus island.

Removal condition: two consecutive quarterly benchmark receipts where the Dioxus island is within
20 percent of the Leptos island on gzip wasm size and first-paint.
