//! `shenron-admin-ai`: the cockpit client. Same component tree, two renderers:
//! - `--features web --target wasm32-unknown-unknown`: mounted on `#main` of the hub's HTML shell;
//! - `--features desktop`: a native window (system WebView) talking to the hub over HTTP + SSE.

#[cfg(feature = "desktop")]
fn main() {
    use dioxus_desktop::{Config, LogicalSize, WindowBuilder};
    let lang = shenron_admin_ai::client::lang();
    let title = shenron_admin_ai::i18n::t(lang, "admin.pages.ai.title");
    let head = format!(
        "<style>{}</style><meta name=\"color-scheme\" content=\"dark light\">",
        shenron_admin_ai::STYLESHEET
    );
    let window = WindowBuilder::new()
        .with_title(title)
        .with_inner_size(LogicalSize::new(1280.0, 860.0))
        .with_min_inner_size(LogicalSize::new(375.0, 560.0));
    dioxus::LaunchBuilder::desktop()
        .with_cfg(
            Config::new()
                .with_window(window)
                .with_custom_head(head)
                .with_menu(None),
        )
        .launch(shenron_admin_ai::ui::App);
}

#[cfg(all(feature = "web", not(feature = "desktop")))]
fn main() {
    dioxus::LaunchBuilder::web()
        .with_cfg(dioxus_web::Config::new().rootname("main"))
        .launch(shenron_admin_ai::ui::App);
}

#[cfg(not(any(feature = "web", feature = "desktop")))]
fn main() {
    eprintln!(
        "shenron-admin-ai: build with --features web (wasm32) or --features desktop; the hub is shenron-admin-ai-server"
    );
}
