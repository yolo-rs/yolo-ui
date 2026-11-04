# Yolo Craft

`yolo-craft` supplies reusable egui workbench chrome and the static catalog for the 13 Craft
creative and engineering workbenches. It does not own application documents, codecs, file-system
access, or native window lifecycles. Each app owns those domain and platform decisions; the shared
crate provides stable IDs, capability descriptions, theme manifests, dock-panel defaults, and
format routing hints.

## Catalog and format routing

Use stable IDs for saved layouts and deep links. Route file-open requests through the extension
catalog, then let the host handle ambiguous formats explicitly:

```rust
use yolo_craft::workbenches::{workbench_by_id, workbenches_for_extension};
use yolo_shared::YoloThemePreset;

let pdf_targets: Vec<_> = workbenches_for_extension(".PDF").collect();
assert!(pdf_targets.iter().any(|workbench| workbench.id == "pdfcraft"));
assert!(pdf_targets.iter().any(|workbench| workbench.id == "vectorcraft"));

let cad = workbench_by_id("CadCraft").expect("known workbench");
let manifest = cad.manifest(YoloThemePreset::Neutral);
assert_eq!(manifest.app_id, "org.yolo.craft.cadcraft");
let initial_workspace = cad.default_layout().expect("valid built-in layout");
initial_workspace.validate().expect("layout remains valid");
```

Document-capable hosts share serializable metadata, immutable revision entries,
format import/export capabilities, and autosave scheduling:

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
use yolo_craft::document::{AutosavePolicy, CraftDocument, CraftDocumentFormat};
use yolo_craft::workbenches::workbench_by_id;

let contract = workbench_by_id("wordcraft").expect("registered workbench");
let mut document = CraftDocument::new(
    "doc-42",
    contract,
    "Chapter One",
    CraftDocumentFormat::new(
        "org.example.story+json",
        Some("application/json"),
        ["story", "json"],
        true,
        true,
    ),
    1_800_000_000_000,
    AutosavePolicy::default(),
)?;
let revision = document.record_revision(1_800_000_000_250, Some("First draft".into()), None)?;
document.mark_saved(revision, 1_800_000_001_000)?;
Ok(())
}
```

`default_layout` turns the contract's panel list into a validated docking tree: a central `canvas`
panel plus left, right, top, and bottom accordion stacks. Side extents set dock width; top and
bottom extents set panel heights. Hosts still provide the actual `DockContent`, document state,
and app-specific panel registry.

`workbenches_for_extension` accepts an extension with or without a leading dot, compares it
case-insensitively, and returns every match without allocating. This matters for shared formats:
PDF can open in PdfCraft, VectorCraft, and DeckCraft; SVG can open in DesignCraft, FigCraft, and
VectorCraft. A host may apply its own user preference or present a chooser.

## Shared and app-owned responsibilities

The catalog currently covers CAD, presentations, UI/UX design, compositing, whiteboarding,
non-linear video, spreadsheets, RAW photo development, PDF, raster image editing, audio, vector
illustration, and long-form publishing. A workbench contract declares its tool IDs, supported
extensions, default dock panels, title, capability, and app manifest preset.

The shared document API validates metadata and tracks revision/save markers; it does not store
document bytes, retain revision content, or schedule a background task. Format profiles describe
host codec capabilities rather than implementing codecs. Hosts still provide persistence, import
and export engines, autosave scheduling, collaboration, accessibility, and native OS integrations.

The docking, tab, button, icon, layout, print-control, and theme modules are renderer primitives.
They do not promise a specific document engine or format implementation.

The crate is Apache-2.0. It is a Yolo UI integration layer; external engine dependencies retain
their own licenses and notices in their respective source trees.
