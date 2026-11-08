// SPDX-License-Identifier: Apache-2.0
//! `proxy.ts` (and deprecated `middleware.ts`) analysis and skeleton emitters.
//!
//! Next.js 16 renamed `middleware` to `proxy` and runs it on the Node.js runtime only. The
//! analyzer finds the exported handler, normalises `config.matcher` into [`RouteRule`]s and
//! records which `NextResponse` capabilities the handler relies on, so the emitters can wrap it
//! for `Bun.serve` or sketch an axum/tower middleware.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use oxc_allocator::Allocator;
use oxc_ast::ast::{
    self, Argument, ArrowFunctionBody, BindingPattern, ChainElement, Declaration,
    ExportDefaultDeclarationKind, Expression, ForStatementInit, ForStatementLeft, ModuleExportName,
    ObjectPropertyKind, PropertyKey, Statement,
};
use oxc_parser::Parser;
use oxc_span::SourceType;
use serde_json::Value;

use crate::config::{conditions_from, expression_to_json};
use crate::rules::compile_source;
use crate::types::{Diagnostic, ProxySpec, RouteRule};

const EXTENSIONS: [&str; 4] = ["ts", "js", "mts", "mjs"];

/// Finds `proxy.(ts|js|mts|mjs)` / `middleware.(ts|js|mts|mjs)` at the root or under `src/`.
pub fn find_proxy(root: &Path) -> Option<PathBuf> {
    [
        ("", "proxy"),
        ("src", "proxy"),
        ("", "middleware"),
        ("src", "middleware"),
    ]
    .iter()
    .flat_map(|(dir, stem)| {
        EXTENSIONS.iter().map(move |ext| {
            let name = format!("{stem}.{ext}");
            if dir.is_empty() {
                root.join(name)
            } else {
                root.join(dir).join(name)
            }
        })
    })
    .find(|path| path.is_file())
}

fn file_stem(file: &str) -> &str {
    let name = file.rsplit(['/', '\\']).next().unwrap_or(file);
    name.split_once('.').map_or(name, |(stem, _)| stem)
}

fn source_type_for(file: &str) -> SourceType {
    match file.rsplit('.').next() {
        Some("js" | "mjs" | "cjs") => SourceType::mjs(),
        Some("tsx" | "jsx") => SourceType::tsx(),
        _ => SourceType::ts(),
    }
}

fn module_export_name(name: &ModuleExportName<'_>) -> String {
    match name {
        ModuleExportName::IdentifierName(id) => id.name.to_string(),
        ModuleExportName::IdentifierReference(id) => id.name.to_string(),
        ModuleExportName::StringLiteral(lit) => lit.value.display().to_string(),
    }
}

/// `config.matcher` (string | (string | { source, has, missing, locale })[]) -> rules.
fn matchers_from(config: &Value) -> Vec<RouteRule> {
    let Some(matcher) = config.get("matcher") else {
        return Vec::new();
    };
    let items: Vec<&Value> = match matcher {
        Value::Array(items) => items.iter().collect(),
        other => vec![other],
    };
    items
        .into_iter()
        .filter_map(|item| match item {
            Value::String(source) => {
                Some(RouteRule { source: source.clone(), ..RouteRule::default() })
            }
            Value::Object(object) => Some(RouteRule {
                source: object.get("source")?.as_str()?.to_owned(),
                locale: object.get("locale").and_then(Value::as_bool),
                has: conditions_from(object.get("has")),
                missing: conditions_from(object.get("missing")),
                ..RouteRule::default()
            }),
            _ => None,
        })
        .collect()
}

/// `NextResponse.next({ request: { headers } })`: first argument is an object whose `request`
/// property is an object literal carrying a `headers` key.
fn has_request_headers(expr: &Expression<'_>) -> bool {
    let Expression::ObjectExpression(object) = expr.get_inner_expression() else {
        return false;
    };
    object.properties.iter().any(|property| {
        let ObjectPropertyKind::ObjectProperty(prop) = property else {
            return false;
        };
        let is_request = matches!(&prop.key, PropertyKey::StaticIdentifier(id) if id.name == "request");
        is_request
            && match prop.value.get_inner_expression() {
                Expression::ObjectExpression(inner) => inner.properties.iter().any(|p| {
                    matches!(p, ObjectPropertyKind::ObjectProperty(p)
                        if matches!(&p.key, PropertyKey::StaticIdentifier(id) if id.name == "headers"))
                }),
                _ => true,
            }
    })
}

/// Recursive usage visitor over statements and expressions.
struct Usage<'s> {
    spec: &'s mut ProxySpec,
}

