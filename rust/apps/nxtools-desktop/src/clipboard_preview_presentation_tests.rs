use super::*;
use gpui_kit::TestAppContext;
use nxtools_ipc::PlatformServerEndpoint;
use nxtools_protocol::{Envelope, Message};

struct Fixture {
    application: Entity<ClipboardApplication>,
    endpoint: PlatformServerEndpoint,
    shutdown: crate::clipboard_worker::ClipboardCaptureWorkerHandle,
    _directory: tempfile::TempDir,
}
fn setup(cx: &mut TestAppContext) -> Fixture {
    let directory = tempfile::tempdir().expect("synthetic store");
    let config_path = directory.path().join("config.json");
    AppConfig::default()
        .save_to(&config_path)
        .expect("seed fixture config");
    let worker = crate::clipboard_worker::ClipboardCaptureWorkerHandle::spawn(
        directory.path().join("clipboard.db"),
        directory.path().join("assets"),
        ClipboardRetentionPolicy {
            max_items: 1000,
            retention_days: 30,
        },
        1000,
    )
    .expect("real temporary worker");
    let shutdown = worker.clone();
    let (platform, endpoint) = nxtools_ipc::platform_bridge(32);
    let _ = endpoint.open_connection_for_contract_test();
    let application = cx.new(|cx| {
        let notifications = cx.new(|_| NotificationCenter::default());
        ClipboardApplication::new(
            worker,
            Rc::new(platform),
            AppConfig::default(),
            config_path,
            notifications,
            cx,
        )
    });
    while endpoint.try_receive_command_for_contract_test().is_ok() {}
    application.update(cx, |application, _| {
        application.preview.native_window_handle = 88;
        application.preview.hovered_item_id = Some(7);
        application.preview.generation = 4;
    });
    Fixture {
        application,
        endpoint,
        shutdown,
        _directory: directory,
    }
}
fn stage() -> PendingPreviewStage {
    let item = ClipboardItem {
        id: 7,
        kind: ClipboardKind::Text,
        text: "synthetic".into(),
        image_path: String::new(),
        image_width: 0,
        image_height: 0,
        file_paths: Vec::new(),
        source: nxtools_clipboard::CaptureSource {
            window_handle: 0,
            process_id: 0,
            process_name: String::new(),
            process_icon_path: String::new(),
            window_title: String::new(),
        },
        captured_at_unix_ms: 1,
        last_used_at_unix_ms: 1,
        copy_count: 1,
        is_favorite: false,
        is_pinned: false,
        deleted_at_unix_ms: None,
    };
    PendingPreviewStage {
        command_id: uuid::Uuid::nil(),
        native_window_handle: 88,
        phase: PreviewStagePhase::InitialNoActivate,
        generation: 4,
        item,
        layout: PreviewLayout {
            window_width: 400.0,
            window_height: 410.0,
            body_height: 350.0,
            requires_vertical_scroll: false,
            maximum_height: 560.0,
            image: None,
        },
        preview_size: size(px(400.0), px(410.0)),
        bounds: Bounds::new(point(px(120.0), px(140.0)), size(px(400.0), px(410.0))),
        panel_scale_factor: 1.25,
        content_measured: true,
    }
}
fn receive(fixture: &Fixture, cx: &mut TestAppContext) -> Envelope {
    let envelope = fixture
        .endpoint
        .try_receive_command_for_contract_test()
        .expect("actual command queue");
    assert_eq!(
        fixture.application.read_with(cx, |app, _| app
            .preview
            .pending_stage
            .as_ref()
            .expect("pending before dispatch")
            .command_id),
        envelope.id
    );
    envelope
}
fn confirm(fixture: &Fixture, envelope: &Envelope, success: bool, cx: &mut TestAppContext) {
    assert!(
        fixture
            .application
            .update(cx, |app, cx| app.handle_preview_stage_result(
                envelope.id,
                success,
                None,
                cx
            ))
    );
}
#[gpui_kit::test]
fn first_preview_confirms_noactivate_bounds_topmost_then_show(cx: &mut TestAppContext) {
    let fixture = setup(cx);
    fixture.application.update(cx, |app, cx| {
        app.begin_initial_preview_presentation(stage(), cx)
    });
    let policy = receive(&fixture, cx);
    assert!(matches!(
        policy.message,
        Message::PlatformCommand(PlatformCommand::SetWindowNoActivate(
            SetWindowNoActivateCommand {
                window_handle: 88,
                enabled: true
            }
        ))
    ));
    assert!(
        fixture
            .endpoint
            .try_receive_command_for_contract_test()
            .is_err()
    );
    confirm(&fixture, &policy, true, cx);
    let bounds = receive(&fixture, cx);
    assert!(matches!(
        bounds.message,
        Message::PlatformCommand(PlatformCommand::SetWindowBounds(SetWindowBoundsCommand {
            window_handle: 88,
            x: 150,
            y: 175,
            width: 500,
            height: 513
        }))
    ));
    confirm(&fixture, &bounds, true, cx);
    let topmost = receive(&fixture, cx);
    assert!(matches!(
        topmost.message,
        Message::PlatformCommand(PlatformCommand::SetWindowTopmost(SetWindowTopmostCommand {
            window_handle: 88,
            enabled: true
        }))
    ));
    assert!(
        fixture
            .application
            .read_with(cx, |app, _| !app.preview.visible
                && app.preview.displayed_item_id.is_none())
    );
    confirm(&fixture, &topmost, true, cx);
    let show = receive(&fixture, cx);
    assert!(matches!(
        show.message,
        Message::PlatformCommand(PlatformCommand::SetWindowVisibility(
            SetWindowVisibilityCommand {
                window_handle: 88,
                visible: true
            }
        ))
    ));
    assert!(
        fixture
            .application
            .read_with(cx, |app, _| app.preview.displayed_item_id.is_none()),
        "queue admission is not presented content"
    );
    confirm(&fixture, &show, true, cx);
    fixture.application.read_with(cx, |app, _| {
        assert!(
            app.preview.visible
                && app.preview.displayed_item_id == Some(7)
                && app.preview.pending_stage.is_none()
        )
    });
    fixture.shutdown.shutdown().expect("close temporary worker");
}
#[gpui_kit::test]
fn first_preview_failure_at_each_native_step_never_advances_or_resurrects(cx: &mut TestAppContext) {
    let fixture = setup(cx);
    for failed_step in 0..4 {
        fixture.application.update(cx, |app, cx| {
            app.preview.visible = false;
            app.preview.displayed_item_id = None;
            app.begin_initial_preview_presentation(stage(), cx);
        });
        let mut envelope = receive(&fixture, cx);
        for _ in 0..failed_step {
            confirm(&fixture, &envelope, true, cx);
            envelope = receive(&fixture, cx);
        }
        confirm(&fixture, &envelope, false, cx);
        assert!(
            fixture
                .application
                .read_with(cx, |app, _| !app.preview.visible
                    && app.preview.pending_stage.is_none()
                    && app.preview.displayed_item_id.is_none())
        );
        assert!(
            !fixture
                .application
                .update(cx, |app, cx| app.handle_preview_stage_result(
                    envelope.id,
                    true,
                    None,
                    cx
                ))
        );
        while let Ok(envelope) = fixture.endpoint.try_receive_command_for_contract_test() {
            assert!(matches!(
                envelope.message,
                Message::PlatformCommand(PlatformCommand::SetWindowVisibility(
                    SetWindowVisibilityCommand { visible: false, .. }
                ))
            ));
        }
    }
    fixture.shutdown.shutdown().expect("close temporary worker");
}
#[gpui_kit::test]
fn cancelled_or_stale_first_show_is_followed_by_hide_and_late_result_is_ignored(
    cx: &mut TestAppContext,
) {
    let fixture = setup(cx);
    for cancelled in [false, true] {
        fixture.application.update(cx, |app, cx| {
            app.preview.visible = false;
            app.preview.generation = 4;
            app.preview.hovered_item_id = Some(7);
            app.begin_initial_preview_presentation(stage(), cx);
        });
        let mut envelope = receive(&fixture, cx);
        for _ in 0..3 {
            confirm(&fixture, &envelope, true, cx);
            envelope = receive(&fixture, cx);
        }
        fixture.application.update(cx, |app, cx| {
            if cancelled {
                app.dismiss_preview(cx);
            } else {
                app.preview.generation = 5;
            }
        });
        if !cancelled {
            confirm(&fixture, &envelope, true, cx);
        } else {
            assert!(
                !fixture
                    .application
                    .update(cx, |app, cx| app.handle_preview_stage_result(
                        envelope.id,
                        true,
                        None,
                        cx
                    ))
            );
        }
        let hide = fixture
            .endpoint
            .try_receive_command_for_contract_test()
            .expect("hide behind possibly in-flight show");
        assert!(matches!(
            hide.message,
            Message::PlatformCommand(PlatformCommand::SetWindowVisibility(
                SetWindowVisibilityCommand {
                    window_handle: 88,
                    visible: false
                }
            ))
        ));
        assert!(
            fixture
                .application
                .read_with(cx, |app, _| !app.preview.visible
                    && app.preview.pending_stage.is_none())
        );
    }
    fixture.shutdown.shutdown().expect("close temporary worker");
}
#[gpui_kit::test]
fn unconfirmed_first_preview_step_expires_without_show(cx: &mut TestAppContext) {
    let fixture = setup(cx);
    fixture.application.update(cx, |app, cx| {
        app.begin_initial_preview_presentation(stage(), cx)
    });
    let envelope = receive(&fixture, cx);
    cx.run_until_parked();
    cx.executor().advance_clock(PRESENTATION_STEP_TIMEOUT);
    cx.run_until_parked();
    assert!(
        fixture
            .application
            .read_with(cx, |app, _| app.preview.pending_stage.is_none()
                && !app.preview.visible)
    );
    assert!(
        fixture
            .endpoint
            .try_receive_command_for_contract_test()
            .is_err()
    );
    assert!(
        !fixture
            .application
            .update(cx, |app, cx| app.handle_preview_stage_result(
                envelope.id,
                true,
                None,
                cx
            ))
    );
    fixture.shutdown.shutdown().expect("close temporary worker");
}

