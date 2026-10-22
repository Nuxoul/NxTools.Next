//! Production-view regressions. Synthetic metadata only; no native worker or Shell launch.
use super::ui_state::{
    UiGeometry, UiSortKey, UiState, UiStateLoad, UiStateLoadIssue, UiStateStore, UiStateStoreError,
};
use super::*;
use gpui_kit::{
    KeyUpEvent, Keystroke, Modifiers, MouseDownEvent, MouseUpEvent, TestAppContext,
    VisualTestContext, size,
};
use std::{cell::RefCell, rc::Rc};

fn entry(ordinal: u64, name: String) -> ArchiveReadEntry {
    serde_json::from_value(serde_json::json!({
        "ordinal": ordinal, "name": name, "kind": "File", "size": ordinal + 1,
        "packed_size": null, "encrypted": false, "warning_codes": [],
        "provenance": "Utf8Flag", "codepage": 65001
    }))
    .expect("synthetic legacy-compatible metadata")
}
fn mount(cx: &mut TestAppContext, count: usize) -> (Entity<ArchiveBrowser>, VisualTestContext) {
    cx.update(gpui_kit::init);
    cx.executor().allow_parking();
    let retained = Rc::new(RefCell::new(None));
    let slot = retained.clone();
    let (_, surface) = cx.add_window_view(move |window, cx| {
        let view = cx.new(|cx| {
            let mut view = ArchiveBrowser::new(
                PathBuf::from(r"C:\synthetic\fixture.zip"),
                Uuid::new_v4(),
                WeakEntity::new_invalid(),
                window,
                cx,
            );
            view.install(
                Uuid::new_v4(),
                (0..count)
                    .map(|ix| entry(ix as u64, format!("file-{ix:05}.txt")))
                    .collect(),
                "Zip".into(),
                window,
                cx,
            );
            view
        });
        *slot.borrow_mut() = Some(view.clone());
        gpui_kit::base::Root::new(view, window, cx)
    });
    let handle = surface.update(|window, _| window.window_handle());
    let mut surface = VisualTestContext::from_window(handle, cx);
    surface.simulate_resize(size(px(960.), px(640.)));
    draw(&mut surface);
    (retained.borrow().clone().expect("production view"), surface)
}
fn draw(surface: &mut VisualTestContext) {
    surface.run_until_parked();
    surface.update(|window, cx| {
        window.activate_window();
        window.draw(cx).clear(cx);
    });
}
fn click(surface: &mut VisualTestContext, ordinal: usize, modifiers: Modifiers, count: usize) {
    draw(surface);
    let position = surface.update(|window, _| {
        gpui_kit::base::test_support::snapshots(window)
            .into_iter()
            .find(|node| {
                node.label() == Some(format!("file-{ordinal:05}.txt").as_str())
                    && node.role() == Some(Role::ListBoxOption)
            })
            .expect("current-frame production ListItem bounds")
            .bounds()
            .center()
    });
    surface.simulate_event(MouseDownEvent {
        position,
        button: MouseButton::Left,
        modifiers,
        click_count: count,
        ..Default::default()
    });
    surface.simulate_event(MouseUpEvent {
        position,
        button: MouseButton::Left,
        modifiers,
        click_count: count,
    });
    draw(surface);
}
fn key(surface: &mut VisualTestContext, stroke: &str) {
    surface.simulate_event(KeyDownEvent {
        keystroke: Keystroke::parse(stroke).expect("key"),
        is_held: false,
        prefer_character_input: false,
    });
    surface.simulate_event(KeyUpEvent {
        keystroke: Keystroke::parse(stroke).expect("release"),
    });
    draw(surface);
}
fn semantic_selected(surface: &mut VisualTestContext) -> usize {
    surface.update(|window, _| {
        gpui_kit::base::test_support::snapshots(window)
            .iter()
            .filter(|node| {
                node.role() == Some(Role::ListBoxOption) && node.selected() == Some(true)
            })
            .count()
    })
}
#[gpui_kit::test]
fn pointer_keyboard_and_semantics_use_production_single_selection_owner(cx: &mut TestAppContext) {
    let (view, mut surface) = mount(cx, 8);
    click(&mut surface, 0, Modifiers::default(), 1);
    click(
        &mut surface,
        2,
        Modifiers {
            control: true,
            ..Default::default()
        },
        1,
    );
    assert_eq!(semantic_selected(&mut surface), 2);
    click(
        &mut surface,
        5,
        Modifiers {
            shift: true,
            ..Default::default()
        },
        1,
    );
    assert_eq!(semantic_selected(&mut surface), 4);
    key(&mut surface, "shift-up");
    assert_eq!(semantic_selected(&mut surface), 3);
    key(&mut surface, "ctrl-a");
    assert_eq!(semantic_selected(&mut surface), 8);
    view.read_with(&surface, |v, _| {
        assert_eq!(v.model.as_ref().expect("model").selected.len(), 8)
    });
    let position = surface
        .debug_bounds("archive-entry-3")
        .expect("row")
        .center();
    surface.simulate_event(MouseDownEvent {
        position,
        button: MouseButton::Right,
        ..Default::default()
    });
    draw(&mut surface);
    view.read_with(&surface, |v, _| {
        assert_eq!(v.model.as_ref().expect("model").selected.len(), 8)
    });
}
#[gpui_kit::test]
fn input_filter_sort_keep_ids_and_do_not_steal_text_shortcuts(cx: &mut TestAppContext) {
    let (view, mut surface) = mount(cx, 8);
    click(&mut surface, 2, Modifiers::default(), 1);
    surface
        .update(|window, cx| view.update(cx, |v, cx| v.sort_column(ColumnKind::Size, window, cx)));
    view.read_with(&surface, |v, _| {
        assert_eq!(
            v.model.as_ref().expect("model").selected_file_ordinals(),
            vec![2]
        )
    });
    key(&mut surface, "ctrl-f");
    let input = view.read_with(&surface, |v, _| v.filter_input.clone());
    surface.update(|window, cx| {
        input.update(cx, |input, cx| {
            input.set_value("file-00002", window, cx);
            cx.emit(InputEvent::Change);
        })
    });
    draw(&mut surface);
    view.read_with(&surface, |v, _| {
        let model = v.model.as_ref().expect("model");
        assert_eq!(model.visible().len(), 1);
        assert_eq!(model.selected_file_ordinals(), vec![2]);
    });
    key(&mut surface, "ctrl-a");
    key(&mut surface, "backspace");
    view.read_with(&surface, |v, _| {
        assert!(v.model.as_ref().expect("model").folder.is_empty())
    });
    input.read_with(&surface, |input, _| {
        assert!(
            input.value().is_empty(),
            "Ctrl+A/Backspace belong to the Input"
        )
    });
}
#[gpui_kit::test]
fn minimum_viewport_has_horizontal_scroll_and_only_visible_listitems(cx: &mut TestAppContext) {
    let (view, mut surface) = mount(cx, 10_000);
    surface.simulate_resize(size(px(720.), px(480.)));
    draw(&mut surface);
    let rendered = surface.update(|window, _| {
        gpui_kit::base::test_support::snapshots(window)
            .iter()
            .filter(|node| node.role() == Some(Role::ListBoxOption))
            .count()
    });
    assert!(
        rendered > 0 && rendered < 100,
        "actual viewport rows, not all metadata controls: {rendered}"
    );
    let viewport = surface
        .debug_bounds("archive-horizontal")
        .expect("horizontal viewport");
    view.read_with(&surface, |v, _| {
        assert!(v.columns_width() > viewport.size.width)
    });
    let footer = surface.debug_bounds("archive-status").expect("footer");
    assert!(footer.bottom() <= px(480.));
    click(&mut surface, 0, Modifiers::default(), 1);
    key(&mut surface, "end");
    view.read_with(&surface, |v, _| {
        assert_eq!(
            v.model.as_ref().expect("model").selected_file_ordinals(),
            vec![9999]
        )
    });
}
#[gpui_kit::test]
fn tree_immediate_notify_and_list_navigation_are_deferred_and_deduplicated(
    cx: &mut TestAppContext,
) {
    let (view, mut surface) = mount(cx, 0);
    surface.update(|window, cx| {
        view.update(cx, |v, cx| {
            v.install(
                Uuid::new_v4(),
                vec![entry(0, "a/one.txt".into()), entry(1, "b/two.txt".into())],
                "Zip".into(),
                window,
                cx,
            )
        })
    });
    draw(&mut surface);
    let tree = view.read_with(&surface, |v, _| v.tree.clone());
    surface.update(|_, cx| {
        tree.update(cx, |tree, cx| {
            tree.set_selected_item(Some(&TreeItem::new("a", "a")), cx)
        })
    });
    draw(&mut surface);
    view.read_with(&surface, |v, _| {
        assert_eq!(v.model.as_ref().expect("model").folder, "a")
    });
    surface.update(|_, cx| tree.update(cx, |_, cx| cx.notify()));
    draw(&mut surface);
    surface.update(|window, cx| view.update(cx, |v, cx| v.navigate("b".into(), window, cx)));
    draw(&mut surface);
    tree.read_with(&surface, |tree, _| {
        assert_eq!(
            tree.selected_item()
                .expect("selected tree folder")
                .id
                .as_ref(),
            "b"
        )
    });
}
#[gpui_kit::test]
fn failure_retains_lastgood_and_retired_callbacks_never_report_ready(cx: &mut TestAppContext) {
    let (view, mut surface) = mount(cx, 4);
    surface.update(|window, cx| {
        view.update(cx, |v, cx| {
            v.set_failure("源压缩包已更改".into(), window, cx)
        })
    });
    draw(&mut surface);
    view.read_with(&surface, |v, _| {
        assert!(matches!(v.status, BrowserStatus::Stale(_)));
        assert_eq!(v.model.as_ref().expect("lastgood").entries.len(), 4);
    });
    surface.update(|window, cx| {
        view.update(cx, |v, cx| {
            v.retire(cx);
            v.install(Uuid::new_v4(), vec![], "Zip".into(), window, cx);
        })
    });
    view.read_with(&surface, |v, _| {
        assert!(!matches!(v.status, BrowserStatus::Ready))
    });
    let (fresh, mut other) = mount(cx, 0);
    other.update(|window, cx| {
        fresh.update(cx, |v, cx| {
            v.model = None;
            v.set_failure("读取失败；请重试".into(), window, cx);
        })
    });
    fresh.read_with(&other, |v, _| {
        assert!(matches!(v.status, BrowserStatus::Failed(_)))
    });
}
#[gpui_kit::test]
fn enter_doubleclick_and_held_repeat_dispatch_at_most_one_pending_open(cx: &mut TestAppContext) {
    let (view, mut surface) = mount(cx, 3);
    click(&mut surface, 0, Modifiers::default(), 2);
    key(&mut surface, "enter");
    surface.simulate_event(KeyDownEvent {
        keystroke: Keystroke::parse("enter").expect("enter"),
        is_held: true,
        prefer_character_input: false,
    });
    draw(&mut surface);
    view.read_with(&surface, |v, _| {
        assert!(v.open_pending);
        assert_eq!(v.open_dispatches.len(), 1);
        assert!(!matches!(v.status, BrowserStatus::Failed(_)));
    });
}

