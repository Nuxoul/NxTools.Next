//! One bounded decision rendezvous per job. This is domain state, not a UI dispatcher.
use crate::{ArchiveError, OperationControl, Result};
use std::sync::{Arc, Condvar, Mutex};
use uuid::Uuid;
use zeroize::Zeroizing;

pub struct SecretPassword(Zeroizing<String>);
impl SecretPassword {
    pub fn new(value: &str) -> Self {
        Self(Zeroizing::new(value.to_owned()))
    }
    pub(crate) fn from_owned(value: String) -> Self {
        Self(Zeroizing::new(value))
    }
    pub fn expose(&self) -> &str {
        &self.0
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PromptTicket {
    pub operation_id: Uuid,
    pub prompt_id: Uuid,
    pub generation: u64,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Prompt {
    Password {
        retry: bool,
    },
    Conflict {
        existing_bytes: Option<u64>,
        incoming_bytes: Option<u64>,
    },
    OutputBudget {
        current: crate::output_readmission::OutputBudget,
        required: crate::output_readmission::OutputBudget,
    },
    Risk {
        declared_bytes: u64,
        limit_bytes: u64,
    },
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictDecision {
    Skip,
    Overwrite,
    AutoRename,
}
pub enum PromptResponse {
    Password(SecretPassword),
    Conflict {
        decision: ConflictDecision,
        apply_to_all: bool,
    },
    OutputBudget {
        approved: crate::output_readmission::OutputBudget,
    },
    Risk {
        approved_limit: u64,
    },
    Cancel,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PendingPrompt {
    pub ticket: PromptTicket,
    pub prompt: Prompt,
}
struct State {
    pending: Option<PendingPrompt>,
    response: Option<PromptResponse>,
    waiting: bool,
    cancelled: bool,
    closed: bool,
}
struct Shared {
    state: Mutex<State>,
    ready: Condvar,
}
#[derive(Clone)]
pub struct InteractionPort {
    operation_id: Uuid,
    generation: u64,
    shared: Arc<Shared>,
}
impl InteractionPort {
    pub fn new(operation_id: Uuid, generation: u64) -> Self {
        Self {
            operation_id,
            generation,
            shared: Arc::new(Shared {
                state: Mutex::new(State {
                    pending: None,
                    response: None,
                    waiting: false,
                    cancelled: false,
                    closed: false,
                }),
                ready: Condvar::new(),
            }),
        }
    }
    pub fn pending(&self) -> Option<PendingPrompt> {
        self.shared.state.lock().ok()?.pending.clone()
    }
    pub fn close(&self) {
        let mut state = self
            .shared
            .state
            .lock()
            .unwrap_or_else(|failure| failure.into_inner());
        state.closed = true;
        state.pending = None;
        state.response = None;
        self.shared.ready.notify_all();
    }
    pub fn respond(&self, ticket: PromptTicket, response: PromptResponse) -> Result<bool> {
        let mut state = self
            .shared
            .state
            .lock()
            .map_err(|_| ArchiveError::IoFailure)?;
        if state.closed || state.cancelled || state.response.is_some() {
            return Ok(false);
        }
        let Some(pending) = &state.pending else {
            return Ok(false);
        };
        if pending.ticket != ticket || !matches_response(&pending.prompt, &response) {
            return Ok(false);
        }
        state.response = Some(response);
        state.pending = None;
        self.shared.ready.notify_all();
        Ok(true)
    }
    pub fn request(&self, prompt: Prompt, control: &OperationControl) -> Result<PromptResponse> {
        self.request_notified(prompt, control, |_| Ok(()))
    }
    /// Install the original ticket before notifying an asynchronous transport.
    pub(crate) fn request_notified(
        &self,
        prompt: Prompt,
        control: &OperationControl,
        notify: impl FnOnce(&PendingPrompt) -> Result<()>,
    ) -> Result<PromptResponse> {
        if self.operation_id != control.operation_id() {
            return Err(ArchiveError::SourceChanged);
        }
        control.checkpoint()?;
        let ticket = PromptTicket {
            operation_id: self.operation_id,
            prompt_id: Uuid::new_v4(),
            generation: self.generation,
        };
        let pending = PendingPrompt { ticket, prompt };
        {
            let mut state = self
                .shared
                .state
                .lock()
                .map_err(|_| ArchiveError::IoFailure)?;
            if state.closed {
                return Err(ArchiveError::Cancelled);
            }
            if state.waiting {
                return Err(ArchiveError::Busy);
            }
            state.waiting = true;
            state.cancelled = false;
            state.response = None;
            state.pending = Some(pending.clone());
        }
        if let Err(error) = notify(&pending) {
            self.close();
            return Err(error);
        }
        let weak = Arc::downgrade(&self.shared);
        let registration = control.on_cancel(Arc::new(move || {
            if let Some(shared) = weak.upgrade()
                && let Ok(mut state) = shared.state.lock()
                && state.waiting
                && state
                    .pending
                    .as_ref()
                    .is_some_and(|pending| pending.ticket == ticket)
            {
                state.cancelled = true;
                state.pending = None;
                state.response = None;
                shared.ready.notify_all();
            }
        }));
        let _registration = match registration {
            Ok(registration) => registration,
            Err(error) => {
                self.close();
                return Err(error);
            }
        };
        let mut state = self
            .shared
            .state
            .lock()
            .map_err(|_| ArchiveError::IoFailure)?;
        loop {
            if state.cancelled || state.closed {
                state.waiting = false;
                state.pending = None;
                state.response = None;
                return Err(ArchiveError::Cancelled);
            }
            if let Some(response) = state.response.take() {
                state.waiting = false;
                return if matches!(response, PromptResponse::Cancel) {
                    Err(ArchiveError::Cancelled)
                } else {
                    Ok(response)
                };
            }
            state = self
                .shared
                .ready
                .wait(state)
                .map_err(|_| ArchiveError::IoFailure)?;
        }
    }
}
fn matches_response(prompt: &Prompt, response: &PromptResponse) -> bool {
    match (prompt, response) {
        (_, PromptResponse::Cancel)
        | (Prompt::Password { .. }, PromptResponse::Password(_))
        | (Prompt::Conflict { .. }, PromptResponse::Conflict { .. }) => true,
        (Prompt::OutputBudget { current, required }, PromptResponse::OutputBudget { approved }) => {
            approved.valid()
                && approved.covers(*current)
                && approved.covers(*required)
                && approved != current
        }
        (
            Prompt::Risk {
                declared_bytes,
                limit_bytes,
            },
            PromptResponse::Risk { approved_limit },
        ) => *approved_limit >= *declared_bytes && *approved_limit == *limit_bytes,
        _ => false,
    }
}

#[cfg(test)]
mod password_notification_tests {
    use super::*;
    #[test]
    fn installed_prompt_accepts_immediate_typed_answer_and_cancel_before_wait() {
        let control = OperationControl::default();
        let port = InteractionPort::new(control.operation_id(), 1);
        let answer = port
            .request_notified(Prompt::Password { retry: false }, &control, |pending| {
                assert_eq!(
                    port.pending().expect("installed before notify").ticket,
                    pending.ticket
                );
                assert!(port.respond(
                    pending.ticket,
                    PromptResponse::Password(SecretPassword::new("synthetic"))
                )?);
                Ok(())
            })
            .expect("immediate response");
        assert!(matches!(answer, PromptResponse::Password(_)));
        let control = OperationControl::default();
        let port = InteractionPort::new(control.operation_id(), 2);
        assert!(matches!(
            port.request_notified(Prompt::Password { retry: false }, &control, |_| {
                control.cancel();
                Ok(())
            }),
            Err(ArchiveError::Cancelled)
        ));
        assert!(port.pending().is_none());
    }
}
