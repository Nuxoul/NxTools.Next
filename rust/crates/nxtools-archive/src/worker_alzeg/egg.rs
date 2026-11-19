//! EGG record framing; only payloads may span validated physical volume boundaries.
use super::{Method, Payload, crypto, decode_payload, exact, input_error, solid, volumes};
use crate::{
    ArchiveEntry, ArchiveError, EntryId, EntryKind, NameProvenance, OperationControl,
    ResourceLimits, Result,
    metadata::{ModifiedTime, OutputMetadata, WindowsAttributes},
    worker::WorkerRequest,
};
use hmac::Mac;
use std::{
    collections::HashSet,
    fs::File,
    io::{Read, Seek, SeekFrom, Write},
};

const END: u32 = 0x08e28222;
const FILE: u32 = 0x0a8590e3;
const BLOCK: u32 = 0x02b50c13;
const NAME: u32 = 0x0a8591ac;
const COMMENT: u32 = 0x04c63672;
const WINDOWS: u32 = 0x2c86950b;
const POSIX: u32 = 0x1ee922e5;
const ENCRYPT: u32 = 0x08d1470f;
const DUMMY: u32 = 0x07463307;

struct Block {
    offset: u64,
    payload: Payload,
}
struct Member {
    blocks: Vec<Block>,
    encryption: Option<crypto::Header>,
    expanded: u64,
    start: u64,
}
pub(crate) struct Egg {
    input: File,
    members: Vec<Member>,
    limits: ResourceLimits,
    control: OperationControl,
    password: Option<zeroize::Zeroizing<String>>,
    solid: Option<Box<solid::Solid>>,
}
fn tag(input: &mut File) -> Result<u32> {
    Ok(u32::from_le_bytes(exact(input)?))
}
fn charge(retained: &mut u64, amount: u64, limits: &ResourceLimits) -> Result<()> {
    *retained = retained
        .checked_add(amount)
        .ok_or(ArchiveError::ResourceLimit)?;
    if *retained > limits.max_metadata_bytes {
        return Err(ArchiveError::ResourceLimit);
    }
    Ok(())
}
fn extra(
    input: &mut File,
    length: u64,
    retained: &mut u64,
    limits: &ResourceLimits,
    encryption: bool,
) -> Result<(u8, u64, u64)> {
    let flags = exact::<1>(input)?[0];
    let declared = if flags & 1 != 0 {
        u64::from(u32::from_le_bytes(exact(input)?))
    } else {
        u64::from(u16::from_le_bytes(exact(input)?))
    };
    let start = input.stream_position()?;
    let size = if encryption {
        if flags & 4 != 0 {
            return Err(ArchiveError::UnsupportedEncryption);
        }
        if declared == 0 {
            return Err(ArchiveError::CorruptArchive);
        }
        let size = crypto::Header::length(exact::<1>(input)?[0])?;
        input.seek(SeekFrom::Start(start))?;
        if size != 17 && declared == size - 10 {
            let end = start
                .checked_add(declared)
                .filter(|end| *end <= length)
                .ok_or(ArchiveError::CorruptArchive)?;
            input.seek(SeekFrom::Start(end))?;
            if tag(input)? != END {
                return Err(ArchiveError::CorruptArchive);
            }
            return Err(ArchiveError::UnsupportedEncryption);
        }
        if declared != size && !(flags == 0 && declared == size + 7) {
            return Err(ArchiveError::CorruptArchive);
        }
        size
    } else {
        declared
    };
    let end = start
        .checked_add(size)
        .filter(|end| *end <= length)
        .ok_or(ArchiveError::CorruptArchive)?;
    if encryption {
        input.seek(SeekFrom::Start(end))?;
        if tag(input)? != END {
            return Err(ArchiveError::CorruptArchive);
        }
        input.seek(SeekFrom::Start(start))?;
    }
    charge(retained, size.saturating_mul(8).saturating_add(32), limits)?;
    Ok((flags, size, end))
}
fn merge_kind(current: EntryKind, detected: EntryKind) -> EntryKind {
    use EntryKind::{Directory, File, Link, Special};
    match (current, detected) {
        (Special, _) | (_, Special) => Special,
        (Link, _) | (_, Link) => Link,
        (Directory, _) | (_, Directory) => Directory,
        (File, File) => File,
    }
}
impl Egg {
    pub(crate) fn open(
        request: &WorkerRequest,
        control: &OperationControl,
        sources: &crate::worker_codecs::volumes::SourceSet,
    ) -> Result<(Self, Vec<ArchiveEntry>)> {
        let limits = crate::worker_browser::decode_limits(request);
        let volumes::EggInput {
            mut file,
            boundaries,
            header_end,
            mut retained,
            solid,
        } = volumes::egg(sources, &limits, control)?;
        let length = file.metadata()?.len();
        file.seek(SeekFrom::Start(header_end))?;
        let mut input = file;
        let mut entries = Vec::new();
        let mut members = Vec::new();
        let mut total_expanded = 0u64;
        let mut ids = HashSet::new();
        loop {
            control.checkpoint()?;
            let header_start = input.stream_position()?;
            match tag(&mut input)? {
                END => {
                    volumes::header_range(&boundaries, header_start, input.stream_position()?)?;
                    if input.stream_position()? != length {
                        return Err(ArchiveError::CorruptArchive);
                    }
                    break;
                }
                COMMENT | DUMMY => {
                    let (flags, _, end) = extra(&mut input, length, &mut retained, &limits, false)?;
                    if flags & 4 != 0 {
                        return Err(ArchiveError::UnsupportedEncryption);
                    }
                    input.seek(SeekFrom::Start(end))?;
                    volumes::header_range(&boundaries, header_start, end)?;
                    continue;
                }
                FILE => {}
                _ => return Err(ArchiveError::CorruptArchive),
            }
            if entries.len() as u64 >= limits.max_entries {
                return Err(ArchiveError::ResourceLimit);
            }
            charge(&mut retained, 512, &limits)?;
            let id = tag(&mut input)?;
            if !ids.insert(id) {
                return Err(ArchiveError::CorruptArchive);
            }
            let expanded = u64::from_le_bytes(exact(&mut input)?);
            let start = total_expanded;
            total_expanded = total_expanded
                .checked_add(expanded)
                .ok_or(ArchiveError::CorruptArchive)?;
            let mut raw_name = None;
            let mut metadata = OutputMetadata::default();
            let mut kind = EntryKind::File;
            let mut encryption = None;
            loop {
                control.checkpoint()?;
                let field = tag(&mut input)?;
                if field == END {
                    break;
                }
                let (flags, size, end) =
                    extra(&mut input, length, &mut retained, &limits, field == ENCRYPT)?;
                match field {
                    NAME => {
                        if flags & 4 != 0 {
                            return Err(ArchiveError::UnsupportedEncryption);
                        }
                        if flags & 0x18 != 0 {
                            return Err(ArchiveError::UnsupportedMethod);
                        }
                        if raw_name.is_some() {
                            return Err(ArchiveError::CorruptArchive);
                        }
                        let mut bytes = vec![
                            0;
                            usize::try_from(size)
                                .map_err(|_| ArchiveError::ResourceLimit)?
                        ];
                        input.read_exact(&mut bytes).map_err(input_error)?;
                        raw_name = Some(bytes);
                    }
                    WINDOWS => {
                        if size != 9 {
                            return Err(ArchiveError::CorruptArchive);
                        }
                        let ticks = u64::from_le_bytes(exact(&mut input)?);
                        let attributes = exact::<1>(&mut input)?[0];
                        kind = merge_kind(
                            kind,
                            if attributes & 8 != 0 {
                                EntryKind::Link
                            } else if attributes & 0x80 != 0 {
                                EntryKind::Directory
                            } else {
                                EntryKind::File
                            },
                        );
                        metadata.modified = Some(ModifiedTime::FiletimeUtc { ticks });
                        metadata.attributes = Some(WindowsAttributes {
                            read_only: attributes & 1 != 0,
                            hidden: attributes & 2 != 0,
                        });
                    }
                    POSIX => {
                        if size != 20 {
                            return Err(ArchiveError::CorruptArchive);
                        }
                        let data = exact::<20>(&mut input)?;
                        let mode = u32::from_le_bytes(data[..4].try_into().expect("mode"));
                        kind = merge_kind(
                            kind,
                            match mode & 0xf000 {
                                0 | 0x8000 => EntryKind::File,
                                0x4000 => EntryKind::Directory,
                                0xa000 => EntryKind::Link,
                                _ => EntryKind::Special,
                            },
                        );
                        metadata.modified = Some(ModifiedTime::UnixUtc {
                            seconds: i64::from_le_bytes(data[12..].try_into().expect("seconds")),
                            nanoseconds: 0,
                            precision_ns: 1_000_000_000,
                        });
                    }
                    ENCRYPT => {
                        if encryption.is_some() {
                            return Err(ArchiveError::CorruptArchive);
                        }
                        encryption = Some(crypto::Header::read(&mut input)?);
                    }
                    COMMENT => {
                        if flags & 4 != 0 {
                            return Err(ArchiveError::UnsupportedEncryption);
                        }
                        input.seek(SeekFrom::Start(end))?;
                    }
                    _ => return Err(ArchiveError::UnsupportedFormat),
                }
                if input.stream_position()? != end {
                    return Err(ArchiveError::CorruptArchive);
                }
            }
            volumes::header_range(&boundaries, header_start, input.stream_position()?)?;
            let encrypted = encryption.is_some();
            if solid && encrypted {
                return Err(ArchiveError::UnsupportedEncryption);
            }
            let raw_name = raw_name.ok_or(ArchiveError::CorruptArchive)?;
            let display_path = std::str::from_utf8(&raw_name)
                .map_err(|_| ArchiveError::InvalidMetadata)?
                .to_owned();
            let mut blocks = Vec::new();
            let mut packed = 0u64;
            let mut block_expanded = 0u64;
            let mut method = 0;
            loop {
                control.checkpoint()?;
                let position = input.stream_position()?;
                if tag(&mut input)? != BLOCK {
                    input.seek(SeekFrom::Start(position))?;
                    break;
                }
                charge(&mut retained, 128, &limits)?;
                let header = exact::<14>(&mut input)?;
                if tag(&mut input)? != END {
                    return Err(ArchiveError::CorruptArchive);
                }
                volumes::header_range(&boundaries, position, input.stream_position()?)?;
                let expanded = u64::from(u32::from_le_bytes(
                    header[2..6].try_into().expect("expanded"),
                ));
                let size = u64::from(u32::from_le_bytes(
                    header[6..10].try_into().expect("packed"),
                ));
                let crc = u32::from_le_bytes(header[10..14].try_into().expect("CRC"));
                if blocks.is_empty() {
                    method = u16::from(header[0]);
                }
                let codec = match header[0] {
                    0 => Method::Stored,
                    1 => Method::Deflate,
                    2 => Method::Bzip2,
                    3 => Method::Azo,
                    4 => Method::Lzma,
                    _ => Method::Unsupported,
                };
                let offset = input.stream_position()?;
                let end = offset
                    .checked_add(size)
                    .filter(|end| *end <= length)
                    .ok_or(ArchiveError::CorruptArchive)?;
                packed = packed
                    .checked_add(size)
                    .ok_or(ArchiveError::CorruptArchive)?;
                block_expanded = block_expanded
                    .checked_add(expanded)
                    .ok_or(ArchiveError::CorruptArchive)?;
                blocks.push(Block {
                    offset,
                    payload: Payload {
                        packed: size,
                        expanded,
                        crc,
                        method: codec,
                    },
                });
                input.seek(SeekFrom::Start(end))?;
            }
            if !solid && block_expanded != expanded
                || kind == EntryKind::Directory
                    && (!blocks.is_empty() || encrypted || expanded != 0)
            {
                return Err(ArchiveError::CorruptArchive);
            }
            let crc32 = (!solid && blocks.len() == 1).then(|| blocks[0].payload.crc);
            entries.push(ArchiveEntry {
                id: EntryId {
                    session: request.operation_id,
                    ordinal: entries.len() as u64,
                },
                raw_name,
                display_path,
                provenance: NameProvenance::FormatUnicode,
                kind,
                size: Some(expanded),
                compressed_size: (!solid).then_some(packed),
                method: if solid { 1 } else { method },
                encrypted,
                crc32,
                dos_modified: None,
                metadata,
                warnings: Vec::new(),
                zip: None,
            });
            members.push(Member {
                blocks,
                encryption,
                expanded,
                start,
            });
        }
        let solid = if solid {
            let mut spans = Vec::new();
            for member in &members {
                for block in &member.blocks {
                    if !matches!(block.payload.method, Method::Deflate) {
                        return Err(ArchiveError::UnsupportedMethod);
                    }
                    spans.push(solid::Span {
                        offset: block.offset,
                        packed: block.payload.packed,
                        expanded: block.payload.expanded,
                        crc: block.payload.crc,
                    });
                }
            }
            Some(Box::new(solid::Solid::new(
                input.try_clone()?,
                spans,
                total_expanded,
                control.clone(),
            )?))
        } else {
            None
        };
        Ok((
            Self {
                input,
                members,
                limits,
                control: control.clone(),
                password: request.password.clone().map(zeroize::Zeroizing::new),
                solid,
            },
            entries,
        ))
    }
    pub(crate) fn extract(&mut self, index: usize, writer: &mut impl Write) -> Result<u64> {
        let member = self.members.get(index).ok_or(ArchiveError::SourceChanged)?;
        if let Some(solid) = &mut self.solid {
            return solid.extract(member.start, member.expanded, &self.limits, writer);
        }
        let mut crypto = if let Some(header) = &member.encryption {
            let password = self
                .password
                .as_deref()
                .ok_or(ArchiveError::PasswordRequired)?;
            if self.limits.max_working_bytes < 256 << 10 {
                return Err(ArchiveError::ResourceLimit);
            }
            let (state, authentication) = crypto::State::new(header, password, &self.control)?;
            if let Some(mut mac) = authentication {
                let mut buffer = [0; 65536];
                for block in &member.blocks {
                    self.input.seek(SeekFrom::Start(block.offset))?;
                    let mut remaining = block.payload.packed;
                    while remaining > 0 {
                        self.control.checkpoint()?;
                        let count = remaining.min(buffer.len() as u64) as usize;
                        self.input
                            .read_exact(&mut buffer[..count])
                            .map_err(input_error)?;
                        mac.update(&buffer[..count]);
                        remaining -= count as u64;
                    }
                }
                mac.verify_truncated_left(header.tag().expect("authenticated EGG method"))
                    .map_err(|_| ArchiveError::AuthenticationOrDataError)?;
            }
            Some(state)
        } else {
            None
        };
        for block in &member.blocks {
            self.control.checkpoint()?;
            self.input.seek(SeekFrom::Start(block.offset))?;
            let result = if let Some(state) = &mut crypto {
                let mut input =
                    state.reader((&mut self.input).take(block.payload.packed), &self.control);
                decode_payload(
                    &mut input,
                    &block.payload,
                    &self.limits,
                    &self.control,
                    writer,
                )
            } else {
                decode_payload(
                    &mut self.input,
                    &block.payload,
                    &self.limits,
                    &self.control,
                    writer,
                )
            };
            result.map_err(|error| {
                if crypto.is_some() && error == ArchiveError::CorruptArchive {
                    ArchiveError::AuthenticationOrDataError
                } else {
                    error
                }
            })?;
        }
        Ok(member.expanded)
    }
}
