use super::super::window_picker_tests::{begin, event, fixture};
use super::*;
use crate::ui::material::{UiMaterialCapabilities, runtime};
use gpui_kit::{TestAppContext, VisualTestContext};
use nxtools_protocol::window_material::{
    AppliedWindowMaterial, ApplyWindowMaterialCommand, EffectiveWindowMaterial,
    MaterialFallbackReason, MaterialSurface,
};
use nxtools_protocol::{Message, PlatformCommandResult};
use std::time::Duration;

fn unprepared(
    application: &Entity<NetworkSettingsApplication>,
    view: &Entity<NetworkSettingsView>,
    surface: &mut VisualTestContext,
    endpoint: &nxtools_ipc::PlatformServerEndpoint,
) -> (uuid::Uuid, AnyWindowHandle) {
    let id = begin(application, view, surface);
    let start = endpoint
        .try_receive_command_for_contract_test()
        .expect("Start");
    assert!(matches!(start.message,
        Message::PlatformCommand(PlatformCommand::ProcessWindowPick(ProcessWindowPickCommand::Start {
            session_id, source_window_handle: 101, overlay_window_handle: 102,
        })) if session_id == id));
    let overlay = application.read_with(surface, |app, _| {
        app.pending_process_window_pick
            .as_ref()
            .expect("pending")
            .overlay
            .window
    });
    surface.update(|_, cx| {
        assert!(runtime::take_preparation(overlay, cx).is_some());
        assert!(runtime::receipt(overlay, cx).is_none());
    });
    (id, overlay)
}

fn frame(
    application: &Entity<NetworkSettingsApplication>,
    surface: &mut VisualTestContext,
    id: uuid::Uuid,
    overlay: AnyWindowHandle,
) {
    event(application, surface, id, WindowPickState::Hover);
    surface.update(|_, cx| {
        overlay
            .update(cx, |_, window, cx| window.simulate_next_frame(cx))
            .expect("Hover frame");
    });
    surface.run_until_parked();
}

fn next_material(
    endpoint: &nxtools_ipc::PlatformServerEndpoint,
) -> (uuid::Uuid, ApplyWindowMaterialCommand) {
    let request = endpoint
        .try_receive_command_for_contract_test()
        .expect("ApplyWindowMaterial before Show");
    let Message::PlatformCommand(PlatformCommand::ApplyWindowMaterial(command)) = request.message
    else {
        panic!("picker must not reveal before Transparent preparation");
    };
    assert_eq!(
        command.window_handle, 102,
        "never apply material to candidate HWND201"
    );
    assert_eq!(command.surface, MaterialSurface::Transparent);
    (request.id, command)
}

fn receipt(command: ApplyWindowMaterialCommand) -> AppliedWindowMaterial {
    AppliedWindowMaterial {
        session_id: command.session_id,
        facts_revision: command.facts_revision,
        window_handle: command.window_handle,
        generation: command.generation,
        style: command.style,
        surface: command.surface,
        dark: command.dark,
        effective: EffectiveWindowMaterial::Transparent,
        fallback_reason: MaterialFallbackReason::None,
        native_error_code: 0,
    }
}

fn terminal(
    surface: &mut VisualTestContext,
    endpoint: &nxtools_ipc::PlatformServerEndpoint,
    id: uuid::Uuid,
    applied: Option<AppliedWindowMaterial>,
) {
    let result = PlatformCommandResult {
        success: applied.is_some(),
        data: applied.map(|applied| serde_json::to_string(&applied).expect("synthetic receipt")),
        error: applied.is_none().then(|| "window_material_failed".into()),
        native_error_code: applied.is_none().then_some(5),
    };
    endpoint
        .route_command_result_for_contract_test(id, result.clone())
        .expect("material terminal");
    surface.update(|_, cx| {
        runtime::handle_notification(
            &PlatformNotification::CommandResult {
                correlation_id: id,
                result,
            },
            cx,
        );
    });
    surface.run_until_parked();
}

