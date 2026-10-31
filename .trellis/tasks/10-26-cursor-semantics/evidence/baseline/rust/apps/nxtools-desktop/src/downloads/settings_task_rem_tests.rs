//! Queue06 production Root regressions for ordinary settings/task roles.
use super::*;
use gpui_kit::component::Theme as ComponentTheme;
use gpui_kit::{AppContext, Modifiers, TestAppContext, VisualTestContext};

struct Fixture {
    application: Entity<DownloadApplication>,
    directory: tempfile::TempDir,
    _endpoint: nxtools_ipc::PlatformServerEndpoint,
}

fn fixture(cx: &mut TestAppContext) -> Fixture {
    cx.update(gpui_kit::init);
    cx.update(crate::theme::apply_component_theme);
    let directory = tempfile::tempdir().expect("isolated settings/task rem fixture");
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
                DownloadEngine::open_in_memory().expect("memory engine, no execution"),
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
    Fixture {
        application,
        directory,
        _endpoint: endpoint,
    }
}

fn draw(surface: &mut VisualTestContext) {
    surface.run_until_parked();
    surface.update(|window, cx| {
        window.refresh();
        window.draw(cx).clear(cx);
    });
}

fn set_base(surface: &mut VisualTestContext, base: f32) {
    surface.update(|window, cx| {
        ComponentTheme::update(cx, |theme| theme.font_size = px(base));
        window.refresh();
    });
    draw(surface);
}

fn settings(
    f: &Fixture,
    cx: &mut TestAppContext,
) -> (Entity<DownloadSettingsView>, VisualTestContext) {
    f.application
        .update(cx, |app, cx| app.open_settings_window(cx));
    cx.run_until_parked();
    let (handle, view) = cx.read(|cx| {
        let app = f.application.read(cx);
        (
            app.settings_window.expect("production settings"),
            app.settings_view.clone().expect("original draft"),
        )
    });
    let mut surface = VisualTestContext::from_window(handle, cx);
    surface.simulate_resize(size(px(984.0), px(650.0)));
    surface.update(|window, _| window.activate_window());
    draw(&mut surface);
    (view, surface)
}

fn select_page(surface: &mut VisualTestContext, page: DownloadSettingsPage) {
    draw(surface);
    let id = gpui_kit::ElementId::from(gpui_kit::SharedString::from(format!("0-{}", page.index())));
    let hit = surface.update(|window, _| {
        gpui_kit::base::test_support::snapshots(window)
            .into_iter()
            .find(|node| node.path().last() == Some(&id))
            .expect("actual Settings navigation")
            .bounds()
    });
    surface.simulate_click(hit.center(), Modifiers::default());
    draw(surface);
}

fn request(f: &Fixture) -> DownloadRequest {
    DownloadRequest {
        source_url: "https://synthetic.invalid/ordinary.bin".into(),
        save_directory: f.directory.path().to_path_buf(),
        save_directory_explicit: false,
        file_name: Some("合成任务文件.bin".into()),
        headers: Vec::new(),
        cookie: None,
        referrer: None,
        certificate_policy: Default::default(),
        connection_limit: Some(4),
        speed_limit_bytes_per_second: None,
        expected_total_bytes: Some(4096),
        scheduled_at_unix_ms: None,
        adaptive_segment_policy: None,
        media: None,
        category_id: None,
        category_name: None,
        duplicate_strategy: nxtools_download::DownloadDuplicateStrategy::UseDefault,
    }
}

