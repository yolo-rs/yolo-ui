//! Header: identity, overall status, header sources, connection, last update, controls, tabs.

use super::components::Glyph;
use super::{connection_text, use_ctx};
use crate::icons::{self, Icon};
use crate::model::{Dashboard, SourceKey, SourceStatus};
use crate::protocol::Mode;
use crate::state::{Connection, Tab};
use dioxus::prelude::*;
use std::rc::Rc;

const HEADER_SOURCES: [(SourceKey, &str); 4] = [
    (SourceKey::AphrodyHealth, "adminAi.header.srcAphrody"),
    (SourceKey::AphrodyWeb, "adminAi.header.srcWeb"),
    (SourceKey::BotRag, "adminAi.header.srcBot"),
    (SourceKey::EvalHistory, "adminAi.header.srcPostgres"),
];

fn tab_meta(tab: Tab) -> (&'static str, Icon) {
    match tab {
        Tab::Overview => ("adminAi.tabs.overview", icons::PULSE),
        Tab::Training => ("adminAi.tabs.training", icons::MEDAL),
        Tab::Canon => ("adminAi.tabs.canon", icons::SCALES),
        Tab::Corpus => ("adminAi.tabs.corpus", icons::BOOK_OPEN),
        Tab::Agents => ("adminAi.tabs.agents", icons::USERS),
    }
}

