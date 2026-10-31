//! Actual ArchiveBrowser ListItem events and original platform queue, no spike view/native OLE.
use super::*;
use gpui_kit::{
    Modifiers, MouseDownEvent, MouseMoveEvent, MouseUpEvent, TestAppContext, VisualTestContext,
    point, size,
};
use nxtools_protocol::{
    Message, PlatformCommand,
    archive_browser::{ArchiveBrowserContext, ArchiveSourceBinding},
};
use std::{cell::RefCell, rc::Rc};

fn mount(
    cx: &mut TestAppContext,
) -> (
    Entity<ArchiveBrowser>,
    VisualTestContext,
    nxtools_ipc::PlatformServerEndpoint,
) {
    cx.update(gpui_kit::init);
    cx.executor().allow_parking();
    let (platform, endpoint) = nxtools_ipc::platform_bridge(32);
    endpoint.set_connected_for_contract_test(true);
    let platform = Rc::new(platform);
    let snapshot = Uuid::new_v4();
    let retained = Rc::new(RefCell::new(None));
    let slot = retained.clone();
    let (_, surface) = cx.add_window_view(move |window, cx| {
        let view = cx.new(|cx| {
            let mut view = ArchiveBrowser::new(PathBuf::from(r"C:\owned\source.zip"), Uuid::new_v4(), WeakEntity::new_invalid(), window, cx);
            view.generation = 7;
            view.codepage = 936;
            view.install(snapshot, (0..4).map(|ordinal| serde_json::from_value(serde_json::json!({
                "ordinal": ordinal, "name": format!("file-{ordinal}.txt"), "kind":"File", "size":3,
                "packed_size":null,"encrypted":false,"warning_codes":[],"provenance":"Utf8Flag","codepage":65001
            })).expect("metadata")).collect(), "Zip".into(), window, cx);
            let connection = platform.connection_generation().expect("fixture original read connection");
            view.configure_drag(platform, ArchiveBrowserContext { snapshot_id:snapshot, format:"Zip".into(),comment:String::new(),source:ArchiveSourceBinding {native:"v1:owned".into(),digest:[1;32],size:99,modified_unix_ns:1}}, connection, 0, true);
            // Test platform Window has no native HWND. Only this dependency is synthetic; row callbacks are production.
            view.drag.source_hwnd = Some(917);
            view
        });
        *slot.borrow_mut()=Some(view.clone());
        gpui_kit::base::Root::new(view,window,cx)
    });
    let handle = surface.update(|window, _| window.window_handle());
    let mut surface = VisualTestContext::from_window(handle, cx);
    surface.simulate_resize(size(px(960.), px(640.)));
    draw(&mut surface);
    (
        retained.borrow().clone().expect("browser"),
        surface,
        endpoint,
    )
}
fn draw(surface: &mut VisualTestContext) {
    surface.run_until_parked();
    surface.update(|window, cx| {
        window.activate_window();
        window.draw(cx).clear(cx);
    });
}
fn position(surface: &mut VisualTestContext, ordinal: u64) -> gpui_kit::Point<Pixels> {
    let selector = match ordinal {
        0 => "archive-entry-0",
        1 => "archive-entry-1",
        2 => "archive-entry-2",
        _ => "archive-entry-3",
    };
    surface
        .debug_bounds(selector)
        .expect("actual row bounds")
        .center()
}
fn down(surface: &mut VisualTestContext, ordinal: u64, modifiers: Modifiers, count: usize) {
    let position = position(surface, ordinal);
    surface.simulate_event(MouseDownEvent {
        position,
        button: MouseButton::Left,
        modifiers,
        click_count: count,
        ..Default::default()
    });
}
fn up(surface: &mut VisualTestContext, ordinal: u64, modifiers: Modifiers, count: usize) {
    let position = position(surface, ordinal);
    surface.simulate_event(MouseUpEvent {
        position,
        button: MouseButton::Left,
        modifiers,
        click_count: count,
    });
}
fn movement(surface: &mut VisualTestContext, ordinal: u64, offset: f32, pressed: bool) {
    let start = position(surface, ordinal);
    surface.simulate_event(MouseMoveEvent {
        position: point(start.x + px(offset), start.y),
        pressed_button: pressed.then_some(MouseButton::Left),
        ..Default::default()
    });
}
#[gpui_kit::test]
fn production_row_drag_threshold_freezes_selection_and_submits_before_release(
    cx: &mut TestAppContext,
) {
    let (view, mut surface, endpoint) = mount(cx);
    down(&mut surface, 0, Modifiers::default(), 1);
    assert!(
        endpoint.try_receive_command_for_contract_test().is_err(),
        "ordinary down is NOT drag"
    );
    up(&mut surface, 0, Modifiers::default(), 1);
    draw(&mut surface);
    let control = Modifiers {
        control: true,
        ..Default::default()
    };
    down(&mut surface, 2, control, 1);
    up(&mut surface, 2, control, 1);
    draw(&mut surface);
    down(&mut surface, 0, Modifiers::default(), 1);
    movement(&mut surface, 0, 2., true);
    assert!(
        endpoint.try_receive_command_for_contract_test().is_err(),
        "subthreshold movement is not drag"
    );
    movement(&mut surface, 0, 12., true);
    // No MouseUp/run_until_parked: immediate pressed callback must enqueue formal production command.
    let wire = endpoint
        .try_receive_command_for_contract_test()
        .expect("formal row must enqueue before mouse-up/deferred turn");
    let Message::PlatformCommand(PlatformCommand::ArchiveVirtualDrag(command)) = wire.message
    else {
        panic!("not formal virtual drag")
    };
    assert_eq!(command.source_window_handle, 917);
    assert_eq!(command.generation, 7);
    let items = command.items.expect("whole frozen selection");
    assert_eq!(
        items
            .iter()
            .map(|item| item.entry.ordinal.expect("original ordinal"))
            .collect::<Vec<_>>(),
        vec![0, 2]
    );
    assert!(items.iter().all(|item| {
        item.entry.codepage == 936
            && item
                .entry
                .browser
                .as_ref()
                .expect("bound source")
                .source
                .native
                == "v1:owned"
    }));
    let session = view.read_with(&surface, |view, _| view.window_id);
    assert_eq!(command.session_id, session);
    movement(&mut surface, 0, 18., true);
    assert!(
        endpoint.try_receive_command_for_contract_test().is_err(),
        "one start only; no eager materialization"
    );
}

