//! Ordinal-bound sevenz edit plan. No filesystem or publication authority.
use crate::planning::{CatalogSelection, SelectionEntry, SelectionWork, expand_selection};
use crate::{ArchiveError, OperationControl, ResourceLimits, Result};
use nxtools_protocol::archive_job::{ArchiveJobModify, ArchiveJobModifyAction as Action};
use std::collections::{HashMap, HashSet};
#[derive(Clone)]
pub(crate) struct Planned {
    pub entry: sevenz_rust2::ArchiveEntry,
    pub origin: Option<usize>,
    pub added: Option<usize>,
}
pub(crate) fn validate(
    entries: &[Planned],
    limits: &ResourceLimits,
    control: &OperationControl,
) -> Result<()> {
    control.checkpoint()?;
    if entries.len() as u64 > limits.max_entries {
        return Err(ArchiveError::ResourceLimit);
    }
    let mut seen = HashMap::new();
    let mut charged = 0u64;
    let mut total = 0u64;
    for item in entries {
        control.checkpoint()?;
        let name = item.entry.name.trim_end_matches('/');
        if name.contains('\\') {
            return Err(ArchiveError::UnsafePath);
        }
        crate::path_policy::validate_relative(name)?;
        charged = charged
            .checked_add(1024 + name.len() as u64 * 16)
            .ok_or(ArchiveError::ResourceLimit)?;
        if charged > limits.max_metadata_bytes {
            return Err(ArchiveError::ResourceLimit);
        }
        let key = name.to_uppercase();
        if seen.insert(key, item.entry.is_directory).is_some() {
            return Err(ArchiveError::OutputCollision);
        }
        if !item.entry.is_directory {
            if item.entry.size > limits.max_entry_bytes {
                return Err(ArchiveError::ResourceLimit);
            }
            total = total
                .checked_add(item.entry.size)
                .ok_or(ArchiveError::ResourceLimit)?;
            if total > limits.max_output_bytes {
                return Err(ArchiveError::ResourceLimit);
            }
        }
    }
    for name in seen.keys() {
        control.checkpoint()?;
        let mut at = name.as_str();
        while let Some((parent, _)) = at.rsplit_once('/') {
            if seen.get(parent) == Some(&false) {
                return Err(ArchiveError::OutputCollision);
            }
            at = parent;
        }
    }
    Ok(())
}
pub(crate) fn build(
    files: &[sevenz_rust2::ArchiveEntry],
    intent: &ArchiveJobModify,
    limits: &ResourceLimits,
    control: &OperationControl,
) -> Result<Vec<Planned>> {
    let mut selection = SelectionWork::default();
    let result = build_observed(files, intent, limits, control, &mut selection);
    control.record_work(|work| work.selection += selection);
    result
}
pub(crate) fn build_observed(
    files: &[sevenz_rust2::ArchiveEntry],
    intent: &ArchiveJobModify,
    limits: &ResourceLimits,
    control: &OperationControl,
    work: &mut SelectionWork,
) -> Result<Vec<Planned>> {
    control.checkpoint()?;
    if intent.ordinals.len()
        != intent
            .ordinals
            .iter()
            .copied()
            .collect::<HashSet<_>>()
            .len()
    {
        return Err(ArchiveError::SourceChanged);
    }
    let selected = intent
        .ordinals
        .iter()
        .map(|n| usize::try_from(*n).map_err(|_| ArchiveError::SourceChanged))
        .collect::<Result<HashSet<_>>>()?;
    if selected.iter().any(|i| *i >= files.len()) {
        return Err(ArchiveError::SourceChanged);
    }
    let original_bytes = files.iter().try_fold(0u64, |bytes, item| {
        work.metadata_visits += 1;
        bytes
            .checked_add(std::mem::size_of::<sevenz_rust2::ArchiveEntry>() as u64)
            .and_then(|bytes| bytes.checked_add(item.name.len() as u64))
            .ok_or(ArchiveError::ResourceLimit)
    })?;
    let selection_credit = limits
        .max_metadata_bytes
        .checked_sub(original_bytes)
        .ok_or(ArchiveError::ResourceLimit)?
        .min(limits.max_working_bytes);
    let mut removed = Vec::new();
    let mut rename = None;
    match intent.action {
        Action::Delete => {
            let positions = selected.iter().copied().collect::<Vec<_>>();
            removed = expand_selection(
                files,
                CatalogSelection::Entries {
                    positions: &positions,
                    directories: &[],
                },
                selection_credit,
                |index, item| SelectionEntry {
                    ordinal: index as u64,
                    path: &item.name,
                    directory: item.is_directory,
                },
                |_, _, _, _| Ok(0),
                || control.checkpoint(),
                work,
            )?;
        }
        Action::Rename => {
            let &index = selected.iter().next().ok_or(ArchiveError::SourceChanged)?;
            if selected.len() != 1 {
                return Err(ArchiveError::UnsupportedOperation);
            }
            let entry = &files[index];
            let new = intent.value.as_deref().ok_or(ArchiveError::UnsafePath)?;
            let directory = new.ends_with('/');
            let old = if let Some(prefix) = intent.source_prefix.as_deref() {
                let prefix = prefix.trim_end_matches('/');
                crate::path_policy::validate_relative(prefix)?;
                if prefix.contains('\\') {
                    return Err(ArchiveError::UnsafePath);
                }
                if !directory
                    || !(entry.name.trim_end_matches('/') == prefix
                        || entry.name.starts_with(&format!("{prefix}/")))
                {
                    return Err(ArchiveError::SourceChanged);
                }
                prefix
            } else {
                if directory && !entry.is_directory {
                    return Err(ArchiveError::UnsupportedOperation);
                }
                entry.name.trim_end_matches('/')
            };
            if directory != entry.is_directory && !directory {
                return Err(ArchiveError::UnsupportedOperation);
            }
            let new = new.trim_end_matches('/');
            crate::path_policy::validate_relative(new)?;
            if new.contains('\\') {
                return Err(ArchiveError::UnsafePath);
            }
            if directory && new.starts_with(&format!("{old}/")) {
                return Err(ArchiveError::UnsafePath);
            }
            rename = Some((old.to_owned(), new.to_owned(), directory));
        }
        Action::ReplaceContent => {
            if selected.len() != 1 || files[*selected.iter().next().expect("one")].is_directory {
                return Err(ArchiveError::UnsupportedOperation);
            }
        }
        Action::Add | Action::NewDirectory => {}
        Action::Comment => return Err(ArchiveError::UnsupportedOperation),
    }
    let mut result = Vec::new();
    let mut plan_bytes = (removed.capacity() as u64)
        .checked_mul(8)
        .ok_or(ArchiveError::ResourceLimit)?;
    for (index, item) in files.iter().enumerate() {
        control.checkpoint()?;
        work.plan_visits += 1;
        if removed.binary_search(&(index as u64)).is_ok() {
            continue;
        }
        plan_bytes = plan_bytes
            .checked_add(2 * std::mem::size_of::<Planned>() as u64)
            .and_then(|bytes| bytes.checked_add(item.name.len() as u64))
            .ok_or(ArchiveError::ResourceLimit)?;
        if plan_bytes > selection_credit {
            return Err(ArchiveError::ResourceLimit);
        }
        let mut entry = item.clone();
        if let Some((old, new, directory)) = &rename {
            if entry.name.trim_end_matches('/') == old {
                entry.name = new.clone();
            } else if *directory && entry.name.starts_with(&format!("{old}/")) {
                entry.name = format!("{new}{}", &entry.name[old.len()..]);
            }
        }
        result.push(Planned {
            entry,
            origin: Some(index),
            added: if intent.action == Action::ReplaceContent && selected.contains(&index) {
                Some(0)
            } else {
                None
            },
        });
    }
    if intent.action == Action::NewDirectory {
        result.push(Planned {
            entry: sevenz_rust2::ArchiveEntry::new_directory(
                intent
                    .value
                    .as_deref()
                    .ok_or(ArchiveError::UnsafePath)?
                    .trim_end_matches('/'),
            ),
            origin: None,
            added: None,
        });
    }
    validate(&result, limits, control)?;
    Ok(result)
}

#[cfg(test)]
#[path = "sevenz_edit_plan_tests.rs"]
mod sevenz_edit_plan_tests;
