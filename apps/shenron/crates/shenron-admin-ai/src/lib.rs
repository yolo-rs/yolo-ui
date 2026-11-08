//! Shenron admin AI cockpit (`/admin/ai`) rebuilt in Rust.
//!
//! - core (no feature): data shapes, typed SSE protocol, reducer, reconnect backoff, admin gate,
//!   formatting and i18n. Shared by every target and covered by `cargo test`.
//! - `ui`: the Dioxus 0.7 component tree, rendered by `web` (wasm32) or `desktop` (WebView).
//! - `server`: the Axum hub (one upstream poller, SSE fan-out, action proxy, fixture mode).

pub mod backoff;
pub mod format;
pub mod gate;
pub mod i18n;
pub mod icons;
pub mod model;
pub mod protocol;
pub mod sse;
pub mod state;

#[allow(dead_code)]
pub mod tokens;

/// The generated stylesheet (`build.rs`: HCT palette with enforced contrast, structural rules).
pub const STYLESHEET: &str = include_str!(concat!(env!("OUT_DIR"), "/admin-ai.css"));

#[cfg(feature = "ui")]
pub mod ui;

#[cfg(any(feature = "web", feature = "desktop"))]
pub mod client;

#[cfg(feature = "server")]
pub mod server;

#[cfg(test)]
mod contrast_tests {
    include!(concat!(env!("OUT_DIR"), "/contrast.rs"));

    #[test]
    fn every_declared_pair_meets_wcag() {
        assert!(CONTRAST.len() > 40);
        for (scheme, fg, bg, ratio, required) in CONTRAST {
            assert!(
                ratio >= required,
                "{scheme}: {fg} on {bg} is {ratio}:1 < {required}:1"
            );
        }
        // Text on every surface in both schemes, and the focus ring at 3:1.
        for scheme in ["dark", "light"] {
            for bg in ["bg", "surface", "surface-2", "surface-3"] {
                assert!(
                    CONTRAST
                        .iter()
                        .any(|c| c.0 == scheme && c.1 == "text" && c.2 == bg && c.3 >= 4.5)
                );
                assert!(
                    CONTRAST
                        .iter()
                        .any(|c| c.0 == scheme && c.1 == "focus" && c.2 == bg && c.3 >= 3.0)
                );
            }
        }
    }

    #[test]
    fn stylesheet_is_generated_with_both_schemes() {
        let css = super::STYLESHEET;
        assert!(css.contains("--c-primary:"));
        assert!(css.contains(":root[data-theme=\"light\"]"));
        assert!(css.contains("prefers-reduced-motion"));
        assert!(css.contains(":focus-visible"));
    }
}
