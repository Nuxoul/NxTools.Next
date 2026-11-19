use super::tests::{choose_mode, open_fixture};
use super::*;
use gpui_kit::{
    Modifiers, MouseButton, MouseDownEvent, MouseUpEvent, TestAppContext, VisualTestContext, point,
};

fn settle(surface: &mut VisualTestContext) {
    for _ in 0..3 {
        surface.refresh().expect("render reader layout");
        surface.run_until_parked();
    }
}

fn drag_divider(surface: &mut VisualTestContext, dx: f32) {
    let at = surface
        .debug_bounds("markdown-split-divider")
        .expect("official divider")
        .center();
    surface.simulate_mouse_down(at, MouseButton::Left, Modifiers::default());
    surface.update(|window, cx| window.draw(cx).clear(cx));
    surface.simulate_mouse_move(
        at + point(px(8.), px(0.)),
        MouseButton::Left,
        Modifiers::default(),
    );
    surface.simulate_mouse_move(
        at + point(px(dx), px(0.)),
        MouseButton::Left,
        Modifiers::default(),
    );
    surface.simulate_mouse_up(
        at + point(px(dx), px(0.)),
        MouseButton::Left,
        Modifiers::default(),
    );
    settle(surface);
}

#[gpui_kit::test]
fn markdown_reader_split_equal_drag_resize_reset_preserves_source(cx: &mut TestAppContext) {
    let directory = tempfile::tempdir().expect("Markdown fixture directory");
    let path = directory.path().join("分栏.md");
    let original = "# 文档\n\n源码与预览共用文档。\n\n## 下一节\n\n正文。\n";
    std::fs::write(&path, original).expect("write Markdown fixture");
    let (reader, surface) = open_fixture(cx);
    surface.simulate_resize(size(px(1200.), px(650.)));
    surface.update(|window, cx| {
        reader.update(cx, |reader, cx| {
            reader.open_paths(vec![path.clone()], None, Uuid::new_v4(), window, cx)
        })
    });
    surface.run_until_parked();
    settle(surface);
    choose_mode(surface, 1);
    let source = surface
        .debug_bounds("markdown-source-editor")
        .expect("source pane");
    assert!(
        (source.size.width - px(600.)).abs() <= px(1.),
        "initial split must be equal: {source:?}"
    );
    let preview = surface
        .debug_bounds("markdown-split-preview")
        .expect("preview pane");
    assert!((source.size.width - preview.size.width).abs() <= px(1.));
    assert!(surface.debug_bounds("markdown-source-gutter").is_none());
    let source_entity = surface.read(|cx| {
        reader
            .read(cx)
            .active_tab()
            .expect("tab")
            .draft
            .source()
            .expect("source")
            .clone()
    });
    surface.simulate_keystrokes("ctrl-end");
    surface.simulate_input("草稿");
    surface.run_until_parked();
    let selected = surface.read(|cx| source_entity.read(cx).selected_range());
    drag_divider(surface, 180.);
    let resized = surface
        .debug_bounds("markdown-source-editor")
        .expect("dragged source");
    assert!(
        resized.size.width > px(750.) && resized.size.width < px(790.),
        "drag must resize the pane: {resized:?}"
    );
    let ratio = resized.size.width / px(1200.);
    surface.simulate_resize(size(px(1600.), px(650.)));
    settle(surface);
    let enlarged = surface
        .debug_bounds("markdown-source-editor")
        .expect("enlarged source");
    assert!((enlarged.size.width - px(1600.) * ratio).abs() <= px(2.));
    assert_eq!(
        surface.read(|cx| source_entity.read(cx).selected_range()),
        selected
    );
    choose_mode(surface, 0);
    assert_eq!(
        surface
            .debug_bounds("markdown-source-editor")
            .expect("code")
            .size
            .width,
        px(1600.)
    );
    choose_mode(surface, 1);
    assert_eq!(
        surface
            .debug_bounds("markdown-source-editor")
            .expect("split again")
            .size
            .width,
        enlarged.size.width
    );
    let at = surface
        .debug_bounds("markdown-split-divider")
        .expect("divider after mode change")
        .center();
    for click_count in 1..=2 {
        surface.simulate_event(MouseDownEvent {
            position: at,
            button: MouseButton::Left,
            modifiers: Modifiers::default(),
            click_count,
            first_mouse: false,
        });
        surface.simulate_event(MouseUpEvent {
            position: at,
            button: MouseButton::Left,
            modifiers: Modifiers::default(),
            click_count,
        });
    }
    settle(surface);
    assert!(
        (surface
            .debug_bounds("markdown-source-editor")
            .expect("reset source")
            .size
            .width
            - px(800.))
        .abs()
            <= px(1.)
    );
    drag_divider(surface, -780.);
    assert_eq!(
        surface
            .debug_bounds("markdown-source-editor")
            .expect("minimum source")
            .size
            .width,
        px(240.)
    );
    drag_divider(surface, 1300.);
    assert_eq!(
        surface
            .debug_bounds("markdown-split-preview")
            .expect("minimum preview")
            .size
            .width,
        px(240.)
    );
    surface.simulate_resize(size(px(460.), px(500.)));
    settle(surface);
    assert!(surface.debug_bounds("markdown-split-preview").is_none());
    assert!(surface.debug_bounds("markdown-split-divider").is_none());
    assert_eq!(
        surface
            .debug_bounds("markdown-source-editor")
            .expect("narrow source fallback")
            .size
            .width,
        px(460.)
    );
    surface.read(|cx| {
        let tab = reader.read(cx).active_tab().expect("active tab");
        assert_eq!(
            tab.draft.source().expect("same source").entity_id(),
            source_entity.entity_id()
        );
        assert!(tab.draft.dirty());
    });
    surface.simulate_resize(size(px(1200.), px(650.)));
    settle(surface);
    choose_mode(surface, 0);
    surface.simulate_keystrokes("ctrl-z");
    surface.run_until_parked();
    assert_eq!(
        surface.read(|cx| source_entity.read(cx).value().to_string()),
        original
    );
    assert_eq!(
        std::fs::read_to_string(path).expect("read original file"),
        original
    );
}

