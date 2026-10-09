use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::io::{BufRead, Write};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, mpsc};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use nxtools_finder::FileQuery;
use nxtools_ipc::{read_bounded_sync_frame, write_envelope_sync};
use nxtools_protocol::{
    Envelope, HelloAck, IndexDriveStatus, IndexPhase, IndexSearchBatch, IndexSearchItem,
    IndexSearchRequest, IndexSettings, IndexStatus, IndexerCommand, IndexerEvent, Message, Pong,
    ProtocolFault,
};
use rayon::prelude::*;
use sysinfo::Disks;
use tracing::{info, warn};

mod mft;
mod settings;
mod usn;

use mft::{MftEntryRef, MftSnapshot};

type SnapshotScanner = dyn Fn(&str) -> Result<MftSnapshot> + Send + Sync;
type JournalPoller = dyn Fn(&MftSnapshot) -> Result<usn::JournalPoll> + Send + Sync;
pub type ServiceCommandHandler = dyn Fn(&Envelope) -> Option<Vec<Envelope>> + Send + Sync;
const RUNTIME_QUEUE_CAPACITY: usize = 64;
const MAX_SEARCH_RESULTS: usize = 200;
const NAME_SEARCH_MATERIALIZATION_HEADROOM: usize = 4;
const NAME_SEARCH_POOL_BUFFERS_PER_THREAD: usize = 2;
const NAME_SEARCH_POOL_MAX_CAPACITY: usize =
    MAX_SEARCH_RESULTS * NAME_SEARCH_MATERIALIZATION_HEADROOM * 2;
const SEARCH_BATCH_SIZE: usize = 20;
const PREFERRED_DIRECTORY_SCORE_BONUS: i64 = 1_250;
const FAIL_CLOSED_INDEX_SCOPE: &str = "__NXTOOLS_INDEX_SETTINGS_RECOVERY_REQUIRED__";

struct BuildOutcome {
    status: IndexStatus,
    snapshots: Vec<MftSnapshot>,
    attempted_mount_points: Vec<String>,
    errors: Vec<(String, String)>,
}

struct PendingBuild {
    generation: u64,
    drives: Vec<IndexDriveStatus>,
    missing_only: bool,
}

fn merge_pending_build(mut current: PendingBuild, incoming: PendingBuild) -> PendingBuild {
    for drive in incoming.drives {
        if let Some(existing) = current.drives.iter_mut().find(|existing| {
            existing
                .mount_point
                .eq_ignore_ascii_case(&drive.mount_point)
        }) {
            *existing = drive;
        } else {
            current.drives.push(drive);
        }
    }
    current.generation = incoming.generation;
    current.missing_only &= incoming.missing_only;
    current
}

enum JournalMaintenance {
    NoChange,
    CursorAdvanced,
    StatusChanged,
    Updated,
    CompactionRequired {
        mount_points: Vec<String>,
    },
    RebuildRequired {
        mount_points: Vec<String>,
        compaction_mount_points: Vec<String>,
        reason: String,
    },
}

const JOURNAL_POLL_MIN_DELAY: Duration = Duration::from_secs(1);
const JOURNAL_POLL_MAX_IDLE_DELAY: Duration = Duration::from_secs(8);
const COMPACTION_RETRY_MIN_DELAY: Duration = Duration::from_secs(5);
const COMPACTION_RETRY_MAX_DELAY: Duration = Duration::from_secs(300);

#[derive(Clone)]
struct CompactionRetry {
    failures: u32,
    retry_at: Instant,
}

fn compaction_retry_delay(failures: u32) -> Duration {
    let multiplier = 1_u32 << failures.saturating_sub(1).min(6);
    (COMPACTION_RETRY_MIN_DELAY * multiplier).min(COMPACTION_RETRY_MAX_DELAY)
}

struct JournalPollSchedule {
    next_due: Instant,
    idle_delay: Duration,
}

impl JournalPollSchedule {
    fn new(now: Instant) -> Self {
        Self {
            next_due: now + JOURNAL_POLL_MIN_DELAY,
            idle_delay: JOURNAL_POLL_MIN_DELAY,
        }
    }

    fn remaining(&self, now: Instant) -> Duration {
        self.next_due.saturating_duration_since(now)
    }

    fn record(&mut self, maintenance: &JournalMaintenance, now: Instant) {
        match maintenance {
            JournalMaintenance::Updated
            | JournalMaintenance::CompactionRequired { .. }
            | JournalMaintenance::RebuildRequired { .. } => {
                self.idle_delay = JOURNAL_POLL_MIN_DELAY;
            }
            JournalMaintenance::NoChange
            | JournalMaintenance::CursorAdvanced
            | JournalMaintenance::StatusChanged => {
                self.idle_delay = (self.idle_delay * 2).min(JOURNAL_POLL_MAX_IDLE_DELAY);
            }
        }
        self.next_due = now + self.idle_delay;
    }

    fn reset(&mut self, now: Instant) {
        self.idle_delay = JOURNAL_POLL_MIN_DELAY;
        self.next_due = now + self.idle_delay;
    }

    fn defer_until(&mut self, retry_at: Instant) {
        // Compaction backoff is per volume. It must never postpone the shared USN poll,
        // otherwise one unhealthy volume would make healthy volumes stale.
        self.next_due = self.next_due.min(retry_at);
    }
}

enum RuntimeEvent {
    Request(Envelope),
    ReaderClosed,
    ReaderFailed(anyhow::Error),
    BuildCompleted {
        generation: u64,
        outcome: BuildOutcome,
    },
    BuildFailed {
        generation: u64,
        mount_points: Vec<String>,
        message: String,
    },
    SearchCompleted {
        task_id: u64,
        response: Envelope,
    },
    CompactionCompleted {
        mount_point: String,
        journal_id: Option<u64>,
        next_usn: Option<i64>,
        snapshot: MftSnapshot,
    },
    CompactionFailed {
        mount_point: String,
        message: String,
    },
}

struct SearchJob {
    task_id: u64,
    request: Envelope,
    snapshots: Arc<Vec<MftSnapshot>>,
    settings: IndexSettings,
}

#[derive(Default)]
struct SearchWorkerState {
    pending: Option<SearchJob>,
    closed: bool,
}

struct SearchExecutor {
    state: Arc<(Mutex<SearchWorkerState>, Condvar)>,
    active_task: Arc<AtomicU64>,
    worker: Option<thread::JoinHandle<()>>,
}

impl SearchExecutor {
    fn start(active_task: Arc<AtomicU64>, events: mpsc::SyncSender<RuntimeEvent>) -> Self {
        let state = Arc::new((Mutex::new(SearchWorkerState::default()), Condvar::new()));
        let worker_state = Arc::clone(&state);
        let worker_active_task = Arc::clone(&active_task);
        let worker = thread::spawn(move || {
            loop {
                let job = {
                    let (lock, ready) = &*worker_state;
                    let mut state = lock.lock().unwrap_or_else(|error| error.into_inner());
                    while state.pending.is_none() && !state.closed {
                        state = ready.wait(state).unwrap_or_else(|error| error.into_inner());
                    }
                    if state.closed {
                        return;
                    }
                    state.pending.take().expect("pending search job")
                };
                run_snapshot_search(job, &worker_active_task, &events);
            }
        });
        Self {
            state,
            active_task,
            worker: Some(worker),
        }
    }

    fn submit(&self, job: SearchJob) {
        let (lock, ready) = &*self.state;
        let mut state = lock.lock().unwrap_or_else(|error| error.into_inner());
        state.pending = Some(job);
        ready.notify_one();
    }
}

