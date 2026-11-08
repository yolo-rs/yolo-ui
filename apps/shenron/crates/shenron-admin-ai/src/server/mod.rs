//! Axum server of the cockpit (feature `server`).
//!
//! Routes, all under [`BASE_PATH`] (the URL of the Next page it replaces):
//! - `GET  /admin/ai`              HTML shell (gate: `requireAdmin` redirects)
//! - `GET  /admin/ai/events`       SSE stream of [`ServerEvent`] (gate: 401 / 403 JSON)
//! - `POST /admin/ai/action`       [`CockpitAction`] → `POST /api/admin/ai` with the admin's session
//! - `POST /admin/ai/refresh`      re-read upstream now and broadcast
//! - `GET  /admin/ai/session`      gate verdict only (lets a browser tell 401 / 403 from a drop)
//! - `GET  /admin/ai/canon-config` → `GET /api/canon-config`
//! - `PUT  /admin/ai/canon-config` → `PUT /api/canon-config` (optimistic concurrency, 409 / 428)
//! - `GET  /admin/ai/assets/app.css`, `/admin/ai/assets/boot.js`, `/admin/ai/pkg/*` static assets
//! - `GET  /admin/ai/healthz`

pub mod fixture;
pub mod hub;
pub mod upstream;

use crate::gate::{self, Access, Credentials};
use crate::i18n::{Lang, t};
use crate::protocol::{
    BASE_PATH, CockpitAction, Failure, FailureKind, HEARTBEAT_MS, Mode, ServerEvent,
};
use axum::Router;
use axum::body::Bytes;
use axum::extract::{DefaultBodyLimit, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::{get, post};
use fixture::{FixtureRole, FixtureUpstream};
use hub::{Frame, Hub};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::convert::Infallible;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use upstream::{LiveUpstream, Passthrough, Upstream};

/// Server configuration, read from the environment (never from a committed file).
#[derive(Clone, Debug)]
pub struct Config {
    pub mode: Mode,
    pub addr: String,
    /// `SHENRON_API_BASE_URL`, e.g. `https://dragonballfr.com` (live mode).
    pub base_url: Option<String>,
    /// `SHENRON_ADMIN_AI_UPSTREAM_TOKEN`: admin bearer for the shared poller (optional).
    pub upstream_token: Option<String>,
    pub poll_ms: u64,
    pub corpus_ms: u64,
    /// Directory holding the wasm-bindgen output (`shenron-admin-ai.js`, `_bg.wasm`).
    pub dist: PathBuf,
    pub cookie_prefixes: Vec<String>,
    pub fixture_role: FixtureRole,
    pub fixture_fail_every: u64,
}

impl Config {
    pub fn from_env() -> Result<Config, String> {
        let var = |k: &str| std::env::var(k).ok().filter(|v| !v.trim().is_empty());
        let default_mode = if cfg!(feature = "fixture") {
            Mode::Fixture
        } else {
            Mode::Live
        };
        let mode = match var("SHENRON_ADMIN_AI_MODE").as_deref() {
            Some("fixture") => Mode::Fixture,
            Some("live") => Mode::Live,
            Some(other) => return Err(format!("SHENRON_ADMIN_AI_MODE: unknown mode {other}")),
            None => default_mode,
        };
        let ms =
            |k: &str, d: u64, min: u64| var(k).and_then(|v| v.parse().ok()).unwrap_or(d).max(min);
        let base_url = var("SHENRON_API_BASE_URL");
        if mode == Mode::Live && base_url.is_none() {
            return Err("SHENRON_API_BASE_URL is required in live mode".into());
        }
        Ok(Config {
            mode,
            addr: var("SHENRON_ADMIN_AI_ADDR").unwrap_or_else(|| "127.0.0.1:8787".into()),
            base_url,
            upstream_token: var("SHENRON_ADMIN_AI_UPSTREAM_TOKEN"),
            poll_ms: ms("SHENRON_ADMIN_AI_POLL_MS", 5_000, 1_000),
            corpus_ms: ms("SHENRON_ADMIN_AI_CORPUS_MS", 60_000, 5_000),
            dist: var("SHENRON_ADMIN_AI_DIST")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("dist/pkg")),
            cookie_prefixes: var("SHENRON_ADMIN_AI_COOKIE_PREFIXES")
                .map(|v| {
                    v.split(',')
                        .map(|s| s.trim().to_owned())
                        .filter(|s| !s.is_empty())
                        .collect()
                })
                .unwrap_or_else(|| {
                    gate::DEFAULT_COOKIE_PREFIXES
                        .iter()
                        .map(|s| (*s).to_owned())
                        .collect()
                }),
            fixture_role: FixtureRole::parse(
                &var("SHENRON_ADMIN_AI_FIXTURE_ROLE").unwrap_or_default(),
            ),
            fixture_fail_every: var("SHENRON_ADMIN_AI_FIXTURE_FAIL_EVERY")
                .and_then(|v| v.parse().ok())
                .unwrap_or(0),
        })
    }
}

