use super::*;

#[gpui_kit::test]
fn default_selection_keeps_first_displayed_identity_when_late_result_appends(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_kit::init);
    let first = search_result("first", ApplicationSource::IndexedFile);
    let (panel, window) = quick_launch_panel_fixture(cx, vec![first.clone()]);
    panel.update_in(window, |panel, _, cx| {
        panel.replace_results(
            vec![first, search_result("better", ApplicationSource::StartMenu)],
            cx,
        );
        assert_eq!(
            panel.selected, 0,
            "default selection keeps its published identity"
        );
        assert_eq!(panel.results[0].entry.id, "first");
    });
}

#[gpui_kit::test]
fn explicit_selection_survives_batches_including_an_explicit_first_row(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let first = search_result("first", ApplicationSource::IndexedFile);
    let second = search_result("second", ApplicationSource::IndexedFile);
    let (panel, window) = quick_launch_panel_fixture(cx, vec![first.clone(), second.clone()]);
    panel.update_in(window, |panel, window, cx| {
        panel.select_result(0, cx);
        panel.replace_results(vec![second.clone(), first.clone()], cx);
        assert_eq!(
            panel.selected, 1,
            "an explicit first-row choice is not a default"
        );
        panel.move_selection(-1, window, cx);
        panel.replace_results(vec![first, second], cx);
        assert_eq!(panel.selected, 1, "keyboard choice follows its identity");
    });
}

#[gpui_kit::test]
fn new_input_resets_default_before_first_current_batch(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (_temporary, application, _platform) = quick_launch_application_fixture(cx);
    let (indexer, endpoint) = crate::runtime::indexer_bridge(8);
    endpoint.connect_for_test();
    application.update(cx, |app, _| {
        app.indexer = Rc::new(indexer);
        app.index_status = Some(IndexStatus {
            phase: nxtools_protocol::IndexPhase::Ready,
            indexed_items: 1,
            drives: Vec::new(),
            last_error: None,
        });
    });
    let first = search_result("first", ApplicationSource::IndexedFile);
    let second = search_result("second", ApplicationSource::IndexedFile);
    let (panel, window) = quick_launch_panel_fixture(cx, vec![first.clone(), second.clone()]);
    panel.update_in(window, |panel, window, cx| {
        panel.application = application.downgrade();
        panel.select_result(1, cx);
        panel.sync_window_content_size("initial", window, cx);
        panel.focus_search(window, cx);
    });
    panel.update_in(window, |panel, window, cx| {
        panel
            .search_input
            .update(cx, |input, cx| input.replace_all("new-query", window, cx));
    });
    window.run_until_parked();
    assert!(matches!(
        endpoint
            .try_recv_command_for_test()
            .expect("current index request")
            .message,
        nxtools_protocol::Message::IndexerCommand(IndexerCommand::Search(_))
    ));
    panel.read_with(window, |panel, _| {
        assert!(panel.search_pending);
        assert_eq!(panel.selected, 0, "typing resets the default immediately");
        assert_eq!(
            panel.content_height(),
            PANEL_HEIGHT,
            "retained rows keep the expanded viewport while pending"
        );
    });
    panel.update_in(window, |panel, _, cx| {
        panel.replace_results(vec![second, first], cx);
        assert_eq!(
            panel.selected, 0,
            "old-query identity cannot select the new batch"
        );
    });
}

#[gpui_kit::test]
fn retained_rows_drop_selection_paint_while_execution_stays_blocked(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (panel, window) = quick_launch_panel_fixture(
        cx,
        vec![search_result("retained", ApplicationSource::IndexedFile)],
    );
    window.refresh().expect("paint current result");
    let selected_color = panel.read_with(window, |panel, _| panel.theme.element_selected);
    assert!(window.update(|window, _| {
        window
            .painted_quads()
            .iter()
            .any(|quad| quad.background == gpui_kit::solid_background(selected_color))
    }));
    panel.update_in(window, |panel, _, cx| {
        panel.schedule_search("next".into(), cx);
        cx.notify();
    });
    window
        .refresh()
        .expect("paint retained row during query work");
    assert!(
        !window.update(|window, _| window
            .painted_quads()
            .iter()
            .any(|quad| { quad.background == gpui_kit::solid_background(selected_color) })),
        "pending input must not paint retained results as actionable selections"
    );
    panel.update_in(window, |panel, window, cx| {
        panel.move_selection(1, window, cx);
        assert_eq!(
            panel.selected, 0,
            "pending navigation cannot select an old target"
        );
        panel.enter_action_mode(0, window, cx);
        assert!(
            panel.action_mode.is_none(),
            "retained paint does not grant execution"
        );
    });
}

