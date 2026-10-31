//! Audit #10: actual panel/ListState/worker paging and retained keyboard identity.
use super::*;
use gpui_kit::base::{Root, test_support};
use gpui_kit::{
    ElementId, FocusHandle, KeyDownEvent, KeyUpEvent, Keystroke, TestAppContext, VisualTestContext,
};

struct IdentityFixture {
    panel: Entity<ClipboardPanel>,
    _application: Entity<ClipboardApplication>,
    endpoint: nxtools_ipc::PlatformServerEndpoint,
    worker: crate::clipboard_worker::ClipboardCaptureWorkerHandle,
    window: gpui_kit::AnyWindowHandle,
    directory: tempfile::TempDir,
    item_id: i64,
}

impl Drop for IdentityFixture {
    fn drop(&mut self) {
        self.worker
            .clone()
            .shutdown()
            .expect("stop isolated identity worker");
    }
}

fn fixture(cx: &mut TestAppContext) -> IdentityFixture {
    fixture_page(cx, 160, 80, 80, 0)
}

fn fixture_page(
    cx: &mut TestAppContext,
    total: i64,
    offset: usize,
    limit: usize,
    focus_index: usize,
) -> IdentityFixture {
    cx.update(gpui_kit::init);
    cx.executor().allow_parking();
    let directory = tempfile::tempdir().expect("identity temporary files");
    let database = directory.path().join("clipboard.db");
    let worker = crate::clipboard_worker::ClipboardCaptureWorkerHandle::spawn(
        database.clone(),
        directory.path().join("images"),
        nxtools_clipboard::ClipboardRetentionPolicy {
            max_items: 1000,
            retention_days: 30,
        },
        nxtools_ipc::unix_time_ms(),
    )
    .expect("initialize isolated worker before seeding");
    {
        let mut history =
            nxtools_clipboard::ClipboardHistory::open(&database).expect("seed identity history");
        for index in 0..total {
            history
                .capture(nxtools_clipboard::TextCapture {
                    text: format!("identity record {index:03} {} FULL-TAIL", "文".repeat(450)),
                    native_formats: vec![],
                    source: nxtools_clipboard::CaptureSource {
                        window_handle: 1,
                        process_id: 1,
                        process_name: "IdentityFixture.exe".into(),
                        process_icon_path: String::new(),
                        window_title: String::new(),
                    },
                    captured_at_unix_ms: nxtools_ipc::unix_time_ms() + index,
                })
                .expect("seed unique record");
        }
    }
    let page = nxtools_clipboard::ClipboardHistory::open(&database)
        .expect("read seed page")
        .summary_category_page_with_total(
            "",
            BUILTIN_ALL_CATEGORY_ID,
            ClipboardSearchScope::Content,
            offset,
            limit,
            None,
        )
        .expect("real initial history page");
    let item_id = page.items[focus_index].item.id;
    let counts = page
        .items
        .iter()
        .map(|summary| (summary.item.id, summary.text_character_count))
        .collect();
    let page = ClipboardHistoryPage {
        items: page.items.into_iter().map(|summary| summary.item).collect(),
        total: page.total,
        has_more: page.has_more,
    };
    let (platform, endpoint) = nxtools_ipc::platform_bridge(32);
    endpoint.set_connected_for_contract_test(true);
    let application = cx.update(|cx| {
        let notifications = cx.new(|_| crate::notifications::NotificationCenter::default());
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
    let weak = application.downgrade();
    let held = worker.clone();
    let slot = Rc::new(RefCell::new(None));
    let output = slot.clone();
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
                vec![],
                ClipboardModifierActions::default(),
                nxtools_protocol::ForegroundContext {
                    window_handle: 42,
                    focus_window_handle: 43,
                    process_id: 2,
                    process_name: "synthetic-target".into(),
                    window_title: String::new(),
                },
                held,
                weak,
                uuid::Uuid::new_v4(),
                None,
                window,
                cx,
            )
            .with_summary_counts(counts)
        });
        *output.borrow_mut() = Some(panel.clone());
        Root::new(panel, window, cx)
    });
    surface.simulate_resize(size(px(300.0), px(440.0)));
    surface.update(|window, _| window.activate_window());
    if focus_index > 0 {
        let panel = slot.borrow().clone().expect("scroll initial view");
        panel.update_in(surface, |panel, _, cx| {
            panel.list_scroll.scroll_to(gpui_kit::ListOffset {
                item_ix: focus_index,
                offset_in_item: px(0.0),
            });
            cx.notify();
        });
    }
    surface.refresh().expect("actual identity panel frame");
    let panel = slot.borrow().clone().expect("actual panel");
    let window = surface.update(|window, _| window.window_handle());
    IdentityFixture {
        panel,
        _application: application,
        endpoint,
        worker,
        window,
        directory,
        item_id,
    }
}

