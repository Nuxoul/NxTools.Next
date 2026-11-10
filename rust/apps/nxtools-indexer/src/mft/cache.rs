use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail, ensure};
use memmap2::{Mmap, MmapOptions};
use tracing::{info, warn};

#[cfg(windows)]
use windows_sys::Win32::System::Memory::{PrefetchVirtualMemory, WIN32_MEMORY_RANGE_ENTRY};
#[cfg(windows)]
use windows_sys::Win32::System::Threading::GetCurrentProcess;

use super::{MftEntry, MftEntryRef, MftIdentity, MftOverlay, MftSnapshot};
use nxtools_finder::search_char_mask;

const MAGIC: u64 = 0x0058_4449_5446_4D4E; // "NMFTIDX\0"
const VERSION: u32 = 3;
const HEADER_SIZE: usize = 256;
const RECORD_SIZE: usize = 56;
const MOUNT_OFFSET: usize = 128;
const MAX_MOUNT_BYTES: usize = HEADER_SIZE - MOUNT_OFFSET;
const NONE_U64: u64 = u64::MAX;
const NONE_I64: i64 = i64::MIN;
const FLAG_IDENTITY: u8 = 1;
const FLAG_DIRECTORY: u8 = 2;
const DELTA_MAGIC: u64 = 0x0041_544C_4544_4D4E; // "NMDELTA\0"
const DELTA_HEADER_SIZE: usize = 64;
const DELTA_RECORD_SIZE: usize = 56;

#[cfg(test)]
mod compaction_schedule_tests;
mod journal;
#[cfg(all(test, windows))]
mod measurements;
#[cfg(test)]
mod publication_tests;
pub(super) use journal::DeltaCheckpoint;

pub(super) struct MappedSnapshot {
    map: Mmap,
    records_offset: usize,
    child_order_offset: usize,
    name_masks_offset: usize,
    name_offsets_offset: usize,
    path_strings_offset: usize,
    path_strings_len: usize,
    name_strings_offset: usize,
    name_strings_len: usize,
    count: usize,
}

impl std::fmt::Debug for MappedSnapshot {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("MappedSnapshot")
            .field("count", &self.count)
            .field("mapped_bytes", &self.map.len())
            .finish()
    }
}

impl MappedSnapshot {
    #[allow(unsafe_code)]
    fn open(path: &Path, expected_mount: &str) -> Result<(Self, Option<u64>, Option<i64>, usize)> {
        let file =
            File::open(path).with_context(|| format!("open index snapshot {}", path.display()))?;
        let map = unsafe { MmapOptions::new().map(&file) }
            .with_context(|| format!("map index snapshot {}", path.display()))?;
        ensure!(
            map.len() >= HEADER_SIZE,
            "index snapshot header was truncated"
        );
        ensure!(read_u64(&map, 0)? == MAGIC, "index snapshot magic mismatch");
        ensure!(
            read_u32(&map, 8)? == VERSION,
            "index snapshot version mismatch"
        );
        ensure!(
            read_u32(&map, 12)? as usize == HEADER_SIZE,
            "index snapshot header size mismatch"
        );
        ensure!(
            read_u32(&map, 16)? as usize == RECORD_SIZE,
            "index snapshot record size mismatch"
        );
        let mount_len = read_u32(&map, 20)? as usize;
        ensure!(
            mount_len <= MAX_MOUNT_BYTES,
            "index snapshot mount was invalid"
        );
        let mount = std::str::from_utf8(
            map.get(MOUNT_OFFSET..MOUNT_OFFSET + mount_len)
                .context("index snapshot mount exceeded header")?,
        )
        .context("index snapshot mount was not UTF-8")?;
        ensure!(
            mount.eq_ignore_ascii_case(expected_mount),
            "index snapshot belongs to {mount}, not {expected_mount}"
        );

        let count = usize::try_from(read_u64(&map, 24)?)
            .context("index snapshot record count exceeded this process")?;
        ensure!(
            count <= u32::MAX as usize,
            "index snapshot had too many rows"
        );
        let journal_raw = read_u64(&map, 32)?;
        let next_usn_raw = read_i64(&map, 40)?;
        let records_offset = usize::try_from(read_u64(&map, 48)?)?;
        let child_order_offset = usize::try_from(read_u64(&map, 56)?)?;
        let path_strings_offset = usize::try_from(read_u64(&map, 64)?)?;
        let path_strings_len = usize::try_from(read_u64(&map, 72)?)?;
        let visible_count = usize::try_from(read_u64(&map, 80)?)?;
        let name_masks_offset = usize::try_from(read_u64(&map, 88)?)?;
        let name_strings_offset = usize::try_from(read_u64(&map, 96)?)?;
        let name_strings_len = usize::try_from(read_u64(&map, 104)?)?;
        let records_end = records_offset
            .checked_add(
                count
                    .checked_mul(RECORD_SIZE)
                    .context("record table overflow")?,
            )
            .context("record table overflow")?;
        let children_end = child_order_offset
            .checked_add(count.checked_mul(4).context("child table overflow")?)
            .context("child table overflow")?;
        let name_masks_end = name_masks_offset
            .checked_add(count.checked_mul(8).context("name-mask table overflow")?)
            .context("name-mask table overflow")?;
        let name_offsets_offset = name_masks_end;
        let name_offsets_end = name_offsets_offset
            .checked_add(
                count
                    .checked_add(1)
                    .and_then(|count| count.checked_mul(4))
                    .context("name-offset table overflow")?,
            )
            .context("name-offset table overflow")?;
        let name_strings_end = name_strings_offset
            .checked_add(name_strings_len)
            .context("name string table overflow")?;
        let path_strings_end = path_strings_offset
            .checked_add(path_strings_len)
            .context("path string table overflow")?;
        ensure!(
            name_masks_offset == HEADER_SIZE,
            "name-mask table offset mismatch"
        );
        ensure!(
            name_strings_offset == name_offsets_end,
            "name string table offset mismatch"
        );
        ensure!(
            records_offset == name_strings_end,
            "record table offset mismatch"
        );
        ensure!(
            child_order_offset == records_end,
            "child table offset mismatch"
        );
        ensure!(
            path_strings_offset == children_end,
            "path string table offset mismatch"
        );
        ensure!(
            path_strings_end == map.len(),
            "index snapshot length mismatch"
        );
        ensure!(visible_count <= count, "visible count exceeded row count");

        let mapped = Self {
            map,
            records_offset,
            child_order_offset,
            name_masks_offset,
            name_offsets_offset,
            path_strings_offset,
            path_strings_len,
            name_strings_offset,
            name_strings_len,
            count,
        };
        let prefetch_started = Instant::now();
        let prefetched = mapped.prefetch_search_data();
        info!(
            bytes = mapped.search_data_range().1,
            elapsed_ms = prefetch_started.elapsed().as_millis(),
            prefetched,
            "requested persisted search-name prefetch"
        );

        Ok((
            mapped,
            (journal_raw != NONE_U64).then_some(journal_raw),
            (next_usn_raw != NONE_I64).then_some(next_usn_raw),
            visible_count,
        ))
    }

    pub(super) fn len(&self) -> usize {
        self.count
    }

    fn search_data_range(&self) -> (usize, usize) {
        let end = self.name_strings_offset + self.name_strings_len;
        (self.name_masks_offset, end - self.name_masks_offset)
    }