fn task(
    f: &Fixture,
    cx: &mut TestAppContext,
) -> (
    DownloadTaskId,
    Entity<DownloadTaskWindowView>,
    VisualTestContext,
) {
    let task_id = f.application.read_with(cx, |app, _| {
        app.engine
            .enqueue(request(f), RequestOrigin::Internal)
            .expect("queue only")
    });
    f.application.update(cx, |app, cx| {
        app.refresh(cx);
        app.request_task_window(task_id, cx);
    });
    cx.run_until_parked();
    let (handle, view) = cx.read(|cx| {
        let app = f.application.read(cx);
        let entry = app
            .task_windows
            .get(&task_id)
            .expect("keyed production task");
        (entry.window, entry.view.clone())
    });
    let mut surface = VisualTestContext::from_window(handle, cx);
    surface.simulate_resize(size(px(440.0), px(184.0)));
    surface.update(|window, _| window.activate_window());
    draw(&mut surface);
    (task_id, view, surface)
}

#[gpui_kit::test]
fn audit06_aux_settings_ordinary_input_and_category_roles_follow_root(cx: &mut TestAppContext) {
    let f = fixture(cx);
    let (_, mut surface) = settings(&f, cx);
    let original = surface.read(|cx| ComponentTheme::global(cx).clone());
    select_page(&mut surface, DownloadSettingsPage::Transfer);
    for base in [16.0, 14.0, 18.0] {
        set_base(&mut surface, base);
        let input = surface
            .debug_bounds("download-settings-connection-limit-field")
            .expect("original Input rail");
        let footer = surface
            .debug_bounds("download-settings-shell-footer")
            .expect("fixed footer");
        assert!(
            (input.size.width - px(88.0 * base / 16.0)).abs() <= px(0.5),
            "ordinary input rail base={base}: {input:?}"
        );
        assert_eq!(input.size.height, px(32.0 * base / 16.0));
        assert_eq!(footer.size.height, px(48.0));
    }
    surface.update(|_, cx| ComponentTheme::update(cx, |theme| *theme = original));
}

#[gpui_kit::test]
fn audit06_aux_task_metrics_and_identity_follow_root_inside_fixed_client(cx: &mut TestAppContext) {
    let f = fixture(cx);
    let (_, _, mut surface) = task(&f, cx);
    let original = surface.read(|cx| ComponentTheme::global(cx).clone());
    for base in [16.0, 14.0, 18.0] {
        set_base(&mut surface, base);
        let metric = surface
            .debug_bounds("download-window-metric-速度-value")
            .expect("production metric text");
        let identity = surface
            .debug_bounds("download-window-progress-identity")
            .expect("identity");
        let root = surface
            .debug_bounds("download-task-window-shell")
            .expect("root");
        let footer = surface
            .debug_bounds("download-task-window-footer")
            .expect("footer");
        assert!(
            (metric.size.height - px(16.0 * base / 16.0)).abs() <= px(0.5),
            "ordinary metric line base={base}: {metric:?}"
        );
        assert_eq!(identity.size.height, px(32.0 * base / 16.0));
        assert_eq!(root.size, size(px(440.0), px(184.0)));
        assert_eq!(footer.size.height, px(44.0));
        let body = surface
            .debug_bounds("download-task-window-body")
            .expect("fixed content viewport");
        let before = surface
            .debug_bounds("download-window-progress-capabilities")
            .expect("capability line");
        wheel(&mut surface, body.center(), -5.0, 12);
        let after = surface
            .debug_bounds("download-window-progress-capabilities")
            .expect("reachable capability line");
        assert!(
            after.bottom() <= footer.top() + px(0.5) && after.bottom() > body.top(),
            "collapsed content end reachable base={base}: {after:?}"
        );
        if before.bottom() > footer.top() {
            assert!(after.top() < before.top(), "overflow must actually scroll");
        }
        assert_eq!(
            surface.debug_bounds("download-task-window-footer"),
            Some(footer)
        );
    }
    surface.update(|_, cx| ComponentTheme::update(cx, |theme| *theme = original));
}

