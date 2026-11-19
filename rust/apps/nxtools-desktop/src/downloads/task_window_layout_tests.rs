use super::*;
use gpui_kit::{AppContext, Modifiers, TestAppContext, VisualTestContext};

fn downloading_snapshot(engine: &DownloadEngine, id: DownloadTaskId) -> DownloadEngineTask {
    let mut snapshot = engine.task(id).expect("task").expect("snapshot");
    let at = snapshot.task.updated_at_unix_ms();
    for status in [
        DownloadStatus::Probing,
        DownloadStatus::Waiting,
        DownloadStatus::Downloading,
    ] {
        snapshot
            .task
            .transition_to(status, at)
            .expect("presentation state");
    }
    snapshot.total_bytes = Some(1024);
    snapshot.downloaded_bytes = 512;
    snapshot
}

fn progress(id: DownloadTaskId, written: u64) -> DownloadProgress {
    DownloadProgress {
        task_id: id,
        downloaded_bytes: written,
        total_bytes: Some(1024),
        bytes_per_second: 128,
        active_connections: 1,
        segments: vec![nxtools_download::DownloadSegmentProgress {
            segment_index: 0,
            start_byte: Some(0),
            total_bytes: Some(1024),
            downloaded_bytes: written,
        }],
        finishing: None,
    }
}

#[gpui_kit::test]
fn progress_layout_snapshot_gap_keeps_written_ranges_until_current_plan_or_retirement(
    cx: &mut TestAppContext,
) {
    let (application, engine, ids, _root) = crate::downloads::detail_request_tests::fixture(cx);
    let snapshot = downloading_snapshot(&engine, ids[0]);
    let live = progress(ids[0], 512);
    let (platform, _) = nxtools_ipc::platform_bridge(8);
    let view = cx.new(|_| {
        let mut view = DownloadTaskWindowView::new(
            application,
            Rc::new(platform),
            ids[0],
            TaskWindowIdentity {
                native_window_handle: 0,
                generation: uuid::Uuid::new_v4(),
            },
            Some(snapshot.clone()),
            Some(live.clone()),
            false,
        );
        view.persisted_range_progress = progress(ids[0], 64).segments;
        view
    });
    view.update(cx, |view, cx| {
        view.replace_snapshot(Some(snapshot.clone()), None, false, cx);
        assert_eq!(
            view.range_segments(),
            live.segments,
            "same-attempt snapshot refresh must not repaint an older checkpoint"
        );
        let mut split = live.clone();
        split.segments[0].total_bytes = Some(512);
        split
            .segments
            .push(nxtools_download::DownloadSegmentProgress {
                segment_index: 1,
                start_byte: Some(512),
                total_bytes: Some(512),
                downloaded_bytes: 16,
            });
        split.downloaded_bytes = 528;
        view.replace_snapshot(Some(snapshot.clone()), Some(split.clone()), false, cx);
        assert_eq!(
            view.range_segments(),
            split.segments,
            "new split plan replaces, never unions old ranges"
        );
        view.replace_snapshot(Some(snapshot.clone()), None, false, cx);
        assert_eq!(view.range_segments(), split.segments);
        for (field, value) in [
            (
                "completion_attempt",
                serde_json::json!(snapshot.task.completion_attempt() + 1),
            ),
            (
                "source_url",
                serde_json::json!("https://example.test/replacement.bin"),
            ),
        ] {
            view.replace_snapshot(Some(snapshot.clone()), Some(live.clone()), false, cx);
            let mut next = snapshot.clone();
            let mut task = serde_json::to_value(&next.task).expect("task wire");
            if field == "source_url" {
                task["request"][field] = value;
            } else {
                task[field] = value;
            }
            next.task = serde_json::from_value(task).expect("next transfer identity");
            view.replace_snapshot(Some(next), None, false, cx);
            assert!(
                view.live_range_progress.is_empty(),
                "{field} retires live ranges"
            );
            assert_eq!(view.range_segments(), progress(ids[0], 64).segments);
        }
        view.replace_snapshot(Some(snapshot.clone()), Some(live.clone()), false, cx);
        let mut paused = snapshot;
        let at = paused.task.updated_at_unix_ms();
        paused
            .task
            .transition_to(DownloadStatus::Paused, at)
            .expect("pause");
        view.replace_snapshot(Some(paused), None, false, cx);
        assert_eq!(
            view.range_segments(),
            progress(ids[0], 64).segments,
            "paused coverage uses durable facts"
        );
    });
}

