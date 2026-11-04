//! Unified `yolo.aphrody.com` Portal & 6-Pillar Rust Migration Engine (`apps/yolo-showcase/src/site_hub.rs`).
//!
//! Unifies:
//! 1. **Awesome Catalog (`/awesome`)**: 11 Awesome lists + 124 `yolo-rs/*` submodules across 31 ecosystems.
//! 2. **Multi-Engine & 7-Theme Showcase (`/showcase`)**: 7 `YoloThemePreset`s, 23 Omarchy subthemes,
//!    Byron Love 8x11 pet, 9 Rust UI toolkits, 13 Craft workbenches, and 10 Storyteller modules.
//! 3. **Starter Templates (`/templates`)**: 8 production Rust templates (Dioxus Site, Iced Desktop,
//!    Egui Workbench, Zed GPUI DevTool, WGPUI 3D Viewport, Taffy Layout, Native Rust CLI, Tauri v3).
//! 4. **6-Pillar Master Migration Matrix (`/migration`)**:
//!    - Sites Web -> **Dioxus 0.8 (`yolo-ui`)**
//!    - Apps Desktop -> **Iced 0.14 (`yolo-iced`/`yolo-cosmic`)** or **Egui 0.36 (`yolo-egui`/`yolo-craft`)**
//!    - CSS / Tailwind -> **Taffy 0.15 (`yolo-layout`)**
//!    - 3D & GPU Rendering -> **GPUI + WGPU 30/29 + WGPUI (`yolo-gpu` + `yolo-render` + `yolo-gpui` + Bevy 0.16)**
//!    - Dev UIs & Agent IDEs -> **Zed & GPUI (`yolo-gpui` + `gpui-component`)**
//!    - Tools & CLIs -> **Native Rust (`crates/*` binaries)**

use yolo_ui::craft::workbenches::all_workbenches;
use yolo_ui::shared::{
    BYRON_LOVE_PALETTE, BYRON_LOVE_PET_SPEC, HYPRLAND_DECORATION_SPEC, OMARCHY_THEMES,
    YoloThemePreset,
};

pub use crate::benchmarks::{ALL_BENCHMARKS, FrameworkBenchmark, render_benchmark_section_html};

/// Domain of the 6-Pillar Master Migration Plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MigrationPillar {
    /// Pillar 1: All web & fullstack sites -> Dioxus 0.8 RSX + Server Functions + Blitz.
    SitesToDioxus,
    /// Pillar 2a: Retained-mode desktop shells, stores & media apps -> Iced 0.14 / libcosmic.
    DesktopToIced,
    /// Pillar 2b: Immediate-mode creative workbenches, DAWs, CAD & emulators -> Egui 0.36.
    DesktopToEgui,
    /// Pillar 3: CSS / Tailwind v4 utilities -> Taffy 0.15 Flexbox/Grid (`yolo-layout`).
    CssToTaffy,
    /// Pillar 4: 3D scenes, shaders & viewports -> GPUI + WGPU 30/29 + WGPUI (`yolo-gpu` / `yolo-render`).
    Render3dToWgpui,
    /// Pillar 5: Developer UIs, IDEs, MCP inspectors & agent cockpits -> Zed & GPUI (`yolo-gpui`).
    DevUiToZedGpui,
    /// Pillar 6: CLIs, build scripts & data tools -> Native Rust binaries (`crates/*`).
    ToolsToNativeRust,
}

impl MigrationPillar {
    pub const ALL: [Self; 7] = [
        Self::SitesToDioxus,
        Self::DesktopToIced,
        Self::DesktopToEgui,
        Self::CssToTaffy,
        Self::Render3dToWgpui,
        Self::DevUiToZedGpui,
        Self::ToolsToNativeRust,
    ];

    pub const fn id(self) -> &'static str {
        match self {
            Self::SitesToDioxus => "sites-to-dioxus",
            Self::DesktopToIced => "desktop-to-iced",
            Self::DesktopToEgui => "desktop-to-egui",
            Self::CssToTaffy => "css-to-taffy",
            Self::Render3dToWgpui => "render3d-to-wgpui",
            Self::DevUiToZedGpui => "dev-ui-to-zed-gpui",
            Self::ToolsToNativeRust => "tools-to-native-rust",
        }
    }

    pub const fn title(self) -> &'static str {
        match self {
            Self::SitesToDioxus => "1. Sites Web & Fullstack -> Dioxus 0.8 (SSR + WASM + Blitz)",
            Self::DesktopToIced => "2A. Apps Desktop Retained MVU -> Iced 0.14 & libcosmic",
            Self::DesktopToEgui => "2B. Apps Desktop Workbenches & Emulateurs -> Egui 0.36 (Craft & Storyteller)",
            Self::CssToTaffy => "3. CSS & Tailwind v4 -> Taffy 0.15 Layout Engine (yolo-layout)",
            Self::Render3dToWgpui => "4. Rendu 3D & Shaders -> GPUI + WGPU 30/29 + WGPUI (yolo-gpu & Bevy 0.16)",
            Self::DevUiToZedGpui => "5. UI de Dev, IDE & Cockpits Agents -> Zed & GPUI (yolo-gpui)",
            Self::ToolsToNativeRust => "6. Outils, CLIs & Pipelines -> Rust Natif (Cargo Binaries)",
        }
    }

    pub const fn target_crate(self) -> &'static str {
        match self {
            Self::SitesToDioxus => "packages/yolo-ui (dioxus 0.8 + blitz + subsecond)",
            Self::DesktopToIced => "packages/yolo-iced + packages/yolo-cosmic",
            Self::DesktopToEgui => "packages/yolo-egui + crates/yolo-craft + crates/yolo-storyteller",
            Self::CssToTaffy => "crates/yolo-layout (taffy 0.15 + TailwindStyleResolver)",
            Self::Render3dToWgpui => "crates/yolo-gpu + crates/yolo-render + packages/yolo-gpui",
            Self::DevUiToZedGpui => "packages/yolo-gpui (zed gpui + longbridge gpui-component)",
            Self::ToolsToNativeRust => "crates/yolo-migrator + apps/yolo-showcase",
        }
    }
}

