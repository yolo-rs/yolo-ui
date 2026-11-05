# Iced, libcosmic and egui ecosystem recon

Checked 2026-11-05. Stars, license and last push come from `gh api repos/<o>/<r>`. Versions come from `https://crates.io/api/v1/crates/<c>` and its `/dependencies` endpoint. Items marked UNVERIFIED were not confirmed against a fetched source.

## 1. Iced and libcosmic

**Iced** (`iced-rs/iced`, 31,917 stars, MIT, pushed 2026-11-01)
- The current release is 0.14.0 (crates.io, 2025-12-07). Docs: https://docs.rs/iced/latest/iced/. On master, `submodules/iced/Cargo.toml` declares workspace version 0.14.0. `iced_aw` main tracks iced master as `0.15.0-dev` (`submodules/iced_aw/Cargo.toml:78-95`).
- Book: https://book.iced.rs/. The landing page names no target version (UNVERIFIED which release it tracks).
- Renderers: `iced_wgpu` (Vulkan/Metal/DX12) plus the `iced_tiny_skia` software fallback (https://github.com/iced-rs/iced/blob/master/README.md). The README lists Windows, macOS, Linux and the Web. The root manifest has a `webgl` feature (`submodules/iced/Cargo.toml:79`) and depends on `wasm-bindgen-futures`. Whether WebGPU works on wasm without `webgl` is UNVERIFIED.
- Debug tooling: devtools, plus "performance metrics and time traveling" (README). The tree contains `devtools/`, `beacon/`, `tester/`, `selector/` and `accessibility/`.
- Widgets on master (`submodules/iced/widget/src`): button, canvas, checkbox, combo_box, container, float, grid, image, keyed, lazy, markdown, mouse_area, pane_grid, pick_list, pin, progress_bar, qr_code, radio, responsive, rule, scrollable, sensor, shader, slider, stack, svg, table, text_editor, text_input, themer, toggler, tooltip, vertical_slider.
- Theming: you can build a custom theme with `Theme::custom`, read colors through `Theme::palette()` and `extended_palette()`, and style a widget with a `style(|theme, status|)` closure. Built-in style functions include `button::primary` and `container::rounded_box` (docs.rs).

**Companion crates**
- `iced_aw` 0.14.1 (691 stars, MIT, pushed 2026-08-29) requires `iced_core ^0.14` and `iced_widget ^0.14.2`.
- `iced_fonts` 0.3.0 requires iced 0.14. The repository listed on crates.io is `Redhawk18/iced_fonts`; it returned 404 to `gh api`, so its license and stars are UNVERIFIED.
- `plotters-iced` 0.11.0 (205 stars, MIT, pushed 2025-12-11) requires `iced_widget ^0.13`. It is a release behind 0.14.
- `iced_audio` 0.17.1 (232 stars, MIT), `comet` (65 stars, MIT) and `cryoglyph` (6 stars, Apache-2.0) are all active in 2026.

**awesome-iced** (525 stars, NOASSERTION, pushed 2026-02-07, https://github.com/iced-rs/awesome-iced)
- Apps worth studying: Halloy (4,589 stars, GPL-3.0), Sniffnet, Veloren, liana, Furtherance, and the COSMIC apps (cosmic-files 309 stars, cosmic-term 600 stars, both GPL-3.0). Also `hecrj/icebreaker` (468 stars, MIT).
- Libraries: iced_code_editor, iced_video_player, iced_gif, iced_color_picker, iced_term, Cosmic Time (animation), anim-rs, bevy_iced, nih-plug.
- The list has no docking, node-editor or markdown entries (markdown is now built into iced).

**libcosmic** (`pop-os/libcosmic`, 966 stars, MPL-2.0, pushed 2026-11-03, not on crates.io)
- Book: https://pop-os.github.io/libcosmic-book/
- The `submodules/libcosmic/src/widget` directory includes nav_bar, header_bar, segmented_button/control, context_drawer, dropdown, flex_row, reorderable_flex_row, settings, spin_button, toaster, calendar, color_picker, dnd_source/destination, table, popover, menu and responsive_menu_bar.
- Theme sources: `cosmic-theme` and `cosmic-config` (RON).
- libcosmic vendors its own iced fork (`libcosmic/Cargo.toml:191`, `path = "./iced"`). Under `submodules/` that nested submodule is not initialized.
- MPL-2.0 is file-level copyleft. Modified libcosmic files must stay MPL.

## 2. egui

**egui** (`emilk/egui`, 32,007 stars, Apache-2.0 per GitHub, MIT OR Apache-2.0 per docs.rs, pushed 2026-11-02)
- The current release is 0.37.0 (2026-10-15, MSRV 1.95). Docs: https://docs.rs/egui/latest/egui/.
- 0.37 release notes (https://github.com/emilk/egui/releases/tag/0.37.0):
  - experimental `StyleProvider` class-based theming behind the `experimental` feature;
  - system glyph and font providers (`egui_system_font_provider`);
  - `CompletionPopup` and `RangeSlider`;
  - `accesskit::Role` replaces `WidgetType`;
  - `egui_kittest` can record MP4 videos of tests;
  - `default_fonts` is no longer a default feature;
  - `Id::new` is deprecated.
- The `submodules/egui` fork is already at 0.37.0. It contains `egui_inspection`, `egui_web`, `egui_plot` and `egui_system_font_provider` crates whose upstream status is UNVERIFIED.
- eframe covers native and wasm32. Web uses the `web_file` feature (docs.rs). Template: `emilk/eframe_template` (1,172 stars, Apache-2.0).

**Companion crates and their egui requirement** (from crates.io dependencies)

| egui ^0.36 | egui ^0.37 | older |
|---|---|---|
| egui_dock 0.21.1 | egui_plot 0.38.0 | puffin_egui 0.30.0 (egui ^0.33.3) |
| egui-phosphor 0.14.0 | egui_tiles 0.18.0 | |
| egui-snarl 0.12.0 | egui_commonmark 0.26.0 | |
| egui_graphs 0.32.0 | egui_kittest 0.37.0 | |

- `egui_dock` moved to `anhosh/egui_dock` (647 stars, MIT).
- `egui_tiles`, `egui_table` 0.11.0 and `egui_mcp` 0.3.0 (agent control and inspection of egui apps) now live in `rerun-io/egui_crates` (599 stars, Apache-2.0, pushed 2026-11-03).
- `puffin` 0.20.0 (1,761 stars, Apache-2.0, pushed 2026-10-26).
- Node graphs: `setzer22/egui_node_graph` returns 404. Its active alternatives are `egui-snarl` (606 stars, license NOASSERTION, pushed 2026-08-26) and `egui_graphs` (708 stars, MIT).
- Learning references: `rerun-io/rerun` (11,696 stars, Apache-2.0) for tiles, tables, theming and kittest at scale; `bevy-inspector-egui` (1,652 stars).
- `vonnieda/awesome-egui` (48 stars, CC0) is archived. Treat it as a historical list only.

## 3. Reuse table

| Crate | Version | License | wasm | What Yolo UI takes | Consumer | Submodule |
|---|---|---|---|---|---|---|
| iced | 0.14.0 | MIT | yes (`webgl`) | Real `Program`/`Task`/`Theme::custom`, wgpu and tiny-skia | yolo-engines `iced` | yes (`submodules/iced`) |
| iced_aw | 0.14.1 | MIT | UNVERIFIED | Tabs, split, card, number_input, menu | yolo-iced `aw` | yes |
| iced_fonts | 0.3.0 | UNVERIFIED | UNVERIFIED | Icon fonts; candidate Phosphor bridge | yolo-iced | no |
| plotters-iced | 0.11.0 | MIT | UNVERIFIED | Charts (requires iced 0.13; wait or fork) | yolo-iced | no |
| iced_audio | 0.17.1 | MIT | UNVERIFIED | Knob, fader, XY pad | yolo-iced `audio` | yes |
| comet | 0.15.0-dev | MIT | no (UNVERIFIED) | Time-travel debugger | yolo-iced `comet` | yes |
| libcosmic | git | MPL-2.0 | no (Wayland focus, UNVERIFIED) | nav_bar, header_bar, settings, segmented, cosmic-theme | yolo-cosmic | yes (nested iced empty) |
| egui / eframe | 0.37.0 | MIT/Apache | yes | StyleProvider, AccessKit, font providers | yolo-engines `egui`, yolo-craft | yes (0.37); workspace pins 0.36.2 |
| egui_extras | 0.37.0 | MIT/Apache | yes | SVG loader, TableBuilder | yolo-craft icons | via egui submodule |
| egui_tiles | 0.18.0 | Apache-2.0 | yes | Tiling and docking reference or replacement | yolo-craft docking | no |
| egui_dock | 0.21.1 | MIT | yes | Tabbed docking (egui 0.36) | yolo-craft docking | no |
| egui_plot | 0.38.0 | Apache-2.0 | yes | Charts | yolo-craft, showcase | in fork tree (UNVERIFIED) |
| egui_table | 0.11.0 | Apache-2.0 | UNVERIFIED | Tables with millions of rows | yolo-craft | no |
| egui-phosphor | 0.14.0 | Apache-2.0 | yes | Phosphor icon font (CLAUDE.md requires Phosphor) | yolo-engines egui, yolo-craft | no |
| egui-snarl | 0.12.0 | NOASSERTION | UNVERIFIED | Node editor for DialogueGraph and Storyteller | yolo-craft, yolo-storyteller | no |
| egui_kittest | 0.37.0 | MIT/Apache | n/a | Snapshot and a11y tests | yolo-engines, yolo-craft tests | via egui submodule |
| egui_mcp | 0.3.0 | Apache-2.0 | n/a | Agent inspection; feeds `ui_devtools` | yolo-ui-mcp bridge | no |
| puffin | 0.20.0 | Apache-2.0 | partial (UNVERIFIED) | Profiler scopes | showcase | listed, but `submodules/puffin` is empty |

## 4. Divergences from upstream best practice (evidence)

1. **The iced adapter does not link iced.** `crates/yolo-engines/Cargo.toml` defines `iced = []`, `aw = []`, `audio = []`, `comet = []` and `cryoglyph = []` with no optional dependencies. `packages/yolo-iced/src/lib.rs` only re-exports `yolo_engines::iced::*`. `crates/yolo-engines/src/iced.rs:91,147` re-implements `Task` and `Subscription` "matching" iced. The workspace declares `iced_aw`, `iced_audio` and `iced_comet` (`Cargo.toml:822-824`), but no crate consumes them. The yolo-iced features `wgpu`, `tiny-skia` and `opengl` are empty.
2. **The cosmic adapter mirrors ThemeBuilder by hand.** `crates/yolo-engines/src/cosmic/cosmic.rs:2-13` says it works "without linking libcosmic". The structure will drift whenever cosmic-theme changes. The comment at `cosmic/mod.rs:3` still says "Iced 0.13 / 0.14".
3. **egui version split.** The workspace pins `egui 0.36.2` (`Cargo.toml:591-592`) while `submodules/egui` is 0.37.0. As a result, egui_tiles, egui_plot and egui_kittest 0.37 cannot be added. egui_dock and egui-phosphor are still on 0.36.
4. **Custom docking instead of egui_tiles or egui_dock.** `crates/yolo-craft/src/docking/` holds about 2,660 lines of source plus 1,739 lines of tests. The dock model and drag/drop are hand-built.
5. **Showcase claims the egui stack, but the dependencies are missing.** `apps/yolo-showcase/src/site_hub.rs:408` advertises "egui_tiles, egui_plot, Rerun SDK, Puffin profiler". `apps/yolo-showcase/Cargo.toml` has no egui dependency, and `submodules/puffin` is empty.
6. **Icons.** `crates/yolo-craft/src/icons/mod.rs` expects the host to install egui_extras' SVG loader, but `crates/yolo-craft/Cargo.toml` does not declare egui_extras. There is no Phosphor integration, although the user's global CLAUDE.md requires the Phosphor vector engine.
7. **Theming happens through Visuals only.** `crates/yolo-engines/src/egui/mod.rs:245,424` sets `Visuals` and `Style`. There is no `StyleProvider` path (new in 0.37).
8. **Stale metadata.** The Cargo descriptions of yolo-egui and yolo-cosmic say "5-preset", but the adapters handle all presets including Chromium. `packages/yolo-iced/README.md` names `aphrody-labs/*` forks, while `.gitmodules` points at `yolo-rs/*`.

## 5. Next steps ranked by value

1. **Make yolo-iced real.** Add `iced = { path = "submodules/iced", optional = true }` to `crates/yolo-engines/Cargo.toml` and wire `iced`/`aw`/`audio`/`comet` to `dep:` entries. Convert `IcedTheme` into `iced::Theme::custom` in `crates/yolo-engines/src/iced.rs`. Drop the shadow `Task`/`Subscription` types or keep them only behind a `no-iced` feature.
2. **Move to egui 0.37.** Bump `Cargo.toml:591-592` to match the submodule, then adopt `egui_kittest` snapshot tests in `crates/yolo-engines/src/egui/` and `crates/yolo-craft/src/docking/tests.rs`. Gate a `StyleProvider` implementation behind `experimental`. Blocker: egui_dock, egui-phosphor and egui-snarl still need 0.36, so either pick egui_tiles or wait for their releases.
3. **Docking.** Evaluate `egui_tiles` 0.18 as the backend for `crates/yolo-craft/src/docking/surface.rs`. Keep `model.rs`'s transactional validation as a layer above it. This cuts drag/drop code and aligns with Rerun.
4. **Phosphor icons.** Add `egui-phosphor` (egui side) and a Phosphor font through `iced_fonts` (iced side). Declare `egui_extras` with the `svg` feature in `crates/yolo-craft/Cargo.toml`.
5. **Link libcosmic's theme crate.** Use `cosmic-theme` and `cosmic-config` from `submodules/libcosmic` behind an optional `libcosmic` feature in `crates/yolo-engines/src/cosmic/cosmic.rs`, so ThemeBuilder fields are checked by the compiler. Initialize the nested `submodules/libcosmic/iced`. Keep MPL-2.0 files separate.
6. **Agent devtools.** Bridge `egui_mcp` 0.3 into `packages/yolo-ui-mcp/src/tools.ts` (`ui_devtools`) for live egui inspection.
7. **Showcase honesty.** Initialize `submodules/puffin`, use `puffin` together with `puffin_http` (puffin_egui is stuck on egui 0.33), and add `egui_plot`/`egui_tiles` to `apps/yolo-showcase/Cargo.toml`. Otherwise remove the claim at `site_hub.rs:408`.
8. **Node editor.** Use `egui-snarl` for DialogueGraph (`crates/yolo-craft/src/workbenches.rs`) once its license is confirmed; GitHub reports NOASSERTION.
