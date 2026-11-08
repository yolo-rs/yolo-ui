// SPDX-License-Identifier: Apache-2.0
//! Static evaluation of `next.config.(js|mjs|ts)` with OXC and conversion to [`YoloWebConfig`].
//!
//! Supported shapes: `export default nextConfig` (resolved through top-level `const` bindings,
//! `satisfies NextConfig`, `as const`), `export default {...}`, `module.exports = {...}`,
//! `export default defineConfig({...})`, plugin wrappers (`withMDX(withSentryConfig({...}))`,
//! recorded in [`NextConfig::wrappers`]) and phase functions whose body returns an object
//! literal. Function-valued keys (`redirects()`, `rewrites()`, `headers()`) are evaluated when
//! they return a literal; otherwise the `*_dynamic` flags are set and the function is awaited at
//! startup by `@yolo-rs/next`.
//!
//! `process.env.X` evaluates to the marker string `${env:X}` (`${env:X|default}` for
//! `process.env.X ?? "default"`); [`yolo_config_ts`] turns the markers back into expressions.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use oxc_allocator::Allocator;
use oxc_ast::ast::{
    self, Argument, ArrayExpressionElement, ArrowFunctionBody, AssignmentTarget, BinaryOperator,
    BindingPattern, Declaration, ExportDefaultDeclarationKind, Expression, FunctionBody,
    LogicalOperator, ModuleExportName, ObjectPropertyKind, PropertyKey, Statement, UnaryOperator,
};
use oxc_parser::Parser;
use oxc_span::{GetSpan, SourceType};
use serde_json::{Map, Number, Value};

use crate::types::{
    CacheLifeProfile, ConfigMigrationReport, Diagnostic, FeatureStatus, HeaderKv, HeaderRule,
    I18nConfig, ImagesConfig, LocalPattern, NextConfig, OutputMode, RemotePattern, RouteCondition,
    RouteRule, Support, TurbopackConfig, TurbopackLoader, TurbopackRule, YoloReactConfig,
    YoloServerConfig, YoloThemeConfig, YoloWebConfig,
};

/// JSON marker standing in for a function-valued key whose body could not be evaluated
/// statically (`webpack(config) { ... }`, `async redirects() { return fetch(...) }`).
pub const FUNCTION_MARKER: &str = "${function}";

/// Default `pageExtensions`.
const DEFAULT_PAGE_EXTENSIONS: [&str; 4] = ["tsx", "ts", "jsx", "js"];

/// Keys removed from `NextConfig` (Next.js 15/16) that are reported and dropped.
const REMOVED_TOP_LEVEL: [&str; 9] = [
    "eslint",
    "amp",
    "serverRuntimeConfig",
    "publicRuntimeConfig",
    "target",
    "future",
    "swcMinify",
    "analyticsId",
    "optimizeFonts",
];

const MAX_DEPTH: usize = 32;

/// Finds `next.config.{ts,mts,mjs,js}` under `root` (first match in that order).
pub fn find_next_config(root: &Path) -> Option<PathBuf> {
    [
        "next.config.ts",
        "next.config.mts",
        "next.config.mjs",
        "next.config.js",
    ]
    .iter()
    .map(|name| root.join(name))
    .find(|path| path.is_file())
}

fn source_type_for(file_name: &str) -> SourceType {
    let ext = file_name.rsplit('.').next().unwrap_or("ts");
    match ext {
        "js" | "mjs" => SourceType::mjs(),
        "cjs" => SourceType::cjs(),
        "tsx" => SourceType::tsx(),
        _ => SourceType::ts(),
    }
}

/// Owned Rust string of an OXC JS string (lossy for lone surrogates). A macro because the
/// `JSStr` type lives in `oxc_str`, which is not a direct dependency.
macro_rules! js_string {
    ($value:expr) => {{
        let value = $value;
        value
            .as_str()
            .map_or_else(|| value.display().to_string(), str::to_owned)
    }};
}

fn number_value(value: f64) -> Value {
    if value.is_finite() && value.fract() == 0.0 && value.abs() < 9.007_199_254_740_992e15 {
        Value::from(value as i64)
    } else {
        Number::from_f64(value).map_or(Value::Null, Value::Number)
    }
}

fn is_env_marker(value: &Value) -> bool {
    value.as_str().is_some_and(|s| s.starts_with("${env:"))
}

/// JavaScript truthiness of a static value; `None` when undecidable (env markers).
fn truthy(value: &Value) -> Option<bool> {
    match value {
        Value::Null => Some(false),
        Value::Bool(b) => Some(*b),
        Value::Number(n) => Some(n.as_f64().is_some_and(|f| f != 0.0)),
        Value::String(s) if s.starts_with("${env:") => None,
        Value::String(s) => Some(!s.is_empty()),
        Value::Array(_) | Value::Object(_) => Some(true),
    }
}

fn scalar_text(value: &Value) -> Option<String> {
    match value {
        Value::String(s) if !s.starts_with("${env:") => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

fn property_key_name(key: &PropertyKey<'_>) -> Option<String> {
    match key {
        PropertyKey::StaticIdentifier(id) => Some(id.name.to_string()),
        PropertyKey::StringLiteral(lit) => Some(js_string!(lit.value)),
        PropertyKey::NumericLiteral(lit) => Some(
            lit.raw
                .as_ref()
                .map_or_else(|| lit.value.to_string(), ToString::to_string),
        ),
        PropertyKey::TemplateLiteral(tpl) if tpl.expressions.is_empty() => {
            Some(tpl.quasis.iter().map(|q| q.value.raw.to_string()).collect())
        }
        _ => None,
    }
}

/// `process.env.X` / `process.env["X"]` -> `X`.
fn env_var_name(expr: &Expression<'_>) -> Option<String> {
    let member = expr.get_member_expr()?;
    if !member.object().is_specific_member_access("process", "env") {
        return None;
    }
    member.static_property_name().map(|name| js_string!(name))
}

fn is_import_meta_property(expr: &Expression<'_>, property: &str) -> bool {
    expr.get_member_expr().is_some_and(|member| {
        matches!(
            member.object().get_inner_expression(),
            Expression::ImportMeta(_)
        ) && member
            .static_property_name()
            .is_some_and(|name| name == property)
    })
}

/// Name of the function being called: `withMDX(...)` -> `withMDX`, `pkg.withX(...)` -> `withX`,
/// `require("@next/bundle-analyzer")(...)` -> `@next/bundle-analyzer`.
fn callee_name(expr: &Expression<'_>) -> String {
    match expr.get_inner_expression() {
        Expression::Identifier(id) => id.name.to_string(),
        Expression::StaticMemberExpression(member) => member.property.name.to_string(),
        Expression::CallExpression(call) => {
            if call.callee.is_specific_id("require")
                && let Some(Expression::StringLiteral(lit)) =
                    call.arguments.first().and_then(Argument::as_expression)
            {
                return js_string!(lit.value);
            }
            callee_name(&call.callee)
        }
        Expression::AwaitExpression(awaited) => callee_name(&awaited.argument),
        _ => String::from("anonymous"),
    }
}

fn normalize_path(joined: &str) -> String {
    let relative = joined.starts_with("./") || joined == ".";
    let mut parts: Vec<&str> = Vec::new();
    for part in joined.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                if parts.last().is_some_and(|p| *p != "..") {
                    parts.pop();
                } else {
                    parts.push("..");
                }
            }
            other => parts.push(other),
        }
    }
    if parts.is_empty() {
        return String::from(".");
    }
    let body = parts.join("/");
    if relative && !body.starts_with("..") {
        format!("./{body}")
    } else {
        body
    }
}

/// Static evaluator over an OXC AST with top-level binding resolution.
struct Evaluator<'a, 'x> {
    source: &'x str,
    bindings: BTreeMap<String, &'x Expression<'a>>,
    functions: BTreeMap<String, &'x ast::Function<'a>>,
    locals: Vec<String>,
    diagnostics: Vec<Diagnostic>,
    depth: usize,
}

impl<'a, 'x> Evaluator<'a, 'x> {
    fn new(source: &'x str) -> Self {
        Self {
            source,
            bindings: BTreeMap::new(),
            functions: BTreeMap::new(),
            locals: Vec::new(),
            diagnostics: Vec::new(),
            depth: 0,
        }
    }

    fn snippet(&self, expr: &Expression<'a>) -> String {
        let span = expr.span();
        if span.end as usize > self.source.len() {
            return String::from("<expression>");
        }
        let text = span.source_text(self.source);
        let collapsed = text.split_whitespace().collect::<Vec<_>>().join(" ");
        let mut out: String = collapsed.chars().take(60).collect();
        if collapsed.chars().count() > 60 {
            out.push_str("...");
        }
        out
    }

    fn dynamic(&mut self, expr: &Expression<'a>, why: &str) {
        let snippet = self.snippet(expr);
        self.diagnostics.push(Diagnostic::info(
            "next-config-dynamic-value",
            format!("`{snippet}` is not statically evaluable ({why}); value omitted"),
        ));
    }

    fn collect_top_level(&mut self, program: &'x ast::Program<'a>) {
        for statement in &program.body {
            match statement {
                Statement::VariableDeclaration(decl) => self.bind_declarators(decl),
                Statement::FunctionDeclaration(func) => self.bind_function(func),
                Statement::ExportDeclaration(export) => match &export.declaration {
                    Declaration::VariableDeclaration(decl) => self.bind_declarators(decl),
                    Declaration::FunctionDeclaration(func) => self.bind_function(func),
                    _ => {}
                },
                _ => {}
            }
        }
    }

    fn bind_function(&mut self, func: &'x ast::Function<'a>) {
        if let Some(id) = &func.id {
            self.functions.insert(id.name.to_string(), func);
        }
    }

    fn bind_declarators(&mut self, decl: &'x ast::VariableDeclaration<'a>) {
        for declarator in &decl.declarations {
            if let BindingPattern::BindingIdentifier(id) = &declarator.id
                && let Some(init) = &declarator.init
            {
                self.bindings.insert(id.name.to_string(), init);
            }
        }
    }

    /// Runs `f` on the last top-level `return` of `body` with the body's `const` declarations
    /// bound and its parameters marked as opaque locals.
    fn with_scope<R>(
        &mut self,
        params: &'x ast::FormalParameters<'a>,
        body: &'x FunctionBody<'a>,
        f: impl FnOnce(&mut Self, &'x Expression<'a>) -> Option<R>,
    ) -> Option<R> {
        let saved_bindings = self.bindings.clone();
        let saved_locals = self.locals.len();
        for param in &params.items {
            collect_binding_names(&param.pattern, &mut self.locals);
        }
        for statement in &body.statements {
            if let Statement::VariableDeclaration(decl) = statement {
                self.bind_declarators(decl);
            }
        }
        let returned = body
            .statements
            .iter()
            .rev()
            .find_map(|statement| match statement {
                Statement::ReturnStatement(ret) => ret.argument.as_ref(),
                _ => None,
            });
        let result = returned.and_then(|expr| f(self, expr));
        self.bindings = saved_bindings;
        self.locals.truncate(saved_locals);
        result
    }

    fn function_value(&mut self, func: &'x ast::Function<'a>) -> Option<Value> {
        let body = func.body.as_deref()?;
        self.with_scope(&func.params, body, Self::eval)
    }

    fn arrow_value(&mut self, arrow: &'x ast::ArrowFunctionExpression<'a>) -> Option<Value> {
        match &arrow.body {
            ArrowFunctionBody::FunctionBody(body) => {
                self.with_scope(&arrow.params, body, Self::eval)
            }
            other => other.as_expression().and_then(|expr| self.eval(expr)),
        }
    }

    /// Value of an object property: functions evaluate to their literal return or to
    /// [`FUNCTION_MARKER`].
    fn property_value(&mut self, value: &'x Expression<'a>) -> Option<Value> {
        let marker = || Value::String(String::from(FUNCTION_MARKER));
        match value.get_inner_expression() {
            Expression::ArrowFunctionExpression(arrow) => {
                Some(self.arrow_value(arrow).unwrap_or_else(marker))
            }
            Expression::FunctionExpression(func) => {
                Some(self.function_value(func).unwrap_or_else(marker))
            }
            _ => self.eval(value),
        }
    }

    fn eval(&mut self, expr: &'x Expression<'a>) -> Option<Value> {
        if self.depth > MAX_DEPTH {
            return None;
        }
        self.depth += 1;
        let inner = expr.get_inner_expression();
        let result = match inner {
            Expression::StringLiteral(lit) => Some(Value::String(js_string!(lit.value))),
            Expression::NumericLiteral(lit) => Some(number_value(lit.value)),
            Expression::BooleanLiteral(lit) => Some(Value::Bool(lit.value)),
            Expression::NullLiteral(_) => Some(Value::Null),
            Expression::TemplateLiteral(tpl) => self.eval_template(tpl, inner),
            Expression::RegExpLiteral(lit) => {
                Some(Value::String(lit.regex.pattern.text.to_string()))
            }
            Expression::Identifier(id) => self.eval_identifier(id.name.as_str(), inner),
            Expression::ArrayExpression(array) => Some(self.eval_array(array)),
            Expression::ObjectExpression(object) => Some(self.eval_object(object)),
            Expression::StaticMemberExpression(_)
            | Expression::ComputedMemberExpression(_)
            | Expression::ChainExpression(_) => self.eval_member(inner),
            Expression::CallExpression(call) => self.eval_call(call, inner),
            Expression::NewExpression(new) => self.eval_new(new, inner),
            Expression::LogicalExpression(logical) => self.eval_logical(logical),
            Expression::UnaryExpression(unary) => self.eval_unary(unary),
            Expression::BinaryExpression(binary) => self.eval_binary(binary, inner),
            Expression::ConditionalExpression(cond) => self.eval_conditional(cond, inner),
            Expression::AwaitExpression(awaited) => self.eval(&awaited.argument),
            Expression::SequenceExpression(seq) => {
                seq.expressions.last().and_then(|last| self.eval(last))
            }
            Expression::ArrowFunctionExpression(_)
            | Expression::FunctionExpression(_)
            | Expression::ClassExpression(_) => None,
            _ => {
                self.dynamic(inner, "unsupported expression");
                None
            }
        };
        self.depth -= 1;
        result
    }

