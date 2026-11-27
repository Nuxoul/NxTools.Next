use super::*;
use crate::ui::material::UiMaterialCapabilities;
use gpui_kit::TestAppContext;
use nxtools_ipc::PlatformServerEndpoint;
use nxtools_protocol::Message;
use nxtools_protocol::window_material::{
    AppliedWindowMaterial, ApplyWindowMaterialCommand, EffectiveWindowMaterial,
    MaterialFallbackReason, SystemMaterialCapabilitiesChanged,
};

fn payload() -> QuickLaunchPreviewPayload {
    QuickLaunchPreviewPayload {
        entry_id: "synthetic".into(),
        name: "fixture".into(),
        target: "fixture.txt".into(),
        detail: "fixture".into(),
        created_at: None,
        modified_at: None,
        body: QuickLaunchPreviewBody::Metadata {
            icon: MaterialIcon::Info,
        },
    }
}

fn fixture(
    cx: &mut TestAppContext,
) -> (
    tempfile::TempDir,
    Entity<QuickLaunchApplication>,
    PlatformServerEndpoint,
) {
    cx.update(|cx| {
        if cx.try_global::<gpui_kit::component::Theme>().is_none() {
            gpui_kit::init(cx);
        }
        cx.set_global(UiMaterialCapabilities::default());
    });
    cx.executor().allow_parking();
    let temp = tempfile::tempdir().expect("isolated preview");
    let catalog =
        FinderCatalog::open(temp.path().join("history.db"), &[]).expect("fixture catalog");
    let (platform, endpoint) = nxtools_ipc::platform_bridge(64);
    endpoint.set_connected_for_contract_test(true);
    let (indexer, _endpoint) = crate::runtime::indexer_bridge(8);
    let notifications = cx.new(|_| NotificationCenter::default());
    let app = cx.new(|_| {
        QuickLaunchApplication::new(
            catalog,
            Rc::new(platform),
            Rc::new(indexer),
            AppConfig::default(),
            temp.path().join("config.json"),
            temp.path().join("logs"),
            notifications,
        )
    });
    let owner = app.downgrade();
    let window = cx.add_window(move |_, _| QuickLaunchPreview::new(payload(), owner));
    let view = window.root(cx).expect("preview root").downgrade();
    app.update(cx, |app, _| {
        app.preview.window = Some(QuickLaunchPreviewWindow {
            identity: QuickLaunchPreviewWindowIdentity {
                gpui: window.into(),
                native: 4242,
            },
            view,
        });
        app.preview.generation = 1;
        app.preview.hovered_entry_id = Some("synthetic".into());
    });
    (temp, app, endpoint)
}
fn facts(cx: &mut App, revision: u64) {
    let mut state = cx
        .try_global::<UiMaterialCapabilities>()
        .copied()
        .unwrap_or_default();
    let session_id = state
        .snapshot()
        .map_or_else(uuid::Uuid::new_v4, |facts| facts.session_id);
    state.connected();
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
fn stage(app: &QuickLaunchApplication) -> QuickLaunchPreviewPendingStage {
    QuickLaunchPreviewPendingStage {
        command_id: uuid::Uuid::nil(),
        phase: QuickLaunchPreviewStagePhase::MoveOffscreen,
        window_identity: app.preview.window.as_ref().expect("preview").identity,
        generation: app.preview.generation,
        entry_id: "synthetic".into(),
        payload: Some(payload()),
        preview_size: size(px(400.), px(410.)),
        bounds: Bounds::new(point(px(200.), px(120.)), size(px(400.), px(410.))),
        panel_scale_factor: 1.,
    }
}
fn next(endpoint: &PlatformServerEndpoint) -> (uuid::Uuid, PlatformCommand) {
    let envelope = endpoint
        .try_receive_command_for_contract_test()
        .expect("owner command");
    let Message::PlatformCommand(command) = envelope.message else {
        panic!("platform command")
    };
    (envelope.id, command)
}
fn receipt(command: ApplyWindowMaterialCommand) -> PlatformCommandResult {
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
            .expect("synthetic receipt"),
        ),
    }
}
fn deliver(
    app: &Entity<QuickLaunchApplication>,
    endpoint: &PlatformServerEndpoint,
    id: uuid::Uuid,
    result: PlatformCommandResult,
    cx: &mut TestAppContext,
) {
    endpoint
        .route_command_result_for_contract_test(id, result)
        .expect("native terminal");
    app.update(cx, |app, cx| {
        let notification = app
            .platform
            .notification_receiver()
            .try_recv()
            .expect("original result");
        app.handle_platform_notification(notification, cx);
    });
}

