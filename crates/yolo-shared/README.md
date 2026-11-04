# `yolo-shared` — Unified Foundation Crate (`crates/yolo-shared`)

`yolo-shared` is the shared foundation crate at the bottom of the `aphrody-labs/yolo-ui` dependency graph:

```text
apps/yolo-showcase
  └── packages/yolo-ui (Umbrella: Dioxus 0.8 + Blitz + AnyRender + Taffy)
        ├── packages/yolo-gpui  (Zed GPUI + Longbridge GPUI-Kit + Rust-i18n)
        ├── packages/yolo-iced  (Iced Elm MVU + Iced_AW + Iced_Audio + Comet)
        ├── packages/yolo-tauri (Tauri v3 Alpha + Wry 0.57 + Tao 0.37 + CEF)
        └── crates/yolo-shared  (Shared M3 Dark-Glass Tokens, RgbaColor, Fleet Manifest)
```

All 4 UI/runtime engines share `yolo_shared::RgbaColor`, `yolo_shared::AphrodyThemeTokens`, and `yolo_shared::APHRODY_DARK_THEME` without duplicating color parsing, CSS variable generation, or palette math.
