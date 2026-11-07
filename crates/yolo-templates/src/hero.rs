// SPDX-License-Identifier: Apache-2.0
//! Stage hero, release pill, the original 3D object and the pausable starfield.

use dioxus::prelude::*;

use crate::content::{Hero, Release, Strings};
use crate::icons::{Icon, IconName};

/// The announcement pill: a tag, one line of text and an arrow, all one link.
#[component]
pub fn ReleasePill(release: Release) -> Element {
    rsx! {
        a { class: "yt-pill", href: release.href,
            span { class: "yt-pill-tag", {release.tag} }
            span { class: "yt-pill-text", {release.text} }
            Icon { name: IconName::ArrowRight }
        }
    }
}

/// Which pose the object takes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObjectPose {
    /// Slow idle turn, used on the stage (stops with the pause switch or reduced motion).
    Idle,
    /// Driven by the scroll timeline of the reveal.
    Scroll,
    /// Static three-quarter view, used in illustrations.
    Still,
}

/// The template 3D object: a glass cube around a solid core, built from twelve CSS faces in
/// accent tints. Original geometry; no model file. Decorative, hidden from assistive tech.
#[component]
pub fn Object3d(pose: ObjectPose) -> Element {
    let class = match pose {
        ObjectPose::Idle => "yt-object yt-object-idle yt-motion",
        ObjectPose::Scroll => "yt-object yt-object-scroll",
        ObjectPose::Still => "yt-object",
    };
    rsx! {
        div { class: class, "aria-hidden": "true",
            div { class: "yt-cube yt-cube-outer",
                for f in ["front", "back", "left", "right", "top", "bottom"] {
                    span { key: "{f}", class: "yt-face yt-face-{f}" }
                }
            }
            div { class: "yt-cube yt-cube-core",
                for f in ["front", "back", "left", "right", "top", "bottom"] {
                    span { key: "{f}", class: "yt-face yt-face-{f}" }
                }
            }
        }
    }
}

/// The starfield behind the stage: three drifting layers of generated stars and a visible
/// pause switch (WCAG 2.2.2). Reduced motion stops the drift in CSS; GPU hosts render
/// `shaders/starfield.wgsl` with [`crate::motion::StarfieldClock`] instead.
#[component]
pub fn Starfield(strings: &'static Strings) -> Element {
    let mut paused = use_signal(|| false);
    rsx! {
        div { class: "yt-starfield", "aria-hidden": "true",
            div { class: "yt-stars yt-stars-1 yt-motion" }
            div { class: "yt-stars yt-stars-2 yt-motion" }
            div { class: "yt-stars yt-stars-3 yt-motion" }
        }
        label { class: "yt-motion-switch",
            input {
                class: "yt-pause-switch",
                r#type: "checkbox",
                role: "switch",
                checked: paused(),
                onchange: move |_| paused.set(!paused()),
            }
            span { {strings.pause_motion} }
        }
    }
}

/// Stage hero: one object on a dark field, one primary action, an optional secondary one,
/// the release pill above the title and an optional starfield.
#[component]
pub fn StageHero(hero: Hero, release: Release, strings: &'static Strings, starfield: bool) -> Element {
    rsx! {
        section { class: "yt-stage", "aria-labelledby": "yt-stage-title",
            if starfield {
                Starfield { strings: strings }
            }
            div { class: "yt-stage-copy",
                ReleasePill { release: release }
                p { class: "yt-eyebrow", {hero.eyebrow} }
                h1 { class: "yt-stage-title", id: "yt-stage-title", {hero.title} }
                p { class: "yt-lede", {hero.lede} }
                div { class: "yt-ctas",
                    a { class: "yt-btn yt-btn-primary", href: hero.primary.href, {hero.primary.label} }
                    if let Some(s) = hero.secondary {
                        a { class: "yt-btn yt-btn-secondary", href: s.href,
                            {s.label}
                            Icon { name: IconName::ArrowRight }
                        }
                    }
                }
            }
            div { class: "yt-stage-object",
                Object3d { pose: ObjectPose::Idle }
            }
        }
    }
}
