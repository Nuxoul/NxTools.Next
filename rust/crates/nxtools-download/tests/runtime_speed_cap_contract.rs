use nxtools_download::{
    DOWNLOAD_COMMAND_PROTOCOL_VERSION, DownloadCommand, DownloadCommandEnvelope,
    DownloadCommandOutcome, DownloadDuplicateStrategy, DownloadEngine, DownloadEngineError,
    DownloadRateLimitError, DownloadRequest, RequestOrigin, SqliteDownloadStore,
};
use uuid::Uuid;

fn apply_runtime_cap(engine: &DownloadEngine, cap: Option<u64>) {
    engine
        .set_runtime_speed_cap(cap, Uuid::new_v4())
        .expect("apply temporary runtime cap");
}

#[test]
fn runtime_cap_uses_minimum_enabled_global_limit_without_persisting_policy() {
    let directory = tempfile::tempdir().expect("create engine directory");
    let database_path = directory.path().join("downloads.sqlite3");
    let engine = DownloadEngine::open(&database_path).expect("open engine");
    let store = SqliteDownloadStore::open(&database_path).expect("open storage observer");

    for (manual, runtime, effective) in [
        (None, None, None),
        (None, Some(200), Some(200)),
        (Some(100), Some(200), Some(100)),
        (Some(400), Some(200), Some(200)),
        (Some(200), Some(200), Some(200)),
        (Some(400), None, Some(400)),
        (None, None, None),
        (Some(u64::MAX), Some(1), Some(1)),
    ] {
        engine
            .set_global_speed_limit(manual)
            .expect("persist manual limit");
        apply_runtime_cap(&engine, runtime);
        assert_eq!(engine.global_speed_limit(), manual);
        assert_eq!(engine.effective_global_speed_limit(), effective);
        assert_eq!(
            store.load_global_speed_limit().expect("read stored limit"),
            manual
        );
    }
}

#[test]
fn manual_command_changes_while_capped_restore_latest_desired_limit() {
    let directory = tempfile::tempdir().expect("create engine directory");
    let database_path = directory.path().join("downloads.sqlite3");
    let engine = DownloadEngine::open(&database_path).expect("open engine");
    let store = SqliteDownloadStore::open(&database_path).expect("open storage observer");
    let cached_engine = engine.clone();
    apply_runtime_cap(&engine, Some(200));

    for (manual, effective) in [
        (Some(800), Some(200)),
        (Some(100), Some(100)),
        (None, Some(200)),
        (Some(600), Some(200)),
    ] {
        let request_id = Uuid::new_v4();
        let result = cached_engine.handle_command(DownloadCommandEnvelope {
            protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
            request_id,
            origin: RequestOrigin::Ui,
            command: DownloadCommand::SetGlobalSpeedLimit {
                bytes_per_second: manual,
            },
        });
        assert_eq!(result.request_id, request_id);
        assert!(matches!(
            result.outcome,
            DownloadCommandOutcome::Accepted { task_id: None }
        ));
        assert_eq!(cached_engine.global_speed_limit(), manual);
        assert_eq!(engine.global_speed_limit(), manual);
        assert_eq!(engine.effective_global_speed_limit(), effective);
        assert_eq!(
            store.load_global_speed_limit().expect("read stored limit"),
            manual
        );
    }

    apply_runtime_cap(&cached_engine, None);
    assert_eq!(engine.global_speed_limit(), Some(600));
    assert_eq!(engine.effective_global_speed_limit(), Some(600));
    drop(cached_engine);
    drop(engine);
    let reopened = DownloadEngine::open(&database_path).expect("reopen engine");
    assert_eq!(reopened.global_speed_limit(), Some(600));
    assert_eq!(reopened.effective_global_speed_limit(), Some(600));
}

#[test]
fn restarting_with_active_runtime_cap_restores_only_manual_limit() {
    let directory = tempfile::tempdir().expect("create engine directory");
    let database_path = directory.path().join("downloads.sqlite3");

    for manual in [None, Some(900)] {
        let engine = DownloadEngine::open(&database_path).expect("open engine");
        engine
            .set_global_speed_limit(manual)
            .expect("save manual limit");
        apply_runtime_cap(&engine, Some(100));
        apply_runtime_cap(&engine, Some(100));
        assert_eq!(engine.global_speed_limit(), manual);
        assert_eq!(engine.effective_global_speed_limit(), Some(100));
        drop(engine);

        let reopened = DownloadEngine::open(&database_path).expect("reopen engine");
        assert_eq!(reopened.global_speed_limit(), manual);
        assert_eq!(reopened.effective_global_speed_limit(), manual);
    }
}

