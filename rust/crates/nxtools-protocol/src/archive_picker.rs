//! Owned, bounded ordinary native dialogs; a selection grants no execution authority.
use crate::{
    archive_job::{valid_job_name, valid_job_path},
    archive_read::ArchiveReadError,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
/// Adapter availability; version-1 picker commands and complete-vector bounds are unchanged.
pub const ARCHIVE_MULTI_FOLDER_PICKER_CAPABILITY: &str = "archive_multi_folder_picker_v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArchivePickerPurpose {
    Files,
    Folders,
    Destination,
    Save,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchivePickerCommand {
    pub version: u32,
    pub operation_id: Uuid,
    pub session_id: Uuid,
    pub generation: u64,
    pub owner_window_handle: i64,
    pub purpose: ArchivePickerPurpose,
    pub current_directory: Option<String>,
    pub suggested_name: Option<String>,
}
impl ArchivePickerCommand {
    pub fn validate(&self) -> Result<(), ArchiveReadError> {
        if self.version != 1 {
            return Err(ArchiveReadError::IncompatibleVersion);
        }
        if self.operation_id.is_nil()
            || self.session_id.is_nil()
            || self.generation == 0
            || self.owner_window_handle == 0
            || self
                .current_directory
                .as_deref()
                .is_some_and(|p| !valid_job_path(p))
            || self
                .suggested_name
                .as_deref()
                .is_some_and(|n| self.purpose != ArchivePickerPurpose::Save || !valid_job_name(n))
        {
            return Err(ArchiveReadError::InvalidRequest);
        }
        Ok(())
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchivePickerResult {
    pub version: u32,
    pub operation_id: Uuid,
    pub session_id: Uuid,
    pub generation: u64,
    pub paths: Vec<String>,
    pub cancelled: bool,
    pub code: Option<String>,
    pub native_error_code: Option<i32>,
}
impl ArchivePickerResult {
    pub fn validate(&self) -> Result<(), ArchiveReadError> {
        if self.version != 1
            || self.operation_id.is_nil()
            || self.session_id.is_nil()
            || self.generation == 0
            || self.paths.len() > 32
            || self.paths.iter().any(|p| !valid_job_path(p))
            || self.paths.iter().map(String::len).sum::<usize>() > 131072
            || (self.cancelled || self.code.is_some()) && !self.paths.is_empty()
            || self.cancelled && self.code.is_some()
            || self.code.as_ref().is_some_and(|c| {
                !matches!(
                    c.as_str(),
                    "invalid_owner" | "picker_failed" | "picker_limit"
                )
            })
        {
            return Err(ArchiveReadError::ProtocolViolation);
        }
        Ok(())
    }
}
