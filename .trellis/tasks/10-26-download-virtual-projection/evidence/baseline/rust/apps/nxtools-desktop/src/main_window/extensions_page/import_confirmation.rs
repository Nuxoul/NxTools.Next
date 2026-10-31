//! Released Kit Dialog owns modal focus/keyboard; application owns the immutable candidate.
use super::*;
use gpui_kit::component::StyledExt;
use gpui_kit::component::{
    WindowExt,
    accordion::Accordion,
    description_list::DescriptionList,
    dialog::{DialogAction, DialogClose, DialogFooter},
    group_box::{GroupBox, GroupBoxVariant, GroupBoxVariants},
};
use std::cell::Cell;

impl ExtensionManagerView {
    pub(super) fn reconcile_import_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let identity = self.application.read(cx).import_candidate_identity();
        if identity == self.import_dialog {
            return;
        }
        if self.import_dialog.take().is_some() {
            let source = window.window_handle();
            let view = cx.entity().downgrade();
            cx.defer(move |cx| {
                let _ = source.update(cx, |_, window, cx| {
                    if window.has_active_dialog(cx) {
                        window.close_dialog(cx);
                    }
                    let _ = view.update(cx, |_, cx| cx.notify());
                });
            });
            return;
        }
        let Some(identity) = identity else { return };
        let Some((preview, source_path)) = self.application.read(cx).preview.clone() else {
            return;
        };
        self.import_dialog = Some(identity.clone());
        let owner = self.application.downgrade();
        let view = cx.entity().downgrade();
        let source = window.window_handle();
        let previous = self
            .application
            .read(cx)
            .packages
            .packages
            .iter()
            .find(|p| p.manifest.id == preview.manifest.id)
            .map(|p| p.content_hash.clone());
        cx.defer(move |cx| {
            let _ = source.update(cx, |_, window, cx| {
                let current = view
                    .upgrade()
                    .is_some_and(|view| view.read(cx).import_dialog.as_ref() == Some(&identity));
                if !current {
                    return;
                }
                let submitted = Rc::new(Cell::new(false));
                let details = Rc::new(Cell::new(false));
                window.open_dialog(cx, move |dialog, _, cx| {
                    let app = owner.upgrade();
                    let state = app.as_ref().map(|app| app.read(cx));
                    let busy = state.is_some_and(|app| app.busy);
                    let status = state.and_then(|app| app.import_failure.clone());
                    if !busy && status.is_some() {
                        submitted.set(false);
                    }
                    let eligible = state.is_some_and(|app| {
                        app.import_candidate_identity().as_ref() == Some(&identity)
                    });
                    let replace = previous.is_some();
                    let confirm_owner = owner.clone();
                    let confirm_id = identity.clone();
                    let expected = previous.clone();
                    let latch = submitted.clone();
                    let cancel_owner = owner.clone();
                    let cancel_id = identity.clone();
                    let close_view = view.clone();
                    let close_id = identity.clone();
                    let cancel_latch = submitted.clone();
                    let cancel_disabled = busy || submitted.get();
                    let confirm = DialogAction::new().child(
                        Button::new("extension-confirm-import")
                            .debug_selector(|| "extension-confirm-import".into())
                            .label(if replace {
                                "替换扩展"
                            } else {
                                "导入扩展"
                            })
                            .primary()
                            .disabled(busy || submitted.get() || !eligible),
                    );
                    let cancel = DialogClose::new().trigger(move |button| {
                        button
                            .debug_selector(|| "extension-cancel-import".into())
                            .label("取消")
                            .disabled(cancel_disabled)
                    });
                    let mut body =
                        confirmation_body(&preview, &source_path, replace, details.clone());
                    if let Some(message) = status {
                        body = body.child(Alert::error("extension-import-error", message).small());
                    }
                    dialog
                        .title(if replace {
                            "替换扩展"
                        } else {
                            "导入扩展"
                        })
                        .w(px(640.0))
                        .overlay_closable(false)
                        .close_button(!busy && !submitted.get())
                        .child(body)
                        .footer(
                            div()
                                .flex_shrink_0()
                                .flex()
                                .flex_col()
                                .gap_3()
                                .child(
                                    div()
                                        .debug_selector(|| {
                                            "extension-import-permission-warning".into()
                                        })
                                        .child(
                                            Alert::warning(
                                                "extension-import-authority",
                                                TRUST_WARNING.replace("。以下", "。\n以下"),
                                            )
                                            .whitespace_normal()
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
                            if latch.get() {
                                return false;
                            }
                            let can_submit = confirm_owner.upgrade().is_some_and(|app| {
                                !app.read(cx).busy
                                    && app.read(cx).import_candidate_identity().as_ref()
                                        == Some(&confirm_id)
                            });
                            if !can_submit {
                                return false;
                            }
                            latch.set(true);
                            let owner = confirm_owner.clone();
                            let identity = confirm_id.clone();
                            let expected = expected.clone();
                            cx.defer(move |cx| {
                                let _ = owner.update(cx, |app, cx| {
                                    app.confirm_import_current(identity, expected, cx)
                                });
                            });
                            false
                        })
                        .on_cancel(move |_, _, cx| {
                            if cancel_latch.get()
                                || cancel_owner.upgrade().is_some_and(|app| app.read(cx).busy)
                            {
                                return false;
                            }
                            let owner = cancel_owner.clone();
                            let identity = cancel_id.clone();
                            cx.defer(move |cx| {
                                let _ = owner
                                    .update(cx, |app, cx| app.cancel_import_current(&identity, cx));
                            });
                            true
                        })
                        .on_close(move |_, _, cx| {
                            let view = close_view.clone();
                            let identity = close_id.clone();
                            cx.defer(move |cx| {
                                let _ = view.update(cx, |view, cx| {
                                    if view.import_dialog.as_ref() == Some(&identity) {
                                        view.import_dialog = None;
                                    }
                                    cx.notify();
                                });
                            });
                        })
                });
            });
        });
    }
}