#[test]
fn runtime_cap_never_mutates_task_caps_in_memory_or_storage() {
    let directory = tempfile::tempdir().expect("create engine directory");
    let database_path = directory.path().join("downloads.sqlite3");
    let engine = DownloadEngine::open(&database_path).expect("open engine");
    engine
        .set_global_speed_limit(Some(800))
        .expect("save manual limit");
    // Queue-only requests never probe or contact these synthetic addresses.
    let mut task_ids = Vec::new();
    for (index, task_cap) in [None, Some(50), Some(500)].into_iter().enumerate() {
        let task_id = engine
            .enqueue(
                DownloadRequest {
                    source_url: format!("https://example.test/runtime-cap-{index}.bin"),
                    save_directory: directory.path().join("artifacts"),
                    save_directory_explicit: false,
                    file_name: Some(format!("runtime-cap-{index}.bin")),
                    headers: Vec::new(),
                    cookie: None,
                    referrer: None,
                    certificate_policy: Default::default(),
                    connection_limit: Some(1),
                    speed_limit_bytes_per_second: task_cap,
                    expected_total_bytes: None,
                    scheduled_at_unix_ms: None,
                    adaptive_segment_policy: None,
                    media: None,
                    category_id: None,
                    category_name: None,
                    duplicate_strategy: DownloadDuplicateStrategy::UseDefault,
                },
                RequestOrigin::Ui,
            )
            .expect("enqueue task without starting network work");
        task_ids.push((task_id, task_cap));
    }
    let store = SqliteDownloadStore::open(&database_path).expect("open storage observer");

    for runtime in [Some(100), Some(1), None, Some(200)] {
        apply_runtime_cap(&engine, runtime);
        assert_eq!(engine.effective_global_speed_limit(), runtime.or(Some(800)));
        for &(task_id, task_cap) in &task_ids {
            let task = engine
                .task(task_id)
                .expect("read task")
                .expect("task exists");
            assert_eq!(task.task.request().speed_limit_bytes_per_second, task_cap);
            let stored = store
                .load_task(task_id)
                .expect("read persisted task")
                .expect("task stored");
            assert_eq!(stored.request().speed_limit_bytes_per_second, task_cap);
        }
    }
    drop(engine);

    let reopened = DownloadEngine::open(&database_path).expect("reopen engine");
    assert_eq!(reopened.effective_global_speed_limit(), Some(800));
    for (task_id, task_cap) in task_ids {
        let task = reopened
            .task(task_id)
            .expect("read restored task")
            .expect("task restored");
        assert_eq!(task.task.request().speed_limit_bytes_per_second, task_cap);
    }
}

#[test]
fn zero_runtime_or_manual_limit_is_rejected_without_changing_state() {
    let directory = tempfile::tempdir().expect("create engine directory");
    let database_path = directory.path().join("downloads.sqlite3");
    let engine = DownloadEngine::open(&database_path).expect("open engine");
    engine
        .set_global_speed_limit(Some(800))
        .expect("save manual limit");
    apply_runtime_cap(&engine, Some(200));

    assert!(matches!(
        engine.set_runtime_speed_cap(Some(0), Uuid::new_v4()),
        Err(DownloadEngineError::RateLimit(
            DownloadRateLimitError::ZeroLimit
        ))
    ));
    assert!(matches!(
        engine.set_global_speed_limit(Some(0)),
        Err(DownloadEngineError::RateLimit(
            DownloadRateLimitError::ZeroLimit
        ))
    ));
    assert_eq!(engine.global_speed_limit(), Some(800));
    assert_eq!(engine.effective_global_speed_limit(), Some(200));
    let store = SqliteDownloadStore::open(&database_path).expect("open storage observer");
    assert_eq!(
        store.load_global_speed_limit().expect("read stored limit"),
        Some(800)
    );
    apply_runtime_cap(&engine, None);
    assert_eq!(engine.effective_global_speed_limit(), Some(800));
}

#[test]
fn failed_manual_persistence_preserves_cached_manual_and_runtime_limit() {
    let directory = tempfile::tempdir().expect("create engine directory");
    let database_path = directory.path().join("downloads.sqlite3");
    let engine = DownloadEngine::open(&database_path).expect("open engine");
    engine
        .set_global_speed_limit(Some(800))
        .expect("save manual limit");
    apply_runtime_cap(&engine, Some(200));
    let connection = rusqlite::Connection::open(&database_path).expect("open failure injector");
    connection
        .execute_batch(
            "CREATE TRIGGER reject_global_limit_update BEFORE UPDATE ON download_settings
             BEGIN SELECT RAISE(ABORT, 'synthetic settings failure'); END;",
        )
        .expect("install storage failure trigger");

    assert!(engine.set_global_speed_limit(Some(50)).is_err());
    assert_eq!(engine.global_speed_limit(), Some(800));
    assert_eq!(engine.effective_global_speed_limit(), Some(200));
    // Runtime policy has no storage dependency, even while manual writes fail.
    apply_runtime_cap(&engine, Some(100));
    apply_runtime_cap(&engine, None);
    assert_eq!(engine.effective_global_speed_limit(), Some(800));
    let store = SqliteDownloadStore::open(&database_path).expect("open storage observer");
    assert_eq!(
        store.load_global_speed_limit().expect("read stored limit"),
        Some(800)
    );
}

