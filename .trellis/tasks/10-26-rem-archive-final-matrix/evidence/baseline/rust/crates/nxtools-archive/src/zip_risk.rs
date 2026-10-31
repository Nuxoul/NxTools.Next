//! Known per-member expansion warning (ZIP and isolated codecs). Never changes admitted limits.
use crate::{
    ArchiveEntry, ArchiveError, EntryKind, OperationControl, ResourceLimits, Result,
    interaction::{InteractionPort, Prompt, PromptResponse, PromptTicket},
    worker::{WorkerFrame, write_frame},
};
use serde::{Deserialize, Serialize};
use std::io::Write;

const WARNING_BYTES: u64 = 64 << 20;
const WARNING_RATIO: u64 = 1000;

#[derive(Clone, Copy)]
pub(crate) struct Thresholds {
    bytes: u64,
    ratio: u64,
}
impl Default for Thresholds {
    fn default() -> Self {
        Self {
            bytes: WARNING_BYTES,
            ratio: WARNING_RATIO,
        }
    }
}
impl Thresholds {
    fn requires_confirmation(self, expanded: Option<u64>, packed: Option<u64>) -> bool {
        matches!((expanded, packed), (Some(expanded), Some(packed)) if packed != 0 && expanded > self.bytes && u128::from(expanded) > u128::from(packed) * u128::from(self.ratio))
    }
}
#[cfg(debug_assertions)]
#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FixtureThresholds {
    pub bytes: u64,
    pub ratio: u64,
}
#[cfg(debug_assertions)]
impl FixtureThresholds {
    pub(crate) fn valid(self) -> bool {
        (1..=WARNING_BYTES).contains(&self.bytes) && (1..=WARNING_RATIO).contains(&self.ratio)
    }
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RiskPrompt {
    pub ticket: PromptTicket,
    pub ordinal: u64,
    pub declared_bytes: u64,
    pub packed_bytes: u64,
    pub admitted_entry_bytes: u64,
    pub admitted_output_bytes: u64,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RiskReply {
    pub ticket: PromptTicket,
    pub approved_limit: Option<u64>,
}
impl RiskReply {
    pub(crate) fn response(&self) -> PromptResponse {
        self.approved_limit
            .map_or(PromptResponse::Cancel, |approved_limit| {
                PromptResponse::Risk { approved_limit }
            })
    }
}

pub(crate) fn required(entry: &ArchiveEntry) -> bool {
    entry.kind() == EntryKind::File
        && Thresholds::default().requires_confirmation(entry.size(), entry.compressed_size())
}
pub(crate) struct ZipRisk<'a> {
    port: &'a InteractionPort,
    thresholds: Thresholds,
}
impl<'a> ZipRisk<'a> {
    pub(crate) fn new(port: &'a InteractionPort) -> Self {
        Self {
            port,
            thresholds: Thresholds::default(),
        }
    }
    #[cfg(debug_assertions)]
    pub(crate) fn fixture(mut self, fixture: Option<FixtureThresholds>) -> Self {
        if let Some(value) = fixture {
            self.thresholds = Thresholds {
                bytes: value.bytes,
                ratio: value.ratio,
            };
        }
        self
    }
    pub(crate) fn confirm<W: Write>(
        &self,
        entry: &ArchiveEntry,
        limits: &ResourceLimits,
        control: &OperationControl,
        writer: &mut W,
    ) -> Result<()> {
        if entry.kind() != EntryKind::File
            || !self
                .thresholds
                .requires_confirmation(entry.size(), entry.compressed_size())
        {
            return Ok(());
        }
        let declared = entry.size().ok_or(ArchiveError::CorruptArchive)?;
        if declared > limits.max_entry_bytes {
            return Err(ArchiveError::ResourceLimit);
        }
        let response = self.port.request_notified(
            Prompt::Risk {
                declared_bytes: declared,
                limit_bytes: limits.max_entry_bytes,
            },
            control,
            |pending| {
                let mut frame = WorkerFrame::state(control.operation_id(), "risk_prompt");
                frame.risk_prompt = Some(RiskPrompt {
                    ticket: pending.ticket,
                    ordinal: entry.id().ordinal,
                    declared_bytes: declared,
                    packed_bytes: entry
                        .compressed_size()
                        .ok_or(ArchiveError::CorruptArchive)?,
                    admitted_entry_bytes: limits.max_entry_bytes,
                    admitted_output_bytes: limits.max_output_bytes,
                });
                write_frame(writer, &frame).map_err(|_| ArchiveError::IoFailure)
            },
        )?;
        match response {
            PromptResponse::Risk { approved_limit } if approved_limit == limits.max_entry_bytes => {
                control.checkpoint()?;
                write_frame(
                    writer,
                    &WorkerFrame::state(control.operation_id(), "risk_confirmed"),
                )
                .map_err(|_| ArchiveError::IoFailure)
            }
            _ => Err(ArchiveError::Cancelled),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn production_threshold_is_strict_and_unknown_or_zero_packed_has_no_ratio() {
        let policy = Thresholds::default();
        assert!(!policy.requires_confirmation(Some(64 << 20), Some(1)));
        assert!(!policy.requires_confirmation(Some(100_000_000), Some(100_000)));
        assert!(policy.requires_confirmation(Some(100_000_001), Some(100_000)));
        assert!(policy.requires_confirmation(Some((64 << 20) + 1), Some(1)));
        for packed in [None, Some(0)] {
            assert!(!policy.requires_confirmation(Some(u64::MAX), packed));
        }
        assert!(!policy.requires_confirmation(None, Some(1)));
        assert!(!policy.requires_confirmation(Some(u64::MAX), Some(u64::MAX)));
    }
    #[test]
    #[cfg(debug_assertions)]
    fn fixture_thresholds_only_increase_test_sensitivity() {
        assert!(
            FixtureThresholds {
                bytes: 1024,
                ratio: 2
            }
            .valid()
        );
        for fixture in [
            FixtureThresholds { bytes: 0, ratio: 2 },
            FixtureThresholds {
                bytes: WARNING_BYTES + 1,
                ratio: 2,
            },
            FixtureThresholds { bytes: 1, ratio: 0 },
            FixtureThresholds {
                bytes: 1,
                ratio: WARNING_RATIO + 1,
            },
        ] {
            assert!(!fixture.valid());
        }
    }
}
