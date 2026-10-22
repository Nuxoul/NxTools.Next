//! Private inherited-pipe protocol for the C#-admitted archive worker.
//! This is not the desktop PlatformHost protocol and is never a diagnostic sink.
use crate::{
    ArchiveEngine, ArchiveError, ArchiveRequest, NameEncoding, OpenRequest, OperationControl,
    ResourceLimits,
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::io::{self, Read, Write};
use uuid::Uuid;
use zeroize::{Zeroize, Zeroizing};

pub const MAX_FRAME_BYTES: usize = 1024 * 1024;

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkerRequest {
    #[serde(rename = "schema")]
    pub version: u32,
    pub operation_id: Uuid,
    pub op: String,
    pub operation: String,
    pub path: String,
    #[serde(default = "auto_codepage")]
    pub codepage: u32,
    #[serde(default)]
    pub password: Option<String>,
    #[serde(default)]
    pub limits: WorkerLimits,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub create_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compression_level: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zip_options: Option<ZipCreateOptions>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tree_options: Option<crate::creation::CreationOptions>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sevenz_options: Option<SevenzCreateOptions>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extract_options: Option<ExtractOptions>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delete_source: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub read_contract: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub read_transport: Option<ReadTransport>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub password_interaction: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conflict_policy: Option<crate::worker_conflict::ConflictPolicy>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_readmission: Option<crate::output_readmission::ReadmissionOptions>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub browser_entry: Option<crate::worker_browser::BrowserEntry>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub job_extract: Option<nxtools_protocol::archive_job::ArchiveJobExtract>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub job_execution: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub job_modify: Option<nxtools_protocol::archive_job::ArchiveJobModify>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub volume_bytes: Option<u64>,
    /// Filled ONLY from the authenticated original native source-acquire reply, never input JSON.
    #[serde(skip)]
    pub volume_paths: Vec<String>,
    #[cfg(debug_assertions)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zip_risk_fixture: Option<crate::zip_risk::FixtureThresholds>,
}

#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReadTransport {
    pub max_total_bytes: u64,
    #[serde(default, skip_serializing_if = "read_browser_disabled")]
    pub browser_context: bool,
}
fn read_browser_disabled(value: &bool) -> bool {
    !*value
}
#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ReadUpgrade {
    pub limits: WorkerLimits,
    pub transport: ReadTransport,
}

