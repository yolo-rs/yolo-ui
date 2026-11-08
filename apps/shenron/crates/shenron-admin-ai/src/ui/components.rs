//! Shared building blocks: icons, badges, source rows, skeletons, notices, fields, dialog.

use super::{Confirm, Notice, NoticeKind, use_ctx};
use crate::icons::{self, Icon};
use crate::model::{SourceMeta, SourceStatus};
use dioxus::prelude::*;

/// Inline Phosphor glyph. Decorative by default (`aria-hidden`); pass `label` to name it.
#[component]
pub fn Glyph(
    icon: Icon,
    #[props(default)] size: &'static str,
    #[props(default)] label: Option<String>,
) -> Element {
    let class = match size {
        "md" => "icon icon--md",
        "lg" => "icon icon--lg",
        _ => "icon",
    };
    let hidden = if label.is_some() { "false" } else { "true" };
    let role = if label.is_some() {
        "img"
    } else {
        "presentation"
    };
    rsx! {
        svg {
            class: "{class}",
            view_box: icons::VIEW_BOX,
            fill: "currentColor",
            "aria-hidden": hidden,
            role,
            "focusable": "false",
            if let Some(l) = label { title { "{l}" } }
            for d in icon.0.iter() {
                path { d: *d }
            }
        }
    }
}

pub fn status_key(status: SourceStatus) -> &'static str {
    match status {
        SourceStatus::Ok => "adminAi.status.ok",
        SourceStatus::Unavailable => "adminAi.status.unavailable",
        SourceStatus::Unconfigured => "adminAi.status.unconfigured",
    }
}

pub fn status_tone(status: SourceStatus) -> &'static str {
    match status {
        SourceStatus::Ok => "ok",
        SourceStatus::Unavailable => "danger",
        SourceStatus::Unconfigured => "warn",
    }
}

/// Status badge: colour plus icon plus text (never colour alone).
#[component]
pub fn StatusBadge(status: SourceStatus) -> Element {
    let ctx = use_ctx();
    let icon = match status {
        SourceStatus::Ok => icons::CHECK_CIRCLE,
        SourceStatus::Unavailable => icons::WARNING_CIRCLE,
        SourceStatus::Unconfigured => icons::WARNING,
    };
    rsx! {
        span { class: "badge badge--{status_tone(status)}",
            Glyph { icon }
            "{ctx.t(status_key(status))}"
        }
    }
}

/// On / off capability flag (Aphrody health).
#[component]
pub fn Flag(label: String, on: bool) -> Element {
    let ctx = use_ctx();
    let (tone, key, icon) = if on {
        ("ok", "adminAi.flag.on", icons::CHECK_CIRCLE)
    } else {
        ("danger", "adminAi.flag.off", icons::WARNING_CIRCLE)
    };
    rsx! {
        span { class: "badge badge--{tone}", Glyph { icon } "{label} · {ctx.t(key)}" }
    }
}

/// Owned copy of a source's metadata, for props.
#[derive(Clone, PartialEq, Debug)]
pub struct Meta {
    pub status: SourceStatus,
    pub label: String,
    pub target: String,
    pub latency_ms: Option<f64>,
    pub error: Option<String>,
}

impl From<SourceMeta<'_>> for Meta {
    fn from(m: SourceMeta<'_>) -> Self {
        Meta {
            status: m.status,
            label: m.label.to_owned(),
            target: m.target.to_owned(),
            latency_ms: m.latency_ms,
            error: m.error.map(str::to_owned),
        }
    }
}

/// One data source: label, latency, status, target, then its details or its error.
#[component]
pub fn SourceRow(meta: Meta, children: Element) -> Element {
    let ctx = use_ctx();
    let fallback = ctx.t("adminAi.status.unavailableDefault");
    rsx! {
        div { class: "card card--inset",
            div { class: "spread",
                span { class: "strong", "{meta.label}" }
                div { class: "row",
                    if let Some(ms) = meta.latency_ms {
                        span { class: "mono muted", "{crate::format::ms(ctx.lang, ms)}" }
                    }
                    StatusBadge { status: meta.status }
                }
            }
            div { class: "mono muted break", "{meta.target}" }
            if meta.status == SourceStatus::Ok {
                {children}
            } else {
                p { class: "small tone-danger", "{meta.error.clone().unwrap_or_else(|| fallback.to_owned())}" }
            }
        }
    }
}

