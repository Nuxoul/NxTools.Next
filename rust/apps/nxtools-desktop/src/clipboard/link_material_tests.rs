use super::*;
use crate::ui::material::{UiMaterialCapabilities, runtime};
use gpui_kit::App;
use nxtools_protocol::window_material::{
    AppliedWindowMaterial, EffectiveWindowMaterial, MaterialFallbackReason,
    SystemMaterialCapabilitiesChanged,
};

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

pub(super) fn facts(cx: &mut TestAppContext, revision: u64) {
    cx.update(|cx| sample(cx, revision));
}

fn material_result(request: &Envelope) -> PlatformCommandResult {
    let Message::PlatformCommand(PlatformCommand::ApplyWindowMaterial(command)) = request.message
    else {
        panic!("material must precede visibility");
    };
    assert_eq!(command.window_handle, 9001);
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
            .expect("material receipt"),
        ),
    }
}

pub(super) fn material_reply(
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
            &PlatformNotification::CommandResult {
                correlation_id: request.id,
                result,
            },
            cx
        ))
    });
}

fn material_reply_then_facts(
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
            &PlatformNotification::CommandResult {
                correlation_id: request.id,
                result,
            },
            cx
        ));
        sample(cx, revision);
    });
}

pub(super) fn approve_stage(
    cx: &mut TestAppContext,
    endpoint: &nxtools_ipc::PlatformServerEndpoint,
    window: AnyWindowHandle,
    first: bool,
) {
    cx.update(|cx| runtime::set_test_window_handle(window, 9001, cx));
    cx.run_until_parked();
    if first {
        let material = command(endpoint);
        material_reply(cx, endpoint, &material);
        cx.run_until_parked();
    }
}

fn stage_wait(
    cx: &mut TestAppContext,
    owner: &Entity<ClipboardApplication>,
    endpoint: &nxtools_ipc::PlatformServerEndpoint,
) -> AnyWindowHandle {
    owner.update(cx, |owner, cx| owner.request_link_panel(links(), cx));
    let query = command(endpoint);
    reply(
        cx,
        owner,
        &query,
        Some(
            serde_json::to_string(&CursorMonitorSnapshot {
                display_id: 9,
                work_left: -1920,
                work_top: 40,
                work_width: 1920,
                work_height: 1040,
                dpi: 144,
            })
            .expect("monitor"),
        ),
    );
    let window = owner.read_with(cx, |owner, _| owner.link_panel.window.expect("panel"));
    cx.update(|cx| runtime::set_test_window_handle(window, 9001, cx));
    for _ in 0..3 {
        let request = command(endpoint);
        assert!(!matches!(
            request.message,
            Message::PlatformCommand(PlatformCommand::SetWindowVisibility(
                SetWindowVisibilityCommand { visible: true, .. }
            ))
        ));
        reply(cx, owner, &request, None);
    }
    cx.run_until_parked();
    window
}

#[gpui_kit::test]
fn link_material_gates_offscreen_stage_and_rechecks_final_reveal(cx: &mut TestAppContext) {
    let (_temporary, owner, endpoint) = fixture(cx);
    facts(cx, 1);
    let window = stage_wait(cx, &owner, &endpoint);
    let material = command(&endpoint);
    assert!(
        endpoint.try_receive_command_for_contract_test().is_err(),
        "no stage show before receipt"
    );
    material_reply_then_facts(cx, &endpoint, &material, 2);
    cx.run_until_parked();
    let replacement = command(&endpoint);
    material_reply(cx, &endpoint, &replacement);
    cx.run_until_parked();
    let show = command(&endpoint);
    assert!(matches!(
        show.message,
        Message::PlatformCommand(PlatformCommand::SetWindowVisibility(
            SetWindowVisibilityCommand { visible: true, .. }
        ))
    ));
    reply(cx, &owner, &show, None);
    cx.update(|cx| {
        window
            .update(cx, |_, window, cx| {
                window.draw(cx).clear(cx);
                window.simulate_next_frame(cx);
            })
            .expect("stage frame")
    });
    cx.run_until_parked();
    for _ in 0..2 {
        let request = command(&endpoint);
        reply(cx, &owner, &request, None);
    }
    let bounds = command(&endpoint);
    facts(cx, 3);
    reply(cx, &owner, &bounds, None);
    cx.run_until_parked();
    let latest = command(&endpoint);
    assert!(endpoint.try_receive_command_for_contract_test().is_err());
    material_reply(cx, &endpoint, &latest);
    cx.run_until_parked();
    let reveal = command(&endpoint);
    assert!(matches!(
        reveal.message,
        Message::PlatformCommand(PlatformCommand::SetWindowVisibility(
            SetWindowVisibilityCommand { visible: true, .. }
        ))
    ));
    reply(cx, &owner, &reveal, None);
    assert!(owner.read_with(cx, |owner, _| owner.pending_link_placement.is_none()));
    assert!(cx.update(|cx| runtime::receipt(window, cx).is_some()));
}

