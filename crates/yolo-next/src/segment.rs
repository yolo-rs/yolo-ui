// SPDX-License-Identifier: Apache-2.0
//! Route segment config exports, `generate*` detection, directives and module facts.
//!
//! Parses page, layout, route and metadata modules with OXC 0.155 and extracts the
//! Next.js 16.4 route segment config (`dynamic`, `dynamicParams`, `revalidate`, `fetchCache`,
//! `runtime`, `preferredRegion`, `maxDuration`, `ensureStatic`, `prefetch`, `instant`,
//! `experimental_ppr`), the `generate*` exports, the static `metadata` / `viewport` objects,
//! the module directive, the default export, the `next/*` imports and the React hooks used.

use std::collections::BTreeMap;

use oxc_allocator::Allocator;
use oxc_ast::ast::{
    AssignmentTarget, BindingPattern, Declaration, ExportDefaultDeclarationKind, Expression,
    Function, ModuleExportName, ObjectPropertyKind, PropertyKey, Statement, StringLiteral,
    UnaryOperator, VariableDeclaration,
};
use oxc_parser::Parser;
use oxc_span::SourceType;
use serde_json::{Map, Number, Value};

use crate::metadata::{metadata_from_value, viewport_from_value};
use crate::types::{Diagnostic, ModuleDirective, SegmentConfig};

/// Route handler methods in canonical order.
const HTTP_METHODS: [&str; 7] = ["GET", "HEAD", "POST", "PUT", "PATCH", "DELETE", "OPTIONS"];
/// Legacy Pages Router data functions in canonical order.
const LEGACY_DATA_FUNCTIONS: [&str; 4] = [
    "getServerSideProps",
    "getStaticProps",
    "getStaticPaths",
    "getInitialProps",
];
/// Maximum number of parse errors reported per module.
const MAX_PARSE_DIAGNOSTICS: usize = 3;

/// Everything the module walk learns (the public [`SegmentConfig`] plus private facts).
struct ModuleFacts {
    config: SegmentConfig,
    /// `Page.getInitialProps = ...` static property assignment seen.
    get_initial_props_assigned: bool,
    parse_errors: usize,
}

/// OXC source type for a file name (`None` for MDX, which is not parsed).
fn source_type_for(file: &str) -> Option<SourceType> {
    let ext = file
        .rsplit('.')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    match ext.as_str() {
        "ts" | "mts" | "cts" => Some(SourceType::ts()),
        "jsx" => Some(SourceType::jsx()),
        "js" | "mjs" | "cjs" => Some(SourceType::mjs().with_jsx(true)),
        "mdx" | "md" => None,
        _ => Some(SourceType::tsx()),
    }
}

/// `true` for `.tsx` / `.jsx` files (hooks are collected with the `yolo-react` analyzer).
fn is_jsx_file(file: &str) -> bool {
    std::path::Path::new(file)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("tsx") || ext.eq_ignore_ascii_case("jsx"))
}

/// Analyzes a page/layout/route module.
pub fn analyze_segment_module(source: &str, file: &str) -> SegmentConfig {
    if let Some(source_type) = source_type_for(file) {
        analyze(source, source_type, file, is_jsx_file(file)).config
    } else {
        {
            let mut config = SegmentConfig {
                default_export: Some(String::from("MDXContent")),
                ..SegmentConfig::default()
            };
            config.diagnostics.push(
                Diagnostic::info(
                    "segment-mdx-skipped",
                    "MDX module not analyzed; its default export is the compiled MDX content",
                )
                .with_file(file),
            );
            config
        }
    }
}

/// Parses with the TypeScript grammar first and falls back to TSX when that produces fewer
/// errors (used by the entry points that receive no file name).
fn best_analysis(source: &str, prefer_tsx: bool) -> ModuleFacts {
    let first = if prefer_tsx {
        SourceType::tsx()
    } else {
        SourceType::ts()
    };
    let facts = analyze(source, first, "module", false);
    if facts.parse_errors == 0 {
        return facts;
    }
    let second = if prefer_tsx {
        SourceType::ts()
    } else {
        SourceType::tsx()
    };
    let other = analyze(source, second, "module", false);
    if other.parse_errors < facts.parse_errors {
        other
    } else {
        facts
    }
}

