<div align="center">

## Install as an agent plugin

```
/plugin marketplace add yolo-rs/yolo-ui
/plugin install yolo-ui --marketplace yolo-rs/yolo-ui
```

  <h1>Yolo UI (<code>yolo-rs/yolo-ui</code>)</h1>
  <p>
    <strong>Vendor-neutral, multi-engine fullstack Rust &amp; Bun v1.4.4+ UI framework monorepo</strong><br />
    <em>7 theme presets &bull; 151 forked submodules &bull; 2 absorbed apps (AGPL + Apache) &bull; 9 Rust UI toolkits &bull; Shadcn/Tailwind-to-Rust migrator &bull; Full-Bun + Rust React 19 &amp; Expo engine &bull; one <code>bun ui</code> CLI, plugin &amp; MCP</em>
  </p>
</div>

## 1. Overview

Yolo UI ([`yolo-rs/yolo-ui`](https://github.com/yolo-rs/yolo-ui)) provides Rust framework crates and toolkit adapters, a Bun v1.4.4+ workspace (`@yolo-rs/*`), and developer tooling. The active [`apps/yolo-showcase`](./apps/yolo-showcase) binary demonstrates the engines and themes together. Its gallery and benchmark tables are not runtime performance evidence. Framework APIs and imported upstream components each need their own build and runtime qualification, because being a workspace member does not prove support.

AnomalyCo's native, terminal, design, and application UI sources are forked into `yolo-rs` and pinned under [`upstreams/anomalyco/`](./upstreams/anomalyco).

The [Locally Uncensored](https://github.com/yolo-rs/locally-uncensored) desktop app stays separately identifiable at [`apps/locally-uncensored/`](./apps/locally-uncensored) and keeps its AGPL-3.0-only license. It is product code and does not show that the framework migration is complete.

[Spacedrive](https://github.com/yolo-rs/spacedrive) V2 is absorbed into [`apps/spacedrive/`](./apps/spacedrive/) with its Apache-2.0 license, Rust daemon/core, CLI, React packages, and Tauri desktop app. The [integration notes](./apps/spacedrive/YOLO-INTEGRATION.md) give the Cargo workspace and Tauri migration status.

### The 7 built-in theme presets (`YoloThemePreset`)

`yolo-ui` is neutral by default (`YoloThemePreset::Neutral`, `pet_companion: None`) and customizable. You can opt in to any of 7 design systems:

| Preset ID                      | Variant                     | Seed (`0xAARRGGBB`)      | Design system and upstream sources                                                                                                                                                       |
| :----------------------------- | :-------------------------- | :----------------------- | :--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `yolo-neutral` _(default)_     | `YoloThemePreset::Neutral`  | `0xFF6366F1` (`#6366F1`) | Vendor-neutral customizable zinc/slate surface with indigo accent                                                                                                                        |
| `yolo-m3`                      | `YoloThemePreset::M3`       | `0xFF6750A4` (`#6750A4`) | Google Material Design 3 & Jetpack Compose (`material-color-utilities`, `androidx`, 49 HCT roles)                                                                                        |
| `yolo-fluent`                  | `YoloThemePreset::Fluent`   | `0xFF0078D4` (`#0078D4`) | Microsoft Fluent UI 2 / WinUI 3 (`fluentui`, `microsoft-ui-xaml`, Mica/Acrylic/Smoke, `4px`/`8px` radii)                                                                                 |
| `yolo-apple`                   | `YoloThemePreset::Apple`    | `0xFF007AFF` (`#007AFF`) | Apple macOS Tahoe Liquid Glass (`NSGlassEffectView`) & iOS Cupertino/SwiftUI HIG (`24px` blur, `10px`/`14px` squircle, traffic lights)                                                   |
| `yolo-omarchy`                 | `YoloThemePreset::Omarchy`  | `0xFF7AA2F7` (`#7AA2F7`) | Omacom Omarchy Linux & `omarchy-site` TUI-modern design system + all 23 official subthemes (`Tokyo Night`, `Catppuccin`, `Osaka Jade`, `Lumon`, `Kanagawa`, `Gruvbox`, `Nord`, etc.)     |
| `yolo-hyprland`                | `YoloThemePreset::Hyprland` | `0xFF33CCFF` (`#33CCFF`) | Hyprwm / Hyprland Wayland compositor (`#33CCFF -> #00FF99` `45deg` active border, `8×3` dual-pass Kawase blur, `hyprtoolkit`, `hyprqt6engine`)                                           |
| `yolo-chromium`                | `YoloThemePreset::Chromium` | `0xFF1A73E8` (`#1A73E8`) | Chromium Views + WebUI surfaces, browser-frame geometry and HCT-customizable Chrome blue seed; see [`docs/CHROMIUM_UI_DESIGN_SYSTEM.md`](docs/CHROMIUM_UI_DESIGN_SYSTEM.md)              |

## 2. Flattened monorepo architecture (`crates/`, `packages/`, `apps/`, `skills/`)

### Core Rust crates (`crates/*`)

- [`crates/yolo-shared`](./crates/yolo-shared): shared foundation types, defined once (`RgbaColor`, `YoloManifest`, `YoloStyle`, `YoloThemePreset`, `YoloThemeTokens`, `OMARCHY_THEMES` [23], `HYPRLAND_DECORATION_SPEC`, `BYRON_LOVE_PALETTE` [10], `BYRON_LOVE_PET_SPEC`).
- [`crates/yolo-tokens`](./crates/yolo-tokens): CAM16 / HCT color solver (`Hct::from_argb`, `TonalPalette`), 49 M3 roles, 9 `Variant`s, elevation, shape, motion, typography.
- [`crates/yolo-contracts`](./crates/yolo-contracts): cross-host accessibility (WCAG contrast solver), 31 component state/a11y contracts, adaptive window size classes (`Compact`/`Medium`/`Expanded`/`Large`/`ExtraLarge`), and a 7-preset conformance checker.
- [`crates/yolo-emit`](./crates/yolo-emit): theme emitter for multiple presets with 11 targets (`Css`, `Gtk`, `Kde`, `Cosmic`, `Qml`, `Winui`, `Fluent`, `Apple`, `Omarchy`, `Hyprland`, `Dtcg`).
- [`crates/yolo-layout`](./crates/yolo-layout): Taffy 0.15 Flexbox/Grid layout engine that snaps to a 4px/8px grid and enforces WCAG 24×24dp / 48×48dp tap targets.
- [`crates/yolo-migrator`](./crates/yolo-migrator): compiler from `oxc_parser` TSX/JSX + Tailwind CSS v3/v4 (`@theme`) + Shadcn UI (`46` components, `cva()`) + Radix UI + Base UI to 6 Rust targets (`Dioxus`, `Wgpu`, `Egui`, `Cosmic`, `Gtk`, `Qml`).
- [`crates/yolo-render`](./crates/yolo-render) and [`crates/yolo-gpu`](./crates/yolo-gpu): analytical SDF scene primitives (rounded rect, squircle, pill, shadow, Kawase blur) and a `wgpu` 24 instanced GPU renderer.
- [`crates/yolo-shell`](./crates/yolo-shell): single-instance desktop lifecycle, `muda` 0.21 menu bar, `tray-icon` 0.26 system tray, `global-hotkey` 0.9, and `window-vibrancy` 0.8.
- [`crates/yolo-craft`](./crates/yolo-craft) and [`crates/yolo-storyteller`](./crates/yolo-storyteller): 13 creative workbenches (`Writer`, `Screenplay`, `Storyboard`, `Comic`, `Novel`, `Worldbuilding`, `CharacterForge`, `Timeline`, `DialogueGraph`, `AudioStage`, `Animatic`, `LayoutPress`, `PublishHub`) and 10 Storyteller narrative studio modules.
- [`crates/yolo-react`](./crates/yolo-react): native Rust serializer and SSR engine for the React 19 RSC Flight wire protocol, a React Native `StyleSheet` -> Taffy 0.15 layout solver, a C++ JSI / TurboModule code generator, and an OXC analyzer for React hooks and directives.

### Rust and Bun packages for each engine (`packages/*`)

- [`packages/yolo-ui`](./packages/yolo-ui): umbrella fullstack crate that re-exports Dioxus 0.8, Blitz, AnyRender, `yolo-shared`, `yolo-tokens`, `yolo-contracts`, `yolo-emit`, `yolo-layout`, `yolo-migrator`, `yolo-shell`, `yolo-craft`, `yolo-storyteller`, `yolo-react`, and all toolkit bridges.
- [`packages/yolo-gpui`](./packages/yolo-gpui): Zed `gpui-fast` + Longbridge `gpui-kit` (60+ GPU widgets, `DockArea`) + `rust-i18n` bridge.
- [`packages/yolo-iced`](./packages/yolo-iced): Iced Elm MVU + `iced_aw` + `iced_audio` DSP (`FreqRange`, `KnobConfig`, `FaderConfig`) + `comet` bridge.
- [`packages/yolo-cosmic`](./packages/yolo-cosmic): System76 `libcosmic` + Iced 0.14 bridge that generates `cosmic-config` RON builders for all 7 presets.
- [`packages/yolo-egui`](./packages/yolo-egui): immediate-mode `egui` 0.36 bridge (`apply_preset`, `Visuals`, `Style`, `puffin` profiler integration).
- [`packages/yolo-gtk`](./packages/yolo-gtk): `gtk4-rs` 0.11 + `libadwaita` 0.9 + `Relm4` bridge that generates a live CSS provider.
- [`packages/yolo-qt`](./packages/yolo-qt): KDAB `cxx-qt` 0.8 + Qt6 Quick / Kirigami `YoloTheme.qml` singleton bridge.
- [`packages/yolo-tauri`](./packages/yolo-tauri): Tauri v3 Alpha (`3.0.0-alpha.5`) with the Wry runtime, `Tao` 0.37.2, ACL capabilities, and the v2-to-v3 auditor. Browser engines such as Chromium and CEF belong to `yolo-web`, and no CEF Tauri adapter is integrated yet.
- [`packages/yolo-shadcn-bun`](./packages/yolo-shadcn-bun) (`@yolo-rs/shadcn-bun`): Full-Bun CLI and programmatic API that converts Shadcn/BaseUI/Radix/Tailwind TSX into Rust.
- [`packages/yolo-react`](./packages/yolo-react) (`@yolo-rs/react`): Full-Bun + Rust runtime for React 19 and React Native / Expo (`useYoloTheme`, `YoloThemeProvider`, `createYoloStyleSheet`, `renderRscStream`), which merges `m3-react`, `m3-baseui`, `m3-primitives`, `m3-forms` and `m3-motion`.
- [`packages/yolo-ui-plugin`](./packages/yolo-ui-plugin) (`@yolo-rs/ui-plugin`): one `Bun.plugin` bundler and runtime loader for `.yolo.toml`, `.shadcn.tsx`, `.tailwind.css`, and `.omarchy.toml`.
- [`packages/yolo-ui-mcp`](./packages/yolo-ui-mcp) (`@yolo-rs/ui-mcp`): one MCP JSON-RPC 2.0 server that exposes `yolo_ui_search_design`, `yolo_ui_emit_theme`, `yolo_ui_migrate_tsx`, `yolo_ui_awesome_catalog`, `yolo_ui_devtools_inspect`, and `yolo_ui_pet_byron_love`.

## 3. Quick start (`bun ui` and `just`)

```bash
# 1. Single unified UI command (search, theme, migrate, awesome, devtools, doctor, mcp)
bun run ui doctor
bun run ui search "glassmorphism dashboard" --stack gpui
bun run ui theme --preset yolo-omarchy --target omarchy --subtheme osaka-jade
bun run ui migrate --target dioxus --preset yolo-m3

# 2. Full-Bun + Rust React 19 & Expo CLI
bun run react:status

# 3. Spacedrive desktop, daemon, CLI, and Rust backend
bun run sd:tauri:dev
bun run sd:daemon
bun run sd:cli -- --help


# 5. Run all Bun v1.4.4+ & Rust unit tests (cached via sccache)
just test
```

## 4. Documentation

- [Documentation hub (`docs/README.md`)](./docs/README.md)
- [Yolo React 19 and Expo Full-Bun + Rust architecture (`docs/YOLO-REACT-AND-EXPO-BUN-RUST.md`)](./docs/YOLO-REACT-AND-EXPO-BUN-RUST.md)
- [Flattened monorepo best practices (`docs/MONOREPO-BEST-PRACTICES.md`)](./docs/MONOREPO-BEST-PRACTICES.md)
- [Spacedrive integration notes (`apps/spacedrive/YOLO-INTEGRATION.md`)](./apps/spacedrive/YOLO-INTEGRATION.md)
- [Node.js -> Bun v1.4.4+ (`n2b`) migration report (`docs/BUN-N2B-MIGRATION.md`)](./docs/BUN-N2B-MIGRATION.md)
- [UI/UX Pro Max skill (`skills/yolo-ui-pro-max/SKILL.md`)](./skills/yolo-ui-pro-max/SKILL.md)

## License

Dual-licensed under MIT OR Apache-2.0.
