// SPDX-License-Identifier: Apache-2.0
//! TSX/JSX front end: oxc parses the module, this file lowers one stateless component into the
//! UI IR. Supports `@aphrody/m3-baseui`, `material-web`, React + Shadcn UI, Radix UI, Base UI,
//! and Tailwind CSS v3/v4 utility classes via `yolo-layout`.

use std::collections::HashMap;
use std::path::Path;

use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use oxc_parser::Parser;
use oxc_span::{GetSpan, SourceType, Span};
use yolo_layout::tailwind::resolve_tailwind;
use yolo_layout::{ContentKeyword, Display, FlexDirection, ItemKeyword, Length, Overflow, RepeatCount, TemplateItem};

use crate::css::{self, ClassMap};
use crate::diag::{Diag, LineIndex};
use crate::ir::{Align, Comp, Dir, Document, Event, Justify, Kind, Node, TextTag, Value};

#[derive(Clone, Debug)]
enum Const {
    Str(String),
    Num(f64),
    Bool(bool),
    Null,
    Array(Vec<Const>),
    Object(Vec<(String, Const)>),
}

fn num_str(n: f64) -> String {
    if n.fract() == 0.0 && n.abs() < 1e15 { format!("{}", n as i64) } else { format!("{n}") }
}

fn kebab(name: &str) -> String {
    let mut out = String::new();
    for ch in name.chars() {
        if ch.is_ascii_uppercase() {
            out.push('-');
            out.push(ch.to_ascii_lowercase());
        } else {
            out.push(ch);
        }
    }
    out
}

/// Unitless numeric CSS properties in React inline styles.
fn unitless(prop: &str) -> bool {
    matches!(prop, "flex" | "flex-grow" | "flex-shrink" | "opacity" | "z-index" | "line-height" | "font-weight")
}

struct Lower<'s> {
    file: String,
    lines: LineIndex<'s>,
    diags: Vec<Diag>,
    scopes: Vec<HashMap<String, Const>>,
    classes: ClassMap,
    handlers: Vec<String>,
}

type R<T> = Result<T, (Span, String)>;

