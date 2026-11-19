mod support;

use nxtools_download::{
    DownloadCertificatePolicy, DownloadCommand, DownloadCommandEnvelope, DownloadCompletionAction,
    DownloadCompletionIntention, DownloadDuplicateStrategy, DownloadEngine, DownloadQueueAction,
    DownloadRequest, DownloadStatus, DownloadSubmitMode, RequestOrigin, SqliteDownloadStore,
};
use uuid::Uuid;

fn request(directory: &std::path::Path, name: &str) -> DownloadRequest {
    DownloadRequest {
        source_url: format!("https://example.invalid/{name}"),
        save_directory: directory.to_path_buf(),
        save_directory_explicit: true,
        file_name: Some(name.to_owned()),
        headers: Vec::new(),
        cookie: None,
        referrer: None,
        source_page: None,
        certificate_policy: DownloadCertificatePolicy::Strict,
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

fn queue_action(engine: &DownloadEngine, action: DownloadQueueAction) {
    let response = engine.handle_command(DownloadCommandEnvelope {
        protocol_version: nxtools_download::DOWNLOAD_COMMAND_PROTOCOL_VERSION,
        request_id: Uuid::new_v4(),
        origin: RequestOrigin::Ui,
        command: DownloadCommand::Queue { action },
    });
    assert!(
        matches!(
            response.outcome,
            nxtools_download::DownloadCommandOutcome::QueueChanged { .. }
        ),
        "{response:?}"
    );
}

#[test]
fn queue_intention_and_assignment_survive_reopen_without_replaying_old_work() {
    let directory = tempfile::tempdir().expect("fixture directory");
    let database = directory.path().join("download.sqlite3");
    let intention = DownloadCompletionIntention {
        action: Some(DownloadCompletionAction::Hibernate),
        open_directory: true,
    };
    let task_id = {
        let engine = DownloadEngine::open(&database).expect("open engine");
        let id = engine
            .enqueue(request(directory.path(), "one.bin"), RequestOrigin::Ui)
            .expect("enqueue");
        assert_eq!(
            engine
                .completion_snapshot()
                .expect("initial snapshot")
                .tasks[0]
                .attempt,
            0
        );
        queue_action(
            &engine,
            DownloadQueueAction::AssignTasks {
                queue_id: "secondary".into(),
                task_ids: vec![id],
            },
        );
        queue_action(
            &engine,
            DownloadQueueAction::SetCompletionIntention {
                queue_id: "secondary".into(),
                intention,
                baseline: None,
            },
        );
        let snapshot = engine.completion_snapshot().expect("assigned snapshot");
        assert_eq!(snapshot.tasks[0].queue_id, "secondary");
        assert_eq!(snapshot.tasks[0].status, DownloadStatus::Created);
        assert_eq!(snapshot.tasks[0].directory, None);
        assert_eq!(
            snapshot
                .queues
                .iter()
                .find(|queue| queue.id == "secondary")
                .expect("queue")
                .completion_intention,
            intention
        );
        engine.shutdown_and_wait().expect("close engine");
        id
    };
    let engine = DownloadEngine::open(&database).expect("reopen");
    let snapshot = engine.completion_snapshot().expect("reopen snapshot");
    assert_eq!(snapshot.tasks[0].task_id, task_id);
    assert_eq!(snapshot.tasks[0].queue_id, "secondary");
    assert_eq!(
        snapshot
            .queues
            .iter()
            .find(|queue| queue.id == "secondary")
            .expect("queue")
            .completion_intention,
        intention
    );
    assert_ne!(snapshot.generation, 0);
    assert_eq!(snapshot.work_revision, 0);
    engine.shutdown_and_wait().expect("close reopened engine");
}

#[test]
fn duplicate_receipt_does_not_increment_accepted_work_and_draining_rejects_new_admission() {
    let directory = tempfile::tempdir().expect("fixture directory");
    let engine = DownloadEngine::open_in_memory().expect("open engine");
    let key = Uuid::new_v4();
    let (id, duplicate) = engine
        .submit_idempotent(
            key,
            request(directory.path(), "one.bin"),
            RequestOrigin::Ui,
            DownloadSubmitMode::QueueOnly,
        )
        .expect("accept");
    assert!(!duplicate);
    let first = engine.completion_snapshot().expect("snapshot");
    assert_eq!(first.work_revision, 1);
    let (replayed, duplicate) = engine
        .submit_idempotent(
            key,
            request(directory.path(), "one.bin"),
            RequestOrigin::Ui,
            DownloadSubmitMode::QueueOnly,
        )
        .expect("replay");
    assert_eq!(id, replayed);
    assert!(duplicate);
    assert_eq!(
        engine
            .completion_snapshot()
            .expect("replayed snapshot")
            .work_revision,
        first.work_revision
    );
    engine.shutdown_and_wait().expect("drain");
    assert!(
        engine
            .enqueue(request(directory.path(), "two.bin"), RequestOrigin::Ui)
            .is_err()
    );
    assert_eq!(
        engine
            .completion_snapshot()
            .expect("drained snapshot")
            .work_revision,
        first.work_revision
    );
}

#[test]
fn queue_intention_failed_save_leaves_previous_value() {
    let directory = tempfile::tempdir().expect("fixture directory");
    let engine =
        DownloadEngine::open(directory.path().join("download.sqlite3")).expect("open engine");
    let connection = rusqlite::Connection::open(directory.path().join("download.sqlite3"))
        .expect("open fixture connection");
    connection.execute_batch("CREATE TRIGGER reject_intention BEFORE UPDATE OF completion_intention_json ON download_queues BEGIN SELECT RAISE(ABORT, 'blocked'); END;").expect("block save");
    let outcome = engine.handle_command(DownloadCommandEnvelope {
        protocol_version: nxtools_download::DOWNLOAD_COMMAND_PROTOCOL_VERSION,
        request_id: Uuid::new_v4(),
        origin: RequestOrigin::Ui,
        command: DownloadCommand::Queue {
            action: DownloadQueueAction::SetCompletionIntention {
                queue_id: "default".into(),
                intention: DownloadCompletionIntention {
                    action: Some(DownloadCompletionAction::Sleep),
                    open_directory: true,
                },
                baseline: None,
            },
        },
    });
    assert!(matches!(
        outcome.outcome,
        nxtools_download::DownloadCommandOutcome::Rejected { .. }
    ));
    assert_eq!(
        engine.completion_snapshot().expect("snapshot").queues[0].completion_intention,
        DownloadCompletionIntention::default()
    );
    engine.shutdown_and_wait().expect("close");
}

#[test]
fn missing_intention_and_attempt_metadata_default_in_original_store() {
    let directory = tempfile::tempdir().expect("fixture directory");
    let store =
        SqliteDownloadStore::open(directory.path().join("downloads.sqlite3")).expect("open store");
    let task = nxtools_download::DownloadTask::new(
        request(directory.path(), "one.bin"),
        RequestOrigin::Ui,
        1000,
    );
    store.save_task(&task).expect("save task");
    let restored = store.load_tasks().expect("read task");
    assert_eq!(
        restored[0].completion_intention(),
        DownloadCompletionIntention::default()
    );
    assert_eq!(restored[0].completion_attempt(), 0);
    assert_eq!(
        store.load_queues().expect("queues")[0].completion_intention,
        DownloadCompletionIntention::default()
    );
}

#[test]
fn live_http_worker_drain_preserves_task_and_rejects_postdrain_admission() {
    let directory = tempfile::tempdir().expect("fixture directory");
    let server = std::sync::Arc::new(support::DeterministicHttpServer::start_concurrent());
    server.set_fallback_stream_failures(1);
    server.set_fallback_stream_hold(true);
    let mut input = request(directory.path(), "writer.bin");
    input.source_url = format!("http://{}/range-probe-only", server.address());
    let engine = DownloadEngine::open_in_memory().expect("open engine");
    let id = engine
        .submit(input, RequestOrigin::Ui)
        .expect("start HTTP worker");
    let until = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while server.fallback_stream_attempts() == 0 && std::time::Instant::now() < until {
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(
        server.fallback_stream_attempts() > 0,
        "worker reached real HTTP fixture"
    );
    let release_server = std::sync::Arc::clone(&server);
    let release = std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(100));
        release_server.set_fallback_stream_hold(false);
    });
    engine.shutdown_and_wait().expect("join actual worker");
    release.join().expect("release fixture");
    let task = engine.task(id).expect("query task").expect("retained task");
    assert_ne!(task.task.status(), DownloadStatus::Cancelled);
    assert_ne!(task.task.status(), DownloadStatus::Failed);
    assert_eq!(engine.queue_order("default").expect("membership"), vec![id]);
    assert!(
        engine
            .enqueue(request(directory.path(), "late.bin"), RequestOrigin::Ui)
            .is_err()
    );
}

