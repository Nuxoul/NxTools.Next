//! Synthetic production observer -> deferred owner -> locked temp-config regressions.
use super::*;
use gpui_kit::{TestAppContext, VisualTestContext};
use nxtools_core::{ClipboardModifierActions, ClipboardPanelSize};

struct Fixture {
    application: Entity<ClipboardApplication>,
    endpoint: nxtools_ipc::PlatformServerEndpoint,
    worker: crate::clipboard_worker::ClipboardCaptureWorkerHandle,
    directory: tempfile::TempDir,
}
fn setup(cx: &mut TestAppContext) -> Fixture {
    cx.update(gpui_kit::init);
    cx.update(crate::theme::apply_component_theme);
    let directory = tempfile::tempdir().expect("synthetic clipboard storage");
    let config_path = directory.path().join("config.json");
    AppConfig::default()
        .save_to(&config_path)
        .expect("seed config");
    let worker = crate::clipboard_worker::ClipboardCaptureWorkerHandle::spawn(
        directory.path().join("history.sqlite"),
        directory.path().join("assets"),
        ClipboardRetentionPolicy {
            max_items: 1000,
            retention_days: 30,
        },
        1000,
    )
    .expect("temporary worker");
    let (platform, endpoint) = nxtools_ipc::platform_bridge(32);
    let _ = endpoint.open_connection_for_contract_test();
    let application = cx.new(|cx| {
        let notifications = cx.new(|_| NotificationCenter::default());
        ClipboardApplication::new(
            worker.clone(),
            Rc::new(platform),
            AppConfig::default(),
            config_path,
            notifications,
            cx,
        )
    });
    while endpoint.try_receive_command_for_contract_test().is_ok() {}
    Fixture {
        application,
        endpoint,
        worker,
        directory,
    }
}
fn panel(fixture: &Fixture, cx: &mut TestAppContext, revealed: bool) -> VisualTestContext {
    let session_id = uuid::Uuid::new_v4();
    let application = fixture.application.downgrade();
    let worker = fixture.worker.clone();
    let initial = fixture
        .application
        .read_with(cx, |app, _| app.config.clipboard_panel);
    let panel_slot = Rc::new(RefCell::new(None));
    let retained_panel = panel_slot.clone();
    let (_root, surface) = cx.add_window_view(move |window, cx| {
        window.resize(size(px(initial.width as f32), px(initial.height as f32)));
        let panel = cx.new(|cx| {
            ClipboardPanel::new(
                ClipboardHistoryPage {
                    items: Vec::new(),
                    total: 0,
                    has_more: false,
                },
                Vec::new(),
                HashMap::new(),
                Vec::new(),
                ClipboardModifierActions::default(),
                ForegroundContext {
                    window_handle: 1,
                    focus_window_handle: 1,
                    process_id: 1,
                    process_name: String::new(),
                    window_title: String::new(),
                },
                worker,
                application,
                session_id,
                None,
                window,
                cx,
            )
        });
        *retained_panel.borrow_mut() = Some(panel.clone());
        gpui_kit::base::Root::new(panel, window, cx)
    });
    let panel = panel_slot
        .borrow()
        .clone()
        .expect("production panel entity");
    let handle = surface.update(|window, _| window.window_handle());
    // TestWindow::resize sets native bounds only; drive the actual resize callback as WM_SIZE would.
    surface.simulate_resize(size(px(initial.width as f32), px(initial.height as f32)));
    surface.run_until_parked();
    if revealed {
        panel.update_in(surface, |panel, window, _| {
            panel.enable_size_tracking(window)
        });
    }
    fixture.application.update(cx, |app, _| {
        app.popup
            .lifecycle
            .replace_without_topmost_confirmation(PopupWindow {
                window: handle,
                panel: panel.downgrade(),
                native_window_handle: 88,
                session_id,
                summon_requested_at_unix_ms: 0,
                summon_event_sequence: 0,
            });
        app.popup.size_tracking_session = revealed.then_some(session_id);
    });
    VisualTestContext::from_window(handle, cx)
}
fn settle(cx: &mut TestAppContext) {
    cx.run_until_parked();
    for _ in 0..3 {
        cx.executor().advance_clock(PANEL_SIZE_SAVE_DELAY);
        cx.run_until_parked();
    }
}
#[gpui_kit::test]
fn clipboard_resize_observer_persists_latest_only_and_reopens_logically_at_125_150(
    cx: &mut TestAppContext,
) {
    let fixture = setup(cx);
    let surface = panel(&fixture, cx, true);
    let config_path = fixture.directory.path().join("config.json");
    AppConfig::update_section(&config_path, |config| {
        config.clipboard.retention_days = 47;
        config.system.theme = nxtools_core::ThemePreference::Light;
        config.clipboard.storage_directory = "synthetic-migrated-root".into();
        config.clipboard_panel = ClipboardPanelSize::clamped(310, 450);
    })
    .expect("concurrent owner writes");
    for scale in [1.25, 1.5] {
        surface.simulate_scale_factor_change(scale);
        surface.simulate_resize(size(px(512.0), px(688.0)));
        surface.run_until_parked();
        assert_eq!(
            fixture
                .application
                .read_with(cx, |app, _| app.config.clipboard_panel),
            ClipboardPanelSize::clamped(512, 688)
        );
    }
    // A rapid drag burst must keep disk unchanged until the latest stable sample.
    surface.simulate_resize(size(px(530.0), px(700.0)));
    surface.simulate_resize(size(px(512.0), px(688.0)));
    surface.run_until_parked();
    assert_eq!(
        AppConfig::load_from(&config_path)
            .expect("read before debounce")
            .clipboard_panel
            .width,
        310
    );
    let popup = fixture.application.read_with(cx, |app, _| {
        app.popup.lifecycle.current().cloned().expect("real panel")
    });
    fixture.application.update(cx, |app, _| {
        app.hide_popup(popup.window, 88, popup.session_id, PopupCloseReason::Escape)
            .expect("synthetic hide queued");
    });
    let _ = fixture
        .endpoint
        .try_receive_command_for_contract_test()
        .expect("native hide command");
    surface.simulate_resize(size(px(1920.0), px(1080.0)));
    surface.run_until_parked();
    let mut reopened = panel(&fixture, cx, true);
    // Reopen initializes directly from memory; no forced test size can hide a stale restore.
    assert_eq!(
        reopened.update(|window, _| window.viewport_size()),
        size(px(512.0), px(688.0))
    );
    settle(cx);
    let saved = AppConfig::load_from(&config_path).expect("restart reads durable size");
    assert_eq!(saved.clipboard_panel, ClipboardPanelSize::clamped(512, 688));
    assert_eq!(saved.clipboard.retention_days, 47);
    assert_eq!(saved.clipboard.storage_directory, "synthetic-migrated-root");
    assert_eq!(saved.system.theme, nxtools_core::ThemePreference::Light);
    fixture.worker.shutdown().expect("close synthetic worker");
}
#[gpui_kit::test]
fn clipboard_resize_ignores_hidden_startup_and_retired_panel_sessions(cx: &mut TestAppContext) {
    let fixture = setup(cx);
    let old = panel(&fixture, cx, false);
    old.simulate_resize(size(px(1920.0), px(1080.0)));
    old.run_until_parked();
    assert_eq!(
        fixture
            .application
            .read_with(cx, |app, _| app.config.clipboard_panel),
        ClipboardPanelSize::default()
    );
    let old_panel = fixture.application.read_with(cx, |app, _| {
        app.popup
            .lifecycle
            .current()
            .expect("old popup")
            .panel
            .clone()
    });
    let mut old_surface = old;
    old_surface.update(|window, cx| {
        old_panel
            .update(cx, |panel, _| panel.enable_size_tracking(window))
            .expect("arm old panel");
    });
    let current = panel(&fixture, cx, true);
    old_surface.simulate_resize(size(px(700.0), px(800.0)));
    current.simulate_resize(size(px(420.0), px(560.0)));
    settle(cx);
    assert_eq!(
        AppConfig::load_from(&fixture.directory.path().join("config.json"))
            .expect("saved config")
            .clipboard_panel,
        ClipboardPanelSize::clamped(420, 560)
    );
    fixture.worker.shutdown().expect("close synthetic worker");
}
#[gpui_kit::test]
fn clipboard_resize_save_failure_keeps_session_size_and_visible_error(cx: &mut TestAppContext) {
    let fixture = setup(cx);
    let surface = panel(&fixture, cx, true);
    std::fs::write(fixture.directory.path().join("config.json"), "not-json")
        .expect("synthetic corrupt config");
    surface.simulate_resize(size(px(510.0), px(680.0)));
    settle(cx);
    fixture.application.read_with(cx, |app, cx| {
        assert_eq!(
            app.config.clipboard_panel,
            ClipboardPanelSize::clamped(510, 680)
        );
        assert!(
            app.notifications
                .read(cx)
                .items()
                .iter()
                .any(|item| item.id.as_ref() == "clipboard.panel_size")
        );
    });
    assert_eq!(
        std::fs::read_to_string(fixture.directory.path().join("config.json"))
            .expect("unchanged corrupt config"),
        "not-json"
    );
    fixture.worker.shutdown().expect("close synthetic worker");
}

