use anyhow::{Context, Result, bail};
use nxtools_download::{
    DOWNLOAD_COMMAND_PROTOCOL_VERSION, DownloadCommand, DownloadCommandEnvelope,
    DownloadCommandResult, DownloadEngine, DownloadEventEnvelope, DownloadFault, DownloadFaultCode,
    DownloadStatus,
};
use nxtools_ipc::{
    LocalPipeConnection, accept_current_user_pipe, connect_local_pipe, read_bounded_sync_frame,
};
use serde::{Deserialize, Serialize};
use std::io::{BufRead, Write};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU8, AtomicU64, AtomicUsize, Ordering},
};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use thiserror::Error;

pub const DOWNLOAD_PIPE_NAME: &str = r"\\.\pipe\nxtools-next-download-v1";
pub const DOWNLOAD_IPC_PROTOCOL_VERSION: u16 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DownloadHostReadiness {
    #[default]
    Starting,
    Ready,
    Draining,
    Stopped,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DownloadHostSnapshot {
    pub readiness: DownloadHostReadiness,
    pub engine_generation: u64,
    pub engine_sequence: u64,
    pub active_task_count: u32,
    pub connected_client_count: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_failure_code: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "frame", content = "data", rename_all = "snake_case")]
pub enum DownloadIpcFrame {
    Hello {
        protocol_version: u16,
        client_version: String,
        #[serde(default)]
        engine_generation: Option<u64>,
        #[serde(default)]
        engine_sequence: Option<u64>,
    },
    HelloAck {
        protocol_version: u16,
        server_version: String,
        #[serde(default)]
        host_snapshot: Option<DownloadHostSnapshot>,
    },
    Command(DownloadCommandEnvelope),
    Result(DownloadCommandResult),
    Event(DownloadEventEnvelope),
    Error {
        code: DownloadIpcErrorCode,
        message: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DownloadIpcErrorCode {
    ProtocolIncompatible,
    InvalidFrame,
    Internal,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("download IPC protocol is incompatible: {message}")]
pub struct DownloadIpcProtocolIncompatible {
    pub message: String,
}
#[derive(Clone)]
pub struct DownloadHostActivity {
    clients: Arc<AtomicUsize>,
    last_activity_unix_ms: Arc<AtomicU64>,
    readiness: Arc<AtomicU8>,
    last_failure_code: Arc<Mutex<Option<String>>>,
}

impl Default for DownloadHostActivity {
    fn default() -> Self {
        Self {
            clients: Arc::new(AtomicUsize::new(0)),
            last_activity_unix_ms: Arc::new(AtomicU64::new(0)),
            readiness: Arc::new(AtomicU8::new(readiness_code(
                DownloadHostReadiness::Starting,
            ))),
            last_failure_code: Arc::new(Mutex::new(None)),
        }
    }
}

impl DownloadHostActivity {
    pub fn connected_clients(&self) -> usize {
        self.clients.load(Ordering::Acquire)
    }

    pub fn last_activity_unix_ms(&self) -> u64 {
        self.last_activity_unix_ms.load(Ordering::Acquire)
    }

    pub fn readiness(&self) -> DownloadHostReadiness {
        readiness_from_code(self.readiness.load(Ordering::Acquire))
    }

    pub fn set_ready(&self) {
        self.readiness.store(
            readiness_code(DownloadHostReadiness::Ready),
            Ordering::Release,
        );
    }

    pub fn begin_draining(&self) {
        self.readiness.store(
            readiness_code(DownloadHostReadiness::Draining),
            Ordering::Release,
        );
    }

    pub fn mark_stopped(&self) {
        self.readiness.store(
            readiness_code(DownloadHostReadiness::Stopped),
            Ordering::Release,
        );
    }

    pub fn mark_failed(&self, failure_code: impl Into<String>) {
        if let Ok(mut failure) = self.last_failure_code.lock() {
            *failure = Some(failure_code.into());
        }
        self.readiness.store(
            readiness_code(DownloadHostReadiness::Failed),
            Ordering::Release,
        );
    }

    fn touch(&self) {
        self.last_activity_unix_ms
            .store(now_unix_ms(), Ordering::Release);
    }

    fn snapshot(&self, engine: &DownloadEngine) -> DownloadHostSnapshot {
        let active_task_count = engine
            .tasks()
            .map(|tasks| {
                tasks
                    .iter()
                    .filter(|task| {
                        matches!(
                            task.task.status(),
                            DownloadStatus::Probing
                                | DownloadStatus::Waiting
                                | DownloadStatus::Downloading
                                | DownloadStatus::Finalizing
                                | DownloadStatus::Verifying
                        )
                    })
                    .count() as u32
            })
            .unwrap_or_default();
        DownloadHostSnapshot {
            readiness: self.readiness(),
            engine_generation: engine.session_generation(),
            engine_sequence: engine.event_sequence(),
            active_task_count,
            connected_client_count: self.connected_clients() as u32,
            last_failure_code: self
                .last_failure_code
                .lock()
                .ok()
                .and_then(|failure| failure.clone()),
        }
    }
}

fn readiness_code(readiness: DownloadHostReadiness) -> u8 {
    match readiness {
        DownloadHostReadiness::Starting => 0,
        DownloadHostReadiness::Ready => 1,
        DownloadHostReadiness::Draining => 2,
        DownloadHostReadiness::Stopped => 3,
        DownloadHostReadiness::Failed => 4,
    }
}

fn readiness_from_code(code: u8) -> DownloadHostReadiness {
    match code {
        1 => DownloadHostReadiness::Ready,
        2 => DownloadHostReadiness::Draining,
        3 => DownloadHostReadiness::Stopped,
        4 => DownloadHostReadiness::Failed,
        _ => DownloadHostReadiness::Starting,
    }
}

pub struct DownloadIpcClient {
    connection: LocalPipeConnection,
    host_snapshot: Option<DownloadHostSnapshot>,
}

impl DownloadIpcClient {
    pub fn connect(pipe_name: &str, timeout: Duration) -> Result<Self> {
        let mut connection = connect_local_pipe(pipe_name, timeout)?;
        write_frame(
            &mut connection.writer,
            &DownloadIpcFrame::Hello {
                protocol_version: DOWNLOAD_IPC_PROTOCOL_VERSION,
                client_version: env!("CARGO_PKG_VERSION").into(),
                engine_generation: None,
                engine_sequence: None,
            },
        )?;
        match read_frame(&mut connection.reader)? {
            Some(DownloadIpcFrame::HelloAck {
                protocol_version: DOWNLOAD_IPC_PROTOCOL_VERSION,
                host_snapshot,
                ..
            }) => Ok(Self {
                connection,
                host_snapshot,
            }),
            Some(DownloadIpcFrame::Error {
                code: DownloadIpcErrorCode::ProtocolIncompatible,
                message,
            }) => Err(DownloadIpcProtocolIncompatible { message }.into()),
            Some(frame) => bail!("download host returned an invalid handshake frame: {frame:?}"),
            None => bail!("download host disconnected during handshake"),
        }
    }

    pub fn host_snapshot(&self) -> Option<&DownloadHostSnapshot> {
        self.host_snapshot.as_ref()
    }

    pub fn command(&mut self, command: DownloadCommandEnvelope) -> Result<DownloadCommandResult> {
        write_frame(
            &mut self.connection.writer,
            &DownloadIpcFrame::Command(command),
        )?;
        match read_frame(&mut self.connection.reader)? {
            Some(DownloadIpcFrame::Result(result)) => Ok(result),
            Some(DownloadIpcFrame::Error { code, message }) => {
                bail!("download host rejected the IPC request ({code:?}): {message}")
            }
            Some(frame) => bail!("download host returned an unexpected frame: {frame:?}"),
            None => bail!("download host disconnected before returning a result"),
        }
    }

    pub fn next_event(&mut self) -> Result<Option<DownloadEventEnvelope>> {
        loop {
            match read_frame(&mut self.connection.reader)? {
                Some(DownloadIpcFrame::Event(event)) => return Ok(Some(event)),
                Some(DownloadIpcFrame::Error { code, message }) => {
                    bail!("download host ended the event stream ({code:?}): {message}")
                }
                Some(_) => {}
                None => return Ok(None),
            }
        }
    }
}

pub fn run_download_host(
    pipe_name: &str,
    engine: DownloadEngine,
    activity: DownloadHostActivity,
) -> Result<()> {
    activity.touch();
    activity.set_ready();
    loop {
        let connection = match accept_current_user_pipe(pipe_name)
            .with_context(|| format!("failed to accept download client on {pipe_name}"))
        {
            Ok(connection) => connection,
            Err(error) => {
                activity.mark_failed("ipc_accept_failed");
                return Err(error);
            }
        };
        let engine = engine.clone();
        let activity = activity.clone();
        std::thread::Builder::new()
            .name("download-ipc-client".into())
            .spawn(move || {
                let _client = ConnectedClient::new(activity.clone());
                if let Err(error) = handle_client(connection, engine, &activity) {
                    tracing::warn!(%error, "download IPC client disconnected with an error");
                }
            })
            .context("failed to start download IPC client thread")?;
    }
}

fn is_mutating_command(command: &DownloadCommand) -> bool {
    matches!(
        command,
        DownloadCommand::Submit { .. }
            | DownloadCommand::SubmitResolved { .. }
            | DownloadCommand::BrowserSubmitResolved { .. }
            | DownloadCommand::BrowserSubmit { .. }
            | DownloadCommand::BrowserBatchSubmit { .. }
            | DownloadCommand::BrowserOpenDialog { .. }
            | DownloadCommand::BrowserOpenBlobDialog { .. }
            | DownloadCommand::BrowserImportBlob { .. }
            | DownloadCommand::BrowserDiscardBlob { .. }
            | DownloadCommand::Task { .. }
            | DownloadCommand::Batch { .. }
            | DownloadCommand::Queue { .. }
            | DownloadCommand::SetGlobalSpeedLimit { .. }
    )
}

fn handle_client(
    mut connection: LocalPipeConnection,
    engine: DownloadEngine,
    activity: &DownloadHostActivity,
) -> Result<()> {
    let hello = read_frame(&mut connection.reader)?
        .context("download IPC client disconnected before handshake")?;
    let DownloadIpcFrame::Hello {
        protocol_version, ..
    } = hello
    else {
        write_frame(
            &mut connection.writer,
            &DownloadIpcFrame::Error {
                code: DownloadIpcErrorCode::InvalidFrame,
                message: "the first frame must be hello".into(),
            },
        )?;
        bail!("download IPC client did not begin with hello");
    };
    if protocol_version != DOWNLOAD_IPC_PROTOCOL_VERSION {
        write_frame(
            &mut connection.writer,
            &DownloadIpcFrame::Error {
                code: DownloadIpcErrorCode::ProtocolIncompatible,
                message: format!(
                    "client requested {protocol_version}; host supports {DOWNLOAD_IPC_PROTOCOL_VERSION}"
                ),
            },
        )?;
        return Ok(());
    }
    write_frame(
        &mut connection.writer,
        &DownloadIpcFrame::HelloAck {
            protocol_version: DOWNLOAD_IPC_PROTOCOL_VERSION,
            server_version: env!("CARGO_PKG_VERSION").into(),
            host_snapshot: Some(activity.snapshot(&engine)),
        },
    )?;

    while let Some(frame) = read_frame(&mut connection.reader)? {
        activity.touch();
        let DownloadIpcFrame::Command(command) = frame else {
            write_frame(
                &mut connection.writer,
                &DownloadIpcFrame::Error {
                    code: DownloadIpcErrorCode::InvalidFrame,
                    message: "expected a download command".into(),
                },
            )?;
            continue;
        };
        let request_id = command.request_id;
        if activity.readiness() == DownloadHostReadiness::Draining
            && is_mutating_command(&command.command)
        {
            let result = DownloadCommandResult {
                request_id,
                outcome: nxtools_download::DownloadCommandOutcome::Rejected {
                    fault: DownloadFault {
                        code: DownloadFaultCode::Unavailable,
                        message: "download host is draining and no longer accepts new mutations"
                            .into(),
                        retryable: true,
                    },
                },
            };
            write_frame(&mut connection.writer, &DownloadIpcFrame::Result(result))?;
            continue;
        }
        let subscription = if matches!(command.command, DownloadCommand::Subscribe { .. }) {
            Some(engine.subscribe_events()?)
        } else {
            None
        };
        let result = engine.handle_command(command);
        let should_stream = subscription.is_some()
            && matches!(
                &result.outcome,
                nxtools_download::DownloadCommandOutcome::Accepted { .. }
                    | nxtools_download::DownloadCommandOutcome::Subscribed { .. }
            );
        write_frame(&mut connection.writer, &DownloadIpcFrame::Result(result))?;
        if should_stream && let Some(events) = subscription {
            while let Ok(event) = events.recv_blocking() {
                activity.touch();
                write_frame(&mut connection.writer, &DownloadIpcFrame::Event(event))?;
            }
            return Ok(());
        }
    }
    Ok(())
}

fn write_frame(writer: &mut impl Write, frame: &DownloadIpcFrame) -> Result<()> {
    let encoded = serde_json::to_string(frame).context("failed to encode download IPC frame")?;
    if encoded.len() > nxtools_protocol::MAX_FRAME_BYTES {
        bail!("download IPC frame exceeds the bounded frame limit");
    }
    writer
        .write_all(encoded.as_bytes())
        .context("failed to write download IPC frame")?;
    writer
        .write_all(b"\n")
        .context("failed to terminate download IPC frame")?;
    writer.flush().context("failed to flush download IPC frame")
}

fn read_frame(reader: &mut impl BufRead) -> Result<Option<DownloadIpcFrame>> {
    read_bounded_sync_frame(reader)?
        .map(|frame| serde_json::from_str(&frame).context("failed to decode download IPC frame"))
        .transpose()
}

struct ConnectedClient {
    activity: DownloadHostActivity,
}

impl ConnectedClient {
    fn new(activity: DownloadHostActivity) -> Self {
        activity.clients.fetch_add(1, Ordering::AcqRel);
        activity.touch();
        Self { activity }
    }
}

impl Drop for ConnectedClient {
    fn drop(&mut self) {
        self.activity.clients.fetch_sub(1, Ordering::AcqRel);
        self.activity.touch();
    }
}

fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}

pub fn command_envelope(command: DownloadCommand) -> DownloadCommandEnvelope {
    DownloadCommandEnvelope {
        protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
        request_id: uuid::Uuid::new_v4(),
        origin: nxtools_download::RequestOrigin::Cli,
        command,
    }
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::path::PathBuf;
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };
    use std::time::Instant;

