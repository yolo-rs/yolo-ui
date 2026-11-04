// SPDX-License-Identifier: Apache-2.0
//! OXC React 19 Component, Hook & Auto-Memoization Analyzer (`oxc_parser` 0.155).
//!
//! Inspects `.tsx` / `.jsx` source files at native speed to detect:
//! - `"use client"`, `"use server"`, and `"use dom"` (Expo DOM Component) directives
//! - Exported React component names and prop identifiers
//! - React 19 hooks (`useState`, `useReducer`, `useMemo`, `useCallback`, `useEffect`,
//!   `useLayoutEffect`, `useRef`, `useContext`, `useTransition`, `useOptimistic`, `useActionState`, `use`)
//! - React Compiler automatic memoization candidate boundaries

use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use oxc_parser::Parser;
use oxc_span::SourceType;
use serde::{Deserialize, Serialize};

/// Static analysis report for a React 19 / React Native `.tsx` module.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReactComponentAnalysis {
    /// Primary component name found in the module.
    pub component_name: String,
    /// True if `"use client"` directive is present at the top of the file.
    pub is_use_client: bool,
    /// True if `"use server"` directive is present at the top of the file.
    pub is_use_server: bool,
    /// True if `"use dom"` (Expo Universal DOM component) directive is present.
    pub is_expo_use_dom: bool,
    /// Destructured or typed prop names accepted by the primary component.
    pub props: Vec<String>,
    /// React hooks invoked inside the module (`useState`, `useMemo`, `useEffect`, `use`, etc.).
    pub hooks_used: Vec<String>,
    /// JSX element tags rendered by the component (`div`, `View`, `Text`, `Button`, `Card`, etc.).
    pub jsx_elements: Vec<String>,
    /// Whether the component is a pure/stateful candidate for React Compiler automatic memoization.
    pub react_compiler_memoizable: bool,
}

fn is_react_hook_name(name: &str) -> bool {
    name == "use"
        || (name.starts_with("use")
            && name
                .chars()
                .nth(3)
                .is_some_and(|c| c.is_ascii_uppercase()))
}

fn collect_arrow_body(
    body: &ArrowFunctionBody<'_>,
    hooks: &mut Vec<String>,
    jsx_tags: &mut Vec<String>,
) {
    match body {
        ArrowFunctionBody::FunctionBody(fb) => {
            for stmt in &fb.statements {
                collect_stmt(stmt, hooks, jsx_tags);
            }
        }
        other => {
            if let Some(expr) = other.as_expression() {
                collect_expr(expr, hooks, jsx_tags);
            }
        }
    }
}

fn collect_expr(
    expr: &Expression<'_>,
    hooks: &mut Vec<String>,
    jsx_tags: &mut Vec<String>,
) {
    match expr {
        Expression::CallExpression(call) => {
            match &call.callee {
                Expression::Identifier(id) => {
                    let name = id.name.as_str();
                    if is_react_hook_name(name) && !hooks.iter().any(|h| h == name) {
                        hooks.push(name.to_owned());
                    }
                }
                Expression::StaticMemberExpression(member) => {
                    let prop_name = member.property.name.as_str();
                    if is_react_hook_name(prop_name) && !hooks.iter().any(|h| h == prop_name) {
                        hooks.push(prop_name.to_owned());
                    }
                }
                _ => {}
            }
            for arg in &call.arguments {
                if let Some(arg_expr) = arg.as_expression() {
                    collect_expr(arg_expr, hooks, jsx_tags);
                }
            }
        }
        Expression::JSXElement(el) => {
            let tag = jsx_element_name(&el.opening_element.name);
            if !tag.is_empty() && !jsx_tags.iter().any(|t| t == &tag) {
                jsx_tags.push(tag);
            }
            for attr in &el.opening_element.attributes {
                if let JSXAttributeItem::Attribute(a) = attr
                    && let Some(JSXAttributeValue::ExpressionContainer(c)) = &a.value
                    && let Some(inner) = c.expression.as_expression()
                {
                    collect_expr(inner, hooks, jsx_tags);
                }
            }
            for child in &el.children {
                collect_jsx_child(child, hooks, jsx_tags);
            }
        }
        Expression::JSXFragment(frag) => {
            for child in &frag.children {
                collect_jsx_child(child, hooks, jsx_tags);
            }
        }
        Expression::ParenthesizedExpression(p) => collect_expr(&p.expression, hooks, jsx_tags),
        Expression::ArrowFunctionExpression(arrow) => {
            collect_arrow_body(&arrow.body, hooks, jsx_tags);
        }
        Expression::ConditionalExpression(cond) => {
            collect_expr(&cond.test, hooks, jsx_tags);
            collect_expr(&cond.consequent, hooks, jsx_tags);
            collect_expr(&cond.alternate, hooks, jsx_tags);
        }
        Expression::LogicalExpression(log) => {
            collect_expr(&log.left, hooks, jsx_tags);
            collect_expr(&log.right, hooks, jsx_tags);
        }
        _ => {}
    }
}

