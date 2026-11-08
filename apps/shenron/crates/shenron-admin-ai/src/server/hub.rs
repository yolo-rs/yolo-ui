//! The hub: one upstream poller for every connected admin, fanned out over SSE.
//!
//! The poller sleeps while nobody is connected, reads `/api/admin/ai` every `poll_ms` (and on
//! demand after a write or a manual refresh), and broadcasts a pre-serialised frame; clients
//! never poll. Corpus probes run at a slower cadence. Frames carry a monotonic `seq` and the
//! process `boot` id so clients drop stale or duplicate snapshots.

use super::upstream::Upstream;
use crate::format::now_iso;
use crate::gate::Credentials;
use crate::protocol::{ActionKind, Failure, FailureKind, Mode, ServerEvent};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;
use tokio::sync::{Notify, broadcast};

/// A serialised server event, ready for every subscriber.
#[derive(Clone, Debug)]
pub struct Frame {
    pub name: &'static str,
    pub data: Arc<str>,
}

impl Frame {
    pub fn of(event: &ServerEvent) -> Frame {
        Frame {
            name: event.name(),
            data: Arc::from(serde_json::to_string(event).expect("events serialise")),
        }
    }
}

pub struct Hub {
    pub mode: Mode,
    pub boot: String,
    pub poll_ms: u64,
    pub corpus_ms: u64,
    upstream: Upstream,
    /// Service credential for the poller (`SHENRON_ADMIN_AI_UPSTREAM_TOKEN`), if any.
    service: Option<Credentials>,
    /// Credentials of connected admins, by connection id: the poller borrows one when there is no
    /// service token, and drops it when upstream refuses it.
    borrowed: Mutex<BTreeMap<u64, Credentials>>,
    tx: broadcast::Sender<Frame>,
    latest: RwLock<Option<Frame>>,
    latest_corpus: RwLock<Option<Frame>>,
    seq: AtomicU64,
    corpus_seq: AtomicU64,
    subscribers: AtomicUsize,
    next_conn: AtomicU64,
    wake: Notify,
    refresh: Notify,
    corpus_refresh: Notify,
}