fn key(surface: &mut VisualTestContext, key: &str, down: bool) {
    let keystroke = Keystroke::parse(key).expect("identity key");
    if down {
        surface.simulate_event(KeyDownEvent {
            keystroke,
            is_held: false,
            prefer_character_input: false,
        });
    } else {
        surface.simulate_event(KeyUpEvent { keystroke });
    }
}

fn row_snapshot(surface: &mut VisualTestContext, item_id: i64) -> test_support::ElementSnapshot {
    let name = format!("剪贴板记录 {item_id}");
    surface
        .update(|window, _| test_support::snapshots(window))
        .into_iter()
        .find(|node| node.label() == Some(name.as_str()))
        .unwrap_or_else(|| {
            let labels = surface
                .update(|window, _| test_support::snapshots(window))
                .iter()
                .filter_map(|node| node.label().map(str::to_owned))
                .collect::<Vec<_>>();
            panic!("mounted actual item ListItem {item_id}, actual labels: {labels:?}");
        })
}

fn row_action_snapshot(
    surface: &mut VisualTestContext,
    item_id: i64,
    action: Option<&str>,
) -> test_support::ElementSnapshot {
    let row = row_snapshot(surface, item_id);
    surface.update(|window, _| test_support::snapshots(window)).into_iter()
        .find(|node| match action {
            None => row.path().starts_with(node.path()) && matches!(node.path().last(),
                Some(ElementId::NamedInteger(name, _)) if name.as_ref() == "history-row-action"),
            Some(action) => node.path().starts_with(row.path()) && matches!(node.path().last(),
                Some(ElementId::Name(name)) if name.starts_with("clipboard-item-action-") && name.ends_with(action)),
        }).expect("mounted actual row/action focus node")
}

fn focus_row_action(
    surface: &mut VisualTestContext,
    item_id: i64,
    action: Option<&str>,
) -> FocusHandle {
    let row = row_snapshot(surface, item_id);
    surface.simulate_mouse_move(
        point(
            row.bounds().left() + px(20.0),
            row.bounds().top() + px(12.0),
        ),
        None,
        gpui_kit::Modifiers::default(),
    );
    surface.refresh().expect("actual row actions rendered");
    let path = row_action_snapshot(surface, item_id, action)
        .path()
        .to_vec();
    surface.update(|window, cx| window.blur(cx));
    for _ in 0..30 {
        surface.update(|window, cx| window.focus_next(cx));
        surface.refresh().expect("real Tab focus frame");
        let focused = surface.update(|window, _| test_support::has_observed_focus(window, &path));
        if focused {
            return surface.update(|window, cx| window.focused(cx).expect("actual retained focus"));
        }
    }
    panic!("actual row/action was not reachable by Tab");
}

