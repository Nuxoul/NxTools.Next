use super::*;
use crate::ui::metrics::ordinary_rem;
use gpui_kit::component::label::Label;

/// Presentation-only draft; applied rates remain in the original download engine.
pub(super) struct SpeedLimitEdit {
    pub(super) applied: Option<u64>,
    pub(super) pending: Option<uuid::Uuid>,
    pub(super) dirty: bool,
    pub(super) error: Option<String>,
    #[cfg(test)]
    pub(super) submissions: usize,
}

#[derive(Clone, Copy)]
struct SpeedLimitSubmission {
    correlation_id: uuid::Uuid,
    limit: Option<u64>,
    started_at: Instant,
}

impl SpeedLimitEdit {
    pub(super) fn new(applied: Option<u64>) -> Self {
        Self {
            applied,
            pending: None,
            dirty: false,
            error: None,
            #[cfg(test)]
            submissions: 0,
        }
    }

    pub(super) fn value(&self) -> String {
        self.applied
            .map_or(0, |limit| limit.div_ceil(1024))
            .to_string()
    }

    pub(super) fn changed(&mut self) {
        self.dirty = true;
        self.error = None;
    }

    fn begin(&mut self, text: &str, scope: &'static str) -> Option<SpeedLimitSubmission> {
        if self.pending.is_some() || !self.dirty {
            return None;
        }
        let parsed = text
            .trim()
            .parse::<u64>()
            .ok()
            .and_then(|kib| kib.checked_mul(1024));
        let Some(bytes) = parsed else {
            self.error = Some("限速必须是非负整数 KiB/s；0 表示不限速".into());
            return None;
        };
        let limit = (bytes != 0).then_some(bytes);
        if limit == self.applied {
            self.dirty = false;
            self.error = None;
            return None;
        }
        let submission = SpeedLimitSubmission {
            correlation_id: uuid::Uuid::new_v4(),
            limit,
            started_at: Instant::now(),
        };
        self.pending = Some(submission.correlation_id);
        self.error = None;
        #[cfg(test)]
        {
            self.submissions += 1;
        }
        info!(event = "download.speed_limit.edit_started", correlation_id = %submission.correlation_id,
            scope, enabled = limit.is_some(), "speed limit edit submission started");
        Some(submission)
    }

    fn finish(&mut self, submission: SpeedLimitSubmission, result: Result<(), &'static str>) {
        if self.pending != Some(submission.correlation_id) {
            return;
        }
        self.pending = None;
        if result.is_ok() {
            self.applied = submission.limit;
            self.dirty = false;
            self.error = None;
        } else {
            self.dirty = true;
            self.error = Some("限速保存失败，请重试；已应用值保持不变".into());
        }
    }
}

fn terminal(
    submission: SpeedLimitSubmission,
    scope: &'static str,
    result: Result<(), &'static str>,
) {
    match result {
        Ok(()) => {
            info!(event = "download.speed_limit.edit_applied", correlation_id = %submission.correlation_id,
            scope, elapsed_ms = submission.started_at.elapsed().as_millis(), "speed limit edit applied")
        }
        Err("window_retired") => {
            info!(event = "download.speed_limit.edit_cancelled", correlation_id = %submission.correlation_id,
            scope, reason = "window_retired", elapsed_ms = submission.started_at.elapsed().as_millis(), "speed limit source retired")
        }
        Err(_) => {
            warn!(event = "download.speed_limit.edit_failed", correlation_id = %submission.correlation_id,
            scope, error_kind = "command_rejected", elapsed_ms = submission.started_at.elapsed().as_millis(), "speed limit edit failed")
        }
    }
}

impl DownloadSettingsView {
    pub(super) fn defer_global_speed_submission(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        cx.defer_in(window, |this, window, cx| {
            if this.saving {
                return;
            }
            let text = this.speed_limit_kib.read(cx).value().to_string();
            let Some(submission) = this.speed_limit_edit.begin(&text, "global") else {
                cx.notify();
                return;
            };
            let source_window = window.window_handle();
            let application = this.application.clone();
            let view = cx.entity().downgrade();
            cx.notify();
            cx.defer(move |cx| {
                let result = application.update(cx, |app, cx| {
                    if app.settings_window != Some(source_window) {
                        return Err("window_retired");
                    }
                    app.set_global_speed_limit(submission.limit, submission.correlation_id)
                        .map_err(|_| "command_rejected")?;
                    app.defer_effective_limit_display(cx);
                    Ok(())
                });
                terminal(submission, "global", result);
                cx.defer(move |cx| {
                    let _ = view.update(cx, |view, cx| {
                        view.speed_limit_edit.finish(submission, result);
                        cx.notify();
                    });
                });
            });
        });
    }

    pub(super) fn defer_global_speed_restore(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        cx.defer_in(window, |this, window, cx| {
            if this.speed_limit_edit.pending.is_some() {
                return;
            }
            let value = this.speed_limit_edit.value();
            this.speed_limit_kib
                .update(cx, |input, cx| input.set_value(value, window, cx));
            this.speed_limit_edit.dirty = false;
            this.speed_limit_edit.error = None;
            cx.notify();
        });
    }
}

