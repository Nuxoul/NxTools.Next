//! Trust is one explicit decision, separate from runtime discovery and enablement.
use super::*;
use gpui_kit::component::{
    StyledExt, WindowExt,
    accordion::Accordion,
    description_list::DescriptionList,
    dialog::{DialogAction, DialogClose, DialogDescription, DialogFooter},
    group_box::{GroupBox, GroupBoxVariant, GroupBoxVariants},
};
use nxtools_protocol::{
    InspectExtensionRuntimeCommand, PickExtensionRuntimeCommand, PlatformCommand,
};

pub(super) struct TrustDraft {
    package: nxtools_extensions::InstalledPackage,
    selected: Option<PathBuf>,
    path: Option<PathBuf>,
    attempted_path: Option<PathBuf>,
    runtime_hash: Option<String>,
    version: Option<String>,
    pending: Option<uuid::Uuid>,
    message: Option<String>,
    submitted: bool,
    submitted_revision: u64,
    details: bool,
}
impl ExtensionManagerView {
    #[cfg(debug_assertions)]
    pub(in crate::main_window) fn initialize_trust_preview(
        &mut self,
        missing: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.open_trust("synthetic.cleaner".into(), window, cx);
        if let Some(draft) = &self.trust_dialog {
            let mut state = draft.borrow_mut();
            state.pending = None;
            if missing {
                state.message = Some(
                    "PATH 中未找到 Python。安装后重新检测，或选择已有解释器／虚拟环境。".into(),
                );
            } else {
                state.path = Some(PathBuf::from("C:\\Python312\\python.exe"));
                state.version = Some("Python 3.12.1".into());
                state.runtime_hash = Some("a".repeat(64));
                state.message = None;
            }
        }
    }
    pub(in crate::main_window) fn open_trust(
        &mut self,
        id: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(package) = self
            .application
            .read(cx)
            .packages
            .packages
            .iter()
            .find(|p| p.manifest.id == id)
            .cloned()
        else {
            return;
        };
        let selected = self
            .application
            .read(cx)
            .packages
            .runtime_executable(&id)
            .ok()
            .flatten();
        let exe = package
            .manifest
            .runtime
            .as_ref()
            .is_some_and(|r| r.program == nxtools_extensions::ScriptProgram::Exe);
        let draft = Rc::new(RefCell::new(TrustDraft {
            package,
            selected: selected.clone(),
            path: if exe { selected.clone() } else { None },
            attempted_path: selected.clone(),
            runtime_hash: None,
            version: exe.then(|| "包内程序".into()),
            pending: None,
            message: None,
            submitted: false,
            submitted_revision: 0,
            details: false,
        }));
        self.trust_target = Some(id);
        self.trust_dialog = Some(draft.clone());
        self.draft_error = None;
        self.runtime_path.update(cx, |input, cx| {
            input.set_value(
                selected
                    .as_ref()
                    .map(|p| p.to_string_lossy().into_owned())
                    .unwrap_or_default(),
                window,
                cx,
            )
        });
        if !exe {
            self.inspect_trust(false, window, cx);
        }
        let owner = self.application.downgrade();
        let view = cx.entity().downgrade();
        let source = window.window_handle();
        cx.defer(move |cx| {
            let _ = source.update(cx, |_, window, cx| {
                if !view.upgrade().is_some_and(|view| {
                    view.read(cx)
                        .trust_dialog
                        .as_ref()
                        .is_some_and(|current| Rc::ptr_eq(current, &draft))
                }) {
                    return;
                }
                window.open_dialog(cx, move |dialog, _, cx| {
                    let state = draft.borrow();
                    let app = owner.upgrade();
                    let app = app.as_ref().map(|app| app.read(cx));
                    let busy = app.is_some_and(|app| app.busy) || state.submitted;
                    let current = app.is_some_and(|app| {
                        app.packages.packages.iter().any(|p| {
                            p.manifest.id == state.package.manifest.id
                                && p.content_hash == state.package.content_hash
                        }) && app
                            .packages
                            .runtime_executable(&state.package.manifest.id)
                            .ok()
                            .flatten()
                            == state.selected
                    });
                    let ready = current && state.pending.is_none() && state.path.is_some();
                    let confirm_view = view.clone();
                    let admission_owner = owner.clone();
                    let cancel_view = view.clone();
                    let close_view = view.clone();
                    let confirm_draft = draft.clone();
                    let cancel_draft = draft.clone();
                    let close_draft = draft.clone();
                    let confirm = DialogAction::new().child(
                        Button::new("extension-approve-runtime")
                            .debug_selector(|| "extension-approve-runtime".into())
                            .label("确认信任")
                            .primary()
                            .disabled(busy || !ready),
                    );
                    let cancel = DialogClose::new().trigger(move |button| {
                        button
                            .debug_selector(|| "extension-trust-cancel".into())
                            .label("取消")
                            .disabled(busy)
                    });
                    let mut body = div()
                        .w_full()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .gap_3()
                        .child(
                            div()
                                .text_lg()
                                .font_semibold()
                                .child(state.package.manifest.name.clone()),
                        )
                        .child(DialogDescription::new().child(format!(
                            "版本 {} · 信任后仍需单独启用",
                            state.package.manifest.version
                        )));
                    if !state.package.manifest.description.is_empty() {
                        body = body.child(state.package.manifest.description.clone());
                    }
                    let runtime = state
                        .package
                        .manifest
                        .runtime
                        .as_ref()
                        .map_or("声明式", |r| r.program.key());
                    let summary = DescriptionList::new()
                        .columns(1)
                        .bordered(false)
                        .label_width(px(88.0))
                        .item("运行环境", runtime, 1)
                        .item(
                            "状态",
                            if state.pending.is_some() {
                                "正在检测本机解释器…".into()
                            } else {
                                state
                                    .version
                                    .clone()
                                    .unwrap_or_else(|| "未找到可用解释器".into())
                            },
                            1,
                        )
                        .item(
                            "选择方式",
                            if state.selected.is_some() {
                                "已保存的配置"
                            } else {
                                "自动检测 PATH"
                            },
                            1,
                        );
                    body = body.child(summary);
                    if let Some(message) = &state.message {
                        body = body.child(
                            Alert::new("extension-trust-detection", message.clone()).small(),
                        );
                    }
                    let retry_view = view.clone();
                    let browse_view = view.clone();
                    let runtime_actions = if !exe {
                        Some(
                            gpui_kit::component::toolbar::Toolbar::new(
                                "extension-trust-runtime-actions",
                            )
                            .with_size(gpui_kit::component::Size::Medium)
                            .h_auto()
                            .content(
                                div()
                                    .flex()
                                    .flex_wrap()
                                    .gap_2()
                                    .child(
                                        extensions_settings::view_button(
                                            "extension-trust-recheck",
                                            "重新检测",
                                            busy || state.pending.is_some(),
                                            retry_view,
                                            |view, window, cx| {
                                                view.inspect_trust(false, window, cx)
                                            },
                                        )
                                        .with_size(gpui_kit::component::Size::Medium),
                                    )
                                    .child(
                                        extensions_settings::view_button(
                                            "extension-trust-browse",
                                            "选择其他解释器",
                                            busy || state.pending.is_some(),
                                            browse_view,
                                            |view, window, cx| view.inspect_trust(true, window, cx),
                                        )
                                        .with_size(gpui_kit::component::Size::Medium),
                                    ),
                            ),
                        )
                    } else {
                        None
                    };
                    let mut permissions = GroupBox::new()
                        .title("作者声明的用途")
                        .with_variant(GroupBoxVariant::Normal);
                    if state.package.manifest.permissions.is_empty() {
                        permissions = permissions.child("未声明权限不代表系统限制访问。");
                    }
                    for permission in &state.package.manifest.permissions {
                        permissions = permissions.child(
                            div()
                                .flex()
                                .flex_col()
                                .gap_1()
                                .child(permission.reason.clone())
                                .child(DialogDescription::new().child(permission.id.clone())),
                        );
                    }
                    body = body.child(permissions);
                    let technical = DescriptionList::new()
                        .columns(1)
                        .bordered(false)
                        .label_width(px(88.0))
                        .item("扩展 ID", state.package.manifest.id.clone(), 1)
                        .item("来源", state.package.source.clone(), 1)
                        .item(
                            "解释器",
                            state
                                .path
                                .as_ref()
                                .map(|p| p.to_string_lossy().into_owned())
                                .unwrap_or_else(|| "未检测到".into()),
                            1,
                        )
                        .item("内容哈希", state.package.content_hash.clone(), 1);
                    let details_draft = draft.clone();
                    let opened = state.details;
                    let runtime_details =
                        runtime_description(state.package.manifest.runtime.as_ref());
                    body = body.child(
                        Accordion::new("extension-trust-technical")
                            .w_full()
                            .h_auto()
                            .item(move |item| {
                                item.title("技术详情")
                                    .open(opened)
                                    .child(technical)
                                    .child(runtime_details)
                            })
                            .on_toggle_click(move |indices, window, _| {
                                details_draft.borrow_mut().details = indices.contains(&0);
                                window.refresh();
                            }),
                    );
                    if !current {
                        body = body.child(
                            Alert::error(
                                "extension-trust-changed",
                                "扩展内容或运行环境已变化，请取消后重新检查。",
                            )
                            .small(),
                        );
                    }
                    if let Some(message) = app.and_then(|app| app.trust_failure.clone()) {
                        body = body.child(Alert::error("extension-trust-error", message).small());
                    }
                    drop(state);
                    dialog
                        .title("信任扩展")
                        .w(px(640.0))
                        .overlay_closable(false)
                        .close_button(!busy)
                        .child(body)
                        .footer(
                            div()
                                .flex_shrink_0()
                                .flex()
                                .flex_col()
                                .gap_3()
                                .children(runtime_actions)
                                .child(
                                    div()
                                        .debug_selector(|| "extension-trust-warning".into())
                                        .child(
                                            Alert::warning(
                                                "extension-trust-authority",
                                                TRUST_WARNING.replace("。以下", "。\n以下"),
                                            )
                                            .small(),
                                        ),
                                )
                                .child(
                                    DialogFooter::new()
                                        .child(div().flex_none().w_auto().child(cancel))
                                        .child(div().flex_none().w_auto().child(confirm)),
                                ),
                        )
                        .on_ok(move |_, _, cx| {
                            let mut state = confirm_draft.borrow_mut();
                            let Some(owner) = admission_owner.upgrade() else {
                                return false;
                            };
                            let current = owner.read(cx);
                            if current.busy
                                || current
                                    .packages
                                    .runtime_executable(&state.package.manifest.id)
                                    .ok()
                                    .flatten()
                                    != state.selected
                                || !current.packages.packages.iter().any(|p| {
                                    p.manifest.id == state.package.manifest.id
                                        && p.content_hash == state.package.content_hash
                                })
                            {
                                return false;
                            }
                            state.submitted_revision = current.trust_revision;
                            if state.submitted || state.pending.is_some() {
                                return false;
                            }
                            let Some(path) = state.path.clone() else {
                                return false;
                            };
                            let id = state.package.manifest.id.clone();
                            let hash = state.package.content_hash.clone();
                            let selected = state.selected.clone();
                            let runtime_hash = state.runtime_hash.clone();
                            state.submitted = true;
                            drop(state);
                            let view = confirm_view.clone();
                            cx.defer(move |cx| {
                                let _ = view.update(cx, |view, cx| {
                                    let owner = view.application.downgrade();
                                    cx.defer(move |cx| {
                                        let _ = owner.update(cx, |app, cx| {
                                            app.approve_runtime_checked(
                                                id,
                                                hash,
                                                path,
                                                runtime_hash,
                                                selected,
                                                cx,
                                            )
                                        });
                                    });
                                    cx.notify();
                                });
                            });
                            false
                        })
                        .on_cancel(move |_, _, cx| {
                            if cancel_draft.borrow().submitted {
                                return false;
                            }
                            let view = cancel_view.clone();
                            let draft = cancel_draft.clone();
                            cx.defer(move |cx| {
                                let _ = view.update(cx, |view, cx| {
                                    if view
                                        .trust_dialog
                                        .as_ref()
                                        .is_some_and(|current| Rc::ptr_eq(current, &draft))
                                    {
                                        view.trust_dialog = None;
                                        view.trust_target = None;
                                    }
                                    cx.notify();
                                });
                            });
                            true
                        })
                        .on_close(move |_, _, cx| {
                            let view = close_view.clone();
                            let draft = close_draft.clone();
                            cx.defer(move |cx| {
                                let _ = view.update(cx, |view, cx| {
                                    if view
                                        .trust_dialog
                                        .as_ref()
                                        .is_some_and(|current| Rc::ptr_eq(current, &draft))
                                    {
                                        view.trust_dialog = None;
                                        view.trust_target = None;
                                    }
                                    cx.notify();
                                });
                            });
                        })
                });
            });
        });
    }
    pub(super) fn reconcile_trust_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(draft) = self.trust_dialog.clone() else {
            return;
        };
        let state = self.application.read(cx);
        let mut edit = draft.borrow_mut();
        if edit.submitted && !state.busy && state.trust_revision != edit.submitted_revision {
            if state.trust_failure.is_some() {
                edit.submitted = false;
            } else if state.packages.packages.iter().any(|p| {
                p.manifest.id == edit.package.manifest.id
                    && p.trust
                        .as_ref()
                        .is_some_and(|t| t.content_hash == edit.package.content_hash)
            }) {
                self.trust_dialog = None;
                self.trust_target = None;
                let source = window.window_handle();
                cx.defer(move |cx| {
                    let _ = source.update(cx, |_, window, cx| {
                        window.close_dialog(cx);
                    });
                });
            }
        }
    }
    fn inspect_trust(&mut self, browse: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(draft) = self.trust_dialog.clone() else {
            return;
        };
        let mut state = draft.borrow_mut();
        if state.submitted || state.pending.is_some() {
            return;
        }
        let Some(runtime) = state.package.manifest.runtime.as_ref() else {
            return;
        };
        if runtime.program == nxtools_extensions::ScriptProgram::Exe {
            return;
        }
        if browse && state.selected.is_some() {
            let target = state.package.manifest.id.clone();
            drop(state);
            self.trust_dialog = None;
            self.trust_target = None;
            let view = cx.entity().downgrade();
            let source = window.window_handle();
            cx.defer(move |cx| {
                let _ = source.update(cx, |_, window, cx| {
                    window.close_dialog(cx);
                    let _ = view.update(cx, |view, cx| {
                        view.open_environment(Some(target), window, cx)
                    });
                });
            });
            return;
        }
        let program = runtime.program.key().to_owned();
        let original = state
            .attempted_path
            .as_ref()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default();
        let request = uuid::Uuid::new_v4();
        state.pending = Some(request);
        state.message = None;
        state.path = None;
        state.runtime_hash = None;
        let command = if browse {
            PlatformCommand::PickExtensionRuntime(PickExtensionRuntimeCommand {
                program: program.clone(),
                current_path: original.clone(),
                source_window_handle: crate::ui::native_window_handle(window).unwrap_or_default(),
                source_process_id: std::process::id(),
            })
        } else {
            PlatformCommand::InspectExtensionRuntime(InspectExtensionRuntimeCommand {
                program: program.clone(),
                executable: original.clone(),
            })
        };
        drop(state);
        let owner = self.application.downgrade();
        let view = cx.entity().downgrade();
        let source = window.window_handle();
        cx.defer(move|cx|{
            let receiver=owner.update(cx,|app,cx|app.request_runtime(command,cx)).ok().flatten();
            let _=view.update(cx,|_,cx|{cx.spawn(async move|this,cx|{
                let result=if let Some(receiver)=receiver{receiver.recv().await.ok()}else{None};
                let _=source.update(cx,|_,window,cx|{let _=this.update(cx,|view,cx|{
                    if !view.trust_dialog.as_ref().is_some_and(|current|Rc::ptr_eq(current,&draft)){return;}
                    let mut state=draft.borrow_mut();if state.pending!=Some(request){return;}state.pending=None;
                    match result {
                        Some(result) if result.success=>{let data=result.data.unwrap_or_default();
                            if browse {if !data.is_empty()&&PathBuf::from(&data).is_absolute(){state.attempted_path=Some(PathBuf::from(data));drop(state);view.inspect_trust(false,window,cx);cx.notify();return;}}
                            else if let Some(probe)=environment::parse_probe(&program,&original,&data){state.path=Some(PathBuf::from(&probe.executable));state.runtime_hash=Some(probe.sha256);state.version=Some(probe.version);state.message=None;}
                            else{state.message=Some("检测返回无效结果，请重新检测。".into());}
                        }
                        Some(result) if result.native_error_code==Some(995)=>{state.message=Some("未选择解释器，可重新检测 PATH 或浏览选择。".into());}
                        Some(result)=>{state.message=Some(format!("未找到可用解释器{}。请确认已安装并加入 PATH，或选择解释器／虚拟环境。",result.native_error_code.map(|code|format!("（错误码 {code}）")).unwrap_or_default()));}
                        None=>{state.message=Some("Windows 适配器暂不可用，请重试检测。".into());}
                    }
                    cx.notify();
                });});
            }).detach();});
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::{TestAppContext, VisualTestContext, size};
    use nxtools_protocol::PlatformCommandResult;
    fn draw(surface: &mut VisualTestContext) {
        surface.update(|window, cx| window.draw(cx).clear(cx));
        surface.run_until_parked();
        surface.update(|window, cx| window.draw(cx).clear(cx));
    }
    fn click(surface: &mut VisualTestContext, selector: &'static str) {
        draw(surface);
        let bounds = surface.debug_bounds(selector).expect("control");
        assert!(bounds.top() >= px(0.0) && bounds.bottom() <= px(430.0));
        surface.simulate_click(bounds.center(), Default::default());
        surface.run_until_parked();
    }
    fn idle(app: &Entity<ExtensionApplication>, surface: &mut VisualTestContext) {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            surface.run_until_parked();
            if surface.read(|cx| !app.read(cx).busy) {
                break;
            }
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        draw(surface);
    }
    #[gpui_kit::test]
    fn path_detected_runtime_can_be_trusted_directly_but_cancel_and_detection_never_write(
        cx: &mut TestAppContext,
    ) {
        cx.update(gpui_kit::init);
        cx.executor().allow_parking();
        let temp = tempfile::tempdir().expect("owned");
        let bundle = temp.path().join("source");
        std::fs::create_dir_all(&bundle).expect("source");
        std::fs::write(bundle.join("main.py"), "# never executed").expect("script");
        std::fs::write(bundle.join("extension.json"),r#"{"schema_version":1,"id":"fixture.path","name":"PATH fixture","version":"1.0.0","runtime":{"program":"python"},"files":["main.py"],"permissions":[{"id":"clipboard.read","reason":"Read selected text"}],"handlers":[{"id":"run","title":"Run","feature":"clipboard","entry":"action","action":{"type":"script","entry_file":"main.py"}}]}"#).expect("manifest");
        let mut config = nxtools_core::AppConfig::default();
        config
            .extensions
            .install(
                nxtools_extensions::ImportPreview::from_directory(&bundle)
                    .expect("preview")
                    .stage_into(&temp.path().join("managed"))
                    .expect("stage"),
                "fixture.zip".into(),
                false,
            )
            .expect("install");
        let path = temp.path().join("config.json");
        config.save_to(&path).expect("config");
        let original = std::fs::read(&path).expect("bytes");
        let runtime = temp.path().join("python.exe");
        std::fs::write(&runtime, b"synthetic interpreter identity; never executed")
            .expect("runtime");
        let identity = nxtools_extensions::RuntimeIdentity::inspect(&runtime).expect("identity");
        let (bridge, endpoint) = nxtools_ipc::platform_bridge(16);
        endpoint.set_connected_for_contract_test(true);
        let app = cx.new(|_| {
            ExtensionApplication::new(
                config,
                path.clone(),
                Some(Rc::new(bridge)),
                gpui_kit::WeakEntity::new_invalid(),
            )
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
        let view = retained.borrow().clone().expect("view");
        for cancel in [true, false] {
            view.update_in(surface, |view, window, cx| {
                view.open_trust("fixture.path".into(), window, cx)
            });
            draw(surface);
            let request = endpoint
                .try_receive_command_for_contract_test()
                .expect("automatic PATH request");
            let nxtools_protocol::Message::PlatformCommand(
                PlatformCommand::InspectExtensionRuntime(command),
            ) = request.message
            else {
                panic!("inspect")
            };
            assert!(command.executable.is_empty());
            let response=serde_json::json!({"program":"python","executable":runtime.to_string_lossy(),"version":"Python 3.12.1","sha256":identity.sha256}).to_string();
            app.update_in(surface, |app, _, cx| {
                app.handle_platform_notification(
                    nxtools_ipc::PlatformNotification::CommandResult {
                        correlation_id: request.id,
                        result: PlatformCommandResult {
                            success: true,
                            error: None,
                            native_error_code: None,
                            data: Some(response),
                        },
                    },
                    cx,
                )
            });
            draw(surface);
            for preference in [
                nxtools_core::ThemePreference::Light,
                nxtools_core::ThemePreference::Dark,
            ] {
                surface.update(|_, cx| crate::theme::set_theme_preference(preference, cx));
                draw(surface);
                for selector in ["extension-trust-recheck", "extension-trust-browse"] {
                    let bounds = surface
                        .debug_bounds(selector)
                        .expect("real recovery action");
                    assert_eq!(bounds.size.height, px(32.0));
                    let painted = surface.update(|window, _| {
                        let scale = window.scale_factor();
                        window.painted_quads().iter().any(|quad| {
                            quad.border_color.a > 0.0
                                && quad.border_widths.top.as_f32() > 0.0
                                && (quad.bounds.origin.x.as_f32() / scale - bounds.left().as_f32())
                                    .abs()
                                    < 1.0
                                && (quad.bounds.origin.y.as_f32() / scale - bounds.top().as_f32())
                                    .abs()
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
                        "{selector} must retain visible official outline in {preference:?}"
                    );
                }
            }
            assert_eq!(std::fs::read(&path).expect("discovery no write"), original);
            let warning = surface
                .debug_bounds("extension-trust-warning")
                .expect("warning");
            let confirm = surface
                .debug_bounds("extension-approve-runtime")
                .expect("confirm");
            assert_eq!(confirm.size.height, px(32.0));
            assert!(warning.top() >= px(0.0) && warning.bottom() <= confirm.top());
            assert!(surface.update(|window, cx| window.has_active_dialog(cx)));
            assert!(
                surface.update(|window, _| gpui_kit::base::test_support::snapshots(window)
                    .iter()
                    .all(|n| n.label() != Some("插件运行时完整路径"))),
                "trust does not render readonly path as disabled input"
            );
            if cancel {
                view.update_in(surface, |view, _, _| {
                    view.trust_dialog
                        .as_ref()
                        .expect("draft")
                        .borrow_mut()
                        .attempted_path = Some(runtime.clone());
                });
                click(surface, "extension-trust-recheck");
                let explicit = endpoint
                    .try_receive_command_for_contract_test()
                    .expect("explicit probe");
                app.update_in(surface, |app, _, cx| {
                    app.handle_platform_notification(
                        nxtools_ipc::PlatformNotification::CommandResult {
                            correlation_id: explicit.id,
                            result: PlatformCommandResult {
                                success: false,
                                error: Some("missing".into()),
                                native_error_code: Some(2),
                                data: None,
                            },
                        },
                        cx,
                    )
                });
                draw(surface);
                click(surface, "extension-trust-recheck");
                let retry = endpoint
                    .try_receive_command_for_contract_test()
                    .expect("explicit retry");
                let nxtools_protocol::Message::PlatformCommand(
                    PlatformCommand::InspectExtensionRuntime(retry_command),
                ) = retry.message
                else {
                    panic!("retry inspect")
                };
                assert_eq!(
                    retry_command.executable,
                    runtime.to_string_lossy(),
                    "explicit failure must not fall back to PATH"
                );
                app.update_in(surface, |app, _, cx| {
                    app.handle_platform_notification(
                        nxtools_ipc::PlatformNotification::CommandResult {
                            correlation_id: retry.id,
                            result: PlatformCommandResult {
                                success: false,
                                error: Some("missing".into()),
                                native_error_code: Some(2),
                                data: None,
                            },
                        },
                        cx,
                    )
                });
                draw(surface);
                click(surface, "extension-trust-cancel");
                draw(surface);
                assert_eq!(std::fs::read(&path).expect("cancel bytes"), original);
            } else {
                click(surface, "extension-approve-runtime");
                idle(&app, surface);
            }
        }
        let saved = nxtools_core::AppConfig::load_from(&path).expect("saved");
        let package = &saved.extensions.packages[0];
        assert!(package.trust.is_some() && !package.enabled);
        assert!(saved.extensions.runtime_defaults.is_empty());
        assert_eq!(
            package.runtime_override.as_ref(),
            Some(&identity.executable)
        );
        assert!(!surface.update(|window, cx| window.has_active_dialog(cx)));
    }
}