fn key(surface: &mut VisualTestContext, key: &str) {
    let handle = surface.update(|window, _| window.window_handle());
    surface.simulate_event(gpui_kit::KeyDownEvent {
        keystroke: gpui_kit::Keystroke::parse(key).expect("test key"),
        is_held: false,
        prefer_character_input: false,
    });
    surface.simulate_event(gpui_kit::KeyUpEvent {
        keystroke: gpui_kit::Keystroke::parse(key).expect("test release"),
    });
    surface.run_until_parked();
    if surface.read(|cx| cx.windows().contains(&handle)) {
        draw(surface);
    }
}

fn tab_button(surface: &mut VisualTestContext, id: &'static str) {
    surface.update(|window, cx| {
        if window.focused(cx).is_none() {
            window.focus_next(cx);
        }
    });
    draw(surface);
    for _ in 0..100 {
        key(surface, "tab");
        if surface.update(|window, _| {
            gpui_kit::base::test_support::snapshots(window)
                .iter()
                .any(|node| {
                    node.focused() == Some(true)
                        && node.path().last() == Some(&gpui_kit::ElementId::from(id))
                })
        }) {
            return;
        }
    }
    let focused = surface.update(|window, _| {
        gpui_kit::base::test_support::snapshots(window)
            .into_iter()
            .filter(|node| node.focused() == Some(true))
            .map(|node| format!("{:?}", node.path()))
            .collect::<Vec<_>>()
    });
    panic!("actual Tab cannot reach original Button {id}, current={focused:?}");
}

fn wheel(
    surface: &mut VisualTestContext,
    hit: gpui_kit::Point<gpui_kit::Pixels>,
    lines: f32,
    count: usize,
) {
    for _ in 0..count {
        surface.simulate_event(gpui_kit::ScrollWheelEvent {
            position: hit,
            delta: gpui_kit::ScrollDelta::Lines(gpui_kit::point(0.0, lines)),
            ..Default::default()
        });
        draw(surface);
    }
}

fn reveal(
    surface: &mut VisualTestContext,
    selector: &'static str,
) -> gpui_kit::Bounds<gpui_kit::Pixels> {
    for _ in 0..80 {
        draw(surface);
        let content = surface
            .debug_bounds("download-settings-shell-content")
            .expect("content");
        let footer = surface
            .debug_bounds("download-settings-shell-footer")
            .expect("footer");
        if let Some(hit) = surface.debug_bounds(selector)
            && hit.top() >= content.top()
            && hit.bottom() <= footer.top()
        {
            return hit;
        }
        wheel(
            surface,
            gpui_kit::point(content.right() - px(30.0), content.center().y),
            -3.0,
            1,
        );
    }
    panic!("setting not reachable: {selector}");
}

