# 12. Iced stack (`iced-rs` / `aphrody-labs/iced` and `packages/yolo-iced`)

Yolo UI brings the whole `iced-rs` Elm-architecture GUI ecosystem into the monorepo through 10 forked repositories under `github.com/aphrody-labs/*` (checked out in `submodules/*`) and the `yolo-iced` bridge crate (`packages/yolo-iced`).

## 1. Forked `iced-rs` repository fleet (`aphrody-labs/*`)

All 10 upstream `iced-rs` repositories are maintained as `aphrody-labs` forks and checked out under `submodules/`:

| Submodule path            | Upstream repository    | Aphrody Labs fork           | Primary responsibility                                                                                                                                                                  |
| :------------------------ | :--------------------- | :-------------------------- | :-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `submodules/iced`         | `iced-rs/iced`         | `aphrody-labs/iced`         | Core Elm-architecture GUI framework (`iced`, `iced_core`, `iced_runtime`, `iced_futures`, `iced_widget`, `iced_renderer`, `iced_wgpu`, `iced_tiny_skia`, `iced_winit`, `iced_devtools`) |
| `submodules/iced_aw`      | `iced-rs/iced_aw`      | `aphrody-labs/iced_aw`      | Additional community widgets (`Modal`, `Split`, `Tabs`, `Badge`, `Card`, `ColorPicker`, `NumberInput`, `Wrap`, `ContextMenu`)                                                           |
| `submodules/iced_audio`   | `iced-rs/iced_audio`   | `aphrody-labs/iced_audio`   | Audio/DSP plugin widgets (`Knob`, `HSlider`, `VSlider`, `XYPad`, `Ramp`, `FreqRange`, `LogDBRange`, `Normal`, `ModRange`)                                                               |
| `submodules/comet`        | `iced-rs/comet`        | `aphrody-labs/comet`        | Time-travel debugger and runtime telemetry daemon for Iced MVU applications                                                                                                             |
| `submodules/cryoglyph`    | `iced-rs/cryoglyph`    | `aphrody-labs/cryoglyph`    | High-performance WGPU text layout and glyph atlas renderer (`cosmic-text` + `swash` + `wgpu`)                                                                                           |
| `submodules/iced-opengl`  | `iced-rs/opengl`       | `aphrody-labs/iced-opengl`  | OpenGL / Glow renderer backend integration for Iced                                                                                                                                     |
| `submodules/iced-book`    | `iced-rs/book`         | `aphrody-labs/iced-book`    | Official Iced book & architectural guide                                                                                                                                                |
| `submodules/mdbook-iced`  | `iced-rs/mdbook-iced`  | `aphrody-labs/mdbook-iced`  | mdBook preprocessor for interactive Iced documentation                                                                                                                                  |
| `submodules/iced-rfcs`    | `iced-rs/rfcs`         | `aphrody-labs/iced-rfcs`    | Design RFCs for widgets, layout, reactive tasks, and GPU rendering                                                                                                                      |
| `submodules/awesome-iced` | `iced-rs/awesome-iced` | `aphrody-labs/awesome-iced` | Curated catalog of Iced applications, libraries, and ecosystem crates                                                                                                                   |

## 2. Crate topology inside `submodules/iced`

```
submodules/iced/
├── src/                # Top-level `iced` facade crate (Application, Daemon, Task, Theme, Element)
├── core/               # `iced_core`: Widget trait, Layout, Event, Clipboard, Color, Theme/Palette
├── runtime/            # `iced_runtime`: Task<T>, Action, window/clipboard/system operations
├── futures/            # `iced_futures`: Subscription<T>, stream executors (tokio/smol/wasm)
├── widget/             # `iced_widget`: Built-in widgets (Button, Column, Row, Scrollable, Canvas, Shader, PaneGrid)
├── renderer/           # `iced_renderer`: Hybrid renderer selector (iced_wgpu + iced_tiny_skia fallback)
├── wgpu/               # `iced_wgpu`: Hardware-accelerated WGPU quad, triangle, image, shader, and text pipeline
├── tiny_skia/          # `iced_tiny_skia`: Software rasterizer backend for CPU fallback
├── winit/              # `iced_winit`: Windowing event loop integration
└── devtools/           # `iced_devtools`: Comet time-travel debugger client integration
```

## 3. `packages/yolo-iced` architecture

`packages/yolo-iced` brings the Iced Elm Architecture (MVU) and the Aphrody Material 3 Dark-Glass design system into the Yolo UI workspace. It has four modules.

1. Aphrody Material 3 Dark-Glass palette (`IcedTheme::aphrody_dark()`). It maps `APHRODY_DARK_THEME` (`#090b10` background, `#111520` surface container, `#181e2e` high surface container, `#f3f6fc` primary text, `#7c5cff` primary accent, `#00d4ff` secondary accent) directly into `IcedPalette` (`background`, `text`, `primary`, `success`, `warning`, `danger`) and `IcedSurfaceTokens`.
2. Elm MVU runtime and task graph (`yolo_iced::mvu`). `IcedApplication`, `Task<Message>`, `SubscriptionDescriptor` and `MvuRuntime<App>` let MVU state machines be unit-tested headlessly, inspected with `CometRecorder`, or bridged with Dioxus signals (`dioxus-signals`).
3. Extended and audio DSP widget specs (`yolo_iced::widgets`). These are serializable state and parameter curves from `iced_audio` (`NormalParam`, `FreqRange`, `ModRange`, `KnobConfig`, `FaderConfig`) and `iced_aw` (`SplitPaneConfig`, `ModalConfig`).
4. Comet telemetry and Cryoglyph diagnostics (`yolo_iced::comet`). A ring-buffered `CometRecorder` captures `CometTraceSpan` entries for every MVU message and `CryoglyphAtlasStats` for observing the GPU glyph cache.
