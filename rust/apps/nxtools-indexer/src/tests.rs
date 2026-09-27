//! Tests for the indexer service. Extracted from lib.rs so the runtime
//! code stays readable.

use super::*;
use std::io::{BufReader, Cursor, Read};
use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};
use std::sync::{Arc, Mutex, mpsc};
use std::time::{Duration, Instant};

use nxtools_protocol::{
    Envelope, Hello, IndexPhase, IndexSearchContext, IndexSearchRequest, IndexSettings,
    IndexerCommand, IndexerEvent, Message, Ping,
};

#[test]
fn file_dialog_preference_boosts_only_direct_children() {
    assert_eq!(
        preferred_directory_score(r"C:\Work\AyuGram.exe", Some(r"c:\work")),
        PREFERRED_DIRECTORY_SCORE_BONUS
    );
    assert_eq!(
        preferred_directory_score(r"C:\Work\Dependencies\grammar.txt", Some(r"C:\Work")),
        0
    );
    assert_eq!(
        preferred_directory_score(r"D:\Other\AyuGram.exe", Some(r"C:\Work")),
        0
    );
}

struct ChannelReader {
    receiver: mpsc::Receiver<Vec<u8>>,
    current: Cursor<Vec<u8>>,
}

impl ChannelReader {
    fn new(receiver: mpsc::Receiver<Vec<u8>>) -> Self {
        Self {
            receiver,
            current: Cursor::new(Vec::new()),
        }
    }
}

impl Read for ChannelReader {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        loop {
            let read = self.current.read(buffer)?;
            if read > 0 {
                return Ok(read);
            }
            let Ok(next) = self.receiver.recv() else {
                return Ok(0);
            };
            self.current = Cursor::new(next);
        }
    }
}

#[derive(Clone, Default)]
struct SharedWriter(Arc<Mutex<Vec<u8>>>);

