use std::path::{Path, PathBuf};

use thiserror::Error;
use tokio::io::{AsyncSeekExt, AsyncWriteExt};

use crate::DownloadSegment;

#[derive(Debug, Clone)]
pub struct SegmentedArtifact {
    path: PathBuf,
    total_bytes: u64,
    allocation: ArtifactAllocation,
    #[cfg(test)]
    pub(crate) write_delay: std::time::Duration,
}

impl SegmentedArtifact {
    pub async fn create(
        path: impl AsRef<Path>,
        total_bytes: u64,
    ) -> Result<Self, SegmentedArtifactError> {
        let path = path.as_ref().to_path_buf();
        let create_path = path.clone();
        let allocation = tokio::task::spawn_blocking(move || {
            create_preallocated_file(&create_path, total_bytes)
        })
        .await
        .map_err(|error| SegmentedArtifactError::BackgroundTask(error.to_string()))??;
        Ok(Self {
            path,
            total_bytes,
            allocation,
            #[cfg(test)]
            write_delay: std::time::Duration::ZERO,
        })
    }

    pub async fn open_existing(
        path: impl AsRef<Path>,
        total_bytes: u64,
    ) -> Result<Self, SegmentedArtifactError> {
        let path = path.as_ref().to_path_buf();
        let actual_bytes = tokio::fs::metadata(&path).await?.len();
        if actual_bytes != total_bytes {
            return Err(SegmentedArtifactError::ArtifactLengthMismatch {
                expected_bytes: total_bytes,
                actual_bytes,
            });
        }
        Ok(Self {
            allocation: artifact_allocation(&path),
            path,
            total_bytes,
            #[cfg(test)]
            write_delay: std::time::Duration::ZERO,
        })
    }

    pub async fn open_segment(
        &self,
        segment: &DownloadSegment,
    ) -> Result<SegmentWriter, SegmentedArtifactError> {
        if segment.end_byte() >= self.total_bytes {
            return Err(SegmentedArtifactError::SegmentOutsideArtifact {
                segment_index: segment.index(),
                end_byte: segment.end_byte(),
                artifact_bytes: self.total_bytes,
            });
        }
        let position = segment
            .start_byte()
            .checked_add(segment.downloaded_bytes())
            .ok_or(SegmentedArtifactError::PositionOverflow)?;
        let path = self.path.clone();
        let file = tokio::task::spawn_blocking(move || open_segment_file(&path))
            .await
            .map_err(|error| SegmentedArtifactError::BackgroundTask(error.to_string()))??;
        let mut file = tokio::fs::File::from_std(file);
        file.seek(std::io::SeekFrom::Start(position)).await?;
        Ok(SegmentWriter {
            file,
            segment_index: segment.index(),
            remaining_bytes: segment.len() - segment.downloaded_bytes(),
            #[cfg(test)]
            fail_write_after: None,
            #[cfg(test)]
            write_delay: self.write_delay,
        })
    }