#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExtractOptions {
    pub copy_motw: bool,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ZipCreateOptions {
    pub method: String,
    pub encryption: String,
    pub strip_single_root: bool,
    pub exclude_patterns: Vec<String>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SevenzCreateOptions {
    pub solid: bool,
    pub encryption: String,
    pub encrypt_header: bool,
}
impl Drop for WorkerRequest {
    fn drop(&mut self) {
        if let Some(password) = &mut self.password {
            password.zeroize();
        }
    }
}
fn auto_codepage() -> u32 {
    0
}

#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct WorkerLimits {
    pub max_entries: u64,
    pub max_metadata_bytes: u64,
    pub max_entry_bytes: u64,
    pub max_output_bytes: u64,
    pub max_working_bytes: u64,
}
impl Default for WorkerLimits {
    fn default() -> Self {
        let limits = ResourceLimits::default();
        Self {
            max_entries: limits.max_entries,
            max_metadata_bytes: limits.max_metadata_bytes,
            max_entry_bytes: limits.max_entry_bytes,
            max_output_bytes: limits.max_output_bytes,
            max_working_bytes: limits.max_working_bytes,
        }
    }
}
impl From<WorkerLimits> for ResourceLimits {
    fn from(value: WorkerLimits) -> Self {
        Self {
            max_entries: value.max_entries,
            max_metadata_bytes: value.max_metadata_bytes,
            max_entry_bytes: value.max_entry_bytes,
            max_output_bytes: value.max_output_bytes,
            max_working_bytes: value.max_working_bytes,
        }
    }
}
impl WorkerRequest {
    pub fn open(operation_id: Uuid, path: String) -> Self {
        Self {
            version: 1,
            operation_id,
            op: "start".into(),
            operation: "open".into(),
            path,
            codepage: auto_codepage(),
            password: None,
            limits: WorkerLimits::default(),
            create_name: None,
            compression_level: None,
            zip_options: None,
            tree_options: None,
            sevenz_options: None,
            extract_options: None,
            delete_source: None,
            read_contract: None,
            read_transport: None,
            password_interaction: None,
            conflict_policy: None,
            output_readmission: None,
            browser_entry: None,
            job_extract: None,
            job_execution: None,
            job_modify: None,
            volume_bytes: None,
            volume_paths: Vec::new(),
            #[cfg(debug_assertions)]
            zip_risk_fixture: None,
        }
    }
    pub fn validate(&self) -> io::Result<()> {
        if self.operation == "modify_zip" {
            if self.version != 2
                || self.job_execution != Some(true)
                || self.read_contract.is_some()
                || self.job_modify.as_ref().is_none_or(|value| {
                    value
                        .validate(
                            if value.action
                                == nxtools_protocol::archive_job::ArchiveJobModifyAction::Add
                            {
                                2
                            } else {
                                1
                            },
                        )
                        .is_err()
                })
                || self.browser_entry.is_some()
                || self.job_extract.is_some()
                || self.output_readmission.is_some()
                || self.delete_source.is_some()
                || self.volume_bytes.is_some()
            {
                return Err(io::ErrorKind::InvalidData.into());
            }
        } else if self.job_modify.is_some() {
            return Err(io::ErrorKind::InvalidData.into());
        }
        if self.volume_bytes.is_some_and(|size| {
            size < 65536
                || size > self.limits.max_output_bytes
                || self.version != 2
                || !matches!(self.operation.as_str(), "create_zip" | "create_7z")
        }) {
            return Err(io::ErrorKind::InvalidData.into());
        }
        if self
            .job_extract
            .as_ref()
            .is_some_and(|options| options.validate().is_err())
            || self.job_extract.is_some()
                && (self.version != 2
                    || self.operation != "extract"
                    || self.browser_entry.is_some()
                    || self.read_contract.is_some())
            || self.job_execution.is_some()
                && (self.version != 2
                    || self.read_contract.is_some()
                    || self.browser_entry.is_some())
        {
            return Err(io::ErrorKind::InvalidData.into());
        }
        if let Some(browser) = &self.browser_entry
            && (self.version != 2
                || self.operation != "extract"
                || self.read_contract.is_some()
                || self.password.is_some()
                || self.conflict_policy.is_some()
                || self.output_readmission.is_some()
                || self.delete_source.is_some()
                || self.limits.max_entry_bytes > 512 << 20
                || self.limits.max_output_bytes > 512 << 20
                || self.limits.max_entries > 100_000
                || self.limits.max_metadata_bytes > 128 << 20
                || browser.expected.validate().is_err())
        {
            return Err(io::ErrorKind::InvalidData.into());
        }
        if self.output_readmission.as_ref().is_some_and(|value| {
            value.logical_id.is_nil()
                || self.version != 2
                || self.operation != "extract"
                || self.read_contract.is_some()
                || self.delete_source.is_some()
                || value
                    .expected_source
                    .as_ref()
                    .is_some_and(|source| source.native.is_empty() || source.native.len() > 256)
        }) {
            return Err(io::ErrorKind::InvalidData.into());
        }
        #[cfg(debug_assertions)]
        if self.zip_risk_fixture.is_some_and(|value| !value.valid())
            || self.zip_risk_fixture.is_some()
                && (self.version != 2
                    || self.operation != "extract"
                    || self.read_contract.is_some())
        {
            return Err(io::ErrorKind::InvalidData.into());
        }
        if self.conflict_policy.is_some()
            && (self.version != 2 || self.operation != "extract" || self.read_contract.is_some())
        {
            return Err(io::ErrorKind::InvalidData.into());
        }
        if self.password_interaction.is_some()
            && (self.password_interaction != Some(true)
                || self.version != 2
                || self.read_contract.is_some() && self.operation != "open"
                || !matches!(
                    self.operation.as_str(),
                    "open" | "test" | "extract" | "modify_zip"
                )
                || self.browser_entry.is_some() && self.operation != "extract")
        {
            return Err(io::ErrorKind::InvalidData.into());
        }
        if self.read_contract.is_some_and(|v| !matches!(v, 1 | 2))
            || self.read_contract.is_some()
                && (self.version != 2
                    || !matches!(self.operation.as_str(), "open" | "test")
                    || self.password.is_some())
            || (self.read_contract == Some(2)) != self.read_transport.is_some()
            || self
                .read_transport
                .is_some_and(|transport| !(1..=64 << 20).contains(&transport.max_total_bytes))
            || self.read_contract == Some(2)
                && (self.limits.max_entries > 100_000 || self.limits.max_metadata_bytes > 128 << 20)
        {
            return Err(io::ErrorKind::InvalidData.into());
        }
        if self.delete_source == Some(true)
            && self.operation == "extract"
            && self.extract_options.is_none()
        {
            return Err(io::ErrorKind::InvalidData.into());
        }
        if self.delete_source.is_some()
            && (self.version != 2
                || !matches!(
                    self.operation.as_str(),
                    "extract"
                        | "create_gzip"
                        | "create_xz"
                        | "create_zip"
                        | "create_7z"
                        | "create_tar"
                        | "create_tgz"
                        | "create_txz"
                ))
        {
            return Err(io::ErrorKind::InvalidData.into());
        }
        if !matches!(self.version, 1 | 2)
            || self.operation_id.is_nil()
            || self.op != "start"
            || !matches!(
                self.operation.as_str(),
                "open"
                    | "test"
                    | "extract"
                    | "create_gzip"
                    | "create_xz"
                    | "create_zip"
                    | "create_7z"
                    | "create_tar"
                    | "create_tgz"
                    | "create_txz"
                    | "modify_zip"
            )
            || self.path.is_empty()
            || self.path.len() > 32767
            || self.path.chars().any(char::is_control)
            || self
                .password
                .as_ref()
                .is_some_and(|value| value.len() > 4096 || value.contains('\0'))
            || self.limits.max_entries == 0
            || self.limits.max_entries > 1_000_000
            || self.limits.max_metadata_bytes == 0
            || self.limits.max_metadata_bytes > 512 << 20
            || self.limits.max_working_bytes == 0
            || self.limits.max_working_bytes > 1 << 30
            || self.limits.max_entry_bytes == 0
            || self.limits.max_output_bytes == 0
        {
            return Err(io::ErrorKind::InvalidData.into());
        }
        if matches!(
            self.operation.as_str(),
            "create_gzip"
                | "create_xz"
                | "create_zip"
                | "create_7z"
                | "create_tar"
                | "create_tgz"
                | "create_txz"
        ) {
            let zip = self.operation == "create_zip";
            let sevenz = self.operation == "create_7z";
            let tar = self.operation == "create_tar";
            if self.version != 2
                || (!zip && !sevenz && self.password.is_some())
                || self.create_name.as_deref().is_none_or(|name| {
                    name.contains(['/', '\\'])
                        || crate::path_policy::validate_relative(name).is_err()
                })
                || !(if tar {
                    self.compression_level.is_none()
                } else {
                    matches!(
                        self.compression_level.as_deref(),
                        Some("fast" | "normal" | "maximum")
                    ) || zip && self.compression_level.as_deref() == Some("store")
                })
            {
                return Err(io::ErrorKind::InvalidData.into());
            }
        } else if self.create_name.is_some() || self.compression_level.is_some() {
            return Err(io::ErrorKind::InvalidData.into());
        }
        if self.operation == "create_zip" {
            let options = self
                .zip_options
                .as_ref()
                .ok_or(io::ErrorKind::InvalidData)?;
            if !matches!(options.method.as_str(), "store" | "deflate")
                || !matches!(options.encryption.as_str(), "none" | "aes256")
                || (options.method == "store")
                    != (self.compression_level.as_deref() == Some("store"))
                || (options.encryption == "aes256")
                    != self.password.as_ref().is_some_and(|p| !p.is_empty())
                || options.encryption == "none" && self.password.is_some()
                || options.exclude_patterns.len() > 32
                || options
                    .exclude_patterns
                    .iter()
                    .map(String::len)
                    .sum::<usize>()
                    > 8192
            {
                return Err(io::ErrorKind::InvalidData.into());
            }
        } else if self.zip_options.is_some() {
            return Err(io::ErrorKind::InvalidData.into());
        }
        if self.operation == "create_7z" {
            let tree = self
                .tree_options
                .as_ref()
                .ok_or(io::ErrorKind::InvalidData)?;
            let options = self
                .sevenz_options
                .as_ref()
                .ok_or(io::ErrorKind::InvalidData)?;
            if !matches!(options.encryption.as_str(), "none" | "aes256")
                || (options.encryption == "aes256")
                    != self.password.as_ref().is_some_and(|p| !p.is_empty())
                || options.encryption == "none"
                    && (self.password.is_some() || options.encrypt_header)
                || tree.exclude_patterns.len() > 32
                || tree.exclude_patterns.iter().map(String::len).sum::<usize>() > 8192
            {
                return Err(io::ErrorKind::InvalidData.into());
            }
        } else if matches!(
            self.operation.as_str(),
            "create_tar" | "create_tgz" | "create_txz"
        ) {
            let tree = self
                .tree_options
                .as_ref()
                .ok_or(io::ErrorKind::InvalidData)?;
            if self.sevenz_options.is_some()
                || tree.exclude_patterns.len() > 32
                || tree.exclude_patterns.iter().map(String::len).sum::<usize>() > 8192
            {
                return Err(io::ErrorKind::InvalidData.into());
            }
        } else if self.tree_options.is_some() || self.sevenz_options.is_some() {
            return Err(io::ErrorKind::InvalidData.into());
        }
        if self.extract_options.is_some() && (self.version != 2 || self.operation != "extract") {
            return Err(io::ErrorKind::InvalidData.into());
        }
        self.encoding()?;
        Ok(())
    }
    pub(crate) fn encoding(&self) -> io::Result<NameEncoding> {
        Ok(match self.codepage {
            0 => NameEncoding::Auto,
            65001 => NameEncoding::Utf8,
            936 => NameEncoding::Gbk,
            950 => NameEncoding::Big5,
            932 => NameEncoding::ShiftJis,
            949 => NameEncoding::Korean,
            437 => NameEncoding::Cp437,
            1250 => NameEncoding::Windows1250,
            1251 => NameEncoding::Windows1251,
            1252 => NameEncoding::Windows1252,
            _ => return Err(io::ErrorKind::InvalidData.into()),
        })
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ControlFrame {
    version: u32,
    operation_id: Uuid,
    op: String,
    #[serde(default)]
    call_id: Option<Uuid>,
    #[serde(default)]
    success: Option<bool>,
    #[serde(default)]
    token: Option<Uuid>,
    #[serde(default)]
    error_code: Option<String>,
    #[serde(default, rename = "native_error_code")]
    _native_error_code: Option<i32>,
    #[serde(default)]
    read_data: Option<String>,
    #[serde(default)]
    batch_sequence: Option<u64>,
    #[serde(default)]
    upgrade: Option<ReadUpgrade>,
    #[serde(default)]
    password_reply: Option<crate::worker_password::PasswordReply>,
    #[serde(default)]
    conflict_reply: Option<crate::worker_conflict::ConflictReply>,
    #[serde(default)]
    existing_bytes: Option<u64>,
    #[serde(default)]
    risk_reply: Option<crate::zip_risk::RiskReply>,
}

#[derive(Serialize, Deserialize)]
pub struct WorkerEntry {
    pub ordinal: u64,
    pub name: String,
    pub kind: String,
    pub size: Option<u64>,
    pub packed_size: Option<u64>,
    pub encrypted: Option<bool>,
    pub warning_codes: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provenance: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub codepage: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modified: Option<crate::metadata::ModifiedTime>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub method: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub crc32: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attributes: Option<crate::metadata::WindowsAttributes>,
}

#[derive(Serialize, Deserialize)]
pub struct WorkerFrame {
    #[serde(rename = "schema")]
    pub version: u32,
    pub operation_id: Uuid,
    pub op: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub format: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub entries: Vec<WorkerEntry>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub files_verified: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub bytes_verified: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub error_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub native: Option<crate::worker_output::NativeCall>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub capabilities: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub source_delete: Option<crate::source_deletion::SourceDeleteEligibility>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub read_contract: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snapshot_id: Option<Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub batch_sequence: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entry_count: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub batch_count: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) cancellation: Option<crate::control::CancellationObservation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub password_state: Option<crate::worker_password::PasswordState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conflict_prompt: Option<crate::worker_conflict::ConflictPrompt>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extraction_summary: Option<crate::worker_conflict::ExtractionSummary>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub risk_prompt: Option<crate::zip_risk::RiskPrompt>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_budget_shortfall: Option<crate::output_readmission::OutputShortfall>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub browser: Option<nxtools_protocol::archive_browser::ArchiveBrowserContext>,
}
impl WorkerFrame {
    pub(crate) fn state(operation_id: Uuid, op: &str) -> Self {
        Self {
            version: 1,
            operation_id,
            op: op.into(),
            output_budget_shortfall: None,
            browser: None,
            format: None,
            entries: Vec::new(),
            files_verified: None,
            bytes_verified: None,
            error_code: None,
            conflict_prompt: None,
            extraction_summary: None,
            risk_prompt: None,
            native: None,
            capabilities: Vec::new(),
            source_delete: None,
            read_contract: None,
            snapshot_id: None,
            batch_sequence: None,
            entry_count: None,
            batch_count: None,
            cancellation: None,
            password_state: None,
        }
    }
    pub fn complete(operation_id: Uuid, files: u64, bytes: u64) -> Self {
        Self {
            files_verified: Some(files),
            bytes_verified: Some(bytes),
            ..Self::state(operation_id, "complete")
        }
    }
}

pub fn read_frame<R: Read, T: DeserializeOwned>(input: &mut R) -> io::Result<Option<T>> {
    let mut header = [0u8; 4];
    if input.read(&mut header[..1])? == 0 {
        return Ok(None);
    }
    input.read_exact(&mut header[1..])?;
    let size = u32::from_le_bytes(header) as usize;
    if size == 0 || size > MAX_FRAME_BYTES {
        return Err(io::ErrorKind::InvalidData.into());
    }
    let mut frame = Zeroizing::new(vec![0u8; size]);
    input.read_exact(&mut frame)?;
    serde_json::from_slice(&frame)
        .map(Some)
        .map_err(|_| io::ErrorKind::InvalidData.into())
}
pub fn write_frame<W: Write, T: Serialize>(output: &mut W, value: &T) -> io::Result<()> {
    let bytes = Zeroizing::new(serde_json::to_vec(value).map_err(|_| io::ErrorKind::InvalidData)?);
    if bytes.is_empty() || bytes.len() > MAX_FRAME_BYTES {
        return Err(io::ErrorKind::InvalidData.into());
    }
    output.write_all(&(bytes.len() as u32).to_le_bytes())?;
    output.write_all(&bytes)?;
    output.flush()
}

/// One worker owns exactly one request and one control reader until process exit.
/// The C# parent keeps stdin open; EOF cancels. A blocked codec is terminated by its owning Job.
pub fn run<R: Read + Send + 'static, W: Write + Send>(
    mut input: R,
    mut output: W,
) -> io::Result<()> {
    let mut request: WorkerRequest = read_frame(&mut input)?.ok_or(io::ErrorKind::UnexpectedEof)?;
    request.validate()?;
    let operation_id = request.operation_id;
    let control = OperationControl::with_operation_id(operation_id);
    let reader_control = control.clone();
    let replies = crate::worker_output::ReplyPort::new();
    let reader_replies = replies.clone();
    let metadata = crate::worker_metadata::MetadataCredit::new();
    let reader_metadata = metadata.clone();
    let upgrade_allowed = request.read_contract == Some(1);
    let interaction = crate::interaction::InteractionPort::new(operation_id, 1);
    let reader_interaction = interaction.clone();
    let password_interaction = request.password_interaction == Some(true);
    let conflict_enabled = request.conflict_policy.is_some();
    let risk_enabled = request.version == 2 && request.operation == "extract";
    std::thread::Builder::new()
        .name("archive-worker-control".into())
        .spawn(move || {
            loop {
                match read_frame::<_, ControlFrame>(&mut input) {
                    Ok(Some(frame)) if frame.version == 1 && frame.operation_id == operation_id => {
                        match frame.op.as_str() {
                            "cancel" => {
                                reader_control.receive_cancel();
                                break;
                            }
                            "pause" => reader_control.set_paused(true),
                            "resume" => reader_control.set_paused(false),
                            "risk_reply" if risk_enabled => {
                                if let Some(reply) = frame.risk_reply {
                                    if reader_interaction
                                        .respond(reply.ticket, reply.response())
                                        .is_err()
                                    {
                                        reader_control.cancel();
                                        break;
                                    }
                                } else {
                                    reader_control.cancel();
                                    break;
                                }
                            }
                            "conflict_reply" if conflict_enabled => {
                                if let Some(reply) = frame.conflict_reply {
                                    if reader_interaction
                                        .respond(reply.ticket, reply.response())
                                        .is_err()
                                    {
                                        reader_control.cancel();
                                        break;
                                    }
                                } else {
                                    reader_control.cancel();
                                    break;
                                }
                            }
                            "password_reply" if password_interaction => {
                                if let Some(reply) = frame.password_reply {
                                    // Old tickets are intentionally ignored by the existing typed port.
                                    let ticket = reply.ticket;
                                    if reply
                                        .response()
                                        .and_then(|response| {
                                            reader_interaction.respond(ticket, response)
                                        })
                                        .is_err()
                                    {
                                        reader_control.cancel();
                                        break;
                                    }
                                } else {
                                    reader_control.cancel();
                                    break;
                                }
                            }
                            "read_upgrade" => {
                                let valid = frame.upgrade.is_some_and(|upgrade| {
                                    let limits =
                                        nxtools_protocol::archive_read::ArchiveReadLimits {
                                            max_entries: upgrade.limits.max_entries,
                                            max_metadata_bytes: upgrade.limits.max_metadata_bytes,
                                            max_working_bytes: upgrade.limits.max_working_bytes,
                                            max_entry_bytes: upgrade.limits.max_entry_bytes,
                                            max_output_bytes: upgrade.limits.max_output_bytes,
                                            max_transport_bytes: upgrade.transport.max_total_bytes,
                                            browser_context: upgrade.transport.browser_context,
                                            ..Default::default()
                                        };
                                    upgrade_allowed
                                        && limits.validate_for(2).is_ok()
                                        && reader_metadata.upgrade(upgrade)
                                });
                                if !valid {
                                    reader_control.cancel();
                                    break;
                                }
                            }
                            "metadata_ack" => {
                                if !frame
                                    .batch_sequence
                                    .is_some_and(|sequence| reader_metadata.acknowledge(sequence))
                                {
                                    reader_control.cancel();
                                    break;
                                }
                            }
                            "native_result" => {
                                if let (Some(call_id), Some(success)) =
                                    (frame.call_id, frame.success)
                                {
                                    reader_replies.deliver(crate::worker_output::NativeReply {
                                        call_id,
                                        success,
                                        token: frame.token,
                                        error_code: frame.error_code,
                                        read_data: frame.read_data,
                                        existing_bytes: frame.existing_bytes,
                                    });
                                } else {
                                    reader_control.cancel();
                                    break;
                                }
                            }
                            _ => {
                                reader_control.cancel();
                                break;
                            }
                        }
                    }
                    _ => {
                        reader_control.cancel();
                        break;
                    }
                }
            }
        })?;
    let mut accepted = WorkerFrame::state(operation_id, "accepted");
    if request.version == 2 {
        accepted.capabilities = vec!["source_lease_v1".into(), "budget_reservations_v1".into()];
        if request.job_execution == Some(true) {
            accepted.capabilities.push("product_job_v1".into());
        }
        if request.read_contract.is_some() || request.browser_entry.is_some() {
            accepted.capabilities.push("browser_entry_v1".into());
        }
        accepted
            .capabilities
            .push("source_delete_eligibility_v1".into());
        if request.operation == "extract" {
            accepted
                .capabilities
                .extend(["extraction_metadata_v1".into(), "motw_v1".into()]);
        }
        if matches!(
            request.operation.as_str(),
            "create_gzip"
                | "create_xz"
                | "create_zip"
                | "create_7z"
                | "create_tar"
                | "create_tgz"
                | "create_txz"
        ) {
            accepted.capabilities.push("partial_read_v1".into());
        }
        if request.operation == "modify_zip" {
            accepted.capabilities.extend([
                "zip_modify_v1".into(),
                "partial_read_v1".into(),
                "source_tree_v1".into(),
            ]);
        }
        if request.volume_bytes.is_some() {
            accepted.capabilities.push("volume_create_v1".into());
        }
        if matches!(request.operation.as_str(), "open" | "test" | "extract") {
            accepted.capabilities.push("volume_sources_v1".into());
        }
        if request.operation == "create_zip" {
            accepted
                .capabilities
                .extend(["source_tree_v1".into(), "zip_create_v1".into()]);
        }
        if matches!(
            request.operation.as_str(),
            "create_tar" | "create_tgz" | "create_txz"
        ) {
            accepted.capabilities.extend([
                "source_tree_v1".into(),
                match request.operation.as_str() {
                    "create_tar" => "tar_create_v1",
                    "create_tgz" => "tgz_create_v1",
                    _ => "txz_create_v1",
                }
                .into(),
            ]);
        }
        #[cfg(feature = "isolated-codecs")]
        if request.operation == "create_7z" {
            accepted
                .capabilities
                .extend(["source_tree_v1".into(), "sevenz_create_v1".into()]);
        }
    }
    if conflict_enabled {
        accepted.capabilities.push("zip_conflict_v1".into());
        accepted.capabilities.push("extraction_conflict_v1".into());
    }
    if request.version == 2 && request.operation == "extract" {
        accepted
            .capabilities
            .push("zip_output_readmission_v1".into());
        accepted.capabilities.push("zip_risk_v1".into());
    }
    if password_interaction {
        accepted.capabilities.push("password_interaction_v1".into());
        if request.operation == "extract" {
            accepted.capabilities.push("password_extract_v1".into());
        }
    }
    if let Some(version) = request.read_contract {
        accepted.read_contract = Some(version);
        accepted.capabilities.extend(
            nxtools_protocol::archive_read::archive_read_capabilities(version)
                .iter()
                .map(|v| (*v).to_owned()),
        );
        if version == 1 {
            accepted
                .capabilities
                .extend(["product_read_v2".into(), "bounded_metadata_v2".into()]);
        }
    }
    write_frame(&mut output, &accepted)?;
    let mut execution_output = crate::worker_job::ProgressWriter::new(
        &mut output,
        &control,
        request.job_execution == Some(true),
    );
    let result = execute(
        &mut request,
        &control,
        &mut execution_output,
        replies,
        &metadata,
        &interaction,
    );
    interaction.close();
    let mut terminal = match result {
        Ok((files, bytes)) => WorkerFrame::complete(operation_id, files, bytes),
        Err(code) => {
            let mut frame = WorkerFrame::state(operation_id, "failure");
            frame.error_code = Some(format!("{code:?}")); // Fixed enum only, never backend text.
            frame
        }
    };
    if terminal.error_code.as_deref() == Some("Cancelled") {
        terminal.cancellation = control.cancellation_observation();
    }
    if request.read_contract.is_some() {
        let (entries, batches) = metadata.counts();
        terminal.entry_count = Some(entries);
        terminal.batch_count = Some(batches);
    }
    if let Some(consent) = request.delete_source {
        terminal.source_delete = Some(control.source_delete_eligibility(consent));
    }
    write_frame(&mut output, &terminal)
}

