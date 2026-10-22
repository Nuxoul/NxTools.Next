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
    Reply
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
    Comment
});
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
}
impl ArchiveJobCommand {
    pub fn validate(&self) -> Result<(), ArchiveReadError> {
        if self.version != ARCHIVE_JOB_VERSION {
            return Err(ArchiveReadError::IncompatibleVersion);
        }
        if self.operation_id.is_nil() || self.session_id.is_nil() || self.generation == 0 {
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
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchiveJobSelection {
    pub browser: ArchiveBrowserContext,
    pub ordinals: Vec<u64>,
}
impl ArchiveJobExtract {
    pub fn validate(&self) -> Result<(), ArchiveReadError> {
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
}
pub fn valid_job_path(path: &str) -> bool {
    let b = path.as_bytes();
    b.len() >= 4
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
impl ArchiveJobModify {
    pub fn validate(&self, path_count: usize) -> Result<(), ArchiveReadError> {
        self.browser.validate()?;
        if self.browser.format != "Zip"
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
}
impl ArchiveJobReply {
    fn validate(&self) -> Result<(), ArchiveReadError> {
        if self.ticket.operation_id.is_nil()
            || self.ticket.prompt_id.is_nil()
            || self.ticket.generation == 0
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
                    && self.progress.as_ref().is_some_and(|p| p.phase == "running")
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
                    })
                    && self.terminal.is_none()
            }
            ArchiveJobEventKind::Terminal => {
                self.state.is_none()
                    && self.progress.is_none()
                    && self.prompt.is_none()
                    && self.terminal.as_ref().is_some_and(|t| {
                        t.journal.len() <= 4096
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
}