impl Drop for SearchExecutor {
    fn drop(&mut self) {
        self.active_task.store(u64::MAX, Ordering::Release);
        let (lock, ready) = &*self.state;
        let mut state = lock.lock().unwrap_or_else(|error| error.into_inner());
        state.closed = true;
        state.pending = None;
        ready.notify_one();
        drop(state);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

pub struct IndexerService {
    status: IndexStatus,
    build_errors: BTreeMap<String, String>,
    journal_errors: BTreeMap<String, String>,
    compaction_errors: BTreeMap<String, String>,
    compaction_retries: HashMap<String, CompactionRetry>,
    active_generation: Option<u64>,
    snapshots: Arc<Vec<MftSnapshot>>,
    settings: IndexSettings,
    persist_settings: bool,
}

struct VolumeDescriptor {
    mount_point: String,
    file_system: String,
}

impl VolumeDescriptor {
    fn new(mount_point: impl Into<String>, file_system: impl Into<String>) -> Self {
        Self {
            mount_point: mount_point.into(),
            file_system: file_system.into(),
        }
    }
}

fn system_volume_descriptors() -> Vec<VolumeDescriptor> {
    let disks = Disks::new_with_refreshed_list();
    disks
        .list()
        .iter()
        .map(|disk| {
            VolumeDescriptor::new(
                disk.mount_point().to_string_lossy(),
                disk.file_system().to_string_lossy(),
            )
        })
        .collect()
}

impl IndexerService {
    pub fn discover() -> Self {
        let mut service = Self::from_volume_descriptors(system_volume_descriptors());
        service.persist_settings = true;
        match settings::load_system_index_settings() {
            Ok(settings) => service.settings = normalize_index_settings(settings),
            Err(error) => {
                warn!(%error, "could not restore persisted index settings; indexing is disabled until the desktop resynchronizes settings");
                service.settings = fail_closed_index_settings();
            }
        }
        service.restore_cached_snapshots();
        service
    }

    fn from_volume_descriptors(volumes: impl IntoIterator<Item = VolumeDescriptor>) -> Self {
        let mut drives = volumes
            .into_iter()
            .filter(|volume| volume.file_system.eq_ignore_ascii_case("ntfs"))
            .map(|volume| IndexDriveStatus {
                mount_point: volume.mount_point,
                file_system: "NTFS".into(),
                indexed_items: 0,
                journal_id: None,
                next_usn: None,
            })
            .collect::<Vec<_>>();
        drives.sort_by(|left, right| left.mount_point.cmp(&right.mount_point));
        Self {
            status: IndexStatus {
                phase: IndexPhase::VolumesReady,
                indexed_items: 0,
                drives,
                last_error: None,
            },
            build_errors: BTreeMap::new(),
            journal_errors: BTreeMap::new(),
            compaction_errors: BTreeMap::new(),
            compaction_retries: HashMap::new(),
            active_generation: None,
            snapshots: Arc::new(Vec::new()),
            settings: IndexSettings::default(),
            persist_settings: false,
        }
    }

    pub fn rebuild_system(&mut self) {
        self.rebuild_with(mft::scan_volume_and_persist);
    }

    fn restore_cached_snapshots(&mut self) {
        let mut snapshots = Vec::with_capacity(self.status.drives.len());
        let mut missing = false;
        let settings = self.settings.clone();
        for drive in &mut self.status.drives {
            match mft::load_system_snapshot(&drive.mount_point) {
                Ok(Some(snapshot)) => {
                    drive.indexed_items = snapshot.visible_len() as u64;
                    drive.journal_id = snapshot.journal_id;
                    drive.next_usn = snapshot.next_usn;
                    snapshots.push(snapshot);
                }
                Ok(None) => missing |= index_settings_volume_enabled(&settings, &drive.mount_point),
                Err(error) => {
                    missing |= index_settings_volume_enabled(&settings, &drive.mount_point);
                    warn!(
                        mount_point = drive.mount_point,
                        %error,
                        "ignored invalid persisted MFT snapshot"
                    );
                }
            }
        }
        self.status.indexed_items = self
            .status
            .drives
            .iter()
            .map(|drive| drive.indexed_items)
            .sum();
        self.snapshots = Arc::new(snapshots);
        if !missing && !self.status.drives.is_empty() {
            self.status.phase = IndexPhase::Ready;
            trim_working_set_after_snapshot_restore();
        }
    }

    fn rebuild_with(&mut self, scan: impl Fn(&str) -> Result<MftSnapshot>) {
        self.apply_build(build_snapshots(self.status.drives.clone(), scan));
    }

    fn apply_build(&mut self, outcome: BuildOutcome) {
        let mut snapshots = self.snapshots.as_ref().clone();
        for snapshot in outcome.snapshots {
            if let Some(current) = snapshots.iter_mut().find(|current| {
                current
                    .mount_point
                    .eq_ignore_ascii_case(&snapshot.mount_point)
            }) {
                *current = snapshot;
            } else {
                snapshots.push(snapshot);
            }
        }
        for built_drive in &outcome.status.drives {
            let replacement = snapshots.iter().find(|snapshot| {
                snapshot
                    .mount_point
                    .eq_ignore_ascii_case(&built_drive.mount_point)
                    && snapshot.next_usn == built_drive.next_usn
            });
            let Some(replacement) = replacement else {
                continue;
            };
            if let Some(current_drive) = self.status.drives.iter_mut().find(|drive| {
                drive
                    .mount_point
                    .eq_ignore_ascii_case(&built_drive.mount_point)
            }) {
                current_drive.indexed_items = replacement.visible_len() as u64;
                current_drive.journal_id = replacement.journal_id;
                current_drive.next_usn = replacement.next_usn;
            } else {
                self.status.drives.push(built_drive.clone());
            }
        }
        self.snapshots = Arc::new(snapshots);
        self.status.phase = if self.snapshots.is_empty() {
            IndexPhase::VolumesReady
        } else {
            IndexPhase::Ready
        };
        self.status.indexed_items = self
            .status
            .drives
            .iter()
            .map(|drive| drive.indexed_items)
            .sum();
        for mount_point in outcome.attempted_mount_points {
            let mount_key = normalized_mount_key(&mount_point);
            self.build_errors.remove(&mount_key);
            self.journal_errors.remove(&mount_key);
            self.compaction_errors.remove(&mount_key);
            self.compaction_retries.remove(&mount_key);
        }
        for (mount_point, error) in outcome.errors {
            self.build_errors
                .insert(normalized_mount_key(&mount_point), error);
        }
        self.refresh_last_error();
    }

    fn rediscover_from_volume_descriptors(
        &mut self,
        volumes: impl IntoIterator<Item = VolumeDescriptor>,
    ) {
        let active_generation = self.active_generation;
        let previous_snapshots = Arc::clone(&self.snapshots);
        let mut discovered = Self::from_volume_descriptors(volumes);
        let retained_snapshots = previous_snapshots
            .iter()
            .filter(|snapshot| {
                discovered.status.drives.iter().any(|drive| {
                    drive
                        .mount_point
                        .eq_ignore_ascii_case(&snapshot.mount_point)
                })
            })
            .cloned()
            .collect::<Vec<_>>();
        for drive in &mut discovered.status.drives {
            if let Some(snapshot) = retained_snapshots.iter().find(|snapshot| {
                snapshot
                    .mount_point
                    .eq_ignore_ascii_case(&drive.mount_point)
            }) {
                drive.indexed_items = snapshot.visible_len() as u64;
                drive.journal_id = snapshot.journal_id;
                drive.next_usn = snapshot.next_usn;
            }
        }
        discovered.status.indexed_items = discovered
            .status
            .drives
            .iter()
            .map(|drive| drive.indexed_items)
            .sum();
        discovered.snapshots = Arc::new(retained_snapshots);
        discovered.active_generation = active_generation;
        discovered.settings = self.settings.clone();
        discovered.persist_settings = self.persist_settings;
        discovered.build_errors = self
            .build_errors
            .iter()
            .filter(|(mount_point, _)| {
                discovered
                    .status
                    .drives
                    .iter()
                    .any(|drive| normalized_mount_key(&drive.mount_point) == **mount_point)
            })
            .map(|(mount_point, error)| (mount_point.clone(), error.clone()))
            .collect();
        discovered.journal_errors = self
            .journal_errors
            .iter()
            .filter(|(mount_point, _)| {
                discovered
                    .status
                    .drives
                    .iter()
                    .any(|drive| normalized_mount_key(&drive.mount_point) == **mount_point)
            })
            .map(|(mount_point, error)| (mount_point.clone(), error.clone()))
            .collect();
        discovered.compaction_errors = self
            .compaction_errors
            .iter()
            .filter(|(mount_point, _)| {
                discovered
                    .status
                    .drives
                    .iter()
                    .any(|drive| normalized_mount_key(&drive.mount_point) == **mount_point)
            })
            .map(|(mount_point, error)| (mount_point.clone(), error.clone()))
            .collect();
        discovered.compaction_retries = self
            .compaction_retries
            .iter()
            .filter(|(mount_point, _)| {
                discovered
                    .status
                    .drives
                    .iter()
                    .any(|drive| normalized_mount_key(&drive.mount_point) == **mount_point)
            })
            .map(|(mount_point, retry)| (mount_point.clone(), retry.clone()))
            .collect();
        discovered.refresh_last_error();
        *self = discovered;
    }

    fn rediscover(&mut self) {
        self.rediscover_from_volume_descriptors(system_volume_descriptors());
    }

    fn record_build_failure(&mut self, mount_points: Vec<String>, message: String) {
        self.status.phase = if self.snapshots.is_empty() {
            IndexPhase::VolumesReady
        } else {
            IndexPhase::Ready
        };
        for mount_point in mount_points {
            self.build_errors.insert(
                normalized_mount_key(&mount_point),
                format!("{mount_point}: MFT snapshot worker failed: {message}"),
            );
        }
        self.refresh_last_error();
    }

    fn begin_snapshot_maintenance(&mut self) {
        self.status.phase = IndexPhase::VolumesReady;
        self.refresh_last_error();
    }

    fn refresh_last_error(&mut self) {
        let mut errors = self.build_errors.values().cloned().collect::<Vec<_>>();
        errors.extend(self.journal_errors.values().cloned());
        errors.extend(self.compaction_errors.values().cloned());
        self.status.last_error = (!errors.is_empty()).then(|| errors.join("; "));
    }

    fn build_drives(&self) -> Vec<IndexDriveStatus> {
        self.status
            .drives
            .iter()
            .filter(|drive| self.volume_is_enabled(&drive.mount_point))
            .cloned()
            .collect()
    }

    fn missing_build_drives(&self) -> Vec<IndexDriveStatus> {
        self.build_drives()
            .into_iter()
            .filter(|drive| {
                !self.snapshots.iter().any(|snapshot| {
                    snapshot
                        .mount_point
                        .eq_ignore_ascii_case(&drive.mount_point)
                })
            })
            .collect()
    }

    fn build_drives_for_mounts(&self, mount_points: &[String]) -> Vec<IndexDriveStatus> {
        self.build_drives()
            .into_iter()
            .filter(|drive| {
                mount_points
                    .iter()
                    .any(|mount_point| mount_point.eq_ignore_ascii_case(&drive.mount_point))
            })
            .collect()
    }

    fn snapshot_for_mount(&self, mount_point: &str) -> Option<MftSnapshot> {
        self.snapshots
            .iter()
            .find(|snapshot| snapshot.mount_point.eq_ignore_ascii_case(mount_point))
            .cloned()
    }

    fn enabled_volume_missing_snapshot(&self) -> bool {
        !self.missing_build_drives().is_empty()
    }

    fn apply_compaction(
        &mut self,
        mount_point: &str,
        journal_id: Option<u64>,
        next_usn: Option<i64>,
        compacted: MftSnapshot,
    ) -> bool {
        let mut snapshots = self.snapshots.as_ref().clone();
        let Some(current) = snapshots
            .iter_mut()
            .find(|snapshot| snapshot.mount_point.eq_ignore_ascii_case(mount_point))
        else {
            return false;
        };
        if current.journal_id != journal_id || current.next_usn != next_usn {
            return false;
        }
        *current = compacted;
        self.snapshots = Arc::new(snapshots);
        self.compaction_errors
            .remove(&normalized_mount_key(mount_point));
        self.compaction_retries
            .remove(&normalized_mount_key(mount_point));
        self.refresh_last_error();
        true
    }

    fn record_compaction_failure(&mut self, mount_point: &str, message: &str) {
        let mount_key = normalized_mount_key(mount_point);
        self.compaction_errors.insert(
            mount_key.clone(),
            format!("{mount_point} index compaction failed: {message}"),
        );
        let failures = self
            .compaction_retries
            .get(&mount_key)
            .map_or(1, |retry| retry.failures.saturating_add(1));
        self.compaction_retries.insert(
            mount_key,
            CompactionRetry {
                failures,
                retry_at: Instant::now() + compaction_retry_delay(failures),
            },
        );
        self.refresh_last_error();
    }

    fn clear_compaction_failure(&mut self, mount_point: &str) {
        let mount_key = normalized_mount_key(mount_point);
        self.compaction_errors.remove(&mount_key);
        self.compaction_retries.remove(&mount_key);
        self.refresh_last_error();
    }

    fn compaction_retry_at(&self, mount_point: &str) -> Option<Instant> {
        self.compaction_retries
            .get(&normalized_mount_key(mount_point))
            .map(|retry| retry.retry_at)
    }

    fn apply_settings(&mut self, settings: IndexSettings) {
        let settings = normalize_index_settings(settings);
        if self.settings == settings {
            return;
        }
        self.settings = settings;
        self.build_errors
            .retain(|mount_point, _| index_settings_volume_enabled(&self.settings, mount_point));
        self.journal_errors
            .retain(|mount_point, _| index_settings_volume_enabled(&self.settings, mount_point));
        self.compaction_errors
            .retain(|mount_point, _| index_settings_volume_enabled(&self.settings, mount_point));
        self.compaction_retries
            .retain(|mount_point, _| index_settings_volume_enabled(&self.settings, mount_point));
        self.refresh_last_error();
        if self.persist_settings
            && let Err(error) = settings::persist_system_index_settings(&self.settings)
        {
            warn!(%error, "could not persist index settings");
        }
    }

    fn volume_is_enabled(&self, mount_point: &str) -> bool {
        index_settings_volume_enabled(&self.settings, mount_point)
    }

    fn poll_journals(&mut self, poll: &JournalPoller) -> JournalMaintenance {
        if self.status.phase != IndexPhase::Ready || self.snapshots.is_empty() {
            return JournalMaintenance::NoChange;
        }
        let mut snapshots = self.snapshots.as_ref().clone();
        let mut published = false;
        let mut content_changed = false;
        let mut rebuild_mounts = Vec::new();
        let mut rebuild_reasons = Vec::new();
        let previous_journal_errors = self.journal_errors.clone();
        for snapshot in &mut snapshots {
            if !self.volume_is_enabled(&snapshot.mount_point) {
                continue;
            }
            let mount_key = normalized_mount_key(&snapshot.mount_point);
            match poll(snapshot) {
                Ok(usn::JournalPoll::NoChange) => {
                    self.journal_errors.remove(&mount_key);
                }
                Ok(usn::JournalPoll::CursorAdvanced(updated)) => {
                    if let Err(error) = mft::persist_system_delta(&updated) {
                        warn!(
                            mount_point = updated.mount_point,
                            %error,
                            "failed to persist incremental index cursor"
                        );
                    }
                    *snapshot = updated;
                    self.journal_errors.remove(&mount_key);
                    self.compaction_errors.remove(&mount_key);
                    self.compaction_retries.remove(&mount_key);
                    published = true;
                }
                Ok(usn::JournalPoll::Updated(updated)) => {
                    if let Err(error) = mft::persist_system_delta(&updated) {
                        warn!(
                            mount_point = updated.mount_point,
                            %error,
                            "failed to persist incremental index delta"
                        );
                    }
                    *snapshot = updated;
                    self.journal_errors.remove(&mount_key);
                    self.compaction_errors.remove(&mount_key);
                    self.compaction_retries.remove(&mount_key);
                    published = true;
                    content_changed = true;
                }
                Ok(usn::JournalPoll::RebuildRequired(reason)) => {
                    rebuild_mounts.push(snapshot.mount_point.clone());
                    let message = format!("{}: {reason}", snapshot.mount_point);
                    rebuild_reasons.push(message);
                    // A rebuild request is a bounded recovery decision, not a failed index. The
                    // last good snapshot remains searchable while the replacement is built. Keep
                    // the reason in the maintenance event/log, and reserve last_error for an
                    // actual poll, compaction, or snapshot-worker failure.
                    self.journal_errors.remove(&mount_key);
                }
                Err(error) => {
                    self.journal_errors.insert(
                        mount_key,
                        format!("{} USN polling failed: {error}", snapshot.mount_point),
                    );
                }
            }
        }
        if published {
            for drive in &mut self.status.drives {
                if let Some(snapshot) = snapshots.iter().find(|snapshot| {
                    snapshot
                        .mount_point
                        .eq_ignore_ascii_case(&drive.mount_point)
                }) {
                    drive.indexed_items = snapshot.visible_len() as u64;
                    drive.journal_id = snapshot.journal_id;
                    drive.next_usn = snapshot.next_usn;
                }
            }
            self.status.indexed_items = self
                .status
                .drives
                .iter()
                .map(|drive| drive.indexed_items)
                .sum();
            self.snapshots = Arc::new(snapshots);
        }
        let compaction_mounts = self
            .snapshots
            .iter()
            .filter(|snapshot| {
                self.volume_is_enabled(&snapshot.mount_point)
                    && usn::overlay_requires_compaction(snapshot)
                    && !rebuild_mounts
                        .iter()
                        .any(|mount_point| mount_point.eq_ignore_ascii_case(&snapshot.mount_point))
            })
            .map(|snapshot| snapshot.mount_point.clone())
            .collect();
        let error_changed = self.journal_errors != previous_journal_errors;
        self.refresh_last_error();
        if !rebuild_mounts.is_empty() {
            return JournalMaintenance::RebuildRequired {
                mount_points: rebuild_mounts,
                compaction_mount_points: compaction_mounts,
                reason: rebuild_reasons.join("; "),
            };
        }
        if !compaction_mounts.is_empty() {
            JournalMaintenance::CompactionRequired {
                mount_points: compaction_mounts,
            }
        } else if content_changed {
            JournalMaintenance::Updated
        } else if error_changed {
            JournalMaintenance::StatusChanged
        } else if published {
            JournalMaintenance::CursorAdvanced
        } else {
            JournalMaintenance::NoChange
        }
    }

    pub fn status(&self) -> &IndexStatus {
        &self.status
    }

    fn begin_client_session(&mut self) {
        self.active_generation = None;
    }

    pub fn handle(&mut self, request: Envelope) -> Vec<Envelope> {
        match &request.message {
            Message::Hello(hello) => vec![
                Envelope::reply_to(
                    &request,
                    Message::HelloAck(HelloAck {
                        server_name: "nxtools_indexer".into(),
                        server_version: env!("CARGO_PKG_VERSION").into(),
                        accepted_capabilities: hello
                            .capabilities
                            .iter()
                            .filter(|capability| {
                                matches!(capability.as_str(), "index_status" | "file_search")
                            })
                            .cloned()
                            .collect(),
                    }),
                ),
                Envelope::new(Message::IndexerEvent(IndexerEvent::Status(
                    self.status.clone(),
                ))),
            ],
            Message::IndexerCommand(IndexerCommand::Search(search)) => {
                if !self.accept_search_generation(search.generation) {
                    return Vec::new();
                }
                let started = Instant::now();
                let items = self.search_snapshot(search);
                vec![Envelope::reply_to(
                    &request,
                    Message::IndexerEvent(IndexerEvent::SearchBatch(IndexSearchBatch {
                        generation: search.generation,
                        items,
                        complete: true,
                        elapsed_us: started.elapsed().as_micros() as u64,
                    })),
                )]
            }
            Message::IndexerCommand(IndexerCommand::UpdateSettings(settings)) => {
                self.apply_settings(settings.clone());
                vec![Envelope::reply_to(
                    &request,
                    Message::IndexerEvent(IndexerEvent::Status(self.status.clone())),
                )]
            }
            Message::IndexerCommand(
                IndexerCommand::RefreshVolumes | IndexerCommand::RebuildIndex,
            ) => vec![Envelope::reply_to(
                &request,
                Message::IndexerEvent(IndexerEvent::Status(self.status.clone())),
            )],
            Message::IndexerCommand(IndexerCommand::EnsureElevatedDirectoryHelper(_)) => {
                vec![Envelope::reply_to(
                    &request,
                    Message::Error(ProtocolFault {
                        code: "elevated_helper_requires_service".into(),
                        message:
                            "the elevated directory helper is available only from the Windows service"
                                .into(),
                    }),
                )]
            }
            Message::Ping(ping) => vec![Envelope::reply_to(
                &request,
                Message::Pong(Pong {
                    sent_at_unix_ms: ping.sent_at_unix_ms,
                }),
            )],
            Message::HelloAck(_)
            | Message::Pong(_)
            | Message::PlatformEvent(_)
            | Message::PlatformCommand(_)
            | Message::PlatformCommandResult(_)
            | Message::IndexerEvent(_)
            | Message::Error(_) => vec![Envelope::reply_to(
                &request,
                Message::Error(ProtocolFault {
                    code: "invalid_indexer_message".into(),
                    message: "message type is not accepted by the indexer service".into(),
                }),
            )],
        }
    }

    fn accept_search_generation(&mut self, generation: u64) -> bool {
        if self
            .active_generation
            .is_some_and(|active| generation < active)
        {
            return false;
        }
        self.active_generation = Some(generation);
        true
    }

    fn search_snapshot(&self, request: &IndexSearchRequest) -> Vec<IndexSearchItem> {
        self.search_snapshot_while(request, || true)
            .unwrap_or_default()
    }

    fn search_snapshot_while(
        &self,
        request: &IndexSearchRequest,
        is_current: impl Fn() -> bool + Sync,
    ) -> Option<Vec<IndexSearchItem>> {
        search_snapshots_while(&self.snapshots, &self.settings, request, is_current)
    }
}

fn search_snapshots_while(
    snapshots: &[MftSnapshot],
    settings: &IndexSettings,
    request: &IndexSearchRequest,
    is_current: impl Fn() -> bool + Sync,
) -> Option<Vec<IndexSearchItem>> {
    if request.limit == 0 {
        return Some(Vec::new());
    }
    if request.query.trim().is_empty() {
        return Some(Vec::new());
    }
    let query = FileQuery::parse(&request.query);
    let directory = request
        .context
        .directory
        .as_deref()
        .map(|path| path.to_lowercase());
    let result_limit = request.limit.min(MAX_SEARCH_RESULTS);
    if query.is_name_only() && directory.is_none() {
        return search_name_only_snapshots_while(
            snapshots,
            settings,
            request,
            &query,
            result_limit,
            &is_current,
        );
    }
    search_full_path_snapshots_while(
        snapshots,
        settings,
        request,
        &query,
        directory.as_deref(),
        result_limit,
        &is_current,
    )
}

fn search_full_path_snapshots_while(
    snapshots: &[MftSnapshot],
    settings: &IndexSettings,
    request: &IndexSearchRequest,
    query: &FileQuery,
    directory: Option<&str>,
    result_limit: usize,
    is_current: &(impl Fn() -> bool + Sync),
) -> Option<Vec<IndexSearchItem>> {
    let cancelled = AtomicBool::new(false);
    let mut matches = snapshots
        .par_iter()
        .flat_map(|snapshot| {
            (0..snapshot.base_len())
                .into_par_iter()
                .with_min_len(8_192)
                .fold(
                    || Vec::with_capacity(result_limit.saturating_mul(2)),
                    |mut local, index| {
                        if cancelled.load(Ordering::Acquire) {
                            return local;
                        }
                        if !is_current() {
                            cancelled.store(true, Ordering::Release);
                            return local;
                        }
                        let Some(entry) = snapshot.base_entry(index) else {
                            return local;
                        };
                        if entry.identity.is_some_and(|identity| {
                            snapshot.overlay.deleted.contains(&identity.file_reference)
                                || snapshot
                                    .overlay
                                    .replacements
                                    .contains_key(&identity.file_reference)
                        }) {
                            return local;
                        }
                        if let Some(item) =
                            match_search_candidate(entry, settings, request, query, directory)
                        {
                            local.push(item);
                            prune_search_candidates(&mut local, result_limit);
                        }
                        local
                    },
                )
        })
        .reduce(Vec::new, |mut left, right| {
            left.extend(right);
            prune_search_candidates(&mut left, result_limit);
            left
        });
    for snapshot in snapshots {
        for entry in snapshot.overlay.replacements.values() {
            if !is_current() {
                cancelled.store(true, Ordering::Release);
                break;
            }
            if let Some(item) =
                match_search_candidate(entry.as_ref(), settings, request, query, directory)
            {
                matches.push(item);
                prune_search_candidates(&mut matches, result_limit);
            }
        }
    }
    if cancelled.load(Ordering::Acquire) || !is_current() {
        return None;
    }
    sort_search_candidates(&mut matches);
    matches.truncate(result_limit);
    Some(
        matches
            .into_iter()
            .map(SearchCandidate::into_item)
            .collect(),
    )
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct NameSearchCandidate {
    snapshot_index: usize,
    row_index: usize,
    score: i64,
}

thread_local! {
    static NAME_SEARCH_BUFFER_POOL: RefCell<Vec<Vec<NameSearchCandidate>>> =
        const { RefCell::new(Vec::new()) };
}

fn rent_name_search_buffer(minimum_capacity: usize) -> Vec<NameSearchCandidate> {
    NAME_SEARCH_BUFFER_POOL.with(|pool| {
        let mut buffer = pool.borrow_mut().pop().unwrap_or_default();
        buffer.clear();
        if buffer.capacity() < minimum_capacity {
            buffer.reserve(minimum_capacity - buffer.capacity());
        }
        buffer
    })
}

fn recycle_name_search_buffer(mut buffer: Vec<NameSearchCandidate>) {
    buffer.clear();
    if buffer.capacity() > NAME_SEARCH_POOL_MAX_CAPACITY {
        return;
    }
    NAME_SEARCH_BUFFER_POOL.with(|pool| {
        let mut pool = pool.borrow_mut();
        if pool.len() < NAME_SEARCH_POOL_BUFFERS_PER_THREAD {
            pool.push(buffer);
        }
    });
}

fn sort_name_search_candidates(matches: &mut [NameSearchCandidate]) {
    matches.sort_unstable_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then_with(|| left.snapshot_index.cmp(&right.snapshot_index))
            .then_with(|| left.row_index.cmp(&right.row_index))
    });
}

fn prune_name_search_candidates(matches: &mut Vec<NameSearchCandidate>, limit: usize) {
    if matches.len() >= limit.saturating_mul(2) {
        sort_name_search_candidates(matches);
        matches.truncate(limit);
    }
}

fn search_name_only_snapshots_while(
    snapshots: &[MftSnapshot],
    settings: &IndexSettings,
    request: &IndexSearchRequest,
    query: &FileQuery,
    result_limit: usize,
    is_current: &(impl Fn() -> bool + Sync),
) -> Option<Vec<IndexSearchItem>> {
    let cancelled = AtomicBool::new(false);
    let candidate_limit = result_limit
        .saturating_mul(NAME_SEARCH_MATERIALIZATION_HEADROOM)
        .max(result_limit);
    let initial_capacity = candidate_limit.min(128);
    let mut name_matches = snapshots
        .par_iter()
        .enumerate()
        .filter(|(_, snapshot)| index_settings_volume_enabled(settings, &snapshot.mount_point))
        .flat_map(|(snapshot_index, snapshot)| {
            let cancelled = &cancelled;
            (0..snapshot.base_len())
                .into_par_iter()
                .with_min_len(8_192)
                .fold(
                    || rent_name_search_buffer(initial_capacity),
                    move |mut local, row_index| {
                        if cancelled.load(Ordering::Acquire) {
                            return local;
                        }
                        if !is_current() {
                            cancelled.store(true, Ordering::Release);
                            return local;
                        }
                        let Some(name_mask) = snapshot.base_name_mask(row_index) else {
                            return local;
                        };
                        if !query.name_mask_may_match(name_mask) {
                            return local;
                        }
                        let Some(name) = snapshot.base_name(row_index) else {
                            return local;
                        };
                        let Some(score) = query.score_name(name) else {
                            return local;
                        };
                        local.push(NameSearchCandidate {
                            snapshot_index,
                            row_index,
                            score,
                        });
                        prune_name_search_candidates(&mut local, candidate_limit);
                        local
                    },
                )
        })
        .reduce(
            || rent_name_search_buffer(initial_capacity),
            |mut left, mut right| {
                left.append(&mut right);
                recycle_name_search_buffer(right);
                prune_name_search_candidates(&mut left, candidate_limit);
                left
            },
        );

    if cancelled.load(Ordering::Acquire) || !is_current() {
        recycle_name_search_buffer(name_matches);
        return None;
    }
    sort_name_search_candidates(&mut name_matches);
    name_matches.truncate(candidate_limit);
    let name_candidates_truncated = name_matches.len() == candidate_limit;

    // Full paths live in the cold snapshot section. Materialize them only after the
    // hot name scan has reduced millions of rows to a small, ranked candidate set.
    let mut matches = Vec::with_capacity(result_limit.saturating_mul(2));
    for candidate in &name_matches {
        if !is_current() {
            cancelled.store(true, Ordering::Release);
            break;
        }
        let snapshot = &snapshots[candidate.snapshot_index];
        let Some(entry) = snapshot.base_entry(candidate.row_index) else {
            continue;
        };
        if entry.identity.is_some_and(|identity| {
            snapshot.overlay.deleted.contains(&identity.file_reference)
                || snapshot
                    .overlay
                    .replacements
                    .contains_key(&identity.file_reference)
        }) {
            continue;
        }
        if let Some(item) = match_search_candidate_with_score(
            entry,
            settings,
            request,
            query,
            None,
            candidate.score,
        ) {
            matches.push(item);
        }
    }
    recycle_name_search_buffer(name_matches);

    for snapshot in snapshots {
        if !index_settings_volume_enabled(settings, &snapshot.mount_point) {
            continue;
        }
        for entry in snapshot.overlay.replacements.values() {
            if !is_current() {
                cancelled.store(true, Ordering::Release);
                break;
            }
            if let Some(item) =
                match_search_candidate(entry.as_ref(), settings, request, query, None)
            {
                matches.push(item);
                prune_search_candidates(&mut matches, result_limit);
            }
        }
    }
    if cancelled.load(Ordering::Acquire) || !is_current() {
        return None;
    }
    if name_candidates_truncated && matches.len() < result_limit {
        // Filtering requires cold path/type metadata. If the fast hot-name window was
        // exhausted before it produced a full page, fall back to the complete scan so
        // exclusions, requested kinds, and overlay tombstones cannot silently underfill.
        return search_full_path_snapshots_while(
            snapshots,
            settings,
            request,
            query,
            None,
            result_limit,
            is_current,
        );
    }
    sort_search_candidates(&mut matches);
    matches.truncate(result_limit);
    Some(
        matches
            .into_iter()
            .map(SearchCandidate::into_item)
            .collect(),
    )
}

#[derive(Clone, Copy)]
struct SearchCandidate<'a> {
    entry: MftEntryRef<'a>,
    score: i64,
}

