use super::*;
use crate::ui::material::{UiMaterialCapabilities, runtime};
use gpui_kit::TestAppContext;
use nxtools_protocol::window_material::{
    AppliedWindowMaterial, EffectiveWindowMaterial, MaterialFallbackReason,
    SystemMaterialCapabilitiesChanged,
};
use nxtools_protocol::{Envelope, Message, PlatformCommandResult};

fn sample(cx: &mut App, revision: u64) {
    let mut state = cx
        .try_global::<UiMaterialCapabilities>()
        .copied()
        .unwrap_or_default();
    state.connected();
    let session_id = state
        .snapshot()
        .map(|facts| facts.session_id)
        .unwrap_or_else(uuid::Uuid::new_v4);
    assert!(state.apply(SystemMaterialCapabilitiesChanged {
        correlation_id: uuid::Uuid::new_v4(),
        session_id,
        revision,
        supported: true,
        transparency_enabled: true,
        high_contrast: false,
        energy_saver: false,
        remote_session: false,
        native_error_code: 0,
        system_backdrop: true,
        rounded_corners: true,
    }));
    cx.set_global(state);
}
fn facts(cx: &mut TestAppContext, revision: u64) {
    cx.update(|cx| sample(cx, revision));
}
fn host() -> QuickLaunchFileDialogSearchHost {
    QuickLaunchFileDialogSearchHost {
        window_handle: 77,
        current_directory: Some(r"C:\synthetic".into()),
        left: -1920,
        top: 100,
        width: 900,
        height: 640,
        monitor_dpi: 144,
        visible: true,
        changed_at_unix_ms: nxtools_ipc::unix_time_ms(),
        quick_switch_source: None,
    }
}
fn open(cx: &mut TestAppContext, app: &Entity<QuickLaunchApplication>) -> AnyWindowHandle {
    app.update(cx, |app, cx| {
        app.config.quick_launch.file_dialog_search = true;
        app.config.quick_launch.enabled = true;
        let host = host();
        app.file_dialog_search.host = Some(host.clone());
        app.open_file_dialog_search_window(&host, cx);
        let retained = app
            .file_dialog_search
            .window
            .as_mut()
            .expect("hidden search");
        retained.native_window_handle = 9002;
        runtime::set_test_window_handle(retained.window, 9002, cx);
        retained.window
    })
}
fn next(endpoint: &nxtools_ipc::PlatformServerEndpoint) -> Envelope {
    endpoint
        .try_receive_command_for_contract_test()
        .expect("presentation command")
}
fn material_result(request: &Envelope) -> PlatformCommandResult {
    let Message::PlatformCommand(PlatformCommand::ApplyWindowMaterial(command)) = request.message
    else {
        panic!("material must precede attachment and reveal");
    };
    assert_eq!(
        command.window_handle, 9002,
        "only Desktop HWND, never foreign dialog"
    );
    PlatformCommandResult {
        success: true,
        error: None,
        native_error_code: None,
        data: Some(
            serde_json::to_string(&AppliedWindowMaterial {
                session_id: command.session_id,
                facts_revision: command.facts_revision,
                window_handle: command.window_handle,
                generation: command.generation,
                style: command.style,
                surface: command.surface,
                dark: command.dark,
                effective: EffectiveWindowMaterial::Solid,
                fallback_reason: MaterialFallbackReason::None,
                native_error_code: 0,
            })
            .expect("receipt"),
        ),
    }
}
fn complete(
    cx: &mut TestAppContext,
    endpoint: &nxtools_ipc::PlatformServerEndpoint,
    request: &Envelope,
) {
    let result = material_result(request);
    endpoint
        .route_command_result_for_contract_test(request.id, result.clone())
        .expect("material terminal");
    cx.update(|cx| {
        assert!(runtime::handle_notification(
            &nxtools_ipc::PlatformNotification::CommandResult {
                correlation_id: request.id,
                result,
            },
            cx
        ))
    });
}
fn complete_then_facts(
    cx: &mut TestAppContext,
    endpoint: &nxtools_ipc::PlatformServerEndpoint,
    request: &Envelope,
    revision: u64,
) {
    let result = material_result(request);
    endpoint
        .route_command_result_for_contract_test(request.id, result.clone())
        .expect("material terminal");
    cx.update(|cx| {
        assert!(runtime::handle_notification(
            &nxtools_ipc::PlatformNotification::CommandResult {
                correlation_id: request.id,
                result,
            },
            cx
        ));
        sample(cx, revision);
    });
}
fn reveal(endpoint: &nxtools_ipc::PlatformServerEndpoint, owner: i64) {
    let attach = next(endpoint);
    assert!(
        matches!(attach.message, Message::PlatformCommand(PlatformCommand::SetWindowOwner(SetWindowOwnerCommand { window_handle: 9002, owner_window_handle })) if owner_window_handle == owner)
    );
    let placement = next(endpoint);
    assert!(
        matches!(placement.message, Message::PlatformCommand(PlatformCommand::PlaceFileDialogSearchWindow(PlaceFileDialogSearchWindowCommand { window_handle: 9002, owner_window_handle, .. })) if owner_window_handle == owner)
    );
    let show = next(endpoint);
    assert!(matches!(
        show.message,
        Message::PlatformCommand(PlatformCommand::SetWindowVisibility(
            SetWindowVisibilityCommand {
                window_handle: 9002,
                visible: true
            }
        ))
    ));
    assert!(endpoint.try_receive_command_for_contract_test().is_err());
}

