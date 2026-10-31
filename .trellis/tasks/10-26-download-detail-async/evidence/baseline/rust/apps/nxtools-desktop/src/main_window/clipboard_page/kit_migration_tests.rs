//! Queue06 production Settings regressions; temporary SQLite/config and fake transport only.
use super::*;
use gpui_kit::base::{Root, test_support};
use gpui_kit::{Modifiers, TestAppContext, VisualTestContext, size};
use nxtools_core::AppConfig;
use nxtools_ipc::{PlatformServerEndpoint, platform_bridge};

struct Fixture {
    main: Entity<MainWindow>,
    application: Entity<ClipboardApplication>,
    window: AnyWindowHandle,
    directory: tempfile::TempDir,
    worker: crate::clipboard_worker::ClipboardCaptureWorkerHandle,
    endpoint: PlatformServerEndpoint,
    notifications: Entity<NotificationCenter>,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.worker
            .clone()
            .shutdown()
            .expect("isolated worker shutdown");
    }
}
fn fixture(cx: &mut TestAppContext) -> Fixture {
    fixture_in(cx, "")
}
fn fixture_in(cx: &mut TestAppContext, folder: &str) -> Fixture {
    cx.update(gpui_kit::init);
    cx.update(crate::theme::apply_component_theme);
    cx.executor().allow_parking();
    let directory = tempfile::tempdir().expect("isolated Clipboard Settings");
    let config = AppConfig::default();
    let config_path = directory.path().join("config.json");
    config.save_to(&config_path).expect("seed temp config");
    let storage = directory.path().join(folder);
    let worker = crate::clipboard_worker::ClipboardCaptureWorkerHandle::spawn(
        storage.join("clipboard.db"),
        storage.join("images"),
        nxtools_clipboard::ClipboardRetentionPolicy {
            max_items: 1000,
            retention_days: 30,
        },
        1000,
    )
    .expect("temporary SQLite worker");
    let (platform, endpoint) = platform_bridge(32);
    let notifications = cx.update(|cx| cx.new(|_| NotificationCenter::default()));
    let application = cx.update(|cx| {
        cx.new(|cx| {
            ClipboardApplication::new(
                worker.clone(),
                Rc::new(platform),
                config,
                config_path,
                notifications.clone(),
                cx,
            )
        })
    });
    let retained = Rc::new(RefCell::new(None));
    let slot = retained.clone();
    let app = application.downgrade();
    let notices = notifications.clone();
    let (_root, surface) = cx.add_window_view(move |window, cx| {
        let main = cx.new(|cx| {
            let mut main = MainWindow::new(
                app,
                gpui_kit::WeakEntity::new_invalid(),
                gpui_kit::WeakEntity::new_invalid(),
                MainWindowShellContext {
                    settings: None,
                    platform: None,
                    system_settings: Rc::new(RefCell::new(SystemSettings::default())),
                },
                notices,
                window,
                cx,
            );
            main.shell.activate(MainModule::Clipboard);
            main
        });
        *slot.borrow_mut() = Some(main.clone());
        Root::new(main, window, cx)
    });
    surface.simulate_resize(size(px(920.0), px(480.0)));
    surface.run_until_parked();
    surface.refresh().expect("production Clipboard settings");
    let window = surface.update(|window, _| window.window_handle());
    Fixture {
        main: retained.borrow().clone().expect("MainWindow"),
        application,
        window,
        directory,
        worker,
        endpoint,
        notifications,
    }
}
fn search(surface: &mut VisualTestContext, query: &str) {
    let input = surface.update(|window, _| {
        test_support::snapshots(window)
            .into_iter()
            .find(|node| {
                node.role() == Some(gpui_kit::Role::TextInput)
                    && node
                        .path()
                        .contains(&gpui_kit::ElementId::from("settings-sidebar"))
            })
            .expect("real official Settings search")
            .bounds()
    });
    surface.update(|window, _| window.activate_window());
    surface.run_until_parked();
    surface.simulate_click(input.center(), Modifiers::default());
    surface.simulate_keystrokes("ctrl-a");
    if query.is_empty() {
        surface.simulate_keystrokes("backspace");
    } else {
        surface.simulate_input(query);
    }
    surface.run_until_parked();
    surface.refresh().expect("official search selection/filter");
}
#[gpui_kit::test]
fn queue06_production_mounts_official_settings_search(cx: &mut TestAppContext) {
    let fixture = fixture(cx);
    let mut surface = VisualTestContext::from_window(fixture.window, cx);
    surface.refresh().expect("current production frame");
    let nodes = surface.update(|window, _| test_support::snapshots(window));
    assert!(
        nodes
            .iter()
            .any(|node| node.role() == Some(gpui_kit::Role::TextInput)
                && node
                    .path()
                    .contains(&gpui_kit::ElementId::from("settings-sidebar"))),
        "actual Settings search, not old workbench page"
    );
    assert_eq!(
        fixture
            .main
            .read_with(&surface, |main, _| main.clipboard_overview.total),
        0
    );
}
#[gpui_kit::test]
fn queue06_retention_step_uses_official_small_button(cx: &mut TestAppContext) {
    let fixture = fixture(cx);
    let mut surface = VisualTestContext::from_window(fixture.window, cx);
    search(&mut surface, "最大普通记录");
    let nodes = surface.update(|window, _| test_support::snapshots(window));
    let minus = nodes
        .iter()
        .find(|node| {
            node.path().last() == Some(&gpui_kit::ElementId::from("clipboard-max-items-minus"))
        })
        .expect("actual minus Button");
    assert_eq!(
        minus.bounds().size.height,
        px(24.0),
        "released Small Button; not painted32px stepper"
    );
    surface.simulate_click(minus.bounds().center(), Modifiers::default());
    surface.run_until_parked();
    surface.refresh().expect("pointer step on original owner");
    assert_eq!(
        fixture
            .application
            .read_with(&surface, |app, _| app.clipboard_settings().max_items),
        900
    );
    let focused_minus = |surface: &mut VisualTestContext| {
        surface.update(|window, _| {
            test_support::snapshots(window).iter().any(|node| {
                node.path().last() == Some(&gpui_kit::ElementId::from("clipboard-max-items-minus"))
                    && node.focused() == Some(true)
            })
        })
    };
    for _ in 0..20 {
        if focused_minus(&mut surface) {
            break;
        }
        surface.simulate_keystrokes("tab");
        surface.run_until_parked();
        surface.refresh().expect("actual step focus");
    }
    assert!(
        focused_minus(&mut surface),
        "keyboard must focus actual minus Button before release"
    );
    surface.simulate_event(gpui_kit::KeyDownEvent {
        keystroke: gpui_kit::Keystroke::parse("space").expect("step Space"),
        is_held: false,
        prefer_character_input: false,
    });
    surface.simulate_event(gpui_kit::KeyUpEvent {
        keystroke: gpui_kit::Keystroke::parse("space").expect("step release"),
    });
    surface.run_until_parked();
    assert_eq!(
        AppConfig::load_from(&fixture.directory.path().join("config.json"))
            .expect("keyboard section save")
            .clipboard
            .max_items,
        800,
        "one focused key release dispatches exactly once"
    );
}

