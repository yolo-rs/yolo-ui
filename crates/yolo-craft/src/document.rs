// SPDX-License-Identifier: Apache-2.0
//! Shared document metadata, revision tracking, format capabilities, and autosave policy.
//!
//! This module deliberately leaves document bytes, codecs, filesystem access, and persistence to
//! the host workbench. It supplies the serializable state those services can share.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

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
        if saved_at_ms < self.created_at_ms
            || self
                .last_saved_at_ms
                .is_some_and(|previous| saved_at_ms < previous)
        {
            return Err(CraftDocumentError::NonMonotonicSaveTime);
        }
        self.saved_revision = Some(revision);
        self.last_saved_at_ms = Some(saved_at_ms);
        Ok(())
    }

    /// Saves the current immutable revision payload and advances this document's save marker only
    /// after the store accepts it. Reusing a revision with different bytes is rejected by stores.
    pub fn save_current_revision<S: CraftRevisionStore>(
        &mut self,
        store: &mut S,
        content: &[u8],
        saved_at_ms: u64,
    ) -> Result<u64, CraftSaveError> {
        self.validate().map_err(CraftSaveError::InvalidDocument)?;
        let revision = self.current_revision;
        let mut saved_document = self.clone();
        saved_document
            .mark_saved(revision, saved_at_ms)
            .map_err(CraftSaveError::InvalidDocument)?;
        saved_document
            .validate()
            .map_err(CraftSaveError::InvalidDocument)?;

        store
            .save_revision(&self.id, revision, content)
            .map_err(CraftSaveError::Store)?;
        *self = saved_document;
        Ok(revision)
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

/// Stores immutable serialized payloads independently from codecs, schedulers, and document UI.
///
/// Implementations must make repeated writes of the same `(document_id, revision)` and identical
/// bytes idempotent, and reject a different payload for an already stored revision.
pub trait CraftRevisionStore {
    /// Stores one immutable revision payload.
    fn save_revision(
        &mut self,
        document_id: &str,
        revision: u64,
        content: &[u8],
    ) -> Result<(), CraftRevisionStoreError>;

    /// Loads one revision payload, returning `None` when it has not been saved.
    fn load_revision(
        &self,
        document_id: &str,
        revision: u64,
    ) -> Result<Option<Vec<u8>>, CraftRevisionStoreError>;
}

/// In-memory revision store for previews, tests, and ephemeral workbenches.
#[derive(Clone, Debug, Default)]
pub struct MemoryCraftRevisionStore {
    revisions: HashMap<(String, u64), Vec<u8>>,
}

impl CraftRevisionStore for MemoryCraftRevisionStore {
    fn save_revision(
        &mut self,
        document_id: &str,
        revision: u64,
        content: &[u8],
    ) -> Result<(), CraftRevisionStoreError> {
        validate_store_document_id(document_id)?;
        let key = (document_id.to_owned(), revision);
        if let Some(previous) = self.revisions.get(&key) {
            return if previous == content {
                Ok(())
            } else {
                Err(CraftRevisionStoreError::RevisionConflict)
            };
        }
        self.revisions.insert(key, content.to_vec());
        Ok(())
    }

    fn load_revision(
        &self,
        document_id: &str,
        revision: u64,
    ) -> Result<Option<Vec<u8>>, CraftRevisionStoreError> {
        validate_store_document_id(document_id)?;
        Ok(self
            .revisions
            .get(&(document_id.to_owned(), revision))
            .cloned())
    }
}

/// Portable filesystem-backed immutable revision payload store.
///
/// Document IDs are hex-encoded into path components, so caller-provided IDs cannot traverse out
/// of `root`. IDs longer than 64 UTF-8 bytes are rejected to stay within common path limits.
#[derive(Clone, Debug)]
pub struct FileCraftRevisionStore {
    root: PathBuf,
}

impl FileCraftRevisionStore {
    /// Creates a store rooted at `root`. Directories are created on the first write.
    #[must_use]
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    fn revision_path(&self, document_id: &str, revision: u64) -> PathBuf {
        self.root
            .join(hex_encode(document_id.as_bytes()))
            .join(format!("revision-{revision}.bin"))
    }
}

static TEMP_FILE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

impl CraftRevisionStore for FileCraftRevisionStore {
    fn save_revision(
        &mut self,
        document_id: &str,
        revision: u64,
        content: &[u8],
    ) -> Result<(), CraftRevisionStoreError> {
        validate_store_document_id(document_id)?;
        let destination = self.revision_path(document_id, revision);
        if destination.exists() {
            return compare_existing_revision(&destination, content);
        }
        let parent = destination
            .parent()
            .ok_or(CraftRevisionStoreError::InvalidStoreRoot)?;
        fs::create_dir_all(parent).map_err(CraftRevisionStoreError::Io)?;

        let sequence = TEMP_FILE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let temporary = parent.join(format!(
            ".revision-{revision}-{}-{sequence}.tmp",
            std::process::id()
        ));
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(CraftRevisionStoreError::Io)?;
        let write_result = file.write_all(content).and_then(|()| file.sync_all());
        drop(file);
        if let Err(error) = write_result {
            let _ = fs::remove_file(&temporary);
            return Err(CraftRevisionStoreError::Io(error));
        }

        match fs::rename(&temporary, &destination) {
            Ok(()) => Ok(()),
            Err(_error) if destination.exists() => {
                let _ = fs::remove_file(&temporary);
                compare_existing_revision(&destination, content)
            }
            Err(error) => {
                let _ = fs::remove_file(&temporary);
                Err(CraftRevisionStoreError::Io(error))
            }
        }
    }

    fn load_revision(
        &self,
        document_id: &str,
        revision: u64,
    ) -> Result<Option<Vec<u8>>, CraftRevisionStoreError> {
        validate_store_document_id(document_id)?;
        let path = self.revision_path(document_id, revision);
        let mut file = match OpenOptions::new().read(true).open(path) {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(CraftRevisionStoreError::Io(error)),
        };
        let mut content = Vec::new();
        file.read_to_end(&mut content)
            .map_err(CraftRevisionStoreError::Io)?;
        Ok(Some(content))
    }
}

fn compare_existing_revision(path: &Path, content: &[u8]) -> Result<(), CraftRevisionStoreError> {
    let mut existing = Vec::new();
    OpenOptions::new()
        .read(true)
        .open(path)
        .and_then(|mut file| file.read_to_end(&mut existing))
        .map_err(CraftRevisionStoreError::Io)?;
    if existing == content {
        Ok(())
    } else {
        Err(CraftRevisionStoreError::RevisionConflict)
    }
}

fn validate_store_document_id(document_id: &str) -> Result<(), CraftRevisionStoreError> {
    if document_id.trim().is_empty() || document_id.len() > 64 {
        return Err(CraftRevisionStoreError::InvalidDocumentId);
    }
    Ok(())
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(char::from(HEX[usize::from(byte >> 4)]));
        encoded.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    encoded
}

/// Failures from immutable revision payload stores.
#[derive(Debug)]
pub enum CraftRevisionStoreError {
    /// The ID is empty or too long for a portable filesystem path component.
    InvalidDocumentId,
    /// A revision already exists with different payload bytes.
    RevisionConflict,
    /// A filesystem store root could not be represented by a parent path.
    InvalidStoreRoot,
    /// Filesystem access failed.
    Io(io::Error),
}

impl fmt::Display for CraftRevisionStoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDocumentId => formatter
                .write_str("document ID must contain 1 to 64 UTF-8 bytes for revision storage"),
            Self::RevisionConflict => {
                formatter.write_str("revision already exists with different content")
            }
            Self::InvalidStoreRoot => formatter.write_str("revision store root is invalid"),
            Self::Io(error) => write!(formatter, "revision store I/O failed: {error}"),
        }
    }
}

