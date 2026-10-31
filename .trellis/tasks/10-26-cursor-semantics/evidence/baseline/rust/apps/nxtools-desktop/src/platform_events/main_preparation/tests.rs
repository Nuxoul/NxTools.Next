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
