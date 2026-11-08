//! The cockpit component tree (Dioxus 0.7), shared by the web and desktop renderers.

mod agents;
mod canon;
pub mod components;
mod corpus;
mod header;
mod overview;
mod training;

use crate::client::{self, Endpoint, Reply};
use crate::i18n::{self, Lang};
use crate::model::Dashboard;
use crate::protocol::CockpitAction;
use crate::state::{CockpitState, Connection, Tab};
use components::{ConfirmDialog, NoticeBar};
use dioxus::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoticeKind {
    Ok,
    Info,
    Error,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Notice {
    pub kind: NoticeKind,
    pub text: String,
}

/// A pending confirmation (destructive writes).
#[derive(Clone, PartialEq)]
pub struct Confirm {
    pub title: String,
    pub body: String,
    pub confirm_label: String,
    pub danger: bool,
    pub on_confirm: Callback<()>,
}

/// Shared context of every view.
#[derive(Clone, PartialEq)]
pub struct Ctx {
    pub lang: Lang,
    pub tz: i32,
    pub ep: Endpoint,
    pub state: Signal<CockpitState>,
    pub notice: Signal<Option<Notice>>,
    pub confirm: Signal<Option<Confirm>>,
    /// Text of the polite live region (connection changes, completed actions).
    pub announce: Signal<String>,
}

impl Ctx {
    pub fn t(&self, key: &str) -> &'static str {
        i18n::t(self.lang, key)
    }

    pub fn tf(&self, key: &str, args: &[(&str, &str)]) -> String {
        i18n::tf(self.lang, key, args)
    }

    pub fn int(&self, v: Option<u64>) -> String {
        crate::format::int(self.lang, v)
    }

    pub fn pct(&self, v: Option<f64>) -> String {
        crate::format::pct(self.lang, v)
    }

    pub fn datetime(&self, iso: Option<&str>) -> String {
        crate::format::datetime(self.lang, iso, self.tz)
    }

    pub fn mode_label(&self, mode: Option<&str>) -> String {
        match crate::format::mode_key(mode) {
            Some(k) => self.t(k).to_owned(),
            None => mode.unwrap_or_default().to_owned(),
        }
    }

    pub fn say(&self, text: impl Into<String>) {
        let mut a = self.announce;
        a.set(text.into());
    }

    pub fn set_notice(&self, kind: NoticeKind, text: impl Into<String>) {
        let mut n = self.notice;
        let text = text.into();
        n.set(Some(Notice { kind, text: text.clone() }));
        self.say(text);
    }

    /// Opens the confirmation dialog for `on_confirm`.
    pub fn ask(
        &self,
        title: String,
        body: String,
        confirm_label: String,
        danger: bool,
        on_confirm: Callback<()>,
    ) {
        client::remember_focus();
        let mut c = self.confirm;
        c.set(Some(Confirm {
            title,
            body,
            confirm_label,
            danger,
            on_confirm,
        }));
    }
}

pub fn use_ctx() -> Ctx {
    use_context::<Ctx>()
}

/// `POST {base}/refresh`: the hub re-reads upstream and broadcasts.
pub async fn refresh(ctx: &Ctx) {
    match client::post(&ctx.ep, "/refresh", &serde_json::json!({})).await {
        Ok(r) if r.ok() => ctx.say(ctx.t("adminAi.state.refreshing")),
        Ok(r) => ctx.set_notice(
            NoticeKind::Error,
            r.error().unwrap_or_else(|| {
                ctx.tf("adminAi.error.http", &[("status", &r.status.to_string())])
            }),
        ),
        Err(f) => ctx.set_notice(NoticeKind::Error, f.message),
    }
}

/// `postAction`: sends an action; surfaces the server error or notice instead of failing silently.
pub async fn run_action(ctx: &Ctx, action: CockpitAction) -> Option<Reply> {
    match client::post(&ctx.ep, "/action", &action).await {
        Ok(r) if r.ok() => {
            if let Some(n) = r.notice() {
                ctx.set_notice(NoticeKind::Info, n);
            }
            Some(r)
        }
        Ok(r) => {
            ctx.set_notice(
                NoticeKind::Error,
                r.error().unwrap_or_else(|| {
                    ctx.tf("adminAi.error.http", &[("status", &r.status.to_string())])
                }),
            );
            None
        }
        Err(f) => {
            ctx.set_notice(NoticeKind::Error, f.message);
            None
        }
    }
}

fn connection_text(ctx: &Ctx, c: Connection) -> String {
    match c {
        Connection::Connecting { .. } => ctx.t("adminAi.conn.connecting").to_owned(),
        Connection::Live => ctx.t("adminAi.conn.live").to_owned(),
        Connection::Reconnecting { attempt, retry_in_ms } => ctx.tf(
            "adminAi.conn.reconnecting",
            &[
                ("attempt", &attempt.to_string()),
                ("seconds", &retry_in_ms.div_ceil(1000).to_string()),
            ],
        ),
        Connection::Offline { .. } => ctx.t("adminAi.conn.offline").to_owned(),
        Connection::Ended => ctx.t("adminAi.conn.ended").to_owned(),
    }
}

