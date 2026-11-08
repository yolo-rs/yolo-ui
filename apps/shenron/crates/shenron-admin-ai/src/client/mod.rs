//! Client transport: SSE subscription with reconnect backoff, JSON calls to the hub, and the few
//! platform facts the UI needs (locale, time zone, URL tab). The browser build uses
//! `EventSource` and `fetch`; the desktop build uses reqwest and the shared SSE parser.

use crate::backoff::{Backoff, Jitter};
use crate::protocol::{Failure, STALE_AFTER_MS, ServerEvent};
use crate::state::Input;
use serde::Serialize;
use serde_json::Value;

#[cfg(target_arch = "wasm32")]
mod web;
#[cfg(target_arch = "wasm32")]
use web as imp;

#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(not(target_arch = "wasm32"))]
use native as imp;

pub use imp::{
    initial_tab, lang, on_popstate, online, push_tab, random_seed, remember_focus, restore_focus,
    sleep_ms, store_theme_light, stored_theme_light, tz_offset_min,
};

/// Where the hub is and how to authenticate to it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Endpoint {
    /// Origin + base path, e.g. `""` + `/admin/ai` in the browser (same origin, cookies),
    /// `http://127.0.0.1:8787/admin/ai` on desktop.
    pub base: String,
    /// Bearer token (desktop; the browser relies on the session cookie).
    pub bearer: Option<String>,
}

impl Endpoint {
    pub fn detect() -> Endpoint {
        imp::endpoint()
    }

    pub fn url(&self, path: &str) -> String {
        format!("{}{}", self.base, path)
    }
}

/// A hub answer: HTTP status and JSON body (`{}` when unreadable).
#[derive(Clone, Debug, PartialEq)]
pub struct Reply {
    pub status: u16,
    pub body: Value,
}

impl Reply {
    pub fn ok(&self) -> bool {
        (200..300).contains(&self.status)
    }

    /// The server's `error` text, if any.
    pub fn error(&self) -> Option<String> {
        self.body
            .get("error")
            .and_then(Value::as_str)
            .map(str::to_owned)
    }

    pub fn notice(&self) -> Option<String> {
        self.body
            .get("notice")
            .and_then(Value::as_str)
            .map(str::to_owned)
    }

    pub fn failure(&self) -> Failure {
        Failure::from_status(self.status, self.error().unwrap_or_default())
    }
}

pub async fn get(ep: &Endpoint, path: &str) -> Result<Reply, Failure> {
    imp::request(ep, "GET", path, None).await
}

pub async fn post<T: Serialize>(ep: &Endpoint, path: &str, body: &T) -> Result<Reply, Failure> {
    let json = serde_json::to_string(body).map_err(|e| Failure::network(e.to_string()))?;
    imp::request(ep, "POST", path, Some(json)).await
}

pub async fn put<T: Serialize>(ep: &Endpoint, path: &str, body: &T) -> Result<Reply, Failure> {
    let json = serde_json::to_string(body).map_err(|e| Failure::network(e.to_string()))?;
    imp::request(ep, "PUT", path, Some(json)).await
}

/// What a stream attempt ended with.
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub(crate) enum StreamEnd {
    /// Dropped, errored, or stale: retry with backoff.
    Lost,
    /// The browser went offline.
    Offline,
    /// The hub ended the session (or refused it): stop.
    Ended,
}

/// Subscribes forever (until the session ends), feeding the reducer.
pub async fn run_stream(ep: Endpoint, mut dispatch: impl FnMut(Input) + 'static) {
    let backoff = Backoff::default();
    let mut jitter = Jitter::new(random_seed());
    let mut failures: u32 = 0;
    loop {
        dispatch(Input::Connecting);
        let mut heard_hello = false;
        let end = imp::stream_once(&ep, STALE_AFTER_MS, &mut |event: ServerEvent| {
            if matches!(event, ServerEvent::Hello { .. }) {
                heard_hello = true;
            }
            dispatch(Input::Event(event));
        })
        .await;
        if heard_hello {
            failures = 0;
        }
        match end {
            StreamEnd::Ended => return,
            StreamEnd::Offline => dispatch(Input::BrowserOffline),
            StreamEnd::Lost => {}
        }
        // A refused stream looks like a drop to EventSource: ask the gate what happened.
        if let Ok(reply) = get(&ep, "/session").await {
            if reply.status == 401 || reply.status == 403 {
                dispatch(Input::Event(ServerEvent::SessionEnded {
                    failure: reply.failure(),
                }));
                return;
            }
        }
        dispatch(Input::Lost);
        let delay = backoff.delay(failures, jitter.next_unit());
        failures = failures.saturating_add(1);
        dispatch(Input::RetryScheduled { delay_ms: delay });
        imp::wait_retry(delay).await;
        if imp::online() {
            dispatch(Input::BrowserOnline);
        }
    }
}
