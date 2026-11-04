# Yolo React 19 & Expo SDK 53/54 — 100% Full-Bun v1.4.4+ & Rust Architecture

> **Organization**: `yolo-rs/yolo-ui`
> **Rust Engine Crate**: `crates/yolo-react` (`yolo-react v0.8.0`)
> **Universal Bun Package**: `packages/yolo-react` (`@yolo-rs/react v0.8.0`)
> **CLI & Orchestrator**: `scripts/react-bun.ts` (`bun run scripts/react-bun.ts <command>`)
> **Default Theme Preset**: `YoloThemePreset::Neutral` (`"yolo-neutral"`, customizable across all 7 built-in presets)

---

## 1. Executive Summary

`yolo-ui` unifies **React 19** (Web, Server Components, Streaming SSR, Tauri v3) and **React Native / Expo SDK 53+ (Universal Web + iOS + Android + macOS + Windows)** on top of **100% Upstream Bun v1.4.4+** and **native Rust crates**.

Every legacy Node.js / npm / Webpack / Babel bottleneck is purged and replaced by deterministic Bun + Rust pipelines:

| Legacy Layer (Purged) | Yolo Full-Bun + Rust Replacement | Speedup / Architecture Benefit |
| :--- | :--- | :--- |
| `npm`, `yarn`, `pnpm`, `npx` | **`bun install`, `bun pm`, `bunx`** (`packageManager: "bun@1.4.4"`) | Zero `package-lock.json` / `yarn.lock`, hardlink global cache |
| `node`, `ts-node`, `tsx`, `cross-env` | **`bun <file.ts>`**, **`Bun.$`**, **`Bun.spawn`** | Native Zig/JSC TypeScript & TSX execution without transpiler wrappers |
| Webpack / Vite / Rollup | **`Bun.build`** + **`Bun.serve`** HTML imports | Single-pass native bundler, CSS lightning bundler, React Fast Refresh HMR |
| Babel (`metro-react-native-babel-preset`) | **`facetpack`** (`@ecrindigital/facetpack` Rust OXC Metro serializer/transformer) | Rust `oxc_parser` + `oxc_transformer` TypeScript/JSX stripping & Fast Refresh in Metro |
| React Native C++ Yoga layout | **`yolo-layout`** (`Taffy 0.15` + `crates/yolo-react/src/react_native.rs`) | Pure-Rust Flexbox + CSS Grid layout engine for server-driven UI, pre-measurement & WebGPU |
| Hand-written C++ JSI / Objective-C / Kotlin bridges | **`uniffi-bindgen-react-native`** + `yolo_react::react_native::emit_turborust_module` | Zero-boilerplate Rust -> C++ JSI TurboModules + TypeScript spec codegen |
| Node SSR & RSC streams | **`Bun.serve`** + **`yolo_react::rsc`** (Rust React 19 Flight Protocol) | Zero-copy React 19 Flight wire format (`0:`, `1:`, `$`) + streaming HTML shell |
| Fragmented `m3-*` React packages | **`@yolo-rs/react`** (`packages/yolo-react`) + **`yolo-tokens`** | Unified universal primitives (`Button`, `Card`, `Dialog`, `Input`, `Select`, `Switch`, `Tabs`, `Toast`, `FormField`, `SpringMotion`) across Web DOM + React Native |

---

## 2. Forked Upstream React, React Native, Expo & Rust-React Fleet (`yolo-rs/*`)

The `yolo-ui` monorepo integrates and patches the highest-starred upstream repositories from **`facebook/react` (`reactjs`)**, **`expo`**, and the **Rust + React / React Native** ecosystem:

### 2.1 React & React Native Core (`submodules/*`)
| Submodule Path | Upstream Origin | Role in `yolo-ui` |
| :--- | :--- | :--- |
| `submodules/react` | `facebook/react` (React 19.x + React Compiler) | Core React 19 fiber, Server Components (RSC), Actions, and React Compiler (`babel-plugin-react-compiler` replaced by OXC analyzer in `yolo-react`) |
| `submodules/react-native` | `facebook/react-native` (New Architecture / Fabric / TurboModules) | JSI C++ host interface, Fabric renderer, bridgeless mode |
| `submodules/react-strict-dom` | `facebook/react-strict-dom` | W3C-standard CSS + DOM primitives shared 1:1 between React Web and React Native |

