//! Minimal i18n layer.
//!
//! The three catalogs `messages/{fr,en,ja}.json` are embedded at compile time
//! with `include_str!` and parsed once (lazily) with serde. Every string shown
//! to a visitor goes through [`t`] / [`tf`]: no hardcoded visible text lives
//! in the components.

use std::{collections::HashMap, sync::LazyLock};

use serde::Deserialize;

/// Supported locales, in canonical order (also the sitemap / hreflang order).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Lang {
    Fr,
    En,
    Ja,
}

impl Lang {
    pub const ALL: [Lang; 3] = [Lang::Fr, Lang::En, Lang::Ja];
    /// Locale used when the URL carries none (root redirect, x-default).
    pub const DEFAULT: Lang = Lang::Fr;

    /// URL prefix segment and `hreflang` value.
    pub const fn code(self) -> &'static str {
        match self {
            Lang::Fr => "fr",
            Lang::En => "en",
            Lang::Ja => "ja",
        }
    }

    /// `og:locale` value.
    pub const fn og_locale(self) -> &'static str {
        match self {
            Lang::Fr => "fr_FR",
            Lang::En => "en_US",
            Lang::Ja => "ja_JP",
        }
    }

    /// Catalog key of the locale's own display name (`lang.fr`, ...).
    pub fn name_key(self) -> String {
        format!("lang.{}", self.code())
    }

    pub fn parse(code: &str) -> Option<Lang> {
        Lang::ALL.iter().copied().find(|l| l.code() == code)
    }

    /// Reads the locale from the first segment of an absolute path.
    pub fn from_path(path: &str) -> Option<Lang> {
        path.trim_start_matches('/')
            .split('/')
            .next()
            .and_then(Lang::parse)
    }

    fn catalog(self) -> &'static Catalog {
        match self {
            Lang::Fr => &CATALOGS.fr,
            Lang::En => &CATALOGS.en,
            Lang::Ja => &CATALOGS.ja,
        }
    }
}

#[derive(Deserialize)]
#[serde(transparent)]
struct Catalog(HashMap<String, String>);

struct Catalogs {
    fr: Catalog,
    en: Catalog,
    ja: Catalog,
}

const FR_JSON: &str = include_str!("../messages/fr.json");
const EN_JSON: &str = include_str!("../messages/en.json");
const JA_JSON: &str = include_str!("../messages/ja.json");

static CATALOGS: LazyLock<Catalogs> = LazyLock::new(|| Catalogs {
    fr: parse(FR_JSON, "fr"),
    en: parse(EN_JSON, "en"),
    ja: parse(JA_JSON, "ja"),
});

fn parse(json: &'static str, code: &str) -> Catalog {
    serde_json::from_str(json).unwrap_or_else(|e| panic!("messages/{code}.json: {e}"))
}

/// Looks up `key` in the locale catalog, falling back to the default locale,
/// then to the key itself (so a missing translation is visible, never a panic).
pub fn t(lang: Lang, key: &str) -> &'static str {
    lang.catalog()
        .0
        .get(key)
        .or_else(|| Lang::DEFAULT.catalog().0.get(key))
        .map(String::as_str)
        .unwrap_or_else(|| leak_key(key))
}

fn leak_key(key: &str) -> &'static str {
    // Only reached for a key missing from every catalog (caught by tests);
    // leaking keeps the public signature `&'static str`.
    Box::leak(key.to_owned().into_boxed_str())
}

/// Like [`t`] but substitutes `{name}` placeholders.
pub fn tf(lang: Lang, key: &str, args: &[(&str, &str)]) -> String {
    let mut out = t(lang, key).to_owned();
    for (name, value) in args {
        out = out.replace(&format!("{{{name}}}"), value);
    }
    out
}

/// Keys present in the default catalog (used by tests and tooling).
pub fn keys() -> impl Iterator<Item = &'static str> {
    Lang::DEFAULT.catalog().0.keys().map(String::as_str)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_locale_has_every_key() {
        for lang in Lang::ALL {
            for key in keys() {
                assert!(
                    lang.catalog().0.contains_key(key),
                    "messages/{}.json misses {key}",
                    lang.code()
                );
            }
        }
    }

    #[test]
    fn placeholders_are_substituted() {
        let s = tf(
            Lang::En,
            "manga.heading",
            &[("volume", "1"), ("chapter", "3")],
        );
        assert_eq!(s, "Volume 1 - Chapter 3");
    }

    #[test]
    fn lang_from_path() {
        assert_eq!(Lang::from_path("/ja/wiki/goku"), Some(Lang::Ja));
        assert_eq!(Lang::from_path("/sitemap.xml"), None);
    }
}