#[gpui_kit::test]
fn encoding_refresh_invalidates_actions_and_install_replaces_selection(cx: &mut TestAppContext) {
    let (view, mut surface) = mount(cx, 4);
    click(&mut surface, 1, Modifiers::default(), 1);
    surface.update(|window, cx| {
        view.update(cx, |v, cx| v.command(Command::Encoding(936), window, cx))
    });
    key(&mut surface, "enter");
    view.read_with(&surface, |v, _| {
        assert_eq!(v.codepage, 936);
        assert!(matches!(v.status, BrowserStatus::Loading));
        assert!(v.open_dispatches.is_empty());
        assert_eq!(v.model.as_ref().expect("lastgood").selected.len(), 1);
    });
    surface.update(|window, cx| {
        view.update(cx, |v, cx| {
            v.install(
                Uuid::new_v4(),
                vec![entry(7, "fresh.txt".into())],
                "Zip".into(),
                window,
                cx,
            )
        })
    });
    draw(&mut surface);
    view.read_with(&surface, |v, _| {
        assert!(matches!(v.status, BrowserStatus::Ready));
        assert!(v.model.as_ref().expect("new snapshot").selected.is_empty());
    });
}

#[gpui_kit::test]
fn folder_doubleclick_and_scoped_backspace_sync_tree_and_clear_selection(cx: &mut TestAppContext) {
    let (view, mut surface) = mount(cx, 0);
    surface.update(|window, cx| {
        view.update(cx, |v, cx| {
            v.install(
                Uuid::new_v4(),
                vec![
                    entry(0, "folder/child.txt".into()),
                    entry(1, "other.txt".into()),
                ],
                "Zip".into(),
                window,
                cx,
            )
        })
    });
    draw(&mut surface);
    let position = surface
        .debug_bounds("archive-folder-folder")
        .expect("folder ListItem")
        .center();
    surface.simulate_event(MouseDownEvent {
        position,
        button: MouseButton::Left,
        click_count: 2,
        ..Default::default()
    });
    surface.simulate_event(MouseUpEvent {
        position,
        button: MouseButton::Left,
        click_count: 2,
        modifiers: Modifiers::default(),
    });
    draw(&mut surface);
    view.read_with(&surface, |v, _| {
        assert_eq!(v.model.as_ref().expect("model").folder, "folder");
        assert!(v.model.as_ref().expect("model").selected.is_empty());
    });
    key(&mut surface, "backspace");
    view.read_with(&surface, |v, _| {
        assert!(v.model.as_ref().expect("model").folder.is_empty())
    });
}

