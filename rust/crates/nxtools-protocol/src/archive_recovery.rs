//! Read-only recovery record discovery. This descriptor grants no execution authority.
use crate::{archive_job::valid_job_path, archive_read::ArchiveReadError};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

fn valid_path(path: &str) -> bool {
    valid_job_path(path)
        && path[3..].split('\\').count() <= 64
        && path[3..].split('\\').all(|part| {
            (1..=255).contains(&part.encode_utf16().count())
                && part != "."
                && part != ".."
                && !part.ends_with(['.', ' '])
                && !part
                    .chars()
                    .any(|ch| ch.is_control() || ":<>\"|?*".contains(ch))
        })
}

fn valid_source_identity(identity: &str) -> bool {
    let parts: Vec<_> = identity.split(':').collect();
    parts.len() == 4
        && parts[0] == "v1"
        && (1..=16).contains(&parts[1].len())
        && parts[2].len() == 32
        && (1..=16).contains(&parts[3].len())
        && parts[1..]
            .iter()
            .all(|part| part.bytes().all(|b| b.is_ascii_hexdigit()))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchiveRecoveryInspectCommand {
    pub version: u32,
    pub operation_id: Uuid,
    pub session_id: Uuid,
    pub generation: u64,
    pub source: String,
    pub destination: Option<String>,
}
impl ArchiveRecoveryInspectCommand {
    pub fn validate(&self) -> Result<(), ArchiveReadError> {
        if self.version != 1 {
            return Err(ArchiveReadError::IncompatibleVersion);
        }
        if self.operation_id.is_nil()
            || self.session_id.is_nil()
            || self.generation == 0
            || !valid_path(&self.source)
            || self.destination.as_deref().is_some_and(|s| !valid_path(s))
        {
            return Err(ArchiveReadError::InvalidRequest);
        }
        Ok(())
    }
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchiveRecoveryInspection {
    pub version: u32,
    pub operation_id: Uuid,
    pub session_id: Uuid,
    pub generation: u64,
    pub recovery_id: Uuid,
    pub source_identity: String,
    pub destination: Option<String>,
    pub copy_motw: bool,
    pub completed: bool,
}
impl std::fmt::Debug for ArchiveRecoveryInspection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ArchiveRecoveryInspection([PRIVATE])")
    }
}
impl ArchiveRecoveryInspection {
    pub fn validate(
        &self,
        command: &ArchiveRecoveryInspectCommand,
    ) -> Result<(), ArchiveReadError> {
        let identity = &self.source_identity;
        if command.validate().is_err()
            || self.version != 1
            || self.operation_id != command.operation_id
            || self.session_id != command.session_id
            || self.generation != command.generation
            || self.recovery_id.is_nil()
            || !valid_source_identity(identity)
            || self.destination.as_deref().is_some_and(|path| {
                !valid_path(path)
                    || match command.destination.as_deref() {
                        Some(candidate) => path != candidate,
                        None => {
                            path != command
                                .source
                                .rsplit_once('\\')
                                .map(|(parent, _)| parent)
                                .unwrap_or_default()
                        }
                    }
            })
        {
            return Err(ArchiveReadError::ProtocolViolation);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn discovery_wire_is_strict_and_request_bound() {
        let command = ArchiveRecoveryInspectCommand {
            version: 1,
            operation_id: Uuid::new_v4(),
            session_id: Uuid::new_v4(),
            generation: 1,
            source: r"C:\scratch\source.zip".into(),
            destination: Some(r"C:\scratch\original".into()),
        };
        assert!(command.validate().is_ok());
        let encoded = serde_json::to_value(crate::PlatformCommand::ArchiveRecoveryInspect(
            command.clone(),
        ))
        .expect("encode");
        assert_eq!(encoded["command"], "archive_recovery_inspect");
        let decoded: crate::PlatformCommand = serde_json::from_value(encoded).expect("decode");
        assert!(matches!(
            decoded,
            crate::PlatformCommand::ArchiveRecoveryInspect(_)
        ));
        let response = ArchiveRecoveryInspection {
            version: 1,
            operation_id: command.operation_id,
            session_id: command.session_id,
            generation: command.generation,
            recovery_id: Uuid::new_v4(),
            source_identity: "v1:1:00112233445566778899AABBCCDDEEFF:4".into(),
            destination: None,
            copy_motw: false,
            completed: false,
        };
        assert!(response.validate(&command).is_ok());
        let wire = serde_json::to_value(&response).expect("response");
        assert!(wire["destination"].is_null());
        for bad in [
            ArchiveRecoveryInspectCommand {
                generation: 0,
                ..command.clone()
            },
            ArchiveRecoveryInspectCommand {
                operation_id: Uuid::nil(),
                ..command.clone()
            },
            ArchiveRecoveryInspectCommand {
                source: "relative".into(),
                ..command.clone()
            },
        ] {
            assert!(bad.validate().is_err());
        }
        let mut stale = response.clone();
        stale.generation += 1;
        assert!(stale.validate(&command).is_err());
        let mut invalid = wire;
        invalid["copy_motw"] = serde_json::json!("false");
        assert!(serde_json::from_value::<ArchiveRecoveryInspection>(invalid).is_err());
    }
}
