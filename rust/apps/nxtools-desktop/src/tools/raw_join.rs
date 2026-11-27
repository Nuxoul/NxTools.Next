//! Explicit ordinary numbered-file operation; archive detection and decoding are unchanged.
use gpui_kit::component::{
    ActiveTheme, Disableable, Sizable, Size, WindowExt,
    button::{Button, ButtonVariants},
    dialog::{DialogAction, DialogClose, DialogFooter},
    input::{InputEvent, InputState},
};
use gpui_kit::{
    AnyWindowHandle, App, AppContext, Context, Entity, Focusable, Global, Subscription, WeakEntity,
    Window, WindowBounds, WindowOptions, prelude::*, px, size,
};
use nxtools_ipc::{PlatformBridge, PlatformNotification};
use nxtools_protocol::{
    PlatformCommand, PlatformEvent,
    archive_job::valid_job_path,
    archive_picker::{ArchivePickerCommand, ArchivePickerPurpose, ArchivePickerResult},
    raw_numbered_join::{
        RawJoinAction, RawJoinPreview, RawJoinTerminal, RawNumberedJoinCommand,
        RawNumberedJoinReply,
    },
};
use std::{rc::Rc, time::Instant};
use uuid::Uuid;

mod layout;
mod material;
#[cfg(test)]
mod material_tests;
#[cfg(test)]
mod tests;

const LIMITATION: &str = "将同一目录中同名的 .001、.002… 文件按编号顺序直接连接成一个新文件。不解析压缩包，也不能判断分卷是否完整；请先核对文件列表。不会删除源文件。";

#[derive(Clone)]
struct Target(WeakEntity<JoinWindow>, AnyWindowHandle);
impl Global for Target {}

pub(crate) fn handle(notification: PlatformNotification, cx: &mut App) {
    let Some(Target(view, window)) = cx.try_global::<Target>().cloned() else {
        return;
    };
    cx.defer(move |cx| {
        let _ = window.update(cx, |_, window, cx| {
            let _ = view.update(cx, |view, cx| view.receive(notification, window, cx));
        });
    });
}

pub(crate) fn open(
    platform: Option<Rc<PlatformBridge>>,
    existing: Option<AnyWindowHandle>,
    cx: &mut App,
) -> Option<AnyWindowHandle> {
    let existing = existing.or_else(|| cx.try_global::<Target>().map(|target| target.1));
    let material_pending = existing.is_some_and(|window| {
        cx.try_global::<Target>()
            .filter(|target| target.1 == window)
            .is_some_and(|target| {
                target
                    .0
                    .read_with(cx, |view, _| view.material_pending)
                    .unwrap_or(false)
            })
    });
    if let Some(existing) = existing
        && existing
            .update(cx, |_, window, _| {
                if !material_pending {
                    window.activate_window();
                }
                window.refresh();
            })
            .is_ok()
    {
        return Some(existing);
    }
    let material_platform = platform.clone();
    let material_pending = material_platform.is_some();
    let handle = cx
        .open_window(
            WindowOptions {
                show: !material_pending,
                focus: !material_pending,
                window_bounds: Some(WindowBounds::centered(size(px(680.), px(600.)), cx)),
                window_min_size: Some(size(px(480.), px(360.))),
                app_id: Some("NxTools.RawNumberedJoin".into()),
                ..crate::ui::components::titlebar::caption_window_options()
            },
            move |window, cx| {
                let view = cx.new(|cx| {
                    let mut view = JoinWindow::new(platform, window, cx);
                    view.material_pending = material_pending;
                    view
                });
                let closing = view.downgrade();
                window.on_window_should_close(cx, move |window, cx| {
                    let closing = closing.clone();
                    window.defer(cx, move |window, cx| {
                        let _ = closing.update(cx, |view, cx| view.close(window, cx));
                    });
                    false
                });
                cx.set_global(Target(view.downgrade(), window.window_handle()));
                cx.new(|cx| gpui_kit::base::Root::new(view, window, cx))
            },
        )
        .map(Into::into)
        .map_err(|_| tracing::warn!(event = "tools.raw_numbered_join.window_failed"))
        .ok();
    if let (Some(handle), Some(platform)) = (handle, material_platform) {
        material::prepare(handle, platform, cx);
    }
    handle
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Idle,
    Inspecting,
    Preview,
    Confirming,
    Running,
    Cancelling,
    Terminal,
    Disconnected,
}