    fn eval_template(
        &mut self,
        tpl: &'x ast::TemplateLiteral<'a>,
        expr: &'x Expression<'a>,
    ) -> Option<Value> {
        if !tpl.expressions.is_empty() {
            self.dynamic(expr, "template literal with expressions");
            return None;
        }
        let text: String = tpl
            .quasis
            .iter()
            .map(|quasi| {
                quasi
                    .value
                    .cooked
                    .map_or_else(|| quasi.value.raw.to_string(), |cooked| js_string!(cooked))
            })
            .collect();
        Some(Value::String(text))
    }

    fn eval_identifier(&mut self, name: &str, expr: &'x Expression<'a>) -> Option<Value> {
        match name {
            "undefined" | "NaN" | "Infinity" => return None,
            "__dirname" => return Some(Value::String(String::from("."))),
            "__filename" => return Some(Value::String(String::from("./next.config.js"))),
            _ => {}
        }
        if self.locals.iter().any(|local| local == name) {
            return None;
        }
        if let Some(bound) = self.bindings.get(name).copied() {
            return self.eval(bound);
        }
        if let Some(func) = self.functions.get(name).copied() {
            return Some(
                self.function_value(func)
                    .unwrap_or_else(|| Value::String(String::from(FUNCTION_MARKER))),
            );
        }
        self.dynamic(expr, "unresolved identifier");
        None
    }

    fn eval_array(&mut self, array: &'x ast::ArrayExpression<'a>) -> Value {
        let mut out = Vec::new();
        for element in &array.elements {
            match element {
                ArrayExpressionElement::SpreadElement(spread) => {
                    match self.eval(&spread.argument) {
                        Some(Value::Array(items)) => out.extend(items),
                        _ => self.spread_unsupported(&spread.argument),
                    }
                }
                ArrayExpressionElement::Elision(_) => out.push(Value::Null),
                other => {
                    if let Some(expr) = other.as_expression()
                        && let Some(value) = self.eval(expr)
                    {
                        out.push(value);
                    }
                }
            }
        }
        Value::Array(out)
    }

    fn spread_unsupported(&mut self, argument: &Expression<'a>) {
        let snippet = self.snippet(argument);
        let is_local = argument
            .get_identifier_reference()
            .is_some_and(|id| self.locals.iter().any(|local| local == id.name.as_str()));
        if is_local {
            self.diagnostics.push(Diagnostic::info(
                "next-config-spread-unsupported",
                format!("spread of parameter `{snippet}` ignored (Next.js defaults assumed)"),
            ));
        } else {
            self.diagnostics.push(Diagnostic::warning(
                "next-config-spread-unsupported",
                format!(
                    "spread `...{snippet}` cannot be evaluated statically; its entries are missing"
                ),
            ));
        }
    }

    fn eval_object(&mut self, object: &'x ast::ObjectExpression<'a>) -> Value {
        let mut map = Map::new();
        for property in &object.properties {
            match property {
                ObjectPropertyKind::SpreadProperty(spread) => match self.eval(&spread.argument) {
                    Some(Value::Object(inner)) => map.extend(inner),
                    _ => self.spread_unsupported(&spread.argument),
                },
                ObjectPropertyKind::ObjectProperty(prop) => {
                    let Some(key) = property_key_name(&prop.key) else {
                        self.diagnostics.push(Diagnostic::info(
                            "next-config-computed-key-unsupported",
                            "computed object key skipped",
                        ));
                        continue;
                    };
                    if let Some(value) = self.property_value(&prop.value) {
                        map.insert(key, value);
                    }
                }
            }
        }
        Value::Object(map)
    }

    fn eval_member(&mut self, expr: &'x Expression<'a>) -> Option<Value> {
        if let Some(name) = env_var_name(expr) {
            return Some(Value::String(format!("${{env:{name}}}")));
        }
        if is_import_meta_property(expr, "dirname") {
            return Some(Value::String(String::from(".")));
        }
        let member = expr.get_member_expr()?;
        let Some(property) = member.static_property_name() else {
            self.dynamic(expr, "computed member access");
            return None;
        };
        let property = js_string!(property);
        let object = member.object();
        if matches!(object.get_inner_expression(), Expression::ImportMeta(_)) {
            self.dynamic(expr, "import.meta");
            return None;
        }
        match self.eval(object) {
            Some(Value::Object(map)) => map.get(&property).cloned(),
            Some(Value::Array(items)) if property == "length" => Some(Value::from(items.len())),
            Some(Value::String(text)) if property == "length" => {
                Some(Value::from(text.chars().count()))
            }
            _ => None,
        }
    }

    fn eval_call(
        &mut self,
        call: &'x ast::CallExpression<'a>,
        expr: &'x Expression<'a>,
    ) -> Option<Value> {
        let callee = call.callee.get_inner_expression();
        let args: Vec<&'x Expression<'a>> = call
            .arguments
            .iter()
            .filter_map(Argument::as_expression)
            .collect();
        if callee.is_specific_member_access("process", "cwd") {
            return Some(Value::String(String::from(".")));
        }
        if callee.is_specific_member_access("JSON", "parse") {
            let text = self.eval(args.first()?)?;
            return serde_json::from_str(text.as_str()?).ok();
        }
        let name = callee_name(callee);
        match name.as_str() {
            "dirname"
                if args
                    .first()
                    .is_some_and(|arg| is_file_url_to_path_call(arg)) =>
            {
                Some(Value::String(String::from(".")))
            }
            "fileURLToPath"
                if args
                    .first()
                    .is_some_and(|arg| is_import_meta_property(arg, "url")) =>
            {
                Some(Value::String(String::from("./next.config.js")))
            }
            "join" | "resolve" | "normalize" => self.join_paths(&args),
            "freeze" | "defineConfig" | "defineNextConfig" => {
                args.first().and_then(|arg| self.eval(arg))
            }
            "String" => self
                .eval(args.first()?)
                .and_then(|v| scalar_text(&v))
                .map(Value::String),
            "Number" | "parseInt" | "parseFloat" => {
                let value = self.eval(args.first()?)?;
                match value {
                    Value::Number(n) => Some(Value::Number(n)),
                    Value::String(s) => s.trim().parse::<f64>().ok().map(number_value),
                    Value::Bool(b) => Some(Value::from(u8::from(b))),
                    _ => None,
                }
            }
            "Boolean" => self
                .eval(args.first()?)
                .and_then(|v| truthy(&v))
                .map(Value::Bool),
            _ => {
                self.dynamic(expr, "function call");
                None
            }
        }
    }

    fn join_paths(&mut self, args: &[&'x Expression<'a>]) -> Option<Value> {
        let mut parts = Vec::new();
        for arg in args {
            let value = self.eval(arg)?;
            parts.push(value.as_str()?.to_owned());
        }
        if parts.is_empty() {
            return None;
        }
        Some(Value::String(normalize_path(&parts.join("/"))))
    }

    fn eval_new(
        &mut self,
        new: &'x ast::NewExpression<'a>,
        expr: &'x Expression<'a>,
    ) -> Option<Value> {
        let args: Vec<&'x Expression<'a>> = new
            .arguments
            .iter()
            .filter_map(Argument::as_expression)
            .collect();
        match callee_name(&new.callee).as_str() {
            "URL" => {
                let first = self.eval(args.first()?)?;
                let first = first.as_str()?.to_owned();
                if first.contains("://") {
                    return Some(Value::String(first));
                }
                let base = args.get(1).and_then(|arg| self.eval(arg))?;
                let base = base.as_str()?.trim_end_matches('/').to_owned();
                Some(Value::String(format!(
                    "{base}/{}",
                    first.trim_start_matches('/')
                )))
            }
            "RegExp" => match args.first()?.get_inner_expression() {
                Expression::RegExpLiteral(lit) => {
                    Some(Value::String(lit.regex.pattern.text.to_string()))
                }
                other => self.eval(other).filter(Value::is_string),
            },
            _ => {
                self.dynamic(expr, "constructor call");
                None
            }
        }
    }

    fn eval_logical(&mut self, logical: &'x ast::LogicalExpression<'a>) -> Option<Value> {
        if matches!(
            logical.operator,
            LogicalOperator::Coalesce | LogicalOperator::Or
        ) && let Some(name) = env_var_name(&logical.left)
        {
            let default = self.eval(&logical.right).and_then(|v| scalar_text(&v));
            return Some(Value::String(match default {
                Some(default) => format!("${{env:{name}|{default}}}"),
                None => format!("${{env:{name}}}"),
            }));
        }
        let left = self.eval(&logical.left)?;
        match logical.operator {
            LogicalOperator::Coalesce => {
                if left.is_null() {
                    self.eval(&logical.right)
                } else {
                    Some(left)
                }
            }
            LogicalOperator::Or => {
                if truthy(&left)? {
                    Some(left)
                } else {
                    self.eval(&logical.right)
                }
            }
            LogicalOperator::And => {
                if truthy(&left)? {
                    self.eval(&logical.right)
                } else {
                    Some(left)
                }
            }
        }
    }

    fn eval_unary(&mut self, unary: &'x ast::UnaryExpression<'a>) -> Option<Value> {
        let value = self.eval(&unary.argument)?;
        match unary.operator {
            UnaryOperator::UnaryNegation => {
                let number = value.as_f64()?;
                Some(number_value(-number))
            }
            UnaryOperator::UnaryPlus => match value {
                Value::Number(n) => Some(Value::Number(n)),
                Value::String(s) => s.trim().parse::<f64>().ok().map(number_value),
                Value::Bool(b) => Some(Value::from(u8::from(b))),
                _ => None,
            },
            UnaryOperator::LogicalNot => truthy(&value).map(|t| Value::Bool(!t)),
            _ => None,
        }
    }

    fn eval_binary(
        &mut self,
        binary: &'x ast::BinaryExpression<'a>,
        expr: &'x Expression<'a>,
    ) -> Option<Value> {
        let left = self.eval(&binary.left)?;
        let right = self.eval(&binary.right)?;
        if is_env_marker(&left) || is_env_marker(&right) {
            self.dynamic(expr, "depends on process.env");
            return None;
        }
        match binary.operator {
            BinaryOperator::Addition => match (&left, &right) {
                (Value::Number(a), Value::Number(b)) => {
                    Some(number_value(a.as_f64()? + b.as_f64()?))
                }
                _ => Some(Value::String(format!(
                    "{}{}",
                    scalar_text(&left)?,
                    scalar_text(&right)?
                ))),
            },
            BinaryOperator::Subtraction => Some(number_value(left.as_f64()? - right.as_f64()?)),
            BinaryOperator::Multiplication => Some(number_value(left.as_f64()? * right.as_f64()?)),
            BinaryOperator::Division => Some(number_value(left.as_f64()? / right.as_f64()?)),
            BinaryOperator::StrictEquality | BinaryOperator::Equality => {
                Some(Value::Bool(left == right))
            }
            BinaryOperator::StrictInequality | BinaryOperator::Inequality => {
                Some(Value::Bool(left != right))
            }
            _ => {
                self.dynamic(expr, "binary operator");
                None
            }
        }
    }

    fn eval_conditional(
        &mut self,
        cond: &'x ast::ConditionalExpression<'a>,
        expr: &'x Expression<'a>,
    ) -> Option<Value> {
        let Some(test) = self.eval(&cond.test).and_then(|v| truthy(&v)) else {
            self.dynamic(expr, "conditional on a runtime value");
            return None;
        };
        if test {
            self.eval(&cond.consequent)
        } else {
            self.eval(&cond.alternate)
        }
    }

    // ---- root config resolution -------------------------------------------------------------

    /// Resolves the exported config expression: object literal, binding, plugin wrapper call,
    /// `defineConfig(...)`, or phase function.
    fn resolve_config(
        &mut self,
        expr: &'x Expression<'a>,
        wrappers: &mut Vec<String>,
        phase: &mut bool,
    ) -> Option<Value> {
        if self.depth > MAX_DEPTH {
            return None;
        }
        self.depth += 1;
        let result = match expr.get_inner_expression() {
            Expression::ObjectExpression(object) => Some(self.eval_object(object)),
            Expression::Identifier(id) => self.resolve_name(id.name.as_str(), wrappers, phase),
            Expression::CallExpression(call) => self.resolve_wrapper(call, wrappers, phase),
            Expression::ArrowFunctionExpression(arrow) => {
                *phase = true;
                match &arrow.body {
                    ArrowFunctionBody::FunctionBody(body) => {
                        self.with_scope(&arrow.params, body, |ev, ret| {
                            ev.resolve_config(ret, wrappers, phase)
                        })
                    }
                    other => other
                        .as_expression()
                        .and_then(|body| self.resolve_config(body, wrappers, phase)),
                }
            }
            Expression::FunctionExpression(func) => {
                *phase = true;
                self.resolve_function(func, wrappers, phase)
            }
            Expression::AwaitExpression(awaited) => {
                self.resolve_config(&awaited.argument, wrappers, phase)
            }
            other => self.eval(other).filter(Value::is_object),
        };
        self.depth -= 1;
        result
    }

