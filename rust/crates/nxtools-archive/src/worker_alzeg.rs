//! ALZ/EGG container adaptation inside the original native-admitted codec worker.
mod azo;
mod crypto;
mod dlz;
mod egg;
mod solid;
mod volumes;
pub(crate) use egg::Egg;

use crate::{
    ArchiveEntry, ArchiveError, EntryId, EntryKind, NameEncoding, NameProvenance, OperationControl,
    Result,
    metadata::{ModifiedTime, OutputMetadata, WindowsAttributes},
    worker::WorkerRequest,
};
use std::{
    fs::File,
    io::{self, Read, Seek, SeekFrom, Write},
};

struct Member {
    offset: u64,
    packed: u64,
    expanded: u64,
    crc: u32,
    method: u16,
    encrypted: bool,
    verifier: u8,
}
pub(crate) struct Alz {
    input: File,
    members: Vec<Member>,
    control: OperationControl,
    limits: crate::ResourceLimits,
    password: Option<zeroize::Zeroizing<String>>,
}
fn exact<const N: usize>(input: &mut impl Read) -> Result<[u8; N]> {
    let mut bytes = [0; N];
    input.read_exact(&mut bytes).map_err(input_error)?;
    Ok(bytes)
}
fn input_error(error: io::Error) -> ArchiveError {
    if error.kind() == io::ErrorKind::UnexpectedEof {
        ArchiveError::CorruptArchive
    } else {
        error.into()
    }
}
fn sized(input: &mut File, width: usize) -> Result<u64> {
    let mut bytes = [0; 8];
    input.read_exact(&mut bytes[..width]).map_err(input_error)?;
    Ok(u64::from_le_bytes(bytes))
}
impl Alz {
    pub(crate) fn open(
        request: &WorkerRequest,
        control: &OperationControl,
        sources: &crate::worker_codecs::volumes::SourceSet,
    ) -> Result<(Self, Vec<ArchiveEntry>)> {
        let limits = crate::worker_browser::decode_limits(request);
        let mut input = volumes::alz(sources, &limits, control)?;
        let length = input.metadata()?.len();
        if length < 24 {
            return Err(ArchiveError::CorruptArchive);
        }
        let header = exact::<8>(&mut input)?;
        if &header[..4] != b"ALZ\x01" {
            return Err(ArchiveError::CorruptArchive);
        }
        if header[4..6] != [10, 0] {
            return Err(ArchiveError::UnsupportedFormat);
        }
        if header[6..] != [0, 0] {
            return Err(ArchiveError::MissingVolume);
        }
        let end = length - 16;
        input.seek(SeekFrom::Start(end))?;
        let tail = exact::<16>(&mut input)?;
        if &tail[..4] != b"CLZ\x01" || &tail[12..] != b"CLZ\x02" {
            return Err(ArchiveError::CorruptArchive);
        }
        let comment = u64::from(u32::from_le_bytes(
            tail[4..8].try_into().expect("four bytes"),
        ));
        let mut retained = 0u64;
        let mut entries = Vec::new();
        let mut members = Vec::new();
        input.seek(SeekFrom::Start(8))?;
        while input.stream_position()? < end {
            control.checkpoint()?;
            let signature = exact::<4>(&mut input)?;
            if &signature == b"ELZ\x01" {
                if comment < 4 || input.stream_position()?.checked_add(comment - 4) != Some(end) {
                    return Err(ArchiveError::CorruptArchive);
                }
                input.seek(SeekFrom::Start(end))?;
                break;
            }
            if &signature != b"BLZ\x01" {
                return Err(ArchiveError::CorruptArchive);
            }
            let fixed = exact::<9>(&mut input)?;
            let name_length = usize::from(u16::from_le_bytes([fixed[0], fixed[1]]));
            retained = retained
                .checked_add(512 + name_length as u64 * 8)
                .ok_or(ArchiveError::ResourceLimit)?;
            if entries.len() as u64 >= limits.max_entries || retained > limits.max_metadata_bytes {
                return Err(ArchiveError::ResourceLimit);
            }
            let attributes = fixed[2];
            let time = u32::from_le_bytes(fixed[3..7].try_into().expect("four bytes"));
            let flags = fixed[7];
            let width = usize::from(flags >> 4);
            if !matches!(width, 0 | 1 | 2 | 4 | 8) {
                return Err(ArchiveError::CorruptArchive);
            }
            if flags & 6 != 0 {
                return Err(ArchiveError::UnsupportedMethod);
            }
            let (method, crc, packed, expanded) = if width == 0 {
                (0, 0, 0, 0)
            } else {
                let variable = exact::<6>(&mut input)?;
                (
                    u16::from(variable[0]),
                    u32::from_le_bytes(variable[2..6].try_into().expect("four bytes")),
                    sized(&mut input, width)?,
                    sized(&mut input, width)?,
                )
            };
            let mut raw_name = vec![0; name_length];
            input.read_exact(&mut raw_name).map_err(input_error)?;
            let encoding = request.encoding()?;
            let selected = if encoding == NameEncoding::Auto {
                if std::str::from_utf8(&raw_name).is_ok() {
                    NameEncoding::Utf8
                } else {
                    NameEncoding::Korean
                }
            } else {
                encoding
            };
            let (display_path, mut provenance) =
                crate::encoding::decode(&raw_name, false, &[], selected)?;
            if encoding == NameEncoding::Auto {
                provenance = if raw_name.is_ascii() {
                    NameProvenance::FormatUnicode
                } else {
                    NameProvenance::Heuristic(selected)
                };
            }
            let encrypted = flags & 1 != 0;
            let offset = input.stream_position()?;
            let following = offset
                .checked_add(if encrypted { 12 } else { 0 })
                .and_then(|value| value.checked_add(packed))
                .filter(|value| *value <= end)
                .ok_or(ArchiveError::CorruptArchive)?;
            let kind = if attributes & 0x10 != 0 {
                if packed != 0 || expanded != 0 || encrypted {
                    return Err(ArchiveError::CorruptArchive);
                }
                EntryKind::Directory
            } else {
                EntryKind::File
            };
            entries.push(ArchiveEntry {
                id: EntryId {
                    session: request.operation_id,
                    ordinal: entries.len() as u64,
                },
                raw_name,
                display_path,
                provenance,
                kind,
                size: Some(expanded),
                compressed_size: Some(packed),
                method,
                encrypted,
                crc32: (width != 0).then_some(crc),
                dos_modified: Some(((time >> 16) as u16, time as u16)),
                metadata: OutputMetadata {
                    modified: Some(ModifiedTime::DosLocal {
                        date: (time >> 16) as u16,
                        time: time as u16,
                    }),
                    attributes: Some(WindowsAttributes {
                        read_only: attributes & 1 != 0,
                        hidden: attributes & 2 != 0,
                    }),
                },
                warnings: Vec::new(),
                zip: None,
            });
            members.push(Member {
                offset,
                packed,
                expanded,
                crc,
                method,
                encrypted,
                verifier: if flags & 8 != 0 {
                    (time >> 8) as u8
                } else {
                    (crc >> 24) as u8
                },
            });
            input.seek(SeekFrom::Start(following))?;
        }
        if input.stream_position()? != end {
            return Err(ArchiveError::CorruptArchive);
        }
        Ok((
            Self {
                input,
                members,
                control: control.clone(),
                limits,
                password: request.password.clone().map(zeroize::Zeroizing::new),
            },
            entries,
        ))
    }
    pub(crate) fn extract(&mut self, index: usize, writer: &mut impl Write) -> Result<u64> {
        let member = self.members.get(index).ok_or(ArchiveError::SourceChanged)?;
        let method = match member.method {
            0 => Method::Stored,
            1 => Method::Dlz,
            2 => Method::Deflate,
            _ => return Err(ArchiveError::UnsupportedMethod),
        };
        let payload = Payload {
            packed: member.packed,
            expanded: member.expanded,
            crc: member.crc,
            method,
        };
        let result = if member.encrypted {
            let password = self
                .password
                .as_deref()
                .ok_or(ArchiveError::PasswordRequired)?;
            let (encoded, _, invalid) = encoding_rs::EUC_KR.encode(password);
            let encoded = zeroize::Zeroizing::new(encoded.into_owned());
            if invalid {
                return Err(ArchiveError::AuthenticationOrDataError);
            }
            let packed = member
                .packed
                .checked_add(12)
                .ok_or(ArchiveError::CorruptArchive)?;
            let mut input = crate::formats::zip_crypto::Classic::traditional(
                &mut self.input,
                member.offset,
                packed,
                &encoded,
                member.verifier,
                &self.control,
            )?;
            decode_payload(&mut input, &payload, &self.limits, &self.control, writer)
        } else {
            self.input.seek(SeekFrom::Start(member.offset))?;
            decode_payload(
                &mut self.input,
                &payload,
                &self.limits,
                &self.control,
                writer,
            )
        };
        result.map_err(|error| {
            if member.encrypted && error == ArchiveError::CorruptArchive {
                ArchiveError::AuthenticationOrDataError
            } else {
                error
            }
        })?;
        Ok(member.expanded)
    }
}

