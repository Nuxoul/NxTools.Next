//! Whole-member ZIP scheduling under one native source lease and output owner.
use super::*;
use crate::{
    output::{ArchiveOutput, ExtractOutcome},
    planning::ExtractionPlan,
};
use std::sync::{Arc, Condvar, Mutex};

pub(crate) trait OutputFactory: Sync {
    fn output(&self) -> Box<dyn ArchiveOutput + '_>;
    fn fail(&self, error: ArchiveError);
}
// Stored/Deflate reserve 2MiB stack plus 2MiB for decoder, buffers and lane/control state.
const STACK_BYTES: usize = 2 * 1024 * 1024;
const LANE_BYTES: u64 = 4 * 1024 * 1024;
const ACTIVE_PRESSURE: u64 = 100 * 1024 * 1024;
fn width(selected: usize, cpus: usize, credit: u64) -> usize {
    selected
        .min(32)
        .min((cpus.min(64) / 2).max(1))
        .min((credit / LANE_BYTES) as usize)
}
#[derive(Default)]
struct State {
    next: usize,
    active: u64,
    active_bytes: u64,
    stopped: bool,
    first_error: Option<ArchiveError>,
    committed: u64,
    bytes: u64,
}
struct Group {
    state: Mutex<State>,
    ready: Condvar,
}
impl Group {
    fn fail(&self, error: ArchiveError, factory: &dyn OutputFactory) {
        let first = {
            let mut state = self
                .state
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if state.first_error.is_none()
                || state.first_error == Some(ArchiveError::Cancelled)
                    && error != ArchiveError::Cancelled
            {
                state.first_error = Some(error);
            }
            state.stopped = true;
            self.ready.notify_all();
            state.first_error.unwrap_or(error)
        };
        factory.fail(first);
    }
    fn claim(&self, plan: &ExtractionPlan, control: &OperationControl) -> Result<Option<usize>> {
        control.checkpoint()?;
        let mut state = self.state.lock().map_err(|_| ArchiveError::IoFailure)?;
        loop {
            if state.stopped || state.next == plan.files().len() {
                return Ok(None);
            }
            if state.active < 2 || state.active_bytes <= ACTIVE_PRESSURE {
                let index = state.next;
                state.next += 1;
                state.active += 1;
                state.active_bytes += plan.files()[index]
                    .size()
                    .ok_or(ArchiveError::CorruptArchive)?;
                control.record_work(|work| {
                    work.parallel_workers_high_water =
                        work.parallel_workers_high_water.max(state.active)
                });
                return Ok(Some(index));
            }
            state = self
                .ready
                .wait(state)
                .map_err(|_| ArchiveError::IoFailure)?;
        }
    }
    fn complete(&self, declared: u64, written: Option<u64>, control: &OperationControl) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        state.active -= 1;
        state.active_bytes -= declared;
        if let Some(bytes) = written {
            state.committed += 1;
            state.bytes += bytes;
            control.record_work(|work| work.parallel_members_completed += 1);
        }
        self.ready.notify_all();
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn extract(
    records: &mut ValidatedZip<'_>,
    snapshot: &ArchiveSnapshot,
    plan: &ExtractionPlan,
    limits: &ResourceLimits,
    control: &OperationControl,
    context: ReadContext<'_>,
    output: &mut dyn ArchiveOutput,
    factory: &dyn OutputFactory,
) -> Result<Option<ExtractOutcome>> {
    if !records.matches(&snapshot.entries) {
        return Err(ArchiveError::SourceChanged);
    }
    let Some(lease) = context
        .source_lease
        .filter(|lease| lease.protects(snapshot))
    else {
        return Ok(None);
    };
    let eligible = |entry: &ArchiveEntry| {
        !entry.encrypted
            && matches!(compression_method(entry), 0 | 8)
            && entry.size.is_some()
            && entry.compressed_size.is_some()
    };
    for planned in plan.files() {
        let entry = snapshot
            .entries
            .get(planned.id().ordinal as usize)
            .filter(|entry| entry.id == planned.id())
            .ok_or(ArchiveError::SourceChanged)?;
        if entry.kind != EntryKind::File || !eligible(entry) {
            return Ok(None);
        }
    }
    for id in &plan.explicit_directories {
        let entry = snapshot
            .entries
            .get(id.ordinal as usize)
            .filter(|entry| entry.id == *id)
            .ok_or(ArchiveError::SourceChanged)?;
        if !eligible(entry) {
            return Ok(None);
        }
    }
    let shared = plan
        .retained_working_bytes()
        .checked_add(records.working_bytes())
        .and_then(|bytes| bytes.checked_add(256 * 1024))
        .ok_or(ArchiveError::ResourceLimit)?;
    let workers = width(
        plan.files().len(),
        std::thread::available_parallelism().map_or(1, usize::from),
        limits.max_working_bytes.saturating_sub(shared),
    );
    if workers < 2 {
        return Ok(None);
    }
    super::prepare(records, snapshot, plan, limits, control, context, output)?;
    control.record_work(|work| work.parallel_workers_planned = workers as u64);
    let lane_limits = ResourceLimits {
        max_working_bytes: LANE_BYTES - STACK_BYTES as u64,
        ..*limits
    };
    let group = Arc::new(Group {
        state: Mutex::new(State::default()),
        ready: Condvar::new(),
    });
    let cancelled_group = group.clone();
    let _cancel = control.on_cancel(Arc::new(move || {
        let mut state = cancelled_group
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        state.stopped = true;
        cancelled_group.ready.notify_all();
    }))?;
    std::thread::scope(|scope| {
        let mut threads = Vec::with_capacity(workers);
        for _ in 0..workers {
            let reader = match records.parallel_reader(snapshot, lease, limits, control) {
                Ok(reader) => reader,
                Err(error) => {
                    group.fail(error, factory);
                    break;
                }
            };
            let group_ref = &group;
            let spawned = std::thread::Builder::new()
                .stack_size(STACK_BYTES)
                .spawn_scoped(scope, move || {
                    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        lane(
                            reader,
                            snapshot,
                            plan,
                            &lane_limits,
                            control,
                            factory,
                            group_ref,
                        )
                    }))
                    .unwrap_or(Err(ArchiveError::IoFailure));
                    if let Err(error) = result {
                        group_ref.fail(error, factory);
                    }
                });
            match spawned {
                Ok(thread) => threads.push(thread),
                Err(_) => {
                    group.fail(ArchiveError::ResourceLimit, factory);
                    break;
                }
            }
        }
        for thread in threads {
            if thread.join().is_err() {
                group.fail(ArchiveError::IoFailure, factory);
            }
        }
    });
    let state = group.state.lock().map_err(|_| ArchiveError::IoFailure)?;
    if let Some(error) = state.first_error {
        return Err(error);
    }
    control.checkpoint()?;
    Ok(Some(ExtractOutcome {
        files_committed: state.committed,
        bytes_committed: state.bytes,
        skipped_special: plan.skipped_special(),
    }))
}

