use super::*;
use gpui_kit::{AppContext, TestAppContext};
use nxtools_core::{AppConfig, SystemSettings};
use nxtools_protocol::{Message, PlatformCommandResult, TrayActionRequest};
use std::cell::{Cell, RefCell};

fn fixture(
    cx: &mut TestAppContext,
) -> (
    Entity<PlatformEventDispatcher>,
    nxtools_ipc::PlatformServerEndpoint,
    Rc<Cell<usize>>,
    tempfile::TempDir,
) {
    cx.update(gpui_kit::init);
    cx.executor().allow_parking();
    let directory = tempfile::tempdir().expect("temporary startup fixture");
    let config = AppConfig::default();
    let path = directory.path().join("config.json");
    config.save_to(&path).expect("synthetic config");
    let (bridge, endpoint) = nxtools_ipc::platform_bridge(64);
    endpoint.set_connected_for_contract_test(true);
    let bridge = Rc::new(bridge);
    let notifications = cx.new(|_| crate::notifications::NotificationCenter::default());
    let settings = cx.new(|_| {
        NetworkSettingsApplication::new(
            nxtools_network::NetworkPolicy::new(config.network.clone()).expect("policy"),
            config.clone(),
            path.clone(),
            notifications.clone(),
            Some(bridge.clone()),
            Rc::new(RefCell::new(SystemSettings::default())),
            false,
        )
    });
    let catalog = nxtools_finder::FinderCatalog::open(directory.path().join("finder.db"), &[])
        .expect("catalog");
    let (indexer, _) = crate::runtime::indexer_bridge(8);
    let quick = cx.new(|_| {
        QuickLaunchApplication::new(
            catalog,
            bridge.clone(),
            Rc::new(indexer),
            config.clone(),
            path.clone(),
            directory.path().join("logs"),
            notifications.clone(),
        )
    });
    let downloads = cx.new(|cx| {
        DownloadApplication::new(
            nxtools_download::DownloadEngine::open_in_memory().expect("engine"),
            bridge.clone(),
            notifications,
            crate::downloads::DownloadRuntimeContext {
                fallback_default_directory: directory.path().into(),
                config,
                config_path: path,
                log_directory: directory.path().into(),
                database_path: directory.path().join("download.db"),
                ffmpeg_install_directory: directory.path().join("ffmpeg"),
            },
            cx,
        )
    });
    let calls = Rc::new(Cell::new(0));
    let ready = calls.clone();
    let dispatcher = cx.new(|cx| {
        PlatformEventDispatcher::new(
            bridge,
            None,
            42,
            true,
            PlatformApplicationTargets {
                clipboard: WeakEntity::new_invalid(),
                quick_launch: quick,
                downloads,
                global_settings: settings,
            },
            Some(MainWindowPreparation::new(42, move |_| {
                ready.set(ready.get() + 1)
            })),
            cx,
        )
    });
    (dispatcher, endpoint, calls, directory)
}
fn reply(id: Uuid, data: Option<&str>) -> PlatformNotification {
    PlatformNotification::CommandResult {
        correlation_id: id,
        result: PlatformCommandResult {
            success: true,
            error: None,
            native_error_code: None,
            data: data.map(str::to_owned),
        },
    }
}
const VALID: &str = r#"{"x":750,"y":375,"width":1060,"height":650,"dpi":96}"#;

