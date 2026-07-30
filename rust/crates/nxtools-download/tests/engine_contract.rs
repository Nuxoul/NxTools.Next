mod support;

use std::io::{Seek, Write};
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

use nxtools_download::{
    DOWNLOAD_COMMAND_PROTOCOL_VERSION, DownloadBatchAction, DownloadCommand,
    DownloadCommandEnvelope, DownloadCommandOutcome, DownloadDailySchedule, DownloadEngine,
    DownloadEvent, DownloadQueueAction, DownloadQueuePlacement, DownloadRequest, DownloadSchedule,
    DownloadScheduleMoment, DownloadStatus, DownloadSubmitMode, DownloadTask, DownloadTaskAction,
    DownloadWeekday, RequestOrigin, SqliteDownloadStore, daily_schedule_window_contains,
};
use support::{DeterministicHttpServer, concurrent_fixture_bytes};

fn request(server: &DeterministicHttpServer, save_directory: PathBuf) -> DownloadRequest {
    DownloadRequest {
        source_url: format!("http://{}/range", server.address()),
        save_directory,
        file_name: Some("engine-fixture.bin".to_owned()),
        headers: Vec::new(),
        cookie: None,
        referrer: None,
        connection_limit: Some(1),
        speed_limit_bytes_per_second: None,
        scheduled_at_unix_ms: None,
    }
}

#[tokio::test]
async fn delayed_submission_stays_dormant_until_its_due_time_then_enters_the_queue() {
    let server = DeterministicHttpServer::start();
    let directory = tempfile::tempdir().expect("create engine directory");
    let engine = DownloadEngine::open_in_memory().expect("open download engine");
    let due_at_unix_ms = 9_000_000_000_000_i64;
    let mut delayed = request(&server, directory.path().to_path_buf());
    delayed.scheduled_at_unix_ms = Some(due_at_unix_ms);

    let task_id = engine
        .submit(delayed, RequestOrigin::Ui)
        .expect("accept delayed submission");
    let before_due = engine
        .task(task_id)
        .expect("read delayed task")
        .expect("delayed task exists");
    assert_eq!(before_due.task.status(), DownloadStatus::Created);
    assert!(!before_due.queue_eligible);
    assert_eq!(
        before_due.task.request().scheduled_at_unix_ms,
        Some(due_at_unix_ms)
    );

    engine
        .process_due_schedules_at(due_at_unix_ms)
        .expect("release task at due time");
    tokio::time::sleep(Duration::from_millis(50)).await;
    let after_due = engine
        .task(task_id)
        .expect("read released task")
        .expect("released task exists");
    assert_ne!(after_due.task.status(), DownloadStatus::Created);
    assert!(
        !after_due.queue_eligible,
        "queue admission consumes eligibility once a transfer slot is assigned"
    );
    assert_eq!(after_due.task.request().scheduled_at_unix_ms, None);
}

#[tokio::test]
async fn delayed_submission_survives_restart_and_is_released_when_it_becomes_due() {
    let server = DeterministicHttpServer::start();
    let directory = tempfile::tempdir().expect("create engine directory");
    let database = directory.path().join("downloads.db");
    let due_at_unix_ms = 9_000_000_000_000_i64;
    let task_id = {
        let engine = DownloadEngine::open(&database).expect("open first engine");
        let mut delayed = request(&server, directory.path().join("artifact"));
        delayed.scheduled_at_unix_ms = Some(due_at_unix_ms);
        engine
            .submit(delayed, RequestOrigin::Ui)
            .expect("accept delayed submission")
    };

    let reopened = DownloadEngine::open(&database).expect("reopen engine");
    let restored = reopened
        .task(task_id)
        .expect("read restored task")
        .expect("restored task exists");
    assert_eq!(restored.task.status(), DownloadStatus::Created);
    assert!(!restored.queue_eligible);
    assert_eq!(
        restored.task.request().scheduled_at_unix_ms,
        Some(due_at_unix_ms)
    );

    reopened
        .process_due_schedules_at(due_at_unix_ms)
        .expect("release restored delayed submission");
    tokio::time::sleep(Duration::from_millis(50)).await;
    let released = reopened
        .task(task_id)
        .expect("read released restored task")
        .expect("released task exists");
    assert_ne!(released.task.status(), DownloadStatus::Created);
    assert_eq!(released.task.request().scheduled_at_unix_ms, None);
}

#[test]
fn daily_schedule_window_handles_normal_and_cross_midnight_selected_days() {
    let normal = DownloadDailySchedule {
        days: vec![DownloadWeekday::Monday],
        start_minute: 9 * 60,
        stop_minute: 17 * 60,
        enabled_start: true,
        enabled_stop: true,
    };
    assert!(daily_schedule_window_contains(
        &normal,
        DownloadScheduleMoment {
            weekday: DownloadWeekday::Monday,
            minute_of_day: 9 * 60,
        }
    ));
    assert!(!daily_schedule_window_contains(
        &normal,
        DownloadScheduleMoment {
            weekday: DownloadWeekday::Monday,
            minute_of_day: 17 * 60,
        }
    ));
    assert!(!daily_schedule_window_contains(
        &normal,
        DownloadScheduleMoment {
            weekday: DownloadWeekday::Tuesday,
            minute_of_day: 9 * 60,
        }
    ));

    let overnight = DownloadDailySchedule {
        days: vec![DownloadWeekday::Friday],
        start_minute: 22 * 60,
        stop_minute: 2 * 60,
        enabled_start: true,
        enabled_stop: true,
    };
    assert!(daily_schedule_window_contains(
        &overnight,
        DownloadScheduleMoment {
            weekday: DownloadWeekday::Friday,
            minute_of_day: 23 * 60,
        }
    ));
    assert!(daily_schedule_window_contains(
        &overnight,
        DownloadScheduleMoment {
            weekday: DownloadWeekday::Saturday,
            minute_of_day: 60,
        }
    ));
    assert!(!daily_schedule_window_contains(
        &overnight,
        DownloadScheduleMoment {
            weekday: DownloadWeekday::Saturday,
            minute_of_day: 2 * 60,
        }
    ));
    assert!(!daily_schedule_window_contains(
        &overnight,
        DownloadScheduleMoment {
            weekday: DownloadWeekday::Saturday,
            minute_of_day: 3 * 60,
        }
    ));
}

#[tokio::test]
async fn daily_queue_window_releases_queue_only_tasks_and_stops_after_the_window() {
    let server = DeterministicHttpServer::start();
    let directory = tempfile::tempdir().expect("create engine directory");
    let engine = DownloadEngine::open_in_memory().expect("open download engine");
    let task_id = engine
        .enqueue(
            request(&server, directory.path().to_path_buf()),
            RequestOrigin::Ui,
        )
        .expect("queue task without immediate admission");
    engine
        .set_queue_schedule(
            "default",
            Some(DownloadSchedule {
                daily: DownloadDailySchedule {
                    days: vec![DownloadWeekday::Monday],
                    start_minute: 9 * 60,
                    stop_minute: 17 * 60,
                    enabled_start: true,
                    enabled_stop: true,
                },
            }),
        )
        .expect("save daily queue schedule");

    engine
        .process_daily_schedules_at(DownloadScheduleMoment {
            weekday: DownloadWeekday::Monday,
            minute_of_day: 18 * 60,
        })
        .expect("close queue outside the daily window");
    engine
        .process_daily_schedules_at(DownloadScheduleMoment {
            weekday: DownloadWeekday::Monday,
            minute_of_day: 9 * 60,
        })
        .expect("open queue at start boundary");
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_ne!(
        engine
            .task(task_id)
            .expect("read scheduled task")
            .expect("task exists")
            .task
            .status(),
        DownloadStatus::Created
    );

    engine
        .process_daily_schedules_at(DownloadScheduleMoment {
            weekday: DownloadWeekday::Monday,
            minute_of_day: 17 * 60,
        })
        .expect("close queue at the stop boundary");
    let snapshot = engine.handle_command(DownloadCommandEnvelope {
        protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
        request_id: uuid::Uuid::new_v4(),
        origin: RequestOrigin::Ui,
        command: DownloadCommand::Snapshot {
            query: Default::default(),
        },
    });
    let DownloadCommandOutcome::Snapshot(snapshot) = snapshot.outcome else {
        panic!("snapshot expected");
    };
    assert!(!snapshot.queues[0].is_running);
}