#[tokio::test]
async fn completed_http_writer_projects_durable_attempt_and_actual_final_directory() {
    let directory = tempfile::tempdir().expect("fixture directory");
    let server = support::DeterministicHttpServer::start();
    let mut input = request(directory.path(), "complete.bin");
    input.source_url = format!("http://{}/range", server.address());
    let engine = DownloadEngine::open_in_memory().expect("open engine");
    let id = engine
        .submit(input, RequestOrigin::Ui)
        .expect("submit to original worker");
    let terminal = engine
        .wait_for_terminal(id, std::time::Duration::from_secs(10))
        .await
        .expect("wait for writer");
    assert_eq!(terminal.task.status(), DownloadStatus::Completed);
    let snapshot = engine.completion_snapshot().expect("terminal snapshot");
    let member = snapshot
        .tasks
        .iter()
        .find(|task| task.task_id == id)
        .expect("queue member");
    assert_eq!(member.attempt, 1);
    assert_eq!(member.directory.as_deref(), Some(directory.path()));
    assert!(snapshot.work_revision >= 2);
    engine.shutdown_and_wait().expect("join worker");
}

#[test]
fn task_intention_is_durable_in_original_request_metadata() {
    let directory = tempfile::tempdir().expect("fixture directory");
    let database = directory.path().join("downloads.sqlite3");
    let desired = DownloadCompletionIntention {
        action: Some(DownloadCompletionAction::ExitApplication),
        open_directory: true,
    };
    let id = {
        let engine = DownloadEngine::open(&database).expect("open engine");
        let id = engine
            .enqueue(request(directory.path(), "task.bin"), RequestOrigin::Ui)
            .expect("enqueue");
        let outcome = engine.handle_command(DownloadCommandEnvelope {
            protocol_version: nxtools_download::DOWNLOAD_COMMAND_PROTOCOL_VERSION,
            request_id: Uuid::new_v4(),
            origin: RequestOrigin::Ui,
            command: DownloadCommand::Task {
                task_id: id,
                action: nxtools_download::DownloadTaskAction::SetCompletionIntention {
                    intention: desired,
                },
            },
        });
        assert!(matches!(
            outcome.outcome,
            nxtools_download::DownloadCommandOutcome::Accepted { .. }
        ));
        assert_eq!(
            engine.completion_snapshot().expect("snapshot").tasks[0].intention,
            desired
        );
        engine.shutdown_and_wait().expect("close");
        id
    };
    let engine = DownloadEngine::open(&database).expect("reopen");
    let task = engine
        .completion_snapshot()
        .expect("restored snapshot")
        .tasks
        .into_iter()
        .find(|task| task.task_id == id)
        .expect("restored task");
    assert_eq!(task.intention, desired);
    assert_eq!(task.attempt, 0);
    engine.shutdown_and_wait().expect("close reopened engine");
}

