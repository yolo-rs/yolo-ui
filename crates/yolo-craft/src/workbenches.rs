// SPDX-License-Identifier: Apache-2.0
//! Domain workbench contracts for all 13 Yolo Craft creative & engineering applications
//! migrated from `*-craft-ui-egui`.
//!
//! Every Craft workbench shares the docking, toolbar, tab, and theme chrome in `yolo-craft`
//! while declaring its domain tools, default dock panels, supported file extensions, and
//! [`YoloManifest`] integration.

use crate::brand::CraftBrand;
use crate::docking::Zone as Side;
use yolo_shared::{YoloManifest, YoloThemePreset};

/// Enumeration of the 13 canonical Craft domain workbenches.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum CraftWorkbenchKind {
    /// 3D parametric CAD & B-Rep solid modeling (`cadcraft-ui-egui`).
    CadCraft,
    /// Slide presentation & deck authoring (`deckcraft-ui-egui`).
    DeckCraft,
    /// UI/UX design system & layout studio (`designcraft-ui-egui`).
    DesignCraft,
    /// Real-time node-based VFX & motion compositing (`effectcraft-ui-egui`).
    EffectCraft,
    /// Collaborative vector whiteboard & figjam canvas (`figcraft-ui-egui`).
    FigCraft,
    /// Non-linear video editing & color grading (`filmcraft-ui-egui`).
    FilmCraft,
    /// High-density spreadsheet & formula grid (`gridcraft-ui-egui`).
    GridCraft,
    /// RAW photography darkroom & non-destructive developing (`lightcraft-ui-egui`).
    LightCraft,
    /// PDF document inspection, annotation & preflight (`pdfcraft-ui-egui`).
    PdfCraft,
    /// Multi-layer raster image & pixel painting (`photocraft-ui-egui`).
    PhotoCraft,
    /// Digital audio workstation, stem mixing & DSP (`soundcraft-ui-egui`).
    SoundCraft,
    /// Precision Bézier vector illustration (`vectorcraft-ui-egui`).
    VectorCraft,
    /// Rich-text technical publishing & document processor (`wordcraft-ui-egui`).
    WordCraft,
}

impl CraftWorkbenchKind {
    /// All 13 Craft domain workbenches in canonical order.
    pub const ALL: [Self; 13] = [
        Self::CadCraft,
        Self::DeckCraft,
        Self::DesignCraft,
        Self::EffectCraft,
        Self::FigCraft,
        Self::FilmCraft,
        Self::GridCraft,
        Self::LightCraft,
        Self::PdfCraft,
        Self::PhotoCraft,
        Self::SoundCraft,
        Self::VectorCraft,
        Self::WordCraft,
    ];

