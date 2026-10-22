//! ZIP intake is validation only: no directory creation, extraction or execution.
//! Classic single-disk ZIP, Stored/Deflate, <=4096 files (including manifest),
//! <=4096 directory nodes (explicit + implicit), <=1024-byte names / 64 levels.
//! ZIP64, SFX/prepended data and alternate Unicode-path extra fields are unsupported.
use crate::{ImportPreview, MAX_MANIFEST_BYTES, PackageError, bundle};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::{Cursor, Read},
    path::Path,
};

const MAX_ARCHIVE_BYTES: u64 = 256 * 1024 * 1024;
const MAX_FILES: usize = 4096;
const MAX_DIRECTORIES: usize = 4096;
const MANIFEST: &str = "extension.json";

struct Entry {
    name: String,
    directory: bool,
    size: u64,
    local: usize,
    end: usize,
}

impl ImportPreview {
    /// Validate and retain only declared immutable package bytes. Dropping this
    /// preview (Cancel) has no filesystem effects; confirmation uses `stage_into`.
    pub fn from_archive(path: &Path) -> Result<Self, PackageError> {
        bundle::reject_link(path)?;
        let file = fs::File::open(path).map_err(|_| PackageError::FileOperation)?;
        let metadata = file.metadata().map_err(|_| PackageError::FileOperation)?;
        if !metadata.is_file() {
            return Err(PackageError::ArchiveInvalid);
        }
        if metadata.len() > MAX_ARCHIVE_BYTES {
            return Err(PackageError::ArchiveTooLarge);
        }
        let mut bytes = Vec::new();
        file.take(MAX_ARCHIVE_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| PackageError::FileOperation)?;
        if bytes.len() as u64 > MAX_ARCHIVE_BYTES {
            return Err(PackageError::ArchiveTooLarge);
        }
        // Preflight before ZipArchive allocates its central directory. zip indexes
        // names in a map and can discard exact duplicates: inspect every raw entry.
        let entries = inspect(&bytes)?;
        let manifest_entries: Vec<_> = entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| {
                !entry.directory
                    && entry
                        .name
                        .rsplit('/')
                        .next()
                        .is_some_and(|name| name.eq_ignore_ascii_case(MANIFEST))
            })
            .collect();
        if manifest_entries.len() != 1 {
            return Err(PackageError::ArchiveInvalid);
        }
        let (manifest_index, manifest_entry) = manifest_entries[0];
        let prefix = if manifest_entry.name == MANIFEST {
            ""
        } else {
            let (wrapper, leaf) = manifest_entry
                .name
                .split_once('/')
                .ok_or(PackageError::ArchiveInvalid)?;
            if leaf != MANIFEST || wrapper.is_empty() {
                return Err(PackageError::ArchiveInvalid);
            }
            &manifest_entry.name[..wrapper.len() + 1]
        };
        if !prefix.is_empty()
            && entries.iter().any(|entry| {
                !(entry.name.starts_with(prefix)
                    || (entry.directory && entry.name == prefix.trim_end_matches('/')))
            })
        {
            return Err(PackageError::ArchiveInvalid);
        }
        if manifest_entry.size > MAX_MANIFEST_BYTES as u64 {
            return Err(PackageError::ArchiveTooLarge);
        }
        let mut zip = zip::ZipArchive::new(Cursor::new(bytes.as_slice()))
            .map_err(|_| PackageError::ArchiveInvalid)?;
        if zip.len() != entries.len() {
            return Err(PackageError::ArchiveUnsafe);
        }
        let mut remaining = bundle::MAX_CODE_BYTES + MAX_MANIFEST_BYTES as u64;
        let manifest_bytes = read_entry(
            &mut zip,
            manifest_index,
            manifest_entry,
            MAX_MANIFEST_BYTES as u64,
            &mut remaining,
            true,
        )?;
        let manifest = crate::strict_json::manifest(&manifest_bytes)?;
        manifest.validate()?;
        if serde_json::to_vec(&manifest)
            .map_err(|_| PackageError::InvalidManifest)?
            .len()
            > MAX_MANIFEST_BYTES
        {
            return Err(PackageError::ArchiveTooLarge);
        }
        // Preserve the manifest's spelling/hash, but use canonical separators to
        // locate declared bytes. Never silently map a case alias to a listed file.
        let mut wanted = BTreeMap::new();
        for file in &manifest.files {
            let normalized = file.replace('\\', "/");
            validate_name(&normalized)?;
            wanted.insert(format!("{prefix}{normalized}"), file.clone());
        }
        let mut code_files = BTreeMap::new();
        let mut other_remaining = bundle::MAX_CODE_BYTES;
        for (index, entry) in entries.iter().enumerate() {
            if index == manifest_index {
                continue;
            }
            let listed = wanted.get(&entry.name);
            let content = read_entry(
                &mut zip,
                index,
                entry,
                other_remaining,
                &mut remaining,
                listed.is_some(),
            )?;
            // Actual bytes of ignored extras count too; their CRC is still checked.
            other_remaining = other_remaining
                .checked_sub(entry.size)
                .ok_or(PackageError::ArchiveTooLarge)?;
            if let Some(name) = listed {
                code_files.insert(name.clone(), content);
            }
        }
        if code_files.len() != manifest.files.len() {
            return Err(PackageError::CodeRequired);
        }
        let content_hash = bundle::combined_hash(&manifest, &bundle::file_hashes(&code_files))?;
        let preview = Self {
            manifest,
            content_hash,
            code_files,
            code_directory: None,
            staged: false,
        };
        preview.validate_preview()?;
        Ok(preview)
    }
}

