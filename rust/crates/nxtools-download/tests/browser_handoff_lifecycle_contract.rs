mod support;

use std::path::PathBuf;
use std::time::Duration;

use nxtools_download::{
    BrowserDownloadDialogSource, BrowserHandoffState, DOWNLOAD_COMMAND_PROTOCOL_VERSION,
    DownloadBatchAction, DownloadCommand, DownloadCommandEnvelope, DownloadCommandOutcome,
    DownloadEngine, DownloadFaultCode, DownloadQueueAction, DownloadRequest,
    DownloadResolutionMode, DownloadResolutionOptions, DownloadStatus, DownloadSubmitMode,
    DownloadTaskAction, DownloadTaskId, RequestOrigin,
};
use serde_json::json;
use support::DeterministicHttpServer;
use uuid::Uuid;

fn request(server: &DeterministicHttpServer, save_directory: PathBuf) -> DownloadRequest {
    DownloadRequest {
        source_url: format!("http://{}/range", server.address()),
        save_directory,
        save_directory_explicit: true,
        file_name: Some("controlled-handoff.bin".into()),
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
        duplicate_strategy: Default::default(),
    }
}

fn command(command: DownloadCommand) -> DownloadCommandEnvelope {
    DownloadCommandEnvelope {
        protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
        request_id: Uuid::new_v4(),
        origin: RequestOrigin::Browser,
        command,
    }
}

fn bypass_resolution(handoff_id: Uuid) -> DownloadResolutionOptions {
    DownloadResolutionOptions {
        operation_id: handoff_id,
        mode: DownloadResolutionMode::Bypass,
    }
}

fn assert_handoff(
    outcome: DownloadCommandOutcome,
    expected_id: Uuid,
    expected_state: BrowserHandoffState,
) -> Option<DownloadTaskId> {
    match outcome {
        DownloadCommandOutcome::BrowserHandoff {
            handoff_id,
            state,
            task_id,
        } => {
            assert_eq!(handoff_id, expected_id);
            assert_eq!(state, expected_state);
            task_id
        }
        other => panic!("unexpected browser handoff outcome: {other:?}"),
    }
}

fn prepare_dialog(
    engine: &DownloadEngine,
    request: DownloadRequest,
    handoff_id: Uuid,
) -> DownloadCommandOutcome {
    let dialogs = engine
        .take_browser_dialog_receiver()
        .expect("attach controlled browser dialog consumer");
    let command_engine = engine.clone();
    let prepare = std::thread::spawn(move || {
        command_engine.handle_command(command(DownloadCommand::BrowserPrepareHandoff {
            handoff_id,
            request,
            open_dialog: true,
            mode: DownloadSubmitMode::StartNow,
        }))
    });
    let dialog = dialogs
        .recv_blocking()
        .expect("receive controlled browser dialog");
    assert_eq!(dialog.handoff_id, handoff_id);
    assert!(matches!(
        dialog.source,
        BrowserDownloadDialogSource::ControlledDownload(_)
    ));
    dialog
        .completion
        .send(Ok(()))
        .expect("confirm controlled browser dialog creation");
    prepare.join().expect("controlled prepare thread").outcome
}

fn accept(
    engine: &DownloadEngine,
    request: DownloadRequest,
    handoff_id: Uuid,
    mode: DownloadSubmitMode,
) -> DownloadCommandOutcome {
    engine
        .handle_command(command(DownloadCommand::BrowserAcceptHandoff {
            handoff_id,
            request,
            mode,
            resolution: bypass_resolution(handoff_id),
        }))
        .outcome
}

