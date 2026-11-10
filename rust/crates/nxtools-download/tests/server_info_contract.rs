mod support;

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use nxtools_core::{NetworkProxyMode, NetworkProxySettings, NetworkSettings};
use nxtools_download::{
    DOWNLOAD_COMMAND_PROTOCOL_VERSION, DownloadCommand, DownloadCommandEnvelope,
    DownloadCommandOutcome, DownloadEngine, DownloadProbe, DownloadQuery, DownloadRequest,
    DownloadRouteKind, DownloadStatus, HttpDownloadClient, NetworkPolicy, RequestOrigin,
    SqliteDownloadStore,
};
use support::DeterministicHttpServer;

fn request(url: String) -> DownloadRequest {
    serde_json::from_value(serde_json::json!({
        "source_url": url, "save_directory": "D:\\Downloads", "connection_limit": 1
    }))
    .expect("synthetic request")
}

#[tokio::test]
async fn server_info_direct_probe_records_selected_response_without_extra_requests() {
    let server = DeterministicHttpServer::start_concurrent();
    let probe = HttpDownloadClient::new()
        .expect("HTTP client")
        .probe(&request(format!("http://{}/range", server.address())))
        .await
        .expect("actual HTTP probe");
    let json = serde_json::to_value(&probe).expect("serialized actual probe");
    assert!(
        json.get("server_observation").is_some(),
        "selected HTTP response must record server_observation"
    );
    let observed = probe.server_observation.expect("observed response");
    assert_eq!(observed.host, "127.0.0.1");
    assert_eq!(observed.peer_ip, Some(server.address().ip()));
    assert_eq!(observed.route_kind, DownloadRouteKind::Direct);
    assert_eq!(server.total_requests(), 2, "only original HEAD and Range");
}

struct Reply {
    delay: Duration,
    text: String,
}
fn reply(status: &str, headers: &str, body: &str, ms: u64) -> Reply {
    Reply {
        delay: Duration::from_millis(ms),
        text: format!("HTTP/1.1 {status}\r\nConnection: close\r\n{headers}\r\n{body}"),
    }
}
fn ordinary(ms: u64) -> Reply {
    reply("200 OK", "Content-Length: 16\r\n", "0123456789abcdef", ms)
}
fn ranged(ms: u64) -> Reply {
    reply(
        "206 Partial Content",
        "Content-Length: 1\r\nContent-Range: bytes 0-0/16\r\n",
        "0",
        ms,
    )
}
fn is_head(raw: &str) -> bool {
    raw.starts_with("HEAD ")
}
fn is_range(raw: &str) -> bool {
    raw.to_ascii_lowercase().contains("range: bytes=0-0")
}

