//! Content catalog used to prerender static routes and to emit the sitemap.
//!
//! In the skeleton the catalog is a set of constants; the real site feeds it
//! from the Shenron API (wiki entities, manga volumes, databooks).

use crate::i18n::Lang;

/// Public origin, used for canonical URLs, `og:url`, hreflang and the sitemap.
pub const SITE_ORIGIN: &str = "https://dragonballfr.com";
/// Default OpenGraph image, served from `public/`.
pub const OG_IMAGE_PATH: &str = "/og/shenron.png";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WikiKind {
    Character,
    Technique,
    Place,
}

#[derive(Clone, Copy, Debug)]
pub struct WikiEntry {
    pub slug: &'static str,
    pub kind: WikiKind,
}

pub const WIKI: &[WikiEntry] = &[
    WikiEntry { slug: "son-goku", kind: WikiKind::Character },
    WikiEntry { slug: "bulma", kind: WikiKind::Character },
    WikiEntry { slug: "piccolo", kind: WikiKind::Character },
    WikiEntry { slug: "vegeta", kind: WikiKind::Character },
    WikiEntry { slug: "freezer", kind: WikiKind::Character },
    WikiEntry { slug: "kamehameha", kind: WikiKind::Technique },
    WikiEntry { slug: "genkidama", kind: WikiKind::Technique },
    WikiEntry { slug: "namek", kind: WikiKind::Place },
];

pub const DATABOOKS: &[&str] = &[
    "daizenshuu-1",
    "daizenshuu-7",
    "chouzenshuu-1",
    "super-exciting-guide",
];

/// `(volume, number of chapters in that volume)`.
pub const MANGA_VOLUMES: &[(u32, u32)] = &[(1, 11), (2, 12), (3, 12)];

pub fn wiki_slugs() -> Vec<String> {
    WIKI.iter().map(|e| e.slug.to_owned()).collect()
}

pub fn character_slugs() -> impl Iterator<Item = &'static str> {
    WIKI.iter()
        .filter(|e| e.kind == WikiKind::Character)
        .map(|e| e.slug)
}

pub fn databook_slugs() -> Vec<String> {
    DATABOOKS.iter().map(|s| (*s).to_owned()).collect()
}

/// Every `(volume, chapter)` pair.
pub fn manga_chapters() -> Vec<(u32, u32)> {
    MANGA_VOLUMES
        .iter()
        .flat_map(|(v, n)| (1..=*n).map(move |c| (*v, c)))
        .collect()
}

pub fn langs() -> Vec<String> {
    Lang::ALL.iter().map(|l| l.code().to_owned()).collect()
}

/// Locale-independent paths (without the `/{lang}` prefix) of every page.
pub fn all_paths() -> Vec<String> {
    let mut paths = vec![String::new(), "/characters".to_owned(), "/theme".to_owned()];
    paths.extend(WIKI.iter().map(|e| format!("/wiki/{}", e.slug)));
    paths.extend(
        manga_chapters()
            .into_iter()
            .map(|(v, c)| format!("/manga/{v}/{c}")),
    );
    paths.extend(DATABOOKS.iter().map(|s| format!("/databooks/{s}")));
    paths
}

/// Absolute URL of a page for a locale.
pub fn url(lang: Lang, path: &str) -> String {
    format!("{SITE_ORIGIN}/{}{path}", lang.code())
}

/// Human-readable label from a slug (`son-goku` -> `Son Goku`).
pub fn humanize(slug: &str) -> String {
    slug.split('-')
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// `"{volume}/{chapter}"` value of the `*vc` route parameter for a pair.
pub fn manga_vc(volume: u32, chapter: u32) -> String {
    format!("{volume}/{chapter}")
}

/// Static-route values for the `*vc` splat: exactly the existing pairs, once each.
pub fn manga_vc_values() -> Vec<String> {
    manga_chapters()
        .into_iter()
        .map(|(v, c)| manga_vc(v, c))
        .collect()
}

/// Parses a `*vc` value (`"1/3"`). Rejects malformed input (extra segments, signs, zero,
/// leading zeros, non-digits) and pairs absent from the catalog.
pub fn parse_manga_vc(vc: &str) -> Option<(u32, u32)> {
    let (v, c) = vc.split_once('/')?;
    let canonical = |s: &str| {
        !s.is_empty()
            && s.len() <= 9
            && s.bytes().all(|b| b.is_ascii_digit())
            && !s.starts_with('0')
    };
    if !canonical(v) || !canonical(c) {
        return None;
    }
    let (v, c): (u32, u32) = (v.parse().ok()?, c.parse().ok()?);
    MANGA_VOLUMES
        .iter()
        .any(|(vol, n)| *vol == v && (1..=*n).contains(&c))
        .then_some((v, c))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn vc_values_are_exactly_the_real_pairs_once() {
        let values = manga_vc_values();
        let real: usize = MANGA_VOLUMES.iter().map(|(_, n)| *n as usize).sum();
        assert_eq!(values.len(), real);
        assert_eq!(values.iter().collect::<HashSet<_>>().len(), real);
        for v in &values {
            assert!(parse_manga_vc(v).is_some(), "{v}");
        }
    }

    #[test]
    fn parser_accepts_real_pairs() {
        assert_eq!(parse_manga_vc("1/3"), Some((1, 3)));
        assert_eq!(parse_manga_vc("3/12"), Some((3, 12)));
    }

    #[test]
    fn parser_rejects_malformed_and_absent_pairs() {
        for bad in [
            "",
            "1",
            "1/",
            "/1",
            "1/3/4",
            "a/b",
            "1/x",
            "-1/1",
            "+1/1",
            "0/1",
            "1/0",
            "01/1",
            "1/03",
            "1/12",
            "2/13",
            "4/1",
            "99999999999/1",
            " 1/1",
            "1/1 ",
            "1//1",
        ] {
            assert_eq!(parse_manga_vc(bad), None, "{bad:?}");
        }
    }
}
