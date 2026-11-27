//! TestPlatform automation over production inputs, dialogs and the original IPC router.
use super::*;
use gpui_kit::{TestAppContext, VisualTestContext};
use nxtools_ipc::{PlatformServerEndpoint, platform_bridge};
use nxtools_protocol::{Envelope, Message};
use std::cell::RefCell;

fn mount(
    cx: &mut TestAppContext,
) -> (
    Entity<JoinWindow>,
    VisualTestContext,
    PlatformServerEndpoint,
) {
    cx.update(gpui_kit::init);
    cx.executor().allow_parking();
    let (bridge, server) = platform_bridge(32);
    server.set_connected_for_contract_test(true);
    let bridge = Rc::new(bridge);
    let capture = Rc::new(RefCell::new(None));
    let slot = capture.clone();
    let (_, surface) = cx.add_window_view(move |window, cx| {
        window.resize(size(px(680.), px(600.)));
        let view = cx.new(|cx| JoinWindow::new(Some(bridge), window, cx));
        *slot.borrow_mut() = Some(view.clone());
        cx.set_global(Target(view.downgrade(), window.window_handle()));
        gpui_kit::base::Root::new(view, window, cx)
    });
    let handle = surface.update(|window, _| window.window_handle());
    let mut surface = VisualTestContext::from_window(handle, cx);
    draw(&mut surface);
    let view = capture.borrow().clone().expect("production join window");
    (view, surface, server)
}
fn draw(surface: &mut VisualTestContext) {
    surface.run_until_parked();
    surface.update(|window, cx| {
        window.activate_window();
        window.draw(cx).clear(cx);
    });
}
fn click(surface: &mut VisualTestContext, selector: &'static str) {
    draw(surface);
    let point = surface
        .debug_bounds(selector)
        .expect("real control hitbox")
        .center();
    surface.simulate_click(point, gpui_kit::Modifiers::default());
    draw(surface);
}
fn next(server: &PlatformServerEndpoint) -> (Uuid, RawNumberedJoinCommand) {
    let Envelope { id, message, .. } = server
        .try_receive_command_for_contract_test()
        .expect("real raw operation queued");
    let Message::PlatformCommand(PlatformCommand::RawNumberedJoin(command)) = message else {
        panic!("raw join command");
    };
    (id, command)
}
#[gpui_kit::test]
fn ref18_actual_source_input_and_discover_admit_original_command(cx: &mut TestAppContext) {
    let (view, mut surface, server) = mount(cx);
    click(&mut surface, "raw-join-source");
    surface.simulate_input(r"C:\synthetic\parts.bin.003");
    draw(&mut surface);
    click(&mut surface, "raw-join-inspect");
    let (id, command) = next(&server);
    assert_eq!(command.action, RawJoinAction::Inspect);
    assert_eq!(
        command.selected_path.as_deref(),
        Some(r"C:\synthetic\parts.bin.003")
    );
    surface.update(|_, cx| {
        view.read_with(cx, |view, _| {
            assert_eq!(view.phase, Phase::Inspecting);
            assert_eq!(view.pending.as_ref().map(|p| p.0), Some(id));
        })
    });
    assert!(server.try_receive_command_for_contract_test().is_err());
}

use nxtools_protocol::{PlatformCommandResult, raw_numbered_join::RawJoinPart};

