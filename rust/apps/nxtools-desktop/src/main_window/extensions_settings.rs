//! W9 presentation only. Application owns validation, trust, credentials and atomic saves.
//! Released Kit 0.7.1 Settings public slots retain its search/navigation/scroll owner;
//! Form drafts retain InputState identity and never preload a password.
use super::*;
use gpui_kit::component::description_list::DescriptionList;
use gpui_kit::component::input::NumberInput;
use gpui_kit::component::list::ListItem;
use gpui_kit::component::select::{Select, SelectState};
use gpui_kit::component::setting::SettingField;
use gpui_kit::component::switch::Switch;
use gpui_kit::component::{Disableable, IndexPath};
use gpui_kit::{AnyElement, App, WeakEntity};
use nxtools_extensions::{
    HandlerReference, InstalledPackage, PackageRegistry, Setting, SettingType,
};
use std::collections::{BTreeMap, HashSet};

type SettingsPayload = (
    BTreeMap<String, serde_json::Value>,
    BTreeMap<String, Option<String>>,
);

pub(super) struct PackageSettingsDraft {
    id: String,
    content_hash: String,
    fields: Vec<Setting>,
    inputs: BTreeMap<String, Entity<InputState>>,
    choices: BTreeMap<String, Entity<SelectState<Vec<String>>>>,
    bools: BTreeMap<String, bool>,
    submitted: bool,
    clear_secrets: HashSet<String>,
}

impl PackageSettingsDraft {
    pub(super) fn package_id(&self) -> &str {
        &self.id
    }
    fn new(
        package: &InstalledPackage,
        window: &mut Window,
        cx: &mut Context<ExtensionManagerView>,
    ) -> Self {
        let mut draft = Self {
            id: package.manifest.id.clone(),
            content_hash: package.content_hash.clone(),
            fields: package.manifest.settings.clone(),
            inputs: BTreeMap::new(),
            choices: BTreeMap::new(),
            bools: BTreeMap::new(),
            clear_secrets: HashSet::new(),
            submitted: false,
        };
        for setting in &draft.fields {
            // Passwords are not read from registry, defaults or a credential adapter.
            let value = if matches!(setting.field, SettingType::Password) {
                None
            } else {
                package
                    .settings
                    .get(&setting.key)
                    .cloned()
                    .or_else(|| setting.default_value())
            };
            match &setting.field {
                SettingType::Bool { default } => {
                    draft.bools.insert(
                        setting.key.clone(),
                        value
                            .as_ref()
                            .and_then(serde_json::Value::as_bool)
                            .unwrap_or(*default),
                    );
                }
                SettingType::Choice { options, default } => {
                    let selected = value
                        .as_ref()
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or(default);
                    let index = options
                        .iter()
                        .position(|option| option == selected)
                        .map(|row| IndexPath::default().row(row));
                    let options = options.clone();
                    draft.choices.insert(
                        setting.key.clone(),
                        cx.new(|cx| SelectState::new(options, index, window, cx)),
                    );
                }
                field => {
                    let text = value
                        .map(|value| match value {
                            serde_json::Value::String(value) => value,
                            other => other.to_string(),
                        })
                        .unwrap_or_default();
                    let password = matches!(field, SettingType::Password);
                    let input = cx.new(|cx| {
                        let mut input = InputState::new(window, cx).masked(password);
                        input.set_value(text, window, cx);
                        input
                    });
                    draft.inputs.insert(setting.key.clone(), input);
                }
            }
        }
        draft
    }

    fn payload(&self, cx: &App) -> Result<SettingsPayload, String> {
        let mut values = BTreeMap::new();
        let mut secrets = BTreeMap::new();
        for setting in &self.fields {
            let key = &setting.key;
            let value = match &setting.field {
                SettingType::Password => {
                    let input = self.inputs[key].read(cx).value().to_string();
                    if self.clear_secrets.contains(key) {
                        secrets.insert(key.clone(), None);
                    } else if !input.is_empty() {
                        secrets.insert(key.clone(), Some(input));
                    }
                    continue;
                }
                SettingType::Bool { .. } => serde_json::Value::Bool(self.bools[key]),
                SettingType::Choice { .. } => {
                    let value = self.choices[key]
                        .read(cx)
                        .selected_value()
                        .cloned()
                        .ok_or_else(|| format!("{}：请选择一个选项", setting.title))?;
                    serde_json::Value::String(value)
                }
                SettingType::Number { min, max, .. } => {
                    let value = self.inputs[key]
                        .read(cx)
                        .value()
                        .trim()
                        .parse::<i64>()
                        .map_err(|_| {
                            format!("{}：请输入 {min}–{max} 范围内的整数", setting.title)
                        })?;
                    serde_json::Value::from(value)
                }
                _ => serde_json::Value::String(self.inputs[key].read(cx).value().to_string()),
            };
            if !setting.accepts(&value) {
                return Err(format!("{}：值不符合声明的类型、范围或选项", setting.title));
            }
            values.insert(key.clone(), value);
        }
        Ok((values, secrets))
    }
}

pub(super) struct ChainDraft {
    route: Vec<HandlerReference>,
    candidates: Vec<HandlerReference>,
    choice: Entity<SelectState<Vec<String>>>,
}

pub(super) fn current_trust(package: &InstalledPackage) -> bool {
    // Projection only: disk/hash revalidation belongs to application admission, not render.
    package
        .trust
        .as_ref()
        .is_some_and(|trust| trust.content_hash == package.content_hash)
}

fn candidates(registry: &PackageRegistry) -> Vec<HandlerReference> {
    registry
        .packages
        .iter()
        .filter(|package| current_trust(package))
        .flat_map(|package| {
            package
                .manifest
                .handlers
                .iter()
                .filter(|handler| handler.feature == "downloads" && handler.entry == "resolve")
                .map(move |handler| HandlerReference {
                    package_id: package.manifest.id.clone(),
                    handler_id: handler.id.clone(),
                })
        })
        .collect()
}

fn reference_label(reference: &HandlerReference) -> String {
    format!("{} / {}", reference.package_id, reference.handler_id)
}

pub(super) fn view_button(
    id: impl Into<String>,
    label: impl Into<String>,
    disabled: bool,
    view: WeakEntity<ExtensionManagerView>,
    action: impl Fn(&mut ExtensionManagerView, &mut Window, &mut Context<ExtensionManagerView>)
    + 'static,
) -> Button {
    let id = id.into();
    let label: String = label.into();
    let action = Rc::new(action);
    Button::new(id.clone())
        .debug_selector(move || id.clone())
        .label(label)
        .outline()
        .disabled(disabled)
        .on_click(move |_, window, cx| {
            let view = view.clone();
            let action = action.clone();
            let source = window.window_handle();
            cx.defer(move |cx| {
                let _ = source.update(cx, |_, window, cx| {
                    let _ = view.update(cx, |view, cx| {
                        action(view, window, cx);
                        cx.notify();
                    });
                });
            });
        })
}

pub(super) fn app_button(
    id: impl Into<String>,
    label: impl Into<String>,
    disabled: bool,
    owner: WeakEntity<ExtensionApplication>,
    action: impl Fn(&mut ExtensionApplication, &mut Context<ExtensionApplication>) + 'static,
) -> Button {
    let id = id.into();
    let label: String = label.into();
    let action = Rc::new(action);
    Button::new(id.clone())
        .debug_selector(move || id.clone())
        .label(label)
        .outline()
        .disabled(disabled)
        .on_click(move |_, _, cx| {
            let owner = owner.clone();
            let action = action.clone();
            cx.defer(move |cx| {
                let _ = owner.update(cx, |app, cx| action(app, cx));
            });
        })
}

fn warning_item() -> SettingItem {
    SettingItem::render(|_, _, _| {
        div()
            .w_full()
            .debug_selector(|| "extension-permission-warning".into())
            .child(
                Alert::warning("extension-full-user-trust", TRUST_WARNING)
                    .title("完整用户权限，不是沙箱")
                    .small(),
            )
    })
    .keywords(["信任", "权限", "声明"])
}

