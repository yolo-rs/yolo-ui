# showcase-leptos (planned, optional)

Leptos 0.8.23 SSR + islands app rendering the shared route set. Exists to measure the minimal-wasm
islands claim (Leptos book: 24 KB static vs 274 KB fully hydrated hello world, release,
uncompressed) against the Dioxus island on identical routes.

Planned layout:

```text
Cargo.toml           # depends on crates/yolo-stack-leptos; cargo-leptos metadata; islands feature
src/main.rs          # Axum server via leptos_axum
src/app.rs           # <Routes/> generated from the shared routes, #[island] counter
package.json         # bun run start: spawns target/release/showcase-leptos with PORT
```

Dev loop: `cargo leptos watch` (full rebuild + live reload; CSS-only hot reload).
Measured when it exists: `cargo leptos build --release` time, `target/site/pkg/*.wasm` size,
rps per route.
