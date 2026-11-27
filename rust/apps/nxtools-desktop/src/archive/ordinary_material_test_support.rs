//! Synthetic material receipts for Archive, Reader and raw-join owner tests only.
use gpui_kit::App;
use nxtools_ipc::{PlatformNotification, PlatformServerEndpoint};
use nxtools_protocol::{Message, PlatformCommand, PlatformCommandResult, window_material::*};
use uuid::Uuid;

pub(crate) fn facts(session: Uuid, revision: u64, cx: &mut App) {
    let mut state = cx
        .try_global::<crate::ui::material::UiMaterialCapabilities>()
        .copied()
        .unwrap_or_default();
    state.connected();
    assert!(state.apply(SystemMaterialCapabilitiesChanged {
        correlation_id: Uuid::new_v4(),
        session_id: session,
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

pub(crate) fn next(endpoint: &PlatformServerEndpoint) -> (Uuid, ApplyWindowMaterialCommand) {
    while let Ok(envelope) = endpoint.try_receive_command_for_contract_test() {
        if let Message::PlatformCommand(PlatformCommand::ApplyWindowMaterial(command)) =
            envelope.message
        {
            assert_eq!(command.surface, MaterialSurface::Backdrop);
            return (envelope.id, command);
        }
    }
    panic!("original router must carry the material request");
}

pub(crate) fn accept(
    endpoint: &PlatformServerEndpoint,
    id: Uuid,
    command: ApplyWindowMaterialCommand,
    cx: &mut App,
) {
    let applied = AppliedWindowMaterial {
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
    };
    let result = PlatformCommandResult {
        success: true,
        error: None,
        native_error_code: None,
        data: Some(serde_json::to_string(&applied).expect("typed receipt")),
    };
    endpoint
        .route_command_result_for_contract_test(id, result.clone())
        .expect("original router terminal");
    assert!(crate::ui::material::runtime::handle_notification(
        &PlatformNotification::CommandResult {
            correlation_id: id,
            result
        },
        cx
    ));
}