/// HTTP methods exported by a route handler (`GET`, `HEAD`, `POST`, `PUT`, `PATCH`, `DELETE`, `OPTIONS`).
///
/// `HEAD` is not synthesized from `GET` (the server does that at request time).
pub fn route_handler_methods(source: &str) -> Vec<String> {
    let facts = best_analysis(source, false);
    HTTP_METHODS
        .iter()
        .filter(|method| facts.config.named_exports.iter().any(|n| n == *method))
        .map(|method| (*method).to_owned())
        .collect()
}

/// Legacy Pages Router data functions exported by a module.
pub fn legacy_data_functions(source: &str) -> Vec<String> {
    let facts = best_analysis(source, true);
    LEGACY_DATA_FUNCTIONS
        .iter()
        .filter(|name| {
            facts.config.named_exports.iter().any(|n| n == *name)
                || (**name == "getInitialProps" && facts.get_initial_props_assigned)
        })
        .map(|name| (*name).to_owned())
        .collect()
}

// ============================================================================
// AST helpers
// ============================================================================

/// Owned text of a string literal (lone surrogates replaced).
fn js_string(literal: &StringLiteral<'_>) -> String {
    literal.value.as_str().map_or_else(
        || String::from_utf8_lossy(literal.value.as_bytes()).into_owned(),
        str::to_owned,
    )
}

/// Name of a module export name node.
fn export_name(name: &ModuleExportName<'_>) -> String {
    match name {
        ModuleExportName::IdentifierName(n) => n.name.to_string(),
        ModuleExportName::IdentifierReference(r) => r.name.to_string(),
        ModuleExportName::StringLiteral(s) => js_string(s),
    }
}

/// JSON number (integers stay integers).
fn number_value(value: f64) -> Value {
    if value.fract() == 0.0 && value.abs() < 9.0e15 {
        Value::from(value as i64)
    } else {
        Number::from_f64(value).map_or(Value::Null, Value::Number)
    }
}

/// Static name of an object property key.
fn property_key_name(key: &PropertyKey<'_>) -> Option<String> {
    match key {
        PropertyKey::StaticIdentifier(id) => Some(id.name.to_string()),
        PropertyKey::StringLiteral(s) => Some(js_string(s)),
        PropertyKey::NumericLiteral(n) => Some(number_value(n.value).to_string()),
        PropertyKey::TemplateLiteral(t) if t.expressions.is_empty() => Some(
            t.quasis
                .iter()
                .map(|q| q.value.raw.as_str())
                .collect::<String>(),
        ),
        _ => None,
    }
}

/// Evaluates a literal expression (strings, numbers, booleans, null, arrays, plain objects,
/// template literals without substitutions, negated numbers, `new URL("...")`, `as const`).
fn literal_value(expr: &Expression<'_>) -> Option<Value> {
    match expr {
        Expression::StringLiteral(s) => Some(Value::String(js_string(s))),
        Expression::NumericLiteral(n) => Some(number_value(n.value)),
        Expression::BooleanLiteral(b) => Some(Value::Bool(b.value)),
        Expression::NullLiteral(_) => Some(Value::Null),
        Expression::TemplateLiteral(t) if t.expressions.is_empty() => Some(Value::String(
            t.quasis
                .iter()
                .map(|q| q.value.raw.as_str())
                .collect::<String>(),
        )),
        Expression::UnaryExpression(u) if u.operator == UnaryOperator::UnaryNegation => {
            match literal_value(&u.argument)? {
                Value::Number(n) => Some(number_value(-n.as_f64().unwrap_or_default())),
                _ => None,
            }
        }
        Expression::ArrayExpression(array) => array
            .elements
            .iter()
            .map(|element| element.as_expression().and_then(literal_value))
            .collect::<Option<Vec<Value>>>()
            .map(Value::Array),
        Expression::ObjectExpression(object) => {
            let mut map = Map::new();
            for property in &object.properties {
                match property {
                    ObjectPropertyKind::ObjectProperty(p) => {
                        let key = property_key_name(&p.key)?;
                        map.insert(key, literal_value(&p.value)?);
                    }
                    ObjectPropertyKind::SpreadProperty(_) => return None,
                }
            }
            Some(Value::Object(map))
        }
        Expression::NewExpression(call) => match &call.callee {
            Expression::Identifier(id) if id.name == "URL" => call
                .arguments
                .first()
                .and_then(|arg| arg.as_expression())
                .and_then(literal_value)
                .filter(Value::is_string),
            _ => None,
        },
        Expression::ParenthesizedExpression(p) => literal_value(&p.expression),
        Expression::TSAsExpression(e) => literal_value(&e.expression),
        Expression::TSSatisfiesExpression(e) => literal_value(&e.expression),
        Expression::TSNonNullExpression(e) => literal_value(&e.expression),
        Expression::TSTypeAssertion(e) => literal_value(&e.expression),
        _ => None,
    }
}

