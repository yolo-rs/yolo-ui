// SPDX-License-Identifier: Apache-2.0
//! React 19 Server Components (RSC) Flight Protocol Serializer & Streaming SSR Engine.
//!
//! Serializes React 19 server component trees into the React Flight wire format (`0:[...]`,
//! `1:I[...]`, `2:H[...]`) and renders hydrated HTML5 shells injected with `yolo-shared` /
//! `yolo-tokens` CSS variables for any [`YoloThemePreset`].

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use yolo_shared::{YoloThemePreset, css_preset_variables};

/// A React 19 Server Component node in the Flight tree.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum RscElement {
    /// Standard HTML or Server Component element (`["$", tag, key, props]`).
    Host {
        /// HTML tag name (e.g. `"div"`, `"main"`, `"section"`, `"button"`).
        tag: String,
        /// Optional React reconciliation key.
        key: Option<String>,
        /// Serializable HTML / React props (excluding `children`).
        props: serde_json::Map<String, Value>,
        /// Child nodes.
        children: Vec<RscElement>,
    },
    /// Plain text leaf node.
    Text {
        /// Text content.
        text: String,
    },
    /// Reference to a `"use client"` interactive module boundary (`$L<id>`).
    ClientReference {
        /// Flight chunk ID for the `I[module_id, export_name, chunks]` row.
        chunk_id: u32,
        /// Client module path (e.g. `"./ClientButton.tsx"`).
        module_path: String,
        /// Exported symbol name (e.g. `"Button"` or `"default"`).
        export_name: String,
        /// Serializable props passed from the Server Component to the Client Component.
        props: serde_json::Map<String, Value>,
    },
    /// React 19 `<Suspense>` boundary with fallback tree and streamed content.
    Suspense {
        /// Boundary identifier.
        boundary_id: String,
        /// Fallback element shown until the async child resolves.
        fallback: Box<RscElement>,
        /// Resolved child element.
        resolved: Box<RscElement>,
    },
}

impl RscElement {
    /// Creates a new host HTML element (`<div>`, `<main>`, `<button>`, etc.).
    pub fn host(tag: impl Into<String>) -> Self {
        Self::Host {
            tag: tag.into(),
            key: None,
            props: serde_json::Map::new(),
            children: Vec::new(),
        }
    }

    /// Creates a text node.
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text { text: text.into() }
    }

    /// Creates a `"use client"` reference node.
    pub fn client_ref(
        chunk_id: u32,
        module_path: impl Into<String>,
        export_name: impl Into<String>,
    ) -> Self {
        Self::ClientReference {
            chunk_id,
            module_path: module_path.into(),
            export_name: export_name.into(),
            props: serde_json::Map::new(),
        }
    }

    /// Adds a string attribute/prop to a `Host` or `ClientReference` element.
    pub fn with_prop(mut self, name: impl Into<String>, value: impl Into<Value>) -> Self {
        match &mut self {
            Self::Host { props, .. } | Self::ClientReference { props, .. } => {
                props.insert(name.into(), value.into());
            }
            _ => {}
        }
        self
    }

    /// Appends a child element to a `Host` element.
    pub fn with_child(mut self, child: RscElement) -> Self {
        if let Self::Host { children, .. } = &mut self {
            children.push(child);
        }
        self
    }
}

/// A single row in the React 19 Flight wire stream (`<id>:<tag><json>\n`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RscChunk {
    /// Numeric Flight row identifier (`0`, `1`, `2`, ...).
    pub id: u32,
    /// Optional Flight row prefix (`""` for model, `"I"` for client module import, `"H"` for preload hint).
    pub prefix: String,
    /// JSON payload for this row.
    pub payload: Value,
}

/// Converts an [`RscElement`] tree into its React Flight JSON representation and collects any
/// `"use client"` import chunks (`I[...]`).
fn element_to_flight_json(element: &RscElement, imports: &mut Vec<RscChunk>) -> Value {
    match element {
        RscElement::Text { text } => Value::String(text.clone()),
        RscElement::Host {
            tag,
            key,
            props,
            children,
        } => {
            let mut full_props = props.clone();
            if children.len() == 1 {
                full_props.insert(
                    String::from("children"),
                    element_to_flight_json(&children[0], imports),
                );
            } else if !children.is_empty() {
                let arr: Vec<Value> = children
                    .iter()
                    .map(|c| element_to_flight_json(c, imports))
                    .collect();
                full_props.insert(String::from("children"), Value::Array(arr));
            }
            json!(["$", tag, key, full_props])
        }
        RscElement::ClientReference {
            chunk_id,
            module_path,
            export_name,
            props,
        } => {
            imports.push(RscChunk {
                id: *chunk_id,
                prefix: String::from("I"),
                payload: json!([module_path, [chunk_id.to_string()], export_name]),
            });
            json!(["$", format!("$L{chunk_id}"), null, props])
        }
        RscElement::Suspense {
            boundary_id,
            fallback,
            resolved,
        } => {
            let fb = element_to_flight_json(fallback, imports);
            let res = element_to_flight_json(resolved, imports);
            json!([
                "$",
                "$Sreact.suspense",
                boundary_id,
                {
                    "fallback": fb,
                    "children": res
                }
            ])
        }
    }
}

