//! Explicit ordinary numbered-file concatenation, separate from archive readers.
use crate::{archive_job::valid_job_path, archive_read::ArchiveReadError};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const MAX_PARTS: usize = 1024;
pub const MAX_BYTES: u64 = 34_359_738_368;
pub const MAX_NAME_BYTES: usize = 262_144;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RawJoinAction {
    Inspect,
    Start,
    Cancel,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawNumberedJoinCommand {
    pub version: u32,
    pub operation_id: Uuid,
    pub session_id: Uuid,
    pub generation: u64,
    pub action: RawJoinAction,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preview_id: Option<Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_path: Option<String>,
}
impl std::fmt::Debug for RawNumberedJoinCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("RawNumberedJoinCommand([PRIVATE])")
    }
}
impl RawNumberedJoinCommand {
    pub fn validate(&self) -> Result<(), ArchiveReadError> {
        let shape = match self.action {
            RawJoinAction::Inspect => {
                self.selected_path.as_deref().is_some_and(valid_job_path)
                    && self.preview_id.is_none()
                    && self.output_path.is_none()
            }
            RawJoinAction::Start => {
                self.selected_path.is_none()
                    && self.preview_id.is_some_and(|id| !id.is_nil())
                    && self.output_path.as_deref().is_some_and(valid_job_path)
            }
            RawJoinAction::Cancel => {
                self.selected_path.is_none()
                    && self.preview_id.is_none()
                    && self.output_path.is_none()
            }
        };
        if self.version != 1
            || self.operation_id.is_nil()
            || self.session_id.is_nil()
            || self.generation == 0
            || !shape
        {
            return Err(ArchiveReadError::InvalidRequest);
        }
        Ok(())
    }
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawJoinPart {
    pub name: String,
    pub length: u64,
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawJoinPreview {
    pub preview_id: Uuid,
    pub suggested_output: String,
    pub parts: Vec<RawJoinPart>,
    pub total_bytes: u64,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawJoinTerminal {
    pub outcome: String,
    pub code: Option<String>,
    pub bytes_written: u64,
    pub cleanup_complete: bool,
    pub native_error_code: Option<i32>,
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawNumberedJoinReply {
    pub version: u32,
    pub operation_id: Uuid,
    pub session_id: Uuid,
    pub generation: u64,
    pub preview: Option<RawJoinPreview>,
    pub terminal: Option<RawJoinTerminal>,
}
impl std::fmt::Debug for RawNumberedJoinReply {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("RawNumberedJoinReply([PRIVATE])")
    }
}
impl RawNumberedJoinReply {
    pub fn validate(&self, command: &RawNumberedJoinCommand) -> Result<(), ArchiveReadError> {
        command.validate()?;
        let identity = self.version == command.version
            && self.operation_id == command.operation_id
            && self.session_id == command.session_id
            && self.generation == command.generation;
        let shape = match (&self.preview, &self.terminal) {
            (Some(preview), None) if command.action == RawJoinAction::Inspect => {
                !preview.preview_id.is_nil()
                    && valid_job_path(&preview.suggested_output)
                    && !preview.parts.is_empty()
                    && preview.parts.len() <= MAX_PARTS
                    && preview.total_bytes <= MAX_BYTES
                    && preview
                        .parts
                        .iter()
                        .try_fold(0u64, |sum, p| sum.checked_add(p.length))
                        == Some(preview.total_bytes)
                    && preview.parts.iter().map(|p| p.name.len()).sum::<usize>() <= MAX_NAME_BYTES
                    && preview.parts.iter().enumerate().all(|(index, p)| {
                        p.name.rsplit_once('.').is_some_and(|(stem, suffix)| {
                            !stem.is_empty() && suffix == format!("{:03}", index + 1)
                        }) && !p.name.contains(['/', '\\'])
                            && !p.name.chars().any(char::is_control)
                    })
            }
            (None, Some(t)) => {
                matches!(t.outcome.as_str(), "succeeded" | "failed" | "cancelled")
                    && t.bytes_written <= MAX_BYTES
                    && (t.outcome != "succeeded"
                        || command.action == RawJoinAction::Start
                            && t.code.is_none()
                            && t.cleanup_complete)
                    && t.code.as_ref().is_none_or(|c| {
                        c.len() <= 64 && c.bytes().all(|b| b.is_ascii_lowercase() || b == b'_')
                    })
            }
            _ => false,
        };
        if !identity || !shape {
            return Err(ArchiveReadError::InvalidRequest);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ref18_strict_shapes_identity_and_metadata_bounds() {
        let c = RawNumberedJoinCommand {
            version: 1,
            operation_id: Uuid::new_v4(),
            session_id: Uuid::new_v4(),
            generation: 3,
            action: RawJoinAction::Inspect,
            selected_path: Some(r"C:\owned\data.003".into()),
            preview_id: None,
            output_path: None,
        };
        c.validate().expect("inspect");
        assert!(!format!("{c:?}").contains("owned"));
        let mut value = serde_json::to_value(&c).expect("encode");
        value["overwrite"] = true.into();
        assert!(serde_json::from_value::<RawNumberedJoinCommand>(value).is_err());
        let mut reply = RawNumberedJoinReply {
            version: 1,
            operation_id: c.operation_id,
            session_id: c.session_id,
            generation: c.generation,
            preview: Some(RawJoinPreview {
                preview_id: Uuid::new_v4(),
                suggested_output: r"C:\owned\data".into(),
                parts: vec![RawJoinPart {
                    name: "data.001".into(),
                    length: 100,
                }],
                total_bytes: 100,
            }),
            terminal: None,
        };
        reply.validate(&c).expect("native preview");
        reply.preview.as_mut().expect("preview").parts[0].name = "data.003".into();
        assert!(reply.validate(&c).is_err());
        reply.preview.as_mut().expect("preview").parts[0].name = "data.001".into();
        reply.generation += 1;
        assert!(reply.validate(&c).is_err());
    }
}
