//! Runtime-only capture intent and acknowledged application on the existing native event owner.
use super::*;
use nxtools_protocol::SetClipboardCapturePausedCommand;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct CapturePauseSnapshot {
    pub requested: bool,
    pub pending: bool,
    pub applied: Option<bool>,
    pub failed: bool,
    pub timed: bool,
}
impl CapturePauseSnapshot {
    pub(crate) fn label(&self) -> &'static str {
        if self.pending {
            return if self.requested {
                "正在暂停…"
            } else {
                "正在恢复…"
            };
        }
        if self.failed {
            return if self.requested {
                "Windows 未确认暂停"
            } else {
                "Windows 未确认恢复"
            };
        }
        if self.applied == Some(true) {
            return if self.timed {
                "已暂停记录（15分钟）"
            } else {
                "已暂停记录"
            };
        }
        if self.applied == Some(false) {
            "记录中"
        } else {
            "等待 Windows 确认"
        }
    }
}
#[derive(Default)]
pub(super) struct CapturePauseState {
    requested: bool,
    deadline: Option<Instant>,
    pub(super) generation: u64,
    pending: Option<(uuid::Uuid, bool, Instant)>,
    applied: Option<bool>,
    failed: bool,
    ignore_through_unix_ms: i64,
}
impl CapturePauseState {
    fn snapshot(&self) -> CapturePauseSnapshot {
        CapturePauseSnapshot {
            requested: self.requested,
            pending: self.pending.is_some(),
            applied: self.applied,
            failed: self.failed,
            timed: self.deadline.is_some(),
        }
    }
    pub(super) fn blocks(&self, captured_at: i64) -> bool {
        self.requested
            || self.applied == Some(true)
            || self.pending.is_some()
            || (self.failed && self.applied.is_none())
            || captured_at <= self.ignore_through_unix_ms
    }
}
impl ClipboardApplication {
    pub(crate) fn capture_pause_snapshot(&self) -> CapturePauseSnapshot {
        self.capture_pause.snapshot()
    }

    pub(crate) fn set_capture_pause(&mut self, paused: bool, timed: bool, cx: &mut Context<Self>) {
        if self.capture_pause.pending.is_some() {
            return;
        }
        self.capture_pause.requested = paused;
        self.invalidate_currency_capture();
        self.capture_pause.deadline =
            (paused && timed).then(|| Instant::now() + Duration::from_secs(15 * 60));
        self.capture_pause.generation = self.capture_pause.generation.wrapping_add(1);
        let generation = self.capture_pause.generation;
        if paused && timed {
            cx.spawn(async move |owner, cx| {
                cx.background_executor()
                    .timer(Duration::from_secs(15 * 60))
                    .await;
                let _ = owner.update(cx, |owner, cx| {
                    if owner.capture_pause.generation == generation && owner.capture_pause.requested
                    {
                        if owner.capture_pause.pending.is_some() {
                            owner.capture_pause.pending = None;
                        }
                        info!(
                            event = "clipboard.capture_pause.expired",
                            generation, "temporary capture pause expired"
                        );
                        owner.capture_pause.requested = false;
                        owner.capture_pause.deadline = None;
                        owner.dispatch_capture_pause(cx);
                    }
                });
            })
            .detach();
        }
        self.dispatch_capture_pause(cx);
    }
    pub(super) fn reapply_capture_pause(&mut self, cx: &mut Context<Self>) {
        self.capture_pause.pending = None;
        self.invalidate_currency_capture();
        info!(
            event = "clipboard.capture_pause.reapplied",
            generation = self.capture_pause.generation,
            "reapplying runtime capture intent"
        );
        self.capture_pause.applied = None;
        if self
            .capture_pause
            .deadline
            .is_some_and(|deadline| deadline <= Instant::now())
        {
            self.capture_pause.requested = false;
            self.capture_pause.deadline = None;
        }
        self.dispatch_capture_pause(cx);
    }
    pub(super) fn disconnect_capture_pause(&mut self, cx: &mut Context<Self>) {
        self.capture_pause.pending = None;
        self.invalidate_currency_capture();
        self.capture_pause.applied = None;
        self.capture_pause.failed = true;
        self.publish_capture_pause(cx);
    }
    fn dispatch_capture_pause(&mut self, cx: &mut Context<Self>) {
        let command_id = uuid::Uuid::new_v4();
        let paused = self.capture_pause.requested;
        self.capture_pause.pending = Some((command_id, paused, Instant::now()));
        self.capture_pause.failed = false;
        if paused {
            self.capture_pause.ignore_through_unix_ms = nxtools_ipc::unix_time_ms();
        }
        info!(event="clipboard.capture_pause.started", correlation_id=%command_id, paused, "capture gate requested");
        let result = self.platform.send_command_with_id(
            PlatformCommand::SetClipboardCapturePaused(SetClipboardCapturePausedCommand { paused }),
            command_id,
        );
        if result.is_err() {
            self.capture_pause.pending = None;
            self.capture_pause.failed = true;
            self.capture_pause.applied = None;
            warn!(event="clipboard.capture_pause.failed", correlation_id=%command_id, failure_kind="enqueue", paused);
        } else {
            cx.spawn(async move |owner, cx| {
                cx.background_executor().timer(Duration::from_secs(5)).await;
                let _ = owner.update(cx, |owner, cx| {
                    if owner.capture_pause.pending.is_some_and(|(id,_,_)| id==command_id) {
                        owner.capture_pause.pending = None;
                        owner.capture_pause.applied = None;
                        owner.capture_pause.failed = true;
                        warn!(event="clipboard.capture_pause.failed", correlation_id=%command_id, failure_kind="timeout", paused);
                        owner.publish_capture_pause(cx);
                    }
                });
            }).detach();
        }
        self.publish_capture_pause(cx);
    }
    pub(super) fn handle_capture_pause_result(
        &mut self,
        command_id: uuid::Uuid,
        result: &nxtools_protocol::PlatformCommandResult,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some((pending, paused, started)) = self.capture_pause.pending else {
            return false;
        };
        if pending != command_id {
            return false;
        }
        self.capture_pause.pending = None;
        let acknowledged = result.success
            && result
                .data
                .as_deref()
                .and_then(|value| serde_json::from_str::<serde_json::Value>(value).ok())
                .and_then(|value| {
                    value
                        .get("capture_paused")
                        .and_then(serde_json::Value::as_bool)
                })
                == Some(paused);
        self.capture_pause.applied = acknowledged.then_some(paused);
        self.capture_pause.failed = !acknowledged;
        if acknowledged && !paused {
            self.capture_pause.ignore_through_unix_ms = nxtools_ipc::unix_time_ms();
        }
        info!(event="clipboard.capture_pause.completed", correlation_id=%command_id,
            paused, success=acknowledged, native_error_code=result.native_error_code,
            elapsed_ms=started.elapsed().as_millis(), "capture gate terminal result");
        self.publish_capture_pause(cx);
        true
    }
    fn publish_capture_pause(&self, cx: &mut Context<Self>) {
        if let Some(popup) = self.popup.lifecycle.current() {
            let panel = popup.panel.clone();
            let snapshot = self.capture_pause.snapshot();
            cx.defer(move |cx| {
                let _ = panel.update(cx, |panel, cx| panel.update_capture_pause(snapshot, cx));
            });
        }
        cx.notify();
    }
    pub(super) fn capture_is_paused(&self, capture: &ClipboardCapture) -> bool {
        self.capture_pause.blocks(capture.captured_at_unix_ms)
    }
}