fn pause_result(paused: bool) -> nxtools_protocol::PlatformCommandResult {
    nxtools_protocol::PlatformCommandResult {
        success: true,
        error: None,
        native_error_code: None,
        data: Some(format!("{{\"capture_paused\":{paused}}}")),
    }
}
#[gpui_kit::test]
fn capture_pause_ack_failure_reconnect_and_inflight_capture_cleanup(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let f = setup(cx);
    let surface = panel(&f, cx, true);
    f.application
        .update(cx, |app, cx| app.set_capture_pause(true, false, cx));
    let first = f
        .endpoint
        .try_receive_command_for_contract_test()
        .expect("independent pause command");
    assert!(matches!(
        first.message,
        nxtools_protocol::Message::PlatformCommand(PlatformCommand::SetClipboardCapturePaused(
            nxtools_protocol::SetClipboardCapturePausedCommand { paused: true }
        ))
    ));
    f.application.read_with(cx, |app, _| {
        let status = app.capture_pause_snapshot();
        assert!(status.pending);
        assert_ne!(status.applied, Some(true));
    });
    let staging = f.directory.path().join("synthetic-staging.bin");
    std::fs::write(&staging, b"synthetic").expect("staging");
    f.application.update(cx, |app, cx| {
        app.management_operation_active = true;
        app.handle_platform_notification(
            PlatformNotification::Event(PlatformEvent::ClipboardCaptured(ClipboardCapture {
                text: Some("synthetic secret".into()),
                image: None,
                paths: vec![],
                native_formats: vec![],
                source_icon_staging_path: Some(staging.to_string_lossy().into()),
                source: ForegroundContext {
                    window_handle: 0,
                    focus_window_handle: 0,
                    process_id: 0,
                    process_name: String::new(),
                    window_title: String::new(),
                },
                captured_at_unix_ms: nxtools_ipc::unix_time_ms(),
            })),
            cx,
        );
        assert!(
            app.deferred_captures.is_empty(),
            "pause guard precedes management deferral"
        );
        app.management_operation_active = false;
        app.handle_platform_notification(
            PlatformNotification::CommandResult {
                correlation_id: first.id,
                result: pause_result(true),
            },
            cx,
        );
    });
    assert!(!staging.exists(), "rejected capture staging cleaned");
    cx.run_until_parked();
    f.application.read_with(cx, |app, cx| {
        assert_eq!(app.capture_pause_snapshot().applied, Some(true));
        let current = app
            .popup
            .lifecycle
            .current()
            .expect("real panel registration");
        assert!(
            current
                .panel
                .read_with(cx, |panel, _| panel.capture_pause_test_snapshot().applied
                    == Some(true))
                .expect("panel alive")
        );
    });
    f.application
        .update(cx, |app, cx| app.set_capture_pause(false, false, cx));
    let resume = f
        .endpoint
        .try_receive_command_for_contract_test()
        .expect("resume command");
    f.application.update(cx, |app, cx| {
        app.handle_platform_notification(
            PlatformNotification::CommandResult {
                correlation_id: resume.id,
                result: nxtools_protocol::PlatformCommandResult {
                    success: false,
                    error: Some("fixture".into()),
                    native_error_code: Some(5),
                    data: None,
                },
            },
            cx,
        )
    });
    f.application
        .read_with(cx, |app, _| assert!(app.capture_pause_snapshot().failed));
    f.application.update(cx, |app, cx| {
        app.handle_platform_notification(
            PlatformNotification::Disconnected {
                generation: 1,
                pending_command_ids: vec![],
            },
            cx,
        )
    });
    f.application.update(cx, |app, cx| {
        app.handle_platform_notification(PlatformNotification::Connected, cx)
    });
    let reapplied = f
        .endpoint
        .try_receive_command_for_contract_test()
        .expect("reapply before ordinary configuration");
    f.application.update(cx, |app, cx| {
        app.handle_platform_notification(
            PlatformNotification::CommandResult {
                correlation_id: resume.id,
                result: pause_result(false),
            },
            cx,
        );
        assert!(
            app.capture_pause_snapshot().pending,
            "late result cannot retire reapply"
        );
        app.handle_platform_notification(
            PlatformNotification::CommandResult {
                correlation_id: reapplied.id,
                result: pause_result(false),
            },
            cx,
        );
        assert_eq!(app.capture_pause_snapshot().applied, Some(false));
    });
    drop(surface);
    f.worker.shutdown().expect("fixture worker");
}

