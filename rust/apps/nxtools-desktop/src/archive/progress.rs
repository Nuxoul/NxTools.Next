//! Official progress/prompt presentation; no execution, notification clock or native owner.
use super::ArchiveApplication;
use gpui_kit::component::{
    ActiveTheme, Disableable, Sizable, Size, WindowExt,
    alert::Alert,
    button::Button,
    checkbox::Checkbox,
    collapsible::Collapsible,
    description_list::DescriptionList,
    dialog::{Confirm, DialogClose, DialogFooter},
    form::{Field, Form},
    input::{Input, InputState},
    label::Label,
    progress::Progress,
};
use gpui_kit::{
    App, Context, FocusHandle, Focusable, IntoElement, ParentElement, Render, WeakEntity, Window,
    base::TestSupportExt, div, prelude::*,
};
use nxtools_protocol::{
    SetTaskbarProgressCommand, TaskbarProgressState, archive_job::*,
    archive_password::PrivatePassword,
};
use uuid::Uuid;

#[cfg(debug_assertions)]
pub(super) fn open_visual_preview(
    unknown: bool,
    minimum: bool,
    cx: &mut App,
) -> anyhow::Result<()> {
    use gpui_kit::{AppContext, Bounds, WindowBounds, WindowDecorations, WindowOptions, px, size};
    let viewport = if minimum {
        size(px(480.), px(260.))
    } else {
        size(px(560.), px(260.))
    };
    cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds::centered(None, viewport, cx))),
            titlebar: None,
            window_decorations: Some(WindowDecorations::Client),
            window_min_size: Some(size(px(480.), px(260.))),
            app_id: Some("NxTools.ArchiveProgress.Preview".into()),
            ..Default::default()
        },
        move |window, cx| {
            window.set_window_title("NxTools Archive Progress — isolated");
            let view = cx.new(|cx| {
                let mut view =
                    JobProgressView::new(Uuid::new_v4(), WeakEntity::new_invalid(), true, cx);
                view.kind = ArchiveJobKind::Extract;
                view.state = "运行中".into();
                view.elapsed_ms = 12_000;
                view.progress = Some(
                    serde_json::from_value(serde_json::json!({
                        "phase":"running", "bytes_done":40_000_000, "bytes_total":100_000_000,
                        "files_done":3, "files_total":12,
                        "current_member":"发布资料/companion/dist/bcprov-jdk18on-1.78.1.jar",
                        "current_bytes_done":if unknown {None} else {Some(500_000)},
                        "current_bytes_total":if unknown {None} else {Some(2_000_000)},
                    }))
                    .expect("synthetic progress"),
                );
                view
            });
            println!(
                "ui.archive_progress.ready hwnd={} scale={} runtime=false platform=false",
                crate::ui::native_window_handle(window).unwrap_or_default(),
                window.scale_factor()
            );
            cx.new(|cx| gpui_kit::base::Root::new(view, window, cx))
        },
    )?;
    Ok(())
}

#[path = "progress_metrics.rs"]
mod metrics;
use metrics::RateSamples;

struct PromptButtonKeys;
impl gpui_kit::Global for PromptButtonKeys {}

#[derive(Clone, Default)]
struct PromptButtonInput(std::rc::Rc<std::cell::Cell<bool>>);
impl PromptButtonInput {
    fn guard(&self, button: Button) -> Button {
        let down = self.0.clone();
        let up = self.0.clone();
        button
            .key_context("ArchivePromptButton")
            .capture_key_down(move |event, window, cx| {
                let activation = matches!(event.keystroke.key.as_str(), "enter" | "space");
                if !activation {
                    down.set(false);
                    return;
                }
                if event.prefer_character_input || event.keystroke.modifiers.modified() {
                    down.set(false);
                } else if !event.is_held {
                    down.set(true);
                    return;
                }
                window.prevent_default();
                cx.stop_propagation();
            })
            .capture_key_up(move |event, window, cx| {
                if matches!(event.keystroke.key.as_str(), "enter" | "space") && !up.replace(false) {
                    window.prevent_default();
                    cx.stop_propagation();
                }
            })
    }
}

