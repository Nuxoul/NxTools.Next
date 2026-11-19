#![allow(unsafe_code)]

use std::mem::{offset_of, size_of};
use std::ptr::{null, null_mut};

use anyhow::{Context, Result, ensure};
use windows_sys::Win32::Foundation::{
    CloseHandle, ERROR_HANDLE_EOF, ERROR_MORE_DATA, GetLastError, HANDLE, INVALID_HANDLE_VALUE,
};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, FILE_BEGIN, FILE_READ_ATTRIBUTES, FILE_SHARE_DELETE, FILE_SHARE_READ,
    FILE_SHARE_WRITE, OPEN_EXISTING, ReadFile, SetFilePointerEx,
};
use windows_sys::Win32::System::IO::DeviceIoControl;
use windows_sys::Win32::System::Ioctl::{
    FSCTL_GET_NTFS_VOLUME_DATA, FSCTL_GET_RETRIEVAL_POINTERS, NTFS_VOLUME_DATA_BUFFER,
    RETRIEVAL_POINTERS_BUFFER, RETRIEVAL_POINTERS_BUFFER_0, STARTING_VCN_INPUT_BUFFER,
};

use super::{Extent, Geometry, Source};
use crate::mft::windows::win32_error;

const RETRIEVAL_BUFFER_BYTES: usize = 64 * 1024;
const RETRIEVAL_HEADER_BYTES: usize = offset_of!(RETRIEVAL_POINTERS_BUFFER, Extents);
const RETRIEVAL_EXTENT_BYTES: usize = size_of::<RETRIEVAL_POINTERS_BUFFER_0>();

/// Owns the metadata handle only. The caller must keep the volume handle open
/// and exclusively control its synchronous file position until this is dropped.
pub(in crate::mft) struct Volume {
    borrowed_volume: HANDLE,
    mft: HANDLE,
    cluster: u64,
}

impl Drop for Volume {
    fn drop(&mut self) {
        // SAFETY: open transfers this valid, uniquely owned metadata handle here.
        // borrowed_volume belongs to the caller and is deliberately not closed.
        unsafe {
            let _ = CloseHandle(self.mft);
        }
    }
}

pub(super) fn open(volume: HANDLE, mount_point: &str) -> Result<(Geometry, Volume)> {
    let mut data = NTFS_VOLUME_DATA_BUFFER::default();
    let mut returned = 0_u32;
    // SAFETY: both output pointers are live, correctly aligned, and have the
    // declared capacities. The caller keeps the synchronous volume handle live.
    let success = unsafe {
        DeviceIoControl(
            volume,
            FSCTL_GET_NTFS_VOLUME_DATA,
            null(),
            0,
            (&raw mut data).cast(),
            size_of::<NTFS_VOLUME_DATA_BUFFER>() as u32,
            &raw mut returned,
            null_mut(),
        )
    };
    if success == 0 {
        return Err(win32_error("query NTFS volume geometry", unsafe {
            GetLastError()
        }));
    }
    ensure!(
        returned as usize >= size_of::<NTFS_VOLUME_DATA_BUFFER>(),
        "NTFS volume geometry is truncated"
    );
    let geometry = Geometry {
        sector: data.BytesPerSector as usize,
        cluster: data.BytesPerCluster as usize,
        record: data.BytesPerFileRecordSegment as usize,
    };
    ensure!(
        (512..=4096).contains(&geometry.sector) && geometry.sector.is_power_of_two(),
        "NTFS sector size is unsupported"
    );
    ensure!(
        (geometry.sector..=2 * 1024 * 1024).contains(&geometry.cluster)
            && geometry.cluster.is_power_of_two()
            && geometry.cluster.is_multiple_of(geometry.sector),
        "NTFS cluster size is unsupported"
    );
    ensure!(
        (geometry.sector..=64 * 1024).contains(&geometry.record)
            && geometry.record.is_power_of_two()
            && geometry.record.is_multiple_of(geometry.sector),
        "NTFS file record size is unsupported"
    );

    let root = mount_point.trim_end_matches(['\\', '/']);
    ensure!(!root.is_empty(), "NTFS mount point is empty");
    ensure!(!root.contains('\0'), "NTFS mount point contains a null");
    let path = format!("{root}\\$MFT")
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    // SAFETY: path is a live, null-terminated UTF-16 buffer. No security or
    // template handle is supplied, and this opens only a synchronous metadata handle.
    let mft = unsafe {
        CreateFileW(
            path.as_ptr(),
            FILE_READ_ATTRIBUTES,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            null(),
            OPEN_EXISTING,
            0,
            null_mut(),
        )
    };
    if mft == INVALID_HANDLE_VALUE {
        return Err(win32_error("open NTFS MFT metadata", unsafe {
            GetLastError()
        }));
    }
    Ok((
        geometry,
        Volume {
            borrowed_volume: volume,
            mft,
            cluster: geometry.cluster as u64,
        },
    ))
}

impl Source for Volume {
    fn current_time(&mut self, reference: u64) -> Option<i64> {
        #[cfg(test)]
        super::FALLBACK_READS.with(|count| count.set(count.get() + 1));
        crate::mft::read_last_write_by_id(self.borrowed_volume, reference)
    }

