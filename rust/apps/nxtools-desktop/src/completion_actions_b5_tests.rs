use super::*;
use nxtools_download::{DownloadCompletionIntention, DownloadCompletionTask, DownloadQueue};

fn id(value: u128) -> DownloadTaskId {
    DownloadTaskId::from_uuid(uuid::Uuid::from_u128(value))
}
fn task(
    value: u128,
    queue: &str,
    status: DownloadStatus,
    action: Option<DownloadCompletionAction>,
) -> DownloadCompletionTask {
    DownloadCompletionTask {
        task_id: id(value),
        status,
        attempt: 1,
        accepted_revision: 0,
        queue_id: queue.into(),
        intention: DownloadCompletionIntention {
            action,
            open_directory: false,
        },
        directory: (status == DownloadStatus::Completed)
            .then(|| PathBuf::from(format!(r"C:\done\{value}"))),
        completion_batch_id: None,
    }
}
fn queue(name: &str, action: Option<DownloadCompletionAction>) -> DownloadQueue {
    serde_json::from_value(serde_json::json!({"id":name,"name":name,"position":0,"max_concurrent_tasks":null,"connection_limit":null,"speed_limit_bytes_per_second":null,"default_save_directory":null,"schedule":null,"is_running":true,"completion_intention":{"action":action,"open_directory":true}})).expect("original queue defaults")
}
fn snapshot(
    tasks: Vec<DownloadCompletionTask>,
    queues: Vec<DownloadQueue>,
    revision: u64,
) -> DownloadCompletionSnapshot {
    DownloadCompletionSnapshot {
        generation: 100,
        work_revision: revision,
        queue_invalidations: HashMap::new(),
        tasks,
        queues,
    }
}
#[test]
fn b5_restart_old_terminal_and_empty_queue_never_authorize_actions() {
    let initial = snapshot(
        vec![task(
            1,
            "a",
            DownloadStatus::Completed,
            Some(DownloadCompletionAction::Sleep),
        )],
        vec![
            queue("a", Some(DownloadCompletionAction::Shutdown)),
            queue("empty", Some(DownloadCompletionAction::Hibernate)),
        ],
        1,
    );
    let mut owner = CompletionActionCoordinator::from_snapshot(&initial);
    assert_eq!(
        owner.evaluate_completion(&initial, Instant::now()),
        ShutdownTransition::None
    );
    assert!(
        owner
            .take_feedback_batch_for_directory_policy(false)
            .is_none()
    );
}
#[test]
fn b5_all_success_round_and_task_actions_merge_once_with_stable_priority() {
    let initial = snapshot(
        vec![
            task(
                1,
                "a",
                DownloadStatus::Waiting,
                Some(DownloadCompletionAction::Sleep),
            ),
            task(
                2,
                "b",
                DownloadStatus::Waiting,
                Some(DownloadCompletionAction::ExitApplication),
            ),
        ],
        vec![
            queue("a", Some(DownloadCompletionAction::Hibernate)),
            queue("b", Some(DownloadCompletionAction::Shutdown)),
        ],
        1,
    );
    let mut owner = CompletionActionCoordinator::from_snapshot(&initial);
    let mut done = initial.clone();
    for item in &mut done.tasks {
        item.status = DownloadStatus::Completed;
        item.directory = Some(PathBuf::from(r"C:\Done"));
    }
    let ShutdownTransition::Started(countdown) = owner.evaluate_completion(&done, Instant::now())
    else {
        panic!("successful nonempty rounds start one countdown")
    };
    assert_eq!(countdown.action, DownloadCompletionAction::Shutdown);
    assert_eq!(
        owner.evaluate_completion(&done, Instant::now()),
        ShutdownTransition::None
    );
    let feedback = owner
        .take_feedback_batch_for_directory_policy(false)
        .expect("explicit queue feedback");
    assert_eq!(feedback.directories, vec![PathBuf::from(r"C:\Done")]);
    assert!(owner.cancel_countdown(countdown.correlation_id).is_some());
    assert_eq!(
        owner.evaluate_completion(&done, Instant::now()),
        ShutdownTransition::None,
        "cancel consumes completed attempts, not persistent intentions"
    );
}
#[test]
fn b5_failed_cancelled_waiting_and_removed_members_do_not_complete_queue() {
    for blocked in [
        DownloadStatus::Waiting,
        DownloadStatus::Paused,
        DownloadStatus::Failed,
        DownloadStatus::Cancelled,
    ] {
        let initial = snapshot(
            vec![
                task(1, "a", DownloadStatus::Waiting, None),
                task(2, "a", DownloadStatus::Waiting, None),
            ],
            vec![queue("a", Some(DownloadCompletionAction::Sleep))],
            1,
        );
        let mut owner = CompletionActionCoordinator::from_snapshot(&initial);
        let mut next = initial.clone();
        next.tasks[0].status = DownloadStatus::Completed;
        next.tasks[1].status = blocked;
        assert_eq!(
            owner.evaluate_completion(&next, Instant::now()),
            ShutdownTransition::None,
            "{blocked:?} is not success"
        );
        next.tasks.pop();
        next.work_revision += 1;
        assert_eq!(
            owner.evaluate_completion(&next, Instant::now()),
            ShutdownTransition::None,
            "removal aborts old round"
        );
    }
}
#[test]
fn b5_new_work_fast_terminal_and_late_callbacks_cannot_execute_old_countdown() {
    let initial = snapshot(
        vec![task(
            1,
            "a",
            DownloadStatus::Waiting,
            Some(DownloadCompletionAction::Sleep),
        )],
        vec![queue("a", None)],
        1,
    );
    let mut owner = CompletionActionCoordinator::from_snapshot(&initial);
    let mut done = initial.clone();
    done.tasks[0].status = DownloadStatus::Completed;
    let ShutdownTransition::Started(old) = owner.evaluate_completion(&done, Instant::now()) else {
        panic!("first success")
    };
    done.work_revision += 1;
    done.tasks.push(task(
        2,
        "a",
        DownloadStatus::Completed,
        Some(DownloadCompletionAction::Hibernate),
    ));
    assert!(matches!(
        owner.evaluate_completion(&done, Instant::now()),
        ShutdownTransition::Cancelled(_, "new_work")
    ));
    let ShutdownTransition::Started(current) = owner.evaluate_completion(&done, Instant::now())
    else {
        panic!("fresh success gets its own authority")
    };
    assert!(owner.cancel_countdown(old.correlation_id).is_none());
    assert!(
        owner
            .take_countdown_for_execution(old.correlation_id)
            .is_none()
    );
    assert_eq!(owner.active_countdown(), Some(current));
    assert!(
        owner
            .take_countdown_for_execution(current.correlation_id)
            .is_some()
    );
    assert!(
        owner
            .take_countdown_for_execution(current.correlation_id)
            .is_none()
    );
}
#[test]
fn b5_retry_is_new_attempt_and_generation_change_never_replays_terminal() {
    let initial = snapshot(
        vec![task(1, "a", DownloadStatus::Failed, None)],
        vec![queue("a", Some(DownloadCompletionAction::Sleep))],
        1,
    );
    let mut owner = CompletionActionCoordinator::from_snapshot(&initial);
    let mut retry = initial.clone();
    retry.tasks[0].attempt += 1;
    retry.tasks[0].status = DownloadStatus::Waiting;
    retry.work_revision += 1;
    assert_eq!(
        owner.evaluate_completion(&retry, Instant::now()),
        ShutdownTransition::None
    );
    retry.tasks[0].status = DownloadStatus::Completed;
    assert!(matches!(
        owner.evaluate_completion(&retry, Instant::now()),
        ShutdownTransition::Started(_)
    ));
    retry.generation += 1;
    assert!(matches!(
        owner.evaluate_completion(&retry, Instant::now()),
        ShutdownTransition::Cancelled(_, "generation_changed")
    ));
    assert_eq!(
        owner.evaluate_completion(&retry, Instant::now()),
        ShutdownTransition::None
    );
}