fn discover(
    view: &Entity<JoinWindow>,
    surface: &mut VisualTestContext,
    server: &PlatformServerEndpoint,
) -> (Uuid, RawNumberedJoinCommand) {
    click(surface, "raw-join-source");
    surface.simulate_input(r"C:\synthetic\parts.bin.003");
    draw(surface);
    click(surface, "raw-join-inspect");
    let command = next(server);
    assert_eq!(
        view.read_with(surface, |v, _| v.pending.as_ref().map(|p| p.0)),
        Some(command.0)
    );
    command
}
fn preview_reply(command: &RawNumberedJoinCommand, count: usize) -> RawNumberedJoinReply {
    RawNumberedJoinReply {
        version: 1,
        operation_id: command.operation_id,
        session_id: command.session_id,
        generation: command.generation,
        preview: Some(RawJoinPreview {
            preview_id: Uuid::new_v4(),
            suggested_output: r"C:\synthetic\parts.bin".into(),
            parts: (1..=count)
                .map(|i| RawJoinPart {
                    name: format!("parts.bin.{i:03}"),
                    length: 1,
                })
                .collect(),
            total_bytes: count as u64,
        }),
        terminal: None,
    }
}
fn receipt(id: Uuid, reply: RawNumberedJoinReply) -> PlatformNotification {
    PlatformNotification::CommandResult {
        correlation_id: id,
        result: PlatformCommandResult {
            success: reply.preview.is_some()
                || reply
                    .terminal
                    .as_ref()
                    .is_some_and(|t| t.outcome == "succeeded"),
            error: None,
            native_error_code: None,
            data: Some(serde_json::to_string(&reply).expect("native reply shape")),
        },
    }
}
fn forward(surface: &mut VisualTestContext, notification: PlatformNotification) {
    surface.update(|_, cx| handle(notification, cx));
    draw(surface);
}
fn deliver(
    surface: &mut VisualTestContext,
    server: &PlatformServerEndpoint,
    notification: PlatformNotification,
) {
    server
        .send_notification_for_contract_test(notification.clone())
        .expect("original terminal cleanup");
    forward(surface, notification);
}
fn prepared(
    cx: &mut TestAppContext,
) -> (
    Entity<JoinWindow>,
    VisualTestContext,
    PlatformServerEndpoint,
    RawNumberedJoinCommand,
) {
    let (view, mut surface, server) = mount(cx);
    let (id, command) = discover(&view, &mut surface, &server);
    deliver(
        &mut surface,
        &server,
        receipt(id, preview_reply(&command, 3)),
    );
    assert_eq!(view.read_with(&surface, |v, _| v.phase), Phase::Preview);
    (view, surface, server, command)
}
fn confirmed(
    surface: &mut VisualTestContext,
    server: &PlatformServerEndpoint,
) -> (Uuid, RawNumberedJoinCommand) {
    click(surface, "raw-join-review");
    assert!(
        server.try_receive_command_for_contract_test().is_err(),
        "confirmation must precede all output work"
    );
    click(surface, "raw-join-confirm");
    next(server)
}
fn terminal(
    command: &RawNumberedJoinCommand,
    outcome: &str,
    cleanup: bool,
) -> RawNumberedJoinReply {
    RawNumberedJoinReply {
        version: 1,
        operation_id: command.operation_id,
        session_id: command.session_id,
        generation: command.generation,
        preview: None,
        terminal: Some(RawJoinTerminal {
            outcome: outcome.into(),
            code: (outcome != "succeeded").then(|| {
                if cleanup {
                    "cancelled"
                } else {
                    "cleanup_failed"
                }
                .into()
            }),
            bytes_written: if outcome == "succeeded" { 3 } else { 1 },
            cleanup_complete: cleanup,
            native_error_code: (!cleanup).then_some(5),
        }),
    }
}