/// Encodes an [`RscElement`] tree into a React 19 Flight wire stream (`application/x-react-flight`).
pub fn encode_flight_stream(root: &RscElement) -> String {
    let mut import_chunks = Vec::new();
    let root_model = element_to_flight_json(root, &mut import_chunks);

    let mut out = String::new();
    for chunk in &import_chunks {
        let serialized = serde_json::to_string(&chunk.payload).unwrap_or_default();
        out.push_str(&format!("{}:{}{}\n", chunk.id, chunk.prefix, serialized));
    }
    let root_serialized = serde_json::to_string(&root_model).unwrap_or_default();
    out.push_str(&format!("0:{root_serialized}\n"));
    out
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn render_element_inner_html(element: &RscElement) -> String {
    match element {
        RscElement::Text { text } => escape_html(text),
        RscElement::Host {
            tag,
            props,
            children,
            ..
        } => {
            let mut attrs = String::new();
            for (k, v) in props {
                let attr_name = if k == "className" { "class" } else { k.as_str() };
                if let Some(s) = v.as_str() {
                    attrs.push_str(&format!(" {}=\"{}\"", attr_name, escape_html(s)));
                } else if let Some(b) = v.as_bool() {
                    if b {
                        attrs.push_str(&format!(" {attr_name}"));
                    }
                } else if let Some(n) = v.as_f64() {
                    attrs.push_str(&format!(" {attr_name}=\"{n}\""));
                }
            }
            let inner: String = children.iter().map(render_element_inner_html).collect();
            format!("<{tag}{attrs}>{inner}</{tag}>")
        }
        RscElement::ClientReference {
            chunk_id,
            module_path,
            export_name,
            props,
        } => {
            let label = props
                .get("label")
                .or_else(|| props.get("children"))
                .and_then(Value::as_str)
                .unwrap_or(export_name.as_str());
            format!(
                "<div data-yolo-client-ref=\"{chunk_id}\" data-module=\"{}\" data-export=\"{}\">{}</div>",
                escape_html(module_path),
                escape_html(export_name),
                escape_html(label)
            )
        }
        RscElement::Suspense {
            boundary_id,
            resolved,
            ..
        } => {
            format!(
                "<!--$?:{boundary_id}-->{}<!--/$-->",
                render_element_inner_html(resolved)
            )
        }
    }
}

/// Renders an [`RscElement`] tree into a complete HTML5 SSR document with inline `:root`
/// `--yolo-*` CSS variables for `preset` and an embedded React 19 Flight `<script>` stream.
pub fn render_rsc_to_html(
    root: &RscElement,
    title: &str,
    preset: YoloThemePreset,
    dark: bool,
) -> String {
    let css = css_preset_variables(preset, dark);
    let body_html = render_element_inner_html(root);
    let flight_stream = encode_flight_stream(root).replace('<', "\\u003c");
    let mode_class = if dark { "dark" } else { "light" };

    format!(
        "<!DOCTYPE html>\n\
<html lang=\"en\" class=\"{mode_class}\" data-yolo-preset=\"{preset_id}\">\n\
<head>\n\
  <meta charset=\"utf-8\" />\n\
  <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\" />\n\
  <title>{title}</title>\n\
  <style id=\"yolo-theme-vars\">{css}</style>\n\
</head>\n\
<body style=\"margin:0;background:var(--yolo-bg-primary);color:var(--yolo-text-primary);font-family:var(--yolo-font-sans);\">\n\
  <div id=\"root\">{body_html}</div>\n\
  <script id=\"__YOLO_RSC_FLIGHT__\" type=\"application/x-react-flight\">{flight_stream}</script>\n\
</body>\n\
</html>",
        preset_id = preset.id(),
        title = escape_html(title),
    )
}
