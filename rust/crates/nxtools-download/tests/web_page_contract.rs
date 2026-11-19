use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::Duration;

use nxtools_download::{DownloadEngine, DownloadRequest, DownloadStatus, RequestOrigin};

const PAGE: &[u8] =
    b"\r\n<!DOCTYPE html><html><title>Please sign in</title><body>Wait for your file</body></html>";

struct PageServer {
    address: SocketAddr,
    stopping: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl PageServer {
    fn start(body: Vec<u8>, mime: Option<&'static str>, ranges: bool, fragmented: bool) -> Self {
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("local page fixture");
        listener.set_nonblocking(true).expect("bounded listener");
        let address = listener.local_addr().expect("fixture address");
        let stopping = Arc::new(AtomicBool::new(false));
        let stop = stopping.clone();
        let body = Arc::new(body);
        let thread = std::thread::spawn(move || {
            let mut connections = Vec::new();
            while !stop.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((mut socket, _)) => {
                        let body = body.clone();
                        connections.push(std::thread::spawn(move || {
                            socket.set_nonblocking(false).expect("blocking accepted socket");
                            socket.set_read_timeout(Some(Duration::from_secs(2))).expect("read bound");
                            socket.set_write_timeout(Some(Duration::from_secs(2))).expect("write bound");
                            let mut request = Vec::new();
                            while request.len() < 8192 && !request.ends_with(b"\r\n\r\n") {
                                let mut byte = [0];
                                match socket.read(&mut byte) {
                                    Ok(1) => request.push(byte[0]),
                                    _ => return,
                                }
                            }
                            let request = String::from_utf8_lossy(&request).to_ascii_lowercase();
                            let head = request.starts_with("head ");
                            let requested = request.lines().find_map(|line| line.strip_prefix("range: bytes="));
                            let range = requested.filter(|_| ranges).map(|range| {
                                let (start, end) = range.split_once('-').expect("range pair");
                                let start: usize = start.parse().expect("range start");
                                let end = end.parse::<usize>().unwrap_or(body.len() - 1).min(body.len() - 1);
                                (start, end)
                            });
                            let (status, bytes, range_header) = if let Some((start, end)) = range {
                                ("206 Partial Content", &body[start..=end], format!("Content-Range: bytes {start}-{end}/{}\r\n", body.len()))
                            } else {
                                ("200 OK", body.as_slice(), String::new())
                            };
                            let mime_header = mime.map_or_else(String::new, |mime| format!("Content-Type: {mime}\r\n"));
                            if write!(socket, "HTTP/1.1 {status}\r\nConnection: close\r\nContent-Length: {}\r\nETag: \"fixture\"\r\n{mime_header}{range_header}\r\n", bytes.len()).is_err() || head {
                                return;
                            }
                            if fragmented && requested != Some("0-0") {
                                for part in bytes.chunks(3) {
                                    if socket.write_all(part).is_err() { break; }
                                    std::thread::sleep(Duration::from_millis(1));
                                }
                            } else {
                                let _ = socket.write_all(bytes);
                            }
                        }));
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(1))
                    }
                    Err(error) => panic!("fixture accept: {error}"),
                }
            }
            for connection in connections {
                connection.join().expect("fixture connection");
            }
        });
        Self {
            address,
            stopping,
            thread: Some(thread),
        }
    }
}

impl Drop for PageServer {
    fn drop(&mut self) {
        self.stopping.store(true, Ordering::SeqCst);
        if let Some(thread) = self.thread.take() {
            thread.join().expect("fixture shutdown");
        }
    }
}

async fn download_case(
    file_name: &str,
    body: Vec<u8>,
    mime: Option<&'static str>,
    ranges: bool,
    fragmented: bool,
    succeeds: bool,
) {
    let server = PageServer::start(body.clone(), mime, ranges, fragmented);
    let directory = tempfile::tempdir().expect("isolated download");
    let database = directory.path().join("downloads.sqlite3");
    let request: DownloadRequest = serde_json::from_value(serde_json::json!({
        "source_url": format!("http://{}/file", server.address),
        "save_directory": directory.path(), "file_name": file_name,
        "connection_limit": if ranges { 4 } else { 1 }
    }))
    .expect("download request");
    let engine = DownloadEngine::open(&database).expect("engine");
    let id = engine.submit(request, RequestOrigin::Ui).expect("submit");
    let terminal = engine
        .wait_for_terminal(id, Duration::from_secs(5))
        .await
        .expect("terminal");
    engine.shutdown_and_wait().expect("drain");
    let target = directory.path().join(file_name);
    if succeeds {
        assert_eq!(
            terminal.task.status(),
            DownloadStatus::Completed,
            "{:?}",
            terminal.error
        );
        assert_eq!(std::fs::read(target).expect("completed bytes"), body);
    } else {
        assert_eq!(
            terminal.task.status(),
            DownloadStatus::Failed,
            "a web page was published as {file_name}"
        );
        let fault = terminal.error.expect("actionable failure");
        assert_eq!(
            fault.code,
            nxtools_download::DownloadFaultCode::UnexpectedWebPage
        );
        assert!(
            !fault.retryable,
            "a login page must not cause automatic retries"
        );
        assert!(fault.message.contains("网页"), "{:?}", fault);
        assert!(!target.exists(), "a waiting page is not a final file");
        drop(engine);
        let reopened = DownloadEngine::open(&database).expect("reopen");
        let task = reopened.task(id).expect("query").expect("original task");
        assert_eq!(task.task.status(), DownloadStatus::Failed);
        assert_eq!(task.error, Some(fault));
        reopened.shutdown_and_wait().expect("reopened drain");
    }
}

