//! Compression is separate from the TAR edit catalog. Bounded full decode precedes admission.
use crate::{
    ArchiveError, OperationControl, ResourceLimits, Result,
    worker_tree_io::{Spool, io_error},
};
use std::{
    cell::Cell,
    fs::File,
    io::{BufReader, Read, Seek, Write},
};
#[derive(Clone, PartialEq, Eq)]
pub(crate) enum Outer {
    Plain,
    Gzip {
        mtime: u32,
        os: u8,
        name: Option<Vec<u8>>,
        comment: Option<Vec<u8>>,
        level: u32,
    },
    Xz {
        check: u8,
    },
}
impl Outer {
    pub(crate) fn open(
        mut source: File,
        format: &str,
        limits: &ResourceLimits,
        control: &OperationControl,
    ) -> Result<(Self, File)> {
        let mut signature = [0; 6];
        source
            .read_exact(&mut signature)
            .map_err(|_| ArchiveError::CorruptArchive)?;
        source.rewind()?;
        match format {
            "Tar" => {
                if signature[..2] == [0x1f, 0x8b] || signature == *b"\xfd7zXZ\0" {
                    return Err(ArchiveError::UnsupportedOperation);
                }
                Ok((Self::Plain, source))
            }
            "TarGz" => {
                if signature[..3] != [0x1f, 0x8b, 8] {
                    return Err(ArchiveError::UnsupportedOperation);
                }
                let mut spool = tempfile::tempfile()?;
                let outer_limits = ResourceLimits {
                    max_entry_bytes: limits.max_output_bytes,
                    ..*limits
                };
                let (_, members) = crate::formats::gzip::stream_members(
                    &mut source,
                    &outer_limits,
                    control,
                    |bytes| spool.write_all(bytes).map_err(io_error),
                )?;
                if members != 1 {
                    return Err(ArchiveError::UnsupportedOperation);
                }
                source.rewind()?;
                let mut fixed = [0; 10];
                source.read_exact(&mut fixed)?;
                // Unknown FEXTRA and header CRC/FTEXT semantics cannot be faithfully emitted by GzBuilder.
                if fixed[3] & 7 != 0 {
                    return Err(ArchiveError::UnsupportedOperation);
                }
                let level = match fixed[8] {
                    0 => 6,
                    2 => 9,
                    4 => 1,
                    _ => return Err(ArchiveError::UnsupportedOperation),
                };
                source.rewind()?;
                // The complete bounded parser has already validated the only <=64KiB header.
                let decoder = flate2::bufread::GzDecoder::new(BufReader::new(source));
                let header = decoder.header().ok_or(ArchiveError::CorruptArchive)?;
                let outer = Self::Gzip {
                    mtime: header.mtime(),
                    os: header.operating_system(),
                    name: header.filename().map(Vec::from),
                    comment: header.comment().map(Vec::from),
                    level,
                };
                spool.flush()?;
                spool.rewind()?;
                Ok((outer, spool))
            }
            "TarXz" => {
                if signature != *b"\xfd7zXZ\0" {
                    return Err(ArchiveError::UnsupportedOperation);
                }
                crate::formats::xz::edit_preflight(&mut source, limits, control)?;
                source.rewind()?;
                let mut fixed = [0; 12];
                source.read_exact(&mut fixed)?;
                let check = fixed[7];
                source.rewind()?;
                let spool = crate::formats::tgz::decode_xz(&mut source, limits, control)?;
                Ok((Self::Xz { check }, spool))
            }
            _ => Err(ArchiveError::UnsupportedOperation),
        }
    }
    pub(crate) fn encode<'a>(
        &self,
        mut plain: Spool<'a>,
        limits: &ResourceLimits,
        control: &'a OperationControl,
        failure: &'a Cell<Option<ArchiveError>>,
    ) -> Result<Spool<'a>> {
        if *self == Self::Plain {
            return Ok(plain);
        }
        plain.file.rewind()?;
        let output = Spool::new(limits.max_output_bytes, control, failure)?;
        let result = self.encode_to(&mut plain.file, output, limits, control);
        if let Some(error) = failure.get() {
            return Err(error);
        }
        result
    }
    fn encode_to<R: Read, W: Write>(
        &self,
        input: &mut R,
        output: W,
        limits: &ResourceLimits,
        control: &OperationControl,
    ) -> Result<W> {
        enum Encoder<W: Write> {
            Gzip(flate2::write::GzEncoder<W>),
            Xz(lzma_rust2::XzWriter<W>),
        }
        impl<W: Write> Write for Encoder<W> {
            fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
                match self {
                    Self::Gzip(w) => w.write(b),
                    Self::Xz(w) => w.write(b),
                }
            }
            fn flush(&mut self) -> std::io::Result<()> {
                match self {
                    Self::Gzip(w) => w.flush(),
                    Self::Xz(w) => w.flush(),
                }
            }
        }
        let mut encoder = match self {
            Self::Gzip {
                mtime,
                os,
                name,
                comment,
                level,
            } => {
                if limits.max_working_bytes < 4 << 20 {
                    return Err(ArchiveError::ResourceLimit);
                }
                let mut builder = flate2::GzBuilder::new().mtime(*mtime).operating_system(*os);
                if let Some(name) = name {
                    builder = builder.filename(name.clone());
                }
                if let Some(comment) = comment {
                    builder = builder.comment(comment.clone());
                }
                Encoder::Gzip(builder.write(output, flate2::Compression::new(*level)))
            }
            Self::Xz { check } => {
                let mut options = lzma_rust2::XzOptions::with_preset(1);
                if u64::from(options.lzma_options.dict_size) * 16 + (32 << 20)
                    > limits.max_working_bytes
                {
                    return Err(ArchiveError::ResourceLimit);
                }
                options.check_type = match check {
                    0 => lzma_rust2::CheckType::None,
                    1 => lzma_rust2::CheckType::Crc32,
                    4 => lzma_rust2::CheckType::Crc64,
                    10 => lzma_rust2::CheckType::Sha256,
                    _ => return Err(ArchiveError::UnsupportedMethod),
                };
                Encoder::Xz(lzma_rust2::XzWriter::new(output, options).map_err(io_error)?)
            }
            Self::Plain => return Err(ArchiveError::UnsupportedOperation),
        };
        let mut buffer = [0; 65536];
        loop {
            control.checkpoint()?;
            let n = input.read(&mut buffer)?;
            if n == 0 {
                break;
            }
            encoder.write_all(&buffer[..n]).map_err(io_error)?;
        }
        let finished = match encoder {
            Encoder::Gzip(w) => w.finish(),
            Encoder::Xz(w) => w.finish(),
        }
        .map_err(io_error);
        let mut encoded = finished?;
        encoded.flush().map_err(io_error)?;
        Ok(encoded)
    }
    pub(crate) fn format(&self) -> &'static str {
        match self {
            Self::Plain => "Tar",
            Self::Gzip { .. } => "TarGz",
            Self::Xz { .. } => "TarXz",
        }
    }
}

#[cfg(test)]
#[path = "tar_edit_outer_tests.rs"]
mod tests;
