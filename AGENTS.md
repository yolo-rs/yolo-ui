# Yolo UI (`yolo-rs/yolo-ui`): multi-engine fullstack Rust and Bun v1.4.4+ framework guide

Yolo UI (`yolo-rs/yolo-ui`) is a vendor-neutral, customizable fullstack Rust + Bun v1.4.4+ UI framework monorepo. It brings together 124 `yolo-rs/*` Git submodules (`submodules/*`) from 31 upstream UI and compiler ecosystems, 9 Rust UI toolkits, 8 built-in theme presets (`YoloThemePreset`), a Shadcn/BaseUI/Radix/Tailwind-to-Rust compiler (`yolo-migrator`), a Full-Bun + Rust React 19 and React Native/Expo engine (`yolo-react`), and one CLI, plugin and MCP server (`bun ui` / `@yolo-rs/ui-plugin` / `@yolo-rs/ui-mcp`).

## 1. The 7 first-class theme presets (`YoloThemePreset`)

`yolo-ui` is neutral by default (`YoloManifest::default()` uses `YoloThemePreset::Neutral` with `pet_companion: None`):

1. `yolo-neutral` (`YoloThemePreset::Neutral`, `#6366F1` default indigo seed): unbranded zinc/slate dark and light surfaces.
2. `yolo-m3` (`YoloThemePreset::M3`, `#6750A4`): Google Material Design 3 / Jetpack Compose upstream (`49` HCT roles, `material-color-utilities`).
3. `yolo-fluent` (`YoloThemePreset::Fluent`, `#0078D4`): Microsoft Fluent UI 2 / WinUI 3 (`Mica`, `Acrylic`, `Smoke`, `4px`/`8px` radii).
4. `yolo-apple` (`YoloThemePreset::Apple`, `#007AFF`): Apple macOS Sequoia/Tahoe Liquid Glass (`NSGlassEffectView`) and iOS Cupertino/SwiftUI HIG (`24px` vibrancy blur, `10px`/`14px` squircles, `#FF5F56`/`#FFBD2E`/`#27C93F` traffic lights).
5. `yolo-omarchy` (`YoloThemePreset::Omarchy`, `#7AA2F7`): Omacom Omarchy Linux and the `omarchy-site` TUI-modern design system, with all 23 official subthemes (`OmarchySubTheme::ALL`).
6. `yolo-hyprland` (`YoloThemePreset::Hyprland`, `#33CCFF`): Hyprwm / Hyprland Wayland compositor (`#33CCFF -> #00FF99` `45deg` active border, `8×3` Kawase blur, `hyprtoolkit`, `hyprqt6engine`).
7. `yolo-aphrody` (`YoloThemePreset::Aphrody`, `#17335C`): the Byron Love (Afuro Terumi / Aphrody) signature pet and brand theme (10 measured Oklch sprite-atlas colors `#F7F3F0`..`#131420`, `#FFD700` gold, `#7C5CFF` violet, `#00D4FF` cyan, `8×11` `ByronLovePetSpec`).

## 2. Flattened monorepo structure