fn button_bounds(surface: &mut VisualTestContext, id: &str) -> gpui_kit::Bounds<gpui_kit::Pixels> {
    surface.update(|window, _| {
        test_support::snapshots(window)
            .into_iter()
            .find(|node| node.path().last() == Some(&gpui_kit::ElementId::from(id.to_owned())))
            .expect("mounted real Button")
            .bounds()
    })
}
fn terminal(fixture: &Fixture, id: uuid::Uuid, success: bool, cx: &mut TestAppContext) {
    fixture.application.update(cx, |app, cx| {
        app.handle_platform_notification(
            nxtools_ipc::PlatformNotification::CommandResult {
                correlation_id: id,
                result: nxtools_protocol::PlatformCommandResult {
                    success,
                    error: (!success).then(|| "合成选择器失败".into()),
                    native_error_code: None,
                    data: None,
                },
            },
            cx,
        )
    });
    cx.run_until_parked();
}
#[gpui_kit::test]
fn queue06_original_steps_preserve_section_disk_and_worker_root(cx: &mut TestAppContext) {
    let fixture = fixture(cx);
    fixture.endpoint.set_connected_for_contract_test(true);
    let mut surface = VisualTestContext::from_window(fixture.window, cx);
    let original_root = fixture.worker.database_path();
    for (query, id) in [
        ("最大普通记录", "clipboard-max-items-plus"),
        ("自动清理天数", "clipboard-retention-days-plus"),
        ("文本大小上限", "clipboard-max-text-bytes-plus"),
        ("图片文件上限", "clipboard-max-image-bytes-plus"),
        ("图片像素上限", "clipboard-max-image-pixels-plus"),
    ] {
        search(&mut surface, query);
        let bounds = button_bounds(&mut surface, id);
        surface.simulate_click(bounds.center(), Modifiers::default());
        surface.run_until_parked();
        surface.refresh().expect("deferred step persisted");
    }
    let saved = AppConfig::load_from(&fixture.directory.path().join("config.json"))
        .expect("real section disk");
    let default = AppConfig::default();
    assert_eq!(saved.clipboard.max_items, default.clipboard.max_items + 100);
    assert_eq!(
        saved.clipboard.retention_days,
        default.clipboard.retention_days + 1
    );
    assert_eq!(
        saved.clipboard.max_text_bytes,
        default.clipboard.max_text_bytes + 64 * 1024
    );
    assert_eq!(
        saved.clipboard.max_image_bytes,
        default.clipboard.max_image_bytes + 1024 * 1024
    );
    assert_eq!(
        saved.clipboard.max_image_pixels,
        default.clipboard.max_image_pixels + 1_000_000
    );
    assert_eq!(
        saved.clipboard.storage_directory,
        default.clipboard.storage_directory
    );
    assert_eq!(fixture.worker.database_path(), original_root);
    assert_eq!(saved.quick_launch, default.quick_launch);
    for _ in 0..3 {
        let envelope = fixture
            .endpoint
            .try_receive_command_for_contract_test()
            .expect("one ConfigureClipboard per capture step");
        assert!(matches!(
            envelope.message,
            nxtools_protocol::Message::PlatformCommand(PlatformCommand::ConfigureClipboard(_))
        ));
    }
    assert!(
        fixture
            .endpoint
            .try_receive_command_for_contract_test()
            .is_err()
    );
}
#[gpui_kit::test]
fn queue06_modifier_and_hotkey_cancel_keep_original_owner(cx: &mut TestAppContext) {
    let fixture = fixture(cx);
    fixture.endpoint.set_connected_for_contract_test(true);
    let mut surface = VisualTestContext::from_window(fixture.window, cx);
    search(&mut surface, "Ctrl + 单击");
    let bounds = button_bounds(&mut surface, "clipboard-modifier-control");
    surface.simulate_click(bounds.center(), Modifiers::default());
    surface.run_until_parked();
    surface.refresh().expect("actual official menu");
    let copy = surface.update(|window, _| {
        test_support::snapshots(window)
            .into_iter()
            .find(|node| node.label() == Some("仅复制"))
            .expect("current CopyOnly menu item")
            .bounds()
    });
    surface.simulate_click(copy.center(), Modifiers::default());
    surface.run_until_parked();
    let saved =
        AppConfig::load_from(&fixture.directory.path().join("config.json")).expect("modifier disk");
    assert_eq!(
        saved.clipboard.modifier_actions.control,
        ClipboardClickAction::CopyOnly
    );
    search(&mut surface, "热键");
    let bounds = button_bounds(&mut surface, "record-clipboard-hotkey");
    surface.simulate_click(bounds.center(), Modifiers::default());
    surface.run_until_parked();
    let envelope = fixture
        .endpoint
        .try_receive_command_for_contract_test()
        .expect("original recording command");
    assert!(matches!(
        envelope.message,
        nxtools_protocol::Message::PlatformCommand(PlatformCommand::SetClipboardHotKeyRecording(_))
    ));
    terminal(&fixture, envelope.id, true, cx);
    surface.simulate_keystrokes("escape");
    surface.run_until_parked();
    let envelope = fixture
        .endpoint
        .try_receive_command_for_contract_test()
        .expect("restore after Escape");
    assert!(matches!(
        envelope.message,
        nxtools_protocol::Message::PlatformCommand(PlatformCommand::SetClipboardHotKeyRecording(
            nxtools_protocol::SetClipboardHotKeyRecordingCommand { recording: false }
        ))
    ));
    terminal(&fixture, envelope.id, true, cx);
    assert!(
        fixture
            .main
            .read_with(cx, |main, _| main.recording_hotkey.is_none())
    );
    assert_eq!(
        AppConfig::load_from(&fixture.directory.path().join("config.json"))
            .expect("cancel disk")
            .clipboard
            .hotkey,
        saved.clipboard.hotkey
    );
}
#[gpui_kit::test]
fn queue06_storage_actions_busy_cancel_and_error_never_run_data_operations(
    cx: &mut TestAppContext,
) {
    let fixture = fixture(cx);
    fixture.endpoint.set_connected_for_contract_test(true);
    let mut surface = VisualTestContext::from_window(fixture.window, cx);
    search(&mut surface, "归档");
    let export = button_bounds(&mut surface, "clipboard-export-archive");
    surface.simulate_click(export.center(), Modifiers::default());
    surface.run_until_parked();
    let first = fixture
        .endpoint
        .try_receive_command_for_contract_test()
        .expect("fake export picker");
    assert!(matches!(
        first.message,
        nxtools_protocol::Message::PlatformCommand(PlatformCommand::PickClipboardArchivePath(_))
    ));
    assert!(
        fixture
            .application
            .read_with(cx, |app, _| app.clipboard_management_busy())
    );
    search(&mut surface, "数据库维护");
    for id in ["clipboard-database-backup", "clipboard-database-compact"] {
        let bounds = button_bounds(&mut surface, id);
        surface.simulate_click(bounds.center(), Modifiers::default());
    }
    surface.run_until_parked();
    assert!(
        fixture
            .endpoint
            .try_receive_command_for_contract_test()
            .is_err()
    );
    assert!(
        !fixture.directory.path().join("backups").exists(),
        "busy backup/compact cannot reach worker"
    );
    terminal(&fixture, first.id, true, cx); // data=None is native picker cancellation, never worker import/export.
    assert!(
        !fixture
            .application
            .read_with(cx, |app, _| app.clipboard_management_busy())
    );
    search(&mut surface, "历史归档");
    let import = button_bounds(&mut surface, "clipboard-import-archive");
    surface.simulate_click(import.center(), Modifiers::default());
    surface.run_until_parked();
    let next = fixture
        .endpoint
        .try_receive_command_for_contract_test()
        .expect("fake import picker");
    terminal(&fixture, next.id, false, cx);
    assert!(fixture.notifications.read_with(cx, |center, _| {
        center
            .items()
            .iter()
            .any(|item| item.level == NotificationLevel::Error)
    }));
    search(&mut surface, "存储位置");
    let migrate = button_bounds(&mut surface, "clipboard-storage-migrate");
    surface.simulate_click(migrate.center(), Modifiers::default());
    surface.run_until_parked();
    let next = fixture
        .endpoint
        .try_receive_command_for_contract_test()
        .expect("fake directory picker");
    assert!(matches!(
        next.message,
        nxtools_protocol::Message::PlatformCommand(PlatformCommand::PickClipboardStorageDirectory(
            _
        ))
    ));
    terminal(&fixture, next.id, true, cx);
    assert!(
        !fixture
            .application
            .read_with(cx, |app, _| app.clipboard_management_busy())
    );
    assert_eq!(
        fixture.worker.database_path(),
        fixture.directory.path().join("clipboard.db")
    );
    assert!(!fixture.directory.path().join("backups").exists());
}
#[gpui_kit::test]
fn queue06_minimum_scroll_reaches_storage_footer_and_keeps_notifications(cx: &mut TestAppContext) {
    let fixture = fixture(cx);
    let mut surface = VisualTestContext::from_window(fixture.window, cx);
    search(&mut surface, "数据库维护");
    search(&mut surface, "");
    for _ in 0..12 {
        surface.simulate_event(gpui_kit::ScrollWheelEvent {
            position: point(px(750.0), px(320.0)),
            delta: gpui_kit::ScrollDelta::Pixels(point(px(0.0), px(-80.0))),
            modifiers: Modifiers::default(),
            touch_phase: gpui_kit::TouchPhase::Moved,
        });
        surface.run_until_parked();
        surface.refresh().expect("official page scroll");
    }
    let migrate = button_bounds(&mut surface, "clipboard-storage-migrate");
    assert!(
        migrate.left() >= px(176.0)
            && migrate.right() <= px(920.0)
            && migrate.top() >= px(0.0)
            && migrate.bottom() <= px(480.0),
        "last original operation reachable at minimum: {migrate:?}"
    );
    fixture.notifications.update(cx, |center, cx| {
        center.show_progress("queue06-progress", "存储迁移", "合成进度，未执行迁移", cx)
    });
    surface.run_until_parked();
    surface.refresh().expect("original notification progress");
    assert!(
        surface
            .debug_bounds("notification-queue06-progress")
            .is_some()
    );
}

