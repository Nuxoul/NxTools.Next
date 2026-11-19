use super::*;

#[test]
fn l6_legacy_and_reduced_protocol_two_payloads_share_the_original_projection() {
    for legacy in [true, false] {
        let mut wire = json!({
            "url": "https://files.example.test/owned.zip",
            "fileName": "owned.zip",
            "headers": { "Authorization": "synthetic", "Referer": "https://files.example.test/page", "Range": "bytes=0-", "Host": "ignored", "Content-Length": "99" },
            "cookie": "session=synthetic"
        });
        if legacy {
            wire["headerText"] = json!("ignored legacy representation");
            wire["headers"]["Cookie"] = json!("ignored duplicate");
        }
        let payload: BrowserDownloadPayload =
            serde_json::from_value(wire).expect("protocol-v2 payload");
        let request =
            build_browser_download_request(payload, &AppConfig::default(), std::env::temp_dir())
                .expect("original request projection");
        assert_eq!(request.cookie.as_deref(), Some("session=synthetic"));
        assert_eq!(
            request.referrer.as_deref(),
            Some("https://files.example.test/page")
        );
        assert_eq!(request.headers.len(), 1);
        assert_eq!(request.headers[0].name, "Authorization");
        assert_eq!(request.headers[0].value, "synthetic");
    }
    let minimal: BrowserDownloadPayload =
        serde_json::from_value(json!({ "url": "https://files.example.test/owned.zip" }))
            .expect("optional fields default");
    let request =
        build_browser_download_request(minimal, &AppConfig::default(), std::env::temp_dir())
            .expect("minimal request");
    assert!(request.headers.is_empty());
    assert!(request.cookie.is_none());
    assert!(request.referrer.is_none());
    assert!(request.source_page.is_none());
}

#[test]
fn l6_single_cookie_batch_passes_native_framing_and_host_intake_without_limit_changes() {
    let cookie = format!("s={}", "x".repeat(4094));
    let items: Vec<_> = (0..32).map(|index| json!({
        "handoffId": Uuid::new_v4(), "url": format!("https://files.example.test/file-{index}.zip"),
        "fileName": format!("file-{index}.zip"), "headers": { "authorization": "synthetic" },
        "cookie": cookie, "openDialog": false, "startImmediately": false
    })).collect();
    let wire = serde_json::to_vec(&json!({ "protocolVersion": 2, "type": "create_tasks", "requestId": Uuid::new_v4(), "payload": { "items": items } })).expect("serialize synthetic batch");
    assert!(wire.len() < MAX_NATIVE_MESSAGE_BYTES);
    let mut framed = Vec::with_capacity(wire.len() + 4);
    framed.extend_from_slice(&(wire.len() as u32).to_le_bytes());
    framed.extend_from_slice(&wire);
    let input = read_native_message(&mut framed.as_slice())
        .expect("bounded native frame")
        .expect("complete frame");
    let request: BrowserRequest = serde_json::from_slice(&input).expect("native request");
    let batch: BrowserBatchDownloadPayload =
        serde_json::from_value(request.payload).expect("batch payload");
    let command = build_browser_batch_command(
        batch,
        &AppConfig::default(),
        std::env::temp_dir(),
        Uuid::new_v4(),
    )
    .expect("32 item intake");
    assert!(
        matches!(command.command, DownloadCommand::BrowserBatchSubmit { submissions } if submissions.len() == 32)
    );
}

#[test]
fn source_page_fields_are_bounded_and_batch_uses_the_shared_builder() {
    let page_title = "文".repeat(1024);
    let payload: BrowserDownloadPayload = serde_json::from_value(json!({
        "handoffId": Uuid::new_v4(), "url": "https://files.example.test/archive.zip",
        "sourcePageUrl": "https://pages.example.test/article", "sourcePageTitle": page_title,
    }))
    .expect("native page facts");
    let request = build_browser_download_request(
        payload.clone(),
        &AppConfig::default(),
        std::env::temp_dir(),
    )
    .expect("1024 captured Chinese characters fit 4096 bytes");
    assert_eq!(
        request.source_page.as_ref().expect("page").title,
        page_title
    );
    let command = build_browser_batch_command(
        BrowserBatchDownloadPayload {
            items: vec![payload.clone()],
        },
        &AppConfig::default(),
        std::env::temp_dir(),
        Uuid::new_v4(),
    )
    .expect("batch projection");
    let DownloadCommand::BrowserBatchSubmit { submissions } = command.command else {
        panic!("batch")
    };
    assert_eq!(submissions[0].request.source_page, request.source_page);
    let oversized: BrowserDownloadPayload = serde_json::from_value(json!({
        "url": "https://files.example.test/archive.zip", "sourcePageTitle": "x".repeat(4097)
    }))
    .expect("decode oversized title");
    assert!(
        build_browser_download_request(oversized, &AppConfig::default(), std::env::temp_dir())
            .is_err()
    );
    let oversized_url: BrowserDownloadPayload = serde_json::from_value(json!({
        "url": "https://files.example.test/archive.zip", "sourcePageUrl": "x".repeat(MAX_URL_BYTES + 1)
    })).expect("decode oversized URL");
    assert!(
        build_browser_download_request(oversized_url, &AppConfig::default(), std::env::temp_dir())
            .is_err()
    );
    let title_only: BrowserDownloadPayload = serde_json::from_value(json!({
        "url": "https://files.example.test/archive.zip", "sourcePageTitle": "orphan title"
    }))
    .expect("missing page URL");
    assert!(
        build_browser_download_request(title_only, &AppConfig::default(), std::env::temp_dir())
            .expect("title without URL")
            .source_page
            .is_none()
    );
}