#[gpui_kit::test]
fn progress_layout_separates_configuration_from_connection_details(cx: &mut TestAppContext) {
    let (application, engine, ids, _root) = crate::downloads::detail_request_tests::fixture(cx);
    cx.update(crate::theme::apply_component_theme);
    let snapshot = downloading_snapshot(&engine, ids[0]);
    let (platform, _) = nxtools_ipc::platform_bridge(8);
    let (view, window) = cx.add_window_view(|_, _| {
        let mut view = DownloadTaskWindowView::new(
            application,
            Rc::new(platform),
            ids[0],
            TaskWindowIdentity {
                native_window_handle: 0,
                generation: uuid::Uuid::new_v4(),
            },
            Some(snapshot),
            Some(progress(ids[0], 512)),
            false,
        );
        view.details_expanded = true;
        view
    });
    window.simulate_resize(download_task_window_size_for_details(
        DownloadWindowPhase::Progress,
        true,
    ));
    window.refresh().expect("expanded production view");
    let nodes = window.update(|window, _| gpui_kit::base::test_support::snapshots(window));
    for label in ["连接详情", "限速", "完成后"] {
        assert!(nodes.iter().any(|node| node.role() == Some(gpui_kit::Role::Tab) && node.label() == Some(label)), "missing actual {label} tab");
    }
    assert!(
        !nodes
            .iter()
            .any(|node| node.label() == Some("任务限速 KiB/s")),
        "connection page must not mount the speed editor"
    );
    assert!(
        !nodes
            .iter()
            .any(|node| node.role() == Some(gpui_kit::Role::CheckBox)
                && node.label() == Some("完成后关机")),
        "no duplicated footer power controls"
    );
    let footer = window
        .debug_bounds("download-task-window-footer")
        .expect("fixed footer");
    let table = window
        .debug_bounds("download-window-segment-table")
        .expect("connection viewport");
    assert!(
        table.size.height >= px(96.0) && table.bottom() <= footer.top(),
        "at least three rows and table header fit above footer: {table:?}"
    );
    let speed_tab = nodes
        .iter()
        .find(|node| node.role() == Some(gpui_kit::Role::Tab) && node.label() == Some("限速"))
        .expect("speed tab")
        .bounds();
    window.simulate_click(speed_tab.center(), Modifiers::default());
    window.run_until_parked();
    window.refresh().expect("speed page");
    let input = window
        .debug_bounds("download-task-speed-limit-input")
        .expect("original input");
    assert_eq!(
        input.size.height,
        px(32.0),
        "official Medium input has one 32-DIP frame"
    );
    assert!(input.bottom() < footer.top());
    assert!(view.read_with(window, |view, _| view.details_expanded));
}

pub(crate) fn open_completion_page(surface: &mut VisualTestContext) {
    surface.refresh().expect("current task frame");
    let options = surface
        .debug_bounds("download-window-options")
        .expect("options entry");
    surface.simulate_click(options.center(), Modifiers::default());
    surface.run_until_parked();
    surface.simulate_resize(download_task_window_size_for_details(
        DownloadWindowPhase::Progress,
        true,
    ));
    surface.refresh().expect("expanded options");
    let tab = surface
        .update(|window, _| gpui_kit::base::test_support::snapshots(window))
        .into_iter()
        .find(|node| node.role() == Some(gpui_kit::Role::Tab) && node.label() == Some("完成后"))
        .expect("completion tab")
        .bounds();
    surface.simulate_click(tab.center(), Modifiers::default());
    surface.run_until_parked();
    surface.refresh().expect("completion page");
}