#[gpui_kit::test]
fn capture_pause_deadline_and_timeout_are_generation_scoped(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let f = setup(cx);
    f.application
        .update(cx, |app, cx| app.set_capture_pause(true, true, cx));
    let first = f
        .endpoint
        .try_receive_command_for_contract_test()
        .expect("timed pause");
    cx.run_until_parked();
    cx.executor().advance_clock(Duration::from_secs(6));
    cx.run_until_parked();
    f.application.read_with(cx, |app, _| {
        let status = app.capture_pause_snapshot();
        assert!(status.failed);
        assert!(!status.pending);
    });
    f.application
        .update(cx, |app, cx| app.set_capture_pause(true, false, cx));
    let manual = f
        .endpoint
        .try_receive_command_for_contract_test()
        .expect("manual retry");
    f.application.update(cx, |app, cx| {
        app.handle_platform_notification(
            PlatformNotification::CommandResult {
                correlation_id: manual.id,
                result: pause_result(true),
            },
            cx,
        )
    });
    cx.run_until_parked();
    cx.executor().advance_clock(Duration::from_secs(15 * 60));
    cx.run_until_parked();
    assert!(
        f.endpoint.try_receive_command_for_contract_test().is_err(),
        "old timed generation must not resume manual pause"
    );
    f.application
        .update(cx, |app, cx| app.set_capture_pause(true, true, cx));
    let current = f
        .endpoint
        .try_receive_command_for_contract_test()
        .expect("new timed pause");
    f.application.update(cx, |app, cx| {
        app.handle_platform_notification(
            PlatformNotification::CommandResult {
                correlation_id: current.id,
                result: pause_result(true),
            },
            cx,
        )
    });
    cx.run_until_parked();
    cx.executor().advance_clock(Duration::from_secs(15 * 60));
    cx.run_until_parked();
    let resume = f
        .endpoint
        .try_receive_command_for_contract_test()
        .expect("deadline dispatches resume");
    assert!(matches!(
        resume.message,
        nxtools_protocol::Message::PlatformCommand(PlatformCommand::SetClipboardCapturePaused(
            nxtools_protocol::SetClipboardCapturePausedCommand { paused: false }
        ))
    ));
    f.application.update(cx, |app, cx| {
        app.handle_platform_notification(
            PlatformNotification::CommandResult {
                correlation_id: first.id,
                result: pause_result(true),
            },
            cx,
        )
    });
    f.application
        .read_with(cx, |app, _| assert!(app.capture_pause_snapshot().pending));
    f.worker.shutdown().expect("fixture worker");
}