#[test]
fn controlled_handoff_wire_contract_is_versioned_and_snake_case() {
    assert_eq!(DOWNLOAD_COMMAND_PROTOCOL_VERSION, 14);
    assert_eq!(
        serde_json::to_value(BrowserHandoffState::Pending).expect("serialize pending state"),
        json!("pending")
    );
    assert_eq!(
        serde_json::to_value(BrowserHandoffState::Held).expect("serialize held state"),
        json!("held")
    );
    let handoff_id = Uuid::new_v4();
    let prepare = serde_json::to_value(DownloadCommand::BrowserPrepareHandoff {
        handoff_id,
        request: request(
            &DeterministicHttpServer::start(),
            PathBuf::from("controlled-wire"),
        ),
        open_dialog: false,
        mode: DownloadSubmitMode::QueueOnly,
    })
    .expect("serialize controlled prepare command");
    let task_id = DownloadTaskId::new();
    let status = serde_json::to_value(DownloadCommand::BrowserHandoffStatus { handoff_id })
        .expect("serialize controlled status command");
    let commit = serde_json::to_value(DownloadCommand::BrowserCommitHandoff {
        handoff_id,
        task_id,
    })
    .expect("serialize controlled commit command");
    let outcome = serde_json::to_value(DownloadCommandOutcome::BrowserHandoff {
        handoff_id,
        state: BrowserHandoffState::Committed,
        task_id: Some(task_id),
    })
    .expect("serialize controlled outcome");
    assert_eq!(prepare["command"], json!("browser_prepare_handoff"));
    assert_eq!(prepare["data"]["mode"], json!("queue_only"));
    assert_eq!(status["command"], json!("browser_handoff_status"));
    assert_eq!(commit["command"], json!("browser_commit_handoff"));
    assert_eq!(outcome["outcome"], json!("browser_handoff"));
    assert_eq!(outcome["data"]["state"], json!("committed"));
}

#[tokio::test]
async fn pending_and_held_never_transfer_and_exact_commit_starts_once_after_reopen() {
    let server = DeterministicHttpServer::start();
    let directory = tempfile::tempdir().expect("controlled handoff directory");
    let database = directory.path().join("downloads.sqlite3");
    let output = directory.path().join("artifacts");
    let handoff_id = Uuid::new_v4();
    let download_request = request(&server, output);

    let engine = DownloadEngine::open(&database).expect("open controlled handoff engine");
    assert_handoff(
        prepare_dialog(&engine, download_request.clone(), handoff_id),
        handoff_id,
        BrowserHandoffState::Pending,
    );
    assert!(engine.tasks().expect("pending task snapshot").is_empty());
    assert_eq!(server.total_requests(), 0);

    let task_id = assert_handoff(
        accept(
            &engine,
            download_request,
            handoff_id,
            DownloadSubmitMode::StartNow,
        ),
        handoff_id,
        BrowserHandoffState::Held,
    )
    .expect("held task identity");
    assert_eq!(server.total_requests(), 0);
    assert_eq!(
        engine
            .task(task_id)
            .expect("held task query")
            .expect("held task")
            .task
            .status(),
        DownloadStatus::Created
    );

    let resume = engine.handle_command(command(DownloadCommand::Task {
        task_id,
        action: DownloadTaskAction::Resume,
    }));
    assert!(matches!(
        resume.outcome,
        DownloadCommandOutcome::Rejected { fault }
            if fault.code == DownloadFaultCode::InvalidState
    ));
    let batch = engine.handle_command(command(DownloadCommand::Batch {
        task_ids: vec![task_id],
        action: DownloadBatchAction::Resume,
    }));
    assert!(matches!(
        batch.outcome,
        DownloadCommandOutcome::Batch { results }
            if results.len() == 1 && results[0].fault.as_ref().is_some_and(|fault| fault.code == DownloadFaultCode::InvalidState)
    ));
    engine.handle_command(command(DownloadCommand::Queue {
        action: DownloadQueueAction::SetMaxConcurrent {
            queue_id: "default".into(),
            max_concurrent_tasks: 4,
        },
    }));
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(server.total_requests(), 0);
    drop(engine);

    let reopened = DownloadEngine::open(&database).expect("reopen held handoff engine");
    assert_eq!(server.total_requests(), 0);
    assert_eq!(
        assert_handoff(
            reopened
                .handle_command(command(DownloadCommand::BrowserHandoffStatus {
                    handoff_id,
                }))
                .outcome,
            handoff_id,
            BrowserHandoffState::Held,
        ),
        Some(task_id)
    );
    let mismatched = reopened.handle_command(command(DownloadCommand::BrowserCommitHandoff {
        handoff_id,
        task_id: DownloadTaskId::new(),
    }));
    assert!(matches!(
        mismatched.outcome,
        DownloadCommandOutcome::Rejected { fault }
            if fault.code == DownloadFaultCode::InvalidState
    ));
    assert_eq!(server.total_requests(), 0);

    assert_eq!(
        assert_handoff(
            reopened
                .handle_command(command(DownloadCommand::BrowserCommitHandoff {
                    handoff_id,
                    task_id,
                }))
                .outcome,
            handoff_id,
            BrowserHandoffState::Committed,
        ),
        Some(task_id)
    );
    reopened
        .wait_for_terminal(task_id, Duration::from_secs(5))
        .await
        .expect("committed transfer reaches terminal state");
    let requests_after_commit = server.total_requests();
    assert!(requests_after_commit > 0);
    assert_eq!(reopened.tasks().expect("committed tasks").len(), 1);

    assert_eq!(
        assert_handoff(
            reopened
                .handle_command(command(DownloadCommand::BrowserCommitHandoff {
                    handoff_id,
                    task_id,
                }))
                .outcome,
            handoff_id,
            BrowserHandoffState::Committed,
        ),
        Some(task_id)
    );
    assert_eq!(
        assert_handoff(
            reopened
                .handle_command(command(DownloadCommand::BrowserAbortHandoff { handoff_id }))
                .outcome,
            handoff_id,
            BrowserHandoffState::Committed,
        ),
        Some(task_id)
    );
    assert_eq!(
        reopened
            .task(task_id)
            .expect("task after stale abort query")
            .expect("task after stale abort")
            .task
            .status(),
        DownloadStatus::Completed
    );
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(server.total_requests(), requests_after_commit);
    assert_eq!(reopened.tasks().expect("duplicate commit tasks").len(), 1);
}

