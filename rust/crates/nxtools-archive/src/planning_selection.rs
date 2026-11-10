//! Operation-local membership over original records. No name, source or output authority.
use super::SelectionWork;
use crate::{ArchiveError, Result};
use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
};

#[derive(Clone, Copy)]
pub struct SelectionEntry<'a> {
    pub ordinal: u64,
    pub path: &'a str,
    pub directory: bool,
}

/// Positions have already been bound to original IDs by the owning snapshot/session.
/// Additional directories are display-projection roots, including implicit directories.
pub enum CatalogSelection<'a> {
    All,
    Entries {
        positions: &'a [usize],
        directories: &'a [&'a str],
    },
}

fn charge(used: &mut u64, bytes: u64, credit: u64) -> Result<()> {
    *used = used.checked_add(bytes).ok_or(ArchiveError::ResourceLimit)?;
    if *used > credit {
        return Err(ArchiveError::ResourceLimit);
    }
    Ok(())
}

/// Bind arbitrary original ordinals in request order, without assuming ordinal == vector position.
/// The selected-only index retains the first catalog record, matching the original linear find.
pub fn bind_ordinals<T>(
    catalog: &[T],
    ordinals: &[u64],
    metadata_credit: u64,
    ordinal: impl Fn(&T) -> u64,
    mut checkpoint: impl FnMut() -> Result<()>,
    work: &mut SelectionWork,
) -> Result<Vec<usize>> {
    let mut used = 0;
    // Hash buckets/control bytes plus requested-order output coexist until binding completes.
    charge(
        &mut used,
        (ordinals.len() as u64)
            .checked_mul(96)
            .ok_or(ArchiveError::ResourceLimit)?,
        metadata_credit,
    )?;
    if ordinals.is_empty() {
        return Ok(Vec::new());
    }
    let mut selected = HashMap::with_capacity(ordinals.len());
    for &id in ordinals {
        checkpoint()?;
        selected.entry(id).or_insert(None);
    }
    for (index, item) in catalog.iter().enumerate() {
        checkpoint()?;
        work.ordinal_probes += 1;
        if let Some(position) = selected.get_mut(&ordinal(item)) {
            position.get_or_insert(index);
        }
    }
    let mut result = Vec::with_capacity(ordinals.len());
    for id in ordinals {
        checkpoint()?;
        result.push(selected[id].ok_or(ArchiveError::SourceChanged)?);
    }
    Ok(result)
}

fn normalized_root<'a>(path: &'a str, work: &mut SelectionWork) -> Cow<'a, str> {
    work.root_normalizations += 1;
    if path.contains('\\') {
        work.normalization_allocations += 1;
        Cow::Owned(path.replace('\\', "/"))
    } else {
        Cow::Borrowed(path)
    }
}

fn prefix(path: &str, work: &mut SelectionWork) -> String {
    let path = path.trim_end_matches('/');
    work.prefix_allocations += 1;
    if path.is_empty() {
        String::new()
    } else {
        format!("{path}/")
    }
}

/// Sort slash-ended prefixes and remove duplicates/descendants in place.
pub fn merge_directory_prefixes(
    roots: &mut Vec<String>,
    mut checkpoint: impl FnMut() -> Result<()>,
    work: &mut SelectionWork,
) -> Result<()> {
    roots.sort_unstable();
    let mut kept = 0;
    for index in 0..roots.len() {
        checkpoint()?;
        if kept != 0 {
            work.prefix_probes += 1;
            if roots[index].starts_with(&roots[kept - 1]) {
                continue;
            }
        }
        roots.swap(kept, index);
        kept += 1;
    }
    roots.truncate(kept);
    work.directory_roots += roots.len() as u64;
    Ok(())
}

/// Query sorted, disjoint slash-ended prefixes without allocating a candidate string.
pub fn matches_directory_prefixes(
    roots: &[String],
    path: &str,
    directory: bool,
    work: &mut SelectionWork,
) -> bool {
    let end = roots.partition_point(|root| {
        work.prefix_probes += 1;
        root.as_str() <= path
    });
    let descendant = end != 0 && {
        work.prefix_probes += 1;
        path.starts_with(&roots[end - 1])
    };
    descendant
        || directory
            && roots.get(end).is_some_and(|root| {
                work.prefix_probes += 1;
                path.trim_end_matches('/') == root.trim_end_matches('/')
            })
}

