//! OPT-07 original application/IPC/window callbacks; no native or product GUI effects.
use super::*;

#[gpui_kit::test]
fn opt07_batch_submission_and_details_keep_one_original_window(cx: &mut gpui_kit::TestAppContext) {
    let (owner, endpoint) = super::super::super::tests::setup(cx);
    let ids = owner.update(cx, |app, cx| {
        app.submit_batch(vec![request(7001), request(7002), request(7003)], cx)
            .expect("batch")
    });
    cx.run_until_parked();
    let windows = owner.read_with(cx, |app, _| {
        app.jobs
            .records
            .values()
            .filter(|r| r.window.is_some())
            .count()
    });
    assert_eq!(
        windows, 1,
        "a batch auto-opens one original progress surface, not every member"
    );
    let original = owner.read_with(cx, |app, _| {
        app.jobs.records[&ids[0]]
            .window
            .as_ref()
            .expect("original window")
            .0
    });
    owner.update(cx, |app, cx| app.show_job(ids[1], cx));
    cx.run_until_parked();
    owner.read_with(cx, |app, _| {
        assert_eq!(
            app.jobs
                .records
                .values()
                .filter(|r| r.window.is_some())
                .count(),
            1
        );
        assert_eq!(
            app.jobs.records[&ids[0]]
                .window
                .as_ref()
                .expect("same owner")
                .0,
            original
        );
        assert!(app.jobs.records[&ids[1]].terminal.is_none());
    });
    let starts = std::iter::from_fn(|| endpoint.try_receive_command_for_contract_test().ok()).filter(|wire| matches!(&wire.message, Message::PlatformCommand(PlatformCommand::ArchiveJob(c)) if c.action == ArchiveJobAction::Start)).count();
    assert_eq!(starts, 2, "original two-running admission");
}

