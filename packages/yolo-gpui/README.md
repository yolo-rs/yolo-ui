# `yolo-gpui`: Aphrody Labs GPUI and Longbridge stack bridge

`yolo-gpui` connects the Zed GPUI (`gpui-fast`) and Longbridge (`gpui-kit`, `gpui-mobile`, `rust-i18n`, `wef`, `nsh`, `autocorrect`) ecosystem to Yolo UI (`packages/yolo-ui`) and Dioxus 0.8.

## Capabilities

1. Aphrody Material 3 dark-glass theme and styling primitives for GPUI:
   - `GpuiColorRgba` converts hex `#RRGGBB`/`#RRGGBBAA` and normalized float RGBA, compatible with `gpui::Rgba` / `gpui::Hsla`.
   - `GpuiTheme` and `GpuiComponentStyle` map directly from the `APHRODY_DARK_THEME` tokens (`#090b10` primary background, `#111520` surface, `#7c5cff` primary accent, `#00d4ff` secondary accent).
   - `GpuiWindowOptions` configures GPU-accelerated desktop and mobile windows (`gpui-fast` & `gpui-mobile`).
2. Dock, panel and workspace layout descriptors (`gpui-kit` + Zed Workspace):
   - `DockArea`, `DockPosition`, `SplitAxis`, `PanelDescriptor`, `TabItem` and `StatusBarConfig` describe multi-pane IDE, trading terminal (`longbridge-terminal`) and agentic studio layouts declaratively.
3. Built-in localization and autocorrect catalog (`rust-i18n` + `autocorrect`):
   - `LocaleCatalog` and `I18nContext` provide zero-overhead key lookup, fallback chains, template variable interpolation, and CJK/Latin spacing normalization.
