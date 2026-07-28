use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result, ensure};
use nxtools_finder::search_char_mask;
use tracing::warn;

mod cache;

pub(crate) use cache::{load_system_snapshot, persist_system_delta, persist_system_snapshot};

#[derive(Clone, Debug)]
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
            .map(|(_, entry)| entry.clone()),
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
    let mut compacted = compact_snapshot(snapshot);
    persist_system_snapshot(&mut compacted)?;
    Ok(compacted)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct MftIdentity {
    pub file_reference: u64,
    pub parent_reference: u64,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct MftOverlay {
    pub replacements: HashMap<u64, MftEntry>,
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
    base: MftBase,
    owned_name_masks: Option<Arc<Vec<u64>>>,
    pub overlay: MftOverlay,
    visible_count: usize,
    reference_index: Arc<HashMap<u64, usize>>,
    children_index: Arc<HashMap<u64, Vec<u64>>>,
    cache_path: Option<PathBuf>,
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
        Self {
            mount_point: mount_point.into(),
            journal_id,
            next_usn,
            base: MftBase::Owned(Arc::new(entries)),
            owned_name_masks,
            overlay: MftOverlay::default(),
            visible_count,
            reference_index: Arc::new(reference_index),
            children_index: Arc::new(children_index),
            cache_path: None,
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
        Self {
            mount_point,
            journal_id,
            next_usn,
            base: MftBase::Mapped(Arc::new(mapped)),
            owned_name_masks: None,
            overlay: MftOverlay::default(),
            visible_count,
            reference_index: Arc::new(HashMap::new()),
            children_index: Arc::new(HashMap::new()),
            cache_path: Some(cache_path),
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
            return Some(entry.clone());
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
    file_reference: u64,
    parent_reference: u64,
    name: String,
    is_directory: bool,
}

fn snapshot_from_records(
    mount_point: &str,
    journal_id: Option<u64>,
    next_usn: Option<i64>,
    records: Vec<RawMftRecord>,
) -> MftSnapshot {
    let records = records
        .into_iter()
        .map(|record| (record.file_reference, record))
        .collect::<HashMap<_, _>>();
    let mut paths = HashMap::<u64, Option<String>>::with_capacity(records.len());
    for record in records.values() {
        if is_ntfs_root_reference(record.file_reference) {
            paths.insert(record.file_reference, Some(mount_point.into()));
        }
        if is_ntfs_root_reference(record.parent_reference) {
            paths.insert(record.parent_reference, Some(mount_point.into()));
        }
    }
    for record in records.values() {
        let mut visiting = HashSet::new();
        let _ = resolve_record_path(
            record.file_reference,
            mount_point,
            &records,
            &mut paths,
            &mut visiting,
        );
    }
    let mut entries = Vec::with_capacity(records.len());
    for record in records.values() {
        if !is_ntfs_root_reference(record.file_reference)
            && let Some(path) = paths.remove(&record.file_reference).flatten()
        {
            entries.push(MftEntry {
                identity: Some(MftIdentity {
                    file_reference: record.file_reference,
                    parent_reference: record.parent_reference,
                }),
                path,
                name: record.name.clone(),
                is_directory: record.is_directory,
                size_bytes: None,
                modified_at_unix_ms: None,
            });
        }
    }
    entries.sort_by(|left, right| left.path.cmp(&right.path));
    MftSnapshot::from_entries(mount_point, journal_id, next_usn, entries)
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
) -> Option<String> {
    if let Some(cached) = cache.get(&file_reference) {
        return cached.clone();
    }
    if !visiting.insert(file_reference) {
        cache.insert(file_reference, None);
        return None;
    }
    let record = records.get(&file_reference)?;
    let path = if record.file_reference == record.parent_reference {
        Some(mount_point.to_owned())
    } else {
        resolve_record_path(
            record.parent_reference,
            mount_point,
            records,
            cache,
            visiting,
        )
        .map(|parent| {
            let separator = if parent.ends_with(['\\', '/']) {
                ""
            } else {
                "\\"
            };
            format!("{parent}{separator}{}", record.name)
        })
    };
    visiting.remove(&file_reference);
    cache.insert(file_reference, path.clone());
    path
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
                    file_reference: read_u64(buffer, offset + 8)?,
                    parent_reference: read_u64(buffer, offset + 16)?,
                    name,
                    is_directory: read_u32(buffer, offset + 52)? & 0x10 != 0,
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

#[cfg(windows)]
pub(crate) fn scan_volume(mount_point: &str) -> Result<MftSnapshot> {
    windows::scan_volume(mount_point)
}

#[cfg(not(windows))]
pub(crate) fn scan_volume(_mount_point: &str) -> Result<MftSnapshot> {
    anyhow::bail!("NTFS MFT snapshots are available only on Windows")
}

pub(crate) fn scan_volume_and_persist(mount_point: &str) -> Result<MftSnapshot> {
    let mut snapshot = scan_volume(mount_point)?;
    if let Err(error) = persist_system_snapshot(&mut snapshot) {
        warn!(
            mount_point,
            %error,
            "MFT snapshot cache write failed; index remains usable for this session"
        );
    }
    Ok(snapshot)
}

#[cfg(windows)]
#[allow(unsafe_code)]
mod windows {
    use std::ffi::c_void;
    use std::mem::size_of;
    use std::ptr::{null, null_mut};

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

    use super::{MftSnapshot, RawMftRecord, parse_enumeration_batch, snapshot_from_records};

    const ENUMERATION_BUFFER_BYTES: usize = 1024 * 1024;

    struct VolumeHandle(HANDLE);

    impl Drop for VolumeHandle {
        fn drop(&mut self) {
            unsafe {
                let _ = CloseHandle(self.0);
            }
        }
    }

    pub(super) fn scan_volume(mount_point: &str) -> Result<MftSnapshot> {
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
        let records = enumerate_mft(handle.0, journal.NextUsn)
            .with_context(|| format!("failed to enumerate MFT records for {mount_point}"))?;
        Ok(snapshot_from_records(
            mount_point,
            Some(journal.UsnJournalID),
            Some(journal.NextUsn),
            records,
        ))
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

    fn enumerate_mft(handle: HANDLE, high_usn: i64) -> Result<Vec<RawMftRecord>> {
        let mut request = MFT_ENUM_DATA_V0 {
            StartFileReferenceNumber: 0,
            LowUsn: 0,
            HighUsn: high_usn,
        };
        let mut buffer = vec![0_u8; ENUMERATION_BUFFER_BYTES];
        let mut records = Vec::new();
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
            records.append(&mut batch);
            if next_reference <= request.StartFileReferenceNumber {
                break;
            }
            request.StartFileReferenceNumber = next_reference;
        }
        Ok(records)
    }

    fn win32_error(operation: &str, code: u32) -> anyhow::Error {
        if code == ERROR_ACCESS_DENIED {
            anyhow::anyhow!(
                "{operation} failed: Windows 拒绝原始卷访问；需要以管理员权限运行索引器 (Win32 {code})"
            )
        } else {
            anyhow::anyhow!("{operation} failed with Win32 error {code}")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mft_parent_references_reconstruct_a_descendant_path() {
        let snapshot = snapshot_from_records(
            r"C:\",
            Some(7),
            Some(11),
            vec![
                RawMftRecord {
                    file_reference: 5,
                    parent_reference: 5,
                    name: ".".into(),
                    is_directory: true,
                },
                RawMftRecord {
                    file_reference: 10,
                    parent_reference: 5,
                    name: "Work".into(),
                    is_directory: true,
                },
                RawMftRecord {
                    file_reference: 11,
                    parent_reference: 10,
                    name: "Budget.xlsx".into(),
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
            },
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
            },
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
                    file_reference: (3_u64 << 48) | 10,
                    parent_reference: root_reference,
                    name: "Work".into(),
                    is_directory: true,
                },
                RawMftRecord {
                    file_reference: (8_u64 << 48) | 11,
                    parent_reference: (3_u64 << 48) | 10,
                    name: "Budget.xlsx".into(),
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
                    file_reference: 10,
                    parent_reference: 11,
                    name: "first".into(),
                    is_directory: true,
                },
                RawMftRecord {
                    file_reference: 11,
                    parent_reference: 10,
                    name: "second".into(),
                    is_directory: true,
                },
                RawMftRecord {
                    file_reference: 12,
                    parent_reference: 99,
                    name: "orphan.txt".into(),
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
