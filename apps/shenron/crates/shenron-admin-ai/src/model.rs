//! Data shapes of `GET /api/admin/ai` and its actions, mirrored from the Bun API
//! (`apps/site/src/lib/ia-cockpit.ts`, `app/api/admin/ai/route.ts`, `lib/canon-engine.ts`).
//!
//! Field names keep the upstream camelCase wire format. Every struct is lenient (`default`) so a
//! new or missing upstream field never blanks the whole cockpit; the source status carries the
//! truth, exactly like the React cockpit.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// What a source answered (`ok`), failed (`unavailable`) or could not be asked (`unconfigured`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum SourceStatus {
    Ok,
    #[default]
    Unavailable,
    Unconfigured,
}

/// One data source: status, target, latency, then either its data or its error.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SourceReport<T> {
    pub status: SourceStatus,
    pub label: String,
    pub target: String,
    pub latency_ms: Option<f64>,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T> Default for SourceReport<T> {
    fn default() -> Self {
        Self {
            status: SourceStatus::Unavailable,
            label: String::new(),
            target: String::new(),
            latency_ms: None,
            data: None,
            error: None,
        }
    }
}

impl<T> SourceReport<T> {
    /// Data only when the source answered.
    pub fn ok_data(&self) -> Option<&T> {
        match self.status {
            SourceStatus::Ok => self.data.as_ref(),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct RagBackendInfo {
    /// `aphrody` or `local`.
    pub value: String,
    pub raw: Option<String>,
    pub engine: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct AphrodyHealthData {
    pub rag: bool,
    pub llm: bool,
    pub ocr: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct KindCount {
    pub kind: String,
    pub count: u64,
    pub vectors: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct AphrodyRagStatsData {
    pub chunks: u64,
    pub vectors: u64,
    pub vectors_stored: Option<u64>,
    pub sources: u64,
    pub kinds: Vec<KindCount>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct RagProbeData {
    pub query: String,
    pub mode: String,
    pub hits: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct WebCrawlQueue {
    pub pending: u64,
    pub locked: u64,
    pub done: u64,
    pub failed: u64,
    pub total: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct WebCrawlData {
    pub queue: WebCrawlQueue,
    pub jobs: u64,
    pub running_jobs: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct BotRagData {
    pub chunks: u64,
    pub vectors: u64,
    pub mode: String,
    pub built_at: Option<String>,
    pub gateway_ready: Option<bool>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct EvalSnapshot {
    pub id: u64,
    pub created_at: String,
    pub cases_count: u64,
    pub recall1: f64,
    pub recall3: f64,
    pub recall5: f64,
    pub recall10: f64,
    pub mrr: f64,
    pub ndcg: f64,
    pub duration_ms: f64,
    pub sidecar_online: bool,
    pub misses_count: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct GoldCaseView {
    pub q: String,
    pub expected: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct GoldSetData {
    pub cases: u64,
    pub sample: Vec<GoldCaseView>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct FeedbackRow {
    pub id: u64,
    pub user_tag: Option<String>,
    #[serde(rename = "type")]
    pub kind: Option<String>,
    pub query: String,
    pub status: String,
    pub expected: Option<String>,
    pub comment: Option<String>,
    pub created_at: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct FeedbackData {
    pub total: u64,
    pub pending: u64,
    pub rows: Vec<FeedbackRow>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct GoalRow {
    pub id: u64,
    pub title: String,
    pub description: Option<String>,
    pub priority: Option<String>,
    pub status: String,
    pub created_at: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct GoalsData {
    pub active: u64,
    pub rows: Vec<GoalRow>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct LabMessage {
    pub id: String,
    pub author: String,
    pub content: String,
    pub timestamp: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct AgentKeySummary {
    pub id: String,
    pub name: String,
    pub scopes: Vec<String>,
    pub created_at: Option<String>,
}

/// The eleven sources of the cockpit, in the upstream `SOURCE_KEYS` order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default, Hash)]
#[serde(rename_all = "camelCase")]
pub enum SourceKey {
    #[default]
    AphrodyHealth,
    AphrodyRagStats,
    AphrodyRagProbe,
    AphrodyWeb,
    BotRag,
    EvalHistory,
    GoldSet,
    Feedbacks,
    Goals,
    LabMessages,
    AgentKeys,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Sources {
    pub aphrody_health: SourceReport<AphrodyHealthData>,
    pub aphrody_rag_stats: SourceReport<AphrodyRagStatsData>,
    pub aphrody_rag_probe: SourceReport<RagProbeData>,
    pub aphrody_web: SourceReport<WebCrawlData>,
    pub bot_rag: SourceReport<BotRagData>,
    pub eval_history: SourceReport<Vec<EvalSnapshot>>,
    pub gold_set: SourceReport<GoldSetData>,
    pub feedbacks: SourceReport<FeedbackData>,
    pub goals: SourceReport<GoalsData>,
    pub lab_messages: SourceReport<Vec<LabMessage>>,
    pub agent_keys: SourceReport<Vec<AgentKeySummary>>,
}

/// Status, label, target, latency and error of a source, without its typed payload.
#[derive(Clone, Debug, PartialEq)]
pub struct SourceMeta<'a> {
    pub status: SourceStatus,
    pub label: &'a str,
    pub target: &'a str,
    pub latency_ms: Option<f64>,
    pub error: Option<&'a str>,
}

fn meta<T>(r: &SourceReport<T>) -> SourceMeta<'_> {
    SourceMeta {
        status: r.status,
        label: &r.label,
        target: &r.target,
        latency_ms: r.latency_ms,
        error: r.error.as_deref(),
    }
}

impl Sources {
    pub fn meta(&self, key: SourceKey) -> SourceMeta<'_> {
        match key {
            SourceKey::AphrodyHealth => meta(&self.aphrody_health),
            SourceKey::AphrodyRagStats => meta(&self.aphrody_rag_stats),
            SourceKey::AphrodyRagProbe => meta(&self.aphrody_rag_probe),
            SourceKey::AphrodyWeb => meta(&self.aphrody_web),
            SourceKey::BotRag => meta(&self.bot_rag),
            SourceKey::EvalHistory => meta(&self.eval_history),
            SourceKey::GoldSet => meta(&self.gold_set),
            SourceKey::Feedbacks => meta(&self.feedbacks),
            SourceKey::Goals => meta(&self.goals),
            SourceKey::LabMessages => meta(&self.lab_messages),
            SourceKey::AgentKeys => meta(&self.agent_keys),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct FailingSource {
    pub key: SourceKey,
    pub label: String,
    pub status: SourceStatus,
    pub error: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Serving {
    pub engine: String,
    pub source: SourceKey,
    pub status: SourceStatus,
    pub error: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Summary {
    /// `ok` only when every source answered, else `degraded`.
    pub status: String,
    pub failing: Vec<FailingSource>,
    pub serving: Serving,
    pub documents: Option<u64>,
    pub vectors: Option<u64>,
    pub mode: Option<String>,
    pub counts_source: SourceKey,
    pub gold_cases: Option<u64>,
    pub latest_eval: Option<EvalSnapshot>,
    pub pending_feedbacks: Option<u64>,
    pub active_goals: Option<u64>,
}

/// `GET /api/admin/ai` minus the multi-megabyte `canonConfig` / `canonDefaults` documents, which
/// the hub drops before fanning out (the canon tab reads them on demand from `/api/canon-config`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Dashboard {
    pub backend: RagBackendInfo,
    pub sources: Sources,
    pub summary: Summary,
    /// Version of the saved doctrine (sha-256 hex of the row), used for optimistic concurrency.
    pub canon_expected: Option<String>,
    pub timestamp: String,
}

// ── Corpus tab (`/api/admin/transcriptions?mode=overview`, `/api/admin/databooks/status`) ──

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct CorpusDatabooks {
    pub transcribed_pages: Option<u64>,
    pub defective_pages: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct CorpusManga {
    pub ocr_pages: Option<u64>,
    pub ocr_with_text: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct CorpusTranscription {
    pub databooks: Option<CorpusDatabooks>,
    pub manga: Option<CorpusManga>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct PipelineState {
    /// A string, or an object carrying `status` / `state` (rendered like `LiveMetric`).
    pub state: Option<Value>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct CorpusPipeline {
    pub databooks: Option<PipelineState>,
    pub manga: Option<PipelineState>,
}

/// The two corpus probes, collected by the hub like the other sources (one report each).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct CorpusStatus {
    pub transcription: SourceReport<CorpusTranscription>,
    pub pipeline: SourceReport<CorpusPipeline>,
}

/// `LiveMetric` display rule: a string as is, an object by its `status` or `state`.
pub fn display_state(value: &Value) -> Option<String> {
    match value {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Object(o) => o
            .get("status")
            .or_else(|| o.get("state"))
            .and_then(|v| v.as_str().map(str::to_owned)),
        _ => None,
    }
}

// ── test_prompt ─────────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct RetrievalHit {
    pub title: String,
    pub snippet: String,
    pub url: Option<String>,
    pub score: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct RagSearchData {
    pub mode: String,
    pub results: Vec<RetrievalHit>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct MangaHit {
    pub series: String,
    /// String or number upstream.
    pub volume: Value,
    pub page: Value,
    pub excerpt: String,
    pub score: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Quote {
    pub text: String,
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub year: Option<i32>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct CanonVerdict {
    pub tier: String,
    pub tier_label: String,
    pub score: f64,
    pub verdict: String,
    pub explanation: String,
    pub nature: String,
    pub origin: Option<String>,
    pub toriyama_quote: Option<Quote>,
    pub sources: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct TestPromptResult {
    pub ok: bool,
    pub query: String,
    pub latency_ms: f64,
    pub backend: RagBackendInfo,
    pub rag: SourceReport<RagSearchData>,
    pub canon: Option<CanonVerdict>,
    pub manga: SourceReport<Vec<MangaHit>>,
}

// ── evaluate_rag ────────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct RetrievalMiss {
    pub q: String,
    pub expected: Vec<String>,
    pub returned_title: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Recall {
    #[serde(rename = "1")]
    pub at1: f64,
    #[serde(rename = "3")]
    pub at3: f64,
    #[serde(rename = "5")]
    pub at5: f64,
    #[serde(rename = "10")]
    pub at10: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct EvalResult {
    pub cases: u64,
    pub recall: Recall,
    pub mrr: f64,
    pub ndcg: f64,
    pub duration_ms: f64,
    pub misses: Vec<RetrievalMiss>,
    pub engine: String,
    pub persisted: bool,
}

// ── Canon doctrine (`/api/canon-config`) ────────────────────────────────────

/// Scores are integers upstream: write them back without a trailing `.0`.
fn whole<S: serde::Serializer>(v: &f64, s: S) -> Result<S::Ok, S::Error> {
    if v.fract() == 0.0 && v.abs() < 9.0e15 {
        s.serialize_i64(*v as i64)
    } else {
        s.serialize_f64(*v)
    }
}

/// The five tiers, in decreasing authority. Fixed vocabulary (the public page relies on it).
pub const CANON_TIERS: [&str; 5] = [
    "CANON_ABSOLU",
    "CANON_SUPERVISE",
    "EXTENSION_OFFICIELLE",
    "NON_CANON_DERIVE",
    "APOCRYPHE_FANON",
];

/// Entry categories of `CanonTopicEntry.category`.
pub const CANON_CATEGORIES: [&str; 5] = ["character", "form", "lore", "work", "myth"];

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct CanonTier {
    pub id: String,
    pub label: String,
    pub court: String,
    pub couleur: String,
    #[serde(serialize_with = "whole")]
    pub score: f64,
    pub definition: String,
    pub inclut: Vec<String>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct CanonEntry {
    pub id: String,
    pub title: String,
    pub category: String,
    pub tier: String,
    #[serde(serialize_with = "whole")]
    pub score: f64,
    pub keywords: Vec<String>,
    pub verdict: String,
    pub explanation: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub toriyama_quote: Option<Quote>,
    pub sources: Vec<String>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct CanonHeuristic {
    pub id: String,
    pub label: String,
    pub tier: String,
    #[serde(serialize_with = "whole")]
    pub score: f64,
    pub keywords: Vec<String>,
    pub verdict: String,
    pub explanation: String,
    pub sources: Vec<String>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct CanonReference {
    pub id: String,
    pub subject: String,
    pub keywords: Vec<String>,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_fr: Option<String>,
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub year: Option<i32>,
    pub document: String,
    pub lang: String,
    pub provenance: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub href: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visible: Option<bool>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct CanonFallback {
    pub tier: String,
    #[serde(serialize_with = "whole")]
    pub score: f64,
    pub verdict: String,
    pub explanation: String,
    pub sources: Vec<String>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct CanonEmpty {
    pub verdict: String,
    pub explanation: String,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// The editable doctrine document. Unknown fields (and `powerScaling`, edited as a whole) are kept
/// verbatim so a save never drops what this editor does not render.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct CanonConfig {
    pub version: u32,
    pub intro: String,
    pub placeholder: String,
    pub doctrine: Vec<String>,
    pub sources_admises: Vec<String>,
    pub sources_interdites: Vec<String>,
    pub tiers: Vec<CanonTier>,
    pub entries: Vec<CanonEntry>,
    pub heuristics: Vec<CanonHeuristic>,
    pub references: Vec<CanonReference>,
    pub power_scaling: Value,
    pub fallback: CanonFallback,
    pub vide: CanonEmpty,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// `GET /api/canon-config` → `{ config, expected, defaults }`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct CanonSnapshot {
    pub config: CanonConfig,
    pub expected: Option<String>,
    pub defaults: Option<CanonConfig>,
}

/// Visual tone of a tier (the React cockpit compared `tier` with `canon`/`retcon`, values the
/// engine never returns, so every verdict rendered red; this maps the real vocabulary).
pub fn tier_tone(tier: &str) -> &'static str {
    match tier {
        "CANON_ABSOLU" | "CANON_SUPERVISE" => "ok",
        "EXTENSION_OFFICIELLE" => "info",
        "NON_CANON_DERIVE" => "warn",
        _ => "danger",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dashboard_fixture_parses_and_drops_canon_documents() {
        let raw = include_str!("../fixtures/dashboard.json");
        let d: Dashboard = serde_json::from_str(raw).expect("fixture parses");
        assert_eq!(d.backend.value, "aphrody");
        assert_eq!(d.summary.status, "degraded");
        assert_eq!(d.sources.aphrody_web.status, SourceStatus::Unconfigured);
        assert!(d.sources.gold_set.ok_data().is_some());
        let back = serde_json::to_value(&d).unwrap();
        assert!(
            back.get("canonConfig").is_none(),
            "canon document never fans out"
        );
        assert!(back.get("canonExpected").is_some());
    }

    #[test]
    fn unknown_source_payload_does_not_blank_the_report() {
        let d: Dashboard = serde_json::from_str(
            r#"{"sources":{"botRag":{"status":"ok","label":"Bot","target":"GET /api/ia/status","latencyMs":12,"data":{"chunks":3,"vectors":2,"mode":"hybrid","newField":1},"error":null}}}"#,
        )
        .unwrap();
        assert_eq!(d.sources.bot_rag.ok_data().unwrap().chunks, 3);
        assert_eq!(d.sources.aphrody_health.status, SourceStatus::Unavailable);
    }

    #[test]
    fn canon_config_round_trips_unknown_fields() {
        let raw = include_str!("../fixtures/canon-config.json");
        let snap: CanonSnapshot = serde_json::from_str(raw).unwrap();
        let original: Value = serde_json::from_str::<Value>(raw).unwrap()["config"].clone();
        let again = serde_json::to_value(&snap.config).unwrap();
        assert_eq!(original, again, "a save must not drop or reshape any field");
        assert_eq!(snap.config.tiers.len(), 5);
    }

    #[test]
    fn display_state_follows_live_metric() {
        assert_eq!(
            display_state(&Value::String("idle".into())).as_deref(),
            Some("idle")
        );
        let obj: Value = serde_json::json!({"status": "running"});
        assert_eq!(display_state(&obj).as_deref(), Some("running"));
        assert_eq!(display_state(&Value::Null), None);
    }

    #[test]
    fn tier_tone_uses_engine_vocabulary() {
        assert_eq!(tier_tone("CANON_ABSOLU"), "ok");
        assert_eq!(tier_tone("NON_CANON_DERIVE"), "warn");
        assert_eq!(tier_tone("APOCRYPHE_FANON"), "danger");
    }
}
