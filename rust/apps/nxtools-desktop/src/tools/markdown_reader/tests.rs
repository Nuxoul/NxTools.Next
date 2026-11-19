use super::*;
use gpui_kit::{Modifiers, TestAppContext, VisualTestContext};

pub(super) fn open_fixture(cx: &mut TestAppContext) -> (Entity<Reader>, &mut VisualTestContext) {
    cx.update(gpui_kit::init);
    cx.update(crate::theme::apply_component_theme);
    cx.add_window_view(|window, cx| Reader::new(None, window, cx))
}

pub(super) fn choose_mode(surface: &mut VisualTestContext, index: usize) {
    let modes = surface
        .debug_bounds("markdown-mode-selector")
        .expect("mode selector");
    let center = gpui_kit::point(
        modes.left() + modes.size.width * ((index as f32 + 0.5) / 3.),
        modes.center().y,
    );
    surface.simulate_click(center, Modifiers::default());
    for _ in 0..3 {
        surface.refresh().expect("render requested mode");
        surface.run_until_parked();
    }
}

fn scroll_pane(surface: &mut VisualTestContext, selector: &'static str, dy: f32) {
    let pane = surface.debug_bounds(selector).expect("scroll pane");
    surface.simulate_event(gpui_kit::ScrollWheelEvent {
        position: pane.center(),
        delta: gpui_kit::ScrollDelta::Pixels(gpui_kit::point(px(0.), px(dy))),
        modifiers: Modifiers::default(),
        touch_phase: gpui_kit::TouchPhase::Moved,
    });
    for _ in 0..3 {
        surface.refresh().expect("apply scroll geometry");
        surface.run_until_parked();
    }
}

#[gpui_kit::test]
fn markdown_reader_opens_distinct_tabs_and_searches_rendered_text(cx: &mut TestAppContext) {
    let directory = tempfile::tempdir().unwrap();
    let first = directory.path().join("first.md");
    let second = directory.path().join("second.markdown");
    std::fs::write(&first, "# 标题\n\n**中文**\n\n- 中文").unwrap();
    std::fs::write(&second, "# Second\n\nOther page").unwrap();
    let (reader, surface) = open_fixture(cx);
    surface.update(|window, cx| {
        reader.update(cx, |reader, cx| {
            reader.open_paths(
                vec![first.clone(), second],
                None,
                Uuid::new_v4(),
                window,
                cx,
            );
        });
    });
    surface.run_until_parked();
    assert_eq!(surface.read(|cx| reader.read(cx).tabs.len()), 2);
    surface.update(|window, cx| {
        reader.update(cx, |reader, cx| {
            reader.open_paths(vec![first], None, Uuid::new_v4(), window, cx);
        });
    });
    surface.run_until_parked();
    surface.refresh().expect("render first document");
    let find = surface.debug_bounds("markdown-find").expect("Find action");
    surface.simulate_click(find.center(), Modifiers::default());
    surface.refresh().expect("render focused find input");
    surface.simulate_input("中文");
    surface.run_until_parked();
    surface.read(|cx| {
        let reader = reader.read(cx);
        assert_eq!(reader.tabs.len(), 2);
        assert_eq!(
            file_name(&reader.active_tab().unwrap().document).as_ref(),
            "first.md"
        );
        let find = reader.find.as_ref().expect("rendered document matches");
        assert_eq!(find.ranges.len(), 2);
        assert_eq!(&find.text.as_str()[find.ranges[0].clone()], "中文");
    });
}

