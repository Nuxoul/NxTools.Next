//! Original finite password coordination; extract retries only a no-output preparation test.
use crate::{
    ArchiveError, OperationControl, Result,
    interaction::{InteractionPort, Prompt, PromptResponse, PromptTicket, SecretPassword},
    worker::{WorkerFrame, WorkerRequest, write_frame},
    worker_metadata::MetadataCredit,
    worker_output::ReplyPort,
};
use serde::{Deserialize, Serialize};
use std::io::Write;
use uuid::Uuid;
use zeroize::Zeroize;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PasswordReply {
    pub ticket: PromptTicket,
    pub password: Option<String>,
}
impl PasswordReply {
    pub fn response(mut self) -> Result<PromptResponse> {
        if self
            .password
            .as_ref()
            .is_some_and(|value| value.len() > 4096 || value.contains('\0'))
        {
            return Err(ArchiveError::UnsupportedOperation);
        }
        Ok(self
            .password
            .take()
            .map_or(PromptResponse::Cancel, |value| {
                PromptResponse::Password(SecretPassword::from_owned(value))
            }))
    }
}
impl Drop for PasswordReply {
    fn drop(&mut self) {
        if let Some(value) = &mut self.password {
            value.zeroize();
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PasswordState {
    pub state: String,
    pub attempt_id: Uuid,
    pub attempt: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ticket: Option<PromptTicket>,
}
fn emit<W: Write>(
    output: &mut W,
    operation: Uuid,
    state: &str,
    attempt: u32,
    attempt_id: Uuid,
    error: Option<ArchiveError>,
    ticket: Option<PromptTicket>,
) -> Result<()> {
    let mut frame = WorkerFrame::state(operation, "password_state");
    frame.password_state = Some(PasswordState {
        state: state.into(),
        attempt_id,
        attempt,
        error_code: error.map(|e| format!("{e:?}")),
        ticket,
    });
    write_frame(output, &frame).map_err(|_| ArchiveError::IoFailure)
}
pub(crate) fn execute<W: Write + Send>(
    request: &mut WorkerRequest,
    control: &OperationControl,
    output: &mut W,
    replies: ReplyPort,
    metadata: &MetadataCredit,
    interaction: &InteractionPort,
) -> Result<(u64, u64)> {
    // One original source acquisition/Job. Only the password changes; all request intent stays.
    let mut password = request.password.take().map(SecretPassword::from_owned);
    let extracting = matches!(request.operation.as_str(), "extract" | "modify_zip");
    if extracting {
        use std::io::Read as _;
        let mut signature = [0; 8];
        let count = std::fs::File::open(&request.path)?.read(&mut signature)?;
        if !(count >= 4 && matches!(&signature[..4], b"PK\x03\x04" | b"PK\x05\x06")
            || count >= 6 && &signature[..6] == b"7z\xbc\xaf\x27\x1c"
            || count >= 7 && &signature[..7] == b"Rar!\x1a\x07\0"
            || count == 8 && &signature == b"Rar!\x1a\x07\x01\0")
        {
            return Err(ArchiveError::UnsupportedOperation);
        }
    }
    // A fresh read-only request cannot carry conflict/output/delete authority into preparation.
    let mut preparation = extracting.then(|| {
        let mut read = WorkerRequest::open(request.operation_id, request.path.clone());
        read.version = request.version;
        read.operation = "test".into();
        read.codepage = request.codepage;
        read.limits = request.limits;
        read.job_execution = request.job_execution;
        read.volume_paths = request.volume_paths.clone();
        if request.output_readmission.is_some() {
            let original = crate::ResourceLimits::default();
            read.limits.max_entry_bytes = original.max_entry_bytes;
            read.limits.max_output_bytes = original.max_output_bytes;
        }
        read
    });
    for number in 1..=3 {
        control.checkpoint()?;
        let attempt = control.attempt();
        emit(
            output,
            request.operation_id,
            "running",
            number,
            attempt.operation_id(),
            None,
            None,
        )?;
        let read = preparation.as_mut().unwrap_or(&mut *request);
        read.password = password.as_ref().map(|value| value.expose().to_owned());
        let result = super::worker::execute_attempt(
            read,
            if read.browser_authorize.is_some()
                || read.read_contract.is_some()
                || read.job_execution == Some(true)
            {
                control
            } else {
                &attempt
            },
            output,
            replies.clone(),
            metadata,
            interaction,
        );
        if let Some(mut value) = read.password.take() {
            value.zeroize();
        }
        let error = result.as_ref().err().copied();
        emit(
            output,
            request.operation_id,
            "attempt_finished",
            number,
            attempt.operation_id(),
            error,
            None,
        )?;
        match result {
            Ok(_) if extracting => {
                control.checkpoint()?;
                emit(
                    output,
                    request.operation_id,
                    if request.operation == "modify_zip" {
                        "modifying"
                    } else {
                        "extracting"
                    },
                    number,
                    control.operation_id(),
                    None,
                    None,
                )?;
                request.password = password.as_ref().map(|value| value.expose().to_owned());
                // Native output MUST retain the runner's original ID, never a preparation ID.
                let extracted = super::worker::execute_attempt(
                    request,
                    control,
                    output,
                    replies,
                    metadata,
                    interaction,
                );
                if let Some(mut value) = request.password.take() {
                    value.zeroize();
                }
                return extracted; // No extract error, cancellation or publication can restart preparation.
            }
            Err(error @ (ArchiveError::PasswordRequired | ArchiveError::WrongPassword))
                if number < 3 =>
            {
                drop(password.take());
                let response = interaction.request_notified(
                    Prompt::Password {
                        retry: error == ArchiveError::WrongPassword,
                    },
                    control,
                    |pending| {
                        emit(
                            output,
                            request.operation_id,
                            "awaiting_password",
                            number,
                            attempt.operation_id(),
                            Some(error),
                            Some(pending.ticket),
                        )
                    },
                )?;
                let PromptResponse::Password(value) = response else {
                    return Err(ArchiveError::Cancelled);
                };
                password = Some(value);
            }
            other => return other,
        }
    }
    unreachable!("finite password attempts return on third result")
}
