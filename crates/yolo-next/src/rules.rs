// SPDX-License-Identifier: Apache-2.0
//! `redirects()`, `rewrites()`, `headers()` and proxy matchers: path-to-regexp sources ->
//! matchers, `Bun.serve` patterns, axum patterns.
//!
//! Implements the path-to-regexp v6 subset Next.js 16 accepts for custom routes:
//! static text, `:name` parameters (`[A-Za-z0-9_]`) with an optional inline regex
//! (`:id(\d+)`), the modifiers `*` (zero or more segments), `+` (one or more) and `?`
//! (optional), unnamed `(.*)` groups captured as numeric keys, and an insensitive
//! trailing slash. Static text is compared case-insensitively (`sensitive: false`).
//!
//! Inline regexes and `has`/`missing` values are evaluated with a small backtracking
//! regex engine ([`mini_re`]) so the crate stays dependency-free.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::types::{Diagnostic, NextConfig, RouteCondition, RouteRule};

/// Modifier of a named parameter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ParamModifier {
    /// `:slug`
    One,
    /// `:path*`
    ZeroOrMore,
    /// `:path+`
    OneOrMore,
    /// `:slug?`
    Optional,
}

/// One compiled segment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum PatternSegment {
    /// Literal text.
    Static {
        /// Text.
        text: String,
    },
    /// Named parameter.
    Param {
        /// Name.
        name: String,
        /// Modifier.
        modifier: ParamModifier,
        /// Inline regex (`:id(\d+)`).
        regex: Option<String>,
    },
    /// Unnamed `(.*)` group.
    Wildcard,
}

/// Compiled path-to-regexp source.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct CompiledPattern {
    /// Original source.
    pub source: String,
    /// Segments.
    pub segments: Vec<PatternSegment>,
}

/// Normalized matching piece: the `/` preceding an optional or repeated parameter is split
/// off the static text so it can be absorbed by the optional group (`(?:/(?<p>...))?`).
enum Piece<'a> {
    Static(&'a str),
    Param {
        name: &'a str,
        modifier: ParamModifier,
        regex: Option<&'a str>,
        slash: bool,
    },
    Wildcard {
        index: usize,
    },
}

fn pieces(segments: &[PatternSegment]) -> Vec<Piece<'_>> {
    let mut out: Vec<Piece<'_>> = Vec::new();
    let mut wildcard_index = 0;
    for segment in segments {
        match segment {
            PatternSegment::Static { text } => out.push(Piece::Static(text)),
            PatternSegment::Param { name, modifier, regex } => {
                let mut slash = false;
                if *modifier != ParamModifier::One
                    && let Some(Piece::Static(text)) = out.last()
                    && let Some(stripped) = text.strip_suffix('/')
                {
                    slash = true;
                    if stripped.is_empty() {
                        out.pop();
                    } else {
                        let len = out.len();
                        out[len - 1] = Piece::Static(stripped);
                    }
                }
                out.push(Piece::Param {
                    name,
                    modifier: *modifier,
                    regex: regex.as_deref(),
                    slash,
                });
            }
            PatternSegment::Wildcard => {
                out.push(Piece::Wildcard { index: wildcard_index });
                wildcard_index += 1;
            }
        }
    }
    out
}

/// A repeated parameter value is a `/`-separated list of non-empty segments, each matching the
/// inline regex when present.
fn valid_repeated(value: &[char], regex: Option<&str>) -> bool {
    if value.is_empty() {
        return false;
    }
    let text: String = value.iter().collect();
    text.split('/').all(|segment| {
        !segment.is_empty() && regex.is_none_or(|re| mini_re::full_match(re, segment))
    })
}

fn segment_end(path: &[char], start: usize) -> usize {
    path[start..]
        .iter()
        .position(|c| *c == '/')
        .map_or(path.len(), |offset| start + offset)
}

fn match_pieces(
    pieces: &[Piece<'_>],
    path: &[char],
    pos: usize,
    caps: &mut Vec<(String, String)>,
) -> bool {
    let Some((first, rest)) = pieces.split_first() else {
        return pos == path.len() || (pos + 1 == path.len() && path[pos] == '/');
    };
    match first {
        Piece::Static(text) => {
            let wanted: Vec<char> = text.chars().collect();
            if path.len() < pos + wanted.len() {
                return false;
            }
            let same = path[pos..pos + wanted.len()]
                .iter()
                .zip(&wanted)
                .all(|(a, b)| a.eq_ignore_ascii_case(b));
            same && match_pieces(rest, path, pos + wanted.len(), caps)
        }
        Piece::Param { name, modifier, regex, slash } => {
            let param = ParamPiece { name, modifier: *modifier, regex: *regex, slash: *slash };
            match_param(rest, path, pos, caps, &param)
        }
        Piece::Wildcard { index } => {
            let key = index.to_string();
            for end in (pos..=path.len()).rev() {
                caps.push((key.clone(), path[pos..end].iter().collect()));
                if match_pieces(rest, path, end, caps) {
                    return true;
                }
                caps.pop();
            }
            false
        }
    }
}

struct ParamPiece<'a> {
    name: &'a str,
    modifier: ParamModifier,
    regex: Option<&'a str>,
    slash: bool,
}

