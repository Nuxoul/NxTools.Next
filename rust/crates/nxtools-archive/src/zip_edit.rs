//! ZIP record-preserving candidate generation. No final-path writes or native publication.
use crate::{
    ArchiveError, ArchiveSnapshot, EntryId, EntryKind, OperationControl, ResourceLimits, Result,
    formats::zip,
    path_policy,
    zip_container::{self, Record},
};
use std::{
    collections::{HashMap, HashSet},
    fs::File,
    io::{Read, Seek, SeekFrom, Write},
};

#[derive(Clone)]
pub enum ZipEdit {
    Delete { entries: Vec<EntryId> },
    Rename { entry: EntryId, name: String },
    NewDirectory { name: String },
    Comment { text: String },
}
fn word(bytes: &[u8], at: usize) -> u16 {
    u16::from_le_bytes(bytes[at..at + 2].try_into().expect("checked ZIP field"))
}
fn exact(file: &mut File, bytes: &mut [u8]) -> Result<()> {
    file.read_exact(bytes).map_err(|error| {
        if error.kind() == std::io::ErrorKind::UnexpectedEof {
            ArchiveError::CorruptArchive
        } else {
            ArchiveError::IoFailure
        }
    })
}
fn valid_name(name: &str, directory: bool) -> Result<()> {
    if name.len() > 65535 || name.contains('\\') || directory != name.ends_with('/') {
        return Err(ArchiveError::UnsafePath);
    }
    path_policy::validate_relative(if directory {
        name.trim_end_matches('/')
    } else {
        name
    })
}
fn blocked_container(snapshot: &ArchiveSnapshot) -> bool {
    let extension = snapshot
        .revision
        .path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    matches!(
        extension.as_str(),
        "jar"
            | "apk"
            | "aab"
            | "war"
            | "ear"
            | "ipa"
            | "xpi"
            | "crx"
            | "nupkg"
            | "vsix"
            | "msix"
            | "appx"
    ) || snapshot.entries.iter().any(|entry| {
        let name = entry.display_path.to_ascii_uppercase();
        name == "ANDROIDMANIFEST.XML"
            || name.starts_with("META-INF/")
                && (name.ends_with(".SF")
                    || name.ends_with(".RSA")
                    || name.ends_with(".DSA")
                    || name.ends_with(".EC")
                    || name == "META-INF/MANIFEST.MF")
    })
}
/// Per-operation allowlist. These known fields contain no absolute offsets/alignment outside ZIP64.
/// Unknown records fail before copying kept entries; deleting an unsupported entry is permitted.
fn extras(bytes: &[u8], renamed: Option<&str>) -> Result<Vec<u8>> {
    let mut result = Vec::new();
    let mut at = 0;
    let mut seen = HashSet::new();
    while at < bytes.len() {
        if at + 4 > bytes.len() {
            return Err(ArchiveError::CorruptArchive);
        }
        let tag = word(bytes, at);
        let length = usize::from(word(bytes, at + 2));
        let end = at
            .checked_add(4 + length)
            .filter(|end| *end <= bytes.len())
            .ok_or(ArchiveError::CorruptArchive)?;
        if !seen.insert(tag) {
            return Err(ArchiveError::CorruptArchive);
        }
        match tag {
            0x0001 | 0x000a | 0x5455 | 0x5855 | 0x7855 | 0x7875 | 0x7075 | 0x6375 | 0x9901 => {}
            _ => return Err(ArchiveError::UnsupportedOperation),
        }
        if !(tag == 0x7075 && renamed.is_some()) {
            result.extend_from_slice(&bytes[at..end]);
        }
        at = end;
    }
    if let Some(name) = renamed {
        let mut data = vec![1];
        data.extend_from_slice(&crc32fast::hash(name.as_bytes()).to_le_bytes());
        data.extend_from_slice(name.as_bytes());
        let length = u16::try_from(data.len()).map_err(|_| ArchiveError::ResourceLimit)?;
        result.extend_from_slice(&0x7075u16.to_le_bytes());
        result.extend_from_slice(&length.to_le_bytes());
        result.extend(data);
    }
    if result.len() > 65535 {
        return Err(ArchiveError::ResourceLimit);
    }
    Ok(result)
}
struct Intent {
    deleted: HashSet<u64>,
    renamed: HashMap<u64, String>,
    directories: Vec<String>,
    comment: Option<Vec<u8>>,
}
fn intent(snapshot: &ArchiveSnapshot, edits: &[ZipEdit]) -> Result<Intent> {
    if edits.len() > 4096 {
        return Err(ArchiveError::ResourceLimit);
    }
    let mut result = Intent {
        deleted: HashSet::new(),
        renamed: HashMap::new(),
        directories: Vec::new(),
        comment: None,
    };
    let find = |id: &EntryId| {
        snapshot
            .entries
            .get(id.ordinal as usize)
            .filter(|entry| entry.id == *id)
            .ok_or(ArchiveError::SourceChanged)
    };
    for edit in edits {
        match edit {
            ZipEdit::Delete { entries } => {
                if entries.is_empty() || entries.len() > 4096 {
                    return Err(ArchiveError::ResourceLimit);
                }
                for id in entries {
                    let parent = find(id)?;
                    for entry in snapshot.entries.iter() {
                        if entry.id == *id
                            || parent.kind == EntryKind::Directory
                                && entry.display_path.starts_with(&parent.display_path)
                        {
                            result.deleted.insert(entry.id.ordinal);
                        }
                    }
                }
            }
            ZipEdit::Rename { entry: id, name } => {
                let selected = find(id)?;
                let directory = selected.kind == EntryKind::Directory;
                valid_name(name, directory)?;
                for entry in snapshot.entries.iter() {
                    if entry.id == *id
                        || directory && entry.display_path.starts_with(&selected.display_path)
                    {
                        let target = if entry.id == *id {
                            name.clone()
                        } else {
                            format!(
                                "{name}{}",
                                &entry.display_path[selected.display_path.len()..]
                            )
                        };
                        valid_name(&target, entry.kind == EntryKind::Directory)?;
                        if result.renamed.insert(entry.id.ordinal, target).is_some() {
                            return Err(ArchiveError::OutputCollision);
                        }
                    }
                }
            }
            ZipEdit::NewDirectory { name } => {
                valid_name(name, true)?;
                result.directories.push(name.clone());
            }
            ZipEdit::Comment { text } => {
                if text.len() > 65535
                    || text.contains('\0')
                    || result.comment.replace(text.as_bytes().to_vec()).is_some()
                {
                    return Err(ArchiveError::InvalidMetadata);
                }
            }
        }
    }
    Ok(result)
}
struct Local {
    header: Vec<u8>,
    data: u64,
    end: u64,
}
fn structure(
    file: &mut File,
    entries: &[crate::ArchiveEntry],
    directory: &zip_container::Directory,
    limits: &ResourceLimits,
    control: &OperationControl,
) -> Result<Vec<Local>> {
    if entries.len() != directory.records.len() {
        return Err(ArchiveError::SourceChanged);
    }
    zip::preflight_created(file, entries, limits, control)?;
    let mut locals = Vec::new();
    let mut charged = 0u64;
    for (entry, row) in entries.iter().zip(&directory.records) {
        control.checkpoint()?;
        if entry
            .zip
            .as_ref()
            .is_none_or(|location| location.header_offset != row.local)
        {
            return Err(ArchiveError::SourceChanged);
        }
        let data = zip::validate_local(file, entry)?;
        let end = zip::validate_descriptor(file, entry, data)?;
        let length = usize::try_from(data - row.local).map_err(|_| ArchiveError::ResourceLimit)?;
        charged = charged
            .checked_add(length as u64)
            .and_then(|v| v.checked_add(row.central.len() as u64 * 4 + 512))
            .ok_or(ArchiveError::ResourceLimit)?;
        if charged > limits.max_metadata_bytes {
            return Err(ArchiveError::ResourceLimit);
        }
        let mut header = vec![0; length];
        file.seek(SeekFrom::Start(row.local))?;
        exact(file, &mut header)?;
        locals.push(Local { header, data, end });
    }
    let mut ranges: Vec<_> = directory
        .records
        .iter()
        .zip(&locals)
        .map(|(row, local)| (row.local, local.end))
        .collect();
    ranges.sort_unstable();
    let mut expected = 0;
    for (start, end) in ranges {
        if start != expected {
            return Err(ArchiveError::UnsupportedOperation);
        }
        expected = end;
    }
    if expected != directory.offset {
        return Err(ArchiveError::UnsupportedOperation);
    }
    Ok(locals)
}
fn renamed_record(row: &Record, local: &Local, name: Option<&str>) -> Result<(Vec<u8>, Vec<u8>)> {
    let old_local_name = usize::from(word(&local.header, 26));
    let local_extra = extras(&local.header[30 + old_local_name..], name)?;
    let central_name = usize::from(word(&row.central, 28));
    let central_extra = usize::from(word(&row.central, 30));
    let central_comment = &row.central[46 + central_name + central_extra..];
    let new_extra = extras(
        &row.central[46 + central_name..46 + central_name + central_extra],
        name,
    )?;
    if let Some(name) = name {
        // Turning on UTF8 changes interpretation of member comments too; never silently recode them.
        if word(&row.central, 8) & 0x800 == 0 && !central_comment.is_ascii() {
            return Err(ArchiveError::UnsupportedOperation);
        }
        let mut header = local.header[..30].to_vec();
        let flags = word(&header, 6) | 0x800;
        header[6..8].copy_from_slice(&flags.to_le_bytes());
        header[26..28].copy_from_slice(&(name.len() as u16).to_le_bytes());
        header[28..30].copy_from_slice(&(local_extra.len() as u16).to_le_bytes());
        header.extend_from_slice(name.as_bytes());
        header.extend(local_extra);
        let mut central = row.central[..46].to_vec();
        let flags = word(&central, 8) | 0x800;
        central[8..10].copy_from_slice(&flags.to_le_bytes());
        central[28..30].copy_from_slice(&(name.len() as u16).to_le_bytes());
        central[30..32].copy_from_slice(&(new_extra.len() as u16).to_le_bytes());
        central.extend_from_slice(name.as_bytes());
        central.extend(new_extra);
        central.extend_from_slice(central_comment);
        Ok((header, central))
    } else {
        Ok((local.header.clone(), row.central.clone()))
    }
}
fn copy(
    input: &mut File,
    out: &mut File,
    start: u64,
    mut remaining: u64,
    limits: &ResourceLimits,
    control: &OperationControl,
) -> Result<()> {
    input.seek(SeekFrom::Start(start))?;
    let mut bytes = [0; 65536];
    while remaining > 0 {
        control.checkpoint()?;
        let count = remaining.min(bytes.len() as u64) as usize;
        exact(input, &mut bytes[..count])?;
        out.write_all(&bytes[..count])?;
        remaining -= count as u64;
        if out.stream_position()? > limits.max_output_bytes {
            return Err(ArchiveError::ResourceLimit);
        }
    }
    Ok(())
}
fn write_records(
    input: &mut File,
    out: &mut File,
    directory: zip_container::Directory,
    locals: Vec<Local>,
    selected: &Intent,
    records: &mut Vec<Record>,
    context: (&ResourceLimits, &OperationControl),
) -> Result<()> {
    let (limits, control) = context;
    for (index, (mut row, local)) in directory.records.into_iter().zip(locals).enumerate() {
        if selected.deleted.contains(&(index as u64)) {
            continue;
        }
        control.checkpoint()?;
        let (header, central) = renamed_record(
            &row,
            &local,
            selected.renamed.get(&(index as u64)).map(String::as_str),
        )?;
        row.local = out.stream_position()?;
        row.central = central;
        out.write_all(&header)?;
        copy(
            input,
            out,
            local.data,
            local.end - local.data,
            limits,
            control,
        )?;
        records.push(row);
    }
    Ok(())
}
fn empty_directory(name: &str, offset: u64) -> Record {
    let mut central = vec![0u8; 46];
    central[..4].copy_from_slice(b"PK\x01\x02");
    central[4..6].copy_from_slice(&20u16.to_le_bytes());
    central[6..8].copy_from_slice(&20u16.to_le_bytes());
    central[8..10].copy_from_slice(&0x800u16.to_le_bytes());
    central[14..16].copy_from_slice(&0x21u16.to_le_bytes());
    central[28..30].copy_from_slice(&(name.len() as u16).to_le_bytes());
    central[38..42].copy_from_slice(&16u32.to_le_bytes());
    central.extend_from_slice(name.as_bytes());
    Record {
        central,
        local: offset,
        packed: 0,
        size: 0,
        disk: 0,
    }
}
fn directory_header(name: &str) -> Vec<u8> {
    let mut local = vec![0u8; 30];
    local[..4].copy_from_slice(b"PK\x03\x04");
    local[4..6].copy_from_slice(&20u16.to_le_bytes());
    local[6..8].copy_from_slice(&0x800u16.to_le_bytes());
    local[12..14].copy_from_slice(&0x21u16.to_le_bytes());
    local[26..28].copy_from_slice(&(name.len() as u16).to_le_bytes());
    local.extend_from_slice(name.as_bytes());
    local
}
pub(crate) fn verify(
    file: &mut File,
    limits: &ResourceLimits,
    control: &OperationControl,
    password: Option<&str>,
) -> Result<()> {
    file.rewind()?;
    let (entries, _, safe) = zip::list(file, crate::NameEncoding::Auto, limits, control)?;
    if !safe
        || entries.iter().any(|entry| {
            entry
                .warnings
                .iter()
                .any(|warning| *warning != crate::EntryWarning::UncertainEncoding)
        })
    {
        return Err(ArchiveError::OutputCollision);
    }
    zip::preflight_created(file, &entries, limits, control)?;
    let mut total = 0;
    for entry in &entries {
        zip::stream_entry(file, entry, limits, control, password, &mut total, |_| {
            Ok(())
        })?;
    }
    Ok(())
}
pub(crate) fn candidate(
    snapshot: &ArchiveSnapshot,
    edits: &[ZipEdit],
    additions: Option<&mut File>,
    limits: &ResourceLimits,
    control: &OperationControl,
    password: Option<&str>,
) -> Result<File> {
    control.checkpoint()?;
    if snapshot.format != crate::ArchiveFormat::Zip
        || !snapshot.revision.volumes.is_empty()
        || blocked_container(snapshot)
    {
        return Err(ArchiveError::UnsupportedOperation);
    }
    let mut source = File::open(&snapshot.revision.path)?;
    crate::engine::ensure_revision(&mut source, &snapshot.revision, control)?;
    if !snapshot.backend_safe {
        return Err(ArchiveError::OutputCollision);
    }
    if edits.is_empty() && additions.is_none() {
        return Err(ArchiveError::UnsupportedOperation);
    }
    let selected = intent(snapshot, edits)?;
    verify(&mut source, limits, control, password)?;
    let directory = zip_container::single_directory(&mut source, limits, control)?;
    let comment = selected
        .comment
        .clone()
        .unwrap_or_else(|| directory.comment.clone());
    let locals = structure(&mut source, &snapshot.entries, &directory, limits, control)?;
    // Validate ALL kept metadata before creating the candidate, including distinct local extras.
    for (index, (row, local)) in directory.records.iter().zip(&locals).enumerate() {
        if !selected.deleted.contains(&(index as u64)) {
            renamed_record(
                row,
                local,
                selected.renamed.get(&(index as u64)).map(String::as_str),
            )?;
        }
    }
    let mut output = tempfile::tempfile()?;
    let mut records = Vec::new();
    write_records(
        &mut source,
        &mut output,
        directory,
        locals,
        &selected,
        &mut records,
        (limits, control),
    )?;
    for name in &selected.directories {
        control.checkpoint()?;
        records.push(empty_directory(name, output.stream_position()?));
        output.write_all(&directory_header(name))?;
    }
    if let Some(additions) = additions {
        verify(additions, limits, control, None)?;
        let directory = zip_container::single_directory(additions, limits, control)?;
        let (entries, _, _) = zip::list(additions, crate::NameEncoding::Auto, limits, control)?;
        let locals = structure(additions, &entries, &directory, limits, control)?;
        let all = Intent {
            deleted: HashSet::new(),
            renamed: HashMap::new(),
            directories: Vec::new(),
            comment: None,
        };
        write_records(
            additions,
            &mut output,
            directory,
            locals,
            &all,
            &mut records,
            (limits, control),
        )?;
    }
    if records.len() as u64 > limits.max_entries {
        return Err(ArchiveError::ResourceLimit);
    }
    zip_container::finish_single(&mut output, &records, comment, control)?;
    if output.stream_position()? > limits.max_output_bytes {
        return Err(ArchiveError::ResourceLimit);
    }
    output.flush()?;
    verify(&mut output, limits, control, password)?;
    crate::engine::ensure_revision(&mut source, &snapshot.revision, control)?;
    control.checkpoint()?;
    output.rewind()?;
    Ok(output)
}
