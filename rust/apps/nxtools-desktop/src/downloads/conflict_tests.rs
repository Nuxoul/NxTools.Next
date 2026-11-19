use super::*;
use gpui_kit::{AppContext, TestAppContext, size};

#[gpui_kit::test]
fn destination_conflict_task_window_keeps_named_actions_inside_failed_budget(
    cx: &mut TestAppContext,
) {
    let (application, engine, ids, _directory) = detail_request_tests::fixture(cx);
    cx.update(crate::theme::apply_component_theme);
    let mut request = engine
        .task(ids[0])
        .expect("task")
        .expect("task exists")
        .task
        .request()
        .clone();
    let name = format!("{}.zip", "long-download-file-name-".repeat(8));
    request.file_name = Some(name.clone());
    let task_id = engine
        .enqueue(request, RequestOrigin::Internal)
        .expect("dormant fixture");
    let mut snapshot = engine.task(task_id).expect("task").expect("snapshot");
    let name = snapshot
        .final_path
        .as_ref()
        .expect("resolved target")
        .file_name()
        .expect("resolved filename")
        .to_string_lossy()
        .into_owned();
    let changed_at = snapshot.task.created_at_unix_ms().saturating_add(1);
    snapshot
        .task
        .transition_to(DownloadStatus::Probing, changed_at)
        .expect("probe state");
    snapshot
        .task
        .transition_to(DownloadStatus::Failed, changed_at.saturating_add(1))
        .expect("failure state");
    snapshot.error = Some(nxtools_download::DownloadFault {
        code: nxtools_download::DownloadFaultCode::DestinationExists,
        message: "存储错误".into(),
        retryable: false,
    });
    let (bridge, _) = nxtools_ipc::platform_bridge(16);
    let generation = uuid::Uuid::new_v4();
    let (view, window) = cx.add_window_view(|_, _| {
        DownloadTaskWindowView::new(
            application,
            Rc::new(bridge),
            task_id,
            TaskWindowIdentity {
                native_window_handle: 0,
                generation,
            },
            Some(snapshot),
            None,
            false,
        )
    });
    for preference in [
        nxtools_core::ThemePreference::Dark,
        nxtools_core::ThemePreference::Light,
    ] {
        window.update(|_, cx| crate::theme::set_theme_preference(preference, cx));
        window.simulate_resize(size(px(440.0), px(196.0)));
        window.refresh().expect("production failed window");
        let nodes = window.update(|window, _| gpui_kit::base::test_support::snapshots(window));
        let retry = nodes
            .iter()
            .find(|node| {
                node.role() == Some(gpui_kit::Role::Button)
                    && node.label() == Some("自动重命名后重试")
            })
            .expect("numbered retry is the primary conflict action");
        let cancel = nodes
            .iter()
            .find(|node| {
                node.role() == Some(gpui_kit::Role::Button) && node.label() == Some("取消任务")
            })
            .expect("cancel remains reachable");
        let footer = window
            .debug_bounds("download-task-window-footer")
            .expect("original footer");
        for button in [retry, cancel] {
            assert!(button.bounds().top() >= footer.top());
            assert!(button.bounds().bottom() <= px(196.0));
            assert!(button.bounds().left() >= px(0.0) && button.bounds().right() <= px(440.0));
        }
        assert!(
            window
                .debug_bounds("download-window-destination-conflict")
                .is_some(),
            "typed conflict has explicit presentation"
        );
        assert!(
            nodes.iter().any(|node| node.label() == Some(name.as_str())),
            "complete filename remains accessible"
        );
        assert_eq!(
            view.read_with(window, |view, _| view.window_generation),
            generation
        );
    }
}

