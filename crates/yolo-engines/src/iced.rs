#![doc = include_str!("../README.md")]

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fmt;
pub use yolo_shared::{self, APHRODY_DARK_THEME, AphrodyThemeTokens, RgbaColor as IcedColor};

/// Core 6-slot palette matching `iced_core::theme::Palette`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct IcedPalette {
    pub background: IcedColor,
    pub text: IcedColor,
    pub primary: IcedColor,
    pub success: IcedColor,
    pub warning: IcedColor,
    pub danger: IcedColor,
}

/// Extended Material 3 Dark-Glass surface and container tokens for Iced widgets.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct IcedSurfaceTokens {
    pub surface_container_low: IcedColor,
    pub surface_container: IcedColor,
    pub surface_container_high: IcedColor,
    pub text_secondary: IcedColor,
    pub accent_secondary: IcedColor,
    pub border_subtle: IcedColor,
}

/// Complete Aphrody Labs Iced Theme combining `IcedPalette` with Material 3 Dark-Glass surfaces.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IcedTheme {
    pub name: String,
    pub is_dark: bool,
    pub palette: IcedPalette,
    pub surfaces: IcedSurfaceTokens,
    pub corner_radius_px: f32,
    pub border_width_px: f32,
}

impl IcedTheme {
    /// Returns the canonical Aphrody Material 3 Dark-Glass Iced theme (`aphrody-m3-dark`).
    pub fn aphrody_dark() -> Self {
        Self {
            name: String::from("aphrody-m3-dark"),
            is_dark: true,
            palette: IcedPalette {
                // #090b10
                background: IcedColor::from_rgb8(0x09, 0x0b, 0x10),
                // #f3f6fc
                text: IcedColor::from_rgb8(0xf3, 0xf6, 0xfc),
                // #7c5cff
                primary: IcedColor::from_rgb8(0x7c, 0x5c, 0xff),
                // #10b981
                success: IcedColor::from_rgb8(0x10, 0xb9, 0x81),
                // #f59e0b
                warning: IcedColor::from_rgb8(0xf5, 0x9e, 0x0b),
                // #ef4444
                danger: IcedColor::from_rgb8(0xef, 0x44, 0x44),
            },
            surfaces: IcedSurfaceTokens {
                // #0d1018
                surface_container_low: IcedColor::from_rgb8(0x0d, 0x10, 0x18),
                // #111520
                surface_container: IcedColor::from_rgb8(0x11, 0x15, 0x20),
                // #181e2e
                surface_container_high: IcedColor::from_rgb8(0x18, 0x1e, 0x2e),
                // #9ba6bf
                text_secondary: IcedColor::from_rgb8(0x9b, 0xa6, 0xbf),
                // #00d4ff
                accent_secondary: IcedColor::from_rgb8(0x00, 0xd4, 0xff),
                // rgba(255, 255, 255, 0.08)
                border_subtle: IcedColor::from_rgba8(255, 255, 255, 0.08),
            },
            corner_radius_px: 10.0,
            border_width_px: 1.0,
        }
    }
}

impl Default for IcedTheme {
    fn default() -> Self {
        Self::aphrody_dark()
    }
}

/// Elm Architecture (MVU: Model-View-Update) runtime bridge for Iced and Yolo UI.
pub mod mvu {
    use super::*;

