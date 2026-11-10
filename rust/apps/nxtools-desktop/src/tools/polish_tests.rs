//! Render-bound checks of the real link panel, not only height arithmetic.
use super::link_detector::{DetectedLink, DetectedLinkKind};
use super::link_panel::{LINK_PANEL_WIDTH, LinkPanelView, link_panel_height};
use gpui_kit::base::Root;
use gpui_kit::{AppContext, TestAppContext, px, size};

#[gpui_kit::test]
fn link_panel_collapsed_and_expanded_render_within_the_fixed_client(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let links = [
        DetectedLinkKind::Web,
        DetectedLinkKind::LocalFile,
        DetectedLinkKind::LocalDirectory,
    ]
    .into_iter()
    .enumerate()
    .map(|(index, kind)| DetectedLink {
        display: format!("test-{index}-{}", "长名称与路径".repeat(40)),
        copy_value: format!("test-{index}"),
        kind,
    })
    .collect::<Vec<_>>();
    let collapsed = link_panel_height(links.len(), false);
    let expanded = link_panel_height(links.len(), true);
    let (_root, window) = cx.add_window_view(move |window, cx| {
        window.resize(size(px(LINK_PANEL_WIDTH), px(collapsed)));
        let handle = window.window_handle();
        let view = cx.new(|_| LinkPanelView {
            links,
            application: gpui_kit::WeakEntity::new_invalid(),
            expanded: false,
            auto_hide_ms: 6_000,
            window_handle: Some(handle),
            auto_hide_task: None,
        });
        Root::new(view, window, cx)
    });
    window.refresh().expect("render collapsed production panel");
    let first = window.debug_bounds("link-panel-row-0").expect("first row");
    let expander = window
        .debug_bounds("link-panel-expander")
        .expect("collapsed expander");
    let footer = window.debug_bounds("link-panel-footer").expect("footer");
    assert_eq!(first.size.height, px(28.0));
    assert_eq!(expander.size.height, px(24.0));
    assert_eq!(footer.size.height, px(29.0));
    let name = window
        .debug_bounds("link-panel-name-0")
        .expect("bounded link name");
    let icon = window
        .debug_bounds("link-panel-icon-0")
        .expect("stable kind icon");
    assert!(icon.left() >= first.left() && icon.right() <= name.left());
    assert!(name.right() <= first.right());
    let copy = window
        .debug_bounds("link-panel-copy-0")
        .expect("explicit copy");
    assert!(name.right() <= copy.left());
    assert!(copy.right() <= first.right() && copy.bottom() <= first.bottom());
    assert_eq!(name.size.height, px(20.0));
    assert!(first.bottom() <= expander.top());
    assert!(expander.bottom() <= footer.top());
    assert!(footer.bottom() <= px(collapsed));
    assert!(window.debug_bounds("link-panel-row-1").is_none());

    window.simulate_click(expander.center(), gpui_kit::Modifiers::default());
    window.run_until_parked();
    window.refresh().expect("render expanded production panel");
    let third = window
        .debug_bounds("link-panel-row-2")
        .expect("expanded third row");
    let footer = window
        .debug_bounds("link-panel-footer")
        .expect("expanded footer");
    assert!(third.bottom() <= footer.top());
    assert!(footer.bottom() <= px(expanded));
}

#[gpui_kit::test]
fn link_panel_row_keyup_opens_without_implicit_copy_or_application(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string("unchanged".into()));
    let (_root, window) = cx.add_window_view(|window, cx| {
        window.resize(size(px(LINK_PANEL_WIDTH), px(link_panel_height(1, false))));
        let view = cx.new(|_| LinkPanelView {
            links: vec![DetectedLink {
                display: "https://example.invalid".into(),
                copy_value: "https://example.invalid".into(),
                kind: DetectedLinkKind::Web,
            }],
            application: gpui_kit::WeakEntity::new_invalid(),
            expanded: false,
            auto_hide_ms: 6_000,
            window_handle: Some(window.window_handle()),
            auto_hide_task: None,
        });
        Root::new(view, window, cx)
    });
    window.refresh().expect("render focusable link row");
    let bounds = window
        .debug_bounds("link-panel-row-0")
        .expect("link row hitbox");
    assert_eq!(bounds.size.height, px(28.0));
    window.update(|window, cx| window.focus_next(cx));
    window
        .refresh()
        .expect("focus link row through actual Tab order");
    window.simulate_keystrokes("enter");
    window.simulate_event(gpui_kit::KeyUpEvent {
        keystroke: gpui_kit::Keystroke::parse("enter").expect("Enter stroke"),
    });
    window.run_until_parked();
    // Opening must not mutate even TestPlatform's clipboard on a disconnected owner.
    assert_eq!(
        cx.read_from_clipboard()
            .expect("original clipboard")
            .text()
            .as_deref(),
        Some("unchanged")
    );
}

