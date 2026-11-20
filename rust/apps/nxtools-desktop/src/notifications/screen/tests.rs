use super::*;
use gpui_kit::{TestAppContext, VisualTestContext};

#[test]
fn passive_notification_options_do_not_throttle_official_animation_to_30fps() {
    let options = notification_window_options(None);
    assert!(!options.focus && !options.show);
    assert!(matches!(options.kind, WindowKind::PopUp));
    assert_eq!(
        options.inactive_frame_interval, None,
        "passive notification animation must use compositor cadence, not the inactive 33.333ms default"
    );
}

struct MainFixture {
    center: Entity<NotificationCenter>,
}
impl Render for MainFixture {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .child(crate::notifications::presentation::render(
                self.center.clone(),
                true,
                48.0,
                window,
                cx,
            ))
    }
}
fn fixture(
    cx: &mut TestAppContext,
) -> (
    Entity<NotificationCenter>,
    Entity<ScreenHost>,
    nxtools_ipc::PlatformServerEndpoint,
    AnyWindowHandle,
) {
    cx.update(gpui_kit::init);
    cx.executor().allow_parking();
    cx.update(|cx| cx.set_reduce_motion(true));
    let center = cx.new(|_| NotificationCenter::default());
    let rendered = center.clone();
    let (_, source) = cx.add_window_view(move |window, cx| {
        gpui_kit::base::Root::new(cx.new(|_| MainFixture { center: rendered }), window, cx)
    });
    let source = source.update(|window, _| window.window_handle());
    let (bridge, endpoint) = nxtools_ipc::platform_bridge(16);
    endpoint.set_connected_for_contract_test(true);
    cx.update(|cx| attach(center.clone(), Rc::new(bridge), source, cx));
    let host = cx.read(|cx| cx.global::<ScreenGlobal>().0.clone());
    (center, host, endpoint, source)
}
fn result(host: &Entity<ScreenHost>, id: Uuid, data: Option<String>, cx: &mut TestAppContext) {
    host.update(cx, |host, cx| {
        host.handle(
            PlatformNotification::CommandResult {
                correlation_id: id,
                result: nxtools_protocol::PlatformCommandResult {
                    success: true,
                    error: None,
                    native_error_code: None,
                    data,
                },
            },
            cx,
        )
    });
}
#[gpui_kit::test]
fn screen_notifications_do_not_render_inside_main_window_and_prepare_independent_popup(
    cx: &mut TestAppContext,
) {
    let (center, host, endpoint, source) = fixture(cx);
    center.update(cx, |center, cx| {
        center.show_progress("screen", "合成通知", "相对屏幕", cx)
    });
    cx.run_until_parked();
    let command = endpoint
        .try_receive_command_for_contract_test()
        .expect("screen prepare");
    assert!(
        matches!(&command.message,nxtools_protocol::Message::PlatformCommand(PlatformCommand::PrepareNotificationWindow(p)) if p.position=="top_center"&&p.width==360&&p.height<=376&&p.window_handle!=p.source_window_handle)
    );
    let surface = host.read_with(cx, |host, _| host.surface.as_ref().expect("popup").window);
    assert_ne!(surface, source);
    let mut main = VisualTestContext::from_window(source, cx);
    main.update(|window, cx| window.draw(cx).clear(cx));
    assert!(
        main.debug_bounds("notification-stack").is_none(),
        "screen notice must not duplicate the rejected in-window layer"
    );
    assert!(
        endpoint.try_receive_command_for_contract_test().is_err(),
        "never reveal before native preparation"
    );
}
#[gpui_kit::test]
fn screen_notification_all_positions_use_native_source_monitor_contract_and_stale_prepare_cannot_reveal(
    cx: &mut TestAppContext,
) {
    let (center, host, endpoint, _) = fixture(cx);
    center.update(cx, |center, cx| {
        center.show_progress("screen", "合成通知", "位置", cx)
    });
    cx.run_until_parked();
    let old = endpoint
        .try_receive_command_for_contract_test()
        .expect("first prepare");
    center.update(cx, |center, cx| {
        center.set_position(NotificationPosition::TopCenter, cx)
    });
    cx.run_until_parked();
    result(
        &host,
        old.id,
        Some("{\"x\":-360,\"y\":40,\"width\":360,\"height\":136,\"dpi\":96}".into()),
        cx,
    );
    let fresh = endpoint
        .try_receive_command_for_contract_test()
        .expect("new prepare not reveal");
    assert!(
        matches!(&fresh.message,nxtools_protocol::Message::PlatformCommand(PlatformCommand::PrepareNotificationWindow(p)) if p.position=="top_center")
    );
    result(&host, old.id, None, cx);
    assert!(
        endpoint.try_receive_command_for_contract_test().is_err(),
        "old result cannot dispatch"
    );
    host.update(cx, |host, cx| {
        host.handle(
            PlatformNotification::Disconnected {
                generation: 1,
                pending_command_ids: vec![fresh.id],
            },
            cx,
        )
    });
    cx.run_until_parked();
    assert!(host.read_with(cx, |host, _| host.surface.is_none()
        && host.pending.is_none()));
    result(
        &host,
        fresh.id,
        Some("{\"width\":360,\"height\":136,\"dpi\":96}".into()),
        cx,
    );
    assert!(
        endpoint.try_receive_command_for_contract_test().is_err(),
        "disconnect retires native reveal"
    );
    for position in NotificationPosition::ALL {
        assert_eq!(
            serde_json::to_value(position).expect("enum").as_str(),
            Some(position_name(position))
        );
    }
}

