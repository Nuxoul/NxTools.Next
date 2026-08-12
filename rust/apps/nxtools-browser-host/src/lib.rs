use std::collections::{BTreeMap, HashSet};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use nxtools_core::{AppConfig, AppPaths};
use nxtools_download::{
    BrowserBatchSubmissionResult, BrowserBlob, BrowserDownloadSubmission,
    DOWNLOAD_COMMAND_PROTOCOL_VERSION, DownloadAdaptiveSegmentPolicy, DownloadCommand,
    DownloadCommandOutcome, DownloadCommandResult, DownloadFaultCode, DownloadQuery,
    DownloadRequest, DownloadRequestHeader, DownloadSubmitMode, DownloadTaskAction, DownloadTaskId,
    MAX_BROWSER_BATCH_ITEMS, RequestOrigin,
};
use nxtools_download_ipc::{DOWNLOAD_PIPE_NAME, DownloadIpcClient};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tracing::{info, warn};
use uuid::Uuid;

mod blob_upload;
use blob_upload::{BlobUploadService, MAX_BLOB_CHUNK_BYTES, MAX_BLOB_UPLOAD_BYTES};

pub const BROWSER_PROTOCOL_VERSION: u16 = 2;
pub const MAX_NATIVE_MESSAGE_BYTES: usize = 256 * 1024;
const MAX_URL_BYTES: usize = 16 * 1024;
const MAX_FILE_NAME_BYTES: usize = 512;
const MAX_HEADER_COUNT: usize = 64;
const MAX_HEADER_NAME_BYTES: usize = 128;
const MAX_HEADER_VALUE_BYTES: usize = 16 * 1024;
const MAX_COOKIE_BYTES: usize = 64 * 1024;
const MAX_REQUEST_ID_BYTES: usize = 64;
const MAX_REQUEST_KIND_BYTES: usize = 32;
const MAX_EXTENSION_VERSION_BYTES: usize = 32;
const MAX_TASK_ACTION_BYTES: usize = 16;
const MAX_EXTENSION_LOG_EVENT_BYTES: usize = 96;
const INTERACTIVE_DESKTOP_READY_TIMEOUT: Duration = Duration::from_secs(3);
const ALLOWED_EXTENSION_LOG_EVENTS: &[&str] = &[
    "bilibili_tracks_incomplete",
    "bilibili_tracks_selected",
    "batch_handoff_completed",
    "batch_handoff_started",
    "blob_download_intercept_disabled",
    "blob_download_intercept_failed",
    "blob_download_intercepted",
    "blob_download_resource_missing",
    "blob_source_resolved",
    "browser_download_already_preempted",
    "browser_download_site_disabled",
    "content_bridge_inject_failed",
    "content_media_resource_failed",
    "content_resource_choices_built",
    "context_menu_handoff_completed",
    "context_menu_handoff_failed",
    "douyin_video_candidates_empty",
    "douyin_video_selected",
    "drop_target_download",
    "handoff_failed",
    "handoff_prepare",
    "handoff_rejected",
    "handoff_result",
    "page_context_resources_cleared",
    "page_hook_inject_failed",
    "page_media_dom_empty_resources_present",
    "page_media_download_requested",
    "page_media_no_resource",
    "page_media_resource_selected",
    "page_media_resource_selected_missing",
    "page_media_site_match_failed",
    "page_scan_no_receiver",
    "request_resource_no_tab",
    "resource_cached",
    "resource_no_tab",
    "resource_rejected",
    "send_resource_missing",
    "youtube_captured_selected",
    "youtube_progressive_selected",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DesktopStartMode {
    Interactive,
    Background,
}

impl DesktopStartMode {
    const fn for_browser_request(open_dialog: bool) -> Self {
        if open_dialog {
            Self::Interactive
        } else {
            Self::Background
        }
    }

    const fn command_arguments(self) -> &'static [&'static str] {
        match self {
            Self::Interactive => &["--browser-dialog"],
            Self::Background => &["--background"],
        }
    }

    const fn log_name(self) -> &'static str {
        match self {
            Self::Interactive => "interactive",
            Self::Background => "background",
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BrowserRequest {
    #[serde(default = "browser_protocol_version")]
    protocol_version: u16,
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    request_id: String,
    #[serde(default)]
    extension_version: String,
    #[serde(default)]
    payload: Value,
    #[serde(default)]
    task_id: String,
    #[serde(default)]
    action: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BrowserDownloadPayload {
    #[serde(default)]
    handoff_id: String,
    url: String,
    #[serde(default)]
    file_name: String,
    #[serde(default)]
    headers: BTreeMap<String, String>,
    #[serde(default)]
    cookie: String,
    #[serde(default)]
    referrer: String,
    #[serde(default)]
    size: u64,
    #[serde(default)]
    start_immediately: bool,
    #[serde(default)]
    open_dialog: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BrowserBatchDownloadPayload {
    items: Vec<BrowserDownloadPayload>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BrowserExtensionLogPayload {
    event: String,
    #[serde(default)]
    correlation_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BrowserBlobPreparePayload {
    handoff_id: String,
    size: u64,
    #[serde(default)]
    file_name: String,
    #[serde(default)]
    media_type: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BrowserBlobCommitPayload {
    upload_id: String,
    handoff_id: String,
    file_name: String,
    #[serde(default)]
    media_type: String,
    size: u64,
    #[serde(default)]
    open_dialog: bool,
}

fn validate_extension_log_payload(payload: &BrowserExtensionLogPayload) -> Result<Option<Uuid>> {
    bounded(
        "extensionLog.event",
        &payload.event,
        MAX_EXTENSION_LOG_EVENT_BYTES,
    )?;
    if !ALLOWED_EXTENSION_LOG_EVENTS.contains(&payload.event.as_str()) {
        bail!("browser extension log event is invalid");
    }
    payload
        .correlation_id
        .as_deref()
        .filter(|value| !value.is_empty())
        .map(|value| Uuid::parse_str(value).context("extensionLog.correlationId must be a UUID"))
        .transpose()
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct BrowserResponse {
    protocol_version: u16,
    request_id: String,
    ok: bool,
    accepted: bool,
    duplicate: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    handoff_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    task_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
}

const fn browser_protocol_version() -> u16 {
    BROWSER_PROTOCOL_VERSION
}

pub fn run() -> Result<()> {
    let paths = AppPaths::discover()?;
    let _log_guard = nxtools_core::init_file_logging(&paths, "browser-host")?;
    validate_browser_origin(std::env::args().skip(1))?;
    info!(
        event = "browser_native_host.started",
        "browser native messaging host started"
    );

    let executable = std::env::current_exe().context("failed to locate browser native host")?;
    let blob_uploads = BlobUploadService::start(paths.data_dir.join("browser-blob-staging"))?;
    let mut input = std::io::stdin().lock();
    let mut output = std::io::stdout().lock();
    while let Some(bytes) = read_native_message(&mut input)? {
        let started = Instant::now();
        let request_id = request_id_from_bytes(&bytes);
        let request_kind = request_kind_from_bytes(&bytes);
        if matches!(
            request_kind.as_str(),
            "create_task"
                | "create_tasks"
                | "task_action"
                | "prepare_blob_upload"
                | "commit_blob_upload"
        ) {
            info!(
                event = "browser_native_host.request_received",
                %request_id,
                %request_kind,
                "bounded browser native request received"
            );
        }
        let response = match serde_json::from_slice::<BrowserRequest>(&bytes) {
            Ok(request) => handle_request(request, &paths, &executable, &blob_uploads),
            Err(error) => Err(anyhow::anyhow!("invalid browser request: {error}")),
        }
        .unwrap_or_else(|error| {
            warn!(
                event = "browser_native_host.request_failed",
                %request_id,
                %request_kind,
                elapsed_ms = started.elapsed().as_millis() as u64,
                error_present = true,
                "browser native request failed"
            );
            BrowserResponse::failed(request_id, "request_rejected", error.to_string())
        });
        write_native_message(&mut output, &response)?;
    }
    info!(
        event = "browser_native_host.stopped",
        "browser native messaging host stopped"
    );
    Ok(())
}

fn validate_browser_origin(arguments: impl IntoIterator<Item = String>) -> Result<()> {
    let arguments = arguments.into_iter().collect::<Vec<_>>();
    if arguments.is_empty() {
        bail!("browser origin argument is missing");
    }
    let allowed_chromium_origin = "chrome-extension://haohhnpjfggpdadhcbjakjpkmkljkpif/";
    let allowed_firefox_id = "nxtools-download-interceptor@nxtools.local";
    if arguments
        .iter()
        .any(|argument| argument == allowed_chromium_origin || argument == allowed_firefox_id)
    {
        return Ok(());
    }
    bail!("browser origin is not allowed")
}

fn handle_request(
    request: BrowserRequest,
    paths: &AppPaths,
    executable: &Path,
    blob_uploads: &BlobUploadService,
) -> Result<BrowserResponse> {
    let request_id = if request.request_id.is_empty() {
        Uuid::new_v4().to_string()
    } else {
        request.request_id.clone()
    };
    if request.protocol_version != BROWSER_PROTOCOL_VERSION {
        bail!(
            "unsupported browser protocol version {}; expected {}",
            request.protocol_version,
            BROWSER_PROTOCOL_VERSION
        );
    }
    bounded("requestId", &request_id, MAX_REQUEST_ID_BYTES)?;
    bounded("type", &request.kind, MAX_REQUEST_KIND_BYTES)?;
    bounded(
        "extensionVersion",
        &request.extension_version,
        MAX_EXTENSION_VERSION_BYTES,
    )?;
    bounded("taskId", &request.task_id, MAX_REQUEST_ID_BYTES)?;
    bounded("action", &request.action, MAX_TASK_ACTION_BYTES)?;
    match request.kind.as_str() {
        "hello" => Ok(BrowserResponse::ok(
            request_id,
            Some(json!({
                "hostVersion": env!("CARGO_PKG_VERSION"),
                "extensionVersion": request.extension_version,
                "capabilities": ["ordinary_download", "batch_download_v1", "blob_upload_v1", "task_snapshot", "task_action"]
            })),
        )),
        "create_task" => submit_download(request_id, request.payload, paths, executable),
        "create_tasks" => submit_download_batch(request_id, request.payload, paths, executable),
        "prepare_blob_upload" => prepare_blob_upload(request_id, request.payload, blob_uploads),
        "commit_blob_upload" => {
            commit_blob_upload(request_id, request.payload, paths, executable, blob_uploads)
        }
        "get_snapshot" => snapshot(request_id, executable),
        "task_action" => task_action(request_id, request.task_id, request.action, executable),
        "extension_log" => {
            let payload: BrowserExtensionLogPayload = serde_json::from_value(request.payload)
                .context("browser extension log has an invalid shape")?;
            let correlation_id = validate_extension_log_payload(&payload)?;
            if let Some(correlation_id) = correlation_id {
                info!(
                    event = "browser_extension.event",
                    %correlation_id,
                    extension_event = %payload.event,
                    "bounded correlated browser extension event received"
                );
            } else {
                info!(
                    event = "browser_extension.event",
                    request_id,
                    extension_event = %payload.event,
                    "bounded browser extension event received"
                );
            }
            Ok(BrowserResponse::ok(request_id, None))
        }
        kind => bail!("unsupported browser request type '{kind}'"),
    }
}

fn prepare_blob_upload(
    request_id: String,
    payload: Value,
    blob_uploads: &BlobUploadService,
) -> Result<BrowserResponse> {
    let started_at = Instant::now();
    let payload: BrowserBlobPreparePayload = serde_json::from_value(payload)
        .context("browser Blob prepare payload has an invalid shape")?;
    validate_blob_metadata(payload.size, &payload.file_name, &payload.media_type)?;
    let correlation_id = correlation_id(&request_id)?;
    let handoff_id = Uuid::parse_str(&payload.handoff_id).context("handoffId must be a UUID")?;
    let capability = blob_uploads.prepare(payload.size, correlation_id, handoff_id)?;
    info!(
        event = "browser_native_host.blob_upload_prepared",
        %correlation_id,
        %handoff_id,
        upload_id = %capability.id,
        size = payload.size,
        expires_after_ms = capability.expires_after_ms,
        elapsed_ms = started_at.elapsed().as_millis(),
        "bounded browser Blob upload capability prepared"
    );
    Ok(BrowserResponse::ok(
        request_id,
        Some(json!({
            "uploadId": capability.id,
            "uploadUrl": format!("http://{}{}", blob_uploads.address(), capability.path),
            "maxBytes": MAX_BLOB_UPLOAD_BYTES,
            "chunkBytes": MAX_BLOB_CHUNK_BYTES,
            "expiresAfterMs": capability.expires_after_ms
        })),
    ))
}

fn commit_blob_upload(
    request_id: String,
    payload: Value,
    paths: &AppPaths,
    executable: &Path,
    blob_uploads: &BlobUploadService,
) -> Result<BrowserResponse> {
    let started_at = Instant::now();
    let payload: BrowserBlobCommitPayload = serde_json::from_value(payload)
        .context("browser Blob commit payload has an invalid shape")?;
    validate_blob_metadata(payload.size, &payload.file_name, &payload.media_type)?;
    let upload_id = Uuid::parse_str(&payload.upload_id).context("uploadId must be a UUID")?;
    let handoff_id = Uuid::parse_str(&payload.handoff_id).context("handoffId must be a UUID")?;
    let correlation_id = correlation_id(&request_id)?;
    let config = AppConfig::load_or_create(paths)?;
    let save_directory = if config.download.default_directory.trim().is_empty() {
        default_download_directory(paths)
    } else {
        PathBuf::from(&config.download.default_directory)
    };
    let candidate =
        blob_uploads.commit_candidate(upload_id, payload.size, correlation_id, handoff_id)?;
    let mut client = match connect_or_start(
        executable,
        DesktopStartMode::for_browser_request(payload.open_dialog),
        correlation_id,
    ) {
        Ok(client) => client,
        Err(error) => {
            blob_uploads.forget(upload_id);
            cleanup_blob_staging(
                &candidate.path,
                correlation_id,
                handoff_id,
                upload_id,
                "desktop_connect_failed",
                started_at,
            );
            return Err(error);
        }
    };
    let blob = BrowserBlob {
        id: upload_id,
        file_name: payload.file_name.clone(),
        media_type: non_empty(payload.media_type),
        size: payload.size,
    };
    let command = nxtools_download::DownloadCommandEnvelope {
        protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
        request_id: correlation_id,
        origin: RequestOrigin::Browser,
        command: if payload.open_dialog {
            DownloadCommand::BrowserOpenBlobDialog {
                handoff_id,
                blob,
                save_directory,
            }
        } else {
            DownloadCommand::BrowserImportBlob {
                handoff_id,
                blob,
                save_directory,
                file_name: payload.file_name,
                category_id: None,
                save_directory_explicit: false,
            }
        },
    };
    let result = if payload.open_dialog {
        command_browser_dialog_when_ready(&mut client, command)
    } else {
        client.command(command)
    };
    let result = match result {
        Ok(result) => result,
        Err(error) => {
            blob_uploads.forget(upload_id);
            cleanup_blob_staging(
                &candidate.path,
                correlation_id,
                handoff_id,
                upload_id,
                "engine_command_failed",
                started_at,
            );
            return Err(error);
        }
    };
    blob_uploads.forget(upload_id);
    match result.outcome {
        DownloadCommandOutcome::BrowserAccepted { task_id, duplicate } => {
            if duplicate && candidate.registered {
                cleanup_blob_staging(
                    &candidate.path,
                    correlation_id,
                    handoff_id,
                    upload_id,
                    "duplicate_import",
                    started_at,
                );
            }
            info!(
                event = "browser_native_host.blob_import_accepted",
                %correlation_id,
                %handoff_id,
                %upload_id,
                ?task_id,
                duplicate,
                size = payload.size,
                elapsed_ms = started_at.elapsed().as_millis(),
                "completed browser Blob accepted by the download engine"
            );
            Ok(BrowserResponse::accepted(
                request_id, handoff_id, task_id, duplicate, false,
            ))
        }
        DownloadCommandOutcome::BrowserDialogAccepted { duplicate, .. } => {
            if duplicate && candidate.registered {
                cleanup_blob_staging(
                    &candidate.path,
                    correlation_id,
                    handoff_id,
                    upload_id,
                    "duplicate_dialog",
                    started_at,
                );
            }
            info!(
                event = "browser_native_host.blob_dialog_accepted",
                %correlation_id,
                %handoff_id,
                %upload_id,
                duplicate,
                size = payload.size,
                elapsed_ms = started_at.elapsed().as_millis(),
                "completed browser Blob confirmation window accepted"
            );
            Ok(BrowserResponse::dialog_accepted(
                request_id, handoff_id, duplicate,
            ))
        }
        DownloadCommandOutcome::Rejected { fault } => {
            cleanup_blob_staging(
                &candidate.path,
                correlation_id,
                handoff_id,
                upload_id,
                "engine_rejected",
                started_at,
            );
            warn!(
                event = "browser_native_host.blob_import_rejected",
                %correlation_id,
                %handoff_id,
                %upload_id,
                fault_code = ?fault.code,
                retryable = fault.retryable,
                elapsed_ms = started_at.elapsed().as_millis(),
                "completed browser Blob was rejected by the download engine"
            );
            Ok(BrowserResponse::failed(
                request_id,
                "blob_import_rejected",
                fault.message,
            ))
        }
        _ => {
            cleanup_blob_staging(
                &candidate.path,
                correlation_id,
                handoff_id,
                upload_id,
                "unexpected_engine_result",
                started_at,
            );
            bail!("download engine returned an unexpected Blob import result")
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BlobStagingCleanupOutcome {
    Removed,
    Missing,
    Failed(i32),
}

fn cleanup_blob_staging(
    path: &Path,
    correlation_id: Uuid,
    handoff_id: Uuid,
    upload_id: Uuid,
    reason: &'static str,
    started_at: Instant,
) -> BlobStagingCleanupOutcome {
    match std::fs::remove_file(path) {
        Ok(()) => {
            info!(
                event = "browser_native_host.blob_staging_cleanup_completed",
                %correlation_id,
                %handoff_id,
                %upload_id,
                reason,
                elapsed_ms = started_at.elapsed().as_millis(),
                "browser Blob staging cleanup completed"
            );
            BlobStagingCleanupOutcome::Removed
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            BlobStagingCleanupOutcome::Missing
        }
        Err(error) => {
            let native_error_code = error.raw_os_error().unwrap_or_default();
            warn!(
                event = "browser_native_host.blob_staging_cleanup_failed",
                %correlation_id,
                %handoff_id,
                %upload_id,
                reason,
                native_error_code,
                elapsed_ms = started_at.elapsed().as_millis(),
                "browser Blob staging cleanup failed"
            );
            BlobStagingCleanupOutcome::Failed(native_error_code)
        }
    }
}

fn validate_blob_metadata(size: u64, file_name: &str, media_type: &str) -> Result<()> {
    if size == 0 || size > MAX_BLOB_UPLOAD_BYTES {
        bail!("Blob size is outside the bounded upload limit");
    }
    bounded("fileName", file_name, MAX_FILE_NAME_BYTES)?;
    if file_name.trim().is_empty() {
        bail!("Blob fileName is required");
    }
    bounded("mediaType", media_type, 256)?;
    if media_type.contains(['\r', '\n']) {
        bail!("Blob mediaType is invalid");
    }
    Ok(())
}

fn submit_download(
    request_id: String,
    payload: Value,
    paths: &AppPaths,
    executable: &Path,
) -> Result<BrowserResponse> {
    let payload: BrowserDownloadPayload =
        serde_json::from_value(payload).context("browser download payload has an invalid shape")?;
    let handoff_id = if payload.handoff_id.is_empty() {
        Uuid::parse_str(&request_id).unwrap_or_else(|_| Uuid::new_v4())
    } else {
        Uuid::parse_str(&payload.handoff_id).context("handoffId must be a UUID")?
    };
    let open_dialog = payload.open_dialog;
    let start_immediately = payload.start_immediately;
    let config = AppConfig::load_or_create(paths)?;
    let save_directory = if config.download.default_directory.trim().is_empty() {
        default_download_directory(paths)
    } else {
        PathBuf::from(&config.download.default_directory)
    };
    let request = build_browser_download_request(payload, &config, save_directory)?;
    let mode = if start_immediately {
        DownloadSubmitMode::StartNow
    } else {
        DownloadSubmitMode::QueueOnly
    };
    let command_request_id = correlation_id(&request_id)?;
    let mut client = connect_or_start(
        executable,
        DesktopStartMode::for_browser_request(open_dialog),
        command_request_id,
    )?;
    let command = nxtools_download::DownloadCommandEnvelope {
        protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
        request_id: command_request_id,
        origin: RequestOrigin::Browser,
        command: if open_dialog {
            DownloadCommand::BrowserOpenDialog {
                handoff_id,
                request,
            }
        } else {
            DownloadCommand::BrowserSubmit {
                handoff_id,
                request,
                mode,
            }
        },
    };
    let result = if open_dialog {
        command_browser_dialog_when_ready(&mut client, command)?
    } else {
        client.command(command)?
    };
    match result.outcome {
        DownloadCommandOutcome::BrowserAccepted { task_id, duplicate } => {
            info!(
                event = "browser_native_host.handoff_accepted",
                correlation_id = %command_request_id,
                %handoff_id,
                ?task_id,
                duplicate,
                "browser handoff accepted by the download engine"
            );
            Ok(BrowserResponse::accepted(
                request_id,
                handoff_id,
                task_id,
                duplicate,
                mode == DownloadSubmitMode::QueueOnly,
            ))
        }
        DownloadCommandOutcome::BrowserDialogAccepted {
            handoff_id: accepted_handoff_id,
            duplicate,
        } => {
            if accepted_handoff_id != handoff_id {
                bail!("download engine returned a mismatched browser dialog handoff ID");
            }
            info!(
                event = "browser_native_host.dialog_accepted",
                correlation_id = %command_request_id,
                %handoff_id,
                duplicate,
                "browser confirmation-window request accepted by the desktop"
            );
            Ok(BrowserResponse::dialog_accepted(
                request_id, handoff_id, duplicate,
            ))
        }
        DownloadCommandOutcome::Rejected { fault } => Ok(BrowserResponse::failed(
            request_id,
            "download_rejected",
            fault.message,
        )),
        _ => bail!("download engine returned an unexpected browser submission result"),
    }
}

fn build_browser_download_request(
    payload: BrowserDownloadPayload,
    config: &AppConfig,
    save_directory: PathBuf,
) -> Result<DownloadRequest> {
    validate_payload(&payload)?;
    let referrer = non_empty(payload.referrer.clone()).or_else(|| {
        payload
            .headers
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case("referer"))
            .and_then(|(_, value)| non_empty(value.clone()))
    });
    let mut headers = payload
        .headers
        .into_iter()
        .filter(|(name, _)| {
            !name.eq_ignore_ascii_case("cookie")
                && !name.eq_ignore_ascii_case("referer")
                && !nxtools_download::is_download_transport_owned_header(name)
        })
        .map(|(name, value)| DownloadRequestHeader { name, value })
        .collect::<Vec<_>>();
    headers.sort_by(|left, right| left.name.cmp(&right.name));
    let request = DownloadRequest {
        source_url: payload.url,
        save_directory,
        save_directory_explicit: false,
        file_name: non_empty(payload.file_name),
        headers,
        cookie: non_empty(payload.cookie),
        referrer,
        certificate_policy: Default::default(),
        connection_limit: Some(config.download.default_connection_limit),
        speed_limit_bytes_per_second: None,
        expected_total_bytes: (payload.size > 0).then_some(payload.size),
        scheduled_at_unix_ms: None,
        adaptive_segment_policy: Some(DownloadAdaptiveSegmentPolicy {
            enabled: config.download.adaptive_segment_scaling_enabled,
            max_segments: config
                .download
                .adaptive_segment_max_segments
                .max(config.download.default_connection_limit),
            min_segment_mib: config.download.adaptive_segment_min_mib,
        }),
        media: None,
        category_id: None,
        category_name: None,
        duplicate_strategy: nxtools_download::DownloadDuplicateStrategy::UseDefault,
    };
    request.validate()?;
    Ok(request)
}

fn build_browser_batch_command(
    payload: BrowserBatchDownloadPayload,
    config: &AppConfig,
    save_directory: PathBuf,
    correlation_id: Uuid,
) -> Result<nxtools_download::DownloadCommandEnvelope> {
    if payload.items.is_empty() || payload.items.len() > MAX_BROWSER_BATCH_ITEMS {
        bail!("browser batch must contain between 1 and {MAX_BROWSER_BATCH_ITEMS} items");
    }
    let mut handoff_ids = HashSet::with_capacity(payload.items.len());
    let mut submissions = Vec::with_capacity(payload.items.len());
    for item in payload.items {
        if item.open_dialog || item.start_immediately {
            bail!("browser batch items must use queue-only mode");
        }
        let handoff_id = Uuid::parse_str(&item.handoff_id).context("handoffId must be a UUID")?;
        if !handoff_ids.insert(handoff_id) {
            bail!("browser batch contains a duplicate handoffId");
        }
        submissions.push(BrowserDownloadSubmission {
            handoff_id,
            request: build_browser_download_request(item, config, save_directory.clone())?,
        });
    }
    Ok(command_envelope(
        correlation_id,
        DownloadCommand::BrowserBatchSubmit { submissions },
        RequestOrigin::Browser,
    ))
}

fn submit_download_batch(
    request_id: String,
    payload: Value,
    paths: &AppPaths,
    executable: &Path,
) -> Result<BrowserResponse> {
    let started_at = Instant::now();
    let payload: BrowserBatchDownloadPayload = serde_json::from_value(payload)
        .context("browser batch download payload has an invalid shape")?;
    let config = AppConfig::load_or_create(paths)?;
    let save_directory = if config.download.default_directory.trim().is_empty() {
        default_download_directory(paths)
    } else {
        PathBuf::from(&config.download.default_directory)
    };
    let correlation_id = correlation_id(&request_id)?;
    let item_count = payload.items.len();
    let command = build_browser_batch_command(payload, &config, save_directory, correlation_id)?;
    info!(
        event = "browser_native_host.batch_started",
        %correlation_id,
        item_count,
        "bounded browser batch handoff started"
    );
    let mut client = connect_or_start(executable, DesktopStartMode::Background, correlation_id)?;
    let result = client.command(command)?;
    match result.outcome {
        DownloadCommandOutcome::BrowserBatchAccepted { results } => {
            let accepted_count = results
                .iter()
                .filter(|result| result.fault.is_none())
                .count();
            let duplicate_count = results.iter().filter(|result| result.duplicate).count();
            info!(
                event = "browser_native_host.batch_completed",
                %correlation_id,
                item_count = results.len(),
                accepted_count,
                duplicate_count,
                rejected_count = results.len().saturating_sub(accepted_count),
                elapsed_ms = started_at.elapsed().as_millis() as u64,
                "bounded browser batch handoff completed"
            );
            Ok(BrowserResponse::batch_accepted(request_id, results))
        }
        DownloadCommandOutcome::Rejected { fault } => {
            warn!(
                event = "browser_native_host.batch_rejected",
                %correlation_id,
                item_count,
                elapsed_ms = started_at.elapsed().as_millis() as u64,
                "download engine rejected the bounded browser batch"
            );
            Ok(BrowserResponse::failed(
                request_id,
                "batch_download_rejected",
                fault.message,
            ))
        }
        _ => bail!("download engine returned an unexpected browser batch result"),
    }
}

fn snapshot(request_id: String, executable: &Path) -> Result<BrowserResponse> {
    let command_request_id = correlation_id(&request_id)?;
    let mut client =
        connect_or_start(executable, DesktopStartMode::Background, command_request_id)?;
    let result = client.command(command_envelope(
        command_request_id,
        DownloadCommand::Snapshot {
            query: DownloadQuery {
                statuses: Vec::new(),
                search: None,
                limit: Some(80),
            },
        },
        RequestOrigin::Browser,
    ))?;
    let DownloadCommandOutcome::Snapshot(snapshot) = result.outcome else {
        bail!("download engine returned an unexpected snapshot result");
    };
    let tasks = snapshot
        .task_details
        .into_iter()
        .map(|detail| {
            json!({
                "id": detail.task.id().as_uuid(),
                "fileName": detail.task.request().file_name.clone(),
                "displayFileName": detail.task.request().file_name.clone().unwrap_or_else(|| "download".into()),
                "status": detail.task.status(),
                "downloadedBytes": detail.downloaded_bytes,
                "totalBytes": detail.total_bytes.unwrap_or_default()
            })
        })
        .collect::<Vec<_>>();
    Ok(BrowserResponse::ok(
        request_id,
        Some(json!({ "tasks": tasks, "sequence": snapshot.sequence })),
    ))
}

fn task_action(
    request_id: String,
    task_id: String,
    action: String,
    executable: &Path,
) -> Result<BrowserResponse> {
    let task_id = Uuid::parse_str(&task_id)
        .map(DownloadTaskId::from_uuid)
        .context("taskId must be a UUID")?;
    let action = match action.as_str() {
        "pause" => DownloadTaskAction::Pause,
        "resume" | "retry" => DownloadTaskAction::Resume,
        "cancel" => DownloadTaskAction::Cancel,
        _ => bail!("unsupported task action '{action}'"),
    };
    let command_request_id = correlation_id(&request_id)?;
    let mut client =
        connect_or_start(executable, DesktopStartMode::Background, command_request_id)?;
    let result = client.command(command_envelope(
        command_request_id,
        DownloadCommand::Task { task_id, action },
        RequestOrigin::Browser,
    ))?;
    match result.outcome {
        DownloadCommandOutcome::Accepted { .. } => Ok(BrowserResponse::ok(request_id, None)),
        DownloadCommandOutcome::Rejected { fault } => Ok(BrowserResponse::failed(
            request_id,
            "task_action_rejected",
            fault.message,
        )),
        _ => bail!("download engine returned an unexpected task action result"),
    }
}

fn validate_payload(payload: &BrowserDownloadPayload) -> Result<()> {
    bounded("url", &payload.url, MAX_URL_BYTES)?;
    bounded("fileName", &payload.file_name, MAX_FILE_NAME_BYTES)?;
    bounded("referrer", &payload.referrer, MAX_URL_BYTES)?;
    bounded("cookie", &payload.cookie, MAX_COOKIE_BYTES)?;
    if payload.headers.len() > MAX_HEADER_COUNT {
        bail!("headers exceed the bounded count limit");
    }
    for (name, value) in &payload.headers {
        bounded("header name", name, MAX_HEADER_NAME_BYTES)?;
        bounded("header value", value, MAX_HEADER_VALUE_BYTES)?;
    }
    Ok(())
}

fn bounded(field: &str, value: &str, maximum: usize) -> Result<()> {
    if value.len() > maximum {
        bail!("{field} exceeds the bounded length limit");
    }
    Ok(())
}

fn non_empty(value: String) -> Option<String> {
    (!value.trim().is_empty()).then_some(value)
}

fn command_envelope(
    request_id: Uuid,
    command: DownloadCommand,
    origin: RequestOrigin,
) -> nxtools_download::DownloadCommandEnvelope {
    nxtools_download::DownloadCommandEnvelope {
        protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
        request_id,
        origin,
        command,
    }
}

fn browser_dialog_waiting_for_interactive_consumer(result: &DownloadCommandResult) -> bool {
    matches!(
        &result.outcome,
        DownloadCommandOutcome::Rejected { fault }
            if fault.code == DownloadFaultCode::Unavailable && fault.retryable
    )
}

fn command_browser_dialog_when_ready(
    client: &mut DownloadIpcClient,
    command: nxtools_download::DownloadCommandEnvelope,
) -> Result<DownloadCommandResult> {
    command_browser_dialog_with_retry(command, INTERACTIVE_DESKTOP_READY_TIMEOUT, |command| {
        client.command(command.clone())
    })
}

fn command_browser_dialog_with_retry(
    command: nxtools_download::DownloadCommandEnvelope,
    ready_timeout: Duration,
    mut send_command: impl FnMut(
        &nxtools_download::DownloadCommandEnvelope,
    ) -> Result<DownloadCommandResult>,
) -> Result<DownloadCommandResult> {
    let correlation_id = command.request_id;
    let started = Instant::now();
    let deadline = started + ready_timeout;
    let mut attempts = 1_u16;
    let mut retry_delay = Duration::from_millis(100);
    let mut result = send_command(&command)?;
    if !browser_dialog_waiting_for_interactive_consumer(&result) {
        return Ok(result);
    }
    info!(
        event = "browser_native_host.dialog_consumer_waiting",
        %correlation_id,
        "download pipe is ready while the interactive dialog consumer is still starting"
    );
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if !remaining.is_zero() {
            std::thread::sleep(retry_delay.min(remaining));
        }
        if Instant::now() >= deadline {
            warn!(
                event = "browser_native_host.dialog_consumer_timeout",
                %correlation_id,
                attempts,
                elapsed_ms = started.elapsed().as_millis() as u64,
                "interactive desktop did not attach the browser dialog consumer before the deadline"
            );
            return Ok(result);
        }
        attempts = attempts.saturating_add(1);
        result = send_command(&command)?;
        if !browser_dialog_waiting_for_interactive_consumer(&result) {
            if matches!(
                result.outcome,
                DownloadCommandOutcome::BrowserDialogAccepted { .. }
            ) {
                info!(
                    event = "browser_native_host.dialog_consumer_ready",
                    %correlation_id,
                    attempts,
                    elapsed_ms = started.elapsed().as_millis() as u64,
                    "interactive desktop accepted the browser confirmation-window request"
                );
            }
            return Ok(result);
        }
        retry_delay = (retry_delay * 2).min(Duration::from_secs(1));
    }
}

fn desktop_start_command(
    desktop: &Path,
    start_mode: DesktopStartMode,
    correlation_id: Uuid,
) -> Command {
    let mut command = Command::new(desktop);
    command
        .args(start_mode.command_arguments())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    if start_mode == DesktopStartMode::Interactive {
        command
            .arg("--browser-correlation-id")
            .arg(correlation_id.to_string());
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    command
}

fn connect_or_start(
    executable: &Path,
    start_mode: DesktopStartMode,
    correlation_id: Uuid,
) -> Result<DownloadIpcClient> {
    if let Ok(client) = DownloadIpcClient::connect(DOWNLOAD_PIPE_NAME, Duration::from_millis(150)) {
        return Ok(client);
    }
    let desktop = executable
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("NxTools.exe");
    if !desktop.is_file() {
        bail!("packaged desktop host was not found")
    }
    let started = Instant::now();
    info!(
        event = "browser_native_host.desktop_start_requested",
        %correlation_id,
        start_mode = start_mode.log_name(),
        "browser native host requested the packaged desktop"
    );
    let child = desktop_start_command(&desktop, start_mode, correlation_id)
        .spawn()
        .inspect_err(|error| {
            warn!(
                event = "browser_native_host.desktop_start_failed",
                %correlation_id,
                start_mode = start_mode.log_name(),
                native_error_code = error.raw_os_error().unwrap_or_default(),
                elapsed_ms = started.elapsed().as_millis() as u64,
                "browser native host could not start the packaged desktop"
            );
        })
        .context("failed to start the desktop download host")?;
    info!(
        event = "browser_native_host.desktop_started",
        %correlation_id,
        start_mode = start_mode.log_name(),
        process_id = child.id(),
        elapsed_ms = started.elapsed().as_millis() as u64,
        "browser native host started the packaged desktop"
    );
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if let Ok(client) =
            DownloadIpcClient::connect(DOWNLOAD_PIPE_NAME, Duration::from_millis(250))
        {
            info!(
                event = "browser_native_host.desktop_ready",
                %correlation_id,
                start_mode = start_mode.log_name(),
                elapsed_ms = started.elapsed().as_millis() as u64,
                "packaged desktop download pipe became available"
            );
            return Ok(client);
        }
        std::thread::sleep(Duration::from_millis(75));
    }
    warn!(
        event = "browser_native_host.desktop_ready_timeout",
        %correlation_id,
        start_mode = start_mode.log_name(),
        elapsed_ms = started.elapsed().as_millis() as u64,
        "packaged desktop did not expose the download pipe before the deadline"
    );
    bail!("desktop download host did not become available")
}

fn default_download_directory(paths: &AppPaths) -> PathBuf {
    std::env::var_os("USERPROFILE")
        .map(PathBuf::from)
        .map(|home| home.join("Downloads"))
        .unwrap_or_else(|| paths.data_dir.join("downloads"))
}

fn request_id_from_bytes(bytes: &[u8]) -> String {
    serde_json::from_slice::<Value>(bytes)
        .ok()
        .and_then(|value| value.get("requestId")?.as_str().map(str::to_owned))
        .filter(|value| value.len() <= MAX_REQUEST_ID_BYTES)
        .unwrap_or_else(|| Uuid::new_v4().to_string())
}

fn request_kind_from_bytes(bytes: &[u8]) -> String {
    serde_json::from_slice::<Value>(bytes)
        .ok()
        .and_then(|value| value.get("type")?.as_str().map(str::to_owned))
        .filter(|value| value.len() <= MAX_REQUEST_KIND_BYTES)
        .unwrap_or_else(|| "invalid".into())
}

fn correlation_id(request_id: &str) -> Result<Uuid> {
    Uuid::parse_str(request_id).context("requestId must be a UUID")
}

pub fn read_native_message(reader: &mut impl Read) -> Result<Option<Vec<u8>>> {
    let mut length = [0_u8; 4];
    match reader.read_exact(&mut length) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(error) => return Err(error).context("failed to read native message length"),
    }
    let length = u32::from_le_bytes(length) as usize;
    if length == 0 || length > MAX_NATIVE_MESSAGE_BYTES {
        bail!("native message length is outside the bounded limit");
    }
    let mut bytes = vec![0_u8; length];
    reader
        .read_exact(&mut bytes)
        .context("failed to read native message payload")?;
    Ok(Some(bytes))
}

pub fn write_native_message(writer: &mut impl Write, response: &impl Serialize) -> Result<()> {
    let bytes = serde_json::to_vec(response).context("failed to encode native response")?;
    if bytes.len() > MAX_NATIVE_MESSAGE_BYTES {
        bail!("native response exceeds the bounded limit");
    }
    let length = u32::try_from(bytes.len()).context("native response length overflow")?;
    writer.write_all(&length.to_le_bytes())?;
    writer.write_all(&bytes)?;
    writer.flush()?;
    Ok(())
}

impl BrowserResponse {
    fn ok(request_id: String, data: Option<Value>) -> Self {
        Self {
            protocol_version: BROWSER_PROTOCOL_VERSION,
            request_id,
            ok: true,
            accepted: false,
            duplicate: false,
            handoff_id: None,
            task_id: None,
            message: Some("NxTools Native Messaging 已连接".into()),
            error: None,
            data,
        }
    }

    fn accepted(
        request_id: String,
        handoff_id: Uuid,
        task_id: DownloadTaskId,
        duplicate: bool,
        queued: bool,
    ) -> Self {
        Self {
            protocol_version: BROWSER_PROTOCOL_VERSION,
            request_id,
            ok: true,
            accepted: true,
            duplicate,
            handoff_id: Some(handoff_id.to_string()),
            task_id: Some(task_id.as_uuid().to_string()),
            message: Some(if duplicate {
                "重复交接已映射到现有任务".into()
            } else {
                "下载任务已接受".into()
            }),
            error: None,
            data: Some(json!({
                "taskId": task_id.as_uuid(),
                "duplicate": duplicate,
                "queued": queued
            })),
        }
    }

    fn dialog_accepted(request_id: String, handoff_id: Uuid, duplicate: bool) -> Self {
        Self {
            protocol_version: BROWSER_PROTOCOL_VERSION,
            request_id,
            ok: true,
            accepted: true,
            duplicate,
            handoff_id: Some(handoff_id.to_string()),
            task_id: None,
            message: Some(if duplicate {
                "重复交接已由现有确认窗口处理".into()
            } else {
                "下载确认窗口已打开".into()
            }),
            error: None,
            data: Some(json!({
                "openDialog": true,
                "duplicate": duplicate,
                "queued": false
            })),
        }
    }

    fn batch_accepted(request_id: String, results: Vec<BrowserBatchSubmissionResult>) -> Self {
        let accepted_count = results
            .iter()
            .filter(|result| result.fault.is_none())
            .count();
        let duplicate_count = results.iter().filter(|result| result.duplicate).count();
        let item_count = results.len();
        let items = results
            .into_iter()
            .map(|result| {
                json!({
                    "handoffId": result.handoff_id,
                    "accepted": result.fault.is_none(),
                    "duplicate": result.duplicate,
                    "taskId": result.task_id.map(|task_id| task_id.as_uuid().to_string()),
                    "error": result.fault.map(|fault| fault.message),
                })
            })
            .collect::<Vec<_>>();
        Self {
            protocol_version: BROWSER_PROTOCOL_VERSION,
            request_id,
            ok: true,
            accepted: accepted_count == item_count,
            duplicate: item_count > 0 && duplicate_count == item_count,
            handoff_id: None,
            task_id: None,
            message: Some(if accepted_count == item_count {
                format!("已将 {accepted_count} 个资源加入 NxTools 队列")
            } else {
                format!("已接受 {accepted_count}/{item_count} 个资源")
            }),
            error: None,
            data: Some(json!({
                "items": items,
                "acceptedCount": accepted_count,
                "duplicateCount": duplicate_count,
                "rejectedCount": item_count.saturating_sub(accepted_count),
                "queued": true,
            })),
        }
    }

    fn failed(request_id: String, code: &str, message: String) -> Self {
        Self {
            protocol_version: BROWSER_PROTOCOL_VERSION,
            request_id,
            ok: false,
            accepted: false,
            duplicate: false,
            handoff_id: None,
            task_id: None,
            message: Some(message.clone()),
            error: Some(code.into()),
            data: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blob_staging_cleanup_reports_real_remove_success_and_failure() {
        let directory = std::env::temp_dir().join(format!(
            "nxtools-browser-host-cleanup-{}-{}",
            std::process::id(),
            Uuid::new_v4()
        ));
        std::fs::create_dir_all(&directory).expect("create cleanup fixture");
        let file = directory.join("duplicate.blob");
        std::fs::write(&file, b"duplicate").expect("create duplicate staging");
        assert_eq!(
            cleanup_blob_staging(
                &file,
                Uuid::new_v4(),
                Uuid::new_v4(),
                Uuid::new_v4(),
                "duplicate_import",
                Instant::now(),
            ),
            BlobStagingCleanupOutcome::Removed
        );
        let blocked = directory.join("blocked.blob");
        std::fs::create_dir(&blocked).expect("create blocked cleanup target");
        assert!(matches!(
            cleanup_blob_staging(
                &blocked,
                Uuid::new_v4(),
                Uuid::new_v4(),
                Uuid::new_v4(),
                "duplicate_import",
                Instant::now(),
            ),
            BlobStagingCleanupOutcome::Failed(_)
        ));
        std::fs::remove_dir_all(directory).expect("remove cleanup fixture");
    }

    #[test]
    fn native_message_round_trip_uses_little_endian_length_prefix() {
        let mut bytes = Vec::new();
        write_native_message(&mut bytes, &json!({ "type": "hello" })).expect("write frame");
        let decoded = read_native_message(&mut bytes.as_slice())
            .expect("read frame")
            .expect("frame exists");
        assert_eq!(
            serde_json::from_slice::<Value>(&decoded).expect("decode response JSON"),
            json!({ "type": "hello" })
        );
    }

    #[test]
    fn oversized_native_message_is_rejected_before_allocation() {
        let bytes = ((MAX_NATIVE_MESSAGE_BYTES + 1) as u32).to_le_bytes();
        assert!(read_native_message(&mut bytes.as_slice()).is_err());
    }

    #[test]
    fn native_host_accepts_only_the_preserved_extension_identities() {
        assert!(
            validate_browser_origin([String::from(
                "chrome-extension://haohhnpjfggpdadhcbjakjpkmkljkpif/"
            )])
            .is_ok()
        );
        assert!(
            validate_browser_origin([String::from("nxtools-download-interceptor@nxtools.local")])
                .is_ok()
        );
        assert!(
            validate_browser_origin([String::from(
                "chrome-extension://aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa/"
            )])
            .is_err()
        );
    }

    #[test]
    fn extension_log_keeps_only_bounded_event_identity() {
        let accepted = BrowserExtensionLogPayload {
            event: "resource_cached".into(),
            correlation_id: Some(Uuid::new_v4().to_string()),
        };
        assert!(
            validate_extension_log_payload(&accepted)
                .expect("bounded extension event should pass")
                .is_some()
        );

        let invalid = BrowserExtensionLogPayload {
            event: "not_allowlisted".into(),
            correlation_id: None,
        };
        assert!(validate_extension_log_payload(&invalid).is_err());
        let invalid_correlation = BrowserExtensionLogPayload {
            event: "resource_cached".into(),
            correlation_id: Some("not-a-uuid".into()),
        };
        assert!(validate_extension_log_payload(&invalid_correlation).is_err());
    }

    #[test]
    fn dialog_acceptance_echoes_handoff_without_claiming_a_task() {
        let handoff_id = Uuid::new_v4();
        let response = BrowserResponse::dialog_accepted("request-1".into(), handoff_id, false);
        let expected_handoff_id = handoff_id.to_string();

        assert!(response.ok);
        assert!(response.accepted);
        assert_eq!(
            response.handoff_id.as_deref(),
            Some(expected_handoff_id.as_str())
        );
        assert!(response.task_id.is_none());
        assert_eq!(
            response
                .data
                .as_ref()
                .and_then(|data| data["openDialog"].as_bool()),
            Some(true)
        );
        assert_eq!(
            response
                .data
                .as_ref()
                .and_then(|data| data["queued"].as_bool()),
            Some(false)
        );
    }

    #[test]
    fn browser_request_start_policy_distinguishes_dialog_from_direct_submit() {
        let correlation_id = Uuid::new_v4();
        let correlation_id_argument = correlation_id.to_string();
        let interactive = DesktopStartMode::for_browser_request(true);
        let background = DesktopStartMode::for_browser_request(false);
        let interactive_command =
            desktop_start_command(Path::new("NxTools.exe"), interactive, correlation_id);
        let background_command =
            desktop_start_command(Path::new("NxTools.exe"), background, correlation_id);

        assert_eq!(interactive, DesktopStartMode::Interactive);
        assert_eq!(interactive_command.get_program(), "NxTools.exe");
        assert_eq!(
            interactive_command.get_args().collect::<Vec<_>>(),
            vec![
                "--browser-dialog",
                "--browser-correlation-id",
                correlation_id_argument.as_str(),
            ]
        );
        assert_eq!(background, DesktopStartMode::Background);
        assert_eq!(
            background_command.get_args().collect::<Vec<_>>(),
            vec!["--background"]
        );
    }

    #[test]
    fn browser_dialog_retry_loop_reaches_a_real_engine_consumer_after_startup() {
        let engine = nxtools_download::DownloadEngine::open_in_memory().expect("open engine");
        let consumer_engine = engine.clone();
        let consumer = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(40));
            let dialogs = consumer_engine
                .take_browser_dialog_receiver()
                .expect("attach browser dialog consumer");
            dialogs
                .recv_blocking()
                .expect("receive browser dialog")
                .completion
                .send(Ok(()))
                .expect("confirm browser dialog");
        });
        let command = browser_dialog_command();
        let result =
            command_browser_dialog_with_retry(command, Duration::from_secs(1), |command| {
                Ok(engine.handle_command(command.clone()))
            })
            .expect("command browser dialog");
        consumer.join().expect("join dialog consumer");

        assert!(matches!(
            result.outcome,
            DownloadCommandOutcome::BrowserDialogAccepted {
                duplicate: false,
                ..
            }
        ));
    }

    #[test]
    fn browser_dialog_retry_budget_expires_before_a_second_command() {
        let engine = nxtools_download::DownloadEngine::open_in_memory().expect("open engine");
        let attempts = std::cell::Cell::new(0_u16);
        let result = command_browser_dialog_with_retry(
            browser_dialog_command(),
            Duration::from_millis(40),
            |command| {
                attempts.set(attempts.get().saturating_add(1));
                Ok(engine.handle_command(command.clone()))
            },
        )
        .expect("command browser dialog");

        assert_eq!(attempts.get(), 1);
        assert!(browser_dialog_waiting_for_interactive_consumer(&result));
    }

    #[test]
    fn browser_batch_payload_maps_to_one_bounded_queue_only_domain_command() {
        let config = AppConfig::default();
        let handoff_ids = [Uuid::new_v4(), Uuid::new_v4()];
        let payload = BrowserBatchDownloadPayload {
            items: handoff_ids
                .iter()
                .enumerate()
                .map(|(index, handoff_id)| BrowserDownloadPayload {
                    handoff_id: handoff_id.to_string(),
                    url: format!("https://example.test/{index}.bin"),
                    file_name: format!("{index}.bin"),
                    headers: BTreeMap::new(),
                    cookie: String::new(),
                    referrer: String::new(),
                    size: 0,
                    start_immediately: false,
                    open_dialog: false,
                })
                .collect(),
        };

        let command = build_browser_batch_command(
            payload,
            &config,
            PathBuf::from(r"C:\Downloads"),
            Uuid::new_v4(),
        )
        .expect("build browser batch command");
        let DownloadCommand::BrowserBatchSubmit { submissions } = command.command else {
            panic!("expected browser batch command");
        };
        assert_eq!(
            submissions
                .iter()
                .map(|submission| submission.handoff_id)
                .collect::<Vec<_>>(),
            handoff_ids
        );
    }

    #[test]
    fn browser_request_drops_transport_owned_headers_before_domain_validation() {
        let mut headers = BTreeMap::new();
        headers.insert("Range".into(), "bytes=100-199".into());
        headers.insert("Host".into(), "untrusted.example".into());
        headers.insert("Referer".into(), "https://page.example/".into());
        headers.insert("Authorization".into(), "bounded-test-value".into());
        let request = build_browser_download_request(
            BrowserDownloadPayload {
                handoff_id: Uuid::new_v4().to_string(),
                url: "https://example.test/file.bin".into(),
                file_name: "file.bin".into(),
                headers,
                cookie: String::new(),
                referrer: String::new(),
                size: 0,
                start_immediately: false,
                open_dialog: false,
            },
            &AppConfig::default(),
            PathBuf::from(r"C:\Downloads"),
        )
        .expect("build browser request");

        assert!(
            request
                .headers
                .iter()
                .all(|header| !nxtools_download::is_download_transport_owned_header(&header.name))
        );
        assert!(
            request
                .headers
                .iter()
                .any(|header| header.name.eq_ignore_ascii_case("authorization"))
        );
        assert_eq!(request.referrer.as_deref(), Some("https://page.example/"));
        assert!(
            request
                .headers
                .iter()
                .all(|header| !header.name.eq_ignore_ascii_case("referer"))
        );
    }

    #[test]
    fn browser_batch_response_echoes_each_handoff_and_created_task_identity() {
        let handoff_ids = [Uuid::new_v4(), Uuid::new_v4()];
        let task_ids = [DownloadTaskId::new(), DownloadTaskId::new()];
        let response = BrowserResponse::batch_accepted(
            Uuid::new_v4().to_string(),
            handoff_ids
                .into_iter()
                .zip(task_ids)
                .enumerate()
                .map(
                    |(index, (handoff_id, task_id))| BrowserBatchSubmissionResult {
                        handoff_id,
                        task_id: Some(task_id),
                        duplicate: index == 0,
                        fault: None,
                    },
                )
                .collect(),
        );

        let data = response.data.expect("batch response data");
        assert!(response.ok);
        assert!(response.accepted);
        assert!(!response.duplicate);
        assert_eq!(data["queued"], json!(true));
        assert_eq!(data["items"][0]["handoffId"], json!(handoff_ids[0]));
        assert_eq!(data["items"][0]["taskId"], json!(task_ids[0].as_uuid()));
        assert_eq!(data["items"][1]["handoffId"], json!(handoff_ids[1]));
        assert_eq!(data["items"][1]["taskId"], json!(task_ids[1].as_uuid()));
    }

    #[test]
    fn browser_batch_rejects_empty_oversized_and_interactive_payloads_before_engine_contact() {
        let config = AppConfig::default();
        let destination = PathBuf::from(r"C:\Downloads");
        let correlation_id = Uuid::new_v4();
        assert!(
            build_browser_batch_command(
                BrowserBatchDownloadPayload { items: Vec::new() },
                &config,
                destination.clone(),
                correlation_id,
            )
            .is_err()
        );

        let item = BrowserDownloadPayload {
            handoff_id: Uuid::new_v4().to_string(),
            url: "https://example.test/item.bin".into(),
            file_name: "item.bin".into(),
            headers: BTreeMap::new(),
            cookie: String::new(),
            referrer: String::new(),
            size: 0,
            start_immediately: false,
            open_dialog: false,
        };
        assert!(
            build_browser_batch_command(
                BrowserBatchDownloadPayload {
                    items: vec![item.clone(); nxtools_download::MAX_BROWSER_BATCH_ITEMS + 1],
                },
                &config,
                destination.clone(),
                correlation_id,
            )
            .is_err()
        );
        assert!(
            build_browser_batch_command(
                BrowserBatchDownloadPayload {
                    items: vec![BrowserDownloadPayload {
                        open_dialog: true,
                        ..item
                    }],
                },
                &config,
                destination,
                correlation_id,
            )
            .is_err()
        );
    }

    fn browser_dialog_command() -> nxtools_download::DownloadCommandEnvelope {
        nxtools_download::DownloadCommandEnvelope {
            protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
            request_id: Uuid::new_v4(),
            origin: RequestOrigin::Browser,
            command: DownloadCommand::BrowserOpenDialog {
                handoff_id: Uuid::new_v4(),
                request: DownloadRequest {
                    source_url: "https://example.invalid/download.bin".into(),
                    save_directory: PathBuf::from(r"C:\Downloads"),
                    save_directory_explicit: false,
                    file_name: Some("download.bin".into()),
                    headers: Vec::new(),
                    cookie: None,
                    referrer: None,
                    certificate_policy: Default::default(),
                    connection_limit: Some(1),
                    speed_limit_bytes_per_second: None,
                    expected_total_bytes: None,
                    scheduled_at_unix_ms: None,
                    adaptive_segment_policy: None,
                    media: None,
                    category_id: None,
                    category_name: None,
                    duplicate_strategy: nxtools_download::DownloadDuplicateStrategy::UseDefault,
                },
            },
        }
    }
}