#[gpui_kit::test]
fn file_dialog_material_blocks_original_reveal_and_rechecks_changed_facts(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.executor().allow_parking();
    let (_temporary, app, endpoint) = super::super::tests::quick_launch_application_fixture(cx);
    facts(cx, 1);
    let window = open(cx, &app);
    app.update(cx, |app, cx| app.initialize_file_dialog_search_window(cx));
    cx.run_until_parked();
    let mut topmost = false;
    let mut material = None;
    for _ in 0..2 {
        let request = next(&endpoint);
        match request.message {
            Message::PlatformCommand(PlatformCommand::SetWindowTopmost(_)) => topmost = true,
            Message::PlatformCommand(PlatformCommand::ApplyWindowMaterial(_)) => {
                material = Some(request)
            }
            _ => panic!(
                "only hidden topmost and material preparation may precede reveal: {:?}",
                request.message
            ),
        }
    }
    assert!(topmost);
    let material = material.expect("qualified material request");
    assert!(endpoint.try_receive_command_for_contract_test().is_err());
    complete_then_facts(cx, &endpoint, &material, 2);
    cx.run_until_parked();
    let latest = next(&endpoint);
    complete(cx, &endpoint, &latest);
    cx.run_until_parked();
    reveal(&endpoint, 77);
    assert!(cx.update(|cx| runtime::receipt(window, cx).is_some()));
    facts(cx, 3);
    app.update(cx, |app, cx| {
        app.file_dialog_search.host.as_mut().expect("host").left += 100;
        app.apply_file_dialog_search_window_placement(cx);
    });
    cx.run_until_parked();
    let update = next(&endpoint);
    assert!(endpoint.try_receive_command_for_contract_test().is_err());
    complete(cx, &endpoint, &update);
    cx.run_until_parked();
    reveal(&endpoint, 77);
    assert_eq!(
        app.read_with(cx, |app, _| app
            .file_dialog_search
            .window
            .as_ref()
            .expect("retained")
            .window),
        window
    );
}

#[gpui_kit::test]
fn file_dialog_material_hide_and_host_replacement_reject_old_completion(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.executor().allow_parking();
    let (_temporary, app, endpoint) = super::super::tests::quick_launch_application_fixture(cx);
    facts(cx, 1);
    open(cx, &app);
    app.update(cx, |app, cx| {
        app.apply_file_dialog_search_window_placement(cx)
    });
    cx.run_until_parked();
    let material = next(&endpoint);
    app.update(cx, |app, cx| {
        app.file_dialog_search.host.as_mut().expect("host").visible = false;
        app.hide_file_dialog_search_window(cx);
    });
    let hide = next(&endpoint);
    assert!(matches!(
        hide.message,
        Message::PlatformCommand(PlatformCommand::SetWindowVisibility(
            SetWindowVisibilityCommand { visible: false, .. }
        ))
    ));
    complete(cx, &endpoint, &material);
    cx.run_until_parked();
    assert!(endpoint.try_receive_command_for_contract_test().is_err());
    app.update(cx, |app, cx| {
        let host = app.file_dialog_search.host.as_mut().expect("host");
        host.visible = true;
        host.window_handle = 88;
        app.apply_file_dialog_search_window_placement(cx);
    });
    cx.run_until_parked();
    reveal(&endpoint, 88);
}

#[gpui_kit::test]
fn file_dialog_material_pending_host_replacement_reuses_one_preparation(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.executor().allow_parking();
    let (_temporary, app, endpoint) = super::super::tests::quick_launch_application_fixture(cx);
    facts(cx, 1);
    open(cx, &app);
    app.update(cx, |app, cx| {
        app.apply_file_dialog_search_window_placement(cx)
    });
    cx.run_until_parked();
    let material = next(&endpoint);
    app.update(cx, |app, cx| {
        let mut replacement = host();
        replacement.window_handle = 88;
        replacement.changed_at_unix_ms = app.feature_events_after.saturating_add(1);
        app.handle_file_dialog_search_host(replacement, cx);
    });
    cx.run_until_parked();
    assert!(
        endpoint.try_receive_command_for_contract_test().is_err(),
        "retain the in-flight material record"
    );
    complete(cx, &endpoint, &material);
    cx.run_until_parked();
    reveal(&endpoint, 88);
}

