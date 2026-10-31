//! Ordinal-bound sevenz edit plan. No filesystem or publication authority.
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
    let mut removed = HashSet::new();
    let mut rename = None;
    match intent.action {
        Action::Delete => {
            for &index in &selected {
                control.checkpoint()?;
                let entry = &files[index];
                removed.insert(index);
                if entry.is_directory {
                    let prefix = format!("{}/", entry.name.trim_end_matches('/'));
                    for (n, item) in files.iter().enumerate() {
                        control.checkpoint()?;
                        if item.name.starts_with(&prefix) {
                            removed.insert(n);
                        }
                    }
                }
            }
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
    for (index, item) in files.iter().enumerate() {
        control.checkpoint()?;
        if removed.contains(&index) {
            continue;
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