#[gpui_kit::test]
fn preview_material_offscreen_show_requires_current_exact_receipt(cx: &mut TestAppContext) {
    let (_temp, app, endpoint) = fixture(cx);
    app.update(cx, |app, cx| {
        app.begin_preview_material_presentation(cx);
        let stage = stage(app);
        app.send_preview_stage(
            stage,
            PlatformCommand::SetWindowPosition(SetWindowPositionCommand {
                window_handle: 4242,
                x: QUICK_LAUNCH_PREVIEW_STAGING_COORDINATE,
                y: QUICK_LAUNCH_PREVIEW_STAGING_COORDINATE,
            }),
            cx,
        );
    });
    let (move_id, command) = next(&endpoint);
    assert!(matches!(command, PlatformCommand::SetWindowPosition(_)));
    deliver(&app, &endpoint, move_id, native_ok(), cx);
    assert!(
        endpoint.try_receive_command_for_contract_test().is_err(),
        "no facts cannot show"
    );
    cx.update(|cx| facts(cx, 1));
    cx.run_until_parked();
    let (old_id, command) = next(&endpoint);
    let PlatformCommand::ApplyWindowMaterial(old) = command else {
        panic!("material before show")
    };
    assert_eq!(old.window_handle, 4242);
    assert_eq!(old.surface, MaterialSurface::Transient);
    cx.update(|cx| facts(cx, 2));
    cx.run_until_parked();
    deliver(&app, &endpoint, old_id, receipt(old), cx);
    let (id, command) = next(&endpoint);
    let PlatformCommand::ApplyWindowMaterial(current) = command else {
        panic!("latest material")
    };
    assert_eq!(current.facts_revision, 2);
    assert!(endpoint.try_receive_command_for_contract_test().is_err());
    deliver(&app, &endpoint, id, receipt(current), cx);
    let (_, command) = next(&endpoint);
    assert!(matches!(
        command,
        PlatformCommand::SetWindowVisibility(SetWindowVisibilityCommand {
            window_handle: 4242,
            visible: true
        })
    ));
    app.read_with(cx, |app, _| {
        assert_eq!(
            app.preview.pending_stage.as_ref().expect("show ACK").phase,
            QuickLaunchPreviewStagePhase::ShowOffscreen
        )
    });
}

#[gpui_kit::test]
fn preview_material_retired_continuations_cannot_show(cx: &mut TestAppContext) {
    for outcome in ["dismiss", "supersede", "disconnect", "timeout"] {
        let (_temp, app, endpoint) = fixture(cx);
        cx.update(|cx| {
            cx.set_global(UiMaterialCapabilities::default());
            facts(cx, 1);
        });
        app.update(cx, |app, cx| {
            app.begin_preview_material_presentation(cx);
            let stage = stage(app);
            app.wait_for_preview_material(stage, PreviewMaterialAction::InitialShow, cx);
        });
        let (id, command) = next(&endpoint);
        let PlatformCommand::ApplyWindowMaterial(command) = command else {
            panic!("material")
        };
        app.update(cx, |app, cx| match outcome {
            "dismiss" => app.dismiss_quick_launch_preview(cx),
            "supersede" => {
                app.preview.generation += 1;
                app.preview.hovered_entry_id = Some("replacement".into());
            }
            "disconnect" => app.handle_platform_notification(
                PlatformNotification::Disconnected {
                    generation: 1,
                    pending_command_ids: vec![],
                },
                cx,
            ),
            _ => {}
        });
        if outcome == "timeout" {
            cx.executor().advance_clock(PREVIEW_MATERIAL_TIMEOUT);
            cx.run_until_parked();
        }
        deliver(&app, &endpoint, id, receipt(command), cx);
        while let Ok(envelope) = endpoint.try_receive_command_for_contract_test() {
            assert!(
                !matches!(
                    envelope.message,
                    Message::PlatformCommand(
                        PlatformCommand::SetWindowVisibility(SetWindowVisibilityCommand {
                            visible: true,
                            ..
                        }) | PlatformCommand::ActivateWindow(_)
                            | PlatformCommand::SetWindowPosition(_)
                    )
                ),
                "retired preview must not reveal or activate: {outcome}"
            );
        }
        app.read_with(cx, |app, _| assert!(!app.preview.visible));
    }
}

fn native_ok() -> PlatformCommandResult {
    PlatformCommandResult {
        success: true,
        error: None,
        native_error_code: None,
        data: None,
    }
}