impl SearchCandidate<'_> {
    fn into_item(self) -> IndexSearchItem {
        IndexSearchItem {
            path: self.entry.path.to_owned(),
            name: self.entry.name.to_owned(),
            is_directory: self.entry.is_directory,
            size_bytes: self.entry.size_bytes,
            modified_at_unix_ms: self.entry.modified_at_unix_ms,
            score: self.score,
        }
    }
}

fn match_search_candidate<'a>(
    entry: MftEntryRef<'a>,
    settings: &IndexSettings,
    request: &IndexSearchRequest,
    query: &FileQuery,
    directory: Option<&str>,
) -> Option<SearchCandidate<'a>> {
    let score = query.score(entry.name, entry.path)?;
    match_search_candidate_with_score(entry, settings, request, query, directory, score)
}

fn match_search_candidate_with_score<'a>(
    entry: MftEntryRef<'a>,
    settings: &IndexSettings,
    request: &IndexSearchRequest,
    query: &FileQuery,
    directory: Option<&str>,
    score: i64,
) -> Option<SearchCandidate<'a>> {
    if !index_settings_include_path(settings, entry.path, query.bypasses_exclusions())
        || (entry.is_directory && !request.context.include_directories)
        || (!entry.is_directory && !request.context.include_files)
        || directory.is_some_and(|directory| !case_insensitive_starts_with(entry.path, directory))
    {
        return None;
    }
    Some(SearchCandidate {
        entry,
        score: score.saturating_add(preferred_directory_score(
            entry.path,
            request.context.preferred_directory.as_deref(),
        )),
    })
}

