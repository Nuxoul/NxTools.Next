use super::*;
use nxtools_download::{
    DownloadCommandOutcome, DownloadPausedUrlReplacement, DownloadRequest, DownloadSubmitMode,
    DownloadTaskId, SqliteDownloadStore,
};

#[path = "../../nxtools-download/tests/support/b4_url.rs"]
mod url_fixture;

#[test]
fn b4_e1_draining_pipe_cancels_held_candidate_and_preserves_written_task() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("owned HTTP runtime");
    let server = runtime.block_on(url_fixture::UrlServer::start());
    let directory = tempfile::tempdir().expect("owned downloads");
    let database = directory.path().join("downloads.sqlite3");
    let engine = DownloadEngine::open(&database).expect("original engine");
    let pipe_name = format!(r"\\.\pipe\nxtools-b4-e1-{}", uuid::Uuid::new_v4());
    let activity = DownloadHostActivity::default();
    let host_engine = engine.clone();
    let host_activity = activity.clone();
    let host_name = pipe_name.clone();
    std::thread::spawn(move || {
        run_download_host(&host_name, host_engine, host_activity).expect("original host");
    });
    let mut cleanup = DownloadIpcClient::connect(&pipe_name, Duration::from_secs(3))
        .expect("original cleanup client");
    let mut request: DownloadRequest = serde_json::from_value(serde_json::json!({
        "source_url": server.url("/partial-old"),
        "save_directory": directory.path(),
        "file_name": "retained.bin",
        "connection_limit": 1
    }))
    .expect("ordinary download request");
    request.expected_total_bytes = None;
    let admitted = cleanup
        .command(command_envelope(DownloadCommand::Submit {
            request,
            mode: DownloadSubmitMode::StartNow,
            idempotency_key: None,
        }))
        .expect("original pipe admission");
    let task_id: DownloadTaskId = match admitted.outcome {
        DownloadCommandOutcome::Accepted {
            task_id: Some(task_id),
        } => task_id,
        other => panic!("original admission: {other:?}"),
    };
    let paused = runtime.block_on(async {
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        loop {
            let task = engine.task(task_id).expect("task").expect("identity");
            if task.downloaded_bytes > 0 {
                break;
            }
            assert!(std::time::Instant::now() < deadline, "real partial writer");
            tokio::time::sleep(Duration::from_millis(2)).await;
        }
        engine.pause(task_id).expect("explicit pause");
        engine
            .wait_for_terminal(task_id, Duration::from_secs(5))
            .await
            .expect("writer drain")
    });
    assert_eq!(paused.task.status(), DownloadStatus::Paused);
    assert!(
        paused.downloaded_bytes > 0 && paused.downloaded_bytes < url_fixture::CONTENT.len() as u64
    );
    let store = SqliteDownloadStore::open(&database).expect("original store");
    let plan = store
        .load_transfer_plan(task_id)
        .expect("plan")
        .expect("written plan");
    let artifact = paused.temporary_path.as_ref().expect("artifact");
    let bytes = std::fs::read(artifact).expect("written bytes");
    let detail = cleanup
        .command(command_envelope(DownloadCommand::TaskDetail { task_id }))
        .expect("freshness over original pipe");
    let revision = match detail.outcome {
        DownloadCommandOutcome::TaskDetail(detail) => detail.paused_url_revision,
        other => panic!("original detail: {other:?}"),
    };
    assert_eq!(revision, 0);
    let operation_id = uuid::Uuid::new_v4();
    let replacement = DownloadPausedUrlReplacement {
        task_id,
        operation_id,
        generation: engine.session_generation(),
        expected_revision: revision,
        expected_source_url: paused.task.request().source_url.clone(),
        source_url: server.url("/held"),
    };
    let attempt = command_envelope(DownloadCommand::ReplacePausedUrl(replacement.clone()));
    let attempt_correlation = attempt.request_id;
    let mut candidate = DownloadIpcClient::connect(&pipe_name, Duration::from_secs(3))
        .expect("concurrent original candidate client");
    let (finished, terminal) = std::sync::mpsc::channel();
    let worker = std::thread::spawn(move || {
        let result = candidate
            .command(attempt)
            .expect("candidate terminal over original pipe");
        drop(candidate);
        finished.send(result).expect("terminal observation");
    });
    runtime.block_on(server.wait_held());
    assert!(
        server.hold.load(Ordering::Acquire),
        "fixture has not released candidate"
    );
    assert!(terminal.try_recv().is_err(), "probe is physically held");
    activity.begin_draining();
    assert_eq!(activity.readiness(), DownloadHostReadiness::Draining);
    let requests_before = server.requests.lock().expect("HTTP facts").len();
    let refused = cleanup
        .command(command_envelope(DownloadCommand::ReplacePausedUrl(
            DownloadPausedUrlReplacement {
                operation_id: uuid::Uuid::new_v4(),
                expected_revision: revision + 1,
                source_url: server.url("/same"),
                ..replacement
            },
        )))
        .expect("draining refusal");
    assert!(
        matches!(refused.outcome, DownloadCommandOutcome::Rejected { fault } if fault.code == DownloadFaultCode::Unavailable)
    );
    assert_eq!(
        server.requests.lock().expect("HTTP facts").len(),
        requests_before
    );
    let cancel = command_envelope(DownloadCommand::CancelPausedUrlReplacement {
        task_id,
        operation_id,
    });
    let cancel_correlation = cancel.request_id;
    let acknowledged = cleanup
        .command(cancel.clone())
        .expect("precise cancellation while draining");
    assert_eq!(acknowledged.request_id, cancel_correlation);
    assert!(matches!(
        acknowledged.outcome,
        DownloadCommandOutcome::Accepted { .. }
    ));
    let retired = terminal
        .recv_timeout(Duration::from_secs(5))
        .expect("candidate physically returns");
    assert_eq!(retired.request_id, attempt_correlation);
    assert!(
        matches!(retired.outcome, DownloadCommandOutcome::Rejected { fault } if fault.code == DownloadFaultCode::Cancelled)
    );
    worker.join().expect("candidate caller exited");
    runtime.block_on(server.wait_drained());
    let retired_cancel = cleanup
        .command(cancel)
        .expect("lease retirement observation");
    assert!(
        matches!(retired_cancel.outcome, DownloadCommandOutcome::Rejected { fault } if fault.code == DownloadFaultCode::InvalidState)
    );
    let current = engine.task(task_id).expect("task").expect("same identity");
    assert_eq!(current.task, paused.task);
    assert_eq!(current.probe, paused.probe);
    assert_eq!(current.downloaded_bytes, paused.downloaded_bytes);
    assert_eq!(current.temporary_path, paused.temporary_path);
    assert_eq!(
        store.load_task(task_id).expect("durable task").as_ref(),
        Some(&paused.task)
    );
    assert_eq!(
        store
            .load_transfer_plan(task_id)
            .expect("retained plan")
            .as_ref(),
        Some(&plan)
    );
    assert_eq!(std::fs::read(artifact).expect("retained artifact"), bytes);
    assert!(
        store
            .is_queue_task_user_paused(task_id)
            .expect("explicit user pause")
    );
    assert!(!store.is_queue_task_eligible(task_id).expect("not eligible"));
    assert_eq!(
        engine
            .paused_url_replacement_revision(task_id)
            .expect("retired revision"),
        1
    );
    drop(cleanup);
    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    while activity.connected_clients() != 0 {
        assert!(
            std::time::Instant::now() < deadline,
            "all original pipe clients drain"
        );
        std::thread::sleep(Duration::from_millis(2));
    }
    let address = server.address;
    runtime.block_on(server.close());
    assert!(
        std::net::TcpStream::connect(address).is_err(),
        "owned HTTP listener exited"
    );
}
