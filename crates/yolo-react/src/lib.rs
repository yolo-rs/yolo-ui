// SPDX-License-Identifier: Apache-2.0
//! `yolo-react`: Pure-Rust React 19 & React Native / Expo SDK 53+ Engine for `yolo-rs/yolo-ui`.
//!
//! Unifies:
//! - [`rsc`]: React 19 Server Components (RSC) Flight Protocol serializer & streaming SSR HTML engine.
//! - [`react_native`]: React Native `StyleSheet` -> Rust Taffy 0.15 (`yolo-layout`) layout engine
//!   (replacing C++ Yoga) and C++ JSI `TurboModule` + TypeScript spec code generator.
//! - [`compiler`]: Native `oxc_parser` (`0.155`) React 19 component, hook, directive (`"use client"`,
//!   `"use server"`, `"use dom"`), and React Compiler auto-memoization boundary analyzer.
//! - [`theme_bridge`]: 7-Preset (`Neutral`, `M3`, `Fluent`, `Apple`, `Omarchy` + 23 subthemes,
//!   `Hyprland`, `Aphrody` Byron Love) Web CSS + React Native `StyleSheet` bridge.

pub mod compiler;
pub mod react_native;
pub mod rsc;
pub mod theme_bridge;

pub use compiler::{ReactComponentAnalysis, analyze_react_component};
pub use react_native::{
    RnStyle, RnStyleSheet, RnViewNode, TurboRustGeneratedFiles, TurboRustMethod,
    TurboRustModuleSpec, compute_rn_layout, emit_turborust_module, find_layout_rect,
};
pub use rsc::{RscChunk, RscElement, encode_flight_stream, render_rsc_to_html};
pub use theme_bridge::{
    ReactNativeColorTokens, ReactNativeGeometry, ReactNativeHctRoles, ReactNativeThemePayload,
    build_react_native_theme, export_react_native_theme_json, export_react_web_css,
};
pub use yolo_shared::{OmarchySubTheme, YoloManifest, YoloThemePreset, YoloThemeTokens};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rsc_flight_stream_and_ssr_html_render_across_presets() {
        let tree = RscElement::host("main")
            .with_prop("className", "yolo-shell p-6")
            .with_child(
                RscElement::host("h1").with_child(RscElement::text("Yolo React 19 RSC Engine")),
            )
            .with_child(
                RscElement::client_ref(1, "./ClientCounter.tsx", "ClientCounter")
                    .with_prop("initial", 42)
                    .with_prop("label", "Interactive Counter"),
            );

        let flight = encode_flight_stream(&tree);
        assert!(flight.contains("1:I[\"./ClientCounter.tsx\",[\"1\"],\"ClientCounter\"]"));
        assert!(flight.contains("0:[\"$\",\"main\""));
        assert!(flight.contains("\"$L1\""));

        for preset in YoloThemePreset::ALL {
            let html = render_rsc_to_html(&tree, "Yolo App", preset, true);
            assert!(html.contains(preset.id()));
            assert!(html.contains("Yolo React 19 RSC Engine"));
            assert!(html.contains("__YOLO_RSC_FLIGHT__"));
            assert!(html.contains("data-yolo-client-ref=\"1\""));
        }
    }

    #[test]
    fn react_native_stylesheet_computes_taffy_layout_and_emits_jsi_turbomodule() {
        let root = RnViewNode {
            name: String::from("screen"),
            style: RnStyle {
                flex: Some(1.0),
                flex_direction: Some(String::from("column")),
                padding: Some(16.0),
                gap: Some(12.0),
                ..RnStyle::default()
            },
            text: None,
            children: vec![
                RnViewNode {
                    name: String::from("header"),
                    style: RnStyle {
                        height: Some(56.0),
                        ..RnStyle::default()
                    },
                    text: None,
                    children: vec![],
                },
                RnViewNode {
                    name: String::from("card"),
                    style: RnStyle {
                        height: Some(120.0),
                        padding_horizontal: Some(20.0),
                        ..RnStyle::default()
                    },
                    text: None,
                    children: vec![],
                },
            ],
        };

        let layout = compute_rn_layout(&root, 390.0, 844.0).expect("Taffy RN layout succeeds");
        let screen_rect = find_layout_rect(&layout, "screen").unwrap();
        assert_eq!(screen_rect.width, 390.0);
        assert_eq!(screen_rect.height, 844.0);

        let header_rect = find_layout_rect(&layout, "header").unwrap();
        assert_eq!(header_rect.x, 16.0);
        assert_eq!(header_rect.y, 16.0);
        assert_eq!(header_rect.width, 358.0);
        assert_eq!(header_rect.height, 56.0);

        let card_rect = find_layout_rect(&layout, "card").unwrap();
        assert_eq!(card_rect.x, 16.0);
        assert_eq!(card_rect.y, 16.0 + 56.0 + 12.0);
        assert_eq!(card_rect.height, 120.0);

        let spec = TurboRustModuleSpec {
            module_name: String::from("YoloRustBridge"),
            rust_crate: String::from("yolo_react"),
            methods: vec![TurboRustMethod {
                name: String::from("computeRnLayoutJson"),
                params: vec![
                    (
                        String::from("treeJson"),
                        String::from("string"),
                        String::from("jsi::String"),
                    ),
                    (
                        String::from("width"),
                        String::from("number"),
                        String::from("double"),
                    ),
                ],
                return_ts: String::from("string"),
                return_cpp: String::from("jsi::String"),
            }],
        };

        let generated = emit_turborust_module(&spec);
        assert!(generated.typescript_spec.contains("TurboModuleRegistry.getEnforcing<Spec>(\"YoloRustBridge\")"));
        assert!(generated.typescript_spec.contains("computeRnLayoutJson(treeJson: string, width: number): string;"));
        assert!(generated.cpp_jsi_header.contains("class JSI_EXPORT YoloRustBridgeCxxSpecJSI"));
    }

    #[test]
    fn oxc_compiler_analyzes_react_19_hooks_and_directives() {
        let src = r#"
"use client";
import { useState, useMemo, useEffect, useOptimistic } from "react";

export function DashboardPanel({ title, initialCount, onSync }) {
    const [count, setCount] = useState(initialCount);
    const [optimisticCount] = useOptimistic(count);
    const doubled = useMemo(() => count * 2, [count]);

    useEffect(() => {
        onSync(doubled);
    }, [doubled, onSync]);

    return (
        <section className="flex flex-col gap-4">
            <h1>{title}</h1>
            <Button onClick={() => setCount(count + 1)}>{optimisticCount}</Button>
        </section>
    );
}
"#;

        let report = analyze_react_component(src).expect("OXC parses valid TSX");
        assert_eq!(report.component_name, "DashboardPanel");
        assert!(report.is_use_client);
        assert!(!report.is_use_server);
        assert_eq!(report.props, vec!["title", "initialCount", "onSync"]);
        assert!(report.hooks_used.contains(&String::from("useState")));
        assert!(report.hooks_used.contains(&String::from("useOptimistic")));
        assert!(report.hooks_used.contains(&String::from("useMemo")));
        assert!(report.hooks_used.contains(&String::from("useEffect")));
        assert!(report.jsx_elements.contains(&String::from("section")));
        assert!(report.jsx_elements.contains(&String::from("Button")));
        assert!(report.react_compiler_memoizable);
    }

    #[test]
    fn theme_bridge_exports_all_seven_presets_and_twenty_three_omarchy_subthemes() {
        for preset in YoloThemePreset::ALL {
            let web_css = export_react_web_css(preset, true);
            assert!(web_css.contains(preset.id()));
            assert!(web_css.contains("--md-sys-color-primary:"));

            let rn_json = export_react_native_theme_json(preset, true, None);
            assert!(rn_json.contains(preset.id()));
        }

        for sub in OmarchySubTheme::ALL {
            let payload = build_react_native_theme(YoloThemePreset::Omarchy, true, Some(sub));
            assert_eq!(payload.omarchy_subtheme.as_deref(), Some(sub.spec().id));
            assert_eq!(payload.tokens.bg_primary, sub.spec().background);
        }

        let hypr = build_react_native_theme(YoloThemePreset::Hyprland, true, None);
        assert!(hypr.hyprland_border_gradient.unwrap().contains("#33ccff"));

        let aphrody = build_react_native_theme(YoloThemePreset::Aphrody, true, None);
        assert_eq!(aphrody.byron_love_palette.unwrap().len(), 10);
    }
}
