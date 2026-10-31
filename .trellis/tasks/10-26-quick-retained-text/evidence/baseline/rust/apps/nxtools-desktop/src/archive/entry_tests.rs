use super::*;
use gpui_kit::component::WindowExt as _;
use nxtools_protocol::{Message, PlatformCommand, PlatformCommandResult, archive_browser::*};
fn terminal(operation: Uuid, session: Uuid, generation: u64, lease: Uuid) -> PlatformCommandResult {
    PlatformCommandResult {
        success: true,
        error: None,
        native_error_code: None,
        data: Some(
            serde_json::to_string(&ArchiveEntryResult {
                version: 1,
                operation_id: operation,
                session_id: session,
                generation,
                success: true,
                lease_id: Some(lease),
                path: Some(r"C:\owned\cache\literal.txt".into()),
                code: None,
                native_error_code: None,
                cleanup_complete: true,
                retained: false,
            })
            .expect("typed terminal"),
        ),
    }
}
#[gpui_kit::test]
fn retired_or_cancelled_entry_reply_releases_original_lease_without_shell(
    cx: &mut gpui_kit::TestAppContext,
) {
    cx.update(gpui_kit::init);
    cx.executor().allow_parking();
    let (bridge, endpoint) = nxtools_ipc::platform_bridge(32);
    endpoint.set_connected_for_contract_test(true);
    let owner = cx.new(|cx| ArchiveApplication::new(Rc::new(bridge), None, cx));
    let operation = Uuid::new_v4();
    let session = Uuid::new_v4();
    let snapshot = Uuid::new_v4();
    let lease = Uuid::new_v4();
    owner.update(cx, |app, cx| {
        app.entries.insert(
            operation,
            PendingEntry {
                window_id: session,
                operation,
                generation: 7,
                snapshot,
                nested: false,
                depth: 0,
                started: Instant::now(),
                cancelled: true,
            },
        );
        assert!(app.entry_result(operation, &terminal(operation, session, 7, lease), cx));
    });
    let command = endpoint
        .try_receive_command_for_contract_test()
        .expect("release");
    assert!(
        matches!(command.message,Message::PlatformCommand(PlatformCommand::ArchiveEntry(ArchiveEntryCommand{action:ArchiveEntryAction::Release,operation_id,lease_id:Some(id),..}))if operation_id==operation&&id==lease)
    );
    assert!(
        endpoint.try_receive_command_for_contract_test().is_err(),
        "no Shell command"
    );
    owner.update(cx, |app, cx| {
        assert!(!app.entry_result(operation, &terminal(operation, session, 7, lease), cx))
    });
}

fn ready_browser(
    cx: &mut gpui_kit::TestAppContext,
) -> (
    Entity<ArchiveApplication>,
    nxtools_ipc::PlatformServerEndpoint,
    Uuid,
    Uuid,
) {
    cx.update(gpui_kit::init);
    cx.executor().allow_parking();
    let (bridge, endpoint) = nxtools_ipc::platform_bridge(32);
    endpoint.set_connected_for_contract_test(true);
    let owner = cx.new(|cx| ArchiveApplication::new(Rc::new(bridge), None, cx));
    owner.update(cx, |app, cx| {
        app.open_canonical(PathBuf::from(r"C:\owned\fixture.zip"), 0, vec![], cx)
    });
    cx.run_until_parked();
    let snapshot = Uuid::new_v4();
    let id = owner.update(cx, |app, _| {
        let (id, slot) = app.windows.iter_mut().next().expect("browser");
        slot.browser = Some(ArchiveBrowserContext {
            snapshot_id: snapshot,
            format: "Zip".into(),
            comment: String::new(),
            source: ArchiveSourceBinding {
                native: "v1:fixture".into(),
                digest: [0; 32],
                size: 10,
                modified_unix_ns: 0,
            },
        });
        *id
    });
    (owner, endpoint, id, snapshot)
}