    fn resolve_function(
        &mut self,
        func: &'x ast::Function<'a>,
        wrappers: &mut Vec<String>,
        phase: &mut bool,
    ) -> Option<Value> {
        let body = func.body.as_deref()?;
        self.with_scope(&func.params, body, |ev, ret| {
            ev.resolve_config(ret, wrappers, phase)
        })
    }

    fn resolve_name(
        &mut self,
        name: &str,
        wrappers: &mut Vec<String>,
        phase: &mut bool,
    ) -> Option<Value> {
        if let Some(func) = self.functions.get(name).copied() {
            *phase = true;
            return self.resolve_function(func, wrappers, phase);
        }
        let bound = self.bindings.get(name).copied()?;
        self.resolve_config(bound, wrappers, phase)
    }

    fn resolve_wrapper(
        &mut self,
        call: &'x ast::CallExpression<'a>,
        wrappers: &mut Vec<String>,
        phase: &mut bool,
    ) -> Option<Value> {
        let name = callee_name(&call.callee);
        let is_define = matches!(name.as_str(), "defineConfig" | "defineNextConfig");
        let saved_len = wrappers.len();
        if !is_define {
            wrappers.push(name);
        }
        for argument in &call.arguments {
            let Some(expr) = argument.as_expression() else {
                continue;
            };
            let inner = expr.get_inner_expression();
            if !matches!(
                inner,
                Expression::ObjectExpression(_)
                    | Expression::Identifier(_)
                    | Expression::CallExpression(_)
                    | Expression::ArrowFunctionExpression(_)
                    | Expression::FunctionExpression(_)
                    | Expression::AwaitExpression(_)
            ) {
                continue;
            }
            let depth_before = wrappers.len();
            if let Some(value) = self.resolve_config(inner, wrappers, phase) {
                return Some(value);
            }
            wrappers.truncate(depth_before);
        }
        wrappers.truncate(saved_len);
        None
    }
}

fn collect_binding_names(pattern: &BindingPattern<'_>, out: &mut Vec<String>) {
    match pattern {
        BindingPattern::BindingIdentifier(id) => out.push(id.name.to_string()),
        BindingPattern::ObjectPattern(object) => {
            for property in &object.properties {
                collect_binding_names(&property.value, out);
            }
            if let Some(rest) = &object.rest {
                collect_binding_names(&rest.argument, out);
            }
        }
        BindingPattern::ArrayPattern(array) => {
            for element in array.elements.iter().flatten() {
                collect_binding_names(element, out);
            }
            if let Some(rest) = &array.rest {
                collect_binding_names(&rest.argument, out);
            }
        }
        BindingPattern::AssignmentPattern(assignment) => {
            collect_binding_names(&assignment.left, out);
        }
    }
}

/// `fileURLToPath(import.meta.url)`.
fn is_file_url_to_path_call(expr: &Expression<'_>) -> bool {
    match expr.get_inner_expression() {
        Expression::CallExpression(call) => {
            callee_name(&call.callee) == "fileURLToPath"
                && call
                    .arguments
                    .first()
                    .and_then(Argument::as_expression)
                    .is_some_and(|arg| is_import_meta_property(arg, "url"))
        }
        _ => false,
    }
}

enum Exported<'a, 'x> {
    Expr(&'x Expression<'a>),
    Func(&'x ast::Function<'a>),
    Name(String),
}

fn module_export_name(name: &ModuleExportName<'_>) -> String {
    match name {
        ModuleExportName::IdentifierName(id) => id.name.to_string(),
        ModuleExportName::IdentifierReference(id) => id.name.to_string(),
        ModuleExportName::StringLiteral(lit) => js_string!(lit.value),
    }
}

/// `module.exports = ...` / `exports.default = ...` right-hand side.
fn commonjs_export<'a, 'x>(expr: &'x Expression<'a>) -> Option<&'x Expression<'a>> {
    let Expression::AssignmentExpression(assign) = expr else {
        return None;
    };
    let AssignmentTarget::StaticMemberExpression(member) = &assign.left else {
        return None;
    };
    let object = member.object.get_identifier_reference()?.name.as_str();
    let property = member.property.name.as_str();
    let matches = (object == "module" && property == "exports")
        || (object == "exports" && property == "default");
    matches.then_some(&assign.right)
}

fn find_export<'a, 'x>(program: &'x ast::Program<'a>) -> Option<Exported<'a, 'x>> {
    let mut found = None;
    for statement in &program.body {
        match statement {
            Statement::ExportDefaultDeclaration(decl) => {
                return match &decl.declaration {
                    ExportDefaultDeclarationKind::FunctionDeclaration(func) => {
                        Some(Exported::Func(func))
                    }
                    ExportDefaultDeclarationKind::ClassDeclaration(_)
                    | ExportDefaultDeclarationKind::TSInterfaceDeclaration(_) => None,
                    other => other.as_expression().map(Exported::Expr),
                };
            }
            Statement::ExpressionStatement(stmt) => {
                if let Some(right) = commonjs_export(&stmt.expression) {
                    found = Some(Exported::Expr(right));
                }
            }
            Statement::ExportNamedDeclaration(named) => {
                for specifier in &named.specifiers {
                    if module_export_name(&specifier.exported) == "default" {
                        found = Some(Exported::Name(module_export_name(&specifier.local)));
                    }
                }
            }
            Statement::TSExportAssignment(assign) => {
                found = Some(Exported::Expr(&assign.expression));
            }
            _ => {}
        }
    }
    found
}

/// Parses a `next.config` source file. Returns the config plus warnings, or parse errors.
pub fn parse_next_config(
    source: &str,
    file_name: &str,
) -> Result<(NextConfig, Vec<Diagnostic>), Vec<Diagnostic>> {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, source_type_for(file_name)).parse();
    if parsed.fatal_error || !parsed.diagnostics.is_empty() {
        let mut errors: Vec<Diagnostic> = parsed
            .diagnostics
            .iter()
            .map(|d| {
                Diagnostic::error("next-config-parse-error", d.to_string()).with_file(file_name)
            })
            .collect();
        if errors.is_empty() {
            errors.push(
                Diagnostic::error("next-config-parse-error", "cannot parse next.config")
                    .with_file(file_name),
            );
        }
        return Err(errors);
    }
    let program = &parsed.program;
    let mut evaluator = Evaluator::new(source);
    evaluator.collect_top_level(program);
    let Some(exported) = find_export(program) else {
        return Err(vec![
            Diagnostic::error(
                "next-config-no-export",
                "no `export default`, `module.exports =` or `export =` found",
            )
            .with_file(file_name),
        ]);
    };
    let mut wrappers = Vec::new();
    let mut phase = false;
    let value = match exported {
        Exported::Expr(expr) => evaluator.resolve_config(expr, &mut wrappers, &mut phase),
        Exported::Func(func) => {
            phase = true;
            evaluator.resolve_function(func, &mut wrappers, &mut phase)
        }
        Exported::Name(name) => evaluator.resolve_name(&name, &mut wrappers, &mut phase),
    };
    let Some(value) = value else {
        let mut errors = evaluator.diagnostics;
        errors.push(
            Diagnostic::error(
                "next-config-unresolvable",
                "the exported config does not resolve to an object literal",
            )
            .with_hint("export a plain object (or a wrapper call around one); dynamic configs need manual migration"),
        );
        for d in &mut errors {
            d.file.get_or_insert_with(|| file_name.to_owned());
        }
        return Err(errors);
    };
    let (mut cfg, mut key_diagnostics) = next_config_from_value(&value);
    cfg.wrappers.clone_from(&wrappers);
    let mut diagnostics = evaluator.diagnostics;
    if !wrappers.is_empty() {
        diagnostics.push(Diagnostic::info(
            "next-config-wrapper",
            format!(
                "config is wrapped by {}; the innermost object was evaluated",
                wrappers.join(" -> ")
            ),
        ));
    }
    if phase {
        diagnostics.push(Diagnostic::info(
            "next-config-phase-function",
            "config is a phase function; the returned object literal was evaluated for every phase",
        ));
    }
    diagnostics.append(&mut key_diagnostics);
    for d in &mut diagnostics {
        d.file.get_or_insert_with(|| file_name.to_owned());
    }
    Ok((cfg, diagnostics))
}

/// Statically converts an OXC expression (object/array/literal/template/`process.env.X`) to JSON.
pub fn expression_to_json(expr: &Expression<'_>) -> Option<Value> {
    let mut evaluator = Evaluator::new("");
    evaluator.eval(expr)
}

/// Parses a standalone JavaScript object literal source (`{ a: 1 }`) into JSON.
pub fn object_literal_to_json(source: &str) -> Result<Value, Vec<Diagnostic>> {
    let wrapped = format!("const __yolo_object__ = ({source});");
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, &wrapped, SourceType::ts()).parse();
    if parsed.fatal_error || !parsed.diagnostics.is_empty() {
        return Err(parsed
            .diagnostics
            .iter()
            .map(|d| Diagnostic::error("object-literal-parse-error", d.to_string()))
            .collect());
    }
    let init = parsed
        .program
        .body
        .iter()
        .find_map(|statement| match statement {
            Statement::VariableDeclaration(decl) => {
                decl.declarations.first().and_then(|d| d.init.as_ref())
            }
            _ => None,
        });
    let Some(init) = init else {
        return Err(vec![Diagnostic::error(
            "object-literal-parse-error",
            "no expression found",
        )]);
    };
    let mut evaluator = Evaluator::new(&wrapped);
    let value = evaluator.eval(init);
    value.ok_or_else(|| {
        let mut diagnostics = evaluator.diagnostics;
        diagnostics.push(Diagnostic::error(
            "object-literal-unevaluable",
            "the expression cannot be evaluated statically",
        ));
        diagnostics
    })
}

// ============================================================================
// JSON -> NextConfig
// ============================================================================

fn as_string(value: &Value) -> Option<String> {
    value.as_str().map(str::to_owned)
}

fn as_u64(value: &Value) -> Option<u64> {
    match value {
        Value::Number(n) => n
            .as_u64()
            .or_else(|| n.as_f64().filter(|f| *f >= 0.0).map(|f| f as u64)),
        Value::String(s) => s.trim().parse().ok(),
        _ => None,
    }
}

fn is_truthy(value: &Value) -> bool {
    truthy(value).unwrap_or(true)
}

fn string_list(value: &Value) -> Vec<String> {
    match value {
        Value::Array(items) => items.iter().filter_map(as_string).collect(),
        Value::String(s) => vec![s.clone()],
        _ => Vec::new(),
    }
}

fn number_list<T: TryFrom<u64>>(value: &Value) -> Vec<T> {
    value.as_array().map_or_else(Vec::new, |items| {
        items
            .iter()
            .filter_map(as_u64)
            .filter_map(|n| T::try_from(n).ok())
            .collect()
    })
}

fn string_map(value: &Value) -> BTreeMap<String, String> {
    value.as_object().map_or_else(BTreeMap::new, |map| {
        map.iter()
            .map(|(k, v)| {
                let text = match v {
                    Value::String(s) => s.clone(),
                    other => other.to_string(),
                };
                (k.clone(), text)
            })
            .collect()
    })
}

fn is_function_marker(value: &Value) -> bool {
    value.as_str() == Some(FUNCTION_MARKER)
}

/// Parses a `has` / `missing` array (`[{ type, key, value }]`) into conditions.
pub(crate) fn conditions_from(value: Option<&Value>) -> Vec<RouteCondition> {
    value
        .and_then(Value::as_array)
        .map_or_else(Vec::new, |items| {
            items
                .iter()
                .filter_map(Value::as_object)
                .filter_map(|cond| {
                    Some(RouteCondition {
                        kind: as_string(cond.get("type")?)?,
                        key: cond.get("key").and_then(as_string).unwrap_or_default(),
                        value: cond.get("value").and_then(as_string),
                    })
                })
                .collect()
        })
}

fn route_rule_from(value: &Value) -> Option<RouteRule> {
    let object = value.as_object()?;
    Some(RouteRule {
        source: as_string(object.get("source")?)?,
        destination: object
            .get("destination")
            .and_then(as_string)
            .unwrap_or_default(),
        permanent: object.get("permanent").and_then(Value::as_bool),
        status_code: object
            .get("statusCode")
            .and_then(as_u64)
            .and_then(|n| u16::try_from(n).ok()),
        base_path: object.get("basePath").and_then(Value::as_bool),
        locale: object.get("locale").and_then(Value::as_bool),
        has: conditions_from(object.get("has")),
        missing: conditions_from(object.get("missing")),
    })
}

fn header_rule_from(value: &Value) -> Option<HeaderRule> {
    let object = value.as_object()?;
    let headers = object
        .get("headers")
        .and_then(Value::as_array)
        .map_or_else(Vec::new, |items| {
            items
                .iter()
                .filter_map(Value::as_object)
                .filter_map(|kv| {
                    Some(HeaderKv {
                        key: as_string(kv.get("key")?)?,
                        value: kv.get("value").and_then(as_string).unwrap_or_default(),
                    })
                })
                .collect()
        });
    Some(HeaderRule {
        source: as_string(object.get("source")?)?,
        headers,
        base_path: object.get("basePath").and_then(Value::as_bool),
        locale: object.get("locale").and_then(Value::as_bool),
        has: conditions_from(object.get("has")),
        missing: conditions_from(object.get("missing")),
    })
}

fn cache_life_profiles(value: &Value) -> BTreeMap<String, CacheLifeProfile> {
    value.as_object().map_or_else(BTreeMap::new, |map| {
        map.iter()
            .filter_map(|(name, profile)| {
                let profile = profile.as_object()?;
                Some((
                    name.clone(),
                    CacheLifeProfile {
                        stale: profile.get("stale").and_then(as_u64),
                        revalidate: profile.get("revalidate").and_then(as_u64),
                        expire: profile.get("expire").and_then(as_u64),
                    },
                ))
            })
            .collect()
    })
}

