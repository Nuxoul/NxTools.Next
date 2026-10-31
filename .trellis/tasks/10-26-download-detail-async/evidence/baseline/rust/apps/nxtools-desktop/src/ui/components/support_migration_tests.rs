use super::*;
use gpui_kit::base::Root;
use gpui_kit::prelude::*;
use gpui_kit::{AppContext, Context, IntoElement, Render, TestAppContext, Window, div, px, size};

struct SharedStates;
impl Render for SharedStates {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .w_full()
            .flex()
            .flex_col()
            .child(state::empty_state(
                crate::theme::Theme::zed_dark(),
                "queue10-empty",
                crate::icons::MaterialIcon::Search,
                "无结果",
                "合成纯文本 **plain** <b>literal</b>",
            ))
            .child(state::progress_state(
                crate::theme::Theme::zed_dark(),
                "queue10-loading",
                "加载中",
                "不是百分比",
            ))
            .child(state::inline_error_state(
                crate::theme::Theme::zed_dark(),
                "queue10-failed",
                "加载失败",
                "合成错误，保留重试",
            ))
            .child(field::inline_error(
                crate::theme::Theme::zed_dark(),
                "queue10-inline",
                "合成校验失败",
            ))
    }
}
#[gpui_kit::test]
fn queue10_shared_states_mount_actual_empty_alert_and_loading_without_percent(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_kit::init);
    cx.update(crate::theme::apply_component_theme);
    let (_, surface) = cx.add_window_view(|window, cx| {
        let view = cx.new(|_| SharedStates);
        Root::new(view, window, cx)
    });
    surface.simulate_resize(size(px(300.0), px(440.0)));
    surface.refresh().expect("production shared states");
    assert!(
        surface.debug_bounds("queue10-empty-kit-content").is_some(),
        "actual Empty content slot"
    );
    assert!(
        surface.debug_bounds("queue10-failed-kit-alert").is_some(),
        "actual Alert owner"
    );
    assert!(
        surface.debug_bounds("queue10-inline-kit-alert").is_some(),
        "actual field Alert"
    );
}
#[gpui_kit::test]
fn queue10_direct_notification_dismiss_does_not_complete_original_manager_callback(
    cx: &mut TestAppContext,
) {
    use std::{cell::Cell, rc::Rc};
    cx.update(gpui_kit::init);
    let closed = Rc::new(Cell::new(0));
    let held = closed.clone();
    let slot = Rc::new(std::cell::RefCell::new(None));
    let retained = slot.clone();
    let (_, surface) = cx.add_window_view(move |window, cx| {
        let view = cx.new(|_| {
            gpui_kit::component::notification::Notification::info("synthetic direct")
                .autohide(false)
                .on_close(move |_, _| held.set(held.get() + 1))
        });
        *retained.borrow_mut() = Some(view.clone());
        Root::new(view, window, cx)
    });
    surface.refresh().expect("direct official Notification");
    surface.update(|window, cx| {
        slot.borrow()
            .as_ref()
            .expect("Notification")
            .update(cx, |view, cx| view.dismiss(window, cx))
    });
    surface.run_until_parked();
    assert_eq!(
        closed.get(),
        0,
        "direct dismiss emits private request; public on_close needs library completion owner"
    );
}