/// A concrete project/app entry in the master migration matrix.
#[derive(Debug, Clone, Copy)]
pub struct MigrationTargetEntry {
    pub name: &'static str,
    pub source_repo: &'static str,
    pub legacy_stack: &'static str,
    pub pillar: MigrationPillar,
    pub target_stack: &'static str,
    pub theme_preset: YoloThemePreset,
}

/// Complete inventory of all sites, desktop apps, CSS pipelines, 3D engines, dev UIs, and CLIs
/// across `yolo-ui`, `bun-os`, `bun-ui`, `yolo-web`, `rosegriffon`, `shenron`, `wii-os`, `auto`, and `ft`.
pub const MASTER_MIGRATION_ENTRIES: &[MigrationTargetEntry] = &[
    // Pillar 1: Sites Web -> Dioxus 0.8 (`yolo-ui`)
    MigrationTargetEntry {
        name: "yolo.aphrody.com (Unified Hub: Awesome + Showcase + Templates)",
        source_repo: "yolo-ui/apps/yolo-showcase",
        legacy_stack: "Fragmented Next.js / Astro / Vite showcases",
        pillar: MigrationPillar::SitesToDioxus,
        target_stack: "Dioxus 0.8 Fullstack SSR/WASM + Taffy + 7 YoloThemePresets",
        theme_preset: YoloThemePreset::Neutral,
    },
    MigrationTargetEntry {
        name: "aphrody-web & m3-showcase",
        source_repo: "bun-os/apps/aphrody-web & bun-ui/apps/m3-web",
        legacy_stack: "Next.js 15 + React + Tailwind CSS",
        pillar: MigrationPillar::SitesToDioxus,
        target_stack: "Dioxus 0.8 RSX + #[server] + yolo-m3 / yolo-aphrody",
        theme_preset: YoloThemePreset::Aphrody,
    },
    MigrationTargetEntry {
        name: "13 Craft Web Apps (*craft-web: art, cad, deck, design, effect, fig, film, grid, light, pdf, photo, sound, vector, word)",
        source_repo: "bun-os/apps/*craft-web",
        legacy_stack: "React + Vite + Tailwind wrappers",
        pillar: MigrationPillar::SitesToDioxus,
        target_stack: "Dioxus 0.8 Web/WASM Shell + Embedded Egui/WGPUI Canvas",
        theme_preset: YoloThemePreset::Neutral,
    },
    MigrationTargetEntry {
        name: "iecode-web / iecode-web-wasm / iecode-web-wii-wasm",
        source_repo: "bun-os/apps/iecode-web*",
        legacy_stack: "TypeScript + WebGL / WASM",
        pillar: MigrationPillar::SitesToDioxus,
        target_stack: "Dioxus 0.8 WASM + WGPUI 3D Level-5 Asset Viewer",
        theme_preset: YoloThemePreset::Aphrody,
    },
    MigrationTargetEntry {
        name: "rg-website & azalee-web (Rose Griffon)",
        source_repo: "rosegriffon/apps/rg-website & azalee-web",
        legacy_stack: "Next.js + Shadcn UI + Radix + Tailwind",
        pillar: MigrationPillar::SitesToDioxus,
        target_stack: "Dioxus 0.8 Fullstack (migrated via `bun ui migrate --target dioxus`)",
        theme_preset: YoloThemePreset::Aphrody,
    },
    MigrationTargetEntry {
        name: "shenron-site & shenron-activity",
        source_repo: "shenron/apps/site & shenron/apps/activity",
        legacy_stack: "React + Discord Activity SDK + Tailwind",
        pillar: MigrationPillar::SitesToDioxus,
        target_stack: "Dioxus 0.8 LiveView / WASM + yolo-omarchy",
        theme_preset: YoloThemePreset::Omarchy,
    },
    MigrationTargetEntry {
        name: "auto-web & auto-admin",
        source_repo: "auto/apps/web & auto/apps/admin",
        legacy_stack: "React + Shadcn UI + TanStack Table",
        pillar: MigrationPillar::SitesToDioxus,
        target_stack: "Dioxus 0.8 Fullstack + yolo-contracts DataTable + yolo-fluent",
        theme_preset: YoloThemePreset::Fluent,
    },
    MigrationTargetEntry {
        name: "ft-home & ft-design-system",
        source_repo: "ft/apps/home & ft/apps/design-system",
        legacy_stack: "React + Storybook + CSS Modules",
        pillar: MigrationPillar::SitesToDioxus,
        target_stack: "Dioxus 0.8 Component Docs integrated into yolo.aphrody.com",
        theme_preset: YoloThemePreset::Apple,
    },
    // Pillar 2A: Desktop Retained MVU -> Iced 0.14 & libcosmic (`yolo-iced` / `yolo-cosmic`)
    MigrationTargetEntry {
        name: "bunos-win11-chrome & desktop shell",
        source_repo: "bun-os/crates/bunos-win11-chrome & apps/desktop",
        legacy_stack: "Win32 / WebView hybrid shell",
        pillar: MigrationPillar::DesktopToIced,
        target_stack: "Iced 0.14 + libcosmic (`yolo-iced` / `yolo-cosmic`) + yolo-fluent Mica",
        theme_preset: YoloThemePreset::Fluent,
    },
    MigrationTargetEntry {
        name: "craft-hub & bunos-flutter-store",
        source_repo: "bun-os/apps/craft-hub & bunos-flutter-store",
        legacy_stack: "Flutter / Tauri launcher",
        pillar: MigrationPillar::DesktopToIced,
        target_stack: "Iced 0.14 Elm MVU App Store & Creative Suite Launcher (`yolo-iced`)",
        theme_preset: YoloThemePreset::M3,
    },
    MigrationTargetEntry {
        name: "spacedrive & spotcraft",
        source_repo: "bun-os/apps/spacedrive & spotcraft",
        legacy_stack: "React + Tauri + Audio WebView",
        pillar: MigrationPillar::DesktopToIced,
        target_stack: "Iced 0.14 + `iced_audio` DSP Faders/Knobs + Comet (`yolo-iced`)",
        theme_preset: YoloThemePreset::Apple,
    },
    MigrationTargetEntry {
        name: "wii-os menu-native & auto-desktop",
        source_repo: "wii-os/apps/menu-native & auto/apps/desktop",
        legacy_stack: "GTK / Electron / Tauri",
        pillar: MigrationPillar::DesktopToIced,
        target_stack: "Iced 0.14 Multi-Window + WGPU Custom Shader Pass (`yolo-iced`)",
        theme_preset: YoloThemePreset::Hyprland,
    },
    // Pillar 2B: Desktop Immediate-Mode Workbenches -> Egui 0.36 (`yolo-egui` / `yolo-craft` / `yolo-storyteller`)
    MigrationTargetEntry {
        name: "13 Craft Creative Desktop Workbenches (Word, Grid, Deck, Pdf, Photo, Vector, Light, Film, Effect, Sound, Design, Fig, Cad)",
        source_repo: "bun-os/apps/*craft & crates/yolo-craft",
        legacy_stack: "Mixed Egui 0.31 / Tauri / Web wrappers",
        pillar: MigrationPillar::DesktopToEgui,
        target_stack: "Egui 0.36 Unified (`yolo-egui` + `yolo-craft` DockArea + Puffin Profiler)",
        theme_preset: YoloThemePreset::Neutral,
    },
    MigrationTargetEntry {
        name: "10 Storyteller Narrative Studio Modules (Dashboard, Timeline, Graph, Lore, Character, Storyboard, Comic, Novel, Dialogue, Prompt)",
        source_repo: "crates/yolo-storyteller",
        legacy_stack: "Standalone prototype crates",
        pillar: MigrationPillar::DesktopToEgui,
        target_stack: "Egui 0.36 + `yolo-storyteller` + Byron Love 10-Teinte Character Sheet",
        theme_preset: YoloThemePreset::Aphrody,
    },
    MigrationTargetEntry {
        name: "Emulators & Debuggers (gecko-debugger, aphrody-ctr-desktop, ruzu)",
        source_repo: "wii-os/crates/gecko-debugger & bun-os/crates/aphrody-ctr-desktop",
        legacy_stack: "Custom C++/Qt/legacy egui",
        pillar: MigrationPillar::DesktopToEgui,
        target_stack: "Egui 0.36 Immediate-Mode Telemetry + WGPU Framebuffer (`yolo-egui`)",
        theme_preset: YoloThemePreset::Omarchy,
    },
    // Pillar 3: CSS / Tailwind v4 -> Taffy 0.15 (`yolo-layout`)
    MigrationTargetEntry {
        name: "Tailwind v4 & CSS Stylesheets across all Web/Desktop apps",
        source_repo: "packages/yolo-shadcn-bun & crates/yolo-layout",
        legacy_stack: "PostCSS / Tailwind CLI / Browser DOM CSS",
        pillar: MigrationPillar::CssToTaffy,
        target_stack: "`yolo-layout` Taffy 0.15 (`TailwindStyleResolver` + 4px/8px grid snap + 48px WCAG)",
        theme_preset: YoloThemePreset::Neutral,
    },
    // Pillar 4: 3D & GPU Rendering -> GPUI + WGPU 30/29 + WGPUI (`yolo-gpu` + `yolo-render` + `yolo-gpui`)
    MigrationTargetEntry {
        name: "aphrody-model-view, aphrody-nie-render3d & aphrody-tauri-3d",
        source_repo: "bun-os/apps/aphrody-model-view & crates/aphrody-nie-render3d",
        legacy_stack: "Three.js / WebGL / standalone wgpu 0.20",
        pillar: MigrationPillar::Render3dToWgpui,
        target_stack: "WGPU 30/29 + WGPUI (`yolo-gpu` Instanced SDF + `yolo-gpui` GPU Canvas + Bevy 0.16)",
        theme_preset: YoloThemePreset::Aphrody,
    },
    MigrationTargetEntry {
        name: "gecko-backend-wgpu, aphrody-dust-wgpu & vectorcraft-three-d",
        source_repo: "wii-os/crates/gecko-backend-wgpu & bun-os/crates/aphrody-dust-wgpu",
        legacy_stack: "Disparate wgpu pipelines",
        pillar: MigrationPillar::Render3dToWgpui,
        target_stack: "Unified `yolo-gpu` + `yolo-render` (Analytical SDF, 8x3 Kawase blur, Glyph Atlas)",
        theme_preset: YoloThemePreset::Hyprland,
    },
    // Pillar 5: Dev UIs, IDEs & Agent Cockpits -> Zed & GPUI (`yolo-gpui`)
    MigrationTargetEntry {
        name: "aphrody-agent-home, ai-agent-runner & shenron-cockpit",
        source_repo: "bun-os/crates/aphrody-agent-home & yolo-web/apps/ai-agent-runner",
        legacy_stack: "React / Electron / TUI dashboards",
        pillar: MigrationPillar::DevUiToZedGpui,
        target_stack: "Zed GPUI + Longbridge `gpui-component` (`yolo-gpui` 120 FPS Dock + VirtualList)",
        theme_preset: YoloThemePreset::Omarchy,
    },
    MigrationTargetEntry {
        name: "aphrody-code-graph, aphrody-term & MCP Inspector Studio (ft/mcp, FCMCP)",
        source_repo: "bun-os/crates/aphrody-code-graph, aphrody-term & FCMCP",
        legacy_stack: "xterm.js / React Flow / Node MCP inspector",
        pillar: MigrationPillar::DevUiToZedGpui,
        target_stack: "Zed GPUI Workspace (`yolo-gpui` TreeDiff + CommandPalette + GPUI Terminal)",
        theme_preset: YoloThemePreset::Neutral,
    },
    // Pillar 6: Tools & CLIs -> Native Rust (`crates/*`)
    MigrationTargetEntry {
        name: "All 14 *craft-cli, iecode-dotnet, obscura-cli & gecko-multitool",
        source_repo: "bun-os/apps/*-cli, yolo-web/crates/obscura-cli & wii-os/crates/gecko-multitool",
        legacy_stack: "C# (.NET) / Node.js / Python scripts",
        pillar: MigrationPillar::ToolsToNativeRust,
        target_stack: "Native Rust 2024 Cargo Binaries (`yolo-migrate`, `yolo-showcase`, `obscura-cli`)",
        theme_preset: YoloThemePreset::Neutral,
    },
];

