// SPDX-License-Identifier: MIT OR Apache-2.0
//! Application identity, platform directories, `YoloManifest` bridge, and about box data.

use std::path::PathBuf;

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use yolo_shared::{YoloManifest, YoloThemePreset};

use crate::{Error, Result};

/// Identity and about box metadata of a Yolo UI native application.
///
/// `id` is a reverse-DNS identifier (e.g. `org.example.studio`, `io.yolo.showcase`,
/// or `com.aphrody.photocraft`). It names the single-instance mutex/socket, the Linux
/// desktop entry and the macOS bundle identifier, and its first two segments give the
/// platform directory qualifier and organization.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub authors: Vec<String>,
    #[serde(default)]
    pub website: Option<String>,
    #[serde(default)]
    pub license: Option<String>,
    #[serde(default)]
    pub copyright: Option<String>,
    #[serde(default)]
    pub theme_preset: YoloThemePreset,
}

/// Builds an [`AppInfo`] from the calling crate's Cargo metadata.
///
/// ```ignore
/// let app = yolo_shell::app_info!("org.yolo.studio", "Yolo Studio");
/// ```
#[macro_export]
macro_rules! app_info {
    ($id:expr, $name:expr) => {{
        let authors = env!("CARGO_PKG_AUTHORS");
        let description = env!("CARGO_PKG_DESCRIPTION");
        let website = env!("CARGO_PKG_HOMEPAGE");
        let license = env!("CARGO_PKG_LICENSE");
        let mut info = $crate::AppInfo::new($id, $name, env!("CARGO_PKG_VERSION"));
        info.authors = authors
            .split(':')
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
            .collect();
        info.description = (!description.is_empty()).then(|| description.to_owned());
        info.website = (!website.is_empty()).then(|| website.to_owned());
        info.license = (!license.is_empty()).then(|| license.to_owned());
        info
    }};
}

impl AppInfo {
    /// Creates app metadata with the mandatory fields and neutral default theme preset.
    pub fn new(id: impl Into<String>, name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            version: version.into(),
            description: None,
            authors: Vec::new(),
            website: None,
            license: None,
            copyright: None,
            theme_preset: YoloThemePreset::Neutral,
        }
    }

    /// Constructs an [`AppInfo`] directly from any [`YoloManifest`] (`yolo-neutral`, `yolo-m3`,
    /// `yolo-fluent`, `yolo-apple`, `yolo-aphrody`).
    pub fn from_manifest(manifest: &YoloManifest) -> Self {
        Self {
            id: manifest.app_id.clone(),
            name: manifest.title.clone(),
            version: manifest.version.clone(),
            description: None,
            authors: vec![manifest.organization.clone()],
            website: None,
            license: Some(String::from("MIT OR Apache-2.0")),
            copyright: None,
            theme_preset: manifest.theme_preset,
        }
    }

    /// Converts this [`AppInfo`] into a [`YoloManifest`] with its configured [`YoloThemePreset`].
    pub fn to_manifest(&self) -> YoloManifest {
        let (_, org) = self.qualifier_organization();
        let mut manifest = YoloManifest::new(&self.id, &self.name).with_preset(self.theme_preset);
        manifest.version = self.version.clone();
        manifest.organization = self
            .authors
            .first()
            .cloned()
            .unwrap_or_else(|| org.to_owned());
        manifest
    }

    pub fn with_preset(mut self, preset: YoloThemePreset) -> Self {
        self.theme_preset = preset;
        self
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_authors<I, S>(mut self, authors: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.authors = authors.into_iter().map(Into::into).collect();
        self
    }

    pub fn with_website(mut self, website: impl Into<String>) -> Self {
        self.website = Some(website.into());
        self
    }

    pub fn with_license(mut self, license: impl Into<String>) -> Self {
        self.license = Some(license.into());
        self
    }

    pub fn with_copyright(mut self, copyright: impl Into<String>) -> Self {
        self.copyright = Some(copyright.into());
        self
    }

    /// `(qualifier, organization)` derived from the reverse-DNS id.
    pub fn qualifier_organization(&self) -> (&str, &str) {
        let mut parts = self.id.split('.');
        let qualifier = parts.next().filter(|s| !s.is_empty()).unwrap_or("org");
        let organization = parts.next().filter(|s| !s.is_empty()).unwrap_or("yolo");
        (qualifier, organization)
    }

    /// File-system safe form of the id (used for sockets, mutexes, files).
    pub fn slug(&self) -> String {
        self.id
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '.' || c == '-' {
                    c
                } else {
                    '_'
                }
            })
            .collect()
    }

    /// Platform project directories (`directories::ProjectDirs`).
    pub fn project_dirs(&self) -> Result<ProjectDirs> {
        let (qualifier, organization) = self.qualifier_organization();
        ProjectDirs::from(qualifier, organization, &self.name).ok_or(Error::NoHomeDir)
    }

    /// Per-user configuration directory, created on demand.
    pub fn config_dir(&self) -> Result<PathBuf> {
        let dir = self.project_dirs()?.config_dir().to_path_buf();
        std::fs::create_dir_all(&dir)?;
        Ok(dir)
    }

    /// Per-user cache directory, created on demand.
    pub fn cache_dir(&self) -> Result<PathBuf> {
        let dir = self.project_dirs()?.cache_dir().to_path_buf();
        std::fs::create_dir_all(&dir)?;
        Ok(dir)
    }

    /// Per-user data directory, created on demand.
    pub fn data_dir(&self) -> Result<PathBuf> {
        let dir = self.project_dirs()?.data_dir().to_path_buf();
        std::fs::create_dir_all(&dir)?;
        Ok(dir)
    }

    /// Plain-text about box body, shared by every backend.
    pub fn about_text(&self) -> String {
        let mut out = format!("{} {}", self.name, self.version);
        if let Some(description) = &self.description {
            out.push_str("\n\n");
            out.push_str(description);
        }
        if !self.authors.is_empty() {
            out.push_str("\n\nAuthors: ");
            out.push_str(&self.authors.join(", "));
        }
        if let Some(copyright) = &self.copyright {
            out.push('\n');
            out.push_str(copyright);
        }
        if let Some(license) = &self.license {
            out.push_str("\nLicense: ");
            out.push_str(license);
        }
        if let Some(website) = &self.website {
            out.push('\n');
            out.push_str(website);
        }
        out
    }

    /// muda about metadata for the native About menu role.
    #[cfg(feature = "menu")]
    pub fn about_metadata(&self) -> muda::AboutMetadata {
        muda::AboutMetadata {
            name: Some(self.name.clone()),
            version: Some(self.version.clone()),
            authors: (!self.authors.is_empty()).then(|| self.authors.clone()),
            comments: self.description.clone(),
            copyright: self.copyright.clone(),
            license: self.license.clone(),
            website: self.website.clone(),
            ..Default::default()
        }
    }
}
