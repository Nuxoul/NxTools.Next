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
        display: None,
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

#[gpui_kit::test]
fn remember_form_saves_only_operation_defaults_and_retains_latest_browser_preferences(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (owner, _endpoint) = super::super::tests::setup(cx);
    let temp = tempfile::tempdir().expect("owned remembered defaults");
    let path = temp.path().join("config.json");
    let mut disk = nxtools_core::AppConfig::default();
    disk.archive.show_tree = false;
    disk.archive.single_click_open = true;
    disk.archive.open_after_extract = false;
    disk.archive.shell_integration.desired_revision = 9;
    disk.save_to(&path).expect("latest disk preferences");
    owner.update(cx, |app, cx| {
        app.config_path = Some(path.clone());
        let remembered = nxtools_core::archive::ArchiveSettings {
            keep_create_progress_open: true,
            exclude_patterns: vec![" space;line\nbreak ".into()],
            ..Default::default()
        };
        app.remember_accepted(remembered, ArchiveJobKind::Create, cx);
    });
    cx.run_until_parked();
    let saved = nxtools_core::AppConfig::load_from(&path)
        .expect("remembered disk")
        .archive;
    assert!(saved.keep_create_progress_open);
    assert!(!saved.show_tree && saved.single_click_open && !saved.open_after_extract);
    assert_eq!(saved.shell_integration.desired_revision, 9);
    assert_eq!(saved.exclude_patterns, vec![" space;line\nbreak "]);
    std::fs::write(&path, b"{broken").expect("failed remember fixture");
    let previous = owner.read_with(cx, |app, _| app.settings.clone());
    owner.update(cx, |app, cx| {
        app.remember_accepted(Default::default(), ArchiveJobKind::Extract, cx)
    });
    cx.run_until_parked();
    assert_eq!(owner.read_with(cx, |app, _| app.settings.clone()), previous);
    assert_eq!(std::fs::read(path).expect("failed bytes"), b"{broken");
}

#[gpui_kit::test]
fn direct_extract_batch_snapshots_completion_preferences(cx: &mut gpui_kit::TestAppContext) {
    let (owner, _endpoint) = super::super::tests::setup(cx);
    owner.update(cx, |app, cx| {
        app.settings.open_after_extract = true;
        app.settings.keep_extract_progress_open = true;
        let mut input = request(1);
        input.kind = ArchiveJobKind::Extract;
        input.destination = Some(r"C:\owned\output".into());
        input.extract = Some(ArchiveJobExtract {
            strategy: ArchiveJobStrategy::Smart,
            single_file_folder: true,
            conflict: ArchiveJobConflict::Ask,
            copy_motw: true,
            selection: None,
        });
        let ids = app
            .submit_batch(vec![input], cx)
            .expect("direct extraction batch");
        let captured = &app.jobs.records[&ids[0]].completion;
        assert!(captured.open_after && captured.keep_open);
        app.settings.open_after_extract = false;
        app.settings.keep_extract_progress_open = false;
        let captured = &app.jobs.records[&ids[0]].completion;
        assert!(
            captured.open_after && captured.keep_open,
            "active jobs keep accepted options"
        );
    });
}

#[gpui_kit::test]
fn ui_ac02_late_remember_snapshot_preserves_newer_settings(cx: &mut gpui_kit::TestAppContext) {
    let (owner, _endpoint) = super::super::tests::setup(cx);
    cx.update(|cx| cx.set_global(ArchiveGlobal(owner.clone())));
    let temp = tempfile::tempdir().expect("owned settings publication config");
    let path = temp.path().join("config.json");
    nxtools_core::AppConfig::default()
        .save_to(&path)
        .expect("seed");
    let (release, barrier) = async_channel::bounded(1);
    owner.update(cx, |app, cx| {
        app.config_path = Some(path.clone());
        app.remember_publication_barrier = Some(barrier);
        app.open_canonical(r"C:\owned\settings-order.zip".into(), 0, vec![], cx);
        let remembered = nxtools_core::archive::ArchiveSettings {
            keep_create_progress_open: true,
            ..Default::default()
        };
        app.remember_accepted(remembered, ArchiveJobKind::Create, cx);
    });
    cx.run_until_parked();
    assert!(
        nxtools_core::AppConfig::load_from(&path)
            .expect("remember saved before barrier")
            .archive
            .keep_create_progress_open
    );
    let newer = super::super::save_settings(&path, |settings| {
        settings.show_tree = false;
        settings.single_click_open = true;
    })
    .expect("later foreground save");
    let expected = newer.settings().clone();
    cx.update(|cx| super::super::saved_settings_updated(newer, cx));
    cx.run_until_parked();
    release
        .try_send(())
        .expect("release actual remember completion");
    cx.run_until_parked();
    assert_eq!(
        nxtools_core::AppConfig::load_from(&path)
            .expect("disk stays newer")
            .archive,
        expected
    );
    assert_eq!(owner.read_with(cx, |app, _| app.settings.clone()), expected);
    let state = owner.read_with(cx, |app, cx| {
        app.windows
            .values()
            .next()
            .expect("same browser")
            .view
            .read_with(cx, |view, _| view.snapshot_from_observed())
            .expect("live view")
    });
    assert!(
        !state.show_tree,
        "late completion must not restore older browser preferences"
    );
}