#[gpui_kit::test]
fn markdown_reader_official_file_tabs_keep_selection_close_and_drafts(cx: &mut TestAppContext) {
    let directory = tempfile::tempdir().expect("tab fixtures");
    let first = directory.path().join("第一份.md");
    let second = directory.path().join("第二份.md");
    std::fs::write(&first, "# 第一份\n").expect("first file");
    std::fs::write(&second, "# 第二份\n").expect("second file");
    let (reader, surface) = open_fixture(cx);
    surface.simulate_resize(size(px(1400.), px(650.)));
    surface.update(|window, cx| {
        reader.update(cx, |reader, cx| {
            reader.open_paths(
                vec![first.clone(), second],
                None,
                Uuid::new_v4(),
                window,
                cx,
            )
        })
    });
    surface.run_until_parked();
    settle(surface);
    let first_close = surface
        .debug_bounds("markdown-close-tab-0")
        .expect("first tab close slot");
    surface.simulate_click(
        point(first_close.left() - px(25.), first_close.center().y),
        Modifiers::default(),
    );
    settle(surface);
    assert_eq!(surface.read(|cx| reader.read(cx).active), 0);
    choose_mode(surface, 0);
    surface.simulate_keystrokes("ctrl-end");
    surface.simulate_input("未保存");
    surface.run_until_parked();
    let second_close = surface
        .debug_bounds("markdown-close-tab-1")
        .expect("second tab close slot");
    surface.simulate_click(
        point(second_close.left() - px(25.), second_close.center().y),
        Modifiers::default(),
    );
    settle(surface);
    assert_eq!(surface.read(|cx| reader.read(cx).active), 1);
    surface.simulate_click(second_close.center(), Modifiers::default());
    settle(surface);
    surface.read(|cx| {
        let reader = reader.read(cx);
        assert_eq!(reader.tabs.len(), 1);
        let tab = reader.active_tab().expect("surviving tab");
        assert_eq!(tab.id, 0);
        assert!(tab.draft.dirty());
        assert!(tab.draft.text(&tab.document.text).contains("未保存"));
    });
    assert_eq!(
        std::fs::read_to_string(first).expect("first file remains unchanged"),
        "# 第一份\n"
    );
}
