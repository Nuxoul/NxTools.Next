//! Tests for the parent module. Extracted from the parent file so the
//! runtime code stays readable.

use super::*;

use gpui_kit::TestAppContext;

#[gpui_kit::test]
fn quick_copy_feedback_uses_shared_center_without_a_second_panel_banner(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (_temp, application, _endpoint) = quick_launch_application_fixture(cx);
    let (panel, window) = quick_launch_panel_fixture(
        cx,
        vec![search_result("keep", ApplicationSource::IndexedFile)],
    );
    panel.update_in(window, |panel, _, cx| {
        panel.application = application.downgrade();
        panel.set_status("已复制路径：合成结果".into(), cx);
        assert!(
            panel.status.is_none(),
            "transient copy result must not paint the custom bottom banner"
        );
        assert_eq!(panel.results.len(), 1);
        assert!(panel.is_open, "copy feedback does not change close policy");
    });
    window.run_until_parked();
    application.read_with(window, |app, cx| {
        let notices = app.notifications.read(cx).items();
        assert_eq!(notices.len(), 1);
        assert_eq!(
            notices[0].level,
            crate::notifications::NotificationLevel::Success
        );
        assert_eq!(notices[0].message, "已复制路径：合成结果");
    });
    application.update(window, |_, cx| {
        panel.update(cx, |panel, cx| {
            panel.notify_operation(
                crate::notifications::NotificationLevel::Error,
                "合成 Windows 失败".into(),
                cx,
            )
        });
    });
    window.run_until_parked();
    application.read_with(window, |app, cx| {
        let notices = app.notifications.read(cx).items();
        assert_eq!(notices.len(), 2, "distinct invocations must not replace");
        assert_ne!(notices[0].id, notices[1].id);
        assert_eq!(
            notices[0].level,
            crate::notifications::NotificationLevel::Error
        );
    });
    // Exercise the real typed native terminal routing, including once-only and stale IDs.
    let handle = window.update(|window, _| window.window_handle());
    application.update(window, |app, cx| {
        app.popup = Some(QuickLaunchWindow {
            window: handle,
            panel: panel.downgrade(),
            native_window_handle: 1,
        });
        app.execute(
            search_result("keep", ApplicationSource::IndexedFile),
            QuickLaunchAction::CopyPath,
            "keep".into(),
            QuickLaunchActionTrigger::Enter,
            cx,
        );
    });
    let command = application.read_with(window, |app, _| {
        *app.pending_actions.keys().next().expect("accepted copy")
    });
    application.update(window, |app, cx| {
        let result = PlatformCommandResult {
            success: true,
            error: None,
            native_error_code: None,
            data: None,
        };
        app.handle_command_result(command, result.clone(), cx);
        app.handle_command_result(command, result, cx);
    });
    window.run_until_parked();
    application.read_with(window, |app, cx| {
        assert!(app.pending_actions.is_empty());
        let notices = app.notifications.read(cx).items();
        assert_eq!(notices.len(), 3);
        assert_eq!(
            notices[0].level,
            crate::notifications::NotificationLevel::Success
        );
    });
    assert!(panel.read_with(window, |panel, _| panel.is_open && panel.status.is_none()));
}

#[gpui_kit::test]
fn production_result_row_keeps_height_through_animated_surface(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (_root, window) = cx.add_window_view(move |window, cx| {
        window.resize(size(px(PANEL_WIDTH), px(PANEL_HEIGHT)));
        let panel = cx.new(|cx| {
            let mut panel = QuickLaunchPanel::new(
                vec![search_result(
                    "first-result",
                    ApplicationSource::IndexedFile,
                )],
                nxtools_protocol::ForegroundContext {
                    window_handle: 1,
                    focus_window_handle: 1,
                    process_id: 1,
                    process_name: "editor".into(),
                    window_title: "document".into(),
                },
                None,
                Vec::new(),
                WeakEntity::new_invalid(),
                window,
                cx,
            );
            panel.results_visible = true;
            panel
        });
        Root::new(panel, window, cx)
    });

    window
        .refresh()
        .expect("render populated quick-launch results");
    let row = window
        .debug_bounds("quick-launch-result-row-0")
        .expect("production quick-launch result row");
    assert_eq!(row.size.height, px(LAUNCHER_RESULT_ROW_HEIGHT));
}

#[gpui_kit::test]
fn production_keyboard_pointer_movement_and_press_share_one_selection(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (_temp, application, endpoint) = quick_launch_application_fixture(cx);
    let panel_slot = Rc::new(RefCell::new(None));
    let panel_slot_for_window = Rc::clone(&panel_slot);
    let application_for_window = application.clone();
    let (_root, window) = cx.add_window_view(move |window, cx| {
        window.resize(size(px(PANEL_WIDTH), px(PANEL_HEIGHT)));
        let panel = cx.new(|cx| {
            let mut panel = QuickLaunchPanel::new(
                vec![
                    search_result("first", ApplicationSource::StartMenu),
                    search_result("second", ApplicationSource::IndexedFile),
                    search_result("third", ApplicationSource::Desktop),
                ],
                nxtools_protocol::ForegroundContext {
                    window_handle: 1,
                    focus_window_handle: 1,
                    process_id: 1,
                    process_name: "editor".into(),
                    window_title: "document".into(),
                },
                None,
                Vec::new(),
                application_for_window.downgrade(),
                window,
                cx,
            );
            panel.results_visible = true;
            panel
        });
        *panel_slot_for_window.borrow_mut() = Some(panel.clone());
        Root::new(panel, window, cx)
    });
    let panel = panel_slot
        .borrow_mut()
        .take()
        .expect("production quick-launch panel");

    window.refresh().expect("render quick-launch results");
    panel.update_in(window, |panel, window, cx| panel.focus_search(window, cx));
    window.simulate_keystrokes("down");
    panel.update_in(window, |panel, _, _| assert_eq!(panel.selected, 1));

    let third_row = window
        .debug_bounds("quick-launch-result-icon-2")
        .expect("third production result row");
    window.simulate_mouse_move(third_row.center(), None, gpui_kit::Modifiers::default());
    panel.update_in(window, |panel, _, _| assert_eq!(panel.selected, 1));
    window.simulate_mouse_move(
        gpui_kit::point(third_row.origin.x + px(2.0), third_row.center().y),
        None,
        gpui_kit::Modifiers::default(),
    );
    panel.update_in(window, |panel, _, _| assert_eq!(panel.selected, 2));

    window.simulate_keystrokes("up");
    panel.update_in(window, |panel, _, _| assert_eq!(panel.selected, 1));
    window.simulate_mouse_down(
        third_row.center(),
        gpui_kit::MouseButton::Left,
        gpui_kit::Modifiers::default(),
    );

    panel.update_in(window, |panel, _, _| assert_eq!(panel.selected, 2));
    window.simulate_mouse_up(
        third_row.center(),
        gpui_kit::MouseButton::Left,
        gpui_kit::Modifiers::default(),
    );
    cx.run_until_parked();
    let command = (0..8)
        .filter_map(|_| endpoint.try_receive_command_for_contract_test().ok())
        .find(|envelope| {
            matches!(
                &envelope.message,
                nxtools_protocol::Message::PlatformCommand(
                    PlatformCommand::OpenShellTarget(open)
                ) if open.target == "third.lnk" && open.action == ShellOpenAction::Open
            )
        })
        .expect("the directly clicked third result executes after becoming selected");
    application.read_with(cx, |application, _| {
        let pending = application
            .pending_actions
            .get(&command.id)
            .expect("direct click retains the correlated pending action");
        assert_eq!(pending.entry_id, "third");
        assert_eq!(pending.trigger, QuickLaunchActionTrigger::Pointer);
    });
}

#[gpui_kit::test]
fn production_heterogeneous_result_rows_keep_one_three_rail_geometry(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (_temp, application, endpoint) = quick_launch_application_fixture(cx);
    let results = [
        (
            "application",
            ApplicationSource::StartMenu,
            "NxTools",
            "应用",
        ),
        (
            "file",
            ApplicationSource::IndexedFile,
            "ui-remediation-plan.md",
            "D:\\RiderProject\\NxTools.Next\\docs",
        ),
        (
            "command",
            ApplicationSource::CustomCommand,
            "打开终端",
            "自定义命令",
        ),
        (
            "calculator",
            ApplicationSource::Calculator,
            "42",
            "计算结果",
        ),
    ]
    .into_iter()
    .map(|(id, source, name, subtitle)| {
        let mut result = search_result(id, source);
        result.entry.name = name.into();
        result.subtitle = Some(subtitle.into());
        result
    })
    .collect::<Vec<_>>();
    let panel_slot = Rc::new(RefCell::new(None));
    let panel_slot_for_window = Rc::clone(&panel_slot);
    let application_for_window = application.clone();
    let (_root, window) = cx.add_window_view(move |window, cx| {
        window.resize(size(px(PANEL_WIDTH), px(PANEL_HEIGHT)));
        let panel = cx.new(|cx| {
            let mut panel = QuickLaunchPanel::new(
                results,
                nxtools_protocol::ForegroundContext {
                    window_handle: 1,
                    focus_window_handle: 1,
                    process_id: 1,
                    process_name: "editor".into(),
                    window_title: "document".into(),
                },
                None,
                Vec::new(),
                application_for_window.downgrade(),
                window,
                cx,
            );
            panel.results_visible = true;
            panel
        });
        *panel_slot_for_window.borrow_mut() = Some(panel.clone());
        Root::new(panel, window, cx)
    });
    let panel = panel_slot
        .borrow_mut()
        .take()
        .expect("heterogeneous quick-launch panel");

    window
        .refresh()
        .expect("render heterogeneous quick-launch results");
    assert!(
        window
            .debug_bounds("quick-launch-result-icon-fallback-0")
            .is_some(),
        "a result without a decoded Shell icon must keep a visible semantic fallback"
    );
    let mut expected_icon = None;
    let mut expected_copy = None;
    let mut expected_shortcut = None;
    let selectors = [
        (
            "quick-launch-result-icon-0",
            "quick-launch-result-copy-0",
            "quick-launch-result-title-0",
            "quick-launch-result-subtitle-0",
            "quick-launch-result-shortcut-0",
        ),
        (
            "quick-launch-result-icon-1",
            "quick-launch-result-copy-1",
            "quick-launch-result-title-1",
            "quick-launch-result-subtitle-1",
            "quick-launch-result-shortcut-1",
        ),
        (
            "quick-launch-result-icon-2",
            "quick-launch-result-copy-2",
            "quick-launch-result-title-2",
            "quick-launch-result-subtitle-2",
            "quick-launch-result-shortcut-2",
        ),
        (
            "quick-launch-result-icon-3",
            "quick-launch-result-copy-3",
            "quick-launch-result-title-3",
            "quick-launch-result-subtitle-3",
            "quick-launch-result-shortcut-3",
        ),
    ];
    for (icon_selector, copy_selector, title_selector, subtitle_selector, shortcut_selector) in
        selectors
    {
        let icon = window
            .debug_bounds(icon_selector)
            .expect("fixed result icon rail");
        let copy = window
            .debug_bounds(copy_selector)
            .expect("fixed result copy rail");
        let title = window.debug_bounds(title_selector).expect("result title");
        let subtitle = window
            .debug_bounds(subtitle_selector)
            .expect("result subtitle");
        let shortcut = window
            .debug_bounds(shortcut_selector)
            .expect("fixed shortcut rail");

        assert_eq!(icon.size.width, px(LAUNCHER_RESULT_ICON_SIZE));
        assert_eq!(title.size.height, px(LAUNCHER_TITLE_LINE_HEIGHT));
        assert_eq!(subtitle.size.height, px(LAUNCHER_SUBTITLE_LINE_HEIGHT));
        assert_eq!(shortcut.size.width, px(48.0));
        let icon_rail = (icon.origin.x, icon.size.width);
        let copy_rail = (copy.origin.x, copy.size.width);
        let shortcut_rail = (shortcut.origin.x, shortcut.size.width);
        assert_eq!(*expected_icon.get_or_insert(icon_rail), icon_rail);
        assert_eq!(*expected_copy.get_or_insert(copy_rail), copy_rail);
        assert_eq!(
            *expected_shortcut.get_or_insert(shortcut_rail),
            shortcut_rail
        );
    }

    panel.update_in(window, |panel, window, cx| panel.focus_search(window, cx));
    window.simulate_keystrokes("ctrl-2");
    panel.read_with(cx, |panel, _| assert_eq!(panel.selected, 1));
    cx.run_until_parked();
    let command = (0..8)
        .filter_map(|_| endpoint.try_receive_command_for_contract_test().ok())
        .find(|envelope| {
            matches!(
                &envelope.message,
                nxtools_protocol::Message::PlatformCommand(
                    PlatformCommand::OpenShellTarget(open)
                ) if open.target == "file.lnk" && open.action == ShellOpenAction::Open
            )
        })
        .expect("typed command for the second visible result");
    let action_correlation_id = application.read_with(cx, |application, _| {
        let pending = application
            .pending_actions
            .get(&command.id)
            .expect("transport command id retains the pending action");
        assert_eq!(command.correlation_id, Some(pending.correlation_id));
        assert_eq!(pending.trigger, QuickLaunchActionTrigger::Shortcut);
        assert_eq!(pending.source, ApplicationSource::IndexedFile);
        assert_eq!(pending.action, QuickLaunchAction::Open);
        pending.correlation_id
    });
    application.update(cx, |application, cx| {
        application.handle_platform_notification(
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
        );
    });
    application.read_with(cx, |application, _| {
        assert!(!application.pending_actions.contains_key(&command.id));
        assert_ne!(action_correlation_id, command.id);
    });
}

#[gpui_kit::test]
fn production_popup_keeps_search_only_collapsed_and_expands_one_stable_surface(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_kit::init);
    let panel_slot = Rc::new(RefCell::new(None));
    let panel_slot_for_window = Rc::clone(&panel_slot);
    let (_root, window) = cx.add_window_view(move |window, cx| {
        window.resize(size(px(PANEL_WIDTH), px(COLLAPSED_PANEL_HEIGHT)));
        let panel = cx.new(|cx| {
            QuickLaunchPanel::new(
                Vec::new(),
                nxtools_protocol::ForegroundContext {
                    window_handle: 1,
                    focus_window_handle: 1,
                    process_id: 1,
                    process_name: "editor".into(),
                    window_title: "document".into(),
                },
                None,
                Vec::new(),
                WeakEntity::new_invalid(),
                window,
                cx,
            )
        });
        *panel_slot_for_window.borrow_mut() = Some(panel.clone());
        Root::new(panel, window, cx)
    });
    let panel = panel_slot
        .borrow_mut()
        .take()
        .expect("production quick-launch panel");

    window
        .refresh()
        .expect("render collapsed quick-launch popup");
    assert!(window.debug_bounds("quick-launch-window").is_some());
    let collapsed_header = window
        .debug_bounds("quick-launch-search-header")
        .expect("collapsed quick-launch search header");
    window.update(|window, _| {
        assert_eq!(
            window.bounds().size,
            size(px(PANEL_WIDTH), px(COLLAPSED_PANEL_HEIGHT))
        );
    });
    assert_eq!(collapsed_header.size.height, px(SEARCH_HEADER_HEIGHT));
    assert!(
        window
            .debug_bounds("quick-launch-results-surface")
            .is_none()
    );

    panel.update_in(window, |panel, window, cx| {
        panel.sync_window_content_size("zed", window, cx);
        cx.notify();
    });
    window
        .refresh()
        .expect("render expanded quick-launch popup");
    let expanded_root = window
        .debug_bounds("quick-launch-window")
        .expect("expanded quick-launch root");
    let expanded_header = window
        .debug_bounds("quick-launch-search-header")
        .expect("expanded quick-launch search header");
    let results_surface = window
        .debug_bounds("quick-launch-results-surface")
        .expect("expanded quick-launch results surface");
    window.update(|window, _| {
        assert_eq!(
            window.bounds().size,
            size(px(PANEL_WIDTH), px(PANEL_HEIGHT))
        );
    });
    assert_eq!(expanded_header, collapsed_header);
    assert_eq!(results_surface.top(), expanded_header.bottom());
    assert_eq!(results_surface.bottom(), expanded_root.bottom() - px(1.0));

    panel.update_in(window, |panel, window, cx| {
        let expanded_generation = panel.results_appearance_generation;
        panel.sync_window_content_size("zed editor", window, cx);
        assert_eq!(panel.results_appearance_generation, expanded_generation);

        let mode = QuickLaunchActionMode::enter(
            search_result("first-result", ApplicationSource::IndexedFile),
            "zed editor".into(),
            0,
        );
        let mode_id = mode.id();
        let level_id = mode.current_level_id();
        let entry_id = mode.target().entry.id.clone();
        panel.action_mode = Some(mode);
        panel.apply_shell_actions(mode_id, level_id, &entry_id, None, cx);
        assert_eq!(panel.results_appearance_generation, expanded_generation);
    });
}

#[test]
fn main_popup_search_uses_reference_sized_input_roles() {
    assert_eq!(SEARCH_INPUT_TEXT_SIZE, 26.0);
    assert_eq!(SEARCH_INPUT_LINE_HEIGHT, 34.0);
}

#[gpui_kit::test]
fn production_results_surface_uses_shared_empty_and_progress_states(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let panel_slot = Rc::new(RefCell::new(None));
    let panel_slot_for_window = Rc::clone(&panel_slot);
    let (_root, window) = cx.add_window_view(move |window, cx| {
        window.resize(size(px(PANEL_WIDTH), px(PANEL_HEIGHT)));
        let panel = cx.new(|cx| {
            let mut panel = QuickLaunchPanel::new(
                Vec::new(),
                nxtools_protocol::ForegroundContext {
                    window_handle: 1,
                    focus_window_handle: 1,
                    process_id: 1,
                    process_name: "editor".into(),
                    window_title: "document".into(),
                },
                None,
                Vec::new(),
                WeakEntity::new_invalid(),
                window,
                cx,
            );
            panel.results_visible = true;
            panel
        });
        *panel_slot_for_window.borrow_mut() = Some(panel.clone());
        Root::new(panel, window, cx)
    });
    let panel = panel_slot
        .borrow_mut()
        .take()
        .expect("production quick-launch panel");

    window.refresh().expect("render quick-launch empty state");
    window.update(|window, _| {
        assert_eq!(
            window.bounds().size,
            size(px(PANEL_WIDTH), px(PANEL_HEIGHT))
        );
    });
    let panel_surface = window
        .debug_bounds("quick-launch-window")
        .expect("quick-launch production surface");
    let empty = window
        .debug_bounds("quick-launch-results-empty")
        .expect("quick-launch shared empty state");
    let empty_icon = window
        .debug_bounds("quick-launch-results-empty-icon")
        .expect("quick-launch empty icon");
    let empty_title = window
        .debug_bounds("quick-launch-results-empty-title")
        .expect("quick-launch empty title");
    let empty_description = window
        .debug_bounds("quick-launch-results-empty-description")
        .expect("quick-launch empty description");
    assert_eq!(empty_icon.size, size(px(18.0), px(18.0)));
    assert_eq!(empty_title.size.height, px(20.0));
    assert_eq!(empty_description.size.height, px(20.0));
    assert_eq!(empty_title.left(), empty_description.left());
    assert_eq!(empty.left(), panel_surface.left() + px(1.0));
    assert_eq!(empty.right(), panel_surface.right() - px(1.0));

    panel.update_in(window, |panel, _, cx| {
        panel.search_pending = true;
        cx.notify();
    });
    window.refresh().expect("render newly pending quick launch");
    assert!(window.debug_bounds("quick-launch-results-empty").is_none());
    assert!(
        window
            .debug_bounds("quick-launch-search-feedback")
            .is_none()
    );
    assert_eq!(
        window.debug_bounds("quick-launch-window"),
        Some(panel_surface)
    );
}

pub(super) fn quick_launch_application_fixture(
    cx: &mut TestAppContext,
) -> (
    tempfile::TempDir,
    Entity<QuickLaunchApplication>,
    nxtools_ipc::PlatformServerEndpoint,
) {
    let temp = tempfile::tempdir().expect("temporary quick-launch application fixture");
    let catalog = FinderCatalog::open(temp.path().join("history.db"), &[])
        .expect("open quick-launch application fixture catalog");
    let config_path = temp.path().join("config.json");
    AppConfig::default()
        .save_to(&config_path)
        .expect("seed config fixture");
    let logs_path = temp.path().join("logs");
    let (application, endpoint) = cx.update(|cx| {
        let notifications = cx.new(|_| NotificationCenter::default());
        let (platform, endpoint) = nxtools_ipc::platform_bridge(8);
        endpoint.set_connected_for_contract_test(true);
        let (indexer, _indexer_endpoint) = crate::runtime::indexer_bridge(8);
        let application = cx.new(|_| {
            QuickLaunchApplication::new(
                catalog,
                Rc::new(platform),
                Rc::new(indexer),
                AppConfig::default(),
                config_path,
                logs_path,
                notifications,
            )
        });
        (application, endpoint)
    });
    (temp, application, endpoint)
}