fn read_entry(
    zip: &mut zip::ZipArchive<Cursor<&[u8]>>,
    index: usize,
    entry: &Entry,
    limit: u64,
    remaining: &mut u64,
    retain: bool,
) -> Result<Vec<u8>, PackageError> {
    if entry.size > limit {
        return Err(PackageError::ArchiveTooLarge);
    }
    let mut file = zip
        .by_index(index)
        .map_err(|_| PackageError::ArchiveInvalid)?;
    if file.encrypted()
        || !matches!(
            file.compression(),
            zip::CompressionMethod::Stored | zip::CompressionMethod::Deflated
        )
        || file.name().replace('\\', "/").trim_end_matches('/') != entry.name
        || file.size() != entry.size
    {
        return Err(PackageError::ArchiveInvalid);
    }
    let mut content = Vec::new();
    let mut actual = 0u64;
    let mut buffer = [0u8; 32 * 1024];
    loop {
        let count = file
            .read(&mut buffer)
            .map_err(|_| PackageError::ArchiveInvalid)?;
        if count == 0 {
            break;
        }
        actual += count as u64;
        *remaining = remaining
            .checked_sub(count as u64)
            .ok_or(PackageError::ArchiveTooLarge)?;
        if actual > limit {
            return Err(PackageError::ArchiveTooLarge);
        }
        if actual > entry.size {
            return Err(PackageError::ArchiveInvalid);
        }
        if retain {
            content.extend_from_slice(&buffer[..count]);
        }
    }
    if actual != entry.size || (entry.directory && actual != 0) {
        return Err(PackageError::ArchiveInvalid);
    }
    Ok(content)
}

fn word(bytes: &[u8], at: usize) -> Result<u16, PackageError> {
    let value = bytes.get(at..at + 2).ok_or(PackageError::ArchiveInvalid)?;
    Ok(u16::from_le_bytes([value[0], value[1]]))
}
fn dword(bytes: &[u8], at: usize) -> Result<u32, PackageError> {
    let value = bytes.get(at..at + 4).ok_or(PackageError::ArchiveInvalid)?;
    Ok(u32::from_le_bytes([value[0], value[1], value[2], value[3]]))
}
fn extras(bytes: &[u8]) -> Result<(), PackageError> {
    let mut at = 0;
    while at < bytes.len() {
        let id = word(bytes, at)?;
        let len = word(bytes, at + 2)? as usize;
        at += 4;
        if matches!(id, 0x0001 | 0x7075) || at + len > bytes.len() {
            return Err(PackageError::ArchiveInvalid);
        }
        at += len;
    }
    Ok(())
}

