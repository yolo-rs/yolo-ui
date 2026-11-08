// SPDX-License-Identifier: Apache-2.0
//! Turbopack configuration (`rules`, `resolveAlias`, `resolveExtensions`, `root`) -> Bun bundler plan.
//!
//! Turbopack rules are `glob -> { loaders: (string | { loader, options })[], as?, type?, condition? }`.
//! Each loader becomes a [`BunPluginSpec`] (`Bun.build` plugin) or a plain loader override
//! (`Bun.build({ loader: { ".yaml": "yaml" } })`). `resolveAlias` becomes `onResolve` aliases,
//! `compiler.define` + `env` become `define` entries and `serverExternalPackages` stay external.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use serde_json::{Map, Value, json};

use crate::types::{
    BunBundlerPlan, BunLoader, BunPluginKind, BunPluginSpec, FeatureStatus, NextConfig, Support,
    TurbopackRule, YoloWebConfig,
};

/// Default `resolveExtensions` when the config does not override them (Bun order).
const DEFAULT_EXTENSIONS: [&str; 6] = [".tsx", ".ts", ".jsx", ".js", ".mjs", ".json"];

/// Maps a webpack/Turbopack loader package to a Bun plugin kind.
pub fn map_turbopack_loader(loader: &str) -> (BunPluginKind, Option<BunLoader>, Support) {
    match loader {
        "@svgr/webpack" => (
            BunPluginKind::SvgReact,
            Some(BunLoader::Tsx),
            Support::Mapped,
        ),
        "svg-inline-loader"
        | "raw-loader"
        | "asset/source"
        | "raw"
        | "glslify-loader"
        | "webpack-glsl-loader" => (BunPluginKind::Text, Some(BunLoader::Text), Support::Mapped),
        "graphql-tag/loader" => (BunPluginKind::Text, Some(BunLoader::Text), Support::Partial),
        "yaml-loader" => (
            BunPluginKind::LoaderOverride,
            Some(BunLoader::Yaml),
            Support::Mapped,
        ),
        "@mdx-js/loader" | "@next/mdx" => (
            BunPluginKind::Markdown,
            Some(BunLoader::Tsx),
            Support::Partial,
        ),
        "file-loader" | "url-loader" | "asset/resource" | "asset" => (
            BunPluginKind::LoaderOverride,
            Some(BunLoader::File),
            Support::Mapped,
        ),
        "json5-loader" => (
            BunPluginKind::LoaderOverride,
            Some(BunLoader::Jsonc),
            Support::Partial,
        ),
        "toml-loader" => (
            BunPluginKind::LoaderOverride,
            Some(BunLoader::Toml),
            Support::Mapped,
        ),
        "babel-loader" => (BunPluginKind::Unsupported, None, Support::Partial),
        _ => (BunPluginKind::Unsupported, None, Support::Unsupported),
    }
}

/// Human note attached to the feature entry of a loader mapping.
fn loader_note(loader: &str) -> String {
    match loader {
        "@svgr/webpack" => String::from("SVG -> React component via yolo:svg-react plugin"),
        "graphql-tag/loader" => String::from("exports source string; use graphql() at runtime"),
        "yaml-loader" => String::from("Bun yaml loader"),
        "@mdx-js/loader" | "@next/mdx" => String::from("Bun.markdown + mdx subset"),
        "sass-loader" => String::from(
            "Bun has no Sass; precompile with `sass` CLI or use CSS nesting (Lightning CSS)",
        ),
        "babel-loader" => String::from(
            "Bun transpiles TS/JSX natively; React Compiler via Bun.build reactCompiler",
        ),
        "json5-loader" => {
            String::from("Bun jsonc loader (JSON5 subset: comments, trailing commas)")
        }
        "string-replace-loader" => {
            String::from("no Bun equivalent; apply the replacement in source")
        }
        "file-loader" | "url-loader" | "asset/resource" | "asset" => {
            String::from("Bun file loader (import yields the asset URL)")
        }
        "toml-loader" => String::from("Bun toml loader"),
        "raw-loader"
        | "asset/source"
        | "svg-inline-loader"
        | "raw"
        | "glslify-loader"
        | "webpack-glsl-loader" => String::from("Bun text loader"),
        other => format!("loader `{other}` has no Bun plugin; write a Bun.plugin onLoad hook"),
    }
}