/// Bounded local HTTP/proxy fixture; each accepted connection keeps blocking IO semantics.
struct Server {
    address: SocketAddr,
    requests: Arc<Mutex<Vec<String>>>,
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}
impl Server {
    fn start(handler: impl Fn(&str, SocketAddr) -> Reply + Send + Sync + 'static) -> Self {
        Self::start_at("127.0.0.1", handler)
    }
    fn start_at(
        ip: &str,
        handler: impl Fn(&str, SocketAddr) -> Reply + Send + Sync + 'static,
    ) -> Self {
        let listener = TcpListener::bind((ip, 0)).expect("local fixture");
        listener.set_nonblocking(true).expect("bounded listener");
        let address = listener.local_addr().expect("fixture address");
        let requests = Arc::new(Mutex::new(Vec::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let recorded = requests.clone();
        let stopping = stop.clone();
        let handler = Arc::new(handler);
        let worker = std::thread::spawn(move || {
            let mut connections = Vec::new();
            while !stopping.load(Ordering::Acquire) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        let recorded = recorded.clone();
                        let handler = handler.clone();
                        connections.push(std::thread::spawn(move || {
                            stream
                                .set_nonblocking(false)
                                .expect("blocking accepted socket");
                            stream
                                .set_read_timeout(Some(Duration::from_secs(2)))
                                .expect("read bound");
                            stream
                                .set_write_timeout(Some(Duration::from_secs(2)))
                                .expect("write bound");
                            let mut raw = Vec::new();
                            let mut byte = [0];
                            while raw.len() < 16 * 1024 && !raw.ends_with(b"\r\n\r\n") {
                                if stream.read(&mut byte).expect("fixture request") == 0 {
                                    return;
                                }
                                raw.push(byte[0]);
                            }
                            let raw = String::from_utf8(raw).expect("ASCII fixture request");
                            recorded.lock().expect("record requests").push(raw.clone());
                            let response = handler(&raw, address);
                            std::thread::sleep(response.delay);
                            // An intentionally dropped companion can close before its headers arrive.
                            let _ = stream.write_all(response.text.as_bytes());
                        }));
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(2))
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
            requests,
            stop,
            worker: Some(worker),
        }
    }
    fn url(&self, path: &str) -> String {
        format!("http://{}{path}", self.address)
    }
    fn count(&self) -> usize {
        self.requests.lock().expect("requests").len()
    }
}
impl Drop for Server {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        self.worker
            .take()
            .expect("fixture worker")
            .join()
            .expect("fixture shutdown");
    }
}

#[tokio::test]
async fn server_info_redirect_selected_range_pairs_host_peer_and_headers_time() {
    let destination = Server::start_at("127.0.0.2", |raw, _| {
        assert!(!is_head(raw));
        ranged(100)
    });
    let target = destination.url("/selected");
    let origin = Server::start(move |raw, _| {
        if is_head(raw) {
            ordinary(240)
        } else {
            reply(
                "302 Found",
                &format!("Location: {target}\r\nContent-Length: 0\r\n"),
                "",
                0,
            )
        }
    });
    let probe = HttpDownloadClient::new()
        .expect("client")
        .probe(&request(origin.url("/original?private=value")))
        .await
        .expect("redirect probe");
    let observed = probe.server_observation.expect("selected range");
    assert_eq!(observed.host, "127.0.0.2");
    assert_eq!(observed.peer_ip, Some(destination.address.ip()));
    assert!(
        observed.response_ms >= 90 && observed.response_ms < 230,
        "selected Range time, not companion: {}",
        observed.response_ms
    );
    assert_eq!((origin.count(), destination.count()), (2, 1));
}

#[tokio::test]
async fn server_info_dropped_companion_cannot_replace_selected_endpoint_or_time() {
    for slow_head in [true, false] {
        let server = Server::start(move |raw, _| {
            if is_head(raw) == slow_head {
                if is_head(raw) {
                    ordinary(1000)
                } else {
                    ranged(1000)
                }
            } else if is_head(raw) {
                ordinary(30)
            } else {
                ranged(30)
            }
        });
        let started = Instant::now();
        let probe = HttpDownloadClient::new()
            .expect("client")
            .probe(&request(server.url("/bounded")))
            .await
            .expect("bounded response");
        assert!(started.elapsed() < Duration::from_millis(800));
        let observed = probe
            .server_observation
            .as_ref()
            .expect("retained selected response")
            .clone();
        assert!(observed.response_ms >= 20 && observed.response_ms < 300);
        assert_eq!(
            probe.supports_ranges, slow_head,
            "dropped Range must not claim resume"
        );
        tokio::time::sleep(Duration::from_millis(750)).await;
        assert_eq!(probe.server_observation.as_ref(), Some(&observed));
        assert_eq!(server.count(), 2, "no late or additional request");
    }
}

#[tokio::test]
async fn server_info_head_failure_selection_and_plain_get_fallback_are_original_requests() {
    for fallback in [false, true] {
        let server = Server::start(move |raw, _| {
            if is_head(raw) && !fallback {
                ordinary(70)
            } else if is_head(raw) || is_range(raw) {
                reply("405 Method Not Allowed", "Content-Length: 0\r\n", "", 10)
            } else {
                ordinary(90)
            }
        });
        let probe = HttpDownloadClient::new()
            .expect("client")
            .probe(&request(server.url("/fallback")))
            .await
            .expect("selected success");
        let observed = probe.server_observation.expect("selected headers");
        assert!(observed.response_ms >= if fallback { 80 } else { 60 });
        assert_eq!(observed.host, "127.0.0.1");
        assert_eq!(server.count(), if fallback { 3 } else { 2 });
        assert!(!probe.supports_ranges);
    }
}

#[tokio::test]
async fn server_info_manual_proxy_uses_immutable_route_and_proxy_peer_not_origin_ip() {
    let proxy = Server::start(|raw, _| {
        assert!(
            raw.starts_with("HEAD http://origin.invalid/")
                || raw.starts_with("GET http://origin.invalid/")
        );
        if is_head(raw) {
            ordinary(0)
        } else {
            ranged(30)
        }
    });
    let policy = NetworkPolicy::new(NetworkSettings {
        proxy: NetworkProxySettings {
            mode: NetworkProxyMode::ManualHttp,
            host: "127.0.0.1".into(),
            port: proxy.address.port(),
        },
    })
    .expect("proxy policy");
    let client = HttpDownloadClient::with_stall_timeout_and_network(
        Duration::from_secs(2),
        &policy.snapshot().expect("frozen route"),
    )
    .expect("routed client");
    policy
        .update(NetworkSettings::default())
        .expect("change future operations only");
    let probe = client
        .probe(&request(
            "http://origin.invalid/private/file?token=synthetic".into(),
        ))
        .await
        .expect("actual proxy request");
    let observed = probe.server_observation.expect("proxy observation");
    assert_eq!(observed.host, "origin.invalid");
    assert_eq!(observed.peer_ip, Some(proxy.address.ip()));
    assert_eq!(observed.route_kind, DownloadRouteKind::ManualProxy);
    assert_eq!(proxy.count(), 2);
}

#[tokio::test]
async fn server_info_system_mode_is_policy_not_claimed_proxy_use() {
    let server = Server::start(|raw, _| if is_head(raw) { ordinary(0) } else { ranged(0) });
    let policy = NetworkPolicy::new(NetworkSettings {
        proxy: NetworkProxySettings {
            mode: NetworkProxyMode::System,
            ..Default::default()
        },
    })
    .expect("system policy");
    let client = HttpDownloadClient::with_stall_timeout_and_network(
        Duration::from_secs(2),
        &policy.snapshot().expect("system snapshot"),
    )
    .expect("client");
    let probe = client
        .probe(&request(server.url("/policy")))
        .await
        .expect("local response");
    let observed = probe.server_observation.expect("observation");
    assert_eq!(observed.route_kind, DownloadRouteKind::SystemPolicy);
    assert_eq!(observed.peer_ip, Some(server.address.ip()));
    assert_eq!(server.count(), 2);
}

#[tokio::test]
async fn server_info_media_manifest_redirect_observes_actual_manifest_headers_not_earlier_probe() {
    for (extension, content_type, body) in [
        (
            "m3u8",
            "application/vnd.apple.mpegurl",
            "#EXTM3U\n#EXTINF:1.0,\nsegment.ts\n#EXT-X-ENDLIST\n",
        ),
        (
            "mpd",
            "application/dash+xml",
            "<MPD type=\"static\" mediaPresentationDuration=\"PT2S\"><Period><AdaptationSet contentType=\"video\" mimeType=\"video/mp4\"><SegmentTemplate timescale=\"1000\" initialization=\"init.mp4\" media=\"$Time$.m4s\"><SegmentTimeline><S t=\"0\" d=\"2000\"/></SegmentTimeline></SegmentTemplate><Representation id=\"v\" bandwidth=\"100\"/></AdaptationSet></Period></MPD>",
        ),
    ] {
        let manifest = Server::start_at("127.0.0.2", move |_, _| {
            reply(
                "200 OK",
                &format!(
                    "Content-Type: {content_type}\r\nContent-Length: {}\r\n",
                    body.len()
                ),
                body,
                90,
            )
        });
        let target = manifest.url(&format!("/final.{extension}"));
        let origin = Server::start(move |raw, _| {
            if is_head(raw) || is_range(raw) {
                reply(
                    "200 OK",
                    &format!("Content-Type: {content_type}\r\nContent-Length: 16\r\n"),
                    "0123456789abcdef",
                    0,
                )
            } else {
                reply(
                    "302 Found",
                    &format!("Location: {target}\r\nContent-Length: 0\r\n"),
                    "",
                    0,
                )
            }
        });
        let probe = HttpDownloadClient::new()
            .expect("client")
            .probe(&request(origin.url(&format!("/initial.{extension}"))))
            .await
            .expect("manifest probe");
        assert!(probe.media.is_some());
        assert!(probe.final_url.contains("127.0.0.2"));
        let observed = probe
            .server_observation
            .expect("actual manifest observation");
        assert_eq!(observed.host, "127.0.0.2");
        assert_eq!(observed.peer_ip, Some(manifest.address.ip()));
        assert!(observed.response_ms >= 80);
        assert_eq!(
            (origin.count(), manifest.count()),
            (3, 1),
            "original two probes and redirected manifest only"
        );
    }
}

fn command(engine: &DownloadEngine, command: DownloadCommand) -> DownloadCommandOutcome {
    engine
        .handle_command(DownloadCommandEnvelope {
            protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
            request_id: uuid::Uuid::new_v4(),
            origin: RequestOrigin::Internal,
            command,
        })
        .outcome
}

#[tokio::test]
async fn server_info_sqlite_reopen_full_selected_and_compact_projection_preserve_original_probe() {
    let server = DeterministicHttpServer::start_concurrent();
    let directory = tempfile::tempdir().expect("SQLite fixture");
    let database = directory.path().join("downloads.sqlite3");
    let engine = DownloadEngine::open(&database).expect("engine");
    let mut request = request(format!("http://{}/range", server.address()));
    request.save_directory = directory.path().join("output");
    request.file_name = Some("server-info.bin".into());
    let task_id = engine
        .submit(request, RequestOrigin::Internal)
        .expect("submit real transfer");
    let completed = engine
        .wait_for_terminal(task_id, Duration::from_secs(5))
        .await
        .expect("terminal");
    assert_eq!(
        completed.task.status(),
        DownloadStatus::Completed,
        "{:?}",
        completed.error
    );
    let original_probe = completed.probe.expect("original probe");
    assert!(original_probe.server_observation.is_some());
    assert_eq!(
        server.total_requests(),
        3,
        "HEAD, Range and original transfer only"
    );
    drop(engine);
    let engine = DownloadEngine::open(&database).expect("real reopen");
    assert_eq!(
        engine
            .task_probe(task_id)
            .expect("memory-only original probe")
            .as_ref(),
        Some(&original_probe)
    );
    assert!(
        engine
            .task_probe(nxtools_download::DownloadTaskId::new())
            .expect("missing task probe")
            .is_none()
    );
    assert_eq!(
        engine
            .task(task_id)
            .expect("read task")
            .expect("original ID")
            .probe
            .as_ref(),
        Some(&original_probe)
    );
    let DownloadCommandOutcome::Snapshot(full) = command(
        &engine,
        DownloadCommand::Snapshot {
            query: DownloadQuery::default(),
        },
    ) else {
        panic!("full projection");
    };
    assert_eq!(full.task_details[0].probe.as_ref(), Some(&original_probe));
    let DownloadCommandOutcome::TaskDetail(selected) =
        command(&engine, DownloadCommand::TaskDetail { task_id })
    else {
        panic!("selected projection");
    };
    assert_eq!(selected.task.probe.as_ref(), Some(&original_probe));
    let DownloadCommandOutcome::Snapshot(compact) = command(
        &engine,
        DownloadCommand::Snapshot {
            query: DownloadQuery::compact(),
        },
    ) else {
        panic!("compact projection");
    };
    assert!(compact.task_details.is_empty());
    assert!(
        !serde_json::to_string(&compact.task_summaries)
            .expect("compact wire")
            .contains("server_observation")
    );
    // A task-only save must retain the original response stored in the same JSON envelope.
    let store = SqliteDownloadStore::open(&database).expect("same store");
    store
        .save_task(&selected.task.task)
        .expect("task-only save");
    drop(store);
    drop(engine);
    let engine = DownloadEngine::open(&database).expect("reopen after task save");
    assert_eq!(
        engine
            .task(task_id)
            .expect("task")
            .expect("same ID")
            .probe
            .as_ref(),
        Some(&original_probe)
    );
    drop(engine);
    let connection = rusqlite::Connection::open(&database).expect("legacy fixture connection");
    connection.execute("UPDATE download_tasks SET request_json = json_remove(request_json, '$.probe') WHERE task_id = ?1", [task_id.as_uuid().to_string()]).expect("legacy envelope without observation");
    let before: String = connection
        .query_row("SELECT request_json FROM download_tasks", [], |row| {
            row.get(0)
        })
        .expect("legacy bytes");
    let engine = DownloadEngine::open(&database).expect("legacy reopen");
    let legacy = engine.task(task_id).expect("legacy task").expect("same ID");
    assert!(
        legacy
            .probe
            .expect("legacy scalar probe")
            .server_observation
            .is_none()
    );
    let after: String = connection
        .query_row("SELECT request_json FROM download_tasks", [], |row| {
            row.get(0)
        })
        .expect("unchanged legacy bytes");
    assert_eq!(before, after, "reading legacy data must not rewrite it");
}

#[test]
fn server_info_legacy_wire_defaults_absent_and_peer_is_optional() {
    let probe: DownloadProbe = serde_json::from_value(serde_json::json!({
        "final_url": "https://synthetic.invalid/old", "file_name": null, "total_bytes": null,
        "supports_ranges": false, "content_type": null, "validator": {"etag": null, "last_modified": null}
    })).expect("old serialized probe");
    assert!(probe.server_observation.is_none());
    assert!(
        serde_json::to_value(probe)
            .expect("legacy wire")
            .get("server_observation")
            .is_none()
    );
}

#[tokio::test]
async fn server_info_media_runners_persist_their_actual_redirected_manifest_response() {
    for hls in [true, false] {
        let media = Server::start_at("127.0.0.2", move |raw, _| {
            let body = if raw.contains("/manifest ") {
                if hls {
                    "#EXTM3U\n#EXTINF:1.0,\nsegment.ts\n#EXT-X-ENDLIST\n"
                } else {
                    "<MPD type=\"static\" mediaPresentationDuration=\"PT2S\"><Period><AdaptationSet contentType=\"video\" mimeType=\"video/mp4\"><SegmentTemplate timescale=\"1000\" initialization=\"init.mp4\" media=\"$Time$.m4s\"><SegmentTimeline><S t=\"0\" d=\"2000\"/></SegmentTimeline></SegmentTemplate><Representation id=\"v\" bandwidth=\"100\"/></AdaptationSet></Period></MPD>"
                }
            } else {
                "0123456789abcdef"
            };
            reply(
                "200 OK",
                &format!("Content-Length: {}\r\n", body.len()),
                body,
                30,
            )
        });
        let target = media.url("/manifest");
        let origin = Server::start(move |_, _| {
            reply(
                "302 Found",
                &format!("Location: {target}\r\nContent-Length: 0\r\n"),
                "",
                0,
            )
        });
        let directory = tempfile::tempdir().expect("media store");
        let database = directory.path().join("downloads.sqlite3");
        let engine = DownloadEngine::open(&database).expect("media engine");
        let mut download = request(origin.url("/selected-manifest"));
        download.save_directory = directory.path().join("output");
        download.file_name = Some(if hls { "observed.ts" } else { "observed.mp4" }.into());
        download.media = Some(if hls {
            nxtools_download::DownloadRequestMedia::Hls(nxtools_download::HlsDownloadRequest {
                playlist_url: download.source_url.clone(),
            })
        } else {
            nxtools_download::DownloadRequestMedia::Dash(nxtools_download::DashDownloadRequest {
                manifest_url: download.source_url.clone(),
                video_representation_id: Some("v".into()),
                audio_representation_id: None,
            })
        });
        let task_id = engine
            .submit(download, RequestOrigin::Internal)
            .expect("explicit media task");
        let completed = engine
            .wait_for_terminal(task_id, Duration::from_secs(5))
            .await
            .expect("media terminal");
        assert_eq!(
            completed.task.status(),
            DownloadStatus::Completed,
            "{:?}",
            completed.error
        );
        let probe = completed.probe.expect("original media probe");
        let observed = probe
            .server_observation
            .as_ref()
            .expect("actual manifest response");
        assert_eq!(observed.host, "127.0.0.2");
        assert_eq!(observed.peer_ip, Some(media.address.ip()));
        assert!(observed.response_ms >= 20);
        assert_eq!(origin.count(), 1, "only original manifest GET");
        assert_eq!(
            media.count(),
            if hls { 2 } else { 3 },
            "manifest and original media fragments only"
        );
        drop(engine);
        let reopened = DownloadEngine::open(&database).expect("media reopen");
        assert_eq!(
            reopened
                .task(task_id)
                .expect("lookup")
                .expect("same media task")
                .probe
                .as_ref(),
            Some(&probe)
        );
    }
}

type Fields = std::collections::BTreeMap<String, String>;
#[derive(Clone, Default)]
struct ProbeDiagnostics(Arc<Mutex<Vec<Fields>>>);
#[derive(Default)]
struct ProbeFields(Fields);
impl tracing::field::Visit for ProbeFields {
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        self.0.insert(field.name().into(), value.into());
    }
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        self.0.insert(field.name().into(), format!("{value:?}"));
    }
}
impl tracing::Subscriber for ProbeDiagnostics {
    fn enabled(&self, _: &tracing::Metadata<'_>) -> bool {
        true
    }
    fn new_span(&self, _: &tracing::span::Attributes<'_>) -> tracing::span::Id {
        tracing::span::Id::from_u64(1)
    }
    fn record(&self, _: &tracing::span::Id, _: &tracing::span::Record<'_>) {}
    fn record_follows_from(&self, _: &tracing::span::Id, _: &tracing::span::Id) {}
    fn enter(&self, _: &tracing::span::Id) {}
    fn exit(&self, _: &tracing::span::Id) {}
    fn event(&self, event: &tracing::Event<'_>) {
        let mut fields = ProbeFields::default();
        event.record(&mut fields);
        if fields
            .0
            .get("event")
            .is_some_and(|value| value.starts_with("download.probe."))
        {
            self.0.lock().expect("captured probe events").push(fields.0);
        }
    }
}

#[test]
fn server_info_correlated_completion_logs_only_fixed_route_peer_presence_and_response_time() {
    for success in [true, false] {
        let server = Server::start(move |raw, _| {
            if success {
                if is_head(raw) { ordinary(0) } else { ranged(0) }
            } else {
                reply("503 Service Unavailable", "Content-Length: 0\r\n", "", 0)
            }
        });
        let engine = DownloadEngine::open_in_memory().expect("probe engine");
        let capture = ProbeDiagnostics::default();
        let recorded = capture.0.clone();
        let correlation_id = uuid::Uuid::new_v4();
        let mut download = request(server.url("/private-fixture-path?private_query=secret"));
        download.cookie = Some("private-cookie=secret".into());
        let result = tracing::subscriber::with_default(capture, || {
            engine.handle_command(DownloadCommandEnvelope {
                protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
                request_id: correlation_id,
                origin: RequestOrigin::Internal,
                command: DownloadCommand::Probe { request: download },
            })
        });
        assert_eq!(
            matches!(result.outcome, DownloadCommandOutcome::Probe(_)),
            success
        );
        let events = recorded.lock().expect("probe diagnostics");
        assert_eq!(
            events.len(),
            2,
            "one start and one terminal on original correlation"
        );
        for fields in events.iter() {
            assert!(fields["correlation_id"].contains(&correlation_id.to_string()));
            let text = format!("{fields:?}");
            for secret in [
                "127.0.0.1",
                "private-fixture",
                "private_query",
                "private-cookie",
                "secret",
            ] {
                assert!(!text.contains(secret), "diagnostics leaked fixture payload");
            }
        }
        let terminal = events.last().expect("terminal event");
        if success {
            assert!(terminal["route_kind"].contains("direct"));
            assert_eq!(terminal["peer_present"], "true");
            assert!(terminal.contains_key("response_ms"));
            assert_eq!(terminal["event"], "download.probe.completed");
        } else {
            assert_eq!(terminal["event"], "download.probe.failed");
            assert!(
                !terminal.contains_key("response_ms"),
                "failed request must not invent observation"
            );
        }
    }
}