fn confirmation_body(
    preview: &nxtools_extensions::ImportPreview,
    source: &str,
    replace: bool,
    details: Rc<Cell<bool>>,
) -> gpui_kit::Div {
    let manifest = &preview.manifest;
    let filename = std::path::Path::new(source)
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "本地 ZIP 扩展包".into());
    let runtime = manifest
        .runtime
        .as_ref()
        .map_or("无需解释器", |r| match r.program {
            nxtools_extensions::ScriptProgram::Python => "Python",
            nxtools_extensions::ScriptProgram::Node => "Node.js",
            nxtools_extensions::ScriptProgram::Powershell => "Windows PowerShell",
            nxtools_extensions::ScriptProgram::Pwsh => "PowerShell 7 / pwsh",
            nxtools_extensions::ScriptProgram::Exe => "包内 EXE",
        });
    let summary = DescriptionList::new()
        .columns(1)
        .bordered(false)
        .label_width(px(80.0))
        .item("扩展包", filename, 1)
        .item("运行环境", runtime, 1);
    let mut features = GroupBox::new()
        .title("包含的功能")
        .with_variant(GroupBoxVariant::Normal);
    for handler in &manifest.handlers {
        features = features.child(
            div()
                .min_w_0()
                .flex()
                .flex_col()
                .gap_1()
                .child(handler.title.clone())
                .child(
                    gpui_kit::component::dialog::DialogDescription::new()
                        .child(workspace::feature_label(&handler.feature).to_owned()),
                ),
        );
    }
    let mut permissions = GroupBox::new()
        .title("作者声明的权限")
        .with_variant(GroupBoxVariant::Normal);
    if manifest.permissions.is_empty() {
        permissions = permissions.child("作者未声明权限；这不代表系统限制访问。");
    }
    for permission in &manifest.permissions {
        permissions = permissions.child(
            div()
                .min_w_0()
                .flex()
                .flex_col()
                .gap_1()
                .child(permission_label(&permission.id))
                .child(
                    gpui_kit::component::dialog::DialogDescription::new()
                        .child(permission.reason.clone()),
                ),
        );
    }
    let mut technical = div()
        .debug_selector(|| "extension-import-technical-content".into())
        .min_w_0()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            DescriptionList::new()
                .columns(1)
                .bordered(false)
                .label_width(px(80.0))
                .item("扩展 ID", manifest.id.clone(), 1)
                .item(
                    "来源路径",
                    div().min_w_0().child(source.to_owned()).into_any_element(),
                    1,
                )
                .item(
                    "SHA-256",
                    div()
                        .min_w_0()
                        .child(preview.content_hash.clone())
                        .into_any_element(),
                    1,
                ),
        )
        .child(runtime_description(manifest.runtime.as_ref()));
    for handler in &manifest.handlers {
        technical = technical.child(action_description(&handler.action));
    }
    let opened = details.get();
    let disclosure = Accordion::new("extension-import-technical")
        .w_full()
        .h_auto()
        .item(move |item| item.title("技术详情").open(opened).child(technical))
        .on_toggle_click(move |indices, window, _| {
            details.set(indices.contains(&0));
            window.refresh();
        });
    let mut body = div()
        .debug_selector(|| "extension-import-review".into())
        .min_w_0()
        .flex()
        .flex_col()
        .gap_4()
        .child(
            div()
                .flex_shrink_0()
                .flex()
                .flex_col()
                .gap_1()
                .child(div().text_lg().font_semibold().child(manifest.name.clone()))
                .child(
                    gpui_kit::component::dialog::DialogDescription::new()
                        .child(format!("版本 {}", manifest.version)),
                ),
        );
    if !manifest.description.is_empty() {
        body = body.child(manifest.description.clone());
    }
    body.child(
        Alert::new(
            "extension-import-outcome",
            if replace {
                "将替换已安装的同 ID 扩展。替换后保持停用，需要重新检查并信任。"
            } else {
                "导入后保持停用。脚本扩展需先配置运行环境，再单独确认信任和启用。"
            },
        )
        .small(),
    )
    .child(summary)
    .child(features)
    .child(permissions)
    .child(
        div()
            .debug_selector(|| "extension-import-technical-toggle".into())
            .child(disclosure),
    )
}
fn permission_label(id: &str) -> String {
    let title = match id {
        "clipboard.read" => "读取所选剪贴板条目",
        "clipboard.write" => "创建或写入剪贴板内容",
        "network" => "访问网络",
        "filesystem.read" => "读取文件",
        "filesystem.write" => "写入文件",
        _ => return id.to_owned(),
    };
    format!("{title}（{id}）")
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::{TestAppContext, VisualTestContext, size};
    fn candidate() -> nxtools_extensions::ImportPreview {
        nxtools_extensions::ImportPreview::from_json(br#"{"schema_version":1,"id":"fixture.confirm","name":"Fixture","version":"1.0.0","handlers":[{"id":"open","title":"Open documentation","feature":"quick_launch","entry":"command","keywords":["confirm"],"action":{"type":"website","url_template":"https://example.test/{query}"}}]}"#).expect("candidate")
    }
    pub(super) fn mount(
        path: PathBuf,
        cx: &mut TestAppContext,
    ) -> (Entity<ExtensionApplication>, &mut VisualTestContext) {
        cx.update(gpui_kit::init);
        cx.executor().allow_parking();
        let app = cx.new(|_| {
            let mut app = ExtensionApplication::new(
                nxtools_core::AppConfig::default(),
                path,
                None,
                gpui_kit::WeakEntity::new_invalid(),
            );
            app.preview = Some((candidate(), "C:\\Synthetic\\package.zip".into()));
            app
        });
        let owner = app.clone();
        let (_, surface) = cx.add_window_view(move |window, cx| {
            let view = cx.new(|cx| ExtensionManagerView::new(owner, window, cx));
            gpui_kit::base::Root::new(view, window, cx)
        });
        surface.simulate_resize(size(px(744.0), px(430.0)));
        draw(surface);
        (app, surface)
    }
    pub(super) fn draw(surface: &mut VisualTestContext) {
        surface.update(|window, cx| window.draw(cx).clear(cx));
        surface.run_until_parked();
        surface.update(|window, cx| window.draw(cx).clear(cx));
    }
    fn click(surface: &mut VisualTestContext, selector: &'static str) {
        draw(surface);
        let bounds = surface.debug_bounds(selector).expect("action");
        assert!(bounds.top() >= px(0.0) && bounds.bottom() <= px(430.0));
        surface.simulate_click(bounds.center(), gpui_kit::Modifiers::default());
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
    fn confirmation_failure_keeps_dialog_retry_and_double_confirm_commits_once(
        cx: &mut TestAppContext,
    ) {
        let temp = tempfile::tempdir().expect("owned");
        let path = temp.path().join("config.json");
        let (app, surface) = mount(path.clone(), cx);
        let center = surface.update(|_, cx| {
            let center = cx.new(|_| crate::notifications::NotificationCenter::default());
            crate::notifications::register(&center, cx);
            center
        });
        click(surface, "extension-confirm-import");
        idle(&app, surface);
        assert!(
            surface.update(|window, cx| window.has_active_dialog(cx)),
            "failure keeps dialog"
        );
        surface.read(|cx| {
            let app = app.read(cx);
            assert_eq!(app.last_commit, Some(("import", false, false)));
            assert!(app.import_failure.is_some(), "candidate remains retryable");
            assert!(app.status.is_none(), "no page-wide saved/error row");
            assert!(
                center
                    .read(cx)
                    .items()
                    .iter()
                    .any(|notice| notice.level == crate::notifications::NotificationLevel::Error)
            );
        });
        assert!(
            !temp.path().join("extensions").exists(),
            "missing config fails before staging"
        );
        nxtools_core::AppConfig::default()
            .save_to(&path)
            .expect("repair owned config");
        surface.simulate_keystrokes("enter enter");
        idle(&app, surface);
        assert_eq!(
            nxtools_core::AppConfig::load_from(&path)
                .expect("reopen")
                .extensions
                .packages
                .len(),
            1
        );
        assert!(
            !surface.update(|window, cx| window.has_active_dialog(cx)),
            "success closes dialog"
        );
    }
    #[gpui_kit::test]
    fn confirmation_escape_and_stale_generation_never_install_reopened_same_package(
        cx: &mut TestAppContext,
    ) {
        let temp = tempfile::tempdir().expect("owned");
        let path = temp.path().join("config.json");
        nxtools_core::AppConfig::default()
            .save_to(&path)
            .expect("config");
        let original = std::fs::read(&path).expect("bytes");
        let (app, surface) = mount(path.clone(), cx);
        let old = surface.read(|cx| app.read(cx).import_candidate_identity().expect("identity"));
        surface.simulate_keystrokes("escape");
        draw(surface);
        assert!(!surface.update(|window, cx| window.has_active_dialog(cx)));
        assert!(surface.read(|cx| app.read(cx).preview.is_none()));
        app.update_in(surface, |app, _, cx| {
            app.preview = Some((candidate(), "C:\\Synthetic\\package.zip".into()));
            app.confirm_import_current(old.clone(), None, cx);
            cx.notify();
        });
        draw(surface);
        assert_eq!(std::fs::read(&path).expect("no stale commit"), original);
        assert!(
            surface.read(|cx| app.read(cx).import_candidate_identity().expect("new").0 > old.0)
        );
        click(surface, "extension-cancel-import");
        draw(surface);
        assert_eq!(std::fs::read(&path).expect("cancel bytes"), original);
        assert!(!temp.path().join("extensions").exists());
    }
    #[gpui_kit::test]
    fn confirmation_competing_install_failure_still_allows_cancel(cx: &mut TestAppContext) {
        let temp = tempfile::tempdir().expect("owned");
        let path = temp.path().join("config.json");
        nxtools_core::AppConfig::default()
            .save_to(&path)
            .expect("config");
        let (app, surface) = mount(path.clone(), cx);
        let mut disk = nxtools_core::AppConfig::load_from(&path).expect("disk");
        disk.extensions
            .install(candidate(), "other writer".into(), false)
            .expect("install");
        disk.save_to(&path).expect("other writer commit");
        let bytes = std::fs::read(&path).expect("bytes");
        click(surface, "extension-confirm-import");
        idle(&app, surface);
        assert!(surface.read(|cx| app.read(cx).import_failure.is_some()));
        assert!(surface.update(|window, cx| window.has_active_dialog(cx)));
        surface.simulate_keystrokes("escape");
        draw(surface);
        assert!(!surface.update(|window, cx| window.has_active_dialog(cx)));
        assert_eq!(std::fs::read(path).expect("no overwrite"), bytes);
    }
    #[gpui_kit::test]
    fn retiring_pending_validation_discards_late_candidate_without_releasing_busy_early(
        cx: &mut TestAppContext,
    ) {
        let temp = tempfile::tempdir().expect("owned");
        let path = temp.path().join("config.json");
        nxtools_core::AppConfig::default()
            .save_to(&path)
            .expect("config");
        let (app, surface) = mount(path, cx);
        surface.simulate_keystrokes("escape");
        draw(surface);
        let zip = temp.path().join("candidate.zip");
        super::super::write_zip(
            &zip,
            &serde_json::to_vec(&candidate().manifest).expect("manifest"),
        );
        app.update_in(surface, |app, _, cx| {
            app.preview_path(zip, cx);
            app.retire_import_preview(cx);
            assert!(app.busy, "worker still owns admission");
        });
        idle(&app, surface);
        assert!(surface.read(|cx| app.read(cx).preview.is_none()));
        assert!(!surface.update(|window, cx| window.has_active_dialog(cx)));
    }
}

#[cfg(test)]
mod detail_tests {
    use super::*;
    use gpui_kit::TestAppContext;
    #[gpui_kit::test]
    fn technical_disclosure_scrolls_expands_and_close_preserves_config(cx: &mut TestAppContext) {
        use gpui_kit::component::WindowExt;
        let temp = tempfile::tempdir().expect("owned");
        let path = temp.path().join("config.json");
        nxtools_core::AppConfig::default()
            .save_to(&path)
            .expect("config");
        let original = std::fs::read(&path).expect("bytes");
        let (app, surface) = tests::mount(path.clone(), cx);
        let mut found = false;
        for _ in 0..30 {
            tests::draw(surface);
            let warning = surface
                .debug_bounds("extension-import-permission-warning")
                .expect("warning");
            if let Some(bounds) = surface.debug_bounds("extension-import-technical-toggle")
                && bounds.top() >= px(50.0)
                && bounds.bottom() <= warning.top()
            {
                let before = bounds.size.height;
                surface.simulate_click(
                    gpui_kit::point(bounds.left() + px(80.0), bounds.top() + px(16.0)),
                    gpui_kit::Modifiers::default(),
                );
                surface
                    .executor()
                    .advance_clock(std::time::Duration::from_secs(2));
                tests::draw(surface);
                let after = surface
                    .debug_bounds("extension-import-technical-toggle")
                    .expect("expanded");
                assert!(
                    after.size.height > before,
                    "actual accordion disclosure expands"
                );
                found = true;
                break;
            }
            surface.simulate_event(gpui_kit::ScrollWheelEvent {
                position: gpui_kit::point(px(500.0), px(220.0)),
                delta: gpui_kit::ScrollDelta::Lines(gpui_kit::point(0.0, -3.0)),
                ..Default::default()
            });
        }
        assert!(
            found,
            "technical details reachable through real dialog scroll"
        );
        surface.simulate_keystrokes("escape");
        tests::draw(surface);
        assert!(!surface.update(|window, cx| window.has_active_dialog(cx)));
        assert!(surface.read(|cx| app.read(cx).preview.is_none()));
        assert_eq!(std::fs::read(path).expect("unchanged"), original);
    }
}
