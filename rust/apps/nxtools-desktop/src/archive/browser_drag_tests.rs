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
use std::{cell::RefCell, collections::BTreeSet, rc::Rc};

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
            view.configure_drag(platform, ArchiveBrowserContext { snapshot_id:snapshot, format:"Zip".into(),comment:String::new(),sevenz_edit:None,tar_edit:None,source:ArchiveSourceBinding {native:"v1:owned".into(),digest:[1;32],size:99,modified_unix_ns:1}}, connection, 0, true);
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
        let center = cx.new(|_| crate::notifications::NotificationCenter::default());
        cx.update(|cx| crate::notifications::register(&center, cx));
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
        draw(&mut surface);
        center.read_with(&surface, |center, _| {
            assert!(
                center.items().is_empty(),
                "retired late reply cannot publish a notification"
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
    for mode in ["link", "encrypted", "too_large", "unsafe"] {
        let (view, mut surface, endpoint) = mount(cx);
        let center = cx.new(|_| crate::notifications::NotificationCenter::default());
        cx.update(|cx| crate::notifications::register(&center, cx));
        surface.update(|window, cx| {
            view.update(cx, |view, cx| {
                let model = view.model.as_mut().expect("model");
                match mode {
                    "link" => model.entries[1].kind = "SymbolicLink".into(),
                    "encrypted" => model.entries[1].encrypted = Some(true),
                    "too_large" => model.entries[1].size = Some(i64::MAX as u64 + 1),
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
        draw(&mut surface);
        center.read_with(&surface, |center, _| {
            assert!(
                center
                    .items()
                    .iter()
                    .any(|notice| notice.message.contains("未拖出任何项目"))
            )
        });
        assert!(
            surface.debug_bounds("archive-drag-feedback").is_none(),
            "notification must not resize the list"
        );
    }
}
#[gpui_kit::test]
fn production_row_drag_terminal_failure_retries_but_incomplete_cleanup_stays_busy(
    cx: &mut TestAppContext,
) {
    let (view, mut surface, endpoint) = mount(cx);
    let center = cx.new(|_| crate::notifications::NotificationCenter::default());
    cx.update(|cx| crate::notifications::register(&center, cx));
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
    draw(&mut surface);
    center.read_with(&surface, |center, _| {
        assert!(
            center
                .items()
                .iter()
                .any(|notice| notice.message.contains("目标"))
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
        let center = cx.new(|_| crate::notifications::NotificationCenter::default());
        cx.update(|cx| crate::notifications::register(&center, cx));
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
            draw(&mut surface);
            center.read_with(&surface, |center, _| {
                assert!(
                    center
                        .items()
                        .iter()
                        .any(|notice| notice.message.contains("未拖出任何项目"))
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
    let center = cx.new(|_| crate::notifications::NotificationCenter::default());
    cx.update(|cx| crate::notifications::register(&center, cx));
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
    draw(&mut surface);
    center.read_with(&surface, |center, _| {
        assert!(
            center
                .items()
                .iter()
                .any(|notice| notice.message.contains("解锁")),
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

// Tree regressions use actual released Tree icon/label children and the production root move.
fn install_tree(view: &Entity<ArchiveBrowser>, surface: &mut VisualTestContext) {
    surface.update(|window, cx| {
        view.update(cx, |view, cx| {
            let snapshot = view.model.as_ref().expect("model").snapshot;
            let entries = [
                (10, "tree/", "Directory", Some(0)),
                (11, "tree/empty/", "Directory", Some(0)),
                (12, "tree/sub/deep.txt", "File", None),
                (13, "other.txt", "File", Some(3)),
            ]
            .into_iter()
            .map(|(ordinal, name, kind, size)| {
                serde_json::from_value(serde_json::json!({
                    "ordinal":ordinal,"name":name,"kind":kind,"size":size,"packed_size":null,
                    "encrypted":false,"warning_codes":[],"provenance":"Utf8Flag","codepage":65001
                }))
                .expect("tree metadata")
            })
            .collect();
            view.install(snapshot, entries, "Zip".into(), window, cx);
        })
    });
    draw(surface);
}
fn tree_down(surface: &mut VisualTestContext, selector: &'static str) -> gpui_kit::Point<Pixels> {
    let origin = surface
        .debug_bounds(selector)
        .expect("real Tree child bounds")
        .center();
    surface.simulate_event(MouseDownEvent {
        position: origin,
        button: MouseButton::Left,
        click_count: 1,
        ..Default::default()
    });
    origin
}
fn tree_move(
    surface: &mut VisualTestContext,
    origin: gpui_kit::Point<Pixels>,
    offset: f32,
    held: bool,
) {
    surface.simulate_event(MouseMoveEvent {
        position: point(origin.x + px(offset), origin.y),
        pressed_button: held.then_some(MouseButton::Left),
        ..Default::default()
    });
}
#[gpui_kit::test]
fn production_tree_drag_icon_and_label_preserve_directory_without_list_navigation(
    cx: &mut TestAppContext,
) {
    for child in ["icon", "label"] {
        let (view, mut surface, endpoint) = mount(cx);
        install_tree(&view, &mut surface);
        // Tree source may be outside the current filtered list. Right selection must not change.
        surface.update(|window, cx| {
            view.update(cx, |view, cx| {
                view.navigate("tree/sub".into(), window, cx);
                let model = view.model.as_mut().expect("model");
                model.select_all();
                model.filter = "deep".into();
                model.reproject();
            })
        });
        draw(&mut surface);
        let before = view.read_with(&surface, |view, _| {
            view.model.as_ref().expect("model").selected.clone()
        });
        let selector = if child == "icon" {
            "archive-tree-icon-tree"
        } else {
            "archive-tree-label-tree"
        };
        let origin = tree_down(&mut surface, selector);
        assert!(
            endpoint.try_receive_command_for_contract_test().is_err(),
            "down is not OLE"
        );
        draw(&mut surface); // Real intervening frame / observer turn must not retire the candidate.
        tree_move(&mut surface, origin, 3., true);
        assert!(
            endpoint.try_receive_command_for_contract_test().is_err(),
            "still below threshold"
        );
        tree_move(&mut surface, origin, 12., true);
        let (_, command) = take_drag(&endpoint); // No mouse-up or deferred turn before enqueue.
        assert_eq!(command.source_window_handle, 917);
        assert_eq!(command.generation, 7);
        let items = command.items.expect("whole subtree");
        assert_eq!(
            items
                .iter()
                .map(|item| item.name.as_str())
                .collect::<Vec<_>>(),
            vec!["tree", "tree/empty", "tree/sub", "tree/sub/deep.txt"]
        );
        assert_eq!(items[0].entry.ordinal, Some(10));
        assert_eq!(items[1].entry.ordinal, Some(11));
        assert_eq!(items[2].entry.ordinal, None);
        assert_eq!(items[3].entry.ordinal, Some(12));
        assert_eq!(items[3].size, None);
        assert!(
            items.iter().all(
                |item| item.entry.browser.as_ref().expect("source").source.native == "v1:owned"
            )
        );
        view.read_with(&surface, |view, _| {
            let model = view.model.as_ref().expect("model");
            assert_eq!(model.folder, "tree/sub");
            assert_eq!(
                model.selected, before,
                "Tree drag does not replace list selection"
            );
        });
        tree_move(&mut surface, origin, 18., true);
        assert!(
            endpoint.try_receive_command_for_contract_test().is_err(),
            "one start/no preextract"
        );
    }
}
#[gpui_kit::test]
fn production_tree_root_drag_adds_safe_archive_stem_and_preserves_original_ordinals(
    cx: &mut TestAppContext,
) {
    let (view, mut surface, endpoint) = mount(cx);
    install_tree(&view, &mut surface);
    view.update(&mut surface, |view, _| {
        view.path = PathBuf::from(r"C:\owned\Project.TAR.GZ")
    });
    draw(&mut surface);
    let origin = tree_down(&mut surface, "archive-tree-icon-");
    tree_move(&mut surface, origin, 12., true);
    let (_, command) = take_drag(&endpoint);
    command.validate().expect("same v1 wire");
    let items = command.items.expect("root wrapper");
    assert_eq!(
        items
            .iter()
            .map(|item| item.name.as_str())
            .collect::<Vec<_>>(),
        vec![
            "Project",
            "Project/other.txt",
            "Project/tree",
            "Project/tree/empty",
            "Project/tree/sub",
            "Project/tree/sub/deep.txt"
        ]
    );
    assert!(items[0].directory && items[0].entry.ordinal.is_none());
    assert_eq!(items[1].entry.ordinal, Some(13));
    assert_eq!(items[5].entry.ordinal, Some(12));
    assert!(
        endpoint.try_receive_command_for_contract_test().is_err(),
        "descriptor names only, no materialization"
    );
}
#[gpui_kit::test]
fn production_tree_drag_disclosure_click_release_and_unheld_move_never_start(
    cx: &mut TestAppContext,
) {
    let (view, mut surface, endpoint) = mount(cx);
    install_tree(&view, &mut surface);
    let origin = tree_down(&mut surface, "archive-tree-disclosure-tree");
    tree_move(&mut surface, origin, 12., true);
    assert!(
        endpoint.try_receive_command_for_contract_test().is_err(),
        "arrow is not a source"
    );
    surface.simulate_event(MouseUpEvent {
        position: origin,
        button: MouseButton::Left,
        click_count: 1,
        modifiers: Modifiers::default(),
    });
    draw(&mut surface);
    assert!(
        surface
            .debug_bounds("archive-tree-label-tree/empty")
            .is_some(),
        "official expansion retained"
    );
    let origin = tree_down(&mut surface, "archive-tree-label-tree/empty");
    surface.simulate_event(MouseUpEvent {
        position: origin,
        button: MouseButton::Left,
        click_count: 1,
        modifiers: Modifiers::default(),
    });
    draw(&mut surface);
    tree_move(&mut surface, origin, 12., false);
    assert!(endpoint.try_receive_command_for_contract_test().is_err());
    view.read_with(&surface, |view, _| {
        assert_eq!(
            view.model.as_ref().expect("model").folder,
            "tree/empty",
            "ordinary click keeps navigation"
        )
    });
}
#[gpui_kit::test]
fn production_tree_root_drag_empty_unsafe_and_over_budget_reject_whole(cx: &mut TestAppContext) {
    for mode in [
        "empty",
        "unsafe",
        "files",
        "roots",
        "descriptors",
        "bytes",
        "encrypted",
    ] {
        let (view, mut surface, endpoint) = mount(cx);
        let center = cx.new(|_| crate::notifications::NotificationCenter::default());
        cx.update(|cx| crate::notifications::register(&center, cx));
        surface.update(|window,cx|view.update(cx,|view,cx| {
            let snapshot=view.model.as_ref().expect("model").snapshot;
            let count=match mode {"empty"=>0,"files"|"roots"=>33,"descriptors"=>128,_=>1};
            let entries=(0..count).map(|ordinal|serde_json::from_value(serde_json::json!({
                "ordinal":ordinal,"name":if mode=="roots" {format!("dir{ordinal}/")} else if mode=="descriptors" {format!("tree/d{ordinal}/")} else {format!("file{ordinal}.txt")},
                "kind":if matches!(mode,"roots"|"descriptors") {"Directory"} else {"File"},
                "size":if mode=="bytes" {(512u64<<20)+1} else {3},"packed_size":null,
                "encrypted":mode=="encrypted",
                "warning_codes":[],"provenance":"Utf8Flag","codepage":65001
            })).expect("bounded metadata")).collect();
            view.install(snapshot,entries,"Zip".into(),window,cx);
            if mode=="unsafe" {view.path=PathBuf::from(r"C:\owned\CON.zip");}
        }));
        draw(&mut surface);
        let origin = tree_down(&mut surface, "archive-tree-label-");
        tree_move(&mut surface, origin, 12., true);
        if matches!(mode, "files" | "roots" | "descriptors" | "bytes") {
            let (_, command) = take_drag(&endpoint);
            command.validate().expect("complete representable tree");
            let items = command.items.expect("whole tree");
            assert_eq!(
                items
                    .iter()
                    .filter(|item| item.entry.ordinal.is_some())
                    .count(),
                if mode == "descriptors" {
                    128
                } else if mode == "bytes" {
                    1
                } else {
                    33
                }
            );
        } else {
            assert!(
                endpoint.try_receive_command_for_contract_test().is_err(),
                "{mode}: no truncated start"
            );
            draw(&mut surface);
            center.read_with(&surface, |center, _| {
                assert_eq!(center.items().len(), 1, "{mode}: truthful notification")
            });
        }
    }
}
#[gpui_kit::test]
fn production_tree_drag_reconnect_and_projection_retire_original_candidate(
    cx: &mut TestAppContext,
) {
    for mode in ["reconnect", "sort", "refresh", "source", "close"] {
        let (view, mut surface, endpoint) = mount(cx);
        install_tree(&view, &mut surface);
        let origin = tree_down(&mut surface, "archive-tree-label-tree");
        let fresh = if mode == "reconnect" {
            Some(endpoint.open_connection_for_contract_test())
        } else {
            None
        };
        surface.update(|window, cx| {
            view.update(cx, |view, cx| match mode {
                "sort" => view.sort_column(ColumnKind::Size, window, cx),
                "refresh" => view.command(Command::Refresh, window, cx),
                "source" => view.set_failure("源改变".into(), window, cx),
                "close" => view.retire(cx),
                _ => {}
            })
        });
        tree_move(&mut surface, origin, 12., true);
        if let Some(receiver) = fresh {
            assert!(
                receiver.try_recv().is_err(),
                "old read cannot enter replacement pipe"
            );
        }
        assert!(endpoint.try_receive_command_for_contract_test().is_err());
    }
}

#[gpui_kit::test]
fn production_tree_drag_empty_directory_and_authorized_root_use_original_session(
    cx: &mut TestAppContext,
) {
    let (view, mut surface, endpoint) = mount(cx);
    install_tree(&view, &mut surface);
    surface.update(|window, cx| {
        view.update(cx, |view, cx| {
            view.navigate("tree/empty".into(), window, cx)
        })
    });
    draw(&mut surface);
    let origin = tree_down(&mut surface, "archive-tree-icon-tree/empty");
    tree_move(&mut surface, origin, 12., true);
    let (receipt, command) = take_drag(&endpoint);
    let items = command.items.expect("explicit empty directory");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].name, "empty");
    assert!(items[0].directory && items[0].size.is_none());
    assert_eq!(items[0].entry.ordinal, Some(11));
    surface.update(|window, cx| {
        view.update(cx, |view, cx| view.command(Command::Refresh, window, cx))
    });
    let (cancel_receipt, cancel) = take_drag(&endpoint);
    assert_eq!(cancel.operation_id, command.operation_id);
    assert_eq!(cancel.action, "cancel");
    view.update(&mut surface, |view, cx| {
        view.drag_result(cancel_receipt, &success(), cx);
        assert!(view.drag_matches(receipt), "cancel ACK not native cleanup");
    });

    let (view, mut surface, endpoint) = mount(cx);
    install_tree(&view, &mut surface);
    view.update(&mut surface, |view, _| {
        view.model.as_mut().expect("model").entries[2].encrypted = Some(true)
    });
    let origin = tree_down(&mut surface, "archive-tree-label-");
    tree_move(&mut surface, origin, 12., true);
    assert!(endpoint.try_receive_command_for_contract_test().is_err());
    view.update(&mut surface, |view, cx| view.unlock_drag(cx));
    let wire = endpoint
        .try_receive_command_for_contract_test()
        .expect("original explicit authorization");
    let Message::PlatformCommand(PlatformCommand::ArchiveEntry(authorize)) = wire.message else {
        panic!("authorize")
    };
    view.update(&mut surface, |view, cx| {
        view.authorization_result(wire.id, &authorization_success(&authorize), cx)
    });
    tree_move(&mut surface, origin, 18., true);
    assert!(
        endpoint.try_receive_command_for_contract_test().is_err(),
        "unlock never resumes prior gesture"
    );
    let fresh = tree_down(&mut surface, "archive-tree-icon-");
    tree_move(&mut surface, fresh, 12., true);
    let (_, root) = take_drag(&endpoint);
    assert_eq!(root.authorization_id, Some(authorize.operation_id));
    assert!(
        root.items
            .expect("complete root")
            .iter()
            .all(|item| !item.entry.password_interaction)
    );
}

#[gpui_kit::test]
fn production_tree_drag_split_scroll_and_empty_body_never_become_tree_sources(
    cx: &mut TestAppContext,
) {
    for area in ["split", "scroll", "marquee"] {
        let (view, mut surface, endpoint) = mount(cx);
        install_tree(&view, &mut surface);
        let pane = surface
            .debug_bounds("archive-tree-pane")
            .expect("tree pane");
        let body = surface
            .debug_bounds("archive-selection-body")
            .expect("selection body");
        let origin = match area {
            "split" => point(pane.right() + px(1.), pane.top() + px(80.)),
            "scroll" => point(pane.right() - px(2.), pane.bottom() - px(20.)),
            _ => point(body.left() + px(100.), body.bottom() - px(48.)),
        };
        surface.simulate_event(MouseDownEvent {
            position: origin,
            button: MouseButton::Left,
            click_count: 1,
            ..Default::default()
        });
        tree_move(&mut surface, origin, 12., true);
        assert!(
            endpoint.try_receive_command_for_contract_test().is_err(),
            "{area}: not a Tree/native source"
        );
        if area == "marquee" {
            view.read_with(&surface, |view, _| {
                assert!(view.marquee_click_suppressed(), "original rectangle owner")
            });
        }
    }
}

#[gpui_kit::test]
fn normal_drag_optional_metadata_and_resolvable_names_keep_every_selected_file(
    cx: &mut TestAppContext,
) {
    for mode in ["unknown_metadata", "collision_warning", "case_collision"] {
        let (view, mut surface, endpoint) = mount(cx);
        surface.update(|window, cx| {
            view.update(cx, |view, cx| {
                let model = view.model.as_mut().expect("model");
                for entry in &mut model.entries {
                    entry.encrypted = None;
                    entry.size = None;
                    if mode != "unknown_metadata" {
                        entry.warning_codes = vec!["OutputCollision".into()];
                    }
                    if mode == "case_collision" {
                        entry.name = if entry.ordinal % 2 == 0 {
                            "same.txt"
                        } else {
                            "SAME.txt"
                        }
                        .into();
                    }
                }
                model.reproject();
                model.select_all();
                view.projection_changed(window, cx);
            })
        });
        draw(&mut surface);
        down(&mut surface, 0, Modifiers::default(), 1);
        movement(&mut surface, 0, 12., true);
        let (_, command) = take_drag(&endpoint);
        command
            .validate()
            .expect("one valid original native command");
        let items = command.items.expect("all selected entries");
        assert_eq!(items.len(), 4, "{mode}: never drop a selected ordinal");
        assert_eq!(
            items
                .iter()
                .filter_map(|item| item.entry.ordinal)
                .collect::<BTreeSet<_>>(),
            BTreeSet::from([0, 1, 2, 3])
        );
        assert_eq!(
            items
                .iter()
                .map(|item| item.name.to_lowercase())
                .collect::<BTreeSet<_>>()
                .len(),
            4
        );
        assert!(items.iter().all(|item| item.size.is_none()));
        assert!(
            endpoint.try_receive_command_for_contract_test().is_err(),
            "no eager extraction"
        );
    }
}

fn install_ordinary_directory(
    view: &Entity<ArchiveBrowser>,
    surface: &mut VisualTestContext,
    comment: &str,
) {
    surface.update(|window, cx| {
        view.update(cx, |view, cx| {
            let snapshot = view.model.as_ref().expect("model").snapshot;
            let entries = (0..257)
                .map(|ordinal| {
                    let name = if ordinal == 256 {
                        "unselected.txt".to_owned()
                    } else {
                        format!("ordinary/file-{ordinal:03}.txt")
                    };
                    serde_json::from_value(serde_json::json!({
                        "ordinal": ordinal, "name": name, "kind": "File", "size": 8,
                        "packed_size": null, "encrypted": false, "warning_codes": [],
                        "provenance": "Utf8Flag", "codepage": 65001
                    }))
                    .expect("ordinary archive metadata")
                })
                .collect();
            view.install(snapshot, entries, "Zip".into(), window, cx);
            let platform = view.drag.platform.as_ref().expect("bridge").clone();
            let connection = platform
                .connection_generation()
                .expect("original connection");
            view.configure_drag(
                platform,
                ArchiveBrowserContext {
                    snapshot_id: snapshot,
                    format: "Zip".into(),
                    comment: comment.into(),
                    sevenz_edit: None,
                    tar_edit: None,
                    source: ArchiveSourceBinding {
                        native: "v1:owned".into(),
                        digest: [1; 32],
                        size: 99,
                        modified_unix_ns: 1,
                    },
                },
                connection,
                0,
                true,
            );
        });
    });
    draw(surface);
}

#[gpui_kit::test]
fn ordinary_directory_drag_held_row_enqueues_all_256_files_before_release(cx: &mut TestAppContext) {
    let (view, mut surface, endpoint) = mount(cx);
    install_ordinary_directory(&view, &mut surface, "");
    let origin = tree_down(&mut surface, "archive-folder-ordinary");
    tree_move(&mut surface, origin, 12., true);
    let (_, command) = take_drag(&endpoint);
    command
        .validate()
        .expect("complete ordinary directory command");
    assert_eq!(command.source_window_handle, 917);
    let items = command.items.expect("all frozen descriptors");
    assert_eq!(items.len(), 257, "one directory and all 256 files");
    assert_eq!(items.iter().filter(|item| !item.directory).count(), 256);
    assert_eq!(
        items
            .iter()
            .filter_map(|item| item.entry.ordinal)
            .collect::<BTreeSet<_>>(),
        (0..256).collect::<BTreeSet<_>>()
    );
    assert!(items.iter().all(|item| {
        item.entry.path.as_deref() == Some(r"C:\owned\source.zip")
            && item.entry.codepage == 936
            && item
                .entry
                .browser
                .as_ref()
                .expect("source binding")
                .source
                .native
                == "v1:owned"
    }));
    assert!(
        endpoint.try_receive_command_for_contract_test().is_err(),
        "one enqueue, no eager extraction"
    );
}

#[gpui_kit::test]
fn ordinary_directory_drag_shared_source_context_fits_existing_transport(cx: &mut TestAppContext) {
    let (view, mut surface, endpoint) = mount(cx);
    // A valid ZIP comment belongs to the source once, not to each selected member.
    install_ordinary_directory(&view, &mut surface, &"c".repeat(4096));
    let origin = tree_down(&mut surface, "archive-folder-ordinary");
    tree_move(&mut surface, origin, 12., true);
    let wire = endpoint
        .try_receive_command_for_contract_test()
        .expect("ordinary directory with a valid comment must enqueue wholly");
    let encoded = serde_json::to_vec(&wire).expect("actual transport envelope");
    assert!(
        encoded.len() <= nxtools_protocol::MAX_FRAME_BYTES,
        "one shared source context must fit the real frame bound"
    );
    let Message::PlatformCommand(PlatformCommand::ArchiveVirtualDrag(command)) = wire.message
    else {
        panic!("expected original virtual drag route")
    };
    assert_eq!(
        command.items.as_ref().expect("complete selection").len(),
        257
    );
    assert!(endpoint.try_receive_command_for_contract_test().is_err());
}

#[gpui_kit::test]
fn production_drag_rejection_uses_deferred_notification_without_resizing_browser(
    cx: &mut TestAppContext,
) {
    let (view, mut surface, endpoint) = mount(cx);
    let center = cx.new(|_| crate::notifications::NotificationCenter::default());
    cx.update(|cx| crate::notifications::register(&center, cx));
    surface.update(|_, cx| {
        view.update(cx, |view, _| {
            view.model.as_mut().expect("model").entries[0].encrypted = Some(true);
        })
    });
    let before = surface
        .debug_bounds("archive-entry-0")
        .expect("original row");
    down(&mut surface, 0, Modifiers::default(), 1);
    movement(&mut surface, 0, 12., true);
    assert!(endpoint.try_receive_command_for_contract_test().is_err());
    draw(&mut surface);
    assert!(surface.debug_bounds("archive-drag-feedback").is_none());
    assert_eq!(
        surface
            .debug_bounds("archive-entry-0")
            .expect("unchanged row"),
        before
    );
    center.read_with(&surface, |center, _| {
        let notices = center.items();
        assert_eq!(notices.len(), 1, "one gesture produces one notification");
        assert_eq!(
            notices[0].level,
            crate::notifications::NotificationLevel::Error
        );
        assert!(notices[0].message.contains("加密文件"));
    });
    movement(&mut surface, 0, 18., true);
    draw(&mut surface);
    center.read_with(&surface, |center, _| assert_eq!(center.items().len(), 1));
}

#[gpui_kit::test]
fn production_native_drag_failure_notifies_once_without_inline_feedback(cx: &mut TestAppContext) {
    let (view, mut surface, endpoint) = mount(cx);
    let center = cx.new(|_| crate::notifications::NotificationCenter::default());
    cx.update(|cx| crate::notifications::register(&center, cx));
    down(&mut surface, 0, Modifiers::default(), 1);
    movement(&mut surface, 0, 12., true);
    let (receipt, _) = take_drag(&endpoint);
    let failure = nxtools_protocol::PlatformCommandResult {
        success: false,
        error: Some("virtual_target_not_accepted".into()),
        native_error_code: None,
        data: None,
    };
    surface.update(|_, cx| {
        view.update(cx, |view, cx| {
            view.drag_result(receipt, &failure, cx);
            view.drag_result(receipt, &failure, cx);
        })
    });
    draw(&mut surface);
    center.read_with(&surface, |center, _| {
        assert_eq!(center.items().len(), 1);
        assert_eq!(center.items()[0].message, "目标未接收本次拖出。");
    });
    assert!(surface.debug_bounds("archive-drag-feedback").is_none());
    view.read_with(&surface, |view, _| assert!(!view.native_drag_active()));
}
