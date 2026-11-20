use super::*;
use gpui_kit::{TestAppContext, VisualTestContext};
use nxtools_protocol::process_window_pick::{
    ProcessWindowPickChanged, ProcessWindowPickCommand, WindowPickCandidate, WindowPickRect,
    WindowPickState,
};
use nxtools_protocol::{Message, PlatformEvent};

fn fixture(
    cx: &mut TestAppContext,
) -> (
    Entity<NetworkSettingsApplication>,
    Entity<NetworkSettingsView>,
    VisualTestContext,
    PathBuf,
    nxtools_ipc::PlatformServerEndpoint,
) {
    cx.update(gpui_kit::init);
    cx.update(crate::theme::apply_component_theme);
    let (application, directory, _) = tests::application_fixture(cx, "window-picker");
    let (bridge, endpoint) = nxtools_ipc::platform_bridge(32);
    endpoint.set_connected_for_contract_test(true);
    application.update(cx, |app, cx| {
        app.platform = Some(Rc::new(bridge));
        app.open_window(cx);
    });
    cx.run_until_parked();
    let (handle, view) = cx.read(|cx| {
        let app = application.read(cx);
        (
            app.window.expect("settings window"),
            app.settings_view.clone().expect("original view"),
        )
    });
    let mut surface = VisualTestContext::from_window(handle, cx);
    surface.simulate_resize(size(px(720.0), px(480.0)));
    surface.update(|window, _| window.activate_window());
    surface.refresh().expect("settings frame");
    kit_migration_tests::select_page(&mut surface, 2);
    (application, view, surface, directory, endpoint)
}

fn begin(
    application: &Entity<NetworkSettingsApplication>,
    view: &Entity<NetworkSettingsView>,
    surface: &mut VisualTestContext,
) -> uuid::Uuid {
    let id = uuid::Uuid::new_v4();
    view.update(surface, |view, _| view.process_window_pick = Some(id));
    application.update(surface, |app, cx| {
        let mut overlay =
            window_picker_overlay::WindowPickerOverlay::open(cx).expect("synthetic overlay");
        // TestWindow has no HWND. Only this fixture supplies native facts.
        overlay.native_window_handle = 102;
        app.begin_process_window_pick(
            id,
            view.downgrade(),
            app.window.expect("source"),
            101,
            overlay,
            cx,
        );
    });
    id
}

fn candidate() -> WindowPickCandidate {
    WindowPickCandidate {
        window_handle: 201,
        process_id: 42,
        process_name: "Editor.exe".into(),
        left: -1800,
        top: -80,
        width: 1000,
        height: 800,
    }
}

fn event(
    application: &Entity<NetworkSettingsApplication>,
    surface: &mut VisualTestContext,
    id: uuid::Uuid,
    state: WindowPickState,
) {
    application.update(surface, |app, cx| {
        app.handle_platform_notification(
            PlatformNotification::Event(PlatformEvent::ProcessWindowPickChanged(
                ProcessWindowPickChanged {
                    session_id: id,
                    state,
                    viewport: Some(WindowPickRect {
                        left: -1920,
                        top: -120,
                        width: 3840,
                        height: 1200,
                    }),
                    candidate: Some(candidate()),
                    native_error_code: None,
                },
            )),
            cx,
        )
    });
}

#[gpui_kit::test]
fn blacklist_window_picker_button_is_in_the_original_editor(cx: &mut TestAppContext) {
    let (application, view, mut surface, directory, _) = fixture(cx);
    let picker = surface
        .debug_bounds("global-blacklist-window-pick")
        .expect("selection action");
    let input = surface
        .debug_bounds("global-blacklist-process-field")
        .expect("manual input");
    let add = surface
        .debug_bounds("global-blacklist-add-action")
        .expect("original add action");
    assert!(input.size.width >= px(100.0), "manual input remains usable");
    assert!(input.right() <= picker.left());
    assert!(picker.right() <= add.left());
    surface.simulate_click(picker.center(), Default::default());
    surface.run_until_parked();
    assert!(
        view.read_with(&surface, |view, _| view.process_window_pick.is_none()
            && view.error.is_some()),
        "missing native HWND releases the real button flow"
    );
    kit_migration_tests::focus_control(&mut surface, "global-blacklist-window-pick-button");
    surface.simulate_event(gpui_kit::KeyDownEvent {
        keystroke: gpui_kit::Keystroke::parse("space").expect("Space"),
        is_held: false,
        prefer_character_input: false,
    });
    surface.simulate_event(gpui_kit::KeyUpEvent {
        keystroke: gpui_kit::Keystroke::parse("space").expect("release"),
    });
    surface.run_until_parked();
    assert!(view.read_with(&surface, |view, _| view.process_window_pick.is_none()));
    application.update(&mut surface, |app, cx| app.close_window(cx));
    std::fs::remove_dir_all(directory).expect("cleanup");
}

