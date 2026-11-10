//! Operation-local demand port and retained ZIP/TAR materialization.
use crate::{
    ArchiveEngine, ArchiveError, ArchiveFormat, EntryKind, OpenRequest, OperationControl,
    ResourceLimits, Result,
    worker::{NativeReadLease, WorkerFrame, WorkerRequest, write_frame},
    worker_output::{PrivateOutput, ReplyPort},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    io::Write,
    sync::{Arc, Condvar, Mutex},
};
use uuid::Uuid;

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DragMaterialization {
    pub version: u32,
    pub members: Vec<DragMember>,
}
#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DragMember {
    pub index: u32,
    pub ordinal: u64,
}
impl DragMaterialization {
    pub(crate) fn valid(&self, first: Option<u64>) -> bool {
        if self.version != 1
            || self.members.is_empty()
            || !self
                .members
                .iter()
                .any(|member| Some(member.ordinal) == first)
        {
            return false;
        }
        let mut indices = HashSet::new();
        let mut ordinals = HashSet::new();
        self.members
            .iter()
            .all(|member| indices.insert(member.index) && ordinals.insert(member.ordinal))
    }
}
#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DragDemand {
    pub request_id: Uuid,
    pub index: u32,
    pub ordinal: u64,
    pub output_limit: u64,
}
#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DragMemberReady {
    pub request_id: Uuid,
    pub index: u32,
    pub ordinal: u64,
    pub bytes: u64,
}
struct State {
    pending: Option<DragDemand>,
    active: Option<DragDemand>,
    progress_started: bool,
    ended: bool,
    seen_requests: HashSet<Uuid>,
    seen_indices: HashSet<u32>,
}
struct Shared {
    state: Mutex<State>,
    ready: Condvar,
    members: HashMap<u32, u64>,
    max_output: u64,
}
#[derive(Clone)]
pub(crate) struct DragPort(Arc<Shared>);
impl DragPort {
    pub(crate) fn new(plan: &DragMaterialization, max_output: u64) -> Self {
        Self(Arc::new(Shared {
            members: plan
                .members
                .iter()
                .map(|item| (item.index, item.ordinal))
                .collect(),
            max_output,
            state: Mutex::new(State {
                pending: None,
                active: None,
                progress_started: false,
                ended: false,
                seen_requests: HashSet::new(),
                seen_indices: HashSet::new(),
            }),
            ready: Condvar::new(),
        }))
    }
    pub(crate) fn accept(&self, demand: DragDemand) -> bool {
        let Ok(mut state) = self.0.state.lock() else {
            return false;
        };
        if state.ended
            || state.active.is_some()
            || state.pending.is_some()
            || demand.request_id.is_nil()
            || demand.output_limit == 0
            || demand.output_limit > self.0.max_output
            || self.0.members.get(&demand.index) != Some(&demand.ordinal)
            || state.seen_requests.contains(&demand.request_id)
            || state.seen_indices.contains(&demand.index)
        {
            return false;
        }
        state.seen_requests.insert(demand.request_id);
        state.seen_indices.insert(demand.index);
        state.pending = Some(demand);
        self.0.ready.notify_all();
        true
    }
    pub(crate) fn end(&self) -> bool {
        let Ok(mut state) = self.0.state.lock() else {
            return false;
        };
        if state.ended || state.pending.is_some() {
            return false;
        }
        state.ended = true;
        self.0.ready.notify_all();
        true
    }
    pub(crate) fn wait(&self, control: &OperationControl) -> Result<Option<DragDemand>> {
        let weak = Arc::downgrade(&self.0);
        let _hook = control.on_cancel(Arc::new(move || {
            if let Some(shared) = weak.upgrade()
                && let Ok(_guard) = shared.state.lock()
            {
                shared.ready.notify_all();
            }
        }))?;
        let mut state = self.0.state.lock().map_err(|_| ArchiveError::IoFailure)?;
        loop {
            // Idle has no member progress; the checkpoint only observes cancellation.
            control.checkpoint()?;
            if let Some(demand) = state.pending.take() {
                state.active = Some(demand);
                state.progress_started = false;
                return Ok(Some(demand));
            }
            if state.ended {
                return Ok(None);
            }
            state = self
                .0
                .ready
                .wait(state)
                .map_err(|_| ArchiveError::IoFailure)?;
        }
    }
    pub(crate) fn progress_started(&self) {
        if let Ok(mut state) = self.0.state.lock() {
            state.progress_started = true;
        }
    }
    pub(crate) fn progress_demand(&self) -> Option<DragDemand> {
        let state = self.0.state.lock().ok()?;
        state.progress_started.then_some(state.active).flatten()
    }
    pub(crate) fn finished(&self) {
        if let Ok(mut state) = self.0.state.lock() {
            state.active = None;
            state.progress_started = false;
        }
    }
}

