mod support;

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use nxtools_download::*;
use support::DeterministicHttpServer;
use uuid::Uuid;

type ResolutionFuture<'a> = Pin<
    Box<dyn Future<Output = Result<DownloadResolutionResult, DownloadResolutionError>> + Send + 'a>,
>;

struct Resolver {
    calls: AtomicUsize,
    target: String,
    fail: bool,
    delay: Duration,
    validity: Duration,
    context: Option<DownloadResolvedContext>,
}
impl DownloadRequestResolver for Resolver {
    fn identity(&self) -> DownloadResolverIdentity {
        DownloadResolverIdentity {
            content_version: "fixture-content".into(),
            settings_version: "fixture-settings".into(),
        }
    }
    fn resolve<'a>(
        &'a self,
        _: &'a DownloadRequest,
        operation: &'a DownloadResolutionOperation,
    ) -> ResolutionFuture<'a> {
        Box::pin(async move {
            self.calls.fetch_add(1, Ordering::SeqCst);
            tokio::time::sleep(self.delay).await;
            operation.check()?;
            if self.fail {
                return Err(DownloadResolutionError::Rejected);
            }
            Ok(DownloadResolutionResult::Resolved(Box::new(
                DownloadResolvedRequest {
                    source_url: self.target.clone(),
                    file_name: Some("suggestion.bin".into()),
                    context: self.context.clone(),
                    media: None,
                    provenance: vec![DownloadResolverStage {
                        package_id: "fixture".into(),
                        handler_id: "resolve".into(),
                        content_version: "fixture-content".into(),
                    }],
                    valid_for: Some(self.validity),
                },
            )))
        })
    }
}
fn resolver(target: String) -> Arc<Resolver> {
    Arc::new(Resolver {
        calls: AtomicUsize::new(0),
        target,
        fail: false,
        delay: Duration::ZERO,
        validity: Duration::from_secs(600),
        context: None,
    })
}
fn request(server: &DeterministicHttpServer, output: &std::path::Path) -> DownloadRequest {
    DownloadRequest {
        source_url: format!("http://{}/range", server.address()),
        save_directory: output.to_owned(),
        save_directory_explicit: true,
        file_name: Some("chosen.bin".into()),
        headers: Vec::new(),
        cookie: None,
        referrer: None,
        source_page: None,
        certificate_policy: Default::default(),
        connection_limit: Some(1),
        speed_limit_bytes_per_second: None,
        expected_total_bytes: None,
        scheduled_at_unix_ms: None,
        adaptive_segment_policy: None,
        media: None,
        category_id: None,
        category_name: None,
        duplicate_strategy: DownloadDuplicateStrategy::UseDefault,
    }
}
fn options() -> DownloadResolutionOptions {
    DownloadResolutionOptions {
        operation_id: Uuid::new_v4(),
        mode: DownloadResolutionMode::Resolve,
    }
}
fn probe(
    engine: &DownloadEngine,
    request: DownloadRequest,
    options: DownloadResolutionOptions,
) -> DownloadProbe {
    engine
        .probe_with_resolution(request, options, DownloadResolutionControl::default())
        .expect("probe fixture")
}

#[test]
fn resolution_cancel_during_http_probe_retires_the_operation_without_publication() {
    let server = DeterministicHttpServer::start_concurrent();
    let output = tempfile::tempdir().expect("synthetic output");
    let engine = DownloadEngine::open_in_memory().expect("engine");
    let mut input = request(&server, output.path());
    input.source_url = format!("http://{}/probe-stalled", server.address());
    let control = DownloadResolutionControl::default();
    let worker_control = control.clone();
    let worker_engine = engine.clone();
    let worker = std::thread::spawn(move || {
        worker_engine.probe_with_resolution(input, options(), worker_control)
    });
    let deadline = Instant::now() + Duration::from_secs(2);
    while server.total_requests() == 0 {
        assert!(Instant::now() < deadline, "HTTP probe must actually start");
        std::thread::sleep(Duration::from_millis(5));
    }
    let cancelled = Instant::now();
    control.cancel();
    let result = worker.join().expect("probe worker");
    assert!(matches!(
        result,
        Err(DownloadEngineError::Resolution(
            DownloadResolutionError::Cancelled
        ))
    ));
    assert!(
        cancelled.elapsed() < Duration::from_millis(300),
        "cancel must not await stalled headers"
    );
    assert!(engine.tasks().expect("task snapshot").is_empty());
    assert_eq!(std::fs::read_dir(output.path()).expect("output").count(), 0);
}

