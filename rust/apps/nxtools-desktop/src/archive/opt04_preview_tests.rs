use super::*;
use crate::archive::model::RowId;
use nxtools_protocol::archive_browser::{ArchiveEntryAction, ArchiveEntryCommand};

fn notify_preview(
    owner: &Entity<ArchiveApplication>,
    receipt: Uuid,
    command: &ArchiveEntryCommand,
    lease: Uuid,
    path: String,
    cx: &mut TestAppContext,
) {
    owner.update(cx, |app, cx| {
        app.notification(
            PlatformNotification::CommandResult {
                correlation_id: receipt,
                result: preview_reply(command, lease, path),
            },
            cx,
        );
    });
    cx.run_until_parked();
}

fn assert_release(endpoint: &nxtools_ipc::PlatformServerEndpoint, operation: Uuid, lease: Uuid) {
    let wire = endpoint
        .try_receive_command_for_contract_test()
        .expect("original lease release");
    assert!(
        matches!(wire.message,
            Message::PlatformCommand(PlatformCommand::ArchiveEntry(command))
                if command.action == ArchiveEntryAction::Release
                    && command.operation_id == operation
                    && command.lease_id == Some(lease)
        ),
        "exact original operation and lease released"
    );
}

fn assert_no_preview_release(endpoint: &nxtools_ipc::PlatformServerEndpoint) {
    while let Ok(wire) = endpoint.try_receive_command_for_contract_test() {
        assert!(
            !matches!(
                &wire.message,
                Message::PlatformCommand(PlatformCommand::ArchiveEntry(ArchiveEntryCommand {
                    action: ArchiveEntryAction::Release,
                    ..
                }))
            ),
            "visible Ready must retain the original native preview lease"
        );
        assert!(
            !matches!(
                &wire.message,
                Message::PlatformCommand(PlatformCommand::ArchiveEntry(ArchiveEntryCommand {
                    action: ArchiveEntryAction::Preview | ArchiveEntryAction::PreviewText,
                    ..
                }))
            ),
            "no replacement preview while current Ready is visible"
        );
    }
}

#[gpui_kit::test]
fn opt04_preview_ready_retains_original_lease_until_retirement(cx: &mut TestAppContext) {
    let (owner, endpoint, id, snapshot) = preview_fixture(cx);
    let directory = tempfile::tempdir().expect("owned preview input");
    let path = directory.path().join("item.txt");
    std::fs::write(&path, "OPT04 visible preview text").expect("owned preview bytes");
    let row_a = RowId::Entry(snapshot, 0);
    let row_b = RowId::Entry(snapshot, 1);
    let first_token = preview_select(&owner, id, row_a.clone(), cx);
    owner.update(cx, |app, cx| {
        app.preview_entry(id, Some(row_a.clone()), first_token, cx)
    });
    let (first_receipt, first_command) = next_preview(&endpoint);
    assert_eq!(first_command.ordinal, Some(0));
    assert_eq!(first_command.action, ArchiveEntryAction::PreviewText);
    let first_lease = Uuid::new_v4();
    notify_preview(
        &owner,
        first_receipt,
        &first_command,
        first_lease,
        path.to_string_lossy().into_owned(),
        cx,
    );
    let window = owner.read_with(cx, |app, _| app.windows[&id].window);
    let mut surface = gpui_kit::VisualTestContext::from_window(window, cx);
    surface.refresh().expect("original preview presentation");
    assert!(
        surface
            .debug_bounds("archive-preview-document-scroll")
            .is_some(),
        "real background prepare produced visible text Ready, not a synthetic result"
    );
    assert!(owner.read_with(&surface, |app, _| app.previews[&id].running.is_none()));
    assert_no_preview_release(&endpoint);

    let second_token = preview_select(&owner, id, row_b.clone(), &mut surface);
    owner.update(&mut surface, |app, cx| {
        app.preview_entry(id, Some(row_b.clone()), second_token, cx)
    });
    assert!(
        second_token > first_token,
        "new selection retires the visible preview"
    );
    assert_release(&endpoint, first_command.operation_id, first_lease);
    let (second_receipt, second_command) = next_preview(&endpoint);
    assert_eq!(second_command.ordinal, Some(1));
    let third_token = preview_select(&owner, id, row_a.clone(), &mut surface);
    owner.update(&mut surface, |app, cx| {
        app.preview_entry(id, Some(row_a.clone()), third_token, cx)
    });
    assert!(
        third_token > second_token,
        "A -> B -> A must not reuse the old token"
    );
    let cancel = endpoint
        .try_receive_command_for_contract_test()
        .expect("cancel B");
    assert!(matches!(cancel.message,
        Message::PlatformCommand(PlatformCommand::ArchiveEntry(command))
            if command.action == ArchiveEntryAction::Cancel
                && command.operation_id == second_command.operation_id
    ));
    let stale_lease = Uuid::new_v4();
    notify_preview(
        &owner,
        second_receipt,
        &second_command,
        stale_lease,
        r"C:\never-read-stale.txt".into(),
        &mut surface,
    );
    assert_release(&endpoint, second_command.operation_id, stale_lease);
    surface.refresh().expect("original preview still loading");
    assert!(
        surface
            .debug_bounds("archive-preview-document-scroll")
            .is_none(),
        "retired B cannot publish stale Ready into the newer A selection"
    );
    let (third_receipt, third_command) = next_preview(&endpoint);
    assert_eq!(third_command.ordinal, Some(0));
    assert_ne!(third_command.operation_id, first_command.operation_id);
    let third_lease = Uuid::new_v4();
    notify_preview(
        &owner,
        third_receipt,
        &third_command,
        third_lease,
        path.to_string_lossy().into_owned(),
        &mut surface,
    );
    surface.refresh().expect("new A preview");
    assert!(
        surface
            .debug_bounds("archive-preview-document-scroll")
            .is_some()
    );
    assert_no_preview_release(&endpoint);
    owner.update(&mut surface, |app, cx| app.close_browser(id, cx));
    assert_release(&endpoint, third_command.operation_id, third_lease);
    surface.run_until_parked();
    assert!(owner.read_with(&surface, |app, _| !app.previews.contains_key(&id)));
    assert!(
        endpoint.try_receive_command_for_contract_test().is_err(),
        "each native lease released once"
    );
}

