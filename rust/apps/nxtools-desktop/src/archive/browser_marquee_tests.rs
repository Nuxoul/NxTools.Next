//! Real production GPUI pointer dispatch, not rectangle helper-only tests.
use super::*;
use gpui_kit::{
    Modifiers, MouseDownEvent, MouseMoveEvent, MouseUpEvent, TestAppContext, VisualTestContext,
    point, size,
};
use std::{cell::RefCell, rc::Rc};

fn mount(cx: &mut TestAppContext, count: usize) -> (Entity<ArchiveBrowser>, VisualTestContext) {
    cx.update(gpui_kit::init);
    cx.executor().allow_parking();
    let retained = Rc::new(RefCell::new(None));
    let slot = retained.clone();
    let (_, surface) = cx.add_window_view(move |window, cx| {
        let view = cx.new(|cx| {
            let mut view = ArchiveBrowser::new(PathBuf::from(r"C:\owned\marquee.zip"), Uuid::new_v4(), WeakEntity::new_invalid(), window, cx);
            view.generation = 1;
            view.details_view = true;
            view.install(Uuid::new_v4(), (0..count).map(|ordinal| serde_json::from_value(serde_json::json!({
                "ordinal":ordinal,"name":format!("file-{ordinal:05}.txt"),"kind":"File","size":3,
                "packed_size":null,"encrypted":false,"warning_codes":[],"provenance":"Utf8Flag","codepage":65001
            })).expect("metadata")).collect(), "Zip".into(), window, cx);
            view
        });
        *slot.borrow_mut() = Some(view.clone());
        gpui_kit::base::Root::new(view, window, cx)
    });
    let handle = surface.update(|window, _| window.window_handle());
    let mut surface = VisualTestContext::from_window(handle, cx);
    surface.simulate_resize(size(px(960.), px(640.)));
    draw(&mut surface);
    (retained.borrow().clone().expect("view"), surface)
}
fn draw(surface: &mut VisualTestContext) {
    surface.run_until_parked();
    surface.update(|window, cx| {
        window.activate_window();
        window.draw(cx).clear(cx);
    });
}
fn drag(
    surface: &mut VisualTestContext,
    from: gpui_kit::Point<Pixels>,
    to: gpui_kit::Point<Pixels>,
    modifiers: Modifiers,
    release: bool,
) {
    surface.simulate_event(MouseDownEvent {
        position: from,
        button: MouseButton::Left,
        modifiers,
        click_count: 1,
        ..Default::default()
    });
    surface.simulate_event(MouseMoveEvent {
        position: to,
        pressed_button: Some(MouseButton::Left),
        modifiers,
    });
    if release {
        surface.simulate_event(MouseUpEvent {
            position: to,
            button: MouseButton::Left,
            modifiers,
            click_count: 1,
        });
    }
    draw(surface);
}
fn selected(view: &Entity<ArchiveBrowser>, surface: &VisualTestContext) -> Vec<u64> {
    view.read_with(surface, |view, _| {
        view.model.as_ref().expect("model").selected_file_ordinals()
    })
}
#[gpui_kit::test]
fn marquee_actual_empty_body_press_move_release_replaces_selection(cx: &mut TestAppContext) {
    let (view, mut surface) = mount(cx, 4);
    let row = surface.debug_bounds("archive-entry-1").expect("real row");
    let last = surface.debug_bounds("archive-entry-3").expect("last row");
    drag(
        &mut surface,
        point(row.left() + px(100.), last.bottom() + px(48.)),
        point(row.left() + px(140.), row.top() + px(2.)),
        Modifiers::default(),
        true,
    );
    assert_eq!(
        selected(&view, &surface),
        vec![1, 2, 3],
        "empty-body gesture must update original model"
    );
    view.read_with(&surface, |view, _| {
        assert!(view.open_dispatches.is_empty(), "marquee never opens a row")
    });
}
#[gpui_kit::test]
fn details_correction_default_renders_requested_five_columns(cx: &mut TestAppContext) {
    let (view, mut surface) = mount(cx, 4);
    surface.update(|window, cx| {
        view.update(cx, |view, cx| {
            // The actual constructor default, not a fixture's explicit detail preference.
            let fresh = ArchiveBrowser::new(
                PathBuf::new(),
                Uuid::new_v4(),
                WeakEntity::new_invalid(),
                window,
                cx,
            );
            view.details_view = fresh.details_view;
            cx.notify();
        })
    });
    draw(&mut surface);
    let headers = surface.update(|window, _| {
        gpui_kit::base::test_support::snapshots(window)
            .into_iter()
            .filter_map(|node| node.label().map(str::to_owned))
            .collect::<Vec<_>>()
    });
    for name in ["名称 ↑", "压缩后大小", "原始大小", "类型", "修改日期"] {
        assert!(
            headers.iter().any(|label| label == name),
            "missing default header {name}"
        );
    }
    let packed = surface.debug_bounds("archive-header-2").expect("packed");
    let size = surface.debug_bounds("archive-header-1").expect("original");
    assert!(
        packed.left() < size.left(),
        "approved packed-before-original order"
    );
}

