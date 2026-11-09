# YOLO-DESIGN — Unified Generative UI & Anti-AI-Slop Architecture

## 1. Pipeline Overview
`yolo-design` (integrated in `C:\src\yolo-ui`) unifies visual specification, mathematical verification, and multi-runtime code generation around a single source of truth: **[`DESIGN.md`](file:///C:/src/yolo-ui/DESIGN.md)**.

1. **Normative Specification**: [`DESIGN.md`](file:///C:/src/yolo-ui/DESIGN.md) (compliant with both `@google/design.md` v0.4.1 8-section grammar and `yolo-design-codegen` frontmatter).
2. **Mathematical Kernel (Zig 0.17 — [`zig/src/root.zig`](file:///C:/src/yolo-ui/zig/src/root.zig))**: OKLCH/sRGB conversions, APCA-W3 perceptual contrast ($L_c$), piecewise IEC 61966-2-1 WCAG 2.1 contrast, concentric radius geometry, closed-form rounded-rect SDF coverage (`vercel-labs/native`), damped harmonic oscillator springs (underdamped, critically damped, and overdamped), and 64-bit FNV-1a *Pinned Goldens*.
3. **Multi-Target Compiler (Rust 2024 — [`crates/yolo-design-codegen`](file:///C:/src/yolo-ui/crates/yolo-design-codegen/src/main.rs) & [`crates/yolo-emit`](file:///C:/src/yolo-ui/crates/yolo-emit/src/lib.rs))**: Compiles `DESIGN.md` into 6 production targets:
   - **Tailwind v4 CSS** (`packages/design-tokens/dist/theme.css`)
   - **W3C DTCG** (`packages/design-tokens/dist/tokens.json`)
   - **Rust GPUI** (`packages/design-tokens/dist/gpui_theme.rs` for Longbridge `gpui-component` v0.8)
   - **Rust Dioxus 0.8 / Blitz** (`packages/design-tokens/dist/dioxus_theme.rs`)
   - **COSMIC Epoch** (`packages/design-tokens/dist/cosmic_theme.ron`)
   - **Vercel Native SDK** (`packages/design-tokens/dist/app.native` & `native.theme.schema.json` using lowercase `<column>`, `<row>`, `<text>`, `<button>` tags matching `ui_schema.zig` `schema_version = 2`)
4. **Fullstack Generative UI & FFI Bridge (Bun — [`packages/generative-ui/src/index.ts`](file:///C:/src/yolo-ui/packages/generative-ui/src/index.ts))**:
   - Cross-platform zero-copy `bun:ffi` binding to `yolo_zig.dll` / `libyolo_zig.so` / `libyolo_zig.dylib`.
   - Sub-2ms local agent orchestration via `vercel-labs/fx` (`libfx` & `fx`).
   - Screen and variant generation via `@google/stitch-sdk` (`https://stitch.googleapis.com/mcp`) handling the `Generation<Screen>` Proxy (`getHtml()` -> signed URL -> `fetch().text()`).
   - Bidirectional *Write-to-Canvas* (`use_figma`) and *Code-to-Canvas* (`generate_figma_design`) via the official Figma MCP Server (`https://mcp.figma.com/mcp`) and `@figma/code-connect`.
   - WebGPU DOM shaders via `@figma/expressive-components` (`<ShaderFill>`, `<ShaderEffect>`).

## 2. Active Agent Skills Catalog (`skills/`)

| ID | Skill | Upstream Source | Application Domain |
| :--- | :--- | :--- | :--- |
| 01 | `emilkowalski-design` | `emilkowalski/skills` | Micro-interaction physics, springs, sub-180ms transitions |
| 02 | `impeccable-design` | `pbakaus/impeccable` | Modular scale, vertical rhythm, luminance stratification, anti-slop |
| 03 | `taste-skill` | `Leonxlnx/taste-skill` | Parametric art direction (`DESIGN_VARIANCE`, `MOTION_INTENSITY`, `VISUAL_DENSITY`) |
| 04 | `vercel-interface-guidelines` | `vercel-labs/agent-skills` | `tabular-nums`, `text-wrap: balance`, keyboard focus, zero CLS |
| 05 | `yolo-ui-pro-max` | `nextlevelbuilder/ui-ux-pro-max-skill` | BM25 3-tier token search engine, WCAG/APCA contrast, viewport audit |
| 06 | `anthropic-frontend-design` | `anthropics/skills` | Ban on generic layouts and AI purple gradients |
| 07 | `stitch-design` | `google-labs-code/stitch-skills` | `design-md`, `taste-design`, and `enhance-prompt` for Stitch MCP |
| 08 | `stitch-build` | `google-labs-code/stitch-skills` | `stitch-loop`, `react-components`, `react-native`, `remotion` |
| 09 | `stitch-utilities` | `google-labs-code/stitch-skills` | `extract-static-html` and `code-to-design` |
| 10 | `figma-mcp-canvas` | `figma/mcp-server-guide` | `use_figma` (`$fig`), `generate_figma_design`, `figma-spec` |
| 11 | `figma-figquery` | `figma/mcp-server-guide` | Structured scene-graph queries (`skills-figquery`) |
| 12 | `figma-motion` | `figma/mcp-server-guide` | `video-interaction-mapper` and `figma-implement-motion` |
| 13 | `vercel-native-ui` | `vercel-labs/native` | `.native` declarative markup (`schema_version = 2`) & MVU bindings |
| 14 | `vercel-native-ts-core` | `vercel-labs/native` | Static TypeScript subset compiled to native machine code (`scriptc`) |
| 15 | `vercel-native-ts-services` | `vercel-labs/native` | Isolated I/O services invoked via `Cmd.serviceRequest` |
| 16 | `vercel-native-zig-core` | `vercel-labs/native` | Zig 0.16/0.17 `--template zig-core` (`native_sdk.UiApp`) |
| 17 | `vercel-fx-boundaries` | `vercel-labs/fx` | Single-front-door module boundaries & TLA+ recovery invariants |