#[gpui_kit::test]
fn blacklist_window_picker_selected_only_refills_original_draft(cx: &mut TestAppContext) {
    let (application, view, mut surface, directory, endpoint) = fixture(cx);
    let before = std::fs::read(directory.join("config.json")).expect("config");
    let rules = view.read_with(&surface, |view, _| view.rules.clone());
    view.update(&mut surface, |view, _| {
        view.new_disable_clipboard = false;
        view.new_disable_quick_launch = true;
    });
    let id = begin(&application, &view, &mut surface);
    let command = endpoint
        .try_receive_command_for_contract_test()
        .expect("start command");
    assert_eq!(command.id, id);
    assert_eq!(command.correlation_id, Some(id));
    assert!(
        matches!(command.message, Message::PlatformCommand(PlatformCommand::ProcessWindowPick(ProcessWindowPickCommand::Start { session_id, source_window_handle: 101, overlay_window_handle: 102 })) if session_id == id)
    );
    assert!(
        application.read_with(&surface, |app, _| app.pending_process_window_pick.is_some()),
        "pending installed before dispatch"
    );
    event(
        &application,
        &mut surface,
        uuid::Uuid::new_v4(),
        WindowPickState::Selected,
    );
    assert!(view.read_with(&surface, |view, cx| {
        view.process_input.read(cx).value().is_empty()
    }));
    application.update(&mut surface, |app, cx| {
        app.handle_platform_notification(
            PlatformNotification::Event(PlatformEvent::ProcessWindowPickChanged(
                ProcessWindowPickChanged {
                    session_id: id,
                    state: WindowPickState::Selected,
                    viewport: None,
                    candidate: Some(candidate()),
                    native_error_code: None,
                },
            )),
            cx,
        );
        assert!(
            view.read(cx).process_input.read(cx).value().is_empty(),
            "cross-owner write is deferred beyond the application callback"
        );
    });
    surface.run_until_parked();
    assert_eq!(
        view.read_with(&surface, |view, cx| view
            .process_input
            .read(cx)
            .value()
            .to_string()),
        "editor.exe"
    );
    assert!(view.read_with(&surface, |view, _| view.rules == rules
        && !view.new_disable_clipboard
        && view.new_disable_quick_launch
        && !view.saving
        && !view.draft_busy.get()
        && view.process_window_pick.is_none()));
    assert_eq!(
        std::fs::read(directory.join("config.json")).expect("config"),
        before
    );
    event(&application, &mut surface, id, WindowPickState::Selected);
    surface.run_until_parked();
    surface.update(|window, cx| view.update(cx, |view, cx| view.add_rule(window, cx)));
    assert!(view.read_with(&surface, |view, _| view.rules.iter().any(
        |rule| rule.process == "editor" && !rule.disable_clipboard && rule.disable_quick_launch
    )));
    assert_eq!(
        std::fs::read(directory.join("config.json")).expect("still local draft"),
        before
    );
    application.update(&mut surface, |app, cx| app.close_window(cx));
    std::fs::remove_dir_all(directory).expect("cleanup");
}

