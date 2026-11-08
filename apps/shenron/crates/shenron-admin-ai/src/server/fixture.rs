//! Fixture upstream: recorded payloads of the real API (`fixtures/*.json`), an in-memory copy
//! that the write actions mutate with the same semantics as the bot and the canon store, and a
//! small simulation (ingestion counters, latencies, bot messages) so live updates are visible.

use super::upstream::Passthrough;
use crate::gate::{Credentials, Me};
use crate::model::{
    CanonSnapshot, CorpusPipeline, CorpusStatus, CorpusTranscription, Dashboard, EvalResult,
    EvalSnapshot, GoalRow, GoldCaseView, LabMessage, SourceReport, SourceStatus,
};
use crate::protocol::{CockpitAction, Failure, FeedbackStatus};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::sync::Mutex;

const DASHBOARD: &str = include_str!("../../fixtures/dashboard.json");
const TEST_PROMPT: &str = include_str!("../../fixtures/test-prompt.json");
const EVAL: &str = include_str!("../../fixtures/eval.json");
const CORPUS: &str = include_str!("../../fixtures/corpus.json");
const CANON: &str = include_str!("../../fixtures/canon-config.json");

/// Same notice as the route: writes land in the bot SQLite, the site reads the PostgreSQL mirror.
const MIRROR_NOTICE: &str = "Enregistré par le bot : visible ici après la prochaine synchronisation du miroir PostgreSQL bot.*.";

/// Who the fixture session is (`SHENRON_ADMIN_AI_FIXTURE_ROLE`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FixtureRole {
    Admin,
    User,
    Anonymous,
}

impl FixtureRole {
    pub fn parse(raw: &str) -> FixtureRole {
        match raw.trim() {
            "user" => FixtureRole::User,
            "anonymous" | "anon" => FixtureRole::Anonymous,
            _ => FixtureRole::Admin,
        }
    }
}

struct State {
    dashboard: Dashboard,
    canon_config: Value,
    canon_defaults: Value,
    canon_expected: String,
    tick: u64,
    rng: u64,
    next_goal: u64,
    next_lab: u64,
    /// Every Nth read fails (simulated upstream outage); 0 = never.
    fail_every: u64,
}

pub struct FixtureUpstream {
    state: Mutex<State>,
    role: FixtureRole,
}

