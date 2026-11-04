// SPDX-License-Identifier: Apache-2.0
//! Shared document metadata, revision tracking, format capabilities, and autosave policy.
//!
//! This module deliberately leaves document bytes, codecs, filesystem access, and persistence to
//! the host workbench. It supplies the serializable state those services can share.

use serde::{Deserialize, Serialize};
use std::fmt;

use crate::workbenches::CraftWorkbenchContract;

/// Describes one document format and the operations supported by its host codec.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CraftDocumentFormat {
    /// Stable format identifier, such as `"org.yolo.craft.canvas+json"`.
    pub id: String,
    /// Optional canonical MIME type.
    pub mime_type: Option<String>,
    /// File extensions accepted by the codec, without leading dots.
    pub extensions: Vec<String>,
    /// Whether the host can import this format.
    pub can_import: bool,
    /// Whether the host can export this format.
    pub can_export: bool,
}

impl CraftDocumentFormat {
    /// Creates a format capability declaration.
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        mime_type: Option<impl Into<String>>,
        extensions: impl IntoIterator<Item = impl Into<String>>,
        can_import: bool,
        can_export: bool,
    ) -> Self {
        Self {
            id: id.into(),
            mime_type: mime_type.map(Into::into),
            extensions: extensions.into_iter().map(Into::into).collect(),
            can_import,
            can_export,
        }
    }

    /// Returns whether this profile accepts an extension, with or without a leading dot.
    #[must_use]
    pub fn supports_extension(&self, extension: &str) -> bool {
        let extension = extension.trim().trim_start_matches('.');
        !extension.is_empty()
            && self.extensions.iter().any(|candidate| {
                candidate
                    .trim_start_matches('.')
                    .eq_ignore_ascii_case(extension)
            })
    }

    /// Validates the stable identifier and the declared import/export routes.
    pub fn validate(&self) -> Result<(), CraftDocumentError> {
        if self.id.trim().is_empty() {
            return Err(CraftDocumentError::EmptyFormatId);
        }
        if self.extensions.iter().any(|extension| {
            let normalized = extension.trim().trim_start_matches('.');
            normalized.is_empty() || normalized.contains('/') || normalized.contains('\\')
        }) {
            return Err(CraftDocumentError::InvalidFormatExtension);
        }
        if !self.can_import && !self.can_export {
            return Err(CraftDocumentError::FormatHasNoIoOperation);
        }
        Ok(())
    }
}

/// Autosave behavior shared by document-capable workbenches.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutosavePolicy {
    /// Whether the host should schedule automatic saves.
    pub enabled: bool,
    /// Idle time after the latest revision before autosave becomes due.
    pub idle_delay_ms: u64,
}

impl Default for AutosavePolicy {
    fn default() -> Self {
        Self {
            enabled: true,
            idle_delay_ms: 1_000,
        }
    }
}

impl AutosavePolicy {
    /// Validates that an enabled autosave policy has a nonzero idle delay.
    pub fn validate(self) -> Result<(), CraftDocumentError> {
        if self.enabled && self.idle_delay_ms == 0 {
            return Err(CraftDocumentError::ZeroAutosaveDelay);
        }
        Ok(())
    }
}

/// One immutable entry in a document's revision chain.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CraftRevision {
    /// Monotonic, document-local revision number. The initial revision is zero.
    pub number: u64,
    /// Parent revision, absent only for revision zero.
    pub parent: Option<u64>,
    /// Host-provided Unix timestamp in milliseconds.
    pub created_at_ms: u64,
    /// Optional user-facing change summary.
    pub summary: Option<String>,
    /// Optional opaque digest of the serialized document content.
    pub content_digest: Option<String>,
}

/// Serializable document record shared by the Craft workbenches.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CraftDocument {
    /// Stable identifier supplied by the host or document store.
    pub id: String,
    /// Stable ID from [`CraftWorkbenchContract::id`].
    pub workbench_id: String,
    /// User-facing document title.
    pub title: String,
    /// Import/export capability profile for the document's native format.
    pub format: CraftDocumentFormat,
    /// Creation time supplied by the host, in Unix milliseconds.
    pub created_at_ms: u64,
    /// Timestamp of the latest revision, in Unix milliseconds.
    pub modified_at_ms: u64,
    /// Revision currently represented by the in-memory document.
    pub current_revision: u64,
    /// Latest revision known to have been persisted, if any.
    pub saved_revision: Option<u64>,
    /// Complete revision chain. Hosts may store revision payloads elsewhere.
    pub revisions: Vec<CraftRevision>,
    /// Host autosave scheduling policy.
    pub autosave: AutosavePolicy,
    /// Timestamp of the last successful save, if any.
    pub last_saved_at_ms: Option<u64>,
}

