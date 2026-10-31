use super::*;
use nxtools_protocol::archive_read::ArchiveReadEntry;
fn entry(ordinal: u64, name: &str, kind: &str, size: Option<u64>) -> ArchiveReadEntry {
    serde_json::from_value(serde_json::json!({
        "ordinal": ordinal, "name": name, "kind": kind, "size": size,
        "packed_size": null, "encrypted": false, "warning_codes": [],
        "provenance": "Utf8Flag", "codepage": 65001
    }))
    .expect("synthetic metadata")
}
fn fixture() -> BrowserModel {
    BrowserModel::new(
        Uuid::new_v4(),
        vec![
            entry(0, "root/a.txt", "File", Some(4)),
            entry(1, "root/b.txt", "File", Some(8)),
            entry(2, "other/c.txt", "File", Some(2)),
            entry(3, "empty/", "Directory", None),
        ],
    )
}
#[test]
fn implicit_folders_parent_and_unknown_sizes_are_truthful() {
    let mut m = fixture();
    assert_eq!(m.visible().iter().filter(|r| r.directory).count(), 3);
    m.enter("root");
    assert!(m.visible()[0].parent);
    assert_eq!(m.visible().len(), 3);
    assert_eq!(m.visible()[1].packed_size, None);
    m.up();
    assert_eq!(m.folder, "");
}
#[test]
fn selection_survives_sort_filter_by_identity_not_index() {
    let mut m = fixture();
    m.enter("root");
    let first = m.visible()[1].id.clone();
    m.select(first.clone(), false, false);
    m.sort = SortKey::Size;
    m.descending = true;
    m.reproject();
    assert_eq!(m.selected_file_ordinals(), vec![0]);
    m.filter = "b.txt".into();
    m.reproject();
    assert!(
        m.selected_file_ordinals().is_empty(),
        "hidden items must not be action targets"
    );
    m.filter.clear();
    m.reproject();
    m.select(first, false, false);
    assert_eq!(m.selected_file_ordinals(), vec![0]);
}
#[test]
fn ctrl_shift_keyboard_and_right_click_share_one_owner() {
    let mut m = fixture();
    m.enter("root");
    let a = m.visible()[1].id.clone();
    let b = m.visible()[2].id.clone();
    m.select(a.clone(), false, false);
    m.select(b.clone(), true, false);
    assert_eq!(m.selected.len(), 2);
    m.right_select(a.clone());
    assert_eq!(m.selected.len(), 2);
    m.select(a, false, false);
    m.move_focus(1, true);
    assert_eq!(m.selected.len(), 2);
    m.select_all();
    assert_eq!(m.selected.len(), 2);
    m.enter("other");
    assert!(m.selected.is_empty());
    assert!(m.selected_file_ordinals().is_empty());
}
#[test]
fn unsafe_and_duplicate_names_keep_distinct_ordinals() {
    let id = Uuid::new_v4();
    let mut m = BrowserModel::new(
        id,
        vec![
            entry(0, "same.txt", "File", None),
            entry(1, "same.txt", "File", None),
            entry(2, "../bad", "File", None),
        ],
    );
    assert_eq!(m.visible().len(), 3);
    let ids = m
        .visible()
        .iter()
        .map(|r| r.id.clone())
        .collect::<BTreeSet<_>>();
    assert_eq!(ids.len(), 3);
    m.select_all();
    assert_eq!(
        m.selected_file_ordinals()
            .into_iter()
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([0, 1, 2])
    );
}

#[test]
fn plain_repeat_selection_is_idempotent_and_unfocused_down_starts_first() {
    let mut m = fixture();
    m.enter("root");
    let id = m.visible()[1].id.clone();
    m.select(id.clone(), false, false);
    m.select(id.clone(), false, false);
    assert!(
        m.selected.contains(&id),
        "double click must not deselect its action target"
    );
    m.enter("");
    let first = m.visible()[0].id.clone();
    m.move_focus(1, false);
    assert_eq!(m.focused, Some(first));
}

#[test]
fn actual_modified_and_optional_metadata_sort_do_not_fabricate_values() {
    let mut a = entry(0, "z.txt", "File", None);
    let mut b = entry(1, "a.txt", "File", None);
    a.method = Some(8);
    b.method = Some(0);
    a.modified = Some(
        nxtools_protocol::archive_browser::ArchiveModifiedTime::UnixUtc {
            seconds: 1_700_000_000,
            nanoseconds: 0,
            precision_ns: 1_000_000_000,
        },
    );
    let mut m = BrowserModel::new(Uuid::new_v4(), vec![a, b]);
    m.sort = SortKey::Method;
    m.reproject();
    assert_eq!(m.visible()[0].ordinal, Some(1));
    m.sort = SortKey::Modified;
    m.descending = true;
    m.reproject();
    assert_eq!(m.visible()[0].ordinal, Some(0));
    assert!(m.visible()[1].modified.is_none());
}