#[gpui_kit::test]
fn clipboard_and_tools_workbench_groups_stay_readable_at_minimum_width(cx: &mut TestAppContext) {
    use crate::main_window::{MainWindow, MainWindowShellContext};
    use crate::notifications::NotificationCenter;
    use nxtools_core::SystemSettings;
    use std::{cell::RefCell, rc::Rc};
    cx.update(gpui_kit::init);
    let notifications = cx.update(|cx| cx.new(|_| NotificationCenter::default()));
    let (_main, window) = cx.add_window_view(move |window, cx| {
        window.resize(size(px(920.0), px(480.0)));
        let main = cx.new(|cx| {
            MainWindow::new(
                gpui_kit::WeakEntity::new_invalid(),
                gpui_kit::WeakEntity::new_invalid(),
                gpui_kit::WeakEntity::new_invalid(),
                MainWindowShellContext {
                    settings: None,
                    platform: None,
                    system_settings: Rc::new(RefCell::new(SystemSettings::default())),
                },
                notifications,
                window,
                cx,
            )
        });
        gpui_kit::base::Root::new(main, window, cx)
    });
    window.simulate_resize(size(px(920.0), px(480.0)));
    window.refresh().expect("render actual minimum workbench");
    let clipboard = window
        .debug_bounds("main-module-剪贴板")
        .expect("clipboard entry");
    window.simulate_click(clipboard.center(), gpui_kit::Modifiers::default());
    window
        .refresh()
        .expect("render continuous Clipboard settings");
    let viewport = window
        .debug_bounds("clipboard-settings-workspace")
        .expect("settings viewport");
    assert!(viewport.bottom() <= px(480.0));
    let hotkey = window
        .debug_bounds("record-clipboard-hotkey")
        .expect("hotkey action");
    let modifier = window
        .debug_bounds("clipboard-modifier-control")
        .expect("modifier action");
    assert_eq!(hotkey.size.height, px(32.0));
    assert_eq!(modifier.size.height, px(32.0));
    assert!(hotkey.left() >= viewport.left() && hotkey.right() <= viewport.right());
    assert!(modifier.left() >= viewport.left() && modifier.right() <= viewport.right());
    let nodes = window.update(|window, _| gpui_kit::base::test_support::snapshots(window));
    for page in 0..3 {
        assert!(
            nodes
                .iter()
                .any(|node| node.path().last()
                    == Some(&gpui_kit::ElementId::from(format!("0-{page}")))),
            "all original topics retained in official navigation"
        );
    }
    let tools = window
        .debug_bounds("main-module-其他工具")
        .expect("tools entry");
    window.simulate_click(tools.center(), gpui_kit::Modifiers::default());
    window.refresh().expect("render registered Tools entry");
    let title = window
        .debug_bounds("tool-link-detector-title")
        .expect("workbench title");
    assert_eq!(title.size.height, px(20.0));
    assert!(
        window
            .debug_bounds("tool-link-detector-description")
            .is_none(),
        "tool description is Tooltip-only"
    );
}

