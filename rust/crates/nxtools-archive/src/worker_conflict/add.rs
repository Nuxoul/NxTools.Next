//! Add namespace resolution; tickets and native effects retain their original owners.
use super::{ConflictPolicy, ConflictPrompt};
use crate::{
    ArchiveError, OperationControl, ResourceLimits, Result,
    creation::PlannedSource,
    interaction::{ConflictDecision, InteractionPort, Prompt, PromptResponse},
    worker_output::PrivateOutput,
};
use std::{collections::HashMap, io::Write};

pub(crate) struct ExistingMember<'a> {
    pub name: &'a str,
    pub directory: bool,
    pub size: Option<u64>,
}
pub(crate) struct ResolvedAddition {
    pub input: usize,
    pub name: String,
    pub replaces: Option<usize>,
}
pub(crate) fn display_result(
    inputs: &[PlannedSource],
    prefix: &str,
    selected: &[ResolvedAddition],
) -> nxtools_protocol::archive_job::ArchiveJobAddResult {
    use nxtools_protocol::archive_job::{
        ArchiveJobAddMember, ArchiveJobAddOutcome, ArchiveJobAddResult,
    };
    let mut members = Vec::new();
    let mut bytes = 0usize;
    for item in selected.iter().take(256) {
        let requested = format!("{prefix}{}", inputs[item.input].relative_path());
        let outcome = if item.replaces.is_some() {
            ArchiveJobAddOutcome::Overwritten
        } else if item.name.to_uppercase() != requested.to_uppercase() {
            ArchiveJobAddOutcome::Renamed
        } else {
            ArchiveJobAddOutcome::Added
        };
        if members.len() < 256 && bytes + item.name.len() <= 65536 {
            bytes += item.name.len();
            members.push(ArchiveJobAddMember {
                path: item.name.clone(),
                outcome,
            });
        }
    }
    ArchiveJobAddResult {
        total: selected.len() as u64,
        members,
    }
}
struct Occupied {
    name: String,
    size: Option<u64>,
    original: Option<usize>,
    selected: Option<usize>,
}

