//! Creation is worker-only: native source lease, exclusive token, complete read-back test, publish.
use crate::{
    ArchiveError, ArchiveProgress, OperationControl, Result,
    output::ArchiveOutput,
    worker::WorkerRequest,
    worker_output::{PrivateOutput, ReplyPort},
};
use sha2::{Digest, Sha256};
use std::{
    fs::File,
    io::{Read, Seek, Write},
};
use uuid::Uuid;

fn io_error(failure: std::io::Error) -> ArchiveError {
    failure
        .get_ref()
        .and_then(|value| value.downcast_ref::<ArchiveError>())
        .copied()
        .unwrap_or(ArchiveError::IoFailure)
}
struct NativeWriter<'b, 'a, W: Write>(&'b mut PrivateOutput<'a, W>, Uuid);
impl<W: Write + Send> Write for NativeWriter<'_, '_, W> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        for chunk in bytes.chunks(65536) {
            self.0
                .write_file(self.1, chunk)
                .map_err(std::io::Error::other)?;
        }
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
struct NativeReader<'b, 'a, W: Write> {
    output: &'b mut PrivateOutput<'a, W>,
    token: Uuid,
    offset: u64,
}
impl<W: Write + Send> Read for NativeReader<'_, '_, W> {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        if buffer.is_empty() {
            return Ok(0);
        }
        let bytes = self
            .output
            .read_partial(self.token, self.offset, buffer.len().min(65536) as u32)
            .map_err(std::io::Error::other)?;
        buffer[..bytes.len()].copy_from_slice(&bytes);
        self.offset += bytes.len() as u64;
        Ok(bytes.len())
    }
}
enum Encoder<W: Write> {
    Gzip(flate2::write::GzEncoder<W>),
    Xz(lzma_rust2::XzWriter<W>),
}
impl<W: Write> Write for Encoder<W> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        match self {
            Self::Gzip(writer) => writer.write(bytes),
            Self::Xz(writer) => writer.write(bytes),
        }
    }
    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            Self::Gzip(writer) => writer.flush(),
            Self::Xz(writer) => writer.flush(),
        }
    }
}
impl<W: Write> Encoder<W> {
    fn finish(self) -> std::io::Result<W> {
        match self {
            Self::Gzip(writer) => writer.finish(),
            Self::Xz(writer) => writer.finish(),
        }
    }
}
pub(crate) fn single_stream<W: Write + Send>(
    request: &WorkerRequest,
    control: &OperationControl,
    writer: &mut W,
    replies: ReplyPort,
) -> Result<(u64, u64)> {
    let lifecycle = control.source_delete_begin();
    let level = match request.compression_level.as_deref() {
        Some("fast") => 1,
        Some("normal") => 6,
        Some("maximum") => 9,
        _ => return Err(ArchiveError::UnsupportedOperation),
    };
    let is_xz = request.operation == "create_xz";
    let xz_options = if is_xz {
        let preset = match request.compression_level.as_deref() {
            Some("fast") => 1,
            Some("normal") => 6,
            Some("maximum") => 7,
            _ => return Err(ArchiveError::UnsupportedOperation),
        };
        let options = lzma_rust2::XzOptions::with_preset(preset);
        // Locked upstream estimate mixes byte-sized buffers and KiB finder terms. Reserve
        // conservatively from fixed-preset dictionary/finder allocations before construction.
        let memory = u64::from(options.lzma_options.dict_size) * 16 + 32 * 1024 * 1024;
        if memory > request.limits.max_working_bytes {
            return Err(ArchiveError::ResourceLimit);
        }
        Some(options)
    } else {
        None
    };
    if request.version != 2 || request.password.is_some() {
        return Err(ArchiveError::UnsupportedOperation);
    }
    let mut source = File::open(&request.path)?;
    let before = source.metadata()?;
    if !before.is_file() {
        return Err(ArchiveError::UnsupportedOperation);
    }
    let limits = request.limits.into();
    if before.len() > request.limits.max_entry_bytes
        || before.len() > request.limits.max_output_bytes
        || request.limits.max_working_bytes < 4 * 1024 * 1024
    {
        return Err(ArchiveError::ResourceLimit);
    }
    let mut native = PrivateOutput::new(writer, control, replies);
    native.source_checkpoint("source_validated")?;
    let token = native.begin_file(
        request
            .create_name
            .as_deref()
            .ok_or(ArchiveError::UnsupportedOperation)?,
    )?;
    let result = (|| {
        native.source_checkpoint("creation_encoding")?;
        let writer = NativeWriter(&mut native, token);
        let mut encoder = if let Some(options) = xz_options {
            Encoder::Xz(lzma_rust2::XzWriter::new(writer, options).map_err(io_error)?)
        } else {
            Encoder::Gzip(flate2::write::GzEncoder::new(
                writer,
                flate2::Compression::new(level),
            ))
        };
        let mut digest = Sha256::new();
        let mut count = 0u64;
        let mut buffer = [0u8; 65536];
        loop {
            control.checkpoint()?;
            let read = source.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            count = count
                .checked_add(read as u64)
                .ok_or(ArchiveError::ResourceLimit)?;
            if count > request.limits.max_entry_bytes || count > request.limits.max_output_bytes {
                return Err(ArchiveError::ResourceLimit);
            }
            digest.update(&buffer[..read]);
            encoder.write_all(&buffer[..read]).map_err(io_error)?;
            control.publish(ArchiveProgress {
                bytes: count,
                files: 0,
                total_bytes: Some(before.len()),
            });
        }
        encoder.finish().map_err(io_error)?;
        native.source_checkpoint("creation_verifying")?;
        let expected = digest.finalize();
        let mut actual = Sha256::new();
        let mut input = NativeReader {
            output: &mut native,
            token,
            offset: 0,
        };
        let verified = if is_xz {
            let mut spool = tempfile::tempfile()?;
            let mut compressed = 0u64;
            loop {
                control.checkpoint()?;
                let count = input.read(&mut buffer).map_err(io_error)?;
                if count == 0 {
                    break;
                }
                compressed = compressed
                    .checked_add(count as u64)
                    .ok_or(ArchiveError::ResourceLimit)?;
                if compressed > request.limits.max_output_bytes {
                    return Err(ArchiveError::ResourceLimit);
                }
                spool.write_all(&buffer[..count])?;
            }
            spool.flush()?;
            spool.rewind()?;
            crate::formats::xz::stream(&mut spool, &limits, control, |bytes| {
                actual.update(bytes);
                Ok(())
            })?
        } else {
            crate::formats::gzip::stream(&mut input, &limits, control, |bytes| {
                actual.update(bytes);
                Ok(())
            })?
        };
        if verified.bytes_verified() != count || actual.finalize() != expected {
            return Err(ArchiveError::CorruptArchive);
        }
        let after = source.metadata()?;
        if before.len() != after.len()
            || before.modified()? != after.modified()?
            || count != before.len()
        {
            return Err(ArchiveError::SourceChanged);
        }
        native.source_checkpoint("source_precommit")?;
        control.checkpoint()?;
        native.commit_file(token)?;
        control.source_delete_finish(&lifecycle, "eligible", vec![control.operation_id()]);
        control.finish_progress(ArchiveProgress {
            bytes: count,
            files: 1,
            total_bytes: Some(count),
        });
        Ok((1, count))
    })();
    if result.is_err() {
        native.abort_file(token);
    }
    result
}
