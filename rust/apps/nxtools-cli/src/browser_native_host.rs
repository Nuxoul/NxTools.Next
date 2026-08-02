use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use base64::Engine as _;
use nxtools_download::{
    DOWNLOAD_COMMAND_PROTOCOL_VERSION, DownloadCommand, DownloadCommandEnvelope,
    DownloadCommandOutcome, DownloadQuery, DownloadRequest, DownloadRequestHeader,
    DownloadSubmitMode, DownloadTaskAction, DownloadTaskId, RequestOrigin,
};
use nxtools_download_ipc::{
    DownloadUiDismissRequest, DownloadUiOpenRequest, DownloadUiOpenResponse,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::{connect_download_ui_or_start, connect_or_start, default_download_directory};

pub const NATIVE_HOST_NAME: &str = "com.nxtools.next.browser";
pub const NATIVE_PROTOCOL_VERSION: u16 = 1;
pub const MAX_NATIVE_MESSAGE_BYTES: usize = 768 * 1024;
const MAX_BATCH_ITEMS: usize = 100;
const MAX_HEADERS: usize = 64;
const MAX_HEADER_BYTES: usize = 32 * 1024;
const MAX_BLOB_BYTES: u64 = 512 * 1024 * 1024;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NativeRequest {
    protocol_version: u16,
    request_id: String,
    action: String,
    #[serde(default)]
    idempotency_key: Option<String>,
    #[serde(default)]
    payload: Value,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct NativeResponse {
    protocol_version: u16,
    request_id: String,
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<NativeError>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct NativeError {
    code: String,
    message: String,
    retryable: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BrowserDownloadPayload {
    url: String,
    #[serde(default)]
    file_name: Option<String>,
    #[serde(default)]
    headers: HashMap<String, String>,
    #[serde(default)]
    cookie: Option<String>,
    #[serde(default)]
    size: Option<u64>,
    #[serde(default)]
    supports_range: Option<bool>,
    #[serde(default)]
    open_dialog: bool,
    #[serde(default)]
    start_immediately: Option<bool>,
    #[serde(default)]
    save_directory: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BrowserDownloadTarget {
    OpenDialog,
    Submit(DownloadSubmitMode),
}

struct BlobStage {
    file: File,
    path: PathBuf,
    next_index: u32,
    written_bytes: u64,
    expected_bytes: u64,
    download: BrowserDownloadPayload,
    idempotency_key: String,
}

pub fn is_native_messaging_invocation(arguments: &[String]) -> bool {
    arguments.first().is_some_and(|argument| {
        argument == "browser-native-host"
            || argument.starts_with("chrome-extension://")
            || argument.starts_with("moz-extension://")
            || argument.ends_with("native-host.json")
    }) || arguments
        .iter()
        .any(|argument| argument.starts_with("--parent-window="))
}

pub fn run_stdio() -> Result<()> {
    let executable = std::env::current_exe().context("cannot locate native host executable")?;
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    run(stdin.lock(), stdout.lock(), &executable)
}

fn run(mut reader: impl Read, mut writer: impl Write, executable: &Path) -> Result<()> {
    let mut stages = HashMap::<String, BlobStage>::new();
    while let Some(encoded) = read_message(&mut reader)? {
        let started = Instant::now();
        let decoded = serde_json::from_slice::<NativeRequest>(&encoded);
        let (request_id, response) = match decoded {
            Ok(request) => {
                let request_id = request.request_id.clone();
                let response = handle_request(request, executable, &mut stages, started);
                (request_id, response)
            }
            Err(error) => (
                String::new(),
                failure(
                    "invalid_message",
                    format!("invalid native message: {error}"),
                    false,
                ),
            ),
        };
        write_message(
            &mut writer,
            &NativeResponse {
                protocol_version: NATIVE_PROTOCOL_VERSION,
                request_id,
                ok: response.0,
                result: response.1,
                error: response.2,
            },
        )?;
    }
    Ok(())
}

type ResponseParts = (bool, Option<Value>, Option<NativeError>);

fn handle_request(
    request: NativeRequest,
    executable: &Path,
    stages: &mut HashMap<String, BlobStage>,
    started: Instant,
) -> ResponseParts {
    let correlation = request.request_id.clone();
    let result = (|| -> Result<Value> {
        if request.protocol_version != NATIVE_PROTOCOL_VERSION {
            bail!(
                "unsupported native protocol version {}",
                request.protocol_version
            );
        }
        validate_request_id(&request.request_id)?;
        match request.action.as_str() {
            "ping" | "hello" => Ok(json!({
                "host": NATIVE_HOST_NAME,
                "version": env!("CARGO_PKG_VERSION")
            })),
            "create_task" => {
                let payload = request.payload.get("payload").unwrap_or(&request.payload);
                let download = serde_json::from_value::<BrowserDownloadPayload>(payload.clone())
                    .context("invalid download payload")?;
                let key = validated_idempotency_key(request.idempotency_key.as_deref())?;
                submit_download(executable, download, &key)
            }
            "create_batch" => create_batch(executable, request.payload, request.idempotency_key),
            "dismiss_dialog" => {
                dismiss_download_dialog(executable, &request.request_id, request.payload)
            }
            "get_snapshot" => get_snapshot(executable),
            "task_action" => task_action(executable, request.payload),
            "extension_log" => Ok(json!({ "accepted": true })),
            "blob_begin" => blob_begin(stages, request.payload, request.idempotency_key),
            "blob_chunk" => blob_chunk(stages, request.payload),
            "blob_commit" => blob_commit(stages, request.payload, executable),
            _ => bail!("unsupported native action {}", request.action),
        }
    })();
    match result {
        Ok(value) => {
            emit_diagnostic(
                "browser_native_host.completed",
                &correlation,
                "success",
                started.elapsed(),
                None,
            );
            success(value)
        }
        Err(error) => {
            let code = classify_error(&error);
            emit_diagnostic(
                "browser_native_host.completed",
                &correlation,
                "failed",
                started.elapsed(),
                Some(code),
            );
            failure(code, error.to_string(), code == "host_unavailable")
        }
    }
}

fn dismiss_download_dialog(executable: &Path, request_id: &str, payload: Value) -> Result<Value> {
    let dialog_request_id = payload
        .get("dialogRequestId")
        .and_then(Value::as_str)
        .context("dialogRequestId is required")
        .and_then(|value| Uuid::parse_str(value).context("dialogRequestId is invalid"))?;
    let request_id = Uuid::parse_str(request_id).context("requestId is invalid")?;
    let mut client =
        connect_download_ui_or_start(executable).map_err(|error| anyhow::anyhow!(error.message))?;
    match client.dismiss_dialog(DownloadUiDismissRequest {
        protocol_version: NATIVE_PROTOCOL_VERSION,
        request_id,
        dialog_request_id,
    })? {
        DownloadUiOpenResponse::Dismissed { dialog_request_id } => Ok(json!({
            "ok": true,
            "message": "NxTools 已关闭下载确认窗口",
            "data": { "dialogRequestId": dialog_request_id, "dismissed": true }
        })),
        DownloadUiOpenResponse::Rejected { code, message } => {
            bail!("download UI rejected the dismissal ({code}): {message}")
        }
        DownloadUiOpenResponse::Accepted { .. } => {
            bail!("download UI returned an unexpected dismissal result")
        }
    }
}

fn submit_download(
    executable: &Path,
    payload: BrowserDownloadPayload,
    idempotency_key: &str,
) -> Result<Value> {
    let request_id = deterministic_request_id(idempotency_key);
    let (request, target) = prepare_download_request(payload)?;
    let declared_size = request.declared_size_bytes;
    let declared_supports_ranges = request.declared_supports_ranges;
    if target == BrowserDownloadTarget::OpenDialog {
        let browser_file_name_is_hint = request.file_name.is_some();
        let mut client = connect_download_ui_or_start(executable)
            .map_err(|error| anyhow::anyhow!(error.message))?;
        return match client.open_dialog(DownloadUiOpenRequest {
            protocol_version: NATIVE_PROTOCOL_VERSION,
            request_id,
            idempotency_key: idempotency_key.to_owned(),
            request,
            browser_file_name_is_hint,
        })? {
            DownloadUiOpenResponse::Accepted { dialog_request_id } => Ok(json!({
                "ok": true,
                "message": "NxTools 已打开下载确认窗口",
                "data": {
                    "dialogRequestId": dialog_request_id,
                    "dialogOpened": true,
                    "queued": false,
                    "declaredSize": declared_size,
                    "supportsRange": declared_supports_ranges
                }
            })),
            DownloadUiOpenResponse::Rejected { code, message } => {
                bail!("download UI rejected the request ({code}): {message}")
            }
            DownloadUiOpenResponse::Dismissed { .. } => {
                bail!("download UI returned an unexpected open-dialog result")
            }
        };
    }
    let BrowserDownloadTarget::Submit(mode) = target else {
        unreachable!("dialog target returned before download submission")
    };
    let mut client =
        connect_or_start(executable).map_err(|error| anyhow::anyhow!(error.message))?;
    let result = client.command(DownloadCommandEnvelope {
        protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
        request_id,
        origin: RequestOrigin::Browser,
        command: DownloadCommand::Submit { request, mode },
    })?;
    match result.outcome {
        DownloadCommandOutcome::Accepted {
            task_id: Some(task_id),
        } => Ok(json!({
            "ok": true,
            "message": if mode == DownloadSubmitMode::QueueOnly { "已添加到 NxTools 队列" } else { "NxTools 已接管下载" },
            "data": {
                "taskId": task_id.as_uuid(),
                "queued": mode == DownloadSubmitMode::QueueOnly,
                "declaredSize": declared_size,
                "supportsRange": declared_supports_ranges
            }
        })),
        DownloadCommandOutcome::Rejected { fault } => {
            bail!(
                "download request rejected ({:?}): {}",
                fault.code,
                fault.message
            )
        }
        _ => bail!("download host returned an unexpected submission result"),
    }
}

fn prepare_download_request(
    payload: BrowserDownloadPayload,
) -> Result<(DownloadRequest, BrowserDownloadTarget)> {
    let target = if payload.open_dialog {
        BrowserDownloadTarget::OpenDialog
    } else if payload.start_immediately == Some(false) {
        BrowserDownloadTarget::Submit(DownloadSubmitMode::QueueOnly)
    } else {
        BrowserDownloadTarget::Submit(DownloadSubmitMode::StartNow)
    };
    let (headers, cookie, referrer) = normalize_headers(&payload.headers, payload.cookie)?;
    let request = DownloadRequest {
        source_url: payload.url,
        save_directory: payload
            .save_directory
            .unwrap_or_else(default_download_directory),
        file_name: payload.file_name.filter(|name| !name.trim().is_empty()),
        headers,
        cookie,
        referrer,
        declared_size_bytes: payload.size,
        declared_supports_ranges: payload.supports_range,
        connection_limit: None,
        speed_limit_bytes_per_second: None,
        scheduled_at_unix_ms: None,
        adaptive_segment_policy: None,
    };
    request.validate().context("download request is invalid")?;
    Ok((request, target))
}

fn create_batch(executable: &Path, payload: Value, batch_key: Option<String>) -> Result<Value> {
    let items = payload
        .get("items")
        .and_then(Value::as_array)
        .context("batch items are required")?;
    if items.is_empty() || items.len() > MAX_BATCH_ITEMS {
        bail!("batch must contain between 1 and {MAX_BATCH_ITEMS} items");
    }
    let batch_key = validated_idempotency_key(batch_key.as_deref())?;
    let mut results = Vec::with_capacity(items.len());
    for (index, item) in items.iter().enumerate() {
        let download = serde_json::from_value::<BrowserDownloadPayload>(item.clone())
            .with_context(|| format!("invalid batch item {index}"))?;
        let item_key = format!("{batch_key}:{index}");
        results.push(submit_download(executable, download, &item_key)?);
    }
    Ok(json!({ "ok": true, "data": { "results": results } }))
}

fn get_snapshot(executable: &Path) -> Result<Value> {
    let mut client =
        connect_or_start(executable).map_err(|error| anyhow::anyhow!(error.message))?;
    let result = client.command(DownloadCommandEnvelope {
        protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
        request_id: Uuid::new_v4(),
        origin: RequestOrigin::Browser,
        command: DownloadCommand::Snapshot {
            query: DownloadQuery {
                statuses: Vec::new(),
                search: None,
                limit: Some(50),
            },
        },
    })?;
    let DownloadCommandOutcome::Snapshot(snapshot) = result.outcome else {
        bail!("download host returned an unexpected snapshot result");
    };
    let tasks = snapshot
        .task_details
        .into_iter()
        .map(|detail| {
            let task = detail.task;
            json!({
                "id": task.id().as_uuid(),
                "url": task.request().source_url,
                "displayFileName": task.request().file_name,
                "status": task.status(),
                "downloadedBytes": detail.downloaded_bytes,
                "totalBytes": detail.total_bytes
            })
        })
        .collect::<Vec<_>>();
    Ok(json!({ "ok": true, "data": { "tasks": tasks } }))
}

fn task_action(executable: &Path, payload: Value) -> Result<Value> {
    let task_id = payload
        .get("taskId")
        .and_then(Value::as_str)
        .context("taskId is required")?
        .parse::<Uuid>()
        .map(DownloadTaskId::from_uuid)
        .context("taskId is invalid")?;
    let action = match payload.get("action").and_then(Value::as_str) {
        Some("pause") => DownloadTaskAction::Pause,
        Some("resume") => DownloadTaskAction::Resume,
        Some("cancel") => DownloadTaskAction::Cancel,
        _ => bail!("task action is invalid"),
    };
    let mut client =
        connect_or_start(executable).map_err(|error| anyhow::anyhow!(error.message))?;
    let result = client.command(DownloadCommandEnvelope {
        protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
        request_id: Uuid::new_v4(),
        origin: RequestOrigin::Browser,
        command: DownloadCommand::Task { task_id, action },
    })?;
    match result.outcome {
        DownloadCommandOutcome::Accepted { .. } => Ok(json!({ "ok": true })),
        DownloadCommandOutcome::Rejected { fault } => {
            bail!("task action failed: {}", fault.message)
        }
        _ => bail!("download host returned an unexpected task action result"),
    }
}

fn blob_begin(
    stages: &mut HashMap<String, BlobStage>,
    payload: Value,
    idempotency_key: Option<String>,
) -> Result<Value> {
    let transfer_id = validated_transfer_id(payload.get("transferId").and_then(Value::as_str))?;
    let expected_bytes = payload
        .get("size")
        .and_then(Value::as_u64)
        .context("blob size is required")?;
    if expected_bytes > MAX_BLOB_BYTES {
        bail!("blob exceeds the {MAX_BLOB_BYTES} byte staging limit");
    }
    let download = serde_json::from_value::<BrowserDownloadPayload>(
        payload
            .get("download")
            .cloned()
            .context("blob download metadata is required")?,
    )?;
    let idempotency_key = validated_idempotency_key(idempotency_key.as_deref())?;
    let directory = browser_blob_directory()?;
    std::fs::create_dir_all(&directory)?;
    let path = directory.join(format!("{transfer_id}.blob"));
    let file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&path)
        .context("cannot create browser Blob stage")?;
    stages.insert(
        transfer_id.clone(),
        BlobStage {
            file,
            path,
            next_index: 0,
            written_bytes: 0,
            expected_bytes,
            download,
            idempotency_key,
        },
    );
    Ok(json!({ "accepted": true, "transferId": transfer_id }))
}

fn blob_chunk(stages: &mut HashMap<String, BlobStage>, payload: Value) -> Result<Value> {
    let transfer_id = validated_transfer_id(payload.get("transferId").and_then(Value::as_str))?;
    let index = payload
        .get("index")
        .and_then(Value::as_u64)
        .context("blob chunk index is required")?;
    let data = payload
        .get("data")
        .and_then(Value::as_str)
        .context("blob chunk data is required")?;
    if data.len() > 512 * 1024 {
        bail!("blob chunk exceeds the bounded chunk limit");
    }
    let stage = stages
        .get_mut(&transfer_id)
        .context("blob stage does not exist")?;
    if index != u64::from(stage.next_index) {
        bail!("blob chunks must be sequential");
    }
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(data)
        .context("blob chunk is not valid base64")?;
    let next_size = stage.written_bytes.saturating_add(decoded.len() as u64);
    if next_size > stage.expected_bytes || next_size > MAX_BLOB_BYTES {
        bail!("blob chunk exceeds the declared size");
    }
    stage.file.write_all(&decoded)?;
    stage.written_bytes = next_size;
    stage.next_index = stage.next_index.saturating_add(1);
    Ok(json!({ "accepted": true, "nextIndex": stage.next_index }))
}

fn blob_commit(
    stages: &mut HashMap<String, BlobStage>,
    payload: Value,
    executable: &Path,
) -> Result<Value> {
    let transfer_id = validated_transfer_id(payload.get("transferId").and_then(Value::as_str))?;
    let mut stage = stages
        .remove(&transfer_id)
        .context("blob stage does not exist")?;
    stage.file.flush()?;
    if stage.written_bytes != stage.expected_bytes {
        bail!("blob stage is incomplete");
    }
    let file_name = stage
        .download
        .file_name
        .clone()
        .unwrap_or_else(|| "browser-blob.bin".into());
    let source_url = start_blob_server(stage.path, &file_name)?;
    stage.download.url = source_url;
    stage.download.size = Some(stage.expected_bytes);
    stage.download.supports_range = Some(true);
    submit_download(executable, stage.download, &stage.idempotency_key)
}

fn start_blob_server(path: PathBuf, file_name: &str) -> Result<String> {
    let listener =
        TcpListener::bind(("127.0.0.1", 0)).context("cannot bind Blob staging server")?;
    let address = listener.local_addr()?;
    let token = Uuid::new_v4().simple().to_string();
    let route = format!("/{token}/{}", percent_encode_path_component(file_name));
    let server_route = route.clone();
    std::thread::Builder::new()
        .name("browser-blob-server".into())
        .spawn(move || {
            for connection in listener.incoming() {
                let Ok(stream) = connection else { break };
                let path = path.clone();
                let route = server_route.clone();
                let _ = std::thread::Builder::new()
                    .name("browser-blob-request".into())
                    .spawn(move || {
                        let _ = serve_blob_connection(stream, &path, &route);
                    });
            }
        })?;
    Ok(format!("http://{address}{route}"))
}

fn serve_blob_connection(mut stream: TcpStream, path: &Path, route: &str) -> Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(10)))?;
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut first_line = String::new();
    reader.read_line(&mut first_line)?;
    let mut parts = first_line.split_whitespace();
    let method = parts.next().unwrap_or("");
    let requested_route = parts.next().unwrap_or("");
    if !matches!(method, "GET" | "HEAD") || requested_route != route {
        stream.write_all(
            b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
        )?;
        return Ok(());
    }
    let mut range = None;
    loop {
        let mut line = String::new();
        reader.read_line(&mut line)?;
        if line == "\r\n" || line.is_empty() {
            break;
        }
        if let Some(value) = line
            .strip_prefix("Range: bytes=")
            .or_else(|| line.strip_prefix("range: bytes="))
        {
            range = Some(value.trim().to_owned());
        }
    }
    let mut file = File::open(path)?;
    let total = file.metadata()?.len();
    let (start, end, partial) = parse_http_range(range.as_deref(), total)?;
    let length = end.saturating_sub(start).saturating_add(1);
    let status = if partial {
        "206 Partial Content"
    } else {
        "200 OK"
    };
    write!(
        stream,
        "HTTP/1.1 {status}\r\nAccept-Ranges: bytes\r\nContent-Length: {length}\r\n"
    )?;
    if partial {
        write!(stream, "Content-Range: bytes {start}-{end}/{total}\r\n")?;
    }
    stream.write_all(b"Content-Type: application/octet-stream\r\nConnection: close\r\n\r\n")?;
    if method == "GET" {
        file.seek(SeekFrom::Start(start))?;
        let mut remaining = length;
        let mut buffer = [0_u8; 64 * 1024];
        while remaining > 0 {
            let capacity = buffer.len() as u64;
            let count = file.read(&mut buffer[..remaining.min(capacity) as usize])?;
            if count == 0 {
                break;
            }
            stream.write_all(&buffer[..count])?;
            remaining -= count as u64;
        }
    }
    Ok(())
}

fn parse_http_range(value: Option<&str>, total: u64) -> Result<(u64, u64, bool)> {
    if total == 0 {
        return Ok((0, 0, false));
    }
    let Some(value) = value else {
        return Ok((0, total - 1, false));
    };
    let value = value.trim_end_matches(['\r', '\n']);
    let (start, end) = value.split_once('-').context("invalid HTTP range")?;
    let start = start.parse::<u64>().context("invalid HTTP range start")?;
    let end = if end.is_empty() {
        total - 1
    } else {
        end.parse::<u64>()?
    };
    if start >= total || end < start {
        bail!("HTTP range is outside the Blob");
    }
    Ok((start, end.min(total - 1), true))
}

fn normalize_headers(
    headers: &HashMap<String, String>,
    explicit_cookie: Option<String>,
) -> Result<(Vec<DownloadRequestHeader>, Option<String>, Option<String>)> {
    if headers.len() > MAX_HEADERS {
        bail!("too many request headers");
    }
    let mut total_bytes = 0;
    let mut normalized = Vec::new();
    let mut cookie = explicit_cookie.filter(|value| !value.is_empty());
    let mut referrer = None;
    for (name, value) in headers {
        let name = name.trim();
        let value = value.trim();
        total_bytes += name.len() + value.len();
        if total_bytes > MAX_HEADER_BYTES {
            bail!("request headers exceed the bounded limit");
        }
        if name.eq_ignore_ascii_case("cookie") {
            cookie.get_or_insert_with(|| value.to_owned());
        } else if name.eq_ignore_ascii_case("referer") || name.eq_ignore_ascii_case("referrer") {
            referrer = Some(value.to_owned());
        } else {
            normalized.push(DownloadRequestHeader {
                name: name.to_owned(),
                value: value.to_owned(),
            });
        }
    }
    Ok((normalized, cookie, referrer))
}

fn deterministic_request_id(key: &str) -> Uuid {
    let digest = Sha256::digest(key.as_bytes());
    let mut bytes = [0_u8; 16];
    bytes.copy_from_slice(&digest[..16]);
    bytes[6] = (bytes[6] & 0x0f) | 0x50;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    Uuid::from_bytes(bytes)
}

fn validated_idempotency_key(value: Option<&str>) -> Result<String> {
    let value = value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .context("idempotencyKey is required")?;
    if value.len() > 256 || value.contains(['\r', '\n']) {
        bail!("idempotencyKey is invalid");
    }
    Ok(value.to_owned())
}

fn validate_request_id(value: &str) -> Result<()> {
    if value.is_empty() || value.len() > 128 || value.contains(['\r', '\n']) {
        bail!("requestId is invalid");
    }
    Ok(())
}

fn validated_transfer_id(value: Option<&str>) -> Result<String> {
    let value = value.context("transferId is required")?;
    let parsed = Uuid::parse_str(value).context("transferId is invalid")?;
    Ok(parsed.simple().to_string())
}

fn browser_blob_directory() -> Result<PathBuf> {
    let local = std::env::var_os("LOCALAPPDATA").context("LOCALAPPDATA is unavailable")?;
    Ok(PathBuf::from(local)
        .join("NxTools")
        .join("NxTools.Next")
        .join("data")
        .join("browser-blobs"))
}

fn percent_encode_path_component(value: &str) -> String {
    url::form_urlencoded::byte_serialize(value.as_bytes()).collect()
}

fn read_message(reader: &mut impl Read) -> Result<Option<Vec<u8>>> {
    let mut length = [0_u8; 4];
    match reader.read_exact(&mut length) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(error) => return Err(error.into()),
    }
    let length = u32::from_le_bytes(length) as usize;
    if length == 0 || length > MAX_NATIVE_MESSAGE_BYTES {
        bail!("native message exceeds the bounded frame limit");
    }
    let mut message = vec![0; length];
    reader.read_exact(&mut message)?;
    Ok(Some(message))
}