#[gpui_kit::test]
fn marquee_ctrl_uses_fixed_xor_baseline_and_escape_restores_it(cx: &mut TestAppContext) {
    let (view, mut surface) = mount(cx, 4);
    surface.update(|_, cx| {
        view.update(cx, |view, _| {
            let model = view.model.as_mut().expect("model");
            model.select(model.visible()[1].id.clone(), false, false);
        })
    });
    draw(&mut surface);
    let row = surface.debug_bounds("archive-entry-1").expect("row");
    let last = surface.debug_bounds("archive-entry-3").expect("last");
    let start = point(row.left() + px(80.), last.bottom() + px(45.));
    let end = point(row.left() + px(120.), row.top() + px(1.));
    let control = Modifiers {
        control: true,
        ..Default::default()
    };
    drag(&mut surface, start, end, control, false);
    assert_eq!(
        selected(&view, &surface),
        vec![2, 3],
        "baseline selected1 is removed, others added"
    );
    for _ in 0..3 {
        surface.simulate_event(MouseMoveEvent {
            position: end,
            pressed_button: Some(MouseButton::Left),
            modifiers: control,
        });
        draw(&mut surface);
        assert_eq!(
            selected(&view, &surface),
            vec![2, 3],
            "no repeated toggling"
        );
    }
    surface.simulate_event(KeyDownEvent {
        keystroke: gpui_kit::Keystroke::parse("escape").expect("Escape"),
        is_held: false,
        prefer_character_input: false,
    });
    draw(&mut surface);
    assert_eq!(
        selected(&view, &surface),
        vec![1],
        "cancel restores original model baseline"
    );
    surface.simulate_event(MouseUpEvent {
        position: end,
        button: MouseButton::Left,
        modifiers: control,
        click_count: 1,
    });
    draw(&mut surface);
    assert_eq!(
        selected(&view, &surface),
        vec![1],
        "release cannot click covered row"
    );
}

#[gpui_kit::test]
fn marquee_scrolled_virtual_rows_and_row_drag_are_disjoint(cx: &mut TestAppContext) {
    let (view, mut surface) = mount(cx, 200);
    surface.update(|_, cx| {
        view.update(cx, |view, cx| {
            view.scroll
                .scroll_to_item_strict(200, ScrollStrategy::Bottom);
            cx.notify();
        })
    });
    draw(&mut surface);
    let body = surface
        .debug_bounds("archive-selection-body")
        .expect("body");
    let last = surface
        .debug_bounds("archive-entry-199")
        .expect("last projected row");
    let start = point(last.left() + px(70.), last.bottom() + px(8.));
    assert!(
        start.y < body.bottom() - px(12.),
        "virtual blank tail is reachable"
    );
    let target = point(start.x + px(40.), last.top() - px(ROW_HEIGHT * 2. - 1.));
    drag(&mut surface, start, target, Modifiers::default(), true);
    assert_eq!(
        selected(&view, &surface),
        vec![197, 198, 199],
        "scroll offset maps to immutable projected IDs"
    );
    let before = selected(&view, &surface);
    drag(
        &mut surface,
        last.center(),
        point(last.center().x + px(12.), last.center().y),
        Modifiers::default(),
        false,
    );
    assert_eq!(
        selected(&view, &surface),
        before,
        "still-pressed row movement is not empty-body marquee"
    );
    surface.simulate_event(MouseUpEvent {
        position: last.center(),
        button: MouseButton::Left,
        modifiers: Modifiers::default(),
        click_count: 1,
    });
    view.read_with(&surface, |view, _| assert!(view.open_dispatches.is_empty()));
}

#[gpui_kit::test]
fn marquee_focus_loss_projection_refresh_and_close_end_current_gesture(cx: &mut TestAppContext) {
    for mode in ["focus", "sort", "refresh", "source", "close", "snapshot"] {
        let (view, mut surface) = mount(cx, 4);
        let row = surface.debug_bounds("archive-entry-1").expect("row");
        let last = surface.debug_bounds("archive-entry-3").expect("last");
        let start = point(row.left() + px(80.), last.bottom() + px(45.));
        let end = point(row.left() + px(120.), row.top() + px(1.));
        drag(&mut surface, start, end, Modifiers::default(), false);
        assert_eq!(selected(&view, &surface), vec![1, 2, 3]);
        surface.update(|window, cx| {
            view.update(cx, |view, cx| match mode {
                "focus" => view.root_focus.focus(window, cx),
                "sort" => view.sort_column(ColumnKind::Size, window, cx),
                "refresh" => view.command(Command::Refresh, window, cx),
                "source" => view.set_failure("源已更改".into(), window, cx),
                "close" => view.retire(cx),
                _ => view.install(Uuid::new_v4(), Vec::new(), "Zip".into(), window, cx),
            })
        });
        draw(&mut surface);
        let before = selected(&view, &surface);
        surface.simulate_event(MouseMoveEvent {
            position: point(end.x, end.y - px(32.)),
            pressed_button: Some(MouseButton::Left),
            modifiers: Modifiers::default(),
        });
        surface.simulate_event(MouseUpEvent {
            position: end,
            button: MouseButton::Left,
            modifiers: Modifiers::default(),
            click_count: 1,
        });
        draw(&mut surface);
        assert_eq!(
            selected(&view, &surface),
            before,
            "retired {mode} gesture cannot publish later selection"
        );
        if mode == "focus" {
            assert!(
                before.is_empty(),
                "focus cancellation restores empty baseline"
            );
        }
    }
}