#[test]
fn b5_moving_old_completed_record_to_empty_queue_is_not_new_success() {
    let initial = snapshot(
        vec![task(1, "old", DownloadStatus::Completed, None)],
        vec![
            queue("old", None),
            queue("target", Some(DownloadCompletionAction::Sleep)),
        ],
        1,
    );
    let mut owner = CompletionActionCoordinator::from_snapshot(&initial);
    let mut moved = initial.clone();
    moved.work_revision += 1;
    moved.tasks[0].queue_id = "target".into();
    assert_eq!(
        owner.evaluate_completion(&moved, Instant::now()),
        ShutdownTransition::None
    );
    assert!(
        owner
            .take_feedback_batch_for_directory_policy(false)
            .is_none()
    );
}

use crate::completion_http_fixture as b5_http;

#[test]
fn b5_original_multiqueue_http_success_waiting_and_bounded_feedback_share_one_coordinator() {
    use nxtools_download::{
        DOWNLOAD_COMMAND_PROTOCOL_VERSION, DownloadCommand, DownloadCommandEnvelope,
        DownloadCommandOutcome, DownloadEngine, DownloadQueueAction, DownloadRequest,
        RequestOrigin,
    };
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("owned HTTP runtime");
    let server = runtime.block_on(b5_http::UrlServer::start());
    let directory = tempfile::tempdir().expect("owned SQLite/artifacts");
    let engine =
        DownloadEngine::open(directory.path().join("downloads.sqlite3")).expect("original engine");
    let command = |action| {
        engine
            .handle_command(DownloadCommandEnvelope {
                protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
                request_id: uuid::Uuid::new_v4(),
                origin: RequestOrigin::Ui,
                command: DownloadCommand::Queue { action },
            })
            .outcome
    };
    let mut ids = Vec::new();
    for (index, queue_id, action) in [
        (0, "default", DownloadCompletionAction::Hibernate),
        (1, "secondary", DownloadCompletionAction::Sleep),
    ] {
        let input:DownloadRequest=serde_json::from_value(serde_json::json!({"source_url":server.url("/compatible"),"save_directory":directory.path(),"file_name":format!("result-{index}.bin"),"connection_limit":1})).expect("original request");
        let id = engine
            .enqueue(input, RequestOrigin::Ui)
            .expect("accepted queue-only work");
        ids.push(id);
        if queue_id != "default" {
            assert!(matches!(
                command(DownloadQueueAction::AssignTasks {
                    queue_id: queue_id.into(),
                    task_ids: vec![id]
                }),
                DownloadCommandOutcome::QueueChanged { .. }
            ));
        }
        assert!(matches!(
            command(DownloadQueueAction::SetCompletionIntention {
                queue_id: queue_id.into(),
                intention: DownloadCompletionIntention {
                    action: Some(action),
                    open_directory: true
                },
                baseline: None
            }),
            DownloadCommandOutcome::QueueChanged { .. }
        ));
    }
    let mut owner = CompletionActionCoordinator::from_snapshot(
        &engine
            .completion_snapshot()
            .expect("authoritative accepted members"),
    );
    engine.resume(ids[0]).expect("first queue admission");
    assert_eq!(
        runtime
            .block_on(engine.wait_for_terminal(ids[0], Duration::from_secs(5)))
            .expect("first writer")
            .task
            .status(),
        DownloadStatus::Completed
    );
    assert_eq!(
        owner.evaluate_completion(
            &engine
                .completion_snapshot()
                .expect("other queue still waiting"),
            Instant::now()
        ),
        ShutdownTransition::None
    );
    engine.resume(ids[1]).expect("second queue admission");
    assert_eq!(
        runtime
            .block_on(engine.wait_for_terminal(ids[1], Duration::from_secs(5)))
            .expect("second writer")
            .task
            .status(),
        DownloadStatus::Completed
    );
    let done = engine
        .completion_snapshot()
        .expect("actual original final facts");
    let ShutdownTransition::Started(countdown) = owner.evaluate_completion(&done, Instant::now())
    else {
        panic!("all real queue members succeeded")
    };
    assert_eq!(countdown.action, DownloadCompletionAction::Hibernate);
    let batch = owner
        .take_feedback_batch_for_directory_policy(false)
        .expect("explicit queue directories");
    assert_eq!(batch.directories, vec![directory.path().to_path_buf()]);
    assert_eq!(
        owner.evaluate_completion(&done, Instant::now()),
        ShutdownTransition::None
    );
    let mut restarted = CompletionActionCoordinator::from_snapshot(
        &DownloadEngine::open(directory.path().join("downloads.sqlite3"))
            .expect("reopened original engine")
            .completion_snapshot()
            .expect("reload"),
    );
    assert_eq!(
        restarted.evaluate_completion(&done, Instant::now()),
        ShutdownTransition::None
    );
    engine
        .shutdown_and_wait()
        .expect("joined original transfer workers");
    runtime.block_on(server.wait_drained());
    runtime.block_on(server.close());
}

