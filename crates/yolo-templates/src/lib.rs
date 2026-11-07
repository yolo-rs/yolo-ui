// SPDX-License-Identifier: Apache-2.0
//! Shared page templates for every Aphrody site (Aphrody, YOLO, Shenron, Rose Griffon, Auto,
//! Craft), written as Dioxus 0.8 components and themed by the eight `YoloThemePreset`s.
//!
//! | Template | Component | Keyboard and motion |
//! | --- | --- | --- |
//! | Global nav, flyouts, mega menu | [`GlobalNav`] | native popovers: Enter/Space open, Esc closes and returns focus |
//! | Stage hero with release pill | [`StageHero`], [`ReleasePill`] | one primary action |
//! | Starfield background | [`Starfield`], [`motion::STARFIELD_WGSL`] | visible pause switch, off under reduced motion |
//! | Sticky section tabs | [`SectionTabs`] | in-page links with `aria-current`, `scroll-margin-top` |
//! | Product page | [`ProductPageView`] | sections plus `details` FAQ |
//! | Docs shell | [`DocsShell`] | radio platform switcher, theme switch, generated token page |
//! | Download channels | [`DownloadChannels`] | recommended channel marked in text, not colour alone |
//! | Rail app chrome, centred search | [`RailAppChrome`] | rail switch, labelled search and voice buttons |
//! | Dense media grid | [`MediaGrid`], [`app::visible_window`] | whole-tile link, windowing for native hosts |
//! | Editorial grid | [`EditorialGrid`] | lead story with the 3D object |
//! | Scroll-driven 3D reveal | [`ScrollReveal`], [`motion::RevealFrame`] | CSS scroll timeline, static final frame fallback |
//!
//! Pages render on the server with [`render_document`] and work with scripting off: popovers,
//! `details`, radio groups and `:has()` carry the interaction, and Dioxus signals take over
//! after hydration or in native renderers. Box layout is the `yolo-layout` subset in
//! [`style::LAYOUT_CSS`], so [`layout`] computes the same rects with Taffy.

pub mod app;
pub mod content;
pub mod docs;
pub mod hero;
pub mod icons;
pub mod layout;
pub mod motion;
pub mod nav;
pub mod page;
pub mod product;
pub mod style;
pub mod theme;

pub use app::{MediaGrid, MediaTile, RailAppChrome};
pub use content::{DOCS, SITES, Site, site};
pub use docs::DocsShell;
pub use hero::{Object3d, ObjectPose, ReleasePill, StageHero, Starfield};
pub use icons::{Icon, IconName};
pub use nav::{GlobalNav, SectionTabs};
pub use page::{Template, TemplatePage, ThemeRoot, render_document};
pub use product::{DownloadChannels, EditorialGrid, ProductPageView, ScrollReveal, UiSketch};
pub use style::stylesheet;
pub use theme::{Mode, Palette};

/// Every template, in page order (alias of [`Template::ALL`] for call sites).
pub struct Templates;

impl Templates {
    /// Every template.
    pub const ALL: &'static [Template] = &Template::ALL;
}