#[gpui_kit::test]
fn queue08_link_mounts_stateless_listitem_and_default_label(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(crate::theme::apply_component_theme);
    let (_, window) = cx.add_window_view(|window, cx| {
        let view = cx.new(|_| LinkPanelView {
            links: vec![DetectedLink {
                display: "合成链接".into(),
                copy_value: "synthetic.invalid".into(),
                kind: DetectedLinkKind::Web,
            }],
            application: gpui_kit::WeakEntity::new_invalid(),
            expanded: false,
            auto_hide_ms: 6000,
            window_handle: Some(window.window_handle()),
            auto_hide_task: None,
        });
        Root::new(view, window, cx)
    });
    window.simulate_resize(size(px(420.0), px(link_panel_height(1, false))));
    window.refresh().expect("actual Link presentation");
    let nodes = window.update(|window, _| gpui_kit::base::test_support::snapshots(window));
    assert!(
        nodes.iter().any(|node| node.label() == Some("链接 1")),
        "actual stateless Link ListItem"
    );
    assert_eq!(
        window
            .debug_bounds("link-panel-name-0")
            .expect("default Label")
            .size
            .height,
        px(20.0)
    );
}

#[gpui_kit::test]
fn queue08_link_pointer_keyboard_close_before_deferred_synthetic_copy(cx: &mut TestAppContext) {
    use gpui_kit::{KeyDownEvent, KeyUpEvent, Keystroke, Modifiers};
    cx.update(gpui_kit::init);
    cx.update(crate::theme::apply_component_theme);
    for keyboard in [false, true] {
        let value = format!("synthetic-copy-{keyboard}");
        let held = value.clone();
        let windows_before = cx.windows().len();
        let (_, window) = cx.add_window_view(move |window, cx| {
            let view = cx.new(|_| LinkPanelView {
                links: vec![DetectedLink {
                    display: "合成名称 **plain** <b>text</b>".into(),
                    copy_value: held,
                    kind: DetectedLinkKind::Web,
                }],
                application: gpui_kit::WeakEntity::new_invalid(),
                expanded: false,
                auto_hide_ms: 6000,
                window_handle: Some(window.window_handle()),
                auto_hide_task: None,
            });
            Root::new(view, window, cx)
        });
        window.simulate_resize(size(px(420.0), px(link_panel_height(1, false))));
        window.update(|window, cx| {
            window.activate_window();
            window.focus_next(cx);
            window.focus_next(cx); // The independent secondary Copy button.
        });
        window.refresh().expect("actual Link action");
        if keyboard {
            assert!(window.update(|window, cx| window.focused(cx)).is_some());
            window.simulate_event(KeyDownEvent {
                keystroke: Keystroke::parse("enter").expect("Enter"),
                is_held: false,
                prefer_character_input: false,
            });
            window.simulate_event(KeyUpEvent {
                keystroke: Keystroke::parse("enter").expect("release"),
            });
        } else {
            let row = window
                .debug_bounds("link-panel-copy-0")
                .expect("copy action");
            window.simulate_click(row.center(), Modifiers::default());
        }
        window.run_until_parked();
        assert_eq!(
            cx.windows().len(),
            windows_before,
            "owning window removed by original action"
        );
        let copied = cx
            .read_from_clipboard()
            .expect("TestPlatform synthetic clipboard");
        assert_eq!(copied.text().as_deref(), Some(value.as_str()));
    }
}

#[gpui_kit::test]
fn queue08_link_window_timer_closes_only_its_synthetic_surface(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.executor().allow_parking();
    let executor = cx.executor();
    let windows_before = cx.windows().len();
    let (_, window) = cx.add_window_view(|window, cx| {
        let handle = window.window_handle();
        let view = cx.new(|cx| {
            let mut view = LinkPanelView {
                links: vec![DetectedLink {
                    display: "timer".into(),
                    copy_value: "timer".into(),
                    kind: DetectedLinkKind::Web,
                }],
                application: gpui_kit::WeakEntity::new_invalid(),
                expanded: false,
                auto_hide_ms: 200,
                window_handle: Some(handle),
                auto_hide_task: None,
            };
            view.arm_auto_hide(handle, cx);
            view
        });
        Root::new(view, window, cx)
    });
    window.refresh().expect("timer surface alive");
    window.run_until_parked();
    executor.advance_clock(std::time::Duration::from_millis(250));
    window.run_until_parked();
    assert_eq!(
        cx.windows().len(),
        windows_before,
        "original bounded window timer removes its window"
    );
}

