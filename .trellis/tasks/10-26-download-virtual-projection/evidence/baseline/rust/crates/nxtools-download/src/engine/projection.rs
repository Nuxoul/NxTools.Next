//! Read-only full/compact snapshots and selected-task detail projection.
//! Filtering and ordering never change queue membership or task state.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::Instant;

use tracing::{info, warn};

use super::{DownloadEngine, DownloadEngineError, DownloadEngineTask, EngineTaskState};
use crate::{
    DownloadAggregateMetrics, DownloadQuery, DownloadRequest, DownloadSnapshot,
    DownloadStorageError, DownloadTask, DownloadTaskDetail, DownloadTaskId, DownloadTaskSummary,
};

impl DownloadTaskSummary {
    pub fn into_engine_task(self) -> Result<DownloadEngineTask, crate::DownloadTaskInvariantError> {
        let request = DownloadRequest {
            source_url: self.source_url,
            save_directory: PathBuf::from("."),
            save_directory_explicit: false,
            file_name: self.file_name,
            headers: Vec::new(),
            cookie: None,
            referrer: None,
            certificate_policy: Default::default(),
            connection_limit: Some(1),
            speed_limit_bytes_per_second: None,
            expected_total_bytes: self.total_bytes,
            scheduled_at_unix_ms: None,
            adaptive_segment_policy: None,
            media: None,
            category_id: self.category_id,
            category_name: self.category_name,
            duplicate_strategy: crate::DownloadDuplicateStrategy::UseDefault,
        };
        let mut task = DownloadTask::restore(
            self.task_id,
            request,
            self.origin,
            self.status,
            self.created_at_unix_ms,
            self.updated_at_unix_ms,
            self.completed_at_unix_ms,
        )?;
        task.set_transfer_statistics(self.transfer_statistics);
        Ok(DownloadEngineTask {
            task,
            probe: None,
            downloaded_bytes: self.downloaded_bytes,
            total_bytes: self.total_bytes,
            error: self.error,
            final_path: self.final_path,
            temporary_path: None,
            queue_eligible: self.queue_eligible,
            has_final_artifact: self.has_final_artifact,
            scheduled: self.scheduled,
        })
    }
}

impl DownloadEngine {
    pub fn task(
        &self,
        task_id: DownloadTaskId,
    ) -> Result<Option<DownloadEngineTask>, DownloadEngineError> {
        let mut snapshot = self
            .inner
            .lock_tasks()?
            .get(&task_id)
            .map(snapshot_from_state);
        if let Some(snapshot) = snapshot.as_mut() {
            snapshot.queue_eligible = match self.inner.store.is_queue_task_eligible(task_id) {
                Ok(eligible) => eligible,
                Err(DownloadStorageError::TaskNotFound(_)) => false,
                Err(error) => return Err(error.into()),
            };
        }
        Ok(snapshot)
    }

    /// Stable read-only keyset page. Retains at most33 candidate references and does not
    /// clone the entire task catalog. Newer inserts are excluded after the first key.
    pub fn tasks_page_after(
        &self,
        before: Option<(i64, DownloadTaskId)>,
        statuses: &[crate::DownloadStatus],
        limit: usize,
    ) -> Result<Vec<DownloadEngineTask>, DownloadEngineError> {
        let limit = limit.clamp(1, 33);
        let tasks = self.inner.lock_tasks()?;
        let after = before.map(|(time, id)| (std::cmp::Reverse(time), *id.as_uuid()));
        let mut selected: Vec<&EngineTaskState> = Vec::with_capacity(limit);
        for state in tasks.values() {
            if !statuses.is_empty() && !statuses.contains(&state.task.status()) {
                continue;
            }
            let key = (
                std::cmp::Reverse(state.task.created_at_unix_ms()),
                *state.task.id().as_uuid(),
            );
            if after.is_some_and(|after| key <= after) {
                continue;
            }
            let index = selected.partition_point(|item| {
                (
                    std::cmp::Reverse(item.task.created_at_unix_ms()),
                    *item.task.id().as_uuid(),
                ) < key
            });
            if index < limit {
                selected.insert(index, state);
                if selected.len() > limit {
                    selected.pop();
                }
            }
        }
        Ok(selected.into_iter().map(snapshot_from_state).collect())
    }

