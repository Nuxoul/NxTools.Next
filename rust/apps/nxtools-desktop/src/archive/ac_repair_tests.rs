//! Independent-acceptance counterexamples through the original job and window owners.
use super::*;

fn wire() -> serde_json::Value {
    let path = std::env::var_os("NXTOOLS_ARCHIVE_AC_PROGRESS_WIRE")
        .expect("run the C# AC repair production progress wire fixture and supply its export");
    serde_json::from_slice(&std::fs::read(path).expect("C# production export"))
        .expect("serialized production events")
}
fn edit_request(format: &str) -> ArchiveJobRequest {
    use nxtools_protocol::archive_browser::{
        ArchiveBrowserContext, ArchiveSourceBinding, SevenzEditEligibility, TarEditEligibility,
    };
    ModifyDraft {
        source: r"C:\owned\fixture.archive".into(),
        browser: ArchiveBrowserContext {
            snapshot_id: Uuid::new_v4(),
            format: format.into(),
            comment: String::new(),
            source: ArchiveSourceBinding {
                native: "v1:fixture".into(),
                digest: [0; 32],
                size: 10,
                modified_unix_ns: 1,
            },
            sevenz_edit: (format == "SevenZ").then_some(SevenzEditEligibility { refusal: None }),
            tar_edit: (format != "SevenZ").then_some(TarEditEligibility { refusal: None }),
        },
        action: ArchiveJobModifyAction::Delete,
        ordinals: vec![0],
        value: None,
        source_prefix: None,
        added: vec![],
        codepage: 0,
    }
    .request()
    .expect("matching frozen format request")
}
fn bind(value: &serde_json::Value, id: Uuid) -> ArchiveJobEvent {
    let mut event: ArchiveJobEvent = serde_json::from_value(value.clone()).expect("C# event");
    event.validate().expect("actual wire contract");
    event.operation_id = id;
    event.session_id = id;
    event.generation = 1;
    event
}
fn prompt_event(id: Uuid, kind: &str) -> ArchiveJobEvent {
    let mut prompt = object_prompt(id, kind, Some("incoming.txt"), r"C:\owned\target");
    if !matches!(
        prompt.kind,
        ArchiveJobPromptKind::Conflict | ArchiveJobPromptKind::CreateOverwrite
    ) {
        prompt.display = None;
    } else if prompt.kind == ArchiveJobPromptKind::CreateOverwrite {
        prompt
            .display
            .as_mut()
            .expect("overwrite context")
            .incoming_member = None;
    }
    if prompt.kind == ArchiveJobPromptKind::Risk {
        prompt.approved_limit = Some(10);
    }
    let mut event = terminal(id, true);
    event.kind = ArchiveJobEventKind::Prompt;
    event.terminal = None;
    event.prompt = Some(prompt);
    event.validate().expect("current prompt");
    event
}

