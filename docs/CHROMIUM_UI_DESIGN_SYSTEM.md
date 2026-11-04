# Chromium Views and WebUI design system

`YoloThemePreset::Chromium` (`yolo-chromium`) is the Chromium-derived visual preset for products built with yolo-ui. It carries transferable design tokens and component contracts; Chromium's implementation remains upstream and is not mirrored into this repository.

## Source basis

The baseline was inspected in Chromium `4c7244a405d326e284710231ff609168dceb9044` (Chromium 159 checkout). The current sources of record are:

- [`ui/webui/resources/css/md_colors.css`](https://chromium.googlesource.com/chromium/src/+/4c7244a405d326e284710231ff609168dceb9044/ui/webui/resources/css/md_colors.css): WebUI background, toolbar height and toolbar divider in light and dark mode.
- [`ui/webui/resources/cr_elements/cr_shared_vars.css`](https://chromium.googlesource.com/chromium/src/+/4c7244a405d326e284710231ff609168dceb9044/ui/webui/resources/cr_elements/cr_shared_vars.css): shared card, text, control and border variables.
- [`ui/views/`](https://chromium.googlesource.com/chromium/src/+/4c7244a405d326e284710231ff609168dceb9044/ui/views/): native Views controls, layout, focus and accessibility behavior.
- [`chrome/browser/resources/`](https://chromium.googlesource.com/chromium/src/+/4c7244a405d326e284710231ff609168dceb9044/chrome/browser/resources/): Chrome WebUI surfaces and component composition.
- [`ash/`](https://chromium.googlesource.com/chromium/src/+/4c7244a405d326e284710231ff609168dceb9044/ash/): ChromeOS shell and system surfaces.

The full path inventory from YOLO-Web's local source crawler is [`yolo-web/engines/chromium-ui-crawl.json`](../../yolo-web/engines/chromium-ui-crawl.json); rerun `bun run crawl:chromium-ui` after syncing Chromium to refresh it.

## Theme contract

| Token | Dark | Light | Purpose |
| --- | --- | --- | --- |
| `bg_primary` | `#202124` | `#f8f9fa` | Browser or workbench canvas |
| `bg_surface` | `#292a2d` | `#ffffff` | Toolbar and primary surfaces |
| `bg_elevated` | `#35363a` | `#f1f3f4` | Cards, menus and raised controls |
| `text_primary` | `#e8eaed` | `#202124` | Main content and labels |
| `text_secondary` | `#9aa0a6` | `#5f6368` | Supporting text and metadata |
| `accent_primary` | `#8ab4f8` | `#1a73e8` | Focus, selected and primary actions |
| `accent_secondary` | `#a8c7fa` | `#1967d2` | Hover and secondary emphasis |
| `border_subtle` | white at 10% | `#dadce0` | Dividers and card outlines |
| Toolbar baseline | 56px | 56px | Chromium WebUI toolbar sizing |
| Card radius | 8px | 8px | `--cr-card-border-radius` |

The preset seed is Chrome blue (`#1A73E8`) and uses yolo-ui's HCT `Content` scheme for dynamic palettes. The fixed baseline surfaces preserve the Chromium WebUI character. Applications can derive a fresh accent and tonal palette from any seed while retaining the surface, density and component geometry contracts.

## Component composition map

| Chromium pattern | yolo-ui contract | Native source family |
| --- | --- | --- |
| Browser frame and tab strip | `BrowserFrame`, `Tab`, `TabStrip` | Views browser frame and tab strip |
| Omnibox and navigation controls | `Omnibox`, `ToolbarAction`, `SecurityIndicator` | Views location bar and toolbar |
| Menus and command surfaces | `Menu`, `MenuItem`, `MenuSeparator` | Views menu runner and WebUI action menu |
| Settings and detail cards | `SettingsSection`, `SettingsCard`, `SettingsRow` | WebUI `cr-card` and settings sections |
| Search and filter controls | `SearchField`, `FilterChip`, `ListRow` | WebUI `cr-search-field`, Views controls |
| ChromeOS shelf and system tray | `Shelf`, `SystemTray`, `QuickSettingsTile` | Ash shelf, status area and quick settings |

The names above are semantic component contracts for consistent cross-platform implementations. They do not claim binary or source compatibility with Chromium C++ Views or `cr-*` Web Components. YOLO-Web-authored web surfaces remain Bun TSX and native shell surfaces use Rust RSX; upstream behavior and state logic should be adapted with attribution rather than copying implementation wholesale.

## Customization

Keep the Chromium preset's structure while varying its inputs:

- Generate light and dark HCT schemes from an arbitrary user seed.
- Select a system, light or dark appearance independently from the accent seed.
- Adjust density and surface contrast through semantic tokens.
- Keep Chromium's toolbar, tab, omnibox, menu, settings-card and ChromeOS shell contracts while applying product-specific typography, shape and motion tokens.
- Allow full preset changes through `YoloThemePreset`; custom seed schemes are not a new built-in preset.

Rust consumers use `YoloThemePreset::Chromium`, `YoloThemePreset::tokens(dark)`, and `yolo_tokens::presets::yolo_chromium(dark)`. Bun consumers use `normalizePresetId("chromium")`, `resolveThemeTokens("chromium")`, or `emitThemeCode({ preset: "chromium", target: "dtcg" })`.