#[gpui_kit::test]
fn markdown_reader_three_modes_preserve_the_source_editor_and_draft(cx: &mut TestAppContext) {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("edit.md");
    std::fs::write(&path, "# A draft\n\nInitial text\n").unwrap();
    let (reader, surface) = open_fixture(cx);
    surface.simulate_resize(size(px(680.), px(460.)));
    surface.update(|window, cx| {
        reader.update(cx, |reader, cx| {
            reader.open_paths(vec![path.clone()], None, Uuid::new_v4(), window, cx);
        });
    });
    surface.run_until_parked();
    surface.refresh().expect("reader at minimum size");
    let header = surface
        .debug_bounds("markdown-preview-title")
        .expect("preview title bar");
    let modes = surface
        .debug_bounds("markdown-mode-selector")
        .expect("three-way mode selector in the actual header");
    assert!(modes.left() >= header.left() && modes.right() <= header.right());
    assert!(surface.debug_bounds("markdown-edit-in-place").is_none());
    let prose = surface
        .debug_bounds("markdown-prose-viewport")
        .expect("default Preview");
    surface.simulate_click(
        gpui_kit::point(prose.center().x, prose.top() + px(80.)),
        Modifiers::default(),
    );
    surface.simulate_input("Must not edit Preview");
    surface.simulate_keystrokes("f2 f3");
    surface.refresh().expect("ordinary Preview");
    assert!(surface.debug_bounds("markdown-source-editor").is_none());
    assert!(surface.debug_bounds("markdown-rich-editor").is_none());
    assert!(!surface.read(|cx| {
        reader
            .read(cx)
            .active_tab()
            .expect("open tab")
            .draft
            .dirty()
    }));
    choose_mode(surface, 0);
    assert!(surface.debug_bounds("markdown-source-editor").is_some());
    assert!(surface.debug_bounds("markdown-prose-viewport").is_none());
    surface.simulate_keystrokes("ctrl-end");
    surface.simulate_input("New text");
    surface.run_until_parked();
    choose_mode(surface, 1);
    assert!(surface.debug_bounds("markdown-source-editor").is_some());
    assert!(surface.debug_bounds("markdown-prose-viewport").is_some());
    surface.simulate_keystrokes("ctrl-z");
    surface.run_until_parked();
    assert!(!surface.read(|cx| {
        reader
            .read(cx)
            .active_tab()
            .expect("open tab")
            .draft
            .dirty()
    }));
    surface.simulate_input("New text");
    surface.run_until_parked();
    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        "# A draft\n\nInitial text\n"
    );
    surface.read(|cx| {
        let document = reader.read(cx).active_tab().unwrap();
        assert!(document.draft.dirty());
        assert!(
            document
                .view
                .read(cx)
                .rendered_text()
                .as_str()
                .contains("New text")
        );
    });
    choose_mode(surface, 2);
    assert!(surface.debug_bounds("markdown-source-editor").is_none());
    assert!(surface.debug_bounds("markdown-rich-editor").is_none());
    assert!(surface.debug_bounds("markdown-prose-viewport").is_some());
    surface.simulate_keystrokes("ctrl-s");
    surface.run_until_parked();
    assert!(std::fs::read_to_string(&path).unwrap().contains("New text"));
    assert!(!surface.read(|cx| reader.read(cx).active_tab().unwrap().draft.dirty()));
}

