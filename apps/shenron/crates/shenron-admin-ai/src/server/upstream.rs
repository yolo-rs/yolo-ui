//! The Bun API (live) or the recorded payloads (fixture), behind one interface.

use super::fixture::FixtureUpstream;
use crate::gate::{Credentials, Me};
use crate::model::{CorpusStatus, Dashboard, SourceReport, SourceStatus};
use crate::protocol::{CockpitAction, Failure};
use serde_json::Value;
use std::time::{Duration, Instant};

/// An upstream answer passed through to the client: status and raw JSON body.
#[derive(Clone, Debug)]
pub struct Passthrough {
    pub status: u16,
    pub body: Vec<u8>,
}

impl Passthrough {
    pub fn json(status: u16, value: &Value) -> Self {
        Self { status, body: serde_json::to_vec(value).unwrap_or_else(|_| b"{}".to_vec()) }
    }

    pub fn ok(&self) -> bool {
        (200..300).contains(&self.status)
    }
}

pub enum Upstream {
    Live(LiveUpstream),
    Fixture(Box<FixtureUpstream>),
}

impl Upstream {
    pub async fn dashboard(&self, creds: &Credentials) -> Result<Dashboard, Failure> {
        match self {
            Upstream::Live(l) => l.dashboard(creds).await,
            Upstream::Fixture(f) => f.dashboard(),
        }
    }

    pub async fn corpus(&self, creds: &Credentials) -> CorpusStatus {
        match self {
            Upstream::Live(l) => l.corpus(creds).await,
            Upstream::Fixture(f) => f.corpus(),
        }
    }

    pub async fn me(&self, creds: &Credentials) -> Result<Me, Failure> {
        match self {
            Upstream::Live(l) => l.me(creds).await,
            Upstream::Fixture(f) => Ok(f.me(creds)),
        }
    }

    /// Status of a gated route, used to confirm token admins (not visible to `/api/me`).
    pub async fn gate_status(&self, creds: &Credentials) -> Result<u16, Failure> {
        match self {
            Upstream::Live(l) => l.gate_status(creds).await,
            Upstream::Fixture(f) => Ok(if f.me(creds).is_admin { 200 } else { 403 }),
        }
    }

    pub async fn action(
        &self,
        creds: &Credentials,
        action: &CockpitAction,
        actor: &str,
    ) -> Result<Passthrough, Failure> {
        match self {
            Upstream::Live(l) => l.action(creds, action).await,
            Upstream::Fixture(f) => Ok(f.action(action, actor)),
        }
    }

    pub async fn canon_get(&self, creds: &Credentials) -> Result<Passthrough, Failure> {
        match self {
            Upstream::Live(l) => l.canon_get(creds).await,
            Upstream::Fixture(f) => Ok(f.canon_get()),
        }
    }

    pub async fn canon_put(
        &self,
        creds: &Credentials,
        body: Vec<u8>,
    ) -> Result<Passthrough, Failure> {
        match self {
            Upstream::Live(l) => l.canon_put(creds, body).await,
            Upstream::Fixture(f) => Ok(f.canon_put(&body)),
        }
    }
}

/// Validates the base URL like `fetchAphrodyWebCrawlStats`: HTTPS, or HTTP on loopback, and no
/// credentials in the URL.
pub fn validate_base_url(raw: &str) -> Result<reqwest::Url, String> {
    let url = reqwest::Url::parse(raw.trim()).map_err(|e| format!("invalid URL: {e}"))?;
    let loopback = matches!(
        url.host_str(),
        Some("127.0.0.1" | "localhost" | "[::1]" | "::1")
    );
    if !(url.scheme() == "https" || (url.scheme() == "http" && loopback)) {
        return Err("must use HTTPS (or HTTP on loopback)".into());
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err("must not carry credentials".into());
    }
    Ok(url)
}

pub struct LiveUpstream {
    base: reqwest::Url,
    http: reqwest::Client,
}

fn error_message(body: &[u8]) -> String {
    serde_json::from_slice::<Value>(body)
        .ok()
        .and_then(|v| v.get("error").and_then(Value::as_str).map(str::to_owned))
        .unwrap_or_default()
}

fn network(e: reqwest::Error) -> Failure {
    Failure::network(if e.is_timeout() {
        "timeout".to_owned()
    } else {
        e.to_string()
    })
}

impl LiveUpstream {
    pub fn new(base: reqwest::Url) -> Self {
        let http = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .user_agent(concat!("shenron-admin-ai/", env!("CARGO_PKG_VERSION")))
            .build()
            .expect("HTTP client");
        Self { base, http }
    }

    fn url(&self, path: &str) -> reqwest::Url {
        self.base.join(path).expect("static path")
    }

    fn request(
        &self,
        method: reqwest::Method,
        path: &str,
        creds: &Credentials,
        timeout: Duration,
    ) -> reqwest::RequestBuilder {
        let mut rb = self
            .http
            .request(method, self.url(path))
            .timeout(timeout)
            .header("accept", "application/json")
            .header("cache-control", "no-store");
        for (k, v) in creds.headers() {
            rb = rb.header(k, v);
        }
        rb
    }

    async fn send(&self, rb: reqwest::RequestBuilder) -> Result<Passthrough, Failure> {
        let res = rb.send().await.map_err(network)?;
        let status = res.status().as_u16();
        let body = res.bytes().await.map_err(network)?.to_vec();
        Ok(Passthrough { status, body })
    }