/// Real application owner with only a temporary history store and in-memory IPC.
fn link_application_fixture(
    cx: &mut TestAppContext,
    copy_only: bool,
    auto_open_browser: bool,
) -> (
    tempfile::TempDir,
    crate::clipboard_worker::ClipboardCaptureWorkerHandle,
    gpui_kit::Entity<crate::clipboard::ClipboardApplication>,
    nxtools_ipc::PlatformServerEndpoint,
) {
    use std::rc::Rc;
    let temporary = tempfile::tempdir().expect("synthetic link history");
    let worker = crate::clipboard_worker::ClipboardCaptureWorkerHandle::spawn(
        temporary.path().join("clipboard.db"),
        temporary.path().join("assets"),
        nxtools_clipboard::ClipboardRetentionPolicy {
            max_items: 1000,
            retention_days: 30,
        },
        1000,
    )
    .expect("synthetic history worker");
    let (platform, endpoint) = nxtools_ipc::platform_bridge(32);
    let mut config = nxtools_core::AppConfig::default();
    config.clipboard.link_detector_enabled = true;
    config.clipboard.link_panel_copy_only = copy_only;
    config.clipboard.link_panel_auto_open_browser = auto_open_browser;
    let application = cx.update(|cx| {
        let notifications = cx.new(|_| crate::notifications::NotificationCenter::default());
        cx.new(|cx| {
            crate::clipboard::ClipboardApplication::new(
                worker.clone(),
                Rc::new(platform),
                config,
                temporary.path().join("config.json"),
                notifications,
                cx,
            )
        })
    });
    (temporary, worker, application, endpoint)
}

#[gpui_kit::test]
fn ref01_production_row_and_copy_pointer_key_release_use_current_preferences(
    cx: &mut TestAppContext,
) {
    use gpui_kit::{KeyDownEvent, KeyUpEvent, Keystroke, Modifiers, VisualTestContext};
    use nxtools_protocol::{Message, OpenShellTargetCommand, PlatformCommand, ShellOpenAction};
    cx.update(gpui_kit::init);
    cx.update(crate::theme::apply_component_theme);
    cx.executor().allow_parking();
    for copy_only in [false, true] {
        for auto_open_browser in [false, true] {
            let (_temporary, worker, application, endpoint) =
                link_application_fixture(cx, copy_only, auto_open_browser);
            endpoint.set_connected_for_contract_test(true);
            for (kind, target, action) in [
                (
                    DetectedLinkKind::Web,
                    "https://synthetic.invalid",
                    ShellOpenAction::Open,
                ),
                (
                    DetectedLinkKind::LocalFile,
                    r"C:\synthetic\file.txt",
                    ShellOpenAction::SelectInExplorer,
                ),
                (
                    DetectedLinkKind::LocalDirectory,
                    r"C:\synthetic",
                    ShellOpenAction::Open,
                ),
            ] {
                for key in [None, Some("enter"), Some("space")] {
                    for explicit_copy in [false, true] {
                        cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string(
                            "untouched".into(),
                        ));
                        application.update(cx, |application, cx| {
                            application.show_link_panel(
                                vec![DetectedLink {
                                    display: "合成链接或路径".into(),
                                    copy_value: target.into(),
                                    kind,
                                }],
                                cx,
                            )
                        });
                        let handle = *cx.windows().last().expect("production link window");
                        let mut window = VisualTestContext::from_window(handle, cx);
                        window.simulate_resize(size(px(420.0), px(link_panel_height(1, false))));
                        window.refresh().expect("actual production action bounds");
                        // No Windows process or pipe exists. Drain presentation commands only.
                        while let Ok(envelope) = endpoint.try_receive_command_for_contract_test() {
                            assert!(
                                !matches!(
                                    envelope.message,
                                    Message::PlatformCommand(PlatformCommand::OpenShellTarget(_))
                                ),
                                "show is identification only"
                            );
                        }
                        let copy_bounds = window
                            .debug_bounds("link-panel-copy-0")
                            .expect("copy hit target");
                        let nodes = window
                            .update(|window, _| gpui_kit::base::test_support::snapshots(window));
                        assert!(nodes.iter().any(|node| node.label()
                            == Some(if kind == DetectedLinkKind::Web {
                                "复制链接"
                            } else {
                                "复制路径"
                            })));
                        if let Some(key) = key {
                            window.update(|window, cx| {
                                window.activate_window();
                                window.focus_next(cx);
                                if explicit_copy {
                                    window.focus_next(cx);
                                }
                            });
                            window.refresh().expect("focus original row or Kit copy");
                            window.simulate_event(KeyDownEvent {
                                keystroke: Keystroke::parse(key).expect("activation key"),
                                is_held: false,
                                prefer_character_input: false,
                            });
                            assert_eq!(cx.windows().len(), 1, "no activation on press");
                            assert_eq!(
                                cx.read_from_clipboard()
                                    .expect("clipboard before release")
                                    .text()
                                    .as_deref(),
                                Some("untouched")
                            );
                            assert!(endpoint.try_receive_command_for_contract_test().is_err());
                            window.simulate_event(KeyUpEvent {
                                keystroke: Keystroke::parse(key).expect("activation release"),
                            });
                        } else {
                            let target_bounds = if explicit_copy {
                                copy_bounds
                            } else {
                                window
                                    .debug_bounds("link-panel-name-0")
                                    .expect("primary opening hit target")
                            };
                            window.simulate_click(target_bounds.center(), Modifiers::default());
                        }
                        window.run_until_parked();
                        assert!(cx.windows().is_empty(), "close before deferred dispatch");
                        assert_eq!(
                            cx.read_from_clipboard()
                                .expect("TestPlatform clipboard")
                                .text()
                                .as_deref(),
                            Some(
                                if explicit_copy
                                    || copy_only
                                    || (kind == DetectedLinkKind::Web && !auto_open_browser)
                                {
                                    target
                                } else {
                                    "untouched"
                                }
                            )
                        );
                        if explicit_copy
                            || copy_only
                            || (kind == DetectedLinkKind::Web && !auto_open_browser)
                        {
                            assert!(
                                endpoint.try_receive_command_for_contract_test().is_err(),
                                "copy must not bubble into open"
                            );
                            application.update(cx, |application, cx| {
                                application.handle_link_detector_capture(target, cx)
                            });
                            assert!(
                                cx.windows().is_empty(),
                                "original self-copy echo suppression"
                            );
                        } else {
                            let envelope = endpoint
                                .try_receive_command_for_contract_test()
                                .expect("one open on in-memory bridge");
                            assert!(
                                envelope.correlation_id.is_some(),
                                "one correlated native action"
                            );
                            assert_eq!(
                                envelope.message,
                                Message::PlatformCommand(PlatformCommand::OpenShellTarget(
                                    OpenShellTargetCommand {
                                        target: target.into(),
                                        action
                                    }
                                ))
                            );
                            assert!(
                                endpoint.try_receive_command_for_contract_test().is_err(),
                                "one release/click dispatcher"
                            );
                        }
                    }
                }
            }
            worker.shutdown().expect("flush synthetic link history");
        }
    }
}