#[gpui_kit::test]
fn queue10_production_notification_uses_styled_action_with_original_bounded_manager(
    cx: &mut TestAppContext,
) {
    use crate::main_window::{MainWindow, MainWindowShellContext};
    use crate::notifications::{NotificationAction, NotificationCenter};
    use std::{
        cell::{Cell, RefCell},
        rc::Rc,
    };
    cx.update(gpui_kit::init);
    cx.executor().allow_parking();
    cx.update(|cx| cx.set_reduce_motion(true));
    cx.update(crate::theme::apply_component_theme);
    let center = cx.update(|cx| cx.new(|_| NotificationCenter::default()));
    let calls = Rc::new(Cell::new(0));
    let held = calls.clone();
    center.update(cx, |center, cx| {
        center.show_success_with_details(
            "queue10",
            "合成通知",
            "原动作与文案保留",
            vec![],
            Some(NotificationAction::new("原操作", move |_, _| {
                held.set(held.get() + 1)
            })),
            cx,
        )
    });
    let rendered = center.clone();
    let (_, surface) = cx.add_window_view(move |window, cx| {
        MainWindow::new(
            gpui_kit::WeakEntity::new_invalid(),
            gpui_kit::WeakEntity::new_invalid(),
            gpui_kit::WeakEntity::new_invalid(),
            MainWindowShellContext {
                settings: None,
                platform: None,
                system_settings: Rc::new(RefCell::new(nxtools_core::SystemSettings::default())),
            },
            rendered,
            window,
            cx,
        )
    });
    surface.simulate_resize(size(px(920.0), px(480.0)));
    surface.refresh().expect("actual notification");
    let card = surface.debug_bounds("notification-queue10").expect("card");
    let action = surface
        .debug_bounds("notification-action-queue10")
        .expect("action");
    let official_card = surface.update(|window, _| {
        gpui_kit::base::test_support::snapshots(window)
            .into_iter()
            .find(|node| {
                node.role() == Some(gpui_kit::Role::Alert)
                    && node.path().last() == Some(&"notification".into())
            })
            .expect("official notification")
            .bounds()
    });
    assert_eq!(official_card.size.width, px(336.0));
    assert!(card.left() >= official_card.left() && card.right() <= official_card.right());
    assert!(action.bottom() <= official_card.bottom());
    assert_eq!(
        action.size.height,
        px(24.0),
        "released Small action, no28px reconstruction"
    );
    surface.simulate_click(action.center(), Default::default());
    surface.run_until_parked();
    assert_eq!(calls.get(), 1);
}

struct InteractiveState {
    calls: std::rc::Rc<std::cell::Cell<usize>>,
}
impl Render for InteractiveState {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let calls = self.calls.clone();
        state::interactive_state(
            "queue10-interactive",
            state::empty_state(
                crate::theme::Theme::zed_dark(),
                "queue10-action-empty",
                crate::icons::MaterialIcon::Download,
                "创建任务",
                "保留原动作",
            ),
            move |_, _, _| calls.set(calls.get() + 1),
        )
    }
}
#[gpui_kit::test]
fn queue10_interactive_empty_pointer_tab_release_preserves_one_original_action(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_kit::init);
    cx.update(crate::theme::apply_component_theme);
    let calls = std::rc::Rc::new(std::cell::Cell::new(0));
    let held = calls.clone();
    let (_, surface) = cx.add_window_view(move |window, cx| {
        let view = cx.new(|_| InteractiveState { calls: held });
        Root::new(view, window, cx)
    });
    surface.simulate_resize(size(px(300.0), px(180.0)));
    surface.refresh().expect("actual interactive state");
    let bounds = surface
        .debug_bounds("queue10-interactive")
        .expect("original hit owner");
    surface.simulate_click(bounds.center(), Default::default());
    assert_eq!(calls.get(), 1);
    surface.update(|window, cx| {
        window.activate_window();
        window.focus_next(cx);
    });
    surface.refresh().expect("actual Tab");
    assert!(
        surface.update(|window, _| gpui_kit::base::test_support::snapshots(window)
            .iter()
            .any(|node| node.path().last()
                == Some(&gpui_kit::ElementId::from("queue10-interactive"))
                && node.focused() == Some(true))),
        "actual focus before release"
    );
    surface.simulate_event(gpui_kit::KeyDownEvent {
        keystroke: gpui_kit::Keystroke::parse("space").expect("Space"),
        is_held: false,
        prefer_character_input: false,
    });
    assert_eq!(calls.get(), 1);
    surface.simulate_event(gpui_kit::KeyUpEvent {
        keystroke: gpui_kit::Keystroke::parse("space").expect("release"),
    });
    assert_eq!(calls.get(), 2);
}
struct LongStates;
impl Render for LongStates {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("queue10-scroll")
            .w_full()
            .h_full()
            .overflow_y_scroll()
            .child(state::inline_error_state(
                crate::theme::Theme::zed_dark(),
                "queue10-long",
                "合成长错误",
                "中文纯文本 **literal** <b>text</b> 保留完整失败说明。".repeat(30),
            ))
    }
}
#[gpui_kit::test]
fn queue10_long_error_wraps_inside_minimum_scroll_without_losing_copy(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(crate::theme::apply_component_theme);
    let (_, surface) = cx.add_window_view(|window, cx| {
        let view = cx.new(|_| LongStates);
        Root::new(view, window, cx)
    });
    surface.simulate_resize(size(px(240.0), px(180.0)));
    surface.refresh().expect("long real Alert");
    let bounds = surface
        .debug_bounds("queue10-long-kit-alert")
        .expect("content");
    assert!(bounds.left() >= px(0.0) && bounds.right() <= px(240.0));
    assert!(
        bounds.size.height > px(72.0),
        "full copy wraps, never clipped fixed error height"
    );
}

