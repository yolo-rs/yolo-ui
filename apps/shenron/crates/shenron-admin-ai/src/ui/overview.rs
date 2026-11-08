//! Tab 1: supervision, live sources, multi-engine test bench, admin lab channel, activity.

use super::components::{
    Empty, Flag, Glyph, Meta, Metric, SkeletonLines, SourceError, SourceRow, StatusBadge,
};
use super::{run_action, use_ctx};
use crate::format;
use crate::icons;
use crate::model::{Dashboard, SourceKey, SourceStatus, TestPromptResult, tier_tone};
use crate::protocol::CockpitAction;
use dioxus::prelude::*;

/// Private admin lab channel shown by the React cockpit.
pub const LAB_CHANNEL_ID: &str = "1548677623935139901";

const PRESETS: [&str; 6] = [
    "adminAi.bench.preset1",
    "adminAi.bench.preset2",
    "adminAi.bench.preset3",
    "adminAi.bench.preset4",
    "adminAi.bench.preset5",
    "adminAi.bench.preset6",
];

#[component]
pub fn Overview(dashboard: Option<Dashboard>) -> Element {
    rsx! {
        Metrics { dashboard: dashboard.clone() }
        LiveSources { dashboard: dashboard.clone() }
        Bench { engine: dashboard.as_ref().map(|d| d.backend.engine.clone()) }
        Lab { dashboard: dashboard.clone() }
        ActivityFeed {}
    }
}

#[component]
fn Metrics(dashboard: Option<Dashboard>) -> Element {
    let ctx = use_ctx();
    let d = dashboard.as_ref();
    let s = d.map(|d| &d.summary);
    let latest = s.and_then(|s| s.latest_eval.clone());
    let counts = d.map(|d| d.sources.meta(d.summary.counts_source));
    let gold = d.map(|d| d.sources.meta(SourceKey::GoldSet));
    let eval_err = d.and_then(|d| d.sources.eval_history.error.clone());
    let fb = d.map(|d| &d.sources.feedbacks);
    let have = d.is_some();
    let raw = d.map(|d| {
        d.backend
            .raw
            .clone()
            .unwrap_or_else(|| ctx.t("adminAi.overview.unset").to_owned())
    });
    rsx! {
        section { class: "grid grid--metrics", "aria-label": "{ctx.t(\"adminAi.overview.metrics\")}",
            Metric {
                label: ctx.t("adminAi.overview.engine").to_owned(),
                value: d.map(|d| d.backend.engine.clone()),
                text: true,
                hint: rsx! {
                    if let Some(r) = raw { span { class: "mono", "RAG_BACKEND={r}" } }
                    if let Some(st) = s.map(|s| s.serving.status) { StatusBadge { status: st } }
                },
            }
            Metric {
                label: ctx.t("adminAi.overview.documents").to_owned(),
                value: have.then(|| ctx.int(s.and_then(|s| s.documents))),
                hint: rsx! {
                    if let Some(m) = counts.as_ref() {
                        if m.status == SourceStatus::Ok {
                            span { class: "tone-ok row", Glyph { icon: icons::CHECK_CIRCLE } "{m.label}" }
                        } else {
                            span { class: "tone-danger row", Glyph { icon: icons::WARNING } "{m.label} : {m.error.unwrap_or(ctx.t(super::components::status_key(m.status)))}" }
                        }
                    }
                },
            }
            Metric {
                label: ctx.t("adminAi.overview.vectors").to_owned(),
                value: have.then(|| ctx.int(s.and_then(|s| s.vectors))),
                hint: rsx! {
                    if let Some(s) = s {
                        span { class: if s.mode.is_some() { "tone-ok row" } else { "tone-danger row" }, Glyph { icon: icons::CPU } "{ctx.mode_label(s.mode.as_deref())}" }
                    }
                },
            }
            Metric {
                label: ctx.t("adminAi.overview.gold").to_owned(),
                value: have.then(|| ctx.int(s.and_then(|s| s.gold_cases))),
                hint: rsx! {
                    if let Some(g) = gold.as_ref() {
                        if g.status == SourceStatus::Ok {
                            match latest.as_ref() {
                                Some(e) => rsx! { "{ctx.tf(\"adminAi.overview.lastEvalCases\", &[(\"count\", &e.cases_count.to_string())])}" },
                                None => rsx! { "{ctx.t(\"adminAi.overview.noEval\")}" },
                            }
                        } else {
                            "{g.error.unwrap_or_default()}"
                        }
                    }
                },
            }
            Metric {
                label: ctx.t("adminAi.overview.recall5").to_owned(),
                value: have.then(|| ctx.pct(latest.as_ref().map(|e| e.recall5))),
                tone: "ok",
                hint: rsx! {
                    if have {
                        match latest.as_ref() {
                            Some(e) => rsx! { "{ctx.datetime(Some(&e.created_at))}" },
                            None => rsx! { "{eval_err.clone().unwrap_or_else(|| ctx.t(\"adminAi.overview.noSnapshot\").to_owned())}" },
                        }
                    }
                },
            }
            Metric {
                label: ctx.t("adminAi.overview.mrr").to_owned(),
                value: have.then(|| ctx.pct(latest.as_ref().map(|e| e.mrr))),
                tone: "info",
                hint: rsx! { "{ctx.t(\"adminAi.overview.mrrHint\")}" },
            }
            Metric {
                label: ctx.t("adminAi.overview.feedbacks").to_owned(),
                value: have.then(|| ctx.int(fb.and_then(|f| f.ok_data()).map(|f| f.total))),
                hint: rsx! {
                    if let Some(f) = fb {
                        if let Some(data) = f.ok_data() {
                            span { class: "tone-warn", "{ctx.tf(\"adminAi.overview.pendingArbitration\", &[(\"count\", &ctx.int(Some(data.pending)))])}" }
                        } else {
                            "{f.error.clone().unwrap_or_default()}"
                        }
                    }
                },
            }
        }
    }
}

