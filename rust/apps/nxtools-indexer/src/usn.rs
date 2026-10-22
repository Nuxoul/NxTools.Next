use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt;

use anyhow::{Context, Result, ensure};

use crate::mft::{MftEntry, MftIdentity, MftSnapshot};

const USN_REASON_FILE_DELETE: u32 = 0x0000_0200;
const USN_REASON_RENAME_OLD_NAME: u32 = 0x0000_1000;
const USN_REASON_RENAME_NEW_NAME: u32 = 0x0000_2000;
const USN_REASON_FILE_CREATE: u32 = 0x0000_0100;
const STRUCTURAL_REASON_MASK: u32 = USN_REASON_FILE_CREATE
    | USN_REASON_FILE_DELETE
    | USN_REASON_RENAME_OLD_NAME
    | USN_REASON_RENAME_NEW_NAME;
const USN_REASON_DATA_OVERWRITE: u32 = 0x0000_0001;
const USN_REASON_DATA_EXTEND: u32 = 0x0000_0002;
const USN_REASON_DATA_TRUNCATION: u32 = 0x0000_0004;
const USN_REASON_BASIC_INFO_CHANGE: u32 = 0x0000_8000;
const USN_REASON_CLOSE: u32 = 0x8000_0000;
const METADATA_REASON_MASK: u32 = USN_REASON_DATA_OVERWRITE
    | USN_REASON_DATA_EXTEND
    | USN_REASON_DATA_TRUNCATION
    | USN_REASON_BASIC_INFO_CHANGE
    | USN_REASON_CLOSE;
const MAINTENANCE_REASON_MASK: u32 = STRUCTURAL_REASON_MASK | METADATA_REASON_MASK;
const MAX_METADATA_READS_PER_POLL: usize = 256;
const MIN_OVERLAY_REBUILD_ENTRIES: usize = 4_096;
const OVERLAY_REBUILD_BASE_DIVISOR: usize = 10;
const MAX_RECURSIVE_WORK_UNITS_PER_BATCH: usize = 4_096;
const ERROR_JOURNAL_ENTRY_DELETED_CODE: u32 = 1181;

#[derive(Clone, Debug)]
pub(crate) struct UsnRecord {
    file_reference: u64,
    parent_reference: u64,
    name: String,
    is_directory: bool,
    kind: UsnRecordKind,
    // Outer None means not read (synthetic structural fixture); Some(None) means unknown.
    modified_at_unix_ms: Option<Option<i64>>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum UsnRecordKind {
    Upsert,
    Metadata,
    RenameOld,
    Delete,
}

impl UsnRecord {
    #[cfg(test)]
    pub(crate) fn upsert(
        file_reference: u64,
        parent_reference: u64,
        name: impl Into<String>,
        is_directory: bool,
    ) -> Self {
        Self {
            file_reference,
            parent_reference,
            name: name.into(),
            is_directory,
            kind: UsnRecordKind::Upsert,
            modified_at_unix_ms: None,
        }
    }

    #[cfg(test)]
    pub(crate) fn delete(file_reference: u64) -> Self {
        Self {
            file_reference,
            parent_reference: 0,
            name: String::new(),
            is_directory: false,
            kind: UsnRecordKind::Delete,
            modified_at_unix_ms: None,
        }
    }
}

#[cfg(any(windows, test))]
fn hydrate_journal_times_while(
    records: &mut [UsnRecord],
    mut read: impl FnMut(u64) -> Option<i64>,
    is_current: &impl Fn() -> bool,
) -> bool {
    let mut times = HashMap::new();
    for record in records {
        if !is_current() {
            return false;
        }
        if matches!(record.kind, UsnRecordKind::Upsert | UsnRecordKind::Metadata) {
            let time = *times
                .entry(record.file_reference)
                .or_insert_with(|| read(record.file_reference));
            record.modified_at_unix_ms = Some(time);
        }
    }
    is_current()
}

// The buffer has already passed parse_journal_records; still use checked bounds here.
#[cfg(any(windows, test))]
fn journal_record_usn_at(buffer: &[u8], index: usize) -> Result<i64> {
    let mut offset = 8_usize;
    for _ in 0..index {
        offset = offset
            .checked_add(read_u32(buffer, offset)? as usize)
            .context("USN record offset overflowed")?;
    }
    read_i64(buffer, offset + 24)
}

#[cfg(any(windows, test))]
fn metadata_batch_prefix(previous: &[UsnRecord], batch: &[UsnRecord], limit: usize) -> usize {
    let needs_time =
        |record: &UsnRecord| matches!(record.kind, UsnRecordKind::Upsert | UsnRecordKind::Metadata);
    let mut unique = previous
        .iter()
        .filter(|record| needs_time(record))
        .map(|record| record.file_reference)
        .collect::<HashSet<_>>();
    for (index, record) in batch.iter().enumerate() {
        if needs_time(record) {
            if !unique.contains(&record.file_reference) && unique.len() >= limit {
                return index;
            }
            unique.insert(record.file_reference);
        }
    }
    batch.len()
}

#[derive(Clone, Debug)]
pub(crate) struct JournalBatch {
    pub journal_id: u64,
    pub lowest_valid_usn: i64,
    pub next_usn: i64,
    pub records: Vec<UsnRecord>,
}

#[derive(Debug)]
pub(crate) enum JournalApplyError {
    RebuildRequired(String),
}

impl fmt::Display for JournalApplyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RebuildRequired(reason) => formatter.write_str(reason),
        }
    }
}

impl std::error::Error for JournalApplyError {}

#[derive(Debug)]
pub(crate) enum JournalPoll {
    NoChange,
    CursorAdvanced(MftSnapshot),
    Updated(MftSnapshot),
    RebuildRequired(String),
}

fn classify_applied_update(previous: &MftSnapshot, updated: MftSnapshot) -> JournalPoll {
    if previous.overlay != updated.overlay || previous.visible_len() != updated.visible_len() {
        JournalPoll::Updated(updated)
    } else if previous.next_usn != updated.next_usn {
        JournalPoll::CursorAdvanced(updated)
    } else {
        JournalPoll::NoChange
    }
}