fn inspect(bytes: &[u8]) -> Result<Vec<Entry>, PackageError> {
    let start = bytes.len().saturating_sub(22 + u16::MAX as usize);
    let end = bytes
        .len()
        .checked_sub(22)
        .ok_or(PackageError::ArchiveInvalid)?;
    let eocd = (start..=end)
        .rev()
        .find(|&at| {
            bytes.get(at..at + 4) == Some(&[0x50, 0x4b, 0x05, 0x06])
                && word(bytes, at + 20).is_ok_and(|len| at + 22 + len as usize == bytes.len())
        })
        .ok_or(PackageError::ArchiveInvalid)?;
    let count = word(bytes, eocd + 10)? as usize;
    if count > MAX_FILES + MAX_DIRECTORIES {
        return Err(PackageError::ArchiveTooLarge);
    }
    let central = dword(bytes, eocd + 16)? as usize;
    let central_size = dword(bytes, eocd + 12)? as usize;
    if word(bytes, eocd + 4)? != 0
        || word(bytes, eocd + 6)? != 0
        || word(bytes, eocd + 8)? as usize != count
        || central.checked_add(central_size) != Some(eocd)
    {
        return Err(PackageError::ArchiveInvalid);
    }
    let mut entries = Vec::new();
    let mut nodes = BTreeMap::new();
    let mut explicit = BTreeSet::new();
    let (mut files, mut directories, mut total) = (0, 0, 0u64);
    let mut at = central;
    for _ in 0..count {
        if dword(bytes, at)? != 0x02014b50 {
            return Err(PackageError::ArchiveInvalid);
        }
        let flags = word(bytes, at + 8)?;
        let method = word(bytes, at + 10)?;
        if flags & !0x080e != 0 || !matches!(method, 0 | 8) || (method == 0 && flags & 6 != 0) {
            return Err(PackageError::ArchiveInvalid);
        }
        let compressed = dword(bytes, at + 20)? as usize;
        let size = dword(bytes, at + 24)? as u64;
        let name_len = word(bytes, at + 28)? as usize;
        let extra_len = word(bytes, at + 30)? as usize;
        let comment_len = word(bytes, at + 32)? as usize;
        if name_len > 1024 {
            return Err(PackageError::ArchiveTooLarge);
        }
        if word(bytes, at + 34)? != 0 {
            return Err(PackageError::ArchiveInvalid);
        }
        let raw = bytes
            .get(at + 46..at + 46 + name_len)
            .ok_or(PackageError::ArchiveInvalid)?;
        if flags & 0x800 == 0 && !raw.is_ascii() {
            return Err(PackageError::ArchiveUnsafe);
        }
        let raw_name = std::str::from_utf8(raw).map_err(|_| PackageError::ArchiveUnsafe)?;
        let normalized = raw_name.replace('\\', "/");
        let directory = normalized.ends_with('/');
        let name = if directory {
            &normalized[..normalized.len() - 1]
        } else {
            &normalized
        };
        validate_name(name)?;
        let attrs = dword(bytes, at + 38)?;
        let kind = (attrs >> 16) & 0o170000;
        if (kind != 0 && kind != if directory { 0o040000 } else { 0o100000 })
            || attrs & 0x440 != 0
            || (!directory && attrs & 0x10 != 0)
            || (directory && size != 0)
        {
            return Err(PackageError::ArchiveUnsafe);
        }
        insert_name(name, directory, &mut nodes, &mut explicit, &mut directories)?;
        if !directory {
            files += 1;
        }
        if files > MAX_FILES || directories > MAX_DIRECTORIES {
            return Err(PackageError::ArchiveTooLarge);
        }
        total = total
            .checked_add(size)
            .ok_or(PackageError::ArchiveTooLarge)?;
        if size > bundle::MAX_CODE_BYTES
            || total > bundle::MAX_CODE_BYTES + MAX_MANIFEST_BYTES as u64
        {
            return Err(PackageError::ArchiveTooLarge);
        }
        let next = at + 46 + name_len + extra_len + comment_len;
        if next > eocd {
            return Err(PackageError::ArchiveInvalid);
        }
        extras(&bytes[at + 46 + name_len..at + 46 + name_len + extra_len])?;
        // Require the local record to describe the same object, not an alternate
        // path/type hidden from the central directory or an overlapping payload.
        let local = dword(bytes, at + 42)? as usize;
        if dword(bytes, local)? != 0x04034b50
            || word(bytes, local + 6)? != flags
            || word(bytes, local + 8)? != method
            || word(bytes, local + 26)? as usize != name_len
        {
            return Err(PackageError::ArchiveInvalid);
        }
        let local_extra = word(bytes, local + 28)? as usize;
        let data = local + 30 + name_len + local_extra;
        let mut data_end = data
            .checked_add(compressed)
            .ok_or(PackageError::ArchiveInvalid)?;
        if data_end > central || bytes.get(local + 30..local + 30 + name_len) != Some(raw) {
            return Err(PackageError::ArchiveInvalid);
        }
        extras(
            bytes
                .get(local + 30 + name_len..data)
                .ok_or(PackageError::ArchiveInvalid)?,
        )?;
        if flags & 8 == 0 {
            if dword(bytes, local + 14)? != dword(bytes, at + 16)?
                || dword(bytes, local + 18)? as usize != compressed
                || dword(bytes, local + 22)? as u64 != size
            {
                return Err(PackageError::ArchiveInvalid);
            }
        } else {
            if dword(bytes, data_end)? == 0x08074b50 {
                data_end += 4;
            }
            if dword(bytes, data_end)? != dword(bytes, at + 16)?
                || dword(bytes, data_end + 4)? as usize != compressed
                || dword(bytes, data_end + 8)? as u64 != size
            {
                return Err(PackageError::ArchiveInvalid);
            }
            data_end += 12;
        }
        entries.push(Entry {
            name: name.to_owned(),
            directory,
            size,
            local,
            end: data_end,
        });
        at = next;
    }
    if at != eocd {
        return Err(PackageError::ArchiveInvalid);
    }
    let mut ranges: Vec<_> = entries
        .iter()
        .map(|entry| (entry.local, entry.end))
        .collect();
    ranges.sort_unstable();
    let mut boundary = 0;
    for (local, end) in ranges {
        if local != boundary || end > central {
            return Err(PackageError::ArchiveInvalid);
        }
        boundary = end;
    }
    if boundary != central {
        return Err(PackageError::ArchiveInvalid);
    }
    Ok(entries)
}

