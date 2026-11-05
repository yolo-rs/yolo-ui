# `yolo-ui`: Aphrody Labs unified fullstack framework

`yolo-ui` is the umbrella crate for the Aphrody Labs fullstack Rust UI monorepo (`aphrody-labs/yolo-ui`). It puts Dioxus 0.8, Blitz, AnyRender, Taffy, Dioxus Primitives / Components, Dioxus SDK, Dioxus Icons, Dioxus Code, RSRPC and the Aphrody Material 3 / Dark-Glass tokens and app shell in one Cargo workspace.

## Features

- `default`: `lib`, `router`
- Renderers and fullstack:
  - `fullstack` + `server` + `web`: SSR, hydration and Axum server functions (`#[server]`, `#[get]`, `#[post]`)
  - `desktop` / `mobile`: Wry/Tao native WebView shell for Windows, Linux, macOS, Android, and iOS
  - `native`: GPU-accelerated HTML/CSS rendering via `blitz` + `anyrender` (Vello / Skia / WGPU) + `taffy`
  - `liveview`: server-streamed WebSocket VirtualDOM
  - `ssr`: static and dynamic HTML rendering
- Ecosystem libraries:
  - `primitives`: headless accessible components (`dioxus-primitives`, `dioxus-attributes`). Opt in with `features = ["primitives"]` once the 0.7 component package is migrated to the Dioxus 0.8 module and event API.
  - `sdk`: clipboard, storage, time, geolocation, notifications, window and sync (`dioxus-sdk`). Opt in with `features = ["sdk"]` once the 0.7 SDK adapters are migrated to the Dioxus 0.8 core API.
  - `icons`: Lucide SVG icon components (`dioxus-icons`). Opt in with `features = ["icons"]`. The generated icon VDOM still targets the previous Dioxus template API and must be regenerated or migrated before the default Dioxus 0.8 build can enable it.
  - `code`: syntax-highlighted code blocks and editor (`dioxus-code`)
  - `rpc`: type-safe RPC (`rsrpc`)
