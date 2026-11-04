// SPDX-License-Identifier: Apache-2.0
//! Shared egui UI primitives for Yolo Craft creative and engineering applications.
//!
//! Each module provides a focused, reusable mechanism for desktop workbench chrome:
//!
//! * [`brand`] — Neutral customizable [`brand::CraftBrand`] (defaulting to `"Yolo Craft"`, with
//!   presets for `Neutral`, `M3`, `Fluent`, `Apple`, and `Aphrody` Byron Love).
//! * [`buttons`] — `SemanticButton` for compact toolbar, toggle and modal actions.
//! * [`docking`] — `DockState` and `DockZones` for panels that dock to window edges or float.
//! * [`document`] — Serializable document metadata, revision chains, import/export format
//!   capabilities, and autosave scheduling policy shared by workbench hosts.
//! * [`icons`] — `VectorIcon` painter primitives for crisp geometric glyphs at any DPI.
//! * [`layout`] — `WorkbenchLayout` and `ToolbarFrame` for panel splits, toolbars, status bars
//!   and canvas viewport handoff.
//! * [`printing`] — Print / export preflight, page geometry, crop & bleed marks, and preview chrome.
//! * [`tabs`] — `DocumentTabs` for closable, dirty-aware document tab strips.
//! * [`theme`] — `WidgetPalette`, shape tokens, and the 5-preset M3 adapter (`theme::aphrody`).
//! * [`workbenches`] — Contracts and launch catalog for all 13 Craft domain workbenches
//!   (`workbench_by_id`, `workbenches_for_extension`, `CadCraft`, `DeckCraft`,
//!   `DesignCraft`, `EffectCraft`, `FigCraft`, `FilmCraft`, `GridCraft`, `LightCraft`,
//!   `PdfCraft`, `PhotoCraft`, `SoundCraft`, `VectorCraft`, `WordCraft`).

pub mod brand;
pub mod buttons;
pub mod docking;
#[cfg(feature = "serde")]
pub mod document;
pub mod icons;
pub mod layout;
pub mod printing;
pub mod tabs;
pub mod theme;
pub mod workbenches;