pub(super) struct ProgressSnapshot {
    pub operation: Uuid,
    pub generation: u64,
    pub continuation: Option<(Uuid, u64)>,
    pub state: String,
    pub kind: ArchiveJobKind,
    pub elapsed_ms: u64,
    pub reclamation: Option<ArchiveReclamationMode>,
    pub progress_elapsed_ms: u64,
    pub progress: Option<ArchiveJobProgress>,
    pub terminal: Option<ArchiveJobTerminal>,
    pub prompt: Option<ArchiveJobPrompt>,
    pub paused: bool,
    pub location_status: Option<String>,
    pub pause_requested: Option<bool>,
    pub cancel_requested: bool,
    pub control_status: Option<String>,
    pub batch: Option<super::batch::BatchSnapshot>,
}
pub(super) struct JobProgressView {
    id: Uuid,
    owner: WeakEntity<ArchiveApplication>,
    focus: FocusHandle,
    state: String,
    kind: ArchiveJobKind,
    elapsed_ms: u64,
    reclamation: Option<ArchiveReclamationMode>,
    continuation: Option<(Uuid, u64)>,
    progress: Option<ArchiveJobProgress>,
    terminal: Option<ArchiveJobTerminal>,
    prompt: Option<ArchiveJobPrompt>,
    shown_prompt: Option<ArchiveJobTicket>,
    paused: bool,
    keep_open: bool,
    prompt_inputs: Vec<PromptButtonInput>,
    activation: Option<gpui_kit::Subscription>,
    rate: RateSamples,
    pause_requested: Option<bool>,
    cancel_requested: bool,
    drag: Option<nxtools_protocol::archive_virtual_drag::ArchiveVirtualDragProgress>,
    location_status: Option<String>,
    control_status: Option<String>,
    observed_at: std::time::Instant,
    observed_elapsed_ms: u64,
    refresh: Option<gpui_kit::Task<()>>,
    visibility: Option<gpui_kit::Subscription>,
    presentation_retired: bool,
    batch: Option<super::batch::BatchSnapshot>,
    summary: bool,
    diagnostics_open: bool,
}
impl JobProgressView {
    pub(super) fn new(
        id: Uuid,
        owner: WeakEntity<ArchiveApplication>,
        keep_open: bool,
        cx: &mut Context<Self>,
    ) -> Self {
        Self {
            id,
            owner,
            focus: cx.focus_handle(),
            state: "排队中".into(),
            kind: ArchiveJobKind::Test,
            elapsed_ms: 0,
            reclamation: None,
            continuation: None,
            progress: None,
            terminal: None,
            prompt: None,
            shown_prompt: None,
            paused: false,
            keep_open,
            prompt_inputs: Vec::new(),
            activation: None,
            rate: RateSamples::default(),
            drag: None,
            location_status: None,
            pause_requested: None,
            cancel_requested: false,
            control_status: None,
            observed_at: cx.background_executor().now(),
            observed_elapsed_ms: 0,
            refresh: None,
            visibility: None,
            presentation_retired: false,
            batch: None,
            summary: false,
            diagnostics_open: false,
        }
    }
    pub(super) fn apply(
        &mut self,
        snapshot: ProgressSnapshot,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.activation.is_none() {
            self.activation = Some(cx.observe_window_activation(window, |this, _, _| {
                for input in &this.prompt_inputs {
                    input.0.set(false);
                }
            }));
        }
        if self.visibility.is_none() {
            let view = cx.entity().downgrade();
            self.visibility = Some(window.observe_window_visibility(move |_, window, cx| {
                let _ = view.update(cx, |view, cx| view.sync_refresh(window, cx));
            }));
        }
        let ProgressSnapshot {
            operation,
            generation,
            continuation,
            state,
            kind,
            elapsed_ms,
            reclamation,
            progress_elapsed_ms,
            progress,
            terminal,
            prompt,
            paused,
            location_status,
            pause_requested,
            cancel_requested,
            control_status,
            batch,
        } = snapshot;
        self.pause_requested = pause_requested;
        self.cancel_requested = cancel_requested;
        self.control_status = control_status;
        self.batch = batch;
        if terminal.is_some()
            || paused
            || prompt.is_some()
            || state != "运行中"
            || self.pause_requested.is_some()
            || self.cancel_requested
        {
            self.rate.clear();
        } else if let Some(progress) = &progress {
            self.rate.observe(
                (operation, generation, progress.phase.clone()),
                progress_elapsed_ms,
                progress.bytes_done,
            );
        } else {
            self.rate.clear();
        }
        self.state = state;
        self.kind = kind;
        self.reclamation = reclamation;
        self.continuation = continuation;
        self.elapsed_ms = elapsed_ms;
        self.observed_elapsed_ms = elapsed_ms;
        self.observed_at = cx.background_executor().now();
        self.progress = progress;
        self.terminal = terminal;
        self.prompt = prompt;
        self.paused = paused;
        self.location_status = location_status;
        if self.prompt.is_none() && self.shown_prompt.take().is_some() {
            for input in self.prompt_inputs.drain(..) {
                input.0.set(false);
            }
            window.close_dialog(cx);
        }
        if let Some(prompt) = self.prompt.clone()
            && self.shown_prompt.as_ref() != Some(&prompt.ticket)
        {
            self.shown_prompt = Some(prompt.ticket.clone());
            self.show_prompt(prompt, window, cx);
        }
        if self
            .terminal
            .as_ref()
            .is_some_and(|result| result.outcome == ArchiveJobOutcome::Succeeded)
            && !self.keep_open
            && self
                .batch
                .as_ref()
                .is_none_or(|b| b.members.first().is_none_or(|m| m.id != self.id))
        {
            let owner = self.owner.clone();
            let id = self.id;
            cx.defer(move |cx| {
                let _ = owner.update(cx, |app, cx| app.background_job(id, cx));
            });
        }
        self.sync_refresh(window, cx);
        cx.notify();
    }
    pub(super) fn apply_batch(
        &mut self,
        batch: Option<super::batch::BatchSnapshot>,
        cx: &mut Context<Self>,
    ) {
        self.batch = batch;
        cx.notify();
    }
    pub(super) fn show_detail(&mut self, cx: &mut Context<Self>) {
        self.summary = false;
        cx.notify();
    }
    fn refresh_active(&self) -> bool {
        !self.presentation_retired
            && self.drag.is_none()
            && self.terminal.is_none()
            && self.state != "排队中"
            && !self.state.starts_with("连接断开")
    }
    fn advance_display(&mut self, cx: &Context<Self>) {
        let passed = cx
            .background_executor()
            .now()
            .saturating_duration_since(self.observed_at);
        self.elapsed_ms = self
            .observed_elapsed_ms
            .saturating_add(passed.as_millis().min(u64::MAX as u128) as u64);
        self.rate.expire(self.elapsed_ms);
    }
    fn sync_refresh(&mut self, window: &Window, cx: &mut Context<Self>) {
        if !self.refresh_active() || !window.is_visible() {
            self.refresh = None;
            return;
        }
        self.advance_display(cx);
        if self.refresh.is_none() {
            self.refresh = Some(cx.spawn_in(window, async move |view, cx| {
                loop {
                    cx.background_executor()
                        .timer(std::time::Duration::from_millis(100))
                        .await;
                    let keep = view
                        .update_in(cx, |view, window, cx| {
                            if !view.refresh_active() || !window.is_visible() {
                                view.refresh = None;
                                return false;
                            }
                            view.advance_display(cx);
                            cx.notify();
                            true
                        })
                        .unwrap_or(false);
                    if !keep {
                        break;
                    }
                }
            }));
        }
        cx.notify();
    }
    pub(super) fn retire_display(&mut self) {
        self.presentation_retired = true;
        self.refresh = None;
        self.rate.clear();
    }
    pub(super) fn apply_drag(
        &mut self,
        value: nxtools_protocol::archive_virtual_drag::ArchiveVirtualDragProgress,
        elapsed_ms: u64,
        cx: &mut Context<Self>,
    ) {
        self.rate.clear(); // Cache milestones are not a credible decoded-byte sample stream.
        self.refresh = None;
        self.elapsed_ms = elapsed_ms;
        self.drag = Some(value);
        cx.notify();
    }
    fn command(&mut self, action: ArchiveJobAction, cx: &mut Context<Self>) {
        if self.drag.is_some() {
            return;
        }
        if self.terminal.is_some()
            || self.cancel_requested
            || (self.pause_requested.is_some() && action != ArchiveJobAction::Cancel)
        {
            return;
        }
        match action {
            ArchiveJobAction::Pause => self.pause_requested = Some(true),
            ArchiveJobAction::Resume => self.pause_requested = Some(false),
            ArchiveJobAction::Cancel => self.cancel_requested = true,
            _ => return,
        }
        self.rate.clear();
        self.control_status = None;
        cx.notify();
        let owner = self.owner.clone();
        let id = self.id;
        cx.defer(move |cx| {
            let _ = owner.update(cx, |app, cx| app.control_job(id, action, cx));
        });
    }
    fn background(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.retire_display();
        if self.drag.is_some() {
            let owner = self.owner.clone();
            let id = self.id;
            cx.defer(move |cx| {
                let _ = owner.update(cx, |app, cx| app.background_drag_progress(id, cx));
            });
            return;
        }
        let owner = self.owner.clone();
        let id = self.id;
        cx.defer(move |cx| {
            let _ = owner.update(cx, |app, cx| app.background_job(id, cx));
        });
    }
    fn show_prompt(
        &mut self,
        prompt: ArchiveJobPrompt,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let owner = self.owner.clone();
        let id = self.id;
        let editing = self.kind == ArchiveJobKind::Modify;
        if !cx.has_global::<PromptButtonKeys>() {
            cx.bind_keys([gpui_kit::KeyBinding::new(
                "enter",
                gpui_kit::NoAction,
                Some("ArchivePromptButton"),
            )]);
            cx.set_global(PromptButtonKeys);
        }
        let input = cx.new(|cx| InputState::new(window, cx).masked(true));
        let apply = std::rc::Rc::new(std::cell::Cell::new(false));
        let confirm_anchor = cx.focus_handle();
        let explicit_confirm = std::rc::Rc::new(std::cell::Cell::new(false));
        let confirm_input = PromptButtonInput::default();
        let cancel_input_guard = PromptButtonInput::default();
        let conflict_inputs: [PromptButtonInput; 3] =
            std::array::from_fn(|_| PromptButtonInput::default());
        for input in self.prompt_inputs.drain(..) {
            input.0.set(false);
        }
        self.prompt_inputs = vec![confirm_input.clone(), cancel_input_guard.clone()];
        self.prompt_inputs.extend(conflict_inputs.iter().cloned());
        window.open_dialog(cx,move|dialog,_,_|{
            let ok_owner=owner.clone();let cancel_owner=owner.clone();let ok_prompt=prompt.clone();let cancel_prompt=prompt.clone();let ok_input=input.clone();let cancel_input=input.clone();let ok_apply=apply.clone();
            let button_confirm=explicit_confirm.clone();let ok_explicit=explicit_confirm.clone();let action_anchor=confirm_anchor.clone();
            let body=match prompt.kind{
                ArchiveJobPromptKind::Password=>Form::vertical().child(Field::new().label(if prompt.error_code.as_deref()==Some("WrongPassword"){ "密码错误，请重试" }else{"当前归档密码"}).child(Input::new(&input).mask_toggle().aria_label("当前任务密码"))).into_any_element(),
                ArchiveJobPromptKind::Conflict=>div().flex().flex_col().gap_2().child(if editing {"添加到归档时遇到同名成员。覆盖只在完整候选验证后一次提交；不会先删除原件。日期：未知。"} else {"目标已有同名文件。"}).child(prompt_identifier("archive-prompt-sizes",&format!("现有大小：{}；传入大小：{}。",number(prompt.existing_bytes),number(prompt.incoming_bytes)))).child(prompt_objects(&prompt, editing)).into_any_element(),
                ArchiveJobPromptKind::Risk=>div().child("此文件的解压大小可能远大于压缩后大小。继续仍受当前大小限制约束；取消将停止任务。").into_any_element(),
                ArchiveJobPromptKind::OutputBudget=>div().child(format!("上次尝试未提交文件且已完成清理。若要重新尝试，需要将单文件上限设为 {}、总输出上限设为 {}。仅这两个上限会随新尝试增加。",number(prompt.existing_bytes),number(prompt.incoming_bytes))).into_any_element(),
                ArchiveJobPromptKind::CreateOverwrite=>div().flex().flex_col().gap_2().child("目标压缩包已存在。确认覆盖后将先完成验证，再替换原文件；若目标发生变化则停止。").child(prompt_objects(&prompt, false)).into_any_element(),
                ArchiveJobPromptKind::Destination=>div().child(if prompt.error_code.as_deref()==Some("create_destination"){format!("目标文件夹不存在。要创建文件夹并继续吗？预计输出：{}；可用空间：{}。",number(prompt.incoming_bytes),number(prompt.existing_bytes))}else{format!("目标可用空间不足。预计输出：{}；可用空间：{}。仍要继续吗？如果写入失败，任务会停止。",number(prompt.incoming_bytes),number(prompt.existing_bytes))}).into_any_element(),
            };
            let mut footer=DialogFooter::new().child(DialogClose::new().trigger(|b|cancel_input_guard.guard(b.label("取消任务"))));
            if prompt.kind==ArchiveJobPromptKind::Conflict{
                for (index,(label,decision)) in [("覆盖",ArchiveJobConflictDecision::Overwrite),("跳过",ArchiveJobConflictDecision::Skip),("自动重命名",ArchiveJobConflictDecision::AutoRename)].into_iter().enumerate(){
                    let response_owner=owner.clone();let response_prompt=prompt.clone();let apply=apply.clone();
                    footer=footer.child(conflict_inputs[index].guard(Button::new(gpui_kit::SharedString::from(format!("archive-conflict-{}-{label}",prompt.ticket.prompt_id))).label(label).outline().with_size(Size::Medium)).on_click(move|_,window,cx|{let owner=response_owner.clone();let prompt=response_prompt.clone();let reply=ArchiveJobReply{ticket:prompt.ticket.clone(),kind:prompt.kind,password:None,decision:Some(decision),apply_to_all:apply.get(),approved_limit:None};window.close_dialog(cx);cx.defer(move|cx|{let _=owner.update(cx,|app,cx|app.reply_job(id,prompt,reply,cx));});}));
                }
            }else if matches!(prompt.kind,ArchiveJobPromptKind::Password|ArchiveJobPromptKind::Risk|ArchiveJobPromptKind::OutputBudget|ArchiveJobPromptKind::CreateOverwrite|ArchiveJobPromptKind::Destination){footer=footer.child(confirm_input.guard(Button::new(gpui_kit::SharedString::from(format!("archive-job-prompt-ok-{}",prompt.ticket.prompt_id))).label(if prompt.kind==ArchiveJobPromptKind::CreateOverwrite{"确认覆盖"}else{"继续"})).debug_selector(||"archive-job-prompt-ok".into()).child(div().absolute().size_0().track_focus(&confirm_anchor)).on_click(move|_,window,cx|{button_confirm.set(true);action_anchor.dispatch_action(&Confirm{secondary:false},window,cx);}));}
            let footer=if prompt.kind==ArchiveJobPromptKind::Conflict{
                div().flex_none().flex().flex_col().gap_2()
                    .child(Checkbox::new("archive-conflict-all").label("应用到此任务后续冲突").checked(apply.get()).on_click({let apply=apply.clone();move|checked,_,_|apply.set(*checked)}))
                    .child(footer).into_any_element()
            }else{footer.into_any_element()};
            dialog.title(match prompt.kind{ArchiveJobPromptKind::Password=>"归档需要密码",ArchiveJobPromptKind::Conflict=>"处理同名文件",ArchiveJobPromptKind::Risk=>"确认高膨胀率",ArchiveJobPromptKind::Destination=>"确认目标目录与空间",_=>"归档操作确认"}).child(body).footer(footer)
                .on_ok(move|_,window,cx|{if ok_prompt.kind==ArchiveJobPromptKind::CreateOverwrite && !ok_explicit.replace(false){return false;}let password=(ok_prompt.kind==ArchiveJobPromptKind::Password).then(||PrivatePassword::new(ok_input.read(cx).value().as_ref()));ok_input.update(cx,|input,cx|input.set_value("",window,cx));let prompt=ok_prompt.clone();let reply=ArchiveJobReply{ticket:prompt.ticket.clone(),kind:prompt.kind,password,decision:matches!(prompt.kind,ArchiveJobPromptKind::CreateOverwrite|ArchiveJobPromptKind::Destination).then_some(ArchiveJobConflictDecision::Overwrite),apply_to_all:ok_apply.get(),approved_limit:prompt.approved_limit};let owner=ok_owner.clone();cx.defer(move|cx|{let _=owner.update(cx,|app,cx|app.reply_job(id,prompt,reply,cx));});true})
                .on_cancel(move|_,window,cx|{cancel_input.update(cx,|input,cx|input.set_value("",window,cx));let owner=cancel_owner.clone();let prompt=cancel_prompt.clone();let reply=ArchiveJobReply{ticket:prompt.ticket.clone(),kind:prompt.kind,password:None,decision:None,apply_to_all:false,approved_limit:None};cx.defer(move|cx|{let _=owner.update(cx,|app,cx|app.reply_job(id,prompt,reply,cx));});true})
        });
    }
    fn stage_label(&self) -> String {
        if self.terminal.is_some() {
            return self.state.clone();
        }
        if let Some(drag) = &self.drag {
            return match drag.phase.as_str() {
                "materializing" => "正在解压拖出内容",
                "content_ready" => "当前内容已就绪，等待接收方读取",
                "target_copying" => "接收方正在复制；目标写入进度未知",
                "finished" if drag.code.as_deref() == Some("receiver_reported_complete") => {
                    "接收方报告完成"
                }
                "finished" => "拖出原生操作已结束；目标写入未独立确认",
                "cancelled" => "拖出已取消",
                _ => "拖出未完成",
            }
            .into();
        }
        if self.cancel_requested {
            return "正在取消，等待原生清理".into();
        }
        if let Some(pausing) = self.pause_requested {
            return if pausing {
                "正在暂停"
            } else {
                "正在恢复"
            }
            .into();
        }
        if self.paused {
            return self.state.clone();
        }
        if let Some(prompt) = &self.prompt {
            return match prompt.kind {
                ArchiveJobPromptKind::Password => "等待密码",
                ArchiveJobPromptKind::Conflict => "等待同名处理",
                ArchiveJobPromptKind::Risk => "等待风险确认",
                ArchiveJobPromptKind::Destination => "等待目标与容量确认",
                ArchiveJobPromptKind::CreateOverwrite => "等待覆盖确认",
                ArchiveJobPromptKind::OutputBudget => "等待调整输出上限",
            }
            .into();
        }
        if self.kind == ArchiveJobKind::Modify
            && let Some(label) =
                self.progress
                    .as_ref()
                    .and_then(|progress| match progress.phase.as_str() {
                        "edit_planning" => Some("准备 7z 编辑；容量需求未知"),
                        "edit_reading" => Some("读取并检查原归档"),
                        "edit_reencoding" => {
                            Some("完整解码并重编码为非固实 LZMA2；处理量不是保存进度")
                        }
                        "edit_verifying" => Some("校验完整候选；尚未保存"),
                        "edit_committing" => Some("等待原生提交；尚未确认保存"),
                        _ => None,
                    })
        {
            return label.into();
        }
        if self.state == "运行中" {
            return match (self.kind, self.reclamation) {
                (ArchiveJobKind::Extract, Some(ArchiveReclamationMode::Start)) => {
                    "正在空间回收解压：完整文件核验并发布后才释放对应原件数据"
                }
                (ArchiveJobKind::Extract, Some(ArchiveReclamationMode::Continue)) => {
                    "正在继续空间回收解压：验证恢复记录并处理剩余文件"
                }
                (ArchiveJobKind::Extract, None) => "正在解压",
                (ArchiveJobKind::Create, _) => "正在创建压缩包",
                (ArchiveJobKind::Test, _) => "正在校验",
                (ArchiveJobKind::Modify, _) => "正在修改压缩包",
            }
            .into();
        }
        self.state.clone()
    }
    pub(super) fn taskbar(&self, hwnd: i64) -> SetTaskbarProgressCommand {
        let (state, completed, total) = if self.terminal.is_some() {
            (TaskbarProgressState::None, 0, 0)
        } else if self.paused {
            (
                TaskbarProgressState::Paused,
                self.progress.as_ref().map_or(0, |p| p.bytes_done),
                self.progress
                    .as_ref()
                    .and_then(|p| p.bytes_total)
                    .unwrap_or(0),
            )
        } else if let Some(p) = &self.progress {
            if let Some(total) = p.bytes_total.filter(|n| *n > 0) {
                (TaskbarProgressState::Normal, p.bytes_done.min(total), total)
            } else {
                (TaskbarProgressState::Indeterminate, 0, 0)
            }
        } else {
            (TaskbarProgressState::Indeterminate, 0, 0)
        };
        SetTaskbarProgressCommand {
            window_handle: hwnd,
            state,
            completed,
            total,
        }
    }
}
// Complete plain text, not Markdown/Tooltip-only content; visual line breaks remove no characters.
fn prompt_identifier(id: &'static str, value: &str) -> gpui_kit::AnyElement {
    let mut lines = String::with_capacity(value.len() + value.chars().count() / 24);
    for (index, character) in value.chars().enumerate() {
        if index > 0 && index % 24 == 0 {
            lines.push('\n');
        }
        lines.push(character);
    }
    div()
        .id(id)
        .role(gpui_kit::Role::Label)
        .aria_label(value.to_owned())
        .test_support()
        .debug_selector(move || id.into())
        .w_full()
        .min_w_0()
        .child(Label::new(lines).whitespace_normal())
        .into_any_element()
}
fn prompt_objects(prompt: &ArchiveJobPrompt, editing: bool) -> gpui_kit::AnyElement {
    const MISSING: &str = "本次提示未提供对象标识";
    let display = prompt.display.as_ref();
    let mut objects = DescriptionList::vertical().columns(1).bordered(false);
    if editing {
        objects = objects.item(
            "对象位置",
            prompt_identifier("archive-prompt-scope", "归档内已有成员"),
            1,
        );
    }
    if prompt.kind == ArchiveJobPromptKind::Conflict {
        objects = objects.item(
            "传入成员",
            prompt_identifier(
                "archive-prompt-member",
                display
                    .and_then(|d| d.incoming_member.as_deref())
                    .unwrap_or(MISSING),
            ),
            1,
        );
    }
    objects
        .item(
            if prompt.kind == ArchiveJobPromptKind::Conflict {
                if editing {
                    "归档内已有成员"
                } else {
                    "被覆盖目标"
                }
            } else {
                "输出压缩包"
            },
            prompt_identifier(
                "archive-prompt-target",
                display.map_or(MISSING, |d| d.target.as_str()),
            ),
            1,
        )
        .into_any_element()
}
fn number(value: Option<u64>) -> String {
    value.map_or_else(|| "未知".into(), format_bytes)
}
/// Shared archive size grammar (also used by decision prompts; browser caller can reuse).
pub(crate) fn format_bytes(value: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut scaled = value as f64;
    let mut unit = 0;
    while scaled >= 1000. && unit < UNITS.len() - 1 {
        scaled /= 1000.;
        unit += 1;
    }
    if unit == 0 {
        format!("{value} B")
    } else {
        format!("{scaled:.1} {}", UNITS[unit])
    }
}
pub(super) fn progress_percent(progress: &ArchiveJobProgress) -> Option<f32> {
    progress
        .bytes_total
        .filter(|total| *total > 0)
        .map(|total| (progress.bytes_done.min(total) as f64 * 100. / total as f64) as f32)
}
fn byte_percent(done: Option<u64>, total: Option<u64>) -> Option<f32> {
    done.zip(total)
        .filter(|(_, total)| *total > 0)
        .map(|(done, total)| (done.min(total) as f64 * 100. / total as f64) as f32)
}
fn progress_group(
    ids: [&'static str; 3],
    title: &'static str,
    done: Option<u64>,
    total: Option<u64>,
    active: bool,
    cx: &App,
) -> gpui_kit::AnyElement {
    let [id, bar_id, label_id] = ids;
    let percent = byte_percent(done, total);
    let amount = match (done, total) {
        (Some(done), Some(total)) => format!("{} / {}", format_bytes(done), format_bytes(total)),
        (Some(done), None) => format!("{} / 大小未知", format_bytes(done)),
        _ => "大小未知".into(),
    };
    let text = format!(
        "{title} · {amount}{}",
        percent.map_or_else(String::new, |value| format!(" · {value:.0}%"))
    );
    div()
        .id(id)
        .debug_selector(move || id.into())
        .flex()
        .flex_col()
        .gap_1()
        .min_w_0()
        .child(readable(label_id, text))
        .child(
            Progress::new(bar_id)
                .with_size(Size::Medium)
                .accessibility_label(title)
                .loading(
                    active
                        && percent.is_none()
                        && total != Some(0)
                        && crate::ui::motion::motion_allowed(cx),
                )
                .value(percent.unwrap_or(0.0)),
        )
        .into_any_element()
}
fn readable(id: &'static str, value: String) -> gpui_kit::AnyElement {
    div()
        .id(id)
        .debug_selector(move || id.into())
        .role(gpui_kit::Role::Label)
        .aria_label(value.clone())
        .test_support()
        .min_w_0()
        .child(Label::new(value).whitespace_normal())
        .into_any_element()
}
fn status_alert(id: &'static str, message: &'static str, error: bool) -> gpui_kit::AnyElement {
    div()
        .id(id)
        .role(gpui_kit::Role::Group)
        .aria_label(message)
        .test_support()
        .child(if error {
            Alert::error(id, message)
        } else {
            Alert::warning(id, message)
        })
        .into_any_element()
}
impl Focusable for JobProgressView {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}
impl Render for JobProgressView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let terminal = self.terminal.is_some()
            || self.drag.as_ref().is_some_and(|drag| {
                matches!(drag.phase.as_str(), "finished" | "failed" | "cancelled")
            });
        let known = if let Some(drag) = &self.drag {
            (!terminal && drag.phase == "materializing")
                .then(|| {
                    drag.bytes_total.filter(|n| *n > 0).map(|total| {
                        (drag.bytes_done.min(total) as f64 * 100. / total as f64) as f32
                    })
                })
                .flatten()
        } else {
            (!terminal)
                .then(|| self.progress.as_ref().and_then(progress_percent))
                .flatten()
        };
        let ordinary_extract_running = !terminal
            && self.drag.is_none()
            && self.kind == ArchiveJobKind::Extract
            && self.reclamation.is_none()
            && !self.paused
            && self.pause_requested.is_none()
            && !self.cancel_requested
            && self.prompt.is_none()
            && self
                .progress
                .as_ref()
                .is_some_and(|progress| progress.phase == "running");
        let mut body = div()
            .id("archive-progress-body")
            .debug_selector(|| "archive-progress-body".into())
            .flex_none()
            .min_h_0()
            .p_3()
            .flex()
            .flex_col()
            .gap_2()
            .when(!ordinary_extract_running, |body| {
                body.child(readable("archive-progress-stage", self.stage_label()))
            });
        if !terminal && self.drag.is_none() && self.kind == ArchiveJobKind::Extract {
            let progress = self.progress.as_ref();
            let active = !self.paused
                && self.prompt.is_none()
                && !self.cancel_requested
                && progress.is_some_and(|progress| progress.phase == "running");
            let member = progress
                .and_then(|progress| progress.current_member.clone())
                .unwrap_or_else(|| "等待处理文件".into());
            body = body
                .gap_1()
                .child(
                    div()
                        .id("archive-progress-member")
                        .debug_selector(|| "archive-progress-member".into())
                        .role(gpui_kit::Role::Label)
                        .aria_label(member.clone())
                        .test_support()
                        .min_w_0()
                        .child(Label::new(member).truncate()),
                )
                .child(progress_group(
                    [
                        "archive-current-file-progress",
                        "archive-current-file-bar",
                        "archive-current-file-amount",
                    ],
                    "当前文件",
                    progress.and_then(|progress| progress.current_bytes_done),
                    progress.and_then(|progress| progress.current_bytes_total),
                    active && progress.is_some_and(|progress| progress.current_member.is_some()),
                    cx,
                ))
                .child(progress_group(
                    [
                        "archive-overall-progress",
                        "archive-job-progress",
                        "archive-overall-amount",
                    ],
                    "总体进度",
                    progress.map(|progress| progress.bytes_done),
                    progress.and_then(|progress| progress.bytes_total),
                    active,
                    cx,
                ));
        } else if !terminal && (known.is_some() || crate::ui::motion::motion_allowed(cx)) {
            body = body.child(
                Progress::new("archive-job-progress")
                    .accessibility_label("文件处理进度，非已保存进度")
                    .loading(known.is_none() && crate::ui::motion::motion_allowed(cx))
                    .value(known.unwrap_or(0.0)),
            );
        }
        if let Some(drag) = &self.drag {
            body=body.child(readable("archive-drag-processed",format!("缓存内容 {} / {} · 已就绪文件 {} / {}",format_bytes(drag.bytes_done),number(drag.bytes_total),drag.files_done,drag.files_total)))
                .child(readable("archive-drag-boundary","缓存物化完成不代表接收方已完成目标写入；外部目标目录不可可靠获知。关闭窗口不会取消拖出。".into()));
            if let Some(member) = &drag.current_member {
                body = body.child(readable("archive-progress-member", member.clone()));
            }
            if drag.cleanup_complete == Some(false) {
                body = body.child(Alert::warning(
                    "archive-drag-cleanup",
                    "拖出内容仍在使用或清理尚未确认，请稍后再试。",
                ));
            }
        } else if let Some(result) = &self.terminal {
            let result_text = if self.kind == ArchiveJobKind::Test {
                format!(
                    "实际验证 {} 个文件 · {}",
                    result.files_verified,
                    format_bytes(result.bytes_verified)
                )
            } else {
                format!(
                    "实际提交 {} 个文件 · {}；跳过 {} 个",
                    result.files_committed,
                    format_bytes(result.bytes_committed),
                    result.files_skipped
                )
            };
            body = body
                .child(readable("archive-progress-result", result_text))
                .child(readable(
                    "archive-progress-elapsed",
                    format!(
                        "耗时 {}",
                        crate::main_window::format_duration(result.elapsed_ms / 1000)
                    ),
                ));
            if self.reclamation.is_some() {
                let (id, message) = match result.outcome {
                    ArchiveJobOutcome::Succeeded | ArchiveJobOutcome::SucceededWithWarnings => (
                        "archive-reclamation-completed",
                        "空间回收解压已完成。原压缩包保留为已消耗的稀疏外壳，不是完整可用归档；请保留恢复记录和已解压输出。本版本不能恢复完整原压缩包。",
                    ),
                    ArchiveJobOutcome::Cancelled => (
                        "archive-reclamation-cancelled",
                        "空间回收解压已取消，只停止后续单元；已释放的数据不会恢复。请保留原压缩包、恢复记录和已解压输出，并用“继续空间回收解压”处理剩余文件；本版本不能恢复完整原压缩包。",
                    ),
                    ArchiveJobOutcome::Failed => (
                        "archive-reclamation-failed",
                        "空间回收解压在已确认边界停止；已释放的数据不会恢复，原压缩包可能已是已消耗外壳。请保留原压缩包、恢复记录和已解压输出；只能继续剩余解压，不能恢复完整原压缩包。",
                    ),
                    ArchiveJobOutcome::Rejected => (
                        "archive-reclamation-rejected",
                        "本次未开始新的空间释放。若原压缩包已有恢复标记，仍须保留原压缩包、恢复记录和已有输出，并从“继续空间回收解压”进入；不要使用普通重试。",
                    ),
                };
                body = body.child(
                    div()
                        .id("archive-reclamation-summary")
                        .role(gpui_kit::Role::Group)
                        .aria_label(message)
                        .test_support()
                        .child(Alert::warning(id, message).with_size(Size::Small)),
                );
            }
            if result.files_committed > 0
                && !matches!(
                    result.outcome,
                    ArchiveJobOutcome::Succeeded | ArchiveJobOutcome::SucceededWithWarnings
                )
            {
                body = body.child(status_alert(
                    "archive-progress-partial",
                    "任务未全部完成；已提交的输出仍然保留，不代表整体回滚。",
                    false,
                ));
            }
            if result.outcome == ArchiveJobOutcome::Failed {
                body = body.child(status_alert(
                    "archive-job-failure",
                    "任务失败。请检查源文件、目标位置及可用空间；已提交的文件不会自动撤销。展开诊断信息可查看错误代码。", true,
                ));
            } else if result.outcome == ArchiveJobOutcome::Rejected {
                body = body.child(status_alert(
                    "archive-job-refused",
                    "无法开始此任务。请检查所选压缩包和目标位置；展开诊断信息查看拒绝原因。",
                    false,
                ));
            }
            if !result.cleanup_complete {
                body = body.child(status_alert(
                    "archive-job-cleanup-pending",
                    "清理尚未确认；请勿立即重新运行或移动相关文件。",
                    false,
                ));
            }
            if result.commit_state.is_some()
                || result.native_error_code.is_some()
                || result.code.is_some()
            {
                let mut diagnostics = div().flex().flex_col().gap_1();
                if let Some(commit) = &result.commit_state {
                    diagnostics = diagnostics.child(readable(
                        "archive-progress-commit",
                        format!("提交状态：{commit}"),
                    ));
                }
                if let Some(code) = result.native_error_code {
                    diagnostics = diagnostics.child(readable(
                        "archive-progress-native",
                        format!("原生错误代码：{code}"),
                    ));
                }
                if let Some(code) = &result.code {
                    diagnostics = diagnostics.child(readable(
                        "archive-job-terminal-code",
                        format!("诊断代码：{code}"),
                    ));
                }
                body = body.child(
                    Collapsible::new()
                        .open(self.diagnostics_open)
                        .child(
                            Button::new("archive-progress-diagnostics-toggle")
                                .debug_selector(|| "archive-progress-diagnostics-toggle".into())
                                .label(if self.diagnostics_open {
                                    "收起诊断信息"
                                } else {
                                    "查看诊断信息"
                                })
                                .outline()
                                .with_size(Size::Small)
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.diagnostics_open = !this.diagnostics_open;
                                    cx.notify();
                                })),
                        )
                        .content(diagnostics),
                );
            }
        } else if let Some(progress) = &self.progress {
            if self.kind != ArchiveJobKind::Extract
                && let Some(member) = &progress.current_member
            {
                body = body.child(DescriptionList::vertical().columns(1).bordered(false).item(
                    "当前成员",
                    readable("archive-progress-member", member.clone()),
                    1,
                ));
            }
            body = body
                .when(self.kind != ArchiveJobKind::Extract, |body| {
                    body.child(readable(
                        "archive-progress-processed",
                        format!(
                            "已处理 {} / {} · 文件 {} / {}（处理量不等于实际提交量）",
                            format_bytes(progress.bytes_done),
                            number(progress.bytes_total),
                            progress.files_done,
                            progress
                                .files_total
                                .map_or_else(|| "未知".into(), |n| n.to_string())
                        ),
                    ))
                })
                .child(readable(
                    "archive-progress-elapsed",
                    format!(
                        "{}已用 {}{}{}",
                        if self.kind == ArchiveJobKind::Extract {
                            format!(
                                "文件 {} / {} · ",
                                progress.files_done,
                                progress
                                    .files_total
                                    .map_or_else(|| "未知".into(), |total| total.to_string())
                            )
                        } else {
                            String::new()
                        },
                        crate::main_window::format_duration(self.elapsed_ms / 1000),
                        self.rate
                            .rate()
                            .map_or_else(String::new, |rate| if rate < 1.0 {
                                " · 等待处理数据".into()
                            } else {
                                format!(
                                    " · 处理速度 {}/s",
                                    format_bytes(rate.min(u64::MAX as f64) as u64)
                                )
                            }),
                        self.rate
                            .eta(progress.bytes_done, progress.bytes_total)
                            .map_or_else(String::new, |seconds| format!(
                                " · 预计剩余 {}",
                                crate::main_window::format_duration(seconds)
                            ))
                    ),
                ));
        }
        if self.summary
            && self.prompt.is_none()
            && let Some(batch) = &self.batch
        {
            body = div()
                .id("archive-progress-body")
                .p_3()
                .flex()
                .flex_col()
                .gap_2()
                .child(super::batch::render(batch, self.owner.clone()));
        }
        if let Some(status) = &self.location_status {
            body = body.child(readable("archive-result-location-status", status.clone()));
        }
        let footer = div()
            .debug_selector(|| "archive-progress-footer".into())
            .flex_none()
            .p_3()
            .flex()
            .flex_col()
            .gap_2()
            .when_some(self.control_status.as_ref(), |footer, status| footer.child(readable("archive-control-status", status.clone())))
            .when(self.drag.is_none() && self.batch.is_some(), |footer|footer.child(Button::new("archive-job-batch").label(if self.summary {"任务详情"}else{"批次汇总"}).outline().with_size(Size::Medium).disabled(self.prompt.is_some()).on_click(cx.listener(|this,_,_,cx|{this.summary = !this.summary;cx.notify();}))))
            .child(
                Checkbox::new("archive-job-keep-open")
                    .label("完成后保留窗口")
                    .checked(self.keep_open)
                    .disabled(self.drag.is_some())
                    .on_click(cx.listener(|this, value, _, cx| {
                        this.keep_open = *value;
                        let owner = this.owner.clone();
                        let id = this.id;
                        let value = *value;
                        tracing::info!(event="archive.job.keep_open_requested", job_id=%id, value, "archive completion policy requested");
                        cx.defer(move |cx| {
                            let _ = owner.update(cx, |app, cx| { app.keep_job_open(id, value, cx); });
                        });
                        cx.notify();
                    })),
            )
            .when(self.drag.is_none() && self.terminal.as_ref().is_some_and(|result| result.cleanup_complete && result.outcome==ArchiveJobOutcome::Succeeded && result.result_location.is_some()),|footer|footer.child(
                Button::new("archive-job-locate").label("定位结果").outline().on_click(cx.listener(|this,_,_,cx|{
                    let owner=this.owner.clone();let id=this.id;cx.defer(move|cx|{let _=owner.update(cx,|app,cx|app.locate_job_result(id,cx));});
                }))
            ))
            .when_some(self.continuation.filter(|_| self.drag.is_none()), |footer, (attempt, generation)| footer.child(
                Button::new("archive-job-reclamation-continue")
                    .debug_selector(|| "archive-job-reclamation-continue".into())
                    .label("检查并继续解压…").outline().with_size(Size::Medium)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        let owner = this.owner.clone();
                        let id = this.id;
                        cx.defer(move |cx| {
                            let _ = owner.update(cx, |app, cx| app.continue_reclamation_job(id, attempt, generation, cx));
                        });
                    }))
            ))
            .child(div().flex().justify_end().gap_2()
            .child(
                Button::new("archive-job-pause")
                    .label(if self.paused {
                        if self.reclamation.is_some() {
                            "继续当前任务"
                        } else {
                            "继续"
                        }
                    } else {
                        "暂停"
                    })
                    .outline()
                    .disabled(self.drag.is_some() || terminal || self.prompt.is_some() || self.pause_requested.is_some() || self.cancel_requested)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.command(
                            if this.paused {
                                ArchiveJobAction::Resume
                            } else {
                                ArchiveJobAction::Pause
                            },
                            cx,
                        )
                    })),
            )
            .child(
                Button::new("archive-job-stop")
                    .label("停止")
                    .outline()
                    .disabled(self.drag.is_some() || terminal || self.cancel_requested)
                    .on_click(
                        cx.listener(|this, _, _, cx| this.command(ArchiveJobAction::Cancel, cx)),
                    ),
            )
            .child(
                Button::new("archive-job-background")
                    .label(if terminal { "关闭" } else { "后台" })
                    .outline()
                    .on_click(cx.listener(|this, _, window, cx| this.background(window, cx))),
            ));
        div()
            .id("archive-progress-window")
            .size_full()
            .flex()
            .flex_col()
            .bg(theme.colors.background)
            .text_color(theme.colors.foreground)
            .font_family(crate::ui::typography::UI_FONT_FAMILY)
            .min_h_0()
            .child(crate::ui::components::titlebar::controlled_titlebar(
                "archive-progress-titlebar",
                "archive-progress-close",
                match (self.kind, self.reclamation) {
                    (ArchiveJobKind::Extract, Some(ArchiveReclamationMode::Continue)) => {
                        "继续空间回收解压"
                    }
                    (ArchiveJobKind::Extract, Some(ArchiveReclamationMode::Start)) => {
                        "空间回收解压"
                    }
                    (ArchiveJobKind::Extract, None) => "解压压缩包",
                    (ArchiveJobKind::Create, _) => "新建压缩包",
                    (ArchiveJobKind::Test, _) => "测试压缩包",
                    (ArchiveJobKind::Modify, _) => "修改压缩包",
                },
                crate::ui::components::titlebar::CaptionCapabilities::RESIZABLE,
                cx.listener(|this, _, window, cx| this.background(window, cx)),
            ))
            .child(
                div()
                    .id("archive-progress-scroll")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .child(body),
            )
            .child(footer)
    }
}
pub(super) fn taskbar(
    _id: Uuid,
    window: &Window,
    view: WeakEntity<JobProgressView>,
    cx: &App,
) -> SetTaskbarProgressCommand {
    let hwnd = crate::ui::native_window_handle(window).unwrap_or(0);
    view.read_with(cx, |v, _| v.taskbar(hwnd))
        .unwrap_or(SetTaskbarProgressCommand {
            window_handle: hwnd,
            state: TaskbarProgressState::None,
            completed: 0,
            total: 0,
        })
}

#[cfg(test)]
#[path = "progress_feedback_tests.rs"]
mod feedback_tests;
