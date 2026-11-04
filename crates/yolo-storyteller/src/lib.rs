// SPDX-License-Identifier: Apache-2.0
//! Storyteller creative suite contracts for Yolo UI (`yolo-storyteller`), migrated from
//! `storyteller-*` packages:
//!
//! 1. [`PromptBox`] — Multimodal generation prompt composer (`2D`, `3D`, `Audio`, `Image`, `Video`, `Edit`),
//!    `@mention` chips, and reference asset deck.
//! 2. [`Moodboard`] — Infinite spatial reference canvas with style cards, color swatches, and clusters.
//! 3. [`PageDraw`] — Multi-layer vector/raster sketchpad, pressure-sensitive brush presets, and ink guides.
//! 4. [`PageScene`] — Storyboard panel blocking, focal length, camera framing, and 3-point lighting setup.
//! 5. [`StoryTimeline`] — Multi-track narrative cut sequence, beat markers, and playhead transport.
//! 6. [`VfxGraph`] — Node-based shader and post-processing pipeline (`Bloom`, `ChromaticAberration`,
//!    `ColorGrade`, `Halftone`, `AnimeCel`, `DepthOfField`).
//! 7. [`Viewer3D`] — Interactive 3D glTF/USDZ stage, orbit camera rig, and skeletal pose controller.
//! 8. [`AudioMixer`] — Dialogue voiceover, music stems, foley soundboard, and parametric EQ/compression.
//! 9. [`VideoEditor`] — Clip trimming, transition curves, aspect ratio masks, and export encoding.
//! 10. [`CharacterSheet`] — Character Bible, expression atlas, voice model binding, and Byron Love
//!     10-teinte sprite sheet integration.

use serde::{Deserialize, Serialize};
pub use yolo_shared::{
    BYRON_LOVE_PALETTE, BYRON_LOVE_PET_SPEC, ByronLoveColorTeinte, ByronLovePetSpec, RgbaColor,
    YoloManifest, YoloStyle, YoloThemePreset, YoloThemeTokens,
};

/// Enumeration of all 10 Storyteller creative suite modules.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StorytellerModuleKind {
    /// Multimodal prompt composer & reference deck.
    PromptBox,
    /// Infinite visual reference & style moodboard.
    Moodboard,
    /// Multi-layer sketch & ink drawing surface.
    PageDraw,
    /// Camera blocking, shot composition & lighting stage.
    PageScene,
    /// Multi-track narrative beat & sequence timeline.
    StoryTimeline,
    /// Node-based post-processing & shader VFX graph.
    VfxGraph,
    /// Interactive 3D glTF/USDZ viewport & character poser.
    Viewer3D,
    /// Stem mixer, voiceover track & soundboard DSP.
    AudioMixer,
    /// Non-linear clip editor, transitions & render queue.
    VideoEditor,
    /// Character Bible, LoRA/voice bindings & sprite atlas.
    CharacterSheet,
}

impl StorytellerModuleKind {
    /// All 10 Storyteller creative modules in canonical order.
    pub const ALL: [Self; 10] = [
        Self::PromptBox,
        Self::Moodboard,
        Self::PageDraw,
        Self::PageScene,
        Self::StoryTimeline,
        Self::VfxGraph,
        Self::Viewer3D,
        Self::AudioMixer,
        Self::VideoEditor,
        Self::CharacterSheet,
    ];

    /// Machine-readable module identifier.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::PromptBox => "prompt-box",
            Self::Moodboard => "moodboard",
            Self::PageDraw => "page-draw",
            Self::PageScene => "page-scene",
            Self::StoryTimeline => "story-timeline",
            Self::VfxGraph => "vfx-graph",
            Self::Viewer3D => "viewer-3d",
            Self::AudioMixer => "audio-mixer",
            Self::VideoEditor => "video-editor",
            Self::CharacterSheet => "character-sheet",
        }
    }

    /// Human-readable display name.
    #[must_use]
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::PromptBox => "PromptBox",
            Self::Moodboard => "Moodboard",
            Self::PageDraw => "PageDraw",
            Self::PageScene => "PageScene",
            Self::StoryTimeline => "StoryTimeline",
            Self::VfxGraph => "VfxGraph",
            Self::Viewer3D => "Viewer3D",
            Self::AudioMixer => "AudioMixer",
            Self::VideoEditor => "VideoEditor",
            Self::CharacterSheet => "CharacterSheet",
        }
    }
}

