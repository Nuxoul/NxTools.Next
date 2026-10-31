use super::presentation_tests::{draw, mount, official_cards, tick};
use super::*;
use gpui_kit::{TestAppContext, px};

#[gpui_kit::test]
fn notification_header_copy_button_is_left_of_official_close(cx: &mut TestAppContext) {
    let (center, surface) = mount(cx);
    center.update(surface, |center, cx| {
        center.show_progress("header-copy", "通知标题", "完整正文", cx)
    });
    tick(surface, 500);
    let card = official_cards(surface)[0].bounds();
    surface.simulate_mouse_move(card.center(), None, Default::default());
    draw(surface);
    let copy = surface
        .debug_bounds("notification-copy-button-header-copy")
        .expect("actual copy icon button");
    let close = surface.update(|window, _| {
        gpui_kit::base::test_support::snapshots(window)
            .into_iter()
            .find(|node| node.path().last() == Some(&"close".into()))
            .expect("official close")
            .bounds()
    });
    assert!(
        copy.right() <= close.left(),
        "copy must be immediately left of close"
    );
    assert!((copy.center().y - close.center().y).abs() <= px(1.0));
    assert_eq!(copy.size, close.size);
    assert!(copy.left() >= card.left() && close.right() <= card.right());
}

fn bind_platform(
    center: &Entity<NotificationCenter>,
    surface: &mut gpui_kit::VisualTestContext,
) -> nxtools_ipc::PlatformServerEndpoint {
    let (bridge, endpoint) = nxtools_ipc::platform_bridge(8);
    endpoint.set_connected_for_contract_test(true);
    center.update(surface, |center, _| {
        center.bind_copy_platform(Rc::new(bridge))
    });
    endpoint
}
fn reply(
    center: &Entity<NotificationCenter>,
    surface: &mut gpui_kit::VisualTestContext,
    id: uuid::Uuid,
    success: bool,
) {
    center.update(surface, |center, cx| {
        center.handle_copy(
            nxtools_ipc::PlatformNotification::CommandResult {
                correlation_id: id,
                result: nxtools_protocol::PlatformCommandResult {
                    success,
                    error: None,
                    native_error_code: None,
                    data: None,
                },
            },
            cx,
        )
    });
    draw(surface);
}

#[gpui_kit::test]
fn notification_copy_click_sends_full_copy_only_payload_and_waits_for_result(
    cx: &mut TestAppContext,
) {
    let (center, surface) = mount(cx);
    let endpoint = bind_platform(&center, surface);
    let calls = Rc::new(std::cell::Cell::new(0));
    let counted = calls.clone();
    center.update(surface, |center, cx| {
        center.push(
            AppNotification::new(
                "payload",
                NotificationLevel::Progress,
                "完整标题",
                "第一行\n第二行",
            )
            .with_details(vec![NotificationDetail::new(
                "路径",
                "C:/synthetic/full.txt",
            )])
            .actions(vec![NotificationAction::new(
                "不应复制的按钮",
                move |_, _| counted.set(counted.get() + 1),
            )]),
            cx,
        )
    });
    tick(surface, 500);
    let generation = center.read_with(surface, |center, _| center.items()[0].generation());
    let card = official_cards(surface)[0].bounds();
    surface.simulate_mouse_move(card.center(), None, Default::default());
    draw(surface);
    let button = surface
        .debug_bounds("notification-copy-button-payload")
        .expect("copy");
    surface.simulate_click(button.center(), Default::default());
    surface.run_until_parked();
    let command = endpoint
        .try_receive_command_for_contract_test()
        .expect("copy command");
    let nxtools_protocol::Message::PlatformCommand(nxtools_protocol::PlatformCommand::PasteText(
        payload,
    )) = &command.message
    else {
        panic!("copy-only text");
    };
    assert_eq!(
        payload.text,
        "完整标题\r\n第一行\n第二行\r\n路径：C:/synthetic/full.txt"
    );
    assert!(!payload.paste_after_write);
    assert_eq!(
        (
            payload.target_window_handle,
            payload.target_focus_window_handle
        ),
        (0, 0)
    );
    assert_eq!(command.correlation_id, Some(command.id));
    assert_eq!(
        center.read_with(surface, |center, _| center.copy_status(generation)),
        copy::CopyStatus::Pending
    );
    assert!(
        center.read_with(surface, |center, _| !center.items()[0].exiting
            && !center.items()[0].action_claimed)
    );
    assert_eq!(calls.get(), 0);
    surface.simulate_click(button.center(), Default::default());
    surface.run_until_parked();
    assert!(endpoint.try_receive_command_for_contract_test().is_err());
    reply(&center, surface, uuid::Uuid::new_v4(), true);
    assert_eq!(
        center.read_with(surface, |center, _| center.copy_status(generation)),
        copy::CopyStatus::Pending
    );
    reply(&center, surface, command.id, true);
    assert_eq!(
        center.read_with(surface, |center, _| center.copy_status(generation)),
        copy::CopyStatus::Copied
    );
    assert_eq!(
        center.read_with(surface, |center, _| center.items().len()),
        1,
        "copy must not add another notification"
    );
    assert_eq!(calls.get(), 0);
}