impl ExtensionManagerView {
    pub(super) fn reconcile_settings(&mut self, cx: &mut Context<Self>) {
        let state = self.application.read(cx);
        self.parked_settings.retain(|id, _| {
            state
                .packages
                .packages
                .iter()
                .any(|package| package.manifest.id == *id)
        });
        let completed = self.seen_commit_revision != state.commit_revision;
        self.seen_commit_revision = state.commit_revision;
        if !state.busy
            && state.preview.is_none()
            && completed
            && matches!(state.last_commit, Some(("import", true, _)))
        {
            self.import_open = false;
        }
        if self
            .settings_draft
            .as_ref()
            .is_some_and(|draft| draft.submitted)
            && !state.busy
        {
            if matches!(state.last_commit, Some(("settings", true, _))) {
                self.settings_draft = None;
            } else if let Some(draft) = &mut self.settings_draft {
                draft.submitted = false;
            }
        }
    }
    pub(super) fn render_w9_editor(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        if self.chain_draft.is_some() {
            return Some(self.render_chain(window, cx));
        }
        let id = self.selected_package.clone()?;
        let state = self.application.read(cx);
        let package = state
            .packages
            .packages
            .iter()
            .find(|package| package.manifest.id == id)
            .cloned();
        let busy = state.busy;
        let Some(package) = package else {
            self.selected_package = None;
            self.settings_draft = None;
            self.trust_target = None;
            return None;
        };
        let view = cx.entity().downgrade();
        let access = vec![
            self.package_usage(&package, cx),
            SettingGroup::new().items([warning_item(), self.package_actions(&package, busy, cx)]),
        ];
        let mut settings = vec![SettingGroup::new().item(SettingItem::render({
            let view = view.clone();
            let package = package.clone();
            move |_, _, _| {
                let package = package.clone();
                view_button(
                    "extension-settings-edit",
                    "编辑设置",
                    busy || package.manifest.settings.is_empty(),
                    view.clone(),
                    move |view, window, cx| {
                        if view
                            .settings_draft
                            .as_ref()
                            .is_none_or(|draft| draft.id != package.manifest.id)
                        {
                            view.settings_draft =
                                Some(PackageSettingsDraft::new(&package, window, cx));
                        }
                        view.draft_error = None;
                    },
                )
            }
        }))];
        if self
            .settings_draft
            .as_ref()
            .is_some_and(|draft| draft.id == id)
        {
            settings.push(self.settings_form(&package, busy, cx));
        }
        let header = gpui_kit::component::toolbar::Toolbar::new("extension-detail-header")
            .with_size(gpui_kit::component::Size::Medium)
            .h_auto()
            .min_h_8()
            .w_full()
            .content(view_button(
                "extension-detail-back",
                "返回扩展",
                busy,
                view.clone(),
                |view, _, _| {
                    // Keep drafts and search when navigating back; Cancel is the explicit discard action.
                    view.selected_package = None;
                },
            ))
            .content(div().min_w_0().flex_1().child(format!(
                "{} · {} · {}",
                package.manifest.name,
                package.manifest.version,
                if package.enabled {
                    "已启用"
                } else {
                    "已停用"
                }
            )))
            .content(self.activity_button().outline().disabled(busy));
        let _ = window;
        let mut detail = div()
            .size_full()
            .min_w_0()
            .flex()
            .flex_col()
            .gap_2()
            .child(header)
            .child(
                div()
                    .debug_selector(|| "extension-detail-settings-viewport".into())
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .child(
                        Settings::new(format!("extension-package-detail-{id}"))
                            .default_selected_index(gpui_kit::component::setting::SelectIndex {
                                page_ix: self.detail_initial_page,
                                group_ix: None,
                            })
                            .header_style(&gpui_kit::StyleRefinement {
                                display: Some(gpui_kit::Display::None),
                                ..Default::default()
                            })
                            .sidebar_width(px(160.0))
                            .sidebar_size_range(px(160.0)..px(208.0))
                            .pages([
                                SettingPage::new("用途与启用")
                                    .resettable(false)
                                    .groups(access),
                                SettingPage::new("命令与工具")
                                    .resettable(false)
                                    .group(self.package_commands(&package, busy, cx)),
                                SettingPage::new("插件设置")
                                    .resettable(false)
                                    .groups(settings),
                                SettingPage::new("自动化")
                                    .resettable(false)
                                    .group(self.handler_controls(&package, busy, cx)),
                                SettingPage::new("来源与能力")
                                    .resettable(false)
                                    .group(package_metadata(&package)),
                                SettingPage::new("运行环境")
                                    .resettable(false)
                                    .group(self.package_environment(&package, cx)),
                            ]),
                    ),
            );
        if let Some(message) = self.draft_error.clone() {
            detail = detail.child(Alert::error("extension-operation-validation", message).small());
        }
        if self
            .settings_draft
            .as_ref()
            .is_some_and(|draft| draft.id == id)
        {
            detail = detail.child(self.settings_actions(&package, busy, cx));
        }
        Some(detail.into_any_element())
    }

    fn package_actions(
        &self,
        package: &InstalledPackage,
        busy: bool,
        cx: &Context<Self>,
    ) -> SettingItem {
        let owner = self.application.downgrade();
        let view = cx.entity().downgrade();
        let package = package.clone();
        SettingItem::render(move |_, _, _| {
            let id = package.manifest.id.clone();
            let mut actions = div().w_full().flex().flex_wrap().gap_2();
            if package.manifest.runtime.is_some() {
                let trust_id = id.clone();
                actions = actions.child(view_button(
                    "extension-trust-edit",
                    "查看并确认信任",
                    busy,
                    view.clone(),
                    move |view, window, cx| {
                        view.open_trust(trust_id.clone(), window, cx);
                    },
                ));
                let revoke_id = id.clone();
                actions = actions.child(app_button(
                    "extension-revoke",
                    "撤销信任并停用",
                    busy || package.trust.is_none(),
                    owner.clone(),
                    move |app, cx| app.revoke(revoke_id.clone(), cx),
                ));
            }
            let enabled = package.enabled;
            let toggle_id = id;
            actions = actions.child(app_button(
                "extension-detail-toggle",
                if enabled {
                    "停用插件"
                } else {
                    "启用插件"
                },
                busy || (!enabled && package.manifest.runtime.is_some() && package.trust.is_none()),
                owner.clone(),
                move |app, cx| app.set_enabled(toggle_id.clone(), !enabled, cx),
            ));
            let remove_id = package.manifest.id.clone();
            actions = actions.child(app_button(
                format!("extension-remove-{remove_id}"),
                "移除扩展",
                busy,
                owner.clone(),
                move |app, cx| app.remove(remove_id.clone(), cx),
            ));
            div()
                .w_full()
                .flex()
                .flex_col()
                .gap_2()
                .child(actions)
                .child(if package.manifest.runtime.is_some() {
                    "信任不会自动启用插件；启用后仍保留资源与取消限制。"
                } else {
                    "声明式动作：启用与导入分开；请先检查来源和动作。"
                })
        })
        .keywords(["信任", "启用", "设置", "撤销"])
    }

    // Trust confirmation is a real Dialog; see extensions_page/trust_confirmation.rs.

    fn settings_form(
        &self,
        package: &InstalledPackage,
        busy: bool,
        cx: &Context<Self>,
    ) -> SettingGroup {
        let draft = self
            .settings_draft
            .as_ref()
            .expect("selected settings draft");
        let view = cx.entity().downgrade();
        let mut items = vec![SettingItem::render(|_, _, _| {
            Alert::new("extension-secret-disclosure", "密码保存到 Windows 凭据管理器；受信任插件可按接口读取您配置的密码。密码不预载，留空保持不变；清除必须明确选择。取消不写入。").small()
        })];
        // Independent public items are searchable; persistent entities are never rebuilt on render.
        for setting in &draft.fields {
            let key = setting.key.clone();
            let label = setting.title.clone();
            let description = setting.description.clone();
            let field = match &setting.field {
                SettingType::Bool { .. } => {
                    let checked = draft.bools[&key];
                    let owner = view.clone();
                    let key = key.clone();
                    SettingField::render(move |_, _, _| {
                        let owner = owner.clone();
                        let key = key.clone();
                        let selector = format!("extension-setting-{key}");
                        div().debug_selector(move || selector.clone()).child(
                            Switch::new(format!("extension-setting-{key}"))
                                .checked(checked)
                                .accessibility_label(label.clone())
                                .disabled(busy)
                                .on_change(move |requested, _, cx| {
                                    let requested = *requested;
                                    let owner = owner.clone();
                                    let key = key.clone();
                                    cx.defer(move |cx| {
                                        let _ = owner.update(cx, |view, cx| {
                                            if let Some(draft) = &mut view.settings_draft {
                                                draft.bools.insert(key, requested);
                                            }
                                            cx.notify();
                                        });
                                    });
                                }),
                        )
                    })
                }
                SettingType::Choice { .. } => {
                    let choice = draft.choices[&key].clone();
                    SettingField::render(move |_, _, _| {
                        Select::new(&choice)
                            .accessibility_label(label.clone())
                            .disabled(busy)
                    })
                }
                SettingType::Number { min, max, .. } => {
                    let input = draft.inputs[&key].clone();
                    let range = format!("{min}–{max}（整数）");
                    SettingField::render(move |_, _, _| {
                        NumberInput::new(&input)
                            .suffix(range.clone())
                            .disabled(busy)
                    })
                }
                SettingType::Password => {
                    let input = draft.inputs[&key].clone();
                    let clear = draft.clear_secrets.contains(&key);
                    let owner = view.clone();
                    let key = key.clone();
                    SettingField::render(move |_, _, _| {
                        let owner = owner.clone();
                        let key = key.clone();
                        div()
                            .w_full()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(
                                Input::new(&input)
                                    .aria_label(label.clone())
                                    .disabled(busy || clear),
                            )
                            .child(
                                div()
                                    .debug_selector({
                                        let selector = format!("extension-secret-clear-{key}");
                                        move || selector.clone()
                                    })
                                    .child(
                                        Switch::new(format!("extension-secret-clear-{key}"))
                                            .label("明确清除此密码")
                                            .checked(clear)
                                            .disabled(busy)
                                            .on_change(move |requested, _, cx| {
                                                let requested = *requested;
                                                let owner = owner.clone();
                                                let key = key.clone();
                                                cx.defer(move |cx| {
                                                    let _ = owner.update(cx, |view, cx| {
                                                        if let Some(draft) =
                                                            &mut view.settings_draft
                                                        {
                                                            if requested {
                                                                draft.clear_secrets.insert(key);
                                                            } else {
                                                                draft.clear_secrets.remove(&key);
                                                            }
                                                        }
                                                        cx.notify();
                                                    });
                                                });
                                            }),
                                    ),
                            )
                    })
                }
                _ => {
                    let input = draft.inputs[&key].clone();
                    SettingField::render(move |_, _, _| {
                        Input::new(&input).aria_label(label.clone()).disabled(busy)
                    })
                }
            };
            let mut item = SettingItem::new(setting.title.clone(), field)
                .keywords([setting.key.clone(), "设置".into()]);
            if !description.is_empty() {
                item = item.description(description);
            }
            if matches!(
                setting.field,
                SettingType::File { .. } | SettingType::Folder { .. }
            ) {
                item = item.description("手动输入文件 / 文件夹路径（此字段没有原生选择器）");
            }
            items.push(item);
        }
        let _ = package;
        SettingGroup::new().title("插件设置草稿").items(items)
    }

    fn settings_actions(
        &self,
        package: &InstalledPackage,
        busy: bool,
        cx: &Context<Self>,
    ) -> AnyElement {
        let draft = self
            .settings_draft
            .as_ref()
            .expect("selected settings draft");
        let content_current = draft.content_hash == package.content_hash;
        let submitted = draft.submitted;
        let view_for_footer = cx.entity().downgrade();
        div()
            .w_full()
            .flex_shrink_0()
            .p_2()
            .flex()
            .flex_wrap()
            .gap_2()
            .child(view_button(
                "extension-settings-save",
                "保存设置",
                busy || submitted || !content_current,
                view_for_footer.clone(),
                |view, _, cx| {
                    let Some(draft) = &view.settings_draft else {
                        return;
                    };
                    if draft.submitted {
                        return;
                    }
                    let current =
                        view.application
                            .read(cx)
                            .packages
                            .packages
                            .iter()
                            .any(|package| {
                                package.manifest.id == draft.id
                                    && package.content_hash == draft.content_hash
                            });
                    if !current {
                        view.draft_error =
                            Some("插件内容已变更；请取消并重新编辑，未写入任何值".into());
                        return;
                    }
                    match draft.payload(cx) {
                        Ok((values, secrets)) => {
                            let id = draft.id.clone();
                            let owner = view.application.downgrade();
                            view.draft_error = None;
                            if let Some(draft) = &mut view.settings_draft {
                                draft.submitted = true;
                            }
                            cx.defer(move |cx| {
                                let _ = owner.update(cx, |app, cx| {
                                    app.save_settings(id, values, secrets, cx)
                                });
                            });
                        }
                        Err(message) => view.draft_error = Some(message),
                    }
                },
            ))
            .child(view_button(
                "extension-settings-cancel",
                "取消设置",
                busy,
                view_for_footer.clone(),
                |view, _, _| {
                    view.settings_draft = None;
                    view.draft_error = None;
                },
            ))
            .into_any_element()
    }