#[test]
fn concurrent_manual_and_policy_updates_share_one_serialized_state() {
    let directory = tempfile::tempdir().expect("create engine directory");
    let database_path = directory.path().join("downloads.sqlite3");
    let engine = DownloadEngine::open(&database_path).expect("open engine");
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(3));
    let manual_engine = engine.clone();
    let manual_barrier = barrier.clone();
    let manual_thread = std::thread::spawn(move || {
        manual_barrier.wait();
        for manual in 1..=32 {
            manual_engine
                .set_global_speed_limit(Some(manual))
                .expect("save concurrent manual");
        }
    });
    let policy_engine = engine.clone();
    let policy_barrier = barrier.clone();
    let policy_thread = std::thread::spawn(move || {
        policy_barrier.wait();
        for runtime in 1..=32 {
            apply_runtime_cap(&policy_engine, Some(runtime));
            apply_runtime_cap(&policy_engine, None);
        }
        apply_runtime_cap(&policy_engine, Some(7));
    });
    barrier.wait();
    manual_thread.join().expect("join manual updates");
    policy_thread.join().expect("join policy updates");
    let store = SqliteDownloadStore::open(&database_path).expect("open storage observer");
    assert_eq!(
        store.load_global_speed_limit().expect("read stored limit"),
        Some(32)
    );
    assert_eq!(engine.global_speed_limit(), Some(32));
    assert_eq!(engine.effective_global_speed_limit(), Some(7));
    apply_runtime_cap(&engine, None);
    assert_eq!(engine.effective_global_speed_limit(), Some(32));
}

#[derive(Clone)]
struct SpeedLimitDiagnostics(std::sync::Arc<std::sync::Mutex<Vec<String>>>);

#[derive(Default)]
struct EventNameVisitor(Option<String>);

impl tracing::field::Visit for EventNameVisitor {
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "event" {
            self.0 = Some(value.to_owned());
        }
    }

    fn record_debug(&mut self, _field: &tracing::field::Field, _value: &dyn std::fmt::Debug) {}
}

impl tracing::Subscriber for SpeedLimitDiagnostics {
    fn enabled(&self, _metadata: &tracing::Metadata<'_>) -> bool {
        true
    }

    fn new_span(&self, _attributes: &tracing::span::Attributes<'_>) -> tracing::span::Id {
        tracing::span::Id::from_u64(1)
    }

    fn record(&self, _span: &tracing::span::Id, _values: &tracing::span::Record<'_>) {}

    fn record_follows_from(&self, _span: &tracing::span::Id, _follows: &tracing::span::Id) {}

    fn event(&self, event: &tracing::Event<'_>) {
        let mut visitor = EventNameVisitor::default();
        event.record(&mut visitor);
        if let Some(name) = visitor.0
            && name.starts_with("download.speed_limit.")
        {
            self.0.lock().expect("lock diagnostic events").push(name);
        }
    }

    fn enter(&self, _span: &tracing::span::Id) {}

    fn exit(&self, _span: &tracing::span::Id) {}
}

#[test]
fn unchanged_runtime_and_manual_caps_emit_no_diagnostics() {
    let directory = tempfile::tempdir().expect("create engine directory");
    let database_path = directory.path().join("downloads.sqlite3");
    let engine = DownloadEngine::open(&database_path).expect("open engine");
    let diagnostics = SpeedLimitDiagnostics(Default::default());
    let events = diagnostics.0.clone();
    tracing::subscriber::with_default(diagnostics, || {
        apply_runtime_cap(&engine, None);
        assert!(
            events
                .lock()
                .expect("read unchanged diagnostics")
                .is_empty()
        );
        engine
            .set_global_speed_limit(Some(800))
            .expect("save manual limit");
        apply_runtime_cap(&engine, Some(100));
        let changed_count = events.lock().expect("count changed diagnostics").len();
        for _ in 0..32 {
            apply_runtime_cap(&engine, Some(100));
            engine
                .set_global_speed_limit(Some(800))
                .expect("repeat manual limit");
        }
        assert_eq!(
            events.lock().expect("count unchanged diagnostics").len(),
            changed_count
        );
        apply_runtime_cap(&engine, None);
        apply_runtime_cap(&engine, None);
    });
    assert_eq!(
        *events.lock().expect("read lifecycle diagnostics"),
        [
            "download.speed_limit.global_changed",
            "download.speed_limit.runtime_started",
            "download.speed_limit.runtime_changed",
            "download.speed_limit.runtime_started",
            "download.speed_limit.runtime_changed",
        ]
    );
    assert_eq!(engine.effective_global_speed_limit(), Some(800));
}