/// Static JSON of an export initializer: the config evaluator first, then the local literal
/// evaluator.
fn static_value(init: Option<&Expression<'_>>) -> Option<Value> {
    let init = init?;
    crate::config::expression_to_json(init).or_else(|| literal_value(init))
}

fn push_unique(list: &mut Vec<String>, name: &str) {
    if !list.iter().any(|n| n == name) {
        list.push(name.to_owned());
    }
}

// ============================================================================
// Export handling
// ============================================================================

/// Applies a named export (`export const name = init` / `export function name`) to the config.
fn apply_named_export(
    config: &mut SegmentConfig,
    name: &str,
    init: Option<&Expression<'_>>,
    file: &str,
) {
    match name {
        "generateStaticParams" => config.has_generate_static_params = true,
        "generateMetadata" => config.has_generate_metadata = true,
        "generateViewport" => config.has_generate_viewport = true,
        "generateImageMetadata" => config.has_generate_image_metadata = true,
        "generateSitemaps" => config.has_generate_sitemaps = true,
        "metadata" => match static_value(init) {
            Some(value) => config.static_metadata = Some(metadata_from_value(&value)),
            None => config.diagnostics.push(
                Diagnostic::info(
                    "segment-metadata-dynamic",
                    "`metadata` export is not a static object literal; resolved at runtime",
                )
                .with_file(file),
            ),
        },
        "viewport" => match static_value(init) {
            Some(value) => config.static_viewport = Some(viewport_from_value(&value)),
            None => config.diagnostics.push(
                Diagnostic::info(
                    "segment-viewport-dynamic",
                    "`viewport` export is not a static object literal; resolved at runtime",
                )
                .with_file(file),
            ),
        },
        _ => apply_segment_config(config, name, static_value(init), file),
    }
}

/// Applies a route segment config export.
fn apply_segment_config(config: &mut SegmentConfig, name: &str, value: Option<Value>, file: &str) {
    let as_string = |v: &Option<Value>| v.as_ref().and_then(Value::as_str).map(str::to_owned);
    let as_bool = |v: &Option<Value>| v.as_ref().and_then(Value::as_bool);
    match name {
        "dynamic" => config.dynamic = as_string(&value),
        "dynamicParams" => config.dynamic_params = as_bool(&value),
        "revalidate" => config.revalidate = value,
        "fetchCache" => config.fetch_cache = as_string(&value),
        "runtime" => {
            config.runtime = as_string(&value);
            if matches!(
                config.runtime.as_deref(),
                Some("edge" | "experimental-edge")
            ) {
                config.diagnostics.push(
                    Diagnostic::warning(
                        "segment-config-edge-runtime-deprecated",
                        "`runtime = \"edge\"` is deprecated in Next.js 16; the segment runs on the Bun / native runtime",
                    )
                    .with_file(file)
                    .with_hint("remove the export; Yolo has a single runtime"),
                );
            }
        }
        "preferredRegion" => {
            config.preferred_region = value;
            config.diagnostics.push(
                Diagnostic::warning(
                    "segment-config-preferred-region-deprecated",
                    "`preferredRegion` is a Vercel deployment hint and has no Yolo equivalent",
                )
                .with_file(file),
            );
        }
        "maxDuration" => {
            config.max_duration = value
                .as_ref()
                .and_then(Value::as_f64)
                .filter(|n| *n >= 0.0)
                .map(|n| n as u64);
        }
        "ensureStatic" => config.ensure_static = value,
        "prefetch" => config.prefetch = as_string(&value),
        "instant" => config.instant = as_bool(&value),
        "experimental_ppr" => {
            config.experimental_ppr = as_bool(&value);
            config.diagnostics.push(
                Diagnostic::warning(
                    "segment-config-experimental-ppr-removed",
                    "`experimental_ppr` was removed in Next.js 16; use `cacheComponents` (Yolo: `react.cacheComponents`)",
                )
                .with_file(file),
            );
        }
        _ => {}
    }
}

