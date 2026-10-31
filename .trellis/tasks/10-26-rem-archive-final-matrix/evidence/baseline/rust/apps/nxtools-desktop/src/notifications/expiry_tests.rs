//! Expiry is tested through the real official list, never a parallel center clock.
use super::presentation_tests::{draw, mount, official_cards, tick};
use gpui_kit::{TestAppContext, point, px};
#[gpui_kit::test]
fn hover_pauses_group_remaining_five_second_duration_and_resume_does_not_reset(
    cx: &mut TestAppContext,
) {
    let (center, surface) = mount(cx);
    center.update(surface, |center, cx| center.set_duration_seconds(5, cx));
    for id in ["first", "second"] {
        center.update(surface, |center, cx| {
            center.show_info(id, "提示", "同组剩余倒计时", cx)
        });
    }
    tick(surface, 2400);
    let front = official_cards(surface)
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
    surface.simulate_mouse_move(front.center(), None, Default::default());
    tick(surface, 10000);
    assert_eq!(
        center.read_with(surface, |center, _| center.items().len()),
        2,
        "official group pauses both notices"
    );
    surface.simulate_mouse_move(point(px(2.0), px(2.0)), None, Default::default());
    draw(surface);
    tick(surface, 2600);
    assert_eq!(
        center.read_with(surface, |center, _| center.items().len()),
        2
    );
    tick(surface, 1000);
    assert!(
        center.read_with(surface, |center, _| center.items().is_empty()),
        "remaining3s+exit, not a new5s"
    );
}
#[gpui_kit::test]
fn stale_official_close_does_not_expire_replacement_and_progress_remains(cx: &mut TestAppContext) {
    let (center, surface) = mount(cx);
    center.update(surface, |center, cx| center.set_duration_seconds(5, cx));
    center.update(surface, |center, cx| {
        center.show_info("same", "旧", "旧代次", cx)
    });
    tick(surface, 2400);
    let old = center.read_with(surface, |center, _| center.items()[0].generation());
    center.update(surface, |center, cx| {
        center.show_info("same", "新", "新代次", cx);
        center.show_progress("progress", "进度", "常驻", cx);
        center.completed_close("same", old, cx);
    });
    tick(surface, 3000);
    assert_eq!(
        center.read_with(surface, |center, _| center.items().len()),
        2
    );
    tick(surface, 2800);
    assert_eq!(
        center.read_with(surface, |center, _| center.items().len()),
        1
    );
    assert_eq!(
        center.read_with(surface, |center, _| center.items()[0].id.clone()),
        "progress"
    );
}

#[gpui_kit::test]
fn changing_position_near_expiry_preserves_official_remaining_time(cx: &mut TestAppContext) {
    let (center, surface) = mount(cx);
    center.update(surface, |center, cx| center.set_duration_seconds(5, cx));
    center.update(surface, |center, cx| {
        center.show_info("position", "提示", "保留剩余时间", cx)
    });
    tick(surface, 4400);
    assert_eq!(
        center.read_with(surface, |center, _| center.items().len()),
        1
    );
    center.update(surface, |center, cx| {
        center.set_position(super::NotificationPosition::TopLeft, cx)
    });
    draw(surface);
    tick(surface, 1400);
    assert!(
        center.read_with(surface, |center, _| center.items().is_empty()),
        "position-only change must not restart the official five-second countdown"
    );
}

#[gpui_kit::test]
fn notification_duration_one_six_sixty_seconds_use_official_clock(cx: &mut TestAppContext) {
    let (center, surface) = mount(cx);
    assert_eq!(
        center.read_with(surface, |center, _| center.duration_seconds()),
        6
    );
    for seconds in [1, 6, 60] {
        center.update(surface, |center, cx| {
            center.set_duration_seconds(seconds, cx);
            center.show_info("duration", "时长测试", "官方倒计时", cx);
        });
        tick(surface, seconds as u64 * 1000 + 300);
        assert_eq!(
            center.read_with(surface, |center, _| center.items().len()),
            1,
            "before official dwell+exit: {seconds}"
        );
        tick(surface, 500);
        assert!(
            center.read_with(surface, |center, _| center.items().is_empty()),
            "configured duration expires: {seconds}"
        );
    }
}

#[gpui_kit::test]
fn notification_duration_setting_change_does_not_reset_or_replace_mounted_notice(
    cx: &mut TestAppContext,
) {
    let (center, surface) = mount(cx);
    center.update(surface, |center, cx| {
        center.set_duration_seconds(1, cx);
        center.show_info("old", "旧通知", "保留一秒", cx);
    });
    tick(surface, 800);
    center.update(surface, |center, cx| center.set_duration_seconds(60, cx));
    tick(surface, 1000);
    assert!(
        center.read_with(surface, |center, _| center.items().is_empty()),
        "settings change cannot extend active1snotice"
    );
    center.update(surface, |center, cx| {
        center.show_info("new", "新通知", "六十秒", cx);
        center.show_progress("persistent", "进度", "仍然常驻", cx);
    });
    tick(surface, 6500);
    assert_eq!(
        center.read_with(surface, |center, _| center.items().len()),
        2,
        "new snapshot uses60s,progress remains persistent"
    );
}

#[gpui_kit::test]
fn notification_duration_six_seconds_pauses_remaining_time_for_whole_group(
    cx: &mut TestAppContext,
) {
    let (center, surface) = mount(cx);
    for id in ["six-a", "six-b"] {
        center.update(surface, |center, cx| {
            center.show_info(id, "提示", "默认六秒", cx)
        });
    }
    tick(surface, 2400);
    let front = official_cards(surface)
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
    surface.simulate_mouse_move(front.center(), None, Default::default());
    tick(surface, 10000);
    assert_eq!(
        center.read_with(surface, |center, _| center.items().len()),
        2
    );
    surface.simulate_mouse_move(point(px(2.0), px(2.0)), None, Default::default());
    draw(surface);
    tick(surface, 3600);
    assert_eq!(
        center.read_with(surface, |center, _| center.items().len()),
        2
    );
    tick(surface, 900);
    assert!(
        center.read_with(surface, |center, _| center.items().is_empty()),
        "remaining4s+exit,not fresh6s"
    );
}