#[gpui_kit::test]
fn application_title_is_single_line_and_file_subtitle_is_parent_path(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let mut application = search_result("Editor", ApplicationSource::StartMenu);
    application.entry.target = r"C:\Start Menu\Editor.lnk".into();
    let mut file = search_result("report.txt", ApplicationSource::IndexedFile);
    file.entry.target = r"C:\Documents\Reports\report.txt".into();
    let (panel, window) = quick_launch_panel_fixture(cx, vec![application, file]);
    panel.read_with(window, |panel, _| {
        assert!(panel.results.display(0).subtitle.is_empty());
        assert_eq!(
            panel.results.display(1).subtitle.as_ref(),
            r"C:\Documents\Reports"
        );
        assert!(panel.results.display(1).accessible.contains("report.txt"));
    });
    window.refresh().expect("paint semantic result rows");
    assert!(
        window
            .debug_bounds("quick-launch-result-subtitle-0")
            .is_none()
    );
    assert!(
        window
            .debug_bounds("quick-launch-result-subtitle-1")
            .is_some()
    );
    let title = window
        .debug_bounds("quick-launch-result-title-0")
        .expect("application title");
    let row = window
        .debug_bounds("quick-launch-result-row-0")
        .expect("application row");
    assert!((title.center().y - row.center().y).abs() <= px(1.0));
}

#[test]
fn display_projection_preserves_explicit_subtitles_and_root_paths() {
    let mut explicit = search_result("command", ApplicationSource::CustomCommand);
    explicit.subtitle = Some("--profile work".into());
    let mut root = search_result("C:", ApplicationSource::IndexedDirectory);
    root.entry.target = "C:\\".into();
    let mut file = search_result("note.txt", ApplicationSource::IndexedFile);
    file.entry.target = "C:\\note.txt".into();
    let rows: QuickLaunchResults = vec![explicit, root, file].into();
    assert_eq!(rows.display(0).subtitle.as_ref(), "--profile work");
    assert_eq!(rows.display(1).subtitle.as_ref(), "C:\\");
    assert_eq!(rows.display(2).subtitle.as_ref(), "C:\\");
}

#[gpui_kit::test]
fn published_result_count_keeps_the_expanded_panel_and_header_fixed(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (panel, window) = quick_launch_panel_fixture(cx, Vec::new());
    let input = panel.read_with(window, |panel, _| panel.search_input.entity_id());
    panel.update_in(window, |panel, window, cx| panel.focus_search(window, cx));
    for count in [0, 1, 3, 200] {
        panel.update_in(window, |panel, _, cx| {
            panel.replace_results(
                (0..count)
                    .map(|index| {
                        search_result(&format!("result-{index}"), ApplicationSource::IndexedFile)
                    })
                    .collect(),
                cx,
            );
        });
        window.run_until_parked();
        let applied_size = window.update(|window, _| window.bounds().size);
        window.simulate_resize(applied_size);
        window.refresh().expect("paint published content geometry");
        window.update(|window, _| {
            assert_eq!(window.bounds().size, size(px(620.0), px(PANEL_HEIGHT)));
        });
        assert_eq!(
            window
                .debug_bounds("quick-launch-search-header")
                .expect("fixed header")
                .size
                .height,
            px(58.0)
        );
        let surface = window
            .debug_bounds("quick-launch-results-surface")
            .expect("original results surface");
        assert!(
            surface.size.height <= px(470.0),
            "count={count}, results surface={surface:?}"
        );
        if count > 0 {
            let row = window
                .debug_bounds("quick-launch-result-row-0")
                .expect("first result fits");
            assert_eq!(row.size.height, px(52.0));
            assert!(row.bottom() <= surface.bottom());
            assert_eq!(
                window
                    .debug_bounds("quick-launch-result-icon-0")
                    .expect("normal icon")
                    .size,
                size(px(32.0), px(32.0))
            );
        }
        panel.update_in(window, |panel, window, cx| {
            assert_eq!(panel.search_input.entity_id(), input);
            assert!(panel.search_input.focus_handle(cx).is_focused(window));
            if count == 200 {
                panel.move_selection(-1, window, cx);
            }
        });
        if count == 200 {
            window.run_until_parked();
            window.refresh().expect("scroll overflow to final result");
            let last = window
                .debug_bounds("quick-launch-result-row-199")
                .expect("last result reachable");
            assert!(last.top() >= surface.top() && last.bottom() <= surface.bottom());
        }
    }
    panel.update_in(window, |panel, _, cx| {
        panel.set_error("索引服务连接失败".into(), cx)
    });
    window.run_until_parked();
    let applied_size = window.update(|window, _| window.bounds().size);
    window.simulate_resize(applied_size);
    window.refresh().expect("footer below capped results");
    window.update(|window, _| assert_eq!(window.bounds().size.height, px(PANEL_HEIGHT)));
    let capped = window
        .debug_bounds("quick-launch-results-surface")
        .expect("capped viewport");
    let status = window
        .debug_bounds("quick-launch-status-footer")
        .expect("reserved footer");
    assert_eq!(
        capped.size.height,
        px(LAUNCHER_LIST_MAX_HEIGHT - LAUNCHER_STATUS_FOOTER_HEIGHT)
    );
    assert!(status.top() >= capped.bottom() - px(1.0));
    panel.update_in(window, |panel, _, cx| {
        panel.replace_results(Vec::new(), cx);
        panel.set_error("索引服务连接失败".into(), cx);
    });
    window.run_until_parked();
    let applied_size = window.update(|window, _| window.bounds().size);
    window.simulate_resize(applied_size);
    window
        .refresh()
        .expect("compact error and reachable footer");
    let footer = window
        .debug_bounds("quick-launch-status-footer")
        .expect("status footer");
    let surface = window
        .debug_bounds("quick-launch-results-surface")
        .expect("error surface");
    assert!(footer.top() >= surface.bottom() - px(1.0));
    let root = window.debug_bounds("quick-launch-window").expect("root");
    assert!(footer.bottom() <= root.bottom());
}