/// Gate decision cache entry (30 s), keyed by a hash of the credentials.
struct Decision {
    access: Access,
    actor: String,
    at: Instant,
}

pub struct App {
    pub hub: Arc<Hub>,
    pub config: Config,
    decisions: Mutex<HashMap<[u8; 32], Decision>>,
}

pub type Shared = Arc<App>;

const GATE_TTL: Duration = Duration::from_secs(30);
const REGATE_EVERY: Duration = Duration::from_secs(300);
const STYLESHEET: &str = crate::STYLESHEET;

impl App {
    pub fn new(config: Config) -> Result<Shared, String> {
        let upstream = match config.mode {
            Mode::Fixture => Upstream::Fixture(Box::new(FixtureUpstream::new(
                config.fixture_role,
                config.fixture_fail_every,
            ))),
            Mode::Live => {
                let url =
                    upstream::validate_base_url(config.base_url.as_deref().unwrap_or_default())
                        .map_err(|e| format!("SHENRON_API_BASE_URL {e}"))?;
                Upstream::Live(LiveUpstream::new(url))
            }
        };
        let service = config.upstream_token.as_ref().map(|t| Credentials {
            authorization: Some(format!("Bearer {}", t.trim())),
            ..Default::default()
        });
        let hub = Hub::new(
            config.mode,
            upstream,
            service,
            config.poll_ms,
            config.corpus_ms,
        );
        Ok(Arc::new(App {
            hub,
            config,
            decisions: Mutex::new(HashMap::new()),
        }))
    }

    fn credentials(&self, headers: &HeaderMap) -> Credentials {
        let prefixes: Vec<&str> = self
            .config
            .cookie_prefixes
            .iter()
            .map(String::as_str)
            .collect();
        Credentials::from_headers(
            headers
                .iter()
                .filter_map(|(k, v)| v.to_str().ok().map(|v| (k.as_str(), v))),
            &prefixes,
        )
    }

    /// `requireIaAdmin` through the Bun API, cached briefly per credential.
    pub async fn gate(&self, creds: &Credentials) -> Result<(Access, String), Failure> {
        if creds.is_empty() && self.config.mode == Mode::Live {
            return Ok((Access::Unauthenticated, String::new()));
        }
        let key: [u8; 32] = Sha256::digest(creds.cache_material().as_bytes()).into();
        if let Some(d) = self
            .decisions
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .get(&key)
        {
            if d.at.elapsed() < GATE_TTL {
                return Ok((d.access, d.actor.clone()));
            }
        }
        let me = self.hub.upstream().me(creds).await?;
        let mut access = gate::access_from_me(&me);
        let mut actor = me.username.clone().unwrap_or_else(|| "Admin".into());
        if access == Access::Unauthenticated && creds.is_token() {
            // Token admins have no session: confirm them on a gated route, like `isAdminOrToken`.
            let status = self.hub.upstream().gate_status(creds).await?;
            if let Some(a) = gate::access_from_status(status) {
                access = a;
                actor = "token-admin".into();
            }
        }
        let mut cache = self.decisions.lock().unwrap_or_else(|p| p.into_inner());
        cache.retain(|_, d| d.at.elapsed() < GATE_TTL);
        cache.insert(
            key,
            Decision { access, actor: actor.clone(), at: Instant::now() },
        );
        Ok((access, actor))
    }
}

