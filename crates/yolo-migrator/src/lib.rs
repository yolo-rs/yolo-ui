// SPDX-License-Identifier: Apache-2.0
//! `yolo-migrator`: Automated React + Shadcn UI + Radix UI + Base UI + Material 3 + Tailwind CSS
//! compiler & emitter to Rust WebGPU/WASM/Fullstack UI backends:
//! - [`MigrationTarget::Dioxus`] — Dioxus 0.8 `rsx!` fullstack / WebGPU / WASM component
//! - [`MigrationTarget::Wgpu`] — Pure Rust WebGPU / WGPU + Taffy (`yolo-layout`) scene descriptor
//! - [`MigrationTarget::Egui`] — Immediate-mode `egui` 0.32+ Rust UI module
//! - [`MigrationTarget::Cosmic`] — `libcosmic` / `iced` 0.14 Elm MVU view module
//! - [`MigrationTarget::Gtk`] — GTK 4.16 + libadwaita 1.6 `GtkBuilder` XML + sidecar CSS
//! - [`MigrationTarget::Qml`] — Qt 6.8+ Qt Quick Controls Material + Kirigami QML

pub mod cosmic;
pub mod css;
pub mod diag;
pub mod dioxus;
pub mod egui;
pub mod gtk;
pub mod ir;
pub mod qml;
pub mod shadcn_registry;
pub mod tsx;
pub mod wgpu;

use std::path::Path;

pub use diag::Diag;
pub use ir::Document;
pub use shadcn_registry::{
    extract_cva_specs, resolve_primitive_import, CvaSpec, HeadlessEcosystem, PrimitiveDescriptor,
    ShadcnConfig, ShadcnRegistryItem, TailwindV4Theme,
};
pub use yolo_shared::YoloThemePreset;

/// Target UI framework / runtime emitted by [`migrate_tsx`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MigrationTarget {
    /// Dioxus 0.8 `rsx!` fullstack / WebGPU / WASM component (`#[component] pub fn ... -> Element`).
    Dioxus,
    /// Pure Rust WebGPU / WGPU + Taffy (`yolo-layout`) layout & primitive scene descriptor.
    Wgpu,
    /// Immediate-mode `egui` 0.32+ Rust snippet (`State` + `Actions` + `fn *_ui`).
    Egui,
    /// `libcosmic` / `iced` 0.14 Elm MVU Rust view (`pub fn view(...) -> Element<'_, Message>`).
    Cosmic,
    /// GTK 4.16 + libadwaita 1.6 `GtkBuilder` `.ui` XML + optional sidecar `.gtk.css`.
    Gtk,
    /// Qt 6.8+ Qt Quick Controls (Material + Kirigami) `.qml`.
    Qml,
}

impl MigrationTarget {
    /// All 6 supported migration targets in canonical order.
    pub const ALL: [Self; 6] = [
        Self::Dioxus,
        Self::Wgpu,
        Self::Egui,
        Self::Cosmic,
        Self::Gtk,
        Self::Qml,
    ];

    /// Canonical lowercase target identifier (`"dioxus"`, `"wgpu"`, `"egui"`, `"cosmic"`, `"gtk"`, `"qml"`).
    pub const fn id(self) -> &'static str {
        match self {
            Self::Dioxus => "dioxus",
            Self::Wgpu => "wgpu",
            Self::Egui => "egui",
            Self::Cosmic => "cosmic",
            Self::Gtk => "gtk",
            Self::Qml => "qml",
        }
    }

    /// Parses a target identifier (`"dioxus"`, `"wgpu"`, `"egui"`, `"cosmic"`, `"iced"`, `"gtk"`, `"qml"`, `"qt"`).
    pub fn from_id(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "dioxus" | "rsx" => Some(Self::Dioxus),
            "wgpu" | "webgpu" | "taffy" => Some(Self::Wgpu),
            "egui" => Some(Self::Egui),
            "cosmic" | "libcosmic" | "iced" => Some(Self::Cosmic),
            "gtk" | "gtk4" | "adwaita" => Some(Self::Gtk),
            "qml" | "qt" | "qt6" => Some(Self::Qml),
            _ => None,
        }
    }
}

