use super::*;
use gpui_kit::TestAppContext;
use nxtools_ipc::{PlatformBridge, PlatformServerEndpoint};
use nxtools_protocol::window_material::{EffectiveWindowMaterial, MaterialFallbackReason};
use nxtools_protocol::{Message, PlatformCommand, PlatformCommandResult};
use uuid::Uuid;

fn facts(cx: &mut App, revision: u64) {
    let state = cx.default_global::<UiMaterialCapabilities>();
    state.connected();
    let session_id = state
        .snapshot()
        .map_or_else(Uuid::new_v4, |facts| facts.session_id);
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
}
fn next(endpoint: &PlatformServerEndpoint) -> (Uuid, ApplyWindowMaterialCommand) {
    let envelope = endpoint
        .try_receive_command_for_contract_test()
        .expect("material dispatch");
    let Message::PlatformCommand(PlatformCommand::ApplyWindowMaterial(command)) = envelope.message
    else {
        panic!("material command");
    };
    (envelope.id, command)
}
fn result(command: ApplyWindowMaterialCommand) -> PlatformCommandResult {
    let receipt = AppliedWindowMaterial {
        session_id: command.session_id,
        facts_revision: command.facts_revision,
        window_handle: command.window_handle,
        generation: command.generation,
        style: command.style,
        surface: command.surface,
        dark: command.dark,
        effective: match (command.surface, command.style) {
            (MaterialSurface::Transparent, _) => EffectiveWindowMaterial::Transparent,
            (_, WindowMaterialStyle::Solid) => EffectiveWindowMaterial::Solid,
            (_, _) => EffectiveWindowMaterial::Acrylic,
        },
        fallback_reason: MaterialFallbackReason::None,
        native_error_code: 0,
    };
    PlatformCommandResult {
        success: true,
        error: None,
        native_error_code: None,
        data: Some(serde_json::to_string(&receipt).expect("receipt")),
    }
}
fn ensure(preparation: &mut Preparation, bridge: &PlatformBridge, cx: &mut TestAppContext) -> bool {
    cx.update(|cx| {
        preparation
            .ensure(bridge, 42, MaterialSurface::Backdrop, cx)
            .expect("material admission")
    })
}

#[gpui_kit::test]
fn material_preparation_waits_for_sampled_facts_and_exact_receipt(cx: &mut TestAppContext) {
    let (bridge, endpoint) = nxtools_ipc::platform_bridge(8);
    endpoint.set_connected_for_contract_test(true);
    let mut preparation = Preparation::default();
    assert!(!ensure(&mut preparation, &bridge, cx));
    assert!(endpoint.try_receive_command_for_contract_test().is_err());
    cx.update(|cx| facts(cx, 1));
    assert!(!ensure(&mut preparation, &bridge, cx));
    let (id, command) = next(&endpoint);
    assert!(!ensure(&mut preparation, &bridge, cx));
    assert!(endpoint.try_receive_command_for_contract_test().is_err());
    cx.update(|cx| {
        assert_eq!(
            preparation.accept(Uuid::new_v4(), &result(command), cx),
            None
        );
        assert_eq!(
            preparation.accept(
                id,
                &result(ApplyWindowMaterialCommand {
                    window_handle: 43,
                    ..command
                }),
                cx
            ),
            Some(Err("native_material_result"))
        );
        assert!(preparation.receipt(cx).is_none());
        assert_eq!(preparation.accept(id, &result(command), cx), Some(Ok(true)));
        assert!(preparation.receipt(cx).is_some());
    });
    assert!(ensure(&mut preparation, &bridge, cx));
}