fn framed_browser_request(
    request_id: Uuid,
    handoff_id: Uuid,
    cookie: &str,
    open_dialog: bool,
) -> (BrowserRequest, Vec<u8>) {
    let mut bytes = Vec::new();
    write_native_message(
        &mut bytes,
        &json!({
            "protocolVersion": BROWSER_PROTOCOL_VERSION,
            "type": "create_task",
            "requestId": request_id,
            "payload": {
                "handoffId": handoff_id,
                "url": "https://files.example.test/same.zip",
                "fileName": "same.zip",
                "cookie": cookie,
                "openDialog": open_dialog,
                "startImmediately": false
            }
        }),
    )
    .expect("write synthetic native request");
    let decoded = read_native_message(&mut bytes.as_slice())
        .expect("read bounded native request")
        .expect("request frame exists");
    (
        serde_json::from_slice(&decoded).expect("parse native request"),
        decoded,
    )
}

fn framed_browser_response(response: &BrowserResponse) -> Value {
    let mut bytes = Vec::new();
    write_native_message(&mut bytes, response).expect("write synthetic native reply");
    let frame = read_native_message(&mut bytes.as_slice())
        .expect("read bounded native reply")
        .expect("reply frame exists");
    serde_json::from_slice(&frame).expect("parse native reply")
}

fn browser_fixture_paths(root: &Path) -> AppPaths {
    let paths = AppPaths {
        config_dir: root.join("config"),
        data_dir: root.join("data"),
        cache_dir: root.join("cache"),
        log_dir: root.join("logs"),
    };
    std::fs::create_dir_all(&paths.config_dir).expect("create synthetic config directory");
    let mut config = AppConfig::default();
    config.extensions.bundled_defaults_initialized = true;
    config.download.default_directory = root.join("artifacts").to_string_lossy().into_owned();
    config
        .save_to(&paths.config_file())
        .expect("persist synthetic download settings");
    paths
}

fn start_browser_fixture_host(engine: nxtools_download::DownloadEngine) -> String {
    use nxtools_download_ipc::{DownloadHostActivity, run_download_host};

    let pipe_name = format!(r"\\.\pipe\nxtools-browser-b1-{}", Uuid::new_v4());
    let host_pipe = pipe_name.clone();
    std::thread::spawn(move || {
        let _ = run_download_host(&host_pipe, engine, DownloadHostActivity::default());
    });
    pipe_name
}

fn submit_framed_to_fixture(
    paths: &AppPaths,
    pipe_name: &str,
    request_id: Uuid,
    handoff_id: Uuid,
    cookie: &str,
    open_dialog: bool,
) -> BrowserResponse {
    let (wire, _) = framed_browser_request(request_id, handoff_id, cookie, open_dialog);
    assert_eq!(wire.kind, "create_task");
    submit_download_with_client(
        wire.request_id,
        wire.payload,
        paths,
        |mode, id| {
            assert_eq!(mode, DesktopStartMode::for_browser_request(open_dialog));
            assert_eq!(id, request_id);
            DownloadIpcClient::connect(pipe_name, Duration::from_secs(2))
        },
        |client, command, dialog| {
            if dialog {
                command_browser_dialog_when_ready(client, command)
            } else {
                client.command(command)
            }
        },
    )
    .expect("framed browser request traverses production submit logic")
}

#[test]
fn b1_distinct_same_url_cookie_context_survives_native_framing_and_domain_projection() {
    let root = std::env::temp_dir().join(format!("browser-cookie-{}", Uuid::new_v4()));
    let paths = browser_fixture_paths(&root);
    let engine = nxtools_download::DownloadEngine::open_in_memory().expect("open synthetic engine");
    let pipe = start_browser_fixture_host(engine.clone());
    let mut task_ids = Vec::new();
    for store_cookie in ["container=synthetic-A", "container=synthetic-B"] {
        let request_id = Uuid::new_v4();
        let handoff_id = Uuid::new_v4();
        let response =
            submit_framed_to_fixture(&paths, &pipe, request_id, handoff_id, store_cookie, false);
        let frame = framed_browser_response(&response);
        assert_eq!(frame["requestId"], json!(request_id));
        assert_eq!(frame["handoffId"], json!(handoff_id));
        assert_eq!(frame["accepted"], true);
        let task_id = DownloadTaskId::from_uuid(
            Uuid::parse_str(frame["taskId"].as_str().expect("task ID")).expect("task UUID"),
        );
        let stored = engine
            .task(task_id)
            .expect("read task")
            .expect("persisted task");
        assert_eq!(
            stored.task.request().source_url,
            "https://files.example.test/same.zip"
        );
        assert_eq!(stored.task.request().cookie.as_deref(), Some(store_cookie));
        task_ids.push(task_id);
    }
    assert_ne!(task_ids[0], task_ids[1]);
    assert_eq!(engine.tasks().expect("distinct stored tasks").len(), 2);
}