#[component]
pub fn Header(
    dashboard: Option<Dashboard>,
    tab: Tab,
    on_tab: EventHandler<Tab>,
    paused: bool,
    on_pause: EventHandler<()>,
    light: bool,
    on_theme: EventHandler<()>,
) -> Element {
    let ctx = use_ctx();
    let st = ctx.state.read();
    let conn = st.connection;
    let mode = st.mode;
    let last = st.last_update.clone();
    let poll = st.poll_ms;
    let activity_count = st.activity.len();
    drop(st);
    let mut refs: Signal<Vec<Option<Rc<MountedData>>>> = use_signal(|| vec![None; Tab::ALL.len()]);

    let summary = dashboard.as_ref().map(|d| &d.summary);
    let overall = summary.map(|s| {
        if s.status == "ok" {
            ("ok", ctx.t("adminAi.header.allOk").to_owned())
        } else {
            (
                "danger",
                ctx.tf(
                    "adminAi.header.failing",
                    &[("count", &s.failing.len().to_string())],
                ),
            )
        }
    });
    let pending = summary.and_then(|s| s.pending_feedbacks).unwrap_or(0);
    let goals = summary.and_then(|s| s.active_goals).unwrap_or(0);
    let conn_text = connection_text(&ctx, conn);
    let last_text = match &last {
        Some(t) => ctx.tf(
            "adminAi.header.updatedAt",
            &[("time", &crate::format::time(Some(t), ctx.tz))],
        ),
        None => ctx.t("adminAi.header.waiting").to_owned(),
    };
    let cadence = poll.map(|ms| {
        ctx.tf(
            "adminAi.header.cadence",
            &[("seconds", &(ms / 1000).max(1).to_string())],
        )
    });
    let conn_icon = match conn {
        Connection::Live => icons::WIFI_HIGH,
        Connection::Offline { .. } | Connection::Ended => icons::WIFI_SLASH,
        _ => icons::CIRCLE_NOTCH,
    };
    let lang = ctx.lang;
    let _ = activity_count;
    let rctx = ctx.clone();

    rsx! {
        header { class: "header",
            div { class: "header__row",
                div { class: "brand",
                    div { class: "brand__mark", Glyph { icon: icons::BRAIN, size: "md" } }
                    div { class: "stack stack--tight",
                        div { class: "row",
                            h1 { class: "title", "{ctx.t(\"admin.pages.ai.title\")}" }
                            if let Some((tone, text)) = overall {
                                span { class: "badge badge--{tone}",
                                    Glyph { icon: if tone == "ok" { icons::CHECK_CIRCLE } else { icons::WARNING_CIRCLE } }
                                    "{text}"
                                }
                            }
                            if mode == Some(Mode::Fixture) {
                                span { class: "badge badge--info", "{ctx.t(\"adminAi.header.fixture\")}" }
                            }
                        }
                        p { class: "subtitle", "{ctx.t(\"adminAi.header.subtitle\")}" }
                    }
                }
                div { class: "toolbar",
                    span { class: "conn conn--{conn.slug()}",
                        if conn == Connection::Live { span { class: "dot", "aria-hidden": "true" } } else { Glyph { icon: conn_icon } }
                        "{conn_text}"
                    }
                    span { class: "small muted tabular", title: cadence.clone().unwrap_or_default(), "{last_text}" }
                    button {
                        r#type: "button",
                        class: "btn btn--sm",
                        onclick: move |_| { let c = rctx.clone(); spawn(async move { super::refresh(&c).await; }); },
                        Glyph { icon: icons::ARROWS_CLOCKWISE }
                        "{ctx.t(\"adminAi.header.refresh\")}"
                    }
                    button {
                        r#type: "button",
                        class: "btn btn--sm",
                        "aria-pressed": if paused { "true" } else { "false" },
                        onclick: move |_| on_pause.call(()),
                        Glyph { icon: if paused { icons::PLAY } else { icons::CLOCK } }
                        if paused { "{ctx.t(\"adminAi.header.resume\")}" } else { "{ctx.t(\"adminAi.header.pause\")}" }
                    }
                    button {
                        r#type: "button",
                        class: "btn btn--sm",
                        "aria-pressed": if light { "true" } else { "false" },
                        onclick: move |_| on_theme.call(()),
                        "{ctx.t(\"adminAi.header.lightTheme\")}"
                    }
                    nav { class: "row", "aria-label": "{ctx.t(\"adminAi.header.language\")}",
                        for l in crate::i18n::Lang::ALL {
                            a {
                                key: "{l.code()}",
                                class: "btn btn--sm btn--ghost",
                                href: "?lang={l.code()}&tab={tab.slug()}",
                                hreflang: "{l.code()}",
                                lang: "{l.code()}",
                                "aria-current": if l == lang { "true" } else { "false" },
                                "{l.code().to_uppercase()}"
                            }
                        }
                    }
                    a { class: "btn btn--sm", href: "/wiki/canon", target: "_blank", rel: "noopener",
                        Glyph { icon: icons::ARROW_SQUARE_OUT }
                        "{ctx.t(\"adminAi.header.publicDetector\")}"
                    }
                }
            }
            if let Some(d) = dashboard.as_ref() {
                ul { class: "chips", "aria-label": "{ctx.t(\"adminAi.header.sources\")}",
                    for (key, name) in HEADER_SOURCES {
                        {
                            let m = d.sources.meta(key);
                            let tone = super::components::status_tone(m.status);
                            let tail = match m.latency_ms {
                                Some(ms) => crate::format::ms(lang, ms),
                                None => ctx.t(super::components::status_key(m.status)).to_owned(),
                            };
                            let title = match m.error { Some(e) => format!("{} — {e}", m.target), None => m.target.to_owned() };
                            let status_word = ctx.t(super::components::status_key(m.status));
                            rsx! {
                                li { key: "{name}", class: "badge badge--{tone}", title: "{title}",
                                    Glyph { icon: if m.status == SourceStatus::Ok { icons::CHECK_CIRCLE } else { icons::WARNING_CIRCLE } }
                                    span { "{ctx.t(name)}" }
                                    span { class: "sr-only", "{status_word}" }
                                    span { class: "mono", "{tail}" }
                                }
                            }
                        }
                    }
                }
            } else {
                div { class: "row", "aria-hidden": "true",
                    for i in 0..4 { div { key: "{i}", class: "skeleton skeleton--chip" } }
                }
            }
            div { class: "tabs", role: "tablist", "aria-label": "{ctx.t(\"adminAi.tabs.label\")}",
                for (i, t) in Tab::ALL.into_iter().enumerate() {
                    {
                        let (key, icon) = tab_meta(t);
                        let selected = t == tab;
                        let badge = match t { Tab::Training => pending, Tab::Agents => goals, _ => 0 };
                        let badge_label = match t {
                            Tab::Training => ctx.tf("adminAi.tabs.pendingBadge", &[("count", &badge.to_string())]),
                            _ => ctx.tf("adminAi.tabs.goalsBadge", &[("count", &badge.to_string())]),
                        };
                        rsx! {
                            button {
                                key: "{t.slug()}",
                                id: "tab-{t.slug()}",
                                r#type: "button",
                                role: "tab",
                                class: "tab",
                                "aria-selected": if selected { "true" } else { "false" },
                                "aria-controls": "cockpit-panel",
                                tabindex: if selected { "0" } else { "-1" },
                                onmounted: move |e| { refs.write()[i] = Some(e.data()); },
                                onclick: move |_| on_tab.call(t),
                                onkeydown: move |e: KeyboardEvent| {
                                    if let Some(next) = t.step(&e.key().to_string()) {
                                        e.prevent_default();
                                        on_tab.call(next);
                                        let j = Tab::ALL.iter().position(|x| *x == next).unwrap_or(0);
                                        if let Some(el) = refs.read()[j].clone() {
                                            spawn(async move { let _ = el.set_focus(true).await; });
                                        }
                                    }
                                },
                                Glyph { icon }
                                "{ctx.t(key)}"
                                if badge > 0 {
                                    span { class: "tab__badge", "aria-label": "{badge_label}", "{badge}" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
