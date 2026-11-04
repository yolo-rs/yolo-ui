# GPUI & Longbridge Stack Architecture (`11-GPUI-LONGBRIDGE.md`)

Yolo UI unifies the **Zed GPUI (`zed-industries/*`)** and **Longbridge (`longbridge/*`)** GPU-accelerated desktop, mobile, terminal, localization, and embedded browser ecosystems under `aphrody-labs/*` submodules and the `packages/yolo-gpui` bridge crate.

---

## 1. Forked `aphrody-labs/*` Repository Fleet (Longbridge + Zed Industries)

### Longbridge Ecosystem (`longbridge/*` -> `aphrody-labs/*`)

| Upstream Repository              | Aphrody Labs Fork                  | Submodule Path           | Role in Yolo UI                                                                                                                                                    |
| :------------------------------- | :--------------------------------- | :----------------------- | :----------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `longbridge/gpui-kit`            | `aphrody-labs/gpui-kit`            | `submodules/gpui-kit`    | Rich desktop & workspace UI component kit for GPUI (DockArea, Panels, Tabs, Table, VirtualList, Tree, Charts, Markdown, CodeEditor, Theme, TitleBar, Modal/Drawer) |
| `longbridge/gpui-fast`           | `aphrody-labs/gpui-fast`           | `submodules/gpui-fast`   | Fast-compiling standalone GPUI workspace extracted from Zed (`gpui`, `gpui_macros`, `collections`, `util`, `sum_tree`, `scheduler`, `http_client`, `refineable`)   |
| `longbridge/gpui-mobile`         | `aphrody-labs/gpui-mobile`         | `submodules/gpui-mobile` | Cross-platform GPUI mobile backend targeting iOS (Metal/UIKit) and Android (Vulkan/GLES/JNI)                                                                       |
| `longbridge/nsh`                 | `aphrody-labs/nsh`                 | `submodules/nsh`         | GPU-accelerated native shell & terminal emulator built on GPUI                                                                                                     |
| `longbridge/rust-i18n`           | `aphrody-labs/rust-i18n`           | `submodules/rust-i18n`   | Compile-time codegen & runtime YAML/JSON/TOML localization (`t!` macro, pluralization, locale fallback)                                                            |
| `longbridge/wef`                 | `aphrody-labs/wef`                 | `submodules/wef`         | Web Embedded Framework (CEF / Wry hybrid embedding for GPUI and native Rust windows)                                                                               |
| `longbridge/autocorrect`         | `aphrody-labs/autocorrect`         | `submodules/autocorrect` | CJK + Latin copywriting linter and auto-spacer (`autocorrect`, `autocorrect-derive`, `autocorrect-lsp`)                                                            |
| `longbridge/openapi`             | `aphrody-labs/longbridge-openapi`  | _(on-demand)_            | Longbridge financial & market data OpenAPI Rust SDK (quotes, trading, portfolio streams)                                                                           |
| `longbridge/longbridge-terminal` | `aphrody-labs/longbridge-terminal` | _(on-demand)_            | Terminal-based market quote & portfolio workstation                                                                                                                |
| `longbridge/feishu-pages`        | `aphrody-labs/feishu-pages`        | _(on-demand)_            | Feishu/Lark document static site exporter                                                                                                                          |

### Zed Industries Ecosystem (`zed-industries/*` -> `aphrody-labs/*`)