#[gpui_kit::test]
fn result_publication_cannot_resize_a_hidden_or_new_session(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (panel, window) = quick_launch_panel_fixture(cx, Vec::new());
    panel.update_in(window, |panel, window, cx| {
        panel.replace_results(
            vec![search_result("late", ApplicationSource::IndexedFile)],
            cx,
        );
        panel.prepare_for_hide(window, cx);
    });
    window.run_until_parked();
    window.update(|window, _| assert_eq!(window.bounds().size.height, px(60.0)));
    panel.update_in(window, |panel, window, cx| {
        panel.is_open = true;
        panel.results_visible = false;
        panel.query_dispatch_generation += 1;
        panel.sync_window_content_size("", window, cx);
        cx.notify();
    });
    window.run_until_parked();
    window.update(|window, _| assert_eq!(window.bounds().size.height, px(60.0)));
}

#[gpui_kit::test]
fn unicode_path_middle_ellipsis_uses_measured_width_without_changing_full_text(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_kit::init);
    let mut file = search_result("报告.txt", ApplicationSource::IndexedFile);
    file.entry.target = format!(r"C:\Users\{}\项目🙂\报告.txt", "资料目录é".repeat(30));
    let full_target = file.entry.target.clone();
    let (panel, window) = quick_launch_panel_fixture(cx, vec![file]);
    let subtitle = panel.read_with(window, |panel, _| panel.results.display(0).subtitle.clone());
    for width in [620.0, 480.0, 620.0] {
        window.simulate_resize(size(px(width), px(132.0)));
        window
            .refresh()
            .expect("measure Unicode path in resized rail");
        let copy = window
            .debug_bounds("quick-launch-result-copy-0")
            .expect("text rail");
        let path = window
            .debug_bounds("quick-launch-result-subtitle-0")
            .expect("path rail");
        assert!(path.left() >= copy.left() && path.right() <= copy.right());
        assert!(window.update(|window, _| {
            gpui_kit::base::test_support::snapshots(window)
                .iter()
                .any(|node| {
                    node.label()
                        .is_some_and(|label| label.contains(full_target.as_str()))
                })
        }));
        window.update(|_window, cx| {
            let font = gpui_kit::font(UI_FONT_FAMILY);
            let run = gpui_kit::TextRun {
                len: subtitle.len(),
                font: font.clone(),
                ..Default::default()
            };
            let mut wrapper = cx.text_system().line_wrapper(font, px(TEXT_BODY));
            let runs = [run];
            let (trimmed, _) = wrapper.truncate_line(
                subtitle.clone(),
                path.size.width,
                "…",
                &runs,
                gpui_kit::TruncateFrom::Middle,
            );
            assert!(trimmed.contains('…'));
            assert!(trimmed.starts_with(r"C:\Users\"));
            assert!(trimmed.ends_with("项目🙂"));
            let (untrimmed, _) = wrapper.truncate_line(
                subtitle.clone(),
                px(20_000.0),
                "…",
                &runs,
                gpui_kit::TruncateFrom::Middle,
            );
            assert_eq!(untrimmed, subtitle, "wide layout restores full subtitle");
        });
        panel.read_with(window, |panel, _| {
            assert_eq!(panel.results[0].entry.target, full_target);
            assert_eq!(
                panel.results.display(0).subtitle.as_ref(),
                subtitle.as_ref()
            );
            assert_eq!(
                panel.results.display(0).subtitle.as_ptr(),
                subtitle.as_ptr()
            );
        });
    }
    let source = include_str!("panel.rs");
    let result = source
        .split("pub(crate) fn render_result(")
        .nth(1)
        .expect("result renderer")
        .split("pub(crate) fn render_action(")
        .next()
        .expect("result boundary");
    assert!(
        result.contains(".text_ellipsis_middle()"),
        "actual subtitle must use released measured middle truncation"
    );
    assert!(result.contains("Label::new(subtitle)"));
}

#[test]
fn launcher_density_keeps_both_external_gap_choices_and_compact_attached_metrics() {
    use nxtools_core::QuickLaunchRowStyle::{Continuous, Separated};
    assert_eq!(LAUNCHER_RESULT_ROW_HEIGHT + Separated.gap(), 56.0);
    assert_eq!(LAUNCHER_RESULT_ROW_HEIGHT + Continuous.gap(), 52.0);
    assert_eq!(COMPACT_RESULT_ROW_HEIGHT, 40.0);
    assert_eq!(COMPACT_RESULT_ICON_RAIL_WIDTH, 26.0);
}

#[test]
fn preview_docking_uses_the_primary_edges_at_every_content_height() {
    let screen = Bounds::new(point(px(0.0), px(0.0)), size(px(1920.0), px(1080.0)));
    let preview_size = size(px(400.0), px(410.0));
    for height in [60.0, 132.0, 244.0, 530.0] {
        let primary = Bounds::new(point(px(300.0), px(200.0)), size(px(620.0), px(height)));
        let preview = quick_launch_preview_bounds(primary, preview_size, screen);
        assert_eq!(
            preview.left(),
            primary.right() + px(QUICK_LAUNCH_PREVIEW_GAP)
        );
        assert_eq!(preview.top(), primary.top());
        assert!(preview.right() <= screen.right() && preview.bottom() <= screen.bottom());
    }
}

#[gpui_kit::test]
fn native_content_resize_coalesces_publications_without_activation_or_closed_session_replay(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_kit::init);
    let (_temp, application, endpoint) = quick_launch_application_fixture(cx);
    let (panel, window) = quick_launch_panel_fixture(
        cx,
        vec![search_result("first", ApplicationSource::IndexedFile)],
    );
    let handle = window.update(|window, _| window.window_handle());
    let scale = window.update(|window, _| window.scale_factor());
    window.simulate_resize(size(px(PANEL_WIDTH), px(COLLAPSED_PANEL_HEIGHT)));
    application.update(window, |application, cx| {
        application.popup = Some(QuickLaunchWindow {
            window: handle,
            panel: panel.downgrade(),
            native_window_handle: 900,
        });
        application.popup_origin_device_pixels = Some((80, 120));
        application.resize_popup_content(cx);
    });
    let first = endpoint
        .try_receive_command_for_contract_test()
        .expect("native content resize");
    assert!(
        matches!(&first.message, nxtools_protocol::Message::PlatformCommand(PlatformCommand::SetWindowBounds(bounds)) if bounds.window_handle == 900 && bounds.x == 80 && bounds.y == 120 && bounds.width == (PANEL_WIDTH * scale) as i32 && bounds.height == (PANEL_HEIGHT * scale) as i32)
    );
    panel.update_in(window, |panel, _, _| {
        panel.results = (0..3)
            .map(|index| search_result(&format!("next-{index}"), ApplicationSource::IndexedFile))
            .collect::<Vec<_>>()
            .into();
    });
    application.update(window, |application, cx| {
        application.resize_popup_content(cx)
    });
    assert!(
        endpoint.try_receive_command_for_contract_test().is_err(),
        "one in-flight native resize"
    );
    window.simulate_resize(size(px(PANEL_WIDTH), px(PANEL_HEIGHT)));
    application.update(window, |application, cx| {
        application.handle_command_result(
            first.id,
            PlatformCommandResult {
                success: true,
                error: None,
                native_error_code: None,
                data: None,
            },
            cx,
        )
    });
    assert!(
        endpoint.try_receive_command_for_contract_test().is_err(),
        "counts never resize an expanded panel"
    );
    panel.update_in(window, |panel, _, _| panel.results_visible = false);
    application.update(window, |application, cx| {
        application.resize_popup_content(cx)
    });
    let current = endpoint
        .try_receive_command_for_contract_test()
        .expect("clear input collapses");
    assert!(
        matches!(&current.message, nxtools_protocol::Message::PlatformCommand(PlatformCommand::SetWindowBounds(bounds)) if bounds.height == (COLLAPSED_PANEL_HEIGHT * scale) as i32)
    );
    panel.update_in(window, |panel, window, cx| {
        panel.prepare_for_hide(window, cx)
    });
    application.update(window, |application, cx| {
        application.handle_command_result(
            current.id,
            PlatformCommandResult {
                success: true,
                error: None,
                native_error_code: None,
                data: None,
            },
            cx,
        )
    });
    assert!(
        endpoint.try_receive_command_for_contract_test().is_err(),
        "closed session cannot resize or activate"
    );
    application.read_with(window, |application, _| {
        assert!(application.pending_popup_resize.is_none())
    });
}

