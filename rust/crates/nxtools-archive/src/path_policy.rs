use crate::{ArchiveEntry, ArchiveError, EntryKind, EntryWarning, OperationControl, Result};
use std::collections::HashMap;

pub(crate) fn validate_relative(path: &str) -> Result<()> {
    components(path, false).map(|_| ())
}

/// Lexical policy only. Native no-reparse/handle-relative commit is independently required.
fn components(path: &str, directory: bool) -> Result<Vec<&str>> {
    if path.is_empty() || path.starts_with(['/', '\\']) {
        return Err(ArchiveError::UnsafePath);
    }
    let path = if directory {
        path.strip_suffix(['/', '\\']).unwrap_or(path)
    } else {
        path
    };
    let parts: Vec<_> = path.split(['/', '\\']).collect();
    // Finite depth also bounds projection/trie traversal; long paths are not silently truncated.
    if parts.len() > 256 {
        return Err(ArchiveError::UnsafePath);
    }
    for part in &parts {
        if part.is_empty()
            || *part == "."
            || *part == ".."
            || part.ends_with(['.', ' '])
            || part
                .chars()
                .any(|c| c.is_control() || "<>:\"|?*".contains(c))
        {
            return Err(ArchiveError::UnsafePath);
        }
        let base = part
            .split('.')
            .next()
            .unwrap_or_default()
            .trim_end_matches(' ')
            .to_uppercase();
        if matches!(
            base.as_str(),
            "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
        ) || ["COM", "LPT"].iter().any(|prefix| {
            base.strip_prefix(prefix).is_some_and(|suffix| {
                matches!(
                    suffix,
                    "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
                )
            })
        }) {
            return Err(ArchiveError::UnsafePath);
        }
    }
    Ok(parts)
}

#[derive(Default)]
struct Node {
    children: HashMap<String, usize>,
    spelling: String,
    first_entry: Option<usize>,
    file: bool,
    collision: bool,
}

pub(crate) fn annotate(
    entries: &mut [ArchiveEntry],
    control: &OperationControl,
    mut accounted: u64,
    limit: u64,
) -> Result<()> {
    // Flat arena avoids recursive drop/stack growth on hostile deep paths.
    let mut nodes = vec![Node::default()];
    for (index, entry) in entries.iter_mut().enumerate() {
        control.checkpoint()?;
        let Ok(parts) = components(&entry.display_path, entry.kind == EntryKind::Directory) else {
            entry.warnings.push(EntryWarning::UnsafePath);
            continue;
        };
        let mut current = 0;
        for part in parts {
            control.checkpoint()?;
            let key = part.to_uppercase();
            let next = if let Some(&child) = nodes[current].children.get(&key) {
                child
            } else {
                accounted = accounted
                    .checked_add(320 + (part.len() as u64).saturating_mul(4))
                    .ok_or(ArchiveError::ResourceLimit)?;
                if accounted > limit {
                    return Err(ArchiveError::ResourceLimit);
                }
                let child = nodes.len();
                nodes.push(Node {
                    spelling: part.to_owned(),
                    ..Default::default()
                });
                nodes[current].children.insert(key, child);
                child
            };
            if nodes[next].spelling != part {
                nodes[next].collision = true;
            }
            current = next;
        }
        if nodes[current].first_entry.is_some() {
            entry.warnings.push(EntryWarning::OutputCollision);
            // Mark all colliding records below, not just the latest ordinal.
            nodes[current].collision = true;
        } else {
            nodes[current].first_entry = Some(index);
        }
        nodes[current].file |= entry.kind != EntryKind::Directory;
    }
    for entry in entries {
        control.checkpoint()?;
        let Ok(parts) = components(&entry.display_path, entry.kind == EntryKind::Directory) else {
            continue;
        };
        let mut current = 0;
        let mut collision = false;
        for part in parts {
            control.checkpoint()?;
            // Same validated projection as insertion; no backend/display ordinal conversion.
            current = nodes[current].children[&part.to_uppercase()];
            let node = &nodes[current];
            collision |= node.collision || (node.file && !node.children.is_empty());
        }
        if collision && !entry.warnings.contains(&EntryWarning::OutputCollision) {
            entry.warnings.push(EntryWarning::OutputCollision);
        }
    }
    Ok(())
}