#[test]
fn b1_durable_queue_only_receipt_and_replay_are_not_transport_admission() {
    let root = std::env::temp_dir().join(format!("browser-durable-{}", Uuid::new_v4()));
    let paths = browser_fixture_paths(&root);
    let database = root.join("downloads.sqlite3");
    let handoff_id = Uuid::new_v4();
    let request_id = Uuid::new_v4();
    let first = {
        let engine =
            nxtools_download::DownloadEngine::open(&database).expect("open temporary engine store");
        let pipe = start_browser_fixture_host(engine.clone());
        let first = submit_framed_to_fixture(
            &paths,
            &pipe,
            request_id,
            handoff_id,
            "source=synthetic",
            false,
        );
        let task_id = DownloadTaskId::from_uuid(
            Uuid::parse_str(first.task_id.as_deref().expect("durable task ID")).expect("task UUID"),
        );
        let stored = engine
            .task(task_id)
            .expect("read task")
            .expect("durably stored task");
        assert_eq!(
            stored.task.request().cookie.as_deref(),
            Some("source=synthetic")
        );
        first
    };
    let reopened =
        nxtools_download::DownloadEngine::open(&database).expect("reopen durable engine store");
    let pipe = start_browser_fixture_host(reopened.clone());
    let replay_id = Uuid::new_v4();
    let replay = submit_framed_to_fixture(
        &paths,
        &pipe,
        replay_id,
        handoff_id,
        "source=synthetic",
        false,
    );
    let first_frame = framed_browser_response(&first);
    let replay_frame = framed_browser_response(&replay);
    assert_eq!(first_frame["requestId"], json!(request_id));
    assert_eq!(replay_frame["requestId"], json!(replay_id));
    assert_eq!(first_frame["handoffId"], json!(handoff_id));
    assert_eq!(replay_frame["handoffId"], json!(handoff_id));
    assert_eq!(first_frame["accepted"], true);
    assert_eq!(replay_frame["accepted"], true);
    assert_eq!(first_frame["duplicate"], false);
    assert_eq!(replay_frame["duplicate"], true);
    assert_eq!(replay_frame["taskId"], first_frame["taskId"]);
    assert_eq!(reopened.tasks().expect("one durable task").len(), 1);
    for frame in [&first_frame, &replay_frame] {
        assert_eq!(frame["data"]["queued"], true);
        assert_eq!(frame["data"]["handoffState"], "persisted");
        assert_eq!(frame["data"]["reason"], "desktop_accepted");
    }
}

#[test]
fn source_page_survives_framed_intake_and_sqlite_reopen() {
    use nxtools_download::{
        DownloadResolutionControl, DownloadResolutionMode, DownloadResolutionOptions,
    };
    use std::io::{Read, Write};
    use std::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0").expect("local redirect listener");
    let address = listener.local_addr().expect("local address");
    listener
        .set_nonblocking(true)
        .expect("bounded fixture listener");
    let server = std::thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    stream
                        .set_nonblocking(false)
                        .expect("blocking accepted stream");
                    stream
                        .set_read_timeout(Some(Duration::from_secs(2)))
                        .expect("read deadline");
                    let mut bytes = [0u8; 4096];
                    let length = stream.read(&mut bytes).expect("fixture request");
                    let request = String::from_utf8_lossy(&bytes[..length]);
                    let response = if request.contains(" /original ") {
                        "HTTP/1.1 302 Found\r\nLocation: /final\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_owned()
                    } else {
                        "HTTP/1.1 200 OK\r\nContent-Length: 16\r\nContent-Type: application/octet-stream\r\nConnection: close\r\n\r\n0123456789abcdef".to_owned()
                    };
                    stream
                        .write_all(response.as_bytes())
                        .expect("fixture response");
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(10))
                }
                Err(error) => panic!("fixture accept: {error}"),
            }
        }
    });
    let root = std::env::temp_dir().join(format!("browser-source-page-{}", Uuid::new_v4()));
    let paths = browser_fixture_paths(&root);
    let database = root.join("downloads.sqlite3");
    let engine = nxtools_download::DownloadEngine::open(&database).expect("temporary store");
    let pipe = start_browser_fixture_host(engine.clone());
    let handoff_id = Uuid::new_v4();
    let source_url = format!("http://{address}/original");
    let final_url = format!("http://{address}/final");
    let page_url = "https://pages.example.test/article?chapter=3";
    let referrer = "https://referrer.example.test/";
    let mut frame = Vec::new();
    write_native_message(&mut frame, &json!({
        "protocolVersion": BROWSER_PROTOCOL_VERSION, "type": "create_task", "requestId": Uuid::new_v4(),
        "payload": { "handoffId": handoff_id, "url": source_url, "fileName": "original.bin",
            "referrer": referrer, "cookie": "fixture=secret", "sourcePageUrl": page_url,
            "sourcePageTitle": "来源页面", "startImmediately": false, "openDialog": false }
    })).expect("framed browser payload");
    let bytes = read_native_message(&mut frame.as_slice())
        .expect("read frame")
        .expect("frame");
    let wire: BrowserRequest = serde_json::from_slice(&bytes).expect("browser request");
    let receipt = submit_download_with_client(
        wire.request_id,
        wire.payload,
        &paths,
        |_, _| DownloadIpcClient::connect(&pipe, Duration::from_secs(2)),
        |client, command, _| client.command(command),
    )
    .expect("owned pipe admission");
    assert!(receipt.accepted);
    let task_id = DownloadTaskId::from_uuid(
        Uuid::parse_str(receipt.task_id.as_deref().expect("task ID")).expect("UUID"),
    );
    drop(engine);
    let reopened = nxtools_download::DownloadEngine::open(&database).expect("reopen SQLite store");
    let detail = reopened
        .task(task_id)
        .expect("selected task detail")
        .expect("persisted task");
    let mut request = detail.task.request().clone();
    assert_eq!(
        serde_json::to_value(&request).expect("serialize stored request")["source_page"],
        json!({ "url": page_url, "title": "来源页面" })
    );
    let selected = reopened.handle_command(command_envelope(
        Uuid::new_v4(),
        DownloadCommand::TaskDetail { task_id },
        RequestOrigin::Browser,
    ));
    let DownloadCommandOutcome::TaskDetail(selected) = selected.outcome else {
        panic!("selected TaskDetail must contain the full request")
    };
    assert_eq!(
        selected.task.task.request().source_page,
        request.source_page
    );
    assert_eq!(request.source_url, source_url);
    assert_eq!(request.referrer.as_deref(), Some(referrer));
    assert_eq!(request.cookie.as_deref(), Some("fixture=secret"));
    request.file_name = Some("user-edited.bin".into());
    request.save_directory = root.join("chosen");
    let probe = reopened
        .probe_with_resolution(
            request.clone(),
            DownloadResolutionOptions {
                operation_id: Uuid::new_v4(),
                mode: DownloadResolutionMode::Bypass,
            },
            DownloadResolutionControl::default(),
        )
        .expect("probe follows real redirect");
    assert_eq!(probe.final_url, final_url);
    assert_eq!(request.file_name.as_deref(), Some("user-edited.bin"));
    assert_eq!(
        serde_json::to_value(&request).expect("edited request")["source_page"]["url"],
        page_url
    );
    server.join().expect("redirect fixture");
}