    fn handler_controls(
        &self,
        package: &InstalledPackage,
        busy: bool,
        cx: &Context<Self>,
    ) -> SettingGroup {
        let owner = self.application.downgrade();
        let mut items = Vec::new();
        for handler in &package.manifest.handlers {
            let event = usage::is_event(handler);
            let provider = handler.feature == "quick_launch" && handler.entry == "provider";
            if !event && !provider {
                continue;
            }
            let checked = if event {
                package.enabled_events.contains(&handler.id)
            } else {
                package.enabled_providers.contains(&handler.id)
            };
            let owner = owner.clone();
            let id = package.manifest.id.clone();
            let handler_id = handler.id.clone();
            let label = usage::handler_heading(handler);
            let scopes = self.control_scopes.clone();
            let paused = event && self.application.read(cx).event_paused(&id, &handler_id);
            let retry_owner = owner.clone();
            let retry_id = id.clone();
            let retry_handler = handler_id.clone();
            items.push(
                SettingItem::new(
                    label.clone(),
                    SettingField::render(move |_, window, cx| {
                        let owner = owner.clone();
                        let id = id.clone();
                        let handler_id = handler_id.clone();
                        let selector = format!("extension-opt-in-{id}-{handler_id}");
                        let scope = window
                            .use_keyed_state(format!("scope-{selector}"), cx, |_, cx| {
                                cx.focus_handle()
                            })
                            .read(cx)
                            .clone();
                        scopes.borrow_mut().insert(selector.clone(), scope.clone());
                        div()
                            .id(format!("scope-root-{selector}"))
                            .track_focus(&scope)
                            .tab_index(0)
                            .tab_stop(false)
                            .debug_selector(move || selector.clone())
                            .child(
                                Switch::new(format!("extension-opt-in-{id}-{handler_id}"))
                                    .checked(checked)
                                    .label(if event {
                                        "自动事件（默认关闭）"
                                    } else {
                                        "普通查询提供者（单独选择加入）"
                                    })
                                    .disabled(busy)
                                    .accessibility_label(label.clone())
                                    .on_change(move |requested, _, cx| {
                                        if busy {
                                            return;
                                        }
                                        let requested = *requested;
                                        let owner = owner.clone();
                                        let id = id.clone();
                                        let handler = handler_id.clone();
                                        cx.defer(move |cx| {
                                            let _ = owner.update(cx, |app, cx| {
                                                if event {
                                                    app.set_event(id, handler, requested, cx);
                                                } else {
                                                    app.set_provider(id, handler, requested, cx);
                                                }
                                            });
                                        });
                                    }),
                            )
                            .when(paused, |row| {
                                row.child(
                                    div()
                                        .flex()
                                        .flex_col()
                                        .gap_1()
                                        .child("一分钟内连续失败三次，自动事件已暂停")
                                        .child(app_button(
                                            format!(
                                                "extension-event-retry-{retry_id}-{retry_handler}"
                                            ),
                                            "重试自动事件",
                                            busy,
                                            retry_owner.clone(),
                                            {
                                                let id = retry_id.clone();
                                                let handler = retry_handler.clone();
                                                move |app, cx| {
                                                    app.retry_event(id.clone(), handler.clone(), cx)
                                                }
                                            },
                                        )),
                                )
                            })
                    }),
                )
                .description(format!("{} 仅保存独立参与意图；停用或撤销信任时不会运行。自动事件暂停后可手动恢复，不重放旧事件。", usage::trigger(package, handler))),
            );
        }
        if items.is_empty() {
            items.push(SettingItem::render(|_, _, _| {
                div().child("此插件未声明自动事件或普通查询提供者。")
            }));
        }
        SettingGroup::new()
            .title("自动事件与普通提供者")
            .items(items)
    }

    pub(super) fn chain_entry_button(&self, cx: &Context<Self>) -> Button {
        let view = cx.entity().downgrade();
        let busy = self.application.read(cx).busy;
        view_button(
            "extension-chain-edit",
            "下载解析链",
            busy,
            view,
            |view, window, cx| {
                let registry = view.application.read(cx).packages.clone();
                let candidates = candidates(&registry);
                let labels = candidates.iter().map(reference_label).collect::<Vec<_>>();
                let choice = cx.new(|cx| SelectState::new(labels, None, window, cx));
                view.chain_draft = Some(ChainDraft {
                    route: Vec::new(),
                    candidates,
                    choice,
                });
                view.draft_error = None;
                view.chain_host.update(cx, |input, cx| {
                    input.set_value("", window, cx);
                    input.focus(window, cx);
                });
            },
        )
    }

    fn render_chain(&self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let draft = self.chain_draft.as_ref().expect("chain editor");
        let registry = self.application.read(cx).packages.clone();
        let busy = self.application.read(cx).busy;
        let view = cx.entity().downgrade();
        let host = self.chain_host.clone();
        let choice = draft.choice.clone();
        let route = draft.route.clone();
        let available = candidates(&registry);
        let error = self.draft_error.clone();
        let scopes = self.control_scopes.clone();
        let content = move |window: &mut Window, cx: &mut App| {
            let mut stages = div().w_full().flex().flex_col().gap_2();
            for (index, reference) in route.iter().enumerate() {
                let unavailable = !available.contains(reference);
                let label = format!(
                    "{}. {}{}",
                    index + 1,
                    reference_label(reference),
                    if unavailable {
                        " · 不可用（停用 / 未信任 / 已移除）"
                    } else {
                        ""
                    }
                );
                let mut row = div().flex().flex_wrap().gap_2();
                for (suffix, title, delta) in [
                    ("up", "上移", -1isize),
                    ("down", "下移", 1isize),
                    ("remove", "移除", 0isize),
                ] {
                    let disabled = busy
                        || (delta < 0 && index == 0)
                        || (delta > 0 && index + 1 == route.len());
                    let selector = format!("extension-chain-{suffix}-{index}");
                    let scope = window
                        .use_keyed_state(format!("scope-{selector}"), cx, |_, cx| cx.focus_handle())
                        .read(cx)
                        .clone();
                    scopes.borrow_mut().insert(selector.clone(), scope.clone());
                    row = row.child(
                        div()
                            .id(format!("scope-root-{selector}"))
                            .track_focus(&scope)
                            .tab_stop(false)
                            .child(view_button(
                                selector,
                                title,
                                disabled,
                                view.clone(),
                                move |view, _, _| {
                                    if let Some(draft) = &mut view.chain_draft {
                                        if index >= draft.route.len() {
                                            return;
                                        }
                                        if delta == 0 {
                                            draft.route.remove(index);
                                        } else {
                                            let other = index as isize + delta;
                                            if other >= 0 && (other as usize) < draft.route.len() {
                                                draft.route.swap(index, other as usize);
                                            }
                                        }
                                    }
                                },
                            )),
                    );
                }
                stages = stages.child(
                    ListItem::new(format!("extension-chain-stage-{index}"))
                        .accessibility_label(label.clone())
                        .child(
                            div()
                                .min_w_0()
                                .flex_1()
                                .flex()
                                .flex_col()
                                .gap_2()
                                .child(label)
                                .child(row),
                        ),
                );
            }
            if route.is_empty() {
                stages =
                    stages.child("未配置阶段：保存空链将移除该域名的显式链，恢复默认单一选择。");
            }
            let form = Form::new()
                .child(
                    Field::new()
                        .label("明确的域名")
                        .child(Input::new(&host).aria_label("解析链域名").disabled(busy)),
                )
                .child(
                    Field::new()
                        .label("读取已保存的链 / 单一偏好")
                        .child(view_button(
                            "extension-chain-load",
                            "读取此域名",
                            busy,
                            view.clone(),
                            |view, _, cx| {
                                let host =
                                    view.chain_host.read(cx).value().trim().to_ascii_lowercase();
                                let registry = &view.application.read(cx).packages;
                                let route = registry
                                    .resolver_chains
                                    .get(&host)
                                    .cloned()
                                    .or_else(|| {
                                        registry
                                            .resolver_preferences
                                            .get(&host)
                                            .cloned()
                                            .map(|reference| vec![reference])
                                    })
                                    .unwrap_or_default();
                                if let Some(draft) = &mut view.chain_draft {
                                    draft.route = route;
                                }
                                view.draft_error = None;
                            },
                        )),
                )
                .child(
                    Field::new()
                        .label("加入具有当前信任记录的解析处理器（执行仍要求启用）")
                        .child(
                            div()
                                .flex()
                                .gap_2()
                                .child(
                                    Select::new(&choice)
                                        .accessibility_label("可选解析处理器")
                                        .disabled(busy),
                                )
                                .child(view_button(
                                    "extension-chain-add",
                                    "添加阶段",
                                    busy || route.len() >= 8,
                                    view.clone(),
                                    |view, _, cx| {
                                        if let Some(draft) = &mut view.chain_draft {
                                            let selected =
                                                draft.choice.read(cx).selected_value().cloned();
                                            let ready =
                                                candidates(&view.application.read(cx).packages);
                                            let reference = selected
                                                .and_then(|label| {
                                                    draft.candidates.iter().find(|reference| {
                                                        reference_label(reference) == label
                                                            && ready.contains(reference)
                                                    })
                                                })
                                                .cloned();
                                            if let Some(reference) = reference {
                                                if draft.route.len() < 8
                                                    && !draft.route.contains(&reference)
                                                {
                                                    draft.route.push(reference);
                                                } else {
                                                    view.draft_error = Some(
                                                        "最多 8 阶段，且不能重复同一处理器".into(),
                                                    );
                                                }
                                            } else {
                                                view.draft_error =
                                                    Some("请先选择一个可用处理器".into());
                                            }
                                        }
                                    },
                                )),
                        ),
                )
                .footer(
                    div()
                        .flex()
                        .gap_2()
                        .child(view_button(
                            "extension-chain-save",
                            "保存解析链",
                            busy,
                            view.clone(),
                            |view, _, cx| {
                                let host =
                                    view.chain_host.read(cx).value().trim().to_ascii_lowercase();
                                if host.is_empty()
                                    || host.len() > 253
                                    || host.contains(['/', ':', '\\', ' '])
                                    || url::Host::parse(&host).is_err()
                                {
                                    view.draft_error =
                                        Some("请输入有效域名，不含协议、端口或路径".into());
                                    return;
                                }
                                let Some(draft) = &view.chain_draft else {
                                    return;
                                };
                                let route = draft.route.clone();
                                let owner = view.application.downgrade();
                                view.draft_error = None;
                                cx.defer(move |cx| {
                                    let _ = owner.update(cx, |app, cx| {
                                        app.set_resolver_chain(host, route, cx)
                                    });
                                });
                            },
                        ))
                        .child(view_button(
                            "extension-chain-cancel",
                            "取消链编辑",
                            busy,
                            view.clone(),
                            |view, _, _| {
                                view.chain_draft = None;
                                view.draft_error = None;
                            },
                        )),
                );
            let mut body = div()
                .w_full()
                .flex()
                .flex_col()
                .gap_2()
                .child(form)
                .child(stages)
                .child(
                    "阶段必须已启用且受信任；不可用引用保留供修复，不会静默切换或绕过失败阶段。",
                );
            if let Some(error) = &error {
                body = body.child(Alert::error("extension-chain-error", error.clone()).small());
            }
            body
        };
        div()
            .size_full()
            .min_w_0()
            .flex()
            .flex_col()
            .gap_3()
            .p_4()
            .child(
                gpui_kit::component::toolbar::Toolbar::new("extension-chain-header")
                    .with_size(gpui_kit::component::Size::Medium)
                    .content("下载解析链 · 按顺序执行，失败即停止"),
            )
            .child(
                div()
                    .id("extension-chain-scroll")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .child(content(window, cx)),
            )
            .into_any_element()
    }
}