pub(crate) fn choose_completion_action(surface: &mut VisualTestContext, label: &str) {
    open_completion_page(surface);
    let menu = surface
        .debug_bounds("download-window-completion-menu")
        .expect("completion action picker");
    surface.simulate_click(menu.center(), Modifiers::default());
    surface.run_until_parked();
    surface.refresh().expect("completion choices");
    let item = surface
        .update(|window, _| gpui_kit::base::test_support::snapshots(window))
        .into_iter()
        .find(|node| node.label() == Some(label))
        .expect("requested action")
        .bounds();
    surface.simulate_click(item.center(), Modifiers::default());
    surface.run_until_parked();
    surface.refresh().expect("saved completion intention");
}

#[gpui_kit::test]
fn progress_layout_completion_directory_uses_original_persisted_intention(cx: &mut TestAppContext) {
    let (application, engine, ids, _root) = crate::downloads::detail_request_tests::fixture(cx);
    cx.update(crate::theme::apply_component_theme);
    application.update(cx, |app, cx| app.request_task_window(ids[0], cx));
    cx.run_until_parked();
    let handle = application.read_with(cx, |app, _| {
        app.task_windows.get(&ids[0]).expect("task window").window
    });
    let mut surface = VisualTestContext::from_window(handle, cx);
    open_completion_page(&mut surface);
    let checkbox = surface
        .update(|window, _| gpui_kit::base::test_support::snapshots(window))
        .into_iter()
        .find(|node| {
            node.role() == Some(gpui_kit::Role::CheckBox)
                && node.label() == Some("下载成功后打开所在文件夹")
        })
        .expect("original intention checkbox")
        .bounds();
    surface.simulate_click(checkbox.center(), Modifiers::default());
    surface.run_until_parked();
    assert!(
        engine
            .task(ids[0])
            .expect("task")
            .expect("same task")
            .task
            .completion_intention()
            .open_directory
    );
    application.update(&mut surface, |app, cx| {
        app.close_task_window(ids[0], cx);
        app.request_task_window(ids[0], cx);
    });
    surface.run_until_parked();
    assert!(application.read_with(&surface, |app, cx| {
        app.task_windows
            .get(&ids[0])
            .expect("reopened window")
            .view
            .read(cx)
            .completion_intention
            .open_directory
    }));
    assert_eq!(
        engine
            .task(ids[0])
            .expect("task")
            .expect("queued task")
            .task
            .status(),
        DownloadStatus::Created
    );
}