fn preferred_directory_score(path: &str, preferred_directory: Option<&str>) -> i64 {
    let Some(preferred_directory) = preferred_directory
        .map(|directory| directory.trim_end_matches(['\\', '/']))
        .filter(|directory| !directory.is_empty())
    else {
        return 0;
    };
    let parent = path
        .trim_end_matches(['\\', '/'])
        .rsplit_once(['\\', '/'])
        .map(|(parent, _)| parent);
    if parent.is_some_and(|parent| parent.eq_ignore_ascii_case(preferred_directory)) {
        PREFERRED_DIRECTORY_SCORE_BONUS
    } else {
        0
    }
}

fn normalize_index_settings(mut settings: IndexSettings) -> IndexSettings {
    settings.indexed_volumes = normalize_unique_paths(settings.indexed_volumes);
    settings.excluded_paths = normalize_unique_paths(settings.excluded_paths);
    settings
}

fn fail_closed_index_settings() -> IndexSettings {
    IndexSettings {
        indexed_volumes: vec![FAIL_CLOSED_INDEX_SCOPE.into()],
        excluded_paths: Vec::new(),
    }
}

fn normalized_mount_key(mount_point: &str) -> String {
    mount_point.trim_end_matches('\\').to_ascii_lowercase()
}

fn index_settings_volume_enabled(settings: &IndexSettings, mount_point: &str) -> bool {
    settings.indexed_volumes.is_empty()
        || settings
            .indexed_volumes
            .iter()
            .any(|volume| paths_equal(volume, mount_point))
}

