// SPDX-License-Identifier: Apache-2.0
//! `catalog/assets.lock.toml`: the licensing gate for every icon source.

use crate::Error;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Licenses accepted by the GOAL.md licensing gate (SPDX ids or expressions).
pub const ALLOWED_LICENSES: &[&str] = &[
    "Apache-2.0",
    "MIT",
    "MIT OR Apache-2.0",
    "BSD-2-Clause",
    "BSD-3-Clause",
    "ISC",
    "OFL-1.1",
    "CC0-1.0",
    "CC-BY-3.0",
    "CC-BY-4.0",
    "CC-BY-SA-3.0",
    "CC-BY-SA-4.0",
];

/// Upstream markers that must never be listed (non-redistributable assets).
pub const FORBIDDEN_MARKERS: &[&str] = &[
    "sf-symbols",
    "sf symbols",
    "sfsymbols",
    "segoe fluent",
    "segoe-fluent",
    "segoe ui variable",
    "segmdl2",
];

/// Source file format of a set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SourceFormat {
    Svg,
    ChromiumIcon,
}

impl SourceFormat {
    #[must_use]
    pub const fn extension(self) -> &'static str {
        match self {
            Self::Svg => "svg",
            Self::ChromiumIcon => "icon",
        }
    }
}

/// One icon set.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetSet {
    pub id: String,
    pub name: String,
    pub format: SourceFormat,
    pub license: String,
    pub upstream: String,
    #[serde(default)]
    pub mirror: Option<String>,
    #[serde(default)]
    pub commit: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub retrieved: Option<String>,
    pub attribution: String,
    /// `local` (original assets committed as-is), `github-raw`, `submodule` or `chromium`.
    pub fetch: String,
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default)]
    pub license_path: Option<String>,
    #[serde(default)]
    pub submodule: Option<String>,
    /// Repository-relative directory holding the materialized subset.
    pub dir: String,
    /// Merge priority: lower wins when sets share a canonical name.
    pub priority: u32,
    /// Canonical name -> upstream path (or, for `local` sets, the file name inside `dir`).
    pub icons: BTreeMap<String, String>,
}

impl AssetSet {
    /// Repository-relative path of the materialized file for `name`.
    #[must_use]
    pub fn local_file(&self, root: &Path, name: &str) -> PathBuf {
        let dir = root.join(&self.dir);
        if self.fetch == "local" {
            dir.join(&self.icons[name])
        } else {
            dir.join(format!("{name}.{}", self.format.extension()))
        }
    }
}

/// The whole lock file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetLock {
    pub schema: u32,
    #[serde(rename = "set")]
    pub sets: Vec<AssetSet>,
}

impl AssetLock {
    /// Parses and validates a lock file.
    ///
    /// # Errors
    /// [`Error::Lock`] on TOML errors or when an entry fails the licensing gate.
    pub fn parse(text: &str) -> Result<Self, Error> {
        let lock: Self = toml::from_str(text).map_err(|e| Error::Lock(e.to_string()))?;
        lock.validate()?;
        Ok(lock)
    }

    /// Reads and validates `path`.
    ///
    /// # Errors
    /// I/O or validation errors.
    pub fn load(path: &Path) -> Result<Self, Error> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| Error::Io(format!("{}: {e}", path.display())))?;
        Self::parse(&text)
    }

    /// Applies the licensing gate.
    ///
    /// # Errors
    /// [`Error::Lock`] describing the first violation.
    pub fn validate(&self) -> Result<(), Error> {
        if self.schema != 1 {
            return Err(Error::Lock(format!("unsupported schema {}", self.schema)));
        }
        let mut ids = std::collections::BTreeSet::new();
        for set in &self.sets {
            if !ids.insert(set.id.as_str()) {
                return Err(Error::Lock(format!("duplicate set id `{}`", set.id)));
            }
            if !ALLOWED_LICENSES.contains(&set.license.as_str()) {
                return Err(Error::Lock(format!(
                    "set `{}`: license `{}` not allowed",
                    set.id, set.license
                )));
            }
            if set.commit.is_none() && set.version.is_none() {
                return Err(Error::Lock(format!(
                    "set `{}`: needs a pinned commit or version",
                    set.id
                )));
            }
            if set.attribution.trim().is_empty() {
                return Err(Error::Lock(format!(
                    "set `{}`: attribution is required",
                    set.id
                )));
            }
            let haystack = format!(
                "{} {} {} {}",
                set.name,
                set.upstream,
                set.mirror.as_deref().unwrap_or(""),
                set.base_url.as_deref().unwrap_or("")
            )
            .to_lowercase();
            if let Some(marker) = FORBIDDEN_MARKERS.iter().find(|m| haystack.contains(*m)) {
                return Err(Error::Lock(format!(
                    "set `{}`: non-redistributable source `{marker}`",
                    set.id
                )));
            }
            if !matches!(
                set.fetch.as_str(),
                "local" | "github-raw" | "submodule" | "chromium"
            ) {
                return Err(Error::Lock(format!(
                    "set `{}`: unknown fetch mode `{}`",
                    set.id, set.fetch
                )));
            }
            if set.fetch != "local" && set.license_path.is_none() {
                return Err(Error::Lock(format!(
                    "set `{}`: third-party sets must ship their license",
                    set.id
                )));
            }
            for name in set.icons.keys() {
                if name.is_empty()
                    || !name
                        .bytes()
                        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
                {
                    return Err(Error::Lock(format!(
                        "set `{}`: invalid canonical name `{name}`",
                        set.id
                    )));
                }
            }
        }
        Ok(())
    }

    /// Sets sorted by merge priority.
    #[must_use]
    pub fn by_priority(&self) -> Vec<&AssetSet> {
        let mut sets: Vec<&AssetSet> = self.sets.iter().collect();
        sets.sort_by_key(|s| (s.priority, s.id.clone()));
        sets
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINIMAL: &str = r#"
schema = 1
[[set]]
id = "x"
name = "X"
format = "svg"
license = "MIT"
upstream = "https://example.invalid/x"
commit = "abc"
attribution = "X authors"
fetch = "github-raw"
license_path = "LICENSE"
dir = "d"
priority = 1
[set.icons]
home = "a/home.svg"
"#;

    #[test]
    fn minimal_lock_parses() {
        let lock = AssetLock::parse(MINIMAL).unwrap();
        assert_eq!(lock.sets[0].icons["home"], "a/home.svg");
    }

    #[test]
    fn gate_rejects_unlicensed_and_forbidden_sources() {
        assert!(AssetLock::parse(&MINIMAL.replace("\"MIT\"", "\"Proprietary\"")).is_err());
        assert!(
            AssetLock::parse(&MINIMAL.replace(
                "https://example.invalid/x",
                "https://developer.apple.com/sf-symbols/"
            ))
            .is_err()
        );
        assert!(AssetLock::parse(&MINIMAL.replace("commit = \"abc\"\n", "")).is_err());
    }
}
