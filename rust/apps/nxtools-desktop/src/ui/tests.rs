//! Tests for the clipboard panel module. Extracted from mod.rs so the
//! runtime code stays readable.

use super::*;
use gpui::{ScrollDelta, ScrollWheelEvent, TestAppContext};
use gpui_component::Root;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[test]
fn history_count_belongs_to_the_header_not_the_search_row() {
    assert_eq!(clipboard_header_summary("全部", 134), "全部 · 134 条");
}

#[test]
fn clipboard_page_append_mutates_shared_entries_without_copy_on_write() {
    let source = include_str!("mod.rs");
    assert!(source.contains("entries: Rc<RefCell<Vec<ClipboardItem>>>,"));
    assert!(source.contains("panel.entries.borrow_mut().extend(page.items)"));
    assert!(!source.contains("Rc::make_mut(&mut panel.entries)"));
}

#[test]
fn clipboard_history_window_is_bounded_and_can_load_both_directions() {
    let source = include_str!("mod.rs");
    assert!(source.contains("const HISTORY_WINDOW_SIZE: usize = HISTORY_PAGE_SIZE * 4;"));
    assert!(source.contains("window_start_offset: usize"));
    assert!(source.contains("total_entries: usize"));
    assert!(source.contains("trim_history_window_from_front"));
    assert!(source.contains("trim_history_window_from_back"));
    assert!(source.contains("ClipboardPageRequest::Append"));
    assert!(source.contains("ClipboardPageRequest::Prepend"));
    assert!(source.contains("query_category_page_after_async"));
    assert!(source.contains("self.list_scroll.splice(0..removed, 0)"));
}

#[test]
fn clipboard_query_supersession_keeps_committed_rows_out_of_error_state() {
    let source = include_str!("mod.rs");
    assert!(source.contains("Err(ClipboardHistoryError::QuerySuperseded)"));
    assert!(source.contains("panel.query_error = None;"));
    assert!(
        source
            .contains("if replacing {\n                            panel.search_pending = false;")
    );
}

#[test]
fn image_resource_cache_evicts_the_oldest_entry_beyond_capacity() {
    let mut cache = BoundedImageResourceCache::new(2);
    let a = Path::new("C:\\cache\\a.png");
    let b = Path::new("C:\\cache\\b.png");
    let c = Path::new("C:\\cache\\c.png");

    assert!(cache.touch(a).is_none());
    assert!(cache.touch(b).is_none());
    assert_eq!(cache.len(), 2);

    // Inserting a third entry evicts the least recently used entry.
    assert_eq!(cache.touch(c).as_deref(), Some(a));

    // Re-touching a resident entry refreshes recency instead of evicting it.
    assert!(cache.touch(b).is_none());
    assert_eq!(cache.touch(a).as_deref(), Some(c));
    assert_eq!(cache.len(), 2);

    // Empty paths never enter the cache.
    assert!(cache.touch(Path::new("")).is_none());
    assert_eq!(cache.len(), 2);

    // The production cache is bounded at 96 entries.
    assert_eq!(IMAGE_RESOURCE_CACHE_CAPACITY, 96);
}