fn normalize_unique_paths(paths: Vec<String>) -> Vec<String> {
    let mut normalized = Vec::<String>::new();
    for path in paths {
        let mut path = path.trim().trim_matches('"').replace('/', "\\");
        while path.len() > 3 && path.ends_with('\\') {
            path.pop();
        }
        if path.is_empty()
            || normalized
                .iter()
                .any(|existing| existing.eq_ignore_ascii_case(&path))
        {
            continue;
        }
        normalized.push(path);
    }
    normalized
}

fn index_settings_include_path(
    settings: &IndexSettings,
    path: &str,
    bypass_exclusions: bool,
) -> bool {
    let volume_allowed = settings.indexed_volumes.is_empty()
        || settings
            .indexed_volumes
            .iter()
            .any(|volume| path_is_within(path, volume));
    volume_allowed
        && (bypass_exclusions
            || !settings
                .excluded_paths
                .iter()
                .any(|excluded| path_is_within(path, excluded)))
}

fn paths_equal(left: &str, right: &str) -> bool {
    left.trim_end_matches(['\\', '/'])
        .eq_ignore_ascii_case(right.trim_end_matches(['\\', '/']))
}

fn path_is_within(path: &str, root: &str) -> bool {
    let path = normalized_path_separators(path);
    let root = normalized_path_separators(root);
    if paths_equal(path.as_ref(), root.as_ref()) {
        return true;
    }
    let root = root.trim_end_matches('\\');
    path.get(..root.len())
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case(root))
        && path.as_bytes().get(root.len()).copied() == Some(b'\\')
}

