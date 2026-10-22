//! Native output capability: Rust chooses ordering, Windows adapter owns actual handles.
use crate::{ArchiveSnapshot, Result, planning::ExtractionPlan};
use uuid::Uuid;

/// Implementations must use held-directory/handle-relative exclusive partials and native
/// no-reparse commit. A plain path-joining filesystem writer is not a safe Windows adapter.
pub trait ArchiveOutput {
    fn ensure_directory(&mut self, relative: &str) -> Result<()>;
    fn begin_file(&mut self, relative: &str) -> Result<Uuid>;
    /// Conflict-aware extraction admission; default preserves existing no-replace adapters.
    fn begin_extracted_file(
        &mut self,
        relative: &str,
        _incoming: Option<u64>,
    ) -> Result<Option<Uuid>> {
        self.begin_file(relative).map(Some)
    }
    fn skipped_files(&self) -> u64 {
        0
    }
    /// ZIP warning before current-member decode/partial. No implicit approval without a caller.
    fn confirm_zip_entry(
        &mut self,
        entry: &crate::ArchiveEntry,
        _limits: &crate::ResourceLimits,
    ) -> Result<()> {
        if crate::zip_risk::required(entry) {
            Err(crate::ArchiveError::ResourceLimit)
        } else {
            Ok(())
        }
    }
    fn write_file(&mut self, token: Uuid, bytes: &[u8]) -> Result<()>;
    /// Stage immutable metadata for this exact unpublished token. Unsupported adapters fail closed.
    fn set_file_metadata(
        &mut self,
        _token: Uuid,
        _metadata: crate::metadata::OutputMetadata,
    ) -> Result<()> {
        Err(crate::ArchiveError::UnsupportedOperation)
    }
    fn finalize_directory_metadata(
        &mut self,
        _relative: &str,
        _metadata: crate::metadata::OutputMetadata,
    ) -> Result<()> {
        Err(crate::ArchiveError::UnsupportedOperation)
    }
    fn commit_file(&mut self, token: Uuid) -> Result<()>;
    /// Best-effort cleanup of this exact owned partial. Never delete a final file by path.
    fn abort_file(&mut self, token: Uuid);
}
pub struct ExtractRequest {
    pub(crate) snapshot: ArchiveSnapshot,
    pub(crate) plan: ExtractionPlan,
}
impl ExtractRequest {
    pub fn new(snapshot: ArchiveSnapshot, plan: ExtractionPlan) -> Self {
        Self { snapshot, plan }
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExtractOutcome {
    pub files_committed: u64,
    pub bytes_committed: u64,
    pub skipped_special: u64,
}