// Audit #4: the same typed prompt object must reach the original production Dialog.
fn object_prompt(id: Uuid, kind: &str, member: Option<&str>, target: &str) -> ArchiveJobPrompt {
    serde_json::from_value(serde_json::json!({
        "ticket":{"operation_id":id,"prompt_id":Uuid::new_v4(),"generation":1},
        "kind":kind,"error_code":null,"attempt":0,"existing_bytes":10,
        "incoming_bytes":10,"approved_limit":null,
        "display":{"incoming_member":member,"target":target}
    }))
    .expect("current decision object must cross the real typed wire")
}
fn show_object_prompt(
    owner: &Entity<ArchiveApplication>,
    endpoint: &nxtools_ipc::PlatformServerEndpoint,
    id: Uuid,
    prompt: ArchiveJobPrompt,
    cx: &mut gpui_kit::TestAppContext,
) -> gpui_kit::VisualTestContext {
    let mut event = terminal(id, true);
    event.kind = ArchiveJobEventKind::Prompt;
    event.terminal = None;
    event.prompt = Some(prompt);
    // Exercise serialization, validation, original job notification and retained window.
    let event: ArchiveJobEvent =
        serde_json::from_slice(&serde_json::to_vec(&event).expect("encode original prompt event"))
            .expect("decode original prompt event");
    event.validate().expect("bounded typed event");
    deliver(owner, endpoint, event, cx);
    cx.run_until_parked();
    let window = owner.read_with(cx, |app, _| {
        app.jobs.records[&id]
            .window
            .as_ref()
            .expect("original progress window")
            .0
    });
    let mut surface = gpui_kit::VisualTestContext::from_window(window, cx);
    surface.update(|window, cx| {
        window.activate_window();
        window.draw(cx).clear(cx);
    });
    surface
}
fn object_labels(surface: &mut gpui_kit::VisualTestContext) -> Vec<String> {
    surface.update(|window, _| {
        gpui_kit::base::test_support::snapshots(window)
            .into_iter()
            .filter_map(|node| node.label().map(str::to_owned))
            .collect()
    })
}
#[gpui_kit::test]
fn audit04_conflict_dialog_displays_ticket_member_and_full_target(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (owner, endpoint) = super::super::tests::setup(cx);
    let id = owner.update(cx, |app, cx| {
        app.submit_job(request(1), completion(), cx)
            .expect("original job admission")
    });
    cx.run_until_parked();
    let member = "资料甲/同名.txt";
    let target = r"C:\owned\输出甲\资料甲\同名.txt";
    let prompt = object_prompt(id, "conflict", Some(member), target);
    let mut surface = show_object_prompt(&owner, &endpoint, id, prompt.clone(), cx);
    let labels = object_labels(&mut surface);
    assert!(
        labels.iter().any(|label| label == member),
        "actual Dialog incoming member missing"
    );
    assert!(
        labels.iter().any(|label| label == target),
        "actual Dialog current target missing"
    );
    assert!(
        owner.read_with(cx, |app, _| app.jobs.records[&id].prompt.as_ref()
            == Some(&prompt))
    );
}
#[gpui_kit::test]
fn audit04_create_dialog_displays_current_output_target(cx: &mut gpui_kit::TestAppContext) {
    let (owner, endpoint) = super::super::tests::setup(cx);
    let id = owner.update(cx, |app, cx| {
        app.submit_job(request(1), completion(), cx)
            .expect("original job admission")
    });
    cx.run_until_parked();
    let target = r"C:\owned\输出乙\本次压缩包.zip";
    let prompt = object_prompt(id, "create_overwrite", None, target);
    let mut surface = show_object_prompt(&owner, &endpoint, id, prompt, cx);
    assert!(
        object_labels(&mut surface)
            .iter()
            .any(|label| label == target),
        "actual Dialog output target missing"
    );
}