#[gpui_kit::test]
fn markdown_reader_outline_long_chinese_heading_stays_inside_one_row(cx: &mut TestAppContext) {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("章节 大纲.md");
    let title = "漫长的中文章节标题与内存感知带宽延迟乘积调参方法".repeat(3);
    std::fs::write(
        &path,
        format!("# 文档\n\n## {title}\n\n正文\n\n## 下一节\n\n结尾\n"),
    )
    .unwrap();
    let (reader, surface) = open_fixture(cx);
    surface.simulate_resize(size(px(680.), px(460.)));
    surface.update(|window, cx| {
        reader.update(cx, |reader, cx| {
            reader.open_paths(vec![path], None, Uuid::new_v4(), window, cx);
        })
    });
    surface.run_until_parked();
    surface
        .refresh()
        .expect("render real outline at minimum width");
    assert!(
        surface.debug_bounds("markdown-outline-title-0").is_none(),
        "a narrow reader should not reserve a directory column"
    );
    let body = surface
        .debug_bounds("markdown-prose-viewport")
        .expect("prose viewport");
    assert!(
        body.size.width >= px(540.),
        "a narrow reader reserves room for the rail, not the full outline"
    );
    let trigger = surface
        .debug_bounds("markdown-outline-trigger")
        .expect("outline trigger");
    assert!(surface.debug_bounds("markdown-outline-rail").is_some());
    surface.simulate_click(trigger.center(), Modifiers::default());
    surface
        .refresh()
        .expect("a click on the collapsed marks is not a toggle");
    assert!(surface.debug_bounds("markdown-outline-panel").is_none());
    let focused = surface.update(|window, cx| window.focused(cx));
    surface.simulate_mouse_move(trigger.center(), None, Modifiers::default());
    surface.executor().advance_clock(Duration::from_millis(150));
    surface.refresh().expect("before outline hover deadline");
    assert!(surface.debug_bounds("markdown-outline-title-0").is_none());
    surface.executor().advance_clock(Duration::from_millis(80));
    surface.refresh().expect("hover opens outline");
    assert!(surface.debug_bounds("markdown-outline-title-0").is_some());
    let expanded = surface
        .debug_bounds("markdown-outline-panel")
        .expect("hovered outline card");
    assert!(expanded.left() <= trigger.left() && expanded.top() <= trigger.top());
    let reveal = surface
        .debug_bounds("markdown-outline-reveal")
        .expect("card reveal bounds at the start of the transition");
    assert!(reveal.size.width < expanded.size.width);
    finish_outline_reveal(surface);
    assert_eq!(
        surface.debug_bounds("markdown-outline-reveal"),
        Some(expanded)
    );
    let label = surface
        .debug_bounds("markdown-outline-title-1")
        .expect("expanded label");
    surface.simulate_mouse_move(label.center(), None, Modifiers::default());
    surface.executor().advance_clock(Duration::from_millis(250));
    surface
        .refresh()
        .expect("moving into the card does not dismiss it");
    assert!(surface.debug_bounds("markdown-outline-panel").is_some());
    assert_eq!(surface.update(|window, cx| window.focused(cx)), focused);
    surface.simulate_mouse_move(body.center(), None, Modifiers::default());
    surface.executor().advance_clock(Duration::from_millis(40));
    surface.executor().advance_clock(Duration::from_millis(180));
    surface
        .refresh()
        .expect("hover cancels outside the rail and panel");
    assert!(surface.debug_bounds("markdown-outline-title-0").is_none());
    hover_outline(surface, trigger.center());
    assert_eq!(surface.debug_bounds("markdown-prose-viewport"), Some(body));
    let first = surface
        .debug_bounds("markdown-outline-title-0")
        .expect("first heading");
    let long = surface
        .debug_bounds("markdown-outline-title-1")
        .expect("long heading");
    let next = surface
        .debug_bounds("markdown-outline-title-2")
        .expect("next heading");
    assert!(
        long.size.height <= first.size.height + px(1.) && long.bottom() <= next.top(),
        "long heading painted beyond its row: short={first:?} long={long:?} next={next:?}"
    );
    let panel = surface
        .debug_bounds("markdown-outline-panel")
        .expect("compact popover");
    let pin = surface
        .debug_bounds("markdown-outline-pin")
        .expect("pin overlay");
    assert!(panel.size.height <= px(70.) && pin.top() < long.top());
    surface.simulate_keystrokes("escape");
    surface.refresh().expect("dismiss outline");
    surface.simulate_resize(size(px(1120.), px(600.)));
    surface
        .refresh()
        .expect("wide reader still collapses outline by default");
    surface.run_until_parked();
    surface.refresh().expect("apply wide measurement");
    assert!(surface.debug_bounds("markdown-outline-pinned").is_none());
    let trigger = surface
        .debug_bounds("markdown-outline-trigger")
        .expect("wide unpinned trigger");
    hover_outline(surface, trigger.center());
    let pin = surface
        .debug_bounds("markdown-outline-pin")
        .expect("overlay pin action");
    surface.simulate_click(pin.center(), Modifiers::default());
    surface.refresh().expect("pin outline again");
    assert!(surface.debug_bounds("markdown-outline-pinned").is_some());
    let unpin = surface
        .debug_bounds("markdown-outline-pin")
        .expect("unpin action");
    surface.simulate_click(unpin.center(), Modifiers::default());
    surface
        .refresh()
        .expect("unpin returns to floating outline");
    assert!(surface.debug_bounds("markdown-outline-pinned").is_none());
    assert!(
        surface.debug_bounds("markdown-outline-panel").is_some(),
        "unpin returns to the floating outline"
    );
    finish_outline_reveal(surface);
    let pin = surface
        .debug_bounds("markdown-outline-pin")
        .expect("pin overlay");
    surface.simulate_click(pin.center(), Modifiers::default());
    surface.refresh().expect("re-pin for narrow fallback");
    surface.simulate_resize(size(px(680.), px(460.)));
    surface.refresh().expect("narrow pin fallback");
    assert!(surface.debug_bounds("markdown-outline-rail").is_some());
    assert!(surface.debug_bounds("markdown-outline-pinned").is_none());
}
#[gpui_kit::test]
fn markdown_reader_install_layout_keeps_chapters_beside_centered_prose_without_line_numbers(
    cx: &mut TestAppContext,
) {
    let directory = tempfile::tempdir().expect("temporary Markdown directory");
    let path = directory.path().join("INSTALL.md");
    std::fs::write(&path, "# 安装说明\n\n先读取配置。\n\n| 项目 | 值 |\n| --- | --- |\n| 版本 | `0.7.2` |\n\n## 验证范围\n\n最后一段。\n").expect("write fixture");
    let (reader, surface) = open_fixture(cx);
    surface.simulate_resize(size(px(2560.), px(1400.)));
    surface.update(|window, cx| {
        reader.update(cx, |reader, cx| {
            reader.open_paths(vec![path], None, Uuid::new_v4(), window, cx);
        })
    });
    surface.run_until_parked();
    for _ in 0..3 {
        surface
            .refresh()
            .expect("render and measure reference layout");
        surface.run_until_parked();
    }
    let article = surface
        .debug_bounds("markdown-prose-viewport")
        .expect("actual article column");
    assert!(
        (article.center().x - px(1280.)).abs() <= px(1.),
        "collapsed outline must not push the article off center: {article:?}"
    );
    assert_eq!(article.size.width, px(880.));
    let rail = surface
        .debug_bounds("markdown-outline-rail")
        .expect("collapsed outline by default");
    assert!(
        article.left() - rail.right() < px(100.),
        "not a screen-edge rail"
    );
    assert!(surface.debug_bounds("markdown-outline-pinned").is_none());
    assert!(surface.debug_bounds("markdown-source-gutter").is_none());
    let caption = surface
        .debug_bounds("markdown-reader-caption")
        .expect("native caption hit strip");
    let modes = surface
        .debug_bounds("markdown-mode-selector")
        .expect("mode controls");
    assert!(caption.top() <= modes.top() && modes.bottom() <= caption.bottom());
}

