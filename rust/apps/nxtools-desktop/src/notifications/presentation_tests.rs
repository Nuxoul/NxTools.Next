use super::*;
use gpui_kit::{
    AppContext, IntoElement, Render, TestAppContext, VisualTestContext, div, prelude::*, px, size,
};

struct Host {
    center: Entity<NotificationCenter>,
    main: bool,
}
impl Render for Host {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div().size_full().relative().child(presentation::render(
            self.center.clone(),
            self.main,
            48.0,
            window,
            cx,
        ))
    }
}
pub(crate) fn mount(
    cx: &mut TestAppContext,
) -> (Entity<NotificationCenter>, &mut VisualTestContext) {
    cx.update(gpui_kit::init);
    cx.executor().allow_parking();
    cx.update(|cx| cx.set_reduce_motion(true));
    let center = cx.new(|_| NotificationCenter::default());
    let rendered = center.clone();
    let (_, surface) = cx.add_window_view(move |window, cx| {
        gpui_kit::base::Root::new(
            cx.new(|_| Host {
                center: rendered,
                main: true,
            }),
            window,
            cx,
        )
    });
    surface.simulate_resize(size(px(480.0), px(500.0)));
    surface.update(|window, _| window.activate_window());
    draw(surface);
    (center, surface)
}
pub(crate) fn draw(surface: &mut VisualTestContext) {
    surface.update(|window, cx| {
        window.refresh();
        window.draw(cx).clear(cx);
        window.simulate_next_frame(cx);
    });
    surface.run_until_parked();
}
pub(crate) fn tick(surface: &mut VisualTestContext, ms: u64) {
    for _ in 0..ms.div_ceil(50) {
        surface.executor().advance_clock(Duration::from_millis(50));
        surface.run_until_parked();
        draw(surface);
    }
}
pub(super) fn official_cards(
    surface: &mut VisualTestContext,
) -> Vec<gpui_kit::base::test_support::ElementSnapshot> {
    surface.update(|window, _| {
        gpui_kit::base::test_support::snapshots(window)
            .into_iter()
            .filter(|node| {
                node.role() == Some(gpui_kit::Role::Alert)
                    && node.path().last() == Some(&"notification".into())
            })
            .collect()
    })
}
#[gpui_kit::test]
fn approved_official_notification_card_is_rendered_instead_of_custom_alert_shell(
    cx: &mut TestAppContext,
) {
    let (center, surface) = mount(cx);
    center.update(surface, |center, cx| {
        center.show_progress("official-proof", "官方通知", "官方拥有堆叠与关闭", cx)
    });
    draw(surface);
    assert_eq!(
        official_cards(surface).len(),
        1,
        "must mount released Notification, not GroupBox/Alert substitute"
    );
}
#[gpui_kit::test]
fn official_stack_three_distinct_messages_expand_and_remain_bounded(cx: &mut TestAppContext) {
    let (center, surface) = mount(cx);
    for id in ["one", "two", "three"] {
        center.update(surface, |center, cx| {
            center.show_progress(id, "合成进度", "官方叠层", cx)
        });
    }
    tick(surface, 500);
    let collapsed = official_cards(surface);
    assert_eq!(collapsed.len(), 3);
    let front = collapsed
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
    surface.simulate_mouse_move(front.center(), None, Default::default());
    tick(surface, 600);
    let mut expanded = official_cards(surface)
        .into_iter()
        .map(|node| node.bounds())
        .collect::<Vec<_>>();
    expanded.sort_by(|a, b| a.top().partial_cmp(&b.top()).expect("finite"));
    assert!(
        expanded
            .windows(2)
            .all(|cards| cards[0].bottom() <= cards[1].top()),
        "official hover expands without hiding other messages: {expanded:?}"
    );
    center.update(surface, |center, cx| {
        center.show_progress("four", "第四条", "限制为三条", cx)
    });
    tick(surface, 500);
    assert_eq!(
        center.read_with(surface, |center, _| center.items().len()),
        3
    );
    assert_eq!(official_cards(surface).len(), 3);
}
#[gpui_kit::test]
fn all_six_positions_use_actual_official_card_bounds_in_both_themes(cx: &mut TestAppContext) {
    let (center, surface) = mount(cx);
    center.update(surface, |center, cx| {
        center.show_progress("anchor", "标题", "六个位置", cx)
    });
    for mode in [
        gpui_kit::component::ThemeMode::Light,
        gpui_kit::component::ThemeMode::Dark,
    ] {
        surface.update(|window, cx| gpui_kit::component::Theme::change(mode, Some(window), cx));
        for (width, height) in [(920.0, 480.0), (300.0, 440.0)] {
            surface.simulate_resize(size(px(width), px(height)));
            for position in NotificationPosition::ALL {
                center.update(surface, |center, cx| center.set_position(position, cx));
                tick(surface, 500);
                let card = official_cards(surface)
                    .into_iter()
                    .next()
                    .expect("official card")
                    .bounds();
                assert!(
                    card.left() >= px(11.0)
                        && card.right() <= px(width - 11.0)
                        && card.top() >= px(59.0)
                        && card.bottom() <= px(height - 11.0),
                    "{position:?} card={card:?}"
                );
                if position.is_top() {
                    assert!((card.top() - px(60.0)).abs() <= px(1.0));
                } else {
                    assert!((card.bottom() - px(height - 12.0)).abs() <= px(1.0));
                }
            }
        }
    }
}
#[gpui_kit::test]
fn official_actions_dispatch_once_in_original_source_and_stale_close_cannot_remove_replacement(
    cx: &mut TestAppContext,
) {
    let (center, surface) = mount(cx);
    let source = surface.update(|window, _| window.window_handle());
    let calls = Rc::new(std::cell::Cell::new(0));
    let invoked = Rc::new(std::cell::Cell::new(None));
    let counted = calls.clone();
    let target = invoked.clone();
    center.update(surface, |center, cx| {
        center.push(
            AppNotification::new("action", NotificationLevel::Progress, "操作", "两个动作")
                .actions(vec![
                    NotificationAction::new("查看结果", move |window, _| {
                        counted.set(counted.get() + 1);
                        target.set(Some(window.window_handle()));
                    }),
                    NotificationAction::new("详情", |_, _| {}),
                ]),
            cx,
        )
    });
    tick(surface, 500);
    let action = surface
        .debug_bounds("notification-action-action")
        .expect("real button");
    let generation = center.read_with(surface, |center, _| center.items()[0].generation());
    surface.simulate_click(action.center(), Default::default());
    surface.run_until_parked();
    assert_eq!(calls.get(), 1);
    assert_eq!(invoked.get(), Some(source));
    center.update(surface, |center, cx| {
        assert!(center.invoke_action("action", generation, 1, cx).is_none());
        center.show_progress("action", "替换", "新代次", cx);
        center.completed_close("action", generation, cx);
    });
    tick(surface, 500);
    assert_eq!(
        center.read_with(surface, |center, _| center.items()[0].title.clone()),
        "替换"
    );
}
#[gpui_kit::test]
fn long_content_two_buttons_and_official_close_stay_inside_frame(cx: &mut TestAppContext) {
    let (center, surface) = mount(cx);
    surface.simulate_resize(size(px(300.0), px(440.0)));
    center.update(surface, |center, cx| {
        center.push(
            AppNotification::new(
                "long",
                NotificationLevel::Progress,
                "长标题长标题",
                "长文本，保持滚动。".repeat(80),
            )
            .actions(vec![
                NotificationAction::new("查看结果", |_, _| {}),
                NotificationAction::new("详情", |_, _| {}),
            ]),
            cx,
        )
    });
    tick(surface, 500);
    let card = official_cards(surface)[0].bounds();
    for selector in ["notification-action-long", "notification-action-long-1"] {
        let button = surface.debug_bounds(selector).expect("real action");
        assert!(
            button.left() >= card.left()
                && button.right() <= card.right()
                && button.bottom() <= card.bottom(),
            "action={button:?} card={card:?}"
        );
    }
    surface.simulate_mouse_move(card.center(), None, Default::default());
    draw(surface);
    let close = surface.update(|window, _| {
        gpui_kit::base::test_support::snapshots(window)
            .into_iter()
            .find(|node| node.path().last() == Some(&"close".into()))
            .expect("official close")
            .bounds()
    });
    assert!(
        close.left() >= card.left() && close.right() <= card.right() && close.top() >= card.top()
    );
    surface.simulate_click(close.center(), Default::default());
    tick(surface, 350);
    assert!(
        center.read_with(surface, |center, _| center.items().is_empty()),
        "official completed close releases business metadata"
    );
}
#[gpui_kit::test]
fn expired_notification_is_removed_by_official_lifecycle_not_center_timer(cx: &mut TestAppContext) {
    let (center, surface) = mount(cx);
    center.update(surface, |center, cx| center.set_duration_seconds(5, cx));
    center.update(surface, |center, cx| {
        center.show_info("expiry", "提示", "五秒", cx)
    });
    tick(surface, 5300);
    assert_eq!(
        center.read_with(surface, |center, _| center.items().len()),
        1
    );
    tick(surface, 400);
    assert!(center.read_with(surface, |center, _| center.items().is_empty()));
    assert!(official_cards(surface).is_empty());
}