#[test]
fn resolution_noop_oracle_and_media_pass_through() {
    let server = DeterministicHttpServer::start();
    let output = tempfile::tempdir().expect("output");
    let engine = DownloadEngine::open_in_memory().expect("engine");
    let mut input = request(&server, output.path());
    input.duplicate_strategy = DownloadDuplicateStrategy::AddNumbered;
    let result = probe(&engine, input.clone(), options());
    assert!(result.resolution.is_none());
    let legacy_json = serde_json::to_value(&result).expect("serialize default probe");
    assert!(legacy_json.get("resolution").is_none());
    let decoded: DownloadProbe =
        serde_json::from_value(legacy_json).expect("legacy optional probe decode");
    assert_eq!(decoded, result);
    let id = engine
        .enqueue(input.clone(), RequestOrigin::Ui)
        .expect("enqueue");
    let task = engine.task(id).expect("task").expect("exists").task;
    let mut expected = input.clone();
    expected.headers = vec![DownloadRequestHeader {
        name: "User-Agent".into(),
        value: nxtools_core::DEFAULT_DOWNLOAD_USER_AGENT.into(),
    }];
    assert_eq!(task.request(), &expected);
    assert!(task.resolution().is_none());
    let mut media = input;
    media.file_name = Some("media.ts".into());
    media.media = Some(DownloadRequestMedia::Hls(HlsDownloadRequest {
        playlist_url: format!("http://{}/hls/vod.m3u8", server.address()),
    }));
    let id = engine
        .enqueue(media.clone(), RequestOrigin::Internal)
        .expect("enqueue media");
    assert_eq!(
        engine
            .task(id)
            .expect("task")
            .expect("exists")
            .task
            .request()
            .media,
        media.media
    );
}

#[test]
fn resolution_transformed_probe_receipt_reuse_and_durable_duplicate() {
    let server = DeterministicHttpServer::start();
    let output = tempfile::tempdir().expect("output");
    let db = output.path().join("downloads.sqlite3");
    let engine = DownloadEngine::open(&db).expect("engine");
    let adapter = resolver(format!("http://{}/range", server.address()));
    engine
        .set_request_resolver(adapter.clone())
        .expect("inject");
    let mut original = request(&server, output.path());
    original.source_url = format!("http://{}/page", server.address());
    let mut selection = options();
    let result = probe(&engine, original.clone(), selection.clone());
    assert!(result.final_url.ends_with("/range"));
    let resolved = result.resolution.expect("host resolution");
    selection.mode = DownloadResolutionMode::Receipt(resolved.receipt);
    let key = Uuid::new_v4();
    let (id, duplicate) = engine
        .submit_with_resolution(
            original.clone(),
            RequestOrigin::Cli,
            DownloadSubmitMode::QueueOnly,
            Some(key),
            selection.clone(),
            DownloadResolutionControl::default(),
        )
        .expect("accept receipt");
    assert!(!duplicate);
    assert_eq!(adapter.calls.load(Ordering::SeqCst), 1);
    let task = engine.task(id).expect("task").expect("exists").task;
    assert!(task.request().source_url.ends_with("/range"));
    assert_eq!(task.request().file_name.as_deref(), Some("chosen.bin"));
    assert_eq!(
        task.resolution().expect("provenance").original_request,
        original
    );
    let stored = SqliteDownloadStore::open(&db)
        .expect("reopen store")
        .load_tasks()
        .expect("load");
    assert_eq!(stored[0].resolution(), task.resolution());
    selection.mode = DownloadResolutionMode::Receipt(Uuid::new_v4());
    let replay = engine
        .submit_with_resolution(
            original,
            RequestOrigin::Cli,
            DownloadSubmitMode::QueueOnly,
            Some(key),
            selection,
            DownloadResolutionControl::default(),
        )
        .expect("accepted duplicate before forged receipt");
    assert_eq!(replay, (id, true));
    assert_eq!(adapter.calls.load(Ordering::SeqCst), 1);
}

