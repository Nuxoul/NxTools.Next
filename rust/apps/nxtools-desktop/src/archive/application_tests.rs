//! Real application/window and product transport boundary, no native runtime.
use super::*;
use gpui_kit::TestAppContext;
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
        shell.message,
        Message::PlatformCommand(PlatformCommand::OpenShellTarget(_))
    ));
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
        app.password_reply(prompt, Some(PrivatePassword::new("late")));
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
