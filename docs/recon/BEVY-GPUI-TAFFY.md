# Recon: Bevy, GPUI + GPUI Kit, Taffy (checked 2026-11-05)

Sources: `gh api repos/<o>/<r>` (stars, license, last push), the crates.io API (crate and dependency endpoints), and the files cited. UNVERIFIED means not checked against a primary source.

## 0. Local state (C:\src\yolo-ui)

- Workspace pins (`Cargo.toml`): `wgpu = 30.0.1` (l.589), `taffy` 0.15.0 from `submodules/dioxus-taffy` (l.481), `bevy*` 0.20.0 (l.593-598), `egui` 0.36.2 (l.591), `vello` 0.11 (l.623), and `gpui` 0.2.2 from the `spacedriveapp/zed` fork plus `gpui-component` at a git rev (l.846-849, `[patch]` l.1681-1683). The only consumer of that GPUI pin is `apps/spacedrive/apps/native`.
- `packages/yolo-gpui` has no GPUI dependency. Its features are empty and it only depends on `yolo-engines` with an empty `gpui = []` feature, so it is a data bridge and nothing more.
- `submodules/gpui-kit` is at 0.9.0 (on `gpui-pre` 0.3.11). `submodules/gpui-fast` contains `gpui_web` and `gpui_wgpu`. `submodules/gpui-mobile` carries LICENSE-AGPL/GPL/APACHE.
- `submodules/bevy`, `bevy-inspector-egui` and `awesome-bevy` are not checked out (each holds only `.git`).
- `submodules/taffy` (yolo-rs/taffy) is at 0.12.1 and stale. `submodules/dioxus-taffy` at 0.15.0 is the one the workspace actually uses.
- Docs that disagree with the code: the README says "wgpu 24" (the workspace pins 30). The `crates/yolo-3d` description says "Bevy 0.16" (the workspace pins 0.20). AGENTS.md says yolo-layout has "4px/8px snapping, WCAG tap targets", but a grep of `crates/yolo-layout/src` finds no snapping code. `11-GPUI-LONGBRIDGE.md` says "60+ widgets"; upstream says 75+.

## 1. Bevy

- Repo: https://github.com/bevyengine/bevy, 49,517 stars, Apache-2.0 (the crates are MIT OR Apache-2.0), last push 2026-11-05. Latest release v0.20.1 (2026-10-28). Release notes: https://bevyengine.org/news/bevy-0-20/ (posted 2026-10-08).
- New UI work in 0.20: more `bevy_feathers` widgets (color input, list view, dropdown, number scrubbing), headless tabs in `bevy_ui_widgets`, contextual theming, elliptical radii, BSN syntax, WESL shaders, and a separate `bevy_ui_render`.
- On `main` but not released (absent from the v0.20.1 tag): `crates/bevy_inspector` ("entity inspector built on bevy_ui and bevy_feathers") and `bevy_bsn`. Note that the crates.io `bevy_inspector` 0.0.1 is an unrelated squat. `bevyengine/bevy_editor_prototypes` was archived on 2026-04-14, so editor work now happens in the main repo.
- Rendering: `bevy_render` 0.20.1 depends on `wgpu ^30`. `bevy_ui` 0.20.1 uses `taffy ^0.14`, while `main` has moved to `taffy 0.15`.
- Web: the `webgl2`, `webgpu` and `web` cargo features exist in the root `Cargo.toml`.
- Ecosystem: https://bevyengine.org/assets/ is generated from https://github.com/bevyengine/bevy-assets (1,097 stars, pushed 2026-11-04; the old awesome-bevy name redirects there). The UI category has about 50 entries.

| Crate | Version | Targets | Notes |
|---|---|---|---|
| bevy_egui | 0.43.1 | bevy 0.20, egui ^0.36 | vladbat00, 1,440 stars, MIT, pushed 2026-10-25 |
| bevy-inspector-egui | 0.38.0 | bevy 0.20, egui ^0.36 | 1,652 stars, Apache-2.0, pushed 2026-10-17 |
| bevy_picking | 0.20.1 | upstream | in-tree, replaces bevy_mod_picking |
| bevy_tweening | 0.17.0 | bevy ^0.20 | 577 stars, Apache-2.0 |
| bevy_tween | 0.14.1 | bevy ^0.20 | alternative to bevy_tweening |
| bevy_flair | 0.9.0 | bevy 0.20 | CSS styling for bevy_ui |
| bevy_hui | 0.8.0 | bevy ^0.20 | HTML-like templates |

These lag behind or are stale: bevy_lunex 0.7.0 and bevy_vello 0.14.0 (both on bevy ^0.19), bevy_cobweb_ui 0.22.2 (bevy ^0.17), and sickle_ui 0.4.0 (2024). Editor tooling: bevy_yoleck 0.35.0 and skein 0.2.0.

## 2. GPUI and GPUI Kit

