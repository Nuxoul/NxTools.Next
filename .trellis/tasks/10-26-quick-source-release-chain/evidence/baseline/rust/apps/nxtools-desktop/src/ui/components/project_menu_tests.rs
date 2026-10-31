use super::*;
use gpui_kit::test::{TestSupportExt, TestWindowExt};
use gpui_kit::{TestAppContext, VisualTestContext};
use std::cell::Cell;

struct Fixture {
    previous: FocusHandle,
    shown: bool,
    calls: Rc<Cell<usize>>,
    underlying: Rc<Cell<usize>>,
    long: bool,
}
impl Render for Fixture {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let calls = self.calls.clone();
        let underlying = self.underlying.clone();
        let mut children = vec![
            MenuItem::new("child-disabled", "禁用分类").disabled(true),
            MenuItem::separator(),
        ];
        for ix in 0..if self.long { 40 } else { 2 } {
            let calls = calls.clone();
            children.push(
                MenuItem::new(format!("child-{ix}"), format!("分类 {ix}"))
                    .checked(ix == 0)
                    .on_click(move |_, window, cx| {
                        let menu = Root::read(window, cx)
                            .plugin::<ProjectMenu>()
                            .expect("real menu owner");
                        assert!(
                            menu.read(cx).session.is_none(),
                            "callback must run after closing/releasing menu"
                        );
                        calls.set(calls.get() + 1);
                    }),
            );
        }
        let action_calls = calls.clone();
        div()
            .size_full()
            .on_mouse_down(MouseButton::Left, move |_, _, _| {
                underlying.set(underlying.get() + 1)
            })
            .child(
                div()
                    .id("previous")
                    .track_focus(&self.previous)
                    .w(px(20.0))
                    .h(px(20.0)),
            )
            .when(self.shown, |content| {
                content.child(context_menu(
                    "test-menu-host",
                    div()
                        .id("test-trigger")
                        .test_support()
                        .w(px(50.0))
                        .h(px(28.0))
                        .child("菜单")
                        .into_any_element(),
                    vec![
                        MenuItem::new("disabled-root", "禁用").disabled(true),
                        MenuItem::separator(),
                        MenuItem::new("root-action", "操作")
                            .icon(MaterialIcon::ContentCopy)
                            .on_click(move |_, _, _| action_calls.set(action_calls.get() + 1)),
                        MenuItem::new("root-category", "分类").submenu(children),
                    ],
                ))
            })
    }
}
type MenuFixture = (
    gpui_kit::Entity<Fixture>,
    VisualTestContext,
    Rc<Cell<usize>>,
    Rc<Cell<usize>>,
);
fn setup(cx: &mut TestAppContext, width: f32, height: f32, long: bool) -> MenuFixture {
    cx.update(gpui_kit::init);
    cx.update(crate::theme::apply_component_theme);
    let calls = Rc::new(Cell::new(0));
    let underlying = Rc::new(Cell::new(0));
    let fixture = cx.new({
        let calls = calls.clone();
        let underlying = underlying.clone();
        move |cx| Fixture {
            previous: cx.focus_handle(),
            shown: true,
            calls,
            underlying,
            long,
        }
    });
    let content = fixture.clone();
    let (_, surface) = cx.add_window_view(move |window, cx| {
        window.resize(size(px(width), px(height)));
        Root::new(content, window, cx)
    });
    let handle = surface.update(|window, _| window.window_handle());
    let mut surface = VisualTestContext::from_window(handle, cx);
    surface.simulate_resize(size(px(width), px(height)));
    surface.update(|window, cx| fixture.read(cx).previous.clone().focus(window, cx));
    (fixture, surface, calls, underlying)
}
fn open(surface: &mut VisualTestContext) {
    surface.update(|window, cx| window.right_click("test-trigger", cx));
    surface.refresh().expect("real menu frame");
}
fn press(surface: &mut VisualTestContext, key: &str) {
    surface.update(|window, cx| window.press(key, cx));
    surface.refresh().expect("keyboard frame");
}
fn owner(surface: &mut VisualTestContext) -> gpui_kit::Entity<ProjectMenu> {
    surface.update(|window, cx| {
        Root::read(window, cx)
            .plugin::<ProjectMenu>()
            .expect("menu plugin")
    })
}
#[gpui_kit::test]
fn keyboard_skips_disabled_arms_one_release_and_restores_focus(cx: &mut TestAppContext) {
    let (fixture, mut surface, calls, _) = setup(cx, 300.0, 440.0, false);
    open(&mut surface);
    let menu = owner(&mut surface);
    assert_eq!(menu.read_with(&surface, |menu, _| menu.selected), Some(2));
    press(&mut surface, "end");
    press(&mut surface, "right");
    assert_eq!(
        menu.read_with(&surface, |menu, _| menu.child_selected),
        Some(2)
    );
    surface.update(|window, _| {
        assert_eq!(window.find("child-0").checked(), Some(true));
        assert_eq!(window.find("child-1").checked(), Some(false));
    });
    press(&mut surface, "left");
    assert!(!menu.read_with(&surface, |menu, _| menu.child_open));
    press(&mut surface, "right");
    surface.simulate_keystrokes("enter");
    surface.simulate_event(KeyDownEvent {
        keystroke: gpui_kit::Keystroke::parse("enter").expect("repeat"),
        is_held: true,
        prefer_character_input: false,
    });
    assert_eq!(calls.get(), 0);
    surface.simulate_event(KeyUpEvent {
        keystroke: gpui_kit::Keystroke::parse("enter").expect("release"),
    });
    assert_eq!(calls.get(), 1);
    surface.simulate_event(KeyUpEvent {
        keystroke: gpui_kit::Keystroke::parse("enter").expect("stale release"),
    });
    assert_eq!(calls.get(), 1);
    assert!(surface.update(|window, cx| fixture.read(cx).previous.is_focused(window)));
    open(&mut surface);
    press(&mut surface, "ctrl-enter");
    assert_eq!(calls.get(), 1);
    press(&mut surface, "tab");
    assert!(menu.read_with(&surface, |menu, _| menu.session.is_none()));
    open(&mut surface);
    press(&mut surface, "space");
    assert_eq!(calls.get(), 2);
}
#[gpui_kit::test]
fn outside_dismiss_reopen_hide_and_trigger_retirement_never_dispatch(cx: &mut TestAppContext) {
    let (fixture, mut surface, calls, underlying) = setup(cx, 300.0, 440.0, false);
    open(&mut surface);
    let menu = owner(&mut surface);
    let first = menu.read_with(&surface, |menu, _| menu.session.as_ref().expect("open").id);
    surface.simulate_click(point(px(299.0), px(439.0)), Default::default());
    assert!(menu.read_with(&surface, |menu, _| menu.session.is_none()));
    assert_eq!(
        underlying.get(),
        0,
        "outside dismiss must not activate underlying row"
    );
    open(&mut surface);
    assert_ne!(
        menu.read_with(&surface, |menu, _| menu
            .session
            .as_ref()
            .expect("reopen")
            .id),
        first
    );
    press(&mut surface, "escape");
    assert!(menu.read_with(&surface, |menu, _| menu.session.is_none()));
    open(&mut surface);
    surface.update(dismiss_window);
    surface.refresh().expect("hidden host cleanup");
    assert!(menu.read_with(&surface, |menu, _| menu.session.is_none()));
    open(&mut surface);
    fixture.update(&mut surface, |fixture, cx| {
        fixture.shown = false;
        cx.notify();
    });
    surface.refresh().expect("trigger removed");
    surface.refresh().expect("retired frame");
    assert!(menu.read_with(&surface, |menu, _| menu.session.is_none()));
    assert!(
        surface.update(|window, cx| fixture.read(cx).previous.is_focused(window)),
        "retiring trigger restores prior focus rather than orphaning menu focus"
    );
    assert_eq!(calls.get(), 0);
}
#[gpui_kit::test]
fn short_narrow_viewport_scroll_keeps_last_category_reachable(cx: &mut TestAppContext) {
    let (_, mut surface, calls, _) = setup(cx, 220.0, 140.0, true);
    open(&mut surface);
    press(&mut surface, "end");
    press(&mut surface, "right");
    press(&mut surface, "end");
    let bounds = surface
        .debug_bounds("project-menu-child")
        .expect("actual scroll panel");
    assert!(
        bounds.left() >= px(4.0)
            && bounds.right() <= px(216.0)
            && bounds.top() >= px(4.0)
            && bounds.bottom() <= px(136.0),
    );
    let parent = surface
        .debug_bounds("project-menu-root")
        .expect("parent panel");
    assert!(
        bounds.left() - parent.left() >= px(24.0) || parent.right() - bounds.right() >= px(24.0),
        "narrow submenu must leave a reachable parent edge: parent={parent:?} child={bounds:?}"
    );
    let menu = owner(&mut surface);
    assert!(
        menu.read_with(&surface, |menu, _| menu.child_scroll.offset().y < px(0.0)),
        "real scroll must reveal last item"
    );
    press(&mut surface, "enter");
    assert_eq!(calls.get(), 1);
}