// ── 1. PromptBox ─────────────────────────────────────────────────────────────

/// Generation modality in [`PromptBox`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum PromptModality {
    /// 2D illustration / storyboard frame.
    #[default]
    Illustration2D,
    /// 3D mesh / Gaussian splat / glTF asset.
    Asset3D,
    /// Voiceover, SFX, or musical score.
    Audio,
    /// Photorealistic or stylized still image.
    Image,
    /// Motion video clip or keyframe interpolation.
    Video,
    /// Inpainting / outpainting / region edit.
    Edit,
}

/// Entity or asset `@mention` chip embedded inside a [`PromptBox`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MentionChip {
    /// Unique entity identifier (e.g. `"char:byron-love"`).
    pub id: String,
    /// Display handle (e.g. `"@ByronLove"`).
    pub handle: String,
    /// Category (`"character"`, `"style"`, `"camera"`, `"voice"`).
    pub category: String,
}

/// Multimodal prompt composer (`PromptBox`) contract.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PromptBox {
    /// Active modality.
    pub modality: PromptModality,
    /// Main positive prompt text.
    pub prompt: String,
    /// Optional negative prompt text.
    pub negative_prompt: String,
    /// Attached `@mention` entity chips.
    pub mentions: Vec<MentionChip>,
    /// Reference asset URIs in the prompt deck.
    pub reference_uris: Vec<String>,
    /// Classifier-free guidance scale (`1.0..=20.0`).
    pub guidance_scale: f32,
    /// Deterministic seed (`None` = random).
    pub seed: Option<u64>,
}

impl Default for PromptBox {
    fn default() -> Self {
        Self {
            modality: PromptModality::Illustration2D,
            prompt: String::new(),
            negative_prompt: String::new(),
            mentions: Vec::new(),
            reference_uris: Vec::new(),
            guidance_scale: 7.5,
            seed: None,
        }
    }
}

impl PromptBox {
    /// Creates a new [`PromptBox`] with the given prompt text and modality.
    #[must_use]
    pub fn new(modality: PromptModality, prompt: impl Into<String>) -> Self {
        Self {
            modality,
            prompt: prompt.into(),
            ..Self::default()
        }
    }

    /// Attaches a [`MentionChip`] to the prompt.
    pub fn add_mention(
        &mut self,
        id: impl Into<String>,
        handle: impl Into<String>,
        category: impl Into<String>,
    ) {
        self.mentions.push(MentionChip {
            id: id.into(),
            handle: handle.into(),
            category: category.into(),
        });
    }
}

// ── 2. Moodboard ─────────────────────────────────────────────────────────────

/// A spatial card on a [`Moodboard`] canvas.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MoodboardItem {
    /// Unique card ID.
    pub id: String,
    /// Title or annotation.
    pub title: String,
    /// Image URI or hex swatch code.
    pub source: String,
    /// Canvas position `(x, y)` in logical points.
    pub position: (f32, f32),
    /// Card dimensions `(width, height)`.
    pub size: (f32, f32),
    /// Style influence weight (`0.0..=1.0`).
    pub weight: f32,
}

/// Infinite spatial reference canvas (`Moodboard`) contract.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Moodboard {
    /// Board title.
    pub title: String,
    /// Spatial reference cards.
    pub items: Vec<MoodboardItem>,
    /// Extracted dominant color palette (`0xAARRGGBB`).
    pub palette_argb: Vec<u32>,
    /// Viewport pan `(x, y)` and zoom factor.
    pub viewport: (f32, f32, f32),
}