#[gpui_kit::test]
fn native_query_resize_does_not_contract_for_an_intermediate_result_batch(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (_temp, application, endpoint) = quick_launch_application_fixture(cx);
    let (panel, window) = quick_launch_panel_fixture(cx, Vec::new());
    panel.update_in(window, |panel, window, cx| {
        panel
            .search_input
            .update(cx, |input, cx| input.set_value("shape", window, cx));
    });
    window.run_until_parked();
    panel.update_in(window, |panel, _, cx| {
        // Control provider publication independently of the input coalescer.
        panel.query_dispatch_generation += 1;
        panel.replace_results(
            vec![search_result("shape-app", ApplicationSource::StartMenu)],
            cx,
        );
    });
    window.run_until_parked();
    window.simulate_resize(size(px(PANEL_WIDTH), px(PANEL_HEIGHT)));
    let handle = window.update(|window, _| window.window_handle());
    application.update(window, |application, cx| {
        application.popup = Some(QuickLaunchWindow {
            window: handle,
            panel: panel.downgrade(),
            native_window_handle: 900,
        });
        application.popup_origin_device_pixels = Some((80, 120));
        application
            .index_search
            .begin("shape", INDEX_CANDIDATE_LIMIT);
        application.resize_popup_content(cx);
    });
    panel.read_with(window, |panel, _| {
        assert!(
            !panel.search_pending,
            "current local result stays actionable"
        );
        assert_eq!(panel.results.len(), 1);
    });
    assert!(
        endpoint.try_receive_command_for_contract_test().is_err(),
        "a partial local batch must not shrink the native result viewport"
    );
    panel.update_in(window, |panel, _, cx| {
        panel.replace_results(
            (0..20)
                .map(|index| {
                    search_result(&format!("shape-{index}"), ApplicationSource::IndexedFile)
                })
                .collect(),
            cx,
        );
    });
    application.update(window, |application, cx| {
        application.index_search.complete = true;
        application.resize_popup_content(cx);
    });
    assert!(
        endpoint.try_receive_command_for_contract_test().is_err(),
        "the final large batch must not create a shrink/grow pair"
    );
    window.simulate_resize(size(px(PANEL_WIDTH), px(132.0)));
    application.update(window, |application, cx| {
        application.index_search.complete = false;
        application.resize_popup_content(cx);
    });
    let scale = window.update(|window, _| window.scale_factor());
    let expanded = endpoint
        .try_receive_command_for_contract_test()
        .expect("typed search restores the fixed viewport without waiting for providers");
    assert!(
        matches!(expanded.message, nxtools_protocol::Message::PlatformCommand(PlatformCommand::SetWindowBounds(bounds)) if bounds.height == (PANEL_HEIGHT * scale) as i32)
    );
}

