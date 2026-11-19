//! Private native controls and ordered, bounded extraction output on the original pipes.
use crate::{
    ArchiveError, OperationControl, Result,
    output::ArchiveOutput,
    worker::{WorkerFrame, write_frame},
};
use serde::{Deserialize, Serialize};
use std::{
    io::Write,
    sync::{Arc, Condvar, Mutex},
};
use uuid::Uuid;
use zeroize::{Zeroize, Zeroizing};

#[derive(Serialize, Deserialize)]
pub struct NativeCall {
    pub call_id: Uuid,
    pub action: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub token: Option<Uuid>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub data: Vec<u8>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub offset: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub length: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub sequence: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub metadata: Option<crate::metadata::OutputMetadata>,
}
impl Drop for NativeCall {
    fn drop(&mut self) {
        self.data.zeroize();
    }
}
pub(crate) struct NativeReply {
    pub call_id: Uuid,
    pub success: bool,
    pub token: Option<Uuid>,
    pub error_code: Option<String>,
    pub read_data: Option<String>,
    pub existing_bytes: Option<u64>,
    pub sequence: Option<u64>,
    pub offset: Option<u64>,
    pub length: Option<u32>,
    pub binary: Option<crate::worker_bulk::Binding>,
    pub payload: Option<Zeroizing<Vec<u8>>>,
}
struct Pending {
    bulk: Option<crate::worker_bulk::Binding>,
    reply: Option<NativeReply>,
}
struct State {
    pending: std::collections::HashMap<Uuid, Pending>,
    cancelled: bool,
    failure: Option<ArchiveError>,
    bulk: bool,
    parallel: bool,
    sequence: u64,
    written: std::collections::HashMap<Uuid, u64>,
}
struct Shared {
    state: Mutex<State>,
    ready: Condvar,
    send: Mutex<()>,
}
#[derive(Clone)]
pub(crate) struct ReplyPort(Arc<Shared>);
impl ReplyPort {
    pub fn new() -> Self {
        Self(Arc::new(Shared {
            state: Mutex::new(State {
                pending: Default::default(),
                cancelled: false,
                failure: None,
                bulk: false,
                parallel: false,
                sequence: 0,
                written: Default::default(),
            }),
            ready: Condvar::new(),
            send: Mutex::new(()),
        }))
    }
    pub fn configure_bulk(&self, enabled: bool) {
        if let Ok(mut state) = self.0.state.lock() {
            state.bulk = enabled;
        }
    }
    pub fn configure_parallel(&self, enabled: bool) {
        if let Ok(mut state) = self.0.state.lock() {
            state.parallel = enabled;
        }
    }
    pub fn bulk_enabled(&self) -> bool {
        self.0.state.lock().is_ok_and(|state| state.bulk)
    }
    pub fn fail(&self, failure: ArchiveError) {
        if let Ok(mut state) = self.0.state.lock() {
            state.failure.get_or_insert(failure);
            self.0.ready.notify_all();
        }
    }
    pub fn deliver_binary(
        &self,
        binding: crate::worker_bulk::Binding,
        data: Zeroizing<Vec<u8>>,
    ) -> bool {
        if binding.kind == crate::worker_bulk::OUTPUT {
            return false;
        }
        self.deliver(NativeReply {
            call_id: binding.call,
            success: true,
            token: Some(binding.token),
            error_code: None,
            read_data: None,
            existing_bytes: None,
            sequence: Some(binding.sequence),
            offset: Some(binding.offset),
            length: Some(binding.length),
            binary: Some(binding),
            payload: Some(data),
        })
    }
    pub fn deliver(&self, reply: NativeReply) -> bool {
        let Ok(mut state) = self.0.state.lock() else {
            return false;
        };
        if state.cancelled || state.failure.is_some() {
            return false;
        }
        let Some(pending) = state.pending.get_mut(&reply.call_id) else {
            return false;
        };
        if pending.reply.is_some() {
            return false;
        }
        if let Some(binding) = pending.bulk {
            if reply.token != Some(binding.token)
                || reply.sequence != Some(binding.sequence)
                || reply.offset != Some(binding.offset)
                || reply.length != Some(binding.length)
                || reply.read_data.is_some()
                || (reply.success
                    && binding.kind != crate::worker_bulk::OUTPUT
                    && reply.binary != Some(binding))
                || (binding.kind == crate::worker_bulk::OUTPUT && reply.payload.is_some())
                || (!reply.success && reply.payload.is_some())
            {
                return false;
            }
        } else if reply.sequence.is_some() || reply.binary.is_some() {
            return false;
        }
        pending.reply = Some(reply);
        self.0.ready.notify_all();
        true
    }
    fn install(
        &self,
        id: Uuid,
        mut bulk: Option<crate::worker_bulk::Binding>,
    ) -> Result<Option<crate::worker_bulk::Binding>> {
        let mut state = self.0.state.lock().map_err(|_| ArchiveError::IoFailure)?;
        if let Some(failure) = state.failure {
            return Err(failure);
        }
        if state.cancelled {
            return Err(ArchiveError::Cancelled);
        }
        let capacity = if state.parallel { 32 } else { 1 };
        if state.pending.len() >= capacity || state.pending.contains_key(&id) {
            return Err(ArchiveError::Busy);
        }
        if let Some(binding) = &mut bulk {
            state.sequence = state
                .sequence
                .checked_add(1)
                .ok_or(ArchiveError::ResourceLimit)?;
            binding.sequence = state.sequence;
            if binding.kind == crate::worker_bulk::OUTPUT {
                binding.offset = *state
                    .written
                    .get(&binding.token)
                    .ok_or(ArchiveError::IoFailure)?;
            }
        }
        state.pending.insert(id, Pending { bulk, reply: None });
        Ok(bulk)
    }
    fn queue_output(
        &self,
        mut binding: crate::worker_bulk::Binding,
    ) -> Result<crate::worker_bulk::Binding> {
        let mut state = self.0.state.lock().map_err(|_| ArchiveError::IoFailure)?;
        if let Some(failure) = state.failure {
            return Err(failure);
        }
        if state.cancelled {
            return Err(ArchiveError::Cancelled);
        }
        if !state.bulk || (!state.parallel && !state.pending.is_empty()) {
            return Err(ArchiveError::Busy);
        }
        binding.sequence = state
            .sequence
            .checked_add(1)
            .ok_or(ArchiveError::ResourceLimit)?;
        binding.offset = *state
            .written
            .get(&binding.token)
            .ok_or(ArchiveError::IoFailure)?;
        let next = binding
            .offset
            .checked_add(u64::from(binding.length))
            .ok_or(ArchiveError::ResourceLimit)?;
        state.sequence = binding.sequence;
        state.written.insert(binding.token, next);
        Ok(binding)
    }
    fn output_token(&self, token: Uuid, active: bool) -> Result<()> {
        if !self.bulk_enabled() {
            return Ok(());
        }
        let mut state = self.0.state.lock().map_err(|_| ArchiveError::IoFailure)?;
        if active {
            if token.is_nil()
                || state.written.len() >= 4096
                || state.written.insert(token, 0).is_some()
            {
                return Err(ArchiveError::ResourceLimit);
            }
        } else {
            state.written.remove(&token);
        }
        Ok(())
    }
    fn retire(&self, id: Uuid) {
        if let Ok(mut state) = self.0.state.lock() {
            state.pending.remove(&id);
        }
    }
    fn wait(&self, id: Uuid, control: &OperationControl) -> Result<NativeReply> {
        let weak = Arc::downgrade(&self.0);
        let _registration = control.on_cancel(Arc::new(move || {
            if let Some(shared) = weak.upgrade()
                && let Ok(mut state) = shared.state.lock()
            {
                state.cancelled = true;
                shared.ready.notify_all();
            }
        }))?;
        let mut state = self.0.state.lock().map_err(|_| ArchiveError::IoFailure)?;
        loop {
            if state
                .pending
                .get(&id)
                .and_then(|pending| pending.reply.as_ref())
                .is_some_and(|reply| !reply.success)
            {
                return state
                    .pending
                    .remove(&id)
                    .and_then(|pending| pending.reply)
                    .ok_or(ArchiveError::IoFailure);
            }
            if let Some(failure) = state.failure {
                state.pending.remove(&id);
                return Err(failure);
            }
            if state.cancelled {
                state.pending.remove(&id);
                drop(state);
                control.observe_cancelled(crate::control::CancellationBoundary::NativeReply);
                return Err(ArchiveError::Cancelled);
            }
            let pending = state.pending.get_mut(&id).ok_or(ArchiveError::IoFailure)?;
            if let Some(reply) = pending.reply.take() {
                let bulk = pending.bulk;
                if reply.success
                    && let Some(binding) = bulk
                    && binding.kind == crate::worker_bulk::OUTPUT
                {
                    let next = binding
                        .offset
                        .checked_add(u64::from(binding.length))
                        .ok_or(ArchiveError::ResourceLimit)?;
                    state.written.insert(binding.token, next);
                }
                state.pending.remove(&id);
                return Ok(reply);
            }
            state = self
                .0
                .ready
                .wait(state)
                .map_err(|_| ArchiveError::IoFailure)?;
        }
    }
}
const EXTRACT_WRITE_BYTES: usize = 32 * 1024;