impl Default for Moodboard {
    fn default() -> Self {
        Self {
            title: "Main Moodboard".to_owned(),
            items: Vec::new(),
            palette_argb: vec![0xFF63_66F1, 0xFF17_335C, 0xFFFF_D700],
            viewport: (0.0, 0.0, 1.0),
        }
    }
}

// ── 3. PageDraw ──────────────────────────────────────────────────────────────

/// Brush tool preset in [`PageDraw`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum DrawBrushKind {
    /// Rough blue storyboard pencil.
    Pencil,
    /// Crisp G-pen comic inker.
    #[default]
    GPenInk,
    /// Soft airbrush / screentone shader.
    Screentone,
    /// Watercolor wash marker.
    Marker,
    /// Vector eraser.
    Eraser,
}

/// Drawing layer in [`PageDraw`].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DrawLayer {
    /// Layer identifier.
    pub id: String,
    /// Layer name (e.g. `"Rough Sketch"`, `"Lineart"`, `"Screentone"`).
    pub name: String,
    /// Layer opacity (`0.0..=1.0`).
    pub opacity: f32,
    /// Visibility flag.
    pub visible: bool,
    /// Locked flag.
    pub locked: bool,
}

/// Multi-layer sketch and comic page drawing (`PageDraw`) contract.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PageDraw {
    /// Canvas width in pixels.
    pub width_px: u32,
    /// Canvas height in pixels.
    pub height_px: u32,
    /// Active brush tool.
    pub brush: DrawBrushKind,
    /// Brush diameter in pixels.
    pub brush_size_px: f32,
    /// Active ink color (`0xAARRGGBB`).
    pub brush_color_argb: u32,
    /// Ordered drawing layers (bottom to top).
    pub layers: Vec<DrawLayer>,
}

impl Default for PageDraw {
    fn default() -> Self {
        Self {
            width_px: 2480,
            height_px: 3508,
            brush: DrawBrushKind::GPenInk,
            brush_size_px: 6.0,
            brush_color_argb: 0xFF17_335C,
            layers: vec![
                DrawLayer {
                    id: "draft".to_owned(),
                    name: "Draft Pencil".to_owned(),
                    opacity: 0.45,
                    visible: true,
                    locked: false,
                },
                DrawLayer {
                    id: "ink".to_owned(),
                    name: "Clean Ink".to_owned(),
                    opacity: 1.0,
                    visible: true,
                    locked: false,
                },
            ],
        }
    }
}

// ── 4. PageScene ─────────────────────────────────────────────────────────────

/// Shot framing classification in [`PageScene`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShotFraming {
    /// Extreme Wide Shot (establishing environment).
    ExtremeWide,
    /// Wide / Full body shot.
    Wide,
    /// Medium / Cowboy / Waist-up shot.
    #[default]
    Medium,
    /// Close-up facial expression shot.
    CloseUp,
    /// Extreme close-up detail shot.
    ExtremeCloseUp,
    /// Dutch angle dynamic action shot.
    DutchAction,
}

/// Storyboard panel & scene blocking (`PageScene`) contract.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PageScene {
    /// Scene identifier.
    pub scene_id: String,
    /// Scene heading (e.g. `"EXT. OLYMPUS STADIUM - DUSK"`).
    pub slugline: String,
    /// Camera framing.
    pub framing: ShotFraming,
    /// Lens focal length in millimeters (e.g. `35.0`, `50.0`, `85.0`).
    pub focal_length_mm: f32,
    /// Key light color (`0xAARRGGBB`).
    pub key_light_argb: u32,
    /// Rim light color (`0xAARRGGBB`).
    pub rim_light_argb: u32,
    /// Cast character IDs present in this scene.
    pub cast_ids: Vec<String>,
}

