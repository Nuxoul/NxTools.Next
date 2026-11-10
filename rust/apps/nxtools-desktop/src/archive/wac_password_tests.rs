//! WAC production Root/Dialog/entry/host boundaries; no native runtime or user data.
use super::*;
use gpui_kit::{Focusable as _, KeyDownEvent, Keystroke, TestAppContext, VisualTestContext};
use nxtools_protocol::{Message, PlatformCommandResult, PlatformEvent, archive_browser::*};

fn draw(surface: &mut VisualTestContext) {
    surface.run_until_parked();
    surface.update(|window, cx| {
        window.activate_window();
        window.draw(cx).clear(cx);
    });
    surface.run_until_parked();
}
fn drain_frames(surface: &mut VisualTestContext) {
    draw(surface);
    // Exercise the original Dialog deferred-close animation/focus completion, not a new retry.
    surface
        .executor()
        .advance_clock(std::time::Duration::from_secs(1));
    draw(surface);
    draw(surface);
}
fn loading(
    cx: &mut TestAppContext,
) -> (
    Entity<ArchiveApplication>,
    nxtools_ipc::PlatformServerEndpoint,
    ArchiveReadCommand,
    VisualTestContext,
) {
    let (owner, endpoint) = super::super::tests::setup(cx);
    owner.update(cx, |app, cx| {
        app.open_canonical(PathBuf::from(r"C:\owned\wac-encrypted.zip"), 0, vec![], cx)
    });
    cx.run_until_parked();
    let wire = endpoint
        .try_receive_command_for_contract_test()
        .expect("original read start");
    let Message::PlatformCommand(PlatformCommand::ArchiveBrowserRead(read)) = wire.message else {
        panic!("read start");
    };
    let window = owner.read_with(cx, |app, _| app.windows[&read.session_id].window);
    let mut surface = VisualTestContext::from_window(window, cx);
    draw(&mut surface);
    (owner, endpoint, read, surface)
}
fn prompt(operation: Uuid, session: Uuid, generation: u64) -> ArchivePasswordPrompt {
    ArchivePasswordPrompt {
        operation_id: operation,
        session_id: session,
        generation,
        prompt_id: Uuid::new_v4(),
        ticket_generation: 1,
        error_code: "PasswordRequired".into(),
        attempt: 1,
    }
}
fn password_event(
    owner: &Entity<ArchiveApplication>,
    prompt: ArchivePasswordPrompt,
    cx: &mut impl gpui_kit::AppContext,
) {
    owner.update(cx, |app, cx| {
        app.notification(
            PlatformNotification::Event(PlatformEvent::ArchivePassword(prompt)),
            cx,
        )
    });
}
fn password_nodes(surface: &mut VisualTestContext) -> usize {
    surface.update(|window, _| {
        gpui_kit::base::test_support::snapshots(window)
            .iter()
            .filter(|node| node.label() == Some("压缩包密码"))
            .count()
    })
}

#[gpui_kit::test]
fn wac01_competing_drop_dialog_close_resumes_current_password_once(cx: &mut TestAppContext) {
    let (owner, _endpoint, read, mut surface) = loading(cx);
    let source = surface.update(|window, _| window.window_handle());
    owner.update(&mut surface, |app, cx| {
        app.handle_drop(
            read.session_id,
            vec![PathBuf::from(r"C:\owned\drop.zip")],
            source,
            cx,
        )
    });
    draw(&mut surface);
    let ticket = prompt(read.operation_id, read.session_id, read.generation);
    password_event(&owner, ticket.clone(), &mut surface);
    draw(&mut surface);
    assert_eq!(
        password_nodes(&mut surface),
        0,
        "original competing modal must stay above waiting password"
    );
    surface.simulate_keystrokes("escape");
    drain_frames(&mut surface);
    assert_eq!(
        password_nodes(&mut surface),
        1,
        "closing the actual drop decision must show matching password without tab activation"
    );
    password_event(&owner, ticket.clone(), &mut surface);
    drain_frames(&mut surface);
    assert_eq!(
        password_nodes(&mut surface),
        1,
        "duplicate event cannot stack password dialogs"
    );
    assert_eq!(
        owner.read_with(&surface, |app, _| app.password_dialogs.len()),
        1
    );
    let host = owner.read_with(&surface, |app, _| app.host.as_ref().expect("host").id);
    owner.update(&mut surface, |app, cx| app.close_host(host, cx));
    surface.run_until_parked();
}