#[gpui_kit::test]
fn ref01_named_open_action_bypasses_ordinary_settings_without_copy(cx: &mut TestAppContext) {
    use gpui_kit::{KeyDownEvent, KeyUpEvent, Keystroke, Modifiers, VisualTestContext};
    use nxtools_protocol::{Message, OpenShellTargetCommand, PlatformCommand, ShellOpenAction};
    cx.update(gpui_kit::init);
    cx.update(crate::theme::apply_component_theme);
    cx.executor().allow_parking();
    for copy_only in [false, true] {
        for auto_open_browser in [false, true] {
            let (_temporary, worker, application, endpoint) =
                link_application_fixture(cx, copy_only, auto_open_browser);
            endpoint.set_connected_for_contract_test(true);
            for (kind, target, action) in [
                (
                    DetectedLinkKind::Web,
                    "https://explicit.invalid",
                    ShellOpenAction::Open,
                ),
                (
                    DetectedLinkKind::LocalFile,
                    r"C:\synthetic\explicit.txt",
                    ShellOpenAction::SelectInExplorer,
                ),
                (
                    DetectedLinkKind::LocalDirectory,
                    r"C:\synthetic",
                    ShellOpenAction::Open,
                ),
            ] {
                for keyboard in [false, true] {
                    cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string("untouched".into()));
                    application.update(cx, |application, cx| {
                        application.show_link_panel(
                            vec![DetectedLink {
                                display: "合成目标".into(),
                                copy_value: target.into(),
                                kind,
                            }],
                            cx,
                        );
                    });
                    let mut window = VisualTestContext::from_window(
                        *cx.windows().last().expect("production link window"),
                        cx,
                    );
                    window.simulate_resize(size(px(420.0), px(link_panel_height(1, false))));
                    window.refresh().expect("named open action rendered");
                    while let Ok(envelope) = endpoint.try_receive_command_for_contract_test() {
                        assert!(!matches!(
                            envelope.message,
                            Message::PlatformCommand(PlatformCommand::OpenShellTarget(_))
                        ));
                    }
                    let open = window
                        .debug_bounds("link-panel-open-0")
                        .expect("named open action");
                    let copy = window
                        .debug_bounds("link-panel-copy-0")
                        .expect("named copy action");
                    assert!(open.left() >= copy.right() && open.right() <= px(420.0));
                    if keyboard {
                        window.update(|window, cx| {
                            window.activate_window();
                            window.focus_next(cx);
                            window.focus_next(cx);
                            window.focus_next(cx);
                        });
                        window.refresh().expect("Tab reaches named open action");
                        window.simulate_event(KeyDownEvent {
                            keystroke: Keystroke::parse("enter").expect("Enter"),
                            is_held: false,
                            prefer_character_input: false,
                        });
                        assert_eq!(cx.windows().len(), 1, "no action on press");
                        window.simulate_event(KeyUpEvent {
                            keystroke: Keystroke::parse("enter").expect("release"),
                        });
                    } else {
                        window.simulate_click(open.center(), Modifiers::default());
                    }
                    window.run_until_parked();
                    assert!(cx.windows().is_empty(), "named action closes its own panel");
                    assert_eq!(
                        cx.read_from_clipboard()
                            .expect("clipboard")
                            .text()
                            .as_deref(),
                        Some("untouched")
                    );
                    let envelope = endpoint
                        .try_receive_command_for_contract_test()
                        .expect("typed explicit open");
                    assert!(envelope.correlation_id.is_some());
                    assert_eq!(
                        envelope.message,
                        Message::PlatformCommand(PlatformCommand::OpenShellTarget(
                            OpenShellTargetCommand {
                                target: target.into(),
                                action,
                            }
                        ))
                    );
                    assert!(
                        endpoint.try_receive_command_for_contract_test().is_err(),
                        "one explicit action"
                    );
                }
            }
            worker.shutdown().expect("flush synthetic link history");
        }
    }
}
#[gpui_kit::test]
fn r6_capture_reuses_one_window_and_retires_superseded_six_second_timer(cx: &mut TestAppContext) {
    use gpui_kit::VisualTestContext;
    use std::time::Duration;
    cx.update(gpui_kit::init);
    cx.executor().allow_parking();
    let executor = cx.executor();
    let (_temporary, worker, application, endpoint) = link_application_fixture(cx, true, true);
    endpoint.set_connected_for_contract_test(true);
    cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string(
        "captured-original".into(),
    ));
    application.update(cx, |application, cx| {
        application.handle_link_detector_capture("https://first.invalid", cx)
    });
    crate::clipboard::ClipboardApplication::wait_for_link_detection(cx, &application);
    let first = *cx.windows().last().expect("first capture panel");
    cx.run_until_parked();
    executor.advance_clock(Duration::from_secs(4));
    cx.run_until_parked();
    application.update(cx, |application, cx| {
        application.handle_link_detector_capture("https://second.invalid", cx)
    });
    crate::clipboard::ClipboardApplication::wait_for_link_detection(cx, &application);
    assert_eq!(
        cx.windows(),
        vec![first],
        "restage same Root-owned panel, no replacement or stacked window"
    );
    let mut window = VisualTestContext::from_window(first, cx);
    window.refresh().expect("restaged panel remains visible");
    while let Ok(envelope) = endpoint.try_receive_command_for_contract_test() {
        assert!(
            !matches!(
                envelope.message,
                nxtools_protocol::Message::PlatformCommand(
                    nxtools_protocol::PlatformCommand::OpenShellTarget(_)
                )
            ),
            "neither capture auto-opens"
        );
    }
    assert_eq!(
        cx.read_from_clipboard()
            .expect("capture clipboard unchanged")
            .text()
            .as_deref(),
        Some("captured-original")
    );
    cx.run_until_parked();
    executor.advance_clock(Duration::from_millis(2500));
    cx.run_until_parked();
    assert_eq!(
        cx.windows(),
        vec![first],
        "old capture deadline must not close new generation"
    );
    executor.advance_clock(Duration::from_millis(3600));
    cx.run_until_parked();
    assert!(
        cx.windows().is_empty(),
        "new capture closes at its own six-second deadline"
    );
    application.update(cx, |application, cx| {
        application.handle_link_detector_capture("https://third.invalid", cx);
    });
    crate::clipboard::ClipboardApplication::wait_for_link_detection(cx, &application);
    cx.run_until_parked();
    let third = *cx.windows().last().expect("capture after timer close");
    assert_ne!(third, first);
    assert_eq!(
        cx.windows(),
        vec![third],
        "one rebuilt window after timer close"
    );
    third
        .update(cx, |_, window, _| window.remove_window())
        .expect("synthetic explicit close");
    application.update(cx, |application, cx| {
        application.forget_link_panel();
        application.handle_link_detector_capture("https://fourth.invalid", cx);
    });
    crate::clipboard::ClipboardApplication::wait_for_link_detection(cx, &application);
    cx.run_until_parked();
    let fourth = *cx.windows().last().expect("capture after explicit forget");
    assert_ne!(fourth, third);
    assert_eq!(
        cx.windows(),
        vec![fourth],
        "one rebuilt window after explicit forget"
    );
    fourth
        .update(cx, |_, window, _| window.remove_window())
        .expect("close synthetic rebuilt panel");
    worker.shutdown().expect("flush synthetic timer history");
}

