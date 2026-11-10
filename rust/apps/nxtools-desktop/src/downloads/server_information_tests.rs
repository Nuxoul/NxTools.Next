use super::*;
use gpui_kit::{AppContext, Modifiers, TestAppContext, VisualTestContext};
use nxtools_download::{
    DownloadMediaProbe, DownloadRouteKind, DownloadServerObservation, ResourceValidator,
};

struct Fixture {
    application: Entity<DownloadApplication>,
    _directory: tempfile::TempDir,
    _endpoint: nxtools_ipc::PlatformServerEndpoint,
}

fn fixture(
    cx: &mut TestAppContext,
) -> (
    Fixture,
    DownloadTaskId,
    Entity<DownloadTaskWindowView>,
    VisualTestContext,
) {
    cx.update(gpui_kit::init);
    cx.update(crate::theme::apply_component_theme);
    let directory = tempfile::tempdir().expect("isolated server information surface");
    let paths = directory.path().to_path_buf();
    let mut config = AppConfig::default();
    config.download.default_directory = paths.to_string_lossy().into_owned();
    config
        .save_to(&paths.join("config.json"))
        .expect("synthetic config");
    let (platform, endpoint) = nxtools_ipc::platform_bridge(32);
    let application = cx.update(|cx| {
        let notifications = cx.new(|_| NotificationCenter::default());
        cx.new(|cx| {
            DownloadApplication::new(
                DownloadEngine::open_in_memory().expect("memory engine"),
                Rc::new(platform),
                notifications,
                DownloadRuntimeContext {
                    fallback_default_directory: paths.clone(),
                    config,
                    config_path: paths.join("config.json"),
                    log_directory: paths.clone(),
                    database_path: paths.join("downloads.sqlite3"),
                    ffmpeg_install_directory: paths.join("tools"),
                },
                cx,
            )
        })
    });
    let request: DownloadRequest = serde_json::from_value(serde_json::json!({
        "source_url": "https://synthetic.invalid/not-contacted.bin", "save_directory": paths,
        "file_name": "服务器信息合成任务.bin"
    }))
    .expect("queue-only request");
    let task_id = application.read_with(cx, |app, _| {
        app.engine
            .enqueue(request, RequestOrigin::Internal)
            .expect("queue only")
    });
    application.update(cx, |app, cx| {
        app.refresh(cx);
        let _ = app.configure_task_window_activation_test(42);
        app.request_task_window(task_id, cx);
    });
    cx.run_until_parked();
    let (handle, view) = cx.read(|cx| {
        let app = application.read(cx);
        let entry = app
            .task_windows
            .get(&task_id)
            .expect("original keyed task window");
        (entry.window, entry.view.clone())
    });
    let mut surface = VisualTestContext::from_window(handle, cx);
    surface.simulate_resize(size(px(440.0), px(184.0)));
    surface.update(|window, _| window.activate_window());
    draw(&mut surface);
    (
        Fixture {
            application,
            _directory: directory,
            _endpoint: endpoint,
        },
        task_id,
        view,
        surface,
    )
}

fn draw(surface: &mut VisualTestContext) {
    surface.run_until_parked();
    surface.update(|window, cx| {
        window.refresh();
        window.draw(cx).clear(cx);
    });
}

fn observation_with_ms(
    route_kind: DownloadRouteKind,
    response_ms: u64,
) -> DownloadServerObservation {
    DownloadServerObservation {
        host: format!("{}.synthetic.invalid", "long-endpoint-".repeat(12)),
        peer_ip: Some(
            "2001:db8:1234:5678:abcd:ef01:2345:6789"
                .parse()
                .expect("synthetic IPv6"),
        ),
        route_kind,
        response_ms,
    }
}

fn observation(route_kind: DownloadRouteKind) -> DownloadServerObservation {
    observation_with_ms(route_kind, 123)
}

fn set_probe(
    view: &Entity<DownloadTaskWindowView>,
    surface: &mut VisualTestContext,
    observation: Option<DownloadServerObservation>,
    media: Option<DownloadMediaProbe>,
) {
    view.update_in(surface, |view, _, cx| {
        let task = view.snapshot.as_mut().expect("task snapshot");
        task.probe = Some(DownloadProbe {
            final_url: "https://different.invalid/manifest-or-file".into(),
            file_name: Some("synthetic.bin".into()),
            total_bytes: Some(16),
            supports_ranges: true,
            content_type: None,
            validator: ResourceValidator::default(),
            media,
            resolution: None,
            server_observation: observation,
        });
        cx.notify();
    });
    draw(surface);
}