/// Default Bun loader for a file extension (`svg` -> `file`, `md` -> `text`, ...).
pub fn loader_for_extension(ext: &str) -> Option<BunLoader> {
    let ext = ext.trim_start_matches('.').to_ascii_lowercase();
    Some(match ext.as_str() {
        "js" | "mjs" | "cjs" => BunLoader::Js,
        "jsx" => BunLoader::Jsx,
        "ts" | "mts" | "cts" => BunLoader::Ts,
        "tsx" => BunLoader::Tsx,
        "json" => BunLoader::Json,
        "jsonc" => BunLoader::Jsonc,
        "toml" => BunLoader::Toml,
        "yaml" | "yml" => BunLoader::Yaml,
        "css" => BunLoader::Css,
        "html" => BunLoader::Html,
        "wasm" => BunLoader::Wasm,
        "txt" | "md" => BunLoader::Text,
        "svg" | "png" | "jpg" | "jpeg" | "gif" | "webp" | "avif" | "ico" | "woff" | "woff2"
        | "ttf" | "otf" | "mp4" | "webm" => BunLoader::File,
        "node" => BunLoader::Napi,
        _ => return None,
    })
}

/// `rules[glob].type` module type -> Bun loader.
fn loader_for_module_type(module_type: &str) -> Option<BunLoader> {
    Some(match module_type {
        "raw" | "text" => BunLoader::Text,
        "bytes" | "asset" => BunLoader::File,
        "json" => BunLoader::Json,
        "css" => BunLoader::Css,
        "wasm" => BunLoader::Wasm,
        "ecmascript" | "javascript" => BunLoader::Js,
        "typescript" => BunLoader::Ts,
        _ => return None,
    })
}

fn escape_regex_char(c: char, out: &mut String) {
    if matches!(
        c,
        '.' | '+' | '?' | '^' | '$' | '(' | ')' | '[' | ']' | '{' | '}' | '|' | '\\'
    ) {
        out.push('\\');
    }
    out.push(c);
}

/// Converts a Turbopack glob into a JavaScript regex source for `Bun.plugin` filters.
///
/// `*.svg` -> `\.svg$`, `**/*.md` -> `\.md$`, `src/**/*.graphql` -> `src/.*\.graphql$`,
/// `{a,b}` -> `(?:a|b)`. Globs without `/` match the basename.
pub(crate) fn glob_to_regex_source(glob: &str) -> String {
    let mut rest = glob;
    while let Some(stripped) = rest.strip_prefix("**/") {
        rest = stripped;
    }
    if !rest.contains('/') {
        rest = rest.trim_start_matches('*');
    }
    let chars: Vec<char> = rest.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '*' {
            if chars.get(i + 1) == Some(&'*') {
                // `**/*` -> `.*`, `**/` -> `(?:.*/)?`, bare `**` -> `.*`
                if chars.get(i + 2) == Some(&'/') {
                    if chars.get(i + 3) == Some(&'*') {
                        out.push_str(".*");
                        i += 4;
                    } else {
                        out.push_str("(?:.*/)?");
                        i += 3;
                    }
                } else {
                    out.push_str(".*");
                    i += 2;
                }
            } else {
                out.push_str("[^/]*");
                i += 1;
            }
        } else if c == '?' {
            out.push_str("[^/]");
            i += 1;
        } else if c == '{' {
            let mut j = i + 1;
            let mut inner = String::new();
            while j < chars.len() && chars[j] != '}' {
                inner.push(chars[j]);
                j += 1;
            }
            if j < chars.len() {
                let alternatives: Vec<String> = inner
                    .split(',')
                    .map(|alt| {
                        let mut escaped = String::new();
                        alt.chars()
                            .for_each(|ch| escape_regex_char(ch, &mut escaped));
                        escaped
                    })
                    .collect();
                _ = write!(out, "(?:{})", alternatives.join("|"));
                i = j + 1;
            } else {
                out.push_str("\\{");
                i += 1;
            }
        } else {
            escape_regex_char(c, &mut out);
            i += 1;
        }
    }
    out.push('$');
    out
}

