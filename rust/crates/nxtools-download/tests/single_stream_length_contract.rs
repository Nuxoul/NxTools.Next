use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::Duration;

use nxtools_download::{
    DOWNLOAD_COMMAND_PROTOCOL_VERSION, DownloadCommand, DownloadCommandEnvelope,
    DownloadCommandOutcome, DownloadEngine, DownloadQuery, DownloadRequest, DownloadStatus,
    DownloadTaskId, DownloadTransferControl, DownloadTransferOutcome, HttpDownloadClient,
    RequestOrigin,
};

fn assert_completed_size(engine: &DownloadEngine, task_id: DownloadTaskId) {
    let task = engine
        .task(task_id)
        .expect("task query")
        .expect("same task");
    assert_eq!(task.task.status(), DownloadStatus::Completed);
    assert_eq!(task.downloaded_bytes, GZIP_BODY.len() as u64);
    assert_eq!(task.total_bytes, Some(GZIP_BODY.len() as u64));
    assert_eq!(
        task.probe.expect("original probe").total_bytes,
        Some(GZIP_BODY.len() as u64 + 1)
    );
    for compact in [false, true] {
        let result = engine.handle_command(DownloadCommandEnvelope {
            protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
            request_id: uuid::Uuid::new_v4(),
            origin: RequestOrigin::Cli,
            command: DownloadCommand::Snapshot {
                query: DownloadQuery {
                    compact,
                    ..Default::default()
                },
            },
        });
        let DownloadCommandOutcome::Snapshot(snapshot) = result.outcome else {
            panic!("snapshot: {result:?}");
        };
        let total = if compact {
            snapshot.task_summaries[0].total_bytes
        } else {
            snapshot.task_details[0].total_bytes
        };
        assert_eq!(total, Some(GZIP_BODY.len() as u64));
    }
}

#[tokio::test]
async fn single_stream_engine_persists_actual_completed_size_without_rewriting_probe() {
    let server = DynamicServer::start(GZIP_BODY, true, Framing::Length);
    let directory = tempfile::tempdir().expect("engine root");
    let database = directory.path().join("downloads.sqlite3");
    let request: DownloadRequest = serde_json::from_value(serde_json::json!({
        "source_url": format!("http://{}/dynamic", server.address),
        "save_directory": directory.path(),
        "file_name": "engine-response.bin",
        "connection_limit": 1
    }))
    .expect("engine request");
    let engine = DownloadEngine::open(&database).expect("engine");
    let id = engine.submit(request, RequestOrigin::Ui).expect("submit");
    let terminal = engine
        .wait_for_terminal(id, Duration::from_secs(3))
        .await
        .expect("terminal");
    assert_eq!(
        terminal.task.status(),
        DownloadStatus::Completed,
        "{:?}",
        terminal.error
    );
    engine.shutdown_and_wait().expect("drain");
    assert_eq!(
        std::fs::read(directory.path().join("engine-response.bin")).expect("actual output"),
        GZIP_BODY
    );
    assert_completed_size(&engine, id);
    drop(engine);
    let restored = DownloadEngine::open(&database).expect("reopen");
    assert_completed_size(&restored, id);
    restored.shutdown_and_wait().expect("restored drain");
}

const GZIP_BODY: &[u8] = &[
    31, 139, 8, 0, 0, 0, 0, 0, 2, 255, 75, 175, 202, 44, 40, 72, 77, 81, 40, 74, 45, 46, 200, 207,
    43, 78, 229, 2, 0, 80, 241, 180, 52, 17, 0, 0, 0,
];

#[derive(Clone, Copy)]
enum Framing {
    Length,
    Chunked,
    TruncatedLength,
    TruncatedChunked,
}