#[gpui_kit::test]
fn notification_sizing_currency_multiline_grows_without_initial_content_scroll(
    cx: &mut TestAppContext,
) {
    let (center, surface) = mount(cx);
    center.update(surface, |center, cx| {
        center.show_progress(
            "currency-sizing",
            "汇率换算",
            "5 CAD ≈ 23.73 CNY\n参考汇率 · 2026-10-20 · ExchangeRate-API",
            cx,
        )
    });
    tick(surface, 500);
    let content = surface
        .debug_bounds("notification-content-currency-sizing")
        .expect("actual text viewport");
    assert!(
        content.size.height > px(70.0),
        "title and multiline reference must grow,not clip at54: {content:?}"
    );
    let body = surface
        .debug_bounds("notification-natural-text-body")
        .expect("actual alltext bounds");
    assert!(
        body.top() >= content.top() && body.bottom() <= content.bottom() + px(1.0),
        "allcurrencytext initiallyvisible: {body:?} vs {content:?}"
    );
    let card = official_cards(surface)[0].bounds();
    assert!(
        content.bottom() <= card.bottom() - px(12.0),
        "complete text stays inside official frame"
    );
}

#[gpui_kit::test]
fn notification_sizing_short_copy_stays_natural_and_long_copy_scrolls_to_end(
    cx: &mut TestAppContext,
) {
    let (center, surface) = mount(cx);
    center.update(surface, |center, cx| {
        center.show_progress("short", "提示", "一行内容", cx)
    });
    tick(surface, 500);
    let short = surface
        .debug_bounds("notification-content-short")
        .expect("short text");
    center.update(surface, |center, cx| {
        center.show_progress("short", "提示", "第一行\n第二行\n第三行\n第四行", cx)
    });
    tick(surface, 500);
    let multiline = surface
        .debug_bounds("notification-content-short")
        .expect("multiline");
    assert!(
        multiline.size.height > short.size.height + px(40.0),
        "naturalheight {short:?}->{multiline:?}"
    );
    center.update(surface, |center, cx| {
        center.show_progress(
            "short",
            "长通知",
            format!("{}末行标记", "长文本内容\n".repeat(100)),
            cx,
        )
    });
    tick(surface, 500);
    let copy = surface
        .debug_bounds("notification-content-short")
        .expect("scroll");
    let body = surface
        .debug_bounds("notification-natural-text-body")
        .expect("naturalbody");
    assert!(
        body.size.height > copy.size.height + px(1000.0),
        "real longbody not flexcompressed"
    );
    for _ in 0..8 {
        surface.simulate_event(gpui_kit::ScrollWheelEvent {
            position: copy.center(),
            delta: gpui_kit::ScrollDelta::Pixels(gpui_kit::point(px(0.0), px(-1000.0))),
            modifiers: Default::default(),
            touch_phase: gpui_kit::TouchPhase::Moved,
        });
        draw(surface);
    }
    let body = surface
        .debug_bounds("notification-natural-text-body")
        .expect("scrolledbody");
    assert!(
        body.bottom() <= copy.bottom() + px(1.0),
        "long text scrolls to trueend {body:?} vs {copy:?}"
    );
}

