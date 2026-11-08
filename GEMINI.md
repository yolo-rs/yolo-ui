# GEMINI.md — YOLO-UI Design System, Longbridge GPUI-Kit & Dioxus 0.8 (`C:\src\yolo-ui`)

> **Doctrine**: No Human In The Loop (NHITL) | 100% Upstream Bun v1.4.4+ & Rust 2024 | 120 FPS Native GPU UI

## 1. Architecture (`C:\src\yolo-ui`)
- **Desktop & Studio UI**: Longbridge `gpui-kit` v0.8 (`gpui-base`, `gpui-component`, `gpui-shell`, `DockArea`, `DataTable`, forked from Zed GPUI 0.8) + Taffy 0.15 + WGPU 30.0.1 / Skia 0.156 at 120 FPS.
- **Web & Hybrid SSR/Native UI**: Dioxus 0.8 + Blitz + Taffy + Manganis + Subsecond hot-patching.
- **HCT / CAM16 Chromatic Engine**: Perceptual color science (`design.google` tokens) with 7 brand presets (`Neutral`, `M3`, `Fluent`, `Apple`, `Omarchy`, `Hyprland`, `Aphrody`) and deduplicated Phosphor vector icons.
- **Integration with `yolo-engine`**: Shared WGPU 30 render surfaces and Storytold `*Craft` creative suite viewports.

## 2. Operating Directives
- Zero emojis in all code, UI components, commits, and responses.
- Validate changes with `bun fmt`, `bun check`, `bun test`, and `cargo test`.
