//! Queue07 production component and exact released navigation compatibility checks.
use super::*;
use gpui_kit::base::{Root, test_support};
use gpui_kit::component::Sizable;
use gpui_kit::{Modifiers, TestAppContext, VisualTestContext};
struct Fixture {
    panel: Entity<ClipboardPanel>,
    window: gpui_kit::AnyWindowHandle,
    worker: crate::clipboard_worker::ClipboardCaptureWorkerHandle,
    _directory: tempfile::TempDir,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.worker
            .clone()
            .shutdown()
            .expect("temp worker shutdown");
    }
}
fn fixture(cx: &mut TestAppContext) -> Fixture {
    cx.update(gpui_kit::init);
    cx.update(crate::theme::apply_component_theme);
    cx.executor().allow_parking();
    let directory = tempfile::tempdir().expect("isolated Clipboard popup");
    let worker = crate::clipboard_worker::ClipboardCaptureWorkerHandle::spawn(
        directory.path().join("clipboard.db"),
        directory.path().join("images"),
        nxtools_clipboard::ClipboardRetentionPolicy {
            max_items: 1000,
            retention_days: 30,
        },
        1000,
    )
    .expect("temp SQLite worker initialized before seed");
    let items = {
        let mut history =
            nxtools_clipboard::ClipboardHistory::open(directory.path().join("clipboard.db"))
                .expect("temp seed DB");
        history
            .capture(nxtools_clipboard::TextCapture {
                text: "合成原文\n第二行".into(),
                native_formats: vec![],
                source: nxtools_clipboard::CaptureSource {
                    window_handle: 1,
                    process_id: 1,
                    process_name: "SyntheticEditor.exe".into(),
                    process_icon_path: String::new(),
                    window_title: String::new(),
                },
                captured_at_unix_ms: nxtools_ipc::unix_time_ms(),
            })
            .expect("synthetic text seed");
        history.recent(10).expect("synthetic seed projection")
    };
    let held = worker.clone();
    let slot = Rc::new(RefCell::new(None));
    let retained = slot.clone();
    let (_, window) = cx.add_window_view(move |window, cx| {
        let panel = cx.new(|cx| {
            ClipboardPanel::new(
                ClipboardHistoryPage {
                    items,
                    total: 1,
                    has_more: false,
                },
                vec![ClipboardCategory {
                    id: BUILTIN_ALL_CATEGORY_ID.into(),
                    name: "全部".into(),
                    kind: ClipboardCategoryKind::All,
                    sort_order: 0,
                }],
                HashMap::new(),
                vec!["合成来源Editor.exe".into()],
                ClipboardModifierActions::default(),
                nxtools_protocol::ForegroundContext {
                    window_handle: 1,
                    focus_window_handle: 1,
                    process_id: 1,
                    process_name: "synthetic".into(),
                    window_title: String::new(),
                },
                held,
                WeakEntity::new_invalid(),
                uuid::Uuid::new_v4(),
                None,
                window,
                cx,
            )
        });
        *retained.borrow_mut() = Some(panel.clone());
        Root::new(panel, window, cx)
    });
    window.simulate_resize(size(px(300.0), px(440.0)));
    window.update(|window, _| window.activate_window());
    window.run_until_parked();
    window.refresh().expect("production popup");
    Fixture {
        panel: slot.borrow().clone().expect("panel"),
        window: window.update(|window, _| window.window_handle()),
        worker,
        _directory: directory,
    }
}
#[gpui_kit::test]
fn queue07_search_mounts_real_input_group_on_original_entity(cx: &mut TestAppContext) {
    let f = fixture(cx);
    let mut surface = VisualTestContext::from_window(f.window, cx);
    surface.refresh().expect("search frame");
    let nodes = surface.update(|window, _| test_support::snapshots(window));
    assert!(
        nodes
            .iter()
            .any(|n| n.path().last() == Some(&gpui_kit::ElementId::from("clipboard-search-group"))),
        "real InputGroup frame"
    );
    let search = nodes
        .iter()
        .find(|n| n.path().last() == Some(&gpui_kit::ElementId::from("clipboard-search-group")))
        .expect("search group")
        .bounds();
    let history = surface
        .debug_bounds("clipboard-history-viewport")
        .expect("history viewport");
    assert!(
        history.top() - search.bottom() >= px(8.0),
        "search needs breathing room before first history row: {search:?} {history:?}"
    );
}
#[gpui_kit::test]
fn queue07_category_input_has_no_duplicate_inset_frame(cx: &mut TestAppContext) {
    let f = fixture(cx);
    let mut surface = VisualTestContext::from_window(f.window, cx);
    f.panel.update_in(&mut surface, |panel, window, cx| {
        panel.begin_category_editor(None, window, cx)
    });
    surface.refresh().expect("category Form input");
    let slot = surface
        .debug_bounds("clipboard-category-name-field")
        .expect("name slot");
    let nodes = surface.update(|window, _| test_support::snapshots(window));
    let input = nodes
        .iter()
        .find(|n| {
            n.role() == Some(gpui_kit::Role::TextInput)
                && n.bounds().top() >= slot.top()
                && n.bounds().bottom() <= slot.bottom()
        })
        .expect("real original Input");
    assert_eq!(
        input.bounds().size.width,
        slot.size.width,
        "one styled input frame, no painted inset"
    );
}
struct TabsFixture {
    clicks: Rc<std::cell::Cell<usize>>,
}
impl Render for TabsFixture {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let clicks = self.clicks.clone();
        gpui_kit::component::tab::TabBar::new("queue07-tabs")
            .xsmall()
            .underline()
            .child(
                gpui_kit::component::tab::Tab::new()
                    .label("全部")
                    .debug_selector(|| "queue07-direct-tab".into())
                    .on_click(move |_, _, _| clicks.set(clicks.get() + 1)),
            )
    }
}
#[gpui_kit::test]
fn queue07_released_tabs_direct_keyboard_does_not_replace_category_action(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let clicks = Rc::new(std::cell::Cell::new(0));
    let kept = clicks.clone();
    let (_, surface) = cx.add_window_view(move |window, cx| {
        let view = cx.new(|_| TabsFixture { clicks: kept });
        Root::new(view, window, cx)
    });
    surface.refresh().expect("released Tabs");
    surface.update(|window, cx| {
        window.activate_window();
        window.focus_next(cx);
    });
    surface.refresh().expect("Tab traversal");
    assert!(
        surface.update(|window, cx| window.focused(cx)).is_none(),
        "direct0.7.1 Tab has no tab stop; public focus adapters remain unverified"
    );
    surface.simulate_event(gpui_kit::KeyDownEvent {
        keystroke: gpui_kit::Keystroke::parse("space").expect("Space"),
        is_held: false,
        prefer_character_input: false,
    });
    surface.simulate_event(gpui_kit::KeyUpEvent {
        keystroke: gpui_kit::Keystroke::parse("space").expect("release"),
    });
    assert_eq!(clicks.get(), 0);
    let tab = surface
        .debug_bounds("queue07-direct-tab")
        .expect("actual released Tab");
    assert!(tab.size.height <= px(28.0));
    surface.simulate_click(tab.center(), Modifiers::default());
    assert_eq!(clicks.get(), 1, "direct pointer callback is supported");
}