#[gpui_kit::test]
fn r5_main_preparation_blocks_entrypoints_until_exact_readback_once(cx: &mut TestAppContext) {
    let (dispatcher, endpoint, calls, _directory) = fixture(cx);
    dispatcher.update(cx, |owner, cx| {
        owner.forward(PlatformNotification::Connected, cx)
    });
    let prepare = endpoint
        .try_receive_command_for_contract_test()
        .expect("prepare first");
    assert!(
        matches!(prepare.message, Message::PlatformCommand(PlatformCommand::PrepareMainWindow(ref command)) if command.window_handle == 42 && command.width == 1060 && command.height == 650)
    );
    assert_eq!(
        dispatcher.read_with(cx, |owner, _| owner
            .main_preparation
            .as_ref()
            .expect("barrier")
            .pending),
        Some(prepare.id)
    );
    assert!(
        endpoint.try_receive_command_for_contract_test().is_err(),
        "Connected cannot enable features before prepare ACK"
    );
    dispatcher.update(cx, |owner, cx| {
        owner.forward(
            PlatformNotification::Event(PlatformEvent::TrayActionRequested(TrayActionRequest {
                correlation_id: Uuid::new_v4(),
                action: TrayAction::Open,
            })),
            cx,
        )
    });
    assert_eq!(calls.get(), 0);
    assert!(
        endpoint.try_receive_command_for_contract_test().is_err(),
        "queued tray open stays behind preparation"
    );
    dispatcher.update(cx, |owner, cx| {
        owner.forward(reply(Uuid::new_v4(), Some(VALID)), cx)
    });
    assert_eq!(calls.get(), 0, "stale result cannot publish window");
    dispatcher.update(cx, |owner, cx| {
        owner.forward(reply(prepare.id, Some(VALID)), cx)
    });
    complete_material(&dispatcher, &endpoint, cx);
    cx.run_until_parked();
    assert_eq!(calls.get(), 1);
    assert!(dispatcher.read_with(cx, |owner, _| owner.main_preparation.is_none()));
    let commands = std::iter::from_fn(|| endpoint.try_receive_command_for_contract_test().ok())
        .collect::<Vec<_>>();
    assert!(commands.iter().any(|envelope| matches!(
        envelope.message,
        Message::PlatformCommand(PlatformCommand::SetWindowVisibility(_))
    )));
    dispatcher.update(cx, |owner, cx| {
        owner.forward(reply(prepare.id, Some(VALID)), cx)
    });
    assert_eq!(
        calls.get(),
        1,
        "duplicate ACK cannot reset user's later dimensions"
    );
}