fn object_draw(surface: &mut gpui_kit::VisualTestContext) {
    surface.run_until_parked();
    surface.update(|window, cx| {
        window.activate_window();
        window.draw(cx).clear(cx);
    });
}
fn object_node(
    surface: &mut gpui_kit::VisualTestContext,
    label: &str,
) -> gpui_kit::base::test_support::ElementSnapshot {
    object_draw(surface);
    surface
        .update(|window, _| gpui_kit::base::test_support::snapshots(window))
        .into_iter()
        .find(|node| node.label() == Some(label))
        .expect("painted production control")
}
fn object_click(surface: &mut gpui_kit::VisualTestContext, label: &str) {
    let bounds = object_node(surface, label).bounds();
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
    object_draw(surface);
}
fn object_replies(endpoint: &nxtools_ipc::PlatformServerEndpoint) -> Vec<ArchiveJobReply> {
    let mut replies = vec![];
    while let Ok(envelope) = endpoint.try_receive_command_for_contract_test() {
        if let Message::PlatformCommand(PlatformCommand::ArchiveJob(command)) = envelope.message
            && command.action == ArchiveJobAction::Reply
        {
            replies.push(command.reply.expect("original typed reply"));
        }
    }
    replies
}
#[gpui_kit::test]
fn audit04_two_jobs_consecutive_tickets_keep_objects_and_once_decision(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (owner, endpoint) = super::super::tests::setup(cx);
    let ids: Vec<_> = (1..=2)
        .map(|n| {
            owner.update(cx, |app, cx| {
                app.submit_job(request(n), completion(), cx)
                    .expect("original job admission")
            })
        })
        .collect();
    cx.run_until_parked();
    let first = object_prompt(
        ids[0],
        "conflict",
        Some("甲/同名.txt"),
        r"C:\owned\甲\同名.txt",
    );
    let other = object_prompt(
        ids[1],
        "conflict",
        Some("乙/同名.txt"),
        r"C:\owned\乙\同名.txt",
    );
    let mut a = show_object_prompt(&owner, &endpoint, ids[0], first.clone(), cx);
    let mut b = show_object_prompt(&owner, &endpoint, ids[1], other.clone(), cx);
    assert!(!object_labels(&mut a).contains(&other.display.as_ref().expect("other object").target));
    assert!(!object_labels(&mut b).contains(&first.display.as_ref().expect("first object").target));
    object_replies(&endpoint);
    object_click(&mut a, "应用到此任务后续冲突");
    object_click(&mut a, "覆盖");
    let replies = object_replies(&endpoint);
    assert_eq!(replies.len(), 1);
    assert_eq!(replies[0].ticket, first.ticket);
    assert_eq!(
        replies[0].decision,
        Some(ArchiveJobConflictDecision::Overwrite)
    );
    assert!(replies[0].apply_to_all);
    let next = object_prompt(
        ids[0],
        "conflict",
        Some("丙/同名.txt"),
        r"C:\owned\丙\同名.txt",
    );
    a = show_object_prompt(&owner, &endpoint, ids[0], next.clone(), cx);
    assert!(object_labels(&mut a).contains(&next.display.as_ref().expect("next object").target));
    assert!(!object_labels(&mut a).contains(&first.display.as_ref().expect("first object").target));
    // Negative stale delivery only; successful decisions above/below use actual controls.
    a.update(|_, cx| {
        owner.update(cx, |app, cx| {
            app.reply_job(ids[0], first.clone(), replies[0].clone(), cx)
        })
    });
    object_draw(&mut a);
    assert!(object_replies(&endpoint).is_empty());
    assert!(a.update(|window, cx| window.has_active_dialog(cx)));
    object_click(&mut a, "跳过");
    let replies = object_replies(&endpoint);
    assert_eq!(replies.len(), 1);
    assert_eq!(replies[0].ticket, next.ticket);
    assert_eq!(replies[0].decision, Some(ArchiveJobConflictDecision::Skip));
    assert!(
        !replies[0].apply_to_all,
        "next ticket must not inherit UI checkbox state"
    );
    object_click(&mut b, "自动重命名");
    let replies = object_replies(&endpoint);
    assert_eq!(replies.len(), 1);
    assert_eq!(replies[0].ticket, other.ticket);
    assert_eq!(
        replies[0].decision,
        Some(ArchiveJobConflictDecision::AutoRename)
    );
}
#[gpui_kit::test]
fn audit04_long_unicode_body_scroll_keeps_fixed_footer_and_complete_identifier(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (owner, endpoint) = super::super::tests::setup(cx);
    let id = owner.update(cx, |app, cx| {
        app.submit_job(request(1), completion(), cx)
            .expect("original job admission")
    });
    cx.run_until_parked();
    let member = format!(
        "甲目录/{}/末尾-完整标识.txt",
        "很长的Unicode😀路径/".repeat(100)
    );
    let target = format!("C:\\owned\\输出\\{}", member.replace('/', "\\"));
    let prompt = object_prompt(id, "conflict", Some(&member), &target);
    let mut surface = show_object_prompt(&owner, &endpoint, id, prompt, cx);
    for dimensions in [(560., 300.), (480., 260.)] {
        surface.simulate_resize(gpui_kit::size(px(dimensions.0), px(dimensions.1)));
        object_draw(&mut surface);
        assert!(object_labels(&mut surface).contains(&member));
        assert!(object_labels(&mut surface).contains(&target));
        let button = object_node(&mut surface, "跳过").bounds();
        let apply = object_node(&mut surface, "应用到此任务后续冲突").bounds();
        assert!(button.origin.y >= px(0.) && button.bottom() <= px(dimensions.1));
        assert!(button.right() <= px(dimensions.0) && button.size.height == px(32.));
        assert!(apply.origin.y >= px(0.) && apply.bottom() <= button.origin.y);
        let before = surface
            .debug_bounds("archive-prompt-target")
            .expect("painted target")
            .origin
            .y;
        for _ in 0..80 {
            surface.simulate_event(gpui_kit::ScrollWheelEvent {
                position: gpui_kit::point(px(240.), px(90.)),
                delta: gpui_kit::ScrollDelta::Pixels(gpui_kit::point(px(0.), px(-1000.))),
                ..Default::default()
            });
            object_draw(&mut surface);
        }
        let target_bounds = surface
            .debug_bounds("archive-prompt-target")
            .expect("painted target after scroll");
        assert!(
            target_bounds.origin.y < before,
            "real production body must scroll"
        );
        assert!(
            target_bounds.bottom() <= apply.origin.y,
            "complete target end must be recoverable above fixed actions"
        );
        assert_eq!(
            object_node(&mut surface, "跳过").bounds(),
            button,
            "footer must not scroll"
        );
    }
    object_replies(&endpoint);
    object_click(&mut surface, "跳过");
    assert_eq!(object_replies(&endpoint).len(), 1);
}

