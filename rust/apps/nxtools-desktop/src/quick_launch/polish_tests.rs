//! Production-render contracts for the bounded Quick Find polish slice.
use super::*;
use gpui::TestAppContext;

fn synthetic_result(id: &str) -> SearchResult {
    SearchResult {
        entry: nxtools_finder::ApplicationEntry {
            id: id.into(),
            name: format!("{id} — a deliberately long result title used to exercise bounded rails"),
            target: format!(r"C:\synthetic\{id}.txt"),
            source: ApplicationSource::IndexedFile,
        },
        action: SearchResultAction::OpenTarget,
        subtitle: Some(
            "A deliberately long metadata value that must stay inside its own rail".into(),
        ),
        score: 0,
        activation_count: 0,
        last_activated_at_unix_ms: None,
        query_activation_count: 0,
        last_query_activation_at_unix_ms: None,
    }
}

#[gpui::test]
fn polished_results_keep_title_metadata_and_hint_inside_the_row(cx: &mut TestAppContext) {
    cx.update(gpui_component::init);
    let (_root, window) = cx.add_window_view(|window, cx| {
        window.resize(size(px(PANEL_WIDTH), px(PANEL_HEIGHT)));
        let panel = cx.new(|cx| {
            let mut panel = QuickLaunchPanel::new(
                vec![synthetic_result("first"), synthetic_result("second")],
                nxtools_protocol::ForegroundContext {
                    window_handle: 1,
                    focus_window_handle: 1,
                    process_id: 1,
                    process_name: "synthetic".into(),
                    window_title: "synthetic".into(),
                },
                None,
                Vec::new(),
                WeakEntity::new_invalid(),
                window,
                cx,
            );
            panel.results_visible = true;
            panel
        });
        Root::new(panel, window, cx)
    });
    window.refresh().expect("render production result rows");
    let row = window
        .debug_bounds("quick-launch-result-row-0")
        .expect("result row");
    let copy = window
        .debug_bounds("quick-launch-result-copy-0")
        .expect("text rail");
    let title = window
        .debug_bounds("quick-launch-result-title-0")
        .expect("result title");
    let metadata = window
        .debug_bounds("quick-launch-result-subtitle-0")
        .expect("result metadata");
    let hint = window
        .debug_bounds("quick-launch-result-shortcut-0")
        .expect("shortcut rail");
    assert_eq!(row.size.height, px(COMPACT_RESULT_ROW_HEIGHT));
    assert_eq!(hint.size.width, px(48.0));
    assert_eq!(metadata.top() - title.bottom(), px(4.0));
    let icon = window
        .debug_bounds("quick-launch-result-icon-0")
        .expect("fixed icon rail");
    assert_eq!(icon.size.width, px(26.0));
    assert!(title.left() >= copy.left() && title.right() <= copy.right());
    assert!(metadata.left() >= copy.left() && metadata.right() <= copy.right());
    assert!(title.top() >= row.top() && metadata.bottom() <= row.bottom());
    assert!(copy.right() <= hint.left() && hint.right() <= row.right());
}