#[test]
fn resolution_direct_submit_and_browser_batch_share_ingress() {
    let server = DeterministicHttpServer::start();
    let output = tempfile::tempdir().expect("output");
    let engine = DownloadEngine::open_in_memory().expect("engine");
    let adapter = resolver(format!("http://{}/range", server.address()));
    engine
        .set_request_resolver(adapter.clone())
        .expect("inject");
    let mut input = request(&server, output.path());
    input.source_url = format!("http://{}/page", server.address());
    let id = engine
        .enqueue(input.clone(), RequestOrigin::Internal)
        .expect("direct enqueue");
    assert!(
        engine
            .task(id)
            .expect("task")
            .expect("exists")
            .task
            .request()
            .source_url
            .ends_with("/range")
    );
    input.file_name = Some("browser.bin".into());
    let handoff_id = Uuid::new_v4();
    let command = DownloadCommandEnvelope {
        protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
        request_id: Uuid::new_v4(),
        origin: RequestOrigin::Browser,
        command: DownloadCommand::BrowserBatchSubmit {
            submissions: vec![BrowserDownloadSubmission {
                handoff_id,
                request: input,
            }],
        },
    };
    for (command, expected_duplicate) in [(command.clone(), false), (command, true)] {
        match engine.handle_command(command).outcome {
            DownloadCommandOutcome::BrowserBatchAccepted { results } => {
                assert_eq!(results.len(), 1);
                assert!(results[0].fault.is_none());
                assert!(results[0].task_id.is_some());
                assert_eq!(results[0].duplicate, expected_duplicate);
            }
            _ => panic!("expected browser batch acceptance"),
        }
    }
    assert_eq!(adapter.calls.load(Ordering::SeqCst), 2);
}

#[test]
fn resolution_failure_creates_no_task_and_bypass_is_explicit() {
    let server = DeterministicHttpServer::start();
    let output = tempfile::tempdir().expect("output");
    let engine = DownloadEngine::open_in_memory().expect("engine");
    let adapter = Arc::new(Resolver {
        fail: true,
        ..Resolver {
            calls: AtomicUsize::new(0),
            target: String::new(),
            fail: false,
            delay: Duration::ZERO,
            validity: Duration::from_secs(600),
            context: None,
        }
    });
    engine
        .set_request_resolver(adapter.clone())
        .expect("inject");
    let input = request(&server, output.path());
    assert!(engine.enqueue(input.clone(), RequestOrigin::Ui).is_err());
    assert!(engine.tasks().expect("tasks").is_empty());
    let mut selection = options();
    selection.mode = DownloadResolutionMode::Bypass;
    engine
        .submit_with_resolution(
            input,
            RequestOrigin::Ui,
            DownloadSubmitMode::QueueOnly,
            None,
            selection,
            DownloadResolutionControl::default(),
        )
        .expect("explicit bypass");
    assert_eq!(adapter.calls.load(Ordering::SeqCst), 1);
}

