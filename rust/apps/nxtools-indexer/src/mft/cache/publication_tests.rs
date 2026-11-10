use std::cell::RefCell;
use std::sync::{Arc, mpsc};
use std::time::Duration;

use nxtools_protocol::{IndexPhase, IndexSearchContext, IndexSearchRequest};
use tempfile::tempdir;

use super::*;
use crate::mft::{MftBase, compact_system_snapshot_with, scan_volume_and_persist_with};
use crate::usn::{JournalBatch, UsnRecord, apply_journal_batch};
use crate::{IndexerService, RuntimeEvent, VolumeDescriptor};

fn entry(reference: u64, parent: u64, path: &str, directory: bool) -> MftEntry {
    MftEntry {
        identity: Some(MftIdentity {
            file_reference: reference,
            parent_reference: parent,
        }),
        path: path.into(),
        name: path.rsplit('\\').next().expect("fixture name").into(),
        is_directory: directory,
        size_bytes: (!directory).then_some(reference),
        modified_at_unix_ms: Some(reference as i64),
    }
}

fn scanned(cursor: i64) -> MftSnapshot {
    MftSnapshot::from_entries(
        r"C:\",
        Some(17),
        Some(cursor),
        vec![
            entry(40, 20, r"C:\Archive\Budget.xlsx", false),
            entry(30, 10, r"C:\Work\Notes.md", false),
            entry(10, 5, r"C:\Work", true),
            entry(80, 10, r"C:\Work\Fourth.txt", false),
            entry(11, 10, r"C:\Work\Budget.xlsx", false),
            entry(60, 10, r"C:\Work\Extra.txt", false),
            entry(20, 5, r"C:\Archive", true),
            entry(70, 10, r"C:\Work\Third.txt", false),
            entry(50, 10, r"C:\Work\Other.txt", false),
        ],
    )
}

fn persist_at(snapshot: &mut MftSnapshot, path: &Path) -> Result<()> {
    persist_snapshot(snapshot, path)?;
    snapshot.set_cache_path(path.to_owned());
    Ok(())
}