/// One normalization/expansion pass in catalog order. The callback consumes the same temporary
/// normalized path; no normalized catalog is retained. Its final argument is the conservative
/// selection reservation, including the ordinal result, for caller-owned output coexistence.
/// The callback returns its current retained-output reservation (zero when collecting IDs only).
/// Callback effects remain operation-local: an unresolved directory can fail at the end.
pub fn expand_selection<T>(
    catalog: &[T],
    selection: CatalogSelection<'_>,
    metadata_credit: u64,
    entry: impl Fn(usize, &T) -> SelectionEntry<'_>,
    mut emit: impl FnMut(usize, &T, &str, u64) -> Result<u64>,
    mut checkpoint: impl FnMut() -> Result<()>,
    work: &mut SelectionWork,
) -> Result<Vec<u64>> {
    let (all, positions, directories) = match selection {
        CatalogSelection::All => (true, &[][..], &[][..]),
        CatalogSelection::Entries {
            positions,
            directories,
        } => (false, positions, directories),
    };
    if !all && positions.is_empty() && directories.is_empty() {
        return Ok(Vec::new());
    }
    let mut used = 0u64;
    // Ordinal output plus selected-only hash tables, root strings/index and bound positions.
    charge(
        &mut used,
        (catalog.len() as u64)
            .checked_mul(8)
            .ok_or(ArchiveError::ResourceLimit)?,
        metadata_credit,
    )?;
    let root_capacity = positions
        .len()
        .checked_add(directories.len())
        .ok_or(ArchiveError::ResourceLimit)?;
    charge(
        &mut used,
        (root_capacity as u64)
            .checked_mul(256)
            .ok_or(ArchiveError::ResourceLimit)?,
        metadata_credit,
    )?;
    let mut exact = HashSet::with_capacity(positions.len());
    let mut roots = Vec::with_capacity(root_capacity);
    let mut root_paths = HashMap::with_capacity(positions.len());
    let mut pending = HashSet::with_capacity(directories.len());
    for &position in positions {
        checkpoint()?;
        work.ordinal_probes += 1;
        let item = entry(
            position,
            catalog.get(position).ok_or(ArchiveError::SourceChanged)?,
        );
        exact.insert(item.ordinal);
        if item.directory && !root_paths.contains_key(&position) {
            charge(
                &mut used,
                (item.path.len() as u64)
                    .checked_mul(2)
                    .and_then(|bytes| bytes.checked_add(1))
                    .ok_or(ArchiveError::ResourceLimit)?,
                metadata_credit,
            )?;
            let path = normalized_root(item.path, work);
            roots.push(prefix(&path, work));
            root_paths.insert(position, path);
        }
    }
    for directory in directories {
        checkpoint()?;
        charge(
            &mut used,
            (directory.len() as u64)
                .checked_mul(3)
                .and_then(|bytes| bytes.checked_add(1))
                .ok_or(ArchiveError::ResourceLimit)?,
            metadata_credit,
        )?;
        let path = normalized_root(directory, work);
        pending.insert(path.trim_end_matches('/').to_owned());
        roots.push(prefix(&path, work));
    }
    merge_directory_prefixes(&mut roots, &mut checkpoint, work)?;
    let mut selected = Vec::with_capacity(catalog.len());
    let mut retained = 0u64;
    for (index, original) in catalog.iter().enumerate() {
        checkpoint()?;
        work.catalog_visits += 1;
        let item = entry(index, original);
        if !all && roots.is_empty() && !exact.contains(&item.ordinal) {
            continue;
        }
        let cached = root_paths.get(&index);
        let allocate = cached.is_none() && item.path.contains('\\');
        let reservation = used
            .checked_add(if allocate { item.path.len() as u64 } else { 0 })
            .ok_or(ArchiveError::ResourceLimit)?;
        if reservation
            .checked_add(retained)
            .is_none_or(|bytes| bytes > metadata_credit)
        {
            return Err(ArchiveError::ResourceLimit);
        }
        let path = if let Some(path) = cached {
            Cow::Borrowed(path.as_ref())
        } else {
            work.path_normalizations += 1;
            if allocate {
                work.normalization_allocations += 1;
                Cow::Owned(item.path.replace('\\', "/"))
            } else {
                Cow::Borrowed(item.path)
            }
        };
        if !(all
            || exact.contains(&item.ordinal)
            || matches_directory_prefixes(&roots, &path, item.directory, work))
        {
            continue;
        }
        if !pending.is_empty() {
            // Resolve original implicit roots, including nested roots dropped from the index.
            // Each lookup is a path-component boundary, never a selected-root × catalog loop.
            work.prefix_probes += 1;
            pending.remove("");
            for (at, _) in path.match_indices('/') {
                work.prefix_probes += 1;
                pending.remove(&path[..at]);
            }
            if item.directory {
                work.prefix_probes += 1;
                pending.remove(path.trim_end_matches('/'));
            }
        }
        retained = emit(index, original, &path, reservation)?;
        if reservation
            .checked_add(retained)
            .is_none_or(|bytes| bytes > metadata_credit)
        {
            return Err(ArchiveError::ResourceLimit);
        }
        selected.push(item.ordinal);
    }
    if !pending.is_empty() {
        return Err(ArchiveError::SourceChanged);
    }
    Ok(selected)
}
