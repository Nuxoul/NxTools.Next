use super::*;
use gpui_kit::{Modifiers, TestAppContext, VisualTestContext};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc,
};

fn extend_tasks(engine: &DownloadEngine, ids: &mut Vec<DownloadTaskId>, count: usize) {
    let request = engine
        .task(ids[0])
        .expect("fixture task")
        .expect("task exists")
        .task
        .request()
        .clone();
    for index in ids.len()..count {
        let mut request = request.clone();
        request.file_name = Some(format!("bulk-{index}.bin"));
        ids.push(
            engine
                .enqueue(request, RequestOrigin::Internal)
                .expect("dormant batch task"),
        );
    }
}

#[gpui_kit::test]
async fn batch_feedback_real_store_wait_keeps_next_ui_turn_responsive(cx: &mut TestAppContext) {
    let (application, engine, mut ids, _directory) = detail_request_tests::fixture(cx);
    extend_tasks(&engine, &mut ids, 300);
    cx.run_until_parked();
    application.update(cx, |app, cx| {
        app.refresh(cx);
        app.request_task_window(ids[0], cx);
    });
    cx.run_until_parked();
    let handle = application.read_with(cx, |app, _| app.task_windows[&ids[0]].window);
    let mut surface = VisualTestContext::from_window(handle, cx);
    surface
        .refresh()
        .expect("production task window before batch");
    let (ready_tx, ready_rx) = mpsc::sync_channel(1);
    let (unlock_tx, unlock_rx) = mpsc::sync_channel(1);
    let watchdog = Arc::new(AtomicBool::new(false));
    let timed_out = watchdog.clone();
    let holder = std::thread::spawn(move || {
        engine.with_storage_lock_for_test(|| {
            ready_tx.send(()).expect("real store held");
            if unlock_rx.recv_timeout(Duration::from_secs(3)).is_err() {
                timed_out.store(true, Ordering::SeqCst);
            }
        })
    });
    ready_rx
        .recv_timeout(Duration::from_secs(3))
        .expect("store holder ready");
    let continued = Rc::new(Cell::new(false));
    let observed = continued.clone();
    let timed_out = watchdog.clone();
    let owner = application.clone();
    cx.update(|cx| {
        cx.defer(move |cx| {
            owner.update(cx, |app, cx| {
                app.batch_action(ids, DownloadBatchAction::Remove, cx)
            })
        });
        cx.defer(move |cx| {
            handle
                .update(cx, |_, window, _| window.refresh())
                .expect("next production window turn");
            observed.set(!timed_out.load(Ordering::SeqCst));
            let _ = unlock_tx.send(());
        });
    });
    cx.run_until_parked();
    holder.join().expect("store holder exits");
    assert!(
        continued.get(),
        "real batch must not block the next UI turn on SQLite"
    );
    assert!(
        !watchdog.load(Ordering::SeqCst),
        "UI releases the lock, never watchdog"
    );
    cx.condition(&application, |app, cx| {
        !app.batch_busy()
            && app.tasks().is_empty()
            && app
                .notifications
                .read(cx)
                .items()
                .iter()
                .any(|notice| notice.title == "已移除 300 项")
    })
    .await;
    application.read_with(cx, |app, _| {
        assert!(app.engine.tasks().expect("real engine snapshot").is_empty())
    });
}