fn collect_jsx_child(
    child: &JSXChild<'_>,
    hooks: &mut Vec<String>,
    jsx_tags: &mut Vec<String>,
) {
    match child {
        JSXChild::Element(el) => {
            let tag = jsx_element_name(&el.opening_element.name);
            if !tag.is_empty() && !jsx_tags.iter().any(|t| t == &tag) {
                jsx_tags.push(tag);
            }
            for c in &el.children {
                collect_jsx_child(c, hooks, jsx_tags);
            }
        }
        JSXChild::Fragment(frag) => {
            for c in &frag.children {
                collect_jsx_child(c, hooks, jsx_tags);
            }
        }
        JSXChild::ExpressionContainer(container) => {
            if let Some(expr) = container.expression.as_expression() {
                collect_expr(expr, hooks, jsx_tags);
            }
        }
        _ => {}
    }
}

fn jsx_element_name(name: &JSXElementName<'_>) -> String {
    match name {
        JSXElementName::Identifier(id) => id.name.as_str().to_owned(),
        JSXElementName::IdentifierReference(id) => id.name.as_str().to_owned(),
        JSXElementName::NamespacedName(ns) => {
            format!("{}:{}", ns.namespace.name, ns.name.name)
        }
        JSXElementName::MemberExpression(mem) => member_name(mem),
        JSXElementName::ThisExpression(_) => String::from("this"),
    }
}

fn member_name(mem: &JSXMemberExpression<'_>) -> String {
    let obj = match &mem.object {
        JSXMemberExpressionObject::IdentifierReference(id) => id.name.as_str().to_owned(),
        JSXMemberExpressionObject::MemberExpression(inner) => member_name(inner),
        JSXMemberExpressionObject::ThisExpression(_) => String::from("this"),
    };
    format!("{obj}.{}", mem.property.name)
}

fn collect_stmt(
    stmt: &Statement<'_>,
    hooks: &mut Vec<String>,
    jsx_tags: &mut Vec<String>,
) {
    match stmt {
        Statement::VariableDeclaration(decl) => {
            for d in &decl.declarations {
                if let Some(init) = &d.init {
                    collect_expr(init, hooks, jsx_tags);
                }
            }
        }
        Statement::ExpressionStatement(es) => {
            collect_expr(&es.expression, hooks, jsx_tags);
        }
        Statement::ReturnStatement(ret) => {
            if let Some(arg) = &ret.argument {
                collect_expr(arg, hooks, jsx_tags);
            }
        }
        Statement::BlockStatement(block) => {
            for s in &block.body {
                collect_stmt(s, hooks, jsx_tags);
            }
        }
        Statement::IfStatement(if_stmt) => {
            collect_expr(&if_stmt.test, hooks, jsx_tags);
            collect_stmt(&if_stmt.consequent, hooks, jsx_tags);
            if let Some(alt) = &if_stmt.alternate {
                collect_stmt(alt, hooks, jsx_tags);
            }
        }
        _ => {}
    }
}