/// Resolve the JSX element name into an IR kind plus implied props (Shadcn, Radix, Base UI, M3, HTML).
fn resolve_kind(raw_name: &str) -> Option<(Kind, Option<(&'static str, &'static str)>)> {
    use Comp::*;
    // Support Radix `<Primitive.div>`, `<DialogPrimitive.Content>`, `<Tabs.Trigger>`, etc.
    let normalized = if let Some((prefix, member)) = raw_name.split_once('.') {
        if prefix == "Primitive" {
            member
        } else {
            let base = prefix.strip_suffix("Primitive").unwrap_or(prefix);
            match (base, member) {
                ("Dialog" | "AlertDialog", "Root") => "Dialog",
                ("Dialog" | "AlertDialog", "Trigger") => "DialogTrigger",
                ("Dialog" | "AlertDialog", "Content") => "DialogContent",
                ("Dialog" | "AlertDialog", "Header") => "DialogHeader",
                ("Dialog" | "AlertDialog", "Title") => "DialogTitle",
                ("Dialog" | "AlertDialog", "Description") => "DialogDescription",
                ("Tabs", "Root") => "Tabs",
                ("Tabs", "List") => "TabsList",
                ("Tabs", "Trigger") => "TabsTrigger",
                ("Tabs", "Content") => "TabsContent",
                ("Select", "Root" | "Trigger" | "Content") => "Select",
                ("Select", "Value") => "SelectValue",
                ("Select", "Item") => "SelectItem",
                ("Avatar", "Root") => "Avatar",
                ("Avatar", "Image") => "AvatarImage",
                ("Avatar", "Fallback") => "AvatarFallback",
                ("Tooltip", _) => "Tooltip",
                ("Sheet", "Root") => "Sheet",
                ("Sheet", "Content" | "Trigger" | "Header" | "Footer") => "DialogContent",
                ("ScrollArea", _) => "ScrollArea",
                ("Switch", _) => "Switch",
                ("Checkbox", _) => "Checkbox",
                ("Slider", _) => "Slider",
                ("Progress", _) => "Progress",
                ("Separator", _) => "Separator",
                ("Label", _) => "Label",
                ("NumberField", "Input") => "Input",
                ("NumberField", "Increment" | "Decrement") => "Button",
                ("NumberField", _) => "div",
                ("Field", "Label") => "Label",
                ("Field", "Description" | "Error") => "CardDescription",
                ("Field", "Control") => "Input",
                ("Field", _) => "div",
                ("Popover" | "HoverCard" | "PreviewCard", _) => "div",
                ("Accordion" | "Collapsible", "Trigger") => "Button",
                ("Accordion" | "Collapsible", _) => "div",
                ("Toggle" | "ToggleGroup", _) => "Button",
                ("Drawer", "Root") => "Dialog",
                ("Drawer", "Title") => "DialogTitle",
                ("Drawer", "Description") => "DialogDescription",
                ("Drawer", _) => "DialogContent",
                _ => member,
            }
        }
    } else {
        raw_name
    };

    let name = normalized
        .strip_prefix("M3")
        .filter(|n| n.starts_with(|c: char| c.is_ascii_uppercase()))
        .unwrap_or(normalized);
    let k = |c| Kind::Comp(c);
    Some(match name {
        // Core & Shadcn / Radix / Base UI interactive controls
        "Button" | "button" | "Toggle" | "ToggleGroupItem" | "AccordionTrigger" | "CollapsibleTrigger" => {
            (k(Button), None)
        }
        "a" => (k(Button), Some(("variant", "text"))),
        "IconButton" => (k(IconButton), None),
        "Fab" | "FAB" => (k(Fab), None),
        "Switch" => (k(Switch), None),
        "Checkbox" => (k(Checkbox), None),
        "Radio" | "RadioGroupItem" => (k(Radio), None),
        "RadioGroup" | "ToggleGroup" | "Accordion" | "AccordionItem" | "AccordionContent"
        | "Collapsible" | "CollapsibleContent" | "Popover" | "PopoverTrigger" | "PopoverContent"
        | "HoverCard" | "HoverCardTrigger" | "HoverCardContent" | "PreviewCard" | "NumberField"
        | "Field" => (Kind::Box, None),
        "Slider" => (k(Slider), None),
        "TextField" | "Input" | "Textarea" | "input" | "textarea" => (k(TextField), None),

        // Shadcn / M3 Card family
        "Card" | "Alert" => (k(Card), None),
        "CardHeader" | "CardContent" | "DialogHeader" | "DialogContent" | "DialogTrigger"
        | "SheetHeader" | "SheetContent" | "SheetTrigger" | "DrawerHeader" | "DrawerContent"
        | "DrawerTrigger" => (Kind::Box, None),
        "CardFooter" | "DialogFooter" | "SheetFooter" | "DrawerFooter" => {
            (Kind::Box, Some(("_row", "true")))
        }
        "CardTitle" | "AlertTitle" => (Kind::Text(TextTag::Heading(3)), None),
        "CardDescription" | "AlertDescription" | "DialogDescription" | "SheetDescription"
        | "DrawerDescription" => (Kind::Text(TextTag::Paragraph), Some(("_muted", "true"))),

        // Dialog / Sheet / Drawer
        "Dialog" | "AlertDialog" | "Sheet" | "Drawer" => (k(Dialog), None),
        "DialogTitle" | "SheetTitle" | "DrawerTitle" => (Kind::Text(TextTag::Heading(2)), None),

        // Lists, Menus, Select
        "List" | "ul" | "ol" => (k(List), None),
        "ListItem" | "SelectItem" => (k(ListItem), None),
        "li" => (Kind::Box, None),
        "Select" | "SelectTrigger" | "SelectContent" => (Kind::Box, None),
        "SelectValue" => (Kind::Text(TextTag::Span), None),

        // Dividers / Separators
        "Divider" | "Separator" => (k(Divider), None),

        // Tabs (M3 + Shadcn/Radix)
        "Tabs" => (k(Tabs), None),
        "Tab" | "TabsTrigger" => (k(Tab), None),
        "TabsList" => (Kind::Box, Some(("_tabs_role", "list"))),
        "TabsContent" => (Kind::Box, Some(("_tabs_role", "content"))),

        // Navigation & App Bar
        "TopAppBar" => (k(TopAppBar), None),
        "NavigationRail" => (k(NavigationRail), None),
        "NavigationRailItem" | "NavigationItem" => (k(NavigationRailItem), None),

        // Badge / Chip / Avatar / Tooltip / ScrollArea / Progress
        "Chip" | "Badge" => (k(Chip), None),
        "Avatar" => (Kind::Box, Some(("_avatar", "true"))),
        "AvatarImage" => (Kind::Image, None),
        "AvatarFallback" => (Kind::Text(TextTag::Span), None),
        "Tooltip" | "TooltipProvider" | "TooltipTrigger" | "TooltipContent" => (Kind::Box, None),
        "ScrollArea" => (Kind::Box, Some(("_scroll", "true"))),
        "Progress" | "LinearProgress" => (k(Progress), None),
        "CircularProgress" => (k(Progress), Some(("variant", "circular"))),

        // material-web custom elements
        "md-filled-button" => (k(Button), Some(("variant", "filled"))),
        "md-filled-tonal-button" => (k(Button), Some(("variant", "tonal"))),
        "md-outlined-button" => (k(Button), Some(("variant", "outlined"))),
        "md-text-button" => (k(Button), Some(("variant", "text"))),
        "md-elevated-button" => (k(Button), Some(("variant", "elevated"))),
        "md-icon-button" => (k(IconButton), None),
        "md-filled-icon-button" => (k(IconButton), Some(("variant", "filled"))),
        "md-filled-tonal-icon-button" => (k(IconButton), Some(("variant", "tonal"))),
        "md-outlined-icon-button" => (k(IconButton), Some(("variant", "outlined"))),
        "md-fab" | "md-branded-fab" => (k(Fab), None),
        "md-switch" => (k(Switch), None),
        "md-checkbox" => (k(Checkbox), None),
        "md-radio" => (k(Radio), None),
        "md-slider" => (k(Slider), None),
        "md-filled-text-field" => (k(TextField), Some(("variant", "filled"))),
        "md-outlined-text-field" => (k(TextField), Some(("variant", "outlined"))),
        "md-dialog" => (k(Dialog), None),
        "md-list" => (k(List), None),
        "md-list-item" => (k(ListItem), None),
        "md-divider" => (k(Divider), None),
        "md-tabs" => (k(Tabs), None),
        "md-primary-tab" | "md-secondary-tab" => (k(Tab), None),
        "md-assist-chip" => (k(Chip), Some(("variant", "assist"))),
        "md-filter-chip" => (k(Chip), Some(("variant", "filter"))),
        "md-input-chip" => (k(Chip), Some(("variant", "input"))),
        "md-suggestion-chip" => (k(Chip), Some(("variant", "suggestion"))),
        "md-linear-progress" => (k(Progress), None),
        "md-circular-progress" => (k(Progress), Some(("variant", "circular"))),

        // HTML layout & typography
        "div" | "section" | "main" | "header" | "footer" | "nav" | "form" | "article" | "aside"
        | "slot" | "Slot" => (Kind::Box, None),
        "h1" => (Kind::Text(TextTag::Heading(1)), None),
        "h2" => (Kind::Text(TextTag::Heading(2)), None),
        "h3" => (Kind::Text(TextTag::Heading(3)), None),
        "h4" => (Kind::Text(TextTag::Heading(4)), None),
        "h5" => (Kind::Text(TextTag::Heading(5)), None),
        "h6" => (Kind::Text(TextTag::Heading(6)), None),
        "p" => (Kind::Text(TextTag::Paragraph), None),
        "span" => (Kind::Text(TextTag::Span), None),
        "label" | "Label" => (Kind::Text(TextTag::Label), None),
        "img" => (Kind::Image, None),
        _ => return None,
    })
}

