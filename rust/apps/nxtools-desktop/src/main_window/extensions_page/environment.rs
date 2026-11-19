//! Interpreter configuration is independent from plugin trust and activation.
use super::*;
use gpui_kit::component::toolbar::Toolbar;
use nxtools_protocol::{
    InspectExtensionRuntimeCommand, PickExtensionRuntimeCommand, PlatformCommand,
};
use std::collections::BTreeMap;

const PROGRAMS: [(&str, &str); 4] = [
    ("python", "Python"),
    ("node", "Node.js"),
    ("powershell", "Windows PowerShell"),
    ("pwsh", "PowerShell 7 / pwsh"),
];
pub(super) struct EnvironmentDraft {
    target: Option<String>,
    inputs: BTreeMap<String, Entity<InputState>>,
    defaults: BTreeMap<String, String>,
    inherit: bool,
    pending: BTreeMap<String, uuid::Uuid>,
    results: BTreeMap<String, (String, String)>,
    submitted: bool,
    _input_subscriptions: Vec<Subscription>,
}

impl ExtensionManagerView {
    pub(in crate::main_window) fn open_environment(
        &mut self,
        target: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if target.is_some() {
            self.detail_initial_page = 5;
        }
        let registry = self.application.read(cx).packages.clone();
        let defaults = registry
            .runtime_defaults
            .iter()
            .map(|(k, p)| (k.clone(), p.to_string_lossy().into_owned()))
            .collect();
        let mut values = BTreeMap::new();
        let mut inherit = true;
        if let Some(id) = &target {
            if let Some(package) = registry.packages.iter().find(|p| &p.manifest.id == id)
                && let Some(runtime) = &package.manifest.runtime
                && runtime.program != nxtools_extensions::ScriptProgram::Exe
            {
                inherit = package.runtime_override.is_none();
                values.insert(
                    runtime.program.key().to_owned(),
                    registry
                        .runtime_executable(id)
                        .ok()
                        .flatten()
                        .map(|p| p.to_string_lossy().into_owned())
                        .unwrap_or_default(),
                );
            }
        } else {
            for (key, _) in PROGRAMS {
                values.insert(
                    key.to_owned(),
                    registry
                        .runtime_defaults
                        .get(key)
                        .map(|p| p.to_string_lossy().into_owned())
                        .unwrap_or_default(),
                );
            }
        }
        let inputs: BTreeMap<String, Entity<InputState>> = values
            .into_iter()
            .map(|(key, value)| {
                let input = cx.new(|cx| {
                    let mut input = InputState::new(window, cx)
                        .placeholder("自动检测 PATH；也可指定解释器或虚拟环境");
                    input.set_value(value, window, cx);
                    input
                });
                (key, input)
            })
            .collect();
        let input_subscriptions = inputs
            .values()
            .map(|input| cx.observe(input, |_, _, cx| cx.notify()))
            .collect();
        self.environment = Some(EnvironmentDraft {
            target,
            inputs,
            defaults,
            inherit,
            pending: Default::default(),
            results: Default::default(),
            submitted: false,
            _input_subscriptions: input_subscriptions,
        });
        self.draft_error = None;
        let missing = self
            .environment
            .as_ref()
            .expect("new draft")
            .inputs
            .iter()
            .filter(|(_, input)| input.read(cx).value().is_empty())
            .map(|(key, _)| key.clone())
            .collect::<Vec<_>>();
        for key in missing {
            self.environment_operation(key, false, window, cx);
        }
    }
    pub(super) fn reconcile_environment(&mut self, cx: &Context<Self>) {
        if self.environment.as_ref().is_some_and(|d| d.submitted) && !self.application.read(cx).busy
        {
            if matches!(
                self.application.read(cx).last_commit,
                Some(("runtime_defaults" | "runtime_override", true, true))
            ) {
                self.environment = None;
            } else if let Some(draft) = &mut self.environment {
                draft.submitted = false;
            }
        }
    }
    pub(super) fn package_environment(
        &self,
        package: &nxtools_extensions::InstalledPackage,
        cx: &Context<Self>,
    ) -> SettingGroup {
        let message = match package.manifest.runtime.as_ref().map(|r| r.program) {
            None => "声明式扩展不需要解释器。".to_owned(),
            Some(nxtools_extensions::ScriptProgram::Exe) => {
                "此扩展使用导入包内的 EXE，不使用全局解释器。更换包内容后需重新信任。".to_owned()
            }
            _ => self
                .application
                .read(cx)
                .packages
                .runtime_executable(&package.manifest.id)
                .ok()
                .flatten()
                .map(|p| format!("当前解释器：{}", p.display()))
                .unwrap_or_else(|| "尚未配置解释器。请选择默认环境或单独指定路径。".into()),
        };
        let can_edit = package
            .manifest
            .runtime
            .as_ref()
            .is_some_and(|r| r.program != nxtools_extensions::ScriptProgram::Exe);
        let id = package.manifest.id.clone();
        let view = cx.entity().downgrade();
        let busy = self.application.read(cx).busy;
        SettingGroup::new().item(SettingItem::render(move |_, _, _| {
            let id = id.clone();
            let mut content = div()
                .w_full()
                .min_w_0()
                .flex()
                .flex_col()
                .gap_3()
                .child(message.clone());
            if can_edit {
                content = content.child(
                    extensions_settings::view_button(
                        "environment-package-edit",
                        "编辑运行环境",
                        busy,
                        view.clone(),
                        move |view, window, cx| view.open_environment(Some(id.clone()), window, cx),
                    )
                    .with_size(gpui_kit::component::Size::Medium),
                );
            }
            content
        }))
    }
    pub(super) fn render_environment(&self, cx: &Context<Self>) -> AnyElement {
        let draft = self.environment.as_ref().expect("environment editor");
        let busy = self.application.read(cx).busy || draft.submitted;
        let view = cx.entity().downgrade();
        let mut form=Form::new().child(Field::new().child(Alert::new("environment-help","已自动检测未配置的解释器。可直接使用检测结果，也可浏览选择虚拟环境。保存不会授予信任；不自动安装解释器或依赖。").small()));
        if draft.target.is_some() {
            form = form.child(
                Field::new().label("解释器来源").child(
                    extensions_settings::view_button(
                        "environment-inherit",
                        if draft.inherit {
                            "使用默认环境 · 改为单独设置"
                        } else {
                            "单独设置 · 恢复默认环境"
                        },
                        busy,
                        view.clone(),
                        |view, window, cx| {
                            if let Some(draft) = &mut view.environment {
                                draft.inherit = !draft.inherit;
                                draft.pending.clear();
                                draft.results.clear();
                                if draft.inherit {
                                    for (key, input) in &draft.inputs {
                                        let value =
                                            draft.defaults.get(key).cloned().unwrap_or_default();
                                        input.update(cx, |input, cx| {
                                            input.set_value(value, window, cx)
                                        });
                                    }
                                }
                            }
                        },
                    )
                    .with_size(gpui_kit::component::Size::Medium),
                ),
            );
        }
        for (program, label) in PROGRAMS {
            let Some((key, input)) = draft.inputs.get_key_value(program) else {
                continue;
            };
            let inherited = draft.target.is_some() && draft.inherit;
            let waiting = draft.pending.contains_key(key);
            let pick = key.clone();
            let check = key.clone();
            let row = div()
                .w_full()
                .min_w_0()
                .flex()
                .items_center()
                .gap_2()
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .debug_selector({
                            let key = key.clone();
                            move || format!("environment-path-{key}")
                        })
                        .child(
                            Input::new(input)
                                .aria_label(format!("{label} 解释器路径"))
                                .disabled(busy || inherited),
                        ),
                )
                .child(
                    extensions_settings::view_button(
                        format!("environment-browse-{key}"),
                        "浏览",
                        busy || inherited || waiting,
                        view.clone(),
                        move |view, window, cx| {
                            view.environment_operation(pick.clone(), true, window, cx)
                        },
                    )
                    .with_size(gpui_kit::component::Size::Medium),
                )
                .child(
                    extensions_settings::view_button(
                        format!("environment-check-{key}"),
                        if waiting { "检测中" } else { "重新检测" },
                        busy || waiting,
                        view.clone(),
                        move |view, window, cx| {
                            view.environment_operation(check.clone(), false, window, cx)
                        },
                    )
                    .with_size(gpui_kit::component::Size::Medium),
                );
            let mut field = Field::new().label(label).child(row);
            if let Some((path, message)) = draft.results.get(key)
                && path == input.read(cx).value().as_ref()
            {
                field = field.description(message.clone());
            }
            form = form.child(field);
        }
        let mut body = div()
            .size_full()
            .min_h_0()
            .flex()
            .flex_col()
            .p_4()
            .gap_3()
            .child(
                Toolbar::new("extension-environment-toolbar").content(div().flex_1().child(
                    if draft.target.is_some() {
                        "此扩展的运行环境"
                    } else {
                        "默认运行环境"
                    },
                )),
            )
            .child(
                div()
                    .id("environment-scroll")
                    .debug_selector(|| "environment-viewport".into())
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .child(form),
            );
        if let Some(message) = self.draft_error.clone() {
            body = body.child(Alert::error("environment-validation", message).small());
        }
        body.child(
            div()
                .flex_shrink_0()
                .flex()
                .gap_2()
                .child(
                    extensions_settings::view_button(
                        "environment-save",
                        "保存运行环境",
                        busy || !draft.pending.is_empty(),
                        view.clone(),
                        |view, _, cx| view.save_environment(cx),
                    )
                    .with_size(gpui_kit::component::Size::Medium),
                )
                .child(
                    extensions_settings::view_button(
                        "environment-discard",
                        "取消",
                        busy,
                        view,
                        |view, _, _| {
                            view.environment = None;
                            view.draft_error = None;
                        },
                    )
                    .with_size(gpui_kit::component::Size::Medium),
                ),
        )
        .into_any_element()
    }
    fn save_environment(&mut self, cx: &mut Context<Self>) {
        let Some(draft) = &mut self.environment else {
            return;
        };
        if draft.submitted || !draft.pending.is_empty() {
            return;
        }
        let mut paths = BTreeMap::new();
        for (key, input) in &draft.inputs {
            let text = input.read(cx).value();
            let text = text.trim();
            if text.is_empty() {
                if draft.target.is_some() && !draft.inherit {
                    self.draft_error =
                        Some("单独设置需要解释器完整路径；也可选择恢复默认环境。".into());
                    return;
                }
                continue;
            }
            let path = PathBuf::from(text);
            if !path.is_absolute() {
                self.draft_error = Some("请输入解释器的完整绝对路径。".into());
                return;
            }
            paths.insert(key.clone(), path);
        }
        let target = draft.target.clone();
        let inherit = draft.inherit;
        draft.submitted = true;
        let owner = self.application.downgrade();
        self.draft_error = None;
        cx.defer(move |cx| {
            let _ = owner.update(cx, |app, cx| {
                if let Some(id) = target {
                    app.save_runtime_override(
                        id,
                        if inherit {
                            None
                        } else {
                            paths.into_values().next()
                        },
                        cx,
                    );
                } else {
                    app.save_runtime_defaults(paths, cx);
                }
            });
        });
    }
    fn environment_operation(
        &mut self,
        key: String,
        browse: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(draft) = &mut self.environment else {
            return;
        };
        if draft.pending.contains_key(&key) || draft.submitted {
            return;
        }
        let Some(input) = draft.inputs.get(&key).cloned() else {
            return;
        };
        let original = input.read(cx).value().to_string();
        if !browse && !original.trim().is_empty() && !PathBuf::from(original.trim()).is_absolute() {
            draft
                .results
                .insert(key, (original, "请先配置解释器完整路径。".into()));
            return;
        }
        let request = uuid::Uuid::new_v4();
        draft.pending.insert(key.clone(), request);
        draft.results.remove(&key);
        let command = if browse {
            PlatformCommand::PickExtensionRuntime(PickExtensionRuntimeCommand {
                program: key.clone(),
                current_path: original.clone(),
                source_window_handle: crate::ui::native_window_handle(window).unwrap_or_default(),
                source_process_id: std::process::id(),
            })
        } else {
            PlatformCommand::InspectExtensionRuntime(InspectExtensionRuntimeCommand {
                program: key.clone(),
                executable: original.trim().into(),
            })
        };
        let owner = self.application.downgrade();
        let view = cx.entity().downgrade();
        let source = window.window_handle();
        cx.defer(move |cx| {
            let receiver = owner
                .update(cx, |app, cx| app.request_runtime(command, cx))
                .ok()
                .flatten();
            let _ = view.update(cx, |_, cx| {
                cx.spawn(async move |this, cx| {
                    let result = if let Some(receiver) = receiver {
                        receiver.recv().await.ok()
                    } else {
                        None
                    };
                    let _ = source.update(cx, |_, window, cx| {
                        let _ = this.update(cx, |view, cx| {
                            let Some(draft) = &mut view.environment else {
                                return;
                            };
                            if draft.pending.get(&key) != Some(&request) {
                                return;
                            }
                            draft.pending.remove(&key);
                            if input.read(cx).value().as_ref() != original {
                                cx.notify();
                                return;
                            }
                            match result {
                                Some(result) if result.success => {
                                    let data = result.data.unwrap_or_default();
                                    if browse {
                                        if !data.is_empty() && PathBuf::from(&data).is_absolute() {
                                            input.update(cx, |input, cx| {
                                                input.set_value(data, window, cx)
                                            });
                                        }
                                    } else {
                                        if let Some(probe)=parse_probe(&key,original.trim(),&data) {
                                            if original.trim().is_empty() && draft.target.is_some() { draft.inherit=false; }
                                            let selected=if original.trim().is_empty() {input.update(cx,|input,cx|input.set_value(probe.executable.clone(),window,cx));probe.executable} else {original.clone()};
                                            draft.results.insert(key.clone(),(selected,format!("已检测到 {}{}",probe.version,if original.trim().is_empty(){" · 来自 PATH；保存后使用"}else{" · 配置路径可用"})));
                                        } else { draft.results.insert(key.clone(),(original.clone(),"检测返回无效结果，请重试。".into())); }
                                    }
                                }
                                Some(result) if result.native_error_code == Some(995) => {}
                                Some(result) => {
                                    draft.results.insert(
                                        key,
                                        (
                                            original,
                                            format!(
                                                "未找到可用解释器{}；请确认已安装并加入 PATH，或点击浏览选择。",
                                                result
                                                    .native_error_code
                                                    .map(|code| format!("（错误码 {code}）"))
                                                    .unwrap_or_default()
                                            ),
                                        ),
                                    );
                                }
                                None => {
                                    draft.results.insert(
                                        key,
                                        (original, "Windows 适配器不可用，请重试。".into()),
                                    );
                                }
                            }
                            cx.notify();
                        });
                    });
                })
                .detach();
            });
        });
    }
}
pub(super) struct RuntimeProbe {
    pub(super) executable: String,
    pub(super) version: String,
    pub(super) sha256: String,
}
fn same_probe_path(left: &str, right: &str) -> bool {
    left.strip_prefix(r"\\?\")
        .unwrap_or(left)
        .eq_ignore_ascii_case(right.strip_prefix(r"\\?\").unwrap_or(right))
}
pub(super) fn parse_probe(program: &str, executable: &str, data: &str) -> Option<RuntimeProbe> {
    let value = serde_json::from_str::<serde_json::Value>(data).ok()?;
    let path = value["executable"].as_str()?;
    let version = value["version"].as_str()?;
    let hash = value["sha256"].as_str()?;
    if value["program"].as_str() != Some(program)
        || !PathBuf::from(path).is_absolute()
        || path.chars().any(char::is_control)
        || (!executable.is_empty() && !same_probe_path(path, executable))
        || hash.len() != 64
        || !hash.bytes().all(|b| b.is_ascii_hexdigit())
        || version.is_empty()
        || version.len() > 128
        || version.chars().any(char::is_control)
    {
        return None;
    }
    Some(RuntimeProbe {
        executable: path.into(),
        version: version.into(),
        sha256: hash.into(),
    })
}
#[cfg(test)]
fn probe_message(program: &str, executable: &str, data: &str) -> String {
    if let Some(probe) = parse_probe(program, executable, data) {
        format!("可用：{}", probe.version)
    } else {
        "检测返回无效结果".into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::{Modifiers, TestAppContext, VisualTestContext, size};
    use nxtools_protocol::PlatformCommandResult;

    fn finish_discovery(
        app: &Entity<ExtensionApplication>,
        endpoint: &nxtools_ipc::PlatformServerEndpoint,
        surface: &mut VisualTestContext,
    ) {
        while let Ok(envelope) = endpoint.try_receive_command_for_contract_test() {
            app.update_in(surface, |app, _, cx| {
                app.handle_platform_notification(
                    nxtools_ipc::PlatformNotification::CommandResult {
                        correlation_id: envelope.id,
                        result: PlatformCommandResult {
                            success: false,
                            error: Some("not_found".into()),
                            native_error_code: Some(2),
                            data: None,
                        },
                    },
                    cx,
                )
            });
        }
        surface.run_until_parked();
    }
    pub(super) fn mount(
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
        let app = cx.new(|_| {
            ExtensionApplication::new(config, path, platform, gpui_kit::WeakEntity::new_invalid())
        });
        let owner = app.clone();
        let retained = Rc::new(RefCell::new(None));
        let slot = retained.clone();
        let (_, surface) = cx.add_window_view(move |window, cx| {
            let view = cx.new(|cx| ExtensionManagerView::new(owner, window, cx));
            *slot.borrow_mut() = Some(view.clone());
            gpui_kit::base::Root::new(view, window, cx)
        });
        surface.simulate_resize(size(px(744.0), px(430.0)));
        (app, retained.borrow().clone().expect("view"), surface)
    }
    pub(super) fn click(surface: &mut VisualTestContext, selector: &'static str) {
        if selector == "extension-environment-open" {
            super::super::click_management(surface, "运行环境", false);
            return;
        }
        for _ in 0..24 {
            surface.update(|window, cx| window.draw(cx).clear(cx));
            if let Some(bounds) = surface.debug_bounds(selector)
                && bounds.top() >= px(0.0)
                && bounds.bottom() <= px(430.0)
                && (!(selector.starts_with("environment-browse-")
                    || selector.starts_with("environment-check-"))
                    || surface
                        .debug_bounds("environment-viewport")
                        .is_some_and(|viewport| {
                            bounds.top() >= viewport.top() && bounds.bottom() <= viewport.bottom()
                        }))
            {
                surface.simulate_click(bounds.center(), Modifiers::default());
                surface.run_until_parked();
                return;
            }
            surface.simulate_event(gpui_kit::ScrollWheelEvent {
                position: gpui_kit::point(px(600.0), px(250.0)),
                delta: gpui_kit::ScrollDelta::Lines(gpui_kit::point(0.0, -3.0)),
                ..Default::default()
            });
        }
        panic!("reachable action {selector}");
    }
    pub(super) fn idle(app: &Entity<ExtensionApplication>, surface: &mut VisualTestContext) {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            surface.run_until_parked();
            if surface.read(|cx| !app.read(cx).busy) {
                break;
            }
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }
    pub(super) fn set_path(
        view: &Entity<ExtensionManagerView>,
        surface: &mut VisualTestContext,
        path: &str,
    ) {
        view.update_in(surface, |view, window, cx| {
            view.environment.as_ref().expect("draft").inputs["python"]
                .update(cx, |input, cx| input.set_value(path.to_owned(), window, cx))
        });
    }
    #[gpui_kit::test]
    fn runtime_environment_pointer_save_cancel_reopen_and_matching_control_tiers(
        cx: &mut TestAppContext,
    ) {
        let temp = tempfile::tempdir().expect("owned config");
        let path = temp.path().join("config.json");
        let config = nxtools_core::AppConfig::default();
        config.save_to(&path).expect("save");
        let original = std::fs::read(&path).expect("bytes");
        let (app, view, surface) = mount(config, path.clone(), None, cx);
        click(surface, "extension-environment-open");
        set_path(&view, surface, "C:\\Synthetic\\cancelled.exe");
        click(surface, "environment-discard");
        assert_eq!(std::fs::read(&path).expect("cancel bytes"), original);
        click(surface, "extension-environment-open");
        set_path(&view, surface, "C:\\Synthetic\\python.exe");
        surface.update(|window, cx| window.draw(cx).clear(cx));
        let input = surface
            .debug_bounds("environment-path-python")
            .expect("input");
        for selector in ["environment-browse-python", "environment-check-python"] {
            let button = surface.debug_bounds(selector).expect("action");
            assert_eq!(input.size.height, button.size.height);
            assert_eq!(input.center().y, button.center().y);
        }
        click(surface, "environment-save");
        idle(&app, surface);
        let saved = nxtools_core::AppConfig::load_from(&path).expect("reopen");
        assert_eq!(
            saved.extensions.runtime_defaults["python"],
            PathBuf::from("C:\\Synthetic\\python.exe")
        );
        assert!(saved.extensions.packages.is_empty());
        assert!(
            surface.read(|cx| view.read(cx).environment.is_none()),
            "successful save exits editor"
        );
        click(surface, "extension-environment-open");
        assert_eq!(
            surface.read(|cx| view
                .read(cx)
                .environment
                .as_ref()
                .expect("open draft")
                .inputs["python"]
                .read(cx)
                .value()
                .to_string()),
            "C:\\Synthetic\\python.exe"
        );
        set_path(&view, surface, "relative.exe");
        click(surface, "environment-save");
        assert!(surface.read(|cx| view.read(cx).draft_error.is_some()));
        assert_eq!(
            nxtools_core::AppConfig::load_from(&path)
                .expect("reopen saved defaults")
                .extensions
                .runtime_defaults,
            saved.extensions.runtime_defaults
        );
    }
    #[gpui_kit::test]
    fn runtime_environment_late_browse_does_not_replace_typed_path_or_reopened_draft(
        cx: &mut TestAppContext,
    ) {
        let temp = tempfile::tempdir().expect("owned directory");
        let path = temp.path().join("config.json");
        let config = nxtools_core::AppConfig::default();
        config.save_to(&path).expect("owned config");
        let (bridge, endpoint) = nxtools_ipc::platform_bridge(16);
        endpoint.set_connected_for_contract_test(true);
        let (app, view, surface) = mount(config, path, Some(Rc::new(bridge)), cx);
        click(surface, "extension-environment-open");
        finish_discovery(&app, &endpoint, surface);
        set_path(&view, surface, "C:\\Synthetic\\original.exe");
        click(surface, "environment-browse-python");
        let first = endpoint
            .try_receive_command_for_contract_test()
            .expect("actual typed native request");
        set_path(&view, surface, "C:\\Synthetic\\typed.exe");
        app.update_in(surface, |app, _, cx| {
            app.handle_platform_notification(
                nxtools_ipc::PlatformNotification::CommandResult {
                    correlation_id: first.id,
                    result: PlatformCommandResult {
                        success: true,
                        error: None,
                        native_error_code: None,
                        data: Some("C:\\Synthetic\\late.exe".into()),
                    },
                },
                cx,
            )
        });
        surface.run_until_parked();
        assert_eq!(
            surface.read(|cx| view
                .read(cx)
                .environment
                .as_ref()
                .expect("open draft")
                .inputs["python"]
                .read(cx)
                .value()
                .to_string()),
            "C:\\Synthetic\\typed.exe"
        );
        click(surface, "environment-browse-python");
        let second = endpoint
            .try_receive_command_for_contract_test()
            .expect("second request");
        click(surface, "environment-discard");
        click(surface, "extension-environment-open");
        finish_discovery(&app, &endpoint, surface);
        app.update_in(surface, |app, _, cx| {
            app.handle_platform_notification(
                nxtools_ipc::PlatformNotification::CommandResult {
                    correlation_id: second.id,
                    result: PlatformCommandResult {
                        success: true,
                        error: None,
                        native_error_code: None,
                        data: Some("C:\\Synthetic\\late.exe".into()),
                    },
                },
                cx,
            )
        });
        surface.run_until_parked();
        assert!(surface.read(|cx| {
            view.read(cx)
                .environment
                .as_ref()
                .expect("reopened draft")
                .inputs["python"]
                .read(cx)
                .value()
                .is_empty()
        }));
        click(surface, "environment-browse-python");
        let current = endpoint
            .try_receive_command_for_contract_test()
            .expect("current browse");
        app.update_in(surface, |app, _, cx| {
            app.handle_platform_notification(
                nxtools_ipc::PlatformNotification::CommandResult {
                    correlation_id: current.id,
                    result: PlatformCommandResult {
                        success: true,
                        error: None,
                        native_error_code: None,
                        data: Some("C:\\Synthetic\\selected.exe".into()),
                    },
                },
                cx,
            )
        });
        surface.run_until_parked();
        assert_eq!(
            surface.read(|cx| view
                .read(cx)
                .environment
                .as_ref()
                .expect("open draft")
                .inputs["python"]
                .read(cx)
                .value()
                .to_string()),
            "C:\\Synthetic\\selected.exe"
        );
        set_path(&view, surface, "C:\\Synthetic\\current.exe");
        click(surface, "environment-check-python");
        let _ = endpoint
            .try_receive_command_for_contract_test()
            .expect("check request");
        app.update_in(surface, |app, _, cx| {
            app.handle_platform_notification(
                nxtools_ipc::PlatformNotification::Disconnected {
                    generation: 1,
                    pending_command_ids: vec![],
                },
                cx,
            )
        });
        surface.run_until_parked();
        assert!(surface.read(|cx| {
            view.read(cx)
                .environment
                .as_ref()
                .expect("open draft")
                .pending
                .is_empty()
        }));
        assert!(surface.read(|cx| {
            view.read(cx)
                .environment
                .as_ref()
                .expect("open draft")
                .results["python"]
                .1
                .contains("解释器")
        }));
        click(surface, "environment-check-python");
        let _ = endpoint
            .try_receive_command_for_contract_test()
            .expect("timeout request");
        surface
            .executor()
            .advance_clock(std::time::Duration::from_secs(31));
        surface.run_until_parked();
        assert!(surface.read(|cx| {
            view.read(cx)
                .environment
                .as_ref()
                .expect("open draft")
                .pending
                .is_empty()
        }));
        assert!(surface.read(|cx| {
            view.read(cx)
                .environment
                .as_ref()
                .expect("open draft")
                .results["python"]
                .1
                .contains("1460")
        }));
    }
    #[test]
    fn runtime_probe_result_requires_matching_program_path_hash_and_bounded_version() {
        let value = serde_json::json!({"program":"python","executable":"C:\\Synthetic\\python.exe","sha256":"a".repeat(64),"version":"Python 3.12.1"});
        assert!(
            probe_message("python", "C:\\Synthetic\\python.exe", &value.to_string())
                .starts_with("可用")
        );
        assert!(
            parse_probe("python", r"\\?\C:\Synthetic\python.exe", &value.to_string()).is_some()
        );
        assert!(
            !probe_message("node", "C:\\Synthetic\\python.exe", &value.to_string())
                .starts_with("可用")
        );
        assert!(
            !probe_message("python", "C:\\Synthetic\\other.exe", &value.to_string())
                .starts_with("可用")
        );
        assert!(
            !probe_message(
                "python",
                "C:\\Synthetic\\python.exe",
                "{\"version\":\"Python 3.12.1\"}"
            )
            .starts_with("可用")
        );
    }
}

#[cfg(test)]
mod package_tests {
    use super::*;
    use gpui_kit::TestAppContext;
    #[gpui_kit::test]
    fn runtime_package_override_save_is_independent_from_trust_and_restores_inheritance(
        cx: &mut TestAppContext,
    ) {
        cx.update(gpui_kit::init);
        cx.executor().allow_parking();
        let temp = tempfile::tempdir().expect("owned directory");
        let bundle = temp.path().join("bundle");
        std::fs::create_dir_all(&bundle).expect("bundle directory");
        std::fs::write(bundle.join("run.py"), "# never executed\n").expect("synthetic script");
        std::fs::write(bundle.join("extension.json"),r#"{"schema_version":1,"id":"fixture.python","name":"Python fixture","version":"1.0.0","runtime":{"program":"python"},"files":["run.py"],"handlers":[{"id":"run","title":"Run","feature":"tools","entry":"action","action":{"type":"script","entry_file":"run.py"}}]}"#).expect("synthetic manifest");
        let mut config = nxtools_core::AppConfig::default();
        config
            .extensions
            .install(
                nxtools_extensions::ImportPreview::from_directory(&bundle)
                    .expect("bundle preview")
                    .stage_into(&temp.path().join("staged"))
                    .expect("immutable staging"),
                "owned fixture".into(),
                false,
            )
            .expect("install synthetic bundle");
        config
            .extensions
            .set_runtime_defaults(BTreeMap::from([(
                "python".into(),
                PathBuf::from("C:\\Synthetic\\default.exe"),
            )]))
            .expect("configure default");
        let path = temp.path().join("config.json");
        config.save_to(&path).expect("owned config");
        let (app, view, surface) = tests::mount(config, path.clone(), None, cx);
        tests::click(surface, "extension-manage-fixture.python");
        extensions_settings::tests::select_page(surface, "运行环境");
        tests::click(surface, "environment-package-edit");
        tests::click(surface, "environment-inherit");
        tests::set_path(&view, surface, "");
        tests::click(surface, "environment-save");
        assert!(surface.read(|cx| view.read(cx).draft_error.is_some()));
        tests::set_path(&view, surface, "C:\\Synthetic\\venv\\Scripts\\python.exe");
        tests::click(surface, "environment-save");
        tests::idle(&app, surface);
        let saved = nxtools_core::AppConfig::load_from(&path).expect("reopen override");
        let package = &saved.extensions.packages[0];
        assert_eq!(
            package.runtime_override,
            Some(PathBuf::from("C:\\Synthetic\\venv\\Scripts\\python.exe"))
        );
        assert!(package.trust.is_none() && !package.enabled);
        assert_eq!(
            saved.extensions.runtime_defaults["python"],
            PathBuf::from("C:\\Synthetic\\default.exe")
        );
        tests::click(surface, "environment-package-edit");
        tests::click(surface, "environment-inherit");
        assert_eq!(
            surface.read(|cx| view
                .read(cx)
                .environment
                .as_ref()
                .expect("open draft")
                .inputs["python"]
                .read(cx)
                .value()
                .to_string()),
            "C:\\Synthetic\\default.exe"
        );
        tests::click(surface, "environment-save");
        tests::idle(&app, surface);
        assert!(
            nxtools_core::AppConfig::load_from(&path)
                .expect("reopen inheritance")
                .extensions
                .packages[0]
                .runtime_override
                .is_none()
        );
    }
}

#[cfg(test)]
mod path_tests {
    use super::*;
    use gpui_kit::TestAppContext;
    #[gpui_kit::test]
    fn opening_environment_automatically_requests_path_discovery_without_saving(
        cx: &mut TestAppContext,
    ) {
        let temp = tempfile::tempdir().expect("owned");
        let path = temp.path().join("config.json");
        let config = nxtools_core::AppConfig::default();
        config.save_to(&path).expect("config");
        let original = std::fs::read(&path).expect("bytes");
        let (bridge, endpoint) = nxtools_ipc::platform_bridge(16);
        endpoint.set_connected_for_contract_test(true);
        let (app, view, surface) = tests::mount(config, path.clone(), Some(Rc::new(bridge)), cx);
        tests::click(surface, "extension-environment-open");
        let mut programs = std::collections::BTreeSet::new();
        for _ in 0..4 {
            let envelope = endpoint
                .try_receive_command_for_contract_test()
                .expect("opening environment must detect PATH without manual input");
            let nxtools_protocol::Message::PlatformCommand(
                PlatformCommand::InspectExtensionRuntime(command),
            ) = envelope.message
            else {
                panic!("typed inspect")
            };
            assert!(command.executable.is_empty());
            programs.insert(command.program.clone());
            let response=serde_json::json!({"program":command.program,"executable":format!("C:\\Synthetic\\{}.exe",command.program),"version":"3.12.1","sha256":"a".repeat(64)}).to_string();
            app.update_in(surface, |app, _, cx| {
                app.handle_platform_notification(
                    nxtools_ipc::PlatformNotification::CommandResult {
                        correlation_id: envelope.id,
                        result: nxtools_protocol::PlatformCommandResult {
                            success: true,
                            error: None,
                            native_error_code: None,
                            data: Some(response),
                        },
                    },
                    cx,
                )
            });
        }
        assert_eq!(programs.len(), 4);
        surface.run_until_parked();
        assert!(surface.read(|cx| {
            view.read(cx).environment.as_ref().expect("draft").inputs["python"]
                .read(cx)
                .value()
                .contains("python.exe")
        }));
        assert_eq!(std::fs::read(path).expect("no autosave"), original);
    }
}

#[cfg(test)]
mod discovered_package_tests {
    use super::*;
    use gpui_kit::TestAppContext;
    #[gpui_kit::test]
    fn package_path_discovery_saves_the_detected_override_not_empty_inheritance(
        cx: &mut TestAppContext,
    ) {
        let temp = tempfile::tempdir().expect("owned");
        let bundle = temp.path().join("bundle");
        std::fs::create_dir_all(&bundle).expect("bundle");
        std::fs::write(bundle.join("main.py"), "# never executed").expect("script");
        std::fs::write(bundle.join("extension.json"),r#"{"schema_version":1,"id":"fixture.auto","name":"Auto","version":"1","runtime":{"program":"python"},"files":["main.py"],"handlers":[{"id":"run","title":"Run","feature":"tools","entry":"action","action":{"type":"script","entry_file":"main.py"}}]}"#).expect("manifest");
        let mut config = nxtools_core::AppConfig::default();
        config
            .extensions
            .install(
                nxtools_extensions::ImportPreview::from_directory(&bundle)
                    .expect("preview")
                    .stage_into(&temp.path().join("managed"))
                    .expect("stage"),
                "owned.zip".into(),
                false,
            )
            .expect("install");
        let path = temp.path().join("config.json");
        config.save_to(&path).expect("config");
        let (bridge, endpoint) = nxtools_ipc::platform_bridge(8);
        endpoint.set_connected_for_contract_test(true);
        let (app, view, surface) = tests::mount(config, path.clone(), Some(Rc::new(bridge)), cx);
        view.update_in(surface, |view, window, cx| {
            view.open_environment(Some("fixture.auto".into()), window, cx)
        });
        surface.run_until_parked();
        let request = endpoint
            .try_receive_command_for_contract_test()
            .expect("PATH inspect");
        app.update_in(surface,|app,_,cx|app.handle_platform_notification(nxtools_ipc::PlatformNotification::CommandResult{correlation_id:request.id,result:nxtools_protocol::PlatformCommandResult{success:true,error:None,native_error_code:None,data:Some(serde_json::json!({"program":"python","executable":"C:\\Synthetic\\python.exe","version":"Python 3.12.1","sha256":"a".repeat(64)}).to_string())}},cx));
        surface.run_until_parked();
        assert!(surface.read(|cx| !view.read(cx).environment.as_ref().expect("draft").inherit));
        tests::click(surface, "environment-save");
        tests::idle(&app, surface);
        let saved = nxtools_core::AppConfig::load_from(&path).expect("saved");
        assert_eq!(
            saved.extensions.packages[0].runtime_override,
            Some(PathBuf::from("C:\\Synthetic\\python.exe"))
        );
        assert!(
            saved.extensions.packages[0].trust.is_none() && !saved.extensions.packages[0].enabled
        );
    }
}
