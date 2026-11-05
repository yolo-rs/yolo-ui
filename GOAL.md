# GOAL: one YOLO design system for every Aphrody product

Owner request (2026-11-05): one premium, consistent product across the Aphrody, YOLO, Shenron, RoseGriffon, Auto
and Craft sites and apps, built on `yolo-ui`. Zero copy, zero stub, zero placeholder, zero emoji, no generic fonts.

This file is the plan of record. Each phase lists its deliverable and the command that proves it. A phase is
done only when its proof runs green. "Planned" items stay planned until then.

## Ground rules

- Licensing gate before any asset enters the repo: record source URL, commit, license and attribution in
  `catalog/assets.lock.toml`. Allowed: Apache-2.0, MIT, BSD, ISC, OFL, CC0, CC-BY(-SA) with attribution,
  LGPL/GPL only for assets shipped as separate theme packages.
- Not redistributable, so never vendored, traced or "forked": Apple SF Symbols and SF fonts, Segoe UI Variable
  and Segoe Fluent Icons font files, Nintendo/Pokemon/Mii/Level-5 characters, Google product logos beyond their
  brand guidelines. Platform look-alikes are built from our own geometry plus the open sets below.
- Pets are original Aphrody designs only. Fan-IP pets (Pokemon, Mii, Inazuma Eleven) are out of scope.
- Rust and Bun only (no Node/npm). Layout through Taffy; color through `yolo-tokens` HCT/CAM16.
- Prod host rule: no writes to `/srv/shenron` or `shenron-*` containers from this work.

## Phase 1: icon and favicon pipeline (first, now)

Sources (all open):

| Set | Repo | License |
| --- | --- | --- |
| Material Symbols | google/material-design-icons | Apache-2.0 |
| Fluent UI System Icons | `submodules/fluentui-system-icons` | MIT |
| Chromium vector icons (`.icon`) | chromium/src `components/vector_icons`, `ui/views/vector_icons` | BSD-3 |
| Adwaita / GTK4 | GNOME/adwaita-icon-theme | LGPL-3 / CC-BY-SA-3 |
| COSMIC | `submodules/libcosmic/cosmic-icons` | CC-BY-SA-4 |
| WhiteSur (macOS-like) | `submodules/whitesur-icon-theme` | GPL-3 (separate package) |
| Phosphor | phosphor-icons/core | MIT |

Deliverables:
1. `crates/yolo-icons`: build-time importer (SVG and Chromium `.icon` to a normalized path IR), optimizer
   (usvg parse, path simplification, precision rounding, SVGO-equivalent passes in Rust), and `currentColor`
   tinting so every icon accepts any token color. Output: per-set compressed bundle (zstd) plus a typed Rust
   enum and a Bun TS module.
2. "Aphrody Symbols": Material Symbols variable axes (FILL, wght, GRAD, opsz) as the base grid, with Aphrody
   overrides drawn on that grid; merge order Aphrody > Material > Fluent > Chromium.
3. Favicon generator in `yolo-icons` (Rust: resvg render to 16/32/48/180/192/512 PNG, multi-size ICO,
   maskable PWA icon, `manifest.webmanifest`), driven by the active preset color.
4. `bun ui icons build|search|favicon` in `scripts/ui.ts` and the `ui_icons` MCP tool.

Proof: `cargo test -p yolo-icons`, `bun ui icons favicon --preset aphrody --out dist/favicon` produces valid
ICO/PNG/SVG (checked by decoding them back), size report shows the optimized bundle smaller than the source.

Phase 1 notes (2026-11-05): wired sets are Aphrody (original mark, 24 and 48 dp), Material Symbols outlined
(26 names incl. FILL=1 for star/favorite), Fluent regular 24, Chromium `.icon` (22, multi-representation files
included) and COSMIC symbolic, all pinned in `catalog/assets.lock.toml` with LICENSE and NOTICE next to each subset.
Not done yet: Material variable axes beyond FILL (wght, GRAD, opsz), Adwaita, WhiteSur and Phosphor sets, and
compiling the generated `aphrody_symbols.rs` into a crate (it is emitted and checked textually). `ui_icons` MCP tool
exists and shells out to the same binary.

## Phase 2: complete theme per platform

One preset = one `YoloThemeTokens` set emitted by `yolo-emit` to every target:

| Target | Emitter output | Reference system |
| --- | --- | --- |
| Chromium / Chrome | Chrome theme `manifest.json` (theme colors, images, tints) + `cef_color_id_t` map in yolo-chromium | Chrome color pipeline, Web Store themes |
| Windows 11 | WinUI 3 `ResourceDictionary` (Fluent 2 tokens, Mica/Acrylic) | microsoft/fluentui tokens |
| macOS / iOS | SwiftUI asset catalog colors + NSAppearance names | Apple HIG (structure only) |
| Android 17 | Compose `ColorScheme` + `dynamicColor` fallback, Material 3 Expressive | material-color-utilities |
| GTK4 / libadwaita | `@define-color` stylesheet | libadwaita named colors |
| COSMIC | `cosmic-theme` RON | libcosmic |
| Omarchy / Hyprland | Omarchy theme dir (alacritty, waybar, hyprland, btop...) | `submodules/omarchy` |
| Web | CSS custom properties + Tailwind v4 `@theme` | M3 tokens |

