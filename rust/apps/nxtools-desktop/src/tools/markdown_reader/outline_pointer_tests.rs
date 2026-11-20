use super::tests::{choose_mode, hover_outline};
use super::*;
use gpui_kit::{Modifiers, MouseButton, TestAppContext, VisualTestContext, point};
use std::cell::RefCell;

fn settle(surface: &mut VisualTestContext) {
    for _ in 0..3 {
        surface
            .refresh()
            .expect("render reader with text selection layer");
        surface.run_until_parked();
    }
}

fn select_paragraph(surface: &mut VisualTestContext, view: &Entity<TextViewState>) {
    let paragraph = surface.read(|cx| {
        let list = view.read(cx).list_state();
        list.bounds_for_item(list.logical_scroll_top().item_ix + 1)
            .expect("paragraph after visible heading")
    });
    let from = point(paragraph.right() - px(80.), paragraph.top() + px(15.));
    let to = from + point(px(55.), px(0.));
    surface.simulate_mouse_move(from, None, Modifiers::default());
    surface.simulate_mouse_down(from, MouseButton::Left, Modifiers::default());
    surface.simulate_mouse_move(to, MouseButton::Left, Modifiers::default());
    surface.simulate_mouse_up(to, MouseButton::Left, Modifiers::default());
    settle(surface);
    assert!(
        !surface.read(|cx| view.read(cx).selected_text().is_empty()),
        "fixture must support real paragraph drag selection"
    );
    assert!(!surface.read(|cx| view.read(cx).is_selecting()));
}

#[gpui_kit::test]
fn markdown_reader_outline_click_does_not_leave_text_drag_active(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(crate::theme::apply_component_theme);
    let slot = Rc::new(RefCell::new(None));
    let reader_slot = slot.clone();
    let (_, surface) = cx.add_window_view(move |window, cx| {
        let reader = cx.new(|cx| Reader::new(None, window, cx));
        *reader_slot.borrow_mut() = Some(reader.clone());
        gpui_kit::base::Root::new(reader, window, cx)
    });
    let reader = slot.borrow().clone().expect("reader under production Root");
    let directory = tempfile::tempdir().expect("pointer fixture directory");
    let path = directory.path().join("目录点击.md");
    let paragraph = "正文支持选择和复制，目录跳转不应成为文字拖选。".repeat(35);
    std::fs::write(&path, format!("# 开始\n\n{paragraph}\n\n## 第二节：目录点击不能穿透到正文\n\n{paragraph}\n\n## 最后一节\n\n{paragraph}\n")).expect("write pointer fixture");
    surface.simulate_resize(size(px(880.), px(650.)));
    surface.update(|window, cx| {
        reader.update(cx, |reader, cx| {
            reader.open_paths(vec![path], None, Uuid::new_v4(), window, cx)
        })
    });
    surface.run_until_parked();
    settle(surface);
    let view = surface.read(|cx| reader.read(cx).active_tab().expect("tab").view.clone());

    for mode in [2, 1] {
        choose_mode(surface, mode);
        select_paragraph(surface, &view);
        surface.update(|_, cx| view.update(cx, |state, cx| state.clear_selection(cx)));
        settle(surface);
        let trigger = surface
            .debug_bounds("markdown-outline-trigger")
            .expect("outline marks");
        hover_outline(surface, trigger.center());
        for _ in 0..2 {
            let parent = surface
                .debug_bounds("markdown-outline-title-0")
                .expect("parent heading");
            surface.simulate_mouse_down(parent.center(), MouseButton::Left, Modifiers::default());
            surface.simulate_mouse_up(parent.center(), MouseButton::Left, Modifiers::default());
            settle(surface);
            assert!(
                surface.read(|cx| reader
                    .read(cx)
                    .active_tab()
                    .expect("tab")
                    .outline
                    .read(cx)
                    .entry(1)
                    .is_some()),
                "clicking a parent heading must not collapse its descendant headings"
            );
            assert!(
                surface.debug_bounds("markdown-outline-title-1").is_some(),
                "child heading remains visible after navigation"
            );
            assert!(!surface.read(|cx| view.read(cx).is_selecting()));
        }
        let row = surface
            .debug_bounds("markdown-outline-title-1")
            .expect("second heading");
        let body = surface
            .debug_bounds("markdown-prose-viewport")
            .expect("prose");
        assert!(
            body.contains(&row.center()),
            "exercise the overlapping card/prose boundary"
        );
        surface.simulate_mouse_down(row.center(), MouseButton::Left, Modifiers::default());
        surface.simulate_mouse_up(row.center(), MouseButton::Left, Modifiers::default());
        settle(surface);
        assert_eq!(
            surface.read(|cx| reader
                .read(cx)
                .active_tab()
                .expect("tab")
                .navigation
                .active_heading),
            Some(1)
        );
        assert!(
            !surface.read(|cx| view.read(cx).is_selecting()),
            "outline mouse-up must not leave the window text selection gesture active"
        );
        let selected = surface.read(|cx| view.read(cx).selected_text());
        surface.simulate_mouse_move(
            point(body.right() - px(40.), body.top() + px(150.)),
            None,
            Modifiers::default(),
        );
        settle(surface);
        assert_eq!(
            surface.read(|cx| view.read(cx).selected_text()),
            selected,
            "moving into prose with no pressed button must not extend a selection"
        );
        surface.executor().advance_clock(Duration::from_millis(40));
        surface.executor().advance_clock(Duration::from_millis(180));
        settle(surface);
        select_paragraph(surface, &view);
        surface.update(|_, cx| view.update(cx, |state, cx| state.clear_selection(cx)));
        settle(surface);
    }
}
