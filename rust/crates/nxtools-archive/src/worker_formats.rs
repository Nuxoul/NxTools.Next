//! Additional readers live only in the native-admitted isolated worker.
use crate::{
    ArchiveEntry, ArchiveError, EntryId, EntryKind, EntryWarning, NameProvenance, OperationControl,
    ResourceLimits, Result,
    output::ArchiveOutput,
    worker::WorkerRequest,
    worker_output::{PrivateOutput, ReplyPort},
};
use archive_core::{self, EntryKind as CoreKind};
use std::{
    collections::HashSet,
    fs::File,
    io::{self, Read, Seek, SeekFrom, Write},
    path::Path,
    sync::Arc,
};
use uuid::Uuid;

type Core = archive_core::Archive<File>;
type Wim = archive_core::wim::FileWimArchive<File>;

enum Reader {
    Alz(crate::worker_alzeg::Alz),
    Egg(crate::worker_alzeg::Egg),
    Cab(Core),
    Iso(libmkiso::iso9660::IsoReader<File>),
    Udf(archive_core::udf::UdfArchive<'static>),
    Wim(Vec<Wim>, Vec<(usize, usize)>),
    Ar,
    Cpio,
    Lzma(u32),
    Lz4Frame,
    Lz4Block(u64),
    Brotli,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Family {
    Alz,
    Egg,
    Cab,
    Iso,
    Udf,
    Wim,
    Ar,
    Cpio,
    Lzma,
    Lz4Frame,
    Lz4Block,
    Brotli,
}
impl Family {
    fn label(self) -> &'static str {
        match self {
            Self::Alz => "Alz",
            Self::Egg => "Egg",
            Self::Cab => "Cab",
            Self::Iso => "Iso",
            Self::Udf => "Udf",
            Self::Wim => "Wim",
            Self::Ar => "Ar",
            Self::Cpio => "Cpio",
            Self::Lzma => "Lzma",
            Self::Lz4Frame => "Lz4",
            Self::Lz4Block => "Lz4Block",
            Self::Brotli => "Brotli",
        }
    }
}

fn detect(path: &Path, file: &mut File) -> Result<Option<Family>> {
    let mut signature = [0; 16];
    let n = file.read(&mut signature)?;
    file.rewind()?;
    let name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if signature.starts_with(b"ALZ\x01") {
        return Ok(Some(Family::Alz));
    }
    if signature.starts_with(b"EGGA") {
        return Ok(Some(Family::Egg));
    }
    if signature.starts_with(b"MSCF") {
        return Ok(Some(Family::Cab));
    }
    if signature.starts_with(b"MSWIM\0\0\0") {
        return Ok(Some(Family::Wim));
    }
    if signature.starts_with(b"!<arch>\n") {
        return Ok(Some(Family::Ar));
    }
    if [b"070701".as_slice(), b"070702", b"070707"]
        .iter()
        .any(|magic| signature.starts_with(magic))
        || signature.starts_with(&[0xc7, 0x71])
        || signature.starts_with(&[0x71, 0xc7])
    {
        return Ok(Some(Family::Cpio));
    }
    if signature.starts_with(&[0x04, 0x22, 0x4d, 0x18]) {
        return Ok(Some(Family::Lz4Frame));
    }
    if name.ends_with(".lz4") && n >= 5 {
        return Ok(Some(Family::Lz4Block));
    }
    if name.ends_with(".lzma") && n >= 13 && signature[0] < 225 {
        return Ok(Some(Family::Lzma));
    }
    if name.ends_with(".br") {
        return Ok(Some(Family::Brotli));
    }
    if (name.ends_with(".iso") || name.ends_with(".img") || name.ends_with(".udf"))
        && file.metadata()?.len() >= 32775
    {
        file.seek(SeekFrom::Start(32768))?;
        let mut descriptor = [0u8; 7];
        file.read_exact(&mut descriptor)?;
        let iso = &descriptor[1..6] == b"CD001";
        // Hybrid images have both ISO and UDF descriptors; prefer the UDF namespace.
        for block in 16..=32 {
            file.seek(SeekFrom::Start(block * 2048 + 1))?;
            let mut id = [0; 5];
            if file.read(&mut id)? != id.len() {
                break;
            }
            if &id == b"NSR02" || &id == b"NSR03" {
                return Ok(Some(Family::Udf));
            }
        }
        if iso {
            return Ok(Some(Family::Iso));
        }
    }
    Ok(None)
}

fn core_limits(limits: ResourceLimits, source_size: u64) -> archive_core::Limits {
    archive_core::Limits {
        max_entries: limits.max_entries,
        max_metadata_bytes: limits.max_metadata_bytes,
        max_entry_bytes: limits.max_entry_bytes,
        max_total_bytes: limits.max_output_bytes,
        max_input_bytes: source_size,
        max_buffered_bytes: limits.max_working_bytes,
        max_dictionary_bytes: limits.max_working_bytes,
        max_active_workspace_bytes: limits.max_working_bytes,
        max_pending_output_bytes: 65536,
        max_workers: 1,
        ..archive_core::Limits::default()
    }
}
fn core_error(error: archive_core::Error) -> ArchiveError {
    match error {
        archive_core::Error::ResourceLimit(_) => ArchiveError::ResourceLimit,
        archive_core::Error::Unsupported(_) => ArchiveError::UnsupportedMethod,
        archive_core::Error::PasswordRequired => ArchiveError::PasswordRequired,
        archive_core::Error::Cancelled => ArchiveError::Cancelled,
        archive_core::Error::Io(_) => ArchiveError::IoFailure,
        archive_core::Error::Integrity(_) | archive_core::Error::Malformed(_) => {
            ArchiveError::CorruptArchive
        }
    }
}
fn iso_error(error: libmkiso::iso9660::Error) -> ArchiveError {
    match error {
        libmkiso::iso9660::Error::ResourceLimit(_) => ArchiveError::ResourceLimit,
        libmkiso::iso9660::Error::Unsupported(_) => ArchiveError::UnsupportedMethod,
        libmkiso::iso9660::Error::Io(_) => ArchiveError::IoFailure,
        libmkiso::iso9660::Error::Malformed(_) => ArchiveError::CorruptArchive,
    }
}
fn item(
    request: &WorkerRequest,
    index: usize,
    name: String,
    kind: EntryKind,
    size: Option<u64>,
    packed: Option<u64>,
) -> ArchiveEntry {
    ArchiveEntry {
        id: EntryId {
            session: request.operation_id,
            ordinal: index as u64,
        },
        raw_name: name.as_bytes().to_vec(),
        display_path: name,
        provenance: NameProvenance::FormatUnicode,
        kind,
        size,
        compressed_size: packed,
        method: 0,
        encrypted: false,
        crc32: None,
        dos_modified: None,
        metadata: Default::default(),
        warnings: if matches!(kind, EntryKind::Link | EntryKind::Special) {
            vec![EntryWarning::LinkNotExtracted]
        } else {
            Vec::new()
        },
        zip: None,
    }
}
fn kind(value: CoreKind) -> EntryKind {
    match value {
        CoreKind::File => EntryKind::File,
        CoreKind::Directory => EntryKind::Directory,
        CoreKind::Link => EntryKind::Link,
        CoreKind::Other => EntryKind::Special,
    }
}
fn listed(reader: &[archive_core::Entry], request: &WorkerRequest) -> Vec<ArchiveEntry> {
    reader
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            item(
                request,
                index,
                entry.name.clone(),
                kind(entry.kind),
                Some(entry.size),
                entry.compressed_size,
            )
        })
        .collect()
}
fn ar_entries(
    path: &Path,
    request: &WorkerRequest,
    control: &OperationControl,
) -> Result<Vec<ArchiveEntry>> {
    let mut archive = ar::Archive::new(File::open(path)?);
    let mut result = Vec::new();
    while let Some(entry) = archive.next_entry() {
        control.checkpoint()?;
        let entry = entry.map_err(|_| ArchiveError::CorruptArchive)?;
        let name = std::str::from_utf8(entry.header().identifier())
            .map_err(|_| ArchiveError::UnsupportedMethod)?
            .to_owned();
        result.push(item(
            request,
            result.len(),
            name,
            EntryKind::File,
            Some(entry.header().size()),
            Some(entry.header().size()),
        ));
        if result.len() as u64 > request.limits.max_entries {
            return Err(ArchiveError::ResourceLimit);
        }
    }
    Ok(result)
}
fn cpio_entries(
    path: &Path,
    request: &WorkerRequest,
    control: &OperationControl,
) -> Result<Vec<ArchiveEntry>> {
    let mut archive = kpea::Archive::new(File::open(path)?);
    archive.verify_crc(true);
    let mut result = Vec::new();
    while let Some(entry) = archive
        .read_entry()
        .map_err(|_| ArchiveError::CorruptArchive)?
    {
        control.checkpoint()?;
        let raw_name = entry.path.as_bytes();
        let (name, provenance) = crate::encoding::decode(
            raw_name,
            std::str::from_utf8(raw_name).is_ok(),
            &[],
            request
                .encoding()
                .map_err(|_| ArchiveError::UnsupportedOperation)?,
        )?;
        let kind = if entry.metadata.is_file() && entry.metadata.nlink() > 1 {
            EntryKind::Link
        } else if entry.metadata.is_file() {
            EntryKind::File
        } else if entry.metadata.is_dir() {
            EntryKind::Directory
        } else if entry.metadata.is_symlink() {
            EntryKind::Link
        } else {
            EntryKind::Special
        };
        let mut record = item(
            request,
            result.len(),
            name,
            kind,
            Some(entry.metadata.size()),
            Some(entry.metadata.size()),
        );
        record.raw_name = raw_name.to_vec();
        record.provenance = provenance;
        if matches!(provenance, NameProvenance::UncertainCp437) {
            record.warnings.push(EntryWarning::UncertainEncoding);
        }
        result.push(record);
        if result.len() as u64 > request.limits.max_entries {
            return Err(ArchiveError::ResourceLimit);
        }
    }
    Ok(result)
}
fn open_reader(
    family: Family,
    request: &WorkerRequest,
    control: &OperationControl,
    size: u64,
    sources: Option<&crate::worker_codecs::volumes::SourceSet>,
) -> Result<(Reader, Vec<ArchiveEntry>)> {
    let path = Path::new(&request.path);
    let limits = crate::worker_browser::decode_limits(request);
    let core_limits = core_limits(limits, size);
    match family {
        Family::Alz => {
            let (reader, entries) =
                crate::worker_alzeg::Alz::open(request, control, sources.expect("ALZ source set"))?;
            Ok((Reader::Alz(reader), entries))
        }
        Family::Egg => {
            let (reader, entries) =
                crate::worker_alzeg::Egg::open(request, control, sources.expect("EGG source set"))?;
            Ok((Reader::Egg(reader), entries))
        }
        Family::Cab => {
            let archive = Core::open_as(File::open(path)?, archive_core::Format::Cab, core_limits)
                .map_err(core_error)?;
            let entries = listed(archive.entries(), request);
            Ok((Reader::Cab(archive), entries))
        }
        Family::Iso => {
            let reader = libmkiso::iso9660::IsoReader::open_with_options(
                File::open(path)?,
                libmkiso::iso9660::ReadOptions {
                    namespace: libmkiso::iso9660::Namespace::PreferRockRidge,
                    limits: libmkiso::iso9660::Limits {
                        max_entries: limits.max_entries,
                        max_metadata_bytes: limits.max_metadata_bytes,
                        ..Default::default()
                    },
                },
            )
            .map_err(iso_error)?;
            let entries = reader
                .entries()
                .iter()
                .enumerate()
                .map(|(index, entry)| {
                    item(
                        request,
                        index,
                        entry.name.clone(),
                        if entry.link_target.is_some() {
                            EntryKind::Link
                        } else if entry.directory {
                            EntryKind::Directory
                        } else {
                            EntryKind::File
                        },
                        Some(entry.size),
                        Some(entry.size),
                    )
                })
                .collect();
            Ok((Reader::Iso(reader), entries))
        }
        Family::Udf => {
            let reader = archive_core::udf::UdfArchive::open_reader(File::open(path)?, core_limits)
                .map_err(core_error)?;
            let entries = listed(reader.entries(), request);
            Ok((Reader::Udf(reader), entries))
        }
        Family::Wim => {
            let images = archive_core::wim::images_reader(File::open(path)?, core_limits)
                .map_err(core_error)?;
            let multiple = images.len() > 1;
            let mut readers = Vec::new();
            let mut members = Vec::new();
            let mut entries = Vec::new();
            let mut retained = 0u64;
            for image in images {
                control.checkpoint()?;
                let remaining = archive_core::Limits {
                    max_entries: limits.max_entries.saturating_sub(entries.len() as u64),
                    max_metadata_bytes: limits.max_metadata_bytes.saturating_sub(retained),
                    ..core_limits
                };
                let reader = Wim::open_reader(File::open(path)?, image.index, remaining)
                    .map_err(core_error)?;
                for (local, entry) in reader.entries().iter().enumerate() {
                    let name = if multiple {
                        format!("{}/{}", image.index, entry.name)
                    } else {
                        entry.name.clone()
                    };
                    retained = retained
                        .checked_add(528 + (name.len() as u64) * 8)
                        .ok_or(ArchiveError::ResourceLimit)?;
                    if retained > limits.max_metadata_bytes
                        || entries.len() as u64 >= limits.max_entries
                    {
                        return Err(ArchiveError::ResourceLimit);
                    }
                    members.push((readers.len(), local));
                    entries.push(item(
                        request,
                        entries.len(),
                        name,
                        kind(entry.kind),
                        Some(entry.size),
                        entry.compressed_size,
                    ));
                }
                readers.push(reader);
            }
            Ok((Reader::Wim(readers, members), entries))
        }
        Family::Ar => Ok((Reader::Ar, ar_entries(path, request, control)?)),
        Family::Cpio => Ok((Reader::Cpio, cpio_entries(path, request, control)?)),
        Family::Lzma | Family::Lz4Frame | Family::Lz4Block | Family::Brotli => {
            let name = crate::worker_job::stem(&request.path);
            let packed = Some(size);
            let declared = if family == Family::Lz4Block {
                let mut input = File::open(path)?;
                let mut bytes = [0; 4];
                input
                    .read_exact(&mut bytes)
                    .map_err(|_| ArchiveError::CorruptArchive)?;
                Some(u64::from(u32::from_le_bytes(bytes)))
            } else {
                None
            };
            Ok((
                match family {
                    Family::Lzma => Reader::Lzma(
                        (limits.max_working_bytes / 1024).min(u64::from(u32::MAX)) as u32,
                    ),
                    Family::Lz4Frame => Reader::Lz4Frame,
                    Family::Lz4Block => Reader::Lz4Block(limits.max_working_bytes),
                    _ => Reader::Brotli,
                },
                vec![item(request, 0, name, EntryKind::File, declared, packed)],
            ))
        }
    }
}