#[test]
fn abort_and_unknown_tombstone_block_late_acceptance_and_reprepare() {
    let server = DeterministicHttpServer::start();
    let directory = tempfile::tempdir().expect("abort handoff directory");
    let engine = DownloadEngine::open_in_memory().expect("open abort engine");
    let handoff_id = Uuid::new_v4();
    let download_request = request(&server, directory.path().join("artifacts"));

    let task_id = assert_handoff(
        engine
            .handle_command(command(DownloadCommand::BrowserPrepareHandoff {
                handoff_id,
                request: download_request.clone(),
                open_dialog: false,
                mode: DownloadSubmitMode::StartNow,
            }))
            .outcome,
        handoff_id,
        BrowserHandoffState::Held,
    )
    .expect("automatic held task");
    assert_eq!(server.total_requests(), 0);
    assert_eq!(
        assert_handoff(
            engine
                .handle_command(command(DownloadCommand::BrowserAbortHandoff { handoff_id }))
                .outcome,
            handoff_id,
            BrowserHandoffState::Aborted,
        ),
        Some(task_id)
    );
    assert_eq!(
        engine
            .task(task_id)
            .expect("aborted task query")
            .expect("aborted held task")
            .task
            .status(),
        DownloadStatus::Cancelled
    );
    assert_handoff(
        accept(
            &engine,
            download_request.clone(),
            handoff_id,
            DownloadSubmitMode::StartNow,
        ),
        handoff_id,
        BrowserHandoffState::Aborted,
    );
    assert_handoff(
        engine
            .handle_command(command(DownloadCommand::BrowserPrepareHandoff {
                handoff_id,
                request: download_request,
                open_dialog: false,
                mode: DownloadSubmitMode::StartNow,
            }))
            .outcome,
        handoff_id,
        BrowserHandoffState::Aborted,
    );
    assert_eq!(engine.tasks().expect("aborted task count").len(), 1);
    assert_eq!(server.total_requests(), 0);

    let unknown_id = Uuid::new_v4();
    assert_handoff(
        engine
            .handle_command(command(DownloadCommand::BrowserAbortHandoff {
                handoff_id: unknown_id,
            }))
            .outcome,
        unknown_id,
        BrowserHandoffState::Aborted,
    );
    assert_handoff(
        engine
            .handle_command(command(DownloadCommand::BrowserPrepareHandoff {
                handoff_id: unknown_id,
                request: request(&server, directory.path().join("other")),
                open_dialog: false,
                mode: DownloadSubmitMode::StartNow,
            }))
            .outcome,
        unknown_id,
        BrowserHandoffState::Aborted,
    );
    assert_eq!(engine.tasks().expect("tombstone task count").len(), 1);
}