#[gpui_kit::test]
fn text_card_height_save_publishes_next_turn_and_failure_keeps_current_panel(
    cx: &mut TestAppContext,
) {
    let fixture = setup(cx);
    let mut surface = panel(&fixture, cx, true);
    let current = fixture.application.read_with(&surface, |app, _| {
        app.popup
            .lifecycle
            .current()
            .cloned()
            .expect("mounted panel")
    });
    let panel = current.panel.upgrade().expect("panel entity");
    let leased_panel = panel.clone();
    fixture.application.update(&mut surface, |app, cx| {
        app.set_text_card_max_height(210, cx);
        assert_eq!(
            leased_panel.read(cx).text_card_max_height_for_test(),
            160,
            "no synchronous cross-entity publication"
        );
    });
    surface.run_until_parked();
    assert_eq!(
        panel.read_with(&surface, |panel, _| panel.text_card_max_height_for_test()),
        210
    );
    let path = fixture.directory.path().join("config.json");
    assert_eq!(
        AppConfig::load_from(&path)
            .expect("committed before publish")
            .clipboard
            .text_card_max_height,
        210
    );
    assert!(
        fixture
            .endpoint
            .try_receive_command_for_contract_test()
            .is_err()
    );
    std::fs::write(&path, b"{broken").expect("failed commit fixture");
    fixture.application.update(&mut surface, |app, cx| {
        app.set_text_card_max_height(320, cx)
    });
    surface.run_until_parked();
    assert_eq!(
        panel.read_with(&surface, |panel, _| panel.text_card_max_height_for_test()),
        210
    );
    assert_eq!(
        fixture.application.read_with(&surface, |app, _| app
            .clipboard_settings()
            .text_card_max_height),
        210
    );
    assert_eq!(std::fs::read(&path).expect("no overwrite"), b"{broken");
    fixture
        .worker
        .clone()
        .shutdown()
        .expect("shutdown synthetic worker");
}