fn match_param(
    rest: &[Piece<'_>],
    path: &[char],
    pos: usize,
    caps: &mut Vec<(String, String)>,
    param: &ParamPiece<'_>,
) -> bool {
    let optional = matches!(
        param.modifier,
        ParamModifier::ZeroOrMore | ParamModifier::Optional
    );
    let start = if param.slash {
        (path.get(pos) == Some(&'/')).then_some(pos + 1)
    } else {
        Some(pos)
    };
    if let Some(start) = start {
        let candidates: Vec<usize> = match param.modifier {
            ParamModifier::One | ParamModifier::Optional => match param.regex {
                Some(re) => {
                    let mut ends = mini_re::ends(re, &path[start..]);
                    ends.retain(|end| *end > 0);
                    ends.iter().rev().map(|end| start + end).collect()
                }
                None => (start + 1..=segment_end(path, start)).collect(),
            },
            ParamModifier::ZeroOrMore | ParamModifier::OneOrMore => (start + 1..=path.len())
                .rev()
                .filter(|end| valid_repeated(&path[start..*end], param.regex))
                .collect(),
        };
        for end in candidates {
            caps.push((param.name.to_owned(), path[start..end].iter().collect()));
            if match_pieces(rest, path, end, caps) {
                return true;
            }
            caps.pop();
        }
    }
    optional && match_pieces(rest, path, pos, caps)
}

fn regex_escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if matches!(
            c,
            '.' | '*' | '+' | '?' | '^' | '$' | '(' | ')' | '[' | ']' | '{' | '}' | '|' | '\\'
        ) {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

impl CompiledPattern {
    /// Matches a pathname (query string and hash ignored), returning captured params.
    /// Catch-all values keep their `/` separators (`a/b`); unnamed groups use keys `0`, `1`, ...
    pub fn matches(&self, path: &str) -> Option<BTreeMap<String, String>> {
        let pathname = path.split(['?', '#']).next().unwrap_or(path);
        let chars: Vec<char> = pathname.chars().collect();
        let pieces = pieces(&self.segments);
        let mut caps = Vec::new();
        if match_pieces(&pieces, &chars, 0, &mut caps) {
            Some(caps.into_iter().collect())
        } else {
            None
        }
    }

    /// `Bun.serve` routes key (`/blog/:slug`, `/docs/*`).
    ///
    /// `:slug?` has no Bun equivalent and is emitted as `:slug` (the optional form needs a second
    /// route); a non-terminal catch-all degrades to `:name`; inline regexes are dropped.
    pub fn bun_pattern(&self) -> String {
        let mut out = String::new();
        let last_index = self.segments.len().saturating_sub(1);
        for (index, segment) in self.segments.iter().enumerate() {
            let last = index == last_index;
            match segment {
                PatternSegment::Static { text } => out.push_str(text),
                PatternSegment::Param { name, modifier, .. } => match modifier {
                    ParamModifier::ZeroOrMore | ParamModifier::OneOrMore if last => {
                        out.push('*');
                    }
                    _ => {
                        out.push(':');
                        out.push_str(name);
                    }
                },
                PatternSegment::Wildcard if last => out.push('*'),
                PatternSegment::Wildcard => out.push_str(":rest"),
            }
        }
        out
    }

    /// axum 0.8 route (`/blog/{slug}`, `/docs/{*rest}`).
    pub fn axum_pattern(&self) -> String {
        let mut out = String::new();
        let last_index = self.segments.len().saturating_sub(1);
        let mut wildcards = 0;
        for (index, segment) in self.segments.iter().enumerate() {
            let last = index == last_index;
            match segment {
                PatternSegment::Static { text } => out.push_str(text),
                PatternSegment::Param { name, modifier, .. } => match modifier {
                    ParamModifier::ZeroOrMore | ParamModifier::OneOrMore if last => {
                        _ = write!(out, "{{*{name}}}");
                    }
                    _ => _ = write!(out, "{{{name}}}"),
                },
                PatternSegment::Wildcard => {
                    let name = if wildcards == 0 {
                        String::from("rest")
                    } else {
                        format!("rest{wildcards}")
                    };
                    wildcards += 1;
                    if last {
                        _ = write!(out, "{{*{name}}}");
                    } else {
                        _ = write!(out, "{{{name}}}");
                    }
                }
            }
        }
        out
    }

    /// Equivalent JavaScript regex source with named groups (`^/old/(?<slug>[^/]+)/?$`).
    pub fn to_regex_source(&self) -> String {
        let mut out = String::from("^");
        for piece in pieces(&self.segments) {
            match piece {
                Piece::Static(text) => out.push_str(&regex_escape(text)),
                Piece::Param { name, modifier, regex, slash } => {
                    let prefix = if slash { "/" } else { "" };
                    let group = match modifier {
                        ParamModifier::One => {
                            format!("(?<{name}>{})", regex.unwrap_or("[^/]+"))
                        }
                        ParamModifier::Optional => {
                            format!("(?:{prefix}(?<{name}>{}))?", regex.unwrap_or("[^/]+"))
                        }
                        ParamModifier::ZeroOrMore => match regex {
                            Some(re) => format!("(?:{prefix}(?<{name}>(?:{re})(?:/(?:{re}))*))?"),
                            None => format!("(?:{prefix}(?<{name}>.*))?"),
                        },
                        ParamModifier::OneOrMore => match regex {
                            Some(re) => format!("{prefix}(?<{name}>(?:{re})(?:/(?:{re}))*)"),
                            None => format!("{prefix}(?<{name}>.+)"),
                        },
                    };
                    out.push_str(&group);
                }
                Piece::Wildcard { .. } => out.push_str("(.*)"),
            }
        }
        out.push_str("/?$");
        out
    }
}

/// Reads a parenthesised group starting at `open` (index of `(`); returns the inner text and the
/// index after the closing `)`.
fn read_group(chars: &[char], open: usize, source: &str) -> Result<(String, usize), Diagnostic> {
    let mut depth = 0usize;
    let mut inner = String::new();
    let mut i = open;
    while i < chars.len() {
        let c = chars[i];
        if c == '\\' && i + 1 < chars.len() {
            if depth > 0 {
                inner.push(c);
                inner.push(chars[i + 1]);
            }
            i += 2;
            continue;
        }
        if c == '(' {
            depth += 1;
            if depth > 1 {
                inner.push(c);
            }
        } else if c == ')' {
            depth -= 1;
            if depth == 0 {
                return Ok((inner, i + 1));
            }
            inner.push(c);
        } else {
            inner.push(c);
        }
        i += 1;
    }
    Err(Diagnostic::error(
        "rules-source-invalid",
        format!("source `{source}`: unterminated group at offset {open}"),
    ))
}

fn flush_static(text: &mut String, segments: &mut Vec<PatternSegment>) {
    if !text.is_empty() {
        segments.push(PatternSegment::Static { text: std::mem::take(text) });
    }
}

/// Compiles a Next.js `source` (`/old/:slug*`, `/:path(\d+)`, `/(.*)`).
pub fn compile_source(source: &str) -> Result<CompiledPattern, Diagnostic> {
    if !source.starts_with('/') {
        return Err(Diagnostic::error(
            "rules-source-invalid",
            format!("source `{source}` must start with `/`"),
        ));
    }
    let chars: Vec<char> = source.chars().collect();
    let mut segments = Vec::new();
    let mut text = String::new();
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '\\' if i + 1 < chars.len() => {
                text.push(chars[i + 1]);
                i += 2;
            }
            ':' => {
                let start = i + 1;
                let mut j = start;
                while j < chars.len() && (chars[j].is_ascii_alphanumeric() || chars[j] == '_') {
                    j += 1;
                }
                if j == start {
                    return Err(Diagnostic::error(
                        "rules-source-invalid",
                        format!("source `{source}`: missing parameter name at offset {i}"),
                    ));
                }
                let name: String = chars[start..j].iter().collect();
                flush_static(&mut text, &mut segments);
                let mut regex = None;
                if chars.get(j) == Some(&'(') {
                    let (inner, end) = read_group(&chars, j, source)?;
                    regex = Some(inner);
                    j = end;
                }
                let modifier = match chars.get(j) {
                    Some('*') => ParamModifier::ZeroOrMore,
                    Some('+') => ParamModifier::OneOrMore,
                    Some('?') => ParamModifier::Optional,
                    _ => ParamModifier::One,
                };
                if modifier != ParamModifier::One {
                    j += 1;
                }
                segments.push(PatternSegment::Param { name, modifier, regex });
                i = j;
            }
            '(' => {
                flush_static(&mut text, &mut segments);
                let (_inner, mut end) = read_group(&chars, i, source)?;
                if matches!(chars.get(end), Some('*' | '+' | '?')) {
                    end += 1;
                }
                segments.push(PatternSegment::Wildcard);
                i = end;
            }
            '*' => {
                flush_static(&mut text, &mut segments);
                segments.push(PatternSegment::Wildcard);
                i += 1;
            }
            c => {
                text.push(c);
                i += 1;
            }
        }
    }
    flush_static(&mut text, &mut segments);
    Ok(CompiledPattern { source: source.to_owned(), segments })
}