impl Usage<'_> {
    fn visit_statements<'a>(&mut self, statements: impl IntoIterator<Item = &'a Statement<'a>>) {
        for statement in statements {
            self.visit_statement(statement);
        }
    }

    fn visit_function(&mut self, func: &ast::Function<'_>) {
        if let Some(body) = &func.body {
            self.visit_statements(&body.statements);
        }
    }

    fn visit_declaration(&mut self, declaration: &Declaration<'_>) {
        match declaration {
            Declaration::VariableDeclaration(decl) => {
                for declarator in &decl.declarations {
                    if let Some(init) = &declarator.init {
                        self.visit_expression(init);
                    }
                }
            }
            Declaration::FunctionDeclaration(func) => self.visit_function(func),
            Declaration::ClassDeclaration(class) => self.visit_class(class),
            _ => {}
        }
    }

    fn visit_class(&mut self, class: &ast::Class<'_>) {
        for element in &class.body.body {
            match element {
                ast::ClassElement::MethodDefinition(method) => self.visit_function(&method.value),
                ast::ClassElement::PropertyDefinition(property) => {
                    if let Some(value) = &property.value {
                        self.visit_expression(value);
                    }
                }
                ast::ClassElement::StaticBlock(block) => self.visit_statements(&block.body),
                _ => {}
            }
        }
    }

    fn visit_statement(&mut self, statement: &Statement<'_>) {
        match statement {
            Statement::BlockStatement(block) => self.visit_statements(&block.body),
            Statement::ExpressionStatement(stmt) => self.visit_expression(&stmt.expression),
            Statement::ReturnStatement(ret) => {
                if let Some(argument) = &ret.argument {
                    self.visit_expression(argument);
                }
            }
            Statement::ThrowStatement(stmt) => self.visit_expression(&stmt.argument),
            Statement::IfStatement(stmt) => {
                self.visit_expression(&stmt.test);
                self.visit_statement(&stmt.consequent);
                if let Some(alternate) = &stmt.alternate {
                    self.visit_statement(alternate);
                }
            }
            Statement::VariableDeclaration(decl) => {
                for declarator in &decl.declarations {
                    if let Some(init) = &declarator.init {
                        self.visit_expression(init);
                    }
                }
            }
            Statement::FunctionDeclaration(func) => self.visit_function(func),
            Statement::ClassDeclaration(class) => self.visit_class(class),
            Statement::ForStatement(stmt) => {
                match &stmt.init {
                    Some(ForStatementInit::VariableDeclaration(decl)) => {
                        for declarator in &decl.declarations {
                            if let Some(init) = &declarator.init {
                                self.visit_expression(init);
                            }
                        }
                    }
                    Some(other) => {
                        if let Some(expr) = other.as_expression() {
                            self.visit_expression(expr);
                        }
                    }
                    None => {}
                }
                if let Some(test) = &stmt.test {
                    self.visit_expression(test);
                }
                if let Some(update) = &stmt.update {
                    self.visit_expression(update);
                }
                self.visit_statement(&stmt.body);
            }
            Statement::ForInStatement(stmt) => {
                self.visit_for_left(&stmt.left);
                self.visit_expression(&stmt.right);
                self.visit_statement(&stmt.body);
            }
            Statement::ForOfStatement(stmt) => {
                self.visit_for_left(&stmt.left);
                self.visit_expression(&stmt.right);
                self.visit_statement(&stmt.body);
            }
            Statement::WhileStatement(stmt) => {
                self.visit_expression(&stmt.test);
                self.visit_statement(&stmt.body);
            }
            Statement::DoWhileStatement(stmt) => {
                self.visit_statement(&stmt.body);
                self.visit_expression(&stmt.test);
            }
            Statement::TryStatement(stmt) => {
                self.visit_statements(&stmt.block.body);
                if let Some(handler) = &stmt.handler {
                    self.visit_statements(&handler.body.body);
                }
                if let Some(finalizer) = &stmt.finalizer {
                    self.visit_statements(&finalizer.body);
                }
            }
            Statement::SwitchStatement(stmt) => {
                self.visit_expression(&stmt.discriminant);
                for case in &stmt.cases {
                    if let Some(test) = &case.test {
                        self.visit_expression(test);
                    }
                    self.visit_statements(&case.consequent);
                }
            }
            Statement::LabeledStatement(stmt) => self.visit_statement(&stmt.body),
            Statement::ExportDeclaration(export) => self.visit_declaration(&export.declaration),
            Statement::ExportDefaultDeclaration(export) => match &export.declaration {
                ExportDefaultDeclarationKind::FunctionDeclaration(func) => {
                    self.visit_function(func);
                }
                ExportDefaultDeclarationKind::ClassDeclaration(class) => self.visit_class(class),
                other => {
                    if let Some(expr) = other.as_expression() {
                        self.visit_expression(expr);
                    }
                }
            },
            _ => {}
        }
    }

    fn visit_for_left(&mut self, left: &ForStatementLeft<'_>) {
        if let ForStatementLeft::VariableDeclaration(decl) = left {
            for declarator in &decl.declarations {
                if let Some(init) = &declarator.init {
                    self.visit_expression(init);
                }
            }
        }
    }

    fn visit_arguments<'a>(&mut self, arguments: impl IntoIterator<Item = &'a Argument<'a>>) {
        for argument in arguments {
            match argument {
                Argument::SpreadElement(spread) => self.visit_expression(&spread.argument),
                other => {
                    if let Some(expr) = other.as_expression() {
                        self.visit_expression(expr);
                    }
                }
            }
        }
    }

    fn visit_call(&mut self, call: &ast::CallExpression<'_>) {
        let callee = call.callee.get_inner_expression();
        if callee.is_specific_member_access("NextResponse", "rewrite") {
            self.spec.uses_rewrite = true;
        } else if callee.is_specific_member_access("NextResponse", "redirect") {
            self.spec.uses_redirect = true;
        } else if callee.is_specific_member_access("NextResponse", "next")
            && call
                .arguments
                .first()
                .and_then(Argument::as_expression)
                .is_some_and(has_request_headers)
        {
            self.spec.uses_next_headers = true;
        }
        if callee
            .get_member_expr()
            .and_then(ast::MemberExpression::static_property_name)
            .is_some_and(|name| name == "waitUntil")
        {
            self.spec.uses_wait_until = true;
        }
        self.visit_expression(&call.callee);
        self.visit_arguments(&call.arguments);
    }

    fn note_property(&mut self, name: &str) {
        match name {
            "cookies" => self.spec.uses_cookies = true,
            "geo" | "ip" => self.spec.uses_removed_geo_ip = true,
            _ => {}
        }
    }

    fn visit_member(&mut self, member: &ast::MemberExpression<'_>) {
        if let Some(name) = member.static_property_name() {
            let name = name.display().to_string();
            self.note_property(&name);
        }
        self.visit_expression(member.object());
        if let ast::MemberExpression::ComputedMemberExpression(computed) = member {
            self.visit_expression(&computed.expression);
        }
    }

    #[allow(
        clippy::too_many_lines,
        reason = "flat exhaustive match over Expression variants"
    )]
    fn visit_expression(&mut self, expr: &Expression<'_>) {
        let inner = expr.get_inner_expression();
        match inner {
            Expression::CallExpression(call) => self.visit_call(call),
            Expression::NewExpression(new) => {
                self.visit_expression(&new.callee);
                self.visit_arguments(&new.arguments);
            }
            Expression::StaticMemberExpression(_)
            | Expression::ComputedMemberExpression(_)
            | Expression::PrivateFieldExpression(_) => {
                if let Some(member) = inner.as_member_expression() {
                    self.visit_member(member);
                }
            }
            Expression::ChainExpression(chain) => match &chain.expression {
                ChainElement::CallExpression(call) => self.visit_call(call),
                ChainElement::TSNonNullExpression(e) => self.visit_expression(&e.expression),
                other => {
                    if let Some(member) = other.as_member_expression() {
                        self.visit_member(member);
                    }
                }
            },
            Expression::ArrowFunctionExpression(arrow) => match &arrow.body {
                ArrowFunctionBody::FunctionBody(body) => self.visit_statements(&body.statements),
                other => {
                    if let Some(body) = other.as_expression() {
                        self.visit_expression(body);
                    }
                }
            },
            Expression::FunctionExpression(func) => self.visit_function(func),
            Expression::ClassExpression(class) => self.visit_class(class),
            Expression::AwaitExpression(e) => self.visit_expression(&e.argument),
            Expression::YieldExpression(e) => {
                if let Some(argument) = &e.argument {
                    self.visit_expression(argument);
                }
            }
            Expression::UnaryExpression(e) => self.visit_expression(&e.argument),
            Expression::BinaryExpression(e) => {
                self.visit_expression(&e.left);
                self.visit_expression(&e.right);
            }
            Expression::LogicalExpression(e) => {
                self.visit_expression(&e.left);
                self.visit_expression(&e.right);
            }
            Expression::ConditionalExpression(e) => {
                self.visit_expression(&e.test);
                self.visit_expression(&e.consequent);
                self.visit_expression(&e.alternate);
            }
            Expression::AssignmentExpression(e) => {
                if let Some(member) = e.left.as_member_expression() {
                    self.visit_member(member);
                }
                self.visit_expression(&e.right);
            }
            Expression::SequenceExpression(e) => {
                for item in &e.expressions {
                    self.visit_expression(item);
                }
            }
            Expression::ArrayExpression(array) => {
                for element in &array.elements {
                    match element {
                        ast::ArrayExpressionElement::SpreadElement(spread) => {
                            self.visit_expression(&spread.argument);
                        }
                        other => {
                            if let Some(item) = other.as_expression() {
                                self.visit_expression(item);
                            }
                        }
                    }
                }
            }
            Expression::ObjectExpression(object) => {
                for property in &object.properties {
                    match property {
                        ObjectPropertyKind::ObjectProperty(prop) => {
                            self.visit_expression(&prop.value);
                        }
                        ObjectPropertyKind::SpreadProperty(spread) => {
                            self.visit_expression(&spread.argument);
                        }
                    }
                }
            }
            Expression::TemplateLiteral(tpl) => {
                for item in &tpl.expressions {
                    self.visit_expression(item);
                }
            }
            Expression::TaggedTemplateExpression(tagged) => {
                self.visit_expression(&tagged.tag);
                for item in &tagged.quasi.expressions {
                    self.visit_expression(item);
                }
            }
            Expression::ImportExpression(import) => self.visit_expression(&import.source),
            _ => {}
        }
    }
}