#[gpui_kit::test]
fn audit06_aux_settings_long_category_error_scroll_and_once_actions(cx: &mut TestAppContext) {
    for dimensions in [(984.0, 650.0), (720.0, 480.0)] {
        for base in [16.0, 14.0, 18.0] {
            let f = fixture(cx);
            let (view, mut surface) = settings(&f, cx);
            let original = surface.read(|cx| ComponentTheme::global(cx).clone());
            surface.simulate_resize(size(px(dimensions.0), px(dimensions.1)));
            set_base(&mut surface, base);
            surface.update(|window, cx| {
                view.update(cx, |view, cx| {
                    view.categories = (0..24)
                        .map(|index| {
                            DownloadCategoryEditor::new(
                                DownloadCategorySettings {
                                    id: format!("rem-{index}"),
                                    name: format!("合成分类{index:02}{}", "长中文名称".repeat(4)),
                                    save_directory: format!(
                                        "D:\\合成目录\\{index}\\{}",
                                        "很长的路径层级\\".repeat(12)
                                    ),
                                    enabled: true,
                                    extensions: Vec::new(),
                                    url_patterns: Vec::new(),
                                },
                                window,
                                cx,
                            )
                        })
                        .collect();
                    view.selected_category = Some(0);
                    cx.notify();
                })
            });
            let heading = reveal(&mut surface, "download-settings-category-heading");
            assert!((heading.size.height - px(32.0 * base / 16.0)).abs() <= px(0.5));
            let _ = reveal(&mut surface, "download-settings-category-add");
            let outer = surface
                .debug_bounds("download-settings-category-master")
                .expect("fixed master budget");
            let add = surface
                .debug_bounds("download-settings-category-add")
                .expect("original action rail");
            let content = surface
                .debug_bounds("download-settings-shell-content")
                .expect("bounded content");
            let footer = surface
                .debug_bounds("download-settings-shell-footer")
                .expect("footer");
            let master = gpui_kit::Bounds::from_corners(
                gpui_kit::point(
                    outer.left() + px(1.0),
                    outer.top().max(content.top() + px(64.0)),
                ),
                gpui_kit::point(outer.right() - px(1.0), add.top().min(footer.top())),
            );
            wheel(&mut surface, master.center(), -8.0, 24);
            let last = surface
                .debug_bounds("download-settings-category-23")
                .expect("last category");
            assert!(
                last.top() >= master.top() && last.bottom() <= master.bottom() + px(0.5),
                "last original row reachable base={base} dimensions={dimensions:?}: {last:?} {master:?}"
            );
            let action_id = gpui_kit::ElementId::from(gpui_kit::SharedString::from(
                "download-settings-category-23-action",
            ));
            let hit = surface.update(|window, _| {
                gpui_kit::base::test_support::snapshots(window)
                    .into_iter()
                    .find(|node| node.path().last() == Some(&action_id))
                    .expect("actual category Button")
                    .bounds()
            });
            surface.simulate_click(hit.center(), Modifiers::default());
            draw(&mut surface);
            assert_eq!(
                view.read_with(&surface, |v, _| v.selected_category),
                Some(23),
                "actual category Button hit={hit:?} viewport={master:?} row={last:?}"
            );
            let control = reveal(&mut surface, "download-settings-category-up-control");
            surface.simulate_click(control.center(), Modifiers::default());
            draw(&mut surface);
            assert_eq!(
                view.read_with(&surface, |v, _| (
                    v.selected_category,
                    v.category_action_invocations
                )),
                (Some(22), 1)
            );
            tab_button(&mut surface, "download-settings-category-up");
            key(&mut surface, "space");
            assert_eq!(
                view.read_with(&surface, |v, _| (
                    v.selected_category,
                    v.category_action_invocations
                )),
                (Some(21), 2)
            );
            view.update(&mut surface, |v, cx| {
                v.error = Some(
                    "合成中文验证错误与完整路径 D:\\合成目录\\对象.txt\n".repeat(35)
                        + "验证全文末尾",
                );
                cx.notify();
            });
            draw(&mut surface);
            let viewport = surface
                .debug_bounds("download-settings-error-scroll")
                .expect("bounded error");
            let before = surface
                .debug_bounds("download-inline-error")
                .expect("complete Alert layout");
            wheel(&mut surface, viewport.center(), -8.0, 60);
            let after = surface
                .debug_bounds("download-inline-error")
                .expect("current error");
            let footer = surface
                .debug_bounds("download-settings-shell-footer")
                .expect("fixed footer");
            let apply = surface
                .debug_bounds("download-settings-apply")
                .expect("Apply");
            assert!(before.size.height > viewport.size.height && after.top() < before.top());
            assert!(
                after.bottom() <= viewport.bottom() + px(0.5) && after.bottom() > viewport.top(),
                "complete settings error end"
            );
            assert!(
                viewport.bottom() <= footer.top()
                    && apply.top() >= footer.top()
                    && apply.bottom() <= footer.bottom()
            );
            assert_eq!(
                surface
                    .debug_bounds("download-settings-shell")
                    .expect("root")
                    .size,
                size(px(dimensions.0), px(dimensions.1))
            );
            surface.update(|_, cx| ComponentTheme::update(cx, |theme| *theme = original));
        }
    }
}

