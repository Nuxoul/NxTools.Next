//! Native-source TAR creation. Explicit bounded headers/data, full held-partial verification.
use crate::{
    ArchiveError, ArchiveProgress, OperationControl, ResourceLimits, Result,
    creation::{CreationPlan, CreationSource},
    output::ArchiveOutput,
    worker::WorkerRequest,
    worker_output::{PrivateOutput, ReplyPort},
    worker_sources::{Reader, Sources},
    worker_tree_io::{Spool, io_error, stage_and_readback},
};
use sha2::{Digest, Sha256};
use std::{
    cell::{Cell, RefCell},
    io::{Read, Write},
};

enum Encoder<W: Write> {
    Plain(W),
    Gzip(flate2::write::GzEncoder<W>),
    Xz(lzma_rust2::XzWriter<W>),
}
impl<W: Write> Write for Encoder<W> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        match self {
            Self::Plain(writer) => writer.write(bytes),
            Self::Gzip(writer) => writer.write(bytes),
            Self::Xz(writer) => writer.write(bytes),
        }
    }
    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            Self::Plain(writer) => writer.flush(),
            Self::Gzip(writer) => writer.flush(),
            Self::Xz(writer) => writer.flush(),
        }
    }
}
impl<W: Write> Encoder<W> {
    fn finish(self) -> std::io::Result<W> {
        match self {
            Self::Plain(writer) => Ok(writer),
            Self::Gzip(writer) => writer.finish(),
            Self::Xz(writer) => writer.finish(),
        }
    }
}
struct TarWriter<'a, W: Write> {
    inner: W,
    count: u64,
    maximum: u64,
    control: &'a OperationControl,
    failure: &'a Cell<Option<ArchiveError>>,
}
impl<W: Write> Write for TarWriter<'_, W> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        let result = (|| {
            self.control.checkpoint()?;
            if self
                .count
                .checked_add(bytes.len() as u64)
                .is_none_or(|value| value > self.maximum)
            {
                return Err(ArchiveError::ResourceLimit);
            }
            let count = self.inner.write(bytes).map_err(io_error)?;
            self.count += count as u64;
            Ok(count)
        })();
        result.map_err(|error| {
            if self.failure.get().is_none() {
                self.failure.set(Some(error));
            }
            std::io::Error::other(error)
        })
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}
pub(crate) fn create<W: Write + Send>(
    request: &WorkerRequest,
    control: &OperationControl,
    writer: &mut W,
    replies: ReplyPort,
) -> Result<(u64, u64)> {
    let lifecycle = control.source_delete_begin();
    let limits: ResourceLimits = request.limits.into();
    let tree = request
        .tree_options
        .as_ref()
        .ok_or(ArchiveError::UnsupportedOperation)?;
    let xz_options = if request.operation == "create_txz" {
        let preset = match request.compression_level.as_deref() {
            Some("fast") => 1,
            Some("normal") => 6,
            Some("maximum") => 7,
            _ => return Err(ArchiveError::UnsupportedOperation),
        };
        let options = lzma_rust2::XzOptions::with_preset(preset);
        if u64::from(options.lzma_options.dict_size) * 16 + (32 << 20) > limits.max_working_bytes {
            return Err(ArchiveError::ResourceLimit);
        }
        Some(options)
    } else {
        None
    };
    let mut native = PrivateOutput::new(writer, control, replies);
    let shared = RefCell::new(&mut native);
    shared.borrow_mut().source_checkpoint("creation_planning")?;
    let mut sources = Sources::new(&shared);
    let plan = CreationPlan::build(&mut sources, tree, &limits, control)?;
    let sources = RefCell::new(sources);
    shared.borrow_mut().source_checkpoint("source_validated")?;
    let token = shared.borrow_mut().begin_file(
        request
            .create_name
            .as_deref()
            .ok_or(ArchiveError::UnsupportedOperation)?,
    )?;
    let result = (|| {
        shared.borrow_mut().source_checkpoint("creation_encoding")?;
        let failure = Cell::new(None);
        let digests = RefCell::new(vec![None; plan.entries().len()]);
        let spool = Spool::new(limits.max_output_bytes, control, &failure)?;
        let encoder = if request.operation == "create_tgz" {
            let level = match request.compression_level.as_deref() {
                Some("fast") => 1,
                Some("normal") => 6,
                Some("maximum") => 9,
                _ => return Err(ArchiveError::UnsupportedOperation),
            };
            Encoder::Gzip(flate2::write::GzEncoder::new(
                spool,
                flate2::Compression::new(level),
            ))
        } else if let Some(options) = xz_options {
            Encoder::Xz(lzma_rust2::XzWriter::new(spool, options).map_err(io_error)?)
        } else {
            Encoder::Plain(spool)
        };
        let mut archive = tar::Builder::new(TarWriter {
            inner: encoder,
            count: 0,
            maximum: limits.max_output_bytes,
            control,
            failure: &failure,
        });
        archive.follow_symlinks(false);
        for (index, item) in plan.entries().iter().enumerate() {
            control.checkpoint()?;
            let mut header = tar::Header::new_gnu();
            header.set_size(item.size());
            header.set_uid(0);
            header.set_gid(0);
            header.set_mtime(0);
            header.set_mode(if item.is_directory() { 0o755 } else { 0o644 });
            header.set_entry_type(if item.is_directory() {
                tar::EntryType::Directory
            } else {
                tar::EntryType::Regular
            });
            if item.is_directory() {
                digests.borrow_mut()[index] = Some(Sha256::digest([]).into());
                archive
                    .append_data(&mut header, item.relative_path(), std::io::empty())
                    .map_err(io_error)?;
            } else {
                let mut source = Reader::new(&sources, item, control, &failure, &digests, index);
                let appended = archive
                    .append_data(&mut header, item.relative_path(), &mut source)
                    .map_err(io_error);
                if let Some(error) = failure.get() {
                    return Err(error);
                }
                appended?;
                let mut eof = [0u8; 1];
                if source.read(&mut eof).map_err(io_error)? != 0 {
                    return Err(ArchiveError::SourceChanged);
                }
            }
        }
        let finished = archive.into_inner().map_err(io_error);
        if let Some(error) = failure.get() {
            return Err(error);
        }
        let finalized = finished?.inner.finish().map_err(io_error);
        if let Some(error) = failure.get() {
            return Err(error);
        }
        let mut encoded = finalized?;
        let mut spool = stage_and_readback(
            &mut encoded.file,
            &mut shared.borrow_mut(),
            token,
            limits.max_output_bytes,
            control,
        )?;
        if request.operation == "create_tgz" {
            spool = crate::formats::tgz::decode(&mut spool, &limits, control)?;
        }
        if request.operation == "create_txz" {
            spool = crate::formats::tgz::decode_xz(&mut spool, &limits, control)?;
        }
        crate::formats::tar::verify_created(
            &mut spool,
            &plan,
            &digests.borrow(),
            &limits,
            control,
        )?;
        shared.borrow_mut().source_checkpoint("source_precommit")?;
        sources.borrow_mut().revalidate()?;
        control.checkpoint()?;
        shared.borrow_mut().commit_file(token)?;
        plan.record_published_verification(control, &lifecycle);
        control.finish_progress(ArchiveProgress {
            bytes: plan.total_bytes(),
            files: 1,
            total_bytes: Some(plan.total_bytes()),
        });
        Ok((
            plan.entries()
                .iter()
                .filter(|item| !item.is_directory())
                .count() as u64,
            plan.total_bytes(),
        ))
    })();
    if result.is_err() {
        shared.borrow_mut().abort_file(token);
    }
    result
}