#[gpui_kit::test]
fn link_material_retired_generation_cannot_stage_on_late_receipt(cx: &mut TestAppContext) {
    let (_temporary, owner, endpoint) = fixture(cx);
    facts(cx, 1);
    stage_wait(cx, &owner, &endpoint);
    let material = command(&endpoint);
    owner.update(cx, |owner, cx| owner.retire_link_placement("cancelled", cx));
    let result = material_result(&material);
    endpoint
        .route_command_result_for_contract_test(material.id, result.clone())
        .expect("late native terminal");
    cx.update(|cx| {
        assert!(!runtime::handle_notification(
            &PlatformNotification::CommandResult {
                correlation_id: material.id,
                result,
            },
            cx
        ))
    });
    cx.run_until_parked();
    assert!(endpoint.try_receive_command_for_contract_test().is_err());
    assert!(owner.read_with(cx, |owner, _| owner.link_panel.window.is_none()));
}

#[gpui_kit::test]
fn link_material_timeout_cannot_release_stage(cx: &mut TestAppContext) {
    let (_temporary, owner, endpoint) = fixture(cx);
    facts(cx, 1);
    stage_wait(cx, &owner, &endpoint);
    let material = command(&endpoint);
    cx.executor().advance_clock(Duration::from_secs(4));
    cx.run_until_parked();
    assert!(owner.read_with(cx, |owner, _| owner.pending_link_placement.is_none()));
    let result = material_result(&material);
    cx.update(|cx| {
        assert!(!runtime::handle_notification(
            &PlatformNotification::CommandResult {
                correlation_id: material.id,
                result,
            },
            cx
        ))
    });
    assert!(endpoint.try_receive_command_for_contract_test().is_err());
}

#[gpui_kit::test]
fn link_material_disconnect_cannot_release_stage(cx: &mut TestAppContext) {
    let (_temporary, owner, endpoint) = fixture(cx);
    facts(cx, 1);
    stage_wait(cx, &owner, &endpoint);
    let material = command(&endpoint);
    endpoint.set_connected_for_contract_test(false);
    cx.update(|cx| {
        let mut state = *cx.global::<UiMaterialCapabilities>();
        state.disconnected();
        cx.set_global(state);
    });
    cx.run_until_parked();
    assert!(owner.read_with(cx, |owner, _| owner.pending_link_placement.is_none()));
    let result = material_result(&material);
    cx.update(|cx| {
        assert!(!runtime::handle_notification(
            &PlatformNotification::CommandResult {
                correlation_id: material.id,
                result,
            },
            cx
        ))
    });
    assert!(endpoint.try_receive_command_for_contract_test().is_err());
}

#[gpui_kit::test]
fn link_material_wrong_hwnd_receipt_cannot_stage(cx: &mut TestAppContext) {
    let (_temporary, owner, endpoint) = fixture(cx);
    facts(cx, 1);
    stage_wait(cx, &owner, &endpoint);
    let material = command(&endpoint);
    let mut result = material_result(&material);
    let mut receipt: AppliedWindowMaterial =
        serde_json::from_str(result.data.as_deref().expect("receipt")).expect("typed receipt");
    receipt.window_handle = 77;
    result.data = Some(serde_json::to_string(&receipt).expect("wrong HWND receipt"));
    endpoint
        .route_command_result_for_contract_test(material.id, result.clone())
        .expect("native terminal");
    cx.update(|cx| {
        assert!(runtime::handle_notification(
            &PlatformNotification::CommandResult {
                correlation_id: material.id,
                result,
            },
            cx
        ))
    });
    cx.run_until_parked();
    assert!(
        owner.read_with(cx, |owner, _| owner.pending_link_placement.is_none()
            && owner.link_panel.window.is_none())
    );
    assert!(endpoint.try_receive_command_for_contract_test().is_err());
}