/// Props accepted per kind (besides id, key, className, style, slot, aria-label, disabled).
fn allowed_props(kind: Kind) -> &'static [&'static str] {
    use Comp::*;
    match kind {
        Kind::Comp(Button) => &["variant", "icon", "type", "size", "asChild", "href", "target"],
        Kind::Comp(IconButton) => &["icon", "variant", "selected", "toggle", "size"],
        Kind::Comp(Fab) => &["icon", "label", "variant", "size"],
        Kind::Comp(Switch) => &["checked", "defaultChecked", "selected", "label", "name", "icons"],
        Kind::Comp(Checkbox) | Kind::Comp(Radio) => &["checked", "defaultChecked", "label", "name", "value"],
        Kind::Comp(Slider) => &["min", "max", "value", "defaultValue", "step", "label", "name", "labeled"],
        Kind::Comp(TextField) => &[
            "label",
            "placeholder",
            "value",
            "defaultValue",
            "type",
            "variant",
            "supportingText",
            "name",
            "required",
            "autoComplete",
            "rows",
            "readOnly",
        ],
        Kind::Comp(Card) => &["variant"],
        Kind::Comp(Dialog) => &["title", "headline", "open", "defaultOpen", "modal"],
        Kind::Comp(List) => &[],
        Kind::Comp(ListItem) => &["headline", "supportingText", "icon", "leadingIcon", "trailingText", "type", "value"],
        Kind::Comp(Divider) => &["inset", "orientation", "decorative"],
        Kind::Comp(Tabs) => &["activeIndex", "variant", "defaultValue", "value", "orientation"],
        Kind::Comp(Tab) => &["label", "icon", "value"],
        Kind::Comp(TopAppBar) => &["title", "headline", "subtitle", "variant"],
        Kind::Comp(NavigationRail) => &["activeIndex"],
        Kind::Comp(NavigationRailItem) => &["icon", "label", "selected", "active"],
        Kind::Comp(Chip) => &["variant", "label", "selected", "icon", "elevated"],
        Kind::Comp(Progress) => &["value", "max", "variant", "type", "indeterminate"],
        Kind::Image => &["src", "alt", "width", "height"],
        Kind::Box | Kind::Text(_) => &["htmlFor", "for", "value", "defaultValue", "placeholder", "asChild", "orientation"],
    }
}

/// Events accepted per kind.
fn allowed_events(kind: Kind) -> &'static [&'static str] {
    use Comp::*;
    match kind {
        Kind::Comp(Button | IconButton | Fab | Chip | NavigationRailItem | ListItem) => &["onClick"],
        Kind::Comp(Switch | Checkbox | Radio) => &["onChange", "onCheckedChange", "onClick"],
        Kind::Comp(Slider) => &["onChange", "onInput", "onValueChange"],
        Kind::Comp(TextField) => &["onChange", "onInput", "onSubmit", "onEnter", "onValueChange"],
        Kind::Comp(Dialog) => &["onClose", "onOpenChange"],
        Kind::Comp(Tabs | NavigationRail) => &["onChange", "onValueChange"],
        Kind::Box => &["onClick", "onSubmit", "onValueChange", "onOpenChange"],
        _ => &[],
    }
}

enum Child {
    Text(String, Span),
    Node(Node),
}

impl<'s> Lower<'s> {
    fn err(&mut self, span: Span, msg: impl Into<String>) {
        let (line, col) = self.lines.pos(span.start);
        self.diags.push(Diag { file: self.file.clone(), line, col, msg: msg.into() });
    }

    fn lookup(&self, name: &str) -> Option<&Const> {
        self.scopes.iter().rev().find_map(|s| s.get(name))
    }

    fn eval(&self, e: &Expression<'_>) -> R<Const> {
        match e {
            Expression::StringLiteral(s) => s
                .value
                .as_str()
                .map(|v| Const::Str(v.to_string()))
                .ok_or((s.span, "string with lone surrogate".into())),
            Expression::NumericLiteral(n) => Ok(Const::Num(n.value)),
            Expression::BooleanLiteral(b) => Ok(Const::Bool(b.value)),
            Expression::NullLiteral(_) => Ok(Const::Null),
            Expression::TemplateLiteral(t) => {
                let mut out = String::new();
                for (i, q) in t.quasis.iter().enumerate() {
                    out.push_str(q.value.cooked.and_then(|c| c.as_str()).ok_or((q.span, "invalid template text".to_string()))?);
                    if let Some(ex) = t.expressions.get(i) {
                        match self.eval(ex)? {
                            Const::Str(s) => out.push_str(&s),
                            Const::Num(n) => out.push_str(&num_str(n)),
                            _ => return Err((ex.span(), "template expression must be a string or number constant".into())),
                        }
                    }
                }
                Ok(Const::Str(out))
            }
            Expression::UnaryExpression(u) if u.operator == UnaryOperator::UnaryNegation => match self.eval(&u.argument)? {
                Const::Num(n) => Ok(Const::Num(-n)),
                _ => Err((u.span, "unary minus needs a number".into())),
            },
            Expression::Identifier(id) => self
                .lookup(id.name.as_str())
                .cloned()
                .ok_or((id.span, format!("`{}` is not a compile-time constant (only literals, module constants and .map() items are converted)", id.name))),
            Expression::StaticMemberExpression(m) => match self.eval(&m.object)? {
                Const::Object(fields) => fields
                    .iter()
                    .find(|(k, _)| k == m.property.name.as_str())
                    .map(|(_, v)| v.clone())
                    .ok_or((m.property.span, format!("constant object has no field `{}`", m.property.name))),
                _ => Err((m.span, "member access on a non-object constant".into())),
            },
            Expression::ArrayExpression(a) => {
                let mut items = Vec::new();
                for el in &a.elements {
                    match el.as_expression() {
                        Some(x) => items.push(self.eval(x)?),
                        None => return Err((el.span(), "spread and holes are not supported in constant arrays".into())),
                    }
                }
                Ok(Const::Array(items))
            }
            Expression::ObjectExpression(o) => {
                let mut fields = Vec::new();
                for p in &o.properties {
                    let ObjectPropertyKind::ObjectProperty(p) = p else {
                        return Err((p.span(), "object spread is not supported".into()));
                    };
                    if p.computed || p.method {
                        return Err((p.span, "computed keys and methods are not supported in constant objects".into()));
                    }
                    let key = match &p.key {
                        PropertyKey::StaticIdentifier(id) => id.name.to_string(),
                        k => match k.as_expression() {
                            Some(Expression::StringLiteral(s)) => s.value.as_str().unwrap_or_default().to_string(),
                            _ => return Err((p.key.span(), "unsupported object key".into())),
                        },
                    };
                    fields.push((key, self.eval(&p.value)?));
                }
                Ok(Const::Object(fields))
            }
            Expression::ParenthesizedExpression(p) => self.eval(&p.expression),
            Expression::TSAsExpression(a) => self.eval(&a.expression),
            Expression::TSSatisfiesExpression(a) => self.eval(&a.expression),
            Expression::ConditionalExpression(c) => Err((c.span, "conditional expressions are not converted; generate one variant per state".into())),
            Expression::LogicalExpression(l) => Err((l.span, "logical expressions (`&&`, `||`, `??`) are not converted".into())),
            Expression::CallExpression(c) => Err((c.span, "function calls are not compile-time constants".into())),
            other => Err((other.span(), "unsupported expression: only literals and constants are converted".into())),
        }
    }