#[gpui_kit::test]
fn r5_main_preparation_invalid_native_dimensions_never_publish(cx: &mut TestAppContext) {
    let (dispatcher, endpoint, calls, _directory) = fixture(cx);
    dispatcher.update(cx, |owner, cx| {
        owner.forward(PlatformNotification::Connected, cx)
    });
    let prepare = endpoint
        .try_receive_command_for_contract_test()
        .expect("prepare");
    dispatcher.update(cx, |owner, cx| {
        owner.forward(
            reply(
                prepare.id,
                Some(r#"{"x":208,"y":208,"width":1904,"height":1021,"dpi":96}"#),
            ),
            cx,
        )
    });
    assert_eq!(calls.get(), 0);
}

#[gpui_kit::test]
fn r5_main_preparation_disconnect_before_deferred_commit_never_publishes(cx: &mut TestAppContext) {
    let (dispatcher, endpoint, calls, _directory) = fixture(cx);
    dispatcher.update(cx, |owner, cx| {
        owner.forward(PlatformNotification::Connected, cx)
    });
    let prepare = endpoint
        .try_receive_command_for_contract_test()
        .expect("prepare");
    cx.update(|cx| {
        dispatcher.update(cx, |owner, cx| {
            owner.forward(reply(prepare.id, Some(VALID)), cx)
        });
        dispatcher.update(cx, |owner, cx| {
            owner.forward(
                PlatformNotification::Disconnected {
                    generation: 1,
                    pending_command_ids: vec![prepare.id],
                },
                cx,
            )
        });
    });
    assert_eq!(calls.get(), 0);
}

#[gpui_kit::test]
fn r5_main_preparation_deadline_and_rejected_dispatch_do_not_publish(cx: &mut TestAppContext) {
    let (dispatcher, _endpoint, calls, _directory) = fixture(cx);
    cx.executor()
        .advance_clock(std::time::Duration::from_secs(9));
    cx.run_until_parked();
    assert_eq!(calls.get(), 0);
    assert!(dispatcher.read_with(cx, |owner, _| owner.main_preparation.is_none()));
}

#[gpui_kit::test]
fn r5_main_preparation_notification_budget_never_enables_unprepared_main(cx: &mut TestAppContext) {
    let (dispatcher, _endpoint, calls, _directory) = fixture(cx);
    for _ in 0..=MAX_BUFFERED_STARTUP_NOTIFICATIONS {
        dispatcher.update(cx, |owner, cx| {
            owner.forward(reply(Uuid::new_v4(), None), cx)
        });
    }
    assert_eq!(calls.get(), 0);
    assert!(dispatcher.read_with(cx, |owner, _| owner.main_preparation.is_none()));
}

#[gpui_kit::test]
fn r5_main_preparation_rejected_queue_and_late_ack_cannot_publish(cx: &mut TestAppContext) {
    let (dispatcher, endpoint, calls, _directory) = fixture(cx);
    endpoint.set_connected_for_contract_test(false);
    dispatcher.update(cx, |owner, cx| {
        owner.forward(PlatformNotification::Connected, cx)
    });
    assert_eq!(calls.get(), 0);
    assert!(dispatcher.read_with(cx, |owner, _| owner.main_preparation.is_none()));
    dispatcher.update(cx, |owner, cx| {
        owner.forward(reply(Uuid::new_v4(), Some(VALID)), cx)
    });
    assert_eq!(calls.get(), 0);
}

#[gpui_kit::test]
fn material_main_preparation_requires_facts_and_material_after_size_ack(cx: &mut TestAppContext) {
    let (dispatcher, endpoint, calls, _directory) = fixture(cx);
    dispatcher.update(cx, |owner, cx| {
        owner.forward(PlatformNotification::Connected, cx)
    });
    let prepare = endpoint
        .try_receive_command_for_contract_test()
        .expect("prepare bounds");
    dispatcher.update(cx, |owner, cx| {
        owner.forward(reply(prepare.id, Some(VALID)), cx)
    });
    cx.run_until_parked();
    assert_eq!(
        calls.get(),
        0,
        "bounds ACK alone must not publish a material-unprepared window"
    );
}

fn facts(session_id: Uuid, revision: u64) -> PlatformNotification {
    PlatformNotification::Event(PlatformEvent::SystemMaterialCapabilitiesChanged(
        nxtools_protocol::window_material::SystemMaterialCapabilitiesChanged {
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
        },
    ))
}

fn next_material(
    endpoint: &nxtools_ipc::PlatformServerEndpoint,
) -> (Uuid, ApplyWindowMaterialCommand) {
    let envelope = endpoint
        .try_receive_command_for_contract_test()
        .expect("native material request");
    let Message::PlatformCommand(PlatformCommand::ApplyWindowMaterial(command)) = envelope.message
    else {
        panic!("material must precede entrypoint commands");
    };
    (envelope.id, command)
}

fn material_reply(id: Uuid, command: ApplyWindowMaterialCommand) -> PlatformNotification {
    use nxtools_protocol::window_material::{
        EffectiveWindowMaterial, MaterialFallbackReason, WindowMaterialStyle,
    };
    let receipt = AppliedWindowMaterial {
        session_id: command.session_id,
        facts_revision: command.facts_revision,
        window_handle: command.window_handle,
        generation: command.generation,
        style: command.style,
        surface: command.surface,
        dark: command.dark,
        effective: match command.style {
            WindowMaterialStyle::Solid => EffectiveWindowMaterial::Solid,
            _ => EffectiveWindowMaterial::Acrylic,
        },
        fallback_reason: MaterialFallbackReason::None,
        native_error_code: 0,
    };
    reply(
        id,
        Some(&serde_json::to_string(&receipt).expect("receipt JSON")),
    )
}

fn complete_material(
    dispatcher: &Entity<PlatformEventDispatcher>,
    endpoint: &nxtools_ipc::PlatformServerEndpoint,
    cx: &mut TestAppContext,
) {
    dispatcher.update(cx, |owner, cx| owner.forward(facts(Uuid::new_v4(), 1), cx));
    let (id, command) = next_material(endpoint);
    dispatcher.update(cx, |owner, cx| {
        owner.forward(material_reply(id, command), cx)
    });
}

#[gpui_kit::test]
fn material_main_preparation_facts_before_bounds_use_latest_explicit_dark(cx: &mut TestAppContext) {
    let (dispatcher, endpoint, calls, _directory) = fixture(cx);
    dispatcher.update(cx, |owner, cx| {
        owner.forward(PlatformNotification::Connected, cx)
    });
    let prepare = endpoint
        .try_receive_command_for_contract_test()
        .expect("bounds");
    dispatcher.update(cx, |owner, cx| {
        owner.forward(
            PlatformNotification::Event(PlatformEvent::SystemThemePreferenceChanged(
                nxtools_protocol::SystemThemePreferenceChanged {
                    correlation_id: Uuid::new_v4(),
                    dark: false,
                    native_error_code: 0,
                },
            )),
            cx,
        )
    });
    dispatcher.update(cx, |owner, cx| owner.forward(facts(Uuid::new_v4(), 1), cx));
    assert!(
        endpoint.try_receive_command_for_contract_test().is_err(),
        "facts do not bypass bounds ACK"
    );
    dispatcher.update(cx, |owner, cx| {
        owner.forward(reply(prepare.id, Some(VALID)), cx)
    });
    let (id, command) = next_material(&endpoint);
    assert!(
        !command.dark,
        "system theme must not remain trapped behind startup buffering"
    );
    assert_eq!(calls.get(), 0);
    dispatcher.update(cx, |owner, cx| {
        owner.forward(material_reply(id, command), cx)
    });
    cx.run_until_parked();
    assert_eq!(calls.get(), 1);
}

#[gpui_kit::test]
fn material_main_preparation_superseded_facts_ignore_old_receipt(cx: &mut TestAppContext) {
    let (dispatcher, endpoint, calls, _directory) = fixture(cx);
    dispatcher.update(cx, |owner, cx| {
        owner.forward(PlatformNotification::Connected, cx)
    });
    let bounds = endpoint
        .try_receive_command_for_contract_test()
        .expect("bounds");
    dispatcher.update(cx, |owner, cx| {
        owner.forward(reply(bounds.id, Some(VALID)), cx)
    });
    let session = Uuid::new_v4();
    dispatcher.update(cx, |owner, cx| owner.forward(facts(session, 1), cx));
    let (old_id, old) = next_material(&endpoint);
    dispatcher.update(cx, |owner, cx| owner.forward(facts(session, 2), cx));
    let superseded = dispatcher.read_with(cx, |owner, _| {
        owner
            .main_preparation
            .as_ref()
            .expect("preparation")
            .material
            .as_ref()
            .expect("queued material")
            .id
    });
    assert!(
        endpoint.try_receive_command_for_contract_test().is_err(),
        "native operation remains single-flight"
    );
    dispatcher.update(cx, |owner, cx| owner.forward(facts(session, 3), cx));
    assert!(
        !endpoint.command_pending_for_contract_test(superseded),
        "superseded queued generation retired"
    );
    let PlatformNotification::CommandResult { result, .. } = material_reply(old_id, old) else {
        unreachable!()
    };
    endpoint
        .route_command_result_for_contract_test(old_id, result)
        .expect("original native terminal");
    cx.run_until_parked();
    let (current_id, current) = next_material(&endpoint);
    assert!(current.generation > old.generation);
    assert_eq!(current.facts_revision, 3);
    assert_eq!(current.window_handle, old.window_handle);
    assert_eq!(calls.get(), 0);
    dispatcher.update(cx, |owner, cx| {
        owner.forward(material_reply(current_id, current), cx)
    });
    cx.run_until_parked();
    assert_eq!(calls.get(), 1);
}

#[gpui_kit::test]
fn material_main_preparation_invalid_receipt_never_reveals(cx: &mut TestAppContext) {
    let (dispatcher, endpoint, calls, _directory) = fixture(cx);
    dispatcher.update(cx, |owner, cx| {
        owner.forward(PlatformNotification::Connected, cx)
    });
    let bounds = endpoint
        .try_receive_command_for_contract_test()
        .expect("bounds");
    dispatcher.update(cx, |owner, cx| {
        owner.forward(reply(bounds.id, Some(VALID)), cx)
    });
    dispatcher.update(cx, |owner, cx| owner.forward(facts(Uuid::new_v4(), 1), cx));
    let (id, command) = next_material(&endpoint);
    dispatcher.update(cx, |owner, cx| {
        owner.forward(
            material_reply(
                id,
                ApplyWindowMaterialCommand {
                    window_handle: 43,
                    ..command
                },
            ),
            cx,
        )
    });
    cx.run_until_parked();
    assert_eq!(calls.get(), 0);
    assert!(dispatcher.read_with(cx, |owner, _| owner.main_preparation_failed));
}

#[gpui_kit::test]
fn material_main_preparation_disconnect_after_material_ack_never_publishes(
    cx: &mut TestAppContext,
) {
    let (dispatcher, endpoint, calls, _directory) = fixture(cx);
    dispatcher.update(cx, |owner, cx| {
        owner.forward(PlatformNotification::Connected, cx)
    });
    let bounds = endpoint
        .try_receive_command_for_contract_test()
        .expect("bounds");
    dispatcher.update(cx, |owner, cx| {
        owner.forward(reply(bounds.id, Some(VALID)), cx)
    });
    dispatcher.update(cx, |owner, cx| owner.forward(facts(Uuid::new_v4(), 1), cx));
    let (id, command) = next_material(&endpoint);
    cx.update(|cx| {
        dispatcher.update(cx, |owner, cx| {
            owner.forward(material_reply(id, command), cx)
        });
        dispatcher.update(cx, |owner, cx| {
            owner.forward(
                PlatformNotification::Disconnected {
                    generation: 1,
                    pending_command_ids: vec![id],
                },
                cx,
            )
        });
    });
    assert_eq!(calls.get(), 0);
}

#[gpui_kit::test]
fn material_main_preparation_native_stale_facts_wait_for_latest(cx: &mut TestAppContext) {
    let (dispatcher, endpoint, calls, _directory) = fixture(cx);
    dispatcher.update(cx, |owner, cx| {
        owner.forward(PlatformNotification::Connected, cx)
    });
    let bounds = endpoint
        .try_receive_command_for_contract_test()
        .expect("bounds");
    dispatcher.update(cx, |owner, cx| {
        owner.forward(reply(bounds.id, Some(VALID)), cx)
    });
    let session = Uuid::new_v4();
    dispatcher.update(cx, |owner, cx| owner.forward(facts(session, 1), cx));
    let (id, _) = next_material(&endpoint);
    endpoint
        .route_command_result_for_contract_test(
            id,
            PlatformCommandResult {
                success: false,
                error: Some("stale_material_facts".into()),
                native_error_code: None,
                data: None,
            },
        )
        .expect("native stale-facts terminal");
    cx.run_until_parked();
    assert!(!dispatcher.read_with(cx, |owner, _| owner.main_preparation_failed));
    assert!(endpoint.try_receive_command_for_contract_test().is_err());
    dispatcher.update(cx, |owner, cx| owner.forward(facts(session, 2), cx));
    let (id, command) = next_material(&endpoint);
    dispatcher.update(cx, |owner, cx| {
        owner.forward(material_reply(id, command), cx)
    });
    cx.run_until_parked();
    assert_eq!(calls.get(), 1);
}