fn normalized_path_separators(path: &str) -> Cow<'_, str> {
    if path.contains('/') {
        Cow::Owned(path.replace('/', "\\"))
    } else {
        Cow::Borrowed(path)
    }
}

fn prune_search_candidates(matches: &mut Vec<SearchCandidate<'_>>, result_limit: usize) {
    if matches.len() >= result_limit.saturating_mul(2) {
        sort_search_candidates(matches);
        matches.truncate(result_limit);
    }
}

fn case_insensitive_starts_with(value: &str, lower_query: &str) -> bool {
    if lower_query.is_ascii() {
        value
            .as_bytes()
            .get(..lower_query.len())
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case(lower_query.as_bytes()))
    } else {
        value.to_lowercase().starts_with(lower_query)
    }
}

fn sort_search_candidates(matches: &mut [SearchCandidate<'_>]) {
    matches.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then_with(|| left.entry.path.cmp(right.entry.path))
    });
}

fn build_snapshots(
    mut drives: Vec<IndexDriveStatus>,
    scan: impl Fn(&str) -> Result<MftSnapshot>,
) -> BuildOutcome {
    let mut snapshots = Vec::with_capacity(drives.len());
    let attempted_mount_points = drives
        .iter()
        .map(|drive| drive.mount_point.clone())
        .collect::<Vec<_>>();
    let mut errors = Vec::<(String, String)>::new();
    for drive in &mut drives {
        drive.indexed_items = 0;
        drive.journal_id = None;
        drive.next_usn = None;
        match scan(&drive.mount_point) {
            Ok(snapshot) => {
                if !snapshot
                    .mount_point
                    .eq_ignore_ascii_case(&drive.mount_point)
                {
                    errors.push((
                        drive.mount_point.clone(),
                        format!(
                            "{}: snapshot was returned for {}",
                            drive.mount_point, snapshot.mount_point
                        ),
                    ));
                    continue;
                }
                drive.indexed_items = snapshot.visible_len() as u64;
                drive.journal_id = snapshot.journal_id;
                drive.next_usn = snapshot.next_usn;
                snapshots.push(snapshot);
            }
            Err(error) => errors.push((
                drive.mount_point.clone(),
                format!("{}: {error}", drive.mount_point),
            )),
        }
    }
    let indexed_items = drives.iter().map(|drive| drive.indexed_items).sum();
    BuildOutcome {
        snapshots,
        status: IndexStatus {
            phase: IndexPhase::Ready,
            indexed_items,
            drives,
            last_error: (!errors.is_empty()).then(|| {
                errors
                    .iter()
                    .map(|(_, error)| error.as_str())
                    .collect::<Vec<_>>()
                    .join("; ")
            }),
        },
        attempted_mount_points,
        errors,
    }
}

pub fn serve<R, W>(reader: R, writer: W, service: IndexerService) -> Result<()>
where
    R: BufRead + Send + 'static,
    W: Write,
{
    let mut service = service;
    serve_session_with_sources(
        reader,
        writer,
        &mut service,
        Arc::new(mft::scan_volume_and_persist),
        Arc::new(usn::poll_snapshot),
        None,
    )
}

pub fn serve_session<R, W>(reader: R, writer: W, service: &mut IndexerService) -> Result<()>
where
    R: BufRead + Send + 'static,
    W: Write,
{
    serve_session_with_sources(
        reader,
        writer,
        service,
        Arc::new(mft::scan_volume_and_persist),
        Arc::new(usn::poll_snapshot),
        None,
    )
}

pub fn serve_session_with_command_handler<R, W>(
    reader: R,
    writer: W,
    service: &mut IndexerService,
    command_handler: Arc<ServiceCommandHandler>,
) -> Result<()>
where
    R: BufRead + Send + 'static,
    W: Write,
{
    serve_session_with_sources(
        reader,
        writer,
        service,
        Arc::new(mft::scan_volume_and_persist),
        Arc::new(usn::poll_snapshot),
        Some(command_handler),
    )
}

/// Service-host entry: Hello is read before acquiring the exclusive indexer state lock.
pub fn serve_admitted_session<R, W>(
    reader: R,
    writer: W,
    service: &mut IndexerService,
    handshake: Envelope,
    command_handler: Arc<ServiceCommandHandler>,
) -> Result<()>
where
    R: BufRead + Send + 'static,
    W: Write,
{
    serve_session_with_sources_and_handshake(
        reader,
        writer,
        service,
        Arc::new(mft::scan_volume_and_persist),
        Arc::new(usn::poll_snapshot),
        Some(command_handler),
        Some(handshake),
    )
}

#[cfg(test)]
fn serve_with_scanner<R, W>(
    reader: R,
    writer: W,
    mut service: IndexerService,
    scanner: Arc<SnapshotScanner>,
) -> Result<()>
where
    R: BufRead + Send + 'static,
    W: Write,
{
    serve_session_with_scanner(reader, writer, &mut service, scanner)
}

#[cfg(test)]
fn serve_session_with_scanner<R, W>(
    reader: R,
    writer: W,
    service: &mut IndexerService,
    scanner: Arc<SnapshotScanner>,
) -> Result<()>
where
    R: BufRead + Send + 'static,
    W: Write,
{
    serve_session_with_sources(
        reader,
        writer,
        service,
        scanner,
        Arc::new(|_| Ok(usn::JournalPoll::NoChange)),
        None,
    )
}

fn serve_session_with_sources<R, W>(
    reader: R,
    writer: W,
    service: &mut IndexerService,
    scanner: Arc<SnapshotScanner>,
    journal_poller: Arc<JournalPoller>,
    command_handler: Option<Arc<ServiceCommandHandler>>,
) -> Result<()>
where
    R: BufRead + Send + 'static,
    W: Write,
{
    serve_session_with_sources_and_handshake(
        reader,
        writer,
        service,
        scanner,
        journal_poller,
        command_handler,
        None,
    )
}