#[gpui_kit::test]
fn reused_preview_reconfirms_passive_policy_before_offscreen_move(cx: &mut TestAppContext) {
    let fixture = setup(cx);
    fixture.application.update(cx, |app, cx| {
        app.begin_reused_preview_presentation(stage(), cx)
    });
    let policy = receive(&fixture, cx);
    assert!(matches!(
        policy.message,
        Message::PlatformCommand(PlatformCommand::SetWindowNoActivate(_))
    ));
    confirm(&fixture, &policy, true, cx);
    let movement = receive(&fixture, cx);
    assert!(matches!(
        movement.message,
        Message::PlatformCommand(PlatformCommand::SetWindowPosition(
            SetWindowPositionCommand {
                x: PREVIEW_STAGING_COORDINATE,
                y: PREVIEW_STAGING_COORDINATE,
                ..
            }
        ))
    ));
    assert_eq!(
        fixture.application.read_with(cx, |app, _| app
            .preview
            .pending_stage
            .as_ref()
            .expect("move pending")
            .phase),
        PreviewStagePhase::MoveOffscreen
    );
    fixture
        .application
        .update(cx, |app, cx| app.dismiss_preview(cx));
    fixture.shutdown.shutdown().expect("close temporary worker");
}

#[gpui_kit::test]
fn replaced_preview_hwnd_does_not_accept_old_presentation_result(cx: &mut TestAppContext) {
    let fixture = setup(cx);
    for success in [false, true] {
        fixture.application.update(cx, |app, cx| {
            app.preview.native_window_handle = 88;
            app.begin_initial_preview_presentation(stage(), cx);
            app.preview.native_window_handle = 99;
            app.preview.visible = true;
        });
        let policy = receive(&fixture, cx);
        confirm(&fixture, &policy, success, cx);
        assert!(
            fixture
                .endpoint
                .try_receive_command_for_contract_test()
                .is_err()
        );
        assert!(
            fixture
                .application
                .read_with(cx, |app, _| app.preview.visible)
        );
    }
    fixture.shutdown.shutdown().expect("close temporary worker");
}

