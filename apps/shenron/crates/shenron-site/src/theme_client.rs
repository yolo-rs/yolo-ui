//! Browser side effects of the theme system: storage, cookie, `<html>` attributes, custom CSS.
//!
//! Everything touching `web_sys` is behind the `hydrate` feature; the server build gets inert
//! stubs so the islands compile for both targets.

#[cfg(feature = "hydrate")]
mod imp {
    use wasm_bindgen::{JsCast, prelude::Closure};
    use yolo_stack_theme::prefs::{SchemePref, UserThemePrefs};

    use crate::themes;

    const COOKIE: &str = "shenron_theme";
    const STORAGE_PREFS: &str = "shenron.theme";
    const STORAGE_CSS: &str = "shenron.theme.css";

    fn window() -> Option<web_sys::Window> {
        web_sys::window()
    }

    fn storage() -> Option<web_sys::Storage> {
        window()?.local_storage().ok()?
    }

    /// Saved preferences: cookie first, then localStorage.
    pub fn load() -> UserThemePrefs {
        let from_cookie = window()
            .and_then(|w| w.document())
            .and_then(|d| d.dyn_into::<web_sys::HtmlDocument>().ok())
            .and_then(|d| d.cookie().ok())
            .and_then(|c| {
                c.split("; ")
                    .find_map(|kv| kv.strip_prefix(&format!("{COOKIE}=")).map(str::to_owned))
            });
        let raw = from_cookie
            .or_else(|| storage().and_then(|s| s.get_item(STORAGE_PREFS).ok().flatten()));
        raw.map(|v| UserThemePrefs::from_cookie_value(&v))
            .unwrap_or_default()
    }

    /// Applies preferences to the page and persists them.
    pub fn apply(prefs: &UserThemePrefs) {
        let Some(doc) = window().and_then(|w| w.document()) else {
            return;
        };
        let Some(html) = doc.document_element() else {
            return;
        };
        let theme = themes::resolved(prefs);
        let _ = html.set_attribute("data-theme", &theme.id);
        let _ = match prefs.scheme {
            SchemePref::System => html.remove_attribute("data-scheme"),
            SchemePref::Light => html.set_attribute("data-scheme", "light"),
            SchemePref::Dark => html.set_attribute("data-scheme", "dark"),
        };
        let _ = if prefs.reduced_motion {
            html.set_attribute("data-motion", "reduce")
        } else {
            html.remove_attribute("data-motion")
        };

        let css = themes::is_customized(prefs).then(|| themes::user_css(prefs));
        let style = doc.get_element_by_id("user-theme").or_else(|| {
            let el = doc.create_element("style").ok()?;
            el.set_id("user-theme");
            doc.head()?.append_child(&el).ok()?;
            Some(el)
        });
        if let Some(style) = style {
            style.set_text_content(Some(css.as_deref().unwrap_or("")));
        }
        let _ = if css.is_some() {
            html.set_attribute("data-user", "")
        } else {
            html.remove_attribute("data-user")
        };

        let value = prefs.to_cookie_value();
        if let Ok(doc) = doc.dyn_into::<web_sys::HtmlDocument>() {
            let _ = doc.set_cookie(&format!(
                "{COOKIE}={value}; Path=/; Max-Age=31536000; SameSite=Lax"
            ));
        }
        if let Some(store) = storage() {
            let _ = store.set_item(STORAGE_PREFS, &value);
            let _ = match css {
                Some(css) => store.set_item(STORAGE_CSS, &css),
                None => store.remove_item(STORAGE_CSS),
            };
        }
    }

    /// OS-level reduced motion.
    pub fn os_reduced_motion() -> bool {
        window()
            .and_then(|w| {
                w.match_media("(prefers-reduced-motion: reduce)")
                    .ok()
                    .flatten()
            })
            .is_some_and(|m| m.matches())
    }

    /// Calls `f(x, y)` with the pointer offset from the viewport centre, scaled to -1..1.
    pub fn on_pointer_move(f: impl Fn(f64, f64) + 'static) {
        let Some(win) = window() else { return };
        let cb = Closure::<dyn Fn(web_sys::PointerEvent)>::new(move |e: web_sys::PointerEvent| {
            let Some(win) = web_sys::window() else { return };
            let w = win
                .inner_width()
                .ok()
                .and_then(|v| v.as_f64())
                .unwrap_or(1.0)
                .max(1.0);
            let h = win
                .inner_height()
                .ok()
                .and_then(|v| v.as_f64())
                .unwrap_or(1.0)
                .max(1.0);
            f(
                (f64::from(e.client_x()) / w - 0.5) * 2.0,
                (f64::from(e.client_y()) / h - 0.5) * 2.0,
            );
        });
        let _ = win.add_event_listener_with_callback("pointermove", cb.as_ref().unchecked_ref());
        cb.forget();
    }
}

#[cfg(not(feature = "hydrate"))]
mod imp {
    use yolo_stack_theme::prefs::UserThemePrefs;

    pub fn load() -> UserThemePrefs {
        UserThemePrefs::default()
    }
    pub fn apply(_: &UserThemePrefs) {}
    pub fn os_reduced_motion() -> bool {
        false
    }
    pub fn on_pointer_move(_: impl Fn(f64, f64) + 'static) {}
}

pub use imp::{apply, load, on_pointer_move, os_reduced_motion};

/// Shared reactive preferences for all islands of the page.
pub mod state {
    use leptos::prelude::*;
    use yolo_stack_theme::prefs::UserThemePrefs;

    thread_local! {
        static PREFS: ArcRwSignal<UserThemePrefs> = ArcRwSignal::new(UserThemePrefs::default());
    }

    /// The page-wide preferences signal (islands are separate roots, this is their bridge).
    pub fn prefs() -> ArcRwSignal<UserThemePrefs> {
        PREFS.with(Clone::clone)
    }
}
