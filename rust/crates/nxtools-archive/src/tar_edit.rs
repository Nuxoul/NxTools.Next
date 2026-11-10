//! Edit-only TAR admission and immutable raw metadata. Ordinary readers retain their own policy.
use crate::{ArchiveError, OperationControl, ResourceLimits, Result};
use nxtools_protocol::archive_job::{ArchiveJobModify, ArchiveJobModifyAction as Action};
use std::{
    collections::{HashMap, HashSet},
    fs::File,
    io::{Read, Seek, SeekFrom, Write},
};

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct Extension {
    pub header: [u8; 512],
    pub data: Vec<u8>,
}
#[derive(Clone)]
pub(crate) struct Member {
    pub header: [u8; 512],
    pub extensions: Vec<Extension>,
    pub name: String,
    pub size: u64,
    pub directory: bool,
    pub offset: u64,
    pub origin: Option<usize>,
    pub added: Option<usize>,
}
fn exact(file: &mut impl Read, bytes: &mut [u8]) -> Result<()> {
    file.read_exact(bytes).map_err(|e| {
        if e.kind() == std::io::ErrorKind::UnexpectedEof {
            ArchiveError::CorruptArchive
        } else {
            crate::worker_tree_io::io_error(e)
        }
    })
}
fn header(raw: &[u8; 512]) -> tar::Header {
    let mut h = tar::Header::new_old();
    h.as_mut_bytes().copy_from_slice(raw);
    h
}
fn checked(raw: &[u8; 512]) -> Result<tar::Header> {
    let h = header(raw);
    let sum: u32 = raw
        .iter()
        .enumerate()
        .map(|(i, v)| {
            if (148..156).contains(&i) {
                32
            } else {
                u32::from(*v)
            }
        })
        .sum();
    if h.cksum().map_err(|_| ArchiveError::CorruptArchive)? != sum {
        return Err(ArchiveError::CorruptArchive);
    }
    if h.as_ustar().is_none() && h.as_gnu().is_none() {
        return Err(ArchiveError::UnsupportedOperation);
    }
    // Unknown GNU sparse/offset/continuation or vendor padding cannot be silently reinterpreted.
    if h.as_gnu().is_some() && raw[369..].iter().any(|v| *v != 0)
        || h.as_ustar().is_some() && raw[500..].iter().any(|v| *v != 0)
    {
        return Err(ArchiveError::UnsupportedOperation);
    }
    Ok(h)
}
fn pax(data: &[u8]) -> Result<Vec<(String, Vec<u8>)>> {
    let mut out = Vec::new();
    let mut at = 0;
    let mut seen = HashSet::new();
    while at < data.len() {
        let space = at
            + data[at..]
                .iter()
                .position(|b| *b == b' ')
                .ok_or(ArchiveError::CorruptArchive)?;
        let n = std::str::from_utf8(&data[at..space])
            .ok()
            .and_then(|s| s.parse::<usize>().ok())
            .ok_or(ArchiveError::CorruptArchive)?;
        let end = at
            .checked_add(n)
            .filter(|end| *end <= data.len() && *end > space + 2)
            .ok_or(ArchiveError::CorruptArchive)?;
        let body = &data[space + 1..end];
        if body.last() != Some(&b'\n') {
            return Err(ArchiveError::CorruptArchive);
        }
        let equal = body
            .iter()
            .position(|b| *b == b'=')
            .ok_or(ArchiveError::CorruptArchive)?;
        let key = std::str::from_utf8(&body[..equal]).map_err(|_| ArchiveError::CorruptArchive)?;
        if !matches!(
            key,
            "path" | "size" | "mtime" | "atime" | "ctime" | "uid" | "gid" | "uname" | "gname"
        ) {
            return Err(ArchiveError::UnsupportedOperation);
        }
        if !seen.insert(key.to_owned()) {
            return Err(ArchiveError::UnsupportedOperation);
        }
        let value = body[equal + 1..body.len() - 1].to_vec();
        if value.contains(&0) {
            return Err(ArchiveError::CorruptArchive);
        }
        std::str::from_utf8(&value).map_err(|_| ArchiveError::UnsupportedOperation)?;
        if matches!(key, "size" | "uid" | "gid")
            && std::str::from_utf8(&value)
                .ok()
                .and_then(|s| s.parse::<u64>().ok())
                .is_none()
        {
            return Err(ArchiveError::UnsupportedOperation);
        }
        if matches!(key, "mtime" | "atime" | "ctime")
            && !value.is_empty()
            && crate::metadata::unix_decimal(&value) == crate::metadata::ModifiedTime::Invalid
        {
            return Err(ArchiveError::UnsupportedOperation);
        }
        out.push((key.to_owned(), value));
        at = end;
    }
    Ok(out)
}
fn pax_bytes(fields: &[(String, Vec<u8>)]) -> Vec<u8> {
    let mut out = Vec::new();
    for (key, value) in fields {
        let base = key.len() + value.len() + 3;
        let mut n = base + 1;
        loop {
            let next = base + n.to_string().len();
            if next == n {
                break;
            }
            n = next;
        }
        out.extend_from_slice(format!("{n} {key}=").as_bytes());
        out.extend_from_slice(value);
        out.push(b'\n');
    }
    out
}
/// Replace one validated PAX record, retaining every unrelated record's exact representation.
fn change_pax(data: &mut Vec<u8>, key: &str, value: &[u8]) -> Result<bool> {
    let fields = pax(data)?;
    let mut at = 0;
    for (field, previous) in fields {
        // pax() has validated each extent; retain the original length prefix when untouched.
        let space = data[at..]
            .iter()
            .position(|b| *b == b' ')
            .ok_or(ArchiveError::CorruptArchive)?
            + at;
        let length = std::str::from_utf8(&data[at..space])
            .ok()
            .and_then(|n| n.parse::<usize>().ok())
            .ok_or(ArchiveError::CorruptArchive)?;
        let end = at + length;
        if field == key {
            if previous != value {
                data.splice(at..end, pax_bytes(&[(field, value.to_vec())]));
            }
            return Ok(true);
        }
        at = end;
    }
    Ok(false)
}
impl Extension {
    fn change_data(&mut self, data: Vec<u8>) {
        // TAR header checksum covers only the header, never this extension payload.
        if self.data.len() != data.len() {
            let mut h = header(&self.header);
            h.set_size(data.len() as u64);
            h.set_cksum();
            self.header = *h.as_bytes();
        }
        self.data = data;
    }
}
fn extent(offset: u64, size: u64, length: u64) -> Result<u64> {
    offset
        .checked_add(512)
        .and_then(|n| n.checked_add(size))
        .and_then(|n| n.checked_add((512 - size % 512) % 512))
        .filter(|n| *n <= length)
        .ok_or(ArchiveError::CorruptArchive)
}
pub(crate) fn inspect(
    file: &mut File,
    limits: &ResourceLimits,
    control: &OperationControl,
) -> Result<Vec<Member>> {
    if limits.max_working_bytes < 256 * 1024 {
        return Err(ArchiveError::ResourceLimit);
    }
    let length = file.metadata()?.len();
    if length > limits.max_output_bytes {
        return Err(ArchiveError::ResourceLimit);
    }
    let mut at = 0u64;
    let mut out = Vec::new();
    let mut pending = Vec::<Extension>::new();
    let mut charged = 0u64;
    let mut extension_count = 0u64;
    loop {
        control.checkpoint()?;
        if at.checked_add(512).is_none_or(|n| n > length) {
            return Err(ArchiveError::CorruptArchive);
        }
        file.seek(SeekFrom::Start(at))?;
        let mut raw = [0; 512];
        exact(file, &mut raw)?;
        if raw.iter().all(|v| *v == 0) {
            if !pending.is_empty() || at.checked_add(1024).is_none_or(|n| n > length) {
                return Err(ArchiveError::CorruptArchive);
            }
            let mut buffer = [0; 65536];
            loop {
                control.checkpoint()?;
                let n = file.read(&mut buffer)?;
                if n == 0 {
                    break;
                }
                if buffer[..n].iter().any(|v| *v != 0) {
                    return Err(ArchiveError::CorruptArchive);
                }
            }
            validate(&out, limits, control)?;
            return Ok(out);
        }
        let h = checked(&raw)?;
        let kind = h.entry_type();
        let fixed = h.entry_size().map_err(|_| ArchiveError::CorruptArchive)?;
        if kind.is_pax_local_extensions() || kind.is_gnu_longname() {
            extension_count += 1;
            if extension_count > limits.max_entries || fixed > 65536 || pending.len() >= 2 {
                return Err(ArchiveError::ResourceLimit);
            }
            if pending.iter().any(|e| e.header[156] == raw[156]) {
                return Err(ArchiveError::UnsupportedOperation);
            }
            charged = charged
                .checked_add(4096 + fixed.saturating_mul(16))
                .ok_or(ArchiveError::ResourceLimit)?;
            if charged > limits.max_metadata_bytes {
                return Err(ArchiveError::ResourceLimit);
            }
            let next = extent(at, fixed, length)?;
            let mut data = vec![0; fixed as usize];
            exact(file, &mut data)?;
            if kind.is_pax_local_extensions() {
                pax(&data)?;
            } else if data.last() != Some(&0) || data[..data.len() - 1].contains(&0) {
                return Err(ArchiveError::UnsupportedOperation);
            }
            pending.push(Extension { header: raw, data });
            at = next;
            continue;
        }
        if !matches!(raw[156], 0 | b'0' | b'5') || raw[157..257].iter().any(|v| *v != 0) {
            return Err(ArchiveError::UnsupportedOperation);
        }
        // Device fields on ordinary members are not ordinary metadata; accept only empty/zero fields.
        if raw[329..345].iter().any(|v| !matches!(*v, 0 | b'0' | b' ')) {
            return Err(ArchiveError::UnsupportedOperation);
        }
        let mut name = h.path_bytes().into_owned();
        if raw[..100].contains(&b'\\')
            || h.as_ustar().is_some_and(|_| raw[345..500].contains(&b'\\'))
        {
            return Err(ArchiveError::UnsafePath);
        }
        let mut size = fixed;
        for ext in &pending {
            if ext.header[156] == b'L' {
                name = ext.data[..ext.data.len() - 1].to_vec();
            }
        }
        for ext in &pending {
            if ext.header[156] == b'x' {
                for (key, value) in pax(&ext.data)? {
                    match key.as_str() {
                        "path" => name = value,
                        "size" => {
                            size = std::str::from_utf8(&value)
                                .ok()
                                .and_then(|s| s.parse().ok())
                                .ok_or(ArchiveError::CorruptArchive)?
                        }
                        _ => {}
                    }
                }
            }
        }
        let name = String::from_utf8(name).map_err(|_| ArchiveError::UnsupportedOperation)?;
        let directory = raw[156] == b'5';
        if directory && size != 0 || !directory && name.ends_with('/') {
            return Err(ArchiveError::UnsupportedOperation);
        }
        let next = extent(at, size, length)?;
        charged = charged
            .checked_add(8192 + name.len() as u64 * 128)
            .ok_or(ArchiveError::ResourceLimit)?;
        if charged > limits.max_metadata_bytes || out.len() as u64 >= limits.max_entries {
            return Err(ArchiveError::ResourceLimit);
        }
        out.push(Member {
            header: raw,
            extensions: std::mem::take(&mut pending),
            name,
            size,
            directory,
            offset: at + 512,
            origin: Some(out.len()),
            added: None,
        });
        at = next;
    }
}
pub(crate) fn validate(
    items: &[Member],
    limits: &ResourceLimits,
    control: &OperationControl,
) -> Result<()> {
    if items.len() as u64 > limits.max_entries {
        return Err(ArchiveError::ResourceLimit);
    }
    let mut seen = HashMap::<String, (bool, String)>::new();
    let mut parents = HashMap::<String, String>::new();
    let mut total = 0u64;
    let mut charge = 0u64;
    for item in items {
        control.checkpoint()?;
        let name = item.name.trim_end_matches('/');
        if item.name.contains('\\') {
            return Err(ArchiveError::UnsafePath);
        }
        crate::path_policy::validate_relative(name)?;
        let mut parent_charge = 0u64;
        let mut parent = name;
        while let Some((prefix, _)) = parent.rsplit_once('/') {
            control.checkpoint()?;
            parent_charge = parent_charge
                .checked_add(256 + prefix.len() as u64 * 8)
                .ok_or(ArchiveError::ResourceLimit)?;
            parent = prefix;
        }
        charge = charge
            .checked_add(
                8192 + item.name.len() as u64 * 128
                    + parent_charge
                    + item
                        .extensions
                        .iter()
                        .map(|e| 4096 + e.data.len() as u64 * 16)
                        .sum::<u64>(),
            )
            .ok_or(ArchiveError::ResourceLimit)?;
        if charge > limits.max_metadata_bytes {
            return Err(ArchiveError::ResourceLimit);
        }
        let mut parent = name;
        while let Some((prefix, _)) = parent.rsplit_once('/') {
            control.checkpoint()?;
            let key = prefix.to_uppercase();
            if parents.get(&key).is_some_and(|previous| previous != prefix) {
                return Err(ArchiveError::OutputCollision);
            }
            parents.entry(key).or_insert_with(|| prefix.to_owned());
            parent = prefix;
        }
        if seen
            .insert(name.to_uppercase(), (item.directory, name.to_owned()))
            .is_some()
        {
            return Err(ArchiveError::OutputCollision);
        }
        if item.size > limits.max_entry_bytes {
            return Err(ArchiveError::ResourceLimit);
        }
        total = total
            .checked_add(item.size)
            .ok_or(ArchiveError::ResourceLimit)?;
        if total > limits.max_output_bytes {
            return Err(ArchiveError::ResourceLimit);
        }
    }
    for (parent, exact) in parents {
        control.checkpoint()?;
        if seen
            .get(&parent)
            .is_some_and(|(directory, spelling)| !directory || spelling != &exact)
        {
            return Err(ArchiveError::OutputCollision);
        }
    }
    Ok(())
}
impl Member {
    pub(crate) fn new(name: String, directory: bool, size: u64) -> Result<Self> {
        let mut h = tar::Header::new_ustar();
        h.set_uid(0);
        h.set_gid(0);
        h.set_mode(if directory { 0o755 } else { 0o644 });
        h.set_mtime(0);
        h.set_size(size);
        h.set_entry_type(if directory {
            tar::EntryType::Directory
        } else {
            tar::EntryType::Regular
        });
        h.set_cksum();
        let mut item = Self {
            header: *h.as_bytes(),
            extensions: vec![],
            name: String::new(),
            size,
            directory,
            offset: 0,
            origin: None,
            added: None,
        };
        item.change(&name, size)?;
        Ok(item)
    }
    pub(crate) fn change(&mut self, name: &str, size: u64) -> Result<()> {
        let changed_name = self.name != name;
        let changed_size = self.size != size;
        if !changed_name && !changed_size {
            return Ok(());
        }
        let mut h = header(&self.header);
        if changed_size {
            h.set_size(size);
        }
        let mut overridden = false;
        for ext in &mut self.extensions {
            if ext.header[156] == b'L' && changed_name {
                let mut data = name.as_bytes().to_vec();
                data.push(0);
                ext.change_data(data);
                overridden = true;
            }
            if ext.header[156] == b'x' {
                let mut data = ext.data.clone();
                if changed_name && change_pax(&mut data, "path", name.as_bytes())? {
                    overridden = true;
                }
                if changed_size {
                    change_pax(&mut data, "size", size.to_string().as_bytes())?;
                }
                ext.change_data(data);
            }
        }
        if changed_name && !overridden {
            let mut named = h.clone();
            if named.set_path(name).is_ok() {
                h = named;
            } else {
                let field = ("path".to_owned(), name.as_bytes().to_vec());
                if let Some(ext) = self.extensions.iter_mut().find(|e| e.header[156] == b'x') {
                    let mut data = ext.data.clone();
                    data.extend_from_slice(&pax_bytes(&[field]));
                    ext.change_data(data);
                } else {
                    let data = pax_bytes(&[field]);
                    let mut eh = tar::Header::new_ustar();
                    eh.set_path("PaxHeader")
                        .map_err(|_| ArchiveError::UnsafePath)?;
                    eh.set_entry_type(tar::EntryType::XHeader);
                    eh.set_size(data.len() as u64);
                    eh.set_cksum();
                    self.extensions.push(Extension {
                        header: *eh.as_bytes(),
                        data,
                    });
                }
            }
        }
        if h.as_bytes() != &self.header {
            h.set_cksum();
            self.header = *h.as_bytes();
        }
        self.name = name.to_owned();
        self.size = size;
        Ok(())
    }
    pub(crate) fn write_header(&self, writer: &mut impl Write) -> Result<()> {
        for ext in &self.extensions {
            writer.write_all(&ext.header)?;
            writer.write_all(&ext.data)?;
            padding(writer, ext.data.len() as u64)?;
        }
        writer.write_all(&self.header)?;
        Ok(())
    }
}
pub(crate) fn padding(writer: &mut impl Write, size: u64) -> Result<()> {
    writer.write_all(&[0; 512][..((512 - size % 512) % 512) as usize])?;
    Ok(())
}
/// TAR's explicit end-of-archive is its finish boundary; publication may not precede flush.
pub(crate) fn finish(writer: &mut impl Write) -> Result<()> {
    writer
        .write_all(&[0; 1024])
        .map_err(crate::worker_tree_io::io_error)?;
    writer.flush().map_err(crate::worker_tree_io::io_error)
}
pub(crate) fn plan(
    old: &[Member],
    intent: &ArchiveJobModify,
    limits: &ResourceLimits,
    control: &OperationControl,
) -> Result<Vec<Member>> {
    let mut work = crate::planning::SelectionWork::default();
    let result = plan_observed(old, intent, limits, control, &mut work);
    control.record_work(|current| current.selection += work);
    result
}
fn retained_member_bytes(item: &Member) -> u64 {
    (std::mem::size_of::<Member>()
        + item.name.capacity()
        + item.extensions.capacity() * std::mem::size_of::<Extension>()) as u64
        + item
            .extensions
            .iter()
            .map(|extension| extension.data.capacity() as u64)
            .sum::<u64>()
}
fn plan_observed(
    old: &[Member],
    intent: &ArchiveJobModify,
    limits: &ResourceLimits,
    control: &OperationControl,
    work: &mut crate::planning::SelectionWork,
) -> Result<Vec<Member>> {
    control.checkpoint()?;
    let original_bytes = old.iter().try_fold(0u64, |bytes, item| {
        work.metadata_visits += 1;
        bytes
            .checked_add(retained_member_bytes(item))
            .ok_or(ArchiveError::ResourceLimit)
    })?;
    let selected_bytes = (intent.ordinals.len() as u64).saturating_mul(96);
    let credit = limits
        .max_metadata_bytes
        .checked_sub(original_bytes)
        .and_then(|bytes| bytes.checked_sub(selected_bytes))
        .ok_or(ArchiveError::ResourceLimit)?
        .min(limits.max_working_bytes);
    let selected = intent
        .ordinals
        .iter()
        .map(|n| usize::try_from(*n).map_err(|_| ArchiveError::SourceChanged))
        .collect::<Result<HashSet<_>>>()?;
    if selected.len() != intent.ordinals.len() || selected.iter().any(|n| *n >= old.len()) {
        return Err(ArchiveError::SourceChanged);
    }
    let mut removed = Vec::<u64>::new();
    let mut rename = None;
    match intent.action {
        Action::Delete => {
            let positions = selected.iter().copied().collect::<Vec<_>>();
            removed = crate::planning::expand_selection(
                old,
                crate::planning::CatalogSelection::Entries {
                    positions: &positions,
                    directories: &[],
                },
                credit,
                |index, item| crate::planning::SelectionEntry {
                    ordinal: index as u64,
                    path: &item.name,
                    directory: item.directory,
                },
                |_, _, _, _| Ok(0),
                || control.checkpoint(),
                work,
            )?;
        }
        Action::Rename => {
            if selected.len() != 1 {
                return Err(ArchiveError::UnsupportedOperation);
            }
            let n = *selected.iter().next().expect("one");
            let new = intent.value.as_deref().ok_or(ArchiveError::UnsafePath)?;
            let directory = new.ends_with('/');
            let old_name = if let Some(prefix) = intent.source_prefix.as_deref() {
                if !directory
                    || !(old[n].directory
                        && old[n].name.trim_end_matches('/') == prefix.trim_end_matches('/')
                        || old[n].name.starts_with(prefix))
                {
                    return Err(ArchiveError::SourceChanged);
                }
                prefix.trim_end_matches('/')
            } else {
                if directory != old[n].directory {
                    return Err(ArchiveError::UnsupportedOperation);
                }
                old[n].name.trim_end_matches('/')
            };
            let new = new.trim_end_matches('/');
            crate::path_policy::validate_relative(new)?;
            if directory
                && new
                    .to_uppercase()
                    .starts_with(&format!("{}/", old_name.to_uppercase()))
            {
                return Err(ArchiveError::UnsafePath);
            }
            if directory && !new.eq_ignore_ascii_case(old_name) {
                let old_key = old_name.to_uppercase();
                let new_key = new.to_uppercase();
                for item in old {
                    control.checkpoint()?;
                    let key = item.name.trim_end_matches('/').to_uppercase();
                    let moving = key == old_key || key.starts_with(&format!("{old_key}/"));
                    if !moving && (key == new_key || key.starts_with(&format!("{new_key}/"))) {
                        return Err(ArchiveError::OutputCollision);
                    }
                }
            }
            rename = Some((old_name.to_owned(), new.to_owned(), directory));
        }
        Action::ReplaceContent => {
            if selected.len() != 1 || old[*selected.iter().next().expect("one")].directory {
                return Err(ArchiveError::UnsupportedOperation);
            }
        }
        Action::Add | Action::NewDirectory => {}
        Action::Comment => return Err(ArchiveError::UnsupportedOperation),
    }
    let mut out = Vec::new();
    let mut output_bytes = original_bytes
        .saturating_add(selected_bytes)
        .saturating_add(removed.capacity() as u64 * 8);
    for (i, item) in old.iter().enumerate() {
        control.checkpoint()?;
        work.plan_visits += 1;
        if removed.binary_search(&(i as u64)).is_ok() {
            continue;
        }
        output_bytes = output_bytes
            .checked_add(retained_member_bytes(item).saturating_mul(2))
            .ok_or(ArchiveError::ResourceLimit)?;
        if output_bytes > limits.max_metadata_bytes {
            return Err(ArchiveError::ResourceLimit);
        }
        let mut item = item.clone();
        if let Some((old, new, directory)) = &rename {
            let name = if item.name.trim_end_matches('/') == old {
                format!("{new}{}", if item.directory { "/" } else { "" })
            } else if *directory && item.name.starts_with(&format!("{old}/")) {
                format!("{new}{}", &item.name[old.len()..])
            } else {
                item.name.clone()
            };
            item.change(&name, item.size)?;
        }
        if intent.action == Action::ReplaceContent && selected.contains(&i) {
            item.added = Some(0);
        }
        out.push(item);
    }
    if intent.action == Action::NewDirectory {
        let name = intent.value.clone().ok_or(ArchiveError::UnsafePath)?;
        out.push(Member::new(name, true, 0)?);
    }
    validate(&out, limits, control)?;
    Ok(out)
}

#[cfg(test)]
#[path = "tar_edit_tests.rs"]
mod tests;
