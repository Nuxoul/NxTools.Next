use super::*;
use gpui_kit::component::{
    WindowExt,
    alert::Alert,
    dialog::{DialogAction, DialogClose, DialogFooter},
    form::{Field, Form},
};
use nxtools_download::{DownloadPausedUrlReplacement, DownloadResolutionControl};

pub(super) struct PausedUrlDraft {
    pub(super) input: Entity<InputState>,
    operation_id: uuid::Uuid,
    expected_source_url: String,
    engine_generation: u64,
    control: DownloadResolutionControl,
    busy: bool,
    error: Option<String>,
}

impl Drop for PausedUrlDraft {
    fn drop(&mut self) {
        self.control.cancel();
    }
}

impl DownloadTaskWindowView {
    pub(super) fn cancel_source_draft(&self) {
        if let Some(draft) = &self.source_draft {
            draft.control.cancel();
        }
    }

    pub(super) fn open_source_draft(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.source_draft.is_some() {
            return;
        }
        let Some(task) = self.snapshot.as_ref().filter(|task| {
            task.task.status() == DownloadStatus::Paused && task.task.request().media.is_none()
        }) else {
            return;
        };
        let source_url = task.task.request().source_url.clone();
        let input = cx.new(|cx| InputState::new(window, cx).default_value(source_url.clone()));
        let operation_id = uuid::Uuid::new_v4();
        let control = DownloadResolutionControl::default()
            .with_host_cancellation(self.source_window_lifetime.subscribe());
        let engine_generation = self.application.read(cx).engine.session_generation();
        self.source_draft = Some(PausedUrlDraft {
            input: input.clone(),
            operation_id,
            expected_source_url: source_url,
            engine_generation,
            control: control.clone(),
            busy: false,
            error: None,
        });
        self.resize_source_window(window, true);
        let weak = cx.entity().downgrade();
        window.defer(cx, move |window, cx| {
            if control.is_cancelled() || weak.upgrade().is_none_or(|view| view.read(cx).source_draft.as_ref().is_none_or(|draft| draft.operation_id != operation_id)) {
                return;
            }
        window.open_dialog(cx, move |dialog, _, cx| {
            let Some(view) = weak.upgrade() else { return dialog; };
            let Some(draft) = view.read(cx).source_draft.as_ref().filter(|draft| draft.operation_id == operation_id) else { return dialog; };
            let input = draft.input.clone();
            let busy = draft.busy;
            let error = draft.error.clone();
            let confirm = weak.clone();
            let decline = weak.clone();
            let close = weak.clone();
            let cancel_control = control.clone();
            let close_control = control.clone();
            dialog
                .title("更换下载链接")
                .width(px(408.0))
                .margin_top(px(12.0))
                .overlay_closable(false)
                .child(
                    Form::vertical().child(
                        Field::new().label("新下载链接")
                            .description("仅在资源标识、总量和续传能力兼容时保留已下载区间。拒绝或取消会保留原链接和断点；不会自动从头下载。")
                            .child(div()
                                .debug_selector(|| "download-replace-url-input".into())
                                .w_full().min_w_0()
                                .child(Input::new(&input)
                                    .aria_label("新下载链接")
                                    .disabled(busy))),
                    ),
                )
                .when(busy, |dialog| dialog.child("正在验证新链接；任务保持暂停。"))
                .when_some(error, |dialog, error| dialog.child(div()
                    .debug_selector(|| "download-replace-url-error".into())
                    .child(Alert::error("download-replace-url-error", error).small())))
                .footer(
                    DialogFooter::new()
                        .child(DialogClose::new().trigger(|button| button
                            .debug_selector(|| "download-replace-url-cancel".into())
                            .label("取消").outline()))
                        .child(DialogAction::new().child(
                            Button::new("download-replace-url-confirm")
                                .debug_selector(|| "download-replace-url-confirm".into())
                                .label("验证并更换").primary().disabled(busy)
                                .on_click(|event, window, cx| {
                                    if event.is_keyboard() {
                                        window.dispatch_action(Box::new(gpui_kit::component::dialog::Confirm { secondary: false }), cx);
                                    }
                                }),
                        )),
                )
                .on_ok(move |_, window, cx| {
                    let weak = confirm.clone();
                    window.defer(cx, move |window, cx| {
                        let _ = weak.update(cx, |view, cx| view.submit_source_draft(operation_id, window, cx));
                    });
                    false
                })
                .on_cancel(move |_, window, cx| {
                    cancel_control.cancel();
                    Self::dismiss_source_draft(decline.clone(), operation_id, window, cx);
                    true
                })
                .on_close(move |_, window, cx| {
                    close_control.cancel();
                    Self::dismiss_source_draft(close.clone(), operation_id, window, cx);
                })
        });
            input.read(cx).focus_handle(cx).focus(window, cx);
        });
        cx.notify();
    }