#[gpui_kit::test]
fn screen_notification_prepare_frame_reveal_and_empty_hide_reuse_one_surface(
    cx: &mut TestAppContext,
) {
    let (center, host, endpoint, source) = fixture(cx);
    center.update(cx, |center, cx| {
        center.show_progress("flow", "合成通知", "完整生命周期", cx)
    });
    cx.run_until_parked();
    let surface = host.read_with(cx, |host, _| host.surface.as_ref().expect("surface").window);
    let mut revealed = None;
    for _ in 0..8 {
        let command = endpoint
            .try_receive_command_for_contract_test()
            .expect("bounded next phase");
        match command.message {
            nxtools_protocol::Message::PlatformCommand(
                PlatformCommand::PrepareNotificationWindow(prepare),
            ) => {
                let mut popup = VisualTestContext::from_window(surface, cx);
                popup.simulate_scale_factor_change(1.0);
                popup.simulate_resize(size(px(prepare.width as f32), px(prepare.height as f32)));
                // Ignore initial prepaint measurement until preparation is complete.
                result(
                    &host,
                    command.id,
                    Some(format!(
                        "{{\"width\":{},\"height\":{},\"dpi\":96}}",
                        prepare.width, prepare.height
                    )),
                    cx,
                );
                let phase = host.read_with(cx, |host, _| {
                    host.pending.as_ref().map(|pending| pending.phase)
                });
                assert!(
                    matches!(phase, Some(Phase::Frame | Phase::Prepare | Phase::Reveal)),
                    "prepare may synchronously build the hidden scene, never skip visibility acknowledgement"
                );
                assert!(
                    !host.read_with(cx, |host, _| host.visible),
                    "native visibility result has not arrived"
                );
                popup.update(|window, cx| {
                    window.draw(cx).clear(cx);
                    window.simulate_next_frame(cx);
                });
                cx.run_until_parked();
            }
            nxtools_protocol::Message::PlatformCommand(PlatformCommand::SetWindowVisibility(
                show,
            )) => {
                assert!(show.visible);
                result(&host, command.id, None, cx);
                revealed = Some(command.id);
                break;
            }
            _ => panic!("notification must use prepare or passive visibility only"),
        }
    }
    assert!(
        revealed.is_some(),
        "bounded preparation and measured frame must settle"
    );
    assert!(host.read_with(cx, |host, _| host.visible));
    assert_ne!(source, surface);
    let generation = center.read_with(cx, |center, _| center.items()[0].generation());
    center.update(cx, |center, cx| {
        center.dismiss_generation("flow", generation, cx)
    });
    cx.run_until_parked();
    for _ in 0..6 {
        cx.executor().advance_clock(Duration::from_millis(50));
        let mut popup = VisualTestContext::from_window(surface, cx);
        popup.update(|window, cx| window.draw(cx).clear(cx));
        cx.run_until_parked();
    }
    let hide = endpoint
        .try_receive_command_for_contract_test()
        .expect("empty hides");
    assert!(
        matches!(hide.message,nxtools_protocol::Message::PlatformCommand(PlatformCommand::SetWindowVisibility(show)) if !show.visible)
    );
    result(&host, hide.id, None, cx);
    assert_eq!(
        host.read_with(cx, |host, _| host
            .surface
            .as_ref()
            .expect("reused hidden surface")
            .window),
        surface
    );
    assert!(!host.read_with(cx, |host, _| host.visible));
}

