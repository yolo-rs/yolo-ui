//! 308 redirects: `/` -> `/fr`, legacy French segments -> canonical segments.

use axum::{
    extract::Request,
    middleware::Next,
    response::{IntoResponse, Redirect, Response},
};

use crate::i18n::Lang;

/// Legacy (French) first-level segments of dragonballfr.com and their canonical
/// replacement. The rest of the path and the query string are preserved.
pub const LEGACY_SEGMENTS: &[(&str, &str)] = &[
    ("personnages", "characters"),
    ("encyclopedie", "wiki"),
    ("fiches", "wiki"),
    ("mangas", "manga"),
    ("tomes", "manga"),
    ("livres", "databooks"),
    ("guides", "databooks"),
];

/// Returns the canonical path for a legacy path, or `None` when the path is
/// already canonical. A missing locale prefix is completed with the default
/// locale (`/personnages/goku` -> `/fr/characters/goku`).
pub fn rewrite_legacy_path(path: &str) -> Option<String> {
    let mut segments = path
        .trim_start_matches('/')
        .split('/')
        .filter(|s| !s.is_empty());
    let first = segments.next()?;
    let (lang, legacy) = match Lang::parse(first) {
        Some(lang) => (lang, segments.next()?),
        None => (Lang::DEFAULT, first),
    };
    let canonical = LEGACY_SEGMENTS
        .iter()
        .find(|(from, _)| *from == legacy)
        .map(|(_, to)| *to)?;
    let mut out = format!("/{}/{canonical}", lang.code());
    for rest in segments {
        out.push('/');
        out.push_str(rest);
    }
    Some(out)
}

/// Axum middleware: answers 308 for legacy paths, otherwise passes through.
pub async fn legacy_redirect(req: Request, next: Next) -> Response {
    if let Some(mut target) = rewrite_legacy_path(req.uri().path()) {
        if let Some(q) = req.uri().query() {
            target.push('?');
            target.push_str(q);
        }
        return Redirect::permanent(&target).into_response();
    }
    next.run(req).await
}

/// `GET /` -> 308 to the default locale home.
pub async fn root_redirect() -> Redirect {
    Redirect::permanent(&format!("/{}", Lang::DEFAULT.code()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_segments_are_rewritten() {
        assert_eq!(
            rewrite_legacy_path("/fr/personnages"),
            Some("/fr/characters".into())
        );
        assert_eq!(
            rewrite_legacy_path("/fr/personnages/son-goku"),
            Some("/fr/characters/son-goku".into())
        );
        assert_eq!(
            rewrite_legacy_path("/en/encyclopedie/bulma"),
            Some("/en/wiki/bulma".into())
        );
        assert_eq!(
            rewrite_legacy_path("/personnages"),
            Some("/fr/characters".into())
        );
    }

    #[test]
    fn canonical_paths_pass_through() {
        assert_eq!(rewrite_legacy_path("/fr/characters"), None);
        assert_eq!(rewrite_legacy_path("/ja/manga/1/1"), None);
        assert_eq!(rewrite_legacy_path("/"), None);
        assert_eq!(rewrite_legacy_path("/sitemap.xml"), None);
    }
}
