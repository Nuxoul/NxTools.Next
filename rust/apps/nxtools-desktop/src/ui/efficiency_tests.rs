//! First-tier tests use the real panel, temporary worker and bounded fake IPC endpoint.
use super::*;
use gpui_kit::base::Root;
use gpui_kit::{
    EntityInputHandler, KeyDownEvent, KeyUpEvent, Keystroke, Modifiers, TestAppContext,
    VisualTestContext,
};

struct Fixture {
    panel: Entity<ClipboardPanel>,
    application: Entity<ClipboardApplication>,
    endpoint: nxtools_ipc::PlatformServerEndpoint,
    window: gpui_kit::AnyWindowHandle,
    worker: crate::clipboard_worker::ClipboardCaptureWorkerHandle,
    _directory: tempfile::TempDir,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.worker
            .clone()
            .shutdown()
            .expect("close synthetic history worker");
    }
}
fn fixture(cx: &mut TestAppContext) -> Fixture {
    cx.update(gpui_kit::init);
    cx.executor().allow_parking();
    let directory = tempfile::tempdir().expect("temporary efficiency fixture");
    let worker = crate::clipboard_worker::ClipboardCaptureWorkerHandle::spawn(
        directory.path().join("clipboard.db"),
        directory.path().join("images"),
        nxtools_clipboard::ClipboardRetentionPolicy {
            max_items: 1000,
            retention_days: 30,
        },
        1000,
    )
    .expect("initialize before seeding");
    let page = {
        let mut history =
            nxtools_clipboard::ClipboardHistory::open(directory.path().join("clipboard.db"))
                .expect("seed connection");
        for index in 0..20 {
            history
                .capture(nxtools_clipboard::TextCapture {
                    text: format!("synthetic record {index:02} {} TAIL", "文😀".repeat(1000)),
                    native_formats: vec![],
                    source: nxtools_clipboard::CaptureSource {
                        window_handle: 1,
                        process_id: 1,
                        process_name: "synthetic".into(),
                        process_icon_path: String::new(),
                        window_title: String::new(),
                    },
                    captured_at_unix_ms: 2000 + index,
                })
                .expect("seed record");
        }
        history
            .summary_category_page_with_total(
                "",
                BUILTIN_ALL_CATEGORY_ID,
                ClipboardSearchScope::Content,
                0,
                80,
                None,
            )
            .expect("first summary page")
    };
    let summary_counts = page
        .items
        .iter()
        .map(|row| (row.item.id, row.text_character_count))
        .collect();
    let page = ClipboardHistoryPage {
        items: page.items.into_iter().map(|row| row.item).collect(),
        total: page.total,
        has_more: page.has_more,
    };
    let (platform, endpoint) = nxtools_ipc::platform_bridge(32);
    endpoint.set_connected_for_contract_test(true);
    let notifications =
        cx.update(|cx| cx.new(|_| crate::notifications::NotificationCenter::default()));
    let application = cx.update(|cx| {
        cx.new(|cx| {
            ClipboardApplication::new(
                worker.clone(),
                Rc::new(platform),
                nxtools_core::AppConfig::default(),
                directory.path().join("config.json"),
                notifications,
                cx,
            )
        })
    });
    let held = worker.clone();
    let weak = application.downgrade();
    let slot = Rc::new(RefCell::new(None));
    let retained = slot.clone();
    let (_, surface) = cx.add_window_view(move |window, cx| {
        let panel = cx.new(|cx| {
            ClipboardPanel::new(
                page,
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
                    window_handle: 42,
                    focus_window_handle: 43,
                    process_id: 2,
                    process_name: "target".into(),
                    window_title: String::new(),
                },
                held,
                weak,
                uuid::Uuid::new_v4(),
                None,
                window,
                cx,
            )
            .with_summary_counts(summary_counts)
        });
        *retained.borrow_mut() = Some(panel.clone());
        Root::new(panel, window, cx)
    });
    surface.simulate_resize(size(px(300.0), px(440.0)));
    surface.update(|window, _| window.activate_window());
    surface.refresh().expect("production panel");
    let panel = slot.borrow().clone().expect("panel entity");
    panel.update_in(surface, |panel, window, cx| {
        panel.focus_search_input(window, cx)
    });
    let window = surface.update(|window, _| window.window_handle());
    Fixture {
        panel,
        application,
        endpoint,
        window,
        worker,
        _directory: directory,
    }
}
fn press(surface: &mut VisualTestContext, key: &str) {
    let keystroke = Keystroke::parse(key).expect("test keystroke");
    surface.simulate_event(KeyDownEvent {
        keystroke: keystroke.clone(),
        is_held: false,
        prefer_character_input: false,
    });
    surface.simulate_event(KeyUpEvent { keystroke });
}

#[gpui_kit::test]
fn efficiency_page_keys_move_selection_without_changing_search_or_pasting(cx: &mut TestAppContext) {
    let f = fixture(cx);
    let mut surface = VisualTestContext::from_window(f.window, cx);
    press(&mut surface, "pagedown");
    let next = f.panel.read_with(&surface, |panel, _| {
        panel.selected_entry_index.expect("selected")
    });
    assert!(
        next > 0 && next < 10,
        "page should fit compact viewport, got {next}"
    );
    press(&mut surface, "pageup");
    f.panel.read_with(&surface, |panel, cx| {
        assert_eq!(panel.selected_entry_index, Some(0));
        assert!(panel.search_input.read(cx).value().is_empty());
        assert!(panel.pending_action.is_none());
    });
    assert!(f.endpoint.try_receive_command_for_contract_test().is_err());
}

