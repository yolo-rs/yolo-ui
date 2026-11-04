# `yolo-iced` — Aphrody Labs Iced Stack & Elm MVU Integration

`yolo-iced` unifies the forked **`aphrody-labs/iced`** ecosystem (`iced`, `iced_aw`, `iced_audio`, `comet`, `cryoglyph`, `iced-opengl`, `iced-book`, `mdbook-iced`, `iced-rfcs`, and `awesome-iced`) into the **Yolo UI** monorepo (`aphrody-labs/yolo-ui`).

## Key Capabilities

1. **Elm Architecture (MVU) Bridge (`mvu` module)**:
   - Pure Rust `IcedApplication`, `Task<Message>` (formerly `Command<Message>`), `Subscription<Message>`, and `MvuRouter<State, Message>` abstractions that can run standalone or synchronize bi-directionally with Dioxus reactive signals (`dioxus-signals`).
2. **Aphrody Material 3 Dark-Glass Palette & Theme (`theme` module)**:
   - Deterministic hex/RGBA parser and `IcedPalette` + `IcedExtendedPalette` generator matching `iced_core::theme::Palette` (`background`, `text`, `primary`, `success`, `warning`, `danger`) and extended surface containers.
3. **Extended & Audio DSP Widget Descriptors (`widgets` module)**:
   - Declarative configurations for `iced_aw` (`ModalConfig`, `SplitPaneConfig`, `TabBarConfig`, `BadgeConfig`) and `iced_audio` (`KnobConfig`, `FaderConfig`, `XYPadConfig`, `ModRange`, `NormalParam`, `FreqRange`).
4. **Comet Elm Debugger & Cryoglyph Text Telemetry (`comet` module)**:
   - Time-travel MVU message span tracing (`CometTraceSpan`, `CometRecorder`) compatible with `aphrody-labs/comet` and glyph atlas diagnostics for `aphrody-labs/cryoglyph`.

## Submodule Fleet (`submodules/*`)

| Submodule Path            | Upstream Origin        | Aphrody Labs Fork           | Role                                                                                                                     |
| :------------------------ | :--------------------- | :-------------------------- | :----------------------------------------------------------------------------------------------------------------------- |
| `submodules/iced`         | `iced-rs/iced`         | `aphrody-labs/iced`         | Core Elm-inspired cross-platform GUI library (`iced_core`, `iced_runtime`, `iced_widget`, `iced_wgpu`, `iced_tiny_skia`) |
| `submodules/iced_aw`      | `iced-rs/iced_aw`      | `aphrody-labs/iced_aw`      | Additional widgets (modals, split panes, tabs, badges, color picker, number input)                                       |
| `submodules/iced_audio`   | `iced-rs/iced_audio`   | `aphrody-labs/iced_audio`   | Audio DSP UI widgets (knobs, faders, XY pads, modulation ranges, dB/Hz scales)                                           |
| `submodules/comet`        | `iced-rs/comet`        | `aphrody-labs/comet`        | Elm-architecture time-travel debugger & runtime telemetry inspector for Iced                                             |
| `submodules/cryoglyph`    | `iced-rs/cryoglyph`    | `aphrody-labs/cryoglyph`    | Fast GPU text renderer for `wgpu` powering Iced's text pipeline                                                          |
| `submodules/iced-opengl`  | `iced-rs/opengl`       | `aphrody-labs/iced-opengl`  | OpenGL / Glow renderer backend experiments and bindings for Iced                                                         |
| `submodules/iced-book`    | `iced-rs/book`         | `aphrody-labs/iced-book`    | Official Iced book and architectural guide                                                                               |
| `submodules/mdbook-iced`  | `iced-rs/mdbook-iced`  | `aphrody-labs/mdbook-iced`  | mdBook preprocessor for interactive Iced examples                                                                        |
| `submodules/iced-rfcs`    | `iced-rs/rfcs`         | `aphrody-labs/iced-rfcs`    | Architectural RFCs for the Iced ecosystem                                                                                |
| `submodules/awesome-iced` | `iced-rs/awesome-iced` | `aphrody-labs/awesome-iced` | Curated catalog of Iced applications, widgets, and plugins                                                               |