#[gpui_kit::test]
fn preview_material_initial_noactivate_bounds_frame_show_chain(cx: &mut TestAppContext) {
    let (_temp, app, endpoint) = fixture(cx);
    cx.update(|cx| facts(cx, 1));
    app.update(cx, |app, cx| {
        app.begin_preview_material_presentation(cx);
        let mut stage = stage(app);
        stage.phase = QuickLaunchPreviewStagePhase::InitialNoActivate;
        stage.payload = None;
        app.send_preview_stage(
            stage,
            PlatformCommand::SetWindowNoActivate(SetWindowNoActivateCommand {
                window_handle: 4242,
                enabled: true,
            }),
            cx,
        );
    });
    let (id, command) = next(&endpoint);
    assert!(matches!(
        command,
        PlatformCommand::SetWindowNoActivate(SetWindowNoActivateCommand { enabled: true, .. })
    ));
    deliver(&app, &endpoint, id, native_ok(), cx);
    let (id, command) = next(&endpoint);
    assert!(matches!(
        command,
        PlatformCommand::SetWindowBounds(SetWindowBoundsCommand {
            window_handle: 4242,
            x: 200,
            y: 120,
            width: 400,
            height: 410,
            ..
        })
    ));
    deliver(&app, &endpoint, id, native_ok(), cx);
    let (id, command) = next(&endpoint);
    let PlatformCommand::ApplyWindowMaterial(command) = command else {
        panic!("bounds alone cannot reveal")
    };
    deliver(&app, &endpoint, id, receipt(command), cx);
    assert!(
        endpoint.try_receive_command_for_contract_test().is_err(),
        "material alone cannot skip the frame"
    );
    cx.run_until_parked();
    let window = app.read_with(cx, |app, _| {
        app.preview
            .window
            .as_ref()
            .expect("retained preview")
            .identity
            .gpui
    });
    let mut surface = gpui_kit::VisualTestContext::from_window(window, cx);
    surface.refresh().expect("hidden preview frame");
    surface.update(|window, cx| {
        window.simulate_next_frame(cx);
    });
    surface.run_until_parked();
    let (id, command) = next(&endpoint);
    assert!(matches!(
        command,
        PlatformCommand::SetWindowVisibility(SetWindowVisibilityCommand { visible: true, .. })
    ));
    app.read_with(cx, |app, _| {
        assert!(!app.preview.visible, "show ACK still pending")
    });
    deliver(&app, &endpoint, id, native_ok(), cx);
    cx.run_until_parked();
    app.read_with(cx, |app, cx| {
        assert!(app.preview.visible);
        assert_eq!(app.preview.displayed_entry_id.as_deref(), Some("synthetic"));
        assert!(
            app.preview.material.receipt(cx).is_none(),
            "ownership transferred"
        );
        let window = app.preview.window.as_ref().expect("same preview");
        assert_eq!(
            runtime::receipt(window.identity.gpui, cx)
                .expect("runtime receipt")
                .window_handle,
            4242
        );
    });
}

#[gpui_kit::test]
fn preview_material_wrong_receipt_identity_never_reveals(cx: &mut TestAppContext) {
    for field in [
        "session",
        "facts",
        "hwnd",
        "generation",
        "style",
        "dark",
        "surface",
    ] {
        let (_temp, app, endpoint) = fixture(cx);
        cx.update(|cx| facts(cx, 1));
        app.update(cx, |app, cx| {
            app.begin_preview_material_presentation(cx);
            let stage = stage(app);
            app.wait_for_preview_material(stage, PreviewMaterialAction::InitialShow, cx);
        });
        let (id, command) = next(&endpoint);
        let PlatformCommand::ApplyWindowMaterial(mut command) = command else {
            panic!("material")
        };
        match field {
            "session" => command.session_id = uuid::Uuid::new_v4(),
            "facts" => command.facts_revision += 1,
            "hwnd" => command.window_handle += 1,
            "generation" => command.generation += 1,
            "style" => {
                command.style = nxtools_protocol::window_material::WindowMaterialStyle::Acrylic
            }
            "dark" => command.dark = !command.dark,
            "surface" => command.surface = MaterialSurface::Transparent,
            _ => unreachable!(),
        }
        deliver(&app, &endpoint, id, receipt(command), cx);
        while let Ok(envelope) = endpoint.try_receive_command_for_contract_test() {
            assert!(
                !matches!(
                    envelope.message,
                    Message::PlatformCommand(
                        PlatformCommand::SetWindowVisibility(SetWindowVisibilityCommand {
                            visible: true,
                            ..
                        }) | PlatformCommand::ActivateWindow(_)
                            | PlatformCommand::SetWindowPosition(_)
                    )
                ),
                "wrong {field} cannot reveal"
            );
        }
        app.read_with(cx, |app, _| assert!(!app.preview.visible));
    }
}