#[test]
fn b1_dialog_result_only_confirms_window_creation_not_durable_acceptance() {
    let root = std::env::temp_dir().join(format!("browser-dialog-{}", Uuid::new_v4()));
    let paths = browser_fixture_paths(&root);
    let request_id = Uuid::new_v4();
    let handoff_id = Uuid::new_v4();
    let engine = nxtools_download::DownloadEngine::open_in_memory().expect("open temporary engine");
    let dialogs = engine
        .take_browser_dialog_receiver()
        .expect("attach synthetic UI consumer");
    let pipe = start_browser_fixture_host(engine.clone());
    let consumer = std::thread::spawn(move || {
        let dialog = dialogs.recv_blocking().expect("real engine dialog request");
        assert_eq!(dialog.correlation_id, request_id);
        assert_eq!(dialog.handoff_id, handoff_id);
        dialog
            .completion
            .send(Ok(()))
            .expect("synthetic window created");
    });
    let receipt = submit_framed_to_fixture(
        &paths,
        &pipe,
        request_id,
        handoff_id,
        "source=synthetic",
        true,
    );
    consumer.join().expect("UI consumer completed");
    assert!(engine.tasks().expect("inspect persistent tasks").is_empty());
    let frame = framed_browser_response(&receipt);
    assert_eq!(frame["requestId"], json!(request_id));
    assert_eq!(frame["handoffId"], json!(handoff_id));
    assert_eq!(frame["accepted"], false);
    assert_eq!(frame["data"]["queued"], false);
    assert_eq!(frame["data"]["handoffState"], "awaiting_confirmation");
    assert_eq!(frame["data"]["reason"], "confirmation_pending");
}

#[test]
fn b1_rejected_handoff_uses_fixed_reason_without_payload() {
    let root = std::env::temp_dir().join(format!("browser-refused-{}", Uuid::new_v4()));
    let paths = browser_fixture_paths(&root);
    let engine = nxtools_download::DownloadEngine::open_in_memory().expect("open temporary engine");
    let pipe = start_browser_fixture_host(engine.clone());
    let handoff_id = Uuid::new_v4();
    let accepted = submit_framed_to_fixture(
        &paths,
        &pipe,
        Uuid::new_v4(),
        handoff_id,
        "source=original",
        false,
    );
    assert!(accepted.accepted);
    let request_id = Uuid::new_v4();
    let response = submit_framed_to_fixture(
        &paths,
        &pipe,
        request_id,
        handoff_id,
        "source=conflicting-secret",
        false,
    );
    let frame = framed_browser_response(&response);
    assert_eq!(frame["requestId"], json!(request_id));
    assert_eq!(frame["handoffId"], json!(handoff_id));
    assert_eq!(frame["accepted"], false);
    assert_eq!(frame["data"]["handoffState"], "rejected");
    assert_eq!(frame["data"]["reason"], "desktop_refused");
    assert!(!frame.to_string().contains("conflicting-secret"));
    assert_eq!(engine.tasks().expect("one original task").len(), 1);
}

