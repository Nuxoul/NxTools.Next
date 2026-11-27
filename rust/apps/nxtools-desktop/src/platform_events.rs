use gpui_kit::{Context, Entity, Global, Task, WeakEntity};
use nxtools_ipc::{PlatformBridge, PlatformNotification};
use nxtools_protocol::{
    ActivateWindowCommand, PlatformCommand, PlatformEvent, SetWindowMinimizedCommand,
    SetWindowVisibilityCommand, TrayAction,
};
use std::{rc::Rc, time::Instant};
use tracing::{info, warn};
use uuid::Uuid;

use crate::clipboard::ClipboardApplication;
use crate::downloads::DownloadApplication;
use crate::network_settings::NetworkSettingsApplication;
use crate::quick_launch::QuickLaunchApplication;

mod appearance;
mod main_preparation;
use crate::ui::motion::UiMotionPreference;
pub(crate) use main_preparation::MainWindowPreparation;
pub(crate) struct PlatformEventDispatcher {
    platform: Rc<PlatformBridge>,
    browser_startup_window: Option<BrowserStartupWindow>,
    main_window_handle: i64,
    global_settings: Entity<NetworkSettingsApplication>,
    clipboard: WeakEntity<ClipboardApplication>,
    // The dispatcher is the lifetime owner for background modules that do not have another
    // application-global owner. Keeping only a WeakEntity here drops quick launch before the
    // platform Connected notification can configure its summon gesture.
    quick_launch: Entity<QuickLaunchApplication>,
    downloads: Entity<DownloadApplication>,
    pending_tray_restore: Option<PendingTrayRestore>,
    silent_startup_wait: Option<SilentStartupWait>,
    main_preparation: Option<MainWindowPreparation>,
    main_preparation_failed: bool,
    _task: Task<()>,
}

struct SilentStartupWait {
    correlation_id: Uuid,
    started_at: Instant,
}

struct PendingTrayRestore {
    correlation_id: Uuid,
    visibility_command_id: Uuid,
    activation_command_id: Uuid,
    visibility_completed: bool,
    activation_completed: bool,
    started_at: Instant,
}

pub(crate) struct PlatformApplicationTargets {
    pub(crate) clipboard: WeakEntity<ClipboardApplication>,
    pub(crate) quick_launch: Entity<QuickLaunchApplication>,
    pub(crate) downloads: Entity<DownloadApplication>,
    pub(crate) global_settings: Entity<NetworkSettingsApplication>,
}

impl PlatformEventDispatcher {
    pub(crate) fn new(
        platform: Rc<PlatformBridge>,
        browser_startup_window: Option<BrowserStartupWindow>,
        main_window_handle: i64,
        hidden_startup: bool,
        targets: PlatformApplicationTargets,
        main_preparation: Option<MainWindowPreparation>,
        cx: &mut Context<Self>,
    ) -> Self {
        let receiver = platform.notification_receiver();
        let task = cx.spawn(async move |this, cx| {
            while let Ok(notification) = receiver.recv().await {
                let Some(this) = this.upgrade() else {
                    return;
                };
                this.update(cx, |dispatcher, cx| dispatcher.forward(notification, cx));
            }
        });
        cx.observe_global::<crate::ui::material::UiMaterialCapabilities>(|_, cx| {
            let dispatcher = cx.entity().downgrade();
            cx.defer(move |cx| {
                let _ =
                    dispatcher.update(cx, |dispatcher, cx| dispatcher.advance_main_material(cx));
            });
        })
        .detach();
        let silent_startup_wait = hidden_startup.then(|| {
            let wait = SilentStartupWait {
                correlation_id: Uuid::new_v4(),
                started_at: Instant::now(),
            };
            info!(
                event = "product.shell.silent_startup_started",
                correlation_id = %wait.correlation_id,
                "silent startup is waiting for the Windows shell adapter"
            );
            cx.spawn(async move |this, cx| {
                cx.background_executor()
                    .timer(std::time::Duration::from_secs(10))
                    .await;
                let _ = this.update(cx, |dispatcher, cx| {
                    dispatcher.silent_startup_timed_out(cx);
                });
            })
            .detach();
            wait
        });
        let dispatcher = Self {
            platform,
            browser_startup_window,
            main_window_handle,
            global_settings: targets.global_settings,
            clipboard: targets.clipboard,
            quick_launch: targets.quick_launch,
            downloads: targets.downloads,
            pending_tray_restore: None,
            silent_startup_wait,
            main_preparation,
            main_preparation_failed: false,
            _task: task,
        };
        dispatcher.schedule_main_preparation_deadline(cx);
        dispatcher
    }