#[gpui_kit::test]
fn screen_notification_timeout_removes_old_surface_and_late_reply_cannot_show(
    cx: &mut TestAppContext,
) {
    let (center, host, endpoint, _) = fixture(cx);
    center.update(cx, |center, cx| {
        center.show_progress("timeout", "合成通知", "等待定位", cx)
    });
    cx.run_until_parked();
    let prepare = endpoint
        .try_receive_command_for_contract_test()
        .expect("prepare");
    cx.executor().advance_clock(Duration::from_secs(6));
    cx.run_until_parked();
    assert!(host.read_with(cx, |host, _| host.surface.is_none()
        && host.pending.is_none()));
    result(
        &host,
        prepare.id,
        Some("{\"width\":360,\"height\":136,\"dpi\":96}".into()),
        cx,
    );
    assert!(endpoint.try_receive_command_for_contract_test().is_err());
}

#[gpui_kit::test]
fn screen_notification_visible_surface_hides_before_repositioning(cx: &mut TestAppContext) {
    let (center, host, endpoint, _) = fixture(cx);
    center.update(cx, |center, cx| {
        center.show_progress("reuse", "合成通知", "屏幕位置", cx)
    });
    cx.run_until_parked();
    let prepare = endpoint
        .try_receive_command_for_contract_test()
        .expect("initial prepare");
    host.update(cx, |host, _| {
        host.pending = None;
        host.visible = true;
    });
    center.update(cx, |center, cx| {
        center.set_position(NotificationPosition::TopLeft, cx)
    });
    cx.run_until_parked();
    let hide = endpoint
        .try_receive_command_for_contract_test()
        .expect("hide before prepare");
    assert!(
        matches!(hide.message,nxtools_protocol::Message::PlatformCommand(PlatformCommand::SetWindowVisibility(show)) if !show.visible)
    );
    result(&host, hide.id, None, cx);
    let next = endpoint
        .try_receive_command_for_contract_test()
        .expect("prepare after hide acknowledgement");
    assert!(
        matches!(next.message,nxtools_protocol::Message::PlatformCommand(PlatformCommand::PrepareNotificationWindow(p)) if p.position=="top_left")
    );
    result(&host, prepare.id, None, cx);
    assert!(endpoint.try_receive_command_for_contract_test().is_err());
}

#[gpui_kit::test]
fn screen_notification_mismatched_viewport_never_reveals(cx: &mut TestAppContext) {
    let (center, host, endpoint, _) = fixture(cx);
    center.update(cx, |center, cx| {
        center.show_progress("mismatch", "合成通知", "等待正确视口", cx)
    });
    cx.run_until_parked();
    let prepare = endpoint
        .try_receive_command_for_contract_test()
        .expect("prepare");
    let surface = host.read_with(cx, |host, _| host.surface.as_ref().expect("popup").window);
    result(
        &host,
        prepare.id,
        Some("{\"width\":720,\"height\":272,\"dpi\":192}".into()),
        cx,
    );
    let mut popup = VisualTestContext::from_window(surface, cx);
    popup.simulate_scale_factor_change(1.0);
    popup.simulate_resize(size(px(360.0), px(136.0)));
    popup.update(|window, cx| {
        window.draw(cx).clear(cx);
        window.simulate_next_frame(cx);
    });
    cx.run_until_parked();
    while let Ok(command) = endpoint.try_receive_command_for_contract_test() {
        assert!(
            !matches!(command.message,nxtools_protocol::Message::PlatformCommand(PlatformCommand::SetWindowVisibility(show)) if show.visible),
            "wrong device viewport must never reveal"
        );
    }
}