    fn read_at(&mut self, offset: u64, output: &mut [u8]) -> Result<()> {
        let position = i64::try_from(offset).context("NTFS raw read offset is too large")?;
        let length = u32::try_from(output.len()).context("NTFS raw read is too large")?;
        // SAFETY: the caller retains this synchronous handle and serializes its
        // position; no output position pointer is needed.
        let success =
            unsafe { SetFilePointerEx(self.borrowed_volume, position, null_mut(), FILE_BEGIN) };
        if success == 0 {
            return Err(win32_error("seek NTFS raw volume", unsafe {
                GetLastError()
            }));
        }
        let mut read = 0_u32;
        // SAFETY: output is writable for length bytes. The safe reader supplies
        // sector-aligned offset, length and allocation; the handle stays live.
        let success = unsafe {
            ReadFile(
                self.borrowed_volume,
                output.as_mut_ptr(),
                length,
                &raw mut read,
                null_mut(),
            )
        };
        if success == 0 {
            return Err(win32_error("read NTFS raw volume", unsafe {
                GetLastError()
            }));
        }
        ensure!(read == length, "NTFS raw volume read is incomplete");
        Ok(())
    }

    fn extents(&mut self) -> Result<Vec<Extent>> {
        let mut request = STARTING_VCN_INPUT_BUFFER { StartingVcn: 0 };
        // u64 storage satisfies the native structure's alignment without casting
        // its variable-length records into Rust references.
        let mut buffer = vec![0_u64; RETRIEVAL_BUFFER_BYTES / size_of::<u64>()];
        let mut extents = Vec::new();
        loop {
            let mut returned = 0_u32;
            // SAFETY: request and returned are live aligned structures; buffer
            // has the declared writable byte capacity for this synchronous call.
            let success = unsafe {
                DeviceIoControl(
                    self.mft,
                    FSCTL_GET_RETRIEVAL_POINTERS,
                    (&raw const request).cast(),
                    size_of::<STARTING_VCN_INPUT_BUFFER>() as u32,
                    buffer.as_mut_ptr().cast(),
                    RETRIEVAL_BUFFER_BYTES as u32,
                    &raw mut returned,
                    null_mut(),
                )
            };
            let more = if success == 0 {
                let code = unsafe { GetLastError() };
                if code == ERROR_HANDLE_EOF && !extents.is_empty() {
                    return Ok(extents);
                }
                if code != ERROR_MORE_DATA {
                    return Err(win32_error("query NTFS MFT extents", code));
                }
                true
            } else {
                false
            };
            ensure!(
                (RETRIEVAL_HEADER_BYTES..=RETRIEVAL_BUFFER_BYTES).contains(&(returned as usize)),
                "NTFS MFT extent response has an invalid length"
            );
            // SAFETY: u64 has no uninitialized bytes and the checked returned
            // length is within the allocation. Read fields by byte copy below.
            let bytes = unsafe {
                std::slice::from_raw_parts(buffer.as_ptr().cast::<u8>(), returned as usize)
            };
            let count = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
            ensure!(
                count <= (bytes.len() - RETRIEVAL_HEADER_BYTES) / RETRIEVAL_EXTENT_BYTES,
                "NTFS MFT extent response is truncated"
            );
            let start_offset = offset_of!(RETRIEVAL_POINTERS_BUFFER, StartingVcn);
            let mut run_start = signed_field(&bytes[start_offset..start_offset + 8]);
            ensure!(
                run_start >= 0 && run_start <= request.StartingVcn,
                "NTFS MFT extents have a logical gap"
            );
            let mut covered = request.StartingVcn;
            for run in bytes[RETRIEVAL_HEADER_BYTES..]
                .chunks_exact(RETRIEVAL_EXTENT_BYTES)
                .take(count)
            {
                let next = signed_field(&run[..8]);
                let lcn = signed_field(&run[8..16]);
                ensure!(next > run_start, "NTFS MFT extent does not advance");
                ensure!(lcn >= -1, "NTFS MFT extent has an invalid physical cluster");
                // Windows may round StartingVcn down to an existing run. Omit
                // already mapped clusters and adjust its physical start in place.
                if next > request.StartingVcn {
                    let start = run_start.max(request.StartingVcn);
                    ensure!(start == covered, "NTFS MFT extents are not contiguous");
                    let physical = if lcn == -1 {
                        None
                    } else {
                        let physical_cluster = (lcn as u64)
                            .checked_add((start - run_start) as u64)
                            .context("NTFS MFT physical cluster overflows")?;
                        Some(self.cluster_bytes(physical_cluster)?)
                    };
                    let logical_start = self.cluster_bytes(start as u64)?;
                    let logical_end = self.cluster_bytes(next as u64)?;
                    if let Some(offset) = physical {
                        offset
                            .checked_add(logical_end - logical_start)
                            .context("NTFS MFT physical extent overflows")?;
                    }
                    extents.push(Extent {
                        start: logical_start,
                        end: logical_end,
                        physical,
                    });
                    covered = next;
                }
                run_start = next;
            }
            ensure!(
                covered > request.StartingVcn,
                "NTFS MFT extent query made no progress"
            );
            if !more {
                return Ok(extents);
            }
            request.StartingVcn = covered;
        }
    }
}

impl Volume {
    fn cluster_bytes(&self, clusters: u64) -> Result<u64> {
        clusters
            .checked_mul(self.cluster)
            .context("NTFS MFT extent byte offset overflows")
    }
}

fn signed_field(bytes: &[u8]) -> i64 {
    let mut field = [0_u8; 8];
    field.copy_from_slice(bytes);
    i64::from_le_bytes(field)
}