    async fn dashboard(&self, creds: &Credentials) -> Result<Dashboard, Failure> {
        let p = self
            .send(self.request(
                reqwest::Method::GET,
                "/api/admin/ai",
                creds,
                Duration::from_secs(30),
            ))
            .await?;
        if !p.ok() {
            return Err(Failure::from_status(p.status, error_message(&p.body)));
        }
        serde_json::from_slice(&p.body)
            .map_err(|e| Failure::network(format!("invalid payload: {e}")))
    }

    async fn probe<T: serde::de::DeserializeOwned + Default>(
        &self,
        creds: &Credentials,
        path: &str,
        label: &str,
        pick: impl Fn(Value) -> Option<Value>,
    ) -> SourceReport<T> {
        let started = Instant::now();
        let mut report = SourceReport::<T> {
            label: label.into(),
            target: format!("GET {path}"),
            ..Default::default()
        };
        match self
            .send(self.request(reqwest::Method::GET, path, creds, Duration::from_secs(15)))
            .await
        {
            Ok(p) if p.ok() => {
                report.latency_ms = Some(started.elapsed().as_millis() as f64);
                match serde_json::from_slice::<Value>(&p.body)
                    .ok()
                    .and_then(&pick)
                    .map(serde_json::from_value::<T>)
                {
                    Some(Ok(data)) => {
                        report.status = SourceStatus::Ok;
                        report.data = Some(data);
                    }
                    _ => report.error = Some("invalid payload".into()),
                }
            }
            Ok(p) => {
                report.latency_ms = Some(started.elapsed().as_millis() as f64);
                let msg = error_message(&p.body);
                report.error = Some(format!(
                    "HTTP {}{}",
                    p.status,
                    if msg.is_empty() {
                        String::new()
                    } else {
                        format!(" : {msg}")
                    }
                ));
            }
            Err(f) => report.error = Some(f.message),
        }
        report
    }

    async fn corpus(&self, creds: &Credentials) -> CorpusStatus {
        let (transcription, pipeline) = tokio::join!(
            self.probe(
                creds,
                "/api/admin/transcriptions?mode=overview",
                "Transcriptions",
                Some
            ),
            self.probe(creds, "/api/admin/databooks/status", "Pipeline OCR", |v| v
                .get("local")
                .cloned()),
        );
        CorpusStatus { transcription, pipeline }
    }

    async fn me(&self, creds: &Credentials) -> Result<Me, Failure> {
        let p = self
            .send(self.request(
                reqwest::Method::GET,
                "/api/me",
                creds,
                Duration::from_secs(10),
            ))
            .await?;
        if !p.ok() {
            return Err(Failure::from_status(p.status, error_message(&p.body)));
        }
        serde_json::from_slice(&p.body)
            .map_err(|e| Failure::network(format!("invalid /api/me: {e}")))
    }

    async fn gate_status(&self, creds: &Credentials) -> Result<u16, Failure> {
        // `/api/canon-config` runs `isAdminOrToken` (session or admin token) like `requireIaAdmin`;
        // only its status is read, the body is dropped unread.
        let res = self
            .request(
                reqwest::Method::GET,
                "/api/canon-config",
                creds,
                Duration::from_secs(15),
            )
            .send()
            .await
            .map_err(network)?;
        Ok(res.status().as_u16())
    }

    async fn action(
        &self,
        creds: &Credentials,
        action: &CockpitAction,
    ) -> Result<Passthrough, Failure> {
        // evaluate_rag runs the whole gold set and test_prompt queries three engines.
        let timeout = match action {
            CockpitAction::EvaluateRag { .. } => Duration::from_secs(300),
            CockpitAction::OcrProbe { .. } => Duration::from_secs(90),
            _ => Duration::from_secs(45),
        };
        self.send(
            self.request(reqwest::Method::POST, "/api/admin/ai", creds, timeout)
                .json(action),
        )
        .await
    }

    async fn canon_get(&self, creds: &Credentials) -> Result<Passthrough, Failure> {
        self.send(self.request(
            reqwest::Method::GET,
            "/api/canon-config",
            creds,
            Duration::from_secs(30),
        ))
        .await
    }

    async fn canon_put(&self, creds: &Credentials, body: Vec<u8>) -> Result<Passthrough, Failure> {
        self.send(
            self.request(
                reqwest::Method::PUT,
                "/api/canon-config",
                creds,
                Duration::from_secs(60),
            )
            .header("content-type", "application/json")
            .body(body),
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_url_rules() {
        assert!(validate_base_url("https://dragonballfr.com").is_ok());
        assert!(validate_base_url("http://127.0.0.1:3000").is_ok());
        assert!(validate_base_url("http://dragonballfr.com").is_err());
        assert!(validate_base_url("https://user:pw@dragonballfr.com").is_err());
        assert!(validate_base_url("not a url").is_err());
    }

    #[test]
    fn error_message_reads_error_field() {
        assert_eq!(
            error_message(r#"{"error":"Accès réservé aux administrateurs."}"#.as_bytes()),
            "Accès réservé aux administrateurs."
        );
        assert_eq!(error_message(b"<html>"), "");
    }
}
