# Yolo UI (`aphrody-labs/yolo-ui`) — Aphrody Labs Unified Fullstack Framework & Dioxus Monorepo Guide

Yolo UI (`aphrody-labs/yolo-ui`) is the unified fullstack Rust UI framework monorepo for all Aphrody Labs sites and applications. It unifies **Dioxus 0.8**, **Blitz**, **AnyRender**, **Taffy**, **Dioxus Components / Primitives**, **Dioxus SDK**, **Dioxus Icons**, **Dioxus Code**, **RSRPC**, **Accessibility CLI**, and **Aphrody Material 3 Dark-Glass Tokens & App Shell (`packages/yolo-ui`)** into **one Cargo workspace (174 members)** and **32 `aphrody-labs/*` Git submodules (`submodules/*`)**.

## Quick Overview

- **Language**: Rust (Edition 2024)
- **Umbrella Crate**: `yolo-ui` (`packages/yolo-ui`) & `dioxus` (`packages/dioxus`)
- **UI Model**: React-like VirtualDOM, components, hooks, signals, stores, and Radix/shadcn-style accessible primitives (`dioxus-primitives`)
- **Syntax**: JSX-like `rsx!` macro for declaring UI + `#[server]` / `#[get]` / `#[post]` for fullstack Axum RPC
- **Platforms**: Web (WASM + SSR/Hydration), Desktop (Wry/Tao WebView), Native (Blitz + AnyRender Vello/Skia/WGPU + Taffy GPU Engine), Mobile (Android/iOS), LiveView (WebSocket streaming)

## Monorepo & Submodule Fleet Structure

```
yolo-ui/
├── packages/
│   ├── yolo-ui/          # Aphrody Labs umbrella fullstack crate (Theme, SEO, AppShell, Dioxus + Ecosystem re-exports)
│   ├── dioxus/           # Core Dioxus re-export crate
│   ├── core/             # VirtualDOM, components, diffing, scheduling
│   ├── rsx/              # RSX macro parsing and code generation
│   ├── rsx-hotreload/    # Template diffing for hot-reload
│   ├── signals/          # Reactive state (Signal, Memo, Store)
│   ├── hooks/            # Built-in hooks (use_signal, use_effect, etc.)
│   ├── router/           # Type-safe routing with #[derive(Routable)]
│   ├── fullstack/        # SSR, hydration, #[server] functions (Axum)
│   ├── dx/               # `dx` build tool, dev server, bundling
│   ├── web/              # WASM renderer
│   ├── desktop/          # Wry/Tao webview renderer
│   ├── native/           # Blitz/Vello GPU renderer
│   ├── liveview/         # WebSocket streaming renderer
│   ├── manganis/         # asset!() macro for compile-time assets
│   ├── subsecond/        # Hot-patching system (jump table indirection)
│   ├── devtools/         # Dev server communication protocol
│   ├── interpreter/      # Sledgehammer JS for DOM mutations
│   └── wasm-split/       # WASM code splitting
└── submodules/           # Forked DioxusLabs repositories under github.com/aphrody-labs/*
    ├── blitz/            # Modular HTML/CSS native rendering engine (blitz-dom, blitz-paint, blitz-html, stylo_taffy, ...)
    ├── anyrender/        # 2D GPU/CPU rendering abstraction (Vello, Vello CPU, Vello Hybrid, Skia, SVG, WGPU)
    ├── dioxus-taffy/     # High-performance CSS Flexbox & Grid layout library (taffy 0.15)
    ├── taffy/            # Aphrody Obscura taffy fork
    ├── dioxus-components/# Accessible unstyled primitives (dioxus-primitives, dioxus-attributes)
    ├── sdk/              # Platform utilities (storage, time, geolocation, notification, sync, window, util)
    ├── dioxus-icons/     # Lucide SVG icon components (dioxus-icons, dioxus-icons-codegen)
    ├── dioxus-code/      # Syntax-highlighted code blocks & code editor (dioxus-code, dioxus-code-editor)
    ├── dioxus-html-macro/# HTML macro for Dioxus applications
    ├── markdown/         # Markdown-to-VNode renderer (dioxus_markdown)
    ├── rsrpc/            # Trait-based Rust-to-Rust RPC (rsrpc, rsrpc-macro)
    ├── dioxus-test/      # Native-renderer testing library for Dioxus
    ├── accessibility-cli/# Cross-platform computer-use & accessibility CLI/server (Windows, Linux, macOS, Android, iOS)
    ├── warnings/         # Runtime warning macros
    ├── prettier-please/  # Rust AST pretty-printer for autofmt/tests
    ├── web-adapters/     # Fullstack & LiveView adapters
    ├── wasm-bindgen-wry/ # Wry native bindings for wasm-bindgen
    ├── bedina/           # Bevy <-> Dioxus Native integration
    ├── rustd/            # Rust compiler daemon
    ├── docsite/          # Fullstack documentation site, playground & search
    ├── dioxus-ai/        # Component generation & automated QA
    ├── dioxus-template/  # Starter templates for `dx`
    └── ...               # Rink, include_mdbook, dioxus-search, playground, parley, vello_svg, winkin, etc.
```

## Architecture Documentation

For deeper understanding, see `notes/architecture/`:

| When working on...                         | Read...            |
| ------------------------------------------ | ------------------ |
| VirtualDOM, components, diffing, events    | `01-CORE.md`       |
| CLI, build system, bundling, dev server    | `02-CLI.md`        |
| RSX macro, parsing, formatting             | `03-RSX.md`        |
| Signals, state management, reactivity      | `04-SIGNALS.md`    |
| Server functions, SSR, hydration           | `05-FULLSTACK.md`  |
| Web/desktop/native/liveview renderers      | `06-RENDERERS.md`  |
| Hot-reload, hot-patching, devtools         | `07-HOTRELOAD.md`  |
| Asset macro, manganis, const serialization | `08-ASSETS.md`     |
| Router, navigation, nested routes          | `09-ROUTER.md`     |
| WASM code splitting                        | `10-WASM-SPLIT.md` |

## Key Concepts

- **VirtualDOM**: Tree of `VNode` with templates, dynamic nodes, and attributes
- **Signals**: Copy-able reactive primitives via generational-box (generation-based validity)
- **WriteMutations**: Trait that renderers implement to apply DOM changes
- **RSX**: Proc macro that compiles JSX-like syntax to `VNode` construction
- **Server Functions**: `#[server]` macro generates client RPC stubs and server handlers
- **Subsecond**: Hot-patches Rust code via jump table indirection (no memory modification)
- **Manganis**: `asset!("/main.css")` macro for including assets by embedding data via linker symbols

## Common Patterns

**Component definition**:
```rust
#[component]
fn MyComponent(name: String) -> Element {
    let mut count = use_signal(|| 0);
    rsx! {
        button { onclick: move |_| count += 1, "{name}: {count}" }
    }
}
```

## Notes for Agents

1. The `dioxus` crate re-exports from other crates - most implementation is in `packages/vdom`, `packages/signals`, etc.
2. RSX macro expansion happens in `packages/rsx` - look there for syntax questions
3. Each renderer implements `WriteMutations` differently - see `06-RENDERERS.md`
4. Hot-reload has two systems: RSX template diffing (fast) and Subsecond code patching (full Rust)
5. Assets use link sections and binary patching - the `asset!()` macro creates symbols the CLI processes
