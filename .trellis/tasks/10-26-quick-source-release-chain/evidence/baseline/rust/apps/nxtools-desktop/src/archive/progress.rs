//! Official progress/prompt presentation; no execution, notification clock or native owner.
use super::ArchiveApplication;
use gpui_kit::component::{
    ActiveTheme, Disableable, WindowExt,
    alert::Alert,
    button::Button,
    checkbox::Checkbox,
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
    pub state: String,
    pub progress: Option<ArchiveJobProgress>,
    pub terminal: Option<ArchiveJobTerminal>,
    pub prompt: Option<ArchiveJobPrompt>,
    pub paused: bool,
}
pub(super) struct JobProgressView {
    id: Uuid,
    owner: WeakEntity<ArchiveApplication>,
    focus: FocusHandle,
    state: String,
    progress: Option<ArchiveJobProgress>,
    terminal: Option<ArchiveJobTerminal>,
    prompt: Option<ArchiveJobPrompt>,
    shown_prompt: Option<ArchiveJobTicket>,
    paused: bool,
    keep_open: bool,
    prompt_inputs: Vec<PromptButtonInput>,
    activation: Option<gpui_kit::Subscription>,
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
            progress: None,
            terminal: None,
            prompt: None,
            shown_prompt: None,
            paused: false,
            keep_open,
            prompt_inputs: Vec::new(),
            activation: None,
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
        let ProgressSnapshot {
            state,
            progress,
            terminal,
            prompt,
            paused,
        } = snapshot;
        self.state = state;
        self.progress = progress;
        self.terminal = terminal;
        self.prompt = prompt;
        self.paused = paused;
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
        {
            let owner = self.owner.clone();
            let id = self.id;
            cx.defer(move |cx| {
                let _ = owner.update(cx, |app, cx| app.background_job(id, cx));
            });
        }
        cx.notify();
    }
    fn command(&self, action: ArchiveJobAction, cx: &mut Context<Self>) {
        let owner = self.owner.clone();
        let id = self.id;
        cx.defer(move |cx| {
            let _ = owner.update(cx, |app, cx| app.control_job(id, action, cx));
        });
    }
    fn background(&self, _window: &mut Window, cx: &mut Context<Self>) {
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
                ArchiveJobPromptKind::Conflict=>div().flex().flex_col().gap_2().child(format!("目标已有同名文件。现有大小：{}；传入大小：{}。",number(prompt.existing_bytes),number(prompt.incoming_bytes))).child(prompt_objects(&prompt)).into_any_element(),
                ArchiveJobPromptKind::Risk=>div().child("当前成员具有高膨胀率；继续仍遵守原先有限预算，不增加预算。取消将停止当前任务。").into_any_element(),
                ArchiveJobPromptKind::OutputBudget=>div().child(format!("前次尝试已确认零提交并完成清理。需要有限预算：单文件 {}，总输出 {}。继续只在新准入中增加这两个输出上限。",number(prompt.existing_bytes),number(prompt.incoming_bytes))).into_any_element(),
                ArchiveJobPromptKind::CreateOverwrite=>div().flex().flex_col().gap_2().child("目标压缩包已存在。覆盖将保留原生确认的目标身份并在完整验证后一次发布；目标变化会拒绝覆盖。").child(prompt_objects(&prompt)).into_any_element(),
                ArchiveJobPromptKind::Destination=>div().child(if prompt.error_code.as_deref()==Some("create_destination"){format!("目标目录尚不存在。是否在当前已验证的目标位置创建目录并继续？预计输出：{}；当前可用：{}。继续不会增加任务预算。",number(prompt.incoming_bytes),number(prompt.existing_bytes))}else{format!("目标卷可用空间少于预计输出。预计输出：{}；当前可用：{}。是否仍然继续？实际写入失败将停止任务，继续不会增加任务预算。",number(prompt.incoming_bytes),number(prompt.existing_bytes))}).into_any_element(),
            };
            let mut footer=DialogFooter::new().child(DialogClose::new().trigger(|b|cancel_input_guard.guard(b.label("取消任务"))));
            if prompt.kind==ArchiveJobPromptKind::Conflict{
                for (index,(label,decision)) in [("覆盖",ArchiveJobConflictDecision::Overwrite),("跳过",ArchiveJobConflictDecision::Skip),("自动重命名",ArchiveJobConflictDecision::AutoRename)].into_iter().enumerate(){
                    let response_owner=owner.clone();let response_prompt=prompt.clone();let apply=apply.clone();
                    footer=footer.child(conflict_inputs[index].guard(Button::new(gpui_kit::SharedString::from(format!("archive-conflict-{}-{label}",prompt.ticket.prompt_id))).label(label).outline()).on_click(move|_,window,cx|{let owner=response_owner.clone();let prompt=response_prompt.clone();let reply=ArchiveJobReply{ticket:prompt.ticket.clone(),kind:prompt.kind,password:None,decision:Some(decision),apply_to_all:apply.get(),approved_limit:None};window.close_dialog(cx);cx.defer(move|cx|{let _=owner.update(cx,|app,cx|app.reply_job(id,prompt,reply,cx));});}));
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
fn prompt_objects(prompt: &ArchiveJobPrompt) -> gpui_kit::AnyElement {
    const MISSING: &str = "本次提示未提供对象标识";
    let display = prompt.display.as_ref();
    let mut objects = DescriptionList::vertical().columns(1).bordered(false);
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
                "被覆盖目标"
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
    value.map_or_else(|| "未知".into(), |n| format!("{n} 字节"))
}
impl Focusable for JobProgressView {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}
impl Render for JobProgressView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let terminal = self.terminal.is_some();
        let known = self.progress.as_ref().and_then(|p| {
            p.bytes_total
                .filter(|n| *n > 0)
                .map(|total| (p.bytes_done as f64 * 100.0 / total as f64) as f32)
        });
        let mut body = div()
            .id("archive-progress-body")
            .debug_selector(|| "archive-progress-body".into())
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .p_4()
            .flex()
            .flex_col()
            .gap_3()
            .child(self.state.clone())
            .child(
                Progress::new("archive-job-progress")
                    .accessibility_label("归档任务进度")
                    .loading(known.is_none() && !terminal)
                    .value(known.unwrap_or(0.0)),
            );
        if let Some(progress) = &self.progress {
            body = body.child(format!(
                "已处理 {} 字节 / {}；文件 {} / {}",
                progress.bytes_done,
                number(progress.bytes_total),
                progress.files_done,
                progress
                    .files_total
                    .map_or_else(|| "未知".into(), |n| n.to_string())
            ));
        }
        if let Some(result) = &self.terminal {
            body = body.child(format!(
                "实际提交 {} 个文件 / {} 字节；跳过 {} 个。耗时 {} ms",
                result.files_committed,
                result.bytes_committed,
                result.files_skipped,
                result.elapsed_ms
            ));
            if let Some(code) = &result.code {
                body = body.child(Alert::warning("archive-job-terminal-code", code.clone()));
            }
            if !result.cleanup_complete {
                body = body.child(Alert::warning(
                    "archive-job-cleanup-pending",
                    "原生清理未确认；执行槽仍保留，不会启动替代 worker。",
                ));
            }
        }
        let footer = div()
            .debug_selector(|| "archive-progress-footer".into())
            .flex_none()
            .p_3()
            .flex()
            .items_center()
            .gap_2()
            .child(
                Checkbox::new("archive-job-keep-open")
                    .label("完成后保留窗口")
                    .checked(self.keep_open)
                    .on_click(cx.listener(|this, value, _, cx| {
                        this.keep_open = *value;
                        let owner = this.owner.clone();
                        let id = this.id;
                        let value = *value;
                        cx.defer(move |cx| {
                            let _ = owner.update(cx, |app, _| {
                                app.keep_job_open(id, value);
                            });
                        });
                        cx.notify();
                    })),
            )
            .child(div().flex_1())
            .child(
                Button::new("archive-job-pause")
                    .label(if self.paused { "继续" } else { "暂停" })
                    .outline()
                    .disabled(terminal || self.prompt.is_some())
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
                    .disabled(terminal)
                    .on_click(
                        cx.listener(|this, _, _, cx| this.command(ArchiveJobAction::Cancel, cx)),
                    ),
            )
            .child(
                Button::new("archive-job-background")
                    .label(if terminal { "关闭" } else { "后台" })
                    .outline()
                    .on_click(cx.listener(|this, _, window, cx| this.background(window, cx))),
            );
        div()
            .id("archive-progress-window")
            .size_full()
            .flex()
            .flex_col()
            .bg(theme.colors.background)
            .text_color(theme.colors.foreground)
            .font_family(crate::ui::typography::UI_FONT_FAMILY)
            .child(body)
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