pub(crate) fn apply_journal_batch(
    snapshot: &MftSnapshot,
    batch: JournalBatch,
) -> std::result::Result<MftSnapshot, JournalApplyError> {
    let Some(expected_journal) = snapshot.journal_id else {
        return Err(JournalApplyError::RebuildRequired(format!(
            "{} has no cached journal identity",
            snapshot.mount_point
        )));
    };
    let Some(start_usn) = snapshot.next_usn else {
        return Err(JournalApplyError::RebuildRequired(format!(
            "{} has no cached USN cursor",
            snapshot.mount_point
        )));
    };
    if expected_journal != batch.journal_id {
        return Err(JournalApplyError::RebuildRequired(format!(
            "{} journal changed from {} to {}",
            snapshot.mount_point, expected_journal, batch.journal_id
        )));
    }
    if start_usn < batch.lowest_valid_usn || start_usn > batch.next_usn {
        return Err(JournalApplyError::RebuildRequired(format!(
            "{} USN cursor {} is outside {}..{}",
            snapshot.mount_point, start_usn, batch.lowest_valid_usn, batch.next_usn
        )));
    }

    let mut updated = snapshot.clone();
    let mut changed_directories = Vec::new();
    let mut upserts = Vec::new();
    let mut seen_directories = HashSet::new();
    let mut seen_upserts = HashSet::new();
    let mut live_work = 0_usize;
    let mut overlay_children = overlay_children_index(&updated);
    for record in batch.records {
        match record.kind {
            UsnRecordKind::Metadata => {
                // A close/data/basic-info event must not turn a split rename's old name into
                // a structural upsert, resurrect a deleted row or expand directory descendants.
                if let Some(mut previous) = updated.visible_entry(record.file_reference) {
                    let time = record
                        .modified_at_unix_ms
                        .unwrap_or(previous.modified_at_unix_ms);
                    if previous.modified_at_unix_ms != time {
                        previous.modified_at_unix_ms = time;
                        updated
                            .overlay
                            .replacements
                            .insert(record.file_reference, previous);
                    }
                }
            }
            // A rename's old and new records may straddle read buffers. Keeping the old row until
            // RENAME_NEW arrives avoids publishing a half-applied directory tree.
            UsnRecordKind::RenameOld => {}
            UsnRecordKind::Delete => {
                delete_visible_subtree(
                    &mut updated,
                    record.file_reference,
                    &overlay_children,
                    &mut live_work,
                )?;
            }
            UsnRecordKind::Upsert => {
                let was_visible = updated.visible_entry(record.file_reference).is_some();
                let previous = updated.visible_entry(record.file_reference);
                let path_changed = previous.as_ref().is_none_or(|entry| {
                    entry.name != record.name
                        || entry.identity.is_none_or(|identity| {
                            identity.parent_reference != record.parent_reference
                        })
                });
                let replacement = MftEntry {
                    identity: Some(MftIdentity {
                        file_reference: record.file_reference,
                        parent_reference: record.parent_reference,
                    }),
                    path: String::new(),
                    name: record.name,
                    is_directory: record.is_directory,
                    size_bytes: previous.as_ref().and_then(|entry| entry.size_bytes),
                    modified_at_unix_ms: record.modified_at_unix_ms.unwrap_or_else(|| {
                        previous
                            .as_ref()
                            .and_then(|entry| entry.modified_at_unix_ms)
                    }),
                };
                updated.overlay.deleted.remove(&record.file_reference);
                updated
                    .overlay
                    .replacements
                    .insert(record.file_reference, replacement);
                overlay_children
                    .entry(record.parent_reference)
                    .or_default()
                    .insert(record.file_reference);
                if !was_visible {
                    updated.record_visible_insert();
                }
                if seen_upserts.insert(record.file_reference) {
                    upserts.push(record.file_reference);
                }
                if record.is_directory
                    && path_changed
                    && seen_directories.insert(record.file_reference)
                {
                    changed_directories.push(record.file_reference);
                }
            }
        }
    }

    for file_reference in upserts {
        if !refresh_visible_path(&mut updated, file_reference)? {
            // A create/move can be followed by deletion of its parent in the same journal
            // window. That transient row is no longer reachable from the indexed tree; hiding
            // only that subtree is correct and avoids turning routine temp-file churn into a
            // multi-million-row rebuild.
            hide_unreachable_entry(
                &mut updated,
                file_reference,
                &overlay_children,
                &mut live_work,
            )?;
        }
    }
    for directory in changed_directories {
        if updated.visible_entry(directory).is_none() {
            continue;
        }
        refresh_descendant_paths(&mut updated, directory, &overlay_children, &mut live_work)?;
    }
    updated.next_usn = Some(batch.next_usn);
    Ok(updated)
}

pub(crate) fn overlay_requires_compaction(snapshot: &MftSnapshot) -> bool {
    let overlay_entries = snapshot.overlay.replacements.len() + snapshot.overlay.deleted.len();
    let overlay_limit =
        MIN_OVERLAY_REBUILD_ENTRIES.max(snapshot.base_len() / OVERLAY_REBUILD_BASE_DIVISOR);
    overlay_entries > overlay_limit
}

fn refresh_visible_path(
    snapshot: &mut MftSnapshot,
    file_reference: u64,
) -> std::result::Result<bool, JournalApplyError> {
    let Some(path) = resolve_visible_path(snapshot, file_reference)? else {
        return Ok(false);
    };
    let Some(entry) = snapshot.overlay.replacements.get_mut(&file_reference) else {
        return Ok(false);
    };
    entry.path = path;
    Ok(true)
}

fn resolve_visible_path(
    snapshot: &MftSnapshot,
    file_reference: u64,
) -> std::result::Result<Option<String>, JournalApplyError> {
    let mut current = file_reference;
    let mut visiting = HashSet::new();
    let mut names = Vec::new();
    while !is_ntfs_root_reference(current) {
        if !visiting.insert(current) {
            return Ok(None);
        }
        let Some(entry) = snapshot.visible_entry(current) else {
            return Ok(None);
        };
        let Some(identity) = entry.identity else {
            return Ok(None);
        };
        names.push(entry.name.clone());
        current = identity.parent_reference;
    }
    let mut path = snapshot.mount_point.clone();
    for name in names.iter().rev() {
        if !path.ends_with(['\\', '/']) {
            path.push('\\');
        }
        path.push_str(name);
    }
    Ok(Some(path))
}