/// Export analysis: handler name, `config.matcher`, `runtime`.
fn analyze_exports(program: &ast::Program<'_>, spec: &mut ProxySpec) {
    let mut bindings: Vec<(String, &Expression<'_>)> = Vec::new();
    for statement in &program.body {
        let decl = match statement {
            Statement::VariableDeclaration(decl) => Some(decl),
            Statement::ExportDeclaration(export) => match &export.declaration {
                Declaration::VariableDeclaration(decl) => Some(decl),
                _ => None,
            },
            _ => None,
        };
        if let Some(decl) = decl {
            for declarator in &decl.declarations {
                if let BindingPattern::BindingIdentifier(id) = &declarator.id
                    && let Some(init) = &declarator.init
                {
                    bindings.push((id.name.to_string(), init));
                }
            }
        }
    }
    let lookup = |name: &str| {
        bindings
            .iter()
            .rev()
            .find(|(bound, _)| bound == name)
            .map(|(_, init)| *init)
    };
    for statement in &program.body {
        match statement {
            Statement::ExportDeclaration(export) => match &export.declaration {
                Declaration::FunctionDeclaration(func) => {
                    if let Some(id) = &func.id
                        && matches!(id.name.as_str(), "proxy" | "middleware")
                    {
                        spec.export_name = id.name.to_string();
                    }
                }
                Declaration::VariableDeclaration(decl) => {
                    for declarator in &decl.declarations {
                        let BindingPattern::BindingIdentifier(id) = &declarator.id else {
                            continue;
                        };
                        apply_named_export(spec, id.name.as_str(), declarator.init.as_ref());
                    }
                }
                _ => {}
            },
            Statement::ExportNamedDeclaration(named) => {
                for specifier in &named.specifiers {
                    let exported = module_export_name(&specifier.exported);
                    let local = module_export_name(&specifier.local);
                    apply_named_export(spec, &exported, lookup(&local));
                }
            }
            Statement::ExportDefaultDeclaration(_) => spec.export_name = String::from("default"),
            _ => {}
        }
    }
}

fn apply_named_export(spec: &mut ProxySpec, name: &str, init: Option<&Expression<'_>>) {
    match name {
        "proxy" | "middleware" | "default" => name.clone_into(&mut spec.export_name),
        "config" => {
            if let Some(value) = init.and_then(expression_to_json) {
                spec.matchers = matchers_from(&value);
            }
        }
        "runtime" => {
            let value = init
                .and_then(expression_to_json)
                .and_then(|v| v.as_str().map(str::to_owned))
                .unwrap_or_else(|| String::from("unknown"));
            spec.runtime_export = Some(value);
        }
        _ => {}
    }
}

/// Analyzes the proxy module (matchers, `NextResponse` usage).
pub fn analyze_proxy(source: &str, file: &str) -> ProxySpec {
    let mut spec = ProxySpec {
        file: file.to_owned(),
        legacy_middleware_name: file_stem(file) == "middleware",
        ..ProxySpec::default()
    };
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, source_type_for(file)).parse();
    if parsed.fatal_error || !parsed.diagnostics.is_empty() {
        for diagnostic in &*parsed.diagnostics {
            spec.diagnostics.push(
                Diagnostic::error("proxy-parse-error", diagnostic.to_string()).with_file(file),
            );
        }
        if parsed.fatal_error {
            return spec;
        }
    }
    analyze_exports(&parsed.program, &mut spec);
    {
        let mut usage = Usage { spec: &mut spec };
        usage.visit_statements(&parsed.program.body);
    }
    finalize_diagnostics(&mut spec);
    spec
}