#[gpui_kit::test]
fn screen_notification_action_runs_in_original_source_not_popup(cx: &mut TestAppContext) {
    let (center, host, _endpoint, source) = fixture(cx);
    let invoked = Rc::new(std::cell::Cell::new(None));
    let callback = invoked.clone();
    center.update(cx, |center, cx| {
        center.push(
            AppNotification::new("action", NotificationLevel::Success, "完成", "原窗口动作")
                .actions(vec![NotificationAction::new("详情", move |window, _| {
                    callback.set(Some(window.window_handle()))
                })])
                .duration(None),
            cx,
        )
    });
    cx.run_until_parked();
    let surface = host.read_with(cx, |host, _| host.surface.as_ref().expect("popup").window);
    let mut popup = VisualTestContext::from_window(surface, cx);
    popup.simulate_resize(size(px(360.0), px(136.0)));
    popup.update(|window, cx| window.draw(cx).clear(cx));
    let bounds = popup
        .debug_bounds("notification-action-action")
        .expect("real action control");
    popup.simulate_click(bounds.center(), gpui_kit::Modifiers::default());
    popup.run_until_parked();
    assert_eq!(invoked.get(), Some(source));
    assert_ne!(invoked.get(), Some(surface));
}

#[gpui_kit::test]
fn notification_exit_never_hides_or_reprepares_surface_before_animation_finishes(
    cx: &mut TestAppContext,
) {
    let (center, host, endpoint, _) = fixture(cx);
    cx.update(|cx| {
        cx.set_global(crate::ui::motion::UiMotionPreference::Known {
            reduced_motion: false,
        })
    });
    center.update(cx, |center, cx| {
        center.show_progress("exit-stable", "完成", "退出时不重建浮窗", cx)
    });
    cx.run_until_parked();
    let _ = endpoint
        .try_receive_command_for_contract_test()
        .expect("initial prepare");
    host.update(cx, |host, _| {
        host.pending = None;
        host.visible = true;
    });
    let generation = center.read_with(cx, |center, _| center.items()[0].generation());
    center.update(cx, |center, cx| {
        center.dismiss_generation("exit-stable", generation, cx)
    });
    cx.run_until_parked();
    assert!(
        endpoint.try_receive_command_for_contract_test().is_err(),
        "exit must animate in the existing HWND, not hide/reprepare/reveal it"
    );
    assert!(host.read_with(cx, |host, _| host.visible));
}

