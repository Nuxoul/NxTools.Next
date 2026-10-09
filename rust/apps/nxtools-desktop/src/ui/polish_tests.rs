//! Production clipboard popup render geometry/state regressions.
use super::*;
use gpui::{AppContext, TestAppContext};
use gpui_component::Root;

#[gpui::test]
fn clipboard_category_editor_keeps_its_input_focus_and_its_hitbox_inside_the_popup(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_component::init);
    let directory = tempfile::tempdir().expect("isolated clipboard history");
    let worker = crate::clipboard_worker::ClipboardCaptureWorkerHandle::spawn(
        directory.path().join("clipboard.db"),
        directory.path().join("clipboard-images"),
        nxtools_clipboard::ClipboardRetentionPolicy {
            max_items: 1000,
            retention_days: 30,
        },
        1_000,
    )
    .expect("start isolated worker");
    let panel_slot = Rc::new(RefCell::new(None));
    let panel_slot_for_window = Rc::clone(&panel_slot);
    let (_root, window) = cx.add_window_view(move |window, cx| {
        window.resize(size(px(300.0), px(440.0)));
        let panel = cx.new(|cx| {
            ClipboardPanel::new(
                ClipboardHistoryPage {
                    items: Vec::new(),
                    total: 0,
                    has_more: false,
                },
                vec![ClipboardCategory {
                    id: BUILTIN_ALL_CATEGORY_ID.into(),
                    name: "全部".into(),
                    kind: ClipboardCategoryKind::All,
                    sort_order: 0,
                }],
                HashMap::new(),
                Vec::new(),
                ClipboardModifierActions::default(),
                nxtools_protocol::ForegroundContext {
                    window_handle: 1,
                    focus_window_handle: 1,
                    process_id: 1,
                    process_name: "fixture".into(),
                    window_title: String::new(),
                },
                worker,
                WeakEntity::new_invalid(),
                uuid::Uuid::new_v4(),
                None,
                window,
                cx,
            )
        });
        *panel_slot_for_window.borrow_mut() = Some(panel.clone());
        Root::new(panel, window, cx)
    });
    window.simulate_resize(size(px(300.0), px(440.0)));
    window.update(|window, _| assert_eq!(window.viewport_size(), size(px(300.0), px(440.0))));
    window
        .refresh()
        .expect("render popup with category trigger");
    let panel = panel_slot
        .borrow()
        .clone()
        .expect("production panel entity");
    panel.update_in(window, |panel, _window, cx| {
        panel.source_process_names = vec!["Editor.exe".into()];
        cx.notify();
    });
    let add = window
        .debug_bounds("add-clipboard-category")
        .expect("category trigger");
    assert_eq!(add.size, size(px(28.0), px(28.0)));
    // Exercise GPUI's actual KeyUp click boundary, not a manually invoked handler.
    window.simulate_mouse_down(
        add.center(),
        gpui::MouseButton::Left,
        gpui::Modifiers::default(),
    );
    window.refresh().expect("focus actual add-category action");
    window.simulate_keystrokes("enter");
    window.simulate_event(gpui::KeyUpEvent {
        keystroke: gpui::Keystroke::parse("enter").expect("Enter stroke"),
    });
    window.refresh().expect("render editor after real click");
    let editor = window
        .debug_bounds("clipboard-category-editor")
        .expect("editor hit bounds");
    let source_list = window
        .debug_bounds("clipboard-category-source-list")
        .expect("source list bounds");
    let viewport = window
        .debug_bounds("clipboard-history-viewport")
        .expect("history viewport");
    assert!(
        editor.left() >= px(0.0) && editor.right() <= px(300.0),
        "actual editor={editor:?}"
    );
    assert!(editor.top() >= px(0.0) && editor.bottom() <= px(440.0));
    assert!(source_list.top() >= editor.top() && source_list.bottom() <= editor.bottom());
    assert!(viewport.bottom() <= px(440.0));
    let panel = panel_slot
        .borrow()
        .clone()
        .expect("production panel entity");
    panel.update_in(window, |panel, window, cx| {
        assert!(panel.category_editor.is_some());
        assert!(
            panel
                .category_name_input
                .focus_handle(cx)
                .is_focused(window)
        );
    });
    let source = window
        .debug_bounds("clipboard-category-source-0")
        .expect("source checkbox hitbox");
    assert_eq!(source.size.height, px(28.0));
    window.simulate_keystrokes("tab");
    window
        .refresh()
        .expect("Tab from name input to source action");
    window.simulate_keystrokes("enter");
    window.simulate_event(gpui::KeyUpEvent {
        keystroke: gpui::Keystroke::parse("enter").expect("Enter stroke"),
    });
    panel.update_in(window, |panel, _window, _| {
        assert!(
            panel
                .category_editor
                .as_ref()
                .expect("editor")
                .selected_sources
                .contains("editor.exe")
        );
    });
    window.simulate_keystrokes("tab");
    window.refresh().expect("Tab to cancel after source action");
    window.simulate_keystrokes("space");
    window.simulate_event(gpui::KeyUpEvent {
        keystroke: gpui::Keystroke::parse("space").expect("Space stroke"),
    });
    panel.update_in(window, |panel, window, cx| {
        assert!(
            panel.category_editor.is_none(),
            "Space cancels the modal only once"
        );
        assert!(panel.search_input.focus_handle(cx).is_focused(window));
    });
}