#[gpui_kit::test]
fn blacklist_window_picker_page_cancel_and_close_reject_late_results(cx: &mut TestAppContext) {
    let (application, view, mut surface, directory, endpoint) = fixture(cx);
    surface.update(|window, cx| {
        view.update(cx, |view, cx| {
            view.process_input
                .update(cx, |input, cx| input.set_value("manual.exe", window, cx))
        })
    });
    let id = begin(&application, &view, &mut surface);
    endpoint
        .try_receive_command_for_contract_test()
        .expect("start");
    event(&application, &mut surface, id, WindowPickState::Hover);
    kit_migration_tests::select_page(&mut surface, 0);
    surface.run_until_parked();
    assert!(application.read_with(&surface, |app, _| app.pending_process_window_pick.is_none()));
    event(&application, &mut surface, id, WindowPickState::Selected);
    surface.run_until_parked();
    assert_eq!(
        view.read_with(&surface, |view, cx| view
            .process_input
            .read(cx)
            .value()
            .to_string()),
        "manual.exe"
    );
    kit_migration_tests::select_page(&mut surface, 2);
    let second = begin(&application, &view, &mut surface);
    surface.update(|window, cx| {
        view.update(cx, |view, cx| view.toggle_process_window_pick(window, cx))
    });
    event(
        &application,
        &mut surface,
        second,
        WindowPickState::Selected,
    );
    surface.run_until_parked();
    assert_eq!(
        view.read_with(&surface, |view, cx| view
            .process_input
            .read(cx)
            .value()
            .to_string()),
        "manual.exe"
    );
    let third = begin(&application, &view, &mut surface);
    application.update(&mut surface, |app, cx| app.close_window(cx));
    event(&application, &mut surface, third, WindowPickState::Selected);
    surface.run_until_parked();
    assert!(
        application.read_with(&surface, |app, _| app.window.is_none()
            && app.pending_process_window_pick.is_none())
    );
    assert_eq!(
        view.read_with(&surface, |view, cx| view
            .process_input
            .read(cx)
            .value()
            .to_string()),
        "manual.exe"
    );
    std::fs::remove_dir_all(directory).expect("cleanup");
}

#[gpui_kit::test]
fn blacklist_window_picker_connection_and_native_failure_retire_pending(cx: &mut TestAppContext) {
    let (application, view, mut surface, directory, endpoint) = fixture(cx);
    let id = begin(&application, &view, &mut surface);
    let retired_connection = application.read_with(&surface, |app, _| {
        app.pending_process_window_pick
            .as_ref()
            .expect("pending")
            .connection
    });
    endpoint.set_connected_for_contract_test(false);
    endpoint.set_connected_for_contract_test(true);
    event(&application, &mut surface, id, WindowPickState::Selected);
    surface.run_until_parked();
    assert!(application.read_with(&surface, |app, _| app.pending_process_window_pick.is_none()));
    assert!(view.read_with(&surface, |view, cx| {
        view.process_input.read(cx).value().is_empty()
    }));
    let second = begin(&application, &view, &mut surface);
    application.update(&mut surface, |app, cx| {
        app.handle_platform_notification(
            PlatformNotification::Disconnected {
                generation: retired_connection,
                pending_command_ids: vec![id],
            },
            cx,
        )
    });
    surface.run_until_parked();
    assert!(
        application.read_with(&surface, |app, _| app
            .pending_process_window_pick
            .as_ref()
            .is_some_and(|pending| pending.id == second)),
        "old disconnect cannot retire the replacement session"
    );
    application.update(&mut surface, |app, cx| {
        app.handle_platform_notification(
            PlatformNotification::CommandResult {
                correlation_id: second,
                result: nxtools_protocol::PlatformCommandResult {
                    success: false,
                    data: None,
                    error: Some("native failure".into()),
                    native_error_code: Some(5),
                },
            },
            cx,
        )
    });
    surface.run_until_parked();
    assert!(
        view.read_with(&surface, |view, _| view.process_window_pick.is_none()
            && view.error.is_some())
    );
    assert!(application.read_with(&surface, |app, _| app.pending_process_window_pick.is_none()));
    let third = begin(&application, &view, &mut surface);
    let connection = application.read_with(&surface, |app, _| {
        app.pending_process_window_pick
            .as_ref()
            .expect("pending")
            .connection
    });
    endpoint.set_connected_for_contract_test(false);
    application.update(&mut surface, |app, cx| {
        app.handle_platform_notification(
            PlatformNotification::Disconnected {
                generation: connection,
                pending_command_ids: vec![third],
            },
            cx,
        )
    });
    surface.run_until_parked();
    assert!(application.read_with(&surface, |app, _| app.pending_process_window_pick.is_none()));
    assert!(view.read_with(&surface, |view, _| view.process_window_pick.is_none()));
    application.update(&mut surface, |app, cx| app.close_window(cx));
    std::fs::remove_dir_all(directory).expect("cleanup");
}

