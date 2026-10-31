//! Explicit notification-button copy; no global shortcut or second notification clock.
use super::*;
use gpui_kit::component::{
    Disableable, IconName, Sizable,
    button::{Button, ButtonVariants as _},
};
use gpui_kit::{IntoElement, Render, Subscription, prelude::*};
use nxtools_ipc::{PlatformBridge, PlatformNotification};
use nxtools_protocol::{PasteTextCommand, PlatformCommand};
use std::{collections::HashMap, time::Instant};
use uuid::Uuid;

const MAX_COPY_BYTES: usize = 256 * 1024;
const COPY_TIMEOUT: Duration = Duration::from_secs(15);
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum CopyStatus {
    #[default]
    Idle,
    Pending,
    Copied,
    Failed,
}
struct PendingCopy {
    command: Uuid,
    id: SharedString,
    generation: u64,
    started: Instant,
}
#[derive(Default)]
pub(super) struct CopyRuntime {
    platform: Option<Rc<PlatformBridge>>,
    pending: Option<PendingCopy>,
    states: HashMap<u64, CopyStatus>,
}
impl CopyRuntime {
    pub(super) fn retain(&mut self, items: &[AppNotification]) {
        self.states
            .retain(|generation, _| items.iter().any(|item| item.generation() == *generation));
    }
}
fn plain_text(item: &AppNotification) -> Result<String, ()> {
    let detail_lengths = item.details.iter().take(8).map(|detail| {
        detail
            .label
            .len()
            .checked_add("：".len())?
            .checked_add(detail.value.len())
    });
    let mut lengths = [Some(item.title.len()), Some(item.message.len())]
        .into_iter()
        .chain(detail_lengths);
    let length = lengths
        .try_fold(0_usize, |length, line| {
            length.checked_add(line?)?.checked_add(2)
        })
        .ok_or(())?;
    if length > MAX_COPY_BYTES {
        return Err(());
    }
    let mut lines = Vec::with_capacity(10);
    lines.extend(
        [item.title.as_ref(), item.message.as_ref()]
            .into_iter()
            .filter(|line| !line.is_empty())
            .map(str::to_owned),
    );
    lines.extend(
        item.details
            .iter()
            .take(8)
            .map(|detail| format!("{}：{}", detail.label, detail.value)),
    );
    if lines.is_empty() {
        return Err(());
    }
    Ok(lines.join("\r\n"))
}