    /// Asynchronous or synchronous side-effect task produced by `update`, matching `iced::Task<Message>`.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum Task<Message> {
        /// No side effects to execute.
        None,
        /// Immediately dispatch a single follow-up message.
        Perform(Message),
        /// Execute a batch of tasks in order.
        Batch(Vec<Task<Message>>),
    }

    impl<Message> Task<Message> {
        pub const fn none() -> Self {
            Self::None
        }

        pub fn done(message: Message) -> Self {
            Self::Perform(message)
        }

        pub fn batch(tasks: impl IntoIterator<Item = Self>) -> Self {
            Self::Batch(tasks.into_iter().collect())
        }

        pub fn map<Next>(self, mut f: impl FnMut(Message) -> Next) -> Task<Next> {
            self.map_ref(&mut f)
        }

        fn map_ref<Next>(self, f: &mut impl FnMut(Message) -> Next) -> Task<Next> {
            match self {
                Self::None => Task::None,
                Self::Perform(msg) => Task::Perform(f(msg)),
                Self::Batch(list) => Task::Batch(list.into_iter().map(|t| t.map_ref(f)).collect()),
            }
        }

        /// Flattens ready messages out of the task tree.
        pub fn into_messages(self) -> Vec<Message> {
            let mut out = Vec::new();
            self.collect_messages(&mut out);
            out
        }

        fn collect_messages(self, out: &mut Vec<Message>) {
            match self {
                Self::None => {}
                Self::Perform(msg) => out.push(msg),
                Self::Batch(list) => {
                    for task in list {
                        task.collect_messages(out);
                    }
                }
            }
        }
    }

    /// Passive event stream descriptor matching `iced::Subscription<Message>`.
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub struct SubscriptionDescriptor {
        pub id: String,
        pub kind: SubscriptionKind,
        pub interval_ms: Option<u64>,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    pub enum SubscriptionKind {
        WindowEvents,
        KeyboardEvents,
        AnimationFrame,
        TimeInterval,
        AudioBufferTick,
        Custom,
    }

    /// Trait modeling an Elm-architecture Iced application (`boot`, `title`, `update`, `theme`).
    pub trait IcedApplication {
        type Flags;
        type Message: Clone + fmt::Debug;

        fn boot(flags: Self::Flags) -> (Self, Task<Self::Message>)
        where
            Self: Sized;

        fn title(&self) -> String;

        fn update(&mut self, message: Self::Message) -> Task<Self::Message>;

        fn theme(&self) -> IcedTheme {
            IcedTheme::aphrody_dark()
        }

        fn subscriptions(&self) -> Vec<SubscriptionDescriptor> {
            Vec::new()
        }
    }

    /// Deterministic MVU state machine runner with optional Comet trace recording.
    pub struct MvuRuntime<App: IcedApplication> {
        pub app: App,
        queue: VecDeque<App::Message>,
        pub recorder: super::comet::CometRecorder,
    }

    impl<App: IcedApplication> MvuRuntime<App> {
        pub fn new(flags: App::Flags) -> Self {
            let (app, boot_task) = App::boot(flags);
            let mut queue = VecDeque::new();
            for msg in boot_task.into_messages() {
                queue.push_back(msg);
            }
            let mut runtime = Self {
                app,
                queue,
                recorder: super::comet::CometRecorder::new(256),
            };
            runtime.drain_until_idle(64);
            runtime
        }

        /// Enqueues a message and processes all synchronous follow-up `Task::Perform` hops up to `max_hops`.
        pub fn dispatch(&mut self, message: App::Message) -> usize {
            self.queue.push_back(message);
            self.drain_until_idle(64)
        }

        pub fn drain_until_idle(&mut self, max_hops: usize) -> usize {
            let mut processed = 0;
            while processed < max_hops {
                let Some(msg) = self.queue.pop_front() else {
                    break;
                };
                let label = format!("{msg:?}");
                let task = self.app.update(msg);
                let follow_ups = task.into_messages();
                self.recorder.record_message(label, follow_ups.len() as u32);
                for next in follow_ups {
                    self.queue.push_back(next);
                }
                processed += 1;
            }
            processed
        }
    }
}

/// Extended widgets (`iced_aw`) and Audio DSP widgets (`iced_audio`) descriptors.
pub mod widgets {
    use super::*;

    /// Normalized parameter in `0.0..=1.0` matching `iced_audio::Normal`.
    #[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
    pub struct NormalParam(f32);

    impl NormalParam {
        pub const MIN: Self = Self(0.0);
        pub const CENTER: Self = Self(0.5);
        pub const MAX: Self = Self(1.0);

        pub fn new(value: f32) -> Self {
            Self(value.clamp(0.0, 1.0))
        }

        pub const fn value(self) -> f32 {
            self.0
        }
    }

    impl Default for NormalParam {
        fn default() -> Self {
            Self::CENTER
        }
    }

    /// Bipolar modulation range on a rotary knob or fader (`iced_audio::ModRange`).
    #[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
    pub struct ModRange {
        pub start: NormalParam,
        pub end: NormalParam,
        pub enabled: bool,
    }

    impl ModRange {
        pub fn new(start: f32, end: f32) -> Self {
            Self {
                start: NormalParam::new(start),
                end: NormalParam::new(end),
                enabled: true,
            }
        }