#[gpui_kit::test]
fn material_preparation_latest_queued_facts_replace_without_overlapping_native_work(
    cx: &mut TestAppContext,
) {
    let (bridge, endpoint) = nxtools_ipc::platform_bridge(8);
    endpoint.set_connected_for_contract_test(true);
    cx.update(|cx| facts(cx, 1));
    let mut active = Preparation::default();
    cx.update(|cx| {
        assert!(
            !active
                .ensure(&bridge, 43, MaterialSurface::Backdrop, cx)
                .expect("first window")
        );
    });
    let (active_id, active_command) = next(&endpoint);
    let mut queued = Preparation::default();
    assert!(!ensure(&mut queued, &bridge, cx));
    let old_id = queued.pending.as_ref().expect("queued").id;
    cx.update(|cx| facts(cx, 2));
    assert!(!ensure(&mut queued, &bridge, cx));
    assert!(!endpoint.command_pending_for_contract_test(old_id));
    assert!(endpoint.try_receive_command_for_contract_test().is_err());
    endpoint
        .route_command_result_for_contract_test(active_id, result(active_command))
        .expect("native finished");
    let (current_id, current) = next(&endpoint);
    assert_eq!(current.facts_revision, 2);
    cx.update(|cx| {
        assert_eq!(queued.accept(old_id, &result(current), cx), None);
        assert_eq!(
            queued.accept(current_id, &result(current), cx),
            Some(Ok(true))
        );
    });
    assert!(ensure(&mut queued, &bridge, cx));
}

#[gpui_kit::test]
fn material_preparation_retirement_disconnect_and_transparent_host_keep_identity(
    cx: &mut TestAppContext,
) {
    let (bridge, endpoint) = nxtools_ipc::platform_bridge(8);
    endpoint.set_connected_for_contract_test(true);
    cx.update(|cx| facts(cx, 1));
    let mut preparation = Preparation::default();
    cx.update(|cx| {
        assert!(
            !preparation
                .ensure(&bridge, 42, MaterialSurface::Transparent, cx)
                .expect("transparent policy")
        );
    });
    let (id, command) = next(&endpoint);
    assert_eq!(command.surface, MaterialSurface::Transparent);
    cx.update(|cx| {
        assert_eq!(preparation.accept(id, &result(command), cx), Some(Ok(true)));
    });
    preparation.retire(&bridge);
    assert!(
        endpoint.command_pending_for_contract_test(id),
        "active native slot is not locally cancelled"
    );
    cx.update(|cx| {
        assert!(preparation.receipt(cx).is_none());
        assert_eq!(preparation.accept(id, &result(command), cx), None);
        cx.default_global::<UiMaterialCapabilities>().disconnected();
        assert!(
            !preparation
                .ensure(&bridge, 42, MaterialSurface::Transparent, cx)
                .expect("waiting reconnect facts")
        );
    });
    assert!(endpoint.try_receive_command_for_contract_test().is_err());
}

#[gpui_kit::test]
fn material_preparation_stale_native_facts_wait_without_resubmission(cx: &mut TestAppContext) {
    let (bridge, endpoint) = nxtools_ipc::platform_bridge(8);
    endpoint.set_connected_for_contract_test(true);
    cx.update(|cx| facts(cx, 1));
    let mut preparation = Preparation::default();
    assert!(!ensure(&mut preparation, &bridge, cx));
    let (id, _) = next(&endpoint);
    let stale = PlatformCommandResult {
        success: false,
        error: Some("stale_material_facts".into()),
        native_error_code: None,
        data: None,
    };
    endpoint
        .route_command_result_for_contract_test(id, stale.clone())
        .expect("stale terminal");
    cx.update(|cx| {
        assert_eq!(preparation.accept(id, &stale, cx), Some(Ok(false)));
    });
    assert!(!ensure(&mut preparation, &bridge, cx));
    assert!(endpoint.try_receive_command_for_contract_test().is_err());
    cx.update(|cx| facts(cx, 2));
    assert!(!ensure(&mut preparation, &bridge, cx));
    let (_, current) = next(&endpoint);
    assert_eq!(current.facts_revision, 2);
}