fn serve_session_with_sources_and_handshake<R, W>(
    mut reader: R,
    mut writer: W,
    service: &mut IndexerService,
    scanner: Arc<SnapshotScanner>,
    journal_poller: Arc<JournalPoller>,
    command_handler: Option<Arc<ServiceCommandHandler>>,
    handshake: Option<Envelope>,
) -> Result<()>
where
    R: BufRead + Send + 'static,
    W: Write,
{
    let handshake = match handshake {
        Some(handshake) => handshake,
        None => {
            let Some(frame) = read_bounded_sync_frame(&mut reader)? else {
                bail!("desktop disconnected before indexer handshake");
            };
            Envelope::decode_line(&frame).context("invalid indexer handshake")?
        }
    };
    if !matches!(handshake.message, Message::Hello(_)) {
        bail!("desktop did not begin the indexer session with hello");
    }
    service.begin_client_session();
    write_responses(&mut writer, service.handle(handshake))?;

    let (events, runtime) = mpsc::sync_channel(RUNTIME_QUEUE_CAPACITY);
    let reader_events = events.clone();
    thread::spawn(move || {
        loop {
            match read_bounded_sync_frame(&mut reader) {
                Ok(Some(frame)) => match Envelope::decode_line(&frame) {
                    Ok(request) => {
                        if reader_events.send(RuntimeEvent::Request(request)).is_err() {
                            break;
                        }
                    }
                    Err(error) => {
                        let _ = reader_events.send(RuntimeEvent::ReaderFailed(
                            anyhow::Error::new(error).context("invalid indexer request"),
                        ));
                        break;
                    }
                },
                Ok(None) => {
                    let _ = reader_events.send(RuntimeEvent::ReaderClosed);
                    break;
                }
                Err(error) => {
                    let _ = reader_events.send(RuntimeEvent::ReaderFailed(error));
                    break;
                }
            }
        }
    });

    let mut next_build_generation = 1_u64;
    let initial_build_drives = service.missing_build_drives();
    let mut active_build = (!initial_build_drives.is_empty()).then_some(next_build_generation);
    let mut pending_build = None;
    let active_search_task = Arc::new(AtomicU64::new(0));
    let search_executor = SearchExecutor::start(Arc::clone(&active_search_task), events.clone());
    let mut active_compactions = HashSet::<String>::new();
    let mut next_search_task = 0_u64;
    if active_build.is_some() {
        spawn_snapshot_build(
            next_build_generation,
            initial_build_drives,
            Arc::clone(&scanner),
            events.clone(),
        );
    }
    let mut reader_closed = false;
    let mut journal_schedule = JournalPollSchedule::new(Instant::now());

    loop {
        if active_build.is_none()
            && active_compactions.is_empty()
            && journal_schedule.remaining(Instant::now()).is_zero()
        {
            let maintenance = service.poll_journals(journal_poller.as_ref());
            journal_schedule.record(&maintenance, Instant::now());
            match maintenance {
                JournalMaintenance::NoChange | JournalMaintenance::CursorAdvanced => {}
                JournalMaintenance::StatusChanged | JournalMaintenance::Updated => {
                    write_responses(
                        &mut writer,
                        vec![Envelope::new(Message::IndexerEvent(IndexerEvent::Status(
                            service.status().clone(),
                        )))],
                    )?;
                }
                JournalMaintenance::CompactionRequired { mount_points } => {
                    write_responses(
                        &mut writer,
                        vec![Envelope::new(Message::IndexerEvent(IndexerEvent::Status(
                            service.status().clone(),
                        )))],
                    )?;
                    if let Some(retry_at) = schedule_snapshot_compactions(
                        mount_points,
                        service,
                        &mut active_compactions,
                        &events,
                    ) {
                        journal_schedule.defer_until(retry_at);
                    }
                }
                JournalMaintenance::RebuildRequired {
                    mount_points,
                    compaction_mount_points,
                    reason,
                } => {
                    if let Some(retry_at) = schedule_snapshot_compactions(
                        compaction_mount_points,
                        service,
                        &mut active_compactions,
                        &events,
                    ) {
                        journal_schedule.defer_until(retry_at);
                    }
                    next_build_generation = next_build_generation.saturating_add(1);
                    let drives = service.build_drives_for_mounts(&mount_points);
                    if drives.is_empty() {
                        continue;
                    }
                    active_build = Some(next_build_generation);
                    service.begin_snapshot_maintenance();
                    info!(%reason, "rebuilding MFT snapshot while the last good index remains searchable");
                    write_responses(
                        &mut writer,
                        vec![Envelope::new(Message::IndexerEvent(IndexerEvent::Status(
                            service.status().clone(),
                        )))],
                    )?;
                    spawn_snapshot_build(
                        next_build_generation,
                        drives,
                        Arc::clone(&scanner),
                        events.clone(),
                    );
                }
            }
            continue;
        }
        let wait = if active_build.is_some() {
            JOURNAL_POLL_MIN_DELAY
        } else {
            journal_schedule.remaining(Instant::now())
        };
        let event = match runtime.recv_timeout(wait) {
            Ok(event) => event,
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        };
        match event {
            RuntimeEvent::Request(request) => {
                if let Some(responses) = command_handler
                    .as_ref()
                    .and_then(|handler| handler(&request))
                {
                    write_responses(&mut writer, responses)?;
                    continue;
                }
                if let Message::IndexerCommand(IndexerCommand::Search(search)) = &request.message {
                    if service.accept_search_generation(search.generation) {
                        next_search_task = next_search_task.saturating_add(1);
                        active_search_task.store(next_search_task, Ordering::Release);
                        search_executor.submit(SearchJob {
                            task_id: next_search_task,
                            request,
                            snapshots: Arc::clone(&service.snapshots),
                            settings: service.settings.clone(),
                        });
                    }
                    continue;
                }
                let refresh = matches!(
                    request.message,
                    Message::IndexerCommand(IndexerCommand::RefreshVolumes)
                );
                let explicit_rebuild = matches!(
                    request.message,
                    Message::IndexerCommand(IndexerCommand::RebuildIndex)
                );
                let settings_update = matches!(
                    request.message,
                    Message::IndexerCommand(IndexerCommand::UpdateSettings(_))
                );
                let settings_changed = match &request.message {
                    Message::IndexerCommand(IndexerCommand::UpdateSettings(settings)) => {
                        normalize_index_settings(settings.clone()) != service.settings
                    }
                    _ => false,
                };
                if refresh {
                    next_search_task = next_search_task.saturating_add(1);
                    active_search_task.store(next_search_task, Ordering::Release);
                    service.rediscover();
                }
                write_responses(&mut writer, service.handle(request))?;
                let rebuild = refresh
                    || explicit_rebuild
                    || (settings_changed && service.enabled_volume_missing_snapshot());
                if rebuild {
                    next_build_generation = next_build_generation.saturating_add(1);
                    let drives = if settings_update && !refresh && !explicit_rebuild {
                        service.missing_build_drives()
                    } else {
                        service.build_drives()
                    };
                    if drives.is_empty() {
                        continue;
                    }
                    let job = PendingBuild {
                        generation: next_build_generation,
                        drives,
                        missing_only: settings_update && !refresh && !explicit_rebuild,
                    };
                    if active_build.is_some() {
                        pending_build = Some(match pending_build.take() {
                            Some(current) => merge_pending_build(current, job),
                            None => job,
                        });
                    } else {
                        active_build = Some(job.generation);
                        spawn_snapshot_build(
                            job.generation,
                            job.drives,
                            Arc::clone(&scanner),
                            events.clone(),
                        );
                    }
                }
            }
            RuntimeEvent::ReaderClosed => {
                next_search_task = next_search_task.saturating_add(1);
                active_search_task.store(next_search_task, Ordering::Release);
                if active_build.is_none()
                    && pending_build.is_none()
                    && active_compactions.is_empty()
                {
                    return Ok(());
                }
                reader_closed = true;
            }
            RuntimeEvent::ReaderFailed(error) => return Err(error),
            RuntimeEvent::BuildCompleted {
                generation,
                outcome,
            } => {
                if active_build != Some(generation) {
                    continue;
                }
                active_build = None;
                service.apply_build(outcome);
                if let Some(mut pending) = pending_build.take() {
                    pending
                        .drives
                        .retain(|drive| service.volume_is_enabled(&drive.mount_point));
                    if pending.missing_only {
                        pending.drives.retain(|drive| {
                            service.snapshot_for_mount(&drive.mount_point).is_none()
                        });
                    }
                    if !pending.drives.is_empty() {
                        active_build = Some(pending.generation);
                        spawn_snapshot_build(
                            pending.generation,
                            pending.drives,
                            Arc::clone(&scanner),
                            events.clone(),
                        );
                    }
                }
                journal_schedule.reset(Instant::now());
                if reader_closed && active_build.is_none() {
                    return Ok(());
                }
                write_responses(
                    &mut writer,
                    vec![Envelope::new(Message::IndexerEvent(IndexerEvent::Status(
                        service.status().clone(),
                    )))],
                )?;
            }
            RuntimeEvent::BuildFailed {
                generation,
                mount_points,
                message,
            } => {
                if active_build != Some(generation) {
                    continue;
                }
                active_build = None;
                service.record_build_failure(mount_points, message);
                if let Some(mut pending) = pending_build.take() {
                    pending
                        .drives
                        .retain(|drive| service.volume_is_enabled(&drive.mount_point));
                    if !pending.drives.is_empty() {
                        active_build = Some(pending.generation);
                        spawn_snapshot_build(
                            pending.generation,
                            pending.drives,
                            Arc::clone(&scanner),
                            events.clone(),
                        );
                    }
                }
                if reader_closed && active_build.is_none() {
                    return Ok(());
                }
                write_responses(
                    &mut writer,
                    vec![Envelope::new(Message::IndexerEvent(IndexerEvent::Status(
                        service.status().clone(),
                    )))],
                )?;
            }
            RuntimeEvent::SearchCompleted { task_id, response } => {
                if active_search_task.load(Ordering::Acquire) == task_id {
                    write_responses(&mut writer, vec![response])?;
                }
            }
            RuntimeEvent::CompactionCompleted {
                mount_point,
                journal_id,
                next_usn,
                snapshot,
            } => {
                active_compactions.remove(&mount_point.to_ascii_lowercase());
                if service.apply_compaction(&mount_point, journal_id, next_usn, snapshot) {
                    info!(%mount_point, "published compacted incremental index snapshot");
                } else {
                    service.clear_compaction_failure(&mount_point);
                    warn!(%mount_point, "discarded stale compacted index snapshot");
                }
                journal_schedule.reset(Instant::now());
                if reader_closed
                    && active_build.is_none()
                    && pending_build.is_none()
                    && active_compactions.is_empty()
                {
                    return Ok(());
                }
            }
            RuntimeEvent::CompactionFailed {
                mount_point,
                message,
            } => {
                active_compactions.remove(&mount_point.to_ascii_lowercase());
                service.record_compaction_failure(&mount_point, &message);
                warn!(%mount_point, %message, "incremental index compaction failed");
                if let Some(retry_at) = service.compaction_retry_at(&mount_point) {
                    journal_schedule.defer_until(retry_at);
                }
                if !reader_closed {
                    write_responses(
                        &mut writer,
                        vec![Envelope::new(Message::IndexerEvent(IndexerEvent::Status(
                            service.status().clone(),
                        )))],
                    )?;
                }
                if reader_closed
                    && active_build.is_none()
                    && pending_build.is_none()
                    && active_compactions.is_empty()
                {
                    return Ok(());
                }
            }
        }
    }
    Ok(())
}

