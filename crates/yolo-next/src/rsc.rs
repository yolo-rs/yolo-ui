// SPDX-License-Identifier: Apache-2.0
//! React Server Components module graph: `"use client"` / `"use server"` / `"use cache"`
//! classification, Flight client/server reference manifests, boundary violations.
//!
//! Verified against Next.js 16.4.5: `crates/next-custom-transforms/src/transforms/server_actions.rs`
//! (inline `"use server"` functions hoisted as `$$RSC_SERVER_ACTION_<n>`, `"use cache: <variant>"`
//! directives), the `react-server-dom-webpack` manifest shapes (`clientModules`,
//! `ssrModuleMapping`, `server-reference-manifest.json`) and the Server Components boundary rules
//! (hooks and `client-only` are client-side, `server-only` / `next/headers` / `next/cache` are
//! server-side, request APIs are forbidden inside `"use cache"`).
//!
//! This module also hosts the crate-internal OXC 0.155 helpers (the [`Visit`] walker and the
//! static literal readers) shared with [`crate::fonts`] and [`crate::images`].

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use oxc_allocator::Allocator;
#[allow(clippy::wildcard_imports)]
use oxc_ast::ast::*;
use oxc_parser::Parser;
use oxc_span::SourceType;
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};

use crate::types::{
    CacheFunction, ClientReference, Diagnostic, ModuleDirective, ModuleGraph, ModuleInfo,
    ModuleKind, ServerReference,
};

// ============================================================================
// Crate-internal OXC helpers (shared with `fonts` and `images`)
// ============================================================================

/// Source type for a module path: `.ts` never enables JSX (generics), everything else does.
pub(crate) fn source_type_for(file: &str) -> SourceType {
    let ext = Path::new(file)
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();
    match ext.as_str() {
        "ts" | "mts" | "cts" => SourceType::ts(),
        "js" | "jsx" | "mjs" | "cjs" => SourceType::jsx(),
        _ => SourceType::tsx(),
    }
}

/// UTF-8 text of a string literal (lone surrogates are rendered lossily).
pub(crate) fn literal_text(lit: &StringLiteral<'_>) -> String {
    lit.value
        .as_str()
        .map_or_else(|| lit.value.display().to_string(), str::to_owned)
}

/// Text of a template literal; `placeholder` replaces every `${expr}` hole.
pub(crate) fn template_text(tpl: &TemplateLiteral<'_>, placeholder: &str) -> String {
    let mut out = String::new();
    for (i, quasi) in tpl.quasis.iter().enumerate() {
        if i > 0 {
            out.push_str(placeholder);
        }
        let cooked = match quasi.value.cooked {
            Some(text) => text.as_str(),
            None => None,
        };
        match cooked {
            Some(cooked) => out.push_str(cooked),
            None => out.push_str(quasi.value.raw.as_str()),
        }
    }
    out
}

/// Strips parentheses and TypeScript-only wrappers (`as`, `satisfies`, `!`, `<T>`).
pub(crate) fn unwrap_ts<'a, 'b>(expr: &'b Expression<'a>) -> &'b Expression<'a> {
    match expr {
        Expression::ParenthesizedExpression(e) => unwrap_ts(&e.expression),
        Expression::TSAsExpression(e) => unwrap_ts(&e.expression),
        Expression::TSSatisfiesExpression(e) => unwrap_ts(&e.expression),
        Expression::TSNonNullExpression(e) => unwrap_ts(&e.expression),
        Expression::TSTypeAssertion(e) => unwrap_ts(&e.expression),
        Expression::TSInstantiationExpression(e) => unwrap_ts(&e.expression),
        other => other,
    }
}

/// Statically known string (`"x"`, `'x'`, `` `x` `` without holes).
pub(crate) fn static_string(expr: &Expression<'_>) -> Option<String> {
    match unwrap_ts(expr) {
        Expression::StringLiteral(s) => Some(literal_text(s)),
        Expression::TemplateLiteral(t) if t.expressions.is_empty() => Some(template_text(t, "")),
        _ => None,
    }
}

/// Statically known number (`800`, `-1`, `"800"`).
pub(crate) fn static_number(expr: &Expression<'_>) -> Option<f64> {
    match unwrap_ts(expr) {
        Expression::NumericLiteral(n) => Some(n.value),
        Expression::UnaryExpression(u) if u.operator.as_str() == "-" => {
            static_number(&u.argument).map(|v| -v)
        }
        Expression::StringLiteral(s) => literal_text(s).trim().parse().ok(),
        _ => None,
    }
}

/// Statically known boolean.
pub(crate) fn static_bool(expr: &Expression<'_>) -> Option<bool> {
    match unwrap_ts(expr) {
        Expression::BooleanLiteral(b) => Some(b.value),
        _ => None,
    }
}

/// Formats a JS number the way `String(n)` does for integers.
pub(crate) fn format_number(value: f64) -> String {
    if value.fract() == 0.0 && value.abs() < 1e15 {
        format!("{}", value as i64)
    } else {
        format!("{value}")
    }
}

/// Name of a non-computed property key.
pub(crate) fn property_key_name(key: &PropertyKey<'_>) -> Option<String> {
    match key {
        PropertyKey::StaticIdentifier(id) => Some(id.name.as_str().to_owned()),
        PropertyKey::StringLiteral(s) => Some(literal_text(s)),
        PropertyKey::NumericLiteral(n) => Some(format_number(n.value)),
        PropertyKey::TemplateLiteral(t) if t.expressions.is_empty() => Some(template_text(t, "")),
        _ => None,
    }
}

/// `(key, value)` pairs of an object literal with static keys (spreads are skipped).
pub(crate) fn object_entries<'a, 'b>(
    expr: &'b Expression<'a>,
) -> Option<Vec<(String, &'b Expression<'a>)>> {
    let Expression::ObjectExpression(obj) = unwrap_ts(expr) else {
        return None;
    };
    Some(
        obj.properties
            .iter()
            .filter_map(|prop| match prop {
                ObjectPropertyKind::ObjectProperty(p) => {
                    property_key_name(&p.key).map(|k| (k, &p.value))
                }
                ObjectPropertyKind::SpreadProperty(_) => None,
            })
            .collect(),
    )
}

/// Elements of an array literal (spreads and holes are skipped).
pub(crate) fn array_items<'a, 'b>(expr: &'b Expression<'a>) -> Option<Vec<&'b Expression<'a>>> {
    let Expression::ArrayExpression(arr) = unwrap_ts(expr) else {
        return None;
    };
    Some(
        arr.elements
            .iter()
            .filter_map(ArrayExpressionElement::as_expression)
            .collect(),
    )
}

/// A string or an array of strings (numbers are stringified, e.g. `weight: 400`).
pub(crate) fn string_list(expr: &Expression<'_>) -> Vec<String> {
    if let Some(single) = static_string(expr) {
        return vec![single];
    }
    if let Some(number) = static_number(expr) {
        return vec![format_number(number)];
    }
    array_items(expr)
        .map(|items| {
            items
                .into_iter()
                .filter_map(|item| {
                    static_string(item).or_else(|| static_number(item).map(format_number))
                })
                .collect()
        })
        .unwrap_or_default()
}

/// JSON value of a static literal tree (strings, numbers, booleans, null, arrays, objects).
pub(crate) fn static_json(expr: &Expression<'_>) -> Option<Value> {
    let expr = unwrap_ts(expr);
    if let Some(s) = static_string(expr) {
        return Some(Value::String(s));
    }
    if let Some(b) = static_bool(expr) {
        return Some(Value::Bool(b));
    }
    if let Some(n) = static_number(expr) {
        if n.fract() == 0.0 && n.abs() < 9.0e15 {
            return Some(Value::from(n as i64));
        }
        return serde_json::Number::from_f64(n).map(Value::Number);
    }
    match expr {
        Expression::NullLiteral(_) => Some(Value::Null),
        Expression::ArrayExpression(_) => Some(Value::Array(
            array_items(expr)?
                .into_iter()
                .filter_map(static_json)
                .collect(),
        )),
        Expression::ObjectExpression(_) => {
            let mut map = Map::new();
            for (key, value) in object_entries(expr)? {
                if let Some(value) = static_json(value) {
                    map.insert(key, value);
                }
            }
            Some(Value::Object(map))
        }
        _ => None,
    }
}

/// Name of an export / import specifier.
pub(crate) fn module_export_name(name: &ModuleExportName<'_>) -> String {
    match name {
        ModuleExportName::IdentifierName(id) => id.name.as_str().to_owned(),
        ModuleExportName::IdentifierReference(id) => id.name.as_str().to_owned(),
        ModuleExportName::StringLiteral(s) => literal_text(s),
    }
}