    use nxtools_download::{
        DownloadCommandOutcome, DownloadCommandResult, DownloadMediaProbe, DownloadProbe,
        DownloadQuery, DownloadRequest, DownloadStatus, DownloadSubmitMode, HlsProbe,
        HlsVariantProbe, MAX_HLS_ATTRIBUTE_BYTES, MAX_HLS_PROBE_SERIALIZED_BYTES,
        MAX_HLS_VARIANT_URL_BYTES, MAX_HLS_VARIANTS, ResourceValidator,
    };

    use super::*;

    #[test]
    fn maximum_hls_probe_metadata_fits_one_download_ipc_frame() {
        let variants = (0..MAX_HLS_VARIANTS)
            .map(|index| {
                let prefix = format!("https://example.test/{index}/");
                let playlist_url = format!(
                    "{prefix}{}",
                    "u".repeat(MAX_HLS_VARIANT_URL_BYTES - prefix.len())
                );
                assert_eq!(playlist_url.len(), MAX_HLS_VARIANT_URL_BYTES);
                HlsVariantProbe {
                    playlist_url,
                    bandwidth: u64::MAX,
                    average_bandwidth: Some(u64::MAX),
                    resolution: Some((u32::MAX, u32::MAX)),
                    frame_rate_milli: Some(u32::MAX),
                    codecs: "c".repeat(MAX_HLS_ATTRIBUTE_BYTES),
                    audio_group: Some("a".repeat(MAX_HLS_ATTRIBUTE_BYTES)),
                    subtitles_group: Some("s".repeat(MAX_HLS_ATTRIBUTE_BYTES)),
                }
            })
            .collect();
        let final_url_prefix = "https://example.test/";
        let final_url = format!(
            "{final_url_prefix}{}",
            "f".repeat(MAX_HLS_VARIANT_URL_BYTES - final_url_prefix.len())
        );
        assert_eq!(final_url.len(), MAX_HLS_VARIANT_URL_BYTES);
        let probe = DownloadProbe {
            resolution: None,
            final_url,
            file_name: Some("master.m3u8".into()),
            total_bytes: Some(2 * 1024 * 1024),
            supports_ranges: false,
            content_type: Some("application/vnd.apple.mpegurl".into()),
            validator: ResourceValidator::default(),
            media: Some(DownloadMediaProbe::Hls(HlsProbe {
                is_master: true,
                variants,
            })),
        };
        assert!(
            serde_json::to_vec(&probe)
                .expect("serialize bounded HLS probe")
                .len()
                <= MAX_HLS_PROBE_SERIALIZED_BYTES
        );
        let frame = DownloadIpcFrame::Result(DownloadCommandResult {
            request_id: uuid::Uuid::new_v4(),
            outcome: DownloadCommandOutcome::Probe(probe),
        });
        let mut encoded = Vec::new();

        write_frame(&mut encoded, &frame).expect("maximum bounded HLS probe fits IPC frame");

        assert!(encoded.len() <= nxtools_protocol::MAX_FRAME_BYTES + 1);
    }