pub(crate) fn handle(notification: PlatformNotification, cx: &mut App) {
    if !matches!(
        notification,
        PlatformNotification::CommandResult { .. } | PlatformNotification::Disconnected { .. }
    ) {
        return;
    }
    let Some(center) = current(cx).map(|center| center.downgrade()) else {
        return;
    };
    cx.defer(move |cx| {
        let _ = center.update(cx, |center, cx| center.handle_copy(notification, cx));
    });
}
impl NotificationCenter {
    pub(crate) fn bind_copy_platform(&mut self, platform: Rc<PlatformBridge>) {
        self.copy.platform = Some(platform);
    }
    pub(super) fn copy_status(&self, generation: u64) -> CopyStatus {
        self.copy
            .states
            .get(&generation)
            .copied()
            .unwrap_or_default()
    }
    pub(super) fn request_copy(&mut self, id: &str, generation: u64, cx: &mut Context<Self>) {
        if self.copy.pending.is_some() {
            return;
        }
        let Some(item) = self.items.iter().find(|item| {
            item.id.as_ref() == id && item.generation() == generation && !item.exiting
        }) else {
            return;
        };
        let command = Uuid::new_v4();
        let started = Instant::now();
        tracing::info!(event="ui.notification.copy.accepted",correlation_id=%command,generation,"notification copy admitted");
        let Ok(text) = plain_text(item) else {
            self.copy.states.insert(generation, CopyStatus::Failed);
            tracing::warn!(event="ui.notification.copy.terminal",correlation_id=%command,generation,success=false,reason="payload_budget",elapsed_ms=started.elapsed().as_millis(),"notification copy rejected");
            cx.notify();
            return;
        };
        let pending = PendingCopy {
            command,
            id: item.id.clone(),
            generation,
            started,
        };
        self.copy.pending = Some(pending);
        self.copy.states.insert(generation, CopyStatus::Pending);
        cx.notify();
        let sent = self.copy.platform.as_ref().is_some_and(|platform| {
            platform
                .send_command_with_id_correlated(
                    PlatformCommand::PasteText(PasteTextCommand {
                        text,
                        paste_after_write: false,
                        target_window_handle: 0,
                        target_focus_window_handle: 0,
                    }),
                    command,
                    command,
                )
                .is_ok()
        });
        if !sent {
            self.finish_copy(command, false, "platform_enqueue", None, cx);
            return;
        }
        tracing::info!(event="ui.notification.copy.routed",correlation_id=%command,generation,"notification copy sent to Windows owner");
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(COPY_TIMEOUT).await;
            let _ = this.update(cx, |center, cx| {
                center.finish_copy(command, false, "timeout", None, cx)
            });
        })
        .detach();
    }
    fn finish_copy(
        &mut self,
        command: Uuid,
        success: bool,
        reason: &'static str,
        native_error: Option<i32>,
        cx: &mut Context<Self>,
    ) {
        if self
            .copy
            .pending
            .as_ref()
            .is_none_or(|pending| pending.command != command)
        {
            return;
        }
        let pending = self.copy.pending.take().expect("matching copy");
        if self.items.iter().any(|item| {
            item.id == pending.id && item.generation() == pending.generation && !item.exiting
        }) {
            self.copy.states.insert(
                pending.generation,
                if success {
                    CopyStatus::Copied
                } else {
                    CopyStatus::Failed
                },
            );
        }
        self.copy.retain(&self.items);
        tracing::info!(event="ui.notification.copy.terminal",correlation_id=%command,generation=pending.generation,success,reason,native_error_code=?native_error,elapsed_ms=pending.started.elapsed().as_millis(),"notification copy terminated");
        cx.notify();
    }
    pub(super) fn handle_copy(
        &mut self,
        notification: PlatformNotification,
        cx: &mut Context<Self>,
    ) {
        match notification {
            PlatformNotification::CommandResult {
                correlation_id,
                result,
            } => self.finish_copy(
                correlation_id,
                result.success,
                "native_result",
                result.native_error_code,
                cx,
            ),
            PlatformNotification::Disconnected { .. } => {
                if let Some(command) = self.copy.pending.as_ref().map(|pending| pending.command) {
                    self.finish_copy(command, false, "disconnected", None, cx);
                }
            }
            _ => {}
        }
    }
}

pub(super) struct CopyButton {
    center: WeakEntity<NotificationCenter>,
    id: SharedString,
    generation: u64,
    _subscription: Subscription,
}
impl CopyButton {
    pub(super) fn new(
        center: Entity<NotificationCenter>,
        id: SharedString,
        generation: u64,
        cx: &mut Context<Self>,
    ) -> Self {
        let subscription = cx.observe(&center, |_, _, cx| cx.notify());
        Self {
            center: center.downgrade(),
            id,
            generation,
            _subscription: subscription,
        }
    }
}
impl Render for CopyButton {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let center = self.center.upgrade();
        let (status, disabled) = center
            .as_ref()
            .map(|center| {
                let center = center.read(cx);
                let valid = center.items().iter().any(|item| {
                    item.id == self.id && item.generation() == self.generation && !item.exiting
                });
                (
                    center.copy_status(self.generation),
                    !valid || center.copy.pending.is_some(),
                )
            })
            .unwrap_or((CopyStatus::Failed, true));
        let (icon, label) = match status {
            CopyStatus::Idle => (IconName::Copy, "复制通知内容"),
            CopyStatus::Pending => (IconName::Copy, "正在复制通知内容"),
            CopyStatus::Copied => (IconName::Check, "已复制通知内容"),
            CopyStatus::Failed => (
                IconName::Copy,
                "复制失败，请重试（内容过长或 Windows 平台不可用）",
            ),
        };
        let selector = format!("notification-copy-button-{}", self.id);
        let center = self.center.clone();
        let id = self.id.clone();
        let generation = self.generation;
        Button::new("notification-copy")
            .debug_selector(move || selector.clone())
            .icon(icon)
            .ghost()
            .xsmall()
            .tooltip(label)
            .accessibility_label(label)
            .disabled(disabled)
            .on_click(move |_, _, cx| {
                cx.stop_propagation();
                let center = center.clone();
                let id = id.clone();
                cx.defer(move |cx| {
                    let _ =
                        center.update(cx, |center, cx| center.request_copy(&id, generation, cx));
                });
            })
    }
}