fn assert_cancel(endpoint: &nxtools_ipc::PlatformServerEndpoint, id: uuid::Uuid) {
    let cancel = endpoint
        .try_receive_command_for_contract_test()
        .expect("original native Cancel");
    assert!(matches!(cancel.message,
        Message::PlatformCommand(PlatformCommand::ProcessWindowPick(ProcessWindowPickCommand::Cancel { session_id }))
        if session_id == id));
    assert!(
        endpoint.try_receive_command_for_contract_test().is_err(),
        "no late ShowOverlay"
    );
}

#[gpui_kit::test]
fn material_picker_hover_frame_waits_for_current_transparent_receipt(cx: &mut TestAppContext) {
    let (application, view, mut surface, directory, endpoint) = fixture(cx);
    let (id, overlay) = unprepared(&application, &view, &mut surface, &endpoint);
    surface.run_until_parked();
    assert!(
        endpoint.try_receive_command_for_contract_test().is_err(),
        "Start stays hidden"
    );
    frame(&application, &mut surface, id, overlay);
    let (material_id, command) = next_material(&endpoint);
    assert!(application.read_with(&surface, |app, _| {
        app.pending_process_window_pick
            .as_ref()
            .expect("pending")
            .show_command
            .is_none()
    }));
    application.update(&mut surface, |app, cx| {
        app.show_process_picker_overlay(id, cx)
    });
    surface.run_until_parked();
    assert!(
        endpoint.try_receive_command_for_contract_test().is_err(),
        "repeat wait is single flight"
    );
    terminal(&mut surface, &endpoint, material_id, Some(receipt(command)));
    let show = endpoint
        .try_receive_command_for_contract_test()
        .expect("Show after receipt");
    assert_eq!(show.correlation_id, Some(id));
    assert!(matches!(show.message,
        Message::PlatformCommand(PlatformCommand::ProcessWindowPick(ProcessWindowPickCommand::ShowOverlay { session_id })) if session_id == id));
    surface.update(|_, cx| assert_eq!(runtime::receipt(overlay, cx), Some(receipt(command))));
    application.update(&mut surface, |app, cx| {
        app.handle_platform_notification(
            PlatformNotification::CommandResult {
                correlation_id: show.id,
                result: PlatformCommandResult {
                    success: true,
                    data: None,
                    error: None,
                    native_error_code: None,
                },
            },
            cx,
        );
        assert!(
            app.pending_process_window_pick.is_some(),
            "Show ACK keeps original selection session"
        );
    });
    application.update(&mut surface, |app, cx| app.close_window(cx));
    std::fs::remove_dir_all(directory).expect("cleanup");
}

#[gpui_kit::test]
fn material_picker_cached_receipt_does_not_reenter_overlay_lease(cx: &mut TestAppContext) {
    let (application, view, mut surface, directory, endpoint) = fixture(cx);
    let id = begin(&application, &view, &mut surface);
    endpoint
        .try_receive_command_for_contract_test()
        .expect("Start");
    let overlay = application.read_with(&surface, |app, _| {
        app.pending_process_window_pick
            .as_ref()
            .expect("pending")
            .overlay
            .window
    });
    surface.update(|_, cx| {
        overlay
            .update(cx, |_, _, cx| {
                application.update(cx, |app, cx| app.show_process_picker_overlay(id, cx));
            })
            .expect("cached readiness cannot lease target window again");
    });
    let show = endpoint
        .try_receive_command_for_contract_test()
        .expect("cached receipt admits original Show");
    assert!(
        matches!(show.message, Message::PlatformCommand(PlatformCommand::ProcessWindowPick(ProcessWindowPickCommand::ShowOverlay { session_id })) if session_id == id)
    );
    application.update(&mut surface, |app, cx| app.close_window(cx));
    std::fs::remove_dir_all(directory).expect("cleanup");
}