#[gpui_kit::test]
fn markdown_reader_outline_jumps_duplicate_headings_and_split_scrolls_both_ways(
    cx: &mut TestAppContext,
) {
    let directory = tempfile::tempdir().expect("temporary fixture directory");
    let path = directory.path().join("分栏 技术文档.md");
    let prose = (0..30)
        .map(|i| format!("第 {i} 段：**拥塞窗口** 和 `RTT` 决定发送速率。\n\n"))
        .collect::<String>();
    std::fs::write(
        &path,
        format!(
            "# 开始\n\n{prose}## 相同标题\n\n{prose}## 相同标题\n\n{prose}## 结尾\n\n最后一行\n"
        ),
    )
    .expect("write long Markdown fixture");
    let (reader, surface) = open_fixture(cx);
    surface.simulate_resize(size(px(880.), px(520.)));
    surface.update(|window, cx| {
        reader.update(cx, |reader, cx| {
            reader.open_paths(vec![path], None, Uuid::new_v4(), window, cx)
        });
    });
    surface.run_until_parked();
    surface.refresh().expect("render first section");
    surface.run_until_parked();
    surface.refresh().expect("render active rail");
    assert!(surface.debug_bounds("markdown-outline-active-0").is_some());
    let trigger = surface
        .debug_bounds("markdown-outline-trigger")
        .expect("outline trigger");
    hover_outline(surface, trigger.center());
    let panel = surface
        .debug_bounds("markdown-outline-panel")
        .expect("expanded card before jumping");
    surface.update(|window, cx| {
        let outline = reader
            .read(cx)
            .active_tab()
            .expect("active file")
            .outline
            .clone();
        outline.update(cx, |state, cx| state.focus(window, cx));
    });
    surface.simulate_keystrokes("down enter");
    surface.run_until_parked();
    surface.refresh().expect("keyboard jump to first duplicate");
    surface.run_until_parked();
    surface.refresh().expect("highlight the keyboard target");
    assert!(surface.debug_bounds("markdown-outline-active-1").is_some());
    assert!(
        surface.debug_bounds("markdown-outline-title-1").is_some(),
        "heading jumps keep the card open"
    );
    let row = surface
        .debug_bounds("markdown-outline-title-2")
        .expect("second duplicate heading");
    surface.simulate_click(row.center(), Modifiers::default());
    surface.run_until_parked();
    surface.refresh().expect("jump to the selected duplicate");
    surface.run_until_parked();
    surface.refresh().expect("highlight the selected duplicate");
    assert!(surface.debug_bounds("markdown-outline-active-2").is_some());
    assert_eq!(surface.debug_bounds("markdown-outline-panel"), Some(panel));
    surface.simulate_keystrokes("escape");
    surface
        .refresh()
        .expect("dismiss the outline before changing modes");
    let committed_top = |surface: &VisualTestContext| {
        surface.read(|cx| {
            reader
                .read(cx)
                .active_tab()
                .expect("tab")
                .view
                .read(cx)
                .list_state()
                .logical_scroll_top()
                .item_ix
        })
    };
    let duplicate_block = committed_top(surface);
    assert!(
        duplicate_block > 50,
        "second duplicate was not the jump target: {duplicate_block}"
    );
    choose_mode(surface, 1);
    let source_top = |surface: &mut VisualTestContext| {
        surface.update(|_, cx| {
            reader
                .read(cx)
                .active_tab()
                .expect("tab")
                .draft
                .source_top_line(cx)
                .expect("laid-out source line")
        })
    };
    assert!(
        source_top(surface) > 100,
        "entering Split should keep the preview section"
    );
    scroll_pane(surface, "markdown-prose-viewport", 680.);
    let from_preview = source_top(surface);
    let preview_block = committed_top(surface);
    assert!(preview_block < duplicate_block && from_preview < 120);
    scroll_pane(surface, "markdown-source-editor", -580.);
    let from_source = source_top(surface);
    let new_preview_block = committed_top(surface);
    assert!(new_preview_block > preview_block && from_source > from_preview);
    surface
        .refresh()
        .expect("no reverse feedback after the reflected scroll");
    assert_eq!(source_top(surface), from_source);
    assert_eq!(committed_top(surface), new_preview_block);
    choose_mode(surface, 2);
    scroll_pane(surface, "markdown-prose-viewport", -12000.);
    assert!(
        surface.debug_bounds("markdown-outline-active-3").is_some(),
        "at the end the final visible heading is active"
    );
}

