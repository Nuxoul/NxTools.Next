//! Audit #12: actual selected-detail command/store contention, never real downloads.
use super::*;
use gpui_kit::{AppContext, TestAppContext};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc,
};

pub(crate) fn fixture(
    cx: &mut TestAppContext,
) -> (
    Entity<DownloadApplication>,
    DownloadEngine,
    Vec<DownloadTaskId>,
    tempfile::TempDir,
) {
    cx.update(gpui_kit::init);
    cx.executor().allow_parking();
    let temp = tempfile::tempdir().expect("isolated detail fixture");
    let engine = DownloadEngine::open(temp.path().join("details.sqlite3"))
        .expect("temporary SQLite engine, no runner");
    let request: DownloadRequest = serde_json::from_value(serde_json::json!({
        "source_url": "https://synthetic.invalid/never-requested.bin",
        "save_directory": temp.path(), "save_directory_explicit": true,
        "file_name": "detail-fixture.bin", "headers": [], "cookie": null,
        "referrer": null, "connection_limit": 1, "speed_limit_bytes_per_second": null,
        "expected_total_bytes": 1024, "scheduled_at_unix_ms": null, "media": null,
        "category_id": null, "category_name": null
    }))
    .expect("valid synthetic request");
    let mut ids = Vec::new();
    for index in 0..3 {
        let mut request = request.clone();
        request.file_name = Some(format!("detail-{index}.bin"));
        ids.push(
            engine
                .enqueue(request, RequestOrigin::Internal)
                .expect("queue-only synthetic record"),
        );
    }
    let copy = engine.clone();
    let paths = temp.path().to_path_buf();
    let (platform, _) = nxtools_ipc::platform_bridge(8);
    let notifications = cx.update(|cx| cx.new(|_| NotificationCenter::default()));
    let app = cx.update(|cx| {
        cx.new(|cx| {
            super::ordinary_presentation_fixture::confirmed_application(
                copy,
                Rc::new(platform),
                notifications,
                DownloadRuntimeContext {
                    fallback_default_directory: paths.clone(),
                    config: AppConfig::default(),
                    config_path: paths.join("config.json"),
                    log_directory: paths.clone(),
                    database_path: paths.join("unused.db"),
                    ffmpeg_install_directory: paths.join("ffmpeg"),
                },
                cx,
            )
        })
    });
    cx.run_until_parked();
    (app, engine, ids, temp)
}

#[gpui_kit::test]
async fn audit12_real_detail_request_does_not_block_next_ui_turn_on_store_lock(
    cx: &mut TestAppContext,
) {
    let (app, engine, ids, _temp) = fixture(cx);
    let (ready_tx, ready_rx) = mpsc::sync_channel(1);
    let (unlock_tx, unlock_rx) = mpsc::sync_channel(1);
    let watchdog = Arc::new(AtomicBool::new(false));
    let observed = watchdog.clone();
    let holder = std::thread::spawn(move || {
        engine.with_storage_lock_for_test(|| {
            ready_tx.send(()).expect("lock held signal");
            if unlock_rx.recv_timeout(Duration::from_secs(3)).is_err() {
                observed.store(true, Ordering::SeqCst);
            }
        })
    });
    ready_rx
        .recv_timeout(Duration::from_secs(3))
        .expect("real SQLite connection is held");
    let ui_progress = Rc::new(Cell::new(false));
    let progressed = ui_progress.clone();
    let timed_out = watchdog.clone();
    let owner = app.clone();
    let id = ids[0];
    let started = Instant::now();
    cx.update(|cx| {
        cx.defer(move |cx| owner.update(cx, |app, cx| app.request_task_detail(id, cx)));
        cx.defer(move |_| {
            progressed.set(!timed_out.load(Ordering::SeqCst));
            let _ = unlock_tx.send(());
        });
    });
    cx.run_until_parked();
    holder.join().expect("fixture lock owner joins");
    println!(
        "audit12 real_request next_ui_turn_before_watchdog={} watchdog_unlocked={} elapsed_ms={}",
        ui_progress.get(),
        watchdog.load(Ordering::SeqCst),
        started.elapsed().as_millis()
    );
    assert!(
        ui_progress.get(),
        "defer is not background IO: UI must progress while the actual store mutex remains held"
    );
    assert!(
        !watchdog.load(Ordering::SeqCst),
        "watchdog must never be the unlock source"
    );
    cx.condition(&app, move |app, _| app.task_detail(id).is_some())
        .await;
}

