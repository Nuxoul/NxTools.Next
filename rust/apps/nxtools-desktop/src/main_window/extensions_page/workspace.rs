//! Business workspace: released Toolbar/Form/ListItem/Empty; Settings is reserved for detail.
//! No replacement list/search controller: the stable InputState filters immutable package projections.
use super::*;
use gpui_kit::component::empty::{Empty, EmptyDescription, EmptyHeader, EmptyTitle};
use gpui_kit::component::toolbar::Toolbar;

impl ExtensionManagerView {
    pub(super) fn render_workspace(
        &self,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> gpui_kit::AnyElement {
        let state = self.application.read(cx);
        let busy = state.busy;
        let packages = state.packages.packages.clone();
        let preview = state.preview.is_some();
        let query = self.search.read(cx).value().trim().to_lowercase();
        let view = cx.entity().downgrade();
        let owner = self.application.downgrade();
        let import_owner = owner.clone();
        let toolbar = Toolbar::new("extension-workspace-toolbar")
            .with_size(gpui_kit::component::Size::Medium)
            .w_full()
            .p_0()
            .content(div().flex_1().min_w_0().text_lg().child("扩展"))
            .content(
                div()
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(extensions_settings::view_button(
                        "extension-import-open",
                        "导入扩展",
                        busy || preview,
                        view.clone(),
                        move |view, _, cx| {
                            view.import_open = true;
                            let owner = import_owner.clone();
                            cx.defer(move |cx| {
                                let _ = owner.update(cx, |app, cx| app.pick_manifest(cx));
                            });
                        },
                    ))
                    .child(extensions_settings::app_button(
                        "extension-directory-open",
                        "打开插件目录",
                        busy || state.opening_directory(),
                        owner.clone(),
                        |app, cx| app.open_directory(cx),
                    ))
                    .child(extensions_settings::view_button(
                        "extension-environment-open",
                        "运行环境",
                        busy,
                        view.clone(),
                        |view, window, cx| view.open_environment(None, window, cx),
                    ))
                    .child(self.chain_entry_button(cx))
                    .child(self.activity_button().outline().disabled(busy)),
            );
        let mut body = div()
            .id("extension-workspace-scroll")
            .flex_1()
            .min_h_0()
            .w_full()
            .min_w_0()
            .overflow_y_scroll()
            .track_scroll(&self.workspace_scroll)
            .flex()
            .flex_col()
            .gap_3();
        let has_preview = preview;
        if self.import_open && !preview {
            let path = self.path.clone();
            let preview_path = path.clone();
            let preview_owner = owner.clone();
            let pick = owner.clone();
            body = body.child(
                div()
                    .flex_shrink_0()
                    .w_full()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child("手动检查 ZIP 路径")
                            .child(extensions_settings::view_button(
                                "extension-import-collapse",
                                "收起",
                                busy,
                                view.clone(),
                                |view, _, _| view.import_open = false,
                            )),
                    )
                    .gap_3()
                    .child(
                        div()
                            .debug_selector(|| "extension-permission-warning".into())
                            .child(
                                Alert::warning("extension-trust-warning", TRUST_WARNING)
                                    .title("导入前检查来源")
                                    .small(),
                            ),
                    )
                    .child(
                        Form::new().child(
                            Field::new().label("ZIP 扩展包").child(
                                div()
                                    .w_full()
                                    .min_w_0()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(
                                        div()
                                            .debug_selector(|| {
                                                "extension-import-path-control".into()
                                            })
                                            .min_w_0()
                                            .flex_1()
                                            .child(
                                                Input::new(&path)
                                                    .aria_label("ZIP 扩展包路径")
                                                    .disabled(busy),
                                            ),
                                    )
                                    .child(
                                        extensions_settings::app_button(
                                            "extension-pick-file",
                                            "选择 ZIP",
                                            busy,
                                            pick,
                                            |app, cx| app.pick_manifest(cx),
                                        )
                                        .with_size(gpui_kit::component::Size::Medium),
                                    )
                                    .child(
                                        Button::new("extension-import-preview")
                                            .debug_selector(|| "extension-import-preview".into())
                                            .label("检查扩展包")
                                            .disabled(busy)
                                            .on_click(move |_, _, cx| {
                                                let path = PathBuf::from(
                                                    preview_path.read(cx).value().as_ref(),
                                                );
                                                let owner = preview_owner.clone();
                                                cx.defer(move |cx| {
                                                    let _ = owner.update(cx, |app, cx| {
                                                        app.preview_path(path, cx)
                                                    });
                                                });
                                            }),
                                    ),
                            ),
                        ),
                    ),
            );
        }
        let filtered = packages
            .iter()
            .filter(|package| package_matches(package, &query))
            .collect::<Vec<_>>();
        let count = filtered.len();
        if !packages.is_empty() {
            body = body.child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .text_sm()
                    .child(format!(
                        "已安装 {} 个 · 已启用 {} 个",
                        packages.len(),
                        packages.iter().filter(|p| p.enabled).count()
                    ))
                    .child("手动插件只在您选择动作后运行"),
            );
        }
        if !filtered.is_empty() {
            body = body.child(
                div()
                    .id("extension-catalog-grid")
                    .debug_selector(|| "extension-catalog-grid".into())
                    .w_full()
                    .min_w_0()
                    .flex_shrink_0()
                    .grid()
                    .grid_cols(if window.viewport_size().width >= px(620.0) {
                        2
                    } else {
                        1
                    })
                    .gap_3()
                    .children(
                        filtered
                            .into_iter()
                            .map(|package| self.catalog_cell(package, cx)),
                    ),
            );
        }
        if count == 0 && !self.import_open && !has_preview {
            let empty = packages.is_empty();
            body = body.child(
                Empty::new()
                    .header(
                        EmptyHeader::new()
                            .title(EmptyTitle::new().child(if empty {
                                "尚未安装扩展"
                            } else {
                                "没有匹配的扩展"
                            }))
                            .description(EmptyDescription::new().child(if empty {
                                "从本地导入命令包或插件。导入后可检查功能，再独立信任和启用。"
                            } else {
                                "尝试名称、用途、入口或关键词，或清除搜索查看全部扩展。"
                            })),
                    )
                    .child(extensions_settings::view_button(
                        "extension-empty-action",
                        if empty {
                            "导入第一个扩展"
                        } else {
                            "清除搜索"
                        },
                        busy,
                        view.clone(),
                        move |view, window, cx| {
                            if empty {
                                view.import_open = true;
                                let owner = view.application.downgrade();
                                cx.defer(move |cx| {
                                    let _ = owner.update(cx, |app, cx| app.pick_manifest(cx));
                                });
                            } else {
                                view.search
                                    .update(cx, |input, cx| input.set_value("", window, cx));
                            }
                        },
                    )),
            );
        }
        div()
            .size_full()
            .min_w_0()
            .flex()
            .flex_col()
            .gap_3()
            .p_4()
            .child(toolbar)
            .child(
                Input::new(&self.search)
                    .aria_label("搜索已安装扩展（名称、用途、入口或关键词）")
                    .prefix(gpui_kit::component::IconName::Search)
                    .disabled(busy),
            )
            .child(body)
            .into_any_element()
    }

    pub(super) fn package_commands(
        &self,
        package: &nxtools_extensions::InstalledPackage,
        busy: bool,
        cx: &Context<Self>,
    ) -> SettingGroup {
        let package = package.clone();
        let view = cx.entity().downgrade();
        let owner = self.application.downgrade();
        let aliases = self.aliases.clone();
        let target = self.alias_target.clone();
        SettingGroup::new().title("命令与工具").item(
            SettingItem::render(move |_, _, _| {
                let mut content = div().w_full().min_w_0().flex().flex_col().gap_3();
                for handler in &package.manifest.handlers {
                    let mut row = div()
                        .w_full()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(usage::handler_heading(handler))
                        .child(usage::copy(
                            format!("extension-command-trigger-{}", handler.id),
                            usage::trigger(&package, handler),
                        ));
                    if handler.feature == "quick_launch" {
                        let package_id = package.manifest.id.clone();
                        let handler_id = handler.id.clone();
                        let keywords = package.keywords(handler).join(" ");
                        row = row.child(format!("关键词：{keywords}"));
                        if target.as_ref() == Some(&(package_id.clone(), handler_id.clone())) {
                            let input = aliases.clone();
                            row = row.child(
                                Form::new()
                                    .child(
                                        Field::new().label("新关键词").child(
                                            Input::new(&aliases)
                                                .aria_label("扩展命令关键词")
                                                .disabled(busy),
                                        ),
                                    )
                                    .footer(
                                        div()
                                            .flex()
                                            .flex_wrap()
                                            .gap_2()
                                            .child(extensions_settings::view_button(
                                                "extension-save-aliases",
                                                "保存关键词",
                                                busy,
                                                view.clone(),
                                                move |view, _, cx| {
                                                    let aliases = input
                                                        .read(cx)
                                                        .value()
                                                        .split_whitespace()
                                                        .map(str::to_owned)
                                                        .collect();
                                                    let owner = view.application.downgrade();
                                                    let id = package_id.clone();
                                                    let handler = handler_id.clone();
                                                    cx.defer(move |cx| {
                                                        let _ = owner.update(cx, |app, cx| {
                                                            app.set_aliases(
                                                                id, handler, aliases, cx,
                                                            )
                                                        });
                                                    });
                                                },
                                            ))
                                            .child(extensions_settings::view_button(
                                                "extension-cancel-aliases",
                                                "取消编辑",
                                                busy,
                                                view.clone(),
                                                |view, _, _| view.alias_target = None,
                                            )),
                                    ),
                            );
                        } else {
                            row = row.child(extensions_settings::view_button(
                                format!("extension-edit-{}-{}", package_id, handler_id),
                                "修改关键词",
                                busy,
                                view.clone(),
                                move |view, window, cx| {
                                    view.alias_target =
                                        Some((package_id.clone(), handler_id.clone()));
                                    view.aliases.update(cx, |input, cx| {
                                        input.set_value(keywords.clone(), window, cx);
                                        input.focus(window, cx);
                                    });
                                },
                            ));
                        }
                    }
                    if handler.feature == "tools" {
                        let choices = match &handler.action {
                            nxtools_extensions::DeclarativeAction::Items { items } => items
                                .iter()
                                .map(|item| (Some(item.id.clone()), item.title.clone()))
                                .collect::<Vec<_>>(),
                            _ => vec![(None, handler.title.clone())],
                        };
                        for (item, title) in choices {
                            let id = package.manifest.id.clone();
                            let handler_id = handler.id.clone();
                            row = row.child(extensions_settings::app_button(
                                format!(
                                    "extension-run-{}-{}-{}",
                                    id,
                                    handler_id,
                                    item.as_deref().unwrap_or("action")
                                ),
                                title,
                                busy || !package.enabled,
                                owner.clone(),
                                move |app, cx| {
                                    app.execute_tool_action(&id, &handler_id, item.as_deref(), cx)
                                },
                            ));
                        }
                    }
                    content = content.child(row);
                }
                content
            })
            .keywords(["命令", "工具", "关键词"]),
        )
    }
}

pub(super) fn feature_label(feature: &str) -> &str {
    match feature {
        "quick_launch" => "快速启动",
        "clipboard" => "剪贴板",
        "downloads" => "下载",
        "tools" => "工具",
        other => other,
    }
}

fn package_matches(package: &nxtools_extensions::InstalledPackage, query: &str) -> bool {
    query.is_empty()
        || package.manifest.name.to_lowercase().contains(query)
        || package.manifest.id.to_lowercase().contains(query)
        || package.manifest.description.to_lowercase().contains(query)
        || package.manifest.handlers.iter().any(|handler| {
            handler.title.to_lowercase().contains(query)
                || feature_label(&handler.feature).contains(query)
                || handler.feature.contains(query)
                || usage::entry_label(handler).contains(query)
                || usage::trigger(package, handler)
                    .to_lowercase()
                    .contains(query)
                || package
                    .keywords(handler)
                    .iter()
                    .any(|keyword| keyword.to_lowercase().contains(query))
        })
}
