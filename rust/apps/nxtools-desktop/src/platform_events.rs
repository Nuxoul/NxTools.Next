use gpui::{Context, Entity, Global, Task, WeakEntity};
use nxtools_ipc::{PlatformBridge, PlatformNotification};
use std::rc::Rc;

use crate::clipboard::ClipboardApplication;
use crate::downloads::DownloadApplication;
use crate::quick_launch::QuickLaunchApplication;

pub(crate) struct PlatformEventDispatcher {
    clipboard: WeakEntity<ClipboardApplication>,
    // The dispatcher is the lifetime owner for background modules that do not have another
    // application-global owner. Keeping only a WeakEntity here drops quick launch before the
    // platform Connected notification can configure its summon gesture.
    quick_launch: Entity<QuickLaunchApplication>,
    downloads: Entity<DownloadApplication>,
    _task: Task<()>,
}

impl PlatformEventDispatcher {
    pub(crate) fn new(
        platform: Rc<PlatformBridge>,
        clipboard: WeakEntity<ClipboardApplication>,
        quick_launch: Entity<QuickLaunchApplication>,
        downloads: Entity<DownloadApplication>,
        cx: &mut Context<Self>,
    ) -> Self {
        let receiver = platform.notification_receiver();
        let task = cx.spawn(async move |this, cx| {
            while let Ok(notification) = receiver.recv().await {
                let Some(this) = this.upgrade() else {
                    return;
                };
                if this
                    .update(cx, |dispatcher, cx| dispatcher.forward(notification, cx))
                    .is_err()
                {
                    return;
                }
            }
        });
        Self {
            clipboard,
            quick_launch,
            downloads,
            _task: task,
        }
    }

    fn forward(&self, notification: PlatformNotification, cx: &mut Context<Self>) {
        let _ = self.clipboard.update(cx, |application, cx| {
            application.handle_platform_notification(notification.clone(), cx);
        });
        self.quick_launch.update(cx, |application, cx| {
            application.handle_platform_notification(notification.clone(), cx);
        });
        self.downloads.update(cx, |application, cx| {
            application.handle_platform_notification(notification, cx);
        });
    }
}

pub(crate) struct PlatformEventDispatcherGlobal {
    _dispatcher: Entity<PlatformEventDispatcher>,
}

impl PlatformEventDispatcherGlobal {
    pub(crate) fn new(dispatcher: Entity<PlatformEventDispatcher>) -> Self {
        Self {
            _dispatcher: dispatcher,
        }
    }
}

impl Global for PlatformEventDispatcherGlobal {}
