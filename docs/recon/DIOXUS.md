# Dioxus ecosystem recon for Yolo UI

Checked 2026-11-05. Star counts, licenses and push dates come from `gh api repos/<o>/<r>`; versions come from `https://crates.io/api/v1/crates/<name>`. Repo paths are relative to `C:\src\yolo-ui`. Lines marked **[unverified]** are inference.

## 0. Baseline

- Upstream `DioxusLabs/dioxus`: 39,607 stars, Apache-2.0 on GitHub (the crates are MIT OR Apache-2.0), last push 2026-11-03. Releases: v0.8.0-alpha.3 (2026-10-16), alpha.2 (2026-10-14), alpha.1 (2026-07-31), stable v0.7.10 (2026-07-30).
- Ours: `packages/*` vendors the 0.8.0-alpha.3 tree (`Cargo.toml` line 345). `packages/yolo-ui/Cargo.toml` re-exports every Dioxus crate behind features (`router`, `fullstack`, `server`, `web`, `desktop`, `native`, ...).
- Evidence of actual use: grepping `crates/`, `apps/` and `packages/yolo-*` for `use_signal|use_store|#[server|asset!(|Routable|wasm_split` finds one match, a doc comment in `apps/yolo-showcase/src/site_hub.rs`. `apps/yolo-showcase` only uses `dioxus` (`lib`) and `dioxus-ssr`. The framework is wired up, but no app code uses it yet.

## 1. Docs in depth: what to use, where we diverge

Sources: https://dioxuslabs.com/learn/0.7/, https://dioxuslabs.com/blog/release-070/, https://dioxuslabs.com/learn/0.7/migration/to_07, and the GitHub release notes for v0.8.0-alpha.1, alpha.2 and alpha.3.