fn refresh_descendant_paths(
    snapshot: &mut MftSnapshot,
    directory_reference: u64,
    overlay_children: &HashMap<u64, HashSet<u64>>,
    live_work: &mut usize,
) -> std::result::Result<(), JournalApplyError> {
    let mut queue = VecDeque::from([directory_reference]);
    let mut visited = HashSet::from([directory_reference]);
    while let Some(parent) = queue.pop_front() {
        let parent_path = snapshot
            .visible_entry(parent)
            .map(|entry| entry.path.clone())
            .ok_or_else(|| {
                JournalApplyError::RebuildRequired(format!(
                    "{} lost live directory reference {}",
                    snapshot.mount_point, parent
                ))
            })?;
        let children =
            bounded_visible_child_candidates(snapshot, overlay_children, parent, *live_work)?;
        for child in children {
            if !visited.insert(child) {
                continue;
            }
            reserve_recursive_work(snapshot, live_work)?;
            let Some(entry) = snapshot.visible_entry(child) else {
                continue;
            };
            if entry
                .identity
                .is_none_or(|identity| identity.parent_reference != parent)
            {
                continue;
            }
            let is_directory = entry.is_directory;
            let mut replacement = entry;
            let separator = if parent_path.ends_with(['\\', '/']) {
                ""
            } else {
                "\\"
            };
            replacement.path = format!("{parent_path}{separator}{}", replacement.name);
            snapshot.overlay.replacements.insert(child, replacement);
            if is_directory {
                queue.push_back(child);
            }
        }
    }
    Ok(())
}

fn delete_visible_subtree(
    snapshot: &mut MftSnapshot,
    root: u64,
    overlay_children: &HashMap<u64, HashSet<u64>>,
    live_work: &mut usize,
) -> std::result::Result<(), JournalApplyError> {
    let mut queue = VecDeque::from([root]);
    let mut visited = HashSet::new();
    while let Some(parent) = queue.pop_front() {
        if !visited.insert(parent) {
            continue;
        }
        reserve_recursive_work(snapshot, live_work)?;
        let children =
            bounded_visible_child_candidates(snapshot, overlay_children, parent, *live_work)?;
        queue.extend(children.into_iter().filter(|child| {
            snapshot
                .visible_entry(*child)
                .and_then(|entry| entry.identity)
                .is_some_and(|identity| identity.parent_reference == parent)
        }));
        let was_visible = snapshot.visible_entry(parent).is_some();
        snapshot.overlay.replacements.remove(&parent);
        if snapshot.contains_base_reference(parent) {
            snapshot.overlay.deleted.insert(parent);
        } else {
            snapshot.overlay.deleted.remove(&parent);
        }
        if was_visible {
            snapshot.record_visible_delete();
        }
    }
    Ok(())
}

fn hide_unreachable_entry(
    snapshot: &mut MftSnapshot,
    file_reference: u64,
    overlay_children: &HashMap<u64, HashSet<u64>>,
    live_work: &mut usize,
) -> std::result::Result<(), JournalApplyError> {
    let Some(entry) = snapshot.visible_entry(file_reference) else {
        return Ok(());
    };
    if entry.is_directory && snapshot.contains_base_reference(file_reference) {
        return delete_visible_subtree(snapshot, file_reference, overlay_children, live_work);
    }
    snapshot.overlay.replacements.remove(&file_reference);
    if snapshot.contains_base_reference(file_reference) {
        snapshot.overlay.deleted.insert(file_reference);
    } else {
        snapshot.overlay.deleted.remove(&file_reference);
    }
    snapshot.record_visible_delete();
    Ok(())
}

fn overlay_children_index(snapshot: &MftSnapshot) -> HashMap<u64, HashSet<u64>> {
    let mut children = HashMap::<u64, HashSet<u64>>::new();
    for identity in snapshot
        .overlay
        .replacements
        .values()
        .filter_map(|entry| entry.identity)
    {
        children
            .entry(identity.parent_reference)
            .or_default()
            .insert(identity.file_reference);
    }
    children
}

fn bounded_visible_child_candidates(
    snapshot: &MftSnapshot,
    overlay_children: &HashMap<u64, HashSet<u64>>,
    parent: u64,
    mutations_used: usize,
) -> std::result::Result<Vec<u64>, JournalApplyError> {
    let base = snapshot.base_children(parent);
    let replacements = overlay_children.get(&parent);
    let remaining = MAX_RECURSIVE_WORK_UNITS_PER_BATCH.saturating_sub(mutations_used);
    if base.len() > remaining {
        return Err(JournalApplyError::RebuildRequired(format!(
            "{} directory reference {} has at least {} child candidates; rebuilding before expansion",
            snapshot.mount_point,
            parent,
            base.len()
        )));
    }
    let mut unique =
        HashSet::with_capacity(remaining.min(base.len() + replacements.map_or(0, HashSet::len)));
    unique.extend(base);
    if let Some(replacements) = replacements {
        for &reference in replacements {
            unique.insert(reference);
            if unique.len() > remaining {
                return Err(JournalApplyError::RebuildRequired(format!(
                    "{} directory reference {} has at least {} child candidates; rebuilding before expansion",
                    snapshot.mount_point,
                    parent,
                    unique.len()
                )));
            }
        }
    }
    Ok(unique.into_iter().collect())
}

fn reserve_recursive_work(
    snapshot: &MftSnapshot,
    live_work: &mut usize,
) -> std::result::Result<(), JournalApplyError> {
    *live_work = live_work.saturating_add(1);
    if *live_work > MAX_RECURSIVE_WORK_UNITS_PER_BATCH {
        return Err(JournalApplyError::RebuildRequired(format!(
            "{} recursive journal update exceeded {} bounded work units; rebuilding snapshot",
            snapshot.mount_point, MAX_RECURSIVE_WORK_UNITS_PER_BATCH
        )));
    }
    Ok(())
}