#[test]
fn blacklist_window_picker_highlight_preserves_negative_device_pixel_origins() {
    let viewport = WindowPickRect {
        left: -1920,
        top: -120,
        width: 3840,
        height: 1200,
    };
    for scale in [1.0, 1.25, 1.5, 2.0] {
        let bounds = window_picker_overlay::highlight_bounds(viewport, &candidate(), scale);
        assert_eq!(
            bounds.origin,
            gpui_kit::point(px(120.0 / scale), px(40.0 / scale))
        );
        assert_eq!(bounds.size, size(px(1000.0 / scale), px(800.0 / scale)));
        let maximized = WindowPickCandidate {
            left: viewport.left - 8,
            top: viewport.top - 8,
            width: viewport.width + 16,
            height: viewport.height + 16,
            ..candidate()
        };
        let clipped = window_picker_overlay::highlight_bounds(viewport, &maximized, scale);
        assert_eq!(clipped.origin, gpui_kit::point(px(0.0), px(0.0)));
        assert_eq!(
            clipped.size,
            size(px(3840.0 / scale), px(1200.0 / scale)),
            "visible outline and caption stay on the virtual desktop"
        );
    }
}

#[gpui_kit::test]
fn blacklist_window_picker_frame_show_and_actual_timeout_are_session_scoped(
    cx: &mut TestAppContext,
) {
    let (application, view, mut surface, directory, endpoint) = fixture(cx);
    let id = begin(&application, &view, &mut surface);
    endpoint
        .try_receive_command_for_contract_test()
        .expect("start");
    event(&application, &mut surface, id, WindowPickState::Hover);
    let overlay = application.read_with(&surface, |app, _| {
        app.pending_process_window_pick
            .as_ref()
            .expect("pending")
            .overlay
            .window
    });
    surface.update(|_, cx| {
        overlay
            .update(cx, |_, window, cx| window.simulate_next_frame(cx))
            .expect("overlay frame");
    });
    surface.run_until_parked();
    let show = endpoint
        .try_receive_command_for_contract_test()
        .expect("show after frame");
    assert!(
        matches!(show.message, Message::PlatformCommand(PlatformCommand::ProcessWindowPick(ProcessWindowPickCommand::ShowOverlay { session_id })) if session_id == id)
    );
    event(&application, &mut surface, id, WindowPickState::Hover);
    surface.run_until_parked();
    assert!(
        endpoint.try_receive_command_for_contract_test().is_err(),
        "one show per session"
    );
    surface
        .executor()
        .advance_clock(std::time::Duration::from_secs(126));
    surface.run_until_parked();
    assert!(application.read_with(&surface, |app, _| app.pending_process_window_pick.is_none()));
    assert!(view.read_with(&surface, |view, _| {
        view.process_window_pick.is_none()
            && view
                .error
                .as_deref()
                .is_some_and(|error| error.contains("超时"))
    }));
    let cancel = endpoint
        .try_receive_command_for_contract_test()
        .expect("native cancellation");
    assert!(
        matches!(cancel.message, Message::PlatformCommand(PlatformCommand::ProcessWindowPick(ProcessWindowPickCommand::Cancel { session_id })) if session_id == id)
    );
    event(&application, &mut surface, id, WindowPickState::Selected);
    surface.run_until_parked();
    assert!(view.read_with(&surface, |view, cx| {
        view.process_input.read(cx).value().is_empty()
    }));
    application.update(&mut surface, |app, cx| app.close_window(cx));
    std::fs::remove_dir_all(directory).expect("cleanup");
}