#[gpui_kit::test]
fn wac02_ordinary_entry_terminal_retires_password_input_and_late_reply(cx: &mut TestAppContext) {
    ordinary_terminal(cx, "timeout", false);
}

#[gpui_kit::test]
fn wac02_failure_under_newer_dialog_retires_only_original_scope(cx: &mut TestAppContext) {
    ordinary_terminal(cx, "failure", true);
}

#[gpui_kit::test]
fn wac02_cancel_terminal_under_newer_dialog_is_idempotent(cx: &mut TestAppContext) {
    ordinary_terminal(cx, "cancel", true);
}

fn ordinary_terminal(cx: &mut TestAppContext, outcome: &str, covered: bool) {
    let (owner, endpoint, read, mut surface) = loading(cx);
    let snapshot = Uuid::new_v4();
    let view = owner.update(&mut surface, |app, cx| {
        app.reads.clear();
        let slot = app.windows.get_mut(&read.session_id).expect("original session");
        slot.browser = Some(ArchiveBrowserContext { snapshot_id:snapshot, format:"Zip".into(), comment:String::new(),
            sevenz_edit:None,
            tar_edit:None,
            source:ArchiveSourceBinding { native:"v1:wac".into(), digest:[0;32], size:10, modified_unix_ns:0 } });
        let view = slot.view.upgrade().expect("view");
        slot.window.update(cx, |_, window, cx| view.update(cx, |view, cx| {
            let entry = serde_json::from_value(serde_json::json!({"ordinal":0,"name":"ordinary.txt","kind":"File","size":3,"packed_size":3,"encrypted":true,"warning_codes":[],"provenance":"Utf8Flag","codepage":65001})).expect("metadata");
            view.install(snapshot, vec![entry], "Zip".into(), window, cx);
        })).expect("host");
        view
    });
    draw(&mut surface);
    while endpoint.try_receive_command_for_contract_test().is_ok() {}
    owner.update(&mut surface, |app, cx| {
        app.open_entry(
            read.session_id,
            crate::archive::model::RowId::Entry(snapshot, 0),
            cx,
        )
    });
    let (receipt, command) = loop {
        let wire = endpoint
            .try_receive_command_for_contract_test()
            .expect("original ordinary entry dispatch");
        if let Message::PlatformCommand(PlatformCommand::ArchiveEntry(command)) = wire.message
            && command.action == ArchiveEntryAction::Materialize
        {
            break (wire.id, command);
        }
    };
    let ticket = prompt(command.operation_id, command.session_id, command.generation);
    password_event(&owner, ticket.clone(), &mut surface);
    draw(&mut surface);
    let (input, alive) = owner.read_with(&surface, |app, _| {
        let dialog = &app.password_dialogs[&command.operation_id];
        (dialog.input.clone(), dialog.alive.clone())
    });
    surface.update(|window, cx| {
        input.update(cx, |input, cx| {
            input.set_value("synthetic-wac-sentinel", window, cx)
        })
    });
    let newer = covered.then(|| overlay(&mut surface));
    if outcome == "cancel" {
        owner.update(&mut surface, |app, _| app.cancel_entries(read.session_id));
    }
    owner.update(&mut surface, |app, cx| {
        app.notification(
            PlatformNotification::CommandResult {
                correlation_id: receipt,
                result: PlatformCommandResult {
                    success: false,
                    error: Some(outcome.into()),
                    native_error_code: None,
                    data: None,
                },
            },
            cx,
        )
    });
    draw(&mut surface);
    assert!(
        !alive.get(),
        "ordinary terminal must retire the original password alive token"
    );
    assert!(
        input.read_with(&surface, |input, _| input.value().is_empty()),
        "retired password input must be cleared"
    );
    assert!(owner.read_with(
        &surface,
        |app, _| !app.passwords.contains_key(&command.operation_id)
            && !app.password_dialogs.contains_key(&command.operation_id)
    ));
    if let Some(newer) = newer {
        if outcome != "cancel" {
            // Keep the existing failure-notice semantics above the newer business modal.
            surface.simulate_keystrokes("escape");
            drain_frames(&mut surface);
        }
        assert!(
            surface.update(|window, cx| newer.contains_focused(window, cx)),
            "terminal cannot pop the newer business modal"
        );
        assert!(owner.read_with(&surface, |app, _| {
            app.host
                .as_ref()
                .expect("original host")
                .password_recovery
                .retired
                .len()
                == 1
        }));
        surface.simulate_keystrokes("escape");
        drain_frames(&mut surface);
        assert!(
            !surface.update(|window, cx| window.has_active_dialog(cx)),
            "restored retired scope must be removed automatically"
        );
    }
    assert_eq!(
        password_nodes(&mut surface),
        0,
        "failure notice may remain, but no stale password control"
    );
    assert!(!view.read_with(&surface, |view, _| view.open_pending));
    while endpoint.try_receive_command_for_contract_test().is_ok() {}
    owner.update(&mut surface, |app, cx| {
        app.password_reply(ticket, Some(PrivatePassword::new("late-fixture")), cx)
    });
    surface.run_until_parked();
    assert!(
        endpoint.try_receive_command_for_contract_test().is_err(),
        "late confirmation cannot send another password"
    );
    // Duplicate native result and late cancel are no-ops; then a fresh original operation works.
    owner.update(&mut surface, |app, cx| {
        assert!(!app.entry_result(
            receipt,
            &PlatformCommandResult {
                success: false,
                error: None,
                native_error_code: None,
                data: None
            },
            cx
        ));
        app.password_reply(
            prompt(command.operation_id, command.session_id, command.generation),
            None,
            cx,
        );
    });
    if surface.update(|window, cx| window.has_active_dialog(cx)) {
        surface.simulate_keystrokes("escape");
        drain_frames(&mut surface);
    }
    owner.update(&mut surface, |app, cx| {
        app.open_entry(
            read.session_id,
            crate::archive::model::RowId::Entry(snapshot, 0),
            cx,
        )
    });
    let next = loop {
        let wire = endpoint
            .try_receive_command_for_contract_test()
            .expect("next operation");
        if let Message::PlatformCommand(PlatformCommand::ArchiveEntry(next)) = wire.message
            && next.action == ArchiveEntryAction::Materialize
        {
            break next;
        }
    };
    assert_ne!(next.operation_id, command.operation_id);
    password_event(
        &owner,
        prompt(next.operation_id, next.session_id, next.generation),
        &mut surface,
    );
    draw(&mut surface);
    assert_eq!(
        password_nodes(&mut surface),
        1,
        "next operation gets a fresh password input"
    );
    let host = owner.read_with(&surface, |app, _| app.host.as_ref().expect("host").id);
    owner.update(&mut surface, |app, cx| app.close_host(host, cx));
    surface.run_until_parked();
}

