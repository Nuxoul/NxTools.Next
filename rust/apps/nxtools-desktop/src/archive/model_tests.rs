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
fn drag_tree_preserves_complete_representable_selection_and_rejects_unsafe_paths() {
    for entries in [
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
        assert_eq!(model.selected.len(), 1, "one selected root");
        assert_eq!(
            model
                .freeze_drag_selection()
                .expect("complete subtree")
                .len(),
            count as usize + 1
        );
    }
    let mut model = BrowserModel::new(
        Uuid::new_v4(),
        (0..33)
            .map(|ordinal| entry(ordinal, &format!("empty{ordinal}/"), "Directory", None))
            .collect(),
    );
    model.select_all();
    assert_eq!(model.freeze_drag_selection().expect("all roots").len(), 33);
    let mut model = BrowserModel::new(
        Uuid::new_v4(),
        vec![
            entry(0, "a", "File", Some(512 << 20)),
            entry(1, "b", "File", Some(1)),
        ],
    );
    model.select_all();
    assert_eq!(
        model
            .freeze_drag_selection()
            .expect("known sizes retained")
            .len(),
        2
    );
}

#[test]
fn normal_drag_conflicts_preserve_only_selected_ordinals_and_directory_contents() {
    for entries in [
        vec![
            entry(0, "root/a", "File", Some(1)),
            entry(1, "root/a/b", "File", Some(1)),
        ],
        vec![
            entry(0, "root/a", "File", Some(1)),
            entry(1, "ROOT/b", "File", None),
        ],
        vec![
            entry(0, "root/a", "File", Some(1)),
            entry(1, "root/A", "File", Some(1)),
        ],
        vec![
            entry(0, "root/", "Directory", None),
            entry(1, "root/", "Directory", None),
            entry(2, "root/a", "File", None),
        ],
    ] {
        let mut model = BrowserModel::new(Uuid::new_v4(), entries);
        for entry in &mut model.entries {
            entry.encrypted = None;
        }
        model.select_all();
        let expected = model
            .entries
            .iter()
            .filter(|entry| entry.kind == "File")
            .map(|entry| entry.ordinal)
            .collect::<BTreeSet<_>>();
        let items = model
            .freeze_drag_selection()
            .expect("representable selection");
        assert_eq!(
            items
                .iter()
                .filter(|item| !item.directory)
                .filter_map(|item| item.ordinal)
                .collect::<BTreeSet<_>>(),
            expected
        );
        assert_eq!(
            items
                .iter()
                .map(|item| item.name.to_lowercase())
                .collect::<BTreeSet<_>>()
                .len(),
            items.len()
        );
    }
    let snapshot = Uuid::new_v4();
    let mut model = BrowserModel::new(
        snapshot,
        vec![
            entry(0, "a.txt", "File", Some(1)),
            entry(1, "A.txt", "File", Some(2)),
        ],
    );
    model.selected.insert(RowId::Entry(snapshot, 1));
    let items = model.freeze_drag_selection().expect("one exact ordinal");
    assert_eq!(items.len(), 1);
    assert_eq!(
        items[0].ordinal,
        Some(1),
        "unselected case peer is not an implicit extra selection"
    );
}

#[test]
fn ordinary_directory_drag_freezer_keeps_all_256_files_and_original_ordinals() {
    let snapshot = Uuid::new_v4();
    let mut model = BrowserModel::new(
        snapshot,
        (0..256)
            .map(|ordinal| {
                entry(
                    ordinal,
                    &format!("ordinary/file-{ordinal:03}.txt"),
                    "File",
                    Some(8),
                )
            })
            .collect(),
    );
    model.select_all();
    assert_eq!(model.selected.len(), 1, "one ordinary directory selection");
    let selection = model.selected.clone();
    let items = model
        .freeze_drag_selection()
        .expect("ordinary directory must not be refused at 128 descriptors or 32 files");
    assert_eq!(items.len(), 257, "explicit parent plus every file");
    assert_eq!(items.iter().filter(|item| !item.directory).count(), 256);
    assert_eq!(
        items
            .iter()
            .filter_map(|item| item.ordinal)
            .collect::<BTreeSet<_>>(),
        (0..256).collect::<BTreeSet<_>>()
    );
    for ordinal in [0, 128, 255] {
        let item = items
            .iter()
            .find(|item| item.ordinal == Some(ordinal))
            .expect("first, middle and last source members retained");
        assert_eq!(item.name, format!("ordinary/file-{ordinal:03}.txt"));
        assert_eq!(item.size, Some(8));
    }
    let tree = model
        .freeze_drag_tree("ordinary", "bundle.zip", false)
        .expect("Tree and list use the same complete freezer");
    let root = model
        .freeze_drag_tree("", "bundle.zip", false)
        .expect("archive root wrapper must not impose a member-count cap");
    assert_eq!(tree.len(), 257);
    assert_eq!(root.len(), 258);
    assert_eq!(root.iter().filter(|item| !item.directory).count(), 256);
    assert_eq!(
        model.selected, selection,
        "freezing does not replace selection"
    );
}