#[gpui_kit::test]
fn queue06_search_filters_without_saving_and_optional_help_is_real_tooltip(
    cx: &mut TestAppContext,
) {
    let fixture = fixture(cx);
    let mut surface = VisualTestContext::from_window(fixture.window, cx);
    let before =
        std::fs::read(fixture.directory.path().join("config.json")).expect("temp config bytes");
    search(&mut surface, "Ctrl + 单击");
    let help = surface
        .debug_bounds("clipboard-setting-1-help")
        .expect("optional help trigger");
    surface.simulate_mouse_move(help.center(), None, Modifiers::default());
    cx.executor().advance_clock(Duration::from_millis(700));
    surface.run_until_parked();
    surface.refresh().expect("real optional Tooltip");
    assert!(surface.update(|window, _| {
        test_support::snapshots(window).iter().any(|node| {
            node.path().contains(&gpui_kit::ElementId::from(
                "clipboard-setting-1-help-tooltip",
            ))
        })
    }));
    search(&mut surface, "没有匹配的合成设置");
    let nodes = surface.update(|window, _| test_support::snapshots(window));
    assert!(!nodes.iter().any(|node| {
        [
            "record-clipboard-hotkey",
            "clipboard-modifier-control",
            "clipboard-storage-migrate",
        ]
        .iter()
        .any(|id| node.path().last() == Some(&gpui_kit::ElementId::from(*id)))
    }));
    assert_eq!(
        std::fs::read(fixture.directory.path().join("config.json")).expect("unchanged disk"),
        before
    );
    assert!(
        fixture
            .endpoint
            .try_receive_command_for_contract_test()
            .is_err()
    );
}

