use super::{
    ArchiveEntry, ArchiveError, EntryId, EntryKind, OperationControl, ResourceLimits, Result,
};
use std::{fs::File, mem::size_of};

pub(crate) struct ValidatedRecord {
    pub id: EntryId,
    pub data: u64,
    pub packed: u64,
    pub end: u64,
    pub header: Vec<u8>,
}

/// Exclusive reader borrow and immutable original entries keep record facts inside one attempt.
/// No independent file or caller-supplied offset can be substituted at the streaming boundary.
pub(crate) struct ValidatedZip<'a> {
    file: &'a mut File,
    entries: &'a [ArchiveEntry],
    records: Vec<ValidatedRecord>,
    working_bytes: u64,
    contiguous: bool,
}
impl<'a> ValidatedZip<'a> {
    pub(crate) fn new(
        file: &'a mut File,
        entries: &'a [ArchiveEntry],
        limits: &ResourceLimits,
        control: &OperationControl,
        retain_headers: bool,
    ) -> Result<Self> {
        let central = super::directory(file, limits)?;
        if central.count != entries.len() as u64 {
            return Err(ArchiveError::SourceChanged);
        }
        let record_bytes = (entries.len() as u64)
            .checked_mul(size_of::<ValidatedRecord>() as u64)
            .ok_or(ArchiveError::ResourceLimit)?;
        let range_bytes = (entries.len() as u64)
            .checked_mul(size_of::<(u64, u64)>() as u64)
            .ok_or(ArchiveError::ResourceLimit)?;
        let mut metadata_bytes = central
            .retained
            .checked_add(record_bytes)
            .ok_or(ArchiveError::ResourceLimit)?;
        let mut working_bytes = record_bytes;
        if metadata_bytes
            .checked_add(range_bytes)
            .is_none_or(|bytes| bytes > limits.max_metadata_bytes)
            || record_bytes
                .checked_add(range_bytes)
                .is_none_or(|bytes| bytes > limits.max_working_bytes)
        {
            return Err(ArchiveError::ResourceLimit);
        }
        let mut records = Vec::with_capacity(entries.len());
        let mut extents = Vec::with_capacity(entries.len());
        for (ordinal, entry) in entries.iter().enumerate() {
            control.checkpoint()?;
            if entry.id.ordinal != ordinal as u64
                || entries
                    .first()
                    .is_some_and(|first| first.id.session != entry.id.session)
            {
                return Err(ArchiveError::SourceChanged);
            }
            let credit = (limits.max_metadata_bytes - metadata_bytes - range_bytes)
                .min(limits.max_working_bytes - working_bytes - range_bytes);
            let (data, header) =
                super::local_record(file, entry, control, Some((credit, retain_headers)))?;
            let location = entry.zip.as_ref().ok_or(ArchiveError::CorruptArchive)?;
            let packed = entry.compressed_size.ok_or(ArchiveError::CorruptArchive)?;
            let end = super::validate_descriptor(file, entry, data, control)?;
            if end > central.offset || entry.kind == EntryKind::Directory && entry.size != Some(0) {
                return Err(ArchiveError::CorruptArchive);
            }
            metadata_bytes = metadata_bytes
                .checked_add(header.len() as u64)
                .ok_or(ArchiveError::ResourceLimit)?;
            working_bytes = working_bytes
                .checked_add(header.capacity() as u64)
                .ok_or(ArchiveError::ResourceLimit)?;
            extents.push((location.header_offset, end));
            records.push(ValidatedRecord {
                id: entry.id,
                data,
                packed,
                end,
                header,
            });
        }
        extents.sort_unstable();
        let mut previous_end = 0;
        let mut contiguous = true;
        for (start, end) in extents {
            control.checkpoint()?;
            if start < previous_end {
                return Err(ArchiveError::CorruptArchive);
            }
            contiguous &= start == previous_end;
            previous_end = end;
        }
        Ok(Self {
            file,
            entries,
            records,
            working_bytes,
            contiguous: contiguous && previous_end == central.offset,
        })
    }
    pub(super) fn working_bytes(&self) -> u64 {
        self.working_bytes
    }
    pub(super) fn parallel_reader(
        &self,
        snapshot: &crate::ArchiveSnapshot,
        lease: &crate::worker::NativeReadLease,
        limits: &ResourceLimits,
        control: &OperationControl,
    ) -> Result<ParallelReader<'_>> {
        if !self.matches(&snapshot.entries) || !lease.protects(snapshot) {
            return Err(ArchiveError::SourceChanged);
        }
        Ok(ParallelReader {
            file: crate::engine::open_snapshot(snapshot, limits, control)?,
            entries: self.entries,
            records: &self.records,
        })
    }
    pub(crate) fn matches(&self, entries: &[ArchiveEntry]) -> bool {
        std::ptr::eq(self.entries, entries)
    }
    pub(crate) fn records(&self) -> &[ValidatedRecord] {
        &self.records
    }
    pub(crate) fn record(&self, ordinal: u64) -> Result<&ValidatedRecord> {
        let index = usize::try_from(ordinal).map_err(|_| ArchiveError::SourceChanged)?;
        self.records
            .get(index)
            .filter(|record| {
                record.id.ordinal == ordinal
                    && self
                        .entries
                        .get(index)
                        .is_some_and(|entry| entry.id == record.id)
            })
            .ok_or(ArchiveError::SourceChanged)
    }
    pub(crate) fn file(&mut self) -> &mut File {
        self.file
    }
    pub(crate) fn contiguous(&self) -> bool {
        self.contiguous
    }
    pub(crate) fn stream(
        &mut self,
        ordinal: u64,
        limits: &ResourceLimits,
        control: &OperationControl,
        password: Option<&str>,
        total: &mut u64,
        on_chunk: impl FnMut(&[u8]) -> Result<()>,
    ) -> Result<u64> {
        let index = usize::try_from(ordinal).map_err(|_| ArchiveError::SourceChanged)?;
        let record = self.record(ordinal)?;
        let offset = record.data;
        let entry = self.entries.get(index).ok_or(ArchiveError::SourceChanged)?;
        if entry.compressed_size != Some(record.packed) {
            return Err(ArchiveError::SourceChanged);
        }
        let limits = ResourceLimits {
            max_working_bytes: limits
                .max_working_bytes
                .checked_sub(self.working_bytes)
                .filter(|bytes| *bytes >= 64 * 1024)
                .ok_or(ArchiveError::ResourceLimit)?,
            ..*limits
        };
        super::stream_at(
            self.file,
            (entry, Some(offset)),
            &limits,
            control,
            password,
            total,
            on_chunk,
        )
    }
}

/// Only the original validated proof can create an independently positioned cursor.
pub(super) struct ParallelReader<'a> {
    file: File,
    entries: &'a [ArchiveEntry],
    records: &'a [ValidatedRecord],
}
impl ParallelReader<'_> {
    pub(super) fn stream(
        &mut self,
        id: EntryId,
        limits: &ResourceLimits,
        control: &OperationControl,
        total: &mut u64,
        on_chunk: impl FnMut(&[u8]) -> Result<()>,
    ) -> Result<u64> {
        let index = usize::try_from(id.ordinal).map_err(|_| ArchiveError::SourceChanged)?;
        let entry = self
            .entries
            .get(index)
            .filter(|entry| entry.id == id)
            .ok_or(ArchiveError::SourceChanged)?;
        let record = self
            .records
            .get(index)
            .filter(|record| record.id == id && Some(record.packed) == entry.compressed_size)
            .ok_or(ArchiveError::SourceChanged)?;
        super::stream_at(
            &mut self.file,
            (entry, Some(record.data)),
            limits,
            control,
            None,
            total,
            on_chunk,
        )
    }
}