impl CraftDocument {
    /// Creates an unsaved document associated with a registered workbench.
    pub fn new(
        id: impl Into<String>,
        workbench: CraftWorkbenchContract,
        title: impl Into<String>,
        format: CraftDocumentFormat,
        created_at_ms: u64,
        autosave: AutosavePolicy,
    ) -> Result<Self, CraftDocumentError> {
        let id = id.into();
        let title = title.into();
        if id.trim().is_empty() {
            return Err(CraftDocumentError::EmptyDocumentId);
        }
        if title.trim().is_empty() {
            return Err(CraftDocumentError::EmptyTitle);
        }
        format.validate()?;
        autosave.validate()?;

        let document = Self {
            id,
            workbench_id: workbench.id.to_owned(),
            title,
            format,
            created_at_ms,
            modified_at_ms: created_at_ms,
            current_revision: 0,
            saved_revision: None,
            revisions: vec![CraftRevision {
                number: 0,
                parent: None,
                created_at_ms,
                summary: Some("Created".into()),
                content_digest: None,
            }],
            autosave,
            last_saved_at_ms: None,
        };
        document.validate()?;
        Ok(document)
    }

    /// Returns whether the current in-memory revision has not been saved.
    #[must_use]
    pub const fn is_dirty(&self) -> bool {
        !matches!(self.saved_revision, Some(revision) if revision == self.current_revision)
    }

    /// Returns the time at which autosave becomes due, if autosave is enabled and the document is
    /// dirty. `None` means autosave is disabled, the document is clean, or the timestamp overflows.
    #[must_use]
    pub fn autosave_due_at_ms(&self) -> Option<u64> {
        if !self.autosave.enabled || !self.is_dirty() {
            return None;
        }
        self.modified_at_ms.checked_add(self.autosave.idle_delay_ms)
    }

    /// Returns whether an enabled autosave should run at the supplied host clock time.
    #[must_use]
    pub fn autosave_due(&self, now_ms: u64) -> bool {
        self.autosave_due_at_ms()
            .is_some_and(|deadline| now_ms >= deadline)
    }

    /// Appends a new in-memory revision and marks the document dirty.
    pub fn record_revision(
        &mut self,
        created_at_ms: u64,
        summary: Option<String>,
        content_digest: Option<String>,
    ) -> Result<u64, CraftDocumentError> {
        if created_at_ms < self.modified_at_ms {
            return Err(CraftDocumentError::NonMonotonicRevisionTime);
        }
        let number = self
            .current_revision
            .checked_add(1)
            .ok_or(CraftDocumentError::RevisionNumberOverflow)?;
        self.revisions.push(CraftRevision {
            number,
            parent: Some(self.current_revision),
            created_at_ms,
            summary,
            content_digest,
        });
        self.current_revision = number;
        self.modified_at_ms = created_at_ms;
        Ok(number)
    }

    /// Marks a revision as persisted. Saving an older snapshot is allowed; a newer in-memory
    /// revision will remain dirty while an asynchronous save completes.
    pub fn mark_saved(
        &mut self,
        revision: u64,
        saved_at_ms: u64,
    ) -> Result<(), CraftDocumentError> {
        if !self.revisions.iter().any(|entry| entry.number == revision) {
            return Err(CraftDocumentError::UnknownSavedRevision);
        }
        if self
            .last_saved_at_ms
            .is_some_and(|previous| saved_at_ms < previous)
        {
            return Err(CraftDocumentError::NonMonotonicSaveTime);
        }
        self.saved_revision = Some(revision);
        self.last_saved_at_ms = Some(saved_at_ms);
        Ok(())
    }

    /// Validates metadata invariants, including a contiguous revision chain and valid save marker.
    pub fn validate(&self) -> Result<(), CraftDocumentError> {
        if self.id.trim().is_empty() {
            return Err(CraftDocumentError::EmptyDocumentId);
        }
        if self.workbench_id.trim().is_empty() {
            return Err(CraftDocumentError::EmptyWorkbenchId);
        }
        if self.title.trim().is_empty() {
            return Err(CraftDocumentError::EmptyTitle);
        }
        self.format.validate()?;
        self.autosave.validate()?;
        if self.modified_at_ms < self.created_at_ms || self.revisions.is_empty() {
            return Err(CraftDocumentError::InvalidRevisionHistory);
        }

        for (index, revision) in self.revisions.iter().enumerate() {
            let expected =
                u64::try_from(index).map_err(|_| CraftDocumentError::RevisionNumberOverflow)?;
            let expected_parent = expected.checked_sub(1);
            if revision.number != expected || revision.parent != expected_parent {
                return Err(CraftDocumentError::InvalidRevisionHistory);
            }
            if index > 0 && revision.created_at_ms < self.revisions[index - 1].created_at_ms {
                return Err(CraftDocumentError::InvalidRevisionHistory);
            }
        }
        if self.revisions.last().map(|revision| revision.number) != Some(self.current_revision)
            || self.revisions.last().map(|revision| revision.created_at_ms)
                != Some(self.modified_at_ms)
        {
            return Err(CraftDocumentError::InvalidRevisionHistory);
        }
        if self.saved_revision.is_some_and(|saved| {
            saved > self.current_revision
                || !self
                    .revisions
                    .iter()
                    .any(|revision| revision.number == saved)
        }) {
            return Err(CraftDocumentError::InvalidSavedRevision);
        }
        if self.last_saved_at_ms.is_some() != self.saved_revision.is_some() {
            return Err(CraftDocumentError::InvalidSavedRevision);
        }
        Ok(())
    }
}