#[gpui_kit::test]
fn failed_task_window_recommends_the_action_that_matches_the_fault(cx: &mut TestAppContext) {
    use nxtools_download::{DownloadFault, DownloadFaultCode};
    let (application, engine, ids, _directory) = detail_request_tests::fixture(cx);
    cx.update(crate::theme::apply_component_theme);
    let mut snapshot = engine.task(ids[0]).expect("task").expect("snapshot");
    let at = snapshot.task.created_at_unix_ms().saturating_add(1);
    snapshot
        .task
        .transition_to(DownloadStatus::Probing, at)
        .expect("probe");
    snapshot
        .task
        .transition_to(DownloadStatus::Failed, at + 1)
        .expect("failed");
    for (code, message, retryable, expected) in [
        (
            DownloadFaultCode::UnexpectedWebPage,
            "unexpected web page",
            false,
            "在浏览器中打开",
        ),
        (
            DownloadFaultCode::Protocol,
            "resource changed",
            true,
            "从头下载",
        ),
        (
            DownloadFaultCode::FileInUse,
            "file in use",
            false,
            "重新下载",
        ),
        (
            DownloadFaultCode::Network,
            "connection reset",
            true,
            "重试下载",
        ),
    ] {
        snapshot.error = Some(DownloadFault {
            code,
            message: message.into(),
            retryable,
        });
        let (bridge, _) = nxtools_ipc::platform_bridge(16);
        let (_, window) = cx.add_window_view(|_, _| {
            DownloadTaskWindowView::new(
                application.clone(),
                Rc::new(bridge),
                ids[0],
                TaskWindowIdentity {
                    native_window_handle: 0,
                    generation: uuid::Uuid::new_v4(),
                },
                Some(snapshot.clone()),
                None,
                false,
            )
        });
        window.simulate_resize(size(px(440.0), px(196.0)));
        window.refresh().expect("production failed window");
        let bounds = window
            .debug_bounds("download-window-failed-primary")
            .expect("recommended action");
        let nodes = window.update(|window, _| gpui_kit::base::test_support::snapshots(window));
        assert!(
            nodes
                .iter()
                .any(|node| node.label() == Some(expected) && node.bounds() == bounds),
            "{expected}"
        );
    }
}

#[gpui_kit::test]
fn failed_task_window_routes_raw_error_to_existing_notification_only(cx: &mut TestAppContext) {
    let (application, engine, ids, _directory) = detail_request_tests::fixture(cx);
    cx.update(crate::theme::apply_component_theme);
    let mut snapshot = engine.task(ids[0]).expect("task").expect("snapshot");
    let at = snapshot.task.created_at_unix_ms().saturating_add(1);
    snapshot
        .task
        .transition_to(DownloadStatus::Probing, at)
        .expect("probe");
    snapshot
        .task
        .transition_to(DownloadStatus::Failed, at + 1)
        .expect("failed");
    let raw_error = "The remote host returned a very long original English error: connection aborted before the response completed";
    snapshot.error = Some(nxtools_download::DownloadFault {
        code: nxtools_download::DownloadFaultCode::Network,
        message: raw_error.into(),
        retryable: true,
    });
    let (bridge, _) = nxtools_ipc::platform_bridge(16);
    let (view, window) = cx.add_window_view(|_, _| {
        DownloadTaskWindowView::new(
            application.clone(),
            Rc::new(bridge),
            ids[0],
            TaskWindowIdentity {
                native_window_handle: 0,
                generation: uuid::Uuid::new_v4(),
            },
            Some(snapshot.clone()),
            None,
            false,
        )
    });
    window.simulate_resize(size(px(440.0), px(196.0)));
    window.refresh().expect("production failed window");
    let nodes = window.update(|window, _| gpui_kit::base::test_support::snapshots(window));
    assert!(
        window
            .debug_bounds("download-task-window-phase-failed")
            .is_some()
    );
    assert!(
        window
            .debug_bounds("download-window-failure-kit-alert")
            .is_none()
    );
    assert!(
        nodes
            .iter()
            .all(|node| node.label().is_none_or(|label| !label.contains(raw_error)))
    );
    for action in ["取消任务", "重试下载", "重新下载"] {
        assert!(
            nodes.iter().any(|node| node.label() == Some(action)),
            "missing {action}"
        );
    }
    assert_eq!(
        view.read_with(window, |view, _| view
            .snapshot
            .as_ref()
            .expect("failed snapshot")
            .task
            .status()),
        DownloadStatus::Failed
    );
    application.update(cx, |app, cx| {
        app.notified_terminal_states
            .remove(&(ids[0], DownloadStatus::Failed));
        app.publish_terminal_notifications(&[snapshot.clone()], cx);
        app.publish_terminal_notifications(&[snapshot.clone()], cx);
        assert!(app.completion_feedback_scheduled);
        app.flush_completion_feedback(cx);
        {
            let notices = app.notifications.read(cx).items();
            assert_eq!(notices.len(), 1);
            assert_eq!(
                notices[0].level,
                crate::notifications::NotificationLevel::Error
            );
            assert_eq!(notices[0].message.as_ref(), raw_error);
        }
        app.publish_terminal_notifications(&[snapshot], cx);
        assert!(!app.completion_feedback_scheduled);
        assert_eq!(app.notifications.read(cx).items().len(), 1);
    });
}