#[gpui_kit::test]
fn failed_outer_mismatched_or_unclean_materialization_never_opens_shell(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (owner, endpoint, id, snapshot) = ready_browser(cx);
    for mode in 0..3 {
        let operation = Uuid::new_v4();
        let lease = Uuid::new_v4();
        let mut result = terminal(operation, id, 1, lease);
        if mode == 0 {
            result.success = false;
        }
        if mode != 0 {
            let mut value: ArchiveEntryResult =
                serde_json::from_str(result.data.as_deref().expect("data")).expect("typed");
            if mode == 1 {
                value.generation = 99;
            } else {
                value.cleanup_complete = false;
            }
            result.data = Some(serde_json::to_string(&value).expect("wire"));
        }
        owner.update(cx, |app, cx| {
            app.entries.insert(
                operation,
                PendingEntry {
                    operation,
                    window_id: id,
                    generation: 1,
                    snapshot,
                    nested: false,
                    depth: 0,
                    started: Instant::now(),
                    cancelled: false,
                },
            );
            assert!(app.entry_result(operation, &result, cx));
            assert!(app.shells.is_empty());
        });
        let release = endpoint
            .try_receive_command_for_contract_test()
            .expect("release original lease");
        assert!(
            matches!(release.message, Message::PlatformCommand(PlatformCommand::ArchiveEntry(ArchiveEntryCommand { action: ArchiveEntryAction::Release, operation_id, .. })) if operation_id == operation)
        );
        assert!(endpoint.try_receive_command_for_contract_test().is_err());
    }
}

#[gpui_kit::test]
fn source_change_retires_execution_binding_and_old_reply_cannot_restore_it(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (owner, endpoint, id, snapshot) = ready_browser(cx);
    let operation = Uuid::new_v4();
    let mut result = terminal(operation, id, 1, Uuid::new_v4());
    let mut value: ArchiveEntryResult =
        serde_json::from_str(result.data.as_deref().expect("data")).expect("typed");
    value.success = false;
    value.code = Some(ArchiveReadError::SourceChanged);
    value.path = None;
    value.lease_id = None;
    result.success = false;
    result.data = Some(serde_json::to_string(&value).expect("wire"));
    owner.update(cx, |app, cx| {
        app.entries.insert(
            operation,
            PendingEntry {
                operation,
                window_id: id,
                generation: 1,
                snapshot,
                nested: false,
                depth: 0,
                started: Instant::now(),
                cancelled: false,
            },
        );
        assert!(app.entry_result(operation, &result, cx));
        assert!(app.windows.get(&id).expect("live").browser.is_none());
        assert!(!app.entry_result(operation, &terminal(operation, id, 1, Uuid::new_v4()), cx));
    });
    assert!(endpoint.try_receive_command_for_contract_test().is_err());
}

#[gpui_kit::test]
fn active_content_escape_clears_pending_without_materialize_or_shell(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (owner, endpoint, id, snapshot) = ready_browser(cx);
    let (source, view) = owner.read_with(cx, |app, _| {
        let slot = app.windows.get(&id).expect("browser");
        (slot.window, slot.view.upgrade().expect("view"))
    });
    let mut surface = gpui_kit::VisualTestContext::from_window(source, cx);
    source.update(cx, |_, window, cx| {
        view.update(cx, |view, cx| {
            let entry: ArchiveReadEntry = serde_json::from_value(serde_json::json!({"ordinal":0,"name":"script.hta","kind":"File","size":8,"packed_size":8,"encrypted":false,"warning_codes":[],"provenance":"Utf8Flag","codepage":65001})).expect("metadata");
            view.install(snapshot, vec![entry], "Zip".into(), window, cx);
            view.model.as_mut().expect("model").select(crate::archive::model::RowId::Entry(snapshot, 0), false, false);
            view.open_pending = true;
        });
    }).expect("window");
    cx.run_until_parked();
    owner.update(cx, |app, cx| {
        app.open_entry(id, crate::archive::model::RowId::Entry(snapshot, 0), cx)
    });
    cx.run_until_parked();
    surface.update(|window, cx| {
        window.activate_window();
        window.draw(cx).clear(cx);
        assert!(window.has_active_dialog(cx));
    });
    surface.simulate_keystrokes("escape");
    surface.run_until_parked();
    assert!(!view.read_with(&surface, |view, _| view.open_pending));
    assert!(
        endpoint.try_receive_command_for_contract_test().is_err(),
        "consent cancel sends neither materialize nor Shell"
    );
}

#[test]
fn active_content_warning_includes_scripts_shortcuts_and_macro_documents() {
    for name in [
        "x.EXE", "x.hta", "x.url", "x.reg", "x.docm", "x.xlsm", "x.psm1",
    ] {
        assert!(entry::active_content(name), "{name}");
    }
    assert!(!entry::active_content("notes.txt"));
}