#[gpui_kit::test]
fn menu_label_and_arrow_lanes_stay_aligned_across_icon_disabled_and_submenu_rows(
    cx: &mut TestAppContext,
) {
    let (_, mut surface, calls, _) = setup(cx, 300.0, 440.0, false);
    open(&mut surface);
    let labels = [
        "disabled-root-label",
        "root-action-label",
        "root-category-label",
    ]
    .map(|id| surface.debug_bounds(id).expect("real menu label lane"));
    assert!(
        labels
            .windows(2)
            .all(|pair| pair[0].left() == pair[1].left() && pair[0].right() == pair[1].right()),
        "label rails={labels:?}"
    );
    let before = surface
        .debug_bounds("root-action")
        .expect("real action hit bounds");
    let disabled = surface
        .debug_bounds("disabled-root")
        .expect("disabled hit bounds");
    surface.simulate_click(disabled.center(), Default::default());
    assert_eq!(calls.get(), 0);
    surface.simulate_mouse_move(before.center(), None, Default::default());
    surface.refresh().expect("hover actual row");
    assert_eq!(
        surface.debug_bounds("root-action"),
        Some(before),
        "hover must not shift geometry"
    );
}

#[gpui_kit::test]
fn focused_menu_closes_on_focus_transfer_and_shift_tab(cx: &mut TestAppContext) {
    let (fixture, mut surface, calls, _) = setup(cx, 600.0, 440.0, false);
    open(&mut surface);
    let menu = owner(&mut surface);
    surface.update(|window, cx| fixture.read(cx).previous.clone().focus(window, cx));
    surface.run_until_parked();
    surface.refresh().expect("focus transferred outside menu");
    assert!(menu.read_with(&surface, |menu, _| menu.session.is_none()));
    open(&mut surface);
    press(&mut surface, "shift-tab");
    assert!(menu.read_with(&surface, |menu, _| menu.session.is_none()));
    assert!(surface.update(|window, cx| fixture.read(cx).previous.is_focused(window)));
    assert_eq!(calls.get(), 0);
}