#[gpui_kit::test]
fn queue06_long_storage_path_stays_bounded_and_footer_reachable(cx: &mut TestAppContext) {
    let fixture = fixture_in(cx, &"合成目录abcdefgh\\".repeat(5));
    let mut surface = VisualTestContext::from_window(fixture.window, cx);
    search(&mut surface, "存储位置");
    let path = surface
        .debug_bounds("clipboard-storage-path")
        .expect("actual long worker path");
    let migrate = button_bounds(&mut surface, "clipboard-storage-migrate");
    assert!(
        path.left() >= px(176.0) && path.right() <= px(920.0),
        "bounded path: {path:?}"
    );
    assert!(
        path.bottom() <= migrate.top() && migrate.bottom() <= px(480.0),
        "path must not push original action out: path={path:?} migrate={migrate:?}"
    );
    assert!(
        fixture
            .endpoint
            .try_receive_command_for_contract_test()
            .is_err()
    );
}

#[gpui_kit::test]
fn queue06_long_registration_error_keeps_original_status_and_last_action(cx: &mut TestAppContext) {
    let fixture = fixture(cx);
    fixture.endpoint.set_connected_for_contract_test(true);
    let mut surface = VisualTestContext::from_window(fixture.window, cx);
    search(&mut surface, "热键");
    let hotkey = button_bounds(&mut surface, "record-clipboard-hotkey");
    surface.simulate_click(hotkey.center(), Modifiers::default());
    surface.run_until_parked();
    let command = fixture
        .endpoint
        .try_receive_command_for_contract_test()
        .expect("recording request");
    fixture.application.update(cx, |app, cx| {
        app.handle_platform_notification(
            nxtools_ipc::PlatformNotification::CommandResult {
                correlation_id: command.id,
                result: nxtools_protocol::PlatformCommandResult {
                    success: false,
                    error: Some("合成 Windows 热键暂停失败，请检查注册状态。".repeat(12)),
                    native_error_code: Some(5),
                    data: None,
                },
            },
            cx,
        )
    });
    surface.run_until_parked();
    surface
        .refresh()
        .expect("visible original long registration status");
    assert!(fixture.application.read_with(&surface, |app, _| {
        app.clipboard_configuration_status()
            .starts_with("热键录制失败")
    }));
    let status = surface
        .debug_bounds("clipboard-setting-0")
        .expect("actual Form with visible status");
    assert!(
        status.left() >= px(176.0) && status.right() <= px(920.0) && status.size.height > px(60.0)
    );
    search(&mut surface, "");
    for _ in 0..12 {
        surface.simulate_event(gpui_kit::ScrollWheelEvent {
            position: point(px(750.0), px(320.0)),
            delta: gpui_kit::ScrollDelta::Pixels(point(px(0.0), px(-80.0))),
            modifiers: Modifiers::default(),
            touch_phase: gpui_kit::TouchPhase::Moved,
        });
        surface.run_until_parked();
        surface.refresh().expect("official error-page scroll");
    }
    let last = button_bounds(&mut surface, "clipboard-modifier-shift");
    assert!(
        last.left() >= px(176.0)
            && last.right() <= px(920.0)
            && last.top() >= px(0.0)
            && last.bottom() <= px(480.0),
        "long status keeps final action reachable: {last:?}"
    );
    assert_eq!(
        AppConfig::load_from(&fixture.directory.path().join("config.json"))
            .expect("failure disk")
            .clipboard
            .hotkey,
        AppConfig::default().clipboard.hotkey
    );
}