        pub fn span(&self) -> f32 {
            self.end.value() - self.start.value()
        }
    }

    /// Logarithmic frequency parameter range (`iced_audio::FreqRange`, e.g. 20 Hz .. 20,000 Hz).
    #[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
    pub struct FreqRange {
        pub min_hz: f32,
        pub max_hz: f32,
    }

    impl FreqRange {
        pub fn audible_spectrum() -> Self {
            Self {
                min_hz: 20.0,
                max_hz: 20_000.0,
            }
        }

        pub fn map_to_normal(&self, hz: f32) -> NormalParam {
            let clamped = hz.clamp(self.min_hz, self.max_hz);
            let ratio = (clamped / self.min_hz).ln() / (self.max_hz / self.min_hz).ln();
            NormalParam::new(ratio)
        }

        pub fn map_from_normal(&self, normal: NormalParam) -> f32 {
            self.min_hz * (self.max_hz / self.min_hz).powf(normal.value())
        }
    }

    /// Rotary DSP knob configuration inspired by `iced_audio::Knob`.
    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct KnobConfig {
        pub label: String,
        pub normal: NormalParam,
        pub default_normal: NormalParam,
        pub diameter_px: f32,
        pub bipolar: bool,
        pub mod_range: Option<ModRange>,
    }

    impl KnobConfig {
        pub fn new(label: impl Into<String>, value: f32) -> Self {
            let normal = NormalParam::new(value);
            Self {
                label: label.into(),
                normal,
                default_normal: normal,
                diameter_px: 42.0,
                bipolar: false,
                mod_range: None,
            }
        }
    }

    /// Linear fader/slider configuration inspired by `iced_audio::VSlider` / `HSlider`.
    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct FaderConfig {
        pub label: String,
        pub normal: NormalParam,
        pub min_db: f32,
        pub max_db: f32,
        pub length_px: f32,
        pub vertical: bool,
    }

    impl FaderConfig {
        pub fn channel_strip(label: impl Into<String>, normal: f32) -> Self {
            Self {
                label: label.into(),
                normal: NormalParam::new(normal),
                min_db: -60.0,
                max_db: 6.0,
                length_px: 160.0,
                vertical: true,
            }
        }

        pub fn current_db(&self) -> f32 {
            self.min_db + (self.max_db - self.min_db) * self.normal.value()
        }
    }

    /// Split-pane layout configuration inspired by `iced_aw::Split`.
    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct SplitPaneConfig {
        pub divider_position: f32,
        pub vertical_split: bool,
        pub min_ratio: f32,
        pub max_ratio: f32,
    }

    impl Default for SplitPaneConfig {
        fn default() -> Self {
            Self {
                divider_position: 0.5,
                vertical_split: true,
                min_ratio: 0.15,
                max_ratio: 0.85,
            }
        }
    }

    /// Modal overlay configuration inspired by `iced_aw::Modal`.
    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct ModalConfig {
        pub title: String,
        pub open: bool,
        pub close_on_backdrop_click: bool,
        pub close_on_escape: bool,
        pub backdrop_opacity: f32,
    }

    impl ModalConfig {
        pub fn new(title: impl Into<String>) -> Self {
            Self {
                title: title.into(),
                open: false,
                close_on_backdrop_click: true,
                close_on_escape: true,
                backdrop_opacity: 0.65,
            }
        }
    }
}

/// Time-travel MVU telemetry (`aphrody-labs/comet`) & GPU text atlas metrics (`aphrody-labs/cryoglyph`).
pub mod comet {
    use super::*;

    /// Single MVU update span captured for the Comet debugger.
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub struct CometTraceSpan {
        pub sequence_id: u64,
        pub message_label: String,
        pub spawned_tasks: u32,
    }

    /// Ring-buffered recorder for Comet time-travel inspection.
    #[derive(Debug, Clone, Default, Serialize, Deserialize)]
    pub struct CometRecorder {
        capacity: usize,
        next_seq: u64,
        spans: VecDeque<CometTraceSpan>,
    }

    impl CometRecorder {
        pub fn new(capacity: usize) -> Self {
            Self {
                capacity: capacity.max(1),
                next_seq: 1,
                spans: VecDeque::with_capacity(capacity.max(1)),
            }
        }

