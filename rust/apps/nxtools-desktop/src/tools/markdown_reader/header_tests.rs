use super::tests::{choose_mode, open_fixture};
use super::*;
use gpui_kit::{Modifiers, TestAppContext, VisualTestContext, point};

fn settle(surface: &mut VisualTestContext) {
    for _ in 0..3 {
        surface.refresh().expect("render reader controls");
        surface.run_until_parked();
    }
}

#[gpui_kit::test]
fn markdown_reader_header_font_buttons_resize_code_and_split(cx: &mut TestAppContext) {
    let directory = tempfile::tempdir().expect("font fixture directory");
    let path = directory.path().join("字号.md");
    std::fs::write(&path, "统一字号 text\n").expect("write font fixture");
    let (reader, surface) = open_fixture(cx);
    surface.simulate_resize(size(px(1200.), px(650.)));
    surface.update(|window, cx| {
        reader.update(cx, |reader, cx| {
            reader.open_paths(vec![path.clone()], None, Uuid::new_v4(), window, cx)
        })
    });
    surface.run_until_parked();
    settle(surface);
    for mode_index in [0, 1] {
        choose_mode(surface, mode_index);
        let font = surface.read(|cx| reader.read(cx).font);
        let source = surface.read(|cx| {
            reader
                .read(cx)
                .active_tab()
                .expect("tab")
                .draft
                .source()
                .expect("source editor")
                .clone()
        });
        let larger = surface
            .debug_bounds("markdown-font-larger")
            .expect("font increase button");
        surface.simulate_click(larger.center(), Modifiers::default());
        settle(surface);
        assert_eq!(surface.read(|cx| reader.read(cx).font), font + 1.);
        assert_eq!(
            surface.read(|cx| source.read(cx).line_height()),
            Some(px(((font + 1.) * 1.5).round())),
            "A+ must change the laid-out source, not only Preview"
        );
        let smaller = surface
            .debug_bounds("markdown-font-smaller")
            .expect("font decrease button");
        surface.simulate_click(smaller.center(), Modifiers::default());
        settle(surface);
        assert_eq!(
            surface.read(|cx| source.read(cx).line_height()),
            Some(px((font * 1.5).round()))
        );
        assert_eq!(
            surface.read(|cx| reader
                .read(cx)
                .active_tab()
                .expect("tab")
                .draft
                .source()
                .expect("same source")
                .entity_id()),
            source.entity_id()
        );
    }
    choose_mode(surface, 2);
    let before = surface.read(|cx| {
        reader
            .read(cx)
            .active_tab()
            .expect("preview")
            .view
            .read(cx)
            .list_state()
            .bounds_for_item(0)
            .expect("paragraph")
            .size
            .height
    });
    let larger = surface
        .debug_bounds("markdown-font-larger")
        .expect("Preview font button");
    surface.simulate_click(larger.center(), Modifiers::default());
    settle(surface);
    let after = surface.read(|cx| {
        reader
            .read(cx)
            .active_tab()
            .expect("preview")
            .view
            .read(cx)
            .list_state()
            .bounds_for_item(0)
            .expect("larger paragraph")
            .size
            .height
    });
    assert!(
        after > before,
        "A+ must enlarge the laid-out Preview paragraph"
    );
    assert_eq!(
        std::fs::read_to_string(path).expect("font changes never save"),
        "统一字号 text\n"
    );
}

#[gpui_kit::test]
fn markdown_reader_header_preview_has_no_in_place_edit(cx: &mut TestAppContext) {
    let directory = tempfile::tempdir().expect("preview fixture directory");
    let path = directory.path().join("只读.md");
    std::fs::write(&path, "# 只读预览\n\n正文\n").expect("write Preview fixture");
    let (reader, surface) = open_fixture(cx);
    surface.simulate_resize(size(px(680.), px(460.)));
    surface.update(|window, cx| {
        reader.update(cx, |reader, cx| {
            reader.open_paths(vec![path.clone()], None, Uuid::new_v4(), window, cx)
        })
    });
    surface.run_until_parked();
    settle(surface);
    assert!(
        surface.debug_bounds("markdown-edit-in-place").is_none(),
        "Preview has no in-place editing command"
    );
    let prose = surface
        .debug_bounds("markdown-prose-viewport")
        .expect("Preview body");
    surface.simulate_click(
        point(prose.center().x, prose.top() + px(70.)),
        Modifiers::default(),
    );
    surface.simulate_input("不能编辑");
    settle(surface);
    assert!(surface.debug_bounds("markdown-rich-editor").is_none());
    assert!(!surface.read(|cx| reader.read(cx).active_tab().expect("tab").draft.dirty()));
    assert_eq!(
        std::fs::read_to_string(path).expect("unchanged file"),
        "# 只读预览\n\n正文\n"
    );
}

#[gpui_kit::test]
fn markdown_reader_header_single_tab_has_no_redundant_list_menu(cx: &mut TestAppContext) {
    let directory = tempfile::tempdir().expect("tab fixture directory");
    let path = directory.path().join("INSTALL.md");
    std::fs::write(&path, "# 文档\n").expect("write tab fixture");
    let (reader, surface) = open_fixture(cx);
    surface.simulate_resize(size(px(1400.), px(650.)));
    surface.update(|window, cx| {
        reader.update(cx, |reader, cx| {
            reader.open_paths(vec![path], None, Uuid::new_v4(), window, cx)
        })
    });
    surface.run_until_parked();
    settle(surface);
    let bar = surface
        .debug_bounds("markdown-file-tabs")
        .expect("official file tabs");
    let close = surface
        .debug_bounds("markdown-close-tab-0")
        .expect("official close slot");
    assert!(
        bar.right() - close.right() <= px(16.),
        "single tab must not leave room for a duplicate tab-list button: bar={bar:?}, close={close:?}"
    );
}