#[test]
fn directories_remain_first_in_both_sort_directions_and_unknown_stays_none() {
    let mut m = BrowserModel::new(
        Uuid::new_v4(),
        vec![
            entry(0, "z.txt", "File", Some(1)),
            entry(1, "a/", "Directory", None),
            entry(2, "unknown.bin", "File", None),
        ],
    );
    for descending in [false, true] {
        m.sort = SortKey::Size;
        m.descending = descending;
        m.reproject();
        assert!(m.visible()[0].directory);
        assert!(
            m.visible()
                .iter()
                .find(|r| r.ordinal == Some(2))
                .expect("unknown")
                .size
                .is_none()
        );
    }
}

#[test]
fn encoding_uncertainty_is_not_an_unsafe_path_rejection() {
    let mut uncertain = entry(0, "readme.txt", "File", Some(2));
    uncertain.warning_codes = vec!["UncertainEncoding".into()];
    let m = BrowserModel::new(Uuid::new_v4(), vec![uncertain]);
    assert!(!m.visible()[0].warning);
}

#[test]
fn drag_tree_overlap_and_hidden_descendants_preserve_original_identity() {
    let snapshot = Uuid::new_v4();
    let mut model = BrowserModel::new(
        snapshot,
        vec![
            entry(8, "root/sub/child.txt", "File", None),
            entry(3, "root/", "Directory", Some(0)),
            entry(9, "root/empty/", "Directory", None),
            entry(4, "root/peer.txt", "File", Some(3)),
        ],
    );
    model.selected.extend([
        RowId::Folder(snapshot, "root".into()),
        RowId::Folder(snapshot, "root/sub".into()),
        RowId::Entry(snapshot, 8),
    ]);
    let frozen = model.freeze_drag_selection().expect("overlap dedup");
    assert_eq!(
        frozen.iter().map(|i| i.name.as_str()).collect::<Vec<_>>(),
        vec![
            "root",
            "root/empty",
            "root/peer.txt",
            "root/sub",
            "root/sub/child.txt"
        ]
    );
    assert_eq!(
        frozen.iter().filter_map(|i| i.ordinal).collect::<Vec<_>>(),
        vec![3, 9, 4, 8]
    );
    assert_eq!(frozen.last().expect("child").size, None);
    model.enter("root");
    model.filter = "sub".into();
    model.reproject();
    model.select_all();
    let frozen = model
        .freeze_drag_selection()
        .expect("filtered folder retains complete descendants");
    assert_eq!(
        frozen.iter().map(|i| i.name.as_str()).collect::<Vec<_>>(),
        vec!["sub", "sub/child.txt"]
    );
    assert_eq!(frozen[1].ordinal, Some(8));
    model.entries[0].encrypted = Some(true);
    assert!(
        model.freeze_drag_selection().is_err(),
        "hidden encrypted descendant rejects whole selection"
    );
}

#[test]
fn drag_tree_rejects_conflicts_unsafe_links_unknown_encryption_and_each_finite_limit() {
    for entries in [
        vec![
            entry(0, "root/a", "File", Some(1)),
            entry(1, "root/a/b", "File", Some(1)),
        ],
        vec![
            entry(0, "root/a", "File", Some(1)),
            entry(1, "ROOT/b", "File", Some(1)),
        ],
        vec![
            entry(0, "root/a", "File", Some(1)),
            entry(1, "root/A", "File", Some(1)),
        ],
        vec![entry(0, "root/../bad", "File", Some(1))],
        vec![entry(0, "root//", "Directory", None)],
        vec![entry(0, "root/file/", "File", Some(1))],
        vec![entry(0, "root/link", "SymbolicLink", Some(1))],
    ] {
        let mut model = BrowserModel::new(Uuid::new_v4(), entries);
        model.select_all();
        assert!(
            model.freeze_drag_selection().is_err(),
            "unsafe snapshot tree refused"
        );
    }
    let mut model = fixture();
    model.select_all();
    model.entries[0].encrypted = None;
    assert!(model.freeze_drag_selection().is_err());
    for (count, directory) in [(33, false), (129, true)] {
        let mut model = BrowserModel::new(
            Uuid::new_v4(),
            (0..count)
                .map(|ordinal| {
                    entry(
                        ordinal,
                        &format!("root/item{ordinal}"),
                        if directory { "Directory" } else { "File" },
                        Some(1),
                    )
                })
                .collect(),
        );
        model.select_all();
        assert_eq!(
            model.selected.len(),
            1,
            "one selected root, expanded limit separate"
        );
        assert!(
            model.freeze_drag_selection().is_err(),
            "expanded file/descriptor bound"
        );
    }
    let mut model = BrowserModel::new(
        Uuid::new_v4(),
        (0..33)
            .map(|ordinal| entry(ordinal, &format!("empty{ordinal}/"), "Directory", None))
            .collect(),
    );
    model.select_all();
    assert!(
        model.freeze_drag_selection().is_err(),
        "root count independent of file count"
    );
    let mut model = BrowserModel::new(
        Uuid::new_v4(),
        vec![
            entry(0, "a", "File", Some(512 << 20)),
            entry(1, "b", "File", Some(1)),
        ],
    );
    model.select_all();
    assert!(
        model.freeze_drag_selection().is_err(),
        "known aggregate cap"
    );
}