#[gpui_kit::test]
fn ref18_actual_dialog_is_single_explicit_start_and_cancel_ack_is_not_terminal(
    cx: &mut TestAppContext,
) {
    let (view, mut surface, server, inspected) = prepared(cx);
    let preview_id = view.read_with(&surface, |v, _| {
        v.preview.as_ref().expect("preview").preview_id
    });
    click(&mut surface, "raw-join-review");
    assert_eq!(view.read_with(&surface, |v, _| v.phase), Phase::Confirming);
    assert!(server.try_receive_command_for_contract_test().is_err());
    surface.simulate_keystrokes("escape");
    draw(&mut surface);
    assert_eq!(view.read_with(&surface, |v, _| v.phase), Phase::Preview);
    assert!(server.try_receive_command_for_contract_test().is_err());
    let (start_id, start) = confirmed(&mut surface, &server);
    assert_eq!(start.action, RawJoinAction::Start);
    assert_eq!(start.operation_id, inspected.operation_id);
    assert_eq!(start.preview_id, Some(preview_id));
    assert_eq!(
        start.output_path.as_deref(),
        Some(r"C:\synthetic\parts.bin")
    );
    assert_eq!(
        view.read_with(&surface, |v, _| v.pending.as_ref().map(|p| p.0)),
        Some(start_id)
    );
    surface.simulate_keystrokes("enter enter");
    draw(&mut surface);
    assert!(
        server.try_receive_command_for_contract_test().is_err(),
        "no Enter repeat can create a second Start"
    );
    click(&mut surface, "raw-join-cancel");
    let (cancel_id, cancel) = next(&server);
    assert_eq!(cancel.action, RawJoinAction::Cancel);
    assert_eq!(cancel.operation_id, start.operation_id);
    deliver(
        &mut surface,
        &server,
        PlatformNotification::CommandResult {
            correlation_id: cancel_id,
            result: PlatformCommandResult {
                success: true,
                error: None,
                native_error_code: None,
                data: None,
            },
        },
    );
    assert_eq!(view.read_with(&surface, |v, _| v.phase), Phase::Cancelling);
    assert_eq!(
        view.read_with(&surface, |v, _| v.pending.as_ref().map(|p| p.0)),
        Some(start_id)
    );
    deliver(
        &mut surface,
        &server,
        receipt(start_id, terminal(&start, "succeeded", true)),
    );
    assert_eq!(view.read_with(&surface, |v, _| v.phase), Phase::Terminal);
    assert!(
        view.read_with(&surface, |v, _| v
            .message
            .as_deref()
            .is_some_and(|m| m.contains("合并完成"))),
        "late success remains visible"
    );
}

#[gpui_kit::test]
fn ref18_caption_close_waits_for_actual_cancelled_terminal(cx: &mut TestAppContext) {
    let (view, mut surface, server, _) = prepared(cx);
    let (start_id, start) = confirmed(&mut surface, &server);
    let window = surface.update(|window, _| window.window_handle());
    click(&mut surface, "raw-join-caption-close");
    let (cancel_id, _) = next(&server);
    assert!(view.read_with(&surface, |v, _| v.closing && v.pending.is_some()));
    deliver(
        &mut surface,
        &server,
        PlatformNotification::CommandResult {
            correlation_id: cancel_id,
            result: PlatformCommandResult {
                success: true,
                error: None,
                native_error_code: None,
                data: None,
            },
        },
    );
    assert!(
        surface.debug_bounds("raw-join-footer").is_some(),
        "ACK does not close the window"
    );
    let actual = receipt(start_id, terminal(&start, "cancelled", true));
    server
        .send_notification_for_contract_test(actual.clone())
        .expect("actual terminal");
    surface.update(|_, cx| handle(actual, cx));
    surface.run_until_parked();
    cx.update(|cx| {
        assert!(
            window.update(cx, |_, _, _| ()).is_err(),
            "actual cleanup permits close"
        )
    });
}

#[gpui_kit::test]
fn ref18_cleanup_failure_remains_visible_after_close_request(cx: &mut TestAppContext) {
    let (view, mut surface, server, _) = prepared(cx);
    let (start_id, start) = confirmed(&mut surface, &server);
    click(&mut surface, "raw-join-close");
    let _ = next(&server);
    deliver(
        &mut surface,
        &server,
        receipt(start_id, terminal(&start, "failed", false)),
    );
    assert_eq!(view.read_with(&surface, |v, _| v.phase), Phase::Terminal);
    assert!(view.read_with(&surface, |v, _| {
        v.message.as_deref().is_some_and(|m| m.contains("清理失败"))
    }));
    assert!(!view.read_with(&surface, |v, _| v.closing));
    assert!(surface.debug_bounds("raw-join-close").is_some());
}