fn last_good(directory: &Path) -> MftSnapshot {
    let path = directory.join("volume-433a5c-100.idx");
    let mut snapshot = scanned(100);
    persist_at(&mut snapshot, &path).expect("real last-good base");
    persist_system_delta(&snapshot).expect("real last-good delta");
    let (mapped, journal, cursor, visible) =
        MappedSnapshot::open(&path, r"C:\").expect("last-good mapping");
    MftSnapshot::from_mapped(r"C:\".into(), journal, cursor, visible, mapped, path)
}

fn service_with(snapshot: MftSnapshot) -> IndexerService {
    let mut service =
        IndexerService::from_volume_descriptors([VolumeDescriptor::new(r"C:\", "NTFS")]);
    service.rebuild_with(|_| Ok(snapshot.clone()));
    service
}

fn search_paths(service: &IndexerService, query: &str) -> Vec<String> {
    let mut paths = service
        .search_snapshot(&IndexSearchRequest {
            generation: 1,
            query: query.into(),
            limit: 20,
            context: IndexSearchContext {
                include_files: true,
                include_directories: true,
                ..Default::default()
            },
        })
        .into_iter()
        .map(|item| item.path)
        .collect::<Vec<_>>();
    paths.sort();
    paths
}

fn assert_mapped(snapshot: &MftSnapshot) {
    assert!(
        matches!(&snapshot.base, MftBase::Mapped(_)),
        "durable build must publish mapped, not the full owned snapshot"
    );
    assert!(snapshot.owned_entries().is_none());
    assert!(snapshot.owned_name_masks.is_none());
    assert!(snapshot.reference_index.is_empty());
    assert!(snapshot.children_index.is_empty());
    let references = (0..snapshot.base_len())
        .map(|row| {
            snapshot
                .base_entry(row)
                .expect("mapped row")
                .identity
                .expect("persisted identity")
                .file_reference
        })
        .collect::<Vec<_>>();
    assert!(references.windows(2).all(|pair| pair[0] < pair[1]));
    let children = snapshot.base_children(10);
    assert!(children.contains(&11));
    let auxiliary = snapshot
        .search_data
        .as_ref()
        .expect("small fixture search data");
    let budgets = auxiliary
        .groups
        .iter()
        .find(|group| snapshot.base_name(group.representative) == Some("Budget.xlsx"))
        .expect("duplicate-name group");
    assert_eq!(budgets.rows.len(), 2);
    for &row in &budgets.rows {
        assert_eq!(snapshot.base_name(row as usize), Some("Budget.xlsx"));
    }
    let candidates = auxiliary
        .candidates(&nxtools_finder::FileQuery::parse("\"Budget\""))
        .expect("selective posting candidates");
    assert_eq!(
        candidates,
        vec![
            auxiliary
                .groups
                .iter()
                .position(|group| std::ptr::eq(group, budgets))
                .expect("group belongs to auxiliary data")
        ]
    );
}

fn changed(snapshot: &MftSnapshot, cursor: i64) -> MftSnapshot {
    apply_journal_batch(
        snapshot,
        JournalBatch {
            journal_id: 17,
            lowest_valid_usn: 90,
            next_usn: cursor,
            records: vec![
                UsnRecord::delete(11),
                UsnRecord::upsert(40, 20, "Revised.xlsx", false),
                UsnRecord::upsert(90, 10, "Created.txt", false),
                UsnRecord::metadata(90, 999),
            ],
        },
    )
    .expect("real journal apply against published base")
}

#[test]
fn mapped_publication_scan_reopens_exact_file_and_rebuilds_search_rows() {
    let directory = tempdir().expect("isolated scan publication cache");
    let mut service = service_with(last_good(directory.path()));
    let path = directory.path().join("volume-433a5c-200.idx");
    let owned_lease = RefCell::new(None);
    let auxiliary_lease = RefCell::new(None);
    service.rebuild_with(|mount| {
        scan_volume_and_persist_with(
            mount,
            |_| Ok(scanned(200)),
            |snapshot| {
                let MftBase::Owned(entries) = &snapshot.base else {
                    panic!("scan starts owned")
                };
                *owned_lease.borrow_mut() = Some(Arc::downgrade(entries));
                *auxiliary_lease.borrow_mut() = snapshot.search_data.as_ref().map(Arc::downgrade);
                persist_at(snapshot, &path)
            },
        )
    });
    assert_mapped(&service.snapshots[0]);
    assert_eq!(
        owned_lease
            .into_inner()
            .expect("observed owned rows")
            .strong_count(),
        0
    );
    assert_eq!(
        auxiliary_lease
            .into_inner()
            .expect("observed owned search data")
            .strong_count(),
        0
    );
    assert_eq!(service.snapshots[0].cache_path(), Some(path.as_path()));
    assert_eq!(service.status().phase, IndexPhase::Ready);
    assert_eq!(service.status().indexed_items, 9);
    assert_eq!(service.status().drives[0].journal_id, Some(17));
    assert_eq!(service.status().drives[0].next_usn, Some(200));
    assert!(service.status().last_error.is_none());
    assert!(service.maintenance.is_empty());
    assert_eq!(
        search_paths(&service, "\"Budget\""),
        vec![r"C:\Archive\Budget.xlsx", r"C:\Work\Budget.xlsx"]
    );

    let updated = changed(&service.snapshots[0], 210);
    persist_system_delta(&updated).expect("delta continues on new mapped base");
    service.rebuild_with(|_| Ok(updated.clone()));
    let restored = load_cached_snapshot(directory.path(), "volume-433a5c", r"C:\")
        .expect("restore base and delta")
        .expect("published base");
    assert_eq!(restored.cache_path(), Some(path.as_path()));
    assert_eq!(restored.next_usn, Some(210));
    assert_eq!(restored.visible_len(), 9);
    assert!(restored.visible_entry(11).is_none());
    assert_eq!(
        restored
            .visible_entry(90)
            .expect("replayed new row")
            .modified_at_unix_ms,
        Some(999)
    );
    let restored_service = service_with(restored);
    assert_eq!(
        search_paths(&service, "\"Revised\""),
        search_paths(&restored_service, "\"Revised\"")
    );
    assert_eq!(
        search_paths(&restored_service, "\"Created\""),
        vec![r"C:\Work\Created.txt"]
    );
    assert!(search_paths(&restored_service, "\"Budget\"").is_empty());
}

#[test]
fn mapped_publication_compaction_preserves_overlay_old_lease_and_delta_continuation() {
    let directory = tempdir().expect("isolated compaction publication cache");
    let input = changed(&last_good(directory.path()), 110);
    persist_system_delta(&input).expect("persist input overlay");
    let mut service = service_with(input);
    let old = Arc::clone(&service.snapshots);
    let old_mapping = match &old[0].base {
        MftBase::Mapped(mapped) => Arc::downgrade(mapped),
        _ => panic!("mapped input"),
    };
    let path = directory.path().join("volume-433a5c-200.idx");
    let output = path.clone();
    let (events, receiver) = mpsc::sync_channel(1);
    let worker = crate::spawn_snapshot_compaction_with(old[0].clone(), events, move |snapshot| {
        compact_system_snapshot_with(snapshot, |snapshot| persist_at(snapshot, &output))
    });
    let event = receiver
        .recv_timeout(Duration::from_secs(5))
        .expect("compaction terminal");
    worker.join().expect("compaction worker exit");
    let RuntimeEvent::CompactionCompleted {
        mount_point,
        journal_id,
        next_usn,
        snapshot,
    } = event
    else {
        panic!("successful mapped compaction")
    };
    assert_mapped_compacted(&snapshot);
    assert!(service.apply_compaction(&mount_point, journal_id, next_usn, snapshot));
    service.cleanup_published_caches();
    assert_eq!(
        search_paths(&service, "\"Revised\""),
        vec![r"C:\Archive\Revised.xlsx"]
    );
    assert_eq!(
        old[0].visible_entry(40).expect("leased overlay row").name,
        "Revised.xlsx"
    );
    assert_eq!(
        old_mapping.strong_count(),
        1,
        "only outstanding search lease owns old mapping"
    );
    assert_eq!(service.snapshots[0].cache_path(), Some(path.as_path()));
    assert_eq!(service.snapshots[0].next_usn, Some(110));
    assert_eq!(service.snapshots[0].journal_id, Some(17));
    assert!(service.snapshots[0].overlay.replacements.is_empty());
    assert!(service.snapshots[0].overlay.deleted.is_empty());
    assert_eq!(service.snapshots[0].visible_len(), 9);
    drop(old);
    assert_eq!(old_mapping.strong_count(), 0);
    service.cleanup_published_caches();
    assert!(!directory.path().join("volume-433a5c-100.idx").exists());

    let mut updated = service.snapshots[0].clone();
    updated.next_usn = Some(120);
    persist_system_delta(&updated).expect("persist new-base cursor");
    let restored = load_cached_snapshot(directory.path(), "volume-433a5c", r"C:\")
        .expect("restore compacted base and delta")
        .expect("compacted base");
    assert_eq!(restored.next_usn, Some(120));
    assert_eq!(
        restored
            .visible_entry(90)
            .expect("compacted new row")
            .modified_at_unix_ms,
        Some(999)
    );
    assert_eq!(
        search_paths(&service_with(restored), "\"Revised\""),
        vec![r"C:\Archive\Revised.xlsx"]
    );
}

fn assert_mapped_compacted(snapshot: &MftSnapshot) {
    assert!(
        matches!(&snapshot.base, MftBase::Mapped(_)),
        "compaction must return mapped before service publication"
    );
    assert!(snapshot.owned_entries().is_none());
    assert!(snapshot.owned_name_masks.is_none());
    assert!(snapshot.reference_index.is_empty());
    assert!(snapshot.children_index.is_empty());
}

#[test]
fn mapped_publication_scan_reopen_failure_keeps_last_good_and_reports_failure() {
    let directory = tempdir().expect("isolated failed scan cache");
    let mut service = service_with(last_good(directory.path()));
    let old = Arc::clone(&service.snapshots);
    let old_path = old[0].cache_path().expect("last-good base path").to_owned();
    let path = directory.path().join("volume-433a5c-200.idx");
    service.rebuild_with(|mount| {
        let outcome = scan_volume_and_persist_with(
            mount,
            |_| Ok(scanned(200)),
            |snapshot| {
                persist_at(snapshot, &path)?;
                fs::write(&path, [0_u8; 8])?;
                Ok(())
            },
        );
        assert!(
            old_path.exists(),
            "reopen must not select/clean older candidates"
        );
        assert!(delta_path(&old_path).exists());
        outcome
    });
    assert!(
        service.status().last_error.is_some(),
        "reopen failure is a build failure"
    );
    assert_eq!(service.status().phase, IndexPhase::Ready);
    assert_eq!(service.snapshots[0].cache_path(), Some(old_path.as_path()));
    assert_eq!(service.snapshots[0].next_usn, Some(100));
    assert!(old_path.exists());
    assert!(delta_path(&old_path).exists());
    assert_eq!(search_paths(&service, "\"Budget\"").len(), 2);
    assert!(service.maintenance.is_empty());
}

#[test]
fn mapped_publication_first_scan_reopen_failure_does_not_claim_ready() {
    let directory = tempdir().expect("isolated failed initial scan cache");
    let path = directory.path().join("volume-433a5c-200.idx");
    let mut service =
        IndexerService::from_volume_descriptors([VolumeDescriptor::new(r"C:\", "NTFS")]);
    service.rebuild_with(|mount| {
        scan_volume_and_persist_with(
            mount,
            |_| Ok(scanned(200)),
            |snapshot| {
                persist_at(snapshot, &path)?;
                fs::remove_file(&path)?;
                Ok(())
            },
        )
    });
    assert!(
        service.snapshots.is_empty(),
        "no fallback to the owned build after reopen failure"
    );
    assert_ne!(service.status().phase, IndexPhase::Ready);
    assert!(service.status().last_error.is_some());
    assert!(service.maintenance.is_empty());
}

#[test]
fn mapped_publication_compaction_reopen_failure_uses_failure_handoff_and_keeps_last_good() {
    let directory = tempdir().expect("isolated failed compaction cache");
    let mut service = service_with(last_good(directory.path()));
    let old = Arc::clone(&service.snapshots);
    let old_path = old[0].cache_path().expect("last-good base path").to_owned();
    let path = directory.path().join("volume-433a5c-200.idx");
    let (events, receiver) = mpsc::sync_channel(1);
    let worker = crate::spawn_snapshot_compaction_with(old[0].clone(), events, move |snapshot| {
        let outcome = compact_system_snapshot_with(snapshot, |snapshot| {
            persist_at(snapshot, &path)?;
            fs::remove_file(&path)?;
            Ok(())
        });
        if let Err(error) = &outcome {
            assert_eq!(crate::persistence_native_error(error), Some(2));
        }
        outcome
    });
    let event = receiver
        .recv_timeout(Duration::from_secs(5))
        .expect("compaction failure terminal");
    worker.join().expect("failed compaction worker exit");
    let RuntimeEvent::CompactionFailed {
        mount_point,
        message,
    } = event
    else {
        panic!("exact reopen failure must not publish CompactionCompleted")
    };
    assert!(old_path.exists());
    assert!(delta_path(&old_path).exists());
    service.record_compaction_failure(&mount_point, &message);
    service.cleanup_published_caches();
    assert!(service.status().last_error.is_some());
    assert!(service.compaction_retry_at(r"C:\").is_some());
    assert!(Arc::ptr_eq(&old, &service.snapshots));
    assert_eq!(service.snapshots[0].next_usn, Some(100));
    assert_eq!(search_paths(&service, "\"Budget\"").len(), 2);
    assert!(old_path.exists());
    assert!(delta_path(&old_path).exists());
}

#[test]
fn mapped_publication_stale_compaction_does_not_replace_advanced_snapshot() {
    let directory = tempdir().expect("isolated stale compaction cache");
    let mut service = service_with(last_good(directory.path()));
    let path = directory.path().join("volume-433a5c-200.idx");
    let compacted = compact_system_snapshot_with(&service.snapshots[0], |snapshot| {
        persist_at(snapshot, &path)
    })
    .expect("persist and reopen stale compaction");
    assert_mapped(&compacted);
    assert!(
        directory.path().join("volume-433a5c-100.idx").exists(),
        "reopen does not retire last-good before publication"
    );
    let updated = changed(&service.snapshots[0], 110);
    service.rebuild_with(|_| Ok(updated.clone()));
    assert!(!service.apply_compaction(r"C:\", Some(17), Some(100), compacted));
    assert_eq!(service.snapshots[0].next_usn, Some(110));
    assert_eq!(
        search_paths(&service, "\"Revised\""),
        vec![r"C:\Archive\Revised.xlsx"]
    );
}