fn version_of(config: &Value) -> String {
    let mut h = Sha256::new();
    h.update(serde_json::to_vec(config).unwrap_or_default());
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

impl FixtureUpstream {
    pub fn new(role: FixtureRole, fail_every: u64) -> Self {
        let dashboard: Dashboard = serde_json::from_str(DASHBOARD).expect("fixture dashboard");
        let canon: Value = serde_json::from_str(CANON).expect("fixture canon");
        let canon_config = canon["config"].clone();
        let canon_expected = version_of(&canon_config);
        let mut dashboard = dashboard;
        dashboard.canon_expected = Some(canon_expected.clone());
        Self {
            state: Mutex::new(State {
                dashboard,
                canon_defaults: canon["defaults"].clone(),
                canon_config,
                canon_expected,
                tick: 0,
                rng: 0x9E37_79B9_7F4A_7C15,
                next_goal: 15,
                next_lab: 1_549_002,
                fail_every,
            }),
            role,
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|p| p.into_inner())
    }

    pub fn me(&self, creds: &Credentials) -> Me {
        let _ = creds;
        match self.role {
            FixtureRole::Admin => {
                Me { authenticated: true, username: Some("Admin (fixture)".into()), is_admin: true }
            }
            FixtureRole::User => {
                Me { authenticated: true, username: Some("Saiyan".into()), is_admin: false }
            }
            FixtureRole::Anonymous => Me::default(),
        }
    }

    /// One upstream read: advances the simulation, then answers the current state.
    pub fn dashboard(&self) -> Result<Dashboard, Failure> {
        let mut s = self.lock();
        s.tick += 1;
        if s.fail_every > 0 && s.tick.is_multiple_of(s.fail_every) {
            return Err(Failure::network("fixture: simulated upstream timeout"));
        }
        simulate(&mut s);
        Ok(s.dashboard.clone())
    }

    pub fn corpus(&self) -> CorpusStatus {
        let raw: Value = serde_json::from_str(CORPUS).expect("fixture corpus");
        let tick = self.lock().tick;
        let mut transcription: CorpusTranscription =
            serde_json::from_value(raw["transcription"].clone()).unwrap_or_default();
        if let Some(db) = transcription.databooks.as_mut() {
            db.transcribed_pages = db.transcribed_pages.map(|p| p + tick / 2);
        }
        let pipeline: CorpusPipeline =
            serde_json::from_value(raw["status"]["local"].clone()).unwrap_or_default();
        let ok = |label: &str, target: &str, ms: f64| (label.to_owned(), target.to_owned(), ms);
        let (l1, t1, m1) = ok(
            "Transcriptions",
            "GET /api/admin/transcriptions?mode=overview",
            23.0,
        );
        let (l2, t2, m2) = ok("Pipeline OCR", "GET /api/admin/databooks/status", 31.0);
        CorpusStatus {
            transcription: SourceReport {
                status: SourceStatus::Ok,
                label: l1,
                target: t1,
                latency_ms: Some(m1),
                data: Some(transcription),
                error: None,
            },
            pipeline: SourceReport {
                status: SourceStatus::Ok,
                label: l2,
                target: t2,
                latency_ms: Some(m2),
                data: Some(pipeline),
                error: None,
            },
        }
    }

    pub fn action(&self, action: &CockpitAction, actor: &str) -> Passthrough {
        if let Some(reason) = action.invalid_reason() {
            return Passthrough::json(
                400,
                &json!({ "error": format!("Paramètre invalide : {reason}.") }),
            );
        }
        let mut s = self.lock();
        match action {
            CockpitAction::TestPrompt { query } => {
                let mut v: Value = serde_json::from_str(TEST_PROMPT).expect("fixture test prompt");
                v["query"] = json!(query.trim());
                Passthrough::json(200, &v)
            }
            CockpitAction::OcrProbe { language, .. } => Passthrough::json(
                200,
                &json!({ "ok": true, "engine": "Aphrody AI", "language": language,
                         "result": { "text": "かめはめ波ーっ!!\n(fixture : texte OCR enregistré)" } }),
            ),
            CockpitAction::EvaluateRag { samples } => {
                let v: Value = serde_json::from_str(EVAL).expect("fixture eval");
                let mut metrics: EvalResult =
                    serde_json::from_value(v["metrics"].clone()).expect("eval");
                if *samples > 0 {
                    metrics.cases = u64::from(*samples);
                } else {
                    metrics.cases = s
                        .dashboard
                        .sources
                        .gold_set
                        .data
                        .as_ref()
                        .map_or(0, |g| g.cases);
                }
                let snap = EvalSnapshot {
                    id: s
                        .dashboard
                        .sources
                        .eval_history
                        .data
                        .as_ref()
                        .and_then(|h| h.first())
                        .map_or(1, |e| e.id + 1),
                    created_at: crate::format::now_iso(),
                    cases_count: metrics.cases,
                    recall1: metrics.recall.at1,
                    recall3: metrics.recall.at3,
                    recall5: metrics.recall.at5,
                    recall10: metrics.recall.at10,
                    mrr: metrics.mrr,
                    ndcg: metrics.ndcg,
                    duration_ms: metrics.duration_ms,
                    sidecar_online: true,
                    misses_count: metrics.misses.len() as u64,
                };
                if let Some(h) = s.dashboard.sources.eval_history.data.as_mut() {
                    h.insert(0, snap.clone());
                    h.truncate(20);
                }
                s.dashboard.summary.latest_eval = Some(snap);
                Passthrough::json(200, &json!({ "ok": true, "metrics": metrics }))
            }
            CockpitAction::AddGoldCase { q, expect } => {
                let expect: Vec<String> = expect
                    .iter()
                    .map(|e| e.trim().to_owned())
                    .filter(|e| !e.is_empty())
                    .collect();
                if let Some(g) = s.dashboard.sources.gold_set.data.as_mut() {
                    g.cases += 1;
                    g.sample.insert(
                        0,
                        GoldCaseView { q: q.trim().to_owned(), expected: expect.clone() },
                    );
                }
                s.dashboard.summary.gold_cases = s.dashboard.summary.gold_cases.map(|c| c + 1);
                Passthrough::json(
                    200,
                    &json!({ "ok": true, "q": q.trim(), "expect": expect, "notice": MIRROR_NOTICE }),
                )
            }
            CockpitAction::UpdateFeedback { id, status } => {
                let Some(f) = s.dashboard.sources.feedbacks.data.as_mut() else {
                    return Passthrough::json(
                        503,
                        &json!({ "ok": false, "error": "Retours indisponibles." }),
                    );
                };
                let Some(row) = f.rows.iter_mut().find(|r| r.id == *id) else {
                    return Passthrough::json(
                        404,
                        &json!({ "ok": false, "error": "Retour introuvable." }),
                    );
                };
                row.status = serde_json::to_value(status)
                    .ok()
                    .and_then(|v| v.as_str().map(str::to_owned))
                    .unwrap_or_default();
                f.pending = f.rows.iter().filter(|r| r.status == "pending").count() as u64;
                let pending = f.pending;
                s.dashboard.summary.pending_feedbacks = Some(pending);
                let _ = FeedbackStatus::Pending;
                Passthrough::json(
                    200,
                    &json!({ "ok": true, "id": id, "status": status, "notice": MIRROR_NOTICE }),
                )
            }
            CockpitAction::SendLabMessage { message } => {
                let id = s.next_lab;
                s.next_lab += 1;
                let msg = LabMessage {
                    id: id.to_string(),
                    author: "Shenron Bot".into(),
                    content: format!("[**{actor}** via Web Cockpit] : {}", message.trim()),
                    timestamp: Some(crate::format::now_iso()),
                };
                if let Some(m) = s.dashboard.sources.lab_messages.data.as_mut() {
                    m.insert(0, msg);
                    m.truncate(25);
                }
                Passthrough::json(200, &json!({ "ok": true, "id": id.to_string() }))
            }
            CockpitAction::CreateGoal { title, description } => {
                let id = s.next_goal;
                s.next_goal += 1;
                let description = if description.trim().is_empty() {
                    title.trim()
                } else {
                    description.trim()
                };
                let goal = GoalRow {
                    id,
                    title: title.trim().to_owned(),
                    description: Some(description.to_owned()),
                    priority: Some("medium".into()),
                    status: "pending".into(),
                    created_at: Some(crate::format::now_iso()),
                };
                if let Some(g) = s.dashboard.sources.goals.data.as_mut() {
                    g.rows.insert(0, goal.clone());
                    g.active += 1;
                }
                s.dashboard.summary.active_goals = s.dashboard.summary.active_goals.map(|a| a + 1);
                Passthrough::json(
                    200,
                    &json!({ "ok": true, "goal": goal, "notice": MIRROR_NOTICE }),
                )
            }
            CockpitAction::CompleteGoal { id, .. } => {
                let Some(g) = s.dashboard.sources.goals.data.as_mut() else {
                    return Passthrough::json(
                        503,
                        &json!({ "ok": false, "error": "Objectifs indisponibles." }),
                    );
                };
                let before = g.rows.len();
                g.rows.retain(|r| r.id != *id);
                if g.rows.len() == before {
                    return Passthrough::json(
                        404,
                        &json!({ "ok": false, "error": "Objectif introuvable." }),
                    );
                }
                g.active = g.active.saturating_sub(1);
                let active = g.active;
                s.dashboard.summary.active_goals = Some(active);
                Passthrough::json(
                    200,
                    &json!({ "ok": true, "id": id, "notice": MIRROR_NOTICE }),
                )
            }
        }
    }

    pub fn canon_get(&self) -> Passthrough {
        let s = self.lock();
        Passthrough::json(
            200,
            &json!({ "config": s.canon_config, "expected": s.canon_expected, "defaults": s.canon_defaults }),
        )
    }

    /// `saveCanonConfig` semantics: 428 without a valid snapshot, 409 when it changed since read.
    pub fn canon_put(&self, body: &[u8]) -> Passthrough {
        let Ok(input) = serde_json::from_slice::<Value>(body) else {
            return Passthrough::json(400, &json!({ "error": "JSON body requis" }));
        };
        let Some(config) = input.get("config").filter(|c| c.is_object()).cloned() else {
            return Passthrough::json(400, &json!({ "error": "Configuration document required" }));
        };
        let expected = input.get("expected").and_then(Value::as_str).unwrap_or("");
        if expected.len() != 64
            || !expected
                .bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        {
            return Passthrough::json(
                428,
                &json!({ "error": "A current configuration snapshot is required" }),
            );
        }
        let mut s = self.lock();
        if expected != s.canon_expected {
            return Passthrough::json(
                409,
                &json!({ "error": "Configuration changed since it was read" }),
            );
        }
        // Validate the shape the editor relies on before accepting it.
        if serde_json::from_value::<CanonSnapshot>(json!({ "config": config })).is_err() {
            return Passthrough::json(400, &json!({ "error": "Configuration document required" }));
        }
        s.canon_config = config;
        s.canon_expected = version_of(&s.canon_config);
        let exp = s.canon_expected.clone();
        s.dashboard.canon_expected = Some(exp.clone());
        Passthrough::json(
            200,
            &json!({ "ok": true, "config": s.canon_config, "expected": exp }),
        )
    }
}

fn next(rng: &mut u64) -> u64 {
    *rng ^= *rng << 13;
    *rng ^= *rng >> 7;
    *rng ^= *rng << 17;
    *rng
}

/// Ingestion progresses, latencies move, and the bot posts now and then.
fn simulate(s: &mut State) {
    let tick = s.tick;
    let mut rng = s.rng;
    let d = &mut s.dashboard;
    let jitter =
        |rng: &mut u64, base: f64| (base * (0.75 + (next(rng) % 50) as f64 / 100.0)).round();
    d.sources.aphrody_health.latency_ms = Some(jitter(&mut rng, 42.0));
    d.sources.aphrody_rag_stats.latency_ms = Some(jitter(&mut rng, 88.0));
    d.sources.aphrody_rag_probe.latency_ms = Some(jitter(&mut rng, 131.0));
    d.sources.bot_rag.latency_ms = Some(jitter(&mut rng, 17.0));
    d.sources.lab_messages.latency_ms = Some(jitter(&mut rng, 211.0));
    if let Some(stats) = d.sources.aphrody_rag_stats.data.as_mut() {
        let added = next(&mut rng) % 40;
        stats.chunks += added;
        stats.vectors += added.saturating_sub(next(&mut rng) % 3);
        if let Some(k) = stats.kinds.first_mut() {
            k.count += added;
        }
        d.summary.documents = Some(stats.chunks);
        d.summary.vectors = Some(stats.vectors);
    }
    if tick.is_multiple_of(4) {
        let id = s.next_lab;
        s.next_lab += 1;
        let docs = d.summary.documents.unwrap_or(0);
        if let Some(m) = d.sources.lab_messages.data.as_mut() {
            m.insert(
                0,
                LabMessage {
                    id: id.to_string(),
                    author: "Shenron Bot".into(),
                    content: format!(
                        "Ingestion continue : {docs} passages indexés (cycle {tick})."
                    ),
                    timestamp: Some(crate::format::now_iso()),
                },
            );
            m.truncate(25);
        }
    }
    d.timestamp = crate::format::now_iso();
    s.rng = rng;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn up() -> FixtureUpstream {
        FixtureUpstream::new(FixtureRole::Admin, 0)
    }

    #[test]
    fn reads_change_over_time() {
        let f = up();
        let a = f.dashboard().unwrap();
        let b = f.dashboard().unwrap();
        assert!(b.summary.documents >= a.summary.documents);
        assert!(a.canon_expected.is_some());
    }

    #[test]
    fn simulated_outage() {
        let f = FixtureUpstream::new(FixtureRole::Admin, 2);
        assert!(f.dashboard().is_ok());
        assert!(f.dashboard().is_err());
    }

    #[test]
    fn writes_keep_route_semantics() {
        let f = up();
        let r = f.action(
            &CockpitAction::AddGoldCase { q: "Q ?".into(), expect: vec!["/wiki/x".into()] },
            "A",
        );
        assert_eq!(r.status, 200);
        assert_eq!(f.dashboard().unwrap().summary.gold_cases, Some(149));

        let r = f.action(
            &CockpitAction::UpdateFeedback { id: 91, status: FeedbackStatus::Rejected },
            "A",
        );
        assert_eq!(r.status, 200);
        assert_eq!(f.dashboard().unwrap().summary.pending_feedbacks, Some(2));
        assert_eq!(
            f.action(
                &CockpitAction::UpdateFeedback { id: 999, status: FeedbackStatus::Rejected },
                "A"
            )
            .status,
            404
        );

        let r = f.action(
            &CockpitAction::CreateGoal { title: "T".into(), description: String::new() },
            "A",
        );
        let v: Value = serde_json::from_slice(&r.body).unwrap();
        assert_eq!(
            v["goal"]["description"], "T",
            "description defaults to the title"
        );
        let id = v["goal"]["id"].as_u64().unwrap();
        assert_eq!(
            f.action(&CockpitAction::CompleteGoal { id, notes: "ok".into() }, "A")
                .status,
            200
        );
        assert_eq!(
            f.action(&CockpitAction::TestPrompt { query: " ".into() }, "A")
                .status,
            400
        );

        let r = f.action(
            &CockpitAction::SendLabMessage { message: "salut".into() },
            "Goku",
        );
        assert_eq!(r.status, 200);
        let lab = f.dashboard().unwrap().sources.lab_messages.data.unwrap();
        assert!(
            lab.iter()
                .any(|m| m.content == "[**Goku** via Web Cockpit] : salut")
        );
    }

    #[test]
    fn canon_save_uses_optimistic_concurrency() {
        let f = up();
        let snap: Value = serde_json::from_slice(&f.canon_get().body).unwrap();
        let expected = snap["expected"].as_str().unwrap().to_owned();
        let mut config = snap["config"].clone();
        config["intro"] = json!("Nouvelle promesse");
        let missing = f.canon_put(&serde_json::to_vec(&json!({ "config": config })).unwrap());
        assert_eq!(missing.status, 428);
        let ok = f.canon_put(
            &serde_json::to_vec(&json!({ "config": config, "expected": expected })).unwrap(),
        );
        assert_eq!(ok.status, 200);
        let stale = f.canon_put(
            &serde_json::to_vec(&json!({ "config": config, "expected": expected })).unwrap(),
        );
        assert_eq!(
            stale.status, 409,
            "second save with the old version conflicts"
        );
        assert_ne!(
            f.dashboard().unwrap().canon_expected.as_deref(),
            Some(expected.as_str())
        );
    }

    #[test]
    fn roles() {
        assert!(
            FixtureUpstream::new(FixtureRole::User, 0)
                .me(&Credentials::default())
                .authenticated
        );
        assert!(
            !FixtureUpstream::new(FixtureRole::Anonymous, 0)
                .me(&Credentials::default())
                .authenticated
        );
        assert_eq!(FixtureRole::parse("anon"), FixtureRole::Anonymous);
    }
}