#[test]
fn resolution_forged_expired_wrong_operation_and_revoked_receipts_do_not_rerun() {
    let server = DeterministicHttpServer::start();
    let output = tempfile::tempdir().expect("output");
    let engine = DownloadEngine::open_in_memory().expect("engine");
    let adapter = Arc::new(Resolver {
        calls: AtomicUsize::new(0),
        target: format!("http://{}/range", server.address()),
        fail: false,
        delay: Duration::ZERO,
        validity: Duration::from_secs(2),
        context: None,
    });
    engine
        .set_request_resolver(adapter.clone())
        .expect("inject");
    let input = request(&server, output.path());
    let mut selection = options();
    selection.mode = DownloadResolutionMode::Receipt(Uuid::new_v4());
    assert!(matches!(
        engine.submit_with_resolution(
            input.clone(),
            RequestOrigin::Ui,
            DownloadSubmitMode::QueueOnly,
            None,
            selection,
            DownloadResolutionControl::default()
        ),
        Err(DownloadEngineError::Resolution(
            DownloadResolutionError::ResolutionRequired
        ))
    ));
    let mut selection = options();
    let token = probe(&engine, input.clone(), selection.clone())
        .resolution
        .expect("receipt")
        .receipt;
    selection.mode = DownloadResolutionMode::Receipt(token);
    let mut wrong = selection.clone();
    wrong.operation_id = Uuid::new_v4();
    assert!(
        engine
            .submit_with_resolution(
                input.clone(),
                RequestOrigin::Ui,
                DownloadSubmitMode::QueueOnly,
                None,
                wrong,
                DownloadResolutionControl::default()
            )
            .is_err()
    );
    let mut changed = input.clone();
    changed.cookie = Some("different=1".into());
    assert!(
        engine
            .submit_with_resolution(
                changed,
                RequestOrigin::Ui,
                DownloadSubmitMode::QueueOnly,
                None,
                selection.clone(),
                DownloadResolutionControl::default()
            )
            .is_err()
    );
    let other_session = DownloadEngine::open_in_memory().expect("other engine session");
    other_session
        .set_request_resolver(adapter.clone())
        .expect("other inject");
    assert!(
        other_session
            .submit_with_resolution(
                input.clone(),
                RequestOrigin::Ui,
                DownloadSubmitMode::QueueOnly,
                None,
                selection.clone(),
                DownloadResolutionControl::default()
            )
            .is_err()
    );
    std::thread::sleep(Duration::from_millis(2020));
    assert!(
        engine
            .submit_with_resolution(
                input.clone(),
                RequestOrigin::Ui,
                DownloadSubmitMode::QueueOnly,
                None,
                selection,
                DownloadResolutionControl::default()
            )
            .is_err()
    );
    let mut selection = options();
    let token = probe(&engine, input.clone(), selection.clone())
        .resolution
        .expect("receipt")
        .receipt;
    selection.mode = DownloadResolutionMode::Receipt(token);
    engine
        .set_request_resolver(adapter.clone())
        .expect("invalidate session approval");
    assert!(
        engine
            .submit_with_resolution(
                input,
                RequestOrigin::Ui,
                DownloadSubmitMode::QueueOnly,
                None,
                selection,
                DownloadResolutionControl::default()
            )
            .is_err()
    );
    assert_eq!(adapter.calls.load(Ordering::SeqCst), 2);
    assert!(engine.tasks().expect("tasks").is_empty());
}

#[test]
fn resolution_cross_origin_drops_all_inherited_context_and_old_media() {
    let first = DeterministicHttpServer::start();
    let second = DeterministicHttpServer::start();
    let output = tempfile::tempdir().expect("output");
    let engine = DownloadEngine::open_in_memory().expect("engine");
    let settings = nxtools_core::DownloadSettings {
        default_request_headers: "X-Global: secret".into(),
        default_cookie: "global=secret".into(),
        ..Default::default()
    };
    engine
        .update_download_http_policy(&settings)
        .expect("policy");
    let adapter = resolver(format!("http://{}/range", second.address()));
    engine.set_request_resolver(adapter).expect("inject");
    let mut input = request(&first, output.path());
    input.headers = vec![
        DownloadRequestHeader {
            name: "Authorization".into(),
            value: "private".into(),
        },
        DownloadRequestHeader {
            name: "X-Custom-Secret".into(),
            value: "private".into(),
        },
    ];
    input.cookie = Some("private=1".into());
    input.referrer = Some(input.source_url.clone());
    input.media = Some(DownloadRequestMedia::Hls(HlsDownloadRequest {
        playlist_url: input.source_url.clone(),
    }));
    input.expected_total_bytes = Some(999);
    let id = engine
        .enqueue(input, RequestOrigin::Browser)
        .expect("enqueue");
    let task = engine.task(id).expect("task").expect("exists").task;
    assert!(task.request().headers.is_empty());
    assert!(task.request().cookie.is_none());
    assert!(task.request().referrer.is_none());
    assert!(task.request().media.is_none());
    assert!(task.request().expected_total_bytes.is_none());
    let explicit = Arc::new(Resolver {
        calls: AtomicUsize::new(0),
        target: format!("http://{}/range", second.address()),
        fail: false,
        delay: Duration::ZERO,
        validity: Duration::from_secs(600),
        context: Some(DownloadResolvedContext {
            headers: vec![DownloadRequestHeader {
                name: "X-New".into(),
                value: "intentional".into(),
            }],
            cookie: Some("new=1".into()),
            referrer: None,
        }),
    });
    engine
        .set_request_resolver(explicit)
        .expect("inject explicit context");
    let mut input = request(&first, output.path());
    input.file_name = Some("explicit.bin".into());
    let id = engine
        .enqueue(input, RequestOrigin::Ui)
        .expect("enqueue explicit");
    let task = engine.task(id).expect("task").expect("exists").task;
    assert_eq!(task.request().headers.len(), 1);
    assert_eq!(task.request().headers[0].name, "X-New");
    assert_eq!(task.request().cookie.as_deref(), Some("new=1"));
}