```text
yolo-ui/
├── Cargo.toml                  # Unified Cargo workspace (all crates/*, packages/*, apps/*)
├── Justfile                    # Unified task runner (just + bun + cargo + sccache + gh)
├── package.json                # Root Bun v1.4.4+ workspace (@yolo-rs/yolo-ui)
├── catalog/conversion.toml     # Cross-host M3 + 7-preset role conversion matrix
├── crates/
│   ├── yolo-shared/            # Neutral manifest (YoloManifest), 7 presets, Omarchy (23), Hyprland, Byron Love
│   ├── yolo-tokens/            # CAM16 / HCT dynamic color engine, 49 M3 roles, shape/motion/typography
│   ├── yolo-contracts/         # WCAG contrast solver, 31 widget contracts, adaptive window classes
│   ├── yolo-emit/              # 11-target theme emitter (css, gtk, kde, cosmic, qml, winui, fluent, apple, omarchy, hyprland, dtcg)
│   ├── yolo-layout/            # Taffy 0.15 Flexbox/Grid layout engine with 4px/8px snapping & WCAG tap targets
│   ├── yolo-migrator/          # OXC TSX/JSX + Tailwind v4 + Shadcn/Radix/BaseUI -> 6 Rust targets compiler
│   ├── yolo-render/            # Analytical SDF primitives, damage tracking, Kawase blur, spring physics
│   ├── yolo-gpu/               # WGPU 30/29 instanced SDF GPU pipeline & glyph atlas
│   ├── yolo-shell/             # Desktop lifecycle, muda menus, tray-icon, global-hotkey, window-vibrancy
│   ├── yolo-craft/             # 13 Creative Workbenches (Writer, Screenplay, Storyboard, Comic, etc.)
│   ├── yolo-storyteller/       # 10 Storyteller narrative studio modules
│   └── yolo-react/             # Rust React 19 RSC Flight stream, RN Taffy layout, JSI/TurboModule codegen, OXC analyzer
├── apps/
│   └── yolo-showcase/          # Active multi-engine/theme showcase binary in the Cargo workspace
├── packages/
│   ├── yolo-ui/                # Umbrella fullstack crate re-exporting Dioxus 0.8 + all Yolo crates & toolkits
│   ├── yolo-gpui/              # Zed GPUI-Fast + Longbridge GPUI-Kit + Rust-i18n bridge
│   ├── yolo-iced/              # Iced Elm MVU + Iced_AW + Iced_Audio + Comet bridge
│   ├── yolo-cosmic/            # System76 libcosmic + Iced 0.14 + cosmic-config RON bridge
│   ├── yolo-egui/              # Egui 0.36 immediate-mode bridge + Puffin profiler hook
│   ├── yolo-gtk/               # GTK 4.16 + libadwaita 1.6 + Relm4 bridge
│   ├── yolo-qt/                # KDAB cxx-qt 0.8 + Qt6 Quick / Kirigami QML singleton bridge
│   ├── yolo-tauri/             # Tauri v3 Alpha + Wry 0.57 + Tao 0.37 + ACL bridge; browser engines belong to yolo-web
│   ├── yolo-shadcn-bun/        # @yolo-rs/shadcn-bun Full-Bun TSX/Tailwind -> Rust CLI & SDK
│   ├── yolo-react/             # @yolo-rs/react Full-Bun + Rust React 19 & Expo runtime package
│   ├── yolo-ui-plugin/         # @yolo-rs/ui-plugin Single Bun.plugin for .yolo.toml, .shadcn.tsx, .tailwind.css
│   └── yolo-ui-mcp/            # @yolo-rs/ui-mcp Single MCP server for UI search, theme emit, migration & devtools
├── skills/
│   └── yolo-ui-pro-max/        # UI/UX Pro Max skill (34 stacks, 13 domains, 7 presets, BM25 search)
├── scripts/
│   ├── ui.ts                   # Single unified UI command (`bun run ui <subcommand>`)
│   ├── react-bun.ts            # Full-Bun React 19 & Expo patcher, node-purger & bundler
│   ├── fleet.ts                # `yolo-rs/*` inventory & shallow sync generator
│   ├── build.ts                # Multi-engine Cargo check/release builder
│   ├── test.ts                 # Multi-engine Rust unit test runner
│   ├── optimize.ts             # Cargo dependency deduplication & profile auditor
│   └── migrate-bun.ts          # Node.js -> Bun v1.4.4+ (n2b) workspace auditor
└── submodules/                 # Pinned `yolo-rs/*` forks and upstream source inputs
```

## 3. Core toolchain commands (`bun ui` + `just` + `cargo` + `sccache`)

```bash
bun run ui doctor     # Verify the submodule fleet, 7 theme presets, 34 UI/UX Pro Max stacks, and toolchains
bun run ui search <q> # Query the 13-domain UI/UX Pro Max BM25 engine across 34 stacks
bun run ui theme      # Emit any of the 7 YoloThemePresets to 11 targets (css, gtk, cosmic, qml, fluent, apple, omarchy, hyprland, dtcg)
bun run ui migrate    # Compile Shadcn/Tailwind/Radix/BaseUI TSX into Dioxus, WGPU, Egui, Cosmic, GTK4, or QML
bun run react:status  # Inspect the 12 React/Expo/Rust submodules and Full-Bun React pipeline
just check            # Run bun check (tsgo) + cargo metadata + cargo check
just test             # Run bun test + Rust unit tests across all engines
```
