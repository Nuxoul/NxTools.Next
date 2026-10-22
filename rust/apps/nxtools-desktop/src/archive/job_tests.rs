//! Actual ArchiveApplication + production PlatformBridge queue, no native GUI or user data.
use super::*;
use gpui_kit::component::WindowExt as _;
use nxtools_protocol::{Message, PlatformCommand, PlatformEvent};
fn request(n: usize) -> ArchiveJobRequest {
    ArchiveJobRequest {
        kind: ArchiveJobKind::Test,
        paths: vec![format!(r"C:\owned\archive-{n}.zip")],
        destination: None,
        codepage: 0,
        limits: Default::default(),
        extract: None,
        create: None,
        readmission: None,
        modify: None,
    }
}
fn completion() -> FormCompletion {
    FormCompletion {
        open_after: false,
        keep_open: true,
        remember: None,
        additional_requests: vec![],
    }
}
fn terminal(id: Uuid, cleanup: bool) -> ArchiveJobEvent {
    ArchiveJobEvent {
        version: 1,
        operation_id: id,
        session_id: id,
        generation: 1,
        kind: ArchiveJobEventKind::Terminal,
        state: None,
        progress: None,
        prompt: None,
        terminal: Some(ArchiveJobTerminal {
            outcome: ArchiveJobOutcome::Cancelled,
            code: Some("cancelled_with_committed_output".into()),
            files_committed: 1,
            bytes_committed: 3,
            files_verified: 1,
            bytes_verified: 3,
            files_skipped: 0,
            cleanup_complete: cleanup,
            commit_state: Some("Committed".into()),
            elapsed_ms: 7,
            native_error_code: None,
            journal: vec![ArchiveJobCommit {
                ordinal: 0,
                state: "Committed".into(),
                replaced: false,
                bytes: 3,
                native_identity: None,
                published: None,
                cleanup: None,
            }],
            output_shortfall: None,
        }),
    }
}
fn deliver(
    owner: &Entity<ArchiveApplication>,
    endpoint: &nxtools_ipc::PlatformServerEndpoint,
    event: ArchiveJobEvent,
    cx: &mut gpui_kit::TestAppContext,
) {
    let notification = PlatformNotification::Event(PlatformEvent::ArchiveJob(Box::new(event)));
    endpoint
        .send_notification_for_contract_test(notification.clone())
        .expect("original IPC cleanup observation");
    owner.update(cx, |app, cx| {
        app.job_notification(&notification, cx);
    });
}
#[gpui_kit::test]
fn phase_d_production_fifo_terminal_keeps_real_commits_and_notifies_once(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (owner, endpoint) = super::super::tests::setup(cx);
    let center = cx.new(|_| crate::notifications::NotificationCenter::default());
    cx.update(|cx| crate::notifications::register(&center, cx));
    let mut ids = vec![];
    for n in 0..3 {
        ids.push(owner.update(cx, |app, cx| {
            app.submit_job(request(n), completion(), cx)
                .expect("actual admission")
        }));
    }
    let mut commands = vec![];
    while let Ok(command) = endpoint.try_receive_command_for_contract_test() {
        if let Message::PlatformCommand(PlatformCommand::ArchiveJob(command)) = command.message {
            commands.push(command);
        }
    }
    assert_eq!(commands.len(), 2, "only max2original starts");
    assert_eq!(commands[0].operation_id, ids[0]);
    assert_eq!(commands[1].operation_id, ids[1]);
    assert!(owner.read_with(cx, |app, _| app.jobs.records.contains_key(&ids[0])));
    deliver(&owner, &endpoint, terminal(ids[0], true), cx);
    deliver(&owner, &endpoint, terminal(ids[0], true), cx);
    cx.run_until_parked();
    assert_eq!(
        owner.read_with(cx, |app, _| app.jobs.records[&ids[0]]
            .terminal
            .as_ref()
            .expect("terminal")
            .files_committed),
        1
    );
    let notification_id = format!("archive-job-{}", ids[0]);
    assert_eq!(
        center.read_with(cx, |center, _| center
            .items()
            .iter()
            .filter(|item| item.id.as_ref() == notification_id)
            .count()),
        1
    );
    let mut started_third = false;
    while let Ok(command) = endpoint.try_receive_command_for_contract_test() {
        if let Message::PlatformCommand(PlatformCommand::ArchiveJob(command)) = command.message {
            started_third |=
                command.operation_id == ids[2] && command.action == ArchiveJobAction::Start;
        }
    }
    assert!(
        started_third,
        "third FIFO starts only after real cleanup terminal"
    );
}
#[gpui_kit::test]
fn phase_d_cleanup_unknown_and_stale_events_cannot_free_or_replace_job(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (owner, endpoint) = super::super::tests::setup(cx);
    let id = owner.update(cx, |app, cx| {
        app.submit_job(request(1), completion(), cx).expect("job")
    });
    let mut stale = terminal(id, true);
    stale.generation = 2;
    deliver(&owner, &endpoint, stale, cx);
    assert!(owner.read_with(cx, |app, _| app.jobs.records[&id].terminal.is_none()));
    deliver(&owner, &endpoint, terminal(id, false), cx);
    assert!(
        owner.read_with(cx, |app, _| app.jobs.queue.is_running(id)),
        "cleanup=false keeps native slot"
    );
    owner.update(cx, |app, cx| {
        app.control_job(id, ArchiveJobAction::Cancel, cx)
    });
    assert_eq!(
        owner.read_with(cx, |app, _| app.jobs.records[&id]
            .terminal
            .as_ref()
            .expect("original terminal")
            .files_committed),
        1
    );
}

