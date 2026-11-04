# `yolo-ui` — Aphrody Labs Unified Fullstack Framework

`yolo-ui` is the umbrella crate for the Aphrody Labs fullstack Rust UI monorepo (`aphrody-labs/yolo-ui`), unifying **Dioxus 0.8**, **Blitz**, **AnyRender**, **Taffy**, **Dioxus Primitives / Components**, **Dioxus SDK**, **Dioxus Icons**, **Dioxus Code**, **RSRPC**, and **Aphrody Material 3 / Dark-Glass Tokens & App Shell** in one Cargo workspace.

## Features

- **`default`**: `lib`, `router`
- **Renderers & Fullstack**:
  - `fullstack` + `server` + `web`: SSR + Hydration + Axum Server Functions (`#[server]`, `#[get]`, `#[post]`)
  - `desktop` / `mobile`: Wry/Tao native WebView shell for Windows, Linux, macOS, Android, and iOS
  - `native`: GPU-accelerated HTML/CSS rendering via `blitz` + `anyrender` (Vello / Skia / WGPU) + `taffy`
  - `liveview`: Server-streamed WebSocket VirtualDOM
  - `ssr`: Static & dynamic HTML rendering
- **Ecosystem Batteries**:
  - `primitives`: Headless accessible components (`dioxus-primitives`, `dioxus-attributes`); opt in with `features = ["primitives"]` after the 0.7 component package is migrated to Dioxus 0.8's module and event API.
  - `sdk`: Clipboard, storage, time, geolocation, notifications, window, sync (`dioxus-sdk`); opt in with `features = ["sdk"]` after the 0.7 SDK adapters are migrated to the Dioxus 0.8 core API.
  - `icons`: Lucide SVG icon components (`dioxus-icons`); opt in with `features = ["icons"]`. The generated icon VDOM currently targets the previous Dioxus template API and must be regenerated/migrated before it can be enabled in the default Dioxus 0.8 build.
  - `code`: Syntax-highlighted code blocks & editor (`dioxus-code`)
  - `rpc`: Type-safe RPC (`rsrpc`)