#[test]
fn b1_mismatched_engine_result_cannot_claim_acceptance() {
    let root = std::env::temp_dir().join(format!("browser-id-mismatch-{}", Uuid::new_v4()));
    let paths = browser_fixture_paths(&root);
    let engine = nxtools_download::DownloadEngine::open_in_memory().expect("open temporary engine");
    let pipe = start_browser_fixture_host(engine.clone());
    let request_id = Uuid::new_v4();
    let handoff_id = Uuid::new_v4();
    let (wire, _) = framed_browser_request(request_id, handoff_id, "source=synthetic", false);
    let response = submit_download_with_client(
        wire.request_id,
        wire.payload,
        &paths,
        |mode, id| {
            assert_eq!(mode, DesktopStartMode::Background);
            assert_eq!(id, request_id);
            DownloadIpcClient::connect(&pipe, Duration::from_secs(2))
        },
        |client, command, dialog| {
            assert!(!dialog);
            assert_eq!(command.request_id, request_id);
            let mut result = client.command(command)?;
            assert!(matches!(
                result.outcome,
                DownloadCommandOutcome::BrowserAccepted { .. }
            ));
            result.request_id = Uuid::new_v4();
            Ok(result)
        },
    )
    .expect("mismatched result has a browser-safe terminal receipt");
    let frame = framed_browser_response(&response);
    assert_eq!(
        engine
            .tasks()
            .expect("durable task exists despite lost result identity")
            .len(),
        1
    );
    assert_eq!(frame["requestId"], json!(request_id));
    assert_eq!(frame["handoffId"], json!(handoff_id));
    assert_eq!(frame["accepted"], false);
    assert_eq!(frame["data"]["handoffState"], "rejected");
    assert_eq!(frame["data"]["reason"], "identity_mismatch");
}

#[test]
fn b1_unavailable_desktop_keeps_original_request_and_handoff_identity() {
    let root = std::env::temp_dir().join(format!("browser-unavailable-{}", Uuid::new_v4()));
    let paths = browser_fixture_paths(&root);
    let request_id = Uuid::new_v4();
    let handoff_id = Uuid::new_v4();
    let (wire, _) = framed_browser_request(request_id, handoff_id, "secret=synthetic", false);
    let result = submit_download_with_client(
        wire.request_id,
        wire.payload,
        &paths,
        |_, id| {
            assert_eq!(id, request_id);
            Err(anyhow::anyhow!("synthetic desktop connection unavailable"))
        },
        |_, _, _| panic!("no command may be sent without a connected desktop"),
    )
    .expect("known handoff identity survives failed desktop connection");
    let frame = framed_browser_response(&result);
    assert_eq!(frame["requestId"], json!(request_id));
    assert_eq!(frame["handoffId"], json!(handoff_id));
    assert_eq!(frame["accepted"], false);
    assert_eq!(frame["data"]["handoffState"], "rejected");
    assert_eq!(frame["data"]["reason"], "desktop_unavailable");
    assert!(!frame.to_string().contains("secret=synthetic"));
}

type DiagnosticFields = std::collections::BTreeMap<String, String>;

#[derive(Clone, Default)]
struct BrowserDiagnostics(std::sync::Arc<std::sync::Mutex<Vec<DiagnosticFields>>>);

#[derive(Default)]
struct BrowserDiagnosticFields(DiagnosticFields);

impl tracing::field::Visit for BrowserDiagnosticFields {
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        self.0.insert(field.name().into(), value.into());
    }

    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        self.0.insert(field.name().into(), format!("{value:?}"));
    }
}

impl tracing::Subscriber for BrowserDiagnostics {
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
        let mut fields = BrowserDiagnosticFields::default();
        event.record(&mut fields);
        if fields
            .0
            .get("event")
            .is_some_and(|value| value == "browser_extension.event")
        {
            self.0.lock().expect("browser event capture").push(fields.0);
        }
    }
}

