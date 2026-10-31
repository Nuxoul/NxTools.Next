//! Archive operation decision surfaces: current Kit 0.8.0 Form/Field and stable Input/Select.
//! Create 680 / extract 560 starting widths belong to the parent window. One scroll body,
//! fixed Medium (32px) primary/outline footer; no Toolbar.child variant coercion or extra Root.
//! Native picker, admission, settings persistence and jobs remain ArchiveApplication's owners.
use super::{
    ArchiveApplication,
    draft::{self, CreateDraft, DraftSelection, DraftSource, ExtractDraft, ModifyDraft},
};
use gpui_kit::component::{
    ActiveTheme, Disableable, IndexPath, Sizable, Size, TitleBar, WindowExt,
    alert::Alert,
    button::{Button, ButtonVariants},
    checkbox::Checkbox,
    dialog::{DialogAction, DialogClose, DialogFooter},
    form::{Field, Form},
    input::{Input, InputEvent, InputGroup, InputState, Textarea, TextareaState},
    radio::{Radio, RadioGroup},
    select::{Select, SelectEvent, SelectState},
};
use gpui_kit::{
    App, AppContext, Context, Entity, EntityInputHandler, FocusHandle, Focusable, IntoElement,
    KeyDownEvent, ParentElement, Render, Subscription, WeakEntity, Window, div, prelude::*, px,
};
use nxtools_core::archive::{
    ArchiveCompressionLevel, ArchiveConflictPolicy, ArchiveCreateFormat, ArchiveDestination,
    ArchiveSettings,
};
use nxtools_protocol::{
    archive_job::*,
    archive_picker::{ArchivePickerPurpose, ArchivePickerResult},
};
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
use uuid::Uuid;

const FORMATS: &[&str] = &["ZIP", "7Z", "TAR", "TGZ", "TXZ", "GZ", "XZ"];
const LEVELS: &[&str] = &["仅存储", "快速", "标准", "最大"];
const STRATEGIES: &[&str] = &["自定义目标", "智能解压", "当前文件夹", "压缩包名称文件夹"];
const CONFLICTS: &[&str] = &["询问", "覆盖", "跳过", "自动重命名"];
type Choice = Entity<SelectState<Vec<&'static str>>>;

#[derive(Default)]
pub(super) struct FormCompletion {
    pub(super) open_after: bool,
    pub(super) keep_open: bool,
    pub(super) remember: Option<ArchiveSettings>,
    /// Parent admits this batch atomically, or rejects the entire form without starting any job.
    pub(super) additional_requests: Vec<ArchiveJobRequest>,
}
enum Draft {
    Create(CreateDraft),
    Extract(ExtractDraft),
    Modify(ModifyDraft),
}
struct PendingPicker {
    operation: Option<Uuid>,
    purpose: ArchivePickerPurpose,
}
#[derive(Clone, Copy)]
enum Flag {
    Solid,
    Header,
    Strip,
    Test,
    Separately,
    SingleFile,
    Motw,
    Open,
    Keep,
    Remember,
}

