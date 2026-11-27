//! Real application/window and product transport boundary, no native runtime.
use super::*;
use gpui_kit::TestAppContext;
use gpui_kit::component::WindowExt as _;
use nxtools_protocol::{Envelope, Message, PlatformCommand, PlatformCommandResult};
#[path = "ordinary_presentation_fixture.rs"]
mod ordinary_presentation_fixture;
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
    let (owner, endpoint) = setup_unprepared(cx);
    prepare_presentations(&owner, cx);
    (owner, endpoint)
}
pub(super) fn prepare_presentations(owner: &Entity<ArchiveApplication>, cx: &mut TestAppContext) {
    let platform = owner.read_with(cx, |app, _| app.platform.clone());
    cx.update(|cx| ordinary_presentation_fixture::install(platform, cx));
}
pub(super) fn setup_unprepared(
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
        sevenz_edit: None,
        tar_edit: None,
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
        .try_receive_business_command()
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
    let mut matching_releases = 0;
    while let Ok(release) = endpoint.try_receive_command_for_contract_test() {
        if matches!(release.message, Message::PlatformCommand(PlatformCommand::ArchiveEntry(ArchiveEntryCommand { action: ArchiveEntryAction::Release, operation_id, lease_id: Some(value), .. })) if operation_id == command.operation_id && value == lease_id)
        {
            matching_releases += 1;
        }
    }
    assert_eq!(
        matching_releases, 1,
        "retirement releases exact opened lease once amid passive preview cancellation"
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
    prepare_presentations(&owner, cx);
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
    batch.batch=Some(ArchiveReadBatch {snapshot_id:snapshot,sequence:0,format:"Zip".into(),entries:[(0,"drag.txt",3),(1,"second.txt",7)].into_iter().map(|(ordinal,name,size)| serde_json::from_value(serde_json::json!({"ordinal":ordinal,"name":name,"kind":"File","size":size,"packed_size":size,"encrypted":false,"warning_codes":[],"provenance":"Utf8Flag","codepage":65001})).expect("entry")).collect()});
    deliver(&endpoint, batch, cx);
    let _ack = endpoint
        .try_receive_command_for_contract_test()
        .expect("credit");
    let browser = ArchiveBrowserContext {
        snapshot_id: snapshot,
        format: "Zip".into(),
        comment: String::new(),
        sevenz_edit: None,
        tar_edit: None,
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
        files_verified: 2,
        bytes_verified: 0,
        entry_count: 2,
        batch_count: 1,
        elapsed_ms: 1,
        native_error_code: None,
        cleanup_complete: true,
        transfer: Some(ArchiveReadTransfer {
            private_bytes: 4096,
            received_bytes: 4096,
            max_batch_entries: 2,
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
    surface.simulate_click(point, gpui_kit::Modifiers::default());
    surface.simulate_keystrokes("ctrl-a");
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
    let assert_no_content_command = |wire: &Envelope| match &wire.message {
        Message::PlatformCommand(PlatformCommand::ArchiveJob(_)) => {
            panic!("no extraction before the locate receipt")
        }
        Message::PlatformCommand(PlatformCommand::ArchiveEntry(command)) => {
            assert_ne!(command.action, ArchiveEntryAction::Materialize);
        }
        _ => {}
    };
    let (wire, drag) = loop {
        let wire = endpoint
            .try_receive_business_command()
            .expect("formal row command");
        assert_no_content_command(&wire);
        if let Message::PlatformCommand(PlatformCommand::ArchiveVirtualDrag(drag)) = &wire.message {
            break (wire.clone(), drag.clone());
        }
    };
    assert_eq!(drag.action, "locate");
    let items = drag.items.as_ref().expect("direct-drop marker");
    assert_eq!(items.len(), 1, "one marker to locate the receiver parent");
    let marker = &items[0];
    assert_eq!(marker.entry.browser, Some(browser.clone()));
    assert!(!marker.directory);
    assert_eq!(marker.name, "selected-files");
    assert_eq!(marker.size, Some(0));
    assert_eq!(marker.entry.ordinal, Some(0), "first frozen file ordinal");
    assert_eq!(drag.source_window_handle, 311);
    let drag_progress = nxtools_protocol::archive_virtual_drag::ArchiveVirtualDragProgress {
        version: 1,
        operation_id: drag.operation_id,
        session_id: drag.session_id,
        generation: drag.generation,
        phase: "materializing".into(),
        bytes_done: 0,
        bytes_total: Some(10),
        files_done: 0,
        files_total: 2,
        current_index: Some(0),
        current_member: Some("drag.txt".into()),
        current_request_id: Some(marker.entry.operation_id),
        current_bytes_done: Some(1),
        current_bytes_total: Some(3),
        bytes_processed: Some(1),
        sequence: Some(1),
        cleanup_complete: None,
        code: None,
        native_error_code: None,
    };
    drag_progress
        .validate()
        .expect("well-formed content progress for the marker identity");
    let before_progress = surface.update(|_, cx| cx.windows());
    r45_deliver_drag_progress(&owner, drag_progress, &mut surface);
    assert!(
        owner.read_with(&surface, |app, _| app.jobs.drag_progress.is_none()
            && app.jobs.records.is_empty()),
        "locate has no virtual content progress"
    );
    assert_eq!(surface.update(|_, cx| cx.windows()), before_progress);
    while let Ok(wire) = endpoint.try_receive_business_command() {
        assert_no_content_command(&wire);
    }
    let result = PlatformCommandResult {
        success: true,
        error: None,
        native_error_code: None,
        data: Some(
            serde_json::json!({"status":"located","destination":r"D:\out","drop_effect":1})
                .to_string(),
        ),
    };
    endpoint
        .route_command_result_for_contract_test(wire.id, result.clone())
        .expect("original locate router receipt");
    owner.update(&mut surface, |app, cx| {
        app.notification(
            PlatformNotification::CommandResult {
                correlation_id: wire.id,
                result,
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
    owner.read_with(&surface, |app, _| {
        assert_eq!(
            app.jobs.records.len(),
            1,
            "one ordinary extraction after deferred location"
        );
        let request = app
            .jobs
            .records
            .values()
            .next()
            .expect("job")
            .command
            .request
            .as_ref()
            .expect("extract request");
        assert_eq!(request.destination.as_deref(), Some(r"D:\out"));
        let options = request.extract.as_ref().expect("options");
        assert_eq!(
            options.strategy,
            nxtools_protocol::archive_job::ArchiveJobStrategy::Here
        );
        let selection = options.selection.as_ref().expect("frozen whole selection");
        assert_eq!(selection.browser, browser);
        assert_eq!(selection.ordinals, vec![0, 1]);
        assert!(selection.strip_prefix.is_none());
    });
}

fn r45_deliver_drag_progress(
    owner: &Entity<ArchiveApplication>,
    event: nxtools_protocol::archive_virtual_drag::ArchiveVirtualDragProgress,
    surface: &mut gpui_kit::VisualTestContext,
) {
    let serialized = serde_json::to_vec(&Envelope::new(Message::PlatformEvent(
        nxtools_protocol::PlatformEvent::ArchiveVirtualDragProgress(Box::new(event)),
    )))
    .expect("native progress wire");
    let decoded: Envelope = serde_json::from_slice(&serialized).expect("decoded progress");
    let Message::PlatformEvent(event) = decoded.message else {
        panic!("progress event")
    };
    owner.update(surface, |app, cx| {
        app.notification(PlatformNotification::Event(event), cx)
    });
    surface.run_until_parked();
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
        .try_receive_business_command()
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
        sevenz_edit: None,
        tar_edit: None,
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
        receive_business_command(|| receiver.try_recv().map_err(|e| e.to_string())).is_err(),
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
    assert!(
        receive_business_command(|| receiver.try_recv().map_err(|e| e.to_string())).is_err(),
        "down alone never starts"
    );
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
    while let Ok(extra) =
        receive_business_command(|| receiver.try_recv().map_err(|e| e.to_string()))
    {
        assert!(
            !matches!(
                extra.message,
                Message::PlatformCommand(PlatformCommand::ArchiveVirtualDrag(_))
            ),
            "same held gesture submits once; passive preview is independent"
        );
    }
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
fn production_ordinary_drop_never_requires_a_visible_password_unlock_action(
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
            assert!(
                unlock.is_empty(),
                "password is requested by ordinary extraction after the drop"
            );
        });
        assert!(
            endpoint.try_receive_business_command().is_err(),
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
    assert!(surface.debug_bounds("archive-unlock-drag").is_none());
    view.update(&mut surface, |view, cx| view.unlock_drag(cx));
    let wire = endpoint
        .try_receive_business_command()
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
    let wire = endpoint.try_receive_business_command().expect("prepare");
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

#[gpui_kit::test]
fn render_repair_shared_type_icon_result_uses_original_deferred_application_route(
    cx: &mut TestAppContext,
) {
    let (owner, endpoint) = setup(cx);
    owner.update(cx, |app, cx| {
        app.open_canonical(PathBuf::from(r"C:\owned\icons.zip"), 0, vec![], cx)
    });
    cx.run_until_parked();
    let _read = endpoint
        .try_receive_command_for_contract_test()
        .expect("original browser read");
    let (view, handle) = owner.read_with(cx, |app, _| {
        let slot = app
            .windows
            .values()
            .next()
            .expect("original browser window");
        (slot.view.upgrade().expect("view"), slot.window)
    });
    let mut surface = gpui_kit::VisualTestContext::from_window(handle, cx);
    surface.update(|window, cx| view.update(cx, |view, cx| {
        let metadata = serde_json::from_value(serde_json::json!({"ordinal":0,"name":"readme.txt","kind":"File","size":1,"packed_size":1,"encrypted":false,"warning_codes":[],"provenance":"Utf8Flag","codepage":65001})).expect("synthetic file");
        view.install(Uuid::new_v4(), vec![metadata], "Zip".into(), window, cx);
    }));
    surface.update(|window, cx| {
        window.draw(cx).clear(cx);
    });
    surface.run_until_parked();
    let command = endpoint
        .try_receive_command_for_contract_test()
        .expect("shared icon request from real created window");
    assert!(matches!(
        command.message,
        Message::PlatformCommand(PlatformCommand::ExtractShellIcon(_))
    ));
    let directory = tempfile::tempdir().expect("owned icon");
    let image = directory.path().join("native.png");
    std::fs::write(&image, crate::icons::ONE_PIXEL_PNG).expect("fixture PNG");
    owner.update(&mut surface, |app, cx| {
        app.notification(
            PlatformNotification::CommandResult {
                correlation_id: command.id,
                result: PlatformCommandResult {
                    success: true,
                    error: None,
                    native_error_code: None,
                    data: Some(image.to_string_lossy().into_owned()),
                },
            },
            cx,
        )
    });
    surface.run_until_parked();
    surface.update(|window, cx| {
        window.draw(cx).clear(cx);
    });
    assert!(
        surface.debug_bounds("archive-shell-type-icon").is_some(),
        "original notification router must deliver image next turn"
    );
}

// Business lifecycle tests tolerate ONLY the new read-only type-icon command. All other
// commands retain their original ordering/assertions; real icon-route tests use the raw queue.
trait BusinessCommands {
    fn try_receive_business_command(&self) -> Result<nxtools_protocol::Envelope, String>;
}
impl BusinessCommands for nxtools_ipc::PlatformServerEndpoint {
    fn try_receive_business_command(&self) -> Result<nxtools_protocol::Envelope, String> {
        receive_business_command(|| self.try_receive_command_for_contract_test())
    }
}
fn receive_business_command(
    mut receive: impl FnMut() -> Result<nxtools_protocol::Envelope, String>,
) -> Result<nxtools_protocol::Envelope, String> {
    for _ in 0..=8 {
        let wire = receive()?;
        if let Message::PlatformCommand(PlatformCommand::ExtractShellIcon(icon)) = &wire.message {
            assert_eq!(icon.size, 32);
            let extension = icon
                .target
                .strip_prefix("nxtools-file-type:.")
                .expect("extension-only icon authority");
            assert!(
                !extension.is_empty()
                    && extension.len() <= 16
                    && extension.bytes().all(|b| b.is_ascii_alphanumeric())
            );
            continue;
        }
        return Ok(wire);
    }
    panic!("type icon requests exceeded the8pending bound");
}

pub(super) fn preview_fixture(
    cx: &mut TestAppContext,
) -> (
    Entity<ArchiveApplication>,
    nxtools_ipc::PlatformServerEndpoint,
    Uuid,
    Uuid,
) {
    let (owner, endpoint) = setup(cx);
    owner.update(cx, |app, cx| {
        app.open_canonical(PathBuf::from(r"C:\owned\preview.zip"), 0, vec![], cx)
    });
    cx.run_until_parked();
    let snapshot = Uuid::new_v4();
    let id = owner.update(cx, |app, cx| {
        app.reads.clear();
        let (id, slot) = app.windows.iter_mut().next().expect("browser");
        slot.browser = Some(ArchiveBrowserContext { snapshot_id: snapshot, format: "Zip".into(), comment: String::new(),
            sevenz_edit: None,
            tar_edit: None,
            source: nxtools_protocol::archive_browser::ArchiveSourceBinding { native: "v1:fixture".into(), digest: [0;32], size: 1, modified_unix_ns: 0 } });
        let view = slot.view.upgrade().expect("view");
        let platform=app.platform.clone();let browser=slot.browser.clone().expect("installed context");
        let connection=platform.connection_generation().expect("original read connection");
        slot.window.update(cx, |_, window, cx| view.update(cx, |view, cx| {
            let entries = (0..3).map(|ordinal| serde_json::from_value(serde_json::json!({"ordinal":ordinal,"name":format!("item{ordinal}.txt"),"kind":"File","size":null,"packed_size":null,"encrypted":false,"warning_codes":[],"provenance":"Utf8Flag","codepage":65001})).expect("metadata")).collect();
            view.install(snapshot, entries, "Zip".into(), window, cx);
            view.configure_drag(platform,browser,connection,0,false);
        })).expect("window");
        *id
    });
    while endpoint.try_receive_command_for_contract_test().is_ok() {}
    (owner, endpoint, id, snapshot)
}
fn preview_select(
    owner: &Entity<ArchiveApplication>,
    id: Uuid,
    row: super::super::model::RowId,
    cx: &mut TestAppContext,
) -> u64 {
    owner.update(cx, |app, cx| {
        let slot = app.windows.get(&id).expect("browser");
        slot.view.upgrade().expect("view").update(cx, |view, cx| {
            let model = view.model.as_mut().expect("model");
            model.selected.clear();
            model.selected.insert(row);
            view.prepare_preview_for_test(cx)
        })
    })
}
fn preview_reply(
    command: &nxtools_protocol::archive_browser::ArchiveEntryCommand,
    lease: Uuid,
    path: String,
) -> PlatformCommandResult {
    PlatformCommandResult {
        success: true,
        error: None,
        native_error_code: None,
        data: Some(
            serde_json::to_string(&nxtools_protocol::archive_browser::ArchiveEntryResult {
                version: 1,
                operation_id: command.operation_id,
                session_id: command.session_id,
                generation: command.generation,
                success: true,
                lease_id: Some(lease),
                path: Some(path),
                code: None,
                native_error_code: None,
                cleanup_complete: true,
                retained: false,
            })
            .expect("terminal"),
        ),
    }
}
fn next_preview(
    endpoint: &nxtools_ipc::PlatformServerEndpoint,
) -> (Uuid, nxtools_protocol::archive_browser::ArchiveEntryCommand) {
    for _ in 0..16 {
        let wire = endpoint
            .try_receive_command_for_contract_test()
            .expect("preview command");
        if let Message::PlatformCommand(PlatformCommand::ArchiveEntry(command)) = wire.message {
            if matches!(
                command.action,
                nxtools_protocol::archive_browser::ArchiveEntryAction::Preview
                    | nxtools_protocol::archive_browser::ArchiveEntryAction::PreviewText
            ) {
                return (wire.id, command);
            }
            assert_eq!(
                command.action,
                nxtools_protocol::archive_browser::ArchiveEntryAction::Cancel
            );
        }
    }
    panic!("preview command bound");
}
#[gpui_kit::test]
fn preview_latest_only_waits_for_actual_native_cleanup_and_releases_stale_lease(
    cx: &mut TestAppContext,
) {
    use super::super::model::RowId;
    use nxtools_protocol::archive_browser::ArchiveEntryAction;
    let (owner, endpoint, id, snapshot) = preview_fixture(cx);
    let row = RowId::Entry(snapshot, 0);
    let token = preview_select(&owner, id, row.clone(), cx);
    owner.update(cx, |app, cx| app.preview_entry(id, Some(row), token, cx));
    let (receipt, first) = next_preview(&endpoint);
    assert_eq!(first.action, ArchiveEntryAction::PreviewText);
    assert!(first.validate().is_ok());
    for ordinal in [1, 2] {
        let row = RowId::Entry(snapshot, ordinal);
        let token = preview_select(&owner, id, row.clone(), cx);
        owner.update(cx, |app, cx| app.preview_entry(id, Some(row), token, cx));
    }
    while let Ok(wire) = endpoint.try_receive_command_for_contract_test() {
        assert!(
            matches!(wire.message, Message::PlatformCommand(PlatformCommand::ArchiveEntry(command)) if command.action == ArchiveEntryAction::Cancel)
        );
    }
    let lease = Uuid::new_v4();
    owner.update(cx, |app, cx| {
        assert!(app.entry_result(
            receipt,
            &preview_reply(&first, lease, r"C:\never-read-stale.txt".into()),
            cx
        ));
    });
    let release = endpoint
        .try_receive_command_for_contract_test()
        .expect("release before latest");
    assert!(
        matches!(release.message, Message::PlatformCommand(PlatformCommand::ArchiveEntry(command)) if command.action == ArchiveEntryAction::Release && command.lease_id == Some(lease))
    );
    let (_, latest) = next_preview(&endpoint);
    assert_eq!(latest.ordinal, Some(2));
    assert!(
        endpoint.try_receive_command_for_contract_test().is_err(),
        "only latest, no Shell"
    );
    owner.update(cx, |app, cx| app.close_browser(id, cx));
}
#[gpui_kit::test]
fn preview_ui_token_rejects_aba_before_actual_dispatch(cx: &mut TestAppContext) {
    use super::super::model::RowId;
    let (owner, endpoint, id, snapshot) = preview_fixture(cx);
    let row = RowId::Entry(snapshot, 0);
    let old = preview_select(&owner, id, row.clone(), cx);
    preview_select(&owner, id, RowId::Entry(snapshot, 1), cx);
    let current = preview_select(&owner, id, row.clone(), cx);
    assert_ne!(old, current);
    owner.update(cx, |app, cx| {
        app.preview_entry(id, Some(row.clone()), old, cx)
    });
    assert!(endpoint.try_receive_command_for_contract_test().is_err());
    owner.update(cx, |app, cx| app.preview_entry(id, Some(row), current, cx));
    let (_, command) = next_preview(&endpoint);
    assert_eq!(command.ordinal, Some(0));
    owner.update(cx, |app, cx| app.close_browser(id, cx));
}
#[gpui_kit::test]
fn preview_close_then_native_completion_removes_state_and_never_reads_or_opens(
    cx: &mut TestAppContext,
) {
    use super::super::model::RowId;
    use nxtools_protocol::archive_browser::ArchiveEntryAction;
    let (owner, endpoint, id, snapshot) = preview_fixture(cx);
    let row = RowId::Entry(snapshot, 0);
    let token = preview_select(&owner, id, row.clone(), cx);
    owner.update(cx, |app, cx| app.preview_entry(id, Some(row), token, cx));
    let (receipt, command) = next_preview(&endpoint);
    owner.update(cx, |app, cx| app.close_browser(id, cx));
    while endpoint.try_receive_command_for_contract_test().is_ok() {}
    let lease = Uuid::new_v4();
    owner.update(cx, |app, cx| {
        app.entry_result(
            receipt,
            &preview_reply(&command, lease, r"C:\not-read.txt".into()),
            cx,
        );
        assert!(!app.previews.contains_key(&id) && app.preview_entries.is_empty());
    });
    let wire = endpoint
        .try_receive_command_for_contract_test()
        .expect("release");
    assert!(
        matches!(wire.message, Message::PlatformCommand(PlatformCommand::ArchiveEntry(command)) if command.action == ArchiveEntryAction::Release && command.lease_id == Some(lease))
    );
    assert!(endpoint.try_receive_command_for_contract_test().is_err());
}
#[gpui_kit::test]
fn preview_decode_holds_lease_until_actual_worker_returns_even_after_close(
    cx: &mut TestAppContext,
) {
    use super::super::model::RowId;
    use nxtools_protocol::archive_browser::ArchiveEntryAction;
    let (owner, endpoint, id, snapshot) = preview_fixture(cx);
    let row = RowId::Entry(snapshot, 0);
    let token = preview_select(&owner, id, row.clone(), cx);
    owner.update(cx, |app, cx| app.preview_entry(id, Some(row), token, cx));
    let (receipt, command) = next_preview(&endpoint);
    let directory = tempfile::tempdir().expect("owned source");
    let path = directory.path().join("item.txt");
    std::fs::write(&path, "safe text").expect("owned bytes");
    let (send, receive) = async_channel::bounded(1);
    owner.update(cx, |app, _| app.preview_decode_barrier = Some(receive));
    let lease = Uuid::new_v4();
    owner.update(cx, |app, cx| {
        app.entry_result(
            receipt,
            &preview_reply(&command, lease, path.to_string_lossy().into_owned()),
            cx,
        );
    });
    cx.run_until_parked();
    assert!(
        endpoint.try_receive_command_for_contract_test().is_err(),
        "lease held while decode in flight"
    );
    owner.update(cx, |app, cx| app.close_browser(id, cx));
    assert!(owner.read_with(cx, |app, _| {
        app.previews.get(&id).expect("draining").running.is_some()
    }));
    send.try_send(()).expect("release worker");
    cx.run_until_parked();
    let wire = endpoint
        .try_receive_command_for_contract_test()
        .expect("actual decode release");
    assert!(
        matches!(wire.message, Message::PlatformCommand(PlatformCommand::ArchiveEntry(command)) if command.action == ArchiveEntryAction::Release && command.lease_id == Some(lease))
    );
    assert!(owner.read_with(cx, |app, _| !app.previews.contains_key(&id)));
    assert!(
        endpoint.try_receive_command_for_contract_test().is_err(),
        "no Shell or replacement"
    );
}

#[gpui_kit::test]
fn preview_unclean_native_reply_blocks_replacement_but_closed_state_is_removed(
    cx: &mut TestAppContext,
) {
    use super::super::model::RowId;
    use nxtools_protocol::archive_browser::ArchiveEntryResult;
    let (owner, endpoint, id, snapshot) = preview_fixture(cx);
    let row = RowId::Entry(snapshot, 0);
    let token = preview_select(&owner, id, row.clone(), cx);
    owner.update(cx, |app, cx| app.preview_entry(id, Some(row), token, cx));
    let (receipt, command) = next_preview(&endpoint);
    let mut outer = preview_reply(&command, Uuid::new_v4(), r"C:\not-read.txt".into());
    let mut terminal: ArchiveEntryResult =
        serde_json::from_str(outer.data.as_deref().expect("wire")).expect("terminal");
    terminal.cleanup_complete = false;
    terminal.lease_id = None;
    outer.data = Some(serde_json::to_string(&terminal).expect("unclean"));
    owner.update(cx, |app, cx| {
        app.entry_result(receipt, &outer, cx);
    });
    let row = RowId::Entry(snapshot, 1);
    let token = preview_select(&owner, id, row.clone(), cx);
    owner.update(cx, |app, cx| app.preview_entry(id, Some(row), token, cx));
    assert!(
        endpoint.try_receive_command_for_contract_test().is_err(),
        "no replacement before actualcleanup"
    );
    owner.update(cx, |app, cx| {
        app.close_browser(id, cx);
        assert!(!app.previews.contains_key(&id));
    });
}

#[gpui_kit::test]
fn preview_disconnect_clears_native_ownership_and_never_restarts(cx: &mut TestAppContext) {
    use super::super::model::RowId;
    let (owner, endpoint, id, snapshot) = preview_fixture(cx);
    let row = RowId::Entry(snapshot, 0);
    let token = preview_select(&owner, id, row.clone(), cx);
    owner.update(cx, |app, cx| app.preview_entry(id, Some(row), token, cx));
    let (receipt, _) = next_preview(&endpoint);
    owner.update(cx, |app, cx| {
        app.notification(
            PlatformNotification::Disconnected {
                generation: 1,
                pending_command_ids: vec![receipt],
            },
            cx,
        );
        assert!(
            app.preview_entries.is_empty()
                && app.preview_prompts.is_empty()
                && app.previews.is_empty()
        );
    });
    assert!(endpoint.try_receive_command_for_contract_test().is_err());
    owner.update(cx, |app, cx| app.close_browser(id, cx));
}

#[gpui_kit::test]
fn preview_cancel_dismisses_only_its_password_dialog_not_a_new_overlay(cx: &mut TestAppContext) {
    use super::super::model::RowId;
    use nxtools_protocol::archive_password::ArchivePasswordPrompt;
    let (owner, endpoint, id, snapshot) = preview_fixture(cx);
    let row = RowId::Entry(snapshot, 0);
    let token = preview_select(&owner, id, row.clone(), cx);
    owner.update(cx, |app, cx| app.preview_entry(id, Some(row), token, cx));
    let (_, command) = next_preview(&endpoint);
    let prompt = ArchivePasswordPrompt {
        operation_id: command.operation_id,
        session_id: id,
        generation: command.generation,
        prompt_id: Uuid::new_v4(),
        ticket_generation: 1,
        error_code: "PasswordRequired".into(),
        attempt: 1,
    };
    owner.update(cx, |app, cx| {
        assert!(app.preview_password_prompt(prompt.clone(), cx))
    });
    cx.run_until_parked();
    let window = owner.read_with(cx, |app, _| app.windows[&id].window);
    cx.update(|cx| {
        window
            .update(cx, |_, window, cx| {
                assert!(window.has_active_dialog(cx));
                window.open_dialog(cx, |dialog, _, _| dialog.title("unrelated fixture dialog"));
            })
            .expect("window")
    });
    owner.update(cx, |app, cx| app.cancel_preview(id, cx));
    cx.run_until_parked();
    assert!(owner.read_with(
        cx,
        |app, _| !app.passwords.contains_key(&command.operation_id)
            && app.preview_prompts.is_empty()
    ));
    cx.update(|cx| {
        window
            .update(cx, |_, window, cx| {
                assert!(window.has_active_dialog(cx), "unrelated overlay preserved")
            })
            .expect("window")
    });
    // An old prompt racing selection cancellation is consumed without showing/focus effects.
    owner.update(cx, |app, cx| {
        assert!(app.preview_password_prompt(prompt, cx))
    });
    assert!(owner.read_with(cx, |app, _| app.preview_prompts.is_empty()));
    owner.update(cx, |app, cx| app.close_browser(id, cx));
}

#[gpui_kit::test]
fn preview_source_changed_invalidates_same_source_even_after_selection_switch(
    cx: &mut TestAppContext,
) {
    use super::super::model::RowId;
    use nxtools_protocol::archive_browser::ArchiveEntryResult;
    let (owner, endpoint, id, snapshot) = preview_fixture(cx);
    let row = RowId::Entry(snapshot, 0);
    let token = preview_select(&owner, id, row.clone(), cx);
    owner.update(cx, |app, cx| app.preview_entry(id, Some(row), token, cx));
    let (receipt, command) = next_preview(&endpoint);
    let row = RowId::Entry(snapshot, 1);
    let token = preview_select(&owner, id, row.clone(), cx);
    owner.update(cx, |app, cx| app.preview_entry(id, Some(row), token, cx));
    while endpoint.try_receive_command_for_contract_test().is_ok() {}
    let terminal = ArchiveEntryResult {
        version: 1,
        operation_id: command.operation_id,
        session_id: id,
        generation: command.generation,
        success: false,
        lease_id: None,
        path: None,
        code: Some(ArchiveReadError::SourceChanged),
        native_error_code: None,
        cleanup_complete: true,
        retained: false,
    };
    let outer = PlatformCommandResult {
        success: false,
        error: None,
        native_error_code: None,
        data: Some(serde_json::to_string(&terminal).expect("source changed")),
    };
    owner.update(cx, |app, cx| {
        app.entry_result(receipt, &outer, cx);
        assert!(app.windows[&id].browser.is_none());
    });
    assert!(
        endpoint.try_receive_command_for_contract_test().is_err(),
        "latest cannot use invalid source"
    );
    owner.update(cx, |app, cx| app.close_browser(id, cx));
}

#[gpui_kit::test]
fn preview_explicit_open_cancels_passive_work_and_keeps_original_materialize(
    cx: &mut TestAppContext,
) {
    use super::super::model::RowId;
    use nxtools_protocol::archive_browser::ArchiveEntryAction;
    let (owner, endpoint, id, snapshot) = preview_fixture(cx);
    let row = RowId::Entry(snapshot, 0);
    let token = preview_select(&owner, id, row.clone(), cx);
    owner.update(cx, |app, cx| {
        app.preview_entry(id, Some(row.clone()), token, cx)
    });
    let (receipt, passive) = next_preview(&endpoint);
    owner.update(cx, |app, cx| app.open_entry(id, row, cx));
    let cancel = endpoint
        .try_receive_command_for_contract_test()
        .expect("passive preview cancellation");
    assert!(
        matches!(cancel.message,Message::PlatformCommand(PlatformCommand::ArchiveEntry(command)) if command.action==ArchiveEntryAction::Cancel&&command.operation_id==passive.operation_id),
        "explicit open first retires original passive operation"
    );
    let open = endpoint
        .try_receive_command_for_contract_test()
        .expect("original explicit materialization");
    assert!(
        matches!(open.message,Message::PlatformCommand(PlatformCommand::ArchiveEntry(command)) if command.action==ArchiveEntryAction::Materialize&&command.ordinal==Some(0)),
        "doubleclick retains original open purpose, never preview-lease Shell authority"
    );
    let lease = Uuid::new_v4();
    owner.update(cx, |app, cx| {
        app.entry_result(
            receipt,
            &preview_reply(&passive, lease, r"C:\never-read-cancelled.txt".into()),
            cx,
        );
    });
    let release = endpoint
        .try_receive_command_for_contract_test()
        .expect("cancelled passive lease release");
    assert!(
        matches!(release.message,Message::PlatformCommand(PlatformCommand::ArchiveEntry(command)) if command.action==ArchiveEntryAction::Release&&command.lease_id==Some(lease))
    );
    assert!(
        endpoint.try_receive_command_for_contract_test().is_err(),
        "passive completion cannot externally open"
    );
}

#[gpui_kit::test]
fn browser_open_requests_share_host_and_duplicate_does_not_reread(cx: &mut TestAppContext) {
    let (owner, endpoint) = setup(cx);
    let first = PathBuf::from(r"C:\owned\first.zip");
    let second = PathBuf::from(r"C:\owned\second.7z");
    owner.update(cx, |app, cx| {
        app.open_canonical(first.clone(), 0, vec![], cx)
    });
    cx.run_until_parked();
    let first_window = owner.read_with(cx, |app, _| app.windows[&app.paths[&first]].window);
    let first_read = endpoint
        .try_receive_command_for_contract_test()
        .expect("first read");
    assert!(matches!(
        first_read.message,
        Message::PlatformCommand(PlatformCommand::ArchiveBrowserRead(_))
    ));
    owner.update(cx, |app, cx| {
        app.open_canonical(second.clone(), 0, vec![], cx)
    });
    cx.run_until_parked();
    let second_window = owner.read_with(cx, |app, _| app.windows[&app.paths[&second]].window);
    assert_eq!(
        first_window, second_window,
        "different archives must use tabs in the same host HWND"
    );
    let second_read = endpoint
        .try_receive_command_for_contract_test()
        .expect("second read");
    assert!(matches!(
        second_read.message,
        Message::PlatformCommand(PlatformCommand::ArchiveBrowserRead(_))
    ));
    owner.update(cx, |app, cx| {
        app.open_canonical(first.clone(), 0, vec![], cx)
    });
    cx.run_until_parked();
    assert_eq!(owner.read_with(cx, |app, _| app.windows.len()), 2);
    assert!(
        endpoint.try_receive_command_for_contract_test().is_err(),
        "duplicate open must not restart metadata IO"
    );
    let ids = owner.read_with(cx, |app, _| app.windows.keys().copied().collect::<Vec<_>>());
    for id in ids {
        owner.update(cx, |app, cx| app.close_browser(id, cx));
    }
    cx.run_until_parked();
}

#[gpui_kit::test]
fn browser_tabs_pointer_selection_and_close_preserve_other_session(cx: &mut TestAppContext) {
    use gpui_kit::{MouseButton, MouseDownEvent, MouseUpEvent, VisualTestContext};
    let (owner, _native) = setup(cx);
    let first = PathBuf::from(r"C:\owned\first.zip");
    let second = PathBuf::from(r"C:\owned\second.7z");
    owner.update(cx, |app, cx| {
        app.open_canonical(first.clone(), 0, vec![], cx);
        app.open_canonical(second.clone(), 0, vec![], cx);
    });
    cx.run_until_parked();
    let (host, window, first_id, second_id) = owner.read_with(cx, |app, _| {
        let host = app.host.as_ref().expect("shared host");
        (
            host.view.upgrade().expect("host entity"),
            host.window,
            app.paths[&first],
            app.paths[&second],
        )
    });
    let mut surface = VisualTestContext::from_window(window, cx);
    surface.update(|window, cx| {
        window.activate_window();
        window.draw(cx).clear(cx);
    });
    let tab_center = surface.update(|window, _| {
        gpui_kit::base::test_support::snapshots(window)
            .into_iter()
            .find(|node| {
                node.role() == Some(gpui_kit::accesskit::Role::Tab)
                    && node.label() == Some("first.zip")
            })
            .expect("actual official first tab")
            .bounds()
            .center()
    });
    surface.simulate_event(MouseDownEvent {
        position: tab_center,
        button: MouseButton::Left,
        click_count: 1,
        ..Default::default()
    });
    surface.simulate_event(MouseUpEvent {
        position: tab_center,
        button: MouseButton::Left,
        click_count: 1,
        modifiers: Default::default(),
    });
    surface.run_until_parked();
    assert_eq!(
        host.read_with(&surface, |host, _| host.active),
        Some(first_id)
    );
    surface.update(|window, cx| {
        window.draw(cx).clear(cx);
    });
    let close_center = surface.update(|window, _| {
        let nodes = gpui_kit::base::test_support::snapshots(window);
        let tab = nodes
            .iter()
            .find(|node| {
                node.role() == Some(gpui_kit::Role::Tab) && node.label() == Some("first.zip")
            })
            .expect("current-frame exact first tab")
            .bounds();
        nodes
            .iter()
            .find(|node| {
                node.label() == Some("关闭压缩包标签页") && tab.contains(&node.bounds().center())
            })
            .expect("official close inside first tab")
            .bounds()
            .center()
    });
    surface.simulate_event(MouseDownEvent {
        position: close_center,
        button: MouseButton::Left,
        click_count: 1,
        ..Default::default()
    });
    surface.simulate_event(MouseUpEvent {
        position: close_center,
        button: MouseButton::Left,
        click_count: 1,
        modifiers: Default::default(),
    });
    surface.run_until_parked();
    owner.read_with(&surface, |app, _| {
        assert!(!app.windows.contains_key(&first_id));
        assert!(app.windows.contains_key(&second_id));
        assert_eq!(app.windows[&second_id].window, window);
        assert!(app.reads.values().all(|read| read.window_id != first_id));
    });
    assert_eq!(
        host.read_with(&surface, |host, _| host.active),
        Some(second_id)
    );
    owner.update(&mut surface, |app, cx| app.close_browser(second_id, cx));
    surface.run_until_parked();
    assert!(owner.read_with(&surface, |app, _| app.host.is_none()
        && app.windows.is_empty()));
}

#[gpui_kit::test]
fn browser_tabs_launch_request_acknowledges_before_metadata_and_reuses_canonical_source(
    cx: &mut TestAppContext,
) {
    use nxtools_protocol::desktop_archive_intent::{
        ArchiveIntentAction, DesktopArchiveIntent, LaunchReceiptState,
    };
    let (owner, _native) = setup(cx);
    cx.update(|cx| cx.set_global(ArchiveGlobal(owner.clone())));
    let files = tempfile::tempdir().expect("owned launch sources");
    let first = files.path().join("one.zip");
    let second = files.path().join("two.7z");
    std::fs::write(&first, b"metadata transport is controlled").expect("first source");
    std::fs::write(&second, b"metadata transport is controlled").expect("second source");
    let (endpoint, receiver) = nxtools_ipc::desktop_archive_intent::archive_intent_channel();
    let mut handles = Vec::new();
    for path in [&first, &second, &first] {
        let intent = DesktopArchiveIntent {
            version: 1,
            request_id: Uuid::new_v4(),
            action: ArchiveIntentAction::Open,
            paths: vec![path.to_string_lossy().into_owned()],
            mode: None,
        };
        endpoint.submit(intent.clone());
        let (request, completion) = receiver
            .try_next()
            .expect("original bounded launch ingress");
        cx.update(|cx| handle_launch_intent(request, completion, cx));
        cx.run_until_parked();
        assert_eq!(
            endpoint.receipt(intent.request_id).expect("receipt").state,
            LaunchReceiptState::Handled,
            "metadata completion is not required for the original open request acknowledgement"
        );
        handles.push(owner.read_with(cx, |app, _| {
            app.windows.values().next().expect("tab").window
        }));
    }
    assert!(handles.iter().all(|window| *window == handles[0]));
    owner.read_with(cx, |app, _| {
        assert_eq!(app.windows.len(), 2);
        assert_eq!(app.reads.len(), 2);
    });
    let ids = owner.read_with(cx, |app, _| app.windows.keys().copied().collect::<Vec<_>>());
    for id in ids {
        owner.update(cx, |app, cx| app.close_browser(id, cx));
    }
    cx.run_until_parked();
}

#[gpui_kit::test]
fn browser_tabs_background_password_waits_for_its_active_tab(cx: &mut TestAppContext) {
    use nxtools_protocol::archive_password::ArchivePasswordPrompt;
    let (owner, endpoint) = setup(cx);
    let first = PathBuf::from(r"C:\owned\encrypted.zip");
    owner.update(cx, |app, cx| {
        app.open_canonical(first.clone(), 0, vec![], cx)
    });
    cx.run_until_parked();
    let command = endpoint
        .try_receive_command_for_contract_test()
        .expect("read");
    let Message::PlatformCommand(PlatformCommand::ArchiveBrowserRead(read)) = command.message
    else {
        panic!("read");
    };
    owner.update(cx, |app, cx| {
        app.open_canonical(PathBuf::from(r"C:\owned\other.zip"), 0, vec![], cx)
    });
    cx.run_until_parked();
    let prompt = ArchivePasswordPrompt {
        operation_id: read.operation_id,
        session_id: read.session_id,
        generation: read.generation,
        prompt_id: Uuid::new_v4(),
        ticket_generation: 1,
        error_code: "PasswordRequired".into(),
        attempt: 1,
    };
    owner.update(cx, |app, cx| app.password_prompt(prompt, cx));
    cx.run_until_parked();
    let window = owner.read_with(cx, |app, _| app.host.as_ref().expect("host").window);
    window
        .update(cx, |_, window, cx| {
            window.draw(cx).clear(cx);
            assert!(
                !window.has_active_dialog(cx),
                "background source must not prompt over another tab"
            );
        })
        .expect("host");
    owner.update(cx, |app, cx| app.activate_browser(read.session_id, cx));
    cx.run_until_parked();
    window
        .update(cx, |_, window, cx| {
            window.draw(cx).clear(cx);
            assert!(
                window.has_active_dialog(cx),
                "original pending password ticket is shown on its own tab"
            );
        })
        .expect("host");
    let ids = owner.read_with(cx, |app, _| app.windows.keys().copied().collect::<Vec<_>>());
    for id in ids {
        owner.update(cx, |app, cx| app.close_browser(id, cx));
    }
    cx.run_until_parked();
}

#[gpui_kit::test]
fn browser_tabs_switch_retains_original_model_selection_preview_and_generation(
    cx: &mut TestAppContext,
) {
    let (owner, _native, first, snapshot) = preview_fixture(cx);
    let view = owner.read_with(cx, |app, _| {
        app.windows[&first].view.upgrade().expect("original view")
    });
    let row = super::super::model::RowId::Entry(snapshot, 1);
    let token = preview_select(&owner, first, row.clone(), cx);
    let generation = view.read_with(cx, |view, _| view.generation);
    owner.update(cx, |app, cx| {
        app.open_canonical(PathBuf::from(r"C:\owned\independent.zip"), 0, vec![], cx)
    });
    cx.run_until_parked();
    owner.update(cx, |app, cx| app.activate_browser(first, cx));
    cx.run_until_parked();
    view.read_with(cx, |view, _| {
        assert_eq!(view.generation, generation);
        assert_eq!(view.model.as_ref().expect("same model").snapshot, snapshot);
        assert!(
            view.model
                .as_ref()
                .expect("same selection")
                .selected
                .contains(&row)
        );
        assert!(
            !view.preview_selection_matches(token, &Some(row.clone())),
            "hidden tab token cannot survive its retirement"
        );
        let renewed = view.preview_token_for_test();
        assert!(
            renewed > token && view.preview_selection_matches(renewed, &Some(row)),
            "same selected member gets a fresh preview generation on return"
        );
    });
    owner.read_with(cx, |app, cx| {
        assert_eq!(
            app.windows[&first]
                .view
                .upgrade()
                .expect("same retained entity"),
            view
        );
        assert_eq!(
            app.host
                .as_ref()
                .expect("one host")
                .view
                .read_with(cx, |host, _| host.active)
                .expect("host"),
            Some(first)
        );
    });
    let host = owner.read_with(cx, |app, _| app.host.as_ref().expect("host").id);
    owner.update(cx, |app, cx| app.close_host(host, cx));
    cx.run_until_parked();
}

#[gpui_kit::test]
fn browser_tabs_window_close_retires_every_source_without_replacement_host_effect(
    cx: &mut TestAppContext,
) {
    let (owner, _native) = setup(cx);
    owner.update(cx, |app, cx| {
        app.open_canonical(PathBuf::from(r"C:\owned\one.zip"), 0, vec![], cx);
        app.open_canonical(PathBuf::from(r"C:\owned\two.zip"), 0, vec![], cx);
    });
    cx.run_until_parked();
    let (old, window) = owner.read_with(cx, |app, _| {
        let host = app.host.as_ref().expect("host");
        (host.id, host.window)
    });
    let mut surface = gpui_kit::VisualTestContext::from_window(window, cx);
    surface.refresh().expect("real shared caption");
    let position = surface
        .debug_bounds("archive-browser-window-close")
        .expect("single window close")
        .center();
    surface.simulate_event(gpui_kit::MouseDownEvent {
        position,
        button: gpui_kit::MouseButton::Left,
        ..Default::default()
    });
    surface.simulate_event(gpui_kit::MouseUpEvent {
        position,
        button: gpui_kit::MouseButton::Left,
        click_count: 1,
        modifiers: Default::default(),
    });
    surface.run_until_parked();
    assert!(owner.read_with(&surface, |app, _| app.host.is_none()
        && app.windows.is_empty()
        && app.reads.is_empty()
        && app.paths.is_empty()));
    owner.update(&mut surface, |app, cx| {
        app.open_canonical(PathBuf::from(r"C:\owned\new.zip"), 0, vec![], cx)
    });
    surface.run_until_parked();
    owner.update(&mut surface, |app, cx| app.retire_host(old, cx));
    assert_eq!(
        owner.read_with(&surface, |app, _| app.windows.len()),
        1,
        "late old close must not retire replacement host"
    );
    let current = owner.read_with(&surface, |app, _| app.host.as_ref().expect("new host").id);
    owner.update(&mut surface, |app, cx| app.close_host(current, cx));
    surface.run_until_parked();
}

#[gpui_kit::test]
fn browser_tabs_open_request_hides_old_password_dialog_without_losing_ticket(
    cx: &mut TestAppContext,
) {
    use nxtools_protocol::archive_password::ArchivePasswordPrompt;
    let (owner, endpoint) = setup(cx);
    owner.update(cx, |app, cx| {
        app.open_canonical(PathBuf::from(r"C:\owned\encrypted.zip"), 0, vec![], cx)
    });
    cx.run_until_parked();
    let command = endpoint
        .try_receive_command_for_contract_test()
        .expect("read");
    let Message::PlatformCommand(PlatformCommand::ArchiveBrowserRead(read)) = command.message
    else {
        panic!("read")
    };
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
    let window = owner.read_with(cx, |app, _| app.host.as_ref().expect("host").window);
    window
        .update(cx, |_, window, cx| assert!(window.has_active_dialog(cx)))
        .expect("old dialog");
    owner.update(cx, |app, cx| {
        app.open_canonical(PathBuf::from(r"C:\owned\next.zip"), 0, vec![], cx)
    });
    cx.run_until_parked();
    window
        .update(cx, |_, window, cx| {
            window.draw(cx).clear(cx);
            assert!(
                !window.has_active_dialog(cx),
                "old source password dialog cannot cover the newly opened tab"
            );
        })
        .expect("current host");
    assert!(
        owner.read_with(cx, |app, _| app.passwords.get(&read.operation_id)
            == Some(&prompt)),
        "original source ticket remains pending"
    );
    owner.update(cx, |app, cx| app.activate_browser(read.session_id, cx));
    cx.run_until_parked();
    window
        .update(cx, |_, window, cx| assert!(window.has_active_dialog(cx)))
        .expect("original source dialog restored");
    let id = owner.read_with(cx, |app, _| app.host.as_ref().expect("host").id);
    owner.update(cx, |app, cx| app.close_host(id, cx));
    cx.run_until_parked();
}

fn edit_application_pointer_menu_key_form_dispatch_once_and_stale_refuses(
    cx: &mut TestAppContext,
    format: &str,
) {
    use gpui_kit::{MouseButton, MouseDownEvent, MouseUpEvent, VisualTestContext};
    use nxtools_protocol::archive_browser::{SevenzEditEligibility, TarEditEligibility};
    use nxtools_protocol::archive_job::{ArchiveJobAction, ArchiveJobModifyAction as A};
    let (owner, endpoint, id, snapshot) = preview_fixture(cx);
    owner.update(cx, |app, cx| {
        let slot = app.windows.get_mut(&id).expect("original browser");
        let browser = slot.browser.as_mut().expect("read context");
        browser.format = format.into();
        if format == "SevenZ" {
            browser.sevenz_edit = Some(SevenzEditEligibility { refusal: None });
        } else {
            browser.tar_edit = Some(TarEditEligibility { refusal: None });
        }
        let browser = browser.clone();
        let view = slot.view.upgrade().expect("original view");
        slot.window
            .update(cx, |_, window, cx| {
                view.update(cx, |view, cx| {
                    let entries = view.model.as_ref().expect("snapshot").entries.clone();
                    view.install(snapshot, entries, format.into(), window, cx);
                    view.bind_edit_context(&browser, cx);
                })
            })
            .expect("original window");
    });
    let window = owner.read_with(cx, |app, _| app.windows[&id].window);
    let mut surface = VisualTestContext::from_window(window, cx);
    surface.simulate_resize(gpui_kit::size(px(1200.), px(800.)));
    surface.run_until_parked();
    let draw = |surface: &mut VisualTestContext| {
        surface.run_until_parked();
        surface.update(|window, cx| {
            window.activate_window();
            window.draw(cx).clear(cx);
        });
    };
    let click = |surface: &mut VisualTestContext, position| {
        surface.simulate_event(MouseDownEvent {
            position,
            button: MouseButton::Left,
            click_count: 1,
            ..Default::default()
        });
        surface.simulate_event(MouseUpEvent {
            position,
            button: MouseButton::Left,
            click_count: 1,
            modifiers: Default::default(),
        });
        surface.run_until_parked();
    };
    draw(&mut surface);
    let row = surface
        .debug_bounds("archive-entry-1")
        .expect("immutable member")
        .center();
    click(&mut surface, row);
    draw(&mut surface);
    // No invented global key for replacement: existing named menu and toolbar command.
    for route in 0..3 {
        if route == 0 {
            let point = surface
                .debug_bounds("archive-replace-content")
                .expect("official outlined action")
                .center();
            click(&mut surface, point);
        } else if route == 1 {
            let point = surface.update(|window, _| {
                gpui_kit::base::test_support::snapshots(window)
                    .into_iter()
                    .find(|node| {
                        node.label() == Some("编辑") && node.role() == Some(gpui_kit::Role::Button)
                    })
                    .expect("original edit menu")
                    .bounds()
                    .center()
            });
            click(&mut surface, point);
            draw(&mut surface);
            let point = surface.update(|window, _| {
                gpui_kit::base::test_support::snapshots(window)
                    .into_iter()
                    .find(|node| node.label() == Some("替换内容…"))
                    .expect("named replacement menu action")
                    .bounds()
                    .center()
            });
            click(&mut surface, point);
        } else {
            // Existing F2 grammar is preserved for 7z rename; held/KeyUp do not double dispatch.
            surface.simulate_keystrokes("f2");
            surface.run_until_parked();
        }
        assert_eq!(
            owner.read_with(&surface, |app, _| app.jobs.forms.len()),
            1,
            "one form per production action"
        );
        let (form_id, form_window) = owner.read_with(&surface, |app, _| {
            let (form_id, slot) = app
                .jobs
                .forms
                .iter()
                .next()
                .expect("original ModifyDraft/form");
            (*form_id, slot.window)
        });
        if route == 2 {
            let mut form_surface = VisualTestContext::from_window(form_window, cx);
            draw(&mut form_surface);
            let point = form_surface
                .debug_bounds("archive-form-submit")
                .expect("original submit")
                .center();
            click(&mut form_surface, point);
            click(&mut form_surface, point);
            owner.read_with(&form_surface, |app, _| {
                assert_eq!(
                    app.jobs.records.len(),
                    1,
                    "double pointer submit reaches original FIFO once"
                );
                let request = app
                    .jobs
                    .records
                    .values()
                    .next()
                    .expect("original job")
                    .command
                    .request
                    .as_ref()
                    .expect("request");
                let modify = request.modify.as_ref().expect("Modify");
                assert_eq!(modify.action, A::Rename);
                assert_eq!(modify.ordinals, vec![1]);
            });
            break;
        }
        // Stale refusal is checked again at original submit owner, not only at the browser button.
        owner.update(&mut surface, |app, cx| {
            app.windows[&id]
                .view
                .upgrade()
                .expect("view")
                .update(cx, |view, _| {
                    view.status =
                        super::super::browser::BrowserStatus::Stale("source changed".into());
                });
            let browser = app.windows[&id].browser.clone().expect("captured source");
            let draft = crate::archive::draft::ModifyDraft {
                source: app.windows[&id].path.clone(),
                browser,
                action: A::Delete,
                ordinals: vec![1],
                value: None,
                source_prefix: None,
                added: vec![],
                codepage: 0,
            };
            app.submit_form(
                form_id,
                1,
                draft.request().expect("structurally valid frozen intent"),
                crate::archive::forms::FormCompletion {
                    open_after: false,
                    keep_open: false,
                    remember: None,
                    remember_baseline: None,
                    additional_requests: vec![],
                },
                cx,
            );
            assert!(
                app.jobs.records.is_empty(),
                "stale form cannot reach original queue"
            );
            app.windows[&id]
                .view
                .upgrade()
                .expect("view")
                .update(cx, |view, _| {
                    view.status = super::super::browser::BrowserStatus::Ready
                });
            app.retire_form(form_id);
        });
        let mut form_surface = VisualTestContext::from_window(form_window, cx);
        form_surface.run_until_parked();
        form_surface.simulate_keystrokes("escape");
        form_surface.run_until_parked();
        click(&mut surface, row);
        draw(&mut surface);
    }
    assert_eq!(
        owner.read_with(&surface, |app, _| app.jobs.records.len()),
        1
    );
    let mut starts = 0;
    while let Ok(command) = endpoint.try_receive_command_for_contract_test() {
        if matches!(command.message, Message::PlatformCommand(PlatformCommand::ArchiveJob(ref command)) if command.action == ArchiveJobAction::Start)
        {
            starts += 1;
        }
    }
    assert_eq!(starts, 1, "exactly one original IPC Start");
    assert_eq!(
        owner.read_with(&surface, |app, _| app.windows[&id]
            .browser
            .as_ref()
            .expect("original context")
            .snapshot_id),
        snapshot
    );
}

#[gpui_kit::test]
fn sevenz_subtree_rename_original_owner_rejects_old_generation_and_stale_snapshot(
    cx: &mut TestAppContext,
) {
    use nxtools_protocol::archive_job::ArchiveJobModifyAction;
    let (owner, _endpoint, id, snapshot) = preview_fixture(cx);
    let generation = owner.update(cx, |app, cx| {
        let slot = app.windows.get_mut(&id).expect("browser");
        let browser = slot.browser.as_mut().expect("context");
        browser.format = "SevenZ".into();
        browser.sevenz_edit = Some(nxtools_protocol::archive_browser::SevenzEditEligibility { refusal: None });
        let browser = browser.clone();
        let view = slot.view.upgrade().expect("view");
        slot.window.update(cx, |_, window, cx| view.update(cx, |view, cx| {
            let entry = serde_json::from_value(serde_json::json!({"ordinal":71,"name":"a/b/c.txt","kind":"File","size":1,"packed_size":1,"encrypted":false,"warning_codes":[],"provenance":"Utf8Flag","codepage":65001})).expect("immutable descendant");
            view.install(snapshot, vec![entry], "SevenZ".into(), window, cx);
            view.bind_edit_context(&browser, cx);
        })).expect("window");
        slot.generation
    });
    owner.update(cx, |app, cx| {
        for origin in [
            (id, generation + 1, snapshot),
            (id, generation, Uuid::new_v4()),
        ] {
            app.modify_browser(
                origin,
                ArchiveJobModifyAction::Rename,
                vec![71],
                (Some("renamed/".into()), Some("a/".into())),
                0,
                cx,
            );
            assert!(
                app.jobs.forms.is_empty(),
                "old origin cannot open edit form"
            );
        }
        app.windows[&id]
            .view
            .upgrade()
            .expect("view")
            .update(cx, |view, _| {
                view.status = BrowserStatus::Stale("changed".into())
            });
        app.modify_browser(
            (id, generation, snapshot),
            ArchiveJobModifyAction::Rename,
            vec![71],
            (Some("renamed/".into()), Some("a/".into())),
            0,
            cx,
        );
        assert!(
            app.jobs.forms.is_empty(),
            "stale source cannot open edit form"
        );
        assert!(app.jobs.records.is_empty());
    });
}

#[gpui_kit::test]
fn sevenz_edit_application_original_pointer_menu_key_form_dispatch_once_and_stale_refuses(
    cx: &mut TestAppContext,
) {
    edit_application_pointer_menu_key_form_dispatch_once_and_stale_refuses(cx, "SevenZ");
}
#[gpui_kit::test]
fn step4_tar_original_pointer_menu_key_form_dispatch_once_and_stale_refuses(
    cx: &mut TestAppContext,
) {
    edit_application_pointer_menu_key_form_dispatch_once_and_stale_refuses(cx, "Tar");
}
#[gpui_kit::test]
fn step4_tgz_original_pointer_menu_key_form_dispatch_once_and_stale_refuses(
    cx: &mut TestAppContext,
) {
    edit_application_pointer_menu_key_form_dispatch_once_and_stale_refuses(cx, "TarGz");
}
#[gpui_kit::test]
fn step4_txz_original_pointer_menu_key_form_dispatch_once_and_stale_refuses(
    cx: &mut TestAppContext,
) {
    edit_application_pointer_menu_key_form_dispatch_once_and_stale_refuses(cx, "TarXz");
}

#[gpui_kit::test]
fn step4_drop_add_uses_format_neutral_dialog_and_cancel_has_no_edit(cx: &mut TestAppContext) {
    let (owner, endpoint, id, snapshot) = preview_fixture(cx);
    owner.update(cx, |app, cx| {
        let slot = app.windows.get_mut(&id).expect("browser");
        let browser = slot.browser.as_mut().expect("context");
        browser.format = "Tar".into();
        browser.tar_edit =
            Some(nxtools_protocol::archive_browser::TarEditEligibility { refusal: None });
        let browser = browser.clone();
        let view = slot.view.upgrade().expect("view");
        slot.window
            .update(cx, |_, window, cx| {
                view.update(cx, |view, cx| {
                    let entries = view.model.as_ref().expect("model").entries.clone();
                    view.install(snapshot, entries, "Tar".into(), window, cx);
                    view.bind_edit_context(&browser, cx);
                })
            })
            .expect("window");
    });
    let window = owner.read_with(cx, |app, _| app.windows[&id].window);
    let mut surface = gpui_kit::VisualTestContext::from_window(window, cx);
    owner.update(&mut surface, |app, cx| {
        app.handle_drop(id, vec![r"C:\owned\incoming.txt".into()], window, cx)
    });
    surface.run_until_parked();
    surface.update(|window, cx| {
        window.draw(cx).clear(cx);
        let nodes = gpui_kit::base::test_support::snapshots(window);
        assert!(
            nodes
                .iter()
                .any(|node| node.label() == Some("添加到当前压缩包")),
            "TAR drop must not claim ZIP"
        );
    });
    surface.simulate_keystrokes("escape");
    surface.run_until_parked();
    assert!(owner.read_with(&surface, |app, _| app.jobs.forms.is_empty()
        && app.jobs.records.is_empty()));
    while let Ok(envelope) = endpoint.try_receive_command_for_contract_test() {
        assert!(!matches!(
            envelope.message,
            Message::PlatformCommand(PlatformCommand::ArchiveJob(_))
        ));
    }
}

#[gpui_kit::test]
fn step4_committed_edit_refresh_restores_folder_and_new_name_not_old_ordinal(
    cx: &mut TestAppContext,
) {
    use nxtools_protocol::archive_job::*;
    let (owner, endpoint, id, snapshot) = preview_fixture(cx);
    let generation=owner.update(cx,|app,cx|{let slot=app.windows.get_mut(&id).expect("browser");let browser=slot.browser.as_mut().expect("context");browser.format="Tar".into();browser.tar_edit=Some(nxtools_protocol::archive_browser::TarEditEligibility{refusal:None});let browser=browser.clone();let view=slot.view.upgrade().expect("view");slot.window.update(cx,|_,window,cx|view.update(cx,|view,cx|{let entry=serde_json::from_value(serde_json::json!({"ordinal":71,"name":"a/old.txt","kind":"File","size":1,"packed_size":null,"encrypted":false,"warning_codes":[],"provenance":"Utf8Flag","codepage":65001})).expect("old entry");view.install(snapshot,vec![entry],"Tar".into(),window,cx);view.bind_edit_context(&browser,cx);let model=view.model.as_mut().expect("model");model.enter("a");model.select(crate::archive::model::RowId::Entry(snapshot,71),false,false);})).expect("window");slot.generation});
    let job=owner.update(cx,|app,cx|{let browser=app.windows[&id].browser.clone().expect("context");let request:ArchiveJobRequest=serde_json::from_value(serde_json::json!({"kind":"modify","paths":["C:\\owned\\preview.zip"],"destination":null,"extract":null,"create":null,"modify":{"browser":browser,"action":"rename","ordinals":[71],"value":"a/new.txt"}})).expect("request");let job=app.submit_job(request,crate::archive::forms::FormCompletion::default(),cx).expect("job");app.jobs.records.get_mut(&job).expect("record").edit_origin=Some((id,generation,snapshot));job});
    while endpoint.try_receive_command_for_contract_test().is_ok() {}
    owner.update(cx,|app,cx|{let terminal:ArchiveJobTerminal=serde_json::from_value(serde_json::json!({"outcome":"succeeded","code":null,"files_committed":1,"bytes_committed":100,"files_verified":1,"bytes_verified":100,"files_skipped":0,"cleanup_complete":true,"commit_state":"Committed","elapsed_ms":1,"native_error_code":null,"journal":[]})).expect("terminal");app.job_terminal(job,terminal,cx);});
    cx.run_until_parked();
    let mut read = None;
    while let Ok(envelope) = endpoint.try_receive_command_for_contract_test() {
        if let Message::PlatformCommand(PlatformCommand::ArchiveBrowserRead(command)) =
            envelope.message
            && command.action == ArchiveReadAction::Open
        {
            read = Some(command);
        }
    }
    let read = read.expect("original refresh read IPC");
    deliver(
        &endpoint,
        read_event(&read, ArchiveReadEventKind::Accepted),
        cx,
    );
    let mut negotiated = read_event(&read, ArchiveReadEventKind::Negotiated);
    negotiated.capabilities = archive_read_capabilities(2)
        .iter()
        .map(|value| (*value).into())
        .collect();
    deliver(&endpoint, negotiated, cx);
    let fresh = Uuid::new_v4();
    let entry = |ordinal, name: &str| {
        serde_json::from_value(serde_json::json!({"ordinal":ordinal,"name":name,"kind":"File","size":1,"packed_size":null,"encrypted":false,"warning_codes":[],"provenance":"Utf8Flag","codepage":65001})).expect("fresh entry")
    };
    let mut batch = read_event(&read, ArchiveReadEventKind::Batch);
    batch.batch = Some(ArchiveReadBatch {
        snapshot_id: fresh,
        sequence: 0,
        format: "Tar".into(),
        entries: vec![entry(0, "other.txt"), entry(1, "a/new.txt")],
    });
    deliver(&endpoint, batch, cx);
    let browser = owner.read_with(cx, |app, _| {
        let mut browser = app.jobs.records[&job]
            .command
            .request
            .as_ref()
            .expect("request")
            .modify
            .as_ref()
            .expect("intent")
            .browser
            .clone();
        browser.snapshot_id = fresh;
        browser.source.digest = [2; 32];
        browser
    });
    let mut terminal = read_event(&read, ArchiveReadEventKind::Terminal);
    terminal.terminal = Some(ArchiveReadTerminal {
        status: ArchiveReadStatus::Succeeded,
        code: None,
        files_verified: 0,
        bytes_verified: 0,
        entry_count: 2,
        batch_count: 1,
        elapsed_ms: 1,
        native_error_code: None,
        cleanup_complete: true,
        transfer: Some(ArchiveReadTransfer {
            private_bytes: 4096,
            received_bytes: 4096,
            max_batch_entries: 2,
            max_batch_bytes: 1024,
            max_inflight_batches: 1,
            process_budget_bytes: 536870912,
        }),
        browser: Some(Box::new(browser)),
    });
    deliver(&endpoint, terminal, cx);
    cx.run_until_parked();
    owner.read_with(cx, |app, cx| {
        app.windows[&id]
            .view
            .read_with(cx, |view, _| {
                let model = view.model.as_ref().expect("new model");
                assert_eq!(
                    model.snapshot, fresh,
                    "only a ready new snapshot tests restoration"
                );
                assert_eq!(
                    model.folder, "a",
                    "current folder survives successful edit refresh"
                );
                assert_eq!(
                    model.selected.iter().cloned().collect::<Vec<_>>(),
                    vec![crate::archive::model::RowId::Entry(fresh, 1)],
                    "new snapshot matches transformed name, never old ordinal71"
                );
            })
            .expect("view");
    });
}

#[gpui_kit::test]
fn step4_drop_add_rechecks_frozen_folder_snapshot_and_generation(cx: &mut TestAppContext) {
    let (owner, endpoint, id, snapshot) = preview_fixture(cx);
    owner.update(cx, |app, cx| {
        let slot = app.windows.get_mut(&id).expect("browser");
        let context = slot.browser.as_mut().expect("context");
        context.format = "Tar".into();
        context.tar_edit =
            Some(nxtools_protocol::archive_browser::TarEditEligibility { refusal: None });
        let browser = slot.browser.clone().expect("browser");
        let view = slot.view.upgrade().expect("view");
        slot.window
            .update(cx, |_, window, cx| {
                view.update(cx, |view, cx| {
                    let entries = view.model.as_ref().expect("model").entries.clone();
                    view.install(snapshot, entries, "Tar".into(), window, cx);
                    view.bind_edit_context(&browser, cx);
                    view.model.as_mut().expect("model").folder = String::new();
                })
            })
            .expect("window");
        let target = app
            .drop_add_target(id, &[r"C:\owned\incoming.txt".into()], cx)
            .expect("frozen Add");
        for case in 0..5 {
            let mut stale = target.clone();
            match case {
                0 => stale.origin.browser.1 += 1,
                1 => stale.origin.browser.2 = Uuid::new_v4(),
                2 => {
                    stale.request.modify.as_mut().expect("Modify").value =
                        Some("changed-folder/".into())
                }
                3 => stale.origin.connection += 1,
                _ => stale.request.paths[0] = r"C:\owned\different.zip".into(),
            }
            app.add_drop(stale, cx);
            assert!(app.jobs.forms.is_empty(), "stale drop cannot create form");
            assert!(app.jobs.records.is_empty(), "stale drop cannot submit");
        }
        app.add_drop(target, cx);
        assert!(app.jobs.forms.is_empty(), "direct Add has no form");
        assert_eq!(
            app.jobs.records.len(),
            1,
            "unchanged frozen target submits once"
        );
    });
    cx.run_until_parked();
    let mut jobs = 0;
    while let Ok(envelope) = endpoint.try_receive_command_for_contract_test() {
        if matches!(
            envelope.message,
            Message::PlatformCommand(PlatformCommand::ArchiveJob(_))
        ) {
            jobs += 1;
        }
    }
    assert_eq!(jobs, 1, "one original typed Add request");
}

#[gpui_kit::test]
fn step4_original_app_and_final_form_refuse_missing_or_wrong_kind_member(cx: &mut TestAppContext) {
    use nxtools_protocol::archive_job::ArchiveJobModifyAction as A;
    let (owner, endpoint, id, snapshot) = preview_fixture(cx);
    owner.update(cx, |app, cx| {
        let slot = app.windows.get_mut(&id).expect("browser");
        let context = slot.browser.as_mut().expect("context");
        context.format = "Tar".into();
        context.tar_edit =
            Some(nxtools_protocol::archive_browser::TarEditEligibility { refusal: None });
        let context = context.clone();
        let generation = slot.generation;
        let view = slot.view.upgrade().expect("view");
        slot.window
            .update(cx, |_, window, cx| {
                view.update(cx, |view, cx| {
                    let entries = view.model.as_ref().expect("model").entries.clone();
                    view.install(snapshot, entries, "Tar".into(), window, cx);
                    view.bind_edit_context(&context, cx);
                })
            })
            .expect("window");
        for (action, ordinal, value) in [(A::Delete, 999, None), (A::Rename, 0, Some("new/"))] {
            app.modify_browser(
                (id, generation, snapshot),
                action,
                vec![ordinal],
                (value.map(str::to_owned), None),
                0,
                cx,
            );
            assert!(
                app.jobs.forms.is_empty(),
                "original app rejects missing/wrong-kind member before form"
            );
        }
        app.modify_browser(
            (id, generation, snapshot),
            A::Rename,
            vec![0],
            (Some("renamed.txt".into()), None),
            0,
            cx,
        );
        let form = *app.jobs.forms.keys().next().expect("valid original form");
        for (action, ordinal, value) in [(A::Delete, 999, None), (A::Rename, 0, Some("new/"))] {
            let draft = crate::archive::draft::ModifyDraft {
                source: app.windows[&id].path.clone(),
                browser: context.clone(),
                action,
                ordinals: vec![ordinal],
                value: value.map(str::to_owned),
                source_prefix: None,
                added: vec![],
                codepage: 0,
            };
            app.submit_form(
                form,
                1,
                draft
                    .request()
                    .expect("wire-valid but stale/wrong-kind intent"),
                crate::archive::forms::FormCompletion::default(),
                cx,
            );
            assert!(
                app.jobs.records.is_empty(),
                "final app guard refuses before queue/IPC"
            );
        }
    });
    cx.run_until_parked();
    while let Ok(envelope) = endpoint.try_receive_command_for_contract_test() {
        assert!(!matches!(
            envelope.message,
            Message::PlatformCommand(PlatformCommand::ArchiveJob(_))
        ));
    }
}

#[gpui_kit::test]
fn opt03_drop_new_keeps_all_sources_in_original_create_form(cx: &mut TestAppContext) {
    use gpui_kit::{MouseButton, MouseDownEvent, MouseUpEvent};
    let (owner, endpoint, id, _) = preview_fixture(cx);
    let root = std::env::temp_dir().join(format!("archive-opt03-drop-{}", Uuid::new_v4()));
    std::fs::create_dir_all(root.join("folder")).expect("owned directory");
    std::fs::write(root.join("first.zip"), b"fixture").expect("owned archive input");
    std::fs::write(root.join("last.txt"), b"last").expect("owned ordinary input");
    let paths = vec![
        root.join("first.zip"),
        root.join("folder"),
        root.join("last.txt"),
    ];
    let window = owner.read_with(cx, |app, _| app.windows[&id].window);
    let mut surface = gpui_kit::VisualTestContext::from_window(window, cx);
    owner.update(&mut surface, |app, cx| {
        app.handle_drop(id, paths.clone(), window, cx)
    });
    surface.run_until_parked();
    let position = surface.update(|window, cx| {
        window.draw(cx).clear(cx);
        gpui_kit::base::test_support::snapshots(window)
            .into_iter()
            .find(|node| node.label() == Some("全部来源新建"))
            .expect("real New-all drop button")
            .bounds()
            .center()
    });
    surface.simulate_event(MouseDownEvent {
        button: MouseButton::Left,
        position,
        click_count: 1,
        ..Default::default()
    });
    surface.simulate_event(MouseUpEvent {
        button: MouseButton::Left,
        position,
        click_count: 1,
        ..Default::default()
    });
    surface.run_until_parked();
    let form_window = owner.read_with(&surface, |app, _| {
        assert_eq!(app.jobs.forms.len(), 1);
        assert!(
            app.jobs.records.is_empty(),
            "choosing New is not submission"
        );
        app.jobs
            .forms
            .values()
            .next()
            .expect("original form")
            .window
    });
    let mut form_surface = gpui_kit::VisualTestContext::from_window(form_window, cx);
    form_surface.run_until_parked();
    form_surface.update(|window, cx| {
        window.draw(cx).clear(cx);
        let nodes = gpui_kit::base::test_support::snapshots(window);
        assert_eq!(
            nodes
                .iter()
                .filter(|node| node.label() == Some("移除"))
                .count(),
            paths.len(),
            "real source rows"
        );
    });
    owner.read_with(&form_surface, |app, cx| {
        let form = app
            .jobs
            .forms
            .values()
            .next()
            .expect("form")
            .view
            .upgrade()
            .expect("live form");
        let (enumerating, actual) = form.read(cx).sources_for_test();
        assert!(!enumerating, "original metadata enumeration completed");
        assert_eq!(actual, paths, "all sources keep their order and identity");
    });
    while let Ok(envelope) = endpoint.try_receive_command_for_contract_test() {
        assert!(!matches!(
            envelope.message,
            Message::PlatformCommand(PlatformCommand::ArchiveJob(_))
        ));
    }
    std::fs::remove_dir_all(root).expect("owned fixture cleanup");
}

#[path = "opt03_add_selection_tests.rs"]
mod opt03_add_selection_tests;

#[path = "incoming_add_tests.rs"]
mod incoming_add_tests;

#[path = "opt04_preview_tests.rs"]
mod opt04_preview_tests;

#[gpui_kit::test]
fn opt07_find_extract_and_delete_reuse_original_forms_and_frozen_submit_guards(
    cx: &mut TestAppContext,
) {
    use crate::archive::model::RowId;
    use gpui_kit::{Role, VisualTestContext};
    fn click(surface: &mut VisualTestContext, label: &str, role: Role) {
        surface.run_until_parked();
        let position = surface.update(|window, cx| {
            window.activate_window();
            window.draw(cx).clear(cx);
            gpui_kit::base::test_support::snapshots(window)
                .into_iter()
                .find(|node| {
                    node.label() == Some(label) && node.role() == Some(role) && node.visible()
                })
                .expect("original visible control")
                .bounds()
                .center()
        });
        surface.simulate_event(gpui_kit::MouseDownEvent {
            position,
            button: gpui_kit::MouseButton::Left,
            click_count: 1,
            ..Default::default()
        });
        surface.simulate_event(gpui_kit::MouseUpEvent {
            position,
            button: gpui_kit::MouseButton::Left,
            click_count: 1,
            modifiers: Default::default(),
        });
        surface.run_until_parked();
    }
    for (action, stale) in [
        ("提取结果", false),
        ("提取结果", true),
        ("删除结果", false),
        ("删除结果", true),
    ] {
        let (owner, _endpoint, id, snapshot) = preview_fixture(cx);
        owner.update(cx, |app, cx| {
            let slot = app.windows.get_mut(&id).expect("original source");
            let browser = slot.browser.as_mut().expect("original context");
            browser.format = "Tar".into();
            browser.tar_edit =
                Some(nxtools_protocol::archive_browser::TarEditEligibility { refusal: None });
            let context = browser.clone();
            slot.window
                .update(cx, |_, window, cx| {
                    slot.view.update(cx, |view, cx| {
                        let entries = view.model.as_ref().expect("catalog").entries.clone();
                        view.install(snapshot, entries, "Tar".into(), window, cx);
                        view.bind_edit_context(&context, cx);
                    })
                })
                .expect("source window")
                .expect("source view");
        });
        let window = owner.read_with(cx, |app, _| app.windows[&id].window);
        let mut surface = VisualTestContext::from_window(window, cx);
        click(&mut surface, "查看", Role::Button);
        click(&mut surface, "查找整个压缩包  Ctrl+Shift+F", Role::MenuItem);
        surface.simulate_input("item1");
        surface.run_until_parked();
        surface.update(|window, cx| {
            window.draw(cx).clear(cx);
        });
        let position = surface
            .debug_bounds("archive-find-result-0")
            .expect("Find row")
            .center();
        surface.simulate_event(gpui_kit::MouseDownEvent {
            position,
            button: gpui_kit::MouseButton::Left,
            click_count: 1,
            ..Default::default()
        });
        surface.simulate_event(gpui_kit::MouseUpEvent {
            position,
            button: gpui_kit::MouseButton::Left,
            click_count: 1,
            modifiers: Default::default(),
        });
        surface.run_until_parked();
        surface.update(|window, cx| {
            window.draw(cx).clear(cx);
            let nodes = gpui_kit::base::test_support::snapshots(window);
            assert!(
                nodes
                    .iter()
                    .any(|node| node.label() == Some("item1.txt") && node.selected() == Some(true)),
                "Find row selected before {action}"
            );
            assert!(
                nodes.iter().any(|node| node.label() == Some(action)
                    && node.role() == Some(Role::Button)
                    && node.disabled() != Some(true)),
                "{action}: Find action available"
            );
        });
        click(&mut surface, action, Role::Button);
        let form_window = owner.read_with(&surface, |app, _| {
            assert_eq!(
                app.jobs.forms.len(),
                1,
                "{action}: one original decision form"
            );
            assert!(
                app.jobs.records.is_empty(),
                "Find never starts a Job directly"
            );
            let slot = app.windows.get(&id).expect("same source");
            assert_eq!(
                slot.view
                    .read_with(&surface, |view, _| view
                        .model
                        .as_ref()
                        .expect("same catalog")
                        .selected
                        .iter()
                        .cloned()
                        .collect::<Vec<_>>())
                    .expect("view"),
                vec![RowId::Entry(snapshot, 1)]
            );
            app.jobs
                .forms
                .values()
                .next()
                .expect("original form")
                .window
        });
        if stale {
            owner.update(&mut surface, |app, cx| {
                app.windows[&id]
                    .view
                    .update(cx, |view, _| {
                        view.status = BrowserStatus::Stale("来源已变化".into())
                    })
                    .expect("source invalidated");
            });
        }
        let mut form_surface = VisualTestContext::from_window(form_window, cx);
        click(
            &mut form_surface,
            if action == "删除结果" {
                "应用编辑"
            } else {
                "解压"
            },
            Role::Button,
        );
        owner.read_with(&form_surface, |app, _| {
            if stale {
                assert!(
                    app.jobs.records.is_empty(),
                    "original submit refuses stale Find-derived intent"
                );
            } else {
                assert_eq!(
                    app.jobs.records.len(),
                    1,
                    "original valid form starts exactly one Job"
                );
                let request = app
                    .jobs
                    .records
                    .values()
                    .next()
                    .expect("original Job")
                    .command
                    .request
                    .as_ref()
                    .expect("request");
                if action == "提取结果" {
                    let selected = request
                        .extract
                        .as_ref()
                        .expect("Extract")
                        .selection
                        .as_ref()
                        .expect("original selected snapshot");
                    assert_eq!(selected.browser.snapshot_id, snapshot);
                    assert_eq!(selected.ordinals, vec![1]);
                } else {
                    let modify = request.modify.as_ref().expect("Modify");
                    assert_eq!(modify.browser.snapshot_id, snapshot);
                    assert_eq!(modify.ordinals, vec![1]);
                }
            }
        });
    }
}

fn opt07_assert_find_warning_refusal(cx: &mut TestAppContext, action: &str) {
    use crate::archive::model::RowId;
    use gpui_kit::{Role, VisualTestContext};
    fn draw(surface: &mut VisualTestContext) {
        surface.run_until_parked();
        surface.update(|window, cx| {
            window.activate_window();
            window.draw(cx).clear(cx);
        });
    }
    fn click(surface: &mut VisualTestContext, label: &str, role: Role, count: usize) {
        draw(surface);
        let position = surface.update(|window, _| {
            gpui_kit::base::test_support::snapshots(window)
                .into_iter()
                .find(|node| {
                    node.label() == Some(label) && node.role() == Some(role) && node.visible()
                })
                .expect("current original control")
                .bounds()
                .center()
        });
        surface.simulate_event(gpui_kit::MouseDownEvent {
            position,
            button: gpui_kit::MouseButton::Left,
            click_count: count,
            ..Default::default()
        });
        surface.simulate_event(gpui_kit::MouseUpEvent {
            position,
            button: gpui_kit::MouseButton::Left,
            click_count: count,
            modifiers: Default::default(),
        });
        draw(surface);
    }
    fn key(surface: &mut VisualTestContext, stroke: &'static str) {
        surface.simulate_event(gpui_kit::KeyDownEvent {
            keystroke: gpui_kit::Keystroke::parse(stroke).expect("key"),
            is_held: false,
            prefer_character_input: false,
        });
        surface.simulate_event(gpui_kit::KeyUpEvent {
            keystroke: gpui_kit::Keystroke::parse(stroke).expect("release"),
        });
        draw(surface);
    }
    let (owner, endpoint, id, snapshot) = preview_fixture(cx);
    let (window, browser) = owner.update(cx, |app, cx| {
        let slot = app.windows.get_mut(&id).expect("original source");
        let context = slot.browser.as_mut().expect("original context");
        context.format = "Tar".into();
        context.tar_edit =
            Some(nxtools_protocol::archive_browser::TarEditEligibility { refusal: None });
        let context = context.clone();
        let browser = slot.view.upgrade().expect("original browser");
        slot.window
            .update(cx, |_, window, cx| {
                browser.update(cx, |view, cx| {
                    let template = view.model.as_ref().expect("catalog").entries[0].clone();
                    let mut warning = template.clone();
                    warning.ordinal = 411;
                    warning.name = "blocked/collision.txt".into();
                    warning.warning_codes = vec!["OutputCollision".into()];
                    let mut keep = template;
                    keep.ordinal = 777;
                    keep.name = "current/keep.txt".into();
                    view.install(snapshot, vec![warning, keep], "Tar".into(), window, cx);
                    view.bind_edit_context(&context, cx);
                })
            })
            .expect("source window");
        (slot.window, browser)
    });
    let mut surface = VisualTestContext::from_window(window, cx);
    click(&mut surface, "current", Role::ListBoxOption, 2);
    key(&mut surface, "ctrl-f");
    surface.simulate_input("keep");
    draw(&mut surface);
    click(&mut surface, "keep.txt", Role::ListBoxOption, 1);
    let state = |view: &ArchiveBrowser| {
        let model = view.model.as_ref().expect("same loaded catalog");
        (
            model.folder.clone(),
            model.filter.clone(),
            model.selected.clone(),
            model.focused.clone(),
            model.anchor.clone(),
            model
                .visible()
                .iter()
                .map(|row| row.id.clone())
                .collect::<Vec<_>>(),
            model.work.get(),
        )
    };
    let before = browser.read_with(&surface, |view, _| state(view));
    assert_eq!(before.0, "current");
    assert_eq!(before.1, "keep");
    assert_eq!(before.3, Some(RowId::Entry(snapshot, 777)));
    assert_eq!(before.4, before.3);
    key(&mut surface, "ctrl-shift-f");
    surface.simulate_input("collision");
    draw(&mut surface);
    let result = surface
        .debug_bounds("archive-find-result-0")
        .expect("original warning Find row")
        .center();
    surface.simulate_click(result, Default::default());
    draw(&mut surface);
    while let Ok(wire) = endpoint.try_receive_command_for_contract_test() {
        assert!(
            !matches!(
                wire.message,
                Message::PlatformCommand(PlatformCommand::ArchiveJob(_))
            ),
            "setup does not start a Job"
        );
    }
    click(&mut surface, action, Role::Button, 1);
    assert!(
        surface.update(|window, cx| window.has_active_dialog(cx)),
        "{action}: refusal retains original Find Dialog"
    );
    assert_eq!(
        browser.read_with(&surface, |view, _| state(view)),
        before,
        "{action}: refusal cannot mutate folder/filter/selection/focus/anchor or projection"
    );
    surface.update(|window, _| {
        let nodes = gpui_kit::base::test_support::snapshots(window);
        assert!(
            nodes
                .iter()
                .any(|node| node.label() == Some("名称或内部路径")
                    && node.value() == Some("collision")),
            "same Find query/input remains"
        );
        assert!(
            nodes
                .iter()
                .any(|node| node.label() == Some("blocked/collision.txt")
                    && node.selected() == Some(true)),
            "same Find result remains selected"
        );
    });
    owner.read_with(&surface, |app, _| {
        assert!(
            app.jobs.forms.is_empty() && app.jobs.records.is_empty(),
            "{action}: zero original form or Job"
        );
    });
    assert!(
        endpoint.try_receive_command_for_contract_test().is_err(),
        "{action}: zero dispatch"
    );
}

#[gpui_kit::test]
fn opt07_ac01_find_warning_extract_preserves_dialog_and_browser(cx: &mut TestAppContext) {
    opt07_assert_find_warning_refusal(cx, "提取结果");
}

#[gpui_kit::test]
fn opt07_ac01_find_warning_delete_preserves_dialog_and_browser(cx: &mut TestAppContext) {
    opt07_assert_find_warning_refusal(cx, "删除结果");
}

#[gpui_kit::test]
fn root_drop_extracts_whole_archive_into_receiver_created_folder(cx: &mut TestAppContext) {
    use nxtools_protocol::archive_job::*;
    let (owner, endpoint, _, _) = preview_fixture(cx);
    while endpoint.try_receive_command_for_contract_test().is_ok() {}
    let job = owner.update(cx, |app, cx| {
        app.extract_dropped(
            crate::archive::browser::LocatedDrop {
                operation: Uuid::new_v4(),
                selection: None,
                source: PathBuf::from(r"C:\owned\preview.zip"),
                codepage: 936,
                destination: r"D:\Downloads\preview".into(),
            },
            cx,
        );
        assert_eq!(app.jobs.records.len(), 1, "one ordinary job");
        *app.jobs.records.keys().next().expect("job")
    });
    owner.read_with(cx, |app, _| {
        let request = app.jobs.records[&job]
            .command
            .request
            .as_ref()
            .expect("start request");
        assert_eq!(request.kind, ArchiveJobKind::Extract);
        assert_eq!(request.paths, vec![r"C:\owned\preview.zip".to_owned()]);
        assert_eq!(
            request.destination.as_deref(),
            Some(r"D:\Downloads\preview")
        );
        assert_eq!(request.codepage, 936, "browser code page is retained");
        let extract = request.extract.as_ref().expect("extract options");
        assert_eq!(
            extract.strategy,
            ArchiveJobStrategy::Here,
            "the receiver already created the wrapper folder"
        );
        assert!(!extract.single_file_folder);
        assert!(
            extract.selection.is_none(),
            "whole archive, not a member list"
        );
        assert!(extract.reclamation.is_none());
    });
}

#[gpui_kit::test]
fn selected_drop_submits_ordinary_snapshot_bound_extraction_with_output_prefix(
    cx: &mut TestAppContext,
) {
    use nxtools_protocol::archive_job::*;
    let (owner, endpoint, session, _) = preview_fixture(cx);
    while endpoint.try_receive_command_for_contract_test().is_ok() {}
    owner.update(cx, |app, cx| {
        let browser = app.windows[&session]
            .browser
            .clone()
            .expect("browser binding");
        let selection: ArchiveJobSelection = serde_json::from_value(serde_json::json!({
            "browser":browser,"ordinals":[7,9],"strip_prefix":"top/sub/"
        }))
        .expect("selected extraction intent");
        app.extract_dropped(
            crate::archive::browser::LocatedDrop {
                operation: Uuid::new_v4(),
                source: PathBuf::from(r"C:\owned\preview.zip"),
                codepage: 936,
                destination: r"D:\out\sub (2)".into(),
                selection: Some(selection.clone()),
            },
            cx,
        );
        assert_eq!(app.jobs.records.len(), 1);
        let request = app
            .jobs
            .records
            .values()
            .next()
            .expect("ordinary job")
            .command
            .request
            .as_ref()
            .expect("request");
        request.validate().expect("ordinary extract admission");
        assert_eq!(request.kind, ArchiveJobKind::Extract);
        assert_eq!(request.codepage, 936);
        assert_eq!(request.destination.as_deref(), Some(r"D:\out\sub (2)"));
        let extract = request.extract.as_ref().expect("extract");
        assert_eq!(extract.strategy, ArchiveJobStrategy::Here);
        assert_eq!(extract.selection.as_ref(), Some(&selection));
        assert!(extract.reclamation.is_none());
        assert!(!extract.single_file_folder);
    });
}

#[gpui_kit::test]
fn ref17_b3_e2_actual_create_submit_freezes_volume_in_job_and_wire(cx: &mut TestAppContext) {
    use gpui_kit::{MouseButton, VisualTestContext, px};
    use nxtools_core::archive::{ArchiveVolumeSize, ArchiveVolumeUnit};
    use nxtools_protocol::archive_job::{ArchiveJobAction, ArchiveJobKind};

    fn draw(surface: &mut VisualTestContext) {
        surface.run_until_parked();
        surface.update(|window, cx| {
            window.activate_window();
            window.draw(cx).clear(cx);
        });
    }
    fn click(surface: &mut VisualTestContext, selector: &'static str) {
        let position = surface
            .debug_bounds(selector)
            .expect("actual form control")
            .center();
        surface.simulate_event(gpui_kit::MouseDownEvent {
            position,
            button: MouseButton::Left,
            ..Default::default()
        });
        surface.simulate_event(gpui_kit::MouseUpEvent {
            position,
            button: MouseButton::Left,
            modifiers: Default::default(),
            click_count: 1,
        });
        surface.run_until_parked();
    }
    fn reveal(surface: &mut VisualTestContext, selector: &'static str) {
        for _ in 0..60 {
            draw(surface);
            let viewport = surface
                .debug_bounds("archive-form-body")
                .expect("original scroll body");
            let bounds = surface.debug_bounds(selector);
            if bounds.is_some_and(|bounds| {
                bounds.top() >= viewport.top() && bounds.bottom() <= viewport.bottom()
            }) {
                return;
            }
            let direction = if bounds.is_some_and(|bounds| bounds.top() < viewport.top()) {
                2.
            } else {
                -2.
            };
            surface.simulate_event(gpui_kit::ScrollEvent {
                position: gpui_kit::point(viewport.right() - px(12.), viewport.center().y),
                delta: gpui_kit::ScrollDelta::Lines(gpui_kit::point(0., direction)),
                ..Default::default()
            });
        }
        panic!("original form control {selector} must be reachable");
    }
    let (owner, endpoint) = setup(cx);
    owner.update(cx, |app, cx| {
        app.settings.create_volume_size = Some(ArchiveVolumeSize {
            value: 64,
            unit: ArchiveVolumeUnit::KiB,
        });
        app.open_create_form(
            vec![crate::archive::draft::DraftSource {
                path: r"C:\synthetic\split-submit.txt".into(),
                directory: false,
                bytes: Some(12),
            }],
            cx,
        );
    });
    cx.run_until_parked();
    let (form_id, form, window) = owner.read_with(cx, |app, _| {
        assert_eq!(app.jobs.forms.len(), 1);
        let (id, slot) = app
            .jobs
            .forms
            .iter()
            .next()
            .expect("original Create form slot");
        (
            *id,
            slot.view.upgrade().expect("retained original form"),
            slot.window,
        )
    });
    let mut surface = VisualTestContext::from_window(window, cx);
    reveal(&mut surface, "archive-options-toggle");
    click(&mut surface, "archive-options-toggle");
    reveal(&mut surface, "archive-volume-number");
    click(&mut surface, "archive-volume-number");
    surface.simulate_keystrokes("ctrl-a");
    surface.simulate_input("65");
    draw(&mut surface);
    assert!(owner.read_with(&surface, |app, _| app.jobs.records.is_empty()));
    let starts_before = std::iter::from_fn(|| endpoint.try_receive_command_for_contract_test().ok())
        .filter(|envelope| matches!(&envelope.message, Message::PlatformCommand(PlatformCommand::ArchiveJob(command)) if command.action == ArchiveJobAction::Start))
        .count();
    assert_eq!(starts_before, 0, "editing must not admit a job");
    click(&mut surface, "archive-form-submit");
    // Accepted Create closes its original window; inspect the retained entity and owner, not a removed surface.
    surface.run_until_parked();
    let (job_id, accepted) = owner.read_with(&surface, |app, _| {
        assert!(
            !app.jobs.forms.contains_key(&form_id),
            "actual form accepted and retired"
        );
        assert_eq!(app.jobs.records.len(), 1, "one original JobRecord");
        let (id, record) = app.jobs.records.iter().next().expect("accepted job");
        assert_eq!(
            record.state, "正在准入",
            "original queue dispatched the accepted job"
        );
        (*id, record.command.clone())
    });
    assert_eq!(accepted.action, ArchiveJobAction::Start);
    assert_eq!(accepted.operation_id, job_id);
    let request = accepted.request.as_ref().expect("immutable job request");
    assert_eq!(request.kind, ArchiveJobKind::Create);
    assert_eq!(
        request.paths,
        vec![r"C:\synthetic\split-submit.txt".to_owned()]
    );
    assert_eq!(
        request.create.as_ref().expect("create").volume_bytes,
        Some(66_560)
    );
    let starts: Vec<_> =
        std::iter::from_fn(|| endpoint.try_receive_command_for_contract_test().ok())
            .filter_map(|envelope| match envelope.message {
                Message::PlatformCommand(PlatformCommand::ArchiveJob(command))
                    if command.action == ArchiveJobAction::Start =>
                {
                    Some(command)
                }
                _ => None,
            })
            .collect();
    assert_eq!(
        starts.len(),
        1,
        "actual Submit crosses the original PlatformBridge once"
    );
    assert_eq!(
        *starts[0], accepted,
        "JobRecord and outbound command are the same snapshot"
    );
    form.update(&mut surface, |form, _| {
        form.revise_submitted_create_volume_for_test(ArchiveVolumeSize {
            value: 128,
            unit: ArchiveVolumeUnit::MiB,
        })
    });
    owner.read_with(&surface, |app, _| {
        assert_eq!(app.jobs.records[&job_id].command, accepted)
    });
    assert_eq!(
        starts[0]
            .request
            .as_ref()
            .expect("outbound request")
            .create
            .as_ref()
            .expect("outbound create")
            .volume_bytes,
        Some(66_560)
    );
    assert!(std::iter::from_fn(|| endpoint.try_receive_command_for_contract_test().ok())
        .all(|envelope| !matches!(envelope.message, Message::PlatformCommand(PlatformCommand::ArchiveJob(command)) if command.action == ArchiveJobAction::Start)), "post-submit draft change cannot enqueue another Start");
}
