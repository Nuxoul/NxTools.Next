use super::*;
use crate::mft::{
    RawMftRecord, SnapshotConstructor, allocation_probe, entries_from_records,
    enumerate_initial_snapshot, scan_volume_and_persist_with,
    snapshot_for_persistence_from_records, snapshot_from_records,
};

fn records() -> Vec<RawMftRecord> {
    let mut rows = Vec::new();
    for directory in 0..64 {
        rows.push(RawMftRecord {
            last_usn: 0,
            file_reference: 100 + directory,
            parent_reference: 5,
            name: format!("folder-{directory:02}"),
            is_directory: true,
            modified_at_unix_ms: Some(1000 + directory as i64),
        });
        for item in 0..64 {
            // Reference order deliberately differs from path and group order.
            let reference = 1000 + item * 64 + directory;
            rows.push(RawMftRecord {
                last_usn: 0,
                file_reference: reference,
                parent_reference: 100 + directory,
                name: format!("document-{:03}.txt", (item + directory * 4) % 256),
                is_directory: false,
                modified_at_unix_ms: Some(reference as i64),
            });
        }
    }
    rows
}

fn pipeline(
    path: &Path,
    rows: Vec<RawMftRecord>,
    select: impl Fn(SnapshotConstructor) -> SnapshotConstructor,
) -> MftSnapshot {
    let rows = std::cell::RefCell::new(Some(rows));
    scan_volume_and_persist_with(
        r"C:\",
        |mount, construct| {
            enumerate_initial_snapshot(
                mount,
                17,
                1234,
                |high_usn| {
                    assert_eq!(high_usn, i64::MAX);
                    Ok(rows
                        .borrow_mut()
                        .take()
                        .expect("enumerate fixture rows once"))
                },
                select(construct),
            )
        },
        |snapshot| {
            persist_snapshot(snapshot, path)?;
            snapshot.set_cache_path(path.to_owned());
            Ok(())
        },
    )
    .expect("durable exact-file publication")
}

#[test]
fn initial_persistence_skips_transient_runtime_auxiliary_allocations() {
    let directory = tempfile::tempdir().expect("isolated allocation fixture cache");
    let full_path = directory.path().join("full.idx");
    let lean_path = directory.path().join("lean.idx");
    let full_rows = records();
    let lean_rows = records();
    let rows = full_rows.len();
    let (full, full_allocations) =
        allocation_probe::measure(|| pipeline(&full_path, full_rows, |_| snapshot_from_records));
    let (lean, lean_allocations) =
        allocation_probe::measure(|| pipeline(&lean_path, lean_rows, |construct| construct));
    assert!(matches!(lean.base, crate::mft::MftBase::Mapped(_)));
    assert_eq!(lean.cache_path(), Some(lean_path.as_path()));
    assert_eq!(lean.journal_id, Some(17));
    assert_eq!(lean.next_usn, Some(1234));
    assert_eq!(lean.base_len(), rows);
    for row in 0..rows {
        let left = full.base_entry(row).expect("full mapped row").to_owned();
        let right = lean.base_entry(row).expect("lean mapped row").to_owned();
        assert_eq!(left, right);
        let identity = left.identity.expect("persisted fixture identity");
        assert_eq!(lean.visible_entry(identity.file_reference), Some(left));
    }
    for parent in 100..164 {
        assert_eq!(full.base_children(parent), lean.base_children(parent));
        assert_eq!(lean.base_children(parent).len(), 64);
    }
    let query = nxtools_finder::FileQuery::parse("\"document-013\"");
    let full_data = full.search_data.as_ref().expect("full mapped search data");
    let lean_data = lean.search_data.as_ref().expect("lean mapped search data");
    assert_eq!(full_data.candidates(&query), lean_data.candidates(&query));
    assert!(
        lean_data
            .candidates(&query)
            .is_some_and(|groups| !groups.is_empty())
    );
    println!(
        "INITIAL_PERSIST_ALLOCATION rows={rows} full_requested_bytes={} lean_requested_bytes={} full_calls={} lean_calls={}",
        full_allocations.requested_bytes,
        lean_allocations.requested_bytes,
        full_allocations.calls,
        lean_allocations.calls
    );
    assert!(
        full_allocations.requested_bytes
            >= lean_allocations.requested_bytes + rows * size_of::<u64>(),
        "production persistence must avoid at least the transient per-row name masks"
    );
}

