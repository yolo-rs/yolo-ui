//! Tab 5: autonomous agents, session goals, agent access keys.

use super::components::{Empty, Glyph, Meta, SkeletonLines, SourceError, TextField};
use super::{run_action, use_ctx};
use crate::icons;
use crate::model::{Dashboard, SourceKey, SourceStatus};
use crate::protocol::CockpitAction;
use dioxus::prelude::*;

#[component]
pub fn Agents(dashboard: Option<Dashboard>) -> Element {
    let ctx = use_ctx();
    let gateway = dashboard
        .as_ref()
        .and_then(|d| d.sources.bot_rag.data.as_ref())
        .and_then(|b| b.gateway_ready);
    let (tone, gw) = match gateway {
        Some(true) => ("ok", ctx.t("adminAi.agents.gatewayReady")),
        Some(false) => ("danger", ctx.t("adminAi.agents.gatewayDown")),
        None => ("warn", ctx.t("adminAi.agents.gatewayUnknown")),
    };
    rsx! {
        section { class: "card", "aria-labelledby": "agents-title",
            div { class: "stack stack--tight",
                h2 { id: "agents-title", class: "card__title", Glyph { icon: icons::USERS, size: "md" } "{ctx.t(\"adminAi.agents.title\")}" }
                p { class: "card__desc", "{ctx.t(\"adminAi.agents.desc\")}" }
            }
            div { class: "grid",
                div { class: "card card--inset",
                    div { class: "spread", span { class: "strong", "{ctx.t(\"adminAi.agents.botName\")}" } span { class: "badge badge--{tone}", "{gw}" } }
                    p { class: "small muted", "{ctx.t(\"adminAi.agents.botDesc\")}" }
                }
                div { class: "card card--inset",
                    div { class: "spread", span { class: "strong", "{ctx.t(\"adminAi.agents.claudeName\")}" } span { class: "badge badge--info mono", "CLAUDE.md" } }
                    p { class: "small muted", "{ctx.t(\"adminAi.agents.claudeDesc\")}" }
                }
                div { class: "card card--inset",
                    div { class: "spread", span { class: "strong", "{ctx.t(\"adminAi.agents.otherName\")}" } span { class: "badge badge--info mono", "GEMINI / CODEX.md" } }
                    p { class: "small muted", "{ctx.t(\"adminAi.agents.otherDesc\")}" }
                }
            }
        }
        div { class: "grid grid--wide",
            Goals { dashboard: dashboard.clone() }
            Keys { dashboard: dashboard.clone() }
        }
    }
}

