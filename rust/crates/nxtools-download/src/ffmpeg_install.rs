use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::time::{Duration, Instant};

use reqwest::redirect::Policy;
use sha2::{Digest, Sha256};
use thiserror::Error;
use tokio::io::AsyncWriteExt;
use tracing::{info, warn};
use zip::ZipArchive;

#[cfg(test)]
use crate::NetworkPolicy;
use crate::http::TransferDirective;
use crate::media::{MediaProcessError, MediaProcessOutcome, detect_ffmpeg_version};
use crate::{
    DownloadTransferControl, FFMPEG_BTBN_DOWNLOAD_URL, FFMPEG_GYAN_DOWNLOAD_URL,
    NetworkOperationSnapshot, NetworkPolicyError,
};
#[cfg(test)]
use nxtools_core::NetworkSettings;

const GYAN_CHECKSUM_URL: &str =
    "https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip.sha256";
const BTBN_CHECKSUM_URL: &str =
    "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/checksums.sha256";
const GYAN_ARCHIVE_NAME: &str = "ffmpeg-release-essentials.zip";
const BTBN_ARCHIVE_NAME: &str = "ffmpeg-master-latest-win64-gpl.zip";
const MAX_CHECKSUM_BYTES: usize = 64 * 1024;
const MAX_ARCHIVE_BYTES: u64 = 512 * 1024 * 1024;
const MAX_ARCHIVE_ENTRIES: usize = 8_192;
const MAX_EXPANDED_BYTES: u64 = 2 * 1024 * 1024 * 1024;
const MAX_ENTRY_BYTES: u64 = 512 * 1024 * 1024;
const MAX_ENTRY_PATH_BYTES: usize = 512;
const MAX_ENTRY_DEPTH: usize = 16;
const MAX_CLEANUP_ENTRIES: usize = 128;
const NETWORK_TIMEOUT: Duration = Duration::from_secs(15 * 60);
const STALL_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FfmpegInstallSource {
    Gyan,
    Btbn,
}

impl FfmpegInstallSource {
    pub const ALL: [Self; 2] = [Self::Gyan, Self::Btbn];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Gyan => "gyan",
            Self::Btbn => "btbn",
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct FfmpegInstallSourceSpec<'a> {
    source: FfmpegInstallSource,
    archive_url: &'a str,
    checksum_url: &'a str,
    archive_name: &'a str,
}

