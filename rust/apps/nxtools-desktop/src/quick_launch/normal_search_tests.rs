use super::*;

#[gpui_kit::test]
fn normal_search_dispatches_without_delay_and_commits_only_complete_results(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_kit::init);
    let (_temporary, application, _platform) = quick_launch_application_fixture(cx);
    let (bridge, endpoint) = crate::runtime::indexer_bridge(16);
    endpoint.connect_for_test();
    application.update(cx, |app, _| {
        app.indexer = Rc::new(bridge);
        app.index_status = Some(IndexStatus {
            phase: nxtools_protocol::IndexPhase::Ready,
            indexed_items: 3,
            drives: Vec::new(),
            last_error: None,
        });
    });
    let (panel, window) = quick_launch_panel_fixture(
        cx,
        vec![search_result(
            "previous-query",
            ApplicationSource::StartMenu,
        )],
    );
    window.update(|window, cx| {
        application.update(cx, |app, _| {
            app.popup = Some(QuickLaunchWindow {
                window: window.window_handle(),
                panel: panel.downgrade(),
                native_window_handle: 1,
            });
        });
    });
    panel.update_in(window, |panel, window, cx| {
        panel.application = application.downgrade();
        panel.focus_search(window, cx);
    });
    window
        .refresh()
        .expect("render original focused search input");
    let mut expected_query = String::new();
    let mut latest = None;
    for character in "normal-mode".chars() {
        expected_query.push(character);
        window.simulate_input(&character.to_string());
        window.run_until_parked();
        let envelope = endpoint
            .try_recv_command_for_test()
            .expect("each changed input must dispatch before advancing any debounce clock");
        let nxtools_protocol::Message::IndexerCommand(IndexerCommand::Search(request)) =
            envelope.message
        else {
            panic!("input must dispatch a typed search");
        };
        assert_eq!(request.query, expected_query);
        latest = Some(request);
    }
    let request = latest.expect("final typed query");
    let deliver =
        |window: &mut gpui_kit::VisualTestContext, generation: u64, name: &str, complete: bool| {
            application.update(window, |app, cx| {
                app.handle_indexer_notification(
                    IndexerNotification::Event(IndexerEvent::SearchBatch(IndexSearchBatch {
                        generation,
                        items: vec![IndexSearchItem {
                            name: name.into(),
                            path: format!(r"C:\synthetic\{name}"),
                            is_directory: false,
                            score: 0,
                            size_bytes: None,
                            modified_at_unix_ms: None,
                        }],
                        complete,
                        elapsed_us: 25,
                    })),
                    cx,
                );
            });
        };
    deliver(window, request.generation, "normal-mode-first.txt", false);
    for (advance_ms, visible) in [(50, false), (99, false), (1, true)] {
        window
            .executor()
            .advance_clock(Duration::from_millis(advance_ms));
        window.run_until_parked();
        window.refresh().expect("render search feedback");
        assert_eq!(
            window
                .debug_bounds("quick-launch-search-feedback")
                .is_some(),
            visible
        );
    }
    window.executor().advance_clock(Duration::from_secs(1));
    window.run_until_parked();
    panel.read_with(window, |panel, _| {
        assert!(
            panel.search_pending,
            "time alone cannot complete a provider"
        );
        assert_eq!(panel.results.len(), 1);
        assert_eq!(panel.results[0].entry.id, "previous-query");
    });
    deliver(window, request.generation, "normal-mode-second.txt", true);
    window.run_until_parked();
    window.refresh().expect("render completed search");
    assert!(
        window
            .debug_bounds("quick-launch-search-feedback")
            .is_none()
    );
    let committed = panel.read_with(window, |panel, _| {
        assert!(!panel.search_pending);
        assert_eq!(panel.committed_query.as_ref(), "normal-mode");
        assert_eq!(
            panel.results.len(),
            2,
            "commit the complete merged result set"
        );
        assert!(
            panel
                .results
                .iter()
                .any(|row| row.entry.name == "normal-mode-first.txt")
        );
        assert!(
            panel
                .results
                .iter()
                .any(|row| row.entry.name == "normal-mode-second.txt")
        );
        panel.results.clone()
    });
    panel.update_in(window, |panel, window, cx| {
        panel
            .search_input
            .update(cx, |input, cx| input.replace_all("replacement", window, cx));
    });
    window.run_until_parked();
    let replacement = endpoint
        .try_recv_command_for_test()
        .expect("replacement input also dispatches without a timer");
    let nxtools_protocol::Message::IndexerCommand(IndexerCommand::Search(replacement)) =
        replacement.message
    else {
        panic!("replacement query must dispatch search");
    };
    assert!(replacement.generation > request.generation);
    window.executor().advance_clock(Duration::from_millis(150));
    window.run_until_parked();
    window.refresh().expect("render replacement feedback");
    assert!(
        window
            .debug_bounds("quick-launch-search-feedback")
            .is_some()
    );
    deliver(window, request.generation, "normal-mode-stale.txt", true);
    window.run_until_parked();
    window.refresh().expect("render after stale completion");
    assert!(
        window
            .debug_bounds("quick-launch-search-feedback")
            .is_some()
    );
    panel.read_with(window, |panel, _| {
        assert!(panel.search_pending);
        assert_eq!(panel.results.as_slice(), committed.as_slice());
    });
    deliver(
        window,
        replacement.generation,
        "replacement-current.txt",
        true,
    );
    window.run_until_parked();
    window.refresh().expect("render replacement completion");
    assert!(
        window
            .debug_bounds("quick-launch-search-feedback")
            .is_none()
    );
    panel.read_with(window, |panel, _| {
        assert!(!panel.search_pending);
        assert_eq!(panel.committed_query.as_ref(), "replacement");
        assert_eq!(panel.results.len(), 1);
        assert_eq!(panel.results[0].entry.name, "replacement-current.txt");
    });

    let start = |window: &mut gpui_kit::VisualTestContext, query: &str| {
        panel.update_in(window, |panel, window, cx| {
            panel
                .search_input
                .update(cx, |input, cx| input.replace_all(query, window, cx));
        });
        window.run_until_parked();
        loop {
            match endpoint
                .try_recv_command_for_test()
                .expect("immediate search")
                .message
            {
                nxtools_protocol::Message::IndexerCommand(IndexerCommand::Search(request)) => {
                    assert_eq!(request.query, query);
                    break request.generation;
                }
                nxtools_protocol::Message::IndexerCommand(IndexerCommand::CancelSearch(_)) => {}
                _ => panic!("search command expected"),
            }
        }
    };
    let fast = start(window, "fast");
    window.executor().advance_clock(Duration::from_millis(50));
    deliver(window, fast, "fast.txt", true);
    window.run_until_parked();
    window.executor().advance_clock(Duration::from_millis(150));
    window.run_until_parked();
    window
        .refresh()
        .expect("fast query must not flash after completion");
    assert!(
        window
            .debug_bounds("quick-launch-search-feedback")
            .is_none()
    );

    start(window, "cleared");
    panel.update_in(window, |panel, window, cx| {
        panel
            .search_input
            .update(cx, |input, cx| input.replace_all("", window, cx));
    });
    window.run_until_parked();
    window.executor().advance_clock(Duration::from_millis(150));
    window.run_until_parked();
    window.refresh().expect("render cleared query");
    assert!(
        window
            .debug_bounds("quick-launch-search-feedback")
            .is_none()
    );

    start(window, "hidden");
    panel.update_in(window, |panel, window, cx| {
        panel.prepare_for_hide(window, cx)
    });
    window.executor().advance_clock(Duration::from_millis(150));
    window.run_until_parked();
    window.refresh().expect("render retired query");
    assert!(
        window
            .debug_bounds("quick-launch-search-feedback")
            .is_none()
    );
}