#[test]
fn unknown_columns_and_invalid_timestamp_are_not_invented() {
    let m = BrowserModel::new(Uuid::new_v4(), vec![entry(0, "file.txt".into())]);
    let row = &m.visible()[0];
    for column in [
        ColumnKind::Packed,
        ColumnKind::Modified,
        ColumnKind::Method,
        ColumnKind::Crc,
        ColumnKind::Attributes,
    ] {
        assert_eq!(cell(row, column), "—");
    }
    assert_eq!(modified(Some(&ArchiveModifiedTime::Invalid)), "—");
    assert!(modified(Some(&ArchiveModifiedTime::DosLocal { date: 0, time: 0 })).contains('—'));
}

#[gpui_kit::test]
fn queued_tree_navigation_cannot_overwrite_a_new_snapshot(cx: &mut TestAppContext) {
    let (view, mut surface) = mount(cx, 0);
    surface.update(|window, cx| {
        view.update(cx, |v, cx| {
            v.install(
                Uuid::new_v4(),
                vec![
                    entry(0, "a/child.txt".into()),
                    entry(1, "b/child.txt".into()),
                ],
                "Zip".into(),
                window,
                cx,
            )
        })
    });
    draw(&mut surface);
    let tree = view.read_with(&surface, |v, _| v.tree.clone());
    surface.update(|window, cx| {
        tree.update(cx, |tree, cx| {
            tree.set_selected_item(Some(&TreeItem::new("a", "a")), cx)
        });
        view.update(cx, |v, cx| {
            v.install(
                Uuid::new_v4(),
                vec![entry(4, "replacement.txt".into())],
                "Zip".into(),
                window,
                cx,
            )
        });
    });
    draw(&mut surface);
    view.read_with(&surface, |v, _| {
        let model = v.model.as_ref().expect("new snapshot");
        assert!(model.folder.is_empty());
        assert_eq!(model.visible()[0].name, "replacement.txt");
    });
}