fn execute<W: Write + Send>(
    request: &mut WorkerRequest,
    control: &OperationControl,
    output: &mut W,
    replies: crate::worker_output::ReplyPort,
    metadata: &crate::worker_metadata::MetadataCredit,
    interaction: &crate::interaction::InteractionPort,
) -> crate::Result<(u64, u64)> {
    // Dispatcher/source-acquisition failure invalidates prior facts. Each actual child lifecycle
    // (engine open then extract/test, isolated codec, or creation) owns its own completion marker.
    control.source_delete_begin();
    if request.version == 2 {
        let acquired = crate::worker_output::PrivateOutput::new(output, control, replies.clone())
            .call("source_acquire", None, None, &[])?;
        if let Some(json) = acquired.read_data {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct Acquired {
                path: String,
                volume_paths: Vec<String>,
            }
            let source: Acquired =
                serde_json::from_str(&json).map_err(|_| ArchiveError::IoFailure)?;
            if source.path.len() > 32767
                || source.volume_paths.len() > 1024
                || source
                    .volume_paths
                    .iter()
                    .any(|path| path.len() > 32767 || path.chars().any(char::is_control))
            {
                return Err(ArchiveError::ResourceLimit);
            }
            request.path = source.path;
            request.volume_paths = source.volume_paths;
        }
        if let Some(upgrade) = metadata.take_upgrade() {
            request.read_contract = Some(2);
            request.limits = upgrade.limits;
            request.read_transport = Some(upgrade.transport);
            request
                .validate()
                .map_err(|_| ArchiveError::UnsupportedOperation)?;
        }
        // General jobs permit unencrypted TAR/stream operations without pretending they support passwords.
    }
    if request.password_interaction == Some(true)
        && (!(request.job_execution == Some(true) || request.output_readmission.is_some())
            || job_password_format(request, control)?)
    {
        return crate::worker_password::execute(
            request,
            control,
            output,
            replies,
            metadata,
            interaction,
        );
    }
    execute_attempt(request, control, output, replies, metadata, interaction)
}
fn job_password_format(request: &WorkerRequest, control: &OperationControl) -> crate::Result<bool> {
    let mut bytes = [0u8; 8];
    let count = std::fs::File::open(&request.path)?.read(&mut bytes)?;
    if count >= 4 && &bytes[..2] == b"PK" {
        let mut open = OpenRequest::new(&request.path);
        open.encoding = request
            .encoding()
            .map_err(|_| ArchiveError::UnsupportedOperation)?;
        let engine = ArchiveEngine::with_limits(request.limits.into());
        let engine = if request.volume_paths.len() > 1 {
            engine.with_zip_volumes()
        } else {
            engine
        };
        return engine
            .open(open, control)
            .map(|snapshot| snapshot.entries().iter().any(|entry| entry.encrypted()));
    }
    Ok(count >= 6 && &bytes[..6] == b"7z\xbc\xaf\x27\x1c" || count >= 7 && &bytes[..4] == b"Rar!")
}