| Upstream Repository               | Aphrody Labs Fork               | Submodule Path                | Role in Yolo UI                                                                                |
| :-------------------------------- | :------------------------------ | :---------------------------- | :--------------------------------------------------------------------------------------------- |
| `zed-industries/zed`              | `aphrody-labs/zed`              | _(via `gpui-fast`)_           | Upstream Zed editor & GPUI origin repository                                                   |
| `zed-industries/awesome-gpui`     | `aphrody-labs/awesome-gpui`     | `submodules/awesome-gpui`     | Curated directory of GPUI applications, libraries, and architectural patterns                  |
| `zed-industries/create-gpui-app`  | `aphrody-labs/create-gpui-app`  | `submodules/create-gpui-app`  | Official GPUI application starter template                                                     |
| `zed-industries/zed-fonts`        | `aphrody-labs/zed-fonts`        | `submodules/zed-fonts`        | Zed Plex Sans & Zed Plex Mono ligature fonts for code and high-density UI                      |
| `zed-industries/font-kit`         | `aphrody-labs/font-kit`         | `submodules/font-kit`         | Cross-platform system font loading & DirectWrite/CoreText/FreeType rasterization               |
| `zed-industries/scap`             | `aphrody-labs/scap`             | `submodules/scap`             | High-FPS native screen & window capture (ScreenCaptureKit, Windows.Graphics.Capture, PipeWire) |
| `zed-industries/async-dispatcher` | `aphrody-labs/async-dispatcher` | `submodules/async-dispatcher` | Lightweight task dispatcher bridging async futures with UI main threads                        |
| `zed-industries/extensions`       | `aphrody-labs/zed-extensions`   | _(on-demand)_                 | WebAssembly extension registry for languages, themes, and slash commands                       |

---

## 2. Crate Topology & Layering

```mermaid
flowchart TD
    YoloUI["packages/yolo-ui (Dioxus 0.8 + Blitz + AnyRender)"]
    YoloGPUI["packages/yolo-gpui (Theme, DockArea, I18nContext Bridge)"]
    GpuiKit["submodules/gpui-kit (60+ GPUI Workspace & Trading Widgets)"]
    GpuiFast["submodules/gpui-fast (Standalone GPUI Engine from Zed)"]
    GpuiMobile["submodules/gpui-mobile (iOS Metal / Android Vulkan GPUI)"]
    Nsh["submodules/nsh (GPUI Terminal & Shell)"]
    Wef["submodules/wef (Web Embedded Framework)"]
    RustI18n["submodules/rust-i18n + submodules/autocorrect"]
    ZedNative["submodules/font-kit + submodules/scap + submodules/async-dispatcher"]

    YoloUI <--> YoloGPUI
    YoloGPUI --> GpuiKit
    YoloGPUI --> RustI18n
    GpuiKit --> GpuiFast
    GpuiMobile --> GpuiFast
    Nsh --> GpuiFast
    Wef --> GpuiFast
    GpuiFast --> ZedNative
```

### Why `gpui-fast` + `gpui-kit`?

- Upstream `zed-industries/zed` contains over 180 internal editor crates (~2 GB checkout). `longbridge/gpui-fast` extracts only the core `gpui` rendering engine and its immediate support crates (`collections`, `gpui_macros`, `sum_tree`, `util`, `http_client`, `scheduler`) so GPUI compiles cleanly in seconds.
- `longbridge/gpui-kit` (formerly `gpui-component`) layers a comprehensive, production-tested desktop component system on top of `gpui-fast` (DockArea, Resizable Panels, Virtualized Tables, Charts, Markdown, Notification Center, Command Palette, Input fields with IME).

---

## 3. How `packages/yolo-gpui` Bridges GPUI with Yolo UI

`packages/yolo-gpui` provides a zero-bloat data and configuration bridge shared between Dioxus (`yolo-ui`) and GPUI (`gpui-kit` / `gpui-fast`):

1. **Unified Material 3 Dark-Glass Theme (`GpuiTheme`, `GpuiColorRgba`, `GpuiComponentStyle`)**:
   - Maps `yolo_ui::theme::APHRODY_DARK_THEME` (`#090b10` primary, `#111520` surface, `#181e2e` elevated, `#7c5cff` primary accent, `#00d4ff` secondary accent) directly into normalized float `GpuiColorRgba` (`0.0..=1.0`) and packed `0xRRGGBBAA` integers expected by `gpui::rgba()`.
   - Configures default typography to `Zed Plex Sans` and `Zed Plex Mono` from `submodules/zed-fonts`.
2. **Declarative Workspace & Dock Layout (`DockArea`, `PanelDescriptor`, `TabItem`, `StatusBarConfig`)**:
   - Enables serializable workspace layouts that can be rendered either by `gpui-kit`'s native `DockArea` or by Dioxus Web/Desktop/Blitz multi-pane shells.
3. **Localization & Copywriting (`LocaleCatalog`, `I18nContext`)**:
   - Mirrors `rust-i18n` key resolution and `%{var}` template interpolation so UI strings are shared across Dioxus components and GPUI views.
