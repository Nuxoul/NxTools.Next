use std::io::Write;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, Weak, mpsc};
use std::thread;
use std::time::Duration;

use nxtools_protocol::{
    Envelope, IndexSearchContext, IndexSearchRequest, IndexerCommand, IndexerEvent, Message, Ping,
};

use super::*;
use crate::mft::{MftBase, compact_system_snapshot_with};
use crate::{IndexerService, RuntimeEvent, RuntimeOutput, VolumeDescriptor};

#[derive(Clone, Default)]
struct Frames(Arc<Mutex<Vec<u8>>>);

impl Write for Frames {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0
            .lock()
            .expect("fixture output")
            .extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl Frames {
    fn wait(&self, matches: impl Fn(&Envelope) -> bool) -> Envelope {
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            let bytes = self.0.lock().expect("fixture output").clone();
            for line in bytes
                .split(|byte| *byte == b'\n')
                .filter(|line| !line.is_empty())
            {
                let frame = Envelope::decode_line(std::str::from_utf8(line).expect("UTF-8 frame"))
                    .expect("production response frame");
                if matches(&frame) {
                    return frame;
                }
            }
            assert!(Instant::now() < deadline, "coordinator response deadline");
            thread::sleep(Duration::from_millis(1));
        }
    }
}

fn mapped_input(directory: &Path, mount: &str) -> MftSnapshot {
    let (_, prefix) = system_cache_location(mount);
    let path = directory.join(format!("{prefix}-100.idx"));
    let owned = MftSnapshot::from_entries(
        mount,
        Some(17),
        Some(100),
        vec![MftEntry {
            identity: Some(MftIdentity {
                file_reference: 11,
                parent_reference: 5,
            }),
            path: format!("{mount}keep.txt"),
            name: "keep.txt".into(),
            is_directory: false,
            size_bytes: Some(42),
            modified_at_unix_ms: Some(7),
        }],
    );
    persist_snapshot(&owned, &path).expect("temporary base persistence");
    let (mapped, journal, cursor, visible) =
        MappedSnapshot::open(&path, mount).expect("mapped input");
    let mut snapshot =
        MftSnapshot::from_mapped(mount.into(), journal, cursor, visible, mapped, path);
    snapshot.overlay.deleted.extend(10_000..=14_096);
    snapshot.next_usn = Some(110);
    persist_system_delta(&snapshot).expect("temporary oversized delta");
    assert!(crate::usn::overlay_requires_compaction(&snapshot));
    snapshot
}

struct Started {
    mount: String,
    first_mapping_owners: usize,
    first_pair_exists: bool,
}

struct Fixture {
    events: mpsc::SyncSender<RuntimeEvent>,
    stopping: Arc<AtomicBool>,
    coordinator: Option<thread::JoinHandle<IndexerService>>,
    release: Vec<mpsc::Sender<()>>,
    started: mpsc::Receiver<Started>,
    polled: mpsc::Receiver<(String, bool)>,
    active: Arc<AtomicUsize>,
    maximum: Arc<AtomicUsize>,
    old_mapping: Weak<MappedSnapshot>,
    output: Frames,
    directory: tempfile::TempDir,
}

