use super::*;
use gpui_kit::{Context, IntoElement, Render, TestAppContext, Window};
use nxtools_ipc::PlatformServerEndpoint;
use nxtools_protocol::window_material::{EffectiveWindowMaterial, MaterialFallbackReason};
use nxtools_protocol::{Message, PlatformCommand, PlatformCommandResult};
use uuid::Uuid;

struct Surface;
impl Render for Surface {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        gpui_kit::div()
    }
}
pub(super) fn sample(session_id: Uuid, revision: u64, cx: &mut App) {
    let mut state = cx
        .try_global::<UiMaterialCapabilities>()
        .copied()
        .unwrap_or_default();
    state.connected();
    assert!(state.apply(SystemMaterialCapabilitiesChanged {
        correlation_id: Uuid::new_v4(),
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
pub(super) fn applied(command: ApplyWindowMaterialCommand) -> AppliedWindowMaterial {
    AppliedWindowMaterial {
        session_id: command.session_id,
        facts_revision: command.facts_revision,
        window_handle: command.window_handle,
        generation: command.generation,
        style: command.style,
        surface: command.surface,
        dark: command.dark,
        effective: if command.style == WindowMaterialStyle::Solid {
            EffectiveWindowMaterial::Solid
        } else {
            EffectiveWindowMaterial::Acrylic
        },
        fallback_reason: MaterialFallbackReason::None,
        native_error_code: 0,
    }
}
pub(super) fn next(endpoint: &PlatformServerEndpoint) -> (Uuid, ApplyWindowMaterialCommand) {
    let envelope = endpoint
        .try_receive_command_for_contract_test()
        .expect("material only");
    let Message::PlatformCommand(PlatformCommand::ApplyWindowMaterial(command)) = envelope.message
    else {
        panic!("runtime must never show, hide, reposition or activate");
    };
    (envelope.id, command)
}
pub(super) fn fixture(
    cx: &mut TestAppContext,
) -> (
    AnyWindowHandle,
    Rc<PlatformBridge>,
    PlatformServerEndpoint,
    Uuid,
) {
    cx.update(gpui_kit::init);
    cx.executor().allow_parking();
    let (bridge, endpoint) = nxtools_ipc::platform_bridge(8);
    endpoint.set_connected_for_contract_test(true);
    let bridge = Rc::new(bridge);
    let window: AnyWindowHandle = cx.add_window(|_, _| Surface).into();
    let session = Uuid::new_v4();
    cx.update(|cx| {
        sample(session, 1, cx);
        let command =
            super::super::command(42, MaterialSurface::Backdrop, cx).expect("initial facts");
        let preparation = Preparation::confirmed(Uuid::new_v4(), command, applied(command), cx)
            .expect("owned first-show receipt");
        assert!(track_prepared(window, Rc::clone(&bridge), preparation, cx));
    });
    cx.run_until_parked();
    (window, bridge, endpoint, session)
}

#[gpui_kit::test]
fn material_runtime_reapplies_intent_without_changing_window_or_presentation(
    cx: &mut TestAppContext,
) {
    let (window, bridge, endpoint, _) = fixture(cx);
    assert!(
        endpoint.try_receive_command_for_contract_test().is_err(),
        "adoption must not repeat native apply"
    );
    cx.update(|cx| {
        let mut appearance = cx
            .try_global::<crate::theme::UiThemeState>()
            .copied()
            .unwrap_or_default();
        appearance.material_style = nxtools_core::MaterialStyle::Acrylic;
        cx.set_global(appearance);
    });
    cx.run_until_parked();
    let (id, command) = next(&endpoint);
    assert_eq!(command.style, WindowMaterialStyle::Acrylic);
    assert_eq!(command.window_handle, 42);
    assert!(cx.update(|cx| cx.windows().contains(&window)));
    assert!(endpoint.try_receive_command_for_contract_test().is_err());
    endpoint
        .route_command_result_for_contract_test(
            id,
            PlatformCommandResult {
                success: true,
                error: None,
                native_error_code: None,
                data: Some(serde_json::to_string(&applied(command)).expect("receipt")),
            },
        )
        .expect("production pending terminal");
    let notification = bridge
        .notification_receiver()
        .try_recv()
        .expect("original notification");
    cx.update(|cx| assert!(handle_notification(&notification, cx)));
    cx.run_until_parked();
    cx.update(|cx| {
        let runtime = cx.global::<MaterialRuntime>();
        assert_eq!(runtime.windows.len(), 1);
        assert_eq!(runtime.windows[0].window, window);
        assert_eq!(
            runtime.windows[0].preparation.receipt(cx),
            Some(applied(command))
        );
    });
    assert!(
        endpoint.try_receive_command_for_contract_test().is_err(),
        "no apply loop after a valid receipt"
    );
}

#[gpui_kit::test]
fn material_runtime_disconnect_waits_for_new_session_facts_and_ignores_old_receipt(
    cx: &mut TestAppContext,
) {
    let (window, _bridge, endpoint, session) = fixture(cx);
    cx.update(|cx| sample(session, 2, cx));
    cx.run_until_parked();
    let (old_id, old) = next(&endpoint);
    endpoint.set_connected_for_contract_test(false);
    cx.update(|cx| {
        let mut state = cx.global::<UiMaterialCapabilities>().to_owned();
        state.disconnected();
        cx.set_global(state);
    });
    cx.run_until_parked();
    endpoint.set_connected_for_contract_test(true);
    cx.update(|cx| {
        let mut state = cx.global::<UiMaterialCapabilities>().to_owned();
        state.connected();
        cx.set_global(state);
    });
    cx.run_until_parked();
    assert!(
        endpoint.try_receive_command_for_contract_test().is_err(),
        "connection alone cannot reapply"
    );
    let current_session = Uuid::new_v4();
    cx.update(|cx| sample(current_session, 1, cx));
    cx.run_until_parked();
    let (_, current) = next(&endpoint);
    assert_eq!(current.session_id, current_session);
    assert!(current.generation > old.generation);
    let old_result = PlatformNotification::CommandResult {
        correlation_id: old_id,
        result: PlatformCommandResult {
            success: true,
            error: None,
            native_error_code: None,
            data: Some(serde_json::to_string(&applied(old)).expect("old receipt")),
        },
    };
    cx.update(|cx| {
        assert!(!handle_notification(&old_result, cx));
        assert_eq!(cx.global::<MaterialRuntime>().windows[0].window, window);
        assert!(
            cx.global::<MaterialRuntime>().windows[0]
                .preparation
                .receipt(cx)
                .is_none()
        );
    });
}

#[gpui_kit::test]
fn material_runtime_closed_window_retires_queued_request_and_cannot_be_revived(
    cx: &mut TestAppContext,
) {
    let (window, bridge, endpoint, session) = fixture(cx);
    let mut blocking = Preparation::default();
    cx.update(|cx| {
        assert_eq!(
            blocking.ensure(&bridge, 43, MaterialSurface::Backdrop, cx),
            Ok(false)
        );
    });
    let (active, _) = next(&endpoint);
    cx.update(|cx| sample(session, 2, cx));
    cx.run_until_parked();
    let queued = cx.update(|cx| {
        cx.global::<MaterialRuntime>().windows[0]
            .preparation
            .pending
            .as_ref()
            .expect("queued update")
            .id
    });
    assert!(endpoint.command_pending_for_contract_test(queued));
    cx.update(|cx| {
        window
            .update(cx, |_, window, _| window.remove_window())
            .expect("close owned window");
    });
    cx.run_until_parked();
    assert!(!endpoint.command_pending_for_contract_test(queued));
    assert!(
        endpoint.command_pending_for_contract_test(active),
        "unrelated active material slot remains owned"
    );
    assert!(endpoint.try_receive_command_for_contract_test().is_err());
    cx.update(|cx| {
        assert!(cx.global::<MaterialRuntime>().windows.is_empty());
        assert!(!handle_notification(
            &PlatformNotification::CommandResult {
                correlation_id: queued,
                result: PlatformCommandResult {
                    success: false,
                    error: None,
                    native_error_code: None,
                    data: None
                },
            },
            cx
        ));
        assert!(!cx.windows().contains(&window));
    });
}

#[gpui_kit::test]
fn material_runtime_checkout_transfers_inflight_receipt_to_only_presentation_owner(
    cx: &mut TestAppContext,
) {
    let (window, bridge, endpoint, session) = fixture(cx);
    cx.update(|cx| sample(session, 2, cx));
    cx.run_until_parked();
    let (id, command) = next(&endpoint);
    let mut preparation = cx.update(|cx| {
        let preparation = take_preparation(window, cx).expect("reuse owns original preparation");
        assert!(take_preparation(window, cx).is_none());
        assert!(receipt(window, cx).is_none());
        preparation
    });
    let result = PlatformCommandResult {
        success: true,
        error: None,
        native_error_code: None,
        data: Some(serde_json::to_string(&applied(command)).expect("receipt")),
    };
    endpoint
        .route_command_result_for_contract_test(id, result)
        .expect("original terminal");
    let notification = bridge
        .notification_receiver()
        .try_recv()
        .expect("original notification");
    cx.update(|cx| {
        assert!(!handle_notification(&notification, cx));
        let PlatformNotification::CommandResult {
            correlation_id,
            result,
        } = notification
        else {
            panic!("terminal");
        };
        assert_eq!(
            preparation.accept(correlation_id, &result, cx),
            Some(Ok(true))
        );
        assert!(track_prepared(window, bridge, preparation, cx));
        assert_eq!(receipt(window, cx), Some(applied(command)));
    });
    cx.run_until_parked();
    assert!(endpoint.try_receive_command_for_contract_test().is_err());
}

#[gpui_kit::test]
fn material_runtime_deferred_handoff_reconciles_new_facts_without_losing_window(
    cx: &mut TestAppContext,
) {
    let (window, bridge, endpoint, session) = fixture(cx);
    cx.update(|cx| {
        let preparation = take_preparation(window, cx).expect("confirmed first show");
        sample(session, 2, cx);
        assert!(track_prepared(window, bridge, preparation, cx));
        assert!(receipt(window, cx).is_none());
    });
    cx.run_until_parked();
    let (_, command) = next(&endpoint);
    assert_eq!(command.facts_revision, 2);
    assert_eq!(command.window_handle, 42);
    assert!(endpoint.try_receive_command_for_contract_test().is_err());
}
