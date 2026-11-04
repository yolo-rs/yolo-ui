<div align="center">

## Install as an agent plugin

```
/plugin marketplace add yolo-rs/yolo-ui
/plugin install yolo-ui --marketplace yolo-rs/yolo-ui
```

  <h1>⚡ Yolo UI (<code>yolo-rs/yolo-ui</code>)</h1>
  <p>
    <strong>Vendor-Neutral, Multi-Engine Fullstack Rust &amp; Bun v1.4.4+ UI Framework Monorepo</strong><br />
    <em>7 Theme Presets &bull; 124 Forked Submodules &bull; 9 Rust UI Toolkits &bull; Shadcn/Tailwind-to-Rust Migrator &bull; Full-Bun + Rust React 19 &amp; Expo Engine &bull; Unified <code>bun ui</code> CLI, Plugin &amp; MCP</em>
  </p>
</div>

---

## 1. Overview

**Yolo UI ([`yolo-rs/yolo-ui`](https://github.com/yolo-rs/yolo-ui))** unifies **124 forked repositories** in [`submodules/`](./submodules) across **31 upstream ecosystems** into **one flattened Cargo workspace**, **one Bun v1.4.4+ workspace (`@yolo-rs/*`)**, **8 first-class theme presets** (neutral by default via `YoloManifest::default()`), and **one unified CLI/Plugin/MCP (`bun ui`)**.

### The 7 Built-in Theme Presets (`YoloThemePreset`)

`yolo-ui` is **100% neutral and customizable by default** (`YoloThemePreset::Neutral`, `pet_companion: None`), with instant opt-in to 7 complete design systems:

| Preset ID | Variant | Seed (`0xAARRGGBB`) | Design System & Upstream Sources |
| :--- | :--- | :--- | :--- |
| **`yolo-neutral`** *(default)* | `YoloThemePreset::Neutral` | `0xFF6366F1` (`#6366F1`) | Vendor-neutral customizable zinc/slate surface with indigo accent |
| **`yolo-m3`** | `YoloThemePreset::M3` | `0xFF6750A4` (`#6750A4`) | Google Material Design 3 & Jetpack Compose (`material-color-utilities`, `androidx`, 49 HCT roles) |
| **`yolo-fluent`** | `YoloThemePreset::Fluent` | `0xFF0078D4` (`#0078D4`) | Microsoft Fluent UI 2 / WinUI 3 (`fluentui`, `microsoft-ui-xaml`, Mica/Acrylic/Smoke, `4px`/`8px` radii) |
| **`yolo-apple`** | `YoloThemePreset::Apple` | `0xFF007AFF` (`#007AFF`) | Apple macOS Tahoe Liquid Glass (`NSGlassEffectView`) & iOS Cupertino/SwiftUI HIG (`24px` blur, `10px`/`14px` squircle, traffic lights) |
| **`yolo-omarchy`** | `YoloThemePreset::Omarchy` | `0xFF7AA2F7` (`#7AA2F7`) | Omacom Omarchy Linux & `omarchy-site` TUI-modern design system + **all 23 official subthemes** (`Tokyo Night`, `Catppuccin`, `Osaka Jade`, `Lumon`, `Kanagawa`, `Gruvbox`, `Nord`, etc.) |
| **`yolo-hyprland`** | `YoloThemePreset::Hyprland` | `0xFF33CCFF` (`#33CCFF`) | Hyprwm / Hyprland Wayland Compositor (`#33CCFF -> #00FF99` `45deg` active border, `8×3` dual-pass Kawase blur, `hyprtoolkit`, `hyprqt6engine`) |
| **`yolo-aphrody`** | `YoloThemePreset::Aphrody` | `0xFF17335C` (`#17335C`) | Byron Love (Afuro Terumi / Aphrody) pet & brand theme (10 measured Oklch sprite-atlas colors `#F7F3F0`..`#131420`, `#FFD700` celestial gold, `#7C5CFF` divine violet, `8×11` `ByronLovePetSpec`) |
| **`yolo-chromium`** | `YoloThemePreset::Chromium` | `0xFF1A73E8` (`#1A73E8`) | Chromium Views + WebUI surfaces, browser-frame geometry and HCT-customizable Chrome blue seed; see [`docs/CHROMIUM_UI_DESIGN_SYSTEM.md`](docs/CHROMIUM_UI_DESIGN_SYSTEM.md) |

---

## 2. Flattened Monorepo Architecture (`crates/`, `packages/`, `apps/`, `skills/`)

### Core Rust Crates (`crates/*`)
- **[`crates/yolo-shared`](./crates/yolo-shared)**: Zero-duplication foundation (`RgbaColor`, `YoloManifest`, `YoloStyle`, `YoloThemePreset`, `YoloThemeTokens`, `OMARCHY_THEMES` [23], `HYPRLAND_DECORATION_SPEC`, `BYRON_LOVE_PALETTE` [10], `BYRON_LOVE_PET_SPEC`).
- **[`crates/yolo-tokens`](./crates/yolo-tokens)**: CAM16 / HCT color solver (`Hct::from_argb`, `TonalPalette`), 49 M3 roles, 9 `Variant`s, elevation, shape, motion, typography.
- **[`crates/yolo-contracts`](./crates/yolo-contracts)**: Cross-host accessibility (WCAG contrast solver), 31 component state/a11y contracts, adaptive window size classes (`Compact`/`Medium`/`Expanded`/`Large`/`ExtraLarge`), and 7-preset conformance checker.
- **[`crates/yolo-emit`](./crates/yolo-emit)**: Multi-preset theme emitter across **11 targets** (`Css`, `Gtk`, `Kde`, `Cosmic`, `Qml`, `Winui`, `Fluent`, `Apple`, `Omarchy`, `Hyprland`, `Dtcg`).
- **[`crates/yolo-layout`](./crates/yolo-layout)**: Taffy 0.15 Flexbox/Grid layout engine with 4px/8px grid snapping and WCAG 24×24dp / 48×48dp tap target enforcement.
- **[`crates/yolo-migrator`](./crates/yolo-migrator)**: `oxc_parser` TSX/JSX + Tailwind CSS v3/v4 (`@theme`) + Shadcn UI (`46` components, `cva()`) + Radix UI + Base UI compiler to **6 Rust targets** (`Dioxus`, `Wgpu`, `Egui`, `Cosmic`, `Gtk`, `Qml`).
- **[`crates/yolo-render`](./crates/yolo-render)** & **[`crates/yolo-gpu`](./crates/yolo-gpu)**: Analytical SDF rounded-rect / squircle / pill / shadow / Kawase blur scene primitives and `wgpu` 24 instanced GPU renderer.
- **[`crates/yolo-shell`](./crates/yolo-shell)**: Single-instance desktop lifecycle, `muda` 0.21 menu bar, `tray-icon` 0.26 system tray, `global-hotkey` 0.9, and `window-vibrancy` 0.8.
- **[`crates/yolo-craft`](./crates/yolo-craft)** & **[`crates/yolo-storyteller`](./crates/yolo-storyteller)**: 13 Creative Workbenches (`Writer`, `Screenplay`, `Storyboard`, `Comic`, `Novel`, `Worldbuilding`, `CharacterForge`, `Timeline`, `DialogueGraph`, `AudioStage`, `Animatic`, `LayoutPress`, `PublishHub`) and 10 Storyteller narrative studio modules.
- **[`crates/yolo-react`](./crates/yolo-react)**: Native Rust React 19 RSC Flight wire protocol serializer/SSR engine, React Native `StyleSheet` -> Taffy 0.15 layout solver, C++ JSI / TurboModule code generator, and OXC React hook/directive analyzer.

### Multi-Engine Rust & Bun Packages (`packages/*`)
- **[`packages/yolo-ui`](./packages/yolo-ui)**: Umbrella fullstack crate re-exporting Dioxus 0.8, Blitz, AnyRender, `yolo-shared`, `yolo-tokens`, `yolo-contracts`, `yolo-emit`, `yolo-layout`, `yolo-migrator`, `yolo-shell`, `yolo-craft`, `yolo-storyteller`, `yolo-react`, and all toolkit bridges.
- **[`packages/yolo-gpui`](./packages/yolo-gpui)**: Zed `gpui-fast` + Longbridge `gpui-kit` (60+ GPU widgets, `DockArea`) + `rust-i18n` bridge.
- **[`packages/yolo-iced`](./packages/yolo-iced)**: Iced Elm MVU + `iced_aw` + `iced_audio` DSP (`FreqRange`, `KnobConfig`, `FaderConfig`) + `comet` bridge.
- **[`packages/yolo-cosmic`](./packages/yolo-cosmic)**: System76 `libcosmic` + Iced 0.14 bridge with `cosmic-config` RON builder generation for all 7 presets.
- **[`packages/yolo-egui`](./packages/yolo-egui)**: Immediate-mode `egui` 0.36 bridge (`apply_preset`, `Visuals`, `Style`, `puffin` profiler integration).
- **[`packages/yolo-gtk`](./packages/yolo-gtk)**: `gtk4-rs` 0.11 + `libadwaita` 0.9 + `Relm4` bridge with live CSS provider generation.
- **[`packages/yolo-qt`](./packages/yolo-qt)**: KDAB `cxx-qt` 0.8 + Qt6 Quick / Kirigami `YoloTheme.qml` singleton bridge.
- **[`packages/yolo-tauri`](./packages/yolo-tauri)**: Tauri v3 Alpha (`3.0.0-alpha.5`, `Wry` 0.57.1 + `Cef` dual runtime, `Tao` 0.37.2, ACL capabilities, v2->v3 auditor).
- **[`packages/yolo-shadcn-bun`](./packages/yolo-shadcn-bun)** (`@yolo-rs/shadcn-bun`): Full-Bun CLI & programmatic API converting Shadcn/BaseUI/Radix/Tailwind TSX into Rust.
- **[`packages/yolo-react`](./packages/yolo-react)** (`@yolo-rs/react`): Full-Bun + Rust React 19 & React Native / Expo runtime (`useYoloTheme`, `YoloThemeProvider`, `createYoloStyleSheet`, `renderRscStream`, `m3-react`/`m3-baseui`/`m3-primitives`/`m3-forms`/`m3-motion` unified).
- **[`packages/yolo-ui-plugin`](./packages/yolo-ui-plugin)** (`@yolo-rs/ui-plugin`): Single `Bun.plugin` bundler & runtime loader for `.yolo.toml`, `.shadcn.tsx`, `.tailwind.css`, and `.omarchy.toml`.
- **[`packages/yolo-ui-mcp`](./packages/yolo-ui-mcp)** (`@yolo-rs/ui-mcp`): Single MCP JSON-RPC 2.0 server exposing `yolo_ui_search_design`, `yolo_ui_emit_theme`, `yolo_ui_migrate_tsx`, `yolo_ui_awesome_catalog`, `yolo_ui_devtools_inspect`, and `yolo_ui_pet_byron_love`.

---

## 3. Quick Start (`bun ui` & `just`)

```bash
# 1. Single unified UI command (search, theme, migrate, awesome, devtools, doctor, mcp)
bun run ui doctor
bun run ui search "glassmorphism dashboard" --stack gpui
bun run ui theme --preset yolo-omarchy --target omarchy --subtheme osaka-jade
bun run ui migrate --target dioxus --preset yolo-m3

# 2. Full-Bun + Rust React 19 & Expo CLI
bun run react:status

# 3. Run the unified 9-engine & 7-theme showcase binary
just showcase

# 4. Run all Bun v1.4.4+ & Rust unit tests (cached via sccache)
just test
```

---

## 4. Documentation

- **[Documentation Hub (`docs/README.md`)](./docs/README.md)**
- **[Yolo React 19 & Expo Full-Bun + Rust Architecture (`docs/YOLO-REACT-AND-EXPO-BUN-RUST.md`)](./docs/YOLO-REACT-AND-EXPO-BUN-RUST.md)**
- **[Flattened Monorepo Best Practices (`docs/MONOREPO-BEST-PRACTICES.md`)](./docs/MONOREPO-BEST-PRACTICES.md)**
- **[Node.js -> Bun v1.4.4+ (`n2b`) Migration Report (`docs/BUN-N2B-MIGRATION.md`)](./docs/BUN-N2B-MIGRATION.md)**
- **[UI/UX Pro Max Skill (`skills/yolo-ui-pro-max/SKILL.md`)](./skills/yolo-ui-pro-max/SKILL.md)**

## License

Dual-licensed under **MIT OR Apache-2.0**.