#[gpui_kit::test]
fn efficiency_number_paste_selects_exact_id_and_keeps_failure_retryable(cx: &mut TestAppContext) {
    let f = fixture(cx);
    let mut surface = VisualTestContext::from_window(f.window, cx);
    let id = f
        .panel
        .read_with(&surface, |panel, _| panel.entries.borrow()[1].id);
    press(&mut surface, "ctrl-2");
    let command_id = f.panel.read_with(&surface, |panel, _| {
        let (command_id, action, item_id) =
            panel.pending_action.expect("number shortcut starts action");
        assert_eq!(item_id, id);
        assert_eq!(action, ClipboardAction::OriginalPaste);
        command_id
    });
    let mut envelope = None;
    for _ in 0..200 {
        surface.run_until_parked();
        if let Ok(next) = f.endpoint.try_receive_command_for_contract_test() {
            envelope = Some(next);
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(
        envelope.expect("real correlated native command").id,
        command_id
    );
    press(&mut surface, "ctrl-3");
    assert!(
        f.endpoint.try_receive_command_for_contract_test().is_err(),
        "pending action blocks duplicate"
    );
    f.panel.update_in(&mut surface, |panel, window, cx| {
        panel.handle_command_result(
            command_id,
            PlatformCommandResult {
                success: false,
                error: Some("synthetic rejection".into()),
                native_error_code: Some(5),
                data: None,
            },
            window,
            cx,
        )
    });
    f.panel.read_with(&surface, |panel, _| {
        assert!(panel.is_open);
        assert!(panel.pending_action.is_none());
    });
}

#[gpui_kit::test]
fn efficiency_number_and_page_keys_ignore_edit_query_pending_and_ime(cx: &mut TestAppContext) {
    let f = fixture(cx);
    let mut surface = VisualTestContext::from_window(f.window, cx);
    f.panel
        .update_in(&mut surface, |panel, _, _| panel.search_pending = true);
    press(&mut surface, "ctrl-1");
    press(&mut surface, "pagedown");
    f.panel.read_with(&surface, |panel, _| {
        assert!(panel.pending_action.is_none());
        assert_eq!(panel.selected_entry_index, Some(0));
    });
    f.panel
        .update_in(&mut surface, |panel, _, _| panel.search_pending = false);
    f.panel.update_in(&mut surface, |panel, window, cx| {
        panel.search_input.update(cx, |input, cx| {
            input.replace_and_mark_text_in_range(None, "中", Some(1..1), window, cx)
        });
    });
    press(&mut surface, "ctrl-1");
    press(&mut surface, "pagedown");
    f.panel
        .read_with(&surface, |panel, _| assert!(panel.pending_action.is_none()));
    assert!(f.endpoint.try_receive_command_for_contract_test().is_err());
    let item = f
        .panel
        .read_with(&surface, |panel, _| panel.entries.borrow()[0].clone());
    f.panel.update_in(&mut surface, |panel, window, cx| {
        panel.begin_edit(&item, window, cx)
    });
    press(&mut surface, "ctrl-2");
    f.panel
        .read_with(&surface, |panel, _| assert!(panel.pending_action.is_none()));
    let _ = (&f.application, Modifiers::default());
}

#[gpui_kit::test]
fn efficiency_summary_panel_hydrates_full_text_for_paste_and_edit(cx: &mut TestAppContext) {
    let f = fixture(cx);
    let mut surface = VisualTestContext::from_window(f.window, cx);
    let item = f
        .panel
        .read_with(&surface, |panel, _| panel.entries.borrow()[0].clone());
    assert_eq!(item.text.chars().count(), 400);
    f.panel.update_in(&mut surface, |panel, window, cx| {
        panel.activate_entry(&item, ClipboardAction::PlainText, window, cx)
    });
    let mut received = None;
    for _ in 0..200 {
        surface.run_until_parked();
        if let Ok(envelope) = f.endpoint.try_receive_command_for_contract_test() {
            received = Some(envelope);
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    let envelope = received.expect("hydrated native payload");
    match envelope.message {
        nxtools_protocol::Message::PlatformCommand(
            nxtools_protocol::PlatformCommand::PasteText(command),
        ) => {
            assert!(command.text.ends_with(" TAIL"));
            assert!(command.text.chars().count() > 2000);
        }
        other => panic!(
            "unexpected synthetic command kind: {}",
            matches!(other, nxtools_protocol::Message::PlatformCommand(_))
        ),
    }
    f.panel.update_in(&mut surface, |panel, window, cx| {
        panel.handle_platform_disconnected(cx);
        panel.begin_edit(&item, window, cx);
    });
    for _ in 0..200 {
        surface.run_until_parked();
        if f.panel
            .read_with(&surface, |panel, _| panel.editing_item_id.is_some())
        {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    f.panel.read_with(&surface, |panel, cx| {
        assert_eq!(panel.editing_item_id, Some(item.id));
        assert!(panel.edit_input.read(cx).value().ends_with(" TAIL"));
    });
}

#[gpui_kit::test]
fn efficiency_number_release_rejects_replaced_generation_and_held_repeat(cx: &mut TestAppContext) {
    let f = fixture(cx);
    let mut surface = VisualTestContext::from_window(f.window, cx);
    let key = Keystroke::parse("ctrl-2").expect("number key");
    surface.simulate_event(KeyDownEvent {
        keystroke: key.clone(),
        is_held: false,
        prefer_character_input: false,
    });
    surface.simulate_event(KeyDownEvent {
        keystroke: key.clone(),
        is_held: true,
        prefer_character_input: false,
    });
    assert!(
        f.endpoint.try_receive_command_for_contract_test().is_err(),
        "no keydown paste"
    );
    f.panel.update_in(&mut surface, |panel, _, _| {
        panel.query_dispatch_generation += 1
    });
    surface.simulate_event(KeyUpEvent { keystroke: key });
    f.panel
        .read_with(&surface, |panel, _| assert!(panel.pending_action.is_none()));
    assert!(f.endpoint.try_receive_command_for_contract_test().is_err());
}

#[gpui_kit::test]
fn efficiency_summary_deleted_item_cannot_dispatch_and_metadata_is_bounded(
    cx: &mut TestAppContext,
) {
    let f = fixture(cx);
    let mut surface = VisualTestContext::from_window(f.window, cx);
    let item = f
        .panel
        .read_with(&surface, |panel, _| panel.entries.borrow()[0].clone());
    {
        let mut history =
            nxtools_clipboard::ClipboardHistory::open(f._directory.path().join("clipboard.db"))
                .expect("fixture DB");
        history
            .soft_delete(item.id, 3000)
            .expect("delete exact record");
    }
    f.panel.update_in(&mut surface, |panel, window, cx| {
        panel.activate_entry(&item, ClipboardAction::PlainText, window, cx)
    });
    for _ in 0..200 {
        surface.run_until_parked();
        if f.panel
            .read_with(&surface, |panel, _| panel.pending_action.is_none())
        {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    f.panel
        .read_with(&surface, |panel, _| assert!(panel.pending_action.is_none()));
    assert!(
        f.endpoint.try_receive_command_for_contract_test().is_err(),
        "deleted summary never becomes a native payload"
    );
    f.panel.update_in(&mut surface, |panel, _, _| {
        panel.entries.borrow_mut().truncate(1);
        panel.prune_list_metadata();
        assert_eq!(panel.summary_counts.len(), 1);
    });
}

#[gpui_kit::test]
fn efficiency_summary_targets_use_full_tail_and_recheck_before_open(cx: &mut TestAppContext) {
    let f = fixture(cx);
    let mut surface = VisualTestContext::from_window(f.window, cx);
    let item = f
        .panel
        .read_with(&surface, |panel, _| panel.entries.borrow()[0].clone());
    for _ in 0..200 {
        surface.run_until_parked();
        if f.panel.read_with(&surface, |panel, _| {
            !panel.open_targets_pending && panel.open_targets.contains_key(&item.id)
        }) {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    let url = "https://example.invalid/full-tail";
    {
        let mut history =
            nxtools_clipboard::ClipboardHistory::open(f._directory.path().join("clipboard.db"))
                .expect("fixture DB");
        history
            .update_text(item.id, &format!("{}\n{url}", "文".repeat(800)))
            .expect("tail URL");
    }
    f.panel.update_in(&mut surface, |panel, _, cx| {
        panel.open_targets.remove(&item.id);
        panel.request_open_targets(item.id, cx);
    });
    for _ in 0..200 {
        surface.run_until_parked();
        if f.panel.read_with(&surface, |panel, _| {
            panel
                .open_targets
                .get(&item.id)
                .is_some_and(|targets| targets.iter().any(|target| target.value == url))
        }) {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    let target = f.panel.read_with(&surface, |panel, _| {
        panel.open_targets[&item.id]
            .iter()
            .find(|target| target.value == url)
            .expect("tail resolved from full body")
            .clone()
    });
    {
        let mut history =
            nxtools_clipboard::ClipboardHistory::open(f._directory.path().join("clipboard.db"))
                .expect("fixture DB");
        history
            .soft_delete(item.id, 3000)
            .expect("delete before open");
    }
    f.panel.update_in(&mut surface, |panel, window, cx| {
        panel.open_full_item_target(item.id, target, window, cx)
    });
    for _ in 0..200 {
        surface.run_until_parked();
        if f.panel
            .read_with(&surface, |panel, _| panel.full_item_request.is_none())
        {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(
        f.endpoint.try_receive_command_for_contract_test().is_err(),
        "stale cached target cannot open deleted item"
    );
}

#[gpui_kit::test]
fn efficiency_capture_pause_official_controls_keep_search_and_history_usable(
    cx: &mut TestAppContext,
) {
    let f = fixture(cx);
    let mut surface = VisualTestContext::from_window(f.window, cx);
    let trigger = surface
        .debug_bounds("clipboard-capture-pause")
        .expect("official pause button");
    let header = surface
        .debug_bounds("clipboard-panel-header")
        .expect("header");
    assert!(trigger.origin.x + trigger.size.width >= header.origin.x + header.size.width - px(1.0));
    assert!(
        trigger.origin.y >= header.origin.y
            && trigger.origin.y + trigger.size.height <= header.origin.y + header.size.height
    );
    assert!(
        trigger.size.width <= px(32.0),
        "icon only, not a labelled action"
    );
    surface.simulate_click(trigger.center(), Modifiers::default());
    surface.run_until_parked();
    surface.refresh().expect("direct pause toggle");
    let envelope = f
        .endpoint
        .try_receive_command_for_contract_test()
        .expect("real panel to application command");
    f.panel.update_in(&mut surface, |panel, window, cx| {
        assert!(panel.search_input.focus_handle(cx).is_focused(window))
    });
    assert!(matches!(
        envelope.message,
        nxtools_protocol::Message::PlatformCommand(
            nxtools_protocol::PlatformCommand::SetClipboardCapturePaused(
                nxtools_protocol::SetClipboardCapturePausedCommand { paused: true }
            )
        )
    ));
    f.application.update(&mut surface, |owner, cx| {
        owner.handle_platform_notification(
            nxtools_ipc::PlatformNotification::CommandResult {
                correlation_id: envelope.id,
                result: PlatformCommandResult {
                    success: true,
                    error: None,
                    native_error_code: None,
                    data: Some(r#"{"capture_paused":true}"#.into()),
                },
            },
            cx,
        );
    });
    let applied = f
        .application
        .read_with(&surface, |owner, _| owner.capture_pause_snapshot());
    assert_eq!(applied.applied, Some(true));
    assert!(!applied.pending && !applied.failed);
    f.panel.update_in(&mut surface, |panel, _, cx| {
        panel.update_capture_pause(applied, cx)
    });
    surface.refresh().expect("play icon after pause");
    let resume = surface
        .debug_bounds("clipboard-capture-pause")
        .expect("same header toggle");
    surface.simulate_click(resume.center(), Modifiers::default());
    surface.run_until_parked();
    let resume = f
        .endpoint
        .try_receive_command_for_contract_test()
        .expect("one click resumes");
    assert!(matches!(
        resume.message,
        nxtools_protocol::Message::PlatformCommand(
            nxtools_protocol::PlatformCommand::SetClipboardCapturePaused(
                nxtools_protocol::SetClipboardCapturePausedCommand { paused: false }
            )
        )
    ));
    assert!(f.endpoint.try_receive_command_for_contract_test().is_err());
    press(&mut surface, "pagedown");
    f.panel.read_with(&surface, |panel, _| {
        assert!(panel.selected_entry_index.is_some_and(|index| index > 0))
    });
}

#[gpui_kit::test]
fn efficiency_list_thumbnail_resource_never_substitutes_original_payload(cx: &mut TestAppContext) {
    let f = fixture(cx);
    let mut surface = VisualTestContext::from_window(f.window, cx);
    let root = f._directory.path().join("images");
    let original = root.join(format!("{}.png", "c".repeat(64)));
    image::RgbaImage::from_pixel(1000, 600, image::Rgba([10, 20, 30, 90]))
        .save(&original)
        .expect("synthetic image");
    let before = std::fs::read(&original).expect("original bytes");
    f.panel.update_in(&mut surface, |panel, _, cx| {
        let mut item = panel.entries.borrow()[0].clone();
        item.kind = ClipboardKind::Image;
        item.image_path = original.to_string_lossy().into_owned();
        item.image_width = 1000;
        item.image_height = 600;
        *panel.entries.borrow_mut() = vec![item];
        panel.list_scroll.reset(1);
        panel.prune_list_metadata();
        cx.notify();
    });
    for _ in 0..200 {
        surface.run_until_parked();
        if f.panel.read_with(&surface, |panel, _| {
            panel
                .list_thumbnails
                .get(original.to_str().expect("fixture path"))
                .is_some_and(Option::is_some)
        }) {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    surface.refresh().expect("derived image row");
    f.panel.read_with(&surface, |panel, _| {
        let derivative = panel.list_thumbnails[original.to_str().expect("fixture path")]
            .as_ref()
            .expect("worker derivative");
        assert_ne!(derivative, &original);
        assert!(
            panel
                .image_resource_cache
                .borrow()
                .entries
                .contains(derivative)
        );
        assert!(
            !panel
                .image_resource_cache
                .borrow()
                .entries
                .contains(&original)
        );
        assert_eq!(
            panel.entries.borrow()[0].image_path,
            original.to_string_lossy()
        );
    });
    assert_eq!(
        std::fs::read(&original).expect("original untouched"),
        before
    );
}

#[gpui_kit::test]
fn efficiency_summary_preparation_cancels_on_query_replacement(cx: &mut TestAppContext) {
    let f = fixture(cx);
    let mut surface = VisualTestContext::from_window(f.window, cx);
    f.panel.update_in(&mut surface, |panel, window, cx| {
        let entry = panel.entries.borrow()[0].clone();
        panel.activate_entry(&entry, ClipboardAction::PlainText, window, cx);
        panel.query_dispatch_generation += 1;
    });
    for _ in 0..200 {
        surface.run_until_parked();
        if f.panel
            .read_with(&surface, |panel, _| panel.pending_action.is_none())
        {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(
        f.endpoint.try_receive_command_for_contract_test().is_err(),
        "obsolete query must not dispatch after hydration"
    );
    f.panel.read_with(&surface, |panel, _| {
        assert!(panel.pending_action.is_none());
        assert!(panel.pending_action_started_at.is_none());
    });
}

#[gpui_kit::test]
fn feedback_shortcuts_work_from_history_controls_focus(cx: &mut TestAppContext) {
    let f = fixture(cx);
    let mut surface = VisualTestContext::from_window(f.window, cx);
    surface.update(|window, cx| window.focus_next(cx));
    f.panel.update_in(&mut surface, |panel, window, cx| {
        assert!(!panel.search_input.focus_handle(cx).is_focused(window));
    });
    press(&mut surface, "pagedown");
    assert!(
        f.panel.read_with(&surface, |panel, _| panel
            .selected_entry_index
            .is_some_and(|i| i > 0)),
        "history focus must not disable PageDown"
    );
    press(&mut surface, "ctrl-2");
    assert_number_paste(&f, &mut surface, 1);
}

fn assert_number_paste(f: &Fixture, surface: &mut VisualTestContext, index: usize) {
    let (command_id, expected_text) = f.panel.read_with(surface, |panel, _| {
        let entry = &panel.entries.borrow()[index];
        let (id, action, item_id) = panel.pending_action.expect("shortcut must start a paste");
        assert_eq!(item_id, entry.id);
        assert_eq!(action, ClipboardAction::OriginalPaste);
        (id, entry.text.clone())
    });
    let mut received = None;
    for _ in 0..200 {
        surface.run_until_parked();
        if let Ok(envelope) = f.endpoint.try_receive_command_for_contract_test() {
            received = Some(envelope);
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    let envelope = received.expect("typed native paste command");
    assert_eq!(envelope.id, command_id);
    match envelope.message {
        nxtools_protocol::Message::PlatformCommand(
            nxtools_protocol::PlatformCommand::PasteText(command),
        ) => {
            assert!(command.text.starts_with(&expected_text));
            assert!(command.text.ends_with(" TAIL"));
            assert_eq!(command.target_window_handle, 42);
        }
        _ => panic!("must be PasteText, not unrelated platform traffic"),
    }
}

#[gpui_kit::test]
fn feedback_shortcut_allows_control_released_before_number(cx: &mut TestAppContext) {
    let f = fixture(cx);
    let mut surface = VisualTestContext::from_window(f.window, cx);
    surface.simulate_event(KeyDownEvent {
        keystroke: Keystroke::parse("ctrl-2").expect("key"),
        is_held: false,
        prefer_character_input: false,
    });
    surface.simulate_event(KeyUpEvent {
        keystroke: Keystroke::parse("control").expect("control release"),
    });
    surface.simulate_event(KeyUpEvent {
        keystroke: Keystroke::parse("2").expect("number release"),
    });
    assert_number_paste(&f, &mut surface, 1);
}

#[gpui_kit::test]
fn feedback_legacy_number_shortcut_works_with_empty_search(cx: &mut TestAppContext) {
    let f = fixture(cx);
    let mut surface = VisualTestContext::from_window(f.window, cx);
    press(&mut surface, "2");
    assert_number_paste(&f, &mut surface, 1);
}

#[gpui_kit::test]
fn feedback_history_text_has_markdown_presentation(cx: &mut TestAppContext) {
    let f = fixture(cx);
    let mut surface = VisualTestContext::from_window(f.window, cx);
    f.panel.update_in(&mut surface, |panel, _, cx| {
        panel.entries.borrow_mut()[0].text = "# 标题\n\n_强调_ 与 `代码`\n\n- 条目".into();
        panel.list_scroll.reset(20);
        cx.notify();
    });
    surface.refresh().expect("history Markdown");
    assert!(
        surface.debug_bounds("clipboard-history-markdown").is_some(),
        "actual row Markdown renderer"
    );
    let row = surface
        .debug_bounds("history-row-slot-0")
        .expect("first Markdown row");
    assert!(
        row.size.height > px(history::TEXT_ROW_HEIGHT + history::ROW_GAP),
        "Markdown must have room beyond the old single line"
    );
    let id = f
        .panel
        .read_with(&surface, |panel, _| panel.entries.borrow()[0].id);
    let state = history::RENDERED_MARKDOWN
        .with_borrow(|rows| rows[&id].upgrade().expect("mounted row state"));
    for _ in 0..200 {
        surface.run_until_parked();
        if state.read_with(&surface, |state, _| !state.rendered_text().is_empty()) {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    state.read_with(&surface, |state, _| {
        let text = state.rendered_text();
        assert!(text.as_str().contains("标题") && text.as_str().contains("强调"));
        assert!(!text.as_str().contains("# 标题") && !text.as_str().contains("_强调_"));
    });
    f.panel.read_with(&surface, |panel, _| {
        assert!(panel.entries.borrow()[0].text.contains("_强调_"))
    });
    surface.refresh().expect("unchanged row repaint");
    let same =
        history::RENDERED_MARKDOWN.with_borrow(|rows| rows[&id].upgrade().expect("cached state"));
    assert_eq!(
        state.entity_id(),
        same.entity_id(),
        "no reparse or state recreation on repaint"
    );
    f.panel.update_in(&mut surface, |panel, _, cx| {
        panel.entries.borrow_mut()[0].text =
            "**Updated** ![blocked](https://example.invalid/image.png)".into();
        panel.list_scroll.reset(20);
        cx.notify();
    });
    surface
        .refresh()
        .expect("changed summary invalidates cached parse");
    let changed =
        history::RENDERED_MARKDOWN.with_borrow(|rows| rows[&id].upgrade().expect("updated state"));
    assert_ne!(same.entity_id(), changed.entity_id());
    for _ in 0..200 {
        surface.run_until_parked();
        if changed.read_with(&surface, |state, _| !state.rendered_text().is_empty()) {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    changed.read_with(&surface, |state, _| {
        let text = state.rendered_text();
        assert!(text.as_str().contains("Updated") && text.as_str().contains("未加载"));
        assert!(!text.as_str().contains("https://example.invalid"));
    });
}

#[test]
fn feedback_pause_play_use_distinct_real_official_assets() {
    use gpui_kit::{AssetSource, assets::IconName};
    let mut icons = vec![];
    for icon in [IconName::Pause, IconName::Play] {
        let bytes = crate::assets::NxToolsAssets
            .load(icon.path().as_ref())
            .expect("asset lookup")
            .expect("official SVG");
        assert!(bytes.len() > 50);
        icons.push(bytes.into_owned());
    }
    assert_ne!(icons[0], icons[1]);
}

#[gpui_kit::test]
fn feedback_markdown_row_click_pastes_original_not_rendered_text(cx: &mut TestAppContext) {
    let f = fixture(cx);
    let mut surface = VisualTestContext::from_window(f.window, cx);
    check_markdown_body_click(
        f,
        &mut surface,
        "# Heading\n\n**Original** with `source`\n\n- Item TAIL",
    );
}

#[gpui_kit::test]
fn feedback_markdown_link_click_keeps_the_history_paste_owner(cx: &mut TestAppContext) {
    let f = fixture(cx);
    let mut surface = VisualTestContext::from_window(f.window, cx);
    check_markdown_body_click(
        f,
        &mut surface,
        "# [Heading](https://example.invalid/no-navigation)\n\n**Original** TAIL",
    );
}

fn check_markdown_body_click(f: Fixture, surface: &mut VisualTestContext, source: &str) {
    let id = f
        .panel
        .read_with(surface, |panel, _| panel.entries.borrow()[0].id);
    let original = source.to_owned();
    {
        let mut history =
            nxtools_clipboard::ClipboardHistory::open(f._directory.path().join("clipboard.db"))
                .expect("fixture DB");
        history
            .update_text(id, &original)
            .expect("synthetic Markdown");
    }
    f.panel.update_in(surface, |panel, _, cx| {
        panel.entries.borrow_mut()[0].text = original.clone();
        panel.list_scroll.reset(20);
        cx.notify();
    });
    surface.refresh().expect("Markdown row");
    let row = surface
        .debug_bounds("history-row-slot-0")
        .expect("first row");
    surface.simulate_click(
        point(row.origin.x + px(28.0), row.origin.y + px(52.0)),
        Modifiers::default(),
    );
    let mut received = None;
    for _ in 0..200 {
        surface.run_until_parked();
        if let Ok(envelope) = f.endpoint.try_receive_command_for_contract_test() {
            received = Some(envelope);
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    match received.expect("Markdown body click still pastes").message {
        nxtools_protocol::Message::PlatformCommand(
            nxtools_protocol::PlatformCommand::PasteText(command),
        ) => assert_eq!(command.text, original),
        _ => panic!("must paste original text"),
    }
}

#[gpui_kit::test]
fn feedback_nonempty_search_digits_are_text_not_quick_paste(cx: &mut TestAppContext) {
    let f = fixture(cx);
    let mut surface = VisualTestContext::from_window(f.window, cx);
    f.panel.update_in(&mut surface, |panel, window, cx| {
        panel
            .search_input
            .update(cx, |input, cx| input.set_value("record", window, cx));
    });
    surface.run_until_parked();
    f.panel
        .update_in(&mut surface, |panel, _, _| panel.search_pending = false);
    press(&mut surface, "2");
    assert!(
        f.panel
            .read_with(&surface, |panel, _| panel.pending_action.is_none())
    );
    surface.simulate_input("2");
    assert!(f.panel.read_with(&surface, |panel, cx| {
        panel.pending_action.is_none() && panel.search_input.read(cx).value().ends_with('2')
    }));
    assert!(f.endpoint.try_receive_command_for_contract_test().is_err());
}

#[gpui_kit::test]
fn feedback_pause_pending_blocks_duplicates_and_failure_retries_same_intent(
    cx: &mut TestAppContext,
) {
    let f = fixture(cx);
    let mut surface = VisualTestContext::from_window(f.window, cx);
    let toggle = surface
        .debug_bounds("clipboard-capture-pause")
        .expect("toggle");
    surface.simulate_click(toggle.center(), Modifiers::default());
    surface.run_until_parked();
    surface.refresh().expect("pending icon");
    let first = f
        .endpoint
        .try_receive_command_for_contract_test()
        .expect("pause request");
    surface.simulate_click(toggle.center(), Modifiers::default());
    surface.run_until_parked();
    assert!(f.endpoint.try_receive_command_for_contract_test().is_err());
    f.application.update(&mut surface, |owner, cx| {
        owner.handle_platform_notification(
            nxtools_ipc::PlatformNotification::CommandResult {
                correlation_id: first.id,
                result: PlatformCommandResult {
                    success: false,
                    error: None,
                    native_error_code: Some(5),
                    data: None,
                },
            },
            cx,
        )
    });
    let failed = f
        .application
        .read_with(&surface, |owner, _| owner.capture_pause_snapshot());
    assert!(failed.failed && !failed.pending && failed.requested);
    f.panel.update_in(&mut surface, |panel, _, cx| {
        panel.update_capture_pause(failed, cx)
    });
    surface.refresh().expect("failure stays visible");
    assert!(surface.debug_bounds("clipboard-capture-failure").is_some());
    let retry = surface
        .debug_bounds("clipboard-capture-pause")
        .expect("same retry icon");
    surface.simulate_click(retry.center(), Modifiers::default());
    surface.run_until_parked();
    let retry = f
        .endpoint
        .try_receive_command_for_contract_test()
        .expect("retry admission");
    assert!(matches!(
        retry.message,
        nxtools_protocol::Message::PlatformCommand(
            nxtools_protocol::PlatformCommand::SetClipboardCapturePaused(
                nxtools_protocol::SetClipboardCapturePausedCommand { paused: true }
            )
        )
    ));
    assert_ne!(first.id, retry.id);
}

thread_local! {
    static HEADER_DRAG_PRESSES: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}
pub(super) fn record_header_drag_press() {
    HEADER_DRAG_PRESSES.with(|count| count.set(count.get() + 1));
}

#[gpui_kit::test]
fn feedback_pause_icon_press_never_enters_header_window_drag(cx: &mut TestAppContext) {
    let f = fixture(cx);
    let mut surface = VisualTestContext::from_window(f.window, cx);
    HEADER_DRAG_PRESSES.with(|count| count.set(0));
    let icon = surface
        .debug_bounds("clipboard-capture-pause")
        .expect("header icon");
    surface.simulate_click(icon.center(), Modifiers::default());
    assert_eq!(
        HEADER_DRAG_PRESSES.with(std::cell::Cell::get),
        0,
        "a pause button press must never reach native header drag"
    );
    let header = surface
        .debug_bounds("clipboard-panel-header")
        .expect("header");
    surface.simulate_click(
        point(header.origin.x + px(5.0), header.center().y),
        Modifiers::default(),
    );
    assert_eq!(
        HEADER_DRAG_PRESSES.with(std::cell::Cell::get),
        1,
        "header space retains the original move gesture"
    );
}

#[gpui_kit::test]
fn height_feedback_history_multiline_rows_grow_with_rendered_content(cx: &mut TestAppContext) {
    let f = fixture(cx);
    let mut surface = VisualTestContext::from_window(f.window, cx);
    let mut heights = vec![];
    for source in [
        "第一行\n\n最后一行",
        "第一行\n\n第二段\n\n第三段\n\n第四段\n\n最后一行",
    ] {
        f.panel.update_in(&mut surface, |panel, _, cx| {
            panel.entries.borrow_mut()[0].text = source.into();
            panel.list_scroll.reset(20);
            cx.notify();
        });
        surface.refresh().expect("production history layout");
        heights.push(
            surface
                .debug_bounds("history-row-slot-0")
                .expect("row")
                .size
                .height,
        );
        let id = f
            .panel
            .read_with(&surface, |panel, _| panel.entries.borrow()[0].id);
        let rich = history::RENDERED_MARKDOWN
            .with_borrow(|rows| rows[&id].upgrade().expect("mounted Markdown"));
        let natural_bottom = rich.read_with(&surface, |state, _| state.bounds().bottom());
        let row = surface.debug_bounds("history-row-slot-0").expect("row");
        assert!(
            natural_bottom <= row.bottom() - px(history::ROW_GAP),
            "rendered final paragraph fits its row"
        );
    }
    assert!(
        heights[1] > heights[0] + px(24.0),
        "actual paragraphs must grow the row: {heights:?}"
    );
}

#[gpui_kit::test]
fn height_feedback_long_links_reflow_when_history_width_changes(cx: &mut TestAppContext) {
    let f = fixture(cx);
    let mut surface = VisualTestContext::from_window(f.window, cx);
    f.panel.update_in(&mut surface, |panel, _, cx| {
        panel.entries.borrow_mut()[0].text = "价格：https://example.invalid/product/12345678901234567890\n\n便宜的：https://example.invalid/another-long-product\n\n最后一行不会消失".into();
        panel.list_scroll.reset(20); cx.notify();
    });
    let mut heights = vec![];
    for width in [460.0, 280.0] {
        surface.simulate_resize(size(px(width), px(440.0)));
        surface.refresh().expect("actual width reflow");
        let row = surface.debug_bounds("history-row-slot-0").expect("row");
        heights.push(row.size.height);
        let id = f
            .panel
            .read_with(&surface, |panel, _| panel.entries.borrow()[0].id);
        let state =
            history::RENDERED_MARKDOWN.with_borrow(|rows| rows[&id].upgrade().expect("state"));
        state.read_with(&surface, |state, _| {
            if row.size.height < px(history::TEXT_CARD_MAX_HEIGHT + history::ROW_GAP) {
                assert!(state.bounds().bottom() <= row.bottom() - px(history::ROW_GAP));
            } else {
                assert_eq!(row.size.height, px(218.0));
                assert!(state.bounds().bottom() > row.bottom() - px(history::ROW_GAP));
            }
            assert!(state.rendered_text().as_str().contains("最后一行"));
        });
    }
    assert!(
        heights[1] > heights[0],
        "narrower text must reflow to a taller cell: {heights:?}"
    );
}

#[gpui_kit::test]
fn style_feedback_cells_have_real_gap_and_visible_official_boundary(cx: &mut TestAppContext) {
    let f = fixture(cx);
    let mut surface = VisualTestContext::from_window(f.window, cx);
    f.panel.update_in(&mut surface, |panel, _, cx| {
        for (index, item) in panel.entries.borrow_mut().iter_mut().enumerate() {
            item.text = format!("短记录 {index}");
        }
        panel.list_scroll.reset(20);
        cx.notify();
    });
    surface.refresh().expect("two adjacent short cells");
    let first_id = f
        .panel
        .read_with(&surface, |panel, _| panel.entries.borrow()[0].id);
    let slot = surface
        .debug_bounds("history-row-slot-0")
        .expect("first slot");
    let next = surface
        .debug_bounds("history-row-slot-1")
        .expect("next slot");
    let card = surface.update(|window, _| {
        let card = gpui_kit::base::test_support::snapshots(window)
            .into_iter()
            .find(|node| node.label() == Some(format!("剪贴板记录 {first_id}").as_str()))
            .expect("real official ListItem")
            .bounds();
        let scale = window.scale_factor();
        assert!(
            window.painted_quads().iter().any(|quad| {
                (quad.bounds.size.width.0 / scale - card.size.width.as_f32()).abs() < 1.0
                    && (quad.bounds.size.height.0 / scale - card.size.height.as_f32()).abs() < 1.0
                    && quad.border_widths.top.0 > 0.0
                    && quad.border_color.a > 0.0
            }),
            "official card paints its own visible boundary"
        );
        card
    });
    assert!(slot.bottom() - card.bottom() >= px(8.0));
    assert!(next.top() - card.bottom() >= px(8.0));
    surface.simulate_click(
        point(card.center().x, card.bottom() + px(4.0)),
        Modifiers::default(),
    );
    assert!(
        f.panel
            .read_with(&surface, |panel, _| panel.pending_action.is_none()),
        "spacing cannot paste"
    );
}

#[gpui_kit::test]
fn style_feedback_neutral_fallback_is_type_independent_and_preserves_click(
    cx: &mut TestAppContext,
) {
    let f = fixture(cx);
    let mut surface = VisualTestContext::from_window(f.window, cx);
    let mut colors = vec![];
    for kind in [
        ClipboardKind::Text,
        ClipboardKind::Image,
        ClipboardKind::Files,
    ] {
        f.panel.update_in(&mut surface, |panel, _, cx| {
            panel.entries.borrow_mut()[0].kind = kind;
            panel.list_scroll.reset(20);
            cx.notify();
        });
        surface.refresh().expect("type tint render");
        let source = surface
            .debug_bounds("clipboard-source-0")
            .expect("source rail");
        if kind != ClipboardKind::Image {
            assert!(
                source.bottom()
                    <= surface
                        .debug_bounds("clipboard-history-content")
                        .expect("body")
                        .top()
            );
        }
        colors.push(surface.update(|window, _| {
            let scale = window.scale_factor();
            window
                .painted_quads()
                .into_iter()
                .find(|quad| {
                    quad.background.as_solid().is_none()
                        && !quad.background.is_transparent()
                        && (quad.bounds.origin.x.0 / scale - source.origin.x.as_f32()).abs() < 1.0
                        && (quad.bounds.size.width.0 / scale - source.size.width.as_f32()).abs()
                            < 1.0
                        && quad.bounds.size.height.0 / scale <= 25.0
                })
                .expect("actual localized gradient")
                .background
        }));
    }
    assert!(
        colors.windows(2).all(|pair| pair[0] == pair[1]),
        "missing source icon must not invent a type palette"
    );
    for background in &colors {
        let serialized = serde_json::to_value(background).expect("typed GPUI gradient");
        assert_eq!(serialized["gradient_angle_or_pattern_height"], 90.0);
        assert_eq!(serialized["colors"][0]["percentage"], 0.0);
        assert_eq!(serialized["colors"][1]["percentage"], 1.0);
        let from: gpui_kit::Hsla = serde_json::from_value(serialized["colors"][0]["color"].clone())
            .expect("gradient start");
        let to: gpui_kit::Hsla =
            serde_json::from_value(serialized["colors"][1]["color"].clone()).expect("gradient end");
        assert!(
            from.a > 0.0 && to.a == 0.0,
            "solid tint must fade to transparent"
        );
    }
    f.panel.update_in(&mut surface, |panel, _, cx| {
        panel.entries.borrow_mut()[0].kind = ClipboardKind::Text;
        panel.list_scroll.reset(20);
        cx.notify();
    });
    surface.refresh().expect("original text entry");
    let source = surface
        .debug_bounds("clipboard-source-0")
        .expect("icon rail");
    surface.simulate_click(
        point(source.left() + px(8.0), source.center().y),
        Modifiers::default(),
    );
    assert_number_paste(&f, &mut surface, 0);
}

#[gpui_kit::test]
fn style_feedback_fade_and_card_keep_theme_selection_states(cx: &mut TestAppContext) {
    let f = fixture(cx);
    let mut surface = VisualTestContext::from_window(f.window, cx);
    let first_id = f
        .panel
        .read_with(&surface, |panel, _| panel.entries.borrow()[0].id);
    let mut theme_fades = vec![];
    for preference in [
        nxtools_core::ThemePreference::Dark,
        nxtools_core::ThemePreference::Light,
    ] {
        surface.update(|_, cx| crate::theme::set_theme_preference(preference, cx));
        f.panel.update_in(&mut surface, |panel, _, cx| {
            panel.entries.borrow_mut()[0].text = "短记录".into();
            panel.selected_entry_index = None;
            panel.list_scroll.reset(20);
            cx.notify();
        });
        surface.update(|window, cx| {
            window.refresh();
            window.draw(cx).clear(cx);
        });
        let source = surface.debug_bounds("clipboard-source-0").expect("source");
        let tint = surface.update(|window, _| {
            let scale = window.scale_factor();
            window
                .painted_quads()
                .into_iter()
                .find(|quad| {
                    quad.background.as_solid().is_none()
                        && (quad.bounds.origin.x.0 / scale - source.left().as_f32()).abs() < 1.0
                        && (quad.bounds.size.width.0 / scale - source.size.width.as_f32()).abs()
                            < 1.0
                        && quad.bounds.size.height.0 / scale <= 25.0
                })
                .expect("localized theme gradient")
                .background
        });
        theme_fades.push(tint);
        let idle = painted_history_card(&mut surface, first_id);
        f.panel.update_in(&mut surface, |panel, _, cx| {
            panel.selected_entry_index = Some(0);
            cx.notify();
        });
        surface.update(|window, cx| {
            window.refresh();
            window.draw(cx).clear(cx);
        });
        let selected = painted_history_card(&mut surface, first_id);
        assert_ne!(
            idle, selected,
            "official selection remains distinct in {preference:?}"
        );
    }
    assert_ne!(
        theme_fades[0], theme_fades[1],
        "gradient resolves current theme"
    );
}

fn painted_history_card(surface: &mut VisualTestContext, item_id: i64) -> gpui_kit::Background {
    surface.update(|window, _| {
        let label = format!("剪贴板记录 {item_id}");
        let card = gpui_kit::base::test_support::snapshots(window)
            .into_iter()
            .find(|node| node.label() == Some(label.as_str()))
            .expect("official item")
            .bounds();
        let scale = window.scale_factor();
        window
            .painted_quads()
            .into_iter()
            .find(|quad| {
                quad.background
                    .as_solid()
                    .is_some_and(|color| color.a > 0.0)
                    && (quad.bounds.origin.x.0 / scale - card.left().as_f32()).abs() < 1.0
                    && (quad.bounds.origin.y.0 / scale - card.top().as_f32()).abs() < 1.0
                    && (quad.bounds.size.width.0 / scale - card.size.width.as_f32()).abs() < 1.0
                    && (quad.bounds.size.height.0 / scale - card.size.height.as_f32()).abs() < 1.0
            })
            .unwrap_or_else(|| {
                panic!(
                    "no painted card fill: {card:?}; matching={:?}",
                    window
                        .painted_quads()
                        .iter()
                        .filter(
                            |quad| (quad.bounds.origin.y.0 / scale - card.top().as_f32()).abs()
                                < 2.0
                        )
                        .map(|quad| (quad.bounds, quad.background, quad.border_widths))
                        .collect::<Vec<_>>()
                )
            })
            .background
    })
}

fn painted_source_gradient(surface: &mut VisualTestContext) -> gpui_kit::Hsla {
    let source = surface
        .debug_bounds("clipboard-source-0")
        .expect("source rail");
    let background = surface.update(|window, _| {
        let scale = window.scale_factor();
        window
            .painted_quads()
            .into_iter()
            .find(|quad| {
                quad.background.as_solid().is_none()
                    && !quad.background.is_transparent()
                    && (quad.bounds.origin.x.0 / scale - source.left().as_f32()).abs() < 1.0
                    && (quad.bounds.size.width.0 / scale - source.size.width.as_f32()).abs() < 1.0
                    && quad.bounds.size.height.0 / scale <= 25.0
            })
            .expect("actual source gradient")
            .background
    });
    let value = serde_json::to_value(background).expect("gradient metadata");
    serde_json::from_value(value["colors"][0]["color"].clone()).expect("gradient color")
}

#[gpui_kit::test]
fn icon_palette_source_application_not_record_type_controls_rendered_hue(cx: &mut TestAppContext) {
    let f = fixture(cx);
    let mut surface = VisualTestContext::from_window(f.window, cx);
    let root = f._directory.path().join("images");
    let green = root.join(format!("source-{}.png", "c".repeat(64)));
    let blue = root.join(format!("source-{}.png", "d".repeat(64)));
    image::RgbaImage::from_pixel(32, 32, image::Rgba([20, 180, 70, 255]))
        .save(&green)
        .expect("green app icon");
    image::RgbaImage::from_pixel(32, 32, image::Rgba([30, 75, 220, 255]))
        .save(&blue)
        .expect("blue app icon");
    let mut same_icon = vec![];
    for kind in [
        ClipboardKind::Text,
        ClipboardKind::Image,
        ClipboardKind::Files,
    ] {
        f.panel.update_in(&mut surface, |panel, _, cx| {
            panel.entries.borrow_mut()[0].source.process_icon_path =
                green.to_string_lossy().into_owned();
            panel.entries.borrow_mut()[0].kind = kind;
            panel.list_scroll.reset(20);
            cx.notify();
        });
        let mut color = gpui_kit::Hsla::default();
        for _ in 0..120 {
            surface.run_until_parked();
            surface.refresh().expect("palette frame");
            color = painted_source_gradient(&mut surface);
            let rgb = color.to_rgb();
            if rgb.g > rgb.r * 2.0 && rgb.g > rgb.b * 1.5 {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        let rgb = color.to_rgb();
        assert!(
            rgb.g > rgb.r * 2.0 && rgb.g > rgb.b * 1.5,
            "green application icon must produce green regardless of {kind:?}: {rgb:?}"
        );
        same_icon.push(color);
    }
    assert!(same_icon.windows(2).all(|pair| pair[0] == pair[1]));
    f.panel.update_in(&mut surface, |panel, _, cx| {
        panel.entries.borrow_mut()[0].source.process_icon_path =
            blue.to_string_lossy().into_owned();
        panel.prune_list_metadata();
        panel.list_scroll.reset(20);
        cx.notify();
    });
    let mut changed = gpui_kit::Hsla::default();
    for _ in 0..120 {
        surface.run_until_parked();
        surface.refresh().expect("new app icon palette");
        changed = painted_source_gradient(&mut surface);
        let rgb = changed.to_rgb();
        if rgb.b > rgb.r * 2.0 && rgb.b > rgb.g * 1.5 {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    let rgb = changed.to_rgb();
    assert!(
        rgb.b > rgb.r * 2.0 && rgb.b > rgb.g * 1.5,
        "changed app icon must produce blue"
    );
    f.panel.read_with(&surface, |panel, _| {
        assert!(
            !panel
                .source_icon_colors
                .contains_key(green.to_str().expect("fixture path"))
        );
        assert_eq!(panel.source_icon_colors.len(), 1);
        assert_eq!(
            panel
                .source_icon_colors
                .get(blue.to_str().expect("fixture path")),
            Some(&Some([30, 75, 220]))
        );
    });
}

#[gpui_kit::test]
fn icon_palette_late_reply_cannot_restore_retired_source_metadata(cx: &mut TestAppContext) {
    let f = fixture(cx);
    let mut surface = VisualTestContext::from_window(f.window, cx);
    let path = f
        ._directory
        .path()
        .join("images")
        .join(format!("source-{}.png", "e".repeat(64)));
    image::RgbaImage::from_pixel(32, 32, image::Rgba([20, 180, 70, 255]))
        .save(&path)
        .expect("app icon");
    let key = path.to_string_lossy().into_owned();
    f.panel.update_in(&mut surface, |panel, _, cx| {
        panel.entries.borrow_mut()[0].source.process_icon_path = key.clone();
        panel.request_source_icon_colors(vec![key.clone()], cx);
        panel.query_dispatch_generation += 1;
        panel.entries.borrow_mut()[0]
            .source
            .process_icon_path
            .clear();
        panel.prune_list_metadata();
    });
    for _ in 0..120 {
        surface.run_until_parked();
        if f.panel
            .read_with(&surface, |panel, _| !panel.icon_colors_pending)
        {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    f.panel.read_with(&surface, |panel, _| {
        assert!(!panel.icon_colors_pending);
        assert!(!panel.source_icon_colors.contains_key(&key));
    });
}

#[gpui_kit::test]
fn text_cap_whole_text_card_is_at_most_210_and_short_content_stays_natural(
    cx: &mut TestAppContext,
) {
    let f = fixture(cx);
    let mut surface = VisualTestContext::from_window(f.window, cx);
    let first_id = f
        .panel
        .read_with(&surface, |panel, _| panel.entries.borrow()[0].id);
    for preference in [
        nxtools_core::ThemePreference::Dark,
        nxtools_core::ThemePreference::Light,
    ] {
        surface.update(|_, cx| crate::theme::set_theme_preference(preference, cx));
        for width in [280.0, 460.0] {
            surface.simulate_resize(size(px(width), px(440.0)));
            let mut heights = Vec::new();
            for source in ["短记录".to_owned(), "段落内容\n\n".repeat(24)] {
                f.panel.update_in(&mut surface, |panel, _, cx| {
                    panel.entries.borrow_mut()[0].text = source;
                    panel.list_scroll.reset(20);
                    cx.notify();
                });
                surface.refresh().expect("actual card layout");
                let card = surface.update(|window, _| {
                    let label = format!("剪贴板记录 {first_id}");
                    gpui_kit::base::test_support::snapshots(window)
                        .into_iter()
                        .find(|node| node.label() == Some(label.as_str()))
                        .expect("official ListItem card")
                        .bounds()
                });
                assert!(
                    card.size.height <= px(210.0),
                    "whole text card exceeds210DIP at{width}/{preference:?}: {card:?}"
                );
                let slot = surface.debug_bounds("history-row-slot-0").expect("slot");
                assert!(slot.size.height <= px(218.0));
                assert!((slot.bottom() - card.bottom() - px(8.0)).abs() < px(0.1));
                let header = surface
                    .debug_bounds("clipboard-source-0")
                    .expect("source header");
                assert!(header.top() >= card.top() && header.bottom() <= card.bottom());
                assert_eq!(header.size.height, px(24.0));
                heights.push(card.size.height);
            }
            assert!(
                heights[0] < px(120.0),
                "short content must not fill the maximum"
            );
            assert_eq!(heights[1], px(210.0), "long content reaches the card cap");
        }
    }
}

#[gpui_kit::test]
fn text_cap_clipped_summary_click_still_pastes_the_complete_original(cx: &mut TestAppContext) {
    let f = fixture(cx);
    let mut surface = VisualTestContext::from_window(f.window, cx);
    check_markdown_body_click(
        f,
        &mut surface,
        &format!(
            "# Heading\n\n{}ORIGINAL_FULL_TAIL",
            "**完整原文**\n\n".repeat(100)
        ),
    );
}

#[gpui_kit::test]
fn text_cap_preserves_existing_image_and_file_card_heights(cx: &mut TestAppContext) {
    let f = fixture(cx);
    let mut surface = VisualTestContext::from_window(f.window, cx);
    for (kind, expected) in [(ClipboardKind::Image, 190.0), (ClipboardKind::Files, 62.0)] {
        f.panel.update_in(&mut surface, |panel, _, cx| {
            let mut items = panel.entries.borrow_mut();
            items[0].kind = kind;
            items[0].image_width = 1000;
            items[0].image_height = 2000;
            items[0].file_paths = vec!["synthetic-long-file-name.txt".repeat(20)];
            drop(items);
            panel.list_scroll.reset(20);
            cx.notify();
        });
        surface.refresh().expect("nontext card layout");
        let slot = surface.debug_bounds("history-row-slot-0").expect("slot");
        assert_eq!(slot.size.height, px(expected + 8.0));
    }
}

#[gpui_kit::test]
fn input_repair_ctrl_digit_with_no_focused_descendant_starts_exact_paste(cx: &mut TestAppContext) {
    let f = fixture(cx);
    let mut surface = VisualTestContext::from_window(f.window, cx);
    surface.update(|window, cx| window.blur(cx));
    press(&mut surface, "ctrl-2");
    assert_number_paste(&f, &mut surface, 1);
}

#[gpui_kit::test]
fn input_repair_typing_from_header_control_goes_to_search(cx: &mut TestAppContext) {
    let f = fixture(cx);
    let mut surface = VisualTestContext::from_window(f.window, cx);
    surface.update(|window, cx| window.focus_next(cx));
    surface.simulate_keystrokes("a");
    f.panel.update_in(&mut surface, |panel, window, cx| {
        assert_eq!(panel.search_input.read(cx).value().as_str(), "a");
        assert!(panel.search_input.focus_handle(cx).is_focused(window));
        assert!(panel.is_open);
    });
}

#[gpui_kit::test]
fn input_repair_pause_click_retires_pending_hover_preview(cx: &mut TestAppContext) {
    let f = fixture(cx);
    let mut surface = VisualTestContext::from_window(f.window, cx);
    let row = surface
        .debug_bounds("history-row-slot-0")
        .expect("first row");
    surface.simulate_mouse_move(
        point(row.left() + px(25.0), row.top() + px(48.0)),
        None,
        Modifiers::default(),
    );
    surface.run_until_parked();
    let pause = surface
        .debug_bounds("clipboard-capture-pause")
        .expect("pause");
    surface.simulate_click(pause.center(), Modifiers::default());
    surface.run_until_parked();
    // A later hover timer must have no right to open a preview after a header action.
    surface
        .executor()
        .advance_clock(Duration::from_millis(1000));
    surface.run_until_parked();
    for _ in 0..40 {
        surface.run_until_parked();
        std::thread::sleep(Duration::from_millis(2));
    }
    assert!(
        f.application
            .read_with(&surface, |owner, _| !owner.has_pending_preview_for_test())
    );
}

fn seed_pending_preview(f: &Fixture, surface: &mut VisualTestContext) {
    let item = f
        .panel
        .read_with(surface, |panel, _| panel.entries.borrow()[0].clone());
    let bounds = surface.update(|window, _| window.bounds());
    f.application.update(surface, |owner, cx| {
        owner.schedule_preview(true, item, bounds, point(px(40.0), px(140.0)), 1.0, cx);
    });
    assert!(
        f.application
            .read_with(surface, |owner, _| owner.has_pending_preview_for_test())
    );
}

#[gpui_kit::test]
fn input_repair_pause_retires_a_queued_hover_candidate_immediately(cx: &mut TestAppContext) {
    let f = fixture(cx);
    let mut surface = VisualTestContext::from_window(f.window, cx);
    let pause = surface
        .debug_bounds("clipboard-capture-pause")
        .expect("pause");
    surface.simulate_mouse_move(pause.center(), None, Modifiers::default());
    seed_pending_preview(&f, &mut surface);
    surface.simulate_click(pause.center(), Modifiers::default());
    surface.run_until_parked();
    assert!(
        !f.application
            .read_with(&surface, |owner, _| owner.has_pending_preview_for_test())
    );
}

#[gpui_kit::test]
fn input_repair_number_paste_retires_a_queued_hover_candidate(cx: &mut TestAppContext) {
    let f = fixture(cx);
    let mut surface = VisualTestContext::from_window(f.window, cx);
    seed_pending_preview(&f, &mut surface);
    press(&mut surface, "ctrl-2");
    assert_number_paste(&f, &mut surface, 1);
    assert!(
        !f.application
            .read_with(&surface, |owner, _| owner.has_pending_preview_for_test())
    );
}

#[gpui_kit::test]
fn input_repair_native_modifier_sequence_pastes_once_without_typing_a_digit(
    cx: &mut TestAppContext,
) {
    let f = fixture(cx);
    let mut surface = VisualTestContext::from_window(f.window, cx);
    surface.update(|window, cx| window.blur(cx));
    surface.simulate_event(gpui_kit::ModifiersChangedEvent {
        modifiers: Modifiers {
            control: true,
            ..Default::default()
        },
        capslock: gpui_kit::Capslock { on: false },
    });
    let key = gpui_kit::Keystroke {
        key: "2".into(),
        key_char: None,
        modifiers: Modifiers {
            control: true,
            ..Default::default()
        },
    };
    surface.simulate_event(KeyDownEvent {
        keystroke: key.clone(),
        is_held: false,
        prefer_character_input: false,
    });
    surface.simulate_event(KeyDownEvent {
        keystroke: key,
        is_held: true,
        prefer_character_input: false,
    });
    surface.simulate_event(gpui_kit::ModifiersChangedEvent {
        modifiers: Modifiers::default(),
        capslock: gpui_kit::Capslock { on: false },
    });
    surface.simulate_event(KeyUpEvent {
        keystroke: gpui_kit::Keystroke {
            key: "2".into(),
            key_char: Some("2".into()),
            modifiers: Modifiers::default(),
        },
    });
    assert_number_paste(&f, &mut surface, 1);
    f.panel.read_with(&surface, |panel, cx| {
        assert!(panel.search_input.read(cx).value().is_empty())
    });
    assert!(f.endpoint.try_receive_command_for_contract_test().is_err());
}

#[gpui_kit::test]
fn input_repair_unicode_ime_from_control_delegates_to_original_search_state(
    cx: &mut TestAppContext,
) {
    use gpui_kit::InputHandler as _;
    let f = fixture(cx);
    let mut surface = VisualTestContext::from_window(f.window, cx);
    surface.update(|window, cx| window.focus_next(cx));
    let mut fallback = surface.update(|_, cx| {
        let input = f.panel.read(cx).search_input.clone();
        super::search_input::SearchInputFallback::new(
            input.read(cx).input_bounds(),
            input.clone(),
            f.panel.downgrade(),
        )
    });
    surface.update(|window, cx| {
        fallback.replace_and_mark_text_in_range(None, "中文", Some(2..2), window, cx);
        assert!(fallback.marked_text_range(window, cx).is_some());
        assert!(
            f.panel
                .read(cx)
                .search_input
                .focus_handle(cx)
                .is_focused(window)
        );
        fallback.replace_text_in_range(None, "中文😀", window, cx);
    });
    f.panel.read_with(&surface, |panel, cx| {
        assert_eq!(panel.search_input.read(cx).value().as_str(), "中文😀");
        assert!(panel.is_open && panel.pending_action.is_none());
    });
}

#[gpui_kit::test]
fn input_repair_activation_restores_search_but_editing_keeps_its_input(cx: &mut TestAppContext) {
    let f = fixture(cx);
    let mut surface = VisualTestContext::from_window(f.window, cx);
    surface.update(|window, cx| window.focus_next(cx));
    surface.deactivate_window();
    surface.update(|window, _| window.activate_window());
    surface.run_until_parked();
    f.panel.update_in(&mut surface, |panel, window, cx| {
        assert!(panel.search_input.focus_handle(cx).is_focused(window));
        let item = panel.entries.borrow()[0].clone();
        panel.summary_counts.remove(&item.id);
        panel.begin_edit(&item, window, cx);
    });
    surface.simulate_input("Q");
    f.panel.read_with(&surface, |panel, cx| {
        assert!(panel.markdown_edit_value(cx).contains('Q'));
        assert!(panel.search_input.read(cx).value().is_empty());
        assert!(panel.editing_item_id.is_some());
    });
}

#[gpui_kit::test]
fn markdown_unified_unchanged_save_and_cancel_preserve_original_bytes(cx: &mut TestAppContext) {
    let f = fixture(cx);
    let mut surface = VisualTestContext::from_window(f.window, cx);
    let mut item = f
        .panel
        .read_with(&surface, |panel, _| panel.entries.borrow()[0].clone());
    item.text = "# Heading\r\n\r\n- one\r\n- two\r\n\r\n\r\n".into();
    {
        let mut history =
            nxtools_clipboard::ClipboardHistory::open(f._directory.path().join("clipboard.db"))
                .expect("temporary history");
        history
            .update_text(item.id, &item.text)
            .expect("seed exact bytes");
    }
    f.panel.update_in(&mut surface, |panel, window, cx| {
        panel.summary_counts.remove(&item.id);
        panel.begin_edit(&item, window, cx);
        assert!(!panel.edit_source_mode);
        assert_eq!(panel.markdown_edit_value(cx), item.text);
        panel.save_edit(window, cx);
        assert!(panel.editing_item_id.is_none());
    });
    surface.run_until_parked();
    f.panel.update_in(&mut surface, |panel, window, cx| {
        panel.begin_edit(&item, window, cx)
    });
    surface.refresh().expect("interactive document");
    surface.simulate_keystrokes("ctrl-a");
    surface.simulate_input("changed draft");
    surface.simulate_keystrokes("escape");
    let history =
        nxtools_clipboard::ClipboardHistory::open(f._directory.path().join("clipboard.db"))
            .expect("readback");
    assert_eq!(
        history.active_item(item.id).expect("active").text,
        item.text
    );
}

#[gpui_kit::test]
fn markdown_unified_editor_ime_and_undo_keep_search_and_original_payload(cx: &mut TestAppContext) {
    let f = fixture(cx);
    let mut surface = VisualTestContext::from_window(f.window, cx);
    let item = f
        .panel
        .read_with(&surface, |panel, _| panel.entries.borrow()[0].clone());
    f.panel.update_in(&mut surface, |panel, window, cx| {
        panel.summary_counts.remove(&item.id);
        panel.begin_edit(&item, window, cx);
    });
    surface.refresh().expect("editor keys and input handler");
    surface.simulate_keystrokes("ctrl-a");
    surface.simulate_input("alpha");
    surface.simulate_keystrokes("ctrl-b");
    surface.simulate_input("bold");
    assert!(
        f.panel
            .read_with(&surface, |panel, cx| panel.markdown_edit_value(cx))
            .contains("**bold**")
    );
    surface.simulate_keystrokes("ctrl-z");
    assert!(
        !f.panel
            .read_with(&surface, |panel, cx| panel.markdown_edit_value(cx))
            .contains("bold")
    );
    f.panel.update_in(&mut surface, |panel, window, cx| {
        panel.edit_rich.update(cx, |editor, cx| {
            editor.replace_and_mark_text_in_range(None, "中文", Some(2..2), window, cx);
            assert!(editor.is_composing());
        });
        panel.save_edit(window, cx);
        assert!(panel.editing_item_id.is_some(), "composition must not save");
        panel
            .edit_rich
            .update(cx, |editor, cx| editor.unmark_text(window, cx));
        assert!(panel.search_input.read(cx).value().is_empty());
        panel.cancel_edit(window, cx);
    });
}
