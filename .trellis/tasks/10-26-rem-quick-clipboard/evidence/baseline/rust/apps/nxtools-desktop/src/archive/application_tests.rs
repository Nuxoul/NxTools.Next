//! Real application/window and product transport boundary, no native runtime.
use super::*;
use gpui_kit::TestAppContext;
use gpui_kit::component::WindowExt as _;
use nxtools_protocol::{Message, PlatformCommand, PlatformCommandResult};
struct PickerHost;
impl gpui_kit::Render for PickerHost {
    fn render(
        &mut self,
        _: &mut gpui_kit::Window,
        _: &mut Context<Self>,
    ) -> impl gpui_kit::IntoElement {
        gpui_kit::div()
    }
}
pub(super) fn setup(
    cx: &mut TestAppContext,
) -> (
    Entity<ArchiveApplication>,
    nxtools_ipc::PlatformServerEndpoint,
) {
    cx.update(gpui_kit::init);
    cx.executor().allow_parking();
    let (bridge, endpoint) = nxtools_ipc::platform_bridge(32);
    endpoint.set_connected_for_contract_test(true);
    let bridge = Rc::new(bridge);
    let client = Arc::new(
        ArchiveReadClient::new(bridge.archive_client_bridge()).expect("original read client"),
    );
    let owner = cx.new(|cx| ArchiveApplication::new(bridge, Some(client), cx));
    (owner, endpoint)
}
#[gpui_kit::test]
fn production_keyed_windows_retire_and_reopen_without_stale_view(cx: &mut TestAppContext) {
    let (owner, endpoint) = setup(cx);
    let path = PathBuf::from(r"C:\owned\archive.zip");
    owner.update(cx, |app, cx| {
        app.open_canonical(path.clone(), 0, vec![], cx)
    });
    cx.run_until_parked();
    let (first, first_view) = owner.read_with(cx, |app, _| {
        let (id, slot) = app.windows.iter().next().expect("browser");
        (*id, slot.view.clone())
    });
    let start = endpoint
        .try_receive_command_for_contract_test()
        .expect("real typed read");
    assert!(matches!(
        start.message,
        Message::PlatformCommand(PlatformCommand::ArchiveBrowserRead(ArchiveReadCommand {
            version: 2,
            limits: Some(ArchiveReadLimits {
                browser_context: true,
                ..
            }),
            ..
        }))
    ));
    owner.update(cx, |app, cx| {
        app.open_canonical(path.clone(), 0, vec![], cx)
    });
    cx.run_until_parked();
    assert_eq!(owner.read_with(cx, |app, _| app.windows.len()), 1);
    owner.update(cx, |app, cx| app.close_browser(first, cx));
    cx.run_until_parked();
    assert!(owner.read_with(cx, |app, _| app.windows.is_empty() && app.reads.is_empty()));
    assert!(first_view.upgrade().is_none());
    owner.update(cx, |app, cx| app.open_canonical(path, 0, vec![], cx));
    cx.run_until_parked();
    assert_ne!(
        owner.read_with(cx, |app, _| *app.windows.keys().next().expect("reopen")),
        first
    );
}
#[gpui_kit::test]
fn picker_cancel_and_closed_origin_never_open_a_browser(cx: &mut TestAppContext) {
    let (owner, endpoint) = setup(cx);
    let (_, surface) = cx.add_window_view(|_, _| PickerHost);
    let source = surface.update(|w, _| w.window_handle());
    let mut surface = gpui_kit::VisualTestContext::from_window(source, cx);
    owner.update(cx, |app, cx| app.pick_archive(source, cx));
    let command = endpoint
        .try_receive_command_for_contract_test()
        .expect("picker");
    assert!(matches!(
        command.message,
        Message::PlatformCommand(PlatformCommand::PickArchivePath(_))
    ));
    owner.update(cx, |app, cx| {
        app.notification(
            PlatformNotification::CommandResult {
                correlation_id: command.id,
                result: PlatformCommandResult {
                    success: true,
                    error: None,
                    native_error_code: None,
                    data: Some(
                        serde_json::to_string(&nxtools_protocol::ArchivePickerResult::Cancelled)
                            .expect("cancelled wire"),
                    ),
                },
            },
            cx,
        )
    });
    assert!(owner.read_with(cx, |app, _| app.picker.is_none() && app.windows.is_empty()));
    owner.update(cx, |app, cx| app.pick_archive(source, cx));
    let command = endpoint
        .try_receive_command_for_contract_test()
        .expect("second picker");
    surface.update(|w, _| w.remove_window());
    owner.update(cx, |app, cx| {
        app.notification(
            PlatformNotification::CommandResult {
                correlation_id: command.id,
                result: PlatformCommandResult {
                    success: true,
                    error: None,
                    native_error_code: None,
                    data: Some(
                        serde_json::to_string(&nxtools_protocol::ArchivePickerResult::Selected {
                            path: r"C:\owned\late.zip".into(),
                        })
                        .expect("selected wire"),
                    ),
                },
            },
            cx,
        )
    });
    cx.run_until_parked();
    assert!(owner.read_with(cx, |app, _| app.windows.is_empty()));
}
#[gpui_kit::test]
fn cancel_retires_original_read_and_stops_loading(cx: &mut TestAppContext) {
    let (owner, _) = setup(cx);
    owner.update(cx, |app, cx| {
        app.open_canonical(PathBuf::from(r"C:\owned\archive.zip"), 0, vec![], cx)
    });
    cx.run_until_parked();
    let (id, view) = owner.read_with(cx, |app, _| {
        let (id, s) = app.windows.iter().next().expect("browser");
        (*id, s.view.clone())
    });
    owner.update(cx, |app, cx| app.cancel_browser(id, cx));
    cx.run_until_parked();
    assert!(owner.read_with(cx, |app, _| app.reads.is_empty()));
    assert!(
        view.read_with(cx, |view, _| matches!(
            view.status,
            BrowserStatus::Cancelled
        ))
        .expect("live view")
    );
}

fn read_event(request: &ArchiveReadCommand, kind: ArchiveReadEventKind) -> ArchiveReadEvent {
    ArchiveReadEvent {
        version: request.version,
        operation_id: request.operation_id,
        session_id: request.session_id,
        generation: request.generation,
        kind,
        capabilities: vec![],
        batch: None,
        terminal: None,
    }
}
fn deliver(
    endpoint: &nxtools_ipc::PlatformServerEndpoint,
    event: ArchiveReadEvent,
    cx: &mut TestAppContext,
) {
    endpoint
        .send_notification_for_contract_test(PlatformNotification::Event(
            nxtools_protocol::PlatformEvent::ArchiveRead(event),
        ))
        .expect("original dedicated transport");
    cx.run_until_parked();
}

