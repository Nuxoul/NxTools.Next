use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use anyhow::{Context, Result, ensure};
use nxtools_finder::search_char_mask;

#[cfg(test)]
pub(crate) mod allocation_probe;
#[cfg(any(windows, test))]
mod bulk_times;
mod cache;
#[cfg(all(test, windows))]
#[path = "mft/initial_build_tests.rs"]
mod initial_build_tests;
mod time_metadata;
#[cfg(windows)]
pub(crate) use time_metadata::read_last_write_by_id;

pub(crate) use cache::{
    cleanup_published_snapshot, load_system_snapshot, persist_system_delta, persist_system_snapshot,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MftEntry {
    pub identity: Option<MftIdentity>,
    pub path: String,
    pub name: String,
    pub is_directory: bool,
    pub size_bytes: Option<u64>,
    pub modified_at_unix_ms: Option<i64>,
}

#[derive(Clone, Copy)]
pub(crate) struct MftEntryRef<'a> {
    pub identity: Option<MftIdentity>,
    pub path: &'a str,
    pub name: &'a str,
    pub is_directory: bool,
    pub size_bytes: Option<u64>,
    pub modified_at_unix_ms: Option<i64>,
}

impl MftEntry {
    pub(crate) fn as_ref(&self) -> MftEntryRef<'_> {
        MftEntryRef {
            identity: self.identity,
            path: &self.path,
            name: &self.name,
            is_directory: self.is_directory,
            size_bytes: self.size_bytes,
            modified_at_unix_ms: self.modified_at_unix_ms,
        }
    }
}

impl MftEntryRef<'_> {
    fn to_owned(self) -> MftEntry {
        MftEntry {
            identity: self.identity,
            path: self.path.to_owned(),
            name: self.name.to_owned(),
            is_directory: self.is_directory,
            size_bytes: self.size_bytes,
            modified_at_unix_ms: self.modified_at_unix_ms,
        }
    }
}

