//! Tab 3: canon doctrine editor (`CanonEditor` port).
//!
//! The document is read from `GET {base}/canon-config` and written with `PUT` and its `expected`
//! version (optimistic concurrency: 409 when the server changed, 428 without a version). The hub
//! pushes the saved version with every snapshot: an unmodified editor reloads, a modified one
//! keeps the draft and refuses to save until reloaded (`canonEditorSync.conflict`).

use super::components::{Empty, Glyph, SkeletonLines, TextField};
use super::{Ctx, NoticeKind, use_ctx};
use crate::icons;
use crate::model::{CANON_CATEGORIES, CANON_TIERS, CanonConfig, CanonEntry, CanonSnapshot};
use crate::protocol::CanonSave;
use dioxus::prelude::*;

const PAGE: usize = 20;

fn lines(v: &[String]) -> String {
    v.join("\n")
}

fn from_lines(s: &str) -> Vec<String> {
    s.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(str::to_owned)
        .collect()
}

fn from_commas(s: &str) -> Vec<String> {
    crate::protocol::split_keywords(s)
}

fn score_error(ctx: &Ctx, v: f64) -> Option<String> {
    (!(0.0..=100.0).contains(&v)).then(|| ctx.t("adminAi.canon.scoreRange").to_owned())
}

/// Editor state shared by the sections.
#[derive(Clone, Copy, PartialEq)]
struct Doc {
    config: Signal<Option<CanonConfig>>,
    modified: Signal<bool>,
}

impl Doc {
    fn edit(self, f: impl FnOnce(&mut CanonConfig)) {
        let (mut config, mut modified) = (self.config, self.modified);
        config.with_mut(|c| {
            if let Some(c) = c.as_mut() {
                f(c);
            }
        });
        modified.set(true);
    }
}