    #[test]
    fn a_cli_client_uses_the_host_owned_engine_over_one_pipe() {
        let pipe_name = format!(r"\\.\pipe\nxtools-download-test-{}", uuid::Uuid::new_v4());
        let engine = DownloadEngine::open_in_memory().expect("open engine");
        let host_engine = engine.clone();
        let host_name = pipe_name.clone();
        std::thread::spawn(move || {
            let _ = run_download_host(&host_name, host_engine, DownloadHostActivity::default());
        });

        let mut client =
            DownloadIpcClient::connect(&pipe_name, Duration::from_secs(2)).expect("connect CLI");
        let request = DownloadRequest {
            source_url: "https://example.com/archive.zip".into(),
            save_directory: PathBuf::from(r"C:\Downloads"),
            save_directory_explicit: false,
            file_name: Some("archive.zip".into()),
            headers: Vec::new(),
            cookie: None,
            referrer: None,
            certificate_policy: Default::default(),
            connection_limit: None,
            speed_limit_bytes_per_second: None,
            expected_total_bytes: None,
            scheduled_at_unix_ms: None,
            adaptive_segment_policy: None,
            media: None,
            category_id: None,
            category_name: None,
            duplicate_strategy: nxtools_download::DownloadDuplicateStrategy::UseDefault,
        };
        let added = client
            .command(command_envelope(DownloadCommand::Submit {
                request,
                mode: DownloadSubmitMode::QueueOnly,
                idempotency_key: None,
            }))
            .expect("submit command");
        assert!(matches!(
            added.outcome,
            DownloadCommandOutcome::Accepted { task_id: Some(_) }
        ));

        let snapshot = client
            .command(command_envelope(DownloadCommand::Snapshot {
                query: DownloadQuery::default(),
            }))
            .expect("snapshot command");
        let DownloadCommandOutcome::Snapshot(snapshot) = snapshot.outcome else {
            panic!("expected snapshot");
        };
        assert_eq!(snapshot.tasks.len(), 1);
        assert_eq!(engine.tasks().expect("host engine tasks").len(), 1);
    }