#[gpui_kit::test]
fn material_preparation_intent_change_invalidates_previous_effective_receipt(
    cx: &mut TestAppContext,
) {
    let (bridge, endpoint) = nxtools_ipc::platform_bridge(8);
    endpoint.set_connected_for_contract_test(true);
    cx.update(|cx| facts(cx, 1));
    let mut preparation = Preparation::default();
    assert!(!ensure(&mut preparation, &bridge, cx));
    let (id, command) = next(&endpoint);
    endpoint
        .route_command_result_for_contract_test(id, result(command))
        .expect("first terminal");
    cx.update(|cx| {
        assert_eq!(preparation.accept(id, &result(command), cx), Some(Ok(true)));
        let state = cx.default_global::<crate::theme::UiThemeState>();
        state.material_style = nxtools_core::MaterialStyle::Acrylic;
        assert!(preparation.receipt(cx).is_none());
    });
    assert!(!ensure(&mut preparation, &bridge, cx));
    let (_, current) = next(&endpoint);
    assert_eq!(current.style, WindowMaterialStyle::Acrylic);
    assert!(current.generation > command.generation);
}

#[gpui_kit::test]
fn material_preparation_failed_admission_never_retains_a_pending_record(cx: &mut TestAppContext) {
    let (bridge, endpoint) = nxtools_ipc::platform_bridge(1);
    endpoint.set_connected_for_contract_test(true);
    cx.update(|cx| facts(cx, 1));
    let mut first = Preparation::default();
    assert!(!ensure(&mut first, &bridge, cx));
    let mut rejected = Preparation::default();
    cx.update(|cx| {
        assert_eq!(
            rejected.ensure(&bridge, 43, MaterialSurface::Backdrop, cx),
            Err("material_enqueue")
        );
        assert!(rejected.pending.is_none());
        assert!(rejected.receipt(cx).is_none());
    });
    first.retire(&bridge);
    cx.update(|cx| {
        assert_eq!(
            rejected.ensure(&bridge, 43, MaterialSurface::Backdrop, cx),
            Ok(false)
        );
    });
    let (_, command) = next(&endpoint);
    assert_eq!(command.window_handle, 43);
}

#[gpui_kit::test]
fn material_preparation_native_appearance_invalidates_receipt_without_changing_explicit_mode(
    cx: &mut TestAppContext,
) {
    use gpui_kit::{Context, IntoElement, Render, Window};
    struct Surface;
    impl Render for Surface {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            gpui_kit::div()
        }
    }
    let window = cx.add_window(|_, _| Surface).into();
    let (bridge, endpoint) = nxtools_ipc::platform_bridge(8);
    endpoint.set_connected_for_contract_test(true);
    cx.update(|cx| {
        facts(cx, 1);
        let appearance = crate::theme::UiThemeState {
            preference: nxtools_core::ThemePreference::Dark,
            ..Default::default()
        };
        cx.set_global(appearance);
        native_appearance::observe_window(window, 42, cx);
    });
    let mut preparation = Preparation::default();
    assert!(!ensure(&mut preparation, &bridge, cx));
    let (id, old) = next(&endpoint);
    endpoint
        .route_command_result_for_contract_test(id, result(old))
        .expect("terminal");
    cx.update(|cx| {
        assert_eq!(preparation.accept(id, &result(old), cx), Some(Ok(true)));
        native_appearance::invalidate(42, cx);
        assert!(preparation.receipt(cx).is_none());
    });
    assert!(!ensure(&mut preparation, &bridge, cx));
    let (_, latest) = next(&endpoint);
    assert!(latest.dark);
    assert_eq!(latest.facts_revision, old.facts_revision);
    assert!(latest.generation > old.generation);
    cx.update(|cx| {
        window
            .update(cx, |_, window, _| window.remove_window())
            .expect("close");
    });
    cx.run_until_parked();
    cx.update(|cx| assert!(!native_appearance::is_observed(42, cx)));
}