pub(crate) struct HeldStore {
    unlock: Option<mpsc::SyncSender<()>>,
    thread: Option<std::thread::JoinHandle<()>>,
    watchdog: Arc<AtomicBool>,
}
impl HeldStore {
    pub(crate) fn new(engine: DownloadEngine) -> Self {
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);
        let (unlock_tx, unlock_rx) = mpsc::sync_channel(1);
        let watchdog = Arc::new(AtomicBool::new(false));
        let observed = watchdog.clone();
        let thread = std::thread::spawn(move || {
            engine.with_storage_lock_for_test(|| {
                ready_tx.send(()).expect("store held");
                if unlock_rx.recv_timeout(Duration::from_secs(3)).is_err() {
                    observed.store(true, Ordering::SeqCst);
                }
            })
        });
        ready_rx
            .recv_timeout(Duration::from_secs(3))
            .expect("actual store lock acquired");
        Self {
            unlock: Some(unlock_tx),
            thread: Some(thread),
            watchdog,
        }
    }
    pub(crate) fn release(mut self) {
        self.unlock
            .take()
            .expect("release once")
            .send(())
            .expect("unlock worker");
        self.thread
            .take()
            .expect("holder")
            .join()
            .expect("holder joins");
        assert!(
            !self.watchdog.load(Ordering::SeqCst),
            "watchdog never releases a passing test"
        );
    }
}
impl Drop for HeldStore {
    fn drop(&mut self) {
        if let Some(unlock) = self.unlock.take() {
            let _ = unlock.send(());
        }
        if let Some(thread) = self.thread.take() {
            thread.join().expect("fixture holder cleanup");
        }
    }
}

#[gpui_kit::test]
async fn audit12_duplicate_aba_and_cache_use_one_real_worker_and_latest_generation(
    cx: &mut TestAppContext,
) {
    let (app, engine, ids, _temp) = fixture(cx);
    let held = HeldStore::new(engine);
    let (a, b) = (ids[0], ids[1]);
    let first = app.update(cx, |app, cx| {
        app.request_task_detail(a, cx);
        app.detail_request_state_for_test()
    });
    let duplicate = app.update(cx, |app, cx| {
        app.request_task_detail(a, cx);
        app.detail_request_state_for_test()
    });
    assert_eq!(
        first, duplicate,
        "same pending request must not dispatch or advance generation"
    );
    app.update(cx, |app, cx| app.request_task_detail(b, cx));
    let latest = app.update(cx, |app, cx| {
        app.request_task_detail(a, cx);
        app.detail_request_state_for_test()
    });
    assert!(
        latest.0 > first.0 && latest.1 && latest.2,
        "one running plus one coalesced pending"
    );
    assert_eq!(
        app.read_with(cx, |app, _| app.detail_request_events_for_test()),
        (1, 0, 0)
    );
    held.release();
    cx.condition(&app, |app, _| {
        !app.detail_request_state_for_test().1 && app.task_detail(a).is_some()
    })
    .await;
    assert_eq!(
        app.read_with(cx, |app, _| app.detail_request_events_for_test()),
        (2, 1, 1),
        "old A discarded, intermediate B never dispatched, new A published"
    );
    assert!(app.read_with(cx, |app, _| app.task_detail(b).is_none()));
    let events = app.read_with(cx, |app, _| app.detail_request_events_for_test());
    app.update(cx, |app, cx| app.request_task_detail(a, cx));
    assert_eq!(
        app.read_with(cx, |app, _| app.detail_request_events_for_test()),
        events,
        "cache hit never starts IO or reenters another view"
    );
    assert!(!app.read_with(cx, |app, _| app.detail_request_state_for_test().1));
}

#[gpui_kit::test]
async fn audit12_retired_scope_and_late_failure_cannot_replace_new_detail(cx: &mut TestAppContext) {
    let (app, engine, ids, _temp) = fixture(cx);
    let held = HeldStore::new(engine);
    let missing = DownloadTaskId::new();
    let old = DownloadDetailScope::new(missing);
    app.update(cx, |app, cx| {
        app.bind_task_detail_scope(old.clone());
        app.request_task_detail(missing, cx);
    });
    old.retire();
    let current = ids[1];
    app.update(cx, |app, cx| app.request_task_detail(current, cx));
    held.release();
    cx.condition(&app, |app, _| {
        !app.detail_request_state_for_test().1 && app.task_detail(current).is_some()
    })
    .await;
    assert_eq!(
        app.read_with(cx, |app, _| app.detail_request_events_for_test()),
        (2, 1, 1)
    );
    assert!(app.read_with(cx, |app, _| app.task_detail_error(missing).is_none()));
    assert!(app.read_with(cx, |app, _| app.task_detail_error(current).is_none()));
}