/// Starter template descriptor for `yolo.aphrody.com/templates`.
#[derive(Debug, Clone, Copy)]
pub struct YoloStarterTemplate {
    pub id: &'static str,
    pub title: &'static str,
    pub pillar: MigrationPillar,
    pub stack: &'static str,
    pub command: &'static str,
    pub description: &'static str,
}

/// The 8 official starter templates available on `yolo.aphrody.com/templates`.
pub const YOLO_STARTER_TEMPLATES: &[YoloStarterTemplate] = &[
    YoloStarterTemplate {
        id: "dioxus-fullstack-site",
        title: "Dioxus 0.8 Fullstack Web & SSR Site",
        pillar: MigrationPillar::SitesToDioxus,
        stack: "yolo-ui + dioxus 0.8 + blitz + subsecond + taffy",
        command: "bun run ui migrate ./src/App.tsx --target dioxus --preset neutral",
        description: "Fullstack SSR + WASM + Server Functions site with hot-patching and 7 YoloThemePresets.",
    },
    YoloStarterTemplate {
        id: "iced-desktop-mvu",
        title: "Iced 0.14 & libcosmic Desktop MVU App",
        pillar: MigrationPillar::DesktopToIced,
        stack: "yolo-iced + yolo-cosmic + iced_aw + iced_audio",
        command: "bun run ui migrate ./src/Shell.tsx --target cosmic --preset fluent",
        description: "Retained-mode Elm MVU desktop application with Wayland/Win11 vibrancy and audio DSP widgets.",
    },
    YoloStarterTemplate {
        id: "egui-craft-workbench",
        title: "Egui 0.36 Creative Workbench & Storyteller Studio",
        pillar: MigrationPillar::DesktopToEgui,
        stack: "yolo-egui + yolo-craft + yolo-storyteller + puffin",
        command: "bun run ui migrate ./src/Workbench.tsx --target egui --preset aphrody",
        description: "60/120 FPS immediate-mode creative workbench with multi-surface docking and Byron Love palette.",
    },
    YoloStarterTemplate {
        id: "zed-gpui-devtool",
        title: "Zed GPUI & Longbridge GPUI-Component DevTool / IDE",
        pillar: MigrationPillar::DevUiToZedGpui,
        stack: "yolo-gpui + gpui-component + zed blade renderer",
        command: "bun run ui devtools gpui",
        description: "Hardware-accelerated 120 FPS developer cockpit with DockArea, VirtualList, CommandPalette, and TreeDiff.",
    },
    YoloStarterTemplate {
        id: "wgpui-3d-viewport",
        title: "GPUI + WGPU 30/29 + WGPUI 3D & Analytical SDF Viewport",
        pillar: MigrationPillar::Render3dToWgpui,
        stack: "yolo-gpu + yolo-render + yolo-gpui + bevy 0.16",
        command: "bun run ui migrate ./src/Viewport.tsx --target wgpu --preset hyprland",
        description: "Zero-DOM instanced SDF primitives, 8x3 Kawase dual-filter blur, and 3D WGPU scene surface inside GPUI.",
    },
    YoloStarterTemplate {
        id: "taffy-tailwind-layout",
        title: "Tailwind v4 -> Taffy 0.15 Pure Rust Layout Tree",
        pillar: MigrationPillar::CssToTaffy,
        stack: "yolo-layout + taffy 0.15 + yolo-contracts",
        command: "bun run ui system \"responsive bento dashboard\" --preset omarchy",
        description: "Compile Tailwind Flexbox/Grid classes directly into Taffy layout nodes with 4px/8px snapping and 48px WCAG targets.",
    },
    YoloStarterTemplate {
        id: "rust-native-cli",
        title: "Native Rust 2024 CLI & MCP Server",
        pillar: MigrationPillar::ToolsToNativeRust,
        stack: "yolo-shared + yolo-migrator + oxc + clap",
        command: "cargo run -p yolo-migrator -- --help",
        description: "Zero-Node native Rust CLI binary with OXC AST parsing, token emission, and JSON-RPC 2.0 MCP transport.",
    },
    YoloStarterTemplate {
        id: "tauri-v3-hybrid",
        title: "Tauri v3 Alpha Multi-Webview (Wry 0.57 + CEF) Shell",
        pillar: MigrationPillar::SitesToDioxus,
        stack: "yolo-tauri + wry 0.57 + tao 0.37 + acl v3",
        command: "bun run ui devtools tauri",
        description: "Capability-hardened Tauri v3 Alpha desktop/mobile host bridging Dioxus Webviews and WGPU surfaces.",
    },
];