impl Write for SharedWriter {
    fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
        self.0
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .extend_from_slice(buffer);
        Ok(buffer.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn test_entry(
    path: impl Into<String>,
    name: impl Into<String>,
    is_directory: bool,
) -> mft::MftEntry {
    mft::MftEntry {
        identity: None,
        path: path.into(),
        name: name.into(),
        is_directory,
        size_bytes: None,
        modified_at_unix_ms: None,
    }
}

fn test_snapshot(
    mount_point: &str,
    journal_id: Option<u64>,
    next_usn: Option<i64>,
    entries: Vec<mft::MftEntry>,
) -> MftSnapshot {
    MftSnapshot::from_entries(mount_point, journal_id, next_usn, entries)
}

fn wait_for_frame(writer: &SharedWriter, predicate: impl Fn(&Envelope) -> bool) -> Envelope {
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        let output = writer
            .0
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone();
        if let Some(frame) = String::from_utf8_lossy(&output)
            .lines()
            .filter_map(|line| Envelope::decode_line(line).ok())
            .find(|frame| predicate(frame))
        {
            return frame;
        }
        assert!(
            Instant::now() < deadline,
            "timed out waiting for indexer protocol frame; output={}",
            String::from_utf8_lossy(&output)
        );
        thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn discovery_publishes_only_real_ntfs_volumes() {
    let service = IndexerService::from_volume_descriptors([
        VolumeDescriptor::new(r"C:\", "NTFS"),
        VolumeDescriptor::new(r"D:\", "ReFS"),
        VolumeDescriptor::new(r"Z:\", "NTFS"),
    ]);

    let status = service.status();
    assert_eq!(status.phase, IndexPhase::VolumesReady);
    assert_eq!(status.indexed_items, 0);
    assert_eq!(
        status
            .drives
            .iter()
            .map(|drive| drive.mount_point.as_str())
            .collect::<Vec<_>>(),
        vec![r"C:\", r"Z:\"]
    );
    assert!(
        status
            .drives
            .iter()
            .all(|drive| drive.file_system == "NTFS")
    );
}

#[test]
fn a_new_generation_makes_older_search_requests_stale() {
    let mut service = IndexerService::from_volume_descriptors([]);
    let search = |generation| {
        Envelope::new(Message::IndexerCommand(IndexerCommand::Search(
            IndexSearchRequest {
                generation,
                query: "report".into(),
                limit: 20,
                context: IndexSearchContext::default(),
            },
        )))
    };

    let first = service.handle(search(7));
    let newer = service.handle(search(8));
    let stale = service.handle(search(7));

    assert!(matches!(
        first.as_slice(),
        [Envelope {
            message: Message::IndexerEvent(IndexerEvent::SearchBatch(batch)),
            ..
        }] if batch.generation == 7 && batch.complete
    ));
    assert!(matches!(
        newer.as_slice(),
        [Envelope {
            message: Message::IndexerEvent(IndexerEvent::SearchBatch(batch)),
            ..
        }] if batch.generation == 8 && batch.complete
    ));
    assert!(stale.is_empty());
}

#[test]
fn a_reconnected_desktop_can_restart_its_query_generation() {
    let mut service = IndexerService::from_volume_descriptors([]);
    service.status.phase = IndexPhase::Ready;
    fn run_session(service: &mut IndexerService, generation: u64) {
        let hello = Envelope::new(Message::Hello(Hello {
            client_name: "reconnect_test".into(),
            client_version: "0.1.0".into(),
            capabilities: vec!["file_search".into()],
        }));
        let search = Envelope::new(Message::IndexerCommand(IndexerCommand::Search(
            IndexSearchRequest {
                generation,
                query: "report".into(),
                limit: 20,
                context: IndexSearchContext::default(),
            },
        )));
        let (input, receiver) = mpsc::sync_channel(4);
        let output = SharedWriter::default();
        let observed = output.clone();
        let sender = thread::spawn(move || {
            input
                .send(hello.encode_line().expect("hello frame").into_bytes())
                .expect("queue hello");
            input
                .send(search.encode_line().expect("search frame").into_bytes())
                .expect("queue search");
            wait_for_frame(&observed, |frame| {
                matches!(
                    &frame.message,
                    Message::IndexerEvent(IndexerEvent::SearchBatch(batch))
                        if batch.generation == generation && batch.complete
                )
            });
        });
        serve_session(
            BufReader::new(ChannelReader::new(receiver)),
            output,
            service,
        )
        .expect("desktop session");
        sender.join().expect("join desktop writer");
    }

    run_session(&mut service, 99);
    run_session(&mut service, 1);
}

#[test]
fn refreshing_volumes_preserves_the_search_generation_high_water_mark() {
    let mut service = IndexerService::from_volume_descriptors([]);
    let search = |generation| {
        Envelope::new(Message::IndexerCommand(IndexerCommand::Search(
            IndexSearchRequest {
                generation,
                query: "report".into(),
                limit: 20,
                context: IndexSearchContext::default(),
            },
        )))
    };

    assert!(!service.handle(search(100)).is_empty());
    service.rediscover();

    assert!(service.handle(search(99)).is_empty());
}

#[test]
fn rediscovering_volumes_keeps_the_last_good_snapshot_searchable() {
    let mut service =
        IndexerService::from_volume_descriptors([VolumeDescriptor::new(r"C:\", "NTFS")]);
    service.rebuild_with(|mount_point| {
        Ok(test_snapshot(
            mount_point,
            Some(17),
            Some(42),
            vec![test_entry(
                r"C:\Work\retainedsnapshot.txt",
                "retainedsnapshot.txt",
                false,
            )],
        ))
    });
    let search_request = IndexSearchRequest {
        generation: 1,
        query: "retainedsnapshot".into(),
        limit: 20,
        context: IndexSearchContext {
            directory: None,
            preferred_directory: None,
            include_files: true,
            include_directories: true,
        },
    };
    assert_eq!(service.search_snapshot(&search_request).len(), 1);

    service.rediscover();

    assert_eq!(service.status.indexed_items, 1);
    assert_eq!(service.snapshots.len(), 1);
    assert_eq!(service.search_snapshot(&search_request).len(), 1);
}

#[test]
fn a_completed_mft_snapshot_is_searchable_through_the_process_protocol() {
    let mut service =
        IndexerService::from_volume_descriptors([VolumeDescriptor::new(r"C:\", "NTFS")]);
    service.rebuild_with(|mount_point| {
        let mut entry = test_entry(r"C:\Work\Budget 2026.xlsx", "Budget 2026.xlsx", false);
        entry.size_bytes = Some(1024);
        entry.modified_at_unix_ms = Some(123);
        Ok(test_snapshot(mount_point, Some(17), Some(42), vec![entry]))
    });
    assert_eq!(service.status().phase, IndexPhase::Ready);
    assert_eq!(service.status().indexed_items, 1);
    assert_eq!(service.status().drives[0].indexed_items, 1);
    assert_eq!(service.status().drives[0].journal_id, Some(17));
    assert_eq!(service.status().drives[0].next_usn, Some(42));
    let request = Envelope::new(Message::IndexerCommand(IndexerCommand::Search(
        IndexSearchRequest {
            generation: 1,
            query: "budget".into(),
            limit: 20,
            context: IndexSearchContext {
                directory: None,
                preferred_directory: None,
                include_files: true,
                include_directories: true,
            },
        },
    )));

    let responses = service.handle(request);

    assert!(matches!(
        responses.as_slice(),
        [Envelope {
            message: Message::IndexerEvent(IndexerEvent::SearchBatch(batch)),
            ..
        }] if batch.complete
            && batch.items.len() == 1
            && batch.items[0].path == r"C:\Work\Budget 2026.xlsx"
    ));
}

#[test]
fn process_search_uses_the_shared_query_language() {
    let mut service =
        IndexerService::from_volume_descriptors([VolumeDescriptor::new(r"D:\", "NTFS")]);
    service.rebuild_with(|mount_point| {
        Ok(test_snapshot(
            mount_point,
            Some(17),
            Some(42),
            vec![
                test_entry(r"D:\Work\Budget 2026.xlsx", "Budget 2026.xlsx", false),
                test_entry(r"D:\Work\Budget 2025.xlsx", "Budget 2025.xlsx", false),
            ],
        ))
    });
    let request = Envelope::new(Message::IndexerCommand(IndexerCommand::Search(
        IndexSearchRequest {
            generation: 1,
            query: "d: ^Budget !2025".into(),
            limit: 20,
            context: IndexSearchContext {
                directory: None,
                preferred_directory: None,
                include_files: true,
                include_directories: true,
            },
        },
    )));

    let responses = service.handle(request);

    assert!(matches!(
        responses.as_slice(),
        [Envelope {
            message: Message::IndexerEvent(IndexerEvent::SearchBatch(batch)),
            ..
        }] if batch.complete
            && batch.items.len() == 1
            && batch.items[0].path == r"D:\Work\Budget 2026.xlsx"
    ));
}

#[test]
fn worker_results_are_partitioned_into_ordered_complete_batches() {
    let items = (0..45)
        .map(|index| IndexSearchItem {
            path: format!(r"C:\{index}.txt"),
            name: format!("{index}.txt"),
            is_directory: false,
            size_bytes: None,
            modified_at_unix_ms: None,
            score: 100 - index,
        })
        .collect();

    let batches = partition_search_batches(7, items, 123);

    assert_eq!(batches.len(), 3);
    assert_eq!(
        batches.iter().map(|batch| batch.items.len()).sum::<usize>(),
        45
    );
    assert!(!batches[0].complete);
    assert!(!batches[1].complete);
    assert!(batches[2].complete);
    assert!(batches.iter().all(|batch| batch.generation == 7));
}

#[test]
fn a_pending_explicit_rebuild_absorbs_later_missing_volume_work() {
    let explicit = PendingBuild {
        generation: 2,
        drives: vec![IndexDriveStatus {
            mount_point: r"C:\".into(),
            file_system: "NTFS".into(),
            indexed_items: 0,
            journal_id: None,
            next_usn: None,
        }],
        missing_only: false,
    };
    let settings_update = PendingBuild {
        generation: 3,
        drives: vec![IndexDriveStatus {
            mount_point: r"D:\".into(),
            file_system: "NTFS".into(),
            indexed_items: 0,
            journal_id: None,
            next_usn: None,
        }],
        missing_only: true,
    };

    let merged = merge_pending_build(explicit, settings_update);

    assert_eq!(merged.generation, 3);
    assert!(!merged.missing_only);
    assert_eq!(
        merged
            .drives
            .iter()
            .map(|drive| drive.mount_point.as_str())
            .collect::<Vec<_>>(),
        vec![r"C:\", r"D:\"]
    );
}

#[test]
fn index_settings_limit_volumes_and_filter_excluded_paths_at_the_service_boundary() {
    let mut service = IndexerService::from_volume_descriptors([
        VolumeDescriptor::new(r"C:\", "NTFS"),
        VolumeDescriptor::new(r"D:\", "NTFS"),
    ]);
    service.rebuild_with(|mount_point| {
        Ok(test_snapshot(
            mount_point,
            Some(17),
            Some(42),
            vec![
                test_entry(
                    format!(r"{mount_point}Work\report.txt"),
                    "report.txt",
                    false,
                ),
                test_entry(
                    format!(r"{mount_point}Build\report.txt"),
                    "report.txt",
                    false,
                ),
            ],
        ))
    });
    service.handle(Envelope::new(Message::IndexerCommand(
        IndexerCommand::UpdateSettings(IndexSettings {
            indexed_volumes: vec![r"D:\".into()],
            excluded_paths: vec![r"D:\Build".into()],
        }),
    )));
    assert!(!service.enabled_volume_missing_snapshot());

    let results = service.search_snapshot(&IndexSearchRequest {
        generation: 1,
        query: "report".into(),
        limit: 20,
        context: IndexSearchContext {
            directory: None,
            preferred_directory: None,
            include_files: true,
            include_directories: true,
        },
    });

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].path, r"D:\Work\report.txt");

    let bypassed = service.search_snapshot(&IndexSearchRequest {
        generation: 2,
        query: "*report".into(),
        limit: 20,
        context: IndexSearchContext {
            directory: None,
            preferred_directory: None,
            include_files: true,
            include_directories: true,
        },
    });
    assert_eq!(bypassed.len(), 2);
    assert!(
        bypassed
            .iter()
            .any(|item| item.path == r"D:\Build\report.txt")
    );
}

#[test]
fn name_search_falls_back_when_filtered_headroom_would_underfill_the_page() {
    let mut service =
        IndexerService::from_volume_descriptors([VolumeDescriptor::new(r"C:\", "NTFS")]);
    service.rebuild_with(|mount_point| {
        let mut entries = (0..8)
            .map(|index| {
                test_entry(
                    format!(r"{mount_point}Build\{index}\report.txt"),
                    "report.txt",
                    false,
                )
            })
            .collect::<Vec<_>>();
        entries.extend((0..2).map(|index| {
            test_entry(
                format!(r"{mount_point}Work\{index}\report.txt"),
                "report.txt",
                false,
            )
        }));
        Ok(test_snapshot(mount_point, Some(17), Some(42), entries))
    });
    service.settings.excluded_paths = vec![r"C:\Build".into()];

    let results = service.search_snapshot(&IndexSearchRequest {
        generation: 1,
        query: "report".into(),
        limit: 2,
        context: IndexSearchContext {
            directory: None,
            preferred_directory: None,
            include_files: true,
            include_directories: true,
        },
    });

    assert_eq!(results.len(), 2);
    assert!(results.iter().all(|item| item.path.contains(r"C:\Work\")));
}

#[test]
fn corrupted_persisted_settings_fail_closed_until_the_desktop_resynchronizes() {
    let settings = fail_closed_index_settings();

    assert!(!index_settings_volume_enabled(&settings, r"C:\"));
    assert!(!index_settings_volume_enabled(&settings, r"D:\"));
}

#[test]
fn rebuilding_a_selected_volume_does_not_forget_other_discovered_snapshots() {
    let mut service = IndexerService::from_volume_descriptors([
        VolumeDescriptor::new(r"C:\", "NTFS"),
        VolumeDescriptor::new(r"D:\", "NTFS"),
    ]);
    service.rebuild_with(|mount_point| {
        Ok(test_snapshot(
            mount_point,
            Some(17),
            Some(42),
            vec![test_entry(
                format!(r"{mount_point}report.txt"),
                "report.txt",
                false,
            )],
        ))
    });
    service.apply_settings(IndexSettings {
        indexed_volumes: vec![r"D:\".into()],
        excluded_paths: Vec::new(),
    });

    let selected_build = build_snapshots(service.build_drives(), |mount_point| {
        Ok(test_snapshot(
            mount_point,
            Some(17),
            Some(43),
            vec![test_entry(
                format!(r"{mount_point}updated.txt"),
                "updated.txt",
                false,
            )],
        ))
    });
    service.apply_build(selected_build);
    service.apply_settings(IndexSettings {
        indexed_volumes: vec![r"C:\".into(), r"D:\".into()],
        excluded_paths: Vec::new(),
    });

    assert_eq!(service.status.drives.len(), 2);
    assert_eq!(service.snapshots.len(), 2);
    assert!(!service.enabled_volume_missing_snapshot());
}

#[test]
fn a_successful_volume_rebuild_does_not_clear_another_volumes_error() {
    let mut service = IndexerService::from_volume_descriptors([
        VolumeDescriptor::new(r"C:\", "NTFS"),
        VolumeDescriptor::new(r"D:\", "NTFS"),
    ]);
    let failed = build_snapshots(service.build_drives(), |mount_point| {
        bail!("{mount_point} is unavailable")
    });
    service.apply_build(failed);
    assert!(
        service
            .status
            .last_error
            .as_deref()
            .is_some_and(|error| error.contains(r"C:\") && error.contains(r"D:\"))
    );

    let repaired = build_snapshots(
        service.build_drives_for_mounts(&[r"C:\".into()]),
        |mount_point| {
            Ok(test_snapshot(
                mount_point,
                Some(17),
                Some(42),
                vec![test_entry(r"C:\Work", "Work", true)],
            ))
        },
    );
    service.apply_build(repaired);

    let error = service
        .status
        .last_error
        .as_deref()
        .expect("D: error remains");
    assert!(!error.contains(r"C:\"));
    assert!(error.contains(r"D:\"));
}

#[test]
fn a_published_usn_overlay_is_visible_to_subsequent_searches() {
    let mut service =
        IndexerService::from_volume_descriptors([VolumeDescriptor::new(r"C:\", "NTFS")]);
    service.rebuild_with(|mount_point| {
        let mut work = test_entry(r"C:\Work", "Work", true);
        work.identity = Some(mft::MftIdentity {
            file_reference: 10,
            parent_reference: 5,
        });
        let mut old = test_entry(r"C:\Work\old.txt", "old.txt", false);
        old.identity = Some(mft::MftIdentity {
            file_reference: 11,
            parent_reference: 10,
        });
        Ok(test_snapshot(
            mount_point,
            Some(17),
            Some(100),
            vec![work, old],
        ))
    });

    let maintenance = service.poll_journals(&|snapshot| {
        let updated = usn::apply_journal_batch(
            snapshot,
            usn::JournalBatch {
                journal_id: 17,
                lowest_valid_usn: 90,
                next_usn: 110,
                records: vec![
                    usn::UsnRecord::delete(11),
                    usn::UsnRecord::upsert(12, 10, "new.txt", false),
                ],
            },
        )
        .map_err(anyhow::Error::from)?;
        Ok(usn::JournalPoll::Updated(updated))
    });

    assert!(matches!(maintenance, JournalMaintenance::Updated));
    assert_eq!(service.status.drives[0].next_usn, Some(110));
    assert!(
        service
            .search_snapshot(&IndexSearchRequest {
                generation: 1,
                query: "old".into(),
                limit: 20,
                context: IndexSearchContext {
                    directory: None,
                    preferred_directory: None,
                    include_files: true,
                    include_directories: true,
                },
            })
            .is_empty()
    );
    let results = service.search_snapshot(&IndexSearchRequest {
        generation: 2,
        query: "new".into(),
        limit: 20,
        context: IndexSearchContext {
            directory: None,
            preferred_directory: None,
            include_files: true,
            include_directories: true,
        },
    });
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].path, r"C:\Work\new.txt");
}

#[test]
fn an_oversized_overlay_is_published_before_background_compaction() {
    let mut service =
        IndexerService::from_volume_descriptors([VolumeDescriptor::new(r"C:\", "NTFS")]);
    let mut snapshot = test_snapshot(
        r"C:\",
        Some(17),
        Some(100),
        vec![test_entry(r"C:\base.txt", "base.txt", false)],
    );
    for offset in 0..=4_096_u64 {
        snapshot.overlay.replacements.insert(
            10_000 + offset,
            test_entry(
                format!(r"C:\overlay-{offset}.txt"),
                format!("overlay-{offset}.txt"),
                false,
            ),
        );
    }
    service.status.phase = IndexPhase::Ready;
    service.snapshots = Arc::new(vec![snapshot.clone()]);

    let maintenance =
        service.poll_journals(&move |_| Ok(usn::JournalPoll::Updated(snapshot.clone())));

    assert!(matches!(
        maintenance,
        JournalMaintenance::CompactionRequired { .. }
    ));
    let results = service.search_snapshot(&IndexSearchRequest {
        generation: 1,
        query: "overlay-4096".into(),
        limit: 20,
        context: IndexSearchContext {
            directory: None,
            preferred_directory: None,
            include_files: true,
            include_directories: true,
        },
    });
    assert_eq!(results.len(), 1);
}

#[test]
fn one_volume_poll_failure_does_not_starve_a_healthy_volume() {
    let mut service = IndexerService::from_volume_descriptors([
        VolumeDescriptor::new(r"C:\", "NTFS"),
        VolumeDescriptor::new(r"D:\", "NTFS"),
    ]);
    service.rebuild_with(|mount_point| {
        Ok(test_snapshot(mount_point, Some(17), Some(100), Vec::new()))
    });

    let maintenance = service.poll_journals(&|snapshot| {
        if snapshot.mount_point.eq_ignore_ascii_case(r"C:\") {
            anyhow::bail!("simulated volume fault");
        }
        let mut updated = snapshot.clone();
        updated.next_usn = Some(110);
        Ok(usn::JournalPoll::CursorAdvanced(updated))
    });

    assert!(matches!(maintenance, JournalMaintenance::StatusChanged));
    assert_eq!(service.status.drives[1].next_usn, Some(110));
    assert!(
        service
            .status
            .last_error
            .as_deref()
            .is_some_and(|message| message.contains(r"C:\ USN polling failed"))
    );

    let repaired_d = build_snapshots(
        service.build_drives_for_mounts(&[r"D:\".into()]),
        |mount_point| Ok(test_snapshot(mount_point, Some(17), Some(120), Vec::new())),
    );
    service.apply_build(repaired_d);
    assert!(
        service
            .status
            .last_error
            .as_deref()
            .is_some_and(|message| message.contains(r"C:\ USN polling failed"))
    );
}

#[test]
fn journal_recovery_scopes_rebuild_to_the_affected_volume() {
    let mut service = IndexerService::from_volume_descriptors([
        VolumeDescriptor::new(r"C:\", "NTFS"),
        VolumeDescriptor::new(r"D:\", "NTFS"),
    ]);
    service.rebuild_with(|mount_point| {
        Ok(test_snapshot(mount_point, Some(17), Some(100), Vec::new()))
    });

    let maintenance = service.poll_journals(&|snapshot| {
        if snapshot.mount_point.eq_ignore_ascii_case(r"C:\") {
            Ok(usn::JournalPoll::RebuildRequired("expired cursor".into()))
        } else {
            let mut updated = snapshot.clone();
            updated.next_usn = Some(110);
            Ok(usn::JournalPoll::CursorAdvanced(updated))
        }
    });

    assert!(matches!(
        maintenance,
        JournalMaintenance::RebuildRequired { mount_points, .. }
            if mount_points == vec![r"C:\".to_owned()]
    ));
    assert_eq!(service.status.drives[1].next_usn, Some(110));
    assert!(service.status.last_error.is_none());

    let repaired_c = build_snapshots(
        service.build_drives_for_mounts(&[r"C:\".into()]),
        |mount_point| Ok(test_snapshot(mount_point, Some(18), Some(1), Vec::new())),
    );
    service.apply_build(repaired_c);
    assert!(service.status.last_error.is_none());
}

#[test]
fn scheduled_journal_rebuild_is_maintenance_not_a_user_facing_error() {
    let mut service =
        IndexerService::from_volume_descriptors([VolumeDescriptor::new(r"C:\", "NTFS")]);
    service.rebuild_with(|mount_point| {
        Ok(test_snapshot(
            mount_point,
            Some(17),
            Some(100),
            vec![test_entry(
                format!(r"{mount_point}kept.txt"),
                "kept.txt",
                false,
            )],
        ))
    });

    let maintenance = service.poll_journals(&|_| {
        Ok(usn::JournalPoll::RebuildRequired(
            "bounded directory expansion requested a rebuild".into(),
        ))
    });

    assert!(matches!(
        maintenance,
        JournalMaintenance::RebuildRequired { .. }
    ));
    assert_eq!(service.status.indexed_items, 1);
    assert!(
        service.status.last_error.is_none(),
        "a planned background rebuild keeps the last good snapshot searchable and must not be painted as an error"
    );
}

#[test]
fn one_poll_schedules_compaction_for_every_oversized_volume() {
    let mut service = IndexerService::from_volume_descriptors([
        VolumeDescriptor::new(r"C:\", "NTFS"),
        VolumeDescriptor::new(r"D:\", "NTFS"),
    ]);
    service.status.phase = IndexPhase::Ready;
    let snapshots = [r"C:\", r"D:\"]
        .into_iter()
        .map(|mount_point| {
            let mut snapshot = test_snapshot(mount_point, Some(17), Some(100), Vec::new());
            for offset in 0..=4_096_u64 {
                snapshot.overlay.replacements.insert(
                    10_000 + offset,
                    test_entry(
                        format!(r"{mount_point}overlay-{offset}.txt"),
                        format!("overlay-{offset}.txt"),
                        false,
                    ),
                );
            }
            snapshot
        })
        .collect::<Vec<_>>();
    service.snapshots = Arc::new(snapshots);

    let maintenance =
        service.poll_journals(&|snapshot| Ok(usn::JournalPoll::CursorAdvanced(snapshot.clone())));

    assert!(matches!(
        maintenance,
        JournalMaintenance::CompactionRequired { mount_points }
            if mount_points == vec![r"C:\".to_owned(), r"D:\".to_owned()]
    ));
}

#[test]
fn a_restored_oversized_overlay_retries_compaction_without_new_journal_records() {
    let mut service =
        IndexerService::from_volume_descriptors([VolumeDescriptor::new(r"C:\", "NTFS")]);
    service.status.phase = IndexPhase::Ready;
    let mut snapshot = test_snapshot(r"C:\", Some(17), Some(100), Vec::new());
    for offset in 0..=4_096_u64 {
        snapshot.overlay.replacements.insert(
            10_000 + offset,
            test_entry(
                format!(r"C:\overlay-{offset}.txt"),
                format!("overlay-{offset}.txt"),
                false,
            ),
        );
    }
    service.snapshots = Arc::new(vec![snapshot]);

    let maintenance = service.poll_journals(&|_| Ok(usn::JournalPoll::NoChange));

    assert!(matches!(
        maintenance,
        JournalMaintenance::CompactionRequired { mount_points }
            if mount_points == vec![r"C:\".to_owned()]
    ));
}

#[test]
fn compaction_failures_are_visible_and_exponentially_backed_off_per_volume() {
    assert_eq!(compaction_retry_delay(1), Duration::from_secs(5));
    assert_eq!(compaction_retry_delay(2), Duration::from_secs(10));
    assert_eq!(compaction_retry_delay(20), Duration::from_secs(300));

    let mut service =
        IndexerService::from_volume_descriptors([VolumeDescriptor::new(r"C:\", "NTFS")]);
    service.record_compaction_failure(r"C:\", "disk full");

    assert!(
        service
            .compaction_errors
            .contains_key(&normalized_mount_key(r"C:\"))
    );
    assert!(
        service
            .status
            .last_error
            .as_deref()
            .is_some_and(|error| error.contains("disk full"))
    );
    assert!(
        service
            .compaction_retry_at(r"C:\")
            .is_some_and(|retry| { retry > Instant::now() + Duration::from_secs(4) })
    );

    service.clear_compaction_failure(r"C:\");
    assert!(
        !service
            .compaction_errors
            .contains_key(&normalized_mount_key(r"C:\"))
    );
    assert!(service.status.last_error.is_none());
}

#[test]
fn background_snapshot_maintenance_keeps_the_last_good_index_searchable() {
    let mut service =
        IndexerService::from_volume_descriptors([VolumeDescriptor::new(r"C:\", "NTFS")]);
    service.rebuild_with(|mount_point| {
        Ok(test_snapshot(
            mount_point,
            Some(17),
            Some(100),
            vec![test_entry(r"C:\Reports\Budget.xlsx", "Budget.xlsx", false)],
        ))
    });

    service.begin_snapshot_maintenance();

    assert_eq!(service.status.phase, IndexPhase::VolumesReady);
    assert_eq!(service.status.indexed_items, 1);
    assert!(service.status.last_error.is_none());
    let results = service.search_snapshot(&IndexSearchRequest {
        generation: 1,
        query: "budget".into(),
        limit: 20,
        context: IndexSearchContext {
            directory: None,
            preferred_directory: None,
            include_files: true,
            include_directories: true,
        },
    });
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].path, r"C:\Reports\Budget.xlsx");
}

#[test]
fn healthy_journal_polling_does_not_hide_an_initial_volume_build_failure() {
    let mut service = IndexerService::from_volume_descriptors([
        VolumeDescriptor::new(r"C:\", "NTFS"),
        VolumeDescriptor::new(r"D:\", "NTFS"),
    ]);
    service.rebuild_with(|mount_point| {
        if mount_point.eq_ignore_ascii_case(r"C:\") {
            anyhow::bail!("simulated initial MFT failure");
        }
        Ok(test_snapshot(mount_point, Some(17), Some(100), Vec::new()))
    });
    assert!(
        service
            .status
            .last_error
            .as_deref()
            .is_some_and(|message| message.contains("initial MFT failure"))
    );

    let maintenance = service.poll_journals(&|snapshot| {
        let mut updated = snapshot.clone();
        updated.next_usn = Some(110);
        Ok(usn::JournalPoll::CursorAdvanced(updated))
    });

    assert!(matches!(maintenance, JournalMaintenance::CursorAdvanced));
    assert_eq!(service.status.drives[1].next_usn, Some(110));
    assert!(
        service
            .status
            .last_error
            .as_deref()
            .is_some_and(|message| message.contains("initial MFT failure")),
        "journal success must not clear a missing-volume build error"
    );
}

#[test]
fn idle_journal_polling_backs_off_and_live_changes_restore_the_fast_interval() {
    let now = Instant::now();
    let mut schedule = JournalPollSchedule::new(now);
    schedule.record(&JournalMaintenance::NoChange, now);
    assert_eq!(schedule.remaining(now), Duration::from_secs(2));
    schedule.record(&JournalMaintenance::NoChange, now);
    assert_eq!(schedule.remaining(now), Duration::from_secs(4));

    schedule.record(&JournalMaintenance::Updated, now);

    assert_eq!(schedule.remaining(now), JOURNAL_POLL_MIN_DELAY);

    schedule.defer_until(now + COMPACTION_RETRY_MAX_DELAY);
    assert_eq!(
        schedule.remaining(now),
        JOURNAL_POLL_MIN_DELAY,
        "one volume's compaction cooldown must not delay shared USN polling"
    );
}

#[test]
fn the_service_loop_publishes_usn_changes_before_the_next_search() {
    let mut service =
        IndexerService::from_volume_descriptors([VolumeDescriptor::new(r"C:\", "NTFS")]);
    service.rebuild_with(|mount_point| {
        let mut work = test_entry(r"C:\Work", "Work", true);
        work.identity = Some(mft::MftIdentity {
            file_reference: 10,
            parent_reference: 5,
        });
        Ok(test_snapshot(mount_point, Some(17), Some(100), vec![work]))
    });
    let poller: Arc<JournalPoller> = Arc::new(|snapshot| {
        if snapshot.next_usn != Some(100) {
            return Ok(usn::JournalPoll::NoChange);
        }
        let updated = usn::apply_journal_batch(
            snapshot,
            usn::JournalBatch {
                journal_id: 17,
                lowest_valid_usn: 90,
                next_usn: 110,
                records: vec![usn::UsnRecord::upsert(12, 10, "live.txt", false)],
            },
        )
        .map_err(anyhow::Error::from)?;
        Ok(usn::JournalPoll::Updated(updated))
    });
    let scanner: Arc<SnapshotScanner> =
        Arc::new(|_| anyhow::bail!("snapshot rebuild was not expected during valid catch-up"));
    let (input, receiver) = mpsc::channel();
    let hello = Envelope::new(Message::Hello(Hello {
        client_name: "usn_runtime_test".into(),
        client_version: "0.1.0".into(),
        capabilities: vec!["index_status".into(), "file_search".into()],
    }));
    input
        .send(hello.encode_line().expect("hello frame").into_bytes())
        .expect("queue hello");
    let writer = SharedWriter::default();
    let session_writer = writer.clone();
    let session = thread::spawn(move || {
        serve_session_with_sources(
            BufReader::new(ChannelReader::new(receiver)),
            session_writer,
            &mut service,
            scanner,
            poller,
            None,
        )
    });

    wait_for_frame(&writer, |frame| {
        matches!(
            &frame.message,
            Message::IndexerEvent(IndexerEvent::Status(status))
                if status.drives.first().and_then(|drive| drive.next_usn) == Some(110)
        )
    });
    let search = Envelope::new(Message::IndexerCommand(IndexerCommand::Search(
        IndexSearchRequest {
            generation: 1,
            query: "live".into(),
            limit: 20,
            context: IndexSearchContext {
                directory: None,
                preferred_directory: None,
                include_files: true,
                include_directories: true,
            },
        },
    )));
    input
        .send(search.encode_line().expect("search frame").into_bytes())
        .expect("queue search");
    wait_for_frame(&writer, |frame| {
        matches!(
            &frame.message,
            Message::IndexerEvent(IndexerEvent::SearchBatch(batch))
                if batch.items.len() == 1 && batch.items[0].path == r"C:\Work\live.txt"
        )
    });

    drop(input);
    session
        .join()
        .expect("join USN runtime session")
        .expect("USN runtime session exits cleanly");
}

#[test]
fn continuous_protocol_traffic_cannot_starve_journal_maintenance() {
    let mut service =
        IndexerService::from_volume_descriptors([VolumeDescriptor::new(r"C:\", "NTFS")]);
    service.rebuild_with(|mount_point| {
        Ok(test_snapshot(mount_point, Some(17), Some(100), Vec::new()))
    });
    let polls = Arc::new(AtomicUsize::new(0));
    let poll_calls = Arc::clone(&polls);
    let poller: Arc<JournalPoller> = Arc::new(move |snapshot| {
        poll_calls.fetch_add(1, AtomicOrdering::Relaxed);
        if snapshot.next_usn != Some(100) {
            return Ok(usn::JournalPoll::NoChange);
        }
        let mut updated = snapshot.clone();
        updated.next_usn = Some(110);
        Ok(usn::JournalPoll::CursorAdvanced(updated))
    });
    let scanner: Arc<SnapshotScanner> =
        Arc::new(|_| anyhow::bail!("snapshot rebuild was not expected"));
    let (input, receiver) = mpsc::channel();
    input
        .send(
            Envelope::new(Message::Hello(Hello {
                client_name: "busy_runtime_test".into(),
                client_version: "0.1.0".into(),
                capabilities: vec!["index_status".into()],
            }))
            .encode_line()
            .expect("hello frame")
            .into_bytes(),
        )
        .expect("queue hello");
    let writer = SharedWriter::default();
    let session_writer = writer.clone();
    let session = thread::spawn(move || {
        serve_session_with_sources(
            BufReader::new(ChannelReader::new(receiver)),
            session_writer,
            &mut service,
            scanner,
            poller,
            None,
        )
    });
    let traffic = thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_millis(1_300);
        while Instant::now() < deadline {
            let ping = Envelope::new(Message::Ping(Ping { sent_at_unix_ms: 1 }));
            if input
                .send(ping.encode_line().expect("ping frame").into_bytes())
                .is_err()
            {
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }
    });

    wait_for_frame(&writer, |frame| {
        matches!(
            &frame.message,
            Message::Pong(_) | Message::IndexerEvent(IndexerEvent::Status(_))
        )
    });
    traffic.join().expect("join traffic producer");
    session
        .join()
        .expect("join busy runtime session")
        .expect("busy runtime session exits cleanly");
    assert!(
        polls.load(AtomicOrdering::Relaxed) > 0,
        "continuous request traffic starved the journal deadline"
    );
}

#[test]
fn file_search_context_can_return_only_directories() {
    let mut service =
        IndexerService::from_volume_descriptors([VolumeDescriptor::new(r"C:\", "NTFS")]);
    service.rebuild_with(|mount_point| {
        Ok(test_snapshot(
            mount_point,
            None,
            None,
            vec![
                test_entry(r"C:\Work", "Work", true),
                test_entry(r"C:\Work\notes.txt", "notes.txt", false),
            ],
        ))
    });

    let items = service.search_snapshot(&IndexSearchRequest {
        generation: 1,
        query: "work".into(),
        limit: 20,
        context: IndexSearchContext {
            directory: None,
            preferred_directory: None,
            include_files: false,
            include_directories: true,
        },
    });

    assert_eq!(items.len(), 1);
    assert!(items[0].is_directory);
}

#[test]
fn name_search_candidate_buffers_are_reused_per_worker_thread() {
    NAME_SEARCH_BUFFER_POOL.with(|pool| pool.borrow_mut().clear());
    let mut first = rent_name_search_buffer(32);
    first.push(NameSearchCandidate {
        snapshot_index: 0,
        row_index: 1,
        score: 2,
    });
    let allocation = first.as_ptr();
    recycle_name_search_buffer(first);

    let second = rent_name_search_buffer(16);

    assert_eq!(second.as_ptr(), allocation);
    assert!(second.is_empty());
    recycle_name_search_buffer(second);
}

#[test]
fn a_search_stops_work_when_its_generation_is_cancelled() {
    let mut service =
        IndexerService::from_volume_descriptors([VolumeDescriptor::new(r"C:\", "NTFS")]);
    service.rebuild_with(|mount_point| {
        Ok(test_snapshot(
            mount_point,
            None,
            None,
            (0..20)
                .map(|index| {
                    test_entry(
                        format!(r"C:\Work\report-{index}.txt"),
                        format!("report-{index}.txt"),
                        false,
                    )
                })
                .collect(),
        ))
    });
    let checks = AtomicUsize::new(0);
    let request = IndexSearchRequest {
        generation: 7,
        query: "report".into(),
        limit: 20,
        context: IndexSearchContext::default(),
    };

    let result = service.search_snapshot_while(&request, || {
        checks.fetch_add(1, AtomicOrdering::Relaxed) + 1 < 4
    });

    assert!(result.is_none());
    assert!(checks.load(AtomicOrdering::Relaxed) < 20);
}

#[test]
fn a_panicking_snapshot_worker_reports_failure_instead_of_stalling() {
    let (events, runtime) = mpsc::sync_channel(1);
    let scanner: Arc<SnapshotScanner> = Arc::new(|_| panic!("scanner crashed"));

    spawn_snapshot_build(
        9,
        vec![IndexDriveStatus {
            mount_point: r"C:\".into(),
            file_system: "NTFS".into(),
            indexed_items: 0,
            journal_id: None,
            next_usn: None,
        }],
        scanner,
        events,
    );

    assert!(matches!(
        runtime
            .recv_timeout(Duration::from_secs(1))
            .expect("worker terminal event"),
        RuntimeEvent::BuildFailed {
            generation: 9,
            message,
            ..
        } if message.contains("panic")
    ));
}

#[test]
fn closing_search_executor_cancels_a_completion_blocked_by_backpressure() {
    let (events, _runtime) = mpsc::sync_channel(1);
    events
        .send(RuntimeEvent::ReaderClosed)
        .expect("fill runtime queue");
    let active_task = Arc::new(AtomicU64::new(1));
    let executor = SearchExecutor::start(Arc::clone(&active_task), events);
    executor.submit(SearchJob {
        task_id: 1,
        request: Envelope::new(Message::IndexerCommand(IndexerCommand::Search(
            IndexSearchRequest {
                generation: 1,
                query: "nothing".into(),
                limit: 20,
                context: IndexSearchContext::default(),
            },
        ))),
        snapshots: Arc::new(Vec::new()),
        settings: IndexSettings::default(),
    });
    thread::sleep(Duration::from_millis(10));
    let started = Instant::now();

    drop(executor);

    assert!(started.elapsed() < Duration::from_millis(100));
}

#[test]
fn the_real_coordinator_runs_the_latest_pending_build_after_a_worker_panic() {
    let (input, receiver) = mpsc::channel();
    let (started, scan_started) = mpsc::channel();
    let (release, release_scan) = mpsc::channel();
    let release_scan = Mutex::new(release_scan);
    let calls = AtomicUsize::new(0);
    let scanner: Arc<SnapshotScanner> = Arc::new(move |mount_point| {
        if calls.fetch_add(1, AtomicOrdering::SeqCst) == 0 {
            started.send(()).expect("announce first scan");
            release_scan
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .recv()
                .expect("release first scan");
            panic!("first scan crashed");
        }
        Ok(test_snapshot(
            mount_point,
            Some(5),
            Some(9),
            vec![test_entry(r"C:\Work", "Work", true)],
        ))
    });
    let hello = Envelope::new(Message::Hello(Hello {
        client_name: "coordinator_test".into(),
        client_version: "0.1.0".into(),
        capabilities: vec!["index_status".into(), "file_search".into()],
    }));
    input
        .send(hello.encode_line().expect("hello frame").into_bytes())
        .expect("queue hello");
    let writer = SharedWriter::default();
    let session_writer = writer.clone();
    let session = thread::spawn(move || {
        serve_with_scanner(
            BufReader::new(ChannelReader::new(receiver)),
            session_writer,
            IndexerService::from_volume_descriptors([VolumeDescriptor::new(r"C:\", "NTFS")]),
            scanner,
        )
    });
    scan_started
        .recv_timeout(Duration::from_secs(1))
        .expect("initial build started");
    let rebuild = Envelope::new(Message::IndexerCommand(IndexerCommand::RebuildIndex));
    input
        .send(rebuild.encode_line().expect("rebuild frame").into_bytes())
        .expect("queue rebuild");
    wait_for_frame(&writer, |frame| frame.correlation_id == Some(rebuild.id));

    release.send(()).expect("release crashing scan");
    wait_for_frame(&writer, |frame| {
        matches!(
            &frame.message,
            Message::IndexerEvent(IndexerEvent::Status(status))
                if status.phase == IndexPhase::Ready
                    && status.indexed_items == 1
                    && status.last_error.is_none()
        )
    });

    drop(input);
    session
        .join()
        .expect("coordinator thread")
        .expect("coordinator exits cleanly");
}

#[test]
fn the_real_coordinator_publishes_only_the_latest_equal_generation_retry() {
    let mut service =
        IndexerService::from_volume_descriptors([VolumeDescriptor::new(r"C:\", "NTFS")]);
    service.rebuild_with(|mount_point| {
        let mut entries = (0..50_000)
            .map(|index| {
                test_entry(
                    format!(r"C:\Old\old-result-{index}.txt"),
                    format!("old-result-{index}.txt"),
                    false,
                )
            })
            .collect::<Vec<_>>();
        entries.push(test_entry(
            r"C:\New\new-target.txt",
            "new-target.txt",
            false,
        ));
        Ok(test_snapshot(mount_point, None, None, entries))
    });
    let (input, receiver) = mpsc::channel();
    let (release, release_scan) = mpsc::channel();
    let release_scan = Mutex::new(release_scan);
    let scanner: Arc<SnapshotScanner> = Arc::new(move |mount_point| {
        release_scan
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .recv()
            .expect("release background rebuild");
        Ok(test_snapshot(mount_point, None, None, Vec::new()))
    });
    let hello = Envelope::new(Message::Hello(Hello {
        client_name: "search_retry_test".into(),
        client_version: "0.1.0".into(),
        capabilities: vec!["index_status".into(), "file_search".into()],
    }));
    input
        .send(hello.encode_line().expect("hello frame").into_bytes())
        .expect("queue hello");
    let writer = SharedWriter::default();
    let session_writer = writer.clone();
    let session = thread::spawn(move || {
        serve_with_scanner(
            BufReader::new(ChannelReader::new(receiver)),
            session_writer,
            service,
            scanner,
        )
    });
    let search = |query: &str| {
        Envelope::new(Message::IndexerCommand(IndexerCommand::Search(
            IndexSearchRequest {
                generation: 7,
                query: query.into(),
                limit: 20,
                context: IndexSearchContext {
                    directory: None,
                    preferred_directory: None,
                    include_files: true,
                    include_directories: true,
                },
            },
        )))
    };
    let old = search("old-result");
    let current = search("new-target");
    input
        .send(
            format!(
                "{}{}",
                old.encode_line().expect("old search frame"),
                current.encode_line().expect("current search frame")
            )
            .into_bytes(),
        )
        .expect("queue equal-generation searches");

    wait_for_frame(&writer, |frame| {
        matches!(
            &frame.message,
            Message::IndexerEvent(IndexerEvent::SearchBatch(batch))
                if batch.generation == 7
                    && batch.items.len() == 1
                    && batch.items[0].name == "new-target.txt"
        )
    });
    thread::sleep(Duration::from_millis(50));
    let output = writer
        .0
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .clone();
    let last_batch = String::from_utf8_lossy(&output)
        .lines()
        .filter_map(|line| Envelope::decode_line(line).ok())
        .filter_map(|frame| match frame.message {
            Message::IndexerEvent(IndexerEvent::SearchBatch(batch)) => Some(batch),
            _ => None,
        })
        .next_back()
        .expect("at least one search batch");
    assert_eq!(last_batch.items.len(), 1);
    assert_eq!(last_batch.items[0].name, "new-target.txt");

    release.send(()).expect("release background rebuild");
    drop(input);
    session
        .join()
        .expect("coordinator thread")
        .expect("coordinator exits cleanly");
}

#[test]
fn stdio_handshake_returns_acknowledgement_then_current_status() {
    let hello = Envelope::new(Message::Hello(Hello {
        client_name: "nxtools_desktop".into(),
        client_version: "0.1.0".into(),
        capabilities: vec!["index_status".into(), "file_search".into()],
    }));
    let mut output = Vec::new();

    serve(
        Cursor::new(hello.encode_line().expect("hello frame")),
        &mut output,
        IndexerService::from_volume_descriptors([VolumeDescriptor::new(r"C:\", "NTFS")]),
    )
    .expect("serve one client session");

    let frames = String::from_utf8(output)
        .expect("UTF-8 output")
        .lines()
        .map(Envelope::decode_line)
        .collect::<Result<Vec<_>, _>>()
        .expect("valid response frames");
    assert!(matches!(
        frames.first(),
        Some(Envelope {
            message: Message::HelloAck(_),
            ..
        })
    ));
    assert!(matches!(
        frames.get(1),
        Some(Envelope {
            message: Message::IndexerEvent(IndexerEvent::Status(_)),
            ..
        })
    ));
    assert_eq!(frames[0].correlation_id, Some(hello.id));
}

#[test]
fn an_initial_snapshot_finishes_and_is_retained_after_the_client_disconnects() {
    let hello = Envelope::new(Message::Hello(Hello {
        client_name: "disconnect_test".into(),
        client_version: "0.1.0".into(),
        capabilities: vec!["index_status".into()],
    }));
    let scanner = Arc::new(|mount_point: &str| {
        std::thread::sleep(Duration::from_millis(25));
        Ok(test_snapshot(
            mount_point,
            Some(7),
            Some(11),
            vec![test_entry(
                format!(r"{mount_point}retained.txt"),
                "retained.txt",
                false,
            )],
        ))
    });
    let mut service =
        IndexerService::from_volume_descriptors([VolumeDescriptor::new(r"C:\", "NTFS")]);
    let mut output = Vec::new();

    serve_session_with_scanner(
        Cursor::new(hello.encode_line().expect("hello frame")),
        &mut output,
        &mut service,
        scanner,
    )
    .expect("disconnected session retains completed build");

    assert_eq!(service.status.phase, IndexPhase::Ready);
    assert_eq!(service.status.indexed_items, 1);
    assert_eq!(service.snapshots.len(), 1);
}

#[test]
fn a_slow_initial_snapshot_does_not_block_handshake_or_ping() {
    let hello = Envelope::new(Message::Hello(Hello {
        client_name: "nxtools_desktop".into(),
        client_version: "0.1.0".into(),
        capabilities: vec!["index_status".into(), "file_search".into()],
    }));
    let ping = Envelope::new(Message::Ping(nxtools_protocol::Ping {
        sent_at_unix_ms: 42,
    }));
    let (input, receiver) = mpsc::sync_channel(4);
    input
        .send(hello.encode_line().expect("hello frame").into_bytes())
        .expect("queue hello");
    input
        .send(ping.encode_line().expect("ping frame").into_bytes())
        .expect("queue ping");
    let scanner = Arc::new(|mount_point: &str| {
        std::thread::sleep(Duration::from_millis(250));
        Ok(test_snapshot(mount_point, None, None, Vec::new()))
    });
    let output = SharedWriter::default();
    let session_output = output.clone();
    let started = Instant::now();
    let session = thread::spawn(move || {
        serve_with_scanner(
            BufReader::new(ChannelReader::new(receiver)),
            session_output,
            IndexerService::from_volume_descriptors([VolumeDescriptor::new(r"C:\", "NTFS")]),
            scanner,
        )
    });
    wait_for_frame(&output, |frame| {
        matches!(
            frame.message,
            Message::Pong(nxtools_protocol::Pong {
                sent_at_unix_ms: 42
            })
        )
    });
    assert!(started.elapsed() < Duration::from_millis(150));
    let bytes = output
        .0
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .clone();
    let frames = String::from_utf8(bytes)
        .expect("UTF-8 output")
        .lines()
        .map(Envelope::decode_line)
        .collect::<Result<Vec<_>, _>>()
        .expect("valid response frames");
    assert!(matches!(frames[0].message, Message::HelloAck(_)));
    assert!(matches!(
        frames[1].message,
        Message::IndexerEvent(IndexerEvent::Status(IndexStatus {
            phase: IndexPhase::VolumesReady,
            ..
        }))
    ));
    assert!(matches!(
        frames[2].message,
        Message::Pong(nxtools_protocol::Pong {
            sent_at_unix_ms: 42
        })
    ));
    drop(input);
    session
        .join()
        .expect("join slow scanner session")
        .expect("serve while the snapshot worker is busy");
}

#[test]
fn a_wrongly_routed_message_receives_a_correlated_protocol_fault() {
    let mut service = IndexerService::from_volume_descriptors([]);
    let request = Envelope::new(Message::Pong(nxtools_protocol::Pong { sent_at_unix_ms: 7 }));

    let responses = service.handle(request.clone());

    assert!(matches!(
        responses.as_slice(),
        [Envelope {
            correlation_id: Some(correlation_id),
            message: Message::Error(ProtocolFault { code, .. }),
            ..
        }] if *correlation_id == request.id && code == "invalid_indexer_message"
    ));
}

#[cfg(windows)]
#[allow(unsafe_code)]
fn current_working_set_bytes() -> u64 {
    use std::mem::size_of;
    use windows_sys::Win32::System::ProcessStatus::{
        GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS,
    };
    use windows_sys::Win32::System::Threading::GetCurrentProcess;

    let mut counters = unsafe { std::mem::zeroed::<PROCESS_MEMORY_COUNTERS>() };
    counters.cb = size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
    let succeeded = unsafe {
        GetProcessMemoryInfo(
            GetCurrentProcess(),
            &raw mut counters,
            size_of::<PROCESS_MEMORY_COUNTERS>() as u32,
        )
    };
    if succeeded == 0 {
        0
    } else {
        counters.WorkingSetSize as u64
    }
}

#[cfg(not(windows))]
fn current_working_set_bytes() -> u64 {
    0
}

#[test]
#[ignore = "explicit 1m/3m phase-two scale gate; run through tests/performance/indexer-scale-baseline.ps1"]
fn synthetic_index_scale_baseline() {
    let entry_count = std::env::var("NXTOOLS_INDEX_BENCH_ENTRIES")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(1_000_000);
    assert!(matches!(entry_count, 1_000_000 | 3_000_000));
    let memory_before = current_working_set_bytes();
    let build_started = Instant::now();
    let entries = (0..entry_count)
        .map(|index| mft::MftEntry {
            identity: Some(mft::MftIdentity {
                file_reference: 10 + index as u64,
                parent_reference: 5,
            }),
            path: format!(r"C:\bench\item-{index:07}.txt"),
            name: format!("item-{index:07}.txt"),
            is_directory: false,
            size_bytes: Some(index as u64),
            modified_at_unix_ms: None,
        })
        .collect::<Vec<_>>();
    let snapshot = MftSnapshot::from_entries(r"C:\", Some(17), Some(100), entries);
    let build_ms = build_started.elapsed().as_millis();
    let working_set_bytes = current_working_set_bytes().saturating_sub(memory_before);
    let request = IndexSearchRequest {
        generation: 1,
        query: format!("item-{:07}", entry_count - 1),
        limit: 20,
        context: IndexSearchContext {
            directory: None,
            preferred_directory: None,
            include_files: true,
            include_directories: true,
        },
    };
    let settings = IndexSettings::default();
    let mut query_us = Vec::with_capacity(7);
    for _ in 0..7 {
        let started = Instant::now();
        let results =
            search_snapshots_while(std::slice::from_ref(&snapshot), &settings, &request, || {
                true
            })
            .expect("scale query was not cancelled");
        assert_eq!(results.len(), 1);
        query_us.push(started.elapsed().as_micros() as u64);
    }
    query_us.sort_unstable();
    let incremental_started = Instant::now();
    let incremented = usn::apply_journal_batch(
        &snapshot,
        usn::JournalBatch {
            journal_id: 17,
            lowest_valid_usn: 90,
            next_usn: 101,
            records: vec![usn::UsnRecord::upsert(
                entry_count as u64 + 100,
                5,
                "incremental-probe.txt",
                false,
            )],
        },
    )
    .expect("one-row incremental update");
    assert!(
        incremented
            .visible_entry(entry_count as u64 + 100)
            .is_some()
    );
    let incremental_us = incremental_started.elapsed().as_micros();
    println!(
        "NXTOOLS_SCALE_BASELINE {{\"entries\":{entry_count},\"build_ms\":{build_ms},\"query_p50_us\":{},\"query_p95_us\":{},\"incremental_us\":{incremental_us},\"working_set_bytes\":{working_set_bytes}}}",
        query_us[3], query_us[6]
    );
}