#[gpui_kit::test]
fn local_entry_open_is_not_dispatched_after_same_turn_close(cx: &mut TestAppContext) {
    let (view, mut surface) = mount(cx, 2);
    click(&mut surface, 0, Modifiers::default(), 1);
    surface.update(|window, cx| {
        view.update(cx, |v, cx| {
            v.activate(window, cx);
            v.retire(cx);
        })
    });
    draw(&mut surface);
    view.read_with(&surface, |v, _| {
        assert!(v.retired);
        assert!(!v.open_pending);
        assert!(matches!(v.status, BrowserStatus::Cancelled));
    });
}

#[gpui_kit::test]
fn optional_column_toggle_rebuilds_released_resize_tracks_without_reentry(cx: &mut TestAppContext) {
    let (view, mut surface) = mount(cx, 3);
    surface.update(|window, cx| {
        view.update(cx, |v, cx| {
            v.command(Command::Column(ColumnKind::Method), window, cx)
        })
    });
    draw(&mut surface);
    view.read_with(&surface, |v, _| {
        assert!(v.columns[ColumnKind::Method.index()])
    });
    surface.update(|window, cx| {
        view.update(cx, |v, cx| {
            v.command(Command::Column(ColumnKind::Method), window, cx)
        })
    });
    draw(&mut surface);
    view.read_with(&surface, |v, _| {
        assert!(!v.columns[ColumnKind::Method.index()])
    });
}