fn reveal_server(surface: &mut VisualTestContext) {
    for _ in 0..12 {
        draw(surface);
        let row = surface
            .debug_bounds("download-window-server-value")
            .expect("production server row");
        let body = surface
            .debug_bounds("download-task-window-body")
            .expect("original body viewport");
        if row.top() >= body.top() && row.bottom() <= body.bottom() {
            return;
        }
        surface.simulate_event(gpui_kit::ScrollWheelEvent {
            position: gpui_kit::point(body.right() - px(8.0), body.center().y),
            delta: gpui_kit::ScrollDelta::Lines(gpui_kit::point(0.0, -1.0)),
            ..Default::default()
        });
    }
    panic!("server row not fully reachable in original compact viewport");
}

fn server_label(surface: &mut VisualTestContext) -> String {
    surface
        .update(|window, _| gpui_kit::base::test_support::snapshots(window))
        .into_iter()
        .find(|node| node.path().last() == Some(&"download-window-server-value".into()))
        .expect("real server semantic node")
        .label()
        .expect("server label")
        .to_string()
}

#[gpui_kit::test]
fn server_info_production_unknown_and_routes_are_bounded_with_original_actions(
    cx: &mut TestAppContext,
) {
    let (_f, _, view, mut surface) = fixture(cx);
    reveal_server(&mut surface);
    assert!(server_label(&mut surface).contains("待探测"));
    assert!(
        surface
            .debug_bounds("download-window-server-time")
            .is_none(),
        "unknown observations must not invent a zero-millisecond value or tone"
    );
    let footer = surface
        .debug_bounds("download-task-window-footer")
        .expect("footer");
    for (route, copy) in [
        (DownloadRouteKind::Direct, "直连"),
        (DownloadRouteKind::ManualProxy, "代理"),
        (DownloadRouteKind::SystemPolicy, "系统策略"),
    ] {
        let value = observation(route);
        set_probe(&view, &mut surface, Some(value.clone()), None);
        reveal_server(&mut surface);
        let label = server_label(&mut surface);
        assert!(label.contains(&value.host));
        assert!(!label.contains("2001:db8") && !label.contains("IP"));
        assert!(label.contains(copy));
        assert!(label.contains("123 ms"));
        if route == DownloadRouteKind::SystemPolicy {
            assert!(!label.contains("经代理") && !label.contains("已使用代理"));
        }
        let row = surface
            .debug_bounds("download-window-server-value")
            .expect("server");
        assert!(row.left() >= px(16.0) && row.right() <= px(424.0));
        assert!(row.size.height <= px(32.5));
        assert!(row.bottom() <= footer.top());
        let host = surface
            .debug_bounds("download-window-server-host")
            .expect("bounded host");
        let route = surface
            .debug_bounds("download-window-server-route")
            .expect("short route rail");
        let time = surface
            .debug_bounds("download-window-server-time")
            .expect("response time rail");
        assert!(host.size.width > px(16.0) && host.right() <= route.left());
        assert!(route.right() <= time.left() && time.right() <= row.right() + px(0.5));
        assert!(
            surface
                .debug_bounds("download-window-server-peer")
                .is_none(),
            "peer IP must not have a visible presentation rail"
        );
        assert_eq!(
            surface.debug_bounds("download-task-window-footer"),
            Some(footer)
        );
        let cancel = surface
            .debug_bounds("download-window-cancel")
            .expect("original cancel");
        assert!(cancel.top() >= footer.top() && cancel.bottom() <= footer.bottom());
    }
    set_probe(&view, &mut surface, None, None);
    reveal_server(&mut surface);
    let label = server_label(&mut surface);
    assert!(label.contains("未记录"));
    assert!(!label.contains("0 ms") && !label.contains("different.invalid"));
    assert!(
        surface
            .debug_bounds("download-window-server-time")
            .is_none()
    );
}

