//! Production-view regressions. Synthetic metadata only; no native worker or Shell launch.
use super::ui_state::{
    UiGeometry, UiSortKey, UiState, UiStateLoad, UiStateLoadIssue, UiStateStore, UiStateStoreError,
};
use super::*;
use gpui_kit::{
    KeyUpEvent, Keystroke, Modifiers, MouseDownEvent, MouseUpEvent, TestAppContext,
    VisualTestContext, point, size,
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
fn capability_source() -> nxtools_protocol::archive_browser::ArchiveSourceBinding {
    nxtools_protocol::archive_browser::ArchiveSourceBinding {
        native: "v1:fixture".into(),
        digest: [1; 32],
        size: 123,
        modified_unix_ns: 1,
    }
}
pub(super) fn mount(
    cx: &mut TestAppContext,
    count: usize,
) -> (Entity<ArchiveBrowser>, VisualTestContext) {
    mount_mode(cx, count, true)
}
fn mount_mode(
    cx: &mut TestAppContext,
    count: usize,
    details: bool,
) -> (Entity<ArchiveBrowser>, VisualTestContext) {
    cx.update(gpui_kit::init);
    cx.update(crate::theme::apply_component_theme);
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
            // Both optional modes are explicit here; default is tested independently.
            view.details_view = details;
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
    let row = surface.update(|window, _| {
        gpui_kit::base::test_support::snapshots(window)
            .into_iter()
            .find(|node| {
                node.label() == Some(format!("file-{ordinal:05}.txt").as_str())
                    && node.role() == Some(Role::ListBoxOption)
            })
            .expect("current-frame production ListItem bounds")
            .bounds()
    });
    let viewport = surface
        .debug_bounds("archive-horizontal")
        .expect("actual row viewport");
    let left = row.left().max(viewport.left());
    let right = row.right().min(viewport.right());
    assert!(
        left < right && row.center().y >= viewport.top() && row.center().y <= viewport.bottom()
    );
    let position = point((left + right) / 2., row.center().y);
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
fn key(surface: &mut VisualTestContext, stroke: &'static str) {
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
    let mut restored = UiState {
        details_view: true,
        ..UiState::default()
    };
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
            browser.edit_source = Some(capability_source());
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

#[gpui_kit::test]
fn ui_correction_vertical_wheel_never_moves_horizontal_offset(cx: &mut TestAppContext) {
    let (view, mut surface) = mount(cx, 180);
    surface.simulate_resize(size(px(720.), px(480.)));
    draw(&mut surface);
    let viewport = surface
        .debug_bounds("archive-horizontal")
        .expect("production viewport");
    surface.update(|_, cx| {
        view.update(cx, |v, cx| {
            v.horizontal.set_offset(point(px(-64.), px(0.)));
            v.scroll.scroll_to_item(60, ScrollStrategy::Top);
            cx.notify();
        })
    });
    draw(&mut surface);
    let vertical_before =
        view.read_with(&surface, |v, _| v.scroll.0.borrow().base_handle.offset().y);
    let horizontal_before = view.read_with(&surface, |v, _| v.horizontal.offset().x);
    surface.simulate_event(gpui_kit::ScrollEvent {
        position: point(viewport.left() + px(80.), viewport.top() + px(90.)),
        delta: gpui_kit::ScrollDelta::Lines(point(0., -3.)),
        ..Default::default()
    });
    draw(&mut surface);
    view.read_with(&surface, |v, _| {
        assert!(
            v.scroll.0.borrow().base_handle.offset().y < vertical_before,
            "ordinary wheel really moves vertically"
        );
        assert_eq!(v.horizontal.offset().x, horizontal_before);
    });
    let position = point(viewport.left() + px(80.), viewport.top() + px(90.));
    for at_end in [false, true] {
        surface.update(|_, cx| {
            view.update(cx, |v, cx| {
                v.horizontal.set_offset(point(px(-64.), px(0.)));
                v.scroll
                    .scroll_to_item(if at_end { 179 } else { 0 }, ScrollStrategy::Top);
                cx.notify();
            })
        });
        draw(&mut surface);
        let before = view.read_with(&surface, |v, _| v.horizontal.offset().x);
        surface.simulate_event(gpui_kit::ScrollEvent {
            position,
            delta: gpui_kit::ScrollDelta::Lines(point(0., if at_end { -4. } else { 4. })),
            ..Default::default()
        });
        draw(&mut surface);
        assert_eq!(
            view.read_with(&surface, |v, _| v.horizontal.offset().x),
            before,
            "vertical wheel must not fall through into horizontal movement at either boundary"
        );
    }
    for position in [
        point(viewport.left() + px(80.), viewport.top() + px(12.)),
        point(viewport.right() - px(4.), viewport.center().y),
        point(viewport.center().x, viewport.bottom() - px(4.)),
    ] {
        let before = view.read_with(&surface, |v, _| v.horizontal.offset().x);
        surface.simulate_event(gpui_kit::ScrollEvent {
            position,
            delta: gpui_kit::ScrollDelta::Lines(point(0., -3.)),
            ..Default::default()
        });
        draw(&mut surface);
        assert_eq!(
            view.read_with(&surface, |v, _| v.horizontal.offset().x),
            before,
            "header and related scrollbar areas cannot remap vertical wheel"
        );
    }
    let before = view.read_with(&surface, |v, _| v.horizontal.offset().x);
    surface.simulate_event(gpui_kit::ScrollEvent {
        position,
        delta: gpui_kit::ScrollDelta::Lines(point(-3., 0.)),
        ..Default::default()
    });
    draw(&mut surface);
    assert!(
        view.read_with(&surface, |v, _| v.horizontal.offset().x) < before,
        "explicit horizontal input remains available"
    );
}

#[gpui_kit::test]
fn ui_correction_tree_indentation_and_split_are_real_production_geometry(cx: &mut TestAppContext) {
    let (view, mut surface) = mount(cx, 0);
    surface.update(|window, cx| {
        view.update(cx, |v, cx| {
            v.install(
                Uuid::new_v4(),
                vec![
                    entry(0, "root/child/deep/file.txt".into()),
                    entry(1, "other/file.txt".into()),
                ],
                "Zip".into(),
                window,
                cx,
            );
            v.navigate("root/child".into(), window, cx);
        })
    });
    draw(&mut surface);
    for dimensions in [size(px(960.), px(640.)), size(px(720.), px(480.))] {
        surface.simulate_resize(dimensions);
        draw(&mut surface);
        let tree = surface
            .debug_bounds("archive-tree-pane")
            .expect("tree pane");
        let details = surface
            .debug_bounds("archive-horizontal")
            .expect("details pane");
        assert!(tree.size.width >= px(160.) && tree.size.width <= px(280.));
        assert!(details.left() >= tree.right() && details.right() <= dimensions.width);
        let root = surface
            .debug_bounds("archive-tree-label-root")
            .expect("root label");
        let child = surface
            .debug_bounds("archive-tree-label-root/child")
            .expect("revealed child label");
        assert!(
            child.left() >= root.left() + px(16.),
            "visible depth indentation"
        );
        assert!(
            child.right() <= tree.right(),
            "long labels remain inside tree"
        );
        assert!(
            surface
                .debug_bounds("archive-tree-disclosure-root")
                .is_some()
        );
        assert!(
            surface
                .debug_bounds("archive-tree-icon-root/child")
                .is_some()
        );
    }
}

#[gpui_kit::test]
fn ui_correction_file_type_icons_and_toolbar_hierarchy_are_mounted(cx: &mut TestAppContext) {
    let (view, mut surface) = mount(cx, 2);
    surface.update(|window, cx| {
        view.update(cx, |v, cx| {
            v.install(
                Uuid::new_v4(),
                vec![entry(0, "folder/a.txt".into()), entry(1, "b.txt".into())],
                "Zip".into(),
                window,
                cx,
            );
        })
    });
    draw(&mut surface);
    assert!(surface.debug_bounds("archive-kind-folder-folder").is_some());
    assert!(surface.debug_bounds("archive-kind-file-b.txt").is_some());
    let menus = surface.debug_bounds("archive-menu-bar").expect("menu row");
    let toolbar = surface
        .debug_bounds("archive-icon-toolbar")
        .expect("command toolbar");
    assert!(menus.bottom() <= toolbar.top());
    for (icon_selector, label_selector) in [
        ("archive-open-icon", "archive-open-label"),
        ("archive-extract-all-icon", "archive-extract-all-label"),
        ("archive-new-icon", "archive-new-label"),
        ("archive-add-icon", "archive-add-label"),
        ("archive-delete-icon", "archive-delete-label"),
        ("archive-test-icon", "archive-test-label"),
    ] {
        let icon = surface
            .debug_bounds(icon_selector)
            .expect("actual action icon");
        let label = surface
            .debug_bounds(label_selector)
            .expect("actual action label");
        assert!(
            icon.bottom() <= label.top(),
            "icon is above label, not another text-only action"
        );
        assert!(icon.top() >= toolbar.top() && label.bottom() <= toolbar.bottom());
    }
    surface.update(|window, cx| view.update(cx, |v, cx| v.navigate("folder".into(), window, cx)));
    draw(&mut surface);
    assert!(surface.debug_bounds("archive-kind-parent-..").is_some());
}

#[gpui_kit::test]
fn browser_primary_commands_precede_view_and_preserve_edit_scope(cx: &mut TestAppContext) {
    let (view, mut surface) = mount(cx, 2);
    draw(&mut surface);
    let mut previous = surface.debug_bounds("archive-open").expect("open").left();
    for selector in [
        "archive-extract-all",
        "archive-new",
        "archive-add",
        "archive-delete",
        "archive-test",
        "archive-view",
    ] {
        let bounds = surface
            .debug_bounds(selector)
            .expect("primary command or view");
        assert!(previous < bounds.left(), "command hierarchy: {selector}");
        previous = bounds.left();
    }
    let disabled = surface.update(|window, _| {
        gpui_kit::base::test_support::snapshots(window)
            .iter()
            .find(|node| node.label() == Some("删除"))
            .and_then(|node| node.disabled())
    });
    assert_eq!(disabled, Some(true), "deletion needs an eligible selection");
    click(&mut surface, 0, Modifiers::default(), 1);
    view.read_with(&surface, |v, _| {
        assert_eq!(v.model.as_ref().expect("model").selected.len(), 1)
    });
    surface.update(|_, cx| {
        view.update(cx, |v, cx| {
            v.encrypted = Some(true);
            cx.notify();
        })
    });
    draw(&mut surface);
    assert!(surface.debug_bounds("archive-unlock-drag").is_none());
    assert!(
        surface
            .debug_bounds("archive-view")
            .expect("view last")
            .left()
            > surface
                .debug_bounds("archive-test")
                .expect("test action")
                .left()
    );
}

#[gpui_kit::test]
fn ui_correction_browser_preferences_apply_without_replacing_selection_owner(
    cx: &mut TestAppContext,
) {
    let (view, mut surface) = mount(cx, 0);
    surface.update(|window, cx| {
        view.update(cx, |v, cx| {
            let settings = ArchiveSettings {
                auto_enter_single_root: false,
                show_parent_row: false,
                single_click_open: true,
                full_row_select: false,
                show_full_path: true,
                escape_closes_window: true,
                ..Default::default()
            };
            v.apply_settings(settings, window, cx);
            v.install(
                Uuid::new_v4(),
                vec![entry(0, "only/file.txt".into())],
                "Zip".into(),
                window,
                cx,
            );
            assert!(v.model.as_ref().expect("model").folder.is_empty());
            v.navigate("only".into(), window, cx);
        })
    });
    draw(&mut surface);
    view.read_with(&surface, |v, _| {
        assert_eq!(v.model.as_ref().expect("model").visible().len(), 1)
    });
    let name = surface
        .debug_bounds("archive-entry-0")
        .expect("name-only hit region");
    view.read_with(&surface, |v, _| {
        assert_eq!(name.size.width, px(v.widths[0]))
    });
    surface.simulate_click(name.center(), Modifiers::default());
    draw(&mut surface);
    view.read_with(&surface, |v, _| assert_eq!(v.open_dispatches.len(), 1));
    key(&mut surface, "ctrl-f");
    key(&mut surface, "escape");
    view.read_with(&surface, |v, _| {
        assert!(!v.retired, "first Escape belongs to filter")
    });
    key(&mut surface, "escape");
    view.read_with(&surface, |v, _| {
        assert!(v.retired, "second Escape uses saved window behavior")
    });
}

#[gpui_kit::test]
fn ui_correction_themes_minimum_busy_and_horizontal_alignment(cx: &mut TestAppContext) {
    let (view, mut surface) = mount(cx, 140);
    surface.update(|_, cx| crate::theme::apply_component_theme(cx));
    for preference in [
        nxtools_core::ThemePreference::Light,
        nxtools_core::ThemePreference::Dark,
    ] {
        surface.update(|_, cx| crate::theme::set_theme_preference(preference, cx));
        for dimensions in [size(px(960.), px(640.)), size(px(720.), px(480.))] {
            surface.simulate_resize(dimensions);
            draw(&mut surface);
            let menu = surface.debug_bounds("archive-menu-bar").expect("menu");
            let toolbar = surface
                .debug_bounds("archive-icon-toolbar")
                .expect("toolbar");
            assert!(menu.bottom() <= toolbar.top() && toolbar.right() <= dimensions.width);
            let header_before = surface
                .debug_bounds("archive-header-0")
                .expect("name header");
            let row_before = surface.debug_bounds("archive-entry-0").expect("first row");
            surface.update(|_, cx| {
                view.update(cx, |v, cx| {
                    v.horizontal.set_offset(point(px(-80.), px(0.)));
                    cx.notify();
                })
            });
            draw(&mut surface);
            let header_after = surface
                .debug_bounds("archive-header-0")
                .expect("name header");
            let row_after = surface.debug_bounds("archive-entry-0").expect("first row");
            assert_eq!(
                header_after.left() - header_before.left(),
                row_after.left() - row_before.left()
            );
            surface.update(|_, cx| {
                view.update(cx, |v, cx| {
                    v.status = BrowserStatus::Loading;
                    cx.notify();
                })
            });
            draw(&mut surface);
            let disabled = surface.update(|window, _| {
                gpui_kit::base::test_support::snapshots(window)
                    .into_iter()
                    .find(|node| node.label() == Some("测试") && node.role() == Some(Role::Button))
                    .expect("test action")
                    .disabled()
            });
            assert_eq!(disabled, Some(true));
            surface.update(|_, cx| {
                view.update(cx, |v, cx| {
                    v.status = BrowserStatus::Ready;
                    v.horizontal.set_offset(point(px(0.), px(0.)));
                    cx.notify();
                })
            });
        }
    }
}

#[test]
fn ui_remaining_recent_paths_are_part_of_original_bounded_ui_state() {
    let mut value = serde_json::to_value(UiState::default()).expect("presentation state");
    value["recent"] = serde_json::json!([r"C:\owned\recent.zip"]);
    let state: UiState = serde_json::from_value(value).expect("recent state");
    assert!(state.validate().is_ok());
    assert_eq!(
        serde_json::to_value(state).expect("roundtrip")["recent"],
        serde_json::json!([r"C:\owned\recent.zip"])
    );
}

#[test]
fn ui_remaining_recent_clear_survives_old_geometry_save_and_restart() {
    let directory = tempfile::tempdir().expect("owned recent state");
    let path = directory.path().join("archive-ui.json");
    let store = UiStateStore::initialize(path.clone()).expect("original writer");
    assert!(matches!(
        wait_for_ui_state_load(&store),
        UiStateLoad::Ready(_)
    ));
    store
        .update_recent(|recent| recent.push(PathBuf::from(r"C:\owned\one.zip")))
        .expect("recent mutation");
    assert!(
        store
            .flush()
            .expect("receipt")
            .recv_blocking()
            .expect("durable")
    );
    let old_window = store.current_snapshot();
    store.update_recent(Vec::clear).expect("clear");
    store.enqueue_save(old_window).expect("late close geometry");
    assert!(
        store
            .flush()
            .expect("clear receipt")
            .recv_blocking()
            .expect("durable clear")
    );
    store.finish_for_test();
    let state: UiState =
        serde_json::from_slice(&std::fs::read(&path).expect("saved")).expect("state");
    assert!(
        state.recent.is_empty(),
        "late geometry cannot resurrect cleared history"
    );
    let restarted = UiStateStore::initialize(path).expect("restart");
    assert!(
        matches!(wait_for_ui_state_load(&restarted), UiStateLoad::Ready(state) if state.recent.is_empty())
    );
    restarted.finish_for_test();
}

#[test]
fn ui_remaining_recent_invalid_and_failed_storage_preserve_original_bytes() {
    let directory = tempfile::tempdir().expect("owned recent state");
    let path = directory.path().join("archive-ui.json");
    let store = UiStateStore::initialize(path.clone()).expect("writer");
    wait_for_ui_state_load(&store);
    assert_eq!(
        store.update_recent(|recent| recent.push(PathBuf::from("relative.zip"))),
        Err(UiStateStoreError::Invalid)
    );
    std::fs::write(&path, b"{broken").expect("corrupt original");
    store
        .update_recent(|recent| recent.push(PathBuf::from(r"C:\owned\one.zip")))
        .expect("queue only");
    match store.flush() {
        Ok(receipt) => assert!(!receipt.recv_blocking().expect("failure result")),
        Err(reason) => assert_eq!(reason, UiStateStoreError::Preserved),
    }
    assert_eq!(std::fs::read(&path).expect("preserved"), b"{broken");
    store.finish_for_test();
}

#[gpui_kit::test]
fn ui_remaining_real_splitter_drag_resizes_without_losing_details(cx: &mut TestAppContext) {
    let (_, mut surface) = mount(cx, 50);
    draw(&mut surface);
    let before = surface.debug_bounds("archive-tree-pane").expect("tree");
    let start = point(before.right(), before.center().y);
    surface.simulate_event(MouseDownEvent {
        position: start,
        button: MouseButton::Left,
        click_count: 1,
        ..Default::default()
    });
    for distance in [8., 32., 56.] {
        surface.simulate_event(gpui_kit::MouseMoveEvent {
            position: point(start.x + px(distance), start.y),
            pressed_button: Some(MouseButton::Left),
            ..Default::default()
        });
        draw(&mut surface);
    }
    surface.simulate_event(MouseUpEvent {
        position: point(start.x + px(56.), start.y),
        button: MouseButton::Left,
        click_count: 1,
        modifiers: Modifiers::default(),
    });
    draw(&mut surface);
    let after = surface
        .debug_bounds("archive-tree-pane")
        .expect("resized tree");
    assert!(
        after.size.width > before.size.width + px(8.),
        "real handle drag must change width: {before:?} -> {after:?}"
    );
    assert!((px(160.)..=px(280.)).contains(&after.size.width));
    let details = surface.debug_bounds("archive-horizontal").expect("details");
    assert!(details.left() >= after.right());
}

#[gpui_kit::test]
fn ui_remaining_horizontal_bar_drag_moves_only_horizontal_axis(cx: &mut TestAppContext) {
    let (view, mut surface) = mount(cx, 100);
    surface.simulate_resize(size(px(720.), px(480.)));
    draw(&mut surface);
    let viewport = surface
        .debug_bounds("archive-horizontal")
        .expect("viewport");
    let start = point(viewport.left() + px(24.), viewport.bottom() - px(5.));
    let before_y = view.read_with(&surface, |view, _| {
        view.scroll.0.borrow().base_handle.offset().y
    });
    surface.simulate_event(gpui_kit::MouseMoveEvent {
        position: start,
        ..Default::default()
    });
    draw(&mut surface);
    surface.simulate_event(MouseDownEvent {
        position: start,
        button: MouseButton::Left,
        click_count: 1,
        ..Default::default()
    });
    surface.simulate_event(gpui_kit::MouseMoveEvent {
        position: point(start.x + px(100.), start.y),
        pressed_button: Some(MouseButton::Left),
        ..Default::default()
    });
    surface.simulate_event(MouseUpEvent {
        position: point(start.x + px(100.), start.y),
        button: MouseButton::Left,
        click_count: 1,
        modifiers: Modifiers::default(),
    });
    draw(&mut surface);
    assert!(
        view.read_with(&surface, |view, _| view.horizontal.offset().x) < px(-1.),
        "drag actual scrollbar, not set_offset"
    );
    assert_eq!(
        view.read_with(&surface, |view, _| view
            .scroll
            .0
            .borrow()
            .base_handle
            .offset()
            .y),
        before_y
    );
}

#[gpui_kit::test]
fn render_repair_split_halves_share_full_action_height(cx: &mut TestAppContext) {
    let (_, mut surface) = mount(cx, 2);
    let main = surface.debug_bounds("archive-open").expect("main action");
    let menu = surface.update(|window, _| {
        gpui_kit::base::test_support::snapshots(window)
            .into_iter()
            .find(|node| node.label() == Some("打开选项"))
            .expect("official split menu trigger")
            .bounds()
    });
    assert!(
        menu.size.width <= px(32.),
        "menu trigger must stay narrow, not64px square"
    );
    assert_eq!(main.top(), menu.top(), "both official halves align at top");
    assert_eq!(
        main.size.height, menu.size.height,
        "not a64px action plus32px caret"
    );
}

#[gpui_kit::test]
fn render_repair_header_uses_the_same_widths_as_file_rows(cx: &mut TestAppContext) {
    let (view, mut surface) = mount(cx, 2);
    for width in [960., 720., 1200.] {
        surface.simulate_resize(size(px(width), px(640.)));
        draw(&mut surface);
        draw(&mut surface);
        let widths = view.read_with(&surface, |view, _| view.widths);
        for (index, selector) in [
            "archive-header-0",
            "archive-header-1",
            "archive-header-2",
            "archive-header-3",
            "archive-header-4",
        ]
        .into_iter()
        .enumerate()
        {
            let header = surface.debug_bounds(selector).expect("header");
            let cell_selector = [
                "archive-cell-0-0",
                "archive-cell-0-1",
                "archive-cell-0-2",
                "archive-cell-0-3",
                "archive-cell-0-4",
            ][index];
            let cell = surface
                .debug_bounds(cell_selector)
                .expect("actual row cell");
            assert!(
                (f32::from(cell.size.width) - widths[index]).abs() <= 1.,
                "row cell {index} was squeezed: {:?} vs {}",
                cell.size.width,
                widths[index]
            );
            assert!(
                (f32::from(cell.left() - header.left())).abs() <= 1.,
                "header/row start lane {index}"
            );
            assert!(
                (f32::from(header.size.width) - widths[index]).abs() <= 1.,
                "header {index}={:?}, row width={} at viewport={width}",
                header.size.width,
                widths[index]
            );
        }
    }
}

#[gpui_kit::test]
fn render_repair_real_file_rows_request_shared_shell_type_icons(cx: &mut TestAppContext) {
    use nxtools_protocol::{Message, PlatformCommand};
    let (view, mut surface) = mount(cx, 0);
    let (bridge, endpoint) = nxtools_ipc::platform_bridge(32);
    endpoint.set_connected_for_contract_test(true);
    surface.update(|window, cx| {
        view.update(cx, |view, cx| {
            view.configure_icons(Rc::new(bridge));
            view.install(
                Uuid::new_v4(),
                vec![
                    entry(0, "sample.EXE".into()),
                    entry(1, "another.exe".into()),
                    entry(2, "settings.xml".into()),
                ],
                "Zip".into(),
                window,
                cx,
            );
        })
    });
    draw(&mut surface);
    let first = endpoint
        .try_receive_command_for_contract_test()
        .expect("file rows must request real shared Shell icon");
    let second = endpoint
        .try_receive_command_for_contract_test()
        .expect("one resource per extension");
    for command in [&first, &second] {
        assert!(
            matches!(&command.message, Message::PlatformCommand(PlatformCommand::ExtractShellIcon(value)) if value.size == 32 && matches!(value.target.as_str(), "nxtools-file-type:.exe" | "nxtools-file-type:.xml"))
        );
    }
    assert!(
        endpoint.try_receive_command_for_contract_test().is_err(),
        "duplicate exe shares type resource; no materialize/open"
    );
    let directory = tempfile::tempdir().expect("owned PNG");
    let image = directory.path().join("type.png");
    std::fs::write(&image, crate::icons::ONE_PIXEL_PNG).expect("synthetic native result");
    surface.update(|_, cx| {
        view.update(cx, |view, cx| {
            view.icon_result(
                first.id,
                &nxtools_protocol::PlatformCommandResult {
                    success: true,
                    error: None,
                    native_error_code: None,
                    data: Some(image.to_string_lossy().into_owned()),
                },
                cx,
            )
        })
    });
    draw(&mut surface);
    draw(&mut surface);
    assert!(
        surface.debug_bounds("archive-shell-type-icon").is_some(),
        "resolved PNG must be mounted, not generic File"
    );
    surface.update(|window, cx| {
        view.update(cx, |view, cx| {
            view.install(
                Uuid::new_v4(),
                vec![entry(3, "new.txt".into())],
                "Zip".into(),
                window,
                cx,
            );
            view.icon_result(
                second.id,
                &nxtools_protocol::PlatformCommandResult {
                    success: true,
                    error: None,
                    native_error_code: None,
                    data: Some(image.to_string_lossy().into_owned()),
                },
                cx,
            );
        })
    });
    draw(&mut surface);
    assert!(
        surface.debug_bounds("archive-shell-type-icon").is_none(),
        "old snapshot result cannot replace new type"
    );
}

thread_local! {
    static RENDER_REPAIR_FONTS: RefCell<Vec<gpui_kit::SharedString>> = const { RefCell::new(Vec::new()) };
}
pub(super) fn render_repair_font_probe() -> AnyElement {
    gpui_kit::canvas(
        |_, _, _| (),
        |_, (), window, _| {
            RENDER_REPAIR_FONTS.with(|values| {
                let mut values = values.borrow_mut();
                if values.len() < 64 {
                    values.push(window.text_style().font_family.clone());
                }
            });
        },
    )
    .absolute()
    .size_0()
    .into_any_element()
}
#[gpui_kit::test]
fn render_repair_file_cells_use_platform_ui_font_and_readable_sizes(cx: &mut TestAppContext) {
    RENDER_REPAIR_FONTS.with(|values| values.borrow_mut().clear());
    let (view, mut surface) = mount(cx, 2);
    draw(&mut surface);
    RENDER_REPAIR_FONTS.with(|values| {
        let values = values.borrow();
        assert!(!values.is_empty(), "actual file cell paint must be sampled");
        assert!(
            values
                .iter()
                .all(|font| font.as_ref() == crate::ui::typography::UI_FONT_FAMILY),
            "file cells must not fall back to a serif family: {values:?}"
        );
    });
    RENDER_REPAIR_FONTS.with(|values| values.borrow_mut().clear());
    let chosen = crate::ui::typography::TEST_UI_FONT_FAMILY;
    surface.update(|_, cx| {
        cx.set_global(crate::ui::typography::InstalledUiFontsFixture(vec![
            chosen.into(),
        ]));
        assert!(crate::theme::set_ui_font_family(chosen, cx));
    });
    draw(&mut surface);
    RENDER_REPAIR_FONTS.with(|values| {
        let values = values.borrow();
        assert!(!values.is_empty(), "configured font must be painted");
        assert!(values.iter().all(|font| font == chosen));
    });
    assert_eq!(bytes(None), "—");
    assert_eq!(bytes(Some(0)), "0 B");
    assert_eq!(bytes(Some(2_021_888)), "2.0 MB");
    surface.update(|window, cx| {
        view.update(cx, |view, cx| {
            let mut state = UiState::default();
            state.widths[0] = 96.;
            view.apply_restored_state(state, window, cx);
            assert!(
                view.widths[0] >= 240.,
                "restore cannot crush the primary filename column"
            );
        })
    });
}

#[gpui_kit::test]
fn render_repair_type_icons_are_bounded_and_disconnect_retires_results(cx: &mut TestAppContext) {
    let (view, mut surface) = mount(cx, 0);
    let (bridge, endpoint) = nxtools_ipc::platform_bridge(32);
    endpoint.set_connected_for_contract_test(true);
    surface.update(|window, cx| {
        view.update(cx, |view, cx| {
            view.configure_icons(Rc::new(bridge));
            view.install(
                Uuid::new_v4(),
                (0..12)
                    .map(|n| entry(n, format!("file{n}.ext{n}")))
                    .collect(),
                "Zip".into(),
                window,
                cx,
            );
        })
    });
    draw(&mut surface);
    let mut commands = Vec::new();
    while let Ok(command) = endpoint.try_receive_command_for_contract_test() {
        commands.push(command);
    }
    assert_eq!(
        commands.len(),
        8,
        "visible types cannot flood the native reader"
    );
    draw(&mut surface);
    assert!(
        endpoint.try_receive_command_for_contract_test().is_err(),
        "pending resources deduplicate redraw"
    );
    surface.update(|_, cx| view.update(cx, |view, _| view.icons_disconnected()));
    view.read_with(&surface, |view, _| {
        assert!(
            commands
                .iter()
                .all(|command| !view.icon_matches(command.id)),
            "disconnect retires every pending receipt"
        );
    });
    surface.update(|_, cx| view.update(cx, |view, cx| view.retire(cx)));
    draw(&mut surface);
    assert!(
        endpoint.try_receive_command_for_contract_test().is_err(),
        "retired rows cannot issue new icon work"
    );
}

#[gpui_kit::test]
fn optional_name_view_fills_content_pane(cx: &mut TestAppContext) {
    let (_, mut surface) = mount_mode(cx, 2, false);
    for width in [960., 720., 1200.] {
        surface.simulate_resize(size(px(width), px(640.)));
        draw(&mut surface);
        draw(&mut surface);
        assert!(
            surface.debug_bounds("archive-header-1").is_none(),
            "explicit optional name list hides detail columns"
        );
        let pane = surface
            .debug_bounds("archive-horizontal")
            .expect("viewport");
        let name = surface
            .debug_bounds("archive-cell-0-0")
            .expect("filename lane");
        assert!(
            (f32::from(name.size.width - pane.size.width)).abs() <= 2.,
            "filename lane must fill viewport: {name:?} vs {pane:?}"
        );
    }
    let tree = surface.debug_bounds("archive-tree-pane").expect("tree");
    let start = point(tree.right(), tree.center().y);
    surface.simulate_event(MouseDownEvent {
        position: start,
        button: MouseButton::Left,
        click_count: 1,
        ..Default::default()
    });
    for distance in [8., 24., 48.] {
        surface.simulate_event(gpui_kit::MouseMoveEvent {
            position: point(start.x + px(distance), start.y),
            pressed_button: Some(MouseButton::Left),
            ..Default::default()
        });
        draw(&mut surface);
    }
    surface.simulate_event(MouseUpEvent {
        position: point(start.x + px(48.), start.y),
        button: MouseButton::Left,
        click_count: 1,
        modifiers: Modifiers::default(),
    });
    draw(&mut surface);
    draw(&mut surface);
    let resized_tree = surface
        .debug_bounds("archive-tree-pane")
        .expect("resized tree");
    assert!(
        resized_tree.size.width > tree.size.width + px(8.),
        "name-view split: {tree:?} -> {resized_tree:?}"
    );
    let pane = surface
        .debug_bounds("archive-horizontal")
        .expect("resized viewport");
    let name = surface
        .debug_bounds("archive-cell-0-0")
        .expect("resized name lane");
    assert!((f32::from(name.size.width - pane.size.width)).abs() <= 2.);
}

#[gpui_kit::test]
fn name_view_menu_switch_preserves_selection_columns_and_widths(cx: &mut TestAppContext) {
    let (view, mut surface) = mount_mode(cx, 2, false);
    surface.update(|window, cx| {
        view.update(cx, |view, cx| {
            view.install(
                Uuid::new_v4(),
                vec![
                    entry(0, "TrafficMonitor.VisualElementsManifest.xml".into()),
                    entry(1, "LibreHardwareMonitorLib.dll".into()),
                ],
                "Zip".into(),
                window,
                cx,
            );
            view.model.as_mut().expect("model").select_all();
            view.widths[0] = 344.;
            view.columns[ColumnKind::Method.index()] = true;
        })
    });
    draw(&mut surface);
    let before = view.read_with(&surface, |view, _| {
        (
            view.widths,
            view.columns,
            view.model.as_ref().expect("model").selected.clone(),
        )
    });
    click_named(&mut surface, "查看", Role::Button);
    click_named(&mut surface, "详细信息", Role::MenuItem);
    draw(&mut surface);
    assert!(surface.debug_bounds("archive-header-1").is_some());
    view.read_with(&surface, |view, _| {
        assert!(view.details_view);
        assert_eq!((view.widths, view.columns), (before.0, before.1));
        assert_eq!(view.model.as_ref().expect("model").selected, before.2);
    });
    surface.update(|_, cx| {
        view.update(cx, |view, _| {
            view.horizontal.set_offset(point(px(-64.), px(0.)))
        })
    });
    click_named(&mut surface, "查看", Role::Button);
    click_named(&mut surface, "名称列表", Role::MenuItem);
    draw(&mut surface);
    assert!(surface.debug_bounds("archive-header-1").is_none());
    view.read_with(&surface, |view, _| {
        assert!(!view.details_view);
        assert_eq!(view.horizontal.offset().x, px(0.));
        assert_eq!((view.widths, view.columns), (before.0, before.1));
        assert_eq!(view.model.as_ref().expect("model").selected, before.2);
        assert!(!view.open_pending && view.open_dispatches.is_empty());
    });
    for theme in [
        nxtools_core::ThemePreference::Light,
        nxtools_core::ThemePreference::Dark,
    ] {
        surface.update(|_, cx| crate::theme::set_theme_preference(theme, cx));
        surface.simulate_resize(size(px(720.), px(480.)));
        draw(&mut surface);
        draw(&mut surface);
        let name = surface
            .debug_bounds("archive-cell-0-0")
            .expect("long name lane");
        let viewport = surface
            .debug_bounds("archive-horizontal")
            .expect("current list viewport");
        assert!(name.size.width > px(0.) && name.size.width <= viewport.size.width);
        assert!(
            name.size.width >= viewport.size.width - px(64.),
            "name lane uses available list space, not a fixed hidden-preview width"
        );
    }
}

#[test]
fn name_view_legacy_default_and_explicit_mode_persist_in_original_store() {
    let mut document = serde_json::to_value(UiState::default()).expect("state");
    document
        .as_object_mut()
        .expect("object")
        .remove("details_view");
    let legacy: UiState = serde_json::from_value(document.clone()).expect("legacy");
    assert!(
        legacy.details_view,
        "new missing-field default is detailed mode"
    );
    document["details_view"] = serde_json::Value::Null;
    assert!(serde_json::from_value::<UiState>(document).is_err());
    let directory = tempfile::tempdir().expect("owned layout file");
    let path = directory.path().join("ui.json");
    for details in [true, false] {
        let store = UiStateStore::initialize(path.clone()).expect("store");
        assert!(matches!(
            wait_for_ui_state_load(&store),
            UiStateLoad::Ready(_)
        ));
        let mut state = store.current_snapshot();
        state.details_view = details;
        state.widths[0] = 344.;
        state.visible[ColumnKind::Method.index()] = true;
        store.enqueue_save(state).expect("save layout");
        store.finish_for_test();
        let reopened = UiStateStore::initialize(path.clone()).expect("reopen");
        let UiStateLoad::Ready(state) = wait_for_ui_state_load(&reopened) else {
            panic!("ready layout");
        };
        assert_eq!(state.details_view, details);
        assert_eq!(state.widths[0], 344.);
        assert!(state.visible[ColumnKind::Method.index()]);
        reopened.finish_for_test();
    }
}

#[gpui_kit::test]
fn browser_completion_folder_icons_are_recognizable_in_tree_and_list(cx: &mut TestAppContext) {
    let (view, mut surface) = mount(cx, 0);
    surface.update(|window, cx| {
        view.update(cx, |view, cx| {
            view.settings.auto_enter_single_root = false;
            view.install(
                Uuid::new_v4(),
                vec![entry(0, "docs/readme.md".into())],
                "Zip".into(),
                window,
                cx,
            );
        })
    });
    draw(&mut surface);
    let tree = surface
        .debug_bounds("archive-tree-icon-docs")
        .expect("real tree folder icon");
    assert_eq!(
        tree.size,
        gpui_kit::size(px(20.), px(20.)),
        "Tree uses the balanced fixed20DIP slot independently of the list: {tree:?}"
    );
    let list = surface
        .debug_bounds("archive-kind-folder-docs")
        .expect("real directory row icon");
    assert!(
        list.size.width >= px(24.),
        "right-list folder icons retain their original role/size: tree={tree:?}, list={list:?}"
    );
}

#[test]
fn automatic_preview_legacy_preferences_do_not_survive_ui_state_reopen() {
    let directory = tempfile::tempdir().expect("owned UI state");
    let path = directory.path().join("archive-ui-state.json");
    let store = UiStateStore::initialize(path.clone()).expect("original writer");
    let UiStateLoad::Ready(state) = wait_for_ui_state_load(&store) else {
        panic!("loaded defaults")
    };
    assert!(
        serde_json::to_value(&state)
            .expect("default state")
            .get("show_preview")
            .is_none()
    );
    store.finish_for_test();
    for version in [1, 2] {
        for choice in [None, Some(false), Some(true)] {
            let mut json = serde_json::to_value(UiState::default()).expect("state");
            json["version"] = version.into();
            json.as_object_mut().expect("object").remove("show_preview");
            if let Some(choice) = choice {
                json["show_preview"] = choice.into();
            }
            std::fs::write(&path, serde_json::to_vec(&json).expect("legacy state")).expect("seed");
            let store = UiStateStore::initialize(path.clone()).expect("original load");
            let UiStateLoad::Ready(state) = wait_for_ui_state_load(&store) else {
                panic!("legacy ready")
            };
            assert!(
                serde_json::to_value(&state)
                    .expect("loaded")
                    .get("show_preview")
                    .is_none()
            );
            store.enqueue_save(state).expect("same original writer");
            store.finish_for_test();
            let reopened = UiStateStore::initialize(path.clone()).expect("restart");
            let UiStateLoad::Ready(state) = wait_for_ui_state_load(&reopened) else {
                panic!("restart ready")
            };
            assert!(
                serde_json::to_value(&state)
                    .expect("reloaded")
                    .get("show_preview")
                    .is_none()
            );
            reopened.finish_for_test();
        }
    }
}

#[gpui_kit::test]
fn feedback_capabilities_readable_formats_have_no_global_modification_banner(
    cx: &mut TestAppContext,
) {
    let (view, mut surface) = mount(cx, 2);
    surface.update(|_, cx| {
        view.update(cx, |view, cx| {
            view.edit_eligibility = Some(Ok(()));
            view.edit_source = Some(capability_source());
            cx.notify();
        })
    });
    draw(&mut surface);
    let healthy_top = surface
        .debug_bounds("archive-horizontal")
        .expect("healthy list")
        .top();
    for format in [
        "Zip", "SevenZ", "Rar4", "Rar5", "Tar", "TarGz", "TarBz2", "TarXz", "TarZstd", "Gzip",
        "Bzip2", "Xz", "Zstd",
    ] {
        surface.update(|window, cx| {
            view.update(cx, |view, cx| {
                view.install(
                    Uuid::new_v4(),
                    vec![entry(0, "readme.md".into())],
                    format.into(),
                    window,
                    cx,
                );
                view.edit_eligibility =
                    (!matches!(format, "SevenZ" | "Tar" | "TarGz" | "TarXz")).then_some(Ok(()));
                view.edit_source = Some(capability_source());
            })
        });
        draw(&mut surface);
        surface.update(|_, cx| {
            view.update(cx, |view, cx| {
                view.edit_eligibility =
                    (!matches!(format, "SevenZ" | "Tar" | "TarGz" | "TarXz")).then_some(Ok(()));
                view.edit_source = Some(capability_source());
                cx.notify();
            })
        });
        draw(&mut surface);
        assert_eq!(
            surface
                .debug_bounds("archive-horizontal")
                .expect("actual list")
                .top(),
            healthy_top,
            "healthy {format} must not insert a persistent modification banner"
        );
        surface.update(|window, _| {
            let nodes = gpui_kit::base::test_support::snapshots(window);
            for label in ["解压全部", "测试"] {
                let action = nodes
                    .iter()
                    .find(|node| node.label() == Some(label) && node.role() == Some(Role::Button))
                    .expect("read action");
                assert_ne!(
                    action.disabled(),
                    Some(true),
                    "{format} {label} remains available"
                );
            }
            for label in ["添加", "删除"] {
                let action = nodes
                    .iter()
                    .find(|node| node.label() == Some(label) && node.role() == Some(Role::Button))
                    .expect("modify action");
                if format != "Zip" {
                    assert_eq!(
                        action.disabled(),
                        Some(true),
                        "{format} must not acquire write authority"
                    );
                }
            }
        });
        view.read_with(&surface, |view, _| {
            if format == "Zip" {
                assert!(view.edit_reason().is_none());
            } else if matches!(format, "SevenZ" | "Tar" | "TarGz" | "TarXz") {
                assert_eq!(
                    view.edit_reason().as_deref(),
                    Some("尚未确认源结构或元数据可安全保留，不能修改。")
                );
            } else {
                assert_eq!(
                    view.edit_reason().as_deref(),
                    Some("此格式支持浏览和解压，暂不支持修改压缩包内容。")
                );
            }
        });
    }
}

#[gpui_kit::test]
fn feedback_capabilities_split_zip_and_unknown_never_gain_modification_authority(
    cx: &mut TestAppContext,
) {
    let (view, mut surface) = mount(cx, 2);
    let ready_top = surface
        .debug_bounds("archive-horizontal")
        .expect("ready list")
        .top();
    surface.update(|_, cx| {
        view.update(cx, |view, cx| {
            view.edit_eligibility = Some(Err(
                nxtools_protocol::archive_job::ArchiveModificationRefusal::Volumes,
            ));
            cx.notify();
        })
    });
    draw(&mut surface);
    assert_eq!(
        surface
            .debug_bounds("archive-horizontal")
            .expect("split ZIP list")
            .top(),
        ready_top,
        "split ZIP must not add another global warning"
    );
    view.read_with(&surface, |view, _| {
        assert!(view.edit_reason().is_some());
        assert!(view.edit_selection(ArchiveJobModifyAction::Add).is_none());
    });
    surface.update(|_, cx| {
        view.update(cx, |view, cx| {
            view.format = "Unknown".into();
            cx.notify();
        })
    });
    view.read_with(&surface, |view, _| {
        let reason = view.edit_reason().expect("unknown remains disabled");
        assert!(
            !reason.contains("支持浏览和解压"),
            "never promise unsupported read capability"
        );
    });
}

#[gpui_kit::test]
fn feedback_capabilities_original_read_failures_remain_visible(cx: &mut TestAppContext) {
    let (view, mut surface) = mount(cx, 1);
    let ready_top = surface
        .debug_bounds("archive-horizontal")
        .expect("ready list")
        .top();
    for message in ["不支持此格式", "压缩包损坏", "密码错误"] {
        surface.update(|window, cx| {
            view.update(cx, |view, cx| view.set_failure(message.into(), window, cx))
        });
        draw(&mut surface);
        assert!(view.read_with(
            &surface,
            |view, _| matches!(&view.status, BrowserStatus::Stale(current) if current == message)
        ));
        assert!(
            surface
                .debug_bounds("archive-horizontal")
                .expect("retained last-good list")
                .top()
                > ready_top,
            "real failure must still paint its original warning above the last-good list"
        );
    }
}

#[gpui_kit::test]
fn feedback_capabilities_attempted_modify_uses_original_dialog_without_read_error(
    cx: &mut TestAppContext,
) {
    let (view, mut surface) = mount(cx, 2);
    surface.update(|_, cx| {
        view.update(cx, |view, cx| {
            view.format = "SevenZ".into();
            cx.notify();
        })
    });
    draw(&mut surface);
    click(&mut surface, 0, Modifiers::default(), 1);
    key(&mut surface, "delete");
    assert!(
        surface.update(|window, cx| window.has_active_dialog(cx)),
        "attempted modification explains the restriction through original Dialog owner"
    );
    assert!(
        surface.update(|window, _| gpui_kit::base::test_support::snapshots(window)
            .iter()
            .filter_map(|node| node.label())
            .any(|label| label.contains("尚未确认源结构或元数据可安全保留，不能修改。"))),
        "actual dialog carries precise capability copy"
    );
    view.read_with(&surface, |view, _| {
        assert!(matches!(view.status, BrowserStatus::Ready));
        assert!(
            view.edit_selection(ArchiveJobModifyAction::Delete)
                .is_none()
        );
    });
}

#[gpui_kit::test]
fn capability_foundation_zip_filename_does_not_supply_source_or_snapshot_proof(
    cx: &mut TestAppContext,
) {
    let (view, mut surface) = mount(cx, 2);
    surface.update(|_, cx| {
        view.update(cx, |view, _| {
            // The old four-byte inspection can say OK without any original source binding.
            view.edit_eligibility = Some(Ok(()));
            assert!(
                view.edit_reason().is_some(),
                "missing original source binding must refuse modification"
            );
            assert!(view.edit_selection(ArchiveJobModifyAction::Add).is_none());
        })
    });
}

#[gpui_kit::test]
fn capability_foundation_source_state_is_shared_by_pointer_shortcut_menu_and_drop_admission(
    cx: &mut TestAppContext,
) {
    use nxtools_protocol::archive_job::ArchiveModificationRefusal as R;
    let (view, mut surface) = mount(cx, 2);
    for refusal in [
        None,
        Some(R::SourceChanged),
        Some(R::Volumes),
        Some(R::UnverifiedMetadata),
    ] {
        surface.update(|_, cx| {
            view.update(cx, |view, cx| {
                view.edit_source = Some(capability_source());
                view.edit_eligibility = Some(refusal.map_or(Ok(()), Err));
                cx.notify();
            })
        });
        click(&mut surface, 0, Modifiers::default(), 1);
        let eligible = refusal.is_none();
        surface.update(|_, cx| {
            view.read_with(cx, |view, _| {
                // The application drop owner and deferred final form check both use edit_reason.
                assert_eq!(view.edit_reason().is_none(), eligible);
                assert_eq!(
                    view.edit_selection(ArchiveJobModifyAction::Add).is_some(),
                    eligible
                );
                assert_eq!(
                    view.edit_selection(ArchiveJobModifyAction::Delete)
                        .is_some(),
                    eligible || refusal == Some(R::UnverifiedMetadata)
                );
                assert_eq!(
                    view.edit_selection(ArchiveJobModifyAction::Rename)
                        .is_some(),
                    eligible
                );
            })
        });
        surface.update(|window, _| {
            let nodes = gpui_kit::base::test_support::snapshots(window);
            let add = nodes
                .iter()
                .find(|node| node.label() == Some("添加") && node.role() == Some(Role::Button))
                .expect("original official toolbar button");
            assert_eq!(add.disabled() == Some(true), !eligible);
        });
    }
    surface.update(|_, cx| {
        view.update(cx, |view, cx| {
            view.edit_source = None;
            cx.notify();
        })
    });
    key(&mut surface, "delete");
    assert!(
        surface.update(|window, cx| window.has_active_dialog(cx)),
        "shortcut refuses through original dialog without dispatch"
    );
}

#[gpui_kit::test]
fn sevenz_edit_proven_facts_enable_original_controls_and_preserve_zip(cx: &mut TestAppContext) {
    use nxtools_protocol::archive_job::{ArchiveModificationRefusal as R, ArchiveOperation as O};
    let (view, mut surface) = mount(cx, 2);
    click(&mut surface, 0, Modifiers::default(), 1);
    surface.update(|_, cx| {
        view.update(cx, |view, cx| {
            view.format = "SevenZ".into();
            let context = nxtools_protocol::archive_browser::ArchiveBrowserContext {
                snapshot_id: view.model.as_ref().expect("snapshot").snapshot,
                format: "SevenZ".into(),
                comment: String::new(),
                source: capability_source(),
                sevenz_edit: Some(nxtools_protocol::archive_browser::SevenzEditEligibility {
                    refusal: None,
                }),
                tar_edit: None,
            };
            view.bind_edit_context(&context, cx);
            assert_eq!(
                view.operation_refusal(O::Add),
                None,
                "strict isolated-worker facts admit ordinary 7z"
            );
            assert_eq!(
                view.edit_selection(ArchiveJobModifyAction::ReplaceContent)
                    .expect("one original file")
                    .1,
                vec![0]
            );
            assert_eq!(view.operation_refusal(O::Comment), Some(R::BackendMissing));
            cx.notify();
        })
    });
    draw(&mut surface);
    surface.update(|window, _| {
        let nodes = gpui_kit::base::test_support::snapshots(window);
        let replace = nodes
            .iter()
            .find(|node| node.label() == Some("替换内容") && node.role() == Some(Role::Button))
            .expect("official original toolbar action");
        assert_ne!(replace.disabled(), Some(true));
    });
    surface.update(|_, cx| {
        view.update(cx, |view, _| {
            view.edit_eligibility = None;
            assert_eq!(view.operation_refusal(O::Add), Some(R::UnverifiedMetadata));
            view.edit_eligibility = Some(Err(R::UnsupportedEncryption));
            assert_eq!(
                view.operation_refusal(O::Delete),
                Some(R::UnsupportedEncryption)
            );
            view.edit_eligibility = Some(Ok(()));
            view.status = BrowserStatus::Stale("refresh failed".into());
            assert_eq!(
                view.operation_refusal(O::ReplaceContent),
                Some(R::SourceChanged)
            );
            view.status = BrowserStatus::Ready;
            view.format = "Zip".into();
            assert_eq!(
                view.operation_refusal(O::ReplaceContent),
                Some(R::BackendMissing)
            );
            assert_eq!(view.operation_refusal(O::Add), None);
        })
    });
}

#[gpui_kit::test]
fn sevenz_edit_context_binding_requires_matching_worker_snapshot_and_keeps_readonly_healthy(
    cx: &mut TestAppContext,
) {
    use nxtools_protocol::archive_browser::{ArchiveBrowserContext, SevenzEditEligibility};
    use nxtools_protocol::archive_job::{ArchiveModificationRefusal as R, ArchiveOperation as O};
    let (view, mut surface) = mount(cx, 2);
    surface.update(|_, cx| {
        view.update(cx, |view, cx| {
            view.format = "SevenZ".into();
            let mut context = ArchiveBrowserContext {
                snapshot_id: view.model.as_ref().expect("snapshot").snapshot,
                format: "SevenZ".into(),
                comment: String::new(),
                source: capability_source(),
                sevenz_edit: None,
                tar_edit: None,
            };
            view.bind_edit_context(&context, cx);
            assert_eq!(view.operation_refusal(O::Add), Some(R::UnverifiedMetadata));
            context.sevenz_edit = Some(SevenzEditEligibility {
                refusal: Some(R::UnsupportedStructure),
            });
            view.bind_edit_context(&context, cx);
            assert_eq!(
                view.operation_refusal(O::Delete),
                Some(R::UnsupportedStructure)
            );
            assert!(
                matches!(view.status, BrowserStatus::Ready),
                "readonly is not a global read error"
            );
            context.sevenz_edit = Some(SevenzEditEligibility { refusal: None });
            context.snapshot_id = Uuid::new_v4();
            view.bind_edit_context(&context, cx);
            assert_eq!(
                view.operation_refusal(O::Add),
                Some(R::UnsupportedStructure),
                "old worker context cannot admit new snapshot"
            );
            context.snapshot_id = view.model.as_ref().expect("snapshot").snapshot;
            view.bind_edit_context(&context, cx);
            for operation in [
                O::Add,
                O::Delete,
                O::RenameFile,
                O::RenameDirectorySubtree,
                O::NewDirectory,
                O::ReplaceContent,
            ] {
                assert_eq!(view.operation_refusal(operation), None);
            }
        })
    });
}

#[gpui_kit::test]
fn sevenz_implicit_directory_rename_uses_original_descendant_not_sorted_row(
    cx: &mut TestAppContext,
) {
    let (view, mut surface) = mount(cx, 2);
    surface.update(|window, cx| {
        view.update(cx, |view, cx| {
            view.install(
                Uuid::new_v4(),
                vec![entry(71, "a/b/c.txt".into()), entry(42, "z.txt".into())],
                "SevenZ".into(),
                window,
                cx,
            );
            view.edit_source = Some(capability_source());
            view.edit_eligibility = Some(Ok(()));
            let model = view.model.as_mut().expect("snapshot");
            model.select(RowId::Folder(model.snapshot, "a".into()), false, false);
            let (_, ordinals, prefix) = view
                .edit_selection(ArchiveJobModifyAction::Rename)
                .expect("implicit directory");
            assert_eq!(ordinals, vec![71]);
            assert_eq!(
                prefix.as_deref(),
                Some("a/"),
                "not immediate descendant parent a/b/"
            );
            view.status = BrowserStatus::Stale("source changed".into());
            assert!(
                view.edit_selection(ArchiveJobModifyAction::Rename)
                    .is_none()
            );
        })
    });
}

#[gpui_kit::test]
fn tar_edit_worker_facts_enable_original_actions_and_missing_facts_refuse(cx: &mut TestAppContext) {
    use nxtools_protocol::archive_browser::{ArchiveBrowserContext, TarEditEligibility};
    use nxtools_protocol::archive_job::{ArchiveModificationRefusal as R, ArchiveOperation as O};
    let (view, mut surface) = mount(cx, 2);
    click(&mut surface, 0, Modifiers::default(), 1);
    for format in ["Tar", "TarGz", "TarXz"] {
        surface.update(|_, cx| {
            view.update(cx, |view, cx| {
                view.format = format.into();
                let mut context = ArchiveBrowserContext {
                    snapshot_id: view.model.as_ref().expect("model").snapshot,
                    format: format.into(),
                    comment: String::new(),
                    source: capability_source(),
                    sevenz_edit: None,
                    tar_edit: None,
                };
                view.bind_edit_context(&context, cx);
                assert_eq!(view.operation_refusal(O::Add), Some(R::UnverifiedMetadata));
                context.tar_edit = Some(TarEditEligibility { refusal: None });
                view.bind_edit_context(&context, cx);
                for operation in [
                    O::Add,
                    O::Delete,
                    O::ReplaceContent,
                    O::RenameFile,
                    O::RenameDirectorySubtree,
                    O::NewDirectory,
                ] {
                    assert_eq!(view.operation_refusal(operation), None);
                }
                assert_eq!(
                    view.edit_selection(ArchiveJobModifyAction::ReplaceContent)
                        .expect("original ordinal")
                        .1,
                    vec![0]
                );
                assert_eq!(view.operation_refusal(O::Comment), Some(R::BackendMissing));
                context.tar_edit = Some(TarEditEligibility {
                    refusal: Some(R::UnsupportedStructure),
                });
                view.bind_edit_context(&context, cx);
                assert_eq!(
                    view.operation_refusal(O::Delete),
                    Some(R::UnsupportedStructure)
                );
                context.tar_edit = Some(TarEditEligibility { refusal: None });
                view.bind_edit_context(&context, cx);
            })
        });
        draw(&mut surface);
        surface.update(|window, _| {
            let nodes = gpui_kit::base::test_support::snapshots(window);
            assert!(nodes.iter().any(|node| node.label() == Some("替换内容")
                && node.role() == Some(Role::Button)
                && node.disabled() != Some(true)));
        });
    }
}

#[gpui_kit::test]
fn step4_directory_refresh_transforms_paths_and_drops_stale_generation(cx: &mut TestAppContext) {
    use nxtools_protocol::archive_job::ArchiveJobModify;
    let (view, mut surface) = mount(cx, 0);
    for (action, value, next, actual, expected) in [
        ("rename", Some("renamed/"), 2, 2, "renamed/child"),
        ("delete", None, 3, 3, ""),
        ("rename", Some("renamed/"), 4, 5, ""),
    ] {
        view.update(&mut surface, |view, cx| {
            let snapshot = Uuid::new_v4();
            view.settings.auto_enter_single_root = false;
            let model = BrowserModel::new(snapshot, vec![entry(71, "a/child/old.txt".into())]);
            view.model = Some(model);
            view.model.as_mut().expect("model").enter("a/child");
            let selected = RowId::Entry(snapshot, 71);
            view.model.as_mut().expect("model").select(selected, false, false);
            let intent: ArchiveJobModify = serde_json::from_value(serde_json::json!({
                "browser":{"snapshot_id":snapshot,"format":"Tar","comment":"","source":capability_source(),"tar_edit":{"refusal":null}},
                "action":action,"ordinals":[71],"value":value,"source_prefix":if action=="rename" {Some("a/")}else{None}
            })).expect("typed intent");
            view.prepare_edit_refresh(&intent, next);
            view.generation = actual;
            cx.notify();
        });
        surface.update(|window, cx| {
            view.update(cx, |view, cx| {
                let fresh = Uuid::new_v4();
                view.install(
                    fresh,
                    vec![
                        entry(0, "renamed/child/old.txt".into()),
                        entry(1, "keep.txt".into()),
                    ],
                    "Tar".into(),
                    window,
                    cx,
                );
                let model = view.model.as_ref().expect("new model");
                assert_eq!(model.folder, expected);
                if action == "rename" && next == actual {
                    assert!(model.selected.contains(&RowId::Entry(fresh, 0)));
                    assert_eq!(model.selected.len(), 1);
                } else {
                    assert!(model.selected.is_empty());
                }
                assert!(view.edit_refresh.is_none(), "one-shot generation hint");
            })
        });
    }
}

fn opt05_browser_catalog(browser: &mut ArchiveBrowser) -> Vec<u64> {
    let snapshot = Uuid::new_v4();
    let mut entries = (0..61_440)
        .map(|ordinal| entry(ordinal, format!("unselected-{ordinal:05}")))
        .collect::<Vec<_>>();
    for ordinal in 61_440..65_536 {
        let mut directory = entry(ordinal, format!("d{ordinal:05}/"));
        directory.kind = "Directory".into();
        directory.size = None;
        entries.push(directory);
    }
    let mut model = BrowserModel::new(snapshot, entries);
    model
        .selected
        .extend((61_440..65_536).map(|ordinal| RowId::Folder(snapshot, format!("d{ordinal:05}"))));
    assert_eq!(model.entries.len(), 65_536);
    assert_eq!(model.selected.len(), 4_096);
    browser.model = Some(model);
    browser.edit_eligibility = Some(Ok(()));
    browser.edit_source = Some(capability_source());
    (61_440..65_536).collect()
}

#[gpui_kit::test]
fn opt05_before_browser_selection_three_trials(cx: &mut TestAppContext) {
    let (view, mut surface) = mount(cx, 0);
    view.update(&mut surface, |browser, _| {
        let expected = opt05_browser_catalog(browser);
        for trial in 1..=3 {
            for boundary in ["extract", "delete", "edit_admission"] {
                let mut work = SelectionWork::default();
                let started = std::time::Instant::now();
                let (ordinals, admitted) = match boundary {
                    "extract" => (
                        browser
                            .selected_ordinals_observed(&mut work)
                            .expect("extract selection"),
                        true,
                    ),
                    "delete" => (
                        browser
                            .edit_selection_observed(ArchiveJobModifyAction::Delete, &mut work)
                            .expect("exactly 4096 empty directories fit original intent limit")
                            .1,
                        true,
                    ),
                    _ => (
                        Vec::new(),
                        browser.edit_members_current_observed(
                            ArchiveJobModifyAction::Delete,
                            &expected,
                            None,
                            None,
                            &mut work,
                        ),
                    ),
                };
                let elapsed_us = started.elapsed().as_micros();
                assert!(admitted);
                if boundary != "edit_admission" {
                    assert_eq!(ordinals, expected);
                }
                eprintln!(
                    "OPT05_SELECTION {}",
                    serde_json::json!({
                        "boundary": boundary, "trial": trial,
                        "build_mode": if cfg!(debug_assertions) { "debug" } else { "release" },
                        "catalog": 65_536, "selected_ids": expected.len(),
                        "elapsed_us": elapsed_us, "work": work,
                    })
                );
            }
        }
    });
}

#[gpui_kit::test]
fn opt05_red_browser_extract_single_pass(cx: &mut TestAppContext) {
    let (view, mut surface) = mount(cx, 0);
    view.update(&mut surface, |browser, _| {
        let expected = opt05_browser_catalog(browser);
        let mut work = SelectionWork::default();
        assert_eq!(
            browser
                .selected_ordinals_observed(&mut work)
                .expect("extract"),
            expected
        );
        assert!(
            work.catalog_visits <= 65_536
                && work.path_normalizations <= 65_536
                && work.prefix_probes <= 65_536 * 16 + 4_096 * 64,
            "Extract must expand once, not once per selected directory: {work:?}"
        );
    });
}

#[gpui_kit::test]
fn opt05_red_browser_delete_single_pass(cx: &mut TestAppContext) {
    let (view, mut surface) = mount(cx, 0);
    view.update(&mut surface, |browser, _| {
        let expected = opt05_browser_catalog(browser);
        let mut work = SelectionWork::default();
        assert_eq!(
            browser
                .edit_selection_observed(ArchiveJobModifyAction::Delete, &mut work)
                .expect("delete")
                .1,
            expected
        );
        assert!(
            work.catalog_visits <= 65_536
                && work.path_normalizations <= 65_536
                && work.prefix_probes <= 65_536 * 16 + 4_096 * 64,
            "Delete must expand once, not once per selected directory: {work:?}"
        );
    });
}

#[gpui_kit::test]
fn opt05_red_browser_edit_admission_single_lookup(cx: &mut TestAppContext) {
    let (view, mut surface) = mount(cx, 0);
    view.update(&mut surface, |browser, _| {
        let expected = opt05_browser_catalog(browser);
        let mut work = SelectionWork::default();
        assert!(browser.edit_members_current_observed(
            ArchiveJobModifyAction::Delete,
            &expected,
            None,
            None,
            &mut work
        ));
        assert!(
            work.ordinal_probes <= 65_536 + 4_096,
            "final edit admission must not linearly find every selected ordinal: {work:?}"
        );
    });
}

#[gpui_kit::test]
fn opt05_browser_selection_preserves_subtrees_empty_hidden_and_stale_guards(
    cx: &mut TestAppContext,
) {
    let (view, mut surface) = mount(cx, 0);
    view.update(&mut surface, |browser, _| {
        let snapshot = Uuid::new_v4();
        let mut entries = [r"a\sub\z", "ab/peer", "a/", "a/empty/", "a/sub/x"]
            .into_iter()
            .enumerate()
            .map(|(ordinal, name)| {
                let mut item = entry(ordinal as u64, name.into());
                if name.ends_with('/') {
                    item.kind = "Directory".into();
                    item.size = None;
                }
                item
            })
            .collect::<Vec<_>>();
        let original = entries
            .iter()
            .map(|entry| entry.name.clone())
            .collect::<Vec<_>>();
        browser.model = Some(BrowserModel::new(snapshot, entries.clone()));
        browser.edit_eligibility = Some(Ok(()));
        browser.edit_source = Some(capability_source());
        browser.model.as_mut().expect("model").select(
            RowId::Folder(snapshot, "a".into()),
            false,
            false,
        );
        let mut work = SelectionWork::default();
        assert_eq!(
            browser
                .selected_ordinals_observed(&mut work)
                .expect("extract subtree"),
            [0, 2, 3, 4]
        );
        assert_eq!(
            browser
                .edit_selection_observed(ArchiveJobModifyAction::Delete, &mut work)
                .expect("delete subtree")
                .1,
            [0, 2, 3, 4]
        );
        let model = browser.model.as_mut().expect("model");
        assert_eq!(
            model
                .entries
                .iter()
                .map(|entry| entry.name.clone())
                .collect::<Vec<_>>(),
            original
        );
        model.enter("a");
        model.filter = "empty".into();
        model.reproject();
        model.select_all();
        assert_eq!(
            browser
                .selected_ordinals()
                .expect("explicit empty directory"),
            [3]
        );
        assert!(
            browser.edit_members_current(ArchiveJobModifyAction::Delete, &[0, 2, 3, 4], None, None),
            "frozen intent is independent of current filtered selection"
        );
        assert!(!browser.edit_members_current(
            ArchiveJobModifyAction::Delete,
            &[65_536],
            None,
            None
        ));
        browser.model.as_mut().expect("model").selected =
            std::collections::BTreeSet::from([RowId::Folder(Uuid::new_v4(), "a/empty".into())]);
        assert!(browser.selected_ordinals().is_none());
        assert!(
            browser
                .edit_selection(ArchiveJobModifyAction::Delete)
                .is_none()
        );
        browser.status = BrowserStatus::Stale("source changed".into());
        assert!(
            browser
                .edit_selection(ArchiveJobModifyAction::Delete)
                .is_none()
        );
        browser.status = BrowserStatus::Ready;
        entries[0].warning_codes.push("OutputCollision".into());
        browser.model = Some(BrowserModel::new(snapshot, entries));
        assert!(!browser.edit_members_current(ArchiveJobModifyAction::Delete, &[0], None, None));
    });
}

#[gpui_kit::test]
fn opt05_browser_noncontiguous_ordinals_bind_once_and_expand_in_catalog_order(
    cx: &mut TestAppContext,
) {
    let (view, mut surface) = mount(cx, 0);
    view.update(&mut surface, |browser, _| {
        let snapshot = Uuid::new_v4();
        let mut empty = entry(3, "a/empty/".into());
        empty.kind = "Directory".into();
        browser.model = Some(BrowserModel::new(
            snapshot,
            vec![
                entry(9, "a\\sub\\z".into()),
                empty,
                entry(7, "ab/peer".into()),
                entry(2, "a/sub/x".into()),
            ],
        ));
        browser.edit_source = Some(capability_source());
        browser.edit_eligibility = Some(Ok(()));
        browser.model.as_mut().expect("original model").select(
            RowId::Folder(snapshot, "a".into()),
            false,
            false,
        );
        let mut work = SelectionWork::default();
        assert_eq!(
            browser
                .selected_ordinals_observed(&mut work)
                .expect("catalog order"),
            [9, 3, 2]
        );
        assert_eq!(
            browser
                .edit_selection(ArchiveJobModifyAction::Delete)
                .expect("delete")
                .1,
            [9, 3, 2]
        );
        work = SelectionWork::default();
        assert!(browser.edit_members_current_observed(
            ArchiveJobModifyAction::Delete,
            &[2, 9],
            None,
            None,
            &mut work
        ));
        assert_eq!(work.ordinal_probes, 6);
        assert!(!browser.edit_members_current(ArchiveJobModifyAction::Delete, &[0], None, None));
    });
}

#[gpui_kit::test]
fn opt05_browser_edit_refresh_binds_frozen_members_once(cx: &mut TestAppContext) {
    let (view, mut surface) = mount(cx, 0);
    view.update(&mut surface, |browser, _| {
        let ordinals = opt05_browser_catalog(browser);
        let snapshot = browser.model.as_ref().expect("original snapshot").snapshot;
        let intent = serde_json::from_value(serde_json::json!({
            "browser": {"snapshot_id": snapshot, "format": "Zip", "comment": "", "source": capability_source()},
            "action": "delete", "ordinals": ordinals, "value": null, "source_prefix": null
        })).expect("frozen delete intent");
        let mut work = SelectionWork::default();
        browser.prepare_edit_refresh_observed(&intent, 7, &mut work);
        let hint = browser.edit_refresh.as_ref().expect("original one-shot hint");
        assert_eq!(hint.generation, 7);
        assert!(hint.selected.is_empty());
        assert_eq!(hint.folder, "");
        assert!(work.ordinal_probes <= 65_536 + 4_096,
            "refresh must bind frozen ordinals once, not per transformed path: {work:?}");
        assert!(work.root_normalizations <= 4_096
            && work.prefix_probes <= 65_536 * 16 + 4_096 * 64, "{work:?}");
    });
}

#[gpui_kit::test]
fn opt07_backspace_selects_focuses_and_reveals_departed_child_and_root_is_noop(
    cx: &mut TestAppContext,
) {
    let (view, mut surface) = mount(cx, 0);
    let snapshot = Uuid::new_v4();
    surface.update(|window, cx| {
        view.update(cx, |v, cx| {
            v.install(
                snapshot,
                (0..240)
                    .map(|i| entry((i * 7 + 3) as u64, format!("folder-{i:03}/child.txt")))
                    .collect(),
                "Zip".into(),
                window,
                cx,
            );
            v.navigate("folder-219".into(), window, cx);
            v.list_focus.focus(window, cx);
        })
    });
    draw(&mut surface);
    key(&mut surface, "backspace");
    let expected = RowId::Folder(snapshot, "folder-219".into());
    view.read_with(&surface, |v, _| {
        let model = v.model.as_ref().expect("same catalog");
        assert!(model.folder.is_empty());
        assert_eq!(
            model.selected.iter().cloned().collect::<Vec<_>>(),
            vec![expected.clone()]
        );
        assert_eq!(model.focused.as_ref(), Some(&expected));
    });
    let row = surface
        .debug_bounds("archive-folder-folder-219")
        .expect("departed child rendered after reveal");
    let viewport = surface
        .debug_bounds("archive-horizontal")
        .expect("original list viewport");
    assert!(
        row.top() >= viewport.top() && row.bottom() <= viewport.bottom(),
        "selected child is visible"
    );
    key(&mut surface, "backspace");
    view.read_with(&surface, |v, _| {
        let model = v.model.as_ref().expect("root catalog");
        assert_eq!(
            model.selected.iter().cloned().collect::<Vec<_>>(),
            vec![expected.clone()]
        );
        assert_eq!(
            model.focused.as_ref(),
            Some(&expected),
            "root Up cannot clear current selection"
        );
    });
}

#[gpui_kit::test]
fn opt07_archive_find_shortcut_is_independent_of_current_folder_filter(cx: &mut TestAppContext) {
    let (view, mut surface) = mount(cx, 4);
    click(&mut surface, 1, Modifiers::default(), 1);
    let selected = view.read_with(&surface, |v, _| {
        v.model.as_ref().expect("catalog").selected.clone()
    });
    key(&mut surface, "ctrl-f");
    assert!(view.read_with(&surface, |v, _| v.filter_visible));
    assert!(!surface.update(|window, cx| window.has_active_dialog(cx)));
    key(&mut surface, "escape");
    key(&mut surface, "ctrl-shift-f");
    assert!(
        surface.update(|window, cx| window.has_active_dialog(cx)),
        "archive-wide Find opens in the original Root"
    );
    assert_eq!(
        view.read_with(&surface, |v, _| v
            .model
            .as_ref()
            .expect("catalog")
            .selected
            .clone()),
        selected,
        "opening Find does not replace unrelated browser selection"
    );
    assert!(matches!(
        view.read_with(&surface, |v, _| v.status.clone()),
        BrowserStatus::Ready
    ));
}

#[gpui_kit::test]
fn opt07_refresh_and_encoding_restore_location_or_nearest_surviving_ancestor(
    cx: &mut TestAppContext,
) {
    let (view, mut surface) = mount(cx, 0);
    surface.update(|window, cx| {
        view.update(cx, |v, cx| {
            v.settings.auto_enter_single_root = false;
            v.install(
                Uuid::new_v4(),
                vec![
                    entry(71, "a/b/item.txt".into()),
                    entry(99, "other.txt".into()),
                ],
                "Zip".into(),
                window,
                cx,
            );
            v.navigate("a/b".into(), window, cx);
            let id = v
                .model
                .as_ref()
                .expect("catalog")
                .visible()
                .iter()
                .find(|r| !r.parent)
                .expect("file")
                .id
                .clone();
            v.model.as_mut().expect("catalog").select(id, false, false);
            v.command(Command::Refresh, window, cx);
            v.generation += 1;
            v.install(
                Uuid::new_v4(),
                vec![
                    entry(107, "a/b/item.txt".into()),
                    entry(119, "other.txt".into()),
                ],
                "Zip".into(),
                window,
                cx,
            );
            let model = v.model.as_ref().expect("fresh snapshot");
            assert_eq!(
                model.folder, "a/b",
                "refresh preserves an existing internal location"
            );
            assert!(
                model.selected.is_empty() && model.focused.is_none(),
                "old immutable IDs retire"
            );
            v.command(Command::Encoding(936), window, cx);
            v.generation += 1;
            v.install(
                Uuid::new_v4(),
                vec![
                    entry(201, "a/sibling.txt".into()),
                    entry(219, "other.txt".into()),
                ],
                "Zip".into(),
                window,
                cx,
            );
            assert_eq!(
                v.model.as_ref().expect("encoding snapshot").folder,
                "a",
                "missing child restores nearest surviving ancestor"
            );
            assert_eq!(v.codepage, 936);
        })
    });
    draw(&mut surface);
}

#[gpui_kit::test]
fn opt07_find_query_pointer_and_enter_navigate_original_noncontiguous_member(
    cx: &mut TestAppContext,
) {
    let (view, mut surface) = mount(cx, 0);
    let snapshot = Uuid::new_v4();
    surface.update(|window, cx| {
        view.update(cx, |v, cx| {
            v.settings.auto_enter_single_root = false;
            v.install(
                snapshot,
                vec![
                    entry(71, "a/needle.txt".into()),
                    entry(503, r"b\needle.txt".into()),
                    entry(999, "unrelated.txt".into()),
                ],
                "Zip".into(),
                window,
                cx,
            );
            let model = v.model.as_mut().expect("catalog");
            model.select(RowId::Entry(snapshot, 999), false, false);
            v.update_summary();
            v.list_focus.focus(window, cx);
        })
    });
    key(&mut surface, "ctrl-shift-f");
    assert!(
        surface.update(|window, cx| window.has_active_dialog(cx)),
        "original Find modal"
    );
    surface.simulate_input("needle");
    draw(&mut surface);
    click_named(&mut surface, "b/needle.txt", Role::ListBoxOption);
    assert_eq!(
        view.read_with(&surface, |v, _| v
            .model
            .as_ref()
            .expect("browser")
            .selected
            .iter()
            .cloned()
            .collect::<Vec<_>>()),
        vec![RowId::Entry(snapshot, 999)],
        "query/result selection is local until explicit navigation"
    );
    key(&mut surface, "enter");
    assert!(!surface.update(|window, cx| window.has_active_dialog(cx)));
    view.read_with(&surface, |v, _| {
        let model = v.model.as_ref().expect("same catalog");
        assert_eq!(model.folder, "b");
        assert_eq!(
            model.selected.iter().cloned().collect::<Vec<_>>(),
            vec![RowId::Entry(snapshot, 503)]
        );
        assert_eq!(model.focused, Some(RowId::Entry(snapshot, 503)));
        assert!(
            v.open_dispatches.is_empty(),
            "Find navigation never executes the member"
        );
        assert!(!v.open_pending && v.read_id.is_none());
    });
}

#[gpui_kit::test]
fn opt07_find_menu_invalid_query_empty_and_cancel_leave_browser_selection(cx: &mut TestAppContext) {
    let (view, mut surface) = mount(cx, 3);
    click(&mut surface, 1, Modifiers::default(), 1);
    let selected = view.read_with(&surface, |v, _| {
        v.model.as_ref().expect("catalog").selected.clone()
    });
    click_named(&mut surface, "查看", Role::Button);
    click_named(&mut surface, "查找整个压缩包  Ctrl+Shift+F", Role::MenuItem);
    assert!(surface.update(|window, cx| window.has_active_dialog(cx)));
    surface.simulate_input("no-such-member");
    draw(&mut surface);
    assert!(
        surface.debug_bounds("archive-find-empty").is_some(),
        "no-match state is explicit"
    );
    surface.simulate_keystrokes("ctrl-a");
    surface.simulate_input("[");
    click_named(&mut surface, "通配符", Role::CheckBox);
    assert!(
        surface.debug_bounds("archive-find-invalid").is_some(),
        "invalid wildcard cannot reuse old results"
    );
    click_named(&mut surface, "关闭查找", Role::Button);
    assert!(!surface.update(|window, cx| window.has_active_dialog(cx)));
    assert_eq!(
        view.read_with(&surface, |v, _| v
            .model
            .as_ref()
            .expect("catalog")
            .selected
            .clone()),
        selected
    );
}

#[gpui_kit::test]
fn opt07_repeated_render_does_not_expand_catalog_and_keeps_virtual_controls_bounded(
    cx: &mut TestAppContext,
) {
    let (view, mut surface) = mount(cx, 10_000);
    view.update(&mut surface, |v, cx| {
        v.format = "Tar".into();
        v.edit_source = Some(capability_source());
        v.edit_eligibility = Some(Ok(()));
        cx.notify();
    });
    surface.update(|window, cx| view.update(cx, |v, cx| v.list_focus.focus(window, cx)));
    key(&mut surface, "end");
    assert_eq!(
        view.read_with(&surface, |v, _| v
            .model
            .as_ref()
            .expect("catalog")
            .selected
            .len()),
        1,
        "actual keyboard selection before measurement"
    );
    let projection = view.read_with(&surface, |v, _| {
        v.model.as_ref().expect("loaded catalog").work.get()
    });
    view.update(&mut surface, |v, cx| {
        v.catalog_presentation_visits.set(0);
        v.rendered_rows.set(0);
        cx.notify();
    });
    draw(&mut surface);
    draw(&mut surface);
    view.read_with(&surface, |v, _| {
        let visits = v.catalog_presentation_visits.get();
        let rendered = v.rendered_rows.get();
        println!("OPT07 render catalog=10000 projection={projection:?} repeated_catalog_visits={visits} rendered_controls={rendered}");
        assert!(rendered > 0 && rendered < 200, "only visible controls are built");
        assert_eq!(v.model.as_ref().expect("same catalog").work.get(), projection, "render does not rebuild model/tree");
        assert_eq!(visits, 0, "render uses transition-owned readiness, not full catalog scans");
    });
}

#[gpui_kit::test]
fn opt07_find_explicit_implicit_directories_and_duplicate_names_keep_original_ids(
    cx: &mut TestAppContext,
) {
    let (view, mut surface) = mount(cx, 0);
    let snapshot = Uuid::new_v4();
    surface.update(|window, cx| {
        view.update(cx, |v, cx| {
            let mut directory = entry(101, "explicit/".into());
            directory.kind = "Directory".into();
            v.install(
                snapshot,
                vec![
                    directory,
                    entry(717, "implicit/child.txt".into()),
                    entry(23, "x/same.txt".into()),
                    entry(77, r"y\same.txt".into()),
                ],
                "Zip".into(),
                window,
                cx,
            );
            v.list_focus.focus(window, cx);
        })
    });
    for (query, label, expected) in [
        (
            "explicit",
            "explicit",
            RowId::Folder(snapshot, "explicit".into()),
        ),
        (
            "implicit",
            "implicit",
            RowId::Folder(snapshot, "implicit".into()),
        ),
        ("same", "y/same.txt", RowId::Entry(snapshot, 77)),
    ] {
        key(&mut surface, "ctrl-shift-f");
        surface.simulate_input(query);
        draw(&mut surface);
        let selector = if query == "same" {
            "archive-find-result-1"
        } else {
            "archive-find-result-0"
        };
        let position = surface
            .debug_bounds(selector)
            .expect("Find result bounds")
            .center();
        surface.simulate_click(position, Default::default());
        draw(&mut surface);
        assert!(
            surface.update(|window, _| gpui_kit::base::test_support::snapshots(window)
                .iter()
                .any(|node| node.label() == Some(label) && node.selected() == Some(true))),
            "{query}: exact Find result selected"
        );
        key(&mut surface, "enter");
        assert_eq!(
            view.read_with(&surface, |v, _| v
                .model
                .as_ref()
                .expect("same snapshot")
                .selected
                .iter()
                .cloned()
                .collect::<Vec<_>>()),
            vec![expected],
            "{query}: original identity selected after Enter"
        );
        assert!(view.read_with(&surface, |v, _| v.open_dispatches.is_empty()));
    }
}

#[gpui_kit::test]
fn opt07_find_can_locate_original_warning_row_without_executing(cx: &mut TestAppContext) {
    let (view, mut surface) = mount(cx, 0);
    let snapshot = Uuid::new_v4();
    surface.update(|window, cx| {
        view.update(cx, |v, cx| {
            let mut warning = entry(411, "folder/collision.txt".into());
            warning.warning_codes = vec!["OutputCollision".into()];
            v.install(
                snapshot,
                vec![warning, entry(777, "other.txt".into())],
                "Zip".into(),
                window,
                cx,
            );
            v.list_focus.focus(window, cx);
        })
    });
    key(&mut surface, "ctrl-shift-f");
    surface.simulate_input("collision");
    draw(&mut surface);
    click_named(&mut surface, "folder/collision.txt", Role::ListBoxOption);
    key(&mut surface, "enter");
    assert_eq!(
        view.read_with(&surface, |v, _| v
            .model
            .as_ref()
            .expect("catalog")
            .focused
            .clone()),
        Some(RowId::Entry(snapshot, 411))
    );
    assert!(view.read_with(&surface, |v, _| v.open_dispatches.is_empty()));
}
