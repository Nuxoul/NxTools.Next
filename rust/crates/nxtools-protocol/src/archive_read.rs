//! Product archive read protocol; deliberately separate from inherited worker/native frames.
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const ARCHIVE_READ_VERSION: u32 = 1;
pub const ARCHIVE_READ_LARGE_VERSION: u32 = 2;
pub const ARCHIVE_READ_LARGE_BATCH_BYTES: usize = 256 * 1024;
pub const ARCHIVE_READ_LARGE_BATCH_ENTRIES: usize = 256;
pub const ARCHIVE_READ_TRANSPORT_BYTES: u64 = 8 << 20;
pub const ARCHIVE_READ_LARGE_CAPABILITIES: [&str; 5] = [
    "product_read_v2",
    "bounded_metadata_v2",
    "metadata_credit_v1",
    "read_open_v1",
    "read_test_v1",
];
pub fn archive_read_capabilities(version: u32) -> &'static [&'static str] {
    if version == ARCHIVE_READ_LARGE_VERSION {
        &ARCHIVE_READ_LARGE_CAPABILITIES
    } else {
        &ARCHIVE_READ_CAPABILITIES
    }
}
fn default_transport_bytes() -> u64 {
    ARCHIVE_READ_TRANSPORT_BYTES
}
fn default_transport(value: &u64) -> bool {
    *value == ARCHIVE_READ_TRANSPORT_BYTES
}
fn is_false(value: &bool) -> bool {
    !*value
}
pub const ARCHIVE_READ_REQUEST_BYTES: usize = 256 * 1024;
pub const ARCHIVE_READ_BATCH_BYTES: usize = 512 * 1024;
pub const ARCHIVE_READ_EVENT_CAPACITY: usize = 8;
pub const ARCHIVE_READ_CAPABILITIES: [&str; 4] = [
    "product_read_v1",
    "metadata_credit_v1",
    "read_open_v1",
    "read_test_v1",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArchiveReadError {
    InvalidRequest,
    HelperMissing,
    HelperUntrusted,
    IncompatibleVersion,
    MissingCapability,
    UnsupportedOperation,
    UnsupportedFormat,
    UnsupportedMethod,
    UnsupportedEncryption,
    PasswordRequired,
    WrongPassword,
    AuthenticationOrDataError,
    CorruptArchive,
    MissingVolume,
    UnsafePath,
    OutputCollision,
    ResourceLimit,
    SelectedOutputLimit,
    SourceUnavailable,
    SourceChanged,
    Busy,
    Cancelled,
    Disconnected,
    WorkerCrashed,
    ProtocolViolation,
    ConsumerTooSlow,
    ConsumerQueueFull,
    MetadataBudget,
    IoFailure,
    CleanupIncomplete,
}
impl std::fmt::Display for ArchiveReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Fixed enum only, never a path or backend error message.
        write!(f, "{self:?}")
    }
}
impl std::error::Error for ArchiveReadError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArchiveReadAction {
    Open,
    Test,
    Ack,
    Cancel,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchiveReadCommand {
    pub version: u32,
    pub operation_id: Uuid,
    pub session_id: Uuid,
    pub generation: u64,
    pub action: ArchiveReadAction,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(default)]
    pub codepage: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limits: Option<ArchiveReadLimits>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sequence: Option<u64>,
}
impl ArchiveReadCommand {
    pub fn validate(&self) -> Result<(), ArchiveReadError> {
        if !matches!(
            self.version,
            ARCHIVE_READ_VERSION | ARCHIVE_READ_LARGE_VERSION
        ) {
            return Err(ArchiveReadError::IncompatibleVersion);
        }
        if self.operation_id.is_nil() || self.session_id.is_nil() || self.generation == 0 {
            return Err(ArchiveReadError::InvalidRequest);
        }
        match self.action {
            ArchiveReadAction::Open | ArchiveReadAction::Test => {
                let path = self
                    .path
                    .as_deref()
                    .ok_or(ArchiveReadError::InvalidRequest)?;
                let bytes = path.as_bytes();
                if bytes.len() < 4
                    || bytes.len() > 32767
                    || !bytes[0].is_ascii_alphabetic()
                    || bytes[1..3] != *b":\\"
                    || path.chars().any(char::is_control)
                    || path.contains('/')
                    || self.sequence.is_some()
                    || !valid_codepage(self.codepage)
                {
                    return Err(ArchiveReadError::InvalidRequest);
                }
                if self.version == ARCHIVE_READ_LARGE_VERSION && self.limits.is_none() {
                    return Err(ArchiveReadError::InvalidRequest);
                }
                self.limits.unwrap_or_default().validate_for(self.version)?;
            }
            ArchiveReadAction::Ack | ArchiveReadAction::Cancel => {
                if self.path.is_some()
                    || self.limits.is_some()
                    || self.codepage != 0
                    || (self.action == ArchiveReadAction::Ack) != self.sequence.is_some()
                {
                    return Err(ArchiveReadError::InvalidRequest);
                }
            }
        }
        let size = serde_json::to_vec(self)
            .map_err(|_| ArchiveReadError::InvalidRequest)?
            .len();
        if size > ARCHIVE_READ_REQUEST_BYTES {
            return Err(ArchiveReadError::InvalidRequest);
        }
        Ok(())
    }
}
pub fn valid_codepage(value: u32) -> bool {
    matches!(
        value,
        0 | 65001 | 936 | 950 | 932 | 949 | 437 | 1250 | 1251 | 1252
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ArchiveReadLimits {
    pub max_entries: u64,
    pub max_metadata_bytes: u64,
    pub max_working_bytes: u64,
    pub max_entry_bytes: u64,
    pub max_output_bytes: u64,
    pub max_retained_bytes: u64,
    pub ack_timeout_ms: u32,
    #[serde(
        default = "default_transport_bytes",
        skip_serializing_if = "default_transport"
    )]
    pub max_transport_bytes: u64,
    #[serde(default, skip_serializing_if = "is_false")]
    pub browser_context: bool,
}
impl Default for ArchiveReadLimits {
    fn default() -> Self {
        Self {
            max_entries: 4096,
            max_metadata_bytes: 4 << 20,
            max_working_bytes: 64 << 20,
            max_entry_bytes: 32 << 30,
            max_output_bytes: 100 << 30,
            max_retained_bytes: 4 << 20,
            ack_timeout_ms: 2000,
            max_transport_bytes: ARCHIVE_READ_TRANSPORT_BYTES,
            browser_context: false,
        }
    }
}
impl ArchiveReadLimits {
    pub fn validate(&self) -> Result<(), ArchiveReadError> {
        self.validate_for(ARCHIVE_READ_VERSION)
    }
    pub fn validate_for(&self, version: u32) -> Result<(), ArchiveReadError> {
        let large = version == ARCHIVE_READ_LARGE_VERSION;
        if !(1..=if large { 100_000 } else { 4096 }).contains(&self.max_entries)
            || !(1..=if large { 128 << 20 } else { 4 << 20 }).contains(&self.max_metadata_bytes)
            || !(64 << 20..=512 << 20).contains(&self.max_working_bytes)
            || self.max_entry_bytes == 0
            || self.max_output_bytes == 0
            || !large && self.browser_context
            || !(1..=if large { 64 << 20 } else { 4 << 20 }).contains(&self.max_retained_bytes)
            || if large {
                !(1..=64 << 20).contains(&self.max_transport_bytes)
            } else {
                self.max_transport_bytes != ARCHIVE_READ_TRANSPORT_BYTES
            }
            || !(200..=10000).contains(&self.ack_timeout_ms)
        {
            return Err(ArchiveReadError::InvalidRequest);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchiveReadEntry {
    pub ordinal: u64,
    pub name: String,
    pub kind: String,
    pub size: Option<u64>,
    pub packed_size: Option<u64>,
    pub encrypted: Option<bool>,
    pub warning_codes: Vec<String>,
    pub provenance: String,
    pub codepage: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modified: Option<crate::archive_browser::ArchiveModifiedTime>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub method: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub crc32: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attributes: Option<crate::archive_browser::ArchiveAttributes>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchiveReadBatch {
    pub snapshot_id: Uuid,
    pub sequence: u64,
    pub format: String,
    pub entries: Vec<ArchiveReadEntry>,
}
impl ArchiveReadBatch {
    pub fn validate(&self) -> Result<usize, ArchiveReadError> {
        self.validate_for(ARCHIVE_READ_VERSION)
    }
    pub fn validate_for(&self, version: u32) -> Result<usize, ArchiveReadError> {
        if self.snapshot_id.is_nil()
            || self.entries.is_empty()
            || self.entries.len()
                > if version == ARCHIVE_READ_LARGE_VERSION {
                    ARCHIVE_READ_LARGE_BATCH_ENTRIES
                } else {
                    2
                }
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
        for entry in &self.entries {
            let unicode = matches!(
                entry.provenance.as_str(),
                "Utf8Flag" | "UnicodeExtra" | "FormatUnicode"
            );
            if entry.name.len() > 3 * 65536
                || !matches!(
                    entry.kind.as_str(),
                    "File" | "Directory" | "Link" | "Special"
                )
                || !(unicode
                    || matches!(
                        entry.provenance.as_str(),
                        "Explicit" | "UncertainCp437" | "Heuristic"
                    ))
                || unicode && entry.codepage != Some(65001)
                || entry.provenance == "Explicit"
                    && !entry
                        .codepage
                        .is_some_and(|cp| cp != 0 && valid_codepage(cp))
                || entry.provenance == "UncertainCp437" && entry.codepage != Some(437)
                || entry.provenance == "Heuristic"
                    && !matches!(entry.codepage, Some(65001 | 936 | 932))
                || entry.warning_codes.len() > 4
                || entry.warning_codes.iter().any(|code| {
                    !matches!(
                        code.as_str(),
                        "UnsafePath" | "OutputCollision" | "LinkNotExtracted" | "UncertainEncoding"
                    )
                })
            {
                return Err(ArchiveReadError::ProtocolViolation);
            }
        }
        let bytes = serde_json::to_vec(self)
            .map_err(|_| ArchiveReadError::ProtocolViolation)?
            .len();
        if bytes
            > if version == ARCHIVE_READ_LARGE_VERSION {
                ARCHIVE_READ_LARGE_BATCH_BYTES
            } else {
                ARCHIVE_READ_BATCH_BYTES
            }
        {
            return Err(ArchiveReadError::MetadataBudget);
        }
        Ok(bytes)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArchiveReadStatus {
    Succeeded,
    Rejected,
    Failed,
    Cancelled,
    Disconnected,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchiveReadTransfer {
    pub private_bytes: u64,
    pub received_bytes: u64,
    pub max_batch_entries: u64,
    pub max_batch_bytes: u64,
    pub max_inflight_batches: u64,
    pub process_budget_bytes: u64,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchiveReadTerminal {
    pub status: ArchiveReadStatus,
    pub code: Option<ArchiveReadError>,
    pub files_verified: u64,
    pub bytes_verified: u64,
    pub entry_count: u64,
    pub batch_count: u64,
    pub elapsed_ms: u64,
    pub native_error_code: Option<i32>,
    pub cleanup_complete: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transfer: Option<ArchiveReadTransfer>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub browser: Option<Box<crate::archive_browser::ArchiveBrowserContext>>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArchiveReadEventKind {
    Accepted,
    Negotiated,
    Batch,
    Terminal,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchiveReadEvent {
    pub version: u32,
    pub operation_id: Uuid,
    pub session_id: Uuid,
    pub generation: u64,
    pub kind: ArchiveReadEventKind,
    #[serde(default)]
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub batch: Option<ArchiveReadBatch>,
    #[serde(default)]
    pub terminal: Option<ArchiveReadTerminal>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchiveReadReceipt {
    pub accepted: bool,
    pub code: Option<ArchiveReadError>,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn read_wire_rejects_versions_write_authority_and_unknown_fields() {
        let mut value = serde_json::json!({"version":1,"operation_id":Uuid::new_v4(),"session_id":Uuid::new_v4(),
            "generation":1,"action":"open","path":"C:\\owned\\fixture.zip"});
        let command: ArchiveReadCommand =
            serde_json::from_value(value.clone()).expect("valid read request");
        assert_eq!(command.validate(), Ok(()));
        for key in ["password", "output_root", "delete_source", "worker_path"] {
            value[key] = serde_json::json!("forbidden");
            assert!(
                serde_json::from_value::<ArchiveReadCommand>(value.clone()).is_err(),
                "{key}"
            );
            value.as_object_mut().expect("object").remove(key);
        }
        value["version"] = serde_json::json!(3);
        assert_eq!(
            serde_json::from_value::<ArchiveReadCommand>(value.clone())
                .expect("typed version")
                .validate(),
            Err(ArchiveReadError::IncompatibleVersion)
        );
        value.as_object_mut().expect("object").remove("version");
        assert!(serde_json::from_value::<ArchiveReadCommand>(value).is_err());
    }
    #[test]
    fn read_defaults_and_control_validation_are_explicit() {
        let limits: ArchiveReadLimits = serde_json::from_str("{}").expect("absent fields default");
        assert_eq!(limits, ArchiveReadLimits::default());
        for json in [
            "{\"max_entries\":0}",
            "{\"max_entries\":4097}",
            "{\"ack_timeout_ms\":0}",
            "{\"unknown\":1}",
        ] {
            assert!(
                serde_json::from_str::<ArchiveReadLimits>(json)
                    .map_or(true, |v| v.validate().is_err())
            );
        }
        let mut command = ArchiveReadCommand {
            version: 1,
            operation_id: Uuid::new_v4(),
            session_id: Uuid::new_v4(),
            generation: 1,
            action: ArchiveReadAction::Ack,
            path: None,
            codepage: 0,
            limits: None,
            sequence: Some(0),
        };
        assert_eq!(command.validate(), Ok(()));
        command.path = Some("C:\\forbidden.zip".into());
        assert_eq!(command.validate(), Err(ArchiveReadError::InvalidRequest));
    }
    #[test]
    fn selected_output_limit_has_a_fixed_public_code() {
        let encoded = serde_json::to_string(&ArchiveReadError::SelectedOutputLimit)
            .expect("fixed selected output code");
        assert_eq!(encoded, "\"selected_output_limit\"");
        assert_eq!(
            serde_json::from_str::<ArchiveReadError>(&encoded).expect("roundtrip"),
            ArchiveReadError::SelectedOutputLimit
        );
    }
}

#[cfg(test)]
mod large_tests {
    use super::*;
    #[test]
    fn explicit_v2_bounds_do_not_change_legacy_defaults() {
        let defaults = ArchiveReadLimits::default();
        assert_eq!(
            (
                defaults.max_entries,
                defaults.max_metadata_bytes,
                defaults.max_retained_bytes
            ),
            (4096, 4 << 20, 4 << 20)
        );
        assert!(
            serde_json::to_value(defaults)
                .expect("legacy JSON")
                .get("max_transport_bytes")
                .is_none()
        );
        let large = ArchiveReadLimits {
            max_entries: 100000,
            max_metadata_bytes: 128 << 20,
            max_retained_bytes: 64 << 20,
            max_transport_bytes: 64 << 20,
            ..defaults
        };
        assert_eq!(large.validate_for(2), Ok(()));
        assert_eq!(large.validate(), Err(ArchiveReadError::InvalidRequest));
        for invalid in [
            ArchiveReadLimits {
                max_entries: 100001,
                ..large
            },
            ArchiveReadLimits {
                max_transport_bytes: 0,
                ..large
            },
            ArchiveReadLimits {
                max_transport_bytes: (64 << 20) + 1,
                ..large
            },
            ArchiveReadLimits {
                max_retained_bytes: (64 << 20) + 1,
                ..large
            },
            ArchiveReadLimits {
                max_metadata_bytes: (128 << 20) + 1,
                ..large
            },
        ] {
            assert_eq!(
                invalid.validate_for(2),
                Err(ArchiveReadError::InvalidRequest)
            );
        }
        let mut request:ArchiveReadCommand = serde_json::from_value(serde_json::json!({"version":2,
            "operation_id":Uuid::new_v4(),"session_id":Uuid::new_v4(),"generation":1,"action":"open","path":"C:\\owned\\small.zip"})).expect("typed v2");
        assert_eq!(request.validate(), Err(ArchiveReadError::InvalidRequest));
        request.limits = Some(large);
        assert_eq!(request.validate(), Ok(()));
        request.version = 1;
        assert_eq!(request.validate(), Err(ArchiveReadError::InvalidRequest));
    }
    #[test]
    fn heuristic_filename_provenance_accepts_only_the_three_bounded_candidates() {
        let mut batch = ArchiveReadBatch {
            snapshot_id: Uuid::new_v4(),
            sequence: 0,
            format: "Zip".into(),
            entries: vec![ArchiveReadEntry {
                ordinal: 0,
                name: "candidate.txt".into(),
                kind: "File".into(),
                size: Some(0),
                packed_size: Some(0),
                encrypted: Some(false),
                warning_codes: vec!["UncertainEncoding".into()],
                provenance: "Heuristic".into(),
                codepage: None,
                modified: None,
                method: None,
                crc32: None,
                attributes: None,
            }],
        };
        for cp in [
            Some(65001),
            Some(936),
            Some(932),
            Some(437),
            Some(950),
            None,
        ] {
            batch.entries[0].codepage = cp;
            assert_eq!(
                batch.validate().is_ok(),
                matches!(cp, Some(65001 | 936 | 932))
            );
        }
    }

    #[test]
    fn v2_batch_entry_and_serialized_byte_limits_are_independent() {
        let entry = ArchiveReadEntry {
            ordinal: 0,
            name: "literal".into(),
            kind: "File".into(),
            size: Some(0),
            packed_size: Some(0),
            encrypted: Some(false),
            warning_codes: vec![],
            provenance: "Utf8Flag".into(),
            codepage: Some(65001),
            modified: None,
            method: None,
            crc32: None,
            attributes: None,
        };
        let mut batch = ArchiveReadBatch {
            snapshot_id: Uuid::new_v4(),
            sequence: 0,
            format: "Zip".into(),
            entries: vec![entry.clone(); 256],
        };
        assert!(batch.validate_for(2).is_ok());
        assert_eq!(batch.validate(), Err(ArchiveReadError::ProtocolViolation));
        batch.entries.push(entry.clone());
        assert_eq!(
            batch.validate_for(2),
            Err(ArchiveReadError::ProtocolViolation)
        );
        batch.entries = vec![entry; 2];
        batch.entries[0].name = "x".repeat(150000);
        batch.entries[1].name = "y".repeat(150000);
        assert_eq!(batch.validate_for(2), Err(ArchiveReadError::MetadataBudget));
    }
}
