//! Formal row gesture; sole BrowserModel selection, original bridge and native cleanup owner.
use super::*;
use gpui_kit::{Modifiers, MouseDownEvent, MouseMoveEvent, Point};
use nxtools_protocol::{
    PlatformCommandResult,
    archive_browser::{ArchiveBrowserContext, ArchiveEntryAction, ArchiveEntryCommand},
    archive_virtual_drag::{ArchiveVirtualDragCommand, ArchiveVirtualDragItem},
};
use std::{rc::Rc, time::Instant};

struct Candidate {
    row: RowId,
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
    pub(super) message: Option<String>,
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
            Some(model.snapshot) == snapshot && model.visible().iter().any(|item| item.id == row)
        }) else {
            return;
        };
        self.drag.candidate = Some(Candidate {
            row,
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
        self.drag.message = None;
        if let Some(model) = &mut self.model {
            // Pressing an already selected row preserves the whole multi-selection for drag.
            // An ordinary click still runs the original Click owner on release instead.
            if model.selected.contains(&candidate.row) {
                model.focused = Some(candidate.row.clone());
            } else {
                model.select(
                    candidate.row,
                    candidate.modifiers.control,
                    candidate.modifiers.shift,
                );
            }
        }
        self.list_focus.focus(window, cx);
        self.update_summary();
        #[cfg(not(test))]
        let source_hwnd = crate::ui::native_window_handle(window);
        #[cfg(test)]
        let source_hwnd = self.drag.source_hwnd;
        match self.drag_command(source_hwnd.unwrap_or(0)) {
            Err(reason) => self.drag.message = Some(reason.into()),
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
                    self.drag.message =
                        Some("拖出未提交：原生连接不可用，请重新连接后刷新。".into());
                }
                tracing::info!(event="archive.virtual_drag.submitted",correlation_id=%operation,submitted,"archive virtual gesture submission");
            }
        }
        cx.notify();
    }
    fn drag_command(&self, hwnd: i64) -> Result<ArchiveVirtualDragCommand, &'static str> {
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
        let frozen = if authorized.is_some() {
            model.freeze_drag_selection_authorized(true)
        } else {
            model.freeze_drag_selection().map_err(|reason| {
                if model.freeze_drag_selection_authorized(true).is_ok() {
                    "加密条目需要先点击“解锁拖出”并输入密码，再重新按住拖动；未拖出任何项目。"
                } else {
                    reason
                }
            })
        }?;
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
            return Err("来源窗口不可用，未开始拖出。");
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
            .map_err(|_| "选择的名称或源身份不满足安全拖出限制；未拖出任何项目。")?;
        Ok(command)
    }
    pub(crate) fn invalidate_drag(&mut self) {
        self.drag.candidate = None;
        self.revoke_drag_authorization();
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
        self.drag.message = None;
        cx.notify();
    }
    pub(crate) fn drag_matches(&self, request: Uuid) -> bool {
        self.drag
            .pending
            .as_ref()
            .is_some_and(|pending| pending.request == request)
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
            self.drag.message =
                Some("拖出资源尚未清理完成；为避免重复操作，请关闭浏览器并重新连接后重试。".into());
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
            self.drag.message = Some(
                match result.error.as_deref() {
                    Some("virtual_drag_busy") => "已有拖出正在结束，请稍后重试。",
                    Some("invalid_source_window") => "来源窗口已失效，未开始拖出。",
                    Some("virtual_target_not_accepted") => "目标未接收本次拖出。",
                    _ => "拖出未完成；源可能已变化或目标无法接收，请刷新后重试。",
                }
                .into(),
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
            self.drag.message = Some("当前格式不支持密码解锁拖出。".into());
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
        self.drag.message = Some(
            if sent {
                "正在解锁：将完整测试归档，期间不生成文件；完成后请重新按住拖动。"
            } else {
                "解锁未提交，请连接后刷新归档。"
            }
            .into(),
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
            self.drag.message =
                Some("已解锁拖出（短期有效）；请重新按住条目并拖动，不会自动开始。".into());
        } else {
            self.revoke_drag_authorization();
            self.drag.message = Some("解锁未完成或已失效，未生成文件；请重试解锁。".into());
        }
        cx.notify();
    }
}