fn prepend(f: &IdentityFixture, surface: &mut VisualTestContext) {
    f.panel.update_in(surface, |panel, _, cx| {
        panel.window_start_offset = 80;
        panel.list_scroll.scroll_to(gpui_kit::ListOffset {
            item_ix: 0,
            offset_in_item: px(0.0),
        });
        panel.load_previous_page(cx);
    });
    for _ in 0..400 {
        surface.run_until_parked();
        if !f
            .panel
            .read_with(surface, |panel, _| panel.page_request_pending)
        {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    f.panel.read_with(surface, |panel, _| {
        assert!(
            !panel.page_request_pending,
            "original worker page callback completed"
        );
        assert_eq!(panel.window_start_offset, 0);
        assert_eq!(panel.entries.borrow()[80].id, f.item_id);
        assert_eq!(panel.entries.borrow().len(), 160);
    });
    f.panel.update_in(surface, |panel, _, cx| {
        panel.list_scroll.scroll_to(gpui_kit::ListOffset {
            item_ix: 80,
            offset_in_item: px(0.0),
        });
        cx.notify();
    });
    surface.refresh().expect("real ListState splice frame");
    surface.update(|window, cx| window.draw(cx).clear(cx));
}

#[gpui_kit::test]
fn history_identity_prepend_keeps_focused_record_and_enter_full_by_id(cx: &mut TestAppContext) {
    let f = fixture(cx);
    let mut surface = VisualTestContext::from_window(f.window, cx);
    let focus = focus_row_action(&mut surface, f.item_id, None);
    prepend(&f, &mut surface);
    assert_eq!(
        surface.update(|window, cx| window.focused(cx)),
        Some(focus),
        "same record retains its FocusHandle after prepend"
    );
    let owner = surface.update(|window, _| {
        test_support::snapshots(window).into_iter().filter(|node| node.label().is_some_and(|label| label.starts_with("剪贴板记录 ")))
            .find(|node| node.path().iter().position(|id| matches!(id, ElementId::NamedInteger(name, _) if name.as_ref() == "history-row-action"))
                .is_some_and(|at| test_support::has_observed_focus(window, &node.path()[..=at])))
            .and_then(|node| node.label().map(str::to_owned))
    });
    let diagnostic = surface.update(|window, _| test_support::snapshots(window).into_iter()
        .filter(|node| node.label().is_some_and(|label| label.starts_with("剪贴板记录 "))
            || matches!(node.path().last(), Some(ElementId::NamedInteger(name, _)) if name.as_ref() == "history-row-action"))
        .map(|node| format!("{node:?}")).collect::<Vec<_>>());
    let offset = f
        .panel
        .read_with(&surface, |panel, _| panel.list_scroll.logical_scroll_top());
    assert_eq!(
        owner,
        Some(format!("剪贴板记录 {}", f.item_id)),
        "retained focus A, offset={offset:?}, nodes={diagnostic:?}"
    );
    key(&mut surface, "enter", true);
    assert!(
        f.panel
            .read_with(&surface, |panel, _| panel.pending_action.is_none()),
        "row uses original release activation"
    );
    key(&mut surface, "enter", false);
    let operation = f.panel.read_with(&surface, |panel, _| {
        let (operation, _, item) = panel.pending_action.expect("actual row action pending");
        assert_eq!(item, f.item_id);
        operation
    });
    let mut command = None;
    for _ in 0..400 {
        surface.run_until_parked();
        if let Ok(envelope) = f.endpoint.try_receive_command_for_contract_test() {
            command = Some(envelope);
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    let command = command.expect("original typed paste boundary");
    assert_eq!(command.id, operation);
    let nxtools_protocol::Message::PlatformCommand(nxtools_protocol::PlatformCommand::PasteText(
        paste,
    )) = command.message
    else {
        panic!("actual text paste command");
    };
    assert!(
        paste.text.ends_with("FULL-TAIL") && paste.text.chars().count() > 450,
        "full-by-ID payload, not bounded summary"
    );
    assert!(paste.text.starts_with("identity record 079"));
}

#[gpui_kit::test]
fn history_identity_prepend_keeps_favorite_button_and_space_does_not_paste(
    cx: &mut TestAppContext,
) {
    let f = fixture(cx);
    let mut surface = VisualTestContext::from_window(f.window, cx);
    let focus = focus_row_action(&mut surface, f.item_id, Some("favorite"));
    prepend(&f, &mut surface);
    assert_eq!(surface.update(|window, cx| window.focused(cx)), Some(focus));
    assert_eq!(
        row_action_snapshot(&mut surface, f.item_id, Some("favorite")).focused(),
        Some(true)
    );
    key(&mut surface, "space", true);
    key(&mut surface, "space", false);
    for _ in 0..400 {
        surface.run_until_parked();
        let history =
            nxtools_clipboard::ClipboardHistory::open(f.directory.path().join("clipboard.db"))
                .expect("observe favorite");
        if history
            .active_item(f.item_id)
            .expect("domain A still active")
            .is_favorite
        {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    let history =
        nxtools_clipboard::ClipboardHistory::open(f.directory.path().join("clipboard.db"))
            .expect("observe result");
    assert!(history.active_item(f.item_id).expect("A").is_favorite);
    assert!(
        !history
            .active_item(160)
            .expect("old index occupant")
            .is_favorite
    );
    assert!(
        f.panel
            .read_with(&surface, |panel, _| panel.pending_action.is_none())
    );
    assert!(
        f.endpoint.try_receive_command_for_contract_test().is_err(),
        "management Space never bubbles into paste"
    );
}

fn wait_page(f: &IdentityFixture, surface: &mut VisualTestContext) {
    for _ in 0..400 {
        surface.run_until_parked();
        if !f.panel.read_with(surface, |panel, _| {
            panel.page_request_pending || panel.search_pending
        }) {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(
        !f.panel
            .read_with(surface, |panel, _| panel.page_request_pending
                || panel.search_pending)
    );
    surface.refresh().expect("actual updated history frame");
    surface.update(|window, cx| window.draw(cx).clear(cx));
}

fn focused_scope(surface: &mut VisualTestContext, item_id: i64, action: Option<&str>) -> bool {
    let path = row_action_snapshot(surface, item_id, action)
        .path()
        .to_vec();
    surface.update(|window, _| test_support::has_observed_focus(window, &path))
}

#[gpui_kit::test]
fn history_identity_real_append_front_trim_keeps_row_and_space_on_domain_id(
    cx: &mut TestAppContext,
) {
    let f = fixture_page(cx, 500, 80, 320, 100);
    let mut surface = VisualTestContext::from_window(f.window, cx);
    let focus = focus_row_action(&mut surface, f.item_id, None);
    f.panel.update_in(&mut surface, |panel, _, cx| {
        panel.window_start_offset = 80;
        panel.has_more_entries = true;
        panel.load_next_page(cx);
    });
    wait_page(&f, &mut surface);
    f.panel.read_with(&surface, |panel, _| {
        assert_eq!(panel.entries.borrow().len(), HISTORY_WINDOW_SIZE);
        assert_eq!(panel.window_start_offset, 160);
        assert_eq!(panel.entries.borrow()[20].id, f.item_id);
    });
    assert_eq!(surface.update(|window, cx| window.focused(cx)), Some(focus));
    assert!(focused_scope(&mut surface, f.item_id, None));
    key(&mut surface, "space", true);
    key(&mut surface, "space", false);
    assert_eq!(
        f.panel.read_with(&surface, |panel, _| panel
            .pending_action
            .map(|(_, _, id)| id)),
        Some(f.item_id)
    );
}

#[gpui_kit::test]
fn history_identity_real_front_trim_keeps_management_button_without_paste(cx: &mut TestAppContext) {
    let f = fixture_page(cx, 500, 80, 320, 100);
    let mut surface = VisualTestContext::from_window(f.window, cx);
    let focus = focus_row_action(&mut surface, f.item_id, Some("favorite"));
    f.panel.update_in(&mut surface, |panel, _, cx| {
        panel.window_start_offset = 80;
        panel.has_more_entries = true;
        panel.load_next_page(cx);
    });
    wait_page(&f, &mut surface);
    assert_eq!(surface.update(|window, cx| window.focused(cx)), Some(focus));
    assert!(focused_scope(&mut surface, f.item_id, Some("favorite")));
    key(&mut surface, "enter", true);
    key(&mut surface, "enter", false);
    for _ in 0..400 {
        surface.run_until_parked();
        let history =
            nxtools_clipboard::ClipboardHistory::open(f.directory.path().join("clipboard.db"))
                .expect("observe management result");
        if history
            .active_item(f.item_id)
            .expect("A retained")
            .is_favorite
        {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    let history =
        nxtools_clipboard::ClipboardHistory::open(f.directory.path().join("clipboard.db"))
            .expect("observe favorite");
    assert!(history.active_item(f.item_id).expect("A").is_favorite);
    assert!(
        f.panel
            .read_with(&surface, |panel, _| panel.pending_action.is_none())
    );
    assert!(f.endpoint.try_receive_command_for_contract_test().is_err());
}

#[gpui_kit::test]
fn history_identity_armed_row_prepend_release_still_activates_original_item_once(
    cx: &mut TestAppContext,
) {
    let f = fixture(cx);
    let mut surface = VisualTestContext::from_window(f.window, cx);
    focus_row_action(&mut surface, f.item_id, None);
    key(&mut surface, "space", true);
    prepend(&f, &mut surface);
    key(&mut surface, "space", false);
    let operation = f.panel.read_with(&surface, |panel, _| {
        panel.pending_action.expect("original armed row action")
    });
    assert_eq!(operation.2, f.item_id);
    key(&mut surface, "space", false);
    assert_eq!(
        f.panel.read_with(&surface, |panel, _| panel.pending_action),
        Some(operation)
    );
}

#[gpui_kit::test]
fn history_identity_deleted_armed_row_does_not_activate_old_index_occupant(
    cx: &mut TestAppContext,
) {
    let f = fixture(cx);
    let mut surface = VisualTestContext::from_window(f.window, cx);
    let old_focus = focus_row_action(&mut surface, f.item_id, None);
    key(&mut surface, "enter", true);
    nxtools_clipboard::ClipboardHistory::open(f.directory.path().join("clipboard.db"))
        .expect("synthetic delete")
        .soft_delete(f.item_id, nxtools_ipc::unix_time_ms())
        .expect("delete A");
    f.panel
        .update_in(&mut surface, |panel, _, cx| panel.refresh_entries(cx));
    wait_page(&f, &mut surface);
    assert!(!f.panel.read_with(&surface, |panel, _| {
        panel
            .entries
            .borrow()
            .iter()
            .any(|item| item.id == f.item_id)
    }));
    key(&mut surface, "enter", false);
    assert!(
        f.panel
            .read_with(&surface, |panel, _| panel.pending_action.is_none())
    );
    assert!(f.endpoint.try_receive_command_for_contract_test().is_err());
    surface.update(|window, cx| {
        assert!(
            !old_focus.is_focused(window),
            "deleted row must retire actual focus"
        );
        assert!(
            f.panel
                .read(cx)
                .search_input
                .focus_handle(cx)
                .is_focused(window),
            "existing search is explicit fallback"
        );
    });
}

#[gpui_kit::test]
fn history_identity_removed_armed_management_release_does_not_mutate_or_paste(
    cx: &mut TestAppContext,
) {
    let f = fixture(cx);
    let mut surface = VisualTestContext::from_window(f.window, cx);
    let old_focus = focus_row_action(&mut surface, f.item_id, Some("favorite"));
    key(&mut surface, "space", true);
    nxtools_clipboard::ClipboardHistory::open(f.directory.path().join("clipboard.db"))
        .expect("remove action origin")
        .soft_delete(f.item_id, nxtools_ipc::unix_time_ms())
        .expect("delete A");
    f.panel
        .update_in(&mut surface, |panel, _, cx| panel.refresh_entries(cx));
    wait_page(&f, &mut surface);
    key(&mut surface, "space", false);
    surface.run_until_parked();
    assert!(
        f.panel
            .read_with(&surface, |panel, _| panel.pending_action.is_none())
    );
    let history =
        nxtools_clipboard::ClipboardHistory::open(f.directory.path().join("clipboard.db"))
            .expect("observe active neighbor");
    assert!(!history.active_item(160).expect("neighbor").is_favorite);
    assert!(f.endpoint.try_receive_command_for_contract_test().is_err());
    surface.update(|window, cx| {
        assert!(!old_focus.is_focused(window));
        assert!(
            f.panel
                .read(cx)
                .search_input
                .focus_handle(cx)
                .is_focused(window)
        );
    });
}

#[gpui_kit::test]
fn history_identity_reorder_same_mounted_records_preserves_action_and_search_entity(
    cx: &mut TestAppContext,
) {
    let f = fixture(cx);
    let mut surface = VisualTestContext::from_window(f.window, cx);
    let search = f
        .panel
        .read_with(&surface, |panel, _| panel.search_input.entity_id());
    let focus = focus_row_action(&mut surface, f.item_id, Some("edit"));
    f.panel.update_in(&mut surface, |panel, _, cx| {
        panel.entries.borrow_mut().swap(0, 1);
        panel.list_scroll.remeasure_items(0..2);
        panel.selected_entry_index = Some(1);
        cx.notify();
    });
    surface.refresh().expect("real reordering frame");
    assert_eq!(surface.update(|window, cx| window.focused(cx)), Some(focus));
    assert!(focused_scope(&mut surface, f.item_id, Some("edit")));
    key(&mut surface, "enter", true);
    key(&mut surface, "enter", false);
    for _ in 0..400 {
        surface.run_until_parked();
        if f.panel
            .read_with(&surface, |panel, _| panel.editing_item_id.is_some())
        {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    f.panel.read_with(&surface, |panel, cx| {
        assert_eq!(panel.editing_item_id, Some(f.item_id));
        assert_eq!(panel.search_input.entity_id(), search);
        assert!(panel.edit_input.read(cx).value().ends_with("FULL-TAIL"));
        assert!(panel.pending_action.is_none());
    });
}

#[gpui_kit::test]
fn history_identity_real_front_trim_removed_armed_row_retires_to_search(cx: &mut TestAppContext) {
    let f = fixture_page(cx, 500, 80, 320, 0);
    let mut surface = VisualTestContext::from_window(f.window, cx);
    let old_focus = focus_row_action(&mut surface, f.item_id, None);
    key(&mut surface, "space", true);
    f.panel.update_in(&mut surface, |panel, _, cx| {
        panel.window_start_offset = 80;
        panel.has_more_entries = true;
        panel.load_next_page(cx);
    });
    wait_page(&f, &mut surface);
    f.panel.read_with(&surface, |panel, _| {
        assert_eq!(panel.entries.borrow().len(), HISTORY_WINDOW_SIZE);
        assert_eq!(panel.window_start_offset, 160);
        assert!(
            !panel
                .entries
                .borrow()
                .iter()
                .any(|item| item.id == f.item_id)
        );
    });
    surface.update(|window, cx| {
        assert!(!old_focus.is_focused(window));
        assert!(
            f.panel
                .read(cx)
                .search_input
                .focus_handle(cx)
                .is_focused(window)
        );
    });
    key(&mut surface, "space", false);
    assert!(
        f.panel
            .read_with(&surface, |panel, _| panel.pending_action.is_none())
    );
    assert!(f.endpoint.try_receive_command_for_contract_test().is_err());
}

#[gpui_kit::test]
fn history_identity_equal_domain_ids_in_distinct_panels_have_distinct_focus(
    cx: &mut TestAppContext,
) {
    let first = fixture(cx);
    let second = fixture(cx);
    assert_eq!(first.item_id, second.item_id);
    let mut first_surface = VisualTestContext::from_window(first.window, cx);
    let first_focus = focus_row_action(&mut first_surface, first.item_id, Some("favorite"));
    let mut second_surface = VisualTestContext::from_window(second.window, cx);
    let second_focus = focus_row_action(&mut second_surface, second.item_id, Some("favorite"));
    assert_ne!(
        first_focus, second_focus,
        "retained ancestry is panel/window local"
    );
    prepend(&second, &mut second_surface);
    assert!(focused_scope(
        &mut second_surface,
        second.item_id,
        Some("favorite")
    ));
    assert!(focused_scope(
        &mut first_surface,
        first.item_id,
        Some("favorite")
    ));
    assert!(
        first
            .panel
            .read_with(&first_surface, |panel, _| panel.pending_action.is_none())
    );
    assert!(
        second
            .panel
            .read_with(&second_surface, |panel, _| panel.pending_action.is_none())
    );
}

struct UnmountedHistory;
impl Render for UnmountedHistory {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full()
    }
}

#[gpui_kit::test]
fn history_identity_unmount_and_new_session_do_not_reuse_focus_or_armed_release(
    cx: &mut TestAppContext,
) {
    let f = fixture(cx);
    let mut surface = VisualTestContext::from_window(f.window, cx);
    let old_focus = focus_row_action(&mut surface, f.item_id, None);
    key(&mut surface, "enter", true);
    surface.update(|window, cx| {
        window.replace_root(cx, |_, _| UnmountedHistory);
    });
    surface
        .refresh()
        .expect("unmount original panel while its weak subscription is retained");
    assert_eq!(
        surface.update(|window, cx| window.focused(cx)),
        Some(old_focus.clone()),
        "unmounted original panel must not focus its orphaned search"
    );
    let page = f
        .panel
        .read_with(&surface, |panel, _| ClipboardHistoryPage {
            items: panel.entries.borrow().clone(),
            total: panel.total_entries,
            has_more: false,
        });
    let counts = f
        .panel
        .read_with(&surface, |panel, _| panel.summary_counts.clone());
    let worker = f.worker.clone();
    let application = f._application.downgrade();
    let slot = Rc::new(RefCell::new(None));
    let output = slot.clone();
    surface.update(|window, cx| {
        window.replace_root(cx, |window, cx| {
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
                    vec![],
                    ClipboardModifierActions::default(),
                    nxtools_protocol::ForegroundContext {
                        window_handle: 42,
                        focus_window_handle: 43,
                        process_id: 2,
                        process_name: "synthetic-target".into(),
                        window_title: String::new(),
                    },
                    worker,
                    application,
                    uuid::Uuid::new_v4(),
                    None,
                    window,
                    cx,
                )
                .with_summary_counts(counts)
            });
            *output.borrow_mut() = Some(panel.clone());
            Root::new(panel, window, cx)
        });
    });
    surface
        .refresh()
        .expect("new actual panel session in same window");
    let replacement = slot.borrow().clone().expect("replacement actual panel");
    surface.update(|window, cx| {
        assert!(
            replacement
                .read(cx)
                .search_input
                .focus_handle(cx)
                .is_focused(window)
        );
        assert!(
            !f.panel
                .read(cx)
                .search_input
                .focus_handle(cx)
                .is_focused(window)
        );
    });
    let new_focus = focus_row_action(&mut surface, f.item_id, None);
    assert_ne!(old_focus, new_focus);
    key(&mut surface, "enter", false);
    assert!(replacement.read_with(&surface, |panel, _| panel.pending_action.is_none()));
    assert!(
        f.panel
            .read_with(&surface, |panel, _| panel.pending_action.is_none())
    );
    key(&mut surface, "enter", true);
    key(&mut surface, "enter", false);
    assert_eq!(
        replacement.read_with(&surface, |panel, _| panel
            .pending_action
            .map(|(_, _, id)| id)),
        Some(f.item_id)
    );
}
