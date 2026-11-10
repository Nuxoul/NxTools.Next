use super::*;
use nxtools_protocol::{IndexDriveMaintenance, IndexRebuildReason};

pub(super) struct VolumeMaintenance {
    correlation_id: String,
    started: Instant,
    state: IndexDriveMaintenance,
    running: bool,
}

impl IndexerService {
    pub(super) fn begin_snapshot_maintenance(&mut self) {
        self.status.phase = IndexPhase::VolumesReady;
        let drives = self.build_drives();
        self.queue_snapshot_maintenance(&drives, IndexRebuildReason::Requested, None);
        self.start_snapshot_maintenance(&drives);
    }

    pub(super) fn queue_snapshot_maintenance(
        &mut self,
        drives: &[IndexDriveStatus],
        reason: IndexRebuildReason,
        native_error_code: Option<u32>,
    ) {
        for drive in drives {
            let mount_key = normalized_mount_key(&drive.mount_point);
            if self.maintenance.contains_key(&mount_key) {
                continue;
            }
            let state = if self.snapshots.iter().any(|snapshot| {
                snapshot
                    .mount_point
                    .eq_ignore_ascii_case(&drive.mount_point)
            }) {
                IndexDriveMaintenance::Rebuilding { reason }
            } else {
                IndexDriveMaintenance::InitialBuilding
            };
            let operation = VolumeMaintenance {
                correlation_id: Envelope::new(Message::Pong(Pong { sent_at_unix_ms: 0 }))
                    .id
                    .to_string(),
                started: Instant::now(),
                state,
                running: false,
            };
            let state_code = match state {
                IndexDriveMaintenance::InitialBuilding => "initial_building",
                IndexDriveMaintenance::Rebuilding { .. } => "rebuilding",
                IndexDriveMaintenance::Ready => unreachable!(),
            };
            info!(event = "indexer.maintenance.accepted", correlation_id = %operation.correlation_id,
                state = state_code, reason = reason.code(), native_error_code,
                "volume snapshot maintenance accepted");
            self.maintenance.insert(mount_key, operation);
        }
        self.project_maintenance();
    }

    pub(super) fn start_snapshot_maintenance(&mut self, drives: &[IndexDriveStatus]) {
        for drive in drives {
            if let Some(operation) = self
                .maintenance
                .get_mut(&normalized_mount_key(&drive.mount_point))
                && !operation.running
            {
                operation.running = true;
                info!(event = "indexer.maintenance.started", correlation_id = %operation.correlation_id,
                    elapsed_ms = operation.started.elapsed().as_millis() as u64,
                    "volume snapshot maintenance started");
            }
        }
    }

    pub(super) fn snapshot_build_correlations(&self, drives: &[IndexDriveStatus]) -> Vec<String> {
        drives
            .iter()
            .map(|drive| {
                self.maintenance
                    .get(&normalized_mount_key(&drive.mount_point))
                    .map(|operation| operation.correlation_id.clone())
                    .expect("started volume maintenance owns a correlation ID")
            })
            .collect()
    }

    pub(super) fn snapshot_build_correlation(&self, mount_point: &str) -> Option<&str> {
        self.maintenance
            .get(&normalized_mount_key(mount_point))
            .map(|operation| operation.correlation_id.as_str())
    }

    pub(super) fn finish_snapshot_maintenance(
        &mut self,
        mount_point: &str,
        success: bool,
        native_error_code: Option<i32>,
    ) {
        if let Some(operation) = self.maintenance.remove(&normalized_mount_key(mount_point)) {
            let outcome = if success {
                "ready"
            } else if operation.running {
                "failed"
            } else {
                "cancelled"
            };
            info!(event = "indexer.maintenance.completed", correlation_id = %operation.correlation_id,
                success, outcome, native_error_code, elapsed_ms = operation.started.elapsed().as_millis() as u64,
                "volume snapshot maintenance completed");
        }
    }

    pub(super) fn project_maintenance(&mut self) {
        for drive in &mut self.status.drives {
            drive.maintenance = self
                .maintenance
                .get(&normalized_mount_key(&drive.mount_point))
                .map(|operation| operation.state)
                .or_else(|| {
                    self.snapshots
                        .iter()
                        .any(|snapshot| {
                            snapshot
                                .mount_point
                                .eq_ignore_ascii_case(&drive.mount_point)
                        })
                        .then_some(IndexDriveMaintenance::Ready)
                });
        }
    }

    pub(super) fn cancel_unstarted_maintenance(&mut self) {
        let cancelled = self
            .maintenance
            .iter()
            .filter(|(mount, operation)| !operation.running && !self.volume_is_enabled(mount))
            .map(|(mount, _)| mount.clone())
            .collect::<Vec<_>>();
        for mount in cancelled {
            self.finish_snapshot_maintenance(&mount, false, None);
        }
        self.project_maintenance();
    }
}
