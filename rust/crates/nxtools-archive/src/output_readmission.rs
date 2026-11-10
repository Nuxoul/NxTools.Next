//! One manual declared-output preflight readmission; no process/scheduler ownership.
use crate::{
    ArchiveError, OperationControl, Result,
    interaction::{InteractionPort, Prompt, PromptResponse},
    worker::WorkerRequest,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// Mirrors existing ArchiveOutputSession.Write and ArchiveWorkerOutputBridge admission bounds.
pub const MAX_ENTRY_BYTES: u64 = 32 << 30;
pub const MAX_OUTPUT_BYTES: u64 = 1 << 50;
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OutputBudget {
    pub entry_bytes: u64,
    pub output_bytes: u64,
}
impl OutputBudget {
    pub fn valid(self) -> bool {
        self.entry_bytes > 0
            && self.entry_bytes <= MAX_ENTRY_BYTES
            && self.output_bytes > 0
            && self.output_bytes <= MAX_OUTPUT_BYTES
    }
    pub(crate) fn covers(self, other: Self) -> bool {
        self.entry_bytes >= other.entry_bytes && self.output_bytes >= other.output_bytes
    }
}
#[derive(Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceBinding {
    pub native: String,
    pub digest: [u8; 32],
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReadmissionOptions {
    pub logical_id: Uuid,
    pub expected_source: Option<SourceBinding>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OutputShortfall {
    pub current: OutputBudget,
    pub required: OutputBudget,
    pub source: SourceBinding,
}
/// The adapter returns ONLY after the original native runner's exited event, not private complete.
#[derive(Serialize, Deserialize)]
pub struct AttemptResult {
    pub operation_id: Uuid,
    pub cleanup_complete: bool,
    pub files_committed: u64,
    pub error_code: Option<String>,
    pub shortfall: Option<OutputShortfall>,
}
/// Runs at most two attempts. The caller supplies its existing native runner adapter, not a codec.
/// Output root/host-only options remain owned by that adapter across the two invocations.
pub fn execute(
    request: WorkerRequest,
    control: &OperationControl,
    interaction: &InteractionPort,
    run: impl FnMut(&WorkerRequest, &OperationControl) -> Result<AttemptResult>,
) -> Result<AttemptResult> {
    let logical_id = request.operation_id;
    let started = std::time::Instant::now();
    diagnostic(logical_id, logical_id, "started");
    let result = execute_attempts(request, control, interaction, run);
    let operation_id = result
        .as_ref()
        .map_or(logical_id, |result| result.operation_id);
    let phase = match &result {
        Ok(result) if result.error_code.is_none() => "succeeded",
        Err(ArchiveError::Cancelled) => "cancelled",
        _ => "failed",
    };
    tracing::info!(target: "nxtools_archive", %logical_id, %operation_id, phase,
        elapsed_ms = started.elapsed().as_millis() as u64, "archive.output_readmission");
    result
}
fn execute_attempts(
    mut request: WorkerRequest,
    control: &OperationControl,
    interaction: &InteractionPort,
    mut run: impl FnMut(&WorkerRequest, &OperationControl) -> Result<AttemptResult>,
) -> Result<AttemptResult> {
    request
        .validate()
        .map_err(|_| ArchiveError::UnsupportedOperation)?;
    if request.version != 2
        || request.operation != "extract"
        || request.operation_id != control.operation_id()
        || request.output_readmission.is_some()
        || request.delete_source.is_some()
        || request
            .extract_options
            .as_ref()
            .is_some_and(|options| options.reclamation.is_some())
        || request
            .job_extract
            .as_ref()
            .is_some_and(|options| options.reclamation.is_some())
    {
        return Err(ArchiveError::UnsupportedOperation);
    }
    let logical_id = request.operation_id;
    request.output_readmission = Some(ReadmissionOptions {
        logical_id,
        expected_source: None,
    });
    control.checkpoint()?;
    let first = run(&request, control)?;
    if first.operation_id != request.operation_id || !first.cleanup_complete {
        return Err(ArchiveError::IoFailure);
    }
    let Some(shortfall) = first.shortfall.as_ref() else {
        return Ok(first);
    };
    let current = OutputBudget {
        entry_bytes: request.limits.max_entry_bytes,
        output_bytes: request.limits.max_output_bytes,
    };
    if first.files_committed != 0
        || first.error_code.as_deref() != Some("ResourceLimit")
        || shortfall.current != current
        || current.covers(shortfall.required)
        || shortfall.source.native.is_empty()
        || shortfall.source.native.len() > 256
    {
        return Err(ArchiveError::IoFailure);
    }
    if !shortfall.required.valid() {
        return Err(ArchiveError::ResourceLimit);
    }
    diagnostic(logical_id, first.operation_id, "awaiting_output_budget");
    let response = interaction.request(
        Prompt::OutputBudget {
            current,
            required: shortfall.required,
        },
        control,
    )?;
    let PromptResponse::OutputBudget { approved } = response else {
        return Err(ArchiveError::Cancelled);
    };
    control.checkpoint()?;
    // Only the two output fields change, on a NEW request dispatched after real prior cleanup.
    let next_control = control.attempt();
    request.operation_id = next_control.operation_id();
    request.limits.max_entry_bytes = approved.entry_bytes;
    request.limits.max_output_bytes = approved.output_bytes;
    request.output_readmission = Some(ReadmissionOptions {
        logical_id,
        expected_source: Some(shortfall.source.clone()),
    });
    request
        .validate()
        .map_err(|_| ArchiveError::ResourceLimit)?;
    diagnostic(logical_id, request.operation_id, "readmitted");
    let second = run(&request, &next_control)?;
    if second.operation_id != request.operation_id || !second.cleanup_complete {
        return Err(ArchiveError::IoFailure);
    }
    Ok(second) // Actual overflow or any second failure is terminal, never another budget request.
}
fn diagnostic(logical: Uuid, operation: Uuid, phase: &'static str) {
    tracing::info!(target: "nxtools_archive", logical_id=%logical, operation_id=%operation, phase, "archive.output_readmission");
}
