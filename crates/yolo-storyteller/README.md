# Yolo Storyteller

`yolo-storyteller` is the workspace crate for Storyteller project data and
creative-module contracts. It is part of the root Cargo workspace and is
available through the optional `storyteller` feature of `packages/yolo-ui`.

The crate registers ten modules: `PromptBox`, `Moodboard`, `PageDraw`,
`PageScene`, `StoryTimeline`, `VfxGraph`, `Viewer3D`, `AudioMixer`,
`VideoEditor`, and `CharacterSheet`. `StorytellerProject` provides their
default serializable state, module registry, theme manifest, and preset-aware
Byron Love character sheet.

```rust
use yolo_shared::YoloThemePreset;
use yolo_storyteller::{StorytellerModuleKind, StorytellerProject};

let mut project = StorytellerProject::new(
    "org.example.story",
    "Story draft",
    YoloThemePreset::Neutral,
);
project.prompt_box.prompt = "A quiet mountain observatory".into();
assert_eq!(project.modules.len(), StorytellerModuleKind::ALL.len());
```

The crate defines serializable state and integration contracts. It does not
provide the actual audio/video codecs, 3D renderer, model inference pipeline,
document storage, or native app shell. Those engines and lifecycle services
belong to the host application and its selected workspace crates.

The crate is licensed Apache-2.0. See the repository root for the workspace
license and contribution policy.