    fn forward(&mut self, notification: PlatformNotification, cx: &mut Context<Self>) {
        if self.main_preparation_failed {
            return;
        }
        let appearance_event = self.apply_appearance_notification(&notification, cx);
        if crate::ui::material::runtime::handle_notification(&notification, cx) {
            return;
        }
        if self.intercept_main_preparation(&notification, cx) {
            return;
        }
        if appearance_event {
            self.forward_appearance_to_panels(notification, cx);
            return;
        }
        crate::archive::handle(notification.clone(), cx);
        crate::notifications::screen::handle(notification.clone(), cx);
        crate::notifications::copy::handle(notification.clone(), cx);
        crate::tools::floating_assistant::handle(notification.clone(), cx);
        crate::tools::raw_join::handle(notification.clone(), cx);
        match &notification {
            PlatformNotification::Connected => {
                if let Some(wait) = self.silent_startup_wait.take() {
                    info!(
                        event = "product.shell.silent_startup_completed",
                        correlation_id = %wait.correlation_id,
                        elapsed_ms = wait.started_at.elapsed().as_millis(),
                        "silent startup reached the Windows shell adapter"
                    );
                }
                self.begin_browser_startup_minimize(cx);
            }
            PlatformNotification::Disconnected {
                generation,
                pending_command_ids,
            } => {
                warn!(
                    event = "product.shell.platform_disconnected",
                    generation,
                    pending_command_count = pending_command_ids.len(),
                    "platform-dependent shell operations were invalidated"
                );
                // A disconnected native adapter cannot authorize animation on a later turn.
                cx.set_global(UiMotionPreference::default());
                cx.set_reduce_motion(true);
                crate::theme::apply_system_theme(true, cx);
                cx.refresh_windows();
                self.pending_tray_restore = None;
                if let Some(startup) = self.browser_startup_window.as_mut()
                    && startup
                        .pending_command_id
                        .is_some_and(|command_id| pending_command_ids.contains(&command_id))
                {
                    startup.pending_command_id = None;
                }
            }
            PlatformNotification::CommandResult {
                correlation_id,
                result,
            } => {
                if self.handle_tray_restore_result(*correlation_id, result, cx) {
                    self.forward_to_applications(notification, cx);
                    return;
                }
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
            PlatformNotification::Event(PlatformEvent::TrayActionRequested(request)) => {
                let started_at = Instant::now();
                info!(
                    event = "product.shell.tray_action_started",
                    correlation_id = %request.correlation_id,
                    action = match request.action {
                        TrayAction::Open => "open",
                        TrayAction::Exit => "exit",
                    },
                    "notification-area action accepted"
                );
                match request.action {
                    TrayAction::Open if self.main_window_handle != 0 => {
                        self.begin_main_window_restore(request.correlation_id, started_at, cx);
                    }
                    action if tray_action_exits(action) => {
                        info!(
                            event = "product.shell.tray_action_completed",
                            correlation_id = %request.correlation_id,
                            action = "exit",
                            elapsed_ms = started_at.elapsed().as_millis(),
                            "notification-area exit accepted"
                        );
                        cx.quit();
                    }
                    TrayAction::Open => warn!(
                        event = "product.shell.tray_action_failed",
                        correlation_id = %request.correlation_id,
                        action = "open",
                        failure_kind = "main_window_unavailable",
                        elapsed_ms = started_at.elapsed().as_millis(),
                        "notification-area restore has no main window"
                    ),
                    TrayAction::Exit => unreachable!("exit handled by the typed route above"),
                }
            }
            PlatformNotification::Event(_) => {}
        }
        self.forward_to_applications(notification, cx);
    }

    fn silent_startup_timed_out(&mut self, cx: &mut Context<Self>) {
        let Some(wait) = self.silent_startup_wait.take() else {
            return;
        };
        warn!(
            event = "product.shell.silent_startup_failed",
            correlation_id = %wait.correlation_id,
            failure_kind = "platform_connection_timeout",
            elapsed_ms = wait.started_at.elapsed().as_millis(),
            "silent startup could not reach the Windows shell adapter"
        );
        cx.quit();
    }

    fn begin_main_window_restore(
        &mut self,
        correlation_id: Uuid,
        started_at: Instant,
        cx: &mut Context<Self>,
    ) {
        if self.pending_tray_restore.is_some() {
            warn!(event = "product.shell.tray_action_failed", %correlation_id, action = "open",
                failure_kind = "operation_in_progress", elapsed_ms = started_at.elapsed().as_millis(),
                "a main window restore is already pending");
            return;
        }
        match enqueue_tray_restore(
            &self.platform,
            self.main_window_handle,
            correlation_id,
            started_at,
        ) {
            Ok(pending) => {
                self.pending_tray_restore = Some(pending);
                cx.spawn(async move |this, cx| {
                    cx.background_executor()
                        .timer(std::time::Duration::from_secs(5))
                        .await;
                    let _ = this.update(cx, |dispatcher, _| {
                        dispatcher.tray_restore_timed_out(correlation_id)
                    });
                })
                .detach();
            }
            Err(_) => {
                warn!(event = "product.shell.tray_action_failed", %correlation_id, action = "open",
                failure_kind = "platform_enqueue", elapsed_ms = started_at.elapsed().as_millis(),
                "main window restore commands could not be queued")
            }
        }
    }

    fn handle_tray_restore_result(
        &mut self,
        command_id: Uuid,
        result: &nxtools_protocol::PlatformCommandResult,
        _cx: &mut Context<Self>,
    ) -> bool {
        match advance_tray_restore(&mut self.pending_tray_restore, command_id, result) {
            TrayRestoreTransition::Unmatched => return false,
            TrayRestoreTransition::Pending => {}
            TrayRestoreTransition::Failed(pending) => {
                warn!(
                    event = "product.shell.tray_action_failed",
                    correlation_id = %pending.correlation_id,
                    action = "open",
                    failure_kind = "platform_result",
                    native_error_code = result.native_error_code,
                    elapsed_ms = pending.started_at.elapsed().as_millis(),
                    "Windows rejected notification-area restore"
                );
            }
            TrayRestoreTransition::Completed(pending) => {
                info!(
                    event = "product.shell.tray_action_completed",
                    correlation_id = %pending.correlation_id,
                    action = "open",
                    elapsed_ms = pending.started_at.elapsed().as_millis(),
                    "Windows restored the main window from the notification area"
                );
            }
        }
        true
    }

    fn tray_restore_timed_out(&mut self, correlation_id: Uuid) {
        if self
            .pending_tray_restore
            .as_ref()
            .is_none_or(|pending| pending.correlation_id != correlation_id)
        {
            return;
        }
        let pending = self.pending_tray_restore.take().expect("pending restore");
        warn!(
            event = "product.shell.tray_action_failed",
            correlation_id = %pending.correlation_id,
            action = "open",
            failure_kind = "platform_timeout",
            elapsed_ms = pending.started_at.elapsed().as_millis(),
            "Windows did not confirm notification-area restore in time"
        );
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
        crate::tools::markdown_reader::platform_notification(&notification, cx);
        crate::ui::components::native_context_menu::handle_notification(&notification, cx);
        let extension = cx
            .try_global::<crate::extensions::ExtensionApplicationGlobal>()
            .map(|global| global.0.clone());
        if let Some(extension) = extension {
            let _ = extension.update(cx, |app, cx| {
                app.handle_platform_notification(notification.clone(), cx)
            });
        }
        let _ = self.clipboard.update(cx, |application, cx| {
            application.handle_platform_notification(notification.clone(), cx);
        });
        self.quick_launch.update(cx, |application, cx| {
            application.handle_platform_notification(notification.clone(), cx);
        });
        self.downloads.update(cx, |application, cx| {
            application.handle_platform_notification(notification.clone(), cx);
        });
        self.global_settings.update(cx, |application, cx| {
            application.handle_platform_notification(notification, cx);
        });
    }
}

enum TrayRestoreTransition {
    Unmatched,
    Pending,
    Completed(PendingTrayRestore),
    Failed(PendingTrayRestore),
}

fn advance_tray_restore(
    pending: &mut Option<PendingTrayRestore>,
    command_id: Uuid,
    result: &nxtools_protocol::PlatformCommandResult,
) -> TrayRestoreTransition {
    let Some(current) = pending.as_mut() else {
        return TrayRestoreTransition::Unmatched;
    };
    let matched = if current.visibility_command_id == command_id {
        current.visibility_completed = result.success;
        true
    } else if current.activation_command_id == command_id {
        current.activation_completed = result.success;
        true
    } else {
        false
    };
    if !matched {
        return TrayRestoreTransition::Unmatched;
    }
    if !result.success {
        return TrayRestoreTransition::Failed(pending.take().expect("matched restore"));
    }
    if current.visibility_completed && current.activation_completed {
        TrayRestoreTransition::Completed(pending.take().expect("completed restore"))
    } else {
        TrayRestoreTransition::Pending
    }
}

fn enqueue_tray_restore(
    platform: &PlatformBridge,
    window_handle: i64,
    correlation_id: Uuid,
    started_at: Instant,
) -> Result<PendingTrayRestore, nxtools_ipc::PlatformBridgeError> {
    let visibility_command_id = platform.send_command_correlated(
        PlatformCommand::SetWindowVisibility(SetWindowVisibilityCommand {
            window_handle,
            visible: true,
        }),
        correlation_id,
    )?;
    let activation_command_id = platform.send_command_correlated(
        PlatformCommand::ActivateWindow(ActivateWindowCommand { window_handle }),
        correlation_id,
    )?;
    Ok(PendingTrayRestore {
        correlation_id,
        visibility_command_id,
        activation_command_id,
        visibility_completed: false,
        activation_completed: false,
        started_at,
    })
}

const fn tray_action_exits(action: TrayAction) -> bool {
    matches!(action, TrayAction::Exit)
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

pub(crate) fn restore_main_window(correlation_id: Uuid, started: Instant, cx: &mut gpui_kit::App) {
    if let Some(dispatcher) = cx
        .try_global::<PlatformEventDispatcherGlobal>()
        .map(|global| global._dispatcher.clone())
    {
        dispatcher.update(cx, |dispatcher, cx| {
            dispatcher.begin_main_window_restore(correlation_id, started, cx)
        });
    }
}

#[cfg(test)]
mod tray_tests {
    use super::*;
    use nxtools_ipc::platform_bridge;
    use nxtools_protocol::{Message, PlatformCommandResult};

    #[test]
    fn tray_open_routes_visibility_and_activation_to_the_original_window() {
        let (bridge, endpoint) = platform_bridge(4);
        endpoint.set_connected_for_contract_test(true);
        let correlation_id = Uuid::new_v4();
        let pending = enqueue_tray_restore(&bridge, 4242, correlation_id, Instant::now())
            .expect("route tray restore");

        let visible = endpoint
            .try_receive_command_for_contract_test()
            .expect("visibility command");
        let activate = endpoint
            .try_receive_command_for_contract_test()
            .expect("activation command");
        assert_eq!(visible.correlation_id, Some(correlation_id));
        assert_eq!(activate.correlation_id, Some(correlation_id));
        assert!(matches!(
            visible.message,
            Message::PlatformCommand(PlatformCommand::SetWindowVisibility(ref command))
                if command.window_handle == 4242 && command.visible
        ));
        assert!(matches!(
            activate.message,
            Message::PlatformCommand(PlatformCommand::ActivateWindow(ref command))
                if command.window_handle == 4242
        ));
        assert_eq!(pending.visibility_command_id, visible.id);
        assert_eq!(pending.activation_command_id, activate.id);

        let mut pending = Some(pending);
        let success = PlatformCommandResult {
            success: true,
            error: None,
            native_error_code: None,
            data: None,
        };
        assert!(matches!(
            advance_tray_restore(&mut pending, visible.id, &success),
            TrayRestoreTransition::Pending
        ));
        assert!(matches!(
            advance_tray_restore(&mut pending, activate.id, &success),
            TrayRestoreTransition::Completed(completed)
                if completed.correlation_id == correlation_id
        ));
        assert!(pending.is_none());
        assert!(tray_action_exits(TrayAction::Exit));
        assert!(!tray_action_exits(TrayAction::Open));
    }
}