#[gpui_kit::test]
fn markdown_reader_external_entry_reuses_tabs_and_respects_disabled_default(
    cx: &mut TestAppContext,
) {
    use nxtools_core::AppConfig;
    let directory = tempfile::tempdir().unwrap();
    let first = directory.path().join("第一份 文档.md");
    let second = directory.path().join("第二份 文档.markdown");
    std::fs::write(&first, "# First").unwrap();
    std::fs::write(&second, "# Second").unwrap();
    let mut config = AppConfig::default();
    config.markdown_reader.font_size = 20;
    config.markdown_reader.show_outline = false;
    config.markdown_reader.wide_body = true;
    let (_, surface) = open_fixture(cx);
    let config_path = directory.path().join("config.json");
    surface.update(|_, cx| {
        install_settings(&config, config_path.clone(), cx);
        assert!(open_external(first.clone(), Uuid::new_v4(), None, cx));
        assert!(open_external(second.clone(), Uuid::new_v4(), None, cx));
        assert!(open_external(first.clone(), Uuid::new_v4(), None, cx));
    });
    surface.run_until_parked();
    surface.update(|_, cx| {
        let handle = cx.global::<ReaderWindow>().0.expect("reader handle");
        handle
            .update(cx, |root, _, cx| {
                let root = root
                    .downcast::<gpui_kit::base::Root>()
                    .ok()
                    .expect("reader root");
                let view = root
                    .read(cx)
                    .view()
                    .clone()
                    .downcast::<Reader>()
                    .ok()
                    .expect("original reader");
                let reader = view.read(cx);
                assert_eq!(reader.tabs.len(), 2);
                assert_eq!(
                    file_name(&reader.active_tab().unwrap().document).as_ref(),
                    "第一份 文档.md"
                );
                assert_eq!(reader.font, 20.);
                assert_eq!(reader.measure, presentation::WIDE_MEASURE);
                assert!(!reader.outline_visible);
            })
            .unwrap();
        config.markdown_reader.enabled = false;
        install_settings(&config, config_path, cx);
        assert!(!open_external(second, Uuid::new_v4(), None, cx));
        assert!(handle.update(cx, |_, _, _| ()).is_ok());
    });
}

