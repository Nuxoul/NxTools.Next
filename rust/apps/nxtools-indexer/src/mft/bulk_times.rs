//! Bounded, read-only hydration of true LastWriteTime from the enumerated MFT identities.
use anyhow::{Context, Result, ensure};

#[cfg(windows)]
mod native;
#[cfg(windows)]
pub(super) fn open(
    volume: windows_sys::Win32::Foundation::HANDLE,
    mount_point: &str,
) -> Result<BulkTimes<native::Volume>> {
    let (geometry, source) = native::open(volume, mount_point)?;
    BulkTimes::new(geometry, source)
}

#[cfg(all(test, windows))]
thread_local! {
    static FALLBACK_READS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

#[cfg(all(test, windows))]
pub(super) fn fallback_read_count() -> u64 {
    FALLBACK_READS.with(std::cell::Cell::get)
}

#[derive(Clone, Copy, Debug)]
pub(super) struct Geometry {
    sector: usize,
    cluster: usize,
    record: usize,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct Extent {
    start: u64,
    end: u64,
    physical: Option<u64>,
}

pub(super) trait Source {
    fn read_at(&mut self, offset: u64, output: &mut [u8]) -> Result<()>;
    fn extents(&mut self) -> Result<Vec<Extent>>;
    fn current_time(&mut self, reference: u64) -> Option<i64>;
}

pub(super) struct BulkTimes<S> {
    source: S,
    geometry: Geometry,
    extents: Vec<Extent>,
    // Raw volume I/O requires sector-aligned addresses as well as offsets/lengths.
    storage: Vec<u8>,
    aligned: usize,
    block_bytes: usize,
    cached: Option<(u64, usize)>,
    record: Vec<u8>,
    refreshed: bool,
    pub(super) reads: u64,
    pub(super) unknown: u64,
    pub(super) fallback_reads: u64,
}

impl<S: Source> BulkTimes<S> {
    fn new(geometry: Geometry, mut source: S) -> Result<Self> {
        let Geometry {
            sector,
            cluster,
            record,
        } = geometry;
        ensure!(
            (512..=65536).contains(&sector)
                && sector.is_power_of_two()
                && cluster >= sector
                && cluster.is_power_of_two()
                && record >= sector
                && record <= 1024 * 1024
                && record.is_power_of_two(),
            "unsupported NTFS metadata geometry"
        );
        let block_bytes = cluster.saturating_mul(256).clamp(sector, 1024 * 1024);
        let storage = vec![0; block_bytes + sector];
        let aligned = (sector - storage.as_ptr() as usize % sector) % sector;
        let extents = source.extents()?;
        validate_extents(&extents, sector)?;
        Ok(Self {
            source,
            geometry,
            extents,
            storage,
            aligned,
            block_bytes,
            cached: None,
            record: vec![0; record],
            refreshed: false,
            reads: 0,
            unknown: 0,
            fallback_reads: 0,
        })
    }

    pub(super) fn last_write(&mut self, reference: u64, latest_usn: i64) -> Result<Option<i64>> {
        let number = reference & ((1_u64 << 48) - 1);
        let start = number
            .checked_mul(self.geometry.record as u64)
            .context("MFT record offset overflow")?;
        let end = start
            .checked_add(self.geometry.record as u64)
            .context("MFT record end overflow")?;
        // Enumeration may observe growth after the initial map. Refresh once, without
        // turning missing/reused records into repeated metadata-control requests.
        if end > self.extents.last().map_or(0, |extent| extent.end) && !self.refreshed {
            self.extents = self.source.extents()?;
            validate_extents(&self.extents, self.geometry.sector)?;
            self.refreshed = true;
            self.cached = None;
        }
        let mut copied = 0;
        while copied < self.record.len() {
            let position = start + copied as u64;
            let index = self
                .extents
                .partition_point(|extent| extent.end <= position);
            let Some(extent) = self.extents.get(index).copied() else {
                self.unknown += 1;
                return Ok(None);
            };
            let Some(physical_start) = extent.physical else {
                self.unknown += 1;
                return Ok(None);
            };
            let (cache_start, cache_len) = match self.cached {
                Some((cache_start, cache_len))
                    if cache_start <= position && position < cache_start + cache_len as u64 =>
                {
                    (cache_start, cache_len)
                }
                _ => {
                    let block = (position - extent.start) / self.block_bytes as u64;
                    let cache_start = extent.start + block * self.block_bytes as u64;
                    let cache_len =
                        (extent.end - cache_start).min(self.block_bytes as u64) as usize;
                    let physical = physical_start
                        .checked_add(cache_start - extent.start)
                        .context("MFT extent disk offset overflow")?;
                    self.source.read_at(
                        physical,
                        &mut self.storage[self.aligned..self.aligned + cache_len],
                    )?;
                    self.reads += 1;
                    self.cached = Some((cache_start, cache_len));
                    (cache_start, cache_len)
                }
            };
            let offset = (position - cache_start) as usize;
            let count = (cache_len - offset).min(self.record.len() - copied);
            self.record[copied..copied + count].copy_from_slice(
                &self.storage[self.aligned + offset..self.aligned + offset + count],
            );
            copied += count;
        }
        let value = match record_metadata(&mut self.record, self.geometry.sector, reference) {
            Some(metadata) if metadata.usn == Some(latest_usn) => metadata.modified,
            Some(_) => {
                // Raw volume bytes can precede a cached NTFS metadata change that is
                // already before the replay cursor. Never persist that older time.
                self.fallback_reads += 1;
                self.source.current_time(reference)
            }
            None => None,
        };
        if value.is_none() {
            self.unknown += 1;
        }
        Ok(value)
    }
}

fn validate_extents(extents: &[Extent], sector: usize) -> Result<()> {
    let mut next = 0;
    for extent in extents {
        ensure!(
            extent.start == next
                && extent.end > extent.start
                && extent.start.is_multiple_of(sector as u64)
                && extent.end.is_multiple_of(sector as u64)
                && extent
                    .physical
                    .is_none_or(|offset| offset.is_multiple_of(sector as u64)),
            "invalid MFT retrieval extents"
        );
        next = extent.end;
    }
    ensure!(!extents.is_empty(), "MFT retrieval extents were empty");
    Ok(())
}

fn u16_at(bytes: &[u8], offset: usize) -> Option<u16> {
    Some(u16::from_le_bytes(
        bytes.get(offset..offset + 2)?.try_into().ok()?,
    ))
}
fn u32_at(bytes: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_le_bytes(
        bytes.get(offset..offset + 4)?.try_into().ok()?,
    ))
}
fn u64_at(bytes: &[u8], offset: usize) -> Option<u64> {
    Some(u64::from_le_bytes(
        bytes.get(offset..offset + 8)?.try_into().ok()?,
    ))
}

struct TimeMetadata {
    modified: Option<i64>,
    usn: Option<i64>,
}

#[cfg(test)]
fn record_last_write(record: &mut [u8], sector: usize, reference: u64) -> Option<i64> {
    record_metadata(record, sector, reference)?.modified
}

fn record_metadata(record: &mut [u8], sector: usize, reference: u64) -> Option<TimeMetadata> {
    if record.get(..4)? != b"FILE"
        || u16_at(record, 16)? != (reference >> 48) as u16
        || u16_at(record, 22)? & 1 == 0
        || u64_at(record, 32)? != 0
        || u32_at(record, 44)? != reference as u32
    {
        return None;
    }
    let used = u32_at(record, 24)? as usize;
    let mut attribute = u16_at(record, 20)? as usize;
    let usa = u16_at(record, 4)? as usize;
    let count = u16_at(record, 6)? as usize;
    if used > record.len()
        || attribute < 48
        || attribute >= used
        || count != record.len() / sector + 1
        || usa < 48
        || usa.checked_add(count * 2)? > attribute
    {
        return None;
    }
    let sequence = u16_at(record, usa)?;
    // Validate the whole multi-sector transfer before accepting any attribute.
    for index in 1..count {
        if u16_at(record, index * sector - 2)? != sequence {
            return None;
        }
    }
    for index in 1..count {
        let replacement = u16_at(record, usa + index * 2)?.to_le_bytes();
        record[index * sector - 2..index * sector].copy_from_slice(&replacement);
    }
    while attribute.checked_add(8)? <= used {
        let kind = u32_at(record, attribute)?;
        if kind == u32::MAX {
            return None;
        }
        let length = u32_at(record, attribute + 4)? as usize;
        let end = attribute.checked_add(length)?;
        if length < 24 || !length.is_multiple_of(8) || end > used {
            return None;
        }
        if kind == 0x10 {
            if record[attribute + 8] != 0 || record[attribute + 9] != 0 {
                return None;
            }
            let value_len = u32_at(record, attribute + 16)? as usize;
            let value_offset = u16_at(record, attribute + 20)? as usize;
            if value_offset < 24 || value_len < 48 || value_offset.checked_add(value_len)? > length
            {
                return None;
            }
            let ticks = u64_at(record, attribute + value_offset + 8)?;
            return Some(TimeMetadata {
                modified: super::time_metadata::filetime_to_unix_ms(ticks as i64),
                usn: (value_len >= 72)
                    .then(|| u64_at(record, attribute + value_offset + 64))
                    .flatten()
                    .map(|usn| usn as i64),
            });
        }
        attribute = end;
    }
    None
}

#[cfg(test)]
mod tests;