#[gpui_kit::test]
fn wac03_retained_tab_after_old_password_close_has_current_focus_and_keyboard(
    cx: &mut TestAppContext,
) {
    let (owner, _endpoint, read, mut surface) = loading(cx);
    let path = PathBuf::from(r"C:\owned\wac-visible-b.zip");
    owner.update(&mut surface, |app, cx| {
        app.open_canonical(path.clone(), 0, vec![], cx)
    });
    draw(&mut surface);
    owner.update(&mut surface, |app, cx| {
        app.activate_browser(read.session_id, cx)
    });
    draw(&mut surface);
    let ticket = prompt(read.operation_id, read.session_id, read.generation);
    password_event(&owner, ticket.clone(), &mut surface);
    draw(&mut surface);
    // The original duplicate-open ingress activates retained B exactly once.
    owner.update(&mut surface, |app, cx| {
        app.open_canonical(path, 0, vec![], cx)
    });
    drain_frames(&mut surface);
    let (active, view, old) = owner.read_with(&surface, |app, cx| {
        let id = app
            .host
            .as_ref()
            .expect("host")
            .view
            .read_with(cx, |host, _| host.active)
            .expect("host entity")
            .expect("active");
        (
            id,
            app.windows[&id].view.upgrade().expect("B"),
            app.windows[&read.session_id].view.upgrade().expect("A"),
        )
    });
    assert_ne!(active, read.session_id);
    surface.update(|window, cx| view.update(cx, |view, cx| {
        let entry = serde_json::from_value(serde_json::json!({"ordinal":0,"name":"visible.txt","kind":"File","size":3,"packed_size":3,"encrypted":false,"warning_codes":[],"provenance":"Utf8Flag","codepage":65001})).expect("B metadata");
        view.install(Uuid::new_v4(),vec![entry],"Zip".into(),window,cx);
    }));
    draw(&mut surface);
    assert!(
        surface.update(|window, cx| view.focus_handle(cx).is_focused(window)),
        "visible B must own its original focus after old Dialog restores previous A focus"
    );
    surface.simulate_event(KeyDownEvent {
        keystroke: Keystroke::parse("ctrl-f").expect("keyboard"),
        is_held: false,
        prefer_character_input: false,
    });
    draw(&mut surface);
    assert!(
        surface.update(|window, _| gpui_kit::base::test_support::snapshots(window)
            .iter()
            .any(|node| node.label() == Some("筛选当前文件夹") && node.focused() == Some(true))),
        "CtrlF must reach visible B original input"
    );
    assert!(
        old.read_with(&surface, |view, _| view.model.is_none()),
        "hidden A source state unchanged"
    );
    assert!(
        owner.read_with(&surface, |app, _| app.passwords.get(&read.operation_id)
            == Some(&ticket))
    );
    owner.update(&mut surface, |app, cx| {
        app.activate_browser(read.session_id, cx)
    });
    drain_frames(&mut surface);
    assert_eq!(
        password_nodes(&mut surface),
        1,
        "A resumes the retained original ticket"
    );
    let host = owner.read_with(&surface, |app, _| app.host.as_ref().expect("host").id);
    owner.update(&mut surface, |app, cx| app.close_host(host, cx));
    surface.run_until_parked();
}