fn validate_name(name: &str) -> Result<(), PackageError> {
    if name.is_empty()
        || name.len() > 1024
        || name.split('/').count() > 64
        || (name != MANIFEST && !crate::script::relative_code_path(name))
        || name.split('/').any(|part| {
            let stem = part
                .split('.')
                .next()
                .unwrap_or_default()
                .trim_end_matches(' ')
                .to_uppercase();
            matches!(
                stem.as_str(),
                "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
            ) || ["COM", "LPT"].iter().any(|prefix| {
                stem.strip_prefix(*prefix).is_some_and(|suffix| {
                    matches!(
                        suffix,
                        "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
                    )
                })
            })
        })
    {
        return Err(PackageError::ArchiveUnsafe);
    }
    Ok(())
}

fn insert_name(
    name: &str,
    directory: bool,
    nodes: &mut BTreeMap<String, (String, bool)>,
    explicit: &mut BTreeSet<String>,
    directories: &mut usize,
) -> Result<(), PackageError> {
    let mut path = String::new();
    let parts: Vec<_> = name.split('/').collect();
    for (index, part) in parts.iter().enumerate() {
        if index != 0 {
            path.push('/');
        }
        path.push_str(part);
        let is_directory = index + 1 != parts.len() || directory;
        // Upper + lower closure catches aliases such as final sigma / Kelvin sign
        // as well as ASCII case. Conservative collisions are rejected, not merged.
        let key = path.to_uppercase().to_lowercase();
        if let Some((spelling, existing_directory)) = nodes.get(&key) {
            if spelling != &path || !*existing_directory || !is_directory {
                return Err(PackageError::ArchiveUnsafe);
            }
        } else {
            nodes.insert(key.clone(), (path.clone(), is_directory));
            if is_directory {
                *directories += 1;
            }
        }
        if index + 1 == parts.len() && !explicit.insert(key) {
            return Err(PackageError::ArchiveUnsafe);
        }
    }
    Ok(())
}