#[test]
fn dialog_open_failure_aborts_pending_identity_before_late_acceptance() {
    let server = DeterministicHttpServer::start();
    let directory = tempfile::tempdir().expect("dialog failure directory");
    let engine = DownloadEngine::open_in_memory().expect("open dialog failure engine");
    let dialogs = engine
        .take_browser_dialog_receiver()
        .expect("attach failed dialog consumer");
    let handoff_id = Uuid::new_v4();
    let download_request = request(&server, directory.path().join("artifacts"));
    let command_engine = engine.clone();
    let prepare_request = download_request.clone();
    let prepare = std::thread::spawn(move || {
        command_engine.handle_command(command(DownloadCommand::BrowserPrepareHandoff {
            handoff_id,
            request: prepare_request,
            open_dialog: true,
            mode: DownloadSubmitMode::StartNow,
        }))
    });
    dialogs
        .recv_blocking()
        .expect("receive failed controlled dialog")
        .completion
        .send(Err("window creation failed".into()))
        .expect("report controlled dialog failure");
    assert!(matches!(
        prepare.join().expect("failed prepare thread").outcome,
        DownloadCommandOutcome::Rejected { fault }
            if fault.code == DownloadFaultCode::Unavailable
    ));
    assert_handoff(
        engine
            .handle_command(command(DownloadCommand::BrowserHandoffStatus {
                handoff_id,
            }))
            .outcome,
        handoff_id,
        BrowserHandoffState::Aborted,
    );
    assert_handoff(
        accept(
            &engine,
            download_request,
            handoff_id,
            DownloadSubmitMode::StartNow,
        ),
        handoff_id,
        BrowserHandoffState::Aborted,
    );
    assert!(engine.tasks().expect("failed dialog tasks").is_empty());
    assert_eq!(server.total_requests(), 0);
}

#[test]
fn consumer_unavailable_retry_with_same_identity_opens_real_pending_dialog() {
    let server = DeterministicHttpServer::start();
    let directory = tempfile::tempdir().expect("consumer retry directory");
    let engine = DownloadEngine::open_in_memory().expect("open consumer retry engine");
    let handoff_id = Uuid::new_v4();
    let download_request = request(&server, directory.path().join("artifacts"));

    let unavailable = engine.handle_command(command(DownloadCommand::BrowserPrepareHandoff {
        handoff_id,
        request: download_request.clone(),
        open_dialog: true,
        mode: DownloadSubmitMode::StartNow,
    }));
    assert!(matches!(
        unavailable.outcome,
        DownloadCommandOutcome::Rejected { fault }
            if fault.code == DownloadFaultCode::Unavailable && fault.retryable
    ));
    assert!(
        engine
            .tasks()
            .expect("unavailable consumer tasks")
            .is_empty()
    );
    assert_handoff(
        engine
            .handle_command(command(DownloadCommand::BrowserHandoffStatus {
                handoff_id,
            }))
            .outcome,
        handoff_id,
        BrowserHandoffState::Missing,
    );

    let dialogs = engine
        .take_browser_dialog_receiver()
        .expect("attach retry dialog consumer");
    let command_engine = engine.clone();
    let retry = std::thread::spawn(move || {
        command_engine.handle_command(command(DownloadCommand::BrowserPrepareHandoff {
            handoff_id,
            request: download_request,
            open_dialog: true,
            mode: DownloadSubmitMode::StartNow,
        }))
    });
    let dialog = dialogs
        .recv_blocking()
        .expect("receive retried controlled dialog");
    assert_eq!(dialog.handoff_id, handoff_id);
    assert!(matches!(
        dialog.source,
        BrowserDownloadDialogSource::ControlledDownload(_)
    ));
    dialog
        .completion
        .send(Ok(()))
        .expect("confirm retried controlled dialog");
    assert_handoff(
        retry.join().expect("retry prepare thread").outcome,
        handoff_id,
        BrowserHandoffState::Pending,
    );
    assert!(engine.tasks().expect("retried pending tasks").is_empty());
    assert_eq!(server.total_requests(), 0);
}

