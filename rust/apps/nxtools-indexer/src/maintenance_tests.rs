use super::*;

fn maintenance(service: &IndexerService, drive: usize) -> serde_json::Value {
    serde_json::to_value(&service.status.drives[drive]).expect("drive projection")["maintenance"]
        .clone()
}

fn empty_snapshot(mount: &str) -> MftSnapshot {
    MftSnapshot::from_entries(mount, Some(17), Some(100), Vec::new())
}

#[test]
fn u4_initial_build_ready_and_scoped_rebuild_status() {
    let mut service = IndexerService::from_volume_descriptors([
        VolumeDescriptor::new(r"C:\", "NTFS"),
        VolumeDescriptor::new(r"D:\", "NTFS"),
    ]);
    service.begin_snapshot_maintenance();
    for drive in 0..2 {
        assert_eq!(
            maintenance(&service, drive),
            serde_json::json!({"state": "initial_building"}),
            "initial build must have a per-volume maintenance projection"
        );
    }
    service.rebuild_with(|mount| Ok(empty_snapshot(mount)));
    for drive in 0..2 {
        assert_eq!(
            maintenance(&service, drive),
            serde_json::json!({"state": "ready"}),
            "a successfully indexed empty volume is ready"
        );
    }
    service.poll_journals_with_persistence(
        &|snapshot| {
            if snapshot.mount_point == r"D:\" {
                return Ok(usn::JournalPoll::NoChange);
            }
            let error = usn::apply_journal_batch(
                snapshot,
                usn::JournalBatch {
                    journal_id: 18,
                    lowest_valid_usn: 90,
                    next_usn: 110,
                    records: Vec::new(),
                },
            )
            .expect_err("changed journal identity");
            let usn::JournalApplyError::RebuildRequired(reason) = error;
            Ok(usn::JournalPoll::RebuildRequired(reason))
        },
        &|_| Ok(()),
    );
    assert_eq!(
        maintenance(&service, 0),
        serde_json::json!({"state": "rebuilding", "reason": "cursor_invalid"})
    );
    assert_eq!(
        maintenance(&service, 1),
        serde_json::json!({"state": "ready"})
    );
    assert!(
        service.status.last_error.is_none(),
        "maintenance is not a failure"
    );
    assert_eq!(
        service.snapshots.len(),
        2,
        "last-good ownership survives maintenance"
    );
    let outcome = build_snapshots(service.build_drives_for_mounts(&[r"C:\".into()]), |mount| {
        Ok(empty_snapshot(mount))
    });
    service.apply_build(outcome);
    assert_eq!(
        maintenance(&service, 0),
        serde_json::json!({"state": "ready"})
    );
}

#[test]
fn u4_failed_rebuild_ends_maintenance_but_retains_real_error_and_last_good() {
    let mut service =
        IndexerService::from_volume_descriptors([VolumeDescriptor::new(r"C:\", "NTFS")]);
    service.rebuild_with(|mount| Ok(empty_snapshot(mount)));
    service.begin_snapshot_maintenance();
    assert_eq!(
        maintenance(&service, 0),
        serde_json::json!({"state": "rebuilding", "reason": "requested"})
    );
    service.record_build_failure(vec![r"C:\".into()], "synthetic worker failure".into());
    assert_eq!(
        maintenance(&service, 0),
        serde_json::json!({"state": "ready"})
    );
    assert!(
        service
            .status
            .last_error
            .as_deref()
            .is_some_and(|error| error.contains("worker failed"))
    );
    assert_eq!(service.snapshots.len(), 1);
}

#[test]
fn u4_initial_failure_and_queued_cancellation_leave_no_busy_projection() {
    let mut service = IndexerService::from_volume_descriptors([
        VolumeDescriptor::new(r"C:\", "NTFS"),
        VolumeDescriptor::new(r"D:\", "NTFS"),
    ]);
    let drives = service.build_drives();
    service.queue_snapshot_maintenance(&drives, IndexRebuildReason::Requested, None);
    service.queue_snapshot_maintenance(&drives, IndexRebuildReason::Requested, None);
    assert_eq!(
        service.maintenance.len(),
        2,
        "coalesced work has one operation per volume"
    );
    service.start_snapshot_maintenance(&drives[..1]);
    service.record_build_failure(vec![r"C:\".into()], "synthetic failure".into());
    assert!(
        maintenance(&service, 0).is_null(),
        "failed initial build is not ready or busy"
    );
    let settings = IndexSettings {
        indexed_volumes: vec![r"C:\".into()],
        ..IndexSettings::default()
    };
    service.apply_settings_with_persistence(settings, &|_| Ok(()));
    assert!(
        maintenance(&service, 1).is_null(),
        "disabled queued volume stops being busy"
    );
    assert!(service.maintenance.is_empty());
    assert!(
        service.status.last_error.is_some(),
        "actual initial failure remains visible"
    );
}

#[test]
fn u4_rediscovery_retains_ready_and_inflight_volume_maintenance() {
    let mut service = IndexerService::from_volume_descriptors([
        VolumeDescriptor::new(r"C:\", "NTFS"),
        VolumeDescriptor::new(r"D:\", "NTFS"),
    ]);
    service.rebuild_with(|mount| Ok(empty_snapshot(mount)));
    let drives = service.build_drives_for_mounts(&[r"C:\".into()]);
    service.queue_snapshot_maintenance(&drives, IndexRebuildReason::JournalWrapped, Some(1181));
    service.start_snapshot_maintenance(&drives);
    service.rediscover_from_volume_descriptors([
        VolumeDescriptor::new(r"C:\", "NTFS"),
        VolumeDescriptor::new(r"D:\", "NTFS"),
    ]);
    assert_eq!(
        maintenance(&service, 0),
        serde_json::json!({"state": "rebuilding", "reason": "journal_wrapped"})
    );
    assert_eq!(
        maintenance(&service, 1),
        serde_json::json!({"state": "ready"})
    );
    assert_eq!(service.maintenance.len(), 1);
    assert!(service.status.last_error.is_none());
}