pub(crate) fn compact_snapshot(snapshot: &MftSnapshot) -> MftSnapshot {
    let mut entries = Vec::with_capacity(snapshot.visible_len());
    for index in 0..snapshot.base_len() {
        let Some(entry) = snapshot.base_entry(index) else {
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
        entries.push(entry.to_owned());
    }
    entries.extend(
        snapshot
            .overlay
            .replacements
            .iter()
            .filter(|(reference, _)| !snapshot.overlay.deleted.contains(reference))
            .map(|(_, entry)| entry.as_ref().clone()),
    );
    entries.sort_by(|left, right| left.path.cmp(&right.path));
    MftSnapshot::from_entries(
        snapshot.mount_point.clone(),
        snapshot.journal_id,
        snapshot.next_usn,
        entries,
    )
}

pub(crate) fn compact_system_snapshot(snapshot: &MftSnapshot) -> Result<MftSnapshot> {
    compact_system_snapshot_with(snapshot, persist_system_snapshot)
}

fn compact_system_snapshot_with(
    snapshot: &MftSnapshot,
    persist: impl Fn(&mut MftSnapshot) -> Result<()>,
) -> Result<MftSnapshot> {
    let mut compacted = compact_snapshot(snapshot);
    persist(&mut compacted)?;
    cache::reopen_persisted_snapshot(compacted)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct MftIdentity {
    pub file_reference: u64,
    pub parent_reference: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct MftOverlay {
    // Snapshot and durable-checkpoint clones share unchanged immutable row strings.
    pub replacements: HashMap<u64, Arc<MftEntry>>,
    pub deleted: HashSet<u64>,
}

#[derive(Clone, Debug)]
enum MftBase {
    Owned(Arc<Vec<MftEntry>>),
    Mapped(Arc<cache::MappedSnapshot>),
}

#[derive(Clone, Debug)]
pub(crate) struct MftSnapshot {
    pub mount_point: String,
    pub journal_id: Option<u64>,
    pub next_usn: Option<i64>,
    base_next_usn: Option<i64>,
    base: MftBase,
    owned_name_masks: Option<Arc<Vec<u64>>>,
    pub(crate) search_data: Option<Arc<crate::search_data::BaseSearchData>>,
    pub overlay: MftOverlay,
    visible_count: usize,
    reference_index: Arc<HashMap<u64, usize>>,
    children_index: Arc<HashMap<u64, Vec<u64>>>,
    cache_path: Option<PathBuf>,
    delta_checkpoint: Arc<Mutex<Option<cache::DeltaCheckpoint>>>,
}

impl MftSnapshot {
    pub(crate) fn from_entries(
        mount_point: impl Into<String>,
        journal_id: Option<u64>,
        next_usn: Option<i64>,
        entries: Vec<MftEntry>,
    ) -> Self {
        let mut reference_index = HashMap::new();
        let mut children_index = HashMap::<u64, Vec<u64>>::new();
        for (index, entry) in entries.iter().enumerate() {
            if let Some(identity) = entry.identity {
                reference_index.insert(identity.file_reference, index);
                children_index
                    .entry(identity.parent_reference)
                    .or_default()
                    .push(identity.file_reference);
            }
        }
        let visible_count = entries.len();
        let owned_name_masks = Some(Arc::new(
            entries
                .iter()
                .map(|entry| search_char_mask(&entry.name))
                .collect(),
        ));
        let search_data = crate::search_data::BaseSearchData::build(entries.len(), |row| {
            entries.get(row).map(|entry| entry.name.as_str())
        })
        .map(Arc::new);
        Self {
            mount_point: mount_point.into(),
            journal_id,
            next_usn,
            base_next_usn: next_usn,
            base: MftBase::Owned(Arc::new(entries)),
            owned_name_masks,
            search_data,
            overlay: MftOverlay::default(),
            visible_count,
            reference_index: Arc::new(reference_index),
            children_index: Arc::new(children_index),
            cache_path: None,
            delta_checkpoint: Arc::new(Mutex::new(None)),
        }
    }

    // Only durable-build callers may use this unpublished representation. The writer
    // derives its own row order/masks; runtime auxiliaries belong to the final mmap.
    fn from_entries_for_persistence(
        mount_point: impl Into<String>,
        journal_id: Option<u64>,
        next_usn: Option<i64>,
        entries: Vec<MftEntry>,
    ) -> Self {
        let visible_count = entries.len();
        Self {
            mount_point: mount_point.into(),
            journal_id,
            next_usn,
            base_next_usn: next_usn,
            base: MftBase::Owned(Arc::new(entries)),
            owned_name_masks: None,
            search_data: None,
            overlay: MftOverlay::default(),
            visible_count,
            reference_index: Arc::new(HashMap::new()),
            children_index: Arc::new(HashMap::new()),
            cache_path: None,
            delta_checkpoint: Arc::new(Mutex::new(None)),
        }
    }

    fn from_mapped(
        mount_point: String,
        journal_id: Option<u64>,
        next_usn: Option<i64>,
        visible_count: usize,
        mapped: cache::MappedSnapshot,
        cache_path: PathBuf,
    ) -> Self {
        let search_data =
            crate::search_data::BaseSearchData::build(mapped.len(), |row| mapped.name(row))
                .map(Arc::new);
        Self {
            mount_point,
            journal_id,
            next_usn,
            base_next_usn: next_usn,
            base: MftBase::Mapped(Arc::new(mapped)),
            owned_name_masks: None,
            search_data,
            overlay: MftOverlay::default(),
            visible_count,
            reference_index: Arc::new(HashMap::new()),
            children_index: Arc::new(HashMap::new()),
            cache_path: Some(cache_path),
            delta_checkpoint: Arc::new(Mutex::new(None)),
        }
    }

    fn restore_delta(
        &mut self,
        journal_id: Option<u64>,
        next_usn: Option<i64>,
        visible_count: usize,
        overlay: MftOverlay,
    ) {
        self.journal_id = journal_id;
        self.next_usn = next_usn;
        self.visible_count = visible_count;
        self.overlay = overlay;
    }

    fn set_cache_path(&mut self, path: PathBuf) {
        self.cache_path = Some(path);
        self.delta_checkpoint = Arc::new(Mutex::new(None));
    }

    fn cache_path(&self) -> Option<&Path> {
        self.cache_path.as_deref()
    }

    pub(crate) fn base_len(&self) -> usize {
        match &self.base {
            MftBase::Owned(entries) => entries.len(),
            MftBase::Mapped(mapped) => mapped.len(),
        }
    }

    pub(crate) fn base_entry(&self, index: usize) -> Option<MftEntryRef<'_>> {
        match &self.base {
            MftBase::Owned(entries) => entries.get(index).map(MftEntry::as_ref),
            MftBase::Mapped(mapped) => mapped.entry(index),
        }
    }

    pub(crate) fn base_name(&self, index: usize) -> Option<&str> {
        match &self.base {
            MftBase::Owned(entries) => entries.get(index).map(|entry| entry.name.as_str()),
            MftBase::Mapped(mapped) => mapped.name(index),
        }
    }

    pub(crate) fn base_name_mask(&self, index: usize) -> Option<u64> {
        match &self.base {
            MftBase::Owned(_) => self.owned_name_masks.as_ref()?.get(index).copied(),
            MftBase::Mapped(mapped) => mapped.name_mask(index),
        }
    }

    fn owned_entries(&self) -> Option<&[MftEntry]> {
        match &self.base {
            MftBase::Owned(entries) => Some(entries),
            MftBase::Mapped(_) => None,
        }
    }

    pub(crate) fn visible_entry(&self, file_reference: u64) -> Option<MftEntry> {
        if self.overlay.deleted.contains(&file_reference) {
            return None;
        }
        if let Some(entry) = self.overlay.replacements.get(&file_reference) {
            return Some(entry.as_ref().clone());
        }
        match &self.base {
            MftBase::Owned(entries) => self
                .reference_index
                .get(&file_reference)
                .and_then(|index| entries.get(*index))
                .cloned(),
            MftBase::Mapped(mapped) => mapped
                .find_by_reference(file_reference)
                .map(MftEntryRef::to_owned),
        }
    }

    pub(crate) fn base_children(&self, parent_reference: u64) -> Vec<u64> {
        match &self.base {
            MftBase::Owned(_) => self
                .children_index
                .get(&parent_reference)
                .cloned()
                .unwrap_or_default(),
            MftBase::Mapped(mapped) => mapped.children(parent_reference),
        }
    }

    pub(crate) fn contains_base_reference(&self, file_reference: u64) -> bool {
        match &self.base {
            MftBase::Owned(_) => self.reference_index.contains_key(&file_reference),
            MftBase::Mapped(mapped) => mapped.find_by_reference(file_reference).is_some(),
        }
    }

    pub(crate) fn visible_len(&self) -> usize {
        self.visible_count
    }

    pub(crate) fn record_visible_insert(&mut self) {
        self.visible_count = self.visible_count.saturating_add(1);
    }

    pub(crate) fn record_visible_delete(&mut self) {
        self.visible_count = self.visible_count.saturating_sub(1);
    }
}

#[derive(Clone, Debug)]
struct RawMftRecord {
    last_usn: i64,
    file_reference: u64,
    parent_reference: u64,
    name: String,
    is_directory: bool,
    modified_at_unix_ms: Option<i64>,
}

#[cfg(any(windows, test))]
// Injection seam shared by production enumeration and synthetic native-boundary fixtures.
fn hydrate_record_times_while(
    records: &mut [RawMftRecord],
    mut read: impl FnMut(u64, i64) -> Result<Option<i64>>,
    is_current: &impl Fn() -> bool,
) -> Result<bool> {
    for (index, record) in records.iter_mut().enumerate() {
        if !is_current() {
            return Ok(false);
        }
        record.modified_at_unix_ms = read(record.file_reference, record.last_usn)?;
        if index.is_multiple_of(64) {
            std::thread::yield_now();
        }
    }
    Ok(is_current())
}
fn snapshot_for_persistence_from_records(
    mount_point: &str,
    journal_id: Option<u64>,
    next_usn: Option<i64>,
    records: Vec<RawMftRecord>,
) -> MftSnapshot {
    MftSnapshot::from_entries_for_persistence(
        mount_point,
        journal_id,
        next_usn,
        unsorted_entries_from_records(mount_point, records),
    )
}

#[cfg(test)]
fn snapshot_from_records(
    mount_point: &str,
    journal_id: Option<u64>,
    next_usn: Option<i64>,
    records: Vec<RawMftRecord>,
) -> MftSnapshot {
    MftSnapshot::from_entries(
        mount_point,
        journal_id,
        next_usn,
        entries_from_records(mount_point, records),
    )
}

#[cfg(test)]
fn entries_from_records(mount_point: &str, records: Vec<RawMftRecord>) -> Vec<MftEntry> {
    let mut entries = unsorted_entries_from_records(mount_point, records);
    entries.sort_by(|left, right| left.path.cmp(&right.path));
    entries
}

fn unsorted_entries_from_records(mount_point: &str, records: Vec<RawMftRecord>) -> Vec<MftEntry> {
    let records = records
        .into_iter()
        .map(|record| (record.file_reference, record))
        .collect::<HashMap<_, _>>();
    let mut paths = HashMap::<u64, Option<String>>::with_capacity(records.len());
    for record in records.values() {
        if is_ntfs_root_reference(record.file_reference) {
            paths
                .entry(record.file_reference)
                .or_insert_with(|| Some(mount_point.into()));
        }
        if is_ntfs_root_reference(record.parent_reference) {
            paths
                .entry(record.parent_reference)
                .or_insert_with(|| Some(mount_point.into()));
        }
    }
    let mut visiting = HashSet::new();
    for record in records.values() {
        resolve_record_path(
            record.file_reference,
            mount_point,
            &records,
            &mut paths,
            &mut visiting,
        );
    }
    let mut entries = Vec::with_capacity(records.len());
    for record in records.into_values() {
        if !is_ntfs_root_reference(record.file_reference)
            && let Some(path) = paths.remove(&record.file_reference).flatten()
        {
            entries.push(MftEntry {
                identity: Some(MftIdentity {
                    file_reference: record.file_reference,
                    parent_reference: record.parent_reference,
                }),
                path,
                name: record.name,
                is_directory: record.is_directory,
                size_bytes: None,
                modified_at_unix_ms: record.modified_at_unix_ms,
            });
        }
    }
    entries
}

fn is_ntfs_root_reference(file_reference: u64) -> bool {
    const MFT_RECORD_NUMBER_MASK: u64 = (1_u64 << 48) - 1;
    file_reference & MFT_RECORD_NUMBER_MASK == 5
}

fn resolve_record_path(
    file_reference: u64,
    mount_point: &str,
    records: &HashMap<u64, RawMftRecord>,
    cache: &mut HashMap<u64, Option<String>>,
    visiting: &mut HashSet<u64>,
) {
    if cache.contains_key(&file_reference) {
        return;
    }
    if !visiting.insert(file_reference) {
        cache.insert(file_reference, None);
        return;
    }
    let path = records.get(&file_reference).and_then(|record| {
        if record.file_reference == record.parent_reference {
            Some(mount_point.to_owned())
        } else {
            resolve_record_path(
                record.parent_reference,
                mount_point,
                records,
                cache,
                visiting,
            );
            cache.get(&record.parent_reference).and_then(|parent| {
                let parent = parent.as_deref()?;
                let separator = !parent.ends_with(['\\', '/']);
                let mut path = String::with_capacity(
                    parent.len() + usize::from(separator) + record.name.len(),
                );
                path.push_str(parent);
                if separator {
                    path.push('\\');
                }
                path.push_str(&record.name);
                Some(path)
            })
        }
    });
    visiting.remove(&file_reference);
    cache.insert(file_reference, path);
}

fn parse_enumeration_batch(buffer: &[u8]) -> Result<(u64, Vec<RawMftRecord>)> {
    ensure!(
        buffer.len() >= 8,
        "MFT enumeration batch omitted its continuation reference"
    );
    let next_reference = read_u64(buffer, 0)?;
    let mut records = Vec::new();
    let mut offset = 8;
    while offset < buffer.len() {
        ensure!(
            buffer.len() - offset >= 60,
            "MFT enumeration batch ended inside a USN record header"
        );
        let record_length = read_u32(buffer, offset)? as usize;
        ensure!(
            record_length >= 60,
            "USN record length was smaller than its header"
        );
        let record_end = offset
            .checked_add(record_length)
            .context("USN record length overflowed the batch")?;
        ensure!(
            record_end <= buffer.len(),
            "USN record exceeded its MFT batch"
        );
        let major_version = read_u16(buffer, offset + 4)?;
        if major_version == 2 {
            let name_length = read_u16(buffer, offset + 56)? as usize;
            let name_offset = read_u16(buffer, offset + 58)? as usize;
            ensure!(
                name_length.is_multiple_of(2),
                "USN record name was not UTF-16 aligned"
            );
            ensure!(
                name_offset >= 60 && name_offset.is_multiple_of(2),
                "USN record name offset overlapped its header or was not UTF-16 aligned"
            );
            let name_start = offset
                .checked_add(name_offset)
                .context("USN name offset overflowed the batch")?;
            let name_end = name_start
                .checked_add(name_length)
                .context("USN name length overflowed the batch")?;
            ensure!(
                name_start >= offset && name_end <= record_end,
                "USN name exceeded its record"
            );
            let utf16 = buffer[name_start..name_end]
                .chunks_exact(2)
                .map(|unit| u16::from_le_bytes([unit[0], unit[1]]))
                .collect::<Vec<_>>();
            let name = String::from_utf16_lossy(&utf16);
            if !name.is_empty() {
                records.push(RawMftRecord {
                    last_usn: read_u64(buffer, offset + 24)? as i64,
                    file_reference: read_u64(buffer, offset + 8)?,
                    parent_reference: read_u64(buffer, offset + 16)?,
                    name,
                    is_directory: read_u32(buffer, offset + 52)? & 0x10 != 0,
                    modified_at_unix_ms: None,
                });
            }
        }
        offset = record_end;
    }
    Ok((next_reference, records))
}

fn read_u16(buffer: &[u8], offset: usize) -> Result<u16> {
    let bytes = buffer
        .get(offset..offset + 2)
        .context("missing little-endian u16 in MFT data")?;
    Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
}

fn read_u32(buffer: &[u8], offset: usize) -> Result<u32> {
    let bytes = buffer
        .get(offset..offset + 4)
        .context("missing little-endian u32 in MFT data")?;
    Ok(u32::from_le_bytes(
        bytes.try_into().expect("four-byte slice"),
    ))
}

fn read_u64(buffer: &[u8], offset: usize) -> Result<u64> {
    let bytes = buffer
        .get(offset..offset + 8)
        .context("missing little-endian u64 in MFT data")?;
    Ok(u64::from_le_bytes(
        bytes.try_into().expect("eight-byte slice"),
    ))
}

type SnapshotConstructor = fn(&str, Option<u64>, Option<i64>, Vec<RawMftRecord>) -> MftSnapshot;

#[cfg(any(windows, test))]
fn enumerate_initial_snapshot(
    mount_point: &str,
    journal_id: u64,
    start_usn: i64,
    enumerate: impl FnOnce(i64) -> Result<Vec<RawMftRecord>>,
    construct: SnapshotConstructor,
) -> Result<MftSnapshot> {
    // Keep the pre-scan cursor for replay, but include rows whose latest USN changes
    // during enumeration; metadata-only catch-up cannot create an omitted base row.
    let started = Instant::now();
    let records = enumerate(i64::MAX)
        .with_context(|| format!("failed to enumerate MFT records for {mount_point}"))?;
    let raw_rows = records.len();
    tracing::info!(
        event = "indexer.snapshot.enumeration",
        rows = raw_rows,
        elapsed_ms = started.elapsed().as_millis() as u64,
        "MFT enumeration completed"
    );
    let constructed = Instant::now();
    let snapshot = construct(mount_point, Some(journal_id), Some(start_usn), records);
    tracing::info!(
        event = "indexer.snapshot.construction",
        raw_rows,
        rows = snapshot.visible_len(),
        elapsed_ms = constructed.elapsed().as_millis() as u64,
        "MFT snapshot constructed"
    );
    Ok(snapshot)
}

#[cfg(all(windows, test))]
pub(crate) fn scan_volume(mount_point: &str) -> Result<MftSnapshot> {
    windows::scan_volume(mount_point, snapshot_from_records)
}

#[cfg(not(windows))]
pub(crate) fn scan_volume(_mount_point: &str) -> Result<MftSnapshot> {
    anyhow::bail!("NTFS MFT snapshots are available only on Windows")
}

pub(crate) fn scan_volume_and_persist(mount_point: &str) -> Result<MftSnapshot> {
    #[cfg(windows)]
    let scan = windows::scan_volume;
    #[cfg(not(windows))]
    let scan = |mount: &str, _: SnapshotConstructor| scan_volume(mount);
    scan_volume_and_persist_with(mount_point, scan, persist_system_snapshot)
}

fn scan_volume_and_persist_with(
    mount_point: &str,
    scan: impl Fn(&str, SnapshotConstructor) -> Result<MftSnapshot>,
    persist: impl Fn(&mut MftSnapshot) -> Result<()>,
) -> Result<MftSnapshot> {
    let started = Instant::now();
    let mut snapshot = scan(mount_point, snapshot_for_persistence_from_records)?;
    tracing::info!(
        event = "indexer.snapshot.scan",
        rows = snapshot.visible_len(),
        elapsed_ms = started.elapsed().as_millis() as u64,
        "volume scan completed"
    );
    let persisted = Instant::now();
    persist(&mut snapshot).context("persist completed MFT snapshot")?;
    tracing::info!(
        event = "indexer.snapshot.persist",
        rows = snapshot.visible_len(),
        elapsed_ms = persisted.elapsed().as_millis() as u64,
        "volume base persisted"
    );
    let reopened = Instant::now();
    let mapped = cache::reopen_persisted_snapshot(snapshot)?;
    tracing::info!(
        event = "indexer.snapshot.reopen",
        rows = mapped.visible_len(),
        elapsed_ms = reopened.elapsed().as_millis() as u64,
        "volume base mapped and reopened"
    );
    Ok(mapped)
}

#[cfg(windows)]
#[allow(unsafe_code)]
mod windows {
    use std::ffi::c_void;
    use std::mem::size_of;
    use std::ptr::{null, null_mut};
    use std::time::Instant;

    use anyhow::{Context, Result, ensure};
    use windows_sys::Win32::Foundation::{
        CloseHandle, ERROR_ACCESS_DENIED, ERROR_HANDLE_EOF, ERROR_NO_MORE_FILES, GENERIC_READ,
        GetLastError, HANDLE, INVALID_HANDLE_VALUE,
    };
    use windows_sys::Win32::Storage::FileSystem::{
        CreateFileW, FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
    };
    use windows_sys::Win32::System::IO::DeviceIoControl;
    use windows_sys::Win32::System::Ioctl::{
        FSCTL_ENUM_USN_DATA, FSCTL_QUERY_USN_JOURNAL, MFT_ENUM_DATA_V0, USN_JOURNAL_DATA_V0,
    };

    use super::{
        MftSnapshot, RawMftRecord, SnapshotConstructor, enumerate_initial_snapshot,
        hydrate_record_times_while, parse_enumeration_batch,
    };

    const ENUMERATION_BUFFER_BYTES: usize = 1024 * 1024;

    struct VolumeHandle(HANDLE);

    impl Drop for VolumeHandle {
        fn drop(&mut self) {
            unsafe {
                let _ = CloseHandle(self.0);
            }
        }
    }

    pub(super) fn scan_volume(
        mount_point: &str,
        construct: SnapshotConstructor,
    ) -> Result<MftSnapshot> {
        let device_path = volume_device_path(mount_point)?;
        let wide_path = device_path
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect::<Vec<_>>();
        let raw_handle = unsafe {
            CreateFileW(
                wide_path.as_ptr(),
                GENERIC_READ,
                FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                null(),
                OPEN_EXISTING,
                0,
                null_mut(),
            )
        };
        if raw_handle == INVALID_HANDLE_VALUE {
            return Err(win32_error("open NTFS volume", unsafe { GetLastError() }));
        }
        let handle = VolumeHandle(raw_handle);
        let journal = query_journal(handle.0)?;
        enumerate_initial_snapshot(
            mount_point,
            journal.UsnJournalID,
            journal.NextUsn,
            |high_usn| enumerate_mft(handle.0, mount_point, high_usn),
            construct,
        )
    }

    fn volume_device_path(mount_point: &str) -> Result<String> {
        let mut characters = mount_point.chars();
        let drive = characters
            .next()
            .context("NTFS mount point omitted a drive letter")?;
        ensure!(
            drive.is_ascii_alphabetic() && characters.next() == Some(':'),
            "NTFS MFT snapshot currently requires a drive-letter mount point"
        );
        Ok(format!(r"\\.\{}:", drive.to_ascii_uppercase()))
    }

    fn query_journal(handle: HANDLE) -> Result<USN_JOURNAL_DATA_V0> {
        let mut journal = USN_JOURNAL_DATA_V0::default();
        let mut returned = 0_u32;
        let success = unsafe {
            DeviceIoControl(
                handle,
                FSCTL_QUERY_USN_JOURNAL,
                null(),
                0,
                (&raw mut journal).cast::<c_void>(),
                size_of::<USN_JOURNAL_DATA_V0>() as u32,
                &raw mut returned,
                null_mut(),
            )
        };
        if success == 0 {
            return Err(win32_error("query NTFS USN journal", unsafe {
                GetLastError()
            }));
        }
        ensure!(
            returned as usize >= size_of::<USN_JOURNAL_DATA_V0>(),
            "USN journal query returned a truncated structure"
        );
        Ok(journal)
    }

    fn enumerate_mft(
        handle: HANDLE,
        mount_point: &str,
        high_usn: i64,
    ) -> Result<Vec<RawMftRecord>> {
        let mut times = super::bulk_times::open(handle, mount_point)?;
        let mut request = MFT_ENUM_DATA_V0 {
            StartFileReferenceNumber: 0,
            LowUsn: 0,
            HighUsn: high_usn,
        };
        let mut buffer = vec![0_u8; ENUMERATION_BUFFER_BYTES];
        let mut records = Vec::new();
        let mut metadata_elapsed = std::time::Duration::ZERO;
        let mut metadata_rows = 0_usize;
        let started = Instant::now();
        loop {
            let mut returned = 0_u32;
            let success = unsafe {
                DeviceIoControl(
                    handle,
                    FSCTL_ENUM_USN_DATA,
                    (&raw const request).cast::<c_void>(),
                    size_of::<MFT_ENUM_DATA_V0>() as u32,
                    buffer.as_mut_ptr().cast::<c_void>(),
                    buffer.len() as u32,
                    &raw mut returned,
                    null_mut(),
                )
            };
            if success == 0 {
                let error = unsafe { GetLastError() };
                if error == ERROR_HANDLE_EOF || error == ERROR_NO_MORE_FILES {
                    break;
                }
                return Err(win32_error("enumerate NTFS MFT", error));
            }
            ensure!(
                returned as usize <= buffer.len(),
                "MFT enumeration overflowed its buffer"
            );
            let (next_reference, mut batch) =
                parse_enumeration_batch(&buffer[..returned as usize])?;
            ensure!(
                next_reference > request.StartFileReferenceNumber || batch.is_empty(),
                "MFT enumeration did not advance its continuation reference"
            );
            // USN event time is not LastWriteTime. Read matching base FILE records in
            // bounded raw blocks on this worker, never from interactive search.
            let metadata_started = Instant::now();
            metadata_rows += batch.len();
            let _ = hydrate_record_times_while(
                &mut batch,
                |reference, usn| times.last_write(reference, usn),
                &|| true,
            )?;
            metadata_elapsed += metadata_started.elapsed();
            records.append(&mut batch);
            if next_reference <= request.StartFileReferenceNumber {
                break;
            }
            request.StartFileReferenceNumber = next_reference;
        }
        tracing::info!(
            event = "indexer.snapshot.native_enumeration",
            rows = records.len(),
            metadata_rows,
            metadata_mode = "bulk_mft",
            metadata_reads = times.reads,
            metadata_unknown = times.unknown,
            metadata_fallback_reads = times.fallback_reads,
            metadata_elapsed_ms = metadata_elapsed.as_millis() as u64,
            elapsed_ms = started.elapsed().as_millis() as u64,
            "native MFT enumeration and LastWriteTime hydration completed"
        );
        Ok(records)
    }

    pub(super) fn win32_error(operation: &str, code: u32) -> anyhow::Error {
        let context = if code == ERROR_ACCESS_DENIED {
            format!(
                "{operation} failed: Windows 拒绝原始卷访问；需要以管理员权限运行索引器 (Win32 {code})"
            )
        } else {
            format!("{operation} failed with Win32 error {code}")
        };
        anyhow::Error::new(std::io::Error::from_raw_os_error(code as i32)).context(context)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn u4_snapshot_persistence_failure_keeps_last_good_and_ends_as_failure() {
        let mut service =
            crate::IndexerService::from_volume_descriptors([crate::VolumeDescriptor::new(
                r"C:\", "NTFS",
            )]);
        service.rebuild_with(|mount| {
            Ok(MftSnapshot::from_entries(
                mount,
                Some(17),
                Some(100),
                Vec::new(),
            ))
        });
        service.rebuild_with(|mount| {
            scan_volume_and_persist_with(
                mount,
                |mount, _| {
                    Ok(MftSnapshot::from_entries(
                        mount,
                        Some(17),
                        Some(200),
                        Vec::new(),
                    ))
                },
                |_| Err(std::io::Error::from_raw_os_error(112).into()),
            )
        });
        assert!(
            service.status().last_error.is_some(),
            "failed durable snapshot publication must not report success"
        );
        assert_eq!(
            service.snapshots[0].next_usn,
            Some(100),
            "failed persistence preserves last-good publication"
        );
        assert!(service.maintenance.is_empty());
    }

    #[cfg(windows)]
    #[test]
    fn u4_native_scan_failure_retains_its_error_code() {
        let failure = windows::win32_error("synthetic enumeration", 5);
        assert_eq!(
            crate::persistence_native_error(&failure),
            Some(5),
            "native scan diagnostics retain the owning boundary's error code"
        );
    }

    fn changing_enumeration() -> MftSnapshot {
        enumerate_initial_snapshot(
            r"C:\",
            17,
            100,
            |high_usn| {
                Ok([
                    (110, 10, 5, "changing.txt", false),
                    (111, 20, 5, "ChangingDirectory", true),
                    (90, 21, 20, "child.txt", false),
                    (112, 30, 5, "created.txt", false),
                ]
                .into_iter()
                .filter(|(latest_usn, ..)| *latest_usn <= high_usn)
                .map(
                    |(_, file_reference, parent_reference, name, is_directory)| RawMftRecord {
                        last_usn: 0,
                        file_reference,
                        parent_reference,
                        name: name.into(),
                        is_directory,
                        modified_at_unix_ms: Some(1),
                    },
                )
                .collect())
            },
            snapshot_from_records,
        )
        .expect("synthetic production enumeration")
    }

    fn replay_scan_changes(snapshot: &MftSnapshot) -> MftSnapshot {
        use crate::usn::{JournalBatch, UsnRecord, apply_journal_batch};
        apply_journal_batch(
            snapshot,
            JournalBatch {
                journal_id: 17,
                lowest_valid_usn: 90,
                next_usn: 120,
                records: vec![
                    UsnRecord::metadata(10, 2),
                    UsnRecord::metadata(20, 3),
                    UsnRecord::upsert(30, 5, "created.txt", false),
                    UsnRecord::metadata(30, 4),
                    UsnRecord::upsert(30, 5, "created.txt", false),
                    UsnRecord::metadata(99, 5),
                ],
            },
        )
        .expect("catch up from the pre-enumeration cursor")
    }

    #[test]
    fn h3_scan_modified_preexisting_file_survives_metadata_only_replay() {
        let initial = changing_enumeration();
        assert_eq!(initial.journal_id, Some(17));
        assert_eq!(initial.next_usn, Some(100));
        let updated = replay_scan_changes(&initial);
        let file = updated
            .visible_entry(10)
            .expect("preexisting modified file must remain visible");
        assert_eq!(file.path, r"C:\changing.txt");
        assert_eq!(file.modified_at_unix_ms, Some(2));
        assert_eq!(updated.next_usn, Some(120));
    }

    #[test]
    fn h3_scan_modified_directory_preserves_itself_and_unchanged_descendant() {
        let updated = replay_scan_changes(&changing_enumeration());
        assert!(
            updated
                .visible_entry(20)
                .expect("modified directory must be enumerated")
                .is_directory
        );
        assert_eq!(
            updated.visible_entry(21).expect("unchanged child").path,
            r"C:\ChangingDirectory\child.txt"
        );
    }

    #[test]
    fn h3_scan_create_and_modify_replay_is_idempotent_without_unknown_metadata_resurrection() {
        let first = replay_scan_changes(&changing_enumeration());
        let second = replay_scan_changes(&first);
        assert_eq!(second.visible_len(), first.visible_len());
        assert_eq!(second.overlay, first.overlay);
        assert_eq!(
            second
                .visible_entry(30)
                .expect("created once")
                .modified_at_unix_ms,
            Some(4)
        );
        assert!(second.visible_entry(99).is_none());
    }

    #[test]
    fn w4_mft_metadata_seam_publishes_true_time_and_keeps_failure_unknown() {
        let mut records = vec![
            RawMftRecord {
                last_usn: 0,
                file_reference: (3_u64 << 48) | 10,
                parent_reference: 5,
                name: "notes.txt".into(),
                is_directory: false,
                modified_at_unix_ms: None,
            },
            RawMftRecord {
                last_usn: 0,
                file_reference: 11,
                parent_reference: 5,
                name: "denied.txt".into(),
                is_directory: false,
                modified_at_unix_ms: None,
            },
        ];
        let mut seen = Vec::new();
        assert!(
            hydrate_record_times_while(
                &mut records,
                |reference, _| {
                    seen.push(reference);
                    Ok((reference == ((3_u64 << 48) | 10)).then_some(1234))
                },
                &|| true
            )
            .expect("metadata read")
        );
        assert_eq!(seen, [(3_u64 << 48) | 10, 11]);
        let snapshot = snapshot_from_records("C:\\", Some(17), Some(100), records);
        assert_eq!(
            snapshot
                .visible_entry((3_u64 << 48) | 10)
                .expect("file")
                .modified_at_unix_ms,
            Some(1234)
        );
        assert_eq!(
            snapshot
                .visible_entry(11)
                .expect("unknown metadata file")
                .modified_at_unix_ms,
            None
        );
        assert_eq!(snapshot.next_usn, Some(100));
    }

    #[test]
    fn w4_mft_metadata_seam_cancel_reads_nothing() {
        let mut records = vec![RawMftRecord {
            last_usn: 0,
            file_reference: 10,
            parent_reference: 5,
            name: "notes.txt".into(),
            is_directory: false,
            modified_at_unix_ms: None,
        }];
        assert!(
            !hydrate_record_times_while(
                &mut records,
                |_, _| panic!("cancel must not read"),
                &|| { false }
            )
            .expect("cancelled metadata read")
        );
        assert_eq!(records[0].modified_at_unix_ms, None);
    }

    #[test]
    fn mft_parent_references_reconstruct_a_descendant_path() {
        let snapshot = snapshot_from_records(
            r"C:\",
            Some(7),
            Some(11),
            vec![
                RawMftRecord {
                    last_usn: 0,
                    file_reference: 5,
                    parent_reference: 5,
                    name: ".".into(),
                    modified_at_unix_ms: None,
                    is_directory: true,
                },
                RawMftRecord {
                    last_usn: 0,
                    file_reference: 10,
                    parent_reference: 5,
                    name: "Work".into(),
                    modified_at_unix_ms: None,
                    is_directory: true,
                },
                RawMftRecord {
                    last_usn: 0,
                    file_reference: 11,
                    parent_reference: 10,
                    name: "Budget.xlsx".into(),
                    modified_at_unix_ms: None,
                    is_directory: false,
                },
            ],
        );

        assert!((0..snapshot.base_len()).any(|index| {
            snapshot
                .base_entry(index)
                .is_some_and(|entry| entry.path == r"C:\Work\Budget.xlsx" && !entry.is_directory)
        }));
        assert_eq!(snapshot.journal_id, Some(7));
        assert_eq!(snapshot.next_usn, Some(11));
    }

    #[test]
    fn compaction_folds_live_overlay_rows_into_a_new_searchable_base() {
        let mut snapshot = MftSnapshot::from_entries(
            r"C:\",
            Some(7),
            Some(40),
            vec![
                MftEntry {
                    identity: Some(MftIdentity {
                        file_reference: 10,
                        parent_reference: 5,
                    }),
                    path: r"C:\old.txt".into(),
                    name: "old.txt".into(),
                    is_directory: false,
                    size_bytes: None,
                    modified_at_unix_ms: None,
                },
                MftEntry {
                    identity: Some(MftIdentity {
                        file_reference: 11,
                        parent_reference: 5,
                    }),
                    path: r"C:\deleted.txt".into(),
                    name: "deleted.txt".into(),
                    is_directory: false,
                    size_bytes: None,
                    modified_at_unix_ms: None,
                },
            ],
        );
        snapshot.overlay.deleted.insert(11);
        snapshot.overlay.replacements.insert(
            10,
            MftEntry {
                identity: Some(MftIdentity {
                    file_reference: 10,
                    parent_reference: 5,
                }),
                path: r"C:\renamed.txt".into(),
                name: "renamed.txt".into(),
                is_directory: false,
                size_bytes: None,
                modified_at_unix_ms: None,
            }
            .into(),
        );
        snapshot.overlay.replacements.insert(
            12,
            MftEntry {
                identity: Some(MftIdentity {
                    file_reference: 12,
                    parent_reference: 5,
                }),
                path: r"C:\created.txt".into(),
                name: "created.txt".into(),
                is_directory: false,
                size_bytes: None,
                modified_at_unix_ms: None,
            }
            .into(),
        );

        let compacted = compact_snapshot(&snapshot);
        let names = (0..compacted.base_len())
            .filter_map(|index| compacted.base_entry(index))
            .map(|entry| entry.name.to_owned())
            .collect::<Vec<_>>();

        assert_eq!(compacted.next_usn, Some(40));
        assert_eq!(compacted.visible_len(), 2);
        assert!(compacted.overlay.replacements.is_empty());
        assert!(compacted.overlay.deleted.is_empty());
        assert_eq!(names, vec!["created.txt", "renamed.txt"]);
    }

    #[test]
    fn mft_paths_resolve_when_the_root_record_is_not_enumerated() {
        let root_reference = (5_u64 << 48) | 5;
        let snapshot = snapshot_from_records(
            r"C:\",
            Some(7),
            Some(11),
            vec![
                RawMftRecord {
                    last_usn: 0,
                    file_reference: (3_u64 << 48) | 10,
                    parent_reference: root_reference,
                    name: "Work".into(),
                    modified_at_unix_ms: None,
                    is_directory: true,
                },
                RawMftRecord {
                    last_usn: 0,
                    file_reference: (8_u64 << 48) | 11,
                    parent_reference: (3_u64 << 48) | 10,
                    name: "Budget.xlsx".into(),
                    modified_at_unix_ms: None,
                    is_directory: false,
                },
            ],
        );

        assert!((0..snapshot.base_len()).any(|index| {
            snapshot
                .base_entry(index)
                .is_some_and(|entry| entry.path == r"C:\Work\Budget.xlsx" && !entry.is_directory)
        }));
    }

    #[test]
    fn mft_records_with_a_missing_or_cyclic_parent_are_not_published() {
        let snapshot = snapshot_from_records(
            r"C:\",
            None,
            None,
            vec![
                RawMftRecord {
                    last_usn: 0,
                    file_reference: 10,
                    parent_reference: 11,
                    name: "first".into(),
                    modified_at_unix_ms: None,
                    is_directory: true,
                },
                RawMftRecord {
                    last_usn: 0,
                    file_reference: 11,
                    parent_reference: 10,
                    name: "second".into(),
                    modified_at_unix_ms: None,
                    is_directory: true,
                },
                RawMftRecord {
                    last_usn: 0,
                    file_reference: 12,
                    parent_reference: 99,
                    name: "orphan.txt".into(),
                    modified_at_unix_ms: None,
                    is_directory: false,
                },
            ],
        );

        assert_eq!(snapshot.base_len(), 0);
    }

    #[test]
    fn a_usn_v2_batch_uses_runtime_offsets_to_decode_its_unicode_name() {
        let name = "计划.txt".encode_utf16().collect::<Vec<_>>();
        let record_length = 64 + name.len() * 2;
        let mut batch = vec![0_u8; 8 + record_length];
        batch[..8].copy_from_slice(&99_u64.to_le_bytes());
        let record = &mut batch[8..];
        record[..4].copy_from_slice(&(record_length as u32).to_le_bytes());
        record[4..6].copy_from_slice(&2_u16.to_le_bytes());
        record[8..16].copy_from_slice(&44_u64.to_le_bytes());
        record[16..24].copy_from_slice(&5_u64.to_le_bytes());
        record[52..56].copy_from_slice(&0_u32.to_le_bytes());
        record[56..58].copy_from_slice(&((name.len() * 2) as u16).to_le_bytes());
        record[58..60].copy_from_slice(&64_u16.to_le_bytes());
        for (index, unit) in name.iter().enumerate() {
            let offset = 64 + index * 2;
            record[offset..offset + 2].copy_from_slice(&unit.to_le_bytes());
        }

        let (next_reference, records) =
            parse_enumeration_batch(&batch).expect("valid USN v2 batch");

        assert_eq!(next_reference, 99);
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].file_reference, 44);
        assert_eq!(records[0].parent_reference, 5);
        assert_eq!(records[0].name, "计划.txt");
    }

    #[test]
    fn a_truncated_usn_record_rejects_the_entire_batch() {
        let mut batch = vec![0_u8; 68];
        batch[8..12].copy_from_slice(&80_u32.to_le_bytes());
        batch[12..14].copy_from_slice(&2_u16.to_le_bytes());

        let error = parse_enumeration_batch(&batch).expect_err("truncated record must fail");

        assert!(error.to_string().contains("exceeded its MFT batch"));
    }

    #[test]
    fn a_usn_name_offset_cannot_point_back_into_the_record_header() {
        let mut batch = vec![0_u8; 8 + 64];
        batch[..8].copy_from_slice(&99_u64.to_le_bytes());
        let record = &mut batch[8..];
        record[..4].copy_from_slice(&64_u32.to_le_bytes());
        record[4..6].copy_from_slice(&2_u16.to_le_bytes());
        record[56..58].copy_from_slice(&2_u16.to_le_bytes());
        record[58..60].copy_from_slice(&58_u16.to_le_bytes());

        let error = parse_enumeration_batch(&batch).expect_err("header overlap must fail");

        assert!(error.to_string().contains("offset"));
    }

    #[cfg(windows)]
    #[test]
    fn the_real_windows_mft_boundary_is_either_searchable_or_actionably_denied() {
        let executable = std::env::current_exe().expect("current test executable");
        let mount_point = executable
            .components()
            .next()
            .map(|component| format!("{}\\", component.as_os_str().to_string_lossy()))
            .expect("test executable drive");

        match scan_volume(&mount_point) {
            Ok(snapshot) => {
                let file_name = executable
                    .file_name()
                    .and_then(|name| name.to_str())
                    .expect("UTF-8 test executable name");
                assert!((0..snapshot.base_len()).any(|index| {
                    snapshot
                        .base_entry(index)
                        .is_some_and(|entry| entry.name == file_name)
                }));
            }
            Err(error) => {
                let message = error.to_string();
                assert!(
                    message.contains("管理员权限") && message.contains("Win32 5"),
                    "unexpected MFT boundary failure: {message}"
                );
            }
        }
    }
}