#[gpui_kit::test]
fn queue07_search_edit_scope_clear_preserves_input_and_query_generation(cx: &mut TestAppContext) {
    let f = fixture(cx);
    let mut surface = VisualTestContext::from_window(f.window, cx);
    let input = f
        .panel
        .read_with(&surface, |panel, _| panel.search_input.clone());
    surface.update(|window, cx| input.focus_handle(cx).focus(window, cx));
    surface.simulate_input("中文合成查询");
    surface.run_until_parked();
    let generation = f
        .panel
        .read_with(&surface, |panel, _| panel.query_dispatch_generation);
    assert!(generation > 0);
    assert_eq!(
        input.read_with(&surface, |input, _| input.value().to_string()),
        "中文合成查询"
    );
    let scope = surface
        .debug_bounds("clipboard-search-scope-control")
        .expect("actual addon");
    surface.simulate_click(scope.center(), Modifiers::default());
    surface.run_until_parked();
    surface.refresh().expect("extended scope");
    assert_eq!(
        f.panel.read_with(&surface, |panel, _| panel.search_scope),
        ClipboardSearchScope::Extended
    );
    surface.update(|window, cx| input.focus_handle(cx).focus(window, cx));
    surface.simulate_keystrokes("ctrl-a backspace");
    surface.run_until_parked();
    surface.refresh().expect("clear original input");
    assert!(input.read_with(&surface, |input, _| input.value().is_empty()));
    assert!(
        f.panel
            .read_with(&surface, |panel, _| panel.query_dispatch_generation)
            > generation
    );
    assert_eq!(
        f.panel
            .read_with(&surface, |panel, _| panel.search_input.entity_id()),
        input.entity_id()
    );
}
#[gpui_kit::test]
fn queue07_text_form_edits_saves_temp_worker_and_restores_focus(cx: &mut TestAppContext) {
    let f = fixture(cx);
    let mut surface = VisualTestContext::from_window(f.window, cx);
    let item = f
        .panel
        .read_with(&surface, |panel, _| panel.entries.borrow()[0].clone());
    f.panel.update_in(&mut surface, |panel, window, cx| {
        panel.begin_edit(&item, window, cx)
    });
    surface.refresh().expect("actual text Form");
    let input = f
        .panel
        .read_with(&surface, |panel, _| panel.edit_rich.clone());
    surface.simulate_keystrokes("ctrl-a");
    surface.simulate_input("修改的中文\n保留换行");
    surface.run_until_parked();
    surface.refresh().expect("actual edited text frame");
    assert_eq!(
        f.panel
            .read_with(&surface, |panel, cx| panel.markdown_edit_value(cx)),
        "修改的中文\n\n保留换行"
    );
    assert!(!input.read_with(&surface, |input, _| input.is_readonly()));
    let editor = surface
        .debug_bounds("clipboard-text-editor")
        .expect("editor");
    let nodes = surface.update(|window, _| test_support::snapshots(window));
    assert!(
        surface.update(|window, cx| input.focus_handle(cx).is_focused(window)),
        "actual editor focus owns keyboard input; Kit snapshots are opt-in registrations, not the native accessibility tree"
    );
    let textarea = surface
        .debug_bounds("clipboard-rich-editor")
        .expect("real Markdown editor viewport");
    assert!(textarea.left() >= editor.left() && textarea.right() <= editor.right());
    let save = nodes
        .iter()
        .find(|node| {
            node.path().last() == Some(&gpui_kit::ElementId::from("clipboard-dialog-button-保存"))
        })
        .expect("styled Save")
        .bounds();
    assert!(
        textarea.top() >= editor.top() && textarea.bottom() <= save.top(),
        "text field/footer budget: viewport={textarea:?} save={save:?} editor={editor:?}"
    );
    assert!(
        save.bottom() <= editor.bottom() && save.bottom() <= px(440.0),
        "real Save hit target: {save:?}"
    );
    surface.simulate_click(save.center(), Modifiers::default());
    for _ in 0..200 {
        surface.run_until_parked();
        if f.panel
            .read_with(&surface, |panel, _| panel.editing_item_id.is_none())
        {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    surface.run_until_parked();
    surface.refresh().expect("worker save result");
    assert!(
        f.panel
            .read_with(&surface, |panel, _| panel.editing_item_id.is_none()),
        "save state: {:?}",
        f.panel
            .read_with(&surface, |panel, _| panel.status.label().to_owned())
    );
    assert!(surface.update(|window, cx| {
        f.panel
            .read(cx)
            .search_input
            .focus_handle(cx)
            .is_focused(window)
    }));
    let history =
        nxtools_clipboard::ClipboardHistory::open(f._directory.path().join("clipboard.db"))
            .expect("temp readback");
    assert_eq!(
        history.recent(10).expect("read saved row")[0].text,
        "修改的中文\n\n保留换行"
    );
}
#[gpui_kit::test]
fn markdown_save_completion_does_not_close_a_reopened_draft(cx: &mut TestAppContext) {
    let f = fixture(cx);
    let mut surface = VisualTestContext::from_window(f.window, cx);
    let item = f
        .panel
        .read_with(&surface, |panel, _| panel.entries.borrow()[0].clone());
    f.panel.update_in(&mut surface, |panel, window, cx| {
        panel.begin_edit(&item, window, cx);
        panel.edit_source_mode = true;
        panel.edit_input.update(cx, |state, cx| {
            state.set_value("accepted first save", window, cx)
        });
        panel.save_edit(window, cx);
        panel.cancel_edit(window, cx);
        panel.begin_edit(&item, window, cx);
        panel.edit_source_mode = true;
        panel.edit_input.update(cx, |state, cx| {
            state.set_value("new unsaved draft", window, cx)
        });
    });
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        surface.run_until_parked();
        let history =
            nxtools_clipboard::ClipboardHistory::open(f._directory.path().join("clipboard.db"))
                .expect("readback");
        if history.recent(10).expect("saved entry")[0].text == "accepted first save" {
            break;
        }
        assert!(Instant::now() < deadline, "accepted save must finish");
        std::thread::sleep(Duration::from_millis(10));
    }
    for _ in 0..10 {
        surface.run_until_parked();
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(
        f.panel
            .read_with(&surface, |panel, _| panel.editing_item_id),
        Some(item.id),
        "old save must not close the reopened editor"
    );
    assert_eq!(
        f.panel
            .read_with(&surface, |panel, cx| panel.markdown_edit_value(cx)),
        "new unsaved draft"
    );
}

#[gpui_kit::test]
fn markdown_save_source_bytes_once_and_failure_keeps_draft(cx: &mut TestAppContext) {
    let f = fixture(cx);
    let mut surface = VisualTestContext::from_window(f.window, cx);
    let item = f
        .panel
        .read_with(&surface, |panel, _| panel.entries.borrow()[0].clone());
    let text = "# literal\r\n**raw**\r\n";
    f.panel.update_in(&mut surface, |panel, window, cx| {
        panel.begin_edit(&item, window, cx);
        panel.edit_source_mode = true;
        panel
            .edit_input
            .update(cx, |state, cx| state.set_value(text, window, cx));
        panel.save_edit(window, cx);
        let accepted = panel.edit_save_pending.expect("pending before dispatch");
        panel.save_edit(window, cx);
        assert_eq!(
            panel.edit_save_pending,
            Some(accepted),
            "duplicate save retains one operation"
        );
    });
    let deadline = Instant::now() + Duration::from_secs(3);
    while f
        .panel
        .read_with(&surface, |panel, _| panel.edit_save_pending.is_some())
    {
        surface.run_until_parked();
        assert!(Instant::now() < deadline, "save terminal");
        std::thread::sleep(Duration::from_millis(5));
    }
    let history =
        nxtools_clipboard::ClipboardHistory::open(f._directory.path().join("clipboard.db"))
            .expect("readback");
    assert_eq!(history.recent(10).expect("saved row")[0].text, text);
    f.panel.update_in(&mut surface, |panel, window, cx| {
        panel.begin_edit(&item, window, cx);
        panel.editing_item_id = Some(i64::MAX);
        panel.edit_source_mode = true;
        panel.edit_input.update(cx, |state, cx| {
            state.set_value("retryable draft", window, cx)
        });
        panel.save_edit(window, cx);
    });
    let deadline = Instant::now() + Duration::from_secs(3);
    while f
        .panel
        .read_with(&surface, |panel, _| panel.edit_save_pending.is_some())
    {
        surface.run_until_parked();
        assert!(Instant::now() < deadline, "failed save terminal");
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(f.panel.read_with(&surface, |panel, _| matches!(
        panel.status,
        PanelStatus::Error(_)
    )));
    assert_eq!(
        f.panel
            .read_with(&surface, |panel, _| panel.editing_item_id),
        Some(i64::MAX)
    );
    assert_eq!(
        f.panel
            .read_with(&surface, |panel, cx| panel.markdown_edit_value(cx)),
        "retryable draft"
    );
}

#[gpui_kit::test]
fn queue07_category_modal_occludes_search_and_saves_name_sources(cx: &mut TestAppContext) {
    let f = fixture(cx);
    let mut surface = VisualTestContext::from_window(f.window, cx);
    f.panel.update_in(&mut surface, |panel, window, cx| {
        panel.begin_category_editor(None, window, cx)
    });
    surface.refresh().expect("category Form");
    let input = f
        .panel
        .read_with(&surface, |panel, _| panel.category_name_input.clone());
    surface.simulate_input("中文分类长名字仍保持原实体");
    surface.run_until_parked();
    let scope = surface
        .debug_bounds("clipboard-search-scope-control")
        .expect("underlying scope");
    surface.simulate_click(scope.center(), Modifiers::default());
    assert_eq!(
        f.panel.read_with(&surface, |panel, _| panel.search_scope),
        ClipboardSearchScope::Content,
        "modal occlusion blocks underlying input addon"
    );
    let source = surface
        .debug_bounds("clipboard-category-source-0")
        .expect("original source row");
    surface.simulate_click(source.center(), Modifiers::default());
    surface.refresh().expect("source checked");
    assert_eq!(
        f.panel.read_with(&surface, |panel, _| panel
            .category_editor
            .as_ref()
            .expect("draft")
            .selected_sources
            .len()),
        1
    );
    let editor = surface
        .debug_bounds("clipboard-category-editor")
        .expect("244px editor");
    assert_eq!(editor.size.height, px(244.0));
    let save = surface.update(|window, _| {
        test_support::snapshots(window)
            .into_iter()
            .find(|node| {
                node.path().last()
                    == Some(&gpui_kit::ElementId::from("clipboard-dialog-button-保存"))
            })
            .expect("Save")
            .bounds()
    });
    assert!(save.bottom() <= editor.bottom());
    surface.simulate_click(save.center(), Modifiers::default());
    for _ in 0..200 {
        surface.run_until_parked();
        if f.panel
            .read_with(&surface, |panel, _| panel.category_editor.is_none())
        {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    surface.run_until_parked();
    surface.refresh().expect("worker category save");
    assert!(
        f.panel
            .read_with(&surface, |panel, _| panel.category_editor.is_none())
    );
    assert_eq!(
        f.panel
            .read_with(&surface, |panel, _| panel.category_name_input.entity_id()),
        input.entity_id()
    );
    let history =
        nxtools_clipboard::ClipboardHistory::open(f._directory.path().join("clipboard.db"))
            .expect("category readback");
    assert!(
        history
            .categories()
            .expect("categories")
            .iter()
            .any(|category| category.name == "中文分类长名字仍保持原实体")
    );
}

struct ToggleGroupFixture {
    changes: Rc<std::cell::Cell<usize>>,
}
impl Render for ToggleGroupFixture {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let changes = self.changes.clone();
        gpui_kit::component::button::ToggleGroup::new("queue07-toggles")
            .small()
            .child(
                gpui_kit::component::button::Toggle::new("queue07-toggle")
                    .label("全部")
                    .checked(true),
            )
            .on_click(move |_, _, _| changes.set(changes.get() + 1))
    }
}
#[gpui_kit::test]
fn queue07_released_togglegroup_keyboard_group_callback_is_not_navigation(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let changes = Rc::new(std::cell::Cell::new(0));
    let held = changes.clone();
    let (_, surface) = cx.add_window_view(move |window, cx| {
        let view = cx.new(|_| ToggleGroupFixture { changes: held });
        Root::new(view, window, cx)
    });
    surface.update(|window, cx| {
        window.activate_window();
        window.focus_next(cx);
    });
    surface.refresh().expect("focused official ToggleGroup");
    assert!(surface.update(|window, cx| window.focused(cx)).is_some());
    surface.simulate_event(gpui_kit::KeyDownEvent {
        keystroke: gpui_kit::Keystroke::parse("space").expect("Space"),
        is_held: false,
        prefer_character_input: false,
    });
    surface.simulate_event(gpui_kit::KeyUpEvent {
        keystroke: gpui_kit::Keystroke::parse("space").expect("release"),
    });
    assert_eq!(
        changes.get(),
        0,
        "directgroup keyboard callback does not preserve original selection action"
    );
}
#[gpui_kit::test]
fn queue07_long_text_and_resize_keep_form_footer_and_cancel_identity(cx: &mut TestAppContext) {
    let f = fixture(cx);
    let mut surface = VisualTestContext::from_window(f.window, cx);
    let item = f
        .panel
        .read_with(&surface, |panel, _| panel.entries.borrow()[0].clone());
    let input = f
        .panel
        .read_with(&surface, |panel, _| panel.edit_input.clone());
    for (width, height) in [(300.0, 440.0), (240.0, 380.0), (560.0, 650.0)] {
        surface.simulate_resize(size(px(width), px(height)));
        f.panel.update_in(&mut surface, |panel, window, cx| {
            panel.begin_edit(&item, window, cx)
        });
        f.panel.update_in(&mut surface, |panel, window, cx| {
            panel.edit_source_mode = true;
            input.update(cx, |input, cx| {
                input.set_value("长中文合成编辑\n".repeat(80), window, cx)
            });
            cx.notify();
        });
        surface.refresh().expect("long styled text Form");
        let nodes = surface.update(|window, _| test_support::snapshots(window));
        let field = nodes
            .iter()
            .find(|node| node.label() == Some("剪贴板文本"))
            .expect("actual Textarea")
            .bounds();
        let footer = surface
            .debug_bounds("clipboard-text-editor-footer")
            .expect("fixed footer");
        let editor = surface
            .debug_bounds("clipboard-text-editor")
            .expect("editor");
        assert!(
            field.left() >= editor.left()
                && field.right() <= editor.right()
                && field.top() >= editor.top()
                && field.bottom() <= footer.top(),
            "textarea={field:?} footer={footer:?} editor={editor:?}"
        );
        assert!(footer.bottom() <= px(height) && footer.right() <= px(width));
        for key in ["kbd:ctrl-enter", "kbd:escape"] {
            let hint = surface
                .debug_bounds(key)
                .expect("actual official editor Kbd");
            assert!(
                hint.left() >= footer.left()
                    && hint.right() <= footer.right()
                    && hint.top() >= footer.top()
                    && hint.bottom() <= footer.bottom(),
                "{key} remains visible: {hint:?} footer={footer:?}"
            );
        }
        for label in ["保存", "取消"] {
            let action = nodes
                .iter()
                .find(|node| {
                    node.path().last()
                        == Some(&gpui_kit::ElementId::from(format!(
                            "clipboard-dialog-button-{label}"
                        )))
                })
                .expect("original editor action")
                .bounds();
            assert!(
                action.left() >= footer.left()
                    && action.right() <= footer.right()
                    && action.bottom() <= footer.bottom()
                    && action.bottom() <= px(height)
            );
        }
        surface.simulate_keystrokes("escape");
        assert!(
            f.panel
                .read_with(&surface, |panel, _| panel.editing_item_id.is_none())
        );
        assert!(surface.update(|window, cx| {
            f.panel
                .read(cx)
                .search_input
                .focus_handle(cx)
                .is_focused(window)
        }));
        assert_eq!(
            f.panel
                .read_with(&surface, |panel, _| panel.edit_input.entity_id()),
            input.entity_id()
        );
    }
    let history =
        nxtools_clipboard::ClipboardHistory::open(f._directory.path().join("clipboard.db"))
            .expect("cancel readback");
    assert_eq!(
        history.recent(10).expect("unchanged synthetic row")[0].text,
        item.text
    );
}

struct FocusedTabFixture {
    focus: gpui_kit::FocusHandle,
    clicks: Rc<std::cell::Cell<usize>>,
}
impl Render for FocusedTabFixture {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let clicks = self.clicks.clone();
        gpui_kit::component::tab::TabBar::new("queue07-focused-tabs")
            .xsmall()
            .underline()
            .child(
                gpui_kit::component::tab::Tab::new()
                    .label("全部")
                    .track_focus(&self.focus)
                    .tab_stop(true)
                    .on_click(move |_, _, _| clicks.set(clicks.get() + 1)),
            )
    }
}
#[gpui_kit::test]
fn queue07_tabs_public_focus_adapter_keyboard_evidence(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let clicks = Rc::new(std::cell::Cell::new(0));
    let held = clicks.clone();
    let (_, surface) = cx.add_window_view(move |window, cx| {
        let view = cx.new(|cx| FocusedTabFixture {
            focus: cx.focus_handle(),
            clicks: held,
        });
        Root::new(view, window, cx)
    });
    surface
        .refresh()
        .expect("mount public Tab focus adapter before traversal");
    surface.update(|window, cx| {
        window.activate_window();
        window.focus_next(cx);
    });
    surface.refresh().expect("public Tab focus adapter");
    assert!(
        surface.update(|window, cx| window.focused(cx)).is_none(),
        "tested public track_focus/tab_stop recipe does not expose original keyboard target"
    );
    surface.simulate_event(gpui_kit::KeyDownEvent {
        keystroke: gpui_kit::Keystroke::parse("space").expect("Space"),
        is_held: false,
        prefer_character_input: false,
    });
    surface.simulate_event(gpui_kit::KeyUpEvent {
        keystroke: gpui_kit::Keystroke::parse("space").expect("release"),
    });
    assert_eq!(
        clicks.get(),
        0,
        "tested public Tab recipe does not dispatch keyboard selection; other adapters remain unverified"
    );
}

#[gpui_kit::test]
fn queue07_retained_category_navigation_long_names_wheel_and_context_stay_live(
    cx: &mut TestAppContext,
) {
    let f = fixture(cx);
    let mut surface = VisualTestContext::from_window(f.window, cx);
    f.panel.update_in(&mut surface, |panel, _, cx| {
        panel.categories = (0..8)
            .map(|index| ClipboardCategory {
                id: format!("synthetic-{index}"),
                name: format!("中文长分类{index}"),
                kind: ClipboardCategoryKind::Custom,
                sort_order: index,
            })
            .collect();
        panel.active_category_id = "synthetic-0".into();
        cx.notify();
    });
    surface.refresh().expect("original long-name navigation");
    assert!(
        f.panel
            .read_with(&surface, |panel, _| panel.category_scroll.max_offset().x)
            > px(0.0)
    );
    surface.simulate_event(gpui_kit::ScrollWheelEvent {
        position: point(px(100.0), px(55.0)),
        delta: gpui_kit::ScrollDelta::Lines(point(0.0, -1.0)),
        ..Default::default()
    });
    surface
        .refresh()
        .expect("original vertical-wheel horizontal navigation");
    assert!(
        f.panel
            .read_with(&surface, |panel, _| panel.category_scroll.offset().x)
            < px(0.0)
    );
    f.panel.update_in(&mut surface, |panel, _, cx| {
        panel.category_scroll.set_offset(point(px(0.0), px(0.0)));
        cx.notify();
    });
    surface.refresh().expect("first category revealed");
    surface.simulate_mouse_down(
        point(px(55.0), px(55.0)),
        MouseButton::Right,
        Modifiers::default(),
    );
    surface.simulate_mouse_up(
        point(px(55.0), px(55.0)),
        MouseButton::Right,
        Modifiers::default(),
    );
    surface.run_until_parked();
    surface.refresh().expect("original category context menu");
    assert!(
        surface
            .debug_bounds("clipboard-category-menu-rename")
            .is_some(),
        "right-click rename retained"
    );
    assert!(
        surface
            .debug_bounds("clipboard-category-menu-delete")
            .is_some(),
        "right-click delete retained"
    );
}

#[gpui_kit::test]
fn queue08_history_empty_and_query_error_keep_distinct_actual_content(cx: &mut TestAppContext) {
    let f = fixture(cx);
    let mut surface = VisualTestContext::from_window(f.window, cx);
    f.panel.update_in(&mut surface, |panel, _, cx| {
        panel.entries.borrow_mut().clear();
        panel.search_pending = false;
        panel.query_error = None;
        cx.notify();
    });
    surface.refresh().expect("actual Empty");
    assert!(
        surface.debug_bounds("clipboard-kit-empty-title").is_some(),
        "actual Empty title slot"
    );
    f.panel.update_in(&mut surface, |panel, _, cx| {
        panel.query_error = Some("合成查询错误 **literal** <b>plain</b>".into());
        cx.notify();
    });
    surface.refresh().expect("actual query Alert");
    assert!(
        surface.debug_bounds("clipboard-kit-query-error").is_some(),
        "actual Alert owner slot"
    );
    assert!(
        f.panel
            .read_with(&surface, |panel, _| panel.entries.borrow().is_empty()
                && panel.query_error.is_some())
    );
}

#[gpui_kit::test]
fn queue08_long_query_error_and_empty_keep_minimum_viewport_and_original_input(
    cx: &mut TestAppContext,
) {
    let f = fixture(cx);
    let mut surface = VisualTestContext::from_window(f.window, cx);
    surface.simulate_resize(size(px(240.0), px(380.0)));
    let original_input = f
        .panel
        .read_with(&surface, |panel, _| panel.search_input.entity_id());
    f.panel.update_in(&mut surface, |panel, _, cx| {
        panel.entries.borrow_mut().clear();
        panel.search_pending = false;
        panel.query_error = Some("合成长错误 **literal** <b>plain</b>\n".repeat(30));
        cx.notify();
    });
    surface.refresh().expect("long error bounded scroll");
    let viewport = surface
        .debug_bounds("clipboard-history-viewport")
        .expect("original viewport");
    let error = surface
        .debug_bounds("clipboard-kit-query-error")
        .expect("bounded error");
    assert!(
        error.left() >= viewport.left()
            && error.right() <= viewport.right()
            && error.bottom() <= viewport.bottom(),
        "error={error:?} viewport={viewport:?}"
    );
    f.panel.update_in(&mut surface, |panel, _, cx| {
        panel.query_error = None;
        cx.notify();
    });
    surface.refresh().expect("Empty after terminal error clear");
    let title = surface
        .debug_bounds("clipboard-kit-empty-title")
        .expect("Empty title");
    assert!(
        title.left() >= viewport.left()
            && title.right() <= viewport.right()
            && title.bottom() <= viewport.bottom()
    );
    assert_eq!(
        f.panel
            .read_with(&surface, |panel, _| panel.search_input.entity_id()),
        original_input
    );
}

#[gpui_kit::test]
fn queue10_clipboard_back_to_top_uses_styled_button_and_original_scroll_owner(
    cx: &mut TestAppContext,
) {
    let f = fixture(cx);
    let mut surface = VisualTestContext::from_window(f.window, cx);
    f.panel.update_in(&mut surface, |panel, _, cx| {
        let entry = panel.entries.borrow()[0].clone();
        *panel.entries.borrow_mut() = (0..20).map(|_| entry.clone()).collect();
        panel.list_scroll.reset(20);
        panel.total_entries = 20;
        cx.notify();
    });
    surface.refresh().expect("synthetic long list");
    f.panel.update_in(&mut surface, |panel, _, cx| {
        panel.list_scroll.scroll_to(gpui_kit::ListOffset {
            item_ix: 8,
            offset_in_item: px(0.0),
        });
        panel.list_is_scrolled = true;
        cx.notify();
    });
    surface.refresh().expect("remaining original action");
    let nodes = surface.update(|window, _| test_support::snapshots(window));
    let action = nodes
        .iter()
        .find(|node| node.path().last() == Some(&gpui_kit::ElementId::from("back-to-top")))
        .expect("real styled Button")
        .bounds();
    let viewport = surface
        .debug_bounds("clipboard-history-viewport")
        .expect("viewport");
    assert!(
        action.left() >= viewport.left()
            && action.right() <= viewport.right()
            && action.top() >= viewport.top()
            && action.bottom() <= viewport.bottom()
    );
    assert_eq!(action.size.height, px(24.0));
    surface.simulate_click(action.center(), Default::default());
    surface.run_until_parked();
    assert!(
        !f.panel
            .read_with(&surface, |panel, _| panel.list_is_scrolled)
    );
    assert_eq!(
        f.panel.read_with(&surface, |panel, _| panel
            .list_scroll
            .logical_scroll_top()
            .item_ix),
        0
    );
}

#[gpui_kit::test]
fn feedback_clipboard_compact_search_and_real_category_tab(cx: &mut TestAppContext) {
    let f = fixture(cx);
    let mut surface = VisualTestContext::from_window(f.window, cx);
    surface.refresh().expect("production compact search/nav");
    let nodes = surface.update(|window, _| test_support::snapshots(window));
    let search = nodes
        .iter()
        .find(|n| n.path().last() == Some(&gpui_kit::ElementId::from("clipboard-search-group")))
        .expect("actualInputGroup");
    assert_eq!(
        search.bounds().size.height,
        px(24.0),
        "officialSmall search"
    );
    assert!(
        nodes
            .iter()
            .any(|n| n.role() == Some(gpui_kit::Role::Tab) && n.label() == Some("全部")),
        "actualstyled categoryTab"
    );
}

#[gpui_kit::test]
fn feedback_category_tabs_pointer_and_focused_release_have_one_owner(cx: &mut TestAppContext) {
    let f = fixture(cx);
    let mut surface = VisualTestContext::from_window(f.window, cx);
    surface.update(|window, _| window.activate_window());
    surface.run_until_parked();
    f.panel.update_in(&mut surface, |panel, _, cx| {
        panel.categories = vec![
            ClipboardCategory {
                id: BUILTIN_ALL_CATEGORY_ID.into(),
                name: "全部".into(),
                kind: ClipboardCategoryKind::All,
                sort_order: 0,
            },
            ClipboardCategory {
                id: "synthetic-new".into(),
                name: "新分类".into(),
                kind: ClipboardCategoryKind::Custom,
                sort_order: 1,
            },
        ];
        cx.notify();
    });
    surface.refresh().expect("realTabs");
    let tabs = surface.update(|window, _| test_support::snapshots(window));
    let tab = tabs
        .iter()
        .find(|n| n.role() == Some(gpui_kit::Role::Tab) && n.label() == Some("新分类"))
        .expect("newTab");
    assert!(tab.bounds().size.height <= px(28.0));
    surface.simulate_click(tab.bounds().center(), Modifiers::default());
    surface.run_until_parked();
    assert_eq!(
        f.panel
            .read_with(&surface, |p, _| p.active_category_id.clone()),
        "synthetic-new"
    );
    let generation = f
        .panel
        .read_with(&surface, |p, _| p.query_dispatch_generation);
    f.panel.update_in(&mut surface, |panel, _, cx| {
        panel.active_category_id = BUILTIN_ALL_CATEGORY_ID.into();
        cx.notify();
    });
    let pointer_focus = surface.update(|window, cx| window.focused(cx).expect("Tab pointer focus"));
    surface.refresh().expect("focused real Tab");
    surface.update(|window, cx| {
        assert_eq!(
            window.focused(cx),
            Some(pointer_focus),
            "focus survives repaint"
        );
        assert!(
            !f.panel
                .read(cx)
                .search_input
                .focus_handle(cx)
                .is_focused(window),
            "Tab, not search, owns focus"
        );
    });
    surface.simulate_event(gpui_kit::KeyDownEvent {
        keystroke: gpui_kit::Keystroke::parse("space").expect("space"),
        is_held: false,
        prefer_character_input: false,
    });
    assert_eq!(
        f.panel
            .read_with(&surface, |p, _| p.active_category_id.clone()),
        BUILTIN_ALL_CATEGORY_ID,
        "keydown cannot activate category"
    );
    surface.simulate_event(gpui_kit::KeyUpEvent {
        keystroke: gpui_kit::Keystroke::parse("space").expect("release"),
    });
    surface.run_until_parked();
    assert_eq!(
        f.panel
            .read_with(&surface, |p, _| p.active_category_id.clone()),
        "synthetic-new"
    );
    assert_eq!(
        f.panel
            .read_with(&surface, |p, _| p.query_dispatch_generation),
        generation + 1
    );
    surface.refresh().expect("drag source frame");
    let start = tab.bounds().center();
    surface.simulate_event(gpui_kit::MouseDownEvent {
        position: start,
        button: gpui_kit::MouseButton::Left,
        modifiers: Modifiers::default(),
        click_count: 1,
        first_mouse: false,
    });
    surface.simulate_mouse_move(
        start + point(px(18.0), px(0.0)),
        Some(gpui_kit::MouseButton::Left),
        Modifiers::default(),
    );
    assert!(
        surface.update(|_, cx| cx.has_active_drag()),
        "actual Tab owns original CategoryDrag initiation"
    );
    surface.update(|window, cx| {
        assert!(cx.stop_active_drag(window));
    });
}

#[gpui_kit::test]
fn extension_feedback_escape_closes_only_dialog_not_clipboard_panel(cx: &mut TestAppContext) {
    use gpui_kit::component::WindowExt;
    let f = fixture(cx);
    let mut surface = VisualTestContext::from_window(f.window, cx);
    surface.update(|window, cx| {
        window.open_dialog(cx, |dialog, _, _| {
            dialog.title("合成扩展反馈").child("合成说明")
        })
    });
    surface.update(|window, cx| window.draw(cx).clear(cx));
    surface.simulate_keystrokes("escape");
    surface.run_until_parked();
    assert!(!surface.update(|window, cx| window.has_active_dialog(cx)));
    f.panel.read_with(&surface, |panel, _| {
        assert!(panel.is_open);
        assert!(
            !panel.status.is_error(),
            "Escape must not attempt native popup hide"
        );
    });
}