impl DownloadTaskWindowView {
    pub(super) fn ensure_speed_limit_input(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.speed_limit_kib.is_some() {
            return;
        }
        let value = self.speed_limit_edit.value();
        let input = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(value)
                .placeholder("0")
        });
        self.speed_limit_subscription = Some(cx.subscribe_in(
            &input,
            window,
            |this, _, event: &InputEvent, window, cx| match event {
                InputEvent::Change => {
                    this.speed_limit_edit.changed();
                    cx.notify();
                }
                InputEvent::PressEnter { .. } | InputEvent::Blur => {
                    this.defer_task_speed_submission(window, cx)
                }
                InputEvent::Focus => {}
            },
        ));
        self.speed_limit_kib = Some(input);
    }

    pub(super) fn sync_task_speed_input(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.speed_limit_edit.dirty
            || self.speed_limit_edit.pending.is_some()
            || self.speed_limit_edit.error.is_some()
        {
            return;
        }
        if let Some(input) = self.speed_limit_kib.as_ref() {
            let value = self.speed_limit_edit.value();
            if input.read(cx).value() != value {
                input.update(cx, |input, cx| input.set_value(value, window, cx));
            }
        }
    }

    fn defer_task_speed_submission(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        cx.defer_in(window, |this, window, cx| {
            let Some(input) = this.speed_limit_kib.as_ref() else {
                return;
            };
            let text = input.read(cx).value().to_string();
            let Some(submission) = this.speed_limit_edit.begin(&text, "task") else {
                cx.notify();
                return;
            };
            let task_id = this.task_id;
            let generation = this.window_generation;
            let view_id = cx.entity_id();
            let source_window = window.window_handle();
            let application = this.application.clone();
            let view = cx.entity().downgrade();
            cx.notify();
            cx.defer(move |cx| {
                let result = application.update(cx, |app, cx| {
                    if !app.task_windows.get(&task_id).is_some_and(|entry| {
                        entry.generation == generation
                            && entry.view.entity_id() == view_id
                            && entry.window == source_window
                    }) {
                        return Err("window_retired");
                    }
                    match app.execute_correlated(
                        DownloadCommand::Task {
                            task_id,
                            action: DownloadTaskAction::SetSpeedLimit {
                                bytes_per_second: submission.limit,
                            },
                        },
                        submission.correlation_id,
                    ) {
                        DownloadCommandOutcome::Accepted {
                            task_id: Some(actual),
                        } if actual == task_id => {
                            app.refresh(cx);
                            Ok(())
                        }
                        _ => Err("command_rejected"),
                    }
                });
                terminal(submission, "task", result);
                cx.defer(move |cx| {
                    let _ = view.update(cx, |view, cx| {
                        view.speed_limit_edit.finish(submission, result);
                        cx.notify();
                    });
                });
            });
        });
    }

    fn defer_task_speed_restore(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        cx.defer_in(window, |this, window, cx| {
            if this.speed_limit_edit.pending.is_some() {
                return;
            }
            if let Some(input) = this.speed_limit_kib.as_ref() {
                let value = this.speed_limit_edit.value();
                input.update(cx, |input, cx| input.set_value(value, window, cx));
            }
            this.speed_limit_edit.dirty = false;
            this.speed_limit_edit.error = None;
            cx.notify();
        });
    }

    pub(super) fn render_speed_limit_editor(&self, cx: &mut Context<Self>) -> AnyElement {
        let Some(input) = self.speed_limit_kib.as_ref() else {
            return div().into_any_element();
        };
        div()
            .w_full()
            .flex_none()
            .flex()
            .flex_col()
            .on_action(
                cx.listener(|this, _: &gpui_kit::component::input::Escape, window, cx| {
                    cx.stop_propagation();
                    this.defer_task_speed_restore(window, cx);
                }),
            )
            .child(
                div()
                    .h(ordinary_rem(32.0))
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(Label::new("限速 KiB/s").text_size(ordinary_rem(TEXT_LABEL)))
                    .child(
                        div().w(ordinary_rem(96.0)).child(field_shell(
                            self.theme,
                            "download-task-speed-limit-input",
                            Input::new(input)
                                .aria_label("任务限速 KiB/s")
                                .disabled(self.speed_limit_edit.pending.is_some()),
                        )),
                    )
                    .child(
                        Label::new(if self.speed_limit_edit.pending.is_some() {
                            "保存中"
                        } else {
                            "0 不限速"
                        })
                        .text_size(ordinary_rem(TEXT_LABEL)),
                    ),
            )
            .when_some(self.speed_limit_edit.error.clone(), |row, error| {
                row.child(
                    div()
                        .debug_selector(|| "download-task-speed-limit-error".into())
                        .child(
                            Label::new(error)
                                .text_size(ordinary_rem(TEXT_LABEL))
                                .text_color(self.theme.danger),
                        ),
                )
            })
            .into_any_element()
    }
}