/// Resolve all names before any incoming payload is opened. Returned indices always refer to
/// the complete protected input vector, never a name-keyed subset of source identities.
pub(crate) fn resolve<'a, W: Write + Send>(
    inputs: &[PlannedSource],
    originals: impl Iterator<Item = ExistingMember<'a>>,
    prefix: &str,
    policy: Option<ConflictPolicy>,
    native: &mut PrivateOutput<'_, W>,
    interaction: &InteractionPort,
    limits: &ResourceLimits,
) -> Result<(Vec<ResolvedAddition>, u64)> {
    let control = native.control().clone();
    let mut reserved = HashMap::<String, bool>::new();
    let mut occupied = HashMap::<String, Occupied>::new();
    let mut metadata = 0u64;
    for (index, old) in originals.enumerate() {
        let normalized = old.name.replace('\\', "/");
        let name = normalized.trim_end_matches('/');
        reserve(
            name,
            old.directory,
            &mut reserved,
            &mut metadata,
            limits,
            &control,
        )?;
        occupy_parents(name, &mut occupied);
        occupied.insert(
            name.to_uppercase(),
            Occupied {
                name: name.to_owned(),
                size: old.size,
                original: Some(index),
                selected: None,
            },
        );
    }
    // Reservations include implicit parents, but do not establish an accepted directory.
    for input in inputs {
        let name = format!("{prefix}{}", input.relative_path().trim_end_matches('/'));
        reserve(
            &name,
            input.is_directory(),
            &mut reserved,
            &mut metadata,
            limits,
            &control,
        )?;
    }
    let mut selected: Vec<ResolvedAddition> = Vec::new();
    let mut skipped = 0;
    let mut remembered = None;
    for (index, input) in inputs.iter().enumerate() {
        control.checkpoint()?;
        let requested = format!("{prefix}{}", input.relative_path().trim_end_matches('/'));
        let mut name = requested.clone();
        // Logical directory merging retains the accepted spelling for every descendant.
        if let Some((parent, leaf)) = requested.rsplit_once('/')
            && let Some(existing) = occupied.get(&parent.to_uppercase())
        {
            name = format!("{}/{leaf}", existing.name);
        }
        let key = name.to_uppercase();
        let mut replaces = None;
        if let Some(old) = occupied.get(&key) {
            // The complete type reservation has already refused every file/directory mismatch.
            if input.is_directory() {
                continue;
            }
            let decision = if let Some(value) = remembered {
                value
            } else {
                match policy {
                    Some(ConflictPolicy::Overwrite) => ConflictDecision::Overwrite,
                    Some(ConflictPolicy::Skip) => ConflictDecision::Skip,
                    Some(ConflictPolicy::AutoRename) => ConflictDecision::AutoRename,
                    Some(ConflictPolicy::Ask) => {
                        let response = interaction.request_notified(
                            Prompt::Conflict {
                                existing_bytes: old.size,
                                incoming_bytes: Some(input.size()),
                            },
                            &control,
                            |pending| {
                                native.conflict_prompt(&ConflictPrompt {
                                    ticket: pending.ticket,
                                    existing_bytes: old.size,
                                    incoming_bytes: Some(input.size()),
                                    display: super::edit_display(&name, &old.name),
                                })
                            },
                        )?;
                        let PromptResponse::Conflict {
                            decision,
                            apply_to_all,
                        } = response
                        else {
                            return Err(ArchiveError::Cancelled);
                        };
                        if apply_to_all {
                            remembered = Some(decision);
                        }
                        decision
                    }
                    None => return Err(ArchiveError::OutputCollision),
                }
            };
            match decision {
                ConflictDecision::Skip => {
                    skipped += 1;
                    continue;
                }
                ConflictDecision::Overwrite => {
                    if let Some(at) = old.selected {
                        selected[at].input = index;
                        occupied.get_mut(&key).expect("current conflict").size = Some(input.size());
                        continue;
                    }
                    name = old.name.clone();
                    replaces = old.original;
                }
                ConflictDecision::AutoRename => {
                    let mut available = None;
                    for number in 2..=4097 {
                        control.checkpoint()?;
                        let candidate = super::numbered(&name, number)?;
                        if !reserved.contains_key(&candidate.to_uppercase()) {
                            available = Some(candidate);
                            break;
                        }
                    }
                    name = available.ok_or(ArchiveError::OutputCollision)?;
                    reserve(&name, false, &mut reserved, &mut metadata, limits, &control)?;
                }
            }
        }
        occupy_parents(&name, &mut occupied);
        occupied.insert(
            name.to_uppercase(),
            Occupied {
                name: name.clone(),
                size: Some(input.size()),
                original: replaces,
                selected: Some(selected.len()),
            },
        );
        if input.is_directory() {
            name.push('/');
        }
        selected.push(ResolvedAddition {
            input: index,
            name,
            replaces,
        });
    }
    Ok((selected, skipped))
}

fn reserve(
    name: &str,
    directory: bool,
    reserved: &mut HashMap<String, bool>,
    metadata: &mut u64,
    limits: &ResourceLimits,
    control: &OperationControl,
) -> Result<()> {
    crate::path_policy::validate_relative(name)?;
    if name.len() > 32767 {
        return Err(ArchiveError::ResourceLimit);
    }
    let mut prefix = String::new();
    let mut components = name.split('/').peekable();
    while let Some(component) = components.next() {
        control.checkpoint()?;
        if !prefix.is_empty() {
            prefix.push('/');
        }
        prefix.push_str(component);
        let is_directory = directory || components.peek().is_some();
        // Includes reserved/live maps, result strings and case-expanded keys, before allocation.
        *metadata = metadata
            .checked_add(512 + prefix.len() as u64 * 16)
            .ok_or(ArchiveError::ResourceLimit)?;
        if *metadata > limits.max_metadata_bytes {
            return Err(ArchiveError::ResourceLimit);
        }
        match reserved.insert(prefix.to_uppercase(), is_directory) {
            Some(old_directory) if old_directory != is_directory => {
                return Err(ArchiveError::OutputCollision);
            }
            _ => {}
        }
    }
    Ok(())
}

fn occupy_parents(name: &str, occupied: &mut HashMap<String, Occupied>) {
    for (end, _) in name.match_indices('/') {
        let parent = &name[..end];
        occupied
            .entry(parent.to_uppercase())
            .or_insert_with(|| Occupied {
                name: parent.to_owned(),
                size: None,
                original: None,
                selected: None,
            });
    }
}