#[gpui_kit::test]
fn audit06_aux_settings_pointer_apply_and_tab_cancel_keep_section_merge(cx: &mut TestAppContext) {
    for base in [16.0, 14.0, 18.0] {
        let f = fixture(cx);
        let (view, mut surface) = settings(&f, cx);
        let original = surface.read(|cx| ComponentTheme::global(cx).clone());
        set_base(&mut surface, base);
        select_page(&mut surface, DownloadSettingsPage::Transfer);
        let input = reveal(&mut surface, "download-settings-connection-limit-field");
        surface.simulate_click(input.center(), Modifiers::default());
        surface.simulate_keystrokes("ctrl-a");
        surface.simulate_input("7");
        draw(&mut surface);
        assert_eq!(
            view.read_with(&surface, |v, cx| v.connection_limit.read(cx).value()),
            "7"
        );
        let path = f.directory.path().join("config.json");
        let mut external: AppConfig =
            serde_json::from_slice(&std::fs::read(&path).expect("synthetic disk bytes"))
                .expect("synthetic current disk");
        external.clipboard.max_items = 3210;
        external
            .save_to(&path)
            .expect("independent synthetic section write");
        let external_disk: AppConfig =
            serde_json::from_slice(&std::fs::read(&path).expect("independent write bytes"))
                .expect("read independent write");
        assert_eq!(
            external_disk.clipboard.max_items, 3210,
            "independent synthetic write persisted"
        );
        let expected_clipboard = external.clipboard.max_items;
        let apply = surface
            .debug_bounds("download-settings-apply")
            .expect("Apply");
        surface.simulate_click(apply.center(), Modifiers::default());
        surface.run_until_parked();
        let saved: AppConfig =
            serde_json::from_slice(&std::fs::read(&path).expect("original Apply bytes"))
                .expect("original Apply persistence");
        assert_eq!(saved.download.default_connection_limit, 7);
        assert_eq!(
            saved.clipboard.max_items, expected_clipboard,
            "original section merge preserves newer unrelated section"
        );
        assert_eq!(
            f.application
                .read_with(cx, |app, _| app.default_connection_limit()),
            7
        );
        let (reopened, mut surface) = settings(&f, cx);
        set_base(&mut surface, base);
        select_page(&mut surface, DownloadSettingsPage::Transfer);
        let input = reveal(&mut surface, "download-settings-connection-limit-field");
        surface.simulate_click(input.center(), Modifiers::default());
        surface.simulate_keystrokes("ctrl-a");
        surface.simulate_input("9");
        draw(&mut surface);
        assert_eq!(
            reopened.read_with(&surface, |v, cx| v.connection_limit.read(cx).value()),
            "9"
        );
        let bytes = std::fs::read(&path).expect("before Cancel");
        tab_button(&mut surface, "download-settings-cancel-button");
        key(&mut surface, "space");
        assert_eq!(std::fs::read(&path).expect("after Cancel"), bytes);
        assert_eq!(
            f.application
                .read_with(cx, |app, _| app.default_connection_limit()),
            7
        );
        cx.update(|cx| ComponentTheme::update(cx, |theme| *theme = original));
    }
}