#[gpui_kit::test]
fn markdown_reader_prose_code_action_copies_source_inside_narrow_view(cx: &mut TestAppContext) {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("中文技术文档.md");
    std::fs::write(&path, "# 网络调参\n\n## 带宽与延迟\n\n```rust\nlet rtt_ms = 120;\n```\n\n| 参数 | 计算方法 |\n| --- | --- |\n| RTT | long_unbroken_table_cell_that_must_not_widen_the_window_more |\n").unwrap();
    let (reader, surface) = open_fixture(cx);
    surface.simulate_resize(size(px(680.), px(460.)));
    surface.update(|window, cx| {
        reader.update(cx, |reader, cx| {
            reader.open_paths(vec![path], None, Uuid::new_v4(), window, cx);
        })
    });
    surface.run_until_parked();
    surface.refresh().expect("narrow technical document");
    let body = surface
        .debug_bounds("markdown-prose-viewport")
        .expect("article viewport");
    let action = surface
        .debug_bounds("markdown-code-copy")
        .expect("code action rendered by Kit");
    assert!(action.left() >= body.left() && action.right() <= body.right());
    surface.simulate_click(action.center(), Modifiers::default());
    let copied = surface.update(|_, cx| cx.read_from_clipboard().and_then(|item| item.text()));
    assert_eq!(
        copied.as_deref().map(str::trim_end),
        Some("let rtt_ms = 120;")
    );
}

#[gpui_kit::test]
fn markdown_reader_foreground_receipt_is_separate_and_correlated(cx: &mut TestAppContext) {
    let (reader, surface) = open_fixture(cx);
    let current = Uuid::new_v4();
    let unrelated = Uuid::new_v4();
    let reply = |correlation_id, success| PlatformNotification::CommandResult {
        correlation_id,
        result: nxtools_protocol::PlatformCommandResult {
            success,
            error: None,
            native_error_code: (!success).then_some(5),
            data: None,
        },
    };
    surface.update(|window, cx| {
        reader.update(cx, |reader, cx| {
            reader.foreground = Some((current, Instant::now()));
            reader.on_platform_event(reply(unrelated, true), window, cx);
            assert!(
                reader.foreground.is_some(),
                "a different command cannot complete the Shell activation"
            );
            reader.on_platform_event(reply(current, false), window, cx);
            assert!(
                reader.foreground.is_none(),
                "a native failure must retire the current request"
            );
            assert!(
                reader.tabs.is_empty(),
                "foreground feedback does not open or select files"
            );
        })
    });
}

pub(super) fn hover_outline(
    surface: &mut VisualTestContext,
    at: gpui_kit::Point<gpui_kit::Pixels>,
) {
    surface.simulate_mouse_move(at, None, Modifiers::default());
    surface.executor().advance_clock(Duration::from_millis(210));
    surface
        .refresh()
        .expect("expand outline after resting on the marks");
    finish_outline_reveal(surface);
}

fn finish_outline_reveal(surface: &mut VisualTestContext) {
    surface.executor().advance_clock(Duration::from_millis(170));
    surface.update(|window, cx| window.simulate_next_frame(cx));
    surface.run_until_parked();
    surface
        .refresh()
        .expect("paint the completed outline reveal");
    let card = surface
        .debug_bounds("markdown-outline-panel")
        .expect("expanded outline");
    assert_eq!(surface.debug_bounds("markdown-outline-reveal"), Some(card));
}