/// Errors raised when creating or updating shared Craft document metadata.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CraftDocumentError {
    /// The document ID was empty.
    EmptyDocumentId,
    /// The workbench ID was empty.
    EmptyWorkbenchId,
    /// The title was empty.
    EmptyTitle,
    /// The document format ID was empty.
    EmptyFormatId,
    /// An extension entry was empty or contained a second extension separator.
    InvalidFormatExtension,
    /// A format profile declared neither import nor export support.
    FormatHasNoIoOperation,
    /// Enabled autosave was configured with a zero delay.
    ZeroAutosaveDelay,
    /// Revision metadata was empty, noncontiguous, or inconsistent.
    InvalidRevisionHistory,
    /// The saved revision was absent or greater than the current revision.
    InvalidSavedRevision,
    /// A save operation referred to a revision not retained by this document.
    UnknownSavedRevision,
    /// A revision timestamp preceded the latest revision timestamp.
    NonMonotonicRevisionTime,
    /// A save timestamp preceded the previous successful save.
    NonMonotonicSaveTime,
    /// The revision counter reached its maximum value.
    RevisionNumberOverflow,
}

impl fmt::Display for CraftDocumentError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::EmptyDocumentId => "document ID must not be empty",
            Self::EmptyWorkbenchId => "workbench ID must not be empty",
            Self::EmptyTitle => "document title must not be empty",
            Self::EmptyFormatId => "document format ID must not be empty",
            Self::InvalidFormatExtension => "document format contains an invalid extension",
            Self::FormatHasNoIoOperation => "document format supports neither import nor export",
            Self::ZeroAutosaveDelay => "enabled autosave requires a nonzero delay",
            Self::InvalidRevisionHistory => "document revision history is inconsistent",
            Self::InvalidSavedRevision => "document saved revision is inconsistent",
            Self::UnknownSavedRevision => "saved revision is not present in document history",
            Self::NonMonotonicRevisionTime => {
                "revision time precedes the current document revision"
            }
            Self::NonMonotonicSaveTime => "save time precedes the previous successful save",
            Self::RevisionNumberOverflow => "document revision number overflowed",
        })
    }
}

impl std::error::Error for CraftDocumentError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workbenches::workbench_by_id;

    fn document() -> CraftDocument {
        CraftDocument::new(
            "doc-1",
            workbench_by_id("wordcraft").expect("registered workbench"),
            "Draft",
            CraftDocumentFormat::new(
                "org.yolo.craft.document+json",
                Some("application/json"),
                ["yolo", "json"],
                true,
                true,
            ),
            100,
            AutosavePolicy::default(),
        )
        .expect("valid new document")
    }

    #[test]
    fn revisions_track_dirty_and_autosave_state() {
        let mut document = document();
        assert!(document.is_dirty());
        assert_eq!(document.autosave_due_at_ms(), Some(1_100));
        assert!(!document.autosave_due(1_099));
        assert!(document.autosave_due(1_100));

        let revision = document
            .record_revision(200, Some("Edited title".into()), None)
            .expect("append revision");
        assert_eq!(revision, 1);
        document
            .mark_saved(revision, 250)
            .expect("save current revision");
        assert!(!document.is_dirty());
        document
            .record_revision(300, None, None)
            .expect("append next revision");
        assert!(document.is_dirty());
        assert_eq!(document.saved_revision, Some(1));
    }

    #[test]
    fn format_profiles_route_case_insensitive_extensions() {
        let format = document().format;
        assert!(format.supports_extension(".JSON"));
        assert!(!format.supports_extension(""));
        assert!(format.validate().is_ok());
    }

    #[cfg(feature = "serde")]
    #[test]
    fn document_metadata_round_trips_through_json() {
        let document = document();
        let encoded = serde_json::to_string(&document).expect("serialize document metadata");
        let decoded: CraftDocument =
            serde_json::from_str(&encoded).expect("deserialize document metadata");
        assert_eq!(decoded, document);
        decoded
            .validate()
            .expect("round-tripped document stays valid");
    }
}