#[derive(Clone, Copy)]
enum Method {
    Stored,
    Deflate,
    Bzip2,
    Dlz,
    Azo,
    Lzma,
    Unsupported,
}

#[derive(Clone, Copy)]
struct Payload {
    packed: u64,
    expanded: u64,
    crc: u32,
    method: Method,
}
fn decode_payload(
    input: &mut impl Read,
    payload: &Payload,
    limits: &crate::ResourceLimits,
    control: &OperationControl,
    writer: &mut impl Write,
) -> Result<()> {
    let Payload {
        packed,
        expanded,
        crc,
        method,
    } = *payload;
    if limits.max_working_bytes < 256 << 10 {
        return Err(ArchiveError::ResourceLimit);
    }
    let input = input.take(packed);
    let consumed = match method {
        Method::Stored => {
            let mut input = input;
            copy_checked(&mut input, expanded, crc, control, writer)?;
            packed - input.limit()
        }
        Method::Deflate => {
            let mut decoder = crate::formats::zip::Deflated::new(input, control.clone(), false);
            copy_checked(&mut decoder, expanded, crc, control, writer)?;
            decoder.consumed()
        }
        Method::Bzip2 => {
            if limits.max_working_bytes < 5 << 20 {
                return Err(ArchiveError::ResourceLimit);
            }
            let mut decoder = crate::formats::zip_bzip2::Bzip2::new(input, control.clone());
            copy_checked(&mut decoder, expanded, crc, control, writer)?;
            decoder.consumed()
        }
        Method::Dlz => {
            let mut decoder = dlz::Dlz::new(input, limits, control.clone())?;
            copy_checked(&mut decoder, expanded, crc, control, writer)?;
            decoder.consumed()
        }
        Method::Azo => {
            let mut decoder = azo::Azo::new(input, expanded, limits, control.clone())?;
            copy_checked(&mut decoder, expanded, crc, control, writer)?;
            decoder.consumed()
        }
        Method::Lzma => {
            let mut decoder =
                crate::formats::zip_lzma::Lzma::egg(input, expanded, limits, control.clone())?;
            copy_checked(&mut decoder, expanded, crc, control, writer)?;
            decoder.consumed()
        }
        Method::Unsupported => return Err(ArchiveError::UnsupportedMethod),
    };
    if consumed != packed {
        return Err(ArchiveError::CorruptArchive);
    }
    Ok(())
}
fn copy_checked(
    input: &mut impl Read,
    expanded: u64,
    expected_crc: u32,
    control: &OperationControl,
    writer: &mut impl Write,
) -> Result<()> {
    let mut remaining = expanded;
    let mut crc = crc32fast::Hasher::new();
    let mut buffer = [0; 65536];
    loop {
        control.checkpoint()?;
        let count = input
            .read(&mut buffer)
            .map_err(crate::formats::zip::payload_error)?;
        if count == 0 {
            break;
        }
        remaining = remaining
            .checked_sub(count as u64)
            .ok_or(ArchiveError::CorruptArchive)?;
        crc.update(&buffer[..count]);
        writer.write_all(&buffer[..count])?;
    }
    if remaining != 0 || crc.finalize() != expected_crc {
        return Err(ArchiveError::CorruptArchive);
    }
    Ok(())
}
