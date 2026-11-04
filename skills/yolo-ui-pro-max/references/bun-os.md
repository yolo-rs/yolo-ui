# Yolo UI (`yolo-rs/yolo-ui`) Unified Multi-Engine Ecosystem

Map of the UI owners in the Yolo UI workspace (`yolo-rs/yolo-ui`, branch `main`). Paths are
repository-relative. Search the matching data with `--stack dioxus`, `--stack gpui`, `--stack egui`,
`--stack iced-libcosmic`, `--stack bevy`, `--stack tauri-v3`, `--stack gtk4-rs`, `--stack cxx-qt`,
`--stack base-ui`, `--stack shadcn`, `--stack html-tailwind`, `--stack omarchy`, `--stack hyprland`,
`--stack m3`, and the `brands.csv` palette rows.

## Which layer to use

| Building                                      | Use                                                                                 | Why                                                                                                                     |
| --------------------------------------------- | ----------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------- |
| Multi-engine Rust + Web theme & tokens        | `crates/yolo-shared` + `crates/yolo-tokens` + `crates/yolo-emit`                    | 7 `YoloThemePreset`s, 23 Omarchy sub-themes, 10 Byron Love teintes, and 11 native emitters (`css`, `gtk`, `kde`, etc.)  |
| Any Bun v1.4.4+ build, virtual CSS, TSX->Rust | `@yolo-rs/ui-plugin` (`packages/yolo-ui-plugin`)                                    | `Bun.plugin` resolving `virtual:yolo-theme`, `yolo-ui/theme.css`, `.tsx?rust` imports, and `DEVTOOLS_REGISTRY`          |
| Unified CLI, MCP Server & DevTools            | `bun ui` (`scripts/ui.ts`) + `@yolo-rs/ui-mcp` (`packages/yolo-ui-mcp`)             | Single command and single 10-tool MCP server unifying search, design system, theme emission, migration, and devtools    |
| React / Shadcn / Radix / Base UI -> Rust      | `crates/yolo-migrator` (`bun ui migrate`)                                           | Converts TSX primitives to `dioxus`, `wgpu`, `egui`, `cosmic`, `gtk`, and `qml`                                         |
| Creative Workbenches & Storyteller AI         | `crates/yolo-craft` (13 workbenches) + `crates/yolo-storyteller` (10 modules)       | `CadCraft`..`WordCraft` and `PromptBox`..`CharacterSheet` across all 10 engines                                         |

## Packages & Crates

### Core Rust Crates (`crates/`)

- `yolo-shared`: `RgbaColor`, `YoloStyle`, `YoloThemePreset` (`Neutral`, `M3`, `Fluent`, `Apple`, `Omarchy`, `Hyprland`, `Aphrody`), `OmarchySubTheme` (23 themes), `HYPRLAND_DECORATION_SPEC`, `BYRON_LOVE_PALETTE`, WCAG 2.2 contrast calculator.
- `yolo-tokens`: Semantic token graphs (`ColorToken`, `SpacingToken`, `RadiusToken`, `ShadowToken`, `TypographyToken`, `MotionToken`), OKLCH color conversions, and W3C DTCG JSON export.
- `yolo-contracts`: Platform adaptation contracts (`WindowClass`: Compact/Medium/Expanded/Large/ExtraLarge, `InputModality`, `PlatformId`, `DensityScale`, `MotionPolicy`).
- `yolo-emit`: Native code emitters for 11 targets (`css`, `gtk`, `kde`, `cosmic`, `qml`, `winui`, `fluent`, `apple`, `omarchy`, `hyprland`, `dtcg`).
- `yolo-layout`: Taffy 0.15 flexbox/grid layout engine bridge + AccessKit a11y tree builder.
- `yolo-migrator`: React / Shadcn / Radix / Base UI / Tailwind TSX to Rust (`dioxus`, `wgpu`, `egui`, `cosmic`, `gtk`, `qml`) transpiler.
- `yolo-craft`: 13 creative workbenches (`CadCraft`, `MeshCraft`, `PixelCraft`, `VectorCraft`, `CutCraft`, `MotionCraft`, `WaveCraft`, `TuneCraft`, `LevelCraft`, `NodeCraft`, `SheetCraft`, `DeckCraft`, `WordCraft`).
- `yolo-storyteller`: 10 multimodal creative AI studio modules (`PromptBox`, `VoiceSelector`, `StyleWheel`, `CameraRig`, `TimelineEditor`, `NodeGraph`, `MediaViewer`, `AssetBrowser`, `ExportQueue`, `CharacterSheet`).

### Engine Bridges & Workspace Packages (`packages/`)

- `yolo-ui`: Umbrella Rust fullstack crate re-exporting all 10 engine bridges.
- `yolo-gpui`, `yolo-iced`, `yolo-tauri`, `yolo-egui`, `yolo-bevy`, `yolo-cosmic`, `yolo-gtk4`, `yolo-qt`: Native Rust UI engine adapters.
- `yolo-ui-plugin` (`@yolo-rs/ui-plugin`): Unified Bun plugin (`yoloUiBunPlugin`) and 10-stack `DEVTOOLS_REGISTRY`.
- `yolo-ui-mcp` (`@yolo-rs/ui-mcp`): Unified 10-tool MCP server (`ui_search`, `ui_design_system`, `ui_theme_emit`, `ui_migrate`, `ui_contrast`, `ui_devtools`, `ui_awesome`, `ui_catalog_lookup`, `ui_workbenches`, `ui_fleet_status`).

## Themes and Selectors (`data/brands.csv`)

| Brand ID (`brands.csv`) | Preset (`YoloThemePreset`) | Selector                | Accent / Primary              |
| ----------------------- | -------------------------- | ----------------------- | ----------------------------- |
| `yolo-neutral`          | `neutral`                  | `[data-theme="neutral"]`| `#6366F1` (Indigo)            |
| `yolo-m3`               | `m3`                       | `[data-theme="m3"]`     | `#6750A4` / `#D0BCFF` (M3)    |
| `yolo-fluent`           | `fluent`                   | `[data-theme="fluent"]` | `#0078D4` / `#60CDFF` (Mica)  |
| `yolo-apple`            | `apple`                    | `[data-theme="apple"]`  | `#007AFF` / `#0A84FF` (HIG)   |
| `yolo-omarchy`          | `omarchy`                  | `[data-theme="omarchy"]`| `#89B4FA` (23 TUI sub-themes) |
| `yolo-hyprland`         | `hyprland`                 | `[data-theme="hyprland"]`| `#33CCFF` -> `#00FF99` (45deg)|
| `yolo-aphrody`          | `aphrody`                  | `[data-theme="aphrody"]`| `#E8C547` (Byron Love 10-tone)|

## Accessibility Defaults

Visible focus with a forced-colors outline (`CanvasText` / `2px solid`), `prefers-reduced-motion` gating across all animations, WCAG 2.2 AA (`4.5:1` text, `3:1` UI) verified via `bun ui contrast`, 44px minimum touch targets, `aria-current="page"` in navigation, and AccessKit semantic trees across all native Rust renderers.
