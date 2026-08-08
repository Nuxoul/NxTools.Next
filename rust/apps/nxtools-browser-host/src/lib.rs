use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use nxtools_core::{AppConfig, AppPaths};
use nxtools_download::{
    DOWNLOAD_COMMAND_PROTOCOL_VERSION, DownloadAdaptiveSegmentPolicy, DownloadCommand,
    DownloadCommandOutcome, DownloadQuery, DownloadRequest, DownloadRequestHeader,
    DownloadSubmitMode, DownloadTaskAction, DownloadTaskId, RequestOrigin,
};
use nxtools_download_ipc::{DOWNLOAD_PIPE_NAME, DownloadIpcClient};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tracing::{info, warn};
use uuid::Uuid;

pub const BROWSER_PROTOCOL_VERSION: u16 = 1;
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

#[derive(Debug, Deserialize)]
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
    let mut input = std::io::stdin().lock();
    let mut output = std::io::stdout().lock();
    while let Some(bytes) = read_native_message(&mut input)? {
        let started = Instant::now();
        let request_id = request_id_from_bytes(&bytes);
        let request_kind = request_kind_from_bytes(&bytes);
        if matches!(request_kind.as_str(), "create_task" | "task_action") {
            info!(
                event = "browser_native_host.request_received",
                %request_id,
                %request_kind,
                "bounded browser native request received"
            );
        }
        let response = match serde_json::from_slice::<BrowserRequest>(&bytes) {
            Ok(request) => handle_request(request, &paths, &executable),
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
                "capabilities": ["ordinary_download", "task_snapshot", "task_action"]
            })),
        )),
        "create_task" => submit_download(request_id, request.payload, paths, executable),
        "get_snapshot" => snapshot(request_id, executable),
        "task_action" => task_action(request_id, request.task_id, request.action, executable),
        "extension_log" => {
            info!(
                event = "browser_extension.event",
                request_id, "bounded browser extension event received"
            );
            Ok(BrowserResponse::ok(request_id, None))
        }
        kind => bail!("unsupported browser request type '{kind}'"),
    }
}

fn submit_download(
    request_id: String,
    payload: Value,
    paths: &AppPaths,
    executable: &Path,
) -> Result<BrowserResponse> {
    let payload: BrowserDownloadPayload =
        serde_json::from_value(payload).context("browser download payload has an invalid shape")?;
    validate_payload(&payload)?;
    let handoff_id = if payload.handoff_id.is_empty() {
        Uuid::parse_str(&request_id).unwrap_or_else(|_| Uuid::new_v4())
    } else {
        Uuid::parse_str(&payload.handoff_id).context("handoffId must be a UUID")?
    };
    let config = AppConfig::load_or_create(paths)?;
    let save_directory = if config.download.default_directory.trim().is_empty() {
        default_download_directory(paths)
    } else {
        PathBuf::from(&config.download.default_directory)
    };
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
        .filter(|(name, _)| !name.eq_ignore_ascii_case("cookie"))
        .map(|(name, value)| DownloadRequestHeader { name, value })
        .collect::<Vec<_>>();
    headers.sort_by(|left, right| left.name.cmp(&right.name));
    let request = DownloadRequest {
        source_url: payload.url,
        save_directory,
        file_name: non_empty(payload.file_name),
        headers,
        cookie: non_empty(payload.cookie),
        referrer,
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
    };
    request.validate()?;
    let open_dialog = payload.open_dialog;
    let mode = if payload.start_immediately {
        DownloadSubmitMode::StartNow
    } else {
        DownloadSubmitMode::QueueOnly
    };
    let command_request_id = correlation_id(&request_id)?;
    let mut client = connect_or_start(executable, !open_dialog)?;
    let result = client.command(nxtools_download::DownloadCommandEnvelope {
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
    })?;
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

fn snapshot(request_id: String, executable: &Path) -> Result<BrowserResponse> {
    let mut client = connect_or_start(executable, true)?;
    let result = client.command(command_envelope(
        correlation_id(&request_id)?,
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
    let mut client = connect_or_start(executable, true)?;
    let result = client.command(command_envelope(
        correlation_id(&request_id)?,
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

fn connect_or_start(executable: &Path, allow_background_start: bool) -> Result<DownloadIpcClient> {
    if let Ok(client) = DownloadIpcClient::connect(DOWNLOAD_PIPE_NAME, Duration::from_millis(150)) {
        return Ok(client);
    }
    if !allow_background_start {
        bail!("the interactive NxTools desktop is not running")
    }
    let desktop = executable
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("NxTools.exe");
    if !desktop.is_file() {
        bail!("packaged desktop host was not found")
    }
    let mut command = Command::new(&desktop);
    command
        .arg("--background")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    command
        .spawn()
        .context("failed to start the desktop download host")?;
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if let Ok(client) =
            DownloadIpcClient::connect(DOWNLOAD_PIPE_NAME, Duration::from_millis(250))
        {
            return Ok(client);
        }
        std::thread::sleep(Duration::from_millis(75));
    }
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
}