#[gpui::test]
fn polished_action_mode_keeps_disabled_rows_bounded_without_activating(cx: &mut TestAppContext) {
    cx.update(gpui_component::init);
    let panel_slot = Rc::new(RefCell::new(None));
    let slot = Rc::clone(&panel_slot);
    let (_root, window) = cx.add_window_view(move |window, cx| {
        window.resize(size(px(PANEL_WIDTH), px(PANEL_HEIGHT)));
        let panel = cx.new(|cx| {
            let mut panel = QuickLaunchPanel::new(
                vec![synthetic_result("first")],
                nxtools_protocol::ForegroundContext {
                    window_handle: 1,
                    focus_window_handle: 1,
                    process_id: 1,
                    process_name: "synthetic".into(),
                    window_title: "synthetic".into(),
                },
                None,
                Vec::new(),
                WeakEntity::new_invalid(),
                window,
                cx,
            );
            panel.results_visible = true;
            let mut mode =
                QuickLaunchActionMode::enter(panel.results[0].clone(), "query".into(), 0);
            mode.current_level_mut().shell_pending = false;
            mode.current_level_mut().items = vec![QuickLaunchActionItem {
                action: Some(QuickLaunchAction::Open),
                label: "Unavailable action".into(),
                icon: None,
                icon_path: None,
                enabled: false,
                submenu: None,
                section_label: None,
                divider_before: false,
            }];
            panel.action_mode = Some(mode);
            panel
        });
        *slot.borrow_mut() = Some(panel.clone());
        Root::new(panel, window, cx)
    });
    window.refresh().expect("render disabled action");
    let row = window
        .debug_bounds("quick-launch-action-row-0")
        .expect("disabled action row");
    let title = window
        .debug_bounds("quick-launch-action-title-0")
        .expect("disabled action title");
    let trailing = window
        .debug_bounds("quick-launch-action-trailing-0")
        .expect("action trailing rail");
    assert_eq!(row.size.height, px(COMPACT_RESULT_ROW_HEIGHT));
    assert!(title.right() <= trailing.left() && trailing.right() <= row.right());
    assert!(title.top() >= row.top() && title.bottom() <= row.bottom());
    let panel = panel_slot
        .borrow_mut()
        .take()
        .expect("production action panel");
    panel.update_in(window, |panel, window, cx| panel.focus_search(window, cx));
    window
        .refresh()
        .expect("paint search focus and current input key bindings");
    window.simulate_click(row.center(), gpui::Modifiers::default());
    window.simulate_event(gpui::KeyDownEvent {
        keystroke: gpui::Keystroke::parse("enter").expect("native Enter"),
        is_held: false,
    });
    window.simulate_event(gpui::KeyUpEvent {
        keystroke: gpui::Keystroke::parse("enter").expect("native Enter release"),
    });
    panel.update_in(window, |panel, _, cx| {
        assert!(
            panel
                .action_mode
                .as_ref()
                .is_some_and(|mode| mode.selected_action().is_none())
        );
        assert!(!panel.search_input.read(cx).value().contains('\n'));
    });
}

#[gpui::test]
fn polished_preview_keeps_identity_and_path_inside_fixed_native_shell(cx: &mut TestAppContext) {
    cx.update(gpui_component::init);
    let payload = QuickLaunchPreviewPayload {
        entry_id: "preview".into(),
        name: "A very long synthetic file name".repeat(20),
        target: r"C:\synthetic\very\long\nested\path\file.txt".repeat(20),
        detail: "TXT file · metadata".into(),
        created_at: Some("2026-10-09 12:00:00".into()),
        modified_at: Some("2026-10-09 12:01:00".into()),
        body: QuickLaunchPreviewBody::Metadata {
            icon: MaterialIcon::Info,
        },
    };
    let (_root, window) = cx.add_window_view(move |window, cx| {
        window.resize(size(
            px(QUICK_LAUNCH_PREVIEW_WIDTH),
            px(QUICK_LAUNCH_PREVIEW_HEIGHT),
        ));
        let preview = cx.new(|_| QuickLaunchPreview::new(payload, WeakEntity::new_invalid()));
        Root::new(preview, window, cx)
    });
    window.refresh().expect("render fixed auxiliary preview");
    let shell = window
        .debug_bounds("preview-shell")
        .expect("production preview shell");
    let title = window
        .debug_bounds("quick-launch-preview-title")
        .expect("bounded preview title");
    let footer = window
        .debug_bounds("quick-launch-preview-footer")
        .expect("fixed path footer");
    assert_eq!(
        shell.size,
        size(
            px(QUICK_LAUNCH_PREVIEW_WIDTH),
            px(QUICK_LAUNCH_PREVIEW_HEIGHT)
        )
    );
    assert!(title.left() >= shell.left() && title.right() <= shell.right());
    assert!(footer.bottom() <= shell.bottom() && footer.top() >= shell.top());
}

fn release_activation(window: &mut gpui::VisualTestContext, key: &str) {
    window.simulate_event(gpui::KeyUpEvent {
        keystroke: gpui::Keystroke::parse(key).expect("activation release"),
    });
}