struct LoadingOnly;
impl Render for LoadingOnly {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        state::progress_state(
            crate::theme::Theme::zed_dark(),
            "queue10-motion-loading",
            "合成加载",
            "不是百分比",
        )
    }
}
#[gpui_kit::test]
fn queue10_loading_unknown_reduced_policy_does_not_request_animation_frames(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_kit::init);
    let (_, surface) = cx.add_window_view(|_, _| LoadingOnly);
    surface.simulate_resize(size(px(300.0), px(100.0)));
    surface.refresh().expect("unknown static loading");
    surface.update(|window, cx| assert_eq!(window.simulate_next_frame(cx), 0));
    surface.update(|_, cx| {
        cx.set_global(crate::ui::motion::UiMotionPreference::Known {
            reduced_motion: true,
        })
    });
    surface.refresh().expect("reduced loading");
    surface.update(|window, cx| assert_eq!(window.simulate_next_frame(cx), 0));
}

struct NativeTitleContent;
impl Render for NativeTitleContent {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        titlebar::client_titlebar(
            crate::theme::Theme::zed_dark(),
            "queue10-title",
            "queue10-close",
            "合成独立窗口",
            |_, _, _| {},
        )
    }
}
#[gpui_kit::test]
fn queue10_native_titlebar_keeps_original_drag_host_with_styled_small_close(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_kit::init);
    let (_, surface) = cx.add_window_view(|_, _| NativeTitleContent);
    surface.simulate_resize(size(px(380.0), px(178.0)));
    surface.refresh().expect("actual native title content");
    let title = surface
        .debug_bounds("queue10-title")
        .expect("native drag host");
    let close = surface
        .debug_bounds("queue10-close")
        .expect("actual Button");
    assert_eq!(title.size.height, px(36.0));
    assert_eq!(close.size.height, px(24.0));
    assert!(
        close.left() >= title.left()
            && close.right() <= title.right()
            && close.bottom() <= title.bottom()
    );
}

#[gpui_kit::test]
fn queue10_notification_original_timers_preserve_same_id_replacement_and_progress(
    cx: &mut TestAppContext,
) {
    let (center, surface) = crate::notifications::presentation_tests::mount(cx);
    center.update(surface, |center, cx| {
        center.set_duration_seconds(5, cx);
        center.show_info("same", "默认5秒", "合成通知", cx);
        center.show_error("error", "默认5秒", "合成错误", cx);
        center.show_progress("progress", "加载", "无百分比", cx);
    });
    crate::notifications::presentation_tests::tick(surface, 4400);
    center.update(surface, |center, cx| {
        center.show_info("same", "替换5秒", "新代次", cx)
    });
    crate::notifications::presentation_tests::tick(surface, 1500);
    assert_eq!(
        center.read_with(surface, |center, _| center.items().len()),
        2,
        "old official close cannot dismiss replacement"
    );
    crate::notifications::presentation_tests::tick(surface, 4200);
    assert_eq!(
        center.read_with(surface, |center, _| center.items().len()),
        1
    );
    assert_eq!(
        center.read_with(surface, |center, _| center.items()[0].id.clone()),
        "progress"
    );
}