/// Records a top-level function declaration and whether it is `async`.
fn record_function(function: &Function<'_>, functions: &mut BTreeMap<String, bool>) {
    if let Some(id) = &function.id {
        functions.insert(id.name.to_string(), function.r#async);
    }
}

/// Records top-level `const name = async () => {}` / `function` expression bindings.
fn record_variables(var: &VariableDeclaration<'_>, functions: &mut BTreeMap<String, bool>) {
    for d in &var.declarations {
        if let BindingPattern::BindingIdentifier(id) = &d.id {
            let is_async = match &d.init {
                Some(Expression::ArrowFunctionExpression(a)) => Some(a.r#async),
                Some(Expression::FunctionExpression(f)) => Some(f.r#async),
                _ => None,
            };
            if let Some(is_async) = is_async {
                functions.insert(id.name.to_string(), is_async);
            }
        }
    }
}

/// Records top-level function bindings (exported or not) and whether they are `async`.
fn record_function_facts(statement: &Statement<'_>, functions: &mut BTreeMap<String, bool>) {
    match statement {
        Statement::FunctionDeclaration(f) => record_function(f, functions),
        Statement::VariableDeclaration(var) => record_variables(var, functions),
        Statement::ExportDeclaration(export) => match &export.declaration {
            Declaration::FunctionDeclaration(f) => record_function(f, functions),
            Declaration::VariableDeclaration(var) => record_variables(var, functions),
            _ => {}
        },
        _ => {}
    }
}

/// Default export name and `async` flag.
fn default_export_facts(
    kind: &ExportDefaultDeclarationKind<'_>,
    functions: &BTreeMap<String, bool>,
) -> Option<(String, bool)> {
    let anonymous = || String::from("default");
    Some(match kind {
        ExportDefaultDeclarationKind::FunctionDeclaration(f) => (
            f.id.as_ref()
                .map_or_else(anonymous, |id| id.name.to_string()),
            f.r#async,
        ),
        ExportDefaultDeclarationKind::ClassDeclaration(c) => (
            c.id.as_ref()
                .map_or_else(anonymous, |id| id.name.to_string()),
            false,
        ),
        ExportDefaultDeclarationKind::TSInterfaceDeclaration(_) => return None,
        ExportDefaultDeclarationKind::Identifier(id) => {
            let name = id.name.to_string();
            let is_async = functions.get(&name).copied().unwrap_or(false);
            (name, is_async)
        }
        ExportDefaultDeclarationKind::ArrowFunctionExpression(a) => (anonymous(), a.r#async),
        ExportDefaultDeclarationKind::FunctionExpression(f) => (
            f.id.as_ref()
                .map_or_else(anonymous, |id| id.name.to_string()),
            f.r#async,
        ),
        _ => (anonymous(), false),
    })
}

/// `next`, `next/*`, `server-only`, `client-only` import specifiers.
fn is_next_import(specifier: &str) -> bool {
    specifier == "next"
        || specifier.starts_with("next/")
        || specifier == "server-only"
        || specifier == "client-only"
}

/// Walks the module and fills the facts.
// One pass over the top-level statements; each arm is a distinct export form.
#[allow(clippy::too_many_lines)]
fn analyze(source: &str, source_type: SourceType, file: &str, collect_hooks: bool) -> ModuleFacts {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, source_type).parse();
    let mut config = SegmentConfig::default();
    let parse_errors = parsed.diagnostics.iter().count();
    for error in parsed.diagnostics.iter().take(MAX_PARSE_DIAGNOSTICS) {
        config
            .diagnostics
            .push(Diagnostic::error("segment-parse-error", error.to_string()).with_file(file));
    }
    let program = &parsed.program;

    for directive in &program.directives {
        let text = directive.directive.as_str();
        let found = if text == "use client" {
            ModuleDirective::UseClient
        } else if text == "use server" {
            ModuleDirective::UseServer
        } else if text.starts_with("use cache") {
            ModuleDirective::UseCache
        } else {
            continue;
        };
        if config.directive == ModuleDirective::None {
            config.directive = found;
        }
    }

    let mut functions = BTreeMap::new();
    for statement in &program.body {
        record_function_facts(statement, &mut functions);
    }

    let mut get_initial_props_assigned = false;
    for statement in &program.body {
        match statement {
            Statement::ImportDeclaration(import) => {
                let specifier = js_string(&import.source);
                if is_next_import(&specifier) {
                    push_unique(&mut config.next_imports, &specifier);
                }
            }
            Statement::ExportDeclaration(export) => match &export.declaration {
                Declaration::VariableDeclaration(var) => {
                    for d in &var.declarations {
                        if let BindingPattern::BindingIdentifier(id) = &d.id {
                            let name = id.name.to_string();
                            push_unique(&mut config.named_exports, &name);
                            apply_named_export(&mut config, &name, d.init.as_ref(), file);
                        }
                    }
                }
                Declaration::FunctionDeclaration(f) => {
                    if let Some(id) = &f.id {
                        let name = id.name.to_string();
                        push_unique(&mut config.named_exports, &name);
                        apply_named_export(&mut config, &name, None, file);
                    }
                }
                Declaration::ClassDeclaration(c) => {
                    if let Some(id) = &c.id {
                        push_unique(&mut config.named_exports, id.name.as_str());
                    }
                }
                Declaration::TSEnumDeclaration(e) => {
                    push_unique(&mut config.named_exports, e.id.name.as_str());
                }
                _ => {}
            },
            Statement::ExportNamedDeclaration(export) => {
                for specifier in &export.specifiers {
                    if export.export_kind.is_type() || specifier.export_kind.is_type() {
                        continue;
                    }
                    let exported = export_name(&specifier.exported);
                    let local = export_name(&specifier.local);
                    if exported == "default" {
                        config.default_export_async =
                            functions.get(&local).copied().unwrap_or(false);
                        config.default_export = Some(local);
                    } else {
                        apply_named_export(&mut config, &exported, None, file);
                        push_unique(&mut config.named_exports, &exported);
                    }
                }
            }
            Statement::ExportFromDeclaration(export) => {
                for specifier in &export.specifiers {
                    if export.export_kind.is_type() || specifier.export_kind.is_type() {
                        continue;
                    }
                    let exported = export_name(&specifier.exported);
                    if exported == "default" {
                        config.default_export = Some(export_name(&specifier.local));
                    } else {
                        apply_named_export(&mut config, &exported, None, file);
                        push_unique(&mut config.named_exports, &exported);
                    }
                }
            }
            Statement::ExportAllDeclaration(export) => {
                let name = export
                    .exported
                    .as_ref()
                    .map_or_else(|| String::from("*"), export_name);
                push_unique(&mut config.named_exports, &name);
            }
            Statement::ExportDefaultDeclaration(export) => {
                if let Some((name, is_async)) =
                    default_export_facts(&export.declaration, &functions)
                {
                    config.default_export = Some(name);
                    config.default_export_async = is_async;
                }
            }
            Statement::ExpressionStatement(statement) => {
                if let Expression::AssignmentExpression(assignment) = &statement.expression
                    && let AssignmentTarget::StaticMemberExpression(member) = &assignment.left
                    && member.property.name == "getInitialProps"
                {
                    get_initial_props_assigned = true;
                }
            }
            _ => {}
        }
    }

    if collect_hooks && let Ok(analysis) = yolo_react::analyze_react_component(source) {
        config.hooks_used = analysis.hooks_used;
    }

    ModuleFacts { config, get_initial_props_assigned, parse_errors }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn directives_and_config_exports() {
        let source = r##""use client";
import Link from "next/link";
import { useRouter } from "next/navigation";
import "server-only";
import { useState } from "react";

export const dynamic = "force-dynamic";
export const dynamicParams = false;
export const revalidate = 3600;
export const fetchCache = "force-no-store";
export const runtime = "edge";
export const preferredRegion = ["iad1", "sfo1"];
export const maxDuration = 30;
export const ensureStatic = true;
export const prefetch = "static";
export const instant = true;
export const experimental_ppr = true;
export const metadata = { title: { default: "Home", template: "%s | Site" }, description: "Desc", keywords: "one" };
export const viewport = { width: "device-width", initialScale: 1, themeColor: "#000" };

export default function Page() {
  const [n] = useState(0);
  const router = useRouter();
  return <Link href="/">{n}</Link>;
}
"##;
        let config = analyze_segment_module(source, "app/page.tsx");
        assert_eq!(config.directive, ModuleDirective::UseClient);
        assert_eq!(config.dynamic.as_deref(), Some("force-dynamic"));
        assert_eq!(config.dynamic_params, Some(false));
        assert_eq!(config.revalidate, Some(Value::from(3600)));
        assert_eq!(config.fetch_cache.as_deref(), Some("force-no-store"));
        assert_eq!(config.runtime.as_deref(), Some("edge"));
        assert_eq!(
            config.preferred_region,
            Some(serde_json::json!(["iad1", "sfo1"]))
        );
        assert_eq!(config.max_duration, Some(30));
        assert_eq!(config.ensure_static, Some(Value::Bool(true)));
        assert_eq!(config.prefetch.as_deref(), Some("static"));
        assert_eq!(config.instant, Some(true));
        assert_eq!(config.experimental_ppr, Some(true));
        let metadata = config.static_metadata.as_ref().expect("static metadata");
        assert_eq!(
            metadata.title.as_ref().and_then(|t| t.template.as_deref()),
            Some("%s | Site")
        );
        assert_eq!(metadata.description.as_deref(), Some("Desc"));
        assert_eq!(metadata.keywords, vec!["one"]);
        let viewport = config.static_viewport.as_ref().expect("static viewport");
        assert_eq!(viewport.width.as_deref(), Some("device-width"));
        assert_eq!(viewport.theme_color.len(), 1);
        assert_eq!(config.default_export.as_deref(), Some("Page"));
        assert!(!config.default_export_async);
        assert_eq!(config.hooks_used, vec!["useState", "useRouter"]);
        assert_eq!(
            config.next_imports,
            vec!["next/link", "next/navigation", "server-only"]
        );
        let codes: Vec<&str> = config.diagnostics.iter().map(|d| d.code.as_str()).collect();
        assert!(codes.contains(&"segment-config-edge-runtime-deprecated"));
        assert!(codes.contains(&"segment-config-preferred-region-deprecated"));
        assert!(codes.contains(&"segment-config-experimental-ppr-removed"));
        assert!(!codes.contains(&"segment-parse-error"));
    }

    #[test]
    fn generate_exports_and_default_forms() {
        let source = r#"
export async function generateStaticParams() { return []; }
export const generateMetadata = async () => ({ title: "x" });
export function generateViewport() { return {}; }
export async function generateImageMetadata() { return []; }
export async function generateSitemaps() { return []; }
async function Page() { return null; }
export default Page;
export const revalidate = false;
export { Page as Alias };
export * from "./shared";
export * as ns from "./ns";
export type { Foo } from "./types";
"#;
        let config = analyze_segment_module(source, "app/sitemap.ts");
        assert!(config.has_generate_static_params);
        assert!(config.has_generate_metadata);
        assert!(config.has_generate_viewport);
        assert!(config.has_generate_image_metadata);
        assert!(config.has_generate_sitemaps);
        assert_eq!(config.default_export.as_deref(), Some("Page"));
        assert!(config.default_export_async);
        assert_eq!(config.revalidate, Some(Value::Bool(false)));
        assert!(config.named_exports.contains(&"Alias".to_owned()));
        assert!(config.named_exports.contains(&"*".to_owned()));
        assert!(config.named_exports.contains(&"ns".to_owned()));
        assert!(!config.named_exports.contains(&"Foo".to_owned()));

        let anon = analyze_segment_module("export default async function () { return 1 }", "a.ts");
        assert_eq!(anon.default_export.as_deref(), Some("default"));
        assert!(anon.default_export_async);
        let arrow = analyze_segment_module("export default async () => 1", "a.ts");
        assert!(arrow.default_export_async);
        let call = analyze_segment_module(
            "import dynamic from \"next/dynamic\";\nexport default dynamic(() => import(\"./x\"));",
            "a.tsx",
        );
        assert_eq!(call.default_export.as_deref(), Some("default"));
        assert_eq!(call.next_imports, vec!["next/dynamic"]);
        let reexport = analyze_segment_module("export { default } from \"./x\";", "a.ts");
        assert_eq!(reexport.default_export.as_deref(), Some("default"));
    }

    #[test]
    fn directive_variants() {
        assert_eq!(
            analyze_segment_module("\"use server\";\nexport async function act() {}", "a.ts")
                .directive,
            ModuleDirective::UseServer
        );
        assert_eq!(
            analyze_segment_module(
                "\"use cache: private\";\nexport async function get() {}",
                "a.ts"
            )
            .directive,
            ModuleDirective::UseCache
        );
        assert_eq!(
            analyze_segment_module("export const x = 1;", "a.ts").directive,
            ModuleDirective::None
        );
    }

    #[test]
    fn dynamic_metadata_is_reported() {
        let source = "export const metadata = buildMetadata();\nexport const viewport = computeViewport(\"mobile\");\n";
        let config = analyze_segment_module(source, "app/layout.tsx");
        assert!(config.static_metadata.is_none());
        assert!(config.static_viewport.is_none());
        let codes: Vec<&str> = config.diagnostics.iter().map(|d| d.code.as_str()).collect();
        assert!(codes.contains(&"segment-metadata-dynamic"));
        assert!(codes.contains(&"segment-viewport-dynamic"));
    }

    #[test]
    fn parse_errors_are_diagnostics() {
        let config = analyze_segment_module("export const = ;", "app/page.tsx");
        assert!(
            config
                .diagnostics
                .iter()
                .any(|d| d.code == "segment-parse-error")
        );
    }

    #[test]
    fn mdx_is_skipped() {
        let config = analyze_segment_module("# Title", "app/page.mdx");
        assert_eq!(config.default_export.as_deref(), Some("MDXContent"));
        assert!(
            config
                .diagnostics
                .iter()
                .any(|d| d.code == "segment-mdx-skipped")
        );
    }

    #[test]
    fn route_handler_methods_in_canonical_order() {
        let source = r"
export async function DELETE() {}
export const PATCH = async () => {};
async function post() {}
export { post as POST };
export async function GET() {}
export function helper() {}
";
        assert_eq!(
            route_handler_methods(source),
            vec!["GET", "POST", "PATCH", "DELETE"]
        );
        assert_eq!(
            route_handler_methods("export const x = 1;"),
            Vec::<String>::new()
        );
        let tsx = "export async function GET() { return <div/>; }";
        assert_eq!(route_handler_methods(tsx), vec!["GET"]);
    }

    #[test]
    fn legacy_data_functions_forms() {
        let source = r"
export async function getStaticPaths() { return { paths: [], fallback: false }; }
export const getStaticProps = async () => ({ props: {} });
function Page() { return <p/>; }
Page.getInitialProps = async () => ({});
export default Page;
";
        assert_eq!(
            legacy_data_functions(source),
            vec!["getStaticProps", "getStaticPaths", "getInitialProps"]
        );
        assert_eq!(
            legacy_data_functions(
                "export async function getServerSideProps() { return { props: {} }; }"
            ),
            vec!["getServerSideProps"]
        );
        assert_eq!(
            legacy_data_functions("export default function P() { return null }"),
            Vec::<String>::new()
        );
    }

    #[test]
    fn literal_evaluation_handles_urls_and_negatives() {
        let config = analyze_segment_module(
            "export const metadata = { metadataBase: new URL(\"https://x.dev\"), other: { n: -2 } } satisfies Record<string, unknown>;",
            "app/layout.tsx",
        );
        let metadata = config.static_metadata.expect("metadata");
        assert_eq!(metadata.metadata_base.as_deref(), Some("https://x.dev"));
        assert_eq!(metadata.other.get("n"), Some(&Value::from(-2)));
    }
}