#[gpui_kit::test]
fn ref18_stale_generation_connection_and_deferred_callback_cannot_replace_current_preview(
    cx: &mut TestAppContext,
) {
    let (view, mut surface, server) = mount(cx);
    let (id, command) = discover(&view, &mut surface, &server);
    let mut stale = preview_reply(&command, 3);
    stale.generation += 1;
    forward(&mut surface, receipt(id, stale));
    assert_eq!(view.read_with(&surface, |v, _| v.phase), Phase::Inspecting);
    let current = receipt(id, preview_reply(&command, 3));
    server
        .send_notification_for_contract_test(current.clone())
        .expect("actual Inspect completion");
    view.update_in(&mut surface, |_, _, cx| handle(current, cx));
    draw(&mut surface);
    assert_eq!(
        view.read_with(&surface, |v, _| v.phase),
        Phase::Preview,
        "forwarding during entity lease is deferred"
    );
    let old_connection = view.read_with(&surface, |v, _| v.origin.expect("origin"));
    server.set_connected_for_contract_test(false);
    forward(
        &mut surface,
        PlatformNotification::Disconnected {
            generation: old_connection,
            pending_command_ids: vec![],
        },
    );
    assert_eq!(
        view.read_with(&surface, |v, _| v.phase),
        Phase::Disconnected
    );
    assert!(view.read_with(&surface, |v, _| v.preview.is_none()));
    server.set_connected_for_contract_test(true);
    click(&mut surface, "raw-join-inspect");
    let (new_id, new_command) = next(&server);
    forward(&mut surface, receipt(id, preview_reply(&command, 2)));
    assert_eq!(
        view.read_with(&surface, |v, _| v.pending.as_ref().map(|p| p.0)),
        Some(new_id)
    );
    deliver(
        &mut surface,
        &server,
        receipt(new_id, preview_reply(&new_command, 4)),
    );
    assert_eq!(
        view.read_with(&surface, |v, _| v
            .preview
            .as_ref()
            .expect("new preview")
            .parts
            .len()),
        4
    );
}

#[gpui_kit::test]
fn ref18_official_input_ime_enter_and_source_edit_do_not_start_work(cx: &mut TestAppContext) {
    use gpui_kit::EntityInputHandler;
    let (view, mut surface, server, _) = prepared(cx);
    view.update_in(&mut surface, |v, window, cx| {
        v.source.update(cx, |input, cx| {
            input.focus_handle(cx).focus(window, cx);
            input.replace_and_mark_text_in_range(None, "中", Some(1..1), window, cx);
        });
    });
    surface.simulate_event(gpui_kit::KeyDownEvent {
        keystroke: gpui_kit::Keystroke::parse("enter").expect("IME Enter"),
        is_held: false,
        prefer_character_input: true,
    });
    draw(&mut surface);
    assert!(server.try_receive_command_for_contract_test().is_err());
    view.update_in(&mut surface, |v, window, cx| {
        v.source
            .update(cx, |input, cx| input.unmark_text(window, cx))
    });
    surface.simulate_keystrokes("ctrl-a");
    surface.simulate_input(r"C:\synthetic\another.002");
    draw(&mut surface);
    assert!(view.read_with(&surface, |v, _| v.preview.is_none()));
    click(&mut surface, "raw-join-review");
    assert!(
        server.try_receive_command_for_contract_test().is_err(),
        "edited source invalidates confirmation"
    );
}