#[gpui::test]
fn polished_result_tab_focus_activates_its_own_row_only_on_release(cx: &mut TestAppContext) {
    cx.update(gpui_component::init);
    let temp = tempfile::tempdir().expect("synthetic result catalog");
    let root = temp.path().join("applications");
    std::fs::create_dir_all(&root).expect("synthetic shortcut directory");
    for name in ["row-first.lnk", "row-second.lnk"] {
        std::fs::write(root.join(name), b"").expect("synthetic shortcut");
    }
    let catalog = FinderCatalog::open(
        temp.path().join("history.db"),
        &[nxtools_finder::ApplicationRoot {
            path: root,
            source: ApplicationSource::StartMenu,
        }],
    )
    .expect("actual synthetic Finder catalog");
    let results = catalog
        .search("row", RESULT_LIMIT)
        .expect("synthetic results");
    assert_eq!(results.len(), 2);
    let second_id = results[1].entry.id.clone();
    let application = cx.update(|cx| {
        let notifications = cx.new(|_| NotificationCenter::default());
        let (platform, _endpoint) = nxtools_ipc::platform_bridge(8);
        let (indexer, _endpoint) = crate::runtime::indexer_bridge(8);
        cx.new(|_| {
            QuickLaunchApplication::new(
                catalog,
                Rc::new(platform),
                Rc::new(indexer),
                AppConfig::default(),
                temp.path().join("config.json"),
                temp.path().join("logs"),
                notifications,
            )
        })
    });
    let rendered_application = application.clone();
    let slot = Rc::new(RefCell::new(None));
    let panel_slot = slot.clone();
    let (_root, window) = cx.add_window_view(move |window, cx| {
        window.resize(size(px(PANEL_WIDTH), px(PANEL_HEIGHT)));
        let panel = cx.new(|cx| {
            let mut panel = QuickLaunchPanel::new(
                results,
                nxtools_protocol::ForegroundContext {
                    window_handle: 1,
                    focus_window_handle: 1,
                    process_id: 1,
                    process_name: "synthetic".into(),
                    window_title: "synthetic".into(),
                },
                None,
                Vec::new(),
                rendered_application.downgrade(),
                window,
                cx,
            );
            panel.results_visible = true;
            panel
                .search_input
                .update(cx, |input, cx| input.set_value("row", window, cx));
            panel
        });
        *panel_slot.borrow_mut() = Some(panel.clone());
        Root::new(panel, window, cx)
    });
    let panel = slot.borrow_mut().take().expect("production panel");
    panel.update_in(window, |panel, window, cx| panel.focus_search(window, cx));
    window.refresh().expect("paint search and row focus order");
    window.simulate_keystrokes("tab tab");
    panel.update_in(window, |panel, window, cx| {
        assert!(!panel.search_input.focus_handle(cx).is_focused(window));
        assert_eq!(panel.selected, 0);
    });
    window.refresh().expect("keyed row focus survives repaint");
    window.simulate_keystrokes("enter");
    panel.read_with(window, |panel, _| assert_eq!(panel.selected, 0));
    release_activation(window, "enter");
    panel.read_with(window, |panel, _| assert_eq!(panel.selected, 1));
    window.simulate_keystrokes("right");
    panel.read_with(window, |panel, _| {
        assert_eq!(
            panel
                .action_mode
                .as_ref()
                .expect("row action mode")
                .target()
                .entry
                .id,
            second_id
        );
    });
    window.simulate_keystrokes("escape");
    panel.update_in(window, |panel, window, cx| {
        assert!(panel.action_mode.is_none());
        assert_eq!(panel.selected, 1);
        assert!(panel.search_input.focus_handle(cx).is_focused(window));
    });
}

