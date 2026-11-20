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

#[gpui_kit::test]
fn window_feedback_about_caption_has_only_one_close_action(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (_, surface) = cx.add_window_view(|window, cx| {
        let view = cx.new(|cx| About::new(Uuid::new_v4(), cx));
        Root::new(view, window, cx)
    });
    surface.simulate_resize(size(px(420.), px(480.)));
    surface.refresh().expect("About caption frame");
    surface.update(|window, _| {
        let nodes = gpui_kit::base::test_support::snapshots(window);
        assert!(
            !nodes.iter().any(|node| [
                "minimize",
                "maximize",
                "restore",
                "window-minimize",
                "window-maximize"
            ]
            .iter()
            .any(|id| node.path().last() == Some(&gpui_kit::ElementId::from(*id)))),
            "About must not offer Minimize or Maximize"
        );
        assert_eq!(
            nodes
                .iter()
                .filter(|node| node.label() == Some("关闭"))
                .count(),
            1,
            "one named original-owner Close action"
        );
    });
}

#[gpui_kit::test]
fn window_feedback_about_close_retires_update_and_removes_window_once(cx: &mut TestAppContext) {
    use std::{
        cell::{Cell, RefCell},
        rc::Rc,
    };
    cx.update(gpui_kit::init);
    let slot = Rc::new(RefCell::new(None));
    let captured = slot.clone();
    let (_, surface) = cx.add_window_view(move |window, cx| {
        let view = cx.new(|cx| {
            let mut view = About::new(Uuid::new_v4(), cx);
            view.update_request = Some(Uuid::new_v4());
            view.update = UpdateStatus::Checking;
            view
        });
        About::observe_close(&view, window, cx);
        *captured.borrow_mut() = Some(view.clone());
        Root::new(view, window, cx)
    });
    let view = slot.borrow().clone().expect("original About entity");
    let handle = surface.update(|window, _| window.window_handle());
    let closes = Rc::new(Cell::new(0));
    let seen = closes.clone();
    let _closed = surface.update(|_, cx| {
        cx.on_window_closed(move |_, id| {
            if id == handle.window_id() {
                seen.set(seen.get() + 1);
            }
        })
    });
    surface.simulate_resize(size(px(420.), px(480.)));
    surface.refresh().expect("About with pending update");
    let close = surface.update(|window, _| {
        gpui_kit::base::test_support::snapshots(window)
            .iter()
            .find(|node| {
                node.label() == Some("关闭")
                    || node.path().last() == Some(&gpui_kit::ElementId::from("close"))
            })
            .expect("visible Close caption control")
            .bounds()
    });
    surface.simulate_click(close.center(), Modifiers::default());
    surface.run_until_parked();
    assert!(
        !surface.read(|cx| cx.windows().contains(&handle)),
        "caption click must close the actual About window"
    );
    assert_eq!(closes.get(), 1, "only one close lifecycle dispatch");
    assert!(
        surface.read(|cx| view.read(cx).closed && view.read(cx).update_request.is_none()),
        "close retires update work before releasing the source window"
    );
}

#[gpui_kit::test]
fn window_feedback_about_creation_keeps_main_window_outside_modal_lifecycle(
    cx: &mut TestAppContext,
) {
    let options = cx.update(|cx| about_window_options(cx));
    assert!(
        matches!(options.kind, WindowKind::Normal),
        "About must not disable/re-enable its main window through the framework Dialog path"
    );
    assert!(!options.is_minimizable && !options.is_resizable);
    assert_eq!(options.window_min_size, Some(size(px(420.), px(480.))));
}