fn lang_of(headers: &HeaderMap) -> Lang {
    headers
        .get(header::ACCEPT_LANGUAGE)
        .and_then(|v| v.to_str().ok())
        .map(Lang::from_accept_language)
        .unwrap_or_default()
}

const NO_STORE: (header::HeaderName, &str) =
    (header::CACHE_CONTROL, "private, no-store, max-age=0");

fn json_error(status: u16, message: &str, kind: Option<FailureKind>) -> Response {
    let body = serde_json::json!({ "ok": false, "error": message, "kind": kind });
    let mut res = (
        StatusCode::from_u16(status).unwrap_or(StatusCode::BAD_GATEWAY),
        [NO_STORE],
        axum::Json(body),
    )
        .into_response();
    if status == 401 {
        res.headers_mut()
            .insert(header::WWW_AUTHENTICATE, HeaderValue::from_static("Bearer"));
    }
    res
}

fn denial(lang: Lang, access: Access) -> Option<Response> {
    gate::api_denial(access).map(|(status, key)| {
        let kind = if status == 401 {
            FailureKind::Unauthenticated
        } else {
            FailureKind::Forbidden
        };
        json_error(status, t(lang, key), Some(kind))
    })
}

fn upstream_failure(lang: Lang, f: &Failure) -> Response {
    let msg = if f.message.is_empty() {
        t(lang, "adminAi.gate.upstream").to_owned()
    } else {
        f.message.clone()
    };
    json_error(f.status.unwrap_or(502), &msg, Some(f.kind))
}

/// Gate for API routes: `Ok((creds, actor))` or the denial response.
#[allow(clippy::result_large_err)]
async fn api_gate(app: &App, headers: &HeaderMap) -> Result<(Credentials, String), Response> {
    let lang = lang_of(headers);
    let creds = app.credentials(headers);
    match app.gate(&creds).await {
        Ok((Access::Admin, actor)) => Ok((creds, actor)),
        Ok((access, _)) => Err(denial(lang, access).expect("denied")),
        Err(f) => Err(upstream_failure(lang, &f)),
    }
}

/// Cross-site writes are refused: a browser `Origin` must match the `Host` it was sent to.
fn same_origin(headers: &HeaderMap) -> bool {
    let Some(origin) = headers.get(header::ORIGIN).and_then(|v| v.to_str().ok()) else {
        return true;
    };
    let host = headers
        .get(header::HOST)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    origin.split("://").nth(1).map(|o| o.trim_end_matches('/')) == Some(host)
}

fn passthrough(p: Passthrough) -> Response {
    (
        StatusCode::from_u16(p.status).unwrap_or(StatusCode::BAD_GATEWAY),
        [(header::CONTENT_TYPE, "application/json"), NO_STORE],
        p.body,
    )
        .into_response()
}

pub fn router(app: Shared) -> Router {
    let dist = tower_http::services::ServeDir::new(app.config.dist.clone());
    Router::new()
        .route(BASE_PATH, get(page))
        .route(&format!("{BASE_PATH}/"), get(page))
        .route(&format!("{BASE_PATH}/events"), get(events))
        .route(&format!("{BASE_PATH}/action"), post(action))
        .route(&format!("{BASE_PATH}/refresh"), post(refresh))
        .route(&format!("{BASE_PATH}/session"), get(session))
        .route(&format!("{BASE_PATH}/canon-config"), get(canon_get).put(canon_put))
        .route(&format!("{BASE_PATH}/assets/app.css"), get(stylesheet))
        .route(&format!("{BASE_PATH}/assets/boot.js"), get(boot))
        .route(&format!("{BASE_PATH}/healthz"), get(healthz))
        .nest_service(&format!("{BASE_PATH}/pkg"), dist)
        // The doctrine document with its quote corpus weighs a few megabytes (upstream cap 16 MiB).
        .layer(DefaultBodyLimit::max(16 * 1024 * 1024))
        .with_state(app)
}

