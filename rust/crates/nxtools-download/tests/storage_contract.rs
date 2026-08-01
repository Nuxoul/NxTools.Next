use std::path::PathBuf;

use nxtools_download::{
    DOWNLOAD_SCHEMA_VERSION, DownloadRequest, DownloadRequestHeader, DownloadSegment,
    DownloadSegmentState, DownloadStatus, DownloadStorageError, DownloadTask, DownloadTaskId,
    RequestOrigin, ResourceValidator, SqliteDownloadStore, TransferPlan, TransferProtocol,
};

fn task(at: i64) -> DownloadTask {
    DownloadTask::new(
        DownloadRequest {
            source_url: "https://example.test/large.iso".to_owned(),
            save_directory: PathBuf::from(r"D:\Downloads"),
            file_name: Some("large.iso".to_owned()),
            headers: vec![DownloadRequestHeader {
                name: "Authorization".to_owned(),
                value: "Bearer test".to_owned(),
            }],
            cookie: Some("session=test".to_owned()),
            referrer: Some("https://example.test/downloads".to_owned()),
            connection_limit: Some(8),
            speed_limit_bytes_per_second: None,
            scheduled_at_unix_ms: Some(5_000),
            adaptive_segment_policy: None,
        },
        RequestOrigin::Browser,
        at,
    )
}

#[test]
fn current_schema_round_trips_a_task_through_the_public_store() {
    let store = SqliteDownloadStore::open_in_memory().expect("open in-memory store");
    assert_eq!(
        store.schema_version().expect("read schema version"),
        DOWNLOAD_SCHEMA_VERSION
    );

    let original = task(1_000);
    store.save_task(&original).expect("persist task");

    let restored = store
        .load_task(original.id())
        .expect("load task")
        .expect("task exists");

    assert_eq!(restored, original);
}

#[test]
fn startup_recovery_persists_interrupted_tasks_as_paused() {
    let store = SqliteDownloadStore::open_in_memory().expect("open in-memory store");
    let mut interrupted = task(1_000);
    for status in [
        DownloadStatus::Probing,
        DownloadStatus::Waiting,
        DownloadStatus::Downloading,
    ] {
        interrupted
            .transition_to(status, 1_100)
            .expect("setup active task");
    }
    store.save_task(&interrupted).expect("persist active task");

    let recovered = store
        .recover_interrupted_tasks(2_000)
        .expect("recover interrupted tasks");
    let restored = store
        .load_task(interrupted.id())
        .expect("load task")
        .expect("task exists");

    assert_eq!(recovered, 1);
    assert_eq!(restored.status(), DownloadStatus::Paused);
    assert_eq!(restored.updated_at_unix_ms(), 2_000);
}

#[test]
fn reopening_a_file_store_preserves_request_context() {
    let temporary = tempfile::tempdir().expect("create temporary directory");
    let database_path = temporary.path().join("downloads.sqlite3");
    let original = task(1_000);

    {
        let store = SqliteDownloadStore::open(&database_path).expect("open file store");
        store.save_task(&original).expect("persist task");
    }

    let reopened = SqliteDownloadStore::open(&database_path).expect("reopen file store");
    let restored = reopened
        .load_task(original.id())
        .expect("load task")
        .expect("task exists");

    assert_eq!(restored.request().headers, original.request().headers);
    assert_eq!(restored.request().cookie, original.request().cookie);
    assert_eq!(restored.request().referrer, original.request().referrer);
}

#[test]
fn newer_schema_is_rejected_before_journal_mode_can_be_changed() {
    let temporary = tempfile::tempdir().expect("create temporary directory");
    let database_path = temporary.path().join("newer.sqlite3");
    let raw = rusqlite::Connection::open(&database_path).expect("create newer database");
    let newer_version = DOWNLOAD_SCHEMA_VERSION + 1;
    raw.execute_batch(&format!(
        "PRAGMA journal_mode = DELETE; PRAGMA user_version = {newer_version};"
    ))
    .expect("mark database as newer");
    drop(raw);

    let error = match SqliteDownloadStore::open(&database_path) {
        Ok(_) => panic!("newer schema must be rejected"),
        Err(error) => error,
    };
    assert!(matches!(
        error,
        DownloadStorageError::UnsupportedSchema {
            current,
            supported: DOWNLOAD_SCHEMA_VERSION,
        } if current == newer_version
    ));

    let raw = rusqlite::Connection::open(&database_path).expect("reopen raw database");
    let journal_mode: String = raw
        .query_row("PRAGMA journal_mode", [], |row| row.get(0))
        .expect("read journal mode");
    assert_eq!(journal_mode, "delete");
}

