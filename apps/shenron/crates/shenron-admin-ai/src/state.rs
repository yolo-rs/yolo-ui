//! Client state of the cockpit and its pure reducer: what the transport feeds in, what the UI
//! renders. No I/O, no timers; the web and desktop transports drive it identically.

use crate::backoff::Backoff;
use crate::model::{CorpusStatus, Dashboard, GoldCaseView};
use crate::protocol::{ActionKind, Failure, Mode, ServerEvent};

/// The five tabs, in the display order of the React cockpit.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Tab {
    Overview,
    Training,
    Canon,
    Corpus,
    Agents,
}

impl Tab {
    pub const ALL: [Tab; 5] = [
        Tab::Overview,
        Tab::Training,
        Tab::Canon,
        Tab::Corpus,
        Tab::Agents,
    ];

    pub fn slug(self) -> &'static str {
        match self {
            Tab::Overview => "overview",
            Tab::Training => "training",
            Tab::Canon => "canon",
            Tab::Corpus => "corpus",
            Tab::Agents => "agents",
        }
    }

    /// `?tab=` is user-controlled: anything unknown falls back (same rule as `parseCockpitTab`).
    pub fn parse(value: Option<&str>, fallback: Tab) -> Tab {
        Tab::ALL
            .into_iter()
            .find(|t| Some(t.slug()) == value)
            .unwrap_or(fallback)
    }

    /// Roving focus of the tablist: ArrowLeft/ArrowRight wrap, Home/End jump.
    pub fn step(self, key: &str) -> Option<Tab> {
        let i = Tab::ALL.iter().position(|t| *t == self)?;
        let n = Tab::ALL.len();
        let j = match key {
            "ArrowRight" | "ArrowDown" => (i + 1) % n,
            "ArrowLeft" | "ArrowUp" => (i + n - 1) % n,
            "Home" => 0,
            "End" => n - 1,
            _ => return None,
        };
        Some(Tab::ALL[j])
    }
}

/// Connection to the hub, as shown in the header.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Connection {
    /// Opening the stream (`attempt` consecutive failures so far).
    Connecting { attempt: u32 },
    /// Stream open and the hub said hello.
    Live,
    /// Stream lost; next try in `retry_in_ms`.
    Reconnecting { attempt: u32, retry_in_ms: u64 },
    /// The browser is offline, or the hub has been unreachable for too long (still retrying).
    Offline { attempt: u32, retry_in_ms: u64 },
    /// The hub ended the stream: session expired or admin role lost. No automatic retry.
    Ended,
}

impl Connection {
    /// CSS modifier and i18n suffix.
    pub fn slug(self) -> &'static str {
        match self {
            Connection::Connecting { .. } => "connecting",
            Connection::Live => "live",
            Connection::Reconnecting { .. } => "reconnecting",
            Connection::Offline { .. } => "offline",
            Connection::Ended => "ended",
        }
    }
}

/// What the transport reports.
#[derive(Clone, Debug, PartialEq)]
pub enum Input {
    Connecting,
    Event(ServerEvent),
    /// The stream failed or closed.
    Lost,
    /// The transport will retry after `delay_ms`.
    RetryScheduled {
        delay_ms: u64,
    },
    BrowserOffline,
    BrowserOnline,
}

/// One line of the activity feed.
#[derive(Clone, Debug, PartialEq)]
pub struct Activity {
    pub at: String,
    pub actor: String,
    pub action: ActionKind,
}

pub const ACTIVITY_CAP: usize = 8;

#[derive(Clone, Debug, PartialEq)]
pub struct CockpitState {
    pub connection: Connection,
    /// Consecutive transport failures since the last hello.
    pub failures: u32,
    pub boot: Option<String>,
    pub mode: Option<Mode>,
    pub actor: Option<String>,
    pub poll_ms: Option<u64>,
    pub dashboard: Option<Dashboard>,
    pub corpus: Option<CorpusStatus>,
    pub last_seq: u64,
    pub corpus_seq: u64,
    /// Hub time of the last applied snapshot.
    pub last_update: Option<String>,
    pub last_heartbeat: Option<String>,
    /// The last upstream read failed (data on screen is from `last_update`).
    pub upstream_failure: Option<Failure>,
    /// Set when the hub ended the stream.
    pub session: Option<Failure>,
    pub activity: Vec<Activity>,
    /// Bumped every time a snapshot is applied (lets views refresh derived state).
    pub revision: u64,
    pub backoff: Backoff,
}

impl Default for CockpitState {
    fn default() -> Self {
        Self {
            connection: Connection::Connecting { attempt: 0 },
            failures: 0,
            boot: None,
            mode: None,
            actor: None,
            poll_ms: None,
            dashboard: None,
            corpus: None,
            last_seq: 0,
            corpus_seq: 0,
            last_update: None,
            last_heartbeat: None,
            upstream_failure: None,
            session: None,
            activity: Vec::new(),
            revision: 0,
            backoff: Backoff::default(),
        }
    }
}