    /// Handler reference: `onSave`, `props.onSave`, `() => onSave()`.
    fn handler(&self, e: &Expression<'_>) -> R<String> {
        match e {
            Expression::Identifier(id) => Ok(id.name.to_string()),
            Expression::StaticMemberExpression(m) => Ok(m.property.name.to_string()),
            Expression::ParenthesizedExpression(p) => self.handler(&p.expression),
            Expression::ArrowFunctionExpression(a) => {
                let body = match &a.body {
                    ArrowFunctionBody::FunctionBody(b) => match b.statements.as_slice() {
                        [Statement::ExpressionStatement(s)] => Some(&s.expression),
                        _ => None,
                    },
                    b => b.as_expression(),
                };
                match body {
                    Some(Expression::CallExpression(c)) if c.arguments.is_empty() => match &c.callee {
                        Expression::Identifier(id) => Ok(id.name.to_string()),
                        Expression::StaticMemberExpression(m) => Ok(m.property.name.to_string()),
                        _ => Err((c.callee.span(), "handler callee must be a named function".into())),
                    },
                    Some(Expression::CallExpression(c)) => Err((c.span, "handler calls with arguments are not converted; pass a named handler (onClick={onSave})".into())),
                    _ => Err((a.span, "inline handler logic is not converted; pass a named handler (onClick={onSave})".into())),
                }
            }
            other => Err((other.span(), "event handler must be a named function reference (onClick={onSave})".into())),
        }
    }

    fn inline_style(&self, e: &Expression<'_>) -> R<String> {
        let Expression::ObjectExpression(o) = e else {
            return Err((e.span(), "style must be an object literal (style={{ gap: 12 }})".into()));
        };
        let mut css = String::new();
        for p in &o.properties {
            let ObjectPropertyKind::ObjectProperty(p) = p else {
                return Err((p.span(), "spread in style objects is not supported".into()));
            };
            let key = match &p.key {
                PropertyKey::StaticIdentifier(id) => kebab(id.name.as_str()),
                k => match k.as_expression() {
                    Some(Expression::StringLiteral(s)) => s.value.as_str().unwrap_or_default().to_string(),
                    _ => return Err((p.key.span(), "unsupported style key".into())),
                },
            };
            let value = match self.eval(&p.value)? {
                Const::Str(s) => s,
                Const::Num(n) if unitless(&key) => num_str(n),
                Const::Num(n) => format!("{}px", num_str(n)),
                _ => return Err((p.value.span(), "style values must be strings or numbers".into())),
            };
            css.push_str(&format!("{key}: {value}; "));
        }
        Ok(css)
    }

    fn apply_class(&mut self, node: &mut Node, class: &str, span: Span) {
        node.classes.push(class.to_owned());
        for prefix in ["md-typescale-", "m3-typescale-", "typescale-"] {
            if let Some(ts) = class.strip_prefix(prefix) {
                if crate::ir::typescale_metrics(ts).is_some() {
                    node.style.typescale = Some(ts.to_string());
                } else {
                    self.err(span, format!("unknown M3 typescale `{ts}`"));
                }
                return;
            }
        }
        if let Some(rules) = self.classes.get(class).cloned() {
            for rule in rules {
                for msg in css::apply(&mut node.style, &rule.decls) {
                    self.diags.push(Diag {
                        file: rule.file.clone(),
                        line: rule.line,
                        col: rule.col,
                        msg: format!("in `.{class}`: {msg}"),
                    });
                }
            }
            return;
        }
        // Try Tailwind CSS v3/v4 utility translation via `yolo-layout`
        let tw = resolve_tailwind(class);
        if tw.unknown_classes.is_empty() {
            apply_tailwind_to_ir_style(&mut node.style, &tw);
            return;
        }
        self.err(
            span,
            format!("class `{class}` has no rule in the imported CSS and is not a recognized M3 typescale or Tailwind utility"),
        );
    }