        pub fn record_message(&mut self, message_label: impl Into<String>, spawned_tasks: u32) {
            if self.spans.len() >= self.capacity {
                self.spans.pop_front();
            }
            let seq = self.next_seq;
            self.next_seq += 1;
            self.spans.push_back(CometTraceSpan {
                sequence_id: seq,
                message_label: message_label.into(),
                spawned_tasks,
            });
        }

        pub fn spans(&self) -> &VecDeque<CometTraceSpan> {
            &self.spans
        }

        pub fn clear(&mut self) {
            self.spans.clear();
        }
    }

    /// Cryoglyph WGPU glyph atlas diagnostics snapshot (`aphrody-labs/cryoglyph`).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    pub struct CryoglyphAtlasStats {
        pub atlas_size_px: u32,
        pub cached_glyphs: u32,
        pub color_glyphs: u32,
    }

    impl Default for CryoglyphAtlasStats {
        fn default() -> Self {
            Self {
                atlas_size_px: 2048,
                cached_glyphs: 0,
                color_glyphs: 0,
            }
        }
    }
}

/// Submodule fleet manifest for the 10 forked `aphrody-labs` Iced repositories.
pub mod fleet {
    use super::*;

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    pub struct IcedSubmoduleSpec {
        pub name: &'static str,
        pub submodule_path: &'static str,
        pub upstream_repo: &'static str,
        pub aphrody_repo: &'static str,
        pub role: &'static str,
    }

    pub const ICED_SUBMODULE_FLEET: &[IcedSubmoduleSpec] = &[
        IcedSubmoduleSpec {
            name: "iced",
            submodule_path: "submodules/iced",
            upstream_repo: "iced-rs/iced",
            aphrody_repo: "aphrody-labs/iced",
            role: "Elm-inspired cross-platform GUI library (core, runtime, widget, wgpu, tiny_skia)",
        },
        IcedSubmoduleSpec {
            name: "iced_aw",
            submodule_path: "submodules/iced_aw",
            upstream_repo: "iced-rs/iced_aw",
            aphrody_repo: "aphrody-labs/iced_aw",
            role: "Additional community widgets (modals, split panes, tabs, badges, color picker)",
        },
        IcedSubmoduleSpec {
            name: "awesome-iced",
            submodule_path: "submodules/awesome-iced",
            upstream_repo: "iced-rs/awesome-iced",
            aphrody_repo: "aphrody-labs/awesome-iced",
            role: "Curated awesome list of Iced applications, libraries, and ecosystem extensions",
        },
        IcedSubmoduleSpec {
            name: "iced_audio",
            submodule_path: "submodules/iced_audio",
            upstream_repo: "iced-rs/iced_audio",
            aphrody_repo: "aphrody-labs/iced_audio",
            role: "Audio DSP plugin UI widgets (rotary knobs, faders, XY pads, FreqRange, ModRange)",
        },
        IcedSubmoduleSpec {
            name: "iced-book",
            submodule_path: "submodules/iced-book",
            upstream_repo: "iced-rs/book",
            aphrody_repo: "aphrody-labs/iced-book",
            role: "The official Iced guide and book",
        },
        IcedSubmoduleSpec {
            name: "comet",
            submodule_path: "submodules/comet",
            upstream_repo: "iced-rs/comet",
            aphrody_repo: "aphrody-labs/comet",
            role: "Elm-architecture time-travel debugger and runtime inspector for Iced",
        },
        IcedSubmoduleSpec {
            name: "mdbook-iced",
            submodule_path: "submodules/mdbook-iced",
            upstream_repo: "iced-rs/mdbook-iced",
            aphrody_repo: "aphrody-labs/mdbook-iced",
            role: "mdBook preprocessor for rendering interactive Iced snippets",
        },
        IcedSubmoduleSpec {
            name: "iced-rfcs",
            submodule_path: "submodules/iced-rfcs",
            upstream_repo: "iced-rs/rfcs",
            aphrody_repo: "aphrody-labs/iced-rfcs",
            role: "Iced Request for Comments (RFCs) and design specifications",
        },
        IcedSubmoduleSpec {
            name: "iced-opengl",
            submodule_path: "submodules/iced-opengl",
            upstream_repo: "iced-rs/opengl",
            aphrody_repo: "aphrody-labs/iced-opengl",
            role: "OpenGL renderer backend and Glow integration for Iced",
        },
        IcedSubmoduleSpec {
            name: "cryoglyph",
            submodule_path: "submodules/cryoglyph",
            upstream_repo: "iced-rs/cryoglyph",
            aphrody_repo: "aphrody-labs/cryoglyph",
            role: "Fast WGPU text rendering engine powering Iced",
        },
    ];
}