impl Default for PageScene {
    fn default() -> Self {
        Self {
            scene_id: "scene-01".to_owned(),
            slugline: "EXT. SANCTUARY SKYLINE - GOLDEN HOUR".to_owned(),
            framing: ShotFraming::Medium,
            focal_length_mm: 50.0,
            key_light_argb: 0xFFFF_D700,
            rim_light_argb: 0xFF7C_5CFF,
            cast_ids: vec!["byron-love".to_owned()],
        }
    }
}

// ── 5. StoryTimeline ─────────────────────────────────────────────────────────

/// A single narrative cut or beat clip on a [`StoryTimeline`].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TimelineBeat {
    /// Beat ID.
    pub id: String,
    /// Label or dialogue snippet.
    pub label: String,
    /// Start time in seconds.
    pub start_sec: f64,
    /// Duration in seconds.
    pub duration_sec: f64,
    /// Track index (`0` = Storyboard, `1` = Dialogue, `2` = Music, `3` = VFX).
    pub track: u32,
}

/// Multi-track narrative timeline (`StoryTimeline`) contract.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StoryTimeline {
    /// Frame rate in frames per second (`24`, `30`, `60`).
    pub fps: u32,
    /// Current playhead position in seconds.
    pub playhead_sec: f64,
    /// Total sequence duration in seconds.
    pub duration_sec: f64,
    /// Beats scheduled across tracks.
    pub beats: Vec<TimelineBeat>,
}

impl Default for StoryTimeline {
    fn default() -> Self {
        Self {
            fps: 24,
            playhead_sec: 0.0,
            duration_sec: 12.0,
            beats: vec![
                TimelineBeat {
                    id: "beat-1".to_owned(),
                    label: "Opening Establishing Shot".to_owned(),
                    start_sec: 0.0,
                    duration_sec: 4.0,
                    track: 0,
                },
                TimelineBeat {
                    id: "beat-2".to_owned(),
                    label: "Heaven's Time Activation".to_owned(),
                    start_sec: 4.0,
                    duration_sec: 8.0,
                    track: 0,
                },
            ],
        }
    }
}

// ── 6. VfxGraph ──────────────────────────────────────────────────────────────

/// Built-in shader / post-FX pass type in [`VfxGraph`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum VfxPassKind {
    /// HDR glow & celestial bloom.
    Bloom,
    /// Anime cel-shading & rim-light threshold.
    AnimeCel,
    /// Lens chromatic aberration.
    ChromaticAberration,
    /// Film LUT & color grading wheels.
    ColorGrade,
    /// Comic halftone & Ben-Day dots.
    Halftone,
    /// Bokeh depth-of-field blur.
    DepthOfField,
}

/// A node in the [`VfxGraph`] pipeline.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VfxNode {
    /// Node ID.
    pub id: String,
    /// Effect type.
    pub kind: VfxPassKind,
    /// Dry/wet mix factor (`0.0..=1.0`).
    pub mix: f32,
    /// Whether this pass is bypassed.
    pub enabled: bool,
}

/// Node-based shader and post-processing pipeline (`VfxGraph`) contract.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VfxGraph {
    /// Ordered post-processing nodes.
    pub nodes: Vec<VfxNode>,
}

impl Default for VfxGraph {
    fn default() -> Self {
        Self {
            nodes: vec![
                VfxNode {
                    id: "cel-1".to_owned(),
                    kind: VfxPassKind::AnimeCel,
                    mix: 1.0,
                    enabled: true,
                },
                VfxNode {
                    id: "bloom-1".to_owned(),
                    kind: VfxPassKind::Bloom,
                    mix: 0.35,
                    enabled: true,
                },
            ],
        }
    }
}

// ── 7. Viewer3D ──────────────────────────────────────────────────────────────

/// Interactive 3D glTF/USDZ viewport and character stage (`Viewer3D`) contract.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Viewer3D {
    /// Loaded glTF / GLB / USDZ scene URI.
    pub scene_uri: String,
    /// Orbit camera yaw, pitch (degrees), and distance (meters).
    pub orbit_camera: (f32, f32, f32),
    /// Camera target `(x, y, z)`.
    pub target: (f32, f32, f32),
    /// Environment HDRI preset name.
    pub hdri_preset: String,
    /// Show ground grid flag.
    pub show_grid: bool,
}