impl Fixture {
    fn new(fail_first: bool) -> Self {
        let directory = tempfile::tempdir().expect("isolated two-volume compaction cache");
        let first = mapped_input(directory.path(), r"C:\");
        let second = mapped_input(directory.path(), r"D:\");
        let old_mapping = match &first.base {
            MftBase::Mapped(mapped) => Arc::downgrade(mapped),
            _ => panic!("mapped fixture"),
        };
        let first_path = first.cache_path().expect("first persisted base").to_owned();
        let mut service = IndexerService::from_volume_descriptors([
            VolumeDescriptor::new(r"C:\", "NTFS"),
            VolumeDescriptor::new(r"D:\", "NTFS"),
        ]);
        service.rebuild_with(|mount| {
            Ok(if mount == r"C:\" {
                first.clone()
            } else {
                second.clone()
            })
        });
        drop(first);
        drop(second);
        let (first_release, first_gate) = mpsc::channel();
        let (second_release, second_gate) = mpsc::channel();
        let gates = [Mutex::new(first_gate), Mutex::new(second_gate)];
        let (started_sender, started) = mpsc::channel();
        let active = Arc::new(AtomicUsize::new(0));
        let maximum = Arc::new(AtomicUsize::new(0));
        let worker_active = Arc::clone(&active);
        let worker_maximum = Arc::clone(&maximum);
        let input_mapping = old_mapping.clone();
        let root = directory.path().to_owned();
        service.compaction_source = Some(Arc::new(move |snapshot| {
            let current = worker_active.fetch_add(1, Ordering::SeqCst) + 1;
            worker_maximum.fetch_max(current, Ordering::SeqCst);
            let _ = started_sender.send(Started {
                mount: snapshot.mount_point.clone(),
                first_mapping_owners: input_mapping.strong_count(),
                first_pair_exists: first_path.exists() && delta_path(&first_path).exists(),
            });
            let index = usize::from(snapshot.mount_point != r"C:\");
            let _ = gates[index].lock().expect("compaction gate").recv();
            let outcome = if fail_first && index == 0 {
                Err(std::io::Error::from_raw_os_error(112).into())
            } else {
                compact_system_snapshot_with(snapshot, |compacted| {
                    let (_, prefix) = system_cache_location(&compacted.mount_point);
                    let path = root.join(format!("{prefix}-200.idx"));
                    persist_snapshot(compacted, &path)?;
                    compacted.set_cache_path(path);
                    Ok(())
                })
            };
            worker_active.fetch_sub(1, Ordering::SeqCst);
            outcome
        }));
        let (poll_sender, polled) = mpsc::channel();
        let poller = Arc::new(move |snapshot: &MftSnapshot| {
            let _ = poll_sender.send((
                snapshot.mount_point.clone(),
                crate::usn::overlay_requires_compaction(snapshot),
            ));
            Ok(crate::usn::JournalPoll::NoChange)
        });
        let (events, runtime) = mpsc::sync_channel(64);
        let coordinator_events = events.clone();
        let stopping = Arc::new(AtomicBool::new(false));
        let coordinator_stopping = Arc::clone(&stopping);
        let output = Frames::default();
        let writer = output.clone();
        let coordinator = thread::spawn(move || {
            crate::run_coordinator(
                &mut service,
                Arc::new(|_| panic!("compaction fixture must not scan real volumes")),
                poller,
                RuntimeOutput::standalone(writer, None),
                coordinator_events,
                runtime,
                coordinator_stopping,
            )
            .expect("production coordinator");
            service
        });
        Self {
            events,
            stopping,
            coordinator: Some(coordinator),
            release: vec![first_release, second_release],
            started,
            polled,
            active,
            maximum,
            old_mapping,
            output,
            directory,
        }
    }

    fn request(&self, message: Message) -> Envelope {
        let request = Envelope::new(message);
        self.events
            .send(RuntimeEvent::Request {
                lease: 0,
                request: request.clone(),
            })
            .expect("coordinator request");
        request
    }

    fn fence(&self) {
        let request = self.request(Message::Ping(Ping { sent_at_unix_ms: 1 }));
        self.output
            .wait(|frame| frame.correlation_id == Some(request.id));
    }

    fn wait_second_publication(&self) {
        let deadline = Instant::now() + Duration::from_secs(4);
        loop {
            let (mount, oversized) = self
                .polled
                .recv_timeout(deadline.saturating_duration_since(Instant::now()))
                .expect("poll after second publication");
            if mount == r"D:\" && !oversized {
                return;
            }
        }
    }

    fn stop(&mut self) -> IndexerService {
        self.stopping.store(true, Ordering::Release);
        let _ = self.events.send(RuntimeEvent::ReaderClosed { lease: 0 });
        self.coordinator
            .take()
            .expect("live coordinator")
            .join()
            .expect("coordinator exit")
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.stopping.store(true, Ordering::Release);
        self.release.clear();
        if self.coordinator.is_some() {
            drop(self.stop());
        }
        let deadline = Instant::now() + Duration::from_secs(5);
        while self.active.load(Ordering::SeqCst) != 0 && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(1));
        }
    }
}

fn assert_two_volumes_are_serial(fail_first: bool) {
    let mut fixture = Fixture::new(fail_first);
    let first = fixture
        .started
        .recv_timeout(Duration::from_secs(4))
        .expect("first compaction admission");
    assert_eq!(first.mount, r"C:\");
    assert_eq!(
        first.first_mapping_owners, 2,
        "service and active worker own the first input"
    );
    fixture.fence();
    let concurrent = fixture.started.recv_timeout(Duration::from_millis(250));
    assert!(
        concurrent.is_err(),
        "another full compaction started before the first worker terminal/cleanup"
    );
    assert_eq!(
        fixture.maximum.load(Ordering::SeqCst),
        1,
        "global full-compaction concurrency"
    );

    let search = fixture.request(Message::IndexerCommand(IndexerCommand::Search(
        IndexSearchRequest {
            generation: 1,
            query: "keep".into(),
            limit: 20,
            context: IndexSearchContext {
                include_files: true,
                ..Default::default()
            },
        },
    )));
    let response = fixture
        .output
        .wait(|frame| frame.correlation_id == Some(search.id));
    let Message::IndexerEvent(IndexerEvent::SearchBatch(batch)) = response.message else {
        panic!("search batch during held compaction")
    };
    assert!(batch.complete);
    assert_eq!(
        batch.items.len(),
        2,
        "last-good search must not wait for compaction"
    );

    fixture.release[0].send(()).expect("release first worker");
    let second = fixture
        .started
        .recv_timeout(Duration::from_secs(4))
        .expect("waiting volume progresses after terminal");
    assert_eq!(second.mount, r"D:\");
    assert_eq!(
        second.first_mapping_owners,
        usize::from(fail_first),
        "first worker mapping must be released before next admission; failure keeps only last-good service owner"
    );
    assert_eq!(
        second.first_pair_exists, fail_first,
        "quiescent cleanup precedes next admission; failure preserves last-good pair"
    );
    assert_eq!(fixture.maximum.load(Ordering::SeqCst), 1);
    fixture.fence();
    assert!(
        fixture
            .started
            .recv_timeout(Duration::from_millis(150))
            .is_err(),
        "neither an active nor a backed-off volume starts a replacement worker"
    );
    fixture.release[1].send(()).expect("release second worker");
    fixture.wait_second_publication();
    let service = fixture.stop();
    assert_eq!(service.snapshots[1].next_usn, Some(110));
    assert!(!crate::usn::overlay_requires_compaction(
        &service.snapshots[1]
    ));
    assert_eq!(fixture.maximum.load(Ordering::SeqCst), 1);
    if fail_first {
        assert!(
            crate::usn::overlay_requires_compaction(&service.snapshots[0]),
            "failed volume remains pending"
        );
        assert_eq!(fixture.old_mapping.strong_count(), 1);
        assert!(service.status().last_error.is_some());
        assert_eq!(
            service.compaction_retries[&crate::normalized_mount_key(r"C:\")].failures,
            1
        );
        assert!(
            service
                .compaction_retry_at(r"C:\")
                .expect("retry remains scheduled")
                > Instant::now()
        );
        assert!(
            service.compaction_retry_at(r"D:\").is_none(),
            "other-volume success does not acknowledge the first failure"
        );
    } else {
        assert!(!crate::usn::overlay_requires_compaction(
            &service.snapshots[0]
        ));
        assert_eq!(fixture.old_mapping.strong_count(), 0);
        assert!(service.status().last_error.is_none());
    }
    for snapshot in service.snapshots.iter() {
        assert!(snapshot.visible_entry(11).is_some());
        assert!(
            snapshot
                .cache_path()
                .expect("current persisted base")
                .exists()
        );
    }
    assert!(fixture.directory.path().is_dir());
}

#[test]
fn serial_compaction_success_advances_next_volume_after_mapping_release_and_cleanup() {
    assert_two_volumes_are_serial(false);
}

#[test]
fn serial_compaction_failure_preserves_retry_and_advances_other_volume_after_input_release() {
    assert_two_volumes_are_serial(true);
}

#[test]
fn serial_compaction_reader_close_drains_already_discovered_volumes() {
    for fail_first in [false, true] {
        let mut fixture = Fixture::new(fail_first);
        let first = fixture
            .started
            .recv_timeout(Duration::from_secs(4))
            .expect("first compaction before reader close");
        assert_eq!(first.mount, r"C:\");
        fixture
            .events
            .send(RuntimeEvent::ReaderClosed { lease: 0 })
            .expect("standalone reader EOF");
        fixture.release[0]
            .send(())
            .expect("finish first compaction after EOF");
        let second = fixture
            .started
            .recv_timeout(Duration::from_secs(4))
            .expect("EOF must not discard the already discovered next volume");
        assert_eq!(second.mount, r"D:\");
        assert_eq!(second.first_mapping_owners, usize::from(fail_first));
        assert_eq!(second.first_pair_exists, fail_first);
        fixture.release[1]
            .send(())
            .expect("finish last discovered compaction");
        let service = fixture
            .coordinator
            .take()
            .expect("draining coordinator")
            .join()
            .expect("EOF completes after pending compactions");
        assert!(!crate::usn::overlay_requires_compaction(
            &service.snapshots[1]
        ));
        assert_eq!(
            crate::usn::overlay_requires_compaction(&service.snapshots[0]),
            fail_first
        );
        assert_eq!(service.status().last_error.is_some(), fail_first);
        assert_eq!(fixture.maximum.load(Ordering::SeqCst), 1);
    }
}