- Zed: https://github.com/zed-industries/zed, 92,745 stars. The license reports as NOASSERTION because the repo mixes GPL, AGPL and Apache; the `gpui` crate itself is Apache-2.0. Last push 2026-11-05, latest release v1.27.0.
- Crates outside Zed:
  - `gpui` 0.2.2 on crates.io, owned by Zed staff, was last published 2025-10-22 and pins `taffy =0.9.0`. It is stale.
  - `gpui-pre` 0.3.11 (2026-11-02), owned by huacnlee of Longbridge, is described as a "snapshot of zed@f816e1f". It uses `taffy ^0.15.0`, and `gpui-pre-wgpu` uses `wgpu ^30.0.1`.
  - `gpui-fast` 0.1.13 (https://github.com/longbridge/gpui-fast, 368 stars, Apache-2.0) is GPUI extracted from the monorepo, "with the layout engine reworked to keep what did not change between frames". It pins `taffy =0.15.0`.
  - Zed trunk pins `wgpu` to git rev `cddb2f5` and `taffy =0.15.0`.
- Web/wasm: yes. Zed trunk has `crates/gpui_web` (wasm32, wgpu with `webgpu` + `webgl`, multithreaded via `wasm_thread`), and gpui-fast vendors it. GPUI Kit advertises `wasm32-unknown-unknown` support and runs a live web gallery (https://gpui-kit.com/gallery/ returned HTTP 200). Its build is in `submodules/gpui-kit/crates/story-web`. Stability is UNVERIFIED and there is no Zed release note on it.
- Platforms: macOS, Windows, Linux (x11/wayland) on desktop. iOS/Android come from gpui-mobile (https://github.com/itsbalamurali/gpui-mobile, 470 stars, AGPL-3.0), which is unusable in an MIT/Apache crate.
- GPUI Kit: https://github.com/longbridge/gpui-kit (the gpui-component URL redirects here), 19,204 stars. Its crates are Apache-2.0, last push 2026-11-05, release v0.9.0 (2026-11-02). Layers: `gpui-kit` (re-exports a pinned GPUI), `gpui-base` (unstyled), `gpui-component` (styled), plus `gpui-shell` (JS runtime) and `gpui-webview`. It has AccessKit and a headless test harness.
- Components (`submodules/gpui-kit/crates/component/src`): accordion, alert, attachment, avatar, badge, breadcrumb, bubble, button, carousel, chart, plot, checkbox, clipboard, collapsible, color_picker, combobox, command, description_list, dialog, sheet, diff, dock, empty, form, grid_layout, group_box, highlighter, hover_card, icon, inspector, input, kbd, label, link, list, searchable_list, menu, native_menu, message, message_scroller, notification, pagination, popover, progress, questionnaire, radio, rating, resizable, scroll, select, separator, setting, shimmer, sidebar, skeleton, slider, speech, spinner, status_bar, stepper, switch, tab, table, tag, text, theme, time, title_bar, toolbar, tooltip, touch_selection, tree, virtual_list, window_border.
- Apps (https://github.com/zed-industries/awesome-gpui, 1,555 stars): Zed, Delta, GitComet, Arbor, Eleven, DBDelve, Loungy (archived). Libraries: GPUIX (React bindings, https://github.com/remorses/gpuix, 2,867 stars, Apache-2.0) and gpui-router.

## 3. Taffy

- Repo: https://github.com/DioxusLabs/taffy, 3,734 stars, MIT, last push 2026-11-03. Release v0.15.1 (2026-10-25), with flexbox 15-20% faster on deep trees.
- 0.15.0 added `position: static` (now the default) and `fixed` with containing-block hoisting, a `normal` alignment keyword, ~60% faster grid and `fit-content(calc())`, and dropped no-alloc mode.
- Earlier releases: 0.14 added sizing keywords, `flex-wrap: balance` and partial containment.
- Changes not yet released: a `ScrollableOverflow` type and float-context APIs.
- Default features: `flexbox`, `flexbox_balance`, `grid`, `block_layout`, `float_layout`, `calc`, `content_size`, `detailed_layout_info`.
- Users, per the README: Servo, Blitz, Bevy, Slint, Floem/Lapce, Zed/GPUI.
- What yolo-layout lacks (`crates/yolo-layout/src/style.rs`, `to_taffy.rs`):
  - lengths: `Length` covers only `Auto | Px | Percent`, so there is no `calc()`, `rem/em`, `vw/vh`, `min/max/fit-content` keywords or `fr` outside tracks;
  - float layout: no `float`/`clear` and no `display: flow-root`;
  - grid: no `grid-template-areas`, named lines or `detailed_layout_info` readback;
  - no `flex-wrap: balance`;
  - no scrollbar-gutter or ScrollableOverflow output;
  - no incremental dirty marking beyond what `TaffyTree` provides;
  - no pixel snapping or tap-target pass (contrary to AGENTS.md);
  - its `bevy` feature emits `bevy::ui::Node`, which Bevy 0.20 lays out with taffy 0.14, so two Taffy versions get compiled in. This ends once a Bevy release ships taffy 0.15 (timing UNVERIFIED); until then `to_bevy.rs` should keep emitting `Node` values rather than sharing Taffy types.

## 4. Reuse table

| Crate | Version | License | wasm | What we take | Consumer | Submodule |
|---|---|---|---|---|---|---|
| taffy | 0.15.1 | MIT | yes | layout core, calc, float, areas | yolo-layout, yolo-react | yes (dioxus-taffy 0.15.0; taffy 0.12.1 stale) |
| gpui-kit | 0.9.0 | Apache-2.0 | yes (story-web) | components, DockArea, theme JSON, test harness | yolo-gpui | yes |
| gpui-pre | 0.3.11 | Apache-2.0 | yes (gpui-pre-web) | GPUI runtime, wgpu 30 | yolo-gpui, yolo-engines | via gpui-fast |
| gpui-fast | 0.1.13 | Apache-2.0 | yes (gpui_web) | faster-compiling GPUI, optional | yolo-gpui | yes |
| gpui-mobile | 0.1.0 | AGPL-3.0 | no | reference only | none | yes (license risk) |
| bevy_ui + bevy_ui_widgets + bevy_feathers | 0.20.1 | MIT/Apache-2.0 | yes | headless widgets, feathers theming target | yolo-layout (to_bevy), yolo-emit | bevy (not checked out) |
| bevy_picking | 0.20.1 | MIT/Apache-2.0 | yes | pointer picking for 3D | yolo-3d | in bevy |
| bevy_egui | 0.43.1 | MIT | yes (UNVERIFIED) | egui in Bevy | yolo-egui, yolo-3d | no |
| bevy-inspector-egui | 0.38.0 | MIT/Apache-2.0 | UNVERIFIED | inspector/editor panels | yolo-3d dev tools | yes (not checked out) |
| bevy_tweening | 0.17.0 | MIT/Apache-2.0 | UNVERIFIED | tween easing to motion tokens | yolo-tokens motion, yolo-3d | no |
| bevy_flair | 0.9.0 | MIT/Apache-2.0 | UNVERIFIED | CSS to bevy_ui, compare with yolo-layout | yolo-layout | no |
| vello | 0.11.0 | Apache-2.0/MIT | yes | 2D vector path | yolo-render | via anyrender/vello_svg |

## 5. Next steps, ranked by value

1. **Retire the stale GPUI pin.** In `Cargo.toml` l.846-849 and the `[patch]` at l.1681-1683, `gpui` 0.2.2 (spacedrive fork, taffy 0.9) and the `gpui-component` git rev sit beside the `gpui-kit` 0.9.0 submodule (gpui-pre 0.3.11, taffy 0.15, wgpu 30.0.1). Move to `gpui-kit = { path = "submodules/gpui-kit/crates/kit" }`. `apps/spacedrive/apps/native` is the only consumer: migrate it, or keep it on its own feature.
2. **Make `packages/yolo-gpui` real.** Add optional `gpui-kit`, then emit `yolo-tokens` roles as GPUI Kit theme JSON (format in `submodules/gpui-kit/themes/`). That needs a new `Gpui` target in `crates/yolo-emit` covering the 7 presets, with `crates/story-web` as the web gallery reference.
3. **Close the yolo-layout gaps.** Extend `Length` with `Calc`, `Rem`, `Em`, `Vw`, `Vh` and sizing keywords. Add float/clear, `grid-template-areas`, named lines and balance wrap in `style.rs`, `parse.rs`, `tailwind.rs` and `to_taffy.rs`. Bump `submodules/dioxus-taffy` to 0.15.1. Drop or fast-forward `submodules/taffy` (0.12.1). Either implement the snapping/tap-target pass or remove the claim from AGENTS.md.
4. **wgpu alignment.** Everything lands on wgpu 30: workspace 30.0.1, bevy_render ^30, gpui-pre-wgpu ^30.0.1, vello 0.11 ^30, egui-wgpu 0.37 ^30, gpui-mobile 30.0.1. The exceptions:
   - Zed trunk and the `submodules/gpui-fast` root `Cargo.toml` use a git wgpu rev (`cddb2f5`), which forks the graph.
   - `submodules/iced` (0.14) uses wgpu 28; libcosmic is assumed to follow it (UNVERIFIED).

   Policy: consume crates.io `gpui-pre`, not Zed git. Isolate the iced/cosmic engines behind features until iced moves to 30. Confirm with `cargo tree -d -i wgpu` (not run here).
5. **Bevy editor path.** Check out `submodules/bevy` (v0.20.1) and `submodules/bevy-inspector-egui`. Add `bevy_egui` 0.43 and `bevy-inspector-egui` 0.38 under the `crates/yolo-3d` `bevy` feature; both use egui ^0.36, matching the workspace's 0.36.2, so leave egui alone. Follow `bevy_inspector` and `bevy_feathers` on main for an egui-free editor. Fix the "Bevy 0.16" string in `crates/yolo-3d/Cargo.toml`.
6. **Fix the drifting docs** listed in section 0, and point `submodules/awesome-bevy` at `bevyengine/bevy-assets`.
