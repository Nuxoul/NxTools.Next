//! Tests for the clipboard panel module. Extracted from mod.rs so the
//! runtime code stays readable.

use super::*;
    use gpui::{ScrollDelta, ScrollWheelEvent, TestAppContext};
    use gpui_component::Root;
    use std::cell::Cell;

    #[test]
    fn history_count_belongs_to_the_header_not_the_search_row() {
        assert_eq!(clipboard_header_summary("全部", 134), "全部 · 134 条");
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
        let (_root, window) = cx.add_window_view(move |window, cx| {
            window.resize(size(px(520.0), px(440.0)));
            let panel = cx.new(|cx| {
                ClipboardPanel::new(
                    page,
                    categories,
                    ClipboardModifierActions::default(),
                    nxtools_protocol::ForegroundContext {
                        window_handle: 1,
                        focus_window_handle: 1,
                        process_id: 1,
                        process_name: "editor".into(),
                        window_title: "document".into(),
                    },
                    WeakEntity::new_invalid(),
                    window,
                    cx,
                )
            });
            Root::new(panel, window, cx)
        });

        window.refresh().expect("render clipboard empty state");
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
    }

    #[gpui::test]
    fn panel_shell_fills_the_native_window_without_a_uniform_transparent_gutter(
        cx: &mut TestAppContext,
    ) {
        struct PanelFrameFixture(Rc<Cell<u32>>);

        impl Render for PanelFrameFixture {
            fn render(
                &mut self,
                _window: &mut Window,
                _cx: &mut Context<Self>,
            ) -> impl IntoElement {
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
    fn inset_sections_leave_safe_edges_and_resize_stays_on_the_native_corner(
        cx: &mut TestAppContext,
    ) {
        struct SpacingFixture {
            content_presses: Rc<Cell<u32>>,
            resize_presses: Rc<Cell<u32>>,
        }

        impl Render for SpacingFixture {
            fn render(
                &mut self,
                _window: &mut Window,
                _cx: &mut Context<Self>,
            ) -> impl IntoElement {
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
            fn render(
                &mut self,
                _window: &mut Window,
                _cx: &mut Context<Self>,
            ) -> impl IntoElement {
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
    fn panel_close_side_effects_are_deferred_outside_the_current_entity_lease(
        cx: &mut TestAppContext,
    ) {
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
    fn internal_windows_and_the_opening_grace_period_do_not_hide_the_panel() {
        assert!(!should_hide_for_foreground_change(
            dismiss_context(10, 30, false, true),
            30,
            100,
            "explorer.exe",
            false,
        ));
        assert!(!should_hide_for_foreground_change(
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
    fn panel_metrics_match_the_reference_window() {
        let size = nxtools_core::ClipboardPanelSize::default();
        assert_eq!(size.width, 300);
        assert_eq!(size.height, 440);
        assert_eq!(PANEL_CONTENT_INSET, 8.0);
        assert_eq!(PANEL_SECTION_GAP, 6.0);
        assert_eq!(panel_list_width(size.width as f32), 284.0);
        assert_eq!(panel_list_width(240.0), 224.0);
        assert_eq!(panel_list_width(560.0), 544.0);
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

        assert!(handler.contains("application.mark_item_used(item_id)"));
        assert!(handler.find("if result.success").is_some());
        assert!(
            handler.find("application.mark_item_used(item_id)")
                < handler.find("self.close(window, cx)")
        );
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