#[gpui_kit::test]
fn reopened_persisted_failure_does_not_publish_old_notification(cx: &mut TestAppContext) {
    let (app, engine, id, _directory, _temporary, _target) = persisted_conflict_fixture(cx);
    app.update(cx, |app, cx| {
        assert!(
            app.notified_terminal_states
                .contains(&(id, DownloadStatus::Failed))
        );
        let task = engine.task(id).expect("task").expect("failed task");
        app.publish_terminal_notifications(std::slice::from_ref(&task), cx);
        app.publish_terminal_notifications(&[task], cx);
        assert!(!app.completion_feedback_scheduled);
        assert!(app.notifications.read(cx).items().is_empty());
    });
}

fn persisted_conflict_fixture(
    cx: &mut TestAppContext,
) -> (
    Entity<DownloadApplication>,
    DownloadEngine,
    DownloadTaskId,
    tempfile::TempDir,
    PathBuf,
    PathBuf,
) {
    cx.update(gpui_kit::init);
    cx.update(crate::theme::apply_component_theme);
    let directory = tempfile::tempdir().expect("isolated conflict database");
    let database = directory.path().join("conflict.sqlite3");
    let target = directory.path().join("conflict.bin");
    let temporary = directory.path().join("conflict.part");
    let request: DownloadRequest = serde_json::from_value(serde_json::json!({
        "source_url":"https://synthetic.invalid/never-requested.bin", "save_directory":directory.path(),
        "save_directory_explicit":true, "file_name":"conflict.bin", "headers":[], "cookie":null,
        "referrer":null, "connection_limit":1, "speed_limit_bytes_per_second":null,
        "expected_total_bytes":8, "scheduled_at_unix_ms":null, "media":null, "category_id":null, "category_name":null
    })).expect("synthetic request");
    let engine = DownloadEngine::open(&database).expect("temporary engine");
    let id = engine
        .enqueue(request, RequestOrigin::Internal)
        .expect("dormant conflict task");
    let mut task = engine.task(id).expect("snapshot").expect("task").task;
    let changed_at = task.created_at_unix_ms().saturating_add(1);
    task.transition_to(DownloadStatus::Probing, changed_at)
        .expect("probing");
    task.transition_to(DownloadStatus::Failed, changed_at.saturating_add(1))
        .expect("failed");
    drop(engine);
    let store = nxtools_download::SqliteDownloadStore::open(&database).expect("temporary store");
    store.save_task(&task).expect("persist failed state");
    store
        .set_queue_running("default", false)
        .expect("queue is stopped: no network after retry");
    drop(store);
    std::fs::write(&target, b"existing").expect("preexisting destination");
    std::fs::write(&temporary, b"retained").expect("retained partial data");
    let connection = rusqlite::Connection::open(&database).expect("synthetic database only");
    connection.execute("UPDATE download_tasks SET error_code='destination_exists', error_message='conflict fixture', error_retryable=0, downloaded_bytes=8 WHERE task_id=?1", [id.as_uuid().to_string()]).expect("persist typed conflict");
    for (kind, path) in [("temporary", &temporary), ("final", &target)] {
        connection.execute("INSERT INTO download_artifacts(artifact_id,task_id,kind,path,state,created_at_unix_ms) VALUES (?1,?2,?3,?4,'pending',1)",
            rusqlite::params![format!("{}:{kind}", id.as_uuid()), id.as_uuid().to_string(), kind, path.to_string_lossy().to_string()]).expect("owned synthetic artifact");
    }
    drop(connection);
    let engine = DownloadEngine::open(&database).expect("restore actual typed failed task");
    let (platform, _) = nxtools_ipc::platform_bridge(16);
    let notifications = cx.update(|cx| cx.new(|_| NotificationCenter::default()));
    let paths = directory.path().to_path_buf();
    let copy = engine.clone();
    let app = cx.update(|cx| {
        cx.new(|cx| {
            DownloadApplication::new(
                copy,
                Rc::new(platform),
                notifications,
                DownloadRuntimeContext {
                    fallback_default_directory: paths.clone(),
                    config: AppConfig::default(),
                    config_path: paths.join("config.json"),
                    log_directory: paths.clone(),
                    database_path: database,
                    ffmpeg_install_directory: paths.join("ffmpeg"),
                },
                cx,
            )
        })
    });
    app.update(cx, |app, cx| app.request_task_window(id, cx));
    cx.run_until_parked();
    (app, engine, id, directory, temporary, target)
}