#[gpui_kit::test]
fn clipboard_hotkey_recorder_is_right_aligned_beside_title(cx: &mut TestAppContext) {
    let fixture = fixture(cx);
    let mut surface = VisualTestContext::from_window(fixture.window, cx);
    for width in [920.0, 1060.0] {
        surface.simulate_resize(size(px(width), px(650.0)));
        surface.refresh().expect("actual Clipboard settings");
        let row = surface
            .debug_bounds("clipboard-setting-0")
            .expect("hotkey setting width");
        let recorder = button_bounds(&mut surface, "record-clipboard-hotkey");
        let title = surface
            .debug_bounds("clipboard-hotkey-row-title")
            .expect("hotkey title");
        assert!(
            (recorder.center().y - title.center().y).abs() <= px(2.0),
            "recorder and title share center"
        );
        assert!(
            (recorder.right() - row.right()).abs() <= px(2.0),
            "recorder at right edge"
        );
        assert!(
            (recorder.top() - row.top()).abs() <= px(2.0),
            "hotkey editor must be in the first title row, not stacked below: {row:?} {recorder:?}"
        );
        let state = surface
            .debug_bounds("clipboard-hotkey-registration-status")
            .expect("visible registration state");
        assert!(
            state.top() >= recorder.bottom(),
            "registration state remains below, not removed"
        );
    }
}