#[tokio::test]
async fn held_schedule_stays_dormant_and_committed_schedule_is_preserved_until_due() {
    let server = DeterministicHttpServer::start();
    let directory = tempfile::tempdir().expect("scheduled handoff directory");
    let engine = DownloadEngine::open_in_memory().expect("open scheduled handoff engine");
    let handoff_id = Uuid::new_v4();
    let mut download_request = request(&server, directory.path().join("artifacts"));
    let due_at = chrono::Utc::now().timestamp_millis() + 60_000;
    download_request.scheduled_at_unix_ms = Some(due_at);
    let task_id = assert_handoff(
        engine
            .handle_command(command(DownloadCommand::BrowserPrepareHandoff {
                handoff_id,
                request: download_request,
                open_dialog: false,
                mode: DownloadSubmitMode::StartNow,
            }))
            .outcome,
        handoff_id,
        BrowserHandoffState::Held,
    )
    .expect("scheduled held task");
    assert_eq!(
        engine
            .process_due_schedules_at(due_at)
            .expect("held due schedule pass"),
        0
    );
    assert_eq!(server.total_requests(), 0);
    assert_handoff(
        engine
            .handle_command(command(DownloadCommand::BrowserCommitHandoff {
                handoff_id,
                task_id,
            }))
            .outcome,
        handoff_id,
        BrowserHandoffState::Committed,
    );
    assert_eq!(server.total_requests(), 0);
    assert_eq!(
        engine
            .task(task_id)
            .expect("scheduled committed task query")
            .expect("scheduled committed task")
            .task
            .request()
            .scheduled_at_unix_ms,
        Some(due_at)
    );
    assert_eq!(
        engine
            .process_due_schedules_at(due_at)
            .expect("committed due schedule pass"),
        1
    );
    engine
        .wait_for_terminal(task_id, Duration::from_secs(5))
        .await
        .expect("due committed schedule transfers");
    assert!(server.total_requests() > 0);
}

#[test]
fn expired_pending_handoff_rejects_late_confirmation_without_a_task() {
    let server = DeterministicHttpServer::start();
    let directory = tempfile::tempdir().expect("expiry handoff directory");
    let database = directory.path().join("downloads.sqlite3");
    let engine = DownloadEngine::open(&database).expect("open expiry engine");
    let handoff_id = Uuid::new_v4();
    let download_request = request(&server, directory.path().join("artifacts"));
    assert_handoff(
        prepare_dialog(&engine, download_request.clone(), handoff_id),
        handoff_id,
        BrowserHandoffState::Pending,
    );
    let connection = rusqlite::Connection::open(&database).expect("open expiry database");
    connection
        .execute(
            "UPDATE download_controlled_browser_handoffs SET deadline_unix_ms = 0 WHERE handoff_id = ?1",
            [handoff_id.to_string()],
        )
        .expect("expire pending lease");
    drop(connection);

    assert_handoff(
        engine
            .handle_command(command(DownloadCommand::BrowserHandoffStatus {
                handoff_id,
            }))
            .outcome,
        handoff_id,
        BrowserHandoffState::Expired,
    );
    assert_handoff(
        accept(
            &engine,
            download_request,
            handoff_id,
            DownloadSubmitMode::StartNow,
        ),
        handoff_id,
        BrowserHandoffState::Expired,
    );
    assert!(engine.tasks().expect("expired task snapshot").is_empty());
    assert_eq!(server.total_requests(), 0);
}

#[test]
fn held_acceptance_sql_failure_rolls_back_task_queue_and_state() {
    let server = DeterministicHttpServer::start();
    let directory = tempfile::tempdir().expect("accept rollback directory");
    let database = directory.path().join("downloads.sqlite3");
    let engine = DownloadEngine::open(&database).expect("open accept rollback engine");
    let handoff_id = Uuid::new_v4();
    let download_request = request(&server, directory.path().join("artifacts"));
    assert_handoff(
        prepare_dialog(&engine, download_request.clone(), handoff_id),
        handoff_id,
        BrowserHandoffState::Pending,
    );
    let connection = rusqlite::Connection::open(&database).expect("open accept rollback database");
    connection
        .execute_batch(
            "CREATE TRIGGER reject_controlled_task_insert BEFORE INSERT ON download_tasks BEGIN SELECT RAISE(ABORT, 'reject controlled task'); END;",
        )
        .expect("install task insert trigger");
    drop(connection);

    let rejected = accept(
        &engine,
        download_request,
        handoff_id,
        DownloadSubmitMode::StartNow,
    );
    assert!(matches!(
        rejected,
        DownloadCommandOutcome::Rejected { fault } if fault.code == DownloadFaultCode::Storage
    ));
    assert!(
        engine
            .tasks()
            .expect("rolled back task snapshot")
            .is_empty()
    );
    assert_handoff(
        engine
            .handle_command(command(DownloadCommand::BrowserHandoffStatus {
                handoff_id,
            }))
            .outcome,
        handoff_id,
        BrowserHandoffState::Pending,
    );
    assert_eq!(server.total_requests(), 0);
}