    fn element(&mut self, el: &JSXElement<'_>) -> Option<Node> {
        let open = &el.opening_element;
        let name = match &open.name {
            JSXElementName::Identifier(id) => id.name.as_str().to_string(),
            JSXElementName::IdentifierReference(id) => id.name.as_str().to_string(),
            JSXElementName::MemberExpression(m) => {
                if let JSXMemberExpressionObject::IdentifierReference(obj) = &m.object {
                    format!("{}.{}", obj.name, m.property.name)
                } else {
                    self.err(open.name.span(), "nested member element names are not supported");
                    return None;
                }
            }
            _ => {
                self.err(open.name.span(), "namespaced and `this` element names are not supported");
                return None;
            }
        };
        let Some((kind, preset)) = resolve_kind(&name) else {
            self.err(
                open.span,
                format!("<{name}> is not a supported Shadcn/Radix/BaseUI/M3 component or HTML layout element"),
            );
            return None;
        };
        let (line, col) = self.lines.pos(open.span.start);
        let mut node = Node::new(kind, line, col);
        node.tag_name = name.clone();
        if let Some((k, v)) = preset {
            match (k, v) {
                ("_row", "true") => {
                    node.style.display_flex = true;
                    node.style.direction = Some(Dir::Row);
                }
                ("_muted", "true") => {
                    node.style.color = Some("on-surface-variant".into());
                    node.style.typescale = Some("body-small".into());
                }
                ("_scroll", "true") => {
                    node.style.scroll = true;
                }
                ("_avatar", "true") => {
                    node.style.shape = Some("full".into());
                }
                _ => {
                    node.props.insert(k.into(), Value::Str(v.into()));
                }
            }
        }
        let mut inline_css: Option<(String, Span)> = None;

        for attr in &open.attributes {
            let attr = match attr {
                JSXAttributeItem::Attribute(a) => a,
                JSXAttributeItem::SpreadAttribute(s) => {
                    self.err(s.span, "spread attributes ({...props}) are not supported");
                    continue;
                }
            };
            let JSXAttributeName::Identifier(id) = &attr.name else {
                self.err(attr.span, "namespaced attributes are not supported");
                continue;
            };
            let key = id.name.as_str();
            let expr = match &attr.value {
                Some(JSXAttributeValue::ExpressionContainer(c)) => match c.expression.as_expression() {
                    Some(e) => Some(e),
                    None => {
                        self.err(c.span, "empty expression");
                        continue;
                    }
                },
                Some(JSXAttributeValue::Element(_) | JSXAttributeValue::Fragment(_)) => {
                    self.err(attr.span, "JSX as an attribute value is not supported");
                    continue;
                }
                _ => None,
            };
            let lit = match &attr.value {
                None => Some(Value::Bool(true)),
                Some(JSXAttributeValue::StringLiteral(s)) => Some(Value::Str(s.value.as_str().unwrap_or_default().to_string())),
                _ => None,
            };

            if key.len() > 2 && key.starts_with("on") && key[2..].starts_with(|c: char| c.is_ascii_uppercase()) {
                if !allowed_events(kind).contains(&key) {
                    self.err(attr.span, format!("event `{key}` is not supported on <{name}>"));
                    continue;
                }
                let Some(e) = expr else {
                    self.err(attr.span, "event handlers must be expressions (onClick={onSave})");
                    continue;
                };
                match self.handler(e) {
                    Ok(h) => {
                        if !self.handlers.contains(&h) {
                            self.handlers.push(h.clone());
                        }
                        node.events.push(Event { prop: key.into(), handler: h });
                    }
                    Err((sp, m)) => self.err(sp, m),
                }
                continue;
            }
            if key == "style" {
                match expr.map(|e| self.inline_style(e)) {
                    Some(Ok(css)) => inline_css = Some((css, attr.span)),
                    Some(Err((sp, m))) => self.err(sp, m),
                    None => self.err(attr.span, "style must be an object literal (style={{ gap: 12 }})"),
                }
                continue;
            }
            // Handle Slider defaultValue={[50]} / value={[50]} array literal in Shadcn/Radix
            let value = match (lit, expr) {
                (Some(v), _) => v,
                (None, Some(e)) => match self.eval(e) {
                    Ok(Const::Str(s)) => Value::Str(s),
                    Ok(Const::Num(n)) => Value::Num(n),
                    Ok(Const::Bool(b)) => Value::Bool(b),
                    Ok(Const::Array(items)) if matches!(kind, Kind::Comp(Comp::Slider)) && items.len() == 1 => {
                        match &items[0] {
                            Const::Num(n) => Value::Num(*n),
                            _ => {
                                self.err(e.span(), format!("prop `{key}` array element must be a number"));
                                continue;
                            }
                        }
                    }
                    Ok(_) => {
                        self.err(e.span(), format!("prop `{key}` must be a string, number or boolean constant"));
                        continue;
                    }
                    Err((sp, m)) => {
                        self.err(sp, format!("prop `{key}`: {m}"));
                        continue;
                    }
                },
                (None, None) => continue,
            };
            let as_str = |v: &Value| match v {
                Value::Str(s) => s.clone(),
                Value::Num(n) => num_str(*n),
                Value::Bool(b) => b.to_string(),
            };
            match key {
                "key" => {}
                "className" | "class" => {
                    let s = as_str(&value);
                    for c in s.split_whitespace() {
                        self.apply_class(&mut node, c, attr.span);
                    }
                }
                "id" => node.id = Some(as_str(&value)),
                "slot" => node.slot = Some(as_str(&value)),
                "aria-label" | "ariaLabel" | "title" if !matches!(kind, Kind::Comp(Comp::Dialog | Comp::TopAppBar)) => {
                    node.aria_label = Some(as_str(&value))
                }
                "disabled" => {
                    node.props.insert("disabled".into(), value);
                }
                "defaultValue" if allowed_props(kind).contains(&"defaultValue") => {
                    node.props.entry("value".into()).or_insert_with(|| value.clone());
                    node.props.insert("defaultValue".into(), value);
                }
                "defaultChecked" if allowed_props(kind).contains(&"defaultChecked") => {
                    node.props.entry("checked".into()).or_insert_with(|| value.clone());
                    node.props.insert("defaultChecked".into(), value);
                }
                k if allowed_props(kind).contains(&k) => {
                    node.props.insert(k.into(), value);
                }
                k => self.err(attr.span, format!("prop `{k}` is not supported on <{name}>")),
            }
        }
        if let Some((css_text, span)) = inline_css {
            match css::parse_inline(&css_text) {
                Ok(decls) => {
                    for m in css::apply(&mut node.style, &decls) {
                        self.err(span, format!("inline style: {m}"));
                    }
                }
                Err(m) => self.err(span, format!("inline style: {m}")),
            }
        }

        let children = self.children(&el.children);
        self.attach(&mut node, &name, children);
        Some(node)
    }