pub mod prelude {
    pub use super::comet::{CometRecorder, CometTraceSpan, CryoglyphAtlasStats};
    pub use super::fleet::{ICED_SUBMODULE_FLEET, IcedSubmoduleSpec};
    pub use super::mvu::{
        IcedApplication, MvuRuntime, SubscriptionDescriptor, SubscriptionKind, Task,
    };
    pub use super::widgets::{
        FaderConfig, FreqRange, KnobConfig, ModRange, ModalConfig, NormalParam, SplitPaneConfig,
    };
    pub use super::{IcedColor, IcedPalette, IcedSurfaceTokens, IcedTheme};
}

#[cfg(test)]
mod tests {
    use super::prelude::*;

    #[derive(Debug, Clone, PartialEq)]
    enum CounterMsg {
        Increment,
        Add(i32),
        CascadeDouble,
    }

    struct CounterApp {
        value: i32,
    }

    impl IcedApplication for CounterApp {
        type Flags = i32;
        type Message = CounterMsg;

        fn boot(initial: Self::Flags) -> (Self, Task<Self::Message>) {
            (Self { value: initial }, Task::done(CounterMsg::Increment))
        }

        fn title(&self) -> String {
            format!("Counter: {}", self.value)
        }

        fn update(&mut self, message: Self::Message) -> Task<Self::Message> {
            match message {
                CounterMsg::Increment => {
                    self.value += 1;
                    Task::none()
                }
                CounterMsg::Add(delta) => {
                    self.value += delta;
                    Task::none()
                }
                CounterMsg::CascadeDouble => {
                    let current = self.value;
                    Task::done(CounterMsg::Add(current))
                }
            }
        }
    }

    #[test]
    fn test_aphrody_dark_theme_and_css_parsing() {
        let theme = IcedTheme::aphrody_dark();
        assert_eq!(theme.name, "aphrody-m3-dark");
        assert_eq!(theme.palette.background.to_css_string(), "#090b10");
        assert_eq!(theme.palette.primary.to_css_string(), "#7c5cff");
        assert_eq!(
            IcedColor::parse_css("#00d4ff"),
            Some(theme.surfaces.accent_secondary)
        );
        let rgba = IcedColor::parse_css("rgba(255, 255, 255, 0.08)").unwrap();
        assert!((rgba.a - 0.08).abs() < 1e-4);
    }

    #[test]
    fn test_mvu_runtime_and_comet_recorder() {
        let mut runtime = MvuRuntime::<CounterApp>::new(10);
        // Boot task dispatched Increment -> 11
        assert_eq!(runtime.app.value, 11);
        assert_eq!(runtime.recorder.spans().len(), 1);

        let hops = runtime.dispatch(CounterMsg::CascadeDouble);
        assert_eq!(hops, 2);
        assert_eq!(runtime.app.value, 22);
        assert_eq!(runtime.recorder.spans().len(), 3);
    }

    #[test]
    fn test_iced_audio_freq_range_and_fader() {
        let freq = FreqRange::audible_spectrum();
        let norm = freq.map_to_normal(1000.0);
        let roundtrip = freq.map_from_normal(norm);
        assert!((roundtrip - 1000.0).abs() < 0.05);

        let fader = FaderConfig::channel_strip("Main Out", 1.0);
        assert!((fader.current_db() - 6.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_iced_submodule_fleet_completeness() {
        assert_eq!(ICED_SUBMODULE_FLEET.len(), 10);
        assert!(ICED_SUBMODULE_FLEET.iter().any(|s| s.name == "iced"));
        assert!(ICED_SUBMODULE_FLEET.iter().any(|s| s.name == "iced_aw"));
        assert!(ICED_SUBMODULE_FLEET.iter().any(|s| s.name == "iced_audio"));
        assert!(ICED_SUBMODULE_FLEET.iter().any(|s| s.name == "comet"));
        assert!(ICED_SUBMODULE_FLEET.iter().any(|s| s.name == "cryoglyph"));
    }
}