#[gpui_kit::test]
fn opt01_ac01_prompt_before_background_recovers(cx: &mut gpui_kit::TestAppContext) {
    close_prompt(false, cx);
}
#[gpui_kit::test]
fn opt01_ac01_prompt_between_close_dispatch_and_defer_recovers(cx: &mut gpui_kit::TestAppContext) {
    close_prompt(true, cx);
}
fn close_prompt(between: bool, cx: &mut gpui_kit::TestAppContext) {
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
        let (owner, endpoint) = super::super::super::tests::setup(cx);
        let center = cx.new(|_| crate::notifications::NotificationCenter::default());
        cx.update(|cx| crate::notifications::register(&center, cx));
        let id = owner.update(cx, |app, cx| {
            app.submit_job(request(1700 + index), completion(), cx)
                .expect("job admitted")
        });
        cx.run_until_parked();
        let old = owner.read_with(cx, |app, _| {
            app.jobs.records[&id]
                .window
                .as_ref()
                .expect("original window")
                .0
        });
        let event = prompt_event(id, kind);
        let mut surface = gpui_kit::VisualTestContext::from_window(old, cx);
        let bounds = object_node(&mut surface, "后台").bounds();
        let notification =
            PlatformNotification::Event(PlatformEvent::ArchiveJob(Box::new(event.clone())));
        endpoint
            .send_notification_for_contract_test(notification.clone())
            .expect("original notification channel");
        surface.update(|window, cx| {
            use gpui_kit::InputEvent as _;
            if !between {
                owner.update(cx, |app, cx| {
                    app.job_notification(&notification, cx);
                });
            }
            window.dispatch_event(
                gpui_kit::MouseDownEvent {
                    position: bounds.center(),
                    button: gpui_kit::MouseButton::Left,
                    ..Default::default()
                }
                .to_platform_input(),
                cx,
            );
            window.dispatch_event(
                gpui_kit::MouseUpEvent {
                    position: bounds.center(),
                    button: gpui_kit::MouseButton::Left,
                    modifiers: Default::default(),
                    click_count: 1,
                }
                .to_platform_input(),
                cx,
            );
            if between {
                owner.update(cx, |app, cx| {
                    app.job_notification(&notification, cx);
                });
            }
            owner.read_with(cx, |app, _| {
                assert_eq!(
                    app.jobs.records[&id].window.as_ref().map(|slot| slot.0),
                    Some(old),
                    "background retirement is deferred"
                );
                assert_eq!(app.jobs.records[&id].prompt, event.prompt);
            });
        });
        cx.run_until_parked();
        cx.executor()
            .advance_clock(std::time::Duration::from_secs(7));
        cx.run_until_parked();
        let restored = owner
            .read_with(cx, |app, _| {
                app.jobs.records[&id].window.as_ref().map(|slot| slot.0)
            })
            .expect(
                "AC-01 current prompt must remain reachable after native close and toast expiry",
            );
        assert_ne!(old, restored);
        let mut restored_surface = gpui_kit::VisualTestContext::from_window(restored, cx);
        object_draw(&mut restored_surface);
        assert!(
            object_labels(&mut restored_surface)
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
            Some(restored)
        );
        assert_eq!(
            owner.read_with(cx, |app, _| app.jobs.records[&id].prompt.clone()),
            event.prompt
        );
        assert!(
            object_replies(&endpoint).is_empty(),
            "close must not answer a prompt"
        );
        answer_prompt_once(&owner, &endpoint, id, &event, &mut restored_surface, cx);
        deliver(&owner, &endpoint, terminal(id, true), cx);
        owner.update(cx, |app, cx| app.background_job(id, cx));
        deliver(&owner, &endpoint, event, cx);
        cx.run_until_parked();
        assert!(owner.read_with(cx, |app, _| app.jobs.records[&id].window.is_none()));
    }
}

#[gpui_kit::test]
fn opt01_ac02_two_channel_reorder_preserves_phase_and_metric_epoch(
    cx: &mut gpui_kit::TestAppContext,
) {
    let export = wire();
    let rows = export["reordered"]
        .as_array()
        .expect("held production frames");
    assert_eq!(rows.len(), 8);
    for row in rows {
        let (owner, endpoint) = super::super::super::tests::setup(cx);
        let id = owner.update(cx, |app, cx| {
            app.submit_job(
                edit_request(row["format"].as_str().expect("exported format")),
                completion(),
                cx,
            )
            .expect("edit job admitted")
        });
        cx.run_until_parked();
        let newer = bind(&row["newer"], id);
        let phase = newer
            .progress
            .as_ref()
            .expect("newer progress")
            .phase
            .clone();
        opt01_state(
            &owner,
            &endpoint,
            id,
            if phase == "edit_verifying" {
                ArchiveJobState::Verifying
            } else {
                ArchiveJobState::Committing
            },
            cx,
        );
        deliver(&owner, &endpoint, newer.clone(), cx);
        cx.run_until_parked();
        let epoch = owner.read_with(cx, |app, _| app.jobs.records[&id].progress_elapsed_ms);
        let window = owner.read_with(cx, |app, _| {
            app.jobs.records[&id]
                .window
                .as_ref()
                .expect("original window")
                .0
        });
        let mut surface = gpui_kit::VisualTestContext::from_window(window, cx);
        object_draw(&mut surface);
        cx.executor()
            .advance_clock(std::time::Duration::from_millis(1200));
        deliver(&owner, &endpoint, bind(&row["held"], id), cx);
        object_draw(&mut surface);
        owner.read_with(cx, |app, _| {
            assert_eq!(
                app.jobs.records[&id].progress, newer.progress,
                "AC-02 late coalesced phase must not replace lossless phase"
            );
            assert_eq!(
                app.jobs.records[&id].progress_elapsed_ms, epoch,
                "late progress cannot reset metric observation time"
            );
        });
        let label = if phase == "edit_verifying" {
            "校验完整候选；尚未保存"
        } else {
            "等待原生提交；尚未确认保存"
        };
        assert!(
            object_labels(&mut surface)
                .iter()
                .any(|value| value == label)
        );
        assert_eq!(object_node(&mut surface, "暂停").disabled(), Some(true));
        deliver(&owner, &endpoint, terminal(id, true), cx);
    }
}