/// Shared native member bridge and ready transition for isolated codecs.
#[cfg(feature = "isolated-codecs")]
pub(crate) fn run_codec<W: Write + Send>(
    request: &WorkerRequest,
    control: &OperationControl,
    output: &mut W,
    replies: ReplyPort,
    drag: &DragPort,
    mut decode: impl FnMut(&WorkerRequest, DragDemand, &mut PrivateOutput<'_, W>) -> Result<u64>,
) -> Result<(u64, u64)> {
    let original = request.limits;
    let mut request = request.clone();
    let mut files = 0u64;
    let mut bytes = 0u64;
    while let Some(demand) = drag.wait(control)? {
        if demand.output_limit > original.max_output_bytes.saturating_sub(bytes) {
            return Err(ArchiveError::ResourceLimit);
        }
        request.limits.max_output_bytes = demand.output_limit;
        request.limits.max_entry_bytes = demand.output_limit.min(original.max_entry_bytes);
        request
            .browser_entry
            .as_mut()
            .ok_or(ArchiveError::SourceChanged)?
            .ordinal = demand.ordinal;
        PrivateOutput::new(output, control, replies.clone()).call(
            "member_acquire",
            None,
            Some(demand.request_id),
            &[],
        )?;
        control.configure_extraction_progress(None, 1);
        drag.progress_started();
        let mut native = PrivateOutput::new(output, control, replies.clone())
            .extraction_metadata(request.extract_options.is_some());
        let written = decode(&request, demand, &mut native)?;
        if written > demand.output_limit {
            return Err(ArchiveError::ResourceLimit);
        }
        drop(native);
        let mut frame = WorkerFrame::state(request.operation_id, "drag_member_ready");
        frame.member_ready = Some(DragMemberReady {
            request_id: demand.request_id,
            index: demand.index,
            ordinal: demand.ordinal,
            bytes: written,
        });
        bytes = bytes
            .checked_add(written)
            .ok_or(ArchiveError::ResourceLimit)?;
        files += 1;
        drag.finished();
        write_frame(output, &frame).map_err(|_| ArchiveError::IoFailure)?;
    }
    Ok((files, bytes))
}
pub(crate) fn execute<W: Write + Send>(
    request: &mut WorkerRequest,
    control: &OperationControl,
    output: &mut W,
    replies: ReplyPort,
    drag: &DragPort,
    source_lease: Option<&NativeReadLease>,
) -> Result<(u64, u64)> {
    #[cfg(feature = "isolated-codecs")]
    if let Some(result) =
        crate::worker_codecs::try_execute_drag(request, control, output, replies.clone(), drag)
    {
        return result;
    }
    let original_limits: ResourceLimits = request.limits.into();
    let discovery_limits = if crate::worker_job::compound(&request.path).is_some() {
        crate::worker_job::spool_limits(original_limits)
    } else {
        original_limits
    };
    let engine = if request.volume_paths.len() > 1 {
        ArchiveEngine::with_limits(discovery_limits).with_zip_volumes()
    } else {
        ArchiveEngine::with_limits(discovery_limits)
    };
    let engine = if let Some(secret) = request.password.as_deref() {
        engine.with_password(secret)
    } else {
        engine
    };
    let mut open = OpenRequest::new(&request.path);
    open.encoding = request
        .encoding()
        .map_err(|_| ArchiveError::UnsupportedOperation)?;
    let (snapshot, mut retained_input) = engine.open_retained_catalog(open, control)?;
    crate::worker_browser::bind(
        request,
        snapshot.session_id(),
        &format!("{:?}", snapshot.format()),
        snapshot.comment(),
        snapshot.revision.digest,
        snapshot.revision.size,
        snapshot.revision.modified,
        output,
        control,
        replies.clone(),
    )?;
    let immutable = request
        .drag_materialization
        .as_ref()
        .ok_or(ArchiveError::UnsupportedOperation)?;
    for member in &immutable.members {
        let entry = snapshot
            .entries()
            .get(member.ordinal as usize)
            .filter(|entry| entry.id().ordinal == member.ordinal && entry.kind() == EntryKind::File)
            .ok_or(ArchiveError::SourceChanged)?;
        entry.metadata.validate()?;
        crate::path_policy::validate_relative(&entry.display_path().replace('\\', "/"))?;
    }
    let mut source = if snapshot.revision.volumes.is_empty() {
        crate::engine::open_snapshot(&snapshot, &discovery_limits, control)?
    } else {
        retained_input
            .input
            .take()
            .ok_or(ArchiveError::SourceChanged)?
    };
    crate::engine::ensure_revision(&mut source, &snapshot.revision, control)?;
    let mut zip_input = source.try_clone()?;
    let mut zip_records = if snapshot.format() == ArchiveFormat::Zip {
        Some(crate::formats::zip::ValidatedZip::new(
            &mut zip_input,
            snapshot.entries(),
            &discovery_limits,
            control,
            false,
        )?)
    } else {
        None
    };
    let tar_input = match snapshot.format() {
        ArchiveFormat::Zip => None,
        ArchiveFormat::Tar
        | ArchiveFormat::TarGz
        | ArchiveFormat::TarBz2
        | ArchiveFormat::TarXz
        | ArchiveFormat::TarZstd => Some(
            retained_input
                .input
                .take()
                .ok_or(ArchiveError::SourceChanged)?,
        ),
        ArchiveFormat::Gzip | ArchiveFormat::Bzip2 | ArchiveFormat::Xz | ArchiveFormat::Zstd => {
            None
        }
        _ => return Err(ArchiveError::UnsupportedOperation),
    };
    let records = retained_input.tar_plan.take();
    if let Some(records) = &records {
        let working = (snapshot.entries().len() as u64)
            .saturating_mul(128)
            .saturating_add(records.locator_bytes())
            .saturating_add(256 * 1024);
        if working > original_limits.max_working_bytes {
            return Err(ArchiveError::ResourceLimit);
        }
    }
    crate::engine::ensure_revision(&mut source, &snapshot.revision, control)?;
    let mut files = 0u64;
    let mut bytes = 0u64;
    while let Some(demand) = drag.wait(control)? {
        if demand.output_limit > original_limits.max_output_bytes.saturating_sub(bytes) {
            return Err(ArchiveError::ResourceLimit);
        }
        let limit = demand.output_limit.min(original_limits.max_entry_bytes);
        request.limits.max_entry_bytes = limit;
        request.limits.max_output_bytes = demand.output_limit;
        request
            .browser_entry
            .as_mut()
            .ok_or(ArchiveError::UnsupportedOperation)?
            .ordinal = demand.ordinal;
        let plan = crate::worker_job::plan(request, &snapshot, control)?;
        if plan.files().len() != 1 || plan.files()[0].id().ordinal != demand.ordinal {
            return Err(ArchiveError::SourceChanged);
        }
        let limits: ResourceLimits = request.limits.into();
        // The native owner installs and checks this member's cache lease/root on the original Run.
        PrivateOutput::new(output, control, replies.clone()).call(
            "member_acquire",
            None,
            Some(demand.request_id),
            &[],
        )?;
        control.configure_extraction_progress(plan.total_bytes(), 1);
        drag.progress_started();
        let mut native = PrivateOutput::new(output, control, replies.clone())
            .extraction_metadata(request.extract_options.is_some());
        let result = match snapshot.format() {
            ArchiveFormat::Zip => crate::formats::zip::extract_validated(
                zip_records.as_mut().ok_or(ArchiveError::SourceChanged)?,
                &snapshot,
                &plan,
                &limits,
                control,
                crate::formats::zip::ReadContext {
                    password: request.password.as_deref(),
                    source_lease,
                },
                &mut native,
            )?,
            ArchiveFormat::Tar
            | ArchiveFormat::TarGz
            | ArchiveFormat::TarBz2
            | ArchiveFormat::TarXz
            | ArchiveFormat::TarZstd => {
                let input = tar_input
                    .as_ref()
                    .ok_or(ArchiveError::SourceChanged)?
                    .try_clone()?;
                control.record_work(|work| work.tar_member_reads += 1);
                crate::formats::tar::extract_validated(
                    (input, records.as_ref().ok_or(ArchiveError::SourceChanged)?),
                    &snapshot,
                    &plan,
                    &limits,
                    control,
                    &mut source,
                    &mut native,
                )?
            }
            _ => {
                let engine = ArchiveEngine::with_limits(limits);
                engine.extract_with_source_lease(
                    crate::output::ExtractRequest::new(snapshot.clone(), plan),
                    control,
                    &mut native,
                    source_lease,
                )?
            }
        };
        if result.files_committed != 1 || result.bytes_committed > demand.output_limit {
            return Err(ArchiveError::ResourceLimit);
        }
        // Commit has acknowledged and closed the writer before readiness; no Run terminal here.
        native.source_checkpoint("source_precommit")?;
        drop(native);
        let mut frame = WorkerFrame::state(request.operation_id, "drag_member_ready");
        frame.member_ready = Some(DragMemberReady {
            request_id: demand.request_id,
            index: demand.index,
            ordinal: demand.ordinal,
            bytes: result.bytes_committed,
        });
        files += 1;
        bytes = bytes
            .checked_add(result.bytes_committed)
            .ok_or(ArchiveError::ResourceLimit)?;
        drag.finished();
        write_frame(output, &frame).map_err(|_| ArchiveError::IoFailure)?;
    }
    Ok((files, bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn retained_tar_plan_rejects_another_catalog_session() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/independent.tar.gz");
        let engine = ArchiveEngine::default();
        let (_, retained) = engine
            .open_retained_catalog(OpenRequest::new(&path), &OperationControl::default())
            .expect("first admitted operation");
        let snapshot = engine
            .open_catalog(OpenRequest::new(&path), &OperationControl::default())
            .expect("independent catalog session");
        let control = OperationControl::default();
        assert!(matches!(
            crate::formats::tar::test_validated(
                retained
                    .input
                    .as_ref()
                    .expect("verified spool")
                    .try_clone()
                    .expect("reader"),
                &snapshot,
                retained.tar_plan.as_ref().expect("ordinal plan"),
                &ResourceLimits::default(),
                &control,
            ),
            Err(ArchiveError::SourceChanged)
        ));
        assert_eq!(control.work_observation().tar_payload_seeks, 0);
    }
    #[test]
    fn unique_member_map_and_original_demand_identity() {
        let plan = DragMaterialization {
            version: 1,
            members: vec![
                DragMember {
                    index: 2,
                    ordinal: 17,
                },
                DragMember {
                    index: 0,
                    ordinal: 5,
                },
            ],
        };
        assert!(plan.valid(Some(17)));
        assert!(plan.valid(Some(5)));
        let port = DragPort::new(&plan, 100);
        let id = Uuid::new_v4();
        assert!(!port.accept(DragDemand {
            request_id: id,
            index: 2,
            ordinal: 5,
            output_limit: 10
        }));
        assert!(port.accept(DragDemand {
            request_id: id,
            index: 2,
            ordinal: 17,
            output_limit: 10
        }));
        assert!(!port.accept(DragDemand {
            request_id: Uuid::new_v4(),
            index: 0,
            ordinal: 5,
            output_limit: 10
        }));
        let control = OperationControl::default();
        assert_eq!(
            port.wait(&control)
                .expect("first demand")
                .expect("present")
                .index,
            2
        );
        port.finished();
        assert!(!port.accept(DragDemand {
            request_id: id,
            index: 0,
            ordinal: 5,
            output_limit: 10
        }));
        assert!(port.accept(DragDemand {
            request_id: Uuid::new_v4(),
            index: 0,
            ordinal: 5,
            output_limit: 100
        }));
        assert!(port.wait(&control).expect("second demand").is_some());
        port.finished();
        assert!(port.end());
        assert!(port.wait(&control).expect("end").is_none());
        assert!(!port.end());
    }
    #[test]
    fn cancellation_wakes_idle_retained_worker() {
        let plan = DragMaterialization {
            version: 1,
            members: vec![DragMember {
                index: 0,
                ordinal: 0,
            }],
        };
        let port = DragPort::new(&plan, 10);
        let control = OperationControl::default();
        let waiting = port.clone();
        let cancelled = control.clone();
        let worker = std::thread::spawn(move || waiting.wait(&cancelled));
        control.cancel();
        assert!(matches!(
            worker.join().expect("worker thread"),
            Err(ArchiveError::Cancelled)
        ));
    }
    #[test]
    fn compound_catalog_retains_one_verified_outer_spool() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/independent.tar.gz");
        let control = OperationControl::default();
        let (snapshot, retained) = ArchiveEngine::default()
            .open_retained_catalog(OpenRequest::new(path), &control)
            .expect("compound admission");
        assert_eq!(snapshot.format(), ArchiveFormat::TarGz);
        assert!(
            retained.input.is_some(),
            "retained verified anonymous spool"
        );
        assert!(retained.tar_plan.is_some(), "original ordinal plan");
        assert_eq!(control.work_observation().tar_record_scans, 1);
        assert_eq!(control.work_observation().tar_index_builds, 1);
        assert_eq!(control.work_observation().outer_decodes, 1);
    }
    #[test]
    fn retained_control_rejects_pending_end_and_out_of_budget_demand() {
        let plan = DragMaterialization {
            version: 1,
            members: vec![DragMember {
                index: 3,
                ordinal: 7,
            }],
        };
        let port = DragPort::new(&plan, 8);
        let id = Uuid::new_v4();
        assert!(!port.accept(DragDemand {
            request_id: id,
            index: 3,
            ordinal: 7,
            output_limit: 9
        }));
        assert!(!port.accept(DragDemand {
            request_id: id,
            index: 3,
            ordinal: 7,
            output_limit: 0
        }));
        assert!(port.accept(DragDemand {
            request_id: id,
            index: 3,
            ordinal: 7,
            output_limit: 8
        }));
        assert!(!port.end());
        assert!(
            port.wait(&OperationControl::default())
                .expect("pending")
                .is_some()
        );
        port.finished();
        assert!(port.end());
        assert!(!port.accept(DragDemand {
            request_id: Uuid::new_v4(),
            index: 3,
            ordinal: 7,
            output_limit: 1
        }));
    }
}