#[gpui_kit::test]
fn native_query_completion_keeps_fixed_geometry_without_a_shrink_timer(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (_temp, application, endpoint) = quick_launch_application_fixture(cx);
    let (panel, window) = quick_launch_panel_fixture(cx, Vec::new());
    panel.update_in(window, |panel, window, cx| {
        panel
            .search_input
            .update(cx, |input, cx| input.set_value("shape", window, cx));
    });
    window.run_until_parked();
    panel.update_in(window, |panel, _, cx| {
        panel.query_dispatch_generation += 1;
        panel.replace_results(
            vec![search_result("shape", ApplicationSource::IndexedFile)],
            cx,
        );
    });
    window.run_until_parked();
    window.simulate_resize(size(px(PANEL_WIDTH), px(PANEL_HEIGHT)));
    let handle = window.update(|window, _| window.window_handle());
    application.update(window, |application, cx| {
        application.popup = Some(QuickLaunchWindow {
            window: handle,
            panel: panel.downgrade(),
            native_window_handle: 901,
        });
        application.popup_origin_device_pixels = Some((80, 120));
        application
            .index_search
            .begin("shape", INDEX_CANDIDATE_LIMIT);
        application.index_search.complete = true;
        application.resize_popup_content(cx);
    });
    assert!(
        endpoint.try_receive_command_for_contract_test().is_err(),
        "completed small result sets keep the stable expanded viewport"
    );
}

#[gpui_kit::test]
fn listary_search_header_matches_result_scale_without_replacing_input(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (panel, window) = quick_launch_panel_fixture(cx, Vec::new());
    window.simulate_resize(size(px(620.0), px(60.0)));
    let input = panel.read_with(window, |panel, _| panel.search_input.clone());
    panel.update_in(window, |panel, window, cx| panel.focus_search(window, cx));
    window.refresh().expect("paint reference-sized input");
    let header = window
        .debug_bounds("quick-launch-search-header")
        .expect("header");
    let group = window
        .debug_bounds("quick-launch-search-group")
        .expect("input frame");
    assert_eq!(header.size.height, px(58.0));
    assert_eq!(group.size.height, px(44.0));
    assert!(group.top() >= header.top() && group.bottom() <= header.bottom());
    window.simulate_input("搜索示例 Editor");
    window.run_until_parked();
    window.refresh().expect("paint input text");
    panel.read_with(window, |panel, cx| {
        assert_eq!(panel.search_input.entity_id(), input.entity_id());
        assert_eq!(input.read(cx).value().as_ref(), "搜索示例 Editor");
    });
    window.update(|window, cx| {
        assert!(input.focus_handle(cx).is_focused(window));
        let text = input.read(cx).input_bounds();
        assert!(
            text.size.height >= px(34.0),
            "input glyph lane is not the former tiny body text"
        );
        assert!(text.top() >= group.top() && text.bottom() <= group.bottom());
    });
}

#[gpui_kit::test]
fn native_typed_query_expands_without_waiting_for_rows_or_providers(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (_temporary, application, endpoint) = quick_launch_application_fixture(cx);
    let (panel, window) = quick_launch_panel_fixture(cx, Vec::new());
    panel.update_in(window, |panel, window, cx| {
        panel
            .search_input
            .update(cx, |input, cx| input.set_value("shape", window, cx));
    });
    window.run_until_parked();
    panel.update_in(window, |panel, _, _| {
        panel.query_dispatch_generation += 1;
        panel.search_pending = true;
    });
    window.simulate_resize(size(px(PANEL_WIDTH), px(COLLAPSED_PANEL_HEIGHT)));
    let handle = window.update(|window, _| window.window_handle());
    let scale = window.update(|window, _| window.scale_factor());
    application.update(window, |application, cx| {
        application.popup = Some(QuickLaunchWindow {
            window: handle,
            panel: panel.downgrade(),
            native_window_handle: 902,
        });
        application.popup_origin_device_pixels = Some((80, 120));
        application
            .index_search
            .begin("shape", INDEX_CANDIDATE_LIMIT);
        application.resize_popup_content(cx);
    });
    let command = endpoint
        .try_receive_command_for_contract_test()
        .expect("typed query opens the fixed viewport before delayed results");
    assert!(
        matches!(command.message, nxtools_protocol::Message::PlatformCommand(PlatformCommand::SetWindowBounds(bounds)) if bounds.height == (PANEL_HEIGHT * scale) as i32)
    );
    window.simulate_resize(size(px(PANEL_WIDTH), px(PANEL_HEIGHT)));
    panel.update_in(window, |panel, _, _| {
        panel.results = vec![search_result("shape", ApplicationSource::StartMenu)].into();
        panel.search_pending = false;
    });
    application.update(window, |application, cx| {
        application.handle_command_result(
            command.id,
            PlatformCommandResult {
                success: true,
                error: None,
                native_error_code: None,
                data: None,
            },
            cx,
        );
    });
    assert!(
        endpoint.try_receive_command_for_contract_test().is_err(),
        "first current rows do not resize the established viewport"
    );
}