#[gpui_kit::test]
fn list_home_page_and_shift_end_follow_same_immutable_anchor(cx: &mut TestAppContext) {
    let (view, mut surface) = mount(cx, 80);
    click(&mut surface, 0, Modifiers::default(), 1);
    key(&mut surface, "pagedown");
    let ordinal = view.read_with(&surface, |v, _| {
        v.model.as_ref().expect("model").selected_file_ordinals()[0]
    });
    assert!(ordinal > 0 && ordinal < 80);
    key(&mut surface, "home");
    key(&mut surface, "shift-end");
    view.read_with(&surface, |v, _| {
        assert_eq!(v.model.as_ref().expect("model").selected.len(), 80)
    });
}

fn click_named(surface: &mut VisualTestContext, name: &str, role: Role) {
    draw(surface);
    let position = surface.update(|window, _| {
        gpui_kit::base::test_support::snapshots(window)
            .into_iter()
            .find(|node| node.label() == Some(name) && node.role() == Some(role) && node.visible())
            .expect("released control current-frame semantic hit bounds")
            .bounds()
            .center()
    });
    surface.simulate_event(MouseDownEvent {
        position,
        button: MouseButton::Left,
        ..Default::default()
    });
    surface.simulate_event(MouseUpEvent {
        position,
        button: MouseButton::Left,
        modifiers: Modifiers::default(),
        click_count: 1,
    });
    draw(surface);
}

#[gpui_kit::test]
fn real_per_window_file_popup_refresh_dispatches_local_loading_state(cx: &mut TestAppContext) {
    let (view, mut surface) = mount(cx, 3);
    click_named(&mut surface, "文件", Role::Button);
    click_named(&mut surface, "刷新", Role::MenuItem);
    view.read_with(&surface, |v, _| {
        assert!(matches!(v.status, BrowserStatus::Loading))
    });
    assert!(
        view.read_with(&surface, |v, _| v.model.is_some()),
        "refresh keeps lastgood until terminal"
    );
}