#[gpui_kit::test]
fn finishing_events_render_stage_and_keep_transfer_bytes_in_the_open_task_window(
    cx: &mut TestAppContext,
) {
    let (application, engine, ids, _root) = crate::downloads::detail_request_tests::fixture(cx);
    cx.update(crate::theme::apply_component_theme);
    let mut snapshot = downloading_snapshot(&engine, ids[0]);
    snapshot
        .task
        .transition_to(
            DownloadStatus::Finalizing,
            snapshot.task.updated_at_unix_ms(),
        )
        .expect("finalizing snapshot");
    let mut task_wire = serde_json::to_value(&snapshot.task).expect("serialize media fixture");
    task_wire["request"]["media"] = serde_json::json!({
        "kind": "dash",
        "data": {
            "manifest_url": snapshot.task.request().source_url,
            "video_representation_id": "video",
            "audio_representation_id": null
        }
    });
    snapshot.task = serde_json::from_value(task_wire).expect("media finalization fixture");
    let mut live = progress(ids[0], 1024);
    live.finishing = Some(nxtools_download::DownloadFinishingProgress {
        stage: nxtools_download::DownloadFinishingStage::Muxing,
        completed: 500_000,
        total: Some(2_000_000),
    });
    let (platform, _) = nxtools_ipc::platform_bridge(8);
    let (view, window) = cx.add_window_view(|_, _| {
        DownloadTaskWindowView::new(
            application,
            Rc::new(platform),
            ids[0],
            TaskWindowIdentity {
                native_window_handle: 0,
                generation: uuid::Uuid::new_v4(),
            },
            Some(snapshot.clone()),
            Some(live.clone()),
            false,
        )
    });
    window.refresh().expect("render mux stage");
    let nodes = window.update(|window, _| gpui_kit::base::test_support::snapshots(window));
    assert!(
        nodes.iter().any(|node| node.label() == Some("暂停下载")),
        "media finalization must retain pause"
    );
    assert!(
        nodes.iter().any(|node| node.label() == Some("取消")),
        "media finalization must retain cancellation"
    );
    assert!(
        nodes.iter().any(|node| node
            .label()
            .is_some_and(|label| label.contains("正在合成 · 25%"))),
        "labels: {:?}; progress: {:?}",
        nodes
            .iter()
            .filter_map(|node| node.label())
            .collect::<Vec<_>>(),
        window.update(|_, cx| view.read(cx).progress.clone())
    );
    assert_eq!(
        window.update(|_, cx| view
            .read(cx)
            .progress
            .as_ref()
            .map(|progress| progress.downloaded_bytes)),
        Some(1024)
    );
    live.finishing = Some(nxtools_download::DownloadFinishingProgress {
        stage: nxtools_download::DownloadFinishingStage::Publishing,
        completed: 0,
        total: None,
    });
    window.update(|_, cx| {
        view.update(cx, |view, cx| {
            view.replace_snapshot(Some(snapshot.clone()), Some(live), false, cx)
        })
    });
    window.refresh().expect("render unknown publish stage");
    let nodes = window.update(|window, _| gpui_kit::base::test_support::snapshots(window));
    assert!(nodes.iter().any(|node| node.label() == Some("正在发布")));
    assert!(!nodes.iter().any(|node| node.label() == Some("100%")));
}

#[test]
fn finishing_taskbar_uses_phase_units_and_unknown_totals_are_indeterminate() {
    let engine = DownloadEngine::open_in_memory().expect("open taskbar engine");
    let id = engine
        .enqueue(
            DownloadRequest {
                source_url: "https://example.test/media.mpd".into(),
                save_directory: std::env::temp_dir(),
                save_directory_explicit: false,
                file_name: Some("media.mp4".into()),
                headers: Vec::new(),
                cookie: None,
                referrer: None,
                source_page: None,
                certificate_policy: Default::default(),
                connection_limit: None,
                speed_limit_bytes_per_second: None,
                expected_total_bytes: None,
                scheduled_at_unix_ms: None,
                adaptive_segment_policy: None,
                media: None,
                category_id: None,
                category_name: None,
                duplicate_strategy: nxtools_download::DownloadDuplicateStrategy::UseDefault,
            },
            RequestOrigin::Internal,
        )
        .expect("enqueue taskbar fixture");
    let mut task = downloading_snapshot(&engine, id);
    task.task
        .transition_to(DownloadStatus::Finalizing, task.task.updated_at_unix_ms())
        .expect("finalizing");
    let mut event = progress(id, 1024);
    event.finishing = Some(nxtools_download::DownloadFinishingProgress {
        stage: nxtools_download::DownloadFinishingStage::Concatenating,
        completed: 9,
        total: Some(9),
    });
    let command = taskbar_progress_command(42, &task, Some(&event));
    assert_eq!(
        command.state,
        nxtools_protocol::TaskbarProgressState::Normal
    );
    assert!(command.completed < command.total);
    event.finishing = Some(nxtools_download::DownloadFinishingProgress {
        stage: nxtools_download::DownloadFinishingStage::Muxing,
        completed: 500_000,
        total: Some(2_000_000),
    });
    let command = taskbar_progress_command(42, &task, Some(&event));
    assert_eq!((command.completed, command.total), (500_000, 2_000_000));
    event.finishing.as_mut().expect("mux").total = None;
    assert_eq!(
        taskbar_progress_command(42, &task, Some(&event)).state,
        nxtools_protocol::TaskbarProgressState::Indeterminate
    );
}