#[gpui_kit::test]
fn quick_launch_result_tween_stable_expanded_viewport_admits_late_rows(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (_temporary, application, endpoint) = quick_launch_application_fixture(cx);
    let (panel, window) = quick_launch_panel_fixture(cx, Vec::new());
    panel.update_in(window, |panel, window, cx| {
        panel
            .search_input
            .update(cx, |input, cx| input.set_value("shape", window, cx));
    });
    window.run_until_parked();
    panel.update_in(window, |panel, _, cx| {
        panel.query_dispatch_generation += 1;
        panel.replace_results(
            vec![search_result("local", ApplicationSource::StartMenu)],
            cx,
        );
    });
    window.simulate_resize(size(px(PANEL_WIDTH), px(COLLAPSED_PANEL_HEIGHT)));
    let handle = window.update(|window, _| window.window_handle());
    let scale = window.update(|window, _| window.scale_factor());
    application.update(window, |application, cx| {
        application.popup = Some(QuickLaunchWindow {
            window: handle,
            panel: panel.downgrade(),
            native_window_handle: 912,
        });
        application.popup_origin_device_pixels = Some((80, 120));
        application
            .index_search
            .begin("shape", INDEX_CANDIDATE_LIMIT);
        application.resize_popup_content(cx);
    });
    let first = endpoint
        .try_receive_command_for_contract_test()
        .expect("first current result opens the stable viewport immediately");
    assert!(
        matches!(first.message, nxtools_protocol::Message::PlatformCommand(PlatformCommand::SetWindowBounds(bounds)) if bounds.height == (PANEL_HEIGHT * scale) as i32)
    );
    window.simulate_resize(size(px(PANEL_WIDTH), px(PANEL_HEIGHT)));
    panel.update_in(window, |panel, _, cx| {
        panel.replace_results(
            (0..20)
                .map(|index| {
                    search_result(&format!("late-{index}"), ApplicationSource::IndexedFile)
                })
                .collect(),
            cx,
        );
        assert!(
            !panel.search_pending,
            "late rows are actionable immediately"
        );
    });
    application.update(window, |application, cx| {
        application.index_search.complete = true;
        application.handle_command_result(
            first.id,
            PlatformCommandResult {
                success: true,
                error: None,
                native_error_code: None,
                data: None,
            },
            cx,
        );
    });
    assert!(
        endpoint.try_receive_command_for_contract_test().is_err(),
        "a late result batch must not request another native height"
    );
}

#[gpui_kit::test]
fn quick_launch_result_tween_content_height_does_not_follow_ordinary_result_counts(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_kit::init);
    let (panel, window) = quick_launch_panel_fixture(cx, Vec::new());
    panel.update_in(window, |panel, window, cx| {
        panel.sync_window_content_size("shape", window, cx);
        panel.replace_results(
            vec![search_result("first", ApplicationSource::StartMenu)],
            cx,
        );
        assert_eq!(panel.content_height(), PANEL_HEIGHT);
        panel.replace_results(
            (0..3)
                .map(|index| {
                    search_result(&format!("result-{index}"), ApplicationSource::IndexedFile)
                })
                .collect(),
            cx,
        );
        assert_eq!(panel.content_height(), PANEL_HEIGHT);
        panel.recent_directory_mode = true;
        assert_eq!(
            panel.content_height(),
            COLLAPSED_PANEL_HEIGHT
                + 2.0 * SPACING_8
                + 3.0 * (LAUNCHER_RESULT_ROW_HEIGHT + panel.row_style.gap()),
            "recent directories retain their content geometry"
        );
        panel.recent_directory_mode = false;
        panel.action_mode = Some(QuickLaunchActionMode::enter(
            search_result("first", ApplicationSource::StartMenu),
            "shape".into(),
            0,
        ));
        assert!(
            panel.content_height() > COLLAPSED_PANEL_HEIGHT
                && panel.content_height() < PANEL_HEIGHT,
            "pending Shell actions retain their content geometry"
        );
        panel.action_mode = None;
        panel.sync_window_content_size("", window, cx);
        assert_eq!(panel.content_height(), COLLAPSED_PANEL_HEIGHT);
    });
}