fn package_metadata(package: &InstalledPackage) -> SettingGroup {
    let package = package.clone();
    SettingGroup::new().title("来源、身份与权限声明").item(
        SettingItem::render(move |_, _, _| {
            let trust = package.trust.as_ref();
            let metadata = DescriptionList::new()
                .columns(1)
                .bordered(false)
                .item("来源", package.source.clone(), 1)
                .item(
                    "版本 / 标识",
                    format!("{} / {}", package.manifest.version, package.manifest.id),
                    1,
                )
                .item("作者", package.manifest.author.clone(), 1)
                .item("代码 / 声明 SHA-256", package.content_hash.clone(), 1)
                .item(
                    "运行时",
                    runtime_description(package.manifest.runtime.as_ref()),
                    1,
                )
                .item(
                    "已审批运行时",
                    trust
                        .map(|trust| trust.runtime.executable.to_string_lossy().into_owned())
                        .unwrap_or_else(|| "未审批".into()),
                    1,
                )
                .item(
                    "运行时 SHA-256",
                    trust
                        .map(|trust| trust.runtime.sha256.clone())
                        .unwrap_or_else(|| "未审批".into()),
                    1,
                )
                .item(
                    "信任记录",
                    if current_trust(&package) {
                        "记录匹配当前内容；执行前仍需重新验证"
                    } else {
                        "无当前审批 / 内容已变更"
                    },
                    1,
                )
                .item(
                    "启用状态",
                    if package.enabled {
                        "已启用"
                    } else {
                        "已停用"
                    },
                    1,
                );
            let mut permissions = DescriptionList::new().columns(1).bordered(false);
            for permission in &package.manifest.permissions {
                permissions = permissions.item(
                    format!("声明 {}", permission.id),
                    permission.reason.clone(),
                    1,
                );
            }
            if package.manifest.permissions.is_empty() {
                permissions = permissions.item(
                    "权限声明",
                    "作者未声明权限；这不代表系统限制插件的访问。",
                    1,
                );
            }
            let mut handlers = div().w_full().min_w_0().flex().flex_col().gap_2();
            for handler in &package.manifest.handlers {
                let selector = format!("extension-handler-metadata-{}", handler.id);
                handlers = handlers.child(
                    ListItem::new(selector.clone()).w_full().min_w_0().child(
                        div()
                            .debug_selector(move || selector.clone())
                            .w_full()
                            .min_w_0()
                            .child(
                                DescriptionList::new()
                                    .columns(1)
                                    .bordered(false)
                                    .item("扩展", package.manifest.id.clone(), 1)
                                    .item("处理器", handler.id.clone(), 1)
                                    .item("功能", handler.feature.to_string(), 1)
                                    .item("入口", handler.entry.clone(), 1)
                                    .item("动作", action_description(&handler.action), 1),
                            ),
                    ),
                );
            }
            div()
                .w_full()
                .min_w_0()
                .flex()
                .flex_col()
                .gap_3()
                .child(metadata)
                .child(
                    gpui_kit::component::group_box::GroupBox::new()
                        .title("作者声明的用途")
                        .child(permissions),
                )
                .child(
                    gpui_kit::component::group_box::GroupBox::new()
                        .title("处理器信息")
                        .child(handlers),
                )
        })
        .keywords(["来源", "版本", "运行时", "权限", "哈希"]),
    )
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use gpui_kit::Focusable;
    use gpui_kit::component::WindowExt;
    use gpui_kit::{KeyUpEvent, Keystroke, Modifiers, TestAppContext, VisualTestContext, size};
    #[gpui_kit::test]
    fn component_followup_empty_activity_retains_message_close_and_no_execution(
        cx: &mut TestAppContext,
    ) {
        let temp = tempfile::tempdir().expect("owned activity fixture");
        let (application, _, window) = mount(
            nxtools_core::AppConfig::default(),
            temp.path().join("unused.json"),
            cx,
        );
        window.update(|window, cx| {
            window.activate_window();
            window.draw(cx).clear(cx);
        });
        let activity = window.update(|window, _| {
            gpui_kit::base::test_support::snapshots(window)
                .into_iter()
                .find(|node| node.path().last() == Some(&"extension-recent-activity".into()))
                .expect("actual activity Button")
                .bounds()
        });
        assert!(activity.top() >= px(0.0) && activity.bottom() <= px(430.0));
        window.simulate_click(activity.center(), Modifiers::default());
        window.run_until_parked();
        window.update(|window, cx| window.draw(cx).clear(cx));
        let empty = window
            .debug_bounds("extension-activity-empty")
            .expect("official Empty header");
        assert!(empty.top() >= px(0.0) && empty.bottom() <= px(430.0));
        assert!(application.read_with(window, |app, _| app.recent_manual_runs().is_empty()));
        assert!(window.update(|window, cx| window.has_active_dialog(cx)));
        window.simulate_keystrokes("escape");
        window.run_until_parked();
        assert!(!window.update(|window, cx| window.has_active_dialog(cx)));
    }

    #[gpui_kit::test]
    fn component_followup_permissions_and_handlers_use_structured_metadata(
        cx: &mut TestAppContext,
    ) {
        let temp = tempfile::tempdir().expect("owned metadata fixture");
        let config = fixture(&temp);
        let package = config.extensions.packages[0].clone();
        let (application, view, window) = mount(config, temp.path().join("unused.json"), cx);
        view.update_in(window, |view, _, cx| {
            view.selected_package = Some(package.manifest.id.clone());
            view.detail_initial_page = 4;
            cx.notify();
        });
        window.update(|window, cx| window.draw(cx).clear(cx));
        assert!(
            window
                .debug_bounds("extension-handler-metadata-one")
                .is_some(),
            "handler projected through structured public slots"
        );
        let mut last_reachable = false;
        for _ in 0..80 {
            window.update(|window, cx| window.draw(cx).clear(cx));
            let viewport = window
                .debug_bounds("extension-detail-settings-viewport")
                .expect("original Settings viewport");
            if window
                .debug_bounds("extension-handler-metadata-provider")
                .is_some_and(|bounds| {
                    bounds.top() >= viewport.top() && bounds.bottom() <= viewport.bottom()
                })
            {
                last_reachable = true;
                break;
            }
            window.simulate_event(gpui_kit::ScrollWheelEvent {
                position: gpui_kit::point(px(650.0), px(350.0)),
                delta: gpui_kit::ScrollDelta::Lines(gpui_kit::point(0.0, -3.0)),
                ..Default::default()
            });
        }
        assert!(
            last_reachable,
            "all original handler metadata remains reachable via the official Settings scroll"
        );
        assert_eq!(
            application.read_with(window, |app, _| app.packages.packages[0]
                .manifest
                .handlers
                .len()),
            package.manifest.handlers.len()
        );
        assert!(
            application.read_with(window, |app, _| !app.packages.packages[0].enabled
                && app.packages.packages[0].trust.is_none())
        );
    }

    #[gpui_kit::test]
    fn extension_transient_status_never_reflows_busy_catalog_or_replaces_input_entities(
        cx: &mut TestAppContext,
    ) {
        let temp = tempfile::tempdir().expect("owned layout fixture");
        let (application, view, window) =
            mount(fixture(&temp), temp.path().join("unused.json"), cx);
        window.update(|window, cx| {
            view.update(cx, |view, cx| {
                view.import_open = true;
                view.path.update(cx, |input, cx| {
                    input.set_value("C:\\Synthetic\\draft.zip", window, cx)
                });
                view.search
                    .update(cx, |input, cx| input.set_value("manager", window, cx));
                cx.notify();
            });
        });
        window.update(|window, cx| window.draw(cx).clear(cx));
        let grid = window
            .debug_bounds("extension-catalog-grid")
            .expect("mounted catalog");
        let input = window
            .debug_bounds("extension-import-path-control")
            .expect("mounted import input");
        let identities = window.read(|cx| {
            (
                view.read(cx).search.entity_id(),
                view.read(cx).path.entity_id(),
            )
        });
        for busy in [true, false] {
            application.update(window, |app, cx| {
                app.busy = busy;
                // A legacy/non-visual status must never become a page-wide banner again.
                app.status = Some("已保存".into());
                cx.notify();
            });
            window.run_until_parked();
            window.update(|window, cx| window.draw(cx).clear(cx));
            assert_eq!(
                window.debug_bounds("extension-catalog-grid"),
                Some(grid),
                "no busy/status layout shift"
            );
            assert_eq!(
                window.debug_bounds("extension-import-path-control"),
                Some(input)
            );
            window.read(|cx| {
                let view = view.read(cx);
                assert_eq!((view.search.entity_id(), view.path.entity_id()), identities);
                assert_eq!(view.path.read(cx).value(), "C:\\Synthetic\\draft.zip");
                assert_eq!(view.search.read(cx).value(), "manager");
                assert!(
                    view.import_open,
                    "unrelated operation must not collapse a draft"
                );
            });
            if busy {
                let manage = window
                    .debug_bounds("extension-manage-fixture.manager")
                    .expect("mounted official button");
                window.simulate_click(manage.center(), Modifiers::default());
                window.run_until_parked();
                assert!(
                    window.read(|cx| view.read(cx).selected_package.is_none()),
                    "disabled catalog command must not dispatch"
                );
            }
        }
        // The same render path must also keep a settings draft mounted while a management
        // commit is pending; disabling is not cancellation or draft replacement.
        view.update_in(window, |view, window, cx| {
            view.open_package("fixture.manager".into());
            let package = view.application.read(cx).packages.packages[0].clone();
            view.settings_draft = Some(PackageSettingsDraft::new(&package, window, cx));
            cx.notify();
        });
        let draft_input = window.read(|cx| {
            view.read(cx).settings_draft.as_ref().expect("draft").inputs["name"].entity_id()
        });
        application.update(window, |app, cx| {
            app.busy = true;
            cx.notify();
        });
        window.run_until_parked();
        window.update(|window, cx| window.draw(cx).clear(cx));
        assert!(window.read(|cx| {
            view.read(cx)
                .settings_draft
                .as_ref()
                .expect("busy draft")
                .inputs["name"]
                .entity_id()
                == draft_input
        }));
    }

    #[gpui_kit::test]
    fn ux_refinement_catalog_short_content_does_not_force_252px(cx: &mut TestAppContext) {
        let temp = tempfile::tempdir().expect("compact cell fixture");
        let (_, _, window) = mount(fixture(&temp), temp.path().join("unused.json"), cx);
        window.update(|window, cx| window.draw(cx).clear(cx));
        let grid = window
            .debug_bounds("extension-catalog-grid")
            .expect("actual cell grid");
        assert!(
            grid.size.height <= px(220.0),
            "short catalog content must not be stretched: {grid:?}"
        );
    }

    #[gpui_kit::test]
    fn extension_catalog_renders_cells_side_by_side_not_prose_rows(cx: &mut TestAppContext) {
        let temp = tempfile::tempdir().expect("owned cells");
        let mut config = fixture(&temp);
        config.extensions.install(nxtools_extensions::ImportPreview::from_json(br#"{"schema_version":1,"id":"fixture.second","name":"Second","version":"1","description":"Short purpose","handlers":[{"id":"search","title":"Search","feature":"quick_launch","entry":"command","keywords":["second"],"action":{"type":"website","url_template":"https://example.invalid/?q={query}"}}]}"#).expect("manifest"),"owned".into(),false).expect("install");
        let (_, _, window) = mount(config, temp.path().join("unused.json"), cx);
        window.update(|window, cx| window.draw(cx).clear(cx));
        let first = window
            .debug_bounds("extension-manage-fixture.manager")
            .expect("first action");
        let second = window
            .debug_bounds("extension-manage-fixture.second")
            .expect("second action");
        assert!(
            (first.center().y - second.center().y).abs() < px(2.0),
            "catalog actions must share a tile row: {first:?} / {second:?}"
        );
        assert!(
            first.right() < second.left(),
            "distinct separated tile hit targets"
        );
        assert_eq!(first.size.height, px(32.0));
    }

    #[gpui_kit::test]
    fn extension_header_actions_paint_real_outlines_in_both_kit_themes(cx: &mut TestAppContext) {
        let temp = tempfile::tempdir().expect("paint fixture");
        let (_, _, window) = mount(
            nxtools_core::AppConfig::default(),
            temp.path().join("unused.json"),
            cx,
        );
        for preference in [
            nxtools_core::ThemePreference::Light,
            nxtools_core::ThemePreference::Dark,
        ] {
            window.update(|_, cx| crate::theme::set_theme_preference(preference, cx));
            window.update(|window, cx| window.draw(cx).clear(cx));
            for selector in [
                "extension-import-open",
                "extension-directory-open",
                "extension-environment-open",
                "extension-chain-edit",
                "extension-recent-activity",
            ] {
                let bounds = window.update(|window, _| {
                    gpui_kit::base::test_support::snapshots(window)
                        .into_iter()
                        .find(|node| {
                            node.path().last() == Some(&gpui_kit::ElementId::from(selector))
                        })
                        .expect("actual official header button")
                        .bounds()
                });
                let painted = window.update(|window, _| {
                    let scale = window.scale_factor();
                    window.painted_quads().iter().any(|quad| {
                        quad.border_color.a > 0.0
                            && quad.border_widths.top.as_f32() > 0.0
                            && (quad.bounds.origin.x.as_f32() / scale - bounds.left().as_f32())
                                .abs()
                                < 1.0
                            && (quad.bounds.origin.y.as_f32() / scale - bounds.top().as_f32()).abs()
                                < 1.0
                            && (quad.bounds.size.width.as_f32() / scale
                                - bounds.size.width.as_f32())
                            .abs()
                                < 1.0
                            && (quad.bounds.size.height.as_f32() / scale
                                - bounds.size.height.as_f32())
                            .abs()
                                < 1.0
                    })
                });
                assert!(
                    painted,
                    "{selector} must paint an actual nontransparent official border in {preference:?}"
                );
                assert_eq!(bounds.size.height, px(32.0));
            }
        }
    }

    #[gpui_kit::test]
    fn extension_outlined_header_retains_official_toolbar_keyboard_roving(cx: &mut TestAppContext) {
        let temp = tempfile::tempdir().expect("keyboard fixture");
        let (_, view, window) = mount(
            nxtools_core::AppConfig::default(),
            temp.path().join("unused.json"),
            cx,
        );
        window.update(|window, _| window.activate_window());
        window.run_until_parked();
        window.update(|window, cx| {
            view.read(cx)
                .search
                .read(cx)
                .focus_handle(cx)
                .focus(window, cx);
        });
        window.simulate_keystrokes("shift-tab");
        let first = window
            .update(|window, cx| window.focused(cx))
            .expect("last header button");
        window.simulate_keystrokes("right");
        let second = window
            .update(|window, cx| window.focused(cx))
            .expect("header next");
        assert_ne!(
            first, second,
            "official Toolbar content still roves real descendant focus"
        );
        window.simulate_keystrokes("right right right right");
        assert_eq!(
            window.update(|window, cx| window.focused(cx)),
            Some(first),
            "five outlined actions wrap within header"
        );
    }

    #[gpui_kit::test]
    fn plugin_directory_header_pointer_keyboard_dispatch_once_and_fit_minimum_default(
        cx: &mut TestAppContext,
    ) {
        let temp = tempfile::tempdir().expect("owned header action");
        let path = temp.path().join("config.json");
        let config = fixture(&temp);
        config.save_to(&path).expect("owned config");
        let before = std::fs::read(&path).expect("before");
        let (bridge, endpoint) = nxtools_ipc::platform_bridge(4);
        endpoint.set_connected_for_contract_test(true);
        let (owner, view, window) =
            mount_with_platform(config, path.clone(), Some(Rc::new(bridge)), cx);
        for width in [744.0, 1060.0] {
            window.simulate_resize(size(px(width), px(430.0)));
            window.update(|window, cx| window.draw(cx).clear(cx));
            let mut previous_right = px(0.0);
            for selector in [
                "extension-import-open",
                "extension-directory-open",
                "extension-environment-open",
                "extension-chain-edit",
                "extension-recent-activity",
            ] {
                let bounds = window.update(|window, _| {
                    gpui_kit::base::test_support::snapshots(window)
                        .into_iter()
                        .find(|node| {
                            node.path().last() == Some(&gpui_kit::ElementId::from(selector))
                        })
                        .unwrap_or_else(|| panic!("actual header control {selector}"))
                        .bounds()
                });
                assert_eq!(bounds.size.height, px(32.0));
                assert!(
                    bounds.left() >= previous_right
                        && bounds.right() <= px(width)
                        && bounds.top() >= px(0.0),
                    "{selector} fits {width}: {bounds:?}"
                );
                previous_right = bounds.right();
            }
        }
        window.simulate_resize(size(px(744.0), px(430.0)));
        for keyboard in [false, true] {
            if keyboard {
                window.update(|window, cx| {
                    view.read(cx)
                        .search
                        .read(cx)
                        .focus_handle(cx)
                        .focus(window, cx)
                });
                window.simulate_keystrokes("shift-tab right right");
                window.simulate_event(gpui_kit::KeyDownEvent {
                    keystroke: Keystroke::parse("space").expect("Space"),
                    is_held: false,
                    prefer_character_input: false,
                });
                window.simulate_event(KeyUpEvent {
                    keystroke: Keystroke::parse("space").expect("release"),
                });
                window.run_until_parked();
            } else {
                click(window, "extension-directory-open");
            }
            let command = endpoint
                .try_receive_command_for_contract_test()
                .expect("actual button dispatch");
            assert!(
                matches!(command.message, nxtools_protocol::Message::PlatformCommand(nxtools_protocol::PlatformCommand::OpenShellTarget(open)) if std::path::Path::new(&open.target) == temp.path().join("extensions"))
            );
            click(window, "extension-directory-open");
            assert!(
                endpoint.try_receive_command_for_contract_test().is_err(),
                "pending disabled control is inert"
            );
            owner.update(window, |app, cx| {
                app.handle_platform_notification(
                    nxtools_ipc::PlatformNotification::CommandResult {
                        correlation_id: command.id,
                        result: nxtools_protocol::PlatformCommandResult {
                            success: true,
                            error: None,
                            native_error_code: None,
                            data: None,
                        },
                    },
                    cx,
                )
            });
            window.run_until_parked();
            assert!(!owner.read_with(window, |app, _| app.opening_directory()));
        }
        assert_eq!(std::fs::read(path).expect("no settings writes"), before);
        assert!(owner.read_with(window, |app, _| app.recent_manual_runs().is_empty()));
    }

    #[gpui_kit::test]
    fn extension_cell_next_step_opens_trust_without_enabling_or_saving(cx: &mut TestAppContext) {
        use gpui_kit::component::WindowExt;
        let temp = tempfile::tempdir().expect("trust cell");
        let config = fixture(&temp);
        let path = temp.path().join("config.json");
        config.save_to(&path).expect("config");
        let original = std::fs::read(&path).expect("bytes");
        let (owner, _, window) = mount(config, path.clone(), cx);
        click(window, "extension-next-fixture.manager");
        window.update(|window, cx| window.draw(cx).clear(cx));
        assert!(
            window.update(|window, cx| window.has_active_dialog(cx)),
            "next step opens actual trust Dialog"
        );
        assert!(!owner.read_with(window, |app, _| app.packages.packages[0].enabled));
        click(window, "extension-trust-cancel");
        assert_eq!(std::fs::read(path).expect("no writes"), original);
    }

    #[gpui_kit::test]
    fn enabled_cell_use_guide_is_explicit_and_never_dispatches_a_run(cx: &mut TestAppContext) {
        let temp = tempfile::tempdir().expect("use cell");
        let mut config = nxtools_core::AppConfig::default();
        config.extensions.install(nxtools_extensions::ImportPreview::from_json(br#"{"schema_version":1,"id":"fixture.use","name":"Use","version":"1","handlers":[{"id":"search","title":"Search","feature":"quick_launch","entry":"command","keywords":["findme"],"action":{"type":"website","url_template":"https://example.invalid/?q={query}"}}]}"#).expect("manifest"),"owned".into(),false).expect("install");
        config
            .extensions
            .set_enabled("fixture.use", true, &[])
            .expect("enable");
        let (owner, _, window) = mount(config, temp.path().join("unused.json"), cx);
        click(window, "extension-next-fixture.use");
        window.update(|window, cx| window.draw(cx).clear(cx));
        assert!(window.debug_bounds("extension-usage-guide").is_some());
        assert!(window.debug_bounds("extension-usage-trigger-search:快速启动 → 输入关键词（可在空格后附查询）：findme").is_some());
        assert!(
            owner.read_with(window, |app, _| app.recent_manual_runs().is_empty()),
            "guide must not execute or guess input"
        );
        let close = window
            .debug_bounds("extension-usage-close")
            .expect("guide action");
        assert!(close.bottom() <= px(430.0));
        click(window, "extension-usage-close");
    }

    // Added before the presentation change. Parent owns serialized Cargo/red verification.
    #[gpui_kit::test]
    fn extension_purpose_and_clipboard_usage_render_in_overview_and_first_detail_page(
        cx: &mut TestAppContext,
    ) {
        let temp = tempfile::tempdir().expect("owned render fixture");
        let mut config = fixture(&temp);
        // Presentation-only fixture: no service, broker, clipboard or executable is started.
        let package = &mut config.extensions.packages[0];
        package.manifest.description = "整理所选文本，保留原始记录".into();
        package.manifest.handlers = vec![
            serde_json::from_value(serde_json::json!({
                "id":"clean","title":"整理文本","feature":"clipboard","entry":"action",
                "action":{"type":"script","entry_file":"fixture.exe"}
            }))
            .expect("clipboard handler"),
        ];
        let (_, _, window) = mount(config, temp.path().join("unused.json"), cx);
        window.update(|window, cx| window.draw(cx).clear(cx));
        let purpose = window
            .debug_bounds("extension-overview-purpose-fixture.manager:整理所选文本，保留原始记录")
            .expect("actual overview must render the author's purpose");
        let next = window
            .debug_bounds("extension-next-fixture.manager")
            .expect("state-specific next step");
        assert!(
            purpose.bottom() <= next.top(),
            "purpose before next-step action"
        );
        assert!(
            next.bottom() <= px(430.0),
            "next step visible in minimum content size"
        );
        assert_eq!(
            window
                .debug_bounds("extension-manage-fixture.manager")
                .expect("manage action")
                .size
                .height,
            px(32.0)
        );
        click(window, "extension-manage-fixture.manager");
        window.update(|window, cx| window.draw(cx).clear(cx));
        let purpose = window
            .debug_bounds("extension-detail-purpose:整理所选文本，保留原始记录")
            .expect("first detail page must begin with purpose, not just access controls");
        let instruction = window
            .debug_bounds(
                "extension-detail-trigger-clean:剪贴板历史 → 条目菜单 → 扩展处理 → 整理文本",
            )
            .expect("first detail page must show the human trigger");
        assert!(purpose.bottom() <= instruction.top());
        assert!(
            window.update(|window, _| gpui_kit::base::test_support::snapshots(window)
                .iter()
                .any(|node| node.path().last() == Some(&"0-0".into()))),
            "usage retains page index zero"
        );
        select_page(window, "运行环境");
        assert!(
            window.update(|window, _| gpui_kit::base::test_support::snapshots(window)
                .iter()
                .any(|node| node.path().last() == Some(&"0-5".into()))),
            "environment remains page five"
        );
    }

    #[gpui_kit::test]
    fn extension_overview_search_matches_author_description_at_real_render(
        cx: &mut TestAppContext,
    ) {
        let temp = tempfile::tempdir().expect("owned render fixture");
        let mut config = fixture(&temp);
        config.extensions.packages[0].manifest.description = "独特用途检索词".into();
        let (_, view, window) = mount(config, temp.path().join("unused.json"), cx);
        // Observe current-frame Button snapshots below, never cached debug-bound presence.
        view.update_in(window, |view, window, cx| {
            view.search.update(cx, |input, cx| {
                input.set_value("独特用途检索词", window, cx)
            });
        });
        window.update(|window, cx| window.draw(cx).clear(cx));
        assert!(
            window.update(|window, _| gpui_kit::base::test_support::snapshots(window)
                .iter()
                .any(|node| node.path().last()
                    == Some(&"extension-manage-fixture.manager".into())
                    && node.role() == Some(gpui_kit::Role::Button))),
            "description-only query must retain the actual current-frame package action"
        );
    }

    #[gpui_kit::test]
    fn workspace_search_and_back_retain_original_inputs_and_package_draft(cx: &mut TestAppContext) {
        let temp = tempfile::tempdir().expect("fixture");
        let config = fixture(&temp);
        let path = temp.path().join("config.json");
        config.save_to(&path).expect("owned config");
        let (_, view, window) = mount(config, path, cx);
        let search = window.read(|cx| view.read(cx).search.entity_id());
        view.update_in(window, |view, window, cx| {
            view.search.update(cx, |input, cx| {
                input.set_value("no matching extension", window, cx)
            })
        });
        window.refresh().expect("filtered empty");
        assert!(
            window
                .debug_bounds("extension-manage-fixture.manager")
                .is_none()
        );
        click(window, "extension-empty-action");
        click(window, "extension-manage-fixture.manager");
        select_page(window, "插件设置");
        click(window, "extension-settings-edit");
        let input = window.read(|cx| {
            view.read(cx).settings_draft.as_ref().expect("draft").inputs["name"].entity_id()
        });
        click(window, "extension-detail-back");
        click(window, "extension-manage-fixture.manager");
        select_page(window, "插件设置");
        assert_eq!(window.read(|cx| view.read(cx).search.entity_id()), search);
        assert_eq!(
            window.read(|cx| view
                .read(cx)
                .settings_draft
                .as_ref()
                .expect("retained draft")
                .inputs["name"]
                .entity_id()),
            input
        );
        click(window, "extension-settings-cancel");
    }

    fn fixture(temp: &tempfile::TempDir) -> nxtools_core::AppConfig {
        let source = temp.path().join("source");
        std::fs::create_dir(&source).expect("owned source");
        std::fs::write(
            source.join("fixture.exe"),
            b"owned identity only; never execute",
        )
        .expect("runtime identity");
        std::fs::write(source.join("extension.json"), br#"{
            "schema_version":1,"id":"fixture.manager","name":"Manager fixture","version":"1",
            "runtime":{"program":"exe"},"files":["fixture.exe"],
            "permissions":[{"id":"network","reason":"Resolve the chosen download"}],
            "settings":[
                {"key":"name","title":"Name","type":"text","default":"initial"},
                {"key":"flag","title":"Flag","type":"bool","default":false},
                {"key":"count","title":"Count","type":"number","default":2,"min":1,"max":8},
                {"key":"mode","title":"Mode","type":"choice","default":"first","options":["first","second"]},
                {"key":"file","title":"File","type":"file","default":""},
                {"key":"folder","title":"Folder","type":"folder","default":""},
                {"key":"token","title":"Token","type":"password"}],
            "handlers":[
                {"id":"one","title":"First","feature":"downloads","entry":"resolve","action":{"type":"script","entry_file":"fixture.exe"}},
                {"id":"two","title":"Second","feature":"downloads","entry":"resolve","action":{"type":"script","entry_file":"fixture.exe"}},
                {"id":"created","title":"Created","feature":"downloads","entry":"created","action":{"type":"script","entry_file":"fixture.exe"}},
                {"id":"completed","title":"Completed","feature":"downloads","entry":"completed","action":{"type":"script","entry_file":"fixture.exe"}},
                {"id":"failed","title":"Failed","feature":"downloads","entry":"failed","action":{"type":"script","entry_file":"fixture.exe"}},
                {"id":"committed","title":"Committed","feature":"clipboard","entry":"committed","action":{"type":"script","entry_file":"fixture.exe"}},
                {"id":"provider","title":"Provider","feature":"quick_launch","entry":"provider","action":{"type":"script","entry_file":"fixture.exe"}}]}
        "#).expect("manifest");
        let preview = nxtools_extensions::ImportPreview::from_directory(&source)
            .expect("preview")
            .stage_into(&temp.path().join("packages"))
            .expect("stage");
        let mut config = nxtools_core::AppConfig::default();
        config
            .extensions
            .install(preview, "owned fixture".into(), false)
            .expect("install");
        config
    }

    fn mount(
        config: nxtools_core::AppConfig,
        path: PathBuf,
        cx: &mut TestAppContext,
    ) -> (
        Entity<ExtensionApplication>,
        Entity<ExtensionManagerView>,
        &mut VisualTestContext,
    ) {
        mount_with_platform(config, path, None, cx)
    }

    fn mount_with_platform(
        config: nxtools_core::AppConfig,
        path: PathBuf,
        platform: Option<Rc<nxtools_ipc::PlatformBridge>>,
        cx: &mut TestAppContext,
    ) -> (
        Entity<ExtensionApplication>,
        Entity<ExtensionManagerView>,
        &mut VisualTestContext,
    ) {
        cx.update(gpui_kit::init);
        cx.executor().allow_parking();
        let center = cx.new(|_| crate::notifications::NotificationCenter::default());
        cx.update(|cx| crate::notifications::register(&center, cx));
        let application = cx
            .new(|_| ExtensionApplication::new(config, path, platform, WeakEntity::new_invalid()));
        application.update(cx, |app, cx| {
            app.bind_ui(WeakEntity::new_invalid(), center, cx)
        });
        let owner = application.clone();
        let slot = Rc::new(RefCell::new(None));
        let retained = slot.clone();
        let (_, window) = cx.add_window_view(move |window, cx| {
            window.resize(size(px(744.0), px(430.0)));
            let view = cx.new(|cx| ExtensionManagerView::new(owner, window, cx));
            *retained.borrow_mut() = Some(view.clone());
            gpui_kit::base::Root::new(view, window, cx)
        });
        window.simulate_resize(size(px(744.0), px(430.0)));
        let view = slot.borrow().clone().expect("view");
        (application, view, window)
    }

    pub(in crate::main_window::extensions_page) fn select_page(
        window: &mut VisualTestContext,
        label: &str,
    ) {
        // Historical callers still name page zero by its previous title; the real Settings
        // item and production label are now usage-first. Preserve the original index seam.
        let label = if label == "信任与启用" {
            "用途与启用"
        } else {
            label
        };
        let index = [
            "用途与启用",
            "命令与工具",
            "插件设置",
            "自动化",
            "来源与能力",
            "运行环境",
        ]
        .iter()
        .position(|title| *title == label)
        .expect("known package page");
        window.update(|window, cx| window.draw(cx).clear(cx));
        let bounds = window.update(|window, _| {
            gpui_kit::base::test_support::snapshots(window)
                .into_iter()
                .find(|snapshot| snapshot.path().last() == Some(&format!("0-{index}").into()))
                .expect("actual released Settings page item")
                .bounds()
        });
        window.simulate_click(bounds.center(), Modifiers::default());
        window.run_until_parked();
        window.refresh().expect("selected actual package page");
    }

    fn click(window: &mut VisualTestContext, selector: &'static str) {
        // Real scrolling and current bounds, not mere cached selector presence.
        top(window);
        for _ in 0..40 {
            window.update(|window, cx| window.draw(cx).clear(cx));
            if let Some(bounds) = window.debug_bounds(selector)
                && bounds.top() >= px(0.0)
                && bounds.bottom() <= px(430.0)
                && bounds.left() >= px(0.0)
                && bounds.right() <= px(744.0)
                && (!(selector.starts_with("extension-opt-in-")
                    || selector.starts_with("extension-secret-clear-")
                    || selector.starts_with("extension-setting-")
                    || matches!(
                        selector,
                        "extension-trust-edit" | "extension-detail-toggle" | "extension-revoke"
                    ))
                    || window
                        .debug_bounds("extension-detail-settings-viewport")
                        .is_some_and(|viewport| {
                            bounds.top() >= viewport.top() && bounds.bottom() <= viewport.bottom()
                        }))
            {
                let point = if selector.starts_with("extension-opt-in-")
                    || selector.starts_with("extension-secret-clear-")
                    || selector.starts_with("extension-setting-")
                {
                    window.update(|window, _| {
                        gpui_kit::base::test_support::snapshots(window)
                            .into_iter()
                            .find(|node| {
                                node.path()
                                    .iter()
                                    .any(|id| id == &gpui_kit::ElementId::from(selector))
                                    && node.role() == Some(gpui_kit::Role::Switch)
                            })
                            .map_or_else(
                                || gpui_kit::point(bounds.left() + px(18.0), bounds.center().y),
                                |node| node.bounds().center(),
                            )
                    })
                } else {
                    bounds.center()
                };
                window.simulate_click(point, Modifiers::default());
                window.run_until_parked();
                return;
            }
            window.simulate_event(gpui_kit::ScrollWheelEvent {
                position: gpui_kit::point(px(650.0), px(350.0)),
                delta: gpui_kit::ScrollDelta::Lines(gpui_kit::point(0.0, -3.0)),
                ..Default::default()
            });
        }
        panic!("manager action {selector} must be reachable at minimum viewport");
    }

    fn top(window: &mut VisualTestContext) {
        window.simulate_event(gpui_kit::ScrollWheelEvent {
            position: gpui_kit::point(px(650.0), px(350.0)),
            delta: gpui_kit::ScrollDelta::Lines(gpui_kit::point(0.0, 100.0)),
            ..Default::default()
        });
        window.refresh().expect("top");
    }

    fn idle(application: &Entity<ExtensionApplication>, window: &mut VisualTestContext) {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            window.run_until_parked();
            if window.read(|cx| !application.read(cx).busy) {
                break;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "owned configuration terminal deadline"
            );
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }

    #[gpui_kit::test]
    fn w9_manager_pointer_trust_enable_and_revoke_are_separate_explicit_operations(
        cx: &mut TestAppContext,
    ) {
        let temp = tempfile::tempdir().expect("fixture");
        let config = fixture(&temp);
        let path = temp.path().join("config.json");
        config.save_to(&path).expect("owned config");
        let (application, view, window) = mount(config, path.clone(), cx);
        click(window, "extension-manage-fixture.manager");
        window.refresh().expect("disclosure");
        assert_eq!(
            TRUST_WARNING,
            "信任后，此插件可使用当前用户的全部权限。以下权限由插件作者声明，并非系统访问限制。"
        );
        for _ in 0..20 {
            window.update(|window, cx| window.draw(cx).clear(cx));
            if window
                .debug_bounds("extension-permission-warning")
                .is_some_and(|bounds| bounds.top() >= px(0.0) && bounds.bottom() <= px(430.0))
            {
                break;
            }
            window.simulate_event(gpui_kit::ScrollWheelEvent {
                position: gpui_kit::point(px(650.0), px(350.0)),
                delta: gpui_kit::ScrollDelta::Lines(gpui_kit::point(0.0, -3.0)),
                ..Default::default()
            });
        }
        assert!(
            window
                .debug_bounds("extension-permission-warning")
                .is_some()
        );
        click(window, "extension-trust-edit");
        assert!(
            window.read(|cx| !view.read(cx).runtime_path.read(cx).value().is_empty()),
            "staged EXE is resolved by configuration, not a manual trust input"
        );
        click(window, "extension-trust-cancel");
        assert!(
            nxtools_core::AppConfig::load_from(&path)
                .expect("cancel")
                .extensions
                .packages[0]
                .trust
                .is_none()
        );
        top(window);
        click(window, "extension-trust-edit");
        assert!(window.read(|cx| {
            application
                .read(cx)
                .packages
                .runtime_executable("fixture.manager")
                .expect("resolved staged EXE")
                .is_some()
        }));
        click(window, "extension-approve-runtime");
        idle(&application, window);
        let package = &nxtools_core::AppConfig::load_from(&path)
            .expect("approval")
            .extensions
            .packages[0];
        assert!(package.trust.is_some());
        assert!(!package.enabled, "approval is not execution enablement");
        top(window);
        click(window, "extension-detail-toggle");
        idle(&application, window);
        assert!(
            nxtools_core::AppConfig::load_from(&path)
                .expect("explicit enable")
                .extensions
                .packages[0]
                .enabled
        );
        top(window);
        click(window, "extension-revoke");
        idle(&application, window);
        let package = &nxtools_core::AppConfig::load_from(&path)
            .expect("revocation")
            .extensions
            .packages[0];
        assert!(package.trust.is_none());
        assert!(!package.enabled);
    }

    #[gpui_kit::test]
    fn w9_manager_settings_pointer_keyboard_cancel_validation_and_single_explicit_save(
        cx: &mut TestAppContext,
    ) {
        let temp = tempfile::tempdir().expect("fixture");
        let config = fixture(&temp);
        let path = temp.path().join("config.json");
        config.save_to(&path).expect("owned config");
        let (application, view, window) = mount(config, path.clone(), cx);
        click(window, "extension-manage-fixture.manager");
        select_page(window, "插件设置");
        click(window, "extension-settings-edit");
        view.update_in(window, |view, window, cx| {
            let draft = view.settings_draft.as_ref().expect("draft");
            assert!(draft.inputs["token"].read(cx).value().is_empty());
            assert!(
                draft.payload(cx).expect("defaults").1.is_empty(),
                "unchanged password omitted"
            );
            draft.inputs["name"].update(cx, |input, cx| input.set_value("cancelled", window, cx));
            draft.inputs["token"].update(cx, |input, cx| {
                input.set_value("owned draft secret", window, cx)
            });
        });
        // Draft survives actual page navigation; Save/Cancel are outside Settings scrolling.
        let draft_input = window.read(|cx| {
            view.read(cx).settings_draft.as_ref().expect("draft").inputs["name"].entity_id()
        });
        select_page(window, "自动化");
        window.refresh().expect("draft actions remain reachable");
        for selector in ["extension-settings-save", "extension-settings-cancel"] {
            let bounds = window.debug_bounds(selector).expect("pinned draft action");
            assert!(
                bounds.top() >= px(0.0)
                    && bounds.bottom() <= px(430.0)
                    && bounds.right() <= px(744.0)
            );
        }
        select_page(window, "插件设置");
        assert_eq!(
            window.read(|cx| view
                .read(cx)
                .settings_draft
                .as_ref()
                .expect("retained draft")
                .inputs["name"]
                .entity_id()),
            draft_input
        );
        click(window, "extension-secret-clear-token");
        window.read(|cx| {
            let draft = view
                .read(cx)
                .settings_draft
                .as_ref()
                .expect("secret clear draft");
            assert_eq!(
                draft.payload(cx).expect("explicit clear").1.get("token"),
                Some(&None)
            );
        });
        click(window, "extension-settings-cancel");
        assert_eq!(
            nxtools_core::AppConfig::load_from(&path)
                .expect("cancel")
                .extensions
                .packages[0]
                .settings["name"],
            "initial"
        );
        assert!(window.read(|cx| view.read(cx).settings_draft.is_none()));
        top(window);
        click(window, "extension-settings-edit");
        click(window, "extension-setting-flag");
        window.simulate_keystrokes("space");
        window.simulate_event(KeyUpEvent {
            keystroke: Keystroke::parse("space").expect("Space release"),
        });
        window.run_until_parked();
        assert!(
            !window.read(|cx| view.read(cx).settings_draft.as_ref().expect("draft").bools["flag"]),
            "pointer then Space toggles once each"
        );
        view.update_in(window, |view, window, cx| {
            let draft = view.settings_draft.as_ref().expect("draft");
            draft.inputs["name"].update(cx, |input, cx| input.set_value("saved", window, cx));
            draft.inputs["count"].update(cx, |input, cx| input.set_value("99", window, cx));
        });
        click(window, "extension-settings-save");
        assert!(window.read(|cx| view.read(cx).draft_error.is_some()));
        assert_eq!(
            nxtools_core::AppConfig::load_from(&path)
                .expect("invalid no save")
                .extensions
                .packages[0]
                .settings["name"],
            "initial"
        );
        view.update_in(window, |view, window, cx| {
            view.settings_draft.as_ref().expect("draft").inputs["count"]
                .update(cx, |input, cx| input.set_value("3", window, cx))
        });
        click(window, "extension-settings-save");
        idle(&application, window);
        let config = nxtools_core::AppConfig::load_from(&path).expect("saved once");
        assert_eq!(config.extensions.packages[0].settings["name"], "saved");
        assert_eq!(config.extensions.packages[0].settings["count"], 3);
        assert!(!config.extensions.packages[0].settings.contains_key("token"));
        assert!(
            window.read(|cx| view.read(cx).settings_draft.is_none()),
            "successful save retires draft, preventing replay"
        );
        assert!(
            !std::fs::read_to_string(path)
                .expect("config bytes")
                .contains("owned draft secret")
        );
    }

    #[gpui_kit::test]
    fn w9_manager_event_provider_opt_ins_are_independent_and_keyboard_accessible(
        cx: &mut TestAppContext,
    ) {
        let temp = tempfile::tempdir().expect("fixture");
        let config = fixture(&temp);
        let path = temp.path().join("config.json");
        config.save_to(&path).expect("owned config");
        let (application, view, window) = mount(config, path.clone(), cx);
        click(window, "extension-manage-fixture.manager");
        select_page(window, "自动化");
        for selector in [
            "extension-opt-in-fixture.manager-created",
            "extension-opt-in-fixture.manager-completed",
            "extension-opt-in-fixture.manager-failed",
            "extension-opt-in-fixture.manager-committed",
            "extension-opt-in-fixture.manager-provider",
        ] {
            click(window, selector);
            idle(&application, window);
        }
        let config = nxtools_core::AppConfig::load_from(&path).expect("subscriptions");
        assert_eq!(config.extensions.packages[0].enabled_events.len(), 4);
        assert!(
            config.extensions.packages[0]
                .enabled_providers
                .contains("provider")
        );
        assert!(!config.extensions.packages[0].enabled);
        top(window);
        for _ in 0..40 {
            window.update(|window, cx| {
                window.draw(cx).clear(cx);
            });
            if window
                .debug_bounds("extension-opt-in-fixture.manager-provider")
                .is_some_and(|bounds| {
                    window
                        .debug_bounds("extension-detail-settings-viewport")
                        .is_some_and(|viewport| {
                            bounds.top() >= viewport.top() && bounds.bottom() <= viewport.bottom()
                        })
                })
            {
                break;
            }
            window.simulate_event(gpui_kit::ScrollWheelEvent {
                position: gpui_kit::point(px(650.0), px(350.0)),
                delta: gpui_kit::ScrollDelta::Lines(gpui_kit::point(0.0, -3.0)),
                ..Default::default()
            });
        }
        window.update(|window, cx| {
            window.draw(cx).clear(cx);
            let scope =
                view.read(cx).control_scopes.borrow()["extension-opt-in-fixture.manager-provider"]
                    .clone();
            scope.focus(window, cx);
            for _ in 0..100 {
                window.focus_next(cx);
                window.draw(cx).clear(cx);
                if scope.contains_focused(window, cx) && !scope.is_focused(window) {
                    break;
                }
            }
            assert!(
                scope.contains_focused(window, cx) && !scope.is_focused(window),
                "Tab reaches actual provider control"
            );
        });
        window.simulate_event(gpui_kit::KeyDownEvent {
            keystroke: Keystroke::parse("space").expect("provider press"),
            is_held: false,
            prefer_character_input: false,
        });
        window.simulate_event(KeyUpEvent {
            keystroke: Keystroke::parse("space").expect("provider release"),
        });
        idle(&application, window);
        assert!(
            !nxtools_core::AppConfig::load_from(&path)
                .expect("provider off")
                .extensions
                .packages[0]
                .enabled_providers
                .contains("provider")
        );
    }

    #[gpui_kit::test]
    fn w9_manager_chain_exact_refs_reorder_cancel_and_save_owned_config(cx: &mut TestAppContext) {
        let temp = tempfile::tempdir().expect("fixture");
        let mut config = fixture(&temp);
        let package = &mut config.extensions.packages[0];
        let executable = package
            .code_directory
            .as_ref()
            .expect("code")
            .join("fixture.exe");
        package.trust = Some(nxtools_extensions::TrustApproval {
            content_hash: package.content_hash.clone(),
            runtime: nxtools_extensions::RuntimeIdentity::inspect(&executable).expect("identity"),
        });
        // Configure exact chain references while disabled; runtime enablement remains separate.
        package.enabled = false;
        let original = vec![
            HandlerReference {
                package_id: "fixture.manager".into(),
                handler_id: "one".into(),
            },
            HandlerReference {
                package_id: "fixture.manager".into(),
                handler_id: "two".into(),
            },
        ];
        config
            .extensions
            .set_resolver_chain("example.test".into(), original.clone())
            .expect("configured chain");
        let path = temp.path().join("config.json");
        config.save_to(&path).expect("owned config");
        let (application, view, window) = mount(config, path.clone(), cx);
        click(window, "extension-chain-edit");
        view.update_in(window, |view, window, cx| {
            view.chain_host
                .update(cx, |input, cx| input.set_value("EXAMPLE.TEST", window, cx))
        });
        click(window, "extension-chain-load");
        click(window, "extension-chain-up-1");
        assert_eq!(
            window.read(
                |cx| view.read(cx).chain_draft.as_ref().expect("chain").route[0]
                    .handler_id
                    .clone()
            ),
            "two"
        );
        top(window);
        click(window, "extension-chain-cancel");
        assert_eq!(
            nxtools_core::AppConfig::load_from(&path)
                .expect("cancel no write")
                .extensions
                .resolver_chains["example.test"],
            original
        );
        top(window);
        click(window, "extension-chain-edit");
        view.update_in(window, |view, window, cx| {
            view.chain_host
                .update(cx, |input, cx| input.set_value("example.test", window, cx))
        });
        click(window, "extension-chain-load");
        click(window, "extension-chain-remove-1");
        view.update_in(window, |view, window, cx| {
            let choice = view.chain_draft.as_ref().expect("chain").choice.clone();
            choice.update(cx, |choice, cx| {
                choice.set_selected_value(&"fixture.manager / two".to_owned(), window, cx)
            });
        });
        click(window, "extension-chain-add");
        assert_eq!(
            window.read(|cx| view
                .read(cx)
                .chain_draft
                .as_ref()
                .expect("restored chain")
                .route
                .clone()),
            original
        );
        click(window, "extension-chain-down-0");
        window.update(|window, cx| {
            window.draw(cx).clear(cx);
            let scope = view.read(cx).control_scopes.borrow()["extension-chain-down-0"].clone();
            scope.focus(window, cx);
            window.focus_next(cx);
            window.draw(cx).clear(cx);
            assert!(
                scope.contains_focused(window, cx),
                "Tab reaches actual chain action"
            );
        });
        window.simulate_event(gpui_kit::KeyDownEvent {
            keystroke: Keystroke::parse("space").expect("chain press"),
            is_held: false,
            prefer_character_input: false,
        });
        window.simulate_event(KeyUpEvent {
            keystroke: Keystroke::parse("space").expect("chain keyboard release"),
        });
        window.run_until_parked();
        assert_eq!(
            window.read(|cx| view
                .read(cx)
                .chain_draft
                .as_ref()
                .expect("keyboard chain")
                .route
                .clone()),
            original
        );
        click(window, "extension-chain-down-0");
        top(window);
        click(window, "extension-chain-save");
        idle(&application, window);
        let reversed = vec![original[1].clone(), original[0].clone()];
        assert_eq!(
            nxtools_core::AppConfig::load_from(&path)
                .expect("ordered save")
                .extensions
                .resolver_chains["example.test"],
            reversed
        );
    }
    #[gpui_kit::test]
    fn w9_event_circuit_retry_is_reachable_and_preserves_saved_subscription(
        cx: &mut TestAppContext,
    ) {
        let temp = tempfile::tempdir().expect("fixture");
        let config = fixture(&temp);
        let path = temp.path().join("config.json");
        config.save_to(&path).expect("config");
        let before = std::fs::read(&path).expect("bytes");
        let (application, _view, window) = mount(config, path.clone(), cx);
        let control = crate::extensions::committed_events::EventControl::default();
        control.pause_for_test("fixture.manager", "committed");
        application.update(window, |app, cx| {
            app.bind_event_control_for_test(control.clone());
            cx.notify();
        });
        click(window, "extension-manage-fixture.manager");
        select_page(window, "自动化");
        click(window, "extension-event-retry-fixture.manager-committed");
        assert!(!control.is_paused("fixture.manager", "committed"));
        assert_eq!(
            std::fs::read(&path).expect("unchanged config"),
            before,
            "retry neither trusts/enables nor saves subscriptions"
        );
        assert!(window.read(|cx| {
            crate::notifications::current(cx)
                .expect("shared center")
                .read(cx)
                .items()
                .iter()
                .any(|notice| notice.message.contains("不重放旧事件"))
        }));
    }
}