#[test]
fn resolution_cancel_does_not_hold_scheduler_or_persist_task() {
    let server = DeterministicHttpServer::start();
    let output = tempfile::tempdir().expect("output");
    let engine = DownloadEngine::open_in_memory().expect("engine");
    let adapter = Arc::new(Resolver {
        calls: AtomicUsize::new(0),
        target: format!("http://{}/range", server.address()),
        fail: false,
        delay: Duration::from_secs(30),
        validity: Duration::from_secs(600),
        context: None,
    });
    engine
        .set_request_resolver(adapter.clone())
        .expect("inject");
    let control = DownloadResolutionControl::default();
    let worker_engine = engine.clone();
    let worker_control = control.clone();
    let input = request(&server, output.path());
    let worker = std::thread::spawn(move || {
        worker_engine.submit_with_resolution(
            input,
            RequestOrigin::Ui,
            DownloadSubmitMode::QueueOnly,
            None,
            options(),
            worker_control,
        )
    });
    let until = Instant::now() + Duration::from_secs(2);
    while adapter.calls.load(Ordering::SeqCst) == 0 {
        assert!(Instant::now() < until, "resolver started");
        std::thread::sleep(Duration::from_millis(5));
    }
    let started = Instant::now();
    engine
        .set_global_speed_limit(Some(1024))
        .expect("independent scheduler operation");
    engine.tasks().expect("responsive snapshot");
    assert!(started.elapsed() < Duration::from_secs(1));
    control.cancel();
    assert!(matches!(
        worker.join().expect("join"),
        Err(DownloadEngineError::Resolution(
            DownloadResolutionError::Cancelled
        ))
    ));
    assert!(engine.tasks().expect("tasks").is_empty());
}

#[test]
fn resolution_direct_submit_transfers_effective_artifact_without_rerun() {
    let server = DeterministicHttpServer::start();
    let output = tempfile::tempdir().expect("output");
    let engine = DownloadEngine::open(output.path().join("downloads.sqlite3")).expect("engine");
    let adapter = resolver(format!("http://{}/range", server.address()));
    engine
        .set_request_resolver(adapter.clone())
        .expect("inject");
    let mut input = request(&server, output.path());
    input.source_url = format!("http://{}/page", server.address());
    let id = engine
        .submit(input, RequestOrigin::Internal)
        .expect("submit");
    let runtime = tokio::runtime::Runtime::new().expect("fixture runtime");
    let terminal = runtime
        .block_on(engine.wait_for_terminal(id, Duration::from_secs(10)))
        .expect("terminal");
    assert_eq!(terminal.task.status(), DownloadStatus::Completed);
    assert_eq!(
        std::fs::read(output.path().join("chosen.bin")).expect("artifact"),
        b"0123456789abcdef"
    );
    assert_eq!(adapter.calls.load(Ordering::SeqCst), 1);
}

