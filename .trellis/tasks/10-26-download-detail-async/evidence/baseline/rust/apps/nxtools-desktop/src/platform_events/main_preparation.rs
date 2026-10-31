//! One startup barrier on the original dispatcher; no competing notification receiver.
use super::*;
use gpui_kit::App;
use nxtools_protocol::main_window::{PrepareMainWindowCommand, PreparedMainWindow};
use std::collections::VecDeque;

const MAX_BUFFERED_STARTUP_NOTIFICATIONS: usize = 128;
const MAIN_PREPARATION_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(8);
type Ready = Box<dyn FnOnce(&mut App)>;

pub(crate) struct MainWindowPreparation {
    command: PrepareMainWindowCommand,
    pending: Option<Uuid>,
    acknowledged: bool,
    buffered: VecDeque<PlatformNotification>,
    ready: Option<Ready>,
    started: Instant,
}
impl MainWindowPreparation {
    pub(crate) fn new(window_handle: i64, ready: impl FnOnce(&mut App) + 'static) -> Self {
        Self {
            command: PrepareMainWindowCommand {
                window_handle,
                source_process_id: std::process::id(),
                width: crate::MAIN_WINDOW_LOGICAL_WIDTH as i32,
                height: crate::MAIN_WINDOW_LOGICAL_HEIGHT as i32,
            },
            pending: None,
            acknowledged: false,
            buffered: VecDeque::new(),
            ready: Some(Box::new(ready)),
            started: Instant::now(),
        }
    }
}
impl PlatformEventDispatcher {
    pub(super) fn intercept_main_preparation(
        &mut self,
        notification: &PlatformNotification,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(preparation) = self.main_preparation.as_mut() else {
            return false;
        };
        if let PlatformNotification::CommandResult {
            correlation_id,
            result,
        } = notification
            && preparation.pending == Some(*correlation_id)
            && !preparation.acknowledged
        {
            let valid = result.success
                && result
                    .data
                    .as_deref()
                    .filter(|data| data.len() <= 1024)
                    .and_then(|data| serde_json::from_str::<PreparedMainWindow>(data).ok())
                    .is_some_and(|readback| {
                        readback.matches(preparation.command.width, preparation.command.height)
                    });
            if !valid {
                self.fail_main_preparation(
                    "native_result_or_readback",
                    result.native_error_code,
                    cx,
                );
                return true;
            }
            preparation.acknowledged = true;
            let owner = cx.entity().downgrade();
            let correlation_id = *correlation_id;
            cx.defer(move |cx| {
                // Recheck the exact ACK after all current leases before publishing entrypoints.
                let ready = owner.update(cx, |dispatcher, _| {
                    dispatcher.main_preparation.as_mut()
                        .filter(|preparation| preparation.pending == Some(correlation_id) && preparation.acknowledged)
                        .and_then(|preparation| preparation.ready.take())
                }).ok().flatten();
                if let Some(ready) = ready { ready(cx); }
                let _ = owner.update(cx, |dispatcher, cx| {
                    if dispatcher.main_preparation.as_ref().is_none_or(|preparation| preparation.pending != Some(correlation_id) || !preparation.acknowledged) { return; }
                    let preparation = dispatcher.main_preparation.take().expect("current acknowledged startup");
                    info!(event="product.main_window.prepare.completed", correlation_id=%correlation_id, elapsed_ms=preparation.started.elapsed().as_millis());
                    for buffered in preparation.buffered { dispatcher.forward(buffered, cx); }
                });
            });
            return true;
        }
        if matches!(notification, PlatformNotification::Disconnected { .. }) {
            self.fail_main_preparation("platform_disconnected", None, cx);
            return true;
        }
        if preparation.buffered.len() >= MAX_BUFFERED_STARTUP_NOTIFICATIONS {
            self.fail_main_preparation("startup_notification_budget", None, cx);
            return true;
        }
        if matches!(notification, PlatformNotification::Connected) {
            if preparation.pending.is_some() {
                return true;
            }
            preparation.buffered.push_back(notification.clone());
            let id = Uuid::new_v4();
            preparation.pending = Some(id);
            info!(event="product.main_window.prepare.requested", correlation_id=%id, window_handle=preparation.command.window_handle);
            if self
                .platform
                .send_command_with_id(
                    PlatformCommand::PrepareMainWindow(preparation.command.clone()),
                    id,
                )
                .is_err()
            {
                self.fail_main_preparation("platform_enqueue", None, cx);
            }
        } else {
            preparation.buffered.push_back(notification.clone());
        }
        true
    }
    pub(super) fn fail_main_preparation(
        &mut self,
        reason: &'static str,
        native_error_code: Option<i32>,
        cx: &mut Context<Self>,
    ) {
        if let Some(preparation) = self.main_preparation.take() {
            warn!(event="product.main_window.prepare.failed", command_id=?preparation.pending, reason, ?native_error_code, elapsed_ms=preparation.started.elapsed().as_millis());
            self.main_preparation_failed = true;
            // Fail hidden: do not publish a default-sized HWND or enable a native reveal path.
            cx.quit();
        }
    }
    pub(super) fn schedule_main_preparation_deadline(&self, cx: &mut Context<Self>) {
        if self.main_preparation.is_none() {
            return;
        }
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(MAIN_PREPARATION_TIMEOUT)
                .await;
            let _ = this.update(cx, |dispatcher, cx| {
                dispatcher.fail_main_preparation("deadline", None, cx)
            });
        })
        .detach();
    }
}

#[cfg(test)]
mod tests;
