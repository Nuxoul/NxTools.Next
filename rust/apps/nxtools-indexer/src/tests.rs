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
    retired: Option<Arc<AtomicBool>>,
}

impl ChannelReader {
    fn new(receiver: mpsc::Receiver<Vec<u8>>) -> Self {
        Self {
            receiver,
            current: Cursor::new(Vec::new()),
            retired: None,
        }
    }

    fn retiring(receiver: mpsc::Receiver<Vec<u8>>, retired: Arc<AtomicBool>) -> Self {
        Self {
            receiver,
            current: Cursor::new(Vec::new()),
            retired: Some(retired),
        }
    }
}

impl Read for ChannelReader {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        loop {
            if self
                .retired
                .as_ref()
                .is_some_and(|flag| flag.load(Ordering::Acquire))
            {
                return Ok(0);
            }
            let read = self.current.read(buffer)?;
            if read > 0 {
                return Ok(read);
            }
            match self.receiver.recv_timeout(Duration::from_millis(8)) {
                Ok(next) => self.current = Cursor::new(next),
                Err(mpsc::RecvTimeoutError::Timeout) => continue,
                Err(mpsc::RecvTimeoutError::Disconnected) => return Ok(0),
            }
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
            retain_kinds_apart: false,
            ..Default::default()
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
                retain_kinds_apart: false,
                ..Default::default()
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
fn failed_first_build_without_a_snapshot_never_publishes_ready() {
    let mut service =
        IndexerService::from_volume_descriptors([VolumeDescriptor::new(r"C:\", "NTFS")]);
    service.rebuild_with(|_| anyhow::bail!("initial scan failed"));
    assert_eq!(service.status.phase, IndexPhase::VolumesReady);
    assert!(service.snapshots.is_empty());
    assert!(
        service
            .status
            .last_error
            .as_deref()
            .unwrap_or("")
            .contains("initial scan failed")
    );

    service.record_build_failure(vec![r"C:\".into()], "worker panicked".into());
    assert_eq!(service.status.phase, IndexPhase::VolumesReady);
    assert!(
        service
            .status
            .last_error
            .as_deref()
            .unwrap_or("")
            .contains("worker panicked")
    );
}

#[test]
fn failed_maintenance_build_keeps_last_good_snapshot_ready_and_searchable() {
    let mut service =
        IndexerService::from_volume_descriptors([VolumeDescriptor::new(r"C:\", "NTFS")]);
    service.rebuild_with(|mount_point| {
        Ok(test_snapshot(
            mount_point,
            Some(17),
            Some(100),
            vec![test_entry(r"C:\Work\Budget.xlsx", "Budget.xlsx", false)],
        ))
    });
    service.begin_snapshot_maintenance();
    service.rebuild_with(|_| anyhow::bail!("replacement scan failed"));
    assert_eq!(service.status.phase, IndexPhase::Ready);
    assert_eq!(service.snapshots.len(), 1);
    assert!(
        service
            .status
            .last_error
            .as_deref()
            .unwrap_or("")
            .contains("replacement scan failed")
    );
    service.record_build_failure(vec![r"C:\".into()], "worker panicked".into());
    assert_eq!(service.status.phase, IndexPhase::Ready);
    assert_eq!(
        service
            .search_snapshot(&IndexSearchRequest {
                generation: 1,
                query: "budget".into(),
                limit: 20,
                context: IndexSearchContext {
                    include_files: true,
                    include_directories: true,
                    ..IndexSearchContext::default()
                },
            })
            .len(),
        1
    );
}

#[test]
fn preadmitted_session_rejects_non_hello_before_state_or_ack() {
    let mut service = IndexerService::from_volume_descriptors([]);
    let mut output = Vec::new();
    let result = serve_session_with_sources_and_handshake(
        Cursor::new(Vec::<u8>::new()),
        &mut output,
        &mut service,
        Arc::new(|_| anyhow::bail!("unexpected scan")),
        Arc::new(|_| Ok(usn::JournalPoll::NoChange)),
        None,
        Some(Envelope::new(Message::Ping(Ping { sent_at_unix_ms: 1 }))),
    );
    assert!(result.is_err());
    assert!(
        output.is_empty(),
        "invalid handshake cannot publish HelloAck"
    );
    assert!(service.active_generation.is_none());
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
                retain_kinds_apart: false,
                ..Default::default()
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
            maintenance: None,
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
            maintenance: None,
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
            retain_kinds_apart: false,
            ..Default::default()
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
            retain_kinds_apart: false,
            ..Default::default()
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
            retain_kinds_apart: false,
            ..Default::default()
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

    let maintenance = service.poll_journals_with_persistence(
        &|snapshot| {
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
        },
        &|_| Ok(()),
    );

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
                    retain_kinds_apart: false,
                    ..Default::default()
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
            retain_kinds_apart: false,
            ..Default::default()
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
            )
            .into(),
        );
    }
    service.status.phase = IndexPhase::Ready;
    service.snapshots = Arc::new(vec![snapshot.clone()]);

    let maintenance = service.poll_journals_with_persistence(
        &move |_| Ok(usn::JournalPoll::Updated(snapshot.clone())),
        &|_| Ok(()),
    );

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
            retain_kinds_apart: false,
            ..Default::default()
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

    let maintenance = service.poll_journals_with_persistence(
        &|snapshot| {
            if snapshot.mount_point.eq_ignore_ascii_case(r"C:\") {
                anyhow::bail!("simulated volume fault");
            }
            let mut updated = snapshot.clone();
            updated.next_usn = Some(110);
            Ok(usn::JournalPoll::CursorAdvanced(updated))
        },
        &|_| Ok(()),
    );

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

    let maintenance = service.poll_journals_with_persistence(
        &|snapshot| {
            if snapshot.mount_point.eq_ignore_ascii_case(r"C:\") {
                Ok(usn::JournalPoll::RebuildRequired(usn::JournalRebuild::new(
                    IndexRebuildReason::CursorInvalid,
                )))
            } else {
                let mut updated = snapshot.clone();
                updated.next_usn = Some(110);
                Ok(usn::JournalPoll::CursorAdvanced(updated))
            }
        },
        &|_| Ok(()),
    );

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

    let maintenance = service.poll_journals_with_persistence(
        &|_| {
            Ok(usn::JournalPoll::RebuildRequired(usn::JournalRebuild::new(
                IndexRebuildReason::BulkChanges,
            )))
        },
        &|_| Ok(()),
    );

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
                    )
                    .into(),
                );
            }
            snapshot
        })
        .collect::<Vec<_>>();
    service.snapshots = Arc::new(snapshots);

    let maintenance = service.poll_journals_with_persistence(
        &|snapshot| Ok(usn::JournalPoll::CursorAdvanced(snapshot.clone())),
        &|_| Ok(()),
    );

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
            )
            .into(),
        );
    }
    service.snapshots = Arc::new(vec![snapshot]);

    let maintenance =
        service.poll_journals_with_persistence(&|_| Ok(usn::JournalPoll::NoChange), &|_| Ok(()));

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
            retain_kinds_apart: false,
            ..Default::default()
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