/// Extension targeted by a glob (`*.svg` -> `svg`, `*.{png,jpg}` -> `png-jpg`); falls back to a
/// sanitized glob when none can be derived.
fn glob_extension(glob: &str) -> String {
    let basename = glob.rsplit('/').next().unwrap_or(glob);
    if let Some((_, ext)) = basename.rsplit_once('.') {
        let ext = ext.trim_matches(|c| c == '{' || c == '}');
        if !ext.is_empty() && !ext.contains(['*', '?', '[']) {
            return ext.replace(',', "-");
        }
    }
    let sanitized: String = glob
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    sanitized.trim_matches('-').to_owned()
}

fn kind_id(kind: BunPluginKind) -> &'static str {
    match kind {
        BunPluginKind::AliasResolve => "alias-resolve",
        BunPluginKind::LoaderOverride => "loader-override",
        BunPluginKind::SvgReact => "svg-react",
        BunPluginKind::Markdown => "markdown",
        BunPluginKind::Text => "text",
        BunPluginKind::Unsupported => "unsupported",
    }
}

fn feature(feature: impl Into<String>, support: Support, note: impl Into<String>) -> FeatureStatus {
    FeatureStatus { feature: feature.into(), support, note: note.into(), target: None }
}

fn translate_rule(glob: &str, rule: &TurbopackRule, plan: &mut BunBundlerPlan) {
    let filter = glob_to_regex_source(glob);
    let ext = glob_extension(glob);
    let loader_key = format!(".{ext}");
    if let Some(module_type) = rule.type_.as_deref() {
        match loader_for_module_type(module_type) {
            Some(loader) => {
                plan.loaders.insert(loader_key.clone(), loader);
            }
            None => plan.unsupported.push(feature(
                format!("turbopack.rules[{glob}].type"),
                Support::Unsupported,
                format!("module type `{module_type}` has no Bun loader"),
            )),
        }
    }
    for loader in &rule.loaders {
        let (kind, bun_loader, support) = map_turbopack_loader(&loader.loader);
        let mut options = loader.options.clone();
        if let Some(as_) = &rule.as_ {
            // `as: "*.js"` is implicit for Bun plugins (onLoad returns JS/TSX); kept for reference.
            options.insert(String::from("as"), Value::String(as_.clone()));
        }
        if let Some(condition) = &rule.condition {
            options.insert(String::from("condition"), condition.clone());
        }
        if kind == BunPluginKind::LoaderOverride
            && let Some(bun_loader) = bun_loader
        {
            plan.loaders.insert(loader_key.clone(), bun_loader);
        }
        plan.plugins.push(BunPluginSpec {
            name: format!("yolo:{}:{ext}", kind_id(kind)),
            kind,
            filter: filter.clone(),
            origin_loader: Some(loader.loader.clone()),
            loader: bun_loader,
            options,
        });
        if support != Support::Mapped {
            plan.unsupported.push(feature(
                format!("turbopack.rules[{glob}] {}", loader.loader),
                support,
                loader_note(&loader.loader),
            ));
        }
    }
}

/// First string found in a (possibly nested) alias value.
fn first_string(value: &Value) -> Option<String> {
    match value {
        Value::String(s) => Some(s.clone()),
        Value::Array(items) => items.iter().find_map(first_string),
        Value::Object(map) => map.values().find_map(first_string),
        _ => None,
    }
}