#[gpui_kit::test]
async fn batch_feedback_duplicate_submission_is_ignored_until_actual_work_finishes(
    cx: &mut TestAppContext,
) {
    let (application, engine, ids, _directory) = detail_request_tests::fixture(cx);
    let held = detail_request_tests::HeldStore::new(engine.clone());
    application.update(cx, |app, cx| {
        app.batch_action(ids.clone(), DownloadBatchAction::Remove, cx)
    });
    assert!(application.read_with(cx, |app, _| app.batch_loading(DownloadBatchAction::Remove)));
    application.update(cx, |app, cx| {
        app.batch_action(ids, DownloadBatchAction::Cancel, cx)
    });
    assert!(!application.read_with(cx, |app, _| app.batch_loading(DownloadBatchAction::Cancel)));
    assert!(application.read_with(cx, |app, _| app.batch_busy()));
    held.release();
    cx.condition(&application, |app, cx| {
        !app.batch_busy()
            && app
                .notifications
                .read(cx)
                .items()
                .iter()
                .any(|notice| notice.title == "已移除 3 项")
    })
    .await;
    assert!(engine.tasks().expect("removed by one batch").is_empty());
    assert_eq!(
        application.read_with(cx, |app, cx| app
            .notifications
            .read(cx)
            .items()
            .iter()
            .filter(|notice| notice.id.starts_with("download-batch-"))
            .count()),
        1
    );
}

#[gpui_kit::test]
async fn batch_feedback_destructive_threshold_reuses_confirmation_and_list_remove_never_confirms(
    cx: &mut TestAppContext,
) {
    let (application, engine, mut ids, _directory) = detail_request_tests::fixture(cx);
    extend_tasks(&engine, &mut ids, 20);
    cx.run_until_parked();
    application.update(cx, |app, cx| {
        app.config.download.task_removal_deletes_file = true;
        app.request_delete_confirmation(ids[0], cx);
    });
    for action in [
        DownloadBatchAction::Cancel,
        DownloadBatchAction::RemoveWithArtifact,
    ] {
        application.update(cx, |app, cx| app.batch_action(ids.clone(), action, cx));
        let handle = application.read_with(cx, |app, _| {
            assert!(app.batch_busy());
            assert_eq!(app.pending_batch_confirmation, Some(ids[0]));
            assert_eq!(app.delete_confirmation_windows.len(), 1);
            app.delete_confirmation_windows[&ids[0]]
        });
        application.update(cx, |app, cx| app.batch_action(ids.clone(), action, cx));
        let mut surface = VisualTestContext::from_window(handle, cx);
        surface.simulate_resize(size(px(380.0), px(178.0)));
        surface.refresh().expect("production quantity confirmation");
        let confirm = surface
            .debug_bounds("download-delete-danger-confirm-action")
            .expect("original danger action");
        assert!(confirm.bottom() <= px(178.0));
        let cancel = surface
            .debug_bounds("download-delete-secondary-cancel-action")
            .expect("original cancel");
        surface.simulate_click(cancel.center(), Modifiers::default());
        surface.run_until_parked();
        assert!(!application.read_with(&surface, |app, _| app.batch_busy()));
        assert_eq!(
            engine
                .tasks()
                .expect("confirmation cancellation keeps tasks")
                .len(),
            20
        );
        application.update(cx, |app, cx| app.batch_action(ids.clone(), action, cx));
        let handle = application.read_with(cx, |app, _| app.delete_confirmation_windows[&ids[0]]);
        let mut surface = VisualTestContext::from_window(handle, cx);
        surface
            .refresh()
            .expect("batch confirmation before close event");
        assert!(surface.simulate_close());
        surface.run_until_parked();
        assert!(application.read_with(&surface, |app, _| {
            !app.batch_busy() && app.delete_confirmation_windows.is_empty()
        }));
        assert_eq!(engine.tasks().expect("closing confirms nothing").len(), 20);
    }
    application.update(cx, |app, cx| {
        app.batch_action(ids[..19].to_vec(), DownloadBatchAction::Cancel, cx)
    });
    assert!(application.read_with(cx, |app, _| app.delete_confirmation_windows.is_empty()));
    cx.condition(&application, |app, _| !app.batch_busy()).await;
    assert_eq!(
        engine
            .tasks()
            .expect("cancelled tasks")
            .iter()
            .filter(|task| task.task.status() == DownloadStatus::Cancelled)
            .count(),
        19
    );
    application.update(cx, |app, cx| {
        app.batch_action(ids, DownloadBatchAction::Remove, cx)
    });
    assert!(application.read_with(cx, |app, _| app.delete_confirmation_windows.is_empty()));
    cx.condition(&application, |app, _| {
        !app.batch_busy() && app.tasks().is_empty()
    })
    .await;
}