#[component]
fn Goals(dashboard: Option<Dashboard>) -> Element {
    let ctx = use_ctx();
    let mut title = use_signal(String::new);
    let mut desc = use_signal(String::new);
    let busy = use_signal(|| false);
    let error = use_signal(|| None::<String>);
    let report = dashboard.as_ref().map(|d| &d.sources.goals);
    let rows = report
        .and_then(|r| r.ok_data())
        .map(|g| g.rows.clone())
        .unwrap_or_default();
    let ok = report.map(|r| r.status) == Some(SourceStatus::Ok);
    let active = dashboard.as_ref().and_then(|d| d.summary.active_goals);
    let submit = {
        let ctx = ctx.clone();
        move |_| {
            let (mut busy, mut error, mut title, mut desc) = (busy, error, title, desc);
            let t = title().trim().to_owned();
            if t.is_empty() {
                error.set(Some(ctx.t("adminAi.goals.titleRequired").to_owned()));
                return;
            }
            if t.chars().count() > 200 {
                error.set(Some(ctx.t("adminAi.goals.titleTooLong").to_owned()));
                return;
            }
            error.set(None);
            let d = desc().trim().to_owned();
            let description = if d.is_empty() { t.clone() } else { d };
            busy.set(true);
            let ctx = ctx.clone();
            spawn(async move {
                if run_action(&ctx, CockpitAction::CreateGoal { title: t, description })
                    .await
                    .is_some()
                {
                    title.set(String::new());
                    desc.set(String::new());
                }
                busy.set(false);
            });
        }
    };
    rsx! {
        section { class: "card", "aria-labelledby": "goals-title",
            div { class: "spread",
                h2 { id: "goals-title", class: "card__title card__title--sm", Glyph { icon: icons::TREND_UP } "{ctx.t(\"adminAi.goals.title\")}" }
                span { class: "small muted tabular", "{ctx.tf(\"adminAi.goals.active\", &[(\"count\", &ctx.int(active))])}" }
            }
            if let Some(d) = dashboard.as_ref() { SourceError { meta: Meta::from(d.sources.meta(SourceKey::Goals)) } }
            form { class: "form", onsubmit: move |e: FormEvent| { e.prevent_default(); submit(()); },
                p { class: "strong small", "{ctx.t(\"adminAi.goals.addTitle\")}" }
                TextField { id: "goal-title".to_owned(), label: ctx.t("adminAi.goals.titleLabel").to_owned(), value: title(), on_input: move |v| title.set(v), placeholder: ctx.t("adminAi.goals.titlePlaceholder").to_owned(), error: error() }
                TextField { id: "goal-desc".to_owned(), label: ctx.t("adminAi.goals.descLabel").to_owned(), value: desc(), on_input: move |v| desc.set(v), placeholder: ctx.t("adminAi.goals.descPlaceholder").to_owned(), hint: Some(ctx.t("adminAi.goals.descHint").to_owned()) }
                div { class: "row", button { r#type: "submit", class: "btn btn--primary", disabled: busy(), Glyph { icon: icons::PLUS } "{ctx.t(\"adminAi.goals.add\")}" } }
            }
            if report.is_none() {
                SkeletonLines { count: 4 }
            } else if rows.is_empty() && ok {
                Empty { text: ctx.t("adminAi.goals.empty").to_owned() }
            } else {
                ul { class: "scroll plain",
                    for g in rows.iter() {
                        {
                            let id = g.id;
                            let gt = g.title.clone();
                            let c = ctx.clone();
                            rsx! {
                                li { key: "{g.id}", class: "item",
                                    div { class: "item__head",
                                        span { class: "strong break", "#{g.id} — {g.title}" }
                                        button { r#type: "button", class: "btn btn--sm",
                                            onclick: move |_| {
                                                let c2 = c.clone();
                                                c.ask(
                                                    c.t("adminAi.goals.confirmTitle").to_owned(),
                                                    c.tf("adminAi.goals.confirmBody", &[("id", &id.to_string()), ("title", &gt)]),
                                                    c.t("adminAi.goals.complete").to_owned(),
                                                    false,
                                                    Callback::new(move |_: ()| {
                                                        let c3 = c2.clone();
                                                        spawn(async move {
                                                            let notes = c3.t("adminAi.goals.completionNote").to_owned();
                                                            if run_action(&c3, CockpitAction::CompleteGoal { id, notes }).await.is_some() {
                                                                c3.say(c3.t("adminAi.goals.completed"));
                                                            }
                                                        });
                                                    }),
                                                );
                                            },
                                            Glyph { icon: icons::CHECK_CIRCLE } "{ctx.t(\"adminAi.goals.complete\")}"
                                        }
                                    }
                                    if let Some(d) = g.description.as_ref() { p { class: "small muted", "{d}" } }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn Keys(dashboard: Option<Dashboard>) -> Element {
    let ctx = use_ctx();
    let report = dashboard.as_ref().map(|d| &d.sources.agent_keys);
    let rows = report
        .and_then(|r| r.ok_data())
        .cloned()
        .unwrap_or_default();
    let ok = report.map(|r| r.status) == Some(SourceStatus::Ok);
    rsx! {
        section { class: "card", "aria-labelledby": "keys-title",
            div { class: "spread",
                h2 { id: "keys-title", class: "card__title card__title--sm", Glyph { icon: icons::SHIELD_CHECK } "{ctx.t(\"adminAi.keys.title\")}" }
                a { href: "/admin/agents", class: "btn btn--sm btn--ghost", "{ctx.t(\"adminAi.keys.manager\")}" }
            }
            if let Some(d) = dashboard.as_ref() { SourceError { meta: Meta::from(d.sources.meta(SourceKey::AgentKeys)) } }
            if report.is_none() {
                SkeletonLines { count: 3 }
            } else if rows.is_empty() && ok {
                Empty { text: ctx.t("adminAi.keys.empty").to_owned() }
            } else {
                ul { class: "scroll plain",
                    for k in rows.iter() {
                        li { key: "{k.id}", class: "item",
                            div { class: "item__head",
                                span { class: "strong", Glyph { icon: icons::KEY } " {k.name}" }
                                span { class: "mono muted", "{ctx.tf(\"adminAi.keys.created\", &[(\"date\", &crate::format::date(ctx.lang, k.created_at.as_deref(), ctx.tz))])}" }
                            }
                            span { class: "small muted", "{ctx.t(\"adminAi.keys.scopes\")} " code { "{k.scopes.join(\", \")}" } }
                        }
                    }
                }
            }
        }
    }
}
