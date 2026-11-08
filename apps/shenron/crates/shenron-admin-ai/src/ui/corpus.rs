//! Tab 4: editorial corpus links, live corpus status (pushed by the hub), Aphrody OCR probe.

use super::components::{Glyph, SkeletonLines};
use super::{NoticeKind, run_action, use_ctx};
use crate::format;
use crate::icons;
use crate::model::{SourceStatus, display_state};
use crate::protocol::CockpitAction;
use base64::Engine;
use dioxus::prelude::*;

const LINKS: [(&str, &str, &str); 4] = [
    (
        "/workshop/transcription",
        "adminAi.corpus.mangaTitle",
        "adminAi.corpus.mangaText",
    ),
    (
        "/workshop/translation",
        "adminAi.corpus.databooksTitle",
        "adminAi.corpus.databooksText",
    ),
    (
        "/admin/db-universe/databooks",
        "adminAi.corpus.catalogTitle",
        "adminAi.corpus.catalogText",
    ),
    (
        "/admin/ai?tab=canon",
        "adminAi.corpus.canonTitle",
        "adminAi.corpus.canonText",
    ),
];

#[component]
pub fn Corpus() -> Element {
    let ctx = use_ctx();
    rsx! {
        section { class: "stack", "aria-labelledby": "corpus-title",
            div { class: "stack stack--tight",
                h2 { id: "corpus-title", class: "card__title", Glyph { icon: icons::BOOK_OPEN, size: "md" } "{ctx.t(\"adminAi.corpus.title\")}" }
                p { class: "card__desc", "{ctx.t(\"adminAi.corpus.desc\")}" }
            }
            div { class: "grid grid--metrics",
                for (href, title, text) in LINKS {
                    a { key: "{href}", href: "{href}", class: "card card--link",
                        div { class: "spread", span { class: "card__title card__title--sm", "{ctx.t(title)}" } Glyph { icon: icons::ARROW_SQUARE_OUT } }
                        p { class: "small muted", "{ctx.t(text)}" }
                    }
                }
            }
            LiveStatus {}
            OcrProbe {}
        }
    }
}