#[test]
fn ordinary_directory_drag_freezer_keeps_more_than_128_empty_directories() {
    let mut model = BrowserModel::new(
        Uuid::new_v4(),
        (0..256)
            .map(|ordinal| {
                entry(
                    ordinal,
                    &format!("ordinary/dir-{ordinal:03}/"),
                    "Directory",
                    None,
                )
            })
            .collect(),
    );
    model.select_all();
    let items = model
        .freeze_drag_selection()
        .expect("complete empty-directory tree");
    assert_eq!(items.len(), 257);
    assert!(
        items
            .iter()
            .all(|item| item.directory && item.size.is_none())
    );
}

#[test]
fn ordinary_drag_freezer_keeps_more_than_32_selected_roots_wholly() {
    let mut model = BrowserModel::new(
        Uuid::new_v4(),
        (0..64)
            .map(|ordinal| entry(ordinal, &format!("file-{ordinal:03}.txt"), "File", Some(8)))
            .collect(),
    );
    model.select_all();
    let items = model
        .freeze_drag_selection()
        .expect("complete multi-selection");
    assert_eq!(items.len(), 64);
    assert_eq!(
        items
            .iter()
            .filter_map(|item| item.ordinal)
            .collect::<BTreeSet<_>>(),
        (0..64).collect::<BTreeSet<_>>()
    );
}

#[test]
fn ordinary_drag_freezer_preserves_actual_64_bit_size_and_unknown_separately() {
    let known = (1u64 << 33) + 17;
    let mut model = BrowserModel::new(
        Uuid::new_v4(),
        vec![
            entry(0, "ordinary/large.bin", "File", Some(known)),
            entry(1, "ordinary/unknown.bin", "File", None),
        ],
    );
    model.select_all();
    let items = model
        .freeze_drag_selection()
        .expect("size is metadata, not a 512MiB selection veto");
    assert_eq!(
        items
            .iter()
            .find(|item| item.ordinal == Some(0))
            .expect("known")
            .size,
        Some(known)
    );
    assert_eq!(
        items
            .iter()
            .find(|item| item.ordinal == Some(1))
            .expect("unknown")
            .size,
        None
    );
}

#[test]
fn ordinary_directory_drag_freezer_keeps_more_than_32_files() {
    let mut model = BrowserModel::new(
        Uuid::new_v4(),
        (0..33)
            .map(|ordinal| {
                entry(
                    ordinal,
                    &format!("ordinary/file-{ordinal:03}.txt"),
                    "File",
                    Some(8),
                )
            })
            .collect(),
    );
    model.select_all();
    let items = model
        .freeze_drag_selection()
        .expect("33 small files remain eligible");
    assert_eq!(items.len(), 34);
    assert_eq!(items.iter().filter(|item| !item.directory).count(), 33);
}

#[test]
fn zip_backslash_members_share_folder_projection_and_drag_original_ordinals() {
    let snapshot = Uuid::new_v4();
    let original = r"companion\dist\bcprov-jdk18on-1.78.1.jar";
    let mut model = BrowserModel::new(
        snapshot,
        vec![
            entry(0, original, "File", Some(8_324_412)),
            entry(1, r"companion\dist\empty\", "Directory", None),
        ],
    );
    assert_eq!(
        model.folders(),
        vec!["companion", "companion/dist", "companion/dist/empty"]
    );
    assert_eq!(
        model.entries[0].name, original,
        "raw source identity stays unchanged"
    );
    model.enter("companion/dist");
    let row = model
        .visible()
        .iter()
        .find(|row| row.ordinal == Some(0))
        .expect("projected file");
    assert_eq!(row.name, "bcprov-jdk18on-1.78.1.jar");
    model.select(row.id.clone(), false, false);
    let frozen = model.freeze_drag_selection().expect("normal ZIP separator");
    assert_eq!(frozen.len(), 1);
    assert_eq!(frozen[0].name, "bcprov-jdk18on-1.78.1.jar");
    assert_eq!(frozen[0].ordinal, Some(0));
    let tree = model
        .freeze_drag_tree("companion", "source.zip", false)
        .expect("normalized folder selection");
    assert!(tree.iter().any(
        |item| item.name == "companion/dist/bcprov-jdk18on-1.78.1.jar" && item.ordinal == Some(0)
    ));
    for name in [
        r"companion\..\escape.jar",
        r"companion\\empty.jar",
        r"\rooted.jar",
    ] {
        let mut invalid = BrowserModel::new(snapshot, vec![entry(0, name, "File", Some(1))]);
        invalid.selected.insert(RowId::Entry(snapshot, 0));
        assert!(
            invalid.freeze_drag_selection().is_err(),
            "invalid original path: {name}"
        );
    }
}