#[gpui_kit::test]
fn input_followup_reveal_ack_requires_native_activation_before_search_focus(
    cx: &mut TestAppContext,
) {
    let fixture = setup(cx);
    let mut surface = panel(&fixture, cx, false);
    let reveal_id = uuid::Uuid::new_v4();
    let popup = fixture.application.read_with(cx, |app, _| {
        app.popup.lifecycle.current().cloned().expect("popup")
    });
    fixture.application.update(cx, |app, cx| {
        use nxtools_protocol::window_material::{
            AppliedWindowMaterial, EffectiveWindowMaterial, MaterialFallbackReason,
            MaterialSurface, SystemMaterialCapabilitiesChanged, WindowMaterialStyle,
        };
        let facts = cx.default_global::<crate::ui::material::UiMaterialCapabilities>();
        facts.connected();
        assert!(facts.apply(SystemMaterialCapabilitiesChanged {
            correlation_id: uuid::Uuid::new_v4(),
            session_id: uuid::Uuid::new_v4(),
            revision: 1,
            supported: true,
            transparency_enabled: true,
            high_contrast: false,
            energy_saver: false,
            remote_session: false,
            native_error_code: 0,
            system_backdrop: true,
            rounded_corners: true,
        }));
        let command = crate::ui::material::command(
            popup.native_window_handle,
            MaterialSurface::Transient,
            cx,
        )
        .expect("synthetic current material identity");
        let receipt = AppliedWindowMaterial {
            session_id: command.session_id,
            facts_revision: command.facts_revision,
            window_handle: command.window_handle,
            generation: command.generation,
            style: command.style,
            surface: command.surface,
            dark: command.dark,
            effective: match command.style {
                WindowMaterialStyle::Solid => EffectiveWindowMaterial::Solid,
                _ => EffectiveWindowMaterial::Acrylic,
            },
            fallback_reason: MaterialFallbackReason::None,
            native_error_code: 0,
        };
        app.popup.material =
            crate::ui::material::Preparation::confirmed(uuid::Uuid::new_v4(), command, receipt, cx)
                .expect("activation fixture starts after material confirmation");
    });
    surface.update(|window, cx| window.blur(cx));
    fixture.application.update(cx, |app, cx| {
        app.popup.pending_visibility.insert(
            reveal_id,
            PendingPopupVisibility {
                window: popup.window,
                native_window_handle: popup.native_window_handle,
                visible: true,
                session_id: popup.session_id,
                close_reason: None,
                started_at: Instant::now(),
            },
        );
        app.handle_platform_notification(
            PlatformNotification::CommandResult {
                correlation_id: reveal_id,
                result: nxtools_protocol::PlatformCommandResult {
                    success: true,
                    error: None,
                    data: None,
                    native_error_code: None,
                },
            },
            cx,
        );
    });
    cx.run_until_parked();
    let activation = fixture
        .endpoint
        .try_receive_command_for_contract_test()
        .expect("SW_SHOWNOACTIVATE requires explicit primary activation");
    assert!(
        matches!(activation.message, nxtools_protocol::Message::PlatformCommand(PlatformCommand::ActivateWindow(command)) if command.window_handle == 88)
    );
    assert!(
        fixture
            .endpoint
            .try_receive_command_for_contract_test()
            .is_err(),
        "one activation only"
    );
    fixture.application.update(cx, |app, cx| {
        app.handle_platform_notification(
            PlatformNotification::CommandResult {
                correlation_id: activation.id,
                result: nxtools_protocol::PlatformCommandResult {
                    success: true,
                    error: None,
                    data: None,
                    native_error_code: None,
                },
            },
            cx,
        )
    });
    cx.run_until_parked();
    assert!(
        surface.update(|window, cx| window.focused(cx).is_some()),
        "activation completion leaves the original input focus ready"
    );
    fixture.worker.shutdown().expect("close worker");
}