#[gpui_kit::test]
fn opt04_preview_ready_refresh_codepage_retires_lease(cx: &mut TestAppContext) {
    let (owner, endpoint, id, snapshot) = preview_fixture(cx);
    let directory = tempfile::tempdir().expect("owned preview input");
    let path = directory.path().join("text.txt");
    std::fs::write(&path, "refresh generation").expect("owned preview bytes");
    let row = RowId::Entry(snapshot, 0);
    let token = preview_select(&owner, id, row.clone(), cx);
    owner.update(cx, |app, cx| app.preview_entry(id, Some(row), token, cx));
    let (receipt, command) = next_preview(&endpoint);
    let lease = Uuid::new_v4();
    notify_preview(
        &owner,
        receipt,
        &command,
        lease,
        path.to_string_lossy().into_owned(),
        cx,
    );
    let window = owner.read_with(cx, |app, _| app.windows[&id].window);
    let mut surface = gpui_kit::VisualTestContext::from_window(window, cx);
    surface.refresh().expect("visible original preview");
    assert!(
        surface
            .debug_bounds("archive-preview-document-scroll")
            .is_some()
    );
    assert_no_preview_release(&endpoint);
    let prior_generation = owner.read_with(&surface, |app, _| app.windows[&id].generation);
    owner.update(&mut surface, |app, cx| app.refresh_browser(id, 936, cx));
    assert_release(&endpoint, command.operation_id, lease);
    assert!(owner.read_with(&surface, |app, _| {
        let slot = &app.windows[&id];
        slot.generation > prior_generation && slot.browser.is_none()
    }));
    surface.run_until_parked();
    let mut new_read = false;
    while let Ok(wire) = endpoint.try_receive_command_for_contract_test() {
        match wire.message {
            Message::PlatformCommand(PlatformCommand::ArchiveBrowserRead(read))
                if read.codepage == 936 && read.generation > prior_generation =>
            {
                new_read = true
            }
            Message::PlatformCommand(PlatformCommand::ArchiveEntry(entry)) => {
                assert_ne!(
                    entry.action,
                    ArchiveEntryAction::Release,
                    "refresh releases exactly once"
                );
                assert_ne!(
                    entry.action,
                    ArchiveEntryAction::PreviewText,
                    "old snapshot cannot start a new preview"
                );
            }
            _ => {}
        }
    }
    assert!(
        new_read,
        "original refresh uses updated encoding and generation"
    );
    owner.update(&mut surface, |app, cx| app.close_browser(id, cx));
}

#[gpui_kit::test]
fn opt04_preview_ready_tab_switch_retires_original_lease(cx: &mut TestAppContext) {
    let (owner, endpoint, id, snapshot) = preview_fixture(cx);
    let directory = tempfile::tempdir().expect("owned preview input");
    let path = directory.path().join("preview.txt");
    std::fs::write(&path, "tab preview").expect("owned preview bytes");
    let row = RowId::Entry(snapshot, 0);
    let token = preview_select(&owner, id, row.clone(), cx);
    owner.update(cx, |app, cx| {
        app.preview_entry(id, Some(row.clone()), token, cx)
    });
    let (receipt, command) = next_preview(&endpoint);
    let lease = Uuid::new_v4();
    notify_preview(
        &owner,
        receipt,
        &command,
        lease,
        path.to_string_lossy().into_owned(),
        cx,
    );
    let window = owner.read_with(cx, |app, _| app.windows[&id].window);
    let mut surface = gpui_kit::VisualTestContext::from_window(window, cx);
    surface.refresh().expect("visible preview on original tab");
    assert!(
        surface
            .debug_bounds("archive-preview-document-scroll")
            .is_some()
    );
    assert_no_preview_release(&endpoint);

    owner.update(&mut surface, |app, cx| {
        app.open_canonical(
            PathBuf::from(r"C:\owned\independent-tab.zip"),
            0,
            vec![],
            cx,
        );
    });
    surface.run_until_parked();
    let old_view = owner.read_with(&surface, |app, _| {
        app.windows[&id].view.upgrade().expect("old tab")
    });
    assert!(!old_view.read_with(&surface, |view, _| {
        view.preview_selection_matches(token, &Some(row))
    }));
    let mut releases = 0;
    while let Ok(wire) = endpoint.try_receive_command_for_contract_test() {
        if let Message::PlatformCommand(PlatformCommand::ArchiveEntry(entry)) = wire.message {
            assert_eq!(
                entry.action,
                ArchiveEntryAction::Release,
                "hidden Ready cannot restart preview"
            );
            assert_eq!(
                (entry.operation_id, entry.lease_id),
                (command.operation_id, Some(lease))
            );
            releases += 1;
        }
    }
    assert_eq!(
        releases, 1,
        "switching tabs releases only the visible preview share"
    );
    owner.update(&mut surface, |app, cx| app.close_browser(id, cx));
    assert!(
        endpoint.try_receive_command_for_contract_test().is_err(),
        "tab close does not release twice"
    );
    let remaining = owner.read_with(&surface, |app, _| {
        app.windows.keys().copied().collect::<Vec<_>>()
    });
    for id in remaining {
        owner.update(&mut surface, |app, cx| app.close_browser(id, cx));
    }
}