#[gpui_kit::test]
fn audit06_aux_task_details_last_segment_and_real_toggle_cancel_keep_budget(
    cx: &mut TestAppContext,
) {
    for base in [16.0, 14.0, 18.0] {
        let f = fixture(cx);
        let (task_id, view, mut surface) = task(&f, cx);
        let original = surface.read(|cx| ComponentTheme::global(cx).clone());
        set_base(&mut surface, base);
        view.update(&mut surface, |v, cx| {
            v.progress = Some(DownloadProgress {
                task_id,
                downloaded_bytes: 512,
                total_bytes: Some(4096),
                bytes_per_second: 0,
                active_connections: 0,
                segments: (0..40)
                    .map(|segment_index| nxtools_download::DownloadSegmentProgress {
                        segment_index,
                        downloaded_bytes: 32,
                        total_bytes: Some(1024),
                    })
                    .collect(),
            });
            cx.notify();
        });
        draw(&mut surface);
        let toggle = surface
            .debug_bounds("download-window-details-toggle")
            .expect("original disclosure");
        surface.simulate_click(toggle.center(), Modifiers::default());
        draw(&mut surface);
        assert!(view.read_with(&surface, |v, _| v.details_expanded));
        surface.simulate_resize(download_task_window_size_for_details(
            DownloadWindowPhase::Progress,
            true,
        ));
        draw(&mut surface);
        let viewport = surface
            .debug_bounds("download-window-segment-table")
            .expect("original table viewport");
        let before = surface
            .debug_bounds("download-window-segment-39-3")
            .expect("last cell before wheel");
        wheel(&mut surface, viewport.center(), -8.0, 40);
        let after = surface
            .debug_bounds("download-window-segment-39-3")
            .expect("last cell after wheel");
        assert!(
            after.top() < before.top()
                && after.bottom() <= viewport.bottom() + px(0.5)
                && after.bottom() > viewport.top(),
            "last segment remains reachable base={base}: {after:?} {viewport:?}"
        );
        assert!(
            after.right() <= viewport.right() + px(0.5),
            "same table columns remain inside content budget"
        );
        assert_eq!(
            surface
                .debug_bounds("download-task-window-shell")
                .expect("expanded root")
                .size,
            size(px(440.0), px(280.0))
        );
        let current = surface
            .debug_bounds("download-window-details-toggle")
            .expect("current disclosure hit");
        surface.simulate_click(current.center(), Modifiers::default());
        draw(&mut surface);
        assert!(
            !view.read_with(&surface, |v, _| v.details_expanded),
            "real pointer closes original details once"
        );
        tab_button(&mut surface, "download-window-details-toggle");
        key(&mut surface, "space");
        assert!(
            view.read_with(&surface, |v, _| v.details_expanded),
            "Space release toggles original disclosure once"
        );
        assert_eq!(
            f.application
                .read_with(&surface, |app, _| app.task_windows.len()),
            1
        );
        let cancel = surface
            .debug_bounds("download-window-cancel")
            .expect("original safe cancel");
        let footer = surface
            .debug_bounds("download-task-window-footer")
            .expect("fixed footer");
        assert!(cancel.top() >= footer.top() && cancel.bottom() <= footer.bottom());
        surface.simulate_click(cancel.center(), Modifiers::default());
        surface.run_until_parked();
        assert_eq!(
            f.application.read_with(cx, |app, _| app
                .task(task_id)
                .expect("retained canceled task")
                .task
                .status()),
            DownloadStatus::Cancelled
        );
        cx.update(|cx| ComponentTheme::update(cx, |theme| *theme = original));
    }
}