#[gpui_kit::test]
fn audit04_create_pointer_then_missing_object_cancel_preserves_original_ticket(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (owner, endpoint) = super::super::tests::setup(cx);
    let id = owner.update(cx, |app, cx| {
        app.submit_job(request(1), completion(), cx)
            .expect("original admission")
    });
    cx.run_until_parked();
    let prompt = object_prompt(id, "create_overwrite", None, r"C:\owned\pointer-target.zip");
    let mut surface = show_object_prompt(&owner, &endpoint, id, prompt.clone(), cx);
    object_replies(&endpoint);
    object_click(&mut surface, "确认覆盖");
    let replies = object_replies(&endpoint);
    assert_eq!(replies.len(), 1);
    assert_eq!(replies[0].ticket, prompt.ticket);
    assert_eq!(
        replies[0].decision,
        Some(ArchiveJobConflictDecision::Overwrite)
    );
    assert!(!surface.update(|window, cx| window.has_active_dialog(cx)));
    let mut missing = object_prompt(id, "conflict", Some("absent-member"), "absent-target");
    missing.display = None;
    let ticket = missing.ticket.clone();
    surface = show_object_prompt(&owner, &endpoint, id, missing, cx);
    let labels = object_labels(&mut surface);
    assert_eq!(
        labels
            .iter()
            .filter(|label| label.as_str() == "本次提示未提供对象标识")
            .count(),
        2
    );
    assert!(!labels.iter().any(|label| label.contains("pointer-target")
        || label.contains("absent-member")
        || label.contains("absent-target")));
    assert!(object_replies(&endpoint).is_empty());
    object_click(&mut surface, "取消任务");
    let replies = object_replies(&endpoint);
    assert_eq!(replies.len(), 1);
    assert_eq!(replies[0].ticket, ticket);
    assert!(replies[0].decision.is_none() && !replies[0].apply_to_all);
}
#[gpui_kit::test]
fn audit04_conflict_tab_space_keeps_original_button_and_deferred_reply(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (owner, endpoint) = super::super::tests::setup(cx);
    let id = owner.update(cx, |app, cx| {
        app.submit_job(request(1), completion(), cx)
            .expect("original admission")
    });
    cx.run_until_parked();
    let prompt = object_prompt(
        id,
        "conflict",
        Some("keyboard/同名.txt"),
        r"C:\owned\keyboard\同名.txt",
    );
    let mut surface = show_object_prompt(&owner, &endpoint, id, prompt.clone(), cx);
    object_replies(&endpoint);
    surface.update(|window, cx| window.focus_next(cx));
    for _ in 0..12 {
        if object_node(&mut surface, "跳过").focused() == Some(true) {
            break;
        }
        surface.simulate_keystrokes("tab");
    }
    assert_eq!(object_node(&mut surface, "跳过").focused(), Some(true));
    surface.simulate_event(gpui_kit::KeyDownEvent {
        keystroke: gpui_kit::Keystroke::parse("space").expect("space press"),
        is_held: false,
        prefer_character_input: false,
    });
    surface.simulate_event(gpui_kit::KeyUpEvent {
        keystroke: gpui_kit::Keystroke::parse("space").expect("space release"),
    });
    object_draw(&mut surface);
    let replies = object_replies(&endpoint);
    assert_eq!(replies.len(), 1);
    assert_eq!(replies[0].ticket, prompt.ticket);
    assert_eq!(replies[0].decision, Some(ArchiveJobConflictDecision::Skip));
    surface.simulate_event(gpui_kit::KeyUpEvent {
        keystroke: gpui_kit::Keystroke::parse("space").expect("duplicate release"),
    });
    object_draw(&mut surface);
    assert!(object_replies(&endpoint).is_empty());
}