#[gpui_kit::test]
fn u3_followup_recording_fault_notifies_once_when_double_control_resumes(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (_temp, application, endpoint) = quick_launch_application_fixture(cx);
    application.update(cx, |app, _| app.set_hotkey_recording(true));
    let start = endpoint
        .try_receive_command_for_contract_test()
        .expect("pause hotkey for recording");
    application.update(cx, |app, cx| {
        app.handle_platform_notification(
            PlatformNotification::CommandResult {
                correlation_id: start.id,
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
    let fault = || {
        PlatformNotification::Event(PlatformEvent::QuickLaunchKeyboardHealthChanged(
            QuickLaunchKeyboardHealthChanged {
                correlation_id: uuid::Uuid::new_v4(),
                state: QuickLaunchKeyboardHealthState::ThreadExited,
                native_error_code: 17,
            },
        ))
    };
    application.update(cx, |app, cx| app.handle_platform_notification(fault(), cx));
    cx.run_until_parked();
    assert!(cx.read(|cx| application.read(cx).keyboard_health_warning().is_none()));
    assert!(cx.read(|cx| {
        application
            .read(cx)
            .notifications
            .read(cx)
            .items()
            .is_empty()
    }));
    application.update(cx, |app, _| app.set_hotkey_recording(false));
    let stop = endpoint
        .try_receive_command_for_contract_test()
        .expect("resume hotkey after recording");
    cx.run_until_parked();
    assert!(
        cx.read(|cx| application
            .read(cx)
            .notifications
            .read(cx)
            .items()
            .is_empty()),
        "recording remains active until its native ACK"
    );
    application.update(cx, |app, cx| {
        app.handle_platform_notification(
            PlatformNotification::CommandResult {
                correlation_id: stop.id,
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
    cx.run_until_parked();
    let generation = cx.read(|cx| {
        let app = application.read(cx);
        assert!(
            app.keyboard_health_warning()
                .expect("Alert becomes eligible")
                .0
                .contains("17")
        );
        let notices = app.notifications.read(cx);
        assert_eq!(
            notices.items().len(),
            1,
            "one deferred notice after recording ends"
        );
        assert_eq!(notices.items()[0].title, "双击 Ctrl 已失效");
        notices.items()[0].generation()
    });
    application.update(cx, |app, cx| app.handle_platform_notification(fault(), cx));
    cx.run_until_parked();
    assert_eq!(
        cx.read(|cx| application.read(cx).notifications.read(cx).items()[0].generation()),
        generation,
        "repeated unavailable events do not restart the notification"
    );
    application.update(cx, |app, cx| {
        app.handle_platform_notification(
            PlatformNotification::Event(PlatformEvent::QuickLaunchKeyboardHealthChanged(
                QuickLaunchKeyboardHealthChanged {
                    correlation_id: uuid::Uuid::new_v4(),
                    state: QuickLaunchKeyboardHealthState::Reinstalled,
                    native_error_code: 0,
                },
            )),
            cx,
        )
    });
    application.update(cx, |app, cx| app.handle_platform_notification(fault(), cx));
    cx.run_until_parked();
    assert!(
        cx.read(|cx| application.read(cx).notifications.read(cx).items()[0].generation())
            > generation,
        "recovery starts a new notification episode"
    );
}

type KeyboardHealthCapturedEvents = std::sync::Arc<std::sync::Mutex<Vec<Vec<(String, String)>>>>;

#[derive(Clone)]
struct KeyboardHealthDiagnostics(KeyboardHealthCapturedEvents);

#[derive(Default)]
struct KeyboardHealthFields(Vec<(String, String)>);

impl tracing::field::Visit for KeyboardHealthFields {
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        self.0.push((field.name().to_owned(), value.to_owned()));
    }
    fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
        self.0.push((field.name().to_owned(), value.to_string()));
    }
    fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
        self.0.push((field.name().to_owned(), value.to_string()));
    }
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        self.0.push((field.name().to_owned(), format!("{value:?}")));
    }
}

impl tracing::Subscriber for KeyboardHealthDiagnostics {
    fn enabled(&self, _: &tracing::Metadata<'_>) -> bool {
        true
    }
    fn new_span(&self, _: &tracing::span::Attributes<'_>) -> tracing::span::Id {
        tracing::span::Id::from_u64(1)
    }
    fn record(&self, _: &tracing::span::Id, _: &tracing::span::Record<'_>) {}
    fn record_follows_from(&self, _: &tracing::span::Id, _: &tracing::span::Id) {}
    fn event(&self, event: &tracing::Event<'_>) {
        let mut fields = KeyboardHealthFields::default();
        event.record(&mut fields);
        if fields.0.iter().any(|(name, value)| {
            name == "event" && value == "quick_launch.keyboard_reenable.completed"
        }) {
            self.0.lock().expect("diagnostic records").push(fields.0);
        }
    }
    fn enter(&self, _: &tracing::span::Id) {}
    fn exit(&self, _: &tracing::span::Id) {}
}

#[gpui_kit::test]
fn u3_followup_disconnect_closes_only_pending_keyboard_reenable_diagnostic(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_kit::init);
    let (_temp, application, endpoint) = quick_launch_application_fixture(cx);
    let records = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    tracing::subscriber::with_default(KeyboardHealthDiagnostics(records.clone()), || {
        application.update(cx, |app, cx| {
            app.handle_platform_notification(
                PlatformNotification::Event(PlatformEvent::QuickLaunchKeyboardHealthChanged(
                    QuickLaunchKeyboardHealthChanged {
                        correlation_id: uuid::Uuid::new_v4(),
                        state: QuickLaunchKeyboardHealthState::InstallFailed,
                        native_error_code: 5,
                    },
                )),
                cx,
            )
        });
        application.update(cx, |app, cx| app.reenable_keyboard_hotkey(cx));
        let command = endpoint
            .try_receive_command_for_contract_test()
            .expect("accepted ConfigureQuickLaunch");
        let pending_id = command.id;
        assert!(cx.read(|cx| application.read(cx).pending_keyboard_reenable.is_some()));
        application.update(cx, |app, cx| {
            app.handle_platform_notification(
                PlatformNotification::Disconnected {
                    generation: 1,
                    pending_command_ids: vec![pending_id],
                },
                cx,
            )
        });
        application.update(cx, |app, cx| {
            app.handle_platform_notification(
                PlatformNotification::CommandResult {
                    correlation_id: pending_id,
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
        assert!(cx.read(|cx| application.read(cx).pending_keyboard_reenable.is_none()));
        let events = records.lock().expect("captured terminal events");
        let matching = events
            .iter()
            .filter(|fields| {
                fields.iter().any(|(key, value)| {
                    key == "correlation_id" && value.contains(&pending_id.to_string())
                })
            })
            .collect::<Vec<_>>();
        assert_eq!(
            matching.len(),
            1,
            "accepted retry has one correlated terminal on disconnect"
        );
        let terminal = matching[0];
        assert!(
            terminal
                .iter()
                .any(|(key, value)| key == "success" && value == "false")
        );
        assert!(
            terminal
                .iter()
                .any(|(key, value)| key == "failure_kind" && value == "disconnected")
        );
        assert!(
            terminal
                .iter()
                .any(|(key, value)| key == "elapsed_ms" && value.parse::<u64>().is_ok())
        );
    });
}

#[gpui_kit::test]
fn u3_followup_completed_keyboard_retry_has_no_second_disconnect_terminal(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (_temp, application, endpoint) = quick_launch_application_fixture(cx);
    let records = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    tracing::subscriber::with_default(KeyboardHealthDiagnostics(records.clone()), || {
        application.update(cx, |app, cx| {
            app.handle_platform_notification(
                PlatformNotification::Event(PlatformEvent::QuickLaunchKeyboardHealthChanged(
                    QuickLaunchKeyboardHealthChanged {
                        correlation_id: uuid::Uuid::new_v4(),
                        state: QuickLaunchKeyboardHealthState::InstallFailed,
                        native_error_code: 5,
                    },
                )),
                cx,
            )
        });
        application.update(cx, |app, cx| app.reenable_keyboard_hotkey(cx));
        let command = endpoint
            .try_receive_command_for_contract_test()
            .expect("accepted retry");
        application.update(cx, |app, cx| {
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
        application.update(cx, |app, cx| {
            app.handle_platform_notification(
                PlatformNotification::Disconnected {
                    generation: 1,
                    pending_command_ids: Vec::new(),
                },
                cx,
            )
        });
        let events = records.lock().expect("captured terminal events");
        let matching = events
            .iter()
            .filter(|fields| {
                fields.iter().any(|(key, value)| {
                    key == "correlation_id" && value.contains(&command.id.to_string())
                })
            })
            .collect::<Vec<_>>();
        assert_eq!(
            matching.len(),
            1,
            "matching result terminates retry once before disconnect"
        );
        assert!(
            matching[0]
                .iter()
                .any(|(key, value)| key == "success" && value == "true")
        );
    });
}

#[gpui_kit::test]
fn keyboard_health_new_thread_exit_outlives_earlier_successful_retry(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (_temp, application, endpoint) = quick_launch_application_fixture(cx);
    let health = |state, code| {
        PlatformNotification::Event(PlatformEvent::QuickLaunchKeyboardHealthChanged(
            QuickLaunchKeyboardHealthChanged {
                correlation_id: uuid::Uuid::new_v4(),
                state,
                native_error_code: code,
            },
        ))
    };
    application.update(cx, |app, cx| {
        app.handle_platform_notification(PlatformNotification::Connected, cx)
    });
    let initial = endpoint
        .try_receive_command_for_contract_test()
        .expect("initial configuration");
    while endpoint.try_receive_command_for_contract_test().is_ok() {}
    application.update(cx, |app, cx| {
        app.handle_platform_notification(
            PlatformNotification::CommandResult {
                correlation_id: initial.id,
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
    application.update(cx, |app, cx| {
        app.handle_platform_notification(
            health(QuickLaunchKeyboardHealthState::InstallFailed, 5),
            cx,
        );
        app.reenable_keyboard_hotkey(cx);
    });
    let retry = (0..8)
        .filter_map(|_| endpoint.try_receive_command_for_contract_test().ok())
        .find(|envelope| {
            matches!(
                envelope.message,
                nxtools_protocol::Message::PlatformCommand(PlatformCommand::ConfigureQuickLaunch(
                    _
                ))
            )
        })
        .expect("original configuration retry");
    assert!(matches!(
        retry.message,
        nxtools_protocol::Message::PlatformCommand(PlatformCommand::ConfigureQuickLaunch(_))
    ));
    application.update(cx, |app, cx| {
        app.handle_platform_notification(
            health(QuickLaunchKeyboardHealthState::Reinstalled, 0),
            cx,
        );
        app.handle_platform_notification(
            health(QuickLaunchKeyboardHealthState::ThreadExited, 11),
            cx,
        );
        app.handle_platform_notification(
            PlatformNotification::CommandResult {
                correlation_id: retry.id,
                result: PlatformCommandResult {
                    success: true,
                    error: None,
                    native_error_code: None,
                    data: None,
                },
            },
            cx,
        );
    });
    let (reason, pending) = cx
        .read(|cx| application.read(cx).keyboard_health_warning())
        .expect("new thread exit must survive old successful result");
    assert!(reason.contains("线程已退出") && reason.contains("11"));
    assert!(!pending);
}

#[gpui_kit::test]
fn keyboard_health_hello_unavailable_clears_on_ordinary_configuration_success(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_kit::init);
    let (_temp, application, endpoint) = quick_launch_application_fixture(cx);
    endpoint.set_quick_launch_panel_hotkey_available_for_contract_test(false);
    application.update(cx, |app, cx| {
        app.handle_platform_notification(PlatformNotification::Connected, cx)
    });
    let command = endpoint
        .try_receive_command_for_contract_test()
        .expect("automatic configuration");
    assert!(cx.read(|cx| application.read(cx).keyboard_health_warning().is_some()));
    application.update(cx, |app, cx| {
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
    assert!(
        cx.read(|cx| application.read(cx).keyboard_health_warning().is_none()),
        "successful ordinary configuration clears the old Hello capability snapshot warning"
    );
}

#[gpui_kit::test]
fn keyboard_health_ordinary_configuration_success_keeps_new_native_failure(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_kit::init);
    let (_temp, application, endpoint) = quick_launch_application_fixture(cx);
    endpoint.set_quick_launch_panel_hotkey_available_for_contract_test(false);
    application.update(cx, |app, cx| {
        app.handle_platform_notification(PlatformNotification::Connected, cx)
    });
    let command = endpoint
        .try_receive_command_for_contract_test()
        .expect("automatic configuration");
    application.update(cx, |app, cx| {
        app.handle_platform_notification(
            PlatformNotification::Event(PlatformEvent::QuickLaunchKeyboardHealthChanged(
                QuickLaunchKeyboardHealthChanged {
                    correlation_id: uuid::Uuid::new_v4(),
                    state: QuickLaunchKeyboardHealthState::ThreadExited,
                    native_error_code: 17,
                },
            )),
            cx,
        );
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
        );
    });
    let (reason, _) = cx
        .read(|cx| application.read(cx).keyboard_health_warning())
        .expect("new native failure must survive old ordinary configuration result");
    assert!(reason.contains("线程已退出") && reason.contains("17"));
}

pub(super) fn quick_launch_panel_fixture(
    cx: &mut TestAppContext,
    results: Vec<SearchResult>,
) -> (Entity<QuickLaunchPanel>, &mut gpui_kit::VisualTestContext) {
    let panel_slot = Rc::new(RefCell::new(None));
    let panel_slot_for_window = Rc::clone(&panel_slot);
    let (_root, window) = cx.add_window_view(move |window, cx| {
        window.resize(size(px(PANEL_WIDTH), px(PANEL_HEIGHT)));
        let panel = cx.new(|cx| {
            let mut panel = QuickLaunchPanel::new(
                results,
                nxtools_protocol::ForegroundContext {
                    window_handle: 1,
                    focus_window_handle: 1,
                    process_id: 1,
                    process_name: "editor".into(),
                    window_title: "document".into(),
                },
                None,
                Vec::new(),
                WeakEntity::new_invalid(),
                window,
                cx,
            );
            panel.results_visible = true;
            panel
        });
        *panel_slot_for_window.borrow_mut() = Some(panel.clone());
        Root::new(panel, window, cx)
    });
    let panel = panel_slot
        .borrow_mut()
        .take()
        .expect("production quick-launch panel");
    (panel, window)
}

pub(super) fn search_result(id: &str, source: ApplicationSource) -> SearchResult {
    SearchResult {
        entry: nxtools_finder::ApplicationEntry {
            id: id.into(),
            name: id.into(),
            target: format!("{id}.lnk"),
            source,
        },
        action: if source == ApplicationSource::Calculator {
            SearchResultAction::CopyText { text: id.into() }
        } else {
            SearchResultAction::OpenTarget
        },
        subtitle: None,
        score: 0,
        activation_count: 0,
        last_activated_at_unix_ms: None,
        query_activation_count: 0,
        last_query_activation_at_unix_ms: None,
    }
}

#[test]
fn result_shortcuts_address_only_the_visible_first_nine_rows() {
    let control = gpui_kit::Modifiers {
        control: true,
        ..Default::default()
    };

    assert_eq!(quick_launch_shortcut_hint(0), Some("Ctrl+1"));
    assert_eq!(quick_launch_shortcut_hint(8), Some("Ctrl+9"));
    assert_eq!(quick_launch_shortcut_hint(9), None);
    assert_eq!(
        quick_launch_shortcut_target("1", control, 5..13, 20),
        Some(5)
    );
    assert_eq!(
        quick_launch_shortcut_target("8", control, 5..13, 20),
        Some(12)
    );
    assert_eq!(quick_launch_shortcut_target("9", control, 5..13, 20), None);
    assert_eq!(
        quick_launch_shortcut_target("1", gpui_kit::Modifiers::default(), 5..13, 20),
        None
    );
}

fn finish_shell_loading(mode: &mut QuickLaunchActionMode) {
    let level_id = mode.current_level_id();
    mode.apply_shell_menu(level_id, None);
}

fn finish_shell_loading_with_navigation(
    mode: &mut QuickLaunchActionMode,
    navigation_directory: &str,
) {
    let level_id = mode.current_level_id();
    mode.apply_shell_menu(
        level_id,
        Some(ShellActionMenu {
            session_id: "navigation-session".into(),
            navigation_directory: Some(navigation_directory.into()),
            items: Vec::new(),
        }),
    );
}

#[test]
fn result_name_highlights_follow_contiguous_and_fuzzy_matches() {
    assert_eq!(quick_launch_match_ranges("AyuGram.exe", "gram"), vec![3..7]);
    assert_eq!(
        quick_launch_match_ranges("AyuGram.exe", "agm"),
        vec![0..1, 3..4, 6..7]
    );
}

#[test]
fn w1_result_highlights_share_pinyin_query_ranges_without_action_changes() {
    for (name, query) in [
        ("年度报告.docx", "baogao"),
        ("银行卡.txt", "yinhang"),
        ("V2报告", "v2bg"),
    ] {
        let parsed = nxtools_finder::FileQuery::parse(query);
        assert!(parsed.score_name(name).is_some());
        assert_eq!(
            quick_launch_match_ranges(name, query),
            parsed.match_ranges(name)
        );
        assert!(!quick_launch_match_ranges(name, query).is_empty());
    }
    assert_eq!(
        quick_launch_match_ranges("年度报告.docx", "baogao"),
        vec![6..12]
    );
    assert!(
        match_ranges_for_pattern("报告", "baogao").is_empty(),
        "actions remain literal"
    );
}

#[test]
fn quick_launch_settings_map_to_the_indexer_contract() {
    let settings = QuickLaunchSettings {
        indexed_volumes: vec!["C:\\".into(), "D:\\".into()],
        excluded_paths: vec!["C:\\Build".into(), "D:\\Cache".into()],
        ..QuickLaunchSettings::default()
    };

    assert_eq!(
        index_settings_from_quick_launch(&settings),
        IndexSettings {
            indexed_volumes: vec!["C:\\".into(), "D:\\".into()],
            excluded_paths: vec!["C:\\Build".into(), "D:\\Cache".into()],
        }
    );
}

#[test]
fn auto_quick_switch_request_becomes_one_file_dialog_navigation_command() {
    let settings = QuickLaunchSettings::default();
    let request = nxtools_protocol::QuickLaunchAutoSwitchRequest {
        target: nxtools_protocol::DirectoryNavigationContext {
            host: DirectoryNavigationHost::FileDialog,
            window_handle: 77,
            current_directory: Some(r"C:\Initial".into()),
            host_context_id: None,
        },
        source_directory: r"D:\Work".into(),
        requested_at_unix_ms: 10_000,
    };

    assert_eq!(
        auto_quick_switch_command(&settings, &request, 10_500),
        Some(NavigateDirectoryCommand {
            host: DirectoryNavigationHost::FileDialog,
            window_handle: 77,
            target_directory: r"D:\Work".into(),
            host_context_id: None,
        })
    );
    assert_eq!(auto_quick_switch_command(&settings, &request, 13_001), None);

    let mut disabled = settings;
    disabled.auto_quick_switch = false;
    assert_eq!(auto_quick_switch_command(&disabled, &request, 10_500), None);
}

#[test]
fn library_uri_is_a_navigable_directory_target_and_nested_paths_are_preserved() {
    assert!(is_navigable_directory_target("lib://Pictures"));
    assert!(is_navigable_directory_target(r"D:\Qingfeng"));
    assert!(!is_navigable_directory_target("lib://"));

    let context = nxtools_protocol::DirectoryNavigationContext {
        host: DirectoryNavigationHost::ThirdPartyFileManager,
        window_handle: 66,
        current_directory: Some(r"D:\Before".into()),
        host_context_id: Some("dopus-pane".into()),
    };
    let mut result = search_result("Qingfeng", ApplicationSource::IndexedDirectory);
    result.entry.target = r"D:\Qingfeng".into();
    let action = recent_directory_navigation_action(&result, Some(&context))
        .expect("nested directory must expose navigation");
    assert!(matches!(
        action,
        QuickLaunchAction::SwitchDirectory { target_directory, .. }
            if target_directory == r"D:\Qingfeng"
    ));
}

#[test]
fn icon_prefetch_tracks_the_visible_result_window_with_small_overscan() {
    assert_eq!(visible_icon_prefetch_range(0, 7, 50), 0..10);
    assert_eq!(visible_icon_prefetch_range(18, 25, 50), 16..28);
    assert_eq!(visible_icon_prefetch_range(47, 49, 50), 45..50);
}

#[test]
fn failed_visible_icon_requests_retry_after_a_bounded_cooldown() {
    let failed_at = Instant::now();

    assert!(icon_retry_pending(
        failed_at,
        failed_at + ICON_RETRY_DELAY / 2
    ));
    assert!(!icon_retry_pending(failed_at, failed_at + ICON_RETRY_DELAY));
}

#[test]
fn shell_icon_results_accept_valid_pngs_and_reject_undecodable_files() {
    let temp = tempfile::tempdir().expect("temporary Shell icon fixtures");
    let valid = temp.path().join("valid.png");
    let invalid = temp.path().join("invalid.png");
    let wrong_extension = temp.path().join("valid.svg");
    std::fs::write(&valid, crate::icons::ONE_PIXEL_PNG).expect("valid PNG fixture");
    std::fs::write(&invalid, b"not a decodable image").expect("invalid image fixture");
    std::fs::write(&wrong_extension, crate::icons::ONE_PIXEL_PNG).expect("wrong-extension fixture");

    assert!(shell_icon_path_is_renderable(&valid));
    assert!(!shell_icon_path_is_renderable(&invalid));
    assert!(!shell_icon_path_is_renderable(&wrong_extension));
    assert!(!shell_icon_path_is_renderable(
        &temp.path().join("missing.png")
    ));
}

#[test]
fn ordinary_indexed_files_reuse_one_type_icon_resource() {
    let mut first = search_result("first", ApplicationSource::IndexedFile);
    first.entry.target = r"C:\Images\first.PNG".into();
    let mut second = search_result("second", ApplicationSource::IndexedFile);
    second.entry.target = r"D:\Cache\second.png".into();
    let mut executable = search_result("tool", ApplicationSource::IndexedFile);
    executable.entry.target = r"C:\Tools\tool.exe".into();

    assert_eq!(
        icon_resource_key(&first, 32),
        icon_resource_key(&second, 32)
    );
    assert_ne!(
        icon_resource_key(&first, 32),
        icon_resource_key(&executable, 32)
    );
}

#[test]
fn icon_resource_cache_is_bounded_and_keeps_recent_entries() {
    let mut cache = IconResourceCache::bounded(2);
    cache.insert("first".into(), PathBuf::from("first.png"));
    cache.insert("second".into(), PathBuf::from("second.png"));
    assert_eq!(cache.get("first"), Some(PathBuf::from("first.png")));
    cache.insert("third".into(), PathBuf::from("third.png"));

    assert!(cache.get("second").is_none());
    assert_eq!(cache.get("first"), Some(PathBuf::from("first.png")));
    assert_eq!(cache.get("third"), Some(PathBuf::from("third.png")));
}

#[test]
fn action_mode_restores_the_parent_query_and_selection() {
    let target = search_result("alpha", ApplicationSource::StartMenu);
    let mode = QuickLaunchActionMode::enter(target.clone(), "alp".into(), 3);

    assert_eq!(mode.target().entry.id, target.entry.id);
    assert_eq!(
        mode.exit(),
        QuickLaunchResultsContext {
            query: "alp".into(),
            selected: 3,
            selected_entry_id: "alpha".into(),
        }
    );
}

#[test]
fn recent_directory_mode_filters_and_navigates_the_frozen_host() {
    let recent = vec![
        nxtools_finder::RecentDirectory {
            path: r"C:\Work\Reports".into(),
            visit_count: 3,
            last_visited_at_unix_ms: 30,
        },
        nxtools_finder::RecentDirectory {
            path: r"D:\Archive".into(),
            visit_count: 1,
            last_visited_at_unix_ms: 20,
        },
    ];
    let results = recent_directory_results(&recent, "repo");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].entry.target, r"C:\Work\Reports");
    assert_eq!(
        recent_directory_navigation_action(
            &results[0],
            Some(&nxtools_protocol::DirectoryNavigationContext {
                host: DirectoryNavigationHost::Desktop,
                window_handle: 66,
                current_directory: Some(r"C:\Users\Test\Desktop".into()),
                host_context_id: None,
            }),
        ),
        Some(QuickLaunchAction::SwitchDirectory {
            host: DirectoryNavigationHost::Desktop,
            window_handle: 66,
            target_directory: r"C:\Work\Reports".into(),
            host_context_id: None,
        })
    );
    assert_eq!(recent_directory_navigation_action(&results[0], None), None);
}

#[gpui_kit::test]
fn clearing_history_resets_ordinary_and_recent_panel_state(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let panel_slot = Rc::new(RefCell::new(None));
    let panel_slot_for_window = Rc::clone(&panel_slot);
    let (_root, window) = cx.add_window_view(move |window, cx| {
        window.resize(size(px(PANEL_WIDTH), px(PANEL_HEIGHT)));
        let panel = cx.new(|cx| {
            let mut panel = QuickLaunchPanel::new(
                vec![search_result("old", ApplicationSource::StartMenu)],
                nxtools_protocol::ForegroundContext {
                    window_handle: 1,
                    focus_window_handle: 1,
                    process_id: 1,
                    process_name: "editor".into(),
                    window_title: "document".into(),
                },
                Some(nxtools_protocol::DirectoryNavigationContext {
                    host: DirectoryNavigationHost::Desktop,
                    window_handle: 1,
                    current_directory: None,
                    host_context_id: None,
                }),
                vec![nxtools_finder::RecentDirectory {
                    path: r"C:\Work\Reports".into(),
                    visit_count: 2,
                    last_visited_at_unix_ms: 1,
                }],
                WeakEntity::new_invalid(),
                window,
                cx,
            );
            panel.results_visible = true;
            panel
        });
        *panel_slot_for_window.borrow_mut() = Some(panel.clone());
        Root::new(panel, window, cx)
    });
    let panel = panel_slot.borrow_mut().take().expect("quick-launch panel");

    panel.update_in(window, |panel, window, cx| {
        panel.search_input.update(cx, |input, cx| {
            input.set_value("old query", window, cx);
        });
        panel.query_dispatch_generation = 7;
        let (query, generation) = panel.reset_after_history_clear(window, cx);
        assert_eq!(query, "old query");
        assert_eq!(generation, 8);
        assert!(panel.results.is_empty());
        assert!(!panel.recent_directory_mode);
        assert!(panel.recent_directories.is_empty());
    });

    panel.update_in(window, |panel, window, cx| {
        panel.recent_directory_mode = true;
        panel.recent_directories = vec![nxtools_finder::RecentDirectory {
            path: r"C:\Work\Reports".into(),
            visit_count: 2,
            last_visited_at_unix_ms: 1,
        }];
        panel.results = recent_directory_results(&panel.recent_directories, "").into();
        panel.search_input.update(cx, |input, cx| {
            input.set_placeholder("筛选最近目录", window, cx);
            input.set_value("repo", window, cx);
        });
        let (query, _) = panel.reset_after_history_clear(window, cx);
        assert!(query.is_empty());
        assert!(panel.search_input.read(cx).value().is_empty());
        assert!(!panel.recent_directory_mode);
        assert!(panel.recent_directories.is_empty());
        assert!(panel.results.is_empty());
    });
}

#[test]
fn elevated_directory_helper_is_requested_once_after_both_processes_are_ready() {
    let request = ElevatedDirectoryHelperRequest {
        pipe_name: "nxtools-next-elevated-nav-0123456789abcdef".into(),
        authentication_token: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
            .into(),
    };
    assert_eq!(
        elevated_directory_helper_command(Some(&request), false, None, true),
        Some(IndexerCommand::EnsureElevatedDirectoryHelper(
            request.clone()
        ))
    );
    assert_eq!(
        elevated_directory_helper_command(Some(&request), true, None, true),
        None
    );
    assert_eq!(
        elevated_directory_helper_command(Some(&request), false, None, false),
        None
    );
    assert_eq!(
        elevated_directory_helper_command(Some(&request), false, Some(&request.pipe_name), true,),
        None
    );
    assert_eq!(
        elevated_directory_helper_command(None, false, None, true),
        None
    );
    assert!(elevated_helper_status_matches_request(
        Some(&request),
        &nxtools_protocol::ElevatedDirectoryHelperStatus {
            pipe_name: request.pipe_name.clone(),
            launched: true,
            error: None,
        },
    ));
    assert!(!elevated_helper_status_matches_request(
        Some(&request),
        &nxtools_protocol::ElevatedDirectoryHelperStatus {
            pipe_name: "nxtools-next-elevated-nav-stale".into(),
            launched: true,
            error: None,
        },
    ));
}

#[test]
fn deferred_directory_context_updates_only_the_matching_summon() {
    let resolution = nxtools_protocol::QuickLaunchDirectoryContextResolution {
        target_window_handle: 88,
        requested_at_unix_ms: 1_700_000_000_000,
        directory_navigation_context: nxtools_protocol::DirectoryNavigationContext {
            host: DirectoryNavigationHost::ThirdPartyFileManager,
            window_handle: 88,
            current_directory: Some(r"D:\Elevated".into()),
            host_context_id: Some("elevated:request-88".into()),
        },
    };
    assert!(directory_context_resolution_matches(
        Some((88, 1_700_000_000_000)),
        &resolution,
        1_700_000_000_500,
    ));
    assert!(!directory_context_resolution_matches(
        Some((77, 1_700_000_000_000)),
        &resolution,
        1_700_000_000_500,
    ));
    assert!(!directory_context_resolution_matches(
        None,
        &resolution,
        1_700_000_000_500,
    ));
    assert!(!directory_context_resolution_matches(
        Some((88, 1_700_000_000_000)),
        &resolution,
        1_700_000_001_001,
    ));
}

#[gpui_kit::test]
fn captured_directory_reaches_sqlite_after_the_popup_session_changes(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let temp = tempfile::tempdir().expect("temporary recent-directory catalog");
    let current_directory = temp.path().join("Current");
    std::fs::create_dir_all(&current_directory).expect("captured current directory");
    let history_path = temp.path().join("history.db");
    let catalog = FinderCatalog::open(&history_path, &[]).expect("open recent-directory catalog");
    let application = cx.update(|cx| {
        let notifications = cx.new(|_| NotificationCenter::default());
        let (platform, _endpoint) = nxtools_ipc::platform_bridge(8);
        let (indexer, _indexer_endpoint) = crate::runtime::indexer_bridge(8);
        cx.new(|_| {
            QuickLaunchApplication::new(
                catalog,
                Rc::new(platform),
                Rc::new(indexer),
                AppConfig::default(),
                temp.path().join("config.json"),
                temp.path().join("logs"),
                notifications,
            )
        })
    });
    let context = nxtools_protocol::DirectoryNavigationContext {
        host: DirectoryNavigationHost::Desktop,
        window_handle: 66,
        current_directory: Some(current_directory.to_string_lossy().into_owned()),
        host_context_id: None,
    };

    application.update(cx, |application, cx| {
        application.schedule_captured_directory_record(Some(&context), cx);
        application.popup_session_id = uuid::Uuid::new_v4();
    });
    cx.run_until_parked();

    application.read_with(cx, |application, _| {
        let recent = application
            .catalog
            .borrow()
            .recent_directories(RECENT_DIRECTORY_LIMIT);
        assert_eq!(recent.len(), 1);
        assert_eq!(Path::new(&recent[0].path), current_directory.as_path());
    });
    let reopened =
        FinderCatalog::open(&history_path, &[]).expect("reopen persisted directory history");
    assert_eq!(
        reopened
            .recent_directories(RECENT_DIRECTORY_LIMIT)
            .first()
            .map(|entry| Path::new(&entry.path)),
        Some(current_directory.as_path())
    );
}

#[gpui_kit::test]
fn clearing_history_defers_feedback_and_persists_the_empty_state(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let temp = tempfile::tempdir().expect("temporary quick-launch history");
    let recent_directory = temp.path().join("Recent");
    std::fs::create_dir_all(&recent_directory).expect("recent directory");
    let history_path = temp.path().join("history.db");
    let mut catalog = FinderCatalog::open(&history_path, &[]).expect("open history");
    catalog
        .record_activation("editor", "edit", 1_700_000_000_000)
        .expect("record activation");
    catalog
        .record_directory_visit(&recent_directory, 1_700_000_001_000)
        .expect("record directory");
    let (application, notifications) = cx.update(|cx| {
        let notifications = cx.new(|_| NotificationCenter::default());
        let application_notifications = notifications.clone();
        let (platform, _endpoint) = nxtools_ipc::platform_bridge(8);
        let (indexer, _indexer_endpoint) = crate::runtime::indexer_bridge(8);
        let application = cx.new(|_| {
            QuickLaunchApplication::new(
                catalog,
                Rc::new(platform),
                Rc::new(indexer),
                AppConfig::default(),
                temp.path().join("config.json"),
                temp.path().join("logs"),
                application_notifications,
            )
        });
        (application, notifications)
    });

    cx.update(|cx| {
        application.update(cx, |application, cx| {
            assert_eq!(application.history_summary().total_activations, 1);
            let old_request = application.index_search.begin("old", INDEX_CANDIDATE_LIMIT);
            application
                .index_search
                .apply(nxtools_protocol::IndexSearchBatch {
                    generation: old_request.generation,
                    items: vec![nxtools_protocol::IndexSearchItem {
                        size_bytes: None,
                        modified_at_unix_ms: None,
                        score: 0,
                        name: "old-result".into(),
                        path: r"C:\old-result.txt".into(),
                        is_directory: false,
                    }],
                    complete: true,
                    elapsed_us: 12,
                });
            let old_generation = application.index_search.generation;
            application.clear_history(cx);
            assert_eq!(
                application.history_summary(),
                QuickLaunchHistorySummary::default()
            );
            assert!(application.index_search.generation > old_generation);
            assert!(application.index_search.query.is_empty());
            assert!(application.index_search.items.is_empty());
        });
        assert!(
            notifications.read(cx).items().is_empty(),
            "cross-entity feedback must wait for the next application turn"
        );
    });

    cx.run_until_parked();
    notifications.read_with(cx, |notifications, _| {
        assert_eq!(notifications.items().len(), 1);
        assert_eq!(notifications.items()[0].title, "速启历史已清空");
    });
    let reopened = FinderCatalog::open(&history_path, &[]).expect("reopen cleared history");
    assert_eq!(
        reopened.history_summary(),
        QuickLaunchHistorySummary::default()
    );
}

#[test]
fn action_mode_restores_the_same_result_after_a_background_reorder() {
    let target = search_result("alpha", ApplicationSource::StartMenu);
    let context = QuickLaunchActionMode::enter(target, "a".into(), 1).exit();
    let refreshed = vec![
        search_result("beta", ApplicationSource::StartMenu),
        search_result("alpha", ApplicationSource::StartMenu),
    ];

    assert_eq!(selection_for_results_context(&refreshed, &context), 1);
}

#[test]
fn action_mode_filters_only_actions_available_for_the_target() {
    let mut shortcut = QuickLaunchActionMode::enter(
        search_result("alpha", ApplicationSource::StartMenu),
        String::new(),
        0,
    );
    finish_shell_loading(&mut shortcut);
    assert_eq!(
        shortcut
            .visible_actions()
            .iter()
            .map(|item| item.label.as_str())
            .collect::<Vec<_>>(),
        vec!["打开", "打开文件所在位置", "复制路径", "以管理员身份运行"]
    );

    shortcut.set_filter("管理");
    assert_eq!(
        shortcut
            .visible_actions()
            .iter()
            .filter_map(|item| item.action.clone())
            .collect::<Vec<_>>(),
        vec![QuickLaunchAction::RunElevated]
    );

    let mut system = QuickLaunchActionMode::enter(
        search_result("settings", ApplicationSource::System),
        String::new(),
        0,
    );
    finish_shell_loading(&mut system);
    assert!(
        !system
            .visible_actions()
            .iter()
            .any(|item| item.action == Some(QuickLaunchAction::Reveal))
    );

    let mut website = search_result("browser", ApplicationSource::StartMenu);
    website.entry.target = "browser.url".into();
    let mut website = QuickLaunchActionMode::enter(website, String::new(), 0);
    finish_shell_loading(&mut website);
    assert!(
        !website
            .visible_actions()
            .iter()
            .any(|item| item.action == Some(QuickLaunchAction::RunElevated))
    );
}

#[test]
fn action_mode_exposes_copy_path_for_every_result_target() {
    for source in [
        ApplicationSource::StartMenu,
        ApplicationSource::System,
        ApplicationSource::IndexedFile,
        ApplicationSource::IndexedDirectory,
    ] {
        let mut mode =
            QuickLaunchActionMode::enter(search_result("alpha", source), String::new(), 0);
        finish_shell_loading(&mut mode);

        assert!(
            mode.visible_actions()
                .iter()
                .any(|item| item.label == "复制路径"),
            "copy-path action missing for {source:?}"
        );
    }
}

#[test]
fn navigation_context_switches_files_to_their_parent_directory() {
    let context = nxtools_protocol::DirectoryNavigationContext {
        host: DirectoryNavigationHost::Explorer,
        window_handle: 88,
        current_directory: Some(r"C:\Before".into()),
        host_context_id: None,
    };
    let mut folder_result = search_result("Reports", ApplicationSource::IndexedDirectory);
    folder_result.entry.target = r"C:\Work\Reports".into();
    let mut folder = QuickLaunchActionMode::enter_in_context(
        folder_result.clone(),
        String::new(),
        0,
        Some(&context),
    );
    finish_shell_loading_with_navigation(&mut folder, r"C:\Work\Reports");
    assert!(folder.visible_actions().iter().any(|item| {
        item.action
            == Some(QuickLaunchAction::SwitchDirectory {
                host: DirectoryNavigationHost::Explorer,
                window_handle: 88,
                target_directory: r"C:\Work\Reports".into(),
                host_context_id: None,
            })
    }));

    let mut file_result = search_result("report.txt", ApplicationSource::IndexedFile);
    file_result.entry.target = r"C:\Work\Reports\report.txt".into();
    let mut file = QuickLaunchActionMode::enter_in_context(
        file_result.clone(),
        String::new(),
        0,
        Some(&context),
    );
    finish_shell_loading_with_navigation(&mut file, r"C:\Work\Reports");
    let file_action = file
        .visible_actions()
        .into_iter()
        .find_map(|item| match item.action {
            Some(action @ QuickLaunchAction::SwitchDirectory { .. }) => Some(action),
            _ => None,
        })
        .expect("file result must expose parent-directory navigation");
    let PlatformCommand::NavigateDirectory(file_command) =
        platform_command_for_action(&file_result, &file_action, 999)
            .expect("file parent-directory command")
    else {
        panic!("file navigation must use the directory adapter");
    };
    assert_eq!(file_command.target_directory, r"C:\Work\Reports");

    let mut ordinary = QuickLaunchActionMode::enter(folder_result, String::new(), 0);
    finish_shell_loading(&mut ordinary);
    assert!(
        !ordinary
            .visible_actions()
            .iter()
            .any(|item| { matches!(item.action, Some(QuickLaunchAction::SwitchDirectory { .. })) })
    );
}

#[test]
fn file_dialog_navigation_is_available_for_local_application_results() {
    let context = nxtools_protocol::DirectoryNavigationContext {
        host: DirectoryNavigationHost::FileDialog,
        window_handle: 77,
        current_directory: Some(r"C:\Before".into()),
        host_context_id: None,
    };
    let mut application = search_result("AMD Software", ApplicationSource::StartMenu);
    application.entry.target = r"C:\Program Files\AMD\CNext\CNext\RadeonSoftware.exe".into();
    let mut mode = QuickLaunchActionMode::enter_in_context(
        application.clone(),
        String::new(),
        0,
        Some(&context),
    );
    finish_shell_loading_with_navigation(&mut mode, r"C:\Program Files\AMD\CNext\CNext");

    let action = mode
        .visible_actions()
        .into_iter()
        .find_map(|item| match item.action {
            Some(action @ QuickLaunchAction::SwitchDirectory { .. }) => Some(action),
            _ => None,
        })
        .expect("a local application result must expose file-dialog directory navigation");
    let PlatformCommand::NavigateDirectory(command) =
        platform_command_for_action(&application, &action, 999)
            .expect("local application directory command")
    else {
        panic!("application navigation must use the directory adapter");
    };
    assert_eq!(
        command.target_directory,
        r"C:\Program Files\AMD\CNext\CNext"
    );
}

#[test]
fn shortcut_navigation_uses_the_resolved_target_directory_without_changing_launch_target() {
    let context = nxtools_protocol::DirectoryNavigationContext {
        host: DirectoryNavigationHost::FileDialog,
        window_handle: 77,
        current_directory: Some(r"C:\Before".into()),
        host_context_id: None,
    };
    let mut shortcut = search_result("AMD Software", ApplicationSource::StartMenu);
    shortcut.entry.target =
        r"C:\ProgramData\Microsoft\Windows\Start Menu\Programs\AMD Software.lnk".into();
    let launch_target = shortcut.entry.target.clone();
    let mut mode =
        QuickLaunchActionMode::enter_in_context(shortcut.clone(), String::new(), 0, Some(&context));
    let level_id = mode.current_level_id();
    mode.apply_shell_menu(
        level_id,
        Some(ShellActionMenu {
            session_id: "shortcut-session".into(),
            navigation_directory: Some(r"C:\Program Files\AMD\CNext\CNext".into()),
            items: Vec::new(),
        }),
    );

    let action = mode
        .visible_actions()
        .into_iter()
        .find_map(|item| match item.action {
            Some(action @ QuickLaunchAction::SwitchDirectory { .. }) => Some(action),
            _ => None,
        })
        .expect("resolved shortcut must expose target-directory navigation");
    let PlatformCommand::NavigateDirectory(command) =
        platform_command_for_action(&shortcut, &action, 999)
            .expect("resolved shortcut directory command")
    else {
        panic!("shortcut navigation must use the directory adapter");
    };
    assert_eq!(
        command.target_directory,
        r"C:\Program Files\AMD\CNext\CNext"
    );

    let PlatformCommand::OpenShellTarget(open) =
        platform_command_for_action(&shortcut, &QuickLaunchAction::Open, 999)
            .expect("shortcut launch command")
    else {
        panic!("normal shortcut launch must use the Shell target adapter");
    };
    assert_eq!(open.target, launch_target);
}

#[test]
fn file_dialog_context_uses_the_captured_dialog_for_folder_results() {
    let context = nxtools_protocol::DirectoryNavigationContext {
        host: DirectoryNavigationHost::FileDialog,
        window_handle: 77,
        current_directory: Some(r"C:\Before".into()),
        host_context_id: None,
    };
    let mut result = search_result("Reports", ApplicationSource::IndexedDirectory);
    result.entry.target = r"C:\Work\Reports".into();
    let mut mode =
        QuickLaunchActionMode::enter_in_context(result.clone(), String::new(), 0, Some(&context));
    finish_shell_loading_with_navigation(&mut mode, r"C:\Work\Reports");
    let action = mode
        .visible_actions()
        .into_iter()
        .find_map(|item| item.action)
        .expect("file-dialog directory action");
    assert_eq!(
        action,
        QuickLaunchAction::SwitchDirectory {
            host: DirectoryNavigationHost::FileDialog,
            window_handle: 77,
            target_directory: r"C:\Work\Reports".into(),
            host_context_id: None,
        }
    );

    let command =
        platform_command_for_action(&result, &action, 999).expect("file-dialog navigation command");
    let PlatformCommand::NavigateDirectory(command) = command else {
        panic!("switch-directory must use the directory adapter");
    };
    assert_eq!(command.host, DirectoryNavigationHost::FileDialog);
    assert_eq!(command.window_handle, 77);
    assert_eq!(command.target_directory, result.entry.target);
}

#[test]
fn file_navigation_uses_absolute_parents_for_every_directory_host() {
    let cases = [
        (r"C:\Work\Reports\report.txt", r"C:\Work\Reports"),
        (r"C:\report.txt", r"C:\"),
        (r"\\server\share\report.txt", r"\\server\share"),
    ];
    for (host, window_handle) in [
        (DirectoryNavigationHost::Explorer, 88),
        (DirectoryNavigationHost::FileDialog, 77),
        (DirectoryNavigationHost::ThirdPartyFileManager, 66),
    ] {
        let context = nxtools_protocol::DirectoryNavigationContext {
            host,
            window_handle,
            current_directory: Some(r"C:\Before".into()),
            host_context_id: (host == DirectoryNavigationHost::ThirdPartyFileManager)
                .then(|| "pane-context-66".into()),
        };
        for (target, expected_parent) in cases {
            let mut result = search_result("report.txt", ApplicationSource::IndexedFile);
            result.entry.target = target.into();
            let mut mode = QuickLaunchActionMode::enter_in_context(
                result.clone(),
                String::new(),
                0,
                Some(&context),
            );
            finish_shell_loading_with_navigation(&mut mode, expected_parent);
            let action = mode
                .visible_actions()
                .into_iter()
                .find_map(|item| match item.action {
                    Some(action @ QuickLaunchAction::SwitchDirectory { .. }) => Some(action),
                    _ => None,
                })
                .expect("absolute file result must expose parent-directory navigation");
            let PlatformCommand::NavigateDirectory(command) =
                platform_command_for_action(&result, &action, 999)
                    .expect("absolute file parent-directory command")
            else {
                panic!("file navigation must use the directory adapter");
            };
            assert_eq!(command.host, host);
            assert_eq!(command.window_handle, window_handle);
            assert_eq!(command.host_context_id, context.host_context_id);
            assert_eq!(
                Path::new(&command.target_directory),
                Path::new(expected_parent)
            );
            assert_ne!(command.target_directory, result.entry.target);
        }
    }
}

#[test]
fn relative_files_and_missing_context_do_not_expose_directory_navigation() {
    let context = nxtools_protocol::DirectoryNavigationContext {
        host: DirectoryNavigationHost::FileDialog,
        window_handle: 77,
        current_directory: Some(r"C:\Before".into()),
        host_context_id: None,
    };
    for target in ["report.txt", r"Reports\report.txt", r"C:report.txt"] {
        let mut result = search_result("report.txt", ApplicationSource::IndexedFile);
        result.entry.target = target.into();
        let mut mode =
            QuickLaunchActionMode::enter_in_context(result, String::new(), 0, Some(&context));
        finish_shell_loading(&mut mode);
        assert!(!mode.visible_actions().iter().any(|item| {
            matches!(item.action, Some(QuickLaunchAction::SwitchDirectory { .. }))
        }));
    }

    for rejected_directory in [None, Some("relative"), Some(r"C:relative")] {
        let mut unresolved = search_result("Editor", ApplicationSource::StartMenu);
        unresolved.entry.target = r"C:\Apps\Editor.exe --safe-mode".into();
        let mut mode =
            QuickLaunchActionMode::enter_in_context(unresolved, String::new(), 0, Some(&context));
        let level_id = mode.current_level_id();
        mode.apply_shell_menu(
            level_id,
            Some(ShellActionMenu {
                session_id: "rejected-navigation".into(),
                navigation_directory: rejected_directory.map(str::to_owned),
                items: Vec::new(),
            }),
        );
        assert!(!mode.visible_actions().iter().any(|item| {
            matches!(item.action, Some(QuickLaunchAction::SwitchDirectory { .. }))
        }));
    }

    let mut absolute_file = search_result("report.txt", ApplicationSource::IndexedFile);
    absolute_file.entry.target = r"C:\Work\Reports\report.txt".into();
    let mut unresolved = QuickLaunchActionMode::enter_in_context(
        absolute_file.clone(),
        String::new(),
        0,
        Some(&context),
    );
    finish_shell_loading(&mut unresolved);
    assert!(
        !unresolved
            .visible_actions()
            .iter()
            .any(|item| { matches!(item.action, Some(QuickLaunchAction::SwitchDirectory { .. })) })
    );

    let mut ordinary = QuickLaunchActionMode::enter(absolute_file, String::new(), 0);
    finish_shell_loading_with_navigation(&mut ordinary, r"C:\Work\Reports");
    assert!(
        !ordinary
            .visible_actions()
            .iter()
            .any(|item| { matches!(item.action, Some(QuickLaunchAction::SwitchDirectory { .. })) })
    );
}

#[test]
fn switch_directory_command_uses_the_window_captured_at_summon() {
    let mut result = search_result("Reports", ApplicationSource::IndexedDirectory);
    result.entry.target = r"C:\Work\Reports".into();
    let command = platform_command_for_action(
        &result,
        &QuickLaunchAction::SwitchDirectory {
            host: DirectoryNavigationHost::Explorer,
            window_handle: 88,
            target_directory: result.entry.target.clone(),
            host_context_id: None,
        },
        999,
    )
    .expect("Explorer navigation command");

    let PlatformCommand::NavigateDirectory(command) = command else {
        panic!("switch-directory must use the directory adapter");
    };
    assert_eq!(command.host, DirectoryNavigationHost::Explorer);
    assert_eq!(command.window_handle, 88);
    assert_eq!(command.target_directory, result.entry.target);
}

#[test]
fn copy_path_action_builds_a_copy_only_clipboard_command() {
    let result = search_result("alpha", ApplicationSource::IndexedFile);

    let command = platform_command_for_action(&result, &QuickLaunchAction::CopyPath, 0)
        .expect("copy-path command");

    let PlatformCommand::PasteText(command) = command else {
        panic!("copy-path must use the Windows clipboard adapter");
    };
    assert_eq!(command.text, result.entry.target);
    assert!(!command.paste_after_write);
    assert_eq!(command.target_window_handle, 0);
    assert_eq!(command.target_focus_window_handle, 0);
}

#[test]
fn calculator_enter_uses_copy_result_without_searching_the_file_index() {
    let mut result = search_result("calculator", ApplicationSource::Calculator);
    result.entry.name = "6 * 7 = 42".into();
    result.entry.target.clear();
    result.action = SearchResultAction::CopyText { text: "42".into() };

    assert_eq!(
        quick_launch_primary_action(&result, false, false),
        QuickLaunchAction::CopyResult
    );
    assert!(!query_requests_file_index("=6 * 7"));
    assert!(query_requests_file_index("6 * 7"));

    let command = platform_command_for_action(&result, &QuickLaunchAction::CopyResult, 0)
        .expect("copy-result command");
    let PlatformCommand::PasteText(command) = command else {
        panic!("calculator must use the Windows clipboard adapter");
    };
    assert_eq!(command.text, "42");
    assert!(!command.paste_after_write);
    assert_eq!(
        action_success_policy(&QuickLaunchAction::CopyResult),
        ActionSuccessPolicy::HideWithoutRecording
    );
}

#[test]
fn custom_command_uses_typed_launch_and_local_actions_only() {
    let result = SearchResult {
        entry: nxtools_finder::ApplicationEntry {
            id: "custom-command:tool".into(),
            name: "Run tool".into(),
            target: r"C:\Tools\tool.exe".into(),
            source: ApplicationSource::CustomCommand,
        },
        action: SearchResultAction::RunCommand {
            command: nxtools_finder::PreparedCustomCommand {
                executable: r"C:\Tools\tool.exe".into(),
                arguments: vec!["--name".into(), "two words".into()],
                working_directory: Some(r"D:\Work".into()),
                run_as_admin: true,
                run_silently: true,
            },
        },
        subtitle: Some(r#"C:\Tools\tool.exe --name "two words""#.into()),
        score: 10_000,
        activation_count: 0,
        last_activated_at_unix_ms: None,
        query_activation_count: 0,
        last_query_activation_at_unix_ms: None,
    };

    assert_eq!(
        quick_launch_primary_action(&result, false, false),
        QuickLaunchAction::Open
    );
    assert_eq!(
        quick_launch_result_capabilities(&result),
        QuickLaunchResultCapabilities {
            preview: false,
            shell_actions: false,
            shell_icon: true,
        }
    );
    let mode = QuickLaunchActionMode::enter(result.clone(), "tool arg".into(), 0);
    assert!(!mode.current_level().shell_pending);
    assert_eq!(
        mode.visible_actions()
            .iter()
            .map(|item| (item.label.as_str(), item.action.clone()))
            .collect::<Vec<_>>(),
        vec![
            ("运行命令", Some(QuickLaunchAction::Open)),
            ("复制命令", Some(QuickLaunchAction::CopyCommand)),
        ]
    );

    let launch = platform_command_for_action(&result, &QuickLaunchAction::Open, 0)
        .expect("typed launch command");
    let PlatformCommand::LaunchProcess(launch) = launch else {
        panic!("custom command must use the typed process adapter");
    };
    assert_eq!(launch.executable, r"C:\Tools\tool.exe");
    assert_eq!(launch.arguments, ["--name", "two words"]);
    assert_eq!(launch.working_directory.as_deref(), Some(r"D:\Work"));
    assert!(launch.run_as_admin);
    assert!(launch.run_silently);

    let copy = platform_command_for_action(&result, &QuickLaunchAction::CopyCommand, 0)
        .expect("copy command");
    let PlatformCommand::PasteText(copy) = copy else {
        panic!("copy command must use the clipboard adapter");
    };
    assert_eq!(copy.text, result.subtitle.expect("display line"));
    assert!(!copy.paste_after_write);
}

#[test]
fn environment_variable_uses_copy_action_without_file_integration() {
    let result = SearchResult {
        entry: nxtools_finder::ApplicationEntry {
            id: "environment-variable:prompt".into(),
            name: "%PROMPT%".into(),
            target: String::new(),
            source: ApplicationSource::EnvironmentVariable,
        },
        action: SearchResultAction::CopyText {
            text: "$P$G".into(),
        },
        subtitle: Some("$P$G".into()),
        score: 1_000,
        activation_count: 0,
        last_activated_at_unix_ms: None,
        query_activation_count: 0,
        last_query_activation_at_unix_ms: None,
    };

    assert!(!query_requests_file_index("%PATH%"));
    assert!(!query_requests_file_index(
        "%NXTOOLS_VARIABLE_THAT_DOES_NOT_EXIST_7D3E2A1C%"
    ));
    assert!(query_requests_file_index("%PATH"));
    assert_eq!(source_icon(result.entry.source), MaterialIcon::DataObject);
    assert_eq!(
        quick_launch_result_capabilities(&result),
        QuickLaunchResultCapabilities {
            preview: false,
            shell_actions: false,
            shell_icon: false,
        }
    );
    assert_eq!(
        quick_launch_primary_action(&result, false, false),
        QuickLaunchAction::CopyResult
    );
    let command = platform_command_for_action(&result, &QuickLaunchAction::CopyResult, 0)
        .expect("environment variable copy command");
    let PlatformCommand::PasteText(command) = command else {
        panic!("environment variable must use the clipboard adapter");
    };
    assert_eq!(command.text, "$P$G");

    let mode = QuickLaunchActionMode::enter(result, "%PATH%".into(), 0);
    assert!(!mode.current_level().shell_pending);
    assert_eq!(
        mode.visible_actions()
            .iter()
            .map(|item| (item.label.as_str(), item.action.clone()))
            .collect::<Vec<_>>(),
        vec![("复制变量值", Some(QuickLaunchAction::CopyResult))]
    );
}

#[test]
fn environment_directory_uses_open_and_native_file_actions() {
    let result = SearchResult {
        entry: nxtools_finder::ApplicationEntry {
            id: "environment-variable:temp:0".into(),
            name: r"C:\Temp".into(),
            target: r"C:\Temp".into(),
            source: ApplicationSource::EnvironmentVariable,
        },
        action: SearchResultAction::OpenTarget,
        subtitle: Some("%TEMP% · 打开文件夹".into()),
        score: 1_000,
        activation_count: 0,
        last_activated_at_unix_ms: None,
        query_activation_count: 0,
        last_query_activation_at_unix_ms: None,
    };

    assert_eq!(
        quick_launch_primary_action(&result, false, false),
        QuickLaunchAction::Open
    );
    assert_eq!(
        quick_launch_result_capabilities(&result),
        QuickLaunchResultCapabilities {
            preview: true,
            shell_actions: true,
            shell_icon: true,
        }
    );
    assert_eq!(
        platform_command_for_action(&result, &QuickLaunchAction::Open, 42),
        Some(PlatformCommand::OpenShellTarget(OpenShellTargetCommand {
            target: r"C:\Temp".into(),
            action: ShellOpenAction::Open,
        }))
    );

    let mut mode = QuickLaunchActionMode::enter(result, "%TEMP%".into(), 0);
    assert!(mode.current_level().shell_pending);
    finish_shell_loading(&mut mode);
    let actions = mode.visible_actions();
    assert!(actions.iter().any(|item| item.label == "打开"));
    assert!(actions.iter().any(|item| item.label == "复制路径"));
    assert!(!actions.iter().any(|item| item.label == "复制变量值"));
}

#[test]
fn calculator_action_mode_has_one_local_action_and_no_shell_loading() {
    let result = search_result("calculator", ApplicationSource::Calculator);
    let mode = QuickLaunchActionMode::enter(result, "6 * 7".into(), 0);

    assert!(!mode.current_level().shell_pending);
    let actions = mode.visible_actions();
    assert_eq!(actions.len(), 1);
    assert_eq!(actions[0].label, "复制结果");
    assert_eq!(actions[0].action, Some(QuickLaunchAction::CopyResult));
    assert_eq!(
        source_icon(ApplicationSource::Calculator),
        MaterialIcon::Calculate
    );
}

#[gpui_kit::test]
fn copy_actions_dispatch_without_hiding_the_active_popup(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (_temp, application, endpoint) = quick_launch_application_fixture(cx);
    let panel_slot = Rc::new(RefCell::new(None));
    let panel_for_window = Rc::clone(&panel_slot);
    let window_slot = Rc::new(RefCell::new(None));
    let window_for_window = Rc::clone(&window_slot);
    let application_for_window = application.clone();
    let (_root, window) = cx.add_window_view(move |window, cx| {
        *window_for_window.borrow_mut() = Some(window.window_handle());
        let panel = cx.new(|cx| {
            QuickLaunchPanel::new(
                Vec::new(),
                nxtools_protocol::ForegroundContext {
                    window_handle: 1,
                    focus_window_handle: 1,
                    process_id: 1,
                    process_name: "editor".into(),
                    window_title: String::new(),
                },
                None,
                Vec::new(),
                application_for_window.downgrade(),
                window,
                cx,
            )
        });
        *panel_for_window.borrow_mut() = Some(panel.clone());
        Root::new(panel, window, cx)
    });
    let panel = panel_slot
        .borrow_mut()
        .take()
        .expect("mounted launcher panel");
    let popup_window = window_slot
        .borrow_mut()
        .take()
        .expect("launcher window handle");
    application.update(window, |application, _| {
        application.popup = Some(QuickLaunchWindow {
            window: popup_window,
            panel: panel.downgrade(),
            native_window_handle: 1,
        });
    });
    for action in [QuickLaunchAction::CopyPath, QuickLaunchAction::CopyCommand] {
        let source = if action == QuickLaunchAction::CopyPath {
            ApplicationSource::IndexedFile
        } else {
            ApplicationSource::CustomCommand
        };
        let mut result = search_result("fixture", source);
        result.subtitle = Some("synthetic command".into());
        application.update(window, |application, cx| {
            application.execute(
                result,
                action.clone(),
                "fixture".into(),
                QuickLaunchActionTrigger::Enter,
                cx,
            )
        });
        let accepted = endpoint
            .try_receive_command_for_contract_test()
            .expect("copy action dispatched");
        assert!(matches!(
            accepted.message,
            nxtools_protocol::Message::PlatformCommand(_)
        ));
        panel.read_with(window, |panel, _| assert!(panel.is_open));
        application.read_with(window, |application, _| {
            assert!(application.pending_actions.contains_key(&accepted.id));
        });
    }
}

#[test]
fn copy_path_success_keeps_the_launcher_open_without_recording_a_launch() {
    assert_eq!(
        action_success_policy(&QuickLaunchAction::CopyPath),
        ActionSuccessPolicy::KeepOpen
    );
    for action in [
        QuickLaunchAction::Open,
        QuickLaunchAction::Reveal,
        QuickLaunchAction::RunElevated,
    ] {
        assert_eq!(
            action_success_policy(&action),
            ActionSuccessPolicy::RecordAndHide
        );
    }
}

#[test]
fn a_late_action_result_cannot_target_a_reopened_popup_session() {
    let original_session = uuid::Uuid::new_v4();
    let reopened_session = uuid::Uuid::new_v4();
    let pending = PendingQuickLaunchAction {
        correlation_id: uuid::Uuid::new_v4(),
        popup_session_id: original_session,
        entry_id: "reports".into(),
        display_name: "Reports".into(),
        target: r"C:\Reports".into(),
        query: "reports".into(),
        action: QuickLaunchAction::SwitchDirectory {
            host: DirectoryNavigationHost::Explorer,
            window_handle: 88,
            target_directory: r"C:\Reports".into(),
            host_context_id: None,
        },
        directory_visit: Some(r"C:\Reports".into()),
        trigger: QuickLaunchActionTrigger::RecentDirectory,
        source: ApplicationSource::IndexedDirectory,
        started_at: Instant::now(),
    };

    assert!(pending_action_targets_session(&pending, original_session));
    assert!(!pending_action_targets_session(&pending, reopened_session));
}

#[test]
fn disconnected_actions_only_report_the_current_popup_session() {
    let current_session = uuid::Uuid::new_v4();
    let stale_session = uuid::Uuid::new_v4();
    let make_pending = |popup_session_id| PendingQuickLaunchAction {
        correlation_id: uuid::Uuid::new_v4(),
        popup_session_id,
        entry_id: "reports".into(),
        display_name: "Reports".into(),
        target: r"C:\Reports".into(),
        query: "reports".into(),
        action: QuickLaunchAction::Open,
        directory_visit: None,
        trigger: QuickLaunchActionTrigger::Enter,
        source: ApplicationSource::IndexedFile,
        started_at: Instant::now(),
    };
    let current_id = uuid::Uuid::new_v4();
    let stale_id = uuid::Uuid::new_v4();
    let mut actions = std::collections::HashMap::new();
    actions.insert(current_id, make_pending(current_session));
    actions.insert(stale_id, make_pending(stale_session));

    let (current, stale_count) = partition_actions_for_disconnect(actions, current_session);
    assert_eq!(current.len(), 1);
    assert_eq!(current[0].popup_session_id, current_session);
    assert_eq!(stale_count, 1);
}

#[test]
fn pending_directory_navigation_owns_its_foreground_transition() {
    let session = uuid::Uuid::new_v4();
    let pending = PendingQuickLaunchAction {
        correlation_id: uuid::Uuid::new_v4(),
        popup_session_id: session,
        entry_id: "reports".into(),
        display_name: "Reports".into(),
        target: r"C:\Reports".into(),
        query: "reports".into(),
        action: QuickLaunchAction::SwitchDirectory {
            host: DirectoryNavigationHost::FileDialog,
            window_handle: 77,
            target_directory: r"C:\Reports".into(),
            host_context_id: None,
        },
        directory_visit: Some(r"C:\Reports".into()),
        trigger: QuickLaunchActionTrigger::RecentDirectory,
        source: ApplicationSource::IndexedDirectory,
        started_at: Instant::now(),
    };

    assert!(pending_action_owns_foreground(&pending, session, 77));
    assert!(!pending_action_owns_foreground(&pending, session, 78));
    assert!(!pending_action_owns_foreground(
        &pending,
        uuid::Uuid::new_v4(),
        77
    ));
}

#[test]
fn failed_directory_navigation_guards_late_foreground_events_until_expiry() {
    let now = Instant::now();
    let session = uuid::Uuid::new_v4();
    let mut guard = Some(NavigationForegroundGuard {
        popup_session_id: session,
        window_handle: 77,
        expires_at: now + FAILED_NAVIGATION_FOREGROUND_GUARD,
    });

    assert!(navigation_guard_owns_foreground(
        &mut guard, session, 77, now
    ));
    assert!(navigation_guard_owns_foreground(
        &mut guard,
        session,
        77,
        now + FAILED_NAVIGATION_FOREGROUND_GUARD / 2
    ));
    assert!(!navigation_guard_owns_foreground(
        &mut guard,
        session,
        77,
        now + FAILED_NAVIGATION_FOREGROUND_GUARD
    ));
    assert!(guard.is_none());
}

#[test]
fn action_mode_navigation_wraps_and_filtering_resets_selection() {
    let mut mode = QuickLaunchActionMode::enter(
        search_result("alpha", ApplicationSource::StartMenu),
        String::new(),
        0,
    );
    finish_shell_loading(&mut mode);

    assert_eq!(mode.selected_action(), Some(QuickLaunchAction::Open));
    mode.move_selection(-1);
    assert_eq!(mode.selected_action(), Some(QuickLaunchAction::RunElevated));
    mode.move_selection(1);
    assert_eq!(mode.selected_action(), Some(QuickLaunchAction::Open));

    mode.move_selection(1);
    mode.set_filter("管理");
    assert_eq!(mode.selected_action(), Some(QuickLaunchAction::RunElevated));
}

#[test]
fn pending_root_action_mode_does_not_publish_a_partial_builtin_list() {
    let mode = QuickLaunchActionMode::enter(
        search_result("alpha", ApplicationSource::IndexedFile),
        "alp".into(),
        0,
    );

    assert!(mode.shell_pending());
    assert!(mode.visible_actions().is_empty());
}

#[test]
fn action_mode_appends_shell_groups_and_enters_nested_commands() {
    let mut mode = QuickLaunchActionMode::enter(
        search_result("alpha", ApplicationSource::IndexedFile),
        "alp".into(),
        0,
    );
    let icon = tempfile::NamedTempFile::new().expect("staged Shell icon");
    let root_level_id = mode.current_level_id();
    mode.apply_shell_menu(
        root_level_id,
        Some(ShellActionMenu {
            session_id: "session-1".into(),
            navigation_directory: None,
            items: vec![
                ShellActionItem {
                    command_id: None,
                    title: String::new(),
                    enabled: false,
                    separator: true,
                    submenu_id: None,
                    icon_path: None,
                },
                ShellActionItem {
                    command_id: None,
                    title: "打开方式".into(),
                    enabled: true,
                    separator: false,
                    submenu_id: Some(7),
                    icon_path: Some(icon.path().to_string_lossy().into_owned()),
                },
                ShellActionItem {
                    command_id: Some(43),
                    title: "不可用操作".into(),
                    enabled: false,
                    separator: false,
                    submenu_id: None,
                    icon_path: None,
                },
            ],
        }),
    );

    let root = mode.visible_actions();
    assert!(root.len() > QUICK_LAUNCH_ACTIONS.len());
    let submenu_index = root
        .iter()
        .position(|item| item.label == "打开方式")
        .expect("Shell submenu");
    assert_eq!(
        root[submenu_index].section_label.as_deref(),
        Some("Windows 操作")
    );
    assert!(root[submenu_index].divider_before);
    assert_eq!(root[submenu_index].icon_path.as_deref(), Some(icon.path()));
    assert!(
        !root
            .iter()
            .find(|item| item.label == "不可用操作")
            .expect("disabled Shell item")
            .enabled
    );

    mode.set_selected(submenu_index);
    let submenu = mode.push_selected_submenu().expect("lazy Shell submenu");
    assert_eq!(submenu.session_id, "session-1");
    assert_eq!(submenu.submenu_id, 7);
    assert_eq!(mode.depth(), 2);
    assert!(mode.shell_pending());
    assert_eq!(mode.selected_action(), None);
    mode.apply_shell_menu(
        submenu.level_id,
        Some(ShellActionMenu {
            session_id: "session-1".into(),
            navigation_directory: None,
            items: vec![ShellActionItem {
                command_id: Some(42),
                title: "测试程序".into(),
                enabled: true,
                separator: false,
                submenu_id: None,
                icon_path: None,
            }],
        }),
    );
    assert_eq!(
        mode.selected_action(),
        Some(QuickLaunchAction::Shell {
            session_id: "session-1".into(),
            command_id: 42,
        })
    );
    assert_eq!(mode.pop_level(), Some(String::new()));
    assert_eq!(mode.depth(), 1);
}

#[test]
fn shell_action_invocation_preserves_session_command_and_owner_window() {
    let result = search_result("alpha", ApplicationSource::IndexedFile);
    let action = QuickLaunchAction::Shell {
        session_id: "session-2".into(),
        command_id: 17,
    };
    let command =
        platform_command_for_action(&result, &action, 1234).expect("Shell action command");
    assert_eq!(
        command,
        PlatformCommand::InvokeShellAction(InvokeShellActionCommand {
            session_id: "session-2".into(),
            command_id: 17,
            owner_window_handle: 1234,
        })
    );
}

#[test]
fn a_late_shell_menu_cannot_replace_a_reentered_action_mode() {
    let result = search_result("alpha", ApplicationSource::IndexedFile);
    let entry_id = result.entry.id.clone();
    let previous = QuickLaunchActionMode::enter(result.clone(), String::new(), 0);
    let current = QuickLaunchActionMode::enter(result, String::new(), 0);
    let current_level_id = current.current_level_id();

    assert_ne!(previous.id(), current.id());
    assert!(!current.matches_shell_query(previous.id(), current_level_id, &entry_id));
    assert!(current.matches_shell_query(current.id(), current_level_id, &entry_id));
}

#[test]
fn clearing_history_rejects_a_late_index_batch_from_the_old_generation() {
    let mut session = IndexSearchSession::default();
    let request = session.begin("old query", INDEX_CANDIDATE_LIMIT);
    let old_generation = request.generation;
    assert!(session.apply(nxtools_protocol::IndexSearchBatch {
        generation: old_generation,
        items: vec![nxtools_protocol::IndexSearchItem {
            name: "old-result".into(),
            path: r"C:\old-result.txt".into(),
            is_directory: false,
            size_bytes: None,
            modified_at_unix_ms: None,
            score: 0,
        }],
        complete: true,
        elapsed_us: 10,
    }));
    session.invalidate();
    assert!(!session.apply(nxtools_protocol::IndexSearchBatch {
        generation: old_generation,
        items: vec![nxtools_protocol::IndexSearchItem {
            name: "late-old-result".into(),
            path: r"C:\late-old-result.txt".into(),
            is_directory: false,
            size_bytes: None,
            modified_at_unix_ms: None,
            score: 0,
        }],
        complete: true,
        elapsed_us: 11,
    }));
    assert!(session.items.is_empty());
    assert!(!session.complete);
}

#[test]
fn escape_always_returns_from_action_mode_before_clearing_the_parent_query() {
    assert_eq!(
        quick_launch_escape_action(true, false),
        QuickLaunchEscapeAction::ExitActions
    );
    assert_eq!(
        quick_launch_escape_action(false, false),
        QuickLaunchEscapeAction::ClearQuery
    );
    assert_eq!(
        quick_launch_escape_action(false, true),
        QuickLaunchEscapeAction::Close
    );
}

#[test]
fn escape_hides_the_popup_then_restores_the_captured_host_for_the_next_summon() {
    assert_eq!(
        quick_launch_dismiss_commands(900, Some(88)),
        vec![
            PlatformCommand::SetWindowVisibility(SetWindowVisibilityCommand {
                window_handle: 900,
                visible: false,
            }),
            PlatformCommand::SetWindowTopmost(SetWindowTopmostCommand {
                window_handle: 900,
                enabled: false,
            }),
            PlatformCommand::ActivateWindow(ActivateWindowCommand { window_handle: 88 }),
        ]
    );
    assert_eq!(
        quick_launch_dismiss_commands(900, None),
        vec![
            PlatformCommand::SetWindowVisibility(SetWindowVisibilityCommand {
                window_handle: 900,
                visible: false,
            }),
            PlatformCommand::SetWindowTopmost(SetWindowTopmostCommand {
                window_handle: 900,
                enabled: false,
            }),
        ]
    );
}

#[test]
fn a_late_file_batch_cannot_replace_the_newer_query_generation() {
    let mut session = IndexSearchSession::default();
    let first = session.begin("report", 20);
    let second = session.begin("report final", 20);
    let batch = |generation, elapsed_us| nxtools_protocol::IndexSearchBatch {
        generation,
        items: Vec::new(),
        complete: true,
        elapsed_us,
    };

    assert!(!session.apply(batch(first.generation, 999)));
    assert_eq!(session.last_completed_elapsed_us, None);
    assert!(session.apply(batch(second.generation, 17)));
    assert!(session.complete);
    assert_eq!(session.last_completed_elapsed_us, Some(17));
}

#[test]
fn an_indexer_disconnect_clears_visible_status_and_invalidates_search() {
    let mut status = Some(IndexStatus {
        phase: nxtools_protocol::IndexPhase::VolumesReady,
        indexed_items: 0,
        drives: Vec::new(),
        last_error: None,
    });
    let mut session = IndexSearchSession::default();
    let request = session.begin("report", 20);

    assert!(apply_indexer_notification(
        &mut status,
        &mut session,
        IndexerNotification::Disconnected,
    ));
    assert!(status.is_none());
    assert_ne!(session.generation, request.generation);
}

#[gpui_kit::test]
fn gpui_action_mode_handles_keyboard_mouse_and_parent_restoration(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let temp = tempfile::tempdir().expect("temporary quick-launch test directory");
    let root = temp.path().join("applications");
    std::fs::create_dir_all(&root).expect("application test root");
    std::fs::write(root.join("alpha.lnk"), b"").expect("test shortcut");
    let catalog = FinderCatalog::open(
        temp.path().join("history.db"),
        &[nxtools_finder::ApplicationRoot {
            path: root,
            source: ApplicationSource::StartMenu,
        }],
    )
    .expect("test quick-launch catalog");
    let results = catalog.search("", RESULT_LIMIT).expect("initial results");
    let application = cx.update(|cx| {
        let notifications = cx.new(|_| NotificationCenter::default());
        let (platform, _endpoint) = nxtools_ipc::platform_bridge(8);
        let (indexer, _indexer_endpoint) = crate::runtime::indexer_bridge(8);
        cx.new(|_| {
            QuickLaunchApplication::new(
                catalog,
                Rc::new(platform),
                Rc::new(indexer),
                AppConfig::default(),
                temp.path().join("config.json"),
                temp.path().join("logs"),
                notifications,
            )
        })
    });
    let target = nxtools_protocol::ForegroundContext {
        window_handle: 1,
        focus_window_handle: 1,
        process_id: 1,
        process_name: "editor".into(),
        window_title: "document".into(),
    };
    let panel_slot = Rc::new(RefCell::new(None));
    let panel_slot_for_window = panel_slot.clone();
    let application_for_window = application.clone();
    let (_root, cx) = cx.add_window_view(move |window, cx| {
        let panel = cx.new(|cx| {
            QuickLaunchPanel::new(
                results,
                target,
                None,
                Vec::new(),
                application_for_window.downgrade(),
                window,
                cx,
            )
        });
        *panel_slot_for_window.borrow_mut() = Some(panel.clone());
        Root::new(panel, window, cx)
    });
    let panel = panel_slot
        .borrow_mut()
        .take()
        .expect("quick-launch panel entity");
    panel.update_in(cx, |panel, window, cx| panel.focus_search(window, cx));

    cx.simulate_input("alp");
    panel.update_in(cx, |panel, _window, cx| {
        panel.run_search("alp".into(), cx);
    });
    panel.update_in(cx, |panel, window, _cx| {
        assert_eq!(window.bounds().size.height, px(panel.content_height()));
    });
    panel.read_with(cx, |panel, cx| {
        assert_eq!(panel.search_input.read(cx).value().as_ref(), "alp");
        assert_eq!(panel.results.len(), 1);
    });
    cx.simulate_keystrokes("right");
    panel.read_with(cx, |panel, cx| {
        assert!(panel.action_mode.is_some());
        assert!(panel.search_input.read(cx).value().is_empty());
    });

    cx.simulate_input("管理");
    cx.simulate_keystrokes("escape");
    panel.read_with(cx, |panel, cx| {
        assert!(panel.action_mode.is_none());
        assert_eq!(panel.search_input.read(cx).value().as_ref(), "alp");
        assert_eq!(panel.results[panel.selected].entry.name, "alpha");
    });

    cx.refresh().expect("render restored results");
    let result_bounds = cx
        .debug_bounds("quick-launch-result-row-0")
        .expect("restored result row");
    cx.simulate_mouse_down(
        result_bounds.center(),
        gpui_kit::MouseButton::Right,
        gpui_kit::Modifiers::none(),
    );
    panel.read_with(cx, |panel, _| assert!(panel.action_mode.is_some()));
    cx.simulate_keystrokes("backspace");
    panel.read_with(cx, |panel, _| assert!(panel.action_mode.is_none()));

    let keyboard_selected_id = panel.read_with(cx, |panel, _| {
        panel.results[panel.selected].entry.id.clone()
    });
    cx.simulate_keystrokes("down");
    cx.executor()
        .advance_clock(QUICK_LAUNCH_PREVIEW_HOVER_DELAY);
    cx.run_until_parked();
    application.read_with(cx, |application, _| {
        assert!(application.preview.visible);
        assert_eq!(
            application.preview.displayed_entry_id.as_deref(),
            Some(keyboard_selected_id.as_str())
        );
    });
    application.update(cx, |application, cx| {
        application.dismiss_quick_launch_preview(cx);
    });

    application.update(cx, |application, _| {
        application.preview.visible = true;
    });
    cx.simulate_event(gpui_kit::ScrollEvent {
        position: point(px(20.0), px(80.0)),
        delta: gpui_kit::ScrollDelta::Lines(point(0.0, -1.0)),
        ..Default::default()
    });
    application.read_with(cx, |application, _| {
        assert!(!application.preview.visible);
    });

    cx.simulate_keystrokes("right");
    panel.update_in(cx, |panel, _window, cx| {
        let mode = panel.action_mode.as_ref().expect("pending action mode");
        let mode_id = mode.id();
        let level_id = mode.current_level_id();
        let entry_id = mode.target().entry.id.clone();
        panel.apply_shell_actions(mode_id, level_id, &entry_id, None, cx);
    });
    panel.read_with(cx, |panel, _| {
        assert_eq!(
            panel
                .action_mode
                .as_ref()
                .and_then(QuickLaunchActionMode::selected_action),
            Some(QuickLaunchAction::Open)
        );
    });
    cx.simulate_keystrokes("escape escape");
    panel.update_in(cx, |panel, window, cx| {
        assert!(panel.action_mode.is_none());
        assert!(panel.search_input.read(cx).value().is_empty());
        assert_eq!(window.bounds().size.height, px(COLLAPSED_PANEL_HEIGHT));
    });
    panel.update_in(cx, |panel, _window, cx| {
        panel.search_pending = true;
        panel.replace_results(Vec::new(), cx);
        assert!(!panel.search_pending);
    });
}

#[gpui_kit::test]
fn action_mode_returns_to_the_parent_viewport_on_its_first_frame(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let results = (0..40)
        .map(|index| {
            search_result(
                format!("item-{index:02}").as_str(),
                ApplicationSource::Calculator,
            )
        })
        .collect::<Vec<_>>();
    let (panel, window) = quick_launch_panel_fixture(cx, results);

    panel.update_in(window, |panel, window, cx| {
        panel.focus_search(window, cx);
        panel
            .search_input
            .update(cx, |input, cx| input.set_value("item", window, cx));
        panel.replace_results(panel.results.to_vec(), cx);
        panel.selected = 24;
        panel
            .result_scroll
            .scroll_to_item_strict(24, ScrollStrategy::Center);
        cx.notify();
    });
    window
        .refresh()
        .expect("render the scrolled parent results");
    window
        .debug_bounds("quick-launch-result-title-24")
        .expect("the selected lower result is rendered before entering actions");
    let parent_scroll_offset = panel.update_in(window, |panel, _window, _cx| {
        panel.result_scroll.0.borrow().base_handle.offset()
    });
    assert!(
        parent_scroll_offset.y < px(0.0),
        "the parent viewport must start below the first page"
    );

    for exit_key in ["escape", "left", "backspace"] {
        window.simulate_keystrokes("right");
        panel.update_in(window, |panel, _window, _cx| {
            assert!(panel.action_mode.is_some())
        });
        window
            .refresh()
            .expect("render action mode before returning");
        window
            .debug_bounds("quick-launch-action-context")
            .expect("action mode is mounted before returning");

        window.simulate_keystrokes(exit_key);
        window
            .refresh()
            .expect("render the first returned parent frame");
        window
            .debug_bounds("quick-launch-result-title-24")
            .expect("the selected lower result is rendered on the first returned frame");
        panel.update_in(window, |panel, _window, cx| {
            assert!(panel.action_mode.is_none());
            assert_eq!(panel.selected, 24);
            assert_eq!(panel.search_input.read(cx).value().as_ref(), "item");
            assert_eq!(
                panel.result_scroll.0.borrow().base_handle.offset(),
                parent_scroll_offset,
                "{exit_key} must return on the preserved parent viewport without a top flash"
            );
        });
    }
}

#[gpui_kit::test]
fn action_mode_rails_stay_right_aligned_for_one_and_eight_actions(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (panel, window) = quick_launch_panel_fixture(
        cx,
        vec![search_result("target", ApplicationSource::Calculator)],
    );
    for (source, expected_count) in [
        (ApplicationSource::Calculator, 1_usize),
        (ApplicationSource::IndexedFile, 8_usize),
    ] {
        panel.update_in(window, |panel, _window, cx| {
            let mut mode =
                QuickLaunchActionMode::enter(search_result("target", source), "target".into(), 0);
            if expected_count == 1 {
                assert_eq!(mode.visible_actions().len(), expected_count);
            } else {
                let level_id = mode.current_level_id();
                mode.apply_shell_menu(
                    level_id,
                    Some(ShellActionMenu {
                        session_id: "rail-session".into(),
                        navigation_directory: None,
                        items: (0..4)
                            .map(|index| ShellActionItem {
                                command_id: Some(100 + index),
                                title: format!("Windows 操作 {index}"),
                                enabled: true,
                                separator: false,
                                submenu_id: None,
                                icon_path: None,
                            })
                            .collect(),
                    }),
                );
                assert_eq!(mode.visible_actions().len(), expected_count);
            }
            panel.action_mode = Some(mode);
            panel.results_visible = true;
            cx.notify();
        });
        window.run_until_parked();
        window.refresh().expect("render action-mode rail fixture");
        let first_row = window
            .debug_bounds("quick-launch-action-row-0")
            .expect("first action row");
        let last_index = expected_count - 1;
        let last_row_selector: &'static str =
            Box::leak(format!("quick-launch-action-row-{last_index}").into_boxed_str());
        let last_row = window
            .debug_bounds(last_row_selector)
            .expect("last action row");
        let first_rail = window
            .debug_bounds("quick-launch-action-trailing-0")
            .expect("first action trailing rail");
        let last_rail_selector: &'static str =
            Box::leak(format!("quick-launch-action-trailing-{last_index}").into_boxed_str());
        let last_rail = window
            .debug_bounds(last_rail_selector)
            .expect("last action trailing rail");
        assert_eq!(first_row.size.height, px(COMPACT_RESULT_ROW_HEIGHT));
        assert_eq!(last_row.size.height, px(COMPACT_RESULT_ROW_HEIGHT));
        assert_eq!(
            first_rail.size.width,
            px(COMPACT_RESULT_TRAILING_RAIL_WIDTH)
        );
        assert_eq!(last_rail.size.width, first_rail.size.width);
        assert_eq!(last_rail.right(), first_rail.right());
        assert!(last_rail.right() <= last_row.right());
    }
}

#[gpui_kit::test]
fn action_mode_reuses_the_result_row_rails_and_only_adds_a_lightweight_context(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_kit::init);
    let (panel, window) =
        quick_launch_panel_fixture(cx, vec![search_result("42", ApplicationSource::Calculator)]);

    panel.update_in(window, |panel, window, cx| panel.focus_search(window, cx));
    window.refresh().expect("render the ordinary result row");
    let result_row = window
        .debug_bounds("quick-launch-result-row-0")
        .expect("ordinary result row");
    let result_icon = window
        .debug_bounds("quick-launch-result-icon-0")
        .expect("ordinary result icon rail");
    let result_copy = window
        .debug_bounds("quick-launch-result-copy-0")
        .expect("ordinary result copy rail");
    let result_title = window
        .debug_bounds("quick-launch-result-title-0")
        .expect("ordinary result title");
    let result_trailing = window
        .debug_bounds("quick-launch-result-shortcut-0")
        .expect("ordinary result trailing rail");

    window.simulate_keystrokes("right");
    panel.update_in(window, |panel, _window, _cx| {
        assert!(panel.action_mode.is_some())
    });
    window.refresh().expect("render the action-mode row");
    let context = window
        .debug_bounds("quick-launch-action-context")
        .expect("lightweight action-mode context");
    let action_row = window
        .debug_bounds("quick-launch-action-row-0")
        .expect("action row reusing the result shell");
    let action_icon = window
        .debug_bounds("quick-launch-action-icon-0")
        .expect("action icon rail");
    let action_copy = window
        .debug_bounds("quick-launch-action-copy-0")
        .expect("action copy rail");
    let action_title = window
        .debug_bounds("quick-launch-action-title-0")
        .expect("action title");
    let action_trailing = window
        .debug_bounds("quick-launch-action-trailing-0")
        .expect("action trailing rail");

    assert_eq!(context.size.height, px(QUICK_LAUNCH_ACTION_CONTEXT_HEIGHT));
    assert_eq!(result_row.size.height, px(LAUNCHER_RESULT_ROW_HEIGHT));
    assert_eq!(action_row.size.height, px(COMPACT_RESULT_ROW_HEIGHT));
    assert_eq!(action_icon.origin.x, result_icon.origin.x);
    assert_eq!(action_icon.size.width, px(COMPACT_RESULT_ICON_RAIL_WIDTH));
    assert_eq!(result_icon.size.width, px(LAUNCHER_RESULT_ICON_SIZE));
    assert_eq!(
        result_copy.origin.x - action_copy.origin.x,
        px(LAUNCHER_RESULT_ICON_SIZE - COMPACT_RESULT_ICON_RAIL_WIDTH)
    );
    assert_eq!(action_title.size.height, px(LINE_HEIGHT_BODY));
    assert_eq!(result_title.size.height, px(LAUNCHER_TITLE_LINE_HEIGHT));
    assert_eq!(action_trailing.origin.x, result_trailing.origin.x);
    assert_eq!(action_trailing.size.width, result_trailing.size.width);
}

#[gpui_kit::test]
fn production_quick_launch_preview_uses_shared_fixed_shell_and_media_surface(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_kit::init);
    let payload = QuickLaunchPreviewPayload {
        entry_id: "preview-image".into(),
        name: "NxTools.png".into(),
        detail: "PNG 图片 · 48 × 48".into(),
        target: r"C:\fixtures\NxTools.png".into(),
        created_at: Some("2026-08-24 12:00:00".into()),
        modified_at: Some("2026-08-24 12:01:00".into()),
        body: QuickLaunchPreviewBody::Image(Arc::new(Image::from_bytes(
            ImageFormat::Png,
            include_bytes!(
                "../../../../../browser_extension/nxtools-interceptor/icons/icon-48.png"
            )
            .to_vec(),
        ))),
    };
    let (_root, window) = cx.add_window_view(move |window, cx| {
        window.resize(size(
            px(QUICK_LAUNCH_PREVIEW_WIDTH),
            px(QUICK_LAUNCH_PREVIEW_HEIGHT),
        ));
        let preview = cx.new(|_| {
            QuickLaunchPreview::new(payload, WeakEntity::<QuickLaunchApplication>::new_invalid())
        });
        Root::new(preview, window, cx)
    });

    window
        .refresh()
        .expect("render production Quick Launch preview");
    let shell = window
        .debug_bounds("preview-shell")
        .expect("shared preview shell");
    let header = window
        .debug_bounds("quick-launch-preview-header")
        .expect("stable preview identity header");
    let media = window
        .debug_bounds("preview-media-surface")
        .expect("shared preview media surface");
    let footer = window
        .debug_bounds("quick-launch-preview-footer")
        .expect("stable preview path footer");

    assert_eq!(shell.size, size(px(400.0), px(410.0)));
    assert_eq!(header.size.height, px(72.0));
    assert_eq!(footer.size.height, px(30.0));
    assert_eq!(media.size, size(px(382.0), px(290.0)));
}

#[gpui_kit::test]
fn production_quick_launch_text_preview_starts_at_the_media_inset(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let payload = QuickLaunchPreviewPayload {
        entry_id: "preview-text".into(),
        name: "notes.txt".into(),
        detail: "TXT 文件 · 24 B".into(),
        target: r"C:\fixtures\notes.txt".into(),
        created_at: None,
        modified_at: None,
        body: QuickLaunchPreviewBody::Text {
            text: "first line\nsecond line".into(),
            truncated: true,
        },
    };
    let (_root, window) = cx.add_window_view(move |window, cx| {
        window.resize(size(
            px(QUICK_LAUNCH_PREVIEW_WIDTH),
            px(QUICK_LAUNCH_PREVIEW_HEIGHT),
        ));
        let preview = cx.new(|_| {
            QuickLaunchPreview::new(payload, WeakEntity::<QuickLaunchApplication>::new_invalid())
        });
        Root::new(preview, window, cx)
    });

    window.refresh().expect("render text preview");
    let media = window
        .debug_bounds("preview-media-surface")
        .expect("text preview media surface");
    let copy = window
        .debug_bounds("quick-launch-preview-text-copy")
        .expect("top-aligned text preview copy");

    // The 12px inset is asserted with a 0.5px tolerance: subpixel centering
    // can jitter by half a pixel depending on the window origin the test
    // harness assigns.
    let expected_left = media.left() + px(SPACING_12);
    assert!(
        (copy.left() - expected_left).abs() <= px(0.5),
        "copy.left={} expected {}",
        copy.left(),
        expected_left
    );
    let expected_top = media.top() + px(SPACING_12);
    assert!(
        (copy.top() - expected_top).abs() <= px(0.5),
        "copy.top={} expected {}",
        copy.top(),
        expected_top
    );
}

#[gpui_kit::test]
fn keyboard_navigation_retains_focus_after_first_preview_frame(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let temp = tempfile::tempdir().expect("temporary keyboard preview test directory");
    let root = temp.path().join("applications");
    std::fs::create_dir_all(&root).expect("application test root");
    std::fs::write(root.join("alpha.lnk"), b"").expect("first test shortcut");
    std::fs::write(root.join("alpine.lnk"), b"").expect("second test shortcut");
    let catalog = FinderCatalog::open(
        temp.path().join("history.db"),
        &[nxtools_finder::ApplicationRoot {
            path: root,
            source: ApplicationSource::StartMenu,
        }],
    )
    .expect("keyboard preview catalog");
    let results = catalog
        .search("alp", RESULT_LIMIT)
        .expect("preview results");
    assert_eq!(results.len(), 2);
    let application = cx.update(|cx| {
        let notifications = cx.new(|_| NotificationCenter::default());
        let (platform, _endpoint) = nxtools_ipc::platform_bridge(8);
        let (indexer, _indexer_endpoint) = crate::runtime::indexer_bridge(8);
        cx.new(|_| {
            QuickLaunchApplication::new(
                catalog,
                Rc::new(platform),
                Rc::new(indexer),
                AppConfig::default(),
                temp.path().join("config.json"),
                temp.path().join("logs"),
                notifications,
            )
        })
    });
    let panel_slot = Rc::new(RefCell::new(None));
    let panel_slot_for_window = panel_slot.clone();
    let popup_window_slot = Rc::new(RefCell::new(None));
    let popup_window_slot_for_window = popup_window_slot.clone();
    let application_for_window = application.clone();
    let (_root, cx) = cx.add_window_view(move |window, cx| {
        *popup_window_slot_for_window.borrow_mut() = Some(window.window_handle());
        let panel = cx.new(|cx| {
            QuickLaunchPanel::new(
                results,
                nxtools_protocol::ForegroundContext {
                    window_handle: 1,
                    focus_window_handle: 1,
                    process_id: 1,
                    process_name: "editor".into(),
                    window_title: "document".into(),
                },
                None,
                Vec::new(),
                application_for_window.downgrade(),
                window,
                cx,
            )
        });
        *panel_slot_for_window.borrow_mut() = Some(panel.clone());
        Root::new(panel, window, cx)
    });
    let panel = panel_slot
        .borrow_mut()
        .take()
        .expect("keyboard preview panel");
    let popup_window = popup_window_slot
        .borrow_mut()
        .take()
        .expect("keyboard preview popup window");
    application.update(cx, |application, _| {
        application.popup = Some(QuickLaunchWindow {
            window: popup_window,
            panel: panel.downgrade(),
            native_window_handle: 1,
        });
    });
    panel.update_in(cx, |panel, window, cx| panel.focus_search(window, cx));
    cx.simulate_input("alp");
    panel.update_in(cx, |panel, _window, cx| panel.run_search("alp".into(), cx));

    let initial_selection = panel.read_with(cx, |panel, _| panel.selected);
    cx.simulate_keystrokes("down");
    let first_selection = panel.read_with(cx, |panel, _| panel.selected);
    assert_ne!(first_selection, initial_selection);
    cx.executor()
        .advance_clock(QUICK_LAUNCH_PREVIEW_HOVER_DELAY);
    cx.run_until_parked();

    let preview_window = application.read_with(cx, |application, _| {
        assert!(application.preview.visible);
        application
            .preview
            .window
            .as_ref()
            .expect("first keyboard preview window")
            .identity
            .gpui
    });
    let preview_bounds = preview_window
        .update(cx, |_, window, _| window.bounds())
        .expect("first preview window bounds");
    assert_eq!(preview_bounds.size.width, px(QUICK_LAUNCH_PREVIEW_WIDTH));
    assert_eq!(preview_bounds.size.height, px(QUICK_LAUNCH_PREVIEW_HEIGHT));
    panel.update_in(cx, |panel, window, cx| {
        assert!(panel.search_input.focus_handle(cx).is_focused(window));
    });

    let selection_before_second_key = panel.read_with(cx, |panel, _| panel.selected);
    cx.simulate_keystrokes("down");
    panel.read_with(cx, |panel, _| {
        assert_ne!(panel.selected, selection_before_second_key);
    });
}

#[gpui_kit::test]
fn quick_launch_preview_lifecycle_discards_stale_hover_and_honors_pointer_grace(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_kit::init);
    let temp = tempfile::tempdir().expect("temporary quick-launch preview test directory");
    let root = temp.path().join("applications");
    std::fs::create_dir_all(&root).expect("application root");
    let first_path = temp.path().join("first.txt");
    let second_path = temp.path().join("second.txt");
    std::fs::write(&first_path, "first preview").expect("first preview fixture");
    std::fs::write(&second_path, "second preview").expect("second preview fixture");
    let catalog = FinderCatalog::open(
        temp.path().join("history.db"),
        &[nxtools_finder::ApplicationRoot {
            path: root,
            source: ApplicationSource::StartMenu,
        }],
    )
    .expect("test quick-launch catalog");
    let application = cx.update(|cx| {
        let notifications = cx.new(|_| NotificationCenter::default());
        let (platform, _endpoint) = nxtools_ipc::platform_bridge(8);
        let (indexer, _indexer_endpoint) = crate::runtime::indexer_bridge(8);
        cx.new(|_| {
            QuickLaunchApplication::new(
                catalog,
                Rc::new(platform),
                Rc::new(indexer),
                AppConfig::default(),
                temp.path().join("config.json"),
                temp.path().join("logs"),
                notifications,
            )
        })
    });
    let make_result = |id: &str, path: &Path| {
        let mut result = search_result(id, ApplicationSource::IndexedFile);
        result.entry.target = path.to_string_lossy().into_owned();
        result
    };
    let first = make_result("first", &first_path);
    let second = make_result("second", &second_path);
    let replacement = first.clone();
    let panel_bounds = Bounds::new(
        point(px(200.0), px(120.0)),
        size(px(PANEL_WIDTH), px(PANEL_HEIGHT)),
    );

    application.update(cx, |application, cx| {
        application.schedule_quick_launch_preview(true, first, panel_bounds, 1.0, cx);
        application.schedule_quick_launch_preview(true, second.clone(), panel_bounds, 1.0, cx);
    });
    cx.executor()
        .advance_clock(QUICK_LAUNCH_PREVIEW_HOVER_DELAY);
    cx.run_until_parked();
    application.read_with(cx, |application, _| {
        assert!(application.preview.visible);
        assert_eq!(
            application.preview.displayed_entry_id.as_deref(),
            Some("second")
        );
        assert!(application.preview.window.is_some());
    });

    let visible_preview = application.read_with(cx, |application, _| {
        application
            .preview
            .window
            .as_ref()
            .expect("visible reused preview window")
            .view
            .clone()
    });
    let interrupted_exit_generation = visible_preview
        .update(cx, |preview, cx| {
            preview.begin_exit(cx);
            preview.appearance_generation
        })
        .expect("begin preview exit before target switch");
    application.update(cx, |application, cx| {
        application.schedule_quick_launch_preview(true, replacement, panel_bounds, 1.0, cx);
    });
    visible_preview
        .read_with(cx, |preview, _| {
            assert!(
                !preview.is_exiting,
                "a visible reused shell must not play an exit before replacement"
            );
            assert_eq!(
                preview.appearance_generation, interrupted_exit_generation,
                "cancelling an old exit must not animate the old payload back in"
            );
            assert!(
                preview.motion_suppressed,
                "the interrupted old payload must render without an animation wrapper"
            );
        })
        .expect("visible reused preview view");
    application.update(cx, |application, cx| {
        application.schedule_quick_launch_preview(true, second.clone(), panel_bounds, 1.0, cx);
    });

    application.update(cx, |application, cx| {
        application.schedule_quick_launch_preview(false, second, panel_bounds, 1.0, cx);
        application.set_quick_launch_preview_pointer_inside(true, cx);
    });
    cx.executor()
        .advance_clock(QUICK_LAUNCH_PREVIEW_LEAVE_DELAY);
    cx.run_until_parked();
    application.read_with(cx, |application, _| {
        assert!(application.preview.visible);
    });

    application.update(cx, |application, cx| {
        application.set_quick_launch_preview_pointer_inside(false, cx);
    });
    cx.executor()
        .advance_clock(QUICK_LAUNCH_PREVIEW_LEAVE_DELAY);
    cx.run_until_parked();
    application.read_with(cx, |application, _| {
        assert!(application.preview.visible);
    });
    cx.executor()
        .advance_clock(QUICK_LAUNCH_PREVIEW_DISAPPEAR_DURATION);
    cx.run_until_parked();
    application.read_with(cx, |application, _| {
        assert!(!application.preview.visible);
    });

    let failed_reveal = uuid::Uuid::new_v4();
    application.update(cx, |application, cx| {
        let window_identity = application
            .preview
            .window
            .as_ref()
            .expect("failed reveal preview window")
            .identity;
        application.preview.generation = 20;
        application.preview.hovered_entry_id = Some("second".into());
        application.preview.visible = true;
        application.preview.pending_stage = Some(QuickLaunchPreviewPendingStage {
            command_id: failed_reveal,
            phase: QuickLaunchPreviewStagePhase::RevealOnscreen,
            window_identity,
            generation: 20,
            entry_id: "second".into(),
            payload: None,
            preview_size: size(
                px(QUICK_LAUNCH_PREVIEW_WIDTH),
                px(QUICK_LAUNCH_PREVIEW_HEIGHT),
            ),
            bounds: panel_bounds,
            panel_scale_factor: 1.0,
        });
        application.handle_command_result(
            failed_reveal,
            PlatformCommandResult {
                success: false,
                error: Some("move failed".into()),
                native_error_code: None,
                data: None,
            },
            cx,
        );
    });
    application.read_with(cx, |application, _| {
        assert!(!application.preview.visible);
        assert!(application.preview.displayed_entry_id.is_none());
        assert!(application.preview.pending_stage.is_none());
    });

    let successful_reveal = uuid::Uuid::new_v4();
    application.update(cx, |application, cx| {
        let window_identity = application
            .preview
            .window
            .as_ref()
            .expect("successful reveal preview window")
            .identity;
        application.preview.generation = 21;
        application.preview.hovered_entry_id = Some("second".into());
        application.preview.visible = true;
        application.preview.pending_stage = Some(QuickLaunchPreviewPendingStage {
            command_id: successful_reveal,
            phase: QuickLaunchPreviewStagePhase::RevealOnscreen,
            window_identity,
            generation: 21,
            entry_id: "second".into(),
            payload: None,
            preview_size: size(
                px(QUICK_LAUNCH_PREVIEW_WIDTH),
                px(QUICK_LAUNCH_PREVIEW_HEIGHT),
            ),
            bounds: panel_bounds,
            panel_scale_factor: 1.0,
        });
        application.handle_command_result(
            successful_reveal,
            PlatformCommandResult {
                success: true,
                error: None,
                native_error_code: None,
                data: None,
            },
            cx,
        );
    });
    application.read_with(cx, |application, _| {
        assert!(application.preview.visible);
        assert_eq!(
            application.preview.displayed_entry_id.as_deref(),
            Some("second")
        );
        assert!(application.preview.pending_stage.is_none());
    });

    let stale_window_stage = uuid::Uuid::new_v4();
    application.update(cx, |application, cx| {
        let preview_window = application
            .preview
            .window
            .as_mut()
            .expect("stale stage preview window");
        preview_window.identity.native = 111;
        let window_identity = preview_window.identity;
        application.preview.generation = 22;
        application.preview.hovered_entry_id = Some("second".into());
        application.preview.visible = true;
        application.preview.pending_stage = Some(QuickLaunchPreviewPendingStage {
            command_id: stale_window_stage,
            phase: QuickLaunchPreviewStagePhase::RevealOnscreen,
            window_identity,
            generation: 22,
            entry_id: "second".into(),
            payload: None,
            preview_size: size(
                px(QUICK_LAUNCH_PREVIEW_WIDTH),
                px(QUICK_LAUNCH_PREVIEW_HEIGHT),
            ),
            bounds: panel_bounds,
            panel_scale_factor: 1.0,
        });
        application
            .preview
            .window
            .as_mut()
            .expect("replacement preview window")
            .identity
            .native = 222;
        application.preview.displayed_entry_id = Some("current-window".into());
        application.handle_command_result(
            stale_window_stage,
            PlatformCommandResult {
                success: true,
                error: None,
                native_error_code: None,
                data: None,
            },
            cx,
        );
    });
    application.read_with(cx, |application, _| {
        assert_eq!(
            application.preview.displayed_entry_id.as_deref(),
            Some("current-window"),
            "a stale stage must not publish into a replacement native window"
        );
        assert!(application.preview.pending_stage.is_none());
    });
}

#[test]
fn popup_dimensions_are_compact_and_do_not_depend_on_the_main_window() {
    assert_eq!(PANEL_WIDTH, 620.0);
    assert_eq!(PANEL_HEIGHT, 530.0);
    assert_eq!(LAUNCHER_LIST_MAX_HEIGHT, 470.0);
    assert_eq!(SEARCH_HEADER_HEIGHT, 58.0);
    assert_eq!(RESULT_LIMIT, 200);
    assert_eq!(INDEX_CANDIDATE_LIMIT, 200);
}

#[test]
fn launcher_keeps_two_hundred_ranked_results_scrollable() {
    let mut session = IndexSearchSession::default();
    let request = session.begin("needle", INDEX_CANDIDATE_LIMIT);
    assert_eq!(request.limit, 200);

    let temp = tempfile::tempdir().expect("temporary quick-launch catalog");
    let root = temp.path().join("empty-applications");
    std::fs::create_dir_all(&root).expect("empty application root");
    let catalog = FinderCatalog::open(
        temp.path().join("history.db"),
        &[nxtools_finder::ApplicationRoot {
            path: root,
            source: ApplicationSource::StartMenu,
        }],
    )
    .expect("test quick-launch catalog");
    let external = (0..260)
        .map(|index| ExternalSearchEntry {
            id: format!("indexed:{index}"),
            name: format!("needle-{index:02}"),
            target: format!(r"C:\needle-{index:02}.txt"),
            source: ApplicationSource::IndexedFile,
        })
        .collect::<Vec<_>>();

    let results = catalog
        .search_with_external("needle", &external, RESULT_LIMIT)
        .expect("merged quick-launch results");

    assert_eq!(results.len(), 200);
}

#[test]
fn stationary_pointer_does_not_steal_keyboard_selection() {
    let mut gate = PointerSelectionGate::default();

    assert!(!gate.observe_move(3, (120.0, 80.0)));
    gate.reset();
    assert!(!gate.observe_move(3, (120.0, 80.0)));
    assert!(!gate.observe_move(3, (120.0, 80.0)));
    assert!(gate.observe_move(3, (121.0, 80.0)));
    assert!(!gate.observe_move(4, (121.0, 94.0)));
    assert!(gate.observe_move(4, (122.0, 94.0)));
}

#[test]
fn popup_hides_only_for_external_focus_or_pointer_after_opening_grace() {
    assert!(!should_hide_for_foreground(true, false, 10, 11, 20));
    assert!(!should_hide_for_foreground(true, true, 10, 11, 10));
    assert!(!should_hide_for_foreground(true, true, 10, 11, 11));
    assert!(should_hide_for_foreground(true, true, 10, 11, 20));
    assert!(!should_hide_for_foreground(false, true, 10, 11, 20));
    assert!(!should_hide_for_pointer(true, 10, 10, 200, 100));
    assert!(should_hide_for_pointer(true, 10, 20, 200, 100));
    assert!(!should_hide_for_pointer(true, 10, 20, 100, 100));
}

#[test]
fn popup_activation_retries_topmost_and_finishes_with_post_activation_confirmation() {
    use QuickLaunchWindowActivationStep::*;

    assert_eq!(
        next_window_activation_step(PreTopmost(0), false),
        Some(PreTopmost(1))
    );
    assert_eq!(
        next_window_activation_step(PreTopmost(1), false),
        Some(Activate(0))
    );
    assert_eq!(
        next_window_activation_step(PreTopmost(0), true),
        Some(Activate(0))
    );
    assert_eq!(
        next_window_activation_step(Activate(0), false),
        Some(Activate(1))
    );
    assert_eq!(next_window_activation_step(Activate(1), false), None);
    assert_eq!(
        next_window_activation_step(Activate(0), true),
        Some(ConfirmTopmost(0))
    );
    assert_eq!(
        next_window_activation_step(ConfirmTopmost(0), false),
        Some(ConfirmTopmost(1))
    );
    assert_eq!(
        next_window_activation_step(ConfirmTopmost(1), false),
        Some(ConfirmTopmost(2))
    );
    assert_eq!(next_window_activation_step(ConfirmTopmost(2), false), None);
    assert_eq!(next_window_activation_step(ConfirmTopmost(0), true), None);
}

#[gpui_kit::test]
fn popup_activation_owner_stops_on_foreground_failure_and_preserves_success_chain(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_kit::init);
    let (_temp, application, endpoint) = quick_launch_application_fixture(cx);
    let (panel, window) = quick_launch_panel_fixture(cx, Vec::new());
    let popup_window = window.update(|window, _| window.window_handle());
    let native_window_handle = 77;
    let result = |success| PlatformCommandResult {
        success,
        error: (!success).then(|| "foreground_not_confirmed".into()),
        native_error_code: (!success).then_some(5),
        data: None,
    };

    panel.update_in(window, |panel, window, cx| {
        panel.application = application.downgrade();
        assert!(!panel.search_input.focus_handle(cx).is_focused(window));
    });
    application.update(window, |application, cx| {
        application.popup = Some(QuickLaunchWindow {
            window: popup_window,
            panel: panel.downgrade(),
            native_window_handle,
        });
        application.begin_window_activation(native_window_handle, cx);
    });

    let pre_topmost = endpoint
        .try_receive_command_for_contract_test()
        .expect("pre-activation topmost command");
    assert!(matches!(
        pre_topmost.message,
        nxtools_protocol::Message::PlatformCommand(PlatformCommand::SetWindowTopmost(_))
    ));
    application.read_with(window, |application, _| {
        assert_eq!(
            application
                .pending_window_activation
                .get(&pre_topmost.id)
                .expect("pre-topmost pending stage")
                .step,
            QuickLaunchWindowActivationStep::PreTopmost(0)
        );
    });
    application.update(window, |application, cx| {
        application.handle_platform_notification(
            PlatformNotification::CommandResult {
                correlation_id: pre_topmost.id,
                result: result(true),
            },
            cx,
        );
    });

    let first_activate = endpoint
        .try_receive_command_for_contract_test()
        .expect("first foreground activation command");
    assert!(matches!(
        first_activate.message,
        nxtools_protocol::Message::PlatformCommand(PlatformCommand::ActivateWindow(_))
    ));
    application.update(window, |application, cx| {
        application.handle_platform_notification(
            PlatformNotification::CommandResult {
                correlation_id: first_activate.id,
                result: result(false),
            },
            cx,
        );
    });
    let second_activate = endpoint
        .try_receive_command_for_contract_test()
        .expect("bounded foreground activation retry");
    application.read_with(window, |application, _| {
        assert_eq!(
            application
                .pending_window_activation
                .get(&second_activate.id)
                .expect("activation retry pending stage")
                .step,
            QuickLaunchWindowActivationStep::Activate(1)
        );
    });
    application.update(window, |application, cx| {
        application.handle_platform_notification(
            PlatformNotification::CommandResult {
                correlation_id: second_activate.id,
                result: result(false),
            },
            cx,
        );
    });
    window.run_until_parked();

    assert!(
        endpoint.try_receive_command_for_contract_test().is_err(),
        "exhausted activation must not queue ConfirmTopmost"
    );
    application.read_with(window, |application, cx| {
        assert!(application.pending_window_activation.is_empty());
        let notices = application.notifications.read(cx).items();
        assert_eq!(notices.len(), 1);
        assert_eq!(
            notices[0].level,
            crate::notifications::NotificationLevel::Error
        );
        assert!(notices[0].message.contains("点击搜索框"));
    });
    panel.update_in(window, |panel, window, cx| {
        assert!(!panel.search_input.focus_handle(cx).is_focused(window));
        assert!(panel.status.is_none());
        assert!(panel.is_open);
    });

    application.update(window, |application, cx| {
        application.begin_window_activation(native_window_handle, cx);
    });
    let stale_pre_topmost = endpoint
        .try_receive_command_for_contract_test()
        .expect("superseded pre-topmost command");
    application.update(window, |application, cx| {
        application.popup_session_id = uuid::Uuid::new_v4();
        application.begin_window_activation(native_window_handle, cx);
    });
    let current_pre_topmost = endpoint
        .try_receive_command_for_contract_test()
        .expect("replacement pre-topmost command");
    application.update(window, |application, cx| {
        application.handle_platform_notification(
            PlatformNotification::CommandResult {
                correlation_id: stale_pre_topmost.id,
                result: result(true),
            },
            cx,
        );
    });
    application.read_with(window, |application, _| {
        assert_eq!(application.pending_window_activation.len(), 1);
        assert!(
            application
                .pending_window_activation
                .contains_key(&current_pre_topmost.id),
            "an old result must not consume the replacement activation"
        );
    });
    assert!(endpoint.try_receive_command_for_contract_test().is_err());

    application.update(window, |application, cx| {
        application.handle_platform_notification(
            PlatformNotification::CommandResult {
                correlation_id: current_pre_topmost.id,
                result: result(true),
            },
            cx,
        );
    });
    let successful_activate = endpoint
        .try_receive_command_for_contract_test()
        .expect("successful foreground activation command");
    application.update(window, |application, cx| {
        application.handle_platform_notification(
            PlatformNotification::CommandResult {
                correlation_id: successful_activate.id,
                result: result(true),
            },
            cx,
        );
    });
    let confirm_topmost = endpoint
        .try_receive_command_for_contract_test()
        .expect("post-activation topmost confirmation");
    application.read_with(window, |application, _| {
        assert_eq!(
            application
                .pending_window_activation
                .get(&confirm_topmost.id)
                .expect("confirm-topmost pending stage")
                .step,
            QuickLaunchWindowActivationStep::ConfirmTopmost(0)
        );
    });
    panel.update_in(window, |panel, window, cx| {
        assert!(panel.search_input.focus_handle(cx).is_focused(window));
    });
    application.update(window, |application, cx| {
        application.handle_platform_notification(
            PlatformNotification::CommandResult {
                correlation_id: confirm_topmost.id,
                result: result(true),
            },
            cx,
        );
    });

    application.read_with(window, |application, cx| {
        assert!(application.pending_window_activation.is_empty());
        assert_eq!(application.notifications.read(cx).items().len(), 1);
    });
    assert!(endpoint.try_receive_command_for_contract_test().is_err());
}

#[test]
fn search_focus_waits_until_windows_confirms_foreground_activation() {
    use QuickLaunchWindowActivationStep::*;

    assert!(!should_focus_search_after_activation(PreTopmost(0), true));
    assert!(!should_focus_search_after_activation(Activate(0), false));
    assert!(should_focus_search_after_activation(Activate(0), true));
    assert!(!should_focus_search_after_activation(
        ConfirmTopmost(0),
        true
    ));
}

#[test]
fn enter_modifiers_map_to_open_reveal_and_elevated_actions() {
    assert_eq!(
        quick_launch_action_for_enter(false, false),
        QuickLaunchAction::Open
    );
    assert_eq!(
        quick_launch_action_for_enter(true, false),
        QuickLaunchAction::Reveal
    );
    assert_eq!(
        quick_launch_action_for_enter(true, true),
        QuickLaunchAction::RunElevated
    );
    assert_eq!(
        shell_action_for(ApplicationSource::StartMenu, &QuickLaunchAction::Open),
        Some(ShellOpenAction::Open)
    );
    assert_eq!(
        shell_action_for(ApplicationSource::System, &QuickLaunchAction::Open),
        Some(ShellOpenAction::SystemTool)
    );
    assert_eq!(
        shell_action_for(ApplicationSource::System, &QuickLaunchAction::Reveal),
        None
    );
    assert_eq!(
        shell_action_for(ApplicationSource::System, &QuickLaunchAction::RunElevated),
        Some(ShellOpenAction::RunElevated)
    );
    assert_eq!(
        shell_action_for(ApplicationSource::IndexedFile, &QuickLaunchAction::Open),
        Some(ShellOpenAction::Open)
    );
    assert_eq!(
        shell_action_for(
            ApplicationSource::IndexedDirectory,
            &QuickLaunchAction::Reveal
        ),
        Some(ShellOpenAction::SelectInExplorer)
    );
}

#[test]
fn catalog_refresh_preserves_the_selected_entry_and_falls_back_to_first() {
    fn result(id: &str) -> SearchResult {
        SearchResult {
            entry: nxtools_finder::ApplicationEntry {
                id: id.into(),
                name: id.into(),
                target: format!("{id}.lnk"),
                source: ApplicationSource::StartMenu,
            },
            action: SearchResultAction::OpenTarget,
            subtitle: None,
            score: 0,
            activation_count: 0,
            last_activated_at_unix_ms: None,
            query_activation_count: 0,
            last_query_activation_at_unix_ms: None,
        }
    }

    let current = vec![result("alpha"), result("beta")];
    let reordered = vec![result("beta"), result("alpha")];
    let replaced = vec![result("gamma")];

    assert_eq!(selection_after_results_refresh(&current, 1, &reordered), 0);
    assert_eq!(selection_after_results_refresh(&current, 1, &replaced), 0);
}

#[test]
fn catalog_refresh_never_replaces_the_frozen_action_mode_parent() {
    assert_eq!(
        quick_launch_catalog_refresh_query(true, false, "parent query".into()),
        Some("parent query".into())
    );
    assert_eq!(
        quick_launch_catalog_refresh_query(true, true, "action filter".into()),
        None
    );
    assert_eq!(
        quick_launch_catalog_refresh_query(false, false, "closed query".into()),
        None
    );
}

#[test]
fn empty_query_keeps_the_launcher_collapsed_until_the_user_types() {
    assert!(!quick_launch_results_visible("", false, false));
    assert!(!quick_launch_results_visible("   ", false, false));
    assert!(quick_launch_results_visible("zed", false, false));
    assert!(quick_launch_results_visible("", true, false));
    assert!(quick_launch_results_visible("", false, true));
    assert_eq!(
        quick_launch_panel_height(0.0, false),
        COLLAPSED_PANEL_HEIGHT
    );
    assert_eq!(
        quick_launch_panel_height(LAUNCHER_LIST_MAX_HEIGHT, false),
        PANEL_HEIGHT
    );
}

#[test]
fn empty_result_copy_is_provider_neutral() {
    assert_eq!(QUICK_LAUNCH_EMPTY_RESULTS_LABEL, "没有匹配项");
}

#[test]
fn ime_confirmation_enter_is_consumed_but_the_following_enter_executes() {
    let mut gate = ImeEnterGate::default();
    gate.observe_composition(true);
    gate.observe_change();

    assert!(!gate.consume_enter());
    assert!(gate.consume_enter());
}

#[test]
fn direct_unicode_input_does_not_look_like_an_ime_confirmation() {
    let mut gate = ImeEnterGate::default();
    gate.observe_change();

    assert!(gate.consume_enter());
}

#[test]
fn index_search_requires_a_connected_searchable_snapshot() {
    assert!(!should_search_indexer(false, false));
    assert!(!should_search_indexer(true, false));
    assert!(should_search_indexer(true, true));
}

#[test]
fn every_non_empty_query_keeps_the_indexed_file_provider_enabled() {
    assert!(!query_requests_file_index(""));
    assert!(query_requests_file_index("a"));
    assert!(query_requests_file_index("* A "));
    assert!(query_requests_file_index("ab"));
    assert!(query_requests_file_index("中"));
    assert!(query_requests_file_index("^a"));
}

#[test]
fn a_pending_query_keeps_the_last_committed_rows_until_the_combined_batch_arrives() {
    let mut committed = vec!["previous-result"];

    let published = apply_search_update(&mut committed, Vec::new(), true);

    assert!(!published);
    assert_eq!(committed, vec!["previous-result"]);
}

#[test]
fn a_rebuilding_index_remains_searchable_while_the_last_snapshot_is_available() {
    let rebuilding_with_snapshot = IndexStatus {
        phase: nxtools_protocol::IndexPhase::VolumesReady,
        indexed_items: 2_000_000,
        drives: Vec::new(),
        last_error: Some("rebuilding in background".into()),
    };
    let initial_build_without_snapshot = IndexStatus {
        indexed_items: 0,
        ..rebuilding_with_snapshot.clone()
    };

    assert!(index_status_has_searchable_snapshot(Some(
        &rebuilding_with_snapshot
    )));
    assert!(!index_status_has_searchable_snapshot(Some(
        &initial_build_without_snapshot
    )));
}

#[test]
fn a_ready_status_retries_a_query_entered_during_indexer_reconnect() {
    let mut session = IndexSearchSession::default();
    session.begin("quick-launch-plan", RESULT_LIMIT);
    session.invalidate();
    assert!(
        session.query.is_empty(),
        "disconnect invalidation must not prevent the visible panel query from being retried"
    );
    let ready = IndexStatus {
        phase: nxtools_protocol::IndexPhase::Ready,
        indexed_items: 2_000_000,
        drives: Vec::new(),
        last_error: None,
    };

    assert!(should_retry_index_search_after_status(None, &ready));
    assert!(!should_retry_index_search_after_status(
        Some(&ready),
        &ready
    ));
    let building = IndexStatus {
        phase: nxtools_protocol::IndexPhase::VolumesReady,
        ..ready.clone()
    };
    assert!(!should_retry_index_search_after_status(None, &building));
}

#[test]
fn a_cursor_only_index_status_update_does_not_repaint_the_launcher() {
    let mut status = Some(IndexStatus {
        phase: nxtools_protocol::IndexPhase::Ready,
        indexed_items: 42,
        drives: vec![nxtools_protocol::IndexDriveStatus {
            maintenance: None,
            mount_point: r"C:\".into(),
            file_system: "NTFS".into(),
            indexed_items: 42,
            journal_id: Some(17),
            next_usn: Some(100),
        }],
        last_error: None,
    });
    let mut session = IndexSearchSession::default();
    let mut cursor_advanced = status.clone().expect("initial status");
    cursor_advanced.drives[0].next_usn = Some(101);

    let changed = apply_indexer_notification(
        &mut status,
        &mut session,
        IndexerNotification::Event(IndexerEvent::Status(cursor_advanced)),
    );

    assert!(!changed);
    assert_eq!(
        status.expect("updated status").drives[0].next_usn,
        Some(101)
    );
}

#[test]
fn result_rows_have_one_selection_visual_and_no_type_suffix() {
    let source = include_str!("../quick_launch/panel.rs");
    let render_result = source
        .split("fn render_result(")
        .nth(1)
        .and_then(|source| source.split("fn render_action(").next())
        .expect("render_result source");

    assert!(render_result.contains(".on_hover("));
    assert!(render_result.contains(".on_mouse_move("));
    assert!(!render_result.contains("result.entry.source.label()"));
}

#[test]
fn preview_header_uses_shared_body_and_label_roles() {
    let source = include_str!("../quick_launch/preview.rs");
    let render = source
        .split("impl Render for QuickLaunchPreview")
        .nth(1)
        .expect("Quick Launch preview render");

    assert!(render.contains(".text_size(px(TEXT_BODY))"));
    assert!(render.contains(".line_height(px(LINE_HEIGHT_BODY))"));
    assert!(render.contains(".text_size(px(TEXT_LABEL))"));
    assert!(render.contains(".line_height(px(LINE_HEIGHT_LABEL))"));
    assert!(!render.contains("QUICK_LAUNCH_PREVIEW_TITLE_SIZE"));
}

#[test]
fn every_preview_payload_keeps_the_reused_window_at_the_readable_reference_height() {
    let metadata = QuickLaunchPreviewPayload {
        entry_id: String::new(),
        name: String::new(),
        target: String::new(),
        detail: String::new(),
        created_at: None,
        modified_at: None,
        body: QuickLaunchPreviewBody::Metadata {
            icon: MaterialIcon::Info,
        },
    };
    let text = QuickLaunchPreviewPayload {
        entry_id: String::new(),
        name: String::new(),
        target: String::new(),
        detail: String::new(),
        created_at: None,
        modified_at: None,
        body: QuickLaunchPreviewBody::Text {
            text: String::new(),
            truncated: false,
        },
    };

    assert_eq!(
        quick_launch_preview_height(&metadata),
        QUICK_LAUNCH_PREVIEW_HEIGHT
    );
    assert_eq!(
        quick_launch_preview_height(&text),
        QUICK_LAUNCH_PREVIEW_HEIGHT
    );
}

#[test]
fn quick_launch_preview_classifies_lightweight_content_without_opening_missing_targets() {
    let temp = tempfile::tempdir().expect("temporary preview directory");
    let image = temp.path().join("sample.PNG");
    let animated_gif = temp.path().join("animated.gif");
    let animated_webp = temp.path().join("animated.webp");
    let text = temp.path().join("notes.md");
    let binary = temp.path().join("archive.zip");
    std::fs::write(&image, b"not decoded by the classifier").expect("image fixture");
    std::fs::write(&animated_gif, b"GIF89a").expect("GIF fixture");
    std::fs::write(&animated_webp, b"RIFF").expect("WebP fixture");
    std::fs::write(&text, "一段预览文本").expect("text fixture");
    std::fs::write(&binary, b"PK").expect("binary fixture");

    assert_eq!(
        quick_launch_preview_kind(&image),
        Some(QuickLaunchPreviewKind::Image)
    );
    assert_eq!(
        quick_launch_preview_kind(&text),
        Some(QuickLaunchPreviewKind::Text)
    );
    assert_eq!(
        quick_launch_preview_kind(&animated_gif),
        Some(QuickLaunchPreviewKind::Metadata)
    );
    assert_eq!(
        quick_launch_preview_kind(&animated_webp),
        Some(QuickLaunchPreviewKind::Metadata)
    );
    assert_eq!(
        quick_launch_preview_kind(&binary),
        Some(QuickLaunchPreviewKind::Metadata)
    );
    assert_eq!(
        quick_launch_preview_kind(&temp.path().join("missing.txt")),
        None
    );
}

#[test]
fn quick_launch_unavailable_targets_still_produce_a_metadata_preview() {
    let temp = tempfile::tempdir().expect("temporary preview directory");
    let missing = temp.path().join("unavailable-result.exe");
    let mut result = search_result("unavailable", ApplicationSource::IndexedFile);
    result.entry.target = missing.to_string_lossy().into_owned();

    let payload = load_quick_launch_preview(&result)
        .expect("every visible result must still have a metadata preview");
    assert_eq!(payload.entry_id, result.entry.id);
    assert!(payload.detail.contains("不可访问"));
    assert!(payload.created_at.is_none());
    assert!(payload.modified_at.is_none());
    assert!(matches!(
        payload.body,
        QuickLaunchPreviewBody::Metadata { .. }
    ));
}

#[test]
fn quick_launch_text_preview_is_bounded_and_marks_truncation() {
    let temp = tempfile::tempdir().expect("temporary preview directory");
    let path = temp.path().join("large.txt");
    std::fs::write(&path, vec![b'a'; QUICK_LAUNCH_TEXT_PREVIEW_LIMIT + 32])
        .expect("large text fixture");
    let mut result = search_result("large", ApplicationSource::IndexedFile);
    result.entry.target = path.to_string_lossy().into_owned();

    let payload = load_quick_launch_preview(&result).expect("preview payload");
    #[cfg(windows)]
    assert!(payload.created_at.is_some());
    assert!(payload.modified_at.is_some());
    assert!(payload.modified_at.as_deref().is_some_and(|value| {
        value
            .chars()
            .filter(|character| character.is_ascii_digit())
            .count()
            == 14
    }));
    let QuickLaunchPreviewBody::Text { text, truncated } = payload.body else {
        panic!("text files must use the bounded text preview body");
    };
    assert!(truncated);
    assert!(text.len() <= QUICK_LAUNCH_TEXT_PREVIEW_LIMIT);
}

#[test]
fn quick_launch_image_preview_rejects_large_files_and_decode_surfaces() {
    assert!(quick_launch_image_within_budget(256 * 1024, 3840, 2160));
    assert!(!quick_launch_image_within_budget(
        QUICK_LAUNCH_IMAGE_FILE_LIMIT + 1,
        1920,
        1080
    ));
    assert!(!quick_launch_image_within_budget(1024, 8192, 8192));
    assert!(!quick_launch_image_within_budget(1024, 0, 1080));
    assert!(!quick_launch_image_within_budget(
        1024,
        usize::MAX,
        usize::MAX
    ));
}

#[test]
fn quick_launch_static_image_preview_owns_the_validated_bounded_bytes() {
    let temp = tempfile::tempdir().expect("temporary preview directory");
    let path = temp.path().join("pixel.bmp");
    let bytes = [
        0x42, 0x4d, 58, 0, 0, 0, 0, 0, 0, 0, 54, 0, 0, 0, 40, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 1,
        0, 24, 0, 0, 0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        255, 0,
    ];
    std::fs::write(&path, bytes).expect("BMP fixture");
    let mut result = search_result("pixel", ApplicationSource::IndexedFile);
    result.entry.target = path.to_string_lossy().into_owned();

    let payload = load_quick_launch_preview(&result).expect("preview payload");
    let QuickLaunchPreviewBody::Image(image) = payload.body else {
        panic!("bounded static images must own the validated bytes");
    };
    assert_eq!(image.format, ImageFormat::Bmp);
    assert_eq!(image.bytes, bytes);
}

#[test]
fn quick_launch_preview_window_is_reused_for_target_changes() {
    assert_eq!(
        quick_launch_preview_open_action(false, false, None, "a"),
        QuickLaunchPreviewOpenAction::Create
    );
    assert_eq!(
        quick_launch_preview_open_action(true, true, Some("a"), "a"),
        QuickLaunchPreviewOpenAction::Ignore
    );
    assert_eq!(
        quick_launch_preview_open_action(true, true, Some("a"), "b"),
        QuickLaunchPreviewOpenAction::Reuse
    );
    assert!(quick_launch_preview_target_changed(true, Some("a"), "b"));
    assert!(!quick_launch_preview_target_changed(true, Some("a"), "a"));
    assert!(QUICK_LAUNCH_PREVIEW_SWITCH_DELAY < QUICK_LAUNCH_PREVIEW_HOVER_DELAY);
}

#[test]
fn reused_quick_launch_preview_waits_for_offscreen_confirmation_before_repainting() {
    let source = include_str!("../quick_launch.rs");
    let stage = source
        .split("fn stage_reused_quick_launch_preview(")
        .nth(1)
        .and_then(|source| {
            source
                .split("fn handle_quick_launch_preview_stage_result(")
                .next()
        })
        .expect("quick-launch preview staging source");
    assert!(stage.contains("self.preview.pending_stage = Some(stage)"));
    assert!(!stage.contains("preview.set_payload"));

    let completion = source
        .split("fn handle_quick_launch_preview_stage_result(")
        .nth(1)
        .and_then(|source| {
            source
                .split("fn render_staged_quick_launch_preview(")
                .next()
        })
        .expect("quick-launch preview stage completion source");
    assert!(completion.contains("render_staged_quick_launch_preview"));
    assert!(completion.contains("QuickLaunchPreviewStagePhase::RevealOnscreen"));

    let set_payload = include_str!("../quick_launch/preview.rs")
        .split("fn set_payload(")
        .nth(1)
        .and_then(|source| source.split("impl Render for QuickLaunchPreview").next())
        .expect("quick-launch preview target update source");
    assert!(set_payload.contains("self.release_image_asset(cx)"));
    assert!(set_payload.contains("self.begin_enter(cx)"));

    let hide = source
        .split("fn hide_quick_launch_preview_window(")
        .nth(1)
        .and_then(|source| source.split("fn show_quick_launch_preview(").next())
        .expect("quick-launch preview hide source");
    assert!(hide.contains("self.preview.image_asset.take()"));
    assert!(hide.contains("remove_asset(cx)"));
    assert!(hide.contains("preview.clear_image_asset(cx)"));
}

#[test]
fn quick_launch_preview_has_short_generation_scoped_enter_and_exit_motion() {
    let source = include_str!("../quick_launch.rs");
    let preview = include_str!("../quick_launch/preview.rs")
        .split("impl QuickLaunchPreview {")
        .nth(1)
        .expect("quick-launch preview source");
    assert!(preview.contains("fn begin_exit("));
    assert!(preview.contains(".opacity("));
    assert!(preview.contains("QUICK_LAUNCH_PREVIEW_DISAPPEAR_DURATION"));

    let hide = source
        .split("fn animate_quick_launch_preview_exit(")
        .nth(1)
        .and_then(|source| {
            source
                .split("fn suspend_quick_launch_preview_for_scroll")
                .next()
        })
        .expect("quick-launch preview animated hide source");
    assert!(hide.contains("begin_exit(cx)"));
    assert!(hide.contains("timer(QUICK_LAUNCH_PREVIEW_DISAPPEAR_DURATION)"));
    assert!(PREVIEW_APPEAR_DURATION <= Duration::from_millis(140));
}

#[test]
fn every_result_scroll_path_suspends_the_hover_preview() {
    let source = include_str!("../quick_launch.rs");
    let panel = include_str!("../quick_launch/panel.rs");
    let render = panel
        .split("impl Render for QuickLaunchPanel")
        .nth(1)
        .and_then(|source| source.split("fn source_icon(").next())
        .expect("quick-launch panel render source");
    assert!(render.contains(".on_scroll("));
    assert!(render.contains("suspend_quick_launch_preview_for_scroll"));

    let pointer_handler = source
        .split("PlatformEvent::GlobalPointerPressed(pointer)")
        .nth(1)
        .and_then(|source| source.split("PlatformEvent::ClipboardCaptured").next())
        .expect("quick-launch global pointer handler");
    assert!(pointer_handler.contains("pointer.root_window_handle == popup.native_window_handle"));
    assert!(pointer_handler.contains("suspend_quick_launch_preview_for_scroll"));
}

#[test]
fn quick_launch_preview_docks_beside_the_panel_and_stays_inside_the_display() {
    let screen = Bounds::new(point(px(0.0), px(0.0)), size(px(1200.0), px(700.0)));
    let preview_size = size(
        px(QUICK_LAUNCH_PREVIEW_WIDTH),
        px(QUICK_LAUNCH_PREVIEW_HEIGHT),
    );
    let right_panel = Bounds::new(point(px(700.0), px(120.0)), size(px(480.0), px(410.0)));
    let left_panel = Bounds::new(point(px(20.0), px(620.0)), size(px(620.0), px(410.0)));

    let right = quick_launch_preview_bounds(right_panel, preview_size, screen);
    assert!(right.right() <= screen.right());
    assert!(right.origin.x < right_panel.origin.x);

    let left = quick_launch_preview_bounds(left_panel, preview_size, screen);
    assert!(left.origin.x >= screen.origin.x);
    assert!(left.bottom() <= screen.bottom());
    assert!(left.origin.x >= left_panel.right());
}

#[test]
fn result_and_action_rows_share_one_full_width_shell() {
    let source = include_str!("../quick_launch/panel.rs");
    let row_shell = source
        .split("fn quick_launch_row_shell(")
        .nth(1)
        .and_then(|source| source.split("impl QuickLaunchPanel").next())
        .expect("shared quick-launch row shell");
    let render_result = source
        .split("fn render_result(")
        .nth(1)
        .and_then(|source| source.split("fn render_action(").next())
        .expect("render_result source");
    let render_action = source
        .split("fn render_action(")
        .nth(1)
        .and_then(|source| source.split("impl Render for QuickLaunchPanel").next())
        .expect("render_action source");

    assert!(render_result.contains(".id((\"quick-launch-result\", index))"));
    assert!(render_result.contains("quick_launch_row_shell(theme, selected)"));
    assert!(render_action.contains("quick_launch_row_shell(theme, selected)"));
    assert!(row_shell.contains(".w_full()"));
    assert!(row_shell.contains(".h(px(COMPACT_RESULT_ROW_HEIGHT))"));
}

#[test]
fn result_appearance_motion_does_not_flash_opacity() {
    let source = include_str!("../quick_launch.rs");
    let production_source = source
        .split("#[cfg(test)]")
        .next()
        .expect("production source");
    assert!(!production_source.contains(".opacity(0.82 + 0.18 * delta)"));
}

#[test]
fn result_rows_use_semantic_selection_without_decorative_icon_tiles() {
    let source = include_str!("../quick_launch/panel.rs");
    let row_shell = source
        .split("fn quick_launch_row_shell(")
        .nth(1)
        .and_then(|source| source.split("impl QuickLaunchPanel").next())
        .expect("shared quick-launch row shell");

    assert!(row_shell.contains("theme.element_selected"));
    assert!(!row_shell.contains("theme.accent_soft"));
    assert!(!row_shell.contains(".bg(theme.surface_raised)"));
}

#[test]
fn result_viewport_and_keyboard_navigation_share_one_scroll_handle() {
    let source = include_str!("../quick_launch/panel.rs");
    let render = source
        .split("impl Render for QuickLaunchPanel")
        .nth(1)
        .and_then(|source| source.split("fn source_icon(").next())
        .expect("quick-launch render source");

    assert!(render.contains("uniform_list("));
    assert!(source.contains("UniformListScrollHandle"));
    assert!(source.contains("ScrollStrategy::Center"));
    assert!(render.contains(".vertical_scrollbar(&self.result_scroll)"));
    assert!(!render.contains(".children(entries)"));
}

#[test]
fn presented_result_range_drives_icon_loading_without_pointer_hover() {
    let source = include_str!("../quick_launch/panel.rs");
    let render = source
        .split("impl Render for QuickLaunchPanel")
        .nth(1)
        .and_then(|source| source.split("fn source_icon(").next())
        .expect("quick-launch render source");

    assert!(render.contains("update_presented_results"));
    assert!(render.contains("visible_range"));
    assert!(source.contains("fn update_presented_results("));
}

#[test]
fn pointer_selection_preserves_the_current_result_scroll_position() {
    let source = include_str!("../quick_launch/panel.rs");
    let select_result = source
        .split("fn select_result(")
        .nth(1)
        .and_then(|source| source.split("fn select_action(").next())
        .expect("select_result source");

    assert!(!select_result.contains("result_scroll.set_offset"));
}

#[test]
fn count_only_status_changes_coalesce_into_rate_limited_logs() {
    let base = IndexStatus {
        phase: nxtools_protocol::IndexPhase::Ready,
        indexed_items: 100,
        drives: vec![nxtools_protocol::IndexDriveStatus {
            maintenance: None,
            mount_point: r"C:\".into(),
            file_system: "NTFS".into(),
            indexed_items: 100,
            journal_id: Some(7),
            next_usn: None,
        }],
        last_error: None,
    };
    let count_change = IndexStatus {
        indexed_items: 101,
        drives: vec![nxtools_protocol::IndexDriveStatus {
            maintenance: None,
            indexed_items: 101,
            ..base.drives[0].clone()
        }],
        ..base.clone()
    };
    assert!(index_status_affects_presentation(
        Some(&base),
        &count_change
    ));
    assert!(index_status_count_only_change(Some(&base), &count_change));

    let phase_change = IndexStatus {
        phase: nxtools_protocol::IndexPhase::VolumesReady,
        ..base.clone()
    };
    assert!(!index_status_count_only_change(Some(&base), &phase_change));

    let mut journal_change = base.clone();
    journal_change.drives[0].journal_id = Some(8);
    assert!(!index_status_count_only_change(
        Some(&base),
        &journal_change
    ));

    let error_change = IndexStatus {
        last_error: Some("volume denied".into()),
        ..base.clone()
    };
    assert!(!index_status_count_only_change(Some(&base), &error_change));

    // A first status always logs; a repeat inside the interval does not.
    let now = Instant::now();
    assert!(should_log_index_status_change(true, None, now));
    assert!(!should_log_index_status_change(true, Some(now), now));
    assert!(should_log_index_status_change(
        true,
        Some(now - INDEX_COUNT_STATUS_LOG_INTERVAL),
        now
    ));
    // Structural changes log regardless of the last count-only record.
    assert!(should_log_index_status_change(false, Some(now), now));
}

#[test]
fn indexer_service_bootstrap_runs_once_and_drops_stale_pipe_commands() {
    let mut bootstrap = IndexerServiceBootstrap::default();
    let stale_command = uuid::Uuid::new_v4();
    bootstrap.pending = Some(stale_command);
    bootstrap.pending_automatic = true;

    assert!(bootstrap.on_platform_connected());
    assert_eq!(bootstrap.pending, None);
    assert!(!bootstrap.pending_automatic);
    assert!(!bootstrap.on_platform_connected());

    let mut already_connected = IndexerServiceBootstrap::default();
    assert!(already_connected.on_platform_connected());
    assert!(already_connected.automatic_attempted);
    let command = uuid::Uuid::new_v4();
    already_connected.begin(command, true);
    assert_eq!(already_connected.finish(command), Some(true));
    assert_eq!(already_connected.finish(command), None);
    assert!(!already_connected.on_platform_connected());
}
#[test]
fn validation_install_directory_skips_protected_indexer_service_management() {
    assert!(!should_manage_indexer_service_for_executable(
        std::path::Path::new(r"C:\Users\Test\NxTools.Next-Validation-20261004\NxTools.exe",)
    ));
    assert!(should_manage_indexer_service_for_executable(
        std::path::Path::new(r"C:\ProgramData\NxTools\NxTools.Next\Service\NxTools.exe",)
    ));
}

#[gpui_kit::test]
fn quick_launch_bounds_use_the_cursor_display_upper_quarter(cx: &mut TestAppContext) {
    let request = QuickLaunchPanelRequest {
        target: nxtools_protocol::ForegroundContext {
            window_handle: 1,
            focus_window_handle: 1,
            process_id: 1,
            process_name: "editor".into(),
            window_title: "document".into(),
        },
        directory_navigation_context: None,
        cursor_x: 10,
        cursor_y: 10,
        cursor_display_index: 0,
        cursor_monitor_dpi: 96,
        cursor_monitor: None,
        requested_at_unix_ms: 1_700_000_000_000,
        correlation_id: None,
    };
    cx.update(|cx| {
        let (bounds, _) = quick_launch_bounds(&request, false, cx);
        assert_eq!(
            bounds.size,
            size(px(PANEL_WIDTH), px(COLLAPSED_PANEL_HEIGHT))
        );
    });
}

#[gpui_kit::test]
fn quick_launch_bounds_keep_logical_size_at_100_125_and_150_percent_dpi(cx: &mut TestAppContext) {
    for dpi in [96, 120, 144] {
        let request = QuickLaunchPanelRequest {
            target: nxtools_protocol::ForegroundContext {
                window_handle: 1,
                focus_window_handle: 1,
                process_id: 1,
                process_name: "editor".into(),
                window_title: "document".into(),
            },
            directory_navigation_context: None,
            cursor_x: 1_440,
            cursor_y: 720,
            cursor_display_index: 0,
            cursor_monitor_dpi: dpi,
            cursor_monitor: None,
            requested_at_unix_ms: 1_700_000_000_000,
            correlation_id: None,
        };
        cx.update(|cx| {
            let (bounds, _) = quick_launch_bounds(&request, true, cx);
            assert_eq!(
                bounds.size,
                size(px(PANEL_WIDTH), px(PANEL_HEIGHT)),
                "DPI {dpi} must not change the logical popup size"
            );
        });
    }
}

#[gpui_kit::test]
fn multimonitor_launcher_uses_cursor_work_area_upper_quarter(cx: &mut TestAppContext) {
    for dpi in [96, 120, 144] {
        let request = QuickLaunchPanelRequest {
            target: nxtools_protocol::ForegroundContext {
                window_handle: 1,
                focus_window_handle: 1,
                process_id: 1,
                process_name: "editor".into(),
                window_title: String::new(),
            },
            directory_navigation_context: None,
            cursor_x: -1200,
            cursor_y: 300,
            cursor_display_index: 1,
            cursor_monitor_dpi: dpi,
            cursor_monitor: Some(nxtools_protocol::CursorMonitorSnapshot {
                display_id: 0xBEEF,
                work_left: -1920,
                work_top: 40,
                work_width: 1920,
                work_height: 1040,
                dpi,
            }),
            requested_at_unix_ms: 1_700_000_000_000,
            correlation_id: None,
        };
        let scale = dpi as f32 / 96.0;
        for expanded in [false, true] {
            let (bounds, _) = cx.update(|cx| quick_launch_bounds(&request, expanded, cx));
            let native = bounds.to_device_pixels(scale);
            assert_eq!(
                i32::from(native.origin.x),
                (-1920.0 + (1920.0 - PANEL_WIDTH * scale) / 2.0).round() as i32
            );
            let height = if expanded {
                PANEL_HEIGHT
            } else {
                COLLAPSED_PANEL_HEIGHT
            };
            let expected_top = 300.0_f32.min(1080.0 - height * scale).max(40.0);
            assert_eq!(i32::from(native.origin.y), expected_top.round() as i32);
            assert!(i32::from(native.origin.y) + i32::from(native.size.height) <= 1080);
            assert_eq!(bounds.size.width, px(PANEL_WIDTH));
        }
    }
}

#[test]
fn panel_hide_is_result_tracked_and_self_healing() {
    // The user report: the panel sometimes never disappears. The hide
    // command must be correlated and result-tracked, with one bounded
    // orphan recheck that re-sends when no confirmation arrives.
    let source = include_str!("../quick_launch.rs");
    for marker in [
        "pending_panel_hides",
        "quick_launch.panel.hide_applied",
        "quick_launch.panel.hide_unconfirmed",
        "quick_launch.panel.hide_failed",
        "schedule_panel_hide_recheck",
        "PANEL_HIDE_RECHECK_DELAY_MS",
        "PANEL_HIDE_RETRY_LIMIT",
    ] {
        assert!(
            source.contains(marker),
            "panel hide lifecycle must contain {marker}"
        );
    }
    assert_eq!(PANEL_HIDE_RETRY_LIMIT, 1);
    assert_eq!(PANEL_HIDE_RECHECK_DELAY_MS, 400);
}

#[test]
fn popup_creation_and_reuse_share_confirmed_monitor_presentation() {
    let source = include_str!("../quick_launch.rs");
    let open = source
        .split("fn open_popup(")
        .nth(1)
        .and_then(|body| body.split("fn schedule_captured_directory_record").next())
        .expect("quick-launch open lifecycle");
    assert!(open.contains("show: false"));
    assert!(open.contains("focus: false"));
    assert!(open.contains("self.begin_popup_presentation(placement, cx)"));
    assert!(!open.contains("visible: true"));
    let reuse = source
        .split("fn show_existing_popup(")
        .nth(1)
        .and_then(|body| body.split("fn open_popup(").next())
        .expect("reuse lifecycle");
    assert!(reuse.contains("self.begin_popup_presentation(placement, cx)"));
    assert!(!reuse.contains("PlatformCommand::SetWindowVisibility"));
    assert!(source.contains("self.retire_popup_presentation(\"hidden\")"));
}

#[test]
fn indexer_install_and_upgrade_results_share_protected_restart_policy() {
    for data in [
        "installed",
        "upgraded",
        "running",
        "repair_required",
        "authorization_cancelled",
    ] {
        let mut result = nxtools_protocol::PlatformCommandResult {
            success: true,
            error: None,
            native_error_code: None,
            data: Some(data.into()),
        };
        assert_eq!(
            indexer_deployment_requires_protected_restart(&result),
            matches!(data, "installed" | "upgraded")
        );
        result.success = false;
        assert!(!indexer_deployment_requires_protected_restart(&result));
    }
}

#[test]
fn w4_recent_modified_route_preserves_provider_order_and_separate_visit_history() {
    assert_eq!(recent_modified_filter(" recent: "), Some(""));
    assert_eq!(recent_modified_filter("RECENT: docx"), Some("docx"));
    assert_eq!(recent_modified_filter(r"C:\recent:notes"), None);
    assert_eq!(recent_modified_filter("\"recent:\""), None);
    let items = [
        IndexSearchItem {
            path: r"C:\Work\new.txt".into(),
            name: "new.txt".into(),
            is_directory: false,
            size_bytes: None,
            modified_at_unix_ms: Some(200),
            score: 200,
        },
        IndexSearchItem {
            path: r"C:\Work\old.txt".into(),
            name: "old.txt".into(),
            is_directory: false,
            size_bytes: None,
            modified_at_unix_ms: Some(100),
            score: 100,
        },
    ];
    let results = recent_modified_results(&items);
    assert_eq!(
        results
            .iter()
            .map(|result| result.entry.name.as_str())
            .collect::<Vec<_>>(),
        vec!["new.txt", "old.txt"]
    );
    assert!(results.iter().all(|result| {
        result
            .subtitle
            .as_deref()
            .is_some_and(|subtitle| subtitle.contains("最近修改"))
    }));
    assert!(
        results
            .iter()
            .all(|result| result.entry.source == ApplicationSource::IndexedFile)
    );
}

#[gpui_kit::test]
fn w5_new_query_pending_rejects_old_rows_at_enter_shortcut_pointer_and_actions(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_kit::init);
    let (_temporary, application, endpoint) = quick_launch_application_fixture(cx);
    let retained = Rc::new(RefCell::new(None));
    let slot = retained.clone();
    let owner = application.downgrade();
    let (_root, window) = cx.add_window_view(move |window, cx| {
        let panel = cx.new(|cx| {
            QuickLaunchPanel::new(
                vec![search_result("old-result", ApplicationSource::IndexedFile)],
                nxtools_protocol::ForegroundContext {
                    window_handle: 1,
                    focus_window_handle: 1,
                    process_id: 1,
                    process_name: "fixture".into(),
                    window_title: String::new(),
                },
                None,
                Vec::new(),
                owner,
                window,
                cx,
            )
        });
        *slot.borrow_mut() = Some(panel.clone());
        Root::new(panel, window, cx)
    });
    let panel = retained.borrow().clone().expect("panel");
    panel.update_in(window, |panel, window, cx| {
        panel.focus_search(window, cx);
        panel
            .search_input
            .update(cx, |input, cx| input.set_value("new-query", window, cx));
        panel.schedule_search("new-query".into(), cx);
        panel.search_input.update(cx, |_, cx| {
            cx.emit(InputEvent::PressEnter {
                secondary: false,
                shift: false,
            })
        });
    });
    panel.update_in(window, |panel, window, cx| {
        panel.visible_result_range = 0..1;
        panel.handle_navigation_key("1", gpui_kit::Modifiers::default(), window, cx);
        panel.activate_selected(
            QuickLaunchAction::Open,
            QuickLaunchActionTrigger::Pointer,
            cx,
        );
        panel.enter_action_mode(0, window, cx);
        assert!(
            panel.action_mode.is_none(),
            "pending old row cannot enter actions"
        );
    });
    window.run_until_parked();
    assert!(
        window.read(|cx| application.read(cx).pending_actions.is_empty()),
        "no old-row action dispatch"
    );
    assert!(
        endpoint.try_receive_command_for_contract_test().is_err(),
        "pending query cannot emit native execution"
    );
    panel.update_in(window, |panel, _, cx| {
        panel.search_pending = false;
        panel.results = vec![search_result("current", ApplicationSource::IndexedFile)].into();
        panel.activate_selected(
            QuickLaunchAction::Open,
            QuickLaunchActionTrigger::Pointer,
            cx,
        );
        panel.query_dispatch_generation = panel.query_dispatch_generation.wrapping_add(1);
        panel.schedule_search("newer-query".into(), cx);
    });
    window.run_until_parked();
    assert!(
        window.read(|cx| application.read(cx).pending_actions.is_empty()),
        "input supersession cancels deferred old action"
    );
    assert!(endpoint.try_receive_command_for_contract_test().is_err());

    for invalidation in ["input", "close_reopen", "action", "mode", "level"] {
        panel.update_in(window, |panel, window, cx| {
            panel.is_open = true;
            panel.search_pending = false;
            let mut mode = QuickLaunchActionMode::enter(
                search_result("current", ApplicationSource::IndexedFile),
                "parent".into(),
                0,
            );
            mode.current_level_mut().shell_pending = false;
            mode.current_level_mut().items = shell_action_items(
                "fixture-session",
                vec![ShellActionItem {
                    command_id: Some(42),
                    title: "合成 Shell 操作".into(),
                    enabled: true,
                    separator: false,
                    submenu_id: None,
                    icon_path: None,
                }],
            );
            panel.action_mode = Some(mode);
            panel.activate_selected_action(window, cx);
            match invalidation {
                "input" => panel.search_input.update(cx, |input, cx| {
                    input.set_value("changed-filter", window, cx);
                }),
                "close_reopen" => {
                    panel.close(cx);
                    panel.is_open = true;
                }
                "action" => {
                    panel
                        .action_mode
                        .as_mut()
                        .expect("mode")
                        .current_level_mut()
                        .items[0]
                        .action = Some(QuickLaunchAction::CopyPath);
                }
                "mode" => panel.action_mode.as_mut().expect("mode").id = uuid::Uuid::new_v4(),
                "level" => {
                    panel
                        .action_mode
                        .as_mut()
                        .expect("mode")
                        .current_level_mut()
                        .id = uuid::Uuid::new_v4()
                }
                _ => unreachable!("known invalidation"),
            }
        });
        window.run_until_parked();
        assert!(
            window.read(|cx| application.read(cx).pending_actions.is_empty()),
            "{invalidation} must cancel the deferred Shell action"
        );
        while let Ok(command) = endpoint.try_receive_command_for_contract_test() {
            assert!(
                !matches!(
                    command.message,
                    nxtools_protocol::Message::PlatformCommand(PlatformCommand::InvokeShellAction(
                        _
                    ))
                ),
                "{invalidation} cannot dispatch a stale Shell command"
            );
        }
    }
    panel.update_in(window, |panel, window, cx| {
        panel
            .action_mode
            .as_mut()
            .expect("mode")
            .current_level_mut()
            .shell_pending = false;
        panel.activate_selected_action(window, cx);
    });
    window.run_until_parked();
    assert!(!window.read(|cx| application.read(cx).pending_actions.is_empty()));
    assert!(matches!(
        endpoint
            .try_receive_command_for_contract_test()
            .expect("current action")
            .message,
        nxtools_protocol::Message::PlatformCommand(PlatformCommand::InvokeShellAction(_))
    ));
}

#[gpui_kit::test]
fn w4_failed_recent_query_clears_only_current_generation_and_retires_execution(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_kit::init);
    let (_temporary, application, _endpoint) = quick_launch_application_fixture(cx);
    let retained = Rc::new(RefCell::new(None));
    let slot = retained.clone();
    let owner = application.downgrade();
    let (_root, window) = cx.add_window_view(move |window, cx| {
        let panel = cx.new(|cx| {
            QuickLaunchPanel::new(
                vec![search_result("old", ApplicationSource::IndexedFile)],
                nxtools_protocol::ForegroundContext {
                    window_handle: 1,
                    focus_window_handle: 1,
                    process_id: 1,
                    process_name: "fixture".into(),
                    window_title: String::new(),
                },
                None,
                Vec::new(),
                owner,
                window,
                cx,
            )
        });
        *slot.borrow_mut() = Some(panel.clone());
        Root::new(panel, window, cx)
    });
    let panel = retained.borrow().clone().expect("panel");
    window.update(|window, cx| {
        application.update(cx, |app, _| {
            app.index_search.generation = 10;
            app.popup = Some(QuickLaunchWindow {
                window: window.window_handle(),
                panel: panel.downgrade(),
                native_window_handle: 1,
            });
        })
    });
    panel.update_in(window, |panel, _, _| panel.search_pending = true);
    for generation in [9, 10] {
        window.update(|_, cx| {
            application.update(cx, |app, cx| {
                app.handle_indexer_notification(
                    IndexerNotification::Event(IndexerEvent::SearchFailed(
                        nxtools_protocol::IndexSearchFailure {
                            generation,
                            reason: "recent_classifier_budget".into(),
                        },
                    )),
                    cx,
                )
            })
        });
        panel.update_in(window, |panel, _, _| {
            assert_eq!(panel.results.is_empty(), generation == 10);
            assert_eq!(panel.search_pending, generation != 10);
            if generation == 10 {
                assert!(
                    panel
                        .status
                        .as_deref()
                        .is_some_and(|status| status.contains("预算"))
                );
            }
        });
    }
}

#[gpui_kit::test]
fn w2_local_ordinary_hit_disables_indexed_homophone_fallback(cx: &mut TestAppContext) {
    let (_temporary, application, _platform) = quick_launch_application_fixture(cx);
    let (bridge, endpoint) = crate::runtime::indexer_bridge(8);
    endpoint.connect_for_test();
    application.update(cx, |app, _| {
        app.indexer = Rc::new(bridge);
        app.index_status = Some(IndexStatus {
            phase: nxtools_protocol::IndexPhase::Ready,
            indexed_items: 1,
            drives: Vec::new(),
            last_error: None,
        });
        app.config.quick_launch.custom_commands = vec![nxtools_core::CustomCommandSettings {
            keyword: "年度报高".into(),
            title: "本地普通命令".into(),
            program: r"C:\Fixture\tool.exe".into(),
            ..Default::default()
        }];
        let manifest = br#"{"schema_version":1,"id":"fixture.local-probe","name":"Probe","version":"1","handlers":[{"id":"open","title":"Open","feature":"quick_launch","entry":"command","keywords":["package"],"action":{"type":"folder","path":"C:\\Fixture"}}]}"#;
        app.config.extensions.install(nxtools_extensions::ImportPreview::from_json(manifest).expect("probe manifest"), "fixture".into(), false).expect("install");
        app.config.extensions.set_enabled("fixture.local-probe", true, &[]).expect("enable");
    });
    for (query, allow, recent) in [
        ("年度报高", false, false),
        ("年度报低", true, false),
        ("package", false, false),
        ("pack", false, false),
        ("recent:", false, true),
    ] {
        application.update(cx, |app, cx| {
            let update = app.search_and_request_icons(query, cx);
            assert!(
                update.pending,
                "indexed queries wait for their current providers"
            );
            assert!(update.results.is_empty());
            assert!(
                !app.index_search.complete,
                "provider completion is independent of usable current rows"
            );
            if query == "年度报高" {
                assert!(
                    app.combined_results(query)
                        .iter()
                        .any(|result| result.entry.source == ApplicationSource::CustomCommand)
                );
            }
        });
        let command = endpoint
            .try_recv_command_for_test()
            .expect("indexed ordinary search stays enabled");
        let nxtools_protocol::Message::IndexerCommand(IndexerCommand::Search(request)) =
            command.message
        else {
            panic!("search request");
        };
        assert_eq!(request.context.allow_homophone_fallback, allow, "{query}");
        assert_eq!(request.context.recent_modified, recent, "{query}");
    }
    application.update(cx, |app, cx| {
        app.config
            .extensions
            .set_enabled("fixture.local-probe", false, &[])
            .expect("disable");
        assert!(app.search_and_request_icons("package", cx).pending);
    });
    let command = endpoint
        .try_recv_command_for_test()
        .expect("disabled package query");
    let nxtools_protocol::Message::IndexerCommand(IndexerCommand::Search(request)) =
        command.message
    else {
        panic!("indexed search request");
    };
    assert!(
        request.context.allow_homophone_fallback,
        "disabled declarations cannot suppress approximation"
    );
}

#[gpui_kit::test]
fn w6_saved_package_appears_in_real_query_and_disappears_when_disabled(cx: &mut TestAppContext) {
    let (temporary, application, _endpoint) = quick_launch_application_fixture(cx);
    let path = temporary.path().join("config.json");
    let package = br#"{"schema_version":1,"id":"example.web","name":"Web","version":"1.0.0","handlers":[{"id":"web","title":"Package web","feature":"quick_launch","entry":"command","keywords":["web"],"action":{"type":"website","url_template":"https://example.test/?q={query}"}}]}"#;
    AppConfig::import_extension_json(&path, package, "fixture".into(), false).expect("import");
    for enabled in [true, false] {
        let config =
            AppConfig::set_extension_enabled(&path, "example.web", enabled).expect("save state");
        application.update(cx, |app, cx| {
            app.config = config;
            let update = app.search_and_request_icons("web hello", cx);
            assert_eq!(
                update
                    .results
                    .iter()
                    .any(|row| row.entry.id.starts_with("extension:example.web/")),
                enabled
            );
        });
    }
}

#[test]
fn w6_package_url_and_folder_actions_reuse_shell_without_elevation() {
    for target in ["https://example.test/?q=words", r"C:\Fixture\Docs"] {
        let mut result = search_result(
            "extension:example.web/web/action",
            ApplicationSource::CustomCommand,
        );
        result.entry.target = target.into();
        assert!(
            matches!(platform_command_for_action(&result, &QuickLaunchAction::Open, 0),
            Some(PlatformCommand::OpenShellTarget(command)) if command.target == target && command.action == ShellOpenAction::Open)
        );
        assert!(!quick_launch_action_available(
            &result,
            &QuickLaunchAction::RunElevated
        ));
    }
}

#[gpui_kit::test]
fn w6_disabled_package_cannot_execute_retained_result(cx: &mut TestAppContext) {
    let (temporary, application, endpoint) = quick_launch_application_fixture(cx);
    let path = temporary.path().join("config.json");
    let package = br#"{"schema_version":1,"id":"example.web","name":"Web","version":"1.0.0","handlers":[{"id":"web","title":"Package web","feature":"quick_launch","entry":"command","keywords":["web"],"action":{"type":"website","url_template":"https://example.test/?q={query}"}}]}"#;
    AppConfig::import_extension_json(&path, package, "fixture".into(), false).expect("import");
    let enabled = AppConfig::set_extension_enabled(&path, "example.web", true).expect("enable");
    application.update(cx, |app, cx| {
        app.config = enabled;
        let result = app
            .combined_results("web hello")
            .into_iter()
            .find(|row| row.entry.id.starts_with("extension:"))
            .expect("current result");
        app.config =
            AppConfig::set_extension_enabled(&path, "example.web", false).expect("disable");
        app.execute(
            result,
            QuickLaunchAction::Open,
            "web hello".into(),
            QuickLaunchActionTrigger::Pointer,
            cx,
        );
        assert!(
            app.pending_actions.is_empty(),
            "retired package must not dispatch"
        );
    });
    assert!(endpoint.try_receive_command_for_contract_test().is_err());
}

#[gpui_kit::test]
fn enter_repair_ime_commit_without_bubbled_enter_does_not_eat_next_gesture(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_kit::init);
    let (_temp, application, endpoint) = quick_launch_application_fixture(cx);
    let (panel, window) = quick_launch_panel_fixture(cx, Vec::new());
    let input = panel.read_with(window, |panel, _| panel.search_input.clone());
    panel.update_in(window, |panel, window, cx| {
        panel.application = application.downgrade();
        panel.focus_search(window, cx);
    });
    input.update_in(window, |input, window, cx| {
        input.replace_and_mark_text_in_range(None, "zhong", Some(5..5), window, cx)
    });
    window.run_until_parked();
    assert!(
        panel.read_with(window, |panel, _| panel.ime_enter_gate.composition_seen),
        "production observer saw preedit"
    );
    input.update_in(window, |input, window, cx| {
        input.replace_text_in_range(None, "中", window, cx)
    });
    // Windows may consume the confirming key inside the IME: no InputEvent::PressEnter.
    window.simulate_event(gpui_kit::KeyUpEvent {
        keystroke: gpui_kit::Keystroke::parse("enter").expect("confirm release"),
    });
    window.run_until_parked();
    panel.update_in(window, |panel, _, _| {
        panel.results = vec![search_result(
            "current-ime-result",
            ApplicationSource::IndexedFile,
        )]
        .into();
        panel.selected = 0;
        panel.search_pending = false;
    });
    window.refresh().expect("current results");
    while endpoint.try_receive_command_for_contract_test().is_ok() {}
    window.simulate_event(gpui_kit::KeyDownEvent {
        keystroke: gpui_kit::Keystroke::parse("enter").expect("ordinary enter"),
        is_held: false,
        prefer_character_input: false,
    });
    window.simulate_event(gpui_kit::KeyUpEvent {
        keystroke: gpui_kit::Keystroke::parse("enter").expect("ordinary release"),
    });
    window.run_until_parked();
    assert!(
        !application.read_with(window, |app, _| app.pending_actions.is_empty()),
        "first ordinary Enter after IME commit must dispatch current result"
    );
}

#[gpui_kit::test]
fn enter_repair_ime_cancel_does_not_eat_first_ordinary_enter(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (_temp, application, endpoint) = quick_launch_application_fixture(cx);
    let (panel, window) = quick_launch_panel_fixture(cx, Vec::new());
    let input = panel.read_with(window, |panel, _| panel.search_input.clone());
    panel.update_in(window, |panel, window, cx| {
        panel.application = application.downgrade();
        panel.focus_search(window, cx);
    });
    input.update_in(window, |input, window, cx| {
        input.replace_and_mark_text_in_range(None, "preedit", None, window, cx)
    });
    window.run_until_parked();
    input.update_in(window, |input, window, cx| {
        input.replace_and_mark_text_in_range(None, "", None, window, cx)
    });
    input.update_in(window, |input, window, cx| input.unmark_text(window, cx));
    window.simulate_event(gpui_kit::KeyUpEvent {
        keystroke: gpui_kit::Keystroke::parse("escape").expect("cancel release"),
    });
    window.run_until_parked();
    input.update_in(window, |input, window, cx| {
        input.replace_text_in_range(None, "current", window, cx)
    });
    window.run_until_parked();
    panel.update_in(window, |panel, _, _| {
        panel.results = vec![search_result(
            "current-cancel-result",
            ApplicationSource::IndexedFile,
        )]
        .into();
        panel.selected = 0;
        panel.search_pending = false;
    });
    window.refresh().expect("current results");
    while endpoint.try_receive_command_for_contract_test().is_ok() {}
    window.simulate_event(gpui_kit::KeyDownEvent {
        keystroke: gpui_kit::Keystroke::parse("enter").expect("enter"),
        is_held: false,
        prefer_character_input: false,
    });
    window.simulate_event(gpui_kit::KeyUpEvent {
        keystroke: gpui_kit::Keystroke::parse("enter").expect("release"),
    });
    window.run_until_parked();
    assert!(
        !application.read_with(window, |app, _| app.pending_actions.is_empty()),
        "cancel cannot poison future Enter"
    );
}

#[gpui_kit::test]
fn enter_repair_active_preedit_never_executes_then_plain_enter_dispatches(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (_temp, application, endpoint) = quick_launch_application_fixture(cx);
    let (panel, window) = quick_launch_panel_fixture(cx, Vec::new());
    let input = panel.read_with(window, |panel, _| panel.search_input.clone());
    panel.update_in(window, |panel, window, cx| {
        panel.application = application.downgrade();
        panel.focus_search(window, cx);
    });
    input.update_in(window, |input, window, cx| {
        input.replace_and_mark_text_in_range(None, "preedit", None, window, cx)
    });
    window.run_until_parked();
    panel.update_in(window, |panel, _, _| {
        panel.results = vec![search_result(
            "composing-result",
            ApplicationSource::IndexedFile,
        )]
        .into();
        panel.selected = 0;
        panel.search_pending = false;
    });
    window.refresh().expect("preedit results");
    while endpoint.try_receive_command_for_contract_test().is_ok() {}
    window.simulate_event(gpui_kit::KeyDownEvent {
        keystroke: gpui_kit::Keystroke::parse("enter").expect("composing enter"),
        is_held: false,
        prefer_character_input: false,
    });
    window.simulate_event(gpui_kit::KeyUpEvent {
        keystroke: gpui_kit::Keystroke::parse("enter").expect("release"),
    });
    window.run_until_parked();
    assert!(
        application.read_with(window, |app, _| app.pending_actions.is_empty()),
        "live IME Enter cannot launch"
    );
    input.update_in(window, |input, window, cx| {
        input.replace_text_in_range(None, "committed", window, cx)
    });
    // Same confirming gesture may also bubble after commit; it must be consumed.
    input.update(window, |_, cx| {
        cx.emit(InputEvent::PressEnter {
            secondary: false,
            shift: false,
        })
    });
    window.run_until_parked();
    assert!(application.read_with(window, |app, _| app.pending_actions.is_empty()));
    window.simulate_event(gpui_kit::KeyUpEvent {
        keystroke: gpui_kit::Keystroke::parse("enter").expect("confirm released"),
    });
    window.run_until_parked();
    panel.update_in(window, |panel, _, _| {
        panel.results = vec![search_result(
            "committed-result",
            ApplicationSource::IndexedFile,
        )]
        .into();
        panel.selected = 0;
        panel.search_pending = false;
    });
    window.refresh().expect("committed results");
    while endpoint.try_receive_command_for_contract_test().is_ok() {}
    window.simulate_event(gpui_kit::KeyDownEvent {
        keystroke: gpui_kit::Keystroke::parse("enter").expect("plain enter"),
        is_held: false,
        prefer_character_input: false,
    });
    window.simulate_event(gpui_kit::KeyUpEvent {
        keystroke: gpui_kit::Keystroke::parse("enter").expect("plain release"),
    });
    window.run_until_parked();
    let command = endpoint
        .try_receive_command_for_contract_test()
        .expect("native open");
    assert!(
        matches!(
            command.message,
            nxtools_protocol::Message::PlatformCommand(PlatformCommand::OpenShellTarget(_))
        ),
        "actual native open, not icon traffic"
    );
    assert!(!application.read_with(window, |app, _| app.pending_actions.is_empty()));
}

#[gpui_kit::test]
fn phase_d_internal_archive_open_preserves_usage_and_shell_fallbacks(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.executor().allow_parking();
    let (temp, application, endpoint) = quick_launch_application_fixture(cx);
    let path = temp.path().join("owned.zip");
    std::fs::write(&path, b"synthetic bytes").expect("source");
    let (archive_bridge, archive_endpoint) = nxtools_ipc::platform_bridge(32);
    archive_endpoint.set_connected_for_contract_test(true);
    cx.update(|cx| {
        crate::archive::register(
            Rc::new(archive_bridge),
            nxtools_core::archive::ArchiveSettings::default(),
            temp.path().join("archive-ui.json"),
            cx,
        )
    });
    let mut result = search_result("archive-owned", ApplicationSource::IndexedFile);
    result.entry.target = path.to_string_lossy().into_owned();
    application.update(cx, |app, cx| {
        app.execute_native_result(
            result.clone(),
            QuickLaunchAction::Open,
            "owned".into(),
            QuickLaunchActionTrigger::Enter,
            cx,
        )
    });
    for _ in 0..100 {
        cx.executor().advance_clock(Duration::from_millis(25));
        cx.run_until_parked();
        if application.read_with(cx, |app, _| app.pending_actions.is_empty()) {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(
        application.read_with(cx, |app, _| app
            .catalog
            .borrow()
            .history_summary()
            .total_activations),
        1
    );
    while let Ok(envelope) = endpoint.try_receive_command_for_contract_test() {
        assert!(
            !matches!(
                envelope.message,
                nxtools_protocol::Message::PlatformCommand(PlatformCommand::OpenShellTarget(_))
            ),
            "internal open must not Shell launch"
        );
    }
    cx.update(|cx| {
        crate::archive::settings_updated(
            nxtools_core::archive::ArchiveSettings {
                open_in_nxtools: false,
                ..Default::default()
            },
            cx,
        )
    });
    cx.run_until_parked();
    application.update(cx, |app, cx| {
        app.execute_native_result(
            result.clone(),
            QuickLaunchAction::Open,
            "owned".into(),
            QuickLaunchActionTrigger::Enter,
            cx,
        )
    });
    assert!(matches!(
        endpoint
            .try_receive_command_for_contract_test()
            .expect("normal Open")
            .message,
        nxtools_protocol::Message::PlatformCommand(PlatformCommand::OpenShellTarget(_))
    ));
    application.update(cx, |app, cx| {
        app.execute_native_result(
            result,
            QuickLaunchAction::Reveal,
            "owned".into(),
            QuickLaunchActionTrigger::Enter,
            cx,
        )
    });
    assert!(matches!(
        endpoint
            .try_receive_command_for_contract_test()
            .expect("unchanged Reveal")
            .message,
        nxtools_protocol::Message::PlatformCommand(PlatformCommand::OpenShellTarget(
            nxtools_protocol::OpenShellTargetCommand {
                action: nxtools_protocol::ShellOpenAction::SelectInExplorer,
                ..
            }
        ))
    ));
}

#[gpui_kit::test]
fn row_spacing_external_gap_is_four_or_zero_and_never_selects_or_executes(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (_temp, application, endpoint) = quick_launch_application_fixture(cx);
    let app = application.downgrade();
    let slot = Rc::new(RefCell::new(None));
    let held = slot.clone();
    let (_, surface) = cx.add_window_view(|window, cx| {
        let panel = cx.new(|cx| {
            let mut panel = QuickLaunchPanel::new(
                vec![
                    search_result("first", ApplicationSource::IndexedFile),
                    search_result("second", ApplicationSource::IndexedFile),
                ],
                nxtools_protocol::ForegroundContext {
                    window_handle: 1,
                    focus_window_handle: 1,
                    process_id: 1,
                    process_name: "fixture".into(),
                    window_title: String::new(),
                },
                None,
                Vec::new(),
                app,
                window,
                cx,
            );
            panel.results_visible = true;
            panel
        });
        *held.borrow_mut() = Some(panel.clone());
        Root::new(panel, window, cx)
    });
    let panel = slot.borrow().clone().expect("panel");
    surface.simulate_resize(size(px(PANEL_WIDTH), px(PANEL_HEIGHT)));
    for style in nxtools_core::QuickLaunchRowStyle::ALL {
        panel.update(surface, |panel, cx| {
            panel.row_style = style;
            panel.selected = 1;
            cx.notify();
        });
        surface.refresh().expect("production row style");
        let first = surface
            .debug_bounds("quick-launch-result-row-0")
            .expect("first");
        let second = surface
            .debug_bounds("quick-launch-result-row-1")
            .expect("second");
        assert_eq!(first.size.height, px(LAUNCHER_RESULT_ROW_HEIGHT));
        assert_eq!(second.top() - first.bottom(), px(style.gap()));
        if style == nxtools_core::QuickLaunchRowStyle::Separated {
            let gap = point(first.center().x, first.bottom() + px(2.));
            surface.simulate_click(gap, Default::default());
            surface.run_until_parked();
            assert_eq!(panel.read_with(surface, |panel, _| panel.selected), 1);
            application.read_with(surface, |app, _| {
                assert!(app.pending_actions.is_empty());
                assert!(app.preview.hovered_entry_id.is_none());
            });
            assert!(
                (0..16)
                    .filter_map(|_| endpoint.try_receive_command_for_contract_test().ok())
                    .all(|envelope| !matches!(
                        envelope.message,
                        nxtools_protocol::Message::PlatformCommand(
                            PlatformCommand::OpenShellTarget(_)
                        )
                    ))
            );
        }
    }
}

#[gpui_kit::test]
fn row_spacing_setting_is_durable_and_failure_preserves_live_style_without_native_configuration(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_kit::init);
    let (directory, application, endpoint) = quick_launch_application_fixture(cx);
    let path = directory.path().join("config.json");
    AppConfig::update_section(&path, |config| {
        config.clipboard.image_view_mode = nxtools_core::ClipboardImageViewMode::Mini;
        config.download.default_connection_limit = 24;
    })
    .expect("other owner fields");
    for style in nxtools_core::QuickLaunchRowStyle::ALL {
        application.update(cx, |owner, cx| owner.set_row_style(style, cx));
        let config = AppConfig::load_from(&path).expect("durable style");
        assert_eq!(config.quick_launch.row_style, style);
        assert_eq!(
            config.clipboard.image_view_mode,
            nxtools_core::ClipboardImageViewMode::Mini
        );
        assert_eq!(config.download.default_connection_limit, 24);
        assert!(
            endpoint.try_receive_command_for_contract_test().is_err(),
            "no hotkey/index/native command for presentation setting"
        );
    }
    std::fs::write(&path, b"{broken").expect("commit failure");
    application.update(cx, |owner, cx| {
        owner.set_row_style(nxtools_core::QuickLaunchRowStyle::Continuous, cx)
    });
    assert_eq!(
        application.read_with(cx, |owner, _| owner.settings().row_style),
        nxtools_core::QuickLaunchRowStyle::Separated
    );
    assert_eq!(std::fs::read(&path).expect("no overwrite"), b"{broken");
}

#[test]
fn pending_results_wait_for_complete_even_when_the_partial_set_is_nonempty() {
    let mut committed = vec!["old"];
    assert!(!apply_search_update(
        &mut committed,
        vec!["current-local"],
        true
    ));
    assert_eq!(committed, vec!["old"]);
    assert!(!apply_search_update(&mut committed, Vec::new(), true));
    assert_eq!(committed, vec!["old"]);
    assert!(apply_search_update(&mut committed, Vec::new(), false));
    assert!(committed.is_empty());
}

#[gpui_kit::test]
fn normal_search_local_command_waits_for_index_completion_before_pointer_execution(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_kit::init);
    let (_temp, app, platform) = quick_launch_application_fixture(cx);
    let (bridge, endpoint) = crate::runtime::indexer_bridge(8);
    endpoint.connect_for_test();
    app.update(cx, |app, _| {
        app.indexer = Rc::new(bridge);
        app.index_status = Some(IndexStatus {
            phase: nxtools_protocol::IndexPhase::Ready,
            indexed_items: 1,
            drives: Vec::new(),
            last_error: None,
        });
        app.config.quick_launch.custom_commands = vec![nxtools_core::CustomCommandSettings {
            keyword: "fixture-fast".into(),
            title: "Current local command".into(),
            program: r"C:\Fixture\tool.exe".into(),
            ..Default::default()
        }];
    });
    let (panel, window) =
        quick_launch_panel_fixture(cx, vec![search_result("old", ApplicationSource::StartMenu)]);
    window.update(|window, cx| {
        app.update(cx, |app, _| {
            app.popup = Some(QuickLaunchWindow {
                window: window.window_handle(),
                panel: panel.downgrade(),
                native_window_handle: 1,
            });
        });
    });
    panel.update_in(window, |panel, window, cx| {
        panel.application = app.downgrade();
        panel.focus_search(window, cx);
    });
    window.refresh().expect("focused command search");
    panel.update_in(window, |panel, window, cx| {
        panel.search_input.update(cx, |input, cx| {
            input.replace_all("fixture-fast", window, cx)
        })
    });
    window.run_until_parked();
    window.run_until_parked();
    panel.read_with(window, |panel, _| {
        assert!(
            panel.search_pending,
            "retained rows stay inert until the current providers complete"
        );
        assert_eq!(panel.results[0].entry.id, "old");
    });
    window.executor().advance_clock(Duration::from_millis(150));
    window.run_until_parked();
    panel.read_with(window, |panel, _| {
        assert!(panel.search_pending);
        assert_eq!(panel.results[0].entry.id, "old");
    });
    let envelope = endpoint
        .try_recv_command_for_test()
        .expect("actual indexed request");
    let nxtools_protocol::Message::IndexerCommand(IndexerCommand::Search(request)) =
        envelope.message
    else {
        panic!("expected indexed search");
    };
    app.update(window, |app, cx| {
        app.handle_indexer_notification(
            IndexerNotification::Event(IndexerEvent::SearchBatch(IndexSearchBatch {
                generation: request.generation,
                items: Vec::new(),
                complete: true,
                elapsed_us: 20,
            })),
            cx,
        );
    });
    app.read_with(window, |app, _| assert!(app.index_search.complete));
    panel.read_with(window, |panel, _| {
        assert!(!panel.search_pending);
        assert!(
            panel
                .results
                .iter()
                .any(|row| row.entry.source == ApplicationSource::CustomCommand)
        );
        assert!(!panel.results.iter().any(|row| row.entry.id == "old"));
    });
    window.refresh().expect("paint current local command");
    let row = window
        .debug_bounds("quick-launch-result-row-0")
        .expect("current command row");
    window.simulate_click(row.center(), gpui_kit::Modifiers::default());
    window.run_until_parked();
    let command = (0..8)
        .filter_map(|_| platform.try_receive_command_for_contract_test().ok())
        .find(|envelope| {
            matches!(&envelope.message,
            nxtools_protocol::Message::PlatformCommand(PlatformCommand::LaunchProcess(command))
                if command.executable == r"C:\Fixture\tool.exe")
        })
        .expect("complete current local command dispatches through the original pointer action");
    app.read_with(window, |app, _| {
        assert_eq!(
            app.pending_actions[&command.id].trigger,
            QuickLaunchActionTrigger::Pointer
        );
        assert!(
            app.index_search.query.is_empty(),
            "successful execution retires its search"
        );
    });
}

#[gpui_kit::test]
fn search_latency_complete_indexed_rows_commit_atomically_with_local_results(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_kit::init);
    let (temporary, application, platform) = quick_launch_application_fixture(cx);
    let applications = temporary.path().join("applications");
    std::fs::create_dir_all(&applications).expect("synthetic application catalog root");
    std::fs::write(applications.join("latency-fixture-app.lnk"), b"")
        .expect("synthetic application shortcut");
    let (indexer, endpoint) = crate::runtime::indexer_bridge(8);
    endpoint.connect_for_test();
    let old_generation = application.update(cx, |application, _| {
        application.indexer = Rc::new(indexer);
        application.index_status = Some(IndexStatus {
            phase: nxtools_protocol::IndexPhase::Ready,
            indexed_items: 2,
            drives: Vec::new(),
            last_error: None,
        });
        application
            .catalog
            .borrow_mut()
            .refresh(&[nxtools_finder::ApplicationRoot {
                path: applications,
                source: ApplicationSource::StartMenu,
            }]);
        application
            .index_search
            .begin("old", INDEX_CANDIDATE_LIMIT)
            .generation
    });
    let (panel, window) =
        quick_launch_panel_fixture(cx, vec![search_result("old", ApplicationSource::StartMenu)]);
    window.update(|window, cx| {
        application.update(cx, |application, _| {
            application.popup = Some(QuickLaunchWindow {
                window: window.window_handle(),
                panel: panel.downgrade(),
                native_window_handle: 1,
            });
        });
    });
    let input = panel.read_with(window, |panel, _| panel.search_input.clone());
    panel.update_in(window, |panel, window, cx| {
        panel.application = application.downgrade();
        panel.focus_search(window, cx);
    });
    window.refresh().expect("focused production search input");
    input.update_in(window, |input, window, cx| {
        input.replace_all("latency-fixture", window, cx)
    });
    window.run_until_parked();
    window.run_until_parked();
    let envelope = endpoint
        .try_recv_command_for_test()
        .expect("typed input queues the actual indexed search");
    let nxtools_protocol::Message::IndexerCommand(IndexerCommand::Search(request)) =
        envelope.message
    else {
        panic!("expected indexed search request");
    };
    assert_eq!(request.query, "latency-fixture");
    assert!(request.generation > old_generation);
    panel.read_with(window, |panel, _| {
        assert!(
            panel.search_pending,
            "current local row waits for the first merge"
        );
        assert_eq!(panel.results.len(), 1);
        assert_eq!(panel.results[0].entry.id, "old");
    });
    window.executor().advance_clock(Duration::from_millis(150));
    window.run_until_parked();
    panel.read_with(window, |panel, _| {
        assert!(
            panel.search_pending,
            "local rows wait for all current providers"
        );
        assert_eq!(panel.results[0].entry.id, "old");
    });
    application.read_with(window, |application, _| {
        assert!(!application.index_search.complete);
    });
    let item = |name: &str| IndexSearchItem {
        name: name.into(),
        path: format!(r"C:\synthetic\{name}"),
        is_directory: false,
        score: 0,
        size_bytes: None,
        modified_at_unix_ms: None,
    };
    application.update(window, |application, cx| {
        application.handle_indexer_notification(
            IndexerNotification::Event(IndexerEvent::SearchBatch(IndexSearchBatch {
                generation: old_generation,
                items: vec![item("latency-fixture-stale.txt")],
                complete: true,
                elapsed_us: 10,
            })),
            cx,
        );
    });
    panel.read_with(window, |panel, _| {
        assert!(panel.search_pending);
        assert_eq!(
            panel.results.len(),
            1,
            "stale reply cannot replace the retained collection"
        );
    });

    // Hold the terminal reply while a current-query batch is already available.
    application.update(window, |application, cx| {
        application.handle_indexer_notification(
            IndexerNotification::Event(IndexerEvent::SearchBatch(IndexSearchBatch {
                generation: request.generation,
                items: vec![item("latency-fixture-ready.txt")],
                complete: false,
                elapsed_us: 20,
            })),
            cx,
        );
    });
    panel.read_with(window, |panel, _| {
        assert_eq!(
            panel.results.len(),
            1,
            "intermediate batch does not replace the list"
        );
    });
    let accepted_rows = application.read_with(window, |application, _| {
        assert!(!application.index_search.complete);
        application.index_search.items.len()
    });
    assert_eq!(accepted_rows, 1);
    panel.read_with(window, |panel, _| {
        assert!(
            panel.search_pending,
            "partial results cannot enable the retained collection"
        );
        assert!(
            panel
                .results
                .iter()
                .all(|row| row.entry.source != ApplicationSource::IndexedFile)
        );
    });
    application.update(window, |application, cx| {
        application.handle_indexer_notification(
            IndexerNotification::Event(IndexerEvent::SearchBatch(IndexSearchBatch {
                generation: request.generation,
                items: vec![item("latency-fixture-final.txt")],
                complete: true,
                elapsed_us: 30,
            })),
            cx,
        );
    });
    window.run_until_parked();
    application.read_with(window, |application, _| {
        assert!(application.index_search.complete);
    });
    panel.read_with(window, |panel, cx| {
        assert!(!panel.search_pending);
        assert_eq!(panel.search_input.entity_id(), input.entity_id());
        assert_eq!(
            panel.search_input.read(cx).value().as_ref(),
            "latency-fixture"
        );
        for name in [
            "latency-fixture-app",
            "latency-fixture-ready.txt",
            "latency-fixture-final.txt",
        ] {
            assert!(
                panel.results.iter().any(|row| row.entry.name == name),
                "{name}"
            );
        }
        assert!(
            !panel
                .results
                .iter()
                .any(|row| row.entry.name.contains("stale"))
        );
    });
    let accepted_actions =
        application.read_with(window, |application, _| application.pending_actions.len());
    input.update_in(window, |input, window, cx| {
        input.replace_all("latency-fixture-new-query", window, cx)
    });
    panel.update_in(window, |panel, window, cx| {
        assert!(
            panel.search_pending,
            "new input retires current result availability"
        );
        assert!(
            !panel.results.is_empty(),
            "old rows remain visible while waiting"
        );
        panel.activate_selected(
            QuickLaunchAction::Open,
            QuickLaunchActionTrigger::Pointer,
            cx,
        );
        panel.enter_action_mode(panel.selected, window, cx);
        assert!(
            panel.action_mode.is_none(),
            "retained rows cannot enter actions"
        );
    });
    window.run_until_parked();
    application.read_with(window, |application, _| {
        assert_eq!(application.pending_actions.len(), accepted_actions);
        assert_ne!(application.index_search.generation, request.generation);
    });
    assert!(
        (0..8)
            .filter_map(|_| platform.try_receive_command_for_contract_test().ok())
            .all(|envelope| !matches!(
                envelope.message,
                nxtools_protocol::Message::PlatformCommand(PlatformCommand::OpenShellTarget(_))
            )),
        "retained rows cannot emit another native open"
    );
    window.run_until_parked();
    let newer_envelope = endpoint
        .try_recv_command_for_test()
        .expect("new query still requests the indexed provider");
    let nxtools_protocol::Message::IndexerCommand(IndexerCommand::Search(newer)) =
        newer_envelope.message
    else {
        panic!("expected new indexed search request");
    };
    let retained = panel.read_with(window, |panel, _| {
        assert!(
            panel.search_pending,
            "empty local query cannot enable retained rows"
        );
        panel.results.to_vec()
    });
    application.update(window, |application, cx| {
        application.handle_indexer_notification(
            IndexerNotification::Event(IndexerEvent::SearchBatch(IndexSearchBatch {
                generation: newer.generation,
                items: Vec::new(),
                complete: false,
                elapsed_us: 40,
            })),
            cx,
        );
    });
    panel.read_with(window, |panel, _| {
        assert!(panel.search_pending);
        assert_eq!(panel.results.to_vec(), retained);
    });
    application.read_with(window, |application, _| {
        assert!(!application.index_search.complete);
    });
    application.update(window, |application, cx| {
        application.handle_indexer_notification(
            IndexerNotification::Event(IndexerEvent::SearchBatch(IndexSearchBatch {
                generation: newer.generation,
                items: Vec::new(),
                complete: true,
                elapsed_us: 50,
            })),
            cx,
        );
    });
    panel.read_with(window, |panel, _| {
        assert!(!panel.search_pending);
        assert!(panel.results.is_empty());
    });
    application.read_with(window, |application, _| {
        assert!(application.index_search.complete);
    });
    panel.update_in(window, |panel, window, cx| panel.focus_search(window, cx));
    window.simulate_keystrokes("ctrl-a");
    input.update_in(window, |input, window, cx| {
        input.replace_all("latency-fixture", window, cx)
    });
    window.run_until_parked();
    window.run_until_parked();
    let action_envelope = endpoint
        .try_recv_command_for_test()
        .expect("fresh action query");
    let nxtools_protocol::Message::IndexerCommand(IndexerCommand::Search(action_request)) =
        action_envelope.message
    else {
        panic!("expected fresh indexed search request");
    };
    application.update(window, |application, cx| {
        application.handle_indexer_notification(
            IndexerNotification::Event(IndexerEvent::SearchBatch(IndexSearchBatch {
                generation: action_request.generation,
                items: vec![item("latency-fixture-ready.txt")],
                complete: true,
                elapsed_us: 20,
            })),
            cx,
        );
    });
    application.read_with(window, |app, _| assert!(app.index_search.complete));
    window.run_until_parked();
    panel.update_in(window, |panel, window, cx| {
        let row = panel
            .results
            .iter()
            .position(|row| row.entry.target == r"C:\synthetic\latency-fixture-ready.txt")
            .expect("fresh completed result");
        assert!(!panel.search_pending);
        panel.select_result(row, cx);
        panel.focus_search(window, cx);
    });
    window
        .refresh()
        .expect("focus current indexed result input");
    window.simulate_event(gpui_kit::KeyDownEvent {
        keystroke: gpui_kit::Keystroke::parse("enter").expect("ready result Enter"),
        is_held: false,
        prefer_character_input: false,
    });
    window.simulate_event(gpui_kit::KeyUpEvent {
        keystroke: gpui_kit::Keystroke::parse("enter").expect("ready result Enter release"),
    });
    window.run_until_parked();
    let ready_command = (0..8)
        .filter_map(|_| platform.try_receive_command_for_contract_test().ok())
        .find(|envelope| {
            matches!(&envelope.message,
            nxtools_protocol::Message::PlatformCommand(PlatformCommand::OpenShellTarget(open))
                if open.target == r"C:\synthetic\latency-fixture-ready.txt")
        })
        .expect("completed indexed row dispatches through the original Enter action");
    application.read_with(window, |application, _| {
        assert_eq!(
            application.pending_actions[&ready_command.id].trigger,
            QuickLaunchActionTrigger::Enter
        );
        assert!(
            application.index_search.query.is_empty(),
            "successful execution retires its search"
        );
    });
}

#[gpui_kit::test]
fn result_ux_refresh_preserves_identity_scroll_and_original_action_entry(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let mut first = search_result("same.txt", ApplicationSource::IndexedFile);
    first.entry.name = format!("same.txt {}", "很长的文件名".repeat(24));
    let mut chosen = search_result("same-folder.txt", ApplicationSource::IndexedFile);
    chosen.entry.name = first.entry.name.clone();
    chosen.entry.target = format!(r"C:\another parent\{}\same.txt", "nested\\".repeat(30));
    let chosen_target = chosen.entry.target.clone();
    let (panel, window) = quick_launch_panel_fixture(cx, vec![first.clone(), chosen.clone()]);
    panel.update_in(window, |panel, _, cx| {
        panel.select_result(1, cx);
        panel
            .result_scroll
            .0
            .borrow()
            .base_handle
            .set_offset(point(px(0.0), px(-20.0)));
    });
    window
        .refresh()
        .expect("render duplicate names from different parents");
    let row = window
        .debug_bounds("quick-launch-result-row-1")
        .expect("selected row");
    assert!(
        window
            .debug_bounds("quick-launch-result-actions-1")
            .is_none(),
        "result rows must not add a repeated action button"
    );
    assert!(
        window.update(|window, _| {
            !gpui_kit::base::test_support::snapshots(window)
                .iter()
                .any(|node| node.label() == Some("搜索此结果的操作"))
        }),
        "removed action must not remain in keyboard/accessibility controls"
    );
    panel.read_with(window, |panel, _| {
        assert_eq!(
            panel.results.display(1).subtitle.as_ref(),
            Path::new(&chosen_target)
                .parent()
                .expect("parent directory")
                .to_string_lossy()
        );
    });
    let hint = window
        .debug_bounds("quick-launch-result-shortcut-1")
        .expect("original shortcut rail");
    assert!(hint.left() >= row.left() && hint.right() <= row.right());
    let accessible = format!("{}，{}", chosen.entry.name, chosen_target);
    assert!(
        window.update(|window, _| gpui_kit::base::test_support::snapshots(window)
            .iter()
            .any(|node| node.label() == Some(accessible.as_str()))),
        "complete duplicate name and parent path must survive visual truncation"
    );
    let before = panel.read_with(window, |panel, _| {
        panel.result_scroll.0.borrow().base_handle.offset()
    });
    panel.update_in(window, |panel, _, cx| {
        panel.replace_results(
            vec![
                first.clone(),
                chosen.clone(),
                search_result("new", ApplicationSource::System),
            ],
            cx,
        );
        assert_eq!(panel.results[panel.selected].entry.target, chosen_target);
        assert_eq!(
            panel.result_scroll.0.borrow().base_handle.offset(),
            before,
            "same-query append must not center an already visible selected row"
        );
    });
    panel.update_in(window, |panel, _, cx| {
        panel.replace_results(
            vec![
                search_result("new", ApplicationSource::System),
                first,
                chosen,
            ],
            cx,
        );
    });
    window
        .refresh()
        .expect("render reordered result identities");
    let row = window
        .debug_bounds("quick-launch-result-row-2")
        .expect("reordered result");
    window.simulate_event(gpui_kit::MouseDownEvent {
        position: row.center(),
        button: gpui_kit::MouseButton::Right,
        ..Default::default()
    });
    panel.read_with(window, |panel, _| {
        assert_eq!(panel.selected, 2);
        assert_eq!(
            panel
                .action_mode
                .as_ref()
                .expect("original right-click enters searchable mode")
                .target()
                .entry
                .target,
            chosen_target
        );
    });
}

#[gpui_kit::test]
fn result_ux_retained_rows_and_pending_have_distinct_state(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (panel, window) = quick_launch_panel_fixture(
        cx,
        vec![search_result("old", ApplicationSource::IndexedFile)],
    );
    panel.update_in(window, |panel, _, _| panel.search_pending = true);
    window
        .refresh()
        .expect("render retained previous-query row");
    assert!(
        window
            .debug_bounds("quick-launch-result-actions-0")
            .is_none()
    );
    panel.update_in(window, |panel, _, cx| {
        panel.results.clear();
        cx.notify();
    });
    window
        .refresh()
        .expect("render pending without pretending no match");
    assert!(
        window
            .debug_bounds("quick-launch-search-feedback")
            .is_none()
    );
    assert!(window.debug_bounds("quick-launch-results-empty").is_none());
    panel.update_in(window, |panel, _, cx| {
        panel.set_error("索引连接不可用".into(), cx)
    });
    window
        .refresh()
        .expect("render unavailable separately from no matches");
    assert!(
        window
            .debug_bounds("quick-launch-results-unavailable")
            .is_some()
    );
    assert!(window.debug_bounds("quick-launch-results-empty").is_none());
}

#[gpui_kit::test]
fn result_ux_failed_query_does_not_reenable_retained_rows(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (panel, window) = quick_launch_panel_fixture(
        cx,
        vec![search_result(
            "previous-query",
            ApplicationSource::IndexedFile,
        )],
    );
    panel.update_in(window, |panel, _, cx| {
        panel.search_pending = true;
        panel.set_error("索引服务不可用".into(), cx);
        assert!(
            panel.search_pending,
            "previous-query rows must stay noninteractive"
        );
    });
    window.refresh().expect("render retained results and error");
    assert!(window.debug_bounds("quick-launch-result-row-0").is_some());
    assert!(
        window
            .debug_bounds("quick-launch-result-actions-0")
            .is_none()
    );
}

#[test]
fn terminal_phonetic_fallback_does_not_replace_published_normal_rows() {
    let normal = search_result("normal", ApplicationSource::StartMenu);
    let fallback = search_result("phonetic", ApplicationSource::IndexedFile);
    let rows = stable_indexed_results(std::slice::from_ref(&normal), vec![fallback.clone()]);
    assert_eq!(rows, vec![normal, fallback]);
}

#[gpui_kit::test]
fn normal_mode_keeps_partial_index_batches_private_until_terminal(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (_temporary, application, _platform) = quick_launch_application_fixture(cx);
    let (bridge, endpoint) = crate::runtime::indexer_bridge(8);
    endpoint.connect_for_test();
    application.update(cx, |app, _| {
        app.indexer = Rc::new(bridge);
        app.index_status = Some(IndexStatus {
            phase: nxtools_protocol::IndexPhase::Ready,
            indexed_items: 3,
            drives: Vec::new(),
            last_error: None,
        });
    });
    let (panel, window) = quick_launch_panel_fixture(
        cx,
        vec![search_result("previous", ApplicationSource::StartMenu)],
    );
    window.update(|window, cx| {
        application.update(cx, |app, _| {
            app.popup = Some(QuickLaunchWindow {
                window: window.window_handle(),
                panel: panel.downgrade(),
                native_window_handle: 1,
            });
        });
    });
    panel.update_in(window, |panel, window, cx| {
        panel.application = application.downgrade();
        panel.focus_search(window, cx);
    });
    window.refresh().expect("focused indexed search");
    panel.update_in(window, |panel, window, cx| {
        panel.search_input.update(cx, |input, cx| {
            input.replace_all("fixture-indexed", window, cx)
        })
    });
    window.run_until_parked();
    window.run_until_parked();
    let envelope = endpoint
        .try_recv_command_for_test()
        .expect("indexed request");
    let nxtools_protocol::Message::IndexerCommand(IndexerCommand::Search(request)) =
        envelope.message
    else {
        panic!("expected indexed search");
    };
    let item = |name: &str| IndexSearchItem {
        name: name.into(),
        path: format!(r"C:\synthetic\{name}"),
        is_directory: false,
        score: 0,
        size_bytes: None,
        modified_at_unix_ms: None,
    };
    let deliver =
        |window: &mut gpui_kit::VisualTestContext, items: Vec<IndexSearchItem>, complete: bool| {
            application.update(window, |app, cx| {
                app.handle_indexer_notification(
                    IndexerNotification::Event(IndexerEvent::SearchBatch(IndexSearchBatch {
                        generation: request.generation,
                        items,
                        complete,
                        elapsed_us: 25,
                    })),
                    cx,
                )
            });
        };
    deliver(window, vec![item("fixture-indexed-first.txt")], false);
    deliver(window, vec![item("fixture-indexed-second.txt")], false);
    panel.read_with(window, |panel, _| {
        assert!(panel.search_pending);
        assert_eq!(panel.results[0].entry.id, "previous");
    });
    window.executor().advance_clock(Duration::from_secs(1));
    window.run_until_parked();
    panel.read_with(window, |panel, _| {
        assert!(panel.search_pending);
        assert_eq!(panel.results[0].entry.id, "previous");
    });
    deliver(window, vec![item("fixture-indexed-third.txt")], false);
    panel.read_with(window, |panel, _| {
        assert!(panel.search_pending);
        assert_eq!(panel.results[0].entry.id, "previous");
    });
    deliver(window, Vec::new(), true);
    panel.read_with(window, |panel, _| {
        assert!(!panel.search_pending);
        assert_eq!(panel.results.len(), 3);
        assert_eq!(panel.selected, 0);
    });
    panel.update_in(window, |panel, window, cx| {
        panel.enter_action_mode(0, window, cx);
        assert!(
            panel.action_mode.is_some(),
            "only the complete current set enters actions"
        );
        panel.exit_action_mode(window, cx);
    });
    window.run_until_parked();
    panel.read_with(window, |panel, _| assert_eq!(panel.results.len(), 3));
}

#[gpui_kit::test]
fn normal_terminal_commits_once_and_rejects_retired_query_notifications(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (_temporary, application, _platform) = quick_launch_application_fixture(cx);
    let (bridge, endpoint) = crate::runtime::indexer_bridge(8);
    endpoint.connect_for_test();
    application.update(cx, |app, _| {
        app.indexer = Rc::new(bridge);
        app.index_status = Some(IndexStatus {
            phase: nxtools_protocol::IndexPhase::Ready,
            indexed_items: 1,
            drives: Vec::new(),
            last_error: None,
        });
    });
    let (panel, window) = quick_launch_panel_fixture(cx, Vec::new());
    window.update(|window, cx| {
        application.update(cx, |app, _| {
            app.popup = Some(QuickLaunchWindow {
                window: window.window_handle(),
                panel: panel.downgrade(),
                native_window_handle: 1,
            });
        })
    });
    panel.update_in(window, |panel, window, cx| {
        panel.application = application.downgrade();
        panel.focus_search(window, cx);
    });
    window.refresh().expect("focused terminal search");
    panel.update_in(window, |panel, window, cx| {
        panel.search_input.update(cx, |input, cx| {
            input.replace_all("fixture-terminal", window, cx)
        })
    });
    window.run_until_parked();
    window.run_until_parked();
    let envelope = endpoint
        .try_recv_command_for_test()
        .expect("indexed request");
    let nxtools_protocol::Message::IndexerCommand(IndexerCommand::Search(request)) =
        envelope.message
    else {
        panic!("expected indexed search");
    };
    application.update(window, |app, cx| {
        app.handle_indexer_notification(
            IndexerNotification::Event(IndexerEvent::SearchBatch(IndexSearchBatch {
                generation: request.generation,
                items: vec![IndexSearchItem {
                    name: "fixture-terminal.txt".into(),
                    path: r"C:\synthetic\fixture-terminal.txt".into(),
                    is_directory: false,
                    score: 0,
                    size_bytes: None,
                    modified_at_unix_ms: None,
                }],
                complete: true,
                elapsed_us: 12,
            })),
            cx,
        )
    });
    let first = panel.read_with(window, |panel, _| {
        assert!(!panel.search_pending);
        panel.results.to_vec()
    });
    window.executor().advance_clock(Duration::from_millis(150));
    window.run_until_parked();
    panel.read_with(window, |panel, _| assert_eq!(panel.results.to_vec(), first));
    panel.update_in(window, |panel, window, cx| {
        panel
            .search_input
            .update(cx, |input, cx| input.set_value("new-input", window, cx));
        panel.schedule_search("new-input".into(), cx);
    });
    application.update(window, |app, cx| {
        app.handle_indexer_notification(
            IndexerNotification::Event(IndexerEvent::SearchBatch(IndexSearchBatch {
                generation: request.generation,
                items: Vec::new(),
                complete: true,
                elapsed_us: 22,
            })),
            cx,
        )
    });
    panel.read_with(window, |panel, _| {
        assert!(panel.search_pending);
        assert_eq!(panel.results.to_vec(), first);
    });
}

#[path = "refresh_tests.rs"]
mod refresh_tests;

enum RetirementGesture {
    Clear,
    Hide,
    Reopen,
    SaturatedHide,
}

#[gpui_kit::test]
fn search_retirement_clear_sends_matching_cancel_frame(cx: &mut TestAppContext) {
    assert_search_retirement_frame(cx, RetirementGesture::Clear);
}

#[gpui_kit::test]
fn search_retirement_hide_sends_matching_cancel_frame(cx: &mut TestAppContext) {
    assert_search_retirement_frame(cx, RetirementGesture::Hide);
}

#[gpui_kit::test]
fn search_retirement_old_deferred_close_does_not_cancel_reopened_query(cx: &mut TestAppContext) {
    assert_search_retirement_frame(cx, RetirementGesture::Reopen);
}

#[gpui_kit::test]
fn search_retirement_full_command_queue_does_not_lose_hide_cancel(cx: &mut TestAppContext) {
    assert_search_retirement_frame(cx, RetirementGesture::SaturatedHide);
}

fn assert_search_retirement_frame(cx: &mut TestAppContext, gesture: RetirementGesture) {
    cx.update(gpui_kit::init);
    let (_temporary, application, _platform) = quick_launch_application_fixture(cx);
    let (indexer, endpoint) = crate::runtime::indexer_bridge(8);
    endpoint.connect_for_test();
    application.update(cx, |application, _| {
        application.indexer = Rc::new(indexer);
        application.index_status = Some(IndexStatus {
            phase: nxtools_protocol::IndexPhase::Ready,
            indexed_items: 1,
            drives: Vec::new(),
            last_error: None,
        });
    });
    let (panel, window) = quick_launch_panel_fixture(cx, Vec::new());
    window.update(|window, cx| {
        application.update(cx, |application, _| {
            application.popup = Some(QuickLaunchWindow {
                window: window.window_handle(),
                panel: panel.downgrade(),
                native_window_handle: 1,
            });
        });
    });
    panel.update_in(window, |panel, window, cx| {
        panel.application = application.downgrade();
        panel.focus_search(window, cx);
    });
    window.refresh().expect("focused production input");
    panel.update_in(window, |panel, window, cx| {
        panel.search_input.update(cx, |input, cx| {
            input.replace_all("retirement-fixture", window, cx)
        })
    });
    window.run_until_parked();
    window.run_until_parked();
    let search = endpoint
        .try_recv_command_for_test()
        .expect("indexed search");
    let nxtools_protocol::Message::IndexerCommand(IndexerCommand::Search(request)) =
        &search.message
    else {
        panic!("expected original search");
    };
    if matches!(gesture, RetirementGesture::Reopen) {
        panel.update_in(window, |panel, _, cx| {
            panel.close(cx);
            panel.is_open = true;
            panel.query_dispatch_generation =
                panel.query_dispatch_generation.wrapping_add(1).max(1);
        });
        application.update(window, |application, cx| {
            application.popup_session_id = uuid::Uuid::new_v4();
            application.search_and_request_icons("replacement-fixture", cx);
        });
        window.run_until_parked();
        panel.read_with(window, |panel, _| {
            assert!(
                panel.is_open,
                "old deferred close cannot hide the reopened panel"
            )
        });
        let commands =
            std::iter::from_fn(|| endpoint.try_recv_command_for_test()).collect::<Vec<_>>();
        let newer = commands
            .iter()
            .find(|envelope| {
                matches!(
                    envelope.message,
                    nxtools_protocol::Message::IndexerCommand(IndexerCommand::Search(_))
                )
            })
            .expect("new search");
        application.read_with(window, |application, _| {
            assert_eq!(
                application
                    .pending_index_search
                    .as_ref()
                    .map(|pending| pending.request_id),
                Some(newer.id)
            )
        });
        assert_eq!(
            commands.len(),
            1,
            "new Search alone replaces the old query; no cancellation of the new session"
        );
        return;
    }
    if matches!(gesture, RetirementGesture::SaturatedHide) {
        application.update(window, |application, _| {
            for _ in 0..8 {
                application
                    .indexer
                    .send_command(IndexerCommand::RefreshVolumes)
                    .expect("fill existing command queue");
            }
        });
    }
    if matches!(
        gesture,
        RetirementGesture::Hide | RetirementGesture::SaturatedHide
    ) {
        application.update(window, |application, cx| application.hide_popup(cx));
    } else {
        window.simulate_keystrokes("ctrl-a backspace");
    }
    window.run_until_parked();
    if matches!(gesture, RetirementGesture::SaturatedHide) {
        for _ in 0..8 {
            assert!(matches!(
                endpoint
                    .try_recv_command_for_test()
                    .expect("queued filler")
                    .message,
                nxtools_protocol::Message::IndexerCommand(IndexerCommand::RefreshVolumes)
            ));
        }
        window.run_until_parked();
    }
    let cancel = endpoint
        .try_recv_command_for_test()
        .expect("clear/hide must send server cancellation, not only discard UI results");
    let wire = cancel.encode_line().expect("cancel wire frame");
    let decoded = nxtools_protocol::Envelope::decode_line(&wire).expect("cancel frame roundtrip");
    let value = serde_json::to_value(&decoded).expect("cancel JSON shape");
    assert_eq!(value["payload"]["command"], "cancel_search");
    assert_eq!(
        value["payload"]["data"]["request_id"],
        search.id.to_string()
    );
    assert_eq!(value["payload"]["data"]["generation"], request.generation);
    application.read_with(window, |application, _| {
        assert_ne!(application.index_search.generation, request.generation);
        assert!(application.index_search.items.is_empty());
    });
    window.run_until_parked();
    assert!(
        endpoint.try_recv_command_for_test().is_none(),
        "retired input must not dispatch another query"
    );
}

#[test]
fn indexed_terminal_keeps_already_displayed_extension_rows_beyond_normal_limit() {
    let mut previous = (0..RESULT_LIMIT)
        .map(|index| search_result(&format!("normal-{index}"), ApplicationSource::StartMenu))
        .collect::<Vec<_>>();
    previous.push(search_result(
        "extension-script:fixture/provider/item",
        ApplicationSource::CustomCommand,
    ));
    assert_eq!(
        stable_indexed_results(&previous, previous.clone()),
        previous
    );
}

#[gpui_kit::test]
fn indexed_ready_retry_keeps_retained_rows_inert_until_complete_replacement(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_kit::init);
    let (_temporary, application, _platform) = quick_launch_application_fixture(cx);
    let (bridge, endpoint) = crate::runtime::indexer_bridge(8);
    endpoint.connect_for_test();
    let (panel, window) = quick_launch_panel_fixture(
        cx,
        vec![search_result("current-local", ApplicationSource::StartMenu)],
    );
    panel.update_in(window, |panel, window, cx| {
        panel
            .search_input
            .update(cx, |input, cx| input.set_value("stable", window, cx));
        panel.committed_query = "stable".into();
        panel.search_pending = false;
    });
    window.update(|window, cx| {
        application.update(cx, |app, cx| {
            app.indexer = Rc::new(bridge);
            app.index_status = Some(IndexStatus {
                phase: nxtools_protocol::IndexPhase::Ready,
                indexed_items: 1,
                drives: Vec::new(),
                last_error: None,
            });
            app.popup = Some(QuickLaunchWindow {
                window: window.window_handle(),
                panel: panel.downgrade(),
                native_window_handle: 1,
            });
            app.retry_index_search_after_ready(cx);
        })
    });
    panel.read_with(window, |panel, _| assert!(panel.search_pending));
    let envelope = endpoint
        .try_recv_command_for_test()
        .expect("same-query indexed retry");
    let nxtools_protocol::Message::IndexerCommand(IndexerCommand::Search(request)) =
        envelope.message
    else {
        panic!("indexed search")
    };
    application.update(window, |app, cx| {
        app.handle_indexer_notification(
            IndexerNotification::Event(IndexerEvent::SearchBatch(IndexSearchBatch {
                generation: request.generation,
                items: vec![IndexSearchItem {
                    name: "stable.txt".into(),
                    path: r"C:\synthetic\stable.txt".into(),
                    is_directory: false,
                    score: 10_000,
                    size_bytes: None,
                    modified_at_unix_ms: None,
                }],
                complete: true,
                elapsed_us: 10,
            })),
            cx,
        )
    });
    panel.read_with(window, |panel, _| {
        assert_eq!(panel.results[0].entry.name, "stable.txt");
        assert_eq!(panel.selected, 0);
        assert_eq!(panel.results.len(), 1);
    });
}

#[path = "normal_search_tests.rs"]
mod normal_search_tests;
