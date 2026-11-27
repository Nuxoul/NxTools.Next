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
            crate::downloads::ordinary_presentation_fixture::confirmed_application(
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
        source_page: None,
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
            finishing: None,
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

fn key(surface: &mut VisualTestContext, key: &'static str) {
    let handle = surface.update(|window, _| window.window_handle());
    surface.simulate_event(gpui_kit::KeyDownEvent {
        keystroke: gpui_kit::Keystroke::parse(key).expect("test key"),
        is_held: false,
        prefer_character_input: false,
    });
    if surface.read(|cx| cx.windows().contains(&handle)) {
        surface.simulate_event(gpui_kit::KeyUpEvent {
            keystroke: gpui_kit::Keystroke::parse(key).expect("test release"),
        });
    }
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
        surface.simulate_event(gpui_kit::ScrollEvent {
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
                            .expect("finite official option bounds")
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
                        start_byte: None,
                        segment_index,
                        downloaded_bytes: 32,
                        total_bytes: Some(1024),
                    })
                    .collect(),
                finishing: None,
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
            download_task_window_size_for_details(DownloadWindowPhase::Progress, true)
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
                let status = surface
                    .debug_bounds("download-window-failure-detail")
                    .expect("failure status");
                assert!(
                    surface
                        .debug_bounds("download-window-failure-kit-alert")
                        .is_none(),
                    "the original error is not painted in the compact task window"
                );
                assert!(status.bottom() <= footer.top());
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
        view.details_tab = 1;
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
        .debug_bounds("download-window-options")
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
        .debug_bounds("download-window-options")
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
        .debug_bounds("download-window-options")
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
        let options = surface
            .debug_bounds("download-window-options")
            .expect("speed page entry");
        surface.simulate_click(options.center(), Modifiers::default());
        draw(&mut surface);
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

const B4_DEFAULT_HEADERS: &str =
    "User-Agent: Mozilla/5.0 (Windows NT 6.1; Trident/7.0; rv:11.0) like Gecko";

#[gpui_kit::test]
fn b4_headers_reset_edits_original_textarea_and_cancel_discards(cx: &mut TestAppContext) {
    use gpui_kit::Focusable as _;
    let f = fixture(cx);
    let path = f.directory.path().join("config.json");
    f.application.update(cx, |app, _| {
        app.config.download.default_request_headers = "X-Saved: original".into();
        app.config.download.default_cookie = "saved-cookie=1".into();
        app.config
            .save_to(&path)
            .expect("seed isolated custom headers");
    });
    let before = std::fs::read(&path).expect("original config bytes");
    let (view, mut surface) = settings(&f, cx);
    select_page(&mut surface, DownloadSettingsPage::Request);
    let input = view.read_with(&surface, |view, _| view.default_request_headers.clone());
    assert_eq!(
        input.read_with(&surface, |input, _| input.value().to_string()),
        "X-Saved: original"
    );
    let hit = reveal(&mut surface, "download-settings-default-headers-field");
    surface.simulate_click(hit.center(), Modifiers::default());
    surface.simulate_keystrokes("ctrl-a");
    surface.simulate_input("User-Agent: custom-draft\nX-Draft: edited");
    draw(&mut surface);
    assert_eq!(
        input.read_with(&surface, |input, _| input.value().to_string()),
        "User-Agent: custom-draft\nX-Draft: edited"
    );
    let reset = surface.update(|window, _| {
        gpui_kit::base::test_support::snapshots(window)
            .into_iter()
            .find(|node| {
                node.role() == Some(gpui_kit::Role::Button) && node.label() == Some("重置")
            })
            .expect("official adjacent Reset button must be present")
            .bounds()
    });
    surface.simulate_click(reset.center(), Modifiers::default());
    draw(&mut surface);
    assert_eq!(
        input.read_with(&surface, |input, _| input.value().to_string()),
        B4_DEFAULT_HEADERS
    );
    assert_eq!(
        input.entity_id(),
        view.read_with(&surface, |view, _| view.default_request_headers.entity_id())
    );
    assert!(surface.update(|window, cx| input.read(cx).focus_handle(cx).is_focused(window)));
    assert_eq!(
        view.read_with(&surface, |view, cx| view
            .default_cookie
            .read(cx)
            .value()
            .to_string()),
        "saved-cookie=1"
    );
    assert_eq!(std::fs::read(&path).expect("Reset is draft-only"), before);
    let cancel = surface
        .debug_bounds("download-settings-cancel")
        .expect("original Cancel");
    surface.simulate_click(cancel.center(), Modifiers::default());
    surface.run_until_parked();
    let (reopened, mut second) = settings(&f, cx);
    select_page(&mut second, DownloadSettingsPage::Request);
    assert_eq!(
        reopened.read_with(&second, |view, cx| view
            .default_request_headers
            .read(cx)
            .value()
            .to_string()),
        "X-Saved: original"
    );
    assert_eq!(
        std::fs::read(&path).expect("Cancel/reopen never saves"),
        before
    );
}

fn b4_edit_headers(
    view: &Entity<DownloadSettingsView>,
    surface: &mut VisualTestContext,
    text: &str,
) {
    use gpui_kit::Focusable as _;
    surface.update(|window, cx| {
        view.read(cx)
            .default_request_headers
            .read(cx)
            .focus_handle(cx)
            .focus(window, cx);
    });
    surface.simulate_keystrokes("ctrl-a");
    if text.is_empty() {
        surface.simulate_keystrokes("backspace");
    } else {
        surface.simulate_input(text);
    }
    draw(surface);
    assert_eq!(
        view.read_with(surface, |view, cx| view
            .default_request_headers
            .read(cx)
            .value()
            .to_string()),
        text
    );
}

fn b4_focus_reset(view: &Entity<DownloadSettingsView>, surface: &mut VisualTestContext) {
    use gpui_kit::Focusable as _;
    surface.update(|window, cx| {
        view.read(cx)
            .default_cookie
            .read(cx)
            .focus_handle(cx)
            .focus(window, cx);
    });
    for _ in 0..20 {
        key(surface, "shift-tab");
        if surface.update(|window, _| {
            gpui_kit::base::test_support::snapshots(window)
                .iter()
                .any(|node| {
                    node.role() == Some(gpui_kit::Role::Button)
                        && node.label() == Some("重置")
                        && node.focused() == Some(true)
                })
        }) {
            return;
        }
    }
    panic!("actual reverse Tab must reach Reset from the adjacent Cookie field");
}

#[gpui_kit::test]
fn b4_headers_reset_first_enter_space_repeat_and_escape_keep_draft_ownership(
    cx: &mut TestAppContext,
) {
    use gpui_kit::Focusable as _;
    let f = fixture(cx);
    let path = f.directory.path().join("config.json");
    let before = std::fs::read(&path).expect("original bytes");
    let (view, mut surface) = settings(&f, cx);
    select_page(&mut surface, DownloadSettingsPage::Request);
    let input = view.read_with(&surface, |view, _| view.default_request_headers.clone());
    for activation in ["enter", "space"] {
        b4_edit_headers(&view, &mut surface, "X-Draft: keyboard");
        b4_focus_reset(&view, &mut surface);
        assert!(surface.update(|window, _| {
            gpui_kit::base::test_support::snapshots(window)
                .iter()
                .any(|node| {
                    node.role() == Some(gpui_kit::Role::Button)
                        && node.label() == Some("重置")
                        && node.focused() == Some(true)
                })
        }));
        key(&mut surface, activation);
        assert_eq!(
            input.read_with(&surface, |input, _| input.value().to_string()),
            B4_DEFAULT_HEADERS
        );
        assert!(surface.update(|window, cx| input.read(cx).focus_handle(cx).is_focused(window)));
        assert_eq!(
            input.entity_id(),
            view.read_with(&surface, |view, _| view.default_request_headers.entity_id())
        );
        b4_focus_reset(&view, &mut surface);
        key(&mut surface, activation);
        assert_eq!(
            input.read_with(&surface, |input, _| input.value().to_string()),
            B4_DEFAULT_HEADERS
        );
    }
    b4_edit_headers(&view, &mut surface, "X-Draft: escape");
    b4_focus_reset(&view, &mut surface);
    key(&mut surface, "escape");
    assert_eq!(
        input.read_with(&surface, |input, _| input.value().to_string()),
        "X-Draft: escape"
    );
    assert!(!view.read_with(&surface, |view, _| view.saving));
    assert_eq!(
        std::fs::read(&path).expect("no implicit persistence"),
        before
    );
}

#[gpui_kit::test]
fn b4_headers_apply_preserves_concurrent_untouched_download_leaves(cx: &mut TestAppContext) {
    let f = fixture(cx);
    let path = f.directory.path().join("config.json");
    let (view, mut surface) = settings(&f, cx);
    select_page(&mut surface, DownloadSettingsPage::Request);
    b4_edit_headers(&view, &mut surface, "User-Agent: selected-draft");
    AppConfig::update_section(&path, |config| {
        config.download.default_cookie = "concurrent=1".into();
        config.download.taskbar_progress_overlay.offset_y = 48;
        config.download.background_speed_limit.bytes_per_second = 1537;
    })
    .expect("original concurrent configuration writer");
    let apply = surface
        .debug_bounds("download-settings-apply")
        .expect("original Apply");
    surface.simulate_click(apply.center(), Modifiers::default());
    surface.run_until_parked();
    let saved = AppConfig::load_from(&path).expect("original save output");
    assert_eq!(
        saved.download.default_request_headers,
        "User-Agent: selected-draft"
    );
    assert_eq!(
        saved.download.default_cookie, "concurrent=1",
        "untouched Header reset/apply must preserve concurrent Cookie"
    );
    assert_eq!(saved.download.taskbar_progress_overlay.offset_y, 48);
    assert_eq!(saved.download.background_speed_limit.bytes_per_second, 1537);
    assert_eq!(
        f.application
            .read_with(cx, |app, _| app.config.download.clone()),
        saved.download
    );
}

fn b4_click(surface: &mut VisualTestContext, selector: &'static str) {
    let hit = surface.debug_bounds(selector).expect("original action");
    surface.simulate_click(hit.center(), Modifiers::default());
    surface.run_until_parked();
}

#[gpui_kit::test]
fn b4_headers_open_displays_effective_text_without_writing(cx: &mut TestAppContext) {
    for variant in ["new", "missing", "whitespace", "custom"] {
        let f = fixture(cx);
        let path = f.directory.path().join("config.json");
        let mut raw =
            serde_json::to_value(f.application.read_with(cx, |app, _| app.config.clone()))
                .expect("owned configuration");
        match variant {
            "new" => raw["download"] = serde_json::json!({}),
            "missing" => {
                raw["download"]
                    .as_object_mut()
                    .expect("download section")
                    .remove("default_request_headers");
            }
            "whitespace" => {
                raw["download"]["default_request_headers"] = serde_json::json!(" \r\n\t ")
            }
            _ => {
                raw["download"]["default_request_headers"] =
                    serde_json::json!("X-Custom: preserved")
            }
        }
        std::fs::write(
            &path,
            serde_json::to_vec_pretty(&raw).expect("configuration bytes"),
        )
        .expect("seed owned file");
        f.application.update(cx, |app, _| {
            app.config = AppConfig::load_from(&path).expect("original load")
        });
        let before = std::fs::read(&path).expect("before opening");
        let (view, mut surface) = settings(&f, cx);
        select_page(&mut surface, DownloadSettingsPage::Request);
        let expected = if variant == "custom" {
            "X-Custom: preserved"
        } else {
            B4_DEFAULT_HEADERS
        };
        assert_eq!(
            view.read_with(&surface, |view, cx| view
                .default_request_headers
                .read(cx)
                .value()
                .to_string()),
            expected
        );
        let field = reveal(&mut surface, "download-settings-default-headers-field");
        assert!(
            field.size.height > px(0.0),
            "actual editable Textarea is mounted"
        );
        assert_eq!(std::fs::read(&path).expect("opening is read only"), before);
        b4_click(&mut surface, "download-settings-cancel");
        assert_eq!(std::fs::read(&path).expect("Cancel is read only"), before);
    }
}

#[gpui_kit::test]
fn b4_headers_blank_and_mouse_reset_apply_reopen_at_narrow_size(cx: &mut TestAppContext) {
    use gpui_kit::Focusable as _;
    for reset in [false, true] {
        let f = fixture(cx);
        let path = f.directory.path().join("config.json");
        f.application.update(cx, |app, _| {
            app.config.download.default_request_headers = "User-Agent: saved-custom".into();
            app.config.download.default_cookie = "preserved=1".into();
            app.config.download.default_directory.clear();
            app.config.save_to(&path).expect("owned settings");
        });
        let before_bytes = std::fs::read(&path).expect("saved bytes before editing");
        let baseline = AppConfig::load_from(&path).expect("baseline").download;
        let (view, mut surface) = settings(&f, cx);
        surface.simulate_resize(size(px(720.0), px(480.0)));
        draw(&mut surface);
        select_page(&mut surface, DownloadSettingsPage::Request);
        let input = view.read_with(&surface, |view, _| view.default_request_headers.clone());
        b4_edit_headers(&view, &mut surface, "");
        assert_eq!(
            std::fs::read(&path).expect("blank edit is draft only"),
            before_bytes
        );
        if reset {
            let hit = reveal(&mut surface, "download-settings-reset-headers");
            let footer = surface
                .debug_bounds("download-settings-shell-footer")
                .expect("fixed footer");
            assert!(hit.top() >= px(0.0) && hit.bottom() <= footer.top());
            assert!(hit.left() >= px(0.0) && hit.right() <= px(720.0));
            surface.simulate_click(hit.center(), Modifiers::default());
            draw(&mut surface);
            assert_eq!(
                input.read_with(&surface, |input, _| input.value().to_string()),
                B4_DEFAULT_HEADERS
            );
            assert!(
                surface.update(|window, cx| input.read(cx).focus_handle(cx).is_focused(window))
            );
        }
        assert_eq!(
            input.entity_id(),
            view.read_with(&surface, |view, _| view.default_request_headers.entity_id())
        );
        let footer = surface
            .debug_bounds("download-settings-shell-footer")
            .expect("footer");
        let apply = surface
            .debug_bounds("download-settings-apply")
            .expect("Apply");
        assert!(apply.top() >= footer.top() && apply.bottom() <= px(480.0));
        let source_window = surface.update(|window, _| window.window_handle());
        b4_click(&mut surface, "download-settings-apply");
        assert!(
            !cx.read(|cx| cx.windows().contains(&source_window)),
            "successful Apply removes the original window"
        );
        let saved = AppConfig::load_from(&path).expect("saved");
        let mut expected = baseline;
        expected.default_request_headers = B4_DEFAULT_HEADERS.into();
        assert_eq!(saved.download, expected, "only Header changes");
        assert_eq!(
            f.application
                .read_with(cx, |app, _| app.default_directory.clone()),
            f.directory.path(),
            "empty configured directory retains the original effective fallback"
        );
        let (reopened, mut surface) = settings(&f, cx);
        select_page(&mut surface, DownloadSettingsPage::Request);
        assert_eq!(
            reopened.read_with(&surface, |view, cx| view
                .default_request_headers
                .read(cx)
                .value()
                .to_string()),
            B4_DEFAULT_HEADERS
        );
        b4_click(&mut surface, "download-settings-cancel");
    }
}

#[gpui_kit::test]
fn b4_headers_failed_apply_keeps_reset_draft_and_runtime_policy(cx: &mut TestAppContext) {
    let f = fixture(cx);
    let path = f.directory.path().join("config.json");
    f.application.update(cx, |app, _| {
        app.config.download.default_request_headers = "User-Agent: saved-before-failure".into();
        app.config.save_to(&path).expect("owned custom settings");
        app.engine
            .update_download_http_policy(&app.config.download)
            .expect("original policy");
    });
    let before = f
        .application
        .read_with(cx, |app, _| app.config.download.clone());
    let (view, mut surface) = settings(&f, cx);
    select_page(&mut surface, DownloadSettingsPage::Request);
    b4_edit_headers(&view, &mut surface, "X-Draft: failure");
    let reset = reveal(&mut surface, "download-settings-reset-headers");
    surface.simulate_click(reset.center(), Modifiers::default());
    draw(&mut surface);
    let input = view.read_with(&surface, |view, _| view.default_request_headers.clone());
    std::fs::write(&path, b"{invalid-owned-config").expect("owned failure fixture");
    b4_click(&mut surface, "download-settings-apply");
    view.read_with(&surface, |view, cx| {
        assert!(!view.saving);
        assert_eq!(view.default_request_headers.entity_id(), input.entity_id());
        assert_eq!(
            view.default_request_headers.read(cx).value().as_str(),
            B4_DEFAULT_HEADERS
        );
    });
    assert_eq!(
        std::fs::read(&path).expect("corrupt file retained"),
        b"{invalid-owned-config"
    );
    f.application.read_with(&surface, |app, cx| {
        assert!(app.settings_window.is_some());
        assert_eq!(app.config.download, before);
        assert!(
            app.notifications
                .read(cx)
                .items()
                .iter()
                .any(|notice| notice.level == crate::notifications::NotificationLevel::Error)
        );
        assert!(
            !app.notifications
                .read(cx)
                .items()
                .iter()
                .any(|notice| notice.level == crate::notifications::NotificationLevel::Success)
        );
        let task_id = app
            .engine
            .enqueue(request(&f), RequestOrigin::Ui)
            .expect("future acceptance after failed save");
        let task = app.engine.task(task_id).expect("query").expect("task");
        assert!(
            task.task
                .request()
                .headers
                .iter()
                .any(|header| header.name.eq_ignore_ascii_case("user-agent")
                    && header.value == "saved-before-failure")
        );
    });
    b4_click(&mut surface, "download-settings-cancel");
}

#[gpui_kit::test]
fn b4_headers_apply_preserves_untouched_non_kib_background_limit(cx: &mut TestAppContext) {
    let f = fixture(cx);
    let path = f.directory.path().join("config.json");
    f.application.update(cx, |app, _| {
        app.config.download.background_speed_limit.enabled = true;
        app.config.download.background_speed_limit.bytes_per_second = 1537;
        app.config
            .save_to(&path)
            .expect("owned exact-rate settings");
    });
    let (view, mut surface) = settings(&f, cx);
    select_page(&mut surface, DownloadSettingsPage::Request);
    b4_edit_headers(&view, &mut surface, "X-Only-Header: edited");
    b4_click(&mut surface, "download-settings-apply");
    assert_eq!(
        AppConfig::load_from(&path)
            .expect("saved")
            .download
            .background_speed_limit
            .bytes_per_second,
        1537,
        "Header-only Apply must not round an untouched background limit"
    );
}

#[gpui_kit::test]
fn b4_headers_apply_preserves_concurrent_queue_and_global_limit(cx: &mut TestAppContext) {
    let f = fixture(cx);
    let (view, mut surface) = settings(&f, cx);
    select_page(&mut surface, DownloadSettingsPage::Request);
    b4_edit_headers(&view, &mut surface, "X-Only-Header: concurrent-runtime");
    f.application.read_with(cx, |app, _| {
        assert!(matches!(
            app.engine
                .handle_command(DownloadCommandEnvelope {
                    protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
                    request_id: uuid::Uuid::new_v4(),
                    origin: RequestOrigin::Ui,
                    command: DownloadCommand::Queue {
                        action: DownloadQueueAction::SetMaxConcurrent {
                            queue_id: "default".into(),
                            max_concurrent_tasks: 7,
                        }
                    },
                })
                .outcome,
            DownloadCommandOutcome::QueueChanged { .. }
        ));
        assert!(matches!(
            app.engine
                .handle_command(DownloadCommandEnvelope {
                    protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
                    request_id: uuid::Uuid::new_v4(),
                    origin: RequestOrigin::Ui,
                    command: DownloadCommand::SetGlobalSpeedLimit {
                        bytes_per_second: Some(2049)
                    },
                })
                .outcome,
            DownloadCommandOutcome::Accepted { .. }
        ));
    });
    b4_click(&mut surface, "download-settings-apply");
    f.application.read_with(cx, |app, _| {
        assert_eq!(
            app.engine.global_speed_limit(),
            Some(2049),
            "untouched global limit keeps the concurrent engine value"
        );
        let reply = app.engine.handle_command(DownloadCommandEnvelope {
            protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
            request_id: uuid::Uuid::new_v4(),
            origin: RequestOrigin::Ui,
            command: DownloadCommand::Snapshot {
                query: Default::default(),
            },
        });
        let DownloadCommandOutcome::Snapshot(snapshot) = reply.outcome else {
            panic!("original queue snapshot");
        };
        assert_eq!(
            snapshot
                .queues
                .into_iter()
                .find(|queue| queue.id == "default")
                .expect("default queue")
                .max_concurrent_tasks,
            Some(7)
        );
    });
}

#[gpui_kit::test]
fn b4_ref08_actual_paused_task_opens_official_source_draft_and_escape_discards(
    cx: &mut TestAppContext,
) {
    let f = fixture(cx);
    let task_id = f.application.read_with(cx, |app, _| {
        let mut request = request(&f);
        request.scheduled_at_unix_ms = Some(i64::MAX / 2);
        let task_id = app
            .engine
            .enqueue(request, RequestOrigin::Ui)
            .expect("original queue admission");
        app.engine
            .pause(task_id)
            .expect("explicit scheduled-task pause");
        assert_eq!(
            app.engine
                .task(task_id)
                .expect("task")
                .expect("identity")
                .task
                .status(),
            DownloadStatus::Paused
        );
        task_id
    });
    f.application.update(cx, |app, cx| {
        app.refresh(cx);
        app.request_task_window(task_id, cx);
    });
    cx.run_until_parked();
    let handle = f.application.read_with(cx, |app, _| {
        app.task_windows
            .get(&task_id)
            .expect("keyed task window")
            .window
    });
    let mut surface = VisualTestContext::from_window(handle, cx);
    surface.simulate_resize(size(px(440.0), px(184.0)));
    surface.update(|window, _| window.activate_window());
    draw(&mut surface);
    let before = f.application.read_with(&surface, |app, _| {
        app.engine
            .task(task_id)
            .expect("task")
            .expect("same identity")
            .task
    });
    let action = surface
        .debug_bounds("download-window-replace-url")
        .expect("official paused source replacement Button must exist");
    surface.simulate_click(action.center(), Modifiers::default());
    draw(&mut surface);
    let input = surface
        .debug_bounds("download-replace-url-input")
        .expect("official source Input");
    surface.simulate_click(input.center(), Modifiers::default());
    key(&mut surface, "ctrl-a");
    surface.simulate_input("https://synthetic.invalid/replacement.bin");
    draw(&mut surface);
    assert!(
        surface
            .debug_bounds("download-replace-url-confirm")
            .is_some()
    );
    key(&mut surface, "escape");
    draw(&mut surface);
    assert!(
        surface
            .debug_bounds("download-replace-url-confirm")
            .is_none()
    );
    assert_eq!(
        f.application.read_with(&surface, |app, _| app
            .engine
            .task(task_id)
            .expect("task")
            .expect("same identity")
            .task),
        before
    );
}

use crate::completion_http_fixture as b4_url_fixture;

#[path = "../../../../crates/nxtools-download/tests/support/mod.rs"]
mod b4_range_fixture;

fn ref08_written_window(
    f: &Fixture,
    server: &b4_url_fixture::UrlServer,
    runtime: &tokio::runtime::Runtime,
    cx: &mut TestAppContext,
) -> (DownloadTaskId, VisualTestContext) {
    let engine = f.application.read_with(cx, |app, _| app.engine.clone());
    let task_id = runtime.block_on(async {
        let mut request = request(f);
        request.source_url = server.url("/partial-old");
        request.connection_limit = Some(1);
        let task_id = engine
            .submit(request, RequestOrigin::Ui)
            .expect("real writer admission");
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            if engine
                .task(task_id)
                .expect("task")
                .expect("identity")
                .downloaded_bytes
                > 0
            {
                break;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "partial bytes must be written"
            );
            tokio::time::sleep(std::time::Duration::from_millis(2)).await;
        }
        engine.pause(task_id).expect("explicit pause");
        let paused = engine
            .wait_for_terminal(task_id, std::time::Duration::from_secs(5))
            .await
            .expect("writer drain");
        assert_eq!(paused.task.status(), DownloadStatus::Paused);
        assert!(
            paused.downloaded_bytes > 0
                && paused.downloaded_bytes < b4_url_fixture::CONTENT.len() as u64
        );
        task_id
    });
    f.application.update(cx, |app, cx| {
        app.refresh(cx);
        app.request_task_window(task_id, cx);
    });
    cx.run_until_parked();
    let handle = f.application.read_with(cx, |app, _| {
        app.task_windows.get(&task_id).expect("keyed window").window
    });
    let mut surface = VisualTestContext::from_window(handle, cx);
    surface.simulate_resize(size(px(440.0), px(184.0)));
    surface.update(|window, _| window.activate_window());
    draw(&mut surface);
    let button = surface
        .debug_bounds("download-window-replace-url")
        .expect("original paused action");
    surface.simulate_click(button.center(), Modifiers::default());
    draw(&mut surface);
    (task_id, surface)
}

fn ref08_edit(surface: &mut VisualTestContext, value: &str) {
    let bounds = surface
        .debug_bounds("download-replace-url-input")
        .expect("official Input");
    surface.simulate_click(bounds.center(), Modifiers::default());
    key(surface, "ctrl-a");
    surface.simulate_input(value);
    draw(surface);
}

fn ref08_confirmation(cx: &mut TestAppContext, activation: &'static str) {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("owned HTTP runtime");
    let server = runtime.block_on(b4_url_fixture::UrlServer::start());
    let f = fixture_with_persistence(cx, true);
    let (task_id, mut surface) = ref08_written_window(&f, &server, &runtime, cx);
    let engine = f
        .application
        .read_with(&surface, |app, _| app.engine.clone());
    let store =
        nxtools_download::SqliteDownloadStore::open(f.directory.path().join("downloads.sqlite3"))
            .expect("original store");
    let before = engine.task(task_id).expect("task").expect("identity");
    let plan = store
        .load_transfer_plan(task_id)
        .expect("plan")
        .expect("persisted plan");
    let artifact = before.temporary_path.clone().expect("partial artifact");
    let bytes = std::fs::read(&artifact).expect("partial bytes");
    let path = if activation == "space" {
        "/held"
    } else {
        "/compatible"
    };
    let replacement = server.url(path);
    ref08_edit(&mut surface, &replacement);
    std::thread::scope(|scope| {
        let release = (activation == "space").then(|| {
            scope.spawn(|| {
                runtime.block_on(server.wait_held());
                server
                    .hold
                    .store(false, std::sync::atomic::Ordering::Release);
            })
        });
        if activation == "mouse" {
            let bounds = surface
                .debug_bounds("download-replace-url-confirm")
                .expect("official confirmation");
            surface.simulate_click(bounds.center(), Modifiers::default());
            draw(&mut surface);
        } else {
            tab_button(&mut surface, "download-replace-url-confirm");
            key(&mut surface, activation);
        }
        if let Some(release) = release {
            release.join().expect("owned candidate release worker");
        }
    });
    draw(&mut surface);
    runtime.block_on(server.wait_drained());
    assert_eq!(
        server
            .requests
            .lock()
            .expect("HTTP facts")
            .iter()
            .filter(|request| request.starts_with(&format!("HEAD {path} ")))
            .count(),
        1,
        "one confirmation probes once before original resume"
    );
    assert!(
        surface
            .debug_bounds("download-replace-url-confirm")
            .is_none(),
        "first actual activation completes replacement"
    );
    let accepted = engine.task(task_id).expect("task").expect("same identity");
    assert_eq!(accepted.task.request().source_url, replacement);
    assert_eq!(accepted.task.status(), DownloadStatus::Paused);
    assert_eq!(accepted.downloaded_bytes, before.downloaded_bytes);
    assert_eq!(std::fs::read(&artifact).expect("same artifact"), bytes);
    assert_eq!(
        store
            .load_transfer_plan(task_id)
            .expect("plan")
            .expect("same plan")
            .segments(),
        plan.segments()
    );
    assert_eq!(
        store
            .load_task(task_id)
            .expect("durable task")
            .expect("identity")
            .request(),
        accepted.task.request()
    );
    assert!(
        surface
            .debug_bounds("download-window-replace-url")
            .is_some(),
        "original paused actions restored"
    );
    runtime.block_on(async {
        engine.resume(task_id).expect("original resume");
        let completed = engine
            .wait_for_terminal(task_id, std::time::Duration::from_secs(5))
            .await
            .expect("final writer");
        assert_eq!(completed.task.status(), DownloadStatus::Completed);
        assert_eq!(
            std::fs::read(completed.final_path.expect("final path")).expect("output"),
            b4_url_fixture::CONTENT
        );
    });
    runtime.block_on(server.close());
}

#[gpui_kit::test]
fn b4_ref08_actual_pointer_confirmation_preserves_writer_and_sqlite(cx: &mut TestAppContext) {
    ref08_confirmation(cx, "mouse");
}

#[gpui_kit::test]
fn b4_ref08_actual_first_focused_enter_preserves_writer_and_sqlite(cx: &mut TestAppContext) {
    ref08_confirmation(cx, "enter");
}

#[gpui_kit::test]
fn b4_ref08_actual_first_focused_space_preserves_writer_and_sqlite(cx: &mut TestAppContext) {
    ref08_confirmation(cx, "space");
}

#[gpui_kit::test]
fn b4_ref08_actual_refusal_retains_input_retry_and_cancel(cx: &mut TestAppContext) {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("owned HTTP runtime");
    let server = runtime.block_on(b4_url_fixture::UrlServer::start());
    let f = fixture_with_persistence(cx, true);
    let (task_id, mut surface) = ref08_written_window(&f, &server, &runtime, cx);
    let view = f.application.read_with(&surface, |app, _| {
        app.task_windows.get(&task_id).expect("window").view.clone()
    });
    let input = view.read_with(&surface, |view, _| {
        view.source_draft.as_ref().expect("draft").input.clone()
    });
    let engine = f
        .application
        .read_with(&surface, |app, _| app.engine.clone());
    let before = engine.task(task_id).expect("task").expect("identity").task;
    ref08_edit(&mut surface, &server.url("/changed"));
    tab_button(&mut surface, "download-replace-url-confirm");
    key(&mut surface, "space");
    draw(&mut surface);
    assert!(
        surface.debug_bounds("download-replace-url-error").is_some(),
        "identity refusal is visible"
    );
    assert_eq!(
        view.read_with(&surface, |view, _| view
            .source_draft
            .as_ref()
            .expect("same draft")
            .input
            .entity_id()),
        input.entity_id()
    );
    assert_eq!(
        input.read_with(&surface, |input, _| input.value().to_string()),
        server.url("/changed")
    );
    assert_eq!(
        engine.task(task_id).expect("task").expect("identity").task,
        before
    );
    ref08_edit(&mut surface, &server.url("/failed"));
    let confirm = surface
        .debug_bounds("download-replace-url-confirm")
        .expect("retry action");
    surface.simulate_click(confirm.center(), Modifiers::default());
    draw(&mut surface);
    assert!(surface.debug_bounds("download-replace-url-error").is_some());
    let cancel = surface
        .debug_bounds("download-replace-url-cancel")
        .expect("official Cancel");
    surface.simulate_click(cancel.center(), Modifiers::default());
    draw(&mut surface);
    assert!(
        surface
            .debug_bounds("download-replace-url-confirm")
            .is_none()
    );
    assert_eq!(
        engine.task(task_id).expect("task").expect("identity").task,
        before
    );
    let open = surface
        .debug_bounds("download-window-replace-url")
        .expect("reopen action");
    surface.simulate_click(open.center(), Modifiers::default());
    draw(&mut surface);
    let reopened = view.read_with(&surface, |view, _| {
        view.source_draft.as_ref().expect("new draft").input.clone()
    });
    assert_ne!(reopened.entity_id(), input.entity_id());
    assert_eq!(
        reopened.read_with(&surface, |input, _| input.value().to_string()),
        before.request().source_url
    );
    key(&mut surface, "escape");
    runtime.block_on(server.close());
}

#[gpui_kit::test]
fn b4_ref09_actual_template_input_previews_locally_before_admission(cx: &mut TestAppContext) {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("owned HTTP runtime");
    let server = runtime.block_on(b4_url_fixture::UrlServer::start());
    let f = fixture_with_persistence(cx, true);
    f.application
        .update(cx, |app, cx| app.open_new_download_window(cx));
    cx.run_until_parked();
    let handle = f.application.read_with(cx, |app, _| {
        app.new_download_window.expect("original intake")
    });
    let mut surface = VisualTestContext::from_window(handle, cx);
    surface.update(|window, _| window.activate_window());
    draw(&mut surface);
    let input = surface
        .debug_bounds("download-new-url-field-input")
        .expect("original URL Input");
    surface.simulate_click(input.center(), Modifiers::default());
    surface.simulate_input(&server.url("/file_{001..003}.bin"));
    draw(&mut surface);
    let next = surface
        .debug_bounds("download-new-probe-action")
        .expect("original next action");
    surface.simulate_click(next.center(), Modifiers::default());
    assert!(
        surface.read(|cx| cx.windows().contains(&handle)),
        "local template preview must retain the original intake without ordinary probing"
    );
    draw(&mut surface);
    assert!(
        surface.debug_bounds("download-template-preview").is_some(),
        "actual template input must open a local exact preview"
    );
    assert!(
        f.application
            .read_with(&surface, |app, _| app.tasks().is_empty()),
        "preview creates no tasks"
    );
    assert!(
        server.requests.lock().expect("HTTP facts").is_empty(),
        "preview sends no requests"
    );
    key(&mut surface, "escape");
    runtime.block_on(server.close());
}

#[gpui_kit::test]
fn b4_ref08_actual_close_and_stale_identity_preserve_reopened_window(cx: &mut TestAppContext) {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("owned HTTP runtime");
    let server = runtime.block_on(b4_url_fixture::UrlServer::start());
    let f = fixture_with_persistence(cx, true);
    let (task_id, mut surface) = ref08_written_window(&f, &server, &runtime, cx);
    let old = surface.update(|window, _| window.window_handle());
    let old_view = f.application.read_with(&surface, |app, _| {
        app.task_windows.get(&task_id).expect("window").view.clone()
    });
    f.application
        .update(&mut surface, |app, cx| app.close_task_window(task_id, cx));
    surface.run_until_parked();
    assert!(!surface.read(|cx| cx.windows().contains(&old)));
    f.application
        .update(&mut surface, |app, cx| app.request_task_window(task_id, cx));
    surface.run_until_parked();
    let current = f.application.read_with(&surface, |app, _| {
        app.task_windows.get(&task_id).expect("reopened").window
    });
    assert_ne!(old, current);
    f.application.update(&mut surface, |app, cx| {
        app.handle_task_window_native_close(task_id, old, cx)
    });
    assert_eq!(
        f.application.read_with(&surface, |app, _| app
            .task_windows
            .get(&task_id)
            .expect("new owner retained")
            .window),
        current
    );
    assert_eq!(
        f.application.read_with(&surface, |app, _| app
            .engine
            .task(task_id)
            .expect("task")
            .expect("identity")
            .task
            .status()),
        DownloadStatus::Paused
    );
    assert!(old_view.read_with(&surface, |view, _| view.source_draft.is_some()));
    runtime.block_on(server.close());
}

fn ref09_open(
    f: &Fixture,
    cx: &mut TestAppContext,
) -> (Entity<NewDownloadView>, VisualTestContext) {
    f.application
        .update(cx, |app, cx| app.open_new_download_window(cx));
    cx.run_until_parked();
    let (handle, view) = f.application.read_with(cx, |app, _| {
        (
            app.new_download_window.expect("intake"),
            app.new_download_view.clone().expect("draft"),
        )
    });
    let mut surface = VisualTestContext::from_window(handle, cx);
    surface.update(|window, _| window.activate_window());
    draw(&mut surface);
    (view, surface)
}

fn ref09_preview(surface: &mut VisualTestContext, value: &str) {
    let input = surface
        .debug_bounds("download-new-url-field-input")
        .expect("URL Input");
    surface.simulate_click(input.center(), Modifiers::default());
    key(surface, "ctrl-a");
    surface.simulate_input(value);
    draw(surface);
    let next = surface
        .debug_bounds("download-new-probe-action")
        .expect("original action");
    surface.simulate_click(next.center(), Modifiers::default());
    let requested_size = surface.update(|window, _| window.bounds().size);
    if requested_size != surface.update(|window, _| window.viewport_size()) {
        surface.simulate_resize(requested_size);
    }
    draw(surface);
}

fn ref09_confirm_selection(cx: &mut TestAppContext, activation: &'static str) {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("owned HTTP runtime");
    let server = runtime.block_on(b4_url_fixture::UrlServer::start());
    let f = fixture_with_persistence(cx, true);
    let (view, mut surface) = ref09_open(&f, cx);
    let input_id = view.read_with(&surface, |view, _| view.url.entity_id());
    ref09_preview(&mut surface, &server.url("/file_{001..003}.bin"));
    let entries = view.read_with(&surface, |view, _| {
        view.template_preview
            .as_ref()
            .expect("local preview")
            .entries
            .clone()
    });
    assert_eq!(
        entries
            .iter()
            .map(|entry| entry.request.source_url.clone())
            .collect::<Vec<_>>(),
        (1..=3)
            .map(|index| server.url(&format!("/file_{index:03}.bin")))
            .collect::<Vec<_>>()
    );
    assert!(server.requests.lock().expect("requests").is_empty());
    let checkbox = surface
        .debug_bounds("download-template-select-1")
        .expect("official second Checkbox");
    assert!(
        checkbox.size.width >= px(12.0) && checkbox.size.height >= px(12.0),
        "official Checkbox retains its pointer hit area: {checkbox:?}"
    );
    surface.simulate_click(checkbox.center(), Modifiers::default());
    draw(&mut surface);
    assert_eq!(
        view.read_with(&surface, |view, _| view
            .template_preview
            .as_ref()
            .expect("preview")
            .selected
            .clone()),
        vec![true, false, true]
    );
    let confirm = surface
        .debug_bounds("download-template-confirm")
        .expect("official confirm");
    let root = surface.update(|window, _| window.viewport_size());
    assert!(
        confirm.top() >= px(0.0) && confirm.bottom() <= root.height,
        "fixed footer reachable"
    );
    if activation == "mouse" {
        surface.simulate_click(confirm.center(), Modifiers::default());
        draw(&mut surface);
    } else {
        tab_button(&mut surface, "download-template-confirm");
        key(&mut surface, activation);
    }
    draw(&mut surface);
    let results = view.read_with(&surface, |view, _| {
        view.template_preview
            .as_ref()
            .expect("same preview")
            .results
            .clone()
            .expect("original batch outcomes")
    });
    assert_eq!(results.len(), 2);
    assert_eq!(
        results
            .iter()
            .map(|result| result.handoff_id)
            .collect::<Vec<_>>(),
        vec![entries[0].handoff_id, entries[2].handoff_id]
    );
    assert!(
        results
            .iter()
            .all(|result| result.task_id.is_some() && result.fault.is_none())
    );
    let engine = f
        .application
        .read_with(&surface, |app, _| app.engine.clone());
    for (result, entry) in results.iter().zip([&entries[0], &entries[2]]) {
        let task = engine
            .task(result.task_id.expect("accepted"))
            .expect("engine")
            .expect("durable identity");
        assert_eq!(task.task.request().source_url, entry.request.source_url);
        assert_eq!(task.task.status(), DownloadStatus::Created);
    }
    assert_eq!(
        f.application
            .read_with(&surface, |app, _| app.tasks().len()),
        2
    );
    let count = server.requests.lock().expect("requests").len();
    surface.simulate_click(confirm.center(), Modifiers::default());
    if activation != "mouse" {
        key(&mut surface, activation);
    }
    draw(&mut surface);
    assert_eq!(
        server.requests.lock().expect("requests").len(),
        count,
        "completed preview never retries the batch"
    );
    assert_eq!(
        f.application
            .read_with(&surface, |app, _| app.tasks().len()),
        2
    );
    key(&mut surface, "escape");
    assert_eq!(
        view.read_with(&surface, |view, _| view.url.entity_id()),
        input_id
    );
    assert!(view.read_with(&surface, |view, _| view.template_preview.is_none()));
    let requests = runtime.block_on(server.close());
    assert!(
        !requests
            .iter()
            .any(|request| request.contains("/file_002.bin")),
        "unselected entry never enters original resolution or admission"
    );
}

#[gpui_kit::test]
fn b4_ref09_actual_pointer_selection_enters_original_batch_once(cx: &mut TestAppContext) {
    ref09_confirm_selection(cx, "mouse");
}
#[gpui_kit::test]
fn b4_ref09_actual_first_focused_enter_enters_original_batch_once(cx: &mut TestAppContext) {
    ref09_confirm_selection(cx, "enter");
}
#[gpui_kit::test]
fn b4_ref09_actual_first_focused_space_enters_original_batch_once(cx: &mut TestAppContext) {
    ref09_confirm_selection(cx, "space");
}

#[gpui_kit::test]
fn b4_ref09_actual_invalid_oversized_and_boundary_previews_are_local(cx: &mut TestAppContext) {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("owned HTTP runtime");
    let server = runtime.block_on(b4_url_fixture::UrlServer::start());
    let f = fixture_with_persistence(cx, true);
    let (view, mut surface) = ref09_open(&f, cx);
    for range in [
        "{3..1}",
        "{a..z}",
        "{1..2..1}",
        "{1..2}{1..2}",
        "{1..33}",
        "{0..18446744073709551615}",
        "{18446744073709551616..18446744073709551616}",
        "{1}",
        "{..2}",
    ] {
        ref09_preview(&mut surface, &server.url(&format!("/file_{range}.bin")));
        assert!(
            view.read_with(&surface, |view, _| view.template_preview.is_none()),
            "invalid {range}"
        );
        assert!(server.requests.lock().expect("requests").is_empty());
        assert!(
            f.application
                .read_with(&surface, |app, _| app.tasks().is_empty())
        );
    }
    for (range, count, first, last) in [
        ("{000..031}", 32, "000", "031"),
        ("{0..0}", 1, "0", "0"),
        (
            "{18446744073709551615..18446744073709551615}",
            1,
            "18446744073709551615",
            "18446744073709551615",
        ),
    ] {
        ref09_preview(&mut surface, &server.url(&format!("/file_{range}.bin")));
        view.read_with(&surface, |view, _| {
            let preview = view.template_preview.as_ref().expect("bounded valid range");
            assert_eq!(preview.entries.len(), count);
            assert_eq!(
                preview.entries[0].request.source_url,
                server.url(&format!("/file_{first}.bin"))
            );
            assert_eq!(
                preview.entries[count - 1].request.source_url,
                server.url(&format!("/file_{last}.bin"))
            );
        });
        let cancel = surface
            .debug_bounds("download-template-cancel")
            .expect("reachable fixed Cancel");
        surface.simulate_click(cancel.center(), Modifiers::default());
        draw(&mut surface);
    }
    assert!(server.requests.lock().expect("requests").is_empty());
    assert!(
        f.application
            .read_with(&surface, |app, _| app.tasks().is_empty())
    );
    runtime.block_on(server.close());
}

#[gpui_kit::test]
fn b4_ref09_actual_partial_acceptance_keeps_order_and_never_retries(cx: &mut TestAppContext) {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("owned HTTP runtime");
    let server = runtime.block_on(b4_url_fixture::UrlServer::start());
    let f = fixture_with_persistence(cx, true);
    f.application.read_with(cx, |app, _| {
        app.engine
            .update_download_organization_policy(&DownloadSettings {
                duplicate_strategy: nxtools_core::DownloadDuplicateStrategySettings::Reject,
                ..DownloadSettings::default()
            })
            .expect("original destination policy")
    });
    let (view, mut surface) = ref09_open(&f, cx);
    ref09_preview(&mut surface, &server.url("/same.bin?item={001..003}"));
    let confirm = surface
        .debug_bounds("download-template-confirm")
        .expect("confirm");
    surface.simulate_click(confirm.center(), Modifiers::default());
    draw(&mut surface);
    let results = view.read_with(&surface, |view, _| {
        view.template_preview
            .as_ref()
            .expect("preview")
            .results
            .clone()
            .expect("ordered results")
    });
    assert_eq!(results.len(), 3);
    assert!(results[0].task_id.is_some() && results[0].fault.is_none());
    assert!(
        results[1..]
            .iter()
            .all(|result| result.task_id.is_none() && result.fault.is_some())
    );
    assert_eq!(
        f.application
            .read_with(&surface, |app, _| app.tasks().len()),
        1
    );
    let count = server.requests.lock().expect("requests").len();
    surface.simulate_click(confirm.center(), Modifiers::default());
    draw(&mut surface);
    assert_eq!(server.requests.lock().expect("requests").len(), count);
    key(&mut surface, "escape");
    assert_eq!(
        f.application
            .read_with(&surface, |app, _| app.tasks().len()),
        1
    );
    runtime.block_on(server.close());
}

#[gpui_kit::test]
fn b4_ref10_actual_paused_window_exposes_written_range_coverage(cx: &mut TestAppContext) {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("owned HTTP runtime");
    let server = runtime.block_on(b4_url_fixture::UrlServer::start());
    let f = fixture_with_persistence(cx, true);
    let (task_id, mut surface) = ref08_written_window(&f, &server, &runtime, cx);
    key(&mut surface, "escape");
    let (view, expected) = f.application.read_with(&surface, |app, _| {
        let detail = load_download_task_detail(&app.engine, task_id).expect("persisted detail");
        (
            app.task_windows.get(&task_id).expect("window").view.clone(),
            super::super::range_coverage::persisted_range_progress(&detail.segments),
        )
    });
    assert!(!expected.is_empty() && expected.iter().any(|segment| segment.downloaded_bytes > 0));
    assert_eq!(
        view.read_with(&surface, |view, _| view.range_segments().to_vec()),
        expected
    );
    view.update(&mut surface, |view, cx| {
        let snapshot = view.snapshot.as_mut().expect("paused snapshot");
        let now = snapshot.task.updated_at_unix_ms();
        for status in [DownloadStatus::Waiting, DownloadStatus::Downloading] {
            snapshot
                .task
                .transition_to(status, now)
                .expect("resume presentation transition");
        }
        view.progress = None;
        assert_eq!(
            view.range_segments(),
            expected,
            "resumed window retains known persisted prefixes before first live range event"
        );
        view.snapshot
            .as_mut()
            .expect("snapshot")
            .task
            .transition_to(DownloadStatus::Paused, now)
            .expect("pause presentation");
        cx.notify();
    });
    let toggle = surface
        .debug_bounds("download-window-details-toggle")
        .expect("original details action");
    surface.simulate_click(toggle.center(), Modifiers::default());
    surface.simulate_resize(download_task_window_size_for_details(
        DownloadWindowPhase::Progress,
        true,
    ));
    draw(&mut surface);
    assert!(
        surface
            .debug_bounds("download-window-range-coverage")
            .is_some(),
        "real paused written ranges must appear in original expanded window"
    );
    assert!(
        surface
            .debug_bounds("download-window-progress-component")
            .is_some()
    );
    assert!(
        surface
            .debug_bounds("download-window-replace-url")
            .is_some()
    );
    let coverage = surface
        .debug_bounds("download-window-range-coverage")
        .expect("coverage");
    let footer = surface
        .debug_bounds("download-task-window-footer")
        .expect("footer");
    assert!(coverage.bottom() <= footer.top() && coverage.size.height >= px(8.0));
    f.application
        .update(&mut surface, |app, cx| app.close_task_window(task_id, cx));
    surface.run_until_parked();
    f.application
        .update(&mut surface, |app, cx| app.request_task_window(task_id, cx));
    surface.run_until_parked();
    let reopened = f.application.read_with(&surface, |app, _| {
        app.task_windows.get(&task_id).expect("reopen").view.clone()
    });
    assert_eq!(
        reopened.read_with(&surface, |view, _| view.range_segments().to_vec()),
        expected
    );
    runtime.block_on(server.close());
}

#[gpui_kit::test]
fn b4_ref09_actual_all_unselected_reselection_and_duplicate_url_use_original_policy(
    cx: &mut TestAppContext,
) {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("HTTP runtime");
    let server = runtime.block_on(b4_url_fixture::UrlServer::start());
    let f = fixture_with_persistence(cx, true);
    f.application.read_with(cx, |app, _| {
        app.engine
            .update_download_organization_policy(&DownloadSettings {
                duplicate_strategy: nxtools_core::DownloadDuplicateStrategySettings::Reject,
                ..DownloadSettings::default()
            })
            .expect("original duplicate policy")
    });
    let (view, mut surface) = ref09_open(&f, cx);
    for attempt in 0..2 {
        ref09_preview(&mut surface, &server.url("/file_{001..001}.bin"));
        let checkbox = surface
            .debug_bounds("download-template-select-0")
            .expect("actual Checkbox");
        surface.simulate_click(checkbox.center(), Modifiers::default());
        draw(&mut surface);
        let confirm = surface
            .debug_bounds("download-template-confirm")
            .expect("confirm");
        let before = server.requests.lock().expect("requests").len();
        surface.simulate_click(confirm.center(), Modifiers::default());
        draw(&mut surface);
        assert_eq!(server.requests.lock().expect("requests").len(), before);
        assert!(view.read_with(&surface, |view, _| {
            view.template_preview
                .as_ref()
                .expect("preview")
                .results
                .is_none()
        }));
        surface.simulate_click(checkbox.center(), Modifiers::default());
        draw(&mut surface);
        let confirm = surface
            .debug_bounds("download-template-confirm")
            .expect("enabled confirm");
        surface.simulate_click(confirm.center(), Modifiers::default());
        draw(&mut surface);
        let results = view.read_with(&surface, |view, _| {
            view.template_preview
                .as_ref()
                .expect("preview")
                .results
                .clone()
                .expect("original outcomes")
        });
        assert_eq!(results.len(), 1);
        if attempt == 0 {
            assert!(results[0].task_id.is_some() && results[0].fault.is_none());
        } else {
            assert!(
                results[0].task_id.is_none() && results[0].fault.is_some(),
                "original reserved destination rejects the repeated URL"
            );
        }
        assert_eq!(
            f.application
                .read_with(&surface, |app, _| app.tasks().len()),
            1
        );
        key(&mut surface, "escape");
        assert!(view.read_with(&surface, |view, _| view.template_preview.is_none()));
        let restored = surface.update(|window, _| window.bounds().size);
        surface.simulate_resize(restored);
        draw(&mut surface);
    }
    runtime.block_on(server.close());
}

#[gpui_kit::test]
fn b4_ref09_actual_long_narrow_preview_freezes_explicit_context(cx: &mut TestAppContext) {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("HTTP runtime");
    let server = runtime.block_on(b4_url_fixture::UrlServer::start());
    let f = fixture_with_persistence(cx, true);
    let (view, mut surface) = ref09_open(&f, cx);
    let mut explicit = request(&f);
    explicit.expected_total_bytes = None;
    explicit.headers = vec![nxtools_download::DownloadRequestHeader {
        name: "User-Agent".into(),
        value: "template-explicit".into(),
    }];
    explicit.cookie = Some("template=1".into());
    explicit.referrer = Some(server.url("/origin"));
    view.update(&mut surface, |view, _| {
        view.external_request = Some(explicit.clone())
    });
    let template = server.url(&format!(
        "/file_{{001..002}}.bin?label={}",
        "long".repeat(50)
    ));
    ref09_preview(&mut surface, &template);
    surface.simulate_resize(size(px(440.0), px(480.0)));
    draw(&mut surface);
    let frozen = view.read_with(&surface, |view, _| {
        view.template_preview
            .as_ref()
            .expect("preview")
            .entries
            .clone()
    });
    for (index, entry) in frozen.iter().enumerate() {
        let exact = format!("选择第 {} 项：{}", index + 1, entry.request.source_url);
        assert!(surface.update(|window, _| {
            gpui_kit::base::test_support::snapshots(window)
                .iter()
                .any(|node| node.label() == Some(exact.as_str()))
        }));
    }
    let confirm = surface
        .debug_bounds("download-template-confirm")
        .expect("fixed confirm");
    assert!(confirm.bottom() <= px(480.0) && confirm.left() >= px(0.0));
    assert!(server.requests.lock().expect("requests").is_empty());
    view.update(&mut surface, |view, _| {
        let changed = view.external_request.as_mut().expect("context");
        changed.headers[0].value = "later-draft".into();
        changed.cookie = Some("later=1".into());
        changed.referrer = Some(server.url("/later"));
    });
    surface.simulate_click(confirm.center(), Modifiers::default());
    draw(&mut surface);
    let results = view.read_with(&surface, |view, _| {
        view.template_preview
            .as_ref()
            .expect("preview")
            .results
            .clone()
            .expect("results")
    });
    let engine = f
        .application
        .read_with(&surface, |app, _| app.engine.clone());
    for result in &results {
        let task = engine
            .task(result.task_id.expect("accepted"))
            .expect("task")
            .expect("identity");
        assert_eq!(task.task.request().headers, explicit.headers);
        assert_eq!(task.task.request().cookie, explicit.cookie);
        assert_eq!(task.task.request().referrer, explicit.referrer);
    }
    key(&mut surface, "escape");
    let requests = runtime.block_on(server.close());
    assert!(!requests.is_empty());
    for fact in requests {
        let fact = fact.to_ascii_lowercase();
        assert!(fact.contains("user-agent: template-explicit\r\n"));
        assert!(fact.contains("cookie: template=1\r\n"));
        assert!(fact.contains(&format!(
            "referer: {}\r\n",
            explicit.referrer.as_ref().expect("referrer")
        )));
        assert!(!fact.contains("later-draft") && !fact.contains("later=1"));
    }
}

#[gpui_kit::test]
fn b4_ref10_actual_unknown_projection_keeps_narrow_actions_metrics_and_editor(
    cx: &mut TestAppContext,
) {
    let f = fixture(cx);
    let (_, view, mut surface) = task(&f, cx);
    view.update(&mut surface, |view, cx| {
        view.snapshot.as_mut().expect("snapshot").total_bytes = None;
        view.snapshot.as_mut().expect("snapshot").probe = None;
        cx.notify();
    });
    draw(&mut surface);
    let toggle = surface
        .debug_bounds("download-window-details-toggle")
        .expect("details");
    surface.simulate_click(toggle.center(), Modifiers::default());
    surface.simulate_resize(download_task_window_size_for_details(
        DownloadWindowPhase::Progress,
        true,
    ));
    draw(&mut surface);
    assert!(surface.update(|window, _| {
        gpui_kit::base::test_support::snapshots(window)
            .iter()
            .any(|node| node.label() == Some("真实字节区间：总大小未知 · 不显示覆盖比例"))
    }));
    let footer = surface
        .debug_bounds("download-task-window-footer")
        .expect("footer");
    for selector in [
        "download-window-range-coverage",
        "download-window-progress-component",
        "download-window-progress-metrics",
        "download-window-progress-capabilities",
        "download-window-segment-table",
    ] {
        let bounds = surface.debug_bounds(selector).expect(selector);
        assert!(
            bounds.left() >= px(0.0)
                && bounds.right() <= px(440.0)
                && bounds.bottom() <= footer.top(),
            "{selector}: {bounds:?}"
        );
    }
    assert!(view.read_with(&surface, |view, _| view.speed_limit_kib.is_some()));
}

#[gpui_kit::test]
fn b4_ref10_cached_detail_applies_current_rebalanced_ranges_without_requery(
    cx: &mut TestAppContext,
) {
    let f = fixture(cx);
    let task_id = f.application.read_with(cx, |app, _| {
        app.engine
            .enqueue(request(&f), RequestOrigin::Ui)
            .expect("original queue")
    });
    let mut detail = f.application.read_with(cx, |app, _| {
        load_download_task_detail(&app.engine, task_id).expect("original TaskDetail")
    });
    detail.segments = vec![
        DownloadSegment::new(0, 0, 99, 30, DownloadSegmentState::Active).expect("original range"),
    ];
    let progress = DownloadProgress {
        task_id,
        downloaded_bytes: 40,
        total_bytes: Some(100),
        bytes_per_second: 0,
        active_connections: 2,
        segments: vec![
            nxtools_download::DownloadSegmentProgress {
                segment_index: 0,
                start_byte: Some(0),
                total_bytes: Some(60),
                downloaded_bytes: 30,
            },
            nxtools_download::DownloadSegmentProgress {
                segment_index: 2,
                start_byte: Some(60),
                total_bytes: Some(40),
                downloaded_bytes: 10,
            },
        ],
        finishing: None,
    };
    apply_progress_to_task_detail(&mut detail, &progress);
    assert_eq!(detail.segments.len(), 2);
    assert_eq!(
        (
            detail.segments[0].start_byte(),
            detail.segments[0].end_byte(),
            detail.segments[0].downloaded_bytes()
        ),
        (0, 59, 30)
    );
    assert_eq!(
        (
            detail.segments[1].start_byte(),
            detail.segments[1].end_byte(),
            detail.segments[1].downloaded_bytes()
        ),
        (60, 99, 10)
    );
}

#[gpui_kit::test]
fn b4_ref10_real_parallel_events_refresh_original_window_through_rebalance(
    cx: &mut TestAppContext,
) {
    cx.executor().allow_parking();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("owned parallel HTTP runtime");
    let server = b4_range_fixture::DeterministicHttpServer::start_concurrent();
    server.hold_imbalanced_response(true);
    let f = fixture_with_persistence(cx, true);
    let engine = f.application.read_with(cx, |app, _| app.engine.clone());
    let events = engine
        .subscribe_events()
        .expect("bounded state-admission observer");
    let task_id = runtime.block_on(async {
        let mut request = request(&f);
        request.source_url = format!("http://{}/range-imbalanced", server.address());
        request.file_name = Some("live-coverage.bin".into());
        request.connection_limit = Some(2);
        request.expected_total_bytes = None;
        engine
            .enqueue(request, RequestOrigin::Ui)
            .expect("original parallel admission")
    });
    f.application.update(cx, |app, cx| {
        app.refresh(cx);
        app.request_task_window(task_id, cx);
    });
    cx.run_until_parked();
    let (handle, view) = f.application.read_with(cx, |app, _| {
        let entry = app
            .task_windows
            .get(&task_id)
            .expect("original keyed window");
        (entry.window, entry.view.clone())
    });
    let mut surface = VisualTestContext::from_window(handle, cx);
    surface.simulate_resize(size(px(440.0), px(184.0)));
    surface.update(|window, _| window.activate_window());
    draw(&mut surface);
    let input = view.read_with(&surface, |view, _| {
        view.speed_limit_kib.clone().expect("original speed editor")
    });
    let toggle = surface
        .debug_bounds("download-window-details-toggle")
        .expect("actual disclosure");
    surface.simulate_click(toggle.center(), Modifiers::default());
    surface.simulate_resize(download_task_window_size_for_details(
        DownloadWindowPhase::Progress,
        true,
    ));
    draw(&mut surface);
    runtime.block_on(async {
        engine
            .resume(task_id)
            .expect("start after original window preparation");
    });
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(8);
    let mut downloading_sequence = None;
    loop {
        surface.run_until_parked();
        while let Ok(event) = events.try_recv() {
            if matches!(&event.event, DownloadEvent::TaskChanged { task }
                if task.id() == task_id && task.status() == DownloadStatus::Downloading)
            {
                downloading_sequence = Some(event.sequence);
            }
        }
        if downloading_sequence.is_some_and(|sequence| {
            f.application
                .read_with(&surface, |app, _| app.last_event_sequence >= sequence)
        }) && view.read_with(&surface, |view, _| {
            view.snapshot
                .as_ref()
                .is_some_and(|snapshot| snapshot.task.status() == DownloadStatus::Downloading)
        }) {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "initial downloading state reaches the original view"
        );
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    server.hold_imbalanced_response(false);
    let progress = loop {
        surface.run_until_parked();
        let progress = view.read_with(&surface, |view, _| view.progress.clone());
        let observed = progress.as_ref().map(|progress| {
            (
                progress.downloaded_bytes,
                progress.total_bytes,
                progress.segments.len(),
                progress
                    .segments
                    .iter()
                    .map(|segment| segment.segment_index)
                    .max(),
            )
        });
        if let Some(progress) = progress.filter(|progress| {
            progress.downloaded_bytes > 0
                && Some(progress.downloaded_bytes) != progress.total_bytes
                && progress.segments.len() >= 2
                && progress
                    .segments
                    .iter()
                    .any(|segment| segment.segment_index >= 2)
                && server.dynamic_range_attempts() >= 4
        }) {
            break progress;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "real rebalanced partial event reaches production view: attempts={}, observed={observed:?}",
            server.dynamic_range_attempts(),
        );
        std::thread::sleep(std::time::Duration::from_millis(2));
    };
    f.application.read_with(&surface, |app, _| {
        let entry = app.task_windows.get(&task_id).expect("same live window");
        assert_eq!(entry.window, handle);
        assert_eq!(entry.view.entity_id(), view.entity_id());
    });
    view.read_with(&surface, |view, _| {
        assert_eq!(view.range_segments(), progress.segments);
        assert_eq!(
            view.speed_limit_kib
                .as_ref()
                .expect("retained editor")
                .entity_id(),
            input.entity_id()
        );
    });
    let total = progress.total_bytes.expect("real known total");
    let mut ranges = Vec::new();
    let mut written = 0;
    for segment in &progress.segments {
        let start = segment.start_byte.expect("original true byte origin");
        let length = segment.total_bytes.expect("original real range length");
        assert!(length > 0 && start < total && length <= total - start);
        assert!(segment.downloaded_bytes <= length);
        ranges.push((start, start + length));
        written += segment.downloaded_bytes;
    }
    assert_eq!(written, progress.downloaded_bytes);
    assert!(written > 0 && written < total);
    ranges.sort_unstable();
    assert!(ranges.windows(2).all(|pair| pair[0].1 <= pair[1].0));
    surface.update(|window, cx| {
        window.refresh();
        window.draw(cx).clear(cx);
    });
    let footer = surface
        .debug_bounds("download-task-window-footer")
        .expect("fixed footer");
    for selector in [
        "download-window-range-coverage",
        "download-window-progress-component",
        "download-window-progress-metrics",
        "download-window-progress-capabilities",
        "download-window-server-value",
        "download-window-segment-table",
    ] {
        let bounds = surface.debug_bounds(selector).expect(selector);
        assert!(bounds.size.width > px(0.0) && bounds.size.height > px(0.0));
        assert!(
            bounds.left() >= px(0.0)
                && bounds.right() <= px(440.0)
                && bounds.bottom() <= footer.top(),
            "{selector}: {bounds:?}"
        );
    }
    let cancel = surface
        .debug_bounds("download-window-cancel-active")
        .expect("original active action");
    assert!(
        cancel.top() >= footer.top() && cancel.bottom() <= px(TASK_WINDOW_PROGRESS_EXPANDED_HEIGHT)
    );
    assert!(surface.update(|window, _| {
        gpui_kit::base::test_support::snapshots(window)
            .iter()
            .any(|node| node.label() == Some("真实字节区间：已完成 · 已写入前缀 · 空白为未写入"))
    }));
    server.hold_imbalanced_response(false);
    let completed = runtime
        .block_on(engine.wait_for_terminal(task_id, std::time::Duration::from_secs(8)))
        .expect("original writer terminal and drain");
    assert_eq!(
        completed.task.status(),
        DownloadStatus::Completed,
        "{:?}",
        completed.error
    );
    assert_eq!(
        std::fs::read(completed.final_path.expect("published output")).expect("exact writer bytes"),
        b4_range_fixture::imbalanced_fixture_bytes()
    );
    let drain_deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    while server.active_connection_workers() != 0 {
        assert!(
            std::time::Instant::now() < drain_deadline,
            "owned HTTP workers drain"
        );
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    surface.run_until_parked();
    f.application
        .update(&mut surface, |app, cx| app.close_task_window(task_id, cx));
    surface.run_until_parked();
    let address = server.address();
    drop(server);
    assert!(
        std::net::TcpStream::connect(address).is_err(),
        "owned listener closes"
    );
    drop(runtime);
}

#[gpui_kit::test]
fn b4_e2_native_close_retires_held_probe_without_changing_reopened_task(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("owned HTTP runtime");
    let server = runtime.block_on(b4_url_fixture::UrlServer::start());
    let f = fixture_with_persistence(cx, true);
    let (task_id, mut surface) = ref08_written_window(&f, &server, &runtime, cx);
    let engine = f
        .application
        .read_with(&surface, |app, _| app.engine.clone());
    let store =
        nxtools_download::SqliteDownloadStore::open(f.directory.path().join("downloads.sqlite3"))
            .expect("original SQLite store");
    let paused = engine.task(task_id).expect("task").expect("identity");
    let plan = store
        .load_transfer_plan(task_id)
        .expect("plan")
        .expect("written plan");
    let artifact = paused.temporary_path.as_ref().expect("original artifact");
    let bytes = std::fs::read(artifact).expect("written prefix");
    let old_window = surface.update(|window, _| window.window_handle());
    let old_view = f.application.read_with(&surface, |app, _| {
        app.task_windows
            .get(&task_id)
            .expect("original owner")
            .view
            .clone()
    });
    ref08_edit(&mut surface, &server.url("/held"));
    let confirm = surface
        .debug_bounds("download-replace-url-confirm")
        .expect("official confirmation");
    use gpui_kit::InputEvent;
    for event in [
        gpui_kit::MouseDownEvent {
            position: confirm.center(),
            modifiers: Modifiers::default(),
            button: gpui_kit::MouseButton::Left,
            click_count: 1,
            first_mouse: false,
        }
        .to_platform_input(),
        gpui_kit::MouseUpEvent {
            position: confirm.center(),
            modifiers: Modifiers::default(),
            button: gpui_kit::MouseButton::Left,
            click_count: 1,
        }
        .to_platform_input(),
    ] {
        surface.update(|window, cx| {
            window.dispatch_event(event, cx);
        });
    }
    let scheduler = cx.dispatcher.scheduler().clone();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    while scheduler.pending_task_counts().1 == 0 {
        assert!(
            std::time::Instant::now() < deadline && cx.dispatcher.tick(false),
            "original confirmation schedules background probe"
        );
    }
    assert_eq!(
        scheduler.pending_task_counts().1,
        1,
        "one original background candidate"
    );
    let dispatcher = cx.dispatcher.clone();
    let candidate_worker = std::thread::spawn(move || {
        assert!(dispatcher.tick(true), "run original background candidate");
    });
    runtime.block_on(server.wait_held());
    assert_eq!(
        engine
            .paused_url_replacement_revision(task_id)
            .expect("acquired candidate"),
        1
    );
    assert!(server.hold.load(std::sync::atomic::Ordering::Acquire));
    assert!(
        surface.simulate_close(),
        "original native-close callback permits close"
    );
    if surface.read(|cx| cx.windows().contains(&old_window)) {
        surface.update(|window, _| window.remove_window());
    }
    candidate_worker
        .join()
        .expect("original candidate physically returned");
    assert!(
        scheduler.pending_task_counts().0 > 0,
        "old foreground result is still queued"
    );
    assert!(!cx.read(|cx| cx.windows().contains(&old_window)));
    f.application
        .update(cx, |app, cx| app.request_task_window(task_id, cx));
    let (current_window, current_view) = f.application.read_with(cx, |app, _| {
        let entry = app.task_windows.get(&task_id).expect("same task reopened");
        (entry.window, entry.view.clone())
    });
    assert_ne!(old_window, current_window);
    assert_ne!(old_view.entity_id(), current_view.entity_id());
    cx.run_until_parked();
    assert_eq!(
        f.application.read_with(cx, |app, _| app
            .task_windows
            .get(&task_id)
            .expect("reopened owner after late result")
            .window),
        current_window
    );
    assert!(
        current_view.read_with(cx, |view, _| view.source_draft.is_none()),
        "old reply cannot install a draft in the new window"
    );
    runtime.block_on(server.wait_drained());
    server
        .hold
        .store(false, std::sync::atomic::Ordering::Release);
    let mut reopened = VisualTestContext::from_window(current_window, cx);
    reopened.simulate_resize(size(px(440.0), px(184.0)));
    reopened.update(|window, _| window.activate_window());
    draw(&mut reopened);
    assert_eq!(
        engine.task(task_id).expect("task").expect("identity").task,
        paused.task
    );
    let action = reopened
        .debug_bounds("download-window-replace-url")
        .expect("original paused action");
    reopened.simulate_click(action.center(), Modifiers::default());
    draw(&mut reopened);
    let new_input = current_view.read_with(&reopened, |view, _| {
        view.source_draft
            .as_ref()
            .expect("new draft")
            .input
            .entity_id()
    });
    ref08_edit(&mut reopened, &server.url("/failed"));
    let confirm = reopened
        .debug_bounds("download-replace-url-confirm")
        .expect("new confirmation");
    reopened.simulate_click(confirm.center(), Modifiers::default());
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        draw(&mut reopened);
        if engine
            .paused_url_replacement_revision(task_id)
            .expect("current revision")
            == 2
            && reopened
                .debug_bounds("download-replace-url-error")
                .is_some()
        {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "new probe acquires retired lease and returns its own error"
        );
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    let requests = server.requests.lock().expect("actual HTTP facts");
    assert_eq!(
        requests
            .iter()
            .filter(|request| request.starts_with("HEAD /held "))
            .count(),
        1
    );
    assert!(
        requests
            .iter()
            .any(|request| request.starts_with("HEAD /failed ")),
        "fresh attempt entered HTTP after old lease retired"
    );
    drop(requests);
    f.application.update(&mut reopened, |app, cx| {
        app.handle_task_window_native_close(task_id, old_window, cx);
    });
    draw(&mut reopened);
    f.application.read_with(&reopened, |app, _| {
        let entry = app.task_windows.get(&task_id).expect("new owner retained");
        assert_eq!(entry.window, current_window);
        assert_eq!(entry.view.entity_id(), current_view.entity_id());
    });
    assert_eq!(
        current_view.read_with(&reopened, |view, _| view
            .source_draft
            .as_ref()
            .expect("new draft retained")
            .input
            .entity_id()),
        new_input
    );
    let current = engine
        .task(task_id)
        .expect("task")
        .expect("original identity");
    assert_eq!(current.task, paused.task);
    assert_eq!(current.probe, paused.probe);
    assert_eq!(current.downloaded_bytes, paused.downloaded_bytes);
    assert_eq!(current.temporary_path, paused.temporary_path);
    assert_eq!(
        store.load_task(task_id).expect("durable task").as_ref(),
        Some(&paused.task)
    );
    assert_eq!(
        store
            .load_transfer_plan(task_id)
            .expect("retained plan")
            .as_ref(),
        Some(&plan)
    );
    assert_eq!(std::fs::read(artifact).expect("retained prefix"), bytes);
    assert!(
        store
            .is_queue_task_user_paused(task_id)
            .expect("explicit pause")
    );
    assert!(!store.is_queue_task_eligible(task_id).expect("not queued"));
    key(&mut reopened, "escape");
    let address = server.address;
    runtime.block_on(server.close());
    assert!(
        std::net::TcpStream::connect(address).is_err(),
        "owned listener exited"
    );
}

#[gpui_kit::test]
fn b5_ref07_task_intention_survives_original_window_and_engine_reload(cx: &mut TestAppContext) {
    let f = fixture_with_persistence(cx, true);
    let (task_id, original_view, mut surface) = task(&f, cx);
    let initial_status = f.application.read_with(cx, |app, _| {
        app.engine
            .task(task_id)
            .expect("original task")
            .expect("accepted task")
            .task
            .status()
    });
    crate::downloads::task_window::layout_tests::choose_completion_action(&mut surface, "关机");
    assert!(
        f.application
            .read_with(cx, |app, _| app.is_shutdown_armed(task_id))
    );
    f.application
        .update(cx, |app, cx| app.close_task_window(task_id, cx));
    cx.run_until_parked();
    f.application
        .update(cx, |app, cx| app.request_task_window(task_id, cx));
    cx.run_until_parked();
    assert!(
        f.application
            .read_with(cx, |app, _| app.is_shutdown_armed(task_id)),
        "presentation close must not clear intention"
    );
    assert_ne!(
        original_view,
        f.application.read_with(cx, |app, _| app
            .task_windows
            .get(&task_id)
            .expect("reopened task window")
            .view
            .clone())
    );
    let paths = f.directory.path().to_path_buf();
    let engine =
        DownloadEngine::open(paths.join("downloads.sqlite3")).expect("reload original SQLite");
    assert_eq!(
        engine
            .task(task_id)
            .expect("reload task")
            .expect("same task")
            .task
            .status(),
        initial_status,
        "closing/reopening cannot admit, pause or cancel work"
    );
    let (platform, _reloaded_endpoint) = nxtools_ipc::platform_bridge(32);
    let config = f.application.read_with(cx, |app, _| app.config.clone());
    let reloaded = cx.update(|cx| {
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
    assert!(
        reloaded.read_with(cx, |app, _| app.is_shutdown_armed(task_id)),
        "accepted completion intention must survive original SQLite/engine/application reload"
    );
    assert!(
        reloaded.read_with(cx, |app, _| app
            .completion_actions
            .active_countdown()
            .is_none()),
        "reload restores intention, not a pending power action"
    );
}

fn b5_menu_choose(
    surface: &mut VisualTestContext,
    selector: &'static str,
    label: &str,
    activation: &'static str,
) {
    if selector == "download-window-completion-menu" {
        crate::downloads::task_window::layout_tests::open_completion_page(surface);
    }
    draw(surface);
    if activation == "mouse" {
        let hit = surface
            .debug_bounds(selector)
            .expect("official completion menu trigger");
        surface.simulate_click(hit.center(), Modifiers::default());
    } else {
        tab_button(surface, selector);
        key(surface, activation);
    }
    draw(surface);
    let item = surface
        .update(|window, _| gpui_kit::base::test_support::snapshots(window))
        .into_iter()
        .find(|node| node.label() == Some(label))
        .expect("official completion menu item")
        .bounds();
    surface.simulate_click(item.center(), Modifiers::default());
    draw(surface);
}

#[gpui_kit::test]
fn b5_ref07_production_action_menu_first_keys_and_close_reopen(cx: &mut TestAppContext) {
    use nxtools_download::DownloadCompletionAction;
    let f = fixture_with_persistence(cx, true);
    let (task_id, view, mut surface) = task(&f, cx);
    for (activation, label, expected) in [
        ("mouse", "睡眠", DownloadCompletionAction::Sleep),
        ("enter", "休眠", DownloadCompletionAction::Hibernate),
        (
            "space",
            "退出应用",
            DownloadCompletionAction::ExitApplication,
        ),
    ] {
        b5_menu_choose(
            &mut surface,
            "download-window-completion-menu",
            label,
            activation,
        );
        assert_eq!(
            view.read_with(cx, |view, _| view.completion_intention.action),
            Some(expected)
        );
        assert_eq!(
            f.application.read_with(cx, |app, _| app
                .engine
                .task(task_id)
                .expect("task")
                .expect("identity")
                .task
                .status()),
            DownloadStatus::Created
        );
    }
    let menu = surface
        .debug_bounds("download-window-completion-menu")
        .expect("minimum task action");
    assert!(menu.left() >= px(0.0) && menu.right() <= px(440.0));
    f.application.update(cx, |app, cx| {
        app.close_task_window(task_id, cx);
        app.request_task_window(task_id, cx);
    });
    cx.run_until_parked();
    let intention = f.application.read_with(cx, |app, _| {
        app.engine
            .task(task_id)
            .expect("task")
            .expect("identity")
            .task
            .completion_intention()
    });
    assert_eq!(
        intention.action,
        Some(DownloadCompletionAction::ExitApplication)
    );
    let reloaded =
        DownloadEngine::open(f.directory.path().join("downloads.sqlite3")).expect("same SQLite");
    assert_eq!(
        reloaded
            .task(task_id)
            .expect("reload")
            .expect("identity")
            .task
            .completion_intention(),
        intention
    );
    assert!(f.application.read_with(cx, |app, _| {
        app.completion_actions.active_countdown().is_none()
    }));
}

#[gpui_kit::test]
fn b5_ref11_original_settings_keeps_queue_draft_until_explicit_save(cx: &mut TestAppContext) {
    use nxtools_download::DownloadCompletionAction;
    let f = fixture_with_persistence(cx, true);
    let (_, _, _task_surface) = task(&f, cx);
    let (view, mut surface) = settings(&f, cx);
    select_page(&mut surface, DownloadSettingsPage::Window);
    b5_menu_choose(
        &mut surface,
        "download-queue-completion-action",
        "休眠",
        "mouse",
    );
    assert_eq!(
        view.read_with(cx, |view, _| view.completion_queues[0].1.action),
        Some(DownloadCompletionAction::Hibernate)
    );
    assert_eq!(
        f.application.read_with(cx, |app, _| app
            .engine
            .completion_snapshot()
            .expect("snapshot")
            .queues[0]
            .completion_intention
            .action),
        None,
        "draft does not persist itself"
    );
    let directory = surface
        .debug_bounds("download-queue-completion-directory")
        .expect("explicit directory Checkbox");
    surface.simulate_click(directory.center(), Modifiers::default());
    draw(&mut surface);
    tab_button(&mut surface, "queue-completion-save-default");
    key(&mut surface, "enter");
    assert!(view.read_with(cx, |view, _| !view.saving && view.error.is_none()));
    let saved = f.application.read_with(cx, |app, _| {
        app.engine.completion_snapshot().expect("snapshot").queues[0].completion_intention
    });
    assert_eq!(saved.action, Some(DownloadCompletionAction::Hibernate));
    assert!(saved.open_directory);
    let reloaded = DownloadEngine::open(f.directory.path().join("downloads.sqlite3"))
        .expect("same original database");
    assert_eq!(
        reloaded.completion_snapshot().expect("snapshot").queues[0].completion_intention,
        saved
    );
}

#[gpui_kit::test]
fn b5_late_cancel_does_not_clear_new_production_countdown(cx: &mut TestAppContext) {
    let f = fixture(cx);
    let (old, current) = f.application.update(cx, |app, cx| {
        let first = DownloadTaskId::new();
        app.completion_actions.set_shutdown_armed(first, true);
        let ShutdownTransition::Started(old) = app
            .completion_actions
            .evaluate_shutdown([(first, DownloadStatus::Completed)], Instant::now())
        else {
            panic!("old countdown")
        };
        app.cancel_shutdown_countdown(old.correlation_id, "explicit", cx);
        let next = DownloadTaskId::new();
        app.completion_actions.set_shutdown_armed(next, true);
        let ShutdownTransition::Started(current) = app.completion_actions.evaluate_shutdown(
            [
                (first, DownloadStatus::Completed),
                (next, DownloadStatus::Completed),
            ],
            Instant::now(),
        ) else {
            panic!("new countdown")
        };
        app.apply_shutdown_transition(ShutdownTransition::Started(current), cx);
        (old, current)
    });
    cx.run_until_parked();
    f.application.update(cx, |app, cx| {
        app.cancel_shutdown_countdown(old.correlation_id, "late_old_window", cx)
    });
    assert_eq!(
        f.application
            .read_with(cx, |app, _| app.completion_actions.active_countdown()),
        Some(current)
    );
    let handle = f.application.read_with(cx, |app, _| {
        app.shutdown_window
            .as_ref()
            .expect("current presentation")
            .window
    });
    let mut surface = VisualTestContext::from_window(handle, cx);
    draw(&mut surface);
    tab_button(&mut surface, "download-shutdown-cancel-button");
    key(&mut surface, "escape");
    assert!(f.application.read_with(
        cx,
        |app, _| app.completion_actions.active_countdown().is_none()
            && app.shutdown_window.is_none()
    ));
}

#[gpui_kit::test]
fn b5_successful_original_writer_dispatches_suspend_once_and_preserves_native_failure(
    cx: &mut TestAppContext,
) {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("owned HTTP runtime");
    let server = runtime.block_on(b4_url_fixture::UrlServer::start());
    let f = fixture_with_persistence(cx, true);
    f._endpoint.set_connected_for_contract_test(true);
    let engine = f.application.read_with(cx, |app, _| app.engine.clone());
    let mut request = request(&f);
    request.source_url = server.url("/compatible");
    request.expected_total_bytes = None;
    let task_id = engine
        .enqueue(request, RequestOrigin::Internal)
        .expect("original accepted work");
    f.application.update(cx, |app, cx| {
        app.refresh(cx);
        app.save_completion_intention(
            CompletionIntentionTarget::Task(task_id),
            Some(Some(nxtools_download::DownloadCompletionAction::Sleep)),
            None,
            cx,
        );
    });
    engine.resume(task_id).expect("original resume");
    let completed = runtime
        .block_on(engine.wait_for_terminal(task_id, std::time::Duration::from_secs(5)))
        .expect("real publication");
    assert_eq!(completed.task.status(), DownloadStatus::Completed);
    assert_eq!(
        std::fs::read(completed.final_path.expect("final artifact")).expect("actual writer bytes"),
        b4_url_fixture::CONTENT
    );
    cx.run_until_parked();
    let (handle, correlation) = f.application.read_with(cx, |app, _| {
        let window = app
            .shutdown_window
            .as_ref()
            .expect("original completion countdown");
        (window.window, window.correlation_id)
    });
    let mut surface = VisualTestContext::from_window(handle, cx);
    draw(&mut surface);
    assert_eq!(
        f.application.read_with(cx, |app, _| app
            .completion_actions
            .active_countdown()
            .expect("authority")
            .action),
        nxtools_download::DownloadCompletionAction::Sleep
    );
    tab_button(&mut surface, "download-shutdown-now-button");
    key(&mut surface, "space");
    key(&mut surface, "space");
    let mut requested = None;
    while let Ok(command) = f._endpoint.try_receive_command_for_contract_test() {
        if matches!(
            command.message,
            nxtools_protocol::Message::PlatformCommand(PlatformCommand::SuspendSystem(
                SuspendSystemCommand { hibernate: false }
            ))
        ) {
            assert!(requested.replace(command).is_none(), "one native request");
        }
    }
    let command = requested.expect("actual typed suspend boundary");
    assert_eq!(command.correlation_id, Some(correlation));
    f.application.update(cx, |app, cx| {
        app.execute_shutdown(correlation, "duplicate", cx);
        app.handle_platform_notification(
            PlatformNotification::CommandResult {
                correlation_id: command.id,
                result: PlatformCommandResult {
                    success: false,
                    error: Some("Sleep unavailable".into()),
                    native_error_code: Some(50),
                    data: None,
                },
            },
            cx,
        );
        app.execute_shutdown(correlation, "late", cx);
    });
    assert!(
        f.application
            .read_with(cx, |app, _| app.shutdown_window.is_none()
                && app.pending_shutdown_command.is_none()
                && app.completion_actions.active_countdown().is_none())
    );
    assert!(
        f._endpoint.try_receive_command_for_contract_test().is_err(),
        "failure does not retry or substitute"
    );
    engine.shutdown_and_wait().expect("worker join");
    runtime.block_on(server.wait_drained());
    runtime.block_on(server.close());
}

#[gpui_kit::test]
fn b5_ref11_failed_queue_save_retains_official_form_draft_and_error(cx: &mut TestAppContext) {
    let f = fixture_with_persistence(cx, true);
    let (view, mut surface) = settings(&f, cx);
    select_page(&mut surface, DownloadSettingsPage::Window);
    b5_menu_choose(
        &mut surface,
        "download-queue-completion-action",
        "睡眠",
        "mouse",
    );
    let connection = rusqlite::Connection::open(f.directory.path().join("downloads.sqlite3"))
        .expect("original SQLite fixture");
    connection.execute_batch("CREATE TRIGGER b5_reject_queue_intention BEFORE UPDATE OF completion_intention_json ON download_queues BEGIN SELECT RAISE(ABORT,'completion intention save rejected'); END;").expect("controlled storage rejection");
    let button = surface
        .debug_bounds("download-queue-completion-save")
        .expect("official save");
    surface.simulate_click(button.center(), Modifiers::default());
    draw(&mut surface);
    assert!(view.read_with(cx, |view, _| !view.saving
        && view.error.as_ref().is_some_and(|message| {
            message.contains("completion intention save rejected")
        })));
    assert_eq!(
        view.read_with(cx, |view, _| view.completion_queues[0].1.action),
        Some(nxtools_download::DownloadCompletionAction::Sleep)
    );
    assert_eq!(
        f.application.read_with(cx, |app, _| app
            .engine
            .completion_snapshot()
            .expect("snapshot")
            .queues[0]
            .completion_intention),
        Default::default()
    );
}

#[gpui_kit::test]
fn b5_exit_requests_normal_testplatform_quit_without_claiming_resource_drain(
    cx: &mut TestAppContext,
) {
    let f = fixture_with_persistence(cx, true);
    let correlation = std::sync::Arc::new(std::sync::Mutex::new(None));
    let observation = correlation.clone();
    cx.update(|cx| {
        cx.set_global(crate::completion_actions::CompletionExitCorrelation(
            observation,
        ))
    });
    crate::downloads::tests::b5_complete_original_action_for_test(
        &f.application,
        cx,
        nxtools_download::DownloadCompletionAction::ExitApplication,
    );
    let (handle, expected) = f.application.read_with(cx, |app, _| {
        let window = app.shutdown_window.as_ref().expect("normal exit countdown");
        (window.window, window.correlation_id)
    });
    let mut surface = VisualTestContext::from_window(handle, cx);
    draw(&mut surface);
    let button = surface
        .debug_bounds("download-shutdown-danger-now-action")
        .expect("normal exit action");
    surface.simulate_click(button.center(), Modifiers::default());
    cx.run_until_parked();
    assert_eq!(
        *correlation.lock().expect("normal close correlation"),
        Some(expected)
    );
    assert!(
        f.application
            .read_with(cx, |app, _| app.shutdown_window.is_none()
                && app.pending_shutdown_command.is_none()
                && app.completion_actions.active_countdown().is_none())
    );
    f.application
        .read_with(cx, |app, _| app.engine.shutdown_and_wait())
        .expect("separate actual worker join");
}

#[gpui_kit::test]
fn b5_repair_held_native_reply_serializes_later_successful_work(cx: &mut TestAppContext) {
    let f = fixture_with_persistence(cx, true);
    f._endpoint.set_connected_for_contract_test(true);
    crate::downloads::tests::b5_complete_original_action_for_test(
        &f.application,
        cx,
        nxtools_download::DownloadCompletionAction::Sleep,
    );
    let first = f.application.read_with(cx, |app, _| {
        app.completion_actions
            .active_countdown()
            .expect("first countdown")
    });
    f.application.update(cx, |app, cx| {
        app.execute_shutdown(first.correlation_id, "fixture", cx)
    });
    let command_id = f.application.read_with(cx, |app, _| {
        app.pending_shutdown_command
            .as_ref()
            .expect("held typed request")
            .command_id
    });
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("HTTP runtime");
    let server = runtime.block_on(b4_url_fixture::UrlServer::start());
    let engine = f.application.read_with(cx, |app, _| app.engine.clone());
    let mut input = request(&f);
    input.source_url = server.url("/compatible");
    input.file_name = Some("later.bin".into());
    input.expected_total_bytes = None;
    let id = engine
        .enqueue(input, RequestOrigin::Ui)
        .expect("later accepted task");
    f.application.update(cx, |app, cx| {
        app.refresh(cx);
        app.save_completion_intention(
            CompletionIntentionTarget::Task(id),
            Some(Some(nxtools_download::DownloadCompletionAction::Hibernate)),
            None,
            cx,
        );
    });
    engine.resume(id).expect("actual worker");
    assert_eq!(
        runtime
            .block_on(engine.wait_for_terminal(id, Duration::from_secs(5)))
            .expect("actual success")
            .task
            .status(),
        DownloadStatus::Completed
    );
    f.application.update(cx, |app, cx| app.refresh(cx));
    cx.run_until_parked();
    let hidden_countdown = f
        .application
        .read_with(cx, |app, _| app.completion_actions.active_countdown());
    f.application.update(cx, |app, cx| {
        app.handle_shutdown_result(
            command_id,
            PlatformCommandResult {
                success: false,
                error: Some("controlled native failure".into()),
                native_error_code: Some(87),
                data: None,
            },
            cx,
        )
    });
    cx.run_until_parked();
    let presented = f.application.read_with(cx, |app, _| {
        app.completion_actions
            .active_countdown()
            .zip(app.shutdown_window.as_ref())
            .map(|(countdown, window)| {
                (
                    countdown.correlation_id,
                    window.correlation_id,
                    countdown.action,
                )
            })
    });
    engine.shutdown_and_wait().expect("actual worker join");
    runtime.block_on(server.wait_drained());
    runtime.block_on(server.close());
    assert!(
        hidden_countdown.is_none(),
        "no second countdown while a native request is held"
    );
    let (authority, window, action) =
        presented.expect("later authority must have its production window and timer");
    assert_eq!(authority, window);
    assert_eq!(
        action,
        nxtools_download::DownloadCompletionAction::Hibernate
    );
}

#[gpui_kit::test]
fn b5_new_accepted_work_cancels_production_countdown_through_original_event_pump(
    cx: &mut TestAppContext,
) {
    let f = fixture_with_persistence(cx, true);
    crate::downloads::tests::b5_complete_original_action_for_test(
        &f.application,
        cx,
        nxtools_download::DownloadCompletionAction::Sleep,
    );
    let old = f.application.read_with(cx, |app, _| {
        app.completion_actions
            .active_countdown()
            .expect("successful authority")
    });
    let engine = f.application.read_with(cx, |app, _| app.engine.clone());
    let accepted = engine
        .enqueue(request(&f), RequestOrigin::Ui)
        .expect("actually accepted queue-only work");
    cx.run_until_parked();
    assert!(
        f.application.read_with(cx, |app, _| app
            .completion_actions
            .active_countdown()
            .is_none()
            && app.shutdown_window.is_none()),
        "original accepted state event must cancel promptly, not at the old deadline"
    );
    f.application.update(cx, |app, cx| {
        app.execute_shutdown(old.correlation_id, "late", cx)
    });
    assert!(
        f.application
            .read_with(cx, |app, _| app.pending_shutdown_command.is_none())
    );
    engine.cancel(accepted).expect("cancel new work");
    cx.run_until_parked();
    assert!(f.application.read_with(cx, |app, _| {
        app.completion_actions.active_countdown().is_none()
    }));
    engine.shutdown_and_wait().expect("actual worker drain");
}

#[gpui_kit::test]
fn b5_suspend_reply_outlives_the_bounded_shutdown_ack_deadline(cx: &mut TestAppContext) {
    use nxtools_download::DownloadCompletionAction;
    let f = fixture_with_persistence(cx, true);
    f._endpoint.set_connected_for_contract_test(true);
    let mut retained = Vec::new();
    for action in [
        DownloadCompletionAction::Sleep,
        DownloadCompletionAction::Hibernate,
    ] {
        crate::downloads::tests::b5_complete_original_action_for_test(&f.application, cx, action);
        let correlation = f.application.read_with(cx, |app, _| {
            app.completion_actions
                .active_countdown()
                .expect("actual successful authority")
                .correlation_id
        });
        f.application.update(cx, |app, cx| {
            app.execute_shutdown(correlation, "fixture", cx)
        });
        let command_id = f.application.read_with(cx, |app, _| {
            app.pending_shutdown_command
                .as_ref()
                .expect("typed native request")
                .command_id
        });
        cx.run_until_parked();
        cx.executor()
            .advance_clock(COMPLETION_PLATFORM_TIMEOUT + Duration::from_secs(1));
        cx.run_until_parked();
        retained.push(f.application.read_with(cx, |app, _| {
            app.pending_shutdown_command
                .as_ref()
                .is_some_and(|pending| pending.command_id == command_id)
        }));
        f.application.update(cx, |app, cx| {
            app.handle_shutdown_result(
                command_id,
                PlatformCommandResult {
                    success: false,
                    error: Some("controlled native failure".into()),
                    native_error_code: Some(87),
                    data: None,
                },
                cx,
            )
        });
        cx.run_until_parked();
    }
    f.application
        .read_with(cx, |app, _| app.engine.shutdown_and_wait())
        .expect("actual worker joins");
    assert_eq!(
        retained,
        vec![true, true],
        "suspend owns its action slot until the actual native terminal, including time spent suspended"
    );
}

#[gpui_kit::test]
fn b5_platform_disconnect_retires_executing_window_with_unconfirmed_feedback(
    cx: &mut TestAppContext,
) {
    let f = fixture_with_persistence(cx, true);
    f._endpoint.set_connected_for_contract_test(true);
    crate::downloads::tests::b5_complete_original_action_for_test(
        &f.application,
        cx,
        nxtools_download::DownloadCompletionAction::Sleep,
    );
    let correlation = f.application.read_with(cx, |app, _| {
        app.completion_actions
            .active_countdown()
            .expect("actual successful authority")
            .correlation_id
    });
    f.application.update(cx, |app, cx| {
        app.execute_shutdown(correlation, "fixture", cx)
    });
    let (command_id, window) = f.application.read_with(cx, |app, _| {
        (
            app.pending_shutdown_command
                .as_ref()
                .expect("typed request")
                .command_id,
            app.shutdown_window
                .as_ref()
                .expect("executing window")
                .window,
        )
    });
    f.application.update(cx, |app, cx| {
        app.handle_platform_notification(
            PlatformNotification::Disconnected {
                generation: 1,
                pending_command_ids: vec![command_id],
            },
            cx,
        );
    });
    cx.run_until_parked();
    let retired = f.application.read_with(cx, |app, cx| {
        app.pending_shutdown_command.is_none()
            && app.shutdown_window.is_none()
            && !cx.windows().contains(&window)
    });
    let unconfirmed = f.application.read_with(cx, |app, cx| {
        app.notifications.read(cx).items().iter().any(|item| {
            item.id.as_ref() == "download-shutdown-failed"
                && item.level == crate::notifications::NotificationLevel::Error
                && item.message.as_ref() == "Windows 连接已断开，未确认完成动作结果"
        })
    });
    f.application.update(cx, |app, cx| {
        app.handle_shutdown_result(
            command_id,
            PlatformCommandResult {
                success: true,
                error: None,
                native_error_code: None,
                data: None,
            },
            cx,
        );
        app.refresh(cx);
    });
    cx.run_until_parked();
    assert!(
        f.application.read_with(cx, |app, _| {
            app.pending_shutdown_command.is_none()
                && app.completion_actions.active_countdown().is_none()
        }),
        "late native success must not replay consumed authority"
    );
    f.application
        .read_with(cx, |app, _| app.engine.shutdown_and_wait())
        .expect("actual worker joins");
    assert!(
        retired,
        "disconnect must retire the original executing presentation"
    );
    assert!(
        unconfirmed,
        "disconnect must display the unconfirmed native outcome"
    );
}

fn b5_repair_command(
    engine: &DownloadEngine,
    command: DownloadCommand,
) -> nxtools_download::DownloadCommandOutcome {
    engine
        .handle_command(DownloadCommandEnvelope {
            protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
            request_id: uuid::Uuid::new_v4(),
            origin: RequestOrigin::Internal,
            command,
        })
        .outcome
}

fn b5_repair_queue(engine: &DownloadEngine, action: nxtools_download::DownloadQueueAction) {
    assert!(matches!(
        b5_repair_command(engine, DownloadCommand::Queue { action }),
        nxtools_download::DownloadCommandOutcome::QueueChanged { .. }
    ));
}

fn b5_repair_request(
    f: &Fixture,
    server: &b4_url_fixture::UrlServer,
    name: &str,
) -> DownloadRequest {
    let mut value = request(f);
    value.source_url = server.url("/compatible");
    value.file_name = Some(name.into());
    value.expected_total_bytes = None;
    value.connection_limit = Some(1);
    value
}

fn b5_repair_complete(
    runtime: &tokio::runtime::Runtime,
    engine: &DownloadEngine,
    task_id: DownloadTaskId,
) {
    engine.resume(task_id).expect("original accepted admission");
    let completed = runtime
        .block_on(engine.wait_for_terminal(task_id, std::time::Duration::from_secs(5)))
        .expect("actual writer terminal");
    assert_eq!(completed.task.status(), DownloadStatus::Completed);
    assert_eq!(
        std::fs::read(completed.final_path.expect("published artifact")).expect("writer bytes"),
        b4_url_fixture::CONTENT
    );
}

fn b5_repair_assert_idle(f: &Fixture, cx: &mut TestAppContext) {
    assert!(
        f.application.read_with(cx, |app, _| {
            app.completion_actions.active_countdown().is_none()
                && app.shutdown_window.is_none()
                && app.pending_shutdown_command.is_none()
        }),
        "aborted original round must not authorize a countdown or native action"
    );
    while let Ok(command) = f._endpoint.try_receive_command_for_contract_test() {
        assert!(
            !matches!(
                command.message,
                nxtools_protocol::Message::PlatformCommand(
                    PlatformCommand::ShutdownSystem(_) | PlatformCommand::SuspendSystem(_)
                )
            ),
            "no typed power command from an aborted round"
        );
    }
}

fn b5_repair_coalesced_membership(cx: &mut TestAppContext, out_and_back: bool) {
    use nxtools_download::{
        DownloadCompletionAction, DownloadCompletionIntention, DownloadQueueAction,
    };
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("owned HTTP runtime");
    let server = runtime.block_on(b4_url_fixture::UrlServer::start());
    let f = fixture_with_persistence(cx, true);
    let engine = f.application.read_with(cx, |app, _| app.engine.clone());
    b5_repair_queue(
        &engine,
        DownloadQueueAction::SetCompletionIntention {
            queue_id: "default".into(),
            intention: DownloadCompletionIntention {
                action: Some(DownloadCompletionAction::Sleep),
                open_directory: false,
            },
            baseline: None,
        },
    );
    let a = engine
        .enqueue(
            b5_repair_request(&f, &server, "round-a.bin"),
            RequestOrigin::Internal,
        )
        .expect("original round A");
    cx.run_until_parked();
    b5_repair_assert_idle(&f, cx);
    let observed_sequence = f
        .application
        .read_with(cx, |app, _| app.last_event_sequence);

    // Do not drive TestApp while the original HTTP/writer and queue owners mutate.
    if out_and_back {
        for queue_id in ["secondary", "default"] {
            b5_repair_queue(
                &engine,
                DownloadQueueAction::AssignTasks {
                    queue_id: queue_id.into(),
                    task_ids: vec![a],
                },
            );
        }
    } else {
        let b = engine
            .enqueue(
                b5_repair_request(&f, &server, "removed-b.bin"),
                RequestOrigin::Internal,
            )
            .expect("accepted B");
        engine
            .remove(b, false)
            .expect("remove B before observation");
    }
    b5_repair_complete(&runtime, &engine, a);
    let store =
        nxtools_download::SqliteDownloadStore::open(f.directory.path().join("downloads.sqlite3"))
            .expect("read original SQLite");
    assert_eq!(
        store
            .load_tasks()
            .expect("durable task")
            .into_iter()
            .find(|task| task.id() == a)
            .expect("A retained")
            .status(),
        DownloadStatus::Completed
    );
    assert_eq!(
        f.application
            .read_with(cx, |app, _| app.last_event_sequence),
        observed_sequence,
        "production pump remained held"
    );
    cx.run_until_parked();
    b5_repair_assert_idle(&f, cx);

    let fresh = engine
        .enqueue(
            b5_repair_request(&f, &server, "fresh-round.bin"),
            RequestOrigin::Internal,
        )
        .expect("genuinely new round");
    b5_repair_complete(&runtime, &engine, fresh);
    cx.run_until_parked();
    let countdown = f.application.read_with(cx, |app, _| {
        app.completion_actions
            .active_countdown()
            .expect("later successful round remains eligible")
    });
    assert_eq!(countdown.action, DownloadCompletionAction::Sleep);
    cx.run_until_parked();
    assert_eq!(
        f.application.read_with(cx, |app, _| app
            .completion_actions
            .active_countdown()
            .expect("same single countdown")
            .correlation_id),
        countdown.correlation_id
    );
    b5_repair_execute_sleep_once(&f, cx, countdown.correlation_id);
    cx.run_until_parked();
    b5_repair_assert_idle(&f, cx);
    engine.shutdown_and_wait().expect("actual worker join");
    runtime.block_on(server.wait_drained());
    runtime.block_on(server.close());
}

#[gpui_kit::test]
fn b5_repair_01_coalesced_accept_remove_aborts_original_round(cx: &mut TestAppContext) {
    b5_repair_coalesced_membership(cx, false);
}

#[gpui_kit::test]
fn b5_repair_01_coalesced_out_back_aborts_original_round(cx: &mut TestAppContext) {
    b5_repair_coalesced_membership(cx, true);
}

#[gpui_kit::test]
fn b5_repair_02_post_terminal_command_cannot_arm_completed_attempt(cx: &mut TestAppContext) {
    use nxtools_download::{
        DownloadCompletionAction, DownloadCompletionIntention, DownloadTaskAction,
    };
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("owned HTTP runtime");
    let server = runtime.block_on(b4_url_fixture::UrlServer::start());
    let f = fixture_with_persistence(cx, true);
    let engine = f.application.read_with(cx, |app, _| app.engine.clone());
    let id = engine
        .enqueue(
            b5_repair_request(&f, &server, "terminal-intention.bin"),
            RequestOrigin::Internal,
        )
        .expect("unarmed task");
    cx.run_until_parked();
    let observed = f
        .application
        .read_with(cx, |app, _| app.last_event_sequence);
    b5_repair_complete(&runtime, &engine, id);
    let outcome = b5_repair_command(
        &engine,
        DownloadCommand::Task {
            task_id: id,
            action: DownloadTaskAction::SetCompletionIntention {
                intention: DownloadCompletionIntention {
                    action: Some(DownloadCompletionAction::Sleep),
                    open_directory: false,
                },
            },
        },
    );
    assert_eq!(
        f.application
            .read_with(cx, |app, _| app.last_event_sequence),
        observed,
        "terminal event still unobserved"
    );
    cx.run_until_parked();
    b5_repair_assert_idle(&f, cx);
    assert!(
        matches!(
            outcome,
            nxtools_download::DownloadCommandOutcome::Rejected { .. }
        ),
        "authoritative terminal command must reject: {outcome:?}"
    );
    let stored =
        nxtools_download::SqliteDownloadStore::open(f.directory.path().join("downloads.sqlite3"))
            .expect("original store");
    assert_eq!(
        stored
            .load_tasks()
            .expect("durable tasks")
            .into_iter()
            .find(|task| task.id() == id)
            .expect("terminal row")
            .completion_intention(),
        DownloadCompletionIntention::default()
    );

    let fresh = engine
        .enqueue(
            b5_repair_request(&f, &server, "active-intention.bin"),
            RequestOrigin::Internal,
        )
        .expect("active task");
    assert!(!matches!(
        b5_repair_command(
            &engine,
            DownloadCommand::Task {
                task_id: fresh,
                action: DownloadTaskAction::SetCompletionIntention {
                    intention: DownloadCompletionIntention {
                        action: Some(DownloadCompletionAction::Sleep),
                        open_directory: false
                    },
                },
            }
        ),
        nxtools_download::DownloadCommandOutcome::Rejected { .. }
    ));
    b5_repair_complete(&runtime, &engine, fresh);
    cx.run_until_parked();
    let countdown = f.application.read_with(cx, |app, _| {
        app.completion_actions
            .active_countdown()
            .expect("nonterminal armed success")
    });
    b5_repair_execute_sleep_once(&f, cx, countdown.correlation_id);
    cx.run_until_parked();
    b5_repair_assert_idle(&f, cx);
    engine.shutdown_and_wait().expect("actual worker join");
    runtime.block_on(server.wait_drained());
    runtime.block_on(server.close());
}

#[gpui_kit::test]
fn b5_repair_03_stopped_retry_cancels_then_succeeds_through_original_pump(cx: &mut TestAppContext) {
    use nxtools_download::{
        DownloadCompletionAction, DownloadCompletionIntention, DownloadDailySchedule,
        DownloadQueueAction, DownloadSchedule, DownloadScheduleMoment, DownloadTaskAction,
        DownloadWeekday,
    };
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("owned HTTP runtime");
    let server = runtime.block_on(b4_url_fixture::UrlServer::start());
    let f = fixture_with_persistence(cx, true);
    let engine = f.application.read_with(cx, |app, _| app.engine.clone());
    let mut failed_request = b5_repair_request(&f, &server, "retry-waiting.bin");
    failed_request.source_url = server.url("/failed");
    let retry = engine
        .enqueue(failed_request, RequestOrigin::Internal)
        .expect("original failed task");
    assert!(!matches!(
        b5_repair_command(
            &engine,
            DownloadCommand::Task {
                task_id: retry,
                action: DownloadTaskAction::SetCompletionIntention {
                    intention: DownloadCompletionIntention {
                        action: Some(DownloadCompletionAction::Sleep),
                        open_directory: false
                    },
                },
            }
        ),
        nxtools_download::DownloadCommandOutcome::Rejected { .. }
    ));
    engine.resume(retry).expect("first admission");
    let failed = runtime
        .block_on(engine.wait_for_terminal(retry, std::time::Duration::from_secs(5)))
        .expect("actual HTTP failure");
    assert_eq!(failed.task.status(), DownloadStatus::Failed);
    let attempt = failed.task.completion_attempt();
    cx.run_until_parked();
    engine
        .set_queue_schedule(
            "default",
            Some(DownloadSchedule {
                daily: DownloadDailySchedule {
                    days: DownloadWeekday::ALL.to_vec(),
                    start_minute: 600,
                    stop_minute: 660,
                    enabled_start: true,
                    enabled_stop: true,
                },
            }),
        )
        .expect("original queue schedule");
    engine
        .process_daily_schedules_at(DownloadScheduleMoment {
            weekday: DownloadWeekday::Monday,
            minute_of_day: 720,
        })
        .expect("original stopped queue boundary");

    let successful = engine
        .enqueue(
            b5_repair_request(&f, &server, "old-countdown.bin"),
            RequestOrigin::Internal,
        )
        .expect("independent accepted task");
    b5_repair_queue(
        &engine,
        DownloadQueueAction::AssignTasks {
            queue_id: "secondary".into(),
            task_ids: vec![successful],
        },
    );
    assert!(!matches!(
        b5_repair_command(
            &engine,
            DownloadCommand::Task {
                task_id: successful,
                action: DownloadTaskAction::SetCompletionIntention {
                    intention: DownloadCompletionIntention {
                        action: Some(DownloadCompletionAction::Sleep),
                        open_directory: false
                    },
                },
            }
        ),
        nxtools_download::DownloadCommandOutcome::Rejected { .. }
    ));
    b5_repair_complete(&runtime, &engine, successful);
    cx.run_until_parked();
    let old = f.application.read_with(cx, |app, _| {
        app.completion_actions
            .active_countdown()
            .expect("legitimate original countdown")
    });
    let request_count = server.requests.lock().expect("HTTP observations").len();
    let accepted = b5_repair_command(
        &engine,
        DownloadCommand::Task {
            task_id: retry,
            action: DownloadTaskAction::Retry,
        },
    );
    assert!(
        !matches!(
            accepted,
            nxtools_download::DownloadCommandOutcome::Rejected { .. }
        ),
        "durable retry accepted: {accepted:?}"
    );
    cx.run_until_parked();
    b5_repair_assert_idle(&f, cx);
    let waiting = engine
        .task(retry)
        .expect("original task read")
        .expect("same retained task");
    assert_eq!(waiting.task.completion_attempt(), attempt + 1);
    assert!(waiting.queue_eligible);
    assert_eq!(
        engine
            .completion_snapshot()
            .expect("authoritative waiting snapshot")
            .tasks
            .into_iter()
            .find(|task| task.task_id == retry)
            .expect("waiting attempt fact")
            .status,
        DownloadStatus::Waiting,
        "accepted new attempt must not remain a Failed terminal"
    );
    assert_eq!(
        server.requests.lock().expect("HTTP observations").len(),
        request_count,
        "stopped queue must not start any worker"
    );
    let stored =
        nxtools_download::SqliteDownloadStore::open(f.directory.path().join("downloads.sqlite3"))
            .expect("original store");
    assert!(
        !stored
            .load_queues()
            .expect("queues")
            .into_iter()
            .find(|queue| queue.id == "default")
            .expect("default queue")
            .is_running
    );
    let revision = engine
        .completion_snapshot()
        .expect("accepted snapshot")
        .work_revision;
    let duplicate = b5_repair_command(
        &engine,
        DownloadCommand::Task {
            task_id: retry,
            action: DownloadTaskAction::Retry,
        },
    );
    assert!(!matches!(
        duplicate,
        nxtools_download::DownloadCommandOutcome::Rejected { .. }
    ));
    assert_eq!(
        engine
            .completion_snapshot()
            .expect("duplicate snapshot")
            .work_revision,
        revision
    );
    assert_eq!(
        engine
            .task(retry)
            .expect("same task")
            .expect("retained")
            .task
            .completion_attempt(),
        attempt + 1
    );

    runtime.block_on(async {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let mut recovery = tokio::net::TcpStream::connect(server.address)
            .await
            .expect("owned recovery connection");
        recovery
            .write_all(
                b"GET /recover-failed HTTP/1.1\r\nHost: fixture\r\nConnection: close\r\n\r\n",
            )
            .await
            .expect("request fixture recovery");
        let mut response = Vec::new();
        tokio::time::timeout(
            std::time::Duration::from_secs(3),
            recovery.read_to_end(&mut response),
        )
        .await
        .expect("bounded fixture recovery")
        .expect("recovery response");
        assert!(response.starts_with(b"HTTP/1.1 204 No Content\r\n"));
    });
    engine
        .process_daily_schedules_at(DownloadScheduleMoment {
            weekday: DownloadWeekday::Monday,
            minute_of_day: 630,
        })
        .expect("original queue start boundary");
    let completed = runtime
        .block_on(engine.wait_for_terminal(retry, std::time::Duration::from_secs(5)))
        .expect("real retry writer completion");
    assert_eq!(completed.task.status(), DownloadStatus::Completed);
    assert_eq!(completed.task.completion_attempt(), attempt + 1);
    assert_eq!(
        std::fs::read(completed.final_path.expect("retry output")).expect("actual retry bytes"),
        b4_url_fixture::CONTENT
    );
    cx.run_until_parked();
    let fresh = f.application.read_with(cx, |app, _| {
        app.completion_actions
            .active_countdown()
            .expect("new attempt retains successful authority")
    });
    assert_ne!(old.correlation_id, fresh.correlation_id);
    f.application.update(cx, |app, cx| {
        app.execute_shutdown(old.correlation_id, "late", cx);
        app.cancel_shutdown_countdown(old.correlation_id, "late", cx);
    });
    assert_eq!(
        f.application.read_with(cx, |app, _| app
            .completion_actions
            .active_countdown()
            .expect("fresh authority retained")
            .correlation_id),
        fresh.correlation_id
    );
    assert!(
        f.application
            .read_with(cx, |app, _| app.pending_shutdown_command.is_none())
    );
    b5_repair_execute_sleep_once(&f, cx, fresh.correlation_id);
    cx.run_until_parked();
    b5_repair_assert_idle(&f, cx);
    engine.shutdown_and_wait().expect("actual worker join");
    runtime.block_on(server.wait_drained());
    runtime.block_on(server.close());
}

fn b5_repair_execute_sleep_once(f: &Fixture, cx: &mut TestAppContext, correlation: uuid::Uuid) {
    f._endpoint.set_connected_for_contract_test(true);
    f.application.update(cx, |app, cx| {
        app.execute_shutdown(correlation, "test", cx);
        app.execute_shutdown(correlation, "duplicate", cx);
    });
    let mut native = None;
    while let Ok(command) = f._endpoint.try_receive_command_for_contract_test() {
        if matches!(
            command.message,
            nxtools_protocol::Message::PlatformCommand(PlatformCommand::SuspendSystem(
                SuspendSystemCommand { hibernate: false }
            ))
        ) {
            assert!(
                native.replace(command).is_none(),
                "exactly one controlled Sleep request"
            );
        }
    }
    let command = native.expect("original typed Sleep boundary");
    assert_eq!(command.correlation_id, Some(correlation));
    f.application.update(cx, |app, cx| {
        app.handle_platform_notification(
            PlatformNotification::CommandResult {
                correlation_id: command.id,
                result: PlatformCommandResult {
                    success: false,
                    error: Some("fixture unavailable".into()),
                    native_error_code: Some(50),
                    data: None,
                },
            },
            cx,
        );
        app.execute_shutdown(correlation, "late", cx);
        app.cancel_shutdown_countdown(correlation, "late", cx);
    });
    cx.run_until_parked();
    b5_repair_assert_idle(f, cx);
}

#[gpui_kit::test]
fn user_feedback_default_solid_settings_navigation_preserves_visible_regions(
    cx: &mut TestAppContext,
) {
    let f = fixture(cx);
    let saved =
        std::fs::read(f.directory.path().join("config.json")).expect("owned settings baseline");
    let (view, mut surface) = settings(&f, cx);
    let directory_input = view.read_with(&surface, |view, _| view.directory.entity_id());
    for page in [
        DownloadSettingsPage::General,
        DownloadSettingsPage::Request,
        DownloadSettingsPage::Transfer,
        DownloadSettingsPage::Window,
        DownloadSettingsPage::Diagnostics,
    ] {
        draw(&mut surface);
        let id =
            gpui_kit::ElementId::from(gpui_kit::SharedString::from(format!("0-{}", page.index())));
        let painted = surface.update(|window, cx| {
            let node = gpui_kit::base::test_support::snapshots(window)
                .into_iter()
                .find(|node| node.path().last() == Some(&id))
                .expect("original Settings navigation item");
            let foreground = ComponentTheme::global(cx).sidebar_foreground;
            assert!(
                node.visible(),
                "default Solid navigation must be inside its clip: {page:?} {:?}",
                node.bounds()
            );
            assert!(
                foreground.a > 0.0,
                "default Solid navigation foreground must remain opaque"
            );
            node.bounds()
        });
        assert!(painted.size.width > px(0.0) && painted.size.height >= px(12.0));
        let viewport = surface.update(|window, _| window.fully_visible_bounds());
        assert!(
            viewport.contains(&painted.center()),
            "navigation is clipped: {page:?} {painted:?}"
        );
        assert!(
            painted.right() <= px(168.0),
            "navigation stays in the original rail: {painted:?}"
        );
    }
    assert_eq!(
        view.read_with(&surface, |view, _| view.directory.entity_id()),
        directory_input
    );
    assert_eq!(
        std::fs::read(f.directory.path().join("config.json"))
            .expect("owned settings after navigation"),
        saved
    );
}