#[gpui_kit::test]
fn phase_d_original_readmission_waits_cleanup_and_preserves_logical_once_terminal(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (owner, endpoint) = super::super::tests::setup(cx);
    let mut input = request(1);
    input.kind = ArchiveJobKind::Extract;
    input.paths = vec![r"C:\owned\archive.tar.gz".into()];
    input.destination = Some(r"C:\owned\out".into());
    input.limits.max_entry_bytes = 2;
    input.limits.max_output_bytes = 2;
    input.extract = Some(ArchiveJobExtract {
        strategy: ArchiveJobStrategy::Here,
        single_file_folder: false,
        conflict: ArchiveJobConflict::Ask,
        copy_motw: true,
        selection: None,
    });
    let id = owner.update(cx, |app, cx| {
        app.submit_job(input, completion(), cx)
            .expect("logical job")
    });
    let wait = |cx: &mut gpui_kit::TestAppContext| {
        for _ in 0..100 {
            cx.executor()
                .advance_clock(std::time::Duration::from_millis(25));
            cx.run_until_parked();
            if let Ok(envelope) = endpoint.try_receive_command_for_contract_test()
                && let Message::PlatformCommand(PlatformCommand::ArchiveJob(command)) =
                    envelope.message
                && command.action == ArchiveJobAction::Start
            {
                return *command;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        panic!("original attempt not dispatched")
    };
    let first = wait(cx);
    assert_eq!(first.operation_id, id);
    assert!(
        first
            .request
            .as_ref()
            .expect("request")
            .readmission
            .is_some()
    );
    let mut shortage = terminal(id, true);
    let facts = shortage.terminal.as_mut().expect("shortfall");
    facts.outcome = ArchiveJobOutcome::Failed;
    facts.code = Some("resource_limit".into());
    facts.files_committed = 0;
    facts.bytes_committed = 0;
    facts.journal.clear();
    facts.commit_state = None;
    facts.output_shortfall = Some(ArchiveJobOutputShortfall {
        current: ArchiveJobOutputBudget {
            entry_bytes: 2,
            output_bytes: 2,
        },
        required: ArchiveJobOutputBudget {
            entry_bytes: 3,
            output_bytes: 3,
        },
        source: ArchiveJobOutputSource {
            native: "v1:owned-fixture".into(),
            digest: [7; 32],
        },
    });
    deliver(&owner, &endpoint, shortage, cx);
    let prompt = {
        let mut found = None;
        for _ in 0..100 {
            cx.executor()
                .advance_clock(std::time::Duration::from_millis(25));
            cx.run_until_parked();
            if let Some(prompt) = owner.read_with(cx, |app, _| app.jobs.records[&id].prompt.clone())
            {
                found = Some(prompt);
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        found.expect("finite original budget prompt")
    };
    assert!(owner.read_with(cx, |app, _| app.jobs.records[&id].terminal.is_none()));
    owner.update(cx, |app, cx| {
        app.reply_job(
            id,
            prompt.clone(),
            ArchiveJobReply {
                ticket: prompt.ticket.clone(),
                kind: prompt.kind,
                password: None,
                decision: None,
                apply_to_all: false,
                approved_limit: Some(3),
            },
            cx,
        )
    });
    let second = wait(cx);
    assert_ne!(second.operation_id, id);
    assert_eq!(second.session_id, id);
    assert_eq!(
        second
            .request
            .as_ref()
            .expect("retry")
            .limits
            .max_output_bytes,
        3
    );
    let mut done = terminal(second.operation_id, true);
    done.session_id = id;
    done.terminal.as_mut().expect("success").outcome = ArchiveJobOutcome::Succeeded;
    done.terminal.as_mut().expect("success").code = None;
    deliver(&owner, &endpoint, done, cx);
    for _ in 0..100 {
        cx.executor()
            .advance_clock(std::time::Duration::from_millis(25));
        cx.run_until_parked();
        if owner.read_with(cx, |app, _| app.jobs.records[&id].terminal.is_some()) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert_eq!(
        owner.read_with(cx, |app, _| app.jobs.records[&id]
            .terminal
            .as_ref()
            .expect("logical terminal")
            .files_committed),
        1
    );
    assert!(!owner.read_with(cx, |app, _| app.jobs.queue.is_running(id)));
}

#[gpui_kit::test]
fn phase_d_test_after_create_failure_preserves_committed_output(cx: &mut gpui_kit::TestAppContext) {
    let (owner, endpoint) = super::super::tests::setup(cx);
    let mut input = request(1);
    input.kind = ArchiveJobKind::Create;
    input.destination = Some(r"C:\owned\out".into());
    input.create = Some(ArchiveJobCreate {
        format: ArchiveJobFormat::Zip,
        level: Some(ArchiveJobLevel::Normal),
        method: Some(ArchiveJobMethod::Deflate),
        password: None,
        solid: None,
        encrypt_header: None,
        strip_single_root: false,
        exclude_patterns: vec![],
        test_after_create: true,
        name: "created.zip".into(),
        volume_bytes: None,
    });
    let id = owner.update(cx, |app, cx| {
        app.submit_job(input, completion(), cx).expect("create")
    });
    let create_receipt = endpoint
        .try_receive_command_for_contract_test()
        .expect("create dispatched")
        .id;
    let mut created = terminal(id, true);
    created.terminal.as_mut().expect("create").outcome = ArchiveJobOutcome::Succeeded;
    created.terminal.as_mut().expect("create").code = None;
    deliver(&owner, &endpoint, created, cx);
    assert!(
        owner.read_with(cx, |app, _| app.jobs.records[&id].terminal.is_none()),
        "post-create test is part of original logical task"
    );
    let mut test = None;
    while let Ok(envelope) = endpoint.try_receive_command_for_contract_test() {
        if let Message::PlatformCommand(PlatformCommand::ArchiveJob(command)) = envelope.message
            && command.action == ArchiveJobAction::Start
        {
            test = Some(command);
            break;
        }
    }
    let test = test.expect("real post-create Test request");
    assert_ne!(test.operation_id, id);
    assert_eq!(
        test.request.as_ref().expect("request").kind,
        ArchiveJobKind::Test
    );
    owner.update(cx, |app, cx| {
        app.job_notification(
            &PlatformNotification::CommandResult {
                correlation_id: create_receipt,
                result: nxtools_protocol::PlatformCommandResult {
                    success: false,
                    error: Some("timeout".into()),
                    native_error_code: None,
                    data: None,
                },
            },
            cx,
        );
    });
    assert!(
        owner.read_with(cx, |app, _| app.jobs.records[&id].terminal.is_none()
            && app.jobs.records[&id].active_operation == test.operation_id),
        "old create receipt cannot cancel post-test"
    );
    let mut failed = terminal(test.operation_id, true);
    failed.session_id = id;
    let facts = failed.terminal.as_mut().expect("test failure");
    facts.outcome = ArchiveJobOutcome::Failed;
    facts.code = Some("corrupt_archive".into());
    facts.files_committed = 0;
    facts.bytes_committed = 0;
    facts.journal.clear();
    facts.commit_state = None;
    deliver(&owner, &endpoint, failed, cx);
    let facts = owner.read_with(cx, |app, _| {
        app.jobs.records[&id]
            .terminal
            .clone()
            .expect("one terminal")
    });
    assert_eq!(facts.outcome, ArchiveJobOutcome::Failed);
    assert_eq!(facts.files_committed, 1);
    assert_eq!(facts.bytes_committed, 3);
}

#[gpui_kit::test]
fn phase_d_terminal_clears_original_taskbar_and_background_does_not_cancel(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (owner, endpoint) = super::super::tests::setup(cx);
    let id = owner.update(cx, |app, cx| {
        app.submit_job(request(1), completion(), cx).expect("job")
    });
    cx.run_until_parked();
    while endpoint.try_receive_command_for_contract_test().is_ok() {}
    owner.update(cx, |app, cx| app.background_job(id, cx));
    cx.run_until_parked();
    assert!(
        owner.read_with(cx, |app, _| app.jobs.records[&id].terminal.is_none()
            && app.jobs.queue.is_running(id))
    );
    while let Ok(envelope) = endpoint.try_receive_command_for_contract_test() {
        assert!(
            !matches!(envelope.message,Message::PlatformCommand(PlatformCommand::ArchiveJob(command))if command.action==ArchiveJobAction::Cancel)
        );
    }
    owner.update(cx, |app, cx| app.show_job(id, cx));
    cx.run_until_parked();
    while endpoint.try_receive_command_for_contract_test().is_ok() {}
    deliver(&owner, &endpoint, terminal(id, true), cx);
    cx.run_until_parked();
    let mut cleared = false;
    while let Ok(envelope) = endpoint.try_receive_command_for_contract_test() {
        if let Message::PlatformCommand(PlatformCommand::SetTaskbarProgress(command)) =
            envelope.message
        {
            cleared |= command.state == nxtools_protocol::TaskbarProgressState::None;
        }
    }
    assert!(
        cleared,
        "actual original taskbar command cleared at terminal"
    );
}

#[gpui_kit::test]
fn phase_d_destination_official_dialog_replies_once_with_original_ticket(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (owner, endpoint) = super::super::tests::setup(cx);
    let id = owner.update(cx, |app, cx| {
        app.submit_job(request(1), completion(), cx).expect("job")
    });
    cx.run_until_parked();
    while endpoint.try_receive_command_for_contract_test().is_ok() {}
    let prompt = ArchiveJobPrompt {
        ticket: ArchiveJobTicket {
            operation_id: id,
            prompt_id: Uuid::new_v4(),
            generation: 1,
        },
        kind: ArchiveJobPromptKind::Destination,
        error_code: Some("low_space".into()),
        existing_bytes: Some(0),
        incoming_bytes: Some(7),
        approved_limit: None,
        attempt: 1,
    };
    let mut event = terminal(id, true);
    event.kind = ArchiveJobEventKind::Prompt;
    event.terminal = None;
    event.prompt = Some(prompt.clone());
    deliver(&owner, &endpoint, event, cx);
    cx.run_until_parked();
    let window = owner.read_with(cx, |app, _| {
        app.jobs.records[&id]
            .window
            .as_ref()
            .expect("progress window")
            .0
    });
    let mut surface = gpui_kit::VisualTestContext::from_window(window, cx);
    surface.update(|window, cx| {
        window.activate_window();
        window.draw(cx).clear(cx);
    });
    assert!(surface.update(|window, cx| window.has_active_dialog(cx)));
    let bounds = surface
        .debug_bounds("archive-job-prompt-ok")
        .expect("official continue Button");
    assert_eq!(bounds.size.height, px(32.));
    surface.simulate_event(gpui_kit::MouseDownEvent {
        position: bounds.center(),
        button: gpui_kit::MouseButton::Left,
        ..Default::default()
    });
    surface.simulate_event(gpui_kit::MouseUpEvent {
        position: bounds.center(),
        button: gpui_kit::MouseButton::Left,
        modifiers: Default::default(),
        click_count: 1,
    });
    surface.run_until_parked();
    let mut replies = vec![];
    while let Ok(envelope) = endpoint.try_receive_command_for_contract_test() {
        if let Message::PlatformCommand(PlatformCommand::ArchiveJob(command)) = envelope.message
            && command.action == ArchiveJobAction::Reply
        {
            replies.push(command);
        }
    }
    assert_eq!(replies.len(), 1);
    let reply = replies[0].reply.as_ref().expect("original reply");
    assert_eq!(reply.ticket, prompt.ticket);
    assert_eq!(reply.decision, Some(ArchiveJobConflictDecision::Overwrite));
    assert!(reply.approved_limit.is_none() && reply.password.is_none() && !reply.apply_to_all);
    surface.update(|_, cx| {
        owner.update(cx, |app, cx| {
            app.reply_job(id, prompt.clone(), reply.clone(), cx)
        })
    });
    assert!(!surface.update(|window, cx| window.has_active_dialog(cx)));
    while let Ok(envelope) = endpoint.try_receive_command_for_contract_test() {
        assert!(
            !matches!(envelope.message, Message::PlatformCommand(PlatformCommand::ArchiveJob(command)) if command.action == ArchiveJobAction::Reply)
        );
    }
    let mut next_prompt = prompt.clone();
    next_prompt.ticket.prompt_id = Uuid::new_v4();
    let mut next_event = terminal(id, true);
    next_event.kind = ArchiveJobEventKind::Prompt;
    next_event.terminal = None;
    next_event.prompt = Some(next_prompt);
    surface.update(|_, cx| {
        owner.update(cx, |app, cx| {
            app.job_notification(
                &PlatformNotification::Event(PlatformEvent::ArchiveJob(Box::new(next_event))),
                cx,
            )
        })
    });
    surface.run_until_parked();
    assert!(surface.update(|window, cx| window.has_active_dialog(cx)));
    surface.update(|_, cx| {
        owner.update(cx, |app, cx| {
            app.job_notification(
                &PlatformNotification::Event(PlatformEvent::ArchiveJob(Box::new(terminal(
                    id, true,
                )))),
                cx,
            )
        })
    });
    surface.run_until_parked();
    assert!(
        !surface.update(|window, cx| window.has_active_dialog(cx)),
        "terminal retires the original prompt surface"
    );
}
#[gpui_kit::test]
fn edit_uses_same_fifo_and_terminal_refreshes_only_matching_browser(
    cx: &mut gpui_kit::TestAppContext,
) {
    use nxtools_protocol::archive_browser::{ArchiveBrowserContext, ArchiveSourceBinding};
    let (owner, endpoint) = super::super::tests::setup(cx);
    let snapshot = Uuid::new_v4();
    let binding = ArchiveBrowserContext {
        snapshot_id: snapshot,
        format: "Zip".into(),
        comment: String::new(),
        source: ArchiveSourceBinding {
            native: "v1:fixture".into(),
            digest: [4; 32],
            size: 7,
            modified_unix_ns: 4,
        },
    };
    let mut edit = request(1);
    edit.kind = ArchiveJobKind::Modify;
    edit.modify = Some(ArchiveJobModify {
        browser: binding,
        action: ArchiveJobModifyAction::Delete,
        ordinals: vec![2],
        value: None,
    });
    let id = owner.update(cx, |app, cx| {
        app.submit_job(edit, completion(), cx).expect("FIFO edit")
    });
    let command = endpoint
        .try_receive_command_for_contract_test()
        .expect("original job wire");
    let Message::PlatformCommand(PlatformCommand::ArchiveJob(wire)) = command.message else {
        panic!("archive job")
    };
    assert_eq!(
        wire.request
            .as_ref()
            .expect("request")
            .modify
            .as_ref()
            .expect("edit")
            .browser
            .snapshot_id,
        snapshot
    );
    let mut failed = terminal(id, true);
    failed.terminal.as_mut().expect("facts").outcome = ArchiveJobOutcome::Failed;
    failed.terminal.as_mut().expect("facts").commit_state = Some("FailedBeforeCommit".into());
    deliver(&owner, &endpoint, failed, cx);
    assert!(owner.read_with(cx, |app, _| {
        app.jobs.records[&id]
            .terminal
            .as_ref()
            .is_some_and(|t| t.outcome == ArchiveJobOutcome::Failed)
    }));
}

#[gpui_kit::test]
fn edit_pre_candidate_failure_reenables_original_form_but_unknown_stays_blocked(
    cx: &mut gpui_kit::TestAppContext,
) {
    use crate::archive::draft::ModifyDraft;
    use gpui_kit::{VisualTestContext, px, size};
    let (owner, endpoint) = super::super::tests::setup(cx);
    let browser = nxtools_protocol::archive_browser::ArchiveBrowserContext {
        snapshot_id: Uuid::new_v4(),
        format: "Zip".into(),
        comment: String::new(),
        source: nxtools_protocol::archive_browser::ArchiveSourceBinding {
            native: "v1:fixture".into(),
            digest: [1; 32],
            size: 1,
            modified_unix_ns: 1,
        },
    };
    owner.update(cx, |app, cx| {
        app.open_modify_form(
            ModifyDraft {
                source: r"C:\owned\archive.zip".into(),
                browser: browser.clone(),
                action: ArchiveJobModifyAction::Comment,
                ordinals: vec![],
                value: Some("draft".into()),
                added: vec![],
                codepage: 0,
            },
            (Uuid::new_v4(), 1, browser.snapshot_id),
            cx,
        )
    });
    let window = owner.read_with(cx, |app, _| {
        app.jobs.forms.values().next().expect("real form").window
    });
    let mut surface = VisualTestContext::from_window(window, cx);
    surface.simulate_resize(size(px(560.), px(600.)));
    for unknown in [false, true] {
        surface.run_until_parked();
        surface.update(|window, cx| {
            window.activate_window();
            window.draw(cx).clear(cx);
        });
        let position = surface
            .debug_bounds("archive-form-submit")
            .expect("official submit")
            .center();
        surface.simulate_event(gpui_kit::MouseDownEvent {
            position,
            button: gpui_kit::MouseButton::Left,
            ..Default::default()
        });
        surface.simulate_event(gpui_kit::MouseUpEvent {
            position,
            button: gpui_kit::MouseButton::Left,
            modifiers: Default::default(),
            click_count: 1,
        });
        surface.run_until_parked();
        let id = owner.read_with(&surface, |app, _| {
            app.jobs
                .records
                .values()
                .find(|record| record.terminal.is_none())
                .expect("resubmitted original draft")
                .command
                .operation_id
        });
        let mut failed = terminal(id, true);
        let facts = failed.terminal.as_mut().expect("terminal");
        facts.outcome = ArchiveJobOutcome::Failed;
        facts.code = Some(
            if unknown {
                "commit_outcome_unknown"
            } else {
                "unsupported_operation"
            }
            .into(),
        );
        facts.files_committed = 0;
        facts.bytes_committed = 0;
        facts.journal.clear();
        facts.commit_state = unknown.then(|| "CommitOutcomeUnknown".into());
        let notification = PlatformNotification::Event(PlatformEvent::ArchiveJob(Box::new(failed)));
        endpoint
            .send_notification_for_contract_test(notification.clone())
            .expect("original IPC cleanup observation");
        surface.update(|_, cx| owner.update(cx, |app, cx| app.job_notification(&notification, cx)));
        surface.run_until_parked();
    }
    // A third click must not submit another modification after unknown publication.
    surface.update(|window, cx| {
        window.draw(cx).clear(cx);
    });
    let position = surface
        .debug_bounds("archive-form-submit")
        .expect("retained form")
        .center();
    surface.simulate_event(gpui_kit::MouseDownEvent {
        position,
        button: gpui_kit::MouseButton::Left,
        ..Default::default()
    });
    surface.simulate_event(gpui_kit::MouseUpEvent {
        position,
        button: gpui_kit::MouseButton::Left,
        modifiers: Default::default(),
        click_count: 1,
    });
    surface.run_until_parked();
    assert_eq!(
        owner.read_with(&surface, |app, _| app.jobs.records.len()),
        2,
        "unknown result cannot blindly retry"
    );
}