fn translate_aliases(aliases: &BTreeMap<String, Value>, plan: &mut BunBundlerPlan) {
    let mut conditions: Vec<String> = Vec::new();
    for (name, value) in aliases {
        match value {
            Value::String(target) => {
                plan.aliases.insert(name.clone(), target.clone());
            }
            Value::Bool(false) => plan.empty_modules.push(name.clone()),
            Value::Array(items) => {
                if let Some(target) = items.iter().find_map(first_string) {
                    plan.aliases.insert(name.clone(), target);
                }
                plan.unsupported.push(feature(
                    format!("turbopack.resolveAlias[{name}]"),
                    Support::Partial,
                    "array alias: Bun resolves to the first entry only",
                ));
            }
            Value::Object(map) => {
                for (condition, target) in map {
                    let Some(target) = first_string(target) else {
                        if target == &Value::Bool(false) && condition == "browser" {
                            plan.empty_modules.push(name.clone());
                        }
                        continue;
                    };
                    match condition.as_str() {
                        "browser" => {
                            plan.browser_aliases.insert(name.clone(), target);
                        }
                        "default" | "import" | "require" | "node" | "module" => {
                            plan.aliases.entry(name.clone()).or_insert(target);
                            if condition != "default" {
                                conditions.push(format!("{name}:{condition}"));
                            }
                        }
                        other => conditions.push(format!("{name}:{other}")),
                    }
                }
            }
            _ => {}
        }
    }
    if !conditions.is_empty() {
        plan.unsupported.push(feature(
            "turbopack.resolveAlias conditions",
            Support::Partial,
            format!(
                "Bun applies universal + browser aliases; other conditions are ignored: {}",
                conditions.join(", ")
            ),
        ));
    }
}

/// Builds the regex filter of the alias plugin (`^(?:underscore|@/.*)$`).
fn alias_filter(plan: &BunBundlerPlan) -> String {
    let mut names: Vec<&String> = plan
        .aliases
        .keys()
        .chain(plan.browser_aliases.keys())
        .chain(plan.empty_modules.iter())
        .collect();
    names.sort();
    names.dedup();
    let alternatives: Vec<String> = names
        .into_iter()
        .map(|name| {
            let mut out = String::new();
            for c in name.chars() {
                if c == '*' {
                    out.push_str(".*");
                } else {
                    escape_regex_char(c, &mut out);
                }
            }
            out
        })
        .collect();
    format!("^(?:{})$", alternatives.join("|"))
}

/// Translates the Turbopack section (plus `compiler.define`, `env`, `serverExternalPackages`) into a plan.
pub fn translate_turbopack(cfg: &NextConfig) -> BunBundlerPlan {
    let mut plan = BunBundlerPlan::default();
    let turbo = &cfg.turbopack;

    translate_aliases(&turbo.resolve_alias, &mut plan);
    if !plan.aliases.is_empty()
        || !plan.browser_aliases.is_empty()
        || !plan.empty_modules.is_empty()
    {
        let mut options = Map::new();
        options.insert(String::from("aliases"), json!(plan.aliases));
        options.insert(String::from("browserAliases"), json!(plan.browser_aliases));
        options.insert(String::from("emptyModules"), json!(plan.empty_modules));
        plan.plugins.push(BunPluginSpec {
            name: String::from("yolo:alias-resolve"),
            kind: BunPluginKind::AliasResolve,
            filter: alias_filter(&plan),
            origin_loader: None,
            loader: None,
            options,
        });
    }

    for (glob, rules) in &turbo.rules {
        for rule in rules {
            translate_rule(glob, rule, &mut plan);
        }
    }

    plan.extensions = if turbo.resolve_extensions.is_empty() {
        DEFAULT_EXTENSIONS.iter().map(|e| (*e).to_owned()).collect()
    } else {
        turbo.resolve_extensions.clone()
    };
    plan.root.clone_from(&turbo.root);

    for (key, value) in cfg
        .compiler
        .define
        .iter()
        .chain(&cfg.compiler.define_server)
    {
        plan.define.insert(key.clone(), value.clone());
    }
    for (key, value) in &cfg.env {
        plan.define.insert(
            format!("process.env.{key}"),
            serde_json::to_string(value).unwrap_or_else(|_| format!("\"{value}\"")),
        );
    }
    plan.external.clone_from(&cfg.server_external_packages);

    if turbo.debug_ids == Some(true) {
        plan.unsupported.push(feature(
            "turbopack.debugIds",
            Support::Partial,
            "Bun.build emits linked sourcemaps; TC39 debug ids are not configurable",
        ));
    }
    if turbo.chunk_loading_global.is_some() {
        plan.unsupported.push(feature(
            "turbopack.chunkLoadingGlobal",
            Support::Unsupported,
            "Bun emits ESM chunks without a chunk loading global",
        ));
    }
    if !turbo.ignore_issue.is_empty() {
        plan.unsupported.push(feature(
            "turbopack.ignoreIssue",
            Support::Unsupported,
            "Bun.build has no issue filter; inspect the build log instead",
        ));
    }
    plan
}