#[test]
fn category_width_reserves_exact_rendered_padding_without_erasing_selection_state() {
    let category = ClipboardCategory {
        id: "fixture".into(),
        name: "全部".into(),
        kind: ClipboardCategoryKind::All,
        sort_order: 0,
    };
    assert_eq!(
        category_tab_width(&category),
        CATEGORY_HORIZONTAL_PADDING
            + CATEGORY_ICON_WIDTH
            + CATEGORY_ICON_TEXT_GAP
            + UnicodeWidthStr::width(category.name.as_str()) as f32 * CATEGORY_TEXT_CELL_WIDTH
    );
    assert_eq!(CATEGORY_HORIZONTAL_PADDING, 16.0);
}

#[gpui::test]
fn passive_preview_content_and_metadata_stay_inside_the_measured_client(cx: &mut TestAppContext) {
    cx.update(gpui_component::init);
    let item = ClipboardItem {
        id: 1,
        kind: ClipboardKind::Text,
        text: "一次性的预览内容".into(),
        image_path: String::new(),
        image_width: 0,
        image_height: 0,
        file_paths: Vec::new(),
        source: nxtools_clipboard::CaptureSource {
            window_handle: 1,
            process_id: 1,
            process_name: "fixture".into(),
            process_icon_path: String::new(),
            window_title: String::new(),
        },
        captured_at_unix_ms: 1,
        last_used_at_unix_ms: 1,
        copy_count: 1,
        is_favorite: false,
        is_pinned: false,
        deleted_at_unix_ms: None,
    };
    let layout = preview_layout(&item, 1920.0, 1080.0);
    let (_root, window) = cx.add_window_view(move |window, cx| {
        window.resize(size(px(layout.window_width), px(layout.window_height)));
        let view = cx.new(|_| ClipboardPreview::new(item, layout, WeakEntity::new_invalid()));
        Root::new(view, window, cx)
    });
    window.simulate_resize(size(px(layout.window_width), px(layout.window_height)));
    window.refresh().expect("render passive production preview");
    let root = window
        .debug_bounds("clipboard-preview-root")
        .expect("preview root");
    assert_eq!(
        root.size,
        size(px(layout.window_width), px(layout.window_height))
    );
    assert!(layout.body_height + 22.0 + 8.0 + 16.0 <= layout.window_height);
    window.update(|window, _| assert_eq!(root.size, window.viewport_size()));
}

struct HistoryKeyboardFixture {
    item: ClipboardItem,
    activations: Rc<std::cell::Cell<usize>>,
}

impl Render for HistoryKeyboardFixture {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let activations = Rc::clone(&self.activations);
        history_row(
            Theme::zed_dark(),
            self.item.clone(),
            0,
            true,
            246.0,
            ClipboardModifierActions::default(),
            CategorySourceMenu {
                options: Vec::new(),
                unavailable_hint: Some("无分类".into()),
            },
            HistoryRowHandlers {
                action: Rc::new(move |_, _, _, _| activations.set(activations.get() + 1)),
                management: Rc::new(|_, _, _, _| {}),
                open: Rc::new(|_, _, _, _| {}),
                preview: Rc::new(|_, _, _, _, _, _| {}),
            },
            false,
        )
    }
}

#[gpui::test]
fn history_row_keyup_dispatches_once_with_fixed_hit_bounds(cx: &mut TestAppContext) {
    cx.update(gpui_component::init);
    let item = ClipboardItem {
        id: 1,
        kind: ClipboardKind::Text,
        text: "fixture".into(),
        image_path: String::new(),
        image_width: 0,
        image_height: 0,
        file_paths: Vec::new(),
        source: nxtools_clipboard::CaptureSource {
            window_handle: 1,
            process_id: 1,
            process_name: "fixture".into(),
            process_icon_path: String::new(),
            window_title: String::new(),
        },
        captured_at_unix_ms: 1,
        last_used_at_unix_ms: 1,
        copy_count: 1,
        is_favorite: false,
        is_pinned: false,
        deleted_at_unix_ms: None,
    };
    let activations = Rc::new(std::cell::Cell::new(0));
    let for_view = Rc::clone(&activations);
    let (_view, window) = cx.add_window_view(move |window, _| {
        window.resize(size(px(300.0), px(100.0)));
        HistoryKeyboardFixture {
            item,
            activations: for_view,
        }
    });
    window.refresh().expect("render actual history row");
    let bounds = window
        .debug_bounds("history-row-slot-0")
        .expect("production row slot");
    // Enter the production action through its actual Tab order.
    window.update(|window, _| window.focus_next());
    window.refresh().expect("focus history row action");
    window.simulate_keystrokes("enter");
    assert_eq!(activations.get(), 0);
    window.simulate_event(gpui::KeyUpEvent {
        keystroke: gpui::Keystroke::parse("enter").expect("Enter stroke"),
    });
    assert_eq!(activations.get(), 1);
    assert_eq!(bounds.size.height, px(64.0));
}