/// Substitutes `:param` placeholders in a destination (`/blog/:slug`, `https://x/:path*`).
/// Modifiers after a token are consumed; unmatched tokens are left untouched.
pub fn apply_destination(destination: &str, params: &BTreeMap<String, String>) -> String {
    let chars: Vec<char> = destination.chars().collect();
    let mut out = String::with_capacity(destination.len());
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == ':' {
            let start = i + 1;
            let mut j = start;
            while j < chars.len() && (chars[j].is_ascii_alphanumeric() || chars[j] == '_') {
                j += 1;
            }
            if j > start {
                let name: String = chars[start..j].iter().collect();
                if let Some(value) = params.get(&name) {
                    out.push_str(value);
                    if matches!(chars.get(j), Some('*' | '+' | '?')) {
                        j += 1;
                    }
                    i = j;
                    continue;
                }
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

/// Status code of a redirect rule (`statusCode` > `permanent` 308 > 307).
pub fn redirect_status(rule: &RouteRule) -> u16 {
    if let Some(code) = rule.status_code {
        return code;
    }
    if rule.permanent == Some(true) {
        308
    } else {
        307
    }
}

/// A `has` value is treated as a regex when it uses regex syntax; otherwise it is compared
/// literally (case-insensitively for headers).
fn looks_like_regex(value: &str) -> bool {
    value.contains(['(', '^', '$', '|', '\\'])
}

fn value_matches(expected: &str, actual: &str, ignore_case: bool) -> bool {
    if looks_like_regex(expected) {
        return mini_re::full_match(expected, actual);
    }
    if ignore_case {
        expected.eq_ignore_ascii_case(actual)
    } else {
        expected == actual
    }
}

/// Evaluates one `has`/`missing` condition against request facts.
///
/// `header`, `cookie` and `query` require the key to be present and, when `value` is set, to be
/// equal (or to regex-match). `host` compares `value` with the request host.
pub fn condition_matches(
    cond: &RouteCondition,
    headers: &BTreeMap<String, String>,
    cookies: &BTreeMap<String, String>,
    query: &BTreeMap<String, String>,
    host: &str,
) -> bool {
    match cond.kind.as_str() {
        "header" => headers
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(&cond.key))
            .is_some_and(|(_, actual)| {
                cond.value
                    .as_deref()
                    .is_none_or(|expected| value_matches(expected, actual, true))
            }),
        "cookie" => cookies.get(&cond.key).is_some_and(|actual| {
            cond.value
                .as_deref()
                .is_none_or(|expected| value_matches(expected, actual, false))
        }),
        "query" => query.get(&cond.key).is_some_and(|actual| {
            cond.value
                .as_deref()
                .is_none_or(|expected| value_matches(expected, actual, false))
        }),
        "host" => cond
            .value
            .as_deref()
            .is_none_or(|expected| value_matches(expected, host, true)),
        _ => false,
    }
}

fn regex_for(source: &str) -> Value {
    compile_source(source).map_or(Value::Null, |p| Value::String(p.to_regex_source()))
}

fn rule_json(rule: &RouteRule, redirect: bool) -> Value {
    let mut object = json!({
        "source": rule.source,
        "destination": rule.destination,
        "regex": regex_for(&rule.source),
        "has": rule.has,
        "missing": rule.missing,
    });
    if redirect {
        object["statusCode"] = json!(redirect_status(rule));
    }
    if let Some(base_path) = rule.base_path {
        object["basePath"] = json!(base_path);
    }
    if let Some(locale) = rule.locale {
        object["locale"] = json!(locale);
    }
    object
}

/// `routes-manifest.json`-shaped JSON for redirects, rewrites and headers.
pub fn rules_manifest_json(cfg: &NextConfig) -> Value {
    let redirects: Vec<Value> = cfg.redirects.iter().map(|r| rule_json(r, true)).collect();
    let rewrites =
        |rules: &[RouteRule]| -> Vec<Value> { rules.iter().map(|r| rule_json(r, false)).collect() };
    let headers: Vec<Value> = cfg
        .headers
        .iter()
        .map(|rule| {
            let header_list: Vec<Value> = rule
                .headers
                .iter()
                .map(|h| json!({ "key": h.key, "value": h.value }))
                .collect();
            let mut object = json!({
                "source": rule.source,
                "headers": header_list,
                "regex": regex_for(&rule.source),
                "has": rule.has,
                "missing": rule.missing,
            });
            if let Some(base_path) = rule.base_path {
                object["basePath"] = json!(base_path);
            }
            if let Some(locale) = rule.locale {
                object["locale"] = json!(locale);
            }
            object
        })
        .collect();
    json!({
        "redirects": redirects,
        "rewrites": {
            "beforeFiles": rewrites(&cfg.rewrites.before_files),
            "afterFiles": rewrites(&cfg.rewrites.after_files),
            "fallback": rewrites(&cfg.rewrites.fallback),
        },
        "headers": headers,
    })
}

/// Minimal backtracking regex engine for inline parameter regexes and `has` values.
///
/// Supported: literals, `.`, `\d \w \s` (and negations), `[...]` classes with ranges and
/// negation, groups `( )` / `(?: )` / `(?<name> )`, alternation `|`, quantifiers
/// `* + ? {n} {n,} {n,m}` (greedy or lazy), anchors `^` `$`. Unsupported syntax makes
/// [`full_match`](mini_re::full_match) fall back to a literal comparison.
pub mod mini_re {
    use std::collections::BTreeSet;

    #[derive(Debug, Clone)]
    enum ClassItem {
        Range(char, char),
        Digit(bool),
        Word(bool),
        Space(bool),
    }

    #[derive(Debug, Clone)]
    enum Node {
        Char(char),
        Any,
        Class {
            negated: bool,
            items: Vec<ClassItem>,
        },
        Group(Vec<Vec<Node>>),
        Repeat {
            node: Box<Node>,
            min: usize,
            max: Option<usize>,
        },
        Start,
        End,
    }

    struct Parser<'a> {
        chars: &'a [char],
        pos: usize,
    }

    impl Parser<'_> {
        fn peek(&self) -> Option<char> {
            self.chars.get(self.pos).copied()
        }

        fn bump(&mut self) -> Option<char> {
            let c = self.peek();
            self.pos += 1;
            c
        }

        fn parse_alternation(&mut self) -> Option<Vec<Vec<Node>>> {
            let mut alternatives = vec![self.parse_sequence()?];
            while self.peek() == Some('|') {
                self.pos += 1;
                alternatives.push(self.parse_sequence()?);
            }
            Some(alternatives)
        }

        fn parse_sequence(&mut self) -> Option<Vec<Node>> {
            let mut nodes = Vec::new();
            while let Some(c) = self.peek() {
                if c == '|' || c == ')' {
                    break;
                }
                let atom = self.parse_atom()?;
                nodes.push(self.parse_quantifier(atom));
            }
            Some(nodes)
        }

        fn parse_braces(&mut self) -> Option<(usize, Option<usize>)> {
            let save = self.pos;
            self.pos += 1;
            let mut text = String::new();
            while let Some(c) = self.peek() {
                if c == '}' {
                    break;
                }
                text.push(c);
                self.pos += 1;
            }
            if self.peek() != Some('}') {
                self.pos = save;
                return None;
            }
            self.pos += 1;
            let parsed = match text.split_once(',') {
                Some((lo, "")) => lo.trim().parse().ok().map(|lo| (lo, None)),
                Some((lo, hi)) => lo
                    .trim()
                    .parse()
                    .ok()
                    .zip(hi.trim().parse().ok())
                    .map(|(lo, hi)| (lo, Some(hi))),
                None => text.trim().parse().ok().map(|n| (n, Some(n))),
            };
            if parsed.is_none() {
                self.pos = save;
            }
            parsed
        }

        fn parse_quantifier(&mut self, atom: Node) -> Node {
            let (min, max) = match self.peek() {
                Some('*') => {
                    self.pos += 1;
                    (0, None)
                }
                Some('+') => {
                    self.pos += 1;
                    (1, None)
                }
                Some('?') => {
                    self.pos += 1;
                    (0, Some(1))
                }
                Some('{') => match self.parse_braces() {
                    Some(bounds) => bounds,
                    None => return atom,
                },
                _ => return atom,
            };
            if self.peek() == Some('?') {
                // Lazy quantifier: the end-position set is identical for a full match.
                self.pos += 1;
            }
            Node::Repeat { node: Box::new(atom), min, max }
        }

        fn parse_group(&mut self) -> Option<Node> {
            if self.peek() == Some('?') {
                self.pos += 1;
                match self.bump()? {
                    ':' => {}
                    '<' => {
                        if matches!(self.peek(), Some('=' | '!')) {
                            return None;
                        }
                        while let Some(c) = self.bump() {
                            if c == '>' {
                                break;
                            }
                        }
                    }
                    _ => return None,
                }
            }
            let alternatives = self.parse_alternation()?;
            if self.bump() != Some(')') {
                return None;
            }
            Some(Node::Group(alternatives))
        }

        fn parse_atom(&mut self) -> Option<Node> {
            match self.bump()? {
                '.' => Some(Node::Any),
                '^' => Some(Node::Start),
                '$' => Some(Node::End),
                '(' => self.parse_group(),
                '[' => self.parse_class(),
                '\\' => Some(Self::escape(self.bump()?)),
                '*' | '+' | '?' => None,
                c => Some(Node::Char(c)),
            }
        }

        fn escape(c: char) -> Node {
            let class = |items: Vec<ClassItem>| Node::Class { negated: false, items };
            match c {
                'd' => class(vec![ClassItem::Digit(false)]),
                'D' => class(vec![ClassItem::Digit(true)]),
                'w' => class(vec![ClassItem::Word(false)]),
                'W' => class(vec![ClassItem::Word(true)]),
                's' => class(vec![ClassItem::Space(false)]),
                'S' => class(vec![ClassItem::Space(true)]),
                'n' => Node::Char('\n'),
                't' => Node::Char('\t'),
                'r' => Node::Char('\r'),
                other => Node::Char(other),
            }
        }

        fn parse_class(&mut self) -> Option<Node> {
            let negated = self.peek() == Some('^');
            if negated {
                self.pos += 1;
            }
            let mut items = Vec::new();
            let mut first = true;
            loop {
                let c = self.bump()?;
                if c == ']' && !first {
                    break;
                }
                first = false;
                let item = if c == '\\' {
                    match self.bump()? {
                        'd' => ClassItem::Digit(false),
                        'D' => ClassItem::Digit(true),
                        'w' => ClassItem::Word(false),
                        'W' => ClassItem::Word(true),
                        's' => ClassItem::Space(false),
                        'S' => ClassItem::Space(true),
                        'n' => ClassItem::Range('\n', '\n'),
                        't' => ClassItem::Range('\t', '\t'),
                        other => ClassItem::Range(other, other),
                    }
                } else if self.peek() == Some('-') && self.chars.get(self.pos + 1) != Some(&']') {
                    self.pos += 1;
                    let hi = self.bump()?;
                    ClassItem::Range(c, hi)
                } else {
                    ClassItem::Range(c, c)
                };
                items.push(item);
            }
            Some(Node::Class { negated, items })
        }
    }

    fn parse(pattern: &str) -> Option<Vec<Vec<Node>>> {
        let chars: Vec<char> = pattern.chars().collect();
        let mut parser = Parser { chars: &chars, pos: 0 };
        let alternatives = parser.parse_alternation()?;
        (parser.pos == chars.len()).then_some(alternatives)
    }

    fn class_matches(items: &[ClassItem], negated: bool, c: char) -> bool {
        let hit = items.iter().any(|item| match item {
            ClassItem::Range(lo, hi) => (*lo..=*hi).contains(&c),
            ClassItem::Digit(neg) => c.is_ascii_digit() != *neg,
            ClassItem::Word(neg) => (c.is_ascii_alphanumeric() || c == '_') != *neg,
            ClassItem::Space(neg) => c.is_whitespace() != *neg,
        });
        hit != negated
    }

    fn repeat_ends(
        node: &Node,
        min: usize,
        max: Option<usize>,
        text: &[char],
        pos: usize,
    ) -> BTreeSet<usize> {
        let mut out = BTreeSet::new();
        let mut current: BTreeSet<usize> = BTreeSet::from([pos]);
        if min == 0 {
            out.insert(pos);
        }
        let mut seen = current.clone();
        let mut count = 0usize;
        while max.is_none_or(|max| count < max) {
            count += 1;
            let mut next = BTreeSet::new();
            for p in &current {
                next.extend(node_ends(node, text, *p));
            }
            if next.is_empty() {
                break;
            }
            if count >= min {
                out.extend(next.iter().copied());
            }
            let progressed = next.iter().any(|p| !seen.contains(p));
            seen.extend(next.iter().copied());
            if !progressed && count >= min {
                break;
            }
            current = next;
        }
        out
    }

    fn node_ends(node: &Node, text: &[char], pos: usize) -> BTreeSet<usize> {
        let mut out = BTreeSet::new();
        match node {
            Node::Char(c) => {
                if text.get(pos) == Some(c) {
                    out.insert(pos + 1);
                }
            }
            Node::Any => {
                if text.get(pos).is_some_and(|c| *c != '\n') {
                    out.insert(pos + 1);
                }
            }
            Node::Class { negated, items } => {
                if text
                    .get(pos)
                    .is_some_and(|c| class_matches(items, *negated, *c))
                {
                    out.insert(pos + 1);
                }
            }
            Node::Group(alternatives) => {
                for sequence in alternatives {
                    out.extend(sequence_ends(sequence, text, pos));
                }
            }
            Node::Repeat { node, min, max } => {
                out = repeat_ends(node, *min, *max, text, pos);
            }
            Node::Start => {
                if pos == 0 {
                    out.insert(pos);
                }
            }
            Node::End => {
                if pos == text.len() {
                    out.insert(pos);
                }
            }
        }
        out
    }

    fn sequence_ends(sequence: &[Node], text: &[char], pos: usize) -> BTreeSet<usize> {
        let mut current = BTreeSet::from([pos]);
        for node in sequence {
            let mut next = BTreeSet::new();
            for p in &current {
                next.extend(node_ends(node, text, *p));
            }
            if next.is_empty() {
                return next;
            }
            current = next;
        }
        current
    }

    /// End offsets (in chars) of every match of `pattern` anchored at the start of `text`,
    /// ascending. Empty when the pattern is unsupported.
    pub fn ends(pattern: &str, text: &[char]) -> Vec<usize> {
        let Some(alternatives) = parse(pattern) else {
            return Vec::new();
        };
        let mut out = BTreeSet::new();
        for sequence in &alternatives {
            out.extend(sequence_ends(sequence, text, 0));
        }
        out.into_iter().collect()
    }

    /// True when `pattern` matches the whole of `text` (`^pattern$`). Falls back to a literal
    /// comparison when the pattern uses unsupported syntax.
    pub fn full_match(pattern: &str, text: &str) -> bool {
        let chars: Vec<char> = text.chars().collect();
        if parse(pattern).is_none() {
            return pattern == text;
        }
        ends(pattern, &chars).contains(&chars.len())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::types::{HeaderKv, HeaderRule, Rewrites};

    fn params(source: &str, path: &str) -> Option<BTreeMap<String, String>> {
        compile_source(source).unwrap().matches(path)
    }

    fn get<'a>(params: &'a BTreeMap<String, String>, key: &str) -> Option<&'a str> {
        params.get(key).map(String::as_str)
    }

    #[test]
    fn named_param_matches_one_segment() {
        let p = params("/old/:slug", "/old/a").unwrap();
        assert_eq!(get(&p, "slug"), Some("a"));
        assert!(params("/old/:slug", "/old/a/b").is_none());
        assert!(params("/old/:slug", "/old/").is_none());
        assert!(
            params("/old/:slug", "/old/a/").is_some(),
            "trailing slash insensitive"
        );
        assert!(
            params("/old/:slug", "/OLD/a").is_some(),
            "static text is case-insensitive"
        );
        assert!(params("/old/:slug", "/old/a?x=1").is_some());
    }

    #[test]
    fn catch_all_captures_joined_remainder_and_matches_bare_prefix() {
        let p = params("/docs/:path*", "/docs/a/b").unwrap();
        assert_eq!(get(&p, "path"), Some("a/b"));
        let bare = params("/docs/:path*", "/docs").unwrap();
        assert!(!bare.contains_key("path"));
        assert!(params("/docs/:path*", "/docs/").is_some());
        assert!(params("/docs/:path+", "/docs").is_none());
        let plus = params("/docs/:path+", "/docs/x/y").unwrap();
        assert_eq!(get(&plus, "path"), Some("x/y"));
        assert!(params("/docs/:path*", "/docsx").is_none());
    }

    #[test]
    fn wildcard_matches_everything() {
        assert!(params("/(.*)", "/").is_some());
        let p = params("/(.*)", "/a/b/c").unwrap();
        assert_eq!(get(&p, "0"), Some("a/b/c"));
        assert!(params("/(.*)", "/_next/static/chunk.js").is_some());
    }

    #[test]
    fn inline_regex_is_enforced() {
        assert!(params("/:id(\\d+)", "/abc").is_none());
        let p = params("/:id(\\d+)", "/42").unwrap();
        assert_eq!(get(&p, "id"), Some("42"));
        assert!(params("/:lang(en|fr)/about", "/fr/about").is_some());
        assert!(params("/:lang(en|fr)/about", "/de/about").is_none());
    }

    #[test]
    fn optional_param() {
        assert!(params("/users/:id?", "/users").is_some());
        let p = params("/users/:id?", "/users/7").unwrap();
        assert_eq!(get(&p, "id"), Some("7"));
        assert!(params("/users/:id?", "/users/7/8").is_none());
    }

    #[test]
    fn bun_axum_and_regex_outputs() {
        let p = compile_source("/old/:slug").unwrap();
        assert_eq!(p.bun_pattern(), "/old/:slug");
        assert_eq!(p.axum_pattern(), "/old/{slug}");
        assert_eq!(p.to_regex_source(), "^/old/(?<slug>[^/]+)/?$");

        let docs = compile_source("/docs/:path*").unwrap();
        assert_eq!(docs.bun_pattern(), "/docs/*");
        assert_eq!(docs.axum_pattern(), "/docs/{*path}");
        assert_eq!(docs.to_regex_source(), "^/docs(?:/(?<path>.*))?/?$");

        let all = compile_source("/(.*)").unwrap();
        assert_eq!(all.bun_pattern(), "/*");
        assert_eq!(all.axum_pattern(), "/{*rest}");
        assert_eq!(all.to_regex_source(), "^/(.*)/?$");

        let opt = compile_source("/u/:slug?").unwrap();
        assert_eq!(opt.bun_pattern(), "/u/:slug");
        assert_eq!(opt.to_regex_source(), "^/u(?:/(?<slug>[^/]+))?/?$");

        let id = compile_source("/:id(\\d+)").unwrap();
        assert_eq!(id.to_regex_source(), "^/(?<id>\\d+)/?$");
        assert_eq!(id.bun_pattern(), "/:id");

        let dotted = compile_source("/file.txt/:name+").unwrap();
        assert_eq!(dotted.to_regex_source(), "^/file\\.txt/(?<name>.+)/?$");
    }

    #[test]
    fn invalid_sources_are_rejected() {
        assert!(compile_source("old/:slug").is_err());
        assert!(compile_source("/:").is_err());
        assert!(compile_source("/:id(\\d+").is_err());
    }

    #[test]
    fn destination_substitution() {
        let p = params("/legacy/:path*", "/legacy/a/b").unwrap();
        assert_eq!(apply_destination("https://x/:path*", &p), "https://x/a/b");
        let one = params("/old/:slug", "/old/hello").unwrap();
        assert_eq!(apply_destination("/blog/:slug", &one), "/blog/hello");
        assert_eq!(apply_destination("/blog/:other", &one), "/blog/:other");
        assert_eq!(
            apply_destination("/blog/:slug/:slug", &one),
            "/blog/hello/hello"
        );
    }

    #[test]
    fn redirect_statuses() {
        let mut rule = RouteRule {
            source: String::from("/a"),
            destination: String::from("/b"),
            ..RouteRule::default()
        };
        assert_eq!(redirect_status(&rule), 307);
        rule.permanent = Some(true);
        assert_eq!(redirect_status(&rule), 308);
        rule.status_code = Some(301);
        assert_eq!(redirect_status(&rule), 301);
    }

    #[test]
    fn conditions() {
        let headers: BTreeMap<String, String> =
            [(String::from("X-Legacy"), String::from("1"))].into();
        let cookies: BTreeMap<String, String> =
            [(String::from("session"), String::from("abc"))].into();
        let query: BTreeMap<String, String> = [(String::from("v"), String::from("42"))].into();
        let header = RouteCondition {
            kind: String::from("header"),
            key: String::from("x-legacy"),
            value: Some(String::from("1")),
        };
        assert!(condition_matches(
            &header,
            &headers,
            &cookies,
            &query,
            "example.com"
        ));
        let header_regex = RouteCondition { value: Some(String::from("(1|2)")), ..header.clone() };
        assert!(condition_matches(
            &header_regex,
            &headers,
            &cookies,
            &query,
            "example.com"
        ));
        let cookie_present = RouteCondition {
            kind: String::from("cookie"),
            key: String::from("session"),
            value: None,
        };
        assert!(condition_matches(
            &cookie_present,
            &headers,
            &cookies,
            &query,
            ""
        ));
        let query_regex = RouteCondition {
            kind: String::from("query"),
            key: String::from("v"),
            value: Some(String::from("^\\d+$")),
        };
        assert!(condition_matches(
            &query_regex,
            &headers,
            &cookies,
            &query,
            ""
        ));
        let host = RouteCondition {
            kind: String::from("host"),
            key: String::new(),
            value: Some(String::from("example.com")),
        };
        assert!(condition_matches(
            &host,
            &headers,
            &cookies,
            &query,
            "example.com"
        ));
        assert!(!condition_matches(
            &host,
            &headers,
            &cookies,
            &query,
            "other.com"
        ));
        let missing = RouteCondition {
            kind: String::from("header"),
            key: String::from("x-internal"),
            value: None,
        };
        assert!(!condition_matches(&missing, &headers, &cookies, &query, ""));
    }

    #[test]
    fn manifest_shape() {
        let cfg = NextConfig {
            redirects: vec![RouteRule {
                source: String::from("/old/:slug"),
                destination: String::from("/blog/:slug"),
                permanent: Some(true),
                ..RouteRule::default()
            }],
            rewrites: Rewrites {
                after_files: vec![RouteRule {
                    source: String::from("/u/:username"),
                    destination: String::from("/users/:username"),
                    ..RouteRule::default()
                }],
                ..Rewrites::default()
            },
            headers: vec![HeaderRule {
                source: String::from("/(.*)"),
                headers: vec![HeaderKv {
                    key: String::from("X-Frame-Options"),
                    value: String::from("DENY"),
                }],
                ..HeaderRule::default()
            }],
            ..NextConfig::default()
        };
        let manifest = rules_manifest_json(&cfg);
        assert_eq!(manifest["redirects"][0]["statusCode"], 308);
        assert_eq!(manifest["redirects"][0]["regex"], "^/old/(?<slug>[^/]+)/?$");
        assert_eq!(
            manifest["rewrites"]["afterFiles"][0]["source"],
            "/u/:username"
        );
        assert_eq!(
            manifest["rewrites"]["beforeFiles"]
                .as_array()
                .unwrap()
                .len(),
            0
        );
        assert_eq!(
            manifest["headers"][0]["headers"][0]["key"],
            "X-Frame-Options"
        );
        assert_eq!(manifest["headers"][0]["regex"], "^/(.*)/?$");
    }

    #[test]
    fn mini_regex_engine() {
        assert!(mini_re::full_match("\\d+", "123"));
        assert!(!mini_re::full_match("\\d+", "12a"));
        assert!(mini_re::full_match("[a-z]{2,3}", "abc"));
        assert!(!mini_re::full_match("[a-z]{2,3}", "abcd"));
        assert!(mini_re::full_match("(?:en|fr)-[A-Z]{2}", "en-US"));
        assert!(mini_re::full_match(".*", ""));
        assert!(mini_re::full_match("a.c", "abc"));
        assert!(mini_re::full_match("[^/]+", "abc"));
        assert!(!mini_re::full_match("[^/]+", "a/c"));
        assert!(mini_re::full_match("(?<year>\\d{4})", "2026"));
        assert!(mini_re::full_match("x*", ""));
        assert!(mini_re::full_match("(a|b)*c", "ababc"));
        assert!(!mini_re::full_match("(a|b)*c", "ababd"));
        assert!(mini_re::full_match("a+?b", "aab"));
    }
}
