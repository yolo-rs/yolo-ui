//! Visible strings. Catalogues are flat JSON files (`messages/{fr,en,ja}.json`) embedded with
//! `include_str!`. French is the reference; English and Japanese are complete (a test enforces
//! identical key sets and that every key referenced in the sources exists).
//!
//! Keys reused verbatim from `apps/site/messages/*.json` of Shenron keep their dotted path there
//! (`admin.pages.ai.title`, `admin.confirm.*`, `canonEditorSync.*`, `adminDashboard.retry`); new
//! keys live under `adminAi.*` and are meant to be upstreamed into the Shenron catalogues.

use std::collections::HashMap;
use std::sync::OnceLock;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum Lang {
    #[default]
    Fr,
    En,
    Ja,
}

impl Lang {
    pub const ALL: [Lang; 3] = [Lang::Fr, Lang::En, Lang::Ja];

    pub fn code(self) -> &'static str {
        match self {
            Lang::Fr => "fr",
            Lang::En => "en",
            Lang::Ja => "ja",
        }
    }

    pub fn parse(code: &str) -> Option<Lang> {
        let primary = code.trim().split(['-', '_']).next()?.to_ascii_lowercase();
        match primary.as_str() {
            "fr" => Some(Lang::Fr),
            "en" => Some(Lang::En),
            "ja" => Some(Lang::Ja),
            _ => None,
        }
    }

    /// First supported language of an `Accept-Language` header (weights in order), else French.
    pub fn from_accept_language(header: &str) -> Lang {
        let mut tags: Vec<(f32, Lang)> = header
            .split(',')
            .filter_map(|part| {
                let mut it = part.split(';');
                let lang = Lang::parse(it.next()?)?;
                let q = it
                    .find_map(|p| {
                        p.trim()
                            .strip_prefix("q=")
                            .and_then(|v| v.parse::<f32>().ok())
                    })
                    .unwrap_or(1.0);
                Some((q, lang))
            })
            .collect();
        tags.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        tags.first().map(|t| t.1).unwrap_or_default()
    }
}

const FR_JSON: &str = include_str!("../messages/fr.json");
const EN_JSON: &str = include_str!("../messages/en.json");
const JA_JSON: &str = include_str!("../messages/ja.json");

type Catalog = HashMap<String, String>;

fn catalog(lang: Lang) -> &'static Catalog {
    static FR: OnceLock<Catalog> = OnceLock::new();
    static EN: OnceLock<Catalog> = OnceLock::new();
    static JA: OnceLock<Catalog> = OnceLock::new();
    let (cell, raw) = match lang {
        Lang::Fr => (&FR, FR_JSON),
        Lang::En => (&EN, EN_JSON),
        Lang::Ja => (&JA, JA_JSON),
    };
    cell.get_or_init(|| serde_json::from_str(raw).expect("valid message catalogue"))
}

/// Message for `key`; falls back to French, then to the key itself (visible in tests).
pub fn t(lang: Lang, key: &str) -> &'static str {
    catalog(lang)
        .get(key)
        .or_else(|| catalog(Lang::Fr).get(key))
        .map(String::as_str)
        .unwrap_or_else(|| {
            // Leaks only for a missing key, which the catalogue test forbids.
            Box::leak(key.to_owned().into_boxed_str())
        })
}

/// Message with `{name}` placeholders replaced.
pub fn tf(lang: Lang, key: &str, args: &[(&str, &str)]) -> String {
    let mut s = t(lang, key).to_owned();
    for (name, value) in args {
        s = s.replace(&format!("{{{name}}}"), value);
    }
    s
}

pub fn keys(lang: Lang) -> impl Iterator<Item = &'static str> {
    catalog(lang).keys().map(String::as_str)
}

pub fn has(lang: Lang, key: &str) -> bool {
    catalog(lang).contains_key(key)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn catalogues_have_identical_keys_and_no_blank_values() {
        let fr: BTreeSet<_> = keys(Lang::Fr).collect();
        for lang in [Lang::En, Lang::Ja] {
            let other: BTreeSet<_> = keys(lang).collect();
            let missing: Vec<_> = fr.difference(&other).collect();
            let extra: Vec<_> = other.difference(&fr).collect();
            assert!(
                missing.is_empty() && extra.is_empty(),
                "{lang:?}: missing {missing:?} extra {extra:?}"
            );
            for k in &other {
                assert!(!t(lang, k).trim().is_empty(), "{lang:?}: blank {k}");
            }
        }
    }

    /// Every `"adminAi.…"`, `"admin.…"`, `"canonEditorSync.…"` or `"adminDashboard.…"` literal in
    /// the sources must exist in the catalogues (no hard-coded fallbacks reach the screen).
    #[test]
    fn every_referenced_key_exists() {
        const SOURCES: &[&str] = &[
            include_str!("ui/mod.rs"),
            include_str!("ui/components.rs"),
            include_str!("ui/header.rs"),
            include_str!("ui/overview.rs"),
            include_str!("ui/training.rs"),
            include_str!("ui/canon.rs"),
            include_str!("ui/corpus.rs"),
            include_str!("ui/agents.rs"),
            include_str!("format.rs"),
            include_str!("gate.rs"),
            include_str!("server/mod.rs"),
        ];
        let prefixes = [
            "\"adminAi.",
            "\"admin.",
            "\"canonEditorSync.",
            "\"adminDashboard.",
        ];
        let mut missing = Vec::new();
        for src in SOURCES {
            for p in prefixes {
                let mut rest = *src;
                while let Some(i) = rest.find(p) {
                    let after = &rest[i + 1..];
                    let end = after.find('"').unwrap_or(after.len());
                    let key = after[..end].trim_end_matches('\\');
                    // Dynamic prefixes end with a dot and are completed at runtime; check suffixes there.
                    if !key.ends_with('.') && !key.contains('{') && !has(Lang::Fr, key) {
                        missing.push(key.to_owned());
                    }
                    rest = &after[end..];
                }
            }
        }
        assert!(missing.is_empty(), "missing keys: {missing:?}");
    }

    #[test]
    fn accept_language_and_placeholders() {
        assert_eq!(
            Lang::from_accept_language("en-US,en;q=0.9,fr;q=0.8"),
            Lang::En
        );
        assert_eq!(Lang::from_accept_language("de, ja;q=0.5"), Lang::Ja);
        assert_eq!(Lang::from_accept_language(""), Lang::Fr);
        assert_eq!(Lang::parse("JA_jp"), Some(Lang::Ja));
        assert!(tf(Lang::Fr, "adminAi.header.failing", &[("count", "3")]).contains('3'));
    }
}