/// Curated Awesome ecosystem category fused into `yolo.aphrody.com/awesome`.
#[derive(Debug, Clone, Copy)]
pub struct AwesomeHubCategory {
    pub id: &'static str,
    pub title: &'static str,
    pub upstream_repo: &'static str,
    pub submodule_count: usize,
    pub highlights: &'static str,
}

/// The 11 Awesome ecosystems unified on `yolo.aphrody.com/awesome`.
pub const AWESOME_HUB_CATEGORIES: &[AwesomeHubCategory] = &[
    AwesomeHubCategory {
        id: "awesome-gpui",
        title: "Zed GPUI & Longbridge GPUI-Component",
        upstream_repo: "yolo-rs/awesome-gpui & yolo-rs/gpui-component",
        submodule_count: 14,
        highlights: "Zed GPUI, gpui-component (60+ widgets),Loungy, Roam, DockArea, VirtualList, CommandPalette",
    },
    AwesomeHubCategory {
        id: "awesome-dioxus",
        title: "Dioxus 0.8 Fullstack, Blitz & Subsecond",
        upstream_repo: "yolo-rs/awesome-dioxus & yolo-rs/blitz",
        submodule_count: 15,
        highlights: "Dioxus 0.8 RSX, Blitz WGPU HTML/CSS renderer, Subsecond hot-patching, Dioxus Primitives, Taffy",
    },
    AwesomeHubCategory {
        id: "awesome-iced",
        title: "Iced 0.14 Elm MVU, Audio DSP & System76 libcosmic",
        upstream_repo: "yolo-rs/awesome-iced & yolo-rs/libcosmic",
        submodule_count: 23,
        highlights: "Iced 0.14, iced_aw, iced_audio, Halloy, System76 libcosmic, COSMIC Applets, cosmic-text",
    },
    AwesomeHubCategory {
        id: "awesome-egui",
        title: "Egui 0.36 Immediate-Mode, Rerun & Puffin",
        upstream_repo: "yolo-rs/egui & yolo-rs/rerun",
        submodule_count: 8,
        highlights: "Egui 0.36, eframe, egui_tiles, egui_plot, Rerun SDK, Puffin profiler, 13 Craft Workbenches",
    },
    AwesomeHubCategory {
        id: "awesome-wgpu-bevy",
        title: "WGPU 30/29, WGPUI, Vello & Bevy 0.16 3D Engine",
        upstream_repo: "yolo-rs/wgpu & yolo-rs/bevy",
        submodule_count: 11,
        highlights: "WGPU 30/29, Naga WGSL, Vello compute 2D, Bevy 0.16 ECS, kayak_ui, bevy_egui, Analytical SDF",
    },
    AwesomeHubCategory {
        id: "awesome-tauri",
        title: "Tauri v3 Alpha, Wry 0.57, Tao 0.37 & Mobile Plugins",
        upstream_repo: "yolo-rs/tauri & yolo-rs/tauri-workspace",
        submodule_count: 17,
        highlights: "Tauri v3 Alpha ACL, Wry 0.57, CEF backend, muda, tray-icon, global-hotkey, window-vibrancy",
    },
    AwesomeHubCategory {
        id: "awesome-design-systems",
        title: "7 Design Systems (Neutral, M3, Fluent, Apple, Omarchy 23, Hyprland, Aphrody)",
        upstream_repo: "yolo-rs/omarchy & yolo-rs/hyprland",
        submodule_count: 20,
        highlights: "Material 3 HCT, Fluent 2 Mica, Apple Liquid Glass, Omarchy 23 themes, Hyprland 45deg, Byron Love",
    },
    AwesomeHubCategory {
        id: "awesome-shadcn-tailwind-rust",
        title: "Shadcn UI, Base UI, Radix & Tailwind v4 -> Rust Compiler",
        upstream_repo: "yolo-rs/shadcn-ui, yolo-rs/base-ui & yolo-rs/tailwindcss",
        submodule_count: 4,
        highlights: "OXC TSX parser, TailwindStyleResolver -> Taffy, CVA -> Rust enums, 6 Rust UI targets",
    },
    AwesomeHubCategory {
        id: "awesome-react-expo-bun-rust",
        title: "React 19, React Native, Expo & Rust RSC/JSI Engine",
        upstream_repo: "yolo-rs/react, yolo-rs/expo & yolo-rs/react-native",
        submodule_count: 12,
        highlights: "React 19 RSC Flight stream in Rust, RN StyleSheet -> Taffy, JSI C++20/Rust TurboModule codegen",
    },
];