#[test]
fn b5_repair_historical_failed_and_cancelled_rows_do_not_join_a_new_round() {
    for terminal in [DownloadStatus::Failed, DownloadStatus::Cancelled] {
        let initial = snapshot(
            vec![task(1, "a", DownloadStatus::Waiting, None)],
            vec![queue("a", Some(DownloadCompletionAction::Sleep))],
            1,
        );
        let mut live = CompletionActionCoordinator::from_snapshot(&initial);
        let mut ended = initial.clone();
        ended.tasks[0].status = terminal;
        assert_eq!(
            live.evaluate_completion(&ended, Instant::now()),
            ShutdownTransition::None
        );
        let mut pending = ended;
        pending.work_revision += 1;
        pending
            .tasks
            .push(task(2, "a", DownloadStatus::Waiting, None));
        let restarted = CompletionActionCoordinator::from_snapshot(&pending);
        for mut owner in [live, restarted] {
            assert_eq!(
                owner.evaluate_completion(&pending, Instant::now()),
                ShutdownTransition::None
            );
            let mut done = pending.clone();
            done.tasks[1].status = DownloadStatus::Completed;
            assert!(
                matches!(
                    owner.evaluate_completion(&done, Instant::now()),
                    ShutdownTransition::Started(_)
                ),
                "terminal history is not membership of the later accepted round: {terminal:?}"
            );
        }
    }
}