#[test]
fn b1_handoff_state_log_accepts_only_typed_fixed_metadata_and_discards_secrets() {
    let correlation_id = Uuid::new_v4();
    let root = std::env::temp_dir().join(format!("browser-log-{}", Uuid::new_v4()));
    let paths = browser_fixture_paths(&root);
    let blob_uploads = BlobUploadService::start(root.join("blob"))
        .expect("start isolated Blob listener for production router");
    let request_id = Uuid::new_v4();
    let payload = json!({
        "event": "handoff_state",
        "correlationId": correlation_id,
        "handoffState": "rejected",
        "reason": "desktop_refused",
        "elapsedMs": 23,
        "boundaryCode": "native_rejected",
        "compensation": "failed",
        "url": "https://private.example/secret-path",
        "cookie": "secret-cookie=private",
        "message": "secret-message",
        "headers": { "Authorization": "secret-token" }
    });
    let typed: BrowserExtensionLogPayload =
        serde_json::from_value(payload.clone()).expect("typed handoff diagnostic payload");
    assert_eq!(
        validate_extension_log_payload(&typed).expect("bounded fixed metadata"),
        Some(correlation_id)
    );
    let capture = BrowserDiagnostics::default();
    let recorded = capture.0.clone();
    let response = tracing::subscriber::with_default(capture, || {
        handle_request(
            serde_json::from_value(json!({
                "protocolVersion": BROWSER_PROTOCOL_VERSION,
                "type": "extension_log",
                "requestId": request_id,
                "payload": payload
            }))
            .expect("actual browser log request"),
            &paths,
            Path::new("not-a-product-host.exe"),
            &blob_uploads,
        )
    })
    .expect("actual extension log route");
    let reply = framed_browser_response(&response);
    assert_eq!(reply["requestId"], json!(request_id));
    assert_eq!(reply["accepted"], false);
    let diagnostics = recorded.lock().expect("captured browser diagnostics");
    assert_eq!(diagnostics.len(), 1);
    let fields = &diagnostics[0];
    assert!(fields["correlation_id"].contains(&correlation_id.to_string()));
    assert_eq!(fields["extension_event"].trim_matches('"'), "handoff_state");
    assert_eq!(fields["handoff_state"].trim_matches('"'), "rejected");
    assert_eq!(fields["reason"].trim_matches('"'), "desktop_refused");
    assert_eq!(fields["elapsed_ms"], "23");
    assert_eq!(fields["boundary_code"].trim_matches('"'), "native_rejected");
    assert_eq!(fields["compensation"].trim_matches('"'), "failed");
    let metadata = format!("{fields:?}");
    for secret in [
        "secret-path",
        "secret-cookie",
        "secret-message",
        "secret-token",
    ] {
        assert!(
            !metadata.contains(secret),
            "diagnostic payload must not enter logs"
        );
    }
}

#[test]
fn b1_handoff_state_log_rejects_unbounded_or_unrecognized_categories() {
    let correlation_id = Uuid::new_v4();
    let valid = json!({
        "event": "handoff_state",
        "correlationId": correlation_id,
        "handoffState": "persisted",
        "reason": "desktop_accepted",
        "elapsedMs": 0
    });
    for (field, bad) in [
        ("handoffState", json!("secret-URL")),
        ("reason", json!("secret-cookie")),
        ("elapsedMs", json!(-1)),
        ("elapsedMs", json!("24")),
        ("boundaryCode", json!("secret-network-error")),
        ("compensation", json!("secret-rollback")),
    ] {
        let mut raw = valid.clone();
        raw[field] = bad;
        let parsed = serde_json::from_value::<BrowserExtensionLogPayload>(raw);
        assert!(
            parsed.is_err()
                || validate_extension_log_payload(&parsed.expect("typed diagnostic")).is_err(),
            "{field} must remain a typed, payload-safe category"
        );
    }
    for (state, reason) in [
        ("submitting", "handoff_started"),
        ("persisted", "desktop_accepted"),
        ("transferred", "browser_transferred"),
        ("skipped", "rule_excluded"),
        ("skipped", "temporarily_paused"),
        ("queued", "queued_not_accepted"),
        ("awaiting_confirmation", "confirmation_pending"),
        ("rejected", "desktop_refused"),
        ("rejected", "identity_mismatch"),
        ("failed", "desktop_unavailable"),
        ("failed", "handoff_timeout"),
        ("failed", "handoff_disconnected"),
        ("failed", "browser_cancel_failed"),
    ] {
        let mut raw = valid.clone();
        raw["handoffState"] = json!(state);
        raw["reason"] = json!(reason);
        let parsed: BrowserExtensionLogPayload =
            serde_json::from_value(raw).expect("documented diagnostic categories");
        assert_eq!(
            validate_extension_log_payload(&parsed).expect("known category"),
            Some(correlation_id)
        );
    }
    for field in ["correlationId", "handoffState", "reason", "elapsedMs"] {
        let mut raw = valid.clone();
        raw.as_object_mut()
            .expect("diagnostic object")
            .remove(field);
        let parsed = serde_json::from_value::<BrowserExtensionLogPayload>(raw);
        assert!(
            parsed.is_err()
                || validate_extension_log_payload(&parsed.expect("typed diagnostic")).is_err(),
            "handoff_state requires {field}"
        );
    }
    for boundary_code in [
        "connect_failed",
        "connect_timeout",
        "port_disconnected",
        "native_rejected",
        "request_too_large",
        "request_timeout",
        "send_failed",
    ] {
        let mut raw = valid.clone();
        raw["boundaryCode"] = json!(boundary_code);
        let parsed: BrowserExtensionLogPayload =
            serde_json::from_value(raw).expect("fixed boundary code");
        assert!(validate_extension_log_payload(&parsed).is_ok());
    }
    for compensation in ["not_available", "succeeded", "failed"] {
        let mut raw = valid.clone();
        raw["compensation"] = json!(compensation);
        let parsed: BrowserExtensionLogPayload =
            serde_json::from_value(raw).expect("fixed compensation");
        assert!(validate_extension_log_payload(&parsed).is_ok());
    }
}