    /// Machine-readable workbench identifier (e.g. `"cadcraft"`).
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::CadCraft => "cadcraft",
            Self::DeckCraft => "deckcraft",
            Self::DesignCraft => "designcraft",
            Self::EffectCraft => "effectcraft",
            Self::FigCraft => "figcraft",
            Self::FilmCraft => "filmcraft",
            Self::GridCraft => "gridcraft",
            Self::LightCraft => "lightcraft",
            Self::PdfCraft => "pdfcraft",
            Self::PhotoCraft => "photocraft",
            Self::SoundCraft => "soundcraft",
            Self::VectorCraft => "vectorcraft",
            Self::WordCraft => "wordcraft",
        }
    }

    /// Human-readable workbench name (e.g. `"CadCraft"`).
    #[must_use]
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::CadCraft => "CadCraft",
            Self::DeckCraft => "DeckCraft",
            Self::DesignCraft => "DesignCraft",
            Self::EffectCraft => "EffectCraft",
            Self::FigCraft => "FigCraft",
            Self::FilmCraft => "FilmCraft",
            Self::GridCraft => "GridCraft",
            Self::LightCraft => "LightCraft",
            Self::PdfCraft => "PdfCraft",
            Self::PhotoCraft => "PhotoCraft",
            Self::SoundCraft => "SoundCraft",
            Self::VectorCraft => "VectorCraft",
            Self::WordCraft => "WordCraft",
        }
    }

    /// Short capability title used in window titles (e.g. `"CAD"`, `"Vector"`).
    #[must_use]
    pub const fn capability(self) -> &'static str {
        match self {
            Self::CadCraft => "CAD",
            Self::DeckCraft => "Deck",
            Self::DesignCraft => "Design",
            Self::EffectCraft => "Effects",
            Self::FigCraft => "Whiteboard",
            Self::FilmCraft => "Film",
            Self::GridCraft => "Spreadsheet",
            Self::LightCraft => "Darkroom",
            Self::PdfCraft => "PDF",
            Self::PhotoCraft => "Photo",
            Self::SoundCraft => "Audio DAW",
            Self::VectorCraft => "Vector",
            Self::WordCraft => "Document",
        }
    }

    /// Returns the full [`CraftWorkbenchContract`] for this workbench.
    #[must_use]
    pub const fn contract(self) -> CraftWorkbenchContract {
        match self {
            Self::CadCraft => CraftWorkbenchContract {
                kind: self,
                id: "cadcraft",
                display_name: "CadCraft",
                capability: "CAD",
                description: "Parametric 3D CAD, sketch constraints, and B-Rep solid modeling workbench",
                extensions: &["step", "stp", "iges", "stl", "obj", "cadcraft"],
                tools: &["Select", "Sketch", "Extrude", "Revolve", "Fillet", "Chamfer", "Boolean", "Measure"],
                default_panels: const {
                    &[
                        DockPanelSpec::new("feature-tree", "Feature Tree", Side::Left, 260.0),
                        DockPanelSpec::new("constraints", "Constraints", Side::Left, 260.0),
                        DockPanelSpec::new("inspector", "Parameters", Side::Right, 300.0),
                        DockPanelSpec::new("diagnostics", "Solver Log", Side::Bottom, 160.0),
                    ]
                },
            },
            Self::DeckCraft => CraftWorkbenchContract {
                kind: self,
                id: "deckcraft",
                display_name: "DeckCraft",
                capability: "Deck",
                description: "Slide presentation authoring, speaker notes, and motion transitions workbench",
                extensions: &["deckcraft", "pptx", "pdf", "md"],
                tools: &["Select", "TextBox", "Shape", "Image", "Chart", "CodeBlock", "Transition"],
                default_panels: const {
                    &[
                        DockPanelSpec::new("slides", "Slide Navigator", Side::Left, 220.0),
                        DockPanelSpec::new("inspector", "Slide Properties", Side::Right, 280.0),
                        DockPanelSpec::new("speaker-notes", "Speaker Notes", Side::Bottom, 160.0),
                    ]
                },
            },
            Self::DesignCraft => CraftWorkbenchContract {
                kind: self,
                id: "designcraft",
                display_name: "DesignCraft",
                capability: "Design",
                description: "UI/UX component design, auto-layout frames, and design token workbench",
                extensions: &["designcraft", "fig", "svg", "json"],
                tools: &["Select", "Frame", "AutoLayout", "Component", "Text", "Pen", "Prototype"],
                default_panels: const {
                    &[
                        DockPanelSpec::new("layers", "Layers & Assets", Side::Left, 260.0),
                        DockPanelSpec::new("design-inspector", "Design & Tokens", Side::Right, 300.0),
                    ]
                },
            },
            Self::EffectCraft => CraftWorkbenchContract {
                kind: self,
                id: "effectcraft",
                display_name: "EffectCraft",
                capability: "Effects",
                description: "Node-based VFX compositing, shader graph, and motion keyframes workbench",
                extensions: &["effectcraft", "exr", "nk", "wgsl"],
                tools: &["Select", "Transform", "RotoBezier", "Tracker", "ColorCorrect", "Blur", "ShaderNode"],
                default_panels: const {
                    &[
                        DockPanelSpec::new("nodes", "Node Library", Side::Left, 240.0),
                        DockPanelSpec::new("parameters", "Node Inspector", Side::Right, 300.0),
                        DockPanelSpec::new("node-graph", "Compositing Graph & Dope Sheet", Side::Bottom, 260.0),
                    ]
                },
            },
            Self::FigCraft => CraftWorkbenchContract {
                kind: self,
                id: "figcraft",
                display_name: "FigCraft",
                capability: "Whiteboard",
                description: "Infinite collaborative whiteboard, sticky notes, and architecture diagramming",
                extensions: &["figcraft", "svg", "png", "json"],
                tools: &["Select", "Hand", "StickyNote", "Connector", "Shape", "Marker", "Stamp"],
                default_panels: const {
                    &[
                        DockPanelSpec::new("stencils", "Stencils & Stickers", Side::Left, 220.0),
                        DockPanelSpec::new("minimap", "Minimap & Comments", Side::Right, 260.0),
                    ]
                },
            },
            Self::FilmCraft => CraftWorkbenchContract {
                kind: self,
                id: "filmcraft",
                display_name: "FilmCraft",
                capability: "Film",
                description: "Non-linear multi-track video editing, color scopes, and audio sync workbench",
                extensions: &["filmcraft", "mp4", "mov", "mkv", "fcpxml", "edl"],
                tools: &["Select", "Blade", "Trim", "Slip", "Slide", "ColorWheels", "Title"],
                default_panels: const {
                    &[
                        DockPanelSpec::new("media-pool", "Media Pool", Side::Left, 280.0),
                        DockPanelSpec::new("inspector", "Clip & Scopes", Side::Right, 300.0),
                        DockPanelSpec::new("timeline", "Multi-Track Timeline", Side::Bottom, 280.0),
                    ]
                },
            },
            Self::GridCraft => CraftWorkbenchContract {
                kind: self,
                id: "gridcraft",
                display_name: "GridCraft",
                capability: "Spreadsheet",
                description: "High-density reactive spreadsheet, pivot tables, and formula evaluation grid",
                extensions: &["gridcraft", "csv", "tsv", "xlsx", "parquet", "json"],
                tools: &["SelectCell", "Formula", "Sort", "Filter", "Pivot", "Chart", "ConditionalFormat"],
                default_panels: const {
                    &[
                        DockPanelSpec::new("sheets", "Sheets & Named Ranges", Side::Left, 220.0),
                        DockPanelSpec::new("formula-inspector", "Cell & Chart Inspector", Side::Right, 280.0),
                    ]
                },
            },
            Self::LightCraft => CraftWorkbenchContract {
                kind: self,
                id: "lightcraft",
                display_name: "LightCraft",
                capability: "Darkroom",
                description: "RAW photography catalog, histogram, tone curves, and non-destructive developing",
                extensions: &["lightcraft", "dng", "cr3", "nef", "arw", "jpg", "tiff"],
                tools: &["Crop", "SpotHeal", "MaskGradient", "MaskRadial", "WhiteBalance", "ToneCurve"],
                default_panels: const {
                    &[
                        DockPanelSpec::new("presets-navigator", "Navigator & Presets", Side::Left, 250.0),
                        DockPanelSpec::new("develop", "Histogram & Develop", Side::Right, 320.0),
                        DockPanelSpec::new("filmstrip", "Filmstrip", Side::Bottom, 140.0),
                    ]
                },
            },
            Self::PdfCraft => CraftWorkbenchContract {
                kind: self,
                id: "pdfcraft",
                display_name: "PdfCraft",
                capability: "PDF",
                description: "PDF viewing, vector redaction, form authoring, and print preflight workbench",
                extensions: &["pdf", "pdfcraft"],
                tools: &["Pan", "SelectText", "Highlight", "Note", "Redact", "Signature", "Measure"],
                default_panels: const {
                    &[
                        DockPanelSpec::new("thumbnails", "Pages & Bookmarks", Side::Left, 240.0),
                        DockPanelSpec::new("preflight", "Annotations & Preflight", Side::Right, 280.0),
                    ]
                },
            },
            Self::PhotoCraft => CraftWorkbenchContract {
                kind: self,
                id: "photocraft",
                display_name: "PhotoCraft",
                capability: "Photo",
                description: "Multi-layer raster image editing, brush engine, channels, and blend modes",
                extensions: &["photocraft", "psd", "png", "jpg", "webp", "avif", "ora"],
                tools: &["Move", "Marquee", "Lasso", "Brush", "Eraser", "CloneStamp", "Gradient", "Text"],
                default_panels: const {
                    &[
                        DockPanelSpec::new("brushes", "Tools & Brushes", Side::Left, 220.0),
                        DockPanelSpec::new("layers-channels", "Layers & Channels", Side::Right, 300.0),
                        DockPanelSpec::new("history", "History & Swatches", Side::Bottom, 160.0),
                    ]
                },
            },
            Self::SoundCraft => CraftWorkbenchContract {
                kind: self,
                id: "soundcraft",
                display_name: "SoundCraft",
                capability: "Audio DAW",
                description: "Digital audio workstation, multi-track arrangement, MIDI piano roll, and DSP mixer",
                extensions: &["soundcraft", "wav", "flac", "ogg", "mp3", "mid"],
                tools: &["Pointer", "Range", "Split", "Pencil", "Mute", "Automation", "TimeStretch"],
                default_panels: const {
                    &[
                        DockPanelSpec::new("browser", "Instruments & Samples", Side::Left, 260.0),
                        DockPanelSpec::new("channel-strip", "Channel Strip & FX", Side::Right, 280.0),
                        DockPanelSpec::new("mixer", "Mixer & Piano Roll", Side::Bottom, 260.0),
                    ]
                },
            },
            Self::VectorCraft => CraftWorkbenchContract {
                kind: self,
                id: "vectorcraft",
                display_name: "VectorCraft",
                capability: "Vector",
                description: "Precision Bézier vector illustration, boolean pathfinders, and print artboards",
                extensions: &["vectorcraft", "svg", "eps", "pdf", "ai"],
                tools: &["Select", "DirectSelect", "Pen", "Curvature", "ShapeBuilder", "Type", "Artboard"],
                default_panels: const {
                    &[
                        DockPanelSpec::new("artboards", "Artboards & Symbols", Side::Left, 240.0),
                        DockPanelSpec::new("stroke-fill", "Appearance, Stroke & Pathfinder", Side::Right, 300.0),
                    ]
                },
            },
            Self::WordCraft => CraftWorkbenchContract {
                kind: self,
                id: "wordcraft",
                display_name: "WordCraft",
                capability: "Document",
                description: "Structured long-form publishing, styles, bibliography, and print pagination",
                extensions: &["wordcraft", "docx", "odt", "md", "typ", "tex"],
                tools: &["Cursor", "Heading", "ParagraphStyle", "Table", "Figure", "Citation", "Footnote"],
                default_panels: const {
                    &[
                        DockPanelSpec::new("outline", "Document Outline", Side::Left, 240.0),
                        DockPanelSpec::new("styles", "Styles & Inspector", Side::Right, 280.0),
                    ]
                },
            },
        }
    }
}

