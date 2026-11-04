<div align="center">
  <h1>⚡ Yolo UI (<code>aphrody-labs/yolo-ui</code>)</h1>
  <p>
    <strong>Aphrody Labs Unified Multi-Engine Fullstack Rust &amp; Bun v1.4.4+ Framework Monorepo</strong><br />
    <em>Dioxus 0.8 &bull; Blitz &amp; AnyRender &bull; Longbridge &amp; Zed GPUI &bull; Iced Elm MVU &bull; Tauri v3 Alpha</em>
  </p>
</div>

---

## Overview

**Yolo UI (`aphrody-labs/yolo-ui`)** unifies **82 forked repositories** (70 Git submodules in [`submodules/`](./submodules)) across **5 leading Rust UI ecosystems** into **one flattened Cargo workspace (186 workspace crates, 349 deduplicated dependencies)** and **one Bun v1.4.4+ toolchain**:

| Pillar | Upstream Org | Umbrella Crate | Key Capabilities |
| :--- | :--- | :--- | :--- |
| **1. Fullstack & Native DOM** | `DioxusLabs` | [`packages/yolo-ui`](./packages/yolo-ui) | Dioxus 0.8 (`rsx!`, `#[server]`, Signals), Blitz HTML/CSS Engine, AnyRender (Vello/Skia/WGPU), Taffy 0.15, Radix-style `dioxus-primitives`, `dioxus-sdk`, `dioxus-icons`, `dioxus-code`, `rsrpc`, `accessibility-cli` |
| **2. GPU Dock & Pro-Apps** | `longbridge` & `zed-industries` | [`packages/yolo-gpui`](./packages/yolo-gpui) | Zed `gpui-fast` (WGPU/Metal/DX12/Blade), Longbridge `gpui-kit` (60+ widgets, `DockArea`, `PanelDescriptor`), `rust-i18n`, `nsh`, `gpui-mobile`, `wef`, `autocorrect`, `font-kit`, `scap`, `async-dispatcher` |
| **3. Elm MVU & Audio DSP** | `iced-rs` | [`packages/yolo-iced`](./packages/yolo-iced) | Iced MVU (`IcedApplication`, `Task`, `MvuRuntime`), `iced_aw`, `iced_audio` (`NormalParam`, `FreqRange`, `KnobConfig`, `FaderConfig`), `cryoglyph` WGPU text atlas, `comet` time-travel debugger |
| **4. Multi-Engine Desktop/Mobile Shell** | `tauri-apps` (`v3` / `dev`) | [`packages/yolo-tauri`](./packages/yolo-tauri) | Tauri 3.0.0-alpha.5 (`WebviewEngine::Wry` & `WebviewEngine::Cef`), `wry` 0.57.1, `tao` 0.37.2, `muda` 0.21.2, `tray-icon` 0.26.1, `window-vibrancy` 0.8.1 (Mica/Acrylic/macOS 26 Liquid Glass), `global-hotkey` 0.9.0, v2->v3 Migration Auditor |
| **5. Shared Foundation** | `aphrody-labs` | [`crates/yolo-shared`](./crates/yolo-shared) | Zero-duplication Material 3 Dark-Glass tokens (`APHRODY_DARK_THEME`), `RgbaColor` (`#RRGGBB`, `#RRGGBBAA`, `rgba(...)`, `0xRRGGBBAA`), CSS custom property generator |

---

## Quick Start (`just` + `bun` + `cargo` + `sccache`)

```bash
# 1. Verify end-to-end toolchain (bun, cargo, rustc, sccache, just, gh)
just doctor

# 2. Run the unified 4-engine showcase app (crates/yolo-shared + yolo-ui + yolo-gpui + yolo-iced + yolo-tauri)
just showcase

# 3. Run all Bun v1.4.4+ & Rust unit tests (cached via sccache)
just test

# 4. Generate fleet.json inventory (70 submodules across 5 ecosystems) & run workspace optimizer
just fleet-inventory
just optimize
```

---

## Documentation

- **[Documentation Hub (`docs/README.md`)](./docs/README.md)**
- **[Flattened Monorepo Best Practices (`docs/MONOREPO-BEST-PRACTICES.md`)](./docs/MONOREPO-BEST-PRACTICES.md)**
- **[Node.js -> Bun v1.4.4+ (`n2b`) Migration Report (`docs/BUN-N2B-MIGRATION.md`)](./docs/BUN-N2B-MIGRATION.md)**
- **[Master Fleet Architecture (`notes/architecture/00-MONOREPO-FLEET.md`)](./notes/architecture/00-MONOREPO-FLEET.md)**
- **[Longbridge & GPUI Stack (`notes/architecture/11-GPUI-LONGBRIDGE.md`)](./notes/architecture/11-GPUI-LONGBRIDGE.md)**
- **[Iced Stack (`notes/architecture/12-ICED.md`)](./notes/architecture/12-ICED.md)**
- **[Tauri v3 Alpha Stack (`notes/architecture/13-TAURI-V3.md`)](./notes/architecture/13-TAURI-V3.md)**

## License

Dual-licensed under **MIT OR Apache-2.0**.