#[tokio::test]
async fn manual_pause_is_not_overridden_by_the_next_daily_start() {
    let server = DeterministicHttpServer::start();
    let directory = tempfile::tempdir().expect("create engine directory");
    let engine = DownloadEngine::open_in_memory().expect("open download engine");
    let mut queued = request(&server, directory.path().to_path_buf());
    queued.speed_limit_bytes_per_second = Some(1);
    let task_id = engine
        .enqueue(queued, RequestOrigin::Ui)
        .expect("queue task");
    engine
        .set_queue_schedule(
            "default",
            Some(DownloadSchedule {
                daily: DownloadDailySchedule {
                    days: vec![DownloadWeekday::Monday],
                    start_minute: 9 * 60,
                    stop_minute: 17 * 60,
                    enabled_start: true,
                    enabled_stop: true,
                },
            }),
        )
        .expect("save schedule");
    engine
        .process_daily_schedules_at(DownloadScheduleMoment {
            weekday: DownloadWeekday::Monday,
            minute_of_day: 18 * 60,
        })
        .expect("close queue");
    engine
        .process_daily_schedules_at(DownloadScheduleMoment {
            weekday: DownloadWeekday::Monday,
            minute_of_day: 9 * 60,
        })
        .expect("start queue");
    tokio::time::sleep(Duration::from_millis(50)).await;
    engine.pause(task_id).expect("manually pause task");
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(
        engine
            .task(task_id)
            .expect("read manually paused task")
            .expect("task exists")
            .task
            .status(),
        DownloadStatus::Paused
    );

    engine
        .process_daily_schedules_at(DownloadScheduleMoment {
            weekday: DownloadWeekday::Monday,
            minute_of_day: 18 * 60,
        })
        .expect("close next window");
    engine
        .process_daily_schedules_at(DownloadScheduleMoment {
            weekday: DownloadWeekday::Monday,
            minute_of_day: 9 * 60,
        })
        .expect("open next window");
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(
        engine
            .task(task_id)
            .expect("read task after next start")
            .expect("task exists")
            .task
            .status(),
        DownloadStatus::Paused
    );
}

#[tokio::test]
async fn queue_move_and_batch_commands_preserve_order_and_report_each_target() {
    let server = DeterministicHttpServer::start();
    let directory = tempfile::tempdir().expect("create engine directory");
    let engine = DownloadEngine::open_in_memory().expect("open download engine");
    let changed = engine.handle_command(DownloadCommandEnvelope {
        protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
        request_id: uuid::Uuid::new_v4(),
        origin: RequestOrigin::Ui,
        command: DownloadCommand::Queue {
            action: DownloadQueueAction::SetMaxConcurrent {
                queue_id: "default".into(),
                max_concurrent_tasks: 1,
            },
        },
    });
    assert!(matches!(
        changed.outcome,
        DownloadCommandOutcome::QueueChanged { .. }
    ));
    let mut first_request = request(&server, directory.path().join("first"));
    first_request.speed_limit_bytes_per_second = Some(1);
    let first = engine
        .submit(first_request, RequestOrigin::Ui)
        .expect("submit first");
    let second = engine
        .submit(
            request(&server, directory.path().join("second")),
            RequestOrigin::Ui,
        )
        .expect("submit second");
    let third = engine
        .submit(
            request(&server, directory.path().join("third")),
            RequestOrigin::Ui,
        )
        .expect("submit third");

    let moved = engine.handle_command(DownloadCommandEnvelope {
        protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
        request_id: uuid::Uuid::new_v4(),
        origin: RequestOrigin::Ui,
        command: DownloadCommand::Queue {
            action: DownloadQueueAction::Move {
                queue_id: "default".into(),
                task_ids: vec![third, second],
                placement: DownloadQueuePlacement::Top,
            },
        },
    });
    assert!(matches!(
        moved.outcome,
        DownloadCommandOutcome::QueueChanged { .. }
    ));
    assert_eq!(
        engine.queue_order("default").expect("read queue order"),
        vec![second, third, first],
        "selected tasks keep their existing relative order"
    );

    let missing = nxtools_download::DownloadTaskId::new();
    let batch = engine.handle_command(DownloadCommandEnvelope {
        protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
        request_id: uuid::Uuid::new_v4(),
        origin: RequestOrigin::Ui,
        command: DownloadCommand::Batch {
            task_ids: vec![first, missing, second],
            action: DownloadBatchAction::Cancel,
        },
    });
    let DownloadCommandOutcome::Batch { results } = batch.outcome else {
        panic!("batch outcome expected");
    };
    assert_eq!(results.len(), 3);
    assert!(results[0].fault.is_none());
    assert!(results[1].fault.is_some());
    assert!(results[2].fault.is_none());
    engine.cancel(third).expect("cancel third");
    engine
        .wait_for_terminal(first, Duration::from_secs(2))
        .await
        .expect("wait for first cancellation");
    engine
        .wait_for_terminal(third, Duration::from_secs(2))
        .await
        .expect("wait for third cancellation");
}