#[gpui_kit::test]
fn r6_disconnected_explicit_open_closes_without_copy_fallback(cx: &mut TestAppContext) {
    use gpui_kit::VisualTestContext;
    cx.update(gpui_kit::init);
    cx.executor().allow_parking();
    let (_temporary, worker, application, _endpoint) = link_application_fixture(cx, true, false);
    cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string("original".into()));
    application.update(cx, |application, cx| {
        application.show_link_panel(
            vec![DetectedLink {
                display: "合成网页".into(),
                copy_value: "https://synthetic.invalid".into(),
                kind: DetectedLinkKind::Web,
            }],
            cx,
        )
    });
    let mut window =
        VisualTestContext::from_window(*cx.windows().last().expect("disconnected panel"), cx);
    window.refresh().expect("disconnected production panel");
    let name = window
        .debug_bounds("link-panel-open-0")
        .expect("opening action");
    window.simulate_click(name.center(), gpui_kit::Modifiers::default());
    window.run_until_parked();
    assert!(cx.windows().is_empty());
    assert_eq!(
        cx.read_from_clipboard()
            .expect("unchanged after rejection")
            .text()
            .as_deref(),
        Some("original")
    );
    worker
        .shutdown()
        .expect("flush synthetic disconnected history");
}

#[gpui_kit::test]
fn round2_link_popup_preserves_official_row_padding_and_statusbar_height(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (_root, window) = cx.add_window_view(|window, cx| {
        window.resize(size(px(LINK_PANEL_WIDTH), px(link_panel_height(1, false))));
        let view = cx.new(|_| LinkPanelView {
            links: vec![DetectedLink {
                display: "合成链接".into(),
                copy_value: "https://example.invalid".into(),
                kind: DetectedLinkKind::Web,
            }],
            application: gpui_kit::WeakEntity::new_invalid(),
            expanded: false,
            auto_hide_ms: 6000,
            window_handle: Some(window.window_handle()),
            auto_hide_task: None,
        });
        Root::new(view, window, cx)
    });
    window.refresh().expect("production link popup");
    let row = window.debug_bounds("link-panel-row-0").expect("row");
    let icon = window.debug_bounds("link-panel-icon-0").expect("icon");
    let footer = window.debug_bounds("link-panel-footer").expect("statusbar");
    assert_eq!(
        icon.left() - row.left(),
        px(12.0),
        "released ListItem padding, no second border/inset wrapper"
    );
    assert!(
        footer.size.height >= px(24.0),
        "official statusbar text and padding must fit"
    );
}