fn spawn_snapshot_build(
    generation: u64,
    drives: Vec<IndexDriveStatus>,
    scanner: Arc<SnapshotScanner>,
    events: mpsc::SyncSender<RuntimeEvent>,
) {
    thread::spawn(move || {
        let mount_points = drives
            .iter()
            .map(|drive| drive.mount_point.clone())
            .collect::<Vec<_>>();
        match catch_unwind(AssertUnwindSafe(|| {
            build_snapshots(drives, |mount_point| scanner(mount_point))
        })) {
            Ok(outcome) => {
                let _ = events.send(RuntimeEvent::BuildCompleted {
                    generation,
                    outcome,
                });
            }
            Err(payload) => {
                let message = payload
                    .downcast_ref::<&str>()
                    .map(|message| (*message).to_owned())
                    .or_else(|| payload.downcast_ref::<String>().cloned())
                    .unwrap_or_else(|| "unknown panic payload".into());
                let _ = events.send(RuntimeEvent::BuildFailed {
                    generation,
                    mount_points,
                    message: format!("panic: {message}"),
                });
            }
        }
    });
}

fn schedule_snapshot_compactions(
    mount_points: Vec<String>,
    service: &IndexerService,
    active_compactions: &mut HashSet<String>,
    events: &mpsc::SyncSender<RuntimeEvent>,
) -> Option<Instant> {
    let now = Instant::now();
    let mut earliest_retry = None::<Instant>;
    for mount_point in mount_points {
        if let Some(retry_at) = service.compaction_retry_at(&mount_point)
            && retry_at > now
        {
            earliest_retry = Some(earliest_retry.map_or(retry_at, |current| current.min(retry_at)));
            continue;
        }
        let key = mount_point.to_ascii_lowercase();
        if active_compactions.contains(&key) {
            continue;
        }
        if let Some(snapshot) = service.snapshot_for_mount(&mount_point) {
            active_compactions.insert(key);
            spawn_snapshot_compaction(snapshot, events.clone());
        }
    }
    earliest_retry
}

/// Trims the process working set after a snapshot restore or rebuild. The
/// MFT snapshot is a memory-mapped file: search faults every touched page
/// into the working set and Windows never returns them on its own, so Task
/// Manager shows the whole snapshot (~1 GB for C:+D:) as resident. Trimming
/// moves those pages to the standby list — the mapping stays valid and
/// subsequent searches fault pages back in on demand.
#[allow(unsafe_code)] // Windows process working-set control has no safe wrapper.
fn trim_working_set_after_snapshot_restore() {
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, SetProcessWorkingSetSize};
    // (-1, -1) empties the working set to the minimum allowed.
    unsafe {
        let _ = SetProcessWorkingSetSize(GetCurrentProcess(), usize::MAX, usize::MAX);
    }
}

fn spawn_snapshot_compaction(snapshot: MftSnapshot, events: mpsc::SyncSender<RuntimeEvent>) {
    thread::spawn(move || {
        let mount_point = snapshot.mount_point.clone();
        let journal_id = snapshot.journal_id;
        let next_usn = snapshot.next_usn;
        match mft::compact_system_snapshot(&snapshot) {
            Ok(snapshot) => {
                let _ = events.send(RuntimeEvent::CompactionCompleted {
                    mount_point,
                    journal_id,
                    next_usn,
                    snapshot,
                });
            }
            Err(error) => {
                let _ = events.send(RuntimeEvent::CompactionFailed {
                    mount_point,
                    message: error.to_string(),
                });
            }
        }
    });
}

fn run_snapshot_search(
    job: SearchJob,
    active_task: &AtomicU64,
    events: &mpsc::SyncSender<RuntimeEvent>,
) {
    let Message::IndexerCommand(IndexerCommand::Search(search)) = &job.request.message else {
        return;
    };
    let task_id = job.task_id;
    let generation = search.generation;
    let started = Instant::now();
    let Some(items) = search_snapshots_while(&job.snapshots, &job.settings, search, || {
        active_task.load(Ordering::Acquire) == task_id
    }) else {
        return;
    };
    if active_task.load(Ordering::Acquire) != task_id {
        return;
    }
    let batches = partition_search_batches(generation, items, started.elapsed().as_micros() as u64);
    for batch in batches {
        if active_task.load(Ordering::Acquire) != task_id {
            return;
        }
        let response = Envelope::reply_to(
            &job.request,
            Message::IndexerEvent(IndexerEvent::SearchBatch(batch)),
        );
        if !send_search_completion(task_id, response, active_task, events) {
            return;
        }
    }
}

fn partition_search_batches(
    generation: u64,
    items: Vec<IndexSearchItem>,
    elapsed_us: u64,
) -> Vec<IndexSearchBatch> {
    if items.is_empty() {
        return vec![IndexSearchBatch {
            generation,
            items,
            complete: true,
            elapsed_us,
        }];
    }
    let batch_count = items.len().div_ceil(SEARCH_BATCH_SIZE);
    let mut chunks = items.into_iter();
    (0..batch_count)
        .map(|index| IndexSearchBatch {
            generation,
            items: chunks.by_ref().take(SEARCH_BATCH_SIZE).collect(),
            complete: index + 1 == batch_count,
            elapsed_us,
        })
        .collect()
}

fn send_search_completion(
    task_id: u64,
    response: Envelope,
    active_task: &AtomicU64,
    events: &mpsc::SyncSender<RuntimeEvent>,
) -> bool {
    let mut completion = RuntimeEvent::SearchCompleted { task_id, response };
    loop {
        match events.try_send(completion) {
            Ok(()) => return true,
            Err(mpsc::TrySendError::Full(returned)) => {
                if active_task.load(Ordering::Acquire) != task_id {
                    return false;
                }
                completion = returned;
                thread::sleep(Duration::from_millis(1));
            }
            Err(mpsc::TrySendError::Disconnected(_)) => return false,
        }
    }
}

fn write_responses(writer: &mut impl Write, responses: Vec<Envelope>) -> Result<()> {
    for response in responses {
        write_envelope_sync(writer, &response)?;
    }
    Ok(())
}

#[cfg(test)]
#[cfg(test)]
mod tests;