#[gpui_kit::test]
fn opt01_ac03_each_frozen_format_uses_its_own_serialized_phases(cx: &mut gpui_kit::TestAppContext) {
    let export = wire();
    let rows = export["formats"].as_array().expect("four actual requests");
    assert_eq!(rows.len(), 4);
    let (owner, endpoint) = super::super::super::tests::setup(cx);
    for row in rows {
        let format = row["format"].as_str().expect("exported format");
        let id = owner.update(cx, |app, cx| {
            app.submit_job(edit_request(format), completion(), cx)
                .expect("edit job admitted")
        });
        opt01_state(&owner, &endpoint, id, ArchiveJobState::Running, cx);
        let window = owner.read_with(cx, |app, _| {
            app.jobs.records[&id]
                .window
                .as_ref()
                .expect("original window")
                .0
        });
        let mut surface = gpui_kit::VisualTestContext::from_window(window, cx);
        let frames = row["events"].as_array().expect("serialized phase events");
        assert_eq!(frames.len(), 6);
        for frame in frames {
            let event = bind(frame, id);
            let phase = event
                .progress
                .as_ref()
                .expect("phase progress")
                .phase
                .clone();
            deliver(&owner, &endpoint, event, cx);
            object_draw(&mut surface);
            let labels = object_labels(&mut surface);
            let expected = match (format, phase.as_str()) {
                ("SevenZ", "edit_planning") => "准备 7z 编辑；容量需求未知",
                ("SevenZ", "edit_reencoding") => {
                    "完整解码并重编码为非固实 LZMA2；处理量不是保存进度"
                }
                ("Tar", "edit_planning") => "准备 TAR 编辑；容量需求未知",
                ("TarGz", "edit_planning") => "准备 TAR.GZ 编辑；容量需求未知",
                ("TarXz", "edit_planning") => "准备 TAR.XZ 编辑；容量需求未知",
                (_, "edit_reencoding") => {
                    "完整重写 TAR 压缩包及外层压缩（如有）；处理量不是保存进度"
                }
                (_, "edit_reading") => "读取并检查原压缩包",
                (_, "edit_verifying") => "校验完整候选；尚未保存",
                (_, "edit_committing") => "等待原生提交；尚未确认保存",
                _ => "正在修改压缩包",
            };
            assert!(
                labels.iter().any(|label| label == expected),
                "AC-03 {format}/{phase} must describe its frozen edit format: {labels:?}"
            );
            if format != "SevenZ" {
                assert!(
                    !labels
                        .iter()
                        .any(|label| label.contains("7z") || label.contains("LZMA2"))
                );
            }
        }
        deliver(&owner, &endpoint, terminal(id, true), cx);
    }
}

