//! Finite anonymous output spool and held-partial readback for tree codecs.
use crate::{ArchiveError, OperationControl, Result, worker_output::PrivateOutput};
use sha2::{Digest, Sha256};
use std::{
    cell::Cell,
    fs::File,
    io::{self, Read, Seek, SeekFrom, Write},
};
use uuid::Uuid;

pub(crate) fn io_error(error: io::Error) -> ArchiveError {
    error
        .get_ref()
        .and_then(|value| value.downcast_ref::<ArchiveError>())
        .copied()
        .unwrap_or(ArchiveError::IoFailure)
}
pub(crate) struct Spool<'a> {
    pub(crate) file: File,
    maximum: u64,
    control: &'a OperationControl,
    failure: &'a Cell<Option<ArchiveError>>,
}
impl<'a> Spool<'a> {
    pub(crate) fn new(
        maximum: u64,
        control: &'a OperationControl,
        failure: &'a Cell<Option<ArchiveError>>,
    ) -> Result<Self> {
        Ok(Self {
            file: tempfile::tempfile()?,
            maximum,
            control,
            failure,
        })
    }
    fn retain<T>(&self, result: Result<T>) -> io::Result<T> {
        result.map_err(|error| {
            if self.failure.get().is_none() {
                self.failure.set(Some(error));
            }
            io::Error::other(error)
        })
    }
}
impl Write for Spool<'_> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let result = (|| {
            self.control.checkpoint()?;
            let position = self.file.stream_position()?;
            if position
                .checked_add(bytes.len() as u64)
                .is_none_or(|end| end > self.maximum)
            {
                return Err(ArchiveError::ResourceLimit);
            }
            self.file.write(bytes).map_err(Into::into)
        })();
        self.retain(result)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()
    }
}
impl Seek for Spool<'_> {
    fn seek(&mut self, from: SeekFrom) -> io::Result<u64> {
        let result = (|| {
            self.control.checkpoint()?;
            let position = match from {
                SeekFrom::Start(value) => i128::from(value),
                SeekFrom::Current(value) => {
                    i128::from(self.file.stream_position()?) + i128::from(value)
                }
                SeekFrom::End(value) => i128::from(self.file.metadata()?.len()) + i128::from(value),
            };
            if position < 0 || position > i128::from(self.maximum) {
                return Err(ArchiveError::ResourceLimit);
            }
            self.file
                .seek(SeekFrom::Start(position as u64))
                .map_err(Into::into)
        })();
        self.retain(result)
    }
}
/// Copy encoded spool only; then independently read exactly the held native partial in fixed chunks.
/// Matching full encoded hash also prevents corrupted headers from reaching allocator-heavy readers.
pub(crate) fn stage_and_readback<W: Write + Send>(
    encoded: &mut File,
    native: &mut PrivateOutput<'_, W>,
    token: Uuid,
    maximum: u64,
    control: &OperationControl,
) -> Result<File> {
    use crate::output::ArchiveOutput;
    encoded.rewind()?;
    let mut expected = Sha256::new();
    let mut length = 0u64;
    let mut buffer = [0u8; 65536];
    loop {
        control.checkpoint()?;
        let count = encoded.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        length = length
            .checked_add(count as u64)
            .ok_or(ArchiveError::ResourceLimit)?;
        if length > maximum {
            return Err(ArchiveError::ResourceLimit);
        }
        expected.update(&buffer[..count]);
        native.write_file(token, &buffer[..count])?;
    }
    native.source_checkpoint("creation_verifying")?;
    let mut actual = Sha256::new();
    let mut offset = 0u64;
    let mut spool = tempfile::tempfile()?;
    loop {
        control.checkpoint()?;
        let bytes = native.read_partial(token, offset, 65536)?;
        if bytes.is_empty() {
            break;
        }
        offset = offset
            .checked_add(bytes.len() as u64)
            .ok_or(ArchiveError::ResourceLimit)?;
        if offset > maximum || offset > length {
            return Err(ArchiveError::CorruptArchive);
        }
        actual.update(&bytes);
        spool.write_all(&bytes)?;
    }
    if offset != length || actual.finalize() != expected.finalize() {
        return Err(ArchiveError::CorruptArchive);
    }
    spool.flush()?;
    spool.rewind()?;
    Ok(spool)
}