fn extract_param_props(params: &FormalParameters<'_>, props: &mut Vec<String>) {
    if let Some(first) = params.items.first() {
        match &first.pattern {
            BindingPattern::ObjectPattern(obj) => {
                for prop in &obj.properties {
                    if let PropertyKey::StaticIdentifier(id) = &prop.key {
                        let name = id.name.as_str().to_owned();
                        if !props.contains(&name) {
                            props.push(name);
                        }
                    }
                }
            }
            BindingPattern::BindingIdentifier(id) => {
                let name = id.name.as_str().to_owned();
                if !props.contains(&name) {
                    props.push(name);
                }
            }
            _ => {}
        }
    }
}

/// Parses and analyzes a React 19 / React Native `.tsx` component using `oxc_parser`.
pub fn analyze_react_component(source: &str) -> Result<ReactComponentAnalysis, Vec<String>> {
    let allocator = Allocator::default();
    let source_type = SourceType::tsx();
    let parsed = Parser::new(&allocator, source, source_type).parse();

    if !parsed.diagnostics.is_empty() {
        return Err(parsed.diagnostics.iter().map(|e| e.to_string()).collect());
    }

    let mut is_use_client = false;
    let mut is_use_server = false;
    let mut is_expo_use_dom = false;

    for directive in &parsed.program.directives {
        match directive.directive.as_str() {
            "use client" => is_use_client = true,
            "use server" => is_use_server = true,
            "use dom" => is_expo_use_dom = true,
            _ => {}
        }
    }

    let mut component_name = String::from("AnonymousComponent");
    let mut props = Vec::new();
    let mut hooks_used = Vec::new();
    let mut jsx_elements = Vec::new();

    for stmt in &parsed.program.body {
        match stmt {
            Statement::FunctionDeclaration(func) => {
                if let Some(id) = &func.id {
                    let name = id.name.as_str();
                    if name.starts_with(|c: char| c.is_ascii_uppercase()) {
                        component_name = name.to_owned();
                        extract_param_props(&func.params, &mut props);
                    }
                }
                if let Some(body) = &func.body {
                    for s in &body.statements {
                        collect_stmt(s, &mut hooks_used, &mut jsx_elements);
                    }
                }
            }
            Statement::ExportDeclaration(export) => match &export.declaration {
                Declaration::FunctionDeclaration(func) => {
                    if let Some(id) = &func.id {
                        let name = id.name.as_str();
                        if name.starts_with(|c: char| c.is_ascii_uppercase()) {
                            component_name = name.to_owned();
                            extract_param_props(&func.params, &mut props);
                        }
                    }
                    if let Some(body) = &func.body {
                        for s in &body.statements {
                            collect_stmt(s, &mut hooks_used, &mut jsx_elements);
                        }
                    }
                }
                Declaration::VariableDeclaration(vdecl) => {
                    for d in &vdecl.declarations {
                        if let BindingPattern::BindingIdentifier(id) = &d.id {
                            let name = id.name.as_str();
                            if name.starts_with(|c: char| c.is_ascii_uppercase()) {
                                component_name = name.to_owned();
                            }
                        }
                        if let Some(Expression::ArrowFunctionExpression(arrow)) = &d.init {
                            extract_param_props(&arrow.params, &mut props);
                            collect_arrow_body(&arrow.body, &mut hooks_used, &mut jsx_elements);
                        } else if let Some(init) = &d.init {
                            collect_expr(init, &mut hooks_used, &mut jsx_elements);
                        }
                    }
                }
                _ => {}
            },
            Statement::ExportDefaultDeclaration(export_def) => {
                if let ExportDefaultDeclarationKind::FunctionDeclaration(func) =
                    &export_def.declaration
                {
                    if let Some(id) = &func.id {
                        component_name = id.name.as_str().to_owned();
                    }
                    extract_param_props(&func.params, &mut props);
                    if let Some(body) = &func.body {
                        for s in &body.statements {
                            collect_stmt(s, &mut hooks_used, &mut jsx_elements);
                        }
                    }
                }
            }
            _ => {
                collect_stmt(stmt, &mut hooks_used, &mut jsx_elements);
            }
        }
    }

    let react_compiler_memoizable = !jsx_elements.is_empty();

    Ok(ReactComponentAnalysis {
        component_name,
        is_use_client,
        is_use_server,
        is_expo_use_dom,
        props,
        hooks_used,
        jsx_elements,
        react_compiler_memoizable,
    })
}
