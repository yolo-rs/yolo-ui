// SPDX-License-Identifier: Apache-2.0
//! yolo-stack facade over the Yolo UI creative studios.
//!
//! - [Craft](yolo_craft): 13 domain workbenches (`CadCraft` to `WordCraft`), each with an id, the file
//!   extensions it opens, its tools and its default docking layout.
//! - [Storyteller](yolo_storyteller): 10 creative modules (`PromptBox` to `CharacterSheet`) and the
//!   [`StorytellerProject`] document that holds them.
//!
//! Apps in yolo-stack depend on this crate instead of reaching into yolo-ui paths. The facade does no
//! I/O, so it builds for `wasm32-unknown-unknown`; Craft's file-backed document store is not part of
//! this API and stays native-only in yolo-ui.

pub use yolo_craft::workbenches::{CraftWorkbenchContract, CraftWorkbenchKind};
pub use yolo_shared::YoloThemePreset;
pub use yolo_storyteller::{StorytellerModuleKind, StorytellerProject};

use serde::Serialize;

/// Serializable summary of one Craft workbench, for catalogs, routing and the web.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct WorkbenchEntry {
    /// Stable kebab-case id (`"cad-craft"`).
    pub id: &'static str,
    /// Display name.
    pub name: &'static str,
    /// One-line capability.
    pub capability: &'static str,
    /// File extensions the workbench opens, without the dot.
    pub extensions: &'static [&'static str],
    /// Tool ids shown in the workbench toolbar.
    pub tools: &'static [&'static str],
}

impl From<&CraftWorkbenchContract> for WorkbenchEntry {
    fn from(c: &CraftWorkbenchContract) -> Self {
        Self {
            id: c.id,
            name: c.display_name,
            capability: c.capability,
            extensions: c.extensions,
            tools: c.tools,
        }
    }
}

/// Serializable summary of one Storyteller module.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct ModuleEntry {
    /// Stable kebab-case id (`"prompt-box"`).
    pub id: &'static str,
    /// Display name.
    pub name: &'static str,
}

impl From<StorytellerModuleKind> for ModuleEntry {
    fn from(kind: StorytellerModuleKind) -> Self {
        Self { id: kind.id(), name: kind.display_name() }
    }
}

/// Full studio catalog: every Craft workbench and every Storyteller module.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct StudioCatalog {
    /// The 13 Craft workbenches in canonical order.
    pub workbenches: Vec<WorkbenchEntry>,
    /// The 10 Storyteller modules in canonical order.
    pub modules: Vec<ModuleEntry>,
}

/// Builds the studio catalog.
#[must_use]
pub fn catalog() -> StudioCatalog {
    StudioCatalog {
        workbenches: yolo_craft::workbenches::all_workbenches()
            .iter()
            .map(WorkbenchEntry::from)
            .collect(),
        modules: StorytellerModuleKind::ALL
            .into_iter()
            .map(ModuleEntry::from)
            .collect(),
    }
}

/// Finds a Craft workbench by its id.
#[must_use]
pub fn workbench(id: &str) -> Option<CraftWorkbenchContract> {
    yolo_craft::workbenches::workbench_by_id(id)
}

/// Ids of the Craft workbenches that open files with `extension` (with or without the leading dot,
/// case-insensitive).
#[must_use]
pub fn workbenches_for_extension(extension: &str) -> Vec<&'static str> {
    let ext = extension.trim_start_matches('.').to_ascii_lowercase();
    yolo_craft::workbenches::all_workbenches()
        .iter()
        .filter(|c| c.supports_extension(&ext))
        .map(|c| c.id)
        .collect()
}

/// Finds a Storyteller module by its id.
#[must_use]
pub fn module(id: &str) -> Option<StorytellerModuleKind> {
    StorytellerModuleKind::ALL
        .into_iter()
        .find(|kind| kind.id() == id)
}

/// Creates a Storyteller project with every module enabled, themed with `preset`.
#[must_use]
pub fn new_story_project(id: &str, title: &str, preset: YoloThemePreset) -> StorytellerProject {
    StorytellerProject::new(id, title, preset)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn catalog_has_every_workbench_and_module_once() {
        let c = catalog();
        assert_eq!(c.workbenches.len(), 13);
        assert_eq!(c.modules.len(), 10);
        let ids: HashSet<_> = c
            .workbenches
            .iter()
            .map(|w| w.id)
            .chain(c.modules.iter().map(|m| m.id))
            .collect();
        assert_eq!(ids.len(), 23, "ids must be unique across studios");
    }

    #[test]
    fn lookups_round_trip() {
        for w in catalog().workbenches {
            assert_eq!(workbench(w.id).map(|c| c.id), Some(w.id));
            for ext in w.extensions {
                assert!(
                    workbenches_for_extension(ext).contains(&w.id),
                    "{} should open .{ext}",
                    w.id
                );
                assert!(
                    workbenches_for_extension(&format!(".{}", ext.to_ascii_uppercase()))
                        .contains(&w.id)
                );
            }
        }
        for m in StorytellerModuleKind::ALL {
            assert_eq!(module(m.id()), Some(m));
        }
        assert!(workbench("no-such-workbench").is_none());
        assert!(module("no-such-module").is_none());
    }

    #[test]
    fn story_project_enables_all_modules_with_the_preset() {
        let p = new_story_project("pilot", "Pilot", YoloThemePreset::Aphrody);
        assert_eq!(p.manifest.theme_preset, YoloThemePreset::Aphrody);
        assert_eq!(p.modules.len(), 10);
    }

    #[test]
    fn catalog_serializes_to_json() {
        let json = serde_json::to_value(catalog()).expect("catalog serializes");
        assert_eq!(json["workbenches"].as_array().map(Vec::len), Some(13));
        assert_eq!(json["modules"][0]["id"], "prompt-box");
    }
}
