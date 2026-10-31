use std::path::Path;

use sysinfo::Disks;
use thiserror::Error;

const MIB: u64 = 1024 * 1024;
const MINIMUM_RESERVE_BYTES: u64 = 64 * MIB;
const MAXIMUM_RESERVE_BYTES: u64 = 512 * MIB;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DownloadStorageRequirement {
    remaining_bytes: u64,
    reserve_bytes: u64,
    required_bytes: u64,
}

impl DownloadStorageRequirement {
    pub fn for_transfer(
        total_bytes: u64,
        durable_downloaded_bytes: u64,
    ) -> Result<Self, DownloadStorageCapacityError> {
        let remaining_bytes = total_bytes.saturating_sub(durable_downloaded_bytes);
        let reserve_bytes = if remaining_bytes == 0 {
            0
        } else {
            (total_bytes / 20).clamp(MINIMUM_RESERVE_BYTES, MAXIMUM_RESERVE_BYTES)
        };
        let required_bytes = remaining_bytes
            .checked_add(reserve_bytes)
            .ok_or(DownloadStorageCapacityError::RequirementOverflow)?;
        Ok(Self {
            remaining_bytes,
            reserve_bytes,
            required_bytes,
        })
    }

    pub fn ensure_available(
        self,
        available_bytes: u64,
    ) -> Result<(), DownloadStorageCapacityError> {
        if available_bytes < self.required_bytes {
            return Err(DownloadStorageCapacityError::Insufficient {
                available_bytes,
                required_bytes: self.required_bytes,
            });
        }
        Ok(())
    }

    pub const fn remaining_bytes(self) -> u64 {
        self.remaining_bytes
    }

    pub const fn reserve_bytes(self) -> u64 {
        self.reserve_bytes
    }

    pub const fn required_bytes(self) -> u64 {
        self.required_bytes
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum DownloadStorageCapacityError {
    #[error("download storage requirement overflowed")]
    RequirementOverflow,
    #[error(
        "insufficient disk space: {available_bytes} bytes available, {required_bytes} bytes required"
    )]
    Insufficient {
        available_bytes: u64,
        required_bytes: u64,
    },
}

pub(crate) trait StorageCapacityProbe: Send + Sync {
    fn available_space(&self, path: &Path) -> Option<u64>;
}

#[derive(Debug, Default)]
pub(crate) struct SystemStorageCapacityProbe;

impl StorageCapacityProbe for SystemStorageCapacityProbe {
    fn available_space(&self, path: &Path) -> Option<u64> {
        let disks = Disks::new_with_refreshed_list();
        disks
            .list()
            .iter()
            .filter(|disk| path_starts_with(path, disk.mount_point()))
            .max_by_key(|disk| disk.mount_point().as_os_str().len())
            .map(sysinfo::Disk::available_space)
    }
}

#[cfg(windows)]
fn path_starts_with(path: &Path, mount_point: &Path) -> bool {
    path.to_string_lossy()
        .to_lowercase()
        .starts_with(&mount_point.to_string_lossy().to_lowercase())
}

#[cfg(not(windows))]
fn path_starts_with(path: &Path, mount_point: &Path) -> bool {
    path.starts_with(mount_point)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overflow_is_rejected_instead_of_wrapping_required_space() {
        assert_eq!(
            DownloadStorageRequirement::for_transfer(u64::MAX, 0),
            Err(DownloadStorageCapacityError::RequirementOverflow)
        );
    }
}