#[allow(clippy::too_many_arguments)]
fn lane(
    mut reader: plan::ParallelReader<'_>,
    snapshot: &ArchiveSnapshot,
    plan: &ExtractionPlan,
    limits: &ResourceLimits,
    control: &OperationControl,
    factory: &dyn OutputFactory,
    group: &Group,
) -> Result<()> {
    let mut output = factory.output();
    let mut decoded = 0;
    while let Some(index) = group.claim(plan, control)? {
        let planned = &plan.files()[index];
        let entry = &snapshot.entries[planned.id().ordinal as usize];
        let relative = plan.subfolder().map_or_else(
            || planned.relative_path().to_owned(),
            |root| format!("{root}/{}", planned.relative_path()),
        );
        let mut active_token = None;
        let operation = (|| -> Result<Option<u64>> {
            output.confirm_zip_entry(entry, limits)?;
            let Some(token) = output.begin_extracted_file(&relative, entry.size)? else {
                return Ok(None);
            };
            active_token = Some(token);
            let bytes = reader.stream(entry.id, limits, control, &mut decoded, |chunk| {
                output.write_file(token, chunk)
            })?;
            control.checkpoint()?;
            entry.metadata.validate()?;
            output.set_file_metadata(token, entry.metadata)?;
            output.commit_file(token)?;
            Ok(Some(bytes))
        })();
        if let Err(error) = operation {
            group.fail(error, factory);
            if let Some(token) = active_token {
                output.abort_file(token);
            }
        }
        group.complete(
            entry.size.ok_or(ArchiveError::CorruptArchive)?,
            operation.as_ref().ok().copied().flatten(),
            control,
        );
        operation?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn width_is_limited_by_complete_lane_credit_cpu_selection_and_32() {
        assert_eq!(width(100, 128, u64::MAX), 32);
        assert_eq!(width(3, 128, u64::MAX), 3);
        assert_eq!(width(100, 12, u64::MAX), 6);
        assert_eq!(width(100, 128, 3 * LANE_BYTES), 3);
        assert_eq!(width(100, 128, 2 * LANE_BYTES - 1), 1);
        assert_eq!(width(100, 128, LANE_BYTES - 1), 0);
        assert_eq!(width(100, 1, u64::MAX), 1);
    }
}