fn take_drag(
    endpoint: &nxtools_ipc::PlatformServerEndpoint,
) -> (
    Uuid,
    nxtools_protocol::archive_virtual_drag::ArchiveVirtualDragCommand,
) {
    let envelope = endpoint
        .try_receive_command_for_contract_test()
        .expect("formal drag command");
    let Message::PlatformCommand(PlatformCommand::ArchiveVirtualDrag(command)) = envelope.message
    else {
        panic!("not formal drag")
    };
    (envelope.id, *command)
}
fn success() -> nxtools_protocol::PlatformCommandResult {
    nxtools_protocol::PlatformCommandResult {
        success: true,
        error: None,
        native_error_code: None,
        data: Some(r#"{"status":"cancelled","drop_effect":0}"#.into()),
    }
}
#[gpui_kit::test]
fn production_row_drag_click_doubleclick_and_mouseup_do_not_start_drag(cx: &mut TestAppContext) {
    let (view, mut surface, endpoint) = mount(cx);
    down(&mut surface, 0, Modifiers::default(), 1);
    up(&mut surface, 0, Modifiers::default(), 1);
    draw(&mut surface);
    movement(&mut surface, 0, 20., false);
    assert!(endpoint.try_receive_command_for_contract_test().is_err());
    down(&mut surface, 0, Modifiers::default(), 2);
    movement(&mut surface, 0, 12., true);
    up(&mut surface, 0, Modifiers::default(), 2);
    view.read_with(&surface, |view, _| {
        assert_eq!(
            view.open_dispatches.len(),
            1,
            "double click still owns open, never virtual drag"
        )
    });
    assert!(endpoint.try_receive_command_for_contract_test().is_err());
}
#[gpui_kit::test]
fn production_row_drag_retirement_cancels_exact_once_and_late_receipt_is_quiet(
    cx: &mut TestAppContext,
) {
    for mode in ["sort", "refresh", "source", "close", "disconnect"] {
        let (view, mut surface, endpoint) = mount(cx);
        down(&mut surface, 0, Modifiers::default(), 1);
        movement(&mut surface, 0, 12., true);
        let (receipt, start) = take_drag(&endpoint);
        surface.update(|window, cx| {
            view.update(cx, |view, cx| match mode {
                "sort" => view.sort_column(ColumnKind::Size, window, cx),
                "refresh" => view.command(Command::Refresh, window, cx),
                "source" => view.set_failure("源压缩包已更改".into(), window, cx),
                "close" => view.retire(cx),
                _ => view.drag_disconnected(cx),
            })
        });
        if mode != "disconnect" {
            let (cancel_receipt, cancel) = take_drag(&endpoint);
            assert_eq!(
                (cancel.operation_id, cancel.session_id, cancel.generation),
                (start.operation_id, start.session_id, start.generation)
            );
            assert_eq!(cancel.action, "cancel");
            assert!(cancel.items.is_none());
            surface.update(|_, cx| {
                view.update(cx, |view, cx| {
                    view.invalidate_drag();
                    view.drag_result(cancel_receipt, &success(), cx);
                    assert!(
                        view.drag_matches(receipt),
                        "cancel ACK is not start terminal"
                    );
                    view.drag_result(receipt, &success(), cx);
                    assert!(!view.drag_matches(receipt));
                })
            });
        }
        let failed = nxtools_protocol::PlatformCommandResult {
            success: false,
            error: Some("stale failure".into()),
            native_error_code: None,
            data: None,
        };
        surface.update(|_, cx| view.update(cx, |view, cx| view.drag_result(receipt, &failed, cx)));
        view.read_with(&surface, |view, _| {
            assert!(
                view.drag.message.is_none(),
                "retired late reply cannot change current presentation"
            )
        });
        assert!(
            endpoint.try_receive_command_for_contract_test().is_err(),
            "one cancel only, no new materialization"
        );
    }
}
#[gpui_kit::test]
fn production_row_drag_projection_before_threshold_never_delivers_old_selection(
    cx: &mut TestAppContext,
) {
    for mode in ["sort", "refresh", "source", "close", "disconnect"] {
        let (view, mut surface, endpoint) = mount(cx);
        down(&mut surface, 0, Modifiers::default(), 1);
        surface.update(|window, cx| {
            view.update(cx, |view, cx| match mode {
                "sort" => view.sort_column(ColumnKind::Size, window, cx),
                "refresh" => view.command(Command::Refresh, window, cx),
                "source" => view.set_failure("源已变化".into(), window, cx),
                "close" => view.retire(cx),
                _ => view.drag_disconnected(cx),
            })
        });
        movement(&mut surface, 0, 15., true);
        assert!(
            endpoint.try_receive_command_for_contract_test().is_err(),
            "no old candidate after {mode}"
        );
    }
}
#[gpui_kit::test]
fn production_row_drag_mixed_unsupported_selection_rejects_whole_and_keeps_reason(
    cx: &mut TestAppContext,
) {
    for mode in [
        "link",
        "unknown_encryption",
        "encrypted",
        "too_large",
        "unsafe",
    ] {
        let (view, mut surface, endpoint) = mount(cx);
        surface.update(|window, cx| {
            view.update(cx, |view, cx| {
                let model = view.model.as_mut().expect("model");
                match mode {
                    "link" => model.entries[1].kind = "SymbolicLink".into(),
                    "unknown_encryption" => model.entries[1].encrypted = None,
                    "encrypted" => model.entries[1].encrypted = Some(true),
                    "too_large" => model.entries[1].size = Some(512 << 20),
                    _ => model.entries[1].warning_codes = vec!["UnsafePath".into()],
                }
                model.reproject();
                model.select_all();
                view.projection_changed(window, cx);
            })
        });
        draw(&mut surface);
        down(&mut surface, 0, Modifiers::default(), 1);
        movement(&mut surface, 0, 12., true);
        assert!(
            endpoint.try_receive_command_for_contract_test().is_err(),
            "mixed selection must not truncate {mode}"
        );
        view.read_with(&surface, |view, _| {
            assert!(
                view.drag
                    .message
                    .as_deref()
                    .is_some_and(|message| message.contains("未拖出任何项目"))
            )
        });
        draw(&mut surface);
        let visible = surface.update(|window, _| {
            gpui_kit::base::test_support::snapshots(window)
                .iter()
                .any(|node| {
                    node.label()
                        .is_some_and(|label| label.contains("未拖出任何项目"))
                        && node.bounds().size.width > px(0.)
                        && node.bounds().size.height > px(0.)
                })
        });
        assert!(visible, "official Alert reason is accessible and visible");
    }
}
#[gpui_kit::test]
fn production_row_drag_terminal_failure_retries_but_incomplete_cleanup_stays_busy(
    cx: &mut TestAppContext,
) {
    let (view, mut surface, endpoint) = mount(cx);
    down(&mut surface, 0, Modifiers::default(), 1);
    movement(&mut surface, 0, 12., true);
    let (receipt, _) = take_drag(&endpoint);
    let failed = nxtools_protocol::PlatformCommandResult {
        success: false,
        error: Some("virtual_target_not_accepted".into()),
        native_error_code: None,
        data: None,
    };
    endpoint
        .send_notification_for_contract_test(nxtools_ipc::PlatformNotification::CommandResult {
            correlation_id: receipt,
            result: failed.clone(),
        })
        .expect("original transport observes native terminal");
    surface.update(|_, cx| view.update(cx, |view, cx| view.drag_result(receipt, &failed, cx)));
    view.read_with(&surface, |view, _| {
        assert!(
            view.drag
                .message
                .as_deref()
                .expect("failure")
                .contains("目标")
        )
    });
    down(&mut surface, 0, Modifiers::default(), 1);
    movement(&mut surface, 0, 12., true);
    let (next, _) = take_drag(&endpoint);
    assert_ne!(receipt, next);
    let incomplete = nxtools_protocol::PlatformCommandResult {
        success: false,
        error: Some("virtual_cleanup_incomplete".into()),
        native_error_code: None,
        data: Some(r#"{"cleanup_complete":false}"#.into()),
    };
    surface.update(|_, cx| view.update(cx, |view, cx| view.drag_result(next, &incomplete, cx)));
    down(&mut surface, 0, Modifiers::default(), 1);
    movement(&mut surface, 0, 20., true);
    assert!(endpoint.try_receive_command_for_contract_test().is_err());
    view.read_with(&surface, |view, _| {
        assert!(view.drag_matches(next), "actual cleanup-bound busy slot")
    });
}

#[gpui_kit::test]
fn production_row_drag_reserved_cancel_survives_full_ordinary_queue(cx: &mut TestAppContext) {
    let (view, mut surface, endpoint) = mount(cx);
    down(&mut surface, 0, Modifiers::default(), 1);
    movement(&mut surface, 0, 12., true);
    let (_, start) = take_drag(&endpoint);
    let bridge = view.read_with(&surface, |view, _| {
        view.drag.platform.as_ref().expect("bridge").clone()
    });
    for _ in 0..32 {
        bridge
            .send_command(PlatformCommand::PlaySystemNotificationSound)
            .expect("fill original ordinary queue");
    }
    assert!(
        bridge
            .send_command(PlatformCommand::PlaySystemNotificationSound)
            .is_err()
    );
    surface.update(|window, cx| {
        view.update(cx, |view, cx| {
            view.sort_column(ColumnKind::Size, window, cx)
        })
    });
    let (_, cancel) = take_drag(&endpoint);
    assert_eq!(cancel.action, "cancel");
    assert_eq!(
        (cancel.operation_id, cancel.session_id, cancel.generation),
        (start.operation_id, start.session_id, start.generation)
    );
}

#[gpui_kit::test]
fn production_row_drag_tree_freezes_explicit_implicit_empty_and_unknown_from_snapshot(
    cx: &mut TestAppContext,
) {
    let (view, mut surface, endpoint) = mount(cx);
    surface.update(|window,cx| view.update(cx,|view,cx| {
        let snapshot=view.model.as_ref().expect("model").snapshot;
        let entries=[(0,"tree/","Directory",Some(0)),(1,"tree/empty/","Directory",Some(0)),(2,"tree/sub/deep.txt","File",None),(3,"loose.txt","File",Some(3))]
            .into_iter().map(|(ordinal,name,kind,size)|serde_json::from_value(serde_json::json!({"ordinal":ordinal,"name":name,"kind":kind,"size":size,"packed_size":null,"encrypted":false,"warning_codes":[],"provenance":"Utf8Flag","codepage":65001})).expect("tree metadata")).collect();
        view.install(snapshot,entries,"Zip".into(),window,cx);
        view.model.as_mut().expect("model").select_all();
    }));
    draw(&mut surface);
    down(&mut surface, 3, Modifiers::default(), 1);
    movement(&mut surface, 3, 12., true);
    let (_, command) = take_drag(&endpoint);
    let items = serde_json::to_value(command.items.expect("whole tree")).expect("wire");
    let items = items.as_array().expect("array");
    assert_eq!(
        items
            .iter()
            .map(|item| item["name"].as_str().expect("name"))
            .collect::<Vec<_>>(),
        vec![
            "loose.txt",
            "tree",
            "tree/empty",
            "tree/sub",
            "tree/sub/deep.txt"
        ]
    );
    assert_eq!(items[1]["directory"], true);
    assert_eq!(items[2]["entry"]["ordinal"], 1);
    assert!(items[3]["entry"].get("ordinal").is_none());
    assert_eq!(items[4]["entry"]["ordinal"], 2);
    assert!(items[4]["size"].is_null(), "unknown must not promise zero");
    assert!(
        endpoint.try_receive_command_for_contract_test().is_err(),
        "no eager contents"
    );
}

#[gpui_kit::test]
fn production_row_drag_tree_directory_pointer_freezes_or_rejects_hidden_member_wholly(
    cx: &mut TestAppContext,
) {
    for encrypted in [false, true] {
        let (view, mut surface, endpoint) = mount(cx);
        surface.update(|window,cx|view.update(cx,|view,cx| {
            let snapshot=view.model.as_ref().expect("model").snapshot;
            // Keep two root rows: a lone safe directory is intentionally auto-entered by install.
            let entries=[(7,"tree/deep/child.txt",None,encrypted),(8,"unselected.txt",Some(1),false)]
                .into_iter().map(|(ordinal,name,size,encrypted)|serde_json::from_value(serde_json::json!({"ordinal":ordinal,"name":name,"kind":"File","size":size,"packed_size":null,"encrypted":encrypted,"warning_codes":[],"provenance":"Utf8Flag","codepage":65001})).expect("immutable descendant")).collect();
            view.install(snapshot,entries,"Zip".into(),window,cx);
        }));
        draw(&mut surface);
        let start = surface
            .debug_bounds("archive-folder-tree")
            .expect("actual directory row")
            .center();
        surface.simulate_event(MouseDownEvent {
            position: start,
            button: MouseButton::Left,
            click_count: 1,
            ..Default::default()
        });
        surface.simulate_event(MouseMoveEvent {
            position: point(start.x + px(12.), start.y),
            pressed_button: Some(MouseButton::Left),
            ..Default::default()
        });
        if encrypted {
            assert!(
                endpoint.try_receive_command_for_contract_test().is_err(),
                "hidden unsupported descendant cannot be excluded"
            );
            view.read_with(&surface, |view, _| {
                assert!(
                    view.drag
                        .message
                        .as_deref()
                        .is_some_and(|m| m.contains("未拖出任何项目"))
                )
            });
        } else {
            let (_, command) = take_drag(&endpoint);
            let items = command.items.expect("tree");
            assert_eq!(
                items.iter().map(|i| i.name.as_str()).collect::<Vec<_>>(),
                vec!["tree", "tree/deep", "tree/deep/child.txt"]
            );
            assert_eq!(items[2].entry.ordinal, Some(7));
            assert_eq!(items[2].size, None);
        }
    }
}

#[gpui_kit::test]
fn production_password_drag_locked_row_has_unlock_feedback_and_no_start(cx: &mut TestAppContext) {
    let (view, mut surface, endpoint) = mount(cx);
    surface.update(|window, cx| {
        view.update(cx, |view, cx| {
            let model = view.model.as_mut().expect("model");
            model.entries[0].encrypted = Some(true);
            model.reproject();
            view.projection_changed(window, cx);
        })
    });
    draw(&mut surface);
    down(&mut surface, 0, Modifiers::default(), 1);
    movement(&mut surface, 0, 12., true);
    assert!(
        endpoint.try_receive_command_for_contract_test().is_err(),
        "locked selection never enters OLE"
    );
    view.read_with(&surface, |view, _| {
        assert!(
            view.drag
                .message
                .as_deref()
                .is_some_and(|message| message.contains("解锁")),
            "locked gesture must explicitly explain unlock first"
        )
    });
}

fn authorization_success(
    command: &nxtools_protocol::archive_browser::ArchiveEntryCommand,
) -> nxtools_protocol::PlatformCommandResult {
    nxtools_protocol::PlatformCommandResult {
        success: true,
        error: None,
        native_error_code: None,
        data: Some(
            serde_json::to_string(&nxtools_protocol::archive_browser::ArchiveEntryResult {
                version: 1,
                operation_id: command.operation_id,
                session_id: command.session_id,
                generation: command.generation,
                success: true,
                lease_id: Some(command.operation_id),
                path: None,
                code: None,
                native_error_code: None,
                cleanup_complete: true,
                retained: false,
            })
            .expect("authorization result"),
        ),
    }
}
#[gpui_kit::test]
fn production_password_drag_unlock_success_requires_fresh_gesture_and_revokes_old_authority(
    cx: &mut TestAppContext,
) {
    let (view, mut surface, endpoint) = mount(cx);
    surface.update(|window, cx| {
        view.update(cx, |view, cx| {
            let model = view.model.as_mut().expect("model");
            model.entries[0].encrypted = Some(true);
            model.reproject();
            model.select_all();
            view.projection_changed(window, cx);
        })
    });
    draw(&mut surface);
    down(&mut surface, 0, Modifiers::default(), 1);
    view.update(&mut surface, |view, cx| view.unlock_drag(cx));
    let wire = endpoint
        .try_receive_command_for_contract_test()
        .expect("explicit unlock, not drag");
    let Message::PlatformCommand(PlatformCommand::ArchiveEntry(command)) = wire.message else {
        panic!("authorize");
    };
    assert_eq!(
        command.action,
        nxtools_protocol::archive_browser::ArchiveEntryAction::Authorize
    );
    assert!(command.ordinal.is_none());
    movement(&mut surface, 0, 12., true);
    assert!(
        endpoint.try_receive_command_for_contract_test().is_err(),
        "unlock retires oldheldcandidate"
    );
    view.update(&mut surface, |view, cx| {
        view.authorization_result(wire.id, &authorization_success(&command), cx)
    });
    assert!(
        endpoint.try_receive_command_for_contract_test().is_err(),
        "success NEVER starts drag"
    );
    movement(&mut surface, 0, 18., true);
    assert!(
        endpoint.try_receive_command_for_contract_test().is_err(),
        "late move still oldgesture"
    );
    up(&mut surface, 0, Modifiers::default(), 1);
    draw(&mut surface);
    view.update(&mut surface, |view, _| {
        view.model
            .as_mut()
            .expect("original selection")
            .select_all()
    });
    down(&mut surface, 0, Modifiers::default(), 1);
    movement(&mut surface, 0, 12., true);
    let (_, drag) = take_drag(&endpoint);
    assert_eq!(drag.authorization_id, Some(command.operation_id));
    assert_eq!(drag.items.as_ref().expect("wholemixedselection").len(), 4);
    assert!(
        drag.items
            .expect("items")
            .iter()
            .all(|item| !item.entry.password_interaction)
    );
    movement(&mut surface, 0, 18., true);
    assert!(endpoint.try_receive_command_for_contract_test().is_err());
    view.update(&mut surface, |view, cx| {
        view.invalidate_drag();
        cx.notify();
    });
    let commands = std::iter::from_fn(|| endpoint.try_receive_command_for_contract_test().ok())
        .collect::<Vec<_>>();
    assert!(commands.iter().any(|wire|matches!(&wire.message,Message::PlatformCommand(PlatformCommand::ArchiveEntry(value)) if value.action==nxtools_protocol::archive_browser::ArchiveEntryAction::Revoke&&value.operation_id==command.operation_id)));
}

#[gpui_kit::test]
fn production_password_drag_retirement_and_reconnect_reject_late_unlock_result(
    cx: &mut TestAppContext,
) {
    for mode in ["refresh", "source", "close", "disconnect", "reconnect"] {
        let (view, mut surface, endpoint) = mount(cx);
        view.update(&mut surface, |view, cx| view.unlock_drag(cx));
        let wire = endpoint
            .try_receive_command_for_contract_test()
            .expect("prepare");
        let Message::PlatformCommand(PlatformCommand::ArchiveEntry(command)) = wire.message else {
            panic!("authorize");
        };
        let fresh = if mode == "reconnect" {
            Some(endpoint.open_connection_for_contract_test())
        } else {
            None
        };
        surface.update(|window, cx| {
            view.update(cx, |view, cx| match mode {
                "refresh" => view.command(Command::Refresh, window, cx),
                "source" => view.set_failure("源改变".into(), window, cx),
                "close" => view.retire(cx),
                "disconnect" => view.drag_disconnected(cx),
                _ => {}
            })
        });
        view.update(&mut surface, |view, cx| {
            view.authorization_result(wire.id, &authorization_success(&command), cx)
        });
        if let Some(receiver) = fresh {
            assert!(
                receiver.try_recv().is_err(),
                "oldprepare cannot authorize newpipe"
            );
        }
        while let Ok(wire) = endpoint.try_receive_command_for_contract_test() {
            assert!(
                !matches!(
                    wire.message,
                    Message::PlatformCommand(PlatformCommand::ArchiveVirtualDrag(_))
                ),
                "retirement neverauto-start"
            );
        }
    }
}
