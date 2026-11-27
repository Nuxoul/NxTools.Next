use super::tests::{fixture_unprepared, next, operation, reply};
use super::*;
use crate::ui::material::{self, Preparation, UiMaterialCapabilities, runtime};
use gpui_kit::{App, TestAppContext};
use nxtools_ipc::PlatformServerEndpoint;
use nxtools_protocol::{Message, PlatformCommandResult, window_material::*};

fn facts(revision: u64, cx: &mut App) {
    let mut state = cx
        .try_global::<UiMaterialCapabilities>()
        .copied()
        .unwrap_or_default();
    let session_id = state
        .snapshot()
        .map_or_else(Uuid::new_v4, |facts| facts.session_id);
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
fn applied(command: ApplyWindowMaterialCommand) -> AppliedWindowMaterial {
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
pub(super) fn prepare_fixture(
    owner: &Entity<FloatingAssistantApplication>,
    cx: &mut TestAppContext,
) {
    cx.update(|cx| {
        if cx
            .try_global::<UiMaterialCapabilities>()
            .and_then(|state| state.snapshot())
            .is_none()
        {
            facts(1, cx);
        }
    });
    let (window, hwnd, platform) = owner.update(cx, |owner, cx| {
        assert!(owner.open_surface(cx));
        let surface = owner.surface.as_ref().expect("retained surface");
        (surface.window, surface.hwnd, owner.platform.clone())
    });
    cx.update(|cx| {
        runtime::set_test_window_handle(window, hwnd, cx);
        let command = material::command(hwnd, MaterialSurface::Transparent, cx).expect("facts");
        let preparation = Preparation::confirmed(Uuid::new_v4(), command, applied(command), cx)
            .expect("coherent transparent receipt");
        assert!(runtime::track_prepared(window, platform, preparation, cx));
    });
}
fn begin(
    cx: &mut TestAppContext,
) -> (
    Entity<FloatingAssistantApplication>,
    PlatformServerEndpoint,
    tempfile::TempDir,
) {
    let (owner, endpoint, directory) = fixture_unprepared(cx);
    cx.update(|cx| facts(1, cx));
    owner.update(cx, |owner, cx| {
        assert!(owner.open_surface(cx));
        let surface = owner.surface.as_ref().expect("surface");
        runtime::set_test_window_handle(surface.window, surface.hwnd, cx);
        owner.start(cx);
    });
    cx.run_until_parked();
    (owner, endpoint, directory)
}
fn next_material(endpoint: &PlatformServerEndpoint) -> (Uuid, ApplyWindowMaterialCommand) {
    let envelope = next(endpoint);
    let Message::PlatformCommand(PlatformCommand::ApplyWindowMaterial(command)) = envelope.message
    else {
        panic!("current transparent material must precede native preparation and reveal");
    };
    assert_eq!(command.window_handle, -40);
    assert_eq!(command.surface, MaterialSurface::Transparent);
    (envelope.id, command)
}
fn accept(
    endpoint: &PlatformServerEndpoint,
    id: Uuid,
    command: ApplyWindowMaterialCommand,
    cx: &mut TestAppContext,
) {
    let result = PlatformCommandResult {
        success: true,
        error: None,
        native_error_code: None,
        data: Some(serde_json::to_string(&applied(command)).expect("typed receipt")),
    };
    endpoint
        .route_command_result_for_contract_test(id, result.clone())
        .expect("router terminal");
    cx.update(|cx| {
        runtime::handle_notification(
            &PlatformNotification::CommandResult {
                correlation_id: id,
                result,
            },
            cx,
        );
    });
    cx.run_until_parked();
}

#[gpui_kit::test]
fn material_assistant_transparent_receipt_precedes_original_prepare_frame_and_show(
    cx: &mut TestAppContext,
) {
    let (owner, endpoint, _directory) = begin(cx);
    let (id, command) = next_material(&endpoint);
    let original = owner.read_with(cx, |owner, _| {
        owner.lifecycle.pending.expect("original operation")
    });
    assert_eq!(original.1, Phase::Prepare);
    assert!(endpoint.try_receive_command_for_contract_test().is_err());
    accept(&endpoint, id, command, cx);
    let prepare = next(&endpoint);
    assert_eq!(
        prepare.id, original.0,
        "material wait retains original operation identity"
    );
    assert_eq!(operation(&prepare), FloatingAssistantOperation::Prepare);
    reply(&owner, prepare.id, true, cx);
    let show = next(&endpoint);
    assert_eq!(operation(&show), FloatingAssistantOperation::Show);
    assert!(!owner.read_with(cx, |owner, _| owner.lifecycle.visible));
    reply(&owner, show.id, true, cx);
    assert!(owner.read_with(cx, |owner, _| owner.lifecycle.visible));
}

#[gpui_kit::test]
fn material_assistant_disable_during_wait_rejects_late_receipt(cx: &mut TestAppContext) {
    let (owner, endpoint, _directory) = begin(cx);
    let (id, command) = next_material(&endpoint);
    owner.update(cx, |owner, cx| {
        let mut settings = owner.settings.clone();
        settings.enabled = false;
        owner.change_settings(settings, cx);
    });
    accept(&endpoint, id, command, cx);
    assert!(endpoint.try_receive_command_for_contract_test().is_err());
    assert!(!owner.read_with(cx, |owner, _| owner.lifecycle.visible));
    assert!(owner.read_with(cx, |owner, _| owner.lifecycle.pending.is_none()));
}

#[gpui_kit::test]
fn material_assistant_wait_uses_original_five_second_deadline(cx: &mut TestAppContext) {
    let (owner, endpoint, _directory) = begin(cx);
    let (id, command) = next_material(&endpoint);
    cx.executor().advance_clock(Duration::from_secs(5));
    cx.run_until_parked();
    assert!(owner.read_with(cx, |owner, _| owner.error.is_some()));
    while let Ok(envelope) = endpoint.try_receive_command_for_contract_test() {
        assert_eq!(operation(&envelope), FloatingAssistantOperation::Hide);
    }
    accept(&endpoint, id, command, cx);
    assert!(endpoint.try_receive_command_for_contract_test().is_err());
    assert!(!owner.read_with(cx, |owner, _| owner.lifecycle.visible));
}

#[gpui_kit::test]
fn material_assistant_update_wait_does_not_block_hide_or_revive(cx: &mut TestAppContext) {
    let (owner, endpoint, _directory) = fixture_unprepared(cx);
    prepare_fixture(&owner, cx);
    owner.update(cx, |owner, cx| owner.start(cx));
    let prepare = next(&endpoint);
    reply(&owner, prepare.id, true, cx);
    let show = next(&endpoint);
    reply(&owner, show.id, true, cx);
    cx.update(|cx| facts(2, cx));
    owner.update(cx, |owner, cx| {
        owner.dirty = true;
        owner.sync(cx);
    });
    cx.run_until_parked();
    let (id, command) = next_material(&endpoint);
    owner.update(cx, |owner, cx| {
        let mut settings = owner.settings.clone();
        settings.enabled = false;
        owner.change_settings(settings, cx);
    });
    let hide = next(&endpoint);
    assert_eq!(operation(&hide), FloatingAssistantOperation::Hide);
    reply(&owner, hide.id, true, cx);
    accept(&endpoint, id, command, cx);
    assert!(endpoint.try_receive_command_for_contract_test().is_err());
    assert!(!owner.read_with(cx, |owner, _| owner.lifecycle.visible));
}

#[gpui_kit::test]
fn material_assistant_retry_keeps_retire_unblocked_and_rejects_old_completion(
    cx: &mut TestAppContext,
) {
    let (owner, endpoint, _directory) = begin(cx);
    let (id, command) = next_material(&endpoint);
    owner.update(cx, |owner, cx| owner.retry(cx));
    let retire = next(&endpoint);
    assert_eq!(operation(&retire), FloatingAssistantOperation::Retire);
    accept(&endpoint, id, command, cx);
    assert_eq!(
        owner.read_with(cx, |owner, _| owner.lifecycle.pending),
        Some((retire.id, Phase::Retire))
    );
    assert!(endpoint.try_receive_command_for_contract_test().is_err());
}