#[gpui_kit::test]
fn input_followup_activation_retirement_and_failures_cannot_refocus(cx: &mut TestAppContext) {
    let fixture = setup(cx);
    for terminal in ["hide", "replace", "disconnect", "failure", "timeout"] {
        let mut surface = panel(&fixture, cx, true);
        let popup = fixture.application.read_with(cx, |app, _| {
            app.popup.lifecycle.current().cloned().expect("popup")
        });
        fixture.application.update(cx, |app, cx| {
            app.request_popup_activation(popup.clone(), cx)
        });
        let activation = fixture
            .endpoint
            .try_receive_command_for_contract_test()
            .expect("activation");
        assert!(
            fixture
                .application
                .read_with(cx, |app, _| app.popup.pending_activation.is_some())
        );
        surface.update(|window, cx| window.focus_next(cx));
        let focus_before = surface.update(|window, cx| window.focused(cx));
        match terminal {
            "hide" => {
                fixture.application.update(cx, |app, _| {
                    app.hide_popup(popup.window, 88, popup.session_id, PopupCloseReason::Escape)
                        .expect("hide");
                });
            }
            "replace" => {
                fixture.application.update(cx, |app, _| {
                    let mut replacement = popup.clone();
                    replacement.session_id = uuid::Uuid::new_v4();
                    app.popup
                        .lifecycle
                        .replace_without_topmost_confirmation(replacement);
                });
            }
            "disconnect" => {
                fixture.application.update(cx, |app, cx| {
                    app.handle_platform_notification(
                        PlatformNotification::Disconnected {
                            generation: 1,
                            pending_command_ids: vec![activation.id],
                        },
                        cx,
                    )
                });
            }
            "failure" => {
                fixture.application.update(cx, |app, cx| {
                    app.finish_popup_activation(activation.id, false, Some(5), "native_result", cx);
                });
            }
            "timeout" => {
                fixture.application.update(cx, |app, cx| {
                    app.finish_popup_activation(activation.id, false, None, "timeout", cx);
                });
            }
            _ => unreachable!(),
        }
        fixture.application.update(cx, |app, cx| {
            app.finish_popup_activation(activation.id, true, None, "late_result", cx);
        });
        cx.run_until_parked();
        assert!(
            fixture
                .application
                .read_with(cx, |app, _| app.popup.pending_activation.is_none()),
            "{terminal} clears pending"
        );
        assert_eq!(
            surface.update(|window, cx| window.focused(cx)),
            focus_before,
            "{terminal} cannot restore search from a late result"
        );
        while fixture
            .endpoint
            .try_receive_command_for_contract_test()
            .is_ok()
        {}
    }
    fixture.worker.shutdown().expect("close worker");
}