/// `https://cdn.example.com:8443/media/**?v=1` -> remote pattern.
fn remote_pattern_from_url(url: &str) -> Option<RemotePattern> {
    let (protocol, rest) = url.split_once("://")?;
    let (rest, search) = rest
        .split_once('?')
        .map_or((rest, None), |(r, s)| (r, Some(format!("?{s}"))));
    let (authority, pathname) = rest
        .split_once('/')
        .map_or((rest, None), |(a, p)| (a, Some(format!("/{p}"))));
    let (hostname, port) = authority
        .rsplit_once(':')
        .map_or((authority, None), |(h, p)| (h, Some(p.to_owned())));
    Some(RemotePattern {
        protocol: Some(protocol.to_owned()),
        hostname: hostname.to_owned(),
        port,
        pathname,
        search,
    })
}

fn remote_patterns_from(value: &Value) -> Vec<RemotePattern> {
    value.as_array().map_or_else(Vec::new, |items| {
        items
            .iter()
            .filter_map(|item| match item {
                Value::String(url) => remote_pattern_from_url(url),
                Value::Object(object) => Some(RemotePattern {
                    protocol: object
                        .get("protocol")
                        .and_then(as_string)
                        .map(|p| p.trim_end_matches(':').to_owned()),
                    hostname: as_string(object.get("hostname")?)?,
                    port: object.get("port").and_then(as_string),
                    pathname: object.get("pathname").and_then(as_string),
                    search: object.get("search").and_then(as_string),
                }),
                _ => None,
            })
            .collect()
    })
}

fn turbopack_loaders(value: Option<&Value>) -> Vec<TurbopackLoader> {
    value
        .and_then(Value::as_array)
        .map_or_else(Vec::new, |items| {
            items
                .iter()
                .filter_map(|item| match item {
                    Value::String(loader) => {
                        Some(TurbopackLoader { loader: loader.clone(), options: Map::new() })
                    }
                    Value::Object(object) => Some(TurbopackLoader {
                        loader: as_string(object.get("loader")?)?,
                        options: object
                            .get("options")
                            .and_then(Value::as_object)
                            .cloned()
                            .unwrap_or_default(),
                    }),
                    _ => None,
                })
                .collect()
        })
}

fn is_loader_item(value: &Value) -> bool {
    match value {
        Value::String(_) => true,
        Value::Object(object) => object.contains_key("loader"),
        _ => false,
    }
}

fn turbopack_rules_from(value: &Value, condition: Option<Value>) -> Vec<TurbopackRule> {
    match value {
        Value::Array(items) if items.iter().all(is_loader_item) => vec![TurbopackRule {
            loaders: turbopack_loaders(Some(value)),
            as_: None,
            type_: None,
            condition,
        }],
        Value::Array(items) => items
            .iter()
            .flat_map(|item| turbopack_rules_from(item, condition.clone()))
            .collect(),
        Value::Object(object) => {
            if object.contains_key("loaders")
                || object.contains_key("as")
                || object.contains_key("type")
            {
                vec![TurbopackRule {
                    loaders: turbopack_loaders(object.get("loaders")),
                    as_: object.get("as").and_then(as_string),
                    type_: object.get("type").and_then(as_string),
                    condition: object.get("condition").cloned().or(condition),
                }]
            } else {
                object
                    .iter()
                    .filter(|(_, sub)| **sub != Value::Bool(false))
                    .flat_map(|(key, sub)| {
                        turbopack_rules_from(sub, Some(Value::String(key.clone())))
                    })
                    .collect()
            }
        }
        _ => Vec::new(),
    }
}

/// Accumulates a [`NextConfig`] from JSON keys with rename/removal bookkeeping.
struct Builder {
    cfg: NextConfig,
    diagnostics: Vec<Diagnostic>,
}

impl Builder {
    fn removed(&mut self, key: &str) {
        self.cfg.removed_keys.push(key.to_owned());
        self.diagnostics.push(Diagnostic::warning(
            "next-config-removed-key",
            format!("`{key}` was removed in Next.js 16 and is ignored"),
        ));
    }

    fn renamed(&mut self, from: &str, to: &str) {
        self.cfg.renamed_keys.push((from.to_owned(), to.to_owned()));
        self.diagnostics.push(Diagnostic::info(
            "next-config-renamed-key",
            format!("`{from}` is read as `{to}`"),
        ));
    }

    fn unknown(&mut self, key: &str, value: &Value) {
        self.cfg.extra.insert(key.to_owned(), value.clone());
        self.diagnostics.push(Diagnostic::info(
            "next-config-unknown-key",
            format!("unknown key `{key}` kept in `extra`"),
        ));
    }

    fn dynamic(&mut self, key: &str) {
        self.diagnostics.push(
            Diagnostic::warning(
                format!("next-config-dynamic-{key}"),
                format!("`{key}()` does not return a literal; no static manifest"),
            )
            .with_hint(
                "the function is awaited at startup by @yolo-rs/next; static manifest unavailable",
            ),
        );
    }

    fn apply(&mut self, key: &str, value: &Value) {
        if self.apply_scalar(key, value) || self.apply_structured(key, value) {
            return;
        }
        match key {
            "webpack" => {
                self.cfg.webpack_present = true;
                self.diagnostics.push(
                    Diagnostic::error(
                        "next-config-webpack-unsupported",
                        "`webpack()` customisation is not supported",
                    )
                    .with_hint("Turbopack-only: translate loaders with turbopack.rules; @yolo-rs/next uses Bun.build plugins"),
                );
            }
            "exportPathMap" => {
                self.cfg.export_path_map_present = true;
                self.diagnostics.push(
                    Diagnostic::warning(
                        "next-config-export-path-map",
                        "`exportPathMap` is ignored; the App Router manifest drives static export",
                    )
                    .with_hint("use generateStaticParams in the App Router"),
                );
            }
            "generateBuildId" => {
                self.cfg.generate_build_id_present = true;
                self.diagnostics.push(Diagnostic::info(
                    "next-config-generate-build-id",
                    "`generateBuildId()` is replaced by the Bun build hash (set YOLO_BUILD_ID to override)",
                ));
            }
            removed if REMOVED_TOP_LEVEL.contains(&removed) => self.removed(removed),
            _ => self.unknown(key, value),
        }
    }

    fn apply_scalar(&mut self, key: &str, value: &Value) -> bool {
        let cfg = &mut self.cfg;
        match key {
            "reactStrictMode" => cfg.react_strict_mode = value.as_bool(),
            "output" => cfg.output = as_string(value),
            "distDir" => cfg.dist_dir = as_string(value),
            "basePath" => cfg.base_path = as_string(value).unwrap_or_default(),
            "assetPrefix" => cfg.asset_prefix = as_string(value).unwrap_or_default(),
            "trailingSlash" => cfg.trailing_slash = is_truthy(value),
            "skipTrailingSlashRedirect" => cfg.skip_trailing_slash_redirect = is_truthy(value),
            "skipProxyUrlNormalize" => cfg.skip_proxy_url_normalize = is_truthy(value),
            "skipMiddlewareUrlNormalize" => {
                self.renamed("skipMiddlewareUrlNormalize", "skipProxyUrlNormalize");
                self.cfg.skip_proxy_url_normalize = is_truthy(value);
            }
            "compress" => cfg.compress = value.as_bool(),
            "poweredByHeader" => cfg.powered_by_header = value.as_bool(),
            "generateEtags" => cfg.generate_etags = value.as_bool(),
            "reactCompiler" => cfg.react_compiler = Some(value.clone()),
            "reactProductionProfiling" => cfg.react_production_profiling = value.as_bool(),
            "reactMaxHeadersLength" => cfg.react_max_headers_length = as_u64(value),
            "cacheComponents" => cfg.cache_components = is_truthy(value),
            "partialPrefetching" => cfg.partial_prefetching = value.as_bool(),
            "typedRoutes" => cfg.typed_routes = is_truthy(value),
            "transpilePackages" => cfg.transpile_packages = string_list(value),
            "serverExternalPackages" => cfg.server_external_packages = string_list(value),
            "allowedDevOrigins" => cfg.allowed_dev_origins = string_list(value),
            "bundlePagesRouterDependencies" => {
                cfg.bundle_pages_router_dependencies = is_truthy(value);
            }
            "pageExtensions" => {
                let list = string_list(value);
                if !list.is_empty() {
                    cfg.page_extensions = list;
                }
            }
            "env" => cfg.env = string_map(value),
            "deploymentId" => cfg.deployment_id = as_string(value),
            "supportsImmutableAssets" => cfg.supports_immutable_assets = value.as_bool(),
            "expireTime" => cfg.expire_time = as_u64(value),
            "crossOrigin" => cfg.cross_origin = as_string(value),
            "productionBrowserSourceMaps" => cfg.production_browser_source_maps = is_truthy(value),
            "staticPageGenerationTimeout" => cfg.static_page_generation_timeout = as_u64(value),
            "cacheHandler" => cfg.cache_handler = as_string(value),
            "cacheMaxMemorySize" => cfg.cache_max_memory_size = as_u64(value),
            "cacheHandlers" => cfg.cache_handlers = string_map(value),
            "logging" => cfg.logging = Some(value.clone()),
            "outputFileTracingRoot" => cfg.output_file_tracing_root = as_string(value),
            "outputFileTracingIncludes" => cfg.output_file_tracing_includes = Some(value.clone()),
            "outputFileTracingExcludes" => cfg.output_file_tracing_excludes = Some(value.clone()),
            "modularizeImports" => {
                cfg.modularize_imports = value.as_object().cloned().unwrap_or_default();
            }
            "sassOptions" => cfg.sass_options = Some(value.clone()),
            "htmlLimitedBots" => cfg.html_limited_bots = as_string(value),
            "adapterPath" => cfg.adapter_path = as_string(value),
            _ => return false,
        }
        true
    }

    fn apply_structured(&mut self, key: &str, value: &Value) -> bool {
        match key {
            "turbopack" => self.apply_turbopack(value, "turbopack"),
            "images" => self.apply_images(value),
            "redirects" => self.apply_redirects(value),
            "rewrites" => self.apply_rewrites(value),
            "headers" => self.apply_headers(value),
            "compiler" => self.apply_compiler(value),
            "typescript" => {
                if let Some(object) = value.as_object() {
                    self.cfg.typescript.ignore_build_errors =
                        object.get("ignoreBuildErrors").is_some_and(is_truthy);
                    self.cfg.typescript.tsconfig_path =
                        object.get("tsconfigPath").and_then(as_string);
                }
            }
            "cacheLife" => self.cfg.cache_life.extend(cache_life_profiles(value)),
            "i18n" => self.apply_i18n(value),
            "devIndicators" => self.apply_dev_indicators(value),
            _ => return false,
        }
        true
    }

    fn apply_turbopack(&mut self, value: &Value, source_key: &str) {
        let Some(object) = value.as_object() else {
            return;
        };
        let turbo: &mut TurbopackConfig = &mut self.cfg.turbopack;
        for (key, v) in object {
            match key.as_str() {
                "root" => turbo.root = as_string(v).or(turbo.root.take()),
                "rules" | "loaders" => {
                    if let Some(rules) = v.as_object() {
                        for (glob, rule) in rules {
                            turbo
                                .rules
                                .entry(glob.clone())
                                .or_default()
                                .extend(turbopack_rules_from(rule, None));
                        }
                    }
                }
                "resolveAlias" => {
                    if let Some(aliases) = v.as_object() {
                        for (alias, target) in aliases {
                            turbo
                                .resolve_alias
                                .entry(alias.clone())
                                .or_insert_with(|| target.clone());
                        }
                    }
                }
                "resolveExtensions" => {
                    if turbo.resolve_extensions.is_empty() {
                        turbo.resolve_extensions = string_list(v);
                    }
                }
                "debugIds" => turbo.debug_ids = v.as_bool().or(turbo.debug_ids),
                "ignoreIssue" | "ignoreIssues" => {
                    if let Some(items) = v.as_array() {
                        turbo.ignore_issue.extend(items.iter().cloned());
                    }
                }
                "chunkLoadingGlobal" => {
                    turbo.chunk_loading_global = as_string(v).or(turbo.chunk_loading_global.take());
                }
                other => self.diagnostics.push(Diagnostic::info(
                    "next-config-unknown-key",
                    format!("unknown key `{source_key}.{other}` ignored"),
                )),
            }
        }
    }