#[component]
fn LiveSources(dashboard: Option<Dashboard>) -> Element {
    let ctx = use_ctx();
    let Some(d) = dashboard else {
        return rsx! {
            section { class: "card", "aria-busy": "true",
                h2 { class: "card__title", Glyph { icon: icons::HARD_DRIVES, size: "md" } "{ctx.t(\"adminAi.overview.sourcesTitle\")}" }
                div { class: "grid grid--wide", for i in 0..6 { div { key: "{i}", class: "skeleton skeleton--block" } } }
            }
        };
    };
    let src = &d.sources;
    let lang = ctx.lang;
    let serving_local = d.backend.value != "aphrody";
    rsx! {
        section { class: "card", "aria-labelledby": "sources-title",
            div { class: "stack stack--tight",
                h2 { id: "sources-title", class: "card__title", Glyph { icon: icons::HARD_DRIVES, size: "md" } "{ctx.t(\"adminAi.overview.sourcesTitle\")}" }
                p { class: "card__desc", "{ctx.t(\"adminAi.overview.sourcesDesc\")}" }
            }
            div { class: "grid grid--wide",
                SourceRow { meta: Meta::from(src.meta(SourceKey::AphrodyHealth)),
                    if let Some(h) = src.aphrody_health.ok_data() {
                        div { class: "row",
                            Flag { label: "RAG".to_owned(), on: h.rag }
                            Flag { label: "LLM".to_owned(), on: h.llm }
                            Flag { label: "OCR".to_owned(), on: h.ocr }
                        }
                    }
                }
                SourceRow { meta: Meta::from(src.meta(SourceKey::AphrodyRagStats)),
                    if let Some(st) = src.aphrody_rag_stats.ok_data() {
                        p { class: "small tabular",
                            {ctx.tf("adminAi.overview.ragStats", &[("docs", &ctx.int(Some(st.chunks))), ("vectors", &ctx.int(Some(st.vectors))), ("sources", &ctx.int(Some(st.sources)))])}
                            if let Some(stored) = st.vectors_stored { " {ctx.tf(\"adminAi.overview.stored\", &[(\"count\", &ctx.int(Some(stored)))])}" }
                        }
                        if !st.kinds.is_empty() {
                            p { class: "small muted tabular",
                                {st.kinds.iter().take(6).map(|k| format!("{} {}", k.kind, format::int(lang, Some(k.count)))).collect::<Vec<_>>().join(" · ")}
                            }
                        }
                    }
                }
                SourceRow { meta: Meta::from(src.meta(SourceKey::AphrodyRagProbe)),
                    if let Some(p) = src.aphrody_rag_probe.ok_data() {
                        p { class: "small", "{ctx.tf(\"adminAi.overview.probe\", &[(\"mode\", &ctx.mode_label(Some(&p.mode))), (\"hits\", &p.hits.to_string()), (\"query\", &p.query)])}" }
                    }
                }
                SourceRow { meta: Meta::from(src.meta(SourceKey::AphrodyWeb)),
                    if let Some(w) = src.aphrody_web.ok_data() {
                        p { class: "small tabular",
                            "{ctx.tf(\"adminAi.overview.crawl\", &[(\"pending\", &ctx.int(Some(w.queue.pending))), (\"locked\", &ctx.int(Some(w.queue.locked))), (\"done\", &ctx.int(Some(w.queue.done))), (\"failed\", &ctx.int(Some(w.queue.failed))), (\"total\", &ctx.int(Some(w.queue.total))), (\"jobs\", &w.jobs.to_string()), (\"running\", &w.running_jobs.to_string())])}"
                        }
                    }
                }
                SourceRow { meta: Meta::from(src.meta(SourceKey::BotRag)),
                    if let Some(b) = src.bot_rag.ok_data() {
                        p { class: "small tabular",
                            "{ctx.tf(\"adminAi.overview.botIndex\", &[(\"docs\", &ctx.int(Some(b.chunks))), (\"vectors\", &ctx.int(Some(b.vectors))), (\"mode\", &ctx.mode_label(Some(&b.mode)))])}"
                            if let Some(at) = b.built_at.as_deref() { " · {ctx.tf(\"adminAi.overview.builtAt\", &[(\"date\", &ctx.datetime(Some(at)))])}" }
                            " · "
                            if serving_local { "{ctx.t(\"adminAi.overview.servesTraffic\")}" } else { "{ctx.t(\"adminAi.overview.offTraffic\")}" }
                        }
                    }
                }
                SourceRow { meta: Meta::from(src.meta(SourceKey::EvalHistory)),
                    if let Some(rows) = src.eval_history.ok_data() {
                        p { class: "small",
                            "{ctx.tf(\"adminAi.overview.snapshots\", &[(\"count\", &rows.len().to_string())])}"
                            if let Some(first) = rows.first() { " · {ctx.tf(\"adminAi.overview.lastSnapshot\", &[(\"date\", &ctx.datetime(Some(&first.created_at)))])}" }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn Bench(engine: Option<String>) -> Element {
    let ctx = use_ctx();
    let mut query = use_signal(String::new);
    let busy = use_signal(|| false);
    let result = use_signal(|| None::<TestPromptResult>);
    let error = use_signal(|| None::<String>);
    let run = {
        let ctx = ctx.clone();
        move |q: String| {
            let (mut error, mut busy, mut result) = (error, busy, result);
            let q = q.trim().to_owned();
            if q.is_empty() {
                error.set(Some(ctx.t("adminAi.bench.queryRequired").to_owned()));
                return;
            }
            error.set(None);
            busy.set(true);
            let ctx = ctx.clone();
            spawn(async move {
                if let Some(r) = run_action(&ctx, CockpitAction::TestPrompt { query: q }).await {
                    match serde_json::from_value::<TestPromptResult>(r.body) {
                        Ok(v) => {
                            ctx.say(ctx.tf(
                                "adminAi.bench.done",
                                &[("ms", &format!("{:.0}", v.latency_ms))],
                            ));
                            result.set(Some(v));
                        }
                        Err(e) => ctx.set_notice(super::NoticeKind::Error, e.to_string()),
                    }
                }
                busy.set(false);
            });
        }
    };
    let engine = engine.unwrap_or_else(|| "—".into());
    let run2 = run.clone();
    rsx! {
        section { class: "card", "aria-labelledby": "bench-title", "aria-busy": if busy() { "true" } else { "false" },
            div { class: "spread",
                div { class: "stack stack--tight",
                    h2 { id: "bench-title", class: "card__title", Glyph { icon: icons::SPARKLE, size: "md" } "{ctx.t(\"adminAi.bench.title\")}" }
                    p { class: "card__desc", "{ctx.tf(\"adminAi.bench.desc\", &[(\"engine\", &engine)])}" }
                }
                div { class: "row", role: "group", "aria-label": "{ctx.t(\"adminAi.bench.presets\")}",
                    for key in PRESETS {
                        {
                            let label = ctx.t(key);
                            let run = run.clone();
                            rsx! {
                                button { key: "{key}", r#type: "button", class: "chip", disabled: busy(),
                                    onclick: move |_| { query.set(label.to_owned()); run(label.to_owned()); },
                                    "{label}"
                                }
                            }
                        }
                    }
                }
            }
            form { class: "form__row",
                onsubmit: move |e: FormEvent| { e.prevent_default(); run2(query()); },
                super::components::TextField {
                    id: "bench-query".to_owned(),
                    label: ctx.t("adminAi.bench.queryLabel").to_owned(),
                    value: query(),
                    on_input: move |v| query.set(v),
                    placeholder: ctx.t("adminAi.bench.placeholder").to_owned(),
                    error: error(),
                }
                button { r#type: "submit", class: "btn btn--primary", disabled: busy(),
                    Glyph { icon: if busy() { icons::CIRCLE_NOTCH } else { icons::PLAY } }
                    if busy() { "{ctx.t(\"admin.confirm.pending\")}" } else { "{ctx.t(\"adminAi.bench.run\")}" }
                }
            }
            if busy() && result().is_none() {
                div { class: "grid", for i in 0..3 { div { key: "{i}", class: "skeleton skeleton--block" } } }
            }
            if let Some(r) = result() { BenchResult { result: r } }
        }
    }
}

#[component]
fn BenchResult(result: TestPromptResult) -> Element {
    let ctx = use_ctx();
    let r = result;
    let rag_meta = Meta {
        status: r.rag.status,
        label: r.rag.label.clone(),
        target: r.rag.target.clone(),
        latency_ms: r.rag.latency_ms,
        error: r.rag.error.clone(),
    };
    let manga_meta = Meta {
        status: r.manga.status,
        label: r.manga.label.clone(),
        target: r.manga.target.clone(),
        latency_ms: r.manga.latency_ms,
        error: r.manga.error.clone(),
    };
    let hits = r
        .rag
        .ok_data()
        .map(|d| d.results.clone())
        .unwrap_or_default();
    let rag_mode = r
        .rag
        .data
        .as_ref()
        .map(|d| ctx.mode_label(Some(&d.mode)))
        .unwrap_or_else(|| {
            ctx.t(super::components::status_key(r.rag.status))
                .to_owned()
        });
    let manga = r.manga.ok_data().cloned().unwrap_or_default();
    rsx! {
        div { class: "stack",
            div { class: "spread small muted",
                span { "{ctx.tf(\"adminAi.bench.resultsFor\", &[(\"query\", &r.query)])}" }
                span { class: "badge badge--neutral tabular", "{ctx.tf(\"adminAi.bench.totalLatency\", &[(\"ms\", &format::ms(ctx.lang, r.latency_ms))])}" }
            }
            div { class: "grid",
                div { class: "card card--inset",
                    div { class: "spread",
                        h3 { class: "card__title card__title--sm", Glyph { icon: icons::SCALES } "{ctx.t(\"adminAi.bench.canon\")}" }
                        if let Some(c) = r.canon.as_ref() {
                            span { class: "badge badge--{tier_tone(&c.tier)}", "{c.tier_label} ({c.score}/100)" }
                        } else {
                            span { class: "badge badge--neutral", "{ctx.t(\"adminAi.bench.unknown\")}" }
                        }
                    }
                    match r.canon.as_ref() {
                        Some(c) => rsx! {
                            p { class: "strong", "{c.verdict}" }
                            p { class: "small muted", "{ctx.tf(\"adminAi.bench.nature\", &[(\"nature\", &c.nature)])}" }
                            if let Some(q) = c.toriyama_quote.as_ref() {
                                blockquote { class: "quote", "« {q.text} »"
                                    p { class: "quote__by", "{ctx.tf(\"adminAi.bench.quoteBy\", &[(\"source\", &q.source)])}" }
                                }
                            }
                        },
                        None => rsx! { Empty { text: ctx.t("adminAi.bench.noRule").to_owned() } },
                    }
                }
                div { class: "card card--inset",
                    div { class: "spread",
                        h3 { class: "card__title card__title--sm", Glyph { icon: icons::DATABASE } "{ctx.tf(\"adminAi.bench.rag\", &[(\"count\", &hits.len().to_string())])}" }
                        span { class: "small muted", "{r.rag.label} · {rag_mode}" }
                    }
                    if r.rag.status != SourceStatus::Ok {
                        SourceError { meta: rag_meta }
                    } else if hits.is_empty() {
                        Empty { text: ctx.t("adminAi.bench.noPassage").to_owned() }
                    } else {
                        ol { class: "scroll plain",
                            for (i, h) in hits.iter().enumerate() {
                                li { key: "{i}", class: "item",
                                    div { class: "item__head",
                                        span { class: "strong tone-primary break", "{h.title}" }
                                        if let Some(sc) = h.score { span { class: "mono muted", "{ctx.tf(\"adminAi.bench.score\", &[(\"score\", &format::decimal(ctx.lang, sc, 2))])}" } }
                                    }
                                    p { class: "small muted", "{h.snippet}" }
                                }
                            }
                        }
                    }
                }
                div { class: "card card--inset",
                    div { class: "spread",
                        h3 { class: "card__title card__title--sm", Glyph { icon: icons::BOOK_OPEN } "{ctx.tf(\"adminAi.bench.manga\", &[(\"count\", &manga.len().to_string())])}" }
                        span { class: "small muted", "{ctx.t(\"adminAi.bench.ocrBubbles\")}" }
                    }
                    if r.manga.status != SourceStatus::Ok {
                        SourceError { meta: manga_meta }
                    } else if manga.is_empty() {
                        Empty { text: ctx.t("adminAi.bench.noPage").to_owned() }
                    } else {
                        ol { class: "scroll plain",
                            for (i, m) in manga.iter().enumerate() {
                                li { key: "{i}", class: "item",
                                    div { class: "item__head",
                                        span { class: "strong tone-ok",
                                            "{ctx.tf(\"adminAi.bench.page\", &[(\"series\", &m.series.to_uppercase()), (\"volume\", &crate::model::display_state(&m.volume).unwrap_or_default()), (\"page\", &crate::model::display_state(&m.page).unwrap_or_default())])}"
                                        }
                                        span { class: "mono muted", "{ctx.tf(\"adminAi.bench.score\", &[(\"score\", &format::decimal(ctx.lang, m.score, 1))])}" }
                                    }
                                    p { class: "small",
                                        for (j, seg) in format::split_marked(&m.excerpt).into_iter().enumerate() {
                                            if seg.marked { mark { key: "{j}", "{seg.text}" } } else { span { key: "{j}", "{seg.text}" } }
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
fn Lab(dashboard: Option<Dashboard>) -> Element {
    let ctx = use_ctx();
    let mut text = use_signal(String::new);
    let busy = use_signal(|| false);
    let error = use_signal(|| None::<String>);
    let report = dashboard.as_ref().map(|d| &d.sources.lab_messages);
    let send = {
        let ctx = ctx.clone();
        move |_| {
            let (mut error, mut busy, mut text) = (error, busy, text);
            let msg = text().trim().to_owned();
            if msg.is_empty() {
                error.set(Some(ctx.t("adminAi.lab.required").to_owned()));
                return;
            }
            error.set(None);
            busy.set(true);
            let ctx = ctx.clone();
            spawn(async move {
                if run_action(&ctx, CockpitAction::SendLabMessage { message: msg })
                    .await
                    .is_some()
                {
                    text.set(String::new());
                    ctx.say(ctx.t("adminAi.lab.sent"));
                }
                busy.set(false);
            });
        }
    };
    rsx! {
        section { class: "card", "aria-labelledby": "lab-title",
            div { class: "spread",
                div { class: "stack stack--tight",
                    h2 { id: "lab-title", class: "card__title", Glyph { icon: icons::CHAT_CIRCLE_TEXT, size: "md" } "{ctx.t(\"adminAi.lab.title\")}" }
                    p { class: "card__desc", "{ctx.t(\"adminAi.lab.desc\")}" }
                }
                span { class: "badge badge--neutral mono", "{ctx.tf(\"adminAi.lab.channel\", &[(\"id\", LAB_CHANNEL_ID)])}" }
            }
            div { class: "lab", role: "log", "aria-label": "{ctx.t(\"adminAi.lab.log\")}",
                match report {
                    None => rsx! { SkeletonLines { count: 4 } },
                    Some(r) if r.status != SourceStatus::Ok => rsx! { SourceError { meta: Meta::from(dashboard.as_ref().unwrap().sources.meta(SourceKey::LabMessages)) } },
                    Some(r) => match r.data.as_ref() {
                        Some(msgs) if !msgs.is_empty() => rsx! {
                            for m in msgs.iter() {
                                div { key: "{m.id}", class: "lab__msg",
                                    div { class: "row small",
                                        span { class: if m.author.to_lowercase().contains("shenron") { "lab__author lab__author--shenron" } else { "lab__author" }, "{m.author}" }
                                        span { class: "muted mono", "{format::time(m.timestamp.as_deref(), ctx.tz)}" }
                                    }
                                    p { class: "pre small", "{m.content}" }
                                }
                            }
                        },
                        _ => rsx! { Empty { text: ctx.t("adminAi.lab.empty").to_owned() } },
                    },
                }
            }
            form { class: "form__row", onsubmit: move |e: FormEvent| { e.prevent_default(); send(()); },
                super::components::TextField {
                    id: "lab-message".to_owned(),
                    label: ctx.t("adminAi.lab.label").to_owned(),
                    value: text(),
                    on_input: move |v| text.set(v),
                    placeholder: ctx.t("adminAi.lab.placeholder").to_owned(),
                    error: error(),
                }
                button { r#type: "submit", class: "btn btn--primary", disabled: busy(),
                    Glyph { icon: if busy() { icons::CIRCLE_NOTCH } else { icons::PAPER_PLANE_RIGHT } }
                    "{ctx.t(\"adminAi.lab.send\")}"
                }
            }
        }
    }
}

#[component]
fn ActivityFeed() -> Element {
    let ctx = use_ctx();
    let items = ctx.state.read().activity.clone();
    rsx! {
        section { class: "card", "aria-labelledby": "activity-title",
            h2 { id: "activity-title", class: "card__title card__title--sm", Glyph { icon: icons::USERS } "{ctx.t(\"adminAi.activity.title\")}" }
            if items.is_empty() {
                p { class: "small muted", "{ctx.t(\"adminAi.activity.empty\")}" }
            } else {
                ul { class: "activity plain",
                    for (i, a) in items.iter().enumerate() {
                        li { key: "{i}",
                            span { class: "mono", "{format::time(Some(&a.at), ctx.tz)} " }
                            "{ctx.tf(\"adminAi.activity.line\", &[(\"actor\", &a.actor), (\"action\", ctx.t(action_key(a.action)))])}"
                        }
                    }
                }
            }
        }
    }
}

pub fn action_key(a: crate::protocol::ActionKind) -> &'static str {
    use crate::protocol::ActionKind::*;
    match a {
        TestPrompt => "adminAi.activity.testPrompt",
        OcrProbe => "adminAi.activity.ocrProbe",
        EvaluateRag => "adminAi.activity.evaluateRag",
        AddGoldCase => "adminAi.activity.addGoldCase",
        UpdateFeedback => "adminAi.activity.updateFeedback",
        SendLabMessage => "adminAi.activity.sendLabMessage",
        CreateGoal => "adminAi.activity.createGoal",
        CompleteGoal => "adminAi.activity.completeGoal",
        SaveCanon => "adminAi.activity.saveCanon",
    }
}