fn finalize_diagnostics(spec: &mut ProxySpec) {
    let file = spec.file.clone();
    let mut diagnostics = Vec::new();
    if spec.export_name.is_empty() {
        diagnostics.push(
            Diagnostic::error(
                "proxy-export-missing",
                "no `proxy`, `middleware` or default export found",
            )
            .with_hint("export function proxy(request: NextRequest) { ... }"),
        );
    }
    if spec.legacy_middleware_name || spec.export_name == "middleware" {
        diagnostics.push(
            Diagnostic::warning(
                "middleware-renamed-to-proxy",
                "`middleware` was renamed to `proxy` in Next.js 16",
            )
            .with_hint("rename the file to proxy.ts and export `proxy`"),
        );
    }
    if let Some(runtime) = &spec.runtime_export {
        diagnostics.push(
            Diagnostic::error(
                "proxy-runtime-export-invalid",
                format!("`export const runtime = \"{runtime}\"` is invalid in proxy"),
            )
            .with_hint(
                "proxy runs on the Node.js runtime only; remove the export (`edge` is unsupported)",
            ),
        );
    }
    if spec.uses_removed_geo_ip {
        diagnostics.push(
            Diagnostic::warning(
                "proxy-geo-ip-removed",
                "`request.geo` / `request.ip` were removed in Next.js 15",
            )
            .with_hint("use request.headers x-forwarded-for / platform headers"),
        );
    }
    for matcher in &spec.matchers {
        if let Err(err) = compile_source(&matcher.source) {
            diagnostics.push(
                Diagnostic::error("proxy-matcher-invalid", err.message)
                    .with_hint("matchers must start with `/` and use the path-to-regexp subset"),
            );
        }
    }
    for diagnostic in diagnostics {
        spec.diagnostics.push(diagnostic.with_file(file.clone()));
    }
}