#[tokio::test]
async fn queue_concurrency_admits_only_one_task_and_refills_released_slot() {
    let server = DeterministicHttpServer::start();
    let directory = tempfile::tempdir().expect("create engine directory");
    let engine = DownloadEngine::open_in_memory().expect("open download engine");
    let changed = engine.handle_command(DownloadCommandEnvelope {
        protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
        request_id: uuid::Uuid::new_v4(),
        origin: RequestOrigin::Ui,
        command: DownloadCommand::Queue {
            action: DownloadQueueAction::SetMaxConcurrent {
                queue_id: "default".into(),
                max_concurrent_tasks: 1,
            },
        },
    });
    assert!(matches!(
        changed.outcome,
        DownloadCommandOutcome::QueueChanged { .. }
    ));

    let mut first_request = request(&server, directory.path().join("first"));
    first_request.speed_limit_bytes_per_second = Some(1);
    let mut second_request = request(&server, directory.path().join("second"));
    second_request.speed_limit_bytes_per_second = Some(1);
    let first = engine
        .submit(first_request, RequestOrigin::Ui)
        .expect("submit first");
    let second = engine
        .submit(second_request, RequestOrigin::Ui)
        .expect("submit second");

    tokio::time::sleep(Duration::from_millis(80)).await;
    assert_ne!(
        engine
            .task(first)
            .expect("read first")
            .expect("first exists")
            .task
            .status(),
        DownloadStatus::Created
    );
    assert_eq!(
        engine
            .task(second)
            .expect("read second")
            .expect("second exists")
            .task
            .status(),
        DownloadStatus::Created
    );
    engine.pause(second).expect("pause pending task");
    assert_eq!(
        engine
            .task(second)
            .expect("read paused pending task")
            .expect("second exists")
            .task
            .status(),
        DownloadStatus::Paused
    );
    assert!(
        !engine
            .task(second)
            .expect("read paused pending task eligibility")
            .expect("second exists")
            .queue_eligible
    );
    engine
        .resume(second)
        .expect("mark pending task eligible again");
    assert!(
        engine
            .task(second)
            .expect("read resumed pending task eligibility")
            .expect("second exists")
            .queue_eligible
    );
    engine
        .pause(second)
        .expect("pause resumed task before it receives a slot");
    engine.cancel(first).expect("cancel first");
    engine
        .wait_for_terminal(first, Duration::from_secs(2))
        .await
        .expect("wait for first cancellation");
    tokio::time::sleep(Duration::from_millis(40)).await;
    assert_eq!(
        engine
            .task(second)
            .expect("read paused task after slot release")
            .expect("second exists")
            .task
            .status(),
        DownloadStatus::Paused,
        "a pending pause must remove the task from automatic admission"
    );
    engine.resume(second).expect("resume pending task");
    let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
    loop {
        let status = engine
            .task(second)
            .expect("read second")
            .expect("second exists")
            .task
            .status();
        if status != DownloadStatus::Paused {
            break;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "second task was not admitted after the first released its slot"
        );
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert!(
        !engine
            .task(second)
            .expect("read admitted task eligibility")
            .expect("second exists")
            .queue_eligible
    );
    engine.cancel(second).expect("cancel second");
    engine
        .wait_for_terminal(second, Duration::from_secs(2))
        .await
        .expect("wait for second cancellation");
}

#[tokio::test]
async fn reopening_engine_admits_a_durable_eligible_pending_task() {
    let server = DeterministicHttpServer::start();
    let directory = tempfile::tempdir().expect("create engine directory");
    let database = directory.path().join("downloads.sqlite3");
    let queued = DownloadTask::new(
        request(&server, directory.path().join("artifacts")),
        RequestOrigin::Ui,
        1_000,
    );
    let task_id = queued.id();
    {
        let store = SqliteDownloadStore::open(&database).expect("open store");
        store.save_task(&queued).expect("save queued task");
        store
            .append_to_queue("default", task_id, true)
            .expect("mark task eligible");
    }

    let engine = DownloadEngine::open(&database).expect("reopen engine");
    let completed = engine
        .wait_for_terminal(task_id, Duration::from_secs(2))
        .await
        .expect("wait for startup admission");
    assert_eq!(completed.task.status(), DownloadStatus::Completed);
}

#[tokio::test]
async fn queue_concurrency_is_scoped_per_queue() {
    let server = DeterministicHttpServer::start();
    let directory = tempfile::tempdir().expect("create engine directory");
    let database = directory.path().join("downloads.sqlite3");
    let mut default_request = request(&server, directory.path().join("default"));
    default_request.speed_limit_bytes_per_second = Some(1);
    let mut secondary_request = request(&server, directory.path().join("secondary"));
    secondary_request.speed_limit_bytes_per_second = Some(1);
    let default_task = DownloadTask::new(default_request, RequestOrigin::Ui, 1_000);
    let secondary_task = DownloadTask::new(secondary_request, RequestOrigin::Ui, 2_000);
    {
        let store = SqliteDownloadStore::open(&database).expect("open store");
        store.save_task(&default_task).expect("save default task");
        store
            .append_to_queue("default", default_task.id(), true)
            .expect("queue default task");
        store
            .save_task(&secondary_task)
            .expect("save secondary task");
        store
            .append_to_queue("secondary", secondary_task.id(), true)
            .expect("queue secondary task");
        store
            .set_queue_max_concurrent("secondary", 1)
            .expect("set secondary concurrency");
    }

    let engine = DownloadEngine::open(&database).expect("open engine");
    tokio::time::sleep(Duration::from_millis(80)).await;
    assert_ne!(
        engine
            .task(secondary_task.id())
            .expect("read secondary")
            .expect("secondary exists")
            .task
            .status(),
        DownloadStatus::Created,
        "activity in the default queue must not consume a secondary queue slot"
    );
    engine
        .cancel(default_task.id())
        .expect("cancel default task");
    engine
        .cancel(secondary_task.id())
        .expect("cancel secondary task");
    engine
        .wait_for_terminal(default_task.id(), Duration::from_secs(2))
        .await
        .expect("wait for default cancellation");
    engine
        .wait_for_terminal(secondary_task.id(), Duration::from_secs(2))
        .await
        .expect("wait for secondary cancellation");
}

#[test]
fn crash_helper_process() {
    if std::env::var_os("NXTOOLS_DOWNLOAD_CRASH_HELPER").is_none() {
        return;
    }
    let database_path =
        PathBuf::from(std::env::var_os("NXTOOLS_DOWNLOAD_CRASH_DB").expect("crash database path"));
    let destination = PathBuf::from(
        std::env::var_os("NXTOOLS_DOWNLOAD_CRASH_DESTINATION").expect("crash destination path"),
    );
    let source_url = std::env::var("NXTOOLS_DOWNLOAD_CRASH_URL").expect("crash fixture source URL");
    let engine = DownloadEngine::open(&database_path).expect("open crash-helper engine");
    let task_id = engine
        .submit(
            DownloadRequest {
                source_url,
                save_directory: destination,
                file_name: Some("forced-exit.bin".into()),
                headers: Vec::new(),
                cookie: None,
                referrer: None,
                connection_limit: Some(1),
                speed_limit_bytes_per_second: None,
                scheduled_at_unix_ms: None,
            },
            RequestOrigin::Internal,
        )
        .expect("submit crash-helper task");
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    loop {
        let task = engine
            .task(task_id)
            .expect("read crash-helper task")
            .expect("crash-helper task exists");
        if task.downloaded_bytes > 0
            && task
                .temporary_path
                .as_ref()
                .is_some_and(|path| path.exists())
        {
            std::process::exit(86);
        }
        assert!(
            std::time::Instant::now() < deadline,
            "crash-helper transfer did not start"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[tokio::test]
async fn one_engine_task_downloads_persists_and_reopens_as_completed() {
    let server = DeterministicHttpServer::start();
    let directory = tempfile::tempdir().expect("create engine directory");
    let database_path = directory.path().join("downloads.sqlite3");
    let destination = directory.path().join("artifacts");
    let engine = DownloadEngine::open(&database_path).expect("open download engine");

    let task_id = engine
        .submit(request(&server, destination.clone()), RequestOrigin::Ui)
        .expect("submit task");
    let completed = engine
        .wait_for_terminal(task_id, Duration::from_secs(2))
        .await
        .expect("wait for terminal task");

    assert_eq!(
        completed.task.status(),
        DownloadStatus::Completed,
        "download failed: {:?}",
        completed.error
    );
    assert_eq!(completed.downloaded_bytes, 16);
    assert_eq!(completed.total_bytes, Some(16));
    assert_eq!(
        std::fs::read(destination.join("engine-fixture.bin")).expect("read completed download"),
        b"0123456789abcdef"
    );
    drop(engine);

    let reopened = DownloadEngine::open(&database_path).expect("reopen download engine");
    let restored = reopened
        .task(task_id)
        .expect("load task")
        .expect("persisted task exists");
    assert_eq!(restored.task.status(), DownloadStatus::Completed);
    assert_eq!(restored.downloaded_bytes, 16);
    assert_eq!(restored.total_bytes, Some(16));
}

#[tokio::test]
async fn paused_task_keeps_its_identity_and_can_restart_to_completion() {
    let server = DeterministicHttpServer::start();
    let directory = tempfile::tempdir().expect("create engine directory");
    let destination = directory.path().join("artifacts");
    let engine = DownloadEngine::open_in_memory().expect("open download engine");
    let mut slow_request = request(&server, destination.clone());
    slow_request.source_url = format!("http://{}/stall", server.address());

    let task_id = engine
        .submit(slow_request, RequestOrigin::Ui)
        .expect("submit slow task");
    wait_for_progress(&engine, task_id).await;
    engine.pause(task_id).expect("request pause");
    let paused = engine
        .wait_for_terminal(task_id, Duration::from_secs(2))
        .await
        .expect("wait for paused task");

    assert_eq!(paused.task.status(), DownloadStatus::Paused);
    assert!(paused.downloaded_bytes > 0);
    assert!(!destination.join("engine-fixture.bin").exists());

    engine.resume(task_id).expect("resume paused task");
    let completed = engine
        .wait_for_terminal(task_id, Duration::from_secs(2))
        .await
        .expect("wait for resumed task");
    assert_eq!(completed.task.id(), task_id);
    assert_eq!(completed.task.status(), DownloadStatus::Completed);
    assert_eq!(completed.downloaded_bytes, 16);
}

#[tokio::test]
async fn ranged_task_resumes_persisted_bytes_after_the_engine_is_reopened() {
    let server = DeterministicHttpServer::start();
    let directory = tempfile::tempdir().expect("create engine directory");
    let database_path = directory.path().join("downloads.sqlite3");
    let destination = directory.path().join("artifacts");
    let engine = DownloadEngine::open(&database_path).expect("open download engine");
    let mut ranged_request = request(&server, destination.clone());
    ranged_request.source_url = format!("http://{}/range-resume-required", server.address());

    let task_id = engine
        .submit(ranged_request, RequestOrigin::Ui)
        .expect("submit ranged task");
    wait_for_progress(&engine, task_id).await;
    engine.pause(task_id).expect("request pause");
    let paused = engine
        .wait_for_terminal(task_id, Duration::from_secs(2))
        .await
        .expect("wait for paused task");
    assert_eq!(paused.task.status(), DownloadStatus::Paused);
    assert!(paused.downloaded_bytes > 0);
    let checkpoint = paused.downloaded_bytes;
    drop(engine);

    let reopened = DownloadEngine::open(&database_path).expect("reopen download engine");
    let restored = reopened
        .task(task_id)
        .expect("read task")
        .expect("persisted task exists");
    assert_eq!(restored.task.status(), DownloadStatus::Paused);
    assert_eq!(restored.downloaded_bytes, checkpoint);

    reopened.resume(task_id).expect("resume persisted task");
    let completed = reopened
        .wait_for_terminal(task_id, Duration::from_secs(2))
        .await
        .expect("wait for resumed task");
    assert_eq!(completed.task.status(), DownloadStatus::Completed);
    assert_eq!(completed.downloaded_bytes, 16);
    assert_eq!(
        std::fs::read(destination.join("engine-fixture.bin")).expect("read resumed artifact"),
        b"0123456789abcdef"
    );
}

#[tokio::test]
async fn range_ignored_after_probe_restarts_cleanly_as_a_single_stream() {
    let server = DeterministicHttpServer::start();
    let directory = tempfile::tempdir().expect("create engine directory");
    let destination = directory.path().join("artifacts");
    let engine = DownloadEngine::open_in_memory().expect("open download engine");
    let mut request = request(&server, destination.clone());
    request.source_url = format!("http://{}/range-probe-only", server.address());

    let task_id = engine
        .submit(request, RequestOrigin::Ui)
        .expect("submit task");
    let completed = engine
        .wait_for_terminal(task_id, Duration::from_secs(2))
        .await
        .expect("wait for fallback task");

    assert_eq!(completed.task.status(), DownloadStatus::Completed);
    assert_eq!(completed.downloaded_bytes, 16);
    assert_eq!(
        std::fs::read(destination.join("engine-fixture.bin")).expect("read fallback artifact"),
        b"0123456789abcdef"
    );
}

#[tokio::test]
async fn changed_resource_after_a_checkpoint_fails_without_publishing_or_discarding_progress() {
    let server = DeterministicHttpServer::start();
    let directory = tempfile::tempdir().expect("create engine directory");
    let database_path = directory.path().join("downloads.sqlite3");
    let destination = directory.path().join("artifacts");
    let engine = DownloadEngine::open(&database_path).expect("open download engine");
    let mut request = request(&server, destination.clone());
    request.source_url = format!("http://{}/range-mutable", server.address());

    let task_id = engine
        .submit(request, RequestOrigin::Ui)
        .expect("submit mutable task");
    wait_for_progress(&engine, task_id).await;
    engine.pause(task_id).expect("pause mutable task");
    let paused = engine
        .wait_for_terminal(task_id, Duration::from_secs(2))
        .await
        .expect("wait for paused task");
    let checkpoint = paused.downloaded_bytes;
    let temporary_path = paused
        .temporary_path
        .clone()
        .expect("temporary artifact is tracked");
    assert!(checkpoint > 0);
    server.advance_resource_revision();
    drop(engine);

    let reopened = DownloadEngine::open(&database_path).expect("reopen download engine");
    reopened.resume(task_id).expect("resume mutable task");
    let failed = reopened
        .wait_for_terminal(task_id, Duration::from_secs(2))
        .await
        .expect("wait for rejected resume");

    assert_eq!(failed.task.status(), DownloadStatus::Failed);
    assert_eq!(failed.downloaded_bytes, checkpoint);
    assert!(temporary_path.exists());
    assert!(!destination.join("engine-fixture.bin").exists());
}

#[tokio::test]
async fn crash_window_bytes_ahead_of_sqlite_checkpoint_are_safely_overwritten_on_resume() {
    let server = DeterministicHttpServer::start();
    let directory = tempfile::tempdir().expect("create crash recovery directory");
    let database_path = directory.path().join("downloads.sqlite3");
    let destination = directory.path().join("artifacts");
    let engine = DownloadEngine::open(&database_path).expect("open download engine");
    let mut ranged_request = request(&server, destination.clone());
    ranged_request.source_url = format!("http://{}/range-resume-required", server.address());

    let task_id = engine
        .submit(ranged_request, RequestOrigin::Ui)
        .expect("submit crash-window task");
    wait_for_progress(&engine, task_id).await;
    engine.pause(task_id).expect("pause at durable checkpoint");
    let paused = engine
        .wait_for_terminal(task_id, Duration::from_secs(2))
        .await
        .expect("wait for durable pause");
    let checkpoint = paused.downloaded_bytes;
    let temporary_path = paused
        .temporary_path
        .clone()
        .expect("temporary artifact is tracked");
    assert!(checkpoint > 0 && checkpoint < 16);
    drop(engine);

    let mut artifact = std::fs::OpenOptions::new()
        .write(true)
        .open(&temporary_path)
        .expect("open crash-window artifact");
    artifact
        .seek(std::io::SeekFrom::Start(checkpoint))
        .expect("seek past durable checkpoint");
    artifact
        .write_all(&vec![0xff; 16 - checkpoint as usize])
        .expect("write uncheckpointed crash-window bytes");
    artifact.sync_all().expect("sync simulated crash bytes");
    drop(artifact);

    let connection = rusqlite::Connection::open(&database_path).expect("open crash database");
    connection
        .execute(
            "UPDATE download_tasks SET status = 'downloading' WHERE task_id = ?1",
            [task_id.as_uuid().to_string()],
        )
        .expect("restore interrupted task state");
    connection
        .execute(
            "UPDATE download_segments SET state = 'active' WHERE task_id = ?1",
            [task_id.as_uuid().to_string()],
        )
        .expect("restore interrupted segment state");
    drop(connection);

    let reopened = DownloadEngine::open(&database_path).expect("recover crashed engine");
    let recovered = reopened
        .task(task_id)
        .expect("read recovered task")
        .expect("recovered task exists");
    assert_eq!(recovered.task.status(), DownloadStatus::Paused);
    assert_eq!(recovered.downloaded_bytes, checkpoint);

    reopened.resume(task_id).expect("resume recovered task");
    let completed = reopened
        .wait_for_terminal(task_id, Duration::from_secs(2))
        .await
        .expect("wait for crash recovery");
    assert_eq!(
        completed.task.status(),
        DownloadStatus::Completed,
        "crash recovery failed: {:?}",
        completed.error
    );
    assert_eq!(
        std::fs::read(destination.join("engine-fixture.bin")).expect("read recovered artifact"),
        b"0123456789abcdef"
    );
}

#[tokio::test]
async fn forcibly_terminated_download_process_recovers_and_resumes_from_sqlite() {
    let server = DeterministicHttpServer::start();
    let directory = tempfile::tempdir().expect("create forced-exit directory");
    let database_path = directory.path().join("downloads.sqlite3");
    let destination = directory.path().join("artifacts");
    let status = Command::new(std::env::current_exe().expect("locate contract test process"))
        .args([
            "--exact",
            "crash_helper_process",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("NXTOOLS_DOWNLOAD_CRASH_HELPER", "1")
        .env("NXTOOLS_DOWNLOAD_CRASH_DB", &database_path)
        .env("NXTOOLS_DOWNLOAD_CRASH_DESTINATION", &destination)
        .env(
            "NXTOOLS_DOWNLOAD_CRASH_URL",
            format!("http://{}/range-disconnect", server.address()),
        )
        .status()
        .expect("run crash-helper process");
    assert_eq!(
        status.code(),
        Some(86),
        "helper must terminate without running Rust destructors"
    );

    let engine = DownloadEngine::open(&database_path).expect("recover force-terminated engine");
    let snapshot = engine.handle_command(DownloadCommandEnvelope {
        protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
        request_id: uuid::Uuid::new_v4(),
        origin: RequestOrigin::Internal,
        command: DownloadCommand::Snapshot {
            query: Default::default(),
        },
    });
    let DownloadCommandOutcome::Snapshot(snapshot) = snapshot.outcome else {
        panic!("recovered engine must return a snapshot");
    };
    let recovered = snapshot
        .task_details
        .into_iter()
        .find(|task| task.task.request().file_name.as_deref() == Some("forced-exit.bin"))
        .expect("recovered forced-exit task");
    assert_eq!(recovered.task.status(), DownloadStatus::Paused);
    assert_eq!(recovered.downloaded_bytes, 0);
    let temporary_path = recovered
        .temporary_path
        .as_ref()
        .expect("forced-exit artifact is tracked");
    assert!(temporary_path.exists());
    assert!(!destination.join("forced-exit.bin").exists());

    engine
        .resume(recovered.task.id())
        .expect("resume force-terminated task");
    let completed = engine
        .wait_for_terminal(recovered.task.id(), Duration::from_secs(5))
        .await
        .expect("wait for forced-exit recovery");
    assert_eq!(
        completed.task.status(),
        DownloadStatus::Completed,
        "forced-exit recovery failed: {:?}",
        completed.error
    );
    assert_eq!(
        std::fs::read(destination.join("forced-exit.bin")).expect("read force-terminated recovery"),
        concurrent_fixture_bytes()
    );
}

#[tokio::test]
async fn task_can_resume_after_network_disconnect_exhausts_the_current_attempt() {
    let server = DeterministicHttpServer::start();
    let directory = tempfile::tempdir().expect("create disconnect recovery directory");
    let destination = directory.path().join("artifacts");
    let engine = DownloadEngine::open_in_memory().expect("open download engine");
    let mut disconnected_request = request(&server, destination.clone());
    disconnected_request.source_url = format!("http://{}/range-disconnect", server.address());
    disconnected_request.file_name = Some("disconnect.bin".into());

    let task_id = engine
        .submit(disconnected_request, RequestOrigin::Ui)
        .expect("submit disconnect task");
    wait_for_progress(&engine, task_id).await;
    server.set_network_available(false);
    let failed = engine
        .wait_for_terminal(task_id, Duration::from_secs(5))
        .await
        .expect("wait for disconnected task failure");
    assert_eq!(failed.task.status(), DownloadStatus::Failed);
    assert_eq!(failed.downloaded_bytes, 0);
    assert!(!destination.join("disconnect.bin").exists());

    server.set_network_available(true);
    engine.resume(task_id).expect("resume after reconnect");
    let completed = engine
        .wait_for_terminal(task_id, Duration::from_secs(5))
        .await
        .expect("wait for reconnected task");
    assert_eq!(
        completed.task.status(),
        DownloadStatus::Completed,
        "reconnect recovery failed: {:?}",
        completed.error
    );
    assert_eq!(
        std::fs::read(destination.join("disconnect.bin")).expect("read reconnected artifact"),
        concurrent_fixture_bytes()
    );
}

#[tokio::test]
async fn connection_limit_runs_multiple_ranges_concurrently_into_one_verified_file() {
    let server = DeterministicHttpServer::start_concurrent();
    let directory = tempfile::tempdir().expect("create engine directory");
    let destination = directory.path().join("artifacts");
    let engine = DownloadEngine::open_in_memory().expect("open download engine");
    let mut request = request(&server, destination.clone());
    request.source_url = format!("http://{}/range-concurrent", server.address());
    request.connection_limit = Some(4);

    let task_id = engine
        .submit(request, RequestOrigin::Ui)
        .expect("submit concurrent task");
    let completed = engine
        .wait_for_terminal(task_id, Duration::from_secs(4))
        .await
        .expect("wait for concurrent task");

    assert_eq!(
        completed.task.status(),
        DownloadStatus::Completed,
        "concurrent transfer failed: {:?}",
        completed.error
    );
    assert_eq!(
        std::fs::read(destination.join("engine-fixture.bin")).expect("read concurrent artifact"),
        concurrent_fixture_bytes()
    );
    assert!(
        server.max_concurrent_range_transfers() >= 2,
        "connection limit above one must produce overlapping Range transfers"
    );
}

#[tokio::test]
async fn fresh_multi_connection_download_avoids_if_range_incompatible_cdn_fallback() {
    let server = DeterministicHttpServer::start_concurrent();
    let directory = tempfile::tempdir().expect("create engine directory");
    let destination = directory.path().join("artifacts");
    let engine = DownloadEngine::open_in_memory().expect("open download engine");
    let mut request = request(&server, destination.clone());
    request.source_url = format!(
        "http://{}/range-concurrent-if-range-incompatible",
        server.address()
    );
    request.connection_limit = Some(4);

    let task_id = engine
        .submit(request, RequestOrigin::Ui)
        .expect("submit conditional-incompatible task");
    let completed = engine
        .wait_for_terminal(task_id, Duration::from_secs(4))
        .await
        .expect("wait for conditional-incompatible task");

    assert_eq!(
        completed.task.status(),
        DownloadStatus::Completed,
        "fresh multi-connection transfer failed: {:?}",
        completed.error
    );
    assert_eq!(
        std::fs::read(destination.join("engine-fixture.bin"))
            .expect("read conditional-incompatible artifact"),
        concurrent_fixture_bytes()
    );
    assert!(
        server.max_concurrent_range_transfers() >= 2,
        "fresh transfer must keep overlapping ranges instead of falling back to one stream"
    );
}

#[tokio::test]
async fn one_task_speed_limit_is_shared_by_all_of_its_range_workers() {
    let server = DeterministicHttpServer::start_concurrent();
    let directory = tempfile::tempdir().expect("create engine directory");
    let destination = directory.path().join("artifacts");
    let engine = DownloadEngine::open_in_memory().expect("open download engine");
    let mut request = request(&server, destination.clone());
    request.source_url = format!("http://{}/range-concurrent", server.address());
    request.connection_limit = Some(4);
    request.speed_limit_bytes_per_second = Some(256 * 1024);
    let started = std::time::Instant::now();

    let task_id = engine
        .submit(request, RequestOrigin::Ui)
        .expect("submit rate-limited task");
    let completed = engine
        .wait_for_terminal(task_id, Duration::from_secs(4))
        .await
        .expect("wait for rate-limited task");

    assert_eq!(
        completed.task.status(),
        DownloadStatus::Completed,
        "rate-limited transfer failed: {:?}",
        completed.error
    );
    assert!(
        started.elapsed() >= Duration::from_millis(750),
        "four workers must share the task limit instead of each receiving the full budget"
    );
    assert_eq!(
        std::fs::read(destination.join("engine-fixture.bin")).expect("read rate-limited artifact"),
        concurrent_fixture_bytes()
    );
}

#[tokio::test]
async fn one_global_speed_limit_is_shared_by_all_active_tasks() {
    let server = DeterministicHttpServer::start_concurrent();
    let directory = tempfile::tempdir().expect("create engine directory");
    let engine = DownloadEngine::open_in_memory().expect("open download engine");
    engine
        .set_global_speed_limit(Some(512 * 1024))
        .expect("set global speed limit");
    let started = std::time::Instant::now();

    let mut first = request(&server, directory.path().join("first"));
    first.source_url = format!("http://{}/range-concurrent", server.address());
    first.connection_limit = Some(4);
    let mut second = request(&server, directory.path().join("second"));
    second.source_url = format!("http://{}/range-concurrent", server.address());
    second.connection_limit = Some(4);
    let first_id = engine
        .submit(first, RequestOrigin::Ui)
        .expect("submit first globally limited task");
    let second_id = engine
        .submit(second, RequestOrigin::Ui)
        .expect("submit second globally limited task");

    let (first_task, second_task) = tokio::join!(
        engine.wait_for_terminal(first_id, Duration::from_secs(4)),
        engine.wait_for_terminal(second_id, Duration::from_secs(4))
    );
    assert_eq!(
        first_task.expect("wait for first task").task.status(),
        DownloadStatus::Completed
    );
    assert_eq!(
        second_task.expect("wait for second task").task.status(),
        DownloadStatus::Completed
    );
    assert!(
        started.elapsed() >= Duration::from_millis(750),
        "active tasks must share the global budget"
    );
}

#[tokio::test]
async fn changing_a_running_task_speed_limit_takes_effect_without_restarting_it() {
    let server = DeterministicHttpServer::start_concurrent();
    let directory = tempfile::tempdir().expect("create engine directory");
    let destination = directory.path().join("artifacts");
    let engine = DownloadEngine::open_in_memory().expect("open download engine");
    let mut request = request(&server, destination);
    request.source_url = format!("http://{}/range-concurrent", server.address());
    request.connection_limit = Some(4);
    request.speed_limit_bytes_per_second = Some(32 * 1024);
    let task_id = engine
        .submit(request, RequestOrigin::Ui)
        .expect("submit slowly limited task");
    wait_for_progress(&engine, task_id).await;
    let changed_at = std::time::Instant::now();

    engine
        .set_task_speed_limit(task_id, None)
        .expect("remove running task limit");
    let completed = engine
        .wait_for_terminal(task_id, Duration::from_secs(3))
        .await
        .expect("wait after live task limit update");

    assert_eq!(completed.task.status(), DownloadStatus::Completed);
    assert!(
        changed_at.elapsed() < Duration::from_secs(2),
        "removing a task limit must wake current workers without a restart"
    );
    assert_eq!(completed.task.request().speed_limit_bytes_per_second, None);
}

#[tokio::test]
async fn a_single_stream_download_uses_the_same_task_speed_limit() {
    let server = DeterministicHttpServer::start();
    let directory = tempfile::tempdir().expect("create engine directory");
    let destination = directory.path().join("artifacts");
    let engine = DownloadEngine::open_in_memory().expect("open download engine");
    let mut request = request(&server, destination);
    request.source_url = format!("http://{}/no-range", server.address());
    request.speed_limit_bytes_per_second = Some(16);
    let started = std::time::Instant::now();

    let task_id = engine
        .submit(request, RequestOrigin::Ui)
        .expect("submit limited single stream");
    let completed = engine
        .wait_for_terminal(task_id, Duration::from_secs(3))
        .await
        .expect("wait for limited single stream");

    assert_eq!(completed.task.status(), DownloadStatus::Completed);
    assert!(
        started.elapsed() >= Duration::from_millis(800),
        "single-stream downloads must pass through the task limiter"
    );
}

#[tokio::test]
async fn pause_interrupts_a_worker_waiting_for_bandwidth() {
    let server = DeterministicHttpServer::start_concurrent();
    let directory = tempfile::tempdir().expect("create engine directory");
    let destination = directory.path().join("artifacts");
    let engine = DownloadEngine::open_in_memory().expect("open download engine");
    let mut request = request(&server, destination);
    request.source_url = format!("http://{}/range-concurrent", server.address());
    request.connection_limit = Some(4);
    request.speed_limit_bytes_per_second = Some(8 * 1024);
    let task_id = engine
        .submit(request, RequestOrigin::Ui)
        .expect("submit bandwidth-waiting task");
    wait_for_progress(&engine, task_id).await;
    let paused_at = std::time::Instant::now();

    engine.pause(task_id).expect("pause bandwidth-waiting task");
    let paused = engine
        .wait_for_terminal(task_id, Duration::from_secs(1))
        .await
        .expect("bandwidth wait must observe pause");

    assert_eq!(paused.task.status(), DownloadStatus::Paused);
    assert!(
        paused_at.elapsed() < Duration::from_millis(500),
        "pause must not wait for the next bandwidth permit"
    );
}

#[tokio::test]
async fn global_and_task_speed_limits_survive_engine_restart() {
    let directory = tempfile::tempdir().expect("create engine directory");
    let database_path = directory.path().join("downloads.sqlite3");
    let engine = DownloadEngine::open(&database_path).expect("open download engine");
    engine
        .set_global_speed_limit(Some(768 * 1024))
        .expect("persist global limit");
    let task_id = engine
        .enqueue(
            DownloadRequest {
                source_url: "https://example.com/archive.bin".to_owned(),
                save_directory: directory.path().join("artifacts"),
                file_name: Some("archive.bin".to_owned()),
                headers: Vec::new(),
                cookie: None,
                referrer: None,
                connection_limit: Some(4),
                speed_limit_bytes_per_second: None,
                scheduled_at_unix_ms: None,
            },
            RequestOrigin::Ui,
        )
        .expect("create queued task");
    engine
        .set_task_speed_limit(task_id, Some(128 * 1024))
        .expect("persist task limit");
    drop(engine);

    let reopened = DownloadEngine::open(&database_path).expect("reopen download engine");
    assert_eq!(reopened.global_speed_limit(), Some(768 * 1024));
    assert_eq!(
        reopened
            .task(task_id)
            .expect("read restored task")
            .expect("restored task exists")
            .task
            .request()
            .speed_limit_bytes_per_second,
        Some(128 * 1024)
    );
}

#[tokio::test]
async fn dynamic_rebalance_splits_a_slow_tail_without_overlapping_output() {
    let server = DeterministicHttpServer::start_concurrent();
    let directory = tempfile::tempdir().expect("create engine directory");
    let destination = directory.path().join("artifacts");
    let engine = DownloadEngine::open_in_memory().expect("open download engine");
    let mut request = request(&server, destination.clone());
    request.source_url = format!("http://{}/range-imbalanced", server.address());
    request.connection_limit = Some(2);

    let task_id = engine
        .submit(request, RequestOrigin::Ui)
        .expect("submit imbalanced task");
    let completed = engine
        .wait_for_terminal(task_id, Duration::from_secs(6))
        .await
        .expect("wait for dynamically balanced task");

    assert_eq!(
        completed.task.status(),
        DownloadStatus::Completed,
        "dynamic transfer failed: {:?}",
        completed.error
    );
    assert_eq!(
        std::fs::read(destination.join("engine-fixture.bin"))
            .expect("read dynamically balanced artifact"),
        concurrent_fixture_bytes()
    );
    assert!(
        server.dynamic_range_attempts() >= 4,
        "a released connection slot must split the slow unwritten tail; attempts={}",
        server.dynamic_range_attempts()
    );
}

#[tokio::test]
async fn a_dynamically_split_plan_resumes_after_engine_restart() {
    let server = DeterministicHttpServer::start_concurrent();
    let directory = tempfile::tempdir().expect("create engine directory");
    let database_path = directory.path().join("downloads.sqlite3");
    let destination = directory.path().join("artifacts");
    let engine = DownloadEngine::open(&database_path).expect("open download engine");
    let mut request = request(&server, destination.clone());
    request.source_url = format!("http://{}/range-imbalanced", server.address());
    request.connection_limit = Some(2);

    let task_id = engine
        .submit(request, RequestOrigin::Ui)
        .expect("submit imbalanced task");
    let split_deadline = tokio::time::Instant::now() + Duration::from_secs(2);
    while server.dynamic_range_attempts() < 4 {
        assert!(
            tokio::time::Instant::now() < split_deadline,
            "dynamic split did not start before pause"
        );
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    engine.pause(task_id).expect("pause dynamically split task");
    let paused = engine
        .wait_for_terminal(task_id, Duration::from_secs(2))
        .await
        .expect("wait for dynamic pause");
    assert_eq!(paused.task.status(), DownloadStatus::Paused);
    assert!(paused.downloaded_bytes > 0);
    assert!(paused.downloaded_bytes < concurrent_fixture_bytes().len() as u64);
    drop(engine);

    let reopened = DownloadEngine::open(&database_path).expect("reopen download engine");
    reopened
        .resume(task_id)
        .expect("resume dynamically split task");
    let completed = reopened
        .wait_for_terminal(task_id, Duration::from_secs(6))
        .await
        .expect("wait for dynamically split resume");

    assert_eq!(
        completed.task.status(),
        DownloadStatus::Completed,
        "dynamic resume failed: {:?}",
        completed.error
    );
    assert_eq!(
        std::fs::read(destination.join("engine-fixture.bin"))
            .expect("read resumed dynamic artifact"),
        concurrent_fixture_bytes()
    );
}

#[tokio::test]
async fn concurrent_ranges_resume_all_persisted_segments_after_engine_restart() {
    let server = DeterministicHttpServer::start_concurrent();
    let directory = tempfile::tempdir().expect("create engine directory");
    let database_path = directory.path().join("downloads.sqlite3");
    let destination = directory.path().join("artifacts");
    let engine = DownloadEngine::open(&database_path).expect("open download engine");
    let mut request = request(&server, destination.clone());
    request.source_url = format!("http://{}/range-concurrent", server.address());
    request.connection_limit = Some(4);

    let task_id = engine
        .submit(request, RequestOrigin::Ui)
        .expect("submit concurrent task");
    wait_for_progress(&engine, task_id).await;
    engine.pause(task_id).expect("pause concurrent task");
    let paused = engine
        .wait_for_terminal(task_id, Duration::from_secs(4))
        .await
        .expect("wait for concurrent pause");
    assert_eq!(paused.task.status(), DownloadStatus::Paused);
    assert!(paused.downloaded_bytes >= 4);
    assert!(paused.downloaded_bytes < concurrent_fixture_bytes().len() as u64);
    let checkpoint = paused.downloaded_bytes;
    drop(engine);

    let reopened = DownloadEngine::open(&database_path).expect("reopen download engine");
    let restored = reopened
        .task(task_id)
        .expect("read task")
        .expect("persisted task exists");
    assert_eq!(restored.task.status(), DownloadStatus::Paused);
    assert_eq!(restored.downloaded_bytes, checkpoint);

    reopened.resume(task_id).expect("resume concurrent task");
    let completed = reopened
        .wait_for_terminal(task_id, Duration::from_secs(4))
        .await
        .expect("wait for concurrent resume");
    assert_eq!(
        completed.task.status(),
        DownloadStatus::Completed,
        "concurrent resume failed: {:?}",
        completed.error
    );
    assert_eq!(
        std::fs::read(destination.join("engine-fixture.bin")).expect("read resumed artifact"),
        concurrent_fixture_bytes()
    );
}

#[tokio::test]
async fn transient_range_failures_retry_without_losing_the_task_or_artifact() {
    let server = DeterministicHttpServer::start();
    let directory = tempfile::tempdir().expect("create engine directory");
    let destination = directory.path().join("artifacts");
    let engine = DownloadEngine::open_in_memory().expect("open download engine");
    let mut request = request(&server, destination.clone());
    request.source_url = format!("http://{}/range-transient", server.address());

    let task_id = engine
        .submit(request, RequestOrigin::Ui)
        .expect("submit transient task");
    let completed = engine
        .wait_for_terminal(task_id, Duration::from_secs(4))
        .await
        .expect("wait for retried task");

    assert_eq!(
        completed.task.status(),
        DownloadStatus::Completed,
        "transient transfer failed after {} attempts: {:?}",
        server.transient_range_attempts(),
        completed.error,
    );
    assert_eq!(server.transient_range_attempts(), 2);
    assert_eq!(
        std::fs::read(destination.join("engine-fixture.bin")).expect("read retried artifact"),
        b"0123456789abcdef"
    );
}

#[tokio::test]
async fn pausing_during_retry_backoff_stops_new_network_attempts() {
    let server = DeterministicHttpServer::start();
    let directory = tempfile::tempdir().expect("create engine directory");
    let destination = directory.path().join("artifacts");
    let engine = DownloadEngine::open_in_memory().expect("open download engine");
    let mut request = request(&server, destination);
    request.source_url = format!("http://{}/range-always-transient", server.address());

    let task_id = engine
        .submit(request, RequestOrigin::Ui)
        .expect("submit retrying task");
    let deadline = std::time::Instant::now() + Duration::from_secs(1);
    while server.transient_range_attempts() == 0 {
        assert!(
            std::time::Instant::now() < deadline,
            "first transient attempt did not arrive"
        );
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    engine.pause(task_id).expect("pause during retry backoff");
    let paused = engine
        .wait_for_terminal(task_id, Duration::from_secs(1))
        .await
        .expect("wait for retry pause");

    assert_eq!(paused.task.status(), DownloadStatus::Paused);
    assert!(
        server.transient_range_attempts() < 3,
        "pause must interrupt backoff before the retry budget is exhausted"
    );
}

#[tokio::test]
async fn permanent_range_failure_is_not_retried() {
    let server = DeterministicHttpServer::start();
    let directory = tempfile::tempdir().expect("create engine directory");
    let destination = directory.path().join("artifacts");
    let engine = DownloadEngine::open_in_memory().expect("open download engine");
    let mut request = request(&server, destination);
    request.source_url = format!("http://{}/range-permanent", server.address());

    let task_id = engine
        .submit(request, RequestOrigin::Ui)
        .expect("submit permanent failure task");
    let failed = engine
        .wait_for_terminal(task_id, Duration::from_secs(1))
        .await
        .expect("wait for permanent failure");

    assert_eq!(failed.task.status(), DownloadStatus::Failed);
    assert_eq!(
        server.transient_range_attempts(),
        1,
        "permanent HTTP failures must not consume the retry budget"
    );
}

#[tokio::test]
async fn cancelled_task_cleans_temporary_work_and_can_be_removed_from_history() {
    let server = DeterministicHttpServer::start();
    let directory = tempfile::tempdir().expect("create engine directory");
    let destination = directory.path().join("artifacts");
    let engine = DownloadEngine::open_in_memory().expect("open download engine");
    let mut slow_request = request(&server, destination.clone());
    slow_request.source_url = format!("http://{}/stall", server.address());

    let task_id = engine
        .submit(slow_request, RequestOrigin::Ui)
        .expect("submit slow task");
    wait_for_progress(&engine, task_id).await;
    engine.cancel(task_id).expect("cancel task");
    let cancelled = engine
        .wait_for_terminal(task_id, Duration::from_secs(2))
        .await
        .expect("wait for cancelled task");

    assert_eq!(cancelled.task.status(), DownloadStatus::Cancelled);
    assert!(!destination.join("engine-fixture.bin").exists());
    assert_eq!(
        std::fs::read_dir(&destination)
            .expect("read destination")
            .count(),
        0
    );

    engine
        .remove(task_id, false)
        .expect("remove cancelled task");
    assert!(engine.task(task_id).expect("read task").is_none());
}

#[tokio::test]
async fn opening_the_engine_recovers_and_resumes_an_orphan_interrupted_task() {
    let server = DeterministicHttpServer::start();
    let directory = tempfile::tempdir().expect("create engine directory");
    let database_path = directory.path().join("downloads.sqlite3");
    let store = SqliteDownloadStore::open(&database_path).expect("open task store");
    let mut task = DownloadTask::new(
        request(&server, directory.path().join("artifacts")),
        RequestOrigin::Ui,
        1_000,
    );
    for status in [
        DownloadStatus::Probing,
        DownloadStatus::Waiting,
        DownloadStatus::Downloading,
    ] {
        task.transition_to(status, 1_100)
            .expect("build interrupted task");
    }
    store.save_task(&task).expect("persist interrupted task");
    drop(store);

    let engine = DownloadEngine::open(&database_path).expect("open recovered engine");
    let recovered = engine
        .task(task.id())
        .expect("read task")
        .expect("recovered task exists");

    assert_eq!(recovered.task.status(), DownloadStatus::Paused);
    assert!(!recovered.queue_eligible);
    engine
        .resume(task.id())
        .expect("resume reconciled recovered task");
    let completed = engine
        .wait_for_terminal(task.id(), Duration::from_secs(2))
        .await
        .expect("wait for reconciled task");
    assert_eq!(completed.task.status(), DownloadStatus::Completed);
}

#[tokio::test]
async fn server_derived_artifact_paths_survive_restart_and_cancel_cleans_the_exact_temp_file() {
    let server = DeterministicHttpServer::start();
    let directory = tempfile::tempdir().expect("create engine directory");
    let database_path = directory.path().join("downloads.sqlite3");
    let destination = directory.path().join("artifacts");
    let engine = DownloadEngine::open(&database_path).expect("open download engine");
    let mut slow_request = request(&server, destination.clone());
    slow_request.source_url = format!("http://{}/stall", server.address());
    slow_request.file_name = None;

    let task_id = engine
        .submit(slow_request, RequestOrigin::Ui)
        .expect("submit slow task");
    wait_for_progress(&engine, task_id).await;
    engine.pause(task_id).expect("pause task");
    let paused = engine
        .wait_for_terminal(task_id, Duration::from_secs(2))
        .await
        .expect("wait for pause");
    let temporary_path = paused
        .temporary_path
        .clone()
        .expect("temporary artifact path is tracked");
    assert!(temporary_path.exists());
    drop(engine);

    let reopened = DownloadEngine::open(&database_path).expect("reopen download engine");
    let restored = reopened
        .task(task_id)
        .expect("read restored task")
        .expect("restored task exists");
    assert_eq!(restored.final_path, Some(destination.join("stall")));
    assert_eq!(
        restored.temporary_path.as_deref(),
        Some(temporary_path.as_path())
    );
    reopened.cancel(task_id).expect("cancel restored task");
    assert!(!temporary_path.exists());
}

#[tokio::test]
async fn an_existing_destination_fails_without_overwriting_the_file() {
    let server = DeterministicHttpServer::start();
    let directory = tempfile::tempdir().expect("create engine directory");
    let destination = directory.path().join("artifacts");
    std::fs::create_dir_all(&destination).expect("create destination");
    let final_path = destination.join("engine-fixture.bin");
    std::fs::write(&final_path, b"keep-me").expect("seed destination");
    let engine = DownloadEngine::open_in_memory().expect("open download engine");

    let task_id = engine
        .submit(request(&server, destination), RequestOrigin::Ui)
        .expect("submit task");
    let failed = engine
        .wait_for_terminal(task_id, Duration::from_secs(2))
        .await
        .expect("wait for failure");

    assert_eq!(failed.task.status(), DownloadStatus::Failed);
    assert_eq!(
        std::fs::read(final_path).expect("read destination"),
        b"keep-me"
    );
}

#[tokio::test]
async fn versioned_commands_publish_sequenced_state_events() {
    let server = DeterministicHttpServer::start();
    let directory = tempfile::tempdir().expect("create engine directory");
    let engine = DownloadEngine::open_in_memory().expect("open download engine");
    let events = engine
        .take_event_receiver()
        .expect("attach event subscriber");
    assert!(
        engine.take_event_receiver().is_err(),
        "the in-process event stream has one owning subscriber"
    );
    let request_id = uuid::Uuid::new_v4();

    let result = engine.handle_command(DownloadCommandEnvelope {
        protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
        request_id,
        origin: RequestOrigin::Cli,
        command: DownloadCommand::Submit {
            request: request(&server, directory.path().join("artifacts")),
            mode: DownloadSubmitMode::StartNow,
        },
    });
    let task_id = match result.outcome {
        DownloadCommandOutcome::Accepted {
            task_id: Some(task_id),
        } => task_id,
        outcome => panic!("unexpected command outcome: {outcome:?}"),
    };
    let first = events.recv().await.expect("receive task event");
    assert_eq!(first.sequence, 1);
    assert!(matches!(
        first.event,
        DownloadEvent::TaskChanged { task } if task.id() == task_id
    ));

    let snapshot = engine.handle_command(DownloadCommandEnvelope {
        protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
        request_id: uuid::Uuid::new_v4(),
        origin: RequestOrigin::Cli,
        command: DownloadCommand::Snapshot {
            query: Default::default(),
        },
    });
    assert!(matches!(
        snapshot.outcome,
        DownloadCommandOutcome::Snapshot(snapshot)
            if snapshot.task_details.iter().any(|task| task.task.id() == task_id)
    ));

    let pause = engine.handle_command(DownloadCommandEnvelope {
        protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
        request_id: uuid::Uuid::new_v4(),
        origin: RequestOrigin::Cli,
        command: DownloadCommand::Task {
            task_id,
            action: DownloadTaskAction::Cancel,
        },
    });
    assert!(matches!(
        pause.outcome,
        DownloadCommandOutcome::Accepted { .. }
    ));
}

#[tokio::test]
async fn queue_only_submission_stays_created_until_resumed() {
    let server = DeterministicHttpServer::start();
    let directory = tempfile::tempdir().expect("create engine directory");
    let destination = directory.path().join("artifacts");
    let engine = DownloadEngine::open_in_memory().expect("open download engine");

    let result = engine.handle_command(DownloadCommandEnvelope {
        protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
        request_id: uuid::Uuid::new_v4(),
        origin: RequestOrigin::Ui,
        command: DownloadCommand::Submit {
            request: request(&server, destination.clone()),
            mode: DownloadSubmitMode::QueueOnly,
        },
    });
    let task_id = match result.outcome {
        DownloadCommandOutcome::Accepted {
            task_id: Some(task_id),
        } => task_id,
        outcome => panic!("unexpected command outcome: {outcome:?}"),
    };

    tokio::time::sleep(Duration::from_millis(40)).await;
    let queued = engine
        .task(task_id)
        .expect("read queued task")
        .expect("queued task exists");
    assert_eq!(queued.task.status(), DownloadStatus::Created);
    assert!(!destination.join("engine-fixture.bin").exists());

    engine.resume(task_id).expect("start queued task");
    let completed = engine
        .wait_for_terminal(task_id, Duration::from_secs(2))
        .await
        .expect("wait for queued task");
    assert_eq!(completed.task.status(), DownloadStatus::Completed);
}

#[tokio::test]
async fn queue_only_submission_can_be_cancelled_without_starting_network_work() {
    let server = DeterministicHttpServer::start();
    let directory = tempfile::tempdir().expect("create engine directory");
    let destination = directory.path().join("artifacts");
    let engine = DownloadEngine::open_in_memory().expect("open download engine");
    let task_id = engine
        .enqueue(request(&server, destination.clone()), RequestOrigin::Ui)
        .expect("queue task");

    engine.cancel(task_id).expect("cancel queued task");
    let cancelled = engine
        .task(task_id)
        .expect("read task")
        .expect("task exists");
    assert_eq!(cancelled.task.status(), DownloadStatus::Cancelled);
    assert!(!destination.join("engine-fixture.bin").exists());
}

async fn wait_for_progress(engine: &DownloadEngine, task_id: nxtools_download::DownloadTaskId) {
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    loop {
        let task = engine
            .task(task_id)
            .expect("read task")
            .expect("task exists");
        if task.downloaded_bytes > 0 {
            return;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "task did not publish progress; status={:?}, error={:?}",
            task.task.status(),
            task.error
        );
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
}
