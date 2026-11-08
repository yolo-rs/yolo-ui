//! Typed messages shared by the hub (server) and the clients (web, desktop).
//!
//! Server → client: Server-Sent Events on `GET {base}/events`; the SSE `event:` field is the
//! variant name and `data:` the JSON of [`ServerEvent`]. Client → server: `POST {base}/action`
//! with a [`CockpitAction`] body, which the hub forwards verbatim to `POST /api/admin/ai`.

use crate::model::{CorpusStatus, Dashboard};
use serde::{Deserialize, Serialize};

/// Base path of the cockpit, the same URL as the Next page it replaces.
pub const BASE_PATH: &str = "/admin/ai";
/// Heartbeat period of the hub; a client that hears nothing for [`STALE_AFTER_MS`] reconnects.
pub const HEARTBEAT_MS: u64 = 15_000;
pub const STALE_AFTER_MS: u64 = 45_000;

/// Where the hub reads its data from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    /// The Bun API (`/api/admin/ai*`), source of truth during the migration.
    Live,
    /// Recorded payloads with simulated changes; never touches production.
    Fixture,
}

/// Why a read or a write did not go through. Mirrors `CockpitFailureKind` of the React page.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FailureKind {
    /// No session (HTTP 401): ask for a new login.
    Unauthenticated,
    /// Session without the admin role (HTTP 403).
    Forbidden,
    /// Upstream answered an error status.
    Server,
    /// Upstream unreachable, timeout or unreadable body.
    Network,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Failure {
    pub kind: FailureKind,
    /// HTTP status when there was one.
    pub status: Option<u16>,
    /// Server message, verbatim (already localised upstream, or empty).
    pub message: String,
}

impl Failure {
    /// Same split as `describeHttpFailure`: 401 → unauthenticated, 403 → forbidden, else server.
    pub fn from_status(status: u16, message: impl Into<String>) -> Self {
        let kind = match status {
            401 => FailureKind::Unauthenticated,
            403 => FailureKind::Forbidden,
            _ => FailureKind::Server,
        };
        Self { kind, status: Some(status), message: message.into() }
    }

    pub fn network(message: impl Into<String>) -> Self {
        Self { kind: FailureKind::Network, status: None, message: message.into() }
    }
}

/// Kinds of write, for the activity feed broadcast to every connected admin.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionKind {
    TestPrompt,
    OcrProbe,
    EvaluateRag,
    AddGoldCase,
    UpdateFeedback,
    SendLabMessage,
    CreateGoal,
    CompleteGoal,
    SaveCanon,
}

/// One server event. `seq` is monotonic per hub process (`boot` identifies the process), so a
/// client applies a snapshot only if it is newer than the last one it saw.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerEvent {
    /// First event of every connection.
    Hello {
        boot: String,
        mode: Mode,
        poll_ms: u64,
        actor: String,
    },
    /// Full cockpit state (sent on connect, then after every upstream read).
    Snapshot {
        seq: u64,
        at: String,
        dashboard: Box<Dashboard>,
    },
    /// Corpus probes, read at a slower cadence.
    Corpus {
        seq: u64,
        at: String,
        corpus: Box<CorpusStatus>,
    },
    /// The last upstream read failed; the previous snapshot stays on screen.
    UpstreamError {
        seq: u64,
        at: String,
        failure: Failure,
    },
    /// Another admin (or this one) performed a write.
    Activity {
        at: String,
        actor: String,
        action: ActionKind,
    },
    /// Keeps the connection open through proxies and proves liveness.
    Heartbeat { at: String },
    /// The session expired or lost the admin role; the stream ends.
    SessionEnded { failure: Failure },
}

impl ServerEvent {
    /// SSE `event:` name.
    pub fn name(&self) -> &'static str {
        match self {
            ServerEvent::Hello { .. } => "hello",
            ServerEvent::Snapshot { .. } => "snapshot",
            ServerEvent::Corpus { .. } => "corpus",
            ServerEvent::UpstreamError { .. } => "upstream_error",
            ServerEvent::Activity { .. } => "activity",
            ServerEvent::Heartbeat { .. } => "heartbeat",
            ServerEvent::SessionEnded { .. } => "session_ended",
        }
    }

    /// Every SSE event name, for clients that must register listeners one by one.
    pub const NAMES: [&'static str; 7] = [
        "hello",
        "snapshot",
        "corpus",
        "upstream_error",
        "activity",
        "heartbeat",
        "session_ended",
    ];
}

/// Feedback arbitration values (`FEEDBACK_STATUSES` of `@aphrody/shenron-db`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeedbackStatus {
    Pending,
    GoldAdded,
    Rejected,
    Reviewed,
}

/// The write and probe actions of `POST /api/admin/ai`, with their exact wire fields.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum CockpitAction {
    TestPrompt {
        query: String,
    },
    OcrProbe {
        #[serde(rename = "imageBase64")]
        image_base64: String,
        language: String,
    },
    EvaluateRag {
        samples: u32,
    },
    AddGoldCase {
        q: String,
        expect: Vec<String>,
    },
    UpdateFeedback {
        id: u64,
        status: FeedbackStatus,
    },
    SendLabMessage {
        message: String,
    },
    CreateGoal {
        title: String,
        description: String,
    },
    CompleteGoal {
        id: u64,
        notes: String,
    },
}

