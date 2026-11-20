use super::*;
use gpui_kit::TestAppContext;
use nxtools_protocol::{Envelope, Message};

fn links() -> Vec<crate::tools::link_panel::DetectedLinkRow> {
    crate::tools::link_detector::detect_links("https://example.invalid/file")
}

fn fixture(
    cx: &mut TestAppContext,
) -> (
    tempfile::TempDir,
    Entity<ClipboardApplication>,
    nxtools_ipc::PlatformServerEndpoint,
) {
    cx.update(gpui_kit::init);
    cx.update(crate::theme::apply_component_theme);
    cx.executor().allow_parking();
    let temporary = tempfile::tempdir().expect("link placement fixture");
    let worker = crate::clipboard_worker::ClipboardCaptureWorkerHandle::spawn(
        temporary.path().join("history.db"),
        temporary.path().join("assets"),
        ClipboardRetentionPolicy {
            max_items: 100,
            retention_days: 30,
        },
        1000,
    )
    .expect("fixture worker");
    let config = AppConfig::default();
    let path = temporary.path().join("config.json");
    config.save_to(&path).expect("fixture config");
    let (platform, endpoint) = nxtools_ipc::platform_bridge(64);
    endpoint.set_connected_for_contract_test(true);
    let owner = cx.update(|cx| {
        let notifications = cx.new(|_| NotificationCenter::default());
        cx.new(|cx| {
            ClipboardApplication::new(worker, Rc::new(platform), config, path, notifications, cx)
        })
    });
    owner.update(cx, |owner, _| {
        owner.link_panel.test_native_window_handle = Some(9001)
    });
    (temporary, owner, endpoint)
}

fn command(endpoint: &nxtools_ipc::PlatformServerEndpoint) -> Envelope {
    endpoint
        .try_receive_command_for_contract_test()
        .expect("native stage command")
}

fn reply(
    cx: &mut TestAppContext,
    owner: &Entity<ClipboardApplication>,
    envelope: &Envelope,
    data: Option<String>,
) {
    owner.update(cx, |owner, cx| {
        owner.handle_platform_notification(
            PlatformNotification::CommandResult {
                correlation_id: envelope.id,
                result: PlatformCommandResult {
                    success: true,
                    error: None,
                    data,
                    native_error_code: None,
                },
            },
            cx,
        )
    });
}

#[gpui_kit::test]
fn multimonitor_link_first_show_and_reuse_use_current_work_area_without_activation(
    cx: &mut TestAppContext,
) {
    let (_temporary, owner, endpoint) = fixture(cx);
    let mut original = None;
    for (left, top, dpi) in [(-1920, -1080, 120), (2560, 40, 144)] {
        owner.update(cx, |owner, cx| owner.request_link_panel(links(), cx));
        let query = command(&endpoint);
        assert!(matches!(
            query.message,
            Message::PlatformCommand(PlatformCommand::QueryCursorMonitor)
        ));
        let monitor = CursorMonitorSnapshot {
            display_id: 0xCAFE,
            work_left: left,
            work_top: top,
            work_width: 1920,
            work_height: 1040,
            dpi,
        };
        reply(
            cx,
            &owner,
            &query,
            Some(serde_json::to_string(&monitor).expect("monitor wire")),
        );
        if original.is_some() {
            let hide = command(&endpoint);
            assert!(matches!(
                hide.message,
                Message::PlatformCommand(PlatformCommand::SetWindowVisibility(
                    SetWindowVisibilityCommand { visible: false, .. }
                ))
            ));
            reply(cx, &owner, &hide, None);
        }
        let window = owner.read_with(cx, |owner, _| {
            owner.link_panel.window.expect("hidden panel")
        });
        if let Some(original) = original {
            assert_eq!(window, original);
        } else {
            original = Some(window);
        }
        let noactivate = command(&endpoint);
        assert!(matches!(
            noactivate.message,
            Message::PlatformCommand(PlatformCommand::SetWindowNoActivate(
                SetWindowNoActivateCommand { enabled: true, .. }
            ))
        ));
        reply(cx, &owner, &noactivate, None);
        let stage = command(&endpoint);
        assert!(matches!(
            stage.message,
            Message::PlatformCommand(PlatformCommand::SetWindowBounds(SetWindowBoundsCommand {
                x: PREVIEW_STAGING_COORDINATE,
                ..
            }))
        ));
        reply(cx, &owner, &stage, None);
        let topmost = command(&endpoint);
        assert!(matches!(
            topmost.message,
            Message::PlatformCommand(PlatformCommand::SetWindowTopmost(_))
        ));
        reply(cx, &owner, &topmost, None);
        let show = command(&endpoint);
        assert!(matches!(
            show.message,
            Message::PlatformCommand(PlatformCommand::SetWindowVisibility(
                SetWindowVisibilityCommand { visible: true, .. }
            ))
        ));
        reply(cx, &owner, &show, None);
        assert!(endpoint.try_receive_command_for_contract_test().is_err());
        cx.update(|cx| {
            window
                .update(cx, |_, window, cx| {
                    window.draw(cx).clear(cx);
                    window.simulate_next_frame(cx);
                })
                .expect("submit staged frame")
        });
        cx.run_until_parked();
        let hide = command(&endpoint);
        assert!(matches!(
            hide.message,
            Message::PlatformCommand(PlatformCommand::SetWindowVisibility(
                SetWindowVisibilityCommand { visible: false, .. }
            ))
        ));
        reply(cx, &owner, &hide, None);
        let target = command(&endpoint);
        assert!(matches!(
            target.message,
            Message::PlatformCommand(PlatformCommand::SetWindowPosition(_))
        ));
        reply(cx, &owner, &target, None);
        let reveal = command(&endpoint);
        let Message::PlatformCommand(PlatformCommand::SetWindowBounds(bounds)) = &reveal.message
        else {
            panic!("final atomic bounds");
        };
        let scale = dpi as f32 / 96.0;
        assert_eq!(
            bounds.x,
            left + 1920 - (420.0 * scale) as i32 - (24.0 * scale) as i32
        );
        assert_eq!(bounds.y, top + (24.0 * scale) as i32);
        assert_eq!(bounds.width, (420.0 * scale) as i32);
        reply(cx, &owner, &reveal, None);
        let show = command(&endpoint);
        assert!(matches!(
            show.message,
            Message::PlatformCommand(PlatformCommand::SetWindowVisibility(
                SetWindowVisibilityCommand { visible: true, .. }
            ))
        ));
        reply(cx, &owner, &show, None);
        assert!(owner.read_with(cx, |owner, _| owner.pending_link_placement.is_none()));
        assert!(endpoint.try_receive_command_for_contract_test().is_err());
    }
}

