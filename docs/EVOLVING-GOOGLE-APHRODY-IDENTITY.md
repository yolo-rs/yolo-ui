# Evolving Google Identity & Aphrody Design System (2025-2026)

Reference: [Evolving the Google Identity (`https://design.google/library/evolving-google-identity`)](https://design.google/library/evolving-google-identity)

This document defines the bridge between **Google's Evolving Identity (2025-2026)** and **Aphrody's own signature identity** inside `yolo-ui` (`crates/yolo-tokens`, `crates/yolo-shared`) and `yolo-hub` (`aphrody.com` / `yolo.aphrody.com`).

## 1. Strategic Architecture: `google.com` UX Parity with Aphrody Identity

Aphrody's primary product surface (`aphrody.com` / `yolo.aphrody.com`) provides full UI/UX and feature parity with `google.com`—universal search, vertical index tabs (`All`, `AI`, `Apps`, `WebGPU`, `Docs`, `Code`), a 9-dot App Waffle Launcher, and the complete productivity/OS suite—while maintaining Aphrody's own sovereign visual identity, assets, and customizable multi-preset engine (`yolo-tokens`).

## 2. Perceptual Multi-Stop Gradient Systems (CAM16 HCT & Oklch)

In the 2025-2026 Evolving Google Identity specification, Google transitioned from hard-segmented 4-color blocks (`#4285F4`, `#EA4335`, `#FBBC04`, `#34A853`) to fluid, luminous gradients that represent AI momentum and cross-application continuity without muddying at intermediate color transitions.

Naive sRGB linear interpolation between complementary hues (such as Blue -> Red or Violet -> Gold) collapses through low-chroma gray in the midpoint. `yolo-tokens::gemini_brand` solves this in two complementary ways:

1. **Compile-time & runtime CAM16 / HCT shortest-arc interpolation (`interpolate_hct`)**:
   - Converts stops into `Hct` (`hue` in `0..360`, `chroma`, `tone` L* in `0..=100`).
   - Interpolates hue along the shortest angular arc (`difference_degrees` + `rotation_direction`) while linearly interpolating perceptual chroma and CIE L* tone, solving back to displayable sRGB via `HctSolver`.
2. **Perceptual CSS serialization (`gradient_to_perceptual_css`)**:
   - Emits `linear-gradient(in oklch <angle>deg, ...)` with intermediate HCT-solved hex stops so both modern CSS Color 4 engines and legacy renderers preserve high chroma across every stop.

### 2.1 Google Evolving Identity Gradient (`EVOLVING_GOOGLE_IDENTITY_GRADIENT`)

Defined in `crates/yolo-tokens/src/gemini_brand.rs` and exported as `--google-evolving-identity-gradient`:

| Stop | Constant | Hex | Position | Role |
| --- | --- | --- | --- | --- |
| 1 | `EVOLVING_GOOGLE_BLUE` | `#4285F4` | `0%` | Core Google Blue anchor |
| 2 | `EVOLVING_GOOGLE_PURPLE` | `#9B72CB` | `25%` | Perceptual bridge preventing Blue-to-Red desaturation |
| 3 | `EVOLVING_GOOGLE_CORAL` | `#D96570` | `50%` | Luminous evolved Coral/Red |
| 4 | `EVOLVING_GOOGLE_YELLOW` | `#F4B400` | `75%` | Warm Amber/Yellow energy stop |
| 5 | `EVOLVING_GOOGLE_GREEN` | `#34A853` | `100%` | Grounded Google Green resolution |

### 2.2 Aphrody Signature 4-Color Gradient (`APHRODY_SIGNATURE_GRADIENT`)

Defined in `crates/yolo-tokens/src/gemini_brand.rs` and exported as `--aphrody-signature-gradient`:

| Stop | Constant | Hex | Position | Role |
| --- | --- | --- | --- | --- |
| 1 | `APHRODY_SIGNATURE_NAVY` | `#17335C` | `0%` | Byron Love Royal Azur (`APHRODY_SEED`) |
| 2 | `APHRODY_SIGNATURE_VIOLET` | `#7C5CFF` | `33%` | Electric Violet (`YOLO_APHRODY_DARK_THEME` primary accent) |
| 3 | `APHRODY_SIGNATURE_CYAN` | `#00D4FF` | `67%` | Celestial Cyan (`YOLO_APHRODY_DARK_THEME` secondary accent) |
| 4 | `APHRODY_SIGNATURE_GOLD` | `#FFD700` | `100%` | Divine Gold (`BYRON_LOVE_PALETTE` celestial highlight) |

## 3. Aphrody App Waffle & Search Portal (`yolo-hub`)

`yolo-hub` consumes `yolo-tokens` at build time (`build.rs`) to emit all 8 `YoloThemePreset` schemes (`chromium`, `aphrody`, `m3`, `fluent`, `apple`, `omarchy`, `hyprland`, `neutral`) in light, dark, and system modes, verified by automated WCAG 2.2 AA contrast tests (`src/contrast.rs`).

### 3.1 Interactive Search & AI Command Bar
- **Keyboard shortcut**: Pressing `/` focuses the command bar immediately from anywhere on the page.
- **Vertical category tabs**: `All` (`all`), `AI` (`ai`), `Apps` (`apps`), `WebGPU` (`webgpu`), `Docs` (`docs`), and `Code` (`code`).
- **Zero-JS resilience + instant client filtering**: Submitting the form works without JavaScript via `?q=&cat=`, while the inline CSP-hashed `CLIENT_SCRIPT` filters `data-filter-item` cards in real time by both text query and `data-category` vertical.

### 3.2 9-Dot Aphrody App Waffle Launcher
The header includes a `<details class="waffle-menu">` 9-dot launcher exposing the 13 flagship Aphrody / Google-equivalent applications:
1. **Aphrody Search** (`search`) — Universal web, code, and instant answer index.
2. **Aphrody AI** (`ai`) — Multimodal reasoning, coding, and agentic research assistant.
3. **Spacedrive** (`drive`) — Virtual distributed filesystem and cloud volume indexer.
4. **WordCraft Docs** (`wordcraft`) — Collaborative rich-text and technical document studio.
5. **GridCraft Sheets** (`gridcraft`) — High-frequency reactive spreadsheets and WASM formulas.
6. **DeckCraft Slides** (`deckcraft`) — GPU-accelerated presentation and storytelling deck builder.
7. **Aphrody Mail** (`mail`) — Fast, zero-tracking encrypted mail and thread triage.
8. **Aphrody Meet** (`meet`) — Low-latency WebRTC and WebTransport rooms with live AI captions.
9. **Aphrody Translate** (`translate`) — Neural document, voice, and code translation across locales.
10. **Aphrody Maps** (`maps`) — Vector tile globe, spatial search, and offline routing engine.
11. **WebGPU Store** (`play`) — Instant-run WebGPU apps, shaders, and verified WASM packages.
12. **GoogleBook WebOS** (`webos`) — ChromiumOS-inspired desktop shell and window manager in the browser.
13. **Locally Uncensored** (`studio`) — Local-first sovereign model workbench and GGUF/WebGPU runner.