/// The HTML shell: one mount point, the generated stylesheet and the wasm bootstrap.
pub fn shell(lang: Lang) -> String {
    let title = t(lang, "admin.pages.ai.title");
    let desc = t(lang, "admin.pages.ai.description");
    let loading = t(lang, "adminAi.state.loading");
    format!(
        "<!doctype html><html lang=\"{code}\" data-theme=\"dark\"><head><meta charset=\"utf-8\">\
<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\
<meta name=\"robots\" content=\"noindex, nofollow\"><meta name=\"description\" content=\"{desc}\">\
<title>{title}</title><link rel=\"stylesheet\" href=\"{BASE_PATH}/assets/app.css\">\
<link rel=\"modulepreload\" href=\"{BASE_PATH}/pkg/shenron-admin-ai.js\">\
<script type=\"module\" src=\"{BASE_PATH}/assets/boot.js\"></script></head>\
<body><div id=\"main\"><p class=\"sr-only\" role=\"status\">{loading}</p></div></body></html>",
        code = lang.code(),
    )
}

async fn page(State(app): State<Shared>, headers: HeaderMap, uri: axum::http::Uri) -> Response {
    let lang = uri
        .query()
        .and_then(|q| q.split('&').find_map(|kv| kv.strip_prefix("lang=")))
        .and_then(Lang::parse)
        .unwrap_or_else(|| lang_of(&headers));
    let creds = app.credentials(&headers);
    match app.gate(&creds).await {
        Ok((Access::Admin, _)) => (
            [
                (header::CONTENT_TYPE, "text/html; charset=utf-8"),
                NO_STORE,
                (header::CONTENT_SECURITY_POLICY, "default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; connect-src 'self'; base-uri 'none'; frame-ancestors 'none'; form-action 'self'"),
                (header::REFERRER_POLICY, "same-origin"),
                (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
            ],
            shell(lang),
        )
            .into_response(),
        Ok((access, _)) => Redirect::to(gate::page_redirect(access).unwrap_or("/")).into_response(),
        Err(f) => upstream_failure(lang, &f),
    }
}

async fn stylesheet() -> Response {
    (
        [
            (header::CONTENT_TYPE, "text/css; charset=utf-8"),
            (header::CACHE_CONTROL, "no-cache"),
        ],
        STYLESHEET,
    )
        .into_response()
}

async fn boot() -> Response {
    let js = format!("import init from \"{BASE_PATH}/pkg/shenron-admin-ai.js\";\ninit();\n");
    (
        [
            (header::CONTENT_TYPE, "text/javascript; charset=utf-8"),
            (header::CACHE_CONTROL, "no-cache"),
        ],
        js,
    )
        .into_response()
}

async fn healthz(State(app): State<Shared>) -> Response {
    axum::Json(serde_json::json!({ "ok": true, "mode": app.hub.mode, "subscribers": app.hub.subscribers() })).into_response()
}

async fn events(State(app): State<Shared>, headers: HeaderMap) -> Response {
    let (creds, actor) = match api_gate(&app, &headers).await {
        Ok(v) => v,
        Err(res) => return res,
    };
    let (tx, rx) = tokio::sync::mpsc::channel::<Frame>(32);
    let hub = app.hub.clone();
    let app2 = app.clone();
    tokio::spawn(async move {
        let mut brx = hub.subscribe();
        let conn = hub.join(creds.clone());
        let _ = tx.send(hub.hello(&actor)).await;
        match hub.latest() {
            (Some(snapshot), corpus) => {
                let _ = tx.send(snapshot).await;
                if let Some(c) = corpus {
                    let _ = tx.send(c).await;
                }
            }
            (None, _) => hub.refresh_now(),
        }
        let mut heartbeat = tokio::time::interval_at(
            tokio::time::Instant::now() + Duration::from_millis(HEARTBEAT_MS),
            Duration::from_millis(HEARTBEAT_MS),
        );
        let mut regate =
            tokio::time::interval_at(tokio::time::Instant::now() + REGATE_EVERY, REGATE_EVERY);
        loop {
            tokio::select! {
                msg = brx.recv() => match msg {
                    Ok(frame) => if tx.send(frame).await.is_err() { break },
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                        // Too slow: skip to the latest full state.
                        if let (Some(s), _) = hub.latest() {
                            if tx.send(s).await.is_err() { break }
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                },
                _ = heartbeat.tick() => {
                    let hb = Frame::of(&ServerEvent::Heartbeat { at: crate::format::now_iso() });
                    if tx.send(hb).await.is_err() { break }
                }
                _ = regate.tick() => {
                    let still = app2.gate(&creds).await;
                    if let Ok((access, _)) = still {
                        if access != Access::Admin {
                            let status = if access == Access::Unauthenticated { 401 } else { 403 };
                            let end = ServerEvent::SessionEnded { failure: Failure::from_status(status, "") };
                            let _ = tx.send(Frame::of(&end)).await;
                            break;
                        }
                    }
                }
                _ = tx.closed() => break,
            }
        }
        hub.leave(conn);
    });
    let stream = futures_util::stream::unfold(rx, |mut rx| async move {
        rx.recv().await.map(|f| {
            (
                Ok::<Event, Infallible>(Event::default().event(f.name).data(&*f.data)),
                rx,
            )
        })
    });
    let mut res = Sse::new(stream)
        .keep_alive(KeepAlive::default())
        .into_response();
    res.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("no-cache, no-transform"),
    );
    res.headers_mut()
        .insert("x-accel-buffering", HeaderValue::from_static("no"));
    res
}

async fn action(State(app): State<Shared>, headers: HeaderMap, body: Bytes) -> Response {
    let lang = lang_of(&headers);
    if !same_origin(&headers) {
        return json_error(
            403,
            t(lang, "adminAi.gate.crossOrigin"),
            Some(FailureKind::Forbidden),
        );
    }
    let (creds, actor) = match api_gate(&app, &headers).await {
        Ok(v) => v,
        Err(res) => return res,
    };
    let Ok(action) = serde_json::from_slice::<CockpitAction>(&body) else {
        return json_error(400, t(lang, "adminAi.gate.badAction"), None);
    };
    match app.hub.upstream().action(&creds, &action, &actor).await {
        Ok(p) => {
            if p.ok() && action.is_write() {
                app.hub.announce(&actor, action.kind());
                if matches!(action, CockpitAction::SendLabMessage { .. }) {
                    // Discord echoes the message after a short delay (the React page waited 800 ms).
                    let hub = app.hub.clone();
                    tokio::spawn(async move {
                        tokio::time::sleep(Duration::from_millis(800)).await;
                        hub.refresh_now();
                    });
                } else {
                    app.hub.refresh_now();
                }
            }
            passthrough(p)
        }
        Err(f) => upstream_failure(lang, &f),
    }
}

async fn refresh(State(app): State<Shared>, headers: HeaderMap) -> Response {
    let lang = lang_of(&headers);
    if !same_origin(&headers) {
        return json_error(
            403,
            t(lang, "adminAi.gate.crossOrigin"),
            Some(FailureKind::Forbidden),
        );
    }
    if let Err(res) = api_gate(&app, &headers).await {
        return res;
    }
    app.hub.refresh_now();
    (
        StatusCode::ACCEPTED,
        [NO_STORE],
        axum::Json(serde_json::json!({ "ok": true })),
    )
        .into_response()
}

async fn session(State(app): State<Shared>, headers: HeaderMap) -> Response {
    match api_gate(&app, &headers).await {
        Ok((_, actor)) => (
            [NO_STORE],
            axum::Json(serde_json::json!({ "ok": true, "actor": actor })),
        )
            .into_response(),
        Err(res) => res,
    }
}

async fn canon_get(State(app): State<Shared>, headers: HeaderMap) -> Response {
    let lang = lang_of(&headers);
    let (creds, _) = match api_gate(&app, &headers).await {
        Ok(v) => v,
        Err(res) => return res,
    };
    match app.hub.upstream().canon_get(&creds).await {
        Ok(p) => passthrough(p),
        Err(f) => upstream_failure(lang, &f),
    }
}

async fn canon_put(State(app): State<Shared>, headers: HeaderMap, body: Bytes) -> Response {
    let lang = lang_of(&headers);
    if !same_origin(&headers) {
        return json_error(
            403,
            t(lang, "adminAi.gate.crossOrigin"),
            Some(FailureKind::Forbidden),
        );
    }
    let (creds, actor) = match api_gate(&app, &headers).await {
        Ok(v) => v,
        Err(res) => return res,
    };
    match app.hub.upstream().canon_put(&creds, body.to_vec()).await {
        Ok(p) => {
            if p.ok() {
                app.hub
                    .announce(&actor, crate::protocol::ActionKind::SaveCanon);
                app.hub.refresh_now();
            }
            passthrough(p)
        }
        Err(f) => upstream_failure(lang, &f),
    }
}

/// Starts the pollers and serves until Ctrl-C.
pub async fn serve(config: Config) -> Result<(), String> {
    let addr = config.addr.clone();
    let app = App::new(config)?;
    tokio::spawn(app.hub.clone().run_poller());
    tokio::spawn(app.hub.clone().run_corpus_poller());
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .map_err(|e| format!("bind {addr}: {e}"))?;
    eprintln!(
        "shenron-admin-ai: {:?} mode on http://{addr}{BASE_PATH}",
        app.hub.mode
    );
    axum::serve(listener, router(app))
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    fn config(role: FixtureRole) -> Config {
        Config {
            mode: Mode::Fixture,
            addr: "127.0.0.1:0".into(),
            base_url: None,
            upstream_token: None,
            poll_ms: 50,
            corpus_ms: 5_000,
            dist: PathBuf::from("dist/pkg"),
            cookie_prefixes: vec!["better-auth.".into()],
            fixture_role: role,
            fixture_fail_every: 0,
        }
    }

    async fn call(app: Shared, req: Request<Body>) -> (StatusCode, HeaderMap, String) {
        let res = router(app).oneshot(req).await.unwrap();
        let status = res.status();
        let headers = res.headers().clone();
        let body = axum::body::to_bytes(res.into_body(), 1 << 22)
            .await
            .unwrap();
        (status, headers, String::from_utf8_lossy(&body).into_owned())
    }

    #[tokio::test]
    async fn page_redirects_like_require_admin() {
        let anon = App::new(config(FixtureRole::Anonymous)).unwrap();
        let (s, h, _) = call(anon, Request::get(BASE_PATH).body(Body::empty()).unwrap()).await;
        assert_eq!(s, StatusCode::SEE_OTHER);
        assert_eq!(h[header::LOCATION], "/signin?callbackURL=%2Fadmin");

        let user = App::new(config(FixtureRole::User)).unwrap();
        let (s, h, _) = call(user, Request::get(BASE_PATH).body(Body::empty()).unwrap()).await;
        assert_eq!(s, StatusCode::SEE_OTHER);
        assert_eq!(h[header::LOCATION], "/");

        let admin = App::new(config(FixtureRole::Admin)).unwrap();
        let (s, h, body) = call(
            admin,
            Request::get(format!("{BASE_PATH}?lang=ja"))
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(s, StatusCode::OK);
        assert!(body.contains("lang=\"ja\""));
        assert!(
            h[header::CONTENT_SECURITY_POLICY]
                .to_str()
                .unwrap()
                .contains("frame-ancestors 'none'")
        );
    }

    #[tokio::test]
    async fn api_answers_401_and_403_like_require_ia_admin() {
        let anon = App::new(config(FixtureRole::Anonymous)).unwrap();
        let (s, h, body) = call(
            anon,
            Request::get(format!("{BASE_PATH}/events"))
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(s, StatusCode::UNAUTHORIZED);
        assert_eq!(h[header::WWW_AUTHENTICATE], "Bearer");
        assert!(body.contains("unauthenticated"));
        let user = App::new(config(FixtureRole::User)).unwrap();
        let req = Request::post(format!("{BASE_PATH}/action"))
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"action":"create_goal","title":"x","description":""}"#,
            ))
            .unwrap();
        let (s, _, _) = call(user, req).await;
        assert_eq!(s, StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn actions_forward_and_reject_cross_origin() {
        let app = App::new(config(FixtureRole::Admin)).unwrap();
        let mut rx = app.hub.subscribe();
        let req = Request::post(format!("{BASE_PATH}/action"))
            .header("host", "localhost:8787")
            .header("origin", "http://localhost:8787")
            .body(Body::from(
                r#"{"action":"create_goal","title":"Nouveau","description":""}"#,
            ))
            .unwrap();
        let (s, _, body) = call(app.clone(), req).await;
        assert_eq!(s, StatusCode::OK, "{body}");
        assert!(body.contains("\"notice\""));
        let f = rx.recv().await.unwrap();
        assert_eq!(f.name, "activity");

        let req = Request::post(format!("{BASE_PATH}/action"))
            .header("host", "localhost:8787")
            .header("origin", "https://evil.example")
            .body(Body::from(
                r#"{"action":"create_goal","title":"x","description":""}"#,
            ))
            .unwrap();
        let (s, _, _) = call(app.clone(), req).await;
        assert_eq!(s, StatusCode::FORBIDDEN);

        let req = Request::post(format!("{BASE_PATH}/action"))
            .body(Body::from("{\"action\":\"drop_tables\"}"))
            .unwrap();
        let (s, _, _) = call(app, req).await;
        assert_eq!(s, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn sse_stream_starts_with_hello_then_snapshot() {
        let app = App::new(config(FixtureRole::Admin)).unwrap();
        tokio::spawn(app.hub.clone().run_poller());
        let res = router(app)
            .oneshot(
                Request::get(format!("{BASE_PATH}/events"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(res.headers()[header::CONTENT_TYPE], "text/event-stream");
        let mut body = res.into_body().into_data_stream();
        use futures_util::StreamExt;
        let mut text = String::new();
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        while !text.contains("event: snapshot") && tokio::time::Instant::now() < deadline {
            if let Ok(Some(Ok(chunk))) =
                tokio::time::timeout(Duration::from_secs(2), body.next()).await
            {
                text.push_str(&String::from_utf8_lossy(&chunk));
            }
        }
        let hello = text.find("event: hello").expect("hello first");
        let snap = text.find("event: snapshot").expect("then a snapshot");
        assert!(hello < snap);
    }

    #[tokio::test]
    async fn canon_proxy_keeps_conflict_statuses() {
        let app = App::new(config(FixtureRole::Admin)).unwrap();
        let (s, _, body) = call(
            app.clone(),
            Request::get(format!("{BASE_PATH}/canon-config"))
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(s, StatusCode::OK);
        let v: serde_json::Value = serde_json::from_str(&body).unwrap();
        let put = serde_json::json!({ "config": v["config"], "expected": "0".repeat(64) });
        let req = Request::put(format!("{BASE_PATH}/canon-config"))
            .body(Body::from(put.to_string()))
            .unwrap();
        let (s, _, _) = call(app, req).await;
        assert_eq!(s, StatusCode::CONFLICT);
    }

    #[test]
    fn shell_has_no_inline_script() {
        let html = shell(Lang::Fr);
        assert!(html.contains("<html lang=\"fr\""));
        assert!(
            !html.contains("<script>"),
            "bootstrap is an external module (CSP script-src 'self')"
        );
    }
}
