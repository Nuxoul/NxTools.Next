//! On-demand file-list intake. Display/input data, never source deletion authority.
use crate::{archive_job::valid_job_path, archive_read::ArchiveReadError};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchiveClipboardCommand {
    pub version: u32,
    pub operation_id: Uuid,
    pub session_id: Uuid,
    pub generation: u64,
}
impl ArchiveClipboardCommand {
    pub fn validate(&self) -> Result<(), ArchiveReadError> {
        if self.version != 1 {
            return Err(ArchiveReadError::IncompatibleVersion);
        }
        if self.operation_id.is_nil() || self.session_id.is_nil() || self.generation == 0 {
            return Err(ArchiveReadError::InvalidRequest);
        }
        Ok(())
    }
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchiveClipboardResult {
    pub version: u32,
    pub operation_id: Uuid,
    pub session_id: Uuid,
    pub generation: u64,
    pub paths: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub native_error_code: Option<i32>,
}
impl std::fmt::Debug for ArchiveClipboardResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ArchiveClipboardResult([PRIVATE])")
    }
}
impl ArchiveClipboardResult {
    pub fn validate(&self, command: &ArchiveClipboardCommand) -> Result<(), ArchiveReadError> {
        command.validate()?;
        if self.version != command.version
            || self.operation_id != command.operation_id
            || self.session_id != command.session_id
            || self.generation != command.generation
            || self.paths.len() > 32
            || self.paths.iter().any(|path| !valid_job_path(path))
            || self
                .paths
                .iter()
                .try_fold(0usize, |size, path| size.checked_add(path.len()))
                .is_none_or(|size| size > 131_072)
            || self.code.as_deref().is_some_and(|code| {
                !matches!(
                    code,
                    "clipboard_unavailable"
                        | "invalid_clipboard_data"
                        | "clipboard_limit"
                        | "clipboard_failed"
                        | "cancelled"
                )
            })
            || (self.code.is_some() && !self.paths.is_empty())
            || (self.code.is_none() && self.native_error_code.is_some())
        {
            return Err(ArchiveReadError::InvalidRequest);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Envelope, Message, PlatformCommand};
    #[test]
    fn opt03_clipboard_wire_is_bounded_atomic_private_and_correlated() {
        let command = ArchiveClipboardCommand {
            version: 1,
            operation_id: Uuid::new_v4(),
            session_id: Uuid::new_v4(),
            generation: 3,
        };
        let wire = Envelope::new(Message::PlatformCommand(
            PlatformCommand::ReadClipboardFileDrop(command.clone()),
        ));
        let encoded = serde_json::to_string(&wire).expect("command wire");
        assert!(encoded.contains("read_clipboard_file_drop"));
        let _: Envelope = serde_json::from_str(&encoded).expect("command roundtrip");
        let mut result: ArchiveClipboardResult = serde_json::from_value(serde_json::json!({
            "version": 1, "operation_id": command.operation_id, "session_id": command.session_id,
            "generation": 3, "paths": ["C:\\private\\input.txt"]
        }))
        .expect("omitted optional fields");
        assert!(result.validate(&command).is_ok());
        assert!(!format!("{result:?}").contains("input.txt"));
        result.paths = vec!["C:\\a.txt".into(); 33];
        assert!(result.validate(&command).is_err());
        result.paths.clear();
        assert!(
            result.validate(&command).is_ok(),
            "empty HDROP is an empty success"
        );
        result.code = Some("clipboard_limit".into());
        assert!(result.validate(&command).is_ok());
        result.paths.push("C:\\partial.txt".into());
        assert!(
            result.validate(&command).is_err(),
            "failed batch cannot retain partial input"
        );
        result.paths.clear();
        result.generation += 1;
        assert!(result.validate(&command).is_err());
    }
}