fn write_message(writer: &mut impl Write, response: &NativeResponse) -> Result<()> {
    let encoded = serde_json::to_vec(response)?;
    if encoded.len() > MAX_NATIVE_MESSAGE_BYTES {
        bail!("native response exceeds the bounded frame limit");
    }
    writer.write_all(&(encoded.len() as u32).to_le_bytes())?;
    writer.write_all(&encoded)?;
    writer.flush()?;
    Ok(())
}

fn success(value: Value) -> ResponseParts {
    (true, Some(value), None)
}

fn failure(code: &str, message: String, retryable: bool) -> ResponseParts {
    (
        false,
        None,
        Some(NativeError {
            code: code.into(),
            message,
            retryable,
        }),
    )
}

fn classify_error(error: &anyhow::Error) -> &'static str {
    let message = error.to_string();
    if message.contains("host") || message.contains("pipe") {
        "host_unavailable"
    } else if message.contains("Blob") || message.contains("blob") {
        "blob_staging"
    } else if message.contains("rejected") {
        "request_rejected"
    } else {
        "invalid_request"
    }
}

fn emit_diagnostic(
    event: &str,
    correlation_id: &str,
    status: &str,
    elapsed: Duration,
    error_code: Option<&str>,
) {
    eprintln!(
        "{}",
        json!({
            "event": event,
            "correlation_id": correlation_id,
            "status": status,
            "elapsed_ms": elapsed.as_millis(),
            "error_code": error_code
        })
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn framing_rejects_oversize_and_preserves_adjacent_messages() {
        let first = br#"{\"protocolVersion\":1}"#;
        let second = br#"{\"protocolVersion\":1,\"requestId\":\"second\"}"#;
        let mut bytes = Vec::new();
        for message in [first.as_slice(), second.as_slice()] {
            bytes.extend_from_slice(&(message.len() as u32).to_le_bytes());
            bytes.extend_from_slice(message);
        }
        let mut cursor = std::io::Cursor::new(bytes);
        assert_eq!(
            read_message(&mut cursor).expect("first frame"),
            Some(first.to_vec())
        );
        assert_eq!(
            read_message(&mut cursor).expect("second frame"),
            Some(second.to_vec())
        );
        assert_eq!(read_message(&mut cursor).expect("end"), None);

        let mut oversized =
            std::io::Cursor::new(((MAX_NATIVE_MESSAGE_BYTES + 1) as u32).to_le_bytes());
        assert!(read_message(&mut oversized).is_err());
    }

    #[test]
    fn idempotency_key_maps_to_a_stable_distinct_command_id() {
        assert_eq!(
            deterministic_request_id("browser:42"),
            deterministic_request_id("browser:42")
        );
        assert_ne!(
            deterministic_request_id("browser:42"),
            deterministic_request_id("browser:43")
        );
    }

    #[test]
    fn native_invocation_detection_accepts_browser_arguments_only() {
        assert!(is_native_messaging_invocation(&[
            "browser-native-host".into()
        ]));
        assert!(is_native_messaging_invocation(&[
            "chrome-extension://abc/".into()
        ]));
        assert!(is_native_messaging_invocation(&[
            "--parent-window=42".into()
        ]));
        assert!(!is_native_messaging_invocation(&[
            "download".into(),
            "list".into()
        ]));
    }

    #[test]
    fn browser_metadata_is_preserved_in_the_durable_download_request() {
        let payload = serde_json::from_value::<BrowserDownloadPayload>(json!({
            "url": "https://example.test/archive.bin",
            "fileName": "archive.bin",
            "saveDirectory": "C:\\Downloads",
            "headers": {
                "Authorization": "Bearer fixture",
                "Referer": "https://example.test/page",
                "Cookie": "header_cookie=fixture"
            },
            "cookie": "explicit_cookie=fixture",
            "size": 123456,
            "supportsRange": true,
            "openDialog": true,
            "startImmediately": false
        }))
        .expect("decode browser payload");

        let (request, target) =
            prepare_download_request(payload).expect("prepare download request");

        assert_eq!(request.source_url, "https://example.test/archive.bin");
        assert_eq!(request.file_name.as_deref(), Some("archive.bin"));
        assert_eq!(request.cookie.as_deref(), Some("explicit_cookie=fixture"));
        assert_eq!(
            request.referrer.as_deref(),
            Some("https://example.test/page")
        );
        assert_eq!(request.declared_size_bytes, Some(123456));
        assert_eq!(request.declared_supports_ranges, Some(true));
        assert_eq!(request.headers.len(), 1);
        assert_eq!(request.headers[0].name, "Authorization");
        assert_eq!(request.headers[0].value, "Bearer fixture");
        assert_eq!(target, BrowserDownloadTarget::OpenDialog);
    }
}