struct JoinWindow {
    platform: Option<Rc<PlatformBridge>>,
    material_pending: bool,
    source: Entity<InputState>,
    output: Entity<InputState>,
    session: Uuid,
    generation: u64,
    operation: Uuid,
    origin: Option<u64>,
    phase: Phase,
    preview: Option<RawJoinPreview>,
    pending: Option<(Uuid, RawNumberedJoinCommand)>,
    cancel: Option<Uuid>,
    picker: Option<(Uuid, ArchivePickerPurpose)>,
    terminal: Option<RawJoinTerminal>,
    message: Option<String>,
    closing: bool,
    started: Instant,
    _subscriptions: Vec<Subscription>,
}

impl JoinWindow {
    fn new(
        platform: Option<Rc<PlatformBridge>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let source = cx
            .new(|cx| InputState::new(window, cx).placeholder("选择任意一个 .001 / .002 / … 文件"));
        let output = cx.new(|cx| {
            InputState::new(window, cx).placeholder("新输出文件的完整路径（不覆盖现有文件）")
        });
        let changed = cx.subscribe_in(&source, window, |_, _, event: &InputEvent, window, cx| {
            if matches!(event, InputEvent::Change) {
                cx.defer_in(window, |this, _, cx| {
                    if !this.locked() {
                        this.invalidate();
                        cx.notify();
                    }
                });
            }
        });
        let output_changed =
            cx.subscribe_in(&output, window, |_, _, event: &InputEvent, window, cx| {
                if matches!(event, InputEvent::Change) {
                    cx.defer_in(window, |this, _, cx| {
                        if !this.locked() {
                            this.message = None;
                            cx.notify();
                        }
                    });
                }
            });
        let release = cx.on_release(|this, _| {
            if this.pending.is_some() {
                this.send_cancel();
            }
        });
        Self {
            platform,
            material_pending: false,
            source,
            output,
            session: Uuid::new_v4(),
            generation: 1,
            operation: Uuid::new_v4(),
            origin: None,
            phase: Phase::Idle,
            preview: None,
            pending: None,
            cancel: None,
            picker: None,
            terminal: None,
            message: None,
            closing: false,
            started: Instant::now(),
            _subscriptions: vec![changed, output_changed, release],
        }
    }
    fn working(&self) -> bool {
        matches!(
            self.phase,
            Phase::Inspecting | Phase::Running | Phase::Cancelling
        )
    }
    fn locked(&self) -> bool {
        self.working() || self.phase == Phase::Confirming || self.picker.is_some()
    }
    fn invalidate(&mut self) {
        self.generation = self.generation.wrapping_add(1).max(1);
        self.preview = None;
        self.terminal = None;
        self.message = None;
        self.phase = Phase::Idle;
    }
    fn command(&self, action: RawJoinAction) -> RawNumberedJoinCommand {
        RawNumberedJoinCommand {
            version: 1,
            operation_id: self.operation,
            session_id: self.session,
            generation: self.generation,
            action,
            selected_path: None,
            preview_id: None,
            output_path: None,
        }
    }
    fn inspect(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if self.locked() {
            return;
        }
        let source = self.source.read(cx).value().to_string();
        if !valid_job_path(&source) {
            self.message = Some("请选择一个 .001 / .002 / … 文件的完整路径。".into());
            cx.notify();
            return;
        }
        self.invalidate();
        self.operation = Uuid::new_v4();
        self.origin = self
            .platform
            .as_ref()
            .and_then(|bridge| bridge.connection_generation());
        self.cancel = None;
        self.closing = false;
        self.phase = Phase::Inspecting;
        let mut command = self.command(RawJoinAction::Inspect);
        command.selected_path = Some(source);
        self.dispatch(command, cx);
    }

