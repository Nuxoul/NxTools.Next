use gpui::{Context, Entity, Global, Task, WeakEntity};
use nxtools_ipc::{PlatformBridge, PlatformNotification};
use nxtools_protocol::{PlatformCommand, SetWindowMinimizedCommand};
use std::{rc::Rc, time::Instant};
use tracing::{info, warn};
use uuid::Uuid;

use crate::clipboard::ClipboardApplication;
use crate::downloads::DownloadApplication;
use crate::quick_launch::QuickLaunchApplication;

pub(crate) struct PlatformEventDispatcher {
    platform: Rc<PlatformBridge>,
    browser_startup_window: Option<BrowserStartupWindow>,
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
        browser_startup_window: Option<BrowserStartupWindow>,
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
            platform,
            browser_startup_window,
            clipboard,
            quick_launch,
            downloads,
            _task: task,
        }
    }

    fn forward(&mut self, notification: PlatformNotification, cx: &mut Context<Self>) {
        match &notification {
            PlatformNotification::Connected => self.begin_browser_startup_minimize(cx),
            PlatformNotification::CommandResult {
                correlation_id,
                result,
            } => {
                let Some(startup) = self.browser_startup_window.as_ref() else {
                    self.forward_to_applications(notification, cx);
                    return;
                };
                if startup.pending_command_id != Some(*correlation_id) {
                    self.forward_to_applications(notification, cx);
                    return;
                }
                let startup = self
                    .browser_startup_window
                    .take()
                    .expect("startup window was checked above");
                if result.success {
                    info!(
                        event = "download.browser_startup.main_window_minimized",
                        success = true,
                        correlation_id = %startup.correlation_id,
                        command_id = %correlation_id,
                        window_handle = startup.window_handle,
                        elapsed_ms = startup.started_at.elapsed().as_millis(),
                        "Windows confirmed the browser-startup main window is minimized"
                    );
                } else {
                    warn!(
                        event = "download.browser_startup.main_window_minimize_failed",
                        success = false,
                        correlation_id = %startup.correlation_id,
                        command_id = %correlation_id,
                        window_handle = startup.window_handle,
                        error_present = result.error.is_some(),
                        elapsed_ms = startup.started_at.elapsed().as_millis(),
                        "Windows rejected the browser-startup main-window minimize request"
                    );
                }
            }
            PlatformNotification::Event(_) => {}
        }
        self.forward_to_applications(notification, cx);
    }

    fn begin_browser_startup_minimize(&mut self, cx: &mut Context<Self>) {
        let Some(startup) = self.browser_startup_window.as_mut() else {
            return;
        };
        if startup.pending_command_id.is_some() {
            return;
        }
        let command = PlatformCommand::SetWindowMinimized(SetWindowMinimizedCommand {
            window_handle: startup.window_handle,
        });
        match self
            .platform
            .send_command_correlated(command, startup.correlation_id)
        {
            Ok(command_id) => {
                startup.pending_command_id = Some(command_id);
                info!(
                    event = "download.browser_startup.main_window_minimize_requested",
                    correlation_id = %startup.correlation_id,
                    %command_id,
                    window_handle = startup.window_handle,
                    elapsed_ms = startup.started_at.elapsed().as_millis(),
                    "desktop requested a minimized browser-startup main window"
                );
                cx.spawn(async move |this, cx| {
                    cx.background_executor()
                        .timer(std::time::Duration::from_secs(5))
                        .await;
                    let _ = this.update(cx, |dispatcher, _| {
                        dispatcher.browser_startup_minimize_timed_out(command_id);
                    });
                })
                .detach();
            }
            Err(error) => {
                let startup = self
                    .browser_startup_window
                    .take()
                    .expect("startup window was checked above");
                warn!(
                    event = "download.browser_startup.main_window_minimize_failed",
                    success = false,
                    correlation_id = %startup.correlation_id,
                    window_handle = startup.window_handle,
                    %error,
                    elapsed_ms = startup.started_at.elapsed().as_millis(),
                    "desktop could not queue the browser-startup main-window minimize request"
                );
            }
        }
    }

    fn browser_startup_minimize_timed_out(&mut self, command_id: Uuid) {
        let Some(startup) = self.browser_startup_window.as_ref() else {
            return;
        };
        if startup.pending_command_id != Some(command_id) {
            return;
        }
        let startup = self
            .browser_startup_window
            .take()
            .expect("startup window was checked above");
        warn!(
            event = "download.browser_startup.main_window_minimize_timeout",
            success = false,
            correlation_id = %startup.correlation_id,
            %command_id,
            window_handle = startup.window_handle,
            elapsed_ms = startup.started_at.elapsed().as_millis(),
            "Windows did not confirm browser-startup main-window minimization in time"
        );
    }

    fn forward_to_applications(&self, notification: PlatformNotification, cx: &mut Context<Self>) {
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

pub(crate) struct BrowserStartupWindow {
    window_handle: i64,
    correlation_id: Uuid,
    started_at: Instant,
    pending_command_id: Option<Uuid>,
}

impl BrowserStartupWindow {
    pub(crate) fn new(window_handle: i64, correlation_id: Uuid, started_at: Instant) -> Self {
        Self {
            window_handle,
            correlation_id,
            started_at,
            pending_command_id: None,
        }
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
