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
        remember_baseline: None,
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
            journal_summary: None,
            output_shortfall: None,
            result_location: None,
            output_published: None,
            add_result: None,
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
        reclamation: None,
    });
    let id = owner.update(cx, |app, cx| {
        app.submit_job(input, completion(), cx)
            .expect("logical job")
    });
    owner.update(cx, |app, cx| {
        app.bind_batch(vec![id], None, cx);
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
                remember_batch_password: false,
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
    owner.read_with(cx, |app, _| {
        let batch = app.batch_snapshot_for(id).expect("logical batch retained");
        assert_eq!(
            batch.members.len(),
            1,
            "finite readmission never adds a batch member"
        );
        assert_eq!(
            batch
                .members
                .iter()
                .filter(|m| m.state == crate::archive::batch::MemberState::Succeeded)
                .count(),
            1,
            "logical terminal counted once"
        );
    });
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
        self_extracting: false,
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
    created.terminal.as_mut().expect("create").output_published = Some(true);
    created.terminal.as_mut().expect("create").result_location = Some(ArchiveJobResultLocation {
        target: r"C:\owned\out\created.zip".into(),
        action: nxtools_protocol::ShellOpenAction::SelectInExplorer,
    });
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
        sevenz_edit: None,
        tar_edit: None,
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
        source_prefix: None,
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
    let (owner, endpoint, browser_id, snapshot) = super::super::tests::preview_fixture(cx);
    let (browser, generation) = owner.update(cx, |app, cx| {
        let slot = app.windows.get_mut(&browser_id).expect("original browser");
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
        (browser, slot.generation)
    });
    owner.update(cx, |app, cx| {
        app.open_modify_form(
            ModifyDraft {
                source: r"C:\owned\preview.zip".into(),
                browser: browser.clone(),
                action: ArchiveJobModifyAction::NewDirectory,
                ordinals: vec![],
                value: Some("draft/".into()),
                source_prefix: None,
                added: vec![],
                codepage: 0,
            },
            (browser_id, generation, browser.snapshot_id),
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
        app.remember_accepted(remembered, ArchiveJobKind::Create, None, cx);
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
        app.remember_accepted(Default::default(), ArchiveJobKind::Extract, None, cx)
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
            reclamation: None,
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
        app.remember_accepted(remembered, ArchiveJobKind::Create, None, cx);
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
    assert!(
        object_replies(&endpoint).is_empty(),
        "Stop cannot answer the conflict ticket"
    );
    assert_eq!(
        owner.read_with(cx, |app, _| app.jobs.records[&id]
            .prompt
            .as_ref()
            .map(|prompt| prompt.ticket.clone())),
        Some(ticket)
    );
    object_click(&mut surface, "确认停止");
    assert_eq!(
        fb_controls(&endpoint)
            .iter()
            .map(|(_, action)| *action)
            .collect::<Vec<_>>(),
        vec![ArchiveJobAction::Cancel]
    );
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
    assert!(
        object_replies(&endpoint).is_empty(),
        "Escape opens Stop without answering the ticket"
    );
    assert_eq!(
        owner.read_with(cx, |app, _| app.jobs.records[&id]
            .prompt
            .as_ref()
            .map(|prompt| prompt.ticket.clone())),
        Some(ticket)
    );
    object_click(&mut surface, "确认停止");
    assert_eq!(
        fb_controls(&endpoint)
            .iter()
            .map(|(_, action)| *action)
            .collect::<Vec<_>>(),
        vec![ArchiveJobAction::Cancel]
    );
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
    assert!(
        object_replies(&endpoint).is_empty(),
        "focused Stop cannot approve overwrite"
    );
    assert_eq!(
        owner.read_with(cx, |app, _| app.jobs.records[&id].prompt.clone()),
        Some(next)
    );
    audit05_tab_prompt(&mut surface, "确认停止");
    audit05_prompt_key(&mut surface, "enter", true, false);
    audit05_prompt_key(&mut surface, "enter", false, false);
    assert_eq!(
        fb_controls(&endpoint)
            .iter()
            .map(|(_, action)| *action)
            .collect::<Vec<_>>(),
        vec![ArchiveJobAction::Cancel]
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
    assert!(object_replies(&endpoint).is_empty());
    object_click(&mut surface, "继续任务");
    assert!(
        fb_controls(&endpoint).is_empty(),
        "declining Stop preserves the current decision"
    );
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
    assert!(
        object_replies(&endpoint).is_empty(),
        "Escape cannot submit the original ticket"
    );
    object_click(&mut surface, "继续任务");
    assert!(fb_controls(&endpoint).is_empty());
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
        reclamation: None,
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

#[gpui_kit::test]
fn progress_terminal_primary_result_uses_native_commits_not_last_processed_snapshot(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (owner, endpoint) = super::super::tests::setup(cx);
    let id = owner.update(cx, |app, cx| {
        let mut input = request(90);
        input.kind = ArchiveJobKind::Extract;
        input.destination = Some(r"C:\owned\output".into());
        input.extract = Some(ArchiveJobExtract {
            strategy: ArchiveJobStrategy::Here,
            single_file_folder: false,
            conflict: ArchiveJobConflict::Ask,
            copy_motw: true,
            selection: None,
            reclamation: None,
        });
        app.submit_job(input, completion(), cx).expect("job")
    });
    let mut started = false;
    for _ in 0..100 {
        cx.executor()
            .advance_clock(std::time::Duration::from_millis(25));
        cx.run_until_parked();
        while let Ok(envelope) = endpoint.try_receive_command_for_contract_test() {
            if matches!(envelope.message, Message::PlatformCommand(PlatformCommand::ArchiveJob(command)) if command.action == ArchiveJobAction::Start)
            {
                started = true;
            }
        }
        if started {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert!(
        started,
        "original extraction coordinator admitted a native attempt"
    );
    let mut running = terminal(id, true);
    running.kind = ArchiveJobEventKind::Progress;
    running.terminal = None;
    running.progress = Some(ArchiveJobProgress {
        phase: "running".into(),
        bytes_done: 1024 * 1024,
        bytes_total: None,
        files_done: 20,
        files_total: None,
        current_member: None,
        current_bytes_done: None,
        current_bytes_total: None,
        sequence: None,
    });
    deliver(&owner, &endpoint, running, cx);
    let mut finished = terminal(id, true);
    let result = finished.terminal.as_mut().expect("native facts");
    result.files_skipped = 2;
    result.elapsed_ms = 65_000;
    deliver(&owner, &endpoint, finished, cx);
    for _ in 0..100 {
        cx.executor()
            .advance_clock(std::time::Duration::from_millis(25));
        cx.run_until_parked();
        if owner.read_with(cx, |app, _| app.jobs.records[&id].terminal.is_some()) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    owner.read_with(cx, |app, _| {
        let record = &app.jobs.records[&id];
        assert_eq!(
            record
                .command
                .request
                .as_ref()
                .expect("original request")
                .kind,
            ArchiveJobKind::Extract
        );
        assert_eq!(
            record
                .terminal
                .as_ref()
                .expect("actual terminal accepted")
                .files_skipped,
            2
        );
    });
    let window = owner.read_with(cx, |app, _| {
        app.jobs.records[&id].window.as_ref().expect("retained").0
    });
    let mut surface = gpui_kit::VisualTestContext::from_window(window, cx);
    surface.simulate_resize(gpui_kit::size(px(480.), px(260.)));
    surface.refresh().expect("terminal production render");
    let nodes = surface.update(|window, _| gpui_kit::base::test_support::snapshots(window));
    assert!(
        nodes.iter().any(|node| node.label().is_some_and(|label| {
            label.contains("实际提交 1 个文件 · 3 B") && label.contains("跳过 2")
        })),
        "actual labels: {:?}",
        nodes
            .iter()
            .filter_map(|node| node.label())
            .collect::<Vec<_>>()
    );
    assert!(
        nodes
            .iter()
            .any(|node| node.label().is_some_and(|label| label.contains("01:05")))
    );
    assert!(surface.debug_bounds("archive-progress-titlebar").is_some());
    let footer = surface
        .debug_bounds("archive-progress-footer")
        .expect("fixed footer");
    assert!(footer.bottom() <= px(260.));
}

#[test]
fn progress_decimal_units_and_unknown_totals_are_honest() {
    use crate::archive::progress::{format_bytes, progress_percent};
    for (bytes, expected) in [
        (0, "0 B"),
        (1000, "1.0 KB"),
        (1_000_000, "1.0 MB"),
        (1_000_000_000, "1.0 GB"),
    ] {
        assert_eq!(format_bytes(bytes), expected);
    }
    let mut snapshot = ArchiveJobProgress {
        phase: "running".into(),
        bytes_done: 2048,
        bytes_total: None,
        files_done: 3,
        files_total: None,
        current_member: None,
        current_bytes_done: None,
        current_bytes_total: None,
        sequence: None,
    };
    assert_eq!(progress_percent(&snapshot), None);
    snapshot.bytes_total = Some(1024);
    assert_eq!(progress_percent(&snapshot), Some(100.));
}

#[gpui_kit::test]
fn extract_remember_background_writer_preserves_concurrent_fields_and_no_runtime_identity(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (owner, _endpoint) = super::super::tests::setup(cx);
    let temp = tempfile::tempdir().expect("isolated remembered config");
    let path = temp.path().join("config.json");
    nxtools_core::AppConfig::default()
        .save_to(&path)
        .expect("seed");
    let baseline = nxtools_core::archive::ArchiveSettings::default();
    nxtools_core::AppConfig::update_section(&path, |config| {
        config.archive.copy_motw = false;
        config.archive.show_tree = false;
        config.archive.shell_integration.desired_revision = 17;
    })
    .expect("concurrent foreground save");
    let mut desired = baseline.clone();
    desired.default_destination = nxtools_core::archive::ArchiveDestination::Smart;
    desired.keep_extract_progress_open = true;
    desired.extract_custom_destination = Some(r"C:\synthetic\explicit-only".into());
    owner.update(cx, |app, cx| {
        app.config_path = Some(path.clone());
        app.remember_accepted(desired.clone(), ArchiveJobKind::Extract, Some(baseline), cx);
    });
    cx.run_until_parked();
    let saved = nxtools_core::AppConfig::load_from(&path)
        .expect("background durable save")
        .archive;
    assert!(!saved.copy_motw && !saved.show_tree);
    assert_eq!(saved.shell_integration.desired_revision, 17);
    assert_eq!(
        saved.extract_custom_destination,
        desired.extract_custom_destination
    );
    assert!(saved.keep_extract_progress_open);
    assert_eq!(owner.read_with(cx, |app, _| app.settings.clone()), saved);
    let json: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).expect("bytes")).expect("json");
    for field in [
        "password",
        "source",
        "snapshot",
        "ordinals",
        "selection",
        "selected_only",
        "authorization",
        "delete_source",
    ] {
        assert!(json["archive"].get(field).is_none(), "runtime-only {field}");
    }
}

#[gpui_kit::test]
fn progress_caption_close_backgrounds_without_sending_cancel(cx: &mut gpui_kit::TestAppContext) {
    let (owner, endpoint) = super::super::tests::setup(cx);
    let id = owner.update(cx, |app, cx| {
        app.submit_job(request(99), completion(), cx).expect("job")
    });
    cx.run_until_parked();
    while endpoint.try_receive_command_for_contract_test().is_ok() {}
    let window = owner.read_with(cx, |app, _| {
        app.jobs.records[&id].window.as_ref().expect("window").0
    });
    let mut surface = gpui_kit::VisualTestContext::from_window(window, cx);
    surface.refresh().expect("actual progress chrome");
    let position = surface
        .debug_bounds("archive-progress-close")
        .expect("official close Button")
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
    assert!(
        owner.read_with(&surface, |app, _| app.jobs.records[&id].window.is_none()
            && app.jobs.records[&id].terminal.is_none())
    );
    while let Ok(envelope) = endpoint.try_receive_command_for_contract_test() {
        assert!(
            !matches!(envelope.message, Message::PlatformCommand(PlatformCommand::ArchiveJob(command)) if command.action == ArchiveJobAction::Cancel)
        );
    }
}

#[gpui_kit::test]
fn cancelled_real_extract_form_never_writes_draft_preferences(cx: &mut gpui_kit::TestAppContext) {
    let (owner, _endpoint) = super::super::tests::setup(cx);
    let temp = tempfile::tempdir().expect("isolated Cancel config");
    let path = temp.path().join("config.json");
    nxtools_core::AppConfig::default()
        .save_to(&path)
        .expect("seed");
    let before = std::fs::read(&path).expect("initial exact bytes");
    owner.update(cx, |app, cx| {
        app.config_path = Some(path.clone());
        app.open_extract_form(r"C:\synthetic\cancel.zip".into(), None, cx);
    });
    cx.run_until_parked();
    let window = owner.read_with(cx, |app, _| {
        app.jobs.forms.values().next().expect("real form").window
    });
    let mut surface = gpui_kit::VisualTestContext::from_window(window, cx);
    surface.simulate_resize(gpui_kit::size(px(560.), px(600.)));
    object_draw(&mut surface);
    object_scroll_to(&mut surface, "更多选项");
    object_click(&mut surface, "更多选项");
    for label in ["记住本次设置", "同时记住自定义目标文件夹"] {
        for _ in 0..40 {
            let node = object_node(&mut surface, label);
            let viewport = surface
                .debug_bounds("archive-form-body")
                .expect("body scroll");
            if node.bounds().top() >= viewport.top() && node.bounds().bottom() <= viewport.bottom()
            {
                break;
            }
            let delta = if node.bounds().top() < viewport.top() {
                3.
            } else {
                -3.
            };
            surface.simulate_event(gpui_kit::ScrollWheelEvent {
                position: viewport.center(),
                delta: gpui_kit::ScrollDelta::Lines(gpui_kit::point(0., delta)),
                ..Default::default()
            });
            object_draw(&mut surface);
        }
        object_click(&mut surface, label);
    }
    let position = surface
        .debug_bounds("archive-form-cancel")
        .expect("fixed Cancel")
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
    assert_eq!(std::fs::read(&path).expect("Cancel bytes"), before);
    assert!(owner.read_with(&surface, |app, _| app.jobs.forms.is_empty()
        && app.jobs.records.is_empty()));
}

#[gpui_kit::test]
fn accepted_real_extract_submit_is_the_only_preference_disk_boundary(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (owner, _endpoint) = super::super::tests::setup(cx);
    let temp = tempfile::tempdir().expect("isolated accepted config");
    let path = temp.path().join("config.json");
    nxtools_core::AppConfig::default()
        .save_to(&path)
        .expect("seed");
    let before = std::fs::read(&path).expect("initial bytes");
    owner.update(cx, |app, cx| {
        app.config_path = Some(path.clone());
        app.open_extract_form(r"C:\synthetic\accepted.zip".into(), None, cx);
    });
    cx.run_until_parked();
    let window = owner.read_with(cx, |app, _| {
        app.jobs
            .forms
            .values()
            .next()
            .expect("production form")
            .window
    });
    let mut surface = gpui_kit::VisualTestContext::from_window(window, cx);
    surface.simulate_resize(gpui_kit::size(px(560.), px(600.)));
    object_draw(&mut surface);
    object_scroll_to(&mut surface, "更多选项");
    object_click(&mut surface, "更多选项");
    for label in ["保留进度窗口", "记住本次设置", "同时记住自定义目标文件夹"]
    {
        for _ in 0..40 {
            let target = object_node(&mut surface, label).bounds();
            let viewport = surface
                .debug_bounds("archive-form-body")
                .expect("body scroll");
            if target.top() >= viewport.top() && target.bottom() <= viewport.bottom() {
                break;
            }
            let delta = if target.top() < viewport.top() {
                3.
            } else {
                -3.
            };
            surface.simulate_event(gpui_kit::ScrollWheelEvent {
                position: viewport.center(),
                delta: gpui_kit::ScrollDelta::Lines(gpui_kit::point(0., delta)),
                ..Default::default()
            });
            object_draw(&mut surface);
        }
        object_click(&mut surface, label);
    }
    assert_eq!(
        std::fs::read(&path).expect("draft bytes"),
        before,
        "editing controls has no persistence effect"
    );
    let position = surface
        .debug_bounds("archive-form-submit")
        .expect("fixed submit")
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
    let saved = nxtools_core::AppConfig::load_from(&path)
        .expect("successful admission saved defaults")
        .archive;
    assert!(saved.keep_extract_progress_open);
    assert_eq!(
        saved.extract_custom_destination.as_deref(),
        Some(r"C:\synthetic")
    );
    assert!(owner.read_with(&surface, |app, _| app.jobs.forms.is_empty()
        && app.jobs.records.len() == 1));
}

#[gpui_kit::test]
fn progress_keep_open_pointer_persists_via_original_section_writer(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (owner, _native) = super::super::tests::setup(cx);
    let temp = tempfile::tempdir().expect("owned progress preferences");
    let path = temp.path().join("config.json");
    let mut original = nxtools_core::AppConfig::default();
    original.archive.show_tree = false;
    original.save_to(&path).expect("seed config");
    let id = owner.update(cx, |app, cx| {
        app.config_path = Some(path.clone());
        let mut policy = completion();
        policy.keep_open = false;
        app.submit_job(request(110), policy, cx)
            .expect("original job")
    });
    cx.run_until_parked();
    let window = owner.read_with(cx, |app, _| {
        app.jobs.records[&id].window.as_ref().expect("progress").0
    });
    let mut surface = gpui_kit::VisualTestContext::from_window(window, cx);
    surface.refresh().expect("production checkbox");
    let position = surface.update(|window, _| {
        gpui_kit::base::test_support::snapshots(window)
            .into_iter()
            .find(|node| node.label() == Some("完成后保留窗口"))
            .expect("actual official checkbox")
            .bounds()
            .center()
    });
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
    assert!(
        owner.read_with(&surface, |app, _| app.jobs.records[&id]
            .completion
            .keep_open),
        "pointer reached original state owner"
    );
    let saved = nxtools_core::AppConfig::load_from(&path).expect("new process/reopen disk source");
    assert!(
        saved.archive.keep_extract_progress_open,
        "explicit progress preference must survive process restart"
    );
    assert!(
        !saved.archive.show_tree,
        "unrelated latest archive setting retained"
    );
}

#[gpui_kit::test]
fn feedback_location_missing_native_fact_never_opens_requested_base(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (owner, endpoint) = super::super::tests::setup(cx);
    let mut input = request(801);
    input.kind = ArchiveJobKind::Extract;
    input.destination = Some(r"C:\owned\requested-base".into());
    input.extract = Some(ArchiveJobExtract {
        strategy: ArchiveJobStrategy::Name,
        single_file_folder: false,
        conflict: ArchiveJobConflict::Skip,
        copy_motw: false,
        selection: None,
        reclamation: None,
    });
    let mut policy = completion();
    policy.open_after = true;
    let id = owner
        .update(cx, |app, cx| app.submit_job(input, policy, cx))
        .expect("original admission");
    while endpoint.try_receive_command_for_contract_test().is_ok() {}
    let mut event = terminal(id, true);
    let facts = event.terminal.as_mut().expect("terminal facts");
    facts.outcome = ArchiveJobOutcome::Succeeded;
    facts.code = None;
    deliver(&owner, &endpoint, event, cx);
    cx.run_until_parked();
    let mut opened = Vec::new();
    while let Ok(envelope) = endpoint.try_receive_command_for_contract_test() {
        if let Message::PlatformCommand(PlatformCommand::OpenShellTarget(target)) = envelope.message
        {
            opened.push(target);
        }
    }
    assert!(
        opened.is_empty(),
        "base destination is not evidence of actual Name/Smart output: {opened:?}"
    );
}

#[gpui_kit::test]
fn feedback_location_actual_fact_once_preference_failure_cancel_and_multi_task(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (owner, endpoint) = super::super::tests::setup(cx);
    for (index, enabled, outcome, clean) in [
        (0, true, ArchiveJobOutcome::Succeeded, true),
        (1, false, ArchiveJobOutcome::Succeeded, true),
        (2, true, ArchiveJobOutcome::Failed, true),
        (3, true, ArchiveJobOutcome::Cancelled, true),
        (4, true, ArchiveJobOutcome::Succeeded, false),
    ] {
        let mut input = request(810 + index);
        input.kind = ArchiveJobKind::Extract;
        input.destination = Some(format!(r"C:\owned\base-{index}"));
        input.extract = Some(ArchiveJobExtract {
            strategy: ArchiveJobStrategy::Here,
            single_file_folder: false,
            conflict: ArchiveJobConflict::Skip,
            copy_motw: false,
            selection: None,
            reclamation: None,
        });
        let mut policy = completion();
        policy.open_after = enabled;
        let id = owner
            .update(cx, |app, cx| app.submit_job(input, policy, cx))
            .expect("original accepted job");
        while endpoint.try_receive_command_for_contract_test().is_ok() {}
        let mut event = terminal(id, clean);
        let facts = event.terminal.as_mut().expect("terminal facts");
        facts.outcome = outcome;
        facts.code = None;
        facts.result_location = Some(ArchiveJobResultLocation {
            target: format!(r"C:\owned\base-{index}\actual (2)"),
            action: nxtools_protocol::ShellOpenAction::Open,
        });
        owner.update(cx, |app, cx| {
            app.job_terminal(id, event.terminal.clone().expect("terminal facts"), cx)
        });
        owner.update(cx, |app, cx| {
            app.job_terminal(id, event.terminal.clone().expect("terminal facts"), cx)
        });
        cx.run_until_parked();
        let mut actions = Vec::new();
        while let Ok(envelope) = endpoint.try_receive_command_for_contract_test() {
            if let Message::PlatformCommand(PlatformCommand::OpenShellTarget(action)) =
                envelope.message
            {
                actions.push((envelope.id, action));
            }
        }
        assert_eq!(
            actions.len(),
            usize::from(index == 0),
            "only explicit enabled trusted success once, case {index}"
        );
        if let Some((receipt, action)) = actions.pop() {
            assert!(action.target.ends_with(r"actual (2)"));
            assert_eq!(
                action.action,
                nxtools_protocol::ShellOpenAction::OpenInExplorer
            );
            owner.update(cx, |app, cx| {
                assert!(app.job_location_result(
                    receipt,
                    &nxtools_protocol::PlatformCommandResult {
                        success: true,
                        error: None,
                        native_error_code: None,
                        data: Some("target_confirmed_foreground_not_confirmed".into())
                    },
                    cx
                ));
            });
            assert!(owner.read_with(cx, |app, _| {
                app.jobs.records[&id]
                    .location_status
                    .as_ref()
                    .expect("location feedback")
                    .contains("未确认前台")
            }));
        }
    }
}

fn fb_control_surface(
    owner: &Entity<ArchiveApplication>,
    endpoint: &nxtools_ipc::PlatformServerEndpoint,
    cx: &mut gpui_kit::TestAppContext,
) -> (Uuid, gpui_kit::VisualTestContext) {
    let id = owner.update(cx, |app, cx| {
        app.submit_job(request(900), completion(), cx)
            .expect("original job admission")
    });
    cx.run_until_parked();
    while endpoint.try_receive_command_for_contract_test().is_ok() {}
    let mut event = terminal(id, true);
    event.kind = ArchiveJobEventKind::State;
    event.terminal = None;
    event.state = Some(ArchiveJobState::Running);
    deliver(owner, endpoint, event, cx);
    cx.run_until_parked();
    let window = owner.read_with(cx, |app, _| {
        app.jobs.records[&id]
            .window
            .as_ref()
            .expect("original window")
            .0
    });
    let mut surface = gpui_kit::VisualTestContext::from_window(window, cx);
    object_draw(&mut surface);
    while endpoint.try_receive_command_for_contract_test().is_ok() {}
    (id, surface)
}

fn fb_controls(endpoint: &nxtools_ipc::PlatformServerEndpoint) -> Vec<(Uuid, ArchiveJobAction)> {
    let mut actions = vec![];
    while let Ok(envelope) = endpoint.try_receive_command_for_contract_test() {
        if let Message::PlatformCommand(PlatformCommand::ArchiveJob(command)) = envelope.message {
            actions.push((envelope.id, command.action));
        }
    }
    actions
}

fn fb_control_receipt(
    owner: &Entity<ArchiveApplication>,
    endpoint: &nxtools_ipc::PlatformServerEndpoint,
    receipt: Uuid,
    success: bool,
    cx: &mut gpui_kit::TestAppContext,
) {
    let notification = PlatformNotification::CommandResult {
        correlation_id: receipt,
        result: nxtools_protocol::PlatformCommandResult {
            success,
            error: (!success).then(|| "pause_refused".into()),
            native_error_code: (!success).then_some(5),
            data: None,
        },
    };
    endpoint
        .send_notification_for_contract_test(notification.clone())
        .expect("real IPC receipt");
    owner.update(cx, |app, cx| {
        app.job_notification(&notification, cx);
    });
    cx.run_until_parked();
}

#[gpui_kit::test]
fn fb_ac02_actual_queue_full_recovers_pause_and_keeps_cancel_reserved(
    cx: &mut gpui_kit::TestAppContext,
) {
    cx.update(gpui_kit::init);
    cx.executor().allow_parking();
    let (bridge, endpoint) = nxtools_ipc::platform_bridge(1);
    endpoint.set_connected_for_contract_test(true);
    let bridge = Rc::new(bridge);
    let client = Arc::new(
        ArchiveReadClient::new(bridge.archive_client_bridge()).expect("original read client"),
    );
    let owner = cx.new(|cx| ArchiveApplication::new(bridge, Some(client), cx));
    let (id, mut surface) = fb_control_surface(&owner, &endpoint, cx);
    owner.read_with(cx, |app, _| {
        for _ in 0..64 {
            if app
                .platform
                .send_command(PlatformCommand::PlaySystemNotificationSound)
                .is_err()
            {
                break;
            }
        }
    });
    object_click(&mut surface, "暂停");
    assert_ne!(
        object_node(&mut surface, "暂停").disabled(),
        Some(true),
        "unsent pause must unlock original button"
    );
    assert!(
        object_labels(&mut surface)
            .iter()
            .any(|label| label.contains("重试")),
        "queue rejection needs visible retryable feedback"
    );
    assert!(
        !fb_controls(&endpoint)
            .iter()
            .any(|(_, action)| *action == ArchiveJobAction::Pause)
    );
    object_click(&mut surface, "暂停");
    object_click(&mut surface, "暂停");
    assert_eq!(
        fb_controls(&endpoint)
            .iter()
            .filter(|(_, action)| *action == ArchiveJobAction::Pause)
            .count(),
        1
    );
    object_click(&mut surface, "停止");
    object_click(&mut surface, "确认停止");
    assert_eq!(
        fb_controls(&endpoint)
            .iter()
            .filter(|(_, action)| *action == ArchiveJobAction::Cancel)
            .count(),
        1
    );
    assert!(
        owner.read_with(cx, |app, _| app.jobs.queue.is_running(id)),
        "UI recovery never releases native execution slot"
    );
}

#[gpui_kit::test]
fn fb_ac02_native_refusal_and_receipt_timeout_restore_original_controls(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (owner, endpoint) = super::super::tests::setup(cx);
    let (id, mut surface) = fb_control_surface(&owner, &endpoint, cx);
    object_click(&mut surface, "暂停");
    let receipt = fb_controls(&endpoint)
        .into_iter()
        .find(|(_, action)| *action == ArchiveJobAction::Pause)
        .expect("real pause command")
        .0;
    fb_control_receipt(&owner, &endpoint, receipt, false, cx);
    assert_ne!(
        object_node(&mut surface, "暂停").disabled(),
        Some(true),
        "native refusal must clear pending presentation"
    );
    object_click(&mut surface, "暂停");
    assert_eq!(fb_controls(&endpoint).len(), 1);
    surface.run_until_parked();
    surface
        .executor()
        .advance_clock(std::time::Duration::from_secs(13));
    object_draw(&mut surface);
    assert_ne!(
        object_node(&mut surface, "暂停").disabled(),
        Some(true),
        "missing receipt must expire locally"
    );
    assert!(owner.read_with(cx, |app, _| app.jobs.queue.is_running(id)));
    assert!(
        fb_controls(&endpoint).is_empty(),
        "timeout must never resend or create a replacement worker"
    );
}

#[gpui_kit::test]
fn fb_ac02_accepted_is_not_paused_state_timeout_and_old_receipt_are_isolated(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (owner, endpoint) = super::super::tests::setup(cx);
    let (id, mut surface) = fb_control_surface(&owner, &endpoint, cx);
    object_click(&mut surface, "暂停");
    let old = fb_controls(&endpoint)
        .into_iter()
        .find(|(_, action)| *action == ArchiveJobAction::Pause)
        .expect("pause")
        .0;
    fb_control_receipt(&owner, &endpoint, old, true, cx);
    assert!(
        !owner.read_with(cx, |app, _| app.jobs.records[&id].paused),
        "ACK is not Paused"
    );
    assert_eq!(object_node(&mut surface, "暂停").disabled(), Some(true));
    surface.run_until_parked();
    surface
        .executor()
        .advance_clock(std::time::Duration::from_secs(11));
    object_draw(&mut surface);
    assert_ne!(object_node(&mut surface, "暂停").disabled(), Some(true));
    object_click(&mut surface, "暂停");
    let new = fb_controls(&endpoint)
        .into_iter()
        .find(|(_, action)| *action == ArchiveJobAction::Pause)
        .expect("explicit retry")
        .0;
    assert_ne!(old, new);
    fb_control_receipt(&owner, &endpoint, old, false, cx);
    assert_eq!(
        object_node(&mut surface, "暂停").disabled(),
        Some(true),
        "old receipt cannot clear new request"
    );
    let mut state = terminal(id, true);
    state.kind = ArchiveJobEventKind::State;
    state.terminal = None;
    state.state = Some(ArchiveJobState::Paused);
    deliver(&owner, &endpoint, state, cx);
    assert_ne!(object_node(&mut surface, "继续").disabled(), Some(true));
    object_click(&mut surface, "继续");
    let resume = fb_controls(&endpoint)
        .into_iter()
        .find(|(_, action)| *action == ArchiveJobAction::Resume)
        .expect("resume")
        .0;
    fb_control_receipt(&owner, &endpoint, resume, false, cx);
    assert_ne!(
        object_node(&mut surface, "继续").disabled(),
        Some(true),
        "resume refusal is retryable too"
    );
    assert!(owner.read_with(cx, |app, _| app.jobs.records[&id].paused));
}

#[gpui_kit::test]
fn fb_ac02_old_attempt_disconnect_terminal_and_cancel_do_not_unlock_new_control(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (owner, endpoint) = super::super::tests::setup(cx);
    let (id, mut surface) = fb_control_surface(&owner, &endpoint, cx);
    object_click(&mut surface, "暂停");
    let old = fb_controls(&endpoint)
        .into_iter()
        .find(|(_, action)| *action == ArchiveJobAction::Pause)
        .expect("pause")
        .0;
    let new_attempt = Uuid::new_v4();
    owner.update(cx, |app, cx| {
        let record = app.jobs.records.get_mut(&id).expect("job");
        record.active_operation = new_attempt;
        record.command.generation += 1;
        record.controls.clear();
        app.refresh_job(id, cx);
    });
    fb_control_receipt(&owner, &endpoint, old, false, cx);
    assert_ne!(object_node(&mut surface, "暂停").disabled(), Some(true));
    // Restore the original native reservation only in this fixture, then race Cancel against Pause.
    owner.update(cx, |app, cx| {
        let record = app.jobs.records.get_mut(&id).expect("job");
        record.active_operation = id;
        record.command.generation = 1;
        app.refresh_job(id, cx);
    });
    object_click(&mut surface, "暂停");
    object_click(&mut surface, "停止");
    object_click(&mut surface, "确认停止");
    let sent = fb_controls(&endpoint);
    assert_eq!(
        sent.iter()
            .filter(|(_, action)| *action == ArchiveJobAction::Pause)
            .count(),
        1
    );
    assert_eq!(
        sent.iter()
            .filter(|(_, action)| *action == ArchiveJobAction::Cancel)
            .count(),
        1
    );
    owner.update(cx, |app, cx| {
        app.job_notification(
            &PlatformNotification::Disconnected {
                generation: app.platform.connection_generation().expect("connection"),
                pending_command_ids: vec![],
            },
            cx,
        );
    });
    assert!(owner.read_with(cx, |app, _| app.jobs.records[&id].controls.is_empty()));
    assert!(owner.read_with(cx, |app, _| app.jobs.queue.is_running(id)));
    deliver(&owner, &endpoint, terminal(id, false), cx);
    assert!(
        owner.read_with(cx, |app, _| app.jobs.records[&id].controls.is_empty()
            && app.jobs.queue.is_running(id))
    );
}

#[gpui_kit::test]
fn fb_ac03_zero_file_directory_terminal_uses_actual_location_once_and_manual_entry(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (owner, endpoint) = super::super::tests::setup(cx);
    for (index, enabled, outcome, cleanup) in [
        (0, true, ArchiveJobOutcome::Succeeded, true),
        (1, false, ArchiveJobOutcome::Succeeded, true),
        (2, true, ArchiveJobOutcome::Failed, true),
        (3, true, ArchiveJobOutcome::Cancelled, true),
        (4, true, ArchiveJobOutcome::Succeeded, false),
    ] {
        let mut input = request(950 + index);
        input.kind = ArchiveJobKind::Extract;
        input.destination = Some(r"C:\owned\base".into());
        input.extract = Some(ArchiveJobExtract {
            strategy: ArchiveJobStrategy::Name,
            single_file_folder: false,
            conflict: ArchiveJobConflict::Skip,
            copy_motw: false,
            selection: None,
            reclamation: None,
        });
        let mut policy = completion();
        policy.open_after = enabled;
        let id = owner.update(cx, |app, cx| {
            app.jobs.defer_pump = true;
            app.submit_job(input, policy, cx).expect("original queue")
        });
        cx.run_until_parked();
        while endpoint.try_receive_command_for_contract_test().is_ok() {}
        let mut event = terminal(id, cleanup);
        let facts = event.terminal.as_mut().expect("native terminal");
        facts.outcome = outcome;
        facts.files_committed = 0;
        facts.bytes_committed = 0;
        facts.journal.clear();
        facts.result_location = Some(ArchiveJobResultLocation {
            target: r"C:\owned\base\actual (2)".into(),
            action: nxtools_protocol::ShellOpenAction::Open,
        });
        deliver(&owner, &endpoint, event.clone(), cx);
        deliver(&owner, &endpoint, event, cx);
        cx.run_until_parked();
        let mut opened = vec![];
        while let Ok(envelope) = endpoint.try_receive_command_for_contract_test() {
            if let Message::PlatformCommand(PlatformCommand::OpenShellTarget(action)) =
                envelope.message
            {
                opened.push(action);
            }
        }
        assert_eq!(
            opened.len(),
            usize::from(index == 0),
            "directory output is not a file commit, and failure policy remains unchanged"
        );
        if let Some(action) = opened.first() {
            assert_eq!(action.target, r"C:\owned\base\actual (2)");
            assert_eq!(
                action.action,
                nxtools_protocol::ShellOpenAction::OpenInExplorer
            );
        }
        let window = owner.read_with(cx, |app, _| {
            app.jobs.records[&id]
                .window
                .as_ref()
                .expect("retained progress")
                .0
        });
        let mut surface = gpui_kit::VisualTestContext::from_window(window, cx);
        assert_eq!(
            object_labels(&mut surface)
                .iter()
                .any(|label| label == "定位结果"),
            outcome == ArchiveJobOutcome::Succeeded && cleanup,
            "manual location uses actual output fact, not committed file count"
        );
    }
}

#[gpui_kit::test]
fn fb_ac03_real_worker_terminal_transcript_reaches_original_desktop_location_gate(
    cx: &mut gpui_kit::TestAppContext,
) {
    let cases: Vec<(String, ArchiveJobTerminal)> = if let Ok(directory) =
        std::env::var("NXTOOLS_ARCHIVE_DIRECTORY_TERMINALS")
    {
        let mut files = std::fs::read_dir(directory)
            .expect("explicit native fixture transcript")
            .map(|entry| entry.expect("entry").path())
            .filter(|path| {
                path.extension()
                    .is_some_and(|extension| extension == "json")
            })
            .collect::<Vec<_>>();
        files.sort();
        files
            .into_iter()
            .map(|path| {
                (
                    path.file_stem()
                        .expect("case name")
                        .to_string_lossy()
                        .into_owned(),
                    serde_json::from_slice(&std::fs::read(path).expect("actual C# terminal bytes"))
                        .expect("Rust consumes exact C# DTO"),
                )
            })
            .collect()
    } else {
        #[derive(serde::Deserialize)]
        struct NativeCase {
            name: String,
            terminal: ArchiveJobTerminal,
        }
        serde_json::from_str::<Vec<NativeCase>>(include_str!("fixtures/directory-terminals.json"))
            .expect("checked-in actual worker transcript")
            .into_iter()
            .map(|case| (case.name, case.terminal))
            .collect()
    };
    assert_eq!(
        cases.len(),
        22,
        "13 actual shapes, six zero-output, three guarded worker terminals"
    );
    let mut located = 0;
    for (index, (name, native)) in cases.into_iter().enumerate() {
        let (owner, endpoint) = super::super::tests::setup(cx);
        assert_eq!(native.files_committed, 0);
        let expected = native.outcome == ArchiveJobOutcome::Succeeded
            && native.cleanup_complete
            && native.result_location.is_some();
        let mut input = request(980 + index);
        input.kind = ArchiveJobKind::Extract;
        input.destination = Some(r"C:\owned\never-guess-requested-base".into());
        input.extract = Some(ArchiveJobExtract {
            strategy: ArchiveJobStrategy::Here,
            single_file_folder: false,
            conflict: ArchiveJobConflict::Skip,
            copy_motw: false,
            selection: None,
            reclamation: None,
        });
        let mut policy = completion();
        policy.open_after = true;
        let id = owner.update(cx, |app, cx| {
            app.jobs.defer_pump = true;
            app.submit_job(input, policy, cx)
                .expect("original logical job")
        });
        cx.run_until_parked();
        while endpoint.try_receive_command_for_contract_test().is_ok() {}
        let mut event = terminal(id, native.cleanup_complete);
        event.terminal = Some(native.clone());
        deliver(&owner, &endpoint, event.clone(), cx);
        deliver(&owner, &endpoint, event, cx);
        cx.run_until_parked();
        let mut targets = vec![];
        while let Ok(envelope) = endpoint.try_receive_command_for_contract_test() {
            if let Message::PlatformCommand(PlatformCommand::OpenShellTarget(action)) =
                envelope.message
            {
                targets.push(action.target);
            }
        }
        assert_eq!(
            targets.len(),
            usize::from(expected),
            "actual native case {}",
            name
        );
        if expected {
            assert_eq!(
                targets[0],
                native.result_location.expect("actual native result").target
            );
            located += 1;
        }
        owner.update(cx, |app, cx| app.background_job(id, cx));
        cx.run_until_parked();
    }
    assert_eq!(
        located, 13,
        "only real successful directory outputs auto locate, once each"
    );
}

#[gpui_kit::test]
fn sevenz_edit_replacement_uses_original_job_queue_and_exact_modify_intent(
    cx: &mut gpui_kit::TestAppContext,
) {
    use nxtools_protocol::archive_browser::{
        ArchiveBrowserContext, ArchiveSourceBinding, SevenzEditEligibility,
    };
    let (owner, endpoint) = super::super::tests::setup(cx);
    let browser = ArchiveBrowserContext {
        snapshot_id: Uuid::new_v4(),
        format: "SevenZ".into(),
        comment: String::new(),
        source: ArchiveSourceBinding {
            native: "v1:fixture".into(),
            digest: [7; 32],
            size: 7,
            modified_unix_ns: 1,
        },
        sevenz_edit: Some(SevenzEditEligibility { refusal: None }),
        tar_edit: None,
    };
    let draft = crate::archive::draft::ModifyDraft {
        source: r"C:\owned\archive.7z".into(),
        browser,
        action: ArchiveJobModifyAction::ReplaceContent,
        ordinals: vec![71],
        value: None,
        source_prefix: None,
        added: vec![r"C:\owned\replacement.txt".into()],
        codepage: 0,
    };
    let request = draft.request().expect("original draft exact replacement");
    owner.update(cx, |app, cx| {
        app.submit_job(request.clone(), completion(), cx)
            .expect("original FIFO");
    });
    let command = endpoint
        .try_receive_command_for_contract_test()
        .expect("original IPC");
    let Message::PlatformCommand(PlatformCommand::ArchiveJob(command)) = command.message else {
        panic!("original job wire")
    };
    assert_eq!(command.action, ArchiveJobAction::Start);
    assert_eq!(command.request, Some(request));
    while let Ok(extra) = endpoint.try_receive_command_for_contract_test() {
        assert!(
            !matches!(
                extra.message,
                Message::PlatformCommand(PlatformCommand::ArchiveJob(_))
            ),
            "one editing enqueue; original taskbar progress is independent"
        );
    }
}

#[gpui_kit::test]
fn step4_modify_conflict_dialog_identifies_internal_objects_and_scoped_decision(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (owner, endpoint) = super::super::tests::setup(cx);
    let edit: ArchiveJobRequest = serde_json::from_value(serde_json::json!({"kind":"modify","paths":["C:\\owned\\archive.tar","C:\\owned\\incoming.txt"],"destination":null,"extract":null,"create":null,"modify":{"browser":{"snapshot_id":Uuid::new_v4(),"format":"Tar","comment":"","source":{"native":"v1:fixture","digest":vec![1;32],"size":10,"modified_unix_ns":1},"tar_edit":{"refusal":null}},"action":"add","ordinals":[],"value":"目录/"}})).expect("original Modify shape");
    edit.validate().expect("bounded source intent");
    let id = owner.update(cx, |app, cx| {
        app.submit_job(edit, completion(), cx)
            .expect("original edit job")
    });
    cx.run_until_parked();
    let mut prompt = object_prompt(id, "conflict", Some("目录/传入.txt"), "目录/已有.txt");
    prompt.existing_bytes = None;
    prompt.incoming_bytes = Some(0);
    let mut surface = show_object_prompt(&owner, &endpoint, id, prompt.clone(), cx);
    let labels = object_labels(&mut surface);
    assert!(
        labels.iter().any(|label| label == "压缩包内已有成员"),
        "archive-relative object must never be described as a disk target"
    );
    assert!(labels.iter().any(|label| label == "目录/已有.txt"));
    assert!(
        labels
            .iter()
            .any(|label| label.contains("未知") && label.contains("0 B")),
        "unknown is not fabricated zero"
    );
    object_replies(&endpoint);
    object_click(&mut surface, "应用到此任务后续冲突");
    object_click(&mut surface, "跳过");
    let replies = object_replies(&endpoint);
    assert_eq!(replies.len(), 1);
    assert_eq!(replies[0].ticket, prompt.ticket);
    assert!(replies[0].apply_to_all);
    assert_eq!(replies[0].decision, Some(ArchiveJobConflictDecision::Skip));
}

#[gpui_kit::test]
fn step4_edit_warning_terminal_closes_original_form_without_retry(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (owner, _endpoint) = super::super::tests::setup(cx);
    for published in [false, true] {
        let request: ArchiveJobRequest = serde_json::from_value(serde_json::json!({"kind":"modify","paths":["C:\\owned\\archive.tar"],"destination":null,"extract":null,"create":null,"modify":{"browser":{"snapshot_id":Uuid::new_v4(),"format":"Tar","comment":"","source":{"native":"v1:fixture","digest":vec![1;32],"size":10,"modified_unix_ns":1},"tar_edit":{"refusal":null}},"action":"delete","ordinals":[0],"value":null}})).expect("Modify");
        let intent = request.modify.clone().expect("intent");
        let (id, form) = owner.update(cx, |app, cx| {
            app.open_modify_form(
                ModifyDraft {
                    source: request.paths[0].clone().into(),
                    browser: intent.browser.clone(),
                    action: intent.action,
                    ordinals: intent.ordinals.clone(),
                    value: None,
                    source_prefix: None,
                    added: vec![],
                    codepage: 0,
                },
                (Uuid::new_v4(), 1, intent.browser.snapshot_id),
                cx,
            );
            let form = *app.jobs.forms.keys().next().expect("original form");
            let id = app.submit_job(request, completion(), cx).expect("job");
            app.jobs.records.get_mut(&id).expect("record").edit_form = Some(form);
            (id, form)
        });
        let mut result = local_terminal("skipped", true);
        result.outcome = ArchiveJobOutcome::SucceededWithWarnings;
        result.files_skipped = 1;
        if published {
            result.files_committed = 1;
            result.commit_state = Some("Committed".into());
        }
        owner.update(cx, |app, cx| app.job_terminal(id, result, cx));
        cx.run_until_parked();
        assert!(
            !owner.read_with(cx, |app, _| app.jobs.forms.contains_key(&form)),
            "successful warning/no-op must not leave an editable retry draft"
        );
    }
}

#[gpui_kit::test]
fn hz3_original_batch_exposes_rendered_summary_without_replacing_two_running_queue(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (owner, endpoint) = super::super::tests::setup(cx);
    let ids = owner.update(cx, |app, cx| {
        app.submit_batch(vec![request(501), request(502), request(503)], cx)
            .expect("original batch")
    });
    cx.run_until_parked();
    let starts=std::iter::from_fn(||endpoint.try_receive_command_for_contract_test().ok()).filter(|wire|matches!(wire.message,Message::PlatformCommand(PlatformCommand::ArchiveJob(ref command)) if command.action==ArchiveJobAction::Start)).count();
    assert_eq!(starts, 2, "original max2 IPC starts");
    let window = owner.read_with(cx, |app, _| {
        app.jobs.records[&ids[0]]
            .window
            .as_ref()
            .expect("original detail")
            .0
    });
    let mut surface = gpui_kit::VisualTestContext::from_window(window, cx);
    surface.update(|window, cx| {
        window.draw(cx).clear(cx);
        let nodes = gpui_kit::base::test_support::snapshots(window);
        assert!(
            nodes.iter().any(|node| node.label() == Some("批次汇总")),
            "original rendered job needs batch entry"
        );
    });
}

#[gpui_kit::test]
fn hz3_retry_uses_original_native_recheck_once_before_new_attempt(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (owner, endpoint) = super::super::tests::setup(cx);
    let ids = owner.update(cx, |app, cx| {
        app.submit_batch(vec![request(601), request(602)], cx)
            .expect("batch")
    });
    while endpoint.try_receive_command_for_contract_test().is_ok() {}
    owner.update(cx,|app,_|{let record=app.jobs.records.get_mut(&ids[0]).expect("record");record.retry_baseline=Some(serde_json::from_value(serde_json::json!({"version":2,"operation_id":Uuid::new_v4(),"session_id":ids[0],"generation":1,"available_bytes":0,"allocation_bytes":4096,"target_identity":"v1:target","volume_identity":"v1:volume","destination_missing":false,"source_matches":true,"source_set":{"identity":"v1:source","primary":record.command.request.as_ref().expect("request").paths[0],"members":[record.command.request.as_ref().expect("request").paths[0]]}})).expect("actual native baseline fixture"));});
    let mut failed = terminal(ids[0], true);
    let result = failed.terminal.as_mut().expect("terminal");
    *result = super::local_terminal("io_failure", true);
    result.commit_state = Some("FailedBeforeCommit".into());
    result.output_published = Some(false); // Explicit native zero-output fixture, not missing legacy facts.
    deliver(&owner, &endpoint, failed, cx);
    owner.update(cx, |app, cx| {
        app.retry_job(ids[0], cx);
        app.retry_job(ids[0], cx);
    });
    let observations = std::iter::from_fn(|| endpoint.try_receive_command_for_contract_test().ok())
        .filter_map(|wire| {
            if let Message::PlatformCommand(PlatformCommand::ArchiveSpace(command)) = wire.message {
                Some(command)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    assert_eq!(
        observations.len(),
        1,
        "eligible explicit retry revalidates once"
    );
    let command = &observations[0];
    let mut observed = owner.read_with(cx, |app, _| {
        app.jobs.records[&ids[0]]
            .retry_baseline
            .clone()
            .expect("baseline")
    });
    observed.operation_id = command.operation_id;
    observed.session_id = command.session_id;
    observed.generation = command.generation;
    let reply = nxtools_protocol::PlatformCommandResult {
        success: true,
        error: None,
        native_error_code: None,
        data: Some(serde_json::to_string(&observed).expect("wire")),
    };
    owner.update(cx, |app, cx| {
        assert!(app.job_observation_result(command.operation_id, &reply, cx));
        assert!(!app.job_observation_result(command.operation_id, &reply, cx));
    });
    let starts = std::iter::from_fn(|| endpoint.try_receive_command_for_contract_test().ok())
        .filter_map(|wire| {
            if let Message::PlatformCommand(PlatformCommand::ArchiveJob(command)) = wire.message {
                Some(command)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    assert_eq!(
        starts
            .iter()
            .map(|command| command.action)
            .collect::<Vec<_>>(),
        [ArchiveJobAction::RetireBatch, ArchiveJobAction::Start],
        "retire old batch password before the sole new attempt"
    );
    let starts = starts
        .into_iter()
        .filter(|command| command.action == ArchiveJobAction::Start)
        .collect::<Vec<_>>();
    assert_eq!(
        starts.len(),
        1,
        "retry state {} terminal {:?}",
        owner.read_with(cx, |app, _| app.jobs.records[&ids[0]].state.clone()),
        owner.read_with(cx, |app, _| app.jobs.records[&ids[0]]
            .terminal
            .as_ref()
            .map(|r| r.code.clone()))
    );
    assert_ne!(starts[0].operation_id, ids[0]);
    assert_eq!(starts[0].session_id, ids[0]);
    assert_eq!(starts[0].generation, 2);
    deliver(&owner, &endpoint, terminal(ids[0], true), cx);
    owner.read_with(cx, |app, _| {
        assert!(
            app.jobs.records[&ids[0]].terminal.is_none(),
            "old attempt terminal refused"
        );
        let batch = app.batch_snapshot_for(ids[0]).expect("batch");
        assert_eq!(batch.members.len(), 2);
        assert_eq!(
            batch
                .members
                .iter()
                .filter(|m| m.state == crate::archive::batch::MemberState::Succeeded)
                .count(),
            0
        );
    });
}

#[gpui_kit::test]
fn hz3_projection_preserves_prompt_pause_queued_cancel_warning_partial_and_cleanup_slot(
    cx: &mut gpui_kit::TestAppContext,
) {
    use crate::archive::batch::MemberState as S;
    let (owner, endpoint) = super::super::tests::setup(cx);
    let ids = owner.update(cx, |app, cx| {
        app.submit_batch(vec![request(701), request(702), request(703)], cx)
            .expect("batch")
    });
    owner.update(cx, |app, cx| {
        let first = app.jobs.records.get_mut(&ids[0]).expect("first");
        first.ever_started = true;
        first.prompt = Some(object_prompt(ids[0], "conflict", Some("成员"), "目标"));
        let second = app.jobs.records.get_mut(&ids[1]).expect("second");
        second.ever_started = true;
        second.paused = true;
        let snapshot = app.batch_snapshot_for(ids[0]).expect("batch");
        assert_ne!(snapshot.id, ids[0]);
        assert_eq!(
            snapshot.members.iter().map(|m| m.state).collect::<Vec<_>>(),
            vec![S::Waiting, S::Paused, S::Queued]
        );
        app.control_job(ids[2], ArchiveJobAction::Cancel, cx);
        let snapshot = app.batch_snapshot_for(ids[0]).expect("batch");
        assert_eq!(snapshot.members[2].state, S::Cancelled);
        assert!(snapshot.members[2].never_started);
        let mut warning = super::local_terminal("members_skipped", true);
        warning.outcome = ArchiveJobOutcome::SucceededWithWarnings;
        warning.files_skipped = 3;
        app.job_terminal(ids[0], warning.clone(), cx);
        app.job_terminal(ids[0], warning, cx);
        let mut partial = super::local_terminal("commit_outcome_unknown", false);
        partial.files_committed = 1;
        partial.commit_state = Some("CommitOutcomeUnknown".into());
        app.job_terminal(ids[1], partial, cx);
        let mut cleanup = terminal(ids[0], true);
        cleanup.terminal = app.jobs.records[&ids[0]].terminal.clone();
        endpoint
            .send_notification_for_contract_test(PlatformNotification::Event(
                PlatformEvent::ArchiveJob(Box::new(cleanup)),
            ))
            .expect("original bridge observes cleanup");
        let snapshot = app.batch_snapshot_for(ids[0]).expect("batch");
        assert_eq!(snapshot.members.len(), 3);
        assert_eq!(snapshot.members[0].state, S::Warnings);
        assert_eq!(snapshot.members[1].state, S::Failed);
        assert!(snapshot.members[1].cleanup_unknown && snapshot.members[1].published);
        assert!(app.jobs.queue.is_running(ids[1]));
        let unrelated = app
            .submit_batch(vec![request(704), request(705)], cx)
            .expect("unrelated batch");
        assert_ne!(
            app.batch_snapshot_for(unrelated[0]).expect("other").id,
            snapshot.id
        );
        assert!(
            app.jobs.queue.is_running(unrelated[0]),
            "other state {} terminal {:?}",
            app.jobs.records[&unrelated[0]].state,
            app.jobs.records[&unrelated[0]]
                .terminal
                .as_ref()
                .map(|r| r.code.clone())
        );
        assert!(!app.jobs.queue.is_running(unrelated[1]));
    });
    while endpoint.try_receive_command_for_contract_test().is_ok() {}
}

#[gpui_kit::test]
fn hz3_all_cancelled_is_not_success_and_retention_never_evicts_cleanup_held(
    cx: &mut gpui_kit::TestAppContext,
) {
    use crate::archive::batch::MemberState as S;
    let (owner, endpoint) = super::super::tests::setup(cx);
    owner.update(cx, |app, cx| {
        let notifications = app.platform.notification_receiver();
        let ids = app
            .submit_batch(vec![request(801), request(802), request(803)], cx)
            .expect("batch");
        app.control_job(ids[2], ArchiveJobAction::Cancel, cx);
        for id in &ids[..2] {
            let mut cancelled = super::local_terminal("cancelled", true);
            cancelled.outcome = ArchiveJobOutcome::Cancelled;
            let mut event = terminal(*id, true);
            event.terminal = Some(cancelled.clone());
            endpoint
                .send_notification_for_contract_test(PlatformNotification::Event(
                    PlatformEvent::ArchiveJob(Box::new(event)),
                ))
                .expect("original cleanup");
            app.job_terminal(*id, cancelled, cx);
        }
        let batch = app.batch_snapshot_for(ids[0]).expect("allcancelled");
        assert!(batch.members.iter().all(|m| m.state == S::Cancelled));
        assert!(batch.heading().contains("成功 0"));
        for n in 0..66 {
            let members = app
                .submit_batch(vec![request(900 + n * 2), request(901 + n * 2)], cx)
                .expect("original queue");
            for id in members {
                let mut done = super::local_terminal("cancelled", true);
                done.outcome = ArchiveJobOutcome::Cancelled;
                let mut event = terminal(id, true);
                event.terminal = Some(done.clone());
                endpoint
                    .send_notification_for_contract_test(PlatformNotification::Event(
                        PlatformEvent::ArchiveJob(Box::new(event)),
                    ))
                    .expect("cleanup bridge");
                app.job_terminal(id, done, cx);
                app.prune_batches();
            }
            while let Ok(wire) = endpoint.try_receive_command_for_contract_test() {
                let receipt = PlatformNotification::CommandResult {
                    correlation_id: wire.id,
                    result: nxtools_protocol::PlatformCommandResult {
                        success: true,
                        error: None,
                        native_error_code: None,
                        data: None,
                    },
                };
                endpoint
                    .send_notification_for_contract_test(receipt.clone())
                    .expect("native receipt");
                app.job_notification(&receipt, cx);
            }
            while notifications.try_recv().is_ok() {}
        }
        while notifications.try_recv().is_ok() {}
        assert!(
            app.jobs.batches.len() <= 64,
            "completed windows must not retain unbounded batch metadata"
        );
    });
}

#[gpui_kit::test]
fn hz3_rendered_summary_pointer_keyboard_details_disabled_retry_and_minimum(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (owner, endpoint) = super::super::tests::setup(cx);
    let ids = owner.update(cx, |app, cx| {
        app.submit_batch(vec![request(1101), request(1102)], cx)
            .expect("batch")
    });
    cx.run_until_parked();
    let window = owner.read_with(cx, |app, _| {
        app.jobs.records[&ids[0]].window.as_ref().expect("view").0
    });
    let mut surface = gpui_kit::VisualTestContext::from_window(window, cx);
    surface.simulate_resize(gpui_kit::size(gpui_kit::px(480.), gpui_kit::px(260.)));
    object_draw(&mut surface);
    object_click(&mut surface, "批次汇总");
    let labels = object_labels(&mut surface);
    assert!(
        labels.iter().any(|s| s.contains("压缩包任务 2")),
        "actual labels {labels:?}"
    );
    assert!(labels.iter().any(|s| s.contains("不汇总未知字节百分比")));
    surface.update(|window, _| {
        let nodes = gpui_kit::base::test_support::snapshots(window);
        let retries = nodes
            .iter()
            .filter(|n| n.label() == Some("重试"))
            .collect::<Vec<_>>();
        assert_eq!(retries.len(), 2);
        assert!(retries.iter().all(|n| n.disabled() == Some(true)));
    });
    while endpoint.try_receive_command_for_contract_test().is_ok() {}
    surface.simulate_keystrokes("tab");
    surface.simulate_keystrokes("space");
    object_draw(&mut surface);
    assert!(!std::iter::from_fn(||endpoint.try_receive_command_for_contract_test().ok()).any(|wire|matches!(wire.message,Message::PlatformCommand(PlatformCommand::ArchiveJob(ref c)) if c.action==ArchiveJobAction::Start)),"presentation keyboard never resubmits task");
    // Return through the same original footer; no new window/controller is created.
    if object_labels(&mut surface).iter().any(|s| s == "任务详情") {
        object_click(&mut surface, "任务详情");
    }
    assert!(object_labels(&mut surface).iter().any(|s| s == "批次汇总"));
}

#[gpui_kit::test]
fn hz3_retry_refuses_changed_source_target_unknown_and_late_baseline(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (owner, endpoint) = super::super::tests::setup(cx);
    let ids = owner.update(cx, |app, cx| {
        app.submit_batch(vec![request(1201), request(1202)], cx)
            .expect("batch")
    });
    while endpoint.try_receive_command_for_contract_test().is_ok() {}
    owner.update(cx,|app,_|{let r=app.jobs.records.get_mut(&ids[0]).expect("record");r.retry_baseline=Some(serde_json::from_value(serde_json::json!({"version":2,"operation_id":Uuid::new_v4(),"session_id":ids[0],"generation":1,"available_bytes":0,"allocation_bytes":4096,"target_identity":"v1:target","volume_identity":"v1:volume","destination_missing":false,"source_matches":true,"source_set":{"identity":"v1:source","primary":r.command.request.as_ref().expect("request").paths[0],"members":[r.command.request.as_ref().expect("request").paths[0]]}})).expect("baseline"));});
    let mut failure = terminal(ids[0], true);
    failure.terminal = Some(super::local_terminal("io_failure", true));
    failure
        .terminal
        .as_mut()
        .expect("native fixture")
        .output_published = Some(false);
    deliver(&owner, &endpoint, failure, cx);
    owner.update(cx, |app, _| {
        let record = app.jobs.records.get_mut(&ids[0]).expect("record");
        assert!(
            batch_jobs::retry_facts(record),
            "original zero-output retry"
        );
        record.terminal.as_mut().expect("terminal").journal_summary =
            Some(ArchiveJobJournalSummary {
                total_records: 1,
                detail_complete: false,
                outcomes: vec![ArchiveJobCommitOutcome {
                    state: "CommitOutcomeUnknown".into(),
                    published: None,
                    cleanup: "retained_uncertain".into(),
                    files: 1,
                    bytes: 0,
                }],
            });
        assert!(
            !batch_jobs::retry_facts(record),
            "bounded detail is not complete zero-output proof"
        );
        record.terminal.as_mut().expect("terminal").journal_summary = None;
    });
    owner.update(cx, |app, cx| app.retry_job(ids[0], cx));
    let command = std::iter::from_fn(|| endpoint.try_receive_command_for_contract_test().ok())
        .find_map(|wire| {
            if let Message::PlatformCommand(PlatformCommand::ArchiveSpace(c)) = wire.message {
                Some(c)
            } else {
                None
            }
        })
        .expect("recheck");
    owner.update(cx, |app, cx| {
        app.job_observation_result(
            command.operation_id,
            &nxtools_protocol::PlatformCommandResult {
                success: false,
                error: Some("source_changed".into()),
                native_error_code: None,
                data: None,
            },
            cx,
        );
        assert!(app.jobs.records[&ids[0]].terminal.is_some());
        assert!(!app.jobs.records[&ids[0]].retry_pending);
        let r = app.jobs.records.get_mut(&ids[0]).expect("record");
        r.terminal.as_mut().expect("terminal").commit_state = Some("CommitOutcomeUnknown".into());
        app.retry_job(ids[0], cx);
    });
    assert!(!std::iter::from_fn(||endpoint.try_receive_command_for_contract_test().ok()).any(|wire|matches!(wire.message,Message::PlatformCommand(PlatformCommand::ArchiveJob(ref c)) if c.action==ArchiveJobAction::Start)));
}

#[gpui_kit::test]
fn hz3_batch_invalid_capacity_memory_is_atomic_and_does_not_leave_deferred_pump(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (owner, _endpoint) = super::super::tests::setup(cx);
    owner.update(cx, |app, cx| {
        let mut invalid = request(1301);
        invalid.limits.max_working_bytes = 2 << 30;
        assert!(app.submit_batch(vec![request(1300), invalid], cx).is_err());
        assert!(app.jobs.records.is_empty());
        assert!(!app.jobs.defer_pump);
        assert!(
            app.submit_batch((0..33).map(|n| request(1400 + n)).collect(), cx)
                .is_err()
        );
        assert!(app.jobs.records.is_empty());
        let ids = app
            .submit_batch(vec![request(1500), request(1501)], cx)
            .expect("next valid");
        assert!(ids.iter().all(|id| app.jobs.queue.is_running(*id)));
    });
}

#[gpui_kit::test]
fn hz3_original_confirmation_forms_share_batch_and_cancel_is_never_started(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (owner, _endpoint) = super::super::tests::setup(cx);
    owner.update(cx, |app, cx| {
        app.open_extract_confirmation(
            r"C:\owned\confirm-a.zip".into(),
            nxtools_core::archive::ArchiveDestination::Here,
            cx,
        );
        app.open_extract_confirmation(
            r"C:\owned\confirm-b.zip".into(),
            nxtools_core::archive::ArchiveDestination::Here,
            cx,
        );
        let forms = app.jobs.forms.keys().copied().collect::<Vec<_>>();
        assert_eq!(forms.len(), 2);
        app.bind_confirmation_forms(forms.clone(), cx);
        let form = forms[0];
        let mut input = request(1600);
        input.paths = vec![r"C:\owned\confirm-a.zip".into()];
        app.submit_form(form, 1, input, completion(), cx);
        let id = *app.jobs.records.keys().next().expect("one original job");
        let snapshot = app.batch_snapshot_for(id).expect("shared batch");
        assert_eq!(snapshot.members.len(), 1);
        assert_eq!(snapshot.pending_forms, 1);
        assert_eq!(snapshot.cancelled_unstarted, 0);
        app.retire_form(forms[1]);
        app.refresh_all_batch_views(cx);
        let snapshot = app.batch_snapshot_for(id).expect("batch remains");
        assert_eq!(snapshot.pending_forms, 0);
        assert_eq!(snapshot.cancelled_unstarted, 1);
        assert_eq!(snapshot.members.len(), 1);
        assert!(snapshot.facts().contains("提交前取消 1"));
    });
}

#[gpui_kit::test]
fn hz3_rendered_eligible_retry_click_rechecks_once_then_original_new_attempt(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (owner, endpoint) = super::super::tests::setup(cx);
    let ids = owner.update(cx, |app, cx| {
        app.submit_batch(vec![request(1700), request(1701)], cx)
            .expect("batch")
    });
    while endpoint.try_receive_command_for_contract_test().is_ok() {}
    owner.update(cx,|app,_|{let r=app.jobs.records.get_mut(&ids[0]).expect("member");r.retry_baseline=Some(serde_json::from_value(serde_json::json!({"version":2,"operation_id":Uuid::new_v4(),"session_id":ids[0],"generation":1,"available_bytes":0,"allocation_bytes":4096,"target_identity":"v1:target","volume_identity":"v1:volume","destination_missing":false,"source_matches":true,"source_set":{"identity":"v1:source","primary":r.command.request.as_ref().expect("request").paths[0],"members":[r.command.request.as_ref().expect("request").paths[0]]}})).expect("native baseline"));});
    let mut failed = terminal(ids[0], true);
    failed.terminal = Some(super::local_terminal("io_failure", true));
    failed
        .terminal
        .as_mut()
        .expect("native zero-output fixture")
        .output_published = Some(false);
    deliver(&owner, &endpoint, failed, cx);
    cx.run_until_parked();
    let window = owner.read_with(cx, |app, _| {
        app.jobs.records[&ids[0]].window.as_ref().expect("detail").0
    });
    let mut surface = gpui_kit::VisualTestContext::from_window(window, cx);
    surface.simulate_resize(gpui_kit::size(gpui_kit::px(760.), gpui_kit::px(720.)));
    object_draw(&mut surface);
    object_click(&mut surface, "批次汇总");
    let retry_point = surface.update(|window, _| {
        gpui_kit::base::test_support::snapshots(window)
            .into_iter()
            .find(|n| n.label() == Some("重试") && n.disabled() != Some(true))
            .expect("enabled original retry")
            .bounds()
            .center()
    });
    surface.simulate_event(gpui_kit::MouseDownEvent {
        position: retry_point,
        button: gpui_kit::MouseButton::Left,
        click_count: 1,
        ..Default::default()
    });
    surface.simulate_event(gpui_kit::MouseUpEvent {
        position: retry_point,
        button: gpui_kit::MouseButton::Left,
        click_count: 1,
        modifiers: Default::default(),
    });
    surface.run_until_parked();
    object_draw(&mut surface);
    // Repeated pointer/keyboard activation while pending cannot enqueue another recheck.
    surface.simulate_keystrokes("space");
    object_draw(&mut surface);
    let rechecks = std::iter::from_fn(|| endpoint.try_receive_command_for_contract_test().ok())
        .filter_map(|wire| {
            if let Message::PlatformCommand(PlatformCommand::ArchiveSpace(c)) = wire.message {
                c.expected_source.is_some().then_some(c)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    assert_eq!(rechecks.len(), 1);
    let command = &rechecks[0];
    let mut observed = owner.read_with(&surface, |app, _| {
        app.jobs.records[&ids[0]]
            .retry_baseline
            .clone()
            .expect("old facts")
    });
    observed.operation_id = command.operation_id;
    observed.generation = command.generation;
    observed.session_id = command.session_id;
    owner.update(&mut surface, |app, cx| {
        app.job_observation_result(
            command.operation_id,
            &nxtools_protocol::PlatformCommandResult {
                success: true,
                error: None,
                native_error_code: None,
                data: Some(serde_json::to_string(&observed).expect("reply")),
            },
            cx,
        );
    });
    let starts = std::iter::from_fn(|| endpoint.try_receive_command_for_contract_test().ok())
        .filter_map(|wire| {
            if let Message::PlatformCommand(PlatformCommand::ArchiveJob(c)) = wire.message {
                Some(c)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    assert_eq!(
        starts
            .iter()
            .map(|command| command.action)
            .collect::<Vec<_>>(),
        [ArchiveJobAction::RetireBatch, ArchiveJobAction::Start],
        "retire old batch password before the sole new attempt"
    );
    let starts = starts
        .into_iter()
        .filter(|command| command.action == ArchiveJobAction::Start)
        .collect::<Vec<_>>();
    assert_eq!(starts.len(), 1);
    assert_ne!(starts[0].operation_id, ids[0]);
    assert_eq!(starts[0].generation, 2);
    assert_eq!(starts[0].session_id, ids[0]);
}

fn hz3_wait_start(
    endpoint: &nxtools_ipc::PlatformServerEndpoint,
    cx: &mut gpui_kit::TestAppContext,
) -> ArchiveJobCommand {
    for _ in 0..100 {
        cx.executor()
            .advance_clock(std::time::Duration::from_millis(25));
        cx.run_until_parked();
        while let Ok(wire) = endpoint.try_receive_command_for_contract_test() {
            if let Message::PlatformCommand(PlatformCommand::ArchiveJob(command)) = wire.message
                && command.action == ArchiveJobAction::Start
            {
                return *command;
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    panic!("original native attempt not dispatched")
}

fn hz3_observation_reply(
    command: &nxtools_protocol::archive_space::ArchiveSpaceCommand,
    mut observation: nxtools_protocol::archive_space::ArchiveSpaceObservation,
) -> PlatformNotification {
    observation.operation_id = command.operation_id;
    observation.session_id = command.session_id;
    observation.generation = command.generation;
    PlatformNotification::CommandResult {
        correlation_id: command.operation_id,
        result: nxtools_protocol::PlatformCommandResult {
            success: true,
            error: None,
            native_error_code: None,
            data: Some(serde_json::to_string(&observation).expect("native observation wire")),
        },
    }
}

#[gpui_kit::test]
fn hz3_ac01_actual_native_directory_terminal_reaches_original_retry_and_summary(
    cx: &mut gpui_kit::TestAppContext,
) {
    #[derive(serde::Deserialize)]
    struct NativeCase {
        name: String,
        command: ArchiveJobCommand,
        baseline: nxtools_protocol::archive_space::ArchiveSpaceObservation,
        terminal: ArchiveJobTerminal,
        source_unchanged: bool,
        fresh_observation_matches: bool,
    }
    #[derive(serde::Deserialize)]
    struct Evidence {
        cases: Vec<NativeCase>,
    }
    let bytes = if let Ok(path) = std::env::var("NXTOOLS_HZ3_NATIVE_EVIDENCE") {
        std::fs::read(path).expect("explicit current measured C# terminals")
    } else {
        // Exact actual-native transcript, checked against its original evidence SHA256.
        // Replay is not a fresh native execution; no filesystem action uses these paths.
        include_bytes!("fixtures/hz3-native-publication.json").to_vec()
    };
    let native: Evidence = serde_json::from_slice(&bytes).expect("native DTO compatibility");
    assert_eq!(native.cases.len(), 2);
    for case in native.cases {
        let published = case.name == "directory-published";
        assert!(case.source_unchanged && case.fresh_observation_matches);
        assert_eq!(case.terminal.files_committed, 0);
        assert_eq!(case.terminal.result_location.is_some(), published);
        let (owner, endpoint) = super::super::tests::setup(cx);
        let id = owner.update(cx, |app, cx| {
            let id = app
                .submit_job(
                    case.command.request.clone().expect("native request"),
                    completion(),
                    cx,
                )
                .expect("original extract");
            app.bind_batch(vec![id], None, cx);
            id
        });
        let start = hz3_wait_start(&endpoint, cx);
        let observation = owner.read_with(cx, |app, _| {
            app.jobs
                .observations
                .values()
                .find(|pending| pending.command.session_id == id)
                .expect("original preterminal observation")
                .command
                .clone()
        });
        let reply = hz3_observation_reply(&observation, case.baseline.clone());
        endpoint
            .send_notification_for_contract_test(reply.clone())
            .expect("original observation receipt");
        owner.update(cx, |app, cx| {
            assert!(app.job_notification(&reply, cx));
        });
        assert!(owner.read_with(cx, |app, _| app.jobs.records[&id].retry_baseline.is_some()));
        let mut ended = terminal(start.operation_id, true);
        ended.session_id = start.session_id;
        ended.generation = start.generation;
        ended.terminal = Some(case.terminal.clone());
        deliver(&owner, &endpoint, ended.clone(), cx);
        for _ in 0..100 {
            cx.executor()
                .advance_clock(std::time::Duration::from_millis(25));
            cx.run_until_parked();
            if owner.read_with(cx, |app, _| app.jobs.records[&id].terminal.is_some()) {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        owner.read_with(cx, |app, _| {
            let snapshot = app.batch_snapshot_for(id).expect("original summary");
            assert_eq!(snapshot.members.len(), 1);
            assert_eq!(
                snapshot.members[0].state,
                crate::archive::batch::MemberState::Failed
            );
            assert_eq!(
                snapshot.members[0].published, published,
                "real directory publication must be visible"
            );
            assert_eq!(
                snapshot.members[0].retry, !published,
                "published directory must not enable blind replay"
            );
            assert_eq!(
                app.jobs.records[&id]
                    .terminal
                    .as_ref()
                    .expect("logical done")
                    .result_location,
                case.terminal.result_location
            );
        });
        while endpoint.try_receive_command_for_contract_test().is_ok() {}
        owner.update(cx, |app, cx| {
            app.retry_job(id, cx);
            app.retry_job(id, cx);
        });
        let checks = std::iter::from_fn(|| endpoint.try_receive_command_for_contract_test().ok())
            .filter_map(|wire| {
                if let Message::PlatformCommand(PlatformCommand::ArchiveSpace(command)) =
                    wire.message
                {
                    Some(command)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        assert_eq!(checks.len(), usize::from(!published));
        if let Some(command) = checks.first() {
            assert_eq!(
                command.expected_source,
                case.baseline
                    .source_set
                    .as_ref()
                    .map(|source| source.identity.clone())
            );
            let reply = hz3_observation_reply(command, case.baseline);
            endpoint
                .send_notification_for_contract_test(reply.clone())
                .expect("original explicit recheck");
            owner.update(cx, |app, cx| {
                app.job_notification(&reply, cx);
            });
            let retry = hz3_wait_start(&endpoint, cx);
            assert_ne!(retry.operation_id, start.operation_id);
            assert_eq!(retry.session_id, start.session_id);
            assert_eq!(retry.generation, 2);
            // Retire the real finite coordinator; no detached native test worker survives.
            let mut done = terminal(retry.operation_id, true);
            done.session_id = retry.session_id;
            done.generation = retry.generation;
            done.terminal = Some(local_terminal("io_failure", true));
            deliver(&owner, &endpoint, done, cx);
        }
    }
}

fn hz3_extract_request(n: usize) -> ArchiveJobRequest {
    let mut input = request(n);
    input.kind = ArchiveJobKind::Extract;
    input.destination = Some(format!(r"C:\owned\out-{n}"));
    input.extract = Some(ArchiveJobExtract {
        strategy: ArchiveJobStrategy::Here,
        single_file_folder: false,
        conflict: ArchiveJobConflict::Ask,
        copy_motw: false,
        selection: None,
        reclamation: None,
    });
    input
}

fn hz3_fixture_observation(
    command: &nxtools_protocol::archive_space::ArchiveSpaceCommand,
    identity: &str,
) -> nxtools_protocol::archive_space::ArchiveSpaceObservation {
    serde_json::from_value(serde_json::json!({"version":2,"operation_id":command.operation_id,"session_id":command.session_id,"generation":command.generation,"available_bytes":0,"allocation_bytes":4096,"target_identity":"v1:target","volume_identity":"v1:volume","destination_missing":false,"source_matches":true,"source_set":{"identity":identity,"primary":command.source,"members":[command.source]}})).expect("bounded original observation DTO fixture")
}

fn hz3_pending_observation(
    owner: &Entity<ArchiveApplication>,
    id: Uuid,
    cx: &gpui_kit::TestAppContext,
) -> nxtools_protocol::archive_space::ArchiveSpaceCommand {
    owner.read_with(cx, |app, _| {
        app.jobs
            .observations
            .values()
            .find(|p| p.command.session_id == id)
            .expect("original pending observation")
            .command
            .clone()
    })
}

fn hz3_wait_terminal(
    owner: &Entity<ArchiveApplication>,
    id: Uuid,
    cx: &mut gpui_kit::TestAppContext,
) {
    for _ in 0..100 {
        cx.executor()
            .advance_clock(std::time::Duration::from_millis(25));
        cx.run_until_parked();
        if owner.read_with(cx, |app, _| app.jobs.records[&id].terminal.is_some()) {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    panic!("original logical terminal not delivered")
}

#[gpui_kit::test]
fn hz3_ac02_native_terminal_gap_refuses_late_first_baseline_and_replay(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (owner, endpoint) = super::super::tests::setup(cx);
    let id = owner.update(cx, |app, cx| {
        let id = app
            .submit_job(hz3_extract_request(2100), completion(), cx)
            .expect("original extract");
        app.bind_batch(vec![id], None, cx);
        id
    });
    let start = hz3_wait_start(&endpoint, cx);
    let pending = hz3_pending_observation(&owner, id, cx);
    while endpoint.try_receive_command_for_contract_test().is_ok() {}
    let late = hz3_observation_reply(
        &pending,
        hz3_fixture_observation(&pending, "v1:replacement-B"),
    );
    let mut ended = terminal(start.operation_id, true);
    ended.session_id = start.session_id;
    ended.terminal = Some(local_terminal("io_failure", true));
    ended
        .terminal
        .as_mut()
        .expect("native zero-output fixture")
        .output_published = Some(false);
    let native = PlatformNotification::Event(PlatformEvent::ArchiveJob(Box::new(ended)));
    endpoint
        .send_notification_for_contract_test(native.clone())
        .expect("actual IPC cleanup observation");
    endpoint
        .send_notification_for_contract_test(late.clone())
        .expect("late observation receipt");
    owner.update(cx, |app, cx| {
        app.job_notification(&native, cx);
        assert!(
            app.jobs.records[&id].terminal.is_none(),
            "exercise real native-ended/logical-pending coordinator gap"
        );
        app.job_notification(&late, cx);
        assert!(
            app.jobs.records[&id].retry_baseline.is_none(),
            "source B observed after native end cannot become first baseline"
        );
        app.observe_job(id, false, cx);
    });
    hz3_wait_terminal(&owner, id, cx);
    owner.update(cx, |app, cx| {
        app.retry_job(id, cx);
    });
    assert!(
        !std::iter::from_fn(|| endpoint.try_receive_command_for_contract_test().ok()).any(|wire| {
            match wire.message {
                Message::PlatformCommand(PlatformCommand::ArchiveJob(command)) => {
                    command.action != ArchiveJobAction::RetireBatch
                }
                Message::PlatformCommand(PlatformCommand::ArchiveSpace(_)) => true,
                _ => false,
            }
        }),
        "late context cannot trigger recheck or new attempt"
    );
    owner.read_with(cx, |app, _| {
        let batch = app.batch_snapshot_for(id).expect("same batch");
        assert_eq!(batch.members.len(), 1);
        assert!(!batch.members[0].retry);
        assert!(app.jobs.records[&id].terminal_notified);
    });
}

#[gpui_kit::test]
fn hz3_ac02_finite_attempt_transition_refuses_old_observation_and_reacquisition(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (owner, endpoint) = super::super::tests::setup(cx);
    let id = owner.update(cx, |app, cx| {
        let mut input = hz3_extract_request(2200);
        input.limits.max_entry_bytes = 2;
        input.limits.max_output_bytes = 2;
        let id = app
            .submit_job(input, completion(), cx)
            .expect("finite original extract");
        app.bind_batch(vec![id], None, cx);
        id
    });
    let first = hz3_wait_start(&endpoint, cx);
    let pending = hz3_pending_observation(&owner, id, cx);
    while endpoint.try_receive_command_for_contract_test().is_ok() {}
    let late = hz3_observation_reply(
        &pending,
        hz3_fixture_observation(&pending, "v1:replacement-B"),
    );
    let mut shortage = terminal(first.operation_id, true);
    shortage.session_id = first.session_id;
    let mut facts = local_terminal("resource_limit", true);
    facts.output_published = Some(false);
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
            native: "v1:original-A".into(),
            digest: [7; 32],
        },
    });
    shortage.terminal = Some(facts);
    deliver(&owner, &endpoint, shortage, cx);
    let mut prompt = None;
    for _ in 0..100 {
        cx.executor()
            .advance_clock(std::time::Duration::from_millis(25));
        cx.run_until_parked();
        prompt = owner.read_with(cx, |app, _| app.jobs.records[&id].prompt.clone());
        if prompt.is_some() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    let prompt = prompt.expect("original finite output prompt");
    owner.update(cx, |app, cx| {
        app.reply_job(
            id,
            prompt.clone(),
            ArchiveJobReply {
                ticket: prompt.ticket,
                kind: prompt.kind,
                password: None,
                decision: None,
                apply_to_all: false,
                approved_limit: Some(3),
                remember_batch_password: false,
            },
            cx,
        )
    });
    let second = hz3_wait_start(&endpoint, cx);
    assert_ne!(second.operation_id, first.operation_id);
    assert_eq!(
        second.generation, first.generation,
        "logical generation alone cannot reject prior attempt"
    );
    endpoint
        .send_notification_for_contract_test(late.clone())
        .expect("original delayed receipt");
    owner.update(cx, |app, cx| {
        app.job_notification(&late, cx);
        assert!(
            app.jobs.records[&id].retry_baseline.is_none(),
            "old attempt cannot capture source B after real finite transition"
        );
        assert!(
            !app.jobs
                .observations
                .values()
                .any(|p| p.command.session_id == id),
            "missing original baseline cannot be reacquired from replacement attempt"
        );
    });
    let mut ended = terminal(second.operation_id, true);
    ended.session_id = second.session_id;
    ended.terminal = Some(local_terminal("io_failure", true));
    ended
        .terminal
        .as_mut()
        .expect("native zero-output fixture")
        .output_published = Some(false);
    deliver(&owner, &endpoint, ended.clone(), cx);
    deliver(&owner, &endpoint, ended, cx);
    hz3_wait_terminal(&owner, id, cx);
    owner.update(cx, |app, cx| {
        app.retry_job(id, cx);
    });
    assert!(
        !std::iter::from_fn(|| endpoint.try_receive_command_for_contract_test().ok()).any(|wire| {
            match wire.message {
                Message::PlatformCommand(PlatformCommand::ArchiveJob(command)) => {
                    command.action != ArchiveJobAction::RetireBatch
                }
                Message::PlatformCommand(PlatformCommand::ArchiveSpace(_)) => true,
                _ => false,
            }
        })
    );
    owner.read_with(cx, |app, _| {
        assert_eq!(app.batch_snapshot_for(id).expect("batch").members.len(), 1);
        assert_eq!(
            app.jobs.retired.iter().filter(|old| **old == id).count(),
            1,
            "finite member finishes exactly once"
        );
    });
}

#[gpui_kit::test]
fn hz3_ac02_current_preterminal_observation_preserves_original_baseline(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (owner, endpoint) = super::super::tests::setup(cx);
    let id = owner.update(cx, |app, cx| {
        let id = app
            .submit_job(hz3_extract_request(2300), completion(), cx)
            .expect("original extract");
        app.bind_batch(vec![id], None, cx);
        id
    });
    let start = hz3_wait_start(&endpoint, cx);
    let pending = hz3_pending_observation(&owner, id, cx);
    let valid = hz3_observation_reply(&pending, hz3_fixture_observation(&pending, "v1:original-A"));
    endpoint
        .send_notification_for_contract_test(valid.clone())
        .expect("current original receipt");
    owner.update(cx, |app, cx| {
        app.job_notification(&valid, cx);
    });
    let mut ended = terminal(start.operation_id, true);
    ended.session_id = start.session_id;
    ended.terminal = Some(local_terminal("io_failure", true));
    ended
        .terminal
        .as_mut()
        .expect("native zero-output fixture")
        .output_published = Some(false);
    deliver(&owner, &endpoint, ended, cx);
    hz3_wait_terminal(&owner, id, cx);
    owner.read_with(cx, |app, _| {
        let baseline = app.jobs.records[&id]
            .retry_baseline
            .as_ref()
            .expect("valid pre-native-terminal baseline retained");
        assert_eq!(
            baseline.source_set.as_ref().expect("source set").identity,
            "v1:original-A"
        );
        assert!(app.batch_snapshot_for(id).expect("batch").members[0].retry);
    });
}

fn hz3_ack_original_starts(
    owner: &Entity<ArchiveApplication>,
    endpoint: &nxtools_ipc::PlatformServerEndpoint,
    cx: &mut gpui_kit::TestAppContext,
) {
    while let Ok(wire) = endpoint.try_receive_command_for_contract_test() {
        if matches!(
            wire.message,
            Message::PlatformCommand(PlatformCommand::ArchiveJob(_))
        ) {
            let reply = PlatformNotification::CommandResult {
                correlation_id: wire.id,
                result: nxtools_protocol::PlatformCommandResult {
                    success: true,
                    error: None,
                    native_error_code: None,
                    data: None,
                },
            };
            endpoint
                .send_notification_for_contract_test(reply.clone())
                .expect("actual original Start receipt");
            owner.update(cx, |app, cx| {
                app.job_notification(&reply, cx);
            });
        }
    }
}

fn hz3_complete_closed_test(
    owner: &Entity<ArchiveApplication>,
    endpoint: &nxtools_ipc::PlatformServerEndpoint,
    id: Uuid,
    cleanup: bool,
    cx: &mut gpui_kit::TestAppContext,
) {
    owner.update(cx, |app, cx| app.background_job(id, cx));
    hz3_ack_original_starts(owner, endpoint, cx);
    let mut ended = terminal(id, cleanup);
    let mut facts = local_terminal("io_failure", cleanup);
    facts.outcome = ArchiveJobOutcome::Succeeded;
    facts.code = None;
    facts.output_published = Some(false);
    ended.terminal = Some(facts);
    deliver(owner, endpoint, ended, cx);
    cx.run_until_parked();
}

#[gpui_kit::test]
fn hz3_ac03_closed_member_survives_late_sibling_then_is_reclaimed_at_original_64_boundary(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (owner, endpoint) = super::super::tests::setup(cx);
    let notifications = owner.read_with(cx, |app, _| app.platform.notification_receiver());
    let ids = owner.update(cx, |app, cx| {
        app.submit_batch(vec![request(3000), request(3001)], cx)
            .expect("original two-slot batch")
    });
    hz3_complete_closed_test(&owner, &endpoint, ids[0], true, cx);
    owner.update(cx, |app, cx| app.background_job(ids[1], cx));
    for n in 0..70 {
        let id = owner.update(cx, |app, cx| {
            app.submit_job(request(3100 + n), completion(), cx)
                .expect("unrelated original second slot")
        });
        hz3_complete_closed_test(&owner, &endpoint, id, true, cx);
        while notifications.try_recv().is_ok() {}
        owner.read_with(cx, |app, _| {
            assert!(
                app.jobs.records.contains_key(&ids[0]),
                "completed A protected until B ends"
            );
            assert!(app.jobs.records[&ids[0]].window.is_none());
            assert!(
                app.jobs.queue.is_running(ids[1]),
                "active B must retain original slot"
            );
            assert_eq!(
                app.batch_snapshot_for(ids[1])
                    .expect("active batch retained")
                    .members
                    .len(),
                2
            );
        });
    }
    hz3_complete_closed_test(&owner, &endpoint, ids[1], true, cx);
    for n in 0..130 {
        let id = owner.update(cx, |app, cx| {
            app.submit_job(request(3300 + n), completion(), cx)
                .expect("subsequent original queue pressure")
        });
        hz3_complete_closed_test(&owner, &endpoint, id, true, cx);
        while notifications.try_recv().is_ok() {}
    }
    owner.read_with(cx, |app, _| {
        let orphaned = app
            .jobs
            .records
            .iter()
            .filter(|(id, record)| {
                record.window.is_none()
                    && record.terminal.as_ref().is_some_and(|t| t.cleanup_complete)
                    && !app.jobs.queue.is_running(**id)
                    && !app.jobs.retired.contains(id)
            })
            .count();
        assert_eq!(
            orphaned, 0,
            "completed batch-held records must never lose original retirement tracking"
        );
        assert_eq!(
            app.jobs.records.len(),
            64,
            "only original bounded completed history remains after protections end"
        );
        assert_eq!(app.jobs.retired.len(), 64);
        assert!(!app.jobs.records.contains_key(&ids[0]));
        assert!(!app.jobs.records.contains_key(&ids[1]));
    });
}

#[gpui_kit::test]
fn hz3_ac03_cleanup_pending_confirmation_and_retry_wait_keep_tracking_and_slots(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (owner, endpoint) = super::super::tests::setup(cx);
    let notifications = owner.read_with(cx, |app, _| app.platform.notification_receiver());
    let held = owner.update(cx, |app, cx| {
        let id = app
            .submit_job(request(4000), completion(), cx)
            .expect("original native slot");
        app.bind_batch(vec![id], None, cx);
        id
    });
    hz3_complete_closed_test(&owner, &endpoint, held, false, cx);
    let (pending_member, pending_form) = owner.update(cx, |app, cx| {
        for name in ["pending-a.zip", "pending-b.zip"] {
            app.open_extract_confirmation(
                format!(r"C:\owned\{name}").into(),
                nxtools_core::archive::ArchiveDestination::Here,
                cx,
            );
        }
        let forms = app.jobs.forms.keys().copied().collect::<Vec<_>>();
        assert_eq!(forms.len(), 2);
        app.bind_confirmation_forms(forms.clone(), cx);
        let mut input = request(4100);
        input.paths = vec![r"C:\owned\pending-a.zip".into()];
        app.submit_form(forms[0], 1, input, completion(), cx);
        let id = *app
            .jobs
            .records
            .keys()
            .find(|id| **id != held)
            .expect("submitted original member");
        (id, forms[1])
    });
    hz3_complete_closed_test(&owner, &endpoint, pending_member, true, cx);
    let retry_member = owner.update(cx, |app, cx| {
        let id = app
            .submit_job(request(4200), completion(), cx)
            .expect("other original slot");
        app.bind_batch(vec![id], None, cx);
        id
    });
    cx.run_until_parked();
    hz3_ack_original_starts(&owner, &endpoint, cx);
    let observation = hz3_pending_observation(&owner, retry_member, cx);
    let reply = hz3_observation_reply(
        &observation,
        hz3_fixture_observation(&observation, "v1:original-A"),
    );
    endpoint
        .send_notification_for_contract_test(reply.clone())
        .expect("original preterminal baseline");
    owner.update(cx, |app, cx| {
        app.job_notification(&reply, cx);
    });
    let mut ended = terminal(retry_member, true);
    let mut facts = local_terminal("io_failure", true);
    facts.output_published = Some(false);
    ended.terminal = Some(facts);
    deliver(&owner, &endpoint, ended, cx);
    owner.update(cx, |app, cx| {
        app.background_job(retry_member, cx);
        app.retry_job(retry_member, cx);
    });
    let recheck = hz3_pending_observation(&owner, retry_member, cx);
    for n in 0..70 {
        let id = owner.update(cx, |app, cx| {
            app.submit_job(request(4300 + n), completion(), cx)
                .expect("only remaining original native slot")
        });
        hz3_complete_closed_test(&owner, &endpoint, id, true, cx);
        while notifications.try_recv().is_ok() {}
    }
    owner.read_with(cx, |app, _| {
        for id in [held, pending_member, retry_member] {
            assert!(app.jobs.records.contains_key(&id));
            assert!(
                app.jobs.retired.contains(&id),
                "all protected completed records remain tracked"
            );
        }
        assert!(
            app.jobs.queue.is_running(held),
            "cleanup false never frees native slot"
        );
        assert!(app.jobs.records[&retry_member].retry_pending);
        assert_eq!(
            app.batch_snapshot_for(pending_member)
                .expect("pending batch")
                .pending_forms,
            1
        );
    });
    let failure = PlatformNotification::CommandResult {
        correlation_id: recheck.operation_id,
        result: nxtools_protocol::PlatformCommandResult {
            success: false,
            error: Some("source_changed".into()),
            native_error_code: None,
            data: None,
        },
    };
    endpoint
        .send_notification_for_contract_test(failure.clone())
        .expect("original failed recheck");
    owner.update(cx, |app, cx| {
        app.job_notification(&failure, cx);
        app.retire_form(pending_form);
    });
    for n in 0..70 {
        let id = owner.update(cx, |app, cx| {
            app.submit_job(request(4500 + n), completion(), cx)
                .expect("same remaining slot")
        });
        hz3_complete_closed_test(&owner, &endpoint, id, true, cx);
        while notifications.try_recv().is_ok() {}
    }
    owner.read_with(cx, |app, _| {
        assert!(app.jobs.records.contains_key(&held));
        assert!(app.jobs.retired.contains(&held));
        assert!(app.jobs.queue.is_running(held));
        assert!(
            !app.jobs.records.contains_key(&pending_member),
            "released pending form becomes reclaimable"
        );
        assert!(
            !app.jobs.records.contains_key(&retry_member),
            "finished recheck becomes reclaimable"
        );
        assert_eq!(app.jobs.records.len(), 64);
        assert_eq!(app.jobs.retired.len(), 64);
    });
}

#[gpui_kit::test]
fn hz3_ac01_missing_native_publication_fact_is_not_zero_output(cx: &mut gpui_kit::TestAppContext) {
    let (owner, endpoint) = super::super::tests::setup(cx);
    let id = owner.update(cx, |app, cx| {
        let id = app
            .submit_job(request(4700), completion(), cx)
            .expect("original Test");
        app.bind_batch(vec![id], None, cx);
        id
    });
    cx.run_until_parked();
    let observation = hz3_pending_observation(&owner, id, cx);
    hz3_ack_original_starts(&owner, &endpoint, cx);
    let reply = hz3_observation_reply(
        &observation,
        hz3_fixture_observation(&observation, "v1:original-A"),
    );
    endpoint
        .send_notification_for_contract_test(reply.clone())
        .expect("original baseline receipt");
    owner.update(cx, |app, cx| {
        app.job_notification(&reply, cx);
    });
    let mut failed = terminal(id, true);
    failed.terminal = Some(local_terminal("io_failure", true)); // Legacy missing fact, empty file journal.
    deliver(&owner, &endpoint, failed, cx);
    owner.update(cx, |app, cx| {
        assert!(!app.batch_snapshot_for(id).expect("summary").members[0].retry);
        app.retry_job(id, cx);
    });
    assert!(
        !std::iter::from_fn(|| endpoint.try_receive_command_for_contract_test().ok()).any(|wire| {
            match wire.message {
                Message::PlatformCommand(PlatformCommand::ArchiveJob(command)) => {
                    command.action != ArchiveJobAction::RetireBatch
                }
                Message::PlatformCommand(PlatformCommand::ArchiveSpace(_)) => true,
                _ => false,
            }
        })
    );
}

fn hz5_continue_request(recovery_id: Uuid) -> ArchiveJobRequest {
    let mut request = hz3_extract_request(5100);
    request.paths = vec![r"C:\owned\preview.zip".into()];
    request.destination = Some(r"C:\owned".into());
    request.extract.as_mut().expect("extract").reclamation = Some(ArchiveReclamationIntent {
        mode: ArchiveReclamationMode::Continue,
        recovery_id,
    });
    request
}

#[gpui_kit::test]
fn hz5_continue_is_new_start_without_readmission_retry_or_live_browser_snapshot(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (owner, endpoint, browser_id, _) = super::super::tests::preview_fixture(cx);
    let recovery_id = Uuid::new_v4();
    let id = owner.update(cx, |app, cx| {
        let id = app
            .submit_job(hz5_continue_request(recovery_id), completion(), cx)
            .expect("reclamation continuation admitted");
        app.bind_batch(vec![id], None, cx);
        assert!(app.windows[&browser_id].browser.is_none());
        assert!(!app.jobs.readmissions.contains_key(&id));
        id
    });
    cx.run_until_parked();
    owner.read_with(cx, |app, cx| {
        let slot = &app.windows[&browser_id];
        assert!(slot.browser.is_none());
        assert!(slot.view.read_with(cx, |view, _| {
            matches!(&view.status, BrowserStatus::Stale(message) if message.contains("空间回收"))
        }).unwrap_or(false));
    });
    let commands = std::iter::from_fn(|| endpoint.try_receive_command_for_contract_test().ok())
        .collect::<Vec<_>>();
    let start = commands
        .iter()
        .find_map(|wire| match &wire.message {
            Message::PlatformCommand(PlatformCommand::ArchiveJob(command)) => Some(command),
            _ => None,
        })
        .expect("direct original Start command");
    assert_eq!(start.action, ArchiveJobAction::Start);
    assert_eq!(start.session_id, id);
    assert_eq!(
        start
            .request
            .as_ref()
            .expect("request")
            .extract
            .as_ref()
            .expect("extract")
            .reclamation,
        Some(ArchiveReclamationIntent {
            mode: ArchiveReclamationMode::Continue,
            recovery_id,
        })
    );
    assert!(
        commands.iter().all(|wire| !matches!(
            &wire.message,
            Message::PlatformCommand(PlatformCommand::ArchiveSpace(_))
        )),
        "reclamation must not enter ordinary retry/readmission observation"
    );

    owner.update(cx, |app, _| {
        app.jobs.records.get_mut(&id).expect("record").retry_baseline = Some(
            serde_json::from_value(serde_json::json!({
                "version":2,
                "operation_id":Uuid::new_v4(),
                "session_id":id,
                "generation":1,
                "available_bytes":0,
                "allocation_bytes":4096,
                "target_identity":"v1:target",
                "volume_identity":"v1:volume",
                "destination_missing":false,
                "source_matches":true,
                "source_set":{"identity":"v1:source","primary":r"C:\owned\preview.zip","members":[r"C:\owned\preview.zip"]}
            }))
            .expect("bounded retry baseline"),
        );
    });
    let mut failed = terminal(id, true);
    let result = failed.terminal.as_mut().expect("terminal");
    *result = local_terminal("io_failure", true);
    result.output_published = Some(false);
    deliver(&owner, &endpoint, failed, cx);
    owner.update(cx, |app, cx| {
        assert!(!app.batch_snapshot_for(id).expect("summary").members[0].retry);
        app.retry_job(id, cx);
    });
    assert!(
        !std::iter::from_fn(|| endpoint.try_receive_command_for_contract_test().ok()).any(|wire| {
            match wire.message {
                Message::PlatformCommand(PlatformCommand::ArchiveJob(command)) => {
                    command.action != ArchiveJobAction::RetireBatch
                }
                Message::PlatformCommand(PlatformCommand::ArchiveSpace(_)) => true,
                _ => false,
            }
        }),
        "reclamation has no generic retry path"
    );
}

fn object_scroll_to(surface: &mut gpui_kit::VisualTestContext, label: &str) {
    for _ in 0..40 {
        let bounds = object_node(surface, label).bounds();
        let viewport = surface
            .debug_bounds("archive-form-body")
            .expect("original form viewport");
        if bounds.top() >= viewport.top() && bounds.bottom() <= viewport.bottom() {
            return;
        }
        let delta = if bounds.top() < viewport.top() {
            3.
        } else {
            -3.
        };
        surface.simulate_event(gpui_kit::ScrollWheelEvent {
            position: viewport.center(),
            delta: gpui_kit::ScrollDelta::Lines(gpui_kit::point(0., delta)),
            ..Default::default()
        });
        object_draw(surface);
    }
    panic!("original form control not reachable: {label}");
}

#[gpui_kit::test]
fn picker_command_rejection_and_disconnect_settle_original_form(cx: &mut gpui_kit::TestAppContext) {
    let (owner, _endpoint) = super::super::tests::setup(cx);
    owner.update(cx, |app, cx| {
        app.open_extract_form(PathBuf::new(), None, cx)
    });
    let (id, generation, view) = owner.read_with(cx, |app, _| {
        let (id, slot) = app.jobs.forms.iter().next().expect("original form");
        (*id, slot.generation, slot.view.clone())
    });
    let operation = Uuid::new_v4();
    view.update(cx, |form, _| form.arm_picker_for_test(operation))
        .expect("open form");
    owner.update(cx, |app, _| {
        app.jobs.pickers.insert(
            operation,
            PendingPicker {
                form: id,
                generation,
                operation,
                started: Instant::now(),
                connection: app.platform.connection_generation().expect("connected"),
                target: None,
            },
        );
    });
    let rejected = PlatformNotification::CommandResult {
        correlation_id: operation,
        result: nxtools_protocol::PlatformCommandResult {
            success: false,
            error: Some("invalid_request".into()),
            data: None,
            native_error_code: None,
        },
    };
    owner.update(cx, |app, cx| assert!(app.job_notification(&rejected, cx)));
    cx.run_until_parked();
    assert!(!owner.read_with(cx, |app, _| app.jobs.pickers.contains_key(&operation)));
    view.read_with(cx, |form, _| assert!(!form.busy()))
        .expect("form still open");
    let next = Uuid::new_v4();
    view.update(cx, |form, _| form.arm_picker_for_test(next))
        .expect("form still open");
    owner.update(cx, |app, _| {
        app.jobs.pickers.insert(
            next,
            PendingPicker {
                form: id,
                generation,
                operation: next,
                started: Instant::now(),
                connection: app.platform.connection_generation().expect("connected"),
                target: None,
            },
        );
    });
    owner.update(cx, |app, cx| {
        app.job_notification(
            &PlatformNotification::Disconnected {
                generation: 1,
                pending_command_ids: vec![next],
            },
            cx,
        );
    });
    cx.run_until_parked();
    view.read_with(cx, |form, _| assert!(!form.busy()))
        .expect("form still open");
}

#[gpui_kit::test]
fn opt01_serialized_edit_phases_reach_original_progress(cx: &mut gpui_kit::TestAppContext) {
    let (owner, endpoint) = super::super::tests::setup(cx);
    use nxtools_protocol::archive_browser::{
        ArchiveBrowserContext, ArchiveSourceBinding, SevenzEditEligibility,
    };
    let edit = ModifyDraft {
        source: r"C:\owned\progress.7z".into(),
        browser: ArchiveBrowserContext {
            snapshot_id: Uuid::new_v4(),
            format: "SevenZ".into(),
            comment: String::new(),
            source: ArchiveSourceBinding {
                native: "v1:fixture".into(),
                digest: [7; 32],
                size: 7,
                modified_unix_ns: 1,
            },
            sevenz_edit: Some(SevenzEditEligibility { refusal: None }),
            tar_edit: None,
        },
        action: ArchiveJobModifyAction::Delete,
        ordinals: vec![0],
        value: None,
        source_prefix: None,
        added: vec![],
        codepage: 0,
    }
    .request()
    .expect("original admitted edit request");
    let id = owner.update(cx, |app, cx| {
        app.submit_job(edit, completion(), cx).expect("edit job")
    });
    opt01_state(&owner, &endpoint, id, ArchiveJobState::Running, cx);
    let window = owner.read_with(cx, |app, _| {
        app.jobs.records[&id]
            .window
            .as_ref()
            .expect("original progress")
            .0
    });
    let mut surface = gpui_kit::VisualTestContext::from_window(window, cx);
    let phases = [
        "running",
        "edit_planning",
        "edit_reading",
        "edit_reencoding",
        "edit_verifying",
        "edit_committing",
    ];
    let events: Vec<ArchiveJobEvent> = if let Some(path) =
        std::env::var_os("NXTOOLS_ARCHIVE_EDIT_PROGRESS_WIRE")
    {
        serde_json::from_slice(&std::fs::read(path).expect("actual C# OnFrame export"))
            .expect("C# serialized production progress events")
    } else {
        phases.iter().map(|phase| {
            serde_json::from_value(serde_json::json!({
                "version":1,"operation_id":id,"session_id":id,"generation":1,"kind":"progress",
                "progress":{"phase":phase,"bytes_done":3,"bytes_total":10,"files_done":1,"files_total":2}
            })).expect("public progress schema")
        }).collect()
    };
    assert_eq!(events.len(), phases.len(), "six production phases required");
    let mut seen = std::collections::HashSet::new();
    for mut event in events {
        event
            .validate()
            .expect("actual serialized progress contract");
        let progress = event.progress.clone().expect("progress frame");
        assert!(phases.contains(&progress.phase.as_str()));
        assert!(seen.insert(progress.phase.clone()), "one frame per phase");
        // Only identity is rebound to this live test record; C# progress stays unchanged.
        event.operation_id = id;
        event.session_id = id;
        event.generation = 1;
        deliver(&owner, &endpoint, event, cx);
        object_draw(&mut surface);
        assert_eq!(
            owner.read_with(cx, |app, _| app.jobs.records[&id].progress.clone()),
            Some(progress.clone())
        );
        assert!(
            object_labels(&mut surface)
                .iter()
                .any(|label| label.contains("3 B")),
            "accepted progress must reach production view"
        );
        let stage = match progress.phase.as_str() {
            "edit_planning" => "准备 7z 编辑；容量需求未知",
            "edit_reading" => "读取并检查原压缩包",
            "edit_reencoding" => "完整解码并重编码为非固实 LZMA2；处理量不是保存进度",
            "edit_verifying" => "校验完整候选；尚未保存",
            "edit_committing" => "等待原生提交；尚未确认保存",
            _ => "正在修改压缩包",
        };
        assert!(
            object_labels(&mut surface)
                .iter()
                .any(|label| label == stage),
            "phase must reach original stage label"
        );
    }
    let last_phase = owner.read_with(cx, |app, _| {
        app.jobs.records[&id]
            .progress
            .as_ref()
            .expect("last progress")
            .phase
            .clone()
    });
    for phase in ["edit_unknown", "", "RUNNING"] {
        let mut event = terminal(id, true);
        event.kind = ArchiveJobEventKind::Progress;
        event.terminal = None;
        event.progress = Some(serde_json::from_value(serde_json::json!({"phase":phase,"bytes_done":99,"bytes_total":null,"files_done":0,"files_total":null})).expect("malformed phase fixture"));
        deliver(&owner, &endpoint, event, cx);
        assert_eq!(
            owner.read_with(cx, |app, _| app.jobs.records[&id]
                .progress
                .as_ref()
                .map(|p| p.phase.clone())),
            Some(last_phase.clone())
        );
    }
}

#[gpui_kit::test]
fn opt01_background_prompt_recovers_once_after_notice_expiry(cx: &mut gpui_kit::TestAppContext) {
    for (index, kind) in [
        "password",
        "conflict",
        "risk",
        "destination",
        "create_overwrite",
    ]
    .into_iter()
    .enumerate()
    {
        let (owner, endpoint) = super::super::tests::setup(cx);
        let id = owner.update(cx, |app, cx| {
            app.submit_job(request(1100 + index), completion(), cx)
                .expect("job")
        });
        cx.run_until_parked();
        owner.update(cx, |app, cx| app.background_job(id, cx));
        cx.run_until_parked();
        cx.executor()
            .advance_clock(std::time::Duration::from_secs(7));
        owner.read_with(cx, |app, _| {
            let record = &app.jobs.records[&id];
            assert!(
                record.terminal.is_none(),
                "{kind}: original member still active: {}, terminal code: {:?}",
                record.state,
                record.terminal.as_ref().and_then(|t| t.code.as_deref())
            );
            assert!(
                record.connection.is_some(),
                "{kind}: current native connection retained"
            );
        });
        let mut prompt = object_prompt(id, kind, Some("incoming.txt"), r"C:\owned\target");
        if !matches!(
            prompt.kind,
            ArchiveJobPromptKind::Conflict | ArchiveJobPromptKind::CreateOverwrite
        ) {
            prompt.display = None;
        }
        if prompt.kind == ArchiveJobPromptKind::CreateOverwrite {
            prompt.display.as_mut().expect("display").incoming_member = None;
        }
        let mut event = terminal(id, true);
        event.kind = ArchiveJobEventKind::Prompt;
        event.terminal = None;
        event.prompt = Some(prompt.clone());
        event.validate().expect("supported current prompt frame");
        let decoded = serde_json::from_slice(&serde_json::to_vec(&event).expect("event encode"))
            .expect("event decode");
        deliver(&owner, &endpoint, decoded, cx);
        cx.run_until_parked();
        owner.read_with(cx, |app, _| {
            let record = &app.jobs.records[&id];
            assert_eq!(
                record.prompt.as_ref(),
                Some(&prompt),
                "{kind}: original prompt installed before restoration"
            );
        });
        let window = owner
            .read_with(cx, |app, _| {
                app.jobs.records[&id].window.as_ref().map(|slot| slot.0)
            })
            .expect("current background decision must restore original progress surface");
        let mut surface = gpui_kit::VisualTestContext::from_window(window, cx);
        object_draw(&mut surface);
        assert!(
            object_labels(&mut surface)
                .iter()
                .any(|label| label == "取消任务")
        );
        deliver(&owner, &endpoint, event.clone(), cx);
        cx.run_until_parked();
        assert_eq!(
            owner.read_with(cx, |app, _| app.jobs.records[&id]
                .window
                .as_ref()
                .map(|slot| slot.0)),
            Some(window)
        );
        assert_eq!(
            owner.read_with(cx, |app, _| app.jobs.records[&id].prompt.clone()),
            Some(prompt)
        );
        deliver(&owner, &endpoint, terminal(id, true), cx);
        owner.update(cx, |app, cx| app.background_job(id, cx));
        deliver(&owner, &endpoint, event, cx);
        cx.run_until_parked();
        assert!(owner.read_with(cx, |app, _| app.jobs.records[&id].window.is_none()));
    }
}

#[gpui_kit::test]
fn opt01_queued_pause_is_disabled_and_stop_only_cancels_that_member(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (owner, endpoint) = super::super::tests::setup(cx);
    let ids = owner.update(cx, |app, cx| {
        (0..3)
            .map(|i| {
                app.submit_job(request(1200 + i), completion(), cx)
                    .expect("job")
            })
            .collect::<Vec<_>>()
    });
    cx.run_until_parked();
    let window = owner.read_with(cx, |app, _| {
        app.jobs.records[&ids[2]]
            .window
            .as_ref()
            .expect("queued window")
            .0
    });
    let mut surface = gpui_kit::VisualTestContext::from_window(window, cx);
    assert_eq!(
        object_node(&mut surface, "暂停").disabled(),
        Some(true),
        "queue has no worker to pause"
    );
    fb_controls(&endpoint);
    object_click(&mut surface, "停止");
    assert!(
        fb_controls(&endpoint).is_empty(),
        "queued stop must not send native controls"
    );
    owner.read_with(cx, |app, _| {
        assert_eq!(app.jobs.records[&ids[2]].terminal.as_ref().expect("local terminal").outcome, ArchiveJobOutcome::Cancelled);
        assert!(ids[..2].iter().all(|id| app.jobs.records[id].terminal.is_none() && app.jobs.queue.is_running(*id)));
    });
}

#[gpui_kit::test]
fn opt01_running_stop_requires_original_dialog_confirmation(cx: &mut gpui_kit::TestAppContext) {
    let (owner, endpoint) = super::super::tests::setup(cx);
    let (id, mut surface) = fb_control_surface(&owner, &endpoint, cx);
    object_click(&mut surface, "停止");
    let controls = fb_controls(&endpoint);
    assert!(
        !controls
            .iter()
            .any(|(_, action)| *action == ArchiveJobAction::Cancel),
        "Stop must ask before Cancel"
    );
    assert_eq!(
        controls
            .iter()
            .filter(|(_, action)| *action == ArchiveJobAction::Pause)
            .count(),
        1,
        "running confirmation requests one temporary pause"
    );
    assert!(
        object_labels(&mut surface)
            .iter()
            .any(|label| label == "确认停止")
    );
    assert!(owner.read_with(cx, |app, _| app.jobs.queue.is_running(id)));
}

fn opt01_state(
    owner: &Entity<ArchiveApplication>,
    endpoint: &nxtools_ipc::PlatformServerEndpoint,
    id: Uuid,
    state: ArchiveJobState,
    cx: &mut gpui_kit::TestAppContext,
) {
    let mut event = terminal(id, true);
    event.kind = ArchiveJobEventKind::State;
    event.terminal = None;
    event.state = Some(state);
    let event = serde_json::from_slice(&serde_json::to_vec(&event).expect("state encode"))
        .expect("state decode");
    deliver(owner, endpoint, event, cx);
    cx.run_until_parked();
}

#[gpui_kit::test]
fn opt01_stop_waits_real_paused_and_decline_restores_only_acquired_pause(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (owner, endpoint) = super::super::tests::setup(cx);
    let (id, mut surface) = fb_control_surface(&owner, &endpoint, cx);
    object_click(&mut surface, "停止");
    let controls = fb_controls(&endpoint);
    let receipt = controls
        .iter()
        .find(|(_, action)| *action == ArchiveJobAction::Pause)
        .expect("temporary pause")
        .0;
    assert_eq!(object_node(&mut surface, "确认停止").disabled(), Some(true));
    fb_control_receipt(&owner, &endpoint, receipt, true, cx);
    assert!(!owner.read_with(cx, |app, _| app.jobs.records[&id].paused));
    assert_eq!(
        object_node(&mut surface, "确认停止").disabled(),
        Some(true),
        "accepted receipt is not paused"
    );
    object_click(&mut surface, "确认停止");
    assert!(fb_controls(&endpoint).is_empty());
    opt01_state(&owner, &endpoint, id, ArchiveJobState::Paused, cx);
    assert_ne!(object_node(&mut surface, "确认停止").disabled(), Some(true));
    object_click(&mut surface, "继续任务");
    assert_eq!(
        fb_controls(&endpoint)
            .iter()
            .map(|(_, action)| *action)
            .collect::<Vec<_>>(),
        vec![ArchiveJobAction::Resume]
    );
    opt01_state(&owner, &endpoint, id, ArchiveJobState::Running, cx);
    assert!(!surface.update(|window, cx| window.has_active_dialog(cx)));
    opt01_state(&owner, &endpoint, id, ArchiveJobState::Paused, cx);
    fb_controls(&endpoint);
    object_click(&mut surface, "停止");
    assert!(
        fb_controls(&endpoint).is_empty(),
        "already paused needs no temporary pause"
    );
    object_click(&mut surface, "继续任务");
    assert!(
        fb_controls(&endpoint).is_empty(),
        "prior pause must not be resumed"
    );
    assert!(owner.read_with(cx, |app, _| app.jobs.records[&id].paused));
}

#[gpui_kit::test]
fn opt01_stop_escape_pending_pause_and_native_refusal_keep_honest_state(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (owner, endpoint) = super::super::tests::setup(cx);
    let (id, mut surface) = fb_control_surface(&owner, &endpoint, cx);
    object_click(&mut surface, "停止");
    let receipt = fb_controls(&endpoint)
        .into_iter()
        .find(|(_, action)| *action == ArchiveJobAction::Pause)
        .expect("pause")
        .0;
    surface.simulate_keystrokes("escape");
    object_draw(&mut surface);
    assert!(
        fb_controls(&endpoint).is_empty(),
        "decline before Paused must not guess Resume"
    );
    fb_control_receipt(&owner, &endpoint, receipt, true, cx);
    opt01_state(&owner, &endpoint, id, ArchiveJobState::Paused, cx);
    assert_eq!(
        fb_controls(&endpoint)
            .iter()
            .map(|(_, action)| *action)
            .collect::<Vec<_>>(),
        vec![ArchiveJobAction::Resume]
    );
    opt01_state(&owner, &endpoint, id, ArchiveJobState::Running, cx);
    object_click(&mut surface, "停止");
    let receipt = fb_controls(&endpoint)
        .into_iter()
        .find(|(_, action)| *action == ArchiveJobAction::Pause)
        .expect("pause retry")
        .0;
    fb_control_receipt(&owner, &endpoint, receipt, false, cx);
    object_click(&mut surface, "继续任务");
    assert!(
        fb_controls(&endpoint).is_empty(),
        "refused pause grants no restoration"
    );
    assert!(!owner.read_with(cx, |app, _| app.jobs.records[&id].paused));
}

#[gpui_kit::test]
fn opt01_confirm_stop_sends_once_and_waits_original_cleanup(cx: &mut gpui_kit::TestAppContext) {
    let (owner, endpoint) = super::super::tests::setup(cx);
    let (id, mut surface) = fb_control_surface(&owner, &endpoint, cx);
    opt01_state(&owner, &endpoint, id, ArchiveJobState::Paused, cx);
    object_click(&mut surface, "停止");
    let token = owner.read_with(cx, |app, _| {
        app.jobs.records[&id]
            .stop_confirmation
            .as_ref()
            .expect("question")
            .token
    });
    object_click(&mut surface, "确认停止");
    owner.update(cx, |app, cx| app.answer_job_stop(id, token, true, cx));
    let controls = fb_controls(&endpoint);
    assert_eq!(
        controls
            .iter()
            .map(|(_, action)| *action)
            .collect::<Vec<_>>(),
        vec![ArchiveJobAction::Cancel]
    );
    assert!(owner.read_with(cx, |app, _| app.jobs.queue.is_running(id)
        && app.jobs.records[&id].terminal.is_none()));
    opt01_state(&owner, &endpoint, id, ArchiveJobState::Cancelling, cx);
    assert_eq!(object_node(&mut surface, "停止").disabled(), Some(true));
    deliver(&owner, &endpoint, terminal(id, false), cx);
    assert!(
        owner.read_with(cx, |app, _| app.jobs.queue.is_running(id)),
        "unknown cleanup keeps original reservation"
    );
}

#[gpui_kit::test]
fn opt01_verification_commit_terminal_and_old_attempt_retire_confirmation(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (owner, endpoint) = super::super::tests::setup(cx);
    let (id, mut surface) = fb_control_surface(&owner, &endpoint, cx);
    opt01_state(&owner, &endpoint, id, ArchiveJobState::Verifying, cx);
    assert_eq!(object_node(&mut surface, "暂停").disabled(), Some(true));
    owner.update(cx, |app, cx| {
        app.control_job(id, ArchiveJobAction::Pause, cx)
    });
    assert!(
        fb_controls(&endpoint).is_empty(),
        "handler also rejects invalid pause"
    );
    object_click(&mut surface, "停止");
    assert!(
        fb_controls(&endpoint).is_empty(),
        "verification cannot be temporarily paused"
    );
    let token = owner.read_with(cx, |app, _| {
        app.jobs.records[&id]
            .stop_confirmation
            .as_ref()
            .expect("question")
            .token
    });
    opt01_state(&owner, &endpoint, id, ArchiveJobState::Committing, cx);
    assert!(
        !surface.update(|window, cx| window.has_active_dialog(cx)),
        "commit transition retires old question"
    );
    owner.update(cx, |app, cx| app.answer_job_stop(id, token, true, cx));
    assert!(fb_controls(&endpoint).is_empty());
    object_click(&mut surface, "停止");
    assert!(
        fb_controls(&endpoint).is_empty(),
        "commit question never pauses"
    );
    let token = owner.read_with(cx, |app, _| {
        app.jobs.records[&id]
            .stop_confirmation
            .as_ref()
            .expect("commit question")
            .token
    });
    deliver(&owner, &endpoint, terminal(id, true), cx);
    cx.run_until_parked();
    owner.update(cx, |app, cx| app.answer_job_stop(id, token, true, cx));
    assert!(
        fb_controls(&endpoint).is_empty(),
        "terminal question cannot cancel or resume"
    );
}

#[gpui_kit::test]
fn opt01_prompt_stop_decline_preserves_original_ticket_and_password(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (owner, endpoint) = super::super::tests::setup(cx);
    let (id, _) = fb_control_surface(&owner, &endpoint, cx);
    for kind in ["password", "conflict"] {
        let mut prompt = object_prompt(id, kind, Some("owned.txt"), r"C:\owned\target");
        if kind == "password" {
            prompt.display = None;
        }
        let mut surface = show_object_prompt(&owner, &endpoint, id, prompt.clone(), cx);
        if kind == "password" {
            let bounds = object_node(&mut surface, "当前任务密码").bounds();
            surface.simulate_click(bounds.center(), gpui_kit::Modifiers::default());
            surface.simulate_input("owned-private-password");
        }
        fb_controls(&endpoint);
        object_click(&mut surface, "取消任务");
        assert!(
            fb_controls(&endpoint).is_empty(),
            "prompt wait is not a temporary pause or a reply"
        );
        assert_eq!(
            owner.read_with(cx, |app, _| app.jobs.records[&id].prompt.clone()),
            Some(prompt.clone())
        );
        object_click(&mut surface, "继续任务");
        assert!(
            object_replies(&endpoint).is_empty(),
            "decline cannot answer original ticket"
        );
        assert!(surface.update(|window, cx| window.has_active_dialog(cx)));
        object_click(
            &mut surface,
            if kind == "password" {
                "继续"
            } else {
                "跳过"
            },
        );
        let replies = object_replies(&endpoint);
        assert_eq!(replies.len(), 1);
        assert_eq!(replies[0].ticket, prompt.ticket);
        if kind == "password" {
            assert_eq!(
                replies[0].password,
                Some(nxtools_protocol::archive_password::PrivatePassword::new(
                    "owned-private-password"
                ))
            );
        }
    }
}

#[gpui_kit::test]
fn opt01_background_recovery_rechecks_terminal_disconnect_and_generation(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (owner, endpoint) = super::super::tests::setup(cx);
    for (index, retire) in ["terminal", "disconnect", "generation"]
        .into_iter()
        .enumerate()
    {
        let id = owner.update(cx, |app, cx| {
            app.submit_job(request(1300 + index), completion(), cx)
                .expect("job")
        });
        cx.run_until_parked();
        owner.update(cx, |app, cx| app.background_job(id, cx));
        cx.run_until_parked();
        let mut event = terminal(id, true);
        event.kind = ArchiveJobEventKind::Prompt;
        event.terminal = None;
        event.prompt = Some(object_prompt(
            id,
            "conflict",
            Some("owned.txt"),
            r"C:\owned\target",
        ));
        owner.update(cx, |app, cx| {
            app.job_notification(
                &PlatformNotification::Event(PlatformEvent::ArchiveJob(Box::new(event.clone()))),
                cx,
            );
            match retire {
                "terminal" => {
                    app.job_notification(
                        &PlatformNotification::Event(PlatformEvent::ArchiveJob(Box::new(
                            terminal(id, true),
                        ))),
                        cx,
                    );
                }
                "disconnect" => {
                    app.job_notification(
                        &PlatformNotification::Disconnected {
                            generation: app.platform.connection_generation().expect("connection"),
                            pending_command_ids: vec![],
                        },
                        cx,
                    );
                }
                _ => {
                    app.jobs
                        .records
                        .get_mut(&id)
                        .expect("job")
                        .command
                        .generation += 1;
                }
            }
        });
        cx.run_until_parked();
        assert!(
            owner.read_with(cx, |app, _| app.jobs.records[&id].window.is_none()),
            "retired {retire} prompt reopened"
        );
        deliver(&owner, &endpoint, event, cx);
        cx.run_until_parked();
        assert!(owner.read_with(cx, |app, _| app.jobs.records[&id].window.is_none()));
        owner.update(cx, |app, cx| {
            app.job_terminal(id, local_terminal("cancelled", true), cx)
        });
    }
}

#[gpui_kit::test]
fn opt01_stop_state_before_receipt_requires_ack_ownership(cx: &mut gpui_kit::TestAppContext) {
    let (owner, endpoint) = super::super::tests::setup(cx);
    let (id, mut surface) = fb_control_surface(&owner, &endpoint, cx);
    object_click(&mut surface, "停止");
    let receipt = fb_controls(&endpoint)
        .into_iter()
        .find(|(_, action)| *action == ArchiveJobAction::Pause)
        .expect("pause receipt")
        .0;
    opt01_state(&owner, &endpoint, id, ArchiveJobState::Paused, cx);
    assert_eq!(
        object_node(&mut surface, "确认停止").disabled(),
        Some(true),
        "Paused without the matching accepted receipt cannot acquire the confirmation pause"
    );
    object_click(&mut surface, "继续任务");
    assert!(fb_controls(&endpoint).is_empty());
    fb_control_receipt(&owner, &endpoint, receipt, true, cx);
    assert_eq!(
        fb_controls(&endpoint)
            .iter()
            .map(|(_, action)| *action)
            .collect::<Vec<_>>(),
        vec![ArchiveJobAction::Resume]
    );
    opt01_state(&owner, &endpoint, id, ArchiveJobState::Running, cx);
    object_click(&mut surface, "停止");
    let receipt = fb_controls(&endpoint)
        .into_iter()
        .find(|(_, action)| *action == ArchiveJobAction::Pause)
        .expect("next pause")
        .0;
    opt01_state(&owner, &endpoint, id, ArchiveJobState::Paused, cx);
    fb_control_receipt(&owner, &endpoint, receipt, false, cx);
    object_click(&mut surface, "继续任务");
    assert!(
        fb_controls(&endpoint).is_empty(),
        "a refused request never owns the observed pause"
    );
    assert!(owner.read_with(cx, |app, _| app.jobs.records[&id].paused));
}

#[gpui_kit::test]
fn opt01_stop_keyboard_confirmation_uses_original_button_once(cx: &mut gpui_kit::TestAppContext) {
    let (owner, endpoint) = super::super::tests::setup(cx);
    let (id, mut surface) = fb_control_surface(&owner, &endpoint, cx);
    opt01_state(&owner, &endpoint, id, ArchiveJobState::Paused, cx);
    object_click(&mut surface, "停止");
    fb_controls(&endpoint);
    audit05_tab_prompt(&mut surface, "确认停止");
    audit05_prompt_key(&mut surface, "enter", true, false);
    assert!(fb_controls(&endpoint).is_empty(), "keydown cannot confirm");
    audit05_prompt_key(&mut surface, "enter", false, false);
    assert_eq!(
        fb_controls(&endpoint)
            .iter()
            .map(|(_, action)| *action)
            .collect::<Vec<_>>(),
        vec![ArchiveJobAction::Cancel]
    );
    audit05_prompt_key(&mut surface, "enter", false, false);
    assert!(
        fb_controls(&endpoint).is_empty(),
        "unarmed release cannot repeat Cancel"
    );
}

#[gpui_kit::test]
fn opt01_new_prompt_retires_stop_question_without_replacing_ticket(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (owner, endpoint) = super::super::tests::setup(cx);
    let (id, mut surface) = fb_control_surface(&owner, &endpoint, cx);
    object_click(&mut surface, "停止");
    let receipt = fb_controls(&endpoint)
        .into_iter()
        .find(|(_, action)| *action == ArchiveJobAction::Pause)
        .expect("pause")
        .0;
    let token = owner.read_with(cx, |app, _| {
        app.jobs.records[&id]
            .stop_confirmation
            .as_ref()
            .expect("question")
            .token
    });
    let mut prompt = object_prompt(id, "password", None, r"C:\owned\target");
    prompt.display = None;
    let mut surface = show_object_prompt(&owner, &endpoint, id, prompt.clone(), cx);
    assert!(
        !object_labels(&mut surface)
            .iter()
            .any(|label| label == "确认停止"),
        "a newer worker decision owns the foreground Dialog"
    );
    owner.update(cx, |app, cx| app.answer_job_stop(id, token, true, cx));
    assert!(
        fb_controls(&endpoint).is_empty(),
        "old stop token cannot cancel a new decision"
    );
    fb_control_receipt(&owner, &endpoint, receipt, true, cx);
    opt01_state(&owner, &endpoint, id, ArchiveJobState::Paused, cx);
    assert_eq!(
        fb_controls(&endpoint)
            .iter()
            .map(|(_, action)| *action)
            .collect::<Vec<_>>(),
        vec![ArchiveJobAction::Resume]
    );
    assert_eq!(
        owner.read_with(cx, |app, _| app.jobs.records[&id].prompt.clone()),
        Some(prompt)
    );
    assert!(object_replies(&endpoint).is_empty());
    assert!(
        object_labels(&mut surface)
            .iter()
            .any(|label| label == "当前任务密码")
    );
}

#[gpui_kit::test]
fn opt01_old_attempt_stop_answer_and_pause_receipt_cannot_control_replacement(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (owner, endpoint) = super::super::tests::setup(cx);
    let (id, mut surface) = fb_control_surface(&owner, &endpoint, cx);
    object_click(&mut surface, "停止");
    let receipt = fb_controls(&endpoint)
        .into_iter()
        .find(|(_, action)| *action == ArchiveJobAction::Pause)
        .expect("pause")
        .0;
    let token = owner.read_with(cx, |app, _| {
        app.jobs.records[&id]
            .stop_confirmation
            .as_ref()
            .expect("question")
            .token
    });
    owner.update(cx, |app, cx| {
        let mut command = app.jobs.records[&id].command.clone();
        command.operation_id = Uuid::new_v4();
        app.send_job_attempt(id, command, cx);
        app.refresh_job(id, cx);
    });
    object_draw(&mut surface);
    assert!(!surface.update(|window, cx| window.has_active_dialog(cx)));
    assert_eq!(
        fb_controls(&endpoint)
            .iter()
            .map(|(_, action)| *action)
            .collect::<Vec<_>>(),
        vec![ArchiveJobAction::Start]
    );
    fb_control_receipt(&owner, &endpoint, receipt, true, cx);
    opt01_state(&owner, &endpoint, id, ArchiveJobState::Paused, cx);
    owner.update(cx, |app, cx| app.answer_job_stop(id, token, true, cx));
    assert!(fb_controls(&endpoint).is_empty());
    assert!(!owner.read_with(cx, |app, _| app.jobs.records[&id].paused));
}

#[gpui_kit::test]
fn opt01_background_during_stop_declines_and_releases_only_acknowledged_pause(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (owner, endpoint) = super::super::tests::setup(cx);
    let (id, mut surface) = fb_control_surface(&owner, &endpoint, cx);
    object_click(&mut surface, "停止");
    let receipt = fb_controls(&endpoint)
        .into_iter()
        .find(|(_, action)| *action == ArchiveJobAction::Pause)
        .expect("pause")
        .0;
    owner.update(cx, |app, cx| app.background_job(id, cx));
    cx.run_until_parked();
    fb_control_receipt(&owner, &endpoint, receipt, true, cx);
    opt01_state(&owner, &endpoint, id, ArchiveJobState::Paused, cx);
    assert_eq!(
        fb_controls(&endpoint)
            .iter()
            .map(|(_, action)| *action)
            .collect::<Vec<_>>(),
        vec![ArchiveJobAction::Resume]
    );
    assert!(owner.read_with(cx, |app, _| app.jobs.records[&id].window.is_none()));
}

#[path = "ac_repair_tests.rs"]
mod ac_repair_tests;

#[gpui_kit::test]
fn opt03_paste_button_uses_original_form_and_correlated_command(cx: &mut gpui_kit::TestAppContext) {
    let (owner, endpoint) = super::super::tests::setup(cx);
    owner.update(cx, |app, cx| app.open_create_form(vec![], cx));
    cx.run_until_parked();
    let (id, window) = owner.read_with(cx, |app, _| {
        let (id, slot) = app.jobs.forms.iter().next().expect("original Create form");
        (*id, slot.window)
    });
    let mut surface = gpui_kit::VisualTestContext::from_window(window, cx);
    object_draw(&mut surface);
    object_click(&mut surface, "粘贴文件");
    surface.run_until_parked();
    let mut reads = Vec::new();
    while let Ok(envelope) = endpoint.try_receive_command_for_contract_test() {
        let wire = serde_json::to_value(&envelope).expect("actual outgoing envelope");
        if wire.to_string().contains("read_clipboard_file_drop") {
            reads.push(wire);
        }
        assert!(!matches!(
            envelope.message,
            Message::PlatformCommand(PlatformCommand::ArchiveJob(_))
        ));
    }
    assert_eq!(
        reads.len(),
        1,
        "one explicit clipboard read, never job admission"
    );
    assert!(
        reads[0].to_string().contains(&id.to_string()),
        "original form session"
    );
}

#[path = "intake_tests.rs"]
mod intake_tests;

#[path = "opt03_result_tests.rs"]
mod opt03_result_tests;

#[path = "create_output_plan_tests.rs"]
mod create_output_plan_tests;

#[path = "opt07_batch_tests.rs"]
mod opt07_batch_tests;

#[test]
fn incoming_add_feedback_preserves_native_publication_facts() {
    let mut request = request(0);
    request.kind = ArchiveJobKind::Modify;
    let mut facts: ArchiveJobTerminal = serde_json::from_value(serde_json::json!({
        "outcome":"failed", "code":"unsafe_path", "files_committed":0, "bytes_committed":0,
        "files_verified":0, "bytes_verified":0, "files_skipped":0, "cleanup_complete":true,
        "commit_state":null, "elapsed_ms":1, "native_error_code":null, "journal":[]
    }))
    .expect("terminal");
    assert!(
        !job_failure_message(Some(&request), &facts).contains("未被替换"),
        "zero count is not unpublished"
    );
    facts.output_published = Some(false);
    assert!(job_failure_message(Some(&request), &facts).contains("原压缩包未被替换"));
    facts.output_published = Some(true);
    assert!(
        job_failure_message(Some(&request), &facts).contains("已有输出发布"),
        "publication despite zero committed files"
    );
    facts.commit_state = Some("CommitOutcomeUnknown".into());
    assert!(job_failure_message(Some(&request), &facts).contains("写入结果尚未确认"));
    facts.commit_state = None;
    facts.output_published = None;
    facts.code = Some("io_failure".into());
    facts.native_error_code = Some(32);
    assert!(job_failure_message(Some(&request), &facts).contains("占用"));
    facts.native_error_code = None;
    facts.code = Some("authentication_or_data_error".into());
    assert!(job_failure_message(Some(&request), &facts).contains("密码验证或加密数据校验失败"));
    assert!(!job_failure_message(Some(&request), &facts).contains("密码不正确"));
}

#[gpui_kit::test]
fn ref17_create_remember_keeps_concurrent_untouched_create_leaves(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (owner, _endpoint) = super::super::tests::setup(cx);
    let temp = tempfile::tempdir().expect("isolated create preferences");
    let path = temp.path().join("config.json");
    nxtools_core::AppConfig::default()
        .save_to(&path)
        .expect("seed");
    let baseline = nxtools_core::archive::ArchiveSettings::default();
    nxtools_core::AppConfig::update_section(&path, |config| {
        config.archive.test_after_create = false;
        config.archive.exclude_patterns = vec!["concurrent.tmp".into()];
        config.archive.show_tree = false;
    })
    .expect("concurrent preference save");
    let mut desired = baseline.clone();
    desired.keep_create_progress_open = true;
    owner.update(cx, |app, cx| {
        app.config_path = Some(path.clone());
        app.remember_accepted(desired, ArchiveJobKind::Create, Some(baseline), cx);
    });
    cx.run_until_parked();
    let saved = nxtools_core::AppConfig::load_from(&path)
        .expect("durable reopen")
        .archive;
    assert!(
        !saved.test_after_create,
        "untouched stale Test preference must not overwrite disk latest"
    );
    assert_eq!(saved.exclude_patterns, ["concurrent.tmp"]);
    assert!(!saved.show_tree && saved.keep_create_progress_open);
}

#[path = "jobs_ref20_tests.rs"]
mod ref20;

const R3_HWND: i64 = 0xC001;

fn r3_activations(endpoint: &nxtools_ipc::PlatformServerEndpoint) -> Vec<Uuid> {
    let mut requests = Vec::new();
    while let Ok(envelope) = endpoint.try_receive_command_for_contract_test() {
        if let Message::PlatformCommand(PlatformCommand::ActivateWindow(command)) = envelope.message
        {
            assert_eq!(command.window_handle, R3_HWND);
            assert!(endpoint.command_pending_for_contract_test(envelope.id));
            requests.push(envelope.id);
        }
    }
    requests
}

fn r3_result(
    id: Uuid,
    error: Option<&str>,
    native_error_code: Option<i32>,
) -> PlatformNotification {
    PlatformNotification::CommandResult {
        correlation_id: id,
        result: nxtools_protocol::PlatformCommandResult {
            success: error.is_none(),
            error: error.map(str::to_owned),
            native_error_code,
            data: None,
        },
    }
}

fn r3_receipt(
    owner: &Entity<ArchiveApplication>,
    endpoint: &nxtools_ipc::PlatformServerEndpoint,
    reply: PlatformNotification,
    cx: &mut gpui_kit::TestAppContext,
) {
    endpoint
        .send_notification_for_contract_test(reply.clone())
        .expect("original IPC result");
    owner.update(cx, |app, cx| app.notification(reply, cx));
}

#[gpui_kit::test]
fn r3_archive_create_show_reuse_and_visible_terminal_request_native_foreground(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (owner, endpoint) = super::super::tests::setup(cx);
    let id = owner.update(cx, |app, cx| {
        app.jobs.activation_test_hwnd = Some(R3_HWND);
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
            test_after_create: false,
            name: "created.zip".into(),
            volume_bytes: None,
            self_extracting: false,
        });
        let id = app.submit_job(input, completion(), cx).expect("create");
        assert!(r3_activations(&endpoint).is_empty(), "must defer dispatch");
        id
    });
    cx.run_until_parked();
    let [first] = r3_activations(&endpoint)[..] else {
        panic!("new progress must request one native activation");
    };
    let window = owner.read_with(cx, |app, _| {
        app.jobs.records[&id].window.clone().expect("shown").0
    });
    assert!(owner.read_with(cx, |app, _| app.jobs.activations.contains_key(&id)));
    r3_receipt(&owner, &endpoint, r3_result(first, None, None), cx);
    assert!(owner.read_with(cx, |app, _| app.jobs.activations.is_empty()));

    owner.update(cx, |app, cx| app.show_job(id, cx));
    cx.run_until_parked();
    let [reuse] = r3_activations(&endpoint)[..] else {
        panic!("reuse must request one native activation");
    };
    assert_ne!(first, reuse);
    r3_receipt(
        &owner,
        &endpoint,
        r3_result(reuse, Some("native refusal"), Some(5)),
        cx,
    );
    assert!(owner.read_with(cx, |app, _| app.jobs.activations.is_empty()));

    let mut tick = terminal(id, true);
    tick.kind = ArchiveJobEventKind::Progress;
    tick.terminal = None;
    tick.progress = Some(ArchiveJobProgress {
        phase: "running".into(),
        bytes_done: 1,
        bytes_total: Some(3),
        files_done: 0,
        files_total: None,
        current_member: None,
        current_bytes_done: None,
        current_bytes_total: None,
        sequence: Some(1),
    });
    deliver(&owner, &endpoint, tick, cx);
    cx.run_until_parked();
    assert!(r3_activations(&endpoint).is_empty());

    let done = terminal(id, true);
    deliver(&owner, &endpoint, done.clone(), cx);
    deliver(&owner, &endpoint, done, cx);
    cx.run_until_parked();
    let [finished] = r3_activations(&endpoint)[..] else {
        panic!("retained completion must request native foreground exactly once");
    };
    assert_ne!(finished, reuse);
    r3_receipt(&owner, &endpoint, r3_result(finished, None, None), cx);
    owner.read_with(cx, |app, _| {
        assert!(app.jobs.activations.is_empty());
        let record = &app.jobs.records[&id];
        assert_eq!(record.window.as_ref().expect("same window").0, window);
        assert_eq!(
            record
                .terminal
                .as_ref()
                .expect("real result")
                .files_committed,
            1
        );
    });
}

#[gpui_kit::test]
fn r3_archive_batch_foregrounds_only_its_retained_completion_and_ignores_retired_receipt(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (owner, endpoint) = super::super::tests::setup(cx);
    let ids = owner.update(cx, |app, cx| {
        app.jobs.activation_test_hwnd = Some(R3_HWND);
        app.submit_batch(vec![request(1), request(2)], cx)
            .expect("batch")
    });
    cx.run_until_parked();
    assert_eq!(r3_activations(&endpoint).len(), 1, "one batch show");
    let window = owner.read_with(cx, |app, _| {
        assert!(app.jobs.records[&ids[1]].window.is_none());
        app.jobs.records[&ids[0]]
            .window
            .as_ref()
            .expect("batch window")
            .0
    });
    deliver(&owner, &endpoint, terminal(ids[0], true), cx);
    cx.run_until_parked();
    assert!(
        r3_activations(&endpoint).is_empty(),
        "not the batch terminal yet"
    );
    deliver(&owner, &endpoint, terminal(ids[1], true), cx);
    cx.run_until_parked();
    let [finished] = r3_activations(&endpoint)[..] else {
        panic!("final sibling must foreground the same batch surface");
    };
    owner.update(cx, |app, cx| {
        assert_eq!(
            app.jobs.records[&ids[0]]
                .window
                .as_ref()
                .expect("batch window")
                .0,
            window
        );
        app.background_job(ids[1], cx);
        assert!(app.jobs.activations.is_empty());
    });
    cx.run_until_parked();
    assert!(r3_activations(&endpoint).is_empty());
    assert!(window.update(cx, |_, _, _| ()).is_err());

    owner.update(cx, |app, cx| app.show_job(ids[1], cx));
    cx.run_until_parked();
    let [reopened] = r3_activations(&endpoint)[..] else {
        panic!("only explicit reopening requests foreground again");
    };
    r3_receipt(&owner, &endpoint, r3_result(finished, None, None), cx);
    assert!(owner.read_with(cx, |app, _| app.jobs.activations.contains_key(&ids[0])));
    r3_receipt(&owner, &endpoint, r3_result(reopened, None, None), cx);
    assert!(owner.read_with(cx, |app, _| app.jobs.activations.is_empty()));
}

#[gpui_kit::test]
fn r3_archive_timeout_disconnect_background_and_auto_close_do_not_reactivate(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (owner, endpoint) = super::super::tests::setup(cx);
    let (id, generation) = owner.update(cx, |app, cx| {
        app.jobs.activation_test_hwnd = Some(R3_HWND);
        let id = app.submit_job(request(1), completion(), cx).expect("job");
        (
            id,
            app.platform.connection_generation().expect("connection"),
        )
    });
    cx.run_until_parked();
    let [first] = r3_activations(&endpoint)[..] else {
        panic!("first activation");
    };
    r3_receipt(
        &owner,
        &endpoint,
        r3_result(first, Some("platform command timed out"), None),
        cx,
    );
    assert!(owner.read_with(cx, |app, _| app.jobs.activations.is_empty()));
    owner.update(cx, |app, cx| app.show_job(id, cx));
    cx.run_until_parked();
    let [second] = r3_activations(&endpoint)[..] else {
        panic!("explicit reuse after timeout");
    };
    endpoint.set_connected_for_contract_test(false);
    owner.update(cx, |app, cx| {
        app.notification(
            PlatformNotification::Disconnected {
                generation,
                pending_command_ids: vec![second],
            },
            cx,
        );
        assert!(app.jobs.activations.is_empty());
        app.background_job(id, cx);
    });
    endpoint.set_connected_for_contract_test(true);
    owner.update(cx, |app, cx| {
        app.notification(r3_result(second, None, None), cx)
    });
    deliver(&owner, &endpoint, terminal(id, true), cx);
    cx.run_until_parked();
    assert!(r3_activations(&endpoint).is_empty());
    assert!(owner.read_with(cx, |app, _| app.jobs.records[&id].window.is_none()));

    let background = owner.update(cx, |app, cx| {
        let id = app.submit_job(request(3), completion(), cx).expect("job");
        assert!(
            app.jobs.activations.contains_key(&id),
            "pending before dispatch"
        );
        app.background_job(id, cx);
        id
    });
    cx.run_until_parked();
    assert!(
        r3_activations(&endpoint).is_empty(),
        "retired before dispatch"
    );
    deliver(&owner, &endpoint, terminal(background, true), cx);
    cx.run_until_parked();
    assert!(
        r3_activations(&endpoint).is_empty(),
        "connected background completion"
    );
    assert!(owner.read_with(cx, |app, _| {
        let record = &app.jobs.records[&background];
        record.terminal.is_some() && record.window.is_none()
    }));

    let auto = owner.update(cx, |app, cx| {
        let mut policy = completion();
        policy.keep_open = false;
        app.submit_job(request(2), policy, cx)
            .expect("auto-closing job")
    });
    cx.run_until_parked();
    assert_eq!(r3_activations(&endpoint).len(), 1);
    let mut done = terminal(auto, true);
    let result = done.terminal.as_mut().expect("terminal");
    result.outcome = ArchiveJobOutcome::Succeeded;
    result.code = None;
    deliver(&owner, &endpoint, done, cx);
    cx.run_until_parked();
    assert!(r3_activations(&endpoint).is_empty());
    assert!(owner.read_with(cx, |app, _| {
        app.jobs.activations.is_empty() && app.jobs.records[&auto].window.is_none()
    }));
}

#[gpui_kit::test]
fn multimonitor_archive_job_reopen_uses_its_retained_owner_work_area(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (owner, _endpoint) = super::super::tests::setup(cx);
    let id = owner.update(cx, |app, cx| {
        app.submit_job(request(0), completion(), cx).expect("task")
    });
    owner.update(cx, |app, cx| {
        app.background_job(id, cx);
        let placement = crate::ui::monitor_placement::MonitorPlacement::from_snapshot(
            nxtools_protocol::CursorMonitorSnapshot {
                display_id: 0xCAFE,
                work_left: -1920,
                work_top: -1080,
                work_width: 1920,
                work_height: 1040,
                dpi: 120,
            },
            cx,
        );
        app.jobs
            .records
            .get_mut(&id)
            .expect("task record")
            .placement = placement;
        app.show_job(id, cx);
        let window = app.jobs.records[&id]
            .window
            .as_ref()
            .expect("progress surface")
            .0;
        let actual = window
            .update(cx, |_, window, _| window.bounds())
            .expect("progress bounds");
        assert_eq!(actual, placement.centered(size(px(560.0), px(260.0))));
    });
}
