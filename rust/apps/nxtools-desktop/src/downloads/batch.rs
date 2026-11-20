use super::*;
use crate::notifications::{AppNotification, NotificationAction, NotificationLevel};

pub(super) const BATCH_DESTRUCTIVE_CONFIRMATION_THRESHOLD: usize = 20;

pub(super) struct PendingDownloadBatch {
    id: uuid::Uuid,
    action: DownloadBatchAction,
    task_ids: Vec<DownloadTaskId>,
    started: Instant,
}

#[derive(Clone)]
pub(super) struct BatchConfirmation {
    pub task_ids: Vec<DownloadTaskId>,
    pub action: DownloadBatchAction,
}

fn completed_label(action: DownloadBatchAction) -> &'static str {
    match action {
        DownloadBatchAction::Pause => "已暂停",
        DownloadBatchAction::Resume => "已继续",
        DownloadBatchAction::Retry => "已重试",
        DownloadBatchAction::Redownload => "已重新下载",
        DownloadBatchAction::Cancel => "已取消",
        DownloadBatchAction::Remove | DownloadBatchAction::RemoveWithArtifact => "已移除",
    }
}

impl DownloadApplication {
    pub(crate) fn batch_busy(&self) -> bool {
        self.pending_batch.is_some() || self.pending_batch_confirmation.is_some()
    }

    pub(crate) fn batch_loading(&self, action: DownloadBatchAction) -> bool {
        self.pending_batch
            .as_ref()
            .is_some_and(|pending| pending.action == action)
    }

    pub(crate) fn batch_action(
        &mut self,
        task_ids: Vec<DownloadTaskId>,
        action: DownloadBatchAction,
        cx: &mut Context<Self>,
    ) {
        if self.batch_busy() {
            return;
        }
        let mut seen = HashSet::new();
        let task_ids: Vec<_> = task_ids.into_iter().filter(|id| seen.insert(*id)).collect();
        if task_ids.is_empty() {
            return;
        }
        if task_ids.len() >= BATCH_DESTRUCTIVE_CONFIRMATION_THRESHOLD
            && matches!(
                action,
                DownloadBatchAction::Cancel | DownloadBatchAction::RemoveWithArtifact
            )
        {
            for task_id in &task_ids {
                self.close_delete_confirmation(*task_id, cx);
            }
            let key = task_ids[0];
            self.open_delete_confirmation(
                key,
                String::new(),
                Some(BatchConfirmation { task_ids, action }),
                cx,
            );
            return;
        }
        self.start_batch(task_ids, action, cx);
    }

    pub(super) fn confirm_batch_action(
        &mut self,
        key: DownloadTaskId,
        confirmation: BatchConfirmation,
        cx: &mut Context<Self>,
    ) {
        if self.pending_batch_confirmation != Some(key) || self.pending_batch.is_some() {
            return;
        }
        self.close_delete_confirmation(key, cx);
        self.start_batch(confirmation.task_ids, confirmation.action, cx);
    }

    fn start_batch(
        &mut self,
        task_ids: Vec<DownloadTaskId>,
        action: DownloadBatchAction,
        cx: &mut Context<Self>,
    ) {
        let id = uuid::Uuid::new_v4();
        let count = task_ids.len();
        self.pending_batch = Some(PendingDownloadBatch {
            id,
            action,
            task_ids: task_ids.clone(),
            started: Instant::now(),
        });
        info!(event = "download.ui.batch.accepted", correlation_id = %id, ?action,
            target_count = count, "download batch accepted");
        cx.notify();
        let receiver = self.engine.request_batch_background(task_ids, action, id);
        match receiver {
            Ok(receiver) => {
                cx.spawn(async move |this, cx| {
                    let outcome = receiver.recv().await.unwrap_or_else(|_| {
                        DownloadCommandOutcome::Rejected {
                            fault: nxtools_download::DownloadFault {
                                code: nxtools_download::DownloadFaultCode::Internal,
                                message: "批量操作结果通道已关闭，请刷新列表后重试".into(),
                                retryable: true,
                            },
                        }
                    });
                    cx.update(|cx| {
                        cx.defer(move |cx| {
                            let _ = this.update(cx, |app, cx| app.finish_batch(id, outcome, cx));
                        })
                    });
                })
                .detach();
            }
            Err(fault) => {
                let application = cx.entity().downgrade();
                cx.defer(move |cx| {
                    let _ = application.update(cx, |app, cx| {
                        app.finish_batch(id, DownloadCommandOutcome::Rejected { fault }, cx)
                    });
                });
            }
        }
    }

