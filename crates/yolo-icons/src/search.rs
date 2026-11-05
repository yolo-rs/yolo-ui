// SPDX-License-Identifier: Apache-2.0
//! Icon search across every set recorded in the lock file.

use crate::lock::AssetLock;
use serde::Serialize;
use std::path::Path;

/// One search hit.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Hit {
    pub name: String,
    pub set: String,
    pub license: String,
    pub score: u32,
    /// Upstream path (or local file name for original sets).
    pub source: String,
    /// Repository-relative materialized file.
    pub file: String,
    /// Whether the file has been fetched.
    pub available: bool,
    /// True when this set wins the merge for `name`.
    pub merged: bool,
}

fn subsequence(needle: &str, hay: &str) -> bool {
    let mut it = hay.chars();
    needle.chars().all(|c| it.any(|h| h == c))
}

/// Scores `name`/`source` against a normalized query; 0 means no match.
fn score(query: &str, name: &str, source: &str) -> u32 {
    if name == query {
        return 100;
    }
    if name.starts_with(query) {
        return 80;
    }
    if name.contains(query) {
        return 60;
    }
    let source = source.to_lowercase().replace(['-', ' '], "_");
    if source.contains(query) {
        return 40;
    }
    let tokens: Vec<&str> = query.split('_').filter(|t| !t.is_empty()).collect();
    if tokens.len() > 1
        && tokens
            .iter()
            .all(|t| name.contains(t) || source.contains(t))
    {
        return 30;
    }
    if query.len() >= 3 && subsequence(query, name) {
        return 15;
    }
    0
}

/// Searches every set; results are sorted by score, then merge priority, then name.
#[must_use]
pub fn search(lock: &AssetLock, root: &Path, query: &str, limit: usize) -> Vec<Hit> {
    let q = query.trim().to_lowercase().replace(['-', ' '], "_");
    let sets = lock.by_priority();
    let mut winners = std::collections::BTreeMap::new();
    for set in &sets {
        for name in set.icons.keys() {
            if set.local_file(root, name).is_file() {
                winners
                    .entry(name.clone())
                    .or_insert_with(|| set.id.clone());
            }
        }
    }
    let mut hits = Vec::new();
    for (rank, set) in sets.iter().enumerate() {
        for (name, source) in &set.icons {
            let s = if q.is_empty() {
                1
            } else {
                score(&q, name, source)
            };
            if s == 0 {
                continue;
            }
            let file = set
                .local_file(Path::new(""), name)
                .to_string_lossy()
                .replace('\\', "/");
            hits.push((
                rank,
                Hit {
                    name: name.clone(),
                    set: set.id.clone(),
                    license: set.license.clone(),
                    score: s,
                    source: source.clone(),
                    available: root.join(&file).is_file(),
                    merged: winners.get(name) == Some(&set.id),
                    file,
                },
            ));
        }
    }
    hits.sort_by(|(ra, a), (rb, b)| {
        b.score
            .cmp(&a.score)
            .then(ra.cmp(rb))
            .then(a.name.cmp(&b.name))
    });
    hits.into_iter().take(limit).map(|(_, h)| h).collect()
}

#[cfg(test)]
mod tests {
    use super::score;

    #[test]
    fn scoring_prefers_exact_then_prefix() {
        assert_eq!(score("home", "home", "x"), 100);
        assert_eq!(score("arrow", "arrow_back", "x"), 80);
        assert_eq!(score("back", "arrow_back", "x"), 60);
        assert_eq!(
            score(
                "dismiss",
                "close",
                "assets/Dismiss/SVG/ic_fluent_dismiss_24_regular.svg"
            ),
            40
        );
        assert_eq!(score("zzz", "home", "x"), 0);
    }
}
