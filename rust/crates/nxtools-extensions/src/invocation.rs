use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;
use uuid::Uuid;

pub const MAX_FRAME_BYTES: usize = 256 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InvocationIdentity {
    pub run_id: Uuid,
    pub correlation_id: Uuid,
    pub generation: u64,
    pub package_id: String,
    pub handler_id: String,
    pub content_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderItem {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub subtitle: String,
    pub action: ExtensionEffect,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum ExtensionEffect {
    Open {
        target: String,
    },
    Copy {
        text: String,
    },
    Program {
        executable: String,
        #[serde(default)]
        arguments: Vec<String>,
        #[serde(default)]
        working_directory: String,
    },
    Query {
        text: String,
    },
    Invoke {
        action_id: String,
    },
}

impl ProviderItem {
    fn validate(&self) -> bool {
        if !crate::valid_id(&self.id)
            || !crate::valid_text(&self.title, 256)
            || self.subtitle.len() > 4096
        {
            return false;
        }
        match &self.action {
            ExtensionEffect::Open { target } => crate::valid_text(target, 32_768),
            ExtensionEffect::Copy { text } | ExtensionEffect::Query { text } => {
                text.len() <= 32_768 && !text.contains('\0')
            }
            ExtensionEffect::Program {
                executable,
                arguments,
                working_directory,
            } => {
                crate::valid_text(executable, 32_768)
                    && arguments.len() <= 128
                    && arguments
                        .iter()
                        .all(|arg| arg.len() <= 32_768 && !arg.contains('\0'))
                    && working_directory.len() <= 32_768
                    && !working_directory.contains('\0')
            }
            ExtensionEffect::Invoke { action_id } => crate::valid_id(action_id),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InvocationNotice {
    Completed {
        status: String,
        value: Value,
    },
    HostCall {
        call_id: String,
        method: String,
        params: Value,
    },
    ResultBatch {
        sequence: u64,
        replace: bool,
        items: Vec<ProviderItem>,
    },
}

#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum InvocationError {
    #[error("invocation retired")]
    Retired,
    #[error("invocation identity is stale")]
    Stale,
    #[error("invocation frame has wrong run")]
    WrongRun,
    #[error("invocation frame is invalid")]
    InvalidFrame,
    #[error("host call limit reached")]
    HostCallLimit,
    #[error("duplicate host call")]
    DuplicateCall,
    #[error("unknown host call")]
    UnknownCall,
    #[error("process exited before protocol completion")]
    MissingCompletion,
    #[error("process cleanup failed")]
    ProcessFailure,
    #[error("result sequence or quota is invalid")]
    ResultLimit,
}

#[derive(Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
enum ScriptFrame {
    Complete {
        protocol_version: u16,
        run_id: Uuid,
        status: String,
        value: Value,
    },
    HostCall {
        protocol_version: u16,
        run_id: Uuid,
        call_id: String,
        method: String,
        params: Value,
    },
    ResultBatch {
        protocol_version: u16,
        run_id: Uuid,
        sequence: u64,
        #[serde(default)]
        replace: bool,
        items: Vec<ProviderItem>,
    },
}

/// One pending run. Native exit is distinct from protocol completion and tree cleanup.
pub struct Invocation {
    identity: InvocationIdentity,
    retired: bool,
    completed: bool,
    exited: bool,
    seen_calls: HashSet<String>,
    pending_calls: HashSet<String>,
    result_ids: HashSet<String>,
    sequence: u64,
    stdout_bytes: usize,
}

impl Invocation {
    pub fn new(identity: InvocationIdentity) -> Self {
        Self {
            identity,
            retired: false,
            completed: false,
            exited: false,
            seen_calls: HashSet::new(),
            pending_calls: HashSet::new(),
            result_ids: HashSet::new(),
            sequence: 0,
            stdout_bytes: 0,
        }
    }

    fn check_current(&self, current: &InvocationIdentity) -> Result<(), InvocationError> {
        if self.retired || self.completed {
            return Err(InvocationError::Retired);
        }
        if current != &self.identity {
            return Err(InvocationError::Stale);
        }
        Ok(())
    }

    pub fn accept(
        &mut self,
        bytes: &[u8],
        current: &InvocationIdentity,
    ) -> Result<InvocationNotice, InvocationError> {
        self.check_current(current)?;
        let outcome = self.accept_frame(bytes);
        if outcome.is_err() {
            self.cancel();
        }
        outcome
    }

    fn accept_frame(&mut self, bytes: &[u8]) -> Result<InvocationNotice, InvocationError> {
        if bytes.len() > MAX_FRAME_BYTES || bytes.contains(&b'\n') || bytes.contains(&b'\r') {
            return Err(InvocationError::InvalidFrame);
        }
        self.stdout_bytes = self
            .stdout_bytes
            .checked_add(bytes.len() + 1)
            .ok_or(InvocationError::InvalidFrame)?;
        if self.stdout_bytes > 4 * 1024 * 1024 {
            return Err(InvocationError::InvalidFrame);
        }
        let crate::strict_json::StrictValue(value) =
            serde_json::from_slice(bytes).map_err(|_| InvocationError::InvalidFrame)?;
        let frame: ScriptFrame =
            serde_json::from_value(value).map_err(|_| InvocationError::InvalidFrame)?;
        let (version, run_id) = match &frame {
            ScriptFrame::Complete {
                protocol_version,
                run_id,
                ..
            }
            | ScriptFrame::HostCall {
                protocol_version,
                run_id,
                ..
            }
            | ScriptFrame::ResultBatch {
                protocol_version,
                run_id,
                ..
            } => (*protocol_version, *run_id),
        };
        if version != 1 {
            return Err(InvocationError::InvalidFrame);
        }
        if run_id != self.identity.run_id {
            return Err(InvocationError::WrongRun);
        }
        match frame {
            ScriptFrame::Complete { status, value, .. } => {
                if !matches!(
                    status.as_str(),
                    "success" | "handled" | "not_handled" | "error"
                ) || !self.pending_calls.is_empty()
                {
                    return Err(InvocationError::InvalidFrame);
                }
                self.completed = true;
                Ok(InvocationNotice::Completed { status, value })
            }
            ScriptFrame::HostCall {
                call_id,
                method,
                params,
                ..
            } => {
                if !crate::valid_id(&call_id)
                    || !crate::valid_text(&method, 128)
                    || !params.is_object()
                {
                    return Err(InvocationError::InvalidFrame);
                }
                if self.seen_calls.contains(&call_id) {
                    return Err(InvocationError::DuplicateCall);
                }
                if self.pending_calls.len() >= 8 || self.seen_calls.len() >= 64 {
                    return Err(InvocationError::HostCallLimit);
                }
                self.seen_calls.insert(call_id.clone());
                self.pending_calls.insert(call_id.clone());
                Ok(InvocationNotice::HostCall {
                    call_id,
                    method,
                    params,
                })
            }
            ScriptFrame::ResultBatch {
                sequence,
                replace,
                items,
                ..
            } => {
                if sequence != self.sequence + 1
                    || items.len() > 200
                    || items.iter().any(|item| !item.validate())
                {
                    return Err(InvocationError::ResultLimit);
                }
                let mut batch_ids = HashSet::new();
                if items.iter().any(|item| !batch_ids.insert(item.id.clone())) {
                    return Err(InvocationError::ResultLimit);
                }
                let mut result_ids = if replace {
                    HashSet::new()
                } else {
                    self.result_ids.clone()
                };
                result_ids.extend(batch_ids);
                if result_ids.len() > 200 {
                    return Err(InvocationError::ResultLimit);
                }
                self.result_ids = result_ids;
                self.sequence = sequence;
                Ok(InvocationNotice::ResultBatch {
                    sequence,
                    replace,
                    items,
                })
            }
        }
    }

    pub fn cancel(&mut self) {
        self.retired = true;
        self.pending_calls.clear();
    }

    pub fn process_exit(
        &mut self,
        success: bool,
        cleanup_complete: bool,
    ) -> Result<(), InvocationError> {
        if self.exited {
            return Err(InvocationError::Retired);
        }
        self.exited = true;
        if !success || !cleanup_complete {
            self.cancel();
            return Err(InvocationError::ProcessFailure);
        }
        if self.retired {
            return Err(InvocationError::Retired);
        }
        if !self.completed {
            self.cancel();
            return Err(InvocationError::MissingCompletion);
        }
        Ok(())
    }

    pub fn host_result(
        &mut self,
        call_id: &str,
        success: bool,
        value: Value,
        current: &InvocationIdentity,
    ) -> Result<String, InvocationError> {
        self.check_current(current)?;
        if !self.pending_calls.contains(call_id) {
            return Err(InvocationError::UnknownCall);
        }
        let result = serde_json::to_string(&serde_json::json!({"protocol_version":1,"op":"host_result","run_id":self.identity.run_id,"call_id":call_id,"success":success,"value":value})).map_err(|_| InvocationError::InvalidFrame)?;
        if result.len() + 1 > MAX_FRAME_BYTES {
            return Err(InvocationError::InvalidFrame);
        }
        self.pending_calls.remove(call_id);
        Ok(result)
    }

    pub fn invoke_frame(
        &self,
        feature: &str,
        context: Value,
        settings: Value,
        causality: Value,
    ) -> Result<String, InvocationError> {
        self.check_current(&self.identity)?;
        let frame = serde_json::to_string(&serde_json::json!({"protocol_version":1,"op":"invoke","run_id":self.identity.run_id,"correlation_id":self.identity.correlation_id,"generation":self.identity.generation,"package_id":self.identity.package_id,"handler_id":self.identity.handler_id,"content_hash":self.identity.content_hash,"feature":feature,"context":context,"settings":settings,"causality":causality})).map_err(|_| InvocationError::InvalidFrame)?;
        if frame.len() + 1 > MAX_FRAME_BYTES {
            return Err(InvocationError::InvalidFrame);
        }
        Ok(frame)
    }
}
