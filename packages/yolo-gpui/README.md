# `yolo-gpui` — Aphrody Labs GPUI & Longbridge Stack Bridge

`yolo-gpui` unifies the **Zed GPUI (`gpui-fast`)** and **Longbridge (`gpui-kit`, `gpui-mobile`, `rust-i18n`, `wef`, `nsh`, `autocorrect`)** ecosystem with **Yolo UI (`packages/yolo-ui`)** and **Dioxus 0.8**.

## Key Capabilities

1. **Aphrody Material 3 Dark-Glass -> GPUI Theme & Styling Primitives**:
   - `GpuiColorRgba` (hex `#RRGGBB`/`#RRGGBBAA` & normalized float RGBA conversion compatible with `gpui::Rgba` / `gpui::Hsla`).
   - `GpuiTheme` & `GpuiComponentStyle` mapped directly from `APHRODY_DARK_THEME` tokens (`#090b10` primary background, `#111520` surface, `#7c5cff` primary accent, `#00d4ff` secondary accent).
   - `GpuiWindowOptions` for configuring GPU-accelerated desktop & mobile windows (`gpui-fast` & `gpui-mobile`).
2. **Dock, Panel & Workspace Layout Descriptors (`gpui-kit` + Zed Workspace)**:
   - `DockArea`, `DockPosition`, `SplitAxis`, `PanelDescriptor`, `TabItem`, and `StatusBarConfig` for declarative multi-pane IDE, trading terminal (`longbridge-terminal`), and agentic studio layouts.
3. **Built-in Localization & Autocorrect Catalog (`rust-i18n` + `autocorrect`)**:
   - `LocaleCatalog` and `I18nContext` providing zero-overhead key lookup, fallback chains, template variable interpolation, and CJK/Latin spacing normalization.