    fn dispatch(&mut self, command: RawNumberedJoinCommand, cx: &mut Context<Self>) {
        let id = Uuid::new_v4();
        self.pending = Some((id, command.clone()));
        self.started = Instant::now();
        let sent = self
            .platform
            .as_ref()
            .zip(self.origin)
            .is_some_and(|(bridge, origin)| {
                bridge.send_raw_numbered_join(command, id, origin).is_ok()
            });
        if sent {
            tracing::info!(event="tools.raw_numbered_join.ui_accepted",correlation_id=%self.operation,session_id=%self.session,generation=self.generation);
        } else {
            self.pending = None;
            self.phase = Phase::Idle;
            self.preview = None;
            self.message = Some("平台未连接或正在执行其他合并，请稍后重试。".into());
        }
        cx.notify();
    }
    fn confirm(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.phase != Phase::Preview || self.locked() {
            return;
        }
        let Some(preview) = self.preview.clone() else {
            return;
        };
        let output = self.output.read(cx).value().to_string();
        if !valid_job_path(&output) {
            self.message = Some("请选择新输出文件的完整路径。".into());
            cx.notify();
            return;
        }
        self.phase = Phase::Confirming;
        let generation = self.generation;
        let weak = cx.entity().downgrade();
        let decline = weak.clone();
        let closed = weak.clone();
        window.open_dialog(cx, move |dialog, _, cx| {
            let dialog = dialog.bg(crate::theme::overlay_background(cx));
            let weak = weak.clone();
            let decline = decline.clone();
            let closed = closed.clone();
            let output = output.clone();
            dialog
                .title("确认合并这些分卷文件")
                .child(layout::confirmation(&preview, &output))
                .footer(
                    DialogFooter::new()
                        .child(DialogClose::new().trigger(|button| {
                            button.label("返回核对").outline().with_size(Size::Medium)
                        }))
                        .child(
                            DialogAction::new().child(
                                Button::new("raw-join-confirm")
                                    .debug_selector(|| "raw-join-confirm".into())
                                    .label("确认合并")
                                    .primary()
                                    .with_size(Size::Medium)
                                    .on_click(|event, window, cx| {
                                        if event.is_keyboard() {
                                            window.dispatch_action(
                                                Box::new(gpui_kit::component::dialog::Confirm {
                                                    secondary: false,
                                                }),
                                                cx,
                                            );
                                        }
                                    }),
                            ),
                        ),
                )
                .on_ok(move |_, window, cx| {
                    let weak = weak.clone();
                    let output = output.clone();
                    window.defer(cx, move |_, cx| {
                        let _ = weak.update(cx, |this, cx| this.start(generation, output, cx));
                    });
                    true
                })
                .on_cancel(move |_, window, cx| {
                    Self::decline(decline.clone(), generation, window, cx);
                    true
                })
                .on_close(move |_, window, cx| {
                    Self::decline(closed.clone(), generation, window, cx)
                })
        });
        cx.notify();
    }
    fn decline(weak: WeakEntity<Self>, generation: u64, window: &mut Window, cx: &mut App) {
        window.defer(cx, move |_, cx| {
            let _ = weak.update(cx, |this, cx| {
                if this.generation == generation && this.phase == Phase::Confirming {
                    this.phase = Phase::Preview;
                    cx.notify();
                }
            });
        });
    }
    fn start(&mut self, generation: u64, output: String, cx: &mut Context<Self>) {
        if self.generation != generation || self.phase != Phase::Confirming {
            return;
        }
        let Some(preview) = &self.preview else {
            return;
        };
        let mut command = self.command(RawJoinAction::Start);
        command.preview_id = Some(preview.preview_id);
        command.output_path = Some(output);
        self.phase = Phase::Running;
        self.message = None;
        self.dispatch(command, cx);
    }
    fn send_cancel(&mut self) {
        if self.cancel.is_some() {
            return;
        }
        if let Some((bridge, origin)) = self.platform.as_ref().zip(self.origin)
            && let Ok(id) = bridge.send_raw_numbered_join(
                self.command(RawJoinAction::Cancel),
                Uuid::new_v4(),
                origin,
            )
        {
            self.cancel = Some(id);
        }
    }
    fn cancel(&mut self, cx: &mut Context<Self>) {
        if !self.working() {
            return;
        }
        self.send_cancel();
        self.phase = Phase::Cancelling;
        self.message = Some(
            if self.cancel.is_some() {
                "正在取消，等待文件句柄和清理实际完成。"
            } else {
                "取消请求未送达；仍在等待原操作结果。"
            }
            .into(),
        );
        cx.notify();
    }
    fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.working() {
            self.closing = true;
            self.cancel(cx);
        } else if self.phase != Phase::Confirming && self.picker.is_none() {
            window.remove_window();
        }
    }
    fn pick(&mut self, purpose: ArchivePickerPurpose, window: &mut Window, cx: &mut Context<Self>) {
        if self.locked() {
            return;
        }
        let Some(bridge) = &self.platform else {
            self.message = Some("平台尚未连接。".into());
            cx.notify();
            return;
        };
        let Some(origin) = bridge.connection_generation() else {
            self.message = Some("平台尚未连接。".into());
            cx.notify();
            return;
        };
        let value = if purpose == ArchivePickerPurpose::Files {
            self.source.read(cx).value()
        } else {
            self.output.read(cx).value()
        };
        let path = std::path::Path::new(value.as_ref());
        let operation = Uuid::new_v4();
        let command = ArchivePickerCommand {
            version: 1,
            operation_id: operation,
            session_id: self.session,
            generation: self.generation,
            owner_window_handle: crate::ui::native_window_handle(window).unwrap_or(0),
            purpose,
            current_directory: path
                .parent()
                .and_then(|p| p.to_str())
                .filter(|p| valid_job_path(p))
                .map(str::to_owned),
            suggested_name: (purpose == ArchivePickerPurpose::Save).then(|| {
                path.file_name()
                    .and_then(|p| p.to_str())
                    .unwrap_or("joined.bin")
                    .to_owned()
            }),
        };
        self.origin = Some(origin);
        self.picker = Some((operation, purpose));
        self.message = None;
        if command.validate().is_err()
            || bridge
                .send_command_with_id_correlated(
                    PlatformCommand::ArchivePicker(command),
                    operation,
                    operation,
                )
                .is_err()
        {
            self.picker = None;
            self.message = Some("文件选择器不可用或繁忙，请重试。".into());
        }
        cx.notify();
    }
    fn picker_result(
        &mut self,
        result: ArchivePickerResult,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.picker.map(|p| p.0) != Some(result.operation_id)
            || result.session_id != self.session
            || result.generation != self.generation
            || result.validate().is_err()
        {
            return;
        }
        let Some((_, purpose)) = self.picker.take() else {
            return;
        };
        window.activate_window();
        if result.cancelled {
            cx.notify();
            return;
        }
        if result.code.is_some() || result.paths.len() != 1 {
            self.message = Some("请只选择一个编号分卷，或一个新输出路径。".into());
            cx.notify();
            return;
        }
        let input = if purpose == ArchivePickerPurpose::Files {
            self.invalidate();
            self.source.clone()
        } else {
            self.output.clone()
        };
        input.update(cx, |input, cx| {
            input.set_value(result.paths[0].clone(), window, cx);
            input.focus_handle(cx).focus(window, cx);
        });
        cx.notify();
    }
    fn receive(
        &mut self,
        notification: PlatformNotification,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let PlatformNotification::Disconnected { generation, .. } = &notification {
            if self.origin == Some(*generation) {
                let active = self.working();
                self.pending = None;
                self.cancel = None;
                self.picker = None;
                self.preview = None;
                self.phase = Phase::Disconnected;
                self.message = Some(
                    if active {
                        "连接已断开，输出结果未知。原生端将取消并清理；请核对输出，勿直接重复合并。"
                    } else {
                        "平台连接已断开，请重新查看文件列表。"
                    }
                    .into(),
                );
                self.closing = false;
                cx.notify();
            }
            return;
        }
        if self.origin
            != self
                .platform
                .as_ref()
                .and_then(|b| b.connection_generation())
        {
            return;
        }
        match notification {
            PlatformNotification::Event(PlatformEvent::ArchivePicker(result)) => {
                self.picker_result(result, window, cx)
            }
            PlatformNotification::CommandResult {
                correlation_id,
                result,
            } => {
                if self.cancel == Some(correlation_id) {
                    if !result.success {
                        self.message = Some("取消请求未确认，仍在等待原操作终态。".into());
                        cx.notify();
                    }
                    return;
                }
                if self.picker.is_some_and(|p| p.0 == correlation_id) && !result.success {
                    self.picker = None;
                    self.message = Some("文件选择器未完成，请重试。".into());
                    cx.notify();
                    return;
                }
                let Some((id, command)) = &self.pending else {
                    return;
                };
                if *id != correlation_id {
                    return;
                }
                let reply = result
                    .data
                    .as_deref()
                    .and_then(|data| serde_json::from_str::<RawNumberedJoinReply>(data).ok());
                if let Some(reply) = &reply
                    && reply.validate(command).is_err()
                {
                    return;
                }
                self.pending = None;
                self.cancel = None;
                if let Some(reply) = reply {
                    if let Some(preview) = reply.preview {
                        self.output.update(cx, |input, cx| {
                            input.set_value(preview.suggested_output.clone(), window, cx)
                        });
                        self.preview = Some(preview);
                        self.phase = Phase::Preview;
                        self.message = None;
                    } else if let Some(terminal) = reply.terminal {
                        self.phase = Phase::Terminal;
                        self.preview = None;
                        self.message = Some(layout::terminal_message(&terminal));
                        self.terminal = Some(terminal);
                    }
                } else {
                    self.phase = Phase::Terminal;
                    self.preview = None;
                    self.message =
                        Some("操作未获有效结果，无法确认输出状态；请核对文件后重新发现。".into());
                }
                tracing::info!(event="tools.raw_numbered_join.ui_result",correlation_id=%self.operation,session_id=%self.session,generation=self.generation,elapsed_ms=self.started.elapsed().as_millis());
                if self.closing
                    && self
                        .terminal
                        .as_ref()
                        .is_some_and(|t| t.cleanup_complete && t.outcome == "cancelled")
                {
                    window.remove_window();
                } else {
                    self.closing = false;
                    cx.notify();
                }
            }
            _ => {}
        }
    }
}