#[gpui_kit::test]
fn window_feedback_blacklist_editor_keeps_scope_controls_only_on_rule_rows(
    cx: &mut TestAppContext,
) {
    let (application, view, mut surface, directory, _) = fixture(cx);
    let original = std::fs::read(directory.join("config.json")).expect("unchanged saved rules");
    view.update_in(&mut surface, |view, window, cx| {
        view.rules
            .push(ProcessBlacklistRule::new("existing.exe", false, true).expect("existing rule"));
        view.process_input
            .update(cx, |input, cx| input.set_value("new.exe", window, cx));
        cx.notify();
    });
    surface.refresh().expect("editor and existing rule");
    assert!(
        surface
            .debug_bounds("global-blacklist-new-clipboard")
            .is_none(),
        "no duplicate preset Clipboard checkbox beside picker"
    );
    assert!(
        surface.debug_bounds("global-blacklist-new-quick").is_none(),
        "no duplicate preset Quick Launch checkbox beside picker"
    );
    assert!(
        surface
            .debug_bounds("global-blacklist-clipboard-0")
            .is_some()
    );
    assert!(surface.debug_bounds("global-blacklist-quick-0").is_some());
    let add = surface
        .debug_bounds("global-blacklist-add-action")
        .expect("original Add");
    surface.simulate_click(add.center(), Default::default());
    surface.run_until_parked();
    view.read_with(&surface, |view, _| {
        let added = view
            .rules
            .iter()
            .find(|rule| rule.process == "new")
            .expect("original draft Add");
        assert!(added.disable_clipboard);
        assert!(
            !added.disable_quick_launch,
            "preserve the existing new-rule defaults"
        );
    });
    assert_eq!(
        std::fs::read(directory.join("config.json")).expect("saved config"),
        original,
        "Add remains local until Apply"
    );
    application.update(&mut surface, |app, cx| app.close_window(cx));
    std::fs::remove_dir_all(directory).expect("cleanup");
}

#[gpui_kit::test]
fn window_feedback_blacklist_update_preserves_live_row_scope_changes(cx: &mut TestAppContext) {
    let (application, view, mut surface, directory, _) = fixture(cx);
    surface.simulate_resize(size(px(720.), px(680.)));
    let mut expected = view.read_with(&surface, |view, _| {
        (
            view.rules[0].disable_clipboard,
            view.rules[0].disable_quick_launch,
        )
    });
    surface.refresh().expect("existing rule row");
    let edit = kit_migration_tests::control_bounds(&mut surface, "global-blacklist-edit-0");
    surface.simulate_click(edit.center(), Default::default());
    surface.run_until_parked();
    for (id, clipboard) in [
        ("global-blacklist-clipboard-0", true),
        ("global-blacklist-quick-0", false),
    ] {
        surface.refresh().expect("selected rule scope");
        let scope = kit_migration_tests::control_bounds(&mut surface, id);
        surface.simulate_click(scope.center(), Default::default());
        surface.run_until_parked();
        if clipboard {
            expected.0 = !expected.0;
        } else {
            expected.1 = !expected.1;
        }
        view.read_with(&surface, |view, _| {
            assert_eq!(
                (
                    view.rules[0].disable_clipboard,
                    view.rules[0].disable_quick_launch
                ),
                expected,
                "actual row checkbox changes its requested scope"
            )
        });
    }
    if !expected.0 && !expected.1 {
        surface
            .refresh()
            .expect("retain one disabled scope before Update");
        let scope =
            kit_migration_tests::control_bounds(&mut surface, "global-blacklist-clipboard-0");
        surface.simulate_click(scope.center(), Default::default());
        surface.run_until_parked();
        expected.0 = true;
    }
    view.update_in(&mut surface, |view, window, cx| {
        view.process_input
            .update(cx, |input, cx| input.set_value("renamed.exe", window, cx));
    });
    surface.refresh().expect("rename selected rule");
    let update = kit_migration_tests::control_bounds(&mut surface, "global-blacklist-add");
    surface.simulate_click(update.center(), Default::default());
    surface.run_until_parked();
    view.read_with(&surface, |view, _| {
        let renamed = view
            .rules
            .iter()
            .find(|rule| rule.process == "renamed")
            .expect("updated rule");
        assert_eq!(
            (renamed.disable_clipboard, renamed.disable_quick_launch),
            expected,
            "Update must retain both newly selected row scopes"
        );
    });
    application.update(&mut surface, |app, cx| app.close_window(cx));
    std::fs::remove_dir_all(directory).expect("cleanup");
}