    fn finish_batch(
        &mut self,
        id: uuid::Uuid,
        outcome: DownloadCommandOutcome,
        cx: &mut Context<Self>,
    ) {
        if self
            .pending_batch
            .as_ref()
            .is_none_or(|pending| pending.id != id)
        {
            return;
        }
        let pending = self.pending_batch.take().expect("matching pending batch");
        let count = pending.task_ids.len();
        let mut started_tasks = Vec::new();
        let (succeeded, failed, message) = match outcome {
            DownloadCommandOutcome::Batch { results } => {
                if self.show_progress_window
                    && matches!(
                        pending.action,
                        DownloadBatchAction::Resume | DownloadBatchAction::Retry
                    )
                {
                    started_tasks.extend(
                        results
                            .iter()
                            .filter_map(|result| result.fault.is_none().then_some(result.task_id)),
                    );
                }
                let failed: Vec<_> = results
                    .iter()
                    .filter(|result| result.fault.is_some())
                    .map(|result| result.task_id)
                    .collect();
                let message = results
                    .iter()
                    .find_map(|result| result.fault.as_ref())
                    .map_or_else(String::new, |fault| fault.message.clone());
                (results.len().saturating_sub(failed.len()), failed, message)
            }
            DownloadCommandOutcome::Rejected { fault } => {
                (0, pending.task_ids.clone(), fault.message)
            }
            _ => (
                0,
                pending.task_ids.clone(),
                "下载引擎返回了无效的批量操作结果".into(),
            ),
        };
        let failed_count = count.saturating_sub(succeeded);
        info!(event = "download.ui.batch.completed", correlation_id = %id,
            action = ?pending.action, target_count = count, failed_count,
            elapsed_ms = pending.started.elapsed().as_millis(), "download batch completed");
        self.refresh(cx);
        for task_id in started_tasks {
            self.present_explicit_task_start(task_id, count > 1, cx);
        }
        let title = if failed_count == 0 {
            format!("{} {succeeded} 项", completed_label(pending.action))
        } else {
            format!(
                "{} {succeeded} 项，{failed_count} 项失败",
                completed_label(pending.action)
            )
        };
        let mut notice = AppNotification::new(
            format!("download-batch-{id}"),
            if failed_count == 0 {
                NotificationLevel::Success
            } else {
                NotificationLevel::Error
            },
            title,
            message,
        );
        if !failed.is_empty() {
            let application = cx.entity().downgrade();
            notice = notice.actions(vec![NotificationAction::new(
                "仅选中失败项",
                move |_, cx| {
                    let application = application.clone();
                    let task_ids = failed.clone();
                    cx.defer(move |cx| {
                        let _ = application.update(cx, |_, cx| {
                            cx.emit(DownloadApplicationEvent::SelectBatchFailures { task_ids })
                        });
                    });
                },
            )]);
        }
        let notifications = self.notifications.clone();
        cx.defer(move |cx| notifications.update(cx, |center, cx| center.push(notice, cx)));
        cx.notify();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::{TestAppContext, VisualTestContext};

    #[gpui_kit::test]
    fn batch_feedback_whole_failure_keeps_every_original_target_selectable(
        cx: &mut TestAppContext,
    ) {
        let (application, _engine, ids, _directory) =
            super::super::detail_request_tests::fixture(cx);
        let selected = Rc::new(RefCell::new(Vec::new()));
        let observed = selected.clone();
        let _observer = cx.new(|cx| {
            cx.subscribe(&application, move |_: &mut (), _, event, _| {
                if let DownloadApplicationEvent::SelectBatchFailures { task_ids } = event {
                    *observed.borrow_mut() = task_ids.clone();
                }
            })
            .detach();
        });
        application.update(cx, |app, cx| app.request_task_window(ids[0], cx));
        cx.run_until_parked();
        let handle = application.read_with(cx, |app, _| app.task_windows[&ids[0]].window);
        let mut surface = VisualTestContext::from_window(handle, cx);
        for outcome in [
            DownloadCommandOutcome::Rejected {
                fault: nxtools_download::DownloadFault {
                    code: nxtools_download::DownloadFaultCode::Internal,
                    message: "批量操作结果通道已关闭，请刷新列表后重试".into(),
                    retryable: true,
                },
            },
            DownloadCommandOutcome::Accepted { task_id: None },
        ] {
            let id = uuid::Uuid::new_v4();
            application.update(&mut surface, |app, cx| {
                app.pending_batch = Some(PendingDownloadBatch {
                    id,
                    action: DownloadBatchAction::Remove,
                    task_ids: ids.clone(),
                    started: Instant::now(),
                });
                app.finish_batch(id, outcome, cx);
            });
            surface.run_until_parked();
            let action = application.read_with(&surface, |app, cx| {
                assert!(!app.batch_busy());
                let notice = app
                    .notifications
                    .read(cx)
                    .items()
                    .iter()
                    .find(|notice| notice.id == format!("download-batch-{id}"))
                    .expect("whole-batch failure notification");
                assert_eq!(notice.title, format!("已移除 0 项，{} 项失败", ids.len()));
                notice
                    .action
                    .clone()
                    .expect("all failed targets remain selectable")
            });
            selected.borrow_mut().clear();
            surface.update(|window, cx| action.invoke(window, cx));
            surface.run_until_parked();
            assert_eq!(*selected.borrow(), ids);
        }
    }
}