/// Root component.
#[allow(non_snake_case)]
pub fn App() -> Element {
    let lang = use_hook(client::lang);
    let tz = use_hook(client::tz_offset_min);
    let ep = use_hook(Endpoint::detect);
    let state = use_signal(CockpitState::default);
    let notice = use_signal(|| None::<Notice>);
    let confirm = use_signal(|| None::<Confirm>);
    let announce = use_signal(String::new);
    let ctx =
        use_context_provider(|| Ctx { lang, tz, ep: ep.clone(), state, notice, confirm, announce });

    let mut tab = use_signal(|| Tab::parse(client::initial_tab().as_deref(), Tab::Overview));
    let mut light = use_signal(client::stored_theme_light);
    // Freeze: keep reading a snapshot while live updates continue underneath.
    let mut frozen = use_signal(|| None::<Dashboard>);

    // One subscription for the life of the page.
    use_hook(|| {
        let mut st = state;
        let ep = ep.clone();
        spawn(async move {
            client::run_stream(ep, move |input| {
                st.with_mut(|s| {
                    s.apply(input);
                });
            })
            .await;
        });
        client::on_popstate(move |t| tab.set(Tab::parse(t.as_deref(), Tab::Overview)));
    });

    // Announce connection changes only (not every snapshot: that would flood screen readers).
    let conn = use_memo(move || state.read().connection);
    {
        let ctx = ctx.clone();
        use_effect(move || {
            let c = conn();
            ctx.say(connection_text(&ctx, c));
        });
    }

    let st = state.read();
    let live: Option<Dashboard> = st.dashboard.clone();
    let shown = frozen().or(live);
    let paused = frozen.read().is_some();
    let theme_class = if light() { "app theme-light" } else { "app" };
    let session = st.session.clone();
    let upstream_failure = st.upstream_failure.clone();
    let last_update = st.last_update.clone();
    drop(st);

    let on_tab = move |t: Tab| {
        tab.set(t);
        client::push_tab(t.slug());
    };

    rsx! {
        div { class: "{theme_class}", "data-theme": if light() { "light" } else { "dark" },
            a { class: "skip-link", href: "#cockpit-panel", "{ctx.t(\"adminAi.a11y.skip\")}" }
            div { class: "sr-only", role: "status", "aria-live": "polite", "aria-atomic": "true", "{announce}" }
            main { class: "shell",
                header::Header {
                    dashboard: shown.clone(),
                    tab: tab(),
                    on_tab,
                    paused,
                    on_pause: move |_| {
                        let next = if frozen.read().is_some() { None } else { state.read().dashboard.clone() };
                        frozen.set(next);
                    },
                    light: light(),
                    on_theme: move |_| {
                        let v = !light();
                        light.set(v);
                        client::store_theme_light(v);
                    },
                }
                if let Some(f) = session {
                    div { class: "notice notice--error", role: "alert",
                        span { class: "notice__text",
                            components::Glyph { icon: crate::icons::WARNING_CIRCLE }
                            if f.kind == crate::protocol::FailureKind::Unauthenticated { "{ctx.t(\"adminAi.gate.sessionExpired\")}" } else { "{ctx.t(\"adminAi.gate.forbidden\")}" }
                        }
                        a { class: "btn btn--sm", href: "/signin?callbackURL=%2Fadmin%2Fai%3Ftab%3D{tab().slug()}", "{ctx.t(\"adminAi.gate.signin\")}" }
                    }
                }
                if let Some(f) = upstream_failure {
                    div { class: "notice notice--error", role: "alert",
                        span { class: "notice__text",
                            components::Glyph { icon: crate::icons::WARNING_CIRCLE }
                            {
                                let detail = if f.message.is_empty() {
                                    match f.status { Some(s) => ctx.tf("adminAi.error.http", &[("status", &s.to_string())]), None => ctx.t("adminAi.gate.upstream").to_owned() }
                                } else { f.message.clone() };
                                let since = last_update.as_deref().map(|t| ctx.tf("adminAi.error.lastData", &[("time", &ctx.datetime(Some(t)))])).unwrap_or_default();
                                format!("{} {detail}{since}", ctx.t("adminAi.error.notRefreshed"))
                            }
                        }
                        components::RetryButton {}
                    }
                }
                if let Some(n) = notice() {
                    NoticeBar { notice: n, on_close: move |_| { let mut n = ctx.notice; n.set(None); } }
                }
                div {
                    id: "cockpit-panel",
                    role: "tabpanel",
                    tabindex: "0",
                    "aria-labelledby": "tab-{tab().slug()}",
                    class: "panel",
                    match tab() {
                        Tab::Overview => rsx! { overview::Overview { dashboard: shown.clone() } },
                        Tab::Training => rsx! { training::Training { dashboard: shown.clone() } },
                        Tab::Canon => rsx! { canon::Canon { expected: shown.as_ref().and_then(|d| d.canon_expected.clone()) } },
                        Tab::Corpus => rsx! { corpus::Corpus {} },
                        Tab::Agents => rsx! { agents::Agents { dashboard: shown.clone() } },
                    }
                }
            }
            if let Some(c) = confirm() {
                ConfirmDialog { confirm: c }
            }
        }
    }
}
