mod support;

#[path = "support/b4_url_cases.rs"]
mod lifecycle;
#[path = "support/b4_url.rs"]
mod url_server;
use std::path::Path;
use std::time::Duration;

use nxtools_download::{
    DOWNLOAD_COMMAND_PROTOCOL_VERSION, DownloadCommand, DownloadCommandEnvelope,
    DownloadCommandOutcome, DownloadDuplicateStrategy, DownloadEngine, DownloadRequest,
    DownloadStatus, DownloadTaskId, RequestOrigin, SqliteDownloadStore,
};
use uuid::Uuid;

fn request(source_url: String, directory: &Path) -> DownloadRequest {
    DownloadRequest {
        source_url,
        save_directory: directory.to_owned(),
        save_directory_explicit: false,
        file_name: Some("retained.bin".into()),
        headers: Vec::new(),
        cookie: None,
        referrer: None,
        source_page: None,
        certificate_policy: Default::default(),
        connection_limit: Some(1),
        speed_limit_bytes_per_second: Some(64 * 1024),
        expected_total_bytes: None,
        scheduled_at_unix_ms: None,
        adaptive_segment_policy: None,
        media: None,
        category_id: None,
        category_name: None,
        duplicate_strategy: DownloadDuplicateStrategy::UseDefault,
    }
}

async fn pause_written(
    engine: &DownloadEngine,
    task_id: DownloadTaskId,
) -> nxtools_download::DownloadEngineTask {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        let task = engine
            .task(task_id)
            .expect("read task")
            .expect("task exists");
        if task.task.status() == DownloadStatus::Downloading
            && task.downloaded_bytes > 0
            && task
                .total_bytes
                .is_some_and(|total| task.downloaded_bytes < total)
        {
            break;
        }
        assert!(
            !matches!(
                task.task.status(),
                DownloadStatus::Completed | DownloadStatus::Failed | DownloadStatus::Finalizing
            ),
            "transfer ended before a partial checkpoint: {:?}",
            task.error
        );
        assert!(
            tokio::time::Instant::now() < deadline,
            "writer must produce partial bytes: {:?}",
            task.error
        );
        tokio::time::sleep(Duration::from_millis(2)).await;
    }
    engine.pause(task_id).expect("explicit pause");
    let paused = engine
        .wait_for_terminal(task_id, Duration::from_secs(5))
        .await
        .expect("writer drain");
    assert_eq!(paused.task.status(), DownloadStatus::Paused);
    assert!(
        paused.downloaded_bytes > 0
            && paused.downloaded_bytes < paused.total_bytes.expect("known total")
    );
    engine
        .set_task_speed_limit(task_id, None)
        .expect("remove checkpoint rate limit before replacement");
    engine
        .task(task_id)
        .expect("updated paused task")
        .expect("same paused identity")
}

fn replacement(
    engine: &DownloadEngine,
    task_id: DownloadTaskId,
    old: &str,
    new: &str,
    operation_id: Uuid,
) -> nxtools_download::DownloadCommandResult {
    engine.handle_command(DownloadCommandEnvelope {
        protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
        request_id: operation_id,
        origin: RequestOrigin::Ui,
        command: DownloadCommand::ReplacePausedUrl(
            nxtools_download::DownloadPausedUrlReplacement {
                task_id,
                operation_id,
                generation: engine.session_generation(),
                expected_revision: engine
                    .paused_url_replacement_revision(task_id)
                    .expect("current attempt revision"),
                expected_source_url: old.to_owned(),
                source_url: new.to_owned(),
            },
        ),
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn b4_ref08_compatible_replacement_preserves_written_plan_and_reopens() {
    let server = url_server::UrlServer::start().await;
    let directory = tempfile::tempdir().expect("owned output");
    let database = directory.path().join("downloads.sqlite3");
    let old = server.url("/old");
    let new = server.url("/new");
    let engine = DownloadEngine::open(&database).expect("original engine");
    let mut original = request(old.clone(), directory.path());
    original.source_page = Some(nxtools_download::DownloadSourcePage {
        url: "https://page.example.test/original".into(),
        title: "Original page".into(),
    });
    let task_id = engine
        .submit(original, RequestOrigin::Ui)
        .expect("original admission");
    let paused = pause_written(&engine, task_id).await;
    let artifact = paused
        .temporary_path
        .clone()
        .expect("owned partial artifact");
    let partial_bytes = std::fs::read(&artifact).expect("written partial file");
    let store = SqliteDownloadStore::open(&database).expect("original SQLite store");
    let before = store
        .load_transfer_plan(task_id)
        .expect("persisted plan")
        .expect("plan exists");
    assert!(
        before
            .segments()
            .iter()
            .any(|segment| segment.downloaded_bytes() > 0)
    );
    assert!(
        store
            .is_queue_task_user_paused(task_id)
            .expect("explicit pause admission")
    );
    assert!(!store.is_queue_task_eligible(task_id).expect("not queued"));

    let operation_id = Uuid::new_v4();
    let result = replacement(&engine, task_id, &old, &new, operation_id);
    assert_eq!(result.request_id, operation_id);
    assert!(
        matches!(result.outcome, DownloadCommandOutcome::Accepted { task_id: Some(id) } if id == task_id),
        "compatible paused replacement must be accepted: {:?}",
        result.outcome
    );
    let accepted = engine
        .task(task_id)
        .expect("accepted task")
        .expect("same identity");
    let mut expected = paused.task.request().clone();
    expected.source_url = new.clone();
    assert_eq!(
        accepted.task.request(),
        &expected,
        "only source changes in retained context"
    );
    assert_eq!(accepted.task.status(), DownloadStatus::Paused);
    assert_eq!(accepted.downloaded_bytes, paused.downloaded_bytes);
    assert_eq!(accepted.temporary_path.as_ref(), Some(&artifact));
    assert_eq!(
        std::fs::read(&artifact).expect("retained artifact"),
        partial_bytes
    );
    let after = store
        .load_transfer_plan(task_id)
        .expect("committed plan")
        .expect("same plan");
    assert_eq!(
        after.segments(),
        before.segments(),
        "segment identity and checkpoints are retained"
    );
    assert_eq!(after.final_url(), new);
    assert_eq!(after.validator(), before.validator());
    assert_eq!(
        store
            .load_task(task_id)
            .expect("durable task")
            .expect("task exists")
            .request(),
        &expected
    );
    assert!(
        matches!(
            replacement(&engine, task_id, &old, &new, operation_id).outcome,
            DownloadCommandOutcome::Rejected { .. }
        ),
        "repeat confirmation cannot publish again"
    );
    drop(store);
    drop(engine);

    let reopened = DownloadEngine::open(&database).expect("reopen original engine");
    let restored = reopened
        .task(task_id)
        .expect("restored task")
        .expect("same task");
    assert_eq!(restored.task.request(), &expected);
    assert_eq!(restored.downloaded_bytes, paused.downloaded_bytes);
    reopened.resume(task_id).expect("explicit resume");
    let completed = reopened
        .wait_for_terminal(task_id, Duration::from_secs(5))
        .await
        .expect("resumed terminal");
    assert_eq!(
        completed.task.status(),
        DownloadStatus::Completed,
        "{:?}",
        completed.error
    );
    assert_eq!(completed.task.id(), task_id);
    assert_eq!(
        std::fs::read(directory.path().join("retained.bin")).expect("final output"),
        url_server::CONTENT
    );
    assert!(!artifact.exists());
    drop(reopened);
    server.close().await;
}