    fn children(&mut self, list: &[JSXChild<'_>]) -> Vec<Child> {
        let mut out = Vec::new();
        for c in list {
            match c {
                JSXChild::Text(t) => {
                    let text = t.value.as_str().split_whitespace().collect::<Vec<_>>().join(" ");
                    if !text.is_empty() {
                        out.push(Child::Text(text, t.span));
                    }
                }
                JSXChild::Element(e) => {
                    if let Some(n) = self.element(e) {
                        out.push(Child::Node(n));
                    }
                }
                JSXChild::Fragment(f) => out.extend(self.children(&f.children)),
                JSXChild::Spread(s) => self.err(s.span, "spread children are not supported"),
                JSXChild::ExpressionContainer(c) => {
                    let Some(e) = c.expression.as_expression() else { continue };
                    if let Some(items) = self.try_map(e) {
                        out.extend(items);
                        continue;
                    }
                    match self.eval(e) {
                        Ok(Const::Str(s)) => out.push(Child::Text(s, c.span)),
                        Ok(Const::Num(n)) => out.push(Child::Text(num_str(n), c.span)),
                        Ok(_) => self.err(c.span, "only string and number constants can be rendered as text"),
                        Err((sp, m)) => self.err(sp, m),
                    }
                }
            }
        }
        out
    }

    /// Statically unrolls `CONST_ARRAY.map((item) => <Jsx/>)`. Returns None if `e` is not a
    /// `.map` call; reports and returns Some(empty) if it is one but outside the subset.
    fn try_map(&mut self, e: &Expression<'_>) -> Option<Vec<Child>> {
        let Expression::CallExpression(call) = e else { return None };
        let Expression::StaticMemberExpression(m) = &call.callee else { return None };
        if m.property.name.as_str() != "map" {
            return None;
        }
        let fail = |this: &mut Self, sp: Span, msg: &str| {
            this.err(sp, msg.to_string());
            Some(Vec::new())
        };
        let items = match self.eval(&m.object) {
            Ok(Const::Array(items)) => items,
            Ok(_) => return fail(self, m.object.span(), ".map() source must be a constant array"),
            Err((sp, msg)) => return fail(self, sp, &format!(".map() source: {msg}")),
        };
        let [arg] = call.arguments.as_slice() else {
            return fail(self, call.span, ".map() takes exactly one arrow function here");
        };
        let Some(Expression::ArrowFunctionExpression(f)) = arg.as_expression() else {
            return fail(self, arg.span(), ".map() callback must be an arrow function");
        };
        let mut params = Vec::new();
        for p in &f.params.items {
            match &p.pattern {
                BindingPattern::BindingIdentifier(b) => params.push(b.name.to_string()),
                _ => return fail(self, p.span, "destructuring in .map() parameters is not supported"),
            }
        }
        if params.is_empty() || params.len() > 2 || f.params.rest.is_some() {
            return fail(self, f.params.span, ".map() callback takes (item) or (item, index)");
        }
        let body = match &f.body {
            ArrowFunctionBody::FunctionBody(b) => match b.statements.as_slice() {
                [Statement::ReturnStatement(r)] => r.argument.as_ref(),
                _ => return fail(self, b.span, ".map() callback body must be a single JSX expression (complex logic is not converted)"),
            },
            b => b.as_expression(),
        };
        let mut jsx = body;
        while let Some(Expression::ParenthesizedExpression(p)) = jsx {
            jsx = Some(&p.expression);
        }
        let Some(Expression::JSXElement(el)) = jsx else {
            return fail(self, f.span, ".map() callback must return one JSX element");
        };
        let mut out = Vec::new();
        for (i, item) in items.into_iter().enumerate() {
            let mut scope = HashMap::new();
            scope.insert(params[0].clone(), item);
            if let Some(ix) = params.get(1) {
                scope.insert(ix.clone(), Const::Num(i as f64));
            }
            self.scopes.push(scope);
            if let Some(n) = self.element(el) {
                out.push(Child::Node(n));
            }
            self.scopes.pop();
        }
        Some(out)
    }

    /// Distributes children by kind: label text, content nodes, or errors.
    fn attach(&mut self, node: &mut Node, name: &str, children: Vec<Child>) {
        use Comp::*;
        let text_label = matches!(
            node.kind,
            Kind::Comp(Button | Chip | Tab | ListItem | NavigationRailItem | Checkbox | Radio | Switch | Fab)
        ) || matches!(node.kind, Kind::Text(_));
        let takes_nodes = matches!(
            node.kind,
            Kind::Box | Kind::Comp(Card | Dialog | List | ListItem | Tabs | Tab | TopAppBar | NavigationRail)
        );
        let mut texts = Vec::new();
        let mut raw_nodes = Vec::new();
        for c in children {
            match c {
                Child::Text(t, sp) => {
                    if text_label {
                        texts.push(t);
                    } else if takes_nodes && !matches!(node.kind, Kind::Comp(List | Tabs | NavigationRail | TopAppBar)) {
                        let (line, col) = self.lines.pos(sp.start);
                        let mut n = Node::new(Kind::Text(TextTag::Span), line, col);
                        n.text = Some(t);
                        raw_nodes.push(n);
                    } else {
                        self.err(sp, format!("text is not allowed directly inside <{name}>"));
                    }
                }
                Child::Node(mut n) => {
                    // Convert `<li>` inside `<List>` / `<ul>` into `Comp::ListItem` automatically
                    if node.is(List) && n.kind == Kind::Box && n.tag_name == "li" {
                        n.kind = Kind::Comp(ListItem);
                    }
                    // If a `<Button>` or `<Text>` wraps a `<span>` text node, extract its text
                    if text_label && !takes_nodes && matches!(n.kind, Kind::Text(_)) {
                        if let Some(t) = n.text.take() {
                            texts.push(t);
                            continue;
                        }
                    }
                    raw_nodes.push(n);
                }
            }
        }

        // Normalize Shadcn/Radix `<Tabs>` (`<TabsList>` + `<TabsTrigger>` + `<TabsContent>`) into `Comp::Tab` children
        if node.is(Tabs)
            && raw_nodes
                .iter()
                .any(|n| n.str("_tabs_role").is_some())
        {
            let mut triggers = Vec::new();
            let mut contents = Vec::new();
            for n in raw_nodes {
                match n.str("_tabs_role") {
                    Some("list") => {
                        for item in n.children {
                            if item.is(Tab) {
                                triggers.push(item);
                            }
                        }
                    }
                    Some("content") => contents.push(n),
                    _ if n.is(Tab) => triggers.push(n),
                    _ => {}
                }
            }
            let mut normalized = Vec::new();
            for (idx, mut trig) in triggers.into_iter().enumerate() {
                if trig.str("label").is_none()
                    && let Some(t) = trig.text.clone()
                {
                    trig.props.insert("label".into(), Value::Str(t));
                }
                let trig_val = trig.str("value").map(str::to_owned);
                let matched_content = contents
                    .iter()
                    .position(|c| trig_val.is_some() && c.str("value").map(str::to_owned) == trig_val)
                    .or(if idx < contents.len() { Some(idx) } else { None });
                if let Some(ci) = matched_content {
                    trig.children.extend(contents[ci].children.clone());
                }
                normalized.push(trig);
            }
            raw_nodes = normalized;
        }

        for n in raw_nodes {
            let ok = takes_nodes
                && match node.kind {
                    Kind::Comp(Tabs) => n.is(Tab),
                    Kind::Comp(NavigationRail) => n.is(NavigationRailItem),
                    Kind::Comp(List) => matches!(n.kind, Kind::Comp(ListItem | Divider | TextField | Switch)),
                    Kind::Comp(TopAppBar) => matches!(n.kind, Kind::Comp(IconButton | Button)),
                    _ => true,
                };
            if ok {
                node.children.push(n);
            } else {
                let (l, c) = (n.line, n.col);
                self.diags.push(Diag {
                    file: self.file.clone(),
                    line: l,
                    col: c,
                    msg: format!("{:?} is not allowed inside <{name}>", n.kind),
                });
            }
        }

        if !texts.is_empty() {
            node.text = Some(texts.join(" "));
        } else if node.tag_name == "SelectValue"
            && let Some(p) = node.str("placeholder").map(str::to_owned)
        {
            node.text = Some(p);
        }
        if let Kind::Text(tag) = node.kind
            && node.style.typescale.is_none()
        {
            node.style.typescale = Some(crate::ir::default_typescale(tag).to_string());
        }
        if node.is(IconButton) && node.str("icon").is_none() {
            self.diags.push(Diag {
                file: self.file.clone(),
                line: node.line,
                col: node.col,
                msg: "<IconButton> needs an `icon` prop".into(),
            });
        }
    }
}

fn apply_tailwind_to_ir_style(style: &mut crate::ir::Style, tw: &yolo_layout::tailwind::TailwindResolution) {
    let l = &tw.layout;
    match l.display {
        Display::Flex => style.display_flex = true,
        Display::Grid => {
            style.display_grid = true;
            if let Some(TemplateItem::Repeat(RepeatCount::Count(n), _)) = l.grid_template_columns.first() {
                style.grid_cols = Some(*n);
            }
        }
        _ => {}
    }
    match l.flex_direction {
        FlexDirection::Column | FlexDirection::ColumnReverse if l.display == Display::Flex => {
            style.direction = Some(Dir::Column);
        }
        FlexDirection::Row | FlexDirection::RowReverse if l.display == Display::Flex => {
            style.direction = Some(Dir::Row);
        }
        _ => {}
    }
    if let Length::Px(g) = l.row_gap
        && g > 0.0
    {
        style.gap = Some(f64::from(g));
    } else if let Length::Px(g) = l.column_gap
        && g > 0.0
    {
        style.gap = Some(f64::from(g));
    }
    let pad = [l.padding.top, l.padding.right, l.padding.bottom, l.padding.left];
    if pad.iter().any(|v| matches!(v, Length::Px(px) if *px != 0.0)) {
        let cur = style.padding.get_or_insert([0.0; 4]);
        for (i, side) in pad.iter().enumerate() {
            if let Length::Px(px) = side
                && *px != 0.0
            {
                cur[i] = f64::from(*px);
            }
        }
    }
    let mar = [l.margin.top, l.margin.right, l.margin.bottom, l.margin.left];
    if mar.iter().any(|v| matches!(v, Length::Px(px) if *px != 0.0)) {
        let cur = style.margin.get_or_insert([0.0; 4]);
        for (i, side) in mar.iter().enumerate() {
            if let Length::Px(px) = side
                && *px != 0.0
            {
                cur[i] = f64::from(*px);
            }
        }
    }
    match l.align_items.keyword {
        ItemKeyword::Start | ItemKeyword::FlexStart | ItemKeyword::SelfStart => style.align = Some(Align::Start),
        ItemKeyword::Center => style.align = Some(Align::Center),
        ItemKeyword::End | ItemKeyword::FlexEnd | ItemKeyword::SelfEnd => style.align = Some(Align::End),
        ItemKeyword::Stretch => style.align = Some(Align::Stretch),
        _ => {}
    }
    match l.justify_content.keyword {
        ContentKeyword::Start | ContentKeyword::FlexStart => style.justify = Some(Justify::Start),
        ContentKeyword::Center => style.justify = Some(Justify::Center),
        ContentKeyword::End | ContentKeyword::FlexEnd => style.justify = Some(Justify::End),
        ContentKeyword::SpaceBetween | ContentKeyword::SpaceAround | ContentKeyword::SpaceEvenly => {
            style.justify = Some(Justify::SpaceBetween);
        }
        _ => {}
    }
    if l.flex_grow > 0.0 {
        style.grow = true;
    }
    match l.width {
        Length::Percent(p) if (p - 1.0).abs() < f32::EPSILON => style.fill_width = true,
        Length::Px(w) => style.width = Some(f64::from(w)),
        _ => {}
    }
    match l.height {
        Length::Percent(p) if (p - 1.0).abs() < f32::EPSILON => style.fill_height = true,
        Length::Px(h) => style.height = Some(f64::from(h)),
        _ => {}
    }
    if let Length::Px(mh) = l.min_height
        && style.height.is_none()
    {
        style.height = Some(f64::from(mh));
    }
    if let Length::Px(mw) = l.max_width {
        style.max_width = Some(f64::from(mw));
    }
    if l.overflow_y == Overflow::Scroll {
        style.scroll = true;
    }
    if let Some(bg) = &tw.bg_token {
        style.background = Some(bg.clone());
    }
    if let Some(fg) = &tw.text_token {
        style.color = Some(fg.clone());
    }
    if let Some(bc) = &tw.border_token {
        style.border_color = Some(bc.clone());
    }
    if let Some(bw) = tw.border_width_px {
        style.border_width = Some(f64::from(bw));
    }
    if let Some(sh) = &tw.shape_token {
        style.shape = Some(sh.clone());
    }
    if let Some(el) = tw.elevation {
        style.elevation = Some(el);
    }
    if let Some(ts) = &tw.typescale {
        style.typescale = Some(ts.clone());
    }
}

fn component_body<'a, 'b>(stmts: &'b [Statement<'a>], span: Span, lw: &mut Lower<'_>) -> Option<&'b Expression<'a>> {
    let mut ret = None;
    for s in stmts {
        match s {
            Statement::ReturnStatement(r) if ret.is_none() => ret = r.argument.as_ref(),
            Statement::VariableDeclaration(v) => {
                let hook = v.declarations.iter().find_map(|d| match &d.init {
                    Some(Expression::CallExpression(c)) => match &c.callee {
                        Expression::Identifier(id) if id.name.starts_with("use") => Some(id.name.to_string()),
                        _ => None,
                    },
                    _ => None,
                });
                match hook {
                    Some(h) => lw.err(v.span, format!("hook `{h}` is not supported: components must be stateless (each backend generates its own state)")),
                    None => lw.err(v.span, "local declarations in the component body are not converted; move constants to module scope"),
                }
            }
            other => lw.err(other.span(), "only a single `return <jsx/>` is allowed in the component body"),
        }
    }
    if ret.is_none() {
        lw.err(span, "component must return JSX");
    }
    ret
}

