// SPDX-License-Identifier: Apache-2.0
//! The UI IR: a tree of Shadcn/Radix/BaseUI/Material 3 contract components, layout boxes, text
//! and images with a closed style subset and Tailwind utility lowering. Backends only read this
//! tree; they never see TSX or raw CSS.

use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Str(String),
    Num(f64),
    Bool(bool),
}

/// Contract components recognised from Shadcn UI, Radix UI, Base UI, `@aphrody/m3-baseui` and `material-web`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Comp {
    Button,
    IconButton,
    Fab,
    Switch,
    Checkbox,
    Radio,
    Slider,
    TextField,
    Card,
    Dialog,
    List,
    ListItem,
    Divider,
    Tabs,
    Tab,
    TopAppBar,
    NavigationRail,
    NavigationRailItem,
    Chip,
    Progress,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextTag {
    Heading(u8),
    Paragraph,
    Span,
    Label,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Comp(Comp),
    Box,
    Text(TextTag),
    Image,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dir {
    Row,
    Column,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Align {
    Start,
    Center,
    End,
    Stretch,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Justify {
    Start,
    Center,
    End,
    SpaceBetween,
}

/// Closed style subset. Colors are semantic role names (`primary`, `on-primary`, `on-surface-variant`),
/// typescale names are `title-large` style, lengths are pixels.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Style {
    pub display_flex: bool,
    pub display_grid: bool,
    pub grid_cols: Option<u16>,
    pub direction: Option<Dir>,
    pub gap: Option<f64>,
    pub padding: Option<[f64; 4]>,
    pub margin: Option<[f64; 4]>,
    pub align: Option<Align>,
    pub justify: Option<Justify>,
    pub text_align: Option<Align>,
    pub grow: bool,
    pub fill_width: bool,
    pub fill_height: bool,
    pub color: Option<String>,
    pub background: Option<String>,
    pub border_color: Option<String>,
    pub border_width: Option<f64>,
    pub elevation: Option<u8>,
    pub typescale: Option<String>,
    pub shape: Option<String>,
    pub width: Option<f64>,
    pub height: Option<f64>,
    pub max_width: Option<f64>,
    pub scroll: bool,
}

#[derive(Clone, Debug)]
pub struct Event {
    /// JSX prop, e.g. `onClick`.
    pub prop: String,
    /// Handler name as written in TSX, e.g. `onSave`.
    pub handler: String,
}

#[derive(Clone, Debug)]
pub struct Node {
    pub kind: Kind,
    pub tag_name: String,
    pub classes: Vec<String>,
    pub props: BTreeMap<String, Value>,
    pub text: Option<String>,
    pub events: Vec<Event>,
    pub style: Style,
    pub children: Vec<Node>,
    pub id: Option<String>,
    pub slot: Option<String>,
    pub aria_label: Option<String>,
    pub line: u32,
    pub col: u32,
}

impl Node {
    pub fn new(kind: Kind, line: u32, col: u32) -> Self {
        Self {
            kind,
            tag_name: String::new(),
            classes: Vec::new(),
            props: BTreeMap::new(),
            text: None,
            events: Vec::new(),
            style: Style::default(),
            children: Vec::new(),
            id: None,
            slot: None,
            aria_label: None,
            line,
            col,
        }
    }

    pub fn comp(&self) -> Option<Comp> {
        match self.kind {
            Kind::Comp(c) => Some(c),
            _ => None,
        }
    }

    pub fn is(&self, c: Comp) -> bool {
        self.kind == Kind::Comp(c)
    }

    pub fn str(&self, key: &str) -> Option<&str> {
        match self.props.get(key) {
            Some(Value::Str(s)) if !s.is_empty() => Some(s),
            _ => None,
        }
    }

    pub fn flag(&self, key: &str) -> bool {
        match self.props.get(key) {
            Some(Value::Bool(b)) => *b,
            Some(Value::Str(s)) => s != "false",
            Some(Value::Num(n)) => *n != 0.0,
            None => false,
        }
    }

    pub fn num(&self, key: &str) -> Option<f64> {
        match self.props.get(key) {
            Some(Value::Num(n)) => Some(*n),
            Some(Value::Str(s)) => s.parse().ok(),
            _ => None,
        }
    }

    /// First present string prop among `keys`, else the text children.
    pub fn label(&self, keys: &[&str]) -> Option<&str> {
        keys.iter().find_map(|k| self.str(k)).or(self.text.as_deref())
    }

    pub fn event(&self, props: &[&str]) -> Option<&Event> {
        self.events.iter().find(|e| props.contains(&e.prop.as_str()))
    }

    pub fn variant(&self) -> &str {
        match self.str("variant").unwrap_or("") {
            "default" => "filled",
            "secondary" => "tonal",
            "outline" => "outlined",
            "ghost" | "link" => "text",
            other => other,
        }
    }

    /// Main axis of a layout container: CSS defaults to block (column) flow unless
    /// `display: flex` is set without a direction.
    pub fn dir(&self) -> Dir {
        self.style.direction.unwrap_or(if self.style.display_flex { Dir::Row } else { Dir::Column })
    }

    /// The catalog row (`catalog/conversion.toml [components.*]`) this node maps to.
    pub fn catalog_key(&self) -> Option<&'static str> {
        Some(match self.comp()? {
            Comp::Button => match self.variant() {
                "tonal" => "button_tonal",
                "outlined" => "button_outlined",
                "text" => "button_text",
                "elevated" => "button_elevated",
                _ => "button_filled",
            },
            Comp::IconButton => "icon_button",
            Comp::Fab => "fab",
            Comp::Switch => "switch",
            Comp::Checkbox => "checkbox",
            Comp::Radio => "radio",
            Comp::Slider => "slider",
            Comp::TextField => "text_field",
            Comp::Card => "card",
            Comp::Dialog => "dialog",
            Comp::List | Comp::ListItem => "list",
            Comp::Divider => "divider",
            Comp::Tabs | Comp::Tab => "tabs",
            Comp::TopAppBar => "top_app_bar",
            Comp::NavigationRail | Comp::NavigationRailItem => "navigation_rail",
            Comp::Chip => "chip",
            Comp::Progress => {
                if self.is_circular_progress() {
                    "progress_circular"
                } else {
                    "progress_linear"
                }
            }
        })
    }

    pub fn is_circular_progress(&self) -> bool {
        self.variant() == "circular" || self.str("type") == Some("circular")
    }

    /// Progress fraction in 0..=1, or `None` when indeterminate.
    pub fn progress_fraction(&self) -> Option<f64> {
        if self.flag("indeterminate") {
            return None;
        }
        let value = self.num("value")?;
        let max = self.num("max").unwrap_or(if value > 1.0 { 100.0 } else { 1.0 });
        Some((value / max).clamp(0.0, 1.0))
    }
}

#[derive(Clone, Debug)]
pub struct Document {
    /// Component function name, e.g. `SettingsPage`.
    pub component: String,
    /// Input file name for headers.
    pub source: String,
    pub root: Node,
    /// Distinct handler names in first-use order, as written in TSX.
    pub handlers: Vec<String>,
}

/// `onSave` -> `on_save`, `save` -> `on_save`, `openFile` -> `on_open_file`.
pub fn snake_handler(name: &str) -> String {
    let mut out = String::new();
    for (i, ch) in name.chars().enumerate() {
        if ch.is_ascii_uppercase() {
            if i > 0 {
                out.push('_');
            }
            out.push(ch.to_ascii_lowercase());
        } else if ch.is_ascii_alphanumeric() {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    if out.starts_with("on_") { out } else { format!("on_{out}") }
}

/// `onSave` -> `save`; names that collide with Qt Quick members get a `Requested` suffix.
pub fn qml_signal(name: &str) -> String {
    let base = match name.strip_prefix("on") {
        Some(rest) if rest.starts_with(|c: char| c.is_ascii_uppercase()) => {
            let mut c = rest.chars();
            let first = c.next().unwrap().to_ascii_lowercase();
            format!("{first}{}", c.as_str())
        }
        _ => name.to_string(),
    };
    const RESERVED: &[&str] = &[
        "close", "open", "accept", "reject", "clicked", "toggled", "pressed", "released", "accepted",
        "rejected", "closed", "opened", "moved", "focus", "visible", "enabled", "data", "state",
        "parent", "children", "width", "height", "x", "y", "z", "destroy", "update", "forceActiveFocus",
    ];
    if RESERVED.contains(&base.as_str()) { format!("{base}Requested") } else { base }
}

/// Lower-case identifier slug: `Wi-Fi network` -> `wi_fi_network`.
pub fn slug(s: &str) -> String {
    let mut out = String::new();
    let mut last_us = true;
    for ch in s.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            last_us = false;
        } else if !last_us {
            out.push('_');
            last_us = true;
        }
    }
    while out.ends_with('_') {
        out.pop();
    }
    if out.is_empty() || out.starts_with(|c: char| c.is_ascii_digit()) {
        out.insert_str(0, "item_");
    }
    out
}

/// M3 typescale: (pixel size, weight 400/500).
pub fn typescale_metrics(name: &str) -> Option<(f64, u16)> {
    Some(match name {
        "display-large" => (57.0, 400),
        "display-medium" => (45.0, 400),
        "display-small" => (36.0, 400),
        "headline-large" => (32.0, 400),
        "headline-medium" => (28.0, 400),
        "headline-small" => (24.0, 400),
        "title-large" => (22.0, 400),
        "title-medium" => (16.0, 500),
        "title-small" => (14.0, 500),
        "body-large" => (16.0, 400),
        "body-medium" => (14.0, 400),
        "body-small" => (12.0, 400),
        "label-large" => (14.0, 500),
        "label-medium" => (12.0, 500),
        "label-small" => (11.0, 500),
        _ => return None,
    })
}

/// Default typescale for text tags without a typescale class.
pub fn default_typescale(tag: TextTag) -> &'static str {
    match tag {
        TextTag::Heading(1) => "headline-large",
        TextTag::Heading(2) => "headline-medium",
        TextTag::Heading(3) => "headline-small",
        TextTag::Heading(4) => "title-large",
        TextTag::Heading(5) => "title-medium",
        TextTag::Heading(_) => "title-small",
        TextTag::Paragraph | TextTag::Span => "body-medium",
        TextTag::Label => "label-large",
    }
}