#[test]
fn b5_repair_normal_drain_retires_original_held_http_probe() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("HTTP runtime");
    let server = runtime.block_on(b5_http::UrlServer::start());
    let directory = tempfile::tempdir().expect("owned request");
    let engine = nxtools_download::DownloadEngine::open_in_memory().expect("original engine");
    let input = serde_json::from_value(serde_json::json!({"source_url":server.url("/held"),"save_directory":directory.path(),"file_name":"probe.bin"})).expect("original request");
    let worker_engine = engine.clone();
    let (terminal, observed) = std::sync::mpsc::sync_channel(1);
    let worker = std::thread::spawn(move || {
        let result =
            worker_engine.probe_with_resolution(input, Default::default(), Default::default());
        terminal.send(result).expect("probe retirement");
    });
    runtime.block_on(server.wait_held());
    let drain = engine.shutdown_and_wait();
    let before_release = observed.recv_timeout(Duration::from_millis(300));
    server
        .hold
        .store(false, std::sync::atomic::Ordering::Release);
    worker
        .join()
        .expect("original blocking probe caller joined");
    runtime.block_on(server.wait_drained());
    let address = server.address;
    runtime.block_on(server.close());
    assert!(std::net::TcpStream::connect(address).is_err());
    drain.expect("drain");
    assert!(
        before_release.is_ok_and(|result| result.is_err()),
        "normal drain must cancel and join the held probe before fixture release"
    );
}