impl Default for Viewer3D {
    fn default() -> Self {
        Self {
            scene_uri: "asset://scenes/sanctuary_stage.glb".to_owned(),
            orbit_camera: (35.0, 20.0, 4.5),
            target: (0.0, 1.1, 0.0),
            hdri_preset: "olympus_sunset.hdr".to_owned(),
            show_grid: true,
        }
    }
}

// ── 8. AudioMixer ────────────────────────────────────────────────────────────

/// An audio stem or voiceover channel in [`AudioMixer`].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AudioChannelStrip {
    /// Channel ID.
    pub id: String,
    /// Channel name (e.g. `"Dialogue (Aoede)"`, `"Orchestral Score"`, `"SFX Foley"`).
    pub name: String,
    /// Gain in decibels (`-60.0..=6.0`).
    pub gain_db: f32,
    /// Stereo pan (`-1.0` left .. `1.0` right).
    pub pan: f32,
    /// Muted flag.
    pub muted: bool,
    /// Soloed flag.
    pub solo: bool,
}

/// Multi-stem audio mixer, voiceover & soundboard (`AudioMixer`) contract.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AudioMixer {
    /// Sample rate in Hz (`48000`).
    pub sample_rate_hz: u32,
    /// Master bus gain in dB.
    pub master_gain_db: f32,
    /// Channel strips.
    pub channels: Vec<AudioChannelStrip>,
}

impl Default for AudioMixer {
    fn default() -> Self {
        Self {
            sample_rate_hz: 48_000,
            master_gain_db: 0.0,
            channels: vec![
                AudioChannelStrip {
                    id: "vo".to_owned(),
                    name: "Voiceover".to_owned(),
                    gain_db: 0.0,
                    pan: 0.0,
                    muted: false,
                    solo: false,
                },
                AudioChannelStrip {
                    id: "bgm".to_owned(),
                    name: "Soundtrack".to_owned(),
                    gain_db: -6.0,
                    pan: 0.0,
                    muted: false,
                    solo: false,
                },
                AudioChannelStrip {
                    id: "sfx".to_owned(),
                    name: "SFX & Ambience".to_owned(),
                    gain_db: -3.0,
                    pan: 0.0,
                    muted: false,
                    solo: false,
                },
            ],
        }
    }
}

// ── 9. VideoEditor ───────────────────────────────────────────────────────────

/// Aspect ratio preset in [`VideoEditor`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum VideoAspectRatio {
    /// 16:9 Widescreen (`1920x1080` / `3840x2160`).
    #[default]
    Widescreen16x9,
    /// 9:16 Vertical Shorts / Reels (`1080x1920`).
    Vertical9x16,
    /// 2.39:1 Anamorphic CinemaScope.
    Cinema239x1,
    /// 1:1 Square (`1080x1080`).
    Square1x1,
}

impl VideoAspectRatio {
    /// Numeric width-to-height ratio.
    #[must_use]
    pub const fn ratio(self) -> f32 {
        match self {
            Self::Widescreen16x9 => 16.0 / 9.0,
            Self::Vertical9x16 => 9.0 / 16.0,
            Self::Cinema239x1 => 2.39,
            Self::Square1x1 => 1.0,
        }
    }
}

/// Non-linear video editor and render queue (`VideoEditor`) contract.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VideoEditor {
    /// Sequence title.
    pub sequence_title: String,
    /// Target aspect ratio.
    pub aspect_ratio: VideoAspectRatio,
    /// Output resolution `(width, height)`.
    pub resolution: (u32, u32),
    /// Target export codec (`"av1"`, `"h264"`, `"prores_422_hq"`).
    pub codec: String,
}