    #[cfg(windows)]
    #[allow(unsafe_code)]
    fn prefetch_search_data(&self) -> bool {
        let (offset, length) = self.search_data_range();
        let range = WIN32_MEMORY_RANGE_ENTRY {
            VirtualAddress: unsafe { self.map.as_ptr().add(offset) }.cast_mut().cast(),
            NumberOfBytes: length,
        };
        unsafe { PrefetchVirtualMemory(GetCurrentProcess(), 1, &range, 0) != 0 }
    }

    #[cfg(not(windows))]
    fn prefetch_search_data(&self) -> bool {
        false
    }

    pub(super) fn entry(&self, index: usize) -> Option<MftEntryRef<'_>> {
        let record = self.record(index)?;
        let flags = *record.get(48)?;
        let file_reference = read_u64(record, 0).ok()?;
        let parent_reference = read_u64(record, 8).ok()?;
        let path = self.path_string(
            read_u32(record, 16).ok()? as usize,
            read_u32(record, 20).ok()? as usize,
        )?;
        let name = self.name(index)?;
        let size = read_u64(record, 32).ok()?;
        let modified = read_i64(record, 40).ok()?;
        Some(MftEntryRef {
            identity: (flags & FLAG_IDENTITY != 0).then_some(MftIdentity {
                file_reference,
                parent_reference,
            }),
            path,
            name,
            is_directory: flags & FLAG_DIRECTORY != 0,
            size_bytes: (size != NONE_U64).then_some(size),
            modified_at_unix_ms: (modified != NONE_I64).then_some(modified),
        })
    }

    pub(super) fn name(&self, index: usize) -> Option<&str> {
        if index >= self.count {
            return None;
        }
        let offset = read_u32(&self.map, self.name_offsets_offset + index * 4).ok()? as usize;
        let end = read_u32(&self.map, self.name_offsets_offset + (index + 1) * 4).ok()? as usize;
        self.name_string(offset, end.checked_sub(offset)?)
    }

    pub(super) fn name_mask(&self, index: usize) -> Option<u64> {
        if index >= self.count {
            return None;
        }
        let offset = self
            .name_masks_offset
            .checked_add(index.checked_mul(std::mem::size_of::<u64>())?)?;
        read_u64(&self.map, offset).ok()
    }

    pub(super) fn find_by_reference(&self, file_reference: u64) -> Option<MftEntryRef<'_>> {
        let mut low = 0;
        let mut high = self.count;
        while low < high {
            let middle = low + (high - low) / 2;
            let candidate = self.file_reference(middle)?;
            match candidate.cmp(&file_reference) {
                Ordering::Less => low = middle + 1,
                Ordering::Greater => high = middle,
                Ordering::Equal => return self.entry(middle),
            }
        }
        None
    }

    pub(super) fn children(&self, parent_reference: u64) -> Vec<u64> {
        let lower = self.child_partition(parent_reference, false);
        let upper = self.child_partition(parent_reference, true);
        (lower..upper)
            .filter_map(|offset| self.child_row(offset))
            .filter_map(|row| self.file_reference(row))
            .collect()
    }

    fn child_partition(&self, parent_reference: u64, upper: bool) -> usize {
        let mut low = 0;
        let mut high = self.count;
        while low < high {
            let middle = low + (high - low) / 2;
            let candidate = self
                .child_row(middle)
                .and_then(|row| self.parent_reference(row))
                .unwrap_or(u64::MAX);
            if candidate < parent_reference || (upper && candidate == parent_reference) {
                low = middle + 1;
            } else {
                high = middle;
            }
        }
        low
    }

    fn record(&self, index: usize) -> Option<&[u8]> {
        if index >= self.count {
            return None;
        }
        let start = self
            .records_offset
            .checked_add(index.checked_mul(RECORD_SIZE)?)?;
        self.map.get(start..start + RECORD_SIZE)
    }

    fn file_reference(&self, index: usize) -> Option<u64> {
        read_u64(self.record(index)?, 0).ok()
    }

    fn parent_reference(&self, index: usize) -> Option<u64> {
        read_u64(self.record(index)?, 8).ok()
    }

    fn child_row(&self, offset: usize) -> Option<usize> {
        if offset >= self.count {
            return None;
        }
        let start = self
            .child_order_offset
            .checked_add(offset.checked_mul(4)?)?;
        usize::try_from(read_u32(&self.map, start).ok()?).ok()
    }

    fn path_string(&self, offset: usize, length: usize) -> Option<&str> {
        let end = offset.checked_add(length)?;
        if end > self.path_strings_len {
            return None;
        }
        std::str::from_utf8(
            self.map
                .get(self.path_strings_offset + offset..self.path_strings_offset + end)?,
        )
        .ok()
    }

    fn name_string(&self, offset: usize, length: usize) -> Option<&str> {
        let end = offset.checked_add(length)?;
        if end > self.name_strings_len {
            return None;
        }
        std::str::from_utf8(
            self.map
                .get(self.name_strings_offset + offset..self.name_strings_offset + end)?,
        )
        .ok()
    }
}

pub(crate) fn load_system_snapshot(mount_point: &str) -> Result<Option<MftSnapshot>> {
    let (directory, prefix) = system_cache_location(mount_point);
    load_cached_snapshot(&directory, &prefix, mount_point)
}

fn load_cached_snapshot(
    directory: &Path,
    prefix: &str,
    mount_point: &str,
) -> Result<Option<MftSnapshot>> {
    let candidates = cache_candidates(directory, prefix)?;
    if candidates.is_empty() {
        return Ok(None);
    }
    type Candidate = (
        PathBuf,
        (MappedSnapshot, Option<u64>, Option<i64>, usize),
        Option<PersistedDelta>,
    );
    let mut opened: Option<Candidate> = None;
    let mut errors = Vec::new();
    for path in candidates {
        let (mapped, journal_id, next_usn, visible_count) =
            match MappedSnapshot::open(&path, mount_point) {
                Ok(base) => base,
                Err(error) => {
                    errors.push(format!("{}: {error}", path.display()));
                    continue;
                }
            };
        let delta = if delta_path(&path).is_file() {
            match load_delta_for_base(&path, mount_point, journal_id, next_usn, mapped.len()) {
                Ok(delta) => Some(delta),
                Err(_) => {
                    warn!(
                        event = "indexer.cache.delta_rejected",
                        "invalid delta rejected; base remains available for journal catch-up"
                    );
                    None
                }
            }
        } else {
            None
        };
        let effective_cursor = delta.as_ref().map_or(next_usn, |delta| delta.next_usn);
        if let Some((_, (_, selected_journal, selected_cursor, _), selected_delta)) = &opened {
            let selected_effective = selected_delta
                .as_ref()
                .map_or(*selected_cursor, |delta: &PersistedDelta| delta.next_usn);
            if *selected_journal != journal_id || selected_effective >= effective_cursor {
                continue;
            }
        }
        opened = Some((path, (mapped, journal_id, next_usn, visible_count), delta));
    }
    let Some((path, (mapped, journal_id, next_usn, visible_count), delta)) = opened else {
        bail!(
            "no valid persisted snapshot remained: {}",
            errors.join("; ")
        );
    };
    // All losing mappings are dropped before cleanup; retain the pair with newest committed
    // cursor within this journal, not merely a later-written stale compaction base.
    cleanup_obsolete_cache_files(directory, prefix, &path);
    info!(
        mount_point,
        rows = mapped.len(),
        "opened persisted MFT base/delta pair"
    );
    let mut snapshot = MftSnapshot::from_mapped(
        mount_point.to_owned(),
        journal_id,
        next_usn,
        visible_count,
        mapped,
        path,
    );
    if let Some(delta) = delta {
        *snapshot
            .delta_checkpoint
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = delta.checkpoint;
        snapshot.restore_delta(
            delta.journal_id,
            delta.next_usn,
            delta.visible_count,
            delta.overlay,
        );
    }
    Ok(Some(snapshot))
}