#[gpui_kit::test]
fn destination_conflict_numbered_retry_click_retains_bytes_and_keyed_window(
    cx: &mut TestAppContext,
) {
    let (app, engine, id, _directory, temporary, target) = persisted_conflict_fixture(cx);
    let (handle, generation) = app.read_with(cx, |app, _| {
        (
            app.task_windows[&id].window,
            app.task_windows[&id].generation,
        )
    });
    let mut window = gpui_kit::VisualTestContext::from_window(handle, cx);
    window.simulate_resize(size(px(440.0), px(196.0)));
    window.refresh().expect("real conflict window");
    let retry = window
        .update(|window, _| gpui_kit::base::test_support::snapshots(window))
        .into_iter()
        .find(|node| node.label() == Some("自动重命名后重试"))
        .expect("real retry button")
        .bounds();
    window.simulate_click(retry.center(), gpui_kit::Modifiers::default());
    window.run_until_parked();
    let task = engine.task(id).expect("renamed task").expect("same task");
    assert_eq!(
        task.task.request().file_name.as_deref(),
        Some("conflict (1).bin")
    );
    assert_eq!(
        std::fs::read(temporary).expect("retained data"),
        b"retained"
    );
    assert_eq!(
        std::fs::read(target).expect("existing destination"),
        b"existing"
    );
    assert_eq!(
        app.read_with(&window, |app, _| app.task_windows[&id].generation),
        generation
    );
}

#[gpui_kit::test]
fn destination_conflict_cancel_click_only_cleans_owned_incomplete_data(cx: &mut TestAppContext) {
    let (app, engine, id, _directory, temporary, target) = persisted_conflict_fixture(cx);
    let handle = app.read_with(cx, |app, _| app.task_windows[&id].window);
    let mut window = gpui_kit::VisualTestContext::from_window(handle, cx);
    window.simulate_resize(size(px(440.0), px(196.0)));
    window.refresh().expect("real conflict cancel");
    let cancel = window
        .update(|window, _| gpui_kit::base::test_support::snapshots(window))
        .into_iter()
        .find(|node| node.label() == Some("取消任务"))
        .expect("real cancel button")
        .bounds();
    window.simulate_click(cancel.center(), gpui_kit::Modifiers::default());
    window.run_until_parked();
    assert_eq!(
        engine
            .task(id)
            .expect("cancelled task")
            .expect("same task")
            .task
            .status(),
        DownloadStatus::Cancelled
    );
    assert!(!temporary.exists());
    assert_eq!(
        std::fs::read(target).expect("existing destination"),
        b"existing"
    );
}