impl Default for VideoEditor {
    fn default() -> Self {
        Self {
            sequence_title: "Episode 01 Master Cut".to_owned(),
            aspect_ratio: VideoAspectRatio::Widescreen16x9,
            resolution: (1920, 1080),
            codec: "av1".to_owned(),
        }
    }
}

// ── 10. CharacterSheet ───────────────────────────────────────────────────────

/// Character Bible, LoRA/voice binding, and sprite atlas (`CharacterSheet`) contract.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CharacterSheet {
    /// Character identifier (e.g. `"byron-love"`).
    pub id: String,
    /// Character display name (e.g. `"Byron Love (Aphrody)"`).
    pub display_name: String,
    /// Character title / archetype.
    pub archetype: String,
    /// Voice synthesis profile ID (e.g. `"Aoede"`).
    pub voice_profile: String,
    /// Canonical color teintes (`(name, hex, role)`).
    pub palette_teintes: Vec<(String, String, String)>,
    /// Sprite sheet path or URI.
    pub spritesheet_uri: String,
    /// Sprite cell dimensions `(width_px, height_px)`.
    pub sprite_cell_size: (u32, u32),
}

impl CharacterSheet {
    /// Builds the canonical Byron Love / Aphrody character sheet from [`BYRON_LOVE_PALETTE`] and
    /// [`BYRON_LOVE_PET_SPEC`].
    #[must_use]
    pub fn byron_love() -> Self {
        Self {
            id: "byron-love".to_owned(),
            display_name: "Byron Love (Aphrody)".to_owned(),
            archetype: "Celestial Midfielder & Sovereign Studio Muse".to_owned(),
            voice_profile: "Aoede".to_owned(),
            palette_teintes: BYRON_LOVE_PALETTE
                .iter()
                .map(|t| (t.id.to_owned(), t.hex.to_owned(), t.role.to_owned()))
                .collect(),
            spritesheet_uri: format!("/assets/openpets/{}.svg", BYRON_LOVE_PET_SPEC.pet_id),
            sprite_cell_size: (
                u32::from(BYRON_LOVE_PET_SPEC.cell_width_px),
                u32::from(BYRON_LOVE_PET_SPEC.cell_height_px),
            ),
        }
    }

    /// Creates a custom neutral [`CharacterSheet`].
    #[must_use]
    pub fn custom(
        id: impl Into<String>,
        display_name: impl Into<String>,
        archetype: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            display_name: display_name.into(),
            archetype: archetype.into(),
            voice_profile: "Puck".to_owned(),
            palette_teintes: vec![(
                "Primary Accent".to_owned(),
                "#6366F1".to_owned(),
                "Main character signature color".to_owned(),
            )],
            spritesheet_uri: "assets/characters/default_atlas.png".to_owned(),
            sprite_cell_size: (64, 64),
        }
    }
}

impl Default for CharacterSheet {
    fn default() -> Self {
        Self::byron_love()
    }
}

// ── Unified StorytellerProject & YoloManifest Integration ────────────────────

/// Complete Storyteller studio project uniting all 10 creative modules with [`YoloManifest`] and
/// the 7 [`YoloThemePreset`]s.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(bound(deserialize = "'de: 'static"))]
pub struct StorytellerProject {
    /// Application manifest & active [`YoloThemePreset`].
    pub manifest: YoloManifest,
    /// Active list of Storyteller studio modules.
    pub modules: Vec<StorytellerModuleKind>,
    /// Multimodal prompt composer.
    pub prompt_box: PromptBox,
    /// Visual reference moodboard.
    pub moodboard: Moodboard,
    /// Page sketch & ink studio.
    pub page_draw: PageDraw,
    /// Scene blocking & camera lighting.
    pub page_scene: PageScene,
    /// Narrative sequence timeline.
    pub timeline: StoryTimeline,
    /// Shader & post-FX node graph.
    pub vfx_graph: VfxGraph,
    /// Interactive 3D scene stage.
    pub viewer_3d: Viewer3D,
    /// Multi-stem audio mixer.
    pub audio_mixer: AudioMixer,
    /// Non-linear video editor.
    pub video_editor: VideoEditor,
    /// Cast character sheets.
    pub characters: Vec<CharacterSheet>,
}

