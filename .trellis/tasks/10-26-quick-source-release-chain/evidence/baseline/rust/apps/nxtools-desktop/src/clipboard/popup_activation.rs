//! Primary popup activation follows nonactivating native reveal; previews remain passive.
use super::*;
use nxtools_protocol::ActivateWindowCommand;

pub(super) struct PendingPopupActivation {
    id: uuid::Uuid,
    popup: PopupWindow,
    started: Instant,
}

impl ClipboardApplication {
    fn popup_activation_is_current(&self, pending: &PendingPopupActivation) -> bool {
        self.popup.size_tracking_session == Some(pending.popup.session_id)
            && self.popup.lifecycle.current().is_some_and(|current| {
                current.session_id == pending.popup.session_id
                    && current.window == pending.popup.window
                    && current.native_window_handle == pending.popup.native_window_handle
            })
    }

    pub(super) fn request_popup_activation(&mut self, popup: PopupWindow, cx: &mut Context<Self>) {
        self.retire_popup_activation("superseded");
        let id = uuid::Uuid::new_v4();
        self.popup.pending_activation = Some(PendingPopupActivation {
            id,
            popup: popup.clone(),
            started: Instant::now(),
        });
        if self
            .platform
            .send_command_with_id(
                PlatformCommand::ActivateWindow(ActivateWindowCommand {
                    window_handle: popup.native_window_handle,
                }),
                id,
            )
            .is_err()
        {
            self.finish_popup_activation(id, false, None, "dispatch_failed", cx);
            return;
        }
        info!(event="clipboard.popup.activation_accepted", correlation_id=%id, session_id=%popup.session_id);
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(Duration::from_secs(3)).await;
            let _ = this.update(cx, |this, cx| {
                this.finish_popup_activation(id, false, None, "timeout", cx);
            });
        })
        .detach();
    }

    pub(super) fn finish_popup_activation(
        &mut self,
        id: uuid::Uuid,
        success: bool,
        native_error_code: Option<i32>,
        reason: &'static str,
        cx: &mut Context<Self>,
    ) -> bool {
        if self
            .popup
            .pending_activation
            .as_ref()
            .is_none_or(|pending| pending.id != id)
        {
            return false;
        }
        let pending = self
            .popup
            .pending_activation
            .take()
            .expect("matching activation");
        let current = self.popup_activation_is_current(&pending);
        info!(event="clipboard.popup.activation_completed", correlation_id=%id,
            session_id=%pending.popup.session_id, success, current, reason, native_error_code,
            elapsed_ms=pending.started.elapsed().as_millis() as u64);
        if !current {
            return true;
        }
        if !success {
            self.notify_error(
                "clipboard-popup-activation",
                "剪贴板未获得键盘焦点",
                "请点击搜索框，或重新呼出剪贴板后重试。",
                cx,
            );
            return true;
        }
        let owner = cx.entity().downgrade();
        cx.defer(move |cx| {
            // Hide/replacement can happen between native completion and the next app turn.
            let still_current = owner
                .update(cx, |owner, _| owner.popup_activation_is_current(&pending))
                .unwrap_or(false);
            if still_current {
                let popup = pending.popup;
                let _ = popup.window.update(cx, |_, window, cx| {
                    let _ = popup.panel.update(cx, |panel, cx| {
                        panel.focus_search_for_session(popup.session_id, window, cx);
                    });
                });
            }
        });
        true
    }

    pub(super) fn retire_popup_activation(&mut self, reason: &'static str) {
        if let Some(pending) = self.popup.pending_activation.take() {
            info!(event="clipboard.popup.activation_cancelled", correlation_id=%pending.id,
                session_id=%pending.popup.session_id, reason,
                elapsed_ms=pending.started.elapsed().as_millis() as u64);
        }
    }
}
