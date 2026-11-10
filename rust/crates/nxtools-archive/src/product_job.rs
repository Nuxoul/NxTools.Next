//! Blocking execution policy adapter. The caller dispatches each typed attempt through the SAME
//! platform Job transport and returns only its actual exited/cleanup terminal. Run off the UI thread.
use crate::{
    ArchiveError, OperationControl, Result, interaction::InteractionPort, worker::WorkerRequest,
};
use nxtools_protocol::archive_job::*;

/// Reuse the original finite Rust coordinator; no retry policy is implemented in C# or a view.
/// Callbacks must preserve all host-only request fields, service controls on the original generation,
/// and return the actual native terminal (never a receipt/private Complete).
/// All original extraction formats may enter: only fully known planned output produces a
/// preflight shortfall; unknown streams retain one attempt with ordinary actual-byte enforcement.
/// Budget prompts are obtained/responded through `interaction.pending()/respond()` like other domain prompts.
pub fn execute_with_output_readmission(
    command: ArchiveJobCommand,
    control: &OperationControl,
    interaction: &InteractionPort,
    mut run: impl FnMut(ArchiveJobCommand, &OperationControl) -> Result<ArchiveJobTerminal>,
) -> Result<ArchiveJobTerminal> {
    command
        .validate()
        .map_err(|_| ArchiveError::UnsupportedOperation)?;
    let request = command
        .request
        .as_ref()
        .ok_or(ArchiveError::UnsupportedOperation)?;
    if command.action != ArchiveJobAction::Start
        || command.operation_id != control.operation_id()
        || request.kind != ArchiveJobKind::Extract
        || request.readmission.is_some()
        || request
            .extract
            .as_ref()
            .is_some_and(|extract| extract.reclamation.is_some())
    {
        return Err(ArchiveError::UnsupportedOperation);
    }
    let mut private = WorkerRequest::open(command.operation_id, request.paths[0].clone());
    private.version = 2;
    private.operation = "extract".into();
    private.bulk_transfer = Some(1);
    private.codepage = request.codepage;
    private.job_execution = Some(true);
    private.job_extract = request.extract.clone();
    private.limits = crate::worker::WorkerLimits {
        max_entries: request.limits.max_entries,
        max_metadata_bytes: request.limits.max_metadata_bytes,
        max_working_bytes: request.limits.max_working_bytes,
        max_entry_bytes: request.limits.max_entry_bytes,
        max_output_bytes: request.limits.max_output_bytes,
    };
    let mut terminal = None;
    let result = crate::output_readmission::execute(
        private,
        control,
        interaction,
        |attempt, attempt_control| {
            let mut next = command.clone();
            next.operation_id = attempt.operation_id;
            let request = next
                .request
                .as_mut()
                .ok_or(ArchiveError::UnsupportedOperation)?;
            request.limits.max_entry_bytes = attempt.limits.max_entry_bytes;
            request.limits.max_output_bytes = attempt.limits.max_output_bytes;
            request.readmission =
                attempt
                    .output_readmission
                    .as_ref()
                    .map(|r| ArchiveJobReadmission {
                        logical_id: r.logical_id,
                        expected_source: r.expected_source.as_ref().map(|s| {
                            ArchiveJobOutputSource {
                                native: s.native.clone(),
                                digest: s.digest,
                            }
                        }),
                    });
            let actual = run(next, attempt_control)?;
            let shortfall = actual.output_shortfall.as_ref().map(|s| {
                crate::output_readmission::OutputShortfall {
                    current: crate::output_readmission::OutputBudget {
                        entry_bytes: s.current.entry_bytes,
                        output_bytes: s.current.output_bytes,
                    },
                    required: crate::output_readmission::OutputBudget {
                        entry_bytes: s.required.entry_bytes,
                        output_bytes: s.required.output_bytes,
                    },
                    source: crate::output_readmission::SourceBinding {
                        native: s.source.native.clone(),
                        digest: s.source.digest,
                    },
                }
            });
            let result = crate::output_readmission::AttemptResult {
                operation_id: attempt.operation_id,
                cleanup_complete: actual.cleanup_complete,
                files_committed: actual.files_committed,
                error_code: actual.code.as_deref().map(|code| {
                    if code == "resource_limit" {
                        "ResourceLimit".into()
                    } else {
                        code.into()
                    }
                }),
                shortfall,
            };
            terminal = Some(actual);
            Ok(result)
        },
    )?;
    let terminal = terminal.ok_or(ArchiveError::IoFailure)?;
    if !result.cleanup_complete {
        return Err(ArchiveError::IoFailure);
    }
    Ok(terminal)
}