#[gpui_kit::test]
fn image_modes_settings_save_roundtrip_and_failed_commit_keeps_live_mode(cx: &mut TestAppContext) {
    let f = fixture(cx);
    let mut surface = VisualTestContext::from_window(f.window, cx);
    search(&mut surface, "视图模式");
    let nodes = surface.update(|window, _| test_support::snapshots(window));
    let control = nodes
        .iter()
        .find(|node| node.label() == Some("视图模式: 自动"))
        .expect("official Settings selector");
    assert!(control.bounds().size.height >= px(32.));
    let path = f.directory.path().join("config.json");
    AppConfig::update_section(&path, |config| {
        config.clipboard.storage_directory = "latest-root".into();
        config.download.default_connection_limit = 24;
    })
    .expect("other owners");
    for mode in nxtools_core::ClipboardImageViewMode::ALL {
        f.application.update(&mut surface, |owner, cx| {
            owner.set_image_view_mode(mode, cx)
        });
        let config = AppConfig::load_from(&path).expect("durable choice");
        assert_eq!(config.clipboard.image_view_mode, mode);
        assert_eq!(config.clipboard.storage_directory, "latest-root");
        assert_eq!(config.download.default_connection_limit, 24);
        assert!(
            f.endpoint.try_receive_command_for_contract_test().is_err(),
            "view mode alone does not configure capture/native services"
        );
    }
    std::fs::write(&path, b"{broken").expect("failed commit fixture");
    f.application.update(&mut surface, |owner, cx| {
        owner.set_image_view_mode(nxtools_core::ClipboardImageViewMode::Fit, cx)
    });
    assert_eq!(
        f.application.read_with(&surface, |owner, _| owner
            .clipboard_settings()
            .image_view_mode),
        nxtools_core::ClipboardImageViewMode::Compact
    );
    assert_eq!(std::fs::read(&path).expect("no overwrite"), b"{broken");
}