/// Callee name of a call: `foo(...)` -> `foo`, `React.useState(...)` -> `useState`.
pub(crate) fn callee_name<'b>(callee: &'b Expression<'_>) -> Option<&'b str> {
    match callee {
        Expression::Identifier(id) => Some(id.name.as_str()),
        Expression::StaticMemberExpression(m) => Some(m.property.name.as_str()),
        _ => None,
    }
}

/// Pushes `value` when it is not already present.
pub(crate) fn push_unique(list: &mut Vec<String>, value: &str) {
    if !list.iter().any(|existing| existing == value) {
        list.push(value.to_owned());
    }
}

/// Callback surface of [`walk_program`].
pub(crate) trait Visit {
    /// Every expression node (pre-order).
    fn expr(&mut self, _expr: &Expression<'_>) {}
    /// Every JSX element, including elements nested as JSX children or attribute values.
    fn jsx_element(&mut self, _element: &JSXElement<'_>) {}
    /// Every function / method / arrow with a block body (`name` is the declared or inferred name).
    fn func(&mut self, _body: &FunctionBody<'_>, _is_async: bool, _name: Option<&str>) {}
}

/// Walks every statement, expression, function body and JSX node of a program.
pub(crate) fn walk_program<V: Visit>(program: &Program<'_>, visitor: &mut V) {
    walk_stmts(&program.body, visitor);
}

fn walk_stmts<V: Visit>(stmts: &[Statement<'_>], v: &mut V) {
    for stmt in stmts {
        walk_stmt(stmt, v);
    }
}

fn walk_stmt<V: Visit>(stmt: &Statement<'_>, v: &mut V) {
    match stmt {
        Statement::VariableDeclaration(decl) => walk_var_decl(decl, v),
        Statement::FunctionDeclaration(f) => walk_function(f, v, None),
        Statement::ClassDeclaration(c) => walk_class(c, v),
        Statement::ExpressionStatement(s) => walk_expr(&s.expression, v, None),
        Statement::ReturnStatement(s) => {
            if let Some(arg) = &s.argument {
                walk_expr(arg, v, None);
            }
        }
        Statement::ThrowStatement(s) => walk_expr(&s.argument, v, None),
        Statement::BlockStatement(b) => walk_stmts(&b.body, v),
        Statement::IfStatement(s) => {
            walk_expr(&s.test, v, None);
            walk_stmt(&s.consequent, v);
            if let Some(alt) = &s.alternate {
                walk_stmt(alt, v);
            }
        }
        Statement::ForStatement(s) => {
            match &s.init {
                Some(ForStatementInit::VariableDeclaration(d)) => walk_var_decl(d, v),
                Some(other) => {
                    if let Some(e) = other.as_expression() {
                        walk_expr(e, v, None);
                    }
                }
                None => {}
            }
            if let Some(test) = &s.test {
                walk_expr(test, v, None);
            }
            if let Some(update) = &s.update {
                walk_expr(update, v, None);
            }
            walk_stmt(&s.body, v);
        }
        Statement::ForInStatement(s) => {
            walk_expr(&s.right, v, None);
            walk_stmt(&s.body, v);
        }
        Statement::ForOfStatement(s) => {
            walk_expr(&s.right, v, None);
            walk_stmt(&s.body, v);
        }
        Statement::WhileStatement(s) => {
            walk_expr(&s.test, v, None);
            walk_stmt(&s.body, v);
        }
        Statement::DoWhileStatement(s) => {
            walk_stmt(&s.body, v);
            walk_expr(&s.test, v, None);
        }
        Statement::TryStatement(s) => {
            walk_stmts(&s.block.body, v);
            if let Some(handler) = &s.handler {
                walk_stmts(&handler.body.body, v);
            }
            if let Some(finalizer) = &s.finalizer {
                walk_stmts(&finalizer.body, v);
            }
        }
        Statement::SwitchStatement(s) => {
            walk_expr(&s.discriminant, v, None);
            for case in &s.cases {
                if let Some(test) = &case.test {
                    walk_expr(test, v, None);
                }
                walk_stmts(&case.consequent, v);
            }
        }
        Statement::LabeledStatement(s) => walk_stmt(&s.body, v),
        Statement::ExportDeclaration(e) => walk_declaration(&e.declaration, v),
        Statement::ExportDefaultDeclaration(e) => match &e.declaration {
            ExportDefaultDeclarationKind::FunctionDeclaration(f) => {
                // References name the export, so a default export is always `default`.
                if let Some(body) = &f.body {
                    v.func(body, f.r#async, Some("default"));
                    walk_stmts(&body.statements, v);
                }
            }
            ExportDefaultDeclarationKind::ClassDeclaration(c) => walk_class(c, v),
            other => {
                if let Some(expr) = other.as_expression() {
                    walk_expr(expr, v, Some("default"));
                }
            }
        },
        _ => {}
    }
}

fn walk_declaration<V: Visit>(decl: &Declaration<'_>, v: &mut V) {
    match decl {
        Declaration::VariableDeclaration(d) => walk_var_decl(d, v),
        Declaration::FunctionDeclaration(f) => walk_function(f, v, None),
        Declaration::ClassDeclaration(c) => walk_class(c, v),
        _ => {}
    }
}

fn walk_var_decl<V: Visit>(decl: &VariableDeclaration<'_>, v: &mut V) {
    for declarator in &decl.declarations {
        if let Some(init) = &declarator.init {
            let name = match &declarator.id {
                BindingPattern::BindingIdentifier(id) => Some(id.name.as_str()),
                _ => None,
            };
            walk_expr(init, v, name);
        }
    }
}

fn walk_function<V: Visit>(func: &Function<'_>, v: &mut V, name_hint: Option<&str>) {
    let name = func.id.as_ref().map(|id| id.name.as_str()).or(name_hint);
    if let Some(body) = &func.body {
        v.func(body, func.r#async, name);
        walk_stmts(&body.statements, v);
    }
}

fn walk_arrow<V: Visit>(arrow: &ArrowFunctionExpression<'_>, v: &mut V, name_hint: Option<&str>) {
    match &arrow.body {
        ArrowFunctionBody::FunctionBody(body) => {
            v.func(body, arrow.r#async, name_hint);
            walk_stmts(&body.statements, v);
        }
        other => {
            if let Some(expr) = other.as_expression() {
                walk_expr(expr, v, None);
            }
        }
    }
}

fn walk_class<V: Visit>(class: &Class<'_>, v: &mut V) {
    for element in &class.body.body {
        match element {
            ClassElement::MethodDefinition(m) => {
                let name = property_key_name(&m.key);
                walk_function(&m.value, v, name.as_deref());
            }
            ClassElement::PropertyDefinition(p) => {
                if let Some(value) = &p.value {
                    let name = property_key_name(&p.key);
                    walk_expr(value, v, name.as_deref());
                }
            }
            ClassElement::AccessorProperty(p) => {
                if let Some(value) = &p.value {
                    walk_expr(value, v, None);
                }
            }
            ClassElement::StaticBlock(b) => walk_stmts(&b.body, v),
            ClassElement::TSIndexSignature(_) => {}
        }
    }
}

fn walk_args<V: Visit>(args: &[Argument<'_>], v: &mut V) {
    for arg in args {
        match arg {
            Argument::SpreadElement(s) => walk_expr(&s.argument, v, None),
            other => {
                if let Some(expr) = other.as_expression() {
                    walk_expr(expr, v, None);
                }
            }
        }
    }
}

fn walk_member<V: Visit>(member: &MemberExpression<'_>, v: &mut V) {
    match member {
        MemberExpression::StaticMemberExpression(m) => walk_expr(&m.object, v, None),
        MemberExpression::ComputedMemberExpression(m) => {
            walk_expr(&m.object, v, None);
            walk_expr(&m.expression, v, None);
        }
        MemberExpression::PrivateFieldExpression(m) => walk_expr(&m.object, v, None),
    }
}

fn walk_expr<V: Visit>(expr: &Expression<'_>, v: &mut V, name_hint: Option<&str>) {
    v.expr(expr);
    match expr {
        Expression::CallExpression(c) => {
            walk_expr(&c.callee, v, None);
            walk_args(&c.arguments, v);
        }
        Expression::NewExpression(n) => {
            walk_expr(&n.callee, v, None);
            walk_args(&n.arguments, v);
        }
        Expression::ArrowFunctionExpression(a) => walk_arrow(a, v, name_hint),
        Expression::FunctionExpression(f) => walk_function(f, v, name_hint),
        Expression::ClassExpression(c) => walk_class(c, v),
        Expression::ObjectExpression(o) => {
            for prop in &o.properties {
                match prop {
                    ObjectPropertyKind::ObjectProperty(p) => {
                        let key = property_key_name(&p.key);
                        walk_expr(&p.value, v, key.as_deref());
                    }
                    ObjectPropertyKind::SpreadProperty(s) => walk_expr(&s.argument, v, None),
                }
            }
        }
        Expression::ArrayExpression(a) => {
            for element in &a.elements {
                match element {
                    ArrayExpressionElement::SpreadElement(s) => walk_expr(&s.argument, v, None),
                    other => {
                        if let Some(e) = other.as_expression() {
                            walk_expr(e, v, None);
                        }
                    }
                }
            }
        }
        Expression::JSXElement(el) => {
            v.jsx_element(el);
            walk_jsx_element(el, v);
        }
        Expression::JSXFragment(f) => walk_jsx_children(&f.children, v),
        Expression::ImportExpression(i) => {
            walk_expr(&i.source, v, None);
            if let Some(options) = &i.options {
                walk_expr(options, v, None);
            }
        }
        other => walk_expr_operators(other, v, name_hint),
    }
}

fn walk_expr_operators<V: Visit>(expr: &Expression<'_>, v: &mut V, name_hint: Option<&str>) {
    match expr {
        Expression::TemplateLiteral(t) => {
            for e in &t.expressions {
                walk_expr(e, v, None);
            }
        }
        Expression::TaggedTemplateExpression(t) => {
            walk_expr(&t.tag, v, None);
            for e in &t.quasi.expressions {
                walk_expr(e, v, None);
            }
        }
        Expression::ConditionalExpression(c) => {
            walk_expr(&c.test, v, None);
            walk_expr(&c.consequent, v, None);
            walk_expr(&c.alternate, v, None);
        }
        Expression::LogicalExpression(l) => {
            walk_expr(&l.left, v, None);
            walk_expr(&l.right, v, None);
        }
        Expression::BinaryExpression(b) => {
            walk_expr(&b.left, v, None);
            walk_expr(&b.right, v, None);
        }
        Expression::UnaryExpression(u) => walk_expr(&u.argument, v, None),
        Expression::AssignmentExpression(a) => walk_expr(&a.right, v, None),
        Expression::SequenceExpression(s) => {
            for e in &s.expressions {
                walk_expr(e, v, None);
            }
        }
        Expression::ParenthesizedExpression(p) => walk_expr(&p.expression, v, name_hint),
        Expression::AwaitExpression(a) => walk_expr(&a.argument, v, None),
        Expression::YieldExpression(y) => {
            if let Some(arg) = &y.argument {
                walk_expr(arg, v, None);
            }
        }
        Expression::ChainExpression(c) => match &c.expression {
            ChainElement::CallExpression(call) => {
                walk_expr(&call.callee, v, None);
                walk_args(&call.arguments, v);
            }
            ChainElement::TSNonNullExpression(e) => walk_expr(&e.expression, v, None),
            other => {
                if let Some(member) = other.as_member_expression() {
                    walk_member(member, v);
                }
            }
        },
        Expression::StaticMemberExpression(m) => walk_expr(&m.object, v, None),
        Expression::ComputedMemberExpression(m) => {
            walk_expr(&m.object, v, None);
            walk_expr(&m.expression, v, None);
        }
        Expression::PrivateFieldExpression(m) => walk_expr(&m.object, v, None),
        Expression::TSAsExpression(e) => walk_expr(&e.expression, v, name_hint),
        Expression::TSSatisfiesExpression(e) => walk_expr(&e.expression, v, name_hint),
        Expression::TSNonNullExpression(e) => walk_expr(&e.expression, v, name_hint),
        Expression::TSTypeAssertion(e) => walk_expr(&e.expression, v, name_hint),
        Expression::TSInstantiationExpression(e) => walk_expr(&e.expression, v, name_hint),
        _ => {}
    }
}

fn walk_jsx_element<V: Visit>(element: &JSXElement<'_>, v: &mut V) {
    for attr in &element.opening_element.attributes {
        match attr {
            JSXAttributeItem::Attribute(a) => match &a.value {
                Some(JSXAttributeValue::ExpressionContainer(c)) => {
                    if let Some(e) = c.expression.as_expression() {
                        walk_expr(e, v, None);
                    }
                }
                Some(JSXAttributeValue::Element(inner)) => {
                    v.jsx_element(inner);
                    walk_jsx_element(inner, v);
                }
                Some(JSXAttributeValue::Fragment(f)) => walk_jsx_children(&f.children, v),
                _ => {}
            },
            JSXAttributeItem::SpreadAttribute(s) => walk_expr(&s.argument, v, None),
        }
    }
    walk_jsx_children(&element.children, v);
}

fn walk_jsx_children<V: Visit>(children: &[JSXChild<'_>], v: &mut V) {
    for child in children {
        match child {
            JSXChild::Element(el) => {
                v.jsx_element(el);
                walk_jsx_element(el, v);
            }
            JSXChild::Fragment(f) => walk_jsx_children(&f.children, v),
            JSXChild::ExpressionContainer(c) => {
                if let Some(e) = c.expression.as_expression() {
                    walk_expr(e, v, None);
                }
            }
            JSXChild::Spread(s) => walk_expr(&s.expression, v, None),
            JSXChild::Text(_) => {}
        }
    }
}

// ============================================================================
// Module analysis
// ============================================================================

/// React hooks that only work in Client Components (every built-in hook except `use`).
const CLIENT_ONLY_HOOKS: [&str; 17] = [
    "useState",
    "useEffect",
    "useLayoutEffect",
    "useInsertionEffect",
    "useReducer",
    "useContext",
    "useRef",
    "useImperativeHandle",
    "useOptimistic",
    "useActionState",
    "useTransition",
    "useDeferredValue",
    "useSyncExternalStore",
    "useMemo",
    "useCallback",
    "useId",
    "useFormStatus",
];

/// `next/navigation` hooks (client-only).
const NAVIGATION_HOOKS: [&str; 5] = [
    "useRouter",
    "usePathname",
    "useSearchParams",
    "useParams",
    "useSelectedLayoutSegments",
];

/// App Router special files that are always server entries (never pure client leaves).
const APP_ENTRY_STEMS: [&str; 11] = [
    "page",
    "layout",
    "template",
    "loading",
    "error",
    "not-found",
    "default",
    "global-error",
    "forbidden",
    "unauthorized",
    "route",
];

fn file_extension(id: &str) -> String {
    Path::new(id)
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default()
}

fn file_stem(id: &str) -> String {
    Path::new(id)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or_default()
        .to_owned()
}

fn is_app_entry(id: &str) -> bool {
    let stem = file_stem(id);
    APP_ENTRY_STEMS.contains(&stem.as_str())
}

fn is_hook_name(name: &str) -> bool {
    name == "use"
        || (name.starts_with("use") && name.chars().nth(3).is_some_and(|c| c.is_ascii_uppercase()))
}

fn is_next_import(spec: &str) -> bool {
    spec == "next"
        || spec.starts_with("next/")
        || spec.starts_with("@next/")
        || spec == "server-only"
        || spec == "client-only"
}

fn is_server_only_import(spec: &str) -> bool {
    matches!(
        spec,
        "server-only"
            | "next/headers"
            | "next/cache"
            | "fs"
            | "fs/promises"
            | "child_process"
            | "bun"
            | "bun:sqlite"
    ) || spec.starts_with("node:")
        || spec.starts_with("bun:")
}

fn module_directive(program: &Program<'_>) -> ModuleDirective {
    for directive in &program.directives {
        match directive.directive.as_str() {
            "use client" => return ModuleDirective::UseClient,
            "use server" => return ModuleDirective::UseServer,
            text if text.starts_with("use cache") => return ModuleDirective::UseCache,
            _ => {}
        }
    }
    ModuleDirective::None
}

fn collect_binding_names(pattern: &BindingPattern<'_>, out: &mut Vec<String>) {
    match pattern {
        BindingPattern::BindingIdentifier(id) => push_unique(out, id.name.as_str()),
        BindingPattern::ObjectPattern(obj) => {
            for prop in &obj.properties {
                collect_binding_names(&prop.value, out);
            }
            if let Some(rest) = &obj.rest {
                collect_binding_names(&rest.argument, out);
            }
        }
        BindingPattern::ArrayPattern(arr) => {
            for element in arr.elements.iter().flatten() {
                collect_binding_names(element, out);
            }
            if let Some(rest) = &arr.rest {
                collect_binding_names(&rest.argument, out);
            }
        }
        BindingPattern::AssignmentPattern(assign) => collect_binding_names(&assign.left, out),
    }
}

/// Top-level imports and exports of a module.
fn collect_module_items(program: &Program<'_>, info: &mut ModuleInfo) {
    for stmt in &program.body {
        match stmt {
            Statement::ImportDeclaration(import) => {
                if import.import_kind.is_type() {
                    continue;
                }
                push_unique(&mut info.imports, &literal_text(&import.source));
            }
            Statement::ExportDeclaration(export) => match &export.declaration {
                Declaration::VariableDeclaration(decl) => {
                    for declarator in &decl.declarations {
                        collect_binding_names(&declarator.id, &mut info.exports);
                    }
                }
                Declaration::FunctionDeclaration(func) => {
                    if let Some(id) = &func.id {
                        push_unique(&mut info.exports, id.name.as_str());
                    }
                }
                Declaration::ClassDeclaration(class) => {
                    if let Some(id) = &class.id {
                        push_unique(&mut info.exports, id.name.as_str());
                    }
                }
                Declaration::TSEnumDeclaration(en) => {
                    push_unique(&mut info.exports, en.id.name.as_str());
                }
                _ => {}
            },
            Statement::ExportNamedDeclaration(export) => {
                for spec in &export.specifiers {
                    if spec.export_kind.is_type() {
                        continue;
                    }
                    push_unique(&mut info.exports, &module_export_name(&spec.exported));
                }
            }
            Statement::ExportFromDeclaration(export) => {
                push_unique(&mut info.imports, &literal_text(&export.source));
                for spec in &export.specifiers {
                    if spec.export_kind.is_type() {
                        continue;
                    }
                    push_unique(&mut info.exports, &module_export_name(&spec.exported));
                }
            }
            Statement::ExportDefaultDeclaration(_) => push_unique(&mut info.exports, "default"),
            Statement::ExportAllDeclaration(export) => {
                push_unique(&mut info.imports, &literal_text(&export.source));
                let name = export
                    .exported
                    .as_ref()
                    .map_or_else(|| String::from("*"), module_export_name);
                push_unique(&mut info.exports, &name);
            }
            _ => {}
        }
    }
}

/// `cacheLife(...)` / `cacheTag(...)` calls inside one `"use cache"` body.
#[derive(Default)]
struct CacheCalls {
    life: Option<String>,
    tags: Vec<String>,
}

impl Visit for CacheCalls {
    fn expr(&mut self, expr: &Expression<'_>) {
        let Expression::CallExpression(call) = expr else {
            return;
        };
        match callee_name(&call.callee) {
            Some("cacheLife" | "unstable_cacheLife") => {
                if let Some(arg) = call.arguments.first().and_then(Argument::as_expression) {
                    self.life = static_string(arg)
                        .or_else(|| static_json(arg).map(|value| value.to_string()));
                }
            }
            Some("cacheTag" | "unstable_cacheTag") => {
                for arg in call.arguments.iter().filter_map(Argument::as_expression) {
                    match unwrap_ts(arg) {
                        Expression::StringLiteral(s) => self.tags.push(literal_text(s)),
                        Expression::TemplateLiteral(t) => self.tags.push(template_text(t, "*")),
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
}

/// Whole-module walk: hooks, dynamic imports, inline actions, cache functions.
struct ModuleScan<'f> {
    file: &'f str,
    hooks: Vec<String>,
    dynamic_imports: Vec<String>,
    inline_actions: Vec<String>,
    cache_functions: Vec<CacheFunction>,
    diagnostics: Vec<Diagnostic>,
    saw_jsx: bool,
}

impl Visit for ModuleScan<'_> {
    fn expr(&mut self, expr: &Expression<'_>) {
        match expr {
            Expression::CallExpression(call) => {
                if let Some(name) = callee_name(&call.callee) {
                    if is_hook_name(name) {
                        push_unique(&mut self.hooks, name);
                    }
                    if name == "require"
                        && let Some(spec) = call
                            .arguments
                            .first()
                            .and_then(Argument::as_expression)
                            .and_then(static_string)
                    {
                        push_unique(&mut self.dynamic_imports, &spec);
                    }
                }
            }
            Expression::ImportExpression(import) => {
                if let Some(spec) = static_string(&import.source) {
                    push_unique(&mut self.dynamic_imports, &spec);
                }
            }
            _ => {}
        }
    }

    fn jsx_element(&mut self, _element: &JSXElement<'_>) {
        self.saw_jsx = true;
    }

    fn func(&mut self, body: &FunctionBody<'_>, is_async: bool, name: Option<&str>) {
        for directive in &body.directives {
            let text = directive.directive.as_str();
            if text == "use server" {
                let name = name.map_or_else(
                    || format!("$$RSC_SERVER_ACTION_{}", self.inline_actions.len()),
                    str::to_owned,
                );
                if !is_async {
                    self.diagnostics.push(
                        Diagnostic::error(
                            "server-action-not-async",
                            format!("inline Server Action `{name}` must be an async function"),
                        )
                        .with_file(self.file)
                        .with_hint("declare it as `async function` (Server Actions always return a Promise)"),
                    );
                }
                self.inline_actions.push(name);
            } else if let Some(rest) = text.strip_prefix("use cache") {
                let variant = rest.trim().trim_start_matches(':').trim().to_owned();
                let name = name.map_or_else(
                    || format!("$$RSC_CACHE_{}", self.cache_functions.len()),
                    str::to_owned,
                );
                if !is_async {
                    self.diagnostics.push(
                        Diagnostic::error(
                            "use-cache-not-async",
                            format!("`\"use cache\"` function `{name}` must be an async function"),
                        )
                        .with_file(self.file)
                        .with_hint(
                            "cached functions are awaited by the cache handler; declare it `async`",
                        ),
                    );
                }
                let mut calls = CacheCalls::default();
                walk_stmts(&body.statements, &mut calls);
                self.cache_functions.push(CacheFunction {
                    name,
                    life: calls.life,
                    tags: calls.tags,
                    variant,
                });
            }
        }
    }
}

fn classify(id: &str, directive: ModuleDirective, saw_jsx: bool) -> ModuleKind {
    match directive {
        ModuleDirective::UseClient => ModuleKind::Client,
        ModuleDirective::UseServer => ModuleKind::ServerAction,
        ModuleDirective::UseCache => ModuleKind::Cache,
        ModuleDirective::None => {
            let ext = file_extension(id);
            if ext == "tsx" || ext == "jsx" || saw_jsx || is_app_entry(id) {
                ModuleKind::Server
            } else {
                ModuleKind::Shared
            }
        }
    }
}

/// Analyzes one module (directives, exports, imports, inline actions, cache functions, hooks).
///
/// `id` is the project-relative path with `/` separators; its extension selects the parser
/// (`.ts` without JSX, everything else with JSX). Modules without a directive are classified
/// [`ModuleKind::Server`] when they are `.tsx` / `.jsx`, contain JSX or are App Router special
/// files, and [`ModuleKind::Shared`] otherwise (plain `.ts` utilities).
pub fn analyze_module(source: &str, id: &str) -> ModuleInfo {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, source_type_for(id)).parse();
    let program = &parsed.program;

    let mut info = ModuleInfo { id: id.to_owned(), ..ModuleInfo::default() };
    if !parsed.diagnostics.is_empty() {
        let first = parsed
            .diagnostics
            .iter()
            .next()
            .map(ToString::to_string)
            .unwrap_or_default();
        info.diagnostics.push(
            Diagnostic::warning(
                "rsc-parse-error",
                format!("`{id}` has parse diagnostics; analysis may be partial: {first}"),
            )
            .with_file(id),
        );
    }

    info.directive = module_directive(program);
    collect_module_items(program, &mut info);

    let mut scan = ModuleScan {
        file: id,
        hooks: Vec::new(),
        dynamic_imports: Vec::new(),
        inline_actions: Vec::new(),
        cache_functions: Vec::new(),
        diagnostics: Vec::new(),
        saw_jsx: false,
    };
    walk_program(program, &mut scan);

    for spec in &scan.dynamic_imports {
        push_unique(&mut info.imports, spec);
    }
    info.hooks = scan.hooks;
    let ext = file_extension(id);
    if (ext == "tsx" || ext == "jsx")
        && let Ok(analysis) = yolo_react::analyze_react_component(source)
    {
        for hook in analysis.hooks_used {
            push_unique(&mut info.hooks, &hook);
        }
    }
    info.next_imports = info
        .imports
        .iter()
        .filter(|spec| is_next_import(spec))
        .cloned()
        .collect();
    info.inline_server_actions = scan.inline_actions;
    info.cache_functions = scan.cache_functions;
    info.diagnostics.extend(scan.diagnostics);
    info.kind = classify(id, info.directive, scan.saw_jsx);
    info
}

// ============================================================================
// Boundary violations
// ============================================================================

/// Server-only imports in client modules and other boundary violations.
///
/// - Client modules importing `server-only`, `next/headers`, `next/cache`, `node:*`, `bun:*`,
///   `fs` or `bun` -> `client-imports-server-only`.
/// - Inline `"use server"` functions inside a Client module -> `inline-action-in-client`.
/// - Server / action / cache modules importing `client-only` -> `server-imports-client-only`.
/// - Server Components (no directive) calling client-only React hooks ->
///   `hooks-in-server-component`; calling `next/navigation` hooks ->
///   `navigation-hooks-in-server-component`.
/// - `"use cache"` modules or functions next to a `next/headers` import ->
///   `next-request-in-use-cache`.
pub fn boundary_violations(module: &ModuleInfo) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    let file = module.id.as_str();
    match module.kind {
        ModuleKind::Client => {
            for import in module.imports.iter().filter(|i| is_server_only_import(i)) {
                out.push(
                    Diagnostic::error(
                        "client-imports-server-only",
                        format!("Client Component `{file}` imports server-only module `{import}`"),
                    )
                    .with_file(file)
                    .with_hint(
                        "read the data in a Server Component, Route Handler or `\"use server\"` module and pass it down as props",
                    ),
                );
            }
            for action in &module.inline_server_actions {
                out.push(
                    Diagnostic::error(
                        "inline-action-in-client",
                        format!("inline `\"use server\"` function `{action}` is not allowed inside Client Component `{file}`"),
                    )
                    .with_file(file)
                    .with_hint("move the action into a `\"use server\"` module and import it"),
                );
            }
        }
        ModuleKind::Server | ModuleKind::ServerAction | ModuleKind::Cache => {
            if module.imports.iter().any(|i| i == "client-only") {
                out.push(
                    Diagnostic::error(
                        "server-imports-client-only",
                        format!("server module `{file}` imports `client-only`"),
                    )
                    .with_file(file)
                    .with_hint("add `\"use client\"` or drop the `client-only` import"),
                );
            }
        }
        ModuleKind::Shared => {}
    }
    if module.kind == ModuleKind::Server {
        let hooks: Vec<&str> = module
            .hooks
            .iter()
            .map(String::as_str)
            .filter(|h| CLIENT_ONLY_HOOKS.contains(h))
            .collect();
        if !hooks.is_empty() {
            out.push(
                Diagnostic::error(
                    "hooks-in-server-component",
                    format!("Server Component `{file}` calls client-only hooks: {}", hooks.join(", ")),
                )
                .with_file(file)
                .with_hint("add `\"use client\"` at the top of the file or move the stateful part into a Client Component"),
            );
        }
        let navigation: Vec<&str> = module
            .hooks
            .iter()
            .map(String::as_str)
            .filter(|h| NAVIGATION_HOOKS.contains(h))
            .collect();
        if !navigation.is_empty() && module.next_imports.iter().any(|i| i == "next/navigation") {
            out.push(
                Diagnostic::error(
                    "navigation-hooks-in-server-component",
                    format!(
                        "Server Component `{file}` calls `next/navigation` hooks: {}",
                        navigation.join(", ")
                    ),
                )
                .with_file(file)
                .with_hint(
                    "read `params` / `searchParams` props on the server or add `\"use client\"`",
                ),
            );
        }
    }
    if (module.kind == ModuleKind::Cache || !module.cache_functions.is_empty())
        && module.imports.iter().any(|i| i == "next/headers")
    {
        out.push(
            Diagnostic::error(
                "next-request-in-use-cache",
                format!("`{file}` mixes `\"use cache\"` with `next/headers` request APIs"),
            )
            .with_file(file)
            .with_hint("read `headers()` / `cookies()` outside the cached function and pass the values as arguments"),
        );
    }
    out
}

// ============================================================================
// Module graph
// ============================================================================

fn normalize_relative(path: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for segment in path.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            other => parts.push(other),
        }
    }
    parts.join("/")
}

/// Resolves a relative or `@/` / `~/` import specifier to a known module id.
fn resolve_import(from: &str, spec: &str, ids: &BTreeSet<String>) -> Option<String> {
    let bases: Vec<String> =
        if let Some(rest) = spec.strip_prefix("@/").or_else(|| spec.strip_prefix("~/")) {
            vec![rest.to_owned(), format!("src/{rest}")]
        } else if spec.starts_with("./") || spec.starts_with("../") {
            let dir = from.rsplit_once('/').map_or("", |(dir, _)| dir);
            vec![normalize_relative(&format!("{dir}/{spec}"))]
        } else {
            return None;
        };
    for base in bases {
        if ids.contains(&base) {
            return Some(base);
        }
        for ext in ["tsx", "ts", "jsx", "js", "mjs"] {
            let candidate = format!("{base}.{ext}");
            if ids.contains(&candidate) {
                return Some(candidate);
            }
            let candidate = format!("{base}/index.{ext}");
            if ids.contains(&candidate) {
                return Some(candidate);
            }
        }
    }
    None
}

fn chunk_name(module: &str) -> String {
    let stem = module.rsplit_once('.').map_or(module, |(stem, _)| stem);
    let sanitized: String = stem
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    format!("static/chunks/{sanitized}.js")
}

fn norm(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// `*.d.ts` (TypeScript declaration files carry no runtime code).
fn is_declaration_file(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.get(lower.len().saturating_sub(5)..) == Some(".d.ts")
}

/// Module sources under `dirs`, sorted by project-relative id (`node_modules` and `.d.ts` skipped).
fn collect_sources(root: &Path, dirs: &[&str]) -> Vec<(String, String)> {
    let mut sources: Vec<(String, String)> = Vec::new();
    for dir in dirs {
        let base = root.join(dir);
        if !base.is_dir() {
            continue;
        }
        let walker = walkdir::WalkDir::new(&base)
            .into_iter()
            .filter_entry(|e| e.file_name() != "node_modules");
        for entry in walker
            .filter_map(Result::ok)
            .filter(|e| e.file_type().is_file())
        {
            let path = entry.path();
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if is_declaration_file(name) {
                continue;
            }
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if !matches!(ext, "ts" | "tsx" | "js" | "jsx" | "mjs") {
                continue;
            }
            let rel = path.strip_prefix(root).map_or_else(|_| norm(path), norm);
            if let Ok(source) = fs::read_to_string(path) {
                sources.push((rel, source));
            }
        }
    }
    sources.sort_by(|a, b| a.0.cmp(&b.0));
    sources.dedup_by(|a, b| a.0 == b.0);
    sources
}

/// Per-module diagnostics plus boundary violations, minus hook violations of client leaves.
fn graph_diagnostics(modules: &[ModuleInfo]) -> Vec<Diagnostic> {
    // Import graph (relative + `@/` specifiers only) for the client-leaf refinement.
    let ids: BTreeSet<String> = modules.iter().map(|m| m.id.clone()).collect();
    let mut importers: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (index, module) in modules.iter().enumerate() {
        for spec in &module.imports {
            if let Some(target) = resolve_import(&module.id, spec, &ids) {
                importers.entry(target).or_default().push(index);
            }
        }
    }

    let mut diagnostics = Vec::new();
    for module in modules {
        diagnostics.extend(module.diagnostics.iter().cloned());
        let client_leaf = !is_app_entry(&module.id)
            && importers.get(&module.id).is_some_and(|list| {
                !list.is_empty()
                    && list
                        .iter()
                        .all(|&i| modules.get(i).is_some_and(|m| m.kind == ModuleKind::Client))
            });
        let mut violations = boundary_violations(module);
        if client_leaf {
            violations.retain(|d| {
                d.code != "hooks-in-server-component"
                    && d.code != "navigation-hooks-in-server-component"
            });
        }
        diagnostics.extend(violations);
    }
    diagnostics
}

/// Client references of `"use client"` modules and deduplicated server references.
fn graph_references(modules: &[ModuleInfo]) -> (Vec<ClientReference>, Vec<ServerReference>) {
    let mut client_manifest = Vec::new();
    let mut server_manifest: Vec<ServerReference> = Vec::new();
    for module in modules {
        if module.kind == ModuleKind::Client {
            let chunks = vec![chunk_name(&module.id)];
            let mut names: Vec<&str> = module
                .exports
                .iter()
                .map(String::as_str)
                .filter(|e| *e != "*")
                .collect();
            names.push("*");
            for export in names {
                client_manifest.push(ClientReference {
                    id: format!("{}#{export}", module.id),
                    module: module.id.clone(),
                    export_name: export.to_owned(),
                    chunks: chunks.clone(),
                    async_: false,
                });
            }
        }
        if module.kind == ModuleKind::ServerAction {
            for export in module.exports.iter().filter(|e| *e != "*") {
                server_manifest.push(ServerReference {
                    id: server_reference_id(&module.id, export, ""),
                    module: module.id.clone(),
                    export_name: export.clone(),
                    inline: false,
                    cache: false,
                });
            }
        }
        for action in &module.inline_server_actions {
            server_manifest.push(ServerReference {
                id: server_reference_id(&module.id, action, ""),
                module: module.id.clone(),
                export_name: action.clone(),
                inline: true,
                cache: false,
            });
        }
        for cached in &module.cache_functions {
            server_manifest.push(ServerReference {
                id: server_reference_id(&module.id, &cached.name, ""),
                module: module.id.clone(),
                export_name: cached.name.clone(),
                inline: false,
                cache: true,
            });
        }
    }
    let mut seen = BTreeSet::new();
    server_manifest.retain(|reference| seen.insert(reference.id.clone()));
    (client_manifest, server_manifest)
}

/// Scans `dirs` (relative to `root`) for `.ts/.tsx/.js/.jsx/.mjs` modules and builds the graph.
///
/// `node_modules` and `.d.ts` files are skipped. Client references are emitted for every export
/// of a `"use client"` module (plus a `*` namespace entry); server references cover the exports
/// of `"use server"` modules, hoisted inline actions and `"use cache"` functions. Hook violations
/// are suppressed for modules that are imported exclusively by Client modules (they become client
/// code at bundle time) unless they are App Router special files.
pub fn scan_module_graph(root: &Path, dirs: &[&str]) -> ModuleGraph {
    let modules: Vec<ModuleInfo> = collect_sources(root, dirs)
        .iter()
        .map(|(id, source)| analyze_module(source, id))
        .collect();
    let diagnostics = graph_diagnostics(&modules);
    let (client_manifest, server_manifest) = graph_references(&modules);
    ModuleGraph { root: norm(root), modules, client_manifest, server_manifest, diagnostics }
}

/// 40-hex server reference id: `hex(0x00 ++ sha256(salt + module + ':' + export)[..19])`.
///
/// Deterministic for a given (salt, module, export) triple so ids survive rebuilds; the leading
/// `00` byte mirrors the type prefix of Next.js action ids.
pub fn server_reference_id(module: &str, export_name: &str, salt: &str) -> String {
    let digest = Sha256::digest(format!("{salt}{module}:{export_name}").as_bytes());
    let mut out = String::with_capacity(40);
    out.push_str("00");
    for byte in digest.as_slice().iter().take(19) {
        let _ = write!(out, "{byte:02x}");
    }
    out
}

// ============================================================================
// Flight manifests
// ============================================================================

/// `react-client-manifest.json`-shaped JSON (`clientModules`).
pub fn client_manifest_json(graph: &ModuleGraph) -> Value {
    let mut client_modules = Map::new();
    let mut ssr: BTreeMap<String, Map<String, Value>> = BTreeMap::new();
    for reference in &graph.client_manifest {
        client_modules.insert(
            reference.id.clone(),
            json!({
                "id": reference.module,
                "name": reference.export_name,
                "chunks": reference.chunks,
                "async": reference.async_,
            }),
        );
        ssr.entry(reference.module.clone()).or_default().insert(
            reference.export_name.clone(),
            json!({
                "id": reference.module,
                "name": reference.export_name,
                "chunks": reference.chunks,
            }),
        );
    }
    let ssr_module_mapping: Map<String, Value> = ssr
        .into_iter()
        .map(|(module, exports)| (module, Value::Object(exports)))
        .collect();
    json!({
        "moduleLoading": { "prefix": "/_yolo/static/chunks/", "crossOrigin": null },
        "clientModules": client_modules,
        "ssrModuleMapping": ssr_module_mapping,
        "entryCSSFiles": {},
    })
}

/// `server-reference-manifest.json`-shaped JSON.
pub fn server_manifest_json(graph: &ModuleGraph) -> Value {
    let mut node = Map::new();
    for reference in &graph.server_manifest {
        node.insert(
            reference.id.clone(),
            json!({
                "workers": { "app/page": { "moduleId": reference.module, "async": false } },
                "layer": { "app/page": "action-browser" },
                "exportName": reference.export_name,
                "module": reference.module,
                "inline": reference.inline,
                "cache": reference.cache,
            }),
        );
    }
    json!({
        "node": node,
        "edge": {},
        "encryptionKey": "${env:NEXT_SERVER_ACTIONS_ENCRYPTION_KEY}",
    })
}

fn ts_string(value: &str) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| format!("\"{value}\""))
}

/// TypeScript module map shim (`__webpack_require__`-compatible) for `react-server-dom-webpack`.
///
/// Emits `clientModuleMap` (module id -> dynamic import), `serverActionMap` (reference id ->
/// loader + export name), `installWebpackShims()` which defines `globalThis.__webpack_require__`
/// and `globalThis.__webpack_chunk_load__` over a module cache, and `clientManifest` (the JSON of
/// [`client_manifest_json`]). Import paths are relative to the `.yolo/` output directory.
pub fn flight_bootstrap_ts(graph: &ModuleGraph) -> String {
    let mut out = String::new();
    out.push_str("// Generated by yolo-next: client module map for react-server-dom-webpack\n");
    out.push_str("// Do not edit; regenerate with `yolo-next rsc <project>`.\n\n");

    let mut client_modules: BTreeSet<&str> = BTreeSet::new();
    let mut chunk_map: BTreeMap<&str, &str> = BTreeMap::new();
    for reference in &graph.client_manifest {
        client_modules.insert(reference.module.as_str());
        for chunk in &reference.chunks {
            chunk_map.insert(chunk.as_str(), reference.module.as_str());
        }
    }

    out.push_str("export const clientModuleMap = {\n");
    for module in &client_modules {
        let _ = writeln!(
            out,
            "  {}: () => import({}),",
            ts_string(module),
            ts_string(&format!("../{module}"))
        );
    }
    out.push_str("} as const;\n\n");

    out.push_str("export const serverActionMap = {\n");
    for reference in &graph.server_manifest {
        let _ = writeln!(
            out,
            "  {}: {{ module: () => import({}), exportName: {}, inline: {}, cache: {} }},",
            ts_string(&reference.id),
            ts_string(&format!("../{}", reference.module)),
            ts_string(&reference.export_name),
            reference.inline,
            reference.cache
        );
    }
    out.push_str("} as const;\n\n");

    out.push_str("const chunkToModule: Record<string, string> = {\n");
    for (chunk, module) in &chunk_map {
        let _ = writeln!(out, "  {}: {},", ts_string(chunk), ts_string(module));
    }
    out.push_str("};\n\n");

    out.push_str(
        "const moduleCache = new Map<string, unknown>();\n\n\
         type WebpackGlobals = typeof globalThis & {\n  \
           __webpack_require__?: (id: string) => unknown;\n  \
           __webpack_chunk_load__?: (chunk: string) => Promise<unknown>;\n\
         };\n\n\
         /** Installs the `__webpack_require__` / `__webpack_chunk_load__` globals react-server-dom-webpack expects. */\n\
         export function installWebpackShims(): void {\n  \
           const g = globalThis as WebpackGlobals;\n  \
           g.__webpack_require__ = (id: string) => moduleCache.get(id);\n  \
           g.__webpack_chunk_load__ = async (chunk: string) => {\n    \
             const id = chunkToModule[chunk] ?? chunk;\n    \
             const loader = (clientModuleMap as Record<string, (() => Promise<unknown>) | undefined>)[id];\n    \
             if (!loader) {\n      \
               throw new Error(`yolo-next: unknown client chunk ${chunk}`);\n    \
             }\n    \
             if (!moduleCache.has(id)) {\n      \
               const mod = await loader();\n      \
               moduleCache.set(id, mod);\n    \
             }\n    \
             return moduleCache.get(id);\n  \
           };\n\
         }\n\n\
         /** Resolves a server reference id to its exported function (actions and `\"use cache\"` functions). */\n\
         export async function loadServerReference(id: string): Promise<unknown> {\n  \
           const entry = (serverActionMap as Record<string, { module: () => Promise<unknown>; exportName: string } | undefined>)[id];\n  \
           if (!entry) {\n    \
             throw new Error(`yolo-next: unknown server reference ${id}`);\n  \
           }\n  \
           const mod = (await entry.module()) as Record<string, unknown>;\n  \
           return mod[entry.exportName];\n\
         }\n\n",
    );

    let manifest = serde_json::to_string_pretty(&client_manifest_json(graph)).unwrap_or_default();
    let _ = writeln!(out, "export const clientManifest = {manifest} as const;");
    out
}

#[cfg(test)]
#[allow(clippy::assert_is_empty)] // `assert!(x.is_empty())` reads better than typed empty arrays
mod tests {
    use super::*;

    const ACTIONS_TS: &str = r#""use server";

import { revalidatePath, revalidateTag, updateTag } from "next/cache";
import { cookies, headers } from "next/headers";
import { redirect } from "next/navigation";

export async function createPost(prevState: { message: string }, formData: FormData) {
  const title = String(formData.get("title") ?? "");
  const h = await headers();
  const c = await cookies();
  updateTag("posts");
  revalidatePath("/");
  return { message: `created ${title} from ${h.get("user-agent") ?? "?"}` };
}

export async function deletePost(slug: string) {
  revalidateTag("posts", "max");
  redirect("/");
}
"#;

    const DASHBOARD_TSX: &str = r#"export const maxDuration = 30;

export default async function Dashboard() {
  async function refreshNow() {
    "use server";
    const { refresh } = await import("next/cache");
    refresh();
  }
  return (
    <form action={refreshNow}>
      <button>Refresh</button>
    </form>
  );
}
"#;

    const DATA_TS: &str = r#"import { cacheLife, cacheTag, unstable_cache } from "next/cache";

export interface Post {
  slug: string;
  title: string;
}

export async function getPosts(): Promise<Post[]> {
  "use cache";
  cacheLife("blog");
  cacheTag("posts");
  return [{ slug: "hello-yolo", title: "Hello Yolo" }];
}

export async function getPost(slug: string) {
  "use cache";
  cacheLife("hours");
  cacheTag("posts", `post:${slug}`);
  const posts = await getPosts();
  return posts.find((p) => p.slug === slug) ?? null;
}

export const getStats = unstable_cache(async () => ({ visits: 42 }), ["stats"], { revalidate: 60 });
"#;

    const COUNTER_TSX: &str = r#""use client";

import { useActionState, useOptimistic, useState, useTransition } from "react";
import { useRouter, usePathname, useSearchParams } from "next/navigation";
import { createPost } from "../actions";

export function Counter({ initial }: { initial: number }) {
  const [count, setCount] = useState(initial);
  const [state, formAction, pending] = useActionState(createPost, { message: "" });
  const [optimistic, addOptimistic] = useOptimistic(count, (c: number, delta: number) => c + delta);
  const [isPending, startTransition] = useTransition();
  const router = useRouter();
  const pathname = usePathname();
  const params = useSearchParams();
  return (
    <form action={formAction}>
      <button type="button" onClick={() => startTransition(() => { addOptimistic(1); setCount((c) => c + 1); router.refresh(); })}>
        {optimistic}
      </button>
      <p>{state.message} {pathname} {params.get("q")} {isPending ? "pending" : ""}</p>
    </form>
  );
}
"#;

    #[test]
    fn actions_module_is_server_action_with_exports() {
        let module = analyze_module(ACTIONS_TS, "app/actions.ts");
        assert_eq!(module.kind, ModuleKind::ServerAction);
        assert_eq!(module.directive, ModuleDirective::UseServer);
        assert_eq!(module.exports, vec!["createPost", "deletePost"]);
        assert!(module.imports.iter().any(|i| i == "next/headers"));
        assert!(module.next_imports.iter().any(|i| i == "next/cache"));
        assert!(module.inline_server_actions.is_empty());
        assert!(boundary_violations(&module).is_empty());
    }

    #[test]
    fn dashboard_page_has_inline_action_and_dynamic_import() {
        let module = analyze_module(DASHBOARD_TSX, "app/dashboard/page.tsx");
        assert_eq!(module.kind, ModuleKind::Server);
        assert_eq!(module.inline_server_actions, vec!["refreshNow"]);
        assert_eq!(module.exports, vec!["maxDuration", "default"]);
        assert!(module.imports.iter().any(|i| i == "next/cache"));
        assert!(module.diagnostics.is_empty());
    }

    #[test]
    fn non_async_inline_action_is_an_error() {
        let source = "export default function Page() {\n  function act() {\n    \"use server\";\n  }\n  return <form action={act} />;\n}\n";
        let module = analyze_module(source, "app/x/page.tsx");
        assert_eq!(module.inline_server_actions, vec!["act"]);
        assert!(
            module
                .diagnostics
                .iter()
                .any(|d| d.code == "server-action-not-async")
        );
    }

    #[test]
    fn anonymous_inline_actions_get_hoisted_names() {
        let source = "export default function Page() {\n  return <form action={async () => { \"use server\"; }} />;\n}\n";
        let module = analyze_module(source, "app/y/page.tsx");
        assert_eq!(module.inline_server_actions, vec!["$$RSC_SERVER_ACTION_0"]);
    }

    #[test]
    fn data_module_collects_cache_functions() {
        let module = analyze_module(DATA_TS, "app/lib/data.ts");
        assert_eq!(module.kind, ModuleKind::Shared);
        assert_eq!(module.cache_functions.len(), 2);
        let posts = &module.cache_functions[0];
        assert_eq!(posts.name, "getPosts");
        assert_eq!(posts.life.as_deref(), Some("blog"));
        assert_eq!(posts.tags, vec!["posts"]);
        assert_eq!(posts.variant, "");
        let post = &module.cache_functions[1];
        assert_eq!(post.name, "getPost");
        assert_eq!(post.life.as_deref(), Some("hours"));
        assert_eq!(post.tags, vec!["posts", "post:*"]);
        assert_eq!(module.exports, vec!["getPosts", "getPost", "getStats"]);
    }

    #[test]
    fn cache_variants_and_inline_life_objects() {
        let source = "export async function stats() {\n  \"use cache: private\";\n  cacheLife({ stale: 60, revalidate: 300 });\n  return 1;\n}\nexport default async function Page() {\n  \"use cache: remote\";\n  return <p />;\n}\n";
        let module = analyze_module(source, "app/stats/page.tsx");
        assert_eq!(module.cache_functions.len(), 2);
        assert_eq!(module.cache_functions[0].variant, "private");
        let life: Value = module.cache_functions[0]
            .life
            .as_deref()
            .and_then(|text| serde_json::from_str(text).ok())
            .expect("inline cacheLife object as JSON");
        assert_eq!(life, json!({ "stale": 60, "revalidate": 300 }));
        assert_eq!(module.cache_functions[1].name, "default");
        assert_eq!(module.cache_functions[1].variant, "remote");
    }

    #[test]
    fn file_level_use_cache_and_non_async_errors() {
        let source = "\"use cache\";\nexport function sync() {\n  \"use cache\";\n  return 1;\n}\n";
        let module = analyze_module(source, "app/lib/cached.ts");
        assert_eq!(module.kind, ModuleKind::Cache);
        assert_eq!(module.directive, ModuleDirective::UseCache);
        assert!(
            module
                .diagnostics
                .iter()
                .any(|d| d.code == "use-cache-not-async")
        );
    }

    #[test]
    fn counter_is_client_with_hooks_and_navigation() {
        let module = analyze_module(COUNTER_TSX, "app/components/Counter.tsx");
        assert_eq!(module.kind, ModuleKind::Client);
        for hook in [
            "useState",
            "useActionState",
            "useOptimistic",
            "useTransition",
            "useRouter",
        ] {
            assert!(
                module.hooks.iter().any(|h| h == hook),
                "missing hook {hook}"
            );
        }
        assert!(module.next_imports.iter().any(|i| i == "next/navigation"));
        assert_eq!(module.exports, vec!["Counter"]);
        assert!(boundary_violations(&module).is_empty());
    }

    #[test]
    fn client_importing_next_headers_is_a_violation() {
        let source = "\"use client\";\nimport { headers } from \"next/headers\";\nimport fs from \"node:fs\";\nexport function Bad() { return <p>{String(headers)}{String(fs)}</p>; }\n";
        let module = analyze_module(source, "app/components/Bad.tsx");
        let violations = boundary_violations(&module);
        let codes: Vec<&str> = violations.iter().map(|d| d.code.as_str()).collect();
        assert_eq!(
            codes,
            vec!["client-imports-server-only", "client-imports-server-only"]
        );
        assert_eq!(
            violations[0].file.as_deref(),
            Some("app/components/Bad.tsx")
        );
    }

    #[test]
    fn hooks_in_server_component_and_inline_action_in_client() {
        let server = analyze_module(
            "import { useState } from \"react\";\nimport { useRouter } from \"next/navigation\";\nexport default function Page() { const [a] = useState(0); const r = useRouter(); return <p>{a}{String(r)}</p>; }\n",
            "app/bad/page.tsx",
        );
        let codes: Vec<String> = boundary_violations(&server)
            .into_iter()
            .map(|d| d.code)
            .collect();
        assert!(codes.contains(&String::from("hooks-in-server-component")));
        assert!(codes.contains(&String::from("navigation-hooks-in-server-component")));

        let client = analyze_module(
            "\"use client\";\nexport function C() { async function a() { \"use server\"; } return <form action={a} />; }\n",
            "app/components/C.tsx",
        );
        let codes: Vec<String> = boundary_violations(&client)
            .into_iter()
            .map(|d| d.code)
            .collect();
        assert_eq!(codes, vec!["inline-action-in-client"]);

        let cached = analyze_module(
            "import { headers } from \"next/headers\";\nexport async function get() { \"use cache\"; return String(headers); }\n",
            "app/lib/c.ts",
        );
        let codes: Vec<String> = boundary_violations(&cached)
            .into_iter()
            .map(|d| d.code)
            .collect();
        assert_eq!(codes, vec!["next-request-in-use-cache"]);
    }

    #[test]
    fn exports_cover_specifiers_reexports_and_destructuring() {
        let source = "const a = 1, b = 2;\nexport { a, b as bee };\nexport * from \"./x\";\nexport * as ns from \"./y\";\nexport { z } from \"./z\";\nexport const { p, q: [r] } = obj;\nexport type T = string;\nexport enum E { A }\n";
        let module = analyze_module(source, "lib/exports.ts");
        assert_eq!(
            module.exports,
            vec!["a", "bee", "*", "ns", "z", "p", "r", "E"]
        );
        assert_eq!(module.imports, vec!["./x", "./y", "./z"]);
    }

    #[test]
    fn server_reference_ids_are_40_hex_and_deterministic() {
        let a = server_reference_id("app/actions.ts", "createPost", "");
        let b = server_reference_id("app/actions.ts", "createPost", "");
        let c = server_reference_id("app/actions.ts", "deletePost", "");
        assert_eq!(a.len(), 40);
        assert!(a.starts_with("00"));
        assert!(
            a.chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
        );
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_ne!(
            a,
            server_reference_id("app/actions.ts", "createPost", "salt")
        );
    }

    fn sample_graph() -> ModuleGraph {
        let counter = analyze_module(COUNTER_TSX, "app/components/Counter.tsx");
        let actions = analyze_module(ACTIONS_TS, "app/actions.ts");
        let dashboard = analyze_module(DASHBOARD_TSX, "app/dashboard/page.tsx");
        let data = analyze_module(DATA_TS, "app/lib/data.ts");
        let client_manifest = vec![
            ClientReference {
                id: String::from("app/components/Counter.tsx#Counter"),
                module: counter.id.clone(),
                export_name: String::from("Counter"),
                chunks: vec![chunk_name(&counter.id)],
                async_: false,
            },
            ClientReference {
                id: String::from("app/components/Counter.tsx#*"),
                module: counter.id.clone(),
                export_name: String::from("*"),
                chunks: vec![chunk_name(&counter.id)],
                async_: false,
            },
        ];
        let server_manifest = vec![
            ServerReference {
                id: server_reference_id(&actions.id, "createPost", ""),
                module: actions.id.clone(),
                export_name: String::from("createPost"),
                inline: false,
                cache: false,
            },
            ServerReference {
                id: server_reference_id(&dashboard.id, "refreshNow", ""),
                module: dashboard.id.clone(),
                export_name: String::from("refreshNow"),
                inline: true,
                cache: false,
            },
            ServerReference {
                id: server_reference_id(&data.id, "getPosts", ""),
                module: data.id.clone(),
                export_name: String::from("getPosts"),
                inline: false,
                cache: true,
            },
        ];
        ModuleGraph {
            modules: vec![counter, actions, dashboard, data],
            client_manifest,
            server_manifest,
            ..ModuleGraph::default()
        }
    }

    #[test]
    fn manifests_have_the_flight_shape() {
        let graph = sample_graph();
        let client = client_manifest_json(&graph);
        assert_eq!(client["moduleLoading"]["prefix"], "/_yolo/static/chunks/");
        let entry = &client["clientModules"]["app/components/Counter.tsx#Counter"];
        assert_eq!(entry["id"], "app/components/Counter.tsx");
        assert_eq!(entry["name"], "Counter");
        assert_eq!(
            entry["chunks"][0],
            "static/chunks/app_components_Counter.js"
        );
        assert_eq!(entry["async"], false);
        assert_eq!(
            client["ssrModuleMapping"]["app/components/Counter.tsx"]["Counter"]["name"],
            "Counter"
        );
        assert!(client["entryCSSFiles"].is_object());

        let server = server_manifest_json(&graph);
        let id = server_reference_id("app/actions.ts", "createPost", "");
        let node = &server["node"][&id];
        assert_eq!(node["workers"]["app/page"]["moduleId"], "app/actions.ts");
        assert_eq!(node["layer"]["app/page"], "action-browser");
        assert_eq!(node["exportName"], "createPost");
        assert_eq!(node["inline"], false);
        let inline_id = server_reference_id("app/dashboard/page.tsx", "refreshNow", "");
        assert_eq!(server["node"][&inline_id]["inline"], true);
        let cache_id = server_reference_id("app/lib/data.ts", "getPosts", "");
        assert_eq!(server["node"][&cache_id]["cache"], true);
        assert_eq!(
            server["encryptionKey"],
            "${env:NEXT_SERVER_ACTIONS_ENCRYPTION_KEY}"
        );
        assert!(server["edge"].is_object());
    }

    #[test]
    fn bootstrap_ts_defines_webpack_shims() {
        let ts = flight_bootstrap_ts(&sample_graph());
        assert!(ts.starts_with("// Generated by yolo-next"));
        assert!(ts.contains("__webpack_require__"));
        assert!(ts.contains("__webpack_chunk_load__"));
        assert!(ts.contains(
            "\"app/components/Counter.tsx\": () => import(\"../app/components/Counter.tsx\")"
        ));
        assert!(ts.contains("export const serverActionMap"));
        assert!(ts.contains("export const clientManifest"));
        assert!(ts.contains("exportName: \"createPost\""));
    }

    #[test]
    fn scan_graph_resolves_client_leaves_and_builds_references() {
        let dir = std::env::temp_dir().join(format!(
            "yolo-next-rsc-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or_default()
        ));
        let app = dir.join("app");
        fs::create_dir_all(app.join("components")).expect("create temp app dir");
        fs::create_dir_all(app.join("node_modules")).expect("create node_modules");
        fs::write(
            app.join("node_modules").join("skip.tsx"),
            "\"use client\";\nexport const S = 1;\n",
        )
        .expect("write skipped module");
        fs::write(app.join("types.d.ts"), "export type X = 1;\n").expect("write d.ts");
        fs::write(app.join("components").join("Counter.tsx"), COUNTER_TSX).expect("write Counter");
        fs::write(
            app.join("components").join("Leaf.tsx"),
            "import { useState } from \"react\";\nexport function Leaf() { const [n] = useState(0); return <i>{n}</i>; }\n",
        )
        .expect("write Leaf");
        fs::write(
            app.join("components").join("Shell.tsx"),
            "\"use client\";\nimport { Leaf } from \"./Leaf\";\nexport default function Shell() { return <Leaf />; }\n",
        )
        .expect("write Shell");
        fs::write(app.join("actions.ts"), ACTIONS_TS).expect("write actions");
        fs::write(
            app.join("page.tsx"),
            "import { Counter } from \"@/app/components/Counter\";\nimport Shell from \"./components/Shell\";\nexport default function Page() { return <><Counter initial={1} /><Shell /></>; }\n",
        )
        .expect("write page");

        let graph = scan_module_graph(&dir, &["app", "src/app"]);
        let _ = fs::remove_dir_all(&dir);

        let ids: Vec<&str> = graph.modules.iter().map(|m| m.id.as_str()).collect();
        assert_eq!(
            ids,
            vec![
                "app/actions.ts",
                "app/components/Counter.tsx",
                "app/components/Leaf.tsx",
                "app/components/Shell.tsx",
                "app/page.tsx"
            ]
        );
        assert!(
            graph
                .client_manifest
                .iter()
                .any(|r| r.id == "app/components/Counter.tsx#Counter")
        );
        assert!(
            graph
                .client_manifest
                .iter()
                .any(|r| r.id == "app/components/Shell.tsx#*")
        );
        assert!(
            graph
                .server_manifest
                .iter()
                .any(|r| r.module == "app/actions.ts"
                    && r.export_name == "createPost"
                    && !r.inline)
        );
        // Leaf.tsx uses hooks without a directive but is only imported by a Client module.
        assert!(
            !graph
                .diagnostics
                .iter()
                .any(|d| d.code == "hooks-in-server-component"),
            "{:?}",
            graph.diagnostics
        );
    }

    #[test]
    fn fixture_graph_when_present() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../apps/next-fixture");
        if !root.join("app").is_dir() {
            return;
        }
        let graph = scan_module_graph(&root, &["app"]);
        let find = |id: &str| graph.modules.iter().find(|m| m.id == id);
        let actions = find("app/actions.ts").expect("actions module");
        assert_eq!(actions.kind, ModuleKind::ServerAction);
        assert_eq!(actions.exports, vec!["createPost", "deletePost"]);
        let dashboard = find("app/dashboard/page.tsx").expect("dashboard page");
        assert_eq!(dashboard.inline_server_actions, vec!["refreshNow"]);
        let data = find("app/lib/data.ts").expect("data module");
        assert_eq!(data.cache_functions.len(), 2);
        assert_eq!(data.cache_functions[1].tags, vec!["posts", "post:*"]);
        let counter = find("app/components/Counter.tsx").expect("counter");
        assert_eq!(counter.kind, ModuleKind::Client);
        assert!(
            graph
                .client_manifest
                .iter()
                .any(|r| r.id == "app/components/Counter.tsx#Counter")
        );
        assert!(graph.server_manifest.iter().all(|r| r.id.len() == 40));
        assert!(
            graph
                .server_manifest
                .iter()
                .any(|r| r.export_name == "refreshNow" && r.inline)
        );
        assert!(
            graph
                .server_manifest
                .iter()
                .any(|r| r.export_name == "getPost" && r.cache)
        );
    }
}