#[test]
fn resolution_blob_import_bypasses_even_a_failing_resolver() {
    let output = tempfile::tempdir().expect("output");
    let engine = DownloadEngine::open(output.path().join("downloads.sqlite3")).expect("engine");
    let adapter = Arc::new(Resolver {
        calls: AtomicUsize::new(0),
        target: String::new(),
        fail: true,
        delay: Duration::ZERO,
        validity: Duration::from_secs(600),
        context: None,
    });
    engine
        .set_request_resolver(adapter.clone())
        .expect("inject");
    let blob_id = Uuid::new_v4();
    let staging = output.path().join("browser-blob-staging");
    std::fs::create_dir_all(&staging).expect("staging");
    std::fs::write(staging.join(format!("{blob_id}.blob")), b"owned-blob").expect("stage blob");
    let command = DownloadCommandEnvelope {
        protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
        request_id: Uuid::new_v4(),
        origin: RequestOrigin::Browser,
        command: DownloadCommand::BrowserImportBlob {
            handoff_id: Uuid::new_v4(),
            blob: BrowserBlob {
                id: blob_id,
                file_name: "blob.bin".into(),
                media_type: None,
                size: 10,
            },
            save_directory: output.path().to_owned(),
            file_name: "blob.bin".into(),
            category_id: None,
            save_directory_explicit: true,
        },
    };
    let id = match engine.handle_command(command).outcome {
        DownloadCommandOutcome::BrowserAccepted {
            task_id,
            duplicate: false,
        } => task_id,
        _ => panic!("Blob must be accepted without resolver"),
    };
    assert_eq!(
        engine
            .task(id)
            .expect("task")
            .expect("exists")
            .task
            .status(),
        DownloadStatus::Completed
    );
    assert_eq!(
        std::fs::read(output.path().join("blob.bin")).expect("artifact"),
        b"owned-blob"
    );
    assert_eq!(adapter.calls.load(Ordering::SeqCst), 0);
}

#[test]
fn resolution_invalid_scheme_or_context_never_creates_task() {
    let server = DeterministicHttpServer::start();
    let output = tempfile::tempdir().expect("output");
    let engine = DownloadEngine::open_in_memory().expect("engine");
    engine
        .set_request_resolver(resolver("file:///not-http".into()))
        .expect("inject");
    assert!(matches!(
        engine.enqueue(request(&server, output.path()), RequestOrigin::Ui),
        Err(DownloadEngineError::Resolution(
            DownloadResolutionError::InvalidResult
        ))
    ));
    let adapter = Arc::new(Resolver {
        calls: AtomicUsize::new(0),
        target: format!("http://{}/range", server.address()),
        fail: false,
        delay: Duration::ZERO,
        validity: Duration::from_secs(600),
        context: Some(DownloadResolvedContext {
            headers: vec![DownloadRequestHeader {
                name: "X-New".into(),
                value: "bad\r\nvalue".into(),
            }],
            cookie: None,
            referrer: None,
        }),
    });
    engine
        .set_request_resolver(adapter)
        .expect("inject invalid context");
    assert!(matches!(
        engine.enqueue(request(&server, output.path()), RequestOrigin::Ui),
        Err(DownloadEngineError::Resolution(
            DownloadResolutionError::InvalidResult
        ))
    ));
    assert!(engine.tasks().expect("tasks").is_empty());
}

#[test]
fn resolution_receipt_cache_is_bounded_and_eviction_requires_explicit_retry() {
    let server = DeterministicHttpServer::start();
    let output = tempfile::tempdir().expect("output");
    let engine = DownloadEngine::open_in_memory().expect("engine");
    let adapter = resolver(format!("http://{}/range", server.address()));
    engine
        .set_request_resolver(adapter.clone())
        .expect("inject");
    let input = request(&server, output.path());
    let mut first = options();
    first.mode = DownloadResolutionMode::Receipt(
        probe(&engine, input.clone(), first.clone())
            .resolution
            .expect("receipt")
            .receipt,
    );
    for _ in 0..MAX_RESOLUTION_RECEIPTS {
        probe(&engine, input.clone(), options());
    }
    assert!(matches!(
        engine.submit_with_resolution(
            input,
            RequestOrigin::Ui,
            DownloadSubmitMode::QueueOnly,
            None,
            first,
            DownloadResolutionControl::default()
        ),
        Err(DownloadEngineError::Resolution(
            DownloadResolutionError::ResolutionRequired
        ))
    ));
    assert_eq!(
        adapter.calls.load(Ordering::SeqCst),
        MAX_RESOLUTION_RECEIPTS + 1
    );
    assert!(engine.tasks().expect("tasks").is_empty());
}