#[test]
fn queue_draft_merges_only_changed_leaves_inside_original_store() {
    let engine = DownloadEngine::open_in_memory().expect("original engine");
    queue_action(
        &engine,
        DownloadQueueAction::SetCompletionIntention {
            queue_id: "default".into(),
            intention: DownloadCompletionIntention {
                action: None,
                open_directory: true,
            },
            baseline: None,
        },
    );
    let action:DownloadQueueAction=serde_json::from_value(serde_json::json!({"action":"set_completion_intention","data":{"queue_id":"default","baseline":{"action":null,"open_directory":false},"intention":{"action":"sleep","open_directory":false}}})).expect("original command/default-compatible baseline");
    queue_action(&engine, action);
    assert_eq!(
        engine.completion_snapshot().expect("snapshot").queues[0].completion_intention,
        DownloadCompletionIntention {
            action: Some(DownloadCompletionAction::Sleep),
            open_directory: true
        },
        "a stale draft must retain concurrently changed directory feedback"
    );
    engine.shutdown_and_wait().expect("drain");
}

#[test]
fn b5_queue_mutations_invalidate_original_completion_subscribers() {
    let directory = tempfile::tempdir().expect("fixture");
    let engine = DownloadEngine::open_in_memory().expect("engine");
    let id = engine
        .enqueue(request(directory.path(), "member.bin"), RequestOrigin::Ui)
        .expect("accepted member");
    let events = engine.subscribe_events().expect("original subscriber");
    for action in [
        DownloadQueueAction::SetCompletionIntention {
            queue_id: "default".into(),
            intention: DownloadCompletionIntention {
                action: Some(DownloadCompletionAction::Sleep),
                open_directory: false,
            },
            baseline: None,
        },
        DownloadQueueAction::AssignTasks {
            queue_id: "secondary".into(),
            task_ids: vec![id],
        },
    ] {
        queue_action(&engine, action);
        assert!(
            std::iter::from_fn(|| events.try_recv().ok()).any(|event| matches!(
                event.event,
                nxtools_download::DownloadEvent::SnapshotRequired { .. }
            )),
            "committed queue facts must invalidate the original completion subscriber"
        );
    }
    engine.shutdown_and_wait().expect("drain");
}