fn overlay(surface: &mut VisualTestContext) -> gpui_kit::FocusHandle {
    let handle = surface.update(|window, cx| {
        window.open_dialog(cx, |dialog, _, _| {
            dialog
                .title("newer WAC fixture")
                .child("newer modal")
                .footer(DialogClose::new().trigger(|button| button.label("关闭")))
        });
        window.focused(cx).expect("newer modal scope")
    });
    draw(surface);
    handle
}

fn finish(owner: &Entity<ArchiveApplication>, surface: &mut VisualTestContext) {
    let host = owner.read_with(surface, |app, _| app.host.as_ref().expect("host").id);
    owner.update(surface, |app, cx| app.close_host(host, cx));
    surface.run_until_parked();
}

fn drop_wait(
    owner: &Entity<ArchiveApplication>,
    read: &ArchiveReadCommand,
    surface: &mut VisualTestContext,
) -> ArchivePasswordPrompt {
    let source = surface.update(|window, _| window.window_handle());
    owner.update(surface, |app, cx| {
        app.handle_drop(
            read.session_id,
            vec![PathBuf::from(r"C:\owned\fixture.zip")],
            source,
            cx,
        )
    });
    draw(surface);
    let ticket = prompt(read.operation_id, read.session_id, read.generation);
    password_event(owner, ticket.clone(), surface);
    password_event(owner, ticket.clone(), surface);
    draw(surface);
    ticket
}

#[gpui_kit::test]
fn wac01_newer_overlay_delays_duplicate_waiting_ticket_until_last_close(cx: &mut TestAppContext) {
    let (owner, _, read, mut surface) = loading(cx);
    drop_wait(&owner, &read, &mut surface);
    let newer = overlay(&mut surface);
    password_event(
        &owner,
        prompt(Uuid::new_v4(), read.session_id, read.generation),
        &mut surface,
    );
    draw(&mut surface);
    assert!(surface.update(|window, cx| newer.contains_focused(window, cx)));
    assert_eq!(password_nodes(&mut surface), 0);
    surface.simulate_keystrokes("escape");
    drain_frames(&mut surface);
    assert_eq!(
        password_nodes(&mut surface),
        0,
        "original drop dialog still owns modality"
    );
    surface.simulate_keystrokes("escape");
    drain_frames(&mut surface);
    assert_eq!(password_nodes(&mut surface), 1);
    assert_eq!(
        owner.read_with(&surface, |app, _| app.password_dialogs.len()),
        1
    );
    finish(&owner, &mut surface);
}

#[gpui_kit::test]
fn wac01_cancelled_or_generation_replaced_ticket_never_resumes(cx: &mut TestAppContext) {
    let (owner, _, read, mut surface) = loading(cx);
    let old = drop_wait(&owner, &read, &mut surface);
    owner.update(&mut surface, |app, cx| {
        app.refresh_browser(read.session_id, 0, cx)
    });
    password_event(&owner, old, &mut surface);
    surface.simulate_keystrokes("escape");
    drain_frames(&mut surface);
    assert_eq!(password_nodes(&mut surface), 0);
    assert!(owner.read_with(&surface, |app, _| app.passwords.is_empty()
        && app.password_dialogs.is_empty()));
    finish(&owner, &mut surface);
}

