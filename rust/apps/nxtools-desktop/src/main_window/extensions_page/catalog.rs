//! Catalog cells and explicit state-driven entry guidance; no execution in render.
use super::*;
use gpui_kit::component::{WindowExt, label::Label, list::ListItem, tag::Tag};
use nxtools_extensions::InstalledPackage;

impl ExtensionManagerView {
    pub(super) fn catalog_cell(
        &self,
        package: &InstalledPackage,
        cx: &Context<Self>,
    ) -> AnyElement {
        let app = self.application.read(cx);
        let busy = app.busy;
        let id = package.manifest.id.clone();
        let needs_trust =
            package.manifest.runtime.is_some() && !extensions_settings::current_trust(package);
        let recent = app
            .recent_manual_runs()
            .into_iter()
            .find(|run| run.package_id == id);
        let pending = app
            .recent_manual_runs()
            .into_iter()
            .find(|run| run.package_id == id && run.state.pending());
        let paused = package
            .manifest
            .handlers
            .iter()
            .any(|handler| usage::is_event(handler) && app.event_paused(&id, &handler.id));
        let status = if pending.is_some() {
            "正在处理"
        } else if needs_trust {
            "待确认信任"
        } else if paused {
            "自动触发已暂停"
        } else if package.enabled {
            "已启用"
        } else {
            "未启用"
        };
        let tag = if needs_trust || paused {
            Tag::warning()
        } else if pending.is_some() {
            Tag::info()
        } else {
            Tag::secondary()
        };
        let feature = package
            .manifest
            .handlers
            .first()
            .map(|handler| handler.feature.as_str())
            .unwrap_or("");
        let icon = match feature {
            "clipboard" => MaterialIcon::ContentPaste,
            "downloads" => MaterialIcon::Download,
            "quick_launch" => MaterialIcon::Search,
            _ => MaterialIcon::DataObject,
        };
        let modes = package
            .manifest
            .handlers
            .iter()
            .map(usage::entry_label)
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>()
            .join(" / ");
        let hint = if pending.is_some() {
            "动作仍在处理中；可查看进度或取消这次调用。".into()
        } else if needs_trust {
            "先检查来源和运行环境，再决定是否信任。".into()
        } else if !package.enabled {
            String::new()
        } else if let Some(run) = &recent {
            format!("最近调用：{}", run.state.label())
        } else if package
            .manifest
            .handlers
            .iter()
            .any(|handler| handler.feature == "clipboard" && handler.entry == "action")
        {
            "选中剪贴板记录，在条目菜单中使用。".into()
        } else if package.manifest.handlers.iter().any(usage::is_event) {
            "自动触发需单独开启；在使用方法中查看。".into()
        } else if package
            .manifest
            .handlers
            .iter()
            .any(|h| h.feature == "quick_launch" && !package.keywords(h).is_empty())
        {
            let keywords = package
                .manifest
                .handlers
                .iter()
                .flat_map(|h| package.keywords(h))
                .map(String::as_str)
                .take(3)
                .collect::<Vec<_>>()
                .join("、");
            format!("在快速启动输入：{keywords}")
        } else {
            "查看使用方法，找到对应功能入口。".into()
        };
        let action_view = cx.entity().downgrade();
        let action_application = self.application.clone();
        let action_id = id.clone();
        let pending_id = pending.as_ref().map(|run| run.id);
        let enabled = package.enabled;
        ListItem::new(format!("extension-package-{id}"))
            .accessibility_label(format!("{}，{}", package.manifest.name, status))
            .w_full()
            .min_w_0()
            .p_4()
            .rounded_lg()
            .border_1()
            .border_color(Theme::active(cx).border_subtle)
            .bg(Theme::active(cx).surface)
            // The public suffix is a root-level slot; pin the official actions to the
            // content-derived grid cell, not its shorter, vertically centered body.
            .suffix(move |_, _| {
                let id = &action_id;
                let primary_id = id.clone();
                let primary = if let Some(run_id) = pending_id {
                    let owner = action_application.clone();
                    Button::new(format!("extension-next-{id}"))
                        .label("查看进度")
                        .outline()
                        .disabled(busy)
                        .on_click(move |_, window, cx| {
                            crate::extensions::manual_actions::show_run(
                                owner.clone(),
                                run_id,
                                None,
                                window,
                                cx,
                            )
                        })
                } else if needs_trust {
                    extensions_settings::view_button(
                        format!("extension-next-{id}"),
                        "检查并信任",
                        busy,
                        action_view.clone(),
                        move |view, window, cx| view.open_trust(primary_id.clone(), window, cx),
                    )
                } else if !enabled {
                    extensions_settings::app_button(
                        format!("extension-toggle-{id}"),
                        "启用插件",
                        busy,
                        action_application.downgrade(),
                        move |app, cx| app.set_enabled(primary_id.clone(), true, cx),
                    )
                } else {
                    extensions_settings::view_button(
                        format!("extension-next-{id}"),
                        "使用方法",
                        busy,
                        action_view.clone(),
                        move |view, window, cx| view.open_usage(primary_id.clone(), window, cx),
                    )
                };
                let manage_id = id.clone();
                let mut actions = div()
                    .absolute()
                    .left_4()
                    .right_4()
                    .bottom_4()
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(primary)
                    .child(extensions_settings::view_button(
                        format!("extension-manage-{id}"),
                        "详情",
                        busy,
                        action_view.clone(),
                        move |view, _, _| view.open_package(manage_id.clone()),
                    ));
                if enabled {
                    let toggle_id = id.clone();
                    actions = actions.child(extensions_settings::app_button(
                        format!("extension-toggle-{id}"),
                        "停用",
                        busy,
                        action_application.downgrade(),
                        move |app, cx| app.set_enabled(toggle_id.clone(), false, cx),
                    ));
                }
                actions
            })
            .child(
                div()
                    .min_w_0()
                    .pb_11()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(
                        div()
                            .flex()
                            .items_start()
                            .gap_3()
                            .child(
                                Icon::new(icon)
                                    .size(px(28.0))
                                    .text_color(Theme::active(cx).text_secondary),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .flex()
                                    .flex_col()
                                    .gap_1()
                                    .child(Label::new(package.manifest.name.clone()))
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(Theme::active(cx).text_secondary)
                                            .child(format!("版本 {}", package.manifest.version)),
                                    ),
                            )
                            .child(tag.outline().small().child(status)),
                    )
                    .child(
                        div()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(
                                div()
                                    .debug_selector(|| {
                                        format!(
                                            "extension-overview-purpose-{id}:{}",
                                            usage::purpose(package)
                                        )
                                    })
                                    .text_sm()
                                    .line_clamp(2)
                                    .child(usage::purpose(package).to_owned()),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(Theme::active(cx).text_secondary)
                                    .line_clamp(1)
                                    .child(modes),
                            )
                            .when(!hint.is_empty(), |body| {
                                body.child(
                                    div()
                                        .text_sm()
                                        .text_color(Theme::active(cx).text_secondary)
                                        .child(hint),
                                )
                            }),
                    ),
            )
            .into_any_element()
    }

    pub(super) fn open_usage(&self, id: String, window: &mut Window, cx: &mut Context<Self>) {
        let source = window.window_handle();
        let owner = self.application.clone();
        let view = cx.entity().downgrade();
        cx.defer(move|cx| {
            let Some(package)=owner.read(cx).packages.packages.iter().find(|p|p.manifest.id==id).cloned() else {return;};
            let _=source.update(cx,|_,window,cx| {
                window.open_dialog(cx,move|dialog,window,_| {
                    let mut body=div().id("extension-usage-guide").debug_selector(||"extension-usage-guide".into())
                        .max_h((window.viewport_size().height-px(200.0)).max(px(80.0)))
                        .overflow_y_scroll().flex().flex_col().gap_3()
                        .child(div().text_lg().child(package.manifest.name.clone()))
                        .child(usage::purpose(&package).to_owned());
                    for handler in &package.manifest.handlers {
                        body=body.child(div().flex().flex_col().gap_1()
                            .child(usage::handler_heading(handler))
                            .child(usage::copy(format!("extension-usage-trigger-{}",handler.id),usage::trigger(&package,handler))));
                    }
                    if package.manifest.handlers.iter().any(|h|h.feature=="clipboard"&&h.entry=="action") {
                        body=body.child(Alert::new("extension-usage-selection","请先在剪贴板历史选中要处理的记录；这里不会替您猜测输入或自动执行。结果与取消状态会在发起窗口显示。").small());
                    }
                    let settings_id=package.manifest.id.clone();let settings_view=view.clone();
                    let target_page=if package.manifest.handlers.iter().any(usage::is_event) {3}else{1};
                    let entry_id=package.manifest.id.clone();let entry_view=view.clone();
                    let entry=Button::new("extension-usage-entry").label(if target_page==3{"配置自动触发"}else{"查看动作与关键词"}).outline()
                        .on_click(move|_,window,cx|{
                            window.close_dialog(cx);let view=entry_view.clone();let id=entry_id.clone();
                            cx.defer(move|cx|{let _=view.update(cx,|view,cx|{view.open_package(id);view.detail_initial_page=target_page;cx.notify();});});
                        });
                    let close=Button::new("extension-usage-close").debug_selector(||"extension-usage-close".into()).label("知道了").outline().on_click(|_,window,cx|window.close_dialog(cx));
                    let settings=Button::new("extension-usage-settings").debug_selector(||"extension-usage-settings".into()).label("打开设置").outline().on_click(move|_,window,cx| {
                        window.close_dialog(cx);let view=settings_view.clone();let id=settings_id.clone();
                        let source=window.window_handle();
                        cx.defer(move|cx|{let _=source.update(cx,|_,window,cx|{let _=view.update(cx,|view,cx|{view.open_settings(id,window,cx);cx.notify();});});});
                    });
                    dialog.title("如何使用").w((window.viewport_size().width-px(32.0)).min(px(560.0)))
                        .child(body).footer(gpui_kit::component::dialog::DialogFooter::new().child(entry).child(settings).child(close))
                });
            });
        });
    }
}
