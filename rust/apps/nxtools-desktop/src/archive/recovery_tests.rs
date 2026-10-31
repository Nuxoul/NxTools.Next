//! Original application/read transport and object-scoped continuation, without native effects.
use super::*;
use gpui_kit::{TestAppContext, VisualTestContext};
use nxtools_protocol::archive_recovery::{
    ArchiveRecoveryInspectCommand, ArchiveRecoveryInspection,
};
use nxtools_protocol::{Message, PlatformCommand, PlatformCommandResult};

fn open(
    cx: &mut TestAppContext,
    depth: u32,
) -> (
    Entity<ArchiveApplication>,
    nxtools_ipc::PlatformServerEndpoint,
    ArchiveReadCommand,
) {
    let (owner, endpoint) = super::tests::setup(cx);
    owner.update(cx, |app, cx| {
        app.open_canonical(r"C:\owned\source.zip".into(), depth, vec![], cx)
    });
    cx.run_until_parked();
    let wire = endpoint
        .try_receive_command_for_contract_test()
        .expect("original read");
    let Message::PlatformCommand(PlatformCommand::ArchiveBrowserRead(command)) = wire.message
    else {
        panic!("read command")
    };
    (owner, endpoint, command)
}
fn read_terminal(
    owner: &Entity<ArchiveApplication>,
    endpoint: &nxtools_ipc::PlatformServerEndpoint,
    command: &ArchiveReadCommand,
    status: ArchiveReadStatus,
    cleanup: bool,
    cx: &mut TestAppContext,
) {
    let event = ArchiveReadEvent {
        version: 2,
        operation_id: command.operation_id,
        session_id: command.session_id,
        generation: command.generation,
        kind: ArchiveReadEventKind::Terminal,
        capabilities: vec![],
        batch: None,
        terminal: Some(ArchiveReadTerminal {
            status,
            code: Some(ArchiveReadError::IoFailure),
            files_verified: 0,
            bytes_verified: 0,
            entry_count: 0,
            batch_count: 0,
            elapsed_ms: 1,
            native_error_code: None,
            cleanup_complete: cleanup,
            transfer: None,
            browser: None,
        }),
    };
    endpoint
        .send_notification_for_contract_test(PlatformNotification::Event(
            nxtools_protocol::PlatformEvent::ArchiveRead(event),
        ))
        .expect("read terminal");
    cx.run_until_parked();
    owner.update(cx, |app, cx| app.consume(cx));
    cx.run_until_parked();
}
fn inspect(endpoint: &nxtools_ipc::PlatformServerEndpoint) -> ArchiveRecoveryInspectCommand {
    while let Ok(wire) = endpoint.try_receive_command_for_contract_test() {
        if let Message::PlatformCommand(PlatformCommand::ArchiveRecoveryInspect(command)) =
            wire.message
        {
            assert_eq!(wire.correlation_id, Some(command.operation_id));
            return command;
        }
    }
    panic!("source-scoped read-only inspection");
}
fn found(command: &ArchiveRecoveryInspectCommand, completed: bool) -> ArchiveRecoveryInspection {
    ArchiveRecoveryInspection {
        version: 1,
        operation_id: command.operation_id,
        session_id: command.session_id,
        generation: command.generation,
        recovery_id: Uuid::new_v4(),
        source_identity: "v1:1:00112233445566778899AABBCCDDEEFF:100".into(),
        destination: Some(r"C:\owned".into()),
        copy_motw: false,
        completed,
    }
}
fn reply(
    owner: &Entity<ArchiveApplication>,
    command: &ArchiveRecoveryInspectCommand,
    value: &ArchiveRecoveryInspection,
    cx: &mut TestAppContext,
) {
    owner.update(cx, |app, cx| {
        app.notification(
            PlatformNotification::CommandResult {
                correlation_id: command.operation_id,
                result: PlatformCommandResult {
                    success: true,
                    error: None,
                    native_error_code: None,
                    data: Some(serde_json::to_string(value).expect("inspection wire")),
                },
            },
            cx,
        )
    });
    cx.run_until_parked();
}
#[gpui_kit::test]
fn recovery_failed_top_level_read_inspects_once_and_never_submits(cx: &mut TestAppContext) {
    let (owner, endpoint, read) = open(cx, 0);
    read_terminal(
        &owner,
        &endpoint,
        &read,
        ArchiveReadStatus::Failed,
        true,
        cx,
    );
    let command = inspect(&endpoint);
    assert_eq!(command.source, r"C:\owned\source.zip");
    assert_eq!(command.session_id, read.session_id);
    assert_eq!(command.generation, read.generation);
    assert!(command.destination.is_none());
    let value = found(&command, false);
    reply(&owner, &command, &value, cx);
    owner.update(cx, |app, cx| app.consume(cx));
    cx.run_until_parked();
    assert!(owner.read_with(cx, |app, _| app.jobs.forms.is_empty()
        && app.jobs.records.is_empty()
        && app.jobs.recoveries.is_empty()));
    let (view, window) = owner.read_with(cx, |app, _| {
        let slot = &app.windows[&read.session_id];
        (slot.view.clone(), slot.window)
    });
    assert!(
        view.read_with(cx, |view, _| view.recovery.as_ref() == Some(&value))
            .expect("live browser")
    );
    let mut surface = VisualTestContext::from_window(window, cx);
    surface.update(|window, cx| {
        window.activate_window();
        window.draw(cx).clear(cx);
    });
    assert!(
        surface
            .debug_bounds("archive-browser-reclamation-continue")
            .is_some()
    );
    assert!(
        !std::iter::from_fn(|| endpoint.try_receive_command_for_contract_test().ok()).any(
            |wire| matches!(
                wire.message,
                Message::PlatformCommand(
                    PlatformCommand::ArchiveJob(_) | PlatformCommand::ArchiveRecoveryInspect(_)
                )
            )
        )
    );
}
#[gpui_kit::test]
fn recovery_cancelled_nested_or_unclean_reads_do_not_probe(cx: &mut TestAppContext) {
    for (depth, status, cleanup) in [
        (0, ArchiveReadStatus::Cancelled, true),
        (1, ArchiveReadStatus::Failed, true),
        (0, ArchiveReadStatus::Failed, false),
    ] {
        let (owner, endpoint, read) = open(cx, depth);
        read_terminal(&owner, &endpoint, &read, status, cleanup, cx);
        assert!(owner.read_with(cx, |app, _| app.jobs.recoveries.is_empty()));
        assert!(
            !std::iter::from_fn(|| endpoint.try_receive_command_for_contract_test().ok()).any(
                |wire| matches!(
                    wire.message,
                    Message::PlatformCommand(PlatformCommand::ArchiveRecoveryInspect(_))
                )
            )
        );
    }
}
#[gpui_kit::test]
fn recovery_completed_record_has_no_continue_and_missing_record_preserves_read_error(
    cx: &mut TestAppContext,
) {
    let (owner, endpoint, read) = open(cx, 0);
    read_terminal(
        &owner,
        &endpoint,
        &read,
        ArchiveReadStatus::Failed,
        true,
        cx,
    );
    let command = inspect(&endpoint);
    reply(&owner, &command, &found(&command, true), cx);
    let window = owner.read_with(cx, |app, _| app.windows[&read.session_id].window);
    let mut surface = VisualTestContext::from_window(window, cx);
    surface.update(|window, cx| {
        window.activate_window();
        window.draw(cx).clear(cx);
    });
    assert!(
        surface
            .debug_bounds("archive-browser-reclamation-continue")
            .is_none()
    );
    owner.update(cx, |app, cx| app.refresh_browser(read.session_id, 0, cx));
    cx.run_until_parked();
    let next = std::iter::from_fn(|| endpoint.try_receive_command_for_contract_test().ok())
        .find_map(|wire| match wire.message {
            Message::PlatformCommand(PlatformCommand::ArchiveBrowserRead(read)) => Some(read),
            _ => None,
        })
        .expect("refresh read");
    read_terminal(
        &owner,
        &endpoint,
        &next,
        ArchiveReadStatus::Failed,
        true,
        cx,
    );
    let next = inspect(&endpoint);
    owner.update(cx, |app, cx| {
        app.notification(
            PlatformNotification::CommandResult {
                correlation_id: next.operation_id,
                result: PlatformCommandResult {
                    success: false,
                    error: Some("recovery_not_found".into()),
                    native_error_code: None,
                    data: None,
                },
            },
            cx,
        )
    });
    cx.run_until_parked();
    assert!(owner.read_with(cx, |app, cx| {
        app.windows[&read.session_id]
            .view
            .read_with(cx, |view, _| {
                view.recovery.is_none() && matches!(view.status, BrowserStatus::Failed(_))
            })
            .unwrap_or(false)
    }));
}
#[gpui_kit::test]
fn recovery_refresh_close_and_disconnect_retire_old_receipts(cx: &mut TestAppContext) {
    for action in ["refresh", "close", "disconnect"] {
        let (owner, endpoint, read) = open(cx, 0);
        read_terminal(
            &owner,
            &endpoint,
            &read,
            ArchiveReadStatus::Failed,
            true,
            cx,
        );
        let command = inspect(&endpoint);
        owner.update(cx, |app, cx| match action {
            "refresh" => app.refresh_browser(read.session_id, 0, cx),
            "close" => app.close_browser(read.session_id, cx),
            _ => app.notification(
                PlatformNotification::Disconnected {
                    generation: app.platform.connection_generation().expect("connection"),
                    pending_command_ids: vec![],
                },
                cx,
            ),
        });
        reply(&owner, &command, &found(&command, false), cx);
        assert!(owner.read_with(cx, |app, cx| app.jobs.recoveries.is_empty()
            && app.jobs.forms.is_empty()
            && app.windows.values().all(|slot| {
                slot.view
                    .read_with(cx, |view, _| view.recovery.is_none())
                    .unwrap_or(true)
            })));
    }
}