impl Hub {
    pub fn new(
        mode: Mode,
        upstream: Upstream,
        service: Option<Credentials>,
        poll_ms: u64,
        corpus_ms: u64,
    ) -> Arc<Hub> {
        let (tx, _) = broadcast::channel(64);
        let boot = format!(
            "{:x}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        );
        Arc::new(Hub {
            mode,
            boot,
            poll_ms,
            corpus_ms,
            upstream,
            service,
            borrowed: Mutex::new(BTreeMap::new()),
            tx,
            latest: RwLock::new(None),
            latest_corpus: RwLock::new(None),
            seq: AtomicU64::new(0),
            corpus_seq: AtomicU64::new(0),
            subscribers: AtomicUsize::new(0),
            next_conn: AtomicU64::new(1),
            wake: Notify::new(),
            refresh: Notify::new(),
            corpus_refresh: Notify::new(),
        })
    }

    pub fn upstream(&self) -> &Upstream {
        &self.upstream
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Frame> {
        self.tx.subscribe()
    }

    pub fn latest(&self) -> (Option<Frame>, Option<Frame>) {
        (
            self.latest
                .read()
                .unwrap_or_else(|p| p.into_inner())
                .clone(),
            self.latest_corpus
                .read()
                .unwrap_or_else(|p| p.into_inner())
                .clone(),
        )
    }

    pub fn subscribers(&self) -> usize {
        self.subscribers.load(Ordering::SeqCst)
    }

    /// Registers a connection; its credentials become available to the poller.
    pub fn join(&self, creds: Credentials) -> u64 {
        let id = self.next_conn.fetch_add(1, Ordering::SeqCst);
        self.borrowed
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .insert(id, creds);
        self.subscribers.fetch_add(1, Ordering::SeqCst);
        self.wake.notify_one();
        id
    }

    pub fn leave(&self, id: u64) {
        self.borrowed
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .remove(&id);
        self.subscribers.fetch_sub(1, Ordering::SeqCst);
    }

    /// Re-read upstream now (after a write, or the manual refresh button).
    pub fn refresh_now(&self) {
        self.refresh.notify_one();
        self.corpus_refresh.notify_one();
    }

    pub fn announce(&self, actor: &str, action: ActionKind) {
        let _ = self.tx.send(Frame::of(&ServerEvent::Activity {
            at: now_iso(),
            actor: actor.to_owned(),
            action,
        }));
    }

    pub fn hello(&self, actor: &str) -> Frame {
        Frame::of(&ServerEvent::Hello {
            boot: self.boot.clone(),
            mode: self.mode,
            poll_ms: self.poll_ms,
            actor: actor.to_owned(),
        })
    }

    fn poll_credentials(&self) -> Option<(Option<u64>, Credentials)> {
        if self.mode == Mode::Fixture {
            return Some((None, Credentials::default()));
        }
        if let Some(s) = &self.service {
            return Some((None, s.clone()));
        }
        self.borrowed
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .iter()
            .next_back()
            .map(|(id, c)| (Some(*id), c.clone()))
    }

    fn publish(&self, event: ServerEvent, corpus: bool) {
        let frame = Frame::of(&event);
        let slot = if corpus {
            &self.latest_corpus
        } else {
            &self.latest
        };
        let keep = !matches!(event, ServerEvent::UpstreamError { .. });
        if keep {
            *slot.write().unwrap_or_else(|p| p.into_inner()) = Some(frame.clone());
        }
        let _ = self.tx.send(frame);
    }

    /// One dashboard read and broadcast. Returns false when no credential could be used.
    pub async fn poll_once(&self) -> bool {
        // Try the borrowed credentials one after the other: an expired one is dropped.
        for _ in 0..4 {
            let Some((conn, creds)) = self.poll_credentials() else {
                return false;
            };
            match self.upstream.dashboard(&creds).await {
                Ok(dashboard) => {
                    let seq = self.seq.fetch_add(1, Ordering::SeqCst) + 1;
                    self.publish(
                        ServerEvent::Snapshot {
                            seq,
                            at: now_iso(),
                            dashboard: Box::new(dashboard),
                        },
                        false,
                    );
                    return true;
                }
                Err(f)
                    if matches!(
                        f.kind,
                        FailureKind::Unauthenticated | FailureKind::Forbidden
                    ) && conn.is_some() =>
                {
                    self.borrowed
                        .lock()
                        .unwrap_or_else(|p| p.into_inner())
                        .remove(&conn.unwrap_or_default());
                    continue;
                }
                Err(failure) => {
                    self.publish_error(failure);
                    return true;
                }
            }
        }
        false
    }

    fn publish_error(&self, failure: Failure) {
        let seq = self.seq.fetch_add(1, Ordering::SeqCst) + 1;
        self.publish(
            ServerEvent::UpstreamError { seq, at: now_iso(), failure },
            false,
        );
    }

    pub async fn poll_corpus(&self) {
        let Some((_, creds)) = self.poll_credentials() else {
            return;
        };
        let corpus = self.upstream.corpus(&creds).await;
        let seq = self.corpus_seq.fetch_add(1, Ordering::SeqCst) + 1;
        self.publish(
            ServerEvent::Corpus { seq, at: now_iso(), corpus: Box::new(corpus) },
            true,
        );
    }

    /// The dashboard poller. Runs forever; idle while nobody is connected.
    pub async fn run_poller(self: Arc<Self>) {
        loop {
            while self.subscribers() == 0 {
                self.wake.notified().await;
            }
            if !self.poll_once().await {
                // Live mode, no service token and no admin credential: wait for a connection.
                tokio::select! {
                    _ = self.wake.notified() => {}
                    _ = tokio::time::sleep(Duration::from_millis(self.poll_ms)) => {}
                }
                continue;
            }
            tokio::select! {
                _ = tokio::time::sleep(Duration::from_millis(self.poll_ms)) => {}
                _ = self.refresh.notified() => {}
            }
        }
    }

    pub async fn run_corpus_poller(self: Arc<Self>) {
        loop {
            while self.subscribers() == 0 {
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
            self.poll_corpus().await;
            tokio::select! {
                _ = tokio::time::sleep(Duration::from_millis(self.corpus_ms)) => {}
                _ = self.corpus_refresh.notified() => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::fixture::{FixtureRole, FixtureUpstream};

    fn hub() -> Arc<Hub> {
        Hub::new(
            Mode::Fixture,
            Upstream::Fixture(Box::new(FixtureUpstream::new(FixtureRole::Admin, 0))),
            None,
            50,
            1000,
        )
    }

    #[tokio::test]
    async fn one_poller_fans_out_to_every_subscriber() {
        let h = hub();
        let mut a = h.subscribe();
        let mut b = h.subscribe();
        assert!(h.poll_once().await);
        let fa = a.recv().await.unwrap();
        let fb = b.recv().await.unwrap();
        assert_eq!(fa.name, "snapshot");
        assert!(
            Arc::ptr_eq(&fa.data, &fb.data),
            "serialised once, shared by all"
        );
        let ev: ServerEvent = serde_json::from_str(&fa.data).unwrap();
        assert!(matches!(ev, ServerEvent::Snapshot { seq: 1, .. }));
        assert!(h.latest().0.is_some());
    }

    #[tokio::test]
    async fn poller_idles_without_subscribers_and_wakes_on_join() {
        let h = hub();
        let mut rx = h.subscribe();
        tokio::spawn(h.clone().run_poller());
        tokio::time::sleep(Duration::from_millis(120)).await;
        assert!(
            rx.try_recv().is_err(),
            "no upstream read while nobody is connected"
        );
        let id = h.join(Credentials::default());
        let f = tokio::time::timeout(Duration::from_secs(2), rx.recv())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(f.name, "snapshot");
        h.refresh_now();
        let f2 = tokio::time::timeout(Duration::from_secs(2), rx.recv())
            .await
            .unwrap()
            .unwrap();
        let (s1, s2): (ServerEvent, ServerEvent) = (
            serde_json::from_str(&f.data).unwrap(),
            serde_json::from_str(&f2.data).unwrap(),
        );
        match (s1, s2) {
            (ServerEvent::Snapshot { seq: a, .. }, ServerEvent::Snapshot { seq: b, .. }) => {
                assert!(b > a)
            }
            other => panic!("unexpected {other:?}"),
        }
        h.leave(id);
        assert_eq!(h.subscribers(), 0);
    }

    #[tokio::test]
    async fn upstream_errors_are_broadcast_but_not_cached() {
        let h = Hub::new(
            Mode::Fixture,
            Upstream::Fixture(Box::new(FixtureUpstream::new(FixtureRole::Admin, 1))),
            None,
            50,
            1000,
        );
        let mut rx = h.subscribe();
        h.poll_once().await;
        let f = rx.recv().await.unwrap();
        assert_eq!(f.name, "upstream_error");
        assert!(h.latest().0.is_none());
    }
}