#[gpui_kit::test]
fn notification_duration_patch_retains_unconfigured_upstream_five_second_default(
    cx: &mut TestAppContext,
) {
    use gpui_kit::component::notification::{Notification, NotificationList};
    struct RawOfficialHost {
        list: Entity<NotificationList>,
    }
    impl Render for RawOfficialHost {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div().size_full().child(self.list.clone())
        }
    }
    cx.update(gpui_kit::init);
    cx.executor().allow_parking();
    cx.update(|cx| cx.set_reduce_motion(true));
    let closed = Rc::new(std::cell::Cell::new(false));
    let callback = closed.clone();
    let (_, surface) = cx.add_window_view(move |window, cx| {
        let list = cx.new(|cx| NotificationList::new(window, cx));
        list.update(cx, |list, cx| {
            list.push(
                Notification::info("upstream default").on_close(move |_, _| callback.set(true)),
                window,
                cx,
            )
        });
        gpui_kit::base::Root::new(cx.new(|_| RawOfficialHost { list }), window, cx)
    });
    surface.simulate_resize(size(px(480.0), px(500.0)));
    draw(surface);
    tick(surface, 5300);
    assert!(
        !closed.get(),
        "unconfigured original5s plus official entry/exit"
    );
    tick(surface, 400);
    assert!(closed.get(), "default remains5s rather than product6s");
}

#[gpui_kit::test]
fn notification_sizing_window_local_long_copy_respects_actual_small_viewport(
    cx: &mut TestAppContext,
) {
    let (center, surface) = mount(cx);
    surface.simulate_resize(size(px(300.0), px(300.0)));
    for position in [
        NotificationPosition::TopRight,
        NotificationPosition::BottomRight,
    ] {
        center.update(surface, |center, cx| {
            center.set_position(position, cx);
            center.push(
                AppNotification::new(
                    "small-viewport",
                    NotificationLevel::Progress,
                    "长内容",
                    "多行测试\n".repeat(100),
                )
                .actions(vec![NotificationAction::new("查看结果", |_, _| {})]),
                cx,
            );
        });
        tick(surface, 500);
        let card = official_cards(surface)[0].bounds();
        let action = surface
            .debug_bounds("notification-action-small-viewport")
            .expect("reachable action");
        assert!(
            card.top() >= px(60.0) && card.bottom() <= px(288.0),
            "actual300pxviewport must bound content: {card:?}"
        );
        assert!(action.top() >= card.top() && action.bottom() <= card.bottom());
    }
}