impl CockpitAction {
    pub fn kind(&self) -> ActionKind {
        match self {
            CockpitAction::TestPrompt { .. } => ActionKind::TestPrompt,
            CockpitAction::OcrProbe { .. } => ActionKind::OcrProbe,
            CockpitAction::EvaluateRag { .. } => ActionKind::EvaluateRag,
            CockpitAction::AddGoldCase { .. } => ActionKind::AddGoldCase,
            CockpitAction::UpdateFeedback { .. } => ActionKind::UpdateFeedback,
            CockpitAction::SendLabMessage { .. } => ActionKind::SendLabMessage,
            CockpitAction::CreateGoal { .. } => ActionKind::CreateGoal,
            CockpitAction::CompleteGoal { .. } => ActionKind::CompleteGoal,
        }
    }

    /// Writes change upstream state: the hub re-reads and broadcasts after them.
    pub fn is_write(&self) -> bool {
        !matches!(
            self,
            CockpitAction::TestPrompt { .. } | CockpitAction::OcrProbe { .. }
        )
    }

    /// Client-side validation, same rules as the route (400 otherwise). `None` when valid.
    pub fn invalid_reason(&self) -> Option<&'static str> {
        let blank = |s: &str| s.trim().is_empty();
        match self {
            CockpitAction::TestPrompt { query } if blank(query) => Some("query"),
            CockpitAction::OcrProbe { image_base64, .. } if blank(image_base64) => Some("image"),
            CockpitAction::EvaluateRag { samples } if *samples > 1000 => Some("samples"),
            CockpitAction::AddGoldCase { q, expect }
                if blank(q)
                    || q.trim().chars().count() > 500
                    || expect.iter().all(|e| blank(e)) =>
            {
                Some("gold")
            }
            CockpitAction::UpdateFeedback { id, .. } if *id == 0 => Some("id"),
            CockpitAction::SendLabMessage { message } if blank(message) => Some("message"),
            CockpitAction::CreateGoal { title, description }
                if blank(title)
                    || title.trim().chars().count() > 200
                    || description.trim().chars().count() > 4000 =>
            {
                Some("goal")
            }
            CockpitAction::CompleteGoal { id, notes }
                if *id == 0 || notes.chars().count() > 4000 =>
            {
                Some("id")
            }
            _ => None,
        }
    }
}

/// Splits the comma-separated keyword field of the gold form, like the React handler.
pub fn split_keywords(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect()
}

/// Body of `PUT {base}/canon-config`, forwarded to `PUT /api/canon-config`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CanonSave {
    pub config: crate::model::CanonConfig,
    pub expected: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn actions_use_exact_wire_fields() {
        let a = CockpitAction::OcrProbe { image_base64: "AAA".into(), language: "ja".into() };
        assert_eq!(
            serde_json::to_value(&a).unwrap(),
            serde_json::json!({"action": "ocr_probe", "imageBase64": "AAA", "language": "ja"})
        );
        let f = CockpitAction::UpdateFeedback { id: 7, status: FeedbackStatus::GoldAdded };
        assert_eq!(
            serde_json::to_value(&f).unwrap(),
            serde_json::json!({"action": "update_feedback", "id": 7, "status": "gold_added"})
        );
        let g: CockpitAction =
            serde_json::from_str(r#"{"action":"complete_goal","id":3,"notes":"ok"}"#).unwrap();
        assert_eq!(g.kind(), ActionKind::CompleteGoal);
        assert!(g.is_write());
        assert!(!CockpitAction::TestPrompt { query: "x".into() }.is_write());
    }

    #[test]
    fn events_round_trip_with_their_sse_name() {
        let e = ServerEvent::Heartbeat { at: "2026-11-05T10:00:00Z".into() };
        let json = serde_json::to_string(&e).unwrap();
        assert!(json.contains("\"type\":\"heartbeat\""));
        let back: ServerEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(back, e);
        assert!(ServerEvent::NAMES.contains(&back.name()));
        let s = ServerEvent::UpstreamError {
            seq: 4,
            at: "t".into(),
            failure: Failure::from_status(403, "Accès réservé aux administrateurs."),
        };
        let back: ServerEvent = serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
        assert_eq!(back.name(), "upstream_error");
    }

    #[test]
    fn failure_split_matches_describe_http_failure() {
        assert_eq!(
            Failure::from_status(401, "").kind,
            FailureKind::Unauthenticated
        );
        assert_eq!(Failure::from_status(403, "").kind, FailureKind::Forbidden);
        assert_eq!(Failure::from_status(500, "").kind, FailureKind::Server);
        assert_eq!(Failure::network("down").kind, FailureKind::Network);
    }

    #[test]
    fn validation_mirrors_route_rules() {
        assert_eq!(
            CockpitAction::TestPrompt { query: "  ".into() }.invalid_reason(),
            Some("query")
        );
        assert_eq!(
            CockpitAction::AddGoldCase { q: "Q".into(), expect: vec![" ".into()] }.invalid_reason(),
            Some("gold")
        );
        assert_eq!(
            CockpitAction::AddGoldCase { q: "Q".into(), expect: vec!["a".into()] }.invalid_reason(),
            None
        );
        assert_eq!(
            CockpitAction::EvaluateRag { samples: 0 }.invalid_reason(),
            None
        );
        assert_eq!(
            CockpitAction::EvaluateRag { samples: 1001 }.invalid_reason(),
            Some("samples")
        );
        assert_eq!(
            CockpitAction::CreateGoal { title: "x".repeat(201), description: String::new() }
                .invalid_reason(),
            Some("goal")
        );
        assert_eq!(
            split_keywords(" a, ,b ,"),
            vec!["a".to_string(), "b".to_string()]
        );
    }
}