/// Import specifier of the user's module from the generated `src/proxy.ts` wrapper.
fn module_import_path(file: &str) -> (String, bool) {
    let normalized = file.replace('\\', "/");
    let without_ext = normalized
        .rsplit_once('.')
        .map_or(normalized.as_str(), |(stem, _)| stem);
    match without_ext.strip_prefix("src/") {
        Some("proxy") => (String::from("./proxy.next"), true),
        Some(rest) => (format!("./{rest}"), false),
        None => (format!("../{without_ext}"), false),
    }
}

fn ts_string(value: &str) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| format!("\"{value}\""))
}

/// Emits a Bun `proxy` wrapper (`src/proxy.ts`) honouring the matchers.
///
/// The wrapper imports the original module relative to `src/` (`../proxy` for a root
/// `proxy.ts`); when the original already lives at `src/proxy.ts` it must be renamed to
/// `src/proxy.next.ts`, which the header comment states.
pub fn emit_bun_proxy(spec: &ProxySpec) -> String {
    let (import_path, renamed) = module_import_path(&spec.file);
    let export_name = if spec.export_name.is_empty() {
        "proxy"
    } else {
        spec.export_name.as_str()
    };
    let mut out = String::new();
    _ = writeln!(
        out,
        "// Generated by yolo-next from {}. Bun.serve wrapper around the Next.js proxy (@yolo-rs/next).",
        spec.file
    );
    if renamed {
        out.push_str("// The original src/proxy.ts is imported as ./proxy.next (rename it to src/proxy.next.ts).\n");
    }
    out.push_str(
        "//\n// Semantics:\n\
         // - config.matcher entries are compiled with compileSource (path-to-regexp subset) and tested\n\
         //   against url.pathname together with their has/missing conditions; non-matching requests\n\
         //   go straight to next(). An empty matcher list matches every request.\n\
         // - NextResponse.next() (shim header x-middleware-next: 1) -> next(); headers set on the\n\
         //   NextResponse are merged into the downstream response. Request header overrides\n\
         //   (x-middleware-override-headers / x-middleware-request-*) are applied by the runtime.\n\
         // - NextResponse.rewrite(url) -> returned with x-middleware-rewrite; the runtime re-dispatches\n\
         //   the rewritten URL internally (same semantics as Next.js).\n\
         // - NextResponse.redirect(url) -> 307/308 with Location, returned as-is.\n\
         // - Any other Response (json, 401, ...) short-circuits the route.\n\
         // `next/server` (NextResponse, NextRequest) is aliased to @yolo-rs/next/server by yolo.bundler.ts.\n",
    );
    out.push_str("import { compileSource, conditionMatches, type CompiledPattern } from \"@yolo-rs/next/rules\";\n");
    _ = write!(
        out,
        "import * as proxyModule from {};\n\n",
        ts_string(&import_path)
    );
    out.push_str(
        "type ProxyResult = Response | undefined | null | void;\n\
         type ProxyHandler = (\n  request: Request,\n  event: { waitUntil(promise: Promise<unknown>): void },\n) => ProxyResult | Promise<ProxyResult>;\n\n",
    );
    _ = write!(
        out,
        "const handler = (proxyModule as Record<string, unknown>)[{}] as ProxyHandler;\n\n",
        ts_string(export_name)
    );
    out.push_str(
        "interface Condition {\n  type: string;\n  key?: string;\n  value?: string;\n}\n\n\
         interface Matcher {\n  source: string;\n  has: Condition[];\n  missing: Condition[];\n  pattern: CompiledPattern;\n}\n\n",
    );
    out.push_str("const MATCHERS: Matcher[] = [\n");
    for matcher in &spec.matchers {
        let has = serde_json::to_string(&matcher.has).unwrap_or_else(|_| String::from("[]"));
        let missing =
            serde_json::to_string(&matcher.missing).unwrap_or_else(|_| String::from("[]"));
        let source = ts_string(&matcher.source);
        _ = writeln!(
            out,
            "  {{ source: {source}, has: {has}, missing: {missing}, pattern: compileSource({source}) }},"
        );
    }
    out.push_str("];\n\n");
    out.push_str(
        "function matches(request: Request, url: URL): boolean {\n\
         \x20 if (MATCHERS.length === 0) return true;\n\
         \x20 return MATCHERS.some(\n\
         \x20   (m) =>\n\
         \x20     m.pattern.test(url.pathname) &&\n\
         \x20     m.has.every((c) => conditionMatches(c, request, url)) &&\n\
         \x20     m.missing.every((c) => !conditionMatches(c, request, url)),\n\
         \x20 );\n}\n\n",
    );
    out.push_str(
        "export async function runProxy(request: Request, next: () => Promise<Response>): Promise<Response> {\n\
         \x20 const url = new URL(request.url);\n\
         \x20 if (!matches(request, url)) return next();\n\
         \x20 const pending: Promise<unknown>[] = [];\n\
         \x20 const result = await handler(request, { waitUntil: (promise) => void pending.push(promise) });\n\
         \x20 if (pending.length > 0) void Promise.allSettled(pending);\n\
         \x20 if (!result) return next();\n\
         \x20 if (result.headers.get(\"x-middleware-next\") === \"1\") {\n\
         \x20   const downstream = await next();\n\
         \x20   const merged = new Response(downstream.body, downstream);\n\
         \x20   result.headers.forEach((value, key) => {\n\
         \x20     if (!key.startsWith(\"x-middleware-\")) merged.headers.set(key, value);\n\
         \x20   });\n\
         \x20   return merged;\n\
         \x20 }\n\
         \x20 // rewrite (x-middleware-rewrite), redirect (Location) or a direct response\n\
         \x20 return result;\n}\n\nexport default runProxy;\n",
    );
    out
}