#[tokio::test]
async fn binary_download_rejects_html_with_truthful_misleading_or_missing_type() {
    for mime in [
        Some("text/html; charset=utf-8"),
        Some("application/zip"),
        None,
    ] {
        download_case("package.zip", PAGE.to_vec(), mime, false, true, false).await;
    }
}

#[tokio::test]
async fn segmented_binary_download_never_publishes_a_web_page() {
    let mut body = PAGE.to_vec();
    body.resize(256 * 1024, b' ');
    download_case(
        "package.zip",
        body,
        Some("application/octet-stream"),
        true,
        false,
        false,
    )
    .await;
}

#[tokio::test]
async fn segmented_binary_prefix_is_written_exactly_once() {
    let mut body = b"PK\x03\x04".to_vec();
    body.extend((0..512 * 1024).map(|offset| (offset % 251) as u8));
    download_case(
        "package.zip",
        body,
        Some("application/zip"),
        true,
        false,
        true,
    )
    .await;
}

#[tokio::test]
async fn intentional_html_and_legacy_office_exports_keep_their_bytes() {
    for name in ["page.html", "report.xls", "document.doc"] {
        download_case(name, PAGE.to_vec(), Some("text/html"), false, true, true).await;
    }
}

#[tokio::test]
async fn short_binary_and_generic_text_are_not_mistaken_for_a_web_page() {
    download_case(
        "package.zip",
        b"PK\x03\x04\0\x01\x02".to_vec(),
        None,
        false,
        true,
        true,
    )
    .await;
    download_case("data.bin", PAGE.to_vec(), None, false, true, true).await;
}

#[tokio::test]
async fn resumed_range_rejects_a_full_waiting_page_without_changing_saved_bytes() {
    use nxtools_download::{
        DownloadProbe, DownloadSegment, DownloadSegmentState, DownloadSegmentTransferControl,
        DownloadSegmentTransferRequest, DownloadTaskId, DownloadTransferContext,
        DownloadTransferControl, HttpDownloadClient, HttpDownloadError, ResourceValidator,
        SegmentedArtifact,
    };
    for mime in [Some("text/html"), Some("application/octet-stream"), None] {
        let server = PageServer::start(PAGE.to_vec(), mime, false, true);
        let directory = tempfile::tempdir().expect("resume root");
        let request: DownloadRequest = serde_json::from_value(serde_json::json!({
            "source_url": format!("http://{}/file", server.address),
            "save_directory": directory.path(), "file_name": "package.zip"
        }))
        .expect("resume request");
        let total = 4096;
        let probe = DownloadProbe {
            final_url: request.source_url.clone(),
            file_name: Some("package.zip".into()),
            total_bytes: Some(total),
            supports_ranges: true,
            content_type: Some("application/zip".into()),
            validator: ResourceValidator {
                etag: Some("\"fixture\"".into()),
                last_modified: None,
            },
            media: None,
            resolution: None,
            server_observation: None,
        };
        let path = directory.path().join("saved.part");
        let artifact = SegmentedArtifact::create(&path, total)
            .await
            .expect("artifact");
        let fresh =
            DownloadSegment::new(0, 0, total - 1, 0, DownloadSegmentState::Active).expect("range");
        let mut writer = artifact.open_segment(&fresh).await.expect("writer");
        writer.write_all(&[0xa5; 512]).await.expect("saved prefix");
        writer.sync().await.expect("durable prefix");
        drop(writer);
        let before = std::fs::read(&path).expect("saved bytes");
        let segment = DownloadSegment::new(0, 0, total - 1, 512, DownloadSegmentState::Active)
            .expect("resumed range");
        let error = HttpDownloadClient::new()
            .expect("client")
            .download_segment(
                DownloadSegmentTransferRequest::new(
                    DownloadTransferContext::new(DownloadTaskId::new(), uuid::Uuid::new_v4()),
                    &request,
                    &probe,
                    &segment,
                    &artifact,
                    &DownloadSegmentTransferControl::new(DownloadTransferControl::new()),
                )
                .with_resume_validator(),
                |_| panic!("waiting page must never become progress"),
            )
            .await
            .expect_err("full HTML is not a resumable response");
        assert!(
            matches!(error, HttpDownloadError::UnexpectedWebPage),
            "{error:?}"
        );
        assert_eq!(std::fs::read(&path).expect("retained bytes"), before);
        assert!(!directory.path().join("package.zip").exists());
    }
}
