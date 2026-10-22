//! Existing-target extraction decisions; native output remains the original session owner.
use crate::{
    ArchiveError, OperationControl, Result,
    interaction::{ConflictDecision, InteractionPort, Prompt, PromptResponse, PromptTicket},
    worker_output::PrivateOutput,
};
use serde::{Deserialize, Serialize};
use std::{io::Write, sync::Arc};
use uuid::Uuid;

#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictPolicy {
    Ask,
    Overwrite,
    Skip,
    AutoRename,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ConflictReply {
    pub ticket: PromptTicket,
    pub decision: Option<ConflictDecision>,
    pub apply_to_all: bool,
}
impl ConflictReply {
    pub fn response(&self) -> PromptResponse {
        self.decision.map_or(PromptResponse::Cancel, |decision| {
            PromptResponse::Conflict {
                decision,
                apply_to_all: self.apply_to_all,
            }
        })
    }
}
#[derive(Clone, Serialize, Deserialize)]
pub struct ConflictPrompt {
    pub ticket: PromptTicket,
    pub existing_bytes: Option<u64>,
    pub incoming_bytes: Option<u64>,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct ExtractionSummary {
    pub files_skipped: u64,
    pub all_members_written: bool,
}

pub(crate) struct ExtractionConflict<'a> {
    policy: ConflictPolicy,
    port: &'a InteractionPort,
    remembered: Option<ConflictDecision>,
    reserved: Arc<[crate::ArchiveEntry]>,
    pub skipped: u64,
}
impl<'a> ExtractionConflict<'a> {
    pub fn new(
        policy: ConflictPolicy,
        port: &'a InteractionPort,
        names: Arc<[crate::ArchiveEntry]>,
    ) -> Self {
        Self {
            policy,
            port,
            remembered: None,
            reserved: names,
            skipped: 0,
        }
    }
    pub fn begin<W: Write + Send>(
        &mut self,
        native: &mut PrivateOutput<'_, W>,
        path: &str,
        incoming: Option<u64>,
    ) -> Result<Option<Uuid>> {
        let observed = native.call("observe_target", Some(path), None, &[])?;
        let Some(target) = observed.token else {
            return native
                .call("begin", Some(path), None, &[])?
                .token
                .ok_or(ArchiveError::IoFailure)
                .map(Some);
        };
        let decision = if let Some(decision) = self.remembered {
            decision
        } else {
            match self.policy {
                ConflictPolicy::Overwrite => ConflictDecision::Overwrite,
                ConflictPolicy::Skip => ConflictDecision::Skip,
                ConflictPolicy::AutoRename => ConflictDecision::AutoRename,
                ConflictPolicy::Ask => {
                    let control: OperationControl = native.control().clone();
                    let response = self.port.request_notified(
                        Prompt::Conflict {
                            existing_bytes: observed.existing_bytes,
                            incoming_bytes: incoming,
                        },
                        &control,
                        |pending| {
                            native.conflict_prompt(&ConflictPrompt {
                                ticket: pending.ticket,
                                existing_bytes: observed.existing_bytes,
                                incoming_bytes: incoming,
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
                        self.remembered = Some(decision);
                    }
                    decision
                }
            }
        };
        match decision {
            ConflictDecision::Skip => {
                self.skipped += 1;
                Ok(None)
            }
            ConflictDecision::Overwrite => native
                .call("begin_overwrite", Some(path), Some(target), &[])?
                .token
                .ok_or(ArchiveError::IoFailure)
                .map(Some),
            ConflictDecision::AutoRename => {
                for number in 2..=4097 {
                    native.control().checkpoint()?;
                    let candidate = numbered(path, number)?;
                    let key = candidate.to_uppercase();
                    if self.reserved.iter().any(|entry| {
                        let name = entry
                            .display_path()
                            .replace('\\', "/")
                            .trim_end_matches('/')
                            .to_uppercase();
                        name == key || name.starts_with(&format!("{key}/"))
                    }) {
                        continue;
                    }
                    if native
                        .call("observe_target", Some(&candidate), None, &[])?
                        .token
                        .is_some()
                    {
                        continue;
                    }
                    return native
                        .call("begin", Some(&candidate), None, &[])?
                        .token
                        .ok_or(ArchiveError::IoFailure)
                        .map(Some);
                }
                Err(ArchiveError::OutputCollision)
            }
        }
    }
}
fn numbered(path: &str, number: u32) -> Result<String> {
    let (parent, name) = path
        .rsplit_once('/')
        .map_or(("", path), |(parent, name)| (parent, name));
    let (stem, extension) = name
        .rsplit_once('.')
        .filter(|(stem, _)| !stem.is_empty())
        .map_or((name, String::new()), |(stem, ext)| {
            (stem, format!(".{ext}"))
        });
    let name = format!("{stem} ({number}){extension}");
    let candidate = if parent.is_empty() {
        name
    } else {
        format!("{parent}/{name}")
    };
    crate::path_policy::validate_relative(&candidate)?;
    Ok(candidate)
}