#[gpui_kit::test]
fn notification_copy_failed_result_retries_and_old_reply_cannot_mark_replacement(
    cx: &mut TestAppContext,
) {
    let (center, surface) = mount(cx);
    let endpoint = bind_platform(&center, surface);
    center.update(surface, |center, cx| {
        center.show_progress("retry", "旧通知", "原文", cx)
    });
    let old = center.read_with(surface, |center, _| center.items()[0].generation());
    center.update(surface, |center, cx| center.request_copy("retry", old, cx));
    let command = endpoint
        .try_receive_command_for_contract_test()
        .expect("first");
    reply(&center, surface, command.id, false);
    assert_eq!(
        center.read_with(surface, |center, _| center.copy_status(old)),
        copy::CopyStatus::Failed
    );
    center.update(surface, |center, cx| center.request_copy("retry", old, cx));
    let second = endpoint
        .try_receive_command_for_contract_test()
        .expect("retry");
    center.update(surface, |center, cx| {
        center.show_progress("retry", "新通知", "新正文", cx)
    });
    let current = center.read_with(surface, |center, _| center.items()[0].generation());
    reply(&center, surface, second.id, true);
    assert_eq!(
        center.read_with(surface, |center, _| center.copy_status(current)),
        copy::CopyStatus::Idle
    );
    center.update(surface, |center, cx| center.request_copy("retry", old, cx));
    assert!(endpoint.try_receive_command_for_contract_test().is_err());
}

#[gpui_kit::test]
fn notification_copy_timeout_disconnect_and_closed_generation_are_retryable(
    cx: &mut TestAppContext,
) {
    let (center, surface) = mount(cx);
    let endpoint = bind_platform(&center, surface);
    center.update(surface, |center, cx| {
        center.show_progress("cleanup", "标题", "正文", cx)
    });
    let generation = center.read_with(surface, |center, _| center.items()[0].generation());
    center.update(surface, |center, cx| {
        center.request_copy("cleanup", generation, cx)
    });
    let command = endpoint
        .try_receive_command_for_contract_test()
        .expect("first");
    tick(surface, 16000);
    assert_eq!(
        center.read_with(surface, |center, _| center.copy_status(generation)),
        copy::CopyStatus::Failed
    );
    center.update(surface, |center, cx| {
        center.request_copy("cleanup", generation, cx)
    });
    let second = endpoint
        .try_receive_command_for_contract_test()
        .expect("after timeout");
    reply(&center, surface, command.id, true);
    assert_eq!(
        center.read_with(surface, |center, _| center.copy_status(generation)),
        copy::CopyStatus::Pending
    );
    center.update(surface, |center, cx| {
        center.handle_copy(
            nxtools_ipc::PlatformNotification::Disconnected {
                generation: 1,
                pending_command_ids: vec![second.id],
            },
            cx,
        )
    });
    assert_eq!(
        center.read_with(surface, |center, _| center.copy_status(generation)),
        copy::CopyStatus::Failed
    );
    center.update(surface, |center, cx| {
        center.completed_close("cleanup", generation, cx);
        center.request_copy("cleanup", generation, cx);
    });
    assert!(endpoint.try_receive_command_for_contract_test().is_err());
}

