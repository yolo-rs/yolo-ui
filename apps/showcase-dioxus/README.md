# showcase-dioxus (planned)

Dioxus 0.8 fullstack app (web + desktop + Blitz native) rendering the shared route set.

Planned layout:

```text
Cargo.toml           # depends on crates/yolo-stack-dioxus; joins the root workspace with it
src/main.rs          # dioxus::launch, Routable enum generated from the shared routes
src/routes/*.rs      # one component per route
Dioxus.toml          # web + desktop platforms, wasm-split per route
package.json         # bun run start: @yolo-rs/fullstack Dioxus bridge serving target/dx web output
```

Dev loop: `dx serve --hotpatch` (Subsecond; Windows x86_64/arm64 supported per docs.rs/subsecond).
Measured when it exists: `dx build --release` time, `.wasm` size (raw/gzip/brotli), rps per route,
first paint of `island-counter`.
