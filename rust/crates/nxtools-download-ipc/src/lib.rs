use std::io::{BufRead, Write};
use std::sync::{
    Arc,
    atomic::{AtomicU64, AtomicUsize, Ordering},
};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};
use nxtools_download::{
    DOWNLOAD_COMMAND_PROTOCOL_VERSION, DownloadCommand, DownloadCommandEnvelope,
    DownloadCommandResult, DownloadEngine, DownloadEventEnvelope,
};
use nxtools_ipc::{
    LocalPipeConnection, accept_current_user_pipe, connect_local_pipe, read_bounded_sync_frame,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const DOWNLOAD_PIPE_NAME: &str = r"\\.\pipe\nxtools-next-download-v1";
pub const DOWNLOAD_IPC_PROTOCOL_VERSION: u16 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "frame", content = "data", rename_all = "snake_case")]
pub enum DownloadIpcFrame {
    Hello {
        protocol_version: u16,
        client_version: String,
    },
    HelloAck {
        protocol_version: u16,
        server_version: String,
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

#[derive(Clone, Default)]
pub struct DownloadHostActivity {
    clients: Arc<AtomicUsize>,
    last_activity_unix_ms: Arc<AtomicU64>,
}

impl DownloadHostActivity {
    pub fn connected_clients(&self) -> usize {
        self.clients.load(Ordering::Acquire)
    }

    pub fn last_activity_unix_ms(&self) -> u64 {
        self.last_activity_unix_ms.load(Ordering::Acquire)
    }

    fn touch(&self) {
        self.last_activity_unix_ms
            .store(now_unix_ms(), Ordering::Release);
    }
}

pub struct DownloadIpcClient {
    connection: LocalPipeConnection,
}

impl DownloadIpcClient {
    pub fn connect(pipe_name: &str, timeout: Duration) -> Result<Self> {
        let mut connection = connect_local_pipe(pipe_name, timeout)?;
        write_frame(
            &mut connection.writer,
            &DownloadIpcFrame::Hello {
                protocol_version: DOWNLOAD_IPC_PROTOCOL_VERSION,
                client_version: env!("CARGO_PKG_VERSION").into(),
            },
        )?;
        match read_frame(&mut connection.reader)? {
            Some(DownloadIpcFrame::HelloAck {
                protocol_version: DOWNLOAD_IPC_PROTOCOL_VERSION,
                ..
            }) => Ok(Self { connection }),
            Some(DownloadIpcFrame::Error {
                code: DownloadIpcErrorCode::ProtocolIncompatible,
                message,
            }) => Err(DownloadIpcProtocolIncompatible { message }.into()),
            Some(frame) => bail!("download host returned an invalid handshake frame: {frame:?}"),
            None => bail!("download host disconnected during handshake"),
        }
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
    loop {
        let connection = accept_current_user_pipe(pipe_name)
            .with_context(|| format!("failed to accept download client on {pipe_name}"))?;
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
            file_name: Some("archive.zip".into()),
            headers: Vec::new(),
            cookie: None,
            referrer: None,
            connection_limit: None,
            speed_limit_bytes_per_second: None,
            expected_total_bytes: None,
            scheduled_at_unix_ms: None,
            adaptive_segment_policy: None,
            media: None,
        };
        let added = client
            .command(command_envelope(DownloadCommand::Submit {
                request,
                mode: DownloadSubmitMode::QueueOnly,
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
            file_name: Some("watched.bin".into()),
            headers: Vec::new(),
            cookie: None,
            referrer: None,
            connection_limit: None,
            speed_limit_bytes_per_second: None,
            expected_total_bytes: None,
            scheduled_at_unix_ms: None,
            adaptive_segment_policy: None,
            media: None,
        };
        actor
            .command(command_envelope(DownloadCommand::Submit {
                request,
                mode: DownloadSubmitMode::QueueOnly,
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
        let submitted = client
            .command(command_envelope(DownloadCommand::Submit {
                request: DownloadRequest {
                    source_url: format!("http://{address}/phase-1b.bin"),
                    save_directory: temp.path().to_path_buf(),
                    file_name: Some("phase-1b.bin".into()),
                    headers: Vec::new(),
                    cookie: None,
                    referrer: None,
                    connection_limit: None,
                    speed_limit_bytes_per_second: None,
                    expected_total_bytes: None,
                    scheduled_at_unix_ms: None,
                    adaptive_segment_policy: None,
                    media: None,
                },
                mode: DownloadSubmitMode::StartNow,
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
}