#[tokio::test]
async fn commit_sql_failure_preserves_held_state_until_retry() {
    let server = DeterministicHttpServer::start();
    let directory = tempfile::tempdir().expect("commit rollback directory");
    let database = directory.path().join("downloads.sqlite3");
    let engine = DownloadEngine::open(&database).expect("open commit rollback engine");
    let handoff_id = Uuid::new_v4();
    let task_id = assert_handoff(
        engine
            .handle_command(command(DownloadCommand::BrowserPrepareHandoff {
                handoff_id,
                request: request(&server, directory.path().join("artifacts")),
                open_dialog: false,
                mode: DownloadSubmitMode::StartNow,
            }))
            .outcome,
        handoff_id,
        BrowserHandoffState::Held,
    )
    .expect("held task before commit rollback");
    let connection = rusqlite::Connection::open(&database).expect("open commit rollback database");
    connection
        .execute_batch(
            "CREATE TRIGGER reject_controlled_commit BEFORE UPDATE OF state ON download_controlled_browser_handoffs WHEN NEW.state = 'committed' BEGIN SELECT RAISE(ABORT, 'reject controlled commit'); END;",
        )
        .expect("install commit trigger");
    drop(connection);

    let rejected = engine.handle_command(command(DownloadCommand::BrowserCommitHandoff {
        handoff_id,
        task_id,
    }));
    assert!(matches!(
        rejected.outcome,
        DownloadCommandOutcome::Rejected { fault } if fault.code == DownloadFaultCode::Storage
    ));
    assert_handoff(
        engine
            .handle_command(command(DownloadCommand::BrowserHandoffStatus {
                handoff_id,
            }))
            .outcome,
        handoff_id,
        BrowserHandoffState::Held,
    );
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(server.total_requests(), 0);

    let connection =
        rusqlite::Connection::open(&database).expect("reopen commit rollback database");
    connection
        .execute_batch("DROP TRIGGER reject_controlled_commit;")
        .expect("remove commit trigger");
    drop(connection);
    assert_handoff(
        engine
            .handle_command(command(DownloadCommand::BrowserCommitHandoff {
                handoff_id,
                task_id,
            }))
            .outcome,
        handoff_id,
        BrowserHandoffState::Committed,
    );
    engine
        .wait_for_terminal(task_id, Duration::from_secs(5))
        .await
        .expect("retried commit transfers held task");
    assert!(server.total_requests() > 0);
}

#[test]
fn queue_only_commit_releases_the_hold_without_starting_transfer() {
    let server = DeterministicHttpServer::start();
    let directory = tempfile::tempdir().expect("queue-only handoff directory");
    let engine = DownloadEngine::open_in_memory().expect("open queue-only engine");
    let handoff_id = Uuid::new_v4();
    let task_id = assert_handoff(
        engine
            .handle_command(command(DownloadCommand::BrowserPrepareHandoff {
                handoff_id,
                request: request(&server, directory.path().join("artifacts")),
                open_dialog: false,
                mode: DownloadSubmitMode::QueueOnly,
            }))
            .outcome,
        handoff_id,
        BrowserHandoffState::Held,
    )
    .expect("no-dialog queue-only held task");
    assert_handoff(
        engine
            .handle_command(command(DownloadCommand::BrowserCommitHandoff {
                handoff_id,
                task_id,
            }))
            .outcome,
        handoff_id,
        BrowserHandoffState::Committed,
    );
    std::thread::sleep(Duration::from_millis(100));
    assert_eq!(server.total_requests(), 0);
    assert_eq!(
        engine
            .task(task_id)
            .expect("queue-only task query")
            .expect("queue-only task")
            .task
            .status(),
        DownloadStatus::Created
    );
}

