//! Browser transport: `EventSource`, `fetch` (same-origin cookies), `history`, `localStorage`.

use super::{Endpoint, Reply, StreamEnd};
use crate::i18n::Lang;
use crate::protocol::{BASE_PATH, Failure, ServerEvent};
use futures_util::StreamExt;
use std::cell::RefCell;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;

fn window() -> web_sys::Window {
    web_sys::window().expect("window")
}

pub fn endpoint() -> Endpoint {
    Endpoint { base: BASE_PATH.to_owned(), bearer: None }
}

pub async fn sleep_ms(ms: u64) {
    let promise = js_sys::Promise::new(&mut |resolve, _| {
        let _ = window().set_timeout_with_callback_and_timeout_and_arguments_0(
            &resolve,
            ms.min(i32::MAX as u64) as i32,
        );
    });
    let _ = JsFuture::from(promise).await;
}

pub fn online() -> bool {
    window().navigator().on_line()
}

pub fn random_seed() -> u64 {
    (js_sys::Math::random() * 9_007_199_254_740_991.0) as u64
}

pub fn tz_offset_min() -> i32 {
    -(js_sys::Date::new_0().get_timezone_offset() as i32)
}

pub fn lang() -> Lang {
    window()
        .document()
        .and_then(|d| d.document_element())
        .and_then(|e| e.get_attribute("lang"))
        .and_then(|l| Lang::parse(&l))
        .unwrap_or_default()
}

pub fn initial_tab() -> Option<String> {
    let search = window().location().search().ok()?;
    web_sys::UrlSearchParams::new_with_str(&search)
        .ok()?
        .get("tab")
}

/// Keeps `?tab=` in the URL (shareable, back button works through `popstate`).
pub fn push_tab(tab: &str) {
    let Ok(href) = window().location().href() else {
        return;
    };
    let Ok(url) = web_sys::Url::new(&href) else {
        return;
    };
    if url.search_params().get("tab").as_deref() == Some(tab) {
        return;
    }
    url.search_params().set("tab", tab);
    if let Ok(history) = window().history() {
        let _ = history.push_state_with_url(&JsValue::NULL, "", Some(&url.href()));
    }
}

/// Calls `f` with the `?tab=` value on back/forward navigation.
pub fn on_popstate(mut f: impl FnMut(Option<String>) + 'static) {
    let cb = Closure::<dyn FnMut(web_sys::Event)>::new(move |_| f(initial_tab()));
    let _ = window().add_event_listener_with_callback("popstate", cb.as_ref().unchecked_ref());
    cb.forget();
}

const THEME_KEY: &str = "shenron-admin-ai-theme";

pub fn stored_theme_light() -> bool {
    window()
        .local_storage()
        .ok()
        .flatten()
        .and_then(|s| s.get_item(THEME_KEY).ok().flatten())
        .as_deref()
        == Some("light")
}

pub fn store_theme_light(light: bool) {
    if let Some(s) = window().local_storage().ok().flatten() {
        let _ = s.set_item(THEME_KEY, if light { "light" } else { "dark" });
    }
}

thread_local! {
    static FOCUS: RefCell<Option<web_sys::HtmlElement>> = const { RefCell::new(None) };
}

/// Remembers the focused element before a dialog opens.
pub fn remember_focus() {
    let el = window()
        .document()
        .and_then(|d| d.active_element())
        .and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok());
    FOCUS.with(|f| *f.borrow_mut() = el);
}

/// Gives focus back to it when the dialog closes.
pub fn restore_focus() {
    FOCUS.with(|f| {
        if let Some(el) = f.borrow_mut().take() {
            let _ = el.focus();
        }
    });
}

fn js_err(e: JsValue) -> Failure {
    Failure::network(e.as_string().unwrap_or_else(|| format!("{e:?}")))
}