#[gpui_kit::test]
fn preview_material_runtime_updates_only_material_and_reuse_reclaims_once(cx: &mut TestAppContext) {
    let (_temp, app, endpoint) = fixture(cx);
    cx.update(|cx| facts(cx, 1));
    app.update(cx, |app, cx| {
        app.begin_preview_material_presentation(cx);
        let stage = stage(app);
        app.wait_for_preview_material(stage, PreviewMaterialAction::InitialShow, cx);
    });
    let (id, command) = next(&endpoint);
    let PlatformCommand::ApplyWindowMaterial(command) = command else {
        panic!("material")
    };
    deliver(&app, &endpoint, id, receipt(command), cx);
    let (id, _) = next(&endpoint);
    deliver(&app, &endpoint, id, native_ok(), cx);
    cx.run_until_parked();
    let (_, command) = next(&endpoint);
    assert!(matches!(command, PlatformCommand::SetWindowTopmost(_)));
    assert!(endpoint.try_receive_command_for_contract_test().is_err());
    let identity = app.read_with(cx, |app, _| {
        app.preview.window.as_ref().expect("preview").identity
    });
    cx.update(|cx| facts(cx, 2));
    cx.run_until_parked();
    let (id, command) = next(&endpoint);
    let PlatformCommand::ApplyWindowMaterial(command) = command else {
        panic!("runtime must not move/show")
    };
    assert_eq!(command.window_handle, 4242);
    assert_eq!(command.facts_revision, 2);
    assert!(endpoint.try_receive_command_for_contract_test().is_err());
    app.update(cx, |app, cx| {
        app.preview.generation += 1;
        app.begin_preview_material_presentation(cx);
        assert!(
            runtime::take_preparation(identity.gpui, cx).is_none(),
            "exact runtime record already reclaimed"
        );
        let stage = stage(app);
        app.wait_for_preview_material(stage, PreviewMaterialAction::RevealOnscreen, cx);
    });
    assert!(
        endpoint.try_receive_command_for_contract_test().is_err(),
        "reclaim preserves in-flight request"
    );
    deliver(&app, &endpoint, id, receipt(command), cx);
    let (_, command) = next(&endpoint);
    assert!(matches!(
        command,
        PlatformCommand::SetWindowPosition(SetWindowPositionCommand {
            window_handle: 4242,
            x: 200,
            y: 120
        })
    ));
    app.read_with(cx, |app, _| {
        assert_eq!(
            app.preview.window.as_ref().expect("retained").identity.gpui,
            identity.gpui
        )
    });
}

#[gpui_kit::test]
fn preview_material_timeout_retires_inflight_show_and_late_ack(cx: &mut TestAppContext) {
    let (_temp, app, endpoint) = fixture(cx);
    cx.update(|cx| facts(cx, 1));
    app.update(cx, |app, cx| {
        app.begin_preview_material_presentation(cx);
        let stage = stage(app);
        app.wait_for_preview_material(stage, PreviewMaterialAction::InitialShow, cx);
    });
    let (id, command) = next(&endpoint);
    let PlatformCommand::ApplyWindowMaterial(command) = command else {
        panic!("material")
    };
    deliver(&app, &endpoint, id, receipt(command), cx);
    let (show_id, command) = next(&endpoint);
    assert!(matches!(
        command,
        PlatformCommand::SetWindowVisibility(SetWindowVisibilityCommand { visible: true, .. })
    ));
    cx.run_until_parked();
    cx.executor().advance_clock(PREVIEW_MATERIAL_TIMEOUT);
    cx.run_until_parked();
    let (_, command) = next(&endpoint);
    assert!(
        matches!(
            command,
            PlatformCommand::SetWindowVisibility(SetWindowVisibilityCommand {
                window_handle: 4242,
                visible: false,
            })
        ),
        "timeout must undo a show whose ACK has not arrived"
    );
    deliver(&app, &endpoint, show_id, native_ok(), cx);
    cx.run_until_parked();
    assert!(
        endpoint.try_receive_command_for_contract_test().is_err(),
        "late ACK cannot topmost or activate"
    );
    app.read_with(cx, |app, cx| {
        assert!(!app.preview.visible);
        assert!(app.preview.material_lifecycle.is_none());
        assert!(
            runtime::receipt(
                app.preview
                    .window
                    .as_ref()
                    .expect("same HWND")
                    .identity
                    .gpui,
                cx
            )
            .is_none()
        );
    });
}