### 2.2 Expo SDK 53/54 Universal Ecosystem (`submodules/*`)
| Submodule Path | Upstream Origin | Role in `yolo-ui` |
| :--- | :--- | :--- |
| `submodules/expo` | `expo/expo` | Universal Expo SDK 53/54 modules, Expo Router v4/v5, Continuous Native Generation (CNG), Expo DOM Components (`"use dom"`) |
| `submodules/expo-cli` | `expo/expo-cli` | Patched for `bun` / `bunx` execution |
| `submodules/eas-cli` | `expo/eas-cli` | Local & cloud builds via `bunx eas-cli` |
| `submodules/ibex` | `expo/ibex` | Expo's official **Rust** static analysis & JSON schema engine |
| `submodules/react-native-reanimated` | `software-mansion/react-native-reanimated` | 120fps UI-thread worklets paired with `@yolo-rs/react` spring physics |
| `submodules/react-native-gesture-handler` | `software-mansion/react-native-gesture-handler` | Native gesture recognition |
| `submodules/react-native-screens` | `software-mansion/react-native-screens` | Native navigation containers for Expo Router |
| `submodules/react-native-skia` | `Shopify/react-native-skia` | GPU 2D canvas sharing shaders/palettes with `anyrender_skia` |
| `submodules/flash-list` | `Shopify/flash-list` | High-velocity cell recycling list |

### 2.3 Rust-Powered React & React Native Accelerators
| Submodule / Crate | Upstream Origin | Role in `yolo-ui` |
| :--- | :--- | :--- |
| `crates/yolo-react` | `yolo-rs/yolo-ui` (first-party) | Rust RSC Flight engine, React Native `StyleSheet` -> Taffy 0.15 layout solver, OXC Hook/Component static analyzer, JSI TurboModule C++ emitter, 7-preset theme bridge |
| `crates/yolo-migrator` | `yolo-rs/yolo-ui` (first-party) | OXC-based TSX (`Shadcn`, `Radix`, `Base UI`, `M3`, `Tailwind v4`) to Dioxus 0.8, WGPU, Egui, Cosmic/Iced, GTK4, and Qt6 QML compiler |
| `submodules/oxc` | `oxc-project/oxc` | `oxc_parser`, `oxc_ast`, `oxc_transformer`, `oxc_minifier` powering both `yolo-react` and `facetpack` |
| `submodules/facetpack` | `ecrindigital/facetpack` | Drop-in Metro bundler transformer & serializer written in Rust (OXC), replacing Babel in Expo / React Native |
| `submodules/uniffi-bindgen-react-native` | `jhugman/uniffi-bindgen-react-native` | Compiles any Rust crate (`yolo-tokens`, `yolo-layout`, `yolo-react`) into a native C++ JSI TurboModule + TypeScript bindings |
| `submodules/rozenite` | `callstackincubator/rozenite` | Rust-accelerated React Native DevTools & performance inspector |
| `submodules/tuono` | `tuono-labs/tuono` | Superfast React 19 fullstack SSR/RSC router pattern integrated with `Bun.serve` + `yolo-react` |

---

## 3. Unification of Existing `bun-os` React Libraries into `@yolo-rs/react`

All battle-tested React & design system libraries from `C:\src\bun-os\packages\*` have been consolidated and upgraded into **`packages/yolo-react` (`@yolo-rs/react`)** and **`crates/yolo-react`**:

1. **`@aphrody/m3-theme` + `@aphrody/m3-tokens` -> `packages/yolo-react/src/core.ts` & `crates/yolo-react/src/theme_bridge.rs`**:
   - Preserves Rust-authoritative color generation (`yolo-tokens` HCT dynamic schemes, zero client-side JS color drift).
   - Expands beyond Material 3 to all **7 first-class `YoloThemePreset`s**, defaulting to **`neutral`** (`YoloThemePreset::Neutral` — `#6366F1` zinc/indigo):
     1. `neutral` (`"yolo-neutral"`) — Clean vendor-neutral default
     2. `m3` (`"yolo-m3"`) — Google Material Design 3 (`#6750A4` HCT tonal spot)
     3. `fluent` (`"yolo-fluent"`) — Microsoft Fluent UI 2 / WinUI 3 (`#0078D4` Mica/Acrylic)
     4. `apple` (`"yolo-apple"`) — Apple HIG Liquid Glass (`#007AFF` macOS Tahoe / iOS Cupertino)
     5. `omarchy` (`"yolo-omarchy"`) — Omacom Omarchy (`Tokyo Night` `#1a1b26`/`#7aa2f7` + all **23 official subthemes**: `catppuccin`, `catppuccin-latte`, `ethereal`, `everforest`, `flexoki-light`, `gruvbox`, `hackerman`, `osaka-jade`, `kanagawa`, `last-horizon`, `lumon`, `lupine`, `matte-black`, `miasma`, `nord`, `retro-82`, `ristretto`, `rose-pine`, `solitude`, `starship`, `vantablack`, `white`)
     6. `hyprland` (`"yolo-hyprland"`) — Hyprland Wayland Compositor (`#33ccff -> #00ff99` 45deg active border, kawase blur)
     7. `aphrody` (`"yolo-aphrody"`) — Signature Byron Love 10-teinte sprite atlas (`#17335C` Royal Azur, `#FFD700` gold, `#7C5CFF` violet)
   - Outputs both **Web CSS custom properties (`--yolo-*`, `--md-sys-color-*`)** and **React Native `StyleSheet` / `PlatformColor` objects**.

