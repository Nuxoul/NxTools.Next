use super::*;
use gpui_kit::component::menu::{DropdownMenu, PopupMenuItem};
use nxtools_download::{DownloadCompletionAction, DownloadCompletionIntention};

#[derive(Clone)]
pub(super) enum CompletionIntentionTarget {
    Task(DownloadTaskId),
}

pub(super) fn completion_intention_menu(
    id: &'static str,
    application: Entity<DownloadApplication>,
    target: CompletionIntentionTarget,
    intention: DownloadCompletionIntention,
) -> AnyElement {
    let label = intention.action.map_or("不执行".to_owned(), |action| {
        format!("完成后{}", completion_action_label(action))
    });
    action_button(id, label)
        .debug_selector(move || id.into())
        .outline()
        .with_size(gpui_kit::component::Size::Medium)
        .dropdown_caret(true)
        .accessibility_label("完成动作")
        .dropdown_menu(move |menu, _, _| {
            [
                None,
                Some(DownloadCompletionAction::Shutdown),
                Some(DownloadCompletionAction::Sleep),
                Some(DownloadCompletionAction::Hibernate),
                Some(DownloadCompletionAction::ExitApplication),
            ]
            .into_iter()
            .fold(menu, |menu, action| {
                let application = application.clone();
                let target = target.clone();
                menu.item(
                    PopupMenuItem::new(action.map_or("无电源或退出动作", completion_action_label))
                        .checked(intention.action == action)
                        .on_click(move |_, _, cx| {
                            let application = application.clone();
                            let target = target.clone();
                            cx.defer(move |cx| {
                                application.update(cx, |app, cx| {
                                    app.save_completion_intention(target, Some(action), None, cx)
                                })
                            });
                        }),
                )
            })
        })
        .into_any_element()
}

impl DownloadApplication {
    pub(super) fn save_completion_intention(
        &mut self,
        target: CompletionIntentionTarget,
        action: Option<Option<DownloadCompletionAction>>,
        open_directory: Option<bool>,
        cx: &mut Context<Self>,
    ) {
        let correlation_id = uuid::Uuid::new_v4();
        let started_at = Instant::now();
        let (mut intention, task_id) = match &target {
            CompletionIntentionTarget::Task(task_id) => {
                let Some(task) = self.engine.task(*task_id).ok().flatten() else {
                    return;
                };
                if matches!(
                    task.task.status(),
                    DownloadStatus::Completed | DownloadStatus::Failed | DownloadStatus::Cancelled
                ) {
                    return;
                }
                (task.task.completion_intention(), Some(*task_id))
            }
        };
        if let Some(action) = action {
            intention.action = action;
        }
        if let Some(value) = open_directory {
            intention.open_directory = value;
        }
        let command = match target {
            CompletionIntentionTarget::Task(task_id) => DownloadCommand::Task {
                task_id,
                action: DownloadTaskAction::SetCompletionIntention { intention },
            },
        };
        let result = self.engine.handle_command(DownloadCommandEnvelope {
            protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
            request_id: correlation_id,
            origin: RequestOrigin::Ui,
            command,
        });
        match result.outcome {
            DownloadCommandOutcome::Rejected { fault } => {
                let notifications = self.notifications.clone();
                cx.defer(move |cx| {
                    notifications.update(cx, |notifications, cx| {
                        notifications.show_error(
                            "download-completion-intention-failed",
                            "完成动作未保存",
                            fault.message,
                            cx,
                        )
                    })
                });
            }
            _ => {
                info!(event="download.completion_intention.accepted",%correlation_id,task_id=?task_id.map(|id|*id.as_uuid()),action=?intention.action,open_directory=intention.open_directory,elapsed_ms=started_at.elapsed().as_millis(),"completion intention persisted");
                if let Some(task_id) = task_id {
                    self.update_task_window(task_id, None, None, cx);
                }
                self.refresh(cx);
            }
        }
    }
}