    fn apply_images(&mut self, value: &Value) {
        let Some(object) = value.as_object() else {
            return;
        };
        let images: &mut ImagesConfig = &mut self.cfg.images;
        let mut deprecated_domains = false;
        for (key, v) in object {
            match key.as_str() {
                "remotePatterns" => images.remote_patterns.extend(remote_patterns_from(v)),
                "localPatterns" => {
                    images.local_patterns = v.as_array().map(|items| {
                        items
                            .iter()
                            .filter_map(Value::as_object)
                            .map(|p| LocalPattern {
                                pathname: p.get("pathname").and_then(as_string),
                                search: p.get("search").and_then(as_string),
                            })
                            .collect()
                    });
                }
                "loader" => images.loader = as_string(v).unwrap_or_else(|| String::from("default")),
                "loaderFile" => images.loader_file = as_string(v).unwrap_or_default(),
                "path" => {
                    images.path = as_string(v).unwrap_or_else(|| String::from("/_next/image"));
                }
                "formats" => images.formats = string_list(v),
                "deviceSizes" => images.device_sizes = number_list(v),
                "imageSizes" => images.image_sizes = number_list(v),
                "qualities" => images.qualities = number_list(v),
                "minimumCacheTTL" => images.minimum_cache_ttl = as_u64(v).unwrap_or(14_400),
                "dangerouslyAllowSVG" => images.dangerously_allow_svg = is_truthy(v),
                "dangerouslyAllowLocalIP" => images.dangerously_allow_local_ip = is_truthy(v),
                "contentSecurityPolicy" => {
                    images.content_security_policy = as_string(v).unwrap_or_default();
                }
                "contentDispositionType" => {
                    images.content_disposition_type =
                        as_string(v).unwrap_or_else(|| String::from("attachment"));
                }
                "unoptimized" => images.unoptimized = is_truthy(v),
                "maximumRedirects" => {
                    images.maximum_redirects =
                        as_u64(v).and_then(|n| u32::try_from(n).ok()).unwrap_or(3);
                }
                "maximumResponseBody" => {
                    images.maximum_response_body = as_u64(v).unwrap_or(50_000_000);
                }
                "domains" => {
                    deprecated_domains = true;
                    for hostname in string_list(v) {
                        images.remote_patterns.push(RemotePattern {
                            protocol: None,
                            hostname: hostname.clone(),
                            port: None,
                            pathname: None,
                            search: None,
                        });
                        images.domains.push(hostname);
                    }
                }
                other => self.diagnostics.push(Diagnostic::info(
                    "next-config-unknown-key",
                    format!("unknown key `images.{other}` ignored"),
                )),
            }
        }
        if deprecated_domains {
            self.renamed("images.domains", "images.remotePatterns");
            self.diagnostics.push(
                Diagnostic::warning(
                    "next-config-images-domains-deprecated",
                    "`images.domains` is deprecated; converted to remotePatterns (any protocol, any path)",
                )
                .with_hint("declare images.remotePatterns with protocol and pathname"),
            );
        }
    }

    fn rules_from_array(&mut self, key: &str, value: &Value) -> Vec<RouteRule> {
        let Some(items) = value.as_array() else {
            self.diagnostics.push(Diagnostic::warning(
                "next-config-rule-invalid",
                format!("`{key}` must be an array of rules"),
            ));
            return Vec::new();
        };
        let mut rules = Vec::new();
        for item in items {
            match route_rule_from(item) {
                Some(rule) => rules.push(rule),
                None => self.diagnostics.push(Diagnostic::warning(
                    "next-config-rule-invalid",
                    format!("`{key}` entry without a string `source` skipped"),
                )),
            }
        }
        rules
    }

    fn apply_redirects(&mut self, value: &Value) {
        if is_function_marker(value) {
            self.cfg.redirects_dynamic = true;
            self.dynamic("redirects");
            return;
        }
        let rules = self.rules_from_array("redirects", value);
        self.cfg.redirects.extend(rules);
    }

    fn apply_rewrites(&mut self, value: &Value) {
        if is_function_marker(value) {
            self.cfg.rewrites_dynamic = true;
            self.dynamic("rewrites");
            return;
        }
        match value {
            Value::Array(_) => {
                let rules = self.rules_from_array("rewrites", value);
                self.cfg.rewrites.after_files.extend(rules);
            }
            Value::Object(object) => {
                for (phase, key) in [
                    ("beforeFiles", "rewrites.beforeFiles"),
                    ("afterFiles", "rewrites.afterFiles"),
                    ("fallback", "rewrites.fallback"),
                ] {
                    if let Some(list) = object.get(phase) {
                        let rules = self.rules_from_array(key, list);
                        match phase {
                            "beforeFiles" => self.cfg.rewrites.before_files.extend(rules),
                            "afterFiles" => self.cfg.rewrites.after_files.extend(rules),
                            _ => self.cfg.rewrites.fallback.extend(rules),
                        }
                    }
                }
            }
            _ => self.diagnostics.push(Diagnostic::warning(
                "next-config-rule-invalid",
                "`rewrites` must return an array or { beforeFiles, afterFiles, fallback }",
            )),
        }
    }

    fn apply_headers(&mut self, value: &Value) {
        if is_function_marker(value) {
            self.cfg.headers_dynamic = true;
            self.dynamic("headers");
            return;
        }
        let Some(items) = value.as_array() else {
            self.diagnostics.push(Diagnostic::warning(
                "next-config-rule-invalid",
                "`headers` must be an array of rules",
            ));
            return;
        };
        for item in items {
            match header_rule_from(item) {
                Some(rule) => self.cfg.headers.push(rule),
                None => self.diagnostics.push(Diagnostic::warning(
                    "next-config-rule-invalid",
                    "`headers` entry without a string `source` skipped",
                )),
            }
        }
    }

    fn apply_compiler(&mut self, value: &Value) {
        let Some(object) = value.as_object() else {
            return;
        };
        let compiler = &mut self.cfg.compiler;
        for (key, v) in object {
            match key.as_str() {
                "removeConsole" => compiler.remove_console = Some(v.clone()),
                "reactRemoveProperties" => compiler.react_remove_properties = Some(v.clone()),
                "styledComponents" => compiler.styled_components = Some(v.clone()),
                "emotion" => compiler.emotion = Some(v.clone()),
                "relay" => compiler.relay = Some(v.clone()),
                "define" => compiler.define = string_map(v),
                "defineServer" => compiler.define_server = string_map(v),
                other => self.diagnostics.push(Diagnostic::info(
                    "next-config-unknown-key",
                    format!("unknown key `compiler.{other}` ignored"),
                )),
            }
        }
    }

    fn apply_i18n(&mut self, value: &Value) {
        let Some(object) = value.as_object() else {
            return;
        };
        self.cfg.i18n = Some(I18nConfig {
            locales: object.get("locales").map(string_list).unwrap_or_default(),
            default_locale: object
                .get("defaultLocale")
                .and_then(as_string)
                .unwrap_or_default(),
            domains: object
                .get("domains")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default(),
            locale_detection: object.get("localeDetection").and_then(Value::as_bool),
        });
        self.diagnostics.push(
            Diagnostic::info(
                "next-config-i18n-app-router",
                "`i18n` only applies to the Pages Router; the App Router has no built-in i18n",
            )
            .with_hint("route locales with the proxy and a `[lang]` segment"),
        );
    }

    fn apply_dev_indicators(&mut self, value: &Value) {
        let Some(object) = value.as_object() else {
            self.cfg.dev_indicators = Some(value.clone());
            return;
        };
        let mut kept = Map::new();
        for (key, v) in object {
            if matches!(
                key.as_str(),
                "appIsrStatus" | "buildActivity" | "buildActivityPosition"
            ) {
                self.removed(&format!("devIndicators.{key}"));
            } else {
                kept.insert(key.clone(), v.clone());
            }
        }
        self.cfg.dev_indicators = Some(Value::Object(kept));
    }

    fn apply_experimental(&mut self, value: &Value) {
        let Some(object) = value.as_object() else {
            return;
        };
        let mut kept = Map::new();
        for (key, v) in object {
            let from = format!("experimental.{key}");
            match key.as_str() {
                "turbo" => {
                    self.renamed(&from, "turbopack");
                    self.apply_turbopack(v, "experimental.turbo");
                }
                "serverComponentsExternalPackages" => {
                    self.renamed(&from, "serverExternalPackages");
                    for package in string_list(v) {
                        if !self.cfg.server_external_packages.contains(&package) {
                            self.cfg.server_external_packages.push(package);
                        }
                    }
                }
                "bundlePagesExternals" => {
                    self.renamed(&from, "bundlePagesRouterDependencies");
                    self.cfg.bundle_pages_router_dependencies |= is_truthy(v);
                }
                "typedRoutes" => {
                    self.renamed(&from, "typedRoutes");
                    self.cfg.typed_routes |= is_truthy(v);
                }
                "reactCompiler" => {
                    self.renamed(&from, "reactCompiler");
                    if self.cfg.react_compiler.is_none() {
                        self.cfg.react_compiler = Some(v.clone());
                    }
                }
                "cacheLife" => {
                    self.renamed(&from, "cacheLife");
                    for (name, profile) in cache_life_profiles(v) {
                        self.cfg.cache_life.entry(name).or_insert(profile);
                    }
                }
                "cacheHandlers" => {
                    self.renamed(&from, "cacheHandlers");
                    for (name, handler) in string_map(v) {
                        self.cfg.cache_handlers.entry(name).or_insert(handler);
                    }
                }
                "expireTime" => {
                    self.renamed(&from, "expireTime");
                    self.cfg.expire_time = self.cfg.expire_time.or_else(|| as_u64(v));
                }
                "adapterPath" => {
                    self.renamed(&from, "adapterPath");
                    self.cfg.adapter_path = self.cfg.adapter_path.take().or_else(|| as_string(v));
                }
                "dynamicIO" | "useCache" | "ppr" => {
                    self.renamed(&from, "cacheComponents");
                    self.cfg.cache_components |= is_truthy(v);
                }
                "outputFileTracingRoot" => {
                    self.renamed(&from, "outputFileTracingRoot");
                    self.cfg.output_file_tracing_root = self
                        .cfg
                        .output_file_tracing_root
                        .take()
                        .or_else(|| as_string(v));
                }
                "outputFileTracingIncludes" => {
                    self.renamed(&from, "outputFileTracingIncludes");
                    self.cfg
                        .output_file_tracing_includes
                        .get_or_insert_with(|| v.clone());
                }
                "outputFileTracingExcludes" => {
                    self.renamed(&from, "outputFileTracingExcludes");
                    self.cfg
                        .output_file_tracing_excludes
                        .get_or_insert_with(|| v.clone());
                }
                "skipMiddlewareUrlNormalize" => {
                    self.renamed(&from, "skipProxyUrlNormalize");
                    self.cfg.skip_proxy_url_normalize |= is_truthy(v);
                }
                "appDir" => self.removed(&from),
                "serverActions" if v.is_boolean() => self.removed(&from),
                _ => {
                    kept.insert(key.clone(), v.clone());
                }
            }
        }
        self.cfg.experimental = kept;
    }
}

/// Converts an already evaluated JSON object into a [`NextConfig`] (renames, removed keys).
///
/// Function-valued keys are expected as [`FUNCTION_MARKER`] strings when they could not be
/// evaluated; `redirects`/`rewrites`/`headers` then set their `*_dynamic` flag.
pub fn next_config_from_value(value: &Value) -> (NextConfig, Vec<Diagnostic>) {
    let mut builder = Builder { cfg: NextConfig::default(), diagnostics: Vec::new() };
    let Some(object) = value.as_object() else {
        builder.diagnostics.push(Diagnostic::error(
            "next-config-not-object",
            "next.config must evaluate to an object",
        ));
        return (builder.cfg, builder.diagnostics);
    };
    let mut experimental = None;
    for (key, v) in object {
        if key == "experimental" {
            experimental = Some(v);
        } else {
            builder.apply(key, v);
        }
    }
    if let Some(experimental) = experimental {
        builder.apply_experimental(experimental);
    }
    if builder.cfg.page_extensions.is_empty() {
        builder.cfg.page_extensions = DEFAULT_PAGE_EXTENSIONS
            .iter()
            .map(|e| (*e).to_owned())
            .collect();
    }
    (builder.cfg, builder.diagnostics)
}

// ============================================================================
// NextConfig -> YoloWebConfig
// ============================================================================

/// Built-in `cacheLife` profiles of Next.js 16.
pub fn builtin_cache_life() -> BTreeMap<String, CacheLifeProfile> {
    let profile = |stale: u64, revalidate: u64, expire: u64| CacheLifeProfile {
        stale: Some(stale),
        revalidate: Some(revalidate),
        expire: Some(expire),
    };
    [
        ("default", profile(300, 900, 4_294_967_294)),
        ("seconds", profile(30, 1, 60)),
        ("minutes", profile(300, 60, 3_600)),
        ("hours", profile(300, 3_600, 86_400)),
        ("days", profile(300, 86_400, 604_800)),
        ("weeks", profile(300, 604_800, 2_592_000)),
        ("max", profile(300, 2_592_000, 31_536_000)),
    ]
    .into_iter()
    .map(|(name, profile)| (name.to_owned(), profile))
    .collect()
}

/// Parses a `bodySizeLimit` (`"2mb"`, `"500kb"`, `1048576`) into bytes.
fn parse_byte_size(value: &Value) -> Option<u64> {
    match value {
        Value::Number(_) => as_u64(value),
        Value::String(text) => {
            let text = text.trim().to_ascii_lowercase();
            let split = text
                .find(|c: char| c.is_ascii_alphabetic())
                .unwrap_or(text.len());
            let (number, unit) = text.split_at(split);
            let number: f64 = number.trim().parse().ok()?;
            let multiplier: f64 = match unit.trim() {
                "" | "b" => 1.0,
                "kb" | "k" => 1024.0,
                "mb" | "m" => 1024.0 * 1024.0,
                "gb" | "g" => 1024.0 * 1024.0 * 1024.0,
                "tb" | "t" => 1024.0 * 1024.0 * 1024.0 * 1024.0,
                _ => return None,
            };
            Some((number * multiplier).round() as u64)
        }
        _ => None,
    }
}