2. **`@aphrody/m3-primitives` + `@aphrody/m3-baseui` + `@aphrody/m3-react` -> `packages/yolo-react/src/primitives.ts`**:
   - Universal headless + styled component contracts (`Button`, `Card`, `Dialog`, `Input`, `Select`, `Switch`, `Tabs`, `Toast`) that emit **both**:
     - **React 19 Web DOM props**: `className`, `style`, WAI-ARIA (`role`, `aria-expanded`, `aria-selected`, `aria-checked`, `aria-busy`, `data-yolo-component`, `data-yolo-preset`).
     - **React Native / Expo Universal props**: `accessibilityRole`, `accessibilityState`, `accessibilityLabel`, and pre-computed Flexbox/Taffy style objects (`ViewStyle`, `TextStyle`).

3. **`@aphrody/m3-forms` -> `packages/yolo-react/src/primitives.ts` (`createFormFieldContract`, `validateYoloForm`)**:
   - Schema-driven form field contracts with automatic `aria-invalid`, `aria-describedby`, error message linking, and React Native `accessibilityHint` support.

4. **`@aphrody/m3-motion` -> `packages/yolo-react/src/primitives.ts` (`yoloSprings`, `computeSpringFrame`)**:
   - Physical spring presets (`fast`, `default`, `slow` × `spatial` with overshoot and `effects` critically damped) compatible with Web CSS `linear()` / WAAPI, `motion/react`, and `react-native-reanimated` `withSpring({ stiffness, damping, mass })`.

5. **`@aphrody/m3-next` -> `packages/yolo-react/src/server.ts` & `src/build.ts`**:
   - Replaces Next.js Node server dependencies with native **`Bun.serve`** streaming RSC Flight + SSR, HTML route imports, and **`Bun.build`** ahead-of-time asset bundling.

---

## 4. Phase-by-Phase Workflow & CLI Reference (`scripts/react-bun.ts`)

All React & Expo operations run through the 100% Bun v1.4.4+ CLI script `scripts/react-bun.ts` (aliased as `bun react:*` in `package.json`).

### 4.1 Check React / Expo / Rust Engine Status
```bash
bun run scripts/react-bun.ts status
```
Inspects `crates/yolo-react`, `packages/yolo-react`, OXC, Taffy, `yolo-tokens`, and submodule readiness, outputting a structured JSON report.

### 4.2 Scaffold a Zero-Node Full-Bun React 19 or Expo Universal App
```bash
# Create a universal React 19 + Expo + Rust app (defaults to --preset neutral)
bun run scripts/react-bun.ts create ./apps/my-yolo-app --template universal --preset neutral

# Create with Omacom Omarchy, Hyprland, Material 3, Apple Liquid Glass, Fluent 2, or Aphrody preset
bun run scripts/react-bun.ts create ./apps/hypr-dashboard --template web --preset hyprland
bun run scripts/react-bun.ts create ./apps/mobile-companion --template expo --preset aphrody
```
Generated apps contain:
- `package.json` locked to `"packageManager": "bun@1.4.4"` with zero `npm`/`npx`/`node` scripts.
- `bun.serve.ts` running `createYoloReactServer` (`Bun.serve` with RSC Flight endpoint `/_yolo/rsc`, dynamic Rust theme endpoint `/_yolo/theme.css`, and React Fast Refresh).
- `metro.config.ts` pre-patched with **`facetpack` (Rust OXC)** and **`uniffi-bindgen-react-native`** when `--template expo` or `universal` is selected.

### 4.3 Purge `npm`, `yarn`, `pnpm`, `npx`, and `node` (`purge-node`)
```bash
bun run scripts/react-bun.ts purge-node ./target-project
```
Automatically performs:
1. Deletes legacy lockfiles (`package-lock.json`, `yarn.lock`, `pnpm-lock.yaml`, `.npmrc`, `.yarnrc.yml`).
2. Rewrites every script in `package.json`:
   - `npm run <x>` / `yarn <x>` / `pnpm <x>` -> `bun run <x>`
   - `npx <x>` / `pnpx <x>` / `yarn dlx <x>` -> `bunx <x>`
   - `node <x>` / `ts-node <x>` / `tsx <x>` -> `bun <x>`
   - `expo start` / `react-native` -> `bunx --bun expo start`
