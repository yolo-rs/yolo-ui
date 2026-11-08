//! Shenron reader PoC: Dioxus 0.7 component tree -> Taffy 0.15 layout -> wgpu 30 paint.

mod app;
mod gpu;
mod layout;
mod platform;
mod theme;

fn main() {
    #[cfg(feature = "desktop")]
    {
        use dioxus::desktop::{Config, LogicalSize, WindowBuilder};
        // The webview is transparent over the native window so the wgpu surface,
        // which paints directly on the tao window, shows through the canvas area.
        let cfg = Config::new()
            .with_window(
                WindowBuilder::new()
                    .with_title("Shenron Reader PoC")
                    .with_transparent(true)
                    .with_inner_size(LogicalSize::new(1280.0, 800.0)),
            )
            .with_background_color((0, 0, 0, 0))
            .with_menu(None);
        dioxus::LaunchBuilder::desktop()
            .with_cfg(cfg)
            .launch(app::App);
    }
    #[cfg(all(feature = "web", not(feature = "desktop")))]
    {
        dioxus::launch(app::App);
    }
    #[cfg(not(any(feature = "desktop", feature = "web")))]
    {
        eprintln!("Build with --features desktop or --features web");
    }
}