#[gpui_kit::test]
fn visible_notification_content_resize_adjusts_existing_surface_without_hide_or_reveal(
    cx: &mut TestAppContext,
) {
    let (center, host, endpoint, _) = fixture(cx);
    center.update(cx, |center, cx| {
        center.show_progress("resize", "处理中", "原位更新", cx)
    });
    cx.run_until_parked();
    let surface = host.read_with(cx, |host, _| host.surface.as_ref().expect("surface").window);
    let mut ready = false;
    for _ in 0..8 {
        let command = endpoint
            .try_receive_command_for_contract_test()
            .expect("initial lifecycle");
        match command.message {
            nxtools_protocol::Message::PlatformCommand(
                PlatformCommand::PrepareNotificationWindow(p),
            ) => {
                let popup = VisualTestContext::from_window(surface, cx);
                popup.simulate_scale_factor_change(1.0);
                popup.simulate_resize(size(px(p.width as f32), px(p.height as f32)));
                result(
                    &host,
                    command.id,
                    Some(format!(
                        "{{\"width\":{},\"height\":{},\"dpi\":96}}",
                        p.width, p.height
                    )),
                    cx,
                );
                cx.run_until_parked();
            }
            nxtools_protocol::Message::PlatformCommand(PlatformCommand::SetWindowVisibility(p)) => {
                assert!(p.visible);
                result(&host, command.id, None, cx);
                ready = true;
                break;
            }
            _ => panic!("unexpected initial command"),
        }
    }
    assert!(ready && host.read_with(cx, |host, _| host.visible));
    host.update(cx, |host, cx| host.measured(180.0, cx));
    let adjustment = endpoint
        .try_receive_command_for_contract_test()
        .expect("visible adjustment");
    assert!(
        matches!(adjustment.message, nxtools_protocol::Message::PlatformCommand(PlatformCommand::PrepareNotificationWindow(p)) if p.keep_visible && p.height == 180)
    );
    let popup = VisualTestContext::from_window(surface, cx);
    popup.simulate_scale_factor_change(1.0);
    popup.simulate_resize(size(px(360.0), px(180.0)));
    result(
        &host,
        adjustment.id,
        Some("{\"width\":360,\"height\":180,\"dpi\":96}".into()),
        cx,
    );
    cx.run_until_parked();
    while let Ok(command) = endpoint.try_receive_command_for_contract_test() {
        assert!(
            !matches!(
                command.message,
                nxtools_protocol::Message::PlatformCommand(PlatformCommand::SetWindowVisibility(_))
            ),
            "resize must not hide or reveal an already visible surface"
        );
    }
    assert!(host.read_with(cx, |host, _| host.visible));
    assert_eq!(
        host.read_with(cx, |host, _| host.surface.as_ref().expect("reused").window),
        surface
    );
}

#[gpui_kit::test]
fn notification_replacement_during_final_hide_reprepares_after_ack(cx: &mut TestAppContext) {
    let (center, host, endpoint, _) = fixture(cx);
    center.update(cx, |center, cx| {
        center.show_progress("hide-race", "旧", "合成", cx)
    });
    cx.run_until_parked();
    let _ = endpoint
        .try_receive_command_for_contract_test()
        .expect("prepare");
    host.update(cx, |host, cx| {
        host.pending = None;
        host.dirty = false;
        host.visible = true;
        host.send_visibility(false, cx);
    });
    let hide = endpoint
        .try_receive_command_for_contract_test()
        .expect("hide");
    center.update(cx, |center, cx| {
        center.show_progress("hide-race", "新", "隐藏确认期间替换", cx)
    });
    cx.run_until_parked();
    assert!(
        endpoint.try_receive_command_for_contract_test().is_err(),
        "wait for hide acknowledgement"
    );
    result(&host, hide.id, None, cx);
    cx.run_until_parked();
    let prepare = endpoint
        .try_receive_command_for_contract_test()
        .expect("replacement cannot remain hidden");
    assert!(
        matches!(prepare.message, nxtools_protocol::Message::PlatformCommand(PlatformCommand::PrepareNotificationWindow(p)) if !p.keep_visible)
    );
}