/// Default dock panel specification for a Craft workbench.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DockPanelSpec {
    /// Stable panel identifier.
    pub id: &'static str,
    /// Human-readable panel title.
    pub title: &'static str,
    /// Default dock side (`Left`, `Right`, `Bottom`).
    pub side: Side,
    /// Default extent in logical points.
    pub default_extent: f32,
}

impl DockPanelSpec {
    /// Creates a new [`DockPanelSpec`].
    #[must_use]
    pub const fn new(id: &'static str, title: &'static str, side: Side, default_extent: f32) -> Self {
        Self {
            id,
            title,
            side,
            default_extent,
        }
    }
}

/// Complete domain contract for one of the 13 Craft workbenches.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CraftWorkbenchContract {
    /// Workbench variant.
    pub kind: CraftWorkbenchKind,
    /// Canonical kebab-case identifier.
    pub id: &'static str,
    /// Display name (e.g. `"CadCraft"`).
    pub display_name: &'static str,
    /// Short capability descriptor (e.g. `"CAD"`).
    pub capability: &'static str,
    /// Summary description of the workbench domain.
    pub description: &'static str,
    /// Supported file extensions (without leading dot).
    pub extensions: &'static [&'static str],
    /// Default toolbar tool identifiers.
    pub tools: &'static [&'static str],
    /// Default dockable panels around the central canvas.
    pub default_panels: &'static [DockPanelSpec],
}

