//! First-show gates through the production owners and original material router.
use super::*;
use gpui_kit::TestAppContext;

fn fixture(
    cx: &mut TestAppContext,
) -> (
    Entity<DownloadApplication>,
    PathBuf,
    nxtools_ipc::PlatformServerEndpoint,
) {
    let directory =
        std::env::temp_dir().join(format!("download-material-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&directory).expect("temporary download material directory");
    let engine = DownloadEngine::open_in_memory().expect("in-memory download engine");
    let (platform, endpoint) = nxtools_ipc::platform_bridge(16);
    let notifications = cx.update(|cx| cx.new(|_| NotificationCenter::default()));
    let app = cx.update(|cx| {
        cx.new(|cx| {
            DownloadApplication::new(
                engine,
                Rc::new(platform),
                notifications,
                DownloadRuntimeContext {
                    fallback_default_directory: directory.clone(),
                    config: AppConfig::default(),
                    config_path: directory.join("config.json"),
                    log_directory: directory.clone(),
                    database_path: directory.join("downloads.sqlite3"),
                    ffmpeg_install_directory: directory.join("ffmpeg"),
                },
                cx,
            )
        })
    });
    (app, directory, endpoint)
}

#[gpui_kit::test]
fn material_missing_hwnd_does_not_leave_ordinary_download_windows_ready(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (app, directory, _endpoint) = fixture(cx);
    app.update(cx, |app, cx| {
        app.open_settings_window(cx);
        app.open_schedule_window(cx);
        app.open_new_download_window(cx);
    });
    cx.run_until_parked();
    app.read_with(cx, |app, _| {
        assert!(
            app.settings_window.is_none(),
            "unprepared settings must be retired"
        );
        assert!(
            app.schedule_window.is_none(),
            "unprepared schedule must be retired"
        );
        assert!(
            app.new_download_window.is_none(),
            "unprepared intake must be retired"
        );
    });
    std::fs::remove_dir_all(directory).expect("remove temporary download material directory");
}

#[gpui_kit::test]
fn material_missing_hwnd_releases_delete_confirmation_busy_owner(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (app, directory, _endpoint) = fixture(cx);
    let task_id = DownloadTaskId::new();
    app.update(cx, |app, cx| {
        app.open_delete_confirmation(task_id, "fixture".into(), None, cx)
    });
    cx.run_until_parked();
    app.read_with(cx, |app, _| {
        assert!(!app.delete_confirmation_windows.contains_key(&task_id));
        assert!(app.pending_batch_confirmation.is_none());
    });
    std::fs::remove_dir_all(directory).expect("remove temporary download material directory");
}

#[gpui_kit::test]
fn material_missing_hwnd_retires_exact_task_registration(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (app, directory, _endpoint) = fixture(cx);
    let task_id = DownloadTaskId::new();
    app.update(cx, |app, cx| {
        app.open_task_window(task_id, DownloadWindowPhase::Progress, None, cx)
    });
    cx.run_until_parked();
    app.read_with(cx, |app, _| {
        assert!(!app.task_windows.contains_key(&task_id))
    });
    std::fs::remove_dir_all(directory).expect("remove temporary download material directory");
}

fn connect_material(endpoint: &nxtools_ipc::PlatformServerEndpoint, cx: &mut TestAppContext) {
    endpoint.set_connected_for_contract_test(true);
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
}

fn next_material(
    endpoint: &nxtools_ipc::PlatformServerEndpoint,
) -> (
    uuid::Uuid,
    nxtools_protocol::window_material::ApplyWindowMaterialCommand,
) {
    loop {
        let envelope = endpoint
            .try_receive_command_for_contract_test()
            .expect("material request before reveal");
        match envelope.message {
            nxtools_protocol::Message::PlatformCommand(PlatformCommand::ApplyWindowMaterial(
                command,
            )) => return (envelope.id, command),
            nxtools_protocol::Message::PlatformCommand(PlatformCommand::ReadClipboardText) => {}
            _ => panic!("native presentation must not precede material admission"),
        }
    }
}

fn accept_material(
    app: &Entity<DownloadApplication>,
    endpoint: &nxtools_ipc::PlatformServerEndpoint,
    id: uuid::Uuid,
    command: nxtools_protocol::window_material::ApplyWindowMaterialCommand,
    cx: &mut TestAppContext,
) {
    use nxtools_protocol::window_material::*;
    let receipt = AppliedWindowMaterial {
        session_id: command.session_id,
        facts_revision: command.facts_revision,
        window_handle: command.window_handle,
        generation: command.generation,
        style: command.style,
        surface: command.surface,
        dark: command.dark,
        effective: if command.surface == MaterialSurface::Transparent {
            EffectiveWindowMaterial::Transparent
        } else if command.style == WindowMaterialStyle::Solid {
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
        data: Some(serde_json::to_string(&receipt).expect("material receipt")),
    };
    endpoint
        .route_command_result_for_contract_test(id, result)
        .expect("original pending result route");
    let bridge = app.read_with(cx, |app, _| Rc::clone(&app.platform));
    loop {
        let notification = bridge
            .notification_receiver()
            .try_recv()
            .expect("original material notification");
        if matches!(&notification, PlatformNotification::CommandResult { correlation_id, .. } if *correlation_id == id)
        {
            cx.update(|cx| {
                assert!(crate::ui::material::runtime::handle_notification(
                    &notification,
                    cx
                ))
            });
            break;
        }
    }
    cx.run_until_parked();
}

#[gpui_kit::test]
fn material_task_reveal_waits_for_qualified_receipt(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (app, directory, endpoint) = fixture(cx);
    connect_material(&endpoint, cx);
    let task_id = DownloadTaskId::new();
    app.update(cx, |app, cx| {
        app.task_window_native_handle_override = Some(701);
        app.open_task_window(task_id, DownloadWindowPhase::Progress, None, cx);
        let window = app
            .task_windows
            .get(&task_id)
            .expect("hidden task window")
            .window;
        crate::ui::material::runtime::set_test_window_handle(window, 701, cx);
    });
    cx.run_until_parked();
    let (id, command) = next_material(&endpoint);
    assert_eq!(command.window_handle, 701);
    assert!(endpoint.try_receive_command_for_contract_test().is_err());
    app.read_with(cx, |app, _| {
        assert!(!app.task_windows[&task_id].material_ready)
    });
    accept_material(&app, &endpoint, id, command, cx);
    app.read_with(cx, |app, _| {
        assert!(app.task_windows[&task_id].material_ready)
    });
    let placement = endpoint
        .try_receive_command_for_contract_test()
        .expect("original placement");
    assert!(matches!(
        placement.message,
        nxtools_protocol::Message::PlatformCommand(PlatformCommand::MoveWindowCenteredOverKey(_))
    ));
    let reveal = endpoint
        .try_receive_command_for_contract_test()
        .expect("original reveal");
    assert!(matches!(
        reveal.message,
        nxtools_protocol::Message::PlatformCommand(PlatformCommand::SetWindowVisibility(
            SetWindowVisibilityCommand {
                window_handle: 701,
                visible: true
            }
        ))
    ));
    std::fs::remove_dir_all(directory).expect("remove temporary download material directory");
}

#[gpui_kit::test]
fn material_task_close_reopen_rejects_old_receipt(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (app, directory, endpoint) = fixture(cx);
    connect_material(&endpoint, cx);
    let task_id = DownloadTaskId::new();
    let old = app.update(cx, |app, cx| {
        app.task_window_native_handle_override = Some(701);
        app.open_task_window(task_id, DownloadWindowPhase::Progress, None, cx);
        let window = app.task_windows[&task_id].window;
        crate::ui::material::runtime::set_test_window_handle(window, 701, cx);
        window
    });
    cx.run_until_parked();
    let (id, command) = next_material(&endpoint);
    let replacement = app.update(cx, |app, cx| {
        app.close_task_window(task_id, cx);
        app.task_window_native_handle_override = Some(702);
        app.open_task_window(task_id, DownloadWindowPhase::Progress, None, cx);
        let window = app.task_windows[&task_id].window;
        crate::ui::material::runtime::set_test_window_handle(window, 702, cx);
        window
    });
    assert_ne!(old, replacement);
    cx.run_until_parked();
    // A closed record is no longer owned by the material runtime.
    let result = PlatformCommandResult {
        success: true,
        error: None,
        native_error_code: None,
        data: None,
    };
    endpoint
        .route_command_result_for_contract_test(id, result.clone())
        .expect("retired in-flight result");
    cx.update(|cx| {
        assert!(!crate::ui::material::runtime::handle_notification(
            &PlatformNotification::CommandResult {
                correlation_id: id,
                result
            },
            cx
        ))
    });
    cx.run_until_parked();
    app.read_with(cx, |app, _| {
        assert_eq!(app.task_windows[&task_id].window, replacement);
        assert!(!app.task_windows[&task_id].material_ready);
    });
    assert_eq!(command.window_handle, 701);
    while let Ok(envelope) = endpoint.try_receive_command_for_contract_test() {
        assert!(!matches!(
            envelope.message,
            nxtools_protocol::Message::PlatformCommand(PlatformCommand::SetWindowVisibility(
                SetWindowVisibilityCommand { visible: true, .. }
            ))
        ));
    }
    std::fs::remove_dir_all(directory).expect("remove temporary download material directory");
}

fn staged_confirmation(
    cx: &mut TestAppContext,
) -> (
    Entity<DownloadApplication>,
    PathBuf,
    nxtools_ipc::PlatformServerEndpoint,
    AnyWindowHandle,
    uuid::Uuid,
    AnyWindowHandle,
) {
    let (app, directory, endpoint) = fixture(cx);
    connect_material(&endpoint, cx);
    let source = app.update(cx, |app, cx| {
        app.open_new_download_window(cx);
        let source = app.new_download_window.expect("intake");
        crate::ui::material::runtime::set_test_window_handle(source, 701, cx);
        source
    });
    cx.run_until_parked();
    let (id, command) = next_material(&endpoint);
    accept_material(&app, &endpoint, id, command, cx);
    while endpoint.try_receive_command_for_contract_test().is_ok() {}
    let view = app.read_with(cx, |app, _| {
        app.new_download_view.clone().expect("intake view")
    });
    source
        .update(cx, |_, window, cx| {
            view.update(cx, |view, cx| {
                view.url.update(cx, |input, cx| {
                    input.set_value("https://fixture.invalid/file.bin", window, cx)
                });
                view.file_name
                    .update(cx, |input, cx| input.set_value("fixture.bin", window, cx));
                view.confirmed_request = Some(view.request(cx).expect("staged request"));
                view.probe = Some(DownloadProbe {
                    resolution: None,
                    server_observation: None,
                    final_url: "https://fixture.invalid/file.bin".into(),
                    file_name: Some("fixture.bin".into()),
                    total_bytes: Some(8),
                    supports_ranges: false,
                    content_type: None,
                    validator: Default::default(),
                    media: None,
                });
                view.probe_generation = 7;
                view.handoff_staged = true;
                view.handoff_pending = true;
            })
        })
        .expect("retained source probe");
    let operation_id = uuid::Uuid::new_v4();
    let target = app.update(cx, |app, cx| {
        app.new_download_native_handles_override = Some((701, 702));
        app.create_fixed_confirmation(source, view.downgrade(), 7, operation_id, cx);
        let target = app
            .pending_new_download_handoff
            .as_ref()
            .expect("pending confirmation")
            .target;
        crate::ui::material::runtime::set_test_window_handle(target, 702, cx);
        target
    });
    (app, directory, endpoint, source, operation_id, target)
}

#[gpui_kit::test]
fn material_fixed_confirmation_gates_internal_native_show_before_original_ack(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_kit::init);
    let (app, directory, endpoint, source, operation_id, target) = staged_confirmation(cx);
    cx.run_until_parked();
    let (id, command) = next_material(&endpoint);
    app.read_with(cx, |app, _| {
        assert!(
            app.new_download_handoff_commands.is_empty(),
            "PresentFixedWindowOverAnchor internally shows its target"
        );
        assert_eq!(app.new_download_window, Some(source));
    });
    accept_material(&app, &endpoint, id, command, cx);
    app.read_with(cx, |app, _| {
        assert_eq!(
            app.new_download_window,
            Some(source),
            "material success is not native placement success"
        );
        assert!(matches!(
            app.new_download_handoff_commands.as_slice(),
            [PlatformCommand::PresentFixedWindowOverAnchor(_)]
        ));
    });
    app.update(cx, |app, cx| {
        app.finish_new_download_handoff(operation_id, true, "native_ack", cx)
    });
    app.read_with(cx, |app, cx| {
        assert_eq!(app.new_download_window, Some(target));
        assert!(!cx.windows().contains(&source));
    });
    std::fs::remove_dir_all(directory).expect("remove temporary download material directory");
}

#[gpui_kit::test]
fn material_fixed_confirmation_stale_receipt_cannot_commit_replacement(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (app, directory, endpoint, source, operation_id, target) = staged_confirmation(cx);
    cx.run_until_parked();
    let (id, command) = next_material(&endpoint);
    accept_material(&app, &endpoint, id, command, cx);
    cx.update(|cx| {
        let mut facts = *cx.global::<crate::ui::material::UiMaterialCapabilities>();
        let mut snapshot = facts.snapshot().expect("connected facts");
        snapshot.revision += 1;
        assert!(facts.apply(snapshot));
        cx.set_global(facts);
    });
    app.update(cx, |app, cx| {
        app.finish_new_download_handoff(operation_id, true, "native_ack", cx)
    });
    app.read_with(cx, |app, cx| {
        assert_eq!(app.new_download_window, Some(source));
        assert!(cx.windows().contains(&source));
        assert!(!cx.windows().contains(&target));
        assert!(app.pending_new_download_handoff.is_none());
    });
    std::fs::remove_dir_all(directory).expect("remove temporary download material directory");
}

#[gpui_kit::test]
fn material_missing_hwnd_cancels_shutdown_instead_of_running_unseen_countdown(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_kit::init);
    let (app, directory, _endpoint) = fixture(cx);
    app.update(cx, |app, cx| {
        let task_id = DownloadTaskId::new();
        app.completion_actions.set_shutdown_armed(task_id, true);
        let transition = app
            .completion_actions
            .evaluate_shutdown([(task_id, DownloadStatus::Completed)], Instant::now());
        app.apply_shutdown_transition(transition, cx);
    });
    cx.run_until_parked();
    app.read_with(cx, |app, _| {
        assert!(app.completion_actions.active_countdown().is_none());
        assert!(app.shutdown_window.is_none());
        assert!(app.pending_shutdown_command.is_none());
    });
    std::fs::remove_dir_all(directory).expect("remove temporary download material directory");
}

#[gpui_kit::test]
fn material_taskbar_uses_transparent_host_and_keeps_original_hidden_projection(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_kit::init);
    let (app, directory, endpoint) = fixture(cx);
    connect_material(&endpoint, cx);
    app.update(cx, |app, cx| {
        app.open_taskbar_overlay(cx);
        let window = app.taskbar_overlay_window.expect("sparse overlay");
        crate::ui::material::runtime::set_test_window_handle(window, 703, cx);
    });
    cx.run_until_parked();
    let (_, command) = next_material(&endpoint);
    assert_eq!(
        command.surface,
        nxtools_protocol::window_material::MaterialSurface::Transparent
    );
    app.read_with(cx, |app, _| {
        assert!(!app.taskbar_overlay_material_ready);
        assert!(!app.taskbar_overlay_model().is_visible());
    });
    assert!(endpoint.try_receive_command_for_contract_test().is_err());
    std::fs::remove_dir_all(directory).expect("remove temporary download material directory");
}