fn wait_for_ui_state_load(store: &UiStateStore) -> UiStateLoad {
    for _ in 0..200 {
        let status = store.load_status();
        if !matches!(status, UiStateLoad::Loading) {
            return status;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    panic!("archive UI state background load did not finish");
}

#[test]
fn corrupt_ui_state_is_preserved_and_blocks_default_overwrite() {
    let directory = tempfile::tempdir().expect("temporary UI state directory");
    let path = directory.path().join("archive-ui-state.json");
    let corrupt = br#"{"version":1,"geometry":BROKEN"#.to_vec();
    std::fs::write(&path, &corrupt).expect("seed corrupt UI state");

    let store = UiStateStore::initialize(path.clone()).expect("start UI state owner");
    assert!(matches!(
        wait_for_ui_state_load(&store),
        UiStateLoad::Preserved(UiStateLoadIssue::Corrupt)
    ));
    assert!(matches!(
        store.enqueue_save(UiState::default()),
        Err(UiStateStoreError::Preserved)
    ));
    store.finish_for_test();

    assert_eq!(
        std::fs::read(path).expect("read preserved corrupt UI state"),
        corrupt
    );
}

#[test]
fn final_close_state_is_atomically_saved_and_reloaded() {
    let directory = tempfile::tempdir().expect("temporary UI state directory");
    let path = directory.path().join("archive-ui-state.json");
    let store = UiStateStore::initialize(path.clone()).expect("start UI state owner");
    assert!(matches!(
        wait_for_ui_state_load(&store),
        UiStateLoad::Ready(_)
    ));

    let mut expected = UiState {
        geometry: UiGeometry {
            x: -420,
            y: 180,
            width: 1180,
            height: 760,
            maximized: true,
        },
        ..UiState::default()
    };
    expected.widths[ColumnKind::Name.index()] = 312.0;
    expected.widths[ColumnKind::Method.index()] = 144.0;
    expected.visible[ColumnKind::Method.index()] = true;
    expected.show_tree = false;
    expected.show_comment = true;
    expected.sort = UiSortKey::Packed;
    expected.descending = true;
    let mut queued = expected.clone();
    queued.geometry.width = 1040;
    store
        .enqueue_save(queued)
        .expect("enqueue earlier presentation snapshot");
    let mut latest = expected.clone();
    latest.widths[ColumnKind::Crc.index()] = 176.0;
    expected = latest;
    // A full bounded queue must not lose the newest close snapshot: the owner coalesces the
    // latest value and Drop is the lossless final barrier.
    store
        .enqueue_save(expected.clone())
        .expect("coalesce final close snapshot");
    store.finish_for_test();

    let reloaded = UiStateStore::initialize(path.clone()).expect("restart UI state owner");
    assert_eq!(
        wait_for_ui_state_load(&reloaded),
        UiStateLoad::Ready(expected)
    );
    reloaded.finish_for_test();
    assert!(
        std::fs::read_dir(directory.path())
            .expect("list state directory")
            .all(|entry| !entry
                .expect("state directory entry")
                .file_name()
                .to_string_lossy()
                .ends_with(".tmp")),
        "atomic writer must not leave a sibling temporary"
    );
}

#[test]
fn invalid_persisted_bounds_are_rejected_without_rewriting_the_file() {
    let directory = tempfile::tempdir().expect("temporary UI state directory");
    let path = directory.path().join("archive-ui-state.json");
    let store = UiStateStore::initialize(path.clone()).expect("start UI state owner");
    assert!(matches!(
        wait_for_ui_state_load(&store),
        UiStateLoad::Ready(_)
    ));
    store
        .enqueue_save(UiState::default())
        .expect("seed valid UI state");
    store.finish_for_test();

    let mut document: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).expect("read valid UI state fixture"))
            .expect("parse valid UI state fixture");
    document["geometry"]["width"] = serde_json::json!(80);
    let invalid = serde_json::to_vec_pretty(&document).expect("encode invalid bounds fixture");
    std::fs::write(&path, &invalid).expect("replace with invalid bounds fixture");

    let rejected = UiStateStore::initialize(path.clone()).expect("restart UI state owner");
    assert!(matches!(
        wait_for_ui_state_load(&rejected),
        UiStateLoad::Preserved(UiStateLoadIssue::Invalid)
    ));
    rejected.finish_for_test();
    assert_eq!(
        std::fs::read(path).expect("read preserved invalid UI state"),
        invalid
    );
}