#[gpui_kit::test]
fn material_picker_repeated_wait_keeps_initial_deadline(cx: &mut TestAppContext) {
    let (application, view, mut surface, directory, endpoint) = fixture(cx);
    let (id, overlay) = unprepared(&application, &view, &mut surface, &endpoint);
    frame(&application, &mut surface, id, overlay);
    let (material_id, command) = next_material(&endpoint);
    let started = application.read_with(&surface, |app, _| {
        app.pending_process_window_pick
            .as_ref()
            .expect("pending")
            .started
    });
    surface.executor().advance_clock(Duration::from_secs(7));
    application.update(&mut surface, |app, cx| {
        app.show_process_picker_overlay(id, cx)
    });
    surface.run_until_parked();
    assert_eq!(
        application.read_with(&surface, |app, _| app
            .pending_process_window_pick
            .as_ref()
            .expect("pending")
            .started),
        started
    );
    surface.executor().advance_clock(Duration::from_secs(2));
    surface.run_until_parked();
    assert!(application.read_with(&surface, |app, _| app.pending_process_window_pick.is_none()));
    assert!(
        view.read_with(&surface, |view, _| view.process_window_pick.is_none()
            && view.error.is_some())
    );
    terminal(&mut surface, &endpoint, material_id, Some(receipt(command)));
    assert_cancel(&endpoint, id);
    application.update(&mut surface, |app, cx| app.close_window(cx));
    std::fs::remove_dir_all(directory).expect("cleanup");
}

#[gpui_kit::test]
fn material_picker_native_failure_uses_original_cancel(cx: &mut TestAppContext) {
    let (application, view, mut surface, directory, endpoint) = fixture(cx);
    let (id, overlay) = unprepared(&application, &view, &mut surface, &endpoint);
    frame(&application, &mut surface, id, overlay);
    let (material_id, _) = next_material(&endpoint);
    terminal(&mut surface, &endpoint, material_id, None);
    assert_cancel(&endpoint, id);
    assert!(
        view.read_with(&surface, |view, _| view.process_window_pick.is_none()
            && view.error.is_some())
    );
    assert!(application.read_with(&surface, |app, _| app.pending_process_window_pick.is_none()));
    application.update(&mut surface, |app, cx| app.close_window(cx));
    std::fs::remove_dir_all(directory).expect("cleanup");
}

#[gpui_kit::test]
fn material_picker_cancel_and_replacement_reject_late_receipt(cx: &mut TestAppContext) {
    let (application, view, mut surface, directory, endpoint) = fixture(cx);
    let (id, overlay) = unprepared(&application, &view, &mut surface, &endpoint);
    frame(&application, &mut surface, id, overlay);
    let (material_id, command) = next_material(&endpoint);
    application.update(&mut surface, |app, cx| {
        app.cancel_process_window_pick(id, "cancelled", cx)
    });
    surface.run_until_parked();
    assert_cancel(&endpoint, id);
    let replacement = begin(&application, &view, &mut surface);
    endpoint
        .try_receive_command_for_contract_test()
        .expect("replacement Start");
    terminal(&mut surface, &endpoint, material_id, Some(receipt(command)));
    assert!(endpoint.try_receive_command_for_contract_test().is_err());
    assert_eq!(
        application.read_with(&surface, |app, _| app
            .pending_process_window_pick
            .as_ref()
            .expect("replacement")
            .id),
        replacement
    );
    application.update(&mut surface, |app, cx| app.close_window(cx));
    std::fs::remove_dir_all(directory).expect("cleanup");
}

#[gpui_kit::test]
fn material_picker_close_rejects_late_receipt(cx: &mut TestAppContext) {
    let (application, view, mut surface, directory, endpoint) = fixture(cx);
    let (id, overlay) = unprepared(&application, &view, &mut surface, &endpoint);
    frame(&application, &mut surface, id, overlay);
    let (material_id, command) = next_material(&endpoint);
    application.update(&mut surface, |app, cx| app.close_window(cx));
    surface.run_until_parked();
    let result = PlatformCommandResult {
        success: true,
        error: None,
        native_error_code: None,
        data: Some(serde_json::to_string(&receipt(command)).expect("late receipt")),
    };
    endpoint
        .route_command_result_for_contract_test(material_id, result.clone())
        .expect("terminal");
    cx.update(|cx| {
        runtime::handle_notification(
            &PlatformNotification::CommandResult {
                correlation_id: material_id,
                result,
            },
            cx,
        );
    });
    cx.run_until_parked();
    assert_cancel(&endpoint, id);
    assert!(application.read_with(cx, |app, _| app.window.is_none()
        && app.pending_process_window_pick.is_none()));
    std::fs::remove_dir_all(directory).expect("cleanup");
}

