use super::*;
use gpui_kit::SharedString;
use gpui_kit::component::menu::{DropdownMenu, PopupMenuItem};
use nxtools_download::{
    DownloadCompletionAction, DownloadCompletionIntention, DownloadQueueAction,
};

impl DownloadSettingsView {
    pub(super) fn render_queue_completion_settings(
        &mut self,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let owner = cx.entity().downgrade();
        Form::vertical()
            .children(
                self.completion_queues
                    .iter()
                    .enumerate()
                    .map(|(index, (queue, draft))| {
                        let queue_id = queue.id.clone();
                        let selected = draft.action;
                        let menu_owner = owner.clone();
                        let menu = Button::new(SharedString::from(format!(
                            "queue-completion-action-{queue_id}"
                        )))
                        .label(selected.map_or("无电源或退出动作", completion_action_label))
                        .debug_selector(|| "download-queue-completion-action".into())
                        .outline()
                        .small()
                        .disabled(self.saving)
                        .dropdown_caret(true)
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
                                let owner = menu_owner.clone();
                                menu.item(
                                    PopupMenuItem::new(
                                        action.map_or("无电源或退出动作", completion_action_label),
                                    )
                                    .checked(selected == action)
                                    .on_click(
                                        move |_, _, cx| {
                                            let owner = owner.clone();
                                            cx.defer(move |cx| {
                                                let _ = owner.update(cx, |view, cx| {
                                                    if !view.saving
                                                        && let Some((_, draft)) =
                                                            view.completion_queues.get_mut(index)
                                                    {
                                                        draft.action = action;
                                                        cx.notify();
                                                    }
                                                });
                                            });
                                        },
                                    ),
                                )
                            })
                        });
                        let directory_owner = owner.clone();
                        let save_owner = owner.clone();
                        Field::new()
                            .label(queue.name.clone())
                            .description("动作保存在队列；关闭任务窗口不会清除。")
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .flex_wrap()
                                    .child(menu)
                                    .child(
                                        gpui_kit::component::checkbox::Checkbox::new(
                                            SharedString::from(format!(
                                                "queue-completion-directory-{queue_id}"
                                            )),
                                        )
                                        .debug_selector(|| {
                                            "download-queue-completion-directory".into()
                                        })
                                        .label("成功后打开目录")
                                        .small()
                                        .checked(draft.open_directory)
                                        .disabled(self.saving)
                                        .on_click(
                                            move |_, _, cx| {
                                                let owner = directory_owner.clone();
                                                cx.defer(move |cx| {
                                                    let _ = owner.update(cx, |view, cx| {
                                                        if !view.saving
                                                            && let Some((_, draft)) = view
                                                                .completion_queues
                                                                .get_mut(index)
                                                        {
                                                            draft.open_directory =
                                                                !draft.open_directory;
                                                            cx.notify();
                                                        }
                                                    });
                                                });
                                            },
                                        ),
                                    )
                                    .child(
                                        Button::new(SharedString::from(format!(
                                            "queue-completion-save-{queue_id}"
                                        )))
                                        .label("保存队列动作")
                                        .outline()
                                        .small()
                                        .debug_selector(|| "download-queue-completion-save".into())
                                        .disabled(
                                            self.saving || *draft == queue.completion_intention,
                                        )
                                        .on_click(
                                            move |_, _, cx| {
                                                let owner = save_owner.clone();
                                                cx.defer(move |cx| {
                                                    let _ = owner.update(cx, |view, cx| {
                                                        view.save_queue_completion(index, cx)
                                                    });
                                                });
                                            },
                                        ),
                                    ),
                            )
                    }),
            )
            .into_any_element()
    }

    fn save_queue_completion(&mut self, index: usize, cx: &mut Context<Self>) {
        if self.saving {
            return;
        }
        let Some((queue, draft)) = self.completion_queues.get(index) else {
            return;
        };
        let queue_id = queue.id.clone();
        let baseline = queue.completion_intention;
        let draft = *draft;
        if draft == baseline {
            return;
        }
        self.saving = true;
        self.error = None;
        cx.notify();
        let application = self.application.clone();
        let owner = cx.entity().downgrade();
        cx.defer(move |cx| {
            let result = application.update(cx, |application, cx| {
                application.save_queue_completion_draft(&queue_id, baseline, draft, cx)
            });
            let _ = owner.update(cx, |view, cx| {
                view.saving = false;
                match result {
                    Ok(intention) => {
                        if let Some((queue, current)) = view
                            .completion_queues
                            .iter_mut()
                            .find(|(queue, _)| queue.id == queue_id)
                        {
                            queue.completion_intention = intention;
                            *current = intention;
                        }
                    }
                    Err(message) => view.error = Some(message),
                }
                cx.notify();
            });
        });
    }
}

impl DownloadApplication {
    fn save_queue_completion_draft(
        &mut self,
        queue_id: &str,
        baseline: DownloadCompletionIntention,
        draft: DownloadCompletionIntention,
        cx: &mut Context<Self>,
    ) -> Result<DownloadCompletionIntention, String> {
        let correlation_id = uuid::Uuid::new_v4();
        match self.execute_correlated(
            DownloadCommand::Queue {
                action: DownloadQueueAction::SetCompletionIntention {
                    queue_id: queue_id.into(),
                    intention: draft,
                    baseline: Some(baseline),
                },
            },
            correlation_id,
        ) {
            DownloadCommandOutcome::QueueChanged { queue } => {
                self.refresh(cx);
                Ok(queue.completion_intention)
            }
            DownloadCommandOutcome::Rejected { fault } => Err(fault.message),
            _ => Err("下载引擎未确认队列完成动作".into()),
        }
    }
}