#[gpui_kit::test]
fn ref18_minimum_scroll_keeps_medium_footer_and_output_input_reachable(cx: &mut TestAppContext) {
    let (view, mut surface, server, _) = prepared(cx);
    for (width, height) in [(680., 600.), (480., 360.)] {
        surface.simulate_resize(size(px(width), px(height)));
        draw(&mut surface);
        let footer = surface
            .debug_bounds("raw-join-footer")
            .expect("fixed footer");
        let primary = surface
            .debug_bounds("raw-join-review")
            .expect("official primary");
        assert_eq!(primary.size.height, px(32.));
        assert!(primary.bottom() <= footer.bottom() && footer.bottom() <= px(height));
        let viewport = surface
            .debug_bounds("raw-join-viewport")
            .expect("bounded viewport");
        assert!(viewport.bottom() <= footer.top());
        for _ in 0..24 {
            if surface
                .debug_bounds("raw-join-output")
                .is_some_and(|b| b.top() >= viewport.top() && b.bottom() <= viewport.bottom())
            {
                break;
            }
            surface.simulate_event(gpui_kit::ScrollEvent {
                position: viewport.center(),
                delta: gpui_kit::ScrollDelta::Lines(gpui_kit::point(0., -2.)),
                ..Default::default()
            });
            draw(&mut surface);
        }
        let output = surface
            .debug_bounds("raw-join-output")
            .expect("actual output input");
        assert!(
            output.top() >= viewport.top() && output.bottom() <= viewport.bottom(),
            "scroll reveals real input"
        );
        click(&mut surface, "raw-join-output");
        surface.simulate_keystrokes("ctrl-a");
        surface.simulate_input(r"C:\synthetic\fresh.bin");
        draw(&mut surface);
        assert_eq!(
            view.read_with(&surface, |v, cx| v.output.read(cx).value().to_string()),
            r"C:\synthetic\fresh.bin"
        );
        assert!(server.try_receive_command_for_contract_test().is_err());
        click(&mut surface, "raw-join-review");
        let confirm = surface
            .debug_bounds("raw-join-confirm")
            .expect("actual confirmation action");
        assert_eq!(confirm.size.height, px(32.));
        assert!(confirm.top() >= px(0.) && confirm.bottom() <= px(height));
        assert!(confirm.left() >= px(0.) && confirm.right() <= px(width));
        assert!(server.try_receive_command_for_contract_test().is_err());
        surface.simulate_keystrokes("escape");
        draw(&mut surface);
        assert_eq!(view.read_with(&surface, |v, _| v.phase), Phase::Preview);
        assert!(server.try_receive_command_for_contract_test().is_err());
    }
}

#[gpui_kit::test]
fn ref18_picker_single_selection_identity_and_focus_use_original_forwarder(
    cx: &mut TestAppContext,
) {
    let (view, mut surface, _server) = mount(cx);
    let operation = Uuid::new_v4();
    let (session, generation) = view.read_with(&surface, |v, _| (v.session, v.generation));
    view.update_in(&mut surface, |v, _, _| {
        v.origin = v.platform.as_ref().and_then(|b| b.connection_generation());
        v.picker = Some((operation, ArchivePickerPurpose::Files));
    });
    let mut result = ArchivePickerResult {
        version: 1,
        operation_id: operation,
        session_id: Uuid::new_v4(),
        generation,
        paths: vec![r"C:\synthetic\parts.bin.003".into()],
        cancelled: false,
        code: None,
        native_error_code: None,
    };
    forward(
        &mut surface,
        PlatformNotification::Event(PlatformEvent::ArchivePicker(result.clone())),
    );
    assert!(view.read_with(&surface, |v, _| v.picker.is_some()));
    result.session_id = session;
    result.paths.push(r"C:\synthetic\other.001".into());
    forward(
        &mut surface,
        PlatformNotification::Event(PlatformEvent::ArchivePicker(result.clone())),
    );
    assert!(view.read_with(&surface, |v, _| v.message.is_some()));
    view.update_in(&mut surface, |v, _, _| {
        v.picker = Some((operation, ArchivePickerPurpose::Files))
    });
    result.paths.pop();
    forward(
        &mut surface,
        PlatformNotification::Event(PlatformEvent::ArchivePicker(result)),
    );
    assert_eq!(
        view.read_with(&surface, |v, cx| v.source.read(cx).value().to_string()),
        r"C:\synthetic\parts.bin.003"
    );
    assert!(surface.update(|window, cx| {
        view.read(cx)
            .source
            .read(cx)
            .focus_handle(cx)
            .is_focused(window)
    }));
}