#[test]
fn acceptance_preserves_browser_resource_identity_but_allows_destination_edits() {
    let server = DeterministicHttpServer::start();
    let directory = tempfile::tempdir().expect("resource identity directory");
    let engine = DownloadEngine::open_in_memory().expect("resource identity engine");
    let handoff_id = Uuid::new_v4();
    let mut original = request(&server, directory.path().join("original"));
    original.source_page = Some(nxtools_download::DownloadSourcePage {
        url: "https://page.example.test/original".into(),
        title: "Original page".into(),
    });
    assert_handoff(
        prepare_dialog(&engine, original.clone(), handoff_id),
        handoff_id,
        BrowserHandoffState::Pending,
    );
    for changed in 0..4 {
        let mut replacement = original.clone();
        match changed {
            0 => replacement.source_url.push_str("?different-resource"),
            1 => replacement.cookie = Some("fixture=other-context".into()),
            2 => replacement.referrer = Some("https://example.test/other".into()),
            _ => replacement
                .headers
                .push(nxtools_download::DownloadRequestHeader {
                    name: "X-Fixture".into(),
                    value: "other".into(),
                }),
        }
        assert!(matches!(
            accept(&engine, replacement, handoff_id, DownloadSubmitMode::StartNow),
            DownloadCommandOutcome::Rejected { fault }
                if fault.code == DownloadFaultCode::InvalidRequest
        ));
        assert!(engine.tasks().expect("no mismatched task").is_empty());
    }
    assert_eq!(server.total_requests(), 0);
    let mut accepted = original;
    accepted.save_directory = directory.path().join("chosen");
    accepted.file_name = Some("chosen.bin".into());
    accepted.source_page = Some(nxtools_download::DownloadSourcePage {
        url: "https://page.example.test/updated".into(),
        title: "Updated page".into(),
    });
    accepted.scheduled_at_unix_ms = Some(chrono::Utc::now().timestamp_millis() + 60_000);
    let task_id = assert_handoff(
        accept(
            &engine,
            accepted.clone(),
            handoff_id,
            DownloadSubmitMode::StartNow,
        ),
        handoff_id,
        BrowserHandoffState::Held,
    )
    .expect("same resource held");
    let task = engine
        .task(task_id)
        .expect("accepted task query")
        .expect("accepted task");
    assert_eq!(task.task.request().save_directory, accepted.save_directory);
    assert_eq!(task.task.request().file_name, accepted.file_name);
    assert_eq!(task.task.request().source_page, accepted.source_page);
    assert_eq!(
        task.task.request().scheduled_at_unix_ms,
        accepted.scheduled_at_unix_ms
    );
    assert_eq!(server.total_requests(), 0);
}

#[tokio::test]
async fn committed_receipt_survives_remove_and_redownload_after_engine_reopen() {
    for action in [
        DownloadTaskAction::Remove {
            delete_artifact: false,
        },
        DownloadTaskAction::Redownload,
    ] {
        let server = DeterministicHttpServer::start();
        let directory = tempfile::tempdir().expect("committed receipt directory");
        let database = directory.path().join("downloads.sqlite3");
        let engine = DownloadEngine::open(&database).expect("committed receipt engine");
        let handoff_id = Uuid::new_v4();
        let task_id = assert_handoff(
            engine
                .handle_command(command(DownloadCommand::BrowserPrepareHandoff {
                    handoff_id,
                    request: request(&server, directory.path().join("artifacts")),
                    open_dialog: false,
                    mode: DownloadSubmitMode::StartNow,
                }))
                .outcome,
            handoff_id,
            BrowserHandoffState::Held,
        )
        .expect("held receipt task");
        assert_handoff(
            engine
                .handle_command(command(DownloadCommand::BrowserCommitHandoff {
                    handoff_id,
                    task_id,
                }))
                .outcome,
            handoff_id,
            BrowserHandoffState::Committed,
        );
        engine
            .wait_for_terminal(task_id, Duration::from_secs(5))
            .await
            .expect("committed task completes");
        let outcome = engine
            .handle_command(command(DownloadCommand::Task { task_id, action }))
            .outcome;
        assert!(
            !matches!(outcome, DownloadCommandOutcome::Rejected { .. }),
            "supported task removal: {outcome:?}"
        );
        assert!(engine.task(task_id).expect("removed task query").is_none());
        drop(engine);
        let reopened = DownloadEngine::open(&database).expect("reopen receipt engine");
        for replay in [
            DownloadCommand::BrowserHandoffStatus { handoff_id },
            DownloadCommand::BrowserCommitHandoff {
                handoff_id,
                task_id,
            },
            DownloadCommand::BrowserAbortHandoff { handoff_id },
        ] {
            assert_eq!(
                assert_handoff(
                    reopened.handle_command(command(replay)).outcome,
                    handoff_id,
                    BrowserHandoffState::Committed
                ),
                Some(task_id),
            );
        }
    }
}