    let maintenance = service.poll_journals_with_persistence(
        &|snapshot| {
            let mut updated = snapshot.clone();
            updated.next_usn = Some(110);
            Ok(usn::JournalPoll::CursorAdvanced(updated))
        },
        &|_| Ok(()),
    );

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
                retain_kinds_apart: false,
                ..Default::default()
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
            retain_kinds_apart: false,
            ..Default::default()
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
        vec!["panic-fixture".into()],
        vec![IndexDriveStatus {
            maintenance: None,
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
        .send(RuntimeEvent::ReaderClosed { lease: 0 })
        .expect("fill runtime queue");
    let active_task = Arc::new(AtomicU64::new(1));
    let executor = SearchExecutor::start(Arc::clone(&active_task), events);
    executor.submit(SearchJob {
        lease: 0,
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
                    retain_kinds_apart: false,
                    ..Default::default()
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
            retain_kinds_apart: false,
            ..Default::default()
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

#[test]
fn r8_search_matches_frozen_row_scan_with_delta_filters_and_cancellation() {
    let mut entries = (0..600)
        .map(|index| {
            let name = [
                "report.txt",
                "my-report.txt",
                "re-port.txt",
                "计算器.txt",
                "Kelvin.txt",
                "notes.txt",
            ][index % 6];
            let mut entry =
                test_entry(format!(r"C:\Work\d{index:04}\{name}"), name, index % 7 == 0);
            entry.identity = Some(mft::MftIdentity {
                file_reference: 10 + index as u64,
                parent_reference: 5,
            });
            entry
        })
        .collect::<Vec<_>>();
    entries.push(test_entry(r"D:\Outside\report.txt", "report.txt", false));
    let mut snapshot = test_snapshot(r"C:\", Some(17), Some(100), entries);
    snapshot.overlay.deleted.insert(10);
    snapshot.overlay.replacements.insert(
        11,
        test_entry(r"C:\Work\renamed-report.txt", "renamed-report.txt", false).into(),
    );
    snapshot.overlay.replacements.insert(
        900,
        test_entry(r"C:\Work\xyz-new.txt", "xyz-new.txt", false).into(),
    );
    let snapshots = [snapshot];
    let settings = IndexSettings {
        indexed_volumes: vec![r"C:\".into()],
        excluded_paths: vec![r"C:\Work\d0002".into()],
    };
    for text in [
        "r",
        "re",
        "report",
        "rpt",
        "'report",
        "^report",
        "txt$",
        "'kelvin",
        "计",
        "report | notes",
        "report !my",
        "report txt",
        r"C:\Work\report",
        "::Work report",
        "xyz",
        "'xyz",
        "*'report",
    ] {
        for limit in [1, 20, 200] {
            for files_only in [false, true] {
                let request = IndexSearchRequest {
                    generation: 1,
                    query: text.into(),
                    limit,
                    context: IndexSearchContext {
                        include_files: true,
                        include_directories: !files_only,
                        ..IndexSearchContext::default()
                    },
                };
                let query = FileQuery::parse(text);
                // The pre-R8 hot scan is preserved separately as the comparison oracle.
                let expected = if query.is_name_only() {
                    legacy_search_name_only_snapshots_while(
                        &snapshots,
                        &settings,
                        &request,
                        &query,
                        limit,
                        &|| true,
                    )
                } else {
                    search_full_path_snapshots_while(
                        &snapshots,
                        &settings,
                        &request,
                        &query,
                        None,
                        limit,
                        &|| true,
                    )
                };
                assert_eq!(
                    search_snapshots_while(&snapshots, &settings, &request, || true),
                    expected,
                    "{text}, limit={limit}, files_only={files_only}"
                );
                assert!(
                    search_snapshots_while(&snapshots, &settings, &request, || false).is_none()
                );
            }
        }
    }
}

#[test]
fn r8_group_fanout_preserves_row_headroom_ties_and_cold_filter_underfill() {
    // Interleaved names: a later group can beat the earlier group's tail on
    // row ties, and the final group has a strictly better name score.
    let entries = (0..2400)
        .map(|row| {
            let name = ["a-needle.txt", "b-needle.txt", "needle.txt"][row % 3];
            test_entry(format!(r"C:\Work\d{row:04}\{name}"), name, row < 2000)
        })
        .collect();
    let snapshots = [test_snapshot(r"C:\", Some(17), Some(100), entries)];
    for raw in ["needle", "'needle", "txt$", "needle | missing"] {
        for limit in [1, 20, 200] {
            for files_only in [false, true] {
                let request = IndexSearchRequest {
                    generation: 1,
                    query: raw.into(),
                    limit,
                    context: IndexSearchContext {
                        include_files: true,
                        include_directories: !files_only,
                        ..IndexSearchContext::default()
                    },
                };
                let query = FileQuery::parse(raw);
                let settings = IndexSettings::default();
                let expected = legacy_search_name_only_snapshots_while(
                    &snapshots,
                    &settings,
                    &request,
                    &query,
                    limit,
                    &|| true,
                );
                assert_eq!(
                    search_snapshots_while(&snapshots, &settings, &request, || true),
                    expected,
                    "{raw}, limit={limit}, files_only={files_only}"
                );
                let checks = AtomicUsize::new(0);
                assert!(
                    search_snapshots_while(&snapshots, &settings, &request, || {
                        checks.fetch_add(1, AtomicOrdering::Relaxed) < 10
                    })
                    .is_none(),
                    "fanout cancellation for {raw}"
                );
            }
        }
    }
}

#[test]
fn r8_selective_scan_reuses_groups_and_finds_delta_only_trigrams() {
    let mut entries = (0..2000)
        .map(|id| {
            test_entry(
                format!(r"C:\Work\file-{id:05}.txt"),
                format!("file-{id:05}.txt"),
                false,
            )
        })
        .collect::<Vec<_>>();
    for id in 0..100 {
        entries.push(test_entry(
            format!(r"C:\Work\d{id}\rare-needle.txt"),
            "rare-needle.txt",
            false,
        ));
    }
    entries.push(test_entry(r"C:\Work\Kelvin.txt", "Kelvin.txt", false));
    let mut snapshot = test_snapshot(r"C:\", Some(17), Some(100), entries);
    snapshot.overlay.replacements.insert(
        9000,
        test_entry(r"C:\Work\xyz-only-delta.txt", "xyz-only-delta.txt", false).into(),
    );
    let data = snapshot.search_data.as_ref().expect("auxiliary built");
    assert_eq!(data.groups.len(), 2002);
    let query = FileQuery::parse("'needle");
    assert_eq!(
        data.candidates(&query)
            .expect("selective candidate path")
            .len(),
        2
    );
    let cloned = snapshot.clone();
    assert!(Arc::ptr_eq(
        snapshot.search_data.as_ref().expect("data"),
        cloned.search_data.as_ref().expect("shared data")
    ));
    for raw in [
        "'needle",
        "'xyz",
        "'kelvin",
        "'file",
        "f",
        "fie",
        "'needle | file",
        "'needle' | file",
        "^rare | file",
        "'needle !old",
        "'needle txt",
    ] {
        if raw.contains(' ') {
            assert!(data.candidates(&FileQuery::parse(raw)).is_none(), "{raw}");
        }
        let request = IndexSearchRequest {
            generation: 1,
            query: raw.into(),
            limit: 200,
            context: IndexSearchContext::default(),
        };
        let query = FileQuery::parse(raw);
        let snapshots = std::slice::from_ref(&snapshot);
        assert_eq!(
            search_snapshots_while(snapshots, &IndexSettings::default(), &request, || true),
            legacy_search_name_only_snapshots_while(
                snapshots,
                &IndexSettings::default(),
                &request,
                &query,
                200,
                &|| true
            ),
            "{raw}"
        );
        let checks = AtomicUsize::new(0);
        assert!(
            search_snapshots_while(snapshots, &IndexSettings::default(), &request, || {
                checks.fetch_add(1, AtomicOrdering::Relaxed) < 1
            })
            .is_none(),
            "mid-scan cancellation for {raw}"
        );
    }
}

// Safe, small synthetic production benchmark; not ignored and no real volume/service IO.
#[test]
fn r8_synthetic_search_benchmark() {
    let entries = (0..20_000)
        .map(|id| {
            let name = format!("file-{:05}.txt", id % 2000);
            test_entry(format!(r"C:\bench\d{id:05}\{name}"), name, false)
        })
        .collect();
    let started = Instant::now();
    let snapshot = test_snapshot(r"C:\", Some(17), Some(100), entries);
    let build_us = started.elapsed().as_micros();
    let auxiliary_payload_bytes = snapshot
        .search_data
        .as_ref()
        .expect("benchmark auxiliary")
        .payload_capacity_bytes();
    for raw in ["f", "file", "'00123", "'file", "fle", "计", "file | notes"] {
        let request = IndexSearchRequest {
            generation: 1,
            query: raw.into(),
            limit: 20,
            context: IndexSearchContext::default(),
        };
        let query = FileQuery::parse(raw);
        let snapshots = std::slice::from_ref(&snapshot);
        let settings = IndexSettings::default();
        let expected = legacy_search_name_only_snapshots_while(
            snapshots,
            &settings,
            &request,
            &query,
            20,
            &|| true,
        );
        let mut old_us = Vec::new();
        let mut new_us = Vec::new();
        for _ in 0..9 {
            let started = Instant::now();
            let old = legacy_search_name_only_snapshots_while(
                snapshots,
                &settings,
                &request,
                &query,
                20,
                &|| true,
            );
            old_us.push(started.elapsed().as_micros());
            let started = Instant::now();
            let new = search_snapshots_while(snapshots, &settings, &request, || true);
            new_us.push(started.elapsed().as_micros());
            assert_eq!(old, expected);
            assert_eq!(new, expected);
        }
        old_us.sort_unstable();
        new_us.sort_unstable();
        println!(
            "R8_SYNTHETIC rows=20000 groups=2000 query={raw:?} build_us={build_us} auxiliary_payload_bytes={auxiliary_payload_bytes} old_p50_us={} old_p95_us={} new_p50_us={} new_p95_us={}",
            old_us[4], old_us[8], new_us[4], new_us[8]
        );
    }
}

fn persistence_fixture_service() -> IndexerService {
    let mut service = IndexerService::from_volume_descriptors([
        VolumeDescriptor::new(r"C:\", "NTFS"),
        VolumeDescriptor::new(r"D:\", "NTFS"),
    ]);
    service.rebuild_with(|mount_point| {
        Ok(test_snapshot(
            mount_point,
            Some(17),
            Some(100),
            vec![test_entry(
                format!("{mount_point}kept.txt"),
                "kept.txt",
                false,
            )],
        ))
    });
    service
}

fn persistence_fixture_search(service: &IndexerService) {
    assert_eq!(
        service
            .search_snapshot(&IndexSearchRequest {
                generation: 1,
                query: "kept".into(),
                limit: 20,
                context: IndexSearchContext {
                    directory: None,
                    preferred_directory: None,
                    include_files: true,
                    include_directories: true,
                    retain_kinds_apart: false,
                    ..Default::default()
                },
            })
            .len(),
        2,
        "last-good snapshots remain searchable"
    );
}

#[test]
fn persistence_fault_cursor_is_visible_until_current_snapshot_is_saved() {
    let mut service = persistence_fixture_service();
    service.poll_journals_with_persistence(
        &|snapshot| {
            let mut updated = snapshot.clone();
            updated.next_usn = Some(110);
            Ok(usn::JournalPoll::CursorAdvanced(updated))
        },
        &|_| Err(std::io::Error::from_raw_os_error(5).into()),
    );
    assert_eq!(service.status.phase, IndexPhase::Ready);
    assert_eq!(
        service.status.drives[0].next_usn,
        Some(110),
        "original memory-publication policy retained"
    );
    persistence_fixture_search(&service);
    assert!(
        service
            .status
            .last_error
            .as_deref()
            .is_some_and(|error| error.contains("persistence")),
        "cursor persistence failure must be visible"
    );
    service.poll_journals_with_persistence(&|_| Ok(usn::JournalPoll::NoChange), &|_| {
        Err(std::io::Error::from_raw_os_error(5).into())
    });
    assert!(
        service.status.last_error.is_some(),
        "healthy idle poll cannot erase unsaved cursor"
    );
    let saves = std::cell::Cell::new(0);
    let result =
        service.poll_journals_with_persistence(&|_| Ok(usn::JournalPoll::NoChange), &|snapshot| {
            assert_eq!(snapshot.next_usn, Some(110));
            saves.set(saves.get() + 1);
            Ok(())
        });
    assert_eq!(
        saves.get(),
        2,
        "retry saves both current per-volume snapshots"
    );
    assert!(matches!(result, JournalMaintenance::StatusChanged));
    assert!(service.status.last_error.is_none());
}

#[test]
fn persistence_fault_delta_survives_rebuild_reason_and_other_volume_success() {
    let mut service = persistence_fixture_service();
    service.poll_journals_with_persistence(
        &|snapshot| Ok(usn::JournalPoll::Updated(snapshot.clone())),
        &|snapshot| {
            if snapshot.mount_point == r"C:\" {
                Err(std::io::Error::from_raw_os_error(112).into())
            } else {
                Ok(())
            }
        },
    );
    assert!(
        service
            .status
            .last_error
            .as_deref()
            .is_some_and(|error| error.contains("persistence"))
    );
    let rebuild = service.poll_journals_with_persistence(
        &|snapshot| {
            if snapshot.mount_point == r"C:\" {
                Ok(usn::JournalPoll::RebuildRequired(usn::JournalRebuild::new(
                    IndexRebuildReason::BulkChanges,
                )))
            } else {
                Ok(usn::JournalPoll::CursorAdvanced(snapshot.clone()))
            }
        },
        &|_| Ok(()),
    );
    assert!(matches!(
        rebuild,
        JournalMaintenance::RebuildRequired { .. }
    ));
    let error = service
        .status
        .last_error
        .as_deref()
        .expect("unsaved delta retained");
    assert!(error.contains("persistence"));
    assert!(
        !error.contains("bounded maintenance reason"),
        "maintenance reason is not user failure"
    );
    service.rediscover_from_volume_descriptors([
        VolumeDescriptor::new(r"C:\", "NTFS"),
        VolumeDescriptor::new(r"D:\", "NTFS"),
    ]);
    assert!(
        service.status.last_error.is_some(),
        "rediscovery retains persistence fault"
    );
    persistence_fixture_search(&service);
    service.rebuild_with(|mount_point| {
        Ok(test_snapshot(
            mount_point,
            Some(17),
            Some(120),
            vec![test_entry(
                format!("{mount_point}kept.txt"),
                "kept.txt",
                false,
            )],
        ))
    });
    assert!(
        service.status.last_error.is_some(),
        "successful scan alone is not delta persistence acknowledgement"
    );
    service.poll_journals_with_persistence(&|_| Ok(usn::JournalPoll::NoChange), &|_| Ok(()));
    assert!(service.status.last_error.is_none());
}

#[test]
fn persistence_fault_settings_retry_same_value_preserves_scope_and_clears_only_after_save() {
    let mut service = persistence_fixture_service();
    service.persist_settings = true;
    let desired = IndexSettings {
        indexed_volumes: vec![r"C:\".into(), r"D:\".into()],
        excluded_paths: vec![r"C:\Private".into()],
    };
    service.apply_settings_with_persistence(desired.clone(), &|_| {
        Err(std::io::Error::from_raw_os_error(5).into())
    });
    assert_eq!(
        service.settings, desired,
        "original live settings application order retained"
    );
    assert!(
        service
            .status
            .last_error
            .as_deref()
            .is_some_and(|error| error.contains("settings persistence"))
    );
    persistence_fixture_search(&service);
    service.poll_journals_with_persistence(&|_| Ok(usn::JournalPoll::NoChange), &|_| Ok(()));
    assert!(
        service.status.last_error.is_some(),
        "journal success is not settings-save success"
    );
    let saves = std::cell::Cell::new(0);
    service.apply_settings_with_persistence(desired.clone(), &|settings| {
        assert_eq!(settings, &desired);
        saves.set(saves.get() + 1);
        Ok(())
    });
    assert_eq!(
        saves.get(),
        1,
        "same desired settings retries prior failed save"
    );
    assert!(service.status.last_error.is_none());
}

#[test]
fn persistence_fault_settings_survives_rediscovery_and_successful_snapshot_build() {
    let mut service = persistence_fixture_service();
    service.persist_settings = true;
    let desired = IndexSettings {
        indexed_volumes: vec![r"C:\".into(), r"D:\".into()],
        excluded_paths: Vec::new(),
    };
    service.apply_settings_with_persistence(desired, &|_| {
        Err(std::io::Error::from_raw_os_error(5).into())
    });
    assert!(service.status.last_error.is_some());
    service.rediscover_from_volume_descriptors([
        VolumeDescriptor::new(r"C:\", "NTFS"),
        VolumeDescriptor::new(r"D:\", "NTFS"),
    ]);
    service.rebuild_with(|mount_point| {
        Ok(test_snapshot(
            mount_point,
            Some(17),
            Some(120),
            vec![test_entry(
                format!("{mount_point}kept.txt"),
                "kept.txt",
                false,
            )],
        ))
    });
    assert!(
        service.status.last_error.is_some(),
        "unrelated build does not confirm settings persistence"
    );
    persistence_fixture_search(&service);
}

#[test]
fn persistence_fault_repeated_failure_is_quiet_and_per_volume_recovery_is_independent() {
    let mut service = persistence_fixture_service();
    service.poll_journals_with_persistence(
        &|snapshot| Ok(usn::JournalPoll::CursorAdvanced(snapshot.clone())),
        &|_| Err(std::io::Error::from_raw_os_error(5).into()),
    );
    let first_error = service.status.last_error.clone();
    let unchanged = service
        .poll_journals_with_persistence(&|_| Ok(usn::JournalPoll::NoChange), &|_| {
            Err(std::io::Error::from_raw_os_error(5).into())
        });
    assert!(matches!(unchanged, JournalMaintenance::NoChange));
    assert_eq!(service.status.last_error, first_error);
    let recovered =
        service.poll_journals_with_persistence(&|_| Ok(usn::JournalPoll::NoChange), &|snapshot| {
            if snapshot.mount_point == r"C:\" {
                Ok(())
            } else {
                Err(std::io::Error::from_raw_os_error(5).into())
            }
        });
    assert!(matches!(recovered, JournalMaintenance::StatusChanged));
    let error = service
        .status
        .last_error
        .as_deref()
        .expect("D persistence fault retained");
    assert!(!error.contains(&normalized_mount_key(r"C:\")));
    assert!(error.contains(&normalized_mount_key(r"D:\")));
    persistence_fixture_search(&service);
}

#[test]
fn persistence_fault_settings_success_does_not_clear_delta_failure() {
    let mut service = persistence_fixture_service();
    service.poll_journals_with_persistence(
        &|snapshot| Ok(usn::JournalPoll::Updated(snapshot.clone())),
        &|_| Err(std::io::Error::from_raw_os_error(5).into()),
    );
    service.persist_settings = true;
    service.apply_settings_with_persistence(
        IndexSettings {
            indexed_volumes: vec![r"C:\".into(), r"D:\".into()],
            excluded_paths: Vec::new(),
        },
        &|_| Ok(()),
    );
    assert!(
        service
            .status
            .last_error
            .as_deref()
            .is_some_and(|error| error.contains("index persistence"))
    );
    service.poll_journals_with_persistence(&|_| Ok(usn::JournalPoll::NoChange), &|_| Ok(()));
    assert!(service.status.last_error.is_none());
}

#[test]
fn persistence_fault_real_settings_write_keeps_old_bytes_then_recovers_same_draft() {
    let directory = tempfile::tempdir().expect("isolated settings directory");
    let path = directory.path().join("settings.json");
    let original = IndexSettings::default();
    settings::persist_index_settings(&path, &original).expect("last-good settings fixture");
    let original_bytes = std::fs::read(&path).expect("last-good bytes");
    let blocked_temporary = path.with_extension("json.tmp");
    std::fs::create_dir(&blocked_temporary).expect("obstruct temporary file");
    let mut service = persistence_fixture_service();
    service.persist_settings = true;
    let desired = IndexSettings {
        indexed_volumes: vec![r"C:\".into(), r"D:\".into()],
        excluded_paths: vec![r"D:\Private".into()],
    };
    service.apply_settings_with_persistence(desired.clone(), &|value| {
        settings::persist_index_settings(&path, value)
    });
    assert_eq!(
        std::fs::read(&path).expect("preserved old file"),
        original_bytes
    );
    let error = service
        .status
        .last_error
        .as_deref()
        .expect("real write failure visible");
    assert!(error.contains("settings persistence"));
    assert!(
        !error.contains(directory.path().to_string_lossy().as_ref()),
        "status does not leak backing storage path"
    );
    persistence_fixture_search(&service);
    std::fs::remove_dir(&blocked_temporary).expect("recover fixture storage");
    service.apply_settings_with_persistence(desired.clone(), &|value| {
        settings::persist_index_settings(&path, value)
    });
    assert!(service.status.last_error.is_none());
    assert_eq!(
        serde_json::from_slice::<IndexSettings>(&std::fs::read(&path).expect("committed settings"))
            .expect("settings JSON"),
        desired
    );
}

#[test]
fn persistence_fault_status_reply_exposes_failure_and_recovery_with_request_identity() {
    let mut service = persistence_fixture_service();
    service.poll_journals_with_persistence(
        &|snapshot| Ok(usn::JournalPoll::CursorAdvanced(snapshot.clone())),
        &|_| Err(std::io::Error::from_raw_os_error(5).into()),
    );
    let request = Envelope::new(Message::IndexerCommand(IndexerCommand::UpdateSettings(
        service.settings.clone(),
    )));
    let responses = service.handle(request.clone());
    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0].correlation_id, Some(request.id));
    assert!(
        matches!(&responses[0].message, Message::IndexerEvent(IndexerEvent::Status(status)) if status.phase == IndexPhase::Ready && status.last_error.is_some())
    );
    service.poll_journals_with_persistence(&|_| Ok(usn::JournalPoll::NoChange), &|_| Ok(()));
    let responses = service.handle(request);
    assert!(
        matches!(&responses[0].message, Message::IndexerEvent(IndexerEvent::Status(status)) if status.last_error.is_none())
    );
}

#[test]
fn w1_pinyin_search_survives_masks_groups_delta_scopes_and_cancellation() {
    let mut base = (0..160)
        .map(|index| {
            let name = ["报告.txt", "年度报告.docx", "baogao.txt", "其他.txt"][index % 4];
            let mut entry = test_entry(format!(r"C:\Work\d{index:04}\{name}"), name, false);
            entry.identity = Some(mft::MftIdentity {
                file_reference: 10 + index as u64,
                parent_reference: 5,
            });
            entry
        })
        .collect::<Vec<_>>();
    base.push(test_entry(r"C:\Work\银行卡.txt", "银行卡.txt", false));
    let mut snapshot = test_snapshot(r"C:\", Some(17), Some(100), base);
    snapshot.overlay.deleted.insert(10);
    snapshot.overlay.replacements.insert(
        14,
        test_entry(r"C:\Work\other.txt", "other.txt", false).into(),
    );
    snapshot.overlay.replacements.insert(
        900,
        test_entry(r"C:\Work\重庆报告.txt", "重庆报告.txt", false).into(),
    );
    let snapshots = [snapshot];
    let settings = IndexSettings {
        indexed_volumes: vec![r"C:\".into()],
        excluded_paths: vec![r"C:\Work\d0008".into()],
    };
    for text in [
        "baogao",
        "bgao",
        "bg",
        "yinhang",
        "chongqing",
        "baogao txt",
        "c: baogao ::Work",
        "baogao | notes",
        "baogao !年度",
    ] {
        let query = FileQuery::parse(text);
        let request = IndexSearchRequest {
            generation: 1,
            query: text.into(),
            limit: 200,
            context: IndexSearchContext {
                include_files: true,
                include_directories: true,
                ..IndexSearchContext::default()
            },
        };
        let actual = search_snapshots_while(&snapshots, &settings, &request, || true)
            .expect("current search");
        assert!(!actual.is_empty(), "lost all pinyin results: {text}");
        let expected = if query.is_name_only() {
            legacy_search_name_only_snapshots_while(
                &snapshots,
                &settings,
                &request,
                &query,
                200,
                &|| true,
            )
        } else {
            search_full_path_snapshots_while(
                &snapshots,
                &settings,
                &request,
                &query,
                None,
                200,
                &|| true,
            )
        }
        .expect("full scan");
        assert_eq!(actual, expected, "{text}");
        assert!(
            actual
                .iter()
                .all(|item| item.path != r"C:\Work\d0000\报告.txt"
                    && item.path != r"C:\Work\d0004\报告.txt"
                    && item.path != r"C:\Work\d0008\报告.txt")
        );
        if text == "baogao" {
            assert!(
                actual
                    .iter()
                    .any(|item| item.path == r"C:\Work\重庆报告.txt")
            );
        }
        assert!(search_snapshots_while(&snapshots, &settings, &request, || false).is_none());
    }
}

#[test]
fn w2_homophone_snapshot_fallback_preserves_normal_priority_scope_and_cancel() {
    let snapshots = [test_snapshot(
        r"C:\",
        Some(1),
        Some(20),
        vec![
            test_entry(r"C:\Work\年度报告.docx", "年度报告.docx", false),
            test_entry(r"C:\Private\年度报告.docx", "年度报告.docx", false),
        ],
    )];
    let settings = IndexSettings {
        indexed_volumes: vec![r"C:\".into()],
        excluded_paths: vec![r"C:\Private".into()],
    };
    let request = IndexSearchRequest {
        generation: 1,
        query: "年度报高".into(),
        limit: 20,
        context: IndexSearchContext {
            include_files: true,
            include_directories: true,
            allow_homophone_fallback: true,
            ..Default::default()
        },
    };
    let results =
        search_snapshots_while(&snapshots, &settings, &request, || true).expect("current");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].path, r"C:\Work\年度报告.docx");
    let ordinary_only = IndexSearchRequest {
        context: IndexSearchContext {
            allow_homophone_fallback: false,
            ..request.context.clone()
        },
        ..request.clone()
    };
    assert!(
        search_snapshots_while(&snapshots, &settings, &ordinary_only, || true)
            .expect("ordinary-only request")
            .is_empty(),
        "caller with local ordinary hits must suppress indexed approximate work"
    );
    assert!(search_snapshots_while(&snapshots, &settings, &request, || false).is_none());
    let normal = [test_snapshot(
        r"C:\",
        Some(1),
        Some(20),
        vec![
            test_entry(r"C:\Work\年度报高.txt", "年度报高.txt", false),
            test_entry(r"C:\Work\年度报告.docx", "年度报告.docx", false),
        ],
    )];
    let results = search_snapshots_while(&normal, &settings, &request, || true).expect("normal");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "年度报高.txt");
    for text in ["年度报高 !报告", "'年度报高", "d: 年度报高 ::Work"] {
        let request = IndexSearchRequest {
            query: text.into(),
            ..request.clone()
        };
        assert!(
            search_snapshots_while(&snapshots, &settings, &request, || true)
                .expect("scope")
                .is_empty()
        );
    }
}

#[test]
fn w3_mixed_candidate_limit_retains_useful_directory_behind_many_files() {
    let mut entries = (0..900)
        .map(|index| test_entry(format!(r"C:\Work\obj\{index:04}\report"), "report", false))
        .collect::<Vec<_>>();
    entries.push(test_entry(r"C:\Work\report", "report", true));
    let snapshots = [test_snapshot(r"C:\", Some(1), Some(20), entries)];
    let request = IndexSearchRequest {
        generation: 1,
        query: "report".into(),
        limit: 20,
        context: IndexSearchContext {
            include_files: true,
            include_directories: true,
            retain_kinds_apart: true,
            ..Default::default()
        },
    };
    let settings = IndexSettings::default();
    let results =
        search_snapshots_while(&snapshots, &settings, &request, || true).expect("current");
    assert!(
        results.iter().any(|item| item.is_directory),
        "mixed headroom must retain a directory candidate for final rank"
    );
    assert!(
        results.len() <= 40,
        "at most the bounded per-kind candidate budgets"
    );
    let files = IndexSearchRequest {
        context: IndexSearchContext {
            include_directories: false,
            ..request.context.clone()
        },
        ..request.clone()
    };
    assert!(
        search_snapshots_while(&snapshots, &settings, &files, || true)
            .expect("files")
            .iter()
            .all(|item| !item.is_directory)
    );
    assert!(search_snapshots_while(&snapshots, &settings, &request, || false).is_none());
}

#[test]
fn w3_normal_match_in_one_kind_never_adds_homophones_in_other_kind() {
    let snapshots = [test_snapshot(
        r"C:\",
        Some(1),
        Some(20),
        vec![
            test_entry(r"C:\Work\年度报高.txt", "年度报高.txt", false),
            test_entry(r"C:\Work\年度报告", "年度报告", true),
        ],
    )];
    let request = IndexSearchRequest {
        generation: 1,
        query: "年度报高".into(),
        limit: 20,
        context: IndexSearchContext {
            include_files: true,
            include_directories: true,
            retain_kinds_apart: true,
            allow_homophone_fallback: true,
            ..Default::default()
        },
    };
    let results = search_snapshots_while(&snapshots, &IndexSettings::default(), &request, || true)
        .expect("current");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "年度报高.txt");
}

#[test]
fn w4_classifier_budget_failure_is_correlated_through_search_worker() {
    let mut entries = (0..4097)
        .map(|index| {
            test_entry(
                format!(r"C:\Cache{index}\CACHEDIR.TAG"),
                "CACHEDIR.TAG",
                false,
            )
        })
        .collect::<Vec<_>>();
    for index in 0..300 {
        let mut entry = test_entry(format!(r"C:\Human{index}\notes.txt"), "notes.txt", false);
        entry.modified_at_unix_ms = Some(1);
        entries.push(entry);
    }
    let request = Envelope::new(Message::IndexerCommand(IndexerCommand::Search(
        IndexSearchRequest {
            generation: 44,
            query: "recent:".into(),
            limit: 20,
            context: IndexSearchContext {
                include_files: true,
                recent_modified: true,
                ..Default::default()
            },
        },
    )));
    let request_id = request.id;
    let job = SearchJob {
        lease: 0,
        task_id: 7,
        request,
        snapshots: Arc::new(vec![test_snapshot(r"C:\", Some(1), Some(20), entries)]),
        settings: IndexSettings::default(),
    };
    let snapshots = job.snapshots.clone();
    let (sender, receiver) = mpsc::sync_channel(10);
    run_snapshot_search(job, &AtomicU64::new(7), &sender);
    let RuntimeEvent::SearchCompleted {
        task_id, response, ..
    } = receiver.recv().expect("terminal failure")
    else {
        panic!("wrong result");
    };
    assert_eq!(task_id, 7);
    assert_eq!(response.correlation_id, Some(request_id));
    assert!(
        matches!(response.message, Message::IndexerEvent(IndexerEvent::SearchFailed(failure))
        if failure.generation == 44 && failure.reason == "recent_classifier_budget")
    );
    let ordinary = Envelope::new(Message::IndexerCommand(IndexerCommand::Search(
        IndexSearchRequest {
            generation: (1 << 63) | 45,
            query: "recent:".into(),
            limit: 20,
            context: IndexSearchContext {
                include_files: true,
                ..Default::default()
            },
        },
    )));
    let ordinary_id = ordinary.id;
    let ordinary_query =
        if let Message::IndexerCommand(IndexerCommand::Search(search)) = &ordinary.message {
            search.clone()
        } else {
            unreachable!("search")
        };
    assert!(
        search_snapshots_while(
            &snapshots,
            &IndexSettings::default(),
            &ordinary_query,
            || true
        )
        .expect("ordinary prefix is not recent mode")
        .is_empty()
    );
    run_snapshot_search(
        SearchJob {
            lease: 0,
            task_id: 7,
            request: ordinary,
            snapshots,
            settings: IndexSettings::default(),
        },
        &AtomicU64::new(7),
        &sender,
    );
    let RuntimeEvent::SearchCompleted { response, .. } =
        receiver.recv().expect("ordinary terminal batch")
    else {
        panic!("search terminal");
    };
    assert_eq!(response.correlation_id, Some(ordinary_id));
    assert!(
        matches!(response.message, Message::IndexerEvent(IndexerEvent::SearchBatch(batch))
        if batch.generation == (1 << 63) | 45 && batch.complete && batch.items.is_empty()),
        "default/file-dialog request must finish as ordinary search, not recent budget failure"
    );
}

fn m4_hello() -> Envelope {
    Envelope::new(Message::Hello(Hello {
        client_name: "session_fixture".into(),
        client_version: "0.1.0".into(),
        capabilities: vec!["index_status".into(), "file_search".into()],
    }))
}

struct M4StoppedOutput {
    output: SharedWriter,
    stopped: Arc<AtomicBool>,
    entered: mpsc::Sender<()>,
    released: Arc<AtomicBool>,
    retired: Arc<AtomicBool>,
}

impl Write for M4StoppedOutput {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if self.stopped.load(Ordering::Acquire) {
            let _ = self.entered.send(());
            while !self.released.load(Ordering::Acquire) && !self.retired.load(Ordering::Acquire) {
                thread::sleep(Duration::from_millis(2));
            }
            if self.retired.load(Ordering::Acquire) {
                return Err(std::io::ErrorKind::BrokenPipe.into());
            }
        }
        self.output.write(bytes)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn m4_admitted_replacement_ack_status_does_not_wait_for_stopped_output_or_scan() {
    let (started, scan_started) = mpsc::channel();
    let (release, scan_release) = mpsc::channel();
    let scan_release = Mutex::new(scan_release);
    let calls = Arc::new(AtomicUsize::new(0));
    let scan_calls = Arc::clone(&calls);
    let runtime = SharedIndexerRuntime::with_sources(
        IndexerService::from_volume_descriptors([VolumeDescriptor::new(r"C:\", "NTFS")]),
        Arc::new(move |mount| {
            scan_calls.fetch_add(1, AtomicOrdering::SeqCst);
            started.send(()).expect("scan started");
            scan_release
                .lock()
                .expect("scan release lock")
                .recv()
                .expect("release scan");
            Ok(test_snapshot(
                mount,
                Some(7),
                Some(100),
                vec![test_entry(r"C:\retained.txt", "retained.txt", false)],
            ))
        }),
        Arc::new(|_| Ok(usn::JournalPoll::NoChange)),
    );
    let (a_input, a_reader) = mpsc::channel();
    let a_output = SharedWriter::default();
    let stopped = Arc::new(AtomicBool::new(false));
    let released = Arc::new(AtomicBool::new(false));
    let retired = Arc::new(AtomicBool::new(false));
    let (entered, output_blocked) = mpsc::channel();
    let a_runtime = runtime.clone();
    let a_writer = M4StoppedOutput {
        output: a_output.clone(),
        stopped: Arc::clone(&stopped),
        entered,
        released: Arc::clone(&released),
        retired: Arc::clone(&retired),
    };
    let a = thread::spawn(move || {
        a_runtime.serve_admitted(
            BufReader::new(ChannelReader::retiring(a_reader, Arc::clone(&retired))),
            a_writer,
            m4_hello(),
            Arc::new(|_| None),
            retired,
        )
    });
    scan_started
        .recv_timeout(Duration::from_secs(2))
        .expect("held initial scan");
    wait_for_frame(&a_output, |frame| {
        matches!(frame.message, Message::HelloAck(_))
    });
    stopped.store(true, Ordering::Release);
    a_input
        .send(
            Envelope::new(Message::Ping(Ping { sent_at_unix_ms: 1 }))
                .encode_line()
                .expect("ping")
                .into_bytes(),
        )
        .expect("A request");
    output_blocked
        .recv_timeout(Duration::from_secs(2))
        .expect("A stopped reading");

    let (b_input, b_reader) = mpsc::channel();
    let b_output = SharedWriter::default();
    let observed = b_output.clone();
    let b_runtime = runtime.clone();
    let b = thread::spawn(move || {
        let retired = Arc::new(AtomicBool::new(false));
        b_runtime.serve_admitted(
            BufReader::new(ChannelReader::retiring(b_reader, Arc::clone(&retired))),
            observed,
            m4_hello(),
            Arc::new(|_| None),
            retired,
        )
    });
    let deadline = Instant::now() + Duration::from_millis(500);
    let responsive = loop {
        let bytes = b_output.0.lock().expect("B output").clone();
        let frames = String::from_utf8_lossy(&bytes)
            .lines()
            .filter_map(|line| Envelope::decode_line(line).ok())
            .collect::<Vec<_>>();
        if matches!(
            frames.first().map(|frame| &frame.message),
            Some(Message::HelloAck(_))
        ) && matches!(
            frames.get(1).map(|frame| &frame.message),
            Some(Message::IndexerEvent(IndexerEvent::Status(_)))
        ) {
            break true;
        }
        if Instant::now() >= deadline {
            break false;
        }
        thread::sleep(Duration::from_millis(5));
    };
    let retire_deadline = Instant::now() + Duration::from_millis(500);
    while responsive && !a.is_finished() && Instant::now() < retire_deadline {
        thread::sleep(Duration::from_millis(5));
    }
    let a_released_while_scan_held = a.is_finished();
    released.store(true, Ordering::Release);
    release.send(()).expect("release retained scan");
    drop(a_input);
    let _ = a.join().expect("A thread");
    assert!(
        responsive,
        "B HelloAck and current status must arrive within 500ms while A is nonreading and its scan is held"
    );
    assert!(
        a_released_while_scan_held,
        "retired A must release both I/O workers without waiting for its scan"
    );
    wait_for_frame(
        &b_output,
        |frame| matches!(&frame.message, Message::IndexerEvent(IndexerEvent::Status(status)) if status.phase == IndexPhase::Ready && status.indexed_items == 1),
    );
    let stale = m4_search(999, "retained");
    let stale_id = stale.id;
    runtime
        .events_for_test()
        .send(RuntimeEvent::Request {
            lease: 1,
            request: stale.clone(),
        })
        .expect("queued old-lease request");
    runtime
        .events_for_test()
        .send(RuntimeEvent::ReaderClosed { lease: 1 })
        .expect("delayed old EOF");
    runtime
        .events_for_test()
        .send(RuntimeEvent::ReaderFailed {
            lease: 1,
            error: anyhow::anyhow!("old endpoint failed"),
        })
        .expect("delayed old failure");
    let current = m4_search(1, "retained");
    b_input
        .send(current.encode_line().expect("B search").into_bytes())
        .expect("B input");
    wait_for_frame(&b_output, |frame| {
        frame.correlation_id == Some(current.id)
            && matches!(&frame.message, Message::IndexerEvent(IndexerEvent::SearchBatch(batch)) if batch.generation == 1 && batch.complete && batch.items.len() == 1 && batch.items[0].path == r"C:\retained.txt")
    });
    runtime
        .events_for_test()
        .send(RuntimeEvent::SearchCompleted {
            lease: 1,
            task_id: 3,
            response: Envelope::reply_to(
                &stale,
                Message::IndexerEvent(IndexerEvent::SearchBatch(IndexSearchBatch {
                    generation: 999,
                    items: Vec::new(),
                    complete: true,
                    elapsed_us: 0,
                })),
            ),
        })
        .expect("delayed old result");
    let barrier = Envelope::new(Message::Ping(Ping { sent_at_unix_ms: 2 }));
    runtime
        .events_for_test()
        .send(RuntimeEvent::Request {
            lease: 2,
            request: barrier.clone(),
        })
        .expect("FIFO boundary barrier");
    wait_for_frame(&b_output, |frame| frame.correlation_id == Some(barrier.id));
    assert!(
        !m4_frames(&b_output)
            .iter()
            .any(|frame| frame.correlation_id == Some(stale_id)),
        "old results cannot route to B"
    );
    drop(b_input);
    b.join().expect("B thread").expect("B session");
    assert_eq!(
        calls.load(AtomicOrdering::SeqCst),
        1,
        "reconnect must not duplicate the original build"
    );
}

fn m4_search(generation: u64, query: &str) -> Envelope {
    Envelope::new(Message::IndexerCommand(IndexerCommand::Search(
        IndexSearchRequest {
            generation,
            query: query.into(),
            limit: 20,
            context: IndexSearchContext {
                include_files: true,
                ..Default::default()
            },
        },
    )))
}

fn m4_frames(output: &SharedWriter) -> Vec<Envelope> {
    let bytes = output.0.lock().expect("fixture output").clone();
    String::from_utf8_lossy(&bytes)
        .lines()
        .filter_map(|line| Envelope::decode_line(line).ok())
        .collect()
}
#[test]
fn first_durable_volume_is_searchable_while_later_volume_is_blocked() {
    let (later_started, later_entered) = mpsc::channel();
    let (release_later, later_release) = mpsc::channel();
    let later_release = Mutex::new(later_release);
    let scanner = Arc::new(move |mount: &str| {
        if mount == r"D:\" {
            later_started.send(()).expect("later scan entered");
            later_release
                .lock()
                .expect("release lock")
                .recv()
                .expect("release later scan");
        }
        Ok(test_snapshot(
            mount,
            Some(17),
            Some(100),
            vec![test_entry(format!(r"{mount}early.txt"), "early.txt", false)],
        ))
    });
    let (input, receiver) = mpsc::channel();
    input
        .send(m4_hello().encode_line().expect("hello").into_bytes())
        .expect("send hello");
    let output = SharedWriter::default();
    let observed = output.clone();
    let worker = thread::spawn(move || {
        serve_with_scanner(
            BufReader::new(ChannelReader::new(receiver)),
            observed,
            IndexerService::from_volume_descriptors([
                VolumeDescriptor::new(r"C:\", "NTFS"),
                VolumeDescriptor::new(r"D:\", "NTFS"),
            ]),
            scanner,
        )
    });
    later_entered
        .recv_timeout(Duration::from_secs(2))
        .expect("second volume blocked");
    let ping = Envelope::new(Message::Ping(Ping {
        sent_at_unix_ms: 37,
    }));
    input
        .send(ping.encode_line().expect("ping").into_bytes())
        .expect("send ping");
    wait_for_frame(&output, |frame| frame.correlation_id == Some(ping.id));
    let search = m4_search(1, "early");
    input
        .send(search.encode_line().expect("search").into_bytes())
        .expect("send search");
    let batch = wait_for_frame(&output, |frame| {
        frame.correlation_id == Some(search.id)
            && matches!(
                frame.message,
                Message::IndexerEvent(IndexerEvent::SearchBatch(_))
            )
    });
    let frames = m4_frames(&output);
    release_later.send(()).expect("finish later scan");
    wait_for_frame(&output, |frame| {
        matches!(
            &frame.message,
            Message::IndexerEvent(IndexerEvent::Status(status))
                if status.phase == IndexPhase::Ready && status.indexed_items == 2
                    && status.drives.iter().all(|drive| drive.next_usn == Some(100))
        )
    });
    drop(input);
    worker
        .join()
        .expect("coordinator thread")
        .expect("coordinator result");
    assert!(
        frames.iter().any(|frame| matches!(
            &frame.message,
            Message::IndexerEvent(IndexerEvent::Status(status))
                if status.indexed_items == 1 && status.phase != IndexPhase::Ready
                    && status.drives[0].next_usn == Some(100)
                    && status.drives[1].next_usn.is_none()
        )),
        "partial volume status must precede full completion"
    );
    assert!(
        matches!(batch.message,
            Message::IndexerEvent(IndexerEvent::SearchBatch(ref result))
                if result.items.len() == 1 && result.items[0].path == r"C:\early.txt"
        ),
        "first completed volume must answer real search"
    );
}

#[test]
fn m4_completed_build_is_published_without_a_client_and_reconnect_does_not_rescan() {
    let (started, scan_started) = mpsc::channel();
    let (release, scan_release) = mpsc::channel();
    let scan_release = Mutex::new(scan_release);
    let (published, publication) = mpsc::channel();
    let calls = Arc::new(AtomicUsize::new(0));
    let scan_calls = Arc::clone(&calls);
    let runtime = SharedIndexerRuntime::with_sources(
        IndexerService::from_volume_descriptors([VolumeDescriptor::new(r"C:\", "NTFS")]),
        Arc::new(move |mount| {
            scan_calls.fetch_add(1, AtomicOrdering::SeqCst);
            started.send(()).expect("scan started");
            scan_release
                .lock()
                .expect("scan release lock")
                .recv()
                .expect("release scan");
            Ok(test_snapshot(
                mount,
                Some(7),
                Some(100),
                vec![test_entry(r"C:\retained.txt", "retained.txt", false)],
            ))
        }),
        Arc::new(move |_| {
            let _ = published.send(());
            Ok(usn::JournalPoll::NoChange)
        }),
    );
    let (input, receiver) = mpsc::channel();
    let output = SharedWriter::default();
    let observed = output.clone();
    let endpoint_runtime = runtime.clone();
    let a = thread::spawn(move || {
        let retired = Arc::new(AtomicBool::new(false));
        endpoint_runtime.serve_admitted(
            BufReader::new(ChannelReader::retiring(receiver, Arc::clone(&retired))),
            observed,
            m4_hello(),
            Arc::new(|_| None),
            retired,
        )
    });
    wait_for_frame(&output, |frame| {
        matches!(frame.message, Message::HelloAck(_))
    });
    scan_started
        .recv_timeout(Duration::from_secs(2))
        .expect("scan held");
    drop(input);
    a.join()
        .expect("A worker")
        .expect("A disconnected without scan wait");
    release.send(()).expect("finish disconnected scan");
    publication.recv_timeout(Duration::from_secs(3)).expect(
        "service applies completed snapshot and resumes journal maintenance without a client",
    );
    let (input, receiver) = mpsc::channel();
    let output = SharedWriter::default();
    let observed = output.clone();
    let endpoint_runtime = runtime.clone();
    let b = thread::spawn(move || {
        let retired = Arc::new(AtomicBool::new(false));
        endpoint_runtime.serve_admitted(
            BufReader::new(ChannelReader::retiring(receiver, Arc::clone(&retired))),
            observed,
            m4_hello(),
            Arc::new(|_| None),
            retired,
        )
    });
    wait_for_frame(
        &output,
        |frame| matches!(&frame.message, Message::IndexerEvent(IndexerEvent::Status(status)) if status.phase == IndexPhase::Ready && status.indexed_items == 1),
    );
    let frames = m4_frames(&output);
    assert!(matches!(
        frames.first().map(|frame| &frame.message),
        Some(Message::HelloAck(_))
    ));
    assert!(
        matches!(frames.get(1).map(|frame| &frame.message), Some(Message::IndexerEvent(IndexerEvent::Status(status))) if status.indexed_items == 1)
    );
    let search = m4_search(1, "retained");
    input
        .send(search.encode_line().expect("search frame").into_bytes())
        .expect("B search");
    wait_for_frame(&output, |frame| {
        frame.correlation_id == Some(search.id)
            && matches!(&frame.message, Message::IndexerEvent(IndexerEvent::SearchBatch(batch)) if batch.items.len() == 1)
    });
    drop(input);
    b.join().expect("B worker").expect("B session");
    assert_eq!(calls.load(AtomicOrdering::SeqCst), 1);
}

#[test]
fn m4_current_authenticated_handler_replaces_the_previous_endpoint_handler() {
    let runtime = SharedIndexerRuntime::with_sources(
        IndexerService::from_volume_descriptors([]),
        Arc::new(|_| anyhow::bail!("unexpected scan")),
        Arc::new(|_| Ok(usn::JournalPoll::NoChange)),
    );
    let mut endpoints = Vec::new();
    for marker in [11, 22] {
        let (input, receiver) = mpsc::channel();
        let output = SharedWriter::default();
        let observed = output.clone();
        let endpoint_runtime = runtime.clone();
        let worker = thread::spawn(move || {
            let retired = Arc::new(AtomicBool::new(false));
            endpoint_runtime.serve_admitted(
                BufReader::new(ChannelReader::retiring(receiver, Arc::clone(&retired))),
                observed,
                m4_hello(),
                Arc::new(move |request| {
                    matches!(request.message, Message::Ping(_)).then(|| {
                        vec![Envelope::reply_to(
                            request,
                            Message::Pong(Pong {
                                sent_at_unix_ms: marker,
                            }),
                        )]
                    })
                }),
                retired,
            )
        });
        wait_for_frame(&output, |frame| {
            matches!(frame.message, Message::HelloAck(_))
        });
        let ping = Envelope::new(Message::Ping(Ping { sent_at_unix_ms: 0 }));
        input
            .send(ping.encode_line().expect("ping").into_bytes())
            .expect("session ping");
        wait_for_frame(&output, |frame| {
            frame.correlation_id == Some(ping.id)
                && matches!(&frame.message, Message::Pong(pong) if pong.sent_at_unix_ms == marker)
        });
        endpoints.push((input, worker));
    }
    for (input, worker) in endpoints {
        drop(input);
        worker
            .join()
            .expect("endpoint worker")
            .expect("endpoint session");
    }
}

#[test]
fn m4_full_endpoint_output_retires_only_its_io_and_keeps_the_coordinator() {
    let runtime = SharedIndexerRuntime::with_sources(
        IndexerService::from_volume_descriptors([]),
        Arc::new(|_| anyhow::bail!("unexpected scan")),
        Arc::new(|_| Ok(usn::JournalPoll::NoChange)),
    );
    let (input, receiver) = mpsc::channel();
    let output = SharedWriter::default();
    let stopped = Arc::new(AtomicBool::new(false));
    let retired = Arc::new(AtomicBool::new(false));
    let endpoint_retired = Arc::clone(&retired);
    let (entered, blocked) = mpsc::channel();
    let writer = M4StoppedOutput {
        output: output.clone(),
        stopped: Arc::clone(&stopped),
        entered,
        released: Arc::new(AtomicBool::new(false)),
        retired: Arc::clone(&retired),
    };
    let endpoint_runtime = runtime.clone();
    let a = thread::spawn(move || {
        endpoint_runtime.serve_admitted(
            BufReader::new(ChannelReader::retiring(
                receiver,
                Arc::clone(&endpoint_retired),
            )),
            writer,
            m4_hello(),
            Arc::new(|_| None),
            endpoint_retired,
        )
    });
    wait_for_frame(&output, |frame| {
        matches!(
            frame.message,
            Message::IndexerEvent(IndexerEvent::Status(_))
        )
    });
    stopped.store(true, Ordering::Release);
    let ping = Envelope::new(Message::Ping(Ping { sent_at_unix_ms: 1 }));
    input
        .send(ping.encode_line().expect("ping").into_bytes())
        .expect("first ping");
    blocked
        .recv_timeout(Duration::from_secs(2))
        .expect("output is stopped");
    for _ in 0..32 {
        // Feed the real coordinator queue after the writer has stopped draining.
        runtime
            .events_for_test()
            .send(RuntimeEvent::Request {
                lease: 1,
                request: ping.clone(),
            })
            .expect("bounded runtime input");
    }
    let deadline = Instant::now() + Duration::from_millis(500);
    while !a.is_finished() && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(5));
    }
    let released_in_bound = a.is_finished();
    retired.store(true, Ordering::Release);
    drop(input);
    a.join().expect("A I/O workers").expect("retired endpoint");
    assert!(
        released_in_bound,
        "full endpoint output releases its writer and idle reader"
    );
    let (input, receiver) = mpsc::channel();
    let output = SharedWriter::default();
    let observed = output.clone();
    let endpoint_runtime = runtime.clone();
    let b = thread::spawn(move || {
        let retired = Arc::new(AtomicBool::new(false));
        endpoint_runtime.serve_admitted(
            BufReader::new(ChannelReader::retiring(receiver, Arc::clone(&retired))),
            observed,
            m4_hello(),
            Arc::new(|_| None),
            retired,
        )
    });
    wait_for_frame(&output, |frame| {
        matches!(frame.message, Message::HelloAck(_))
    });
    let search = m4_search(1, "anything");
    input
        .send(search.encode_line().expect("search").into_bytes())
        .expect("B request");
    wait_for_frame(&output, |frame| {
        frame.correlation_id == Some(search.id)
            && matches!(&frame.message, Message::IndexerEvent(IndexerEvent::SearchBatch(batch)) if batch.generation == 1 && batch.complete)
    });
    drop(input);
    b.join()
        .expect("B I/O workers")
        .expect("replacement session");
}

#[test]
fn m4_shared_admission_rejects_non_hello_without_resetting_the_current_session() {
    let runtime = SharedIndexerRuntime::with_sources(
        IndexerService::from_volume_descriptors([]),
        Arc::new(|_| anyhow::bail!("unexpected scan")),
        Arc::new(|_| Ok(usn::JournalPoll::NoChange)),
    );
    let (input, receiver) = mpsc::channel();
    let output = SharedWriter::default();
    let observed = output.clone();
    let endpoint_runtime = runtime.clone();
    let a = thread::spawn(move || {
        let retired = Arc::new(AtomicBool::new(false));
        endpoint_runtime.serve_admitted(
            BufReader::new(ChannelReader::retiring(receiver, Arc::clone(&retired))),
            observed,
            m4_hello(),
            Arc::new(|_| None),
            retired,
        )
    });
    wait_for_frame(&output, |frame| {
        matches!(frame.message, Message::HelloAck(_))
    });
    let current = m4_search(99, "empty");
    input
        .send(current.encode_line().expect("search").into_bytes())
        .expect("A search");
    wait_for_frame(&output, |frame| frame.correlation_id == Some(current.id));
    let rejected_output = SharedWriter::default();
    assert!(
        runtime
            .serve_admitted(
                Cursor::new(Vec::<u8>::new()),
                rejected_output.clone(),
                Envelope::new(Message::Ping(Ping { sent_at_unix_ms: 0 })),
                Arc::new(|_| None),
                Arc::new(AtomicBool::new(false))
            )
            .is_err()
    );
    assert!(m4_frames(&rejected_output).is_empty());
    let stale = m4_search(1, "empty");
    input
        .send(stale.encode_line().expect("stale search").into_bytes())
        .expect("A stale request");
    let barrier = Envelope::new(Message::Ping(Ping { sent_at_unix_ms: 1 }));
    input
        .send(barrier.encode_line().expect("barrier").into_bytes())
        .expect("A barrier");
    wait_for_frame(&output, |frame| frame.correlation_id == Some(barrier.id));
    assert!(
        !m4_frames(&output)
            .iter()
            .any(|frame| frame.correlation_id == Some(stale.id))
    );
    drop(input);
    a.join().expect("A worker").expect("A session");
}

#[test]
fn launcher_search_sends_real_file_matches_before_directory_pass_finishes() {
    let entries = (0..20_000)
        .map(|row| {
            test_entry(
                format!(r"C:\fixture\{row:05}\needle-{row:05}.txt"),
                format!("needle-{row:05}.txt"),
                false,
            )
        })
        .chain((0..20_000).map(|row| {
            test_entry(
                format!(r"C:\fixture\folders\{row:05}\needle-{row:05}"),
                format!("needle-{row:05}"),
                true,
            )
        }))
        .collect();
    let snapshots = [test_snapshot(r"C:\", Some(1), Some(20), entries)];
    let request = IndexSearchRequest {
        generation: 71,
        query: "needle".into(),
        limit: 40,
        context: IndexSearchContext {
            include_files: true,
            include_directories: true,
            retain_kinds_apart: true,
            ..Default::default()
        },
    };
    let expected = search_snapshots_while(&snapshots, &IndexSettings::default(), &request, || true)
        .expect("production query completes");
    let mut framed = Vec::new();
    let mut observed_first_before_second_pass = false;
    let started = Instant::now();
    let completed = search_launcher_snapshots_while(
        &snapshots,
        &IndexSettings::default(),
        &request,
        &|| true,
        |items, complete| {
            let frame = Envelope::new(Message::IndexerEvent(IndexerEvent::SearchBatch(
                IndexSearchBatch {
                    generation: request.generation,
                    items,
                    complete,
                    elapsed_us: started.elapsed().as_micros() as u64,
                },
            )));
            write_envelope_sync(&mut framed, &frame).expect("production bounded IPC frame");
            if !complete && !observed_first_before_second_pass {
                observed_first_before_second_pass = true;
                assert!(
                    String::from_utf8_lossy(&framed)
                        .lines()
                        .any(|line| matches!(
                            Envelope::decode_line(line).expect("framed IPC").message,
                            Message::IndexerEvent(IndexerEvent::SearchBatch(batch))
                                if !batch.complete && !batch.items.is_empty()
                        )),
                    "file results must cross the framed boundary before scanning directories"
                );
            }
            true
        },
    );
    assert!(completed);
    assert!(observed_first_before_second_pass);
    let received = String::from_utf8(framed)
        .expect("utf8 frames")
        .lines()
        .map(|line| Envelope::decode_line(line).expect("bounded frame"))
        .filter_map(|envelope| match envelope.message {
            Message::IndexerEvent(IndexerEvent::SearchBatch(batch)) => Some(batch),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(received.last().is_some_and(|batch| batch.complete));
    assert_eq!(received.iter().filter(|batch| batch.complete).count(), 1);
    let mut actual = received
        .into_iter()
        .flat_map(|batch| batch.items)
        .collect::<Vec<_>>();
    actual.sort_by(|left, right| left.path.cmp(&right.path));
    let mut expected = expected;
    expected.sort_by(|left, right| left.path.cmp(&right.path));
    assert_eq!(
        actual, expected,
        "streamed candidates retain both kinds and ranking scores"
    );
}

#[test]
fn launcher_search_cancellation_does_not_send_terminal_after_first_kind() {
    let snapshots = [test_snapshot(
        r"C:\",
        Some(1),
        Some(20),
        vec![
            test_entry(r"C:\a\needle.txt", "needle.txt", false),
            test_entry(r"C:\a\needle", "needle", true),
        ],
    )];
    let request = IndexSearchRequest {
        generation: 8,
        query: "needle".into(),
        limit: 20,
        context: IndexSearchContext {
            include_files: true,
            include_directories: true,
            retain_kinds_apart: true,
            ..Default::default()
        },
    };
    let current = AtomicBool::new(true);
    let mut batches = Vec::new();
    assert!(!search_launcher_snapshots_while(
        &snapshots,
        &IndexSettings::default(),
        &request,
        &|| current.load(Ordering::Acquire),
        |items, complete| {
            batches.push((items.len(), complete));
            current.store(false, Ordering::Release);
            true
        }
    ));
    assert_eq!(batches.len(), 1);
    assert!(!batches[0].1);
}

#[test]
fn launcher_worker_emits_framed_first_batch_before_terminal() {
    let snapshots = Arc::new(vec![test_snapshot(
        r"C:\",
        Some(1),
        Some(20),
        (0..20_000)
            .map(|row| {
                test_entry(
                    format!(r"C:\fixture\{row:05}\needle-{row:05}.txt"),
                    format!("needle-{row:05}.txt"),
                    false,
                )
            })
            .chain((0..20_000).map(|row| {
                test_entry(
                    format!(r"C:\fixture\folders\{row:05}\needle-{row:05}"),
                    format!("needle-{row:05}"),
                    true,
                )
            }))
            .collect(),
    )]);
    let request = Envelope::new(Message::IndexerCommand(IndexerCommand::Search(
        IndexSearchRequest {
            generation: 91,
            query: "needle".into(),
            limit: 40,
            context: IndexSearchContext {
                include_files: true,
                include_directories: true,
                retain_kinds_apart: true,
                ..Default::default()
            },
        },
    )));
    let correlation_id = request.id;
    let (sender, receiver) = mpsc::sync_channel(16);
    let started = Instant::now();
    let worker = thread::spawn(move || {
        run_snapshot_search(
            SearchJob {
                lease: 1,
                task_id: 3,
                request,
                snapshots,
                settings: IndexSettings::default(),
            },
            &AtomicU64::new(3),
            &sender,
        );
    });
    let mut wire = Vec::new();
    let mut first_us = None;
    let mut terminal_us = None;
    let mut items = Vec::new();
    while terminal_us.is_none() {
        let RuntimeEvent::SearchCompleted { response, .. } = receiver
            .recv_timeout(Duration::from_secs(5))
            .expect("production search worker reply")
        else {
            panic!("expected worker search batch");
        };
        write_envelope_sync(&mut wire, &response).expect("framed IPC response");
        let frame = String::from_utf8_lossy(&wire);
        let decoded = Envelope::decode_line(frame.lines().last().expect("last frame"))
            .expect("decode actual framed response");
        assert_eq!(decoded.correlation_id, Some(correlation_id));
        let Message::IndexerEvent(IndexerEvent::SearchBatch(batch)) = decoded.message else {
            panic!("search batch on wire");
        };
        if first_us.is_none() && !batch.items.is_empty() {
            first_us = Some(batch.elapsed_us);
            assert!(
                !batch.complete,
                "first file candidates arrive before terminal"
            );
        }
        if batch.complete {
            terminal_us = Some(batch.elapsed_us);
        }
        items.extend(batch.items);
    }
    worker.join().expect("search worker");
    assert!(items.iter().any(|item| !item.is_directory));
    assert!(items.iter().any(|item| item.is_directory));
    assert!(first_us.expect("first batch") < terminal_us.expect("terminal"));
    println!(
        "LAUNCHER_PROVIDER_FIXTURE first_batch_us={} terminal_us={} receive_us={} rows={}",
        first_us.expect("first batch"),
        terminal_us.expect("terminal"),
        started.elapsed().as_micros(),
        items.len()
    );
}

#[test]
fn launcher_stream_preserves_empty_and_homophone_fallback() {
    let snapshots = [test_snapshot(
        r"C:\",
        Some(1),
        Some(20),
        vec![
            test_entry(r"C:\Work\年度报告.docx", "年度报告.docx", false),
            test_entry(r"C:\Work\年度报告", "年度报告", true),
        ],
    )];
    for raw in ["年度报高", "no-matches-anywhere"] {
        let request = IndexSearchRequest {
            generation: 51,
            query: raw.into(),
            limit: 20,
            context: IndexSearchContext {
                include_files: true,
                include_directories: true,
                retain_kinds_apart: true,
                allow_homophone_fallback: true,
                ..Default::default()
            },
        };
        let expected =
            search_snapshots_while(&snapshots, &IndexSettings::default(), &request, || true)
                .expect("original complete result");
        let mut batches = Vec::new();
        assert!(search_launcher_snapshots_while(
            &snapshots,
            &IndexSettings::default(),
            &request,
            &|| true,
            |items, complete| {
                batches.push((items, complete));
                true
            }
        ));
        assert_eq!(batches.iter().filter(|(_, complete)| *complete).count(), 1);
        assert!(batches.last().is_some_and(|(_, complete)| *complete));
        assert_eq!(
            batches
                .into_iter()
                .flat_map(|(items, _)| items)
                .collect::<Vec<_>>(),
            expected
        );
    }
}

#[path = "search_retirement_tests.rs"]
mod search_retirement_tests;
