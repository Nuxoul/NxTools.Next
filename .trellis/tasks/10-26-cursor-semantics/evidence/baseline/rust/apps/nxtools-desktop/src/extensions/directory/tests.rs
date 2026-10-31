use super::*;
use gpui_kit::{AppContext, TestAppContext};

fn fixture(
    path: PathBuf,
    cx: &mut TestAppContext,
) -> (
    Entity<ExtensionApplication>,
    nxtools_ipc::PlatformServerEndpoint,
) {
    cx.executor().allow_parking();
    let (bridge, endpoint) = nxtools_ipc::platform_bridge(4);
    endpoint.set_connected_for_contract_test(true);
    let center = cx.new(|_| crate::notifications::NotificationCenter::default());
    let app = cx.new(|_| {
        ExtensionApplication::new(
            AppConfig::default(),
            path,
            Some(Rc::new(bridge)),
            WeakEntity::new_invalid(),
        )
    });
    app.update(cx, |app, _| app.notifications = Some(center));
    (app, endpoint)
}
fn reply(
    app: &Entity<ExtensionApplication>,
    id: uuid::Uuid,
    success: bool,
    cx: &mut TestAppContext,
) {
    app.update(cx, |app, cx| {
        app.handle_platform_notification(
            PlatformNotification::CommandResult {
                correlation_id: id,
                result: nxtools_protocol::PlatformCommandResult {
                    success,
                    error: None,
                    native_error_code: (!success).then_some(5),
                    data: None,
                },
            },
            cx,
        )
    });
    cx.run_until_parked();
}

fn notice_message(app: &Entity<ExtensionApplication>, cx: &TestAppContext) -> String {
    app.read_with(cx, |app, cx| {
        app.notifications
            .as_ref()
            .expect("shared center")
            .read(cx)
            .items()[0]
            .message
            .to_string()
    })
}

#[gpui_kit::test]
fn plugin_directory_creates_exact_managed_root_and_preserves_existing_contents_and_config(
    cx: &mut TestAppContext,
) {
    let temp = tempfile::tempdir().expect("owned directory fixture");
    let path = temp.path().join("profile").join("config.json");
    std::fs::create_dir_all(path.parent().expect("parent")).expect("owned profile");
    AppConfig::default().save_to(&path).expect("owned config");
    let before = std::fs::read(&path).expect("config bytes");
    let root = path.parent().expect("parent").join("extensions");
    let (app, endpoint) = fixture(path.clone(), cx);
    let packages = app.read_with(cx, |app, _| {
        serde_json::to_value(&app.packages).expect("registry")
    });
    for attempt in 0..2 {
        app.update(cx, |app, cx| {
            app.open_directory(cx);
            app.open_directory(cx);
        });
        cx.run_until_parked();
        assert!(root.is_dir());
        let command = endpoint
            .try_receive_command_for_contract_test()
            .expect("one open command");
        assert!(
            matches!(command.message, nxtools_protocol::Message::PlatformCommand(PlatformCommand::OpenShellTarget(open)) if std::path::Path::new(&open.target) == root && open.action == nxtools_protocol::ShellOpenAction::Open)
        );
        app.read_with(cx, |app, _| {
            assert_eq!(app.pending_directory.map(|(id, _)| id), Some(command.id));
            assert!(
                app.pending_runtime.contains_key(&command.id),
                "pending before command observation"
            );
            assert!(app.status.is_none(), "no page banner while opening");
        });
        assert!(
            endpoint.try_receive_command_for_contract_test().is_err(),
            "duplicate click no command"
        );
        reply(&app, command.id, true, cx);
        assert!(!app.read_with(cx, |app, _| app.opening_directory()));
        if attempt == 0 {
            std::fs::write(root.join("keep.txt"), b"preserve").expect("existing content");
        }
    }
    assert_eq!(
        std::fs::read(root.join("keep.txt")).expect("preserved"),
        b"preserve"
    );
    assert_eq!(std::fs::read(path).expect("unchanged config"), before);
    assert_eq!(
        app.read_with(cx, |app, _| serde_json::to_value(&app.packages)
            .expect("registry")),
        packages
    );
    assert!(app.read_with(cx, |app, _| app.recent_manual_runs().is_empty()));
}