#[gpui_kit::test]
fn wac01_waiting_ticket_stays_background_and_closed_tab_never_resumes(cx: &mut TestAppContext) {
    let (owner, _, read, mut surface) = loading(cx);
    let ticket = drop_wait(&owner, &read, &mut surface);
    owner.update(&mut surface, |app, cx| {
        app.open_canonical(PathBuf::from(r"C:\owned\background-b.zip"), 0, vec![], cx)
    });
    draw(&mut surface);
    surface.simulate_keystrokes("escape");
    drain_frames(&mut surface);
    assert_eq!(
        password_nodes(&mut surface),
        0,
        "background A cannot cover B"
    );
    assert!(
        owner.read_with(&surface, |app, _| app.passwords.get(&read.operation_id)
            == Some(&ticket))
    );
    owner.update(&mut surface, |app, cx| {
        app.close_browser(read.session_id, cx)
    });
    password_event(&owner, ticket, &mut surface);
    drain_frames(&mut surface);
    assert_eq!(password_nodes(&mut surface), 0);
    assert!(owner.read_with(&surface, |app, _| {
        !app.passwords.contains_key(&read.operation_id)
    }));
    finish(&owner, &mut surface);
}

#[gpui_kit::test]
fn wac01_disconnected_waiting_operation_never_resumes(cx: &mut TestAppContext) {
    let (owner, _, read, mut surface) = loading(cx);
    drop_wait(&owner, &read, &mut surface);
    owner.update(&mut surface, |app, cx| {
        app.notification(
            PlatformNotification::Disconnected {
                generation: 1,
                pending_command_ids: vec![],
            },
            cx,
        )
    });
    surface.simulate_keystrokes("escape");
    drain_frames(&mut surface);
    assert_eq!(password_nodes(&mut surface), 0);
    finish(&owner, &mut surface);
}

#[gpui_kit::test]
fn wac03_actual_new_b_focus_then_rapid_c_close_uses_current_session(cx: &mut TestAppContext) {
    let (owner, _, read, mut surface) = loading(cx);
    let ticket = prompt(read.operation_id, read.session_id, read.generation);
    password_event(&owner, ticket.clone(), &mut surface);
    draw(&mut surface);
    owner.update(&mut surface, |app, cx| {
        app.open_canonical(PathBuf::from(r"C:\owned\brand-new-b.zip"), 0, vec![], cx)
    });
    drain_frames(&mut surface);
    let (b, bview) = active_view(&owner, &surface);
    assert!(surface.update(|window, cx| bview.focus_handle(cx).is_focused(window)));
    surface.update(|window, cx| bview.update(cx, |view, cx| {
        let entry = serde_json::from_value(serde_json::json!({"ordinal":0,"name":"new-b.txt","kind":"File","size":3,"packed_size":3,"encrypted":false,"warning_codes":[],"provenance":"Utf8Flag","codepage":65001})).expect("new B metadata");
        view.install(Uuid::new_v4(),vec![entry],"Zip".into(),window,cx);
    }));
    draw(&mut surface);
    surface.simulate_keystrokes("ctrl-f");
    draw(&mut surface);
    assert!(
        surface.update(|window, _| gpui_kit::base::test_support::snapshots(window)
            .iter()
            .any(|node| node.label() == Some("筛选当前文件夹") && node.focused() == Some(true))),
        "actual new B must receive CtrlF through its original focus chain"
    );
    let old = owner.read_with(&surface, |app, _| {
        app.windows[&read.session_id]
            .view
            .upgrade()
            .expect("hidden A")
    });
    assert!(
        old.read_with(&surface, |view, _| view.model.is_none()),
        "new B keyboard must not mutate hidden A"
    );
    owner.update(&mut surface, |app, cx| {
        app.activate_browser(read.session_id, cx);
        app.open_canonical(PathBuf::from(r"C:\owned\rapid-c.zip"), 0, vec![], cx);
        app.close_browser(b, cx);
    });
    drain_frames(&mut surface);
    let (c, cview) = active_view(&owner, &surface);
    assert_ne!(c, b);
    assert_ne!(c, read.session_id);
    assert!(surface.update(|window, cx| cview.focus_handle(cx).is_focused(window)));
    assert!(!surface.update(|window, cx| bview.focus_handle(cx).is_focused(window)));
    owner.update(&mut surface, |app, cx| {
        app.activate_browser(read.session_id, cx)
    });
    drain_frames(&mut surface);
    assert_eq!(password_nodes(&mut surface), 1);
    finish(&owner, &mut surface);
}