#[gpui_kit::test]
fn audit05_create_keyboard_release_and_missing_object_cancel_are_once_only(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (owner, endpoint) = super::super::tests::setup(cx);
    let id = owner.update(cx, |app, cx| {
        app.submit_job(request(1), completion(), cx)
            .expect("original job admission")
    });
    cx.run_until_parked();
    let prompt = object_prompt(id, "create_overwrite", None, r"C:\owned\本次.zip");
    let mut surface = show_object_prompt(&owner, &endpoint, id, prompt.clone(), cx);
    object_replies(&endpoint);
    surface.update(|window, cx| window.focus_next(cx));
    // Traverse the original Dialog focus scope, never call its reply/confirm helper.
    for _ in 0..8 {
        if object_node(&mut surface, "确认覆盖").focused() == Some(true) {
            break;
        }
        surface.simulate_keystrokes("tab");
    }
    assert_eq!(object_node(&mut surface, "确认覆盖").focused(), Some(true));
    surface.simulate_event(gpui_kit::KeyDownEvent {
        keystroke: gpui_kit::Keystroke::parse("space").expect("space press"),
        is_held: false,
        prefer_character_input: false,
    });
    surface.simulate_event(gpui_kit::KeyUpEvent {
        keystroke: gpui_kit::Keystroke::parse("space").expect("space release"),
    });
    object_draw(&mut surface);
    let replies = object_replies(&endpoint);
    assert_eq!(replies.len(), 1);
    assert_eq!(replies[0].ticket, prompt.ticket);
    assert_eq!(
        replies[0].decision,
        Some(ArchiveJobConflictDecision::Overwrite)
    );
    surface.simulate_event(gpui_kit::KeyUpEvent {
        keystroke: gpui_kit::Keystroke::parse("space").expect("duplicate release"),
    });
    object_draw(&mut surface);
    assert!(object_replies(&endpoint).is_empty());
    let mut missing = object_prompt(
        id,
        "conflict",
        Some("not-displayed.txt"),
        r"C:\not-displayed.txt",
    );
    missing.display = None;
    let ticket = missing.ticket.clone();
    surface = show_object_prompt(&owner, &endpoint, id, missing, cx);
    let labels = object_labels(&mut surface);
    assert_eq!(
        labels
            .iter()
            .filter(|label| label.as_str() == "本次提示未提供对象标识")
            .count(),
        2
    );
    assert!(
        !labels
            .iter()
            .any(|label| label.contains("本次.zip") || label.contains("not-displayed"))
    );
    assert!(
        object_replies(&endpoint).is_empty(),
        "missing metadata never auto-approves"
    );
    surface.simulate_keystrokes("escape");
    object_draw(&mut surface);
    let replies = object_replies(&endpoint);
    assert_eq!(replies.len(), 1);
    assert_eq!(replies[0].ticket, ticket);
    assert!(replies[0].decision.is_none() && !replies[0].apply_to_all);
}

fn audit05_prompt_key(
    surface: &mut gpui_kit::VisualTestContext,
    key: &str,
    down: bool,
    held: bool,
) {
    let keystroke = gpui_kit::Keystroke::parse(key).expect("prompt key");
    if down {
        surface.simulate_event(gpui_kit::KeyDownEvent {
            keystroke,
            is_held: held,
            prefer_character_input: false,
        });
    } else {
        surface.simulate_event(gpui_kit::KeyUpEvent { keystroke });
    }
    object_draw(surface);
}
fn audit05_tab_prompt(surface: &mut gpui_kit::VisualTestContext, label: &str) {
    for _ in 0..12 {
        if object_node(surface, label).focused() == Some(true) {
            return;
        }
        surface.simulate_keystrokes("tab");
        object_draw(surface);
    }
    panic!("original Dialog control not Tab reachable: {label}");
}
#[gpui_kit::test]
fn audit05_create_enter_only_confirms_focused_button_on_release_and_cancel_keeps_owner(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (owner, endpoint) = super::super::tests::setup(cx);
    let id = owner.update(cx, |app, cx| {
        app.submit_job(request(1), completion(), cx).expect("job")
    });
    cx.run_until_parked();
    let prompt = object_prompt(id, "create_overwrite", None, r"C:\owned\enter.zip");
    let mut surface = show_object_prompt(&owner, &endpoint, id, prompt.clone(), cx);
    object_replies(&endpoint);
    assert_ne!(object_node(&mut surface, "确认覆盖").focused(), Some(true));
    audit05_prompt_key(&mut surface, "enter", true, false);
    audit05_prompt_key(&mut surface, "enter", false, false);
    assert!(
        object_replies(&endpoint).is_empty(),
        "root Enter cannot authorize overwrite"
    );
    audit05_tab_prompt(&mut surface, "确认覆盖");
    audit05_prompt_key(&mut surface, "enter", true, false);
    assert!(
        object_replies(&endpoint).is_empty(),
        "Button activation must wait for release"
    );
    audit05_prompt_key(&mut surface, "enter", true, true);
    assert!(
        object_replies(&endpoint).is_empty(),
        "held Enter cannot approve early"
    );
    audit05_prompt_key(&mut surface, "enter", false, false);
    let replies = object_replies(&endpoint);
    assert_eq!(replies.len(), 1);
    assert_eq!(replies[0].ticket, prompt.ticket);
    assert_eq!(
        replies[0].decision,
        Some(ArchiveJobConflictDecision::Overwrite)
    );
    let next = object_prompt(id, "create_overwrite", None, r"C:\owned\cancel.zip");
    surface = show_object_prompt(&owner, &endpoint, id, next.clone(), cx);
    audit05_tab_prompt(&mut surface, "取消任务");
    audit05_prompt_key(&mut surface, "enter", true, false);
    audit05_prompt_key(&mut surface, "enter", false, false);
    let replies = object_replies(&endpoint);
    assert_eq!(replies.len(), 1);
    assert_eq!(replies[0].ticket, next.ticket);
    assert!(
        replies[0].decision.is_none(),
        "focused Cancel must not approve"
    );
}

