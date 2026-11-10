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
    fixture_with_persistence(cx, false)
}

fn fixture_with_persistence(cx: &mut TestAppContext, persistent: bool) -> Fixture {
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
    let engine = if persistent {
        DownloadEngine::open(paths.join("downloads.sqlite3")).expect("isolated SQLite engine")
    } else {
        DownloadEngine::open_in_memory().expect("memory engine, no execution")
    };
    let application = cx.update(|cx| {
        let notifications = cx.new(|_| NotificationCenter::default());
        cx.new(|cx| {
            DownloadApplication::new(
                engine,
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
fn download_layout_progress_filename_main_metrics_and_fixed_footer_do_not_require_body_scroll(
    cx: &mut TestAppContext,
) {
    let f = fixture(cx);
    let (_, view, mut surface) = task(&f, cx);
    view.update(&mut surface, |view, cx| {
        let task = view.snapshot.as_mut().expect("original task snapshot");
        task.final_path = Some(PathBuf::from(format!(
            "D:\\Synthetic\\{}\\下载文件.bin",
            "非常长的文件名称不会挤掉展开操作".repeat(12)
        )));
        view.progress = Some(DownloadProgress {
            task_id: view.task_id,
            downloaded_bytes: 2048,
            total_bytes: Some(4096),
            bytes_per_second: 256,
            active_connections: 2,
            segments: Vec::new(),
        });
        cx.notify();
    });
    draw(&mut surface);
    let body = surface
        .debug_bounds("download-task-window-body")
        .expect("fixed content viewport");
    let footer = surface
        .debug_bounds("download-task-window-footer")
        .expect("fixed footer");
    let filename = surface
        .debug_bounds("download-window-progress-name")
        .expect("single-line filename");
    let toggle = surface
        .debug_bounds("download-window-details-toggle")
        .expect("details action");
    let metric = surface
        .debug_bounds("download-window-metric-速度-value")
        .expect("body metric");
    assert!(
        filename.size.height > metric.size.height,
        "filename must be one typography tier larger: {filename:?} {metric:?}"
    );
    assert!(filename.right() <= toggle.left());
    assert!(toggle.right() <= body.right());
    assert!(
        surface
            .debug_bounds("download-window-metric-状态-value")
            .is_none(),
        "window title already owns task status"
    );
    assert!(
        surface
            .debug_bounds("download-window-progress-capabilities")
            .is_none(),
        "technical capability details belong to the existing expansion"
    );
    let selectors = [
        "download-window-progress-name",
        "download-window-server-value",
        "download-window-progress-track",
        "download-window-metric-已下载-value",
        "download-window-metric-速度-value",
        "download-window-metric-剩余-value",
    ];
    let before = selectors
        .iter()
        .map(|selector| {
            surface
                .debug_bounds(selector)
                .unwrap_or_else(|| panic!("main progress item missing: {selector}"))
        })
        .collect::<Vec<_>>();
    for bounds in &before {
        assert!(bounds.top() >= body.top() && bounds.bottom() <= footer.top());
    }
    wheel(&mut surface, body.center(), -8.0, 12);
    let after = selectors
        .iter()
        .map(|selector| surface.debug_bounds(selector).expect("stable main item"))
        .collect::<Vec<_>>();
    assert_eq!(
        before, after,
        "collapsed 440x184 main progress must not require body scrolling"
    );
    assert_eq!(
        surface
            .debug_bounds("download-task-window-shell")
            .expect("root")
            .size,
        size(px(440.0), px(184.0))
    );
    assert_eq!(footer.size.height, px(44.0));
    let cancel = surface
        .debug_bounds("download-window-cancel")
        .expect("original cancel");
    assert!(cancel.top() >= footer.top() && cancel.bottom() <= footer.bottom());
    surface.simulate_mouse_move(filename.center(), None, Modifiers::default());
    cx.executor().advance_clock(Duration::from_millis(700));
    draw(&mut surface);
    assert!(
        surface
            .debug_bounds("download-window-progress-name-tooltip")
            .is_some(),
        "full long filename remains available"
    );
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
fn download_layout_category_select_heading_help_and_add_share_row_with_narrow_wrap(
    cx: &mut TestAppContext,
) {
    for dimensions in [(984.0, 650.0), (720.0, 480.0)] {
        let f = fixture(cx);
        let (_, mut surface) = settings(&f, cx);
        surface.simulate_resize(size(px(dimensions.0), px(dimensions.1)));
        draw(&mut surface);
        let header = reveal(&mut surface, "download-settings-category-heading");
        let snapshots = surface.update(|window, _| gpui_kit::base::test_support::snapshots(window));
        let select = snapshots
            .iter()
            .find(|node| {
                node.role() == Some(gpui_kit::Role::ComboBox) && node.label() == Some("选择分类")
            })
            .expect("released Select keeps the existing selected-category owner")
            .bounds();
        let heading = surface
            .debug_bounds("download-settings-category-heading-title")
            .expect("category rules heading");
        let help = surface
            .debug_bounds("download-settings-category-heading-help")
            .expect("category rules help");
        let add = surface
            .debug_bounds("download-settings-category-add-button")
            .expect("official add action");
        let form = surface
            .debug_bounds("download-settings-category-form")
            .expect("actual rules below header");
        let content = surface
            .debug_bounds("download-settings-shell-content")
            .expect("settings viewport");
        let footer = surface
            .debug_bounds("download-settings-shell-footer")
            .expect("fixed footer");
        for bounds in [select, heading, help, add] {
            assert!(bounds.left() >= header.left() && bounds.right() <= header.right());
            assert!(bounds.top() >= header.top() && bounds.bottom() <= header.bottom());
        }
        assert!(header.bottom() <= form.top());
        assert!(add.bottom() <= footer.top() && footer.bottom() <= px(dimensions.1));
        assert!(content.bottom() <= footer.top());
        if dimensions.0 >= 900.0 {
            for bounds in [heading, help, add] {
                assert!(
                    (bounds.center().y - select.center().y).abs() <= px(1.0),
                    "ordinary desktop header must stay on one row: {select:?} {bounds:?}"
                );
            }
        }
    }
}

#[gpui_kit::test]
fn download_layout_category_select_keeps_existing_draft_and_durable_apply_owner(
    cx: &mut TestAppContext,
) {
    let f = fixture(cx);
    let (view, mut surface) = settings(&f, cx);
    let path = f.directory.path().join("config.json");
    let before = std::fs::read(&path).expect("settings bytes before local selection");
    let _ = reveal(&mut surface, "download-settings-category-select");
    let select = surface
        .update(|window, _| gpui_kit::base::test_support::snapshots(window))
        .into_iter()
        .find(|node| {
            node.role() == Some(gpui_kit::Role::ComboBox) && node.label() == Some("选择分类")
        })
        .expect("actual category Select trigger")
        .bounds();
    surface.simulate_click(select.center(), Modifiers::default());
    draw(&mut surface);
    cx.executor().advance_clock(Duration::from_millis(180));
    draw(&mut surface);
    let select_nodes = surface.update(|window, _| gpui_kit::base::test_support::snapshots(window));
    assert!(
        select_nodes.iter().any(|node| {
            node.role() == Some(gpui_kit::Role::ComboBox)
                && node.label() == Some("选择分类")
                && node.expanded() == Some(true)
        }),
        "the real category trigger must expand the released Select"
    );
    let second_label = view.read_with(&surface, |view, cx| {
        view.categories[1].name.read(cx).value().to_string()
    });
    let option = surface
        .update(|window, _| gpui_kit::base::test_support::snapshots(window))
        .into_iter()
        .find(|node| node.label() == Some(second_label.as_str()))
        .expect("second category option")
        .bounds();
    surface.simulate_click(option.center(), Modifiers::default());
    draw(&mut surface);
    assert_eq!(
        view.read_with(&surface, |view, _| view.selected_category),
        Some(1)
    );
    assert_eq!(
        std::fs::read(&path).expect("selection must remain draft-only"),
        before
    );
    let name = reveal(&mut surface, "download-settings-category-name-field");
    surface.simulate_click(name.center(), Modifiers::default());
    surface.simulate_keystrokes("ctrl-a");
    surface.simulate_input("同一草稿保存分类");
    draw(&mut surface);
    let apply = surface
        .debug_bounds("download-settings-apply")
        .expect("original Apply");
    surface.simulate_click(apply.center(), Modifiers::default());
    surface.run_until_parked();
    let saved: AppConfig = serde_json::from_slice(
        &std::fs::read(&path).expect("original durable section saver output"),
    )
    .expect("saved settings");
    assert_eq!(saved.download.categories[1].name, "同一草稿保存分类");
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
                    view.sync_category_select(window, cx);
                    cx.notify();
                })
            });
            let _ = reveal(&mut surface, "download-settings-category-heading");
            let selector = surface
                .update(|window, _| gpui_kit::base::test_support::snapshots(window))
                .into_iter()
                .find(|node| {
                    node.role() == Some(gpui_kit::Role::ComboBox)
                        && node.label() == Some("选择分类")
                })
                .expect("official category Select")
                .bounds();
            let content = surface
                .debug_bounds("download-settings-shell-content")
                .expect("content");
            assert!(selector.top() >= content.top() && selector.bottom() <= content.bottom());
            let heading = surface
                .debug_bounds("download-settings-category-heading")
                .expect("editor");
            assert!(heading.size.height >= px(32.0 * base / 16.0) - px(0.5));
            assert!(
                surface
                    .debug_bounds("download-settings-category-master-scroll")
                    .is_none()
            );
            assert!(
                surface
                    .debug_bounds("download-settings-category-form-scroll")
                    .is_none()
            );
            surface.simulate_click(selector.center(), Modifiers::default());
            draw(&mut surface);
            cx.executor().advance_clock(Duration::from_millis(180));
            draw(&mut surface);
            let last_name = view.read_with(&surface, |v, cx| {
                v.categories[23].name.read(cx).value().to_string()
            });
            let popup = surface
                .update(|window, _| gpui_kit::base::test_support::snapshots(window))
                .into_iter()
                .find(|node| {
                    node.role() == Some(gpui_kit::Role::TextInput)
                        && node.label() == Some("搜索分类")
                })
                .expect("official Select search field")
                .bounds();
            let menu_bottom = (popup.bottom() + px(220.0 * base / 16.0)).min(px(dimensions.1));
            let mut clicked = false;
            for _ in 0..32 {
                let snapshots =
                    surface.update(|window, _| gpui_kit::base::test_support::snapshots(window));
                let options = snapshots
                    .iter()
                    .filter(|node| {
                        node.role() == Some(gpui_kit::Role::Label)
                            && node.visible()
                            && node
                                .label()
                                .is_some_and(|label| label.starts_with("合成分类"))
                            && node.bounds().top() >= popup.bottom()
                            && node.bounds().bottom() <= menu_bottom
                    })
                    .collect::<Vec<_>>();
                assert!(options.len() < 24, "the official list must remain bounded");
                if let Some(last) = options
                    .iter()
                    .find(|node| node.label() == Some(last_name.as_str()))
                {
                    surface.simulate_click(last.bounds().center(), Modifiers::default());
                    clicked = true;
                    break;
                }
                let scroll_anchor = options
                    .iter()
                    .min_by(|left, right| {
                        left.bounds()
                            .top()
                            .partial_cmp(&right.bounds().top())
                            .unwrap()
                    })
                    .expect("a visible official option for pointer scrolling")
                    .bounds()
                    .center();
                wheel(&mut surface, scroll_anchor, -4.0, 1);
            }
            assert!(
                clicked,
                "last option must scroll into the official popup: {popup:?}, {dimensions:?}, rem={base}"
            );
            draw(&mut surface);
            assert_eq!(
                view.read_with(&surface, |v, _| v.selected_category),
                Some(23),
                "last category from official scrollable menu: {popup:?}"
            );
            let form = surface
                .debug_bounds("download-settings-category-form")
                .expect("original form");
            assert!(form.size.height > px(0.0));
            let url = reveal(&mut surface, "download-settings-category-url-field");
            assert!(
                url.bottom()
                    <= surface
                        .debug_bounds("download-settings-shell-footer")
                        .expect("footer")
                        .top()
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
        draw(&mut surface);
        let toggle = surface
            .debug_bounds("download-window-details-toggle")
            .expect("original disclosure");
        surface.simulate_click(toggle.center(), Modifiers::default());
        draw(&mut surface);
        assert!(view.read_with(&surface, |v, _| v.details_expanded));
        assert!(
            surface
                .debug_bounds("download-window-progress-capabilities")
                .is_some(),
            "resume, limit, connection, and segment summary move into expansion"
        );
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

#[gpui_kit::test]
fn ref06_global_speed_typing_is_draft_and_enter_submits(cx: &mut TestAppContext) {
    let f = fixture(cx);
    let (view, mut surface) = settings(&f, cx);
    select_page(&mut surface, DownloadSettingsPage::Transfer);
    view.update_in(&mut surface, |view, window, cx| {
        view.speed_limit_kib
            .update(cx, |input, cx| input.focus(window, cx));
    });
    surface.simulate_keystrokes("ctrl-a");
    for digit in ["1", "0", "0"] {
        surface.simulate_input(digit);
        draw(&mut surface);
        assert_eq!(
            f.application
                .read_with(cx, |app, _| app.engine.global_speed_limit()),
            None,
            "partial numeric input must not apply a rate"
        );
    }
    surface.simulate_keystrokes("enter");
    draw(&mut surface);
    assert_eq!(
        f.application
            .read_with(cx, |app, _| app.engine.global_speed_limit()),
        Some(100 * 1024),
        "complete Enter submits the original global speed command"
    );
}

#[gpui_kit::test]
fn ref06_task_details_exposes_original_limit_editor(cx: &mut TestAppContext) {
    let f = fixture(cx);
    let (_, view, mut surface) = task(&f, cx);
    view.update(&mut surface, |view, cx| {
        view.details_expanded = true;
        cx.notify();
    });
    surface.simulate_resize(download_task_window_size_for_details(
        DownloadWindowPhase::Progress,
        true,
    ));
    draw(&mut surface);
    assert!(
        surface
            .debug_bounds("download-task-speed-limit-input")
            .is_some(),
        "expanded original task details expose a deliberate numeric speed editor"
    );
}

fn global_rate(f: &Fixture, cx: &TestAppContext) -> Option<u64> {
    f.application
        .read_with(cx, |app, _| app.engine.global_speed_limit())
}

fn edit_global(view: &Entity<DownloadSettingsView>, surface: &mut VisualTestContext, value: &str) {
    view.update_in(surface, |view, window, cx| {
        view.speed_limit_kib
            .update(cx, |input, cx| input.focus(window, cx));
    });
    surface.simulate_keystrokes("ctrl-a");
    surface.simulate_input(value);
    draw(surface);
}

#[gpui_kit::test]
fn ref06_global_enter_blur_escape_invalid_and_disable(cx: &mut TestAppContext) {
    let f = fixture(cx);
    let (view, mut surface) = settings(&f, cx);
    select_page(&mut surface, DownloadSettingsPage::Transfer);
    edit_global(&view, &mut surface, "100");
    surface.simulate_keystrokes("enter");
    surface.simulate_keystrokes("tab");
    draw(&mut surface);
    assert_eq!(global_rate(&f, cx), Some(100 * 1024));
    assert_eq!(
        view.read_with(cx, |view, _| view.speed_limit_edit.submissions),
        1
    );
    edit_global(&view, &mut surface, "999");
    surface.simulate_keystrokes("escape");
    draw(&mut surface);
    assert_eq!(
        view.read_with(cx, |view, cx| view
            .speed_limit_kib
            .read(cx)
            .value()
            .to_string()),
        "100"
    );
    assert_eq!(global_rate(&f, cx), Some(100 * 1024));
    for invalid in ["", "abc", "-1", "1.5", "18446744073709551615"] {
        edit_global(&view, &mut surface, invalid);
        surface.simulate_keystrokes("enter");
        draw(&mut surface);
        assert_eq!(global_rate(&f, cx), Some(100 * 1024));
        assert!(view.read_with(cx, |view, _| view.speed_limit_edit.error.is_some()));
        assert!(
            surface
                .debug_bounds("download-settings-speed-limit-error")
                .is_some()
        );
    }
    edit_global(&view, &mut surface, "64");
    surface.simulate_keystrokes("tab");
    draw(&mut surface);
    assert_eq!(
        global_rate(&f, cx),
        Some(64 * 1024),
        "focus loss commits once"
    );
    edit_global(&view, &mut surface, "0");
    surface.simulate_keystrokes("enter");
    draw(&mut surface);
    assert_eq!(
        global_rate(&f, cx),
        None,
        "zero uses original unlimited representation"
    );
}

#[gpui_kit::test]
fn ref06_global_storage_failure_retry_and_reopen(cx: &mut TestAppContext) {
    let f = fixture_with_persistence(cx, true);
    f.application.read_with(cx, |app, _| {
        app.engine
            .set_global_speed_limit(Some(64 * 1024))
            .expect("initial intent")
    });
    let database = f.directory.path().join("downloads.sqlite3");
    let connection = rusqlite::Connection::open(&database).expect("storage observer");
    connection.execute_batch("CREATE TRIGGER reject_speed BEFORE UPDATE ON download_settings BEGIN SELECT RAISE(ABORT, 'synthetic'); END;").expect("failure injector");
    let (view, mut surface) = settings(&f, cx);
    select_page(&mut surface, DownloadSettingsPage::Transfer);
    edit_global(&view, &mut surface, "100");
    surface.simulate_keystrokes("enter");
    draw(&mut surface);
    assert_eq!(global_rate(&f, cx), Some(64 * 1024));
    assert_eq!(
        view.read_with(cx, |view, _| view.speed_limit_edit.applied),
        Some(64 * 1024)
    );
    assert!(view.read_with(cx, |view, _| view.speed_limit_edit.error.is_some()));
    assert_eq!(
        DownloadEngine::open(&database)
            .expect("reopen failed save")
            .global_speed_limit(),
        Some(64 * 1024)
    );
    connection
        .execute_batch("DROP TRIGGER reject_speed")
        .expect("allow retry");
    surface.simulate_keystrokes("enter");
    draw(&mut surface);
    assert_eq!(global_rate(&f, cx), Some(100 * 1024));
    assert_eq!(
        DownloadEngine::open(&database)
            .expect("reopen saved intent")
            .global_speed_limit(),
        Some(100 * 1024)
    );
    tab_button(&mut surface, "download-settings-cancel-button");
    key(&mut surface, "space");
    cx.run_until_parked();
    let (reopened, _) = settings(&f, cx);
    assert_eq!(
        reopened.read_with(cx, |view, cx| view
            .speed_limit_kib
            .read(cx)
            .value()
            .to_string()),
        "100"
    );
}

#[gpui_kit::test]
fn ref06_task_keyboard_commits_once_preserves_ceilings_and_reopens(cx: &mut TestAppContext) {
    let f = fixture_with_persistence(cx, true);
    f.application.read_with(cx, |app, _| {
        app.engine
            .set_global_speed_limit(Some(64 * 1024))
            .expect("global ceiling");
        app.engine
            .set_runtime_speed_cap(Some(32 * 1024), uuid::Uuid::new_v4())
            .expect("runtime ceiling");
    });
    let (task_id, view, mut surface) = task(&f, cx);
    let toggle = surface
        .debug_bounds("download-window-details-toggle")
        .expect("original details action");
    surface.simulate_click(toggle.center(), Modifiers::default());
    surface.simulate_resize(download_task_window_size_for_details(
        DownloadWindowPhase::Progress,
        true,
    ));
    draw(&mut surface);
    let bounds = surface
        .debug_bounds("download-task-speed-limit-input")
        .expect("official input bounds");
    let body = surface
        .debug_bounds("download-window-details")
        .expect("original details bounds");
    assert!(
        bounds.origin.x >= body.origin.x
            && bounds.right() <= body.right()
            && bounds.bottom() <= body.bottom(),
        "input={bounds:?}, details={body:?}"
    );
    let input = view.read_with(cx, |view, _| {
        view.speed_limit_kib.clone().expect("stable InputState")
    });
    input.update_in(&mut surface, |input, window, cx| input.focus(window, cx));
    surface.simulate_keystrokes("ctrl-a");
    for digit in ["1", "0", "0"] {
        surface.simulate_input(digit);
        draw(&mut surface);
        assert_eq!(
            f.application.read_with(cx, |app, _| app
                .engine
                .task(task_id)
                .expect("query")
                .expect("task")
                .task
                .request()
                .speed_limit_bytes_per_second),
            None
        );
    }
    let events = f.application.read_with(cx, |app, _| {
        app.engine.subscribe_events().expect("bounded observer")
    });
    surface.simulate_keystrokes("enter");
    surface.simulate_keystrokes("tab");
    draw(&mut surface);
    let changed = std::iter::from_fn(|| events.try_recv().ok()).filter(|event| matches!(&event.event, DownloadEvent::TaskChanged { task } if task.id() == task_id)).count();
    assert_eq!(
        changed, 1,
        "one durable TaskChanged for Enter followed by blur"
    );
    assert_eq!(
        view.read_with(cx, |view, _| view.speed_limit_edit.submissions),
        1
    );
    assert_eq!(
        f.application
            .read_with(cx, |app, _| app.engine.effective_global_speed_limit()),
        Some(32 * 1024)
    );
    let reopened =
        DownloadEngine::open(f.directory.path().join("downloads.sqlite3")).expect("reopen task");
    assert_eq!(
        reopened
            .task(task_id)
            .expect("query")
            .expect("task")
            .task
            .request()
            .speed_limit_bytes_per_second,
        Some(100 * 1024)
    );
    input.update_in(&mut surface, |input, window, cx| input.focus(window, cx));
    surface.simulate_keystrokes("ctrl-a");
    surface.simulate_input("999");
    surface.simulate_keystrokes("escape");
    draw(&mut surface);
    assert_eq!(
        input.read_with(cx, |input, _| input.value().to_string()),
        "100"
    );
    assert_eq!(
        view.read_with(cx, |view, _| view.speed_limit_edit.submissions),
        1
    );
    f.application
        .update(cx, |app, cx| app.close_task_window(task_id, cx));
    cx.run_until_parked();
    f.application
        .update(cx, |app, cx| app.request_task_window(task_id, cx));
    cx.run_until_parked();
    let restored = f.application.read_with(cx, |app, _| {
        app.task_windows
            .get(&task_id)
            .expect("reopened window")
            .view
            .clone()
    });
    assert_eq!(
        restored.read_with(cx, |view, _| view.speed_limit_edit.applied),
        Some(100 * 1024)
    );
}

#[gpui_kit::test]
fn ref06_task_invalid_storage_failure_retry_and_disable(cx: &mut TestAppContext) {
    let f = fixture_with_persistence(cx, true);
    let (task_id, view, mut surface) = task(&f, cx);
    let toggle = surface
        .debug_bounds("download-window-details-toggle")
        .expect("details action");
    surface.simulate_click(toggle.center(), Modifiers::default());
    surface.simulate_resize(download_task_window_size_for_details(
        DownloadWindowPhase::Progress,
        true,
    ));
    draw(&mut surface);
    let input = view.read_with(cx, |view, _| {
        view.speed_limit_kib.clone().expect("original input")
    });
    let edit = |surface: &mut VisualTestContext, value: &str| {
        input.update_in(surface, |input, window, cx| input.focus(window, cx));
        surface.simulate_keystrokes("ctrl-a");
        surface.simulate_input(value);
        surface.simulate_keystrokes("enter");
        draw(surface);
    };
    edit(&mut surface, "100");
    assert_eq!(
        view.read_with(cx, |view, _| view.speed_limit_edit.applied),
        Some(100 * 1024)
    );
    for invalid in ["", "abc", "-1", "1.5", "18446744073709551615"] {
        edit(&mut surface, invalid);
        assert_eq!(
            view.read_with(cx, |view, _| view.speed_limit_edit.applied),
            Some(100 * 1024)
        );
        assert!(view.read_with(cx, |view, _| view.speed_limit_edit.error.is_some()));
        assert!(
            surface
                .debug_bounds("download-task-speed-limit-error")
                .is_some()
        );
    }
    let database = f.directory.path().join("downloads.sqlite3");
    let connection = rusqlite::Connection::open(&database).expect("failure injector");
    connection.execute_batch("CREATE TRIGGER reject_task_speed BEFORE UPDATE ON download_tasks BEGIN SELECT RAISE(ABORT, 'synthetic'); END;").expect("reject save");
    edit(&mut surface, "64");
    assert_eq!(
        view.read_with(cx, |view, _| view.speed_limit_edit.applied),
        Some(100 * 1024)
    );
    assert!(view.read_with(cx, |view, _| view.speed_limit_edit.error.is_some()));
    assert_eq!(
        f.application.read_with(cx, |app, _| app
            .engine
            .task(task_id)
            .expect("query")
            .expect("task")
            .task
            .request()
            .speed_limit_bytes_per_second),
        Some(100 * 1024)
    );
    connection
        .execute_batch("DROP TRIGGER reject_task_speed")
        .expect("allow retry");
    edit(&mut surface, "64");
    assert_eq!(
        view.read_with(cx, |view, _| view.speed_limit_edit.applied),
        Some(64 * 1024)
    );
    let reopened = DownloadEngine::open(&database).expect("reopen intent");
    assert_eq!(
        reopened
            .task(task_id)
            .expect("query")
            .expect("task")
            .task
            .request()
            .speed_limit_bytes_per_second,
        Some(64 * 1024)
    );
    edit(&mut surface, "0");
    assert_eq!(
        view.read_with(cx, |view, _| view.speed_limit_edit.applied),
        None
    );
    assert_eq!(
        input.entity_id(),
        view.read_with(cx, |view, _| view
            .speed_limit_kib
            .as_ref()
            .expect("same input")
            .entity_id())
    );
}

#[gpui_kit::test]
fn ref06_closed_task_window_retires_deferred_limit_submission(cx: &mut TestAppContext) {
    let f = fixture(cx);
    let (task_id, view, mut surface) = task(&f, cx);
    let toggle = surface
        .debug_bounds("download-window-details-toggle")
        .expect("original details action");
    surface.simulate_click(toggle.center(), Modifiers::default());
    surface.simulate_resize(download_task_window_size_for_details(
        DownloadWindowPhase::Progress,
        true,
    ));
    draw(&mut surface);
    let input = view.read_with(cx, |view, _| {
        view.speed_limit_kib.clone().expect("original input")
    });
    let application = f.application.clone();
    let _close_subscription = view.update_in(&mut surface, |_, window, cx| {
        cx.subscribe_in(&input, window, move |_, _, event: &InputEvent, _, cx| {
            if matches!(event, InputEvent::PressEnter { .. }) {
                let application = application.clone();
                cx.defer(move |cx| {
                    application.update(cx, |app, cx| app.close_task_window(task_id, cx))
                });
            }
        })
    });
    input.update_in(&mut surface, |input, window, cx| input.focus(window, cx));
    surface.simulate_keystrokes("ctrl-a");
    surface.simulate_input("100");
    surface.simulate_keystrokes("enter");
    surface.run_until_parked();
    assert_eq!(
        f.application.read_with(cx, |app, _| app
            .engine
            .task(task_id)
            .expect("query")
            .expect("task")
            .task
            .request()
            .speed_limit_bytes_per_second),
        None
    );
    assert!(
        f.application
            .read_with(cx, |app, _| !app.task_windows.contains_key(&task_id))
    );
}

fn assert_ref06_untouched_byte_rate(cx: &mut TestAppContext, bytes: u64) {
    let f = fixture_with_persistence(cx, true);
    f.application.read_with(cx, |app, _| {
        app.engine
            .set_global_speed_limit(Some(bytes))
            .expect("positive byte rate");
    });
    let (view, mut surface) = settings(&f, cx);
    select_page(&mut surface, DownloadSettingsPage::Transfer);
    let displayed = view.read_with(cx, |view, cx| {
        view.speed_limit_kib.read(cx).value().to_string()
    });
    view.update_in(&mut surface, |view, window, cx| {
        view.connection_limit
            .update(cx, |input, cx| input.focus(window, cx));
    });
    surface.simulate_keystrokes("ctrl-a");
    surface.simulate_input("5");
    draw(&mut surface);
    let apply = surface
        .debug_bounds("download-settings-apply")
        .expect("original Apply");
    surface.simulate_click(apply.center(), Modifiers::default());
    surface.run_until_parked();
    assert_eq!(
        global_rate(&f, cx),
        Some(bytes),
        "unrelated Apply must preserve exact {bytes} B/s; initial display={displayed}"
    );
    let restored = DownloadEngine::open(f.directory.path().join("downloads.sqlite3"))
        .expect("reopen settings storage");
    assert_eq!(restored.global_speed_limit(), Some(bytes));
    assert_eq!(
        displayed,
        bytes.div_ceil(1024).to_string(),
        "positive rates never display unlimited"
    );
    let (reopened, _) = settings(&f, cx);
    assert_eq!(
        reopened.read_with(cx, |view, _| view.speed_limit_edit.applied),
        Some(bytes)
    );
    assert_eq!(
        reopened.read_with(cx, |view, cx| view
            .speed_limit_kib
            .read(cx)
            .value()
            .to_string()),
        bytes.div_ceil(1024).to_string()
    );
}

#[gpui_kit::test]
fn ref06_acceptance_global_one_byte_survives_unrelated_apply(cx: &mut TestAppContext) {
    assert_ref06_untouched_byte_rate(cx, 1);
}

#[gpui_kit::test]
fn ref06_acceptance_global_sub_kib_survives_unrelated_apply(cx: &mut TestAppContext) {
    assert_ref06_untouched_byte_rate(cx, 1023);
}

#[gpui_kit::test]
fn ref06_acceptance_global_fractional_kib_survives_unrelated_apply(cx: &mut TestAppContext) {
    assert_ref06_untouched_byte_rate(cx, 1537);
}

#[gpui_kit::test]
fn ref06_acceptance_task_compact_refresh_tracks_authoritative_rate(cx: &mut TestAppContext) {
    for global in [None, Some(128 * 1024)] {
        let f = fixture_with_persistence(cx, true);
        f.application.read_with(cx, |app, _| {
            app.engine
                .set_global_speed_limit(global)
                .expect("global ceiling")
        });
        let (task_id, view, mut surface) = task(&f, cx);
        view.update(&mut surface, |view, cx| {
            view.details_expanded = true;
            cx.notify();
        });
        surface.simulate_resize(download_task_window_size_for_details(
            DownloadWindowPhase::Progress,
            true,
        ));
        draw(&mut surface);
        let input = view.read_with(cx, |view, _| {
            view.speed_limit_kib.clone().expect("stable editor")
        });
        for limit in [Some(64 * 1024), Some(32 * 1024), None, Some(1537)] {
            f.application.update(cx, |app, cx| {
                let result = app.engine.handle_command(DownloadCommandEnvelope {
                    protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
                    request_id: uuid::Uuid::new_v4(),
                    origin: RequestOrigin::Internal,
                    command: DownloadCommand::Task { task_id, action: DownloadTaskAction::SetSpeedLimit { bytes_per_second: limit } },
                });
                assert!(matches!(result.outcome, DownloadCommandOutcome::Accepted { task_id: Some(actual) } if actual == task_id));
                app.refresh(cx);
                assert_eq!(app.task(task_id).expect("compact task").task.request().speed_limit_bytes_per_second, None, "list projection remains compact");
                app.refresh_open_task_window_for_progress(task_id, cx);
            });
            draw(&mut surface);
            let effective = match (limit, global) {
                (Some(task), Some(global)) => Some(task.min(global)),
                (task, global) => task.or(global),
            };
            let expected = effective
                .map(|bytes| format!("限速 {}/s", format_download_size(bytes)))
                .unwrap_or_else(|| "不限速".into());
            let label = surface
                .update(|window, _| gpui_kit::base::test_support::snapshots(window))
                .into_iter()
                .find(|node| {
                    node.path().last()
                        == Some(&gpui_kit::ElementId::from(
                            "download-window-progress-capabilities",
                        ))
                })
                .expect("actual capabilities node")
                .label()
                .expect("visible summary")
                .to_owned();
            assert!(
                label.contains(&expected),
                "summary={label:?}, expected={expected:?}"
            );
            assert_eq!(
                view.read_with(cx, |view, _| view.speed_limit_edit.applied),
                limit
            );
            assert!(view.read_with(cx, |view, _| !view.speed_limit_edit.dirty
                && view.speed_limit_edit.pending.is_none()
                && view.speed_limit_edit.error.is_none()));
            assert_eq!(
                input.read_with(cx, |input, _| input.value().to_string()),
                limit.map_or(0, |bytes| bytes.div_ceil(1024)).to_string()
            );
            assert_eq!(
                view.read_with(cx, |view, _| view
                    .speed_limit_kib
                    .as_ref()
                    .expect("same editor")
                    .entity_id()),
                input.entity_id()
            );
            assert_eq!(
                f.application.read_with(cx, |app, _| app
                    .engine
                    .task(task_id)
                    .expect("query")
                    .expect("task")
                    .task
                    .request()
                    .speed_limit_bytes_per_second),
                limit
            );
        }
        input.update_in(&mut surface, |input, window, cx| input.focus(window, cx));
        surface.simulate_keystrokes("ctrl-a");
        surface.simulate_input("abc");
        surface.simulate_keystrokes("enter");
        draw(&mut surface);
        assert!(view.read_with(cx, |view, _| view.speed_limit_edit.error.is_some()));
        f.application.update(cx, |app, cx| {
            let result = app.engine.handle_command(DownloadCommandEnvelope {
                protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
                request_id: uuid::Uuid::new_v4(),
                origin: RequestOrigin::Internal,
                command: DownloadCommand::Task {
                    task_id,
                    action: DownloadTaskAction::SetSpeedLimit {
                        bytes_per_second: Some(2049),
                    },
                },
            });
            assert!(matches!(
                result.outcome,
                DownloadCommandOutcome::Accepted { .. }
            ));
            app.refresh(cx);
            app.refresh_open_task_window_for_progress(task_id, cx);
        });
        draw(&mut surface);
        assert_eq!(
            input.read_with(cx, |input, _| input.value().to_string()),
            "abc"
        );
        assert!(view.read_with(cx, |view, _| view.speed_limit_edit.dirty
            && view.speed_limit_edit.error.is_some()));
        assert_eq!(
            view.read_with(cx, |view, _| view.speed_limit_edit.applied),
            Some(2049)
        );
        surface.simulate_keystrokes("escape");
        draw(&mut surface);
        assert_eq!(
            input.read_with(cx, |input, _| input.value().to_string()),
            "3"
        );
        assert!(view.read_with(cx, |view, _| !view.speed_limit_edit.dirty
            && view.speed_limit_edit.error.is_none()));
        f.application
            .update(cx, |app, cx| app.close_task_window(task_id, cx));
        cx.run_until_parked();
        f.application
            .update(cx, |app, cx| app.request_task_window(task_id, cx));
        cx.run_until_parked();
        let restored = f.application.read_with(cx, |app, _| {
            app.task_windows
                .get(&task_id)
                .expect("reopened window")
                .view
                .clone()
        });
        assert_eq!(
            restored.read_with(cx, |view, _| view.speed_limit_edit.applied),
            Some(2049)
        );
        assert_eq!(
            DownloadEngine::open(f.directory.path().join("downloads.sqlite3"))
                .expect("reopen engine")
                .task(task_id)
                .expect("query")
                .expect("task")
                .task
                .request()
                .speed_limit_bytes_per_second,
            Some(2049)
        );
    }
}