/// Inline error for a failed source, with a retry that asks the hub to re-read now.
#[component]
pub fn SourceError(meta: Meta) -> Element {
    let ctx = use_ctx();
    if meta.status == SourceStatus::Ok {
        return rsx! {};
    }
    let tone = if meta.status == SourceStatus::Unconfigured {
        "warn"
    } else {
        "error"
    };
    let text = format!(
        "{} — {} : {}",
        meta.label,
        ctx.t(status_key(meta.status)),
        meta.error
            .clone()
            .unwrap_or_else(|| ctx.t("adminAi.status.unavailableDefault").to_owned())
    );
    rsx! {
        div { class: "notice notice--{tone}",
            span { class: "notice__text", Glyph { icon: icons::WARNING } "{text}" }
            RetryButton {}
        }
    }
}

/// Asks the hub for an immediate upstream read (the result arrives over the stream).
#[component]
pub fn RetryButton() -> Element {
    let ctx = use_ctx();
    let label = ctx.t("adminDashboard.retry");
    rsx! {
        button {
            r#type: "button",
            class: "btn btn--sm",
            onclick: move |_| {
                let ctx = ctx.clone();
                spawn(async move { super::refresh(&ctx).await; });
            },
            Glyph { icon: icons::ARROWS_CLOCKWISE }
            "{label}"
        }
    }
}

#[component]
pub fn SkeletonLines(#[props(default = 3)] count: usize) -> Element {
    rsx! {
        div { class: "stack stack--tight", "aria-hidden": "true",
            for i in 0..count {
                div { key: "{i}", class: "skeleton skeleton--line" }
            }
        }
    }
}

/// Big number with label and hint; skeleton while nothing has arrived yet.
#[component]
pub fn Metric(
    label: String,
    value: Option<String>,
    #[props(default)] tone: &'static str,
    #[props(default)] text: bool,
    hint: Element,
) -> Element {
    let value_class = if text {
        "metric__value metric__value--text"
    } else {
        "metric__value tabular"
    };
    rsx! {
        div { class: "metric",
            div { class: "metric__label", "{label}" }
            match value {
                Some(v) => rsx! { div { class: "{value_class} tone-{tone}", "{v}" } },
                None => rsx! { div { class: "skeleton skeleton--value", "aria-hidden": "true" } },
            }
            div { class: "metric__hint", {hint} }
        }
    }
}

#[component]
pub fn NoticeBar(notice: Notice, on_close: EventHandler<()>) -> Element {
    let ctx = use_ctx();
    let (tone, icon) = match notice.kind {
        NoticeKind::Ok => ("ok", icons::CHECK_CIRCLE),
        NoticeKind::Info => ("info", icons::INFO),
        NoticeKind::Error => ("error", icons::WARNING_CIRCLE),
    };
    let close = ctx.t("adminAi.action.close");
    rsx! {
        div { class: "notice notice--{tone}", role: if notice.kind == NoticeKind::Error { "alert" } else { "status" },
            span { class: "notice__text", Glyph { icon } "{notice.text}" }
            button { r#type: "button", class: "btn btn--sm btn--ghost", "aria-label": "{close}", onclick: move |_| on_close.call(()),
                Glyph { icon: icons::X }
            }
        }
    }
}