#[gpui_kit::test]
fn audit05_prompt_character_held_and_conflict_enter_respect_current_decision(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (owner, endpoint) = super::super::tests::setup(cx);
    let id = owner.update(cx, |app, cx| {
        app.submit_job(request(1), completion(), cx).expect("job")
    });
    cx.run_until_parked();
    let first = object_prompt(id, "create_overwrite", None, r"C:\owned\guarded.zip");
    let mut surface = show_object_prompt(&owner, &endpoint, id, first, cx);
    object_replies(&endpoint);
    audit05_tab_prompt(&mut surface, "确认覆盖");
    surface.simulate_event(gpui_kit::KeyDownEvent {
        keystroke: gpui_kit::Keystroke::parse("space").expect("preferred input"),
        is_held: false,
        prefer_character_input: true,
    });
    audit05_prompt_key(&mut surface, "space", false, false);
    assert!(
        object_replies(&endpoint).is_empty(),
        "character-preferred input cannot authorize overwrite"
    );
    audit05_prompt_key(&mut surface, "space", true, true);
    audit05_prompt_key(&mut surface, "space", false, false);
    assert!(
        object_replies(&endpoint).is_empty(),
        "held without an initiating down cannot approve"
    );
    audit05_prompt_key(&mut surface, "space", true, false);
    surface.simulate_event(gpui_kit::KeyDownEvent {
        keystroke: gpui_kit::Keystroke::parse("space").expect("changed input intent"),
        is_held: false,
        prefer_character_input: true,
    });
    audit05_prompt_key(&mut surface, "space", false, false);
    assert!(
        object_replies(&endpoint).is_empty(),
        "changed input intent cancels an already armed release"
    );
    for key in ["ctrl-enter", "alt-enter", "shift-enter"] {
        audit05_prompt_key(&mut surface, key, true, false);
        audit05_prompt_key(&mut surface, key, false, false);
    }
    assert!(object_replies(&endpoint).is_empty());
    surface.simulate_keystrokes("escape");
    object_draw(&mut surface);
    assert_eq!(object_replies(&endpoint).len(), 1);
    let conflict = object_prompt(
        id,
        "conflict",
        Some("current/同名.txt"),
        r"C:\owned\current\同名.txt",
    );
    surface = show_object_prompt(&owner, &endpoint, id, conflict.clone(), cx);
    audit05_tab_prompt(&mut surface, "跳过");
    audit05_prompt_key(&mut surface, "enter", true, false);
    assert!(
        object_replies(&endpoint).is_empty(),
        "focused conflict Button owns release, not root Confirm"
    );
    audit05_prompt_key(&mut surface, "enter", false, false);
    let replies = object_replies(&endpoint);
    assert_eq!(replies.len(), 1);
    assert_eq!(replies[0].ticket, conflict.ticket);
    assert_eq!(replies[0].decision, Some(ArchiveJobConflictDecision::Skip));
}
#[gpui_kit::test]
fn audit05_create_form_enter_dispatches_original_typed_admission(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (owner, endpoint) = super::super::tests::setup(cx);
    owner.update(cx, |app, cx| {
        app.open_create_form(
            vec![crate::archive::draft::DraftSource {
                path: r"C:\synthetic\keyboard.txt".into(),
                directory: false,
                bytes: Some(12),
            }],
            cx,
        )
    });
    cx.run_until_parked();
    let window = owner.read_with(cx, |app, _| {
        app.jobs
            .forms
            .values()
            .next()
            .expect("original form slot")
            .window
    });
    let mut surface = gpui_kit::VisualTestContext::from_window(window, cx);
    object_draw(&mut surface);
    for _ in 0..40 {
        let focused = surface.update(|window, _| {
            gpui_kit::base::test_support::snapshots(window)
                .into_iter()
                .any(|node| node.label() == Some("压缩包名称") && node.focused() == Some(true))
        });
        if focused {
            break;
        }
        surface.simulate_keystrokes("tab");
        object_draw(&mut surface);
    }
    assert_eq!(
        object_node(&mut surface, "压缩包名称").focused(),
        Some(true)
    );
    object_replies(&endpoint);
    surface.simulate_event(gpui_kit::KeyDownEvent {
        keystroke: gpui_kit::Keystroke::parse("enter").expect("form Enter"),
        is_held: false,
        prefer_character_input: false,
    });
    surface.run_until_parked();
    let mut starts = vec![];
    while let Ok(envelope) = endpoint.try_receive_command_for_contract_test() {
        if let Message::PlatformCommand(PlatformCommand::ArchiveJob(command)) = envelope.message
            && command.action == ArchiveJobAction::Start
        {
            starts.push(command);
        }
    }
    assert_eq!(
        starts.len(),
        1,
        "real Enter must cross original typed bridge once"
    );
    let request = starts[0].request.as_ref().expect("original request");
    assert_eq!(request.kind, ArchiveJobKind::Create);
    assert_eq!(request.paths, vec![r"C:\synthetic\keyboard.txt"]);
    assert_eq!(
        request.create.as_ref().expect("create options").name,
        "keyboard.zip"
    );
    assert!(
        request
            .create
            .as_ref()
            .expect("create options")
            .password
            .is_none()
    );
    assert!(
        owner.read_with(&surface, |app, _| app.jobs.forms.is_empty()),
        "accepted form closes through its original owner"
    );
}