fn is_ntfs_root_reference(file_reference: u64) -> bool {
    const MFT_RECORD_NUMBER_MASK: u64 = (1_u64 << 48) - 1;
    file_reference & MFT_RECORD_NUMBER_MASK == 5
}

pub(crate) fn parse_journal_records(buffer: &[u8]) -> Result<(i64, Vec<UsnRecord>)> {
    ensure!(buffer.len() >= 8, "USN journal batch omitted its cursor");
    let next_usn = read_i64(buffer, 0)?;
    let mut records = Vec::new();
    let mut offset = 8;
    while offset < buffer.len() {
        ensure!(
            buffer.len() - offset >= 60,
            "USN journal record header was truncated"
        );
        let record_length = read_u32(buffer, offset)? as usize;
        ensure!(record_length >= 60, "USN journal record length was invalid");
        let record_end = offset
            .checked_add(record_length)
            .context("USN journal record length overflowed")?;
        ensure!(
            record_end <= buffer.len(),
            "USN journal record exceeded its batch"
        );
        let major_version = read_u16(buffer, offset + 4)?;
        ensure!(
            major_version == 2,
            "unsupported USN record version {major_version}"
        );
        let name_length = read_u16(buffer, offset + 56)? as usize;
        let name_offset = read_u16(buffer, offset + 58)? as usize;
        ensure!(
            name_length.is_multiple_of(2),
            "USN record name was not UTF-16 aligned"
        );
        ensure!(
            name_offset >= 60 && name_offset.is_multiple_of(2),
            "USN record name offset was invalid"
        );
        let name_start = offset
            .checked_add(name_offset)
            .context("USN name offset overflowed")?;
        let name_end = name_start
            .checked_add(name_length)
            .context("USN name length overflowed")?;
        ensure!(
            name_end <= record_end,
            "USN record name exceeded its record"
        );
        let name = String::from_utf16_lossy(
            &buffer[name_start..name_end]
                .chunks_exact(2)
                .map(|bytes| u16::from_le_bytes([bytes[0], bytes[1]]))
                .collect::<Vec<_>>(),
        );
        let reason = read_u32(buffer, offset + 40)?;
        let kind = if reason & USN_REASON_FILE_DELETE != 0 {
            UsnRecordKind::Delete
        } else if reason & USN_REASON_RENAME_OLD_NAME != 0
            && reason & USN_REASON_RENAME_NEW_NAME == 0
        {
            UsnRecordKind::RenameOld
        } else if reason & (USN_REASON_FILE_CREATE | USN_REASON_RENAME_NEW_NAME) != 0 {
            UsnRecordKind::Upsert
        } else {
            UsnRecordKind::Metadata
        };
        records.push(UsnRecord {
            file_reference: read_u64(buffer, offset + 8)?,
            parent_reference: read_u64(buffer, offset + 16)?,
            name,
            is_directory: read_u32(buffer, offset + 52)? & 0x10 != 0,
            kind,
            modified_at_unix_ms: None,
        });
        offset = record_end;
    }
    Ok((next_usn, records))
}

fn read_u16(buffer: &[u8], offset: usize) -> Result<u16> {
    let bytes = buffer.get(offset..offset + 2).context("missing USN u16")?;
    Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
}

fn read_u32(buffer: &[u8], offset: usize) -> Result<u32> {
    let bytes = buffer.get(offset..offset + 4).context("missing USN u32")?;
    Ok(u32::from_le_bytes(bytes.try_into().expect("four bytes")))
}

fn read_u64(buffer: &[u8], offset: usize) -> Result<u64> {
    let bytes = buffer.get(offset..offset + 8).context("missing USN u64")?;
    Ok(u64::from_le_bytes(bytes.try_into().expect("eight bytes")))
}

fn read_i64(buffer: &[u8], offset: usize) -> Result<i64> {
    let bytes = buffer.get(offset..offset + 8).context("missing USN i64")?;
    Ok(i64::from_le_bytes(bytes.try_into().expect("eight bytes")))
}

#[cfg(windows)]
pub(crate) fn poll_snapshot(snapshot: &MftSnapshot) -> Result<JournalPoll> {
    windows::poll_snapshot(snapshot)
}

#[cfg(not(windows))]
pub(crate) fn poll_snapshot(_snapshot: &MftSnapshot) -> Result<JournalPoll> {
    Ok(JournalPoll::NoChange)
}

fn classify_usn_read_failure(mount_point: &str, code: u32) -> Result<JournalPoll> {
    if code == ERROR_JOURNAL_ENTRY_DELETED_CODE {
        return Ok(JournalPoll::RebuildRequired(format!(
            "USN journal entry was deleted by Windows (Win32 {code})"
        )));
    }
    Err(anyhow::anyhow!(
        "read USN journal for {mount_point} failed with Win32 {code}"
    ))
}

#[cfg(windows)]
#[allow(unsafe_code)]
mod windows {
    use std::ffi::c_void;
    use std::mem::size_of;
    use std::ptr::{null, null_mut};

    use anyhow::{Context, Result, ensure};
    use windows_sys::Win32::Foundation::{
        CloseHandle, GENERIC_READ, GetLastError, HANDLE, INVALID_HANDLE_VALUE,
    };
    use windows_sys::Win32::Storage::FileSystem::{
        CreateFileW, FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
    };
    use windows_sys::Win32::System::IO::DeviceIoControl;
    use windows_sys::Win32::System::Ioctl::{
        FSCTL_QUERY_USN_JOURNAL, FSCTL_READ_USN_JOURNAL, READ_USN_JOURNAL_DATA_V0,
        USN_JOURNAL_DATA_V0,
    };

    use super::{
        JournalApplyError, JournalBatch, JournalPoll, MAINTENANCE_REASON_MASK,
        MAX_METADATA_READS_PER_POLL, MftSnapshot, apply_journal_batch, classify_usn_read_failure,
        hydrate_journal_times_while, journal_record_usn_at, metadata_batch_prefix,
        parse_journal_records,
    };

    const JOURNAL_BUFFER_BYTES: usize = 256 * 1024;
    const MAX_READS_PER_POLL: usize = 4;

