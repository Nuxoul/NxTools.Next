use super::*;
use gpui_kit::{Bounds, Modifiers, Pixels, TestAppContext, VisualTestContext};

fn button(window: &mut VisualTestContext, label: &str) -> Bounds<Pixels> {
    window.update(|window, _| {
        gpui_kit::base::test_support::snapshots(window)
            .iter()
            .find(|snapshot| snapshot.label() == Some(label))
            .expect("named About button")
            .bounds()
    })
}

fn click(window: &mut VisualTestContext, bounds: Bounds<Pixels>) {
    window.simulate_click(bounds.center(), Modifiers::default());
    window.run_until_parked();
    window.refresh().expect("render result of click");
}

fn copied_feedback(window: &mut VisualTestContext) -> bool {
    window.update(|window, _| {
        gpui_kit::base::test_support::snapshots(window)
            .iter()
            .any(|snapshot| {
                snapshot
                    .label()
                    .is_some_and(|label| label.contains("已复制 ✓"))
            })
    })
}

#[gpui_kit::test]
fn about_copy_and_license_navigation_at_fixed_size(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(|cx| cx.set_reduce_motion(true));
    let time = cx.background_executor.clone();
    let (_, window) = cx.add_window_view(|window, cx| {
        let view = cx.new(|cx| About::new(Uuid::nil(), cx));
        Root::new(view, window, cx)
    });
    window.simulate_resize(size(px(420.), px(480.)));
    window.refresh().expect("render fixed size");
    let version = window.debug_bounds("about-version").expect("version pill");
    let specifications = window
        .debug_bounds("about-specifications")
        .expect("technical card");
    let actions = window
        .debug_bounds("about-actions")
        .expect("bottom actions");
    assert!(version.bottom() < specifications.top() && specifications.bottom() < actions.top());
    assert!(actions.bottom() < px(448.));

    click(window, version);
    assert!(copied_feedback(window));
    let clipboard = window.update(|_, cx| cx.read_from_clipboard().and_then(|item| item.text()));
    let clipboard = clipboard.expect("copy writes diagnostic text");
    assert!(clipboard.contains(&format!("NxTools v{VERSION}")));
    assert!(clipboard.contains(TARGET));
    assert!(clipboard.contains(COMPILER));
    time.advance_clock(Duration::from_secs(2));
    window.run_until_parked();
    window.refresh().expect("copy feedback expires");
    assert!(!copied_feedback(window));

    let licenses = button(window, "开源许可");
    click(window, licenses);
    assert!(window.debug_bounds("about-license-list").is_some());
    assert!(window.debug_bounds("about-version").is_none());
    let back = button(window, "返回");
    click(window, back);
    assert!(window.debug_bounds("about-version").is_some());
}

#[gpui_kit::test]
fn about_short_client_keeps_the_complete_footer_visible(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (_, window) = cx.add_window_view(|window, cx| {
        let view = cx.new(|cx| {
            let mut view = About::new(Uuid::nil(), cx);
            view.update = UpdateStatus::Unavailable;
            view
        });
        Root::new(view, window, cx)
    });
    for height in [442., 480.] {
        window.simulate_resize(size(px(406.), px(height)));
        window.refresh().expect("render short Windows client");
        let card = window
            .debug_bounds("about-specifications")
            .expect("technical card");
        let actions = window.debug_bounds("about-actions").expect("action row");
        assert!(card.bottom() < actions.top());
        assert!(
            actions.bottom() + px(34.) <= px(height - 12.),
            "footer must fit after the controls: {actions:?}"
        );
        let status = window
            .debug_bounds("about-update-status")
            .expect("update feedback");
        let author = window
            .debug_bounds("about-author")
            .expect("visible author credit");
        let copyright = window
            .debug_bounds("about-copyright")
            .expect("visible copyright");
        assert!(actions.bottom() <= status.top() && status.bottom() <= author.top());
        assert!(author.bottom() <= px(height - 12.) && copyright.bottom() <= px(height - 12.));
    }
}