#[gpui_kit::test]
fn pointer_child_escape_and_repeated_right_keep_the_owning_level(cx: &mut TestAppContext) {
    let (_, mut surface, calls, _) = setup(cx, 600.0, 440.0, false);
    open(&mut surface);
    let menu = owner(&mut surface);
    let bounds = surface
        .debug_bounds("root-category")
        .expect("submenu trigger");
    surface.simulate_mouse_move(bounds.center(), None, Default::default());
    surface.refresh().expect("pointer-open child");
    assert!(menu.read_with(&surface, |menu, _| menu.child_open));
    press(&mut surface, "escape");
    assert!(
        menu.read_with(&surface, |menu, _| menu.session.is_some()),
        "child Escape must keep parent open"
    );
    assert!(!menu.read_with(&surface, |menu, _| menu.child_open));
    press(&mut surface, "right");
    press(&mut surface, "down");
    let selected = menu.read_with(&surface, |menu, _| menu.child_selected);
    press(&mut surface, "right");
    assert_eq!(
        menu.read_with(&surface, |menu, _| menu.child_selected),
        selected,
        "Right in leaf submenu must not reset selection"
    );
    assert_eq!(calls.get(), 0);
}

#[gpui_kit::test]
fn reopening_an_active_menu_restores_the_original_trigger_focus(cx: &mut TestAppContext) {
    let (fixture, mut surface, calls, _) = setup(cx, 600.0, 440.0, false);
    open(&mut surface);
    // The trigger is outside the menu on this wider viewport; a second right press
    // dismisses the predecessor and the next right press opens a fresh session.
    open(&mut surface);
    open(&mut surface);
    press(&mut surface, "escape");
    assert!(
        surface.update(|window, cx| fixture.read(cx).previous.is_focused(window)),
        "reopen must not save the predecessor menu focus as its return target"
    );
    assert_eq!(calls.get(), 0);
}

#[gpui_kit::test]
fn pointer_entering_child_routes_arrow_navigation_to_that_child(cx: &mut TestAppContext) {
    let (_, mut surface, calls, _) = setup(cx, 600.0, 440.0, false);
    open(&mut surface);
    let row = surface
        .debug_bounds("root-category")
        .expect("parent category");
    surface.simulate_mouse_move(row.center(), None, Default::default());
    surface.refresh().expect("child open");
    let row = surface.debug_bounds("child-0").expect("child category");
    surface.simulate_mouse_move(row.center(), None, Default::default());
    press(&mut surface, "down");
    let menu = owner(&mut surface);
    assert!(menu.read_with(&surface, |menu, _| menu.child_open));
    assert_eq!(
        menu.read_with(&surface, |menu, _| menu.child_selected),
        Some(3)
    );
    assert_eq!(calls.get(), 0);
}