    pub(crate) async fn sync_data(&self) -> Result<(), SegmentedArtifactError> {
        let path = self.path.clone();
        tokio::task::spawn_blocking(move || open_segment_file(&path)?.sync_data())
            .await
            .map_err(|error| SegmentedArtifactError::BackgroundTask(error.to_string()))??;
        Ok(())
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub const fn total_bytes(&self) -> u64 {
        self.total_bytes
    }

    pub const fn allocation(&self) -> ArtifactAllocation {
        self.allocation
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArtifactAllocation {
    Normal,
    Sparse,
}

fn create_preallocated_file(
    path: &Path,
    total_bytes: u64,
) -> Result<ArtifactAllocation, std::io::Error> {
    let (file, allocation) = create_artifact_file(path)?;
    file.set_len(total_bytes)?;
    Ok(allocation)
}

#[cfg(windows)]
fn create_artifact_file(
    path: &Path,
) -> Result<(std::fs::File, ArtifactAllocation), std::io::Error> {
    use std::os::windows::fs::MetadataExt;

    const FILE_ATTRIBUTE_SPARSE_FILE: u32 = 0x0000_0200;

    let file = open_artifact_file(path, true)?;
    let allocation = if mark_file_sparse(&file).is_ok()
        && file.metadata()?.file_attributes() & FILE_ATTRIBUTE_SPARSE_FILE != 0
    {
        ArtifactAllocation::Sparse
    } else {
        ArtifactAllocation::Normal
    };
    Ok((file, allocation))
}

#[cfg(not(windows))]
fn create_artifact_file(
    path: &Path,
) -> Result<(std::fs::File, ArtifactAllocation), std::io::Error> {
    open_artifact_file(path, true).map(|file| (file, ArtifactAllocation::Normal))
}

fn open_artifact_file(path: &Path, truncate: bool) -> Result<std::fs::File, std::io::Error> {
    let mut options = std::fs::OpenOptions::new();
    options.create(true).truncate(truncate).write(true);
    configure_share_mode(&mut options);
    options.open(path)
}

fn open_segment_file(path: &Path) -> Result<std::fs::File, std::io::Error> {
    let mut options = std::fs::OpenOptions::new();
    options.write(true);
    configure_share_mode(&mut options);
    options.open(path)
}

#[cfg(windows)]
fn configure_share_mode(options: &mut std::fs::OpenOptions) {
    use std::os::windows::fs::OpenOptionsExt;

    options.share_mode(windows_share_mode());
}

#[cfg(not(windows))]
fn configure_share_mode(_options: &mut std::fs::OpenOptions) {}

#[cfg(windows)]
const fn windows_share_mode() -> u32 {
    const FILE_SHARE_READ: u32 = 0x0000_0001;
    const FILE_SHARE_WRITE: u32 = 0x0000_0002;
    FILE_SHARE_READ | FILE_SHARE_WRITE
}

#[cfg(windows)]
#[allow(unsafe_code)]
fn mark_file_sparse(file: &std::fs::File) -> Result<(), std::io::Error> {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::System::IO::DeviceIoControl;
    use windows_sys::Win32::System::Ioctl::FSCTL_SET_SPARSE;

    let mut returned_bytes = 0_u32;
    // SAFETY: `file` owns a valid synchronous file handle for the whole call. This FSCTL accepts
    // null input/output buffers to set the sparse flag, while `returned_bytes` is writable.
    let succeeded = unsafe {
        DeviceIoControl(
            file.as_raw_handle(),
            FSCTL_SET_SPARSE,
            std::ptr::null(),
            0,
            std::ptr::null_mut(),
            0,
            &mut returned_bytes,
            std::ptr::null_mut(),
        )
    };
    if succeeded == 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

fn artifact_allocation(path: &Path) -> ArtifactAllocation {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;

        const FILE_ATTRIBUTE_SPARSE_FILE: u32 = 0x0000_0200;
        if std::fs::metadata(path)
            .is_ok_and(|metadata| metadata.file_attributes() & FILE_ATTRIBUTE_SPARSE_FILE != 0)
        {
            return ArtifactAllocation::Sparse;
        }
    }
    ArtifactAllocation::Normal
}

#[derive(Debug)]
pub struct SegmentWriter {
    file: tokio::fs::File,
    segment_index: u32,
    remaining_bytes: u64,
    #[cfg(test)]
    fail_write_after: Option<usize>,
    #[cfg(test)]
    write_delay: std::time::Duration,
}

impl SegmentWriter {
    #[cfg(test)]
    pub(crate) fn fail_next_write_after(&mut self, prefix: usize) {
        self.fail_write_after = Some(prefix);
    }

    pub async fn write_all(&mut self, bytes: &[u8]) -> Result<(), SegmentedArtifactError> {
        let attempted_bytes =
            u64::try_from(bytes.len()).map_err(|_| SegmentedArtifactError::PositionOverflow)?;
        if attempted_bytes > self.remaining_bytes {
            return Err(SegmentedArtifactError::SegmentWriteOutOfBounds {
                segment_index: self.segment_index,
                remaining_bytes: self.remaining_bytes,
                attempted_bytes,
            });
        }
        #[cfg(test)]
        tokio::time::sleep(self.write_delay).await;
        #[cfg(test)]
        if let Some(prefix) = self.fail_write_after.take() {
            self.file
                .write_all(&bytes[..prefix.min(bytes.len())])
                .await?;
            self.file.flush().await?;
            return Err(std::io::Error::other("injected partial write failure").into());
        }
        self.file.write_all(bytes).await?;
        self.remaining_bytes -= attempted_bytes;
        Ok(())
    }

    pub(crate) async fn flush(&mut self) -> Result<(), SegmentedArtifactError> {
        self.file.flush().await?;
        Ok(())
    }

    pub async fn sync(&mut self) -> Result<(), SegmentedArtifactError> {
        self.file.flush().await?;
        self.file.sync_data().await?;
        Ok(())
    }

    pub const fn remaining_bytes(&self) -> u64 {
        self.remaining_bytes
    }
}

#[derive(Debug, Error)]
pub enum SegmentedArtifactError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("segment {segment_index} ends at {end_byte}, outside artifact length {artifact_bytes}")]
    SegmentOutsideArtifact {
        segment_index: u32,
        end_byte: u64,
        artifact_bytes: u64,
    },
    #[error(
        "segment {segment_index} has {remaining_bytes} bytes remaining but write attempted {attempted_bytes} bytes"
    )]
    SegmentWriteOutOfBounds {
        segment_index: u32,
        remaining_bytes: u64,
        attempted_bytes: u64,
    },
    #[error(
        "artifact length does not match transfer plan: expected {expected_bytes}, actual {actual_bytes}"
    )]
    ArtifactLengthMismatch {
        expected_bytes: u64,
        actual_bytes: u64,
    },
    #[error("segment artifact position overflowed")]
    PositionOverflow,
    #[error("artifact file operation worker failed: {0}")]
    BackgroundTask(String),
}
