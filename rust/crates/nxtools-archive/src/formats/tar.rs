//! Fixed header parsing only; never tar::Archive::entries/unpack (extension allocations/native IO).
use crate::{
    ArchiveEntry, ArchiveError, ArchiveOutcome, ArchiveSnapshot, EntryId, EntryKind, EntryWarning,
    NameEncoding, NameProvenance, OperationControl, ResourceLimits, Result,
};
use std::{
    collections::BTreeMap,
    fs::File,
    io::{Read, Seek, SeekFrom},
};
use uuid::Uuid;
/// Immutable operation-local payload locations. Metadata lives only in the snapshot.
pub(crate) struct ValidatedPlan {
    offsets: Vec<u64>,
    session: Uuid,
    metadata_bytes: u64,
}
impl ValidatedPlan {
    pub(crate) fn session_id(&self) -> Uuid {
        self.session
    }
    pub(crate) fn metadata_bytes(&self) -> u64 {
        self.metadata_bytes
    }
    pub(crate) fn locator_bytes(&self) -> u64 {
        (self.offsets.capacity() * std::mem::size_of::<u64>()) as u64
    }
    fn check(&self, snapshot: &ArchiveSnapshot) -> Result<()> {
        if self.session != snapshot.session || self.offsets.len() != snapshot.entries.len() {
            return Err(ArchiveError::SourceChanged);
        }
        Ok(())
    }
    fn admit_execution(&self, snapshot: &ArchiveSnapshot, limits: &ResourceLimits) -> Result<()> {
        self.check(snapshot)?;
        let bytes = (snapshot.entries.len() as u64)
            .checked_mul(128)
            .and_then(|bytes| bytes.checked_add(self.locator_bytes()))
            .and_then(|bytes| bytes.checked_add(256 * 1024))
            .ok_or(ArchiveError::ResourceLimit)?;
        if bytes > limits.max_working_bytes || self.metadata_bytes > limits.max_metadata_bytes {
            return Err(ArchiveError::ResourceLimit);
        }
        Ok(())
    }
    fn locate<'a>(
        &self,
        snapshot: &'a ArchiveSnapshot,
        id: EntryId,
    ) -> Result<(u64, &'a ArchiveEntry)> {
        self.check(snapshot)?;
        let ordinal = usize::try_from(id.ordinal).map_err(|_| ArchiveError::SourceChanged)?;
        let entry = snapshot
            .entries
            .get(ordinal)
            .filter(|entry| entry.id == id)
            .ok_or(ArchiveError::SourceChanged)?;
        Ok((
            *self
                .offsets
                .get(ordinal)
                .ok_or(ArchiveError::SourceChanged)?,
            entry,
        ))
    }
}
pub(crate) fn metadata_bytes(snapshot: &ArchiveSnapshot) -> Result<u64> {
    snapshot.entries.iter().try_fold(0u64, |bytes, entry| {
        bytes
            .checked_add(1024 + (entry.raw_name.len() as u64).saturating_mul(32))
            .ok_or(ArchiveError::ResourceLimit)
    })
}
/// Aggregate once per parser invocation, including rejected/truncated scans. Header seeks
/// include extension headers and the first zero end block, but exclude sequential tail reads.
struct ScanObservation<'a> {
    control: &'a OperationControl,
    header_seeks: u64,
    plan_bytes: u64,
}
impl Drop for ScanObservation<'_> {
    fn drop(&mut self) {
        self.control.record_work(|work| {
            work.tar_record_scans += 1;
            work.tar_header_seeks += self.header_seeks;
            work.tar_index_builds += 1;
            // Cumulative admitted ordinal-Vec backing bytes, including failed scans; not peak/RSS.
            work.tar_plan_bytes += self.plan_bytes;
        });
    }
}
#[derive(Clone, Copy, Default)]
enum PaxModified {
    #[default]
    Unspecified,
    Set(crate::metadata::ModifiedTime),
    Delete,
}
#[derive(Default)]
struct Extensions {
    path: Option<Vec<u8>>,
    size: Option<u64>,
    modified: PaxModified,
}
fn corrupt(_: std::io::Error) -> ArchiveError {
    ArchiveError::CorruptArchive
}
fn exact(file: &mut File, bytes: &mut [u8]) -> Result<()> {
    file.read_exact(bytes).map_err(|failure| {
        if failure.kind() == std::io::ErrorKind::UnexpectedEof {
            ArchiveError::CorruptArchive
        } else {
            ArchiveError::IoFailure
        }
    })
}
fn pax(bytes: &[u8], extension: &mut Extensions) -> Result<()> {
    let mut cursor = 0;
    while cursor < bytes.len() {
        let space = bytes[cursor..]
            .iter()
            .position(|value| *value == b' ')
            .ok_or(ArchiveError::CorruptArchive)?
            + cursor;
        let length = std::str::from_utf8(&bytes[cursor..space])
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .ok_or(ArchiveError::CorruptArchive)?;
        let end = cursor
            .checked_add(length)
            .filter(|end| *end <= bytes.len() && *end > space + 1)
            .ok_or(ArchiveError::CorruptArchive)?;
        let value = &bytes[space + 1..end];
        if value.last() != Some(&b'\n') {
            return Err(ArchiveError::CorruptArchive);
        }
        let equal = value
            .iter()
            .position(|value| *value == b'=')
            .ok_or(ArchiveError::CorruptArchive)?;
        let key = &value[..equal];
        let data = &value[equal + 1..value.len() - 1];
        if key.starts_with(b"GNU.sparse") || key == b"SCHILY.realsize" {
            return Err(ArchiveError::UnsupportedMethod);
        }
        match key {
            b"path" => {
                std::str::from_utf8(data).map_err(|_| ArchiveError::CorruptArchive)?;
                extension.path = Some(data.to_vec());
            }
            b"size" => {
                extension.size = Some(
                    std::str::from_utf8(data)
                        .ok()
                        .and_then(|value| value.parse().ok())
                        .ok_or(ArchiveError::CorruptArchive)?,
                )
            }
            b"mtime" => {
                extension.modified = if data.is_empty() {
                    PaxModified::Delete
                } else {
                    PaxModified::Set(crate::metadata::unix_decimal(data))
                };
            }
            _ => {}
        }
        cursor = end;
    }
    Ok(())
}
/// Decode only the fixed mtime field; size and other numeric authority stay unchanged.
fn header_mtime(bytes: &[u8; 12]) -> Option<crate::metadata::ModifiedTime> {
    use crate::metadata::ModifiedTime;
    if bytes.iter().all(|byte| matches!(byte, 0 | b' ')) {
        return None;
    }
    let seconds = (|| {
        if bytes[0] & 0x80 != 0 {
            // GNU's marker is exactly 0x80 or 0xff. Consume ALL remaining bytes with
            // sign extension, never reinterpret only the low64 bits of an overflow.
            let initial = match bytes[0] {
                0x80 => 0i128,
                0xff => -1i128,
                _ => return None,
            };
            let value = bytes[1..].iter().try_fold(initial, |value, byte| {
                value.checked_mul(256)?.checked_add(i128::from(*byte))
            })?;
            i64::try_from(value).ok()
        } else {
            let start = bytes.iter().position(|byte| *byte != b' ')?;
            let end = bytes.iter().rposition(|byte| !matches!(byte, 0 | b' '))? + 1;
            bytes[start..end].iter().try_fold(0i64, |value, byte| {
                if !(b'0'..=b'7').contains(byte) {
                    return None;
                }
                value.checked_mul(8)?.checked_add(i64::from(byte - b'0'))
            })
        }
    })();
    Some(
        seconds.map_or(ModifiedTime::Invalid, |seconds| ModifiedTime::UnixUtc {
            seconds,
            nanoseconds: 0,
            precision_ns: 1_000_000_000,
        }),
    )
}
fn records(
    file: &mut File,
    encoding: NameEncoding,
    limits: &ResourceLimits,
    control: &OperationControl,
    snapshot: Option<&ArchiveSnapshot>,
) -> Result<(Vec<ArchiveEntry>, ValidatedPlan)> {
    let mut observation = ScanObservation {
        control,
        header_seeks: 0,
        plan_bytes: 0,
    };
    if limits.max_working_bytes < 256 * 1024 || limits.max_metadata_bytes < 512 {
        return Err(ArchiveError::ResourceLimit);
    }
    let length = file.metadata()?.len();
    let session = snapshot.map_or_else(Uuid::new_v4, |snapshot| snapshot.session);
    let mut offset = 0u64;
    let mut items = Vec::new();
    let mut offsets = Vec::<u64>::new();
    let mut global = Extensions::default();
    let mut local = Extensions::default();
    let mut longname = None;
    let mut pending_extension = false;
    let mut extension_count = 0u64;
    let mut metadata = snapshot.map(metadata_bytes).transpose()?.unwrap_or(0);
    if metadata > limits.max_metadata_bytes {
        return Err(ArchiveError::ResourceLimit);
    }
    loop {
        control.checkpoint()?;
        if offset.checked_add(512).is_none_or(|end| end > length) {
            return Err(ArchiveError::CorruptArchive);
        }
        file.seek(SeekFrom::Start(offset))?;
        observation.header_seeks += 1;
        let mut header = tar::Header::new_old();
        exact(file, header.as_mut_bytes())?;
        let raw = header.as_bytes();
        if raw.iter().all(|value| *value == 0) {
            if offset.checked_add(1024).is_none_or(|end| end > length)
                || local.path.is_some()
                || local.size.is_some()
                || longname.is_some()
                || pending_extension
            {
                return Err(ArchiveError::CorruptArchive);
            }
            let mut buffer = [0u8; 8192];
            loop {
                control.checkpoint()?;
                let count = file.read(&mut buffer)?;
                if count == 0 {
                    break;
                }
                if buffer[..count].iter().any(|value| *value != 0) {
                    return Err(ArchiveError::CorruptArchive);
                }
            }
            if snapshot.is_some_and(|snapshot| offsets.len() != snapshot.entries.len()) {
                return Err(ArchiveError::SourceChanged);
            }
            return Ok((
                items,
                ValidatedPlan {
                    offsets,
                    session,
                    metadata_bytes: metadata
                        .checked_add(observation.plan_bytes)
                        .ok_or(ArchiveError::ResourceLimit)?,
                },
            ));
        }
        let checksum: u32 = raw
            .iter()
            .enumerate()
            .map(|(index, value)| {
                if (148..156).contains(&index) {
                    32
                } else {
                    u32::from(*value)
                }
            })
            .sum();
        if header.cksum().map_err(corrupt)? != checksum {
            return Err(ArchiveError::CorruptArchive);
        }
        let kind = header.entry_type();
        let extension = kind.is_gnu_longname()
            || kind.is_gnu_longlink()
            || kind.is_pax_local_extensions()
            || kind.is_pax_global_extensions();
        if kind.is_gnu_sparse() {
            return Err(ArchiveError::UnsupportedMethod);
        }
        let size = if extension {
            header.entry_size().map_err(corrupt)?
        } else {
            local
                .size
                .or(global.size)
                .unwrap_or(header.entry_size().map_err(corrupt)?)
        };
        let payload = offset
            .checked_add(512)
            .ok_or(ArchiveError::CorruptArchive)?;
        let end = payload
            .checked_add(size)
            .filter(|end| *end <= length)
            .ok_or(ArchiveError::CorruptArchive)?;
        let next = end
            .checked_add((512 - size % 512) % 512)
            .filter(|end| *end <= length)
            .ok_or(ArchiveError::CorruptArchive)?;
        if extension {
            if !kind.is_pax_global_extensions() {
                pending_extension = true;
            }
            extension_count += 1;
            let pending_bytes = global.path.as_ref().map_or(0, |path| path.len() as u64)
                + local.path.as_ref().map_or(0, |path| path.len() as u64)
                + longname
                    .as_ref()
                    .map_or(0, |path: &Vec<u8>| path.len() as u64);
            if extension_count > limits.max_entries
                || size > 65536
                || metadata
                    .saturating_add(observation.plan_bytes)
                    .saturating_add(pending_bytes)
                    .saturating_add(size.saturating_mul(4))
                    > limits.max_metadata_bytes
                || observation
                    .plan_bytes
                    .saturating_add(256 * 1024)
                    .saturating_add(pending_bytes)
                    .saturating_add(size)
                    > limits.max_working_bytes
            {
                return Err(ArchiveError::ResourceLimit);
            }
            let mut bytes = vec![0; size as usize];
            exact(file, &mut bytes)?;
            if kind.is_gnu_longname() {
                if longname.is_some() {
                    return Err(ArchiveError::CorruptArchive);
                }
                if bytes.last() == Some(&0) {
                    bytes.pop();
                }
                longname = Some(bytes);
            } else if kind.is_pax_local_extensions() {
                pax(&bytes, &mut local)?;
            } else if kind.is_pax_global_extensions() {
                pax(&bytes, &mut global)?;
            }
            offset = next;
            continue;
        }
        if offsets.len() as u64 >= limits.max_entries {
            return Err(ArchiveError::ResourceLimit);
        }
        let authoritative = local.path.take().or_else(|| global.path.clone());
        let raw_name = authoritative
            .clone()
            .or_else(|| longname.take())
            .unwrap_or_else(|| header.path_bytes().into_owned());
        longname = None;
        let modified = match local.modified {
            PaxModified::Set(time) => Some(time),
            // A local empty record deletes even the raw header value for this member.
            PaxModified::Delete => None,
            PaxModified::Unspecified => match global.modified {
                PaxModified::Set(time) => Some(time),
                // Global deletion removes the persistent override, restoring member headers.
                PaxModified::Delete | PaxModified::Unspecified => {
                    header_mtime(&header.as_old().mtime)
                }
            },
        };
        local = Extensions::default();
        pending_extension = false;
        if snapshot.is_none() {
            metadata = metadata
                .checked_add(1024 + (raw_name.len() as u64).saturating_mul(32))
                .ok_or(ArchiveError::ResourceLimit)?;
        }
        // Charge geometric locator capacity together with the catalog before allocation.
        if offsets.len() == offsets.capacity() {
            let capacity = offsets
                .capacity()
                .saturating_mul(2)
                .max(1)
                .min(usize::try_from(limits.max_entries).unwrap_or(usize::MAX));
            let bytes = (capacity as u64)
                .checked_mul(8)
                .ok_or(ArchiveError::ResourceLimit)?;
            if metadata.saturating_add(bytes) > limits.max_metadata_bytes
                || bytes.saturating_add(256 * 1024) > limits.max_working_bytes
            {
                return Err(ArchiveError::ResourceLimit);
            }
            offsets
                .try_reserve_exact(capacity - offsets.len())
                .map_err(|_| ArchiveError::ResourceLimit)?;
            observation.plan_bytes = (offsets.capacity() * std::mem::size_of::<u64>()) as u64;
        }
        if metadata.saturating_add(observation.plan_bytes) > limits.max_metadata_bytes {
            return Err(ArchiveError::ResourceLimit);
        }
        let (display_path, provenance) = if authoritative.is_some() {
            (
                String::from_utf8(raw_name.clone()).map_err(|_| ArchiveError::CorruptArchive)?,
                NameProvenance::FormatUnicode,
            )
        } else {
            crate::encoding::decode(
                &raw_name,
                std::str::from_utf8(&raw_name).is_ok(),
                &[],
                encoding,
            )?
        };
        let entry_kind = if kind.is_file() || kind.is_contiguous() {
            EntryKind::File
        } else if kind.is_dir() {
            EntryKind::Directory
        } else if kind.is_symlink() || kind.is_hard_link() {
            EntryKind::Link
        } else {
            EntryKind::Special
        };
        if entry_kind == EntryKind::Directory && size != 0 {
            return Err(ArchiveError::CorruptArchive);
        }
        let warnings = if matches!(entry_kind, EntryKind::Link | EntryKind::Special) {
            vec![EntryWarning::LinkNotExtracted]
        } else {
            Vec::new()
        };
        let entry = ArchiveEntry {
            id: EntryId {
                session,
                ordinal: offsets.len() as u64,
            },
            raw_name,
            display_path,
            provenance,
            kind: entry_kind,
            size: Some(size),
            compressed_size: None,
            method: 0,
            encrypted: false,
            crc32: None,
            dos_modified: None,
            metadata: crate::metadata::OutputMetadata {
                modified,
                attributes: None,
            },
            warnings,
            zip: None,
        };
        if let Some(snapshot) = snapshot {
            let old = snapshot
                .entries
                .get(offsets.len())
                .ok_or(ArchiveError::SourceChanged)?;
            if entry.raw_name != old.raw_name
                || entry.size != old.size
                || entry.kind != old.kind
                || entry.metadata != old.metadata
            {
                return Err(ArchiveError::SourceChanged);
            }
        } else {
            items.push(entry);
        }
        offsets.push(payload);
        offset = next;
    }
}
pub(crate) fn list_planned(
    file: &mut File,
    encoding: NameEncoding,
    limits: &ResourceLimits,
    control: &OperationControl,
) -> Result<(Vec<ArchiveEntry>, ValidatedPlan)> {
    let (mut entries, plan) = records(file, encoding, limits, control, None)?;
    crate::path_policy::annotate(
        &mut entries,
        control,
        plan.metadata_bytes,
        limits.max_metadata_bytes,
    )?;
    Ok((entries, plan))
}
pub(crate) fn validate(
    file: &mut File,
    snapshot: &ArchiveSnapshot,
    limits: &ResourceLimits,
    control: &OperationControl,
) -> Result<ValidatedPlan> {
    records(file, NameEncoding::Auto, limits, control, Some(snapshot)).map(|(_, plan)| plan)
}
fn copy(
    file: &mut File,
    (offset, entry): (u64, &ArchiveEntry),
    total: &mut u64,
    limits: &ResourceLimits,
    control: &OperationControl,
    mut chunk: impl FnMut(&[u8]) -> Result<()>,
) -> Result<u64> {
    let size = entry.size.ok_or(ArchiveError::CorruptArchive)?;
    if size > limits.max_entry_bytes {
        return Err(ArchiveError::ResourceLimit);
    }
    file.seek(SeekFrom::Start(offset))?;
    control.record_work(|work| work.tar_payload_seeks += 1);
    let mut remaining = size;
    let mut buffer = [0u8; 65536];
    while remaining > 0 {
        control.checkpoint()?;
        let length = remaining.min(buffer.len() as u64) as usize;
        exact(file, &mut buffer[..length])?;
        *total = total
            .checked_add(length as u64)
            .ok_or(ArchiveError::ResourceLimit)?;
        if *total > limits.max_output_bytes {
            return Err(ArchiveError::ResourceLimit);
        }
        chunk(&buffer[..length])?;
        remaining -= length as u64;
    }
    Ok(size)
}
pub(crate) fn test(
    mut file: File,
    snapshot: &ArchiveSnapshot,
    limits: &ResourceLimits,
    control: &OperationControl,
) -> Result<ArchiveOutcome> {
    let plan = validate(&mut file, snapshot, limits, control)?;
    test_validated(file, snapshot, &plan, limits, control)
}
pub(crate) fn test_validated(
    mut file: File,
    snapshot: &ArchiveSnapshot,
    plan: &ValidatedPlan,
    limits: &ResourceLimits,
    control: &OperationControl,
) -> Result<ArchiveOutcome> {
    plan.admit_execution(snapshot, limits)?;
    let mut bytes = 0;
    let mut files = 0;
    for entry in snapshot
        .entries
        .iter()
        .filter(|entry| entry.kind == EntryKind::File)
    {
        copy(
            &mut file,
            plan.locate(snapshot, entry.id)?,
            &mut bytes,
            limits,
            control,
            |_| Ok(()),
        )?;
        files += 1;
    }
    Ok(ArchiveOutcome {
        files_verified: files,
        bytes_verified: bytes,
    })
}
pub(crate) fn extract(
    mut file: File,
    snapshot: &ArchiveSnapshot,
    plan: &crate::planning::ExtractionPlan,
    limits: &ResourceLimits,
    control: &OperationControl,
    source: &mut File,
    output: &mut dyn crate::output::ArchiveOutput,
) -> Result<crate::output::ExtractOutcome> {
    let records = validate(&mut file, snapshot, limits, control)?;
    extract_validated(
        (file, &records),
        snapshot,
        plan,
        limits,
        control,
        source,
        output,
    )
}
pub(crate) fn extract_validated(
    (mut file, records): (File, &ValidatedPlan),
    snapshot: &ArchiveSnapshot,
    plan: &crate::planning::ExtractionPlan,
    limits: &ResourceLimits,
    control: &OperationControl,
    source: &mut File,
    output: &mut dyn crate::output::ArchiveOutput,
) -> Result<crate::output::ExtractOutcome> {
    records.admit_execution(snapshot, limits)?;
    if plan.session != snapshot.session
        || !std::sync::Arc::ptr_eq(&plan.projection, &snapshot.entries)
    {
        return Err(ArchiveError::SourceChanged);
    }
    let relative = |path: &str| {
        plan.subfolder()
            .map_or_else(|| path.to_owned(), |root| format!("{root}/{path}"))
    };
    if let Some(root) = plan.subfolder() {
        output.ensure_directory(root)?;
    }
    for directory in plan.directories() {
        control.checkpoint()?;
        output.ensure_directory(&relative(directory))?;
    }
    let mut pending = BTreeMap::new();
    let mut bytes = 0;
    let mut written_bytes = 0;
    let result = (|| {
        for planned in plan.files() {
            control.checkpoint()?;
            let (offset, entry) = records.locate(snapshot, planned.id())?;
            let token =
                output.begin_extracted_file(&relative(planned.relative_path()), entry.size)?;
            if let Some(token) = token {
                pending.insert(token, entry.metadata);
            }
            let count = copy(
                &mut file,
                (offset, entry),
                &mut bytes,
                limits,
                control,
                |chunk| {
                    if let Some(token) = token {
                        output.write_file(token, chunk)
                    } else {
                        Ok(())
                    }
                },
            )?;
            if token.is_some() {
                written_bytes += count;
            }
        }
        crate::engine::ensure_revision(source, &snapshot.revision, control)?;
        let mut files = 0;
        for token in pending.keys().copied().collect::<Vec<_>>() {
            control.checkpoint()?;
            output.set_file_metadata(token, pending[&token])?;
            output.commit_file(token)?;
            pending.remove(&token);
            files += 1;
        }
        Ok(crate::output::ExtractOutcome {
            files_committed: files,
            bytes_committed: written_bytes,
            skipped_special: plan.skipped_special(),
        })
    })();
    for token in pending.keys() {
        output.abort_file(*token);
    }
    result
}