#[gpui_kit::test]
fn recovery_task_entry_requires_started_attempt_and_actual_cleanup(cx: &mut TestAppContext) {
    use nxtools_protocol::archive_job::*;
    for (reclamation, started, cleanup) in [
        (true, false, true),
        (true, true, false),
        (false, true, true),
        (true, true, true),
    ] {
        let (owner, endpoint) = super::tests::setup(cx);
        let settings = nxtools_core::archive::ArchiveSettings::default();
        let mut draft = crate::archive::draft::ExtractDraft::continue_reclamation(
            &settings,
            r"C:\owned\source.zip".into(),
        );
        draft.continue_reclamation = false;
        draft.reclamation = reclamation;
        draft.recovery_id = Uuid::new_v4().to_string();
        draft.destination = r"C:\owned".into();
        let request = draft
            .request(&settings)
            .expect("original extraction request");
        let id = owner.update(cx, |app, cx| {
            app.submit_job(
                request,
                crate::archive::forms::FormCompletion {
                    keep_open: true,
                    ..Default::default()
                },
                cx,
            )
            .expect("original job")
        });
        if started {
            owner.update(cx, |app, cx| {
                app.job_notification(
                    &PlatformNotification::Event(nxtools_protocol::PlatformEvent::ArchiveJob(
                        Box::new(ArchiveJobEvent {
                            version: 1,
                            operation_id: id,
                            session_id: id,
                            generation: 1,
                            kind: ArchiveJobEventKind::Started,
                            state: None,
                            progress: None,
                            prompt: None,
                            terminal: None,
                        }),
                    )),
                    cx,
                );
            });
        }
        owner.update(cx, |app, cx| {
            let terminal = ArchiveJobTerminal {
                outcome: ArchiveJobOutcome::Cancelled,
                ..super::jobs::local_terminal("cancelled", cleanup)
            };
            app.job_notification(
                &PlatformNotification::Event(nxtools_protocol::PlatformEvent::ArchiveJob(
                    Box::new(ArchiveJobEvent {
                        version: 1,
                        operation_id: id,
                        session_id: id,
                        generation: 1,
                        kind: ArchiveJobEventKind::Terminal,
                        state: None,
                        progress: None,
                        prompt: None,
                        terminal: Some(terminal),
                    }),
                )),
                cx,
            );
            app.continue_reclamation_job(id, Uuid::new_v4(), 1, cx);
            app.continue_reclamation_job(id, id, 2, cx);
        });
        cx.run_until_parked();
        assert!(
            owner.read_with(cx, |app, _| app.jobs.forms.is_empty()),
            "stale attempt/generation cannot open continuation"
        );
        owner.update(cx, |app, cx| app.continue_reclamation_job(id, id, 1, cx));
        cx.run_until_parked();
        let allowed = reclamation && started && cleanup;
        assert_eq!(
            owner.read_with(cx, |app, _| app.jobs.forms.len()),
            usize::from(allowed)
        );
        if allowed {
            let command = inspect(&endpoint);
            assert_eq!(command.source, r"C:\owned\source.zip");
            assert_eq!(command.destination.as_deref(), Some(r"C:\owned"));
            assert_ne!(
                command.session_id, id,
                "continuation uses an independent fresh form"
            );
            owner.update(cx, |app, _| app.retire_form(command.session_id));
            reply(&owner, &command, &found(&command, false), cx);
            assert!(owner.read_with(cx, |app, _| app.jobs.forms.is_empty()
                && app.jobs.recoveries.is_empty()));
        }
    }
}