#[gpui_kit::test]
fn multimonitor_link_close_or_native_failure_retires_pending_reveal(cx: &mut TestAppContext) {
    let (_temporary, owner, endpoint) = fixture(cx);
    owner.update(cx, |owner, cx| owner.request_link_panel(links(), cx));
    let query = command(&endpoint);
    owner.update(cx, |owner, _| owner.forget_link_panel());
    reply(
        cx,
        &owner,
        &query,
        Some(
            serde_json::to_string(&CursorMonitorSnapshot {
                display_id: 9,
                work_left: 100,
                work_top: 40,
                work_width: 1920,
                work_height: 1040,
                dpi: 120,
            })
            .expect("monitor wire"),
        ),
    );
    assert!(
        owner.read_with(cx, |owner, _| owner.pending_link_placement.is_none()
            && owner.link_panel.window.is_none())
    );
    assert!(endpoint.try_receive_command_for_contract_test().is_err());
    owner.update(cx, |owner, cx| owner.request_link_panel(links(), cx));
    let query = command(&endpoint);
    owner.update(cx, |owner, cx| {
        owner.handle_platform_notification(
            PlatformNotification::CommandResult {
                correlation_id: query.id,
                result: PlatformCommandResult {
                    success: false,
                    error: None,
                    data: None,
                    native_error_code: Some(5),
                },
            },
            cx,
        )
    });
    assert!(owner.read_with(cx, |owner, _| owner.pending_link_placement.is_none()));
    assert!(endpoint.try_receive_command_for_contract_test().is_err());
}

#[gpui_kit::test]
fn link_panel_expander_applies_complete_native_bounds_on_the_same_monitor(cx: &mut TestAppContext) {
    use gpui_kit::VisualTestContext;
    let (_temporary, owner, endpoint) = fixture(cx);
    let monitor = CursorMonitorSnapshot {
        display_id: 0xCAFE,
        work_left: -1920,
        work_top: -1080,
        work_width: 1920,
        work_height: 360,
        dpi: 144,
    };
    owner.update(cx, |owner, _| {
        owner.config.clipboard.link_detector_enabled = true;
        owner.link_panel.monitor = Some(monitor);
    });
    for count in [3, 32] {
        let targets = (0..count)
            .map(|index| crate::tools::link_detector::DetectedLink {
                display: format!("https://example.invalid/{index}"),
                copy_value: format!("https://example.invalid/{index}"),
                kind: crate::tools::link_detector::DetectedLinkKind::Web,
            })
            .collect();
        owner.update(cx, |owner, cx| owner.show_link_panel(targets, cx));
        let handle = owner.read_with(cx, |owner, _| owner.link_panel.window.expect("link panel"));
        {
            let mut window = VisualTestContext::from_window(handle, cx);
            window.simulate_resize(size(
                px(420.0),
                px(crate::tools::link_panel::link_panel_height(count, false)),
            ));
            window.refresh().expect("collapsed production panel");
            let expander = window
                .debug_bounds("link-panel-expander")
                .expect("expand action");
            window.simulate_click(expander.center(), gpui_kit::Modifiers::default());
            window.run_until_parked();
        }
        let resize = command(&endpoint);
        let Message::PlatformCommand(PlatformCommand::SetWindowBounds(bounds)) = &resize.message
        else {
            panic!("expansion must use the native client-bounds owner");
        };
        assert_eq!(bounds.window_handle, 9001);
        assert_eq!(bounds.width, 630);
        let expected_height =
            (crate::tools::link_panel::link_panel_height(count, true) * 1.5).round() as i32;
        assert_eq!(bounds.height, expected_height.min(360));
        assert!(bounds.x >= -1920 && bounds.x + bounds.width <= 0);
        assert!(bounds.y >= -1080 && bounds.y + bounds.height <= -720);
        reply(cx, &owner, &resize, None);
        assert!(owner.read_with(cx, |owner, _| owner.pending_link_placement.is_none()));
        assert!(endpoint.try_receive_command_for_contract_test().is_err());
        let mut window = VisualTestContext::from_window(handle, cx);
        window.simulate_resize(size(px(420.0), px(bounds.height as f32 / 1.5)));
        window.refresh().expect("expanded native client viewport");
        let footer = window
            .debug_bounds("link-panel-footer")
            .expect("count footer");
        assert!(footer.bottom() <= px(bounds.height as f32 / 1.5));
    }
}