#[gpui_kit::test]
fn web_page_failure_opens_browser_once_and_retains_the_failed_window(cx: &mut TestAppContext) {
    let (application, engine, ids, _directory) = detail_request_tests::fixture(cx);
    cx.update(crate::theme::apply_component_theme);
    let task_id = ids[0];
    let mut snapshot = engine.task(task_id).expect("task").expect("snapshot");
    let source = snapshot.task.request().source_url.clone();
    let at = snapshot.task.created_at_unix_ms() + 1;
    snapshot
        .task
        .transition_to(DownloadStatus::Probing, at)
        .expect("probe");
    snapshot
        .task
        .transition_to(DownloadStatus::Failed, at + 1)
        .expect("failed");
    snapshot.error = Some(nxtools_download::DownloadFault {
        code: nxtools_download::DownloadFaultCode::UnexpectedWebPage,
        message: "网站返回了网页而不是文件。".into(),
        retryable: false,
    });
    let (bridge, endpoint) = nxtools_ipc::platform_bridge(16);
    endpoint.set_connected_for_contract_test(true);
    let bridge = Rc::new(bridge);
    application.update(cx, |app, _| app.platform = bridge.clone());
    let generation = uuid::Uuid::new_v4();
    let (view, window) = cx.add_window_view(|_, _| {
        DownloadTaskWindowView::new(
            application.clone(),
            bridge,
            task_id,
            TaskWindowIdentity {
                native_window_handle: 0,
                generation,
            },
            Some(snapshot),
            None,
            false,
        )
    });
    window.simulate_resize(size(px(440.0), px(196.0)));
    window.refresh().expect("failed window");
    let action = window
        .update(|window, _| gpui_kit::base::test_support::snapshots(window))
        .into_iter()
        .find(|node| {
            node.role() == Some(gpui_kit::Role::Button) && node.label() == Some("在浏览器中打开")
        })
        .expect("actionable web-page failure");
    assert!(action.bounds().right() <= px(440.0) && action.bounds().bottom() <= px(196.0));
    window.simulate_click(action.bounds().center(), gpui_kit::Modifiers::default());
    window.run_until_parked();
    let command = endpoint
        .try_receive_command_for_contract_test()
        .expect("native browser command");
    assert!(matches!(&command.message,
        nxtools_protocol::Message::PlatformCommand(PlatformCommand::OpenShellTarget(open))
        if open.target == source && open.action == ShellOpenAction::Open));
    window.simulate_click(action.bounds().center(), gpui_kit::Modifiers::default());
    window.run_until_parked();
    assert!(
        endpoint.try_receive_command_for_contract_test().is_err(),
        "pending opens coalesce"
    );
    application.update(window, |app, cx| {
        app.handle_platform_notification(
            PlatformNotification::CommandResult {
                correlation_id: command.id,
                result: PlatformCommandResult {
                    success: true,
                    error: None,
                    native_error_code: None,
                    data: None,
                },
            },
            cx,
        )
    });
    for failure in ["native", "timeout", "disconnect"] {
        window.simulate_click(action.bounds().center(), gpui_kit::Modifiers::default());
        window.run_until_parked();
        let command = endpoint
            .try_receive_command_for_contract_test()
            .expect("browser retry");
        application.update(window, |app, cx| {
            match failure {
                "native" => app.handle_platform_notification(
                    PlatformNotification::CommandResult {
                        correlation_id: command.id,
                        result: PlatformCommandResult {
                            success: false,
                            error: Some("Shell failed".into()),
                            native_error_code: Some(5),
                            data: None,
                        },
                    },
                    cx,
                ),
                "timeout" => app.artifact_action_timed_out(command.id, cx),
                _ => app.disconnect_artifact_actions(cx),
            }
            assert!(!app.pending_artifact_actions.contains_key(&command.id));
        });
        window.refresh().expect("browser failure retains window");
    }
    window
        .refresh()
        .expect("browser open retains failure presentation");
    view.read_with(window, |view, _| {
        assert_eq!(view.window_generation, generation);
        assert_eq!(
            view.snapshot.as_ref().expect("snapshot").task.status(),
            DownloadStatus::Failed
        );
    });
    assert_eq!(
        engine
            .task(task_id)
            .expect("task")
            .expect("record")
            .task
            .status(),
        DownloadStatus::Created
    );
}