/// Parses `src` and lowers its single component. Errors are returned as diagnostics.
pub fn lower(path: &Path, src: &str) -> Result<Document, Vec<Diag>> {
    let file = path.display().to_string();
    let allocator = Allocator::default();
    let source_type = SourceType::from_path(path).unwrap_or_else(|_| SourceType::tsx());
    let parsed = Parser::new(&allocator, src, source_type).parse();
    let mut lw = Lower {
        file: file.clone(),
        lines: LineIndex::new(src),
        diags: Vec::new(),
        scopes: vec![HashMap::new()],
        classes: ClassMap::new(),
        handlers: Vec::new(),
    };
    if !parsed.diagnostics.is_empty() {
        for d in parsed.diagnostics.iter() {
            let off = d.labels.first().map_or(0, |l| l.offset());
            let (line, col) = lw.lines.pos(off);
            lw.diags.push(Diag { file: file.clone(), line, col, msg: format!("syntax: {}", d.message) });
        }
        return Err(lw.diags);
    }

    // Module scope: imports, constants, one component.
    let mut components: Comps<'_, '_> = Vec::new();
    let base = path.parent().unwrap_or(Path::new("."));
    for stmt in &parsed.program.body {
        match stmt {
            Statement::ImportDeclaration(d) => {
                let spec = d.source.value.as_str().unwrap_or_default();
                if spec.ends_with(".css") {
                    let css_path = base.join(spec);
                    match std::fs::read_to_string(&css_path) {
                        Ok(text) => {
                            let mut map = std::mem::take(&mut lw.classes);
                            css::parse_stylesheet(&text, &css_path.display().to_string(), &mut lw.diags, &mut map);
                            lw.classes = map;
                        }
                        Err(e) => lw.err(d.source.span, format!("cannot read `{}`: {e}", css_path.display())),
                    }
                }
            }
            Statement::TSTypeAliasDeclaration(_) | Statement::TSInterfaceDeclaration(_) | Statement::EmptyStatement(_) => {}
            Statement::FunctionDeclaration(f) => push_fn(f, &mut components),
            Statement::ExportDefaultDeclaration(d) => match &d.declaration {
                ExportDefaultDeclarationKind::FunctionDeclaration(f) => push_fn(f, &mut components),
                ExportDefaultDeclarationKind::ArrowFunctionExpression(a) => match &a.body {
                    ArrowFunctionBody::FunctionBody(b) => components.push(("Component".into(), a.span, b.statements.as_slice(), None)),
                    b => components.push(("Component".into(), a.span, &[], b.as_expression())),
                },
                ExportDefaultDeclarationKind::Identifier(_) => {}
                other => lw.err(other.span(), "default export must be a function component"),
            },
            Statement::ExportDeclaration(d) => match &d.declaration {
                Declaration::FunctionDeclaration(f) => push_fn(f, &mut components),
                Declaration::VariableDeclaration(v) => module_const(v, &mut lw, &mut components),
                Declaration::TSTypeAliasDeclaration(_) | Declaration::TSInterfaceDeclaration(_) => {}
                other => lw.err(other.span(), "unsupported export"),
            },
            Statement::VariableDeclaration(v) => module_const(v, &mut lw, &mut components),
            Statement::ExportNamedDeclaration(_) => {}
            other => lw.err(other.span(), "unsupported top-level statement"),
        }
    }

    if components.len() != 1 {
        lw.diags.push(Diag {
            file: file.clone(),
            line: 1,
            col: 1,
            msg: format!("expected exactly one component function per file, found {}", components.len()),
        });
        return Err(lw.diags);
    }
    let (component, span, stmts, expr) = components.pop().unwrap();
    let ret = match expr {
        Some(e) => Some(e),
        None => component_body(stmts, span, &mut lw),
    };
    let mut root = None;
    if let Some(mut e) = ret {
        while let Expression::ParenthesizedExpression(p) = e {
            e = &p.expression;
        }
        match e {
            Expression::JSXElement(el) => root = lw.element(el),
            Expression::JSXFragment(f) => {
                let (line, col) = lw.lines.pos(f.span.start);
                let mut n = Node::new(Kind::Box, line, col);
                let kids = lw.children(&f.children);
                lw.attach(&mut n, "fragment", kids);
                root = Some(n);
            }
            other => lw.err(other.span(), "component must return a JSX element"),
        }
    }
    if !lw.diags.is_empty() {
        return Err(lw.diags);
    }
    let Some(root) = root else { return Err(lw.diags) };
    Ok(Document {
        component,
        source: path.file_name().map_or(file.clone(), |n| n.to_string_lossy().into_owned()),
        root,
        handlers: lw.handlers,
    })
}