    struct VolumeHandle(HANDLE);

    impl Drop for VolumeHandle {
        fn drop(&mut self) {
            unsafe {
                let _ = CloseHandle(self.0);
            }
        }
    }

    pub(super) fn poll_snapshot(snapshot: &MftSnapshot) -> Result<JournalPoll> {
        let (Some(journal_id), Some(start_usn)) = (snapshot.journal_id, snapshot.next_usn) else {
            return Ok(JournalPoll::RebuildRequired(format!(
                "{} has no resumable USN cursor",
                snapshot.mount_point
            )));
        };
        let handle = open_volume(&snapshot.mount_point)?;
        let journal = query_journal(handle.0)?;
        if journal.UsnJournalID != journal_id
            || start_usn < journal.LowestValidUsn
            || start_usn > journal.NextUsn
        {
            return Ok(JournalPoll::RebuildRequired(format!(
                "{} USN journal can no longer resume at {}",
                snapshot.mount_point, start_usn
            )));
        }
        if start_usn == journal.NextUsn {
            return Ok(JournalPoll::NoChange);
        }

        let mut cursor = start_usn;
        let mut records = Vec::new();
        let mut buffer = vec![0_u8; JOURNAL_BUFFER_BYTES];
        let mut reads = 0_usize;
        while cursor < journal.NextUsn && reads < MAX_READS_PER_POLL {
            let request = READ_USN_JOURNAL_DATA_V0 {
                StartUsn: cursor,
                ReasonMask: MAINTENANCE_REASON_MASK,
                ReturnOnlyOnClose: 0,
                Timeout: 0,
                BytesToWaitFor: 0,
                UsnJournalID: journal_id,
            };
            let mut returned = 0_u32;
            let success = unsafe {
                DeviceIoControl(
                    handle.0,
                    FSCTL_READ_USN_JOURNAL,
                    (&raw const request).cast::<c_void>(),
                    size_of::<READ_USN_JOURNAL_DATA_V0>() as u32,
                    buffer.as_mut_ptr().cast::<c_void>(),
                    buffer.len() as u32,
                    &raw mut returned,
                    null_mut(),
                )
            };
            if success == 0 {
                return classify_usn_read_failure(&snapshot.mount_point, unsafe { GetLastError() });
            }
            ensure!(
                returned as usize <= buffer.len(),
                "USN journal read overflowed its buffer"
            );
            let (next, mut batch) = parse_journal_records(&buffer[..returned as usize])?;
            ensure!(next >= cursor, "USN journal cursor moved backwards");
            // Bound native metadata reads without advancing past deferred records.
            let keep = metadata_batch_prefix(&records, &batch, MAX_METADATA_READS_PER_POLL);
            if keep < batch.len() {
                let deferred_usn = journal_record_usn_at(&buffer[..returned as usize], keep)?;
                ensure!(
                    deferred_usn >= cursor && deferred_usn <= next,
                    "deferred USN cursor was invalid"
                );
                cursor = deferred_usn;
                batch.truncate(keep);
                records.append(&mut batch);
                break;
            }
            records.append(&mut batch);
            if next == cursor {
                break;
            }
            cursor = next;
            reads += 1;
        }
        if cursor == start_usn && records.is_empty() {
            return Ok(JournalPoll::NoChange);
        }
        let _ = hydrate_journal_times_while(
            &mut records,
            |reference| crate::mft::read_last_write_by_id(handle.0, reference),
            &|| true,
        );
        match apply_journal_batch(
            snapshot,
            JournalBatch {
                journal_id,
                lowest_valid_usn: journal.LowestValidUsn,
                next_usn: cursor,
                records,
            },
        ) {
            Ok(updated) => Ok(super::classify_applied_update(snapshot, updated)),
            Err(JournalApplyError::RebuildRequired(reason)) => {
                Ok(JournalPoll::RebuildRequired(reason))
            }
        }
    }

    fn open_volume(mount_point: &str) -> Result<VolumeHandle> {
        let drive = mount_point
            .chars()
            .next()
            .context("mount point omitted drive")?;
        let device = format!(r"\\.\{}:", drive.to_ascii_uppercase());
        let wide = device
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect::<Vec<_>>();
        let handle = unsafe {
            CreateFileW(
                wide.as_ptr(),
                GENERIC_READ,
                FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                null(),
                OPEN_EXISTING,
                0,
                null_mut(),
            )
        };
        if handle == INVALID_HANDLE_VALUE {
            return Err(anyhow::anyhow!(
                "open {} for USN polling failed with Win32 {}",
                mount_point,
                unsafe { GetLastError() }
            ));
        }
        Ok(VolumeHandle(handle))
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
            return Err(anyhow::anyhow!(
                "query USN journal failed with Win32 {}",
                unsafe { GetLastError() }
            ));
        }
        ensure!(
            returned as usize >= size_of::<USN_JOURNAL_DATA_V0>(),
            "USN journal query was truncated"
        );
        Ok(journal)
    }
}

#[cfg(test)]
mod tests {
    fn w4_wire_record(reason: u32, reference: u64, usn: i64) -> Vec<u8> {
        let name = "notes.txt".encode_utf16().collect::<Vec<_>>();
        let mut record = vec![0_u8; 60 + name.len() * 2];
        let length = record.len() as u32;
        record[..4].copy_from_slice(&length.to_le_bytes());
        record[4..6].copy_from_slice(&2_u16.to_le_bytes());
        record[8..16].copy_from_slice(&reference.to_le_bytes());
        record[16..24].copy_from_slice(&10_u64.to_le_bytes());
        record[24..32].copy_from_slice(&usn.to_le_bytes());
        // A plausible but intentionally different event time must never become last-write.
        record[32..40].copy_from_slice(&133_000_000_000_000_000_i64.to_le_bytes());
        record[40..44].copy_from_slice(&reason.to_le_bytes());
        record[56..58].copy_from_slice(&((name.len() * 2) as u16).to_le_bytes());
        record[58..60].copy_from_slice(&60_u16.to_le_bytes());
        for (index, unit) in name.iter().enumerate() {
            record[60 + index * 2..62 + index * 2].copy_from_slice(&unit.to_le_bytes());
        }
        record
    }