| Area | Upstream (0.7 docs, plus 0.8 deltas) | Yolo UI today | Action |
|---|---|---|---|
| Routing | `#[derive(Routable)]`, `#[route]`, `Router::<R>{}`, typed `Link` (https://dioxuslabs.com/learn/0.7/essentials/router/). 0.8.0-alpha.1 fixes child routes dropping the query and hash. | Feature `router` exists. No `Routable` enum anywhere. | Give the showcase gallery a `Route` enum (`apps/yolo-showcase/src/gallery.rs`). |
| Fullstack | Server functions run on Axum 0.8: `#[get("/api/{x}?q")]`, `#[post]`, `#[server]`, server extractors, `Websocket`/`use_websocket`, `Streaming<T>`, `ServerEvents<T>`, `FileStream`, `OrHttpError` (https://dioxuslabs.com/learn/0.7/essentials/fullstack/server_functions). The default codec is now JSON. In alpha.3, `dioxus::launch` installs ring as the rustls provider. | `packages/yolo-fullstack` is a Bun/Next.js replacement. Its README says `dioxusWebRoutes()` serves `dx build` web output from `Bun.serve`. No Rust `#[server]` code exists. | Decide on the boundary. One option is Rust server functions for Dioxus sites, with Bun serving only static output. Document the split in `packages/yolo-fullstack/README.md`. |
| Assets / Manganis | `asset!()` is const and writes metadata to a linker section. CSS and SCSS are minified. `AssetOptions::image().with_size/with_format(Avif)`, folders, `with_hash_suffix(false)`, `read_asset_bytes` on every target (https://dioxuslabs.com/learn/0.7/essentials/ui/assets). 0.8.0-alpha.1 adds build-time `OUT_DIR` assets and removes the 0.7 asset-format fallback. | No `asset!` calls. Themes are emitted as strings (`css_preset_variables`, `crates/yolo-shared/src/lib.rs:1755`). | Have `yolo-emit` write CSS into `OUT_DIR` from `build.rs` and include it with `asset!`. This is the 0.8 generated-assets path and fits the zero-handwritten-CSS policy. |
| Signals and Stores | Signals notify every reader. `#[derive(Store)]` plus `use_store` give per-field lenses and per-entry reactivity for `HashMap`/`BTreeMap`/`Vec` (https://dioxuslabs.com/learn/0.7/essentials/basics/collections). | `dioxus-stores` is re-exported but unused. `crates/yolo-craft/src/document.rs` holds large document models (43 matches for the grep pattern, mostly non-Dioxus). | Use `Store` for craft and storyteller documents (large collections) and signals for theme and preset state. |
| Renderers | Web, Desktop (wry 0.57 / tao 0.37 in alpha.3), Liveview, SSR, and Native (Blitz 0.3.0-beta.4; mutations buffered after render since alpha.3). Blitz is in beta: Stylo, Taffy and Parley, AccessKit, deliberately no WebSockets or localStorage (https://github.com/DioxusLabs/blitz). | Blitz and AnyRender are workspace members. Desktop dependency pins already match alpha.3: wry 0.57.1, tao 0.37.2, muda 0.21.2, keyboard-types 0.8 (`Cargo.toml` lines 794-841). | Add a `--native` gallery target. Use `blitz-test-harness` (already a member) for render tests. |
| Hot reload | RSX hot reload, asset hot reload (Tailwind watcher when `tailwind.css` is at the project root), and `dx serve --hotpatch` (Subsecond, experimental). Only the tip crate is patched; workspace dependencies are not (https://dioxuslabs.com/learn/0.7/essentials/ui/hotreload). | `subsecond` is a hard dependency of `yolo-ui`. No `subsecond::call` anywhere. The Justfile has no `dx` recipe. | Add a `just dev` recipe that runs `dx serve --hotpatch -p yolo-showcase`. Keep the showcase as the tip crate. |
| WASM splitting | `#[wasm_split("/route")]` on router variants (0.7 release post). The 0.7 learn index mentions bundle splitting only in passing. | `packages/wasm-split-*` are vendored. No uses. | Lazy-load the heavy gallery routes (3D, code editor). |
| 0.8 breaking changes | HTML elements are no longer under `dioxus_elements`, `rsx!` expands to a const builder, `define_elements!` (alpha.2). `Key::Named(NamedKey::Enter)` (alpha.3). The `linux-libxdo` feature is gone (alpha.3). | `yolo-contracts` has its own `Key` enum (`crates/yolo-contracts/src/keyboard.rs`), so it compiles, but nothing maps it from Dioxus `Key`. The migrator emits `class:` and `onclick:` (`crates/yolo-migrator/src/dioxus.rs`). It does not use `dioxus_elements`, so alpha.2 does not break it. | Add `From<dioxus::prelude::Key>` for `yolo_contracts::Key` (handling `NamedKey`) so the 31 contracts drive real events. |

## 2. Templates, examples, awesome list

- **Templates.** `DioxusLabs/templates` returns 404. The real repo is `DioxusLabs/dioxus-template` (74 stars, no license file, push 2026-10-16), with Desktop, Fullstack, Liveview and Web sub-templates for `dx new`. Vendored at `submodules/dioxus-template`.
- **Examples.** The `examples/` folder in the main repo is grouped as 01-app-demos through 10-integrations (`gh api repos/DioxusLabs/dioxus/contents/examples`). `DioxusLabs/example-projects` is archived (528 stars).
- **Showcase.** The https://dioxuslabs.com/awesome page we fetched lists only the Dioxus, CLI, Taffy, Blitz and SDK projects. **[unverified]** The full showcase is on a page we did not fetch.
- **Awesome list.** Source: `submodules/awesome-dioxus/awesome.json` (`DioxusLabs/awesome-dioxus`, 323 stars, no license, push 2026-08-04), checked against GitHub and crates.io.
  - **Components / UI kits:** first-party `dioxus-primitives` plus 45 styled components (`DioxusLabs/dioxus-components`, 354 stars, Apache-2.0, push 2026-10-24). Also daisy-rsx (204 stars, MIT), Lumen Blocks (114 stars, MIT), rust-ui/dioxus-ui (77 stars, MIT), Freyr.
  - **Icons:** `dioxus-icons` (first-party, MIT AND ISC), `lucide-dioxus` 3.63.0 (RustForWeb/lucide, 86 stars, MIT), `dioxus-free-icons` 0.10.0 (195 stars, MIT), `dioxus-material-icons` (38 stars, MIT).
  - **Charts:** `dioxus-charts` 0.4.0 (100 stars, MIT OR Apache-2.0).
  - **Forms / drag and drop:** primitives (form, combobox, date_picker, drag_and_drop_list) and `dioxus-sortable` 0.1.2, which is LGPL-3.0-or-later and last pushed in 2023, so avoid it.
  - **i18n:** `dioxus-i18n` 0.7.0-alpha.1 (69 stars, MIT).
  - **State / data:** `dioxus-query` 0.9.2 (140 stars, MIT), `dioxus-radio` 0.7.0 (75 stars, MIT), `dioxus-provider` 0.2.1.
  - **Animation:** `dioxus-motion` 0.4.0 (wheregmis, 203 stars, MIT on crates.io).
  - **Editors:** `dioxus-code` (first-party, MIT), Freya editor.
  - **Maps:** not in the awesome list. `dioxus-leaflet` 0.5.0 exists on crates.io (MIT OR Apache-2.0, updated 2026-11-03). **[unverified]** We did not check its repo or quality.
  - **Alternative renderer:** Freya (marc2332/freya, 3,302 stars, MIT, 0.5.0-rc.13), built on Skia. It competes with Blitz rather than complementing it.

## 3. Reuse table

Versions show what our workspace pins first, then the latest on crates.io. Wasm support is from crate scope and is **[unverified]** unless noted.

| Crate | Ours / latest | License | Wasm | Take | Consumer | Submodule |
|---|---|---|---|---|---|---|
| dioxus (+router, fullstack, stores) | 0.8.0-alpha.3 / same | MIT OR Apache-2.0 | yes | Everything in section 1 | `packages/yolo-ui`, showcase | vendored in `packages/` |
| dioxus-primitives | 0.0.1 path / crates.io 0.0.0 (stale) | MIT OR Apache-2.0 | yes | 45 headless ARIA components | `yolo-contracts` adapters, migrator target | yes (`dioxus-components`) |
| dx-components-theme.css | upstream 0.8 branch | Apache-2.0 | yes | Variable schema `--primary-color-1..7`, `--secondary-color-1..6`, status colors, `--focused-border-color` | `yolo-emit` CSS target | yes |
| dioxus-sdk (time, storage, geolocation, notification, sync, util, window) | 0.7.0 / 0.8.0-alpha.3 | MIT OR Apache-2.0 | partly | Storage, window size, timers | `yolo-layout` window classes, showcase | yes (`sdk`), stale |
| dioxus-icons | 0.1.0 / 0.2.0-alpha.3 | MIT AND ISC | yes | Icon codegen pipeline, fed with Phosphor | `yolo-ui` icons | yes, stale |
| dioxus-code / code-editor | 0.1.3 / 0.2.0-alpha.3 | MIT | yes | Highlighted code, editor | showcase migrator view, `yolo-craft` | yes, stale |
| blitz-* / dioxus-native | 0.3.0-beta.4 / same | MIT OR Apache-2.0 | no (native) | GPU HTML/CSS renderer | `yolo-ui` `native` feature | yes |
| anyrender (+vello, skia) | 0.14 / n/a | Apache-2.0 (repo) | partly | Paint backend shared with `yolo-render` | `yolo-render`, `yolo-gpu` | yes |
| taffy | 0.15.0 (path `submodules/dioxus-taffy`) / 0.15.1 | MIT | yes | Layout | `yolo-layout`, Blitz | yes, two copies (`taffy` and `dioxus-taffy`) |
| dioxus-motion | none / 0.4.0 | MIT | yes | Springs and transitions mapped to M3 motion tokens | `yolo-tokens` motion, `yolo-ui` | no |
| dioxus-i18n | none / 0.7.0-alpha.1 | MIT | yes | Fluent-based i18n for 0.8 | `yolo-ui`. Overlaps `rust-i18n` (GPUI bridge) | no |
| dioxus-query | none / 0.9.2 | MIT | yes | Async cache | `yolo-fullstack` Rust side | no |
| dioxus-charts | none / 0.4.0 | MIT OR Apache-2.0 | yes | SVG charts | showcase benchmarks | no |
| lucide-dioxus | none / 3.63.0 | MIT | yes | Reference only; Phosphor is the house icon set | none | no |
| dioxus-leaflet | none / 0.5.0 | MIT OR Apache-2.0 | yes (needs JS) | Maps **[unverified]** | none yet | no |
| dioxus-search, playground, include_mdbook, example-projects | vendored | various | n/a | Upstream archived, nothing to take | none | yes, remove candidates |

## 4. Next steps, ranked by value

1. **Bridge the theme tokens to Dioxus Components.** In `crates/yolo-emit/src/lib.rs`, add a `DxComponents` mode or target that maps HCT roles to the `dx-components-theme.css` variables, using the `--dark`/`--light` toggle pattern. Then load the result with `asset!` from `OUT_DIR` in `packages/yolo-ui`. This makes all 45 upstream components follow the 7 presets.
2. **Bring the stale vendored crates to alpha.3.** Bump `dioxus-sdk` (0.7.0), `dioxus-icons` (0.1.0) and `dioxus-code` (0.1.3) to their `0.8.0-alpha.3` / `0.2.0-alpha.3` releases in `Cargo.toml` lines 463-475 and the matching submodules. Confirm that `submodules/dioxus-components` tracks the upstream `0.8` branch. Upstream `main` still targets 0.7 (alpha.3 notes); the branch is **[unverified]**.
3. **Build a real Dioxus app in the showcase.** Add a `Routable` gallery in `apps/yolo-showcase/src/gallery.rs` with `use_signal` for the preset, `Store` for the component catalog, `#[wasm_split]` routes, and a `just dev` recipe running `dx serve --hotpatch`. Today the showcase only prints text and runs SSR.
4. **Connect contracts to events.** Map Dioxus `Key`/`NamedKey` to `yolo_contracts::Key` in `crates/yolo-contracts/src/keyboard.rs`. Then wrap the matching `dioxus-primitives` components so they emit contract events. Point the migrator output in `crates/yolo-migrator/src/dioxus.rs` at those wrappers instead of bare `div`/`button`.
5. **Clean up the submodules.** Unify `submodules/taffy` and `submodules/dioxus-taffy` (bump to 0.15.1). Drop or archive `dioxus-search`, `playground`, `include_mdbook` and `example-projects`, which are all archived upstream. Add `dioxus-motion`, `dioxus-i18n` and `dioxus-charts` as `yolo-rs` forks only once a consumer exists. Skip `dioxus-sortable` because of its LGPL license.
6. **Write down the fullstack boundary.** In `packages/yolo-fullstack/README.md` and `docs/WORKSPACE-CONSOLIDATION.md`, state where Rust `#[get]`/`#[server]` functions (Axum 0.8) take over from Bun routes. Note the alpha.3 rustls/ring provider default.