#[gpui_kit::test]
fn audit05_prompt_focus_transfer_escape_and_new_ticket_cancel_old_release(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (owner, endpoint) = super::super::tests::setup(cx);
    let id = owner.update(cx, |app, cx| {
        app.submit_job(request(1), completion(), cx).expect("job")
    });
    cx.run_until_parked();
    let prompt = object_prompt(id, "create_overwrite", None, r"C:\owned\first.zip");
    let mut surface = show_object_prompt(&owner, &endpoint, id, prompt, cx);
    object_replies(&endpoint);
    audit05_tab_prompt(&mut surface, "确认覆盖");
    audit05_prompt_key(&mut surface, "space", true, false);
    surface.simulate_keystrokes("tab");
    object_draw(&mut surface);
    audit05_tab_prompt(&mut surface, "确认覆盖");
    audit05_prompt_key(&mut surface, "space", false, false);
    assert!(
        object_replies(&endpoint).is_empty(),
        "focus ABA invalidates original native click generation"
    );
    audit05_prompt_key(&mut surface, "space", true, false);
    surface.simulate_keystrokes("escape");
    object_draw(&mut surface);
    let cancelled = object_replies(&endpoint);
    assert_eq!(cancelled.len(), 1);
    assert!(cancelled[0].decision.is_none());
    let next = object_prompt(id, "create_overwrite", None, r"C:\owned\next.zip");
    surface = show_object_prompt(&owner, &endpoint, id, next.clone(), cx);
    audit05_tab_prompt(&mut surface, "确认覆盖");
    audit05_prompt_key(&mut surface, "space", false, false);
    assert!(
        object_replies(&endpoint).is_empty(),
        "old released key cannot authorize a new ticket"
    );
    assert!(object_labels(&mut surface).contains(&r"C:\owned\next.zip".to_owned()));
    audit05_prompt_key(&mut surface, "space", true, false);
    audit05_prompt_key(&mut surface, "space", false, false);
    let replies = object_replies(&endpoint);
    assert_eq!(replies.len(), 1);
    assert_eq!(replies[0].ticket, next.ticket);
    assert_eq!(
        replies[0].decision,
        Some(ArchiveJobConflictDecision::Overwrite)
    );
}
#[gpui_kit::test]
fn audit05_shared_prompt_buttons_preserve_password_risk_and_destination_reply_shapes(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (owner, endpoint) = super::super::tests::setup(cx);
    let id = owner.update(cx, |app, cx| {
        app.submit_job(request(1), completion(), cx).expect("job")
    });
    cx.run_until_parked();
    for kind in [
        ArchiveJobPromptKind::Password,
        ArchiveJobPromptKind::Risk,
        ArchiveJobPromptKind::Destination,
    ] {
        let mut prompt = object_prompt(id, "conflict", Some("unused"), "unused");
        prompt.display = None;
        prompt.kind = kind;
        prompt.approved_limit = (kind == ArchiveJobPromptKind::Risk).then_some(4096);
        let mut surface = show_object_prompt(&owner, &endpoint, id, prompt.clone(), cx);
        object_replies(&endpoint);
        if kind == ArchiveJobPromptKind::Password {
            let input = object_node(&mut surface, "当前任务密码").bounds();
            surface.simulate_click(input.center(), Default::default());
            surface.simulate_input("synthetic-private-password");
            object_draw(&mut surface);
        }
        audit05_tab_prompt(&mut surface, "继续");
        audit05_prompt_key(&mut surface, "space", true, false);
        assert!(object_replies(&endpoint).is_empty());
        audit05_prompt_key(&mut surface, "space", false, false);
        let replies = object_replies(&endpoint);
        assert_eq!(replies.len(), 1);
        let reply = &replies[0];
        assert_eq!(reply.ticket, prompt.ticket);
        assert_eq!(reply.kind, kind);
        assert_eq!(
            reply.decision,
            (kind == ArchiveJobPromptKind::Destination)
                .then_some(ArchiveJobConflictDecision::Overwrite)
        );
        assert_eq!(reply.approved_limit, prompt.approved_limit);
        assert!(!reply.apply_to_all);
        if kind == ArchiveJobPromptKind::Password {
            assert!(reply.password.is_some());
            assert!(!format!("{reply:?}").contains("synthetic-private-password"));
        } else {
            assert!(reply.password.is_none());
        }
    }
}