#[component]
fn LiveStatus() -> Element {
    let ctx = use_ctx();
    let corpus = ctx.state.read().corpus.clone();
    rsx! {
        section { class: "card", "aria-labelledby": "corpus-live",
            h3 { id: "corpus-live", class: "card__title card__title--sm", Glyph { icon: icons::PULSE } "{ctx.t(\"adminAi.corpus.liveTitle\")}" }
            match corpus {
                None => rsx! { p { class: "sr-only", role: "status", "{ctx.t(\"adminAi.corpus.measuring\")}" } SkeletonLines { count: 2 } },
                Some(c) => {
                    let mut errors = Vec::new();
                    if c.transcription.status != SourceStatus::Ok {
                        errors.push(ctx.tf("adminAi.corpus.errTranscriptions", &[("error", c.transcription.error.as_deref().unwrap_or("—"))]));
                    }
                    if c.pipeline.status != SourceStatus::Ok {
                        errors.push(ctx.tf("adminAi.corpus.errPipeline", &[("error", c.pipeline.error.as_deref().unwrap_or("—"))]));
                    }
                    let t = c.transcription.ok_data();
                    let books = t.and_then(|t| t.databooks.as_ref()).and_then(|d| d.transcribed_pages);
                    let manga = t.and_then(|t| t.manga.as_ref()).and_then(|m| m.ocr_with_text.or(m.ocr_pages));
                    let p = c.pipeline.ok_data();
                    let state = p
                        .and_then(|p| p.databooks.as_ref().and_then(|d| d.state.as_ref()).or(p.manga.as_ref().and_then(|m| m.state.as_ref())))
                        .and_then(display_state)
                        .unwrap_or_else(|| "—".into());
                    rsx! {
                        for e in errors.iter() { p { key: "{e}", class: "small tone-danger", role: "alert", "{e}" } }
                        div { class: "grid grid--metrics",
                            div { class: "metric", div { class: "metric__label", "{ctx.t(\"adminAi.corpus.databookPages\")}" } div { class: "metric__value tabular", "{ctx.int(books)}" } }
                            div { class: "metric", div { class: "metric__label", "{ctx.t(\"adminAi.corpus.mangaPages\")}" } div { class: "metric__value tabular", "{ctx.int(manga)}" } }
                            div { class: "metric", div { class: "metric__label", "{ctx.t(\"adminAi.corpus.pipeline\")}" } div { class: "metric__value metric__value--text", "{state}" } }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn OcrProbe() -> Element {
    let ctx = use_ctx();
    let image = use_signal(|| None::<String>);
    let file_error = use_signal(|| None::<String>);
    let result = use_signal(|| None::<String>);
    let busy = use_signal(|| false);
    let on_file = {
        let ctx = ctx.clone();
        move |e: FormEvent| {
            let (mut image, mut file_error, mut result) = (image, file_error, result);
            let ctx = ctx.clone();
            result.set(None);
            let files = e.files();
            spawn(async move {
                let Some(f) = files.into_iter().next() else {
                    image.set(None);
                    return;
                };
                match f.read_bytes().await {
                    Ok(bytes) => match format::ocr_file_error(&bytes) {
                        Some(key) => {
                            image.set(None);
                            let text = if key == "adminAi.corpus.ocrTooLarge" {
                                ctx.tf(
                                    key,
                                    &[
                                        ("size", &format::megabytes(ctx.lang, bytes.len())),
                                        ("max", "6"),
                                    ],
                                )
                            } else {
                                ctx.t(key).to_owned()
                            };
                            file_error.set(Some(text));
                        }
                        None => {
                            file_error.set(None);
                            image.set(Some(
                                base64::engine::general_purpose::STANDARD.encode(&bytes),
                            ));
                        }
                    },
                    Err(err) => file_error.set(Some(err.to_string())),
                }
            });
        }
    };
    let probe = {
        let ctx = ctx.clone();
        move |_| {
            let (mut busy, mut result) = (busy, result);
            let Some(img) = image() else { return };
            busy.set(true);
            result.set(None);
            let ctx = ctx.clone();
            spawn(async move {
                if let Some(r) = run_action(
                    &ctx,
                    CockpitAction::OcrProbe { image_base64: img, language: "ja".into() },
                )
                .await
                {
                    let text = r
                        .body
                        .pointer("/result/text")
                        .and_then(|v| v.as_str())
                        .map(str::to_owned);
                    result.set(Some(
                        text.unwrap_or_else(|| ctx.t("adminAi.corpus.ocrNoText").to_owned()),
                    ));
                    ctx.say(ctx.t("adminAi.corpus.ocrDone"));
                } else {
                    ctx.set_notice(NoticeKind::Error, ctx.t("adminAi.corpus.ocrFailed"));
                }
                busy.set(false);
            });
        }
    };
    let err = file_error();
    rsx! {
        section { class: "card", "aria-labelledby": "ocr-title",
            h3 { id: "ocr-title", class: "card__title card__title--sm", Glyph { icon: icons::UPLOAD_SIMPLE } "{ctx.t(\"adminAi.corpus.ocrTitle\")}" }
            p { class: "small muted", "{ctx.t(\"adminAi.corpus.ocrDesc\")}" }
            div { class: "form__row",
                div { class: "field",
                    label { class: "field__label", r#for: "ocr-file", "{ctx.t(\"adminAi.corpus.ocrFile\")}" }
                    input { id: "ocr-file", class: "input", r#type: "file", accept: "image/png,image/jpeg,image/webp",
                        "aria-invalid": if err.is_some() { "true" } else { "false" },
                        "aria-describedby": "ocr-hint ocr-error",
                        onchange: on_file,
                    }
                    p { id: "ocr-hint", class: "field__hint", "{ctx.t(\"adminAi.corpus.ocrHint\")}" }
                    if let Some(e) = err.clone() { p { id: "ocr-error", class: "field__error", Glyph { icon: icons::WARNING_CIRCLE } "{e}" } }
                }
                button { r#type: "button", class: "btn btn--primary", disabled: image().is_none() || busy(), onclick: probe,
                    Glyph { icon: if busy() { icons::CIRCLE_NOTCH } else { icons::PLAY } }
                    if busy() { "{ctx.t(\"adminAi.corpus.ocrBusy\")}" } else { "{ctx.t(\"adminAi.corpus.ocrRun\")}" }
                }
            }
            if let Some(r) = result() { pre { class: "card card--inset pre mono", "{r}" } }
        }
    }
}