impl CockpitState {
    /// Applies one input. Returns `true` when visible data changed.
    pub fn apply(&mut self, input: Input) -> bool {
        match input {
            Input::Connecting => {
                if self.connection != Connection::Ended {
                    self.connection = Connection::Connecting { attempt: self.failures };
                }
                false
            }
            Input::Lost => {
                if self.connection != Connection::Ended {
                    self.failures = self.failures.saturating_add(1);
                }
                false
            }
            Input::RetryScheduled { delay_ms } => {
                if self.connection == Connection::Ended {
                    return false;
                }
                let attempt = self.failures;
                self.connection = if self.backoff.is_offline(attempt)
                    || matches!(self.connection, Connection::Offline { .. })
                {
                    Connection::Offline { attempt, retry_in_ms: delay_ms }
                } else {
                    Connection::Reconnecting { attempt, retry_in_ms: delay_ms }
                };
                false
            }
            Input::BrowserOffline => {
                if self.connection != Connection::Ended {
                    self.connection =
                        Connection::Offline { attempt: self.failures, retry_in_ms: 0 };
                }
                false
            }
            Input::BrowserOnline => {
                if matches!(self.connection, Connection::Offline { .. }) {
                    self.connection = Connection::Connecting { attempt: self.failures };
                }
                false
            }
            Input::Event(event) => self.on_event(event),
        }
    }

    fn on_event(&mut self, event: ServerEvent) -> bool {
        match event {
            ServerEvent::Hello { boot, mode, poll_ms, actor } => {
                if self.boot.as_deref() != Some(boot.as_str()) {
                    // New hub process: its sequence restarts.
                    self.last_seq = 0;
                    self.corpus_seq = 0;
                }
                self.boot = Some(boot);
                self.mode = Some(mode);
                self.poll_ms = Some(poll_ms);
                self.actor = Some(actor);
                self.failures = 0;
                self.session = None;
                self.connection = Connection::Live;
                false
            }
            ServerEvent::Snapshot { seq, at, dashboard } => {
                if seq <= self.last_seq {
                    return false;
                }
                self.last_seq = seq;
                self.dashboard = Some(*dashboard);
                self.last_update = Some(at);
                self.upstream_failure = None;
                self.revision += 1;
                true
            }
            ServerEvent::Corpus { seq, corpus, .. } => {
                if seq <= self.corpus_seq {
                    return false;
                }
                self.corpus_seq = seq;
                self.corpus = Some(*corpus);
                true
            }
            ServerEvent::UpstreamError { seq, failure, .. } => {
                if seq <= self.last_seq {
                    return false;
                }
                self.last_seq = seq;
                self.upstream_failure = Some(failure);
                true
            }
            ServerEvent::Activity { at, actor, action } => {
                self.activity.insert(0, Activity { at, actor, action });
                self.activity.truncate(ACTIVITY_CAP);
                true
            }
            ServerEvent::Heartbeat { at } => {
                self.last_heartbeat = Some(at);
                false
            }
            ServerEvent::SessionEnded { failure } => {
                self.session = Some(failure);
                self.connection = Connection::Ended;
                true
            }
        }
    }

    /// The transport keeps retrying unless the hub ended the session.
    pub fn should_retry(&self) -> bool {
        self.connection != Connection::Ended
    }
}