#[gpui_kit::test]
fn audit05_prompt_window_deactivation_cancels_armed_release(cx: &mut gpui_kit::TestAppContext) {
    let (owner, endpoint) = super::super::tests::setup(cx);
    let id = owner.update(cx, |app, cx| {
        app.submit_job(request(1), completion(), cx).expect("job")
    });
    cx.run_until_parked();
    let prompt = object_prompt(id, "create_overwrite", None, r"C:\owned\activation.zip");
    let mut surface = show_object_prompt(&owner, &endpoint, id, prompt, cx);
    object_replies(&endpoint);
    audit05_tab_prompt(&mut surface, "确认覆盖");
    audit05_prompt_key(&mut surface, "space", true, false);
    surface.deactivate_window();
    object_draw(&mut surface);
    audit05_prompt_key(&mut surface, "space", false, false);
    assert!(
        object_replies(&endpoint).is_empty(),
        "window activation change cancels old armed release"
    );
    assert!(surface.update(|window, cx| window.has_active_dialog(cx)));
    audit05_prompt_key(&mut surface, "space", true, false);
    audit05_prompt_key(&mut surface, "space", false, false);
    assert_eq!(object_replies(&endpoint).len(), 1);
}
#[gpui_kit::test]
fn audit05_output_budget_official_button_resumes_one_bound_attempt(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (owner, endpoint) = super::super::tests::setup(cx);
    let mut input = request(1);
    input.kind = ArchiveJobKind::Extract;
    input.paths = vec![r"C:\owned\budget.tar.gz".into()];
    input.destination = Some(r"C:\owned\budget-out".into());
    input.limits.max_entry_bytes = 2;
    input.limits.max_output_bytes = 2;
    input.extract = Some(ArchiveJobExtract {
        strategy: ArchiveJobStrategy::Here,
        single_file_folder: false,
        conflict: ArchiveJobConflict::Ask,
        copy_motw: false,
        selection: None,
    });
    let id = owner.update(cx, |app, cx| {
        app.submit_job(input, completion(), cx)
            .expect("logical budget job")
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
        panic!("original bounded start")
    };
    let first = wait(cx);
    assert_eq!(first.operation_id, id);
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
            native: "v1:owned-budget".into(),
            digest: [8; 32],
        },
    });
    deliver(&owner, &endpoint, shortage, cx);
    let mut observed = None;
    for _ in 0..100 {
        cx.executor()
            .advance_clock(std::time::Duration::from_millis(25));
        cx.run_until_parked();
        if let Some(prompt) = owner.read_with(cx, |app, _| app.jobs.records[&id].prompt.clone()) {
            observed = Some(prompt);
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    let prompt = observed.expect("bounded original budget prompt");
    assert_eq!(prompt.kind, ArchiveJobPromptKind::OutputBudget);
    assert_eq!(prompt.approved_limit, Some(3));
    let window = owner.read_with(cx, |app, _| {
        app.jobs.records[&id]
            .window
            .as_ref()
            .expect("budget Dialog")
            .0
    });
    let mut surface = gpui_kit::VisualTestContext::from_window(window, cx);
    object_draw(&mut surface);
    audit05_tab_prompt(&mut surface, "继续");
    audit05_prompt_key(&mut surface, "enter", true, false);
    assert!(
        owner.read_with(&surface, |app, _| app.jobs.records[&id].prompt.as_ref()
            == Some(&prompt)),
        "press alone cannot grant budget"
    );
    audit05_prompt_key(&mut surface, "enter", false, false);
    let second = wait(&mut surface);
    assert_ne!(second.operation_id, id);
    assert_eq!(second.session_id, id);
    let retry = second.request.as_ref().expect("bound retry");
    assert_eq!(retry.limits.max_entry_bytes, 3);
    assert_eq!(retry.limits.max_output_bytes, 3);
    assert_eq!(
        retry
            .readmission
            .as_ref()
            .expect("source binding")
            .expected_source
            .as_ref()
            .expect("source")
            .native,
        "v1:owned-budget"
    );
    audit05_prompt_key(&mut surface, "enter", false, false);
    while let Ok(envelope) = endpoint.try_receive_command_for_contract_test() {
        assert!(
            !matches!(envelope.message,Message::PlatformCommand(PlatformCommand::ArchiveJob(command))if command.action==ArchiveJobAction::Start),
            "duplicate release cannot readmit again"
        );
    }
}