#[gpui_kit::test]
fn unconfirmed_reused_preview_policy_expires_without_moving_or_showing(cx: &mut TestAppContext) {
    let fixture = setup(cx);
    fixture.application.update(cx, |app, cx| {
        app.begin_reused_preview_presentation(stage(), cx)
    });
    let envelope = receive(&fixture, cx);
    cx.run_until_parked();
    cx.executor().advance_clock(PRESENTATION_STEP_TIMEOUT);
    cx.run_until_parked();
    assert!(
        fixture
            .application
            .read_with(cx, |app, _| app.preview.pending_stage.is_none()
                && !app.preview.visible)
    );
    assert!(
        fixture
            .endpoint
            .try_receive_command_for_contract_test()
            .is_err()
    );
    assert!(
        !fixture
            .application
            .update(cx, |app, cx| app.handle_preview_stage_result(
                envelope.id,
                true,
                None,
                cx
            ))
    );
    fixture.shutdown.shutdown().expect("close temporary worker");
}

#[gpui_kit::test]
fn replaced_preview_hwnd_does_not_receive_old_presentation_timeout_hide(cx: &mut TestAppContext) {
    let fixture = setup(cx);
    fixture.application.update(cx, |app, cx| {
        app.begin_initial_preview_presentation(stage(), cx);
        app.preview.native_window_handle = 99;
        app.preview.visible = true;
    });
    let _ = receive(&fixture, cx);
    cx.run_until_parked();
    cx.executor().advance_clock(PRESENTATION_STEP_TIMEOUT);
    cx.run_until_parked();
    assert!(
        fixture
            .application
            .read_with(cx, |app, _| app.preview.pending_stage.is_none()
                && app.preview.visible)
    );
    assert!(
        fixture
            .endpoint
            .try_receive_command_for_contract_test()
            .is_err()
    );
    fixture.shutdown.shutdown().expect("close temporary worker");
}