    #[test]
    fn w4_usn_data_basic_close_metadata_seam_uses_true_time_and_preserves_tree() {
        for reason in [
            USN_REASON_DATA_OVERWRITE,
            USN_REASON_DATA_EXTEND,
            USN_REASON_DATA_TRUNCATION,
            USN_REASON_BASIC_INFO_CHANGE,
            USN_REASON_CLOSE,
        ] {
            assert_ne!(MAINTENANCE_REASON_MASK & reason, 0);
            let mut buffer = 110_i64.to_le_bytes().to_vec();
            buffer.extend(w4_wire_record(reason, 11, 100));
            let (next_usn, mut records) = parse_journal_records(&buffer).expect("metadata record");
            assert_eq!(records[0].kind, UsnRecordKind::Metadata);
            assert_eq!(records[0].modified_at_unix_ms, None);
            assert!(hydrate_journal_times_while(
                &mut records,
                |reference| {
                    assert_eq!(reference, 11);
                    Some(1234)
                },
                &|| true
            ));
            let updated = apply_journal_batch(
                &snapshot(),
                JournalBatch {
                    journal_id: 17,
                    lowest_valid_usn: 90,
                    next_usn,
                    records,
                },
            )
            .expect("metadata batch");
            let file = updated.visible_entry(11).expect("updated file");
            assert_eq!(file.modified_at_unix_ms, Some(1234));
            assert_eq!(file.path, "C:\\Work\\notes.txt");
            assert_eq!(updated.visible_len(), 2);
            assert_eq!(updated.next_usn, Some(110));
            assert_eq!(
                snapshot()
                    .visible_entry(11)
                    .expect("last-good")
                    .modified_at_unix_ms,
                None
            );
            let mut older = vec![UsnRecord {
                modified_at_unix_ms: Some(Some(1)),
                kind: UsnRecordKind::Metadata,
                ..UsnRecord::upsert(11, 10, "wrong-old-name", false)
            }];
            assert!(hydrate_journal_times_while(&mut older, |_| None, &|| true));
            let unknown = apply_journal_batch(
                &updated,
                JournalBatch {
                    journal_id: 17,
                    lowest_valid_usn: 90,
                    next_usn: 120,
                    records: older,
                },
            )
            .expect("metadata read failure remains unknown");
            assert_eq!(
                unknown
                    .visible_entry(11)
                    .expect("unknown file")
                    .modified_at_unix_ms,
                None
            );
            assert_eq!(
                unknown.visible_entry(11).expect("unchanged path").path,
                file.path
            );
        }
    }

    #[test]
    fn w4_usn_budget_retains_deferred_cursor_and_deduplicates_native_reads() {
        let mut buffer = 120_i64.to_le_bytes().to_vec();
        for (reference, usn) in [(11, 100), (11, 101), (12, 102), (13, 103)] {
            buffer.extend(w4_wire_record(USN_REASON_DATA_EXTEND, reference, usn));
        }
        let (_, mut records) = parse_journal_records(&buffer).expect("fixture batch");
        assert_eq!(metadata_batch_prefix(&[], &records, 2), 3);
        assert_eq!(
            journal_record_usn_at(&buffer, 3).expect("deferred cursor"),
            103
        );
        records.truncate(3);
        let mut calls = Vec::new();
        assert!(hydrate_journal_times_while(
            &mut records,
            |id| {
                calls.push(id);
                Some(7)
            },
            &|| true
        ));
        assert_eq!(calls, [11, 12]);
        assert!(!hydrate_journal_times_while(
            &mut records,
            |_| panic!("cancelled"),
            &|| false
        ));
        let updated = apply_journal_batch(
            &snapshot(),
            JournalBatch {
                journal_id: 17,
                lowest_valid_usn: 90,
                next_usn: 103,
                records,
            },
        )
        .expect("bounded prefix");
        assert_eq!(updated.next_usn, Some(103));
        assert!(
            updated.visible_entry(12).is_none(),
            "metadata-only records cannot resurrect unknown files"
        );
    }

    #[test]
    fn w4_usn_backwards_true_last_write_is_not_clamped_to_event_or_previous_time() {
        let mut old = snapshot();
        let mut file = old.visible_entry(11).expect("file");
        file.modified_at_unix_ms = Some(9000);
        old.overlay.replacements.insert(11, file);
        let records = vec![UsnRecord {
            kind: UsnRecordKind::Metadata,
            modified_at_unix_ms: Some(Some(10)),
            ..UsnRecord::upsert(11, 10, "notes.txt", false)
        }];
        let updated = apply_journal_batch(
            &old,
            JournalBatch {
                journal_id: 17,
                lowest_valid_usn: 90,
                next_usn: 110,
                records,
            },
        )
        .expect("restored timestamp");
        assert_eq!(
            updated.visible_entry(11).expect("file").modified_at_unix_ms,
            Some(10)
        );
    }

    use super::*;
    use crate::mft::{MftEntry, MftIdentity, MftSnapshot};
    use nxtools_protocol::{IndexSearchContext, IndexSearchRequest};

    fn entry(
        file_reference: u64,
        parent_reference: u64,
        path: &str,
        is_directory: bool,
    ) -> MftEntry {
        MftEntry {
            identity: Some(MftIdentity {
                file_reference,
                parent_reference,
            }),
            path: path.into(),
            name: path.rsplit('\\').next().unwrap_or(path).into(),
            is_directory,
            size_bytes: None,
            modified_at_unix_ms: None,
        }
    }