type Comps<'a, 'b> = Vec<(String, Span, &'b [Statement<'a>], Option<&'b Expression<'a>>)>;

fn push_fn<'a, 'b>(f: &'b Function<'a>, comps: &mut Comps<'a, 'b>) {
    let name = f.id.as_ref().map_or("Component".into(), |i| i.name.to_string());
    if let Some(b) = &f.body {
        comps.push((name, f.span, b.statements.as_slice(), None));
    }
}

fn module_const<'a, 'b>(
    v: &'b VariableDeclaration<'a>,
    lw: &mut Lower<'_>,
    comps: &mut Comps<'a, 'b>,
) {
    if v.kind != VariableDeclarationKind::Const {
        lw.err(v.span, "only `const` declarations are allowed at module scope");
        return;
    }
    for d in &v.declarations {
        let BindingPattern::BindingIdentifier(id) = &d.id else {
            lw.err(d.span, "destructuring is not supported at module scope");
            continue;
        };
        let name = id.name.to_string();
        let Some(init) = &d.init else {
            lw.err(d.span, "constant without initializer");
            continue;
        };
        match init {
            Expression::ArrowFunctionExpression(a) => match &a.body {
                ArrowFunctionBody::FunctionBody(b) => comps.push((name, a.span, b.statements.as_slice(), None)),
                b => comps.push((name, a.span, &[], b.as_expression())),
            },
            Expression::FunctionExpression(f) => {
                if let Some(b) = &f.body {
                    comps.push((name, f.span, b.statements.as_slice(), None));
                }
            }
            e => match lw.eval(e) {
                Ok(c) => {
                    lw.scopes[0].insert(name, c);
                }
                Err((sp, m)) => lw.err(sp, format!("module constant `{name}`: {m}")),
            },
        }
    }
}