/// Labelled text field with its hint and error next to it (`aria-describedby`, `aria-invalid`).
#[component]
pub fn TextField(
    id: String,
    label: String,
    value: String,
    on_input: EventHandler<String>,
    #[props(default)] placeholder: String,
    #[props(default)] hint: Option<String>,
    #[props(default)] error: Option<String>,
    #[props(default)] multiline: bool,
    #[props(default)] mono: bool,
    #[props(default = 1)] rows: u32,
) -> Element {
    let hint_id = format!("{id}-hint");
    let err_id = format!("{id}-error");
    let described = match (&hint, &error) {
        (Some(_), Some(_)) => format!("{hint_id} {err_id}"),
        (Some(_), None) => hint_id.clone(),
        (None, Some(_)) => err_id.clone(),
        (None, None) => String::new(),
    };
    let invalid = if error.is_some() { "true" } else { "false" };
    let extra = if mono { " mono" } else { "" };
    rsx! {
        div { class: "field",
            label { class: "field__label", r#for: "{id}", "{label}" }
            if multiline {
                textarea {
                    id: "{id}",
                    class: "textarea{extra}",
                    rows: "{rows}",
                    placeholder: "{placeholder}",
                    value: "{value}",
                    "aria-invalid": invalid,
                    "aria-describedby": "{described}",
                    oninput: move |e| on_input.call(e.value()),
                }
            } else {
                input {
                    id: "{id}",
                    class: "input{extra}",
                    r#type: "text",
                    placeholder: "{placeholder}",
                    value: "{value}",
                    "aria-invalid": invalid,
                    "aria-describedby": "{described}",
                    oninput: move |e| on_input.call(e.value()),
                }
            }
            if let Some(h) = hint { p { id: "{hint_id}", class: "field__hint", "{h}" } }
            if let Some(e) = error.clone() { p { id: "{err_id}", class: "field__error", Glyph { icon: icons::WARNING_CIRCLE } "{e}" } }
        }
    }
}

/// Modal confirmation for destructive or irreversible writes. Escape and the cancel button
/// close it; focus starts on cancel, Tab stays inside, and returns to the trigger afterwards.
#[component]
pub fn ConfirmDialog(confirm: Confirm) -> Element {
    let ctx = use_ctx();
    let slot = ctx.confirm;
    let mut cancel_el: Signal<Option<std::rc::Rc<MountedData>>> = use_signal(|| None);
    let mut ok_el: Signal<Option<std::rc::Rc<MountedData>>> = use_signal(|| None);
    let close = move || {
        let mut slot = slot;
        slot.set(None);
        crate::client::restore_focus();
    };
    let on_confirm = confirm.on_confirm;
    let cancel_label = ctx.t("admin.confirm.cancel");
    let btn = if confirm.danger {
        "btn btn--danger"
    } else {
        "btn btn--primary"
    };
    rsx! {
        div { class: "dialog-backdrop",
            onclick: move |_| close(),
            div {
                class: "dialog",
                role: "alertdialog",
                "aria-modal": "true",
                "aria-labelledby": "confirm-title",
                "aria-describedby": "confirm-body",
                onclick: move |e| e.stop_propagation(),
                onkeydown: move |e: KeyboardEvent| {
                    let key = e.key().to_string();
                    if key == "Escape" {
                        e.prevent_default();
                        close();
                    } else if key == "Tab" {
                        // Two focusable controls: keep focus on them.
                        e.prevent_default();
                        let target = if e.modifiers().shift() { cancel_el() } else { ok_el() };
                        let other = if e.modifiers().shift() { ok_el() } else { cancel_el() };
                        spawn(async move {
                            if let Some(el) = target.clone() {
                                if el.set_focus(true).await.is_err() {
                                    if let Some(o) = other { let _ = o.set_focus(true).await; }
                                }
                            }
                        });
                    }
                },
                h2 { id: "confirm-title", class: "dialog__title", "{confirm.title}" }
                p { id: "confirm-body", class: "pre", "{confirm.body}" }
                div { class: "dialog__actions",
                    button {
                        r#type: "button",
                        class: "btn",
                        onmounted: move |e| {
                            let el = e.data();
                            cancel_el.set(Some(el.clone()));
                            spawn(async move { let _ = el.set_focus(true).await; });
                        },
                        onclick: move |_| close(),
                        "{cancel_label}"
                    }
                    button {
                        r#type: "button",
                        class: "{btn}",
                        onmounted: move |e| ok_el.set(Some(e.data())),
                        onclick: move |_| {
                            close();
                            on_confirm.call(());
                        },
                        "{confirm.confirm_label}"
                    }
                }
            }
        }
    }
}

/// Empty state with an explicit sentence.
#[component]
pub fn Empty(text: String) -> Element {
    rsx! { div { class: "empty", "{text}" } }
}