    #[test]
    fn a_watch_connection_receives_events_created_by_another_cli_connection() {
        let pipe_name = format!(
            r"\\.\pipe\nxtools-download-watch-test-{}",
            uuid::Uuid::new_v4()
        );
        let engine = DownloadEngine::open_in_memory().expect("open engine");
        let host_name = pipe_name.clone();
        std::thread::spawn(move || {
            let _ = run_download_host(&host_name, engine, DownloadHostActivity::default());
        });

        let mut watcher = DownloadIpcClient::connect(&pipe_name, Duration::from_secs(2))
            .expect("connect watcher");
        let subscription = watcher
            .command(command_envelope(DownloadCommand::Subscribe {
                after_sequence: None,
                generation: None,
            }))
            .expect("subscribe");
        assert!(matches!(
            subscription.outcome,
            DownloadCommandOutcome::Accepted { task_id: None }
        ));

        let mut actor =
            DownloadIpcClient::connect(&pipe_name, Duration::from_secs(2)).expect("connect actor");
        let request = DownloadRequest {
            source_url: "https://example.com/watched.bin".into(),
            save_directory: PathBuf::from(r"C:\Downloads"),
            save_directory_explicit: false,
            file_name: Some("watched.bin".into()),
            headers: Vec::new(),
            cookie: None,
            referrer: None,
            certificate_policy: Default::default(),
            connection_limit: None,
            speed_limit_bytes_per_second: None,
            expected_total_bytes: None,
            scheduled_at_unix_ms: None,
            adaptive_segment_policy: None,
            media: None,
            category_id: None,
            category_name: None,
            duplicate_strategy: nxtools_download::DownloadDuplicateStrategy::UseDefault,
        };
        actor
            .command(command_envelope(DownloadCommand::Submit {
                request,
                mode: DownloadSubmitMode::QueueOnly,
                idempotency_key: None,
            }))
            .expect("submit watched task");
        let event = watcher
            .next_event()
            .expect("read watch event")
            .expect("watch event");
        assert!(matches!(
            event.event,
            nxtools_download::DownloadEvent::TaskChanged { .. }
        ));
    }