fn load_delta_for_base(
    base_path: &Path,
    mount_point: &str,
    journal_id: Option<u64>,
    base_cursor: Option<i64>,
    rows: usize,
) -> Result<PersistedDelta> {
    let path = delta_path(base_path);
    let bytes = fs::read(&path).context("read paired index delta")?;
    if journal::is_journal(&bytes) {
        journal::validate_base_binding(&bytes, base_path, mount_point, base_cursor, rows)?;
        journal::decode(&bytes, journal_id)
    } else {
        load_delta(&path, journal_id)
    }
}

pub(crate) fn persist_system_snapshot(snapshot: &mut MftSnapshot) -> Result<()> {
    let (directory, prefix) = system_cache_location(&snapshot.mount_point);
    fs::create_dir_all(&directory)
        .with_context(|| format!("create snapshot directory {}", directory.display()))?;
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let path = directory.join(format!("{prefix}-{nonce}.idx"));
    persist_snapshot(snapshot, &path)?;
    snapshot.set_cache_path(path.clone());
    info!(
        mount_point = snapshot.mount_point,
        rows = snapshot.base_len(),
        cache = %path.display(),
        "persisted MFT snapshot"
    );
    Ok(())
}

pub(super) fn reopen_persisted_snapshot(snapshot: MftSnapshot) -> Result<MftSnapshot> {
    let path = snapshot
        .cache_path()
        .context("completed snapshot has no persisted base")?
        .to_owned();
    let mount_point = snapshot.mount_point.clone();
    // Release build rows and their row-indexed auxiliaries before building mapped search data.
    // Existing service/search leases own independent snapshots and remain untouched.
    drop(snapshot);
    let (mapped, journal_id, next_usn, visible_count) =
        MappedSnapshot::open(&path, &mount_point).context("reopen persisted MFT base")?;
    Ok(MftSnapshot::from_mapped(
        mount_point,
        journal_id,
        next_usn,
        visible_count,
        mapped,
        path,
    ))
}

pub(crate) fn cleanup_published_snapshot(snapshot: &MftSnapshot) {
    let Some(path) = snapshot.cache_path() else {
        return;
    };
    let Some(directory) = path.parent() else {
        return;
    };
    let (_, prefix) = system_cache_location(&snapshot.mount_point);
    cleanup_obsolete_cache_files(directory, &prefix, path);
}

pub(crate) fn persist_system_delta(snapshot: &MftSnapshot) -> Result<()> {
    let cache_path = snapshot
        .cache_path()
        .context("snapshot has no persisted base for its delta")?;
    let path = delta_path(cache_path);
    journal::persist(snapshot, &path)
}

fn delta_path(snapshot_path: &Path) -> PathBuf {
    snapshot_path.with_extension("delta")
}

fn system_cache_location(mount_point: &str) -> (PathBuf, String) {
    let root = std::env::var_os("PROGRAMDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\ProgramData"));
    let key = mount_point
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let directory = root.join("NxTools").join("NxTools.Next").join("indexer");
    (directory, format!("volume-{key}"))
}

fn cache_candidates(directory: &Path, prefix: &str) -> Result<Vec<PathBuf>> {
    if !directory.is_dir() {
        return Ok(Vec::new());
    }
    let legacy = format!("{prefix}.idx");
    let version_prefix = format!("{prefix}-");
    let mut candidates = fs::read_dir(directory)
        .with_context(|| format!("read snapshot directory {}", directory.display()))?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| {
                    name == legacy || (name.starts_with(&version_prefix) && name.ends_with(".idx"))
                })
        })
        .collect::<Vec<_>>();
    candidates.sort_by_key(|path| {
        std::cmp::Reverse(
            path.metadata()
                .and_then(|metadata| metadata.modified())
                .unwrap_or(UNIX_EPOCH),
        )
    });
    Ok(candidates)
}

fn cleanup_obsolete_cache_files(directory: &Path, prefix: &str, keep: &Path) {
    let started = Instant::now();
    let mut removed = 0_usize;
    let mut deferred = 0_usize;
    let mut native_error_code = None;
    let keep_delta = delta_path(keep);
    let legacy = format!("{prefix}.idx");
    let legacy_delta = format!("{prefix}.delta");
    let version_prefix = format!("{prefix}-");
    let temporary_prefix = format!("{prefix}.tmp-");
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path == keep || path == keep_delta {
            continue;
        }
        let removable = path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| {
                name == legacy
                    || name == legacy_delta
                    || name.starts_with(&temporary_prefix)
                    || (name.starts_with(&version_prefix)
                        && (name.ends_with(".idx")
                            || name.ends_with(".delta")
                            || name.contains(".tmp-")))
            });
        if removable {
            match fs::remove_file(&path) {
                Ok(()) => removed += 1,
                Err(error) => {
                    deferred += 1;
                    native_error_code = error.raw_os_error();
                }
            }
        }
    }
    if removed != 0 || deferred != 0 {
        let correlation_id = nxtools_protocol::Envelope::new(nxtools_protocol::Message::Pong(
            nxtools_protocol::Pong { sent_at_unix_ms: 0 },
        ))
        .id;
        info!(event = "indexer.cache.retired", %correlation_id, removed, deferred,
            native_error_code, elapsed_ms = started.elapsed().as_millis() as u64,
            "obsolete index cache retirement completed");
    }
}

struct PersistedDelta {
    journal_id: Option<u64>,
    next_usn: Option<i64>,
    visible_count: usize,
    overlay: MftOverlay,
    checkpoint: Option<DeltaCheckpoint>,
}

