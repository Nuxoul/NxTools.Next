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
pub(crate) fn blocked_container(snapshot: &ArchiveSnapshot) -> bool {
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
/// Read-only structural admission; no candidate, native output, payload decode or new authority.
/// Unknown structures remain unknown, not a guessed SFX/encryption/volume diagnosis.
pub(crate) fn eligibility(
    snapshot: &ArchiveSnapshot,
    limits: &ResourceLimits,
    control: &OperationControl,
) -> std::result::Result<(), crate::ArchiveModificationRefusal> {
    use crate::ArchiveModificationRefusal as Refusal;
    if snapshot.format != crate::ArchiveFormat::Zip {
        return Err(Refusal::BackendMissing);
    }
    if !snapshot.revision.volumes.is_empty() {
        return Err(Refusal::Volumes);
    }
    if blocked_container(snapshot) {
        return Err(Refusal::ApplicationContainer);
    }
    let mut file = File::open(&snapshot.revision.path).map_err(|_| Refusal::SourceUnavailable)?;
    let classify = classify_eligibility;
    crate::engine::ensure_revision(&mut file, &snapshot.revision, control).map_err(classify)?;
    if !snapshot.backend_safe {
        return Err(Refusal::UnsupportedStructure);
    }
    let directory =
        zip_container::single_directory(&mut file, limits, control).map_err(classify)?;
    eligibility_from_open(&mut file, snapshot, &directory, limits, control)?;
    crate::engine::ensure_revision(&mut file, &snapshot.revision, control).map_err(classify)
}

fn classify_eligibility(error: ArchiveError) -> crate::ArchiveModificationRefusal {
    use crate::ArchiveModificationRefusal as Refusal;
    match error {
        ArchiveError::SourceChanged => Refusal::SourceChanged,
        ArchiveError::CorruptArchive => Refusal::Corrupt,
        ArchiveError::UnsupportedEncryption => Refusal::UnsupportedEncryption,
        ArchiveError::UnsupportedOperation
        | ArchiveError::UnsupportedMethod
        | ArchiveError::OutputCollision => Refusal::UnsupportedStructure,
        _ => Refusal::UnverifiedMetadata,
    }
}

/// The caller encloses catalog and qualification in the same before/final revision checks.
pub(crate) fn eligibility_from_open(
    file: &mut File,
    snapshot: &ArchiveSnapshot,
    directory: &zip_container::Directory,
    limits: &ResourceLimits,
    control: &OperationControl,
) -> std::result::Result<(), crate::ArchiveModificationRefusal> {
    use crate::ArchiveModificationRefusal as Refusal;
    if blocked_container(snapshot) {
        return Err(Refusal::ApplicationContainer);
    }
    if !snapshot.backend_safe {
        return Err(Refusal::UnsupportedStructure);
    }
    let classify = classify_eligibility;
    let locals =
        structure(file, &snapshot.entries, directory, limits, control).map_err(classify)?;
    for (row, local) in directory.records.iter().zip(locals.records()) {
        // No normalization or speculative metadata support. Candidate planning still owns
        // the original per-kept-record rule (including deletion of unsupported records).
        renamed_record(row, local, None).map_err(|error| match error {
            ArchiveError::UnsupportedOperation => Refusal::UnverifiedMetadata,
            other => classify(other),
        })?;
    }
    Ok(())
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
fn intent(
    snapshot: &ArchiveSnapshot,
    edits: &[ZipEdit],
    limits: &ResourceLimits,
    control: &OperationControl,
) -> Result<Intent> {
    let mut work = crate::planning::SelectionWork::default();
    let result = intent_observed(snapshot, edits, limits, control, &mut work);
    control.record_work(|current| current.selection += work);
    result
}
fn intent_observed(
    snapshot: &ArchiveSnapshot,
    edits: &[ZipEdit],
    limits: &ResourceLimits,
    control: &OperationControl,
    work: &mut crate::planning::SelectionWork,
) -> Result<Intent> {
    if edits.len() > 4096 {
        return Err(ArchiveError::ResourceLimit);
    }
    let mut result = Intent {
        deleted: HashSet::new(),
        renamed: HashMap::new(),
        directories: Vec::new(),
        comment: None,
    };
    let original_bytes = retained_snapshot_bytes(snapshot)?;
    work.metadata_visits += snapshot.entries.len() as u64;
    let mut delete_positions = HashSet::new();
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
                    control.checkpoint()?;
                    work.ordinal_probes += 1;
                    find(id)?;
                    let count = delete_positions.len() as u64 + 1;
                    if original_bytes
                        .checked_add(count.saturating_mul(96))
                        .is_none_or(|bytes| bytes > limits.max_metadata_bytes)
                    {
                        return Err(ArchiveError::ResourceLimit);
                    }
                    delete_positions.insert(id.ordinal as usize);
                }
            }
            ZipEdit::Rename { entry: id, name } => {
                let selected = find(id)?;
                let directory = selected.kind == EntryKind::Directory;
                valid_name(name, directory)?;
                let source = selected.display_path.replace('\\', "/");
                let prefix = format!("{}/", source.trim_end_matches('/'));
                for entry in snapshot.entries.iter() {
                    control.checkpoint()?;
                    let mut target = if entry.id == *id {
                        name.clone()
                    } else if directory {
                        let path = if entry.display_path.contains('\\') {
                            std::borrow::Cow::Owned(entry.display_path.replace('\\', "/"))
                        } else {
                            std::borrow::Cow::Borrowed(entry.display_path.as_str())
                        };
                        let Some(suffix) = path.strip_prefix(&prefix) else {
                            continue;
                        };
                        format!("{name}{suffix}")
                    } else {
                        continue;
                    };
                    if entry.kind == EntryKind::Directory && !target.ends_with('/') {
                        target.push('/');
                    }
                    valid_name(&target, entry.kind == EntryKind::Directory)?;
                    if result.renamed.insert(entry.id.ordinal, target).is_some() {
                        return Err(ArchiveError::OutputCollision);
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
    if !delete_positions.is_empty() {
        let positions = delete_positions.into_iter().collect::<Vec<_>>();
        let credit = limits
            .max_metadata_bytes
            .checked_sub(original_bytes)
            .and_then(|bytes| bytes.checked_sub(intent_metadata(&result)))
            .ok_or(ArchiveError::ResourceLimit)?
            .min(limits.max_working_bytes);
        crate::planning::expand_selection(
            &snapshot.entries,
            crate::planning::CatalogSelection::Entries {
                positions: &positions,
                directories: &[],
            },
            credit,
            |_, entry| crate::planning::SelectionEntry {
                ordinal: entry.id.ordinal,
                path: &entry.display_path,
                directory: entry.kind == EntryKind::Directory,
            },
            |_, entry, _, reservation| {
                if reservation
                    .checked_add((result.deleted.len() as u64 + 1).saturating_mul(64))
                    .is_none_or(|bytes| bytes > credit)
                {
                    return Err(ArchiveError::ResourceLimit);
                }
                result.deleted.insert(entry.id.ordinal);
                Ok(result.deleted.capacity() as u64 * 32)
            },
            || control.checkpoint(),
            work,
        )?;
    }
    Ok(result)
}
type Local = zip::ValidatedRecord;
fn intent_metadata(intent: &Intent) -> u64 {
    intent.deleted.capacity() as u64 * 32
        + intent.renamed.capacity() as u64 * 96
        + intent
            .renamed
            .values()
            .map(|name| name.capacity() as u64)
            .sum::<u64>()
        + intent.directories.capacity() as u64 * 24
        + intent
            .directories
            .iter()
            .map(|name| name.capacity() as u64)
            .sum::<u64>()
        + intent
            .comment
            .as_ref()
            .map_or(0, |comment| comment.capacity() as u64)
}
fn reserve_metadata(limits: &ResourceLimits, retained: u64) -> Result<ResourceLimits> {
    Ok(ResourceLimits {
        max_metadata_bytes: limits
            .max_metadata_bytes
            .checked_sub(retained)
            .ok_or(ArchiveError::ResourceLimit)?,
        ..*limits
    })
}
fn records_metadata(records: &[Record]) -> Result<u64> {
    records.iter().try_fold(0u64, |bytes, row| {
        bytes
            .checked_add(row.central.capacity() as u64 * 4 + 512)
            .ok_or(ArchiveError::ResourceLimit)
    })
}
fn retained_snapshot_bytes(snapshot: &ArchiveSnapshot) -> Result<u64> {
    let fixed = (std::mem::size_of::<ArchiveSnapshot>()
        + snapshot.comment.capacity()
        + snapshot.revision.path.as_os_str().as_encoded_bytes().len() * 2) as u64;
    snapshot.entries.iter().try_fold(fixed, |bytes, entry| {
        bytes
            .checked_add(
                (std::mem::size_of::<crate::ArchiveEntry>()
                    + 64
                    + entry.raw_name.capacity()
                    + entry.display_path.capacity()
                    + entry.warnings.capacity() * std::mem::size_of::<crate::EntryWarning>())
                    as u64,
            )
            .ok_or(ArchiveError::ResourceLimit)
    })
}
fn structure<'a>(
    file: &'a mut File,
    entries: &'a [crate::ArchiveEntry],
    directory: &zip_container::Directory,
    limits: &ResourceLimits,
    control: &OperationControl,
) -> Result<zip::ValidatedZip<'a>> {
    if entries.len() != directory.records.len() {
        return Err(ArchiveError::SourceChanged);
    }
    let limits = reserve_metadata(
        limits,
        records_metadata(&directory.records)?
            .checked_add(directory.comment.capacity() as u64)
            .ok_or(ArchiveError::ResourceLimit)?,
    )?;
    let locals = zip::ValidatedZip::new(file, entries, &limits, control, true)?;
    for (entry, row) in entries.iter().zip(&directory.records) {
        control.checkpoint()?;
        if entry
            .zip
            .as_ref()
            .is_none_or(|location| location.header_offset != row.local)
        {
            return Err(ArchiveError::SourceChanged);
        }
    }
    if !locals.contiguous() {
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
    input: &mut zip::ValidatedZip<'_>,
    out: &mut File,
    directory: zip_container::Directory,
    selected: &Intent,
    records: &mut Vec<Record>,
    context: (&ResourceLimits, &OperationControl),
) -> Result<()> {
    let (limits, control) = context;
    for (index, mut row) in directory.records.into_iter().enumerate() {
        if selected.deleted.contains(&(index as u64)) {
            continue;
        }
        control.checkpoint()?;
        let local = input.record(index as u64)?;
        let (header, central) = renamed_record(
            &row,
            local,
            selected.renamed.get(&(index as u64)).map(String::as_str),
        )?;
        let (data, end) = (local.data, local.end);
        row.local = out.stream_position()?;
        row.central = central;
        out.write_all(&header)?;
        copy(input.file(), out, data, end - data, limits, control)?;
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
fn validate_metadata(entries: &[crate::ArchiveEntry], safe: bool) -> Result<()> {
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
    Ok(())
}
pub(crate) fn verify(
    file: &mut File,
    limits: &ResourceLimits,
    control: &OperationControl,
    password: Option<&str>,
) -> Result<()> {
    verify_with_source_budget(file, limits, control, password, None)
}
fn verify_with_source_budget(
    file: &mut File,
    limits: &ResourceLimits,
    control: &OperationControl,
    password: Option<&str>,
    source_budget: Option<(u64, usize)>,
) -> Result<()> {
    file.rewind()?;
    let (entries, _, safe) = zip::list(file, crate::NameEncoding::Auto, limits, control)?;
    validate_metadata(&entries, safe)?;
    let mut plan = zip::ValidatedZip::new(file, &entries, limits, control, false)?;
    let mut total = 0;
    let (mut source_bytes, retained_count) = source_budget.unwrap_or((0, 0));
    for (index, entry) in entries.iter().enumerate() {
        control.record_work(|work| work.zip_candidate_payload_decodes += 1);
        let bytes = plan.stream(
            entry.id.ordinal,
            limits,
            control,
            password,
            &mut total,
            |_| Ok(()),
        )?;
        if index < retained_count {
            source_bytes = source_bytes
                .checked_add(bytes)
                .ok_or(ArchiveError::ResourceLimit)?;
            if source_bytes > limits.max_output_bytes {
                return Err(ArchiveError::ResourceLimit);
            }
        }
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
    let selected = intent(snapshot, edits, limits, control)?;
    let intent_bytes = intent_metadata(&selected);
    let intent_limits = reserve_metadata(limits, intent_bytes)?;
    validate_metadata(&snapshot.entries, snapshot.backend_safe)?;
    let directory = zip_container::single_directory(&mut source, &intent_limits, control)?;
    let comment = selected
        .comment
        .clone()
        .unwrap_or_else(|| directory.comment.clone());
    let mut locals = structure(
        &mut source,
        &snapshot.entries,
        &directory,
        &intent_limits,
        control,
    )?;
    let mut removed_bytes = 0;
    for entry in snapshot
        .entries
        .iter()
        .filter(|entry| selected.deleted.contains(&entry.id.ordinal))
    {
        control.record_work(|work| work.zip_source_payload_decodes += 1);
        locals.stream(
            entry.id.ordinal,
            limits,
            control,
            password,
            &mut removed_bytes,
            |_| Ok(()),
        )?;
    }
    // Validate ALL kept metadata before creating the candidate, including distinct local extras.
    for (index, (row, local)) in directory.records.iter().zip(locals.records()).enumerate() {
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
        &mut locals,
        &mut output,
        directory,
        &selected,
        &mut records,
        (limits, control),
    )?;
    let retained_count = records.len();
    drop(locals);
    let source_metadata = retained_snapshot_bytes(snapshot)?;
    for name in &selected.directories {
        control.checkpoint()?;
        records.push(empty_directory(name, output.stream_position()?));
        output.write_all(&directory_header(name))?;
    }
    if let Some(additions) = additions {
        let retained = source_metadata
            .checked_add(records_metadata(&records)?)
            .and_then(|bytes| bytes.checked_add(intent_bytes))
            .and_then(|bytes| bytes.checked_add(comment.capacity() as u64))
            .ok_or(ArchiveError::ResourceLimit)?;
        let addition_limits = reserve_metadata(limits, retained)?;
        let directory = zip_container::single_directory(additions, &addition_limits, control)?;
        let listing_limits =
            reserve_metadata(&addition_limits, records_metadata(&directory.records)?)?;
        let (entries, _, safe) = zip::list(
            additions,
            crate::NameEncoding::Auto,
            &listing_limits,
            control,
        )?;
        validate_metadata(&entries, safe)?;
        let mut locals = structure(additions, &entries, &directory, &addition_limits, control)?;
        let all = Intent {
            deleted: HashSet::new(),
            renamed: HashMap::new(),
            directories: Vec::new(),
            comment: None,
        };
        write_records(
            &mut locals,
            &mut output,
            directory,
            &all,
            &mut records,
            (limits, control),
        )?;
    }
    if records.len() as u64 > limits.max_entries {
        return Err(ArchiveError::ResourceLimit);
    }
    zip_container::finish_single(&mut output, &records, comment, control)?;
    drop(records);
    drop(selected);
    let candidate_limits = reserve_metadata(limits, source_metadata)?;
    if output.stream_position()? > limits.max_output_bytes {
        return Err(ArchiveError::ResourceLimit);
    }
    output.flush()?;
    verify_with_source_budget(
        &mut output,
        &candidate_limits,
        control,
        password,
        Some((removed_bytes, retained_count)),
    )?;
    crate::engine::ensure_revision(&mut source, &snapshot.revision, control)?;
    control.checkpoint()?;
    output.rewind()?;
    Ok(output)
}

#[cfg(test)]
#[path = "zip_edit_selection_tests.rs"]
mod selection_tests;