const FFMPEG_INSTALL_SOURCES: [FfmpegInstallSourceSpec<'static>; 2] = [
    FfmpegInstallSourceSpec {
        source: FfmpegInstallSource::Gyan,
        archive_url: FFMPEG_GYAN_DOWNLOAD_URL,
        checksum_url: GYAN_CHECKSUM_URL,
        archive_name: GYAN_ARCHIVE_NAME,
    },
    FfmpegInstallSourceSpec {
        source: FfmpegInstallSource::Btbn,
        archive_url: FFMPEG_BTBN_DOWNLOAD_URL,
        checksum_url: BTBN_CHECKSUM_URL,
        archive_name: BTBN_ARCHIVE_NAME,
    },
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FfmpegInstallPhase {
    Connecting,
    Downloading,
    Verifying,
    Extracting,
    Validating,
    Publishing,
}

impl FfmpegInstallPhase {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Connecting => "connecting",
            Self::Downloading => "downloading",
            Self::Verifying => "verifying",
            Self::Extracting => "extracting",
            Self::Validating => "validating",
            Self::Publishing => "publishing",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FfmpegInstallProgress {
    pub phase: FfmpegInstallPhase,
    pub percent: u8,
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FfmpegInstallation {
    pub executable: PathBuf,
    pub source: FfmpegInstallSource,
    pub downloaded_bytes: u64,
}

#[derive(Debug, Clone)]
pub struct FfmpegInstallControl {
    control: DownloadTransferControl,
}

impl FfmpegInstallControl {
    pub(crate) fn new(control: DownloadTransferControl) -> Self {
        Self { control }
    }

    pub fn cancel(&self) {
        self.control.cancel();
    }
}

#[derive(Debug)]
pub struct FfmpegInstallOperation {
    pub correlation_id: uuid::Uuid,
    pub network_proxy_mode: nxtools_core::NetworkProxyMode,
    pub network_proxy_source: &'static str,
    pub control: FfmpegInstallControl,
    pub progress: async_channel::Receiver<FfmpegInstallProgress>,
    pub completion: async_channel::Receiver<Result<FfmpegInstallation, FfmpegInstallFailure>>,
}

#[derive(Debug)]
pub struct FfmpegInstallFailure {
    error: FfmpegInstallError,
    downloaded_bytes: u64,
}

impl FfmpegInstallFailure {
    pub fn failure_kind(&self) -> &'static str {
        self.error.failure_kind()
    }

    pub fn native_error_code(&self) -> Option<i32> {
        self.error.native_error_code()
    }

    pub fn http_status(&self) -> Option<u16> {
        self.error.http_status()
    }

    pub fn is_cancelled(&self) -> bool {
        matches!(self.error, FfmpegInstallError::Cancelled { .. })
    }

    pub fn downloaded_bytes(&self) -> u64 {
        self.downloaded_bytes
    }

    pub fn phase(&self) -> FfmpegInstallPhase {
        self.error.phase()
    }

    pub fn user_message(&self) -> &'static str {
        let mut error = &self.error;
        while let FfmpegInstallError::Context { source, .. } = error {
            error = source;
        }
        match error {
            FfmpegInstallError::Cancelled { .. } => "FFmpeg 下载已取消",
            FfmpegInstallError::UnsupportedPlatform => "当前平台不支持自动安装 FFmpeg",
            FfmpegInstallError::ChecksumMissing
            | FfmpegInstallError::ChecksumInvalid
            | FfmpegInstallError::ChecksumMismatch => {
                "FFmpeg 下载校验失败，请重试或手动选择 ffmpeg.exe"
            }
            FfmpegInstallError::ArchiveInvalid(_)
            | FfmpegInstallError::ExecutableMissing
            | FfmpegInstallError::Version(_) => {
                "下载的 FFmpeg 包无效，请重试或使用公开来源手动安装"
            }
            _ => "自动下载 FFmpeg 失败，请检查网络和磁盘后重试",
        }
    }
}

impl From<FfmpegInstallError> for FfmpegInstallFailure {
    fn from(error: FfmpegInstallError) -> Self {
        Self {
            error,
            downloaded_bytes: 0,
        }
    }
}

#[derive(Debug, Error)]
enum FfmpegInstallError {
    #[error("FFmpeg installation failed during {phase:?}")]
    Context {
        phase: FfmpegInstallPhase,
        #[source]
        source: Box<FfmpegInstallError>,
    },
    #[error("managed FFmpeg installation is unsupported on this platform")]
    UnsupportedPlatform,
    #[error("managed FFmpeg installation was cancelled during {phase:?}")]
    Cancelled { phase: FfmpegInstallPhase },
    #[error("FFmpeg source request failed")]
    Network(#[source] reqwest::Error),
    #[error("FFmpeg network policy could not be applied")]
    NetworkPolicy(#[source] NetworkPolicyError),
    #[error("FFmpeg source returned HTTP {0}")]
    HttpStatus(u16),
    #[error("FFmpeg checksum response exceeded its bound")]
    ChecksumTooLarge,
    #[error("FFmpeg checksum did not name the requested archive")]
    ChecksumMissing,
    #[error("FFmpeg checksum was malformed")]
    ChecksumInvalid,
    #[error("FFmpeg archive checksum did not match")]
    ChecksumMismatch,
    #[error("FFmpeg archive exceeded its download bound")]
    ArchiveTooLarge,
    #[error("FFmpeg archive is invalid")]
    ArchiveInvalid(#[source] zip::result::ZipError),
    #[error("FFmpeg archive did not contain ffmpeg.exe")]
    ExecutableMissing,
    #[error("FFmpeg version validation failed")]
    Version(#[source] MediaProcessError),
    #[error("FFmpeg filesystem operation failed")]
    Io(#[source] std::io::Error),
}

impl FfmpegInstallError {
    fn failure_kind(&self) -> &'static str {
        match self {
            Self::Context { source, .. } => source.failure_kind(),
            Self::UnsupportedPlatform => "unsupported_platform",
            Self::Cancelled { .. } => "cancelled",
            Self::Network(_) => "network",
            Self::NetworkPolicy(_) => "network_policy",
            Self::HttpStatus(_) => "http_status",
            Self::ChecksumTooLarge => "checksum_too_large",
            Self::ChecksumMissing => "checksum_missing",
            Self::ChecksumInvalid => "checksum_invalid",
            Self::ChecksumMismatch => "checksum_mismatch",
            Self::ArchiveTooLarge => "archive_too_large",
            Self::ArchiveInvalid(_) => "archive_invalid",
            Self::ExecutableMissing => "executable_missing",
            Self::Version(_) => "version_invalid",
            Self::Io(_) => "io",
        }
    }

    fn phase(&self) -> FfmpegInstallPhase {
        match self {
            Self::Context { phase, .. } => *phase,
            Self::UnsupportedPlatform
            | Self::Network(_)
            | Self::NetworkPolicy(_)
            | Self::HttpStatus(_) => FfmpegInstallPhase::Connecting,
            Self::ChecksumTooLarge | Self::ChecksumMissing | Self::ChecksumInvalid => {
                FfmpegInstallPhase::Verifying
            }
            Self::ChecksumMismatch => FfmpegInstallPhase::Verifying,
            Self::ArchiveTooLarge => FfmpegInstallPhase::Downloading,
            Self::ArchiveInvalid(_) | Self::ExecutableMissing => FfmpegInstallPhase::Extracting,
            Self::Version(_) => FfmpegInstallPhase::Validating,
            Self::Io(_) => FfmpegInstallPhase::Publishing,
            Self::Cancelled { phase } => *phase,
        }
    }

    fn native_error_code(&self) -> Option<i32> {
        match self {
            Self::Context { source, .. } => source.native_error_code(),
            Self::Io(error) => error.raw_os_error(),
            Self::Version(error) => error.native_error_code(),
            _ => None,
        }
    }

    fn http_status(&self) -> Option<u16> {
        match self {
            Self::Context { source, .. } => source.http_status(),
            Self::HttpStatus(status) => Some(*status),
            _ => None,
        }
    }

    fn with_phase(self, phase: FfmpegInstallPhase) -> Self {
        match self {
            Self::Cancelled { .. } => self,
            _ => Self::Context {
                phase,
                source: Box::new(self),
            },
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct ArchiveLimits {
    max_entries: usize,
    max_expanded_bytes: u64,
    max_entry_bytes: u64,
}

impl Default for ArchiveLimits {
    fn default() -> Self {
        Self {
            max_entries: MAX_ARCHIVE_ENTRIES,
            max_expanded_bytes: MAX_EXPANDED_BYTES,
            max_entry_bytes: MAX_ENTRY_BYTES,
        }
    }
}

pub(crate) async fn run_ffmpeg_install(
    correlation_id: uuid::Uuid,
    install_root: PathBuf,
    preserve_executable: Option<PathBuf>,
    control: DownloadTransferControl,
    progress: async_channel::Sender<FfmpegInstallProgress>,
    network: NetworkOperationSnapshot,
) -> Result<FfmpegInstallation, FfmpegInstallFailure> {
    run_ffmpeg_install_inner(
        correlation_id,
        &install_root,
        preserve_executable.as_deref(),
        &control,
        &progress,
        &network,
    )
    .await
}

async fn run_ffmpeg_install_inner(
    correlation_id: uuid::Uuid,
    install_root: &Path,
    preserve_executable: Option<&Path>,
    control: &DownloadTransferControl,
    progress: &async_channel::Sender<FfmpegInstallProgress>,
    network: &NetworkOperationSnapshot,
) -> Result<FfmpegInstallation, FfmpegInstallFailure> {
    run_ffmpeg_install_with_sources_and_network(
        correlation_id,
        install_root,
        preserve_executable,
        control,
        progress,
        &FFMPEG_INSTALL_SOURCES,
        network,
    )
    .await
}

#[cfg(test)]
async fn run_ffmpeg_install_with_sources(
    correlation_id: uuid::Uuid,
    install_root: &Path,
    preserve_executable: Option<&Path>,
    control: &DownloadTransferControl,
    progress: &async_channel::Sender<FfmpegInstallProgress>,
    sources: &[FfmpegInstallSourceSpec<'_>],
) -> Result<FfmpegInstallation, FfmpegInstallFailure> {
    let policy = NetworkPolicy::new(NetworkSettings::default())
        .map_err(FfmpegInstallError::NetworkPolicy)?;
    let network = policy
        .snapshot()
        .map_err(FfmpegInstallError::NetworkPolicy)?;
    run_ffmpeg_install_with_sources_and_network(
        correlation_id,
        install_root,
        preserve_executable,
        control,
        progress,
        sources,
        &network,
    )
    .await
}

async fn run_ffmpeg_install_with_sources_and_network(
    correlation_id: uuid::Uuid,
    install_root: &Path,
    preserve_executable: Option<&Path>,
    control: &DownloadTransferControl,
    progress: &async_channel::Sender<FfmpegInstallProgress>,
    sources: &[FfmpegInstallSourceSpec<'_>],
    network: &NetworkOperationSnapshot,
) -> Result<FfmpegInstallation, FfmpegInstallFailure> {
    let proxy_mode = nxtools_network::proxy_mode_name(network.mode());
    let proxy_source = network.source();
    if !cfg!(windows) {
        return Err(FfmpegInstallError::UnsupportedPlatform.into());
    }
    fs::create_dir_all(install_root).map_err(|error| {
        FfmpegInstallError::Io(error).with_phase(FfmpegInstallPhase::Publishing)
    })?;
    cleanup_owned_installations(
        correlation_id,
        install_root,
        None,
        preserve_executable,
        "startup_reconciliation",
    );
    let client = network
        .build_client(
            reqwest::Client::builder()
                .timeout(NETWORK_TIMEOUT)
                .read_timeout(STALL_TIMEOUT)
                .redirect(Policy::custom(|attempt| {
                    if attempt.previous().len() < 5 && attempt.url().scheme() == "https" {
                        attempt.follow()
                    } else {
                        attempt.stop()
                    }
                })),
        )
        .map_err(FfmpegInstallError::NetworkPolicy)?;
    let mut last_error = None;
    let mut downloaded_bytes = 0_u64;
    for (index, spec) in sources.iter().copied().enumerate() {
        let source = spec.source;
        if let Err(error) = ensure_running(control, FfmpegInstallPhase::Connecting) {
            return Err(FfmpegInstallFailure {
                error,
                downloaded_bytes,
            });
        }
        info!(
            event = "download.ffmpeg_install.source_started",
            %correlation_id,
            proxy_mode,
            proxy_source,
            source = source.as_str(),
            attempt = index + 1,
            "managed FFmpeg source attempt started"
        );
        let mut attempt = FfmpegInstallAttemptContext {
            correlation_id,
            client: &client,
            install_root,
            preserve_executable,
            control,
            progress,
            downloaded_bytes: &mut downloaded_bytes,
        };
        match install_from_source(spec, &mut attempt).await {
            Ok(mut installation) => {
                installation.downloaded_bytes = downloaded_bytes;
                return Ok(installation);
            }
            Err(error @ FfmpegInstallError::Cancelled { .. }) => {
                return Err(FfmpegInstallFailure {
                    error,
                    downloaded_bytes,
                });
            }
            Err(error) => {
                warn!(
                    event = "download.ffmpeg_install.source_failed",
                    %correlation_id,
                    proxy_mode,
                    proxy_source,
                    source = source.as_str(),
                    attempt = index + 1,
                    phase = error.phase().as_str(),
                    failure_kind = error.failure_kind(),
                    native_error_code = error.native_error_code(),
                    http_status = error.http_status(),
                    "managed FFmpeg source attempt failed"
                );
                last_error = Some(error);
            }
        }
    }
    Err(FfmpegInstallFailure {
        error: last_error.unwrap_or(FfmpegInstallError::ChecksumMissing),
        downloaded_bytes,
    })
}

struct FfmpegInstallAttemptContext<'a> {
    correlation_id: uuid::Uuid,
    client: &'a reqwest::Client,
    install_root: &'a Path,
    preserve_executable: Option<&'a Path>,
    control: &'a DownloadTransferControl,
    progress: &'a async_channel::Sender<FfmpegInstallProgress>,
    downloaded_bytes: &'a mut u64,
}

async fn install_from_source(
    spec: FfmpegInstallSourceSpec<'_>,
    context: &mut FfmpegInstallAttemptContext<'_>,
) -> Result<FfmpegInstallation, FfmpegInstallError> {
    let attempt_id = uuid::Uuid::new_v4();
    let staging_root = context.install_root.join(format!(".staging-{attempt_id}"));
    fs::create_dir(&staging_root).map_err(|error| {
        FfmpegInstallError::Io(error).with_phase(FfmpegInstallPhase::Connecting)
    })?;
    let result = install_from_source_staged(spec, context, &staging_root, attempt_id).await;
    if staging_root.exists()
        && let Err(cleanup_error) = fs::remove_dir_all(&staging_root)
    {
        return Err(
            FfmpegInstallError::Io(cleanup_error).with_phase(FfmpegInstallPhase::Publishing)
        );
    }
    result
}

async fn install_from_source_staged(
    spec: FfmpegInstallSourceSpec<'_>,
    context: &mut FfmpegInstallAttemptContext<'_>,
    staging_root: &Path,
    attempt_id: uuid::Uuid,
) -> Result<FfmpegInstallation, FfmpegInstallError> {
    let FfmpegInstallAttemptContext {
        correlation_id,
        client,
        install_root,
        preserve_executable,
        control,
        progress,
        downloaded_bytes: authoritative_downloaded_bytes,
    } = context;
    let source = spec.source;
    report_progress(progress, FfmpegInstallPhase::Connecting, 0, 0, None);
    let checksum_text = download_bounded_text(client, spec.checksum_url, control).await?;
    let expected_checksum = parse_checksum(&checksum_text, spec.archive_name)?;
    let archive_path = staging_root.join("ffmpeg.zip");
    let (downloaded_bytes, actual_checksum) = download_archive(
        client,
        spec.archive_url,
        &archive_path,
        control,
        progress,
        authoritative_downloaded_bytes,
    )
    .await?;
    report_progress(
        progress,
        FfmpegInstallPhase::Verifying,
        72,
        downloaded_bytes,
        Some(downloaded_bytes),
    );
    if actual_checksum != expected_checksum {
        return Err(FfmpegInstallError::ChecksumMismatch);
    }
    ensure_running(control, FfmpegInstallPhase::Extracting)?;
    report_progress(
        progress,
        FfmpegInstallPhase::Extracting,
        76,
        downloaded_bytes,
        Some(downloaded_bytes),
    );
    let extract_root = staging_root.join("extract");
    fs::create_dir(&extract_root).map_err(|error| {
        FfmpegInstallError::Io(error).with_phase(FfmpegInstallPhase::Extracting)
    })?;
    let archive_for_extract = archive_path.clone();
    let extract_for_worker = extract_root.clone();
    let extract_control = control.clone();
    let candidate = tokio::task::spawn_blocking(move || {
        extract_ffmpeg_binary_with_limits(
            &archive_for_extract,
            &extract_for_worker,
            &extract_control,
            ArchiveLimits::default(),
        )
    })
    .await
    .map_err(|_| {
        FfmpegInstallError::Io(std::io::Error::other("FFmpeg extraction worker failed"))
            .with_phase(FfmpegInstallPhase::Extracting)
    })?
    .map_err(|error| error.with_phase(FfmpegInstallPhase::Extracting))?;
    ensure_running(control, FfmpegInstallPhase::Validating)?;
    report_progress(
        progress,
        FfmpegInstallPhase::Validating,
        92,
        downloaded_bytes,
        Some(downloaded_bytes),
    );
    match detect_ffmpeg_version(&candidate, control).await {
        Ok(MediaProcessOutcome::Completed) => {}
        Ok(MediaProcessOutcome::Paused | MediaProcessOutcome::Cancelled) => {
            return Err(FfmpegInstallError::Cancelled {
                phase: FfmpegInstallPhase::Validating,
            });
        }
        Err(error) => return Err(FfmpegInstallError::Version(error)),
    }
    ensure_running(control, FfmpegInstallPhase::Publishing)?;
    report_progress(
        progress,
        FfmpegInstallPhase::Publishing,
        97,
        downloaded_bytes,
        Some(downloaded_bytes),
    );
    let binary_directory = candidate
        .parent()
        .ok_or(FfmpegInstallError::ExecutableMissing)?;
    let executable_name = candidate
        .file_name()
        .ok_or(FfmpegInstallError::ExecutableMissing)?
        .to_owned();
    let publish_candidate = install_root.join(format!(".candidate-{attempt_id}"));
    fs::rename(binary_directory, &publish_candidate).map_err(|error| {
        FfmpegInstallError::Io(error).with_phase(FfmpegInstallPhase::Publishing)
    })?;
    if let Err(error) = fs::remove_dir_all(staging_root) {
        let _ = fs::remove_dir_all(&publish_candidate);
        return Err(FfmpegInstallError::Io(error).with_phase(FfmpegInstallPhase::Publishing));
    }
    let final_directory = install_root.join(format!("managed-{attempt_id}"));
    if let Err(error) = fs::rename(&publish_candidate, &final_directory) {
        let _ = fs::remove_dir_all(&publish_candidate);
        return Err(FfmpegInstallError::Io(error).with_phase(FfmpegInstallPhase::Publishing));
    }
    let installed_executable = final_directory.join(executable_name);
    cleanup_owned_installations(
        *correlation_id,
        install_root,
        Some(&installed_executable),
        *preserve_executable,
        "superseded_install",
    );
    report_progress(
        progress,
        FfmpegInstallPhase::Publishing,
        100,
        downloaded_bytes,
        Some(downloaded_bytes),
    );
    Ok(FfmpegInstallation {
        executable: installed_executable,
        source,
        downloaded_bytes,
    })
}

async fn download_bounded_text(
    client: &reqwest::Client,
    url: &str,
    control: &DownloadTransferControl,
) -> Result<String, FfmpegInstallError> {
    ensure_running(control, FfmpegInstallPhase::Connecting)?;
    let mut directives = control.subscribe();
    ensure_running(control, FfmpegInstallPhase::Connecting)?;
    let request = client.get(url).send();
    tokio::pin!(request);
    let mut response = tokio::select! {
        biased;
        changed = directives.changed() => {
            let _ = changed;
            return Err(FfmpegInstallError::Cancelled { phase: FfmpegInstallPhase::Connecting });
        }
        response = &mut request => response.map_err(|error| {
            FfmpegInstallError::Network(error).with_phase(FfmpegInstallPhase::Connecting)
        })?,
    };
    if !response.status().is_success() {
        return Err(FfmpegInstallError::HttpStatus(response.status().as_u16()));
    }
    if response
        .content_length()
        .is_some_and(|length| length > MAX_CHECKSUM_BYTES as u64)
    {
        return Err(FfmpegInstallError::ChecksumTooLarge);
    }
    let mut body = Vec::new();
    loop {
        let chunk = response.chunk();
        tokio::pin!(chunk);
        let next = tokio::select! {
            biased;
            changed = directives.changed() => {
                let _ = changed;
                return Err(FfmpegInstallError::Cancelled { phase: FfmpegInstallPhase::Verifying });
            }
            chunk = &mut chunk => chunk.map_err(|error| {
                FfmpegInstallError::Network(error).with_phase(FfmpegInstallPhase::Verifying)
            })?,
        };
        let Some(chunk) = next else { break };
        if body.len().saturating_add(chunk.len()) > MAX_CHECKSUM_BYTES {
            return Err(FfmpegInstallError::ChecksumTooLarge);
        }
        body.extend_from_slice(&chunk);
    }
    String::from_utf8(body).map_err(|_| FfmpegInstallError::ChecksumInvalid)
}

async fn download_archive(
    client: &reqwest::Client,
    url: &str,
    archive_path: &Path,
    control: &DownloadTransferControl,
    progress: &async_channel::Sender<FfmpegInstallProgress>,
    authoritative_downloaded_bytes: &mut u64,
) -> Result<(u64, [u8; 32]), FfmpegInstallError> {
    let mut directives = control.subscribe();
    ensure_running(control, FfmpegInstallPhase::Connecting)?;
    let request = client.get(url).send();
    tokio::pin!(request);
    let mut response = tokio::select! {
        biased;
        changed = directives.changed() => {
            let _ = changed;
            return Err(FfmpegInstallError::Cancelled { phase: FfmpegInstallPhase::Connecting });
        }
        response = &mut request => response.map_err(|error| {
            FfmpegInstallError::Network(error).with_phase(FfmpegInstallPhase::Connecting)
        })?,
    };
    if !response.status().is_success() {
        return Err(FfmpegInstallError::HttpStatus(response.status().as_u16()));
    }
    let total_bytes = response.content_length();
    if total_bytes.is_some_and(|length| length > MAX_ARCHIVE_BYTES) {
        return Err(FfmpegInstallError::ArchiveTooLarge);
    }
    let mut output = tokio::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(archive_path)
        .await
        .map_err(|error| {
            FfmpegInstallError::Io(error).with_phase(FfmpegInstallPhase::Downloading)
        })?;
    let mut digest = Sha256::new();
    let attempt_base_bytes = *authoritative_downloaded_bytes;
    let mut downloaded_bytes = 0_u64;
    let mut last_percent = 0_u8;
    let mut last_reported_bytes = 0_u64;
    report_progress(progress, FfmpegInstallPhase::Downloading, 0, 0, total_bytes);
    loop {
        let chunk = response.chunk();
        tokio::pin!(chunk);
        let next = tokio::select! {
            biased;
            changed = directives.changed() => {
                let _ = changed;
                return Err(FfmpegInstallError::Cancelled { phase: FfmpegInstallPhase::Downloading });
            }
            chunk = &mut chunk => chunk.map_err(|error| {
                FfmpegInstallError::Network(error).with_phase(FfmpegInstallPhase::Downloading)
            })?,
        };
        let Some(chunk) = next else { break };
        downloaded_bytes = downloaded_bytes
            .checked_add(chunk.len() as u64)
            .ok_or(FfmpegInstallError::ArchiveTooLarge)?;
        *authoritative_downloaded_bytes = attempt_base_bytes.saturating_add(downloaded_bytes);
        if downloaded_bytes > MAX_ARCHIVE_BYTES {
            return Err(FfmpegInstallError::ArchiveTooLarge);
        }
        output.write_all(&chunk).await.map_err(|error| {
            FfmpegInstallError::Io(error).with_phase(FfmpegInstallPhase::Downloading)
        })?;
        digest.update(&chunk);
        let percent = total_bytes.filter(|total| *total > 0).map_or(0, |total| {
            ((downloaded_bytes.saturating_mul(70) / total).min(70)) as u8
        });
        let byte_milestone = downloaded_bytes.saturating_sub(last_reported_bytes) >= 1024 * 1024;
        if percent != last_percent || last_reported_bytes == 0 || byte_milestone {
            last_percent = percent;
            last_reported_bytes = downloaded_bytes;
            report_progress(
                progress,
                FfmpegInstallPhase::Downloading,
                percent,
                downloaded_bytes,
                total_bytes,
            );
        }
    }
    output.flush().await.map_err(|error| {
        FfmpegInstallError::Io(error).with_phase(FfmpegInstallPhase::Downloading)
    })?;
    output.sync_all().await.map_err(|error| {
        FfmpegInstallError::Io(error).with_phase(FfmpegInstallPhase::Downloading)
    })?;
    let actual_length = output
        .metadata()
        .await
        .map_err(|error| FfmpegInstallError::Io(error).with_phase(FfmpegInstallPhase::Downloading))?
        .len();
    if total_bytes.is_some_and(|total| total != actual_length) {
        return Err(
            FfmpegInstallError::ArchiveInvalid(zip::result::ZipError::InvalidArchive(
                "download length mismatch".into(),
            ))
            .with_phase(FfmpegInstallPhase::Downloading),
        );
    }
    Ok((downloaded_bytes, digest.finalize().into()))
}

fn parse_checksum(text: &str, archive_name: &str) -> Result<[u8; 32], FfmpegInstallError> {
    let candidate = text.lines().find_map(|line| {
        let trimmed = line.trim();
        if trimmed.len() == 64 && trimmed.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Some(trimmed);
        }
        let mut fields = trimmed.split_whitespace();
        let checksum = fields.next()?;
        let name = fields.next()?.trim_start_matches('*');
        (name == archive_name).then_some(checksum)
    });
    let candidate = candidate.ok_or(FfmpegInstallError::ChecksumMissing)?;
    if candidate.len() != 64 || !candidate.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(FfmpegInstallError::ChecksumInvalid);
    }
    let mut bytes = [0_u8; 32];
    for (index, output) in bytes.iter_mut().enumerate() {
        *output = u8::from_str_radix(&candidate[index * 2..index * 2 + 2], 16)
            .map_err(|_| FfmpegInstallError::ChecksumInvalid)?;
    }
    Ok(bytes)
}

fn extract_ffmpeg_binary_with_limits(
    archive_path: &Path,
    extract_root: &Path,
    control: &DownloadTransferControl,
    limits: ArchiveLimits,
) -> Result<PathBuf, FfmpegInstallError> {
    let input = File::open(archive_path).map_err(FfmpegInstallError::Io)?;
    let mut archive = ZipArchive::new(input).map_err(FfmpegInstallError::ArchiveInvalid)?;
    if archive.is_empty() || archive.len() > limits.max_entries {
        return Err(FfmpegInstallError::ArchiveInvalid(
            zip::result::ZipError::InvalidArchive("entry count outside bound".into()),
        ));
    }
    let mut expanded_bytes = 0_u64;
    let mut executable = None;
    for index in 0..archive.len() {
        ensure_running(control, FfmpegInstallPhase::Extracting)?;
        let mut entry = archive
            .by_index(index)
            .map_err(FfmpegInstallError::ArchiveInvalid)?;
        let relative = entry.enclosed_name().ok_or_else(|| {
            FfmpegInstallError::ArchiveInvalid(zip::result::ZipError::InvalidArchive(
                "unsafe entry path".into(),
            ))
        })?;
        if relative.as_os_str().as_encoded_bytes().len() > MAX_ENTRY_PATH_BYTES
            || relative.components().count() > MAX_ENTRY_DEPTH
            || relative
                .components()
                .any(|component| !matches!(component, Component::Normal(_)))
        {
            return Err(FfmpegInstallError::ArchiveInvalid(
                zip::result::ZipError::InvalidArchive("entry path outside bound".into()),
            ));
        }
        if entry
            .unix_mode()
            .is_some_and(|mode| mode & 0o170000 == 0o120000)
        {
            return Err(FfmpegInstallError::ArchiveInvalid(
                zip::result::ZipError::InvalidArchive("symbolic links are not supported".into()),
            ));
        }
        if entry.is_dir() {
            fs::create_dir_all(extract_root.join(relative)).map_err(FfmpegInstallError::Io)?;
            continue;
        }
        if entry.size() > limits.max_entry_bytes {
            return Err(FfmpegInstallError::ArchiveTooLarge);
        }
        expanded_bytes = expanded_bytes
            .checked_add(entry.size())
            .ok_or(FfmpegInstallError::ArchiveTooLarge)?;
        if expanded_bytes > limits.max_expanded_bytes {
            return Err(FfmpegInstallError::ArchiveTooLarge);
        }
        let output_path = extract_root.join(relative);
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent).map_err(FfmpegInstallError::Io)?;
        }
        let mut output = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&output_path)
            .map_err(FfmpegInstallError::Io)?;
        let mut written = 0_u64;
        let mut buffer = [0_u8; 128 * 1024];
        loop {
            ensure_running(control, FfmpegInstallPhase::Extracting)?;
            let read = entry.read(&mut buffer).map_err(FfmpegInstallError::Io)?;
            if read == 0 {
                break;
            }
            written = written
                .checked_add(read as u64)
                .ok_or(FfmpegInstallError::ArchiveTooLarge)?;
            if written > entry.size() || written > limits.max_entry_bytes {
                return Err(FfmpegInstallError::ArchiveTooLarge);
            }
            output
                .write_all(&buffer[..read])
                .map_err(FfmpegInstallError::Io)?;
        }
        output.sync_all().map_err(FfmpegInstallError::Io)?;
        if written != entry.size() {
            return Err(FfmpegInstallError::ArchiveInvalid(
                zip::result::ZipError::InvalidArchive("entry length mismatch".into()),
            ));
        }
        if output_path
            .file_name()
            .is_some_and(|name| name.to_string_lossy().eq_ignore_ascii_case("ffmpeg.exe"))
            && executable.as_ref().is_none_or(|current: &PathBuf| {
                output_path.components().count() < current.components().count()
            })
        {
            executable = Some(output_path);
        }
    }
    executable.ok_or(FfmpegInstallError::ExecutableMissing)
}

fn ensure_running(
    control: &DownloadTransferControl,
    phase: FfmpegInstallPhase,
) -> Result<(), FfmpegInstallError> {
    match control.current_directive() {
        TransferDirective::Running => Ok(()),
        TransferDirective::Pause | TransferDirective::Cancel => {
            Err(FfmpegInstallError::Cancelled { phase })
        }
    }
}

fn report_progress(
    progress: &async_channel::Sender<FfmpegInstallProgress>,
    phase: FfmpegInstallPhase,
    percent: u8,
    downloaded_bytes: u64,
    total_bytes: Option<u64>,
) {
    let _ = progress.try_send(FfmpegInstallProgress {
        phase,
        percent: percent.min(100),
        downloaded_bytes,
        total_bytes,
    });
}

fn cleanup_owned_installations(
    correlation_id: uuid::Uuid,
    install_root: &Path,
    installed_executable: Option<&Path>,
    preserve_executable: Option<&Path>,
    reason: &'static str,
) {
    let started_at = Instant::now();
    let installed_directory = protected_install_directory(installed_executable);
    let preserved_directory = protected_install_directory(preserve_executable);
    let entries = match fs::read_dir(install_root) {
        Ok(entries) => entries,
        Err(error) => {
            warn!(
                event = "download.ffmpeg_install.cleanup_failed",
                %correlation_id,
                artifact_kind = "managed_install_directory",
                reason,
                failure_count = 1,
                native_error_code = error.raw_os_error(),
                elapsed_ms = started_at.elapsed().as_millis(),
                "managed FFmpeg cleanup could not enumerate its owned directory"
            );
            return;
        }
    };
    let mut failure_count = 0_u32;
    let mut first_native_error_code = None;
    let mut deferred = false;
    for (index, entry) in entries.enumerate() {
        if index >= MAX_CLEANUP_ENTRIES {
            deferred = true;
            break;
        }
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                failure_count = failure_count.saturating_add(1);
                first_native_error_code = first_native_error_code.or(error.raw_os_error());
                continue;
            }
        };
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        let is_managed = name.starts_with("managed-");
        let is_staging = name.starts_with(".staging-") || name.starts_with(".candidate-");
        if !is_managed && !is_staging {
            continue;
        }
        if is_managed {
            if matches!(installed_directory, ProtectedInstallDirectory::Unresolved)
                || matches!(preserved_directory, ProtectedInstallDirectory::Unresolved)
            {
                deferred = true;
                continue;
            }
            let canonical_path = match fs::canonicalize(&path) {
                Ok(path) => path,
                Err(error) => {
                    failure_count = failure_count.saturating_add(1);
                    first_native_error_code = first_native_error_code.or(error.raw_os_error());
                    continue;
                }
            };
            if installed_directory.matches(&canonical_path)
                || preserved_directory.matches(&canonical_path)
            {
                continue;
            }
        }
        if let Err(error) = fs::remove_dir_all(path) {
            failure_count = failure_count.saturating_add(1);
            first_native_error_code = first_native_error_code.or(error.raw_os_error());
        }
    }
    if failure_count > 0 || deferred {
        warn!(
            event = "download.ffmpeg_install.cleanup_failed",
            %correlation_id,
            artifact_kind = "managed_install_directory",
            reason,
            failure_count,
            deferred,
            native_error_code = first_native_error_code,
            elapsed_ms = started_at.elapsed().as_millis(),
            "managed FFmpeg cleanup remains owned for a later bounded attempt"
        );
    }
}

#[derive(Debug)]
enum ProtectedInstallDirectory {
    None,
    Resolved(PathBuf),
    Unresolved,
}

impl ProtectedInstallDirectory {
    fn matches(&self, candidate: &Path) -> bool {
        matches!(self, Self::Resolved(directory) if directory == candidate)
    }
}

fn protected_install_directory(executable: Option<&Path>) -> ProtectedInstallDirectory {
    let Some(executable) = executable else {
        return ProtectedInstallDirectory::None;
    };
    match fs::canonicalize(executable) {
        Ok(executable) => executable
            .parent()
            .map_or(ProtectedInstallDirectory::Unresolved, |directory| {
                ProtectedInstallDirectory::Resolved(directory.to_owned())
            }),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            ProtectedInstallDirectory::None
        }
        Err(_) => ProtectedInstallDirectory::Unresolved,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(windows)]
    use std::io::Cursor;
    #[cfg(windows)]
    use std::net::TcpListener;
    #[cfg(windows)]
    use std::sync::Arc;
    #[cfg(windows)]
    use std::sync::atomic::{AtomicBool, Ordering};
    #[cfg(windows)]
    use std::thread;
    use zip::ZipWriter;
    use zip::write::SimpleFileOptions;

    #[test]
    fn checksum_parser_accepts_single_hash_and_named_github_manifest() {
        let hash = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        assert_eq!(
            parse_checksum(hash, GYAN_ARCHIVE_NAME).expect("parse Gyan checksum fixture"),
            decode(hash)
        );
        let manifest = format!("{hash}  other.zip\n{hash} *{BTBN_ARCHIVE_NAME}\n");
        assert_eq!(
            parse_checksum(&manifest, BTBN_ARCHIVE_NAME).expect("parse BtbN checksum fixture"),
            decode(hash)
        );
        assert!(matches!(
            parse_checksum(&manifest, "missing.zip"),
            Err(FfmpegInstallError::ChecksumMissing)
        ));
    }

    #[test]
    fn bounded_extraction_finds_nested_ffmpeg_and_rejects_traversal() {
        let directory = tempfile::tempdir().expect("create FFmpeg archive fixture");
        let valid_archive = directory.path().join("valid.zip");
        write_zip(
            &valid_archive,
            &[
                ("ffmpeg-build/bin/ffmpeg.exe", b"fixture"),
                ("ffmpeg-build/bin/ffprobe.exe", b"probe"),
            ],
        );
        let extract = directory.path().join("extract-valid");
        fs::create_dir(&extract).expect("create extraction root");
        let executable = extract_ffmpeg_binary_with_limits(
            &valid_archive,
            &extract,
            &DownloadTransferControl::new(),
            ArchiveLimits::default(),
        )
        .expect("extract bounded FFmpeg archive");
        assert_eq!(
            fs::read(executable).expect("read extracted FFmpeg fixture"),
            b"fixture"
        );

        let unsafe_archive = directory.path().join("unsafe.zip");
        write_zip(&unsafe_archive, &[("../ffmpeg.exe", b"unsafe")]);
        let unsafe_extract = directory.path().join("extract-unsafe");
        fs::create_dir(&unsafe_extract).expect("create unsafe extraction root");
        assert!(matches!(
            extract_ffmpeg_binary_with_limits(
                &unsafe_archive,
                &unsafe_extract,
                &DownloadTransferControl::new(),
                ArchiveLimits::default(),
            ),
            Err(FfmpegInstallError::ArchiveInvalid(_))
        ));
        assert!(!directory.path().join("ffmpeg.exe").exists());
    }

    #[test]
    fn extraction_honors_expansion_budget_and_cancellation() {
        let directory = tempfile::tempdir().expect("create FFmpeg bounded fixture");
        let archive = directory.path().join("bounded.zip");
        write_zip(&archive, &[("bin/ffmpeg.exe", b"12345")]);
        let extract = directory.path().join("extract-budget");
        fs::create_dir(&extract).expect("create bounded extraction root");
        assert!(matches!(
            extract_ffmpeg_binary_with_limits(
                &archive,
                &extract,
                &DownloadTransferControl::new(),
                ArchiveLimits {
                    max_entries: 2,
                    max_expanded_bytes: 4,
                    max_entry_bytes: 4,
                },
            ),
            Err(FfmpegInstallError::ArchiveTooLarge)
        ));

        let cancelled_extract = directory.path().join("extract-cancelled");
        fs::create_dir(&cancelled_extract).expect("create cancelled extraction root");
        let control = DownloadTransferControl::new();
        control.cancel();
        assert!(matches!(
            extract_ffmpeg_binary_with_limits(
                &archive,
                &cancelled_extract,
                &control,
                ArchiveLimits::default(),
            ),
            Err(FfmpegInstallError::Cancelled {
                phase: FfmpegInstallPhase::Extracting
            })
        ));
    }

    #[test]
    fn bounded_cleanup_removes_stale_owned_artifacts_and_preserves_current_install() {
        let directory = tempfile::tempdir().expect("create managed FFmpeg cleanup fixture");
        let stale_managed = directory.path().join("managed-stale");
        let stale_staging = directory.path().join(".staging-stale");
        let stale_candidate = directory.path().join(".candidate-stale");
        let current = directory.path().join("managed-current");
        let unrelated = directory.path().join("user-content");
        for path in [
            &stale_managed,
            &stale_staging,
            &stale_candidate,
            &current,
            &unrelated,
        ] {
            fs::create_dir(path).expect("create cleanup fixture directory");
        }
        let current_executable = current.join("ffmpeg.exe");
        fs::write(&current_executable, b"fixture").expect("write current FFmpeg fixture");
        #[cfg(windows)]
        let preserved_spelling =
            PathBuf::from(current_executable.to_string_lossy().to_ascii_uppercase());
        #[cfg(not(windows))]
        let preserved_spelling = current_executable.clone();

        cleanup_owned_installations(
            uuid::Uuid::new_v4(),
            directory.path(),
            None,
            Some(&preserved_spelling),
            "test_reconciliation",
        );

        assert!(!stale_managed.exists());
        assert!(!stale_staging.exists());
        assert!(!stale_candidate.exists());
        assert!(current.exists());
        assert!(unrelated.exists());
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn real_http_fallback_validates_and_atomically_publishes_ffmpeg() {
        let directory = tempfile::tempdir().expect("create real installer fixture directory");
        let archive = valid_ffmpeg_archive_fixture(directory.path());
        let checksum = checksum_hex(&archive);
        let (base_url, archive_started, server) = start_http_fixture(vec![
            HttpFixtureResponse::failure("/gyan.sha256", 503),
            HttpFixtureResponse::ok(
                "/btbn.sha256",
                format!("{checksum} *{BTBN_ARCHIVE_NAME}\n").into_bytes(),
                true,
            ),
            HttpFixtureResponse::ok("/btbn.zip", archive.clone(), false),
        ]);
        let gyan_checksum_url = format!("{base_url}/gyan.sha256");
        let gyan_archive_url = format!("{base_url}/gyan.zip");
        let btbn_checksum_url = format!("{base_url}/btbn.sha256");
        let btbn_archive_url = format!("{base_url}/btbn.zip");
        let sources = [
            FfmpegInstallSourceSpec {
                source: FfmpegInstallSource::Gyan,
                archive_url: &gyan_archive_url,
                checksum_url: &gyan_checksum_url,
                archive_name: GYAN_ARCHIVE_NAME,
            },
            FfmpegInstallSourceSpec {
                source: FfmpegInstallSource::Btbn,
                archive_url: &btbn_archive_url,
                checksum_url: &btbn_checksum_url,
                archive_name: BTBN_ARCHIVE_NAME,
            },
        ];
        let install_root = directory.path().join("ffmpeg");
        let control = DownloadTransferControl::new();
        let (progress_sender, progress_receiver) = async_channel::bounded(64);

        let installation = run_ffmpeg_install_with_sources(
            uuid::Uuid::new_v4(),
            &install_root,
            None,
            &control,
            &progress_sender,
            &sources,
        )
        .await
        .expect("fallback source should install FFmpeg");
        drop(progress_sender);
        server.join().expect("join real installer fixture");

        assert_eq!(installation.source, FfmpegInstallSource::Btbn);
        assert!(installation.executable.is_file());
        assert!(archive_started.load(Ordering::Acquire));
        let mut progress = Vec::new();
        while let Ok(update) = progress_receiver.try_recv() {
            progress.push(update);
        }
        assert!(progress.iter().any(|update| {
            update.phase == FfmpegInstallPhase::Downloading
                && update.total_bytes.is_none()
                && update.downloaded_bytes > 0
        }));
        assert!(
            progress
                .iter()
                .any(|update| update.phase == FfmpegInstallPhase::Validating)
        );
        assert_eq!(
            fs::read_dir(&install_root)
                .expect("read published installer root")
                .filter_map(Result::ok)
                .filter(|entry| entry.file_name().to_string_lossy().starts_with("managed-"))
                .count(),
            1
        );
        assert!(
            !fs::read_dir(&install_root)
                .expect("read clean installer root")
                .filter_map(Result::ok)
                .any(|entry| {
                    let name = entry.file_name();
                    let name = name.to_string_lossy();
                    name.starts_with(".staging-") || name.starts_with(".candidate-")
                })
        );
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn real_http_cancel_interrupts_stream_and_removes_staging() {
        let archive = cancellable_archive_fixture();
        let checksum = checksum_hex(&archive);
        let (base_url, archive_started, server) = start_http_fixture(vec![
            HttpFixtureResponse::ok("/checksum", checksum.into_bytes(), true),
            HttpFixtureResponse::slow("/archive.zip", archive),
        ]);
        let checksum_url = format!("{base_url}/checksum");
        let archive_url = format!("{base_url}/archive.zip");
        let sources = [FfmpegInstallSourceSpec {
            source: FfmpegInstallSource::Gyan,
            archive_url: &archive_url,
            checksum_url: &checksum_url,
            archive_name: GYAN_ARCHIVE_NAME,
        }];
        let directory = tempfile::tempdir().expect("create cancel installer fixture directory");
        let install_root = directory.path().join("ffmpeg");
        let control = DownloadTransferControl::new();
        let cancel_control = control.clone();
        let cancel_signal = Arc::clone(&archive_started);
        let cancel = tokio::spawn(async move {
            while !cancel_signal.load(Ordering::Acquire) {
                tokio::task::yield_now().await;
            }
            cancel_control.cancel();
        });
        let (progress_sender, _) = async_channel::bounded(64);

        let error = run_ffmpeg_install_with_sources(
            uuid::Uuid::new_v4(),
            &install_root,
            None,
            &control,
            &progress_sender,
            &sources,
        )
        .await
        .expect_err("streaming installer should observe cancellation");
        cancel.await.expect("join installer cancellation");
        server.join().expect("join cancellation HTTP fixture");

        assert!(error.is_cancelled());
        assert!(
            !fs::read_dir(&install_root)
                .expect("read cancelled installer root")
                .filter_map(Result::ok)
                .any(|entry| {
                    let name = entry.file_name();
                    let name = name.to_string_lossy();
                    name.starts_with(".staging-") || name.starts_with(".candidate-")
                })
        );
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn pre_cancelled_archive_request_never_reaches_the_loopback_server() {
        let listener =
            TcpListener::bind(("127.0.0.1", 0)).expect("bind pre-cancel installer HTTP fixture");
        listener
            .set_nonblocking(true)
            .expect("make pre-cancel listener nonblocking");
        let address = listener.local_addr().expect("read pre-cancel address");
        let request_observed = Arc::new(AtomicBool::new(false));
        let observed = Arc::clone(&request_observed);
        let server = thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_millis(250);
            while Instant::now() < deadline {
                match listener.accept() {
                    Ok(_) => {
                        observed.store(true, Ordering::Release);
                        return;
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(2));
                    }
                    Err(error) => panic!("pre-cancel fixture accept failed: {error}"),
                }
            }
        });
        let directory = tempfile::tempdir().expect("create pre-cancel installer fixture");
        let archive_path = directory.path().join("ffmpeg.zip");
        let control = DownloadTransferControl::new();
        control.cancel();
        let (progress_sender, _) = async_channel::bounded(4);
        let mut downloaded_bytes = 0_u64;

        let error = download_archive(
            &reqwest::Client::new(),
            &format!("http://{address}/archive.zip"),
            &archive_path,
            &control,
            &progress_sender,
            &mut downloaded_bytes,
        )
        .await
        .expect_err("pre-cancelled archive request must not reach the network");
        server.join().expect("join pre-cancel HTTP fixture");

        assert!(matches!(
            error,
            FfmpegInstallError::Cancelled {
                phase: FfmpegInstallPhase::Connecting
            }
        ));
        assert!(!request_observed.load(Ordering::Acquire));
        assert!(!archive_path.exists());
        assert_eq!(downloaded_bytes, 0);
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn real_http_checksum_mismatch_is_verifying_failure_and_cleans_staging() {
        let archive = cancellable_archive_fixture();
        let wrong_checksum = "00".repeat(32);
        let (base_url, _, server) = start_http_fixture(vec![
            HttpFixtureResponse::ok("/checksum", wrong_checksum.into_bytes(), true),
            HttpFixtureResponse::ok("/archive.zip", archive.clone(), true),
        ]);
        let checksum_url = format!("{base_url}/checksum");
        let archive_url = format!("{base_url}/archive.zip");
        let sources = [FfmpegInstallSourceSpec {
            source: FfmpegInstallSource::Gyan,
            archive_url: &archive_url,
            checksum_url: &checksum_url,
            archive_name: GYAN_ARCHIVE_NAME,
        }];
        let directory = tempfile::tempdir().expect("create checksum installer fixture directory");
        let install_root = directory.path().join("ffmpeg");
        let (progress_sender, _) = async_channel::bounded(64);

        let error = run_ffmpeg_install_with_sources(
            uuid::Uuid::new_v4(),
            &install_root,
            None,
            &DownloadTransferControl::new(),
            &progress_sender,
            &sources,
        )
        .await
        .expect_err("checksum mismatch must reject the downloaded archive");
        server.join().expect("join checksum HTTP fixture");

        assert_eq!(error.phase(), FfmpegInstallPhase::Verifying);
        assert_eq!(error.downloaded_bytes(), archive.len() as u64);
        assert!(
            !fs::read_dir(&install_root)
                .expect("read checksum failure install root")
                .filter_map(Result::ok)
                .any(|entry| entry.file_name().to_string_lossy().starts_with(".staging-"))
        );
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn ffmpeg_source_requests_use_the_shared_manual_proxy_snapshot() {
        let archive = cancellable_archive_fixture();
        let wrong_checksum = "00".repeat(32);
        let (proxy_url, _, server) = start_http_fixture(vec![
            HttpFixtureResponse::ok("/checksum", wrong_checksum.into_bytes(), true),
            HttpFixtureResponse::ok("/archive.zip", archive, true),
        ]);
        let proxy = url::Url::parse(&proxy_url).expect("parse proxy fixture URL");
        let policy = NetworkPolicy::new(NetworkSettings {
            proxy: nxtools_core::NetworkProxySettings {
                mode: nxtools_core::NetworkProxyMode::ManualHttp,
                host: proxy.host_str().expect("proxy host").into(),
                port: proxy.port().expect("proxy port"),
            },
        })
        .expect("manual installer policy");
        let network = policy.snapshot().expect("installer proxy snapshot");
        let sources = [FfmpegInstallSourceSpec {
            source: FfmpegInstallSource::Gyan,
            archive_url: "http://unreachable.invalid/archive.zip",
            checksum_url: "http://unreachable.invalid/checksum",
            archive_name: GYAN_ARCHIVE_NAME,
        }];
        let directory = tempfile::tempdir().expect("create proxied installer fixture");
        let (progress_sender, _) = async_channel::bounded(8);

        let error = run_ffmpeg_install_with_sources_and_network(
            uuid::Uuid::new_v4(),
            &directory.path().join("ffmpeg"),
            None,
            &DownloadTransferControl::new(),
            &progress_sender,
            &sources,
            &network,
        )
        .await
        .expect_err("wrong checksum must fail after both proxied requests");
        server.join().expect("join installer proxy fixture");

        assert_eq!(error.phase(), FfmpegInstallPhase::Verifying);
        assert_eq!(error.failure_kind(), "checksum_mismatch");
    }

    fn decode(hash: &str) -> [u8; 32] {
        let mut output = [0_u8; 32];
        for (index, byte) in output.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&hash[index * 2..index * 2 + 2], 16)
                .expect("decode checksum fixture");
        }
        output
    }

    fn write_zip(path: &Path, entries: &[(&str, &[u8])]) {
        let writer = File::create(path).expect("create zip fixture");
        let mut archive = ZipWriter::new(writer);
        for (name, bytes) in entries {
            archive
                .start_file(*name, SimpleFileOptions::default())
                .expect("start zip entry");
            archive.write_all(bytes).expect("write zip entry");
        }
        let _ = archive.finish().expect("finish zip fixture");
    }

    #[cfg(windows)]
    fn cancellable_archive_fixture() -> Vec<u8> {
        let ffmpeg = fs::read(
            Path::new(&std::env::var("WINDIR").expect("read WINDIR"))
                .join("System32")
                .join("cmd.exe"),
        )
        .expect("read bounded Windows executable fixture");
        let cursor = Cursor::new(Vec::new());
        let mut archive = ZipWriter::new(cursor);
        archive
            .start_file(
                "ffmpeg-fixture/bin/ffmpeg.exe",
                SimpleFileOptions::default(),
            )
            .expect("start executable fixture entry");
        archive
            .write_all(&ffmpeg)
            .expect("write executable fixture entry");
        archive
            .finish()
            .expect("finish executable archive fixture")
            .into_inner()
    }

    #[cfg(windows)]
    fn valid_ffmpeg_archive_fixture(directory: &Path) -> Vec<u8> {
        let source = directory.join("ffmpeg-version-fixture.rs");
        let executable = directory.join("ffmpeg-version-fixture.exe");
        fs::write(
            &source,
            "fn main() { println!(\"ffmpeg version 7.1-fixture\"); }\n",
        )
        .expect("write FFmpeg version fixture source");
        let rustc = std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
        let status = std::process::Command::new(rustc)
            .arg(&source)
            .arg("-C")
            .arg("opt-level=s")
            .arg("-o")
            .arg(&executable)
            .status()
            .expect("compile FFmpeg version fixture");
        assert!(status.success(), "compile FFmpeg version fixture");
        let ffmpeg = fs::read(executable).expect("read FFmpeg version fixture executable");
        let cursor = Cursor::new(Vec::new());
        let mut archive = ZipWriter::new(cursor);
        archive
            .start_file(
                "ffmpeg-fixture/bin/ffmpeg.exe",
                SimpleFileOptions::default(),
            )
            .expect("start valid FFmpeg fixture entry");
        archive
            .write_all(&ffmpeg)
            .expect("write valid FFmpeg fixture entry");
        archive
            .finish()
            .expect("finish valid FFmpeg archive fixture")
            .into_inner()
    }

    #[cfg(windows)]
    fn checksum_hex(bytes: &[u8]) -> String {
        Sha256::digest(bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }

    #[cfg(windows)]
    struct HttpFixtureResponse {
        path: &'static str,
        status: u16,
        body: Vec<u8>,
        include_length: bool,
        chunk_delay: Duration,
    }

    #[cfg(windows)]
    impl HttpFixtureResponse {
        fn ok(path: &'static str, body: Vec<u8>, include_length: bool) -> Self {
            Self {
                path,
                status: 200,
                body,
                include_length,
                chunk_delay: Duration::ZERO,
            }
        }

        fn slow(path: &'static str, body: Vec<u8>) -> Self {
            Self {
                path,
                status: 200,
                body,
                include_length: false,
                chunk_delay: Duration::from_millis(5),
            }
        }

        fn failure(path: &'static str, status: u16) -> Self {
            Self {
                path,
                status,
                body: Vec::new(),
                include_length: true,
                chunk_delay: Duration::ZERO,
            }
        }
    }

    #[cfg(windows)]
    fn start_http_fixture(
        responses: Vec<HttpFixtureResponse>,
    ) -> (String, Arc<AtomicBool>, thread::JoinHandle<()>) {
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind installer HTTP fixture");
        let address = listener
            .local_addr()
            .expect("read installer fixture address");
        let archive_started = Arc::new(AtomicBool::new(false));
        let started = Arc::clone(&archive_started);
        let server = thread::spawn(move || {
            for response in responses {
                let (mut stream, _) = listener.accept().expect("accept installer request");
                stream
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .expect("bound installer fixture read");
                let mut request = [0_u8; 4096];
                let read = stream.read(&mut request).expect("read installer request");
                let request = String::from_utf8_lossy(&request[..read]);
                let request_target = request
                    .lines()
                    .next()
                    .and_then(|line| line.split_whitespace().nth(1))
                    .expect("parse installer request path");
                let request_path = url::Url::parse(request_target)
                    .map(|url| url.path().to_owned())
                    .unwrap_or_else(|_| request_target.to_owned());
                assert_eq!(request_path, response.path);
                let reason = if response.status == 200 {
                    "OK"
                } else {
                    "Fixture Failure"
                };
                let length = if response.include_length {
                    format!("Content-Length: {}\r\n", response.body.len())
                } else {
                    String::new()
                };
                write!(
                    stream,
                    "HTTP/1.1 {} {reason}\r\nConnection: close\r\n{length}\r\n",
                    response.status
                )
                .expect("write installer response headers");
                for chunk in response.body.chunks(4096) {
                    if stream.write_all(chunk).is_err() {
                        break;
                    }
                    if response.path.ends_with(".zip") {
                        started.store(true, Ordering::Release);
                    }
                    if !response.chunk_delay.is_zero() {
                        thread::sleep(response.chunk_delay);
                    }
                }
                let _ = stream.flush();
            }
        });
        (format!("http://{address}"), archive_started, server)
    }
}
