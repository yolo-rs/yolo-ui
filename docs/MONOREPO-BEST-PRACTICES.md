# GitHub & Rust Flattened Monorepo Best Practices (`aphrody-labs/yolo-ui`)

This document codifies the architectural standards implemented across `aphrody-labs/yolo-ui`.

## 1. Flattened Directory Topology

```text
yolo-ui/
├── Cargo.toml                  # Single unified Cargo workspace (237 members; root dependency catalog)
├── Cargo.lock                  # Single Cargo lockfile for deterministic builds
├── Justfile                    # Unified task runner (just + bun + cargo + sccache + gh)
├── package.json                # Root Bun v1.4.4+ workspace manifest (`@aphrody/yolo-ui`)
├── tsconfig.json               # TypeScript 7 (tsgo / `bun check`) configuration
├── bunfig.toml                 # Bun test runner & package manager configuration
├── rustfmt.toml                # Rust 2024 Edition formatting rules
├── clippy.toml                 # Workspace lint thresholds and MSRV (1.85.0)
├── .cargo/
│   └── config.toml             # sccache compiler wrapper, sparse crates-io index, aliases
├── crates/
│   └── yolo-shared/            # Zero-duplication shared foundation (RgbaColor, AphrodyThemeTokens)
├── apps/
│   └── yolo-showcase/          # Unified 4-engine showcase binary (Dioxus + GPUI + Iced + Tauri v3)
├── packages/
│   ├── yolo-ui/                # Umbrella Fullstack Crate (Dioxus 0.8 + Blitz + AnyRender + Taffy)
│   ├── yolo-gpui/              # Zed GPUI + Longbridge GPUI-Kit + Rust-i18n Bridge
│   ├── yolo-iced/              # Iced Elm MVU + Iced_AW + Iced_Audio + Comet Bridge
│   ├── yolo-tauri/             # Tauri v3 Alpha + Wry 0.57 + Tao 0.37 + ACL bridge; browser engines belong to yolo-web
│   └── ...                     # Core Dioxus, Fullstack, Router, Signals, Web, Desktop, Native, CLI
├── scripts/                    # Native Bun v1.4.4+ orchestration (`fleet.ts`, `build.ts`, `test.ts`, `optimize.ts`, `migrate-bun.ts`)
├── tests/                      # Automated Bun monorepo integrity tests (`monorepo.test.ts`)
├── docs/                       # Monorepo best practices & migration guides
└── submodules/                 # 70 shallow-cloned `aphrody-labs/*` forks across 5 ecosystems
```

## 2. Downward Dependency Graph & Maximum Code Sharing

To eliminate circular dependencies and minimize compile steps, `yolo-ui` enforces a strict downward DAG:

1. **`crates/yolo-shared`** (`serde` only): Defines `RgbaColor`, `AphrodyThemeTokens`, `APHRODY_DARK_THEME`, `css_variables()`, and `EcosystemOrg`.
2. **`packages/yolo-gpui`**, **`packages/yolo-iced`**, **`packages/yolo-tauri`**: Depend on `yolo-shared` for all color parsing (`#RRGGBB`, `#RRGGBBAA`, `rgba(...)`), u32 packing (`0xRRGGBBAA`), linear mixing, and M3 Dark-Glass surface palettes.
3. **`packages/yolo-ui`**: Re-exports `yolo-shared`, Dioxus 0.8, Blitz/AnyRender/Taffy, and optionally `yolo-gpui` (`feature = "gpui"`), `yolo-iced` (`feature = "iced"`), and `yolo-tauri` (`feature = "tauri"`).
4. **`apps/yolo-showcase`**: Consumes `yolo-ui` with all 4 engines enabled.

## 3. Compiler Caching with `sccache` & `.cargo/config.toml`

All local and CI builds route `rustc` through `sccache` (`.cargo/config.toml`):
- `rustc-wrapper = "sccache"`
- `CARGO_INCREMENTAL = "0"` (required so `sccache` can hash and cache `.rlib` and proc-macro artifacts across workspace members)
- `registries.crates-io.protocol = "sparse"`
- `[profile.dev]` uses `debug = "line-tables-only"` to shrink target artifacts by ~60% while preserving stack trace line numbers.

## 4. End-to-End Task Execution with `just` + `bun` + `gh`

| Task | `just` Command | `bun` Equivalent |
| :--- | :--- | :--- |
| Check toolchains | `just doctor` | — |
| Format Rust + TS | `just fmt` | `bun run fmt` |
| Lint & Typecheck | `just lint` | `bun run lint && bun run check` |
| Run All Tests (Bun + Rust) | `just test` | `bun test && bun run test:rust` |
| Run 4-Engine Showcase | `just showcase` | `bun run showcase` |
| Generate 70-Submodule Inventory | `just fleet-inventory` | `bun run fleet:inventory` |
| Audit Workspace Optimization | `just optimize` | `bun run optimize` |
| Audit Node -> Bun (`n2b`) | `just n2b-migrate` | `bun run n2b:audit` |
| Inspect `sccache` Hit Rate | `just sccache-stats` | `sccache --show-stats` |