#[test]
fn schema_v1_migration_backfills_stable_default_queue_without_auto_starting() {
    let temporary = tempfile::tempdir().expect("create temporary directory");
    let database_path = temporary.path().join("v1.sqlite3");
    let first = DownloadTaskId::new();
    let second = DownloadTaskId::new();
    let raw = rusqlite::Connection::open(&database_path).expect("create v1 database");
    raw.execute_batch(
        r#"
        CREATE TABLE download_tasks (
            task_id TEXT PRIMARY KEY NOT NULL,
            created_at_unix_ms INTEGER NOT NULL
        );
        CREATE TABLE download_queues (
            queue_id TEXT PRIMARY KEY NOT NULL,
            name TEXT NOT NULL,
            position INTEGER NOT NULL,
            max_concurrent_tasks INTEGER,
            connection_limit INTEGER,
            speed_limit_bytes_per_second INTEGER,
            schedule_json TEXT,
            is_running INTEGER NOT NULL DEFAULT 1
        );
        PRAGMA user_version = 1;
        "#,
    )
    .expect("create v1 schema boundary");
    raw.execute(
        "INSERT INTO download_tasks(task_id, created_at_unix_ms) VALUES (?1, 200)",
        [second.as_uuid().to_string()],
    )
    .expect("insert later task");
    raw.execute(
        "INSERT INTO download_tasks(task_id, created_at_unix_ms) VALUES (?1, 100)",
        [first.as_uuid().to_string()],
    )
    .expect("insert earlier task");
    drop(raw);

    let store = SqliteDownloadStore::open(&database_path).expect("migrate v1 store");
    assert_eq!(
        store.load_queue_order("default").expect("load queue order"),
        vec![first, second]
    );
    assert!(
        store
            .load_eligible_queue_order("default")
            .expect("load eligible queue order")
            .is_empty(),
        "migration must not unexpectedly start existing tasks"
    );
}

#[test]
fn transfer_plan_and_segment_checkpoints_survive_reopening_the_store() {
    let temporary = tempfile::tempdir().expect("create temporary directory");
    let database_path = temporary.path().join("downloads.sqlite3");
    let original_task = task(1_000);
    let plan = TransferPlan::new(
        TransferProtocol::Http,
        "https://cdn.example.test/large.iso".to_owned(),
        Some(16),
        true,
        ResourceValidator {
            etag: Some("\"fixture-v1\"".to_owned()),
            last_modified: Some("Mon, 27 Jul 2026 00:00:00 GMT".to_owned()),
        },
        vec![
            DownloadSegment::new(0, 0, 7, 5, DownloadSegmentState::Active)
                .expect("valid first segment"),
            DownloadSegment::new(1, 8, 15, 8, DownloadSegmentState::Completed)
                .expect("valid second segment"),
        ],
    )
    .expect("valid transfer plan");

    {
        let store = SqliteDownloadStore::open(&database_path).expect("open file store");
        store.save_task(&original_task).expect("persist task");
        store
            .save_transfer_plan(original_task.id(), &plan)
            .expect("persist transfer plan");
    }

    let reopened = SqliteDownloadStore::open(&database_path).expect("reopen file store");
    let restored = reopened
        .load_transfer_plan(original_task.id())
        .expect("load transfer plan")
        .expect("transfer plan exists");

    assert_eq!(restored, plan);
}

#[test]
fn startup_recovery_keeps_segment_bytes_and_releases_transient_active_state() {
    let store = SqliteDownloadStore::open_in_memory().expect("open store");
    let mut interrupted = task(1_000);
    for status in [
        DownloadStatus::Probing,
        DownloadStatus::Waiting,
        DownloadStatus::Downloading,
    ] {
        interrupted
            .transition_to(status, 1_100)
            .expect("setup active task");
    }
    store.save_task(&interrupted).expect("persist task");
    let plan = TransferPlan::new(
        TransferProtocol::Http,
        interrupted.request().source_url.clone(),
        Some(16),
        true,
        ResourceValidator::default(),
        vec![
            DownloadSegment::new(0, 0, 15, 0, DownloadSegmentState::Active).expect("valid segment"),
        ],
    )
    .expect("valid plan");
    store
        .save_transfer_plan(interrupted.id(), &plan)
        .expect("persist plan");
    store
        .save_segment_checkpoint(
            interrupted.id(),
            &DownloadSegment::new(0, 0, 15, 6, DownloadSegmentState::Active)
                .expect("valid checkpoint"),
        )
        .expect("persist checkpoint");

    store
        .recover_interrupted_tasks(2_000)
        .expect("recover interrupted task");
    let restored = store
        .load_transfer_plan(interrupted.id())
        .expect("load plan")
        .expect("plan exists");

    assert_eq!(restored.segments()[0].downloaded_bytes(), 6);
    assert_eq!(restored.segments()[0].state, DownloadSegmentState::Waiting);
}
