//! Narrow browser materialization, never a Shell launch or arbitrary output authority.
use crate::archive_read::{ArchiveReadError, valid_codepage};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const ARCHIVE_ENTRY_VERSION: u32 = 1;
pub const ARCHIVE_ENTRY_TEMP_BYTES: u64 = 512 << 20;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ArchiveModifiedTime {
    DosLocal {
        date: u16,
        time: u16,
    },
    FiletimeUtc {
        ticks: u64,
    },
    UnixUtc {
        seconds: i64,
        nanoseconds: u32,
        precision_ns: u32,
    },
    Invalid,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchiveAttributes {
    pub read_only: bool,
    pub hidden: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchiveSourceBinding {
    pub native: String,
    pub digest: [u8; 32],
    pub size: u64,
    pub modified_unix_ns: u64,
}
impl ArchiveSourceBinding {
    pub fn validate(&self) -> Result<(), ArchiveReadError> {
        if self.native.len() > 256
            || !self.native.starts_with("v1:")
            || !self.native.is_ascii()
            || self.native.chars().any(char::is_control)
        {
            return Err(ArchiveReadError::InvalidRequest);
        }
        Ok(())
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SevenzEditEligibility {
    pub refusal: Option<crate::archive_job::ArchiveModificationRefusal>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TarEditEligibility {
    pub refusal: Option<crate::archive_job::ArchiveModificationRefusal>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchiveBrowserContext {
    pub snapshot_id: Uuid,
    pub format: String,
    pub comment: String,
    pub source: ArchiveSourceBinding,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sevenz_edit: Option<SevenzEditEligibility>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tar_edit: Option<TarEditEligibility>,
}
impl ArchiveBrowserContext {
    pub fn validate(&self) -> Result<(), ArchiveReadError> {
        self.source.validate()?;
        if self.snapshot_id.is_nil()
            || self.comment.len() > 65536
            || !matches!(
                self.format.as_str(),
                "Zip"
                    | "SevenZ"
                    | "Rar4"
                    | "Rar5"
                    | "Tar"
                    | "TarGz"
                    | "TarBz2"
                    | "TarXz"
                    | "TarZstd"
                    | "Gzip"
                    | "Bzip2"
                    | "Xz"
                    | "Zstd"
                    | "Alz"
                    | "Egg"
                    | "Cab"
                    | "Iso"
                    | "Udf"
                    | "Wim"
                    | "Ar"
                    | "Cpio"
                    | "Lzma"
                    | "Lz4"
                    | "Lz4Block"
                    | "Brotli"
            )
        {
            return Err(ArchiveReadError::ProtocolViolation);
        }
        Ok(())
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchiveTempPolicy {
    pub base_directory: String,
    pub retention_hours: u32,
    pub cleanup_on_child_exit: bool,
}
impl ArchiveTempPolicy {
    pub fn validate(&self) -> Result<(), ArchiveReadError> {
        let path = &self.base_directory;
        if !matches!(self.retention_hours, 1 | 6 | 24 | 72 | 168) {
            return Err(ArchiveReadError::InvalidRequest);
        }
        if path.is_empty() {
            return Ok(());
        }
        let bytes = path.as_bytes();
        if bytes.len() > 4096
            || bytes.len() < 3
            || !bytes[0].is_ascii_alphabetic()
            || &bytes[1..3] != b":\\"
            || path.chars().any(char::is_control)
            || path[3..].contains(['/', ':', '*', '?', '"', '<', '>', '|'])
            || (path.len() > 3 && path[3..].trim_end_matches('\\').split('\\').count() > 59)
            || path.ends_with("\\\\")
            || (path.len() > 3
                && path[3..].trim_end_matches('\\').split('\\').any(|part| {
                    let stem = part
                        .split('.')
                        .next()
                        .unwrap_or_default()
                        .trim_end_matches(' ')
                        .to_ascii_uppercase();
                    part.is_empty()
                        || part.encode_utf16().count() > 255
                        || matches!(part, "." | "..")
                        || part.ends_with([' ', '.'])
                        || matches!(
                            stem.as_str(),
                            "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
                        )
                        || stem
                            .strip_prefix("COM")
                            .or_else(|| stem.strip_prefix("LPT"))
                            .is_some_and(|suffix| {
                                matches!(
                                    suffix,
                                    "1" | "2"
                                        | "3"
                                        | "4"
                                        | "5"
                                        | "6"
                                        | "7"
                                        | "8"
                                        | "9"
                                        | "¹"
                                        | "²"
                                        | "³"
                                )
                            })
                }))
        {
            return Err(ArchiveReadError::InvalidRequest);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArchiveEntryAction {
    Materialize,
    Preview,
    PreviewText,
    Open,
    Cancel,
    Release,
    Authorize,
    Revoke,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchivePreviewOptions {
    pub max_bytes: u64,
    pub timeout_ms: u32,
}
impl ArchivePreviewOptions {
    fn validate(self, action: ArchiveEntryAction) -> Result<(), ArchiveReadError> {
        let maximum = match action {
            ArchiveEntryAction::PreviewText => 8 << 20,
            ArchiveEntryAction::Preview => 128 << 20,
            _ => return Err(ArchiveReadError::InvalidRequest),
        };
        if self.max_bytes == 0
            || self.max_bytes > maximum
            || !(200..=120_000).contains(&self.timeout_ms)
        {
            return Err(ArchiveReadError::InvalidRequest);
        }
        Ok(())
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchiveEntryCommand {
    pub version: u32,
    pub operation_id: Uuid,
    pub session_id: Uuid,
    pub generation: u64,
    pub action: ArchiveEntryAction,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(default)]
    pub codepage: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ordinal: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub browser: Option<ArchiveBrowserContext>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lease_id: Option<Uuid>,
    #[serde(default)]
    pub depth: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub copy_motw: Option<bool>,
    /// Non-secret opt-in for the original private worker InteractionPort; never a password.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub password_interaction: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temp_policy: Option<ArchiveTempPolicy>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preview_options: Option<ArchivePreviewOptions>,
}
impl ArchiveEntryCommand {
    pub fn validate(&self) -> Result<(), ArchiveReadError> {
        if self.version != ARCHIVE_ENTRY_VERSION {
            return Err(ArchiveReadError::IncompatibleVersion);
        }
        if self.operation_id.is_nil()
            || self.session_id.is_nil()
            || self.generation == 0
            || self.lease_id.is_some_and(|id| id.is_nil())
        {
            return Err(ArchiveReadError::InvalidRequest);
        }
        if let Some(policy) = &self.temp_policy {
            if !matches!(
                self.action,
                ArchiveEntryAction::Materialize
                    | ArchiveEntryAction::Preview
                    | ArchiveEntryAction::PreviewText
            ) {
                return Err(ArchiveReadError::InvalidRequest);
            }
            policy.validate()?;
        }
        if let Some(options) = self.preview_options {
            options.validate(self.action)?;
        }
        match self.action {
            ArchiveEntryAction::Materialize
            | ArchiveEntryAction::Preview
            | ArchiveEntryAction::PreviewText
            | ArchiveEntryAction::Authorize => {
                let path = self
                    .path
                    .as_deref()
                    .ok_or(ArchiveReadError::InvalidRequest)?;
                let b = path.as_bytes();
                if b.len() < 4
                    || b.len() > 32767
                    || !b[0].is_ascii_alphabetic()
                    || b[1..3] != *b":\\"
                    || path.contains('/')
                    || path.chars().any(char::is_control)
                    || !valid_codepage(self.codepage)
                    || (self.action == ArchiveEntryAction::Authorize) != self.ordinal.is_none()
                    || self.action == ArchiveEntryAction::Authorize
                        && (!self.password_interaction || self.lease_id.is_some())
                    || matches!(
                        self.action,
                        ArchiveEntryAction::Preview | ArchiveEntryAction::PreviewText
                    ) && self.lease_id.is_some()
                    || self.depth > 3
                {
                    return Err(ArchiveReadError::InvalidRequest);
                }
                self.browser
                    .as_ref()
                    .ok_or(ArchiveReadError::InvalidRequest)?
                    .validate()?;
                if self.action == ArchiveEntryAction::Authorize
                    && !matches!(
                        self.browser.as_ref().map(|browser| browser.format.as_str()),
                        Some("Zip" | "SevenZ" | "Rar4" | "Rar5")
                    )
                {
                    return Err(ArchiveReadError::InvalidRequest);
                }
            }
            ArchiveEntryAction::Cancel
            | ArchiveEntryAction::Release
            | ArchiveEntryAction::Open
            | ArchiveEntryAction::Revoke => {
                if self.password_interaction
                    || self.path.is_some()
                    || self.browser.is_some()
                    || self.ordinal.is_some()
                    || self.codepage != 0
                    || self.depth != 0
                    || self.copy_motw.is_some()
                    || matches!(
                        self.action,
                        ArchiveEntryAction::Release
                            | ArchiveEntryAction::Open
                            | ArchiveEntryAction::Revoke
                    ) != self.lease_id.is_some()
                    || self.action == ArchiveEntryAction::Revoke
                        && self.lease_id != Some(self.operation_id)
                {
                    return Err(ArchiveReadError::InvalidRequest);
                }
            }
        }
        Ok(())
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchiveEntryResult {
    pub version: u32,
    pub operation_id: Uuid,
    pub session_id: Uuid,
    pub generation: u64,
    pub success: bool,
    pub lease_id: Option<Uuid>,
    pub path: Option<String>,
    pub code: Option<ArchiveReadError>,
    pub native_error_code: Option<i32>,
    pub cleanup_complete: bool,
    pub retained: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preview_purposes_keep_original_shape_and_refuse_lease_authority() {
        for action in ["preview", "preview_text"] {
            let value = serde_json::json!({"version":1,"operation_id":Uuid::new_v4(),
                "session_id":Uuid::new_v4(),"generation":1,"action":action,
                "path":"C:\\owned\\source.zip","ordinal":0,
                "browser":{"snapshot_id":Uuid::new_v4(),"format":"Zip","comment":"",
                    "source":{"native":"v1:fixture","digest":vec![0;32],"size":0,"modified_unix_ns":0}}});
            let command: ArchiveEntryCommand = serde_json::from_value(value).expect("preview wire");
            assert!(command.validate().is_ok());
            assert_eq!(
                serde_json::to_value(&command).expect("roundtrip")["action"],
                action
            );
            let mut illegal = command.clone();
            illegal.lease_id = Some(Uuid::new_v4());
            assert_eq!(illegal.validate(), Err(ArchiveReadError::InvalidRequest));
            illegal = command.clone();
            illegal.ordinal = None;
            assert_eq!(illegal.validate(), Err(ArchiveReadError::InvalidRequest));
        }
    }
    #[test]
    fn browser_command_refuses_output_authority_and_stale_shape() {
        let mut value = serde_json::json!({"version":1,"operation_id":Uuid::new_v4(),"session_id":Uuid::new_v4(),"generation":1,"action":"cancel"});
        assert!(
            serde_json::from_value::<ArchiveEntryCommand>(value.clone())
                .expect("cancel")
                .validate()
                .is_ok()
        );
        value["output_root"] = serde_json::json!("C:\\forbidden");
        assert!(serde_json::from_value::<ArchiveEntryCommand>(value.clone()).is_err());
        value.as_object_mut().expect("object").remove("output_root");
        value["action"] = serde_json::json!("materialize");
        assert!(
            serde_json::from_value::<ArchiveEntryCommand>(value)
                .expect("shape")
                .validate()
                .is_err()
        );
    }
    #[test]
    fn browser_password_opt_in_is_not_secret_or_read_authority() {
        let mut command: ArchiveEntryCommand = serde_json::from_value(serde_json::json!({
            "version":1, "operation_id":Uuid::new_v4(), "session_id":Uuid::new_v4(),
            "generation":1, "action":"cancel" }))
        .expect("legacy default");
        assert!(!command.password_interaction);
        command.password_interaction = true;
        assert_eq!(command.validate(), Err(ArchiveReadError::InvalidRequest));
        let value = serde_json::to_value(&command).expect("wire");
        assert!(value.get("password").is_none());
        assert!(
            serde_json::from_value::<ArchiveEntryCommand>(serde_json::json!({
            "version":1,"operation_id":command.operation_id,"session_id":command.session_id,
            "generation":1,"action":"cancel","password":"forbidden"}))
            .is_err()
        );
    }
    #[test]
    fn legacy_entry_and_terminal_omit_browser_extensions() {
        let entry: crate::archive_read::ArchiveReadEntry = serde_json::from_value(serde_json::json!({"ordinal":0,"name":"x","kind":"File","size":0,"packed_size":0,"encrypted":false,"warning_codes":[],"provenance":"Utf8Flag","codepage":65001})).expect("legacy entry");
        let value = serde_json::to_value(entry).expect("serialize");
        for field in ["modified", "method", "crc32", "attributes"] {
            assert!(value.get(field).is_none());
        }
        let terminal: crate::archive_read::ArchiveReadTerminal = serde_json::from_value(serde_json::json!({"status":"succeeded","code":null,"files_verified":0,"bytes_verified":0,"entry_count":0,"batch_count":0,"elapsed_ms":0,"native_error_code":null,"cleanup_complete":true})).expect("legacy terminal");
        assert!(terminal.browser.is_none());
        assert!(
            serde_json::to_value(terminal)
                .expect("serialize")
                .get("browser")
                .is_none()
        );
    }
    #[test]
    fn browser_opt_in_preserves_both_read_defaults() {
        let mut limits = crate::archive_read::ArchiveReadLimits::default();
        assert!(!limits.browser_context);
        assert!(
            serde_json::to_value(limits)
                .expect("legacy limits")
                .get("browser_context")
                .is_none()
        );
        limits.browser_context = true;
        assert!(limits.validate().is_err());
        assert!(limits.validate_for(2).is_ok());
    }
}

#[cfg(test)]
mod authorization_tests {
    use super::*;
    #[test]
    fn authorize_revoke_are_nonsecret_no_output_shapes_and_unknown_formats_refuse() {
        let mut command = ArchiveEntryCommand {
            version: 1,
            operation_id: Uuid::new_v4(),
            session_id: Uuid::new_v4(),
            generation: 1,
            action: ArchiveEntryAction::Authorize,
            path: Some(r"C:\owned\source.zip".into()),
            codepage: 0,
            ordinal: None,
            browser: Some(ArchiveBrowserContext {
                snapshot_id: Uuid::new_v4(),
                format: "Zip".into(),
                comment: String::new(),
                source: ArchiveSourceBinding {
                    native: "v1:test".into(),
                    digest: [1; 32],
                    size: 1,
                    modified_unix_ns: 1,
                },
                sevenz_edit: None,
                tar_edit: None,
            }),
            lease_id: None,
            depth: 0,
            copy_motw: Some(true),
            password_interaction: true,
            temp_policy: None,
            preview_options: None,
        };
        command.validate().expect("authorize source only");
        let value = serde_json::to_value(&command).expect("wire");
        assert!(
            value.get("password").is_none()
                && value.get("output_root").is_none()
                && value.get("ordinal").is_none()
        );
        command.ordinal = Some(0);
        assert!(command.validate().is_err());
        command.ordinal = None;
        command.browser.as_mut().expect("browser").format = "Gzip".into();
        assert!(command.validate().is_err());
        command.action = ArchiveEntryAction::Revoke;
        command.path = None;
        command.browser = None;
        command.copy_motw = None;
        command.password_interaction = false;
        command.lease_id = Some(command.operation_id);
        command.validate().expect("exact revoke");
        command.lease_id = Some(Uuid::new_v4());
        assert!(command.validate().is_err());
    }
}

#[cfg(test)]
mod remaining_temp_tests {
    use super::*;
    #[test]
    fn ui_remaining_temp_policy_is_consumed_and_open_is_lease_scoped() {
        let mut value = serde_json::json!({"version":1,"operation_id":Uuid::new_v4(),"session_id":Uuid::new_v4(),"generation":1,"action":"open","lease_id":Uuid::new_v4()});
        let command: ArchiveEntryCommand =
            serde_json::from_value(value.clone()).expect("lease open");
        assert!(command.validate().is_ok(), "owned lease open action");
        value["temp_policy"] = serde_json::json!({"base_directory":"C:\\owned","retention_hours":6,"cleanup_on_child_exit":true});
        assert!(
            serde_json::from_value::<ArchiveEntryCommand>(value.clone())
                .map_or(true, |command| command.validate().is_err()),
            "control never grants output authority"
        );
        for (base, hours) in [("", 24), (r"C:\owned", 6), (r"D:\", 168)] {
            assert!(
                ArchiveTempPolicy {
                    base_directory: base.into(),
                    retention_hours: hours,
                    cleanup_on_child_exit: true
                }
                .validate()
                .is_ok()
            );
        }
        for (base, hours) in [
            (r"relative", 24),
            (r"\\server\share", 24),
            (r"C:\owned\..\other", 24),
            ("", 0),
        ] {
            assert!(
                ArchiveTempPolicy {
                    base_directory: base.into(),
                    retention_hours: hours,
                    cleanup_on_child_exit: false
                }
                .validate()
                .is_err()
            );
        }
    }
}

#[cfg(test)]
mod preview_options_tests {
    use super::*;
    #[test]
    fn preview_options_are_strict_optional_and_never_materialize_authority() {
        let mut value = serde_json::json!({
            "version":1, "operation_id":Uuid::new_v4(), "session_id":Uuid::new_v4(),
            "generation":1, "action":"preview_text", "path":"C:\\owned\\source.zip", "ordinal":0,
            "browser": {"snapshot_id":Uuid::new_v4(), "format":"Zip", "comment":"",
                "source":{"native":"v1:test", "digest":[1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1],"size":1,"modified_unix_ns":1}}
        });
        let parse = |value: serde_json::Value| {
            serde_json::from_value::<ArchiveEntryCommand>(value)
                .map_or(false, |command| command.validate().is_ok())
        };
        assert!(parse(value.clone()), "legacy request");
        let legacy: ArchiveEntryCommand = serde_json::from_value(value.clone()).expect("legacy");
        assert!(
            serde_json::to_value(legacy)
                .expect("wire")
                .get("preview_options")
                .is_none()
        );
        value["preview_options"] = serde_json::Value::Null;
        assert!(parse(value.clone()), "explicit legacy null");
        value["preview_options"] = serde_json::json!({"max_bytes":1048576,"timeout_ms":120000});
        assert!(parse(value.clone()), "configured preview options");
        for action in [
            "materialize",
            "authorize",
            "cancel",
            "release",
            "open",
            "revoke",
        ] {
            let mut invalid = value.clone();
            invalid["action"] = serde_json::json!(action);
            assert!(!parse(invalid), "options cannot authorize {action}");
        }
        for (action, maximum) in [("preview_text", 8u64 << 20), ("preview", 128u64 << 20)] {
            value["action"] = serde_json::json!(action);
            for (bytes, timeout, valid) in [
                (1, 200, true),
                (maximum, 120000, true),
                (0, 30000, false),
                (maximum + 1, 30000, false),
                (1, 199, false),
                (1, 120001, false),
            ] {
                value["preview_options"] =
                    serde_json::json!({"max_bytes":bytes,"timeout_ms":timeout});
                assert_eq!(parse(value.clone()), valid, "{action}/{bytes}/{timeout}");
            }
        }
        for invalid in [
            serde_json::json!({}),
            serde_json::json!({"max_bytes":1}),
            serde_json::json!({"max_bytes":1,"timeout_ms":30000,"unknown":1}),
            serde_json::json!({"max_bytes":"1","timeout_ms":30000}),
            serde_json::json!({"max_bytes":1.5,"timeout_ms":30000}),
            serde_json::json!([]),
        ] {
            value["preview_options"] = invalid;
            assert!(!parse(value.clone()), "malformed options");
        }
        let wire = serde_json::to_string(&value).expect("JSON");
        let duplicate = wire.replace(
            "\"preview_options\":[]",
            "\"preview_options\":{\"max_bytes\":1,\"max_bytes\":2,\"timeout_ms\":30000}",
        );
        assert!(serde_json::from_str::<ArchiveEntryCommand>(&duplicate).is_err());
    }
}