#[gpui_kit::test]
fn ref18_archive_menu_opens_reuses_and_retires_real_window(cx: &mut TestAppContext) {
    use crate::archive::ArchiveBrowser;
    use crate::main_window::{MainWindow, MainWindowShellContext};
    use crate::notifications::NotificationCenter;
    use nxtools_core::SystemSettings;
    use nxtools_protocol::archive_browser::{ArchiveBrowserContext, ArchiveSourceBinding};

    cx.update(gpui_kit::init);
    cx.executor().allow_parking();

    let notifications = cx.update(|cx| cx.new(|_| NotificationCenter::default()));
    let (_, tools_surface) = cx.add_window_view(move |window, cx| {
        window.resize(size(px(920.), px(480.)));
        let main = cx.new(|cx| {
            MainWindow::new(
                WeakEntity::new_invalid(),
                WeakEntity::new_invalid(),
                WeakEntity::new_invalid(),
                MainWindowShellContext {
                    settings: None,
                    platform: None,
                    system_settings: Rc::new(RefCell::new(SystemSettings::default())),
                },
                notifications,
                window,
                cx,
            )
        });
        gpui_kit::base::Root::new(main, window, cx)
    });
    let tools_handle = tools_surface.update(|window, _| window.window_handle());
    let mut tools_surface = VisualTestContext::from_window(tools_handle, cx);
    draw(&mut tools_surface);
    click(&mut tools_surface, "main-module-其他工具");
    assert!(
        tools_surface.debug_bounds("tool-raw-join-tile").is_none(),
        "the ordinary numbered join is no longer an Other Tools tile"
    );

    let (bridge, _server) = platform_bridge(32);
    let bridge = Rc::new(bridge);
    cx.update(|cx| super::material_tests::prepare_presentation_fixture(bridge.clone(), cx));
    let (_, browser_surface) = cx.add_window_view(move |window, cx| {
        window.resize(size(px(920.), px(480.)));
        let view = cx.new(|cx| {
            let mut view = ArchiveBrowser::new(
                std::path::PathBuf::from(r"C:\synthetic\sample.zip"),
                Uuid::new_v4(),
                WeakEntity::new_invalid(),
                window,
                cx,
            );
            view.configure_drag(
                bridge,
                ArchiveBrowserContext {
                    snapshot_id: Uuid::new_v4(),
                    format: "Zip".into(),
                    comment: String::new(),
                    source: ArchiveSourceBinding {
                        native: "v1:synthetic".into(),
                        digest: [0; 32],
                        size: 1,
                        modified_unix_ns: 1,
                    },
                    sevenz_edit: None,
                    tar_edit: None,
                },
                1,
                0,
                false,
            );
            view
        });
        gpui_kit::base::Root::new(view, window, cx)
    });
    let browser_handle = browser_surface.update(|window, _| window.window_handle());
    let mut browser_surface = VisualTestContext::from_window(browser_handle, cx);
    draw(&mut browser_surface);

    let click_join_menu = |surface: &mut VisualTestContext| {
        click(surface, "archive-file-menu");
        let item = surface.update(|window, _| {
            gpui_kit::base::test_support::snapshots(window)
                .iter()
                .find(|node| {
                    node.role() == Some(gpui_kit::Role::MenuItem)
                        && node.label() == Some("合并分卷文件（.001）…")
                })
                .map(|node| node.bounds())
                .expect("official PopupMenu entry beside New Archive")
        });
        assert!(item.size.height >= px(26.), "default Medium menu row");
        surface.simulate_click(item.center(), gpui_kit::Modifiers::default());
        draw(surface);
    };

    click_join_menu(&mut browser_surface);
    let Target(first_view, first) = browser_surface.update(|_, cx| cx.global::<Target>().clone());
    {
        let first_entity = first_view.upgrade().expect("opened raw join view");
        assert!(
            first_entity.read_with(&browser_surface, |view, _| view.platform.is_some()),
            "the Archive Browser entrance keeps the original PlatformBridge backend"
        );
    }
    click_join_menu(&mut browser_surface);
    let Target(_, reused) = browser_surface.update(|_, cx| cx.global::<Target>().clone());
    assert_eq!(
        first, reused,
        "the deferred menu entrance reuses the live window"
    );

    let mut child = VisualTestContext::from_window(first, cx);
    draw(&mut child);
    let close = child
        .debug_bounds("raw-join-close")
        .expect("real Close hitbox")
        .center();
    child.simulate_click(close, gpui_kit::Modifiers::default());
    browser_surface.run_until_parked();
    assert!(
        browser_surface.update(
            |_, cx| first_view.upgrade().is_none() && first.update(cx, |_, _, _| ()).is_err()
        )
    );

    click_join_menu(&mut browser_surface);
    let Target(_, reopened) = browser_surface.update(|_, cx| cx.global::<Target>().clone());
    assert_ne!(first, reopened, "a closed menu-owned window is replaced");
}