struct DynamicServer {
    address: SocketAddr,
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

impl DynamicServer {
    fn start(body: &'static [u8], gzip: bool, framing: Framing) -> Self {
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind dynamic response fixture");
        listener.set_nonblocking(true).expect("bounded listener");
        let address = listener.local_addr().expect("fixture address");
        let stop = Arc::new(AtomicBool::new(false));
        let stopping = stop.clone();
        let worker = std::thread::spawn(move || {
            while !stopping.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        stream
                            .set_nonblocking(false)
                            .expect("blocking accepted socket");
                        stream
                            .set_read_timeout(Some(Duration::from_secs(2)))
                            .expect("read timeout");
                        stream
                            .set_write_timeout(Some(Duration::from_secs(2)))
                            .expect("write timeout");
                        let mut request = Vec::new();
                        while request.len() < 8192 && !request.ends_with(b"\r\n\r\n") {
                            let mut byte = [0];
                            if stream.read(&mut byte).unwrap_or(0) == 0 {
                                break;
                            }
                            request.push(byte[0]);
                        }
                        if !request.ends_with(b"\r\n\r\n") {
                            continue;
                        }
                        let request = String::from_utf8_lossy(&request);
                        let head = request.starts_with("HEAD ");
                        let probe =
                            head || request.to_ascii_lowercase().contains("range: bytes=0-0");
                        let encoding = if gzip {
                            "Content-Encoding: gzip\r\n"
                        } else {
                            ""
                        };
                        let headers = if probe {
                            // Both probe methods return a complete, earlier representation.
                            format!("Content-Length: {}\r\n", body.len() + 1)
                        } else if matches!(framing, Framing::Chunked | Framing::TruncatedChunked) {
                            "Transfer-Encoding: chunked\r\n".to_owned()
                        } else {
                            let extra = usize::from(matches!(framing, Framing::TruncatedLength));
                            format!("Content-Length: {}\r\n", body.len() + extra)
                        };
                        let _ = write!(
                            stream,
                            "HTTP/1.1 200 OK\r\nConnection: close\r\nContent-Type: application/octet-stream\r\n{encoding}{headers}\r\n"
                        );
                        if !head {
                            if probe {
                                let _ = stream.write_all(body);
                                let _ = stream.write_all(b" ");
                            } else if matches!(
                                framing,
                                Framing::Chunked | Framing::TruncatedChunked
                            ) {
                                let _ = write!(stream, "{:x}\r\n", body.len());
                                let _ = stream.write_all(body);
                                let _ = stream.write_all(b"\r\n");
                                if matches!(framing, Framing::Chunked) {
                                    let _ = stream.write_all(b"0\r\n\r\n");
                                }
                            } else {
                                let _ = stream.write_all(body);
                            }
                        }
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(1));
                    }
                    Err(error) => panic!("accept fixture connection: {error}"),
                }
            }
        });
        Self {
            address,
            stop,
            worker: Some(worker),
        }
    }
}

impl Drop for DynamicServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        let _ = TcpStream::connect(self.address);
        if let Some(worker) = self.worker.take() {
            worker.join().expect("join fixture");
        }
    }
}

async fn transfer(body: &'static [u8], gzip: bool, framing: Framing, succeeds: bool) {
    let server = DynamicServer::start(body, gzip, framing);
    let directory = tempfile::tempdir().expect("destination");
    let request: DownloadRequest = serde_json::from_value(serde_json::json!({
        "source_url": format!("http://{}/dynamic", server.address),
        "save_directory": directory.path(),
        "file_name": "response.bin",
        "connection_limit": 1
    }))
    .expect("request");
    let client = HttpDownloadClient::new().expect("HTTP client");
    let probe = client.probe(&request).await.expect("dynamic probe");
    assert_eq!(probe.total_bytes, Some(body.len() as u64 + 1));
    assert!(!probe.supports_ranges);
    let mut totals = Vec::new();
    let result = client
        .download(
            &request,
            &probe,
            DownloadTaskId::new(),
            &DownloadTransferControl::new(),
            |progress| totals.push(progress.total_bytes),
        )
        .await;
    let final_path = directory.path().join("response.bin");
    if succeeds {
        let outcome =
            result.expect("complete current response must not be compared with probe length");
        assert!(
            matches!(outcome, DownloadTransferOutcome::Completed { downloaded_bytes, .. } if downloaded_bytes == body.len() as u64)
        );
        assert_eq!(std::fs::read(&final_path).expect("completed output"), body);
        assert!(!totals.is_empty());
        let expected = if matches!(framing, Framing::Chunked) {
            None
        } else {
            Some(body.len() as u64)
        };
        assert!(
            totals.iter().all(|total| *total == expected),
            "current response progress: {totals:?}"
        );
        assert_eq!(
            std::fs::read_dir(directory.path())
                .expect("artifacts")
                .count(),
            1
        );
    } else {
        assert!(result.is_err(), "truncated response must fail");
        assert!(
            !final_path.exists(),
            "truncated output must not be published"
        );
    }
}

#[tokio::test]
async fn single_stream_gzip_uses_current_response_length_and_preserves_encoded_bytes() {
    transfer(GZIP_BODY, true, Framing::Length, true).await;
}

#[tokio::test]
async fn single_stream_plain_dynamic_response_uses_current_length() {
    transfer(b"current response", false, Framing::Length, true).await;
}

#[tokio::test]
async fn single_stream_chunked_response_does_not_inherit_probe_length() {
    transfer(GZIP_BODY, true, Framing::Chunked, true).await;
}

#[tokio::test]
async fn single_stream_declared_length_truncation_never_publishes() {
    transfer(GZIP_BODY, true, Framing::TruncatedLength, false).await;
}

#[tokio::test]
async fn single_stream_incomplete_chunked_body_never_publishes() {
    transfer(GZIP_BODY, true, Framing::TruncatedChunked, false).await;
}