Theme registries (catalog, not copies): index of Omarchy community themes, M3 theme builder exports, Chrome Web
Store theme listings (metadata and link only), GTK/COSMIC theme repos, Android theming references, stored in
`catalog/themes.registry.toml` with source and license per entry.

Proof: `bun ui theme emit --preset <p> --target <t>` for 8 presets x 8 targets, each output validated by the
target's own parser where one exists (Chrome loads the theme in yolo-chromium smoke; `gtk4-builder-tool` /
`sassc` for GTK; RON parse for COSMIC; XAML parse for WinUI), plus `bun ui contrast` AA on all role pairs.

## Phase 3: components

Rust component library on Dioxus 0.8 (primary), with GPUI and Iced/libcosmic bridges, covering the M3 and Fluent
2 component inventories (buttons, fields, lists, menus, dialogs, sheets, navigation rail/bar, tabs, chips, cards,
data tables). Survey to decide reuse vs build: dioxus-components, gpui-component (Longbridge), libcosmic widgets,
iced_aw. Proof: storybook-style gallery app in `apps/yolo-showcase` rendering every component in every preset,
screenshot-diffed in CI.

## Phase 4: surfaces built on the system

1. Chrome new tab page, toolbar menus and MV3 extension (`yolo-web/packages/chrome-extension-yolo`, already
   loads in yolo-chromium) restyled from Phase 2 tokens.
2. Website (aphrody.com hub) and full-stack server (`@yolo-rs/fullstack`).
3. Desktop app (Tauri v3 + yolo-chromium/CEF), mobile app (Expo/Compose path already in repo).
4. Auth: Better Auth on Bun with providers Google, Microsoft, GitHub, Discord, Steam (OpenID 2.0), Epic Games
   (EOS OAuth); Neon or Supabase as the database; provider app verification is an owner action (requires the
   owner's accounts and domain verification).
5. Mail: Cloudflare Email Routing for inbound, Resend for outbound, OVH MX/SPF/DKIM/DMARC records. DNS changes
   are owner-approved actions.
6. Admin and automation under `src/auto`.

## Phase 5: media library and pets

- Wallpapers and backgrounds from open sources with license records (Omarchy theme backgrounds, Unsplash/
  Pexels license, CC0 3D textures and HDRIs from Poly Haven, ambientCG; models from Poly Haven/Kenney CC0).
- Shaders and materials: WGSL in `crates/yolo-gpu`, Bevy materials, respecting `prefers-reduced-motion`.
- Pets: merge the openpets fork into `yolo-ui` as `crates/yolo-pets` (Rust) + Bun generator; three original
  Aphrody starter pets.

## Phase 6: consolidation

Inventory `bun-os-archive` and the yolo-* repos, map every live module to its owner in `yolo-ui` or its pillar
repo, archive the rest (see `yolo-web/dist/folder-inventory.md`). No deletion without a verified remote copy.

## Status

| Phase | State | Evidence |
| --- | --- | --- |
| 1 icons and favicon | done for the listed deliverables (see notes below) | `cargo test -p yolo-icons` on `vps` through the yolo-vps factory: 21 passed (16 unit, 5 integration). `bun ui icons favicon --preset aphrody --out dist/favicon` wrote 11 files (31 KB) and `bun ui icons verify dist/favicon` decoded 8 PNG (16/32/48/180/192/512, maskable 192/512), 3 ICO entries (16/32/48), favicon.svg and the manifest; committed copy in `crates/yolo-icons/examples/out` is decoded again by the `committed_example_favicon_decodes` test and the `ico` crate. `bun ui icons build`: 97 sources, 85 030 B source SVG/.icon to 42 970 B optimized SVG to 16 255 B zstd across 5 sets; merged Aphrody Symbols bundle 31 icons, 3 409 B. Every optimized icon renders within 1.5% mean alpha error of its source at 48 px. `bun test tests/ui-icons.test.ts tests/ui-unified.test.ts`: 9 passed. |
| 2 platform themes | partial: 8 presets exist in yolo-shared/yolo-tokens; yolo-chromium applies 7 via `--theme` | `cargo test -p yolo-chromium` |
| 3 components | not started in this plan | - |
| 4 surfaces | partial: MV3 extension loads in yolo-chromium; managed policy `--search-engine=aphrody` | yolo-web branch `claude-chrome-extension-yolo` |
| 5 media and pets | not started | - |
| 6 consolidation | inventory done | `yolo-web/dist/folder-inventory.md` |