#[test]
fn b5_real_sqlite_failed_retry_scheduled_switch_cancel_and_restart_rounds() {
    use nxtools_download::{
        DOWNLOAD_COMMAND_PROTOCOL_VERSION, DownloadCommand, DownloadCommandEnvelope,
        DownloadCommandOutcome, DownloadEngine, DownloadQueueAction, DownloadTaskAction,
        RequestOrigin,
    };
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("HTTP runtime");
    let server = runtime.block_on(b5_http::UrlServer::start());
    let directory = tempfile::tempdir().expect("SQLite fixture");
    let database = directory.path().join("downloads.sqlite3");
    let engine = DownloadEngine::open(&database).expect("engine");
    let command = |command| {
        engine
            .handle_command(DownloadCommandEnvelope {
                protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
                request_id: uuid::Uuid::new_v4(),
                origin: RequestOrigin::Ui,
                command,
            })
            .outcome
    };
    let input = |path: &str, name: &str| {
        serde_json::from_value(serde_json::json!({"source_url":server.url(path),"save_directory":directory.path(),"file_name":name,"connection_limit":1})).expect("request")
    };
    assert!(matches!(
        command(DownloadCommand::Queue {
            action: DownloadQueueAction::SetCompletionIntention {
                queue_id: "default".into(),
                intention: DownloadCompletionIntention {
                    action: Some(DownloadCompletionAction::Sleep),
                    open_directory: true
                },
                baseline: None
            }
        }),
        DownloadCommandOutcome::QueueChanged { .. }
    ));
    let failed = engine
        .enqueue(input("/failed", "failed.bin"), RequestOrigin::Ui)
        .expect("accepted failed work");
    let mut owner = CompletionActionCoordinator::from_snapshot(
        &engine.completion_snapshot().expect("initial round"),
    );
    engine.resume(failed).expect("first attempt");
    assert_eq!(
        runtime
            .block_on(engine.wait_for_terminal(failed, Duration::from_secs(5)))
            .expect("first terminal")
            .task
            .status(),
        DownloadStatus::Failed
    );
    assert_eq!(
        owner.evaluate_completion(
            &engine.completion_snapshot().expect("failure"),
            Instant::now()
        ),
        ShutdownTransition::None
    );
    let attempt = engine.completion_snapshot().expect("attempt").tasks[0].attempt;
    assert!(matches!(
        command(DownloadCommand::Task {
            task_id: failed,
            action: DownloadTaskAction::Retry
        }),
        DownloadCommandOutcome::Accepted { .. }
    ));
    assert_eq!(
        runtime
            .block_on(engine.wait_for_terminal(failed, Duration::from_secs(5)))
            .expect("retry terminal")
            .task
            .status(),
        DownloadStatus::Failed
    );
    assert!(engine.completion_snapshot().expect("retried attempt").tasks[0].attempt > attempt);
    assert_eq!(
        owner.evaluate_completion(
            &engine.completion_snapshot().expect("retry failure"),
            Instant::now()
        ),
        ShutdownTransition::None
    );
    let mut scheduled: nxtools_download::DownloadRequest = input("/compatible", "scheduled.bin");
    scheduled.scheduled_at_unix_ms = Some(chrono::Utc::now().timestamp_millis() + 3_600_000);
    let waiting = engine
        .enqueue(scheduled, RequestOrigin::Ui)
        .expect("scheduled accepted work");
    let successful = engine
        .enqueue(input("/compatible", "successful.bin"), RequestOrigin::Ui)
        .expect("fresh accepted work");
    assert_eq!(
        owner.evaluate_completion(
            &engine.completion_snapshot().expect("accepted round"),
            Instant::now()
        ),
        ShutdownTransition::None
    );
    engine.resume(successful).expect("actual successful writer");
    assert_eq!(
        runtime
            .block_on(engine.wait_for_terminal(successful, Duration::from_secs(5)))
            .expect("success")
            .task
            .status(),
        DownloadStatus::Completed
    );
    assert_eq!(
        owner.evaluate_completion(
            &engine
                .completion_snapshot()
                .expect("scheduled work remains"),
            Instant::now()
        ),
        ShutdownTransition::None
    );
    assert!(matches!(
        command(DownloadCommand::Queue {
            action: DownloadQueueAction::AssignTasks {
                queue_id: "later".into(),
                task_ids: vec![waiting]
            }
        }),
        DownloadCommandOutcome::QueueChanged { .. }
    ));
    assert_eq!(
        owner.evaluate_completion(
            &engine.completion_snapshot().expect("moved member"),
            Instant::now()
        ),
        ShutdownTransition::None
    );
    engine.cancel(waiting).expect("cancel scheduled work");
    assert_eq!(
        owner.evaluate_completion(
            &engine.completion_snapshot().expect("cancelled member"),
            Instant::now()
        ),
        ShutdownTransition::None
    );
    engine
        .remove(waiting, false)
        .expect("remove cancelled member");
    assert_eq!(
        owner.evaluate_completion(
            &engine.completion_snapshot().expect("removed member"),
            Instant::now()
        ),
        ShutdownTransition::None
    );
    let fresh = engine
        .enqueue(input("/compatible", "fresh-round.bin"), RequestOrigin::Ui)
        .expect("later accepted round");
    assert_eq!(
        owner.evaluate_completion(
            &engine.completion_snapshot().expect("fresh round"),
            Instant::now()
        ),
        ShutdownTransition::None
    );
    engine.resume(fresh).expect("fresh actual writer");
    assert_eq!(
        runtime
            .block_on(engine.wait_for_terminal(fresh, Duration::from_secs(5)))
            .expect("fresh terminal")
            .task
            .status(),
        DownloadStatus::Completed
    );
    let done = engine
        .completion_snapshot()
        .expect("all current members succeeded");
    let ShutdownTransition::Started(countdown) = owner.evaluate_completion(&done, Instant::now())
    else {
        panic!("new successful round must not import failed terminal history")
    };
    assert_eq!(countdown.action, DownloadCompletionAction::Sleep);
    owner
        .cancel_countdown(countdown.correlation_id)
        .expect("explicit cancel");
    assert_eq!(
        owner.evaluate_completion(&done, Instant::now()),
        ShutdownTransition::None
    );
    engine.shutdown_and_wait().expect("join writers");
    let reloaded = DownloadEngine::open(&database).expect("reload original SQLite");
    let restored = reloaded
        .completion_snapshot()
        .expect("restored intentions and history");
    let mut restarted = CompletionActionCoordinator::from_snapshot(&restored);
    assert_eq!(
        restarted.evaluate_completion(&restored, Instant::now()),
        ShutdownTransition::None
    );
    reloaded.shutdown_and_wait().expect("reloaded drain");
    runtime.block_on(server.wait_drained());
    runtime.block_on(server.close());
}