#[test]
fn b1_production_extension_cookie_context_and_ownership_roundtrip() {
    let root = std::env::temp_dir().join(format!("browser-extension-roundtrip-{}", Uuid::new_v4()));
    let paths = browser_fixture_paths(&root);
    let engine = nxtools_download::DownloadEngine::open_in_memory().expect("open roundtrip engine");
    let dialogs = engine
        .take_browser_dialog_receiver()
        .expect("attach synthetic confirmation consumer");
    let pipe = start_browser_fixture_host(engine.clone());
    let controlled = std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new()));
    let pending_controlled = controlled.clone();
    let consumer = std::thread::spawn(move || {
        for expected in [
            "session=firefox-container-2",
            "session=firefox-private",
            "session=firefox-container-3",
        ] {
            let dialog = dialogs
                .recv_blocking()
                .expect("real extension confirmation request");
            match dialog.source {
                nxtools_download::BrowserDownloadDialogSource::ControlledDownload(request) => {
                    assert_eq!(request.cookie.as_deref(), Some(expected));
                    pending_controlled
                        .lock()
                        .expect("pending confirmation")
                        .insert(dialog.handoff_id, *request);
                }
                nxtools_download::BrowserDownloadDialogSource::Download(request) => {
                    assert_eq!(request.cookie.as_deref(), Some(expected));
                }
                _ => panic!("HTTP extension request source required"),
            }
            dialog
                .completion
                .send(Ok(()))
                .expect("synthetic confirmation window created");
        }
    });
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join(
        "../../../browser_extension/nxtools-interceptor/tests/harness/native-host-roundtrip.cjs",
    );
    let mut child = Command::new("node")
        .arg(script)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Node production extension fixture is required");
    let mut input = child.stdin.take().expect("extension native reply pipe");
    let mut output = child.stdout.take().expect("extension native request pipe");
    let mut completed = false;
    let mut requests = 0;
    let mut accepted_handoff = None;
    let mut accepted_task = None;
    while let Some(frame) = read_native_message(&mut output).expect("extension native framing") {
        let raw: Value = serde_json::from_slice(&frame).expect("extension JSON request");
        if raw["type"] == "fixture_complete" {
            assert_eq!(raw["scenarios"], 5);
            completed = true;
            break;
        }
        let wire: BrowserRequest =
            serde_json::from_value(raw).expect("production extension request");
        let response = match wire.kind.as_str() {
            "prepare_browser_handoff" => controlled_handoff::prepare_with_client(
                wire.request_id,
                wire.payload,
                &paths,
                |_, _| DownloadIpcClient::connect(&pipe, Duration::from_secs(2)),
            )
            .expect("controlled preparation through real host"),
            "get_browser_handoff" | "commit_browser_handoff" | "abort_browser_handoff" => {
                let handoff =
                    Uuid::parse_str(wire.payload["handoffId"].as_str().expect("handoff UUID"))
                        .expect("UUID");
                if wire.kind == "get_browser_handoff"
                    && let Some(request) = controlled
                        .lock()
                        .expect("pending confirmations")
                        .remove(&handoff)
                {
                    let accepted = engine.handle_command(command_envelope(
                        handoff,
                        DownloadCommand::BrowserAcceptHandoff {
                            handoff_id: handoff,
                            request,
                            mode: DownloadSubmitMode::QueueOnly,
                            resolution: nxtools_download::DownloadResolutionOptions {
                                operation_id: handoff,
                                mode: nxtools_download::DownloadResolutionMode::Bypass,
                            },
                        },
                        RequestOrigin::Browser,
                    ));
                    assert!(matches!(
                        accepted.outcome,
                        DownloadCommandOutcome::BrowserHandoff {
                            state: nxtools_download::BrowserHandoffState::Held,
                            ..
                        }
                    ));
                }
                controlled_handoff::operate_with_client(
                    wire.request_id,
                    &wire.kind,
                    wire.payload,
                    || DownloadIpcClient::connect(&pipe, Duration::from_secs(2)),
                )
                .expect("controlled ownership through real host")
            }
            "create_tasks" => {
                let payload = serde_json::from_value(wire.payload).expect("batch payload");
                let config = AppConfig::load_or_create(&paths).expect("fixture config");
                let command = build_browser_batch_command(
                    payload,
                    &config,
                    PathBuf::from(&config.download.default_directory),
                    correlation_id(&wire.request_id).expect("batch UUID"),
                )
                .expect("original batch command");
                let result = engine.handle_command(command);
                let DownloadCommandOutcome::BrowserBatchAccepted { results } = result.outcome
                else {
                    panic!("batch receipt")
                };
                BrowserResponse::batch_accepted(wire.request_id, results)
            }
            "create_task" => {
                if wire.payload["openDialog"] == true {
                    assert_eq!(
                        engine
                            .tasks()
                            .expect("only controlled tasks before manual confirmation")
                            .len(),
                        2
                    );
                } else {
                    assert_eq!(wire.payload["startImmediately"], false);
                }
                submit_download_with_client(
                    wire.request_id,
                    wire.payload,
                    &paths,
                    |_, _| DownloadIpcClient::connect(&pipe, Duration::from_secs(2)),
                    |client, command, open_dialog| {
                        if open_dialog {
                            command_browser_dialog_when_ready(client, command)
                        } else {
                            client.command(command)
                        }
                    },
                )
                .expect("production extension request traverses host and engine")
            }
            "task_action" => {
                assert_eq!(wire.action, "cancel");
                assert_eq!(Some(&wire.request_id), accepted_handoff.as_ref());
                assert_eq!(Some(&wire.task_id), accepted_task.as_ref());
                let id =
                    correlation_id(&wire.request_id).expect("original compensation correlation");
                let task_id = DownloadTaskId::from_uuid(
                    Uuid::parse_str(&wire.task_id).expect("accepted task UUID"),
                );
                let mut client = DownloadIpcClient::connect(&pipe, Duration::from_secs(2))
                    .expect("compensation pipe");
                let result = client
                    .command(command_envelope(
                        id,
                        DownloadCommand::Task {
                            task_id,
                            action: DownloadTaskAction::Cancel,
                        },
                        RequestOrigin::Browser,
                    ))
                    .expect("actual compensation command");
                assert_eq!(result.request_id, id);
                let DownloadCommandOutcome::Accepted { task_id } = result.outcome else {
                    panic!("original task cancellation must be accepted");
                };
                task_action_response(wire.request_id, false, task_id)
            }
            _ => panic!("unexpected extension fixture request"),
        };
        if response.accepted {
            accepted_handoff = response.handoff_id.clone();
            accepted_task = response.task_id.clone();
        }
        write_native_message(&mut input, &response).expect("host response framing");
        requests += 1;
    }
    drop(input);
    let result = child
        .wait_with_output()
        .expect("collect extension fixture result");
    assert!(
        result.status.success(),
        "extension fixture: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(
        completed,
        "extension completed its ownership and Cookie assertions"
    );
    assert!(
        requests >= 12,
        "real controlled status/commit plus manual/batch/compensation frames"
    );
    consumer.join().expect("confirmation consumer completed");
    let tasks = engine
        .tasks()
        .expect("controlled, batch and compensated tasks");
    assert_eq!(tasks.len(), 5);
    assert_eq!(
        tasks
            .iter()
            .filter(|entry| entry.task.status() == nxtools_download::DownloadStatus::Cancelled)
            .count(),
        1
    );
    assert_eq!(
        tasks
            .iter()
            .filter(|entry| entry.task.status() == nxtools_download::DownloadStatus::Created)
            .count(),
        4
    );
}