3. Sets `"packageManager": "bun@1.4.4"` and `"engines": { "bun": ">=1.4.4" }`.

### 4.4 Patch Expo & React Native for Rust OXC (`facetpack`) + UniFFI TurboModules (`patch-expo`)
```bash
bun run scripts/react-bun.ts patch-expo ./apps/mobile-companion --preset omarchy
```
Generates or updates:
- **`metro.config.ts`**: Wires `@ecrindigital/facetpack` (`withFacetpack`) so Metro uses Rust `oxc` for JSX/TSX transformation, tree-shaking, and Fast Refresh instead of Babel.
- **`app.json`**: Injects Expo New Architecture (`"newArchEnabled": true`), `"experiments": { "reactCompiler": true, "typedRoutes": true }`, and the active `yoloThemePreset` metadata.
- **`yolo-rust-bridge.d.ts`**: Emits TypeScript TurboModule declarations matching `crates/yolo-react`'s `emit_turborust_module` C++ JSI output.

### 4.5 Build Production Bundles with `Bun.build` (`build`)
```bash
bun run scripts/react-bun.ts build ./src/entry.tsx --outdir ./dist --preset m3
```
Executes `buildYoloReactApp` in `packages/yolo-react/src/build.ts`, running `Bun.build` with dead-code elimination (`define: { "process.env.NODE_ENV": '"production"' }`), CSS bundling, and pre-rendered Rust theme CSS sheet output (`dist/yolo-theme.css` + `dist/yolo-theme-rn.json`).

---

## 5. Deep Dive: `crates/yolo-react` (Rust Engine for React & React Native)

`crates/yolo-react` provides 4 core Rust modules usable natively in Rust servers, Tauri v3 hosts, WASM modules, or React Native C++ JSI TurboModules:

1. **`yolo_react::rsc` (`src/rsc.rs`) — React 19 Server Components (RSC) Flight Protocol & Streaming SSR**:
   - Models `RscNode` (`Element`, `Text`, `ClientReference`, `Suspense`, `Fragment`), `RscChunk`, and `RscPayload`.
   - `encode_flight_stream(&payload)` serializes component trees into the React 19 Flight wire format (`0:["$","div",null,{...}]\n1:I["./ClientButton.tsx","Button"]\n`).
   - `render_rsc_to_html(&node, preset, dark)` renders complete hydrated HTML5 shells with inline `:root` `--yolo-*` CSS variables and streaming Flight `<script>` payloads.

2. **`yolo_react::react_native` (`src/react_native.rs`) — React Native `StyleSheet` -> Rust Taffy 0.15 Layout & JSI Codegen**:
   - Replaces C++ Yoga on the server/desktop/WASM with `yolo-layout` (`Taffy 0.15`).
   - `RnStyle` & `RnNode` accept standard React Native camelCase flexbox properties (`flexDirection: "column"`, `justifyContent`, `alignItems`, `paddingHorizontal`, `paddingVertical`, `gap`, `width`, `height`, `flex`) and compute exact `(x, y, width, height)` `LayoutRect`s via `compute_rn_layout`.
   - `emit_turborust_module(&spec)` generates both the **C++ JSI `TurboModule` header/implementation** and the **TypeScript `TurboModuleRegistry.getEnforcing` spec** for zero-copy calls into Rust (`yolo-tokens`, `yolo-layout`, `yolo-react`).

3. **`yolo_react::compiler` (`src/compiler.rs`) — OXC React 19 Component & Hook Analyzer**:
   - Parses `.tsx` / `.jsx` files at native speed using `oxc_parser` (`0.155`).
   - Extracts component names, props, React hooks (`useState`, `useReducer`, `useEffect`, `useLayoutEffect`, `useMemo`, `useCallback`, `useRef`, `useTransition`, `useOptimistic`, `useActionState`, `use`), `"use client"` / `"use server"` / `"use dom"` directives, and computes React Compiler auto-memoization boundaries.

4. **`yolo_react::theme_bridge` (`src/theme_bridge.rs`) — 7-Preset Universal Web CSS + React Native `StyleSheet` Bridge**:
   - Exports all 7 `YoloThemePreset`s (`Neutral`, `M3`, `Fluent`, `Apple`, `Omarchy` + 23 subthemes, `Hyprland`, `Aphrody`) as:
     - `:root` CSS variable sheets (`export_react_web_css`) combining `yolo-shared` structural tokens and `yolo-tokens` HCT dynamic color roles.
     - Typed React Native `StyleSheet` JSON payloads (`export_react_native_theme_json`) with colors, radii, spacing grid, typography, vibrancy, Hyprland active border gradients, and Byron Love 10-teinte palette metadata.