#[gpui_kit::test]
async fn batch_feedback_partial_result_preserves_success_and_offers_failed_selection(
    cx: &mut TestAppContext,
) {
    let (application, engine, ids, _directory) = detail_request_tests::fixture(cx);
    application.update(cx, |app, cx| {
        app.batch_action(
            vec![ids[0], DownloadTaskId::new()],
            DownloadBatchAction::Remove,
            cx,
        )
    });
    cx.condition(&application, |app, cx| {
        !app.batch_busy()
            && app
                .notifications
                .read(cx)
                .items()
                .iter()
                .any(|notice| notice.title == "已移除 1 项，1 项失败")
    })
    .await;
    assert!(engine.task(ids[0]).expect("successful removal").is_none());
    application.read_with(cx, |app, cx| {
        let notice = app
            .notifications
            .read(cx)
            .items()
            .iter()
            .find(|notice| notice.id.starts_with("download-batch-"))
            .expect("batch summary");
        assert_eq!(
            notice.action.as_ref().expect("failed-only selection").label,
            "仅选中失败项"
        );
    });
}

#[gpui_kit::test]
async fn batch_feedback_retires_every_target_confirmation_but_keeps_unrelated_window(
    cx: &mut TestAppContext,
) {
    let (application, engine, mut ids, _directory) = detail_request_tests::fixture(cx);
    extend_tasks(&engine, &mut ids, 21);
    cx.run_until_parked();
    application.update(cx, |app, cx| {
        app.refresh(cx);
        app.config.download.task_removal_deletes_file = true;
        for id in [ids[0], ids[1], ids[20]] {
            app.request_delete_confirmation(id, cx);
        }
    });
    let (first, nonfirst, unrelated) = application.read_with(cx, |app, _| {
        (
            app.delete_confirmation_windows[&ids[0]],
            app.delete_confirmation_windows[&ids[1]],
            app.delete_confirmation_windows[&ids[20]],
        )
    });
    for confirm in [false, true] {
        if confirm {
            application.update(cx, |app, cx| app.request_delete_confirmation(ids[1], cx));
        }
        application.update(cx, |app, cx| {
            app.batch_action(
                ids[..20].to_vec(),
                DownloadBatchAction::RemoveWithArtifact,
                cx,
            );
        });
        let handle = application.read_with(cx, |app, _| {
            assert_eq!(app.pending_batch_confirmation, Some(ids[0]));
            assert_eq!(
                app.delete_confirmation_windows.len(),
                2,
                "only the batch confirmation and unrelated single confirmation remain"
            );
            assert!(
                !app.delete_confirmation_windows.contains_key(&ids[1]),
                "a nonfirst target cannot retain its single-item confirmation"
            );
            assert_eq!(app.delete_confirmation_windows[&ids[20]], unrelated);
            app.delete_confirmation_windows[&ids[0]]
        });
        cx.run_until_parked();
        let windows = cx.read(|cx| cx.windows());
        assert!(!windows.contains(&first) && !windows.contains(&nonfirst));
        assert!(windows.contains(&unrelated));
        let mut surface = VisualTestContext::from_window(handle, cx);
        surface.refresh().expect("original batch confirmation");
        let action = surface
            .debug_bounds(if confirm {
                "download-delete-danger-confirm-action"
            } else {
                "download-delete-secondary-cancel-action"
            })
            .expect("original decision button");
        surface.simulate_click(action.center(), Modifiers::default());
        surface.run_until_parked();
        cx.condition(&application, |app, _| !app.batch_busy()).await;
        assert_eq!(
            engine.tasks().expect("actual batch result").len(),
            if confirm { 1 } else { 21 }
        );
        application.read_with(cx, |app, _| {
            assert_eq!(app.delete_confirmation_windows.len(), 1);
            assert_eq!(app.delete_confirmation_windows[&ids[20]], unrelated);
        });
    }
}