    #[test]
    fn deleted_journal_entry_requests_a_scoped_rebuild_instead_of_staying_partial() {
        let result = classify_usn_read_failure(r"C:\", 1181)
            .expect("ERROR_JOURNAL_ENTRY_DELETED must be recoverable");

        assert!(matches!(
            result,
            JournalPoll::RebuildRequired(reason)
                if reason.contains("1181") && reason.contains("deleted by Windows")
        ));
    }

    fn snapshot() -> MftSnapshot {
        MftSnapshot::from_entries(
            r"C:\",
            Some(17),
            Some(100),
            vec![
                entry(10, 5, r"C:\Work", true),
                entry(11, 10, r"C:\Work\notes.txt", false),
            ],
        )
    }

    #[test]
    fn create_rename_move_and_delete_are_visible_in_the_published_overlay() {
        let created = apply_journal_batch(
            &snapshot(),
            JournalBatch {
                journal_id: 17,
                lowest_valid_usn: 90,
                next_usn: 110,
                records: vec![UsnRecord::upsert(12, 10, "draft.txt", false)],
            },
        )
        .expect("valid create batch");
        assert_eq!(
            created.visible_entry(12).expect("created entry").path,
            r"C:\Work\draft.txt"
        );

        let renamed = apply_journal_batch(
            &created,
            JournalBatch {
                journal_id: 17,
                lowest_valid_usn: 90,
                next_usn: 120,
                records: vec![UsnRecord::upsert(10, 5, "Projects", true)],
            },
        )
        .expect("valid directory rename");
        assert_eq!(
            renamed.visible_entry(10).expect("renamed directory").path,
            r"C:\Projects"
        );
        assert_eq!(
            renamed.visible_entry(11).expect("renamed descendant").path,
            r"C:\Projects\notes.txt"
        );
        let query = IndexSearchRequest {
            generation: 1,
            query: "notes".into(),
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
        let results = crate::search_snapshots_while(
            std::slice::from_ref(&renamed),
            &nxtools_protocol::IndexSettings::default(),
            &query,
            || true,
        )
        .expect("current query");
        assert_eq!(results[0].path, r"C:\Projects\notes.txt");

        let moved = apply_journal_batch(
            &renamed,
            JournalBatch {
                journal_id: 17,
                lowest_valid_usn: 90,
                next_usn: 130,
                records: vec![UsnRecord::upsert(12, 5, "draft.txt", false)],
            },
        )
        .expect("valid move");
        assert_eq!(
            moved.visible_entry(12).expect("moved entry").path,
            r"C:\draft.txt"
        );

        let deleted = apply_journal_batch(
            &moved,
            JournalBatch {
                journal_id: 17,
                lowest_valid_usn: 90,
                next_usn: 140,
                records: vec![UsnRecord::delete(12)],
            },
        )
        .expect("valid delete");
        assert!(deleted.visible_entry(12).is_none());
        assert_eq!(deleted.next_usn, Some(140));
    }

    #[test]
    fn unreachable_transient_rows_are_hidden_without_rebuilding_the_snapshot() {
        let updated = apply_journal_batch(
            &snapshot(),
            JournalBatch {
                journal_id: 17,
                lowest_valid_usn: 90,
                next_usn: 110,
                records: vec![
                    UsnRecord::upsert(12, 99, "already-deleted.tmp", false),
                    UsnRecord::upsert(11, 99, "moved-then-deleted.tmp", false),
                ],
            },
        )
        .expect("unreachable transient rows do not invalidate the base snapshot");

        assert!(updated.visible_entry(12).is_none());
        assert!(updated.visible_entry(11).is_none());
        assert_eq!(updated.visible_len(), 1);
        assert_eq!(updated.next_usn, Some(110));
    }

    #[test]
    fn thousands_of_unreachable_new_files_cancel_out_without_tombstone_growth() {
        let records = (1_000..6_000)
            .map(|file_reference| {
                UsnRecord::upsert(
                    file_reference,
                    99,
                    format!("transient-{file_reference}.tmp"),
                    false,
                )
            })
            .collect();
        let updated = apply_journal_batch(
            &snapshot(),
            JournalBatch {
                journal_id: 17,
                lowest_valid_usn: 90,
                next_usn: 110,
                records,
            },
        )
        .expect("new transient rows cancel out instead of requesting compaction");

        assert_eq!(updated.visible_len(), 2);
        assert!(updated.overlay.replacements.is_empty());
        assert!(updated.overlay.deleted.is_empty());
    }

    #[test]
    fn changed_journal_or_expired_cursor_requires_a_full_rebuild() {
        let changed = apply_journal_batch(
            &snapshot(),
            JournalBatch {
                journal_id: 18,
                lowest_valid_usn: 90,
                next_usn: 110,
                records: Vec::new(),
            },
        )
        .expect_err("journal identity changed");
        assert!(matches!(changed, JournalApplyError::RebuildRequired(_)));

        let expired = apply_journal_batch(
            &snapshot(),
            JournalBatch {
                journal_id: 17,
                lowest_valid_usn: 101,
                next_usn: 110,
                records: Vec::new(),
            },
        )
        .expect_err("cursor expired");
        assert!(matches!(expired, JournalApplyError::RebuildRequired(_)));
    }

    #[test]
    fn malformed_journal_record_is_rejected_before_overlay_publication() {
        let mut buffer = vec![0_u8; 8 + 60];
        buffer[..8].copy_from_slice(&110_i64.to_le_bytes());
        buffer[8..12].copy_from_slice(&80_u32.to_le_bytes());
        buffer[12..14].copy_from_slice(&2_u16.to_le_bytes());

        assert!(parse_journal_records(&buffer).is_err());
    }

    #[test]
    fn an_oversized_live_overlay_requests_compaction_instead_of_growing_forever() {
        let mut crowded = snapshot();
        for offset in 0..MIN_OVERLAY_REBUILD_ENTRIES {
            crowded.overlay.replacements.insert(
                1_000 + offset as u64,
                entry(
                    1_000 + offset as u64,
                    5,
                    &format!(r"C:\{offset}.txt"),
                    false,
                ),
            );
        }

        let updated = apply_journal_batch(
            &crowded,
            JournalBatch {
                journal_id: 17,
                lowest_valid_usn: 90,
                next_usn: 110,
                records: vec![UsnRecord::upsert(10_000, 5, "next.txt", false)],
            },
        )
        .expect("the latest journal changes remain searchable before compaction");

        assert!(overlay_requires_compaction(&updated));
        assert!(updated.visible_entry(10_000).is_some());
    }

    #[test]
    fn a_bounded_read_batch_with_many_direct_upserts_remains_incremental() {
        let records = (0..=MAX_RECURSIVE_WORK_UNITS_PER_BATCH)
            .map(|offset| {
                UsnRecord::upsert(30_000 + offset as u64, 5, format!("{offset}.txt"), false)
            })
            .collect();

        let mut base_entries = Vec::with_capacity(50_000);
        base_entries.extend((0..50_000).map(|offset| {
            entry(
                100_000 + offset as u64,
                5,
                &format!(r"C:\base-{offset}.txt"),
                false,
            )
        }));
        let base = MftSnapshot::from_entries(r"C:\", Some(17), Some(100), base_entries);
        let updated = apply_journal_batch(
            &base,
            JournalBatch {
                journal_id: 17,
                lowest_valid_usn: 90,
                next_usn: 110,
                records,
            },
        )
        .expect("direct records from a bounded journal read must not force a rebuild");

        assert_eq!(updated.next_usn, Some(110));
        assert_eq!(
            updated.overlay.replacements.len(),
            MAX_RECURSIVE_WORK_UNITS_PER_BATCH + 1
        );
    }

    #[test]
    fn duplicate_heavy_batch_does_not_consume_recursive_candidate_budget() {
        let records = (0..MAX_RECURSIVE_WORK_UNITS_PER_BATCH + 2)
            .map(|_| UsnRecord::upsert(10, 5, "Renamed", true))
            .collect();
        let updated = apply_journal_batch(
            &snapshot(),
            JournalBatch {
                journal_id: 17,
                lowest_valid_usn: 90,
                next_usn: 110,
                records,
            },
        )
        .expect("one distinct directory child must fit within the subtree budget");
        assert_eq!(
            updated.visible_entry(11).expect("descendant").path,
            r"C:\Renamed\notes.txt"
        );
        assert_eq!(updated.next_usn, Some(110));
    }

    #[test]
    fn a_large_directory_change_requests_rebuild_before_unbounded_descendant_work() {
        let mut entries = vec![entry(10, 5, r"C:\Large", true)];
        entries.extend((0..=MAX_RECURSIVE_WORK_UNITS_PER_BATCH).map(|offset| {
            entry(
                20_000 + offset as u64,
                10,
                &format!(r"C:\Large\{offset}.txt"),
                false,
            )
        }));
        let large = MftSnapshot::from_entries(r"C:\", Some(17), Some(100), entries);

        let result = apply_journal_batch(
            &large,
            JournalBatch {
                journal_id: 17,
                lowest_valid_usn: 90,
                next_usn: 110,
                records: vec![UsnRecord::upsert(10, 5, "Renamed", true)],
            },
        )
        .expect_err("large subtree mutation must switch to a background rebuild");

        assert!(result.to_string().contains("rebuilding before expansion"));
    }

    #[cfg(windows)]
    #[test]
    fn the_real_windows_usn_boundary_observes_a_new_file_or_is_actionably_denied() {
        let executable = std::env::current_exe().expect("current test executable");
        let mount_point = executable
            .components()
            .next()
            .map(|component| format!("{}\\", component.as_os_str().to_string_lossy()))
            .expect("test executable drive");
        let snapshot = match crate::mft::scan_volume(&mount_point) {
            Ok(snapshot) => snapshot,
            Err(error) => {
                let message = error.to_string();
                assert!(
                    message.contains("管理员权限") && message.contains("Win32 5"),
                    "unexpected MFT boundary failure: {message}"
                );
                return;
            }
        };
        let file_name = format!(
            "nxtools-usn-boundary-{}-{}.tmp",
            std::process::id(),
            snapshot.next_usn.unwrap_or_default()
        );
        let path = executable
            .parent()
            .expect("test executable directory")
            .join(&file_name);
        std::fs::write(&path, b"usn boundary").expect("create USN boundary file");

        let result = poll_snapshot(&snapshot);
        let _ = std::fs::remove_file(&path);
        match result {
            Ok(JournalPoll::Updated(updated)) => assert!(
                updated
                    .overlay
                    .replacements
                    .values()
                    .any(|entry| entry.name == file_name),
                "new file was not published from the real USN journal"
            ),
            Ok(other) => panic!("real USN boundary did not publish the new file: {other:?}"),
            Err(error) => {
                let message = error.to_string();
                assert!(
                    message.contains("Win32 5"),
                    "unexpected USN boundary failure: {message}"
                );
            }
        }
    }
}

#[cfg(test)]
mod idle_classification_tests {
    use super::*;

    #[test]
    fn idle_io_no_effect_metadata_events_keep_idle_backoff_but_real_time_changes_do_not() {
        let snapshot = MftSnapshot::from_entries(
            r"C:\",
            Some(17),
            Some(100),
            vec![MftEntry {
                identity: Some(MftIdentity {
                    file_reference: 11,
                    parent_reference: 5,
                }),
                path: r"C:\kept.txt".into(),
                name: "kept.txt".into(),
                is_directory: false,
                size_bytes: None,
                modified_at_unix_ms: Some(9),
            }],
        );
        let applied = apply_journal_batch(
            &snapshot,
            JournalBatch {
                journal_id: 17,
                lowest_valid_usn: 0,
                next_usn: 110,
                records: vec![UsnRecord {
                    file_reference: 11,
                    parent_reference: 5,
                    name: "kept.txt".into(),
                    is_directory: false,
                    kind: UsnRecordKind::Metadata,
                    modified_at_unix_ms: Some(Some(9)),
                }],
            },
        )
        .expect("apply unchanged close metadata");
        assert!(matches!(
            classify_applied_update(&snapshot, applied),
            JournalPoll::CursorAdvanced(_)
        ));
        let mut changed = snapshot.clone();
        changed.next_usn = Some(110);
        let mut entry = changed.visible_entry(11).expect("existing row");
        entry.modified_at_unix_ms = Some(10);
        changed.overlay.replacements.insert(11, entry);
        assert!(matches!(
            classify_applied_update(&snapshot, changed),
            JournalPoll::Updated(_)
        ));
        assert!(matches!(
            classify_applied_update(&snapshot, snapshot.clone()),
            JournalPoll::NoChange
        ));
    }
}