#[gpui::test]
fn polished_action_tab_skips_disabled_and_enters_submenu_once_on_keyup(cx: &mut TestAppContext) {
    cx.update(gpui_component::init);
    let slot = Rc::new(RefCell::new(None));
    let panel_slot = slot.clone();
    let (_root, window) = cx.add_window_view(move |window, cx| {
        window.resize(size(px(PANEL_WIDTH), px(PANEL_HEIGHT)));
        let panel = cx.new(|cx| {
            let mut panel = QuickLaunchPanel::new(
                vec![synthetic_result("first")],
                nxtools_protocol::ForegroundContext {
                    window_handle: 1,
                    focus_window_handle: 1,
                    process_id: 1,
                    process_name: "synthetic".into(),
                    window_title: "synthetic".into(),
                },
                None,
                Vec::new(),
                WeakEntity::new_invalid(),
                window,
                cx,
            );
            panel.results_visible = true;
            let mut mode =
                QuickLaunchActionMode::enter(panel.results[0].clone(), "parent".into(), 0);
            mode.current_level_mut().shell_pending = false;
            mode.current_level_mut().items = [false, true]
                .into_iter()
                .map(|enabled| QuickLaunchActionItem {
                    action: None,
                    label: if enabled { "Submenu" } else { "Unavailable" }.into(),
                    icon: None,
                    icon_path: None,
                    enabled,
                    submenu: Some(super::actions::QuickLaunchShellSubmenu {
                        session_id: "synthetic".into(),
                        submenu_id: 7,
                    }),
                    section_label: None,
                    divider_before: false,
                })
                .collect();
            panel.action_mode = Some(mode);
            panel
        });
        *panel_slot.borrow_mut() = Some(panel.clone());
        Root::new(panel, window, cx)
    });
    let panel = slot.borrow_mut().take().expect("production panel");
    for key in ["enter", "space"] {
        panel.update_in(window, |panel, window, cx| panel.focus_search(window, cx));
        window
            .refresh()
            .expect("render disabled and eligible submenu rows");
        let disabled = window
            .debug_bounds("quick-launch-action-row-0")
            .expect("disabled row");
        window.simulate_click(disabled.center(), gpui::Modifiers::default());
        window.simulate_keystrokes("tab");
        panel.update_in(window, |panel, window, cx| {
            assert!(!panel.search_input.focus_handle(cx).is_focused(window));
            assert_eq!(panel.action_mode.as_ref().expect("mode").depth(), 1);
        });
        window.refresh().expect("preserve keyed submenu focus");
        window.simulate_keystrokes(key);
        window.simulate_event(gpui::KeyDownEvent {
            keystroke: gpui::Keystroke::parse(key).expect("repeat"),
            is_held: true,
        });
        panel.read_with(window, |panel, _| {
            assert_eq!(panel.action_mode.as_ref().expect("mode").depth(), 1)
        });
        release_activation(window, key);
        panel.update_in(window, |panel, window, cx| {
            let mode = panel.action_mode.as_ref().expect("submenu mode");
            assert_eq!(mode.depth(), 2);
            assert_eq!(mode.title(), "Submenu");
            assert!(panel.search_input.focus_handle(cx).is_focused(window));
        });
        window
            .refresh()
            .expect("restore search-owned submenu navigation");
        window.simulate_keystrokes("escape");
        panel.read_with(window, |panel, _| {
            assert_eq!(panel.action_mode.as_ref().expect("parent mode").depth(), 1)
        });
    }
}

#[gpui::test]
fn polished_file_dialog_row_keyboard_keeps_fixed_rails_and_search_navigation(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_component::init);
    let slot = Rc::new(RefCell::new(None));
    let panel_slot = slot.clone();
    let (_root, window) = cx.add_window_view(move |window, cx| {
        window.resize(size(px(500.0), px(300.0)));
        let panel = cx.new(|cx| super::file_dialog_search::polish_keyboard_fixture(window, cx));
        *panel_slot.borrow_mut() = Some(panel.clone());
        Root::new(panel, window, cx)
    });
    let panel = slot.borrow_mut().take().expect("attached search panel");
    panel.update_in(window, |panel, window, cx| {
        panel.polish_focus_search(window, cx)
    });
    window
        .refresh()
        .expect("render attached search keyboard boundary");
    let first = window
        .debug_bounds("file-dialog-search-result-row-0")
        .expect("first attached result");
    let second = window
        .debug_bounds("file-dialog-search-result-row-1")
        .expect("second attached result");
    let icon = window
        .debug_bounds("file-dialog-search-result-icon-1")
        .expect("icon rail");
    let title = window
        .debug_bounds("file-dialog-search-result-title-1")
        .expect("title rail");
    let metadata = window
        .debug_bounds("file-dialog-search-result-path-1")
        .expect("path rail");
    assert_eq!(second.size.height, px(40.0));
    assert_eq!(icon.size.width, px(26.0));
    assert_eq!(metadata.top() - title.bottom(), px(4.0));
    assert!(title.top() >= second.top() && metadata.bottom() <= second.bottom());
    window.simulate_click(first.center(), gpui::Modifiers::default());
    window.refresh().expect("paint first result focus");
    window.simulate_keystrokes("tab");
    window.refresh().expect("paint second result keyed focus");
    window.simulate_keystrokes("space");
    panel.read_with(window, |panel, _| assert_eq!(panel.polish_selected(), 0));
    release_activation(window, "space");
    panel.read_with(window, |panel, _| assert_eq!(panel.polish_selected(), 1));
    window.simulate_keystrokes("up");
    panel.read_with(window, |panel, _| assert_eq!(panel.polish_selected(), 0));
}