#[gpui_kit::test]
fn server_info_response_time_thresholds_use_semantic_theme_colors(cx: &mut TestAppContext) {
    let (_f, _, view, mut surface) = fixture(cx);
    for response_ms in [0, 100, 101, 300, 301] {
        set_probe(
            &view,
            &mut surface,
            Some(observation_with_ms(DownloadRouteKind::Direct, response_ms)),
            None,
        );
        reveal_server(&mut surface);
        assert!(server_label(&mut surface).contains(&format!("{response_ms} ms")));
        let time = surface
            .debug_bounds("download-window-server-time")
            .expect("colored production response time");
        let row = surface
            .debug_bounds("download-window-server-value")
            .expect("server row");
        assert!(time.top() >= row.top() && time.bottom() <= row.bottom());
    }
    let source = include_str!("task_window.rs");
    for required in [
        "response_ms <= 100",
        "response_ms <= 300",
        "theme.success",
        "theme.warning",
        "theme.danger",
    ] {
        assert!(
            source.contains(required),
            "production response-time color boundary missing: {required}"
        );
    }
}

#[test]
fn server_info_renderer_does_not_project_peer_ip_into_visible_or_tooltip_copy() {
    let source = include_str!("task_window.rs");
    assert!(
        !source.contains("value.peer_ip"),
        "server rendering must not read peer IP for visible, ARIA, or tooltip copy"
    );
}

#[gpui_kit::test]
fn server_info_media_peer_absent_and_blob_are_truthful(cx: &mut TestAppContext) {
    let (_f, _, view, mut surface) = fixture(cx);
    let mut observed = observation(DownloadRouteKind::SystemPolicy);
    observed.peer_ip = None;
    for media in [
        DownloadMediaProbe::Hls(nxtools_download::HlsProbe {
            is_master: false,
            variants: Vec::new(),
        }),
        DownloadMediaProbe::Dash(nxtools_download::DashProbe {
            video_tracks: Vec::new(),
            audio_tracks: Vec::new(),
        }),
    ] {
        set_probe(&view, &mut surface, Some(observed.clone()), Some(media));
        reveal_server(&mut surface);
        let label = server_label(&mut surface);
        assert!(label.contains("清单探测端点"));
        assert!(label.contains(&observed.host));
        assert!(!label.contains("different.invalid") && !label.contains("2001:db8"));
    }
    view.update_in(&mut surface, |view, _, cx| {
        let probe = view
            .snapshot
            .as_mut()
            .expect("snapshot")
            .probe
            .as_mut()
            .expect("probe");
        probe.final_url = "browser-blob://staged".into();
        probe.media = None;
        probe.server_observation = None;
        cx.notify();
    });
    draw(&mut surface);
    reveal_server(&mut surface);
    assert!(server_label(&mut surface).contains("不可用"));
}

#[gpui_kit::test]
fn server_info_close_reopen_and_cancel_keep_original_task_ownership(cx: &mut TestAppContext) {
    let (f, task_id, view, mut surface) = fixture(cx);
    set_probe(
        &view,
        &mut surface,
        Some(observation(DownloadRouteKind::Direct)),
        None,
    );
    let (handle, generation) = f.application.read_with(&surface, |app, _| {
        let entry = app.task_windows.get(&task_id).expect("window");
        (entry.window, entry.generation)
    });
    f.application
        .update(&mut surface, |app, cx| app.request_task_window(task_id, cx));
    draw(&mut surface);
    assert_eq!(
        f.application
            .read_with(&surface, |app, _| app.task_windows.len()),
        1
    );
    assert_eq!(
        view.read_with(&surface, |view, _| view.window_generation),
        generation
    );
    f.application
        .update(cx, |app, cx| app.close_task_window(task_id, cx));
    cx.run_until_parked();
    assert!(handle.update(cx, |_, _, _| ()).is_err());
    assert_eq!(
        f.application.read_with(cx, |app, _| app
            .engine
            .task(task_id)
            .expect("task lookup")
            .expect("original task")
            .task
            .status()),
        DownloadStatus::Created
    );
    f.application
        .update(cx, |app, cx| app.request_task_window(task_id, cx));
    cx.run_until_parked();
    let reopened = f.application.read_with(cx, |app, _| {
        app.task_windows.get(&task_id).expect("reopen").window
    });
    let mut reopened = VisualTestContext::from_window(reopened, cx);
    draw(&mut reopened);
    let cancel = reopened
        .debug_bounds("download-window-cancel")
        .expect("original explicit cancel");
    reopened.simulate_click(cancel.center(), Modifiers::default());
    cx.run_until_parked();
    assert_eq!(
        f.application.read_with(cx, |app, _| app
            .engine
            .task(task_id)
            .expect("task lookup")
            .expect("same task")
            .task
            .status()),
        DownloadStatus::Cancelled
    );
}