fn merge_cache_life(
    cfg: &NextConfig,
    diagnostics: &mut Vec<Diagnostic>,
) -> BTreeMap<String, CacheLifeProfile> {
    let mut merged = builtin_cache_life();
    let builtin_default = merged.get("default").copied().unwrap_or_default();
    let custom_default = cfg.cache_life.get("default").copied().unwrap_or_default();
    let base = CacheLifeProfile {
        stale: custom_default.stale.or(builtin_default.stale),
        revalidate: custom_default.revalidate.or(builtin_default.revalidate),
        expire: custom_default.expire.or(builtin_default.expire),
    };
    for (name, profile) in &cfg.cache_life {
        let full = CacheLifeProfile {
            stale: profile.stale.or(base.stale),
            revalidate: profile.revalidate.or(base.revalidate),
            expire: profile.expire.or(base.expire),
        };
        if let (Some(revalidate), Some(expire)) = (full.revalidate, full.expire)
            && expire < revalidate
        {
            diagnostics.push(Diagnostic::warning(
                "next-config-cache-life-invalid",
                format!("cacheLife.{name}: expire ({expire}s) is below revalidate ({revalidate}s)"),
            ));
        }
        merged.insert(name.clone(), full);
    }
    merged
}

fn feature(name: impl Into<String>, support: Support, note: impl Into<String>) -> FeatureStatus {
    FeatureStatus { feature: name.into(), support, note: note.into(), target: None }
}

fn mapped_features(cfg: &NextConfig, output: OutputMode, out: &mut Vec<FeatureStatus>) {
    let mut mapped = |name: &str, present: bool, note: &str| {
        if present {
            out.push(feature(name, Support::Mapped, note));
        }
    };
    mapped(
        "reactStrictMode",
        cfg.react_strict_mode.is_some(),
        "React <StrictMode> wrapper",
    );
    mapped(
        "basePath",
        !cfg.base_path.is_empty(),
        "prefixes routes, static routes and rules",
    );
    mapped(
        "assetPrefix",
        !cfg.asset_prefix.is_empty(),
        "CDN origin for hashed assets",
    );
    mapped(
        "trailingSlash",
        cfg.trailing_slash,
        "Bun.serve normalises trailing slashes",
    );
    mapped(
        "compress",
        cfg.compress.is_some(),
        "gzip/zstd negotiated by the Bun server",
    );
    mapped(
        "poweredByHeader",
        cfg.powered_by_header.is_some(),
        "x-powered-by: Yolo",
    );
    mapped(
        "generateEtags",
        cfg.generate_etags.is_some(),
        "ETag on dynamic responses",
    );
    mapped("env", !cfg.env.is_empty(), "inlined with Bun.build define");
    mapped(
        "images.remotePatterns",
        !cfg.images.remote_patterns.is_empty(),
        "optimizer allow-list",
    );
    mapped(
        "images.localPatterns",
        cfg.images.local_patterns.is_some(),
        "optimizer allow-list",
    );
    mapped(
        "images.formats",
        cfg.images.formats != ImagesConfig::default().formats,
        "Accept negotiation",
    );
    mapped(
        "images.deviceSizes",
        cfg.images.device_sizes != ImagesConfig::default().device_sizes,
        "srcset breakpoints",
    );
    mapped(
        "images.imageSizes",
        cfg.images.image_sizes != ImagesConfig::default().image_sizes,
        "srcset widths",
    );
    mapped(
        "images.qualities",
        cfg.images.qualities != ImagesConfig::default().qualities,
        "allowed q values",
    );
    mapped(
        "images.minimumCacheTTL",
        cfg.images.minimum_cache_ttl != 14_400,
        "optimizer Cache-Control",
    );
    mapped(
        "images.unoptimized",
        cfg.images.unoptimized,
        "<Image> passes src through",
    );
    mapped(
        "images.loader",
        cfg.images.loader != "default",
        "loader URL builder",
    );
    mapped(
        "redirects",
        !cfg.redirects.is_empty(),
        "static manifest (path-to-regexp subset)",
    );
    let any_rewrite = !cfg.rewrites.before_files.is_empty()
        || !cfg.rewrites.after_files.is_empty()
        || !cfg.rewrites.fallback.is_empty();
    mapped(
        "rewrites",
        any_rewrite,
        "beforeFiles/afterFiles/fallback phases",
    );
    mapped("headers", !cfg.headers.is_empty(), "static manifest");
    mapped(
        "cacheLife",
        !cfg.cache_life.is_empty(),
        "profiles merged with the built-ins",
    );
    mapped(
        "cacheComponents",
        cfg.cache_components,
        "use cache + Suspense boundaries",
    );
    mapped("typedRoutes", cfg.typed_routes, "routeTree.gen.ts typing");
    mapped(
        "transpilePackages",
        !cfg.transpile_packages.is_empty(),
        "Bun bundles workspace packages",
    );
    mapped(
        "serverExternalPackages",
        !cfg.server_external_packages.is_empty(),
        "Bun.build external",
    );
    mapped(
        "deploymentId",
        cfg.deployment_id.is_some(),
        "?dpl= skew protection",
    );
    mapped(
        "pageExtensions",
        cfg.page_extensions != DEFAULT_PAGE_EXTENSIONS,
        "route scanner extensions",
    );
    match output {
        OutputMode::Standalone => out.push(feature(
            "output: standalone",
            Support::Mapped,
            "bun build --compile",
        )),
        OutputMode::Export => out.push(feature("output: export", Support::Mapped, "static export")),
        OutputMode::Server => out.push(feature(
            "output",
            Support::Mapped,
            "Bun.serve server (default)",
        )),
    }
}

fn partial_features(cfg: &NextConfig, out: &mut Vec<FeatureStatus>) {
    let mut partial = |name: &str, present: bool, note: &str| {
        if present {
            out.push(feature(name, Support::Partial, note));
        }
    };
    partial(
        "reactCompiler",
        cfg.react_compiler.as_ref().is_some_and(is_truthy),
        "Bun.build reactCompiler",
    );
    partial(
        "redirects (dynamic)",
        cfg.redirects_dynamic,
        "awaited at startup; no static manifest",
    );
    partial(
        "rewrites (dynamic)",
        cfg.rewrites_dynamic,
        "awaited at startup; no static manifest",
    );
    partial(
        "headers (dynamic)",
        cfg.headers_dynamic,
        "awaited at startup; no static manifest",
    );
    partial(
        "experimental.staleTimes",
        cfg.experimental.contains_key("staleTimes"),
        "TanStack Query staleTime per route kind",
    );
    partial(
        "experimental.optimizePackageImports",
        cfg.experimental.contains_key("optimizePackageImports"),
        "Bun barrel optimization",
    );
    partial(
        "i18n",
        cfg.i18n.is_some(),
        "Pages Router only; App Router uses proxy + [lang]",
    );
    partial(
        "logging",
        cfg.logging.is_some(),
        "fetch logging via Bun server hooks",
    );
    partial(
        "compiler.removeConsole",
        cfg.compiler.remove_console.is_some(),
        "Bun `drop` (console.*)",
    );
    partial(
        "compiler.define",
        !cfg.compiler.define.is_empty() || !cfg.compiler.define_server.is_empty(),
        "Bun define",
    );
    partial(
        "compiler.styledComponents",
        cfg.compiler.styled_components.is_some(),
        "no SWC transform; runtime styled-components only",
    );
    partial(
        "compiler.emotion",
        cfg.compiler.emotion.is_some(),
        "no SWC transform; babel plugin not applied",
    );
    partial(
        "compiler.relay",
        cfg.compiler.relay.is_some(),
        "run relay-compiler before bun build",
    );
    partial(
        "devIndicators",
        cfg.dev_indicators.is_some(),
        "Yolo dev overlay",
    );
    partial(
        "experimental (other keys)",
        !cfg.experimental.is_empty(),
        "kept raw; no runtime effect",
    );
}

fn unsupported_features(cfg: &NextConfig, out: &mut Vec<FeatureStatus>) {
    let mut unsupported = |name: &str, present: bool, note: &str| {
        if present {
            out.push(feature(name, Support::Unsupported, note));
        }
    };
    unsupported(
        "webpack",
        cfg.webpack_present,
        "Turbopack-only; use turbopack.rules and Bun plugins",
    );
    unsupported(
        "exportPathMap",
        cfg.export_path_map_present,
        "App Router manifest drives export",
    );
    unsupported(
        "amp",
        cfg.removed_keys.iter().any(|k| k == "amp"),
        "AMP support removed in Next.js 16",
    );
    unsupported(
        "eslint",
        cfg.removed_keys.iter().any(|k| k == "eslint"),
        "next lint removed; run oxlint/biome",
    );
    unsupported(
        "sassOptions",
        cfg.sass_options.is_some(),
        "Bun has no Sass; precompile or use Lightning CSS",
    );
    unsupported(
        "modularizeImports",
        !cfg.modularize_imports.is_empty(),
        "use optimizePackageImports",
    );
    unsupported(
        "experimental.urlImports",
        cfg.experimental.contains_key("urlImports"),
        "URL imports are not bundled; vendor the modules",
    );
    unsupported(
        "cacheHandler",
        cfg.cache_handler.is_some(),
        "legacy cacheHandler class; use cacheHandlers",
    );
    unsupported(
        "adapterPath",
        cfg.adapter_path.is_some(),
        "deployment adapters are Next-specific",
    );
    unsupported(
        "serverRuntimeConfig / publicRuntimeConfig",
        cfg.removed_keys
            .iter()
            .any(|k| k == "serverRuntimeConfig" || k == "publicRuntimeConfig"),
        "use env / NEXT_PUBLIC_ variables",
    );
}

fn turbopack_features(cfg: &NextConfig, out: &mut Vec<FeatureStatus>) {
    for (glob, rules) in &cfg.turbopack.rules {
        for loader in rules.iter().flat_map(|rule| rule.loaders.iter()) {
            let (_, _, support) = crate::turbopack::map_turbopack_loader(&loader.loader);
            let note = match support {
                Support::Mapped => format!("Bun plugin for {glob}"),
                Support::Partial => format!("Bun plugin for {glob} with caveats"),
                Support::Unsupported => format!("no Bun plugin for {glob}"),
            };
            out.push(feature(
                format!("turbopack.rules {}", loader.loader),
                support,
                note,
            ));
        }
    }
    if !cfg.turbopack.resolve_alias.is_empty() {
        out.push(feature(
            "turbopack.resolveAlias",
            Support::Mapped,
            "Bun onResolve alias plugin",
        ));
    }
    if !cfg.turbopack.resolve_extensions.is_empty() {
        out.push(feature(
            "turbopack.resolveExtensions",
            Support::Mapped,
            "Bun resolve order",
        ));
    }
}

/// Converts a [`NextConfig`] into the normalized [`YoloWebConfig`] with a migration report.
pub fn next_config_to_yolo(
    cfg: &NextConfig,
    app_dir: &str,
    pages_dir: Option<&str>,
    proxy_file: Option<&str>,
) -> (YoloWebConfig, ConfigMigrationReport) {
    let mut report = ConfigMigrationReport::default();
    let server_actions = cfg
        .experimental
        .get("serverActions")
        .and_then(Value::as_object);
    let action_body_size_limit = server_actions
        .and_then(|sa| sa.get("bodySizeLimit"))
        .and_then(parse_byte_size)
        .unwrap_or(1_048_576);
    let action_allowed_origins = server_actions
        .and_then(|sa| sa.get("allowedOrigins"))
        .map(string_list)
        .unwrap_or_default();
    let server = YoloServerConfig {
        port: 3000,
        hostname: String::from("0.0.0.0"),
        hmr: true,
        browser_console: true,
        compress: cfg.compress.unwrap_or(true),
        powered_by_header: cfg.powered_by_header.unwrap_or(true),
        generate_etags: cfg.generate_etags.unwrap_or(true),
        action_body_size_limit,
        action_allowed_origins,
    };
    let react = YoloReactConfig {
        strict_mode: cfg.react_strict_mode != Some(false),
        compiler: cfg.react_compiler.as_ref().is_some_and(is_truthy),
        fast_refresh: true,
        cache_components: cfg.cache_components,
        typed_routes: cfg.typed_routes,
    };
    let cdn = crate::cdn::cdn_plan(cfg);
    let output = cdn.output;
    let images = if cfg.images.unoptimized {
        None
    } else {
        Some(crate::images::optimizer_policy(&cfg.images, &cfg.base_path))
    };
    let bundler = crate::turbopack::translate_turbopack(cfg);
    let cache_life = merge_cache_life(cfg, &mut report.diagnostics);

    mapped_features(cfg, output, &mut report.features);
    partial_features(cfg, &mut report.features);
    unsupported_features(cfg, &mut report.features);
    turbopack_features(cfg, &mut report.features);
    for extra in &bundler.unsupported {
        if !report.features.iter().any(|f| f.feature == extra.feature) {
            report.features.push(extra.clone());
        }
    }

    let yolo = YoloWebConfig {
        app_dir: app_dir.to_owned(),
        pages_dir: pages_dir.map(str::to_owned),
        dist_dir: cfg
            .dist_dir
            .clone()
            .filter(|d| !d.is_empty() && d != ".next")
            .unwrap_or_else(|| String::from(".yolo")),
        output,
        server,
        react,
        theme: YoloThemeConfig::default(),
        cdn,
        images,
        bundler,
        redirects: cfg.redirects.clone(),
        rewrites: cfg.rewrites.clone(),
        headers: cfg.headers.clone(),
        public_env_prefixes: vec![String::from("NEXT_PUBLIC_"), String::from("YOLO_PUBLIC_")],
        env: cfg.env.clone(),
        cache_life,
        i18n: cfg.i18n.clone(),
        transpile_packages: cfg.transpile_packages.clone(),
        server_external_packages: cfg.server_external_packages.clone(),
        page_extensions: cfg.page_extensions.clone(),
        trailing_slash: cfg.trailing_slash,
        proxy_file: proxy_file.map(str::to_owned),
    };
    (yolo, report)
}

