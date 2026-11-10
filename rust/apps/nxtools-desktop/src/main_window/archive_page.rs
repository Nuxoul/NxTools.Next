//! Owned archive settings and real browser/create/extract entry points.
use super::*;
use crate::archive::shell_integration::IntegrationOutcome;
use gpui_kit::component::Size;
use gpui_kit::component::alert::Alert;
use gpui_kit::component::checkbox::Checkbox;
use gpui_kit::component::group_box::GroupBoxVariant;
use gpui_kit::component::input::{Input, InputEvent, InputState, Textarea, TextareaState};
use gpui_kit::component::setting::{
    SettingField, SettingGroup, SettingItem, SettingPage, Settings,
};
use gpui_kit::component::switch::Switch;
use gpui_kit::{SharedString, WeakEntity};
use nxtools_core::AppConfig;
use nxtools_core::archive::*;
use nxtools_protocol::archive_shell_integration::{
    ArchiveShellIntegrationAction, ArchiveShellIntegrationState,
};
use std::time::Instant;
#[path = "archive_preview_settings.rs"]
mod preview_settings;
#[path = "archive_temp_settings.rs"]
mod temp_settings;

pub(super) struct ArchiveSettingsView {
    pub(super) settings: ArchiveSettings,
    config_path: PathBuf,
    pub(super) exclusions: Vec<Entity<TextareaState>>,
    exclusion_baseline: Vec<String>,
    exclusion_conflict: bool,
    window: AnyWindowHandle,
    pub(super) error: Option<String>,
    /// Last correlated native observation; never derived from the saved desired intent.
    pub(super) integration: Option<IntegrationOutcome>,
    pub(super) integration_pending: bool,
    default_apps_pending: bool,
    default_apps_error: Option<String>,
    recent_clear_pending: bool,
    _recent_subscription: Option<gpui_kit::Subscription>,
    pub(super) temp_path: Entity<InputState>,
    temp_baseline: String,
    temp_revision: Rc<std::cell::Cell<u64>>,
    temp_picker_pending: bool,
    _temp_subscription: gpui_kit::Subscription,
}
impl ArchiveSettingsView {
    pub(super) fn new(
        settings: ArchiveSettings,
        config_path: PathBuf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let (settings, error) = match AppConfig::load_from(&config_path) {
            Ok(config) => (config.archive, None),
            Err(_) => (
                settings,
                Some("无法读取压缩设置；保留上次值，请检查配置文件后重试。".into()),
            ),
        };
        let exclusions = settings
            .exclude_patterns
            .iter()
            .map(|pattern| {
                cx.new(|cx| {
                    let mut input = TextareaState::new(window, cx);
                    input.set_value(pattern.clone(), window, cx);
                    input
                })
            })
            .collect();
        let recent_subscription = crate::archive::recent_owner(cx)
            .map(|owner| cx.observe(&owner, |_, _, cx| cx.notify()));
        let temp_path = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(settings.temp.base_directory.clone())
                .placeholder("默认：NxTools 缓存目录")
        });
        let temp_revision = Rc::new(std::cell::Cell::new(0u64));
        let revision = temp_revision.clone();
        let temp_subscription = cx.subscribe(&temp_path, move |_, _, event: &InputEvent, _| {
            if matches!(event, InputEvent::Change) {
                revision.set(revision.get().wrapping_add(1));
            }
        });
        Self {
            exclusion_baseline: settings.exclude_patterns.clone(),
            exclusion_conflict: false,
            window: window.window_handle(),
            temp_baseline: settings.temp.base_directory.clone(),
            temp_path,
            temp_revision,
            temp_picker_pending: false,
            _temp_subscription: temp_subscription,
            settings,
            config_path,
            exclusions,
            error,
            integration: None,
            integration_pending: false,
            default_apps_pending: false,
            default_apps_error: None,
            recent_clear_pending: false,
            _recent_subscription: recent_subscription,
        }
    }
    fn save(&mut self, settings: ArchiveSettings, cx: &mut Context<Self>) {
        self.save_inner(settings, None, cx);
    }
    fn exclusion_draft(&self, cx: &App) -> Vec<String> {
        self.exclusions
            .iter()
            .map(|input| input.read(cx).value().to_string())
            .collect()
    }
    fn save_exclusions(&mut self, cx: &mut Context<Self>) {
        let draft = self.exclusion_draft(cx);
        self.save_inner(self.settings.clone(), Some(draft), cx);
    }
    fn reload_exclusions(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        match AppConfig::load_from(&self.config_path) {
            Ok(config) => {
                let patterns = config.archive.exclude_patterns;
                self.exclusions = patterns
                    .iter()
                    .map(|pattern| {
                        cx.new(|cx| {
                            let mut input = TextareaState::new(window, cx);
                            input.set_value(pattern.clone(), window, cx);
                            input
                        })
                    })
                    .collect();
                self.settings.exclude_patterns = patterns.clone();
                self.exclusion_baseline = patterns;
                self.exclusion_conflict = false;
                self.error = None;
            }
            Err(_) => {
                self.error = Some("无法载入最新规则；当前草稿未改变，请检查配置文件后重试。".into())
            }
        }
        cx.notify();
    }
    fn save_inner(
        &mut self,
        mut settings: ArchiveSettings,
        rules: Option<Vec<String>>,
        cx: &mut Context<Self>,
    ) {
        let draft_before = self.exclusion_draft(cx);
        let clean_before = draft_before == self.exclusion_baseline;
        let submit_rules = rules.filter(|draft| *draft != self.exclusion_baseline);
        if let Some(draft) = &submit_rules {
            settings.exclude_patterns = draft.clone();
        }
        if let Err(message) = settings.validate() {
            self.error = Some(message);
            cx.notify();
            return;
        }
        let correlation_id = uuid::Uuid::new_v4();
        let started = Instant::now();
        info!(event = "archive.settings.save_started", %correlation_id, "archive settings save accepted");
        let previous = &self.settings;
        let mut rule_conflict = false;
        match crate::archive::save_settings(&self.config_path, |current| {
            if let Some(draft) = &submit_rules
                && current.exclude_patterns != self.exclusion_baseline
                && current.exclude_patterns != *draft
            {
                rule_conflict = true;
                return;
            }
            macro_rules! merge_changed {
                ($($field:ident),+ $(,)?) => { $(if settings.$field != previous.$field {
                    current.$field = settings.$field;
                })+ };
            }
            merge_changed!(
                open_in_nxtools,
                remember_recent,
                recent_limit,
                auto_enter_single_root,
                show_parent_row,
                show_grid_lines,
                full_row_select,
                single_click_open,
                show_tree,
                show_full_path,
                auto_expand_tree,
                escape_closes_window,
                open_after_extract,
                keep_extract_progress_open,
                smart_single_file_folder,
                keep_create_progress_open,
                default_destination,
                conflict_policy,
                filename_encoding,
                create_format,
                compression_level,
                test_after_create,
                strip_single_root,
                copy_motw
            );
            current
                .preview
                .merge_changed(&previous.preview, &settings.preview);
            if settings.temp.retention_hours != previous.temp.retention_hours {
                current.temp.retention_hours = settings.temp.retention_hours;
            }
            if settings.temp.cleanup_on_child_exit != previous.temp.cleanup_on_child_exit {
                current.temp.cleanup_on_child_exit = settings.temp.cleanup_on_child_exit;
            }
            if let Some(draft) = &submit_rules {
                current.exclude_patterns = draft.clone();
            }
            if settings.shell_integration != previous.shell_integration {
                let revision = current.shell_integration.desired_revision.saturating_add(1);
                let previous_integration = &previous.shell_integration;
                let requested = &settings.shell_integration;
                let target = &mut current.shell_integration;
                if requested.classic_cascade != previous_integration.classic_cascade {
                    target.classic_cascade = requested.classic_cascade;
                }
                if requested.open_with != previous_integration.open_with {
                    target.open_with = requested.open_with;
                }
                if requested.extensions != previous_integration.extensions {
                    target.extensions = requested.extensions.clone();
                }
                if requested.double_click_action != previous_integration.double_click_action {
                    target.double_click_action = requested.double_click_action;
                }
                if requested.menu_layout != previous_integration.menu_layout {
                    target.menu_layout = requested.menu_layout;
                }
                if requested.menu_commands != previous_integration.menu_commands {
                    target.menu_commands = requested.menu_commands.clone();
                }
                target.desired_revision = revision;
            }
        }) {
            Ok(_) if rule_conflict => {
                self.exclusion_conflict = true;
                self.error = Some("排除规则已被其他操作更新；未覆盖新规则，当前草稿已保留。点击“载入最新规则”放弃本页规则草稿后重新编辑。".into());
                tracing::warn!(event = "archive.settings.save_finished", %correlation_id, outcome = "conflict", reason = "exclusion_rules_changed", elapsed_ms = started.elapsed().as_millis() as u64, "archive settings save finished");
            }
            Ok(saved) => {
                self.settings = saved.settings().clone();
                if submit_rules.is_some() {
                    self.exclusion_baseline = self.settings.exclude_patterns.clone();
                    self.exclusion_conflict = false;
                } else if clean_before && draft_before != self.settings.exclude_patterns {
                    let expected_baseline = self.exclusion_baseline.clone();
                    let patterns = self.settings.exclude_patterns.clone();
                    let owner = cx.entity().downgrade();
                    let source = self.window;
                    cx.defer(move |cx| {
                        let _ = source.update(cx, |_, window, cx| {
                            let _ = owner.update(cx, |view, cx| {
                                if view.exclusion_baseline == expected_baseline
                                    && view.exclusion_draft(cx) == expected_baseline
                                    && view.settings.exclude_patterns == patterns
                                {
                                    view.exclusions = patterns
                                        .iter()
                                        .map(|pattern| {
                                            cx.new(|cx| {
                                                let mut input = TextareaState::new(window, cx);
                                                input.set_value(pattern.clone(), window, cx);
                                                input
                                            })
                                        })
                                        .collect();
                                    view.exclusion_baseline = patterns;
                                    cx.notify();
                                }
                            });
                        });
                    });
                }
                crate::archive::saved_settings_updated(saved, cx);
                self.error = None;
                info!(event = "archive.settings.save_finished", %correlation_id, outcome = "saved", elapsed_ms = started.elapsed().as_millis() as u64, "archive settings save finished");
            }
            Err(_) => {
                self.error = Some(
                    "设置未保存，仍保留上次生效值。请检查配置文件是否完整、可写，然后重试。".into(),
                );
                tracing::warn!(event = "archive.settings.save_finished", %correlation_id, outcome = "failed", reason = "config_update_failed", elapsed_ms = started.elapsed().as_millis() as u64, "archive settings save finished");
            }
        }
        cx.notify();
    }
    fn switch(
        &self,
        owner: WeakEntity<Self>,
        id: &'static str,
        label: &'static str,
        value: bool,
        apply: fn(&mut ArchiveSettings, bool),
    ) -> SettingItem {
        SettingItem::render(move |options, _, _| {
            let owner = owner.clone();
            let control = div().debug_selector(move || id.into()).child(
                Switch::new(id)
                    .checked(value)
                    .disabled(options.is_disabled())
                    .accessibility_label(label)
                    .on_change(move |requested, _, cx| {
                        let requested = *requested;
                        let owner = owner.clone();
                        cx.defer(move |cx| {
                            let _ = owner.update(cx, |view, cx| {
                                let mut settings = view.settings.clone();
                                apply(&mut settings, requested);
                                view.save(settings, cx);
                            });
                        });
                    }),
            );
            crate::ui::components::settings::aligned_setting_row(
                format!("archive-setting-{label}"),
                label,
                "",
                format!("archive-setting-{label}-help"),
                crate::ui::metrics::ordinary_rem(440.),
                control.into_any_element(),
            )
        })
        .keywords([label])
    }
    fn choice<T: Copy + PartialEq + 'static>(
        &self,
        owner: WeakEntity<Self>,
        label: &'static str,
        current: T,
        options: &'static [(T, &'static str)],
        apply: fn(&mut ArchiveSettings, T),
    ) -> SettingItem {
        let value: SharedString = options
            .iter()
            .find(|(value, _)| *value == current)
            .expect("released settings choice")
            .1
            .into();
        let choices = options
            .iter()
            .map(|(_, label)| (SharedString::from(*label), SharedString::from(*label)))
            .collect();
        SettingItem::new(
            label,
            SettingField::scrollable_dropdown(
                choices,
                move |_| value.clone(),
                move |requested, cx| {
                    let Some((requested, _)) = options
                        .iter()
                        .find(|(_, label)| *label == requested.as_ref())
                    else {
                        return;
                    };
                    let requested = *requested;
                    let owner = owner.clone();
                    cx.defer(move |cx| {
                        let _ = owner.update(cx, |view, cx| {
                            let mut settings = view.settings.clone();
                            apply(&mut settings, requested);
                            view.save(settings, cx);
                        });
                    });
                },
            )
            .w(crate::ui::metrics::ordinary_rem(200.)),
        )
    }
    pub(super) fn add_exclusion(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.exclusions.len() < 32 {
            self.exclusions
                .push(cx.new(|cx| TextareaState::new(window, cx)));
            cx.notify();
        }
    }
    fn exclusions_item(&self, owner: WeakEntity<Self>) -> SettingItem {
        let conflict = self.exclusion_conflict;
        let inputs = self.exclusions.clone();
        SettingItem::new(
            "排除规则",
            SettingField::render(move |_, _, _| {
                let mut rules = div().w_full().flex().flex_col().gap_2();
                for (index, input) in inputs.iter().enumerate() {
                    let owner = owner.clone();
                    let selector = SharedString::from(format!("archive-exclusion-remove-{index}"));
                    rules = rules.child(
                        div()
                            .flex()
                            .items_start()
                            .gap_2()
                            .child(
                                div().min_w_0().flex_1().child(
                                    Textarea::new(input)
                                        .aria_label(format!("排除规则 {}", index + 1))
                                        .h(px(48.)),
                                ),
                            )
                            .child(
                                Button::new(selector.clone())
                                    .debug_selector(move || selector.to_string())
                                    .label("删除")
                                    .outline()
                                    .with_size(Size::Small)
                                    .on_click(move |_, _, cx| {
                                        let owner = owner.clone();
                                        cx.defer(move |cx| {
                                            let _ = owner.update(cx, |view, cx| {
                                                if index < view.exclusions.len() {
                                                    view.exclusions.remove(index);
                                                }
                                                cx.notify();
                                            });
                                        });
                                    }),
                            ),
                    );
                }
                let add_owner = owner.clone();
                let save_owner = owner.clone();
                let reload_owner = owner.clone();
                rules
                    .child("每项是一条文件名或相对路径规则，支持 * 和 ?；增删后请保存。")
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .child(
                                Button::new("archive-exclusions-add")
                                    .debug_selector(|| "archive-exclusions-add".into())
                                    .label("增加规则")
                                    .outline()
                                    .with_size(Size::Small)
                                    .disabled(inputs.len() >= 32)
                                    .on_click(move |_, window, cx| {
                                        let owner = add_owner.clone();
                                        let source = window.window_handle();
                                        cx.defer(move |cx| {
                                            let _ = source.update(cx, |_, window, cx| {
                                                let _ = owner.update(cx, |view, cx| {
                                                    view.add_exclusion(window, cx)
                                                });
                                            });
                                        });
                                    }),
                            )
                            .child(
                                Button::new("archive-exclusions-save")
                                    .debug_selector(|| "archive-exclusions-save".into())
                                    .label("保存排除规则")
                                    .outline()
                                    .with_size(Size::Small)
                                    .on_click(move |_, _, cx| {
                                        let owner = save_owner.clone();
                                        cx.defer(move |cx| {
                                            let _ = owner.update(cx, |view, cx| {
                                                view.save_exclusions(cx);
                                            });
                                        });
                                    }),
                            )
                            .when(conflict, |actions| {
                                actions.child(
                                    Button::new("archive-exclusions-reload")
                                        .debug_selector(|| "archive-exclusions-reload".into())
                                        .label("载入最新规则")
                                        .tooltip("放弃本页未保存的排除规则草稿，载入已保存的新规则")
                                        .outline()
                                        .with_size(Size::Small)
                                        .on_click(move |_, window, cx| {
                                            let owner = reload_owner.clone();
                                            let source = window.window_handle();
                                            cx.defer(move |cx| {
                                                let _ = source.update(cx, |_, window, cx| {
                                                    let _ = owner.update(cx, |view, cx| {
                                                        view.reload_exclusions(window, cx)
                                                    });
                                                });
                                            });
                                        }),
                                )
                            }),
                    )
            }),
        )
        .layout(gpui_kit::Axis::Vertical)
        .keywords(["排除", "glob", "Thumbs.db", "desktop.ini"])
    }
}
impl ArchiveSettingsView {
    /// Saves a new desired intent revision. This never applies or reverts anything.
    fn save_integration(
        &mut self,
        update: impl FnOnce(&mut ArchiveShellIntegrationSettings),
        cx: &mut Context<Self>,
    ) {
        let mut settings = self.settings.clone();
        update(&mut settings.shell_integration);
        settings.shell_integration.desired_revision = self
            .settings
            .shell_integration
            .desired_revision
            .saturating_add(1);
        self.save(settings, cx);
    }
    pub(super) fn request_integration(
        &mut self,
        action: ArchiveShellIntegrationAction,
        cx: &mut Context<Self>,
    ) {
        if self.integration_pending {
            return;
        }
        self.integration_pending = true;
        let owner = cx.entity().downgrade();
        crate::archive::shell_integration::request(
            action,
            self.settings.shell_integration.clone(),
            move |outcome, cx| {
                let _ = owner.update(cx, |view, cx| {
                    view.integration_pending = false;
                    view.integration = Some(outcome);
                    cx.notify();
                });
            },
            cx,
        );
        cx.notify();
    }
    fn request_default_apps(&mut self, cx: &mut Context<Self>) {
        if self.default_apps_pending {
            return;
        }
        self.default_apps_pending = true;
        self.default_apps_error = None;
        let owner = cx.entity().downgrade();
        crate::archive::shell_integration::open_default_apps(
            move |outcome, cx| {
                let _ = owner.update(cx, |view, cx| {
                    view.default_apps_pending = false;
                    view.default_apps_error = match outcome {
                        IntegrationOutcome::DefaultAppsOpened => None,
                        _ => Some(
                            "无法打开 Windows 默认应用设置；未更改任何关联，请连接原生组件后重试。"
                                .into(),
                        ),
                    };
                    cx.notify();
                });
            },
            cx,
        );
        cx.notify();
    }
    fn integration_choice<T: Copy + PartialEq + 'static>(
        &self,
        owner: WeakEntity<Self>,
        label: &'static str,
        current: T,
        options: &'static [(T, &'static str)],
        apply: fn(&mut ArchiveShellIntegrationSettings, T),
    ) -> SettingItem {
        let selected: SharedString = options
            .iter()
            .find(|(value, _)| *value == current)
            .expect("supported integration choice")
            .1
            .into();
        SettingItem::new(
            label,
            SettingField::scrollable_dropdown(
                options
                    .iter()
                    .map(|(_, label)| (SharedString::from(*label), SharedString::from(*label)))
                    .collect(),
                move |_| selected.clone(),
                move |requested, cx| {
                    let Some((value, _)) = options
                        .iter()
                        .find(|(_, label)| *label == requested.as_ref())
                    else {
                        return;
                    };
                    let value = *value;
                    let owner = owner.clone();
                    cx.defer(move |cx| {
                        let _ = owner.update(cx, |view, cx| {
                            view.save_integration(|settings| apply(settings, value), cx)
                        });
                    });
                },
            )
            .w(crate::ui::metrics::ordinary_rem(200.)),
        )
    }
    fn menu_commands_item(&self, owner: WeakEntity<Self>) -> SettingItem {
        let selected = self.settings.shell_integration.menu_commands.clone();
        SettingItem::new(
            "菜单命令",
            SettingField::render(move |_, _, _| {
                let mut controls = div().w_full().flex().flex_col().gap_2();
                for command in ArchiveMenuCommand::ALL {
                    let owner = owner.clone();
                    let id =
                        SharedString::from(format!("archive-menu-command-{}", command.wire_name()));
                    controls = controls.child(
                        Checkbox::new(id.clone())
                            .debug_selector(move || id.to_string())
                            .label(match command {
                                ArchiveMenuCommand::Create => "新建压缩包…（普通文件或文件夹）",
                                _ => command.label(),
                            })
                            .checked(selected.contains(&command))
                            .on_change(move |requested, _, cx| {
                                let requested = *requested;
                                let owner = owner.clone();
                                cx.defer(move |cx| {
                                    let _ = owner.update(cx, |view, cx| {
                                        view.save_integration(
                                            |settings| {
                                                settings
                                                    .menu_commands
                                                    .retain(|value| *value != command);
                                                if requested {
                                                    settings.menu_commands.push(command);
                                                }
                                                settings.menu_commands.sort_by_key(|value| {
                                                    ArchiveMenuCommand::ALL
                                                        .iter()
                                                        .position(|candidate| candidate == value)
                                                });
                                            },
                                            cx,
                                        )
                                    });
                                });
                            }),
                    );
                }
                let all_owner = owner.clone();
                let none_owner = owner.clone();
                controls.child(
                    div()
                        .flex()
                        .gap_2()
                        .child(
                            Button::new("archive-menu-all")
                                .debug_selector(|| "archive-menu-all".into())
                                .label("全选命令")
                                .outline()
                                .with_size(Size::Small)
                                .on_click(move |_, _, cx| {
                                    let owner = all_owner.clone();
                                    cx.defer(move |cx| {
                                        let _ = owner.update(cx, |view, cx| {
                                            view.save_integration(
                                                |settings| {
                                                    settings.menu_commands =
                                                        ArchiveMenuCommand::ALL.to_vec()
                                                },
                                                cx,
                                            )
                                        });
                                    });
                                }),
                        )
                        .child(
                            Button::new("archive-menu-none")
                                .debug_selector(|| "archive-menu-none".into())
                                .label("取消全选")
                                .outline()
                                .with_size(Size::Small)
                                .on_click(move |_, _, cx| {
                                    let owner = none_owner.clone();
                                    cx.defer(move |cx| {
                                        let _ = owner.update(cx, |view, cx| {
                                            view.save_integration(
                                                |settings| settings.menu_commands.clear(),
                                                cx,
                                            )
                                        });
                                    });
                                }),
                        ),
                )
            }),
        )
        .layout(gpui_kit::Axis::Vertical)
    }
    fn default_apps_item(&self, owner: WeakEntity<Self>) -> SettingItem {
        let pending = self.default_apps_pending;
        let error = self.default_apps_error.clone();
        SettingItem::new(
            "默认应用",
            SettingField::render(move |_, _, _| {
                let owner = owner.clone();
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        Button::new("archive-default-apps")
                            .debug_selector(|| "archive-default-apps".into())
                            .label("Windows 默认应用设置")
                            .outline()
                            .loading(pending)
                            .disabled(pending)
                            .on_click(move |_, _, cx| {
                                let owner = owner.clone();
                                cx.defer(move |cx| {
                                    let _ =
                                        owner.update(cx, |view, cx| view.request_default_apps(cx));
                                });
                            }),
                    )
                    .when_some(error.clone(), |row, error| {
                        row.child(
                            div()
                                .debug_selector(|| "archive-default-apps-error".into())
                                .child(Alert::error("archive-default-apps-failed", error).small()),
                        )
                    })
            }),
        )
        .layout(gpui_kit::Axis::Vertical)
    }
    fn integration_items(&self, owner: WeakEntity<Self>) -> Vec<SettingItem> {
        let desired = self.settings.shell_integration.clone();
        let pending = self.integration_pending;
        let flag = |id: &'static str,
                    label: &'static str,
                    value: bool,
                    apply: fn(&mut ArchiveShellIntegrationSettings, bool)| {
            let owner = owner.clone();
            SettingItem::new(
                label,
                SettingField::render(move |_, _, _| {
                    let owner = owner.clone();
                    div().debug_selector(move || id.into()).child(
                        Switch::new(id)
                            .checked(value)
                            .accessibility_label(label)
                            .on_change(move |requested, _, cx| {
                                let requested = *requested;
                                let owner = owner.clone();
                                cx.defer(move |cx| {
                                    let _ = owner.update(cx, |view, cx| {
                                        view.save_integration(|s| apply(s, requested), cx)
                                    });
                                });
                            }),
                    )
                }),
            )
        };
        let extensions_owner = owner.clone();
        let selected = desired.extensions.clone();
        let extensions = SettingItem::new(
            "扩展名",
            SettingField::render(move |_, _, _| {
                let list = ARCHIVE_SHELL_EXTENSIONS.iter().fold(
                    div().flex().flex_wrap().gap_3(),
                    |row, extension| {
                        let owner = extensions_owner.clone();
                        let id = SharedString::from(format!("archive-shell-ext-{extension}"));
                        row.child(
                            Checkbox::new(id.clone())
                                .label(*extension)
                                .checked(selected.iter().any(|value| value == extension))
                                .on_change(move |requested, _, cx| {
                                    let requested = *requested;
                                    let owner = owner.clone();
                                    cx.defer(move |cx| {
                                        let _ = owner.update(cx, |view, cx| {
                                            view.save_integration(
                                                |s| {
                                                    s.extensions.retain(|value| value != extension);
                                                    if requested {
                                                        s.extensions.push((*extension).to_owned());
                                                        s.extensions.sort_by_key(|value| {
                                                            ARCHIVE_SHELL_EXTENSIONS
                                                                .iter()
                                                                .position(|known| known == value)
                                                        });
                                                    }
                                                },
                                                cx,
                                            )
                                        });
                                    });
                                }),
                        )
                    },
                );
                let all_owner = extensions_owner.clone();
                let none_owner = extensions_owner.clone();
                div().flex().flex_col().gap_2().child(list).child(
                    div()
                        .flex()
                        .gap_2()
                        .child(
                            Button::new("archive-shell-select-all")
                                .debug_selector(|| "archive-shell-select-all".into())
                                .label("全选")
                                .outline()
                                .with_size(Size::Small)
                                .on_click(move |_, _, cx| {
                                    let owner = all_owner.clone();
                                    cx.defer(move |cx| {
                                        let _ = owner.update(cx, |view, cx| {
                                            view.save_integration(
                                                |s| {
                                                    s.extensions = ARCHIVE_SHELL_EXTENSIONS
                                                        .iter()
                                                        .map(|extension| (*extension).to_owned())
                                                        .collect();
                                                },
                                                cx,
                                            )
                                        });
                                    });
                                }),
                        )
                        .child(
                            Button::new("archive-shell-select-none")
                                .debug_selector(|| "archive-shell-select-none".into())
                                .label("取消全选")
                                .outline()
                                .with_size(Size::Small)
                                .on_click(move |_, _, cx| {
                                    let owner = none_owner.clone();
                                    cx.defer(move |cx| {
                                        let _ = owner.update(cx, |view, cx| {
                                            view.save_integration(|s| s.extensions.clear(), cx)
                                        });
                                    });
                                }),
                        ),
                )
            }),
        )
        .layout(gpui_kit::Axis::Vertical)
        .keywords(["扩展名", "zip", "7z", "rar"]);
        let observed: SharedString = match &self.integration {
            Some(IntegrationOutcome::DefaultAppsOpened) => {
                "Windows 默认应用设置已打开；关联状态未改变。".into()
            }
            None => "尚未查询。保存的选择只是期望状态，不代表已写入 Windows。".into(),
            Some(IntegrationOutcome::NotDelivered(reason)) => format!(
                "请求未取得有效原生结果（{reason}）；Windows 应用状态未确认，请刷新状态后重试。"
            )
            .into(),
            Some(IntegrationOutcome::Observed(result)) => match result.state {
                ArchiveShellIntegrationState::Disabled => {
                    "当前版本未授权写入注册表：应用被拒绝，未写入、未修改任何 Windows 设置。".into()
                }
                ArchiveShellIntegrationState::NotApplied => {
                    "当前未应用（无 NxTools 写入记录）。".into()
                }
                ArchiveShellIntegrationState::Applied
                    if result.applied_revision != Some(desired.desired_revision) =>
                {
                    format!(
                        "已保存版本 {}，尚未应用；Windows 仍使用版本 {}。",
                        desired.desired_revision,
                        result.applied_revision.unwrap_or_default()
                    )
                    .into()
                }
                ArchiveShellIntegrationState::Applied => format!(
                    "已应用版本 {}，共 {} 项。",
                    result.applied_revision.unwrap_or_default(),
                    result.applied_values
                )
                .into(),
                ArchiveShellIntegrationState::Reverted => format!(
                    "已撤销：移除 {} 项，恢复 {} 项，保留被修改的 {} 项。",
                    result.removed, result.restored, result.preserved
                )
                .into(),
                ArchiveShellIntegrationState::RevertRequired => {
                    "已有已应用版本；请先撤销，再应用新的选择。".into()
                }
                ArchiveShellIntegrationState::Failed => format!(
                    "操作失败：{}。应用状态未确认，请刷新状态后重试。",
                    result.failure.as_deref().unwrap_or("unknown")
                )
                .into(),
            },
        };
        let actions_owner = owner.clone();
        let actions = SettingItem::new(
            "应用状态",
            SettingField::render(move |_, _, _| {
                let apply_owner = actions_owner.clone();
                let revert_owner = actions_owner.clone();
                let status_owner = actions_owner.clone();
                div()
                    .w_full()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        div()
                            .debug_selector(|| "archive-shell-observed".into())
                            .text_sm()
                            .child(observed.clone()),
                    )
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .child(
                                Button::new("archive-shell-apply")
                                    .debug_selector(|| "archive-shell-apply".into())
                                    .label("应用")
                                    .outline()
                                    .loading(pending)
                                    .disabled(pending)
                                    .on_click(move |_, _, cx| {
                                        let owner = apply_owner.clone();
                                        cx.defer(move |cx| {
                                            let _ = owner.update(cx, |view, cx| {
                                                view.request_integration(
                                                    ArchiveShellIntegrationAction::Apply,
                                                    cx,
                                                )
                                            });
                                        });
                                    }),
                            )
                            .child(
                                Button::new("archive-shell-revert")
                                    .debug_selector(|| "archive-shell-revert".into())
                                    .label("撤销")
                                    .outline()
                                    .disabled(pending)
                                    .on_click(move |_, _, cx| {
                                        let owner = revert_owner.clone();
                                        cx.defer(move |cx| {
                                            let _ = owner.update(cx, |view, cx| {
                                                view.request_integration(
                                                    ArchiveShellIntegrationAction::Revert,
                                                    cx,
                                                )
                                            });
                                        });
                                    }),
                            )
                            .child(
                                Button::new("archive-shell-status")
                                    .debug_selector(|| "archive-shell-status".into())
                                    .label("刷新状态")
                                    .outline()
                                    .disabled(pending)
                                    .on_click(move |_, _, cx| {
                                        let owner = status_owner.clone();
                                        cx.defer(move |cx| {
                                            let _ = owner.update(cx, |view, cx| {
                                                view.request_integration(
                                                    ArchiveShellIntegrationAction::Status,
                                                    cx,
                                                )
                                            });
                                        });
                                    }),
                            ),
                    )
            }),
        )
        .layout(gpui_kit::Axis::Vertical);
        vec![
            note(
                "范围",
                "仅影响当前用户的经典右键菜单与“打开方式”列表，不更改默认应用。",
            ),
            flag(
                "archive-shell-cascade",
                "资源管理器右键菜单",
                desired.classic_cascade,
                |s, value| s.classic_cascade = value,
            ),
            flag(
                "archive-shell-open-with",
                "加入“打开方式”",
                desired.open_with,
                |s, value| s.open_with = value,
            ),
            extensions,
            self.default_apps_item(owner.clone()),
            self.integration_choice(
                owner.clone(),
                "双击动作",
                desired.double_click_action,
                &[
                    (ArchiveDoubleClickAction::Open, "在 NxTools 中打开"),
                    (ArchiveDoubleClickAction::Smart, "智能解压（确认后开始）"),
                    (
                        ArchiveDoubleClickAction::Name,
                        "解压到同名文件夹（确认后开始）",
                    ),
                ],
                |settings, value| settings.double_click_action = value,
            ),
            self.integration_choice(
                owner.clone(),
                "经典菜单组织",
                desired.menu_layout,
                &[
                    (ArchiveMenuLayout::Cascade, "级联到 NxTools 子菜单"),
                    (ArchiveMenuLayout::Flat, "平铺经典菜单命令"),
                ],
                |settings, value| settings.menu_layout = value,
            ),
            self.menu_commands_item(owner.clone()),
            note(
                "保存与应用",
                "选择会自动保存；点击“应用”后，以实际返回的 Windows 状态为准。",
            ),
            actions,
        ]
    }
    fn recent_items(&self, owner: WeakEntity<Self>) -> Vec<SettingItem> {
        let limit = self.settings.recent_limit;
        let number_owner = owner.clone();
        let clear_owner = owner.clone();
        let pending = self.recent_clear_pending;
        vec![
            self.switch(owner.clone(), "archive-remember-recent", "记录最近打开的压缩包", self.settings.remember_recent, |settings, value| settings.remember_recent = value),
            SettingItem::new("最近记录数量", SettingField::number_input(
                gpui_kit::component::setting::NumberFieldOptions { min: 0., max: 30., step: 1. },
                move |_| f64::from(limit),
                move |value, cx| {
                    if !value.is_finite() || value.fract() != 0. || !(0. ..=30.).contains(&value) { return; }
                    let owner = number_owner.clone();
                    cx.defer(move |cx| { let _ = owner.update(cx, |view, cx| { let mut settings = view.settings.clone(); settings.recent_limit = value as u8; view.save(settings, cx); }); });
                },
            ).w(crate::ui::metrics::ordinary_rem(200.))),
            SettingItem::new("最近压缩包", SettingField::render(move |_, _, cx| {
                let mut content = div().w_full().flex().flex_col().gap_2();
                match crate::archive::recent_snapshot(cx) {
                    Ok(paths) if paths.is_empty() => { content = content.child("没有历史记录"); }
                    Ok(paths) => {
                        for (index, path) in paths.into_iter().enumerate() {
                            let label = path.file_name().unwrap_or_default().to_string_lossy().into_owned();
                            let full_path = path.to_string_lossy().into_owned();
                            content = content.child(gpui_kit::component::list::ListItem::new(("archive-recent-row", index))
                                .child(div().id(("archive-recent-path", index)).flex_1().min_w_0().text_ellipsis().child(label).tooltip(move |window, cx| gpui_kit::component::tooltip::Tooltip::new(full_path.clone()).build(window, cx)))
                                .suffix(move |_, _| { let path = path.clone(); Button::new(("archive-recent-open", index)).debug_selector(move || format!("archive-recent-open-{index}")).label("打开").outline().with_size(Size::Small).on_click(move |_, window, cx| crate::archive::open_recent(path.clone(), window.window_handle(), cx)) }));
                        }
                    }
                    Err("loading") => { content = content.child("正在载入最近记录…"); }
                    Err(_) => { content = content.child(Alert::error("archive-recent-unavailable", "无法读取最近记录；原文件已保留。").small()); }
                }
                let owner = clear_owner.clone();
                content.child(Button::new("archive-recent-clear").debug_selector(|| "archive-recent-clear".into()).label("清除历史记录").outline().with_size(Size::Small).loading(pending).disabled(pending).on_click(move |_, _, cx| {
                    let owner = owner.clone();
                    cx.defer(move |cx| { let _ = owner.update(cx, |view, cx| {
                        view.recent_clear_pending = true; cx.notify();
                        let owner = cx.entity().downgrade();
                        crate::archive::clear_recent(move |result, cx| { let _ = owner.update(cx, |view, cx| {
                            view.recent_clear_pending = false;
                            view.error = result.err().map(|_| "无法保存清除结果；请检查最近记录存储后重试。".into());
                            cx.notify();
                        }); }, cx);
                    }); });
                }))
            })).layout(gpui_kit::Axis::Vertical),
        ]
    }
}
fn note(label: &'static str, text: &'static str) -> SettingItem {
    SettingItem::new(
        label,
        SettingField::render(move |_, _, _| div().min_w_0().w_full().text_sm().child(text)),
    )
    .layout(gpui_kit::Axis::Vertical)
}
impl Render for ArchiveSettingsView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let owner = cx.entity().downgrade();
        let settings = &self.settings;
        let extract = vec![
            self.choice(
                owner.clone(),
                "默认目标策略",
                settings.default_destination,
                &[
                    (ArchiveDestination::Ask, "询问"),
                    (ArchiveDestination::Smart, "智能解压"),
                    (ArchiveDestination::Here, "当前文件夹"),
                    (ArchiveDestination::Name, "压缩包同名文件夹"),
                ],
                |settings, value| settings.default_destination = value,
            ),
            self.choice(
                owner.clone(),
                "同名文件处理",
                settings.conflict_policy,
                &[
                    (ArchiveConflictPolicy::Ask, "询问"),
                    (ArchiveConflictPolicy::Overwrite, "覆盖"),
                    (ArchiveConflictPolicy::Skip, "跳过"),
                    (ArchiveConflictPolicy::AutoRename, "自动重命名"),
                ],
                |settings, value| settings.conflict_policy = value,
            ),
            self.choice(
                owner.clone(),
                "默认代码页",
                settings.filename_encoding,
                &[
                    (ArchiveFilenameEncoding::Auto, "自动检测"),
                    (ArchiveFilenameEncoding::Utf8, "UTF-8"),
                    (ArchiveFilenameEncoding::Gbk, "简体中文 GBK (936)"),
                    (ArchiveFilenameEncoding::Big5, "繁体中文 Big5 (950)"),
                    (ArchiveFilenameEncoding::ShiftJis, "日文 Shift-JIS (932)"),
                    (ArchiveFilenameEncoding::Korean, "韩文 (949)"),
                    (ArchiveFilenameEncoding::Cp437, "CP437"),
                    (ArchiveFilenameEncoding::Windows1250, "Windows-1250"),
                    (ArchiveFilenameEncoding::Windows1251, "Windows-1251"),
                    (ArchiveFilenameEncoding::Windows1252, "Windows-1252"),
                ],
                |settings, value| settings.filename_encoding = value,
            ),
            self.switch(
                owner.clone(),
                "archive-auto-enter-single-root",
                "自动进入唯一根文件夹",
                settings.auto_enter_single_root,
                |settings, value| settings.auto_enter_single_root = value,
            ),
            self.switch(
                owner.clone(),
                "archive-open-after-extract",
                "解压后打开目标文件夹",
                settings.open_after_extract,
                |settings, value| settings.open_after_extract = value,
            ),
            self.switch(
                owner.clone(),
                "archive-keep-extract-progress-open",
                "解压完成后保留进度窗口",
                settings.keep_extract_progress_open,
                |settings, value| settings.keep_extract_progress_open = value,
            ),
            self.switch(
                owner.clone(),
                "archive-smart-single-file-folder",
                "智能解压时为单个文件创建文件夹",
                settings.smart_single_file_folder,
                |settings, value| settings.smart_single_file_folder = value,
            ),
            note(
                "文件名识别",
                "自动检测可能不确定；压缩包中有效的 Unicode 文件名优先，不会被手动代码页覆盖。",
            ),
        ];
        let create = vec![
            self.choice(
                owner.clone(),
                "默认压缩格式",
                settings.create_format,
                &[
                    (ArchiveCreateFormat::Zip, "ZIP"),
                    (ArchiveCreateFormat::SevenZ, "7z"),
                    (ArchiveCreateFormat::Tar, "TAR"),
                    (ArchiveCreateFormat::Tgz, "TAR.GZ"),
                    (ArchiveCreateFormat::Txz, "TAR.XZ"),
                    (ArchiveCreateFormat::Gz, "Gzip（单文件）"),
                    (ArchiveCreateFormat::Xz, "XZ（单文件）"),
                ],
                |settings, value| settings.create_format = value,
            ),
            self.choice(
                owner.clone(),
                "默认压缩级别",
                settings.compression_level,
                if matches!(
                    settings.create_format,
                    ArchiveCreateFormat::Gz | ArchiveCreateFormat::Xz
                ) {
                    &[
                        (ArchiveCompressionLevel::Fast, "快速"),
                        (ArchiveCompressionLevel::Normal, "标准"),
                        (ArchiveCompressionLevel::Maximum, "最大"),
                    ]
                } else {
                    &[
                        (ArchiveCompressionLevel::Store, "仅存储"),
                        (ArchiveCompressionLevel::Fast, "快速"),
                        (ArchiveCompressionLevel::Normal, "标准"),
                        (ArchiveCompressionLevel::Maximum, "最大"),
                    ]
                },
                |settings, value| settings.compression_level = value,
            )
            .disabled(settings.create_format == ArchiveCreateFormat::Tar),
            self.switch(
                owner.clone(),
                "archive-keep-create-progress-open",
                "压缩完成后保留进度窗口",
                settings.keep_create_progress_open,
                |settings, value| settings.keep_create_progress_open = value,
            ),
            self.switch(
                owner.clone(),
                "archive-test-after-create",
                "压缩后测试",
                settings.test_after_create,
                |settings, value| settings.test_after_create = value,
            ),
            self.switch(
                owner.clone(),
                "archive-strip-single-root",
                "单个文件夹不创建根文件夹",
                settings.strip_single_root,
                |settings, value| settings.strip_single_root = value,
            ),
            self.exclusions_item(owner.clone()),
        ];
        let pages = [
            SettingPage::new("常规")
                .resettable(false)
                .group(SettingGroup::new().title("打开与关闭").items([
                    self.switch(
                        owner.clone(),
                        "archive-open-in-nxtools",
                        "使用 NxTools 打开压缩包",
                        settings.open_in_nxtools,
                        |settings, value| settings.open_in_nxtools = value,
                    ),
                    self.switch(
                        owner.clone(),
                        "archive-escape-closes-window",
                        "按 Esc 关闭压缩包窗口",
                        settings.escape_closes_window,
                        |settings, value| settings.escape_closes_window = value,
                    ),
                    note(
                        "修改后验证",
                        "添加、删除、重命名及编辑注释后，提交前始终验证压缩包完整性。",
                    ),
                ]))
                .group(
                    SettingGroup::new()
                        .title("最近记录")
                        .items(self.recent_items(owner.clone())),
                ),
            SettingPage::new("解压")
                .resettable(false)
                .group(SettingGroup::new().title("解压默认参数").items(extract)),
            SettingPage::new("压缩")
                .resettable(false)
                .group(SettingGroup::new().title("新建默认参数").items(create)),
            SettingPage::new("查看").resettable(false).group(
                SettingGroup::new().title("压缩包浏览器").items([
                    self.switch(
                        owner.clone(),
                        "archive-show-parent-row",
                        "显示返回上级项",
                        settings.show_parent_row,
                        |settings, value| settings.show_parent_row = value,
                    ),
                    self.switch(
                        owner.clone(),
                        "archive-show-grid-lines",
                        "显示网格线",
                        settings.show_grid_lines,
                        |settings, value| settings.show_grid_lines = value,
                    ),
                    self.switch(
                        owner.clone(),
                        "archive-full-row-select",
                        "整行选择",
                        settings.full_row_select,
                        |settings, value| settings.full_row_select = value,
                    ),
                    self.switch(
                        owner.clone(),
                        "archive-single-click-open",
                        "单击打开文件或文件夹",
                        settings.single_click_open,
                        |settings, value| settings.single_click_open = value,
                    ),
                    self.switch(
                        owner.clone(),
                        "archive-show-tree",
                        "显示目录树",
                        settings.show_tree,
                        |settings, value| settings.show_tree = value,
                    ),
                    self.switch(
                        owner.clone(),
                        "archive-show-full-path",
                        "标题栏显示完整路径",
                        settings.show_full_path,
                        |settings, value| settings.show_full_path = value,
                    ),
                    self.switch(
                        owner.clone(),
                        "archive-auto-expand-tree",
                        "自动展开当前目录树",
                        settings.auto_expand_tree,
                        |settings, value| settings.auto_expand_tree = value,
                    ),
                ]),
            ),
            SettingPage::new("集成").resettable(false).group(
                SettingGroup::new()
                    .title("Windows 集成")
                    .items(self.integration_items(owner.clone())),
            ),
            SettingPage::new("高级")
                .resettable(false)
                .group(SettingGroup::new().title("安全").items([
                    self.switch(
                        owner.clone(),
                        "archive-copy-motw",
                        "保留下载文件安全标记",
                        settings.copy_motw,
                        |settings, value| settings.copy_motw = value,
                    ),
                    note(
                        "来源保护",
                        "源文件始终保留。取消操作不保证撤销已完成的输出。",
                    ),
                ]))
                .group(
                    SettingGroup::new()
                        .title("临时文件")
                        .items(self.temp_items(owner.clone())),
                ),
            self.preview_page(owner.clone()),
        ];
        let mut page = div()
            .id("archive-settings-page")
            .debug_selector(|| "archive-settings-page".into())
            .size_full()
            .min_w_0()
            .min_h_0()
            .flex()
            .flex_col()
            .child(
                ModuleHeader::new(
                    "archive-module-header",
                    "archive-module-title",
                    "archive-module-status",
                    "压缩",
                    "打开、新建或解压压缩包",
                )
                .render(Theme::active(cx)),
            )
            .child(div().flex_none().px_4().pt_2().child("选择操作"))
            .child(
                div()
                    .flex_none()
                    .px_4()
                    .debug_selector(|| "archive-primary-tasks".into())
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        Button::new("archive-open")
                            .debug_selector(|| "archive-open".into())
                            .label("打开压缩包…")
                            .outline()
                            .with_size(Size::Medium)
                            .on_click(|_, window, cx| {
                                crate::archive::pick_from_window(window.window_handle(), cx)
                            }),
                    )
                    .child(
                        Button::new("archive-create")
                            .debug_selector(|| "archive-create".into())
                            .label("新建压缩包…")
                            .outline()
                            .with_size(Size::Medium)
                            .on_click(|_, _, cx| crate::archive::new_from_window(cx)),
                    )
                    .child(
                        Button::new("archive-extract")
                            .debug_selector(|| "archive-extract".into())
                            .label("解压到…")
                            .outline()
                            .with_size(Size::Medium)
                            .on_click(|_, _, cx| crate::archive::extract_from_window(cx)),
                    ),
            )
            .child(div().flex_none().px_4().pt_2().child("偏好设置"))
            .child(
                div()
                    .debug_selector(|| "archive-settings-viewport".into())
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .child(
                        Settings::new("archive-official-settings")
                            .sidebar_width(
                                crate::ui::metrics::ordinary_rem(164.).to_pixels(window.rem_size()),
                            )
                            .sidebar_size_range(px(160.0)..px(208.0))
                            .with_group_variant(GroupBoxVariant::Fill)
                            .pages(pages),
                    ),
            );
        if let Some(message) = &self.error {
            page = page.child(
                div()
                    .debug_selector(|| "archive-settings-error".into())
                    .flex_none()
                    .p_2()
                    .child(Alert::error("archive-save-error", message.clone()).small()),
            );
        }
        page
    }
}
impl MainWindow {
    pub(super) fn render_archive(&self) -> AnyElement {
        match &self.archive_settings {
            Some(view) => view.clone().into_any_element(),
            None => div()
                .p_4()
                .child("压缩设置当前不可用：配置所有者尚未初始化。")
                .into_any_element(),
        }
    }
}
