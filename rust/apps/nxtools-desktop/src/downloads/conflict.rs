use super::*;

impl DownloadApplication {
    fn record_destination_conflict(&mut self, task_id: DownloadTaskId) {
        if self.destination_conflicts.insert(task_id) {
            info!(event = "download.ui.conflict.detected", correlation_id = %task_id.as_uuid(),
                ?task_id, "download destination conflict detected");
        }
    }

    pub(super) fn observe_destination_conflicts(&mut self, tasks: &[DownloadEngineTask]) {
        let conflicts: HashSet<_> = tasks
            .iter()
            .filter(|task| {
                task.task.status() == DownloadStatus::Failed
                    && task.error.as_ref().is_some_and(|fault| {
                        fault.code == nxtools_download::DownloadFaultCode::DestinationExists
                    })
            })
            .map(|task| task.task.id())
            .collect();
        self.destination_conflicts
            .retain(|id| conflicts.contains(id));
        for task_id in conflicts {
            self.record_destination_conflict(task_id);
        }
    }

    pub(super) fn observe_opened_destination_conflict(&mut self, task_id: DownloadTaskId) {
        if self.task(task_id).is_some_and(|task| {
            task.error.as_ref().is_some_and(|fault| {
                fault.code == nxtools_download::DownloadFaultCode::DestinationExists
            })
        }) {
            self.record_destination_conflict(task_id);
        }
    }

    pub(super) fn resolve_destination_conflict(
        &mut self,
        task_id: DownloadTaskId,
        rename: bool,
        cx: &mut Context<Self>,
    ) -> Result<(), String> {
        let correlation_id = *task_id.as_uuid();
        let started = Instant::now();
        info!(event = "download.ui.conflict.choice", %correlation_id, ?task_id,
            choice = if rename { "auto_rename" } else { "cancel" }, "download destination conflict choice");
        let outcome = self.execute_correlated(
            DownloadCommand::Task {
                task_id,
                action: if rename {
                    DownloadTaskAction::RetryWithAutoRename
                } else {
                    DownloadTaskAction::Cancel
                },
            },
            correlation_id,
        );
        let accepted = matches!(outcome, DownloadCommandOutcome::Accepted { .. });
        info!(event = "download.ui.conflict.result", %correlation_id, ?task_id, accepted,
            elapsed_ms = started.elapsed().as_millis(), "download destination conflict choice completed");
        match outcome {
            DownloadCommandOutcome::Accepted { .. } => {
                self.refresh(cx);
                Ok(())
            }
            DownloadCommandOutcome::Rejected { fault } => {
                self.action_error("无法处理同名文件冲突", fault.message, cx)
            }
            _ => self.action_error(
                "无法处理同名文件冲突",
                "下载引擎返回了无效的任务操作结果".into(),
                cx,
            ),
        }
    }
}