fn settle_native_fixture(
    host: &Entity<ScreenHost>,
    endpoint: &nxtools_ipc::PlatformServerEndpoint,
    surface: AnyWindowHandle,
    cx: &mut TestAppContext,
    steps: usize,
) {
    for _ in 0..steps {
        let mut popup = VisualTestContext::from_window(surface, cx);
        popup.update(|window, cx| {
            window.refresh();
            window.draw(cx).clear(cx);
            window.simulate_next_frame(cx);
        });
        cx.run_until_parked();
        while let Ok(command) = endpoint.try_receive_command_for_contract_test() {
            match command.message {
                nxtools_protocol::Message::PlatformCommand(
                    PlatformCommand::PrepareNotificationWindow(p),
                ) => {
                    if host.read_with(cx, |host, _| host.visible) {
                        assert!(p.keep_visible, "visible stack resize cannot hide");
                    }
                    let popup = VisualTestContext::from_window(surface, cx);
                    popup.simulate_scale_factor_change(1.0);
                    popup.simulate_resize(size(px(p.width as f32), px(p.height as f32)));
                    result(
                        host,
                        command.id,
                        Some(format!(
                            "{{\"width\":{},\"height\":{},\"dpi\":96}}",
                            p.width, p.height
                        )),
                        cx,
                    );
                }
                nxtools_protocol::Message::PlatformCommand(
                    PlatformCommand::SetWindowVisibility(p),
                ) => {
                    assert!(
                        p.visible || host.read_with(cx, |host, cx| host.snapshot(cx).is_none()),
                        "only terminal empty may hide; stack interaction cannot hide/reveal",
                    );
                    result(host, command.id, None, cx);
                }
                _ => panic!("unexpected notification native command"),
            }
            cx.run_until_parked();
        }
        cx.executor().advance_clock(Duration::from_millis(50));
        cx.run_until_parked();
    }
}
#[gpui_kit::test]
fn official_stack_hover_resizes_native_viewport_and_collapses_without_transparent_catcher(
    cx: &mut TestAppContext,
) {
    let (center, host, endpoint, _) = fixture(cx);
    for id in ["stack-a", "stack-b", "stack-c"] {
        center.update(cx, |center, cx| {
            center.show_progress(id, "不同操作", "官方组件真实叠层", cx)
        });
    }
    cx.run_until_parked();
    let surface = host.read_with(cx, |host, _| host.surface.as_ref().expect("surface").window);
    settle_native_fixture(&host, &endpoint, surface, cx, 16);
    let mut popup = VisualTestContext::from_window(surface, cx);
    let collapsed_height = popup.update(|window, _| window.viewport_size().height);
    let cards = super::super::presentation_tests::official_cards(&mut popup);
    assert_eq!(cards.len(), 3);
    let front = cards
        .iter()
        .max_by(|a, b| {
            a.bounds()
                .size
                .width
                .partial_cmp(&b.bounds().size.width)
                .expect("finite")
        })
        .expect("front")
        .bounds();
    popup.simulate_mouse_move(front.center(), None, Default::default());
    settle_native_fixture(&host, &endpoint, surface, cx, 16);
    let mut popup = VisualTestContext::from_window(surface, cx);
    let expanded_height = popup.update(|window, _| window.viewport_size().height);
    assert!(
        expanded_height > collapsed_height + px(70.0),
        "native viewport must grow: {collapsed_height:?}->{expanded_height:?}"
    );
    let mut cards = super::super::presentation_tests::official_cards(&mut popup)
        .into_iter()
        .map(|card| card.bounds())
        .collect::<Vec<_>>();
    cards.sort_by(|a, b| a.top().partial_cmp(&b.top()).expect("finite"));
    assert!(
        cards
            .windows(2)
            .all(|cards| cards[0].bottom() <= cards[1].top()),
        "expanded official cards must not overlap: {cards:?}"
    );
    assert!(
        cards
            .iter()
            .all(|card| card.top() >= px(0.0) && card.bottom() <= expanded_height),
        "every expanded card inside actual native viewport: {cards:?}"
    );
    popup.simulate_mouse_move(point(px(2.0), px(2.0)), None, Default::default());
    settle_native_fixture(&host, &endpoint, surface, cx, 20);
    let mut popup = VisualTestContext::from_window(surface, cx);
    let restored = popup.update(|window, _| window.viewport_size().height);
    assert!(
        (restored - collapsed_height).abs() <= px(1.0),
        "must release expanded transparent area: {restored:?} vs {collapsed_height:?}"
    );
    assert!(host.read_with(cx, |host, _| host.visible));
    let front = super::super::presentation_tests::official_cards(&mut popup)
        .into_iter()
        .max_by(|a, b| {
            a.bounds()
                .size
                .width
                .partial_cmp(&b.bounds().size.width)
                .expect("finite")
        })
        .expect("front")
        .bounds();
    popup.simulate_mouse_move(front.center(), None, Default::default());
    settle_native_fixture(&host, &endpoint, surface, cx, 16);
    center.update(cx, |center, cx| {
        let live = center
            .items()
            .iter()
            .map(|item| (item.id.clone(), item.generation()))
            .collect::<Vec<_>>();
        for (id, generation) in live {
            center.dismiss_generation(id, generation, cx);
        }
    });
    settle_native_fixture(&host, &endpoint, surface, cx, 24);
    assert!(
        !host.read_with(cx, |host, _| host.visible),
        "official final close hides the empty stack"
    );
    for id in ["reopen-a", "reopen-b", "reopen-c"] {
        center.update(cx, |center, cx| {
            center.show_progress(id, "新操作", "不能继承旧展开状态", cx)
        });
    }
    settle_native_fixture(&host, &endpoint, surface, cx, 20);
    let mut popup = VisualTestContext::from_window(surface, cx);
    assert!(
        (popup.update(|window, _| window.viewport_size().height) - collapsed_height).abs()
            <= px(1.0),
        "reopened stack must not retain expanded transparent area"
    );
}

