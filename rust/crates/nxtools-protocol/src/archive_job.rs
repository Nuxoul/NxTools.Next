//! Bounded general execution contract. No executable, source deletion or Shell authority.
use crate::{
    archive_browser::ArchiveBrowserContext,
    archive_password::PrivatePassword,
    archive_read::{ArchiveReadError, valid_codepage},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const ARCHIVE_JOB_VERSION: u32 = 1;
macro_rules! wire_enum { ($name:ident { $($variant:ident),+ $(,)? }) => {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(rename_all = "snake_case")]
    pub enum $name { $($variant),+ }
}; }
wire_enum!(ArchiveJobAction {
    Start,
    Cancel,
    Pause,
    Resume,
    Reply,
    RetireBatch
});
wire_enum!(ArchiveJobKind {
    Extract,
    Create,
    Test,
    Modify
});
wire_enum!(ArchiveJobModifyAction {
    Add,
    Delete,
    Rename,
    NewDirectory,
    ReplaceContent,
    Comment
});
/// Semantic intent shared by capability projections and the existing modification request.
/// This does not add a wire action or grant any new native/codec authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchiveOperation {
    Browse,
    Extract,
    Preview,
    Add,
    Delete,
    ReplaceContent,
    RenameFile,
    RenameDirectorySubtree,
    NewDirectory,
    Comment,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArchiveModificationRefusal {
    BackendMissing,
    SnapshotNotReady,
    SourceChanged,
    Volumes,
    SelfExtracting,
    ApplicationContainer,
    UnsupportedEncryption,
    UnsupportedStructure,
    Corrupt,
    UnverifiedMetadata,
    SourceUnavailable,
}
impl ArchiveModificationRefusal {
    pub fn message(self) -> &'static str {
        match self {
            Self::BackendMissing => "此格式支持浏览和解压，暂不支持修改压缩包内容。",
            Self::SnapshotNotReady => "仅完整的就绪快照可编辑。",
            Self::SourceChanged => "源已变化或旧快照不能编辑；请刷新。",
            Self::Volumes => "分卷压缩包只读，不能编辑。",
            Self::SelfExtracting => "已确认的自解压压缩包不能编辑。",
            Self::ApplicationContainer => "应用容器不能编辑。",
            Self::UnsupportedEncryption => "此加密结构不支持修改。",
            Self::UnsupportedStructure => "此压缩包结构不支持修改。",
            Self::Corrupt => "压缩包结构已损坏，不能修改。",
            Self::UnverifiedMetadata => "尚未确认源结构或元数据可安全保留，不能修改。",
            Self::SourceUnavailable => "源不可访问，请刷新后重试。",
        }
    }
}
wire_enum!(ArchiveJobStrategy { Here, Name, Smart });
wire_enum!(ArchiveJobConflict {
    Ask,
    Overwrite,
    Skip,
    AutoRename
});
wire_enum!(ArchiveJobFormat {
    Zip,
    SevenZ,
    Tar,
    Tgz,
    Txz,
    Gz,
    Xz
});
wire_enum!(ArchiveJobLevel {
    Store,
    Fast,
    Normal,
    Maximum
});
wire_enum!(ArchiveJobMethod { Store, Deflate });
wire_enum!(ArchiveJobEventKind {
    Accepted,
    Started,
    State,
    Progress,
    Prompt,
    Terminal
});
wire_enum!(ArchiveJobState {
    Running,
    Paused,
    AwaitingPassword,
    AwaitingConflict,
    AwaitingRisk,
    Verifying,
    Committing,
    Cancelling
});
wire_enum!(ArchiveJobOutcome {
    Succeeded,
    SucceededWithWarnings,
    Cancelled,
    Failed,
    Rejected
});
wire_enum!(ArchiveJobPromptKind {
    Password,
    Conflict,
    Risk,
    OutputBudget,
    Destination,
    CreateOverwrite
});
wire_enum!(ArchiveJobConflictDecision {
    Overwrite,
    Skip,
    AutoRename
});
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchiveJobCommand {
    pub version: u32,
    pub operation_id: Uuid,
    pub session_id: Uuid,
    pub generation: u64,
    pub action: ArchiveJobAction,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request: Option<ArchiveJobRequest>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reply: Option<ArchiveJobReply>,
    /// Original immutable batch association; no preview/drag authorization.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub batch_id: Option<Uuid>,
}
impl ArchiveJobCommand {
    pub fn validate(&self) -> Result<(), ArchiveReadError> {
        if self.version != ARCHIVE_JOB_VERSION {
            return Err(ArchiveReadError::IncompatibleVersion);
        }
        if self.operation_id.is_nil() || self.session_id.is_nil() || self.generation == 0 {
            return Err(ArchiveReadError::InvalidRequest);
        }
        if self.batch_id.is_some_and(|id| id.is_nil())
            || self
                .reply
                .as_ref()
                .is_some_and(|r| r.remember_batch_password && self.batch_id.is_none())
        {
            return Err(ArchiveReadError::InvalidRequest);
        }
        match self.action {
            ArchiveJobAction::Start if self.reply.is_none() => self
                .request
                .as_ref()
                .ok_or(ArchiveReadError::InvalidRequest)?
                .validate(),
            ArchiveJobAction::Reply if self.request.is_none() => {
                let reply = self
                    .reply
                    .as_ref()
                    .ok_or(ArchiveReadError::InvalidRequest)?;
                if reply.ticket.operation_id != self.operation_id {
                    return Err(ArchiveReadError::InvalidRequest);
                }
                reply.validate()
            }
            ArchiveJobAction::RetireBatch
                if self.request.is_none()
                    && self.reply.is_none()
                    && self.batch_id == Some(self.operation_id)
                    && self.session_id == self.operation_id =>
            {
                Ok(())
            }
            ArchiveJobAction::Cancel | ArchiveJobAction::Pause | ArchiveJobAction::Resume
                if self.request.is_none() && self.reply.is_none() =>
            {
                Ok(())
            }
            _ => Err(ArchiveReadError::InvalidRequest),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchiveJobRequest {
    pub kind: ArchiveJobKind,
    pub paths: Vec<String>,
    pub destination: Option<String>,
    #[serde(default)]
    pub codepage: u32,
    #[serde(default)]
    pub limits: ArchiveJobLimits,
    pub extract: Option<ArchiveJobExtract>,
    pub create: Option<ArchiveJobCreate>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modify: Option<ArchiveJobModify>,
    /// Only the Rust finite readmission coordinator supplies this per-attempt binding.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub readmission: Option<ArchiveJobReadmission>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ArchiveJobLimits {
    pub max_entries: u64,
    pub max_metadata_bytes: u64,
    pub max_working_bytes: u64,
    pub max_entry_bytes: u64,
    pub max_output_bytes: u64,
}
impl Default for ArchiveJobLimits {
    fn default() -> Self {
        Self {
            max_entries: 1_000_000,
            max_metadata_bytes: 256 << 20,
            max_working_bytes: 512 << 20,
            max_entry_bytes: 32 << 30,
            max_output_bytes: 100 << 30,
        }
    }
}
impl ArchiveJobLimits {
    pub fn validate(&self) -> Result<(), ArchiveReadError> {
        if self.max_entries == 0
            || self.max_entries > 1_000_000
            || self.max_metadata_bytes == 0
            || self.max_metadata_bytes > 256 << 20
            || !(64 << 20..=512 << 20).contains(&self.max_working_bytes)
            || self.max_entry_bytes == 0
            || self.max_entry_bytes > 32 << 30
            || self.max_output_bytes == 0
            || self.max_output_bytes > 1 << 50
        {
            return Err(ArchiveReadError::InvalidRequest);
        }
        Ok(())
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchiveJobExtract {
    pub strategy: ArchiveJobStrategy,
    pub single_file_folder: bool,
    pub conflict: ArchiveJobConflict,
    pub copy_motw: bool,
    pub selection: Option<ArchiveJobSelection>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reclamation: Option<ArchiveReclamationIntent>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArchiveReclamationMode {
    Start,
    Continue,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchiveReclamationIntent {
    pub mode: ArchiveReclamationMode,
    pub recovery_id: Uuid,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchiveJobSelection {
    pub browser: ArchiveBrowserContext,
    pub ordinals: Vec<u64>,
}
impl ArchiveJobExtract {
    pub fn validate(&self) -> Result<(), ArchiveReadError> {
        if self.reclamation.is_some_and(|intent| {
            intent.recovery_id.is_nil()
                || self.strategy != ArchiveJobStrategy::Here
                || self.single_file_folder
                || self.conflict != ArchiveJobConflict::Ask
                || self.selection.is_some()
        }) {
            return Err(ArchiveReadError::InvalidRequest);
        }
        if let Some(selection) = &self.selection {
            selection.browser.validate()?;
            if selection.ordinals.is_empty()
                || selection.ordinals.len() > 4096
                || selection
                    .ordinals
                    .iter()
                    .copied()
                    .collect::<std::collections::HashSet<_>>()
                    .len()
                    != selection.ordinals.len()
            {
                return Err(ArchiveReadError::InvalidRequest);
            }
        }
        Ok(())
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchiveJobCreate {
    pub format: ArchiveJobFormat,
    pub level: Option<ArchiveJobLevel>,
    pub method: Option<ArchiveJobMethod>,
    pub password: Option<PrivatePassword>,
    pub solid: Option<bool>,
    pub encrypt_header: Option<bool>,
    pub strip_single_root: bool,
    pub exclude_patterns: Vec<String>,
    pub test_after_create: bool,
    pub name: String,
    /// Genuine split volumes are allowed only for a new ZIP/7z target family.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub volume_bytes: Option<u64>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchiveJobModify {
    pub browser: ArchiveBrowserContext,
    pub action: ArchiveJobModifyAction,
    pub ordinals: Vec<u64>,
    pub value: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_prefix: Option<String>,
}
pub fn valid_job_path(path: &str) -> bool {
    let b = path.as_bytes();
    b.len() >= 3
        && b.len() <= 32767
        && b[0].is_ascii_alphabetic()
        && b[1..3] == *b":\\"
        && !path.contains('/')
        && !path.chars().any(char::is_control)
}
pub fn valid_job_name(name: &str) -> bool {
    if name.is_empty()
        || name.len() > 255
        || name.ends_with(['.', ' '])
        || name
            .chars()
            .any(|c| c.is_control() || "\\/:*?\"<>|".contains(c))
        || name == "."
        || name == ".."
    {
        return false;
    }
    let base = name.split('.').next().unwrap_or("").to_ascii_uppercase();
    !matches!(base.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        && !(base.len() == 4
            && (base.starts_with("COM") || base.starts_with("LPT"))
            && matches!(base.as_bytes()[3], b'1'..=b'9'))
}
fn valid_password(secret: &PrivatePassword) -> bool {
    let Ok(mut encoded) = serde_json::to_vec(secret) else {
        return false;
    };
    let decoded = serde_json::from_slice::<Vec<u8>>(&encoded);
    encoded.fill(0);
    let Ok(mut bytes) = decoded else {
        return false;
    };
    let valid = !bytes.is_empty()
        && bytes.len() <= 4096
        && !bytes.contains(&0)
        && std::str::from_utf8(&bytes).is_ok();
    bytes.fill(0);
    valid
}
/// ZIP names use forward slashes; every component follows the Windows-safe
/// output policy. Directory targets/prefixes require a final slash.
pub fn valid_modify_target(value: &str, directory: bool) -> bool {
    let path = if directory {
        value.strip_suffix('/').unwrap_or("")
    } else {
        value
    };
    !path.is_empty() && value.len() <= 32767 && path.split('/').all(valid_job_name)
}
impl ArchiveJobModifyAction {
    pub fn operation(self, value: Option<&str>) -> ArchiveOperation {
        match self {
            Self::Add => ArchiveOperation::Add,
            Self::Delete => ArchiveOperation::Delete,
            Self::Rename if value.is_some_and(|value| value.ends_with('/')) => {
                ArchiveOperation::RenameDirectorySubtree
            }
            Self::Rename => ArchiveOperation::RenameFile,
            Self::NewDirectory => ArchiveOperation::NewDirectory,
            Self::ReplaceContent => ArchiveOperation::ReplaceContent,
            Self::Comment => ArchiveOperation::Comment,
        }
    }
}
impl ArchiveJobModify {
    /// Legacy rename wire spelling is preserved; directory intent is explicit in the
    /// validated target grammar and must be checked against the original member kind.
    pub fn operation(&self) -> ArchiveOperation {
        self.action.operation(self.value.as_deref())
    }
    /// Implemented edit backend gate, independent of create/extract format support.
    pub fn backend_refusal(&self) -> Option<ArchiveModificationRefusal> {
        (!matches!(
            self.browser.format.as_str(),
            "Zip" | "SevenZ" | "Tar" | "TarGz" | "TarXz"
        ) || self.browser.format == "Zip" && self.action == ArchiveJobModifyAction::ReplaceContent
            || self.browser.format != "Zip" && self.action == ArchiveJobModifyAction::Comment)
            .then_some(ArchiveModificationRefusal::BackendMissing)
    }
    pub fn validate(&self, path_count: usize) -> Result<(), ArchiveReadError> {
        self.browser.validate()?;
        if self.source_prefix.as_deref().is_some_and(|prefix| {
            !matches!(
                self.browser.format.as_str(),
                "SevenZ" | "Tar" | "TarGz" | "TarXz"
            ) || self.action != ArchiveJobModifyAction::Rename
                || self.ordinals.len() != 1
                || !self
                    .value
                    .as_deref()
                    .is_some_and(|value| value.ends_with('/'))
                || !valid_modify_target(prefix, true)
        }) {
            return Err(ArchiveReadError::InvalidRequest);
        }
        if self.backend_refusal().is_some()
            || self.ordinals.len() > 4096
            || self
                .ordinals
                .iter()
                .copied()
                .collect::<std::collections::HashSet<_>>()
                .len()
                != self.ordinals.len()
        {
            return Err(ArchiveReadError::InvalidRequest);
        }
        let valid = match self.action {
            ArchiveJobModifyAction::Add => {
                self.ordinals.is_empty()
                    && path_count > 1
                    && self
                        .value
                        .as_deref()
                        .is_none_or(|value| valid_modify_target(value, true))
            }
            ArchiveJobModifyAction::ReplaceContent => {
                matches!(
                    self.browser.format.as_str(),
                    "SevenZ" | "Tar" | "TarGz" | "TarXz"
                ) && path_count == 2
                    && self.ordinals.len() == 1
                    && self.value.is_none()
            }
            ArchiveJobModifyAction::Delete => {
                path_count == 1 && !self.ordinals.is_empty() && self.value.is_none()
            }
            ArchiveJobModifyAction::Rename => {
                path_count == 1
                    && self.ordinals.len() == 1
                    && self
                        .value
                        .as_deref()
                        .is_some_and(|value| valid_modify_target(value, value.ends_with('/')))
            }
            ArchiveJobModifyAction::NewDirectory => {
                path_count == 1
                    && self.ordinals.is_empty()
                    && self
                        .value
                        .as_deref()
                        .is_some_and(|value| valid_modify_target(value, true))
            }
            ArchiveJobModifyAction::Comment => {
                path_count == 1
                    && self.ordinals.is_empty()
                    && self
                        .value
                        .as_ref()
                        .is_some_and(|value| value.len() <= 65535 && !value.contains('\0'))
            }
        };
        if valid {
            Ok(())
        } else {
            Err(ArchiveReadError::InvalidRequest)
        }
    }
}
impl ArchiveJobRequest {
    pub fn validate(&self) -> Result<(), ArchiveReadError> {
        self.limits.validate()?;
        if self.readmission.is_some()
            && self
                .extract
                .as_ref()
                .is_some_and(|extract| extract.reclamation.is_some())
        {
            return Err(ArchiveReadError::InvalidRequest);
        }
        if self.readmission.as_ref().is_some_and(|r| {
            self.kind != ArchiveJobKind::Extract
                || r.logical_id.is_nil()
                || r.expected_source.as_ref().is_some_and(|s| {
                    s.native.is_empty()
                        || s.native.len() > 256
                        || !s.native.is_ascii()
                        || s.native.chars().any(char::is_control)
                })
        }) {
            return Err(ArchiveReadError::InvalidRequest);
        }
        if !valid_codepage(self.codepage)
            || self.paths.is_empty()
            || self.paths.len() > 32
            || self.paths.iter().any(|p| !valid_job_path(p))
            || self.paths.iter().map(String::len).sum::<usize>() > 131072
        {
            return Err(ArchiveReadError::InvalidRequest);
        }
        match self.kind {
            ArchiveJobKind::Test
                if self.paths.len() == 1
                    && self.destination.is_none()
                    && self.extract.is_none()
                    && self.create.is_none()
                    && self.modify.is_none() =>
            {
                Ok(())
            }
            ArchiveJobKind::Extract
                if self.paths.len() == 1
                    && self.destination.as_deref().is_some_and(valid_job_path)
                    && self.create.is_none()
                    && self.modify.is_none() =>
            {
                self.extract
                    .as_ref()
                    .ok_or(ArchiveReadError::InvalidRequest)?
                    .validate()
            }
            ArchiveJobKind::Modify
                if self.destination.is_none()
                    && self.extract.is_none()
                    && self.create.is_none()
                    && self.readmission.is_none() =>
            {
                self.modify
                    .as_ref()
                    .ok_or(ArchiveReadError::InvalidRequest)?
                    .validate(self.paths.len())
            }
            ArchiveJobKind::Create
                if self.destination.as_deref().is_some_and(valid_job_path)
                    && self.extract.is_none()
                    && self.modify.is_none() =>
            {
                let c = self
                    .create
                    .as_ref()
                    .ok_or(ArchiveReadError::InvalidRequest)?;
                if !valid_job_name(&c.name)
                    || c.exclude_patterns.len() > 32
                    || c.exclude_patterns.iter().map(String::len).sum::<usize>() > 8192
                    || c.exclude_patterns.iter().any(|p| p.contains('\0'))
                    || c.password.as_ref().is_some_and(|p| !valid_password(p))
                    || c.volume_bytes
                        .is_some_and(|bytes| bytes < 65536 || bytes > self.limits.max_output_bytes)
                    || c.volume_bytes.is_some()
                        && !matches!(c.format, ArchiveJobFormat::Zip | ArchiveJobFormat::SevenZ)
                {
                    return Err(ArchiveReadError::InvalidRequest);
                }
                let zip = c.format == ArchiveJobFormat::Zip;
                let seven = c.format == ArchiveJobFormat::SevenZ;
                if !seven && (c.solid.is_some() || c.encrypt_header.is_some())
                    || !zip && c.method.is_some()
                    || !zip && !seven && c.password.is_some()
                    || c.encrypt_header == Some(true) && c.password.is_none()
                {
                    return Err(ArchiveReadError::InvalidRequest);
                }
                if c.format == ArchiveJobFormat::Tar {
                    if c.level.is_some() {
                        return Err(ArchiveReadError::InvalidRequest);
                    }
                } else if c.level.is_none() || !zip && c.level == Some(ArchiveJobLevel::Store) {
                    return Err(ArchiveReadError::InvalidRequest);
                }
                if zip
                    && (c.method.is_none()
                        || (c.method == Some(ArchiveJobMethod::Store))
                            != (c.level == Some(ArchiveJobLevel::Store)))
                {
                    return Err(ArchiveReadError::InvalidRequest);
                }
                if matches!(c.format, ArchiveJobFormat::Gz | ArchiveJobFormat::Xz)
                    && (self.paths.len() != 1
                        || c.strip_single_root
                        || !c.exclude_patterns.is_empty())
                {
                    return Err(ArchiveReadError::InvalidRequest);
                }
                Ok(())
            }
            _ => Err(ArchiveReadError::InvalidRequest),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchiveJobTicket {
    pub operation_id: Uuid,
    pub prompt_id: Uuid,
    pub generation: u64,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchiveJobReply {
    pub ticket: ArchiveJobTicket,
    pub kind: ArchiveJobPromptKind,
    pub password: Option<PrivatePassword>,
    pub decision: Option<ArchiveJobConflictDecision>,
    pub apply_to_all: bool,
    pub approved_limit: Option<u64>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub remember_batch_password: bool,
}
impl ArchiveJobReply {
    fn validate(&self) -> Result<(), ArchiveReadError> {
        if self.ticket.operation_id.is_nil()
            || self.ticket.prompt_id.is_nil()
            || self.ticket.generation == 0
        {
            return Err(ArchiveReadError::InvalidRequest);
        }
        if self.remember_batch_password
            && (self.kind != ArchiveJobPromptKind::Password || self.password.is_none())
        {
            return Err(ArchiveReadError::InvalidRequest);
        }
        let valid = match self.kind {
            ArchiveJobPromptKind::Password => {
                self.decision.is_none()
                    && self.approved_limit.is_none()
                    && !self.apply_to_all
                    && self.password.as_ref().is_none_or(valid_password)
            }
            ArchiveJobPromptKind::Conflict => {
                self.password.is_none() && self.approved_limit.is_none()
            }
            ArchiveJobPromptKind::CreateOverwrite => {
                self.password.is_none()
                    && self.approved_limit.is_none()
                    && !self.apply_to_all
                    && matches!(
                        self.decision,
                        None | Some(ArchiveJobConflictDecision::Overwrite)
                    )
            }
            ArchiveJobPromptKind::Destination => {
                self.password.is_none()
                    && self.approved_limit.is_none()
                    && !self.apply_to_all
                    && matches!(
                        self.decision,
                        None | Some(ArchiveJobConflictDecision::Overwrite)
                    )
            }
            ArchiveJobPromptKind::Risk => {
                self.password.is_none()
                    && self.decision.is_none()
                    && !self.apply_to_all
                    && self.approved_limit.is_none_or(|v| v > 0 && v <= 32 << 30)
            }
            _ => false,
        };
        if valid {
            Ok(())
        } else {
            Err(ArchiveReadError::InvalidRequest)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchiveJobOutputBudget {
    pub entry_bytes: u64,
    pub output_bytes: u64,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchiveJobOutputSource {
    pub native: String,
    pub digest: [u8; 32],
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchiveJobReadmission {
    pub logical_id: Uuid,
    pub expected_source: Option<ArchiveJobOutputSource>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchiveJobOutputShortfall {
    pub current: ArchiveJobOutputBudget,
    pub required: ArchiveJobOutputBudget,
    pub source: ArchiveJobOutputSource,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchiveJobProgress {
    pub phase: String,
    pub bytes_done: u64,
    pub bytes_total: Option<u64>,
    pub files_done: u64,
    pub files_total: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_member: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_bytes_done: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_bytes_total: Option<u64>,
    /// Operation-local publication order across the lossless and coalesced channels.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sequence: Option<u64>,
}
/// UI-only immutable decision context. Never use these strings for filesystem authority.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchiveJobPromptDisplay {
    pub incoming_member: Option<String>,
    pub target: String,
}
impl std::fmt::Debug for ArchiveJobPromptDisplay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ArchiveJobPromptDisplay([PRIVATE])")
    }
}
impl ArchiveJobPromptDisplay {
    fn valid(&self, kind: ArchiveJobPromptKind) -> bool {
        let text = |value: &str, limit| {
            !value.is_empty() && value.len() <= limit && !value.chars().any(char::is_control)
        };
        text(&self.target, 65535)
            && match kind {
                ArchiveJobPromptKind::Conflict => self
                    .incoming_member
                    .as_deref()
                    .is_some_and(|member| text(member, 32767)),
                ArchiveJobPromptKind::CreateOverwrite => self.incoming_member.is_none(),
                _ => false,
            }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchiveJobPrompt {
    pub ticket: ArchiveJobTicket,
    pub kind: ArchiveJobPromptKind,
    pub error_code: Option<String>,
    pub attempt: u32,
    pub existing_bytes: Option<u64>,
    pub incoming_bytes: Option<u64>,
    pub approved_limit: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display: Option<ArchiveJobPromptDisplay>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchiveJobCommit {
    pub ordinal: u64,
    pub state: String,
    pub replaced: bool,
    pub bytes: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub native_identity: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub published: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cleanup: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchiveJobCommitOutcome {
    pub state: String,
    pub published: Option<bool>,
    pub cleanup: String,
    pub files: u64,
    pub bytes: u64,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchiveJobJournalSummary {
    pub total_records: u64,
    pub detail_complete: bool,
    pub outcomes: Vec<ArchiveJobCommitOutcome>,
}

/// Native committed-output location. Display/Shell use only; never grants write authority.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchiveJobResultLocation {
    pub target: String,
    pub action: crate::ShellOpenAction,
}
impl std::fmt::Debug for ArchiveJobResultLocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ArchiveJobResultLocation([PRIVATE])")
    }
}
impl ArchiveJobResultLocation {
    fn valid(&self) -> bool {
        valid_job_path(&self.target)
            && matches!(
                self.action,
                crate::ShellOpenAction::Open | crate::ShellOpenAction::SelectInExplorer
            )
    }
}
/// Display-only affected Add members. Never use these names as native output authority.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchiveJobAddMember {
    pub path: String,
    pub outcome: ArchiveJobAddOutcome,
}
impl std::fmt::Debug for ArchiveJobAddMember {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ArchiveJobAddMember([PRIVATE])")
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArchiveJobAddOutcome {
    Added,
    Overwritten,
    Renamed,
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchiveJobAddResult {
    pub total: u64,
    pub members: Vec<ArchiveJobAddMember>,
}
impl std::fmt::Debug for ArchiveJobAddResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ArchiveJobAddResult")
            .field("total", &self.total)
            .field("members_count", &self.members.len())
            .finish()
    }
}
impl ArchiveJobAddResult {
    pub fn valid(&self) -> bool {
        self.total > 0
            && self.total >= self.members.len() as u64
            && self.members.len() <= 256
            && self
                .members
                .iter()
                .try_fold(0usize, |sum, member| {
                    let path = member.path.as_str();
                    let path = path.strip_suffix('/').unwrap_or(path);
                    (member.path.len() <= 32767
                        && !path.is_empty()
                        && path.split('/').all(valid_job_name))
                    .then(|| sum.checked_add(member.path.len()))
                    .flatten()
                })
                .is_some_and(|bytes| bytes <= 65536)
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchiveJobTerminal {
    pub outcome: ArchiveJobOutcome,
    pub code: Option<String>,
    pub files_committed: u64,
    pub bytes_committed: u64,
    pub files_verified: u64,
    pub bytes_verified: u64,
    pub files_skipped: u64,
    pub cleanup_complete: bool,
    pub commit_state: Option<String>,
    pub elapsed_ms: u64,
    pub native_error_code: Option<i32>,
    pub journal: Vec<ArchiveJobCommit>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_shortfall: Option<ArchiveJobOutputShortfall>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result_location: Option<ArchiveJobResultLocation>,
    /// Native read-only publication fact, including directories. Missing/null is unknown, not zero.
    /// Does not grant execution, commit, cleanup or retry authority.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_published: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub add_result: Option<ArchiveJobAddResult>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub journal_summary: Option<ArchiveJobJournalSummary>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchiveJobEvent {
    pub version: u32,
    pub operation_id: Uuid,
    pub session_id: Uuid,
    pub generation: u64,
    pub kind: ArchiveJobEventKind,
    pub state: Option<ArchiveJobState>,
    pub progress: Option<ArchiveJobProgress>,
    pub prompt: Option<ArchiveJobPrompt>,
    pub terminal: Option<ArchiveJobTerminal>,
}
impl ArchiveJobEvent {
    pub fn validate(&self) -> Result<(), ArchiveReadError> {
        if self.version != 1
            || self.operation_id.is_nil()
            || self.session_id.is_nil()
            || self.generation == 0
        {
            return Err(ArchiveReadError::ProtocolViolation);
        }
        let shape = match self.kind {
            ArchiveJobEventKind::Accepted | ArchiveJobEventKind::Started => {
                self.state.is_none()
                    && self.progress.is_none()
                    && self.prompt.is_none()
                    && self.terminal.is_none()
            }
            ArchiveJobEventKind::State => {
                self.state.is_some()
                    && self.progress.is_none()
                    && self.prompt.is_none()
                    && self.terminal.is_none()
            }
            ArchiveJobEventKind::Progress => {
                self.state.is_none()
                    && self.progress.as_ref().is_some_and(|p| {
                        p.sequence != Some(0)
                            && matches!(
                                p.phase.as_str(),
                                "running"
                                    | "edit_planning"
                                    | "edit_reading"
                                    | "edit_reencoding"
                                    | "edit_verifying"
                                    | "edit_committing"
                            )
                            && p.current_member.as_ref().is_none_or(|member| {
                                !member.is_empty()
                                    && member.len() <= 32767
                                    && !member.chars().any(char::is_control)
                            })
                            && (p.current_bytes_done.is_none() && p.current_bytes_total.is_none()
                                || p.current_member.is_some()
                                    && p.current_bytes_total.is_none_or(|total| {
                                        p.current_bytes_done.is_some_and(|done| done <= total)
                                    }))
                    })
                    && self.prompt.is_none()
                    && self.terminal.is_none()
            }
            ArchiveJobEventKind::Prompt => {
                self.state.is_none()
                    && self.progress.is_none()
                    && self.prompt.as_ref().is_some_and(|p| {
                        p.ticket.operation_id == self.operation_id
                            && !p.ticket.prompt_id.is_nil()
                            && p.ticket.generation > 0
                            && p.attempt <= 3
                            && p.display
                                .as_ref()
                                .is_none_or(|display| display.valid(p.kind))
                    })
                    && self.terminal.is_none()
            }
            ArchiveJobEventKind::Terminal => {
                self.state.is_none()
                    && self.progress.is_none()
                    && self.prompt.is_none()
                    && self.terminal.as_ref().is_some_and(|t| {
                        (if let Some(summary) = &t.journal_summary {
                            valid_journal_summary(t, summary)
                        } else {
                            t.journal.len() <= 4096
                        })
                            && t.result_location
                                .as_ref()
                                .is_none_or(ArchiveJobResultLocation::valid)
                            && t.add_result.as_ref().is_none_or(ArchiveJobAddResult::valid)
                            && (t.add_result.is_none()
                                || t.cleanup_complete
                                    && t.output_published == Some(true)
                                    && t.commit_state.as_deref() == Some("Committed")
                                    && t.files_committed == 1
                                    && matches!(t.journal.as_slice(), [ArchiveJobCommit { state, published: Some(true), .. }] if state.as_str() == "Committed")
                                    && matches!(
                                        t.outcome,
                                        ArchiveJobOutcome::Succeeded
                                            | ArchiveJobOutcome::SucceededWithWarnings
                                    ))
                            && t.code.as_ref().is_none_or(|code| {
                                code.len() <= 64
                                    && code.bytes().all(|b| b.is_ascii_lowercase() || b == b'_')
                            })
                            && t.commit_state
                                .as_ref()
                                .is_none_or(|s| valid_commit_state(s))
                            && t.journal.iter().all(|f| {
                                valid_commit_state(&f.state)
                                    && f.native_identity.as_ref().is_none_or(|id| {
                                        id.len() <= 64
                                            && id.starts_with("v1:")
                                            && id[3..]
                                                .bytes()
                                                .all(|b| b.is_ascii_hexdigit() || b == b':')
                                    })
                                    && f.cleanup.as_ref().is_none_or(|value| {
                                        matches!(
                                            value.as_str(),
                                            "not_requested"
                                                | "retained"
                                                | "retained_replacement"
                                                | "retained_uncertain"
                                                | "retained_published"
                                                | "partial_removed"
                                                | "new_output_removed"
                                                | "changed_retained"
                                                | "cleanup_failed"
                                        )
                                    })
                            })
                    })
            }
        };
        if shape {
            Ok(())
        } else {
            Err(ArchiveReadError::ProtocolViolation)
        }
    }
}
fn valid_journal_summary(t: &ArchiveJobTerminal, summary: &ArchiveJobJournalSummary) -> bool {
    use std::collections::{HashMap, HashSet};
    if summary.total_records > 1_000_000
        || summary.outcomes.len() > 256
        || summary.outcomes.is_empty() != (summary.total_records == 0)
        || t.journal.len() > 256
        || summary.detail_complete != (summary.total_records == t.journal.len() as u64)
        || !summary.detail_complete && summary.total_records <= t.journal.len() as u64
    {
        return false;
    }
    let mut groups = HashMap::new();
    let mut files = 0u64;
    let mut bytes = 0u64;
    let mut published_files = 0u64;
    let mut published_bytes = 0u64;
    for group in &summary.outcomes {
        if !valid_commit_state(&group.state)
            || !valid_cleanup(&group.cleanup)
            || group.files == 0
            || groups
                .insert(
                    (&group.state, group.published, &group.cleanup),
                    (group.files, group.bytes),
                )
                .is_some()
        {
            return false;
        }
        let Some(next_files) = files.checked_add(group.files) else {
            return false;
        };
        let Some(next_bytes) = bytes.checked_add(group.bytes) else {
            return false;
        };
        bytes = next_bytes;
        files = next_files;
        if group.published == Some(true) {
            let (Some(next_files), Some(next_bytes)) = (
                published_files.checked_add(group.files),
                published_bytes.checked_add(group.bytes),
            ) else {
                return false;
            };
            published_files = next_files;
            published_bytes = next_bytes;
        }
    }
    if files != summary.total_records
        || published_files != t.files_committed
        || published_bytes != t.bytes_committed
    {
        return false;
    }
    let mut ordinals = HashSet::new();
    let mut details: HashMap<(&String, Option<bool>, &String), (u64, u64)> = HashMap::new();
    for detail in &t.journal {
        let Some(cleanup) = detail.cleanup.as_ref() else {
            return false;
        };
        if detail.ordinal >= summary.total_records || !ordinals.insert(detail.ordinal) {
            return false;
        }
        let key = (&detail.state, detail.published, cleanup);
        let Some(&(limit_files, limit_bytes)) = groups.get(&key) else {
            return false;
        };
        let entry = details.entry(key).or_default();
        let (Some(next_files), Some(next_bytes)) =
            (entry.0.checked_add(1), entry.1.checked_add(detail.bytes))
        else {
            return false;
        };
        *entry = (next_files, next_bytes);
        if next_files > limit_files || next_bytes > limit_bytes {
            return false;
        }
    }
    !summary.detail_complete
        || groups
            .iter()
            .all(|(key, counts)| details.get(key).copied() == Some(*counts))
}

fn valid_cleanup(value: &str) -> bool {
    matches!(
        value,
        "not_requested"
            | "retained"
            | "retained_replacement"
            | "retained_uncertain"
            | "retained_published"
            | "partial_removed"
            | "new_output_removed"
            | "changed_retained"
            | "cleanup_failed"
    )
}

fn valid_commit_state(state: &str) -> bool {
    matches!(
        state,
        "Preparing"
            | "Verified"
            | "CommitInFlight"
            | "FailedBeforeCommit"
            | "Committed"
            | "CommittedButVerificationFailed"
            | "TargetChangedAfterCommit"
            | "CommitOutcomeUnknown"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn large_count_terminal_wire_validates_partial_and_complete_native_outcomes() {
        let wire = serde_json::json!({
            "version": 1, "operation_id": Uuid::new_v4(), "session_id": Uuid::new_v4(),
            "generation": 1, "kind": "terminal", "terminal": {
                "outcome": "cancelled", "code": "cancelled_with_committed_output",
                "files_committed": 1, "bytes_committed": 3, "files_verified": 0,
                "bytes_verified": 0, "files_skipped": 0, "cleanup_complete": true,
                "commit_state": "CommitOutcomeUnknown", "elapsed_ms": 10,
                "native_error_code": null, "journal": [{"ordinal": 2, "state": "CommitOutcomeUnknown",
                    "replaced": false, "bytes": 0, "published": null, "cleanup": "retained_uncertain"}],
                "journal_summary": {"total_records": 3, "detail_complete": false, "outcomes": [
                    {"state": "Committed", "published": true, "cleanup": "retained", "files": 1, "bytes": 3},
                    {"state": "FailedBeforeCommit", "published": false, "cleanup": "partial_removed", "files": 1, "bytes": 0},
                    {"state": "CommitOutcomeUnknown", "published": null, "cleanup": "retained_uncertain", "files": 1, "bytes": 0}
                ]}
            }
        });
        let parse = |value: serde_json::Value| {
            serde_json::from_value::<ArchiveJobEvent>(value).expect("terminal wire")
        };
        let event = parse(wire.clone());
        assert!(event.validate().is_ok());
        let mut legacy = wire.clone();
        legacy["terminal"]
            .as_object_mut()
            .expect("terminal")
            .remove("journal_summary");
        assert!(parse(legacy.clone()).validate().is_ok());
        assert!(
            serde_json::to_value(parse(legacy)).expect("legacy encode")["terminal"]
                .get("journal_summary")
                .is_none()
        );
        let mut full = wire.clone();
        full["terminal"]["journal"] = serde_json::json!([
            {"ordinal":0,"state":"Committed","replaced":false,"bytes":3,"published":true,"cleanup":"retained"},
            {"ordinal":1,"state":"FailedBeforeCommit","replaced":false,"bytes":0,"published":false,"cleanup":"partial_removed"},
            {"ordinal":2,"state":"CommitOutcomeUnknown","replaced":false,"bytes":0,"published":null,"cleanup":"retained_uncertain"}
        ]);
        full["terminal"]["journal_summary"]["detail_complete"] = serde_json::json!(true);
        assert!(parse(full.clone()).validate().is_ok());
        for (path, value) in [
            ("total_records", serde_json::json!(2)),
            ("detail_complete", serde_json::json!(true)),
            ("outcomes", serde_json::json!([])),
        ] {
            let mut bad = wire.clone();
            bad["terminal"]["journal_summary"][path] = value;
            assert!(parse(bad).validate().is_err(), "invalid {path}");
        }
        for (path, value) in [
            ("files", serde_json::json!(2)),
            ("bytes", serde_json::json!(4)),
            ("state", serde_json::json!("Unknown")),
            ("cleanup", serde_json::json!("missing")),
        ] {
            let mut bad = wire.clone();
            bad["terminal"]["journal_summary"]["outcomes"][0][path] = value;
            assert!(parse(bad).validate().is_err(), "invalid bucket {path}");
        }
        let mut duplicate = wire.clone();
        let copied = duplicate["terminal"]["journal_summary"]["outcomes"][0].clone();
        duplicate["terminal"]["journal_summary"]["outcomes"][1] = copied;
        assert!(parse(duplicate).validate().is_err());
        full["terminal"]["journal"][1]["bytes"] = serde_json::json!(1);
        assert!(
            parse(full).validate().is_err(),
            "complete bucket bytes must agree"
        );
        let mut oversized = wire;
        oversized["terminal"]["journal"] = serde_json::json!(vec![
            serde_json::json!({"ordinal":0,"state":"Committed","replaced":false,"bytes":0,"cleanup":"retained","published":true});
            257
        ]);
        assert!(parse(oversized).validate().is_err(), "partial detail bound");
        let mut ordinal = serde_json::to_value(&event).expect("detail wire");
        ordinal["terminal"]["journal"][0]["ordinal"] = serde_json::json!(3);
        assert!(
            parse(ordinal).validate().is_err(),
            "detail ordinal outside complete native history"
        );
        let mut huge = serde_json::to_value(&event).expect("summary wire");
        huge["terminal"]["journal_summary"]["total_records"] = serde_json::json!(1_000_001);
        assert!(parse(huge).validate().is_err(), "summary total bound");
        let mut bounded = event.clone();
        let terminal = bounded.terminal.as_mut().expect("terminal");
        terminal.files_committed = 256;
        terminal.bytes_committed = 256;
        terminal.journal = (0..256)
            .map(|ordinal| ArchiveJobCommit {
                ordinal,
                state: "Committed".into(),
                replaced: false,
                bytes: 1,
                native_identity: Some(format!("v1:{}", "a".repeat(60))),
                published: Some(true),
                cleanup: Some("retained".into()),
            })
            .collect();
        terminal.journal_summary = Some(ArchiveJobJournalSummary {
            total_records: 256,
            detail_complete: true,
            outcomes: vec![ArchiveJobCommitOutcome {
                state: "Committed".into(),
                published: Some(true),
                cleanup: "retained".into(),
                files: 256,
                bytes: 256,
            }],
        });
        assert!(bounded.validate().is_ok());
        assert!(
            serde_json::to_vec(&bounded)
                .expect("bounded full detail")
                .len()
                < crate::MAX_FRAME_BYTES
        );
        let encoded = serde_json::to_vec(&event).expect("bounded event");
        assert!(encoded.len() < crate::MAX_FRAME_BYTES);
    }

    #[test]
    fn opt01_progress_phase_wire_vocabulary_and_malformed_frames() {
        for phase in [
            "running",
            "edit_planning",
            "edit_reading",
            "edit_reencoding",
            "edit_verifying",
            "edit_committing",
        ] {
            let value = serde_json::json!({"version":1,"operation_id":Uuid::new_v4(),"session_id":Uuid::new_v4(),"generation":1,"kind":"progress","progress":{"phase":phase,"bytes_done":3,"bytes_total":10,"files_done":0,"files_total":1}});
            let encoded = serde_json::to_vec(&value).expect("serialize wire frame");
            let event: ArchiveJobEvent =
                serde_json::from_slice(&encoded).expect("deserialize wire frame");
            assert!(event.validate().is_ok(), "supported phase {phase}");
            assert_eq!(
                event.progress.as_ref().expect("legacy progress").sequence,
                None
            );
            assert!(
                serde_json::to_value(&event).expect("legacy event encode")["progress"]
                    .get("sequence")
                    .is_none()
            );
            for sequence in [1, u64::MAX] {
                let mut ordered = event.clone();
                ordered
                    .progress
                    .as_mut()
                    .expect("ordered progress")
                    .sequence = Some(sequence);
                assert!(ordered.validate().is_ok());
                assert_eq!(
                    serde_json::from_value::<ArchiveJobEvent>(
                        serde_json::to_value(&ordered).expect("ordered event encode")
                    )
                    .expect("ordered event decode"),
                    ordered
                );
            }
            let mut zero = event.clone();
            zero.progress
                .as_mut()
                .expect("zero sequence progress")
                .sequence = Some(0);
            assert!(zero.validate().is_err());
            for malformed in [
                serde_json::json!(-1),
                serde_json::json!(1.5),
                serde_json::json!("1"),
            ] {
                let mut invalid = value.clone();
                invalid["progress"]["sequence"] = malformed;
                assert!(serde_json::from_value::<ArchiveJobEvent>(invalid).is_err());
            }
            for invalid in ["", "edit_unknown", "RUNNING", "edit_reading\n"] {
                let mut bad = event.clone();
                bad.progress.as_mut().expect("progress").phase = invalid.into();
                assert!(bad.validate().is_err(), "unknown phase {invalid:?}");
            }
            let mut bad = event.clone();
            bad.state = Some(ArchiveJobState::Running);
            assert!(bad.validate().is_err(), "progress cannot carry state");
            let mut bad = event.clone();
            let progress = bad.progress.as_mut().expect("progress");
            progress.current_member = Some("owned.txt".into());
            progress.current_bytes_done = Some(11);
            progress.current_bytes_total = Some(10);
            assert!(bad.validate().is_err(), "phase does not relax byte bounds");
        }
    }
    #[test]
    fn job_rejects_hidden_authority_and_inapplicable_create_options() {
        let mut value = serde_json::json!({"version":1,"operation_id":Uuid::new_v4(),"session_id":Uuid::new_v4(),"generation":1,"action":"start","request":{"kind":"create","paths":["C:\\fixture\\a"],"destination":"C:\\fixture\\out","create":{"format":"tar","level":null,"method":null,"password":null,"solid":null,"encrypt_header":null,"strip_single_root":false,"exclude_patterns":[],"test_after_create":false,"name":"out.tar"},"extract":null}});
        assert!(
            serde_json::from_value::<ArchiveJobCommand>(value.clone())
                .expect("contract")
                .validate()
                .is_ok()
        );
        value["request"]["create"]["level"] = serde_json::json!("normal");
        assert!(
            serde_json::from_value::<ArchiveJobCommand>(value.clone())
                .expect("shape")
                .validate()
                .is_err()
        );
        value["request"]["delete_source"] = serde_json::json!(true);
        assert!(serde_json::from_value::<ArchiveJobCommand>(value).is_err());
    }
    #[test]
    fn destination_and_create_overwrite_reply_only_confirm_or_cancel() {
        for kind in [
            ArchiveJobPromptKind::CreateOverwrite,
            ArchiveJobPromptKind::Destination,
        ] {
            let mut reply = ArchiveJobReply {
                ticket: ArchiveJobTicket {
                    operation_id: Uuid::new_v4(),
                    prompt_id: Uuid::new_v4(),
                    generation: 7,
                },
                kind,
                password: None,
                decision: Some(ArchiveJobConflictDecision::Overwrite),
                apply_to_all: false,
                approved_limit: None,
                remember_batch_password: false,
            };
            assert!(reply.validate().is_ok());
            reply.decision = None;
            assert!(reply.validate().is_ok());
            reply.decision = Some(ArchiveJobConflictDecision::Skip);
            assert!(reply.validate().is_err());
            reply.decision = Some(ArchiveJobConflictDecision::AutoRename);
            assert!(reply.validate().is_err());
            reply.decision = Some(ArchiveJobConflictDecision::Overwrite);
            reply.apply_to_all = true;
            assert!(reply.validate().is_err());
            reply.apply_to_all = false;
            reply.approved_limit = Some(1);
            assert!(reply.validate().is_err());
            reply.approved_limit = None;
            reply.ticket.generation = 0;
            assert!(reply.validate().is_err());
        }
    }
    #[test]
    fn job_names_and_limits_are_finite() {
        for name in ["", "../x", "NUL.zip", "COM1", "bad.", "bad "] {
            assert!(!valid_job_name(name));
        }
        assert!(ArchiveJobLimits::default().validate().is_ok());
        assert!(
            ArchiveJobLimits {
                max_entries: 1_000_001,
                ..Default::default()
            }
            .validate()
            .is_err()
        );
    }
    #[test]
    fn zip_modify_is_bound_and_legacy_fields_omit() {
        use crate::archive_browser::{ArchiveBrowserContext, ArchiveSourceBinding};
        let browser = ArchiveBrowserContext {
            snapshot_id: Uuid::new_v4(),
            format: "Zip".into(),
            comment: String::new(),
            source: ArchiveSourceBinding {
                native: "v1:fixture".into(),
                digest: [1; 32],
                size: 1,
                modified_unix_ns: 1,
            },
            sevenz_edit: None,
            tar_edit: None,
        };
        let mut request: ArchiveJobRequest = serde_json::from_value(serde_json::json!({
            "kind":"test", "paths":["C:\\owned\\a.zip"],"destination":null,"extract":null,"create":null
        })).expect("legacy job");
        assert!(
            serde_json::to_value(&request)
                .expect("wire")
                .get("modify")
                .is_none()
        );
        request.kind = ArchiveJobKind::Modify;
        request.modify = Some(ArchiveJobModify {
            browser,
            action: ArchiveJobModifyAction::Delete,
            ordinals: vec![0, 1],
            value: None,
            source_prefix: None,
        });
        assert!(request.validate().is_ok());
        request.modify.as_mut().expect("edit").ordinals.push(1);
        assert!(request.validate().is_err());
        request.modify.as_mut().expect("edit").ordinals = vec![];
        request.modify.as_mut().expect("edit").action = ArchiveJobModifyAction::Add;
        assert!(request.validate().is_err());
        request.paths.push("C:\\owned\\new.txt".into());
        assert!(request.validate().is_ok());
        request.modify.as_mut().expect("edit").value = Some("../escape/".into());
        assert!(request.validate().is_err());
        assert!(!valid_modify_target("CON/file.txt", false));
        assert!(valid_modify_target("目录/new/", true));
        request.modify.as_mut().expect("edit").action = ArchiveJobModifyAction::Comment;
        request.paths.truncate(1);
        request.modify.as_mut().expect("edit").value = Some("x".repeat(65536));
        assert!(request.validate().is_err());
        request.modify.as_mut().expect("edit").value = Some(String::new());
        assert!(request.validate().is_ok());
    }
    #[test]
    fn per_output_journal_retains_optional_legacy_and_terminal_facts() {
        let old = serde_json::json!({"ordinal":0,"state":"Committed","replaced":false,"bytes":7});
        let mut fact: ArchiveJobCommit =
            serde_json::from_value(old.clone()).expect("legacy journal");
        assert_eq!(serde_json::to_value(&fact).expect("legacy omission"), old);
        fact.native_identity = Some("v1:1:00000000000000000000000000000002".into());
        fact.published = Some(false);
        fact.cleanup = Some("partial_removed".into());
        assert_eq!(
            serde_json::from_value::<ArchiveJobCommit>(
                serde_json::to_value(&fact).expect("new wire")
            )
            .expect("new journal"),
            fact
        );
    }
    #[test]
    fn extraction_member_counts_require_identity_and_known_total_bounds() {
        let mut event: ArchiveJobEvent = serde_json::from_value(serde_json::json!({
            "version":1,"operation_id":Uuid::new_v4(),"session_id":Uuid::new_v4(),"generation":1,
            "kind":"progress","state":null,"prompt":null,"terminal":null,
            "progress":{"phase":"running","bytes_done":40,"bytes_total":30,
                "files_done":0,"files_total":2,"current_member":"one.txt",
                "current_bytes_done":4,"current_bytes_total":10}
        }))
        .expect("dual wire");
        assert!(
            event.validate().is_ok(),
            "processed aggregate may exceed declared total"
        );
        let progress = event.progress.as_mut().expect("progress");
        progress.current_bytes_done = Some(11);
        assert!(event.validate().is_err(), "known member cannot overrun");
        let progress = event.progress.as_mut().expect("progress");
        progress.current_bytes_total = None;
        assert!(
            event.validate().is_ok(),
            "unknown total keeps actual member bytes"
        );
        event.progress.as_mut().expect("progress").current_member = None;
        assert!(event.validate().is_err(), "count requires member identity");
        let progress = event.progress.as_mut().expect("progress");
        progress.current_bytes_done = None;
        assert!(event.validate().is_ok(), "legacy aggregate-only event");
        event
            .progress
            .as_mut()
            .expect("progress")
            .current_bytes_total = Some(10);
        assert!(
            event.validate().is_err(),
            "known total needs a current done count"
        );
    }
    #[test]
    fn feedback_location_and_member_optional_wire_is_bounded_and_private() {
        let mut progress:ArchiveJobProgress=serde_json::from_value(serde_json::json!({"phase":"running","bytes_done":0,"bytes_total":null,"files_done":0,"files_total":null})).expect("legacy progress");
        assert!(progress.current_member.is_none());
        assert!(progress.current_bytes_done.is_none() && progress.current_bytes_total.is_none());
        progress.current_member = Some("member.txt".into());
        progress.current_bytes_done = Some(3);
        progress.current_bytes_total = Some(10);
        assert_eq!(
            serde_json::from_value::<ArchiveJobProgress>(
                serde_json::to_value(&progress).expect("wire")
            )
            .expect("roundtrip"),
            progress
        );
        let location = ArchiveJobResultLocation {
            target: r"C:\owned\actual (2)".into(),
            action: crate::ShellOpenAction::Open,
        };
        assert!(location.valid());
        assert!(!format!("{location:?}").contains("actual"));
        assert!(
            !ArchiveJobResultLocation {
                action: crate::ShellOpenAction::RunElevated,
                ..location.clone()
            }
            .valid()
        );
        assert!(
            !ArchiveJobResultLocation {
                target: "https://example.com".into(),
                ..location
            }
            .valid()
        );
    }
}

#[cfg(test)]
mod prompt_display_tests {
    use super::*;
    fn event() -> ArchiveJobEvent {
        let id = Uuid::new_v4();
        serde_json::from_value(serde_json::json!({
            "version":1,"operation_id":id,"session_id":Uuid::new_v4(),"generation":1,
            "kind":"prompt","state":null,"progress":null,"terminal":null,
            "prompt":{"ticket":{"operation_id":id,"prompt_id":Uuid::new_v4(),"generation":1},
                "kind":"conflict","error_code":null,"attempt":0,"existing_bytes":10,
                "incoming_bytes":10,"approved_limit":null}
        }))
        .expect("legacy typed prompt event")
    }
    #[test]
    fn audit04_legacy_omission_strict_shape_bounds_and_redacted_debug() {
        let mut e = event();
        assert!(e.validate().is_ok());
        assert!(e.prompt.as_ref().expect("prompt").display.is_none());
        assert!(
            serde_json::to_value(&e).expect("legacy event wire")["prompt"]
                .get("display")
                .is_none()
        );
        let sentinel = "private-prompt-object-资料/同名.txt";
        e.prompt.as_mut().expect("prompt").display = Some(ArchiveJobPromptDisplay {
            incoming_member: Some(sentinel.into()),
            target: format!(r"C:\owned\{sentinel}"),
        });
        assert!(e.validate().is_ok());
        let wire = serde_json::to_value(&e).expect("display event wire");
        assert_eq!(
            serde_json::from_value::<ArchiveJobEvent>(wire.clone()).expect("display roundtrip"),
            e
        );
        assert!(!format!("{e:?}").contains(sentinel));
        for field in ["target", "incoming_member"] {
            let mut bad = wire.clone();
            bad["prompt"]["display"][field] = serde_json::json!(5);
            assert!(serde_json::from_value::<ArchiveJobEvent>(bad).is_err());
        }
        let mut unknown = wire;
        unknown["prompt"]["display"]["authority"] = serde_json::json!(true);
        assert!(serde_json::from_value::<ArchiveJobEvent>(unknown).is_err());
        for invalid in [String::new(), "line\nbreak".into(), "x".repeat(65536)] {
            e.prompt
                .as_mut()
                .expect("prompt")
                .display
                .as_mut()
                .expect("display")
                .target = invalid;
            assert!(e.validate().is_err());
        }
        e.prompt
            .as_mut()
            .expect("prompt")
            .display
            .as_mut()
            .expect("display")
            .target = "x".repeat(65535);
        assert!(e.validate().is_ok());
        e.prompt.as_mut().expect("prompt").kind = ArchiveJobPromptKind::Password;
        assert!(
            e.validate().is_err(),
            "non-overwrite prompts cannot carry display"
        );
        e.prompt.as_mut().expect("prompt").kind = ArchiveJobPromptKind::CreateOverwrite;
        assert!(
            e.validate().is_err(),
            "create has no incoming archive member"
        );
        e.prompt
            .as_mut()
            .expect("prompt")
            .display
            .as_mut()
            .expect("display")
            .incoming_member = None;
        assert!(e.validate().is_ok());
    }
}

#[cfg(test)]
mod capability_foundation_tests {
    use super::*;
    #[test]
    fn capability_foundation_existing_request_resolves_distinct_rename_intents_without_wire_expansion()
     {
        let mut request: ArchiveJobModify = serde_json::from_value(serde_json::json!({
            "browser":{"snapshot_id":Uuid::new_v4(),"format":"Zip","comment":"",
                "source":{"native":"v1:fixture","digest":vec![1u8;32],"size":1,"modified_unix_ns":1}},
            "action":"rename","ordinals":[7],"value":"file.txt"
        })).expect("original request shape");
        assert_eq!(request.operation(), ArchiveOperation::RenameFile);
        assert!(request.validate(1).is_ok());
        request.value = Some("folder/".into());
        assert_eq!(
            request.operation(),
            ArchiveOperation::RenameDirectorySubtree
        );
        assert!(request.validate(1).is_ok());
        let wire = serde_json::to_value(&request).expect("legacy compatible wire");
        assert_eq!(wire["action"], "rename");
        assert!(wire.get("operation").is_none());
        for format in [
            "TarBz2", "TarZstd", "Rar4", "Rar5", "Gzip", "Bzip2", "Xz", "Zstd",
        ] {
            request.browser.format = format.into();
            assert_eq!(
                request.backend_refusal(),
                Some(ArchiveModificationRefusal::BackendMissing)
            );
            assert!(
                request.validate(1).is_err(),
                "no unimplemented write request gets admitted"
            );
        }
        let mut unsupported = wire;
        unsupported["action"] = serde_json::json!("replace_content");
        let replacement: ArchiveJobModify =
            serde_json::from_value(unsupported).expect("new typed action");
        assert!(
            replacement.validate(2).is_err(),
            "ZIP replacement is still unavailable"
        );
    }
}

#[cfg(test)]
mod sevenz_edit_tests {
    use super::*;
    #[test]
    fn sevenz_replace_is_a_distinct_bounded_action_and_never_grants_zip_replacement() {
        let wire = serde_json::json!({"browser":{"snapshot_id":Uuid::new_v4(),"format":"SevenZ","comment":"","source":{"native":"v1:fixture","digest":vec![1u8;32],"size":1,"modified_unix_ns":1}},"action":"replace_content","ordinals":[2],"value":null});
        let mut intent: ArchiveJobModify =
            serde_json::from_value(wire).expect("real typed replacement action");
        assert_eq!(intent.operation(), ArchiveOperation::ReplaceContent);
        assert!(intent.validate(2).is_ok());
        for paths in [0, 1, 3, 33] {
            assert!(intent.validate(paths).is_err());
        }
        intent.browser.format = "Zip".into();
        assert!(intent.validate(2).is_err());
    }
}

#[cfg(test)]
mod hz3_publication_tests {
    use super::*;
    #[test]
    fn hz3_optional_native_publication_is_legacy_unknown_and_strict_bool() {
        let legacy = serde_json::json!({"outcome":"failed","code":"io_failure","files_committed":0,"bytes_committed":0,"files_verified":0,"bytes_verified":0,"files_skipped":0,"cleanup_complete":true,"commit_state":null,"elapsed_ms":1,"native_error_code":null,"journal":[]});
        let terminal: ArchiveJobTerminal =
            serde_json::from_value(legacy.clone()).expect("legacy terminal");
        assert_eq!(terminal.output_published, None);
        assert!(
            serde_json::to_value(terminal)
                .expect("legacy encoding")
                .get("output_published")
                .is_none()
        );
        for value in [
            serde_json::Value::Null,
            serde_json::json!(false),
            serde_json::json!(true),
        ] {
            let mut wire = legacy.clone();
            wire["output_published"] = value.clone();
            let terminal: ArchiveJobTerminal =
                serde_json::from_value(wire).expect("optional native fact");
            assert_eq!(terminal.output_published, value.as_bool());
        }
        for value in [
            serde_json::json!(0),
            serde_json::json!("false"),
            serde_json::json!({}),
            serde_json::json!([]),
        ] {
            let mut wire = legacy.clone();
            wire["output_published"] = value;
            assert!(serde_json::from_value::<ArchiveJobTerminal>(wire).is_err());
        }
        let mut unknown = legacy;
        unknown["publication_permission"] = serde_json::json!(true);
        assert!(
            serde_json::from_value::<ArchiveJobTerminal>(unknown).is_err(),
            "facts do not grant new authority"
        );
    }
}

#[cfg(test)]
#[test]
fn opt03_add_result_wire_is_optional_bounded_private_and_publication_gated() {
    use serde_json::json;
    let base = json!({"version":1,"operation_id":Uuid::new_v4(),"session_id":Uuid::new_v4(),
        "generation":1,"kind":"terminal","terminal":{"outcome":"succeeded","code":null,
        "files_committed":1,"bytes_committed":10,"files_verified":1,"bytes_verified":10,
        "files_skipped":0,"cleanup_complete":true,"commit_state":"Committed","elapsed_ms":1,
        "native_error_code":null,"journal":[{"ordinal":0,"state":"Committed","replaced":true,"bytes":10,"published":true}],"output_published":true}});
    let legacy: ArchiveJobEvent = serde_json::from_value(base.clone()).expect("legacy event");
    assert!(legacy.validate().is_ok());
    assert!(
        serde_json::to_value(&legacy).expect("serialize legacy")["terminal"]
            .get("add_result")
            .is_none()
    );
    let mut wire = base;
    wire["terminal"]["add_result"] = json!({"total":300,"members":[
        {"path":"folder/new (2).txt","outcome":"renamed"},
        {"path":"folder/old.txt","outcome":"overwritten"}]});
    let event: ArchiveJobEvent = serde_json::from_value(wire.clone()).expect("strict Add wire");
    assert!(event.validate().is_ok());
    assert!(
        !format!("{event:?}").contains("folder/new"),
        "private Debug"
    );
    assert_eq!(
        serde_json::to_value(&event).expect("serialize")["terminal"]["add_result"],
        wire["terminal"]["add_result"]
    );
    for bad in [
        json!({"total":1,"members":[{"path":"ok.txt","outcome":"added"},{"path":"two.txt","outcome":"added"}]}),
        json!({"total":1,"members":[{"path":"../escape","outcome":"added"}]}),
        json!({"total":1,"members":[{"path":"ok.txt","outcome":"skip"}]}),
    ] {
        let mut invalid = wire.clone();
        invalid["terminal"]["add_result"] = bad;
        let parsed = serde_json::from_value::<ArchiveJobEvent>(invalid);
        assert!(parsed.is_err() || parsed.expect("typed shape").validate().is_err());
    }
    let mut overflow = wire.clone();
    overflow["terminal"]["add_result"] =
        json!({"total":257,"members":vec![json!({"path":"item.txt","outcome":"added"});257]});
    assert!(
        serde_json::from_value::<ArchiveJobEvent>(overflow)
            .expect("typed overflow")
            .validate()
            .is_err()
    );
    for (field, value) in [
        ("output_published", json!(false)),
        ("cleanup_complete", json!(false)),
        ("outcome", json!("failed")),
        ("commit_state", json!("CommitOutcomeUnknown")),
    ] {
        let mut invalid = wire.clone();
        invalid["terminal"][field] = value;
        assert!(
            serde_json::from_value::<ArchiveJobEvent>(invalid)
                .expect("typed event")
                .validate()
                .is_err()
        );
    }
    wire["terminal"]["add_result"]["other"] = serde_json::Value::Bool(true);
    assert!(serde_json::from_value::<ArchiveJobEvent>(wire).is_err());
}

#[cfg(test)]
#[test]
fn windows_drive_root_is_a_valid_archive_destination() {
    assert!(valid_job_path(r"C:\"));
    assert!(valid_job_path(r"d:\"));
    assert!(!valid_job_path("C:"));
    assert!(!valid_job_path("C:/"));
}

#[cfg(test)]
#[test]
fn opt07_batch_password_fields_preserve_legacy_and_bind_explicit_consent() {
    use serde_json::json;
    let operation = Uuid::new_v4();
    let batch = Uuid::new_v4();
    let wire = json!({"version":1,"operation_id":operation,"session_id":operation,"generation":1,
        "action":"reply","reply":{"ticket":{"operation_id":operation,"prompt_id":Uuid::new_v4(),"generation":1},
        "kind":"password","password":null,"decision":null,"apply_to_all":false,"approved_limit":null}});
    let legacy: ArchiveJobCommand = serde_json::from_value(wire).expect("legacy reply");
    assert!(legacy.validate().is_ok());
    assert_eq!(legacy.batch_id, None);
    assert!(
        !legacy
            .reply
            .as_ref()
            .expect("reply")
            .remember_batch_password
    );
    let encoded = serde_json::to_value(&legacy).expect("legacy encoding");
    assert!(encoded.get("batch_id").is_none());
    assert!(encoded["reply"].get("remember_batch_password").is_none());
    let mut current = legacy;
    current.reply.as_mut().expect("reply").password = Some(PrivatePassword::new("fixture-secret"));
    current
        .reply
        .as_mut()
        .expect("reply")
        .remember_batch_password = true;
    assert!(
        current.validate().is_err(),
        "consent requires original batch binding"
    );
    current.batch_id = Some(batch);
    assert!(current.validate().is_ok());
    let encoded = serde_json::to_value(&current).expect("explicit encoding");
    assert_eq!(encoded["reply"]["remember_batch_password"], true);
    assert_eq!(
        serde_json::from_value::<ArchiveJobCommand>(encoded).expect("roundtrip"),
        current
    );
    assert!(!format!("{current:?}").contains("fixture-secret"));
    current.reply.as_mut().expect("reply").password = None;
    assert!(
        current.validate().is_err(),
        "cancellation cannot remember a password"
    );
    current.reply.as_mut().expect("reply").kind = ArchiveJobPromptKind::Conflict;
    assert!(current.validate().is_err(), "consent is password-only");
    let mut retirement = ArchiveJobCommand {
        version: 1,
        operation_id: batch,
        session_id: batch,
        generation: 1,
        action: ArchiveJobAction::RetireBatch,
        request: None,
        reply: None,
        batch_id: Some(batch),
    };
    assert!(retirement.validate().is_ok());
    retirement.session_id = operation;
    assert!(retirement.validate().is_err());
    retirement.session_id = batch;
    retirement.batch_id = Some(operation);
    assert!(retirement.validate().is_err());
}