/// Renders the complete standalone `yolo.aphrody.com` unified HTML portal combining:
/// - `/showcase` (7 Theme Presets, 23 Omarchy palettes, Byron Love 8x11 Pet, 9 Rust Engines, 13 Craft Workbenches)
/// - `/awesome` (Awesome GPUI, Dioxus, Iced, Egui, WGPU/Bevy, Tauri v3, Design Systems, 124 submodules)
/// - `/templates` (8 copy-paste starter templates + Live TSX -> Dioxus/Iced/Egui/GPUI/WGPU preview)
/// - `/migration` (The 6-Pillar Master Migration Matrix for all sites, desktop apps, CSS, 3D, dev UIs & CLIs)
#[must_use]
pub fn render_yolo_aphrody_com_html() -> String {
    let workbenches = all_workbenches();

    let mut presets_cards = String::new();
    for preset in YoloThemePreset::ALL {
        let tokens = preset.tokens(true);
        let style = preset.style();
        presets_cards.push_str(&format!(
            r##"<div class="card preset-card" data-preset="{id}" onclick="window.setYoloPreset('{id}')" style="border-top: 3px solid {accent}; cursor: pointer;">
                <div class="card-header">
                    <span class="badge" style="background:{accent};color:#09090b">{id}</span>
                    <code>{seed}</code>
                </div>
                <h3>{title}</h3>
                <p>Surface <code>{bg}</code> • Accent <code>{accent}</code> • Radius <code>{rad}px</code> • Blur <code>{blur}px</code></p>
            </div>"##,
            id = preset.id(),
            seed = preset.seed_hex(),
            title = preset.display_name(),
            bg = tokens.bg_primary,
            accent = tokens.accent_primary,
            rad = style.radius_md_px,
            blur = style.backdrop_blur_px,
        ));
    }

    let mut migration_rows = String::new();
    for entry in MASTER_MIGRATION_ENTRIES {
        migration_rows.push_str(&format!(
            r#"<tr data-pillar="{pillar_id}">
                <td><strong>{name}</strong><br><small><code>{repo}</code></small></td>
                <td><span class="badge-outline">{pillar}</span></td>
                <td><code>{legacy}</code></td>
                <td><strong>{target}</strong></td>
                <td><span class="badge">{preset}</span></td>
            </tr>"#,
            pillar_id = entry.pillar.id(),
            name = entry.name,
            repo = entry.source_repo,
            pillar = entry.pillar.id(),
            legacy = entry.legacy_stack,
            target = entry.target_stack,
            preset = entry.theme_preset.id(),
        ));
    }

    let mut template_cards = String::new();
    for tpl in YOLO_STARTER_TEMPLATES {
        template_cards.push_str(&format!(
            r#"<div class="card">
                <div class="card-header">
                    <span class="badge">{id}</span>
                    <span class="badge-outline">{pillar}</span>
                </div>
                <h3>{title}</h3>
                <p>{desc}</p>
                <p><small>Stack: <code>{stack}</code></small></p>
                <pre><code>{cmd}</code></pre>
            </div>"#,
            id = tpl.id,
            pillar = tpl.pillar.id(),
            title = tpl.title,
            desc = tpl.description,
            stack = tpl.stack,
            cmd = tpl.command,
        ));
    }

    let mut awesome_cards = String::new();
    for cat in AWESOME_HUB_CATEGORIES {
        awesome_cards.push_str(&format!(
            r#"<div class="card">
                <div class="card-header">
                    <span class="badge">{id}</span>
                    <code>{count} submodules</code>
                </div>
                <h3>{title}</h3>
                <p><small><code>{repo}</code></small></p>
                <p>{highlights}</p>
            </div>"#,
            id = cat.id,
            count = cat.submodule_count,
            title = cat.title,
            repo = cat.upstream_repo,
            highlights = cat.highlights,
        ));
    }

    let mut workbench_badges = String::new();
    for wb in workbenches {
        workbench_badges.push_str(&format!(
            r#"<span class="badge-outline" title="{title}">{id} ({panels} panels)</span> "#,
            id = wb.id,
            title = wb.display_name,
            panels = wb.default_panels.len(),
        ));
    }

    format!(
        r##"<!DOCTYPE html>
<html lang="fr" data-yolo-preset="yolo-neutral">
<head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>yolo.aphrody.com — Unified Dioxus, Iced, Egui, Taffy, WGPUI &amp; Zed GPUI Hub</title>
    <style>
        :root {{
            --yolo-bg: #09090b;
            --yolo-surface: #18181b;
            --yolo-elevated: #27272a;
            --yolo-text: #fafafa;
            --yolo-muted: #a1a1aa;
            --yolo-accent: #6366f1;
            --yolo-accent-2: #818cf8;
            --yolo-radius: 8px;
        }}
        * {{ box-sizing: border-box; }}
        body {{
            margin: 0;
            font-family: Inter, -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
            background: var(--yolo-bg);
            color: var(--yolo-text);
            line-height: 1.55;
        }}
        header {{
            position: sticky;
            top: 0;
            z-index: 20;
            background: rgba(9, 9, 11, 0.86);
            backdrop-filter: blur(18px);
            border-bottom: 1px solid rgba(255,255,255,0.08);
            padding: 0.9rem 2rem;
            display: flex;
            flex-wrap: wrap;
            align-items: center;
            justify-content: space-between;
            gap: 1rem;
        }}
        .brand {{ display: flex; align-items: center; gap: 0.75rem; font-weight: 700; font-size: 1.15rem; }}
        .brand-dot {{ width: 12px; height: 12px; border-radius: 50%; background: var(--yolo-accent); box-shadow: 0 0 14px var(--yolo-accent); }}
        nav {{ display: flex; gap: 0.5rem; flex-wrap: wrap; }}
        nav a {{
            color: var(--yolo-text);
            text-decoration: none;
            padding: 0.4rem 0.85rem;
            border-radius: var(--yolo-radius);
            background: var(--yolo-surface);
            font-size: 0.9rem;
            border: 1px solid rgba(255,255,255,0.07);
        }}
        nav a:hover {{ border-color: var(--yolo-accent); color: var(--yolo-accent); }}
        main {{ max-width: 1360px; margin: 0 auto; padding: 2rem; }}
        .hero {{
            padding: 2.5rem;
            border-radius: calc(var(--yolo-radius) * 1.5);
            background: linear-gradient(135deg, var(--yolo-surface), var(--yolo-elevated));
            border: 1px solid rgba(255,255,255,0.1);
            margin-bottom: 2.5rem;
        }}
        .hero h1 {{ margin: 0 0 0.75rem; font-size: 2.2rem; }}
        .grid {{ display: grid; grid-template-columns: repeat(auto-fill, minmax(300px, 1fr)); gap: 1.15rem; margin-bottom: 2.5rem; }}
        .card {{
            background: var(--yolo-surface);
            border: 1px solid rgba(255,255,255,0.08);
            border-radius: var(--yolo-radius);
            padding: 1.2rem;
        }}
        .card h3 {{ margin: 0.6rem 0 0.4rem; font-size: 1.05rem; }}
        .card-header {{ display: flex; justify-content: space-between; align-items: center; gap: 0.5rem; }}
        .badge {{
            background: var(--yolo-accent);
            color: #09090b;
            font-weight: 600;
            font-size: 0.76rem;
            padding: 0.2rem 0.55rem;
            border-radius: 999px;
        }}
        .badge-outline {{
            display: inline-block;
            border: 1px solid rgba(255,255,255,0.18);
            color: var(--yolo-muted);
            font-size: 0.76rem;
            padding: 0.2rem 0.55rem;
            border-radius: 999px;
            margin: 0.15rem;
        }}
        table {{ width: 100%; border-collapse: collapse; background: var(--yolo-surface); border-radius: var(--yolo-radius); overflow: hidden; margin-bottom: 2.5rem; }}
        th, td {{ text-align: left; padding: 0.85rem 1rem; border-bottom: 1px solid rgba(255,255,255,0.07); font-size: 0.9rem; }}
        th {{ background: var(--yolo-elevated); font-weight: 600; }}
        pre {{ background: #050507; padding: 0.75rem; border-radius: 6px; overflow-x: auto; font-size: 0.82rem; border: 1px solid rgba(255,255,255,0.07); }}
        section {{ margin-bottom: 3rem; scroll-margin-top: 5rem; }}
        .chart-container {{ display: grid; grid-template-columns: repeat(auto-fit, minmax(360px, 1fr)); gap: 1.5rem; margin: 1.5rem 0 2.5rem; }}
        .chart-box {{ background: var(--yolo-surface); border: 1px solid rgba(255,255,255,0.08); border-radius: var(--yolo-radius); padding: 1.25rem; }}
        .bar-row {{ display: flex; align-items: center; margin: 0.6rem 0; font-size: 0.85rem; }}
        .bar-label {{ width: 140px; font-weight: 500; white-space: nowrap; }}
        .bar-track {{ flex: 1; background: rgba(255,255,255,0.05); height: 22px; border-radius: 4px; overflow: hidden; display: flex; }}
        .bar-fill {{ height: 100%; display: flex; align-items: center; padding-left: 8px; font-size: 0.76rem; font-weight: 600; color: #09090b; white-space: nowrap; min-width: 48px; border-radius: 4px; }}
        .baseline-row {{ background: rgba(239, 68, 68, 0.08) !important; }}
        .rust-row {{ background: rgba(16, 185, 129, 0.04); }}
        .metric-val {{ font-family: ui-monospace, SFMono-Regular, Menlo, monospace; font-weight: 600; color: var(--yolo-text); }}
        .progress-bar {{ width: 70px; height: 8px; background: rgba(255,255,255,0.1); border-radius: 4px; overflow: hidden; display: inline-block; vertical-align: middle; margin-right: 6px; }}
        .progress-fill {{ height: 100%; }}
        .benchmark-card {{ font-size: 0.9rem; }}
        .benchmark-card h4 {{ margin: 0; font-size: 1.05rem; }}
        .sweet-spot {{ color: #10B981; margin: 0.4rem 0; }}
        .anti-pattern {{ color: #EF4444; margin: 0.4rem 0; }}
        .pro-tag {{ font-weight: 600; }}
        .con-tag {{ font-weight: 600; }}
        .table-responsive {{ overflow-x: auto; -webkit-overflow-scrolling: touch; }}
    </style>
</head>
<body>
    <header>
        <div class="brand">
            <span class="brand-dot"></span>
            <span>yolo.aphrody.com</span>
            <span class="badge-outline">Dioxus 0.8 • Iced 0.14 • Egui 0.36 • Taffy 0.15 • WGPUI 30/29 • Zed GPUI</span>
        </div>
        <nav>
            <a href="#showcase">Showcase (7 Presets &amp; 9 Engines)</a>
            <a href="#migration">Plan de Migration (6 Piliers)</a>
            <a href="#templates">Starter Templates (8)</a>
            <a href="#awesome">Awesome Hub (124 Submodules)</a>
            <a href="#benchmarks">Benchmarks &amp; Comparatifs</a>
        </nav>
    </header>
    <main>
        <div class="hero">
            <h1>yolo.aphrody.com — Portail Unifié Awesome • Showcase • Templates • Migration Rust</h1>
            <p>
                Architecture unifiée : <strong>Sites Web en Dioxus 0.8</strong> •
                <strong>Apps Desktop en Iced 0.14 ou Egui 0.36</strong> •
                <strong>CSS/Tailwind en Taffy 0.15</strong> •
                <strong>Rendu 3D en GPUI + WGPU 30/29 + WGPUI</strong> •
                <strong>UI de Dev en Zed &amp; GPUI</strong> •
                <strong>Outils en Rust Natif</strong>.
            </p>
            <p>
                <strong>7 Presets Thèmes</strong> (Neutral par défaut, M3, Fluent, Apple, Omarchy {omarchy_count} sous-thèmes, Hyprland {hypr_angle}deg, Aphrody Byron Love {byron_cols}x{byron_rows} {byron_teintes} teintes) •
                <strong>124 Sous-modules <code>yolo-rs/*</code></strong>.
            </p>
        </div>

        <section id="showcase">
            <h2>1. Interactive Theme &amp; Multi-Engine Showcase (Click a Preset to Switch Live)</h2>
            <div class="grid">{presets_cards}</div>
            <div class="card">
                <h3>13 Craft Egui 0.36 Workbenches &amp; 10 Storyteller Modules</h3>
                <p>{workbench_badges}</p>
            </div>
        </section>

        <section id="migration">
            <h2>2. Master Migration Plan — 6 Piliers ({migration_count} Cibles Unifiées)</h2>
            <table>
                <thead>
                    <tr>
                        <th>Projet / Application</th>
                        <th>Pilier</th>
                        <th>Stack Legacy</th>
                        <th>Cible Rust Unifiée (Yolo UI)</th>
                        <th>Preset</th>
                    </tr>
                </thead>
                <tbody>{migration_rows}</tbody>
            </table>
        </section>

        <section id="templates">
            <h2>3. Starter Templates Unifiés ({template_count} Templates Prêts à Cloner)</h2>
            <div class="grid">{template_cards}</div>
        </section>

        <section id="awesome">
            <h2>4. Awesome Ecosystem Hub ({awesome_count} Familles • 124 Sous-modules <code>yolo-rs/*</code>)</h2>
            <div class="grid">{awesome_cards}</div>
        </section>

        {benchmarks_html}
    </main>
    <script>
        const PRESET_VARS = {{
            "yolo-neutral":  {{ bg: "#09090b", surface: "#18181b", elevated: "#27272a", accent: "#6366f1", radius: "8px" }},
            "yolo-m3":       {{ bg: "#141218", surface: "#211f26", elevated: "#2b2930", accent: "#d0bcff", radius: "12px" }},
            "yolo-fluent":   {{ bg: "#1f1f1f", surface: "#292929", elevated: "#333333", accent: "#60cdff", radius: "4px" }},
            "yolo-apple":    {{ bg: "#1c1c1e", surface: "#2c2c2e", elevated: "#3a3a3c", accent: "#0a84ff", radius: "10px" }},
            "yolo-omarchy":  {{ bg: "#1a1b26", surface: "#24283b", elevated: "#414868", accent: "#7aa2f7", radius: "0px" }},
            "yolo-hyprland": {{ bg: "#0b0e14", surface: "#111622", elevated: "#192030", accent: "#33ccff", radius: "10px" }},
            "yolo-aphrody":  {{ bg: "#090b10", surface: "#111520", elevated: "#181e2e", accent: "#7c5cff", radius: "12px" }}
        }};
        window.setYoloPreset = function(id) {{
            const p = PRESET_VARS[id] || PRESET_VARS["yolo-neutral"];
            const r = document.documentElement;
            r.setAttribute("data-yolo-preset", id);
            r.style.setProperty("--yolo-bg", p.bg);
            r.style.setProperty("--yolo-surface", p.surface);
            r.style.setProperty("--yolo-elevated", p.elevated);
            r.style.setProperty("--yolo-accent", p.accent);
            r.style.setProperty("--yolo-radius", p.radius);
        }};
    </script>
</body>
</html>"##,
        omarchy_count = OMARCHY_THEMES.len(),
        hypr_angle = HYPRLAND_DECORATION_SPEC.active_border_angle_deg,
        byron_cols = BYRON_LOVE_PET_SPEC.grid_cols,
        byron_rows = BYRON_LOVE_PET_SPEC.grid_rows,
        byron_teintes = BYRON_LOVE_PALETTE.len(),
        presets_cards = presets_cards,
        workbench_badges = workbench_badges,
        migration_count = MASTER_MIGRATION_ENTRIES.len(),
        migration_rows = migration_rows,
        template_count = YOLO_STARTER_TEMPLATES.len(),
        template_cards = template_cards,
        awesome_count = AWESOME_HUB_CATEGORIES.len(),
        awesome_cards = awesome_cards,
        benchmarks_html = render_benchmark_section_html(),
    )
}
