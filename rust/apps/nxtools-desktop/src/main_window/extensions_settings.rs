//! W9 presentation only. Application owns validation, trust, credentials and atomic saves.
//! Released Kit 0.7.1 Settings public slots retain its search/navigation/scroll owner;
//! Form drafts retain InputState identity and never preload a password.
use super::*;
use gpui_kit::component::description_list::DescriptionList;
use gpui_kit::component::input::NumberInput;
use gpui_kit::component::list::ListItem;
use gpui_kit::component::select::{Select, SelectItem, SelectState};
use gpui_kit::component::setting::SettingField;
use gpui_kit::component::switch::Switch;
use gpui_kit::component::{Disableable, IndexPath};
use gpui_kit::{AnyElement, App, SharedString, WeakEntity};
use nxtools_extensions::{
    HandlerReference, InstalledPackage, PackageRegistry, Setting, SettingType,
};
use std::collections::{BTreeMap, HashSet};

type SettingsPayload = (
    BTreeMap<String, serde_json::Value>,
    BTreeMap<String, Option<String>>,
);

#[path = "extensions_page/setting_paths.rs"]
mod setting_paths;

#[derive(Clone)]
struct SettingChoice {
    value: String,
    label: String,
}
impl SelectItem for SettingChoice {
    type Value = String;
    fn title(&self) -> SharedString {
        self.label.clone().into()
    }
    fn value(&self) -> &String {
        &self.value
    }
}

pub(super) struct PackageSettingsDraft {
    draft_id: uuid::Uuid,
    path_pending: BTreeMap<String, uuid::Uuid>,
    path_revisions: BTreeMap<String, Rc<std::cell::Cell<u64>>>,
    _path_subscriptions: Vec<Subscription>,
    id: String,
    content_hash: String,
    fields: Vec<Setting>,
    inputs: BTreeMap<String, Entity<InputState>>,
    choices: BTreeMap<String, Entity<SelectState<Vec<SettingChoice>>>>,
    bools: BTreeMap<String, bool>,
    submitted: bool,
    clear_secrets: HashSet<String>,
}

