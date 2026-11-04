# Yolo UI (`aphrody-labs/yolo-ui`) — Aphrody Labs Unified Multi-Engine Fullstack Framework & Monorepo Guide

Yolo UI (`aphrody-labs/yolo-ui`) is the unified fullstack Rust + Bun v1.4.4+ UI framework monorepo for all Aphrody Labs sites and applications. It unifies **5 major Rust UI ecosystems** into **one flattened Cargo workspace (186 members, 349 deduplicated dependencies)**, **one Bun v1.4.4+ workspace**, and **70 `aphrody-labs/*` Git submodules (`submodules/*`)**:

1. **DioxusLabs (32 submodules)**: **Dioxus 0.8**, **Blitz**, **AnyRender** (Vello/Skia/WGPU), **Taffy 0.15**, **Dioxus Components / Primitives**, **Dioxus SDK**, **Dioxus Icons**, **Dioxus Code**, **RSRPC**, **Accessibility CLI**.
2. **Longbridge (9 submodules)**: **GPUI-Kit / GPUI-Component** (60+ GPU widgets, DockArea, VirtualList, Charts, Editors), **NSH** shell, **Rust-i18n**, **GPUI-Mobile**, **WEF**, **AutoCorrect**.
3. **Zed Industries (4 submodules)**: **GPUI-Fast** (WGPU/Metal/DirectX/Blade/WebGL2 GPU engine), **Awesome-GPUI**, **Create-GPUI-App**, **Zed-Fonts**, **Font-Kit**, **Scap**, **Async-Dispatcher**.
4. **Iced-rs (10 submodules)**: **Iced** (Elm MVU architecture), **Iced_AW**, **Iced_Audio** (audio DSP knobs/faders/FreqRange), **Cryoglyph** (WGPU text atlas), **Comet** (time-travel MVU debugger), **Iced-OpenGL**, **mdBook-Iced**.
5. **Tauri-Apps v3 Alpha (15 submodules)**: **Tauri 3.0.0-alpha.5** (`v3`/`dev`), **Wry 0.57.1**, **Tao 0.37.2**, **CEF (`tauri-runtime-cef`)**, **Muda 0.21.2**, **Tray-Icon 0.26.1**, **Window-Vibrancy 0.8.1** (Mica/Acrylic/Liquid Glass), **Global-Hotkey 0.9.0**, **Cargo-Mobile2**, **Plugins-Workspace v3**.

## Flattened Monorepo Structure

```text
yolo-ui/
├── Cargo.toml                  # Unified Cargo workspace (186 members, 349 deduplicated workspace deps)
├── Justfile                    # Unified task runner (just + bun + cargo + sccache + n2b + gh)
├── package.json                # Root Bun v1.4.4+ workspace (@aphrody/yolo-ui)
├── tsconfig.json               # TypeScript 7 (tsgo / bun check) config
├── bunfig.toml                 # Bun test & workspace config
├── .cargo/config.toml          # sccache compiler wrapper, sparse crates-io index, Cargo aliases
├── crates/
│   └── yolo-shared/            # Zero-duplication shared foundation (RgbaColor, AphrodyThemeTokens)
├── apps/
│   └── yolo-showcase/          # Unified 4-engine showcase binary (Dioxus + GPUI + Iced + Tauri v3)
├── packages/
│   ├── yolo-ui/                # Aphrody Labs umbrella fullstack crate (re-exports Dioxus, GPUI, Iced, Tauri)
│   ├── yolo-gpui/              # Zed GPUI + Longbridge GPUI-Kit + Rust-i18n bridge
│   ├── yolo-iced/              # Iced Elm MVU + Iced_AW + Iced_Audio + Comet bridge
│   ├── yolo-tauri/             # Tauri v3 Alpha + Wry 0.57 + Tao 0.37 + CEF + ACL bridge
│   ├── dioxus/                 # Core Dioxus re-export crate
│   ├── core/, signals/, rsx/, router/, fullstack/, web/, desktop/, native/, liveview/, dx/ ...
├── scripts/
│   ├── fleet.ts                # 70-submodule status, shallow sync & fleet.json inventory generator
│   ├── build.ts                # Fast metadata & multi-engine Cargo check/release builder
│   ├── test.ts                 # Multi-engine Rust unit test runner
│   ├── optimize.ts             # Cargo dependency deduplication & profile auditor
│   └── migrate-bun.ts          # Node.js -> Bun v1.4.4+ (n2b) workspace auditor
├── tests/
│   └── monorepo.test.ts        # Bun v1.4.4+ automated monorepo integrity test suite
├── docs/
│   ├── README.md               # Documentation index
│   ├── MONOREPO-BEST-PRACTICES.md
│   └── BUN-N2B-MIGRATION.md
└── submodules/                 # 70 shallow-cloned aphrody-labs/* forks across 5 ecosystems
```

## Architecture Documentation (`notes/architecture/`)

| When working on...                                 | Read...                   |
| -------------------------------------------------- | ------------------------- |
| Full 70-submodule fleet & 4-engine overview        | `00-MONOREPO-FLEET.md`    |
| VirtualDOM, components, diffing, events            | `01-CORE.md`              |
| CLI (`dx`), build system, bundling, dev server     | `02-CLI.md`               |
| RSX macro, parsing, formatting                     | `03-RSX.md`               |
| Signals, state management, reactivity              | `04-SIGNALS.md`           |
| Server functions, SSR, hydration                   | `05-FULLSTACK.md`         |
| Web/desktop/native/liveview renderers              | `06-RENDERERS.md`         |
| Hot-reload, hot-patching (`subsecond`), devtools   | `07-HOTRELOAD.md`         |
| Asset macro, manganis, const serialization         | `08-ASSETS.md`            |
| Router, navigation, nested routes                  | `09-ROUTER.md`            |
| WASM code splitting                                | `10-WASM-SPLIT.md`        |
| Longbridge GPUI-Kit, Zed GPUI-Fast, NSH, Rust-i18n | `11-GPUI-LONGBRIDGE.md`   |
| Iced Elm MVU, Iced_AW, Iced_Audio, Cryoglyph, Comet| `12-ICED.md`              |
| Tauri v3 Alpha, Wry 0.57, Tao 0.37, CEF, ACL       | `13-TAURI-V3.md`          |

## Core Toolchain Commands (`just` + `bun` + `cargo` + `sccache`)

```bash
just doctor           # Verify bun, cargo, rustc, sccache, just, gh
just check            # Run bun check (tsgo) + cargo metadata + cargo check
just test             # Run bun test + Rust unit tests across all 4 engines
just showcase         # Run apps/yolo-showcase unifying Dioxus + GPUI + Iced + Tauri v3
just fleet-inventory  # Generate fleet.json for all 70 submodules
just optimize         # Audit Cargo workspace dependencies & profiles
just n2b-migrate      # Verify zero Node/npm lockfiles and 100% Bun v1.4.4+ scripts
just sccache-stats    # Inspect sccache compilation cache hit statistics
```