#[gpui_kit::test]
fn audit06_aux_task_failed_completed_long_content_and_close_keep_fixed_policy(
    cx: &mut TestAppContext,
) {
    for completed in [false, true] {
        for base in [16.0, 14.0, 18.0] {
            let f = fixture(cx);
            let (_, view, mut surface) = task(&f, cx);
            let original = surface.read(|cx| ComponentTheme::global(cx).clone());
            set_base(&mut surface, base);
            let long_path = f
                .directory
                .path()
                .join("合成中文路径层级".repeat(10))
                .join("完整中文文件名称".repeat(12) + ".bin");
            let long_error = "合成中文失败原因：完整错误与对象必须可读，原固定预算不得放大。\n"
                .repeat(40)
                + "失败全文末尾";
            view.update(&mut surface, |v, cx| {
                let mut snapshot = v.snapshot.clone().expect("original snapshot");
                snapshot
                    .task
                    .transition_to(DownloadStatus::Probing, 2)
                    .expect("synthetic probe state");
                if completed {
                    for (status, time) in [
                        (DownloadStatus::Waiting, 3),
                        (DownloadStatus::Downloading, 4),
                        (DownloadStatus::Finalizing, 5),
                        (DownloadStatus::Completed, 6),
                    ] {
                        snapshot
                            .task
                            .transition_to(status, time)
                            .expect("legal synthetic completed display");
                    }
                    snapshot.final_path = Some(long_path.clone());
                    snapshot.downloaded_bytes = 4096;
                    snapshot.total_bytes = Some(4096);
                } else {
                    snapshot
                        .task
                        .transition_to(DownloadStatus::Failed, 3)
                        .expect("synthetic failed display");
                    snapshot.error = Some(nxtools_download::DownloadFault {
                        code: nxtools_download::DownloadFaultCode::Network,
                        message: long_error.clone(),
                        retryable: true,
                    });
                }
                v.replace_snapshot(Some(snapshot), None, false, cx);
            });
            let phase = if completed {
                DownloadWindowPhase::Completed
            } else {
                DownloadWindowPhase::Failed
            };
            surface.simulate_resize(download_task_window_size(phase));
            draw(&mut surface);
            let root = surface
                .debug_bounds("download-task-window-shell")
                .expect("phase root");
            let footer = surface
                .debug_bounds("download-task-window-footer")
                .expect("footer");
            assert_eq!(
                root.size,
                size(px(440.0), px(if completed { 184.0 } else { 196.0 }))
            );
            assert_eq!(
                surface
                    .debug_bounds("download-task-window-titlebar")
                    .expect("title")
                    .size
                    .height,
                px(34.0)
            );
            assert_eq!(footer.size.height, px(44.0));
            if completed {
                let name = surface
                    .debug_bounds("download-complete-name")
                    .expect("bounded two-line name");
                assert!(name.size.height <= px(32.0 * base / 16.0) + px(0.5));
                let path = surface
                    .debug_bounds("download-complete-path")
                    .expect("path");
                assert!(path.bottom() <= footer.top());
                surface.simulate_mouse_move(path.center(), None, Modifiers::default());
                surface.run_until_parked();
                cx.executor().advance_clock(Duration::from_millis(600));
                draw(&mut surface);
                assert!(
                    surface
                        .debug_bounds("download-complete-path-tooltip")
                        .is_some(),
                    "full destination remains available"
                );
                for selector in [
                    "download-complete-secondary-drag-action",
                    "download-complete-secondary-reveal-action",
                    "download-complete-primary-open-action",
                    "download-complete-open-with-action",
                ] {
                    let hit = surface.debug_bounds(selector).expect("original action");
                    assert!(hit.top() >= footer.top() && hit.bottom() <= footer.bottom());
                }
            } else {
                let viewport = surface
                    .debug_bounds("download-window-failure-detail")
                    .expect("original bounded error");
                let before = surface
                    .debug_bounds("download-window-failure-kit-alert")
                    .expect("full Alert");
                wheel(&mut surface, viewport.center(), -8.0, 60);
                let after = surface
                    .debug_bounds("download-window-failure-kit-alert")
                    .expect("scrolled full Alert");
                assert!(before.size.height > viewport.size.height && after.top() < before.top());
                assert!(
                    after.bottom() <= viewport.bottom() + px(0.5)
                        && after.bottom() > viewport.top(),
                    "full failed error end base={base}"
                );
                assert!(viewport.bottom() <= footer.top());
            }
            let close = surface
                .debug_bounds("download-window-close")
                .expect("original caption close");
            let handle = surface.update(|window, _| window.window_handle());
            surface.simulate_click(close.center(), Modifiers::default());
            surface.run_until_parked();
            assert!(
                cx.read(|cx| !cx.windows().contains(&handle)),
                "close stays presentation-only"
            );
            assert_eq!(f.application.read_with(cx, |app, _| app.tasks().len()), 1);
            cx.update(|cx| ComponentTheme::update(cx, |theme| *theme = original));
        }
    }
}