#[gpui_kit::test]
fn material_picker_connection_replacement_rejects_late_receipt(cx: &mut TestAppContext) {
    let (application, view, mut surface, directory, endpoint) = fixture(cx);
    let (id, overlay) = unprepared(&application, &view, &mut surface, &endpoint);
    frame(&application, &mut surface, id, overlay);
    let (material_id, command) = next_material(&endpoint);
    endpoint.set_connected_for_contract_test(false);
    endpoint.set_connected_for_contract_test(true);
    terminal(&mut surface, &endpoint, material_id, Some(receipt(command)));
    assert!(
        endpoint.try_receive_command_for_contract_test().is_err(),
        "no Show on replacement connection"
    );
    assert!(application.read_with(&surface, |app, _| app.pending_process_window_pick.is_none()));
    assert!(view.read_with(&surface, |view, _| view.process_window_pick.is_none()));
    application.update(&mut surface, |app, cx| app.close_window(cx));
    std::fs::remove_dir_all(directory).expect("cleanup");
}

#[gpui_kit::test]
fn material_picker_stale_facts_require_new_current_receipt(cx: &mut TestAppContext) {
    let (application, view, mut surface, directory, endpoint) = fixture(cx);
    let (id, overlay) = unprepared(&application, &view, &mut surface, &endpoint);
    frame(&application, &mut surface, id, overlay);
    let (material_id, command) = next_material(&endpoint);
    surface.update(|_, cx| {
        let mut facts = *cx.global::<UiMaterialCapabilities>();
        let mut snapshot = facts.snapshot().expect("facts");
        snapshot.revision += 1;
        assert!(facts.apply(snapshot));
        cx.set_global(facts);
    });
    terminal(&mut surface, &endpoint, material_id, Some(receipt(command)));
    let (current_id, current) = loop {
        let request = endpoint
            .try_receive_command_for_contract_test()
            .expect("current material request");
        let Message::PlatformCommand(PlatformCommand::ApplyWindowMaterial(next)) = request.message
        else {
            panic!("stale receipt cannot reveal picker");
        };
        if next.window_handle == 102 {
            assert_eq!(next.surface, MaterialSurface::Transparent);
            assert!(next.facts_revision > command.facts_revision);
            break (request.id, next);
        }
        assert_eq!(
            next.window_handle, 101,
            "only source and overlay are material targets"
        );
        assert_eq!(next.surface, MaterialSurface::Backdrop);
        let mut applied = receipt(next);
        applied.effective = EffectiveWindowMaterial::Solid;
        terminal(&mut surface, &endpoint, request.id, Some(applied));
    };
    terminal(&mut surface, &endpoint, current_id, Some(receipt(current)));
    let show = endpoint
        .try_receive_command_for_contract_test()
        .expect("current receipt Show");
    assert!(
        matches!(show.message, Message::PlatformCommand(PlatformCommand::ProcessWindowPick(ProcessWindowPickCommand::ShowOverlay { session_id })) if session_id == id)
    );
    application.update(&mut surface, |app, cx| app.close_window(cx));
    std::fs::remove_dir_all(directory).expect("cleanup");
}

#[gpui_kit::test]
fn material_picker_unprepared_admission_leaves_overlay_lease(cx: &mut TestAppContext) {
    let (application, view, mut surface, directory, endpoint) = fixture(cx);
    let (id, overlay) = unprepared(&application, &view, &mut surface, &endpoint);
    surface.update(|_, cx| {
        overlay
            .update(cx, |_, _, cx| {
                application.update(cx, |app, cx| app.show_process_picker_overlay(id, cx));
                assert!(
                    endpoint.try_receive_command_for_contract_test().is_err(),
                    "native material dispatch leaves the current window lease"
                );
            })
            .expect("unprepared admission cannot synchronously lease its overlay");
    });
    surface.run_until_parked();
    let (material_id, command) = next_material(&endpoint);
    terminal(&mut surface, &endpoint, material_id, Some(receipt(command)));
    let show = endpoint
        .try_receive_command_for_contract_test()
        .expect("prepared Show");
    assert!(matches!(show.message,
        Message::PlatformCommand(PlatformCommand::ProcessWindowPick(ProcessWindowPickCommand::ShowOverlay { session_id })) if session_id == id));
    application.update(&mut surface, |app, cx| app.close_window(cx));
    std::fs::remove_dir_all(directory).expect("cleanup");
}
