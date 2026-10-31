//! Private synchronous output rendezvous; exactly one native call is outstanding.
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
use zeroize::Zeroize;

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
}
struct State {
    pending: Option<Uuid>,
    reply: Option<NativeReply>,
    cancelled: bool,
}
struct Shared {
    state: Mutex<State>,
    ready: Condvar,
}
#[derive(Clone)]
pub(crate) struct ReplyPort(Arc<Shared>);
impl ReplyPort {
    pub fn new() -> Self {
        Self(Arc::new(Shared {
            state: Mutex::new(State {
                pending: None,
                reply: None,
                cancelled: false,
            }),
            ready: Condvar::new(),
        }))
    }
    pub fn deliver(&self, reply: NativeReply) -> bool {
        let Ok(mut state) = self.0.state.lock() else {
            return false;
        };
        if state.pending != Some(reply.call_id) || state.reply.is_some() || state.cancelled {
            return false;
        }
        state.reply = Some(reply);
        self.0.ready.notify_all();
        true
    }
    fn install(&self, id: Uuid) -> Result<()> {
        let mut state = self.0.state.lock().map_err(|_| ArchiveError::IoFailure)?;
        if state.cancelled {
            return Err(ArchiveError::Cancelled);
        }
        if state.pending.is_some() {
            return Err(ArchiveError::Busy);
        }
        state.pending = Some(id);
        state.reply = None;
        Ok(())
    }
    fn retire(&self) {
        if let Ok(mut state) = self.0.state.lock() {
            state.pending = None;
            state.reply = None;
        }
    }
    fn wait(&self, control: &OperationControl) -> Result<NativeReply> {
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
            if state.cancelled {
                state.pending = None;
                state.reply = None;
                drop(state);
                control.observe_cancelled(crate::control::CancellationBoundary::NativeReply);
                return Err(ArchiveError::Cancelled);
            }
            if let Some(reply) = state.reply.take() {
                state.pending = None;
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
pub(crate) struct PrivateOutput<'a, W: Write> {
    writer: &'a mut W,
    operation_id: Uuid,
    control: &'a OperationControl,
    replies: ReplyPort,
    metadata_enabled: bool,
    progress_bytes: u64,
    progress_files: u64,
    pub(crate) conflict: Option<crate::worker_conflict::ExtractionConflict<'a>>,
    pub(crate) risk: Option<crate::zip_risk::ZipRisk<'a>>,
    plan_paths: Option<std::collections::HashMap<String, String>>,
    plan_directories: Option<std::collections::HashSet<String>>,
    plan_prefix: String,
    // Presentation only; unlike plan_prefix this never changes a native operation path.
    display_prefix: Option<String>,
}
impl<'a, W: Write> PrivateOutput<'a, W> {
    pub fn new(writer: &'a mut W, control: &'a OperationControl, replies: ReplyPort) -> Self {
        Self {
            writer,
            operation_id: control.operation_id(),
            control,
            replies,
            metadata_enabled: false,
            progress_bytes: 0,
            progress_files: 0,
            conflict: None,
            risk: None,
            plan_paths: None,
            plan_directories: None,
            plan_prefix: String::new(),
            display_prefix: None,
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
            return Ok(self.planned(Some(plan)));
        }
        Ok(self)
    }
    #[cfg(feature = "isolated-codecs")]
    pub(crate) fn planned(mut self, plan: Option<&crate::planning::ExtractionPlan>) -> Self {
        if let Some(plan) = plan {
            let prefix = plan
                .subfolder()
                .map(|s| format!("{s}/"))
                .unwrap_or_default();
            self.plan_paths = Some(
                plan.files()
                    .iter()
                    .map(|file| {
                        (
                            file.relative_path().to_owned(),
                            format!("{prefix}{}", file.relative_path()),
                        )
                    })
                    .collect(),
            );
            self.plan_prefix = prefix;
            self.plan_directories = Some(plan.directories().iter().cloned().collect());
        }
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
        let mut frame = WorkerFrame::state(self.operation_id, "extraction_summary");
        frame.extraction_summary = Some(crate::worker_conflict::ExtractionSummary {
            files_skipped: skipped,
            all_members_written: all_members_written && skipped == 0,
        });
        write_frame(self.writer, &frame).map_err(|_| ArchiveError::IoFailure)
    }
    pub(crate) fn conflict_prompt(
        &mut self,
        prompt: &crate::worker_conflict::ConflictPrompt,
    ) -> Result<()> {
        let mut frame = WorkerFrame::state(self.operation_id, "conflict_prompt");
        frame.conflict_prompt = Some(prompt.clone());
        write_frame(self.writer, &frame).map_err(|_| ArchiveError::IoFailure)
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
            write_frame(self.writer, &frame).map_err(|_| ArchiveError::IoFailure)?;
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
        self.control.checkpoint()?;
        if data.len() > 65536 {
            return Err(ArchiveError::ResourceLimit);
        }
        let call_id = Uuid::new_v4();
        self.replies.install(call_id)?;
        let mut frame = WorkerFrame::state(self.operation_id, "native_call");
        frame.native = Some(NativeCall {
            call_id,
            action: action.into(),
            path: path.map(str::to_owned),
            token,
            data: data.to_vec(),
            offset: read.map(|value| value.0),
            length: read.map(|value| value.1),
            metadata,
        });
        if write_frame(self.writer, &frame).is_err() {
            self.replies.retire();
            return Err(ArchiveError::IoFailure);
        }
        let reply = self.replies.wait(self.control)?;
        if !reply.success {
            return Err(match reply.error_code.as_deref() {
                Some("source_changed") => ArchiveError::SourceChanged,
                Some("target_exists" | "source_output_collision") => ArchiveError::OutputCollision,
                Some(
                    "reparse_component" | "invalid_component" | "invalid_root" | "invalid_source",
                ) => ArchiveError::UnsafePath,
                Some("write_limit" | "session_entry_limit" | "read_limit") => {
                    ArchiveError::ResourceLimit
                }
                _ => ArchiveError::IoFailure,
            });
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
impl<W: Write + Send> ArchiveOutput for PrivateOutput<'_, W> {
    fn confirm_zip_entry(
        &mut self,
        entry: &crate::ArchiveEntry,
        limits: &crate::ResourceLimits,
    ) -> Result<()> {
        let Some(risk) = &self.risk else {
            return if crate::zip_risk::required(entry) {
                Err(ArchiveError::ResourceLimit)
            } else {
                Ok(())
            };
        };
        risk.confirm(entry, limits, self.control, self.writer)
    }
    fn ensure_directory(&mut self, path: &str) -> Result<()> {
        if self
            .plan_directories
            .as_ref()
            .is_some_and(|paths| !paths.contains(path))
        {
            return Ok(());
        }
        let mapped = self.plan_prefix.as_str();
        let path = format!("{mapped}{path}");
        self.call("directory", Some(&path), None, &[]).map(|_| ())
    }
    fn begin_file(&mut self, path: &str) -> Result<Uuid> {
        self.call("begin", Some(path), None, &[])?
            .token
            .ok_or(ArchiveError::IoFailure)
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
        let path = if let Some(paths) = &self.plan_paths {
            let Some(mapped) = paths.get(path) else {
                return Ok(None);
            };
            mapped.clone()
        } else {
            path.to_owned()
        };
        let Some(mut conflict) = self.conflict.take() else {
            return self.begin_file(&path).map(Some);
        };
        let result = conflict.begin(self, &path, &member_path, incoming);
        self.conflict = Some(conflict);
        result
    }
    fn skipped_files(&self) -> u64 {
        self.conflict.as_ref().map_or(0, |value| value.skipped)
    }
    fn write_file(&mut self, token: Uuid, data: &[u8]) -> Result<()> {
        self.progress_bytes = self.progress_bytes.saturating_add(data.len() as u64);
        self.control.publish(crate::ArchiveProgress {
            bytes: self.progress_bytes,
            files: self.progress_files,
            total_bytes: None,
        });
        self.call("write", None, Some(token), data).map(|_| ())
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
        if self
            .plan_directories
            .as_ref()
            .is_some_and(|paths| !paths.contains(relative))
        {
            return Ok(());
        }
        let mapped = format!("{}{relative}", self.plan_prefix);
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
        self.call("commit", None, Some(token), &[])?;
        self.progress_files += 1;
        self.control.publish(crate::ArchiveProgress {
            bytes: self.progress_bytes,
            files: self.progress_files,
            total_bytes: None,
        });
        Ok(())
    }
    fn abort_file(&mut self, token: Uuid) {
        let _ = self.call("abort", None, Some(token), &[]);
    }
}