#[gpui_kit::test]
fn text_card_height_official_input_persists_and_failed_commit_retains_live(
    cx: &mut TestAppContext,
) {
    let f = fixture(cx);
    let mut surface = VisualTestContext::from_window(f.window, cx);
    search(&mut surface, "文本卡最大高度");
    let input = surface.update(|window, _| {
        let nodes = test_support::snapshots(window);
        nodes
            .iter()
            .find(|node| node.label() == Some("文本卡最大高度"))
            .unwrap_or_else(|| {
                panic!(
                    "official NumberInput: {:?}",
                    nodes
                        .iter()
                        .map(|node| (node.label(), node.role()))
                        .collect::<Vec<_>>()
                )
            })
            .bounds()
    });
    assert_eq!(
        input.size.height,
        px(30.0),
        "official32DIP frame has1DIPborder each edge"
    );
    let path = f.directory.path().join("config.json");
    AppConfig::update_section(&path, |config| {
        config.clipboard.storage_directory = "latest-root".into();
        config.download.default_connection_limit = 24;
    })
    .expect("other owners");
    surface.simulate_click(input.center(), Modifiers::default());
    surface.simulate_keystrokes("ctrl-a");
    surface.simulate_input("210");
    surface.run_until_parked();
    assert_eq!(
        AppConfig::load_from(&path)
            .expect("real input save")
            .clipboard
            .text_card_max_height,
        210
    );
    for height in [120, 160, 210, 320] {
        f.application.update(&mut surface, |owner, cx| {
            owner.set_text_card_max_height(height, cx)
        });
        surface.run_until_parked();
        let saved = AppConfig::load_from(&path).expect("durable height");
        assert_eq!(saved.clipboard.text_card_max_height, height);
        assert_eq!(saved.clipboard.storage_directory, "latest-root");
        assert_eq!(saved.download.default_connection_limit, 24);
        assert!(
            f.endpoint.try_receive_command_for_contract_test().is_err(),
            "presentation has no native capture side effects"
        );
    }
    std::fs::write(&path, b"{broken").expect("failed commit");
    f.application.update(&mut surface, |owner, cx| {
        owner.set_text_card_max_height(160, cx)
    });
    surface.run_until_parked();
    assert_eq!(
        f.application.read_with(&surface, |owner, _| owner
            .clipboard_settings()
            .text_card_max_height),
        320
    );
    assert_eq!(std::fs::read(&path).expect("unchanged"), b"{broken");
}

#[gpui_kit::test]
fn audit08_clipboard_command_labels_match_picker_or_immediate_result(cx: &mut TestAppContext) {
    let fixture = fixture(cx);
    let mut surface = VisualTestContext::from_window(fixture.window, cx);
    for (query, expected) in [
        (
            "历史归档",
            vec![
                ("clipboard-export-archive", "导出…"),
                ("clipboard-import-archive", "导入…"),
            ],
        ),
        (
            "数据库维护",
            vec![
                ("clipboard-database-backup", "备份"),
                ("clipboard-database-compact", "压缩"),
            ],
        ),
        ("存储位置", vec![("clipboard-storage-migrate", "迁移…")]),
    ] {
        search(&mut surface, query);
        for (id, label) in expected {
            let node = surface.update(|window, _| {
                test_support::snapshots(window)
                    .into_iter()
                    .find(|node| {
                        node.role() == Some(gpui_kit::Role::Button)
                            && node.path().last() == Some(&gpui_kit::ElementId::from(id))
                    })
                    .unwrap_or_else(|| panic!("actual production Button {id}"))
            });
            assert_eq!(node.label(), Some(label), "actual label/a11y for {id}");
            let hit = node.bounds();
            assert!(hit.size.width > px(0.) && hit.left() >= px(176.) && hit.right() <= px(920.));
        }
    }
    assert!(!fixture.directory.path().join("backups").exists());
}