pub(crate) struct PrivateOutput<'a, W: Write> {
    writer: parallel::Writer<'a, W>,
    operation_id: Uuid,
    control: &'a OperationControl,
    replies: ReplyPort,
    metadata_enabled: bool,
    progress_bytes: u64,
    current_bytes_done: u64,
    progress_files: u64,
    ipc_ack_us: u64,
    output_write_calls: u64,
    output_write_bytes: u64,
    control_calls: [u64; 4],
    control_wait_us: [u64; 4],
    extracted_token: Option<Uuid>,
    write_buffer: Vec<u8>,
    write_failure: Option<ArchiveError>,
    ordinary_extraction: bool,
    file_metadata: std::collections::HashMap<Uuid, crate::metadata::OutputMetadata>,
    pub(crate) conflict: Option<crate::worker_conflict::ExtractionConflict<'a>>,
    pub(crate) risk: Option<crate::zip_risk::ZipRisk<'a>>,
    plan_paths: Option<std::collections::HashMap<String, String>>,
    plan_directories: Option<std::collections::HashMap<String, String>>,
    plan_prefix: String,
    // Presentation only; unlike plan_prefix this never changes a native operation path.
    display_prefix: Option<String>,
    shared_decisions: Option<&'a dyn parallel::DecisionGate>,
    current_member: String,
    parallel_output: bool,
}
impl<'a, W: Write> PrivateOutput<'a, W> {
    pub fn new(writer: &'a mut W, control: &'a OperationControl, replies: ReplyPort) -> Self {
        Self::from_writer(parallel::Writer::Direct(writer), control, replies)
    }
    fn from_writer(
        writer: parallel::Writer<'a, W>,
        control: &'a OperationControl,
        replies: ReplyPort,
    ) -> Self {
        Self {
            writer,
            operation_id: control.operation_id(),
            control,
            replies,
            metadata_enabled: false,
            progress_bytes: 0,
            progress_files: 0,
            current_bytes_done: 0,
            ipc_ack_us: 0,
            output_write_calls: 0,
            output_write_bytes: 0,
            control_calls: [0; 4],
            control_wait_us: [0; 4],
            extracted_token: None,
            write_buffer: Vec::new(),
            write_failure: None,
            ordinary_extraction: false,
            file_metadata: Default::default(),
            conflict: None,
            risk: None,
            plan_paths: None,
            plan_directories: None,
            plan_prefix: String::new(),
            display_prefix: None,
            shared_decisions: None,
            current_member: String::new(),
            parallel_output: false,
        }
    }
    pub(crate) fn reserve_subfolder(
        &mut self,
        plan: &mut crate::planning::ExtractionPlan,
    ) -> Result<()> {
        let Some(stem) = plan.subfolder().map(str::to_owned) else {
            return Ok(());
        };
        for index in 1..=4096 {
            self.control.checkpoint()?;
            let candidate = if index == 1 {
                stem.clone()
            } else {
                format!("{stem} ({index})")
            };
            match self.call("directory_new", Some(&candidate), None, &[]) {
                Ok(_) => {
                    plan.replace_subfolder(candidate)?;
                    self.display_prefix = plan.subfolder().map(|name| format!("{name}/"));
                    return Ok(());
                }
                Err(ArchiveError::OutputCollision) => continue,
                Err(error) => return Err(error),
            }
        }
        Err(ArchiveError::OutputCollision)
    }
    #[cfg(feature = "isolated-codecs")]
    pub(crate) fn planned_unique(
        mut self,
        plan: Option<&mut crate::planning::ExtractionPlan>,
    ) -> Result<Self> {
        if let Some(plan) = plan {
            self.reserve_subfolder(plan)?;
            return self.planned(Some(plan));
        }
        Ok(self)
    }
    #[cfg(feature = "isolated-codecs")]
    pub(crate) fn planned(
        mut self,
        plan: Option<&crate::planning::ExtractionPlan>,
    ) -> Result<Self> {
        if let Some(plan) = plan {
            self.control
                .configure_extraction_progress(plan.total_bytes(), plan.files().len() as u64);
            let prefix = plan
                .subfolder()
                .map(|s| format!("{s}/"))
                .unwrap_or_default();
            self.plan_paths = Some(
                plan.files()
                    .iter()
                    .map(|file| {
                        Ok((
                            plan.original_file_path(file)?,
                            format!("{prefix}{}", file.relative_path()),
                        ))
                    })
                    .collect::<Result<std::collections::HashMap<_, _>>>()?,
            );
            self.plan_prefix = prefix.clone();
            self.plan_directories = Some(
                plan.directories()
                    .iter()
                    .map(|relative| (plan.original_directory_path(relative), relative.clone()))
                    .collect(),
            );
        }
        Ok(self)
    }
    pub(crate) fn ordinary_extraction(mut self, enabled: bool) -> Self {
        self.ordinary_extraction = enabled;
        self
    }
    pub(crate) fn extraction_metadata(mut self, enabled: bool) -> Self {
        self.metadata_enabled = enabled;
        self
    }
    pub(crate) fn conflicts(
        mut self,
        policy: Option<crate::worker_conflict::ConflictPolicy>,
        interaction: &'a crate::interaction::InteractionPort,
        entries: Arc<[crate::ArchiveEntry]>,
    ) -> Self {
        self.conflict = policy.map(|policy| {
            crate::worker_conflict::ExtractionConflict::new(policy, interaction, entries)
        });
        self
    }
    pub(crate) fn extraction_summary(
        &mut self,
        all_members_written: bool,
        special_skips: u64,
    ) -> Result<()> {
        let Some(conflict) = &self.conflict else {
            return Ok(());
        };
        let skipped = conflict.skipped + special_skips;
        self.member_summary(skipped, all_members_written && skipped == 0)
    }
    pub(crate) fn member_summary(&mut self, skipped: u64, all_members_written: bool) -> Result<()> {
        let mut frame = WorkerFrame::state(self.operation_id, "extraction_summary");
        frame.extraction_summary = Some(crate::worker_conflict::ExtractionSummary {
            files_skipped: skipped,
            all_members_written,
        });
        write_frame(&mut self.writer, &frame).map_err(|_| ArchiveError::IoFailure)
    }
    pub(crate) fn add_result(
        &mut self,
        result: nxtools_protocol::archive_job::ArchiveJobAddResult,
    ) -> Result<()> {
        let mut frame = WorkerFrame::state(self.operation_id, "add_result");
        frame.add_result = Some(result);
        write_frame(&mut self.writer, &frame).map_err(|_| ArchiveError::IoFailure)
    }
    pub(crate) fn conflict_prompt(
        &mut self,
        prompt: &crate::worker_conflict::ConflictPrompt,
    ) -> Result<()> {
        let mut frame = WorkerFrame::state(self.operation_id, "conflict_prompt");
        frame.conflict_prompt = Some(prompt.clone());
        write_frame(&mut self.writer, &frame).map_err(|_| ArchiveError::IoFailure)
    }
    pub(crate) fn control(&self) -> &OperationControl {
        self.control
    }
    pub(crate) fn source_identity(&mut self) -> Result<String> {
        self.call("source_stamp", None, None, &[])?
            .read_data
            .ok_or(ArchiveError::IoFailure)
    }
    pub(crate) fn source_checkpoint(&mut self, stage: &str) -> Result<()> {
        self.call(stage, None, None, &[]).map(|_| ())
    }
    pub(crate) fn edit_stage(&mut self, phase: &'static str) -> Result<()> {
        self.control.set_progress_phase(phase);
        let frame = serde_json::json!({"schema":1,"operation_id":self.operation_id,"op":"progress","progress":{"phase":phase,"bytes_done":0,"bytes_total":null,"files_done":0,"files_total":null,"current_member":null}});
        write_frame(&mut self.writer, &frame).map_err(|_| ArchiveError::IoFailure)
    }
    /// Called only after immutable source/selection/path/format preflight, before any output call.
    pub(crate) fn preflight_output(
        &mut self,
        request: &crate::worker::WorkerRequest,
        digest: [u8; 32],
        required: Option<crate::output_readmission::OutputBudget>,
    ) -> Result<()> {
        use crate::output_readmission::{OutputBudget, OutputShortfall, SourceBinding};
        let Some(options) = &request.output_readmission else {
            return Ok(());
        };
        if options
            .expected_source
            .as_ref()
            .is_some_and(|s| s.digest != digest)
        {
            return Err(ArchiveError::SourceChanged);
        }
        // The native owner rechecks its held original object; preserve its exact failure result.
        self.source_checkpoint("source_precommit")?;
        let current = OutputBudget {
            entry_bytes: request.limits.max_entry_bytes,
            output_bytes: request.limits.max_output_bytes,
        };
        let shortfall = required.filter(|required| !current.covers(*required));
        let identity = if options.expected_source.is_some() || shortfall.is_some() {
            Some(self.source_identity()?)
        } else {
            None
        };
        if options
            .expected_source
            .as_ref()
            .is_some_and(|s| identity.as_ref() != Some(&s.native))
        {
            return Err(ArchiveError::SourceChanged);
        }
        if let Some(required) = shortfall {
            let mut frame = WorkerFrame::state(self.operation_id, "output_budget_shortfall");
            frame.output_budget_shortfall = Some(OutputShortfall {
                current,
                required,
                source: SourceBinding {
                    native: identity.ok_or(ArchiveError::IoFailure)?,
                    digest,
                },
            });
            write_frame(&mut self.writer, &frame).map_err(|_| ArchiveError::IoFailure)?;
            return Err(ArchiveError::ResourceLimit);
        }
        // Unknown sizes never produce a guessed shortfall. Actual byte counters still apply.
        self.source_checkpoint("output_ready")
    }
    pub(crate) fn destination_preflight(
        &mut self,
        request: &crate::worker::WorkerRequest,
        required: Option<u64>,
    ) -> Result<()> {
        if request.job_execution == Some(true) {
            self.call_at(
                "destination_preflight",
                None,
                None,
                &[],
                required.map(|bytes| (bytes, 0)),
            )?;
        }
        Ok(())
    }
    fn discard_output(&mut self) {
        self.write_buffer.fill(0);
        self.write_buffer.clear();
    }
    fn flush_output(&mut self) -> Result<()> {
        if let Some(failure) = self.write_failure {
            return Err(failure);
        }
        if self.write_buffer.is_empty() {
            return Ok(());
        }
        let token = self.extracted_token.ok_or(ArchiveError::IoFailure)?;
        let mut bytes = std::mem::take(&mut self.write_buffer);
        let result = self
            .call_payload("write", None, Some(token), &bytes, None, None)
            .map(|_| ());
        bytes.fill(0);
        bytes.clear();
        self.write_buffer = bytes;
        if let Err(failure) = result {
            self.write_failure = Some(failure);
        }
        result
    }
    pub(crate) fn call(
        &mut self,
        action: &str,
        path: Option<&str>,
        token: Option<Uuid>,
        data: &[u8],
    ) -> Result<NativeReply> {
        self.call_at(action, path, token, data, None)
    }
    pub(crate) fn call_at(
        &mut self,
        action: &str,
        path: Option<&str>,
        token: Option<Uuid>,
        data: &[u8],
        read: Option<(u64, u32)>,
    ) -> Result<NativeReply> {
        if action == "write" {
            self.flush_output()?;
        }
        self.call_payload(action, path, token, data, read, None)
    }
    fn call_payload(
        &mut self,
        action: &str,
        path: Option<&str>,
        token: Option<Uuid>,
        data: &[u8],
        read: Option<(u64, u32)>,
        metadata: Option<crate::metadata::OutputMetadata>,
    ) -> Result<NativeReply> {
        if action == "abort" {
            // Multi-file codecs may abort an earlier staged token first. Cleanup must
            // discard the current tail and remain callable after a sticky write failure.
            self.discard_output();
            self.extracted_token = None;
        } else {
            if let Some(failure) = self.write_failure {
                return Err(failure);
            }
            if action != "write" {
                self.flush_output()?;
            }
        }
        self.control.checkpoint()?;
        if data.len() > 65536 {
            return Err(ArchiveError::ResourceLimit);
        }
        let replies = self.replies.clone();
        let sending = replies.0.send.lock().map_err(|_| ArchiveError::IoFailure)?;
        let call_id = Uuid::new_v4();
        let bulk =
            if self.replies.bulk_enabled() && matches!(action, "write" | "source_read" | "read") {
                let (kind, offset, length) = match action {
                    "write" => (crate::worker_bulk::OUTPUT, 0, data.len() as u32),
                    "source_read" => {
                        let (offset, length) = read.ok_or(ArchiveError::IoFailure)?;
                        (crate::worker_bulk::SOURCE, offset, length)
                    }
                    _ => {
                        let (offset, length) = read.ok_or(ArchiveError::IoFailure)?;
                        (crate::worker_bulk::PARTIAL, offset, length)
                    }
                };
                if !(1..=65536).contains(&length) {
                    return Err(ArchiveError::ResourceLimit);
                }
                Some(crate::worker_bulk::Binding {
                    kind,
                    operation: self.operation_id,
                    call: call_id,
                    token: token
                        .filter(|token| !token.is_nil())
                        .ok_or(ArchiveError::IoFailure)?,
                    sequence: 0,
                    offset,
                    length,
                })
            } else {
                None
            };
        if self.ordinary_extraction
            && let Some(binding) = bulk.filter(|binding| binding.kind == crate::worker_bulk::OUTPUT)
        {
            let binding = self.replies.queue_output(binding)?;
            let started = std::time::Instant::now();
            let sent = binding
                .write(&mut self.writer, data)
                .map_err(|_| ArchiveError::IoFailure);
            self.ipc_ack_us = self
                .ipc_ack_us
                .saturating_add(started.elapsed().as_micros().min(u128::from(u64::MAX)) as u64);
            self.output_write_calls += 1;
            if sent.is_ok() {
                self.output_write_bytes += data.len() as u64;
            }
            if let Err(failure) = sent {
                self.write_failure = Some(failure);
                return Err(failure);
            }
            return Ok(NativeReply {
                call_id,
                success: true,
                token,
                error_code: None,
                read_data: None,
                existing_bytes: None,
                sequence: None,
                offset: None,
                length: None,
                binary: None,
                payload: None,
            });
        }
        let bulk = self.replies.install(call_id, bulk)?;
        let started = std::time::Instant::now();
        let sent = if let Some(binding) =
            bulk.filter(|binding| binding.kind == crate::worker_bulk::OUTPUT)
        {
            binding.write(&mut self.writer, data)
        } else {
            let mut frame = WorkerFrame::state(self.operation_id, "native_call");
            frame.native = Some(NativeCall {
                call_id,
                action: action.into(),
                path: path.map(str::to_owned),
                token,
                data: data.to_vec(),
                offset: read.map(|value| value.0),
                length: read.map(|value| value.1),
                sequence: bulk.map(|binding| binding.sequence),
                metadata,
            });
            write_frame(&mut self.writer, &frame)
        };
        if sent.is_err() {
            self.replies.retire(call_id);
            return Err(ArchiveError::IoFailure);
        }
        drop(sending);
        let reply = self.replies.wait(call_id, self.control);
        let control_index = match action {
            "begin" | "begin_overwrite" => Some(0),
            "commit" => Some(1),
            "directory" | "directory_new" => Some(2),
            "directory_metadata" => Some(3),
            _ => None,
        };
        if let Some(index) = control_index {
            self.control_calls[index] = self.control_calls[index].saturating_add(1);
            self.control_wait_us[index] = self.control_wait_us[index]
                .saturating_add(started.elapsed().as_micros().min(u128::from(u64::MAX)) as u64);
        }
        if action == "write" {
            let elapsed = started.elapsed().as_micros().min(u128::from(u64::MAX)) as u64;
            self.ipc_ack_us = self.ipc_ack_us.saturating_add(elapsed);
            self.output_write_calls += 1;
            if reply.as_ref().is_ok_and(|value| value.success) {
                self.output_write_bytes += data.len() as u64;
            }
        }
        let reply = reply?;
        if action == "begin" && reply.error_code.as_deref() == Some("target_exists") {
            return Ok(reply);
        }
        if !reply.success {
            return Err(match reply.error_code.as_deref() {
                Some("cancelled") => ArchiveError::Cancelled,
                Some("source_changed") => ArchiveError::SourceChanged,
                Some("missing_volume") => ArchiveError::MissingVolume,
                Some("corrupt_archive") => ArchiveError::CorruptArchive,
                Some("unsupported_format") => ArchiveError::UnsupportedFormat,
                Some("unsupported_method") => ArchiveError::UnsupportedMethod,
                Some("unsupported_encryption") => ArchiveError::UnsupportedEncryption,
                Some(
                    "reclamation_unsupported"
                    | "reclamation_filesystem"
                    | "reclamation_source_attributes",
                ) => ArchiveError::ReclamationUnsupported,
                Some("reclamation_no_allocation") => ArchiveError::ReclamationNoAllocation,
                Some(code) if code.starts_with("reclamation_") => {
                    ArchiveError::ReclamationRecoveryInvalid
                }
                Some("target_exists" | "source_output_collision") => ArchiveError::OutputCollision,
                Some(
                    "reparse_component" | "invalid_component" | "invalid_root" | "invalid_source",
                ) => ArchiveError::UnsafePath,
                Some("selected_output_limit") => ArchiveError::SelectedOutputLimit,
                Some("resource_limit" | "write_limit" | "session_entry_limit" | "read_limit") => {
                    ArchiveError::ResourceLimit
                }
                _ => ArchiveError::IoFailure,
            });
        }
        if matches!(action, "begin" | "begin_overwrite") {
            self.replies
                .output_token(reply.token.ok_or(ArchiveError::IoFailure)?, true)?;
        } else if matches!(action, "commit" | "abort")
            && let Some(token) = token
        {
            self.replies.output_token(token, false)?;
            if self.extracted_token == Some(token) {
                self.extracted_token = None;
            }
        }
        Ok(reply)
    }
    pub(crate) fn read_partial(
        &mut self,
        token: Uuid,
        offset: u64,
        length: u32,
    ) -> Result<Vec<u8>> {
        if length == 0 || length > 65536 {
            return Err(ArchiveError::ResourceLimit);
        }
        let reply = self.call_at("read", None, Some(token), &[], Some((offset, length)))?;
        if let Some(mut payload) = reply.payload {
            return Ok(std::mem::take(&mut *payload));
        }
        let encoded = reply.read_data.ok_or(ArchiveError::IoFailure)?;
        if encoded.len() > 87384 {
            return Err(ArchiveError::ResourceLimit);
        }
        use base64::Engine as _;
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .map_err(|_| ArchiveError::IoFailure)?;
        if decoded.len() > length as usize {
            return Err(ArchiveError::ResourceLimit);
        }
        Ok(decoded)
    }
}
impl<W: Write> Drop for PrivateOutput<'_, W> {
    fn drop(&mut self) {
        self.discard_output();
        self.control.record_work(|work| {
            work.output_ipc_ack_us = work.output_ipc_ack_us.saturating_add(self.ipc_ack_us);
            work.output_write_calls = work
                .output_write_calls
                .saturating_add(self.output_write_calls);
            work.output_write_bytes = work
                .output_write_bytes
                .saturating_add(self.output_write_bytes);
            work.output_begin_calls = work
                .output_begin_calls
                .saturating_add(self.control_calls[0]);
            work.output_begin_wait_us = work
                .output_begin_wait_us
                .saturating_add(self.control_wait_us[0]);
            work.output_close_calls = work
                .output_close_calls
                .saturating_add(self.control_calls[1]);
            work.output_close_wait_us = work
                .output_close_wait_us
                .saturating_add(self.control_wait_us[1]);
            work.output_directory_calls = work
                .output_directory_calls
                .saturating_add(self.control_calls[2]);
            work.output_directory_wait_us = work
                .output_directory_wait_us
                .saturating_add(self.control_wait_us[2]);
            work.output_directory_metadata_calls = work
                .output_directory_metadata_calls
                .saturating_add(self.control_calls[3]);
            work.output_directory_metadata_wait_us = work
                .output_directory_metadata_wait_us
                .saturating_add(self.control_wait_us[3]);
        });
    }
}
impl<W: Write + Send> ArchiveOutput for PrivateOutput<'_, W> {
    fn confirm_zip_entry(
        &mut self,
        entry: &crate::ArchiveEntry,
        limits: &crate::ResourceLimits,
    ) -> Result<()> {
        if let Some(decisions) = self.shared_decisions {
            return decisions.confirm(entry, limits, self.control, &mut self.writer);
        }
        let Some(risk) = &self.risk else {
            return if crate::zip_risk::required(entry) {
                Err(ArchiveError::ResourceLimit)
            } else {
                Ok(())
            };
        };
        risk.confirm(entry, limits, self.control, &mut self.writer)
    }
    fn ensure_directory(&mut self, path: &str) -> Result<()> {
        let mapped = if let Some(paths) = &self.plan_directories {
            let Some(mapped) = paths.get(path) else {
                return Ok(());
            };
            mapped.as_str()
        } else {
            path
        };
        let path = format!("{}{mapped}", self.plan_prefix);
        self.call("directory", Some(&path), None, &[]).map(|_| ())
    }
    fn begin_file(&mut self, path: &str) -> Result<Uuid> {
        let reply = self.call("begin", Some(path), None, &[])?;
        if !reply.success {
            return Err(ArchiveError::OutputCollision);
        }
        reply.token.ok_or(ArchiveError::IoFailure)
    }
    fn begin_extracted_file(&mut self, path: &str, incoming: Option<u64>) -> Result<Option<Uuid>> {
        let member_path = if self.plan_paths.is_none() {
            self.display_prefix
                .as_deref()
                .and_then(|prefix| path.strip_prefix(prefix))
                .unwrap_or(path)
        } else {
            path
        }
        .to_owned();
        self.current_bytes_done = 0;
        self.current_member.clone_from(&member_path);
        self.control.begin_extracted_member(&member_path, incoming);
        let path = if let Some(paths) = &self.plan_paths {
            let Some(mapped) = paths.get(path) else {
                self.control.clear_extracted_member();
                return Ok(None);
            };
            mapped.clone()
        } else {
            path.to_owned()
        };
        let result = if let Some(decisions) = self.shared_decisions {
            let observed = self.call("begin", Some(&path), None, &[])?;
            if observed.success {
                observed.token.ok_or(ArchiveError::IoFailure).map(Some)
            } else {
                decisions.resolve(self, observed, &path, &member_path, incoming)
            }
        } else if let Some(mut conflict) = self.conflict.take() {
            let result = conflict.begin(self, &path, &member_path, incoming);
            self.conflict = Some(conflict);
            result
        } else {
            self.begin_file(&path).map(Some)
        };
        if matches!(result, Ok(None)) {
            self.control.clear_extracted_member_if(&self.current_member);
        }
        if let Ok(Some(token)) = result {
            // One reusable staging block fits the existing binary-transport reservation.
            self.extracted_token = self.replies.bulk_enabled().then_some(token);
        }
        result
    }
    fn skipped_files(&self) -> u64 {
        self.shared_decisions.map_or_else(
            || self.conflict.as_ref().map_or(0, |value| value.skipped),
            |decisions| decisions.skipped(),
        )
    }
    fn write_file(&mut self, token: Uuid, data: &[u8]) -> Result<()> {
        self.control.checkpoint()?;
        if data.len() > 65_536 {
            return Err(ArchiveError::ResourceLimit);
        }
        self.progress_bytes = self.progress_bytes.saturating_add(data.len() as u64);
        self.current_bytes_done = self.current_bytes_done.saturating_add(data.len() as u64);
        if self.parallel_output {
            self.control.add_parallel_progress(
                data.len() as u64,
                false,
                &self.current_member,
                self.current_bytes_done,
            )?;
        } else {
            self.control.publish_extraction_progress(
                crate::ArchiveProgress {
                    bytes: self.progress_bytes,
                    files: self.progress_files,
                    total_bytes: None,
                },
                self.current_bytes_done,
            );
        }
        if self.extracted_token != Some(token) || data.is_empty() {
            return self.call("write", None, Some(token), data).map(|_| ());
        }
        if let Some(failure) = self.write_failure {
            return Err(failure);
        }
        if self.write_buffer.capacity() == 0 {
            self.write_buffer.reserve_exact(EXTRACT_WRITE_BYTES);
        }
        let mut remaining = data;
        while !remaining.is_empty() {
            self.control.checkpoint()?;
            let count = remaining
                .len()
                .min(EXTRACT_WRITE_BYTES - self.write_buffer.len());
            self.write_buffer.extend_from_slice(&remaining[..count]);
            remaining = &remaining[count..];
            if self.write_buffer.len() == EXTRACT_WRITE_BYTES {
                self.flush_output()?;
            }
        }
        Ok(())
    }
    fn set_file_metadata(
        &mut self,
        token: Uuid,
        metadata: crate::metadata::OutputMetadata,
    ) -> Result<()> {
        if !self.metadata_enabled {
            return Ok(());
        }
        metadata.validate()?;
        if self.ordinary_extraction {
            self.file_metadata.insert(token, metadata);
            return Ok(());
        }
        self.call_payload(
            "file_metadata",
            None,
            Some(token),
            &[],
            None,
            Some(metadata),
        )
        .map(|_| ())
    }
    fn finalize_directory_metadata(
        &mut self,
        relative: &str,
        metadata: crate::metadata::OutputMetadata,
    ) -> Result<()> {
        if !self.metadata_enabled {
            return Ok(());
        }
        metadata.validate()?;
        let mapped = if let Some(paths) = &self.plan_directories {
            let Some(mapped) = paths.get(relative) else {
                return Ok(());
            };
            mapped.as_str()
        } else {
            relative
        };
        let mapped = format!("{}{mapped}", self.plan_prefix);
        self.call_payload(
            "directory_metadata",
            Some(&mapped),
            None,
            &[],
            None,
            Some(metadata),
        )
        .map(|_| ())
    }
    fn commit_file(&mut self, token: Uuid) -> Result<()> {
        let metadata = self.file_metadata.remove(&token);
        self.call_payload("commit", None, Some(token), &[], None, metadata)?;
        self.progress_files += 1;
        if self.parallel_output {
            self.control.add_parallel_progress(
                0,
                true,
                &self.current_member,
                self.current_bytes_done,
            )?;
        } else {
            self.control.publish_extraction_progress(
                crate::ArchiveProgress {
                    bytes: self.progress_bytes,
                    files: self.progress_files,
                    total_bytes: None,
                },
                self.current_bytes_done,
            );
        }
        Ok(())
    }
    fn abort_file(&mut self, token: Uuid) {
        self.file_metadata.remove(&token);
        let _ = self.call("abort", None, Some(token), &[]);
    }
}

#[path = "worker_output_parallel.rs"]
mod parallel;

#[cfg(test)]
#[path = "worker_output_bulk_tests.rs"]
mod bulk_tests;

#[cfg(test)]
#[path = "worker_output_direct_tests.rs"]
mod direct_tests;