pub(crate) struct OperationForm {
    pub(super) id: Uuid,
    pub(super) generation: u64,
    owner: WeakEntity<ArchiveApplication>,
    settings: ArchiveSettings,
    draft: Draft,
    destination: Entity<InputState>,
    name: Entity<InputState>,
    password: Entity<InputState>,
    confirm: Entity<InputState>,
    exclusions: Entity<TextareaState>,
    create_exclusions: Vec<Entity<TextareaState>>,
    create_rule_subscriptions: Vec<Subscription>,
    volume: Entity<InputState>,
    format: Choice,
    level: Choice,
    method: Choice,
    strategy: Choice,
    conflict: Choice,
    pending_format: Option<ArchiveCreateFormat>,
    pending_picker: Option<PendingPicker>,
    enumeration_epoch: u64,
    enumeration_cancel: Arc<AtomicBool>,
    enumerating: bool,
    error: Option<String>,
    validation: Option<String>,
    submitted: bool,
    submit_epoch: u64,
    default_enter_held: bool,
    pub(in crate::archive) retired: bool,
    open_after: bool,
    keep_open: bool,
    remember: bool,
    focus: FocusHandle,
    _subscriptions: Vec<Subscription>,
    #[cfg(test)]
    submit_count: usize,
}
#[cfg(test)]
impl OperationForm {
    pub(in crate::archive) fn confirmation_strategy_for_test(&self) -> Option<ArchiveDestination> {
        match &self.draft {
            Draft::Extract(draft) => Some(draft.strategy),
            _ => None,
        }
    }
}
impl OperationForm {
    pub(super) fn create(
        settings: ArchiveSettings,
        sources: Vec<DraftSource>,
        owner: WeakEntity<ArchiveApplication>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let draft = CreateDraft::new(&settings, sources);
        Self::new(settings, Draft::Create(draft), owner, window, cx)
    }
    pub(super) fn extract(
        settings: ArchiveSettings,
        source: PathBuf,
        selection: Option<DraftSelection>,
        owner: WeakEntity<ArchiveApplication>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let draft = ExtractDraft::new(&settings, source, selection);
        Self::new(settings, Draft::Extract(draft), owner, window, cx)
    }
    pub(super) fn modify(
        settings: ArchiveSettings,
        draft: ModifyDraft,
        owner: WeakEntity<ArchiveApplication>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        Self::new(settings, Draft::Modify(draft), owner, window, cx)
    }
    fn new(
        settings: ArchiveSettings,
        draft: Draft,
        owner: WeakEntity<ArchiveApplication>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let (destination_value, name_value, exclusions_value, volume_value) = match &draft {
            Draft::Create(d) => (
                d.destination.clone(),
                d.name.clone(),
                String::new(),
                d.volume_mib.clone(),
            ),
            Draft::Extract(d) => (
                d.destination.clone(),
                String::new(),
                String::new(),
                String::new(),
            ),
            Draft::Modify(d) => (
                String::new(),
                d.value.clone().unwrap_or_default(),
                d.value.clone().unwrap_or_default(),
                String::new(),
            ),
        };
        let destination = input(&destination_value, false, window, cx);
        let name = input(&name_value, false, window, cx);
        let password = input("", true, window, cx);
        let confirm = input("", true, window, cx);
        let volume = input(&volume_value, false, window, cx);
        let exclusions = cx.new(|cx| {
            let mut state = TextareaState::new(window, cx);
            state.set_value(exclusions_value, window, cx);
            state
        });
        let create_exclusions: Vec<_> = match &draft {
            Draft::Create(draft) => draft
                .exclusions
                .iter()
                .map(|pattern| {
                    cx.new(|cx| {
                        let mut state = TextareaState::new(window, cx);
                        state.set_value(pattern.clone(), window, cx);
                        state
                    })
                })
                .collect(),
            _ => Vec::new(),
        };
        let (open_after, keep_open) = match &draft {
            Draft::Extract(draft) => (draft.open_after, draft.keep_open),
            Draft::Create(_) => (true, settings.keep_create_progress_open),
            Draft::Modify(_) => (false, false),
        };
        let (initial_format, initial_level) = match &draft {
            Draft::Create(draft) => (draft.format, draft.level),
            Draft::Extract(_) | Draft::Modify(_) => {
                (settings.create_format, settings.compression_level)
            }
        };
        let format = choice(FORMATS, format_index(initial_format), window, cx);
        let supported_levels = if initial_format == ArchiveCreateFormat::Zip {
            LEVELS
        } else {
            &LEVELS[1..]
        };
        let initial_level_label = LEVELS[level_index(initial_level)];
        let level = choice(
            supported_levels,
            supported_levels
                .iter()
                .position(|label| *label == initial_level_label)
                .unwrap_or(1),
            window,
            cx,
        );
        let method = choice(
            &["Deflate", "Store"],
            usize::from(initial_level == ArchiveCompressionLevel::Store),
            window,
            cx,
        );
        let strategy = choice(
            STRATEGIES,
            strategy_index(settings.default_destination),
            window,
            cx,
        );
        let conflict = choice(
            CONFLICTS,
            conflict_index(settings.conflict_policy),
            window,
            cx,
        );
        let mut subscriptions = Vec::new();
        for state in [&destination, &name, &password, &confirm, &volume] {
            subscriptions.push(cx.subscribe_in(
                state,
                window,
                |_, _, event: &InputEvent, window, cx| {
                    if matches!(event, InputEvent::Change) {
                        cx.defer_in(window, |this, _, cx| this.revalidate(cx));
                    }
                },
            ));
        }
        subscriptions.push(cx.subscribe_in(
            &exclusions,
            window,
            |_, _, event: &InputEvent, window, cx| {
                if matches!(event, InputEvent::Change) {
                    cx.defer_in(window, |this, _, cx| this.revalidate(cx));
                }
            },
        ));
        let mut create_rule_subscriptions = Vec::new();
        for rule in &create_exclusions {
            create_rule_subscriptions.push(cx.subscribe_in(
                rule,
                window,
                |_, _, event: &InputEvent, window, cx| {
                    if matches!(event, InputEvent::Change) {
                        cx.defer_in(window, |this, _, cx| this.revalidate(cx));
                    }
                },
            ));
        }
        subscriptions.push(cx.subscribe_in(
            &format,
            window,
            |_, _, event: &SelectEvent<Vec<&'static str>>, window, cx| {
                let SelectEvent::Confirm(Some(value)) = event else {
                    return;
                };
                let value = *value;
                cx.defer_in(window, move |this, window, cx| {
                    if let Some(index) = FORMATS.iter().position(|label| *label == value) {
                        this.change_format(format_at(index), window, cx);
                    }
                });
            },
        ));
        subscriptions.push(cx.subscribe_in(
            &level,
            window,
            |_, _, event: &SelectEvent<Vec<&'static str>>, window, cx| {
                let SelectEvent::Confirm(Some(value)) = event else {
                    return;
                };
                let value = *value;
                cx.defer_in(window, move |this, window, cx| {
                    if this.busy() {
                        return;
                    }
                    if let Draft::Create(draft) = &mut this.draft {
                        draft.level =
                            level_at(LEVELS.iter().position(|label| *label == value).unwrap_or(2));
                        let method = if draft.level == ArchiveCompressionLevel::Store {
                            "Store"
                        } else {
                            "Deflate"
                        };
                        this.method.update(cx, |state, cx| {
                            state.set_selected_value(&method, window, cx)
                        });
                    }
                    this.revalidate(cx);
                });
            },
        ));
        subscriptions.push(cx.subscribe_in(
            &method,
            window,
            |_, _, event: &SelectEvent<Vec<&'static str>>, window, cx| {
                let SelectEvent::Confirm(Some(value)) = event else {
                    return;
                };
                let value = *value;
                cx.defer_in(window, move |this, window, cx| {
                    if this.busy() {
                        return;
                    }
                    if let Draft::Create(draft) = &mut this.draft {
                        draft.level = if value == "Store" {
                            ArchiveCompressionLevel::Store
                        } else if draft.level == ArchiveCompressionLevel::Store {
                            ArchiveCompressionLevel::Normal
                        } else {
                            draft.level
                        };
                        let label = LEVELS[level_index(draft.level)];
                        this.level
                            .update(cx, |state, cx| state.set_selected_value(&label, window, cx));
                    }
                    this.revalidate(cx);
                });
            },
        ));
        subscriptions.push(cx.subscribe_in(
            &strategy,
            window,
            |_, _, event: &SelectEvent<Vec<&'static str>>, window, cx| {
                let SelectEvent::Confirm(Some(value)) = event else {
                    return;
                };
                let value = *value;
                cx.defer_in(window, move |this, window, cx| {
                    if this.busy() {
                        return;
                    }
                    if let Draft::Extract(draft) = &mut this.draft {
                        draft.strategy = strategy_at(
                            STRATEGIES
                                .iter()
                                .position(|label| *label == value)
                                .unwrap_or(0),
                        );
                        if draft.strategy == ArchiveDestination::Here {
                            let parent = draft
                                .source
                                .parent()
                                .and_then(|p| p.to_str())
                                .unwrap_or("")
                                .to_owned();
                            this.destination
                                .update(cx, |state, cx| state.set_value(parent, window, cx));
                        }
                    }
                    this.revalidate(cx);
                });
            },
        ));
        subscriptions.push(cx.subscribe_in(
            &conflict,
            window,
            |_, _, event: &SelectEvent<Vec<&'static str>>, window, cx| {
                let SelectEvent::Confirm(Some(value)) = event else {
                    return;
                };
                let value = *value;
                cx.defer_in(window, move |this, _, cx| {
                    if this.busy() {
                        return;
                    }
                    if let Draft::Extract(draft) = &mut this.draft {
                        draft.conflict = conflict_at(
                            CONFLICTS
                                .iter()
                                .position(|label| *label == value)
                                .unwrap_or(0),
                        );
                    }
                    this.revalidate(cx);
                });
            },
        ));
        subscriptions.push(cx.on_release(|this, _| {
            this.enumeration_cancel.store(true, Ordering::Release);
            if let Draft::Create(draft) = &mut this.draft {
                draft.clear_secrets();
            }
        }));
        subscriptions.push(cx.observe_window_activation(window, |this, _, _| {
            this.default_enter_held = false;
        }));
        let focus = cx.focus_handle();
        focus.focus(window, cx);
        let mut form = Self {
            id: Uuid::new_v4(),
            generation: 1,
            owner,
            settings,
            draft,
            destination,
            name,
            password,
            confirm,
            volume,
            exclusions,
            create_exclusions,
            create_rule_subscriptions,
            format,
            level,
            method,
            strategy,
            conflict,
            pending_format: None,
            pending_picker: None,
            enumeration_epoch: 0,
            enumeration_cancel: Arc::new(AtomicBool::new(false)),
            enumerating: false,
            error: None,
            validation: None,
            submitted: false,
            submit_epoch: 0,
            default_enter_held: false,
            retired: false,
            open_after,
            keep_open,
            remember: false,
            focus,
            _subscriptions: subscriptions,
            #[cfg(test)]
            submit_count: 0,
        };
        form.revalidate(cx);
        form
    }
    fn default_enter(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if event.keystroke.key != "enter" {
            return;
        }
        if event.is_held || std::mem::replace(&mut self.default_enter_held, true) {
            return;
        }
        if event.keystroke.modifiers.modified()
            || event.prefer_character_input
            || self.busy()
            || window.has_active_dialog(cx)
        {
            return;
        }
        let Some(input) = [&self.name, &self.destination, &self.volume]
            .into_iter()
            .find(|input| input.focus_handle(cx).is_focused(window))
            .cloned()
        else {
            return;
        };
        if input.update(cx, |input, cx| {
            input.marked_text_range(window, cx).is_some()
        }) {
            return;
        }
        cx.stop_propagation();
        self.submit(window, cx);
    }
    fn busy(&self) -> bool {
        self.retired
            || self.submitted
            || self.enumerating
            || self.pending_picker.is_some()
            || self.pending_format.is_some()
    }
    fn sync_inputs(&mut self, cx: &App) {
        match &mut self.draft {
            Draft::Create(draft) => {
                draft.name = self.name.read(cx).value().to_string();
                draft.destination = self.destination.read(cx).value().to_string();
                draft.password = self.password.read(cx).value().to_string();
                draft.confirm = self.confirm.read(cx).value().to_string();
                draft.volume_mib = self.volume.read(cx).value().to_string();
                draft.exclusions = self
                    .create_exclusions
                    .iter()
                    .map(|rule| rule.read(cx).value().to_string())
                    .collect();
            }
            Draft::Extract(draft) => {
                draft.destination = self.destination.read(cx).value().to_string();
                draft.open_after = self.open_after;
                draft.keep_open = self.keep_open;
                draft.remember = self.remember;
            }
            Draft::Modify(draft) => {
                if matches!(
                    draft.action,
                    ArchiveJobModifyAction::Rename | ArchiveJobModifyAction::NewDirectory
                ) {
                    draft.value = Some(self.name.read(cx).value().to_string());
                } else if draft.action == ArchiveJobModifyAction::Comment {
                    draft.value = Some(self.exclusions.read(cx).value().to_string());
                }
            }
        }
    }
    fn capture(&mut self, cx: &App) -> Result<(Vec<ArchiveJobRequest>, FormCompletion), String> {
        self.sync_inputs(cx);
        let mut remembered = self.settings.clone();
        let result = match &mut self.draft {
            Draft::Create(draft) => {
                let requests = draft.requests(&self.settings);
                if requests.is_ok() {
                    draft.clear_secrets();
                }
                remembered.create_format = draft.format;
                remembered.compression_level = draft.level;
                remembered.strip_single_root = draft.strip_single_root;
                remembered.exclude_patterns = draft.exclusions.clone();
                remembered.test_after_create = draft.test_after_create;
                remembered.keep_create_progress_open = self.keep_open;
                requests
            }
            Draft::Extract(draft) => {
                remembered.default_destination = draft.strategy;
                remembered.conflict_policy = draft.conflict;
                remembered.copy_motw = draft.copy_motw;
                remembered.open_after_extract = draft.open_after;
                remembered.keep_extract_progress_open = draft.keep_open;
                remembered.smart_single_file_folder = draft.single_file_folder;
                draft.request(&self.settings).map(|request| vec![request])
            }
            Draft::Modify(draft) => draft.request().map(|request| vec![request]),
        }?;
        if self.remember {
            remembered.validate()?;
        }
        Ok((
            result,
            FormCompletion {
                open_after: self.open_after,
                keep_open: self.keep_open,
                remember: (!matches!(self.draft, Draft::Modify(_)) && self.remember)
                    .then_some(remembered),
                additional_requests: Vec::new(),
            },
        ))
    }
    fn revalidate(&mut self, cx: &mut Context<Self>) {
        if self.retired {
            return;
        }
        if self.submitted {
            return;
        }
        self.validation = self.capture(cx).err();
        self.error = None;
        cx.notify();
    }
    fn submit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy() {
            return;
        }
        let (mut requests, mut completion) = match self.capture(cx) {
            Ok(captured) => captured,
            Err(message) => {
                self.validation = Some(message.clone());
                self.error = Some(message);
                cx.notify();
                return;
            }
        };
        let request = requests.remove(0);
        completion.additional_requests = requests;
        self.submitted = true;
        self.error = None;
        self.submit_epoch = self.submit_epoch.wrapping_add(1);
        let submit_epoch = self.submit_epoch;
        let source = window.window_handle();
        #[cfg(test)]
        {
            self.submit_count += 1;
        }
        let owner = self.owner.clone();
        let view = cx.entity().downgrade();
        let id = self.id;
        let generation = self.generation;
        tracing::info!(event="archive.form.submitted",correlation_id=%id,generation,"archive form submitted for admission");
        cx.defer(move |cx| {
            if !view
                .read_with(cx, |form, _| {
                    form.id == id
                        && form.generation == generation
                        && form.submit_epoch == submit_epoch
                        && form.submitted
                        && !form.retired
                })
                .unwrap_or(false)
            {
                return;
            }
            if owner
                .update(cx, |app, cx| {
                    app.submit_form(id, generation, request, completion, cx)
                })
                .is_err()
            {
                let _ = source.update(cx, |_, window, cx| {
                    let _ = view.update(cx, |form, cx| {
                        form.fail_submit(
                            "归档操作所有者已关闭，请重新打开表单。".into(),
                            window,
                            cx,
                        )
                    });
                });
            }
        });
        cx.notify();
    }
    pub(super) fn fail_submit(
        &mut self,
        message: String,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.retired {
            return;
        }
        self.submit_epoch = self.submit_epoch.wrapping_add(1);
        self.submitted = false;
        self.error = Some(message);
        cx.notify();
    }
    pub(super) fn block_uncertain_retry(&mut self, message: String, cx: &mut Context<Self>) {
        if self.retired {
            return;
        }
        self.submit_epoch = self.submit_epoch.wrapping_add(1);
        self.submitted = true;
        self.error = Some(message);
        cx.notify();
    }
    pub(super) fn retire(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.retired {
            return;
        }
        self.retired = true;
        self.generation = self.generation.wrapping_add(1).max(1);
        self.pending_picker = None;
        self.pending_format = None;
        self.enumeration_cancel.store(true, Ordering::Release);
        tracing::info!(event="archive.form.retired",correlation_id=%self.id,generation=self.generation,"archive draft retired without source mutation");
        if let Draft::Create(draft) = &mut self.draft {
            draft.clear_secrets();
        }
        for input in [&self.password, &self.confirm] {
            input.update(cx, |state, cx| state.set_value("", window, cx));
        }
        cx.notify();
    }
    pub(super) fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.retire(window, cx);
        window.remove_window();
    }
    fn change_format(
        &mut self,
        format: ArchiveCreateFormat,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.busy() {
            return;
        }
        let Draft::Create(draft) = &self.draft else {
            return;
        };
        if draft.format == format {
            return;
        }
        if !draft::encrypted(format)
            && (!self.password.read(cx).value().is_empty()
                || !self.confirm.read(cx).value().is_empty())
        {
            self.pending_format = Some(format);
            let weak = cx.entity().downgrade();
            let cancel = weak.clone();
            let close = weak.clone();
            window.open_dialog(cx, move |dialog, _, _| {
                let weak = weak.clone();
                let cancel = cancel.clone();
                let close = close.clone();
                dialog
                    .title("此格式不支持加密")
                    .child("切换格式将清除密码和确认密码。是否继续？取消会保留当前格式和密码。")
                    .footer(
                        DialogFooter::new()
                            .child(DialogClose::new().trigger(|button| {
                                button.label("保留密码").outline().with_size(Size::Medium)
                            }))
                            .child(
                                DialogAction::new().child(
                                    Button::new("archive-discard-secret")
                                        .label("清除密码并切换")
                                        .primary()
                                        .with_size(Size::Medium)
                                        .debug_selector(|| "archive-discard-secret".into()),
                                ),
                            ),
                    )
                    .on_ok(move |_, window, cx| {
                        let weak = weak.clone();
                        window.defer(cx, move |window, cx| {
                            let _ =
                                weak.update(cx, |form, cx| form.resolve_format(true, window, cx));
                        });
                        true
                    })
                    .on_cancel(move |_, window, cx| {
                        let cancel = cancel.clone();
                        window.defer(cx, move |window, cx| {
                            let _ = cancel
                                .update(cx, |form, cx| form.resolve_format(false, window, cx));
                        });
                        true
                    })
                    .on_close(move |_, window, cx| {
                        let close = close.clone();
                        window.defer(cx, move |window, cx| {
                            let _ =
                                close.update(cx, |form, cx| form.resolve_format(false, window, cx));
                        });
                    })
            });
            cx.notify();
        } else {
            self.apply_format(format, window, cx);
        }
    }
    fn resolve_format(&mut self, accepted: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(format) = self.pending_format.take() else {
            return;
        };
        if self.retired {
            return;
        }
        if accepted {
            for input in [&self.password, &self.confirm] {
                input.update(cx, |state, cx| state.set_value("", window, cx));
            }
            self.apply_format(format, window, cx);
        } else if let Draft::Create(draft) = &self.draft {
            let value = FORMATS[format_index(draft.format)];
            self.format
                .update(cx, |state, cx| state.set_selected_value(&value, window, cx));
            self.revalidate(cx);
        }
    }
    fn apply_format(
        &mut self,
        format: ArchiveCreateFormat,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Draft::Create(draft) = &mut self.draft else {
            return;
        };
        draft.name = self.name.read(cx).value().to_string();
        draft.set_format(format);
        self.name.update(cx, |state, cx| {
            state.set_value(draft.name.clone(), window, cx)
        });
        let value = FORMATS[format_index(format)];
        self.format
            .update(cx, |state, cx| state.set_selected_value(&value, window, cx));
        let levels = if format == ArchiveCreateFormat::Zip {
            LEVELS.to_vec()
        } else {
            LEVELS[1..].to_vec()
        };
        let value = LEVELS[level_index(draft.level)];
        self.level.update(cx, |state, cx| {
            state.set_items(levels, window, cx);
            state.set_selected_value(&value, window, cx);
        });
        let method = if draft.level == ArchiveCompressionLevel::Store {
            "Store"
        } else {
            "Deflate"
        };
        self.method.update(cx, |state, cx| {
            state.set_selected_value(&method, window, cx)
        });
        self.revalidate(cx);
    }
    fn pick(&mut self, purpose: ArchivePickerPurpose, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy() {
            return;
        }
        self.pending_picker = Some(PendingPicker {
            operation: None,
            purpose,
        });
        self.error = None;
        let owner = self.owner.clone();
        let weak = cx.entity().downgrade();
        let id = self.id;
        let generation = self.generation;
        let source = window.window_handle();
        let directory = self.destination.read(cx).value().to_string();
        let directory = (!directory.is_empty()).then_some(directory);
        let suggested =
            (purpose == ArchivePickerPurpose::Save).then(|| self.name.read(cx).value().to_string());
        cx.defer(move |cx| {
            if !weak
                .read_with(cx, |form, _| {
                    form.id == id
                        && form.generation == generation
                        && !form.retired
                        && form.pending_picker.is_some()
                })
                .unwrap_or(false)
            {
                return;
            }
            let operation = owner
                .update(cx, |app, cx| {
                    app.pick_for_form(id, generation, purpose, (directory, suggested), source, cx)
                })
                .ok()
                .flatten();
            let _ = weak.update(cx, |form, cx| {
                if form.generation != generation || form.retired {
                    return;
                }
                if let Some(pending) = &mut form.pending_picker {
                    pending.operation = operation;
                }
                if operation.is_none() {
                    form.pending_picker = None;
                    form.error = Some("文件选择器不可用或繁忙，请重试。".into());
                }
                cx.notify();
            });
        });
        cx.notify();
    }
    pub(super) fn picker_result(
        &mut self,
        result: ArchivePickerResult,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.retired
            || result.version != 1
            || result.session_id != self.id
            || result.generation != self.generation
            || self
                .pending_picker
                .as_ref()
                .is_none_or(|pending| pending.operation != Some(result.operation_id))
        {
            return;
        }
        let Some(pending) = self.pending_picker.take() else {
            return;
        };
        let purpose = pending.purpose;
        if result.cancelled {
            cx.notify();
            return;
        }
        if result.code.is_some() || result.native_error_code.is_some() {
            self.error = Some(format!(
                "选择失败：{}{}",
                result.code.unwrap_or_else(|| "NativeFailure".into()),
                result
                    .native_error_code
                    .map(|code| format!("（Windows {code}）"))
                    .unwrap_or_default()
            ));
            cx.notify();
            return;
        }
        if result.paths.is_empty()
            || result.paths.len() > 32
            || result.paths.iter().any(|p| !valid_job_path(p))
        {
            self.error = Some("选择结果无效或超过 32 个来源；未丢弃或添加任何输入。".into());
            cx.notify();
            return;
        }
        match purpose {
            ArchivePickerPurpose::Files | ArchivePickerPurpose::Folders => match &mut self.draft {
                Draft::Extract(draft) => {
                    if result.paths.len() != 1 {
                        self.error = Some("解压表单一次只接受一个归档；本次未更改来源。".into());
                    } else {
                        draft.source = PathBuf::from(&result.paths[0]);
                        draft.selection = None;
                        draft.selected_only = false;
                        self.revalidate(cx);
                    }
                }
                Draft::Create(_) => self.enumerate_sources(result.paths, cx),
                Draft::Modify(draft) => {
                    let mut unique = std::collections::HashSet::new();
                    if draft.action != ArchiveJobModifyAction::Add
                        || draft.added.len() + result.paths.len() > 31
                        || result.paths.iter().any(|p| {
                            !unique.insert(p.to_lowercase())
                                || p.eq_ignore_ascii_case(&draft.source.to_string_lossy())
                                || draft.added.iter().any(|old| old.eq_ignore_ascii_case(p))
                        })
                    {
                        self.error = Some("来源重复或超过 31 个；未添加任何输入。".into());
                    } else {
                        draft.added.extend(result.paths);
                        self.revalidate(cx);
                    }
                }
            },
            ArchivePickerPurpose::Destination => {
                if result.paths.len() != 1 {
                    self.error = Some("请选择一个目标文件夹。".into());
                } else {
                    self.destination.update(cx, |state, cx| {
                        state.set_value(result.paths[0].clone(), window, cx)
                    });
                    self.revalidate(cx);
                }
            }
            ArchivePickerPurpose::Save => {
                if result.paths.len() != 1 {
                    self.error = Some("请选择一个目标文件。".into());
                } else {
                    let path = PathBuf::from(&result.paths[0]);
                    let name = path
                        .file_name()
                        .and_then(|p| p.to_str())
                        .unwrap_or("")
                        .to_owned();
                    let parent = path
                        .parent()
                        .and_then(|p| p.to_str())
                        .unwrap_or("")
                        .to_owned();
                    self.name
                        .update(cx, |state, cx| state.set_value(name, window, cx));
                    self.destination
                        .update(cx, |state, cx| state.set_value(parent, window, cx));
                    self.revalidate(cx);
                }
            }
        }
        cx.notify();
    }
    fn enumerate_sources(&mut self, paths: Vec<String>, cx: &mut Context<Self>) {
        let Draft::Create(draft) = &self.draft else {
            return;
        };
        if draft.sources.len() + paths.len() > 32 {
            self.error =
                Some("最多选择 32 个来源；本次未添加任何来源，未静默截断。请分批选择。".into());
            return;
        }
        self.enumeration_cancel.store(true, Ordering::Release);
        self.enumeration_cancel = Arc::new(AtomicBool::new(false));
        let cancel = self.enumeration_cancel.clone();
        self.enumeration_epoch = self.enumeration_epoch.wrapping_add(1);
        let epoch = self.enumeration_epoch;
        let generation = self.generation;
        self.enumerating = true;
        let id = self.id;
        let started = std::time::Instant::now();
        tracing::info!(event="archive.form.sources.started",correlation_id=%id,generation,roots=paths.len(),"archive source metadata started");
        cx.spawn(async move |this,cx| {
            let prepared=cx.background_executor().spawn(async move { source_metadata(paths,&cancel) }).await;
            let _=this.update(cx, |form,cx| {
                if form.retired || form.generation!=generation || form.enumeration_epoch!=epoch {return;}
                form.enumerating=false;
                match prepared {
                    Ok(sources) => {
                        if let Draft::Create(draft)=&mut form.draft {
                            let duplicate=sources.iter().any(|source|draft.sources.iter().any(|old|old.path.to_string_lossy().eq_ignore_ascii_case(&source.path.to_string_lossy())));
                            if duplicate {form.error=Some("本次选择包含已有来源；未添加任何重复输入。".into());}
                            else {draft.sources.extend(sources);form.revalidate(cx);}
                        }
                        tracing::info!(event="archive.form.sources.finished",correlation_id=%id,generation,elapsed_ms=started.elapsed().as_millis() as u64,"archive source metadata completed");
                    }
                    Err(message) => {form.error=Some(message);tracing::warn!(event="archive.form.sources.failed",correlation_id=%id,generation,elapsed_ms=started.elapsed().as_millis() as u64,"archive source metadata failed");}
                }
                cx.notify();
            });
        }).detach();
    }
    fn flag(&mut self, flag: Flag, value: bool, cx: &mut Context<Self>) {
        if self.busy() {
            return;
        }
        match flag {
            Flag::Open => self.open_after = value,
            Flag::Keep => self.keep_open = value,
            Flag::Remember => self.remember = value,
            _ => match &mut self.draft {
                Draft::Create(draft) => match flag {
                    Flag::Solid => draft.solid = value,
                    Flag::Header => draft.encrypt_header = value,
                    Flag::Strip => draft.strip_single_root = value,
                    Flag::Test => draft.test_after_create = value,
                    Flag::Separately => draft.separately = value,
                    _ => {}
                },
                Draft::Extract(draft) => match flag {
                    Flag::SingleFile => draft.single_file_folder = value,
                    Flag::Motw => draft.copy_motw = value,
                    _ => {}
                },
                Draft::Modify(_) => {}
            },
        }
        self.revalidate(cx);
    }
    fn check(
        &self,
        id: &'static str,
        label: &'static str,
        value: bool,
        flag: Flag,
        disabled: bool,
        cx: &Context<Self>,
    ) -> Checkbox {
        Checkbox::new(id)
            .label(label)
            .checked(value)
            .disabled(disabled || self.busy())
            .on_change(cx.listener(move |this, value, _, cx| this.flag(flag, *value, cx)))
    }
    fn picker_button(
        &self,
        id: &'static str,
        label: &'static str,
        purpose: ArchivePickerPurpose,
        cx: &Context<Self>,
    ) -> Button {
        Button::new(id)
            .label(label)
            .outline()
            .with_size(Size::Medium)
            .disabled(self.busy())
            .on_click(cx.listener(move |this, _, window, cx| this.pick(purpose, window, cx)))
    }
    fn exclusion_fields(&self, cx: &Context<Self>) -> gpui_kit::Div {
        let mut rows = div().w_full().flex().flex_col().gap_2();
        for (index, rule) in self.create_exclusions.iter().enumerate() {
            rows = rows.child(
                div()
                    .flex()
                    .items_start()
                    .gap_2()
                    .child(
                        div().flex_1().min_w_0().child(
                            Textarea::new(rule)
                                .aria_label(format!("创建排除规则 {}", index + 1))
                                .h(px(48.))
                                .disabled(self.busy()),
                        ),
                    )
                    .child(
                        Button::new(("archive-create-rule-remove", index))
                            .debug_selector(move || format!("archive-create-rule-remove-{index}"))
                            .label("删除")
                            .outline()
                            .with_size(Size::Small)
                            .disabled(self.busy())
                            .on_click(cx.listener(move |this, _, _, cx| {
                                if !this.busy() && index < this.create_exclusions.len() {
                                    this.create_exclusions.remove(index);
                                    drop(this.create_rule_subscriptions.remove(index));
                                    this.revalidate(cx);
                                }
                            })),
                    ),
            );
        }
        rows.child(
            Button::new("archive-create-rule-add")
                .debug_selector(|| "archive-create-rule-add".into())
                .label("增加规则")
                .outline()
                .with_size(Size::Small)
                .disabled(self.busy() || self.create_exclusions.len() >= 32)
                .on_click(cx.listener(|this, _, window, cx| {
                    if this.busy() || this.create_exclusions.len() >= 32 {
                        return;
                    }
                    let rule = cx.new(|cx| TextareaState::new(window, cx));
                    this.create_rule_subscriptions.push(cx.subscribe_in(
                        &rule,
                        window,
                        |_, _, event: &InputEvent, window, cx| {
                            if matches!(event, InputEvent::Change) {
                                cx.defer_in(window, |this, _, cx| this.revalidate(cx));
                            }
                        },
                    ));
                    this.create_exclusions.push(rule);
                    this.revalidate(cx);
                })),
        )
    }
    fn create_fields(&self, cx: &Context<Self>) -> Form {
        let Draft::Create(draft) = &self.draft else {
            return Form::vertical();
        };
        let busy = self.busy();
        let stream = matches!(
            draft.format,
            ArchiveCreateFormat::Gz | ArchiveCreateFormat::Xz
        );
        let mut sources = div().flex().flex_col().gap_2();
        for (index, source) in draft.sources.iter().enumerate() {
            let text = format!(
                "{} · {}",
                source.path.display(),
                if source.directory {
                    "文件夹（大小未扫描）".into()
                } else {
                    source
                        .bytes
                        .map(|bytes| format!("{bytes} 字节"))
                        .unwrap_or_else(|| "大小未知".into())
                }
            );
            sources = sources.child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(div().flex_1().min_w_0().child(text))
                    .child(
                        Button::new(("archive-remove-source", index))
                            .label("移除")
                            .outline()
                            .disabled(busy)
                            .on_click(cx.listener(move |this, _, _, cx| {
                                if this.busy() {
                                    return;
                                }
                                if let Draft::Create(draft) = &mut this.draft
                                    && index < draft.sources.len()
                                {
                                    draft.sources.remove(index);
                                }
                                this.revalidate(cx);
                            })),
                    ),
            );
        }
        let known = draft
            .sources
            .iter()
            .filter_map(|source| source.bytes)
            .fold(0_u64, u64::saturating_add);
        let unknown = draft
            .sources
            .iter()
            .any(|source| source.bytes.is_none() || source.directory);
        let mut form = Form::vertical()
            .child(
                Field::new()
                    .label(format!(
                        "来源（{} / 32）· 已知 {known} 字节{}",
                        draft.sources.len(),
                        if unknown {
                            "；目录总量由工作进程统计"
                        } else {
                            ""
                        }
                    ))
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .flex_wrap()
                            .child(self.picker_button(
                                "archive-add-files",
                                "添加文件…",
                                ArchivePickerPurpose::Files,
                                cx,
                            ))
                            .child(self.picker_button(
                                "archive-add-folders",
                                "添加文件夹…",
                                ArchivePickerPurpose::Folders,
                                cx,
                            )),
                    )
                    .child(sources),
            )
            .child(
                Field::new()
                    .label("压缩包名称")
                    .required(true)
                    .description(if draft.separately {
                        "分别压缩时按每个来源名称生成；同名输出会明确拒绝。"
                    } else if !valid_job_name(self.name.read(cx).value().as_ref()) {
                        "名称无效：不能包含路径、保留设备名、非法字符或尾随点/空格。"
                    } else {
                        "名称扩展名必须与格式一致；不会自动隐藏不匹配。"
                    })
                    .child(
                        Input::new(&self.name)
                            .aria_label("压缩包名称")
                            .disabled(busy || draft.separately),
                    )
                    .child(self.picker_button(
                        "archive-save-picker",
                        "另存为…",
                        ArchivePickerPurpose::Save,
                        cx,
                    )),
            )
            .child(
                Field::new().label("格式").child(
                    Select::new(&self.format)
                        .accessibility_label("压缩格式")
                        .disabled(busy),
                ),
            )
            .child(
                Field::new()
                    .label("压缩等级")
                    .visible(draft.format != ArchiveCreateFormat::Tar)
                    .child(
                        Select::new(&self.level)
                            .accessibility_label("语义压缩等级")
                            .disabled(busy),
                    ),
            )
            .child(
                Field::new()
                    .label("ZIP 方法")
                    .visible(draft.format == ArchiveCreateFormat::Zip)
                    .description("设置密码时使用 AES-256；仅存储等级与 Store 方法同步。")
                    .child(
                        Select::new(&self.method)
                            .accessibility_label("ZIP 方法")
                            .disabled(busy),
                    ),
            );
        if draft::encrypted(draft.format) {
            form = form
                .child(
                    Field::new()
                        .label("密码（可选）")
                        .description("密码仅用于本次操作，不会保存到默认设置。")
                        .child(
                            Input::new(&self.password)
                                .aria_label("创建密码")
                                .mask_toggle()
                                .disabled(busy),
                        ),
                )
                .child(
                    Field::new()
                        .label("确认密码")
                        .description(
                            if self.password.read(cx).value() != self.confirm.read(cx).value() {
                                "两次输入的密码不一致。"
                            } else {
                                "与上方密码一致；各输入框眼睛按钮可独立显示或隐藏。"
                            },
                        )
                        .child(
                            Input::new(&self.confirm)
                                .aria_label("确认创建密码")
                                .mask_toggle()
                                .disabled(busy),
                        ),
                );
        }
        form.child(Field::new().label("7Z 选项").visible(draft.format==ArchiveCreateFormat::SevenZ)
                .child(self.check("archive-solid","固实压缩",draft.solid,Flag::Solid,false,cx))
                .child(self.check("archive-header","加密文件名（需要密码）",draft.encrypt_header,Flag::Header,false,cx)))
            .child(Field::new().label("来源与验证")
                .child(self.check("archive-separately","分别压缩每个来源",draft.separately,Flag::Separately,false,cx))
                .child(self.check("archive-strip","单个文件夹不保留根目录",draft.strip_single_root,Flag::Strip,stream||draft.sources.len()!=1||!draft.sources.first().is_some_and(|s|s.directory),cx))
                .child(self.check("archive-test","创建后再测试一次",draft.test_after_create,Flag::Test,false,cx))
                .description("提交前的安全校验始终执行；此选项只控制创建完成后的额外完整测试。GZ / XZ 只接受普通文件，可分别压缩。"))
            .child(Field::new().label("分卷大小（MiB，可选）")
                .description("仅 ZIP / 7Z；只创建全新的目标分卷组，不覆盖已有压缩包或分卷。")
                .child(Input::new(&self.volume).aria_label("分卷大小 MiB").disabled(busy)))
            .child(Field::new().label("排除规则").visible(!stream).description("每项一条文件名或相对路径规则，支持 * 和 ?，最多 32 条。")
                .child(self.exclusion_fields(cx)))
            .child(Field::new().label("完成后")
                .child(self.check("archive-create-keep-open", "完成后保留进度窗口", self.keep_open, Flag::Keep, false, cx))
                .child(self.check("archive-create-remember", "记住本次默认设置", self.remember, Flag::Remember, false, cx)))
    }
    fn modify_fields(&self, cx: &Context<Self>) -> Form {
        let Draft::Modify(draft) = &self.draft else {
            return Form::vertical();
        };
        let mut form =
            Form::vertical()
                .child(
                    Field::new()
                        .label("当前归档")
                        .description(draft.source.to_string_lossy().into_owned()),
                )
                .child(Field::new().label("源快照").description(
                    "编辑绑定当前读取的原始版本和条目 ID；提交前由工作进程再次验证。",
                ));
        match draft.action {
            ArchiveJobModifyAction::Add => {
                form = form.child(
                    Field::new()
                        .label("添加文件或文件夹")
                        .description(format!(
                            "已选 {} / 31 个来源；仅添加到当前目录。新增文件使用 Deflate，不加密；原有条目的压缩与加密记录保持不变。",
                            draft.added.len()
                        ))
                        .child(
                            div()
                                .flex()
                                .gap_2()
                                .child(self.picker_button(
                                    "archive-edit-add-files",
                                    "添加文件…",
                                    ArchivePickerPurpose::Files,
                                    cx,
                                ))
                                .child(self.picker_button(
                                    "archive-edit-add-folders",
                                    "添加文件夹…",
                                    ArchivePickerPurpose::Folders,
                                    cx,
                                )),
                        )
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .children(draft.added.iter().cloned()),
                        ),
                );
            }
            ArchiveJobModifyAction::Delete => {
                form = form.child(Field::new().label("删除选中条目").description(format!(
                    "从压缩包删除 {} 个条目及所选目录的内容。原文件在提交前保持不变。",
                    draft.ordinals.len()
                )));
            }
            ArchiveJobModifyAction::Rename | ArchiveJobModifyAction::NewDirectory => {
                form = form.child(
                    Field::new()
                        .label(if draft.action == ArchiveJobModifyAction::Rename {
                            "新名称"
                        } else {
                            "新文件夹"
                        })
                        .required(true)
                        .description("相对归档路径；文件夹以 / 结尾。非法路径不可提交。")
                        .child(
                            Input::new(&self.name)
                                .aria_label("编辑归档目标名称")
                                .disabled(self.busy()),
                        ),
                );
            }
            ArchiveJobModifyAction::Comment => {
                form = form.child(
                    Field::new()
                        .label("ZIP 注释")
                        .description("UTF-8 最多 65535 字节；不允许空字符。")
                        .child(
                            Textarea::new(&self.exclusions)
                                .aria_label("ZIP 注释")
                                .h(px(160.))
                                .disabled(self.busy()),
                        ),
                );
            }
        }
        form
    }
    fn extract_fields(&self, cx: &Context<Self>) -> Form {
        let Draft::Extract(draft) = &self.draft else {
            return Form::vertical();
        };
        let selected = draft.selection.as_ref().map_or(0, |s| s.ordinals.len());
        let preview = match draft.strategy {
            ArchiveDestination::Name => format!(
                "{}\\{}（发生冲突时生成唯一名称）",
                self.destination.read(cx).value(),
                draft::archive_stem(
                    &draft
                        .source
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                )
            ),
            ArchiveDestination::Smart => format!(
                "{} · 单一顶层项直接解压，多个顶层项使用压缩包名称文件夹；最终路径由工作进程预检确认",
                self.destination.read(cx).value()
            ),
            _ => self.destination.read(cx).value().to_string(),
        };
        Form::vertical()
            .child(
                Field::new()
                    .label("压缩包")
                    .description(draft.source.to_string_lossy().into_owned())
                    .child(self.picker_button(
                        "archive-extract-source",
                        "选择压缩包…",
                        ArchivePickerPurpose::Files,
                        cx,
                    )),
            )
            .child(
                Field::new()
                    .label("文件范围")
                    .description(format!(
                        "选定 {selected} 个条目；保留原始会话、修订和条目 ID。"
                    ))
                    .child(
                        RadioGroup::horizontal("archive-extract-range")
                            .selected_index(Some(usize::from(draft.selected_only)))
                            .disabled(self.busy())
                            .child(Radio::new("all").label("全部文件"))
                            .child(
                                Radio::new("selected")
                                    .label("选定文件")
                                    .disabled(selected == 0),
                            )
                            .on_change(cx.listener(|this, index, _, cx| {
                                if this.busy() {
                                    return;
                                }
                                if let Draft::Extract(draft) = &mut this.draft {
                                    draft.selected_only = *index == 1
                                        && draft
                                            .selection
                                            .as_ref()
                                            .is_some_and(|s| !s.ordinals.is_empty());
                                }
                                this.revalidate(cx);
                            })),
                    ),
            )
            .child(
                Field::new().label("目标策略").child(
                    Select::new(&self.strategy)
                        .accessibility_label("解压目标策略")
                        .disabled(self.busy()),
                ),
            )
            .child(Field::new().label("目标预览").description(preview))
            .child(
                Field::new().label("冲突处理").child(
                    Select::new(&self.conflict)
                        .accessibility_label("同名文件处理")
                        .disabled(self.busy()),
                ),
            )
            .child(
                Field::new()
                    .label("安全与结构")
                    .child(self.check(
                        "archive-single-file-folder",
                        "智能解压时单个文件也创建文件夹",
                        draft.single_file_folder,
                        Flag::SingleFile,
                        draft.strategy != ArchiveDestination::Smart,
                        cx,
                    ))
                    .child(self.check(
                        "archive-motw",
                        "保留下载来源标记（Mark-of-the-Web）",
                        draft.copy_motw,
                        Flag::Motw,
                        false,
                        cx,
                    )),
            )
    }
}
impl Focusable for OperationForm {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}
impl Render for OperationForm {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let create = matches!(&self.draft, Draft::Create(_));
        let modify = matches!(&self.draft, Draft::Modify(_));
        let here =
            matches!(&self.draft,Draft::Extract(draft) if draft.strategy==ArchiveDestination::Here);
        let form = if create {
            self.create_fields(cx)
        } else if modify {
            self.modify_fields(cx)
        } else {
            self.extract_fields(cx)
        };
        let form = if modify {
            form
        } else {
            form.child(
                Field::new()
                    .label("目标文件夹")
                    .required(true)
                    .child(
                        InputGroup::new("archive-destination-group")
                            .input(Input::new(&self.destination).aria_label("目标文件夹"))
                            .readonly(here)
                            .disabled(self.busy()),
                    )
                    .child(
                        self.picker_button(
                            "archive-destination-picker",
                            "浏览目标…",
                            ArchivePickerPurpose::Destination,
                            cx,
                        )
                        .disabled(self.busy() || here),
                    ),
            )
            .child(
                Field::new()
                    .label("完成后")
                    .child(self.check(
                        "archive-open-after",
                        "打开目标文件夹",
                        self.open_after,
                        Flag::Open,
                        false,
                        cx,
                    ))
                    .child(self.check(
                        "archive-keep-open",
                        "完成后保留进度窗口",
                        self.keep_open,
                        Flag::Keep,
                        false,
                        cx,
                    ))
                    .child(self.check(
                        "archive-remember",
                        "记住本表单支持的默认设置",
                        self.remember,
                        Flag::Remember,
                        false,
                        cx,
                    )),
            )
            .child(
                Field::new()
                    .label("来源保护")
                    .child(
                        Checkbox::new("archive-delete-unavailable")
                            .label("完成后删除来源（当前不可用）")
                            .checked(false)
                            .disabled(true),
                    )
                    .description(
                        "原始对象回收尚未通过验收；来源始终保留，不会永久删除，也不会执行回收。",
                    ),
            )
        };
        div()
            .size_full()
            .flex()
            .flex_col()
            .min_h_0()
            .min_w_0()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .track_focus(&self.focus)
            .capture_key_down(cx.listener(Self::default_enter))
            .capture_key_up(cx.listener(|this, event: &gpui_kit::KeyUpEvent, _, _| {
                if event.keystroke.key == "enter" {
                    this.default_enter_held = false;
                }
            }))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                if event.keystroke.key == "escape" && this.pending_format.is_none() {
                    this.close(window, cx);
                    cx.stop_propagation();
                }
            }))
            .child(TitleBar::new().child(if create {
                "新建压缩包"
            } else if modify {
                "编辑 ZIP"
            } else {
                "解压压缩包"
            }))
            .child(
                div()
                    .id("archive-form-body")
                    .debug_selector(|| "archive-form-body".into())
                    .flex_1()
                    .min_h_0()
                    .min_w_0()
                    .overflow_y_scroll()
                    .p_4()
                    .child(form),
            )
            .child(
                div()
                    .id("archive-form-footer")
                    .flex_none()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .p_4()
                    .when_some(
                        self.error.clone().or_else(|| self.validation.clone()),
                        |footer, message| {
                            footer.child(
                                div()
                                    .id("archive-form-errors")
                                    .max_h(px(80.))
                                    .overflow_y_scroll()
                                    .child(Alert::warning("archive-form-error", message)),
                            )
                        },
                    )
                    .when(self.enumerating, |footer| {
                        footer.child("正在读取来源元数据…（不递归扫描）")
                    })
                    .child(
                        div()
                            .flex()
                            .justify_end()
                            .gap_2()
                            .child(
                                Button::new("archive-form-cancel")
                                    .label("取消")
                                    .outline()
                                    .with_size(Size::Medium)
                                    .debug_selector(|| "archive-form-cancel".into())
                                    .on_click(
                                        cx.listener(|this, _, window, cx| this.close(window, cx)),
                                    ),
                            )
                            .child(
                                Button::new("archive-form-submit")
                                    .label(if create {
                                        "创建"
                                    } else if modify {
                                        "应用编辑"
                                    } else {
                                        "解压"
                                    })
                                    .primary()
                                    .with_size(Size::Medium)
                                    .debug_selector(|| "archive-form-submit".into())
                                    .disabled(self.busy() || self.validation.is_some())
                                    .loading(self.submitted)
                                    .on_click(
                                        cx.listener(|this, _, window, cx| this.submit(window, cx)),
                                    ),
                            ),
                    ),
            )
    }
}
fn input(
    value: &str,
    masked: bool,
    window: &mut Window,
    cx: &mut Context<OperationForm>,
) -> Entity<InputState> {
    cx.new(|cx| {
        let mut state = InputState::new(window, cx).masked(masked);
        state.set_value(value.to_owned(), window, cx);
        state
    })
}
fn choice(
    items: &[&'static str],
    index: usize,
    window: &mut Window,
    cx: &mut Context<OperationForm>,
) -> Choice {
    cx.new(|cx| {
        SelectState::new(
            items.to_vec(),
            Some(IndexPath::default().row(index)),
            window,
            cx,
        )
    })
}
fn source_metadata(paths: Vec<String>, cancel: &AtomicBool) -> Result<Vec<DraftSource>, String> {
    if paths.is_empty() || paths.len() > 32 {
        return Err("请选择 1–32 个来源。".into());
    }
    let mut sources = Vec::with_capacity(paths.len());
    let mut names = std::collections::HashSet::new();
    for path in paths {
        if cancel.load(Ordering::Acquire) {
            return Err("来源读取已取消。".into());
        }
        if !names.insert(path.to_lowercase()) {
            return Err("本次选择包含重复来源；未添加任何输入。".into());
        }
        let metadata = std::fs::symlink_metadata(&path)
            .map_err(|_| "无法读取来源，请检查访问权限并重新选择。")?;
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt as _;
            if metadata.file_attributes() & 0x400 != 0 {
                return Err("不能将重解析点或链接作为来源；请选择原始文件或文件夹。".into());
            }
        }
        if metadata.file_type().is_symlink() || (!metadata.is_file() && !metadata.is_dir()) {
            return Err("来源必须是普通文件或文件夹。".into());
        }
        sources.push(DraftSource {
            path: path.into(),
            directory: metadata.is_dir(),
            bytes: metadata.is_file().then_some(metadata.len()),
        });
    }
    Ok(sources)
}
fn format_at(index: usize) -> ArchiveCreateFormat {
    match index {
        1 => ArchiveCreateFormat::SevenZ,
        2 => ArchiveCreateFormat::Tar,
        3 => ArchiveCreateFormat::Tgz,
        4 => ArchiveCreateFormat::Txz,
        5 => ArchiveCreateFormat::Gz,
        6 => ArchiveCreateFormat::Xz,
        _ => ArchiveCreateFormat::Zip,
    }
}
fn format_index(value: ArchiveCreateFormat) -> usize {
    match value {
        ArchiveCreateFormat::Zip => 0,
        ArchiveCreateFormat::SevenZ => 1,
        ArchiveCreateFormat::Tar => 2,
        ArchiveCreateFormat::Tgz => 3,
        ArchiveCreateFormat::Txz => 4,
        ArchiveCreateFormat::Gz => 5,
        ArchiveCreateFormat::Xz => 6,
    }
}
fn level_at(index: usize) -> ArchiveCompressionLevel {
    match index {
        0 => ArchiveCompressionLevel::Store,
        1 => ArchiveCompressionLevel::Fast,
        3 => ArchiveCompressionLevel::Maximum,
        _ => ArchiveCompressionLevel::Normal,
    }
}
fn level_index(value: ArchiveCompressionLevel) -> usize {
    match value {
        ArchiveCompressionLevel::Store => 0,
        ArchiveCompressionLevel::Fast => 1,
        ArchiveCompressionLevel::Normal => 2,
        ArchiveCompressionLevel::Maximum => 3,
    }
}
fn strategy_at(index: usize) -> ArchiveDestination {
    match index {
        1 => ArchiveDestination::Smart,
        2 => ArchiveDestination::Here,
        3 => ArchiveDestination::Name,
        _ => ArchiveDestination::Ask,
    }
}
fn strategy_index(value: ArchiveDestination) -> usize {
    match value {
        ArchiveDestination::Ask => 0,
        ArchiveDestination::Smart => 1,
        ArchiveDestination::Here => 2,
        ArchiveDestination::Name => 3,
    }
}
fn conflict_at(index: usize) -> ArchiveConflictPolicy {
    match index {
        1 => ArchiveConflictPolicy::Overwrite,
        2 => ArchiveConflictPolicy::Skip,
        3 => ArchiveConflictPolicy::AutoRename,
        _ => ArchiveConflictPolicy::Ask,
    }
}
fn conflict_index(value: ArchiveConflictPolicy) -> usize {
    match value {
        ArchiveConflictPolicy::Ask => 0,
        ArchiveConflictPolicy::Overwrite => 1,
        ArchiveConflictPolicy::Skip => 2,
        ArchiveConflictPolicy::AutoRename => 3,
    }
}

#[cfg(test)]
#[path = "forms_tests.rs"]
mod tests;
