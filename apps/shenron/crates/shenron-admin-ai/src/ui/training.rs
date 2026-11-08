//! Tab 2: live RAG evaluation, evaluation history, gold set, feedback arbitration.

use super::components::{Empty, Glyph, Meta, SkeletonLines, SourceError, TextField};
use super::{NoticeKind, run_action, use_ctx};
use crate::format;
use crate::icons;
use crate::model::{Dashboard, EvalResult, SourceKey, SourceStatus};
use crate::protocol::{CockpitAction, FeedbackStatus, split_keywords};
use crate::state::filter_gold;
use dioxus::prelude::*;

#[component]
pub fn Training(dashboard: Option<Dashboard>) -> Element {
    rsx! {
        Evaluation { dashboard: dashboard.clone() }
        History { dashboard: dashboard.clone() }
        div { class: "grid grid--wide",
            GoldSet { dashboard: dashboard.clone() }
            Feedbacks { dashboard: dashboard.clone() }
        }
    }
}

#[component]
fn Evaluation(dashboard: Option<Dashboard>) -> Element {
    let ctx = use_ctx();
    let mut samples = use_signal(|| 25u32);
    let running = use_signal(|| false);
    let message = use_signal(|| None::<(NoticeKind, String)>);
    let live = use_signal(|| None::<EvalResult>);
    let engine = dashboard
        .as_ref()
        .map(|d| d.backend.engine.clone())
        .unwrap_or_else(|| "—".into());
    let gold_cases = dashboard.as_ref().and_then(|d| d.summary.gold_cases);
    let start = {
        let ctx = ctx.clone();
        move |_| {
            let (mut running, mut message, mut live) = (running, message, live);
            let n = samples();
            let scope = if n > 0 {
                ctx.tf("adminAi.eval.scopeN", &[("count", &n.to_string())])
            } else {
                ctx.t("adminAi.eval.scopeAll").to_owned()
            };
            running.set(true);
            message.set(Some((
                NoticeKind::Info,
                ctx.tf("adminAi.eval.running", &[("scope", &scope)]),
            )));
            let ctx = ctx.clone();
            spawn(async move {
                match crate::client::post(
                    &ctx.ep,
                    "/action",
                    &CockpitAction::EvaluateRag { samples: n },
                )
                .await
                {
                    Ok(r) if r.ok() => {
                        let metrics: EvalResult = serde_json::from_value(
                            r.body.get("metrics").cloned().unwrap_or_default(),
                        )
                        .unwrap_or_default();
                        let key = if metrics.persisted {
                            "adminAi.eval.persisted"
                        } else {
                            "adminAi.eval.notPersisted"
                        };
                        let text = ctx.tf(key, &[("engine", &metrics.engine)]);
                        ctx.say(text.clone());
                        message.set(Some((NoticeKind::Ok, text)));
                        live.set(Some(metrics));
                    }
                    Ok(r) => message.set(Some((
                        NoticeKind::Error,
                        r.error().unwrap_or_else(|| {
                            ctx.tf("adminAi.error.http", &[("status", &r.status.to_string())])
                        }),
                    ))),
                    Err(f) => message.set(Some((NoticeKind::Error, f.message))),
                }
                running.set(false);
            });
        }
    };
    let tone = |k: NoticeKind| match k {
        NoticeKind::Ok => "ok",
        NoticeKind::Info => "info",
        NoticeKind::Error => "error",
    };
    rsx! {
        section { class: "card", "aria-labelledby": "eval-title", "aria-busy": if running() { "true" } else { "false" },
            div { class: "spread",
                div { class: "stack stack--tight",
                    h2 { id: "eval-title", class: "card__title", Glyph { icon: icons::MEDAL, size: "md" } "{ctx.t(\"adminAi.eval.title\")}" }
                    p { class: "card__desc", "{ctx.tf(\"adminAi.eval.desc\", &[(\"engine\", &engine)])}" }
                }
                div { class: "row",
                    label { class: "sr-only", r#for: "eval-samples", "{ctx.t(\"adminAi.eval.samples\")}" }
                    select { id: "eval-samples", class: "select",
                        value: "{samples}",
                        onchange: move |e| samples.set(e.value().parse().unwrap_or(25)),
                        option { value: "10", "{ctx.tf(\"adminAi.eval.cases\", &[(\"count\", \"10\")])}" }
                        option { value: "25", "{ctx.tf(\"adminAi.eval.cases\", &[(\"count\", \"25\")])}" }
                        option { value: "0", "{ctx.tf(\"adminAi.eval.allCases\", &[(\"count\", &ctx.int(gold_cases))])}" }
                    }
                    button { r#type: "button", class: "btn btn--primary", disabled: running(), onclick: start,
                        Glyph { icon: if running() { icons::CIRCLE_NOTCH } else { icons::PLAY } }
                        "{ctx.t(\"adminAi.eval.run\")}"
                    }
                }
            }
            if let Some((k, text)) = message() {
                div { class: "notice notice--{tone(k)}", role: "status", span { class: "notice__text", "{text}" } }
            }
            if running() && live().is_none() {
                div { class: "grid grid--metrics", for i in 0..5 { div { key: "{i}", class: "skeleton skeleton--block" } } }
            }
            if let Some(m) = live() {
                div { class: "grid grid--metrics",
                    MiniMetric { label: ctx.t("adminAi.eval.colCases").to_owned(), value: m.cases.to_string(), tone: "" }
                    MiniMetric { label: "Recall@1".to_owned(), value: ctx.pct(Some(m.recall.at1)), tone: "" }
                    MiniMetric { label: "Recall@5".to_owned(), value: ctx.pct(Some(m.recall.at5)), tone: "ok" }
                    MiniMetric { label: "MRR".to_owned(), value: ctx.pct(Some(m.mrr)), tone: "info" }
                    MiniMetric { label: "nDCG@10".to_owned(), value: ctx.pct(Some(m.ndcg)), tone: "accent" }
                }
                if !m.misses.is_empty() {
                    details { class: "details",
                        summary { "{ctx.tf(\"adminAi.eval.misses\", &[(\"count\", &m.misses.len().to_string())])}" }
                        ul { class: "stack plain",
                            for (i, miss) in m.misses.iter().enumerate() {
                                li { key: "{i}", class: "item",
                                    span { class: "strong", "{miss.q}" }
                                    span { class: "small tone-ok", "{ctx.tf(\"adminAi.gold.expected\", &[(\"list\", &miss.expected.join(\", \"))])}" }
                                    if let Some(t) = miss.returned_title.as_ref() { span { class: "small muted", "{ctx.tf(\"adminAi.eval.returned\", &[(\"title\", t)])}" } }
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
fn MiniMetric(label: String, value: String, tone: &'static str) -> Element {
    rsx! {
        div { class: "metric",
            div { class: "metric__label", "{label}" }
            div { class: "metric__value tabular tone-{tone}", "{value}" }
        }
    }
}

#[component]
fn History(dashboard: Option<Dashboard>) -> Element {
    let ctx = use_ctx();
    let report = dashboard.as_ref().map(|d| &d.sources.eval_history);
    let rows = report
        .and_then(|r| r.ok_data())
        .cloned()
        .unwrap_or_default();
    rsx! {
        section { class: "card", "aria-labelledby": "history-title",
            h2 { id: "history-title", class: "card__title card__title--sm", Glyph { icon: icons::CLOCK } "{ctx.t(\"adminAi.history.title\")}" }
            if let Some(d) = dashboard.as_ref() { SourceError { meta: Meta::from(d.sources.meta(SourceKey::EvalHistory)) } }
            if report.is_none() {
                SkeletonLines { count: 5 }
            } else if rows.is_empty() {
                Empty { text: if report.map(|r| r.status) == Some(SourceStatus::Ok) { ctx.t("adminAi.history.empty").to_owned() } else { ctx.t("adminAi.history.unavailable").to_owned() } }
            } else {
                div { class: "table-wrap", tabindex: "0", role: "region", "aria-labelledby": "history-title",
                    table { class: "table",
                        thead { tr {
                            th { scope: "col", "{ctx.t(\"adminAi.history.date\")}" }
                            th { scope: "col", "{ctx.t(\"adminAi.eval.colCases\")}" }
                            th { scope: "col", "Recall@1" }
                            th { scope: "col", "Recall@5" }
                            th { scope: "col", "MRR" }
                            th { scope: "col", "nDCG@10" }
                            th { scope: "col", "{ctx.t(\"adminAi.history.duration\")}" }
                            th { scope: "col", "{ctx.t(\"adminAi.history.status\")}" }
                        } }
                        tbody {
                            for s in rows.iter() {
                                tr { key: "{s.id}",
                                    td { "{ctx.datetime(Some(&s.created_at))}" }
                                    td { "{s.cases_count}" }
                                    td { "{ctx.pct(Some(s.recall1))}" }
                                    td { class: "tone-ok strong", "{ctx.pct(Some(s.recall5))}" }
                                    td { class: "tone-info", "{ctx.pct(Some(s.mrr))}" }
                                    td { class: "tone-accent", "{ctx.pct(Some(s.ndcg))}" }
                                    td { class: "muted", "{format::ms(ctx.lang, s.duration_ms)}" }
                                    td {
                                        if s.misses_count == 0 {
                                            span { class: "tone-ok row", Glyph { icon: icons::CHECK_CIRCLE } "{ctx.t(\"adminAi.history.perfect\")}" }
                                        } else {
                                            span { class: "tone-warn row", Glyph { icon: icons::WARNING } "{ctx.tf(\"adminAi.history.failures\", &[(\"count\", &s.misses_count.to_string())])}" }
                                        }
                                    }
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
fn GoldSet(dashboard: Option<Dashboard>) -> Element {
    let ctx = use_ctx();
    let mut q = use_signal(String::new);
    let mut expect = use_signal(String::new);
    let mut filter = use_signal(String::new);
    let busy = use_signal(|| false);
    let errors = use_signal(|| (None::<String>, None::<String>));
    let sample = dashboard
        .as_ref()
        .and_then(|d| d.sources.gold_set.ok_data())
        .map(|g| g.sample.clone())
        .unwrap_or_default();
    let total = dashboard.as_ref().and_then(|d| d.summary.gold_cases);
    let f = filter();
    let shown: Vec<_> = filter_gold(&sample, &f).into_iter().cloned().collect();
    let submit = {
        let ctx = ctx.clone();
        move |_| {
            let (mut busy, mut errors, mut q, mut expect) = (busy, errors, q, expect);
            let question = q().trim().to_owned();
            let keywords = split_keywords(&expect());
            let eq = question
                .is_empty()
                .then(|| ctx.t("adminAi.gold.qRequired").to_owned());
            let ee = keywords
                .is_empty()
                .then(|| ctx.t("adminAi.gold.expectRequired").to_owned());
            if eq.is_some() || ee.is_some() {
                errors.set((eq, ee));
                return;
            }
            errors.set((None, None));
            busy.set(true);
            let ctx = ctx.clone();
            spawn(async move {
                if run_action(
                    &ctx,
                    CockpitAction::AddGoldCase { q: question, expect: keywords },
                )
                .await
                .is_some()
                {
                    q.set(String::new());
                    expect.set(String::new());
                }
                busy.set(false);
            });
        }
    };
    rsx! {
        section { class: "card", "aria-labelledby": "gold-title",
            h2 { id: "gold-title", class: "card__title card__title--sm", Glyph { icon: icons::MEDAL } "{ctx.tf(\"adminAi.gold.title\", &[(\"count\", &ctx.int(total))])}" }
            if let Some(d) = dashboard.as_ref() { SourceError { meta: Meta::from(d.sources.meta(SourceKey::GoldSet)) } }
            form { class: "form", onsubmit: move |e: FormEvent| { e.prevent_default(); submit(()); },
                p { class: "strong small", "{ctx.t(\"adminAi.gold.addTitle\")}" }
                TextField { id: "gold-q".to_owned(), label: ctx.t("adminAi.gold.qLabel").to_owned(), value: q(), on_input: move |v| q.set(v), placeholder: ctx.t("adminAi.gold.qPlaceholder").to_owned(), error: errors().0 }
                TextField { id: "gold-expect".to_owned(), label: ctx.t("adminAi.gold.expectLabel").to_owned(), value: expect(), on_input: move |v| expect.set(v), placeholder: ctx.t("adminAi.gold.expectPlaceholder").to_owned(), hint: Some(ctx.t("adminAi.gold.expectHint").to_owned()), error: errors().1 }
                div { class: "row",
                    button { r#type: "submit", class: "btn btn--primary", disabled: busy(), Glyph { icon: icons::PLUS } "{ctx.t(\"adminAi.gold.add\")}" }
                }
            }
            TextField { id: "gold-filter".to_owned(), label: ctx.t("adminAi.gold.filterLabel").to_owned(), value: f.clone(), on_input: move |v| filter.set(v), placeholder: ctx.t("adminAi.gold.filterPlaceholder").to_owned() }
            p { class: "small muted", role: "status", "{ctx.tf(\"adminAi.gold.shown\", &[(\"shown\", &shown.len().to_string()), (\"total\", &sample.len().to_string())])}" }
            if dashboard.is_none() {
                SkeletonLines { count: 4 }
            } else if shown.is_empty() {
                Empty { text: ctx.t("adminAi.gold.empty").to_owned() }
            } else {
                ul { class: "scroll plain",
                    for (i, c) in shown.iter().enumerate() {
                        li { key: "{i}", class: "item",
                            span { class: "strong", "{c.q}" }
                            span { class: "small tone-ok break", "{ctx.tf(\"adminAi.gold.expected\", &[(\"list\", &c.expected.join(\", \"))])}" }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn Feedbacks(dashboard: Option<Dashboard>) -> Element {
    let ctx = use_ctx();
    let report = dashboard.as_ref().map(|d| &d.sources.feedbacks);
    let rows = report
        .and_then(|r| r.ok_data())
        .map(|f| f.rows.clone())
        .unwrap_or_default();
    let ok = report.map(|r| r.status) == Some(SourceStatus::Ok);
    rsx! {
        section { class: "card", "aria-labelledby": "fb-title",
            h2 { id: "fb-title", class: "card__title card__title--sm", Glyph { icon: icons::SPARKLE } "{ctx.t(\"adminAi.feedback.title\")}" }
            if let Some(d) = dashboard.as_ref() { SourceError { meta: Meta::from(d.sources.meta(SourceKey::Feedbacks)) } }
            if report.is_none() {
                SkeletonLines { count: 5 }
            } else if rows.is_empty() && ok {
                Empty { text: ctx.t("adminAi.feedback.empty").to_owned() }
            } else {
                ul { class: "scroll scroll--tall plain",
                    for fb in rows.iter() {
                        {
                            let tone = match fb.status.as_str() { "pending" => "warn", "gold_added" => "ok", _ => "neutral" };
                            let status_label = match fb.status.as_str() {
                                "pending" => ctx.t("adminAi.feedback.pending"),
                                "gold_added" => ctx.t("adminAi.feedback.goldAdded"),
                                "rejected" => ctx.t("adminAi.feedback.rejected"),
                                "reviewed" => ctx.t("adminAi.feedback.reviewed"),
                                other => if other.is_empty() { "—" } else { ctx.t("adminAi.feedback.other") },
                            };
                            let id = fb.id;
                            let query = fb.query.clone();
                            let (c1, c2) = (ctx.clone(), ctx.clone());
                            rsx! {
                                li { key: "{fb.id}", class: "item",
                                    div { class: "item__head",
                                        span { class: "strong break", "#{fb.id} — « {fb.query} »" }
                                        span { class: "badge badge--{tone}", "{status_label}" }
                                    }
                                    if let Some(e) = fb.expected.as_ref() { span { class: "small tone-ok", "{ctx.tf(\"adminAi.feedback.expected\", &[(\"value\", e)])}" } }
                                    if let Some(c) = fb.comment.as_ref() { span { class: "small muted", "« {c} »" } }
                                    if fb.status == "pending" {
                                        div { class: "row",
                                            button { r#type: "button", class: "btn btn--sm btn--primary",
                                                onclick: {
                                                    let query = query.clone();
                                                    move |_| confirm_feedback(&c1, id, &query, FeedbackStatus::GoldAdded)
                                                },
                                                Glyph { icon: icons::CHECK_CIRCLE } "{ctx.t(\"adminAi.feedback.validate\")}"
                                            }
                                            button { r#type: "button", class: "btn btn--sm btn--danger",
                                                onclick: move |_| confirm_feedback(&c2, id, &query, FeedbackStatus::Rejected),
                                                Glyph { icon: icons::X } "{ctx.t(\"adminAi.feedback.reject\")}"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn confirm_feedback(ctx: &Ctx2, id: u64, query: &str, status: FeedbackStatus) {
    let (title, body, label, danger) = match status {
        FeedbackStatus::GoldAdded => (
            ctx.t("adminAi.feedback.confirmValidateTitle"),
            ctx.tf(
                "adminAi.feedback.confirmValidateBody",
                &[("id", &id.to_string()), ("query", query)],
            ),
            ctx.t("adminAi.feedback.validate"),
            false,
        ),
        _ => (
            ctx.t("adminAi.feedback.confirmRejectTitle"),
            ctx.tf(
                "adminAi.feedback.confirmRejectBody",
                &[("id", &id.to_string()), ("query", query)],
            ),
            ctx.t("adminAi.feedback.reject"),
            true,
        ),
    };
    let c = ctx.clone();
    ctx.ask(
        title.to_owned(),
        body,
        label.to_owned(),
        danger,
        Callback::new(move |_: ()| {
            let c = c.clone();
            spawn(async move {
                if run_action(&c, CockpitAction::UpdateFeedback { id, status })
                    .await
                    .is_some()
                {
                    c.say(c.t("adminAi.feedback.updated"));
                }
            });
        }),
    );
}

type Ctx2 = super::Ctx;
