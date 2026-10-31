//! Formal row gesture; sole BrowserModel selection, original bridge and native cleanup owner.
use super::*;
use gpui_kit::{Modifiers, MouseDownEvent, MouseMoveEvent, Point};
use nxtools_protocol::{
    PlatformCommandResult,
    archive_browser::{ArchiveBrowserContext, ArchiveEntryAction, ArchiveEntryCommand},
    archive_virtual_drag::{ArchiveVirtualDragCommand, ArchiveVirtualDragItem},
};
use std::{rc::Rc, time::Instant};

enum DragTarget {
    Selection(RowId),
    Tree(String),
}
struct Candidate {
    target: DragTarget,
    origin: Point<Pixels>,
    modifiers: Modifiers,
    snapshot: Uuid,
    generation: u64,
    epoch: u64,
}
struct Pending {
    operation: Uuid,
    request: Uuid,
    generation: u64,
    connection: u64,
    epoch: u64,
    cancelled: bool,
    started: Instant,
}
#[derive(Default)]
pub(super) struct DragState {
    pub(super) platform: Option<Rc<nxtools_ipc::PlatformBridge>>,
    browser: Option<ArchiveBrowserContext>,
    connection: u64,
    depth: u32,
    copy_motw: bool,
    candidate: Option<Candidate>,
    pending: Option<Pending>,
    suppress_click: bool,
    authorization: Option<Authorization>,
    #[cfg(test)]
    pub(super) source_hwnd: Option<i64>,
}
struct Authorization {
    command: ArchiveEntryCommand,
    receipt: Uuid,
    connection: u64,
    ready: Option<Instant>,
    waiting: bool,
}
impl ArchiveBrowser {
    fn notify_drag(
        &self,
        message: impl Into<gpui_kit::SharedString>,
        error: bool,
        cx: &mut Context<Self>,
    ) {
        let Some(center) = crate::notifications::current(cx) else {
            return;
        };
        let notification = crate::notifications::AppNotification::new(
            format!("archive-drag-{}", self.window_id),
            if error {
                crate::notifications::NotificationLevel::Error
            } else {
                crate::notifications::NotificationLevel::Info
            },
            if error {
                "拖出未完成"
            } else {
                "解锁拖出"
            },
            message,
        );
        cx.defer(move |cx| center.update(cx, |center, cx| center.push(notification, cx)));
    }
    #[cfg(test)]
    pub(crate) fn set_drag_hwnd_for_test(&mut self, hwnd: i64) {
        self.drag.source_hwnd = Some(hwnd);
    }
    pub(crate) fn configure_drag(
        &mut self,
        platform: Rc<nxtools_ipc::PlatformBridge>,
        browser: ArchiveBrowserContext,
        connection: u64,
        depth: u32,
        copy_motw: bool,
    ) {
        self.invalidate_drag();
        self.drag.platform = Some(platform);
        self.drag.browser = self
            .model
            .as_ref()
            .filter(|model| model.snapshot == browser.snapshot_id)
            .map(|_| browser);
        self.drag.depth = depth;
        self.drag.connection = connection;
        self.drag.copy_motw = copy_motw;
    }
    pub(super) fn drag_down(&mut self, row: RowId, snapshot: Option<Uuid>, event: &MouseDownEvent) {
        self.drag_down_target(DragTarget::Selection(row), snapshot, event);
    }
    pub(super) fn tree_drag_down(
        &mut self,
        path: String,
        snapshot: Option<Uuid>,
        event: &MouseDownEvent,
    ) {
        self.drag_down_target(DragTarget::Tree(path), snapshot, event);
    }
    pub(super) fn tree_drag_click(&self) -> bool {
        self.drag.suppress_click
    }
    pub(super) fn tree_drag_reset(&mut self) {
        self.drag.candidate = None;
        self.drag.suppress_click = false;
    }
    fn drag_down_target(
        &mut self,
        target: DragTarget,
        snapshot: Option<Uuid>,
        event: &MouseDownEvent,
    ) {
        self.drag.candidate = None;
        self.drag.suppress_click = false;
        if self.retired
            || self.drag.pending.is_some()
            || self.open_pending
            || self
                .drag
                .authorization
                .as_ref()
                .is_some_and(|authorization| authorization.waiting)
            || event.click_count != 1
            || event.modifiers.alt
            || event.modifiers.platform
            || !matches!(self.status, BrowserStatus::Ready)
        {
            return;
        }
        let Some(model) = self.model.as_ref().filter(|model| {
            Some(model.snapshot) == snapshot
                && match &target {
                    DragTarget::Selection(row) => {
                        model.visible().iter().any(|item| &item.id == row)
                    }
                    // The rendered Tree supplies this path from the current snapshot; resolve it
                    // again in the shared bounded freezer at threshold, not by cloning all folders.
                    DragTarget::Tree(_) => true,
                }
        }) else {
            return;
        };
        self.drag.candidate = Some(Candidate {
            target,
            origin: event.position,
            modifiers: event.modifiers,
            snapshot: model.snapshot,
            generation: self.generation,
            epoch: self.projection_epoch,
        });
    }
    pub(super) fn drag_up(&mut self) {
        self.drag.candidate = None;
    }
    pub(super) fn drag_click_suppressed(&self) -> bool {
        self.drag.suppress_click
    }
    pub(super) fn drag_move(
        &mut self,
        event: &MouseMoveEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(candidate) = self.drag.candidate.as_ref() else {
            return;
        };
        if event.pressed_button != Some(MouseButton::Left) {
            self.drag.candidate = None;
            return;
        }
        if (f32::from(event.position.x - candidate.origin.x)).abs() < 4.
            && (f32::from(event.position.y - candidate.origin.y)).abs() < 4.
        {
            return;
        }
        let candidate = self.drag.candidate.take().expect("current candidate");
        if self.retired
            || self.generation != candidate.generation
            || self.projection_epoch != candidate.epoch
            || !matches!(self.status, BrowserStatus::Ready)
            || self.drag.pending.is_some()
            || self
                .model
                .as_ref()
                .is_none_or(|model| model.snapshot != candidate.snapshot)
            || self
                .drag
                .platform
                .as_ref()
                .is_none_or(|platform| !platform.is_connected())
        {
            return;
        }
        self.drag.suppress_click = true;
        let tree_path = match candidate.target {
            DragTarget::Selection(row) => {
                if let Some(model) = &mut self.model {
                    // Keep original right-list multi-selection and Click ownership.
                    if model.selected.contains(&row) {
                        model.focused = Some(row);
                    } else {
                        model.select(row, candidate.modifiers.control, candidate.modifiers.shift);
                    }
                }
                self.list_focus.focus(window, cx);
                self.update_summary();
                None
            }
            DragTarget::Tree(path) => Some(path),
        };
        #[cfg(not(test))]
        let source_hwnd = crate::ui::native_window_handle(window);
        #[cfg(test)]
        let source_hwnd = self.drag.source_hwnd;
        match self.drag_command(source_hwnd.unwrap_or(0), tree_path.as_deref()) {
            Err(reason) => self.notify_drag(reason, true, cx),
            Ok(command) => {
                let request = Uuid::new_v4();
                let operation = command.operation_id;
                self.drag.pending = Some(Pending {
                    operation,
                    request,
                    generation: self.generation,
                    connection: self.drag.connection,
                    epoch: self.projection_epoch,
                    cancelled: false,
                    started: Instant::now(),
                });
                // Still-pressed MOVE callback: no await/defer/application entity update here.
                let submitted = self.drag.platform.as_ref().is_some_and(|platform| {
                    platform
                        .send_archive_virtual_drag(command, request, self.drag.connection)
                        .is_ok()
                });
                if !submitted {
                    self.drag.pending = None;
                    self.notify_drag("拖出未提交：原生连接不可用，请重新连接后刷新。", true, cx);
                }
                tracing::info!(event="archive.virtual_drag.submitted",correlation_id=%operation,submitted,"archive virtual gesture submission");
            }
        }
        cx.notify();
    }
    fn drag_command(
        &self,
        hwnd: i64,
        tree_path: Option<&str>,
    ) -> Result<ArchiveVirtualDragCommand, String> {
        let model = self.model.as_ref().ok_or("快照不可用，请刷新后再拖出。")?;
        let browser = self
            .drag
            .browser
            .as_ref()
            .filter(|browser| browser.snapshot_id == model.snapshot)
            .ok_or("源身份已失效，请刷新后再拖出。")?;
        let authorized = self.drag.authorization.as_ref().filter(|authorization| {
            authorization
                .ready
                .is_some_and(|at| at.elapsed().as_secs() < 110)
        });
        let freeze = |authorized| {
            if let Some(path) = tree_path {
                let display_name = self
                    .path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .ok_or("归档显示名称不满足Windows安全命名限制；未拖出任何项目。")?;
                model.freeze_drag_tree(path, display_name, authorized)
            } else if authorized {
                model.freeze_drag_selection_authorized(true)
            } else {
                model.freeze_drag_selection()
            }
        };
        let frozen = freeze(authorized.is_some())?;
        let mut items = Vec::with_capacity(frozen.len());
        for row in frozen {
            items.push(ArchiveVirtualDragItem {
                name: row.name,
                size: row.size,
                directory: row.directory,
                entry: ArchiveEntryCommand {
                    version: 1,
                    operation_id: Uuid::new_v4(),
                    session_id: self.window_id,
                    generation: self.generation,
                    action: ArchiveEntryAction::Materialize,
                    path: Some(self.path.to_string_lossy().into_owned()),
                    codepage: self.codepage,
                    ordinal: row.ordinal,
                    browser: Some(browser.clone()),
                    lease_id: None,
                    depth: self.drag.depth,
                    copy_motw: Some(self.drag.copy_motw),
                    password_interaction: false,
                    temp_policy: None,
                },
            });
        }
        if hwnd <= 0 {
            return Err("来源窗口不可用，未开始拖出。".into());
        }
        let command = ArchiveVirtualDragCommand {
            version: 1,
            authorization_id: authorized.map(|authorization| authorization.command.operation_id),
            operation_id: Uuid::new_v4(),
            session_id: self.window_id,
            generation: self.generation,
            action: "start".into(),
            source_window_handle: hwnd,
            items: Some(items),
        };
        command
            .validate()
            .map_err(|_| "原生拖出请求超过传输帧限制或源绑定已失效；未拖出任何项目。")?;
        Ok(command)
    }
    pub(crate) fn invalidate_drag(&mut self) {
        self.revoke_drag_authorization();
        self.suspend_tab_gestures();
    }
    pub(crate) fn suspend_tab_gestures(&mut self) {
        self.drag.candidate = None;
        if let Some(pending) = &mut self.drag.pending {
            if pending.cancelled {
                return;
            }
            pending.cancelled = true;
            let cancel = ArchiveVirtualDragCommand {
                version: 1,
                operation_id: pending.operation,
                authorization_id: None,
                session_id: self.window_id,
                generation: pending.generation,
                action: "cancel".into(),
                source_window_handle: 0,
                items: None,
            };
            let sent = self.drag.platform.as_ref().is_some_and(|platform| {
                platform
                    .send_archive_virtual_drag(cancel, Uuid::new_v4(), pending.connection)
                    .is_ok()
            });
            tracing::info!(event="archive.virtual_drag.cancel_requested",correlation_id=%pending.operation,sent,"archive virtual gesture retired");
        }
    }
    pub(crate) fn drag_disconnected(&mut self, cx: &mut Context<Self>) {
        self.drag.candidate = None;
        self.drag.authorization = None;
        self.drag.browser = None;
        self.drag.pending = None;
        cx.notify();
    }
    pub(crate) fn drag_matches(&self, request: Uuid) -> bool {
        self.drag
            .pending
            .as_ref()
            .is_some_and(|pending| pending.request == request)
    }
    pub(crate) fn preview_connection(&self) -> u64 {
        self.drag.connection
    }
    pub(crate) fn drag_progress_matches(&self, operation: Uuid, generation: u64) -> bool {
        self.drag.pending.as_ref().is_some_and(|pending| {
            pending.operation == operation && pending.generation == generation
        })
    }
    pub(crate) fn native_drag_active(&self) -> bool {
        self.drag.pending.is_some()
    }
    pub(crate) fn drag_result(
        &mut self,
        request: Uuid,
        result: &PlatformCommandResult,
        cx: &mut Context<Self>,
    ) {
        if !self.drag_matches(request) {
            return;
        }
        let incomplete = result
            .data
            .as_deref()
            .and_then(|data| serde_json::from_str::<serde_json::Value>(data).ok())
            .is_some_and(|data| {
                data.get("cleanup_complete")
                    .and_then(|value| value.as_bool())
                    == Some(false)
            });
        let pending = self.drag.pending.as_ref().expect("matched drag");
        tracing::info!(event="archive.virtual_drag.finished",correlation_id=%pending.operation,success=result.success,cleanup_complete=!incomplete,elapsed_ms=pending.started.elapsed().as_millis() as u64,"archive virtual gesture finished");
        if incomplete {
            if !self.retired
                && !pending.cancelled
                && self.generation == pending.generation
                && self.projection_epoch == pending.epoch
            {
                self.notify_drag("拖出资源尚未清理完成；请稍后重试。", true, cx);
            }
            cx.notify();
            return;
        }
        let pending = self.drag.pending.take().expect("matched drag");
        if self.retired
            || pending.cancelled
            || self.generation != pending.generation
            || self.projection_epoch != pending.epoch
        {
            return;
        }
        if !result.success {
            self.notify_drag(
                match result.error.as_deref() {
                    Some("virtual_drag_busy") => "已有拖出正在结束，请稍后重试。",
                    Some("invalid_source_window") => "来源窗口已失效，未开始拖出。",
                    Some("virtual_target_not_accepted") => "目标未接收本次拖出。",
                    _ => "拖出未完成；源可能已变化或目标无法接收，请刷新后重试。",
                },
                true,
                cx,
            );
        }
        cx.notify();
    }
}