#[gpui_kit::test]
fn notification_copy_does_not_reset_remaining_official_lifetime(cx: &mut TestAppContext) {
    let (center, surface) = mount(cx);
    let endpoint = bind_platform(&center, surface);
    center.update(surface, |center, cx| {
        center.set_duration_seconds(2, cx);
        center.show_info("expiry-copy", "标题", "正文", cx);
    });
    tick(surface, 1500);
    let generation = center.read_with(surface, |center, _| center.items()[0].generation());
    center.update(surface, |center, cx| {
        center.request_copy("expiry-copy", generation, cx)
    });
    let command = endpoint
        .try_receive_command_for_contract_test()
        .expect("copy");
    reply(&center, surface, command.id, true);
    tick(surface, 1400);
    assert!(
        center.read_with(surface, |center, _| center.items().is_empty()),
        "copy result cannot repush/reset original clock"
    );
}

#[gpui_kit::test]
fn notification_copy_rejects_oversized_text_without_partial_clipboard_command(
    cx: &mut TestAppContext,
) {
    let (center, surface) = mount(cx);
    let endpoint = bind_platform(&center, surface);
    center.update(surface, |center, cx| {
        center.show_progress("huge", "标题", "中".repeat(100_000), cx)
    });
    let generation = center.read_with(surface, |center, _| center.items()[0].generation());
    center.update(surface, |center, cx| {
        center.request_copy("huge", generation, cx)
    });
    assert!(endpoint.try_receive_command_for_contract_test().is_err());
    assert_eq!(
        center.read_with(surface, |center, _| center.copy_status(generation)),
        copy::CopyStatus::Failed
    );
}

#[gpui_kit::test]
fn notification_copy_keyboard_uses_original_button_release_once(cx: &mut TestAppContext) {
    let (center, surface) = mount(cx);
    let endpoint = bind_platform(&center, surface);
    center.update(surface, |center, cx| {
        center.show_progress("keys", "标题", "正文", cx)
    });
    tick(surface, 500);
    let card = official_cards(surface)[0].bounds();
    surface.simulate_mouse_move(card.center(), None, Default::default());
    draw(surface);
    for _ in 0..8 {
        surface.update(|window, cx| window.focus_next(cx));
        draw(surface);
        let focused = surface.update(|window, _| {
            gpui_kit::base::test_support::snapshots(window)
                .iter()
                .any(|node| node.label() == Some("复制通知内容") && node.focused() == Some(true))
        });
        if focused {
            break;
        }
    }
    assert!(surface.update(|window, _| {
        gpui_kit::base::test_support::snapshots(window)
            .iter()
            .any(|node| node.label() == Some("复制通知内容") && node.focused() == Some(true))
    }));
    let focused = surface.update(|window, cx| window.focused(cx).expect("copy owns focus"));
    surface.update(|window, cx| {
        window.dispatch_event(
            gpui_kit::PlatformInput::KeyDown(gpui_kit::KeyDownEvent {
                keystroke: gpui_kit::Keystroke::parse("space").expect("Space"),
                is_held: false,
                prefer_character_input: false,
            }),
            cx,
        );
        assert!(focused.is_focused(window), "copy focus survives key down");
        assert!(endpoint.try_receive_command_for_contract_test().is_err());
        window.dispatch_event(
            gpui_kit::PlatformInput::KeyUp(gpui_kit::KeyUpEvent {
                keystroke: gpui_kit::Keystroke::parse("space").expect("Space"),
            }),
            cx,
        );
    });
    surface.run_until_parked();
    let command = endpoint
        .try_receive_command_for_contract_test()
        .expect("one keyboard copy");
    assert!(matches!(
        command.message,
        nxtools_protocol::Message::PlatformCommand(nxtools_protocol::PlatformCommand::PasteText(_))
    ));
    assert!(endpoint.try_receive_command_for_contract_test().is_err());
}

#[gpui_kit::test]
fn notification_copy_long_title_keeps_text_clear_of_header_actions(cx: &mut TestAppContext) {
    let (center, surface) = mount(cx);
    surface.simulate_resize(gpui_kit::size(px(300.), px(440.)));
    center.update(surface, |center, cx| {
        center.show_progress(
            "long-title",
            "长标题需要换行并且不能覆盖复制关闭按钮".repeat(4),
            "正文",
            cx,
        )
    });
    tick(surface, 500);
    let card = official_cards(surface)[0].bounds();
    surface.simulate_mouse_move(card.center(), None, Default::default());
    draw(surface);
    let copy = surface
        .debug_bounds("notification-copy-button-long-title")
        .expect("copy");
    let text = surface
        .debug_bounds("notification-title-text")
        .expect("actual title text container");
    assert!(text.right() <= copy.left(), "title={text:?}, copy={copy:?}");
}