fn load_delta(path: &Path, expected_journal_id: Option<u64>) -> Result<PersistedDelta> {
    let bytes = fs::read(path).with_context(|| format!("read index delta {}", path.display()))?;
    if journal::is_journal(&bytes) {
        return journal::decode(&bytes, expected_journal_id);
    }
    ensure!(
        bytes.len() >= DELTA_HEADER_SIZE,
        "index delta header was truncated"
    );
    ensure!(
        read_u64(&bytes, 0)? == DELTA_MAGIC,
        "index delta magic mismatch"
    );
    ensure!(
        read_u32(&bytes, 8)? == VERSION,
        "index delta version mismatch"
    );
    ensure!(
        read_u32(&bytes, 12)? as usize == DELTA_HEADER_SIZE,
        "index delta header size mismatch"
    );
    let journal_raw = read_u64(&bytes, 16)?;
    let journal_id = (journal_raw != NONE_U64).then_some(journal_raw);
    ensure!(
        journal_id == expected_journal_id,
        "index delta journal identity did not match its base"
    );
    let next_usn_raw = read_i64(&bytes, 24)?;
    let next_usn = (next_usn_raw != NONE_I64).then_some(next_usn_raw);
    let visible_count = usize::try_from(read_u64(&bytes, 32)?)?;
    let replacement_count = usize::try_from(read_u64(&bytes, 40)?)?;
    let deleted_count = usize::try_from(read_u64(&bytes, 48)?)?;
    ensure!(
        replacement_count <= u32::MAX as usize && deleted_count <= u32::MAX as usize,
        "index delta row count was invalid"
    );
    let minimum_payload = replacement_count
        .checked_mul(DELTA_RECORD_SIZE)
        .and_then(|records| {
            deleted_count
                .checked_mul(8)
                .and_then(|deletes| records.checked_add(deletes))
        })
        .context("index delta table size overflow")?;
    ensure!(
        minimum_payload <= bytes.len() - DELTA_HEADER_SIZE,
        "index delta declared tables exceeded payload"
    );

    let mut cursor = DELTA_HEADER_SIZE;
    let mut replacements = HashMap::with_capacity(replacement_count);
    for _ in 0..replacement_count {
        let record_end = cursor
            .checked_add(DELTA_RECORD_SIZE)
            .context("index delta record overflow")?;
        let record = bytes
            .get(cursor..record_end)
            .context("index delta record was truncated")?;
        cursor = record_end;
        let flags = *record.get(48).context("index delta flags were truncated")?;
        let file_reference = read_u64(record, 0)?;
        let parent_reference = read_u64(record, 8)?;
        let path_len = read_u32(record, 32)? as usize;
        let name_len = read_u32(record, 36)? as usize;
        let strings_end = cursor
            .checked_add(path_len)
            .and_then(|end| end.checked_add(name_len))
            .context("index delta strings overflow")?;
        let strings = bytes
            .get(cursor..strings_end)
            .context("index delta strings were truncated")?;
        let path_text =
            std::str::from_utf8(&strings[..path_len]).context("index delta path was not UTF-8")?;
        let name_text =
            std::str::from_utf8(&strings[path_len..]).context("index delta name was not UTF-8")?;
        cursor = strings_end;
        replacements.insert(
            file_reference,
            MftEntry {
                identity: (flags & FLAG_IDENTITY != 0).then_some(MftIdentity {
                    file_reference,
                    parent_reference,
                }),
                path: path_text.to_owned(),
                name: name_text.to_owned(),
                is_directory: flags & FLAG_DIRECTORY != 0,
                size_bytes: {
                    let value = read_u64(record, 16)?;
                    (value != NONE_U64).then_some(value)
                },
                modified_at_unix_ms: {
                    let value = read_i64(record, 24)?;
                    (value != NONE_I64).then_some(value)
                },
            }
            .into(),
        );
    }
    let deleted_bytes = deleted_count
        .checked_mul(8)
        .context("index delta delete table overflow")?;
    ensure!(
        cursor.checked_add(deleted_bytes) == Some(bytes.len()),
        "index delta length mismatch"
    );
    let mut deleted = HashSet::with_capacity(deleted_count);
    for _ in 0..deleted_count {
        deleted.insert(read_u64(&bytes, cursor)?);
        cursor += 8;
    }
    Ok(PersistedDelta {
        journal_id,
        next_usn,
        visible_count,
        checkpoint: None,
        overlay: MftOverlay {
            replacements,
            deleted,
        },
    })
}

#[cfg(test)]
fn persist_delta(snapshot: &MftSnapshot, path: &Path) -> Result<()> {
    let parent = path.parent().context("delta path omitted its directory")?;
    fs::create_dir_all(parent)
        .with_context(|| format!("create delta directory {}", parent.display()))?;
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let temporary = path.with_extension(format!("tmp-{}-{nonce}", std::process::id()));
    let file = File::create(&temporary)
        .with_context(|| format!("create temporary delta {}", temporary.display()))?;
    let mut writer = BufWriter::with_capacity(256 * 1024, file);
    let mut header = [0_u8; DELTA_HEADER_SIZE];
    put_u64(&mut header, 0, DELTA_MAGIC);
    put_u32(&mut header, 8, VERSION);
    put_u32(&mut header, 12, DELTA_HEADER_SIZE as u32);
    put_u64(&mut header, 16, snapshot.journal_id.unwrap_or(NONE_U64));
    put_i64(&mut header, 24, snapshot.next_usn.unwrap_or(NONE_I64));
    put_u64(&mut header, 32, snapshot.visible_len() as u64);
    put_u64(&mut header, 40, snapshot.overlay.replacements.len() as u64);
    put_u64(&mut header, 48, snapshot.overlay.deleted.len() as u64);
    writer.write_all(&header)?;

    let mut replacements = snapshot.overlay.replacements.iter().collect::<Vec<_>>();
    replacements.sort_unstable_by_key(|(file_reference, _)| **file_reference);
    for (file_reference, entry) in replacements {
        ensure!(
            entry.path.len() <= u32::MAX as usize && entry.name.len() <= u32::MAX as usize,
            "index delta string exceeded format"
        );
        let mut record = [0_u8; DELTA_RECORD_SIZE];
        let mut flags = 0_u8;
        put_u64(&mut record, 0, *file_reference);
        if let Some(identity) = entry.identity {
            flags |= FLAG_IDENTITY;
            put_u64(&mut record, 8, identity.parent_reference);
        }
        put_u64(&mut record, 16, entry.size_bytes.unwrap_or(NONE_U64));
        put_i64(
            &mut record,
            24,
            entry.modified_at_unix_ms.unwrap_or(NONE_I64),
        );
        put_u32(&mut record, 32, entry.path.len() as u32);
        put_u32(&mut record, 36, entry.name.len() as u32);
        if entry.is_directory {
            flags |= FLAG_DIRECTORY;
        }
        record[48] = flags;
        writer.write_all(&record)?;
        writer.write_all(entry.path.as_bytes())?;
        writer.write_all(entry.name.as_bytes())?;
    }
    let mut deleted = snapshot.overlay.deleted.iter().copied().collect::<Vec<_>>();
    deleted.sort_unstable();
    for file_reference in deleted {
        writer.write_all(&file_reference.to_le_bytes())?;
    }
    writer.flush()?;
    writer.get_ref().sync_all()?;
    drop(writer);
    atomic_replace(&temporary, path)
}