pub const COLOR_ROLES: &[&str] = &[
    "primary",
    "on-primary",
    "primary-container",
    "on-primary-container",
    "secondary",
    "on-secondary",
    "secondary-container",
    "on-secondary-container",
    "tertiary",
    "on-tertiary",
    "tertiary-container",
    "on-tertiary-container",
    "error",
    "on-error",
    "error-container",
    "on-error-container",
    "background",
    "on-background",
    "surface",
    "on-surface",
    "surface-variant",
    "on-surface-variant",
    "surface-dim",
    "surface-bright",
    "surface-container-lowest",
    "surface-container-low",
    "surface-container",
    "surface-container-high",
    "surface-container-highest",
    "surface-tint",
    "inverse-surface",
    "inverse-on-surface",
    "inverse-primary",
    "outline",
    "outline-variant",
    "shadow",
    "scrim",
];

/// `on-surface-variant` -> `onSurfaceVariant` (the `M3Theme` QML singleton property).
pub fn camel(role: &str) -> String {
    let mut out = String::new();
    let mut up = false;
    for ch in role.chars() {
        if ch == '-' {
            up = true;
        } else if up {
            out.push(ch.to_ascii_uppercase());
            up = false;
        } else {
            out.push(ch);
        }
    }
    out
}

/// M3 shape scale corner radius in pixels.
pub fn shape_radius(shape: &str) -> Option<f64> {
    Some(match shape {
        "none" => 0.0,
        "extra-small" => 4.0,
        "small" => 8.0,
        "medium" => 12.0,
        "large" => 16.0,
        "extra-large" => 28.0,
        "full" => 9999.0,
        _ => return None,
    })
}
