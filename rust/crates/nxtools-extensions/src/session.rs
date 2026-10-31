//! Version2 session/request framing over the original invocation validator.
//! Native process/cleanup remains broker-owned; a request finish is not native exit.
use crate::{
    Invocation, InvocationError, InvocationIdentity, InvocationNotice, invocation::MAX_FRAME_BYTES,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use uuid::Uuid;

pub(crate) const INPUT_CONTROL_BYTES: usize = 1024 * 1024;
pub(crate) const OUTPUT_CONTROL_BYTES: usize = 4 * 1024 * 1024;

pub(crate) fn control_limits() -> Value {
    json!({"frame_bytes":MAX_FRAME_BYTES,"input_control_bytes":INPUT_CONTROL_BYTES,
        "output_control_bytes":OUTPUT_CONTROL_BYTES,"host_calls_active":8,"host_calls_total":64})
}

pub(crate) fn transport_description(active_protocol_version: u16) -> Value {
    json!({"active_protocol_version":active_protocol_version,"supported_protocol_versions":[1,2],
        "mode":"once","supported_modes":["once"],"progress":active_protocol_version==2,"blobs":active_protocol_version==2,"limits":control_limits(),
        "data_limits":{"blob_bytes":crate::blobs::MAX_BLOB_BYTES,"live_run_bytes":crate::blobs::MAX_RUN_BLOB_BYTES,"global_bytes":crate::blobs::MAX_GLOBAL_BLOB_BYTES,"chunk_bytes":crate::blobs::MAX_CHUNK_BYTES,"decoded_bytes_each_direction":crate::data_plane::DATA_DECODED_BYTES,"wire_bytes_each_direction":crate::data_plane::DATA_WIRE_BYTES,"chunks":512,"transfers":4,"lease_seconds":120,"ack_timeout_ms":5000},
        "progress_limits":{"frames":256,"text_bytes":512,"coalesce_above_hz":10,"ui_max_hz":4}})
}

#[derive(Debug, PartialEq, Eq)]
pub enum SessionNotice {
    Ready,
    Invocation(InvocationNotice),
    RequestFinished,
    Data(crate::DataFrame),
    DataAck(crate::DataAck),
    Progress(crate::ProgressFrame),
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Initializing,
    Ready,
    Running,
    Completing,
    Finished,
    Retired,
}

pub struct InvocationSession {
    session_id: Uuid,
    phase: Phase,
    invocation: Invocation,
    identity: InvocationIdentity,
    input_bytes: usize,
    output_bytes: usize,
    data: crate::data_plane::DataPlane,
    progress_count: usize,
    progress_sequence: u64,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReadyFrame {
    protocol_version: u16,
    op: String,
    session_id: Uuid,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FinishedFrame {
    protocol_version: u16,
    op: String,
    session_id: Uuid,
    run_id: Uuid,
}
impl InvocationSession {
    pub fn new(identity: InvocationIdentity) -> Self {
        Self {
            session_id: Uuid::new_v4(),
            phase: Phase::Initializing,
            invocation: Invocation::new(identity.clone()),
            identity,
            input_bytes: 0,
            output_bytes: 0,
            data: Default::default(),
            progress_count: 0,
            progress_sequence: 0,
        }
    }
    pub fn session_id(&self) -> Uuid {
        self.session_id
    }
    pub fn initialize_frame(&mut self) -> Result<String, InvocationError> {
        if self.phase != Phase::Initializing || self.input_bytes != 0 {
            return Err(InvocationError::Retired);
        }
        let mut value = transport_description(2);
        let object = value.as_object_mut().ok_or(InvocationError::InvalidFrame)?;
        object.insert("protocol_version".into(), json!(2));
        object.insert("op".into(), json!("initialize"));
        object.insert("session_id".into(), json!(self.session_id));
        self.encode_input(&value)
    }
    pub fn accept(
        &mut self,
        bytes: &[u8],
        current: &InvocationIdentity,
    ) -> Result<SessionNotice, InvocationError> {
        if current != &self.identity {
            return Err(InvocationError::Stale);
        }
        if matches!(self.phase, Phase::Retired | Phase::Finished) {
            return Err(InvocationError::Retired);
        }
        let result = self.accept_inner(bytes, current);
        if result.is_err() {
            self.cancel()
        }
        result
    }
    fn accept_inner(
        &mut self,
        bytes: &[u8],
        current: &InvocationIdentity,
    ) -> Result<SessionNotice, InvocationError> {
        if bytes.len() + 1 > MAX_FRAME_BYTES || bytes.contains(&b'\n') || bytes.contains(&b'\r') {
            return Err(InvocationError::InvalidFrame);
        }
        let crate::strict_json::StrictValue(mut value) =
            serde_json::from_slice(bytes).map_err(|_| InvocationError::InvalidFrame)?;
        let object = value.as_object_mut().ok_or(InvocationError::InvalidFrame)?;
        if object.get("protocol_version").and_then(Value::as_u64) != Some(2)
            || object
                .get("session_id")
                .and_then(Value::as_str)
                .and_then(|id| Uuid::parse_str(id).ok())
                != Some(self.session_id)
        {
            return Err(InvocationError::WrongRun);
        }
        if self.phase == Phase::Running && object.get("op").and_then(Value::as_str) == Some("data")
        {
            let frame: crate::DataFrame =
                serde_json::from_value(value).map_err(|_| InvocationError::InvalidFrame)?;
            if frame.run_id != self.identity.run_id {
                return Err(InvocationError::WrongRun);
            }
            self.data.accept_data(&frame, bytes.len() + 1)?;
            return Ok(SessionNotice::Data(frame));
        }
        self.output_bytes = self
            .output_bytes
            .checked_add(bytes.len() + 1)
            .ok_or(InvocationError::InvalidFrame)?;
        if self.output_bytes > OUTPUT_CONTROL_BYTES {
            return Err(InvocationError::InvalidFrame);
        }
        if self.phase == Phase::Running
            && object.get("op").and_then(Value::as_str) == Some("progress")
        {
            if ["completed", "total"].iter().any(|key| {
                object
                    .get(*key)
                    .is_some_and(|value| value.as_u64().is_none())
            }) || object
                .get("message")
                .is_some_and(|value| value.as_str().is_none())
            {
                return Err(InvocationError::InvalidFrame);
            }
            let frame: crate::ProgressFrame =
                serde_json::from_value(value).map_err(|_| InvocationError::InvalidFrame)?;
            frame.validate()?;
            if frame.run_id != self.identity.run_id {
                return Err(InvocationError::WrongRun);
            }
            if frame.sequence <= self.progress_sequence || self.progress_count >= 256 {
                return Err(InvocationError::ResultLimit);
            }
            self.progress_count += 1;
            self.progress_sequence = frame.sequence;
            return Ok(SessionNotice::Progress(frame));
        }
        if self.phase == Phase::Running
            && object.get("op").and_then(Value::as_str) == Some("data_ack")
        {
            let frame: crate::DataAck =
                serde_json::from_value(value).map_err(|_| InvocationError::InvalidFrame)?;
            if frame.run_id != self.identity.run_id {
                return Err(InvocationError::WrongRun);
            }
            self.data.accept_ack(&frame)?;
            return Ok(SessionNotice::DataAck(frame));
        }
        if object.get("op").and_then(Value::as_str) == Some("complete") && !self.data.quiescent() {
            return Err(InvocationError::InvalidFrame);
        }
        if self.phase == Phase::Initializing {
            let ready: ReadyFrame =
                serde_json::from_value(value).map_err(|_| InvocationError::InvalidFrame)?;
            if ready.protocol_version != 2
                || ready.op != "ready"
                || ready.session_id != self.session_id
                || self.input_bytes == 0
            {
                return Err(InvocationError::InvalidFrame);
            }
            self.phase = Phase::Ready;
            return Ok(SessionNotice::Ready);
        }
        if self.phase == Phase::Completing {
            let frame: FinishedFrame =
                serde_json::from_value(value).map_err(|_| InvocationError::InvalidFrame)?;
            if frame.protocol_version != 2
                || frame.op != "request_finished"
                || frame.session_id != self.session_id
                || frame.run_id != self.identity.run_id
            {
                return Err(InvocationError::InvalidFrame);
            }
            self.phase = Phase::Finished;
            return Ok(SessionNotice::RequestFinished);
        }
        if self.phase != Phase::Running {
            return Err(InvocationError::InvalidFrame);
        }
        object.remove("session_id");
        object.insert("protocol_version".into(), json!(1));
        let normalized = serde_json::to_vec(&value).map_err(|_| InvocationError::InvalidFrame)?;
        let notice = self.invocation.accept(&normalized, current)?;
        if matches!(notice, InvocationNotice::Completed { .. }) {
            self.phase = Phase::Completing
        }
        Ok(SessionNotice::Invocation(notice))
    }
    pub fn invoke_frame(
        &mut self,
        feature: &str,
        context: Value,
        settings: Value,
        causality: Value,
    ) -> Result<String, InvocationError> {
        if self.phase != Phase::Ready {
            return Err(InvocationError::InvalidFrame);
        }
        let raw = self
            .invocation
            .invoke_frame(feature, context, settings, causality)?;
        let mut value: Value =
            serde_json::from_str(&raw).map_err(|_| InvocationError::InvalidFrame)?;
        let object = value.as_object_mut().ok_or(InvocationError::InvalidFrame)?;
        object.insert("protocol_version".into(), json!(2));
        object.insert("session_id".into(), json!(self.session_id));
        let frame = self.encode_input(&value)?;
        self.phase = Phase::Running;
        Ok(frame)
    }
    pub fn host_result(
        &mut self,
        call_id: &str,
        success: bool,
        value: Value,
        current: &InvocationIdentity,
    ) -> Result<String, InvocationError> {
        if self.phase != Phase::Running || current != &self.identity {
            return Err(InvocationError::Retired);
        }
        let raw = self
            .invocation
            .host_result(call_id, success, value, current)?;
        let mut value: Value =
            serde_json::from_str(&raw).map_err(|_| InvocationError::InvalidFrame)?;
        let object = value.as_object_mut().ok_or(InvocationError::InvalidFrame)?;
        object.insert("protocol_version".into(), json!(2));
        object.insert("session_id".into(), json!(self.session_id));
        self.encode_input(&value)
    }
    pub fn register_upload(
        &mut self,
        transfer: Uuid,
        length: usize,
    ) -> Result<(), InvocationError> {
        if self.phase != Phase::Running || length > crate::blobs::MAX_BLOB_BYTES {
            return Err(InvocationError::InvalidFrame);
        }
        self.data.upload(transfer, length)
    }
    pub fn register_download(
        &mut self,
        transfer: Uuid,
        offset: usize,
        length: usize,
    ) -> Result<(), InvocationError> {
        if self.phase != Phase::Running || length > crate::blobs::MAX_BLOB_BYTES {
            return Err(InvocationError::InvalidFrame);
        }
        self.data.download(transfer, offset, length)
    }
    pub fn data_ack_frame(
        &mut self,
        transfer: Uuid,
        sequence: u64,
        offset: usize,
    ) -> Result<String, InvocationError> {
        if self.phase != Phase::Running {
            return Err(InvocationError::Retired);
        }
        if let Err(failure) = self.data.upload_ack(transfer, sequence, offset) {
            self.cancel();
            return Err(failure);
        }
        self.encode_input(&json!({"protocol_version":2,"op":"data_ack","session_id":self.session_id,"run_id":self.identity.run_id,"transfer_id":transfer,"sequence":sequence,"offset":offset}))
    }
    pub fn data_frame(
        &mut self,
        chunk: &crate::blobs::BlobChunk,
    ) -> Result<String, InvocationError> {
        if self.phase != Phase::Running {
            return Err(InvocationError::Retired);
        }
        let frame=serde_json::to_string(&json!({"protocol_version":2,"op":"data","session_id":self.session_id,"run_id":self.identity.run_id,"transfer_id":chunk.transfer_id,"sequence":chunk.sequence,"offset":chunk.offset,"data":chunk.data})).map_err(|_|InvocationError::InvalidFrame)?;
        if frame.len() + 1 > MAX_FRAME_BYTES {
            self.cancel();
            return Err(InvocationError::InvalidFrame);
        }
        if let Err(failure) = self.data.send_data(chunk, frame.len() + 1) {
            self.cancel();
            return Err(failure);
        }
        Ok(frame)
    }
    pub fn process_exit(
        &mut self,
        success: bool,
        cleanup_complete: bool,
    ) -> Result<(), InvocationError> {
        if self.phase != Phase::Finished {
            self.cancel();
            return Err(InvocationError::MissingCompletion);
        }
        let result = self.invocation.process_exit(success, cleanup_complete);
        self.phase = Phase::Retired;
        result
    }
    pub fn cancel(&mut self) {
        self.phase = Phase::Retired;
        self.invocation.cancel()
    }
    fn encode_input(&mut self, value: &impl Serialize) -> Result<String, InvocationError> {
        let encoded = (|| {
            let frame = serde_json::to_string(value).map_err(|_| InvocationError::InvalidFrame)?;
            let total = self
                .input_bytes
                .checked_add(frame.len() + 1)
                .ok_or(InvocationError::InvalidFrame)?;
            if frame.len() + 1 > MAX_FRAME_BYTES || total > INPUT_CONTROL_BYTES {
                return Err(InvocationError::InvalidFrame);
            }
            self.input_bytes = total;
            Ok(frame)
        })();
        if encoded.is_err() {
            self.cancel();
        }
        encoded
    }
}

pub(crate) enum WireInvocation {
    Legacy(Box<Invocation>),
    V2(Box<InvocationSession>),
}
impl WireInvocation {
    pub(crate) fn new(identity: InvocationIdentity, protocol: u16) -> Self {
        if protocol == 2 {
            Self::V2(Box::new(InvocationSession::new(identity)))
        } else {
            Self::Legacy(Box::new(Invocation::new(identity)))
        }
    }
    pub(crate) fn session_id(&self) -> Option<Uuid> {
        match self {
            Self::V2(run) => Some(run.session_id()),
            Self::Legacy(_) => None,
        }
    }
    pub(crate) fn register_transfer(
        &mut self,
        transfer: crate::blob_calls::Transfer,
    ) -> Result<Option<Uuid>, InvocationError> {
        let Self::V2(run) = self else {
            return Err(InvocationError::InvalidFrame);
        };
        match transfer {
            crate::blob_calls::Transfer::Upload { id, length } => {
                run.register_upload(id, length)?;
                Ok(None)
            }
            crate::blob_calls::Transfer::Download { id, offset, length } => {
                run.register_download(id, offset, length)?;
                Ok((length > 0).then_some(id))
            }
            crate::blob_calls::Transfer::Release { upload } => {
                run.data.release_upload(upload)?;
                Ok(None)
            }
        }
    }
    pub(crate) fn data_frame(
        &mut self,
        chunk: &crate::blobs::BlobChunk,
    ) -> Result<String, InvocationError> {
        match self {
            Self::V2(run) => run.data_frame(chunk),
            Self::Legacy(_) => Err(InvocationError::InvalidFrame),
        }
    }
    pub(crate) fn data_ack_frame(
        &mut self,
        transfer: Uuid,
        sequence: u64,
        offset: usize,
    ) -> Result<String, InvocationError> {
        match self {
            Self::V2(run) => run.data_ack_frame(transfer, sequence, offset),
            Self::Legacy(_) => Err(InvocationError::InvalidFrame),
        }
    }
    pub(crate) fn initial_frame(
        &mut self,
        feature: &str,
        context: Value,
        settings: Value,
        causality: Value,
    ) -> Result<String, InvocationError> {
        match self {
            Self::Legacy(run) => run.invoke_frame(feature, context, settings, causality),
            Self::V2(run) => run.initialize_frame(),
        }
    }
    pub(crate) fn invoke_after_ready(
        &mut self,
        feature: &str,
        context: Value,
        settings: Value,
        causality: Value,
    ) -> Result<String, InvocationError> {
        match self {
            Self::Legacy(_) => Err(InvocationError::InvalidFrame),
            Self::V2(run) => run.invoke_frame(feature, context, settings, causality),
        }
    }
    pub(crate) fn accept(
        &mut self,
        bytes: &[u8],
        current: &InvocationIdentity,
    ) -> Result<SessionNotice, InvocationError> {
        match self {
            Self::Legacy(run) => run.accept(bytes, current).map(SessionNotice::Invocation),
            Self::V2(run) => run.accept(bytes, current),
        }
    }
    pub(crate) fn host_result(
        &mut self,
        id: &str,
        success: bool,
        value: Value,
        current: &InvocationIdentity,
    ) -> Result<String, InvocationError> {
        match self {
            Self::Legacy(run) => run.host_result(id, success, value, current),
            Self::V2(run) => run.host_result(id, success, value, current),
        }
    }
    pub(crate) fn process_exit(
        &mut self,
        success: bool,
        cleanup: bool,
    ) -> Result<(), InvocationError> {
        match self {
            Self::Legacy(run) => run.process_exit(success, cleanup),
            Self::V2(run) => run.process_exit(success, cleanup),
        }
    }
}