fn persist_snapshot(snapshot: &MftSnapshot, path: &Path) -> Result<()> {
    ensure!(
        snapshot.overlay.replacements.is_empty() && snapshot.overlay.deleted.is_empty(),
        "only compact base snapshots can be persisted"
    );
    let entries = snapshot
        .owned_entries()
        .context("mapped snapshot does not need to be persisted again")?;
    ensure!(
        entries.len() <= u32::MAX as usize,
        "snapshot row count exceeded format"
    );
    let mount = snapshot.mount_point.as_bytes();
    ensure!(mount.len() <= MAX_MOUNT_BYTES, "mount point was too long");

    let mut order = (0..entries.len()).collect::<Vec<_>>();
    order.sort_unstable_by_key(|index| {
        entries[*index]
            .identity
            .map(|identity| identity.file_reference)
            .unwrap_or(u64::MAX)
    });
    let mut child_order = (0..order.len() as u32).collect::<Vec<_>>();
    child_order.sort_unstable_by_key(|row| {
        let entry = &entries[order[*row as usize]];
        let identity = entry.identity.unwrap_or(MftIdentity {
            file_reference: u64::MAX,
            parent_reference: u64::MAX,
        });
        (identity.parent_reference, identity.file_reference)
    });

    let name_masks_offset = HEADER_SIZE;
    let name_offsets_offset = name_masks_offset + order.len() * std::mem::size_of::<u64>();
    let name_strings_offset = name_offsets_offset + (order.len() + 1) * 4;
    let name_strings_len = order.iter().try_fold(0_u64, |total, index| {
        total
            .checked_add(entries[*index].name.len() as u64)
            .context("snapshot name string table overflow")
    })?;
    ensure!(
        name_strings_len <= u32::MAX as u64,
        "snapshot name string table exceeded 4 GiB"
    );
    let records_offset = name_strings_offset + name_strings_len as usize;
    let child_order_offset = records_offset + order.len() * RECORD_SIZE;
    let path_strings_offset = child_order_offset + child_order.len() * 4;
    let path_strings_len = order.iter().try_fold(0_u64, |total, index| {
        total
            .checked_add(entries[*index].path.len() as u64)
            .context("snapshot path string table overflow")
    })?;
    ensure!(
        path_strings_len <= u32::MAX as u64,
        "snapshot path string table exceeded 4 GiB"
    );

    let parent = path
        .parent()
        .context("snapshot path omitted its directory")?;
    fs::create_dir_all(parent)
        .with_context(|| format!("create snapshot directory {}", parent.display()))?;
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let temporary = path.with_extension(format!("tmp-{}-{nonce}", std::process::id()));
    let file = File::create(&temporary)
        .with_context(|| format!("create temporary snapshot {}", temporary.display()))?;
    let mut writer = BufWriter::with_capacity(1024 * 1024, file);

    let mut header = [0_u8; HEADER_SIZE];
    put_u64(&mut header, 0, MAGIC);
    put_u32(&mut header, 8, VERSION);
    put_u32(&mut header, 12, HEADER_SIZE as u32);
    put_u32(&mut header, 16, RECORD_SIZE as u32);
    put_u32(&mut header, 20, mount.len() as u32);
    put_u64(&mut header, 24, order.len() as u64);
    put_u64(&mut header, 32, snapshot.journal_id.unwrap_or(NONE_U64));
    put_i64(&mut header, 40, snapshot.next_usn.unwrap_or(NONE_I64));
    put_u64(&mut header, 48, records_offset as u64);
    put_u64(&mut header, 56, child_order_offset as u64);
    put_u64(&mut header, 64, path_strings_offset as u64);
    put_u64(&mut header, 72, path_strings_len);
    put_u64(&mut header, 80, snapshot.visible_len() as u64);
    put_u64(&mut header, 88, name_masks_offset as u64);
    put_u64(&mut header, 96, name_strings_offset as u64);
    put_u64(&mut header, 104, name_strings_len);
    header[MOUNT_OFFSET..MOUNT_OFFSET + mount.len()].copy_from_slice(mount);
    writer.write_all(&header)?;

    for index in &order {
        writer.write_all(&search_char_mask(&entries[*index].name).to_le_bytes())?;
    }
    let mut hot_name_cursor = 0_u32;
    for index in &order {
        writer.write_all(&hot_name_cursor.to_le_bytes())?;
        hot_name_cursor = hot_name_cursor
            .checked_add(entries[*index].name.len() as u32)
            .context("snapshot hot-name offsets overflowed")?;
    }
    writer.write_all(&hot_name_cursor.to_le_bytes())?;
    for index in &order {
        writer.write_all(entries[*index].name.as_bytes())?;
    }

    let mut path_cursor = 0_u32;
    let mut name_cursor = 0_u32;
    for index in &order {
        let entry = &entries[*index];
        let mut record = [0_u8; RECORD_SIZE];
        let mut flags = 0_u8;
        if let Some(identity) = entry.identity {
            flags |= FLAG_IDENTITY;
            put_u64(&mut record, 0, identity.file_reference);
            put_u64(&mut record, 8, identity.parent_reference);
        }
        put_u32(&mut record, 16, path_cursor);
        put_u32(&mut record, 20, entry.path.len() as u32);
        path_cursor = path_cursor
            .checked_add(entry.path.len() as u32)
            .context("snapshot path offsets overflowed")?;
        put_u32(&mut record, 24, name_cursor);
        put_u32(&mut record, 28, entry.name.len() as u32);
        name_cursor = name_cursor
            .checked_add(entry.name.len() as u32)
            .context("snapshot name offsets overflowed")?;
        put_u64(&mut record, 32, entry.size_bytes.unwrap_or(NONE_U64));
        put_i64(
            &mut record,
            40,
            entry.modified_at_unix_ms.unwrap_or(NONE_I64),
        );
        if entry.is_directory {
            flags |= FLAG_DIRECTORY;
        }
        record[48] = flags;
        writer.write_all(&record)?;
    }
    for row in child_order {
        writer.write_all(&row.to_le_bytes())?;
    }
    for index in &order {
        writer.write_all(entries[*index].path.as_bytes())?;
    }
    writer.flush()?;
    writer.get_ref().sync_all()?;
    drop(writer);
    let sidecar = delta_path(path);
    if sidecar.is_file() {
        fs::remove_file(&sidecar)
            .with_context(|| format!("remove stale index delta {}", sidecar.display()))?;
    }
    atomic_replace(&temporary, path)?;
    Ok(())
}

