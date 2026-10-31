use super::*;
use gpui_kit::TestAppContext;
use std::{cell::Cell, rc::Rc};

#[gpui_kit::test]
fn official_menu_renders_outside_source_budget(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(crate::theme::apply_component_theme);
    let id = Uuid::new_v4();
    let items = vec![
        MenuItem::new("copy", "仅复制到剪贴板").icon(crate::icons::MaterialIcon::ContentCopy),
        MenuItem::separator(),
        MenuItem::new("categories", "来源分类").submenu(vec![
            MenuItem::new("checked", "已关联分类").checked(true),
            MenuItem::new("unchecked", "未关联分类").checked(false),
        ]),
        MenuItem::new("delete", "删除到历史回收").tone(MenuTone::Danger),
    ];
    let (_, surface) = cx.add_window_view(move |window, cx| {
        let content = cx.new(|cx| MenuSurface::new(id, items, window, cx));
        Root::new(content, window, cx).bg(gpui_kit::transparent_black())
    });
    surface.simulate_resize(size(px(1200.0), px(800.0)));
    surface.update(|window, cx| {
        let root = window.root::<Root>().flatten().expect("popup Root");
        let content = root
            .read(cx)
            .view()
            .clone()
            .downcast::<MenuSurface>()
            .expect("real menu surface");
        content.update(cx, |view, cx| {
            view.place(point(px(299.0), px(30.0)), window, cx)
        });
    });
    surface.update(|window, cx| window.draw(cx).clear(cx));
    let bounds = surface
        .debug_bounds("styled-menu-content")
        .expect("official PopupMenu, not project-painted rows");
    assert!(
        bounds.right() > px(300.0),
        "not clipped to source HWND: {bounds:?}"
    );
    let menu = surface.update(|window, cx| {
        let root = window.root::<Root>().flatten().expect("Root");
        let content = root
            .read(cx)
            .view()
            .clone()
            .downcast::<MenuSurface>()
            .expect("surface");
        content.read(cx).menu.clone()
    });
    for preference in [
        nxtools_core::ThemePreference::Light,
        nxtools_core::ThemePreference::Dark,
    ] {
        surface.update(|window, cx| {
            crate::theme::set_theme_preference(preference, cx);
            menu.focus_handle(cx).focus(window, cx);
        });
        surface.run_until_parked();
        surface.update(|window, cx| window.draw(cx).clear(cx));
        let copy = surface
            .debug_bounds("copy")
            .expect("real styled copy row content");
        let delete = surface
            .debug_bounds("delete")
            .expect("real styled destructive content");
        assert!(copy.left() >= bounds.left() && delete.bottom() <= bounds.bottom());
        let parent = surface.update(|window, _| {
            gpui_kit::base::test_support::snapshots(window)
                .into_iter()
                .find(|node| node.label() == Some("来源分类"))
                .expect("official submenu row")
                .bounds()
        });
        surface.simulate_mouse_move(parent.center(), None, Default::default());
        surface.simulate_keystrokes("right");
        surface.update(|window, cx| window.draw(cx).clear(cx));
        let child = surface
            .debug_bounds("checked")
            .expect("official cascade renders its checked child");
        assert!(child.right() > bounds.right() && child.right() <= px(1200.0));
        surface.simulate_keystrokes("left");
        surface.update(|window, cx| window.draw(cx).clear(cx));
    }
    assert!(bounds.bottom() < px(800.0));
}

// Session/IPC tests below call the same terminal entry used by the official menu callbacks.
#[gpui_kit::test]
fn terminal_is_once_deferred_and_stale_trigger_never_executes(cx: &mut TestAppContext) {
    super::super::native_context_menu::test_terminal_is_once_deferred(cx);
}

#[test]
fn disabled_ancestor_has_no_executable_leaf() {
    let calls = Rc::new(Cell::new(0));
    let child_calls = calls.clone();
    let items = vec![
        MenuItem::new("parent", "禁用扩展")
            .disabled(true)
            .submenu(vec![
                MenuItem::new("child", "不应执行")
                    .on_click(move |_, _, _| child_calls.set(child_calls.get() + 1)),
            ]),
    ];
    assert_eq!(
        super::super::native_context_menu::test_callback_count(items),
        0
    );
}