#[component]
pub fn Canon(expected: Option<String>) -> Element {
    let ctx = use_ctx();
    let config = use_signal(|| None::<CanonConfig>);
    let defaults = use_signal(|| None::<CanonConfig>);
    let mut version = use_signal(|| None::<String>);
    let modified = use_signal(|| false);
    let mut msg = use_signal(|| None::<(bool, String)>);
    let busy = use_signal(|| false);
    let load_failed = use_signal(|| false);
    let doc = Doc { config, modified };

    let load = {
        let ctx = ctx.clone();
        move || {
            let (
                mut config,
                mut defaults,
                mut version,
                mut modified,
                mut msg,
                mut busy,
                mut load_failed,
            ) = (config, defaults, version, modified, msg, busy, load_failed);
            let ctx = ctx.clone();
            busy.set(true);
            spawn(async move {
                match crate::client::get(&ctx.ep, "/canon-config").await {
                    Ok(r) if r.ok() => match serde_json::from_value::<CanonSnapshot>(r.body) {
                        Ok(s) if s.expected.is_some() => {
                            config.set(Some(s.config));
                            if s.defaults.is_some() {
                                defaults.set(s.defaults);
                            }
                            version.set(s.expected);
                            modified.set(false);
                            msg.set(None);
                            load_failed.set(false);
                        }
                        _ => {
                            version.set(None);
                            load_failed.set(true);
                            msg.set(Some((
                                false,
                                ctx.t("canonEditorSync.loadFailed").to_owned(),
                            )));
                        }
                    },
                    _ => {
                        version.set(None);
                        load_failed.set(true);
                        msg.set(Some((
                            false,
                            ctx.t("canonEditorSync.loadFailed").to_owned(),
                        )));
                    }
                }
                busy.set(false);
            });
        }
    };

    {
        let load = load.clone();
        use_hook(load);
    }

    // The hub pushes the saved version: follow it, or flag a conflict if the draft is modified.
    let mut seen = use_signal(|| expected.clone());
    if expected.is_some() && seen() != expected && config.read().is_some() {
        seen.set(expected.clone());
        if version() != expected {
            if modified() {
                version.set(None);
                msg.set(Some((false, ctx.t("canonEditorSync.conflict").to_owned())));
            } else {
                load();
            }
        }
    }

    let save = {
        let ctx = ctx.clone();
        move |_| {
            let Some(exp) = version() else {
                msg.set(Some((
                    false,
                    ctx.t("canonEditorSync.baselineMissing").to_owned(),
                )));
                return;
            };
            let Some(cfg) = config() else { return };
            let c = ctx.clone();
            ctx.ask(
                ctx.t("adminAi.canon.confirmSaveTitle").to_owned(),
                ctx.tf(
                    "adminAi.canon.confirmSaveBody",
                    &[
                        ("entries", &cfg.entries.len().to_string()),
                        ("heuristics", &cfg.heuristics.len().to_string()),
                    ],
                ),
                ctx.t("adminAi.canon.save").to_owned(),
                true,
                Callback::new(move |_: ()| {
                    let (mut config, mut version, mut modified, mut msg, mut busy) =
                        (config, version, modified, msg, busy);
                    let c = c.clone();
                    let body = CanonSave { config: cfg.clone(), expected: exp.clone() };
                    busy.set(true);
                    spawn(async move {
                        match crate::client::put(&c.ep, "/canon-config", &body).await {
                            Ok(r) if r.ok() => {
                                let saved: CanonSnapshot =
                                    serde_json::from_value(r.body).unwrap_or_default();
                                config.set(Some(saved.config));
                                version.set(saved.expected);
                                modified.set(false);
                                let text = c.t("canonEditorSync.saved").to_owned();
                                c.say(text.clone());
                                msg.set(Some((true, text)));
                            }
                            Ok(r) => {
                                if r.status == 409 || r.status == 428 {
                                    version.set(None);
                                }
                                let key = if r.status == 409 {
                                    "canonEditorSync.conflict"
                                } else {
                                    "canonEditorSync.saveFailed"
                                };
                                msg.set(Some((false, c.t(key).to_owned())));
                            }
                            Err(_) => msg.set(Some((
                                false,
                                c.t("canonEditorSync.networkFailed").to_owned(),
                            ))),
                        }
                        busy.set(false);
                    });
                }),
            );
        }
    };

    let restore = {
        let ctx = ctx.clone();
        move |_| {
            let Some(d) = defaults() else { return };
            let c = ctx.clone();
            ctx.ask(
                ctx.t("adminAi.canon.confirmDefaultsTitle").to_owned(),
                ctx.t("adminAi.canon.confirmDefaultsBody").to_owned(),
                ctx.t("adminAi.canon.restoreDefaults").to_owned(),
                true,
                Callback::new(move |_: ()| {
                    let (mut config, mut modified, mut msg) = (config, modified, msg);
                    config.set(Some(d.clone()));
                    modified.set(true);
                    msg.set(Some((true, c.t("adminAi.canon.defaultsLoaded").to_owned())));
                }),
            );
        }
    };

    let Some(cfg) = config() else {
        return rsx! {
            section { class: "card", "aria-busy": "true",
                h2 { class: "card__title", Glyph { icon: icons::SCALES, size: "md" } "{ctx.t(\"canonEditorSync.editorLabel\")}" }
                if load_failed() {
                    div { class: "notice notice--error", role: "alert",
                        span { class: "notice__text", "{ctx.t(\"adminAi.canon.hidden\")}" }
                        button { r#type: "button", class: "btn btn--sm", onclick: move |_| load(), "{ctx.t(\"adminDashboard.retry\")}" }
                    }
                } else {
                    SkeletonLines { count: 8 }
                }
            }
        };
    };

    let fieldset_disabled = busy();
    rsx! {
        div { class: "stack", role: "group", "aria-label": "{ctx.t(\"canonEditorSync.editorLabel\")}",
            div { class: "card sticky",
                div { class: "spread",
                    div { class: "stack stack--tight",
                        p { class: "small tabular",
                            "{ctx.tf(\"adminAi.canon.counts\", &[(\"entries\", &cfg.entries.len().to_string()), (\"heuristics\", &cfg.heuristics.len().to_string()), (\"references\", &cfg.references.len().to_string())])}"
                        }
                        if modified() {
                            p { class: "small tone-accent", "{ctx.t(\"adminAi.canon.unsaved\")}" }
                        } else {
                            p { class: "small muted", "{ctx.t(\"adminAi.canon.engineKnows\")}" }
                        }
                    }
                    div { class: "row",
                        button { r#type: "button", class: "btn", disabled: fieldset_disabled, onclick: { let load = load.clone(); move |_| load() },
                            Glyph { icon: icons::ARROWS_CLOCKWISE } "{ctx.t(\"canonEditorSync.reload\")}"
                        }
                        button { r#type: "button", class: "btn", disabled: fieldset_disabled || defaults.read().is_none(), onclick: restore,
                            Glyph { icon: icons::ARROW_COUNTER_CLOCKWISE } "{ctx.t(\"adminAi.canon.restoreDefaults\")}"
                        }
                        button { r#type: "button", class: "btn btn--primary", disabled: fieldset_disabled || version.read().is_none() || !modified(), onclick: save,
                            Glyph { icon: if busy() { icons::CIRCLE_NOTCH } else { icons::FLOPPY_DISK } } "{ctx.t(\"adminAi.canon.save\")}"
                        }
                    }
                }
                if let Some((ok, text)) = msg() {
                    p { class: if ok { "small tone-ok row" } else { "small tone-danger row" }, role: "status", "aria-live": "polite",
                        if !ok { Glyph { icon: icons::WARNING_CIRCLE } }
                        "{text}"
                    }
                }
            }
            Presentation { doc, cfg: cfg.clone() }
            Rules { doc, cfg: cfg.clone() }
            Tiers { doc, cfg: cfg.clone() }
            Entries { doc, cfg: cfg.clone() }
            Heuristics { doc, cfg: cfg.clone() }
            References { doc, cfg: cfg.clone() }
            Fallback { doc, cfg: cfg.clone() }
            PowerScaling { cfg: cfg.clone() }
            section { class: "card",
                h3 { class: "card__title card__title--sm", Glyph { icon: icons::MAGNIFYING_GLASS } "{ctx.t(\"adminAi.canon.benchTitle\")}" }
                p { class: "small muted", "{ctx.t(\"adminAi.canon.benchText\")}" }
            }
        }
    }
}

#[component]
fn Section(
    title: String,
    desc: String,
    children: Element,
    #[props(default = true)] open: bool,
) -> Element {
    rsx! {
        details { class: "card details", open,
            summary { span { class: "card__title card__title--sm", "{title}" } }
            p { class: "card__desc", "{desc}" }
            {children}
        }
    }
}

#[component]
fn Presentation(doc: Doc, cfg: CanonConfig) -> Element {
    let ctx = use_ctx();
    rsx! {
        Section { title: ctx.t("adminAi.canon.presentationTitle").to_owned(), desc: ctx.t("adminAi.canon.presentationDesc").to_owned(),
            TextField { id: "canon-intro".to_owned(), label: ctx.t("adminAi.canon.intro").to_owned(), value: cfg.intro.clone(), multiline: true, rows: 3,
                on_input: move |v: String| doc.edit(move |c| c.intro = v) }
            TextField { id: "canon-placeholder".to_owned(), label: ctx.t("adminAi.canon.placeholder").to_owned(), value: cfg.placeholder.clone(),
                on_input: move |v: String| doc.edit(move |c| c.placeholder = v) }
        }
    }
}

#[component]
fn Rules(doc: Doc, cfg: CanonConfig) -> Element {
    let ctx = use_ctx();
    let hint = ctx.t("adminAi.canon.onePerLine").to_owned();
    rsx! {
        Section { title: ctx.t("adminAi.canon.rulesTitle").to_owned(), desc: ctx.t("adminAi.canon.rulesDesc").to_owned(),
            TextField { id: "canon-doctrine".to_owned(), label: ctx.t("adminAi.canon.doctrine").to_owned(), value: lines(&cfg.doctrine), multiline: true, rows: 6, hint: Some(hint.clone()),
                on_input: move |v: String| doc.edit(move |c| c.doctrine = from_lines(&v)) }
            div { class: "form__row",
                TextField { id: "canon-allowed".to_owned(), label: ctx.t("adminAi.canon.allowed").to_owned(), value: lines(&cfg.sources_admises), multiline: true, rows: 6, hint: Some(hint.clone()),
                    on_input: move |v: String| doc.edit(move |c| c.sources_admises = from_lines(&v)) }
                TextField { id: "canon-forbidden".to_owned(), label: ctx.t("adminAi.canon.forbidden").to_owned(), value: lines(&cfg.sources_interdites), multiline: true, rows: 6, hint: Some(hint),
                    on_input: move |v: String| doc.edit(move |c| c.sources_interdites = from_lines(&v)) }
            }
        }
    }
}

#[component]
fn ScoreField(id: String, label: String, value: f64, on_change: EventHandler<f64>) -> Element {
    let ctx = use_ctx();
    let err = score_error(&ctx, value);
    let err_id = format!("{id}-error");
    rsx! {
        div { class: "field field--narrow",
            label { class: "field__label", r#for: "{id}", "{label}" }
            input { id: "{id}", class: "input tabular", r#type: "number", min: "0", max: "100", step: "1", value: "{value}",
                "aria-invalid": if err.is_some() { "true" } else { "false" },
                "aria-describedby": "{err_id}",
                oninput: move |e| { if let Ok(v) = e.value().parse::<f64>() { on_change.call(v) } },
            }
            if let Some(e) = err.clone() { p { id: "{err_id}", class: "field__error", "{e}" } }
        }
    }
}

#[component]
fn TierSelect(
    id: String,
    label: String,
    value: String,
    on_change: EventHandler<String>,
) -> Element {
    rsx! {
        div { class: "field field--mid",
            label { class: "field__label", r#for: "{id}", "{label}" }
            select { id: "{id}", class: "select", value: "{value}", onchange: move |e| on_change.call(e.value()),
                for t in CANON_TIERS { option { key: "{t}", value: "{t}", selected: t == value, "{t}" } }
            }
        }
    }
}

#[component]
fn Tiers(doc: Doc, cfg: CanonConfig) -> Element {
    let ctx = use_ctx();
    rsx! {
        Section { title: ctx.t("adminAi.canon.tiersTitle").to_owned(), desc: ctx.t("adminAi.canon.tiersDesc").to_owned(), open: false,
            for (i, tier) in cfg.tiers.iter().enumerate() {
                div { key: "{tier.id}", class: "item",
                    div { class: "item__head",
                        span { class: "row strong", span { class: "swatch", style: "background:{tier.couleur}" } "{tier.id}" }
                    }
                    div { class: "form__row",
                        TextField { id: format!("tier-{i}-label"), label: ctx.t("adminAi.canon.tierLabel").to_owned(), value: tier.label.clone(), on_input: move |v: String| doc.edit(move |c| c.tiers[i].label = v) }
                        TextField { id: format!("tier-{i}-short"), label: ctx.t("adminAi.canon.tierShort").to_owned(), value: tier.court.clone(), on_input: move |v: String| doc.edit(move |c| c.tiers[i].court = v) }
                        div { class: "field field--narrow",
                            label { class: "field__label", r#for: "tier-{i}-color", "{ctx.t(\"adminAi.canon.tierColor\")}" }
                            input { id: "tier-{i}-color", class: "input", r#type: "color", value: "{tier.couleur}", oninput: move |e| { let v = e.value(); doc.edit(move |c| c.tiers[i].couleur = v) } }
                        }
                        ScoreField { id: format!("tier-{i}-score"), label: ctx.t("adminAi.canon.score").to_owned(), value: tier.score, on_change: move |v| doc.edit(move |c| c.tiers[i].score = v) }
                    }
                    TextField { id: format!("tier-{i}-def"), label: ctx.t("adminAi.canon.tierDefinition").to_owned(), value: tier.definition.clone(), multiline: true, rows: 3, on_input: move |v: String| doc.edit(move |c| c.tiers[i].definition = v) }
                    TextField { id: format!("tier-{i}-incl"), label: ctx.t("adminAi.canon.tierIncludes").to_owned(), value: lines(&tier.inclut), multiline: true, rows: 3, hint: Some(ctx.t("adminAi.canon.onePerLine").to_owned()), on_input: move |v: String| doc.edit(move |c| c.tiers[i].inclut = from_lines(&v)) }
                }
            }
        }
    }
}

#[component]
fn Entries(doc: Doc, cfg: CanonConfig) -> Element {
    let ctx = use_ctx();
    let mut filter = use_signal(String::new);
    let mut limit = use_signal(|| PAGE);
    let f = filter().trim().to_lowercase();
    let matches: Vec<usize> = cfg
        .entries
        .iter()
        .enumerate()
        .filter(|(_, e)| {
            f.is_empty()
                || format!("{} {} {}", e.title, e.id, e.keywords.join(" "))
                    .to_lowercase()
                    .contains(&f)
        })
        .map(|(i, _)| i)
        .collect();
    let total = matches.len();
    let shown: Vec<usize> = matches.into_iter().take(limit()).collect();
    rsx! {
        Section { title: ctx.t("adminAi.canon.entriesTitle").to_owned(), desc: ctx.t("adminAi.canon.entriesDesc").to_owned(),
            div { class: "form__row",
                TextField { id: "entries-filter".to_owned(), label: ctx.t("adminAi.canon.filter").to_owned(), value: filter(), on_input: move |v| { filter.set(v); limit.set(PAGE); } }
                button { r#type: "button", class: "btn",
                    onclick: move |_| doc.edit(|c| c.entries.insert(0, CanonEntry { category: "lore".into(), tier: "CANON_ABSOLU".into(), score: 100.0, ..Default::default() })),
                    Glyph { icon: icons::PLUS } "{ctx.t(\"adminAi.canon.addEntry\")}"
                }
            }
            p { class: "small muted", role: "status", "{ctx.tf(\"adminAi.canon.shown\", &[(\"shown\", &shown.len().to_string()), (\"total\", &total.to_string())])}" }
            if total == 0 { Empty { text: ctx.t("adminAi.canon.noMatch").to_owned() } }
            for i in shown.iter().copied() {
                EntryEditor { key: "{i}-{cfg.entries[i].id}", doc, index: i, entry: cfg.entries[i].clone() }
            }
            if total > limit() {
                button { r#type: "button", class: "btn", onclick: move |_| limit.set(limit() + PAGE), "{ctx.tf(\"adminAi.canon.more\", &[(\"count\", &(total - limit()).min(PAGE).to_string())])}" }
            }
        }
    }
}

#[component]
fn EntryEditor(doc: Doc, index: usize, entry: CanonEntry) -> Element {
    let ctx = use_ctx();
    let i = index;
    let title = if entry.title.is_empty() {
        ctx.t("adminAi.canon.untitled").to_owned()
    } else {
        entry.title.clone()
    };
    let title_err = entry
        .title
        .trim()
        .is_empty()
        .then(|| ctx.t("adminAi.canon.titleRequired").to_owned());
    let del = {
        let ctx = ctx.clone();
        let title = title.clone();
        move |_| {
            ctx.ask(
                ctx.t("adminAi.canon.confirmDeleteTitle").to_owned(),
                ctx.tf("adminAi.canon.confirmDeleteBody", &[("title", &title)]),
                ctx.t("adminAi.canon.delete").to_owned(),
                true,
                Callback::new(move |_: ()| {
                    doc.edit(move |c| {
                        if i < c.entries.len() {
                            c.entries.remove(i);
                        }
                    })
                }),
            );
        }
    };
    rsx! {
        details { class: "item details",
            summary {
                span { class: "badge badge--{crate::model::tier_tone(&entry.tier)}", "{entry.tier}" }
                span { class: "break", "{title}" }
                span { class: "mono muted", "{entry.score}" }
            }
            div { class: "form__row",
                TextField { id: format!("entry-{i}-title"), label: ctx.t("adminAi.canon.entryTitle").to_owned(), value: entry.title.clone(), error: title_err, on_input: move |v: String| doc.edit(move |c| c.entries[i].title = v) }
                TextField { id: format!("entry-{i}-id"), label: ctx.t("adminAi.canon.entryId").to_owned(), value: entry.id.clone(), mono: true, on_input: move |v: String| doc.edit(move |c| c.entries[i].id = v) }
            }
            div { class: "form__row",
                div { class: "field field--mid",
                    label { class: "field__label", r#for: "entry-{i}-cat", "{ctx.t(\"adminAi.canon.category\")}" }
                    select { id: "entry-{i}-cat", class: "select", value: "{entry.category}", onchange: move |e| { let v = e.value(); doc.edit(move |c| c.entries[i].category = v) },
                        for cat in CANON_CATEGORIES {
                            {
                                let label = ctx.t(&format!("adminAi.canon.cat.{cat}"));
                                rsx! { option { key: "{cat}", value: "{cat}", selected: cat == entry.category, "{label}" } }
                            }
                        }
                    }
                }
                TierSelect { id: format!("entry-{i}-tier"), label: ctx.t("adminAi.canon.tier").to_owned(), value: entry.tier.clone(), on_change: move |v: String| doc.edit(move |c| c.entries[i].tier = v) }
                ScoreField { id: format!("entry-{i}-score"), label: ctx.t("adminAi.canon.score").to_owned(), value: entry.score, on_change: move |v| doc.edit(move |c| c.entries[i].score = v) }
            }
            TextField { id: format!("entry-{i}-kw"), label: ctx.t("adminAi.canon.keywords").to_owned(), value: entry.keywords.join(", "), hint: Some(ctx.t("adminAi.canon.commaHint").to_owned()), on_input: move |v: String| doc.edit(move |c| c.entries[i].keywords = from_commas(&v)) }
            TextField { id: format!("entry-{i}-verdict"), label: ctx.t("adminAi.canon.verdict").to_owned(), value: entry.verdict.clone(), multiline: true, rows: 2, on_input: move |v: String| doc.edit(move |c| c.entries[i].verdict = v) }
            TextField { id: format!("entry-{i}-expl"), label: ctx.t("adminAi.canon.explanation").to_owned(), value: entry.explanation.clone(), multiline: true, rows: 3, on_input: move |v: String| doc.edit(move |c| c.entries[i].explanation = v) }
            TextField { id: format!("entry-{i}-src"), label: ctx.t("adminAi.canon.sources").to_owned(), value: lines(&entry.sources), multiline: true, rows: 2, hint: Some(ctx.t("adminAi.canon.onePerLine").to_owned()), on_input: move |v: String| doc.edit(move |c| c.entries[i].sources = from_lines(&v)) }
            if let Some(q) = entry.toriyama_quote.as_ref() {
                blockquote { class: "quote", "« {q.text} »" p { class: "quote__by", "{ctx.tf(\"adminAi.bench.quoteBy\", &[(\"source\", &q.source)])}" } }
            }
            div { class: "row", button { r#type: "button", class: "btn btn--sm btn--danger", onclick: del, Glyph { icon: icons::TRASH } "{ctx.t(\"adminAi.canon.delete\")}" } }
        }
    }
}

#[component]
fn Heuristics(doc: Doc, cfg: CanonConfig) -> Element {
    let ctx = use_ctx();
    rsx! {
        Section { title: ctx.t("adminAi.canon.heuristicsTitle").to_owned(), desc: ctx.t("adminAi.canon.heuristicsDesc").to_owned(), open: false,
            for (i, h) in cfg.heuristics.iter().enumerate() {
                details { key: "{i}-{h.id}", class: "item details",
                    summary { span { class: "badge badge--{crate::model::tier_tone(&h.tier)}", "{h.tier}" } span { class: "break", "{h.label}" } }
                    div { class: "form__row",
                        TextField { id: format!("heur-{i}-label"), label: ctx.t("adminAi.canon.heuristicLabel").to_owned(), value: h.label.clone(), on_input: move |v: String| doc.edit(move |c| c.heuristics[i].label = v) }
                        TierSelect { id: format!("heur-{i}-tier"), label: ctx.t("adminAi.canon.tier").to_owned(), value: h.tier.clone(), on_change: move |v: String| doc.edit(move |c| c.heuristics[i].tier = v) }
                        ScoreField { id: format!("heur-{i}-score"), label: ctx.t("adminAi.canon.score").to_owned(), value: h.score, on_change: move |v| doc.edit(move |c| c.heuristics[i].score = v) }
                    }
                    TextField { id: format!("heur-{i}-kw"), label: ctx.t("adminAi.canon.keywords").to_owned(), value: h.keywords.join(", "), hint: Some(ctx.t("adminAi.canon.commaHint").to_owned()), on_input: move |v: String| doc.edit(move |c| c.heuristics[i].keywords = from_commas(&v)) }
                    TextField { id: format!("heur-{i}-verdict"), label: ctx.t("adminAi.canon.verdict").to_owned(), value: h.verdict.clone(), multiline: true, rows: 2, on_input: move |v: String| doc.edit(move |c| c.heuristics[i].verdict = v) }
                    TextField { id: format!("heur-{i}-expl"), label: ctx.t("adminAi.canon.explanation").to_owned(), value: h.explanation.clone(), multiline: true, rows: 2, on_input: move |v: String| doc.edit(move |c| c.heuristics[i].explanation = v) }
                }
            }
        }
    }
}

#[component]
fn References(doc: Doc, cfg: CanonConfig) -> Element {
    let ctx = use_ctx();
    let mut filter = use_signal(String::new);
    let mut all = use_signal(|| false);
    let f = filter().trim().to_lowercase();
    let mut by_source: Vec<(String, usize)> = Vec::new();
    for r in &cfg.references {
        match by_source.iter_mut().find(|(s, _)| *s == r.source) {
            Some(e) => e.1 += 1,
            None => by_source.push((r.source.clone(), 1)),
        }
    }
    by_source.sort_by_key(|b| std::cmp::Reverse(b.1));
    let matching: Vec<usize> = cfg
        .references
        .iter()
        .enumerate()
        .filter(|(_, r)| {
            f.is_empty()
                || format!(
                    "{} {} {} {}",
                    r.subject,
                    r.keywords.join(" "),
                    r.text,
                    r.source
                )
                .to_lowercase()
                .contains(&f)
        })
        .map(|(i, _)| i)
        .collect();
    let total = matching.len();
    let shown: Vec<usize> = if all() {
        matching
    } else {
        matching.into_iter().take(30).collect()
    };
    rsx! {
        Section { title: ctx.t("adminAi.canon.referencesTitle").to_owned(), desc: ctx.t("adminAi.canon.referencesDesc").to_owned(), open: false,
            if !by_source.is_empty() {
                p { class: "small muted", {by_source.iter().take(8).map(|(s, n)| format!("{s} ({n})")).collect::<Vec<_>>().join(" · ")} }
            }
            div { class: "form__row",
                TextField { id: "refs-filter".to_owned(), label: ctx.t("adminAi.canon.filter").to_owned(), value: filter(), on_input: move |v| filter.set(v) }
                if total > 30 {
                    button { r#type: "button", class: "btn", "aria-pressed": if all() { "true" } else { "false" }, onclick: move |_| all.set(!all()), "{ctx.t(\"adminAi.canon.showAll\")}" }
                }
            }
            p { class: "small muted", role: "status", "{ctx.tf(\"adminAi.canon.shown\", &[(\"shown\", &shown.len().to_string()), (\"total\", &total.to_string())])}" }
            if total == 0 { Empty { text: ctx.t("adminAi.canon.noReferences").to_owned() } }
            ul { class: "stack plain",
                for i in shown.iter().copied() {
                    {
                        let r = cfg.references[i].clone();
                        let hidden = r.visible == Some(false);
                        let id = r.id.clone();
                        let c = ctx.clone();
                        let subject = r.subject.clone();
                        rsx! {
                            li { key: "{r.id}", class: "item",
                                div { class: "item__head",
                                    span { class: "strong break", "{r.subject}" }
                                    span { class: "row",
                                        span { class: "badge badge--neutral", "{r.provenance}" }
                                        span { class: "badge badge--neutral", "{r.lang}" }
                                        if hidden { span { class: "badge badge--warn", "{ctx.t(\"adminAi.canon.hiddenRef\")}" } }
                                    }
                                }
                                blockquote { class: "quote", lang: "{r.lang}", "« {r.text} »" }
                                if let Some(fr) = r.text_fr.as_ref() { p { class: "small", lang: "fr", "{fr}" } }
                                p { class: "quote__by", "{r.source}" if let Some(y) = r.year { " ({y})" } }
                                div { class: "row",
                                    button { r#type: "button", class: if hidden { "btn btn--sm" } else { "btn btn--sm btn--danger" },
                                        onclick: move |_| {
                                            let id = id.clone();
                                            if hidden {
                                                doc.edit(move |cf| { if let Some(x) = cf.references.iter_mut().find(|x| x.id == id) { x.visible = Some(true); } });
                                            } else {
                                                c.ask(
                                                    c.t("adminAi.canon.confirmHideTitle").to_owned(),
                                                    c.tf("adminAi.canon.confirmHideBody", &[("subject", &subject)]),
                                                    c.t("adminAi.canon.hide").to_owned(),
                                                    true,
                                                    Callback::new(move |_: ()| {
                                                        let id = id.clone();
                                                        doc.edit(move |cf| { if let Some(x) = cf.references.iter_mut().find(|x| x.id == id) { x.visible = Some(false); } });
                                                    }),
                                                );
                                            }
                                        },
                                        Glyph { icon: if hidden { icons::EYE } else { icons::EYE_SLASH } }
                                        if hidden { "{ctx.t(\"adminAi.canon.show\")}" } else { "{ctx.t(\"adminAi.canon.hide\")}" }
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
fn Fallback(doc: Doc, cfg: CanonConfig) -> Element {
    let ctx = use_ctx();
    rsx! {
        Section { title: ctx.t("adminAi.canon.fallbackTitle").to_owned(), desc: ctx.t("adminAi.canon.fallbackDesc").to_owned(), open: false,
            div { class: "form__row",
                TierSelect { id: "fallback-tier".to_owned(), label: ctx.t("adminAi.canon.tier").to_owned(), value: cfg.fallback.tier.clone(), on_change: move |v: String| doc.edit(move |c| c.fallback.tier = v) }
                ScoreField { id: "fallback-score".to_owned(), label: ctx.t("adminAi.canon.score").to_owned(), value: cfg.fallback.score, on_change: move |v| doc.edit(move |c| c.fallback.score = v) }
            }
            TextField { id: "fallback-verdict".to_owned(), label: ctx.t("adminAi.canon.verdict").to_owned(), value: cfg.fallback.verdict.clone(), multiline: true, rows: 2, on_input: move |v: String| doc.edit(move |c| c.fallback.verdict = v) }
            TextField { id: "fallback-expl".to_owned(), label: ctx.t("adminAi.canon.explanation").to_owned(), value: cfg.fallback.explanation.clone(), multiline: true, rows: 2, on_input: move |v: String| doc.edit(move |c| c.fallback.explanation = v) }
            TextField { id: "empty-verdict".to_owned(), label: ctx.t("adminAi.canon.emptyVerdict").to_owned(), value: cfg.vide.verdict.clone(), multiline: true, rows: 2, on_input: move |v: String| doc.edit(move |c| c.vide.verdict = v) }
            TextField { id: "empty-expl".to_owned(), label: ctx.t("adminAi.canon.emptyExplanation").to_owned(), value: cfg.vide.explanation.clone(), multiline: true, rows: 2, on_input: move |v: String| doc.edit(move |c| c.vide.explanation = v) }
        }
    }
}

#[component]
fn PowerScaling(cfg: CanonConfig) -> Element {
    let ctx = use_ctx();
    let ps = &cfg.power_scaling;
    let count = |k: &str| {
        ps.get(k)
            .and_then(|v| v.as_array())
            .map_or(0, Vec::len)
            .to_string()
    };
    let enabled = ps.get("enabled").and_then(|v| v.as_bool()).unwrap_or(false);
    let rate = ps
        .get("kiriToBpRate")
        .map(|v| v.to_string())
        .unwrap_or_else(|| "—".into());
    let state = ctx.t(if enabled {
        "adminAi.flag.on"
    } else {
        "adminAi.flag.off"
    });
    let summary = ctx.tf(
        "adminAi.canon.powerSummary",
        &[
            ("state", state),
            ("multipliers", &count("multipliers")),
            ("bps", &count("officialBps")),
            ("tiers", &count("tiers")),
            ("rate", &rate),
        ],
    );
    rsx! {
        Section { title: ctx.t("adminAi.canon.powerTitle").to_owned(), desc: ctx.t("adminAi.canon.powerDesc").to_owned(), open: false,
            p { class: "small tabular", "{summary}" }
            p { class: "small muted", "{ctx.t(\"adminAi.canon.powerReadOnly\")}" }
        }
    }
}

#[allow(dead_code)]
fn _notice_kind_used(_: NoticeKind) {}