    fn dismiss_source_draft(
        weak: gpui_kit::WeakEntity<Self>,
        operation_id: uuid::Uuid,
        window: &mut Window,
        cx: &mut App,
    ) {
        window.defer(cx, move |window, cx| {
            let _ = weak.update(cx, |view, cx| {
                if view
                    .source_draft
                    .as_ref()
                    .is_some_and(|draft| draft.operation_id == operation_id)
                {
                    view.source_draft = None;
                    view.resize_source_window(window, false);
                    cx.notify();
                }
            });
        });
    }

    fn resize_source_window(&self, window: &mut Window, editing: bool) {
        let target = if editing {
            size(px(440.0), px(400.0))
        } else {
            download_task_window_size_for_details(
                self.snapshot
                    .as_ref()
                    .map_or(DownloadWindowPhase::Progress, |task| {
                        download_window_phase(task.task.status())
                    }),
                self.details_expanded,
            )
        };
        if self.native_window_handle == 0 {
            window.resize(target);
        } else {
            let scale = window.scale_factor();
            let _ = self
                .platform
                .send_command(PlatformCommand::SetWindowBoundsCentered(
                    SetWindowBoundsCenteredCommand {
                        window_handle: self.native_window_handle,
                        width: (target.width.as_f32() * scale).round() as i32,
                        height: (target.height.as_f32() * scale).round() as i32,
                    },
                ));
        }
    }

    fn submit_source_draft(
        &mut self,
        operation_id: uuid::Uuid,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(draft) = self.source_draft.as_mut().filter(|draft| {
            draft.operation_id == operation_id && !draft.busy && !draft.control.is_cancelled()
        }) else {
            return;
        };
        let source_url = draft.input.read(cx).value().trim().to_owned();
        let Some(task) = self
            .snapshot
            .as_ref()
            .filter(|task| task.task.status() == DownloadStatus::Paused)
        else {
            return;
        };
        let mut candidate = task.task.request().clone();
        candidate.source_url = source_url.clone();
        if let Err(error) = candidate.validate() {
            draft.error = Some(humanize_download_fault(&error.to_string()));
            cx.notify();
            return;
        }
        if source_url == draft.expected_source_url {
            draft.error = Some("请输入与原链接不同的新链接。".into());
            cx.notify();
            return;
        }
        let engine = self.application.read(cx).engine.clone();
        let expected_revision = match engine.paused_url_replacement_revision(self.task_id) {
            Ok(revision) => revision,
            Err(error) => {
                draft.error = Some(humanize_download_fault(&error.to_string()));
                cx.notify();
                return;
            }
        };
        let attempt_id = uuid::Uuid::new_v4();
        let command = DownloadPausedUrlReplacement {
            task_id: self.task_id,
            operation_id: attempt_id,
            generation: draft.engine_generation,
            expected_revision,
            expected_source_url: draft.expected_source_url.clone(),
            source_url,
        };
        let control = draft.control.clone();
        draft.busy = true;
        draft.error = None;
        let application = self.application.downgrade();
        let window_generation = self.window_generation;
        let source_window = window.window_handle();
        let background = cx.background_executor().clone();
        cx.spawn(async move |view, cx| {
            let result = background.spawn(async move {
                engine.replace_paused_url_with_control(command, attempt_id, control)
            }).await;
            let _ = source_window.update(cx, |_, window, cx| {
                let finished = view.update(cx, |view, cx| {
                    if view.window_generation != window_generation { return false; }
                    let Some(draft) = view.source_draft.as_mut().filter(|draft| draft.operation_id == operation_id) else { return false; };
                    draft.busy = false;
                    match result {
                        Ok(()) => {
                            view.source_draft = None;
                            view.resize_source_window(window, false);
                            cx.notify();
                            true
                        },
                        Err(error) => {
                            draft.error = Some(match error {
                                nxtools_download::DownloadEngineError::ResourceChanged(_) => "无法证明是同一资源，或新链接不支持原区间续传。已保留原链接和断点。".into(),
                                _ => humanize_download_fault(&error.to_string()),
                            });
                            cx.notify();
                            false
                        }
                    }
                }).unwrap_or(false);
                if finished { window.close_dialog(cx); }
            });
            cx.update(|cx| {
                cx.defer(move |cx| { let _ = application.update(cx, |app, cx| app.refresh(cx)); });
            });
        }).detach();
        cx.notify();
    }
}
