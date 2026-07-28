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
    let candidates = cache_candidates(&directory, &prefix)?;
    if candidates.is_empty() {
        return Ok(None);
    }
    let mut opened = None;
    let mut errors = Vec::new();
    for path in candidates {
        match MappedSnapshot::open(&path, mount_point) {
            Ok(snapshot) => {
                opened = Some((path, snapshot));
                break;
            }
            Err(error) => errors.push(format!("{}: {error}", path.display())),
        }
    }
    let Some((path, (mapped, journal_id, next_usn, visible_count))) = opened else {
        bail!(
            "no valid persisted snapshot remained: {}",
            errors.join("; ")
        );
    };
    cleanup_obsolete_cache_files(&directory, &prefix, &path);
    info!(
        mount_point,
        rows = mapped.len(),
        cache = %path.display(),
        "opened persisted MFT snapshot"
    );
    let mut snapshot = MftSnapshot::from_mapped(
        mount_point.to_owned(),
        journal_id,
        next_usn,
        visible_count,
        mapped,
        path.clone(),
    );
    let delta_path = delta_path(&path);
    if delta_path.is_file() {
        match load_delta(&delta_path, journal_id) {
            Ok(delta) => {
                snapshot.restore_delta(
                    delta.journal_id,
                    delta.next_usn,
                    delta.visible_count,
                    delta.overlay,
                );
                info!(
                    mount_point,
                    replacements = snapshot.overlay.replacements.len(),
                    deleted = snapshot.overlay.deleted.len(),
                    "restored persisted MFT delta"
                );
            }
            Err(error) => warn!(
                mount_point,
                %error,
                "ignored invalid persisted MFT delta; journal catch-up will repair it"
            ),
        }
    }
    Ok(Some(snapshot))
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

pub(crate) fn persist_system_delta(snapshot: &MftSnapshot) -> Result<()> {
    let cache_path = snapshot
        .cache_path()
        .context("snapshot has no persisted base for its delta")?;
    let path = delta_path(cache_path);
    persist_delta(snapshot, &path)
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
        if removable && let Err(error) = fs::remove_file(&path) {
            warn!(cache = %path.display(), %error, "failed to remove obsolete index cache file");
        }
    }
}

struct PersistedDelta {
    journal_id: Option<u64>,
    next_usn: Option<i64>,
    visible_count: usize,
    overlay: MftOverlay,
}

fn load_delta(path: &Path, expected_journal_id: Option<u64>) -> Result<PersistedDelta> {
    let bytes = fs::read(path).with_context(|| format!("read index delta {}", path.display()))?;
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
            },
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
        overlay: MftOverlay {
            replacements,
            deleted,
        },
    })
}

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
    use tempfile::tempdir;

    use super::*;
    use crate::mft::{MftEntry, MftIdentity};

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
            },
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