#[gpui_kit::test]
fn server_info_compact_task_window_retrieves_original_selected_probe_and_reopens_it(
    cx: &mut TestAppContext,
) {
    let (f, task_id, view, mut surface) = fixture(cx);
    let observed = observation(DownloadRouteKind::ManualProxy);
    set_probe(&view, &mut surface, Some(observed.clone()), None);
    let original = view.read_with(&surface, |view, _| {
        view.snapshot.clone().expect("original task")
    });
    let database = f._directory.path().join("selected.sqlite3");
    {
        let store =
            nxtools_download::SqliteDownloadStore::open(&database).expect("real task store");
        store.save_task(&original.task).expect("same original task");
        let connection = rusqlite::Connection::open(&database).expect("synthetic original probe");
        connection.execute("UPDATE download_tasks SET request_json = json_set(request_json, '$.probe', json(?1)) WHERE task_id = ?2",
            rusqlite::params![serde_json::to_string(&original.probe).expect("probe JSON"), task_id.as_uuid().to_string()]).expect("persist original probe envelope");
    }
    f.application.update(&mut surface, |app, cx| {
        app.close_task_window(task_id, cx);
        app.engine = DownloadEngine::open(&database).expect("reopen actual store");
        app.refresh(cx);
        assert!(
            app.task(task_id).expect("compact task").probe.is_none(),
            "compact list does not duplicate the original observation"
        );
        app.request_task_window(task_id, cx);
    });
    cx.run_until_parked();
    for reopen in [false, true] {
        if reopen {
            f.application.update(cx, |app, cx| {
                app.close_task_window(task_id, cx);
                app.request_task_window(task_id, cx);
            });
            cx.run_until_parked();
        }
        let handle = f.application.read_with(cx, |app, _| {
            app.task_windows.get(&task_id).expect("original ID").window
        });
        let mut opened = VisualTestContext::from_window(handle, cx);
        draw(&mut opened);
        reveal_server(&mut opened);
        let label = server_label(&mut opened);
        assert!(
            label.contains(&observed.host) && label.contains("代理"),
            "selected task window lost stored observation: {label}"
        );
    }
}

#[gpui_kit::test]
fn server_info_delayed_task_probe_updates_same_window_without_replacing_lifecycle(
    cx: &mut TestAppContext,
) {
    let (f, task_id, view, mut surface) = fixture(cx);
    reveal_server(&mut surface);
    assert!(server_label(&mut surface).contains("待探测"));
    let (handle, generation) = f.application.read_with(&surface, |app, _| {
        let entry = app
            .task_windows
            .get(&task_id)
            .expect("original keyed window");
        (entry.window, entry.generation)
    });
    let delayed = view.downgrade();
    surface.update(|_, cx| {
        cx.defer(move |cx| {
            let _ = delayed.update(cx, |view, cx| {
                let task = view.snapshot.as_mut().expect("same task snapshot");
                task.probe = Some(DownloadProbe {
                    final_url: "https://different.invalid/final".into(),
                    file_name: Some("synthetic.bin".into()),
                    total_bytes: Some(16),
                    supports_ranges: true,
                    content_type: None,
                    validator: ResourceValidator::default(),
                    media: None,
                    resolution: None,
                    server_observation: Some(observation_with_ms(
                        DownloadRouteKind::SystemPolicy,
                        301,
                    )),
                });
                cx.notify();
            });
        });
    });
    draw(&mut surface);
    reveal_server(&mut surface);
    let label = server_label(&mut surface);
    assert!(label.contains("系统策略") && label.contains("301 ms"));
    assert!(!label.contains("2001:db8"));
    assert_eq!(
        f.application.read_with(&surface, |app, _| {
            let entry = app.task_windows.get(&task_id).expect("same keyed window");
            (entry.window, entry.generation, app.task_windows.len())
        }),
        (handle, generation, 1)
    );
    assert_eq!(
        view.read_with(&surface, |view, _| view.window_generation),
        generation
    );
}
