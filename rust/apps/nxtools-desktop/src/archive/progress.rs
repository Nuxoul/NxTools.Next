//! Official progress/prompt presentation; no execution, notification clock or native owner.
use super::ArchiveApplication;
use gpui_kit::component::{
    ActiveTheme, Disableable, Sizable, Size, WindowExt,
    alert::Alert,
    button::Button,
    checkbox::Checkbox,
    collapsible::Collapsible,
    description_list::DescriptionList,
    dialog::{Confirm, DialogAction, DialogClose, DialogFooter},
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

#[derive(Clone, Copy, Default)]
pub(super) struct JobControls {
    pub pause: bool,
    pub stop: bool,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct StopQuestion {
    pub token: Uuid,
    pub waiting: bool,
}
pub(super) struct ProgressSnapshot {
    pub id: Uuid,
    pub retry: bool,
    pub retry_pending: bool,
    pub operation: Uuid,
    pub generation: u64,
    pub continuation: Option<(Uuid, u64)>,
    pub state: String,
    pub kind: ArchiveJobKind,
    pub edit_format: Option<String>,
    pub elapsed_ms: u64,
    pub reclamation: Option<ArchiveReclamationMode>,
    pub progress_elapsed_ms: u64,
    pub progress: Option<ArchiveJobProgress>,
    pub terminal: Option<ArchiveJobTerminal>,
    pub prompt: Option<ArchiveJobPrompt>,
    pub paused: bool,
    pub controls: JobControls,
    pub stop_question: Option<StopQuestion>,
    pub location_status: Option<String>,
    pub pause_requested: Option<bool>,
    pub cancel_requested: bool,
    pub control_status: Option<String>,
    pub batch: Option<super::batch::BatchSnapshot>,
}
pub(super) const DRAG_TITLE: &str = "拖出解压";

pub(super) struct JobProgressView {
    id: Uuid,
    owner: WeakEntity<ArchiveApplication>,
    focus: FocusHandle,
    state: String,
    kind: ArchiveJobKind,
    edit_format: Option<String>,
    elapsed_ms: u64,
    reclamation: Option<ArchiveReclamationMode>,
    continuation: Option<(Uuid, u64)>,
    progress: Option<ArchiveJobProgress>,
    terminal: Option<ArchiveJobTerminal>,
    prompt: Option<ArchiveJobPrompt>,
    shown_prompt: Option<ArchiveJobTicket>,
    operation: Uuid,
    generation: u64,
    controls: JobControls,
    stop_question: Option<StopQuestion>,
    shown_stop: std::rc::Rc<std::cell::Cell<Option<Uuid>>>,
    stop_waiting: std::rc::Rc<std::cell::Cell<bool>>,
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
    retry: bool,
    retry_pending: bool,
    diagnostics_open: bool,
}
impl JobProgressView {
    pub(super) fn new_drag(
        value: nxtools_protocol::archive_virtual_drag::ArchiveVirtualDragProgress,
        owner: WeakEntity<ArchiveApplication>,
        cx: &mut Context<Self>,
    ) -> Self {
        let mut view = Self::new(value.operation_id, owner, true, cx);
        view.apply_drag(value, 0, cx);
        view
    }

    fn title(&self) -> &'static str {
        if self.drag.is_some() {
            return DRAG_TITLE;
        }
        match (self.kind, self.reclamation) {
            (ArchiveJobKind::Extract, Some(ArchiveReclamationMode::Continue)) => "继续空间回收解压",
            (ArchiveJobKind::Extract, Some(ArchiveReclamationMode::Start)) => "空间回收解压",
            (ArchiveJobKind::Extract, None) => "解压压缩包",
            (ArchiveJobKind::Create, _) => "新建压缩包",
            (ArchiveJobKind::Test, _) => "测试压缩包",
            (ArchiveJobKind::Modify, _) => "修改压缩包",
        }
    }

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
            edit_format: None,
            elapsed_ms: 0,
            reclamation: None,
            continuation: None,
            progress: None,
            terminal: None,
            prompt: None,
            shown_prompt: None,
            operation: id,
            generation: 1,
            controls: JobControls::default(),
            stop_question: None,
            shown_stop: Default::default(),
            stop_waiting: Default::default(),
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
            retry: false,
            retry_pending: false,
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
            id,
            retry,
            retry_pending,
            operation,
            generation,
            continuation,
            state,
            kind,
            edit_format,
            elapsed_ms,
            reclamation,
            progress_elapsed_ms,
            progress,
            terminal,
            prompt,
            paused,
            controls,
            stop_question,
            location_status,
            pause_requested,
            cancel_requested,
            control_status,
            batch,
        } = snapshot;
        if self.id != id {
            self.shown_prompt = None;
            self.shown_stop.set(None);
            for input in self.prompt_inputs.drain(..) {
                input.0.set(false);
            }
            window.close_dialog(cx);
            self.rate.clear();
        }
        self.id = id;
        self.retry = retry;
        self.retry_pending = retry_pending;
        self.operation = operation;
        self.generation = generation;
        self.controls = controls;
        self.stop_question = stop_question;
        self.stop_waiting
            .set(stop_question.is_some_and(|question| question.waiting));
        if self.shown_stop.get().is_some()
            && (self.shown_stop.get() != stop_question.map(|question| question.token)
                || self.shown_prompt.as_ref() != prompt.as_ref().map(|prompt| &prompt.ticket))
        {
            self.shown_stop.set(None);
            window.close_dialog(cx);
        }
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
        self.edit_format = edit_format;
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
            && !self.summary
            && self.shown_prompt.as_ref() != Some(&prompt.ticket)
        {
            if self.shown_prompt.is_some() {
                window.close_dialog(cx);
            }
            self.shown_prompt = Some(prompt.ticket.clone());
            self.show_prompt(prompt, window, cx);
        }
        if let Some(question) = self.stop_question
            && self.shown_stop.get() != Some(question.token)
        {
            self.shown_stop.set(Some(question.token));
            self.show_stop(question.token, window, cx);
        }
        if self
            .terminal
            .as_ref()
            .is_some_and(|result| result.outcome == ArchiveJobOutcome::Succeeded)
            && !self.keep_open
            && self.batch.is_none()
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
    pub(super) fn show_detail(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.summary = false;
        if let Some(prompt) = self.prompt.clone()
            && self.shown_prompt.as_ref() != Some(&prompt.ticket)
        {
            self.shown_prompt = Some(prompt.ticket.clone());
            self.show_prompt(prompt, window, cx);
        }
        cx.notify();
    }
    fn show_summary(&mut self, cx: &mut Context<Self>) {
        self.summary = true;
        self.shown_prompt = None;
        for input in self.prompt_inputs.drain(..) {
            input.0.set(false);
        }
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
        self.rate.clear(); // Receiver writes are not measured by extraction progress.
        self.refresh = None;
        self.elapsed_ms = elapsed_ms;
        self.drag = Some(value);
        cx.notify();
    }
    fn command(&mut self, action: ArchiveJobAction, cx: &mut Context<Self>) {
        if self.drag.is_some() {
            return;
        }
        if action == ArchiveJobAction::Cancel {
            if !self.controls.stop {
                return;
            }
            self.controls.stop = false;
            cx.notify();
            let owner = self.owner.clone();
            let id = self.id;
            let attempt = self.operation;
            let generation = self.generation;
            cx.defer(move |cx| {
                let _ = owner.update(cx, |app, cx| {
                    app.request_job_stop(id, attempt, generation, cx)
                });
            });
            return;
        }
        if !self.controls.pause {
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
    fn show_stop(&mut self, token: Uuid, window: &mut Window, cx: &mut Context<Self>) {
        if !cx.has_global::<PromptButtonKeys>() {
            cx.bind_keys([gpui_kit::KeyBinding::new(
                "enter",
                gpui_kit::NoAction,
                Some("ArchivePromptButton"),
            )]);
            cx.set_global(PromptButtonKeys);
        }
        let confirm_anchor = cx.focus_handle();
        let owner = self.owner.clone();
        let id = self.id;
        let waiting = self.stop_waiting.clone();
        let shown = self.shown_stop.clone();
        let answered = std::rc::Rc::new(std::cell::Cell::new(false));
        let explicit = std::rc::Rc::new(std::cell::Cell::new(false));
        let confirm_input = PromptButtonInput::default();
        let decline_input = PromptButtonInput::default();
        self.prompt_inputs
            .extend([confirm_input.clone(), decline_input.clone()]);
        window.open_dialog(cx, move |dialog, _, _| {
            let ok_owner = owner.clone();
            let cancel_owner = owner.clone();
            let close_owner = owner.clone();
            let ok_answered = answered.clone();
            let cancel_answered = answered.clone();
            let close_answered = answered.clone();
            let ok_shown = shown.clone();
            let cancel_shown = shown.clone();
            let close_shown = shown.clone();
            let button_explicit = explicit.clone();
            let ok_explicit = explicit.clone();
            let ok_waiting = waiting.clone();
            let action_anchor = confirm_anchor.clone();
            dialog.title("停止当前任务？")
                .child(if waiting.get() {
                    "正在等待任务确认暂停。控制回执不代表已经暂停；关闭此确认会继续原任务。"
                } else {
                    "停止后将等待原生清理，已经提交的输出不会撤销。选择继续任务会保留原来的暂停状态。"
                })
                .footer(DialogFooter::new()
                    .child(DialogClose::new().trigger(|button| decline_input.guard(button.label("继续任务").outline().with_size(Size::Medium))))
                    .child(DialogAction::new().child(confirm_input.guard(Button::new("archive-job-stop-confirm").label("确认停止").with_size(Size::Medium)).disabled(waiting.get()).child(div().absolute().size_0().track_focus(&confirm_anchor)).on_click(move |event, window, cx| {
                        button_explicit.set(true);
                        if event.is_keyboard() {
                            action_anchor.dispatch_action(&Confirm { secondary: false }, window, cx);
                        }
                    }))))
                .on_ok(move |_, _, cx| {
                    if ok_waiting.get() || !ok_explicit.replace(false) || ok_answered.replace(true) { return false; }
                    ok_shown.set(None);
                    let owner = ok_owner.clone();
                    cx.defer(move |cx| { let _ = owner.update(cx, |app, cx| app.answer_job_stop(id, token, true, cx)); });
                    true
                })
                .on_cancel(move |_, _, cx| {
                    if !cancel_answered.replace(true) {
                        cancel_shown.set(None);
                        let owner = cancel_owner.clone();
                        cx.defer(move |cx| { let _ = owner.update(cx, |app, cx| app.answer_job_stop(id, token, false, cx)); });
                    }
                    true
                })
                .on_close(move |_, _, cx| {
                    if !close_answered.replace(true) {
                        close_shown.set(None);
                        let owner = close_owner.clone();
                        cx.defer(move |cx| { let _ = owner.update(cx, |app, cx| app.answer_job_stop(id, token, false, cx)); });
                    }
                })
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
        let attempt = self.operation;
        let generation = self.generation;
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
        let reuse = std::rc::Rc::new(std::cell::Cell::new(false));
        let progress_view = cx.entity().downgrade();
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
        window.open_dialog(cx,move|dialog,_,cx|{
            let reusable = owner.read_with(cx, |app,_| app.batch_password_eligible(id)).unwrap_or(false);
            let ok_owner=owner.clone();let cancel_owner=owner.clone();let ok_prompt=prompt.clone();let ok_input=input.clone();let ok_apply=apply.clone();let ok_reuse=reuse.clone();
            let button_confirm=explicit_confirm.clone();let ok_explicit=explicit_confirm.clone();let action_anchor=confirm_anchor.clone();
            let body=match prompt.kind{
                ArchiveJobPromptKind::Password=>Form::vertical()
                    .child(Field::new().label(if prompt.error_code.as_deref()==Some("WrongPassword"){ "密码错误，请重试" }else{"当前压缩包密码"})
                        .child(Input::new(&input).mask_toggle().aria_label("当前任务密码")))
                    .when(reusable || reuse.get(), |form| form.child(Field::new()
                        .when(!reusable, |field| field.description("本批次的密码复用已结束；取消勾选后可仅回复当前任务。"))
                        .child(
                        Checkbox::new("archive-password-batch-reuse").label("仅在本批次复用密码（仅内存）")
                            .checked(reuse.get()).on_click({let reuse=reuse.clone();move|value,_,cx|{reuse.set(*value);cx.refresh_windows();}}))))
                    .into_any_element(),
                ArchiveJobPromptKind::Conflict=>div().flex().flex_col().gap_2().child(if editing {"添加到压缩包时遇到同名成员。覆盖只在完整候选验证后一次提交；不会先删除原件。日期：未知。"} else {"目标已有同名文件。"}).child(prompt_identifier("archive-prompt-sizes",&format!("现有大小：{}；传入大小：{}。",number(prompt.existing_bytes),number(prompt.incoming_bytes)))).child(prompt_objects(&prompt, editing)).into_any_element(),
                ArchiveJobPromptKind::Risk=>div().child("此文件的解压大小可能远大于压缩后大小。继续仍受当前大小限制约束；取消将停止任务。").into_any_element(),
                ArchiveJobPromptKind::OutputBudget=>div().child(format!("上次尝试未提交文件且已完成清理。若要重新尝试，需要将单文件上限设为 {}、总输出上限设为 {}。仅这两个上限会随新尝试增加。",number(prompt.existing_bytes),number(prompt.incoming_bytes))).into_any_element(),
                ArchiveJobPromptKind::CreateOverwrite=>div().flex().flex_col().gap_2().child("目标压缩包已存在。确认覆盖后将先完成验证，再替换原文件；若目标发生变化则停止。").child(prompt_objects(&prompt, false)).into_any_element(),
                ArchiveJobPromptKind::Destination=>div().child(if prompt.error_code.as_deref()==Some("create_destination"){format!("目标文件夹不存在。要创建文件夹并继续吗？预计输出：{}；可用空间：{}。",number(prompt.incoming_bytes),number(prompt.existing_bytes))}else{format!("目标可用空间不足。预计输出：{}；可用空间：{}。仍要继续吗？如果写入失败，任务会停止。",number(prompt.incoming_bytes),number(prompt.existing_bytes))}).into_any_element(),
            };
            let mut switches = div().flex().flex_wrap().gap_2();
            if let Some(batch) = progress_view.read_with(cx, |view,_| view.batch.clone()).ok().flatten() {
                for (index, member) in batch.members.iter().enumerate().filter(|(_,member)| member.state == super::batch::MemberState::Waiting) {
                    let target = member.id;
                    let owner = owner.clone();
                    let input = input.clone();
                    switches = switches.child(Button::new(gpui_kit::SharedString::from(format!("archive-prompt-member-{target}")))
                        .label(format!("处理待决事项 {}", index + 1)).outline().with_size(Size::Medium).disabled(target == id)
                        .on_click(move|_,window,cx| {
                            input.update(cx, |input,cx| input.set_value("",window,cx));
                            window.close_dialog(cx);
                            let owner=owner.clone();
                            cx.defer(move|cx|{let _=owner.update(cx,|app,cx|app.show_job(target,cx));});
                        }));
                }
                let view=progress_view.clone();let input=input.clone();
                switches=switches.child(Button::new("archive-prompt-summary").label("批次汇总").outline().with_size(Size::Medium).on_click(move|_,window,cx| {
                    input.update(cx, |input,cx|input.set_value("",window,cx));window.close_dialog(cx);
                    let view=view.clone();cx.defer(move|cx|{let _=view.update(cx,|view,cx|view.show_summary(cx));});
                }));
            }
            let body=div().flex().flex_col().gap_3().child(switches).child(body);
            let stop_owner=owner.clone();
            let mut footer=DialogFooter::new().child(cancel_input_guard.guard(Button::new("archive-prompt-stop").label("取消任务").outline().with_size(Size::Medium)).on_click(move|_,_,cx|{
                let owner=stop_owner.clone();cx.defer(move|cx|{let _=owner.update(cx,|app,cx|app.request_job_stop(id,attempt,generation,cx));});
            }));
            if prompt.kind==ArchiveJobPromptKind::Conflict{
                for (index,(label,decision)) in [("覆盖",ArchiveJobConflictDecision::Overwrite),("跳过",ArchiveJobConflictDecision::Skip),("自动重命名",ArchiveJobConflictDecision::AutoRename)].into_iter().enumerate(){
                    let response_owner=owner.clone();let response_prompt=prompt.clone();let apply=apply.clone();
                    footer=footer.child(conflict_inputs[index].guard(Button::new(gpui_kit::SharedString::from(format!("archive-conflict-{}-{label}",prompt.ticket.prompt_id))).label(label).outline().with_size(Size::Medium)).on_click(move|_,window,cx|{let owner=response_owner.clone();let prompt=response_prompt.clone();let reply=ArchiveJobReply{ticket:prompt.ticket.clone(),kind:prompt.kind,password:None,remember_batch_password:false,decision:Some(decision),apply_to_all:apply.get(),approved_limit:None};window.close_dialog(cx);cx.defer(move|cx|{let _=owner.update(cx,|app,cx|app.reply_job(id,prompt,reply,cx));});}));
                }
            }else if matches!(prompt.kind,ArchiveJobPromptKind::Password|ArchiveJobPromptKind::Risk|ArchiveJobPromptKind::OutputBudget|ArchiveJobPromptKind::CreateOverwrite|ArchiveJobPromptKind::Destination){footer=footer.child(confirm_input.guard(Button::new(gpui_kit::SharedString::from(format!("archive-job-prompt-ok-{}",prompt.ticket.prompt_id))).label(if prompt.kind==ArchiveJobPromptKind::CreateOverwrite{"确认覆盖"}else{"继续"})).debug_selector(||"archive-job-prompt-ok".into()).child(div().absolute().size_0().track_focus(&confirm_anchor)).on_click(move|_,window,cx|{button_confirm.set(true);action_anchor.dispatch_action(&Confirm{secondary:false},window,cx);}));}
            let footer=if prompt.kind==ArchiveJobPromptKind::Conflict{
                div().flex_none().flex().flex_col().gap_2()
                    .child(Checkbox::new("archive-conflict-all").label("应用到此任务后续冲突").checked(apply.get()).on_click({let apply=apply.clone();move|checked,_,_|apply.set(*checked)}))
                    .child(footer).into_any_element()
            }else{footer.into_any_element()};
            dialog.title(match prompt.kind{ArchiveJobPromptKind::Password=>"压缩包需要密码",ArchiveJobPromptKind::Conflict=>"处理同名文件",ArchiveJobPromptKind::Risk=>"确认高膨胀率",ArchiveJobPromptKind::Destination=>"确认目标目录与空间",_=>"压缩包操作确认"}).child(body).footer(footer)
                .on_ok(move |_, window, cx| {
                    if ok_prompt.kind == ArchiveJobPromptKind::CreateOverwrite && !ok_explicit.replace(false) {
                        return false;
                    }
                    let remember_batch_password = ok_prompt.kind == ArchiveJobPromptKind::Password && ok_reuse.get();
                    if remember_batch_password
                        && !ok_owner.read_with(cx, |app, _| app.batch_password_eligible(id)).unwrap_or(false)
                    {
                        return false;
                    }
                    let password = (ok_prompt.kind == ArchiveJobPromptKind::Password)
                        .then(|| PrivatePassword::new(ok_input.read(cx).value().as_ref()));
                    ok_input.update(cx, |input, cx| input.set_value("", window, cx));
                    let prompt = ok_prompt.clone();
                    let reply = ArchiveJobReply {
                        ticket: prompt.ticket.clone(), kind: prompt.kind, password, remember_batch_password,
                        decision: matches!(prompt.kind, ArchiveJobPromptKind::CreateOverwrite | ArchiveJobPromptKind::Destination)
                            .then_some(ArchiveJobConflictDecision::Overwrite),
                        apply_to_all: ok_apply.get(), approved_limit: prompt.approved_limit,
                    };
                    let owner = ok_owner.clone();
                    cx.defer(move |cx| { let _ = owner.update(cx, |app, cx| app.reply_job(id, prompt, reply, cx)); });
                    true
                })
                .on_cancel(move|_,_,cx|{let owner=cancel_owner.clone();cx.defer(move|cx|{let _=owner.update(cx,|app,cx|app.request_job_stop(id,attempt,generation,cx));});false})
        });
    }
    fn stage_label(&self) -> String {
        if self.terminal.is_some() {
            return self.state.clone();
        }
        if let Some(drag) = &self.drag {
            return match drag.phase.as_str() {
                "materializing" => "正在解压",
                "content_ready" | "target_copying" => "等待接收方完成",
                "finished" if drag.code.as_deref() == Some("receiver_reported_complete") => {
                    "已完成"
                }
                "finished" => "拖出已结束",
                "cancelled" => "拖出已取消",
                _ => match drag.code.as_deref() {
                    Some("disconnected") => "连接已断开，请重新连接后重试",
                    Some("source_changed" | "source_unavailable") => {
                        "源文件已变化或不可用，请刷新后重试"
                    }
                    Some("virtual_receiver_failed" | "receiver_aborted") => {
                        "接收方未完成拖出，请重试"
                    }
                    Some("resource_limit") => "解压超过可用资源限制",
                    _ => "拖出失败，请重试",
                },
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
                        "edit_planning" => Some(match self.edit_format.as_deref() {
                            Some("SevenZ") => "准备 7z 编辑；容量需求未知",
                            Some("Tar") => "准备 TAR 编辑；容量需求未知",
                            Some("TarGz") => "准备 TAR.GZ 编辑；容量需求未知",
                            Some("TarXz") => "准备 TAR.XZ 编辑；容量需求未知",
                            _ => "准备压缩包编辑；容量需求未知",
                        }),
                        "edit_reading" => Some("读取并检查原压缩包"),
                        "edit_reencoding" => Some(match self.edit_format.as_deref() {
                            Some("SevenZ") => "完整解码并重编码为非固实 LZMA2；处理量不是保存进度",
                            Some("Tar" | "TarGz" | "TarXz") => {
                                "完整重写 TAR 压缩包及外层压缩（如有）；处理量不是保存进度"
                            }
                            _ => "重写压缩包；处理量不是保存进度",
                        }),
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
            prompt_identifier("archive-prompt-scope", "压缩包内已有成员"),
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
                    "压缩包内已有成员"
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
        if !terminal && (self.drag.is_some() || self.kind == ArchiveJobKind::Extract) {
            let progress = self.progress.as_ref();
            let drag = self.drag.as_ref();
            let active = drag.map_or_else(
                || {
                    !self.paused
                        && self.prompt.is_none()
                        && !self.cancel_requested
                        && progress.is_some_and(|progress| progress.phase == "running")
                },
                |drag| drag.phase == "materializing",
            );
            let member = drag
                .and_then(|drag| drag.current_member.clone())
                .or_else(|| progress.and_then(|progress| progress.current_member.clone()))
                .unwrap_or_else(|| "等待处理文件".into());
            let current_done = drag.map_or_else(
                || progress.and_then(|p| p.current_bytes_done),
                |d| d.current_bytes_done,
            );
            let current_total = drag.map_or_else(
                || progress.and_then(|p| p.current_bytes_total),
                |d| d.current_bytes_total,
            );
            let overall_done =
                drag.map_or_else(|| progress.map(|p| p.bytes_done), |d| d.bytes_processed);
            let overall_total =
                drag.map_or_else(|| progress.and_then(|p| p.bytes_total), |d| d.bytes_total);
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
                    current_done,
                    current_total,
                    active,
                    cx,
                ))
                .child(progress_group(
                    [
                        "archive-overall-progress",
                        "archive-job-progress",
                        "archive-overall-amount",
                    ],
                    if drag.is_some() {
                        "总体解压"
                    } else {
                        "总体进度"
                    },
                    overall_done,
                    overall_total,
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
            body = body.child(readable(
                "archive-drag-processed",
                format!("已解压文件 {} / {}", drag.files_done, drag.files_total),
            ));
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
                        "空间回收解压已完成。原压缩包保留为已消耗的稀疏外壳，不是完整可用压缩包；请保留恢复记录和已解压输出。本版本不能恢复完整原压缩包。",
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
            if !result.cleanup_complete {
                body = body.child(status_alert(
                    "archive-job-cleanup-pending",
                    "清理尚未确认；请勿立即重新运行或移动相关文件。",
                    false,
                ));
            }
            // Only a failed or rejected result has a cause to inspect. A completed or user-cancelled
            // job always carries commit_state "Committed", which is not a diagnosis.
            if matches!(
                result.outcome,
                ArchiveJobOutcome::Failed | ArchiveJobOutcome::Rejected
            ) && (result.commit_state.is_some()
                || result.native_error_code.is_some()
                || result.code.is_some())
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
            .when(self.drag.is_none() && self.batch.is_some(), |footer|footer.child(Button::new("archive-job-batch").label(if self.summary {"任务详情"}else{"批次汇总"}).outline().with_size(Size::Medium).on_click(cx.listener(|this,_,window,cx|{if this.summary {this.show_detail(window,cx);}else{this.show_summary(cx);} }))))
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
            .when(self.drag.is_none() && self.terminal.as_ref().is_some_and(|result| result.cleanup_complete && matches!(result.outcome, ArchiveJobOutcome::Succeeded | ArchiveJobOutcome::SucceededWithWarnings) && result.result_location.is_some()),|footer|footer.child(
                Button::new("archive-job-locate").label("定位结果").outline().on_click(cx.listener(|this,_,_,cx|{
                    let owner=this.owner.clone();let id=this.id;cx.defer(move|cx|{let _=owner.update(cx,|app,cx|app.locate_job_result(id,cx));});
                }))
            ))
            .when(self.drag.is_none() && self.terminal.is_some() && (self.retry || self.retry_pending), |footer| footer.child(
                Button::new("archive-job-retry").label(if self.retry_pending { "正在重验" } else { "重试" }).outline().with_size(Size::Medium).disabled(self.retry_pending)
                    .on_click(cx.listener(|this, _, _, cx| { let owner = this.owner.clone(); let id = this.id; this.retry_pending = true; cx.notify(); cx.defer(move |cx| { let _ = owner.update(cx, |app, cx| app.retry_job(id, cx)); }); }))
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
                    .disabled(self.drag.is_some() || !self.controls.pause || terminal || self.prompt.is_some() || self.pause_requested.is_some() || self.cancel_requested)
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
                    .disabled(self.drag.is_some() || !self.controls.stop || terminal || self.cancel_requested)
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
            .role(gpui_kit::Role::Group)
            .aria_label(self.title())
            .test_support()
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
                self.title(),
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