#[gpui::test]
fn production_clipboard_history_uses_the_shared_empty_state(cx: &mut TestAppContext) {
    cx.update(gpui_component::init);
    let page = ClipboardHistoryPage {
        items: Vec::new(),
        total: 0,
        has_more: false,
    };
    let categories = vec![ClipboardCategory {
        id: BUILTIN_ALL_CATEGORY_ID.into(),
        name: "全部".into(),
        kind: ClipboardCategoryKind::All,
        sort_order: 0,
    }];
    let directory = tempfile::tempdir().expect("clipboard panel worker directory");
    let history_worker = crate::clipboard_worker::ClipboardCaptureWorkerHandle::spawn(
        directory.path().join("clipboard.db"),
        directory.path().join("clipboard-images"),
        nxtools_clipboard::ClipboardRetentionPolicy {
            max_items: 1000,
            retention_days: 30,
        },
        1_000,
    )
    .expect("start clipboard panel worker");
    let panel_slot = Rc::new(RefCell::new(None));
    let panel_slot_for_window = Rc::clone(&panel_slot);
    let (_root, window) = cx.add_window_view(move |window, cx| {
        window.resize(size(px(300.0), px(440.0)));
        let panel = cx.new(|cx| {
            ClipboardPanel::new(
                page,
                categories,
                HashMap::new(),
                Vec::new(),
                ClipboardModifierActions::default(),
                nxtools_protocol::ForegroundContext {
                    window_handle: 1,
                    focus_window_handle: 1,
                    process_id: 1,
                    process_name: "editor".into(),
                    window_title: "document".into(),
                },
                history_worker,
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

    window.refresh().expect("render clipboard empty state");
    let panel = panel_slot.borrow().clone().expect("clipboard panel entity");
    panel.update_in(window, |panel, window, cx| {
        assert!(panel.search_input.focus_handle(cx).is_focused(window));
    });
    let state = window
        .debug_bounds("clipboard-history-empty")
        .expect("clipboard shared empty state");
    let icon = window
        .debug_bounds("clipboard-history-empty-icon")
        .expect("clipboard empty icon");
    let title = window
        .debug_bounds("clipboard-history-empty-title")
        .expect("clipboard empty title");
    let description = window
        .debug_bounds("clipboard-history-empty-description")
        .expect("clipboard empty description");
    assert_eq!(icon.size, size(px(18.0), px(18.0)));
    assert_eq!(title.size.height, px(18.0));
    assert_eq!(description.size.height, px(16.0));
    assert_eq!(title.left(), description.left());
    // The empty state fills its viewport exactly (harness-independent fill
    // invariant); absolute window width varies with the test display.
    let viewport = window
        .debug_bounds("clipboard-history-viewport")
        .expect("clipboard history viewport");
    assert_eq!(state.size.width, viewport.size.width);
    panel.update_in(window, |panel, _window, cx| {
        panel.search_pending = true;
        panel.query_error = None;
        panel.status = PanelStatus::Pending("正在搜索…");
        cx.notify();
    });
    window.refresh().expect("render clipboard loading state");
    assert!(window.debug_bounds("clipboard-history-loading").is_some());

    panel.update_in(window, |panel, _window, cx| {
        panel.search_pending = false;
        panel.query_error = Some("worker query failed".into());
        panel.status = PanelStatus::Error("查询失败，请查看日志".into());
        cx.notify();
    });
    window
        .refresh()
        .expect("render clipboard query error state");
    assert!(
        window
            .debug_bounds("clipboard-history-query-error")
            .is_some()
    );
    panel.update_in(window, |panel, _window, cx| {
        panel.category_editor = Some(CategoryEditorState {
            category_id: None,
            selected_sources: HashSet::new(),
        });
        cx.notify();
    });
    window.refresh().expect("render compact category editor");
    let editor = window
        .debug_bounds("clipboard-category-editor")
        .expect("compact category editor");
    let backdrop = window
        .debug_bounds("clipboard-category-editor-backdrop")
        .expect("category editor backdrop");
    assert_eq!(editor.size.height, px(CATEGORY_EDITOR_HEIGHT));
    assert!((editor.top().as_f32() - CATEGORY_EDITOR_TOP).abs() <= 1.0);
    assert!(editor.bottom() <= backdrop.bottom());
    assert!(backdrop.size.height >= px(400.0));
}

#[test]
fn clipboard_category_editor_consumes_pointer_events_before_history_rows() {
    let source = include_str!("mod.rs");
    let editor = source
        .split(".when(is_editing_category")
        .nth(1)
        .and_then(|body| body.split(".child(panel_resize_hitbox").next())
        .expect("category editor render block");
    assert!(editor.contains("clipboard-category-editor"));
    assert!(editor.contains(".on_mouse_down(MouseButton::Left"));
    assert!(editor.contains("cx.stop_propagation()"));
}

#[test]
fn category_editor_is_compact_occluding_and_keeps_preview_outside_the_modal() {
    let source = include_str!("mod.rs");
    let editor = source
        .split(".when(is_editing_category")
        .nth(1)
        .and_then(|body| body.split(".child(panel_resize_hitbox").next())
        .expect("category editor render block");
    assert!(editor.contains(".h(px(CATEGORY_EDITOR_HEIGHT))"));
    assert!(editor.contains("clipboard-category-editor-backdrop"));
    assert!(editor.contains(".occlude()"));
    let dialog = editor
        .split("id(\"clipboard-category-editor\")")
        .nth(1)
        .expect("category editor dialog");
    assert!(!dialog.contains(".bottom_0()"));

    assert!(source.contains("let preview_enabled = !is_editing_category"));
}

#[test]
fn clipboard_context_menu_can_escape_the_panel_content_clip() {
    let source = include_str!("mod.rs");
    let frame = source
        .split("fn panel_window_frame")
        .nth(1)
        .and_then(|body| body.split("struct PopupDismissContext").next())
        .expect("clipboard panel frame");
    assert!(frame.contains("clipboard-panel-shell"));
    assert!(
        !frame.contains(".overflow_hidden()"),
        "the native panel frame must not clip a context menu anchored near its bottom edge"
    );
}

#[test]
fn clipboard_submenu_uses_window_space_and_clamps_inside_the_native_popup() {
    let source = include_str!("../../../../../vendor/gpui-component/src/menu/popup_menu.rs");
    assert!(source.contains("fn resolve_submenu_x("));
    assert!(source.contains("desired_x.clamp(edge, max_x)"));
    assert!(source.contains(".position(submenu_position)"));
    assert!(!source.contains("deferred("));
}

#[gpui::test]
fn panel_shell_fills_the_native_window_without_a_uniform_transparent_gutter(
    cx: &mut TestAppContext,
) {
    struct PanelFrameFixture(Rc<Cell<u32>>);

    impl Render for PanelFrameFixture {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            let presses = Rc::clone(&self.0);
            panel_window_frame(
                Theme::zed_dark(),
                div()
                    .size_full()
                    .on_mouse_down(gpui::MouseButton::Left, move |_event, _window, _cx| {
                        presses.set(presses.get() + 1)
                    }),
            )
        }
    }

    let window = cx.add_empty_window();
    let presses = Rc::new(Cell::new(0));
    let fixture_presses = Rc::clone(&presses);
    window.draw(
        point(px(0.0), px(0.0)),
        size(px(100.0), px(100.0)),
        move |_window, cx| cx.new(|_| PanelFrameFixture(fixture_presses)),
    );

    window.simulate_mouse_down(
        point(px(2.0), px(50.0)),
        gpui::MouseButton::Left,
        gpui::Modifiers::default(),
    );
    assert_eq!(presses.get(), 1);
}

#[gpui::test]
fn inset_sections_leave_safe_edges_and_resize_stays_on_the_native_corner(cx: &mut TestAppContext) {
    struct SpacingFixture {
        content_presses: Rc<Cell<u32>>,
        resize_presses: Rc<Cell<u32>>,
    }

    impl Render for SpacingFixture {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            let content_presses = Rc::clone(&self.content_presses);
            let resize_presses = Rc::clone(&self.resize_presses);
            panel_window_frame(
                Theme::zed_dark(),
                div()
                    .relative()
                    .size_full()
                    .flex()
                    .flex_col()
                    .child(inset_panel_section(div().flex_1().on_mouse_down(
                        gpui::MouseButton::Left,
                        move |_event, _window, _cx| {
                            content_presses.set(content_presses.get() + 1);
                        },
                    )))
                    .child(panel_resize_hitbox(div().on_mouse_down(
                        gpui::MouseButton::Left,
                        move |_event, _window, cx| {
                            cx.stop_propagation();
                            resize_presses.set(resize_presses.get() + 1);
                        },
                    ))),
            )
        }
    }

    let window = cx.add_empty_window();
    let content_presses = Rc::new(Cell::new(0));
    let resize_presses = Rc::new(Cell::new(0));
    let fixture_content_presses = Rc::clone(&content_presses);
    let fixture_resize_presses = Rc::clone(&resize_presses);
    window.draw(
        point(px(0.0), px(0.0)),
        size(px(100.0), px(40.0)),
        move |_window, cx| {
            cx.new(|_| SpacingFixture {
                content_presses: fixture_content_presses,
                resize_presses: fixture_resize_presses,
            })
        },
    );

    window.simulate_mouse_down(
        point(px(4.0), px(20.0)),
        gpui::MouseButton::Left,
        gpui::Modifiers::default(),
    );
    assert_eq!(content_presses.get(), 0);

    window.simulate_mouse_down(
        point(px(9.0), px(20.0)),
        gpui::MouseButton::Left,
        gpui::Modifiers::default(),
    );
    assert_eq!(content_presses.get(), 1);

    window.simulate_mouse_down(
        point(px(99.0), px(39.0)),
        gpui::MouseButton::Left,
        gpui::Modifiers::default(),
    );
    assert_eq!(resize_presses.get(), 1);
}

#[test]
fn transparent_panel_frame_does_not_paint_a_rectangular_drop_shadow() {
    let source = include_str!("mod.rs");
    let frame_start = source
        .find("fn panel_window_frame")
        .expect("clipboard panel frame implementation");
    let frame_end = source[frame_start..]
        .find("#[derive(Debug, Clone, Copy, PartialEq, Eq)]")
        .expect("end of clipboard panel frame implementation");
    let frame_source = &source[frame_start..frame_start + frame_end];

    assert!(
        !frame_source.contains(".shadow_"),
        "drop shadows paint a visible rectangular halo into a transparent Windows surface"
    );
}

#[test]
fn transparent_panel_uses_a_square_outer_shell_without_corner_gutters() {
    let source = include_str!("mod.rs");
    let frame = source
        .split("fn panel_window_frame")
        .nth(1)
        .and_then(|source| source.split("struct PopupDismissContext").next())
        .expect("panel window frame source");

    assert!(frame.contains(".size_full()"));
    assert!(
        !frame.contains(".rounded("),
        "a rounded transparent root exposes the rectangular HWND behind its corners"
    );
}

#[gpui::test]
fn rendered_category_bar_moves_on_vertical_wheel(cx: &mut TestAppContext) {
    struct CategoryScrollFixture(ScrollHandle);

    impl Render for CategoryScrollFixture {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            let rendered_handle = self.0.clone();
            let wheel_handle = self.0.clone();
            div()
                .id("category-fixture-scroll")
                .w(px(120.0))
                .h(px(26.0))
                .overflow_x_scroll()
                .track_scroll(&rendered_handle)
                .on_scroll_wheel(move |event, window, app| {
                    app.stop_propagation();
                    let delta = event.delta.pixel_delta(px(48.0));
                    let update = resolve_category_wheel_offset(
                        wheel_handle.offset().x.as_f32(),
                        wheel_handle.max_offset().width.as_f32(),
                        delta.x.as_f32(),
                        delta.y.as_f32(),
                    );
                    if update.changed {
                        wheel_handle.set_offset(point(px(update.offset), px(0.0)));
                        window.refresh();
                    }
                })
                .child(
                    div()
                        .h(px(26.0))
                        .w(px(8.0 * 48.0 + 7.0 * CATEGORY_GAP))
                        .flex()
                        .flex_none()
                        .gap_1()
                        .children((0_usize..8).map(|index| {
                            div()
                                .id(("category-fixture", index))
                                .w(px(48.0))
                                .h(px(26.0))
                                .flex_none()
                        })),
                )
        }
    }

    let window = cx.add_empty_window();
    let handle = ScrollHandle::new();
    let fixture_handle = handle.clone();

    window.draw(
        point(px(0.0), px(0.0)),
        size(px(120.0), px(30.0)),
        move |_window, cx| cx.new(|_| CategoryScrollFixture(fixture_handle)),
    );

    assert!(handle.max_offset().width > px(0.0));
    window.simulate_event(ScrollWheelEvent {
        position: point(px(10.0), px(10.0)),
        delta: ScrollDelta::Lines(point(0.0, -1.0)),
        ..Default::default()
    });

    assert!(handle.offset().x < px(0.0));
}

#[gpui::test]
fn panel_close_side_effects_are_deferred_outside_the_current_entity_lease(cx: &mut TestAppContext) {
    let entity = cx.update(|cx| cx.new(|_| 0_u8));
    let weak = entity.downgrade();

    entity.update(cx, |value, cx| {
        *value += 1;
        schedule_entity_update(weak, |value, _cx| *value += 1, cx);
    });
    cx.background_executor.run_until_parked();

    assert_eq!(cx.read(|cx| *entity.read(cx)), 2);
}

#[test]
fn panel_status_uses_typed_error_state() {
    assert!(!PanelStatus::Ready("ready".into()).is_error());
    assert!(!PanelStatus::Ready("ready".into()).should_render());
    assert!(!PanelStatus::Notice("copied".into()).is_error());
    assert!(PanelStatus::Notice("copied".into()).should_render());
    assert!(!PanelStatus::Pending("pending").is_error());
    assert!(PanelStatus::Pending("pending").should_render());
    assert!(PanelStatus::Error("failed".into()).is_error());
    assert!(PanelStatus::Error("failed".into()).should_render());
}

#[test]
fn external_foreground_window_hides_open_popup_but_paste_activation_does_not() {
    assert!(should_hide_for_foreground_change(
        dismiss_context(10, 0, false, true),
        20,
        200,
        "explorer.exe",
        false,
    ));
    assert!(!should_hide_for_foreground_change(
        dismiss_context(10, 0, true, true),
        20,
        200,
        "explorer.exe",
        false,
    ));
    assert!(!should_hide_for_foreground_change(
        dismiss_context(10, 0, false, true),
        10,
        200,
        "explorer.exe",
        false,
    ));
    assert!(!should_hide_for_foreground_change(
        dismiss_context(10, 0, false, false),
        20,
        200,
        "explorer.exe",
        false,
    ));
}

#[test]
fn transient_text_services_hosts_never_hide_the_panel() {
    let dismiss = dismiss_context(10, 0, false, true);
    assert!(!should_hide_for_foreground_change(
        dismiss,
        20,
        200,
        "textinputhost.exe",
        false,
    ));
    assert!(!should_hide_for_foreground_change(
        dismiss,
        20,
        200,
        "TextInputHost",
        false,
    ));
    assert!(!should_hide_for_foreground_change(
        dismiss, 20, 200, "ctfmon", false,
    ));
    assert!(!should_hide_for_foreground_change(
        dismiss,
        20,
        200,
        "C:\\Windows\\System32\\ctfmon.exe",
        false,
    ));
    assert!(!should_hide_for_foreground_change(
        dismiss,
        20,
        200,
        "TextInputHost.EXE",
        true,
    ));
    // The kept-open panel re-asserts topmost for transient hosts only.
    assert!(foreground_kept_panel_without_focus(
        dismiss,
        20,
        200,
        "textinputhost.exe"
    ));
    assert!(foreground_kept_panel_without_focus(
        dismiss,
        20,
        200,
        "TextInputHost"
    ));
    assert!(!foreground_kept_panel_without_focus(
        dismiss,
        20,
        200,
        "explorer.exe"
    ));
    assert!(!foreground_kept_panel_without_focus(
        dismiss_context(10, 0, true, true),
        20,
        200,
        "textinputhost.exe",
    ));
}

#[test]
fn popup_and_preview_are_exempt_but_other_windows_hide_after_opening_grace() {
    assert!(!should_hide_for_foreground_change(
        dismiss_context(10, 30, false, true),
        30,
        100,
        "explorer.exe",
        false,
    ));
    assert!(should_hide_for_foreground_change(
        dismiss_context(10, 30, false, true),
        40,
        100,
        "explorer.exe",
        false,
    ));
    assert!(!should_hide_for_foreground_change(
        dismiss_context(10, 30, false, true),
        20,
        200,
        "explorer.exe",
        true,
    ));
    assert!(should_hide_for_foreground_change(
        dismiss_context(10, 30, false, true),
        20,
        200,
        "explorer.exe",
        false,
    ));
}

#[test]
fn same_process_foreground_window_hides_open_popup_but_popup_preview_and_paste_do_not() {
    let open = dismiss_context(10, 30, false, true);
    assert!(should_hide_for_foreground_change(
        open,
        40,
        100,
        "nxtools-desktop",
        false,
    ));
    assert!(!should_hide_for_foreground_change(
        open,
        10,
        100,
        "nxtools-desktop",
        false,
    ));
    assert!(!should_hide_for_foreground_change(
        open,
        30,
        100,
        "nxtools-desktop",
        false,
    ));
    assert!(!should_hide_for_foreground_change(
        dismiss_context(10, 30, true, true),
        40,
        100,
        "nxtools-desktop",
        false,
    ));
}

#[test]
fn pointer_press_outside_the_panel_dismisses_it_even_in_the_app() {
    let open = dismiss_context(10, 30, false, true);
    assert!(should_hide_for_pointer_press(open, 20, 200));
    // The user report: clicking elsewhere inside this app left the panel
    // stuck. Same-process presses on other windows must dismiss too.
    assert!(should_hide_for_pointer_press(open, 40, 100));
    assert!(!should_hide_for_pointer_press(open, 10, 200,));
    assert!(!should_hide_for_pointer_press(open, 30, 200,));
    assert!(!should_hide_for_pointer_press(open, 0, 200,));
    assert!(!should_hide_for_pointer_press(
        dismiss_context(10, 30, true, true),
        20,
        200,
    ));
}

#[test]
fn pointer_press_sampled_before_popup_session_does_not_dismiss_new_popup() {
    let open = dismiss_context(10, 30, false, true);
    assert!(!should_hide_for_pointer_press_with_session(
        open,
        40,
        200,
        1_700_000_000_500,
        10,
        1_700_000_000_499,
        9,
    ));
    assert!(!should_hide_for_pointer_press_with_session(
        open,
        40,
        200,
        1_700_000_000_500,
        10,
        1_700_000_000_501,
        10,
    ));
    assert!(should_hide_for_pointer_press_with_session(
        open,
        40,
        200,
        1_700_000_000_500,
        10,
        1_700_000_000_501,
        11,
    ));
    assert!(!should_hide_for_pointer_press_with_session(
        open,
        10,
        200,
        1_700_000_000_500,
        10,
        1_700_000_000_501,
        11,
    ));
    assert!(!should_hide_for_pointer_press_with_session(
        open,
        30,
        200,
        1_700_000_000_500,
        10,
        1_700_000_000_501,
        11,
    ));
}

fn dismiss_context(
    popup_window_handle: i64,
    preview_window_handle: i64,
    paste_pending: bool,
    is_open: bool,
) -> PopupDismissContext {
    PopupDismissContext {
        popup_window_handle,
        preview_window_handle,
        current_process_id: 100,
        paste_pending,
        is_open,
    }
}

#[test]
fn popup_close_diagnostics_keep_reason_session_and_native_correlation_fields() {
    let clipboard_source = include_str!("../clipboard.rs");
    let ui_source = include_str!("mod.rs");
    for event_name in [
        "clipboard.popup.opened",
        "clipboard.popup.close_requested",
        "clipboard.popup.close_completed",
        "clipboard.popup.close_failed",
        "clipboard.popup.visibility_result_stale",
    ] {
        assert!(
            clipboard_source.contains(event_name),
            "missing {event_name}"
        );
    }
    for field in [
        "session_id",
        "command_id",
        "reason",
        "elapsed_ms",
        "native_error_code",
    ] {
        assert!(
            clipboard_source.contains(field),
            "missing diagnostic field {field}"
        );
    }
    for reason in [
        "PopupCloseReason::Foreground",
        "PopupCloseReason::Pointer",
        "PopupCloseReason::PasteSuccess",
        "PopupCloseReason::Escape",
    ] {
        assert!(ui_source.contains(reason), "missing close reason {reason}");
    }
}

#[test]
fn panel_metrics_match_the_reference_window() {
    let size = nxtools_core::ClipboardPanelSize::default();
    assert_eq!(size.width, 300);
    assert_eq!(size.height, 440);
    assert_eq!(PANEL_CONTENT_INSET, 8.0);
    assert_eq!(panel_list_width(size.width as f32), 284.0);
    assert_eq!(panel_list_width(240.0), 224.0);
    assert_eq!(panel_list_width(560.0), 544.0);
}

#[gpui::test]
fn clipboard_panel_keeps_300x440_logical_size_at_150_percent_dpi(cx: &mut TestAppContext) {
    let request = ClipboardPanelRequest {
        target: nxtools_protocol::ForegroundContext {
            window_handle: 1,
            focus_window_handle: 1,
            process_id: 1,
            process_name: "editor".into(),
            window_title: "document".into(),
        },
        cursor_x: 1_440,
        cursor_y: 720,
        cursor_display_index: 0,
        cursor_monitor_dpi: 144,
        requested_at_unix_ms: 1_000,
        event_sequence: 1,
    };
    let bounds =
        cx.update(|cx| panel_bounds(&request, nxtools_core::ClipboardPanelSize::default(), cx).0);
    assert_eq!(bounds.size, size(px(300.0), px(440.0)));
}

#[test]
fn clipboard_image_resources_are_bounded_and_recent_entries_are_retained() {
    let mut cache = BoundedImageResourceCache::new(3);
    assert_eq!(cache.touch(Path::new("one.png")), None);
    assert_eq!(cache.touch(Path::new("two.png")), None);
    assert_eq!(cache.touch(Path::new("three.png")), None);
    assert_eq!(cache.touch(Path::new("one.png")), None);
    assert_eq!(
        cache.touch(Path::new("four.png")),
        Some(PathBuf::from("two.png"))
    );
    assert_eq!(cache.len(), 3);
    assert!(cache.entries.contains(Path::new("one.png")));
    assert!(cache.entries.contains(Path::new("three.png")));
    assert!(cache.entries.contains(Path::new("four.png")));
}

#[test]
fn exactly_one_supported_modifier_selects_an_accelerated_action() {
    let actions = ClipboardModifierActions::default();
    assert_eq!(
        action_from_modifiers(
            gpui::Modifiers {
                control: true,
                ..Default::default()
            },
            actions
        ),
        Some(ClipboardAction::CopyOnly)
    );
    assert_eq!(
        action_from_modifiers(
            gpui::Modifiers {
                alt: true,
                ..Default::default()
            },
            actions
        ),
        Some(ClipboardAction::PlainText)
    );
    assert_eq!(
        action_from_modifiers(
            gpui::Modifiers {
                shift: true,
                ..Default::default()
            },
            actions
        ),
        Some(ClipboardAction::WithoutLineBreaks)
    );
    assert_eq!(
        action_from_modifiers(
            gpui::Modifiers {
                control: true,
                shift: true,
                ..Default::default()
            },
            actions
        ),
        None
    );
}

#[test]
fn configured_modifier_action_can_be_changed_or_disabled() {
    let actions = ClipboardModifierActions {
        control: ClipboardClickAction::PlainText,
        alt: ClipboardClickAction::Disabled,
        shift: ClipboardClickAction::OriginalPaste,
    };
    assert_eq!(
        action_from_modifiers(
            gpui::Modifiers {
                control: true,
                ..Default::default()
            },
            actions,
        ),
        Some(ClipboardAction::PlainText)
    );
    assert_eq!(
        action_from_modifiers(
            gpui::Modifiers {
                alt: true,
                ..Default::default()
            },
            actions,
        ),
        None
    );
}

#[test]
fn confirmed_clipboard_action_marks_the_item_used_before_the_panel_closes() {
    let source = include_str!("mod.rs");
    let handler = source
        .split("pub(crate) fn handle_command_result")
        .nth(1)
        .and_then(|source| source.split("pub(crate) fn handle_hide_failure").next())
        .expect("clipboard command-result handler source");

    assert!(handler.contains("mark_used_async(item_id"));
    assert!(handler.contains("cx.spawn"));
    assert!(handler.find("if result.success").is_some());
    assert!(handler.find("mark_used_async(item_id").is_some());
}

#[test]
fn panel_render_never_reads_the_parent_application_entity() {
    let source = include_str!("mod.rs");
    let render_start = source
        .find("impl Render for ClipboardPanel")
        .expect("clipboard panel render implementation");
    let render_end = source[render_start..]
        .find("fn panel_window_frame")
        .expect("end of clipboard panel render implementation");
    let render_source = &source[render_start..render_start + render_end];

    assert!(
        !render_source.contains("application.read(cx).clipboard_settings()"),
        "ClipboardPanel::render must use cached settings; reading ClipboardApplication while a notification owns it causes GPUI double_lease_panic"
    );
}

#[test]
fn category_wheel_is_horizontal_clamped_and_uses_vertical_wheel_input() {
    assert_eq!(
        resolve_category_wheel_offset(0.0, 120.0, 0.0, -48.0),
        CategoryWheelUpdate {
            offset: -48.0,
            changed: true
        }
    );
    assert_eq!(
        resolve_category_wheel_offset(-100.0, 120.0, 0.0, -48.0),
        CategoryWheelUpdate {
            offset: -120.0,
            changed: true
        }
    );
    assert_eq!(
        resolve_category_wheel_offset(-80.0, 120.0, 32.0, -10.0),
        CategoryWheelUpdate {
            offset: -48.0,
            changed: true
        }
    );
    assert_eq!(
        resolve_category_wheel_offset(-20.0, 0.0, 0.0, -48.0),
        CategoryWheelUpdate {
            offset: 0.0,
            changed: true
        }
    );
}

#[test]
fn category_drop_reorders_before_target_or_at_end() {
    let ids = vec!["all".to_owned(), "favorite".to_owned(), "work".to_owned()];
    assert_eq!(
        reordered_category_ids(&ids, "all", Some("work")),
        Some(vec!["favorite".into(), "all".into(), "work".into()])
    );
    assert_eq!(
        reordered_category_ids(&ids, "favorite", None),
        Some(vec!["all".into(), "work".into(), "favorite".into()])
    );
    assert_eq!(reordered_category_ids(&ids, "missing", None), None);
}

#[test]
fn clipboard_selection_wraps_and_empty_results_stay_unselected() {
    assert_eq!(next_clipboard_selection(None, 1, 0), None);
    assert_eq!(next_clipboard_selection(None, 1, 3), Some(0));
    assert_eq!(next_clipboard_selection(None, -1, 3), Some(2));
    assert_eq!(next_clipboard_selection(Some(0), -1, 0), None);
    assert_eq!(next_clipboard_selection(Some(0), -1, 3), Some(2));
    assert_eq!(next_clipboard_selection(Some(2), 1, 3), Some(0));
    assert_eq!(next_clipboard_selection(Some(1), 1, 3), Some(2));
}

#[test]
fn clipboard_search_dispatch_requires_the_latest_generation_and_query() {
    assert!(clipboard_search_is_current(4, 4, "ab", "ab"));
    assert!(!clipboard_search_is_current(3, 4, "ab", "ab"));
    assert!(!clipboard_search_is_current(4, 4, "a", "ab"));
    assert_eq!(
        CLIPBOARD_SEARCH_DEBOUNCE,
        std::time::Duration::from_millis(100)
    );
}

#[test]
fn clipboard_query_results_require_matching_generation_query_category_and_scope() {
    assert!(clipboard_query_state_is_current(
        4,
        4,
        "ab",
        "ab",
        BUILTIN_ALL_CATEGORY_ID,
        BUILTIN_ALL_CATEGORY_ID,
        ClipboardSearchScope::Content,
        ClipboardSearchScope::Content,
    ));
    assert!(!clipboard_query_state_is_current(
        3,
        4,
        "ab",
        "ab",
        BUILTIN_ALL_CATEGORY_ID,
        BUILTIN_ALL_CATEGORY_ID,
        ClipboardSearchScope::Content,
        ClipboardSearchScope::Content,
    ));
    assert!(!clipboard_query_state_is_current(
        4,
        4,
        "ab",
        "ab",
        BUILTIN_ALL_CATEGORY_ID,
        "custom-category",
        ClipboardSearchScope::Content,
        ClipboardSearchScope::Content,
    ));
    assert!(!clipboard_query_state_is_current(
        4,
        4,
        "ab",
        "ab",
        BUILTIN_ALL_CATEGORY_ID,
        BUILTIN_ALL_CATEGORY_ID,
        ClipboardSearchScope::Content,
        ClipboardSearchScope::Extended,
    ));
}

#[test]
fn clipboard_hover_action_rail_has_stable_geometry() {
    assert_eq!(HOVER_ACTION_RAIL_WIDTH, 112.0);
    let source = include_str!("history.rs");
    let rail_start = source
        .find(".w(px(HOVER_ACTION_RAIL_WIDTH))")
        .expect("clipboard hover action rail width");
    let rail_tail = &source[rail_start..];
    let rail_end = rail_tail
        .find(".when(!open_targets.is_empty()")
        .expect("clipboard hover action rail body");
    let rail_source = &rail_tail[..rail_end];
    assert!(rail_source.contains(".items_center()"));
    assert!(rail_source.contains(".justify_end()"));
    assert!(
        !rail_source.contains(".bg(theme.surface_hover)"),
        "the clipboard hover rail must stay transparent; only its buttons may paint hover feedback"
    );

    let button_width = 20.0_f32;
    let gap = 2.0_f32;
    for button_count in [3_usize, 4, 5] {
        let buttons_width =
            button_count as f32 * button_width + button_count.saturating_sub(1) as f32 * gap;
        let right_edge = HOVER_ACTION_RAIL_WIDTH;
        let left_edge = right_edge - buttons_width;
        assert!(left_edge >= 0.0);
        assert_eq!(left_edge + buttons_width, right_edge);
    }
}