// Decoder callbacks only receive a fixed-size Write adapter. Keep the first typed native/cancel
// error even when a third-party decoder wraps it in a generic I/O error.
struct MemberWriter<'a> {
    output: Option<&'a mut dyn ArchiveOutput>,
    token: Option<Uuid>,
    control: &'a OperationControl,
    limit: u64,
    written: u64,
    failure: Option<ArchiveError>,
}
impl Write for MemberWriter<'_> {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        if let Some(failure) = self.failure {
            return Err(io::Error::other(format!("{failure:?}")));
        }
        let result = (|| {
            self.control.checkpoint()?;
            self.written = self
                .written
                .checked_add(data.len() as u64)
                .ok_or(ArchiveError::ResourceLimit)?;
            if self.written > self.limit {
                return Err(ArchiveError::ResourceLimit);
            }
            if let (Some(output), Some(token)) = (&mut self.output, self.token) {
                for chunk in data.chunks(65536) {
                    output.write_file(token, chunk)?;
                }
            }
            Ok(())
        })();
        if let Err(error) = result {
            self.failure = Some(error);
            return Err(io::Error::other(format!("{error:?}")));
        }
        Ok(data.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
fn copy_decoded(input: &mut impl Read, writer: &mut MemberWriter<'_>) -> Result<u64> {
    let mut bytes = [0u8; 65536];
    loop {
        writer.control.checkpoint()?;
        let n = input
            .read(&mut bytes)
            .map_err(|_| ArchiveError::CorruptArchive)?;
        if n == 0 {
            return Ok(writer.written);
        }
        if writer.write_all(&bytes[..n]).is_err() {
            return Err(writer.failure.unwrap_or(ArchiveError::IoFailure));
        }
    }
}
impl Reader {
    fn extract(&mut self, index: usize, path: &Path, writer: &mut MemberWriter<'_>) -> Result<u64> {
        match self {
            Self::Alz(reader) => reader.extract(index, writer),
            Self::Egg(reader) => reader.extract(index, writer),
            Self::Cab(reader) => reader
                .extract(archive_core::EntryId(index), writer)
                .map(|r| r.bytes)
                .map_err(core_error),
            Self::Iso(reader) => reader.extract(index, writer).map_err(iso_error),
            Self::Udf(reader) => reader
                .extract(archive_core::EntryId(index), writer)
                .map(|r| r.bytes)
                .map_err(core_error),
            Self::Wim(readers, members) => {
                let &(image, local) = members.get(index).ok_or(ArchiveError::SourceChanged)?;
                readers
                    .get_mut(image)
                    .ok_or(ArchiveError::SourceChanged)?
                    .extract(archive_core::EntryId(local), writer)
                    .map(|r| r.bytes)
                    .map_err(core_error)
            }
            Self::Ar => {
                let mut archive = ar::Archive::new(File::open(path)?);
                for n in 0..=index {
                    let mut entry = archive
                        .next_entry()
                        .ok_or(ArchiveError::SourceChanged)?
                        .map_err(|_| ArchiveError::CorruptArchive)?;
                    if n == index {
                        return copy_decoded(&mut entry, writer);
                    }
                }
                Err(ArchiveError::SourceChanged)
            }
            Self::Cpio => {
                let mut archive = kpea::Archive::new(File::open(path)?);
                archive.verify_crc(true);
                for n in 0..=index {
                    let mut entry = archive
                        .read_entry()
                        .map_err(|_| ArchiveError::CorruptArchive)?
                        .ok_or(ArchiveError::SourceChanged)?;
                    if n == index {
                        return copy_decoded(&mut entry.reader, writer);
                    }
                }
                Err(ArchiveError::SourceChanged)
            }
            Self::Lzma(mem_kb) => {
                let mut reader =
                    lzma_rust2::LzmaReader::new_mem_limit(File::open(path)?, *mem_kb, None)
                        .map_err(|_| ArchiveError::CorruptArchive)?;
                copy_decoded(&mut reader, writer)
            }
            Self::Lz4Frame => copy_decoded(
                &mut lz4_flex::frame::FrameDecoder::new(File::open(path)?),
                writer,
            ),
            Self::Lz4Block(workspace) => {
                let encoded_size = std::fs::metadata(path)?.len();
                let mut header = [0; 4];
                File::open(path)?
                    .read_exact(&mut header)
                    .map_err(|_| ArchiveError::CorruptArchive)?;
                let decoded = u64::from(u32::from_le_bytes(header));
                let required = encoded_size
                    .checked_add(decoded)
                    .and_then(|bytes| bytes.checked_add(65536))
                    .ok_or(ArchiveError::ResourceLimit)?;
                if decoded > writer.limit || required > *workspace {
                    return Err(ArchiveError::ResourceLimit);
                }
                let encoded = std::fs::read(path)?;
                let bytes = lz4_flex::block::decompress_size_prepended(&encoded)
                    .map_err(|_| ArchiveError::CorruptArchive)?;
                copy_decoded(&mut bytes.as_slice(), writer)
            }
            Self::Brotli => copy_decoded(
                &mut brotli::Decompressor::new(File::open(path)?, 65536),
                writer,
            ),
        }
    }
}

fn source_revision(
    path: &Path,
    control: &OperationControl,
) -> Result<([u8; 32], u64, std::time::SystemTime)> {
    use sha2::Digest;
    let mut file = File::open(path)?;
    let before = file.metadata()?;
    let mut digest = sha2::Sha256::new();
    let mut block = [0; 65536];
    loop {
        control.checkpoint()?;
        let n = file.read(&mut block)?;
        if n == 0 {
            break;
        }
        digest.update(&block[..n]);
    }
    let after = file.metadata()?;
    if before.len() != after.len() || before.modified()? != after.modified()? {
        return Err(ArchiveError::SourceChanged);
    }
    Ok((digest.finalize().into(), before.len(), before.modified()?))
}
fn validate_entries(
    entries: &mut [ArchiveEntry],
    limits: &ResourceLimits,
    control: &OperationControl,
) -> Result<()> {
    if entries.len() as u64 > limits.max_entries {
        return Err(ArchiveError::ResourceLimit);
    }
    let retained = entries.iter().try_fold(0u64, |n, entry| {
        n.checked_add(512 + entry.display_path.len() as u64 * 8)
            .ok_or(ArchiveError::ResourceLimit)
    })?;
    if retained > limits.max_metadata_bytes {
        return Err(ArchiveError::ResourceLimit);
    }
    crate::path_policy::annotate(entries, control, retained, limits.max_metadata_bytes)
}

pub(crate) fn try_execute<W: Write + Send>(
    request: &WorkerRequest,
    control: &OperationControl,
    output: &mut W,
    replies: ReplyPort,
    metadata: &crate::worker_metadata::MetadataCredit,
    interaction: &crate::interaction::InteractionPort,
) -> Option<Result<(u64, u64)>> {
    let path = Path::new(&request.path);
    let mut source = match File::open(path) {
        Ok(source) => source,
        Err(error) => return Some(Err(error.into())),
    };
    let family = match detect(path, &mut source) {
        Ok(Some(family)) => family,
        Ok(None) => return None,
        Err(error) => return Some(Err(error)),
    };
    Some(run(
        family,
        request,
        control,
        output,
        replies,
        metadata,
        interaction,
    ))
}
fn run<W: Write + Send>(
    family: Family,
    request: &WorkerRequest,
    control: &OperationControl,
    output: &mut W,
    replies: ReplyPort,
    metadata: &crate::worker_metadata::MetadataCredit,
    interaction: &crate::interaction::InteractionPort,
) -> Result<(u64, u64)> {
    let framed = matches!(family, Family::Alz | Family::Egg);
    if request.version != 2
        || request.volume_paths.len() > 1 && !framed
        || request.password.is_some() && !framed
        || !matches!(request.operation.as_str(), "open" | "test" | "extract")
    {
        return Err(ArchiveError::UnsupportedOperation);
    }
    let lifecycle = control.source_delete_begin();
    let path = Path::new(&request.path);
    let sources = framed
        .then(|| crate::worker_codecs::volumes::SourceSet::open_formats(request, control))
        .transpose()?;
    let revision = if let Some(set) = sources.as_ref() {
        (
            set.digest(),
            set.length(),
            std::fs::metadata(path)?.modified()?,
        )
    } else {
        source_revision(path, control)?
    };
    let verify = || -> Result<()> {
        if let Some(set) = sources.as_ref() {
            set.verify(control)?;
            if std::fs::metadata(path)?.modified()? != revision.2 {
                return Err(ArchiveError::SourceChanged);
            }
        } else if source_revision(path, control)? != revision {
            return Err(ArchiveError::SourceChanged);
        }
        Ok(())
    };
    if request.operation == "extract" {
        PrivateOutput::new(output, control, replies.clone())
            .source_checkpoint("source_validated")?;
    }
    let limits = crate::worker_browser::decode_limits(request);
    let (mut reader, mut entries) =
        open_reader(family, request, control, revision.1, sources.as_ref())?;
    validate_entries(&mut entries, &limits, control)?;
    let entries: Arc<[ArchiveEntry]> = entries.into();
    control.record_work(|work| work.catalog_loads += 1);
    crate::worker_browser::bind(
        request,
        request.operation_id,
        family.label(),
        "",
        revision.0,
        revision.1,
        revision.2,
        output,
        control,
        replies.clone(),
    )?;
    let selected =
        crate::worker_browser::selected(request, &entries)?.map(|entry| entry.id().ordinal);
    let mut plan = crate::worker_job::codec_plan(request, entries.clone(), control)?;
    if request.operation == "open" {
        crate::worker_metadata::emit_entries(
            request,
            &entries,
            request.operation_id,
            family.label(),
            output,
            control,
            metadata,
        )?;
        verify()?;
        return Ok((entries.len() as u64, 0));
    }
    let chosen: HashSet<_> = plan.as_ref().map_or_else(
        || {
            entries
                .iter()
                .filter(|entry| {
                    entry.kind == EntryKind::File && selected.is_none_or(|n| n == entry.id.ordinal)
                })
                .map(|entry| entry.id.ordinal)
                .collect()
        },
        |p| p.files().iter().map(|f| f.id().ordinal).collect(),
    );
    if request.operation == "extract" {
        for entry in &*entries {
            if chosen.contains(&entry.id.ordinal)
                && entry.warnings.contains(&EntryWarning::UnsafePath)
            {
                return Err(ArchiveError::UnsafePath);
            }
            if chosen.contains(&entry.id.ordinal)
                && entry.warnings.contains(&EntryWarning::OutputCollision)
            {
                return Err(ArchiveError::OutputCollision);
            }
        }
    }
    let mut total = 0u64;
    let mut max = 1;
    let mut known = true;
    for entry in entries.iter().filter(|e| chosen.contains(&e.id.ordinal)) {
        if let Some(size) = entry.size {
            max = max.max(size);
            total = total.checked_add(size).ok_or(ArchiveError::ResourceLimit)?;
        } else {
            known = false;
        }
    }
    let required =
        (known && !chosen.is_empty()).then_some(crate::output_readmission::OutputBudget {
            entry_bytes: max,
            output_bytes: total.max(1),
        });
    if request.operation == "extract" {
        let mut native = PrivateOutput::new(output, control, replies.clone());
        native.preflight_output(request, revision.0, required)?;
        native.destination_preflight(request, required.map(|b| b.output_bytes))?;
    }
    if max > limits.max_entry_bytes || total > limits.max_output_bytes {
        return Err(ArchiveError::ResourceLimit);
    }
    let mut native = (request.operation == "extract")
        .then(|| {
            PrivateOutput::new(output, control, replies.clone())
                .ordinary_extraction(request.ordinary_extraction())
                .planned_unique(plan.as_mut())
                .map(|o| {
                    o.extraction_metadata(request.extract_options.is_some())
                        .conflicts(request.conflict_policy, interaction, entries.clone())
                })
        })
        .transpose()?;
    let mut files = 0;
    let mut bytes = 0u64;
    let mut source_verified_before_commit = false;
    let result = (|| {
        if let Some(out) = native.as_mut() {
            let directories: Vec<_> = plan.as_ref().map_or_else(
                || {
                    if selected.is_some() {
                        let relative = entries
                            .iter()
                            .find(|entry| chosen.contains(&entry.id.ordinal))
                            .map(|entry| entry.display_path.replace('\\', "/"))
                            .unwrap_or_default();
                        let parts: Vec<_> = relative.split('/').collect();
                        (1..parts.len()).map(|end| parts[..end].join("/")).collect()
                    } else {
                        entries
                            .iter()
                            .filter(|e| e.kind == EntryKind::Directory)
                            .map(|e| e.display_path.clone())
                            .collect()
                    }
                },
                |p| p.directories().to_vec(),
            );
            for dir in directories {
                control.checkpoint()?;
                out.ensure_directory(dir.replace('\\', "/").trim_end_matches('/'))?;
            }
        }
        for entry in entries.iter().filter(|e| chosen.contains(&e.id.ordinal)) {
            control.checkpoint()?;
            let size_limit = limits
                .max_entry_bytes
                .min(limits.max_output_bytes.saturating_sub(bytes));
            let relative = entry.display_path.replace('\\', "/");
            let token = if let Some(out) = native.as_mut() {
                out.begin_extracted_file(&relative, entry.size)?
            } else {
                None
            };
            let mut sink = MemberWriter {
                output: native.as_mut().map(|o| o as &mut dyn ArchiveOutput),
                token,
                control,
                limit: size_limit,
                written: 0,
                failure: None,
            };
            let decoded = reader.extract(entry.id.ordinal as usize, path, &mut sink);
            let decoded = match (sink.failure, decoded) {
                (Some(error), _) => Err(error),
                (None, value) => value,
            };
            let decoded = decoded.and_then(|count| {
                if count == sink.written && entry.size.is_none_or(|size| size == count) {
                    Ok(count)
                } else {
                    Err(ArchiveError::CorruptArchive)
                }
            });
            if let Err(error) = decoded {
                if let (Some(out), Some(token)) = (native.as_mut(), token) {
                    out.abort_file(token);
                }
                return Err(error);
            }
            let count = decoded?;
            if let (Some(out), Some(token)) = (native.as_mut(), token) {
                let publish = (|| {
                    if sources.is_some() && !source_verified_before_commit {
                        verify()?;
                        source_verified_before_commit = true;
                    }
                    out.set_file_metadata(token, entry.metadata)?;
                    out.source_checkpoint("source_precommit")?;
                    out.commit_file(token)
                })();
                if let Err(error) = publish {
                    out.abort_file(token);
                    return Err(error);
                }
                files += 1;
            } else if native.is_none() {
                files += 1;
            }
            bytes = bytes
                .checked_add(count)
                .ok_or(ArchiveError::ResourceLimit)?;
            if bytes > limits.max_output_bytes {
                return Err(ArchiveError::ResourceLimit);
            }
        }
        Ok(())
    })();
    let result = if native.is_some() && framed {
        result.and_then(|()| verify())
    } else {
        result
    };
    if let Some(out) = native.as_mut() {
        let specials = entries
            .iter()
            .filter(|e| matches!(e.kind, EntryKind::Link | EntryKind::Special))
            .count() as u64;
        out.extraction_summary(result.is_ok() && specials == 0, specials)?;
        let skipped = out.skipped_files();
        result?;
        control.source_delete_finish(
            &lifecycle,
            if skipped != 0 || specials != 0 {
                "members_skipped"
            } else if chosen.is_empty() {
                "empty_source_set"
            } else if files == entries.iter().filter(|e| e.kind == EntryKind::File).count() as u64 {
                "eligible"
            } else {
                "coverage_incomplete"
            },
            vec![request.operation_id],
        );
    } else {
        result?;
        verify()?;
    }
    Ok((files, bytes))
}

#[cfg(test)]
mod resource_tests {
    use super::*;

    #[test]
    fn size_prefixed_lz4_charges_encoded_and_decoded_workspace_before_allocation() {
        let root = tempfile::tempdir().expect("owned LZ4 source");
        let path = root.path().join("large.lz4");
        let mut bytes = (128u32 << 20).to_le_bytes().to_vec();
        bytes.push(0);
        std::fs::write(&path, bytes).expect("declared large block");
        let control = OperationControl::default();
        let mut writer = MemberWriter {
            output: None,
            token: None,
            control: &control,
            limit: 256 << 20,
            written: 0,
            failure: None,
        };
        assert_eq!(
            Reader::Lz4Block(64 << 20).extract(0, &path, &mut writer),
            Err(ArchiveError::ResourceLimit)
        );
        assert_eq!(writer.written, 0);
    }
}