#[gpui_kit::test]
fn official_stack_keyboard_focus_grows_native_viewport_without_pointer_hover(
    cx: &mut TestAppContext,
) {
    let (center, host, endpoint, _) = fixture(cx);
    for id in ["focus-a", "focus-b", "focus-c"] {
        center.update(cx, |center, cx| {
            center.push(
                AppNotification::new(id, NotificationLevel::Progress, "键盘操作", "无需鼠标悬停")
                    .actions(vec![NotificationAction::new("详情", |_, _| {})]),
                cx,
            )
        });
    }
    cx.run_until_parked();
    let surface = host.read_with(cx, |host, _| host.surface.as_ref().expect("surface").window);
    settle_native_fixture(&host, &endpoint, surface, cx, 16);
    let mut popup = VisualTestContext::from_window(surface, cx);
    let collapsed = popup.update(|window, _| window.viewport_size().height);
    let mut focused = false;
    for _ in 0..16 {
        focused = popup.update(|window, _| {
            gpui_kit::base::test_support::snapshots(window)
                .iter()
                .any(|node| {
                    node.path()
                        .iter()
                        .any(|part| part == &"notification-action-focus-a".into())
                        && node.role() == Some(gpui_kit::Role::Button)
                        && node.focused() == Some(true)
                })
        });
        if focused {
            break;
        }
        popup.update(|window, cx| {
            window.focus_next(cx);
            window.refresh();
            window.draw(cx).clear(cx);
        });
    }
    assert!(
        focused,
        "real official action must be focusable without pointer"
    );
    settle_native_fixture(&host, &endpoint, surface, cx, 16);
    let mut popup = VisualTestContext::from_window(surface, cx);
    let expanded = popup.update(|window, _| window.viewport_size().height);
    assert!(
        expanded > collapsed + px(70.0),
        "keyboard expansion needs native space: {collapsed:?}->{expanded:?}"
    );
    let cards = super::super::presentation_tests::official_cards(&mut popup);
    assert!(
        cards
            .iter()
            .all(|card| card.bounds().top() >= px(0.0) && card.bounds().bottom() <= expanded)
    );
    center.update(cx, |center, cx| {
        let generation = center
            .items()
            .iter()
            .find(|item| item.id == "focus-a")
            .expect("focused notice")
            .generation();
        assert!(center.invoke_action("focus-a", generation, 0, cx).is_some());
    });
    settle_native_fixture(&host, &endpoint, surface, cx, 24);
    let mut popup = VisualTestContext::from_window(surface, cx);
    assert_eq!(
        center.read_with(&popup, |center, _| center.items().len()),
        2
    );
    assert!(
        popup.update(|window, _| window.viewport_size().height) <= collapsed,
        "retiring the focused card must release expanded transparent area without a blur event"
    );
    popup.update(|window, cx| window.blur(cx));
    settle_native_fixture(&host, &endpoint, surface, cx, 20);
}