    pub fn tasks(&self) -> Result<Vec<DownloadEngineTask>, DownloadEngineError> {
        let queue_order = self.inner.store.load_queue_order("default")?;
        let queue_positions = queue_order
            .into_iter()
            .enumerate()
            .map(|(position, task_id)| (task_id, position))
            .collect::<HashMap<_, _>>();
        let mut eligible = HashSet::new();
        for queue in self.inner.store.load_queues()? {
            eligible.extend(self.inner.store.load_eligible_queue_order(&queue.id)?);
        }
        let mut snapshots = self
            .inner
            .lock_tasks()?
            .values()
            .map(snapshot_from_state)
            .collect::<Vec<_>>();
        for snapshot in &mut snapshots {
            snapshot.queue_eligible = eligible.contains(&snapshot.task.id());
        }
        snapshots.sort_by_key(|snapshot| {
            (
                queue_positions
                    .get(&snapshot.task.id())
                    .copied()
                    .unwrap_or(usize::MAX),
                std::cmp::Reverse(snapshot.task.created_at_unix_ms()),
            )
        });
        Ok(snapshots)
    }

    pub(super) fn command_snapshot(
        &self,
        query: DownloadQuery,
    ) -> Result<DownloadSnapshot, DownloadEngineError> {
        if query.compact {
            return self.command_compact_snapshot(query);
        }
        let search = query
            .search
            .as_deref()
            .map(str::trim)
            .filter(|search| !search.is_empty())
            .map(str::to_lowercase);
        let limit = query.limit.unwrap_or(u32::MAX) as usize;
        let task_details = self
            .tasks()?
            .into_iter()
            .filter(|snapshot| {
                let request = snapshot.task.request();
                (query.statuses.is_empty() || query.statuses.contains(&snapshot.task.status()))
                    && search.as_ref().is_none_or(|search| {
                        matches_task_search(
                            search,
                            &request.source_url,
                            request.file_name.as_deref(),
                            request.category_name.as_deref(),
                            snapshot.final_path.as_deref(),
                        )
                    })
            })
            .take(limit)
            .collect::<Vec<_>>();
        let tasks = task_details
            .iter()
            .map(|snapshot| snapshot.task.clone())
            .collect();
        Ok(DownloadSnapshot {
            generation: self.inner.session_generation,
            sequence: self.inner.event_sequencer.current(),
            tasks,
            queues: self.inner.store.load_queues()?,
            aggregate_metrics: DownloadAggregateMetrics::default(),
            task_details,
            task_summaries: Vec::new(),
        })
    }

    fn command_compact_snapshot(
        &self,
        query: DownloadQuery,
    ) -> Result<DownloadSnapshot, DownloadEngineError> {
        let queues = self.inner.store.load_queues()?;
        let mut eligible = HashSet::new();
        let mut queue_positions = HashMap::new();
        for queue in &queues {
            if queue.id == "default" {
                for (position, task_id) in self
                    .inner
                    .store
                    .load_queue_order(&queue.id)?
                    .into_iter()
                    .enumerate()
                {
                    queue_positions.entry(task_id).or_insert(position);
                }
            }
            eligible.extend(self.inner.store.load_eligible_queue_order(&queue.id)?);
        }
        let search = query
            .search
            .as_deref()
            .map(str::trim)
            .filter(|search| !search.is_empty())
            .map(str::to_lowercase);
        let limit = query.limit.unwrap_or(u32::MAX) as usize;
        let mut summaries = self
            .inner
            .lock_tasks()?
            .values()
            .map(|state| {
                let request = state.task.request();
                let file_name = request
                    .file_name
                    .clone()
                    .or_else(|| state.transfer.probe.as_ref()?.file_name.clone())
                    .or_else(|| {
                        state
                            .transfer
                            .final_path
                            .as_ref()?
                            .file_name()?
                            .to_str()
                            .map(str::to_owned)
                    });
                DownloadTaskSummary {
                    task_id: state.task.id(),
                    origin: state.task.origin(),
                    status: state.task.status(),
                    created_at_unix_ms: state.task.created_at_unix_ms(),
                    updated_at_unix_ms: state.task.updated_at_unix_ms(),
                    completed_at_unix_ms: state.task.completed_at_unix_ms(),
                    transfer_statistics: state.task.transfer_statistics(),
                    downloaded_bytes: state.transfer.downloaded_bytes,
                    total_bytes: state
                        .transfer
                        .probe
                        .as_ref()
                        .and_then(|probe| probe.total_bytes)
                        .or(request.expected_total_bytes),
                    file_name,
                    final_path: state.transfer.final_path.clone(),
                    source_url: request.source_url.clone(),
                    category_name: request.category_name.clone(),
                    category_id: request.category_id.clone(),
                    error: state.transfer.error.clone(),
                    queue_eligible: eligible.contains(&state.task.id()),
                    has_final_artifact: state.transfer.final_committed
                        && state.transfer.final_path.is_some(),
                    scheduled: request.scheduled_at_unix_ms.is_some(),
                }
            })
            .filter(|summary| {
                (query.statuses.is_empty() || query.statuses.contains(&summary.status))
                    && search.as_ref().is_none_or(|search| {
                        matches_task_search(
                            search,
                            &summary.source_url,
                            summary.file_name.as_deref(),
                            summary.category_name.as_deref(),
                            summary.final_path.as_deref(),
                        )
                    })
            })
            .collect::<Vec<_>>();
        summaries.sort_by_key(|summary| {
            (
                queue_positions
                    .get(&summary.task_id)
                    .copied()
                    .unwrap_or(usize::MAX),
                std::cmp::Reverse(summary.created_at_unix_ms),
            )
        });
        summaries.truncate(limit);
        Ok(DownloadSnapshot {
            generation: self.inner.session_generation,
            sequence: self.inner.event_sequencer.current(),
            tasks: Vec::new(),
            queues,
            aggregate_metrics: DownloadAggregateMetrics::default(),
            task_details: Vec::new(),
            task_summaries: summaries,
        })
    }