#[test]
fn initial_path_construction_moves_names_and_caches_paths_without_cloning() {
    let rows = records();
    let count = rows.len();
    let mut expected = rows
        .iter()
        .map(|record| MftEntry {
            identity: Some(MftIdentity {
                file_reference: record.file_reference,
                parent_reference: record.parent_reference,
            }),
            path: if record.is_directory {
                format!("C:\\{}", record.name)
            } else {
                format!(
                    "C:\\folder-{:02}\\{}",
                    record.parent_reference - 100,
                    record.name
                )
            },
            name: record.name.clone(),
            is_directory: record.is_directory,
            size_bytes: None,
            modified_at_unix_ms: record.modified_at_unix_ms,
        })
        .collect::<Vec<_>>();
    expected.sort_by(|left, right| left.path.cmp(&right.path));
    let (entries, allocations) = allocation_probe::measure(|| entries_from_records(r"C:\", rows));
    assert_eq!(entries, expected);
    println!(
        "INITIAL_PATH_ALLOCATION rows={count} requested_bytes={} calls={}",
        allocations.requested_bytes, allocations.calls
    );
    assert!(
        allocations.calls <= count * 2 + 64,
        "construct each full path once and move the original name; reuse traversal storage"
    );
}

#[test]
fn initial_path_cache_preserves_roots_full_ids_orphans_cycles_and_last_duplicate() {
    let root = (4_u64 << 48) | 5;
    let folder = (7_u64 << 48) | 10;
    let record = |reference, parent, name: &str| RawMftRecord {
        last_usn: 100,
        file_reference: reference,
        parent_reference: parent,
        name: name.into(),
        is_directory: true,
        modified_at_unix_ms: Some(reference as i64),
    };
    let entries = entries_from_records(
        r"C:\",
        vec![
            record(root, root, "."),
            record(folder, root, "obsolete"),
            record(11, folder, "报告.txt"),
            record(folder, root, "Work"),
            record(12, 999, "orphan"),
            record(13, 14, "cycle-a"),
            record(14, 13, "cycle-b"),
            record(15, 13, "cycle-child"),
            record(16, 16, "self-root"),
            record(17, 16, "child"),
            record(18, (8_u64 << 48) | 10, "reused-parent"),
        ],
    );
    assert_eq!(
        entries
            .iter()
            .map(|entry| entry.path.as_str())
            .collect::<Vec<_>>(),
        [r"C:\", r"C:\Work", r"C:\Work\报告.txt", r"C:\child"]
    );
    assert_eq!(
        entries[1].identity.expect("folder identity").file_reference,
        folder
    );
    assert_eq!(entries[2].modified_at_unix_ms, Some(11));
}

#[test]
fn initial_persistence_leaves_row_sorting_to_the_writer() {
    let sorted_rows = records();
    let persistence_rows = records();
    let count = sorted_rows.len();
    let (expected, sorted_allocations) =
        allocation_probe::measure(|| entries_from_records(r"C:\", sorted_rows));
    let (snapshot, persistence_allocations) = allocation_probe::measure(|| {
        snapshot_for_persistence_from_records(r"C:\", Some(17), Some(1234), persistence_rows)
    });
    let mut actual = snapshot
        .owned_entries()
        .expect("unpublished owned entries")
        .to_vec();
    actual.sort_by(|left, right| left.path.cmp(&right.path));
    assert_eq!(actual, expected);
    assert!(snapshot.search_data.is_none());
    assert_eq!(snapshot.next_usn, Some(1234));
    println!(
        "INITIAL_SORT_ALLOCATION rows={count} sorted_requested_bytes={} persistence_requested_bytes={}",
        sorted_allocations.requested_bytes, persistence_allocations.requested_bytes
    );
    assert!(
        sorted_allocations.requested_bytes
            >= persistence_allocations.requested_bytes + count * size_of::<MftEntry>() / 2,
        "writer-only construction must not allocate a temporary path-sort buffer"
    );
}