#[gpui_kit::test]
fn recovery_browser_inspection_timeout_retires_without_changing_read_failure(
    cx: &mut TestAppContext,
) {
    let (owner, endpoint, read) = open(cx, 0);
    read_terminal(
        &owner,
        &endpoint,
        &read,
        ArchiveReadStatus::Failed,
        true,
        cx,
    );
    let command = inspect(&endpoint);
    cx.executor()
        .advance_clock(std::time::Duration::from_secs(8));
    cx.run_until_parked();
    assert!(owner.read_with(cx, |app, _| app.jobs.recoveries.is_empty()));
    reply(&owner, &command, &found(&command, false), cx);
    assert!(owner.read_with(cx, |app, cx| {
        app.windows[&read.session_id]
            .view
            .read_with(cx, |view, _| {
                view.recovery.is_none() && matches!(view.status, BrowserStatus::Failed(_))
            })
            .unwrap_or(false)
    }));
}

#[gpui_kit::test]
fn recovery_browser_button_rechecks_in_bound_form_and_disconnect_disables_ready_start(
    cx: &mut TestAppContext,
) {
    let (owner, endpoint, read) = open(cx, 0);
    read_terminal(
        &owner,
        &endpoint,
        &read,
        ArchiveReadStatus::Failed,
        true,
        cx,
    );
    let command = inspect(&endpoint);
    reply(&owner, &command, &found(&command, false), cx);
    let window = owner.read_with(cx, |app, _| app.windows[&read.session_id].window);
    let mut browser = VisualTestContext::from_window(window, cx);
    browser.update(|window, cx| {
        window.activate_window();
        window.draw(cx).clear(cx);
    });
    let position = browser
        .debug_bounds("archive-browser-reclamation-continue")
        .expect("source action")
        .center();
    browser.simulate_event(gpui_kit::MouseDownEvent {
        position,
        button: gpui_kit::MouseButton::Left,
        ..Default::default()
    });
    browser.simulate_event(gpui_kit::MouseUpEvent {
        position,
        button: gpui_kit::MouseButton::Left,
        modifiers: Default::default(),
        click_count: 1,
    });
    browser.run_until_parked();
    let inspection = inspect(&endpoint);
    assert_eq!(inspection.source, command.source);
    assert_eq!(inspection.destination.as_deref(), Some(r"C:\owned"));
    assert_ne!(inspection.session_id, read.session_id);
    assert!(owner.read_with(cx, |app, _| app.jobs.records.is_empty()));
    reply(&owner, &inspection, &found(&inspection, false), cx);
    let window = owner.read_with(cx, |app, _| app.jobs.forms[&inspection.session_id].window);
    let mut form = VisualTestContext::from_window(window, cx);
    let button_disabled = |surface: &mut VisualTestContext| {
        surface.update(|window, cx| {
            window.activate_window();
            window.draw(cx).clear(cx);
            gpui_kit::base::test_support::snapshots(window)
                .into_iter()
                .find(|node| {
                    node.role() == Some(gpui_kit::Role::Button)
                        && node.label() == Some("继续空间回收解压")
                })
                .and_then(|node| node.disabled())
        })
    };
    assert_ne!(
        button_disabled(&mut form),
        Some(true),
        "matching inspection enables explicit Start only"
    );
    owner.update(cx, |app, cx| {
        app.notification(
            PlatformNotification::Disconnected {
                generation: app.platform.connection_generation().expect("connection"),
                pending_command_ids: vec![],
            },
            cx,
        )
    });
    cx.run_until_parked();
    assert_eq!(
        button_disabled(&mut form),
        Some(true),
        "disconnected recovery facts cannot authorize Start"
    );
    reply(&owner, &inspection, &found(&inspection, false), cx);
    assert_eq!(
        button_disabled(&mut form),
        Some(true),
        "late receipt cannot revive ready state"
    );
    assert!(owner.read_with(cx, |app, _| app.jobs.records.is_empty()));
}