impl CraftWorkbenchContract {
    /// Builds a [`YoloManifest`] for this workbench using the given [`YoloThemePreset`].
    #[must_use]
    pub fn manifest(&self, preset: YoloThemePreset) -> YoloManifest {
        let brand = CraftBrand::for_preset(preset);
        brand.to_manifest(
            &format!("org.yolo.craft.{}", self.id),
            self.capability,
        )
    }
}

/// Returns all 13 [`CraftWorkbenchContract`]s in canonical order.
#[must_use]
pub fn all_workbenches() -> [CraftWorkbenchContract; 13] {
    CraftWorkbenchKind::ALL.map(CraftWorkbenchKind::contract)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_thirteen_workbenches_have_complete_contracts() {
        let contracts = all_workbenches();
        assert_eq!(contracts.len(), 13);
        for contract in contracts {
            assert!(!contract.id.is_empty());
            assert!(!contract.display_name.is_empty());
            assert!(!contract.capability.is_empty());
            assert!(!contract.extensions.is_empty());
            assert!(!contract.tools.is_empty());
            assert!(!contract.default_panels.is_empty());
            let manifest = contract.manifest(YoloThemePreset::Neutral);
            assert!(manifest.app_id.starts_with("org.yolo.craft."));
            assert!(manifest.title.contains(contract.capability));
        }
    }
}