impl ArchiveBrowser {
    pub(crate) fn unlock_drag(&mut self, cx: &mut Context<Self>) {
        self.invalidate_drag();
        if self.retired
            || !matches!(self.status, BrowserStatus::Ready)
            || self.drag.pending.is_some()
        {
            return;
        }
        let Some(browser) = self.drag.browser.clone() else {
            return;
        };
        if !matches!(browser.format.as_str(), "Zip" | "SevenZ" | "Rar4" | "Rar5") {
            self.notify_drag("当前格式不支持密码解锁拖出。", true, cx);
            cx.notify();
            return;
        }
        let command = ArchiveEntryCommand {
            version: 1,
            operation_id: Uuid::new_v4(),
            session_id: self.window_id,
            generation: self.generation,
            action: ArchiveEntryAction::Authorize,
            path: Some(self.path.to_string_lossy().into_owned()),
            codepage: self.codepage,
            ordinal: None,
            browser: Some(browser),
            lease_id: None,
            depth: self.drag.depth,
            copy_motw: Some(self.drag.copy_motw),
            password_interaction: true,
            temp_policy: None,
        };
        let receipt = Uuid::new_v4();
        self.drag.authorization = Some(Authorization {
            command: command.clone(),
            receipt,
            connection: self.drag.connection,
            ready: None,
            waiting: true,
        });
        let sent = self.drag.platform.as_ref().is_some_and(|platform| {
            platform
                .send_archive_authorization(command, receipt, self.drag.connection)
                .is_ok()
        });
        self.notify_drag(
            if sent {
                "正在解锁：完成后请重新按住条目拖动。"
            } else {
                "解锁未提交，请连接后刷新归档。"
            },
            !sent,
            cx,
        );
        if !sent {
            self.drag.authorization = None;
        }
        tracing::info!(event="archive.authorization.submitted",correlation_id=%receipt,sent,"archive password preparation");
        cx.notify();
    }
    fn revoke_drag_authorization(&mut self) {
        if let Some(authorization) = self.drag.authorization.take() {
            let command = ArchiveEntryCommand {
                action: ArchiveEntryAction::Revoke,
                path: None,
                browser: None,
                ordinal: None,
                codepage: 0,
                depth: 0,
                copy_motw: None,
                password_interaction: false,
                temp_policy: None,
                lease_id: Some(authorization.command.operation_id),
                ..authorization.command
            };
            if let Some(platform) = &self.drag.platform {
                let _ = platform.send_archive_authorization(
                    command,
                    Uuid::new_v4(),
                    authorization.connection,
                );
            }
        }
    }
    pub(crate) fn authorization_matches(&self, receipt: Uuid) -> bool {
        self.drag
            .authorization
            .as_ref()
            .is_some_and(|authorization| authorization.waiting && authorization.receipt == receipt)
    }
    pub(crate) fn authorization_password_origin(
        &self,
        prompt: &nxtools_protocol::archive_password::ArchivePasswordPrompt,
    ) -> Option<u64> {
        self.drag
            .authorization
            .as_ref()
            .filter(|authorization| {
                authorization.waiting
                    && authorization.command.operation_id == prompt.operation_id
                    && self.window_id == prompt.session_id
                    && self.generation == prompt.generation
                    && !self.retired
                    && self.drag.platform.as_ref().is_some_and(|platform| {
                        platform.connection_generation() == Some(authorization.connection)
                    })
            })
            .map(|authorization| authorization.connection)
    }
    pub(crate) fn authorization_result(
        &mut self,
        receipt: Uuid,
        result: &PlatformCommandResult,
        cx: &mut Context<Self>,
    ) {
        if !self.authorization_matches(receipt) {
            return;
        }
        self.drag.candidate = None;
        let authorization = self
            .drag
            .authorization
            .as_mut()
            .expect("current preparation");
        let parsed = result.data.as_deref().and_then(|data| {
            serde_json::from_str::<nxtools_protocol::archive_browser::ArchiveEntryResult>(data).ok()
        });
        let valid = parsed.is_some_and(|value| {
            value.success
                && value.cleanup_complete
                && value.operation_id == authorization.command.operation_id
                && value.session_id == self.window_id
                && value.generation == self.generation
                && value.lease_id == Some(authorization.command.operation_id)
                && value.path.is_none()
        });
        if result.success
            && valid
            && self.drag.platform.as_ref().is_some_and(|platform| {
                platform.connection_generation() == Some(authorization.connection)
            })
            && !self.retired
        {
            authorization.waiting = false;
            authorization.ready = Some(Instant::now());
            self.notify_drag("已解锁拖出（短期有效）；请重新按住条目拖动。", false, cx);
        } else {
            self.revoke_drag_authorization();
            self.notify_drag("解锁未完成或已失效，未生成文件；请重试解锁。", true, cx);
        }
        cx.notify();
    }
}