impl std::error::Error for CraftRevisionStoreError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            _ => None,
        }
    }
}

/// Failures while persisting a document's current revision.
#[derive(Debug)]
pub enum CraftSaveError {
    /// The document could not be marked saved at the supplied timestamp.
    InvalidDocument(CraftDocumentError),
    /// The revision payload could not be stored.
    Store(CraftRevisionStoreError),
}

impl fmt::Display for CraftSaveError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDocument(error) => write!(formatter, "invalid document save: {error}"),
            Self::Store(error) => write!(formatter, "document payload save failed: {error}"),
        }
    }
}

impl std::error::Error for CraftSaveError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InvalidDocument(error) => Some(error),
            Self::Store(error) => Some(error),
        }
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
            Self::NonMonotonicSaveTime => {
                "save time precedes document creation or the previous successful save"
            }
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
    fn saving_is_revisioned_idempotent_and_marks_clean_only_after_store_success() {
        let mut document = document();
        let mut store = MemoryCraftRevisionStore::default();

        assert_eq!(
            document
                .save_current_revision(&mut store, b"initial", 120)
                .expect("save initial revision"),
            0
        );
        assert!(!document.is_dirty());
        assert_eq!(
            store.load_revision("doc-1", 0).unwrap(),
            Some(b"initial".to_vec())
        );

        assert!(matches!(
            document.save_current_revision(&mut store, b"different bytes", 121),
            Err(CraftSaveError::Store(
                CraftRevisionStoreError::RevisionConflict
            ))
        ));
        assert!(!document.is_dirty());

        document
            .record_revision(200, Some("Second edit".into()), None)
            .expect("append revision");
        assert!(matches!(
            document.save_current_revision(&mut store, b"revision one", 119),
            Err(CraftSaveError::InvalidDocument(
                CraftDocumentError::NonMonotonicSaveTime
            ))
        ));
        assert!(document.is_dirty());
        assert_eq!(store.load_revision("doc-1", 1).unwrap(), None);
    }

    #[test]
    fn file_store_round_trips_payload_without_using_document_id_as_a_path() {
        let root = std::env::temp_dir().join(format!(
            "yolo-craft-revision-store-{}-{}",
            std::process::id(),
            TEMP_FILE_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        let mut store = FileCraftRevisionStore::new(&root);
        store
            .save_revision("../nested/doc", 7, b"payload")
            .expect("save revision");
        store
            .save_revision("../nested/doc", 7, b"payload")
            .expect("identical revision save is idempotent");
        assert_eq!(
            store.load_revision("../nested/doc", 7).unwrap(),
            Some(b"payload".to_vec())
        );
        assert!(matches!(
            store.save_revision("../nested/doc", 7, b"conflict"),
            Err(CraftRevisionStoreError::RevisionConflict)
        ));
        assert!(matches!(
            store.load_revision("", 7),
            Err(CraftRevisionStoreError::InvalidDocumentId)
        ));
        fs::remove_dir_all(root).expect("remove temporary revision store");
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
