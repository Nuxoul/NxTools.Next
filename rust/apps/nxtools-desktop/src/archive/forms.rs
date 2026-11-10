//! Archive operation decision surfaces: current Kit 0.8.0 Form/Field and stable Input/Select.
//! Create 680 / extract 560 starting widths belong to the parent window. One scroll body,
//! fixed Medium (32px) primary/outline footer; no Toolbar.child variant coercion or extra Root.
//! Native picker, admission, settings persistence and jobs remain ArchiveApplication's owners.
#[path = "forms_intake.rs"]
mod intake;
#[path = "forms_layout.rs"]
mod layout;
#[path = "forms_recovery.rs"]
mod recovery;
#[path = "forms_space.rs"]
mod space_view;
#[path = "forms_volume.rs"]
mod volume_view;
use super::{
    ArchiveApplication,
    draft::{self, CreateDraft, DraftSelection, DraftSource, ExtractDraft, ModifyDraft},
};
use gpui_kit::base::TestSupportExt as _;
use gpui_kit::component::{
    ActiveTheme, Disableable, IndexPath, Sizable, Size, WindowExt,
    alert::Alert,
    button::{Button, ButtonVariants},
    checkbox::Checkbox,
    description_list::DescriptionList,
    dialog::{DialogAction, DialogClose, DialogFooter},
    form::{Field, Form},
    input::{Input, InputEvent, InputGroup, InputState, Textarea, TextareaState},
    radio::{Radio, RadioGroup},
    scroll::ScrollableElement,
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
const SEVENZ_EDIT_COST: &str = "7z 编辑会完整解码并重编码为非固实 LZMA2；保留支持的内容、时间、属性与加密设置。需要原压缩包与候选并存空间；无法准确估算时为未知。";
const TAR_EDIT_COST: &str = "TAR 编辑会完整重写；tar.gz / tar.xz 会完整解码与重编码。保留已确认的内容与原成员元数据；新增项使用 uid/gid/mtime=0、文件0644/目录0755，不复制 ACL/ADS。gzip 保留可支持头属性，压缩级别按 XFL 映射；xz 使用 LZMA2 fast，保留校验类型，压缩参数和布局可能变化。需要原件、候选及有限匿名 spool 并存空间，估算未知，不计源删除收益。TAR 没有内容校验和；提交前另做完整内容比较。";
type Choice = Entity<SelectState<Vec<&'static str>>>;

#[derive(Default)]
pub(super) struct FormCompletion {
    pub(super) open_after: bool,
    pub(super) keep_open: bool,
    pub(super) remember: Option<ArchiveSettings>,
    pub(super) remember_baseline: Option<ArchiveSettings>,
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
    RememberDestination,
    Reclamation,
}

pub(crate) struct OperationForm {
    pub(super) id: Uuid,
    pub(super) generation: u64,
    owner: WeakEntity<ArchiveApplication>,
    settings: ArchiveSettings,
    draft: Draft,
    modify_objects: Vec<String>,
    source_bound: bool,
    destination: Entity<InputState>,
    source: Entity<InputState>,
    name: Entity<InputState>,
    password: Entity<InputState>,
    confirm: Entity<InputState>,
    recovery_id: Entity<InputState>,
    exclusions: Entity<TextareaState>,
    create_exclusions: Vec<Entity<TextareaState>>,
    create_rule_subscriptions: Vec<Subscription>,
    volume: Entity<InputState>,
    volume_unit: Choice,
    volume_preset: Choice,
    format: Choice,
    level: Choice,
    method: Choice,
    strategy: Choice,
    conflict: Choice,
    pending_format: Option<ArchiveCreateFormat>,
    pending_picker: Option<PendingPicker>,
    pending_clipboard: Option<nxtools_protocol::archive_clipboard::ArchiveClipboardCommand>,
    enumeration_epoch: u64,
    enumeration_cancel: Arc<AtomicBool>,
    enumerating: bool,
    error: Option<String>,
    validation: Option<String>,
    submitted: bool,
    submit_epoch: u64,
    default_enter_held: bool,
    reclamation_confirmation_pending: bool,
    reclamation_acknowledged: bool,
    pub(in crate::archive) retired: bool,
    open_after: bool,
    keep_open: bool,
    remember: bool,
    focus: FocusHandle,
    _subscriptions: Vec<Subscription>,
    space: super::space::FormState,
    recovery: recovery::RecoveryState,
    #[cfg(test)]
    submit_count: usize,
}
#[cfg(test)]
impl OperationForm {
    pub(in crate::archive) fn sources_for_test(&self) -> (bool, Vec<PathBuf>) {
        (
            self.enumerating,
            match &self.draft {
                Draft::Create(draft) => draft
                    .sources
                    .iter()
                    .map(|source| source.path.clone())
                    .collect(),
                Draft::Modify(draft) => draft.added.iter().map(PathBuf::from).collect(),
                _ => vec![],
            },
        )
    }
    pub(in crate::archive) fn confirmation_strategy_for_test(&self) -> Option<ArchiveDestination> {
        match &self.draft {
            Draft::Extract(draft) => Some(draft.strategy),
            _ => None,
        }
    }
    pub(in crate::archive) fn revise_submitted_create_volume_for_test(
        &mut self,
        size: nxtools_core::archive::ArchiveVolumeSize,
    ) {
        assert!(self.submitted, "only probe an already submitted draft");
        let Draft::Create(draft) = &mut self.draft else {
            panic!("submitted Create draft");
        };
        draft.volume_mib = size.value.to_string();
        draft.volume_unit = size.unit;
        assert_eq!(draft.volume_size().expect("revised draft size"), Some(size));
    }
}
#[cfg(test)]
impl OperationForm {
    pub(in crate::archive) fn arm_picker_for_test(&mut self, operation: Uuid) {
        self.pending_picker = Some(PendingPicker {
            operation: Some(operation),
            purpose: ArchivePickerPurpose::Destination,
        });
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
    pub(super) fn continue_reclamation(
        settings: ArchiveSettings,
        source: PathBuf,
        destination: Option<PathBuf>,
        owner: WeakEntity<ArchiveApplication>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let destination = destination.map(|path| path.to_string_lossy().into_owned());
        let mut draft = ExtractDraft::continue_reclamation(&settings, source);
        draft.destination = destination.clone().unwrap_or_default();
        let mut form = Self::new(settings, Draft::Extract(draft), owner, window, cx);
        form.begin_recovery(destination, window, cx);
        form
    }
    pub(super) fn extract_with_strategy(
        settings: ArchiveSettings,
        extract: (PathBuf, Option<DraftSelection>, Option<ArchiveDestination>),
        owner: WeakEntity<ArchiveApplication>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let (source, selection, strategy) = extract;
        if strategy.is_none() {
            return Self::extract(settings, source, selection, owner, window, cx);
        }
        let mut draft = ExtractDraft::new(&settings, source, selection);
        if let Some(strategy) = strategy {
            draft.strategy = strategy;
            if strategy == ArchiveDestination::Here {
                draft.destination = draft
                    .source
                    .parent()
                    .map(|parent| parent.to_string_lossy().into_owned())
                    .unwrap_or_default();
            }
        }
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
        let source_bound =
            matches!(&draft, Draft::Extract(draft) if !draft.source.as_os_str().is_empty());
        let source_value = match &draft {
            Draft::Extract(draft) => draft.source.to_string_lossy().into_owned(),
            Draft::Modify(draft) => draft.source.to_string_lossy().into_owned(),
            Draft::Create(_) => String::new(),
        };
        let (destination_value, name_value, exclusions_value, volume_value, recovery_id_value) =
            match &draft {
                Draft::Create(d) => (
                    d.destination.clone(),
                    d.name.clone(),
                    String::new(),
                    d.volume_mib.clone(),
                    String::new(),
                ),
                Draft::Extract(d) => (
                    d.destination.clone(),
                    String::new(),
                    String::new(),
                    String::new(),
                    d.recovery_id.clone(),
                ),
                Draft::Modify(d) => (
                    String::new(),
                    d.value.clone().unwrap_or_default(),
                    d.value.clone().unwrap_or_default(),
                    String::new(),
                    String::new(),
                ),
            };
        let destination = input(&destination_value, false, window, cx);
        let source = input(&source_value, false, window, cx);
        let name = input(&name_value, false, window, cx);
        let password = input("", true, window, cx);
        let confirm = input("", true, window, cx);
        let volume = input(&volume_value, false, window, cx);
        volume.update(cx, |state, cx| state.set_step(None, window, cx));
        let initial_unit = match &draft {
            Draft::Create(draft) => draft.volume_unit,
            _ => nxtools_core::archive::ArchiveVolumeUnit::MiB,
        };
        let volume_unit = choice(volume_view::VOLUME_UNITS, initial_unit as usize, window, cx);
        let volume_preset = choice(
            volume_view::VOLUME_PRESETS,
            volume_view::preset_index(&volume_value, initial_unit),
            window,
            cx,
        );
        let recovery_id = input(&recovery_id_value, false, window, cx);
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
        let initial_strategy = match &draft {
            Draft::Extract(draft) => draft.strategy,
            _ => settings.default_destination,
        };
        let initial_conflict = match &draft {
            Draft::Extract(draft) => draft.conflict,
            _ => settings.conflict_policy,
        };
        let strategy = choice(STRATEGIES, strategy_index(initial_strategy), window, cx);
        let conflict = choice(CONFLICTS, conflict_index(initial_conflict), window, cx);
        let mut subscriptions = Vec::new();
        subscriptions.push(cx.subscribe_in(
            &volume_preset,
            window,
            |_, _, event: &SelectEvent<Vec<&'static str>>, window, cx| {
                let SelectEvent::Confirm(Some(value)) = event else {
                    return;
                };
                let value = *value;
                cx.defer_in(window, move |this, window, cx| {
                    if let Some(index) = volume_view::VOLUME_PRESETS
                        .iter()
                        .position(|label| *label == value)
                    {
                        this.select_volume_preset(index, window, cx);
                    }
                });
            },
        ));
        subscriptions.push(cx.subscribe_in(
            &volume_unit,
            window,
            |_, _, event: &SelectEvent<Vec<&'static str>>, window, cx| {
                let SelectEvent::Confirm(Some(value)) = event else {
                    return;
                };
                let value = *value;
                cx.defer_in(window, move |this, window, cx| {
                    if let Some(unit) = nxtools_core::archive::ArchiveVolumeUnit::ALL
                        .into_iter()
                        .find(|unit| unit.label() == value)
                    {
                        this.select_volume_unit(unit, window, cx);
                    }
                });
            },
        ));
        subscriptions.push(cx.subscribe_in(
            &volume,
            window,
            |_, _, event: &gpui_kit::component::input::NumberInputEvent, window, cx| {
                let gpui_kit::component::input::NumberInputEvent::Step(action) = *event;
                cx.defer_in(window, move |this, window, cx| {
                    this.step_volume(action, window, cx)
                });
            },
        ));
        subscriptions.push(cx.subscribe_in(
            &volume,
            window,
            |_, _, event: &InputEvent, window, cx| {
                if matches!(event, InputEvent::Change) {
                    cx.defer_in(window, |this, window, cx| {
                        this.revalidate(cx);
                        if let Draft::Create(draft) = &this.draft {
                            let label = volume_view::VOLUME_PRESETS
                                [volume_view::preset_index(&draft.volume_mib, draft.volume_unit)];
                            this.volume_preset.update(cx, |state, cx| {
                                state.set_selected_value(&label, window, cx)
                            });
                        }
                    });
                }
            },
        ));
        for state in [&destination, &name, &password, &confirm, &recovery_id] {
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
            modify_objects: Vec::new(),
            destination,
            source,
            source_bound,
            name,
            password,
            confirm,
            recovery_id,
            volume,
            volume_unit,
            volume_preset,
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
            pending_clipboard: None,
            enumeration_epoch: 0,
            enumeration_cancel: Arc::new(AtomicBool::new(false)),
            enumerating: false,
            error: None,
            validation: None,
            submitted: false,
            submit_epoch: 0,
            default_enter_held: false,
            reclamation_confirmation_pending: false,
            reclamation_acknowledged: false,
            retired: false,
            open_after,
            keep_open,
            remember: false,
            focus,
            _subscriptions: subscriptions,
            space: Default::default(),
            recovery: Default::default(),
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
        let Some(input) = [&self.source, &self.name, &self.destination, &self.volume]
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
    pub(super) fn busy(&self) -> bool {
        self.retired
            || self.submitted
            || self.enumerating
            || self.pending_picker.is_some()
            || self.pending_clipboard.is_some()
            || self.pending_format.is_some()
            || self.recovery.pending
            || self.reclamation_confirmation_pending
    }
    pub(in crate::archive) fn output_plan_current(&self, id: Uuid, generation: u64) -> bool {
        self.id == id && self.generation == generation && self.submitted && !self.retired
    }
    fn sync_inputs(&mut self, cx: &App) {
        let recovery_facts = self.recovery_facts();
        let recovery_candidate = self.recovery.candidate_destination.clone();
        match &mut self.draft {
            Draft::Create(draft) => {
                draft.name = self.name.read(cx).value().to_string();
                draft.destination = self.destination.read(cx).value().to_string();
                draft.password = self.password.read(cx).value().to_string();
                draft.confirm = self.confirm.read(cx).value().to_string();
                draft.volume_mib = self.volume.read(cx).value().to_string();
                if draft.password.is_empty() || draft.password != draft.confirm {
                    draft.encrypt_header = false;
                }
                draft.exclusions = self
                    .create_exclusions
                    .iter()
                    .map(|rule| rule.read(cx).value().to_string())
                    .collect();
            }
            Draft::Extract(draft) => {
                if draft.continue_reclamation {
                    if let Some((destination, recovery_id, copy_motw)) = &recovery_facts {
                        draft.destination = destination.clone();
                        draft.recovery_id = recovery_id.clone();
                        draft.copy_motw = *copy_motw;
                    } else {
                        draft.destination = recovery_candidate.clone().unwrap_or_default();
                        draft.recovery_id.clear();
                        draft.copy_motw = false;
                    }
                } else {
                    draft.destination = self.destination.read(cx).value().to_string();
                    draft.recovery_id = self.recovery_id.read(cx).value().to_string();
                }
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
        self.require_recovery_ready()?;
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
                remembered.create_volume_size = draft.volume_size()?;
                requests
            }
            Draft::Extract(draft) => {
                remembered.default_destination = draft.strategy;
                remembered.conflict_policy = draft.conflict;
                remembered.copy_motw = draft.copy_motw;
                remembered.open_after_extract = draft.open_after;
                remembered.keep_extract_progress_open = draft.keep_open;
                remembered.smart_single_file_folder = draft.single_file_folder;
                remembered.extract_custom_destination = if draft.remember_destination {
                    if draft.strategy == ArchiveDestination::Here {
                        self.settings.extract_custom_destination.clone()
                    } else {
                        Some(draft.destination.clone())
                    }
                } else {
                    None
                };
                draft.request(&self.settings).map(|request| vec![request])
            }
            Draft::Modify(draft) => draft.request().map(|request| vec![request]),
        }?;
        let reclamation = matches!(&self.draft, Draft::Extract(draft) if draft.reclamation);
        if self.remember && !reclamation {
            remembered.validate()?;
        }
        Ok((
            result,
            FormCompletion {
                open_after: self.open_after,
                keep_open: self.keep_open,
                remember: (!reclamation
                    && !matches!(self.draft, Draft::Modify(_))
                    && self.remember)
                    .then_some(remembered),
                remember_baseline: (!reclamation && self.remember).then(|| self.settings.clone()),
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
        self.reclamation_acknowledged = false;
        self.validation = self.capture(cx).err();
        self.error = None;
        if let Draft::Extract(draft) = &self.draft {
            if draft.continue_reclamation {
                self.space.invalidate(None);
                self.space.pending = false;
                self.space.waiting = false;
                self.space.plan = None;
                self.space.error = None;
            } else {
                let initial = self.space.invalidate(draft.request(&self.settings).ok());
                if initial && self.space.key.is_some() {
                    let entity = cx.entity().downgrade();
                    cx.defer(move |cx| {
                        let _ = entity.update(cx, |form, cx| form.refresh_space(cx));
                    });
                }
            }
        }
        cx.notify();
    }
    fn submit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy() || !self.recovery_submit_ready() {
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
        let destructive_reclamation = requests.first().is_some_and(|request| {
            request
                .extract
                .as_ref()
                .is_some_and(|extract| extract.reclamation.is_some())
        });
        if destructive_reclamation && !self.reclamation_acknowledged {
            self.show_reclamation_confirmation(window, cx);
            return;
        }
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
                            "压缩包操作所有者已关闭，请重新打开表单。".into(),
                            window,
                            cx,
                        )
                    });
                });
            }
        });
        cx.notify();
    }
    fn show_reclamation_confirmation(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.reclamation_confirmation_pending {
            return;
        }
        self.reclamation_confirmation_pending = true;
        let confirm = cx.entity().downgrade();
        let cancel = confirm.clone();
        let close = confirm.clone();
        let correlation_id = self.id;
        tracing::info!(event="archive.reclamation.confirmation_shown", %correlation_id, "destructive reclamation acknowledgement requested");
        window.open_dialog(cx, move |dialog, _, _| {
            let confirm = confirm.clone();
            let cancel = cancel.clone();
            let close = close.clone();
            dialog
                .title("确认永久消耗原压缩包数据")
                .child(
                    Alert::warning(
                        "archive-reclamation-confirmation-warning",
                        "原压缩包中的文件数据会在解压核验并保存恢复记录后逐步释放，不能撤销，也不会进入回收站。中断后请保留原压缩包、恢复记录和已解压文件，使用“继续空间回收解压”。本版本只能继续剩余解压，不能重建剩余压缩包或恢复完整原压缩包。当前文件完整输出所需的空间仍必须足够。",
                    )
                    .with_size(Size::Small),
                )
                .footer(
                    DialogFooter::new()
                        .child(DialogClose::new().trigger(|button| {
                            button
                                .label("保留原压缩包，不开始")
                                .outline()
                                .with_size(Size::Medium)
                        }))
                        .child(DialogAction::new().child(
                            Button::new("archive-reclamation-confirm")
                                .debug_selector(|| "archive-reclamation-confirm".into())
                                .label("我已了解，开始消耗原压缩包")
                                .outline()
                                .with_size(Size::Medium),
                        )),
                )
                .on_ok(move |_, window, cx| {
                    let confirm = confirm.clone();
                    window.defer(cx, move |window, cx| {
                        let _ = confirm.update(cx, |form, cx| {
                            form.resolve_reclamation_confirmation(true, window, cx)
                        });
                    });
                    true
                })
                .on_cancel(move |_, window, cx| {
                    let cancel = cancel.clone();
                    window.defer(cx, move |window, cx| {
                        let _ = cancel.update(cx, |form, cx| {
                            form.resolve_reclamation_confirmation(false, window, cx)
                        });
                    });
                    true
                })
                .on_close(move |_, window, cx| {
                    let close = close.clone();
                    window.defer(cx, move |window, cx| {
                        let _ = close.update(cx, |form, cx| {
                            form.resolve_reclamation_confirmation(false, window, cx)
                        });
                    });
                })
        });
        cx.notify();
    }
    fn resolve_reclamation_confirmation(
        &mut self,
        accepted: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !std::mem::replace(&mut self.reclamation_confirmation_pending, false) || self.retired {
            return;
        }
        if accepted {
            self.reclamation_acknowledged = true;
            tracing::info!(event="archive.reclamation.acknowledged", correlation_id=%self.id, "destructive reclamation acknowledged");
            self.submit(window, cx);
        } else {
            cx.notify();
        }
    }
    pub(super) fn fail_submit(
        &mut self,
        message: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.retired {
            return;
        }
        self.submit_epoch = self.submit_epoch.wrapping_add(1);
        self.submitted = false;
        if matches!(&self.draft, Draft::Modify(draft) if draft.action == ArchiveJobModifyAction::Add)
        {
            self.error = None;
            if let Some(center) = crate::notifications::current(cx) {
                let notice = crate::notifications::AppNotification::new(
                    format!("archive-form-{}", self.id),
                    crate::notifications::NotificationLevel::Error,
                    "添加文件失败",
                    message,
                )
                .in_window(window.window_handle());
                cx.defer(move |cx| center.update(cx, |center, cx| center.push(notice, cx)));
            }
        } else {
            self.error = Some(message);
        }
        cx.notify();
    }
    pub(super) fn release_failed_submit(&mut self, cx: &mut Context<Self>) {
        if self.retired {
            return;
        }
        self.submit_epoch = self.submit_epoch.wrapping_add(1);
        self.submitted = false;
        self.error = None;
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
        self.retire_recovery();
        self.space.pending = false;
        self.space.epoch = self.space.epoch.wrapping_add(1);
        self.space.plan = None;
        self.generation = self.generation.wrapping_add(1).max(1);
        self.pending_picker = None;
        self.pending_clipboard = None;
        self.pending_format = None;
        self.reclamation_confirmation_pending = false;
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
        let owner = self.owner.clone();
        let id = self.id;
        cx.defer(move |cx| {
            let _ = owner.update(cx, |app, cx| {
                app.retire_form(id);
                app.refresh_all_batch_views(cx);
            });
        });
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
        if self.busy() || (purpose == ArchivePickerPurpose::Files && self.source_bound) {
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
        let directory =
            if purpose == ArchivePickerPurpose::Files && matches!(&self.draft, Draft::Extract(_)) {
                std::path::Path::new(self.source.read(cx).value().as_ref())
                    .parent()
                    .map(|p| p.to_string_lossy().into_owned())
            } else {
                Some(self.destination.read(cx).value().to_string())
            };
        let directory = directory.filter(|path| !path.is_empty() && valid_job_path(path));
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
                    if form.error.is_none() {
                        form.error = Some("文件选择器不可用或繁忙，请重试。".into());
                    }
                }
                cx.notify();
            });
        });
        cx.notify();
    }
    pub(super) fn picker_receipt_failure(
        &mut self,
        operation: Uuid,
        message: &str,
        cx: &mut Context<Self>,
    ) {
        if self.retired
            || self
                .pending_picker
                .as_ref()
                .is_none_or(|pending| pending.operation != Some(operation))
        {
            return;
        }
        self.pending_picker = None;
        self.error = Some(message.into());
        cx.notify();
    }
    pub(super) fn picker_disconnected(&mut self, cx: &mut Context<Self>) {
        if !self.retired && self.pending_picker.take().is_some() {
            self.error = Some("选择器连接已断开，请重试。".into());
            cx.notify();
        }
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
        self.error = None;
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
        if result.validate().is_err()
            || result.paths.is_empty()
            || result.paths.len() > 32
            || result.paths.iter().any(|p| !valid_job_path(p))
        {
            self.error = Some("选择结果无效或超过 32 个来源；未丢弃或添加任何输入。".into());
            cx.notify();
            return;
        }
        match purpose {
            ArchivePickerPurpose::Files | ArchivePickerPurpose::Folders => match &mut self.draft {
                Draft::Extract(_) if self.source_bound => {
                    self.error = Some("当前压缩包已固定，不能替换来源。".into());
                }
                Draft::Extract(draft) => {
                    if result.paths.len() != 1 {
                        self.error = Some("解压表单一次只接受一个压缩包；本次未更改来源。".into());
                    } else {
                        draft.source = PathBuf::from(&result.paths[0]);
                        draft.selection = None;
                        draft.selected_only = false;
                        if draft.strategy == ArchiveDestination::Here {
                            let parent = draft
                                .source
                                .parent()
                                .map(|parent| parent.to_string_lossy().into_owned())
                                .unwrap_or_default();
                            self.destination
                                .update(cx, |input, cx| input.set_value(parent, window, cx));
                        }
                        self.revalidate(cx);
                    }
                }
                Draft::Create(_) => self.accept_sources(result.paths, cx),
                Draft::Modify(draft) if draft.action == ArchiveJobModifyAction::ReplaceContent => {
                    if purpose != ArchivePickerPurpose::Files || result.paths.len() != 1 {
                        self.error =
                            Some("替换内容必须选择一个普通文件，不能选择文件夹或多个来源。".into());
                    } else {
                        self.enumerate_sources(result.paths, cx);
                    }
                }
                Draft::Modify(_) => self.accept_sources(result.paths, cx),
            },
            ArchivePickerPurpose::Destination => {
                if result.paths.len() != 1 {
                    self.error = Some("请选择一个目标文件夹。".into());
                } else if self.is_continuation() {
                    self.begin_recovery(Some(result.paths[0].clone()), window, cx);
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
    pub(super) fn enumerate_sources(&mut self, paths: Vec<String>, cx: &mut Context<Self>) {
        let within_limit = match &self.draft {
            Draft::Create(draft) => draft.sources.len() + paths.len() <= 32,
            Draft::Modify(draft) if draft.action == ArchiveJobModifyAction::ReplaceContent => {
                paths.len() == 1
            }
            _ => return,
        };
        if !within_limit {
            self.error = Some(
                if matches!(self.draft, Draft::Create(_)) {
                    "最多选择 32 个来源；本次未添加任何来源，未静默截断。请分批选择。"
                } else {
                    "替换内容只能选择一个普通文件；未更改任何来源。"
                }
                .into(),
            );
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
                            let roots = draft.sources.iter().chain(&sources).map(|source| source.path.to_string_lossy().into_owned()).collect::<Vec<_>>();
                            if !intake::valid_source_roots(roots.iter().map(String::as_str), 32, None) {form.error=Some("来源重复、重叠或超出预算；本次未添加任何输入。".into());}
                            else {draft.sources.extend(sources);form.revalidate(cx);}
                        }
                        else if let Draft::Modify(draft) = &mut form.draft {
                            if draft.action == ArchiveJobModifyAction::ReplaceContent && sources.len() == 1
                                && !sources[0].directory
                                && !sources[0].path.to_string_lossy().eq_ignore_ascii_case(&draft.source.to_string_lossy())
                            {
                                draft.added = vec![sources[0].path.to_string_lossy().into_owned()];
                                form.revalidate(cx);
                            } else {
                                form.error = Some("替换内容必须选择一个不同于原压缩包的普通文件；原选择保持不变。".into());
                            }
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
        if matches!(flag, Flag::RememberDestination) {
            tracing::info!(event="archive.form.destination_preference_changed", correlation_id=%self.id, value, "archive destination persistence opt-in changed");
        }
        if matches!(flag, Flag::Reclamation) {
            tracing::info!(event="archive.reclamation.choice_changed", correlation_id=%self.id, enabled=value, "archive reclamation explicit choice changed");
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
                    Flag::RememberDestination => draft.remember_destination = value,
                    Flag::Reclamation if !draft.continue_reclamation => draft.reclamation = value,
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
            .debug_selector(move || id.into())
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
            .debug_selector(move || id.into())
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
    pub(super) fn set_modify_objects(&mut self, objects: Vec<String>) {
        self.modify_objects = objects;
    }
    fn modify_fields(&self, cx: &Context<Self>) -> Form {
        let Draft::Modify(draft) = &self.draft else {
            return Form::vertical();
        };
        let mut form =
            Form::vertical()
                .child(
                    Field::new()
                        .label("当前压缩包")
                        .description(draft.source.to_string_lossy().into_owned()),
                )
                .child(Field::new().label("源快照").description(
                    "编辑绑定当前读取的原始版本和条目 ID；提交前由工作进程再次验证。",
                ));
        if !self.modify_objects.is_empty() {
            form = form.child(Field::new().label("本次压缩包内对象（仅供核对）").child(
                div().flex().flex_col().gap_1().children(
                    self.modify_objects.iter().enumerate().map(|(index, name)| {
                        div()
                            .id(gpui_kit::SharedString::from(format!(
                                "archive-edit-object-{index}"
                            )))
                            .role(gpui_kit::Role::Label)
                            .aria_label(name.clone())
                            .test_support()
                            .child(
                                gpui_kit::component::label::Label::new(name.clone())
                                    .whitespace_normal(),
                            )
                    }),
                ),
            ));
        }
        if draft.browser.format == "SevenZ" {
            form = form.child(
                Field::new().label("7z 编辑成本与格式变化").child(
                    div()
                        .id("archive-sevenz-cost")
                        .role(gpui_kit::Role::Note)
                        .aria_label(SEVENZ_EDIT_COST)
                        .debug_selector(|| "archive-sevenz-cost".into())
                        .child(
                            Alert::warning("archive-sevenz-reencode", SEVENZ_EDIT_COST)
                                .with_size(Size::Small),
                        ),
                ),
            );
        }
        if matches!(draft.browser.format.as_str(), "Tar" | "TarGz" | "TarXz") {
            form = form.child(
                Field::new().label("TAR 编辑成本与保真范围").child(
                    div()
                        .id("archive-tar-cost")
                        .role(gpui_kit::Role::Note)
                        .aria_label(TAR_EDIT_COST)
                        .debug_selector(|| "archive-tar-cost".into())
                        .child(
                            Alert::warning("archive-tar-reencode", TAR_EDIT_COST)
                                .with_size(Size::Small),
                        ),
                ),
            );
        }
        match draft.action {
            ArchiveJobModifyAction::Add => {
                let target = draft.value.as_deref().unwrap_or("/").to_owned();
                form = form.child(Field::new().label("压缩包内目标目录").child(
                    div().id("archive-edit-target-folder").role(gpui_kit::Role::Label).aria_label(target.clone()).test_support().child(gpui_kit::component::label::Label::new(target).whitespace_normal()),
                )).child(Field::new().label("同名处理").child(
                    div().id("archive-edit-conflict-policy").role(gpui_kit::Role::Note).aria_label("逐项询问；仅当前任务可选择应用到后续冲突。不同类型或改名目标冲突会拒绝，不会先删除再提交。")
                        .test_support().child("逐项询问；仅当前任务可选择应用到后续冲突。不同类型或改名目标冲突会拒绝，不会先删除再提交。"),
                ));
                form = form.child(
                    Field::new()
                        .label("添加文件或文件夹")
                        .description(if draft.browser.format == "SevenZ" {
                            format!("已选 {} / 31 个来源；总大小未知。仅添加到当前目录，保留原压缩包支持的加密设置。", draft.added.len())
                        } else if matches!(draft.browser.format.as_str(), "Tar" | "TarGz" | "TarXz") {
                            format!("已选 {} / 31 个来源；总大小未知。仅添加到当前目录。新增项使用上方说明的规范化元数据，不加密。", draft.added.len())
                        } else {
                            format!("已选 {} / 31 个来源；总大小未知。仅添加到当前目录。新增文件使用 Deflate，不加密；原有条目的压缩与加密记录保持不变。", draft.added.len())
                        })
                        .child(
                            div()
                                .flex()
                                .gap_2()
                                .flex_wrap()
                                .child(self.paste_button(cx))
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
                                .gap_2()
                                .when(draft.added.is_empty(), |list| {
                                    list.child(div().id("archive-edit-sources-empty")
                                        .debug_selector(|| "archive-edit-sources-empty".into())
                                        .child("尚未选择来源。请添加文件或文件夹。"))
                                })
                                .children(draft.added.iter().enumerate().map(|(index, path)| {
                                    div().flex().items_center().gap_2().min_w_0()
                                        .child(div().flex_1().min_w_0().child(
                                            gpui_kit::component::label::Label::new(path.clone()).whitespace_normal()))
                                        .child(Button::new(("archive-edit-remove", index))
                                            .debug_selector(move || format!("archive-edit-remove-{index}"))
                                            .label("移除").outline().with_size(Size::Small)
                                            .disabled(self.busy())
                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                if this.busy() { return; }
                                                if let Draft::Modify(draft) = &mut this.draft
                                                    && draft.action == ArchiveJobModifyAction::Add
                                                    && index < draft.added.len()
                                                {
                                                    draft.added.remove(index);
                                                }
                                                this.revalidate(cx);
                                            })))
                                })),
                        ),
                );
            }
            ArchiveJobModifyAction::ReplaceContent => {
                form = form.child(Field::new()
                    .label("替换选中文件的内容")
                    .description("仅一个原始文件条目；保留原条目名称、时间与属性。请选择一个普通文件，不能添加条目或替换目录；原生提交前再次保护并验证来源。")
                    .child(self.picker_button("archive-edit-replacement-file", "选择替换文件…", ArchivePickerPurpose::Files, cx))
                    .child(div().children(draft.added.iter().cloned())));
            }
            ArchiveJobModifyAction::Delete => {
                form = form.child(Field::new().label("删除选中条目").description(format!(
                    "从压缩包删除 {} 个原始条目（所选目录后代已去重）。删除全部会生成经校验的合法空压缩包，不会删除压缩包文件；取消不提交。原文件在提交前保持不变。",
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
                        .description("相对压缩包路径；文件夹以 / 结尾。非法路径、目标占用或移入自身会拒绝，不会先删除冲突成员。")
                        .child(
                            Input::new(&self.name)
                                .aria_label("编辑压缩包目标名称")
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
        let reclamation = matches!(&self.draft, Draft::Extract(draft) if draft.reclamation);
        let continuation =
            matches!(&self.draft, Draft::Extract(draft) if draft.continue_reclamation);
        let form = if create {
            self.create_fields(cx)
        } else if modify {
            self.modify_fields(cx).into_any_element()
        } else if continuation {
            self.continuation_fields(cx)
        } else {
            self.extract_fields(cx)
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
            .capture_key_down(cx.listener(Self::file_paste_key))
            .capture_key_down(cx.listener(Self::default_enter))
            .capture_key_up(cx.listener(|this, event: &gpui_kit::KeyUpEvent, _, _| {
                if event.keystroke.key == "enter" {
                    this.default_enter_held = false;
                }
            }))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                if event.keystroke.key == "escape"
                    && this.pending_format.is_none()
                    && !this.reclamation_confirmation_pending
                {
                    this.close(window, cx);
                    cx.stop_propagation();
                }
            }))
            .child(crate::ui::components::titlebar::controlled_titlebar(
                "archive-form-titlebar",
                "archive-form-close",
                if create {
                    "新建压缩包"
                } else if modify {
                    if matches!(&self.draft, Draft::Modify(draft) if draft.browser.format == "SevenZ") { "编辑 7z" } else if matches!(&self.draft, Draft::Modify(draft) if matches!(draft.browser.format.as_str(), "Tar" | "TarGz" | "TarXz")) { "编辑 TAR" } else { "编辑 ZIP" }
                } else if continuation {
                    "继续空间回收解压"
                } else {
                    "解压压缩包"
                },
                crate::ui::components::titlebar::CaptionCapabilities::RESIZABLE,
                cx.listener(|this, _, window, cx| this.close(window, cx)),
            ))
            .child(
                div()
                    .id("archive-form-viewport")
                    .debug_selector(|| "archive-form-body".into())
                    .flex_1()
                    .min_h_0()
                    .min_w_0()
                    .overflow_hidden()
                    .child(div().id("archive-form-body").size_full()
                        .overflow_y_scrollbar()
                        .child(div().p_4().child(form))),
            )
            .child(
                div()
                    .id("archive-form-footer")
                    .debug_selector(|| "archive-form-footer".into())
                    .flex_none()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .p_4()
                    .when_some(
                        self.error.clone(),
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
                    .when_some(self.validation.clone().filter(|_| self.error.is_none()).filter(|_| !continuation).filter(|_| match &self.draft {
                        Draft::Create(draft) => !draft.sources.is_empty(),
                        Draft::Extract(draft) => !draft.source.as_os_str().is_empty(),
                        Draft::Modify(_) => true,
                    }), |footer, message| footer.child(div().text_sm().text_color(cx.theme().muted_foreground).child(message)))
                    .when(self.enumerating, |footer| {
                        footer.child("正在读取来源信息")
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
                                    } else if continuation {
                                        "继续空间回收解压"
                                    } else if reclamation {
                                        "确认并开始"
                                    } else {
                                        "解压"
                                    })
                                    .when(reclamation, |button| button.outline())
                                    .when(!reclamation, |button| button.primary())
                                    .with_size(Size::Medium)
                                    .debug_selector(|| "archive-form-submit".into())
                                    .disabled(
                                        self.busy()
                                            || self.validation.is_some()
                                            || !self.recovery_submit_ready(),
                                    )
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
    if !intake::valid_source_roots(paths.iter().map(String::as_str), 32, None) {
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