fn focus_dialog_button(surface: &mut VisualTestContext, label: &'static str) {
    for _ in 0..48 {
        draw(surface);
        if surface.update(|window, _| {
            gpui_kit::base::test_support::snapshots(window)
                .iter()
                .any(|node| {
                    node.role() == Some(gpui_kit::Role::Button)
                        && node.label() == Some(label)
                        && node.focused() == Some(true)
                })
        }) {
            return;
        }
        surface.update(|window, cx| window.focus_next(cx));
    }
    panic!("actual dialog button {label} must be keyboard reachable");
}

fn release_dialog_key(surface: &mut VisualTestContext, key: &'static str) {
    let keystroke = gpui_kit::Keystroke::parse(key).expect("dialog activation key");
    surface.simulate_event(gpui_kit::KeyDownEvent {
        keystroke: keystroke.clone(),
        is_held: false,
        prefer_character_input: false,
    });
    surface.simulate_event(gpui_kit::KeyUpEvent { keystroke });
    draw(surface);
}

fn first_keyboard_confirmation(cx: &mut TestAppContext, key: &'static str) {
    let (view, mut surface, server, inspected) = prepared(cx);
    let preview_id = view.read_with(&surface, |view, _| {
        view.preview.as_ref().expect("frozen preview").preview_id
    });
    click(&mut surface, "raw-join-review");
    focus_dialog_button(&mut surface, "确认合并");
    surface.simulate_keystrokes("escape");
    draw(&mut surface);
    assert_eq!(
        view.read_with(&surface, |view, _| view.phase),
        Phase::Preview
    );
    assert!(
        server.try_receive_command_for_contract_test().is_err(),
        "Escape never starts output"
    );
    click(&mut surface, "raw-join-review");
    focus_dialog_button(&mut surface, "返回核对");
    release_dialog_key(&mut surface, "space");
    assert_eq!(
        view.read_with(&surface, |view, _| view.phase),
        Phase::Preview
    );
    assert!(
        server.try_receive_command_for_contract_test().is_err(),
        "Back never starts output"
    );
    click(&mut surface, "raw-join-review");
    focus_dialog_button(&mut surface, "确认合并");
    assert!(
        server.try_receive_command_for_contract_test().is_err(),
        "no mouse Start before first keyboard activation"
    );
    release_dialog_key(&mut surface, key);
    let (start_id, start) = next(&server);
    assert_eq!(start.action, RawJoinAction::Start);
    assert_eq!(start.operation_id, inspected.operation_id);
    assert_eq!(start.session_id, inspected.session_id);
    assert_eq!(start.generation, inspected.generation);
    assert_eq!(start.preview_id, Some(preview_id));
    assert_eq!(
        start.output_path.as_deref(),
        Some(r"C:\synthetic\parts.bin")
    );
    assert_eq!(
        view.read_with(&surface, |view, _| view
            .pending
            .as_ref()
            .map(|pending| pending.0)),
        Some(start_id)
    );
    assert_eq!(
        view.read_with(&surface, |view, _| view.phase),
        Phase::Running
    );
    release_dialog_key(&mut surface, key);
    release_dialog_key(&mut surface, key);
    assert!(
        server.try_receive_command_for_contract_test().is_err(),
        "keyboard repeat must not submit a second Start"
    );
    deliver(
        &mut surface,
        &server,
        receipt(start_id, terminal(&start, "succeeded", true)),
    );
    assert_eq!(
        view.read_with(&surface, |view, _| view.phase),
        Phase::Terminal
    );
}

#[gpui_kit::test]
fn ref18_b3_02_first_focused_enter_submits_exactly_one_start(cx: &mut TestAppContext) {
    first_keyboard_confirmation(cx, "enter");
}

#[gpui_kit::test]
fn ref18_b3_02_first_focused_space_submits_exactly_one_start(cx: &mut TestAppContext) {
    first_keyboard_confirmation(cx, "space");
}
