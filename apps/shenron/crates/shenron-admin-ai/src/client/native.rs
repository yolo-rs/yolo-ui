//! Desktop transport: reqwest + the shared SSE parser. Hub URL, bearer token and language come
//! from the environment (`SHENRON_ADMIN_AI_URL`, `SHENRON_ADMIN_AI_TOKEN`, `SHENRON_LANG`).

use super::{Endpoint, Reply, StreamEnd};
use crate::i18n::Lang;
use crate::protocol::{BASE_PATH, Failure, ServerEvent};
use crate::sse::SseParser;
use futures_util::StreamExt;
use std::sync::OnceLock;
use std::time::Duration;

fn http() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .build()
            .expect("HTTP client")
    })
}

pub fn endpoint() -> Endpoint {
    let origin =
        std::env::var("SHENRON_ADMIN_AI_URL").unwrap_or_else(|_| "http://127.0.0.1:8787".into());
    let bearer = std::env::var("SHENRON_ADMIN_AI_TOKEN")
        .ok()
        .filter(|t| !t.trim().is_empty());
    Endpoint { base: format!("{}{BASE_PATH}", origin.trim_end_matches('/')), bearer }
}

pub async fn sleep_ms(ms: u64) {
    tokio::time::sleep(Duration::from_millis(ms)).await;
}

pub fn online() -> bool {
    true
}

pub fn random_seed() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(7)
}

/// The desktop shows UTC (times carry no zone database in this build).
pub fn tz_offset_min() -> i32 {
    0
}

pub fn lang() -> Lang {
    std::env::var("SHENRON_LANG")
        .ok()
        .and_then(|l| Lang::parse(&l))
        .unwrap_or_default()
}

pub fn initial_tab() -> Option<String> {
    std::env::var("SHENRON_ADMIN_AI_TAB").ok()
}

pub fn push_tab(_tab: &str) {}

pub fn on_popstate(_f: impl FnMut(Option<String>) + 'static) {}

pub fn stored_theme_light() -> bool {
    std::env::var("SHENRON_ADMIN_AI_THEME").ok().as_deref() == Some("light")
}

pub fn store_theme_light(_light: bool) {}

pub fn remember_focus() {}

pub fn restore_focus() {}

fn with_auth(ep: &Endpoint, rb: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    match &ep.bearer {
        Some(t) => rb.bearer_auth(t),
        None => rb,
    }
}

pub async fn request(
    ep: &Endpoint,
    method: &str,
    path: &str,
    body: Option<String>,
) -> Result<Reply, Failure> {
    let method = reqwest::Method::from_bytes(method.as_bytes())
        .map_err(|e| Failure::network(e.to_string()))?;
    let mut rb =
        with_auth(ep, http().request(method, ep.url(path))).header("accept", "application/json");
    if let Some(b) = body {
        rb = rb.header("content-type", "application/json").body(b);
    }
    let res = rb
        .send()
        .await
        .map_err(|e| Failure::network(e.to_string()))?;
    let status = res.status().as_u16();
    let bytes = res
        .bytes()
        .await
        .map_err(|e| Failure::network(e.to_string()))?;
    let body =
        serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Object(Default::default()));
    Ok(Reply { status, body })
}

pub(crate) async fn stream_once(
    ep: &Endpoint,
    stale_ms: u64,
    on_event: &mut dyn FnMut(ServerEvent),
) -> StreamEnd {
    let rb = with_auth(ep, http().get(ep.url("/events"))).header("accept", "text/event-stream");
    let Ok(res) = rb.send().await else {
        return StreamEnd::Lost;
    };
    if res.status().as_u16() == 401 || res.status().as_u16() == 403 {
        on_event(ServerEvent::SessionEnded {
            failure: Failure::from_status(res.status().as_u16(), ""),
        });
        return StreamEnd::Ended;
    }
    if !res.status().is_success() {
        return StreamEnd::Lost;
    }
    let mut body = res.bytes_stream();
    let mut parser = SseParser::default();
    loop {
        match tokio::time::timeout(Duration::from_millis(stale_ms), body.next()).await {
            Ok(Some(Ok(chunk))) => {
                for e in parser.push_bytes(&chunk) {
                    if let Ok(ev) = serde_json::from_str::<ServerEvent>(&e.data) {
                        let ended = matches!(ev, ServerEvent::SessionEnded { .. });
                        on_event(ev);
                        if ended {
                            return StreamEnd::Ended;
                        }
                    }
                }
            }
            _ => return StreamEnd::Lost,
        }
    }
}

pub(crate) async fn wait_retry(delay: u64) {
    sleep_ms(delay).await;
}