#[cfg(windows)]
#[allow(unsafe_code)]
fn atomic_replace(source: &Path, destination: &Path) -> Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Foundation::GetLastError;
    use windows_sys::Win32::Storage::FileSystem::{
        MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW,
    };

    let source = source
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let destination = destination
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let moved = unsafe {
        MoveFileExW(
            source.as_ptr(),
            destination.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if moved == 0 {
        bail!("atomic snapshot replace failed with Win32 {}", unsafe {
            GetLastError()
        });
    }
    Ok(())
}

#[cfg(not(windows))]
fn atomic_replace(source: &Path, destination: &Path) -> Result<()> {
    if destination.exists() {
        fs::remove_file(destination)?;
    }
    fs::rename(source, destination)?;
    Ok(())
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32> {
    let value = bytes
        .get(offset..offset + 4)
        .context("snapshot omitted u32")?;
    Ok(u32::from_le_bytes(
        value.try_into().expect("four-byte slice"),
    ))
}

fn read_u64(bytes: &[u8], offset: usize) -> Result<u64> {
    let value = bytes
        .get(offset..offset + 8)
        .context("snapshot omitted u64")?;
    Ok(u64::from_le_bytes(
        value.try_into().expect("eight-byte slice"),
    ))
}

fn read_i64(bytes: &[u8], offset: usize) -> Result<i64> {
    let value = bytes
        .get(offset..offset + 8)
        .context("snapshot omitted i64")?;
    Ok(i64::from_le_bytes(
        value.try_into().expect("eight-byte slice"),
    ))
}

fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn put_u64(bytes: &mut [u8], offset: usize, value: u64) {
    bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

fn put_i64(bytes: &mut [u8], offset: usize, value: i64) {
    bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use tempfile::tempdir;

    use super::*;
    use crate::mft::{MftEntry, MftIdentity};

    fn persisted_publication_fixture(directory: &Path, version: usize) -> MftSnapshot {
        let path = directory.join(format!("volume-433a5c-{version}.idx"));
        let mut snapshot = MftSnapshot::from_entries(
            r"C:\",
            Some(17),
            Some(100 + version as i64),
            vec![MftEntry {
                identity: Some(MftIdentity {
                    file_reference: 11,
                    parent_reference: 5,
                }),
                path: r"C:\retained.txt".into(),
                name: "retained.txt".into(),
                is_directory: false,
                size_bytes: None,
                modified_at_unix_ms: None,
            }],
        );
        persist_snapshot(&snapshot, &path).expect("temporary durable base");
        snapshot.set_cache_path(path.clone());
        persist_system_delta(&snapshot).expect("temporary paired delta");
        let (mapped, journal, cursor, visible) =
            MappedSnapshot::open(&path, r"C:\").expect("mapped base");
        MftSnapshot::from_mapped(r"C:\".into(), journal, cursor, visible, mapped, path)
    }

    fn assert_compaction_worker_releases_input_before_event(fail: bool) {
        use std::sync::mpsc;
        use std::time::{Duration, Instant};

        let directory = tempdir().expect("isolated compaction worker cache");
        let input = persisted_publication_fixture(directory.path(), 1);
        let input_mapping = match &input.base {
            super::super::MftBase::Mapped(mapped) => Arc::downgrade(mapped),
            _ => panic!("fixture must own a real mapping"),
        };
        let new_path = directory.path().join("volume-433a5c-2.idx");
        let output_path = new_path.clone();
        let (events, completed) = mpsc::sync_channel(0);
        let (computed, computation) = mpsc::channel();
        let worker = crate::spawn_snapshot_compaction_with(input, events, move |snapshot| {
            let outcome = if fail {
                Err(std::io::Error::from_raw_os_error(112).into())
            } else {
                let mut compacted = super::super::compact_snapshot(snapshot);
                persist_snapshot(&compacted, &output_path).expect("temporary compacted base");
                compacted.set_cache_path(output_path);
                persist_system_delta(&compacted).expect("temporary compacted delta");
                Ok(compacted)
            };
            computed
                .send(())
                .expect("computation reached event handoff");
            outcome
        });
        computation
            .recv_timeout(Duration::from_secs(2))
            .expect("compaction computed");
        let deadline = Instant::now() + Duration::from_secs(1);
        while input_mapping.strong_count() != 0 && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(1));
        }
        let released_before_receive = input_mapping.strong_count() == 0;
        let event = completed
            .recv_timeout(Duration::from_secs(2))
            .expect("worker terminal event");
        worker.join().expect("compaction worker exits");
        assert!(
            released_before_receive,
            "compaction worker must release its mapped input before the completion event can reach cleanup (failure={fail})"
        );
        match (fail, event) {
            (
                false,
                crate::RuntimeEvent::CompactionCompleted {
                    mount_point,
                    journal_id,
                    next_usn,
                    snapshot,
                },
            ) => {
                assert_eq!(mount_point, r"C:\");
                assert_eq!(journal_id, Some(17));
                assert_eq!(next_usn, Some(101));
                assert_eq!(snapshot.cache_path(), Some(new_path.as_path()));
                assert!(snapshot.visible_entry(11).is_some());
                assert!(load_delta_for_base(&new_path, r"C:\", Some(17), Some(101), 1).is_ok());
            }
            (true, crate::RuntimeEvent::CompactionFailed { mount_point, .. }) => {
                assert_eq!(mount_point, r"C:\");
            }
            _ => panic!("matching compaction terminal event required"),
        }
    }

    #[test]
    fn m3_compaction_worker_releases_mapped_input_before_success_event() {
        assert_compaction_worker_releases_input_before_event(false);
    }

    #[test]
    fn m3_compaction_worker_releases_mapped_input_before_failure_event() {
        assert_compaction_worker_releases_input_before_event(true);
    }

    #[test]
    fn m3_successive_service_build_publications_retire_released_cache_pairs() {
        let directory = tempdir().expect("isolated publication cache");
        let mut service =
            crate::IndexerService::from_volume_descriptors([crate::VolumeDescriptor::new(
                r"C:\", "NTFS",
            )]);
        for version in 1..=3 {
            let snapshot = persisted_publication_fixture(directory.path(), version);
            service.rebuild_with(move |_| Ok(snapshot.clone()));
            let mut remaining = fs::read_dir(directory.path())
                .expect("current cache")
                .map(|entry| {
                    entry
                        .expect("cache entry")
                        .file_name()
                        .to_string_lossy()
                        .into_owned()
                })
                .collect::<Vec<_>>();
            remaining.sort();
            assert_eq!(
                remaining,
                vec![
                    format!("volume-433a5c-{version}.delta"),
                    format!("volume-433a5c-{version}.idx"),
                ],
                "successful service publication must retire the released previous pair"
            );
            let current = &service.snapshots[0];
            assert!(current.visible_entry(11).is_some());
            assert!(
                load_delta_for_base(
                    current.cache_path().expect("persisted base"),
                    r"C:\",
                    Some(17),
                    current.next_usn,
                    1
                )
                .is_ok()
            );
        }
    }

    #[test]
    fn l11_legacy_delta_rejects_impossible_counts_before_preallocation() {
        let directory = tempdir().expect("isolated corrupt delta");
        let path = directory.path().join("volume.delta");
        let empty = MftSnapshot::from_entries(r"C:\", Some(17), Some(100), Vec::new());
        persist_delta(&empty, &path).expect("valid legacy header");
        let mut bytes = fs::read(&path).expect("legacy header");
        put_u64(&mut bytes, 40, 4_096);
        fs::write(&path, bytes).expect("small impossible-count fixture");
        let (result, allocations) =
            crate::mft::allocation_probe::measure(|| load_delta(&path, Some(17)));
        assert!(result.is_err(), "truncated legacy table must be rejected");
        assert!(
            allocations.largest_request < 16 * 1_024,
            "impossible table must be rejected before capacity allocation: {allocations:?}"
        );
    }

    #[test]
    fn m3_compaction_retirement_tolerates_leased_files_and_preserves_unpublished_pairs() {
        let directory = tempdir().expect("isolated compaction cache");
        let mut service =
            crate::IndexerService::from_volume_descriptors([crate::VolumeDescriptor::new(
                r"C:\", "NTFS",
            )]);
        service.rebuild_with(|_| Ok(persisted_publication_fixture(directory.path(), 1)));
        let search_snapshot = Arc::clone(&service.snapshots);
        #[cfg(windows)]
        let delete_lease = {
            use std::os::windows::fs::OpenOptionsExt;
            fs::OpenOptions::new()
                .read(true)
                .share_mode(1)
                .open(search_snapshot[0].cache_path().expect("leased base"))
                .expect("temporary reader denies deletion")
        };
        let mut compacted = super::super::compact_snapshot(&search_snapshot[0]);
        let new_path = directory.path().join("volume-433a5c-2.idx");
        persist_snapshot(&compacted, &new_path).expect("durable compacted base");
        compacted.set_cache_path(new_path.clone());
        persist_system_delta(&compacted).expect("durable compacted delta");
        let unpublished = persisted_publication_fixture(directory.path(), 3);
        assert!(service.apply_compaction(r"C:\", Some(17), Some(101), compacted));
        assert!(
            unpublished.cache_path().expect("worker base").exists(),
            "publication itself must not sweep another worker's unpublished pair"
        );
        drop(unpublished);
        service.cleanup_published_caches();
        assert!(new_path.exists());
        assert!(delta_path(&new_path).exists());
        assert!(
            search_snapshot[0].visible_entry(11).is_some(),
            "leased search remains valid"
        );
        #[cfg(windows)]
        assert!(
            directory.path().join("volume-433a5c-1.idx").exists(),
            "sharing violation defers old-base retirement"
        );
        drop(search_snapshot);
        #[cfg(windows)]
        drop(delete_lease);
        service.cleanup_published_caches();
        assert!(!directory.path().join("volume-433a5c-1.idx").exists());
        assert!(!directory.path().join("volume-433a5c-3.idx").exists());
        let restored = load_cached_snapshot(directory.path(), "volume-433a5c", r"C:\")
            .expect("current pair recovers")
            .expect("published base");
        assert_eq!(restored.cache_path(), Some(new_path.as_path()));
        assert!(restored.visible_entry(11).is_some());
    }

    #[test]
    fn l11_maximal_legacy_counts_reject_without_allocation_and_restore_the_base() {
        let directory = tempdir().expect("isolated legacy recovery");
        let snapshot = persisted_publication_fixture(directory.path(), 1);
        let base_path = snapshot.cache_path().expect("temporary base").to_owned();
        let path = delta_path(&base_path);
        let empty = MftSnapshot::from_entries(r"C:\", Some(17), Some(101), Vec::new());
        persist_delta(&empty, &path).expect("legacy header");
        let header = fs::read(&path).expect("header bytes");
        for (replacements, deleted) in [
            (u32::MAX as u64, 0),
            (0, u32::MAX as u64),
            (u64::MAX, u64::MAX),
        ] {
            let mut bytes = header.clone();
            put_u64(&mut bytes, 40, replacements);
            put_u64(&mut bytes, 48, deleted);
            fs::write(&path, bytes).expect("corrupt count header");
            let (result, allocations) =
                crate::mft::allocation_probe::measure(|| load_delta(&path, Some(17)));
            assert!(result.is_err());
            assert!(
                allocations.largest_request < 16 * 1024,
                "count-bound rejection allocates no row table: {allocations:?}"
            );
            let restored = load_cached_snapshot(directory.path(), "volume-433a5c", r"C:\")
                .expect("invalid delta does not prevent recovery")
                .expect("last-good base");
            assert_eq!(restored.next_usn, Some(101));
            assert!(restored.visible_entry(11).is_some());
            assert!(restored.overlay.replacements.is_empty());
        }
    }

    #[test]
    fn persisted_snapshot_opens_without_materializing_rows_and_preserves_children() {
        let directory = tempdir().expect("temporary cache directory");
        let path = directory.path().join("volume.idx");
        let snapshot = MftSnapshot::from_entries(
            r"C:\",
            Some(17),
            Some(100),
            vec![
                MftEntry {
                    identity: Some(MftIdentity {
                        file_reference: 11,
                        parent_reference: 10,
                    }),
                    path: r"C:\Work\Budget.xlsx".into(),
                    name: "Budget.xlsx".into(),
                    is_directory: false,
                    size_bytes: Some(42),
                    modified_at_unix_ms: Some(7),
                },
                MftEntry {
                    identity: Some(MftIdentity {
                        file_reference: 10,
                        parent_reference: 5,
                    }),
                    path: r"C:\Work".into(),
                    name: "Work".into(),
                    is_directory: true,
                    size_bytes: None,
                    modified_at_unix_ms: None,
                },
            ],
        );

        persist_snapshot(&snapshot, &path).expect("persist snapshot");
        let (mapped, journal_id, next_usn, visible_count) =
            MappedSnapshot::open(&path, r"C:\").expect("open mapped snapshot");

        assert_eq!(journal_id, Some(17));
        assert_eq!(next_usn, Some(100));
        assert_eq!(visible_count, 2);
        assert_eq!(mapped.len(), 2);
        assert!(mapped.name_masks_offset < mapped.name_offsets_offset);
        assert!(mapped.name_offsets_offset < mapped.name_strings_offset);
        assert!(mapped.name_strings_offset < mapped.records_offset);
        assert!(mapped.child_order_offset < mapped.path_strings_offset);
        assert_eq!(
            mapped.search_data_range(),
            (
                mapped.name_masks_offset,
                mapped.name_strings_offset + mapped.name_strings_len - mapped.name_masks_offset,
            )
        );
        assert_eq!(mapped.name_mask(1), Some(search_char_mask("Budget.xlsx")));
        assert_eq!(
            mapped.find_by_reference(11).expect("file row").path,
            r"C:\Work\Budget.xlsx"
        );
        assert_eq!(mapped.children(10), vec![11]);
        // R8 runtime-only search derivation from the real mapped-cache constructor.
        // Persistence bytes/layout above are unchanged; only hot names are grouped.
        let restored = MftSnapshot::from_mapped(
            r"C:\".into(),
            journal_id,
            next_usn,
            visible_count,
            mapped,
            path,
        );
        let request = nxtools_protocol::IndexSearchRequest {
            generation: 1,
            query: "'budget".into(),
            limit: 20,
            context: nxtools_protocol::IndexSearchContext::default(),
        };
        let query = nxtools_finder::FileQuery::parse(&request.query);
        assert_eq!(
            restored
                .search_data
                .as_ref()
                .expect("mapped runtime groups")
                .groups
                .len(),
            2
        );
        assert_eq!(
            crate::search_snapshots_while(
                std::slice::from_ref(&restored),
                &nxtools_protocol::IndexSettings::default(),
                &request,
                || true
            ),
            crate::legacy_search_name_only_snapshots_while(
                std::slice::from_ref(&restored),
                &nxtools_protocol::IndexSettings::default(),
                &request,
                &query,
                20,
                &|| true
            )
        );
        let recent_request = nxtools_protocol::IndexSearchRequest {
            query: "recent:".into(),
            context: nxtools_protocol::IndexSearchContext {
                include_files: true,
                include_directories: false,
                recent_modified: true,
                ..Default::default()
            },
            ..request.clone()
        };
        let recent = crate::search_snapshots_while(
            std::slice::from_ref(&restored),
            &nxtools_protocol::IndexSettings::default(),
            &recent_request,
            || true,
        )
        .expect("mapped recent");
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].modified_at_unix_ms, Some(7));
    }

    #[test]
    fn persisted_delta_restores_cursor_replacements_and_deletions() {
        let directory = tempdir().expect("temporary cache directory");
        let path = directory.path().join("volume.delta");
        let mut snapshot = MftSnapshot::from_entries(
            r"C:\",
            Some(17),
            Some(100),
            vec![MftEntry {
                identity: Some(MftIdentity {
                    file_reference: 11,
                    parent_reference: 10,
                }),
                path: r"C:\Work\old.txt".into(),
                name: "old.txt".into(),
                is_directory: false,
                size_bytes: Some(4),
                modified_at_unix_ms: Some(5),
            }],
        );
        snapshot.next_usn = Some(140);
        snapshot.overlay.deleted.insert(11);
        snapshot.overlay.replacements.insert(
            12,
            MftEntry {
                identity: Some(MftIdentity {
                    file_reference: 12,
                    parent_reference: 10,
                }),
                path: r"C:\Work\new.txt".into(),
                name: "new.txt".into(),
                is_directory: false,
                size_bytes: Some(8),
                modified_at_unix_ms: Some(9),
            }
            .into(),
        );

        persist_delta(&snapshot, &path).expect("persist delta");
        let delta = load_delta(&path, Some(17)).expect("load delta");

        assert_eq!(delta.next_usn, Some(140));
        assert!(delta.overlay.deleted.contains(&11));
        assert_eq!(
            delta
                .overlay
                .replacements
                .get(&12)
                .expect("replacement")
                .path,
            r"C:\Work\new.txt"
        );
        assert_eq!(
            delta
                .overlay
                .replacements
                .get(&12)
                .expect("time preserved")
                .modified_at_unix_ms,
            Some(9)
        );
        snapshot.overlay = delta.overlay;
        let recent = crate::recent::search_recent_modified(
            std::slice::from_ref(&snapshot),
            &nxtools_protocol::IndexSettings::default(),
            &nxtools_protocol::IndexSearchRequest {
                generation: 1,
                query: String::new(),
                limit: 20,
                context: nxtools_protocol::IndexSearchContext {
                    include_files: true,
                    ..Default::default()
                },
            },
            100,
            &|| true,
        )
        .expect("within budget")
        .expect("persisted delta recent");
        assert_eq!(
            recent
                .iter()
                .map(|item| item.modified_at_unix_ms)
                .collect::<Vec<_>>(),
            vec![Some(9)]
        );
    }

    #[test]
    fn cache_cleanup_keeps_only_the_selected_version_and_unrelated_files() {
        let directory = tempdir().expect("temporary cache directory");
        let root = directory.path();
        let keep = root.join("volume-433a5c-200.idx");
        let keep_delta = root.join("volume-433a5c-200.delta");
        let obsolete = [
            "volume-433a5c.idx",
            "volume-433a5c.delta",
            "volume-433a5c-100.idx",
            "volume-433a5c-100.delta",
            "volume-433a5c.tmp-10-20",
            "volume-433a5c-300.tmp-10-20",
        ];
        for path in obsolete.iter().map(|name| root.join(name)).chain([
            keep.clone(),
            keep_delta.clone(),
            root.join("unrelated.txt"),
        ]) {
            fs::write(path, b"test").expect("write cache fixture");
        }

        cleanup_obsolete_cache_files(root, "volume-433a5c", &keep);

        assert!(keep.is_file());
        assert!(keep_delta.is_file());
        assert!(root.join("unrelated.txt").is_file());
        assert!(obsolete.iter().all(|name| !root.join(name).exists()));
    }
}

#[cfg(test)]
mod persistence_fault_tests {
    use super::*;
    use crate::{IndexerService, JournalMaintenance, VolumeDescriptor};
    use nxtools_protocol::{IndexPhase, IndexSearchContext, IndexSearchRequest};

    #[test]
    fn persistence_fault_real_delta_publish_retains_search_and_recovers_current_cursor() {
        let directory = tempfile::tempdir().expect("isolated base and delta directory");
        let base_path = directory.path().join("volume.idx");
        let mut snapshot = MftSnapshot::from_entries(
            r"C:\",
            Some(17),
            Some(100),
            vec![MftEntry {
                identity: None,
                path: r"C:\kept.txt".into(),
                name: "kept.txt".into(),
                is_directory: false,
                size_bytes: Some(4),
                modified_at_unix_ms: None,
            }],
        );
        persist_snapshot(&snapshot, &base_path).expect("last-good base");
        snapshot.set_cache_path(base_path.clone());
        let blocked_delta = delta_path(&base_path);
        fs::create_dir(&blocked_delta).expect("obstruct atomic delta publication");
        let mut service =
            IndexerService::from_volume_descriptors([VolumeDescriptor::new(r"C:\", "NTFS")]);
        service.rebuild_with(|_| Ok(snapshot.clone()));
        service.poll_journals(&|snapshot| {
            let mut updated = snapshot.clone();
            updated.next_usn = Some(110);
            Ok(crate::usn::JournalPoll::CursorAdvanced(updated))
        });
        assert_eq!(service.status.phase, IndexPhase::Ready);
        assert_eq!(service.status.drives[0].next_usn, Some(110));
        let error = service
            .status
            .last_error
            .as_deref()
            .expect("real atomic publish failure visible");
        assert!(error.contains("index persistence"));
        assert!(!error.contains(directory.path().to_string_lossy().as_ref()));
        assert_eq!(
            service
                .search_snapshot(&IndexSearchRequest {
                    generation: 1,
                    query: "kept".into(),
                    limit: 20,
                    context: IndexSearchContext {
                        include_files: true,
                        include_directories: true,
                        ..Default::default()
                    },
                })
                .len(),
            1
        );
        assert!(
            base_path.is_file(),
            "failed delta write preserves last-good base"
        );
        fs::remove_dir(&blocked_delta).expect("recover delta publish target");
        let maintenance = service.poll_journals(&|_| Ok(crate::usn::JournalPoll::NoChange));
        assert!(matches!(maintenance, JournalMaintenance::StatusChanged));
        assert!(service.status.last_error.is_none());
        let restored = load_delta(&blocked_delta, Some(17)).expect("committed delta reload");
        assert_eq!(restored.next_usn, Some(110));
    }
}

#[cfg(all(test, windows))]
mod idle_io_tests {
    use super::*;

    #[allow(unsafe_code)]
    fn written_bytes() -> u64 {
        use windows_sys::Win32::System::Threading::{GetProcessIoCounters, IO_COUNTERS};
        let mut counters: IO_COUNTERS = unsafe { std::mem::zeroed() };
        assert_ne!(
            unsafe { GetProcessIoCounters(GetCurrentProcess(), &mut counters) },
            0,
            "read own process I/O counters"
        );
        counters.WriteTransferCount
    }

    #[test]
    fn idle_io_small_updates_do_not_rewrite_megabyte_overlay() {
        let directory = tempfile::tempdir().expect("isolated incremental cache");
        let base = directory.path().join("volume.idx");
        let mut snapshot = MftSnapshot::from_entries(r"C:\", Some(17), Some(100), Vec::new());
        snapshot.set_cache_path(base.clone());
        for reference in 0..12_000 {
            snapshot.overlay.replacements.insert(
                reference,
                MftEntry {
                    identity: Some(MftIdentity {
                        file_reference: reference,
                        parent_reference: 5,
                    }),
                    path: format!("C:\\{}\\file-{reference}.txt", "synthetic".repeat(16)),
                    name: format!("file-{reference}.txt"),
                    is_directory: false,
                    size_bytes: None,
                    modified_at_unix_ms: Some(1),
                }
                .into(),
            );
        }
        persist_system_delta(&snapshot).expect("initial durable overlay");
        let initial_size = fs::metadata(delta_path(&base))
            .expect("initial delta")
            .len();
        assert!(initial_size > 2 * 1024 * 1024);
        let before = written_bytes();
        for tick in 1..=3 {
            snapshot.next_usn = Some(100 + tick);
            std::sync::Arc::make_mut(
                snapshot
                    .overlay
                    .replacements
                    .get_mut(&11)
                    .expect("one changed row"),
            )
            .modified_at_unix_ms = Some(tick + 1);
            persist_system_delta(&snapshot).expect("small incremental commit");
        }
        let written = written_bytes() - before;
        eprintln!(
            "idle_io: initial_overlay_bytes={initial_size}, three_single_row_commit_bytes={written}"
        );
        assert!(
            written < 32 * 1024,
            "three one-row commits must not rewrite a {initial_size}-byte overlay: actual process writes={written}"
        );
        let restored =
            load_delta(&delta_path(&base), Some(17)).expect("durable incremental replay");
        assert_eq!(restored.next_usn, Some(103));
        assert_eq!(restored.overlay.replacements.len(), 12_000);
        assert_eq!(
            restored
                .overlay
                .replacements
                .get(&11)
                .expect("restored row")
                .modified_at_unix_ms,
            Some(4)
        );
    }
}