#[gpui::test]
fn polished_action_keyboard_dispatches_one_deferred_command_per_release(cx: &mut TestAppContext) {
    cx.update(gpui_component::init);
    let temp = tempfile::tempdir().expect("synthetic action catalog");
    let catalog = FinderCatalog::open(temp.path().join("history.db"), &[]).expect("catalog");
    let (application, endpoint) = cx.update(|cx| {
        let notifications = cx.new(|_| NotificationCenter::default());
        let (platform, endpoint) = nxtools_ipc::platform_bridge(8);
        endpoint.set_connected_for_contract_test(true);
        let (indexer, _endpoint) = crate::runtime::indexer_bridge(8);
        let application = cx.new(|_| {
            QuickLaunchApplication::new(
                catalog,
                Rc::new(platform),
                Rc::new(indexer),
                AppConfig::default(),
                temp.path().join("config.json"),
                temp.path().join("logs"),
                notifications,
            )
        });
        (application, endpoint)
    });
    let slot = Rc::new(RefCell::new(None));
    let panel_slot = slot.clone();
    let application_for_window = application.clone();
    let (_root, window) = cx.add_window_view(move |window, cx| {
        window.resize(size(px(PANEL_WIDTH), px(PANEL_HEIGHT)));
        let panel = cx.new(|cx| {
            let mut panel = QuickLaunchPanel::new(
                vec![synthetic_result("copy")],
                nxtools_protocol::ForegroundContext {
                    window_handle: 1,
                    focus_window_handle: 1,
                    process_id: 1,
                    process_name: "synthetic".into(),
                    window_title: "synthetic".into(),
                },
                None,
                Vec::new(),
                application_for_window.downgrade(),
                window,
                cx,
            );
            panel.results_visible = true;
            let mut mode =
                QuickLaunchActionMode::enter(panel.results[0].clone(), "parent".into(), 0);
            mode.current_level_mut().shell_pending = false;
            mode.current_level_mut().items = vec![QuickLaunchActionItem {
                action: Some(QuickLaunchAction::CopyPath),
                label: "Copy synthetic path".into(),
                icon: None,
                icon_path: None,
                enabled: true,
                submenu: None,
                section_label: None,
                divider_before: false,
            }];
            panel.action_mode = Some(mode);
            panel
        });
        *panel_slot.borrow_mut() = Some(panel.clone());
        Root::new(panel, window, cx)
    });
    let panel = slot.borrow_mut().take().expect("production copy panel");
    panel.update_in(window, |panel, window, cx| panel.focus_search(window, cx));
    window.refresh().expect("render single action");
    window.simulate_keystrokes("tab");
    window.refresh().expect("paint action focus");
    for key in ["enter", "space"] {
        window.simulate_keystrokes(key);
        window.simulate_event(gpui::KeyDownEvent {
            keystroke: gpui::Keystroke::parse(key).expect("repeat"),
            is_held: true,
        });
        assert!(endpoint.try_receive_command_for_contract_test().is_err());
        release_activation(window, key);
        window.run_until_parked();
        let command = endpoint
            .try_receive_command_for_contract_test()
            .expect("one deferred command");
        assert!(matches!(
            command.message,
            nxtools_protocol::Message::PlatformCommand(PlatformCommand::PasteText(_))
        ));
        assert!(
            endpoint.try_receive_command_for_contract_test().is_err(),
            "keyboard release dispatches exactly once"
        );
        panel.read_with(window, |panel, _| {
            assert!(panel.action_mode.is_some(), "copy keeps parent mode open")
        });
    }
}
