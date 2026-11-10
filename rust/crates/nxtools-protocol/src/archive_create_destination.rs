//! Read-only destination namespace observation for complete create-output planning.
use crate::{archive_job::valid_job_path, archive_read::ArchiveReadError};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const ARCHIVE_CREATE_DESTINATION_VERSION: u32 = 1;
pub const MAX_EXISTING_NAMES: usize = 16_384;
pub const MAX_EXISTING_NAME_BYTES: usize = 262_144;

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchiveCreateDestinationCommand {
    pub version: u32,
    pub operation_id: Uuid,
    pub session_id: Uuid,
    pub generation: u64,
    pub destination: String,
}

impl std::fmt::Debug for ArchiveCreateDestinationCommand {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("ArchiveCreateDestinationCommand([PRIVATE])")
    }
}

impl ArchiveCreateDestinationCommand {
    pub fn validate(&self) -> Result<(), ArchiveReadError> {
        if self.version != ARCHIVE_CREATE_DESTINATION_VERSION {
            return Err(ArchiveReadError::IncompatibleVersion);
        }
        if self.operation_id.is_nil()
            || self.session_id.is_nil()
            || self.generation == 0
            || !valid_job_path(&self.destination)
        {
            return Err(ArchiveReadError::InvalidRequest);
        }
        Ok(())
    }
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchiveCreateDestinationResult {
    pub version: u32,
    pub operation_id: Uuid,
    pub session_id: Uuid,
    pub generation: u64,
    pub destination_missing: bool,
    pub existing_names: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub native_error_code: Option<i32>,
}

impl std::fmt::Debug for ArchiveCreateDestinationResult {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("ArchiveCreateDestinationResult([PRIVATE])")
    }
}

impl ArchiveCreateDestinationResult {
    pub fn validate(
        &self,
        command: &ArchiveCreateDestinationCommand,
    ) -> Result<(), ArchiveReadError> {
        command.validate()?;
        let bytes = self
            .existing_names
            .iter()
            .try_fold(0usize, |total, name| total.checked_add(name.len()));
        if self.version != command.version
            || self.operation_id != command.operation_id
            || self.session_id != command.session_id
            || self.generation != command.generation
            || self.existing_names.len() > MAX_EXISTING_NAMES
            || bytes.is_none_or(|bytes| bytes > MAX_EXISTING_NAME_BYTES)
            || self.destination_missing && !self.existing_names.is_empty()
            || self.code.as_deref().is_some_and(|code| {
                !matches!(
                    code,
                    "destination_unavailable"
                        | "destination_limit"
                        | "cancelled"
                        | "invalid_request"
                )
            })
            || self.code.is_some() && (self.destination_missing || !self.existing_names.is_empty())
            || self.code.is_none() && self.native_error_code.is_some()
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
    fn opt03_destination_wire_matches_native_contract_and_bounds_names() {
        let command = ArchiveCreateDestinationCommand {
            version: 1,
            operation_id: Uuid::new_v4(),
            session_id: Uuid::new_v4(),
            generation: 7,
            destination: r"C:\owned\output".into(),
        };
        command.validate().expect("valid command");
        let wire = Envelope::new(Message::PlatformCommand(
            PlatformCommand::ArchiveCreateDestination(command.clone()),
        ));
        let encoded = serde_json::to_string(&wire).expect("command wire");
        assert!(encoded.contains("archive_create_destination"));
        assert!(!format!("{command:?}").contains("owned"));
        let _: Envelope = serde_json::from_str(&encoded).expect("command roundtrip");

        let mut result: ArchiveCreateDestinationResult =
            serde_json::from_value(serde_json::json!({
                "version": 1,
                "operation_id": command.operation_id,
                "session_id": command.session_id,
                "generation": command.generation,
                "destination_missing": false,
                "existing_names": ["report.zip", "report (2).zip"]
            }))
            .expect("native result shape");
        result.validate(&command).expect("bounded success");
        assert!(!format!("{result:?}").contains("report.zip"));

        result.destination_missing = true;
        assert!(result.validate(&command).is_err());
        result.existing_names.clear();
        result.validate(&command).expect("missing destination");
        result.code = Some("destination_limit".into());
        result.destination_missing = false;
        result.validate(&command).expect("bounded native failure");
        result.existing_names = vec!["x".into(); MAX_EXISTING_NAMES + 1];
        assert!(result.validate(&command).is_err());
    }
}