#[gpui_kit::test]
fn restored_state_reaches_actual_browser_columns_without_entity_reentry(cx: &mut TestAppContext) {
    let (view, mut surface) = mount(cx, 12);
    let mut restored = UiState::default();
    restored.geometry.width = 840;
    restored.geometry.height = 520;
    restored.widths[ColumnKind::Name.index()] = 320.0;
    restored.widths[ColumnKind::Method.index()] = 156.0;
    restored.visible = [
        true, false, false, false, false, true, false, false, false, false,
    ];
    restored.show_tree = false;
    restored.show_comment = true;
    restored.sort = UiSortKey::Method;
    restored.descending = true;

    surface.update(|window, cx| {
        view.update(cx, |browser, cx| {
            assert!(browser.apply_restored_state(restored.clone(), window, cx));
            assert!(!browser.apply_restored_state(UiState::default(), window, cx));
        });
    });
    draw(&mut surface);

    view.read_with(&surface, |browser, _| {
        assert_eq!(browser.widths, restored.widths);
        assert_eq!(browser.columns, restored.visible);
        assert!(!browser.show_tree);
        assert!(browser.show_comment);
        assert_eq!(browser.sort, SortKey::Method);
        assert!(browser.descending);
        assert_eq!(browser.columns_width(), px(476.0));
        assert_eq!(browser.snapshot_from_observed().visible, restored.visible);
    });
    surface.update(|window, cx| {
        view.update(cx, |browser, _| {
            let snapshot = browser.snapshot(window);
            browser.final_snapshot = Some(snapshot);
        });
    });
    view.read_with(&surface, |browser, _| {
        let snapshot = browser
            .final_snapshot()
            .expect("captured final browser state");
        assert_eq!(snapshot.visible, restored.visible);
        assert_eq!(snapshot.widths, restored.widths);
    });
    assert!(surface.debug_bounds("archive-comment").is_some());
}
#[gpui_kit::test]
fn edit_selected_ordinals_survive_sort_filter_and_stale_rejects(cx: &mut TestAppContext) {
    let (view, mut surface) = mount(cx, 6);
    surface.update(|window, cx| {
        view.update(cx, |browser, cx| {
            browser.edit_eligibility = Some(Ok(()));
            browser.sort_column(ColumnKind::Size, window, cx);
            let model = browser.model.as_mut().expect("snapshot");
            model.select(RowId::Entry(model.snapshot, 2), false, false);
            assert_eq!(
                browser
                    .edit_selection(ArchiveJobModifyAction::Delete)
                    .expect("delete")
                    .1,
                vec![2]
            );
            browser.status = BrowserStatus::Stale("source changed".into());
            assert!(
                browser
                    .edit_selection(ArchiveJobModifyAction::Delete)
                    .is_none()
            );
            browser.status = BrowserStatus::Ready;
            if let Some(model) = browser.model.as_mut() {
                model.filter = "file-00002".into();
                model.reproject();
            }
        });
    });
    draw(&mut surface);
    view.read_with(&surface, |browser, _| {
        assert_eq!(
            browser
                .edit_selection(ArchiveJobModifyAction::Rename)
                .expect("rename")
                .1,
            vec![2]
        );
    });
    surface.update(|_, cx| view.update(cx, |browser, cx| browser.retire(cx)));
    draw(&mut surface);
    view.read_with(&surface, |browser, _| {
        assert!(
            browser
                .edit_selection(ArchiveJobModifyAction::Delete)
                .is_none()
        )
    });
}