#[gpui_kit::test]
async fn audit12_real_sqlite_failure_and_retry_keep_original_error_owner(cx: &mut TestAppContext) {
    let (app, _engine, ids, temp) = fixture(cx);
    let connection =
        rusqlite::Connection::open(temp.path().join("details.sqlite3")).expect("owned fixture DB");
    connection
        .execute_batch("ALTER TABLE download_task_timeline RENAME TO blocked_timeline;")
        .expect("make actual detail query fail");
    let id = ids[0];
    app.update(cx, |app, cx| app.request_task_detail(id, cx));
    cx.condition(&app, |app, _| app.task_detail_error(id).is_some())
        .await;
    assert!(app.read_with(cx, |app, _| app.task_detail(id).is_none()));
    assert!(!app.read_with(cx, |app, _| app.detail_request_state_for_test().1));
    connection
        .execute_batch("ALTER TABLE blocked_timeline RENAME TO download_task_timeline;")
        .expect("restore fixture schema");
    app.update(cx, |app, cx| app.request_task_detail(id, cx));
    cx.condition(&app, |app, _| app.task_detail(id).is_some())
        .await;
    assert!(app.read_with(cx, |app, _| app.task_detail_error(id).is_none()));
    assert!(app.read_with(cx, |app, _| {
        app.task_detail(id)
            .expect("real selected detail")
            .timeline
            .len()
            <= 300
    }));
}

#[gpui_kit::test]
async fn audit12_refresh_rebound_scope_does_not_lose_pending_refresh_to_cache(
    cx: &mut TestAppContext,
) {
    let (app, engine, ids, _temp) = fixture(cx);
    let id = ids[0];
    app.update(cx, |app, cx| app.request_task_detail(id, cx));
    cx.condition(&app, |app, _| app.task_detail(id).is_some())
        .await;
    let held = HeldStore::new(engine);
    app.update(cx, |app, cx| app.refresh_selected_task_detail(cx));
    let fresh = DownloadDetailScope::new(id);
    app.update(cx, |app, cx| {
        app.bind_task_detail_scope(fresh);
        app.request_task_detail(id, cx);
    });
    assert!(
        app.read_with(cx, |app, _| app.detail_request_state_for_test().2),
        "last-good cache must not erase an invalidated active refresh"
    );
    held.release();
    cx.condition(&app, |app, _| {
        !app.detail_request_state_for_test().1 && !app.detail_request_state_for_test().2
    })
    .await;
    assert_eq!(
        app.read_with(cx, |app, _| app.detail_request_events_for_test()),
        (3, 2, 1),
        "new scope receives a new read; old refresh cannot publish"
    );
    app.update(cx, |app, cx| {
        app.cancel(id, cx)
            .expect("cancel queue-only task, no runner")
    });
    cx.condition(&app, |app, _| {
        app.task_detail(id)
            .is_some_and(|detail| detail.task.task.status() == DownloadStatus::Cancelled)
    })
    .await;
    assert_eq!(
        app.read_with(cx, |app, _| app
            .task_detail(id)
            .expect("cancelled detail")
            .task
            .task
            .id()),
        id
    );
}

#[gpui_kit::test]
async fn audit12_scope_cancel_and_removed_id_reject_real_inflight_success(cx: &mut TestAppContext) {
    let (app, engine, ids, _temp) = fixture(cx);
    let held = HeldStore::new(engine);
    let id = ids[0];
    let scope = DownloadDetailScope::new(id);
    app.update(cx, |app, cx| {
        app.bind_task_detail_scope(scope.clone());
        app.request_task_detail(id, cx);
    });
    app.update(cx, |app, _| app.retire_task_detail_scope(&scope));
    assert!(
        app.read_with(cx, |app, _| app.detail_request_state_for_test().1),
        "logical cancel must not free physically blocked worker"
    );
    let remaining = app.read_with(cx, |app, _| {
        app.tasks()
            .iter()
            .filter(|task| task.task.id() != id)
            .cloned()
            .collect()
    });
    app.update(cx, |app, _| app.replace_tasks_for_test(remaining));
    held.release();
    cx.condition(&app, |app, _| !app.detail_request_state_for_test().1)
        .await;
    assert!(app.read_with(cx, |app, _| app.task_detail(id).is_none()
        && app.task_detail_error(id).is_none()));
    assert_eq!(
        app.read_with(cx, |app, _| app.detail_request_events_for_test()),
        (1, 0, 1)
    );
}
