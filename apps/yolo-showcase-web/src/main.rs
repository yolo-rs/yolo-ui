//! Browser entry point. Build with
//! `cargo build -p yolo-showcase-web --features web --target wasm32-unknown-unknown --release`,
//! then `wasm-bindgen --target web --out-dir dist` and copy `index.html` next to the output.

fn main() {
    #[cfg(target_arch = "wasm32")]
    yolo_showcase_web::app::launch();
    #[cfg(not(target_arch = "wasm32"))]
    eprintln!("yolo-showcase-web runs in the browser: build it for wasm32-unknown-unknown.");
}