fn active_view(
    owner: &Entity<ArchiveApplication>,
    surface: &VisualTestContext,
) -> (Uuid, Entity<ArchiveBrowser>) {
    owner.read_with(surface, |app, cx| {
        let active = app
            .host
            .as_ref()
            .expect("active host")
            .view
            .read_with(cx, |host, _| host.active)
            .expect("live host entity")
            .expect("active session");
        (
            active,
            app.windows[&active].view.upgrade().expect("active browser"),
        )
    })
}

#[gpui_kit::test]
fn wac03_newer_modal_and_b_password_win_over_retired_a_focus(cx: &mut TestAppContext) {
    let (owner, endpoint, read, mut surface) = loading(cx);
    password_event(
        &owner,
        prompt(read.operation_id, read.session_id, read.generation),
        &mut surface,
    );
    draw(&mut surface);
    owner.update(&mut surface, |app, cx| {
        app.open_canonical(PathBuf::from(r"C:\owned\modal-b.zip"), 0, vec![], cx)
    });
    let newer = overlay(&mut surface);
    drain_frames(&mut surface);
    assert!(
        surface.update(|window, cx| newer.contains_focused(window, cx)),
        "new modal must not be closed or focused through"
    );
    let (b, bview) = active_view(&owner, &surface);
    let bread = loop {
        let wire = endpoint
            .try_receive_command_for_contract_test()
            .expect("B original read");
        if let Message::PlatformCommand(PlatformCommand::ArchiveBrowserRead(command)) = wire.message
            && command.session_id == b
        {
            break command;
        }
    };
    password_event(
        &owner,
        prompt(bread.operation_id, b, bread.generation),
        &mut surface,
    );
    draw(&mut surface);
    assert!(surface.update(|window, cx| newer.contains_focused(window, cx)));
    surface.simulate_keystrokes("escape");
    drain_frames(&mut surface);
    assert_eq!(
        password_nodes(&mut surface),
        1,
        "B matching password follows exact A retirement"
    );
    assert!(
        !surface.update(|window, cx| bview.focus_handle(cx).is_focused(window)),
        "B content cannot take modal focus"
    );
    assert!(owner.read_with(&surface, |app, _| {
        app.password_dialogs
            .values()
            .all(|dialog| dialog.session == b)
    }));
    finish(&owner, &mut surface);
}

#[gpui_kit::test]
fn wac03_queued_old_host_recovery_cannot_touch_rebuilt_host(cx: &mut TestAppContext) {
    let (owner, _, read, mut surface) = loading(cx);
    password_event(
        &owner,
        prompt(read.operation_id, read.session_id, read.generation),
        &mut surface,
    );
    draw(&mut surface);
    let input = owner.read_with(&surface, |app, _| {
        app.password_dialogs[&read.operation_id].input.clone()
    });
    surface.update(|window, cx| {
        input.update(cx, |input, cx| {
            input.set_value("synthetic-retired-host", window, cx)
        })
    });
    owner.update(&mut surface, |app, cx| {
        app.activate_browser(read.session_id, cx);
        let old = app.host.as_ref().expect("old host").id;
        app.close_host(old, cx);
        app.open_canonical(
            PathBuf::from(r"C:\owned\replacement-host.zip"),
            0,
            vec![],
            cx,
        );
    });
    cx.run_until_parked();
    assert!(input.read_with(cx, |input, _| input.value().is_empty()));
    let window = owner.read_with(cx, |app, _| {
        app.host.as_ref().expect("replacement host").window
    });
    let mut replacement = VisualTestContext::from_window(window, cx);
    drain_frames(&mut replacement);
    let (_, view) = active_view(&owner, &replacement);
    assert!(replacement.update(|window, cx| view.focus_handle(cx).is_focused(window)));
    assert_eq!(password_nodes(&mut replacement), 0);
    finish(&owner, &mut replacement);
}
