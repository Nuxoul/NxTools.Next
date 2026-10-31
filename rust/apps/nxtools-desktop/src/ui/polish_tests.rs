//! Production clipboard popup render geometry/state regressions.
use super::*;
use gpui_kit::base::Root;
use gpui_kit::{AppContext, TestAppContext};

#[gpui_kit::test]
fn clipboard_category_editor_keeps_its_input_focus_and_its_hitbox_inside_the_popup(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_kit::init);
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
        panel
            .source_process_names
            .extend((0..20).map(|index| format!("VeryLongSyntheticEditorProcessName-{index}.exe")));
        cx.notify();
    });
    let add = window
        .debug_bounds("add-clipboard-category")
        .expect("category trigger");
    assert_eq!(add.size, size(px(28.0), px(28.0)));
    // Exercise GPUI's actual KeyUp click boundary, not a manually invoked handler.
    window.simulate_mouse_down(
        add.center(),
        gpui_kit::MouseButton::Left,
        gpui_kit::Modifiers::default(),
    );
    window.refresh().expect("focus actual add-category action");
    window.simulate_keystrokes("enter");
    window.simulate_event(gpui_kit::KeyUpEvent {
        keystroke: gpui_kit::Keystroke::parse("enter").expect("Enter stroke"),
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
    assert!(
        source_list.top() >= editor.top() && source_list.bottom() <= editor.bottom(),
        "editor={editor:?} source_list={source_list:?}"
    );
    assert!(viewport.bottom() <= px(440.0));
    let input = window
        .debug_bounds("clipboard-category-name-field")
        .expect("actual name input slot");
    let footer = window
        .debug_bounds("clipboard-category-editor-footer")
        .expect("actual cancel/save footer");
    assert_eq!(input.size.height, px(32.0));
    assert_eq!(footer.size.height, px(28.0));
    assert!(input.left() >= editor.left() && input.right() <= editor.right());
    assert!(source_list.bottom() <= footer.top());
    assert!(footer.bottom() <= editor.bottom() - px(8.0));
    assert!(footer.right() <= px(300.0) && footer.bottom() <= px(440.0));
    // Keep the original one-source keyboard/action sequence after the long-list
    // geometry case; no new tab-order assumption replaces that regression.
    panel.update_in(window, |panel, _, cx| {
        panel.source_process_names = vec!["Editor.exe".into()];
        cx.notify();
    });
    window.refresh().expect("restore original keyboard fixture");
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
    window.simulate_event(gpui_kit::KeyUpEvent {
        keystroke: gpui_kit::Keystroke::parse("enter").expect("Enter stroke"),
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
    window.simulate_event(gpui_kit::KeyUpEvent {
        keystroke: gpui_kit::Keystroke::parse("space").expect("Space stroke"),
    });
    panel.update_in(window, |panel, window, cx| {
        assert!(
            panel.category_editor.is_none(),
            "Space cancels the modal only once"
        );
        assert!(panel.search_input.focus_handle(cx).is_focused(window));
    });
    // Busy/error are owner state fixtures only: no request, native operation or
    // payload is generated by these production render checks.
    for pending in [true, false] {
        panel.update_in(window, |panel, _, cx| {
            panel.search_pending = pending;
            panel.query_error = (!pending).then(|| "合成的长查询错误说明".repeat(40));
            cx.notify();
        });
        window
            .refresh()
            .expect("render busy/error in actual compact client");
        let scope = window
            .debug_bounds("clipboard-search-scope-control")
            .expect("search scope remains reachable");
        assert_eq!(scope.size, size(px(24.0), px(24.0)));
        assert!(scope.right() <= px(300.0) && scope.bottom() <= px(440.0));
    }
    let edit_input = panel.update_in(window, |panel, _, cx| {
        panel.editing_item_id = Some(1);
        cx.notify();
        panel.edit_input.clone()
    });
    window.update(|window, cx| {
        edit_input.update(cx, |input, cx| {
            input.set_value("合成编辑内容\n".repeat(50), window, cx);
            input.focus(window, cx);
        });
    });
    window.refresh().expect("render full compact text editor");
    let editor = window
        .debug_bounds("clipboard-text-editor")
        .expect("real text editor");
    let footer = window
        .debug_bounds("clipboard-text-editor-footer")
        .expect("text cancel/save footer");
    assert_eq!(
        footer.size.height,
        px(52.0),
        "official Kbd hint line plus original action line"
    );
    for selector in ["kbd:ctrl-enter", "kbd:escape"] {
        let key = window
            .debug_bounds(selector)
            .expect("real official shortcut");
        assert!(key.top() >= footer.top() && key.bottom() <= footer.bottom());
    }
    assert!(footer.left() >= editor.left() && footer.right() <= editor.right());
    assert!(footer.bottom() <= editor.bottom() - px(8.0));
    assert!(editor.right() <= px(300.0) && editor.bottom() <= px(440.0));
    window.simulate_keystrokes("escape");
    panel.update_in(window, |panel, window, cx| {
        assert!(panel.editing_item_id.is_none());
        assert!(panel.search_input.focus_handle(cx).is_focused(window));
    });
    panel.update_in(window, |panel, window, cx| {
        panel.is_open = false;
        window.blur(cx);
        panel.handle_hide_failure("合成隐藏失败".into(), window, cx);
        assert!(panel.is_open);
        assert!(
            panel.search_input.focus_handle(cx).is_focused(window),
            "failed native hide must restore search keyboard focus"
        );
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

#[gpui_kit::test]
fn passive_preview_content_and_metadata_stay_inside_the_measured_client(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
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
        let view =
            cx.new(|cx| ClipboardPreview::new(item, layout, WeakEntity::new_invalid(), window, cx));
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
    let metadata = window
        .debug_bounds("clipboard-preview-metadata")
        .expect("passive metadata");
    assert!(metadata.left() >= root.left() && metadata.right() <= root.right());
    assert!(metadata.bottom() <= root.bottom() - px(10.0));
    window.update(|window, _| assert_eq!(root.size, window.viewport_size()));
}

struct HistoryKeyboardFixture {
    item: ClipboardItem,
    activations: Rc<std::cell::Cell<usize>>,
    management: Rc<std::cell::Cell<usize>>,
}

impl Render for HistoryKeyboardFixture {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let activations = Rc::clone(&self.activations);
        let management = Rc::clone(&self.management);
        div().w_full().child(history_row(
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
                management: Rc::new(move |_, _, _, _| management.set(management.get() + 1)),
                open: Rc::new(|_, _, _, _| {}),
                preview: Rc::new(|_, _, _, _, _, _| {}),
                list_image: None,
                source_icon_color: None,
                text_character_count: None,
                text_card_max_height: nxtools_core::default_clipboard_text_card_max_height(),
                open_targets: None,
                extension_actions: Vec::new(),
                extension_feedback: None,
            },
        ))
    }
}

#[gpui_kit::test]
fn history_row_keyup_dispatches_once_with_fixed_hit_bounds(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
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
            process_name: "FixtureEditorWithAVeryLongSourceName.exe".into(),
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
    let management = Rc::new(std::cell::Cell::new(0));
    let management_for_view = Rc::clone(&management);
    let (_view, window) = cx.add_window_view(move |window, _| {
        window.resize(size(px(300.0), px(100.0)));
        HistoryKeyboardFixture {
            item,
            activations: for_view,
            management: management_for_view,
        }
    });
    window.simulate_resize(size(px(300.0), px(100.0)));
    window.refresh().expect("render actual history row");
    let bounds = window
        .debug_bounds("history-row-slot-0")
        .expect("production row slot");
    // Enter the production action through its actual Tab order.
    window.update(|window, cx| window.focus_next(cx));
    window.refresh().expect("focus history row action");
    window.simulate_keystrokes("enter");
    assert_eq!(activations.get(), 0);
    window.simulate_event(gpui_kit::KeyUpEvent {
        keystroke: gpui_kit::Keystroke::parse("enter").expect("Enter stroke"),
    });
    assert_eq!(activations.get(), 1);
    assert_eq!(
        bounds.size.height,
        px(history::TEXT_ROW_HEIGHT + history::ROW_GAP)
    );
    let source = window
        .debug_bounds("clipboard-source-0")
        .expect("source slot");
    let name = window
        .debug_bounds("clipboard-source-name-0")
        .expect("visible source name");
    let actions = window
        .debug_bounds("clipboard-history-actions-0")
        .expect("management rail");
    let content = window
        .debug_bounds("clipboard-history-content")
        .expect("content slot");
    assert!(name.left() >= source.left() && name.right() <= source.right());
    assert!(
        source.right() <= actions.left(),
        "source and management must never overlap"
    );
    assert!(actions.right() <= bounds.right());
    assert!(content.top() >= actions.bottom());
    assert!(content.bottom() <= bounds.bottom());
    let delete = window
        .debug_bounds("clipboard-item-action-0-delete")
        .expect("delete action");
    window.simulate_click(delete.center(), gpui_kit::Modifiers::default());
    assert_eq!(management.get(), 1, "nested management dispatches once");
    assert_eq!(activations.get(), 1, "management never also pastes the row");
    // The Button deliberately prevents pointer focus; traverse the real
    // row -> edit -> pin -> favorite -> delete tab order instead.
    window.update(|window, cx| {
        for _ in 0..4 {
            window.focus_next(cx);
        }
    });
    window
        .refresh()
        .expect("keyboard focus reaches delete management action");
    window.simulate_keystrokes("space");
    window.simulate_event(gpui_kit::KeyUpEvent {
        keystroke: gpui_kit::Keystroke::parse("space").expect("Space stroke"),
    });
    assert_eq!(
        management.get(),
        2,
        "keyboard shares the management click path"
    );
    assert_eq!(activations.get(), 1);
}

#[test]
fn clipboard_settings_keep_busy_and_deferred_immediate_action_boundaries() {
    // Static complement to production geometry; never exercise real archive/user data.
    let source = include_str!("../main_window/clipboard_page.rs");
    assert_eq!(source.matches(".disabled(management_busy)").count(), 5);
    for action in [
        "request_clipboard_export(cx)",
        "request_clipboard_import(cx)",
        "request_clipboard_database_backup(cx)",
        "request_clipboard_database_compaction(cx)",
        "request_clipboard_storage_migration(cx)",
    ] {
        assert!(source.contains(action), "preserve existing action {action}");
    }
    assert!(source.contains(".disabled(hotkey_update_pending)"));
}

// The advanced components are main-workbench-only. Match the real minimum's
// available content width: 920 - module rail176 - topic rail160 - insets32.
struct AdvancedSettingsFixture {
    inputs: crate::ui::components::custom_command_editor::CustomCommandEditorInputs,
    commands: Vec<nxtools_core::CustomCommandSettings>,
    page: u8,
    empty: bool,
}

impl Render for AdvancedSettingsFixture {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        use crate::ui::components::custom_command_editor::*;
        use crate::ui::components::path_list::path_exclusion_setting;
        use crate::ui::components::quick_launch_diagnostics::*;
        use crate::ui::components::volume_scope::*;
        let theme = Theme::zed_dark();
        let body = match self.page {
            0 => custom_command_editor(
                theme,
                CustomCommandEditorState {
                    commands: &self.commands,
                    selected_index: (!self.empty).then_some(0),
                    inputs: CustomCommandEditorInputs {
                        keyword: self.inputs.keyword.clone(),
                        title: self.inputs.title.clone(),
                        program: self.inputs.program.clone(),
                        arguments: self.inputs.arguments.clone(),
                        working_directory: self.inputs.working_directory.clone(),
                    },
                    run_as_admin: true,
                    run_silently: true,
                },
                CustomCommandEditorActions {
                    select: Rc::new(|_, _, _| {}),
                    add: Rc::new(|_, _, _| {}),
                    set_enabled: Rc::new(|_, _, _| {}),
                    delete: Rc::new(|_, _, _| {}),
                    pick_program: Rc::new(|_, _, _| {}),
                    set_run_as_admin: Rc::new(|_, _, _| {}),
                    set_run_silently: Rc::new(|_, _, _| {}),
                    save: Rc::new(|_, _, _| {}),
                },
                cx,
            ),
            1 => div()
                .w_full()
                .child(volume_scope_setting(
                    theme,
                    "自动模式会持续包含新卷；取消任一卷后改为仅索引勾选项。保留完整说明而非缩小文字。",
                    vec![
                        automatic_volume_scope_row(theme, self.empty, |_, _, _| {}),
                        volume_scope_row(
                            theme,
                            "advanced-volume",
                            "D: 合成的长卷名称不会挤占状态槽",
                            "NTFS · 尚未建立索引 · 合成失败状态需要完整保留",
                            !self.empty,
                            false,
                            |_, _, _| {},
                        ),
                    ],
                ))
                .child(path_exclusion_setting(
                    theme,
                    &if self.empty {
                        Vec::new()
                    } else {
                        vec![format!("D:\\{}\\合成排除目录", "LongSyntheticPath".repeat(20))]
                    },
                    Some(0),
                    |_, _, _| {},
                    |_, _, _, _| {},
                    |_, _, _| {},
                    cx,
                ))
                .into_any_element(),
            _ => quick_launch_diagnostics(
                theme,
                QuickLaunchDiagnosticsData {
                    ranked_items: usize::MAX,
                    total_activations: if self.empty { 0 } else { u64::MAX },
                    query_associations: usize::MAX,
                    recent_directories: if self.empty { 0 } else { usize::MAX },
                    local_search_elapsed: "999999999999.99 ms".into(),
                    index_search_elapsed: "等待索引服务返回完整查询状态".into(),
                    log_path: format!("D:\\{}\\synthetic.log", "LongDirectory".repeat(30)).into(),
                    log_available: !self.empty,
                },
                QuickLaunchDiagnosticsActions {
                    clear_history: Rc::new(|_, _, _| {}),
                    open_logs: Rc::new(|_, _, _| {}),
                },
                cx,
            ),
        };
        div()
            .size_full()
            .flex()
            .child(div().w(px(176.0)).flex_none())
            .child(div().w(px(160.0)).flex_none())
            .child(
                div()
                    .debug_selector(|| "advanced-settings-content".into())
                    .flex_1()
                    .min_w_0()
                    .p(px(16.0))
                    .child(body),
            )
    }
}

#[gpui_kit::test]
fn advanced_workbench_controls_keep_real_slots_at_minimum_and_long_states(cx: &mut TestAppContext) {
    use crate::ui::components::custom_command_editor::CustomCommandEditorInputs;
    cx.update(gpui_kit::init);
    cx.update(crate::theme::apply_component_theme);
    let (view, window) = cx.add_window_view(|window, cx| {
        let mut input = || cx.new(|cx| InputState::new(window, cx));
        AdvancedSettingsFixture {
            inputs: CustomCommandEditorInputs {
                keyword: input(),
                title: input(),
                program: input(),
                arguments: input(),
                working_directory: input(),
            },
            commands: vec![nxtools_core::CustomCommandSettings {
                title: "合成的很长命令标题仍然保留编辑入口".repeat(10),
                keyword: "very-long-synthetic-keyword".repeat(10),
                ..Default::default()
            }],
            page: 0,
            empty: false,
        }
    });
    view.update_in(window, |view, window, cx| {
        for input in [
            &view.inputs.keyword,
            &view.inputs.title,
            &view.inputs.program,
            &view.inputs.arguments,
            &view.inputs.working_directory,
        ] {
            input.update(cx, |input, cx| {
                input.set_value("SyntheticLongFieldValue".repeat(30), window, cx);
            });
        }
    });
    for width in [920.0, 1060.0] {
        window.simulate_resize(size(px(width), px(480.0)));
        window.update(|window, _| assert_eq!(window.viewport_size(), size(px(width), px(480.0))));
        for page in 0..3 {
            view.update_in(window, |view, _, cx| {
                view.page = page;
                cx.notify();
            });
            window.refresh().expect("render actual advanced components");
            let selectors: &[&str] = match page {
                0 => &[
                    "quick-launch-add-custom-command",
                    "quick-launch-command-enabled",
                    "quick-launch-delete-custom-command",
                    "quick-launch-pick-command-program",
                    "quick-launch-save-custom-command",
                    "quick-launch-command-admin",
                    "quick-launch-command-silent",
                    "quick-launch-command-field-keyword",
                    "quick-launch-command-field-title",
                    "quick-launch-command-field-program",
                    "quick-launch-command-field-arguments",
                    "quick-launch-command-field-working-directory",
                ],
                1 => &["quick-launch-add-index-exclusion"],
                _ => &["quick-launch-clear-history", "quick-launch-open-logs"],
            };
            for selector in selectors {
                let bounds = window
                    .debug_bounds(selector)
                    .expect("retained control slot");
                assert!(
                    bounds.size.height >= px(24.0) && bounds.size.height <= px(32.0),
                    "official control tier {selector}: {bounds:?}"
                );
                assert!(
                    bounds.left() >= px(352.0) && bounds.right() <= px(width - 16.0),
                    "{selector}: {bounds:?}"
                );
                assert!(
                    bounds.top() >= px(0.0) && bounds.bottom() <= px(480.0),
                    "{selector}: {bounds:?}"
                );
            }
            if page == 0 {
                let row = window
                    .debug_bounds("quick-launch-command-0")
                    .expect("master row");
                let title = window
                    .debug_bounds("quick-launch-command-title-0")
                    .expect("command title");
                let context = window
                    .debug_bounds("quick-launch-command-context-0")
                    .expect("command context");
                assert_eq!(title.size.height, px(20.0));
                assert_eq!(context.size.height, px(18.0));
                assert!(title.top() >= row.top() && context.bottom() <= row.bottom());
                let program = window
                    .debug_bounds("quick-launch-command-field-program")
                    .expect("program input");
                let picker = window
                    .debug_bounds("quick-launch-pick-command-program")
                    .expect("picker");
                assert_eq!(program.center().y, picker.center().y);
            } else if page == 1 {
                let title = window
                    .debug_bounds("advanced-volume-title")
                    .expect("volume title");
                let status = window
                    .debug_bounds("advanced-volume-metadata")
                    .expect("volume status");
                assert!(title.right() <= status.left());
                let path = window
                    .debug_bounds("quick-launch-exclusion-path-0")
                    .expect("path");
                let delete = window
                    .debug_bounds("quick-launch-exclusion-delete-0")
                    .expect("delete");
                assert!(path.right() <= delete.left());
                assert_eq!(path.center().y, delete.center().y);
            } else {
                for (cell, value) in [
                    (
                        "quick-launch-diagnostics-activations",
                        "quick-launch-diagnostics-activations-value",
                    ),
                    (
                        "quick-launch-diagnostics-index-search",
                        "quick-launch-diagnostics-index-search-value",
                    ),
                ] {
                    let cell = window.debug_bounds(cell).expect("metric cell");
                    let value = window.debug_bounds(value).expect("real metric value");
                    assert!(value.left() >= cell.left() && value.right() <= cell.right());
                    assert!(value.top() >= cell.top() && value.bottom() <= cell.bottom());
                }
            }
        }
    }
    view.update_in(window, |view, _, cx| {
        view.empty = true;
        view.commands.clear();
        cx.notify();
    });
    window.simulate_resize(size(px(920.0), px(480.0)));
    for page in 0..3 {
        view.update_in(window, |view, _, cx| {
            view.page = page;
            cx.notify();
        });
        window.refresh().expect("empty/disabled advanced state");
        let selector = match page {
            0 => "quick-launch-custom-command-empty",
            1 => "quick-launch-exclusion-empty",
            _ => "quick-launch-open-logs",
        };
        let bounds = window
            .debug_bounds(selector)
            .expect("retained empty/disabled state");
        assert!(bounds.right() <= px(904.0) && bounds.bottom() <= px(480.0));
    }
}