/// Output produced by [`migrate_tsx`] or [`migrate_tsx_file`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationOutput {
    /// Lowered component name from the TSX source (e.g. `"LoginCard"`).
    pub component: String,
    /// Target backend that produced `code`.
    pub target: MigrationTarget,
    /// Active theme preset applied during emission.
    pub theme: YoloThemePreset,
    /// Primary emitted source code (`.rs`, `.ui`, or `.qml`).
    pub code: String,
    /// Optional sidecar stylesheet (e.g. `<stem>.gtk.css` for `MigrationTarget::Gtk`).
    pub sidecar_css: Option<String>,
}

/// Lowers an in-memory React / Shadcn UI / Radix UI / Base UI / Material 3 + Tailwind CSS TSX snippet
/// and emits code for `target` themed with `theme`.
pub fn migrate_tsx(
    source: &str,
    target: MigrationTarget,
    theme: YoloThemePreset,
) -> Result<MigrationOutput, Vec<Diag>> {
    migrate_tsx_file(Path::new("Component.tsx"), source, target, theme)
}

/// Lowers a TSX file at `path` (resolving relative `.css` imports against `path.parent()`)
/// and emits code for `target` themed with `theme`.
pub fn migrate_tsx_file(
    path: &Path,
    source: &str,
    target: MigrationTarget,
    theme: YoloThemePreset,
) -> Result<MigrationOutput, Vec<Diag>> {
    let doc = tsx::lower(path, source)?;
    Ok(emit_document(&doc, target, theme))
}

