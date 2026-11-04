# Migration scope and exclusions

## Product work excluded from completion claims

`apps/yolo-showcase` is an active Rust binary in the root Cargo workspace. It demonstrates the unified engines, themes, migration catalog, and app gallery. It is a showcase, not a framework library or release qualification. Its generated product pages and benchmark table labels do not establish shipped cross-platform applications or measured runtime performance.

Older migration plans and benchmark comparisons included broad product-convergence claims and numerical performance figures without reproducible receipts. They are not evidence of completed migrations or measured performance. Performance claims require matched builds, named workloads, and retained raw samples.

`apps/locally-uncensored` remains separately labeled product code with its AGPL-3.0-only license. It is not counted as framework migration evidence. Do not fold Aphrody or BunOS product catalogs into framework acceptance criteria.

## Retained framework/library candidates

The following first-party crates have dedicated manifests and non-empty Rust library sources in the current root workspace. This verifies workspace/source presence only; it does not claim that every crate builds on every target or has release qualification.

| Crate            | Retained responsibility                                 |
| ---------------- | ------------------------------------------------------- |
| `yolo-shared`    | Shared UI types and primitives                          |
| `yolo-contracts` | UI-facing contracts and accessibility constraints       |
| `yolo-tokens`    | Theme/token model and serialization                     |
| `yolo-emit`      | Typed code/style emission                               |
| `yolo-layout`    | Layout abstractions backed by Taffy                     |
| `yolo-render`    | Renderer-facing abstractions                            |
| `yolo-gpu`       | GPU/rendering integration surface                       |
| `yolo-shell`     | Desktop-shell adapters                                  |
| `yolo-migrator`  | Source transformation and migration tooling             |
| `yolo-engines`   | Explicit toolkit adapters (Iced, Egui, GTK, Qt, Cosmic) |

The imported Dioxus, Tauri, GPUI, Iced, Egui and Spacedrive sources retain their upstream manifests, licenses and provenance. Their presence is not a claim that a unified product, browser, app catalog, or cross-platform release has shipped.

## Current qualification boundary

- Root workspace membership is not test/build/runtime evidence.
- A toolkit adapter is supported only for the host/target combinations with recorded build and runtime receipts.
- Chromium/CEF ownership belongs to `yolo-web`; a UI workbench or CDP client is not an embedded browser engine.
- Cross-target compilers and Windows/MSVC host tooling belong to `yolo-cross`.
- Product showcases can verify workspace wiring; they do not establish release support. Unsupported benchmark tables are excluded from performance claims.