#[gpui_kit::test]
fn plugin_directory_native_failure_disconnect_timeout_and_stale_result_are_retryable(
    cx: &mut TestAppContext,
) {
    let temp = tempfile::tempdir().expect("owned failures");
    let (app, endpoint) = fixture(temp.path().join("config.json"), cx);
    app.update(cx, |app, cx| app.open_directory(cx));
    cx.run_until_parked();
    let first = endpoint
        .try_receive_command_for_contract_test()
        .expect("first");
    reply(&app, first.id, false, cx);
    assert!(!app.read_with(cx, |app, _| app.opening_directory()));
    assert!(notice_message(&app, cx).contains("无法打开"));
    app.update(cx, |app, cx| app.open_directory(cx));
    cx.run_until_parked();
    let second = endpoint
        .try_receive_command_for_contract_test()
        .expect("retry");
    reply(&app, first.id, true, cx);
    assert_eq!(
        app.read_with(cx, |app, _| app.pending_directory.map(|(id, _)| id)),
        Some(second.id)
    );
    app.update(cx, |app, cx| {
        app.handle_platform_notification(
            PlatformNotification::Disconnected {
                generation: 1,
                pending_command_ids: vec![second.id],
            },
            cx,
        )
    });
    cx.run_until_parked();
    assert!(app.read_with(cx, |app, _| app.pending_runtime.is_empty()
        && !app.opening_directory()));
    reply(&app, second.id, true, cx);
    assert!(notice_message(&app, cx).contains("断开"));
    app.update(cx, |app, cx| app.open_directory(cx));
    cx.run_until_parked();
    let third = endpoint
        .try_receive_command_for_contract_test()
        .expect("timeout fixture");
    cx.executor()
        .advance_clock(std::time::Duration::from_secs(31));
    cx.run_until_parked();
    assert!(app.read_with(cx, |app, _| app.pending_runtime.is_empty()
        && !app.opening_directory()));
    assert!(notice_message(&app, cx).contains("超时"));
    reply(&app, third.id, true, cx);
    assert!(notice_message(&app, cx).contains("超时"));
}

#[gpui_kit::test]
fn plugin_directory_creation_and_platform_admission_failures_clear_pending_without_dispatch(
    cx: &mut TestAppContext,
) {
    let temp = tempfile::tempdir().expect("owned admission failures");
    let root = temp.path().join("extensions");
    std::fs::write(&root, b"not a directory").expect("owned conflict");
    let (app, endpoint) = fixture(temp.path().join("config.json"), cx);
    app.update(cx, |app, cx| app.open_directory(cx));
    cx.run_until_parked();
    assert!(!app.read_with(cx, |app, _| app.opening_directory()));
    assert!(notice_message(&app, cx).contains("无法创建"));
    assert!(endpoint.try_receive_command_for_contract_test().is_err());
    assert_eq!(
        std::fs::read(&root).expect("no overwrite"),
        b"not a directory"
    );
    std::fs::remove_file(&root).expect("remove owned conflict");
    endpoint.set_connected_for_contract_test(false);
    app.update(cx, |app, cx| app.open_directory(cx));
    cx.run_until_parked();
    assert!(app.read_with(cx, |app, _| !app.opening_directory()
        && app.pending_runtime.is_empty()));
    assert!(endpoint.try_receive_command_for_contract_test().is_err());
    endpoint.set_connected_for_contract_test(true);
    // Saturate the original bounded reply map using ordinary environment requests.
    app.update(cx, |app, cx| {
        for _ in 0..4 {
            assert!(
                app.request_runtime(
                    PlatformCommand::OpenShellTarget(nxtools_protocol::OpenShellTargetCommand {
                        target: "C:\\owned-unused".into(),
                        action: nxtools_protocol::ShellOpenAction::Open
                    }),
                    cx
                )
                .is_some()
            );
        }
        app.open_directory(cx);
    });
    cx.run_until_parked();
    assert!(app.read_with(cx, |app, _| !app.opening_directory()
        && app.pending_runtime.len() == 4));
    for _ in 0..4 {
        endpoint
            .try_receive_command_for_contract_test()
            .expect("only filler");
    }
    assert!(
        endpoint.try_receive_command_for_contract_test().is_err(),
        "no directory command on full map"
    );
}

#[gpui_kit::test]
fn directory_result_uses_shared_notification_without_page_banner(cx: &mut TestAppContext) {
    let temp = tempfile::tempdir().expect("owned notification fixture");
    let (app, endpoint) = fixture(temp.path().join("config.json"), cx);
    let center = cx.new(|_| crate::notifications::NotificationCenter::default());
    app.update(cx, |app, cx| {
        app.notifications = Some(center.clone());
        app.open_directory(cx);
    });
    cx.run_until_parked();
    let command = endpoint
        .try_receive_command_for_contract_test()
        .expect("folder open");
    reply(&app, command.id, true, cx);
    assert!(
        app.read_with(cx, |app, _| app.status.is_none()),
        "directory operation must not publish the rejected page banner"
    );
    center.read_with(cx, |center, _| {
        assert_eq!(
            center.items().len(),
            1,
            "success must reach the sole notification center"
        );
        assert_eq!(
            center.items()[0].level,
            crate::notifications::NotificationLevel::Success
        );
    });
}
