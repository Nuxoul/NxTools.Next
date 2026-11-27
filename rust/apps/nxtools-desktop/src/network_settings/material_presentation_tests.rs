use super::*;
use gpui_kit::TestAppContext;
use nxtools_protocol::window_material::{
    AppliedWindowMaterial, ApplyWindowMaterialCommand, EffectiveWindowMaterial,
    MaterialFallbackReason, SystemMaterialCapabilitiesChanged,
};

fn fixture(
    cx: &mut TestAppContext,
) -> (
    Entity<NetworkSettingsApplication>,
    Rc<PlatformBridge>,
    nxtools_ipc::PlatformServerEndpoint,
    PathBuf,
) {
    let (application, _, _, directory) = super::kit_migration_tests::fixture(cx);
    let (bridge, endpoint) = nxtools_ipc::platform_bridge(16);
    endpoint.set_connected_for_contract_test(true);
    let bridge = Rc::new(bridge);
    cx.update(|cx| {
        application.update(cx, |application, cx| {
            application.close_window(cx);
            application.platform = Some(bridge.clone());
            application.open_window(cx);
            crate::ui::material::runtime::set_test_window_handle(
                application.window.expect("hidden settings"),
                5252,
                cx,
            );
        });
        let mut facts = crate::ui::material::UiMaterialCapabilities::default();
        facts.connected();
        assert!(facts.apply(SystemMaterialCapabilitiesChanged {
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
        }));
        cx.set_global(facts);
    });
    cx.run_until_parked();
    (application, bridge, endpoint, directory)
}

fn next(
    endpoint: &nxtools_ipc::PlatformServerEndpoint,
) -> (uuid::Uuid, ApplyWindowMaterialCommand) {
    let request = endpoint
        .try_receive_command_for_contract_test()
        .expect("material request");
    let nxtools_protocol::Message::PlatformCommand(PlatformCommand::ApplyWindowMaterial(command)) =
        request.message
    else {
        panic!("material readiness must not replace settings presentation");
    };
    (request.id, command)
}
fn terminal(
    cx: &mut TestAppContext,
    endpoint: &nxtools_ipc::PlatformServerEndpoint,
    id: uuid::Uuid,
    command: ApplyWindowMaterialCommand,
    success: bool,
) {
    let result = nxtools_protocol::PlatformCommandResult {
        success,
        error: (!success).then(|| "window_material_failed".into()),
        native_error_code: None,
        data: success.then(|| {
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
            .expect("synthetic receipt")
        }),
    };
    endpoint
        .route_command_result_for_contract_test(id, result.clone())
        .expect("IPC terminal");
    cx.update(|cx| {
        crate::ui::material::runtime::handle_notification(
            &nxtools_ipc::PlatformNotification::CommandResult {
                correlation_id: id,
                result,
            },
            cx,
        );
    });
    cx.run_until_parked();
}

#[gpui_kit::test]
fn material_global_settings_reopen_waits_without_replacing_draft_or_native_request(
    cx: &mut TestAppContext,
) {
    let (application, _, endpoint, directory) = fixture(cx);
    let (id, command) = next(&endpoint);
    let (handle, draft) = cx.read(|cx| {
        let app = application.read(cx);
        (
            app.window.expect("window"),
            app.settings_view.clone().expect("draft"),
        )
    });
    assert_ne!(cx.read(|cx| cx.active_window()), Some(handle));
    cx.update(|cx| application.update(cx, |app, cx| app.open_window(cx)));
    cx.run_until_parked();
    assert_ne!(cx.read(|cx| cx.active_window()), Some(handle));
    assert!(endpoint.try_receive_command_for_contract_test().is_err());
    assert_eq!(
        cx.read(|cx| application.read(cx).settings_view.clone()),
        Some(draft)
    );
    terminal(cx, &endpoint, id, command, true);
    assert_eq!(cx.read(|cx| cx.active_window()), Some(handle));
    cx.update(|cx| application.update(cx, |app, cx| app.close_window(cx)));
    std::fs::remove_dir_all(directory).expect("synthetic config cleanup");
}

#[gpui_kit::test]
fn material_global_settings_initial_failure_after_reopen_removes_hidden_window(
    cx: &mut TestAppContext,
) {
    let (application, _, endpoint, directory) = fixture(cx);
    let (id, command) = next(&endpoint);
    let handle = cx.read(|cx| application.read(cx).window.expect("hidden window"));
    cx.update(|cx| application.update(cx, |app, cx| app.open_window(cx)));
    cx.run_until_parked();
    terminal(cx, &endpoint, id, command, false);
    assert!(!cx.read(|cx| cx.windows().contains(&handle)));
    assert!(cx.read(|cx| application.read(cx).window.is_none()));
    assert!(cx.read(|cx| application.read(cx).settings_view.is_none()));
    std::fs::remove_dir_all(directory).expect("synthetic config cleanup");
}

#[gpui_kit::test]
fn material_global_settings_reapply_failure_preserves_presented_draft(cx: &mut TestAppContext) {
    let (application, _, endpoint, directory) = fixture(cx);
    let (id, command) = next(&endpoint);
    terminal(cx, &endpoint, id, command, true);
    let (handle, draft) = cx.read(|cx| {
        let app = application.read(cx);
        (
            app.window.expect("window"),
            app.settings_view.clone().expect("draft"),
        )
    });
    cx.update(|cx| {
        let mut facts = *cx.global::<crate::ui::material::UiMaterialCapabilities>();
        let mut snapshot = facts.snapshot().expect("facts");
        snapshot.revision += 1;
        snapshot.transparency_enabled = false;
        assert!(facts.apply(snapshot));
        cx.set_global(facts);
        application.update(cx, |app, cx| app.open_window(cx));
    });
    cx.run_until_parked();
    let (id, command) = next(&endpoint);
    terminal(cx, &endpoint, id, command, false);
    assert!(cx.read(|cx| cx.windows().contains(&handle)));
    assert_eq!(
        cx.read(|cx| application.read(cx).settings_view.clone()),
        Some(draft)
    );
    cx.update(|cx| application.update(cx, |app, cx| app.close_window(cx)));
    std::fs::remove_dir_all(directory).expect("synthetic config cleanup");
}