#[gpui_kit::test]
fn opt04_preview_failed_decode_releases_original_lease_once(cx: &mut TestAppContext) {
    let (owner, endpoint, id, snapshot) = preview_fixture(cx);
    let directory = tempfile::tempdir().expect("owned preview input");
    let path = directory.path().join("broken.txt");
    std::fs::write(&path, [0xff]).expect("owned invalid UTF-8");
    let row = RowId::Entry(snapshot, 0);
    let token = preview_select(&owner, id, row.clone(), cx);
    owner.update(cx, |app, cx| app.preview_entry(id, Some(row), token, cx));
    let (receipt, command) = next_preview(&endpoint);
    let lease = Uuid::new_v4();
    notify_preview(
        &owner,
        receipt,
        &command,
        lease,
        path.to_string_lossy().into_owned(),
        cx,
    );
    assert!(owner.read_with(cx, |app, _| app.previews[&id].running.is_none()));
    assert_release(&endpoint, command.operation_id, lease);
    let window = owner.read_with(cx, |app, _| app.windows[&id].window);
    let mut surface = gpui_kit::VisualTestContext::from_window(window, cx);
    surface.refresh().expect("failure projection");
    assert!(
        surface
            .debug_bounds("archive-preview-document-scroll")
            .is_none()
    );
    owner.update(&mut surface, |app, cx| app.close_browser(id, cx));
    assert!(
        endpoint.try_receive_command_for_contract_test().is_err(),
        "failure/close must not release twice"
    );
}

#[gpui_kit::test]
fn preview_budget_snapshot_survives_settings_change_and_next_selection_uses_latest(
    cx: &mut TestAppContext,
) {
    let (owner, endpoint, id, snapshot) = preview_fixture(cx);
    let directory = tempfile::tempdir().expect("owned preview input");
    let path = directory.path().join("large.txt");
    std::fs::write(
        &path,
        "ordinary archive preview text budget snapshot\n".repeat(2000),
    )
    .expect("owned text");
    let row = RowId::Entry(snapshot, 0);
    let token = preview_select(&owner, id, row.clone(), cx);
    owner.update(cx, |app, cx| {
        app.preview_entry(id, Some(row.clone()), token, cx)
    });
    let (receipt, command) = next_preview(&endpoint);
    let wire = serde_json::to_value(&command).expect("actual preview wire");
    assert_eq!(
        wire["preview_options"],
        serde_json::json!({"max_bytes":1048576,"timeout_ms":120000})
    );
    owner.update(cx, |app, _| {
        app.settings.preview.text_limit_kib = 1;
        app.settings.preview.text_max_lines = 1;
        app.settings.preview.timeout_seconds = 5;
    });
    let lease = Uuid::new_v4();
    notify_preview(
        &owner,
        receipt,
        &command,
        lease,
        path.to_string_lossy().into_owned(),
        cx,
    );
    let handle = owner.read_with(cx, |app, _| app.windows[&id].window);
    let mut surface = gpui_kit::VisualTestContext::from_window(handle, cx);
    surface.refresh().expect("frozen budget render");
    assert!(
        surface
            .debug_bounds("archive-preview-document-scroll")
            .is_some(),
        "running preview retains original text/render budget"
    );
    let token = preview_select(&owner, id, RowId::Entry(snapshot, 1), &mut surface);
    owner.update(&mut surface, |app, cx| {
        app.preview_entry(id, Some(RowId::Entry(snapshot, 1)), token, cx)
    });
    assert_release(&endpoint, command.operation_id, lease);
    let (_, next) = next_preview(&endpoint);
    assert_eq!(
        serde_json::to_value(next).expect("next request")["preview_options"],
        serde_json::json!({"max_bytes":1024,"timeout_ms":5000})
    );
    owner.update(&mut surface, |app, cx| app.close_browser(id, cx));
}