/// Serializes a [`YoloWebConfig`] as `yolo.toml`.
pub fn yolo_config_toml(cfg: &YoloWebConfig) -> String {
    match toml::to_string_pretty(cfg) {
        Ok(body) => {
            format!("# Generated by yolo-next from next.config (Next.js 16 -> Yolo)\n{body}")
        }
        Err(err) => format!("# yolo.toml could not be serialized: {err}\n"),
    }
}

fn is_js_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_' || c == '$')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$')
}

/// Rewrites quoted `"${env:X}"` / `"${env:X|default}"` JSON strings into
/// `process.env.X` / `process.env.X ?? "default"` expressions.
fn env_markers_to_ts(json: &str) -> String {
    const OPEN: &str = "\"${env:";
    let mut out = String::with_capacity(json.len());
    let mut rest = json;
    while let Some(start) = rest.find(OPEN) {
        out.push_str(&rest[..start]);
        let after = &rest[start + OPEN.len()..];
        let replaced = after.find("}\"").and_then(|end| {
            let body = &after[..end];
            if body.contains('"') || body.contains('\\') {
                return None;
            }
            let (name, default) = body
                .split_once('|')
                .map_or((body, None), |(n, d)| (n, Some(d)));
            if name.is_empty() || name.contains(|c: char| c.is_whitespace() || c == '}') {
                return None;
            }
            let access = if is_js_identifier(name) {
                format!("process.env.{name}")
            } else {
                format!("process.env[\"{name}\"]")
            };
            let expr = match default {
                Some(default) => format!("{access} ?? \"{default}\""),
                None => access,
            };
            Some((expr, end + 2))
        });
        if let Some((expr, consumed)) = replaced {
            out.push_str(&expr);
            rest = &after[consumed..];
        } else {
            out.push_str(OPEN);
            rest = after;
        }
    }
    out.push_str(rest);
    out
}

/// Serializes a [`YoloWebConfig`] as `yolo.config.ts` (`defineYoloConfig({...})`).
pub fn yolo_config_ts(cfg: &YoloWebConfig) -> String {
    let json = serde_json::to_string_pretty(cfg).unwrap_or_else(|_| String::from("{}"));
    format!(
        "// Generated by yolo-next\nimport {{ defineYoloConfig }} from \"@yolo-rs/next\";\n\nexport default defineYoloConfig({} as const);\n",
        env_markers_to_ts(&json)
    )
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::types::Severity;

    const FIXTURE_PATH: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../apps/next-fixture/next.config.ts"
    );

    const INLINE_FIXTURE: &str = r#"import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  reactStrictMode: true,
  output: "standalone",
  basePath: "",
  assetPrefix: process.env.CDN_URL ?? "",
  trailingSlash: false,
  compress: true,
  poweredByHeader: false,
  reactCompiler: true,
  cacheComponents: true,
  typedRoutes: true,
  deploymentId: process.env.DEPLOYMENT_ID,
  transpilePackages: ["@acme/ui"],
  serverExternalPackages: ["sharp", "pino"],
  pageExtensions: ["tsx", "ts", "mdx"],
  env: { APP_NAME: "Yolo Fixture", API_BASE: "https://api.example.com" },
  turbopack: {
    root: process.cwd(),
    rules: {
      "*.svg": { loaders: ["@svgr/webpack"], as: "*.js" },
      "*.md": { loaders: ["raw-loader"], as: "*.js" },
      "*.graphql": { loaders: ["graphql-tag/loader"], as: "*.js" },
      "*.yaml": { loaders: ["yaml-loader"], as: "*.js" },
    },
    resolveAlias: {
      underscore: "lodash",
      mocha: { browser: "mocha/browser-entry.js" },
      fs: { browser: "./empty.ts" },
      "@/*": "./*",
    },
    resolveExtensions: [".mdx", ".tsx", ".ts", ".jsx", ".js", ".mjs", ".json"],
  },
  images: {
    formats: ["image/avif", "image/webp"],
    deviceSizes: [640, 750, 828, 1080, 1200, 1920, 2048, 3840],
    imageSizes: [32, 48, 64, 96, 128, 256, 384],
    qualities: [50, 75, 90],
    minimumCacheTTL: 86400,
    dangerouslyAllowSVG: false,
    remotePatterns: [
      { protocol: "https", hostname: "images.unsplash.com", pathname: "/**" },
      { protocol: "https", hostname: "**.cloudfront.net" },
      { protocol: "https", hostname: "cdn.example.com", port: "", pathname: "/media/**", search: "" },
    ],
    localPatterns: [{ pathname: "/assets/images/**", search: "" }],
  },
  async redirects() {
    return [
      { source: "/old-blog/:slug", destination: "/blog/:slug", permanent: true },
      { source: "/docs", destination: "/docs/getting-started", permanent: false },
      {
        source: "/legacy/:path*",
        destination: "https://legacy.example.com/:path*",
        permanent: false,
        has: [{ type: "header", key: "x-legacy", value: "1" }],
      },
    ];
  },
  async rewrites() {
    return {
      beforeFiles: [{ source: "/api/proxy/:path*", destination: "https://api.example.com/:path*" }],
      afterFiles: [{ source: "/u/:username", destination: "/users/:username" }],
      fallback: [{ source: "/:path*", destination: "https://fallback.example.com/:path*" }],
    };
  },
  async headers() {
    return [
      {
        source: "/(.*)",
        headers: [
          { key: "X-Frame-Options", value: "DENY" },
          { key: "Strict-Transport-Security", value: "max-age=63072000; includeSubDomains; preload" },
        ],
      },
      {
        source: "/_next/static/:path*",
        headers: [{ key: "Cache-Control", value: "public, max-age=31536000, immutable" }],
      },
    ];
  },
  compiler: {
    removeConsole: { exclude: ["error", "warn"] },
    define: { __BUILD_FLAG__: "true" },
  },
  cacheLife: { blog: { stale: 3600, revalidate: 900, expire: 86400 } },
  experimental: {
    serverActions: { bodySizeLimit: "2mb", allowedOrigins: ["app.example.com"] },
    staleTimes: { dynamic: 30, static: 180 },
    optimizePackageImports: ["lucide-react"],
    authInterrupts: true,
    viewTransition: true,
  },
  // Removed in Next.js 16 (next lint is gone); kept so the migrator reports it.
  eslint: { ignoreDuringBuilds: true },
  logging: { fetches: { fullUrl: true } },
};

