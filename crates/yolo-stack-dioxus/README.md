# yolo-stack-dioxus (planned)

Dioxus 0.8 islands and native adapter. Not a workspace member until code lands.

Planned `Cargo.toml` dependencies (all from the root `[workspace.dependencies]`):

- `dioxus`, `dioxus-fullstack`, `dioxus-router`, `dioxus-ssr` from `C:/src/yolo-ui/packages/*` (0.8.0-alpha.3).
- `yolo-tokens` (HCT/CAM16 roles) and `yolo-layout` (Taffy 0.15) from `C:/src/yolo-ui/crates/*`.
- `yolo-stack-routes` for the shared route table.

Surface:

- `render_island(name, props_json) -> String`: static HTML for one island, called by the TanStack
  shell at SSR time through the `@yolo-rs/fullstack` Dioxus bridge.
- `/api/*`: Axum router with the Dioxus server functions, mounted behind the bridge proxy.
- `desktop` feature: the same components in `dioxus-desktop` / Blitz for the native showcase.

Removal condition: none; this is the Rust-first layer of the framework.