/// Emits `bunfig.toml` (`[serve.static]` plugins/env/sourcemap, `[install]`, `[test]`).
///
/// Bun accepts exactly one `env` prefix pattern; additional public prefixes are listed as a
/// comment.
pub fn bunfig_toml(plan: &BunBundlerPlan, cfg: &YoloWebConfig) -> String {
    let mut prefixes = cfg.public_env_prefixes.iter();
    let first = prefixes
        .next()
        .cloned()
        .unwrap_or_else(|| String::from("NEXT_PUBLIC_"));
    let others: Vec<String> = prefixes.map(|p| format!("{p}*")).collect();
    let mut out =
        String::from("# Generated by yolo-next (Bun 1.4.4) from next.config turbopack settings.\n");
    _ = writeln!(
        out,
        "# {} plugin(s), {} loader override(s), {} alias(es).",
        plan.plugins.len(),
        plan.loaders.len(),
        plan.aliases.len() + plan.browser_aliases.len()
    );
    out.push_str("[serve.static]\nplugins = [\"./yolo.bundler.ts\"]\n");
    _ = writeln!(out, "env = \"{first}*\"\nsourcemap = \"linked\"");
    if !others.is_empty() {
        _ = writeln!(
            out,
            "# Bun supports one env prefix pattern; also public (inlined by yolo.bundler.ts): {}",
            others.join(", ")
        );
    }
    out.push_str("\n[install]\nexact = true\n\n[test]\nroot = \"./tests\"\n");
    out
}

/// Escapes a string for a double-quoted TypeScript literal.
fn ts_string(value: &str) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| format!("\"{value}\""))
}

fn bun_loader_id(loader: BunLoader) -> &'static str {
    match loader {
        BunLoader::Js => "js",
        BunLoader::Jsx => "jsx",
        BunLoader::Ts => "ts",
        BunLoader::Tsx => "tsx",
        BunLoader::Json => "json",
        BunLoader::Jsonc => "jsonc",
        BunLoader::Toml => "toml",
        BunLoader::Yaml => "yaml",
        BunLoader::Text => "text",
        BunLoader::File => "file",
        BunLoader::Napi => "napi",
        BunLoader::Wasm => "wasm",
        BunLoader::Css => "css",
        BunLoader::Html => "html",
    }
}