#[test]
fn b2_capture_pause_logs_use_bounded_correlated_native_frames() {
    let root = std::env::temp_dir().join(format!("capture-pause-log-{}", Uuid::new_v4()));
    let paths = browser_fixture_paths(&root);
    let blob_uploads = BlobUploadService::start(root.join("blob")).expect("isolated listener");
    let correlation_id = Uuid::new_v4();
    for (event, reason) in [
        ("capture_pause_started", "paused"),
        ("capture_pause_applied", "resumed"),
        ("capture_pause_failed", "session_storage_failed"),
        ("link_collection_started", "started"),
        ("link_collection_completed", "ready"),
        ("link_collection_failed", "page_changed"),
        ("media_inspection_started", "started"),
        ("media_inspection_completed", "protected"),
        ("media_inspection_failed", "too_large"),
        ("connection_check_started", "started"),
        ("connection_check_completed", "ready"),
        ("connection_check_failed", "connection_timeout"),
    ] {
        let mut bytes = Vec::new();
        write_native_message(&mut bytes, &json!({
            "protocolVersion": BROWSER_PROTOCOL_VERSION, "type": "extension_log",
            "requestId": Uuid::new_v4(), "payload": {
                "event": event, "correlationId": correlation_id, "reason": reason, "elapsedMs": 7,
                "url": "https://synthetic.invalid/private", "cookie": "private-cookie",
                "message": "private-message"
            }
        })).expect("native pause frame");
        let frame = read_native_message(&mut bytes.as_slice())
            .expect("bounded decode")
            .expect("frame");
        let capture = BrowserDiagnostics::default();
        let records = capture.0.clone();
        let response = tracing::subscriber::with_default(capture, || {
            handle_request(
                serde_json::from_slice(&frame).expect("typed native pause request"),
                &paths,
                Path::new("not-a-product-host.exe"),
                &blob_uploads,
            )
        })
        .expect("actual pause diagnostic route");
        assert_eq!(framed_browser_response(&response)["ok"], true);
        let records = records.lock().expect("captured pause metadata");
        assert_eq!(records.len(), 1);
        let fields = &records[0];
        assert!(fields["correlation_id"].contains(&correlation_id.to_string()));
        assert_eq!(fields["extension_event"], event);
        assert_eq!(fields["reason"], reason);
        assert_eq!(fields["elapsed_ms"], "7");
        let metadata = format!("{fields:?}");
        for secret in ["synthetic.invalid", "private-cookie", "private-message"] {
            assert!(!metadata.contains(secret));
        }
        for malformed in [
            json!({"event":event,"correlationId":correlation_id,"reason":"private-text","elapsedMs":7}),
            json!({"event":event,"correlationId":correlation_id,"reason":reason}),
            json!({"event":event,"reason":reason,"elapsedMs":7}),
        ] {
            let typed: BrowserExtensionLogPayload =
                serde_json::from_value(malformed).expect("typed malformed log");
            assert!(validate_extension_log_payload(&typed).is_err());
        }
    }
    drop(blob_uploads);
    let _ = std::fs::remove_dir_all(root);
}