/// Creation verification shares the fixed-header parser and bounded payload copier; no new parser.
pub(crate) fn verify_created(
    file: &mut File,
    plan: &crate::creation::CreationPlan,
    digests: &[Option<[u8; 32]>],
    limits: &ResourceLimits,
    control: &OperationControl,
) -> Result<()> {
    use sha2::{Digest, Sha256};
    let (entries, records) = records(file, NameEncoding::Utf8, limits, control, None)?;
    if entries.len() != plan.entries().len() || digests.len() != entries.len() {
        return Err(ArchiveError::CorruptArchive);
    }
    let mut total = 0;
    for (ordinal, ((entry, item), expected)) in
        entries.iter().zip(plan.entries()).zip(digests).enumerate()
    {
        if entry.display_path() != item.relative_path()
            || entry.size() != Some(item.size())
            || entry.kind()
                != if item.is_directory() {
                    EntryKind::Directory
                } else {
                    EntryKind::File
                }
        {
            return Err(ArchiveError::CorruptArchive);
        }
        let mut digest = Sha256::new();
        copy(
            file,
            (records.offsets[ordinal], entry),
            &mut total,
            limits,
            control,
            |bytes| {
                digest.update(bytes);
                Ok(())
            },
        )?;
        if *expected != Some(digest.finalize().into()) {
            return Err(ArchiveError::CorruptArchive);
        }
    }
    Ok(())
}

#[cfg(test)]
mod plan_tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn tar_locator_budget_is_additive_before_allocation() {
        let scratch = std::env::var_os("PI_SCRATCH_DIR").expect("session scratch directory");
        let mut source = tempfile::tempfile_in(scratch).expect("anonymous TAR fixture");
        let mut header = tar::Header::new_ustar();
        header.set_path("a").expect("ordinary path");
        header.set_entry_type(tar::EntryType::Regular);
        header.set_size(0);
        header.set_cksum();
        source.write_all(header.as_bytes()).expect("fixed header");
        source.write_all(&[0; 1024]).expect("complete TAR end");
        let control = OperationControl::default();
        let limits = ResourceLimits {
            max_metadata_bytes: 1024 + 32,
            ..Default::default()
        };
        assert!(matches!(
            list_planned(&mut source, NameEncoding::Utf8, &limits, &control),
            Err(ArchiveError::ResourceLimit)
        ));
        assert_eq!(control.work_observation().tar_record_scans, 1);
        assert_eq!(
            control.work_observation().tar_plan_bytes,
            0,
            "catalog-only credit cannot allocate the first locator"
        );
    }
}