impl PackageSettingsDraft {
    pub(super) fn package_id(&self) -> &str {
        &self.id
    }
    pub(super) fn new(
        package: &InstalledPackage,
        window: &mut Window,
        cx: &mut Context<ExtensionManagerView>,
    ) -> Self {
        let mut draft = Self {
            draft_id: uuid::Uuid::new_v4(),
            path_pending: BTreeMap::new(),
            path_revisions: BTreeMap::new(),
            _path_subscriptions: Vec::new(),
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
                        .position(|option| option.value() == selected)
                        .map(|row| IndexPath::default().row(row));
                    let options = options
                        .iter()
                        .map(|option| SettingChoice {
                            value: option.value().into(),
                            label: option.label().into(),
                        })
                        .collect::<Vec<_>>();
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
                    if matches!(field, SettingType::File { .. } | SettingType::Folder { .. }) {
                        let revision = Rc::new(std::cell::Cell::new(0u64));
                        let observed = revision.clone();
                        // Count every user edit, even A-B-A within one application turn.
                        // Silent set_value hydration does not emit Change; the final text
                        // comparison still rejects a different programmatically set value.
                        draft._path_subscriptions.push(cx.subscribe(
                            &input,
                            move |_, _, event: &gpui_kit::component::input::InputEvent, cx| {
                                if matches!(event, gpui_kit::component::input::InputEvent::Change) {
                                    observed.set(observed.get().wrapping_add(1));
                                    cx.notify();
                                }
                            },
                        ));
                        draft.path_revisions.insert(setting.key.clone(), revision);
                    }
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
    saved_hosts: Entity<SelectState<Vec<String>>>,
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
    pub(super) fn detail_content(
        &self,
        cx: &mut Context<Self>,
    ) -> Option<(String, AnyElement, AnyElement)> {
        use gpui_kit::component::tab::{Tab, TabBar};
        let id = self.selected_package.as_deref()?;
        let state = self.application.read(cx);
        let package = state
            .packages
            .packages
            .iter()
            .find(|p| p.manifest.id == id)?
            .clone();
        let busy = state.busy;
        let view = cx.entity().downgrade();
        let page = self.detail_initial_page;
        let has_commands = package
            .manifest
            .handlers
            .iter()
            .any(|h| matches!(h.feature.as_str(), "quick_launch" | "tools"));
        let has_automation =
            package.manifest.handlers.iter().any(|h| {
                usage::is_event(h) || (h.feature == "quick_launch" && h.entry == "provider")
            });
        let pages = [
            (0, "怎么用", true),
            (1, "命令与工具", has_commands),
            (2, "插件设置", !package.manifest.settings.is_empty()),
            (3, "自动化", has_automation),
            (4, "来源", true),
        ];
        let routes = pages
            .into_iter()
            .filter(|(_, _, available)| *available)
            .collect::<Vec<_>>();
        let page = if routes.iter().any(|(route, _, _)| *route == page) {
            page
        } else {
            0
        };
        let route_ix = routes
            .iter()
            .position(|(route, _, _)| *route == page)
            .unwrap_or(0);
        let tab_view = view.clone();
        let tab_id = id.to_owned();
        let tabs = TabBar::new("extension-detail-tabs")
            .underline()
            .small()
            .selected_index(route_ix)
            .children(routes.iter().map(|(_, label, _)| Tab::new().label(*label)))
            .on_click(move |index, window, cx| {
                let Some((route, _, _)) = routes.get(*index).copied() else {
                    return;
                };
                let tab_view = tab_view.clone();
                let id = tab_id.clone();
                window.defer(cx, move |_, cx| {
                    let _ = tab_view.update(cx, |view, cx| {
                        if view.selected_package.as_deref() == Some(id.as_str()) {
                            view.detail_initial_page = route;
                            cx.notify();
                        }
                    });
                });
            });
        let groups = match page {
            1 => vec![self.package_commands(&package, busy, cx)],
            2 => {
                let mut groups = Vec::new();
                if self
                    .settings_draft
                    .as_ref()
                    .is_some_and(|draft| draft.id == id)
                {
                    groups.push(self.settings_form(&package, busy, cx));
                } else {
                    let edit_view = view.clone();
                    let edit_id = id.to_owned();
                    groups.push(
                        SettingGroup::new().item(SettingItem::render(move |_, _, _| {
                            view_button(
                                "extension-settings-edit",
                                "编辑设置",
                                busy,
                                edit_view.clone(),
                                {
                                    let id = edit_id.clone();
                                    move |view, window, cx| {
                                        view.open_settings(id.clone(), window, cx)
                                    }
                                },
                            )
                        })),
                    );
                }
                groups.push(self.storage_entry_group(&package, cx));
                groups
            }
            3 => vec![self.handler_controls(&package, busy, cx)],
            4 => {
                let mut groups =
                    vec![SettingGroup::new().item(self.package_actions(&package, busy, cx))];
                if package.manifest.runtime.is_some() {
                    groups.push(SettingGroup::new().item(warning_item()));
                    groups.push(self.package_environment(&package, cx));
                }
                groups.push(package_metadata(&package));
                groups
            }
            _ => vec![self.package_usage(&package, cx)],
        };
        let hidden = gpui_kit::StyleRefinement {
            display: Some(gpui_kit::Display::None),
            ..Default::default()
        };
        let settings = Settings::new(format!(
            "extension-package-detail-{id}-{page}-{}",
            self.detail_navigation_revision
        ))
        .sidebar_width(px(0.0))
        .sidebar_size_range(px(0.0)..px(0.0))
        .sidebar_style(&hidden)
        .header_style(&hidden)
        .pages([SettingPage::new("扩展详情")
            .resettable(false)
            .header_style(&hidden)
            .groups(groups)]);
        let mut body = div()
            .size_full()
            .min_w_0()
            .min_h_0()
            .flex()
            .flex_col()
            .gap_2()
            .debug_selector(|| "extension-usage-guide".into())
            .child(tabs)
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .debug_selector(|| "extension-detail-settings-viewport".into())
                    .child(settings),
            );
        if let Some(message) = self.draft_error.clone() {
            body = body.child(Alert::error("extension-operation-validation", message).small());
        }
        let mut footer = div().w_full().flex().justify_end().items_center().gap_2();
        if page == 2
            && self
                .settings_draft
                .as_ref()
                .is_some_and(|draft| draft.id == id)
        {
            footer = footer.child(self.settings_actions(&package, busy, cx));
        } else {
            footer = footer.child(view_button(
                "extension-detail-back",
                "关闭",
                false,
                view.clone(),
                |view, _, _| view.selected_package = None,
            ));
            if page == 0 {
                let action_id = id.to_owned();
                let key = format!("extension-next-{id}");
                let action = match self.attention_reason(&package, cx) {
                    Some(catalog::AttentionReason::Trust) => Some(view_button(
                        key,
                        "检查信任",
                        busy,
                        view,
                        move |view, window, cx| view.open_trust(action_id.clone(), window, cx),
                    )),
                    Some(catalog::AttentionReason::Settings) => Some(view_button(
                        key,
                        "去设置",
                        busy,
                        view,
                        move |view, window, cx| view.open_settings(action_id.clone(), window, cx),
                    )),
                    Some(catalog::AttentionReason::Paused) => Some(view_button(
                        key,
                        "查看自动触发",
                        busy,
                        view,
                        |view, _, _| view.detail_initial_page = 3,
                    )),
                    _ if !package.enabled => Some(app_button(
                        key,
                        "启用扩展",
                        busy,
                        self.application.downgrade(),
                        move |app, cx| app.set_enabled(action_id.clone(), true, cx),
                    )),
                    _ => None,
                };
                if let Some(action) = action {
                    footer = footer.child(action.primary());
                }
            }
        }
        Some((
            package.manifest.name,
            body.into_any_element(),
            footer.into_any_element(),
        ))
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
            actions = actions.child(view_button(
                format!("extension-remove-{remove_id}"),
                "移除扩展",
                busy,
                view.clone(),
                move |view, window, cx| view.confirm_removal(remove_id.clone(), window, cx),
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
                            .placeholder("当前值无效，请重新选择")
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
                SettingType::File { .. } | SettingType::Folder { .. } => {
                    let input = draft.inputs[&key].clone();
                    let pending = draft.path_pending.contains_key(&key);
                    let key = key.clone();
                    let view = view.clone();
                    let kind = if matches!(setting.field, SettingType::Folder { .. }) {
                        "folder"
                    } else {
                        "file"
                    };
                    SettingField::render(move |_, _, _| {
                        let key = key.clone();
                        div()
                            .w_full()
                            .min_w_0()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div().flex_1().min_w_0().child(
                                    Input::new(&input).aria_label(label.clone()).disabled(busy),
                                ),
                            )
                            .child(view_button(
                                format!("extension-setting-browse-{key}"),
                                if pending { "选择中…" } else { "浏览" },
                                busy || pending,
                                view.clone(),
                                move |view, window, cx| {
                                    view.pick_setting_path(key.clone(), kind, window, cx)
                                },
                            ))
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
            // Custom fields do not inherit the built-in field widths. Give the
            // original controls the full row instead of collapsing them beside
            // the title in the constrained detail pane; switches stay compact.
            if !matches!(setting.field, SettingType::Bool { .. }) {
                item = item.layout(gpui_kit::Axis::Vertical);
            }
            if !description.is_empty() {
                item = item.description(description.clone());
            }
            if matches!(
                setting.field,
                SettingType::File { .. } | SettingType::Folder { .. }
            ) && description.is_empty()
            {
                item = item.description("浏览选择现有路径；也可手动输入，保存后生效。");
            }
            items.push(item);
            let invalid_choice = matches!(setting.field, SettingType::Choice { .. })
                && draft.choices[&setting.key]
                    .read(cx)
                    .selected_value()
                    .is_none();
            let invalid_text = self.draft_error.is_some()
                && setting.validation.is_some()
                && draft.inputs.get(&setting.key).is_some_and(|input| {
                    !setting.accepts(&serde_json::Value::String(
                        input.read(cx).value().to_string(),
                    ))
                });
            if invalid_choice || invalid_text {
                let selector = format!("extension-setting-error-{}", setting.key);
                let message = if invalid_choice {
                    "当前选项已不可用，请明确重新选择"
                } else {
                    "请填写非空值，并满足声明的长度和完整匹配规则"
                };
                items.push(SettingItem::render(move |_, _, _| {
                    div()
                        .w_full()
                        .debug_selector({
                            let selector = selector.clone();
                            move || selector.clone()
                        })
                        .child(Alert::error(selector.clone(), message).small())
                }));
            }
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
        let state = self.application.read(cx);
        let package_id = package.manifest.id.as_str();
        let mut handlers = package.manifest.handlers.iter().collect::<Vec<_>>();
        handlers.sort_by_key(|handler| {
            !usage::is_event(handler) || !state.event_paused(package_id, &handler.id)
        });
        for handler in handlers {
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
            let latest = self.application.read(cx).latest_event(&id, &handler_id);
            let retry_owner = owner.clone();
            let retry_id = id.clone();
            let retry_handler = handler_id.clone();
            let recovery_view = cx.entity().downgrade();
            let recovery_id = id.clone();
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
                            .w_full().min_w_0().flex().flex_col().items_start().gap_2()
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
                            .when_some(latest.as_ref(), |row, latest| row.child(div().text_sm().child(latest.summary())))
                            .when(latest.as_ref().is_some_and(|item| !item.outcome.success()), |row| {
                                let id = recovery_id.clone();
                                row.child(view_button(format!("extension-event-environment-{retry_id}-{retry_handler}"), "检查运行环境", busy, recovery_view.clone(), move |view, window, cx| view.open_environment(Some(id.clone()), window, cx)))
                            })
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
                                            "恢复自动触发",
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

    pub(super) fn open_chain(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let view = self;
        let registry = view.application.read(cx).packages.clone();
        let candidates = candidates(&registry);
        let labels = candidates.iter().map(reference_label).collect::<Vec<_>>();
        let choice = cx.new(|cx| SelectState::new(labels, None, window, cx));
        let hosts = registry
            .resolver_chains
            .keys()
            .chain(registry.resolver_preferences.keys())
            .cloned()
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let saved_hosts = cx.new(|cx| SelectState::new(hosts, None, window, cx));
        view.chain_draft = Some(ChainDraft {
            route: Vec::new(),
            candidates,
            choice,
            saved_hosts,
        });
        view.draft_error = None;
        view.chain_host.update(cx, |input, cx| {
            input.set_value("", window, cx);
            input.focus(window, cx);
        });
    }

    pub(super) fn render_chain(&self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let draft = self.chain_draft.as_ref().expect("chain editor");
        let registry = self.application.read(cx).packages.clone();
        let busy = self.application.read(cx).busy;
        let view = cx.entity().downgrade();
        let host = self.chain_host.clone();
        let choice = draft.choice.clone();
        let saved_hosts = draft.saved_hosts.clone();
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
            let saved_choice = saved_hosts.clone();
            let form = Form::new()
                .child(
                    Field::new().label("已保存的域名").child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                Select::new(&saved_hosts)
                                    .accessibility_label("已保存的解析链域名")
                                    .disabled(busy),
                            )
                            .child(view_button(
                                "extension-chain-open-saved",
                                "打开所选配置",
                                busy,
                                view.clone(),
                                move |view, window, cx| {
                                    let Some(host) =
                                        saved_choice.read(cx).selected_value().cloned()
                                    else {
                                        view.draft_error = Some("请选择一个已保存的域名".into());
                                        return;
                                    };
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
                                                .map(|item| vec![item])
                                        })
                                        .unwrap_or_default();
                                    if let Some(draft) = &mut view.chain_draft {
                                        draft.route = route;
                                    }
                                    view.chain_host
                                        .update(cx, |input, cx| input.set_value(host, window, cx));
                                    view.draft_error = None;
                                },
                            )),
                    ),
                )
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
    fn remediation_usage_settings_opens_editable_settings_directly(cx: &mut TestAppContext) {
        let temp = tempfile::tempdir().expect("owned settings navigation");
        let path = temp.path().join("config.json");
        let config = fixture(&temp);
        config.save_to(&path).expect("owned config");
        let before = std::fs::read(&path).expect("config before navigation");
        let (_, view, window) = mount(config, path.clone(), cx);
        view.update_in(window, |view, window, cx| {
            view.open_settings("fixture.manager".into(), window, cx);
            cx.notify();
        });
        window.run_until_parked();
        window.update(|window, cx| {
            window.activate_window();
            window.draw(cx).clear(cx);
        });
        let input = window.update(|window, _| {
            gpui_kit::base::test_support::snapshots(window)
                .into_iter()
                .find(|node| {
                    node.role() == Some(gpui_kit::Role::TextInput) && node.label() == Some("Name")
                })
                .expect("actual named settings Input shown without another Edit click")
        });
        let viewport = window
            .debug_bounds("extension-detail-settings-viewport")
            .expect("settings viewport");
        assert!(input.visible());
        assert!(
            input.bounds().size.width >= px(200.0),
            "editable Input needs usable width, got {:?}",
            input.bounds()
        );
        assert_ne!(
            input.disabled(),
            Some(true),
            "direct settings stays editable"
        );
        assert!(window.update(|window, cx| !window.has_active_dialog(cx)));
        assert!(
            input.bounds().top() >= viewport.top() && input.bounds().bottom() <= viewport.bottom()
        );
        window.simulate_click(input.bounds().center(), Modifiers::default());
        view.update_in(window, |view, window, cx| {
            use gpui_kit::Focusable as _;
            assert!(
                view.settings_draft.as_ref().expect("draft").inputs["name"]
                    .read(cx)
                    .focus_handle(cx)
                    .is_focused(window),
                "pointer must focus the original draft Input after closing usage"
            );
        });
        // The focused Input installs its platform text handler during paint.
        window.update(|window, cx| window.draw(cx).clear(cx));
        window.simulate_keystrokes("ctrl-a");
        window.simulate_input("edited through direct settings");
        window.run_until_parked();
        assert_eq!(
            view.read_with(window, |view, cx| view
                .settings_draft
                .as_ref()
                .expect("editable draft")
                .inputs["name"]
                .read(cx)
                .value()
                .to_string()),
            "edited through direct settings"
        );
        assert_eq!(
            std::fs::read(path).expect("navigation and editing do not save"),
            before
        );
    }
    #[gpui_kit::test]
    fn remediation_setting_path_same_turn_aba_edit_rejects_late_selection(cx: &mut TestAppContext) {
        use gpui_kit::EntityInputHandler as _;
        let temp = tempfile::tempdir().expect("owned ABA path fixture");
        let path = temp.path().join("config.json");
        let config = fixture(&temp);
        config.save_to(&path).expect("owned config");
        let before = std::fs::read(&path).expect("config before picker");
        let (bridge, endpoint) = nxtools_ipc::platform_bridge(8);
        endpoint.set_connected_for_contract_test(true);
        let (owner, view, window) =
            mount_with_platform(config, path.clone(), Some(Rc::new(bridge)), cx);
        let original = "C:\\Synthetic\\original.txt";
        let edited = "C:\\Synthetic\\edited.txt";
        view.update_in(window, |view, window, cx| {
            view.open_settings("fixture.manager".into(), window, cx);
            view.settings_draft.as_ref().expect("draft").inputs["file"]
                .update(cx, |input, cx| input.set_value(original, window, cx));
        });
        window.run_until_parked();
        window.update(|window, cx| window.draw(cx).clear(cx));
        view.update_in(window, |view, window, cx| {
            view.begin_setting_path("file".into(), "file", 123, window, cx)
        });
        window.run_until_parked();
        let request = endpoint
            .try_receive_command_for_contract_test()
            .expect("actual pending picker request");
        view.update_in(window, |view, window, cx| {
            view.settings_draft.as_ref().expect("current draft").inputs["file"].update(
                cx,
                |input, cx| {
                    input.replace_text_in_range(
                        Some(0..original.encode_utf16().count()),
                        edited,
                        window,
                        cx,
                    );
                    input.replace_text_in_range(
                        Some(0..edited.encode_utf16().count()),
                        original,
                        window,
                        cx,
                    );
                },
            );
        });
        window.run_until_parked();
        assert_eq!(
            view.read_with(window, |view, cx| view
                .settings_draft
                .as_ref()
                .expect("draft")
                .inputs["file"]
                .read(cx)
                .value()
                .to_string()),
            original
        );
        owner.update_in(window, |app, _, cx| {
            app.handle_platform_notification(
                nxtools_ipc::PlatformNotification::CommandResult {
                    correlation_id: request.id,
                    result: nxtools_protocol::PlatformCommandResult {
                        success: true,
                        error: None,
                        native_error_code: None,
                        data: Some("C:\\Synthetic\\late.txt".into()),
                    },
                },
                cx,
            )
        });
        window.run_until_parked();
        assert_eq!(
            view.read_with(window, |view, cx| view
                .settings_draft
                .as_ref()
                .expect("draft")
                .inputs["file"]
                .read(cx)
                .value()
                .to_string()),
            original,
            "A-B-A editing must invalidate the older picker even when the text matches again"
        );
        assert!(view.read_with(window, |view, _| {
            view.settings_draft
                .as_ref()
                .expect("draft")
                .path_pending
                .is_empty()
        }));
        assert_eq!(std::fs::read(path).expect("picker never saves"), before);
    }
    #[gpui_kit::test]
    fn remediation_setting_path_current_draft_edit_and_cancel_guards(cx: &mut TestAppContext) {
        let temp = tempfile::tempdir().expect("owned path draft");
        let path = temp.path().join("config.json");
        let config = fixture(&temp);
        config.save_to(&path).expect("save");
        let before = std::fs::read(&path).expect("bytes");
        let (bridge, endpoint) = nxtools_ipc::platform_bridge(8);
        endpoint.set_connected_for_contract_test(true);
        let (owner, view, window) =
            mount_with_platform(config, path.clone(), Some(Rc::new(bridge)), cx);
        view.update_in(window, |view, window, cx| {
            view.open_settings("fixture.manager".into(), window, cx);
        });
        for scenario in 0..3 {
            view.update_in(window, |view, window, cx| {
                view.begin_setting_path("file".into(), "file", 123, window, cx)
            });
            window.run_until_parked();
            let request = endpoint
                .try_receive_command_for_contract_test()
                .expect("pending before dispatch");
            assert!(matches!(
                request.message,
                nxtools_protocol::Message::PlatformCommand(
                    nxtools_protocol::PlatformCommand::PickExtensionSettingPath(_)
                )
            ));
            if scenario == 1 {
                view.update_in(window, |view, window, cx| {
                    view.settings_draft.as_ref().expect("draft").inputs["file"]
                        .update(cx, |input, cx| {
                            input.set_value("C:\\Synthetic\\typed.txt", window, cx)
                        })
                });
                window.run_until_parked();
            }
            if scenario == 2 {
                view.update_in(window, |view, _, _| view.settings_draft = None);
            }
            owner.update_in(window, |app, _, cx| {
                app.handle_platform_notification(
                    nxtools_ipc::PlatformNotification::CommandResult {
                        correlation_id: request.id,
                        result: nxtools_protocol::PlatformCommandResult {
                            success: true,
                            error: None,
                            native_error_code: None,
                            data: Some("C:\\Synthetic\\picked.txt".into()),
                        },
                    },
                    cx,
                )
            });
            window.run_until_parked();
            if scenario < 2 {
                assert_eq!(
                    view.read_with(window, |view, cx| view
                        .settings_draft
                        .as_ref()
                        .expect("draft")
                        .inputs["file"]
                        .read(cx)
                        .value()
                        .to_string()),
                    if scenario == 0 {
                        "C:\\Synthetic\\picked.txt"
                    } else {
                        "C:\\Synthetic\\typed.txt"
                    }
                );
            } else {
                assert!(view.read_with(window, |view, _| view.settings_draft.is_none()));
            }
        }
        assert_eq!(std::fs::read(&path).expect("picker never saves"), before);
    }
    #[gpui_kit::test]
    fn remediation_remove_requires_confirmation_and_cancel_preserves_config(
        cx: &mut TestAppContext,
    ) {
        let temp = tempfile::tempdir().expect("owned removal fixture");
        let path = temp.path().join("config.json");
        let config = fixture(&temp);
        config.save_to(&path).expect("save");
        let before = std::fs::read(&path).expect("before");
        let (owner, _, window) = mount(config, path.clone(), cx);
        click(window, "extension-manage-fixture.manager");
        select_page(window, "来源");
        click(window, "extension-remove-fixture.manager");
        idle(&owner, window);
        assert_eq!(
            std::fs::read(&path).expect("review does not remove"),
            before
        );
        assert!(window.update(|window, cx| window.has_active_dialog(cx)));
        click(window, "extension-remove-cancel");
        assert_eq!(std::fs::read(&path).expect("cancel writes nothing"), before);
        click(window, "extension-remove-fixture.manager");
        click(window, "extension-remove-confirm");
        idle(&owner, window);
        assert!(
            nxtools_core::AppConfig::load_from(&path)
                .expect("removed")
                .extensions
                .packages
                .is_empty()
        );
    }
    #[gpui_kit::test]
    fn remediation_stale_removal_confirmation_cannot_remove_newer_package(cx: &mut TestAppContext) {
        let temp = tempfile::tempdir().expect("owned stale removal");
        let path = temp.path().join("config.json");
        let config = fixture(&temp);
        config.save_to(&path).expect("save");
        let (owner, _, window) = mount(config, path.clone(), cx);
        click(window, "extension-manage-fixture.manager");
        select_page(window, "来源");
        click(window, "extension-remove-fixture.manager");
        owner.update(window, |app, cx| {
            app.packages.packages[0].manifest.description = "changed after review".into();
            cx.notify();
        });
        window.update(|window, cx| window.draw(cx).clear(cx));
        let before = std::fs::read(&path).expect("before");
        window.simulate_keystrokes("enter");
        window.run_until_parked();
        assert_eq!(std::fs::read(&path).expect("stale review no write"), before);
        assert!(!owner.read_with(window, |app, _| app.busy));
        click(window, "extension-remove-cancel");
    }
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
                .find(|node| node.label() == Some("调用记录"))
                .expect("actual activity tab")
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
        assert!(!window.update(|window, cx| window.has_active_dialog(cx)));
        assert!(!window.update(|window, cx| window.has_active_sheet(cx)));
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
                .debug_bounds("extension-remove-fixture.manager")
                .is_some(),
            "management is reachable before metadata"
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
            window.simulate_event(gpui_kit::ScrollEvent {
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
        assert!(
            first.size.height >= px(96.0),
            "the entire cell is the hit target"
        );
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
            for selector in ["extension-management", "extension-import-open"] {
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
                        (selector == "extension-import-open" && !quad.background.is_transparent()
                            || quad.border_color.a > 0.0 && quad.border_widths.top.as_f32() > 0.0)
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
        window.simulate_keystrokes("shift-tab shift-tab");
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
        window.simulate_keystrokes("right");
        assert_eq!(
            window.update(|window, cx| window.focused(cx)),
            Some(first),
            "two header actions wrap within the toolbar"
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
            for selector in ["extension-management", "extension-import-open"] {
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
        let _ = view;
        for keyboard in [false, true] {
            super::click_management(window, "打开插件目录", keyboard);
            let command = endpoint
                .try_receive_command_for_contract_test()
                .expect("actual button dispatch");
            assert!(
                matches!(command.message, nxtools_protocol::Message::PlatformCommand(nxtools_protocol::PlatformCommand::OpenShellTarget(open)) if std::path::Path::new(&open.target) == temp.path().join("extensions"))
            );
            super::click_management(window, "打开插件目录", false);
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
    fn redesigned_extension_workspace_filters_attention_and_opens_one_sheet(
        cx: &mut TestAppContext,
    ) {
        use gpui_kit::component::WindowExt;
        let temp = tempfile::tempdir().expect("owned workspace");
        let mut config = fixture(&temp);
        let other = nxtools_extensions::ImportPreview::from_json(br#"{"schema_version":1,"id":"fixture.off","name":"Off","version":"1","handlers":[{"id":"open","title":"Open","feature":"tools","entry":"action","action":{"type":"website","url_template":"https://example.test"}}]}"#).expect("declarative fixture");
        config
            .extensions
            .install(other, "owned".into(), false)
            .expect("install");
        let path = temp.path().join("config.json");
        config.save_to(&path).expect("config");
        let original = std::fs::read(&path).expect("original");
        let (_, _, window) = mount(config, path.clone(), cx);
        click(window, "extension-attention-open");
        window.update(|window, cx| window.draw(cx).clear(cx));
        assert!(
            window
                .debug_bounds("extension-manage-fixture.off")
                .is_none(),
            "ordinary disabled is not a failure"
        );
        click(window, "extension-manage-fixture.manager");
        assert!(window.update(|window, cx| window.has_active_sheet(cx)));
        assert!(!window.update(|window, cx| window.has_active_dialog(cx)));
        window.simulate_keystrokes("escape");
        window.run_until_parked();
        assert!(!window.update(|window, cx| window.has_active_sheet(cx)));
        assert_eq!(
            std::fs::read(path).expect("discovery never writes"),
            original
        );
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
        click(window, "extension-manage-fixture.manager");
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
    fn extension_cell_missing_required_settings_opens_editable_draft(cx: &mut TestAppContext) {
        let temp = tempfile::tempdir().expect("owned settings fixture");
        let mut config = fixture(&temp);
        let source = temp.path().join("source");
        let mut manifest = config.extensions.packages[0].manifest.clone();
        manifest.settings[0].validation = Some(nxtools_extensions::TextValidation {
            required: true,
            min_length: None,
            max_length: None,
            pattern: None,
        });
        manifest.settings[0].field = nxtools_extensions::SettingType::Text {
            default: String::new(),
        };
        std::fs::write(
            source.join("extension.json"),
            serde_json::to_vec(&manifest).expect("manifest"),
        )
        .expect("source fixture");
        let preview = nxtools_extensions::ImportPreview::from_directory(&source)
            .expect("preview")
            .stage_into(&temp.path().join("packages"))
            .expect("stage");
        let executable = preview
            .code_directory
            .as_ref()
            .expect("staged")
            .join("fixture.exe");
        let hash = preview.content_hash.clone();
        config
            .extensions
            .install(preview, "owned".into(), true)
            .expect("replace fixture");
        config
            .extensions
            .approve(
                "fixture.manager",
                &hash,
                nxtools_extensions::RuntimeIdentity::inspect(&executable).expect("identity"),
            )
            .expect("trust fixture");
        config
            .extensions
            .set_enabled("fixture.manager", true, &[])
            .expect("enable");
        let path = temp.path().join("config.json");
        config.save_to(&path).expect("config");
        let original = std::fs::read(&path).expect("before");
        let (_, view, window) = mount(config, path.clone(), cx);
        click(window, "extension-manage-fixture.manager");
        click(window, "extension-next-fixture.manager");
        assert!(view.read_with(window, |view, _| view.settings_draft.is_some()));
        assert!(
            window
                .debug_bounds("extension-detail-settings-viewport")
                .is_some()
        );
        assert_eq!(
            std::fs::read(path).expect("navigation does not save"),
            original
        );
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
        click(window, "extension-manage-fixture.use");
        window.update(|window, cx| window.draw(cx).clear(cx));
        assert!(window.debug_bounds("extension-usage-guide").is_some());
        assert!(window.debug_bounds("extension-detail-trigger-search:快速启动 → 输入关键词（可在空格后附查询）：findme").is_some());
        assert!(
            owner.read_with(window, |app, _| app.recent_manual_runs().is_empty()),
            "guide must not execute or guess input"
        );
        click(window, "extension-detail-back");
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
        let tile = window
            .debug_bounds("extension-manage-fixture.manager")
            .expect("one tile action");
        assert!(purpose.top() >= tile.top() && purpose.bottom() <= tile.bottom());
        assert!(
            window
                .debug_bounds("extension-next-fixture.manager")
                .is_none()
        );
        click(window, "extension-manage-fixture.manager");
        window.update(|window, cx| window.draw(cx).clear(cx));
        let next = window
            .debug_bounds("extension-next-fixture.manager")
            .expect("detail supplies a next step");
        assert!(next.bottom() <= px(430.0));
        let purpose = window
            .debug_bounds("extension-detail-purpose:整理所选文本，保留原始记录")
            .expect("first detail page must begin with purpose, not just access controls");
        let instruction = window
            .debug_bounds(
                "extension-detail-trigger-clean:剪贴板历史 → 条目菜单 → 扩展处理 → 整理文本",
            )
            .expect("first detail page must show the human trigger");
        assert!(purpose.bottom() <= instruction.top());
        select_page(window, "来源");
        assert!(
            window
                .debug_bounds("extension-remove-fixture.manager")
                .is_some()
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
        window.simulate_keystrokes("escape");
        window.run_until_parked();
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

    #[gpui_kit::test]
    fn expansion_settings_official_label_value_and_inline_validation_keep_draft_and_config(
        cx: &mut TestAppContext,
    ) {
        let temp = tempfile::tempdir().expect("owned validated settings UI");
        let path = temp.path().join("config.json");
        let mut config = nxtools_core::AppConfig::default();
        let manifest = serde_json::json!({"schema_version":1,"id":"fixture.manager","name":"Validation fixture","version":"1",
            "settings":[{"key":"name","title":"Name","type":"text","default":"ok","validation":{"required":true,"min_length":2,"max_length":4}},
                {"key":"mode","title":"Mode","type":"choice","default":"raw","options":[{"value":"raw","label":"Readable label"},{"value":"next","label":"Next label"}]}],
            "handlers":[{"id":"web","title":"Web","feature":"tools","entry":"action","action":{"type":"website","url_template":"https://fixture.test"}}]});
        config
            .extensions
            .install(
                nxtools_extensions::ImportPreview::from_json(
                    &serde_json::to_vec(&manifest).expect("manifest"),
                )
                .expect("formal preview"),
                "owned".into(),
                false,
            )
            .expect("install");
        config.save_to(&path).expect("seed");
        let before = std::fs::read(&path).expect("before");
        let (application, view, window) = mount(config, path.clone(), cx);
        click(window, "extension-manage-fixture.manager");
        select_page(window, "插件设置");
        click(window, "extension-settings-edit");
        let input_id = view.update_in(window, |view, window, cx| {
            let draft = view.settings_draft.as_ref().expect("draft");
            assert_eq!(
                draft.choices["mode"]
                    .read(cx)
                    .selected_value()
                    .map(String::as_str),
                Some("raw")
            );
            assert_eq!(draft.payload(cx).expect("valid defaults").0["mode"], "raw");
            let id = draft.inputs["name"].entity_id();
            draft.inputs["name"].update(cx, |input, cx| input.set_value(" ", window, cx));
            id
        });
        window.update(|window, cx| window.draw(cx).clear(cx));
        let choice_node = window.update(|window, _| {
            gpui_kit::base::test_support::snapshots(window)
                .into_iter()
                .find(|node| node.label() == Some("Mode") && node.value() == Some("Readable label"))
                .expect("official Select name is field title; selected display label is its value")
        });
        assert!(choice_node.bounds().size.width >= px(100.0));
        assert!(choice_node.bounds().size.height >= px(32.0));
        click(window, "extension-settings-save");
        assert_eq!(std::fs::read(&path).expect("invalid save no write"), before);
        assert_eq!(
            view.read_with(window, |view, _| view
                .settings_draft
                .as_ref()
                .expect("retained")
                .inputs["name"]
                .entity_id()),
            input_id
        );
        assert!(
            window
                .debug_bounds("extension-setting-error-name")
                .is_some(),
            "field-level official Alert must be present after invalid Save"
        );
        view.update_in(window, |view, window, cx| {
            let draft = view.settings_draft.as_ref().expect("draft");
            draft.inputs["name"].update(cx, |input, cx| input.set_value("good", window, cx));
            draft.choices["mode"].update(cx, |select, cx| {
                select.set_selected_value(&"next".to_owned(), window, cx)
            });
        });
        click(window, "extension-settings-save");
        idle(&application, window);
        let saved = nxtools_core::AppConfig::load_from(&path).expect("saved");
        assert_eq!(saved.extensions.packages[0].settings["name"], "good");
        assert_eq!(saved.extensions.packages[0].settings["mode"], "next");
        assert!(view.read_with(window, |view, _| view.settings_draft.is_none()));
    }

    #[gpui_kit::test]
    fn expansion_settings_unknown_selection_is_explicit_without_first_option_or_cancel_write(
        cx: &mut TestAppContext,
    ) {
        let temp = tempfile::tempdir().expect("owned stale choice");
        let path = temp.path().join("config.json");
        let config = fixture(&temp);
        config.save_to(&path).expect("seed");
        let before = std::fs::read(&path).expect("before");
        let (application, view, window) = mount(config, path.clone(), cx);
        // Model an already-open owner receiving an unavailable value; never seed corrupt user config.
        application.update(window, |app, cx| {
            app.packages.packages[0]
                .settings
                .insert("mode".into(), serde_json::json!("removed-option"));
            cx.notify();
        });
        click(window, "extension-manage-fixture.manager");
        select_page(window, "插件设置");
        click(window, "extension-settings-edit");
        view.read_with(window, |view, cx| {
            let draft = view.settings_draft.as_ref().expect("draft");
            assert!(draft.choices["mode"].read(cx).selected_value().is_none());
            assert!(draft.payload(cx).is_err());
        });
        assert!(
            window
                .debug_bounds("extension-setting-error-mode")
                .is_some(),
            "unknown value is an explicit error, never first option"
        );
        click(window, "extension-settings-cancel");
        assert_eq!(std::fs::read(&path).expect("cancel unchanged"), before);
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
        let label = match label {
            "信任与启用" => "来源",
            "用途与启用" => "怎么用",
            "来源与能力" | "运行环境" => "来源",
            other => other,
        };
        window.update(|window, cx| window.draw(cx).clear(cx));
        let bounds = window.update(|window, _| {
            gpui_kit::base::test_support::snapshots(window)
                .into_iter()
                .find(|snapshot| snapshot.label() == Some(label))
                .expect("actual detail tab")
                .bounds()
        });
        window.simulate_click(bounds.center(), Modifiers::default());
        window.run_until_parked();
        window.refresh().expect("selected actual package page");
    }

    fn click(window: &mut VisualTestContext, selector: &'static str) {
        if selector == "extension-chain-edit" {
            super::click_management(window, "下载解析链", false);
            return;
        }
        window.update(|window, cx| window.draw(cx).clear(cx));
        window.run_until_parked();
        window.update(|window, cx| window.draw(cx).clear(cx));
        // Settle the official150ms Sheet and250ms Dialog transitions on the test clock.
        window
            .executor()
            .advance_clock(std::time::Duration::from_millis(250));
        window.update(|window, cx| window.draw(cx).clear(cx));
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
                    || selector == "extension-package-storage-open"
                    || selector.starts_with("extension-event-retry-")
                    || selector.starts_with("extension-remove-fixture.")
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
            let position = detail_scroll_point(window);
            window.simulate_event(gpui_kit::ScrollEvent {
                position,
                delta: gpui_kit::ScrollDelta::Lines(gpui_kit::point(0.0, -3.0)),
                ..Default::default()
            });
        }
        panic!("manager action {selector} must be reachable at minimum viewport");
    }

    fn detail_scroll_point(window: &mut VisualTestContext) -> gpui_kit::Point<gpui_kit::Pixels> {
        window
            .debug_bounds("extension-detail-settings-viewport")
            .map_or_else(
                || gpui_kit::point(px(650.0), px(350.0)),
                |bounds| bounds.center(),
            )
    }

    fn top(window: &mut VisualTestContext) {
        let position = detail_scroll_point(window);
        window.simulate_event(gpui_kit::ScrollEvent {
            position,
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
        select_page(window, "来源");
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
            window.simulate_event(gpui_kit::ScrollEvent {
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
        assert!(window.debug_bounds("extension-settings-save").is_none());
        select_page(window, "插件设置");
        window
            .refresh()
            .expect("draft actions fit on the settings tab");
        for selector in ["extension-settings-save", "extension-settings-cancel"] {
            let bounds = window.debug_bounds(selector).expect("pinned draft action");
            assert!(
                bounds.top() >= px(0.0)
                    && bounds.bottom() <= px(430.0)
                    && bounds.right() <= px(744.0)
            );
        }
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
            window.simulate_event(gpui_kit::ScrollEvent {
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
            view.chain_draft
                .as_ref()
                .expect("chain draft")
                .saved_hosts
                .update(cx, |select, cx| {
                    select.set_selected_value(&"example.test".to_owned(), window, cx)
                });
        });
        click(window, "extension-chain-open-saved");
        assert_eq!(
            view.read_with(window, |view, cx| view
                .chain_host
                .read(cx)
                .value()
                .to_string()),
            "example.test"
        );
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

    #[gpui_kit::test]
    fn expansion_storage_ui_package_data_navigation_keeps_original_settings_draft_and_cancel(
        cx: &mut TestAppContext,
    ) {
        let temp = tempfile::tempdir().expect("owned data settings navigation");
        let path = temp.path().join("config.json");
        let config = fixture(&temp);
        config.save_to(&path).expect("config");
        let before = std::fs::read(&path).expect("before");
        let engine = nxtools_download::DownloadEngine::open(temp.path().join("downloads.db"))
            .expect("engine");
        let (service, _) = crate::extensions::service::ExtensionService::new_with_storage(
            &engine,
            config.extensions.clone(),
            None,
            false,
            path.clone(),
            temp.path().join("data"),
        )
        .expect("formal service");
        cx.update(|cx| {
            cx.set_global(crate::extensions::ui_host::ExtensionServiceGlobal(
                service.clone(),
            ))
        });
        let (application, view, window) = mount(config, path.clone(), cx);
        view.update_in(window, |view, window, cx| {
            view.open_settings("fixture.manager".into(), window, cx)
        });
        let input = view.read_with(window, |view, _| {
            view.settings_draft.as_ref().expect("draft").inputs["name"].clone()
        });
        input.update_in(window, |input, window, cx| {
            input.set_value("unsaved data-navigation draft", window, cx)
        });
        click(window, "extension-package-storage-open");
        idle(&application, window);
        assert!(view.read_with(window, |view, _| view.storage_open));
        assert!(application.read_with(window, |app, _| app.storage_management.reviewed.is_some()));
        click(window, "extension-storage-back");
        assert!(!view.read_with(window, |view, _| view.storage_open));
        view.read_with(window, |view, cx| {
            let current = &view.settings_draft.as_ref().expect("original draft").inputs["name"];
            assert_eq!(current.entity_id(), input.entity_id());
            assert_eq!(
                current.read(cx).value().as_ref(),
                "unsaved data-navigation draft"
            );
        });
        for selector in ["extension-settings-save", "extension-settings-cancel"] {
            window.update(|window, cx| window.draw(cx).clear(cx));
            let bounds = window
                .debug_bounds(selector)
                .expect("original pinned draft action");
            assert!(bounds.top() >= px(0.0) && bounds.bottom() <= px(430.0));
        }
        click(window, "extension-settings-cancel");
        assert!(view.read_with(window, |view, _| view.settings_draft.is_none()));
        assert_eq!(
            std::fs::read(&path).expect("navigation and cancel never write"),
            before
        );
        service.runtime.shutdown();
        let released = service.storage_release_probe_for_test();
        application.update(window, |app, _| app.unbind_service_for_test());
        window.update(|_, cx| {
            drop(cx.remove_global::<crate::extensions::ui_host::ExtensionServiceGlobal>())
        });
        drop(service);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
        while !released() {
            window.run_until_parked();
            assert!(
                std::time::Instant::now() < deadline,
                "settings fixture actual service cleanup"
            );
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }
}
