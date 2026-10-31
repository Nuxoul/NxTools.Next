//! Read-only destination observation. No output, source mutation or budget authority.
use crate::{archive_job::valid_job_path, archive_read::ArchiveReadError};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchiveSpaceCommand {
    pub version: u32,
    pub operation_id: Uuid,
    pub session_id: Uuid,
    pub generation: u64,
    pub source: String,
    pub destination: String,
    pub expected_source: Option<String>,
    pub expected_target: Option<String>,
}
fn identity(value: &str) -> bool {
    value.starts_with("v1:")
        && value.len() <= 256
        && value.is_ascii()
        && !value.chars().any(char::is_control)
}
impl ArchiveSpaceCommand {
    pub fn validate(&self) -> Result<(), ArchiveReadError> {
        if !matches!(self.version, 1 | 2) {
            return Err(ArchiveReadError::IncompatibleVersion);
        }
        if self.operation_id.is_nil()
            || self.session_id.is_nil()
            || self.generation == 0
            || !valid_job_path(&self.source)
            || !valid_job_path(&self.destination)
            || self
                .expected_source
                .as_deref()
                .is_some_and(|id| !identity(id))
            || self
                .expected_target
                .as_deref()
                .is_some_and(|id| !identity(id))
        {
            return Err(ArchiveReadError::InvalidRequest);
        }
        Ok(())
    }
}
/// Native read-only source facts, not execution/write authorization. Never log paths/identity.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchiveSpaceSourceSet {
    pub identity: String,
    pub primary: String,
    pub members: Vec<String>,
}
impl std::fmt::Debug for ArchiveSpaceSourceSet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ArchiveSpaceSourceSet([PRIVATE])")
    }
}
impl ArchiveSpaceSourceSet {
    fn valid(&self) -> bool {
        identity(&self.identity)
            && valid_job_path(&self.primary)
            && !self.members.is_empty()
            && self.members.len() <= 1024
            && self.members.iter().all(|p| valid_job_path(p))
            && self.members.contains(&self.primary)
            && self
                .members
                .iter()
                .try_fold(0usize, |n, p| n.checked_add(p.len()))
                .is_some_and(|n| n <= 1 << 20)
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchiveSpaceObservation {
    pub version: u32,
    pub operation_id: Uuid,
    pub session_id: Uuid,
    pub generation: u64,
    pub available_bytes: u64,
    pub allocation_bytes: u64,
    pub target_identity: String,
    pub volume_identity: String,
    pub destination_missing: bool,
    /// Held-native missing target components. Omission is unknown, never an inferred zero.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub missing_destination_directories: Option<u32>,
    pub source_matches: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_set: Option<ArchiveSpaceSourceSet>,
}
impl ArchiveSpaceObservation {
    pub fn validate(&self, command: &ArchiveSpaceCommand) -> Result<(), ArchiveReadError> {
        if self.version != command.version
            || self
                .source_set
                .as_ref()
                .is_some_and(|s| self.version != 2 || !s.valid())
            || self.source_set.as_ref().is_some_and(|s| {
                !s.members.contains(&command.source)
                    || s.members
                        .iter()
                        .collect::<std::collections::HashSet<_>>()
                        .len()
                        != s.members.len()
            })
            || self.operation_id != command.operation_id
            || self.session_id != command.session_id
            || self.generation != command.generation
            || !identity(&self.target_identity)
            || !identity(&self.volume_identity)
            || !self.allocation_bytes.is_power_of_two()
            || self.allocation_bytes > 16 << 20
            || self
                .missing_destination_directories
                .is_some_and(|count| count > 64 || (count != 0) != self.destination_missing)
        {
            return Err(ArchiveReadError::ProtocolViolation);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn hz3_v2_optional_source_set_is_bounded_legacy_and_request_bound() {
        let mut cmd = command();
        cmd.version = 2;
        assert!(
            cmd.validate().is_ok(),
            "v2 read-only observation must be admitted"
        );
        let data = serde_json::json!({"version":2,"operation_id":cmd.operation_id,"session_id":cmd.session_id,"generation":cmd.generation,"available_bytes":0,"allocation_bytes":4096,"target_identity":"v1:target","volume_identity":"v1:volume","destination_missing":false,"source_matches":true,"source_set":{"identity":"v1:source","primary":cmd.source,"members":[cmd.source]}});
        let observed: ArchiveSpaceObservation =
            serde_json::from_value(data.clone()).expect("v2 facts");
        assert!(observed.validate(&cmd).is_ok());
        for bad in [
            serde_json::json!({"identity":"bad","primary":cmd.source,"members":[cmd.source]}),
            serde_json::json!({"identity":"v1:x","primary":cmd.source,"members":[]}),
            serde_json::json!({"identity":"v1:x","primary":"relative","members":[cmd.source]}),
        ] {
            let mut value = data.clone();
            value["source_set"] = bad;
            let observed: ArchiveSpaceObservation =
                serde_json::from_value(value).expect("typed invalid facts");
            assert!(observed.validate(&cmd).is_err());
        }
        let mut legacy = data;
        legacy["version"] = serde_json::json!(1);
        legacy.as_object_mut().expect("object").remove("source_set");
        let observed: ArchiveSpaceObservation =
            serde_json::from_value(legacy).expect("legacy omission");
        cmd.version = 1;
        assert!(observed.validate(&cmd).is_ok());
    }
    use super::*;
    #[test]
    fn hz1_ac02_optional_target_count_is_strict_bounded_and_request_bound() {
        let command = command();
        let observation = ArchiveSpaceObservation {
            version: 1,
            operation_id: command.operation_id,
            session_id: command.session_id,
            generation: command.generation,
            available_bytes: 0,
            allocation_bytes: 4096,
            target_identity: "v1:opaque".into(),
            volume_identity: "v1:volume".into(),
            destination_missing: true,
            missing_destination_directories: Some(2),
            source_matches: true,
            source_set: None,
        };
        let original = serde_json::to_value(&observation).expect("wire");
        for count in [1, 2, 64] {
            let mut data = original.clone();
            data["missing_destination_directories"] = count.into();
            let decoded: ArchiveSpaceObservation =
                serde_json::from_value(data).expect("typed count");
            assert!(decoded.validate(&command).is_ok());
        }
        for count in [
            serde_json::json!(-1),
            serde_json::json!(4294967296u64),
            serde_json::json!("2"),
            serde_json::json!(1.5),
        ] {
            let mut data = original.clone();
            data["missing_destination_directories"] = count;
            assert!(serde_json::from_value::<ArchiveSpaceObservation>(data).is_err());
        }
        for (count, missing) in [(0, true), (65, true), (1, false)] {
            let mut data = observation.clone();
            data.missing_destination_directories = Some(count);
            data.destination_missing = missing;
            assert!(data.validate(&command).is_err());
        }
        let mut legacy = original.clone();
        legacy
            .as_object_mut()
            .expect("object")
            .remove("missing_destination_directories");
        let decoded: ArchiveSpaceObservation =
            serde_json::from_value(legacy.clone()).expect("old v1 result");
        assert_eq!(decoded.missing_destination_directories, None);
        assert!(decoded.validate(&command).is_ok());
        assert!(
            !serde_json::to_string(&decoded)
                .expect("legacy omission")
                .contains("missing_destination_directories")
        );
        let mut bad = original.clone();
        bad["unexpected_authority"] = true.into();
        assert!(serde_json::from_value::<ArchiveSpaceObservation>(bad).is_err());
        let mut stale = command.clone();
        stale.operation_id = Uuid::new_v4();
        assert!(observation.validate(&stale).is_err());
        let mut existing = observation;
        existing.destination_missing = false;
        existing.missing_destination_directories = Some(0);
        assert!(existing.validate(&command).is_ok());
    }
    use crate::{Envelope, Message, PlatformCommand};
    fn command() -> ArchiveSpaceCommand {
        ArchiveSpaceCommand {
            version: 1,
            operation_id: Uuid::new_v4(),
            session_id: Uuid::new_v4(),
            generation: 1,
            source: r"C:\owned\source.zip".into(),
            destination: r"D:\owned\target".into(),
            expected_source: Some("v1:source".into()),
            expected_target: None,
        }
    }
    #[test]
    fn hz1_real_command_roundtrip_has_no_budget_or_mutation_authority() {
        let command = command();
        assert!(command.validate().is_ok());
        let envelope = Envelope::new(Message::PlatformCommand(PlatformCommand::ArchiveSpace(
            command.clone(),
        )));
        let encoded = envelope.encode_line().expect("bounded command");
        let decoded = Envelope::decode_line(&encoded).expect("roundtrip");
        assert_eq!(decoded, envelope);
        assert!(
            encoded.contains("archive_space")
                && !encoded.contains("max_output")
                && !encoded.contains("password")
        );
        let mut wrong = command.clone();
        wrong.version = 3;
        assert_eq!(wrong.validate(), Err(ArchiveReadError::IncompatibleVersion));
        wrong = command.clone();
        wrong.destination = r"\\server\share\target".into();
        assert!(wrong.validate().is_err());
        wrong = command;
        wrong.expected_source = Some("v1:\nsecret".into());
        assert!(wrong.validate().is_err());
    }
    #[test]
    fn hz1_observation_requires_current_ids_and_real_bounded_allocation() {
        let command = command();
        let observation = ArchiveSpaceObservation {
            version: 1,
            operation_id: command.operation_id,
            session_id: command.session_id,
            generation: 1,
            available_bytes: u64::MAX,
            allocation_bytes: 4096,
            target_identity: "v1:target".into(),
            volume_identity: "v1:volume".into(),
            destination_missing: true,
            missing_destination_directories: Some(2),
            source_matches: true,
            source_set: None,
        };
        assert!(observation.validate(&command).is_ok());
        for allocation in [0, 3, 1 << 25] {
            let mut bad = observation.clone();
            bad.allocation_bytes = allocation;
            assert!(bad.validate(&command).is_err());
        }
        let mut bad = observation.clone();
        bad.session_id = Uuid::new_v4();
        assert!(bad.validate(&command).is_err());
        bad = observation;
        bad.generation = 2;
        assert!(bad.validate(&command).is_err());
        let value = serde_json::to_value(command).expect("wire");
        let mut value = value.as_object().expect("object").clone();
        value.insert("max_output_bytes".into(), 1.into());
        assert!(serde_json::from_value::<ArchiveSpaceCommand>(value.into()).is_err());
    }
}