#[gpui_kit::test]
fn quick_launch_result_tween_production_admission_delayed_rows_and_nonreplay(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_kit::init);
    cx.update(|cx| {
        cx.set_reduce_motion(false);
        cx.set_global(crate::ui::motion::UiMotionPreference::Known {
            reduced_motion: false,
        });
    });
    let (panel, window) = quick_launch_panel_fixture(cx, Vec::new());
    panel.update_in(window, |panel, window, cx| {
        panel.query_dispatch_generation += 1;
        panel.sync_window_content_size("shape", window, cx);
        panel.replace_results(
            vec![search_result("early", ApplicationSource::StartMenu)],
            cx,
        );
        panel.present_result_entrances(window, cx);
        let first = panel
            .result_entrances
            .sample("early", cx.background_executor().now());
        assert_eq!(first.translation.y, px(8.0));
        assert_eq!(first.opacity, 0.0);
    });
    window.update(|window, cx| window.draw(cx).clear(cx));
    assert!(window.update(|window, cx| window.simulate_next_frame(cx)) > 0);
    window.executor().advance_clock(RESULTS_APPEAR_DURATION);
    window.update(|window, cx| window.draw(cx).clear(cx));
    panel.update_in(window, |panel, window, cx| {
        assert!(!panel.result_entrances.is_active());
        panel.replace_results(
            vec![
                search_result("late", ApplicationSource::IndexedFile),
                search_result("early", ApplicationSource::StartMenu),
            ],
            cx,
        );
        panel.present_result_entrances(window, cx);
        let now = cx.background_executor().now();
        assert_eq!(panel.result_entrances.sample("late", now).opacity, 0.0);
        assert_eq!(panel.result_entrances.sample("early", now).opacity, 1.0);
        panel.select_result(1, cx);
        panel.set_icons([("early".into(), PathBuf::from("fixture-icon.png"))], cx);
        panel.replace_results(
            vec![
                search_result("early", ApplicationSource::StartMenu),
                search_result("late", ApplicationSource::IndexedFile),
            ],
            cx,
        );
        assert_eq!(panel.selected, 0, "selection still follows identity");
        assert_eq!(panel.result_entrances.sample("early", now).opacity, 1.0);
    });
    window.executor().advance_clock(RESULTS_APPEAR_DURATION);
    window.update(|window, cx| window.draw(cx).clear(cx));
    panel.update_in(window, |panel, window, cx| {
        panel.replace_results(
            (0..200)
                .map(|index| {
                    search_result(&format!("scroll-{index}"), ApplicationSource::IndexedFile)
                })
                .collect(),
            cx,
        );
        panel.present_result_entrances(window, cx);
        let now = cx.background_executor().now();
        assert_eq!(
            panel.result_entrances.sample("scroll-199", now).opacity,
            1.0
        );
        panel.update_presented_results(190..200, cx);
        panel.update_presented_results(0..9, cx);
        assert_eq!(
            panel.result_entrances.sample("scroll-199", now).opacity,
            1.0,
            "virtual scrolling never admits an old identity again"
        );
    });
}

#[gpui_kit::test]
fn quick_launch_result_tween_production_reduced_unknown_disable_hide_close_and_disconnect(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_kit::init);
    let (_directory, application, _endpoint) = quick_launch_application_fixture(cx);
    let (panel, window) = quick_launch_panel_fixture(cx, Vec::new());
    let handle = window.update(|window, _| window.window_handle());
    application.update(window, |application, _| {
        application.popup = Some(QuickLaunchWindow {
            window: handle,
            panel: panel.downgrade(),
            native_window_handle: 914,
        })
    });
    for category in [
        "unknown",
        "reduced",
        "disabled",
        "hide",
        "close",
        "disconnected",
        "new_query",
    ] {
        window.update(|_, cx| {
            cx.set_reduce_motion(false);
            cx.set_global(crate::ui::motion::UiMotionPreference::Known {
                reduced_motion: false,
            });
        });
        window.simulate_resize(size(px(PANEL_WIDTH), px(PANEL_HEIGHT)));
        panel.update_in(window, |panel, window, cx| {
            panel.is_open = true;
            panel.results_visible = true;
            panel.result_motion_available = true;
            panel.result_entrance = nxtools_core::QuickLaunchResultEntrance::SlideUp;
            panel.query_dispatch_generation += 1;
            panel.replace_results(
                vec![search_result(category, ApplicationSource::StartMenu)],
                cx,
            );
            panel.present_result_entrances(window, cx);
            assert!(panel.result_entrances.is_active());
        });
        match category {
            "unknown" => window.update(|_, cx| {
                cx.set_reduce_motion(true);
                cx.set_global(crate::ui::motion::UiMotionPreference::Unknown);
            }),
            "reduced" => window.update(|_, cx| {
                cx.set_reduce_motion(true);
                cx.set_global(crate::ui::motion::UiMotionPreference::Known {
                    reduced_motion: true,
                });
            }),
            "disabled" => panel.update_in(window, |panel, _, _| {
                panel.result_entrance = nxtools_core::QuickLaunchResultEntrance::None
            }),
            "hide" => panel.update_in(window, |panel, window, cx| {
                panel.prepare_for_hide(window, cx)
            }),
            "close" => panel.update_in(window, |panel, _, cx| panel.close(cx)),
            "new_query" => panel.update_in(window, |panel, _, cx| {
                panel.query_dispatch_generation += 1;
                panel.schedule_search("next".into(), cx);
            }),
            _ => application.update(window, |application, cx| {
                application.handle_platform_notification(
                    PlatformNotification::Disconnected {
                        generation: 4,
                        pending_command_ids: Vec::new(),
                    },
                    cx,
                )
            }),
        }
        window.update(|window, cx| window.draw(cx).clear(cx));
        panel.read_with(window, |panel, cx| {
            assert!(!panel.result_entrances.is_active(), "{category}");
            assert_eq!(
                panel
                    .result_entrances
                    .sample(category, cx.background_executor().now())
                    .opacity,
                1.0
            );
        });
        // Drain the one already-requested frame; suppression cannot schedule another.
        window.update(|window, cx| window.simulate_next_frame(cx));
        assert_eq!(
            window.update(|window, cx| window.simulate_next_frame(cx)),
            0,
            "{category}"
        );
    }
}

