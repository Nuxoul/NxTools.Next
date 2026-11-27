use super::*;
use gpui_kit::{AppContext, TestAppContext};
use nxtools_protocol::window_material::{
    AppliedWindowMaterial, EffectiveWindowMaterial, MaterialFallbackReason,
};

#[gpui_kit::test]
fn material_tool_settings_waits_through_reopen_and_keeps_original_window(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.executor().allow_parking();
    let (bridge, endpoint) = nxtools_ipc::platform_bridge(16);
    endpoint.set_connected_for_contract_test(true);
    let bridge = std::rc::Rc::new(bridge);
    let platform = bridge.clone();
    let notifications =
        cx.update(|cx| cx.new(|_| crate::notifications::NotificationCenter::default()));
    let owner = cx
        .add_window_view(move |window, cx| {
            MainWindow::new(
                gpui_kit::WeakEntity::new_invalid(),
                gpui_kit::WeakEntity::new_invalid(),
                gpui_kit::WeakEntity::new_invalid(),
                super::super::MainWindowShellContext {
                    settings: None,
                    platform: Some(platform),
                    system_settings: std::rc::Rc::new(std::cell::RefCell::new(
                        nxtools_core::SystemSettings::default(),
                    )),
                },
                notifications,
                window,
                cx,
            )
        })
        .0;
    let handle = cx.update(|cx| {
        owner.update(cx, |owner, cx| {
            open_tool_settings_window(owner, currency_settings::CARD, cx);
            let handle = owner.tool_settings_windows[currency_settings::CARD.id];
            crate::ui::material::runtime::set_test_window_handle(handle, 6262, cx);
            handle
        })
    });
    cx.update(|cx| {
        let mut facts = crate::ui::material::UiMaterialCapabilities::default();
        facts.connected();
        assert!(facts.apply(
            nxtools_protocol::window_material::SystemMaterialCapabilitiesChanged {
                correlation_id: uuid::Uuid::new_v4(),
                session_id: uuid::Uuid::new_v4(),
                revision: 1,
                supported: true,
                transparency_enabled: true,
                high_contrast: false,
                energy_saver: false,
                remote_session: false,
                native_error_code: 0,
                system_backdrop: true,
                rounded_corners: true,
            }
        ));
        cx.set_global(facts);
    });
    cx.run_until_parked();
    let request = endpoint
        .try_receive_command_for_contract_test()
        .expect("material request");
    let nxtools_protocol::Message::PlatformCommand(
        nxtools_protocol::PlatformCommand::ApplyWindowMaterial(command),
    ) = request.message
    else {
        panic!("material only");
    };
    assert_ne!(cx.read(|cx| cx.active_window()), Some(handle));
    cx.update(|cx| {
        owner.update(cx, |owner, cx| {
            open_tool_settings_window(owner, currency_settings::CARD, cx)
        })
    });
    cx.run_until_parked();
    assert_ne!(cx.read(|cx| cx.active_window()), Some(handle));
    assert!(endpoint.try_receive_command_for_contract_test().is_err());
    let result = nxtools_protocol::PlatformCommandResult {
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
    };
    endpoint
        .route_command_result_for_contract_test(request.id, result.clone())
        .expect("IPC terminal");
    cx.update(|cx| {
        assert!(crate::ui::material::runtime::handle_notification(
            &nxtools_ipc::PlatformNotification::CommandResult {
                correlation_id: request.id,
                result
            },
            cx
        ))
    });
    cx.run_until_parked();
    assert_eq!(cx.read(|cx| cx.active_window()), Some(handle));
    assert_eq!(
        cx.read(|cx| owner.read(cx).tool_settings_windows[currency_settings::CARD.id]),
        handle
    );
    assert!(cx.read(|cx| crate::ui::material::runtime::receipt(handle, cx).is_some()));
}