impl Default for StorytellerProject {
    fn default() -> Self {
        Self::for_preset(YoloThemePreset::Neutral)
    }
}

impl StorytellerProject {
    /// Creates a new [`StorytellerProject`] with custom `app_id`, `title`, and [`YoloThemePreset`].
    #[must_use]
    pub fn new(
        app_id: impl Into<String>,
        title: impl Into<String>,
        preset: YoloThemePreset,
    ) -> Self {
        let mut proj = Self::for_preset(preset);
        proj.manifest = YoloManifest::new(app_id, title).with_preset(preset);
        proj
    }

    /// Creates a new [`StorytellerProject`] configured for one of the 7 [`YoloThemePreset`]s.
    #[must_use]
    pub fn for_preset(preset: YoloThemePreset) -> Self {
        let title = match preset {
            YoloThemePreset::Aphrody => "Aphrody Storyteller Studio",
            _ => "Yolo Storyteller Studio",
        };
        let manifest = YoloManifest::new("org.yolo.storyteller", title).with_preset(preset);
        let character = if preset == YoloThemePreset::Aphrody {
            CharacterSheet::byron_love()
        } else {
            CharacterSheet::custom("protagonist", "Aria Vance", "Lead Storyboard Protagonist")
        };
        Self {
            manifest,
            modules: StorytellerModuleKind::ALL.to_vec(),
            prompt_box: PromptBox::default(),
            moodboard: Moodboard::default(),
            page_draw: PageDraw::default(),
            page_scene: PageScene::default(),
            timeline: StoryTimeline::default(),
            vfx_graph: VfxGraph::default(),
            viewer_3d: Viewer3D::default(),
            audio_mixer: AudioMixer::default(),
            video_editor: VideoEditor::default(),
            characters: vec![character, CharacterSheet::byron_love()],
        }
    }

    /// Resolves the [`YoloThemeTokens`] for this project's active preset in light or dark mode.
    #[must_use]
    pub const fn theme_tokens(&self, dark: bool) -> YoloThemeTokens {
        self.manifest.theme_preset.tokens(dark)
    }

    /// Resolves the [`YoloStyle`] for this project's active preset.
    #[must_use]
    pub const fn style(&self) -> YoloStyle {
        self.manifest.resolved_style()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_ten_storyteller_modules_are_registered() {
        assert_eq!(StorytellerModuleKind::ALL.len(), 10);
        for module in StorytellerModuleKind::ALL {
            assert!(!module.id().is_empty());
            assert!(!module.display_name().is_empty());
        }
    }

    #[test]
    fn storyteller_project_supports_all_five_presets_and_json_roundtrip() {
        for preset in YoloThemePreset::ALL {
            let mut project = StorytellerProject::for_preset(preset);
            assert_eq!(project.manifest.theme_preset, preset);
            project
                .prompt_box
                .add_mention("char:byron-love", "@ByronLove", "character");
            assert_eq!(project.prompt_box.mentions.len(), 1);

            let tokens = project.theme_tokens(true);
            assert!(!tokens.name.is_empty());

            let json: &'static str = Box::leak(
                serde_json::to_string(&project)
                    .expect("serialize StorytellerProject")
                    .into_boxed_str(),
            );
            let roundtrip: StorytellerProject =
                serde_json::from_str(json).expect("deserialize StorytellerProject");
            assert_eq!(roundtrip.manifest.theme_preset, preset);
            assert_eq!(roundtrip.prompt_box.mentions.len(), 1);
        }
    }

    #[test]
    fn byron_love_character_sheet_has_all_ten_teintes() {
        let sheet = CharacterSheet::byron_love();
        assert_eq!(sheet.id, "byron-love");
        assert_eq!(sheet.palette_teintes.len(), 10);
        assert_eq!(sheet.sprite_cell_size, (192, 208));
    }
}