#[gpui_kit::test]
fn real_read_credit_install_and_selected_entry_command_share_original_identity(
    cx: &mut TestAppContext,
) {
    use nxtools_protocol::archive_browser::*;
    let (owner, endpoint) = setup(cx);
    owner.update(cx, |app, cx| {
        app.settings.filename_encoding = nxtools_core::archive::ArchiveFilenameEncoding::Gbk;
        app.settings.copy_motw = false;
        app.settings.temp = nxtools_core::archive::ArchiveTempSettings {
            base_directory: r"C:\owned\temp-base".into(),
            retention_hours: 6,
            cleanup_on_child_exit: true,
        };
        app.open_canonical(PathBuf::from(r"C:\owned\fixture.zip"), 0, vec![], cx)
    });
    cx.run_until_parked();
    let start = endpoint
        .try_receive_command_for_contract_test()
        .expect("start");
    let Message::PlatformCommand(PlatformCommand::ArchiveBrowserRead(request)) = start.message
    else {
        panic!("typed original read")
    };
    assert_eq!(request.codepage, 936);
    deliver(
        &endpoint,
        read_event(&request, ArchiveReadEventKind::Accepted),
        cx,
    );
    let mut negotiated = read_event(&request, ArchiveReadEventKind::Negotiated);
    negotiated.capabilities = archive_read_capabilities(2)
        .iter()
        .map(|value| (*value).into())
        .collect();
    deliver(&endpoint, negotiated, cx);
    let snapshot = Uuid::new_v4();
    let mut batch = read_event(&request, ArchiveReadEventKind::Batch);
    batch.batch = Some(ArchiveReadBatch { snapshot_id: snapshot, sequence: 0, format: "Zip".into(), entries: vec![serde_json::from_value(serde_json::json!({"ordinal":0,"name":"literal.txt","kind":"File","size":7,"packed_size":7,"encrypted":true,"warning_codes":[],"provenance":"Utf8Flag","codepage":65001})).expect("metadata")] });
    deliver(&endpoint, batch, cx);
    let ack = endpoint
        .try_receive_command_for_contract_test()
        .expect("consumer retained before credit");
    assert!(matches!(
        ack.message,
        Message::PlatformCommand(PlatformCommand::ArchiveRead(ArchiveReadCommand {
            action: ArchiveReadAction::Ack,
            sequence: Some(0),
            ..
        }))
    ));
    let browser = ArchiveBrowserContext {
        snapshot_id: snapshot,
        format: "Zip".into(),
        comment: "plain comment".into(),
        source: ArchiveSourceBinding {
            native: "v1:fixture".into(),
            digest: [1; 32],
            size: 100,
            modified_unix_ns: 1,
        },
    };
    let mut terminal = read_event(&request, ArchiveReadEventKind::Terminal);
    terminal.terminal = Some(ArchiveReadTerminal {
        status: ArchiveReadStatus::Succeeded,
        code: None,
        files_verified: 1,
        bytes_verified: 0,
        entry_count: 1,
        batch_count: 1,
        elapsed_ms: 1,
        native_error_code: None,
        cleanup_complete: true,
        transfer: Some(ArchiveReadTransfer {
            private_bytes: 4096,
            received_bytes: 4096,
            max_batch_entries: 1,
            max_batch_bytes: 1024,
            max_inflight_batches: 1,
            process_budget_bytes: 536870912,
        }),
        browser: Some(Box::new(browser.clone())),
    });
    deliver(&endpoint, terminal, cx);
    let (id, view) = owner.read_with(cx, |app, _| {
        let (id, slot) = app.windows.iter().next().expect("live");
        assert_eq!(slot.browser, Some(browser));
        (*id, slot.view.upgrade().expect("view"))
    });
    view.update(cx, |view, _| {
        assert!(matches!(view.status, BrowserStatus::Ready));
        view.model.as_mut().expect("installed").select(
            crate::archive::model::RowId::Entry(snapshot, 0),
            false,
            false,
        );
    });
    owner.update(cx, |app, cx| {
        app.open_entry(id, crate::archive::model::RowId::Entry(snapshot, 0), cx)
    });
    let materialize = endpoint
        .try_receive_command_for_contract_test()
        .expect("materialize");
    let receipt = materialize.id;
    let Message::PlatformCommand(PlatformCommand::ArchiveEntry(command)) = materialize.message
    else {
        panic!("typed entry")
    };
    assert_eq!(materialize.correlation_id, Some(command.operation_id));
    assert_ne!(receipt, command.operation_id);
    assert_eq!(command.ordinal, Some(0));
    assert_eq!(command.copy_motw, Some(false));
    assert_eq!(
        command.temp_policy,
        Some(ArchiveTempPolicy {
            base_directory: r"C:\owned\temp-base".into(),
            retention_hours: 6,
            cleanup_on_child_exit: true
        })
    );
    assert!(
        command.password_interaction,
        "browser materialize opts into private interaction without carrying a secret"
    );
    assert!(
        !serde_json::to_string(&command)
            .expect("wire")
            .contains("\"password\"")
    );
    let lease_id = Uuid::new_v4();
    let result = ArchiveEntryResult {
        version: 1,
        operation_id: command.operation_id,
        session_id: id,
        generation: command.generation,
        success: true,
        lease_id: Some(lease_id),
        path: Some(r"C:\owned\lease\literal.txt".into()),
        code: None,
        native_error_code: None,
        cleanup_complete: true,
        retained: false,
    };
    owner.update(cx, |app, cx| {
        app.notification(
            PlatformNotification::CommandResult {
                correlation_id: receipt,
                result: PlatformCommandResult {
                    success: true,
                    error: None,
                    native_error_code: None,
                    data: Some(serde_json::to_string(&result).expect("terminal")),
                },
            },
            cx,
        )
    });
    let shell = endpoint
        .try_receive_command_for_contract_test()
        .expect("Shell only after completed materialization");
    assert!(matches!(
        &shell.message,
        Message::PlatformCommand(PlatformCommand::ArchiveEntry(ArchiveEntryCommand { action: ArchiveEntryAction::Open, lease_id: Some(value), path: None, temp_policy: None, .. })) if *value == lease_id
    ));
    assert_eq!(shell.correlation_id, Some(command.operation_id));
    owner.update(cx, |app, cx| {
        app.notification(
            PlatformNotification::CommandResult {
                correlation_id: shell.id,
                result: PlatformCommandResult {
                    success: true,
                    error: None,
                    native_error_code: None,
                    data: None,
                },
            },
            cx,
        )
    });
    owner.update(cx, |app, cx| app.close_browser(id, cx));
    let release = endpoint
        .try_receive_command_for_contract_test()
        .expect("release on retirement");
    assert!(
        matches!(release.message, Message::PlatformCommand(PlatformCommand::ArchiveEntry(ArchiveEntryCommand { action: ArchiveEntryAction::Release, operation_id, lease_id: Some(value), .. })) if operation_id == command.operation_id && value == lease_id)
    );
}