/// Emits `yolo.bundler.ts` exporting the `BunPlugin[]` array built from the plan.
pub fn bundler_plugins_ts(plan: &BunBundlerPlan) -> String {
    let mut out = String::from(
        "// Generated by yolo-next: Turbopack rules and resolveAlias translated to Bun plugins.\n\
         // Register with Bun.build({ plugins: yoloBundlerPlugins }) or bunfig.toml [serve.static].\n\
         import {\n  aliasPlugin,\n  loaderOverridePlugin,\n  markdownPlugin,\n  svgReactPlugin,\n  textPlugin,\n} from \"@yolo-rs/next/turbopack\";\n\n",
    );
    out.push_str("export const yoloBundlerPlugins: import(\"bun\").BunPlugin[] = [\n");
    if !plan.aliases.is_empty()
        || !plan.browser_aliases.is_empty()
        || !plan.empty_modules.is_empty()
    {
        _ = writeln!(
            out,
            "  aliasPlugin({}, {}, {}),",
            serde_json::to_string(&plan.aliases).unwrap_or_else(|_| String::from("{}")),
            serde_json::to_string(&plan.browser_aliases).unwrap_or_else(|_| String::from("{}")),
            serde_json::to_string(&plan.empty_modules).unwrap_or_else(|_| String::from("[]")),
        );
    }
    for spec in &plan.plugins {
        let filter = format!("new RegExp({})", ts_string(&spec.filter));
        let origin = spec.origin_loader.as_deref().unwrap_or("rule");
        match spec.kind {
            BunPluginKind::AliasResolve => {}
            BunPluginKind::SvgReact => _ = writeln!(out, "  svgReactPlugin({filter}), // {origin}"),
            BunPluginKind::Text => _ = writeln!(out, "  textPlugin({filter}), // {origin}"),
            BunPluginKind::Markdown => _ = writeln!(out, "  markdownPlugin({filter}), // {origin}"),
            BunPluginKind::LoaderOverride => {
                let loader = spec.loader.map_or("file", bun_loader_id);
                _ = writeln!(
                    out,
                    "  loaderOverridePlugin({filter}, \"{loader}\"), // {origin}"
                );
            }
            BunPluginKind::Unsupported => {
                _ = writeln!(
                    out,
                    "  // unsupported: {origin} for {} ({})",
                    ts_string(&spec.filter),
                    loader_note(origin)
                );
            }
        }
    }
    out.push_str("];\n\nexport default yoloBundlerPlugins;\n");
    out
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::types::{CompilerConfig, TurbopackConfig, TurbopackLoader};

    fn rule(loader: &str) -> Vec<TurbopackRule> {
        vec![TurbopackRule {
            loaders: vec![TurbopackLoader { loader: loader.to_owned(), options: Map::new() }],
            as_: Some(String::from("*.js")),
            type_: None,
            condition: None,
        }]
    }

    fn fixture_config() -> NextConfig {
        let mut rules = BTreeMap::new();
        rules.insert(String::from("*.svg"), rule("@svgr/webpack"));
        rules.insert(String::from("*.md"), rule("raw-loader"));
        rules.insert(String::from("*.graphql"), rule("graphql-tag/loader"));
        rules.insert(String::from("*.yaml"), rule("yaml-loader"));
        rules.insert(String::from("*.scss"), rule("sass-loader"));
        let mut resolve_alias = BTreeMap::new();
        resolve_alias.insert(String::from("underscore"), json!("lodash"));
        resolve_alias.insert(
            String::from("mocha"),
            json!({ "browser": "mocha/browser-entry.js" }),
        );
        resolve_alias.insert(String::from("fs"), json!(false));
        resolve_alias.insert(String::from("@/*"), json!("./*"));
        resolve_alias.insert(
            String::from("pkg"),
            json!({ "import": "pkg/esm", "node": "pkg/node" }),
        );
        resolve_alias.insert(String::from("arr"), json!(["a", "b"]));
        let mut env = BTreeMap::new();
        env.insert(String::from("APP_NAME"), String::from("Yolo Fixture"));
        let mut define = BTreeMap::new();
        define.insert(String::from("__BUILD_FLAG__"), String::from("true"));
        NextConfig {
            turbopack: TurbopackConfig {
                root: Some(String::from(".")),
                rules,
                resolve_alias,
                resolve_extensions: vec![String::from(".mdx"), String::from(".tsx")],
                debug_ids: Some(true),
                ..TurbopackConfig::default()
            },
            env,
            compiler: CompilerConfig { define, ..CompilerConfig::default() },
            server_external_packages: vec![String::from("sharp")],
            ..NextConfig::default()
        }
    }

    #[test]
    fn loader_mapping_table() {
        assert_eq!(
            map_turbopack_loader("@svgr/webpack"),
            (
                BunPluginKind::SvgReact,
                Some(BunLoader::Tsx),
                Support::Mapped
            )
        );
        assert_eq!(
            map_turbopack_loader("raw-loader"),
            (BunPluginKind::Text, Some(BunLoader::Text), Support::Mapped)
        );
        assert_eq!(
            map_turbopack_loader("yaml-loader"),
            (
                BunPluginKind::LoaderOverride,
                Some(BunLoader::Yaml),
                Support::Mapped
            )
        );
        assert_eq!(
            map_turbopack_loader("graphql-tag/loader").2,
            Support::Partial
        );
        assert_eq!(
            map_turbopack_loader("@mdx-js/loader").0,
            BunPluginKind::Markdown
        );
        assert_eq!(
            map_turbopack_loader("sass-loader"),
            (BunPluginKind::Unsupported, None, Support::Unsupported)
        );
        assert_eq!(
            map_turbopack_loader("babel-loader"),
            (BunPluginKind::Unsupported, None, Support::Partial)
        );
        assert_eq!(map_turbopack_loader("file-loader").1, Some(BunLoader::File));
        assert_eq!(
            map_turbopack_loader("json5-loader"),
            (
                BunPluginKind::LoaderOverride,
                Some(BunLoader::Jsonc),
                Support::Partial
            )
        );
        assert_eq!(map_turbopack_loader("toml-loader").1, Some(BunLoader::Toml));
        assert_eq!(
            map_turbopack_loader("whatever-loader").2,
            Support::Unsupported
        );
    }

    #[test]
    fn extension_loaders() {
        assert_eq!(loader_for_extension("svg"), Some(BunLoader::File));
        assert_eq!(loader_for_extension(".md"), Some(BunLoader::Text));
        assert_eq!(loader_for_extension("yml"), Some(BunLoader::Yaml));
        assert_eq!(loader_for_extension("node"), Some(BunLoader::Napi));
        assert_eq!(loader_for_extension("tsx"), Some(BunLoader::Tsx));
        assert_eq!(loader_for_extension("woff2"), Some(BunLoader::File));
        assert_eq!(loader_for_extension("unknown"), None);
    }

    #[test]
    fn glob_regex_conversion() {
        assert_eq!(glob_to_regex_source("*.svg"), "\\.svg$");
        assert_eq!(glob_to_regex_source("**/*.md"), "\\.md$");
        assert_eq!(
            glob_to_regex_source("src/**/*.graphql"),
            "src/.*\\.graphql$"
        );
        assert_eq!(glob_to_regex_source("icon-*.svg"), "icon-[^/]*\\.svg$");
        assert_eq!(glob_to_regex_source("*.{png,jpg}"), "\\.(?:png|jpg)$");
        assert_eq!(glob_to_regex_source("assets/*.txt"), "assets/[^/]*\\.txt$");
        assert_eq!(glob_extension("*.svg"), "svg");
        assert_eq!(glob_extension("*.{png,jpg}"), "png-jpg");
        assert_eq!(glob_extension("src/**/*.graphql"), "graphql");
    }

    #[test]
    fn translation_of_rules_and_aliases() {
        let plan = translate_turbopack(&fixture_config());
        let svg = plan
            .plugins
            .iter()
            .find(|p| p.name == "yolo:svg-react:svg")
            .unwrap();
        assert_eq!(svg.filter, "\\.svg$");
        assert_eq!(svg.loader, Some(BunLoader::Tsx));
        assert_eq!(svg.origin_loader.as_deref(), Some("@svgr/webpack"));
        assert_eq!(svg.options.get("as"), Some(&json!("*.js")));
        let md = plan
            .plugins
            .iter()
            .find(|p| p.name == "yolo:text:md")
            .unwrap();
        assert_eq!(md.kind, BunPluginKind::Text);
        let yaml = plan
            .plugins
            .iter()
            .find(|p| p.name == "yolo:loader-override:yaml")
            .unwrap();
        assert_eq!(yaml.loader, Some(BunLoader::Yaml));
        assert_eq!(plan.loaders.get(".yaml"), Some(&BunLoader::Yaml));
        assert!(
            !plan.loaders.contains_key(".svg"),
            "svg-react is a plugin, not a loader override"
        );
        assert!(
            plan.plugins
                .iter()
                .any(|p| p.name == "yolo:unsupported:scss")
        );
        assert!(
            plan.unsupported
                .iter()
                .any(|f| f.feature.contains("sass-loader") && f.support == Support::Unsupported)
        );
        assert!(
            plan.unsupported
                .iter()
                .any(|f| f.feature.contains("graphql-tag/loader") && f.support == Support::Partial)
        );

        assert_eq!(
            plan.aliases.get("underscore").map(String::as_str),
            Some("lodash")
        );
        assert_eq!(plan.aliases.get("@/*").map(String::as_str), Some("./*"));
        assert_eq!(
            plan.browser_aliases.get("mocha").map(String::as_str),
            Some("mocha/browser-entry.js")
        );
        assert_eq!(plan.empty_modules, vec![String::from("fs")]);
        assert_eq!(plan.aliases.get("arr").map(String::as_str), Some("a"));
        assert_eq!(plan.aliases.get("pkg").map(String::as_str), Some("pkg/esm"));
        assert!(
            plan.unsupported
                .iter()
                .any(|f| f.feature == "turbopack.resolveAlias conditions")
        );
        assert!(
            plan.unsupported
                .iter()
                .any(|f| f.feature == "turbopack.resolveAlias[arr]")
        );
        assert_eq!(plan.plugins[0].kind, BunPluginKind::AliasResolve);
        assert!(plan.plugins[0].filter.contains("underscore"));

        assert_eq!(
            plan.extensions,
            vec![String::from(".mdx"), String::from(".tsx")]
        );
        assert_eq!(plan.root.as_deref(), Some("."));
        assert_eq!(
            plan.define.get("__BUILD_FLAG__").map(String::as_str),
            Some("true")
        );
        assert_eq!(
            plan.define.get("process.env.APP_NAME").map(String::as_str),
            Some("\"Yolo Fixture\"")
        );
        assert_eq!(plan.external, vec![String::from("sharp")]);
        assert!(
            plan.unsupported
                .iter()
                .any(|f| f.feature == "turbopack.debugIds")
        );
    }

    #[test]
    fn module_type_rules_and_defaults() {
        let mut rules = BTreeMap::new();
        rules.insert(
            String::from("*.glsl"),
            vec![TurbopackRule { type_: Some(String::from("raw")), ..TurbopackRule::default() }],
        );
        let cfg = NextConfig {
            turbopack: TurbopackConfig { rules, ..TurbopackConfig::default() },
            ..NextConfig::default()
        };
        let plan = translate_turbopack(&cfg);
        assert_eq!(plan.loaders.get(".glsl"), Some(&BunLoader::Text));
        assert_eq!(plan.plugins, Vec::<BunPluginSpec>::new());
        assert_eq!(plan.extensions.len(), 6);
        assert_eq!(plan.extensions[0], ".tsx");
    }

    #[test]
    fn bunfig_and_plugin_sources() {
        let plan = translate_turbopack(&fixture_config());
        let yolo = YoloWebConfig {
            public_env_prefixes: vec![String::from("NEXT_PUBLIC_"), String::from("YOLO_PUBLIC_")],
            ..YoloWebConfig::default()
        };
        let bunfig = bunfig_toml(&plan, &yolo);
        assert!(bunfig.contains("[serve.static]\nplugins = [\"./yolo.bundler.ts\"]\nenv = \"NEXT_PUBLIC_*\"\nsourcemap = \"linked\"\n"));
        assert!(bunfig.contains("[install]\nexact = true\n"));
        assert!(bunfig.contains("[test]\nroot = \"./tests\"\n"));
        assert!(bunfig.contains("YOLO_PUBLIC_*"));

        let ts = bundler_plugins_ts(&plan);
        assert!(ts.contains("export const yoloBundlerPlugins: import(\"bun\").BunPlugin[] = ["));
        assert!(ts.contains("export default yoloBundlerPlugins;"));
        assert!(ts.contains("from \"@yolo-rs/next/turbopack\""));
        assert!(ts.contains("svgReactPlugin(new RegExp(\"\\\\.svg$\"))"));
        assert!(ts.contains("textPlugin(new RegExp(\"\\\\.md$\"))"));
        assert!(ts.contains("loaderOverridePlugin(new RegExp(\"\\\\.yaml$\"), \"yaml\")"));
        assert!(ts.contains("aliasPlugin({"));
        assert!(ts.contains("\"underscore\":\"lodash\""));
        assert!(ts.contains("// unsupported: sass-loader"));
    }
}