#[gpui_kit::test]
fn about_emblem_tracks_pointer_and_returns_to_rest(cx: &mut TestAppContext) {
    use crate::ui::motion::UiMotionPreference;
    use gpui_kit::point;

    cx.update(gpui_kit::init);
    cx.update(|cx| {
        cx.set_global(UiMotionPreference::Known {
            reduced_motion: false,
        })
    });
    let time = cx.background_executor.clone();
    let (_, window) = cx.add_window_view(|window, cx| {
        let view = cx.new(|cx| About::new(Uuid::nil(), cx));
        Root::new(view, window, cx)
    });
    window.simulate_resize(size(px(406.), px(442.)));
    window.refresh().expect("resting emblem");
    let logo = window
        .debug_bounds("about-emblem")
        .expect("fixed emblem hit area");
    let rest = window
        .debug_bounds("about-emblem-face")
        .expect("resting face")
        .center();
    let face = |window: &mut VisualTestContext| {
        window
            .debug_bounds("about-emblem-face")
            .expect("emblem face")
            .center()
    };
    let frame = |window: &mut VisualTestContext, ms| {
        time.advance_clock(Duration::from_millis(ms));
        window.update(|window, cx| window.simulate_next_frame(cx));
        window.refresh().expect("emblem animation frame");
    };
    let edge = logo.size.height / 2. - px(2.);
    let right = logo.center() + point(px(32.), -edge);
    let left = logo.center() + point(px(-32.), edge);
    for (cursor, sign) in [(right, 1.), (left, -1.)] {
        let previous = face(window);
        window.simulate_mouse_move(cursor, None, Modifiers::default());
        window.run_until_parked();
        window.refresh().expect("retarget emblem");
        assert_eq!(
            face(window),
            previous,
            "pointer input must not snap the first frame"
        );
        frame(window, 50);
        let moving = face(window);
        assert!((moving.x - previous.x) * sign > px(0.));
        frame(window, 220);
        let settled = face(window);
        assert!((settled.x - moving.x) * sign > px(0.));
        assert!((settled.x - rest.x) * sign > px(4.));
        let plate = window
            .debug_bounds("about-emblem-plate")
            .expect("moving plate");
        assert!((settled.x - plate.center().x) * sign > px(1.));
        assert!(
            plate.top() >= logo.top() && plate.bottom() <= logo.bottom(),
            "entire backplate must fit at the hover edges"
        );
        assert_eq!(window.debug_bounds("about-emblem"), Some(logo));
    }
    let previous = face(window);
    window.simulate_mouse_move(point(px(32.), px(240.)), None, Modifiers::default());
    window.run_until_parked();
    window.refresh().expect("start short return animation");
    assert_eq!(
        face(window),
        previous,
        "leaving must not snap the first frame"
    );
    frame(window, 65);
    assert!(face(window).x > previous.x && face(window).x < rest.x);
    frame(window, 230);
    assert_eq!(face(window), rest);

    // Interrupt a return before it settles: re-entry starts at the painted pose.
    window.simulate_mouse_move(right, None, Modifiers::default());
    frame(window, 220);
    let before_return = face(window);
    window.simulate_mouse_move(point(px(32.), px(240.)), None, Modifiers::default());
    frame(window, 70);
    let returning = face(window);
    window.simulate_mouse_move(left, None, Modifiers::default());
    window.refresh().expect("retarget during return");
    assert_eq!(face(window), returning);
    assert!(returning.x > rest.x && returning.x < before_return.x);
    frame(window, 220);
    assert!(face(window).x < rest.x);

    window.update(|_, cx| {
        cx.set_global(UiMotionPreference::Known {
            reduced_motion: true,
        })
    });
    window.simulate_mouse_move(right, None, Modifiers::default());
    window.run_until_parked();
    window.refresh().expect("reduced motion emblem");
    assert_eq!(
        window
            .debug_bounds("about-emblem-face")
            .expect("static face")
            .center(),
        rest
    );
}

#[test]
fn about_updates_compare_versions_numerically_and_reject_unknown_tags() {
    assert!(matches!(
        compare_release("0.7.9", "v0.7.10"),
        Some(UpdateStatus::Newer)
    ));
    assert!(matches!(
        compare_release("0.7.9", "v0.7.9"),
        Some(UpdateStatus::Current)
    ));
    assert!(matches!(
        compare_release("0.7.9", "v0.7.8"),
        Some(UpdateStatus::Current)
    ));
    assert!(compare_release("0.7.9", "nightly").is_none());
}