#[gpui_kit::test]
fn disconnect_ends_pending_and_late_reply_cannot_revive_browser(cx: &mut TestAppContext) {
    let (owner, endpoint) = setup(cx);
    owner.update(cx, |app, cx| {
        app.open_canonical(PathBuf::from(r"C:\owned\fixture.zip"), 0, vec![], cx)
    });
    cx.run_until_parked();
    let start = endpoint
        .try_receive_command_for_contract_test()
        .expect("read");
    owner.update(cx, |app, cx| {
        app.notification(
            PlatformNotification::Disconnected {
                generation: 1,
                pending_command_ids: vec![start.id],
            },
            cx,
        )
    });
    cx.run_until_parked();
    owner.read_with(cx, |app, cx| {
        assert!(app.reads.is_empty() && app.entries.is_empty() && app.shells.is_empty());
        let slot = app.windows.values().next().expect("retained window");
        assert!(slot.browser.is_none());
        assert!(matches!(
            slot.view
                .read_with(cx, |v, _| v.status.clone())
                .expect("view"),
            BrowserStatus::Failed(_)
        ));
    });
}

#[gpui_kit::test]
fn failed_read_admission_stops_loading_on_the_new_generation(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.executor().allow_parking();
    let (bridge, _) = nxtools_ipc::platform_bridge(8);
    let owner = cx.new(|cx| ArchiveApplication::new(Rc::new(bridge), None, cx));
    owner.update(cx, |app, cx| {
        app.open_canonical(PathBuf::from(r"C:\owned\unavailable.zip"), 0, vec![], cx)
    });
    cx.run_until_parked();
    owner.read_with(cx, |app, cx| {
        let slot = app.windows.values().next().expect("failure stays visible");
        slot.view
            .read_with(cx, |view, _| {
                assert_eq!(view.generation, slot.generation);
                assert!(matches!(view.status, BrowserStatus::Failed(_)));
                assert!(view.read_id.is_none());
            })
            .expect("view");
    });
}

#[gpui_kit::test]
fn nested_browser_owns_lease_independently_after_parent_retirement(cx: &mut TestAppContext) {
    let (owner, _) = setup(cx);
    owner.update(cx, |app, cx| {
        app.open_canonical(PathBuf::from(r"C:\owned\outer.zip"), 0, vec![], cx)
    });
    cx.run_until_parked();
    let parent = owner.read_with(cx, |app, _| *app.windows.keys().next().expect("parent"));
    let lease = Lease {
        session: parent,
        generation: 1,
        id: Uuid::new_v4(),
        operation: Uuid::new_v4(),
    };
    let lease_id = lease.id;
    owner.update(cx, |app, cx| {
        app.open_canonical(PathBuf::from(r"C:\owned\child.zip"), 1, vec![lease], cx)
    });
    cx.run_until_parked();
    owner.update(cx, |app, cx| app.close_browser(parent, cx));
    cx.run_until_parked();
    owner.read_with(cx, |app, _| {
        assert_eq!(app.windows.len(), 1);
        let child = app.windows.values().next().expect("child remains");
        assert_eq!(child.depth, 1);
        assert_eq!(child.leases[0].id, lease_id);
    });
}

