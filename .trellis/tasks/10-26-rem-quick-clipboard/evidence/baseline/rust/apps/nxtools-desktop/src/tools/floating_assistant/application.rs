use super::{
    lifecycle::{Lifecycle, Phase},
    media,
    view::FloatingView,
};
use crate::{
    downloads::DownloadApplication,
    notifications::{AppNotification, NotificationCenter, NotificationLevel},
    quick_launch::QuickLaunchApplication,
};
use gpui_kit::{
    AnyWindowHandle, AppContext, Bounds, Context, Entity, Subscription, WeakEntity,
    WindowBackgroundAppearance, WindowBounds, WindowKind, WindowOptions, point, prelude::*, px,
    size,
};
use nxtools_core::{AppConfig, FloatingAssistantSettings};
use nxtools_ipc::{PlatformBridge, PlatformNotification};
use nxtools_protocol::{
    PlatformCommand, PlatformEvent,
    floating_assistant::{
        FloatingAssistantBounds, FloatingAssistantCommand, FloatingAssistantEvent,
        FloatingAssistantEventKind, FloatingAssistantOperation,
    },
};
use std::{
    path::PathBuf,
    rc::Rc,
    time::{Duration, Instant},
};
use uuid::Uuid;

struct Surface {
    window: AnyWindowHandle,
    view: Entity<FloatingView>,
    hwnd: i64,
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct DownloadProjection {
    pub(crate) downloading: usize,
    pub(crate) speed: u64,
    pub(crate) active_count: usize,
    pub(crate) progress: Option<f32>,
}

pub(crate) struct FloatingAssistantApplication {
    pub(super) platform: Rc<PlatformBridge>,
    settings: FloatingAssistantSettings,
    config_path: PathBuf,
    pub(super) notifications: Entity<NotificationCenter>,
    pub(super) source: AnyWindowHandle,
    pub(super) source_hwnd: i64,
    pub(super) downloads: WeakEntity<DownloadApplication>,
    pub(super) quick_launch: WeakEntity<QuickLaunchApplication>,
    surface: Option<Surface>,
    pub(super) session: Uuid,
    pub(super) generation: u64,
    lifecycle: Lifecycle,
    bounds: Option<FloatingAssistantBounds>,
    dirty: bool,
    hovered: bool,
    edge_hidden: bool,
    fullscreen: bool,
    last_event_sequence: u64,
    accepted_drop_ids: std::collections::VecDeque<Uuid>,
    error: Option<String>,
    picker: Option<Uuid>,
    decode_pending: bool,
    decode_queued: bool,
    asset_revision: u64,
    loaded_asset: Option<(Option<String>, u32)>,
    download_projection: DownloadProjection,
    pub(super) pending_actions: std::collections::HashMap<Uuid, super::actions::PendingAction>,
    pub(super) foreground: nxtools_protocol::ForegroundContext,
    _subscriptions: Vec<Subscription>,
}
impl FloatingAssistantApplication {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        platform: Rc<PlatformBridge>,
        config: AppConfig,
        config_path: PathBuf,
        notifications: Entity<NotificationCenter>,
        source: AnyWindowHandle,
        downloads: WeakEntity<DownloadApplication>,
        quick_launch: WeakEntity<QuickLaunchApplication>,
        cx: &mut Context<Self>,
    ) -> Self {
        let source_hwnd = source
            .update(cx, |_, window, _| crate::ui::native_window_handle(window))
            .ok()
            .flatten()
            .unwrap_or_default();
        let mut subscriptions = Vec::new();
        if let Some(downloads) = downloads.upgrade() {
            subscriptions.push(cx.observe(&downloads, |_, _, cx| {
                let owner = cx.entity().downgrade();
                cx.defer(move |cx| {
                    let _ = owner.update(cx, |owner, cx| owner.refresh_downloads(cx));
                });
            }));
        }
        Self {
            platform,
            settings: config.floating_assistant,
            config_path,
            notifications,
            source,
            source_hwnd,
            downloads,
            quick_launch,
            surface: None,
            session: Uuid::new_v4(),
            generation: 1,
            lifecycle: Lifecycle::default(),
            bounds: None,
            dirty: false,
            hovered: false,
            edge_hidden: false,
            fullscreen: false,
            last_event_sequence: 0,
            accepted_drop_ids: std::collections::VecDeque::new(),
            error: None,
            picker: None,
            decode_pending: false,
            decode_queued: false,
            asset_revision: 0,
            loaded_asset: None,
            download_projection: DownloadProjection::default(),
            pending_actions: Default::default(),
            foreground: nxtools_protocol::ForegroundContext {
                window_handle: source_hwnd,
                focus_window_handle: 0,
                process_id: std::process::id(),
                process_name: String::new(),
                window_title: String::new(),
            },
            _subscriptions: subscriptions,
        }
    }
    pub(crate) fn settings(&self) -> &FloatingAssistantSettings {
        &self.settings
    }
    pub(crate) fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }
    pub(crate) fn busy(&self) -> bool {
        self.picker.is_some()
    }
    pub(crate) fn start(&mut self, cx: &mut Context<Self>) {
        self.refresh_downloads(cx);
        self.sync(cx);
    }
    pub(crate) fn retry(&mut self, cx: &mut Context<Self>) {
        self.error = None;
        self.cancel_actions(true);
        self.loaded_asset = None;
        self.lifecycle.pending = None;
        self.lifecycle.visible = false;
        self.update_view(cx);
        if self.surface.is_some() {
            self.send(FloatingAssistantOperation::Retire, Phase::Retire, cx);
            cx.notify();
            return;
        }
        self.load_asset(cx);
        self.sync(cx);
        cx.notify();
    }
    pub(crate) fn change_settings(
        &mut self,
        mut settings: FloatingAssistantSettings,
        cx: &mut Context<Self>,
    ) {
        settings.normalize();
        if self.settings == settings {
            return;
        }
        let id = Uuid::new_v4();
        let started = Instant::now();
        tracing::info!(event="floating_assistant.settings.accepted",correlation_id=%id,"floating settings update admitted");
        match AppConfig::update_floating_assistant_settings(&self.config_path, settings) {
            Ok(config) => {
                let reload = self.settings.image_path != config.floating_assistant.image_path
                    || self.settings.size != config.floating_assistant.size
                    || (!self.settings.enabled && config.floating_assistant.enabled);
                self.settings = config.floating_assistant;
                self.error = None;
                if !self.settings.enabled {
                    self.asset_revision += 1;
                    self.loaded_asset = None;
                    self.decode_queued = false;
                    self.cancel_actions(false);
                }
                if self.settings.click_through {
                    self.cancel_actions(false);
                    self.edge_hidden = false;
                }
                if reload {
                    self.asset_revision += 1;
                    self.loaded_asset = None;
                    if self.settings.enabled {
                        self.load_asset(cx);
                    }
                }
                self.dirty = true;
                self.sync(cx);
                self.update_view(cx);
                tracing::info!(event="floating_assistant.settings.terminal",correlation_id=%id,success=true,elapsed_ms=started.elapsed().as_millis(),"floating settings committed");
            }
            Err(_) => {
                self.fail("悬浮窗配置保存失败，请检查配置文件后重试。", cx);
                tracing::warn!(event="floating_assistant.settings.terminal",correlation_id=%id,success=false,elapsed_ms=started.elapsed().as_millis(),"floating settings commit failed");
            }
        }
        cx.notify();
    }
    pub(crate) fn reset_image(&mut self, cx: &mut Context<Self>) {
        let mut settings = self.settings.clone();
        settings.image_path = None;
        self.change_settings(settings, cx);
    }
    pub(crate) fn pick_image(&mut self, cx: &mut Context<Self>) {
        if self.picker.is_some() {
            return;
        }
        let id = Uuid::new_v4();
        self.picker = Some(id);
        tracing::info!(event="floating_assistant.picker.accepted", correlation_id=%id, session_id=%self.session, generation=self.generation);
        cx.notify();
        let command = self.command(FloatingAssistantOperation::PickImage, None);
        if self
            .platform
            .send_command_with_id(PlatformCommand::FloatingAssistant(command), id)
            .is_err()
        {
            self.picker = None;
            self.fail("图片选择器当前不可用，请确认 Windows 平台连接后重试。", cx);
        }
    }
    fn desired_visible(&self) -> bool {
        self.settings.enabled && !(self.fullscreen && self.settings.hide_in_fullscreen)
    }
    fn command(
        &self,
        operation: FloatingAssistantOperation,
        drop_id: Option<Uuid>,
    ) -> FloatingAssistantCommand {
        let (mut x, mut y) = (self.settings.position.x, self.settings.position.y);
        if self.edge_hidden
            && let Some(bounds) = &self.bounds
        {
            if x <= bounds.work_left {
                x = bounds.work_left - bounds.width as i32 + 8;
            } else if x + bounds.width as i32 >= bounds.work_right {
                x = bounds.work_right - 8;
            } else if y <= bounds.work_top {
                y = bounds.work_top - bounds.height as i32 + 8;
            } else if y + bounds.height as i32 >= bounds.work_bottom {
                y = bounds.work_bottom - 8;
            }
        }
        FloatingAssistantCommand {
            version: 1,
            session_id: self.session,
            generation: self.generation,
            window_handle: self.surface.as_ref().map_or(0, |surface| surface.hwnd),
            source_window_handle: self.source_hwnd,
            source_process_id: std::process::id(),
            operation,
            x,
            y,
            size: self.settings.size,
            opacity: if self.hovered {
                self.settings.hover_opacity
            } else {
                self.settings.base_opacity
            } as u8,
            click_through: self.settings.click_through,
            position_locked: self.settings.position_locked,
            drop_id,
            edge_hidden: self.edge_hidden,
        }
    }
    fn send(
        &mut self,
        operation: FloatingAssistantOperation,
        phase: Phase,
        cx: &mut Context<Self>,
    ) {
        let id = Uuid::new_v4();
        self.lifecycle.begin(id, phase);
        let command = self.command(operation, None);
        if self
            .platform
            .send_command_with_id(PlatformCommand::FloatingAssistant(command), id)
            .is_err()
        {
            self.lifecycle.pending = None;
            self.fail("Windows 平台未连接，悬浮窗尚未应用；请重试。", cx);
            return;
        }
        tracing::info!(event="floating_assistant.window.accepted",correlation_id=%id,session_id=%self.session,generation=self.generation,?phase,"floating native operation admitted");
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(Duration::from_secs(5)).await;
            let _ = this.update(cx, |owner, cx| {
                if owner
                    .lifecycle
                    .pending
                    .is_some_and(|(pending, _)| pending == id)
                {
                    owner.lifecycle.pending = None;
                    owner.fail("悬浮窗的 Windows 应用操作超时，请重试。", cx);
                    owner.emergency_hide(cx);
                }
            });
        })
        .detach();
    }
    fn sync(&mut self, cx: &mut Context<Self>) {
        if !self.platform.is_connected() {
            return;
        }
        if self.lifecycle.pending.is_some() {
            self.dirty = true;
            return;
        }
        if !self.desired_visible() {
            self.dirty = false;
            if self.surface.is_some() && self.lifecycle.prepared && self.lifecycle.visible {
                self.send(FloatingAssistantOperation::Hide, Phase::Hide, cx);
            }
            self.update_view(cx);
            return;
        }
        if self.surface.is_none() && !self.open_surface(cx) {
            return;
        }
        if !self.lifecycle.prepared {
            self.dirty = false;
            self.send(FloatingAssistantOperation::Prepare, Phase::Prepare, cx);
        } else if self.dirty {
            self.dirty = false;
            self.send(FloatingAssistantOperation::Update, Phase::Update, cx);
        } else if !self.lifecycle.visible {
            self.send(FloatingAssistantOperation::Show, Phase::Show, cx);
        }
    }
    fn open_surface(&mut self, cx: &mut Context<Self>) -> bool {
        let owner = cx.entity().downgrade();
        let mut view = None;
        let mut hwnd = 0;
        let side = self.settings.size as f32;
        let opened = cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                    point(px(0.0), px(0.0)),
                    size(px(side), px(side)),
                ))),
                titlebar: None,
                kind: WindowKind::PopUp,
                show: false,
                focus: false,
                is_movable: false,
                is_resizable: false,
                is_minimizable: false,
                window_background: WindowBackgroundAppearance::Transparent,
                inactive_frame_interval: None,
                ..Default::default()
            },
            |window, cx| {
                window.set_window_title("NxTools.FloatingAssistant");
                hwnd = crate::ui::native_window_handle(window).unwrap_or_default();
                #[cfg(test)]
                if hwnd == 0 {
                    hwnd = -40;
                }
                let content = cx.new(|_| FloatingView::new(owner));
                view = Some(content.clone());
                cx.new(|cx| {
                    gpui_kit::base::Root::new(content, window, cx).bg(gpui_kit::transparent_black())
                })
            },
        );
        match (opened, view) {
            (Ok(window), Some(view)) if hwnd != 0 => {
                self.surface = Some(Surface {
                    window: window.into(),
                    view,
                    hwnd,
                });
                self.load_asset(cx);
                true
            }
            (Ok(window), _) => {
                let _ = window.update(cx, |_, window, _| window.remove_window());
                self.fail("无法创建悬浮窗，请重试。", cx);
                false
            }
            _ => {
                self.fail("无法创建悬浮窗，请重试。", cx);
                false
            }
        }
    }
    fn load_asset(&mut self, cx: &mut Context<Self>) {
        let Some(view) = self.surface.as_ref().map(|surface| surface.view.clone()) else {
            return;
        };
        if self.decode_pending {
            self.decode_queued = true;
            return;
        }
        let path = self.settings.image_path.clone();
        let target = (self.settings.size.saturating_mul(3)).min(900);
        if self.loaded_asset.as_ref() == Some(&(path.clone(), target)) {
            return;
        }
        self.loaded_asset = Some((path.clone(), target));
        self.asset_revision += 1;
        let revision = self.asset_revision;
        let fps = self.settings.frame_rate;
        let Some(path) = path else {
            cx.defer(move |cx| view.update(cx, |view, cx| view.replace(None, fps, cx)));
            return;
        };
        self.decode_pending = true;
        let started = Instant::now();
        let id = Uuid::new_v4();
        tracing::info!(event="floating_assistant.asset.start",correlation_id=%id,"floating image decode started");
        cx.spawn(async move |this, cx| {
            let result = cx.background_executor().spawn(async move { media::decode_file(&PathBuf::from(path), target) }).await;
            let _ = this.update(cx, |owner, cx| {
                owner.decode_pending = false;
                if revision == owner.asset_revision {
                    let success = result.is_ok();
                    match result {
                        Ok(media) => {
                            let view = view.clone(); let owner = cx.entity().downgrade();
                            cx.defer(move |cx| {
                                if owner.upgrade().is_some_and(|owner| owner.read(cx).asset_revision == revision) {
                                    view.update(cx, |view, cx| view.replace(Some(media), fps, cx));
                                }
                            });
                        }
                        Err(_) => owner.fail("所选图片无法读取、格式损坏或超过资源限制。可重新选择图片或恢复默认。", cx),
                    }
                    tracing::info!(event="floating_assistant.asset.terminal",correlation_id=%id,success,elapsed_ms=started.elapsed().as_millis(),"floating image decode terminated");
                }
                if owner.decode_queued { owner.decode_queued = false; owner.loaded_asset = None; owner.load_asset(cx); }
            });
        }).detach();
    }
    pub(crate) fn handle(&mut self, notification: PlatformNotification, cx: &mut Context<Self>) {
        match notification {
            PlatformNotification::Connected => {
                self.error = None;
                self.sync(cx);
            }
            PlatformNotification::Disconnected { .. } => {
                self.last_event_sequence = 0;
                self.lifecycle.retire();
                self.session = Uuid::new_v4();
                self.generation += 1;
                self.picker = None;
                self.pending_actions.clear();
                self.accepted_drop_ids.clear();
                self.error = Some("Windows 平台已断开，恢复连接后将重新准备悬浮窗。".into());
                self.update_view(cx);
                cx.notify();
            }
            PlatformNotification::CommandResult {
                correlation_id,
                result,
            } => {
                if self.picker == Some(correlation_id) {
                    self.picker = None;
                    tracing::info!(event="floating_assistant.picker.terminal", correlation_id=%correlation_id, success=result.success, native_error_code=?result.native_error_code);
                    if result.success {
                        let path = result
                            .data
                            .as_deref()
                            .and_then(|data| serde_json::from_str::<serde_json::Value>(data).ok())
                            .and_then(|value| {
                                value
                                    .get("selected_path")
                                    .and_then(|value| value.as_str())
                                    .map(str::to_owned)
                            });
                        if let Some(path) = path {
                            let mut settings = self.settings.clone();
                            settings.image_path = Some(path);
                            self.change_settings(settings, cx);
                        }
                    } else if result.error.as_deref() != Some("picker_retired") {
                        self.fail(
                            match result.error.as_deref() {
                                Some("picker_busy") => {
                                    "上一次图片选择仍在结束，请关闭原选择窗口后重试。"
                                }
                                Some("invalid_source") => {
                                    "图片选择来源窗口已失效，请重新打开设置后重试。"
                                }
                                _ => "Windows 图片选择器未能完成，请重试。",
                            },
                            cx,
                        );
                    }
                    cx.notify();
                    return;
                }
                if self.complete_action(correlation_id, &result, cx) {
                    return;
                }
                let Some(phase) = self.lifecycle.take(correlation_id) else {
                    return;
                };
                tracing::info!(event="floating_assistant.window.terminal",correlation_id=%correlation_id,success=result.success,native_error_code=?result.native_error_code,?phase,"floating native operation terminated");
                if !result.success {
                    self.fail("悬浮窗配置已保存，但 Windows 尚未应用。请重试。", cx);
                    self.emergency_hide(cx);
                    return;
                }
                match phase {
                    Phase::Prepare | Phase::Update => {
                        let bounds = result
                            .data
                            .as_deref()
                            .and_then(|data| {
                                serde_json::from_str::<FloatingAssistantBounds>(data).ok()
                            })
                            .filter(|bounds| {
                                bounds.width > 0
                                    && bounds.height > 0
                                    && bounds.width <= 3000
                                    && bounds.height <= 3000
                                    && (48..=960).contains(&bounds.dpi)
                            });
                        let Some(bounds) = bounds else {
                            self.fail("Windows 返回的悬浮窗尺寸无效，请重试。", cx);
                            self.emergency_hide(cx);
                            return;
                        };
                        self.lifecycle.prepared = true;
                        self.bounds = Some(bounds.clone());
                        if !self.edge_hidden && !self.dirty {
                            self.settings.position.x = bounds.x;
                            self.settings.position.y = bounds.y;
                        }
                        self.lifecycle.begin(correlation_id, Phase::Frame);
                        self.draw_frame(correlation_id, bounds, cx);
                    }
                    Phase::Show => {
                        self.lifecycle.visible = true;
                        self.update_view(cx);
                        self.sync(cx);
                    }
                    Phase::Hide => {
                        self.lifecycle.visible = false;
                        self.update_view(cx);
                        if self.desired_visible() {
                            self.sync(cx);
                        }
                    }
                    Phase::Retire => {
                        self.lifecycle.retire();
                        self.session = Uuid::new_v4();
                        self.generation += 1;
                        self.last_event_sequence = 0;
                        self.accepted_drop_ids.clear();
                        self.dirty = false;
                        self.load_asset(cx);
                        self.sync(cx);
                    }
                    Phase::Frame => {}
                }
            }
            PlatformNotification::Event(PlatformEvent::FloatingAssistant(event)) => {
                self.native_event(event, cx)
            }
            PlatformNotification::Event(PlatformEvent::ForegroundChanged(context)) => {
                if context.window_handle != self.source_hwnd
                    && self
                        .surface
                        .as_ref()
                        .is_none_or(|surface| surface.hwnd != context.window_handle)
                {
                    self.foreground = context;
                }
            }
            PlatformNotification::Event(PlatformEvent::SystemMotionPreferenceChanged(_)) => {
                self.update_view(cx)
            }
            _ => {}
        }
    }
    fn draw_frame(&mut self, id: Uuid, bounds: FloatingAssistantBounds, cx: &mut Context<Self>) {
        let Some(surface) = &self.surface else {
            return;
        };
        let view = surface.view.clone();
        let window = surface.window;
        let owner = cx.entity().downgrade();
        cx.defer(move |cx| {
            let _ = window.update(cx, |_, window, cx| {
                view.update(cx, |view, cx| {
                    view.expected_frame = Some((id, bounds.width, bounds.height));
                    cx.notify();
                });
                window.refresh();
                window.draw(cx).clear(cx);
                if view.read(cx).expected_frame.is_none() {
                    cx.defer(move |cx| {
                        let _ = owner.update(cx, |owner, cx| owner.frame_ready(id, cx));
                    });
                }
            });
        });
    }
    pub(super) fn frame_ready(&mut self, id: Uuid, cx: &mut Context<Self>) {
        if self.lifecycle.take(id) != Some(Phase::Frame) {
            return;
        }
        if !self.desired_visible() {
            self.send(FloatingAssistantOperation::Hide, Phase::Hide, cx);
        } else {
            self.sync(cx);
        }
    }
    fn native_event(&mut self, event: FloatingAssistantEvent, cx: &mut Context<Self>) {
        if !event.is_bounded()
            || event.session_id != self.session
            || event.generation != self.generation
            || self
                .surface
                .as_ref()
                .is_none_or(|surface| surface.hwnd != event.window_handle)
            || event.sequence <= self.last_event_sequence
        {
            return;
        }
        self.last_event_sequence = event.sequence;
        if event.kind == FloatingAssistantEventKind::Drop {
            let Some(drop_id) = event.drop_id.filter(|id| !id.is_nil()) else {
                return;
            };
            if self.accepted_drop_ids.contains(&drop_id) {
                return;
            }
            if self.accepted_drop_ids.len() == 64 {
                self.accepted_drop_ids.pop_front();
            }
            self.accepted_drop_ids.push_back(drop_id);
        }
        if self
            .lifecycle
            .pending
            .is_some_and(|(_, phase)| phase == Phase::Retire)
        {
            return;
        }
        match event.kind {
            FloatingAssistantEventKind::Environment => {
                self.fullscreen = event.fullscreen;
                self.edge_hidden = false;
                self.dirty = true;
                self.sync(cx);
                self.update_view(cx);
            }
            FloatingAssistantEventKind::HoverEnter => {
                self.hovered = true;
                self.edge_hidden = false;
                self.dirty = true;
                self.sync(cx);
                self.update_view(cx);
            }
            FloatingAssistantEventKind::HoverLeave => {
                self.hovered = false;
                self.edge_hidden = self.settings.auto_hide_at_edge
                    && !self.settings.click_through
                    && self.at_edge();
                self.dirty = true;
                self.sync(cx);
                self.update_view(cx);
            }
            FloatingAssistantEventKind::MoveCompleted => {
                let (x, y) = self.snap(event.x, event.y);
                let mut settings = self.settings.clone();
                settings.position.x = x;
                settings.position.y = y;
                settings.position.dpi = self.bounds.as_ref().map(|bounds| bounds.dpi);
                self.change_settings(settings, cx);
            }
            FloatingAssistantEventKind::Wheel if !self.settings.position_locked => {
                let mut settings = self.settings.clone();
                settings.size = (settings.size as i32 + if event.delta > 0 { 6 } else { -6 })
                    .clamp(20, 300) as u32;
                self.change_settings(settings, cx);
            }
            FloatingAssistantEventKind::Failed | FloatingAssistantEventKind::Retired => {
                self.lifecycle.retire();
                self.cancel_actions(true);
                self.fail("悬浮窗的原生输入层已退出，请重试。", cx);
                self.update_view(cx);
            }
            _ if self.settings.enabled
                && !self.settings.click_through
                && self.desired_visible() =>
            {
                self.dispatch_event(event, cx)
            }
            _ if event.kind == FloatingAssistantEventKind::Drop => self.release_drop(event.drop_id),
            _ => {}
        }
    }
    fn at_edge(&self) -> bool {
        self.bounds.as_ref().is_some_and(|bounds| {
            self.settings.position.x <= bounds.work_left
                || self.settings.position.x + bounds.width as i32 >= bounds.work_right
                || self.settings.position.y <= bounds.work_top
                || self.settings.position.y + bounds.height as i32 >= bounds.work_bottom
        })
    }
    fn snap(&self, x: i32, y: i32) -> (i32, i32) {
        let Some(bounds) = &self.bounds else {
            return (x, y);
        };
        let distance = (self.settings.edge_snap_distance * bounds.dpi / 96) as i32;
        let width = bounds.width as i32;
        let height = bounds.height as i32;
        let mut x = x.clamp(
            bounds.work_left,
            (bounds.work_right - width).max(bounds.work_left),
        );
        let mut y = y.clamp(
            bounds.work_top,
            (bounds.work_bottom - height).max(bounds.work_top),
        );
        if (x - bounds.work_left).abs() <= distance {
            x = bounds.work_left;
        } else if (x + width - bounds.work_right).abs() <= distance {
            x = bounds.work_right - width;
        }
        if (y - bounds.work_top).abs() <= distance {
            y = bounds.work_top;
        } else if (y + height - bounds.work_bottom).abs() <= distance {
            y = bounds.work_bottom - height;
        }
        (x, y)
    }
    fn refresh_downloads(&mut self, cx: &mut Context<Self>) {
        let next = self
            .downloads
            .upgrade()
            .map(|owner| owner.read(cx).floating_assistant_projection())
            .unwrap_or_default();
        if next != self.download_projection {
            self.download_projection = next;
            self.update_view(cx);
        }
    }
    fn update_view(&self, cx: &mut Context<Self>) {
        let Some(view) = self.surface.as_ref().map(|surface| surface.view.clone()) else {
            return;
        };
        let visible = self.lifecycle.visible && self.desired_visible() && !self.edge_hidden;
        let fps = self.settings.frame_rate;
        let sync = self.settings.synchronize_download_speed;
        let projection = self.download_projection;
        cx.defer(move |cx| {
            view.update(cx, |view, cx| {
                view.activity(
                    visible,
                    fps,
                    sync,
                    projection.downloading,
                    projection.speed,
                    projection.progress,
                    projection.active_count,
                    cx,
                )
            })
        });
    }
    pub(super) fn fail(&mut self, message: &str, cx: &mut Context<Self>) {
        self.error = Some(message.into());
        self.notice(message, true, cx);
        cx.notify();
    }
    pub(super) fn notice(&self, message: &str, failed: bool, cx: &mut Context<Self>) {
        let notifications = self.notifications.clone();
        let source = self.source;
        let id = Uuid::new_v4();
        let message = message.to_owned();
        cx.defer(move |cx| {
            notifications.update(cx, |center, cx| {
                center.push(
                    AppNotification::new(
                        format!("floating-{id}"),
                        if failed {
                            NotificationLevel::Error
                        } else {
                            NotificationLevel::Success
                        },
                        "悬浮窗",
                        message,
                    )
                    .in_window(source),
                    cx,
                )
            })
        });
    }
    fn emergency_hide(&mut self, cx: &mut Context<Self>) {
        self.lifecycle.visible = false;
        self.update_view(cx);
        if self.surface.is_some() {
            let _ = self
                .platform
                .send_command(PlatformCommand::FloatingAssistant(
                    self.command(FloatingAssistantOperation::Hide, None),
                ));
        }
    }
    pub(super) fn release_drop(&self, drop_id: Option<Uuid>) {
        if drop_id.is_some() {
            let _ = self
                .platform
                .send_command(PlatformCommand::FloatingAssistant(
                    self.command(FloatingAssistantOperation::ReleaseDrop, drop_id),
                ));
        }
    }
}
impl Drop for FloatingAssistantApplication {
    fn drop(&mut self) {
        if self.surface.is_some() {
            let _ = self
                .platform
                .send_command(PlatformCommand::FloatingAssistant(
                    self.command(FloatingAssistantOperation::Retire, None),
                ));
        }
    }
}

#[cfg(test)]
mod tests;