#[test]
fn resolution_whole_operation_deadline_returns_without_task() {
    let server = DeterministicHttpServer::start();
    let output = tempfile::tempdir().expect("output");
    let engine = DownloadEngine::open_in_memory().expect("engine");
    let adapter = Arc::new(Resolver {
        calls: AtomicUsize::new(0),
        target: format!("http://{}/range", server.address()),
        fail: false,
        delay: Duration::from_secs(30),
        validity: Duration::from_secs(600),
        context: None,
    });
    engine.set_request_resolver(adapter).expect("inject");
    let started = Instant::now();
    let result = engine.submit_with_resolution(
        request(&server, output.path()),
        RequestOrigin::Ui,
        DownloadSubmitMode::QueueOnly,
        None,
        options(),
        DownloadResolutionControl::with_timeout(Duration::from_millis(50)),
    );
    assert!(matches!(
        result,
        Err(DownloadEngineError::Resolution(
            DownloadResolutionError::Deadline
        ))
    ));
    assert!(started.elapsed() < Duration::from_secs(1));
    assert!(engine.tasks().expect("tasks").is_empty());
}

#[test]
fn resolution_revoke_cancels_inflight_and_preserves_no_task() {
    let server = DeterministicHttpServer::start();
    let output = tempfile::tempdir().expect("output");
    let engine = DownloadEngine::open_in_memory().expect("engine");
    let adapter = Arc::new(Resolver {
        calls: AtomicUsize::new(0),
        target: format!("http://{}/range", server.address()),
        fail: false,
        delay: Duration::from_secs(30),
        validity: Duration::from_secs(600),
        context: None,
    });
    engine
        .set_request_resolver(adapter.clone())
        .expect("inject");
    let worker_engine = engine.clone();
    let input = request(&server, output.path());
    let worker = std::thread::spawn(move || worker_engine.enqueue(input, RequestOrigin::Internal));
    let until = Instant::now() + Duration::from_secs(2);
    while adapter.calls.load(Ordering::SeqCst) == 0 {
        assert!(Instant::now() < until, "resolver started");
        std::thread::sleep(Duration::from_millis(5));
    }
    engine
        .set_request_resolver(Arc::new(NoopDownloadRequestResolver))
        .expect("revoke");
    assert!(matches!(
        worker.join().expect("join"),
        Err(DownloadEngineError::Resolution(
            DownloadResolutionError::Cancelled
        ))
    ));
    assert!(engine.tasks().expect("tasks").is_empty());
}

#[test]
fn resolution_receipt_keeps_resource_identity_but_accepts_user_confirmed_save_options() {
    let server = DeterministicHttpServer::start();
    let output = tempfile::tempdir().expect("output");
    let engine = DownloadEngine::open(output.path().join("downloads.db")).expect("engine");
    let adapter = resolver(format!("http://{}/range", server.address()));
    engine
        .set_request_resolver(adapter.clone())
        .expect("inject");
    let mut original = request(&server, output.path());
    original.source_url = format!("http://{}/page", server.address());
    let mut options = options();
    let probe = probe(&engine, original.clone(), options.clone());
    options.mode = DownloadResolutionMode::Receipt(probe.resolution.expect("receipt").receipt);
    let mut confirmed = original.clone();
    confirmed.file_name = Some("user-selected.bin".into());
    confirmed.save_directory = output.path().join("selected");
    confirmed.scheduled_at_unix_ms = Some(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time")
            .as_millis() as i64
            + 60000,
    );
    let (id, _) = engine
        .submit_with_resolution(
            confirmed.clone(),
            RequestOrigin::Ui,
            DownloadSubmitMode::QueueOnly,
            None,
            options,
            DownloadResolutionControl::default(),
        )
        .expect("reuse with confirmed presentation policy");
    let task = engine.task(id).expect("task").expect("exists");
    assert_eq!(task.task.request().file_name, confirmed.file_name);
    assert_eq!(task.task.request().save_directory, confirmed.save_directory);
    assert_eq!(
        task.task.request().scheduled_at_unix_ms,
        confirmed.scheduled_at_unix_ms
    );
    assert!(task.task.request().source_url.ends_with("/range"));
    assert_eq!(adapter.calls.load(Ordering::SeqCst), 1);
}