#[gpui_kit::test]
fn close_reopen_restores_actual_window_columns_and_persists_separate_state(
    cx: &mut TestAppContext,
) {
    let (owner, _) = setup(cx);
    let directory = tempfile::tempdir().expect("owned UI state");
    let path = directory.path().join("archive-ui-state.json");
    let store = UiStateStore::initialize(path.clone()).expect("writer");
    for _ in 0..200 {
        if !matches!(store.load_status(), UiStateLoad::Loading) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    owner.update(cx, |app, cx| {
        app.ui_state = Some(store);
        app.open_canonical(PathBuf::from(r"C:\owned\remember.zip"), 0, vec![], cx);
    });
    cx.run_until_parked();
    let (id, window, view) = owner.read_with(cx, |app, _| {
        let (id, slot) = app.windows.iter().next().expect("browser");
        (*id, slot.window, slot.view.clone())
    });
    let mut expected = UiState::default();
    expected.widths[0] = 328.;
    expected.visible[5] = true;
    expected.geometry.width = 1080;
    expected.geometry.height = 720;
    window
        .update(cx, |_, window, cx| {
            view.update(cx, |view, cx| {
                view.apply_restored_state(expected.clone(), window, cx);
                view.final_snapshot = Some(expected.clone());
            })
            .expect("view");
        })
        .expect("window");
    owner.update(cx, |app, cx| app.close_browser(id, cx));
    cx.run_until_parked();
    owner.update(cx, |app, cx| {
        app.open_canonical(PathBuf::from(r"C:\owned\remember.zip"), 0, vec![], cx)
    });
    cx.run_until_parked();
    owner.read_with(cx, |app, cx| {
        let slot = app.windows.values().next().expect("reopened");
        let state = slot
            .view
            .read_with(cx, |view, _| view.snapshot_from_observed())
            .expect("view");
        assert_eq!(state.widths, expected.widths);
        assert_eq!(state.visible, expected.visible);
    });
    owner.update(cx, |app, _| {
        app.ui_state.take().expect("store").finish_for_test()
    });
    let persisted: UiState =
        serde_json::from_slice(&std::fs::read(path).expect("actual state file")).expect("state");
    assert_eq!(persisted.widths, expected.widths);
    assert!(
        !std::fs::read_to_string(directory.path().join("archive-ui-state.json"))
            .expect("json")
            .contains("remember.zip")
    );
}

#[gpui_kit::test]
fn password_escape_and_retired_reply_never_submit_shell(cx: &mut TestAppContext) {
    use gpui_kit::component::WindowExt as _;
    use nxtools_protocol::archive_password::*;
    let (owner, endpoint) = setup(cx);
    owner.update(cx, |app, cx| {
        app.open_canonical(PathBuf::from(r"C:\owned\encrypted.7z"), 0, vec![], cx)
    });
    cx.run_until_parked();
    let start = endpoint
        .try_receive_command_for_contract_test()
        .expect("read");
    let Message::PlatformCommand(PlatformCommand::ArchiveBrowserRead(read)) = start.message else {
        panic!("browser read");
    };
    let source = owner.read_with(cx, |app, _| app.windows[&read.session_id].window);
    let mut surface = gpui_kit::VisualTestContext::from_window(source, cx);
    let prompt = ArchivePasswordPrompt {
        operation_id: read.operation_id,
        session_id: read.session_id,
        generation: read.generation,
        prompt_id: Uuid::new_v4(),
        ticket_generation: 1,
        error_code: "PasswordRequired".into(),
        attempt: 1,
    };
    owner.update(cx, |app, cx| app.password_prompt(prompt.clone(), cx));
    cx.run_until_parked();
    surface.update(|window, cx| {
        window.activate_window();
        window.draw(cx).clear(cx);
        assert!(window.has_active_dialog(cx));
    });
    surface.simulate_keystrokes("escape");
    surface.run_until_parked();
    let reply = endpoint
        .try_receive_command_for_contract_test()
        .expect("private cancel");
    assert!(matches!(
        reply.message,
        Message::PlatformCommand(PlatformCommand::ArchivePasswordReply(
            ArchivePasswordReply { password: None, .. }
        ))
    ));
    owner.update(cx, |app, cx| {
        app.close_browser(read.session_id, cx);
        app.password_reply(prompt, Some(PrivatePassword::new("late")), cx);
    });
    while let Ok(envelope) = endpoint.try_receive_command_for_contract_test() {
        assert!(!matches!(
            envelope.message,
            Message::PlatformCommand(
                PlatformCommand::OpenShellTarget(_) | PlatformCommand::ArchivePasswordReply(_)
            )
        ));
    }
}

#[gpui_kit::test]
fn production_row_drag_read_hydration_and_result_route_share_original_snapshot(
    cx: &mut TestAppContext,
) {
    use nxtools_protocol::archive_browser::*;
    let (owner, endpoint) = setup(cx);
    owner.update(cx, |app, cx| {
        app.open_canonical(PathBuf::from(r"C:\owned\drag.zip"), 0, vec![], cx)
    });
    cx.run_until_parked();
    let start = endpoint
        .try_receive_command_for_contract_test()
        .expect("read start");
    let Message::PlatformCommand(PlatformCommand::ArchiveBrowserRead(request)) = start.message
    else {
        panic!("read")
    };
    deliver(
        &endpoint,
        read_event(&request, ArchiveReadEventKind::Accepted),
        cx,
    );
    let mut negotiated = read_event(&request, ArchiveReadEventKind::Negotiated);
    negotiated.capabilities = archive_read_capabilities(2)
        .iter()
        .map(|value| (*value).into())
        .collect();
    deliver(&endpoint, negotiated, cx);
    let snapshot = Uuid::new_v4();
    let mut batch = read_event(&request, ArchiveReadEventKind::Batch);
    batch.batch=Some(ArchiveReadBatch {snapshot_id:snapshot,sequence:0,format:"Zip".into(),entries:vec![serde_json::from_value(serde_json::json!({"ordinal":0,"name":"drag.txt","kind":"File","size":3,"packed_size":3,"encrypted":false,"warning_codes":[],"provenance":"Utf8Flag","codepage":65001})).expect("entry")]});
    deliver(&endpoint, batch, cx);
    let _ack = endpoint
        .try_receive_command_for_contract_test()
        .expect("credit");
    let browser = ArchiveBrowserContext {
        snapshot_id: snapshot,
        format: "Zip".into(),
        comment: String::new(),
        source: ArchiveSourceBinding {
            native: "v1:hydrated".into(),
            digest: [7; 32],
            size: 100,
            modified_unix_ns: 1,
        },
    };
    let mut terminal = read_event(&request, ArchiveReadEventKind::Terminal);
    terminal.terminal = Some(ArchiveReadTerminal {
        status: ArchiveReadStatus::Succeeded,
        code: None,
        files_verified: 1,
        bytes_verified: 0,
        entry_count: 1,
        batch_count: 1,
        elapsed_ms: 1,
        native_error_code: None,
        cleanup_complete: true,
        transfer: Some(ArchiveReadTransfer {
            private_bytes: 4096,
            received_bytes: 4096,
            max_batch_entries: 1,
            max_batch_bytes: 1024,
            max_inflight_batches: 1,
            process_budget_bytes: 536870912,
        }),
        browser: Some(Box::new(browser.clone())),
    });
    deliver(&endpoint, terminal, cx);
    let (window, view) = owner.read_with(cx, |app, _| {
        let slot = app.windows.values().next().expect("browser");
        (slot.window, slot.view.upgrade().expect("view"))
    });
    let mut surface = gpui_kit::VisualTestContext::from_window(window, cx);
    surface.simulate_resize(gpui_kit::size(px(960.), px(640.)));
    surface.run_until_parked();
    surface.update(|window, cx| {
        window.activate_window();
        window.draw(cx).clear(cx);
    });
    view.read_with(&surface, |view, _| {
        assert!(
            matches!(view.status, BrowserStatus::Ready),
            "original read must install complete view"
        )
    });
    // Native Window resolution alone is synthetic; all source context came through original read install.
    view.update(&mut surface, |view, _| view.set_drag_hwnd_for_test(311));
    let point = surface
        .debug_bounds("archive-entry-0")
        .expect("production row")
        .center();
    surface.simulate_event(gpui_kit::MouseDownEvent {
        position: point,
        button: gpui_kit::MouseButton::Left,
        click_count: 1,
        ..Default::default()
    });
    surface.simulate_event(gpui_kit::MouseMoveEvent {
        position: gpui_kit::point(point.x + px(12.), point.y),
        pressed_button: Some(gpui_kit::MouseButton::Left),
        ..Default::default()
    });
    let wire = endpoint
        .try_receive_command_for_contract_test()
        .expect("formal row command");
    let Message::PlatformCommand(PlatformCommand::ArchiveVirtualDrag(drag)) = wire.message else {
        panic!("formal drag")
    };
    assert_eq!(
        drag.items.as_ref().expect("selected")[0].entry.browser,
        Some(browser)
    );
    assert_eq!(
        drag.items.as_ref().expect("selected")[0].entry.ordinal,
        Some(0)
    );
    assert_eq!(drag.source_window_handle, 311);
    owner.update(&mut surface, |app, cx| {
        app.notification(
            PlatformNotification::CommandResult {
                correlation_id: wire.id,
                result: PlatformCommandResult {
                    success: false,
                    error: Some("virtual_target_not_accepted".into()),
                    native_error_code: None,
                    data: None,
                },
            },
            cx,
        );
        assert!(
            view.read(cx).drag_matches(wire.id),
            "cross-entity result writeback is deferred within current callback"
        );
    });
    surface.run_until_parked();
    view.read_with(&surface, |view, _| assert!(!view.drag_matches(wire.id)));
}

// Keep original transport/client/completion + deferred install; pause only the application drain
// so replacement can occur at a deterministic point before that deferred installation runs.
fn complete_connection_drag_read(
    owner: &Entity<ArchiveApplication>,
    endpoint: &nxtools_ipc::PlatformServerEndpoint,
    cx: &mut TestAppContext,
) -> nxtools_protocol::archive_browser::ArchiveBrowserContext {
    complete_connection_drag_read_with_encryption(owner, endpoint, Some(false), cx)
}

fn complete_connection_drag_read_with_encryption(
    owner: &Entity<ArchiveApplication>,
    endpoint: &nxtools_ipc::PlatformServerEndpoint,
    encrypted: Option<bool>,
    cx: &mut TestAppContext,
) -> nxtools_protocol::archive_browser::ArchiveBrowserContext {
    use nxtools_protocol::archive_browser::*;
    let wire = endpoint
        .try_receive_command_for_contract_test()
        .expect("original read");
    let Message::PlatformCommand(PlatformCommand::ArchiveBrowserRead(request)) = wire.message
    else {
        panic!("browser read");
    };
    deliver(
        endpoint,
        read_event(&request, ArchiveReadEventKind::Accepted),
        cx,
    );
    let mut negotiated = read_event(&request, ArchiveReadEventKind::Negotiated);
    negotiated.capabilities = archive_read_capabilities(2)
        .iter()
        .map(|v| (*v).into())
        .collect();
    deliver(endpoint, negotiated, cx);
    let snapshot = Uuid::new_v4();
    let mut batch = read_event(&request, ArchiveReadEventKind::Batch);
    batch.batch = Some(ArchiveReadBatch {
        snapshot_id: snapshot, sequence: 0, format: "Zip".into(), entries: vec![
            serde_json::from_value(serde_json::json!({"ordinal":0,"name":"drag.txt","kind":"File","size":3,"packed_size":3,"encrypted":encrypted,"warning_codes":[],"provenance":"Utf8Flag","codepage":65001})).expect("entry")
        ],
    });
    deliver(endpoint, batch, cx);
    owner.update(cx, |app, cx| app.consume(cx));
    endpoint
        .try_receive_command_for_contract_test()
        .expect("original credit");
    let browser = ArchiveBrowserContext {
        snapshot_id: snapshot,
        format: "Zip".into(),
        comment: String::new(),
        source: ArchiveSourceBinding {
            native: "v1:origin".into(),
            digest: [7; 32],
            size: 100,
            modified_unix_ns: 1,
        },
    };
    let mut terminal = read_event(&request, ArchiveReadEventKind::Terminal);
    terminal.terminal = Some(ArchiveReadTerminal {
        status: ArchiveReadStatus::Succeeded,
        code: None,
        files_verified: 1,
        bytes_verified: 0,
        entry_count: 1,
        batch_count: 1,
        elapsed_ms: 1,
        native_error_code: None,
        cleanup_complete: true,
        transfer: Some(ArchiveReadTransfer {
            private_bytes: 4096,
            received_bytes: 4096,
            max_batch_entries: 1,
            max_batch_bytes: 1024,
            max_inflight_batches: 1,
            process_budget_bytes: 536870912,
        }),
        browser: Some(Box::new(browser.clone())),
    });
    deliver(endpoint, terminal, cx);
    assert!(owner.read_with(cx, |app, _| {
        app.reads
            .values()
            .next()
            .expect("completed pending install")
            .operation
            .terminal()
            .is_some()
    }));
    browser
}

fn connection_origin_row_race(cx: &mut TestAppContext, replace_before_install: bool) {
    let (owner, endpoint) = setup(cx);
    owner.update(cx, |app, cx| {
        app._drain = cx.spawn(async move |_, _| {});
        app.open_canonical(PathBuf::from(r"C:\owned\origin.zip"), 0, vec![], cx);
    });
    cx.run_until_parked();
    complete_connection_drag_read(&owner, &endpoint, cx);
    let platform = owner.read_with(cx, |app, _| app.platform.clone());
    let original_connection = platform.connection_generation().expect("origin");
    let mut replaced = None;
    owner.update(cx, |app, cx| {
        app.consume(cx);
        if replace_before_install {
            replaced = Some(endpoint.open_connection_for_contract_test());
        }
    });
    let (id, window, view) = owner.read_with(cx, |app, _| {
        let (id, slot) = app.windows.iter().next().expect("browser");
        (*id, slot.window, slot.view.upgrade().expect("view"))
    });
    let mut surface = gpui_kit::VisualTestContext::from_window(window, cx);
    surface.simulate_resize(gpui_kit::size(px(960.), px(640.)));
    surface.run_until_parked();
    surface.update(|window, cx| {
        window.activate_window();
        window.draw(cx).clear(cx);
    });
    view.update(&mut surface, |view, _| view.set_drag_hwnd_for_test(311));
    let point = surface
        .debug_bounds("archive-entry-0")
        .expect("formal row")
        .center();
    surface.simulate_event(gpui_kit::MouseDownEvent {
        position: point,
        button: gpui_kit::MouseButton::Left,
        click_count: 1,
        ..Default::default()
    });
    if !replace_before_install {
        replaced = Some(endpoint.open_connection_for_contract_test());
    }
    let receiver = replaced.expect("new connection");
    let notifications = platform.notification_receiver();
    assert_eq!(
        notifications.len(),
        1,
        "old Disconnected deliberately remains queued"
    );
    surface.simulate_event(gpui_kit::MouseMoveEvent {
        position: gpui_kit::point(point.x + px(12.), point.y),
        pressed_button: Some(gpui_kit::MouseButton::Left),
        ..Default::default()
    });
    assert!(
        receiver.try_recv().is_err(),
        "retired source MUST NOT enqueue start on replacement connection"
    );
    assert_eq!(
        notifications.len(),
        1,
        "refusal must not depend on notification delivery"
    );
    // Only AFTER the assertion deliver the queued old notification by the original deferred owner.
    let notification = notifications.try_recv().expect("old disconnect");
    let PlatformNotification::Disconnected {
        generation,
        pending_command_ids,
    } = &notification
    else {
        panic!("original disconnect");
    };
    assert_eq!(*generation, original_connection);
    assert!(
        pending_command_ids
            .iter()
            .all(|id| !endpoint.command_pending_for_contract_test(*id)),
        "old read receipts cannot remain pending on new connection"
    );
    owner.update(&mut surface, |app, cx| app.notification(notification, cx));
    surface.run_until_parked();
    owner.update(&mut surface, |app, cx| app.refresh_browser(id, 65001, cx));
    surface.run_until_parked();
    let fresh_browser = complete_connection_drag_read(&owner, &endpoint, &mut surface);
    owner.update(&mut surface, |app, cx| app.consume(cx));
    surface.run_until_parked();
    surface.update(|window, cx| {
        window.draw(cx).clear(cx);
    });
    let point = surface
        .debug_bounds("archive-entry-0")
        .expect("fresh row")
        .center();
    surface.simulate_event(gpui_kit::MouseDownEvent {
        position: point,
        button: gpui_kit::MouseButton::Left,
        click_count: 1,
        ..Default::default()
    });
    assert!(receiver.try_recv().is_err(), "down alone never starts");
    surface.simulate_event(gpui_kit::MouseMoveEvent {
        position: gpui_kit::point(point.x + px(12.), point.y),
        pressed_button: Some(gpui_kit::MouseButton::Left),
        ..Default::default()
    });
    let fresh = receiver
        .try_recv()
        .expect("fresh context starts immediately: no leaked reservation");
    assert!(endpoint.command_pending_for_contract_test(fresh.id));
    view.read_with(&surface, |view, _| assert!(view.drag_matches(fresh.id)));
    let Message::PlatformCommand(PlatformCommand::ArchiveVirtualDrag(drag)) = fresh.message else {
        panic!("formal fresh drag");
    };
    assert_eq!(
        drag.items.expect("selected")[0].entry.browser,
        Some(fresh_browser)
    );
    surface.simulate_event(gpui_kit::MouseMoveEvent {
        position: gpui_kit::point(point.x + px(14.), point.y),
        pressed_button: Some(gpui_kit::MouseButton::Left),
        ..Default::default()
    });
    assert!(
        receiver.try_recv().is_err(),
        "same held gesture submits once"
    );
}

#[gpui_kit::test]
fn production_row_drag_connection_origin_rejects_replacement_before_deferred_install(
    cx: &mut TestAppContext,
) {
    connection_origin_row_race(cx, true);
}
#[gpui_kit::test]
fn production_row_drag_connection_origin_rejects_replacement_after_prepared_down(
    cx: &mut TestAppContext,
) {
    connection_origin_row_race(cx, false);
}

#[gpui_kit::test]
fn production_password_unlock_visibility_uses_actual_read_encryption_metadata(
    cx: &mut TestAppContext,
) {
    for encrypted in [Some(false), None, Some(true)] {
        let (owner, endpoint) = setup(cx);
        owner.update(cx, |app, cx| {
            app._drain = cx.spawn(async move |_, _| {});
            app.open_canonical(PathBuf::from(r"C:\owned\visibility.zip"), 0, vec![], cx);
        });
        cx.run_until_parked();
        complete_connection_drag_read_with_encryption(&owner, &endpoint, encrypted, cx);
        owner.update(cx, |app, cx| app.consume(cx));
        cx.run_until_parked();
        let (session, window) = owner.read_with(cx, |app, _| {
            let (session, slot) = app.windows.iter().next().expect("original browser");
            (*session, slot.window)
        });
        let mut surface = gpui_kit::VisualTestContext::from_window(window, cx);
        surface.simulate_resize(gpui_kit::size(px(1100.), px(640.)));
        surface.run_until_parked();
        surface.update(|window, cx| {
            window.draw(cx).clear(cx);
            let nodes = gpui_kit::base::test_support::snapshots(window);
            assert!(nodes.iter().any(|node| node.label() == Some("查看选项")));
            let unlock = nodes
                .iter()
                .filter(|node| {
                    node.role() == Some(gpui_kit::Role::Button)
                        && node.label() == Some("解锁拖出")
                        && node.visible()
                })
                .collect::<Vec<_>>();
            assert_eq!(unlock.len(), usize::from(encrypted == Some(true)));
            if let Some(button) = unlock.first() {
                assert_eq!(button.bounds().size.height, px(64.));
                assert_ne!(button.disabled(), Some(true));
            }
        });
        assert!(
            endpoint.try_receive_command_for_contract_test().is_err(),
            "metadata and rendering must never authorize or start a drag"
        );
        owner.update(&mut surface, |app, cx| app.close_browser(session, cx));
        surface.run_until_parked();
    }
}

#[gpui_kit::test]
fn production_password_application_original_dialog_ticket_and_deferred_grant_never_start_ole(
    cx: &mut TestAppContext,
) {
    use nxtools_protocol::archive_browser::*;
    use nxtools_protocol::archive_password::*;
    let (owner, endpoint) = setup(cx);
    owner.update(cx, |app, cx| {
        app._drain = cx.spawn(async move |_, _| {});
        app.open_canonical(PathBuf::from(r"C:\owned\encrypted.zip"), 0, vec![], cx);
    });
    cx.run_until_parked();
    complete_connection_drag_read_with_encryption(&owner, &endpoint, Some(true), cx);
    owner.update(cx, |app, cx| app.consume(cx));
    cx.run_until_parked();
    let (window, view) = owner.read_with(cx, |app, _| {
        let slot = app.windows.values().next().expect("browser");
        (slot.window, slot.view.upgrade().expect("view"))
    });
    let mut surface = gpui_kit::VisualTestContext::from_window(window, cx);
    surface.simulate_resize(gpui_kit::size(px(1100.), px(640.)));
    surface.run_until_parked();
    surface.update(|w, cx| {
        w.activate_window();
        w.draw(cx).clear(cx);
    });
    let button = surface
        .debug_bounds("archive-unlock-drag")
        .expect("official unlock Button");
    assert_eq!(button.size.height, px(64.));
    assert!(
        surface.update(|window, _| {
            gpui_kit::base::test_support::snapshots(window)
                .iter()
                .any(|node| {
                    node.role() == Some(gpui_kit::Role::Button)
                        && node.label() == Some("解锁拖出")
                        && node.visible()
                        && node.disabled() != Some(true)
                        && node.bounds() == button
                })
        }),
        "actual enabled official unlock action, not only a cached debug selector"
    );
    assert!(button.left() >= px(0.) && button.right() <= px(1100.));
    assert!(button.top() >= px(0.) && button.bottom() <= px(640.));
    surface.simulate_event(gpui_kit::MouseDownEvent {
        position: button.center(),
        button: gpui_kit::MouseButton::Left,
        click_count: 1,
        ..Default::default()
    });
    surface.simulate_event(gpui_kit::MouseUpEvent {
        position: button.center(),
        button: gpui_kit::MouseButton::Left,
        click_count: 1,
        modifiers: Default::default(),
    });
    let wire = endpoint
        .try_receive_command_for_contract_test()
        .expect("button original prepare");
    let Message::PlatformCommand(PlatformCommand::ArchiveEntry(command)) = wire.message else {
        panic!("authorize");
    };
    assert_eq!(command.action, ArchiveEntryAction::Authorize);
    let prompt = ArchivePasswordPrompt {
        operation_id: command.operation_id,
        session_id: command.session_id,
        generation: command.generation,
        prompt_id: Uuid::new_v4(),
        ticket_generation: 1,
        error_code: "PasswordRequired".into(),
        attempt: 1,
    };
    owner.update(&mut surface, |app, cx| {
        app.password_prompt(prompt.clone(), cx)
    });
    surface.run_until_parked();
    surface.update(|w, cx| {
        w.draw(cx).clear(cx);
        assert!(w.has_active_dialog(cx), "same original password Dialog");
    });
    let confirm = surface
        .debug_bounds("archive-password-ok")
        .expect("same original confirm button");
    surface.simulate_event(gpui_kit::MouseDownEvent {
        position: confirm.center(),
        button: gpui_kit::MouseButton::Left,
        click_count: 1,
        ..Default::default()
    });
    surface.simulate_event(gpui_kit::MouseUpEvent {
        position: confirm.center(),
        button: gpui_kit::MouseButton::Left,
        click_count: 1,
        modifiers: Default::default(),
    });
    surface.run_until_parked();
    let reply = endpoint
        .try_receive_command_for_contract_test()
        .expect("original private answer");
    assert!(matches!(
        reply.message,
        Message::PlatformCommand(PlatformCommand::ArchivePasswordReply(
            ArchivePasswordReply {
                password: Some(_),
                ..
            }
        ))
    ));
    let result = PlatformCommandResult {
        success: true,
        error: None,
        native_error_code: None,
        data: Some(
            serde_json::to_string(&ArchiveEntryResult {
                version: 1,
                operation_id: command.operation_id,
                session_id: command.session_id,
                generation: command.generation,
                success: true,
                lease_id: Some(command.operation_id),
                path: None,
                code: None,
                native_error_code: None,
                cleanup_complete: true,
                retained: false,
            })
            .expect("result"),
        ),
    };
    owner.update(&mut surface, |app, cx| {
        app.notification(
            PlatformNotification::CommandResult {
                correlation_id: wire.id,
                result,
            },
            cx,
        );
        assert!(
            view.read(cx).authorization_matches(wire.id),
            "crossview grant publication deferred"
        );
    });
    surface.run_until_parked();
    assert!(
        endpoint.try_receive_command_for_contract_test().is_err(),
        "private reply or grant cannotauto-start"
    );
    owner.update(&mut surface, |app, cx| {
        app.close_browser(command.session_id, cx);
        app.password_reply(
            prompt,
            Some(PrivatePassword::new("late-private-fixture")),
            cx,
        );
    });
    surface.run_until_parked();
    while let Ok(wire) = endpoint.try_receive_command_for_contract_test() {
        assert!(
            !matches!(
                wire.message,
                Message::PlatformCommand(
                    PlatformCommand::ArchivePasswordReply(_)
                        | PlatformCommand::ArchiveVirtualDrag(_)
                )
            ),
            "closed late ticket cannot send secrets/start"
        );
    }
}

#[gpui_kit::test]
fn production_password_application_cancel_retires_prepare_before_late_success(
    cx: &mut TestAppContext,
) {
    use nxtools_protocol::archive_browser::*;
    use nxtools_protocol::archive_password::*;
    let (owner, endpoint) = setup(cx);
    owner.update(cx, |app, cx| {
        app._drain = cx.spawn(async move |_, _| {});
        app.open_canonical(PathBuf::from(r"C:\owned\cancel.zip"), 0, vec![], cx);
    });
    cx.run_until_parked();
    complete_connection_drag_read_with_encryption(&owner, &endpoint, Some(true), cx);
    owner.update(cx, |app, cx| app.consume(cx));
    cx.run_until_parked();
    let view = owner.read_with(cx, |app, _| {
        app.windows
            .values()
            .next()
            .expect("slot")
            .view
            .upgrade()
            .expect("view")
    });
    view.update(cx, |view, cx| view.unlock_drag(cx));
    let wire = endpoint
        .try_receive_command_for_contract_test()
        .expect("prepare");
    let Message::PlatformCommand(PlatformCommand::ArchiveEntry(command)) = wire.message else {
        panic!("prepare");
    };
    let prompt = ArchivePasswordPrompt {
        operation_id: command.operation_id,
        session_id: command.session_id,
        generation: command.generation,
        prompt_id: Uuid::new_v4(),
        ticket_generation: 1,
        error_code: "PasswordRequired".into(),
        attempt: 1,
    };
    owner.update(cx, |app, _| {
        app.passwords.insert(command.operation_id, prompt.clone());
    });
    owner.update(cx, |app, cx| app.password_reply(prompt.clone(), None, cx));
    cx.run_until_parked();
    view.read_with(cx, |view, _| {
        assert!(
            !view.authorization_matches(wire.id),
            "typedcancelretireslocalpreparation"
        )
    });
    let commands = std::iter::from_fn(|| endpoint.try_receive_command_for_contract_test().ok())
        .collect::<Vec<_>>();
    assert!(commands.iter().any(|wire|matches!(&wire.message,Message::PlatformCommand(PlatformCommand::ArchiveEntry(value)) if value.action==ArchiveEntryAction::Revoke)));
    let result = PlatformCommandResult {
        success: true,
        error: None,
        native_error_code: None,
        data: Some(
            serde_json::to_string(&ArchiveEntryResult {
                version: 1,
                operation_id: command.operation_id,
                session_id: command.session_id,
                generation: command.generation,
                success: true,
                lease_id: Some(command.operation_id),
                path: None,
                code: None,
                native_error_code: None,
                cleanup_complete: true,
                retained: false,
            })
            .expect("late result"),
        ),
    };
    owner.update(cx, |app, cx| {
        app.notification(
            PlatformNotification::CommandResult {
                correlation_id: wire.id,
                result,
            },
            cx,
        );
        app.password_reply(
            prompt,
            Some(PrivatePassword::new("late-private-answer")),
            cx,
        );
    });
    cx.run_until_parked();
    while let Ok(wire) = endpoint.try_receive_command_for_contract_test() {
        assert!(!matches!(
            wire.message,
            Message::PlatformCommand(
                PlatformCommand::ArchiveVirtualDrag(_) | PlatformCommand::ArchivePasswordReply(_)
            )
        ));
    }
}

#[gpui_kit::test]
fn production_settings_publication_is_deferred_and_preserves_window_state(cx: &mut TestAppContext) {
    let (owner, endpoint) = setup(cx);
    cx.update(|cx| cx.set_global(ArchiveGlobal(owner.clone())));
    owner.update(cx, |app, cx| {
        app.open_canonical(r"C:\owned\settings.zip".into(), 0, vec![], cx)
    });
    cx.run_until_parked();
    let (view, window, generation, reads) = owner.read_with(cx, |app, _| {
        let slot = app.windows.values().next().expect("original browser");
        (
            slot.view.clone(),
            slot.window,
            slot.generation,
            app.reads.len(),
        )
    });
    let restored = UiState {
        show_tree: false,
        ..Default::default()
    };
    cx.update(|cx| {
        let _ = window.update(cx, |_, window, cx| {
            view.update(cx, |view, cx| {
                view.apply_restored_state(restored, window, cx)
            })
            .expect("live view");
        });
    });
    let original = owner.read_with(cx, |app, _| app.settings.clone());
    let mut changed = original.clone();
    changed.show_grid_lines = true;
    let published = changed.clone();
    // A leased browser initiates publication: synchronous write-back would panic here.
    cx.update(|cx| {
        view.update(cx, |_, cx| settings_updated(published, cx))
            .expect("leased publisher");
        assert_eq!(
            owner.read(cx).settings,
            original,
            "publication waits for callback release"
        );
    });
    cx.run_until_parked();
    assert_eq!(owner.read_with(cx, |app, _| app.settings.clone()), changed);
    assert_eq!(owner.read_with(cx, |app, _| app.reads.len()), reads);
    assert_eq!(
        owner.read_with(cx, |app, _| app
            .windows
            .values()
            .next()
            .expect("same slot")
            .generation),
        generation
    );
    let state = view
        .read_with(cx, |view, _| view.snapshot_from_observed())
        .expect("snapshot");
    assert!(
        !state.show_tree,
        "unrelated change must not reset restored tree visibility"
    );
    let mut hide = changed.clone();
    hide.show_tree = false;
    cx.update(|cx| settings_updated(hide, cx));
    cx.run_until_parked();
    changed.show_tree = true;
    cx.update(|cx| settings_updated(changed, cx));
    cx.run_until_parked();
    assert!(
        view.read_with(cx, |view, _| view.snapshot_from_observed().show_tree)
            .expect("live view")
    );
    let starts = std::iter::from_fn(|| endpoint.try_receive_command_for_contract_test().ok())
        .filter(|wire| {
            matches!(
                wire.message,
                Message::PlatformCommand(PlatformCommand::ArchiveBrowserRead(_))
            )
        })
        .count();
    assert_eq!(
        starts, 1,
        "settings changes never restart the read/job owner"
    );
}

#[gpui_kit::test]
fn ui_remaining_recent_records_only_successful_top_level_reads_and_honors_clear(
    cx: &mut TestAppContext,
) {
    let (owner, endpoint) = setup(cx);
    let directory = tempfile::tempdir().expect("owned recent store");
    let path = directory.path().join("archive-ui.json");
    let store = UiStateStore::initialize(path.clone()).expect("store");
    for _ in 0..100 {
        if !matches!(store.load_status(), UiStateLoad::Loading) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    owner.update(cx, |app, cx| {
        app.install_ui_state(store, cx);
        app._drain = cx.spawn(async move |_, _| {});
        app.settings.recent_limit = 2;
        app.open_canonical(PathBuf::from(r"C:\owned\top.zip"), 0, vec![], cx);
    });
    cx.run_until_parked();
    assert!(
        owner.read_with(cx, |app, _| app.recent_paths().expect("loaded").is_empty()),
        "admission is not a successful open"
    );
    complete_connection_drag_read(&owner, &endpoint, cx);
    owner.update(cx, |app, cx| app.consume(cx));
    cx.run_until_parked();
    assert_eq!(
        owner.read_with(cx, |app, _| app.recent_paths().expect("recent")),
        vec![PathBuf::from(r"C:\owned\top.zip")]
    );
    owner.update(cx, |app, cx| {
        app.open_canonical(PathBuf::from(r"C:\owned\nested.zip"), 1, vec![], cx)
    });
    cx.run_until_parked();
    complete_connection_drag_read(&owner, &endpoint, cx);
    owner.update(cx, |app, cx| app.consume(cx));
    cx.run_until_parked();
    assert_eq!(
        owner.read_with(cx, |app, _| app.recent_paths().expect("recent").len()),
        1,
        "nested temp cannot enter history"
    );
    let receipt = owner.update(cx, |app, cx| {
        app.remember_recent(PathBuf::from(r"C:\owned\two.zip"), cx);
        app.remember_recent(PathBuf::from(r"C:\owned\three.zip"), cx);
        assert_eq!(app.recent_paths().expect("bounded").len(), 2);
        app.clear_recent_paths(cx).expect("clear")
    });
    assert!(receipt.recv_blocking().expect("durable clear"));
    assert!(owner.read_with(cx, |app, _| app.recent_paths().expect("cleared").is_empty()));
    owner.update(cx, |app, cx| {
        app.settings.remember_recent = false;
        app.remember_recent(PathBuf::from(r"C:\owned\disabled.zip"), cx);
        assert!(app.recent_paths().expect("disabled").is_empty());
    });
}