#[gpui_kit::test]
fn opt01_ac02_retry_restart_and_replacement_attempt(cx: &mut gpui_kit::TestAppContext) {
    let export = wire();
    let (owner, endpoint) = super::super::super::tests::setup(cx);
    let id = owner.update(cx, |app, cx| {
        app.submit_job(edit_request("Tar"), completion(), cx)
            .expect("retry job admitted")
    });
    opt01_state(&owner, &endpoint, id, ArchiveJobState::Running, cx);
    let frames = export["retries"].as_array().expect("retry progress frames");
    assert_eq!(frames.len(), 4);
    for (index, frame) in frames.iter().enumerate() {
        if index == 1 || index == 3 {
            let prompt = prompt_event(id, "password")
                .prompt
                .expect("password prompt");
            let mut surface = show_object_prompt(&owner, &endpoint, id, prompt.clone(), cx);
            let bounds = object_node(&mut surface, "当前任务密码").bounds();
            surface.simulate_click(bounds.center(), gpui_kit::Modifiers::default());
            surface.simulate_input("owned-retry-password");
            object_click(&mut surface, "继续");
            let replies = object_replies(&endpoint);
            assert_eq!(replies.len(), 1);
            assert_eq!(replies[0].ticket, prompt.ticket);
            opt01_state(&owner, &endpoint, id, ArchiveJobState::Running, cx);
        }
        let event = bind(frame, id);
        cx.executor()
            .advance_clock(std::time::Duration::from_millis(1200));
        deliver(&owner, &endpoint, event.clone(), cx);
        cx.run_until_parked();
        assert_eq!(
            owner.read_with(cx, |app, _| app.jobs.records[&id].progress.clone()),
            event.progress,
            "newer internal retry may restart phase and same-phase counters"
        );
    }
    let current = bind(frames.last().expect("current retry frame"), id);
    let epoch = owner.read_with(cx, |app, _| app.jobs.records[&id].progress_elapsed_ms);
    for old in [bind(&frames[0], id), current.clone()] {
        deliver(&owner, &endpoint, old, cx);
        assert_eq!(
            owner.read_with(cx, |app, _| app.jobs.records[&id].progress_elapsed_ms),
            epoch
        );
    }
    let mut legacy = current.clone();
    legacy.progress.as_mut().expect("legacy progress").sequence = None;
    legacy
        .progress
        .as_mut()
        .expect("legacy progress")
        .bytes_done = 9;
    deliver(&owner, &endpoint, legacy, cx);
    assert_eq!(
        owner.read_with(cx, |app, _| app.jobs.records[&id].progress.clone()),
        current.progress
    );
    let attempt = Uuid::new_v4();
    owner.update(cx, |app, cx| {
        let mut command = app.jobs.records[&id].command.clone();
        command.operation_id = attempt;
        app.send_job_attempt(id, command, cx);
    });
    let mut first = bind(&export["formats"][1]["events"][0], id);
    first.operation_id = attempt;
    assert_eq!(
        first
            .progress
            .as_ref()
            .expect("new attempt progress")
            .sequence,
        Some(1)
    );
    deliver(&owner, &endpoint, first.clone(), cx);
    deliver(&owner, &endpoint, current, cx);
    assert_eq!(
        owner.read_with(cx, |app, _| app.jobs.records[&id].progress.clone()),
        first.progress,
        "new attempt resets progress order and refuses old attempt observations"
    );
}

#[gpui_kit::test]
fn opt01_ac01_close_recovery_rechecks_retirement(cx: &mut gpui_kit::TestAppContext) {
    for retirement in ["terminal", "disconnect", "replacement"] {
        let (owner, endpoint) = super::super::super::tests::setup(cx);
        let id = owner.update(cx, |app, cx| {
            app.submit_job(request(1900), completion(), cx)
                .expect("job admitted")
        });
        cx.run_until_parked();
        let old = owner.read_with(cx, |app, _| {
            app.jobs.records[&id]
                .window
                .as_ref()
                .expect("original window")
                .0
        });
        let event = prompt_event(id, "conflict");
        let mut surface = gpui_kit::VisualTestContext::from_window(old, cx);
        let bounds = object_node(&mut surface, "后台").bounds();
        surface.update(|window, cx| {
            use gpui_kit::InputEvent as _;
            owner.update(cx, |app, cx| {
                app.job_notification(
                    &PlatformNotification::Event(PlatformEvent::ArchiveJob(Box::new(
                        event.clone(),
                    ))),
                    cx,
                );
            });
            window.dispatch_event(
                gpui_kit::MouseDownEvent {
                    position: bounds.center(),
                    button: gpui_kit::MouseButton::Left,
                    ..Default::default()
                }
                .to_platform_input(),
                cx,
            );
            window.dispatch_event(
                gpui_kit::MouseUpEvent {
                    position: bounds.center(),
                    button: gpui_kit::MouseButton::Left,
                    modifiers: Default::default(),
                    click_count: 1,
                }
                .to_platform_input(),
                cx,
            );
            let owner = owner.clone();
            // Background is already queued; retirement precedes its deferred restoration check.
            cx.defer(move |cx| {
                owner.update(cx, |app, cx| {
                    assert!(app.jobs.records[&id].window.is_none());
                    match retirement {
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
                                    generation: app
                                        .platform
                                        .connection_generation()
                                        .expect("current connection"),
                                    pending_command_ids: vec![],
                                },
                                cx,
                            );
                        }
                        _ => {
                            let mut command = app.jobs.records[&id].command.clone();
                            command.operation_id = Uuid::new_v4();
                            app.send_job_attempt(id, command, cx);
                        }
                    }
                })
            });
        });
        cx.run_until_parked();
        deliver(&owner, &endpoint, event, cx);
        cx.run_until_parked();
        assert!(
            owner.read_with(cx, |app, _| app.jobs.records[&id].window.is_none()),
            "{retirement} cannot revive the old decision"
        );
    }
}