#[gpui_kit::test]
fn audit08_clipboard_picker_pointer_keyboard_once_and_cancel_preserve_data(
    cx: &mut TestAppContext,
) {
    let fixture = fixture(cx);
    fixture.endpoint.set_connected_for_contract_test(true);
    let config_path = fixture.directory.path().join("config.json");
    let before = std::fs::read(&config_path).expect("owned config bytes");
    let database = fixture.worker.database_path();
    let mut surface = VisualTestContext::from_window(fixture.window, cx);
    for (index, (query, id)) in [
        ("历史归档", "clipboard-export-archive"),
        ("历史归档", "clipboard-import-archive"),
        ("存储位置", "clipboard-storage-migrate"),
    ]
    .into_iter()
    .enumerate()
    {
        search(&mut surface, query);
        let hit = button_bounds(&mut surface, id);
        if index == 1 {
            surface.update(|window, cx| window.focus_next(cx));
            for _ in 0..80 {
                let focused = surface.update(|window, _| {
                    test_support::snapshots(window).into_iter().any(|node| {
                        node.path().last() == Some(&gpui_kit::ElementId::from(id))
                            && node.focused() == Some(true)
                    })
                });
                if focused {
                    break;
                }
                surface.simulate_keystrokes("tab");
            }
            assert!(
                surface.update(|window, _| test_support::snapshots(window).into_iter().any(
                    |node| node.path().last() == Some(&gpui_kit::ElementId::from(id))
                        && node.focused() == Some(true)
                ))
            );
            let key = gpui_kit::Keystroke::parse("space").expect("Space");
            surface.simulate_event(gpui_kit::KeyDownEvent {
                keystroke: key.clone(),
                is_held: false,
                prefer_character_input: false,
            });
            surface.run_until_parked();
            assert!(
                fixture
                    .endpoint
                    .try_receive_command_for_contract_test()
                    .is_err(),
                "key down alone cannot dispatch"
            );
            surface.simulate_event(gpui_kit::KeyUpEvent {
                keystroke: key.clone(),
            });
            surface.run_until_parked();
            let request = fixture
                .endpoint
                .try_receive_command_for_contract_test()
                .expect("one keyboard picker");
            assert!(matches!(
                request.message,
                nxtools_protocol::Message::PlatformCommand(
                    PlatformCommand::PickClipboardArchivePath(
                        nxtools_protocol::PickClipboardArchivePathCommand {
                            mode: nxtools_protocol::ClipboardArchivePickerMode::Import,
                            ..
                        }
                    )
                )
            ));
            surface.simulate_event(gpui_kit::KeyUpEvent { keystroke: key });
            surface.run_until_parked();
            assert!(
                fixture
                    .endpoint
                    .try_receive_command_for_contract_test()
                    .is_err(),
                "duplicate release cannot dispatch"
            );
            terminal(&fixture, request.id, true, cx);
        } else {
            surface.simulate_click(hit.center(), Modifiers::default());
            surface.run_until_parked();
            let request = fixture
                .endpoint
                .try_receive_command_for_contract_test()
                .expect("one pointer picker");
            assert!(
                matches!(
                    request.message,
                    nxtools_protocol::Message::PlatformCommand(
                        PlatformCommand::PickClipboardArchivePath(_)
                    )
                ) || matches!(
                    request.message,
                    nxtools_protocol::Message::PlatformCommand(
                        PlatformCommand::PickClipboardStorageDirectory(_)
                    )
                )
            );
            assert!(
                fixture
                    .endpoint
                    .try_receive_command_for_contract_test()
                    .is_err()
            );
            terminal(&fixture, request.id, true, cx);
        }
        assert!(
            !fixture
                .application
                .read_with(cx, |app, _| app.clipboard_management_busy())
        );
        assert_eq!(
            std::fs::read(&config_path).expect("cancel unchanged config"),
            before
        );
        assert_eq!(fixture.worker.database_path(), database);
        assert!(!fixture.directory.path().join("backups").exists());
    }
}
