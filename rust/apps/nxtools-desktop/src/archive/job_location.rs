//! Original JobRecord's completed-result action; no Shell service or configuration owner.
use super::*;
use nxtools_protocol::{
    OpenShellTargetCommand, PlatformCommand, PlatformCommandResult, archive_job::ArchiveJobOutcome,
};
impl ArchiveApplication {
    pub(in crate::archive) fn locate_job_result(&mut self, id: Uuid, cx: &mut Context<Self>) {
        let dragging = self.windows.values().any(|slot| {
            slot.view
                .read_with(cx, |view, _| view.native_drag_active())
                .unwrap_or(false)
        });
        let Some(record) = self.jobs.records.get_mut(&id) else {
            return;
        };
        let Some(result) = record.terminal.as_ref().filter(|result| {
            matches!(
                result.outcome,
                ArchiveJobOutcome::Succeeded | ArchiveJobOutcome::SucceededWithWarnings
            ) && result.cleanup_complete
        }) else {
            return;
        };
        let Some(location) = result.result_location.clone() else {
            return;
        };
        if record.location_pending.is_some() {
            return;
        }
        if dragging {
            record.auto_location_waiting = true;
            record.location_status = Some("等待原生拖出结束后定位，避免抢走目标焦点。".into());
            self.refresh_job(id, cx);
            return;
        }
        record.auto_location_waiting = false;
        let receipt = Uuid::new_v4();
        record.location_pending = Some(receipt);
        record.location_status = Some("正在定位实际结果…".into());
        record.location_deadline = Some(cx.spawn(async move |owner, cx| {
            cx.background_executor()
                .timer(std::time::Duration::from_secs(12))
                .await;
            let _ = owner.update(cx, |app, cx| {
                app.job_location_result(
                    receipt,
                    &PlatformCommandResult {
                        success: false,
                        error: Some("location_timeout".into()),
                        native_error_code: None,
                        data: None,
                    },
                    cx,
                );
            });
        }));
        tracing::info!(event="archive.result_location.started",job_id=%id,correlation_id=%receipt,"archive result location requested");
        let action = nxtools_protocol::ShellOpenAction::OpenInExplorer;
        let sent = self
            .platform
            .send_command_with_id_correlated(
                PlatformCommand::OpenShellTarget(OpenShellTargetCommand {
                    target: location.target,
                    action,
                }),
                receipt,
                id,
            )
            .is_ok();
        if !sent {
            self.job_location_result(
                receipt,
                &PlatformCommandResult {
                    success: false,
                    error: Some("not_submitted".into()),
                    native_error_code: None,
                    data: None,
                },
                cx,
            );
        }
        self.refresh_job(id, cx);
    }
    pub(super) fn job_location_result(
        &mut self,
        receipt: Uuid,
        result: &PlatformCommandResult,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(id) = self
            .jobs
            .records
            .iter()
            .find(|(_, record)| record.location_pending == Some(receipt))
            .map(|(id, _)| *id)
        else {
            return false;
        };
        let record = self
            .jobs
            .records
            .get_mut(&id)
            .expect("matched original job");
        record.location_pending = None;
        record.location_deadline = None;
        let status = match (result.success, result.data.as_deref()) {
            (true, Some("foreground_confirmed")) => "已定位实际结果并确认资源管理器聚焦。",
            (true, Some("target_confirmed_foreground_not_confirmed")) => {
                "已定位实际结果；Windows 未确认前台聚焦。可手动重试。"
            }
            _ => "未确认结果定位完成；可点击“定位结果”重试。",
        };
        record.location_status = Some(status.into());
        tracing::info!(event="archive.result_location.finished",job_id=%id,correlation_id=%receipt,success=result.success,native_error_code=?result.native_error_code,"archive result location finished");
        self.refresh_job(id, cx);
        true
    }
    pub(super) fn flush_job_locations(&mut self, cx: &mut Context<Self>) {
        let ids = self
            .jobs
            .records
            .iter()
            .filter(|(_, record)| record.auto_location_waiting)
            .map(|(id, _)| *id)
            .collect::<Vec<_>>();
        for id in ids {
            self.locate_job_result(id, cx);
        }
    }
}