pub(crate) fn execute_attempt<W: Write + Send>(
    request: &mut WorkerRequest,
    control: &OperationControl,
    output: &mut W,
    replies: crate::worker_output::ReplyPort,
    metadata: &crate::worker_metadata::MetadataCredit,
    interaction: &crate::interaction::InteractionPort,
) -> crate::Result<(u64, u64)> {
    if request.operation == "modify_zip" {
        return crate::worker_zip_edit::execute(request, control, output, replies);
    }
    if matches!(
        request.operation.as_str(),
        "create_tar" | "create_tgz" | "create_txz"
    ) {
        return crate::worker_tar_create::create(request, control, output, replies);
    }
    if request.operation == "create_7z" {
        #[cfg(feature = "isolated-codecs")]
        return crate::worker_sevenz_create::create(request, control, output, replies);
        #[cfg(not(feature = "isolated-codecs"))]
        return Err(ArchiveError::UnsupportedOperation);
    }
    if request.operation == "create_zip" {
        return crate::worker_zip_create::create(request, control, output, replies);
    }
    if matches!(request.operation.as_str(), "create_gzip" | "create_xz") {
        return crate::worker_create::single_stream(request, control, output, replies);
    }
    #[cfg(feature = "isolated-codecs")]
    if let Some(result) = crate::worker_codecs::try_execute(
        request,
        control,
        output,
        replies.clone(),
        metadata,
        interaction,
    ) {
        return result;
    }
    let limits: crate::ResourceLimits = request.limits.into();
    // Compound metadata discovery needs a full verified outer spool, not target output credit.
    // Bound that private disk spool by the existing finite metadata reservation; never widen
    // the request/native admission or treat an unknown outer-stream overflow as a shortfall.
    let compound = request.output_readmission.is_some()
        && crate::worker_job::compound(&request.path).is_some();
    let discovery_limits = if compound {
        crate::worker_job::spool_limits(limits)
    } else {
        limits
    };
    let engine = if request.volume_paths.len() > 1 {
        ArchiveEngine::with_limits(discovery_limits).with_zip_volumes()
    } else {
        ArchiveEngine::with_limits(discovery_limits)
    };
    let engine = if let Some(password) = request.password.as_deref() {
        engine.with_password(password)
    } else {
        engine
    };
    let mut open = OpenRequest::new(&request.path);
    open.encoding = request
        .encoding()
        .map_err(|_| ArchiveError::UnsupportedOperation)?;
    let snapshot = engine.open(open, control)?;
    let compound = compound
        && matches!(
            snapshot.format(),
            crate::ArchiveFormat::TarGz
                | crate::ArchiveFormat::TarBz2
                | crate::ArchiveFormat::TarXz
                | crate::ArchiveFormat::TarZstd
        );
    crate::worker_browser::bind(
        request,
        snapshot.session_id(),
        &format!("{:?}", snapshot.format()),
        snapshot.comment(),
        snapshot.revision.digest,
        snapshot.revision.size,
        snapshot.revision.modified,
        output,
        control,
        replies.clone(),
    )?;
    // General selection maps immutable original ordinals only after source binding above.
    if request.conflict_policy.is_some()
        && !matches!(
            snapshot.format(),
            crate::ArchiveFormat::Zip
                | crate::ArchiveFormat::Tar
                | crate::ArchiveFormat::TarGz
                | crate::ArchiveFormat::TarBz2
                | crate::ArchiveFormat::TarXz
                | crate::ArchiveFormat::TarZstd
                | crate::ArchiveFormat::Gzip
                | crate::ArchiveFormat::Xz
                | crate::ArchiveFormat::Bzip2
                | crate::ArchiveFormat::Zstd
        )
    {
        return Err(ArchiveError::UnsupportedOperation);
    }
    if request
        .output_readmission
        .as_ref()
        .and_then(|r| r.expected_source.as_ref())
        .is_some_and(|s| s.digest != snapshot.revision.digest)
    {
        return Err(ArchiveError::SourceChanged);
    }
    if request.operation == "extract" {
        let mut plan = crate::worker_job::plan(request, &snapshot)?;
        if request.job_execution == Some(true) && plan.files().len() > 4096 {
            return Err(ArchiveError::ResourceLimit);
        }
        if request.output_readmission.is_some() {
            if snapshot.format() == crate::ArchiveFormat::Zip {
                let mut source = crate::engine::open_snapshot(&snapshot, &limits, control)?;
                crate::formats::zip::preflight_created(
                    &mut source,
                    snapshot.entries(),
                    &limits,
                    control,
                )?;
            }
            let mut source = crate::engine::open_snapshot(&snapshot, &limits, control)?;
            crate::engine::ensure_revision(&mut source, &snapshot.revision, control)?;
            crate::worker_output::PrivateOutput::new(output, control, replies.clone())
                .preflight_output(
                    request,
                    snapshot.revision.digest,
                    crate::worker_job::declared_output(&plan),
                )?;
        }
        let mut native = crate::worker_output::PrivateOutput::new(output, control, replies)
            .extraction_metadata(request.extract_options.is_some())
            .conflicts(
                request.conflict_policy,
                interaction,
                snapshot.entries.clone(),
            );
        if request.job_execution == Some(true) {
            if !plan.files().is_empty() || !plan.directories().is_empty() {
                native.destination_preflight(request, plan.total_bytes())?;
            }
            native.reserve_subfolder(&mut plan)?;
        }
        let risk = crate::zip_risk::ZipRisk::new(interaction);
        #[cfg(debug_assertions)]
        let risk = risk.fixture(request.zip_risk_fixture);
        if request.browser_entry.is_none() {
            native.risk = Some(risk);
        }
        let special_skips = plan.skipped_special();
        let outcome = if compound {
            crate::worker_job::extract_compound(request, &snapshot, &plan, control, &mut native)
        } else {
            let extraction = if request.volume_paths.len() > 1 {
                ArchiveEngine::with_limits(limits).with_zip_volumes()
            } else {
                ArchiveEngine::with_limits(limits)
            };
            let extraction = if let Some(password) = request.password.as_deref() {
                extraction.with_password(password)
            } else {
                extraction
            };
            extraction.extract(
                crate::output::ExtractRequest::new(snapshot, plan),
                control,
                &mut native,
            )
        };
        native.extraction_summary(
            outcome
                .as_ref()
                .is_ok_and(|value| value.skipped_special == 0),
            special_skips,
        )?;
        drop(native);
        let outcome = outcome?;
        return Ok((outcome.files_committed, outcome.bytes_committed));
    }
    if request.operation == "test" {
        let result = engine.execute(ArchiveRequest::test(snapshot), control)?;
        return Ok((result.files_verified(), result.bytes_verified()));
    }
    crate::worker_metadata::emit_entries(
        request,
        snapshot.entries(),
        snapshot.session_id(),
        &format!("{:?}", snapshot.format()),
        output,
        control,
        metadata,
    )?;
    Ok((snapshot.entries().len() as u64, 0))
}