#[gpui_kit::test]
fn file_dialog_material_timeout_keeps_retained_popup_hidden(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.executor().allow_parking();
    let (_temporary, app, endpoint) = super::super::tests::quick_launch_application_fixture(cx);
    facts(cx, 1);
    let window = open(cx, &app);
    app.update(cx, |app, cx| {
        app.apply_file_dialog_search_window_placement(cx)
    });
    cx.run_until_parked();
    let material = next(&endpoint);
    cx.executor().advance_clock(Duration::from_secs(4));
    cx.run_until_parked();
    let hide = next(&endpoint);
    assert!(matches!(
        hide.message,
        Message::PlatformCommand(PlatformCommand::SetWindowVisibility(
            SetWindowVisibilityCommand { visible: false, .. }
        ))
    ));
    cx.update(|cx| {
        assert!(!runtime::handle_notification(
            &nxtools_ipc::PlatformNotification::CommandResult {
                correlation_id: material.id,
                result: material_result(&material),
            },
            cx
        ))
    });
    app.update(cx, |app, cx| {
        app.apply_file_dialog_search_window_placement(cx)
    });
    cx.run_until_parked();
    assert!(endpoint.try_receive_command_for_contract_test().is_err());
    assert_eq!(
        app.read_with(cx, |app, _| app
            .file_dialog_search
            .window
            .as_ref()
            .expect("retained")
            .window),
        window
    );
}

#[gpui_kit::test]
fn file_dialog_material_disconnect_cannot_reveal(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.executor().allow_parking();
    let (_temporary, app, endpoint) = super::super::tests::quick_launch_application_fixture(cx);
    facts(cx, 1);
    open(cx, &app);
    app.update(cx, |app, cx| {
        app.apply_file_dialog_search_window_placement(cx)
    });
    cx.run_until_parked();
    let material = next(&endpoint);
    endpoint.set_connected_for_contract_test(false);
    cx.update(|cx| {
        let mut state = *cx.global::<UiMaterialCapabilities>();
        state.disconnected();
        cx.set_global(state);
    });
    cx.run_until_parked();
    cx.update(|cx| {
        assert!(!runtime::handle_notification(
            &nxtools_ipc::PlatformNotification::CommandResult {
                correlation_id: material.id,
                result: material_result(&material),
            },
            cx
        ))
    });
    assert!(endpoint.try_receive_command_for_contract_test().is_err());
    app.read_with(cx, |app, cx| {
        assert!(
            !app.file_dialog_search
                .window
                .as_ref()
                .expect("retained")
                .panel
                .upgrade()
                .expect("panel")
                .read(cx)
                .visible
        )
    });
}

#[gpui_kit::test]
fn file_dialog_material_native_failure_preserves_hidden_retained_window(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.executor().allow_parking();
    let (_temporary, app, endpoint) = super::super::tests::quick_launch_application_fixture(cx);
    facts(cx, 1);
    let window = open(cx, &app);
    app.update(cx, |app, cx| {
        app.apply_file_dialog_search_window_placement(cx)
    });
    cx.run_until_parked();
    let material = next(&endpoint);
    let result = PlatformCommandResult {
        success: false,
        error: Some("window_material_failed".into()),
        data: None,
        native_error_code: Some(5),
    };
    endpoint
        .route_command_result_for_contract_test(material.id, result.clone())
        .expect("native terminal");
    cx.update(|cx| {
        assert!(runtime::handle_notification(
            &nxtools_ipc::PlatformNotification::CommandResult {
                correlation_id: material.id,
                result,
            },
            cx
        ))
    });
    cx.run_until_parked();
    let hide = next(&endpoint);
    assert!(matches!(
        hide.message,
        Message::PlatformCommand(PlatformCommand::SetWindowVisibility(
            SetWindowVisibilityCommand { visible: false, .. }
        ))
    ));
    app.update(cx, |app, cx| {
        app.apply_file_dialog_search_window_placement(cx)
    });
    cx.run_until_parked();
    assert!(endpoint.try_receive_command_for_contract_test().is_err());
    assert_eq!(
        app.read_with(cx, |app, _| app
            .file_dialog_search
            .window
            .as_ref()
            .expect("retained")
            .window),
        window
    );
}

#[gpui_kit::test]
fn file_dialog_material_closed_window_cannot_reveal_from_late_receipt(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.executor().allow_parking();
    let (_temporary, app, endpoint) = super::super::tests::quick_launch_application_fixture(cx);
    facts(cx, 1);
    open(cx, &app);
    app.update(cx, |app, cx| {
        app.apply_file_dialog_search_window_placement(cx)
    });
    cx.run_until_parked();
    let material = next(&endpoint);
    app.update(cx, |app, cx| app.retire_file_dialog_search(cx));
    cx.run_until_parked();
    cx.update(|cx| {
        assert!(!runtime::handle_notification(
            &nxtools_ipc::PlatformNotification::CommandResult {
                correlation_id: material.id,
                result: material_result(&material),
            },
            cx
        ))
    });
    assert!(endpoint.try_receive_command_for_contract_test().is_err());
    assert!(app.read_with(cx, |app, _| app.file_dialog_search.window.is_none()));
}
