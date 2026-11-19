//! Manual queue-order persistence contract: the user's 上移/下移/置顶 arrangement
//! must apply to finished (non-eligible) tasks and survive an engine restart.

use nxtools_download::{
    DOWNLOAD_COMMAND_PROTOCOL_VERSION, DownloadCommand, DownloadCommandEnvelope,
    DownloadCommandOutcome, DownloadEngine, DownloadQueueAction, DownloadQueuePlacement,
    DownloadRequest, DownloadStatus, DownloadTaskAction, RequestOrigin,
};

fn envelope(command: DownloadCommand) -> DownloadCommandEnvelope {
    DownloadCommandEnvelope {
        protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
        request_id: uuid::Uuid::new_v4(),
        origin: RequestOrigin::Ui,
        command,
    }
}

fn fixture_request(index: usize) -> DownloadRequest {
    DownloadRequest {
        source_url: format!("https://example.test/queue-{index}.bin"),
        save_directory: std::env::temp_dir().join("nxtools-queue-order-test"),
        save_directory_explicit: false,
        file_name: Some(format!("queue-{index}.bin")),
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
        duplicate_strategy: nxtools_download::DownloadDuplicateStrategy::UseDefault,
    }
}

#[test]
fn moving_finished_tasks_persists_the_user_order() {
    let directory = tempfile::tempdir().expect("temp directory");
    let db = directory.path().join("downloads.sqlite3");
    let engine = DownloadEngine::open(&db).expect("open engine");

    let mut ids = Vec::new();
    for index in 0..3 {
        let task_id = engine
            .enqueue(fixture_request(index), RequestOrigin::Ui)
            .expect("enqueue fixture");
        // Drive every task to the cancelled terminal state the way a finished
        // download lands in the real queue table (non-eligible).
        engine.handle_command(envelope(DownloadCommand::Task {
            task_id,
            action: DownloadTaskAction::Cancel,
        }));
        let snapshot = engine
            .task(task_id)
            .expect("fixture task lookup succeeds")
            .expect("fixture task exists");
        assert_eq!(snapshot.task.status(), DownloadStatus::Cancelled);
        ids.push(task_id);
    }

    // Move the last task to the top. The old gate rejected this because
    // finished tasks are not admission-eligible.
    let outcome = engine
        .handle_command(envelope(DownloadCommand::Queue {
            action: DownloadQueueAction::Move {
                queue_id: "default".into(),
                task_ids: vec![ids[2]],
                placement: DownloadQueuePlacement::Top,
            },
        }))
        .outcome;
    assert!(
        matches!(outcome, DownloadCommandOutcome::QueueChanged { .. }),
        "moving a finished task must succeed, got {outcome:?}"
    );

    let order = engine.queue_order("default").expect("queue order");
    assert_eq!(order.first(), Some(&ids[2]), "moved task leads the queue");

    // Reopen the engine against the same storage: the user order survives.
    drop(engine);
    let reopened = DownloadEngine::open(&db).expect("reopen engine");
    let restored = reopened.queue_order("default").expect("restored order");
    assert_eq!(
        restored.first(),
        Some(&ids[2]),
        "the persisted user order must survive an engine restart"
    );
    assert_eq!(restored.len(), ids.len(), "no task leaves the queue");
    assert_eq!(restored[1], ids[0]);
    assert_eq!(restored[2], ids[1]);
}