#[gpui_kit::test]
fn opt07_standalone_initial_baseline_and_terminal_retry_use_original_callbacks(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (owner, endpoint) = super::super::super::tests::setup(cx);
    let id = owner.update(cx, |app, cx| {
        app.submit_job(request(7010), completion(), cx)
            .expect("single Test")
    });
    cx.run_until_parked();
    let observations = std::iter::from_fn(|| endpoint.try_receive_command_for_contract_test().ok())
        .filter_map(|wire| match wire.message {
            Message::PlatformCommand(PlatformCommand::ArchiveSpace(c)) => Some(c),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        observations.len(),
        1,
        "standalone initial attempt must acquire the real native baseline before terminal"
    );
    let initial = &observations[0];
    let baseline = hz3_fixture_observation(initial, "v1:single-source");
    owner.update(cx, |app, cx| {
        app.job_notification(&hz3_observation_reply(initial, baseline.clone()), cx);
    });
    let mut failed = terminal(id, true);
    failed.terminal = Some(super::super::local_terminal("io_failure", true));
    failed.terminal.as_mut().expect("terminal").output_published = Some(false);
    deliver(&owner, &endpoint, failed, cx);
    cx.run_until_parked();
    let window = owner.read_with(cx, |app, _| {
        app.jobs.records[&id]
            .window
            .as_ref()
            .expect("single window")
            .0
    });
    let mut surface = gpui_kit::VisualTestContext::from_window(window, cx);
    object_draw(&mut surface);
    object_click(&mut surface, "重试");
    surface.run_until_parked();
    object_click(&mut surface, "正在重验");
    let rechecks = std::iter::from_fn(|| endpoint.try_receive_command_for_contract_test().ok())
        .filter_map(|wire| match wire.message {
            Message::PlatformCommand(PlatformCommand::ArchiveSpace(c))
                if c.expected_source.is_some() =>
            {
                Some(c)
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        rechecks.len(),
        1,
        "pending repeated pointer action does not duplicate the recheck"
    );
    let recheck = &rechecks[0];
    owner.update(&mut surface, |app, cx| {
        app.job_notification(&hz3_observation_reply(recheck, baseline), cx);
    });
    let starts = std::iter::from_fn(|| endpoint.try_receive_command_for_contract_test().ok())
        .filter_map(|wire| match wire.message {
            Message::PlatformCommand(PlatformCommand::ArchiveJob(c))
                if c.action == ArchiveJobAction::Start =>
            {
                Some(c)
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(starts.len(), 1);
    assert_eq!(starts[0].session_id, id);
    assert_eq!(starts[0].generation, 2);
    assert_ne!(starts[0].operation_id, id);
}

#[gpui_kit::test]
fn opt07_warning_location_is_manual_native_fact_and_cleanup_gated(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (owner, endpoint) = super::super::super::tests::setup(cx);
    let id = owner.update(cx, |app, cx| {
        app.submit_job(hz3_extract_request(7020), completion(), cx)
            .expect("Extract")
    });
    let _ = hz3_wait_start(&endpoint, cx);
    let mut warning = terminal(id, true);
    let result = warning.terminal.as_mut().expect("terminal");
    result.outcome = ArchiveJobOutcome::SucceededWithWarnings;
    result.code = None;
    result.files_skipped = 2;
    result.output_published = Some(true);
    result.result_location = Some(ArchiveJobResultLocation {
        target: r"C:\owned\actual-result".into(),
        action: nxtools_protocol::ShellOpenAction::Open,
    });
    warning.validate().expect("real typed warning terminal");
    deliver(&owner, &endpoint, warning, cx);
    hz3_wait_terminal(&owner, id, cx);
    cx.run_until_parked();
    assert!(
        !std::iter::from_fn(|| endpoint.try_receive_command_for_contract_test().ok()).any(
            |wire| matches!(
                wire.message,
                Message::PlatformCommand(PlatformCommand::OpenShellTarget(_))
            )
        ),
        "warnings never auto-open"
    );
    let window = owner.read_with(cx, |app, _| {
        app.jobs.records[&id]
            .window
            .as_ref()
            .expect("original window")
            .0
    });
    let mut surface = gpui_kit::VisualTestContext::from_window(window, cx);
    object_draw(&mut surface);
    object_click(&mut surface, "定位结果");
    surface.run_until_parked();
    let targets = std::iter::from_fn(|| endpoint.try_receive_command_for_contract_test().ok())
        .filter_map(|wire| match wire.message {
            Message::PlatformCommand(PlatformCommand::OpenShellTarget(c)) => Some(c.target),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(targets, vec![r"C:\owned\actual-result"]);
}

fn opt07_prompt_event(id: Uuid, prompt: ArchiveJobPrompt) -> ArchiveJobEvent {
    let mut event = terminal(id, true);
    event.kind = ArchiveJobEventKind::Prompt;
    event.terminal = None;
    event.prompt = Some(prompt);
    event.validate().expect("current typed prompt");
    event
}

#[gpui_kit::test]
fn opt07_password_opt_in_is_unchecked_and_original_ticket_scoped(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (owner, endpoint) = super::super::super::tests::setup(cx);
    let ids = owner.update(cx, |app, cx| {
        app.submit_batch(vec![request(7030), request(7031)], cx)
            .expect("batch")
    });
    cx.run_until_parked();
    let prompt: ArchiveJobPrompt = serde_json::from_value(serde_json::json!({"ticket":{"operation_id":ids[0],"prompt_id":Uuid::new_v4(),"generation":1},"kind":"password","error_code":null,"attempt":0,"existing_bytes":null,"incoming_bytes":null,"approved_limit":null})).expect("password prompt");
    deliver(
        &owner,
        &endpoint,
        opt07_prompt_event(ids[0], prompt.clone()),
        cx,
    );
    cx.run_until_parked();
    let window = owner.read_with(cx, |app, _| {
        app.jobs.records[&ids[0]]
            .window
            .as_ref()
            .expect("batch surface")
            .0
    });
    let mut surface = gpui_kit::VisualTestContext::from_window(window, cx);
    object_draw(&mut surface);
    let reuse = object_node(&mut surface, "仅在本批次复用密码（仅内存）");
    assert_eq!(
        reuse.checked(),
        Some(false),
        "every original prompt starts unchecked"
    );
    surface.simulate_event(gpui_kit::ScrollEvent {
        position: gpui_kit::point(px(240.), px(90.)),
        delta: gpui_kit::ScrollDelta::Pixels(gpui_kit::point(px(0.), px(-1000.))),
        ..Default::default()
    });
    object_draw(&mut surface);
    object_click(&mut surface, "仅在本批次复用密码（仅内存）");
    assert_eq!(
        object_node(&mut surface, "仅在本批次复用密码（仅内存）").checked(),
        Some(true),
        "explicit pointer consent reaches the official Checkbox"
    );
    let input = object_node(&mut surface, "当前任务密码").bounds();
    surface.simulate_click(input.center(), Default::default());
    surface.simulate_input("fixture-password");
    object_click(&mut surface, "继续");
    surface.run_until_parked();
    let replies = object_replies(&endpoint);
    assert_eq!(replies.len(), 1);
    assert_eq!(replies[0].ticket, prompt.ticket);
    assert_eq!(
        serde_json::to_value(&replies[0]).expect("private typed reply")["remember_batch_password"],
        true
    );
}

#[gpui_kit::test]
fn opt07_simultaneous_batch_tickets_switch_on_original_dialog_and_reply_once(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (owner, endpoint) = super::super::super::tests::setup(cx);
    let ids = owner.update(cx, |app, cx| {
        app.submit_batch(vec![request(7040), request(7041), request(7042)], cx)
            .expect("batch")
    });
    cx.run_until_parked();
    let prompts = [
        object_prompt(ids[0], "conflict", Some("a.txt"), r"C:\owned\a.txt"),
        object_prompt(ids[1], "conflict", Some("b.txt"), r"C:\owned\b.txt"),
    ];
    for (id, prompt) in ids.iter().zip(prompts.iter()) {
        deliver(
            &owner,
            &endpoint,
            opt07_prompt_event(*id, prompt.clone()),
            cx,
        );
    }
    cx.run_until_parked();
    let window = owner.read_with(cx, |app, _| {
        app.jobs.records[&ids[0]]
            .window
            .as_ref()
            .expect("one surface")
            .0
    });
    let mut surface = gpui_kit::VisualTestContext::from_window(window, cx);
    surface.simulate_resize(gpui_kit::size(gpui_kit::px(760.), gpui_kit::px(720.)));
    object_draw(&mut surface);
    object_click(&mut surface, "处理待决事项 2");
    surface.run_until_parked();
    assert!(owner.read_with(
        &surface,
        |app, _| app.jobs.records[&ids[0]].prompt.is_some()
            && app.jobs.records[&ids[1]].prompt.is_some()
    ));
    object_click(&mut surface, "跳过");
    surface.run_until_parked();
    owner.update(&mut surface, |app, cx| app.show_job(ids[0], cx));
    object_draw(&mut surface);
    object_click(&mut surface, "覆盖");
    surface.run_until_parked();
    let replies = object_replies(&endpoint);
    assert_eq!(replies.len(), 2);
    assert_eq!(replies[0].ticket, prompts[1].ticket);
    assert_eq!(replies[1].ticket, prompts[0].ticket);
    owner.update(&mut surface, |app, cx| {
        app.reply_job(ids[0], prompts[0].clone(), replies[1].clone(), cx)
    });
    assert!(
        object_replies(&endpoint).is_empty(),
        "stale/double answer remains refused"
    );
    assert_eq!(
        owner.read_with(&surface, |app, _| app
            .jobs
            .records
            .values()
            .filter(|r| r.window.is_some())
            .count()),
        1
    );
}

#[gpui_kit::test]
fn opt07_summary_stop_uses_original_confirmation_before_cancel(cx: &mut gpui_kit::TestAppContext) {
    let (owner, endpoint) = super::super::super::tests::setup(cx);
    let ids = owner.update(cx, |app, cx| {
        app.submit_batch(vec![request(7050), request(7051)], cx)
            .expect("batch")
    });
    cx.run_until_parked();
    let mut running = terminal(ids[0], true);
    running.kind = ArchiveJobEventKind::State;
    running.terminal = None;
    running.state = Some(ArchiveJobState::Running);
    deliver(&owner, &endpoint, running, cx);
    cx.run_until_parked();
    let window = owner.read_with(cx, |app, _| {
        app.jobs.records[&ids[0]]
            .window
            .as_ref()
            .expect("surface")
            .0
    });
    let mut surface = gpui_kit::VisualTestContext::from_window(window, cx);
    surface.simulate_resize(gpui_kit::size(px(760.), px(720.)));
    object_click(&mut surface, "批次汇总");
    object_draw(&mut surface);
    let position = surface
        .debug_bounds("archive-batch-stop-0")
        .expect("exact original member Stop")
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
    assert!(
        owner.read_with(&surface, |app, _| app.jobs.records[&ids[0]]
            .stop_confirmation
            .is_some()),
        "summary Stop must retain original acquired-pause confirmation"
    );
    assert!(!std::iter::from_fn(||endpoint.try_receive_command_for_contract_test().ok()).any(|wire|matches!(wire.message,Message::PlatformCommand(PlatformCommand::ArchiveJob(c)) if c.action==ArchiveJobAction::Cancel)),"no early Cancel");
}

#[gpui_kit::test]
fn opt07_last_confirmation_close_finalizes_zero_member_batch_once(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (owner, _endpoint) = super::super::super::tests::setup(cx);
    let (forms, batch) = owner.update(cx, |app, cx| {
        app.open_extract_form(PathBuf::from(r"C:\owned\one.zip"), None, cx);
        app.open_extract_form(PathBuf::from(r"C:\owned\two.zip"), None, cx);
        let forms = app.jobs.forms.keys().copied().collect::<Vec<_>>();
        app.bind_confirmation_forms(forms.clone(), cx);
        (forms.clone(), app.jobs.confirmation_batches[&forms[0]])
    });
    owner.update(cx, |app, _| {
        for form in &forms {
            app.retire_form(*form);
        }
    });
    assert!(
        owner.read_with(cx, |app, _| app.jobs.batches[&batch].terminal_logged),
        "closing final original confirmation must produce terminal aggregate"
    );
    owner.update(cx, |app, _| app.retire_form(forms[0]));
    assert_eq!(
        owner.read_with(cx, |app, _| app.jobs.batches[&batch].cancelled_unstarted),
        2
    );
}

#[gpui_kit::test]
fn opt07_pending_location_survives_original_64_record_retirement(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (owner, endpoint) = super::super::super::tests::setup(cx);
    let notifications = owner.read_with(cx, |app, _| app.platform.notification_receiver());
    let id = owner.update(cx, |app, cx| {
        app.submit_job(hz3_extract_request(7060), completion(), cx)
            .expect("original Extract")
    });
    let _ = hz3_wait_start(&endpoint, cx);
    let mut event = terminal(id, true);
    let facts = event.terminal.as_mut().expect("native result");
    facts.outcome = ArchiveJobOutcome::SucceededWithWarnings;
    facts.code = None;
    facts.output_published = Some(true);
    facts.result_location = Some(ArchiveJobResultLocation {
        target: r"C:\owned\actual".into(),
        action: nxtools_protocol::ShellOpenAction::Open,
    });
    deliver(&owner, &endpoint, event, cx);
    hz3_wait_terminal(&owner, id, cx);
    owner.update(cx, |app, cx| {
        app.locate_job_result(id, cx);
        app.background_job(id, cx);
    });
    for i in 0..66 {
        let next = owner.update(cx, |app, cx| {
            app.submit_job(request(7100 + i), completion(), cx)
                .expect("bounded Test")
        });
        hz3_complete_closed_test(&owner, &endpoint, next, true, cx);
        while notifications.try_recv().is_ok() {}
    }
    assert!(
        owner.read_with(cx, |app, _| app
            .jobs
            .records
            .get(&id)
            .is_some_and(|record| record.location_pending.is_some())),
        "original native receipt must retain its Job under pressure"
    );
}

#[gpui_kit::test]
fn opt07_retried_batch_member_does_not_offer_original_batch_password_reuse(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (owner, endpoint) = super::super::super::tests::setup(cx);
    let ids = owner.update(cx, |app, cx| {
        app.submit_batch(vec![request(7200), request(7201)], cx)
            .expect("batch")
    });
    cx.run_until_parked();
    let initial = hz3_pending_observation(&owner, ids[0], cx);
    owner.update(cx, |app, cx| {
        app.job_notification(
            &hz3_observation_reply(&initial, hz3_fixture_observation(&initial, "v1:original")),
            cx,
        );
    });
    let mut failed = terminal(ids[0], true);
    failed.terminal = Some(local_terminal("io_failure", true));
    failed.terminal.as_mut().expect("terminal").output_published = Some(false);
    deliver(&owner, &endpoint, failed, cx);
    hz3_wait_terminal(&owner, ids[0], cx);
    while endpoint.try_receive_command_for_contract_test().is_ok() {}
    owner.update(cx, |app, cx| app.retry_job(ids[0], cx));
    let recheck = hz3_pending_observation(&owner, ids[0], cx);
    owner.update(cx, |app, cx| {
        app.job_notification(
            &hz3_observation_reply(&recheck, hz3_fixture_observation(&recheck, "v1:original")),
            cx,
        );
    });
    cx.run_until_parked();
    let start = std::iter::from_fn(|| endpoint.try_receive_command_for_contract_test().ok())
        .find_map(|wire| match wire.message {
            Message::PlatformCommand(PlatformCommand::ArchiveJob(c))
                if c.action == ArchiveJobAction::Start && c.session_id == ids[0] =>
            {
                Some(c)
            }
            _ => None,
        })
        .expect("new attempt");
    assert_eq!(start.generation, 2);
    assert!(
        !owner.read_with(cx, |app, _| app.batch_password_eligible(ids[1])),
        "explicit retry retires reuse for still-active original batch siblings"
    );
    let mut prompt = object_prompt(start.operation_id, "password", None, r"C:\owned\target");
    prompt.display = None;
    let mut event = opt07_prompt_event(start.operation_id, prompt.clone());
    event.session_id = ids[0];
    event.generation = 2;
    deliver(&owner, &endpoint, event, cx);
    cx.run_until_parked();
    owner.update(cx, |app, cx| {
        app.reply_job(
            ids[0],
            prompt.clone(),
            ArchiveJobReply {
                ticket: prompt.ticket.clone(),
                kind: prompt.kind,
                password: Some(nxtools_protocol::archive_password::PrivatePassword::new(
                    "fixture-password",
                )),
                remember_batch_password: true,
                decision: None,
                apply_to_all: false,
                approved_limit: None,
            },
            cx,
        )
    });
    assert_eq!(
        owner.read_with(cx, |app, _| app.jobs.records[&ids[0]].prompt.clone()),
        Some(prompt),
        "ineligible answer cannot retire the current ticket"
    );
    assert!(
        object_replies(&endpoint).is_empty(),
        "ineligible reuse is refused before native dispatch"
    );
    let window = owner.read_with(cx, |app, _| {
        app.jobs.records[&ids[0]]
            .window
            .as_ref()
            .expect("surface")
            .0
    });
    let mut surface = gpui_kit::VisualTestContext::from_window(window, cx);
    assert!(
        !object_labels(&mut surface)
            .iter()
            .any(|label| label == "仅在本批次复用密码（仅内存）"),
        "retry requests need member-only answers"
    );
}

#[gpui_kit::test]
fn opt07_ac02_retried_started_member_waits_for_current_started(cx: &mut gpui_kit::TestAppContext) {
    use crate::archive::batch::MemberState;
    let (owner, endpoint) = super::super::super::tests::setup(cx);
    let id = owner.update(cx, |app, cx| {
        let id = app
            .submit_job(request(7091), completion(), cx)
            .expect("original member");
        app.bind_batch(vec![id], None, cx);
        id
    });
    cx.run_until_parked();
    let initial = std::iter::from_fn(|| endpoint.try_receive_command_for_contract_test().ok())
        .find_map(|wire| match wire.message {
            Message::PlatformCommand(PlatformCommand::ArchiveSpace(command)) => Some(command),
            _ => None,
        })
        .expect("initial native observation");
    let baseline = hz3_fixture_observation(&initial, "v1:ac02-source");
    owner.update(cx, |app, cx| {
        app.job_notification(&hz3_observation_reply(&initial, baseline.clone()), cx);
    });
    let mut first_started = terminal(id, true);
    first_started.kind = ArchiveJobEventKind::Started;
    first_started.terminal = None;
    first_started.validate().expect("original Started event");
    deliver(&owner, &endpoint, first_started.clone(), cx);
    assert!(owner.read_with(cx, |app, _| app.jobs.records[&id].ever_started));
    let mut failed = terminal(id, true);
    failed.terminal = Some(super::super::local_terminal("io_failure", true));
    failed.terminal.as_mut().expect("terminal").output_published = Some(false);
    deliver(&owner, &endpoint, failed, cx);
    cx.run_until_parked();
    let window = owner.read_with(cx, |app, _| {
        app.jobs.records[&id]
            .window
            .as_ref()
            .expect("original window")
            .0
    });
    let mut surface = gpui_kit::VisualTestContext::from_window(window, cx);
    object_click(&mut surface, "重试");
    let recheck = std::iter::from_fn(|| endpoint.try_receive_command_for_contract_test().ok())
        .find_map(|wire| match wire.message {
            Message::PlatformCommand(PlatformCommand::ArchiveSpace(command))
                if command.expected_source.is_some() =>
            {
                Some(command)
            }
            _ => None,
        })
        .expect("explicit fresh Retry recheck");
    owner.update(&mut surface, |app, cx| {
        app.job_notification(&hz3_observation_reply(&recheck, baseline), cx);
    });
    let starts = std::iter::from_fn(|| endpoint.try_receive_command_for_contract_test().ok())
        .filter_map(|wire| match wire.message {
            Message::PlatformCommand(PlatformCommand::ArchiveJob(command))
                if command.action == ArchiveJobAction::Start =>
            {
                Some((wire.id, command))
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(starts.len(), 1, "one new original attempt");
    let (receipt, current) = &starts[0];
    assert_eq!(current.session_id, id);
    assert_eq!(current.generation, 2);
    assert_ne!(current.operation_id, id);
    object_click(&mut surface, "批次汇总");
    let assert_admitting =
        |stage: &str, surface: &mut gpui_kit::VisualTestContext| {
            object_draw(surface);
            owner.read_with(surface, |app, _| {
                let snapshot = app.batch_snapshot_for(id).expect("original batch summary");
                assert_eq!(snapshot.members.len(), 1);
                assert_eq!(
                    snapshot.members[0].state,
                    MemberState::Admitting,
                    "{stage}: historical Started is not current Started"
                );
                assert!(
                    !snapshot.members[0].never_started,
                    "historical start remains accounted"
                );
                assert!(snapshot.facts().starts_with("未开始 0"));
            });
            assert!(
                surface.update(|window, _| gpui_kit::base::test_support::snapshots(window)
                    .iter()
                    .any(|node| {
                        node.label().is_some_and(|label| {
                            label.starts_with("压缩包任务 1：")
                                && label.contains("运行 0")
                                && label.ends_with("正在准入 1")
                        })
                    })),
                "{stage}: rendered original summary remains admitting"
            );
        };
    assert_admitting("dispatch", &mut surface);
    fb_control_receipt(&owner, &endpoint, *receipt, true, &mut surface);
    assert_admitting("receipt", &mut surface);
    deliver(&owner, &endpoint, first_started, &mut surface);
    assert_admitting("stale Started", &mut surface);
    let mut current_started = terminal(id, true);
    current_started.operation_id = current.operation_id;
    current_started.generation = current.generation;
    current_started.kind = ArchiveJobEventKind::Started;
    current_started.terminal = None;
    current_started
        .validate()
        .expect("matching current Started");
    deliver(&owner, &endpoint, current_started, &mut surface);
    object_draw(&mut surface);
    owner.read_with(&surface, |app, _| {
        let snapshot = app.batch_snapshot_for(id).expect("same logical member");
        assert_eq!(snapshot.members[0].state, MemberState::Running);
        assert!(!snapshot.members[0].never_started);
    });
    assert!(
        surface.update(
            |window, _| gpui_kit::base::test_support::snapshots(window)
                .iter()
                .any(|node| {
                    node.label().is_some_and(|label| {
                        label.starts_with("压缩包任务 1：")
                            && label.contains("运行 1")
                            && label.ends_with("正在准入 0")
                    })
                })
        ),
        "only current Started makes the rendered member Running"
    );
}

#[gpui_kit::test]
fn opt07_ac03_checked_password_consent_retirement_keeps_dialog_and_ticket(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (owner, endpoint) = super::super::super::tests::setup(cx);
    let ids = owner.update(cx, |app, cx| {
        app.submit_batch(vec![request(7092), request(7093)], cx)
            .expect("original batch")
    });
    cx.run_until_parked();
    let initial = hz3_pending_observation(&owner, ids[1], cx);
    let baseline = hz3_fixture_observation(&initial, "v1:ac03-source");
    owner.update(cx, |app, cx| {
        app.job_notification(&hz3_observation_reply(&initial, baseline.clone()), cx);
    });
    for id in &ids {
        let mut started = terminal(*id, true);
        started.kind = ArchiveJobEventKind::Started;
        started.terminal = None;
        deliver(&owner, &endpoint, started, cx);
    }
    let mut failed = terminal(ids[1], true);
    failed.terminal = Some(local_terminal("io_failure", true));
    failed.terminal.as_mut().expect("terminal").output_published = Some(false);
    deliver(&owner, &endpoint, failed, cx);
    hz3_wait_terminal(&owner, ids[1], cx);
    while endpoint.try_receive_command_for_contract_test().is_ok() {}
    owner.update(cx, |app, cx| app.show_job(ids[1], cx));
    cx.run_until_parked();
    let window = owner.read_with(cx, |app, _| {
        app.jobs
            .records
            .values()
            .find_map(|record| record.window.as_ref().map(|slot| slot.0))
            .expect("one original window")
    });
    let mut surface = gpui_kit::VisualTestContext::from_window(window, cx);
    surface.simulate_resize(gpui_kit::size(px(760.), px(720.)));
    object_click(&mut surface, "重试");
    let recheck = hz3_pending_observation(&owner, ids[1], &surface);
    assert!(owner.read_with(&surface, |app, _| app.jobs.records[&ids[1]].retry_pending));
    owner.update(&mut surface, |app, cx| app.show_job(ids[0], cx));
    let prompt: ArchiveJobPrompt = serde_json::from_value(serde_json::json!({
        "ticket":{"operation_id":ids[0],"prompt_id":Uuid::new_v4(),"generation":1},
        "kind":"password","error_code":null,"attempt":1,
        "existing_bytes":null,"incoming_bytes":null,"approved_limit":null
    }))
    .expect("original password ticket");
    deliver(
        &owner,
        &endpoint,
        opt07_prompt_event(ids[0], prompt.clone()),
        &mut surface,
    );
    object_draw(&mut surface);
    assert!(owner.read_with(&surface, |app, _| app.batch_password_eligible(ids[0])));
    let input = object_node(&mut surface, "当前任务密码").bounds();
    surface.simulate_click(input.center(), Default::default());
    surface.simulate_input("ac03-private-fixture");
    object_click(&mut surface, "仅在本批次复用密码（仅内存）");
    assert_eq!(
        object_node(&mut surface, "仅在本批次复用密码（仅内存）").checked(),
        Some(true)
    );
    let input_value = object_node(&mut surface, "当前任务密码")
        .value()
        .map(str::to_owned);
    owner.update(&mut surface, |app, cx| {
        app.job_notification(&hz3_observation_reply(&recheck, baseline), cx);
    });
    object_draw(&mut surface);
    owner.read_with(&surface, |app, _| {
        assert!(
            !app.batch_password_eligible(ids[0]),
            "sibling Retry retires consent"
        );
        assert_eq!(app.jobs.records[&ids[1]].command.generation, 2);
        assert_eq!(
            app.jobs.records[&ids[0]].prompt.as_ref(),
            Some(&prompt),
            "same A ticket remains current before confirmation"
        );
    });
    let starts = std::iter::from_fn(|| endpoint.try_receive_command_for_contract_test().ok())
        .filter_map(|wire| match wire.message {
            Message::PlatformCommand(PlatformCommand::ArchiveJob(command))
                if command.action == ArchiveJobAction::Start =>
            {
                Some(command)
            }
            Message::PlatformCommand(PlatformCommand::ArchiveJob(command)) => {
                assert_ne!(
                    command.action,
                    ArchiveJobAction::Reply,
                    "no reply before confirmation"
                );
                None
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(starts.len(), 1, "one original sibling retry attempt");
    assert_eq!(starts[0].session_id, ids[1]);
    object_click(&mut surface, "继续");
    owner.read_with(&surface, |app, _| {
        assert_eq!(
            app.jobs.records[&ids[0]].prompt.as_ref(),
            Some(&prompt),
            "invalid explicit consent cannot retire or downgrade the current ticket"
        );
    });
    assert!(
        surface.update(|window, cx| window.has_active_dialog(cx)),
        "same Dialog stays open"
    );
    assert_eq!(
        object_node(&mut surface, "当前任务密码")
            .value()
            .map(str::to_owned),
        input_value,
        "password input is untouched"
    );
    assert_eq!(
        object_node(&mut surface, "仅在本批次复用密码（仅内存）").checked(),
        Some(true),
        "checked choice remains reachable"
    );
    assert!(
        endpoint.try_receive_command_for_contract_test().is_err(),
        "rejected consent sends no command"
    );
    object_click(&mut surface, "仅在本批次复用密码（仅内存）");
    assert!(
        surface.update(
            |window, _| gpui_kit::base::test_support::snapshots(window)
                .iter()
                .all(|node| {
                    node.label() != Some("仅在本批次复用密码（仅内存）")
                        || node.checked() != Some(true)
                })
        ),
        "explicit uncheck removes the retired choice"
    );
    object_click(&mut surface, "继续");
    let replies = object_replies(&endpoint);
    assert_eq!(
        replies.len(),
        1,
        "explicit uncheck permits exactly one ordinary reply"
    );
    assert_eq!(replies[0].ticket, prompt.ticket);
    assert!(!replies[0].remember_batch_password);
    assert_eq!(
        replies[0].password,
        Some(nxtools_protocol::archive_password::PrivatePassword::new(
            "ac03-private-fixture"
        )),
        "original password bytes survive refusal"
    );
    owner.update(&mut surface, |app, cx| {
        app.reply_job(ids[0], prompt, replies[0].clone(), cx)
    });
    assert!(
        object_replies(&endpoint).is_empty(),
        "ticket remains once-only"
    );
}