export default nextConfig;
"#;

    fn fixture_source() -> String {
        std::fs::read_to_string(FIXTURE_PATH).unwrap_or_else(|_| INLINE_FIXTURE.to_owned())
    }

    fn parse(source: &str, name: &str) -> (NextConfig, Vec<Diagnostic>) {
        parse_next_config(source, name).unwrap()
    }

    #[test]
    fn parses_realistic_fixture() {
        let (cfg, diagnostics) = parse(&fixture_source(), "next.config.ts");
        assert_eq!(cfg.react_strict_mode, Some(true));
        assert_eq!(cfg.output.as_deref(), Some("standalone"));
        assert_eq!(cfg.asset_prefix, "${env:CDN_URL|}");
        assert_eq!(cfg.deployment_id.as_deref(), Some("${env:DEPLOYMENT_ID}"));
        assert_eq!(cfg.powered_by_header, Some(false));
        assert!(cfg.cache_components && cfg.typed_routes);
        assert_eq!(cfg.react_compiler, Some(Value::Bool(true)));
        assert_eq!(cfg.transpile_packages, vec![String::from("@acme/ui")]);
        assert_eq!(
            cfg.server_external_packages,
            vec![String::from("sharp"), String::from("pino")]
        );
        assert_eq!(cfg.page_extensions, vec!["tsx", "ts", "mdx"]);
        assert_eq!(
            cfg.env.get("APP_NAME").map(String::as_str),
            Some("Yolo Fixture")
        );

        assert_eq!(cfg.turbopack.root.as_deref(), Some("."));
        assert_eq!(cfg.turbopack.rules.len(), 4);
        let svg = &cfg.turbopack.rules["*.svg"][0];
        assert_eq!(svg.loaders[0].loader, "@svgr/webpack");
        assert_eq!(svg.as_.as_deref(), Some("*.js"));
        assert_eq!(
            cfg.turbopack.resolve_alias["underscore"],
            Value::String(String::from("lodash"))
        );
        assert_eq!(
            cfg.turbopack.resolve_alias["mocha"]["browser"],
            "mocha/browser-entry.js"
        );
        assert_eq!(cfg.turbopack.resolve_extensions.len(), 7);

        assert_eq!(cfg.images.formats, vec!["image/avif", "image/webp"]);
        assert_eq!(cfg.images.qualities, vec![50, 75, 90]);
        assert_eq!(cfg.images.minimum_cache_ttl, 86_400);
        assert_eq!(cfg.images.remote_patterns.len(), 3);
        assert_eq!(cfg.images.remote_patterns[1].hostname, "**.cloudfront.net");
        assert_eq!(cfg.images.remote_patterns[2].port.as_deref(), Some(""));
        assert_eq!(
            cfg.images.local_patterns.as_ref().unwrap()[0]
                .pathname
                .as_deref(),
            Some("/assets/images/**")
        );

        assert_eq!(cfg.redirects.len(), 3);
        assert_eq!(cfg.redirects[0].permanent, Some(true));
        assert_eq!(cfg.redirects[2].has[0].kind, "header");
        assert_eq!(cfg.redirects[2].has[0].value.as_deref(), Some("1"));
        assert!(!cfg.redirects_dynamic);
        assert_eq!(cfg.rewrites.before_files.len(), 1);
        assert_eq!(cfg.rewrites.after_files[0].destination, "/users/:username");
        assert_eq!(cfg.rewrites.fallback.len(), 1);
        assert_eq!(cfg.headers.len(), 2);
        assert_eq!(cfg.headers[0].headers[1].key, "Strict-Transport-Security");

        assert_eq!(
            cfg.compiler
                .define
                .get("__BUILD_FLAG__")
                .map(String::as_str),
            Some("true")
        );
        assert_eq!(
            cfg.compiler.remove_console.as_ref().unwrap()["exclude"][0],
            "error"
        );
        assert_eq!(cfg.cache_life["blog"].revalidate, Some(900));
        assert_eq!(cfg.experimental["serverActions"]["bodySizeLimit"], "2mb");
        assert!(cfg.experimental.contains_key("staleTimes"));
        assert_eq!(cfg.removed_keys, vec![String::from("eslint")]);
        assert!(cfg.logging.is_some());
        assert_eq!(cfg.wrappers, Vec::<String>::new());
        assert!(
            diagnostics
                .iter()
                .any(|d| d.code == "next-config-removed-key" && d.message.contains("eslint"))
        );
        assert!(
            diagnostics
                .iter()
                .all(|d| d.file.as_deref() == Some("next.config.ts"))
        );
        assert!(!diagnostics.iter().any(|d| d.severity == Severity::Error));
    }

    #[test]
    fn fixture_converts_to_yolo_config() {
        let (cfg, _) = parse(&fixture_source(), "next.config.ts");
        let (yolo, report) = next_config_to_yolo(&cfg, "app", Some("pages"), Some("proxy.ts"));
        assert_eq!(yolo.server.action_body_size_limit, 2_097_152);
        assert_eq!(
            yolo.server.action_allowed_origins,
            vec![String::from("app.example.com")]
        );
        assert!(!yolo.server.powered_by_header);
        assert!(yolo.server.compress);
        assert!(
            yolo.react.strict_mode
                && yolo.react.compiler
                && yolo.react.cache_components
                && yolo.react.typed_routes
        );
        assert_eq!(yolo.output, OutputMode::Standalone);
        assert_eq!(yolo.dist_dir, ".yolo");
        assert_eq!(yolo.app_dir, "app");
        assert_eq!(yolo.pages_dir.as_deref(), Some("pages"));
        assert_eq!(yolo.proxy_file.as_deref(), Some("proxy.ts"));
        assert_eq!(yolo.cache_life.len(), 8);
        assert_eq!(yolo.cache_life["blog"].stale, Some(3600));
        assert_eq!(yolo.cache_life["default"].expire, Some(4_294_967_294));
        assert!(yolo.images.is_some());
        assert_eq!(yolo.bundler.plugins.len(), 5, "alias plugin + 4 rules");
        assert_eq!(yolo.redirects.len(), 3);
        assert_eq!(yolo.public_env_prefixes[0], "NEXT_PUBLIC_");
        assert!(
            report
                .features
                .iter()
                .any(|f| f.feature == "output: standalone" && f.support == Support::Mapped)
        );
        assert!(
            report
                .features
                .iter()
                .any(|f| f.feature == "reactCompiler" && f.support == Support::Partial)
        );
        assert!(
            report
                .features
                .iter()
                .any(|f| f.feature == "eslint" && f.support == Support::Unsupported)
        );
        assert!(
            report
                .features
                .iter()
                .any(|f| f.feature == "turbopack.rules @svgr/webpack"
                    && f.support == Support::Mapped)
        );
        assert!(
            report
                .features
                .iter()
                .any(|f| f.feature == "turbopack.rules graphql-tag/loader"
                    && f.support == Support::Partial)
        );

        let toml_text = yolo_config_toml(&yolo);
        assert!(
            toml_text.starts_with("# Generated by yolo-next"),
            "{toml_text}"
        );
        assert!(
            toml_text.contains("actionBodySizeLimit = 2097152"),
            "{toml_text}"
        );
        assert!(toml_text.contains("[cdn]"));
        let ts = yolo_config_ts(&yolo);
        assert!(ts.starts_with("// Generated by yolo-next\nimport { defineYoloConfig } from \"@yolo-rs/next\";\n\nexport default defineYoloConfig({"));
        assert!(ts.ends_with("} as const);\n"));
        assert!(
            ts.contains("\"assetPrefix\": process.env.CDN_URL ?? \"\""),
            "{ts}"
        );
        assert!(
            ts.contains("\"deploymentId\": process.env.DEPLOYMENT_ID"),
            "{ts}"
        );
        assert!(!ts.contains("${env:"));
    }

    #[test]
    fn module_exports_with_require_and_wrappers() {
        let source = r#"
const withBundleAnalyzer = require("@next/bundle-analyzer")({ enabled: process.env.ANALYZE === "true" });
const { withSentryConfig } = require("@sentry/nextjs");
/** @type {import('next').NextConfig} */
const nextConfig = {
  reactStrictMode: false,
  images: { domains: ["a.com", "b.com"] },
  experimental: { serverComponentsExternalPackages: ["pg"], turbo: { resolveExtensions: [".ts"] }, ppr: "incremental", appDir: true },
  skipMiddlewareUrlNormalize: true,
  webpack(config) { return config; },
  publicRuntimeConfig: { x: 1 },
  unknownThing: 42,
};
module.exports = withBundleAnalyzer(withSentryConfig(nextConfig, { silent: true }));
"#;
        let (cfg, diagnostics) = parse(source, "next.config.js");
        assert_eq!(
            cfg.wrappers,
            vec![
                String::from("withBundleAnalyzer"),
                String::from("withSentryConfig")
            ]
        );
        assert_eq!(cfg.react_strict_mode, Some(false));
        assert_eq!(cfg.images.remote_patterns.len(), 2);
        assert_eq!(cfg.images.remote_patterns[0].hostname, "a.com");
        assert!(cfg.images.remote_patterns[0].protocol.is_none());
        assert_eq!(cfg.server_external_packages, vec![String::from("pg")]);
        assert_eq!(cfg.turbopack.resolve_extensions, vec![String::from(".ts")]);
        assert!(cfg.cache_components);
        assert!(cfg.skip_proxy_url_normalize);
        assert!(cfg.webpack_present);
        assert!(cfg.experimental.is_empty());
        assert!(
            cfg.removed_keys
                .contains(&String::from("publicRuntimeConfig"))
        );
        assert!(
            cfg.removed_keys
                .contains(&String::from("experimental.appDir"))
        );
        assert_eq!(cfg.extra["unknownThing"], 42);
        let renamed: Vec<&str> = cfg
            .renamed_keys
            .iter()
            .map(|(from, _)| from.as_str())
            .collect();
        assert!(renamed.contains(&"experimental.turbo"));
        assert!(renamed.contains(&"experimental.serverComponentsExternalPackages"));
        assert!(renamed.contains(&"experimental.ppr"));
        assert!(renamed.contains(&"skipMiddlewareUrlNormalize"));
        assert!(renamed.contains(&"images.domains"));
        assert!(diagnostics.iter().any(|d| d.code == "next-config-wrapper"));
        assert!(
            diagnostics
                .iter()
                .any(|d| d.code == "next-config-webpack-unsupported"
                    && d.severity == Severity::Error)
        );
        assert!(
            diagnostics
                .iter()
                .any(|d| d.code == "next-config-images-domains-deprecated")
        );
        assert!(
            diagnostics
                .iter()
                .any(|d| d.code == "next-config-unknown-key")
        );
    }

    #[test]
    fn phase_function_and_dynamic_redirects() {
        let source = r#"
export default async (phase, { defaultConfig }) => {
  const config = {
    ...defaultConfig,
    distDir: "build",
    basePath: "/app",
    async redirects() {
      const res = await fetch("https://example.com/redirects.json");
      return res.json();
    },
    rewrites: async () => [{ source: "/a", destination: "/b" }],
    headers: () => [{ source: "/x", headers: [{ key: "a", value: "b" }] }],
    generateBuildId: async () => "build-123",
    exportPathMap: async function () { return {}; },
  };
  return config;
};
"#;
        let (cfg, diagnostics) = parse(source, "next.config.mjs");
        assert_eq!(cfg.dist_dir.as_deref(), Some("build"));
        assert_eq!(cfg.base_path, "/app");
        assert!(cfg.redirects_dynamic);
        assert_eq!(cfg.redirects, Vec::<RouteRule>::new());
        assert!(!cfg.rewrites_dynamic);
        assert_eq!(cfg.rewrites.after_files.len(), 1);
        assert_eq!(cfg.headers.len(), 1);
        assert!(cfg.generate_build_id_present);
        assert!(cfg.export_path_map_present);
        assert!(
            diagnostics
                .iter()
                .any(|d| d.code == "next-config-phase-function")
        );
        let dynamic = diagnostics
            .iter()
            .find(|d| d.code == "next-config-dynamic-redirects")
            .unwrap();
        assert_eq!(dynamic.severity, Severity::Warning);
        assert!(
            dynamic
                .hint
                .as_deref()
                .unwrap()
                .contains("awaited at startup")
        );
    }

    #[test]
    fn define_config_and_satisfies() {
        let source = r#"
import { defineConfig } from "next";
export default defineConfig({
  output: "export",
  trailingSlash: true,
  expireTime: 3600,
  images: { unoptimized: true },
} satisfies import("next").NextConfig);
"#;
        let (cfg, _) = parse(source, "next.config.ts");
        assert_eq!(cfg.wrappers, Vec::<String>::new());
        assert_eq!(cfg.output.as_deref(), Some("export"));
        assert!(cfg.trailing_slash);
        assert_eq!(cfg.expire_time, Some(3600));
        assert!(cfg.images.unoptimized);
        let (yolo, _) = next_config_to_yolo(&cfg, "src/app", None, None);
        assert_eq!(yolo.output, OutputMode::Export);
        assert!(yolo.images.is_none());
    }

    #[test]
    fn expression_evaluation_rules() {
        let value = object_literal_to_json(
            r#"{
              a: "x", b: 1.5, c: true, d: null, e: [1, "two", , 3],
              f: `tpl`, g: process.env.FOO, h: process.env.BAR ?? "d", i: process.env.BAZ || 42,
              j: new URL("https://x.example/a"), k: new RegExp("a+", "i"), l: /b*/g,
              m: -1, n: !0, o: !1, p: __dirname, q: process.cwd(), r: import.meta.dirname,
              s: ("paren" as const), t: undefined, u: someUnknownIdentifier, v: `has ${1}`,
              w: { [computed]: 1, nested: { deep: [true] } }, x: () => 1, y: 2 + 3, z: "a" + "b",
              "quoted-key": 1, 42: "num", fn: function () { return compute(); },
            }"#,
        )
        .unwrap();
        assert_eq!(value["a"], "x");
        assert_eq!(value["b"], 1.5);
        assert_eq!(value["c"], true);
        assert_eq!(value["d"], Value::Null);
        assert_eq!(value["e"], serde_json::json!([1, "two", null, 3]));
        assert_eq!(value["f"], "tpl");
        assert_eq!(value["g"], "${env:FOO}");
        assert_eq!(value["h"], "${env:BAR|d}");
        assert_eq!(value["i"], "${env:BAZ|42}");
        assert_eq!(value["j"], "https://x.example/a");
        assert_eq!(value["k"], "a+");
        assert_eq!(value["l"], "b*");
        assert_eq!(value["m"], -1);
        assert_eq!(value["n"], true);
        assert_eq!(value["o"], false);
        assert_eq!(value["p"], ".");
        assert_eq!(value["q"], ".");
        assert_eq!(value["r"], ".");
        assert_eq!(value["s"], "paren");
        assert!(value.get("t").is_none());
        assert!(value.get("u").is_none());
        assert!(value.get("v").is_none());
        assert_eq!(value["w"]["nested"]["deep"][0], true);
        assert!(value["w"].get("computed").is_none());
        assert_eq!(
            value["x"], 1,
            "literal arrow bodies evaluate to their value"
        );
        assert_eq!(value["fn"], FUNCTION_MARKER);
        assert_eq!(value["y"], 5);
        assert_eq!(value["z"], "ab");
        assert_eq!(value["quoted-key"], 1);
        assert_eq!(value["42"], "num");
        assert!(object_literal_to_json("{ a: ").is_err());
    }

    #[test]
    fn spread_and_identifier_resolution() {
        let source = r#"
const shared = { compress: false, poweredByHeader: false };
const extensions = ["ts", "tsx"];
const nextConfig = { ...shared, pageExtensions: extensions, ...unknownSpread };
export { nextConfig as default };
"#;
        let (cfg, diagnostics) = parse(source, "next.config.mjs");
        assert_eq!(cfg.compress, Some(false));
        assert_eq!(cfg.powered_by_header, Some(false));
        assert_eq!(cfg.page_extensions, vec!["ts", "tsx"]);
        assert!(
            diagnostics
                .iter()
                .any(|d| d.code == "next-config-spread-unsupported"
                    && d.severity == Severity::Warning)
        );
    }

    #[test]
    fn parse_errors_and_missing_export() {
        assert!(parse_next_config("export default {", "next.config.ts").is_err());
        let err = parse_next_config("const a = 1;", "next.config.ts").unwrap_err();
        assert_eq!(err[0].code, "next-config-no-export");
        let err =
            parse_next_config("export default someFunctionCall();", "next.config.ts").unwrap_err();
        assert!(err.iter().any(|d| d.code == "next-config-unresolvable"));
    }

    #[test]
    fn from_value_handles_markers_and_dev_indicators() {
        let value = serde_json::json!({
            "redirects": FUNCTION_MARKER,
            "rewrites": [{ "source": "/a", "destination": "/b" }],
            "devIndicators": { "buildActivity": true, "position": "bottom-right" },
            "i18n": { "locales": ["en", "fr"], "defaultLocale": "en", "localeDetection": false },
            "experimental": { "dynamicIO": true, "serverActions": true, "esmExternals": "loose" },
            "amp": {},
        });
        let (cfg, diagnostics) = next_config_from_value(&value);
        assert!(cfg.redirects_dynamic);
        assert_eq!(cfg.rewrites.after_files.len(), 1);
        assert_eq!(
            cfg.dev_indicators.as_ref().unwrap()["position"],
            "bottom-right"
        );
        assert!(
            cfg.removed_keys
                .contains(&String::from("devIndicators.buildActivity"))
        );
        assert!(
            cfg.removed_keys
                .contains(&String::from("experimental.serverActions"))
        );
        assert!(cfg.removed_keys.contains(&String::from("amp")));
        assert!(cfg.cache_components);
        assert_eq!(cfg.experimental["esmExternals"], "loose");
        let i18n = cfg.i18n.unwrap();
        assert_eq!(i18n.locales, vec!["en", "fr"]);
        assert_eq!(i18n.locale_detection, Some(false));
        assert!(
            diagnostics
                .iter()
                .any(|d| d.code == "next-config-i18n-app-router")
        );
        assert_eq!(cfg.page_extensions, DEFAULT_PAGE_EXTENSIONS);
    }

    #[test]
    fn cache_life_builtins_and_merge() {
        let builtin = builtin_cache_life();
        assert_eq!(builtin.len(), 7);
        assert_eq!(builtin["seconds"].revalidate, Some(1));
        assert_eq!(builtin["max"].expire, Some(31_536_000));
        let mut cfg = NextConfig::default();
        cfg.cache_life.insert(
            String::from("short"),
            CacheLifeProfile { stale: None, revalidate: Some(10), expire: Some(5) },
        );
        let (yolo, report) = next_config_to_yolo(&cfg, "app", None, None);
        assert_eq!(
            yolo.cache_life["short"].stale,
            Some(300),
            "inherits stale from default"
        );
        assert!(
            report
                .diagnostics
                .iter()
                .any(|d| d.code == "next-config-cache-life-invalid")
        );
        assert_eq!(yolo.server.action_body_size_limit, 1_048_576);
        assert!(yolo.react.strict_mode, "strict mode defaults on");
    }

    #[test]
    fn byte_sizes_and_env_markers() {
        assert_eq!(parse_byte_size(&Value::from("1mb")), Some(1_048_576));
        assert_eq!(parse_byte_size(&Value::from("2MB")), Some(2_097_152));
        assert_eq!(parse_byte_size(&Value::from("500kb")), Some(512_000));
        assert_eq!(parse_byte_size(&Value::from("1.5gb")), Some(1_610_612_736));
        assert_eq!(parse_byte_size(&Value::from(4096)), Some(4096));
        assert_eq!(parse_byte_size(&Value::from("12 b")), Some(12));
        assert_eq!(parse_byte_size(&Value::from("nope")), None);
        assert_eq!(
            env_markers_to_ts(
                r#"{"a": "${env:X}", "b": "${env:Y|dflt}", "c": "${env:not valid}", "d": "${env:MY-KEY}"}"#
            ),
            r#"{"a": process.env.X, "b": process.env.Y ?? "dflt", "c": "${env:not valid}", "d": process.env["MY-KEY"]}"#
        );
    }

    #[test]
    fn finds_config_file_in_order() {
        let dir = std::env::temp_dir().join(format!("yolo-next-config-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        assert!(find_next_config(&dir).is_none());
        std::fs::write(dir.join("next.config.js"), "module.exports = {}").unwrap();
        std::fs::write(dir.join("next.config.mjs"), "export default {}").unwrap();
        assert_eq!(
            find_next_config(&dir).unwrap().file_name().unwrap(),
            "next.config.mjs"
        );
        std::fs::write(dir.join("next.config.ts"), "export default {}").unwrap();
        assert_eq!(
            find_next_config(&dir).unwrap().file_name().unwrap(),
            "next.config.ts"
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