/// Emits an already-lowered [`Document`] to `target` with `theme`.
pub fn emit_document(
    doc: &Document,
    target: MigrationTarget,
    theme: YoloThemePreset,
) -> MigrationOutput {
    let (code, sidecar_css) = match target {
        MigrationTarget::Dioxus => (dioxus::emit(doc, theme), None),
        MigrationTarget::Wgpu => (wgpu::emit(doc, theme), None),
        MigrationTarget::Egui => (egui::emit_with_theme(doc, theme), None),
        MigrationTarget::Cosmic => (cosmic::emit(doc, theme), None),
        MigrationTarget::Gtk => {
            let out = gtk::emit_with_theme(doc, theme);
            (out.ui, out.css)
        }
        MigrationTarget::Qml => (qml::emit_with_theme(doc, theme), None),
    };
    MigrationOutput {
        component: doc.component.clone(),
        target,
        theme,
        code,
        sidecar_css,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SHADCN_TAILWIND_SNIPPET: &str = r#"
import { Card, CardHeader, CardTitle, CardDescription, CardContent, CardFooter } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";

export function WorkspaceSignIn({ onSignIn }) {
    return (
        <Card className="w-full max-w-md p-6 rounded-xl shadow-lg bg-card border">
            <CardHeader className="flex flex-col gap-2">
                <div className="flex flex-row items-center justify-between">
                    <CardTitle className="text-xl font-semibold">Sign in to Yolo UI</CardTitle>
                    <Badge variant="secondary">v0.8</Badge>
                </div>
                <CardDescription className="text-sm text-muted-foreground">
                    Enter your workspace credentials below
                </CardDescription>
            </CardHeader>
            <CardContent className="flex flex-col gap-4 py-4">
                <Input placeholder="you@example.com" type="email" />
                <Button
                    onClick={onSignIn}
                    className="flex items-center gap-2 p-4 rounded-lg bg-primary text-primary-foreground"
                >
                    Continue with SSO
                </Button>
            </CardContent>
        </Card>
    );
}
"#;

    #[test]
    fn shadcn_tailwind_emits_all_six_targets_across_themes() {
        for target in MigrationTarget::ALL {
            for theme in YoloThemePreset::ALL {
                let out = migrate_tsx(SHADCN_TAILWIND_SNIPPET, target, theme)
                    .unwrap_or_else(|diags| panic!("failed for {target:?} / {theme:?}: {diags:?}"));
                assert_eq!(out.component, "WorkspaceSignIn");
                assert_eq!(out.target, target);
                assert_eq!(out.theme, theme);
                assert!(!out.code.is_empty());

                match target {
                    MigrationTarget::Dioxus => {
                        assert!(out.code.contains("#[component]"));
                        assert!(out.code.contains("pub fn WorkspaceSignIn("));
                        assert!(out.code.contains("rsx! {"));
                        assert!(out.code.contains("Sign in to Yolo UI"));
                        assert!(out.code.contains("Continue with SSO"));
                        assert!(out.code.contains(theme.id()));
                    }
                    MigrationTarget::Wgpu => {
                        assert!(out.code.contains("pub struct WorkspaceSignInWgpuScene"));
                        assert!(out.code.contains("UiTree::new()"));
                        assert!(out.code.contains("WgpuPrimitive"));
                        assert!(out.code.contains("Continue with SSO"));
                    }
                    MigrationTarget::Egui => {
                        assert!(out.code.contains("pub trait WorkspaceSignInActions"));
                        assert!(out.code.contains("fn on_sign_in(&mut self)"));
                        assert!(out.code.contains("pub fn workspace_sign_in_ui("));
                        assert!(out.code.contains("egui::Button::new"));
                    }
                    MigrationTarget::Cosmic => {
                        assert!(out.code.contains("pub enum WorkspaceSignInMessage"));
                        assert!(out.code.contains("OnSignIn"));
                        assert!(out.code.contains("pub fn view<'a>"));
                        assert!(out.code.contains("Element<'a, Message>"));
                    }
                    MigrationTarget::Gtk => {
                        assert!(out.code.contains("<interface>"));
                        assert!(out.code.contains("<requires lib=\"gtk\" version=\"4.16\"/>"));
                        assert!(out.code.contains("GtkButton"));
                        assert!(out.code.contains("GtkEntry"));
                        assert!(out.sidecar_css.is_some());
                    }
                    MigrationTarget::Qml => {
                        assert!(out.code.contains("import QtQuick"));
                        assert!(out.code.contains("signal signIn()"));
                        assert!(out.code.contains("Button {"));
                        assert!(out.code.contains("TextField {"));
                    }
                }
            }
        }
    }

    #[test]
    fn radix_tabs_and_dialog_lower_cleanly() {
        let src = r#"
import * as Tabs from "@radix-ui/react-tabs";
import { Switch } from "@/components/ui/switch";
import { Slider } from "@/components/ui/slider";

export function AudioSettings() {
    return (
        <Tabs defaultValue="output" className="flex flex-col gap-4 p-4">
            <TabsList className="flex flex-row gap-2">
                <TabsTrigger value="output">Output</TabsTrigger>
                <TabsTrigger value="input">Input</TabsTrigger>
            </TabsList>
            <TabsContent value="output" className="flex flex-col gap-3">
                <Switch label="Spatial Audio" defaultChecked />
                <Slider defaultValue={[75]} min={0} max={100} step={1} label="Master Volume" />
            </TabsContent>
            <TabsContent value="input" className="flex flex-col gap-3">
                <Switch label="Noise Suppression" />
            </TabsContent>
        </Tabs>
    );
}
"#;
        for target in MigrationTarget::ALL {
            let out = migrate_tsx(src, target, YoloThemePreset::Neutral)
                .unwrap_or_else(|e| panic!("failed on {target:?}: {e:?}"));
            assert_eq!(out.component, "AudioSettings");
            assert!(!out.code.is_empty());
        }
    }
}