/// Emits an axum/tower middleware skeleton (`src/proxy.rs`).
pub fn emit_axum_proxy(spec: &ProxySpec) -> String {
    let export_name = if spec.export_name.is_empty() {
        "proxy"
    } else {
        spec.export_name.as_str()
    };
    let mut out = String::new();
    _ = write!(
        out,
        "//! Generated by yolo-next from {}: axum 0.8 / tower middleware skeleton for the Next.js proxy.\n//!\n//! Port the body of `{export_name}` into [`proxy_middleware`]; the matcher table mirrors `config.matcher`.\n\n",
        spec.file
    );
    out.push_str("use axum::extract::Request;\nuse axum::middleware::Next;\nuse axum::response::Response;\n\n");
    out.push_str("/// Matchers from `config.matcher` in axum 0.8 syntax (`{param}`, `{*rest}`).\n");
    out.push_str("pub const PROXY_MATCHERS: &[&str] = &[\n");
    for matcher in &spec.matchers {
        let pattern = compile_source(&matcher.source)
            .map_or_else(|_| matcher.source.clone(), |p| p.axum_pattern());
        _ = write!(out, "    {pattern:?},");
        if !matcher.has.is_empty() || !matcher.missing.is_empty() {
            _ = write!(
                out,
                " // has: {}, missing: {}",
                serde_json::to_string(&matcher.has).unwrap_or_default(),
                serde_json::to_string(&matcher.missing).unwrap_or_default()
            );
        }
        out.push('\n');
    }
    out.push_str("];\n\n");
    out.push_str(
        "/// True when `path` matches one of [`PROXY_MATCHERS`] (segment-wise: `{x}` is one segment,\n\
         /// `{*x}` the rest). An empty table matches everything.\n\
         pub fn proxy_matches(path: &str) -> bool {\n\
         \x20   PROXY_MATCHERS.is_empty() || PROXY_MATCHERS.iter().any(|pattern| pattern_matches(pattern, path))\n\
         }\n\n\
         fn pattern_matches(pattern: &str, path: &str) -> bool {\n\
         \x20   let mut want = pattern.split('/').filter(|s| !s.is_empty());\n\
         \x20   let mut have = path.split('/').filter(|s| !s.is_empty()).peekable();\n\
         \x20   loop {\n\
         \x20       match want.next() {\n\
         \x20           None => return have.peek().is_none(),\n\
         \x20           Some(segment) if segment.starts_with(\"{*\") => return true,\n\
         \x20           Some(segment) if segment.starts_with('{') => {\n\
         \x20               if have.next().is_none() {\n\
         \x20                   return false;\n\
         \x20               }\n\
         \x20           }\n\
         \x20           Some(segment) => {\n\
         \x20               if have.next() != Some(segment) {\n\
         \x20                   return false;\n\
         \x20               }\n\
         \x20           }\n\
         \x20       }\n\
         \x20   }\n\
         }\n\n",
    );
    out.push_str(
        "/// `tower::Layer`-style middleware: `Router::new().layer(axum::middleware::from_fn(proxy_middleware))`.\n\
         pub async fn proxy_middleware(req: Request, next: Next) -> Response {\n\
         \x20   if !proxy_matches(req.uri().path()) {\n\
         \x20       return next.run(req).await;\n\
         \x20   }\n",
    );
    out.push_str("    // TODO(yolo-next): port the Next.js proxy body.\n");
    if spec.uses_redirect {
        out.push_str("    //   NextResponse.redirect(url) -> return axum::response::Redirect::temporary(url).into_response();\n");
    }
    if spec.uses_rewrite {
        out.push_str("    //   NextResponse.rewrite(url) -> *req.uri_mut() = new_uri; before next.run(req)\n");
    }
    if spec.uses_cookies {
        out.push_str("    //   request.cookies -> axum_extra::extract::CookieJar::from_headers(req.headers())\n");
    }
    if spec.uses_next_headers {
        out.push_str("    //   NextResponse.next({ request: { headers } }) -> req.headers_mut().insert(...)\n");
    }
    if spec.uses_wait_until {
        out.push_str("    //   event.waitUntil(promise) -> tokio::spawn(async move { ... })\n");
    }
    if spec.uses_removed_geo_ip {
        out.push_str(
            "    //   request.geo / request.ip -> read x-forwarded-for / platform headers\n",
        );
    }
    out.push_str("    next.run(req).await\n}\n");
    out
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::types::Severity;

    const FIXTURE_PROXY: &str = r#"import { NextResponse, type NextRequest } from "next/server";

export function proxy(request: NextRequest) {
  const token = request.cookies.get("session")?.value;
  if (!token && request.nextUrl.pathname.startsWith("/dashboard")) {
    return NextResponse.redirect(new URL("/login", request.url));
  }
  if (request.nextUrl.pathname.startsWith("/u/")) {
    return NextResponse.rewrite(
      new URL(request.nextUrl.pathname.replace("/u/", "/users/"), request.url),
    );
  }
  const response = NextResponse.next({ request: { headers: new Headers(request.headers) } });
  response.headers.set("x-yolo-proxy", "1");
  return response;
}

export const config = {
  matcher: [
    "/dashboard/:path*",
    "/u/:path*",
    { source: "/api/:path*", missing: [{ type: "header", key: "x-internal" }] },
  ],
};
"#;

    #[test]
    fn analyzes_fixture_proxy() {
        let spec = analyze_proxy(FIXTURE_PROXY, "proxy.ts");
        assert_eq!(spec.export_name, "proxy");
        assert!(!spec.legacy_middleware_name);
        assert_eq!(spec.matchers.len(), 3);
        assert_eq!(spec.matchers[0].source, "/dashboard/:path*");
        assert_eq!(spec.matchers[2].source, "/api/:path*");
        assert_eq!(spec.matchers[2].missing[0].kind, "header");
        assert_eq!(spec.matchers[2].missing[0].key, "x-internal");
        assert!(spec.uses_rewrite);
        assert!(spec.uses_redirect);
        assert!(spec.uses_next_headers);
        assert!(spec.uses_cookies);
        assert!(!spec.uses_wait_until);
        assert!(!spec.uses_removed_geo_ip);
        assert!(spec.runtime_export.is_none());
        assert!(spec.diagnostics.is_empty(), "{:?}", spec.diagnostics);
    }

    #[test]
    fn legacy_middleware_with_runtime_geo_and_wait_until() {
        let source = r#"
import { NextResponse } from "next/server";
export const runtime = "edge";
export default async function middleware(request, event) {
  event.waitUntil(fetch("https://log.example.com", { method: "POST", body: request.ip ?? "" }));
  const country = request.geo?.country ?? "US";
  return NextResponse.next();
}
export const config = { matcher: "/((?!_next/static|favicon.ico).*)" };
"#;
        let spec = analyze_proxy(source, "src/middleware.ts");
        assert!(spec.legacy_middleware_name);
        assert_eq!(spec.export_name, "default");
        assert_eq!(spec.runtime_export.as_deref(), Some("edge"));
        assert!(spec.uses_wait_until);
        assert!(spec.uses_removed_geo_ip);
        assert!(
            !spec.uses_next_headers,
            "NextResponse.next() without request headers"
        );
        assert_eq!(spec.matchers.len(), 1);
        let codes: Vec<&str> = spec.diagnostics.iter().map(|d| d.code.as_str()).collect();
        assert!(codes.contains(&"middleware-renamed-to-proxy"));
        assert!(codes.contains(&"proxy-runtime-export-invalid"));
        assert!(codes.contains(&"proxy-geo-ip-removed"));
        let runtime = spec
            .diagnostics
            .iter()
            .find(|d| d.code == "proxy-runtime-export-invalid")
            .unwrap();
        assert_eq!(runtime.severity, Severity::Error);
        assert_eq!(runtime.file.as_deref(), Some("src/middleware.ts"));
    }

    #[test]
    fn named_specifier_exports_and_invalid_matcher() {
        let source = r#"
const handler = (req) => NextResponse.next();
const config = { matcher: ["dashboard/:path*", { source: "/x", has: [{ type: "cookie", key: "a", value: "b" }], locale: false }] };
export { handler as proxy, config };
"#;
        let spec = analyze_proxy(source, "proxy.js");
        assert_eq!(spec.export_name, "proxy");
        assert_eq!(spec.matchers.len(), 2);
        assert_eq!(spec.matchers[1].has[0].value.as_deref(), Some("b"));
        assert_eq!(spec.matchers[1].locale, Some(false));
        assert!(
            spec.diagnostics
                .iter()
                .any(|d| d.code == "proxy-matcher-invalid")
        );
        let missing = analyze_proxy("export const other = 1;", "proxy.ts");
        assert!(
            missing
                .diagnostics
                .iter()
                .any(|d| d.code == "proxy-export-missing")
        );
        let broken = analyze_proxy("export function proxy( {", "proxy.ts");
        assert!(
            broken
                .diagnostics
                .iter()
                .any(|d| d.code == "proxy-parse-error")
        );
    }

    #[test]
    fn emits_bun_wrapper() {
        let spec = analyze_proxy(FIXTURE_PROXY, "proxy.ts");
        let ts = emit_bun_proxy(&spec);
        assert!(ts.contains("export async function runProxy(request: Request, next: () => Promise<Response>): Promise<Response>"));
        assert!(ts.contains("import { compileSource, conditionMatches, type CompiledPattern } from \"@yolo-rs/next/rules\";"));
        assert!(ts.contains("import * as proxyModule from \"../proxy\";"));
        assert!(ts.contains("[\"proxy\"] as ProxyHandler"));
        assert!(ts.contains("compileSource(\"/dashboard/:path*\")"));
        assert!(ts.contains("missing: [{\"type\":\"header\",\"key\":\"x-internal\"}]"));
        assert!(ts.contains("x-middleware-next"));
        assert!(ts.contains("x-middleware-rewrite"));
        assert!(ts.contains("export default runProxy;"));
        let src_spec = analyze_proxy(FIXTURE_PROXY, "src/proxy.ts");
        let src_ts = emit_bun_proxy(&src_spec);
        assert!(src_ts.contains("from \"./proxy.next\";"));
        let (path, renamed) = module_import_path("src/middleware.ts");
        assert_eq!(path, "./middleware");
        assert!(!renamed);
    }

    #[test]
    fn emits_axum_skeleton() {
        let spec = analyze_proxy(FIXTURE_PROXY, "proxy.ts");
        let rust = emit_axum_proxy(&spec);
        assert!(
            rust.contains("pub async fn proxy_middleware(req: Request, next: Next) -> Response")
        );
        assert!(rust.contains("\"/dashboard/{*path}\","));
        assert!(
            rust.contains("\"/api/{*path}\", // has: [], missing: [{\"type\":\"header\",\"key\":\"x-internal\"}]"),
            "{rust}"
        );
        assert!(rust.contains("pub const PROXY_MATCHERS: &[&str]"));
        assert!(rust.contains("Redirect::temporary"));
        assert!(rust.contains("req.uri_mut()"));
        assert!(rust.contains("CookieJar"));
        assert!(!rust.contains("tokio::spawn"));
    }

    #[test]
    fn finds_proxy_files() {
        let dir = std::env::temp_dir().join(format!("yolo-next-proxy-{}", std::process::id()));
        let src = dir.join("src");
        std::fs::create_dir_all(&src).unwrap();
        assert!(find_proxy(&dir).is_none());
        std::fs::write(src.join("middleware.ts"), "export function middleware() {}").unwrap();
        assert!(find_proxy(&dir).unwrap().ends_with("middleware.ts"));
        std::fs::write(dir.join("proxy.ts"), "export function proxy() {}").unwrap();
        assert_eq!(find_proxy(&dir).unwrap(), dir.join("proxy.ts"));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