#[gpui_kit::test]
fn quick_launch_result_tween_saver_merges_only_its_field_and_keeps_live_value_on_failure(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_kit::init);
    let (directory, application, endpoint) = quick_launch_application_fixture(cx);
    let path = directory.path().join("config.json");
    AppConfig::update_section(&path, |config| {
        config.quick_launch.auto_quick_switch = false;
        config.quick_launch.excluded_paths = vec![r"D:\Fixture".into()];
        config.download.default_connection_limit = 24;
    })
    .expect("concurrent owner settings");
    for mode in nxtools_core::QuickLaunchResultEntrance::ALL {
        application.update(cx, |owner, cx| owner.set_result_entrance(mode, cx));
        let config = AppConfig::load_from(&path).expect("durable entrance setting");
        assert_eq!(config.quick_launch.result_entrance, mode);
        assert!(!config.quick_launch.auto_quick_switch);
        assert_eq!(config.quick_launch.excluded_paths, [r"D:\Fixture"]);
        assert_eq!(config.download.default_connection_limit, 24);
        assert_eq!(
            application.read_with(cx, |owner, _| owner.settings().result_entrance),
            mode
        );
    }
    std::fs::write(&path, b"{broken").expect("synthetic persistence failure");
    application.update(cx, |owner, cx| {
        owner.set_result_entrance(nxtools_core::QuickLaunchResultEntrance::None, cx)
    });
    assert_eq!(
        application.read_with(cx, |owner, _| owner.settings().result_entrance),
        nxtools_core::QuickLaunchResultEntrance::SlideFromRight
    );
    assert_eq!(std::fs::read(&path).expect("retained bytes"), b"{broken");
    assert!(endpoint.try_receive_command_for_contract_test().is_err());
}

#[test]
fn quick_launch_result_tween_source_uses_released_finite_timing_without_animation_tasks_or_bounds()
{
    let motion = include_str!("result_motion.rs")
        .split("#[cfg(test)]")
        .next()
        .expect("production motion");
    assert!(motion.contains("Timing::new(RESULTS_APPEAR_DURATION).ease(Easing::EaseOut)"));
    for forbidden in [
        "Spring",
        ".spawn(",
        ".timer(",
        "SetWindowBounds",
        ".repeat(",
    ] {
        assert!(!motion.contains(forbidden), "{forbidden}");
    }
    let panel = include_str!("panel.rs");
    assert!(
        !panel.contains(".with_animation("),
        "only query-owned row values animate"
    );
    assert!(panel.contains("window.request_animation_frame()"));
}

#[gpui_kit::test]
fn quick_launch_result_tween_waits_for_the_real_viewport_without_a_timer(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(|cx| {
        cx.set_reduce_motion(false);
        cx.set_global(crate::ui::motion::UiMotionPreference::Known {
            reduced_motion: false,
        });
    });
    let (panel, window) = quick_launch_panel_fixture(cx, Vec::new());
    window.simulate_resize(size(px(PANEL_WIDTH), px(COLLAPSED_PANEL_HEIGHT)));
    panel.update_in(window, |panel, window, cx| {
        panel.query_dispatch_generation += 1;
        panel.replace_results(
            vec![search_result("first", ApplicationSource::StartMenu)],
            cx,
        );
        panel.present_result_entrances(window, cx);
        assert!(panel.result_admission_pending);
        assert!(!panel.result_entrances.is_active());
    });
    window
        .executor()
        .advance_clock(RESULTS_APPEAR_DURATION * 10);
    panel.read_with(window, |panel, _| {
        assert!(!panel.result_entrances.is_active())
    });
    window.simulate_resize(size(px(PANEL_WIDTH), px(PANEL_HEIGHT)));
    panel.update_in(window, |panel, window, cx| {
        panel.present_result_entrances(window, cx);
        assert!(!panel.result_admission_pending);
        assert_eq!(
            panel
                .result_entrances
                .sample("first", cx.background_executor().now())
                .opacity,
            0.0,
            "native expansion latency cannot consume an entrance before it is visible"
        );
        panel.prepare_for_hide(window, cx);
        assert!(!panel.result_admission_pending);
        assert!(!panel.result_entrances.is_active());
    });
}