    #[test]
    fn cli_transport_drives_one_http_task_to_atomic_completion() {
        let body = b"phase-1b-cli-download".to_vec();
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind HTTP fixture");
        listener
            .set_nonblocking(true)
            .expect("make HTTP fixture interruptible");
        let address = listener.local_addr().expect("HTTP fixture address");
        let server_body = body.clone();
        let stop_server = Arc::new(AtomicBool::new(false));
        let server_stop = Arc::clone(&stop_server);
        let server = std::thread::spawn(move || {
            while !server_stop.load(Ordering::Acquire) {
                let (mut stream, _) = match listener.accept() {
                    Ok(connection) => connection,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(2));
                        continue;
                    }
                    Err(error) => panic!("accept HTTP request: {error}"),
                };
                stream
                    .set_nonblocking(false)
                    .expect("make accepted HTTP fixture stream blocking");
                let mut request = [0_u8; 2048];
                let read = stream.read(&mut request).expect("read HTTP request");
                let request = String::from_utf8_lossy(&request[..read]);
                if request.starts_with("HEAD ") {
                    write!(
                        stream,
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nAccept-Ranges: bytes\r\nConnection: close\r\n\r\n",
                        server_body.len()
                    )
                    .expect("write HEAD response");
                } else {
                    write!(
                        stream,
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        server_body.len()
                    )
                    .expect("write GET headers");
                    stream.write_all(&server_body).expect("write GET body");
                }
            }
        });

        let temp = tempfile::tempdir().expect("download destination");
        let pipe_name = format!(
            r"\\.\pipe\nxtools-download-e2e-test-{}",
            uuid::Uuid::new_v4()
        );
        let engine = DownloadEngine::open_in_memory().expect("open engine");
        let host_name = pipe_name.clone();
        std::thread::spawn(move || {
            let _ = run_download_host(&host_name, engine, DownloadHostActivity::default());
        });
        let mut client =
            DownloadIpcClient::connect(&pipe_name, Duration::from_secs(2)).expect("connect CLI");
        let host_snapshot = client
            .host_snapshot()
            .expect("host handshake includes a readiness snapshot");
        assert_eq!(host_snapshot.readiness, DownloadHostReadiness::Ready);
        assert_ne!(host_snapshot.engine_generation, 0);
        let submitted = client
            .command(command_envelope(DownloadCommand::Submit {
                request: DownloadRequest {
                    source_url: format!("http://{address}/phase-1b.bin"),
                    save_directory: temp.path().to_path_buf(),
                    save_directory_explicit: false,
                    file_name: Some("phase-1b.bin".into()),
                    headers: Vec::new(),
                    cookie: None,
                    referrer: None,
                    certificate_policy: Default::default(),
                    connection_limit: None,
                    speed_limit_bytes_per_second: None,
                    expected_total_bytes: None,
                    scheduled_at_unix_ms: None,
                    adaptive_segment_policy: None,
                    media: None,
                    category_id: None,
                    category_name: None,
                    duplicate_strategy: nxtools_download::DownloadDuplicateStrategy::UseDefault,
                },
                mode: DownloadSubmitMode::StartNow,
                idempotency_key: None,
            }))
            .expect("submit HTTP task");
        let DownloadCommandOutcome::Accepted {
            task_id: Some(task_id),
        } = submitted.outcome
        else {
            panic!("expected accepted HTTP task");
        };

        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let snapshot = client
                .command(command_envelope(DownloadCommand::Snapshot {
                    query: DownloadQuery::default(),
                }))
                .expect("poll HTTP task");
            let DownloadCommandOutcome::Snapshot(snapshot) = snapshot.outcome else {
                panic!("expected HTTP snapshot");
            };
            let task = snapshot
                .task_details
                .iter()
                .find(|task| task.task.id() == task_id)
                .expect("submitted task remains present");
            if task.task.status() == DownloadStatus::Completed {
                break;
            }
            assert_ne!(
                task.task.status(),
                DownloadStatus::Failed,
                "HTTP task failed: {:?}",
                task.error
            );
            assert!(Instant::now() < deadline, "HTTP task timed out");
            std::thread::sleep(Duration::from_millis(20));
        }

        stop_server.store(true, Ordering::Release);
        server.join().expect("join HTTP fixture");
        assert_eq!(
            std::fs::read(temp.path().join("phase-1b.bin")).expect("read final artifact"),
            body
        );
    }
    #[test]
    fn host_readiness_is_explicit_and_draining_blocks_mutations() {
        let activity = DownloadHostActivity::default();
        assert_eq!(activity.readiness(), DownloadHostReadiness::Starting);
        activity.set_ready();
        assert_eq!(activity.readiness(), DownloadHostReadiness::Ready);
        activity.begin_draining();
        assert_eq!(activity.readiness(), DownloadHostReadiness::Draining);
        assert!(is_mutating_command(&DownloadCommand::Submit {
            request: sample_ipc_request(),
            mode: nxtools_download::DownloadSubmitMode::QueueOnly,
            idempotency_key: None,
        }));
        assert!(is_mutating_command(&DownloadCommand::SubmitResolved {
            request: sample_ipc_request(),
            mode: nxtools_download::DownloadSubmitMode::QueueOnly,
            idempotency_key: None,
            resolution: nxtools_download::DownloadResolutionOptions::default(),
        }));
        assert!(is_mutating_command(
            &DownloadCommand::BrowserSubmitResolved {
                handoff_id: uuid::Uuid::new_v4(),
                request: sample_ipc_request(),
                mode: nxtools_download::DownloadSubmitMode::QueueOnly,
                resolution: nxtools_download::DownloadResolutionOptions::default(),
            }
        ));
        assert!(!is_mutating_command(&DownloadCommand::Snapshot {
            query: DownloadQuery::default(),
        }));
    }

    #[test]
    fn draining_host_rejects_mutation_but_serves_snapshot() {
        let pipe_name = format!(
            r"\\.\pipe\nxtools-download-draining-test-{}",
            uuid::Uuid::new_v4()
        );
        let engine = DownloadEngine::open_in_memory().expect("open draining engine");
        let activity = DownloadHostActivity::default();
        let host_activity = activity.clone();
        let host_name = pipe_name.clone();
        std::thread::spawn(move || {
            let _ = run_download_host(&host_name, engine, activity);
        });

        let mut client =
            DownloadIpcClient::connect(&pipe_name, Duration::from_secs(2)).expect("connect host");
        host_activity.begin_draining();
        let rejected = client
            .command(command_envelope(DownloadCommand::Submit {
                request: sample_ipc_request(),
                mode: nxtools_download::DownloadSubmitMode::QueueOnly,
                idempotency_key: None,
            }))
            .expect("receive draining rejection");
        assert!(matches!(
            rejected.outcome,
            DownloadCommandOutcome::Rejected { fault }
                if fault.code == DownloadFaultCode::Unavailable && fault.retryable
        ));

        let snapshot = client
            .command(command_envelope(DownloadCommand::Snapshot {
                query: DownloadQuery::default(),
            }))
            .expect("query remains available while draining");
        assert!(matches!(
            snapshot.outcome,
            DownloadCommandOutcome::Snapshot(_)
        ));
    }

    fn sample_ipc_request() -> DownloadRequest {
        DownloadRequest {
            source_url: "https://example.com/file.bin".into(),
            save_directory: PathBuf::from(r"C:\Downloads"),
            save_directory_explicit: false,
            file_name: Some("file.bin".into()),
            headers: Vec::new(),
            cookie: None,
            referrer: None,
            certificate_policy: Default::default(),
            connection_limit: None,
            speed_limit_bytes_per_second: None,
            expected_total_bytes: None,
            scheduled_at_unix_ms: None,
            adaptive_segment_policy: None,
            media: None,
            category_id: None,
            category_name: None,
            duplicate_strategy: nxtools_download::DownloadDuplicateStrategy::UseDefault,
        }
    }
}