    pub(super) fn command_task_detail(
        &self,
        task_id: DownloadTaskId,
        correlation_id: uuid::Uuid,
    ) -> Result<DownloadTaskDetail, DownloadEngineError> {
        let started_at = Instant::now();
        info!(
            event = "download.detail.requested",
            %correlation_id,
            task_id = %task_id.as_uuid(),
            "download task detail requested"
        );
        let result: Result<DownloadTaskDetail, DownloadEngineError> = (|| {
            let task = self
                .task(task_id)?
                .ok_or(DownloadEngineError::TaskNotFound(task_id))?;
            let segments = self
                .inner
                .store
                .load_transfer_plan(task_id)?
                .map(|plan| plan.segments().to_vec())
                .unwrap_or_default();
            let timeline = self.inner.store.load_task_timeline(task_id, 300)?;
            Ok(DownloadTaskDetail {
                task,
                segments,
                timeline,
            })
        })();
        match result {
            Ok(detail) => {
                info!(
                    event = "download.detail.loaded",
                    %correlation_id,
                    task_id = %task_id.as_uuid(),
                    segment_count = detail.segments.len(),
                    timeline_count = detail.timeline.len(),
                    elapsed_ms = started_at.elapsed().as_millis(),
                    "download task detail loaded"
                );
                Ok(detail)
            }
            Err(error) => {
                let fault = error.fault();
                warn!(
                    event = "download.detail.failed",
                    %correlation_id,
                    task_id = %task_id.as_uuid(),
                    fault_code = ?fault.code,
                    retryable = fault.retryable,
                    elapsed_ms = started_at.elapsed().as_millis(),
                    "download task detail failed"
                );
                Err(error)
            }
        }
    }
}

pub(super) fn snapshot_from_state(state: &EngineTaskState) -> DownloadEngineTask {
    let file_name = state.task.request().file_name.as_deref().or_else(|| {
        state
            .transfer
            .probe
            .as_ref()
            .and_then(|probe| probe.file_name.as_deref())
    });
    DownloadEngineTask {
        task: state.task.clone(),
        probe: state.transfer.probe.clone(),
        downloaded_bytes: state.transfer.downloaded_bytes,
        total_bytes: state
            .transfer
            .probe
            .as_ref()
            .and_then(|probe| probe.total_bytes)
            .or(state.task.request().expected_total_bytes),
        error: state.transfer.error.clone(),
        final_path: state
            .transfer
            .final_path
            .clone()
            .or_else(|| file_name.map(|name| state.task.request().save_directory.join(name))),
        temporary_path: state.transfer.temporary_path.clone(),
        queue_eligible: false,
        has_final_artifact: state.transfer.final_committed && state.transfer.final_path.is_some(),
        scheduled: state.task.request().scheduled_at_unix_ms.is_some(),
    }
}

fn matches_task_search(
    search: &str,
    source_url: &str,
    file_name: Option<&str>,
    category_name: Option<&str>,
    final_path: Option<&Path>,
) -> bool {
    source_url.to_lowercase().contains(search)
        || file_name.is_some_and(|name| name.to_lowercase().contains(search))
        || category_name.is_some_and(|name| name.to_lowercase().contains(search))
        || final_path.is_some_and(|path| path.to_string_lossy().to_lowercase().contains(search))
}