pub async fn request(
    ep: &Endpoint,
    method: &str,
    path: &str,
    body: Option<String>,
) -> Result<Reply, Failure> {
    let init = web_sys::RequestInit::new();
    init.set_method(method);
    init.set_credentials(web_sys::RequestCredentials::SameOrigin);
    if let Some(b) = &body {
        init.set_body(&JsValue::from_str(b));
    }
    let req = web_sys::Request::new_with_str_and_init(&ep.url(path), &init).map_err(js_err)?;
    let headers = req.headers();
    let _ = headers.set("accept", "application/json");
    if body.is_some() {
        let _ = headers.set("content-type", "application/json");
    }
    let res: web_sys::Response = JsFuture::from(window().fetch_with_request(&req))
        .await
        .map_err(js_err)?
        .dyn_into()
        .map_err(js_err)?;
    let status = res.status();
    let text = match res.text() {
        Ok(p) => JsFuture::from(p)
            .await
            .ok()
            .and_then(|v| v.as_string())
            .unwrap_or_default(),
        Err(_) => String::new(),
    };
    let body = serde_json::from_str(&text).unwrap_or(serde_json::Value::Object(Default::default()));
    Ok(Reply { status, body })
}

enum Msg {
    Event(ServerEvent),
    Error,
    Offline,
}

/// One `EventSource` lifetime. Closed on error so the backoff (not the browser's fixed retry)
/// decides when to come back.
pub(crate) async fn stream_once(
    ep: &Endpoint,
    stale_ms: u64,
    on_event: &mut dyn FnMut(ServerEvent),
) -> StreamEnd {
    let Ok(es) = web_sys::EventSource::new(&ep.url("/events")) else {
        return StreamEnd::Lost;
    };
    let (tx, mut rx) = futures_channel::mpsc::unbounded::<Msg>();
    let mut keep: Vec<Closure<dyn FnMut(web_sys::Event)>> = Vec::new();
    for name in ServerEvent::NAMES {
        let tx = tx.clone();
        let cb = Closure::<dyn FnMut(web_sys::Event)>::new(move |e: web_sys::Event| {
            let Some(data) = e
                .dyn_ref::<web_sys::MessageEvent>()
                .and_then(|m| m.data().as_string())
            else {
                return;
            };
            if let Ok(ev) = serde_json::from_str::<ServerEvent>(&data) {
                let _ = tx.unbounded_send(Msg::Event(ev));
            }
        });
        let _ = es.add_event_listener_with_callback(name, cb.as_ref().unchecked_ref());
        keep.push(cb);
    }
    let txe = tx.clone();
    let on_error = Closure::<dyn FnMut(web_sys::Event)>::new(move |_| {
        let _ = txe.unbounded_send(Msg::Error);
    });
    es.set_onerror(Some(on_error.as_ref().unchecked_ref()));
    let txo = tx.clone();
    let on_offline = Closure::<dyn FnMut(web_sys::Event)>::new(move |_| {
        let _ = txo.unbounded_send(Msg::Offline);
    });
    let _ =
        window().add_event_listener_with_callback("offline", on_offline.as_ref().unchecked_ref());

    let end = loop {
        let next = rx.next();
        let timeout = Box::pin(sleep_ms(stale_ms));
        match futures_util::future::select(next, timeout).await {
            futures_util::future::Either::Left((Some(Msg::Event(ev)), _)) => {
                let ended = matches!(ev, ServerEvent::SessionEnded { .. });
                on_event(ev);
                if ended {
                    break StreamEnd::Ended;
                }
            }
            futures_util::future::Either::Left((Some(Msg::Offline), _)) => {
                break StreamEnd::Offline;
            }
            futures_util::future::Either::Left((Some(Msg::Error) | None, _)) => {
                break StreamEnd::Lost;
            }
            // Nothing heard for `stale_ms` (heartbeats every 15 s): the link is dead.
            futures_util::future::Either::Right(_) => break StreamEnd::Lost,
        }
    };
    es.close();
    es.set_onerror(None);
    let _ = window()
        .remove_event_listener_with_callback("offline", on_offline.as_ref().unchecked_ref());
    drop(keep);
    end
}

/// Waits `delay` ms, or less if the browser comes back online.
pub(crate) async fn wait_retry(delay: u64) {
    let (tx, mut rx) = futures_channel::mpsc::unbounded::<()>();
    let cb = Closure::<dyn FnMut(web_sys::Event)>::new(move |_| {
        let _ = tx.unbounded_send(());
    });
    let _ = window().add_event_listener_with_callback("online", cb.as_ref().unchecked_ref());
    let _ = futures_util::future::select(rx.next(), Box::pin(sleep_ms(delay))).await;
    let _ = window().remove_event_listener_with_callback("online", cb.as_ref().unchecked_ref());
}
