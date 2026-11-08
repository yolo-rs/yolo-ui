// SPDX-License-Identifier: Apache-2.0
//! Metadata API: `metadata` / `viewport` objects, merge semantics and head renderers.
//!
//! Mirrors `packages/next/src/lib/metadata/{resolve-metadata,generate/*}.ts` of Next.js 16.4.5:
//! shallow merge root -> leaf, `title.template` applied to child titles, `metadataBase`
//! resolution of relative URLs, and the tag order of `generate/basic.tsx`, `alternate.tsx`,
//! `icons.tsx`, `opengraph.tsx`, `meta.tsx`.

use std::collections::BTreeMap;

use serde_json::Value;

use crate::types::{Metadata, ProjectAnalysis, ThemeColor, TitleSpec, Viewport};

// ============================================================================
// Value helpers
// ============================================================================

fn str_of(value: &Value) -> Option<String> {
    match value {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

fn field<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    value.get(key).filter(|v| !v.is_null())
}

fn field_str(value: &Value, key: &str) -> Option<String> {
    field(value, key).and_then(str_of)
}

/// String, or `url` / `href` field of an object.
fn url_of(value: &Value) -> Option<String> {
    str_of(value)
        .or_else(|| field_str(value, "url"))
        .or_else(|| field_str(value, "href"))
}

/// Array items, or the value itself as a single item.
fn as_list(value: &Value) -> Vec<&Value> {
    match value {
        Value::Array(items) => items.iter().filter(|v| !v.is_null()).collect(),
        Value::Null => Vec::new(),
        other => vec![other],
    }
}

fn str_list(value: &Value) -> Vec<String> {
    as_list(value).into_iter().filter_map(str_of).collect()
}

/// Formats a number without a trailing `.0`.
fn fmt_num(value: f64) -> String {
    if value.fract() == 0.0 && value.abs() < 1.0e15 {
        format!("{}", value as i64)
    } else {
        value.to_string()
    }
}

fn is_absolute_url(url: &str) -> bool {
    url.starts_with("//")
        || url.starts_with("data:")
        || url.starts_with("mailto:")
        || url.starts_with("tel:")
        || url.split_once(':').is_some_and(|(scheme, rest)| {
            !scheme.is_empty()
                && scheme
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
                && rest.starts_with("//")
        })
}

/// Resolves `url` against `metadataBase` when present.
fn resolve_url(base: Option<&str>, url: &str) -> String {
    if is_absolute_url(url) {
        return url.to_owned();
    }
    match base {
        Some(base) if !base.is_empty() => {
            format!(
                "{}/{}",
                base.trim_end_matches('/'),
                url.trim_start_matches('/')
            )
        }
        _ => url.to_owned(),
    }
}

// ============================================================================
// Parsing
// ============================================================================

fn title_from_value(value: &Value) -> Option<TitleSpec> {
    match value {
        Value::String(s) => {
            Some(TitleSpec { default: Some(s.clone()), template: None, absolute: None })
        }
        Value::Object(_) => Some(TitleSpec {
            default: field_str(value, "default"),
            template: field_str(value, "template"),
            absolute: field_str(value, "absolute"),
        }),
        Value::Number(_) => {
            Some(TitleSpec { default: str_of(value), template: None, absolute: None })
        }
        _ => None,
    }
}

/// Converts a statically evaluated `metadata` object into [`Metadata`].
pub fn metadata_from_value(value: &Value) -> Metadata {
    let mut metadata = Metadata::default();
    let Value::Object(map) = value else {
        return metadata;
    };
    for (key, v) in map {
        if v.is_null() {
            continue;
        }
        match key.as_str() {
            "title" => metadata.title = title_from_value(v),
            "description" => metadata.description = str_of(v),
            "metadataBase" => metadata.metadata_base = url_of(v),
            "applicationName" => metadata.application_name = str_of(v),
            "authors" => {
                metadata.authors = as_list(v).into_iter().cloned().collect();
            }
            "generator" => metadata.generator = str_of(v),
            "keywords" => metadata.keywords = str_list(v),
            "referrer" => metadata.referrer = str_of(v),
            "creator" => metadata.creator = str_of(v),
            "publisher" => metadata.publisher = str_of(v),
            "robots" => metadata.robots = Some(v.clone()),
            "alternates" => metadata.alternates = Some(v.clone()),
            "icons" => metadata.icons = Some(v.clone()),
            "manifest" => metadata.manifest = url_of(v),
            "openGraph" => metadata.open_graph = Some(v.clone()),
            "twitter" => metadata.twitter = Some(v.clone()),
            "verification" => metadata.verification = Some(v.clone()),
            "appleWebApp" => metadata.apple_web_app = Some(v.clone()),
            "formatDetection" => metadata.format_detection = Some(v.clone()),
            "category" => metadata.category = str_of(v),
            "themeColor" => metadata.theme_color_legacy = Some(v.clone()),
            "viewport" => metadata.viewport_legacy = Some(v.clone()),
            "colorScheme" => metadata.color_scheme_legacy = str_of(v),
            "other" => {
                if let Value::Object(other) = v {
                    for (k, val) in other {
                        metadata.other.insert(k.clone(), val.clone());
                    }
                }
            }
            _ => {
                metadata.other.insert(key.clone(), v.clone());
            }
        }
    }
    metadata
}

fn theme_colors_from_value(value: &Value) -> Vec<ThemeColor> {
    as_list(value)
        .into_iter()
        .filter_map(|item| match item {
            Value::String(color) => Some(ThemeColor { media: None, color: color.clone() }),
            Value::Object(_) => field_str(item, "color")
                .map(|color| ThemeColor { media: field_str(item, "media"), color }),
            _ => None,
        })
        .collect()
}

/// Converts a statically evaluated `viewport` object into [`Viewport`].
pub fn viewport_from_value(value: &Value) -> Viewport {
    let mut viewport = Viewport::default();
    let Value::Object(map) = value else {
        return viewport;
    };
    for (key, v) in map {
        if v.is_null() {
            continue;
        }
        match key.as_str() {
            "width" => viewport.width = str_of(v),
            "height" => viewport.height = str_of(v),
            "initialScale" => viewport.initial_scale = v.as_f64(),
            "minimumScale" => viewport.minimum_scale = v.as_f64(),
            "maximumScale" => viewport.maximum_scale = v.as_f64(),
            "userScalable" => viewport.user_scalable = v.as_bool(),
            "viewportFit" => viewport.viewport_fit = str_of(v),
            "interactiveWidget" => viewport.interactive_widget = str_of(v),
            "themeColor" => viewport.theme_color = theme_colors_from_value(v),
            "colorScheme" => viewport.color_scheme = str_of(v),
            _ => {}
        }
    }
    viewport
}

/// Default viewport (`width=device-width, initial-scale=1`).
pub fn default_viewport() -> Viewport {
    Viewport {
        width: Some(String::from("device-width")),
        initial_scale: Some(1.0),
        ..Viewport::default()
    }
}

// ============================================================================
// Merge and title resolution
// ============================================================================

#[allow(clippy::ref_option)]
fn pick<T: Clone>(parent: &Option<T>, child: &Option<T>) -> Option<T> {
    child.clone().or_else(|| parent.clone())
}

fn pick_vec<T: Clone>(parent: &[T], child: &[T]) -> Vec<T> {
    if child.is_empty() {
        parent.to_vec()
    } else {
        child.to_vec()
    }
}

/// Shallow merge root -> leaf (child wins, `title.template` applies to child titles).
///
/// The merged `title.template` stays the parent's when the child defines none, so the
/// template keeps applying to the child's own descendants.
pub fn merge_metadata(parent: &Metadata, child: &Metadata) -> Metadata {
    let title = match (&parent.title, &child.title) {
        (_, None) => parent.title.clone(),
        (None, Some(c)) => Some(c.clone()),
        (Some(p), Some(c)) => Some(TitleSpec {
            default: c.default.clone().or_else(|| c.absolute.clone()),
            template: c.template.clone().or_else(|| p.template.clone()),
            absolute: c.absolute.clone(),
        }),
    };
    let mut other = parent.other.clone();
    for (k, v) in &child.other {
        other.insert(k.clone(), v.clone());
    }
    Metadata {
        title,
        description: pick(&parent.description, &child.description),
        metadata_base: pick(&parent.metadata_base, &child.metadata_base),
        application_name: pick(&parent.application_name, &child.application_name),
        authors: pick_vec(&parent.authors, &child.authors),
        generator: pick(&parent.generator, &child.generator),
        keywords: pick_vec(&parent.keywords, &child.keywords),
        referrer: pick(&parent.referrer, &child.referrer),
        creator: pick(&parent.creator, &child.creator),
        publisher: pick(&parent.publisher, &child.publisher),
        robots: pick(&parent.robots, &child.robots),
        alternates: pick(&parent.alternates, &child.alternates),
        icons: pick(&parent.icons, &child.icons),
        manifest: pick(&parent.manifest, &child.manifest),
        open_graph: pick(&parent.open_graph, &child.open_graph),
        twitter: pick(&parent.twitter, &child.twitter),
        verification: pick(&parent.verification, &child.verification),
        apple_web_app: pick(&parent.apple_web_app, &child.apple_web_app),
        format_detection: pick(&parent.format_detection, &child.format_detection),
        category: pick(&parent.category, &child.category),
        theme_color_legacy: pick(&parent.theme_color_legacy, &child.theme_color_legacy),
        viewport_legacy: pick(&parent.viewport_legacy, &child.viewport_legacy),
        color_scheme_legacy: pick(&parent.color_scheme_legacy, &child.color_scheme_legacy),
        other,
    }
}

/// Resolves the final `<title>` text.
///
/// `child.title.absolute` wins; otherwise the child's default goes through the parent's
/// `%s` template; otherwise the child default; otherwise the parent's own title.
pub fn resolve_title(parent: Option<&Metadata>, child: &Metadata) -> Option<String> {
    let parent_title = parent.and_then(|p| p.title.as_ref());
    if let Some(title) = &child.title {
        if let Some(absolute) = &title.absolute {
            return Some(absolute.clone());
        }
        if let Some(default) = &title.default {
            return Some(match parent_title.and_then(|t| t.template.as_deref()) {
                Some(template) if template.contains("%s") => template.replace("%s", default),
                _ => default.clone(),
            });
        }
    }
    parent_title.and_then(|t| t.absolute.clone().or_else(|| t.default.clone()))
}

// ============================================================================
// Head tag model
// ============================================================================

/// One `<head>` element.
#[derive(Debug, Clone, PartialEq, Eq)]
enum HeadTag {
    Title(String),
    Meta(Vec<(&'static str, String)>),
    Link(Vec<(&'static str, String)>),
}

struct Head {
    base: Option<String>,
    tags: Vec<HeadTag>,
}

impl Head {
    fn meta(&mut self, name: &str, content: impl Into<String>) {
        self.tags.push(HeadTag::Meta(vec![
            ("name", name.to_owned()),
            ("content", content.into()),
        ]));
    }

    fn property(&mut self, property: &str, content: impl Into<String>) {
        self.tags.push(HeadTag::Meta(vec![
            ("property", property.to_owned()),
            ("content", content.into()),
        ]));
    }

    fn link(&mut self, attrs: Vec<(&'static str, String)>) {
        self.tags.push(HeadTag::Link(attrs));
    }

    fn resolve(&self, url: &str) -> String {
        resolve_url(self.base.as_deref(), url)
    }
}

/// `width=device-width, initial-scale=1, ...` content string.
fn viewport_content(viewport: &Viewport) -> String {
    let mut parts = Vec::new();
    if let Some(width) = &viewport.width {
        parts.push(format!("width={width}"));
    }
    if let Some(height) = &viewport.height {
        parts.push(format!("height={height}"));
    }
    if let Some(scale) = viewport.initial_scale {
        parts.push(format!("initial-scale={}", fmt_num(scale)));
    }
    if let Some(scale) = viewport.minimum_scale {
        parts.push(format!("minimum-scale={}", fmt_num(scale)));
    }
    if let Some(scale) = viewport.maximum_scale {
        parts.push(format!("maximum-scale={}", fmt_num(scale)));
    }
    if let Some(scalable) = viewport.user_scalable {
        parts.push(format!(
            "user-scalable={}",
            if scalable { "yes" } else { "no" }
        ));
    }
    if let Some(fit) = &viewport.viewport_fit {
        parts.push(format!("viewport-fit={fit}"));
    }
    if let Some(widget) = &viewport.interactive_widget {
        parts.push(format!("interactive-widget={widget}"));
    }
    parts.join(", ")
}

/// `robots` string or object -> `index, follow, max-snippet:-1 ...`.
fn robots_content(value: &Value) -> Option<String> {
    if let Some(s) = str_of(value) {
        return Some(s);
    }
    let Value::Object(map) = value else {
        return None;
    };
    let mut parts = Vec::new();
    if let Some(index) = map.get("index").and_then(Value::as_bool) {
        parts.push(if index { "index" } else { "noindex" }.to_owned());
    }
    if let Some(follow) = map.get("follow").and_then(Value::as_bool) {
        parts.push(if follow { "follow" } else { "nofollow" }.to_owned());
    }
    for (key, v) in map {
        if matches!(key.as_str(), "index" | "follow" | "googleBot") {
            continue;
        }
        match v {
            Value::Bool(true) => parts.push(key.clone()),
            Value::Bool(false) | Value::Null => {}
            other => {
                if let Some(s) = str_of(other) {
                    parts.push(format!("{key}:{s}"));
                }
            }
        }
    }
    Some(parts.join(", "))
}

fn push_basic(head: &mut Head, m: &Metadata, viewport: Option<&Viewport>) {
    let default_viewport = default_viewport();
    let viewport = viewport.unwrap_or(&default_viewport);
    head.tags
        .push(HeadTag::Meta(vec![("charset", String::from("utf-8"))]));
    let viewport_text = match (&m.viewport_legacy, viewport_content(viewport)) {
        (Some(legacy), content) if content.is_empty() => str_of(legacy).unwrap_or_default(),
        (_, content) => content,
    };
    if !viewport_text.is_empty() {
        head.meta("viewport", viewport_text);
    }
    if let Some(title) = resolve_title(None, m) {
        head.tags.push(HeadTag::Title(title));
    }
    if let Some(description) = &m.description {
        head.meta("description", description.clone());
    }
    if let Some(name) = &m.application_name {
        head.meta("application-name", name.clone());
    }
    for author in &m.authors {
        if let Some(url) = field_str(author, "url") {
            head.link(vec![("rel", String::from("author")), ("href", url)]);
        }
        if let Some(name) = str_of(author).or_else(|| field_str(author, "name")) {
            head.meta("author", name);
        }
    }
    if let Some(generator) = &m.generator {
        head.meta("generator", generator.clone());
    }
    if !m.keywords.is_empty() {
        head.meta("keywords", m.keywords.join(","));
    }
    if let Some(referrer) = &m.referrer {
        head.meta("referrer", referrer.clone());
    }
    if let Some(creator) = &m.creator {
        head.meta("creator", creator.clone());
    }
    if let Some(publisher) = &m.publisher {
        head.meta("publisher", publisher.clone());
    }
    if let Some(robots) = &m.robots {
        if let Some(content) = robots_content(robots) {
            head.meta("robots", content);
        }
        if let Some(content) = field(robots, "googleBot").and_then(robots_content) {
            head.meta("googlebot", content);
        }
    }
    let theme_colors = if viewport.theme_color.is_empty() {
        m.theme_color_legacy
            .as_ref()
            .map(theme_colors_from_value)
            .unwrap_or_default()
    } else {
        viewport.theme_color.clone()
    };
    for theme in theme_colors {
        let mut attrs = vec![("name", String::from("theme-color"))];
        if let Some(media) = theme.media {
            attrs.push(("media", media));
        }
        attrs.push(("content", theme.color));
        head.tags.push(HeadTag::Meta(attrs));
    }
    if let Some(scheme) = viewport
        .color_scheme
        .clone()
        .or_else(|| m.color_scheme_legacy.clone())
    {
        head.meta("color-scheme", scheme);
    }
    if let Some(category) = &m.category {
        head.meta("category", category.clone());
    }
}

fn push_alternates(head: &mut Head, alternates: &Value) {
    if let Some(canonical) = field(alternates, "canonical").and_then(url_of) {
        let href = head.resolve(&canonical);
        head.link(vec![("rel", String::from("canonical")), ("href", href)]);
    }
    if let Some(Value::Object(languages)) = field(alternates, "languages") {
        for (lang, v) in languages {
            for item in as_list(v) {
                if let Some(url) = url_of(item) {
                    let href = head.resolve(&url);
                    head.link(vec![
                        ("rel", String::from("alternate")),
                        ("hreflang", lang.clone()),
                        ("href", href),
                    ]);
                }
            }
        }
    }
    if let Some(Value::Object(media)) = field(alternates, "media") {
        for (query, v) in media {
            if let Some(url) = url_of(v) {
                let href = head.resolve(&url);
                head.link(vec![
                    ("rel", String::from("alternate")),
                    ("media", query.clone()),
                    ("href", href),
                ]);
            }
        }
    }
    if let Some(Value::Object(types)) = field(alternates, "types") {
        for (mime, v) in types {
            for item in as_list(v) {
                if let Some(url) = url_of(item) {
                    let href = head.resolve(&url);
                    head.link(vec![
                        ("rel", String::from("alternate")),
                        ("type", mime.clone()),
                        ("href", href),
                    ]);
                }
            }
        }
    }
}

fn push_icon(head: &mut Head, rel: &str, item: &Value) {
    let Some(url) = url_of(item) else {
        return;
    };
    let mut attrs = vec![
        (
            "rel",
            field_str(item, "rel").unwrap_or_else(|| rel.to_owned()),
        ),
        ("href", url),
    ];
    if let Some(sizes) = field_str(item, "sizes") {
        attrs.push(("sizes", sizes));
    }
    if let Some(mime) = field_str(item, "type") {
        attrs.push(("type", mime));
    }
    if let Some(media) = field_str(item, "media") {
        attrs.push(("media", media));
    }
    head.link(attrs);
}

fn push_icons(head: &mut Head, icons: &Value) {
    match icons {
        Value::String(_) | Value::Array(_) => {
            for item in as_list(icons) {
                push_icon(head, "icon", item);
            }
        }
        Value::Object(_) => {
            for (key, rel) in [
                ("icon", "icon"),
                ("shortcut", "shortcut icon"),
                ("apple", "apple-touch-icon"),
                ("other", "icon"),
            ] {
                if let Some(v) = field(icons, key) {
                    for item in as_list(v) {
                        push_icon(head, rel, item);
                    }
                }
            }
        }
        _ => {}
    }
}

fn og_title(value: &Value) -> Option<String> {
    str_of(value)
        .or_else(|| field_str(value, "absolute"))
        .or_else(|| field_str(value, "default"))
}

// Table of Open Graph properties in `generate/opengraph.tsx` order.
#[allow(clippy::too_many_lines)]
fn push_open_graph(head: &mut Head, og: &Value) {
    if let Some(title) = field(og, "title").and_then(og_title) {
        head.property("og:title", title);
    }
    if let Some(description) = field_str(og, "description") {
        head.property("og:description", description);
    }
    if let Some(url) = field(og, "url").and_then(url_of) {
        let url = head.resolve(&url);
        head.property("og:url", url);
    }
    for (key, property) in [
        ("siteName", "og:site_name"),
        ("locale", "og:locale"),
        ("determiner", "og:determiner"),
        ("countryName", "og:country_name"),
        ("ttl", "og:ttl"),
    ] {
        if let Some(v) = field_str(og, key) {
            head.property(property, v);
        }
    }
    for (key, property) in [
        ("emails", "og:email"),
        ("phoneNumbers", "og:phone_number"),
        ("faxNumbers", "og:fax_number"),
        ("alternateLocale", "og:locale:alternate"),
    ] {
        if let Some(v) = field(og, key) {
            for item in str_list(v) {
                head.property(property, item);
            }
        }
    }
    if let Some(kind) = field_str(og, "type") {
        head.property("og:type", kind);
    }
    if let Some(images) = field(og, "images") {
        for image in as_list(images) {
            let Some(url) = url_of(image) else {
                continue;
            };
            let resolved = head.resolve(&url);
            head.property("og:image", resolved);
            if let Some(secure) = field_str(image, "secureUrl") {
                head.property("og:image:secure_url", secure);
            }
            if let Some(mime) = field_str(image, "type") {
                head.property("og:image:type", mime);
            }
            if let Some(width) = field_str(image, "width") {
                head.property("og:image:width", width);
            }
            if let Some(height) = field_str(image, "height") {
                head.property("og:image:height", height);
            }
            if let Some(alt) = field_str(image, "alt") {
                head.property("og:image:alt", alt);
            }
        }
    }
    for (key, property) in [("audio", "og:audio"), ("videos", "og:video")] {
        if let Some(v) = field(og, key) {
            for item in as_list(v) {
                if let Some(url) = url_of(item) {
                    let url = head.resolve(&url);
                    head.property(property, url);
                }
            }
        }
    }
    for (key, property) in [
        ("publishedTime", "article:published_time"),
        ("modifiedTime", "article:modified_time"),
        ("expirationTime", "article:expiration_time"),
        ("section", "article:section"),
        ("firstName", "profile:first_name"),
        ("lastName", "profile:last_name"),
        ("username", "profile:username"),
        ("gender", "profile:gender"),
        ("isbn", "book:isbn"),
        ("releaseDate", "book:release_date"),
    ] {
        if let Some(v) = field_str(og, key) {
            head.property(property, v);
        }
    }
    let is_book = field_str(og, "type").as_deref() == Some("book");
    for (key, property) in [("authors", "article:author"), ("tags", "article:tag")] {
        if let Some(v) = field(og, key) {
            let property = match (is_book, property) {
                (true, "article:author") => "book:author",
                (true, "article:tag") => "book:tag",
                (_, p) => p,
            };
            for item in as_list(v) {
                if let Some(text) = url_of(item) {
                    head.property(property, text);
                }
            }
        }
    }
}

fn push_twitter(head: &mut Head, twitter: &Value) {
    for (key, name) in [
        ("card", "twitter:card"),
        ("site", "twitter:site"),
        ("siteId", "twitter:site:id"),
        ("creator", "twitter:creator"),
        ("creatorId", "twitter:creator:id"),
    ] {
        if let Some(v) = field_str(twitter, key) {
            head.meta(name, v);
        }
    }
    if let Some(title) = field(twitter, "title").and_then(og_title) {
        head.meta("twitter:title", title);
    }
    if let Some(description) = field_str(twitter, "description") {
        head.meta("twitter:description", description);
    }
    if let Some(images) = field(twitter, "images") {
        for image in as_list(images) {
            if let Some(url) = url_of(image) {
                let url = head.resolve(&url);
                head.meta("twitter:image", url);
                if let Some(alt) = field_str(image, "alt") {
                    head.meta("twitter:image:alt", alt);
                }
            }
        }
    }
}

fn push_verification(head: &mut Head, verification: &Value) {
    for (key, name) in [
        ("google", "google-site-verification"),
        ("yahoo", "y_key"),
        ("yandex", "yandex-verification"),
        ("me", "me"),
    ] {
        if let Some(v) = field(verification, key) {
            for item in str_list(v) {
                head.meta(name, item);
            }
        }
    }
    if let Some(Value::Object(other)) = field(verification, "other") {
        for (name, v) in other {
            for item in str_list(v) {
                head.meta(name, item);
            }
        }
    }
}

fn push_apple_web_app(head: &mut Head, app: &Value) {
    match app {
        Value::Bool(true) => head.meta("apple-mobile-web-app-capable", "yes"),
        Value::Object(_) => {
            if field(app, "capable").and_then(Value::as_bool) == Some(true) {
                head.meta("apple-mobile-web-app-capable", "yes");
            }
            if let Some(title) = field_str(app, "title") {
                head.meta("apple-mobile-web-app-title", title);
            }
            if let Some(style) = field_str(app, "statusBarStyle") {
                head.meta("apple-mobile-web-app-status-bar-style", style);
            }
            if let Some(images) = field(app, "startupImage") {
                for image in as_list(images) {
                    if let Some(url) = url_of(image) {
                        let mut attrs = vec![
                            ("rel", String::from("apple-touch-startup-image")),
                            ("href", url),
                        ];
                        if let Some(media) = field_str(image, "media") {
                            attrs.push(("media", media));
                        }
                        head.link(attrs);
                    }
                }
            }
        }
        _ => {}
    }
}

fn push_format_detection(head: &mut Head, detection: &Value) {
    let Value::Object(map) = detection else {
        return;
    };
    let parts: Vec<String> = ["telephone", "date", "address", "email", "url"]
        .into_iter()
        .filter(|key| map.contains_key(*key))
        .map(|key| format!("{key}=no"))
        .collect();
    if !parts.is_empty() {
        head.meta("format-detection", parts.join(", "));
    }
}

fn push_other(head: &mut Head, other: &BTreeMap<String, Value>) {
    for (name, value) in other {
        for item in as_list(value) {
            if let Some(content) = str_of(item) {
                head.meta(name, content);
            }
        }
    }
}

/// Builds the ordered tag list of a metadata object.
fn head_tags(m: &Metadata, viewport: Option<&Viewport>) -> Vec<HeadTag> {
    let mut head = Head { base: m.metadata_base.clone(), tags: Vec::new() };
    push_basic(&mut head, m, viewport);
    if let Some(alternates) = &m.alternates {
        push_alternates(&mut head, alternates);
    }
    if let Some(manifest) = &m.manifest {
        head.link(vec![
            ("rel", String::from("manifest")),
            ("href", manifest.clone()),
        ]);
    }
    if let Some(icons) = &m.icons {
        push_icons(&mut head, icons);
    }
    if let Some(og) = &m.open_graph {
        push_open_graph(&mut head, og);
    }
    if let Some(twitter) = &m.twitter {
        push_twitter(&mut head, twitter);
    }
    if let Some(verification) = &m.verification {
        push_verification(&mut head, verification);
    }
    if let Some(app) = &m.apple_web_app {
        push_apple_web_app(&mut head, app);
    }
    if let Some(detection) = &m.format_detection {
        push_format_detection(&mut head, detection);
    }
    push_other(&mut head, &m.other);
    head.tags
}

// ============================================================================
// Renderers
// ============================================================================

/// Escapes text for HTML attribute and text content.
fn html_escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

/// Escapes text for a Rust string literal (and RSX format braces when `rsx` is set).
fn rust_escape(text: &str, rsx: bool) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '{' if rsx => out.push_str("{{"),
            '}' if rsx => out.push_str("}}"),
            _ => out.push(c),
        }
    }
    out
}

/// Renders `<head>` tags as HTML (one element per line).
pub fn render_head_html(metadata: &Metadata, viewport: Option<&Viewport>) -> String {
    head_tags(metadata, viewport)
        .iter()
        .map(|tag| match tag {
            HeadTag::Title(text) => format!("<title>{}</title>", html_escape(text)),
            HeadTag::Meta(attrs) => format!("<meta{}>", html_attrs(attrs)),
            HeadTag::Link(attrs) => format!("<link{}>", html_attrs(attrs)),
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn html_attrs(attrs: &[(&'static str, String)]) -> String {
    let mut out = String::new();
    for (k, v) in attrs {
        out.push(' ');
        out.push_str(k);
        out.push_str("=\"");
        out.push_str(&html_escape(v));
        out.push('"');
    }
    out
}

/// Dioxus `document::*` prop name of an HTML attribute.
fn dioxus_prop(attr: &str) -> String {
    match attr {
        "type" => String::from("r#type"),
        "http-equiv" => String::from("http_equiv"),
        other => other.replace('-', "_"),
    }
}

/// Renders Dioxus `document::{Title, Meta, Link}` RSX (4-space indented lines).
pub fn render_dioxus_head_rsx(metadata: &Metadata, viewport: Option<&Viewport>) -> String {
    head_tags(metadata, viewport)
        .iter()
        .map(|tag| match tag {
            HeadTag::Title(text) => {
                format!("    document::Title {{ \"{}\" }}", rust_escape(text, true))
            }
            HeadTag::Meta(attrs) => format!("    document::Meta {{ {} }}", rsx_props(attrs)),
            HeadTag::Link(attrs) => format!("    document::Link {{ {} }}", rsx_props(attrs)),
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn rsx_props(attrs: &[(&'static str, String)]) -> String {
    attrs
        .iter()
        .map(|(k, v)| format!("{}: \"{}\"", dioxus_prop(k), rust_escape(v, true)))
        .collect::<Vec<_>>()
        .join(", ")
}

/// `leptos_meta` prop name of an HTML attribute (`None` when the component has no such prop).
fn leptos_prop(component: &str, attr: &str) -> Option<String> {
    match (component, attr) {
        ("Meta", "http-equiv") => Some(String::from("http_equiv")),
        ("Link", "type") => Some(String::from("type_")),
        ("Link", "as") => Some(String::from("as_")),
        ("Meta", "charset" | "name" | "property" | "content") | ("Link", _) => {
            Some(attr.to_owned())
        }
        _ => None,
    }
}

/// Renders Leptos `leptos_meta` (`<Title/>`, `<Meta/>`, `<Link/>`) view code.
pub fn render_leptos_head(metadata: &Metadata, viewport: Option<&Viewport>) -> String {
    head_tags(metadata, viewport)
        .iter()
        .map(|tag| match tag {
            HeadTag::Title(text) => format!("<Title text=\"{}\"/>", rust_escape(text, false)),
            HeadTag::Meta(attrs) => format!("<Meta{}/>", leptos_attrs("Meta", attrs)),
            HeadTag::Link(attrs) => format!("<Link{}/>", leptos_attrs("Link", attrs)),
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn leptos_attrs(component: &str, attrs: &[(&'static str, String)]) -> String {
    let mut out = String::new();
    for (k, v) in attrs {
        if let Some(prop) = leptos_prop(component, k) {
            out.push(' ');
            out.push_str(&prop);
            out.push_str("=\"");
            out.push_str(&rust_escape(v, false));
            out.push('"');
        }
    }
    out
}

// ============================================================================
// Project helpers
// ============================================================================

/// Static metadata and viewport of the root layout (read from disk; defaults when absent).
pub(crate) fn root_metadata(analysis: &ProjectAnalysis) -> (Metadata, Option<Viewport>) {
    let Some(layout) = analysis.routes.root_layout.as_deref() else {
        return (Metadata::default(), None);
    };
    let path = std::path::Path::new(&analysis.root).join(layout);
    let Ok(source) = std::fs::read_to_string(path) else {
        return (Metadata::default(), None);
    };
    let config = crate::segment::analyze_segment_module(&source, layout);
    (
        config.static_metadata.unwrap_or_default(),
        config.static_viewport,
    )
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn root() -> Metadata {
        metadata_from_value(&json!({
            "metadataBase": "https://fixture.example.com",
            "title": { "default": "Yolo Fixture", "template": "%s | Yolo Fixture" },
            "description": "Root description",
            "applicationName": "Yolo Fixture",
            "keywords": ["yolo", "next"],
            "authors": [{ "name": "Aphrody", "url": "https://aphrody.com" }],
            "openGraph": {
                "title": "Yolo Fixture",
                "description": "Migration fixture",
                "url": "https://fixture.example.com",
                "siteName": "Yolo Fixture",
                "images": [{ "url": "/opengraph-image", "width": 1200, "height": 630, "alt": "Cover" }],
                "locale": "en_US",
                "type": "website"
            },
            "twitter": { "card": "summary_large_image", "site": "@aphrody", "images": "/twitter-image" },
            "robots": { "index": true, "follow": true, "googleBot": { "index": true, "follow": false, "max-snippet": -1 } },
            "alternates": { "canonical": "/", "languages": { "fr-FR": "/fr" } },
            "icons": { "icon": "/icon.svg", "apple": "/apple-icon.png", "shortcut": "/favicon.ico" },
            "manifest": "/manifest.webmanifest",
            "verification": { "google": "g-token", "other": { "me": ["https://me.example"] } },
            "appleWebApp": { "capable": true, "title": "Yolo", "statusBarStyle": "black-translucent" },
            "formatDetection": { "telephone": false },
            "other": { "custom": "value" },
            "themeColor": "#111111",
            "unknownKey": "kept"
        }))
    }

    #[test]
    fn parses_metadata_fields() {
        let m = root();
        assert_eq!(
            m.metadata_base.as_deref(),
            Some("https://fixture.example.com")
        );
        assert_eq!(
            m.title.as_ref().and_then(|t| t.default.as_deref()),
            Some("Yolo Fixture")
        );
        assert_eq!(m.keywords, vec!["yolo", "next"]);
        assert_eq!(m.authors.len(), 1);
        assert_eq!(m.theme_color_legacy, Some(json!("#111111")));
        assert_eq!(m.other.get("custom"), Some(&json!("value")));
        assert_eq!(m.other.get("unknownKey"), Some(&json!("kept")));
        let simple = metadata_from_value(
            &json!({ "title": "Plain", "keywords": "one,two", "authors": { "name": "Solo" } }),
        );
        assert_eq!(
            simple.title.as_ref().and_then(|t| t.default.as_deref()),
            Some("Plain")
        );
        assert_eq!(simple.keywords, vec!["one,two"]);
        assert_eq!(simple.authors.len(), 1);
    }

    #[test]
    fn parses_viewport() {
        let v = viewport_from_value(&json!({
            "width": 1024,
            "initialScale": 1,
            "maximumScale": 2.5,
            "userScalable": false,
            "viewportFit": "cover",
            "themeColor": [{ "media": "(prefers-color-scheme: dark)", "color": "#000" }, "#fff"],
            "colorScheme": "dark light"
        }));
        assert_eq!(v.width.as_deref(), Some("1024"));
        assert_eq!(v.theme_color.len(), 2);
        assert_eq!(v.theme_color[1].color, "#fff");
        assert_eq!(
            viewport_content(&v),
            "width=1024, initial-scale=1, maximum-scale=2.5, user-scalable=no, viewport-fit=cover"
        );
        assert_eq!(
            viewport_content(&default_viewport()),
            "width=device-width, initial-scale=1"
        );
        let simple = viewport_from_value(&json!({ "themeColor": "#abc" }));
        assert_eq!(
            simple.theme_color,
            vec![ThemeColor { media: None, color: "#abc".into() }]
        );
    }

    #[test]
    fn title_resolution_and_merge() {
        let parent = root();
        let child = metadata_from_value(&json!({ "title": "Pricing" }));
        assert_eq!(
            resolve_title(Some(&parent), &child).as_deref(),
            Some("Pricing | Yolo Fixture")
        );
        let absolute = metadata_from_value(&json!({ "title": { "absolute": "Raw" } }));
        assert_eq!(
            resolve_title(Some(&parent), &absolute).as_deref(),
            Some("Raw")
        );
        let none = Metadata::default();
        assert_eq!(
            resolve_title(Some(&parent), &none).as_deref(),
            Some("Yolo Fixture")
        );
        assert_eq!(resolve_title(None, &child).as_deref(), Some("Pricing"));

        let merged = merge_metadata(
            &parent,
            &metadata_from_value(&json!({ "description": "Child", "other": { "extra": "x" } })),
        );
        assert_eq!(merged.description.as_deref(), Some("Child"));
        assert_eq!(merged.application_name.as_deref(), Some("Yolo Fixture"));
        assert_eq!(merged.other.len(), 3);
        let merged_title = merge_metadata(&parent, &child);
        assert_eq!(
            merged_title
                .title
                .as_ref()
                .and_then(|t| t.template.as_deref()),
            Some("%s | Yolo Fixture")
        );
        assert_eq!(
            merged_title
                .title
                .as_ref()
                .and_then(|t| t.default.as_deref()),
            Some("Pricing")
        );
    }

    #[test]
    fn renders_html_head() {
        let m = root();
        let viewport = viewport_from_value(&json!({
            "width": "device-width",
            "initialScale": 1,
            "themeColor": [{ "media": "(prefers-color-scheme: dark)", "color": "#09090b" }],
            "colorScheme": "dark light"
        }));
        let html = render_head_html(&m, Some(&viewport));
        let expect = |needle: &str| assert!(html.contains(needle), "missing {needle} in\n{html}");
        expect("<meta charset=\"utf-8\">");
        expect("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">");
        expect("<title>Yolo Fixture</title>");
        expect("<meta name=\"description\" content=\"Root description\">");
        expect("<meta name=\"application-name\" content=\"Yolo Fixture\">");
        expect("<link rel=\"author\" href=\"https://aphrody.com\">");
        expect("<meta name=\"author\" content=\"Aphrody\">");
        expect("<meta name=\"keywords\" content=\"yolo,next\">");
        expect("<meta name=\"robots\" content=\"index, follow\">");
        expect("<meta name=\"googlebot\" content=\"index, nofollow, max-snippet:-1\">");
        expect(
            "<meta name=\"theme-color\" media=\"(prefers-color-scheme: dark)\" content=\"#09090b\">",
        );
        expect("<meta name=\"color-scheme\" content=\"dark light\">");
        expect("<link rel=\"canonical\" href=\"https://fixture.example.com/\">");
        expect(
            "<link rel=\"alternate\" hreflang=\"fr-FR\" href=\"https://fixture.example.com/fr\">",
        );
        expect("<link rel=\"manifest\" href=\"/manifest.webmanifest\">");
        expect("<link rel=\"icon\" href=\"/icon.svg\">");
        expect("<link rel=\"shortcut icon\" href=\"/favicon.ico\">");
        expect("<link rel=\"apple-touch-icon\" href=\"/apple-icon.png\">");
        expect("<meta property=\"og:title\" content=\"Yolo Fixture\">");
        expect("<meta property=\"og:site_name\" content=\"Yolo Fixture\">");
        expect("<meta property=\"og:locale\" content=\"en_US\">");
        expect("<meta property=\"og:type\" content=\"website\">");
        expect(
            "<meta property=\"og:image\" content=\"https://fixture.example.com/opengraph-image\">",
        );
        expect("<meta property=\"og:image:width\" content=\"1200\">");
        expect("<meta property=\"og:image:height\" content=\"630\">");
        expect("<meta property=\"og:image:alt\" content=\"Cover\">");
        expect("<meta name=\"twitter:card\" content=\"summary_large_image\">");
        expect("<meta name=\"twitter:site\" content=\"@aphrody\">");
        expect(
            "<meta name=\"twitter:image\" content=\"https://fixture.example.com/twitter-image\">",
        );
        expect("<meta name=\"google-site-verification\" content=\"g-token\">");
        expect("<meta name=\"me\" content=\"https://me.example\">");
        expect("<meta name=\"apple-mobile-web-app-capable\" content=\"yes\">");
        expect("<meta name=\"apple-mobile-web-app-title\" content=\"Yolo\">");
        expect(
            "<meta name=\"apple-mobile-web-app-status-bar-style\" content=\"black-translucent\">",
        );
        expect("<meta name=\"format-detection\" content=\"telephone=no\">");
        expect("<meta name=\"custom\" content=\"value\">");
        assert!(
            !html.contains("#111111"),
            "viewport theme color wins over legacy"
        );

        let escaped = render_head_html(
            &metadata_from_value(&json!({ "title": "A & <B>", "description": "say \"hi\"" })),
            None,
        );
        assert!(escaped.contains("<title>A &amp; &lt;B&gt;</title>"));
        assert!(escaped.contains("content=\"say &quot;hi&quot;\""));
        assert!(
            escaped.contains(
                "<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">"
            )
        );
    }

    #[test]
    fn renders_dioxus_and_leptos() {
        let m = metadata_from_value(&json!({
            "title": "Page {x}",
            "description": "Desc",
            "openGraph": { "title": "OG" },
            "alternates": { "canonical": "https://x.dev/" },
            "icons": { "icon": { "url": "/icon.png", "type": "image/png" } }
        }));
        let rsx = render_dioxus_head_rsx(&m, None);
        assert!(
            rsx.contains("    document::Title { \"Page {{x}}\" }"),
            "{rsx}"
        );
        assert!(rsx.contains("    document::Meta { name: \"description\", content: \"Desc\" }"));
        assert!(rsx.contains("    document::Meta { property: \"og:title\", content: \"OG\" }"));
        assert!(
            rsx.contains("    document::Link { rel: \"canonical\", href: \"https://x.dev/\" }")
        );
        assert!(rsx.contains("r#type: \"image/png\""));
        assert!(rsx.contains("document::Meta { charset: \"utf-8\" }"));
        assert!(rsx.lines().all(|l| l.starts_with("    ")));

        let leptos = render_leptos_head(&m, None);
        assert!(leptos.contains("<Title text=\"Page {x}\"/>"), "{leptos}");
        assert!(leptos.contains("<Meta name=\"description\" content=\"Desc\"/>"));
        assert!(leptos.contains("<Meta property=\"og:title\" content=\"OG\"/>"));
        assert!(leptos.contains("<Link rel=\"canonical\" href=\"https://x.dev/\"/>"));
        assert!(leptos.contains("<Link rel=\"icon\" href=\"/icon.png\" type_=\"image/png\"/>"));
        assert!(
            leptos.contains(
                "<Meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"/>"
            )
        );
    }

    #[test]
    fn url_resolution() {
        assert_eq!(resolve_url(Some("https://x.dev"), "/a"), "https://x.dev/a");
        assert_eq!(resolve_url(Some("https://x.dev/"), "a"), "https://x.dev/a");
        assert_eq!(
            resolve_url(Some("https://x.dev"), "https://y.dev/b"),
            "https://y.dev/b"
        );
        assert_eq!(resolve_url(None, "/a"), "/a");
        assert_eq!(
            resolve_url(Some("https://x.dev"), "//cdn/x.png"),
            "//cdn/x.png"
        );
    }
}