fn answer_prompt_once(
    owner: &Entity<ArchiveApplication>,
    endpoint: &nxtools_ipc::PlatformServerEndpoint,
    id: Uuid,
    event: &ArchiveJobEvent,
    surface: &mut gpui_kit::VisualTestContext,
    cx: &mut gpui_kit::TestAppContext,
) {
    let prompt = event.prompt.clone().expect("current prompt");
    if prompt.kind == ArchiveJobPromptKind::Password {
        let bounds = object_node(surface, "当前任务密码").bounds();
        surface.simulate_click(bounds.center(), gpui_kit::Modifiers::default());
        surface.simulate_input("owned-restored-password");
    }
    object_click(
        surface,
        match prompt.kind {
            ArchiveJobPromptKind::Conflict => "跳过",
            ArchiveJobPromptKind::CreateOverwrite => "确认覆盖",
            _ => "继续",
        },
    );
    let replies = object_replies(endpoint);
    assert_eq!(replies.len(), 1, "one restored decision response");
    assert_eq!(replies[0].ticket, prompt.ticket);
    assert_eq!(replies[0].kind, prompt.kind);
    match prompt.kind {
        ArchiveJobPromptKind::Password => assert!(replies[0].password.is_some()),
        ArchiveJobPromptKind::Risk => assert_eq!(replies[0].approved_limit, Some(10)),
        ArchiveJobPromptKind::Conflict => {
            assert_eq!(replies[0].decision, Some(ArchiveJobConflictDecision::Skip))
        }
        _ => assert_eq!(
            replies[0].decision,
            Some(ArchiveJobConflictDecision::Overwrite)
        ),
    }
    deliver(owner, endpoint, event.clone(), cx);
    owner.update(cx, |app, cx| {
        app.reply_job(id, prompt, replies[0].clone(), cx)
    });
    cx.run_until_parked();
    assert!(
        object_replies(endpoint).is_empty(),
        "duplicate ticket or callback cannot reply twice"
    );
    assert!(owner.read_with(cx, |app, _| app.jobs.records[&id].prompt.is_none()));
    assert!(!surface.update(|window, cx| window.has_active_dialog(cx)));
}

#[gpui_kit::test]
fn opt01_ac01_native_close_restores_displayed_prompt_and_reply(cx: &mut gpui_kit::TestAppContext) {
    for kind in [
        "password",
        "conflict",
        "risk",
        "destination",
        "create_overwrite",
    ] {
        let (owner, endpoint) = super::super::super::tests::setup(cx);
        let id = owner.update(cx, |app, cx| {
            app.submit_job(request(2000), completion(), cx)
                .expect("job admitted")
        });
        cx.run_until_parked();
        let event = prompt_event(id, kind);
        let mut surface = show_object_prompt(
            &owner,
            &endpoint,
            id,
            event.prompt.clone().expect("prompt"),
            cx,
        );
        let old = owner.read_with(cx, |app, _| {
            app.jobs.records[&id]
                .window
                .as_ref()
                .expect("visible prompt window")
                .0
        });
        // Locked GPUI reinstalls the handler after invocation; this callback retires that window.
        let missing =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| surface.simulate_close()))
                .expect_err("locked TestWindow reinstalls a handler on the retired window");
        let message = missing
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| missing.downcast_ref::<&str>().copied())
            .expect("TestWindow assertion text");
        assert!(
            message.contains("window not found"),
            "unexpected close panic: {message}"
        );
        assert!(
            old.update(cx, |_, _, _| ()).is_err(),
            "original native-close callback retired its window"
        );
        cx.run_until_parked();
        let restored = owner
            .read_with(cx, |app, _| {
                app.jobs.records[&id].window.as_ref().map(|slot| slot.0)
            })
            .expect("native close must restore the current displayed prompt");
        assert_ne!(old, restored);
        let mut surface = gpui_kit::VisualTestContext::from_window(restored, cx);
        object_draw(&mut surface);
        answer_prompt_once(&owner, &endpoint, id, &event, &mut surface, cx);
        deliver(&owner, &endpoint, terminal(id, true), cx);
        owner.update(cx, |app, cx| app.background_job(id, cx));
        cx.run_until_parked();
    }
}