#[gpui_kit::test]
fn notification_sizing_multiline_drives_real_host_height_and_all_text_is_visible(
    cx: &mut TestAppContext,
) {
    let (center, host, endpoint, _) = fixture(cx);
    center.update(cx, |center, cx| {
        center.show_progress(
            "currency-sizing",
            "汇率换算",
            "5 CAD ≈ 23.73 CNY\n参考汇率 · 2026-10-20 · ExchangeRate-API",
            cx,
        )
    });
    cx.run_until_parked();
    let surface = host.read_with(cx, |host, _| host.surface.as_ref().expect("surface").window);
    settle_native_fixture(&host, &endpoint, surface, cx, 16);
    let mut popup = VisualTestContext::from_window(surface, cx);
    let content = popup
        .debug_bounds("notification-content-currency-sizing")
        .expect("copy viewport");
    assert!(
        content.size.height > px(70.0),
        "real host must not measure clipped54px text: {content:?}"
    );
    let viewport = popup.update(|window, _| window.viewport_size());
    assert!(
        viewport.height > px(124.0),
        "nativeallocation grows with multiline: {viewport:?}"
    );
    assert!(content.bottom() < viewport.height - px(12.0));
    let body = popup
        .debug_bounds("notification-natural-text-body")
        .expect("full text");
    assert!(
        body.top() >= content.top() && body.bottom() <= content.bottom() + px(1.0),
        "alltext visibleinactualnativeallocation {body:?} vs {content:?}"
    );
}

#[gpui_kit::test]
fn notification_sizing_three_long_cards_keep_actions_inside_bounded_native_stack(
    cx: &mut TestAppContext,
) {
    let (center, host, endpoint, _) = fixture(cx);
    for id in ["long-a", "long-b", "long-c"] {
        center.update(cx, |center, cx| {
            center.push(
                AppNotification::new(
                    id,
                    NotificationLevel::Progress,
                    "长通知",
                    "合成多行内容\n".repeat(100),
                )
                .actions(vec![
                    NotificationAction::new("查看结果", |_, _| {}),
                    NotificationAction::new("详情", |_, _| {}),
                ]),
                cx,
            )
        });
    }
    cx.run_until_parked();
    let surface = host.read_with(cx, |host, _| host.surface.as_ref().expect("surface").window);
    settle_native_fixture(&host, &endpoint, surface, cx, 18);
    let mut popup = VisualTestContext::from_window(surface, cx);
    let front = crate::notifications::presentation_tests::official_cards(&mut popup)
        .into_iter()
        .max_by(|a, b| {
            a.bounds()
                .size
                .width
                .partial_cmp(&b.bounds().size.width)
                .expect("finite")
        })
        .expect("front")
        .bounds();
    popup.simulate_mouse_move(front.center(), None, Default::default());
    settle_native_fixture(&host, &endpoint, surface, cx, 20);
    let viewport = popup.update(|window, _| window.viewport_size());
    assert!(viewport.height <= px(400.0));
    let cards = crate::notifications::presentation_tests::official_cards(&mut popup);
    assert_eq!(cards.len(), 3);
    for card in cards {
        let card = card.bounds();
        assert!(
            card.top() >= px(0.0) && card.bottom() <= viewport.height,
            "boundedofficialcard {card:?} vs {viewport:?}"
        );
    }
    for (copy_selector, action_selector) in [
        ("notification-content-long-a", "notification-action-long-a"),
        ("notification-content-long-b", "notification-action-long-b"),
        ("notification-content-long-c", "notification-action-long-c"),
    ] {
        let copy = popup.debug_bounds(copy_selector).expect("copy");
        let first = popup.debug_bounds(action_selector).expect("action");
        assert!(copy.bottom() <= first.top(), "scroll keepsactionoutside");
        assert!(
            first.bottom() <= viewport.height - px(12.0),
            "actions remainreachable"
        );
    }
}