/// Gold-case filter of the training tab (question or any expected value, case-insensitive).
pub fn filter_gold<'a>(sample: &'a [GoldCaseView], filter: &str) -> Vec<&'a GoldCaseView> {
    let f = filter.trim().to_lowercase();
    if f.is_empty() {
        return sample.iter().collect();
    }
    sample
        .iter()
        .filter(|c| {
            c.q.to_lowercase().contains(&f)
                || c.expected.iter().any(|e| e.to_lowercase().contains(&f))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::FailureKind;

    fn hello(boot: &str) -> Input {
        Input::Event(ServerEvent::Hello {
            boot: boot.into(),
            mode: Mode::Fixture,
            poll_ms: 5000,
            actor: "Admin".into(),
        })
    }

    fn snap(seq: u64) -> Input {
        Input::Event(ServerEvent::Snapshot {
            seq,
            at: format!("t{seq}"),
            dashboard: Box::default(),
        })
    }

    #[test]
    fn tab_parse_and_keyboard_roving() {
        assert_eq!(Tab::parse(Some("canon"), Tab::Overview), Tab::Canon);
        assert_eq!(Tab::parse(Some("<script>"), Tab::Overview), Tab::Overview);
        assert_eq!(Tab::parse(None, Tab::Agents), Tab::Agents);
        assert_eq!(Tab::Overview.step("ArrowLeft"), Some(Tab::Agents));
        assert_eq!(Tab::Agents.step("ArrowRight"), Some(Tab::Overview));
        assert_eq!(Tab::Canon.step("Home"), Some(Tab::Overview));
        assert_eq!(Tab::Canon.step("End"), Some(Tab::Agents));
        assert_eq!(Tab::Canon.step("Enter"), None);
    }

    #[test]
    fn hello_goes_live_and_snapshots_apply_in_order() {
        let mut s = CockpitState::default();
        assert_eq!(s.connection, Connection::Connecting { attempt: 0 });
        s.apply(hello("a"));
        assert_eq!(s.connection, Connection::Live);
        assert!(s.apply(snap(2)));
        assert!(
            !s.apply(snap(1)),
            "older snapshot ignored (latest request wins)"
        );
        assert!(!s.apply(snap(2)), "duplicate ignored");
        assert_eq!(s.last_update.as_deref(), Some("t2"));
        assert!(s.apply(snap(3)));
        assert_eq!(s.revision, 2);
    }

    #[test]
    fn reconnect_backoff_then_offline_then_recovery() {
        let mut s = CockpitState::default();
        s.apply(hello("a"));
        s.apply(snap(5));
        for i in 1..=5 {
            s.apply(Input::Lost);
            s.apply(Input::RetryScheduled { delay_ms: 100 * i as u64 });
            assert_eq!(
                s.connection,
                Connection::Reconnecting { attempt: i, retry_in_ms: 100 * i as u64 }
            );
            s.apply(Input::Connecting);
            assert_eq!(s.connection, Connection::Connecting { attempt: i });
        }
        s.apply(Input::Lost);
        s.apply(Input::RetryScheduled { delay_ms: 30_000 });
        assert!(matches!(
            s.connection,
            Connection::Offline { attempt: 6, .. }
        ));
        assert!(
            s.dashboard.is_some(),
            "last data stays on screen while offline"
        );
        // Same hub process comes back: its sequence continues.
        s.apply(hello("a"));
        assert_eq!(s.connection, Connection::Live);
        assert_eq!(s.failures, 0);
        assert!(!s.apply(snap(5)));
        assert!(s.apply(snap(6)));
    }

    #[test]
    fn restarted_hub_resets_sequence() {
        let mut s = CockpitState::default();
        s.apply(hello("a"));
        s.apply(snap(40));
        s.apply(Input::Lost);
        s.apply(hello("b"));
        assert!(s.apply(snap(1)), "new boot id: its first snapshot applies");
    }

    #[test]
    fn browser_offline_and_online() {
        let mut s = CockpitState::default();
        s.apply(hello("a"));
        s.apply(Input::BrowserOffline);
        assert!(matches!(s.connection, Connection::Offline { .. }));
        s.apply(Input::BrowserOnline);
        assert!(matches!(s.connection, Connection::Connecting { .. }));
    }

    #[test]
    fn session_end_stops_retries() {
        let mut s = CockpitState::default();
        s.apply(hello("a"));
        s.apply(Input::Event(ServerEvent::SessionEnded {
            failure: Failure::from_status(401, ""),
        }));
        assert_eq!(s.connection, Connection::Ended);
        assert!(!s.should_retry());
        s.apply(Input::Lost);
        s.apply(Input::RetryScheduled { delay_ms: 5 });
        assert_eq!(s.connection, Connection::Ended);
        assert_eq!(
            s.session.as_ref().unwrap().kind,
            FailureKind::Unauthenticated
        );
    }

    #[test]
    fn upstream_error_keeps_data_and_clears_on_next_snapshot() {
        let mut s = CockpitState::default();
        s.apply(hello("a"));
        s.apply(snap(1));
        s.apply(Input::Event(ServerEvent::UpstreamError {
            seq: 2,
            at: "t".into(),
            failure: Failure::network("timeout"),
        }));
        assert!(s.upstream_failure.is_some());
        assert!(s.dashboard.is_some());
        s.apply(snap(3));
        assert!(s.upstream_failure.is_none());
    }

    #[test]
    fn activity_feed_is_capped_newest_first() {
        let mut s = CockpitState::default();
        for i in 0..12 {
            s.apply(Input::Event(ServerEvent::Activity {
                at: format!("{i}"),
                actor: "A".into(),
                action: ActionKind::CreateGoal,
            }));
        }
        assert_eq!(s.activity.len(), ACTIVITY_CAP);
        assert_eq!(s.activity[0].at, "11");
    }

    #[test]
    fn gold_filter_matches_question_or_expected() {
        let sample = vec![
            GoldCaseView { q: "Qui est Gine ?".into(), expected: vec!["/wiki/gine".into()] },
            GoldCaseView { q: "Broly 1993".into(), expected: vec!["Film".into()] },
        ];
        assert_eq!(filter_gold(&sample, "").len(), 2);
        assert_eq!(filter_gold(&sample, "GINE").len(), 1);
        assert_eq!(filter_gold(&sample, "film")[0].q, "Broly 1993");
    }
}
