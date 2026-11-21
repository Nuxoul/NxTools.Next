use super::*;
use gpui_kit::{ElementId, Modifiers, TestAppContext, VisualTestContext};
use std::{cell::RefCell, rc::Rc};

fn fixture(cx: &mut TestAppContext) -> (Entity<SearchSurface>, &mut VisualTestContext) {
    cx.update(|cx| initialize(cx, true));
    let slot = Rc::new(RefCell::new(None));
    let retained = slot.clone();
    let (_, window) = cx.add_window_view(move |window, cx| {
        window.set_background_appearance(WindowBackgroundAppearance::Transparent);
        let view = cx.new(|cx| SearchSurface::new(true, window, cx));
        *retained.borrow_mut() = Some(view.clone());
        gpui_kit::base::Root::new(view, window, cx)
    });
    window.simulate_resize(size(px(652.0), px(92.0)));
    window.run_until_parked();
    window.refresh().expect("preview frame");
    let view = slot.borrow_mut().take().expect("actual preview content");
    (view, window)
}

fn assert_single_surface(window: &mut VisualTestContext) {
    let nodes = window.update(|window, _| gpui_kit::base::test_support::snapshots(window));
    let group = nodes
        .iter()
        .find(|node| node.path().last() == Some(&ElementId::from("search-surface")))
        .expect("released InputGroup")
        .bounds();
    assert_eq!(
        group,
        Bounds::new(point(px(16.0), px(16.0)), size(px(620.0), px(60.0)))
    );
    let editor = nodes
        .iter()
        .find(|node| node.label() == Some("速启搜索示例"))
        .expect("actual Kit text control")
        .bounds();
    assert!(editor.left() > group.left() && editor.right() < group.right());
    assert!(editor.top() >= group.top() && editor.bottom() <= group.bottom());
    let (quads, scale, fill) = window.update(|window, cx| {
        let color = Theme::global(cx).popover;
        assert_eq!(color.a, 1.0, "theme popup surface must be opaque");
        (
            window.painted_quads(),
            window.scale_factor(),
            gpui_kit::solid_background(color),
        )
    });
    let frames: Vec<_> = quads
        .iter()
        .filter(|quad| quad.border_color.a > 0.0 && quad.border_widths.top.as_f32() > 0.0)
        .collect();
    assert!(!frames.is_empty(), "official frame must remain painted");
    // The renderer splits one border into clipped edge quads; every piece must
    // refer to the same outer contour, never an inner input or square shell.
    for frame in frames {
        assert_eq!(frame.bounds.origin.x.as_f32() / scale, 16.0);
        assert_eq!(frame.bounds.origin.y.as_f32() / scale, 16.0);
        assert_eq!(frame.bounds.size.width.as_f32() / scale, 620.0);
        assert_eq!(frame.bounds.size.height.as_f32() / scale, 60.0);
        assert_eq!(frame.corner_radii.top_left.as_f32() / scale, 8.0);
        assert_eq!(frame.corner_radii.top_right.as_f32() / scale, 8.0);
        assert_eq!(frame.corner_radii.bottom_left.as_f32() / scale, 8.0);
        assert_eq!(frame.corner_radii.bottom_right.as_f32() / scale, 8.0);
    }
    assert!(
        quads.iter().any(|quad| {
            quad.bounds.size.width.as_f32() / scale == 620.0
                && quad.bounds.size.height.as_f32() / scale == 60.0
                && quad.background == fill
                && quad.corner_radii.top_left.as_f32() / scale == 8.0
        }),
        "the search surface must have an opaque rounded fill, not reveal desktop text"
    );
    assert!(
        quads.iter().all(|quad| {
            quad.background.is_transparent()
                || (quad.bounds.size.width.as_f32() / scale <= 620.0
                    && quad.bounds.size.height.as_f32() / scale <= 60.0)
        }),
        "Root must not paint an opaque square outside the rounded search bar: {quads:?}"
    );
}

#[gpui_kit::test]
fn one_frame_transparent_envelope_and_stable_input_across_themes(cx: &mut TestAppContext) {
    let (view, window) = fixture(cx);
    let input = view.read_with(window, |view, _| view.input.clone());
    let identity = input.entity_id();
    assert_single_surface(window);
    assert!(window.update(|window, cx| input.read(cx).focus_handle(cx).is_focused(window)));
    window.simulate_input("中文搜索 test 123");
    window.run_until_parked();
    for dark in [false, true] {
        window.simulate_keystrokes("f2");
        window.run_until_parked();
        window.refresh().expect("theme transition");
        assert_single_surface(window);
        assert_eq!(view.read_with(window, |view, _| view.dark), dark);
        assert_eq!(
            view.read_with(window, |view, _| view.input.entity_id()),
            identity
        );
        assert_eq!(
            window.read(|cx| input.read(cx).value().to_string()),
            "中文搜索 test 123"
        );
        let addon = window
            .update(|window, _| gpui_kit::base::test_support::snapshots(window))
            .into_iter()
            .find(|node| node.path().last() == Some(&ElementId::from("search-icon")))
            .expect("official addon")
            .bounds();
        window.simulate_click(addon.center(), Modifiers::default());
        assert!(window.update(|window, cx| input.read(cx).focus_handle(cx).is_focused(window)));
    }
}

#[gpui_kit::test]
fn escape_closes_the_only_preview_window(cx: &mut TestAppContext) {
    let (_, window) = fixture(cx);
    window.simulate_keystrokes("escape");
    window.run_until_parked();
    assert!(window.read(|cx| cx.windows().is_empty()));
}