#[gpui_kit::test]
fn first_preview_virtual_milestones_isolate_four_serial_acknowledgements(cx: &mut TestAppContext) {
    let fixture = setup(cx);
    fixture.application.update(cx, |app, cx| {
        app.begin_initial_preview_presentation(stage(), cx);
    });
    let mut elapsed = Duration::ZERO;
    for step in 0..4 {
        let envelope = receive(&fixture, cx);
        assert!(
            fixture
                .endpoint
                .try_receive_command_for_contract_test()
                .is_err(),
            "no overlap or early reveal across acknowledged native policy steps"
        );
        cx.run_until_parked();
        let synthetic_round_trip = Duration::from_millis(25);
        cx.executor().advance_clock(synthetic_round_trip);
        elapsed += synthetic_round_trip;
        confirm(&fixture, &envelope, true, cx);
        assert_eq!(
            fixture
                .application
                .read_with(cx, |app, _| app.preview.displayed_item_id),
            (step == 3).then_some(7)
        );
    }
    assert_eq!(elapsed, Duration::from_millis(100));
    assert_eq!(PREVIEW_HOVER_DELAY + elapsed, Duration::from_millis(900));
    fixture.shutdown.shutdown().expect("close temporary worker");
}

#[gpui_kit::test]
fn first_preview_real_hover_scheduler_waits_800ms_and_cancel_prevents_creation(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_kit::init);
    cx.executor().allow_parking();
    let fixture = setup(cx);
    let item = {
        let mut history = nxtools_clipboard::ClipboardHistory::open(
            fixture._directory.path().join("clipboard.db"),
        )
        .expect("actual preview hydration DB");
        for index in 0..7 {
            history
                .capture(nxtools_clipboard::TextCapture {
                    text: format!("synthetic preview {index} {} TAIL", "文".repeat(600)),
                    native_formats: Vec::new(),
                    source: stage().item.source,
                    captured_at_unix_ms: index + 1,
                })
                .expect("seed preview record");
        }
        history.active_item(7).expect("real active preview item")
    };
    let candidate = PreviewCandidate {
        item,
        panel_bounds: Bounds::new(point(px(0.0), px(0.0)), size(px(300.0), px(440.0))),
        pointer_position: point(px(180.0), px(200.0)),
        panel_scale_factor: 1.25,
    };
    fixture.application.update(cx, |app, cx| {
        app.schedule_preview_candidate(candidate.clone(), cx)
    });
    cx.run_until_parked();
    cx.executor().advance_clock(Duration::from_millis(799));
    cx.run_until_parked();
    assert!(
        fixture
            .application
            .read_with(cx, |app, _| app.preview.window.is_none())
    );
    assert!(
        fixture
            .endpoint
            .try_receive_command_for_contract_test()
            .is_err()
    );
    fixture
        .application
        .update(cx, |app, cx| app.dismiss_preview(cx));
    cx.executor().advance_clock(Duration::from_millis(1));
    cx.run_until_parked();
    assert!(
        fixture
            .application
            .read_with(cx, |app, _| app.preview.window.is_none())
    );
    fixture.application.update(cx, |app, cx| {
        app.preview.hovered_item_id = Some(7);
        app.schedule_preview_candidate(candidate, cx);
    });
    cx.run_until_parked();
    cx.executor().advance_clock(PREVIEW_HOVER_DELAY);
    cx.run_until_parked();
    for _ in 0..200 {
        cx.run_until_parked();
        if fixture
            .application
            .read_with(cx, |app, _| app.preview.window.is_some())
        {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(
        fixture
            .application
            .read_with(cx, |app, _| app.preview.window.is_some())
    );
    // TestWindow has no HWND: this is logical construction, never native show success.
    assert!(
        fixture
            .endpoint
            .try_receive_command_for_contract_test()
            .is_err()
    );
    fixture.shutdown.shutdown().expect("close temporary worker");
}

#[gpui_kit::test]
fn round2_reused_preview_confirms_new_native_size_before_render_reveal(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let fixture = setup(cx);
    let mut next = stage();
    next.preview_size = size(px(190.0), px(76.0));
    next.layout = PreviewLayout {
        window_width: 190.0,
        window_height: 76.0,
        body_height: 30.0,
        requires_vertical_scroll: false,
        maximum_height: 560.0,
        image: None,
    };
    fixture.application.update(cx, |app, cx| {
        app.preview.visible = true;
        app.begin_reused_preview_presentation(next, cx);
    });
    let policy = receive(&fixture, cx);
    confirm(&fixture, &policy, true, cx);
    let offscreen = receive(&fixture, cx);
    confirm(&fixture, &offscreen, true, cx);
    let bounds = receive(&fixture, cx);
    assert!(
        matches!(
            bounds.message,
            Message::PlatformCommand(PlatformCommand::SetWindowBounds(SetWindowBoundsCommand {
                window_handle: 88,
                width: 238,
                height: 95,
                ..
            }))
        ),
        "reuse must acknowledge exact new device size, not race GPUI resize with reveal"
    );
    assert!(
        fixture
            .application
            .read_with(cx, |app, _| app.preview.displayed_item_id.is_none())
    );
    fixture.shutdown.shutdown().expect("owned worker closed");
}

#[gpui_kit::test]
fn round2_reused_bounds_failure_timeout_cancel_and_old_hwnd_never_reveal(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let fixture = setup(cx);
    for terminal in ["failure", "timeout", "cancel", "old_hwnd"] {
        fixture.application.update(cx, |app, cx| {
            app.preview.native_window_handle = 88;
            app.preview.generation = 4;
            app.preview.hovered_item_id = Some(7);
            app.preview.visible = true;
            app.begin_reused_preview_presentation(stage(), cx);
        });
        let policy = receive(&fixture, cx);
        confirm(&fixture, &policy, true, cx);
        let movement = receive(&fixture, cx);
        confirm(&fixture, &movement, true, cx);
        let bounds = receive(&fixture, cx);
        assert!(matches!(
            bounds.message,
            Message::PlatformCommand(PlatformCommand::SetWindowBounds(_))
        ));
        match terminal {
            "failure" => confirm(&fixture, &bounds, false, cx),
            "timeout" => {
                cx.run_until_parked();
                cx.executor().advance_clock(PRESENTATION_STEP_TIMEOUT);
                cx.run_until_parked();
            }
            "cancel" => {
                fixture
                    .application
                    .update(cx, |app, cx| app.dismiss_preview(cx));
            }
            _ => {
                fixture.application.update(cx, |app, _| {
                    app.preview.native_window_handle = 99;
                    app.preview.visible = true;
                });
                confirm(&fixture, &bounds, true, cx);
            }
        }
        assert!(
            fixture
                .application
                .read_with(cx, |app, _| app.preview.pending_stage.is_none()
                    && app.preview.displayed_item_id.is_none())
        );
        while let Ok(envelope) = fixture.endpoint.try_receive_command_for_contract_test() {
            assert!(matches!(
                envelope.message,
                Message::PlatformCommand(PlatformCommand::SetWindowVisibility(
                    SetWindowVisibilityCommand {
                        window_handle: 88,
                        visible: false
                    }
                ))
            ));
        }
        if terminal != "old_hwnd" {
            assert!(!fixture.application.update(cx, |app, cx| {
                app.handle_preview_stage_result(bounds.id, true, None, cx)
            }));
        } else {
            assert!(
                fixture
                    .application
                    .read_with(cx, |app, _| app.preview.visible),
                "old bounds cannot hide replacement HWND"
            );
        }
    }
    fixture.shutdown.shutdown().expect("close owned worker");
}

#[gpui_kit::test]
fn height_feedback_measured_content_reaches_native_bounds_before_show(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.executor().allow_parking();
    let fixture = setup(cx);
    let mut next = stage();
    next.item.text = "第一段\n\n第二段\n\n第三段\n\n最后一行".into();
    next.content_measured = false;
    let item = next.item.clone();
    let layout = next.layout;
    let owner = fixture.application.downgrade();
    let retained = Rc::new(RefCell::new(None));
    let slot = retained.clone();
    let (_, surface) = cx.add_window_view(|window, cx| {
        let view = cx.new(|cx| ClipboardPreview::new(item, layout, owner, window, cx));
        *slot.borrow_mut() = Some(view.downgrade());
        gpui_kit::base::Root::new(view, window, cx)
    });
    let window_handle = surface.update(|window, _| window.window_handle());
    fixture.application.update(cx, |owner, cx| {
        owner.preview.window = Some(PreviewWindow {
            window: window_handle,
            view: retained.borrow().clone().expect("view"),
        });
        owner.begin_preview_content_measurement(next, true, cx);
        assert!(
            owner
                .preview
                .pending_stage
                .as_ref()
                .is_some_and(|stage| stage.phase == PreviewStagePhase::MeasureInitial)
        );
    });
    cx.run_until_parked();
    let policy = receive(&fixture, cx);
    assert!(matches!(
        policy.message,
        Message::PlatformCommand(PlatformCommand::SetWindowNoActivate(_))
    ));
    let height = fixture.application.read_with(cx, |owner, _| {
        let staged = owner
            .preview
            .pending_stage
            .as_ref()
            .expect("native pending");
        assert!(staged.content_measured);
        assert!(staged.layout.body_height > 0.0);
        staged.layout.window_height
    });
    confirm(&fixture, &policy, true, cx);
    let bounds = receive(&fixture, cx);
    match &bounds.message {
        Message::PlatformCommand(PlatformCommand::SetWindowBounds(command)) => {
            assert_eq!(command.height, (height * 1.25).round() as i32)
        }
        _ => panic!("measured size must go through existing native bounds"),
    }
    {
        let surface = gpui_kit::VisualTestContext::from_window(window_handle, cx);
        surface.simulate_resize(size(px(400.0), px(height)));
    }
    confirm(&fixture, &bounds, true, cx);
    {
        let mut surface = gpui_kit::VisualTestContext::from_window(window_handle, cx);
        let body = surface
            .debug_bounds("clipboard-preview-markdown")
            .expect("prepared body");
        let footer = surface
            .debug_bounds("clipboard-preview-metadata")
            .expect("prepared footer");
        assert!(
            body.bottom() <= footer.top() && footer.bottom() <= px(height),
            "new native client has a complete prepared frame before show"
        );
    }
    cx.run_until_parked();
    let topmost = receive(&fixture, cx);
    assert!(matches!(
        topmost.message,
        Message::PlatformCommand(PlatformCommand::SetWindowTopmost(_))
    ));
    confirm(&fixture, &topmost, true, cx);
    let show = receive(&fixture, cx);
    assert!(matches!(
        show.message,
        Message::PlatformCommand(PlatformCommand::SetWindowVisibility(
            SetWindowVisibilityCommand { visible: true, .. }
        ))
    ));
    confirm(&fixture, &show, true, cx);
    fixture.application.update(cx, |owner, cx| {
        owner.preview.pending_stage = None;
        let mut replacement = stage();
        replacement.item.text = "---".into();
        replacement.content_measured = false;
        owner.begin_preview_content_measurement(replacement, false, cx);
    });
    cx.run_until_parked();
    let replacement = receive(&fixture, cx);
    assert!(
        matches!(
            replacement.message,
            Message::PlatformCommand(PlatformCommand::SetWindowBounds(_))
        ),
        "reuse measures then sends native size, not show"
    );
    fixture.application.read_with(cx, |owner, _| {
        let pending = owner
            .preview
            .pending_stage
            .as_ref()
            .expect("new native bounds pending");
        assert_eq!(pending.item.text, "---");
        assert!(pending.content_measured);
        assert!(
            pending.layout.window_height < height,
            "new short content replaces previous height"
        );
    });
    fixture.shutdown.shutdown().expect("fixture worker");
}

#[gpui_kit::test]
fn height_feedback_cancelled_measurement_cannot_start_native_presentation(cx: &mut TestAppContext) {
    let fixture = setup(cx);
    let mut cancelled = stage();
    cancelled.command_id = uuid::Uuid::new_v4();
    cancelled.phase = PreviewStagePhase::MeasureInitial;
    let token = cancelled.command_id;
    let layout = cancelled.layout;
    fixture.application.update(cx, |owner, cx| {
        owner.preview.pending_stage = Some(cancelled);
        owner.dismiss_preview(cx);
        owner.complete_preview_content_measurement(token, layout, cx);
    });
    assert!(
        fixture
            .endpoint
            .try_receive_command_for_contract_test()
            .is_err()
    );
    fixture.shutdown.shutdown().expect("fixture worker");
}

#[gpui_kit::test]
fn runtime_feedback_menu_cancels_visible_preview_and_late_ack(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let fixture = setup(cx);
    struct Source;
    impl gpui_kit::Render for Source {
        fn render(
            &mut self,
            _: &mut gpui_kit::Window,
            _: &mut gpui_kit::Context<Self>,
        ) -> impl gpui_kit::IntoElement {
            gpui_kit::div()
        }
    }
    let (_, source) = cx.add_window_view(|_, _| Source);
    let source_handle = source.update(|window, _| window.window_handle());
    fixture.application.update(cx, |owner, cx| {
        owner.begin_initial_preview_presentation(stage(), cx)
    });
    let request = receive(&fixture, cx);
    let (bridge, _endpoint) = nxtools_ipc::platform_bridge(4);
    let _token = cx.update(|cx| {
        crate::ui::components::native_context_menu::install_pending_for_test(
            source_handle,
            Rc::new(bridge),
            cx,
        )
    });
    fixture
        .application
        .update(cx, |owner, _| owner.preview.visible = true);
    fixture.application.update(cx, |owner, cx| {
        owner.handle_platform_notification(
            PlatformNotification::CommandResult {
                correlation_id: request.id,
                result: nxtools_protocol::PlatformCommandResult {
                    success: true,
                    error: None,
                    native_error_code: None,
                    data: None,
                },
            },
            cx,
        )
    });
    fixture.application.read_with(cx, |owner, _| {
        assert!(!owner.preview.visible);
        assert!(owner.preview.pending_stage.is_none());
        assert!(owner.preview.candidate.is_none());
    });
    fixture.shutdown.shutdown().expect("stop fixture");
}
