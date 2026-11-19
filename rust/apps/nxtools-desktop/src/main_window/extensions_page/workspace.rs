//! Business workspace: released Toolbar/Form/ListItem/Empty; Settings is reserved for detail.
//! No replacement list/search controller: the stable InputState filters immutable package projections.
use super::*;
use gpui_kit::component::empty::{Empty, EmptyDescription, EmptyHeader, EmptyTitle};
use gpui_kit::component::toolbar::Toolbar;
use gpui_kit::component::{
    ActiveTheme as _, IndexPath, Size, StyledExt as _,
    menu::PopupMenuItem,
    select::Select,
    tab::{Tab, TabBar},
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum WorkspaceTab {
    Installed,
    Attention,
    Activity,
}

impl WorkspaceTab {
    const ALL: [Self; 3] = [Self::Installed, Self::Attention, Self::Activity];
}

pub(super) const LOCATIONS: [(&str, &str); 5] = [
    ("所有使用位置", ""),
    ("剪贴板", "clipboard"),
    ("下载", "downloads"),
    ("速启", "quick_launch"),
    ("其他工具", "tools"),
];

impl ExtensionManagerView {
    pub(super) fn render_workspace(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui_kit::AnyElement {
        let state = self.application.read(cx);
        let busy = state.busy;
        let packages = state.packages.packages.clone();
        let preview = state.preview.is_some();
        let query = self.search.read(cx).value().trim().to_lowercase();
        let view = cx.entity().downgrade();
        let owner = self.application.downgrade();
        let pending = packages
            .iter()
            .filter_map(|package| self.attention_reason(package, cx))
            .collect::<Vec<_>>();
        let selected_tab = WorkspaceTab::ALL
            .iter()
            .position(|tab| *tab == self.workspace_tab)
            .unwrap_or(0);
        let tab_view = view.clone();
        let tabs = TabBar::new("extension-workspace-tabs")
            .underline()
            .small()
            .selected_index(selected_tab)
            .children([
                Tab::new().label(format!("已安装 {}", packages.len())),
                Tab::new().label(format!("待处理 {}", pending.len())),
                Tab::new().label("调用记录"),
            ])
            .on_click(move |index, _, cx| {
                let Some(tab) = WorkspaceTab::ALL.get(*index).copied() else {
                    return;
                };
                let view = tab_view.clone();
                cx.defer(move |cx| {
                    let _ = view.update(cx, |view, cx| view.select_workspace_tab(tab, cx));
                });
            });
        let manage_view = view.clone();
        let manage = Button::new("extension-management")
            .debug_selector(|| "extension-management".into())
            .label("管理")
            .outline()
            .with_size(Size::Medium)
            .disabled(busy || preview)
            .dropdown_menu(move |menu, _, _| {
                let mut menu = menu;
                for (index, label) in ["运行环境", "下载解析链", "打开插件目录", "保留的扩展数据"]
                    .into_iter()
                    .enumerate()
                {
                    let manager = manage_view.clone();
                    menu = menu.item(PopupMenuItem::new(label).on_click(move |_, window, cx| {
                        let manager = manager.clone();
                        window.defer(cx, move |window, cx| {
                            let _ = manager.update(cx, |view, cx| {
                                match index {
                                    0 => view.open_environment(None, window, cx),
                                    1 => view.open_chain(window, cx),
                                    2 => {
                                        let owner = view.application.downgrade();
                                        cx.defer(move |cx| {
                                            let _ =
                                                owner.update(cx, |app, cx| app.open_directory(cx));
                                        });
                                    }
                                    _ => view.open_storage(None, cx),
                                }
                                cx.notify();
                            });
                        });
                    }));
                }
                menu
            });
        let toolbar = Toolbar::new("extension-workspace-toolbar")
            .with_size(Size::Medium)
            .w_full()
            .h_auto()
            .p_0()
            .content(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .child(div().text_xl().font_semibold().child("扩展"))
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child("管理剪贴板、下载和速启里的附加功能。"),
                    ),
            )
            .content(
                div().flex().items_center().gap_2().child(manage).child(
                    extensions_settings::view_button(
                        "extension-import-open",
                        "导入扩展",
                        busy || preview,
                        view.clone(),
                        |view, window, cx| view.start_workspace_import(window, cx),
                    )
                    .primary()
                    .with_size(Size::Medium),
                ),
            );
        let workspace = div()
            .size_full()
            .min_w_0()
            .flex()
            .flex_col()
            .gap_2()
            .p_4()
            .child(toolbar)
            .child(tabs);
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
        if self.workspace_tab == WorkspaceTab::Installed && !pending.is_empty() {
            let reasons = catalog::AttentionReason::ALL
                .into_iter()
                .filter_map(|reason| {
                    let count = pending.iter().filter(|value| **value == reason).count();
                    (count > 0).then(|| format!("{count} 项{}", reason.label()))
                })
                .collect::<Vec<_>>()
                .join("，");
            body = body.child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .flex_shrink_0()
                    .child(
                        div().min_w_0().flex_1().child(
                            Alert::new("extension-attention", reasons)
                                .title(format!("{} 项需要处理", pending.len()))
                                .small(),
                        ),
                    )
                    .child(extensions_settings::view_button(
                        "extension-attention-open",
                        "查看待处理",
                        false,
                        view.clone(),
                        |view, window, cx| {
                            view.clear_catalog_filters(window, cx);
                            view.select_workspace_tab(WorkspaceTab::Attention, cx);
                        },
                    )),
            );
        }
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
                                        extensions_settings::view_button(
                                            "extension-pick-file",
                                            "选择 ZIP",
                                            busy,
                                            view.clone(),
                                            move |_, window, cx| {
                                                let owner = pick.clone();
                                                let source_window_handle =
                                                    crate::ui::native_window_handle(window)
                                                        .unwrap_or_default();
                                                cx.defer(move |cx| {
                                                    let _ = owner.update(cx, |app, cx| {
                                                        app.pick_manifest(source_window_handle, cx)
                                                    });
                                                });
                                            },
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
        if self.workspace_tab == WorkspaceTab::Activity {
            return workspace
                .child(body.child(self.render_activity(window, cx)))
                .into_any_element();
        }
        let location = self
            .location
            .read(cx)
            .selected_index(cx)
            .map_or(0, |index| index.row);
        let feature = LOCATIONS.get(location).map_or("", |(_, feature)| *feature);
        let filtered = packages
            .iter()
            .filter(|package| {
                (feature.is_empty()
                    || package
                        .manifest
                        .handlers
                        .iter()
                        .any(|h| h.feature == feature))
                    && (self.workspace_tab != WorkspaceTab::Attention
                        || self.attention_reason(package, cx).is_some())
                    && package_matches(package, &query)
            })
            .collect::<Vec<_>>();
        let count = filtered.len();
        body = body.child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .flex_shrink_0()
                .child(
                    div().flex_1().min_w_0().max_w(px(365.0)).child(
                        Input::new(&self.search)
                            .aria_label("搜索扩展")
                            .prefix(gpui_kit::component::IconName::Search),
                    ),
                )
                .child(div().flex_1())
                .child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(format!("{count} 项")),
                )
                .child(
                    Select::new(&self.location)
                        .with_size(Size::Medium)
                        .w(px(142.0)),
                ),
        );
        if count > 0 {
            let width = window.viewport_size().width;
            let compact = window.viewport_size().height <= px(510.0);
            body = body.child(
                div()
                    .id("extension-catalog-grid")
                    .debug_selector(|| "extension-catalog-grid".into())
                    .w_full()
                    .min_w_0()
                    .flex_shrink_0()
                    .grid()
                    .grid_cols(if width >= px(980.0) {
                        3
                    } else if width >= px(600.0) {
                        2
                    } else {
                        1
                    })
                    .gap_2()
                    .children(
                        filtered
                            .into_iter()
                            .map(|package| self.catalog_cell(package, compact, cx)),
                    ),
            );
        }
        if count == 0 && !self.import_open && !has_preview {
            let empty = packages.is_empty();
            let clear = !query.is_empty() || location != 0;
            body = body.child(
                Empty::new()
                    .header(
                        EmptyHeader::new()
                            .title(EmptyTitle::new().child(if empty {
                                "尚未安装扩展"
                            } else if clear {
                                "没有匹配的扩展"
                            } else {
                                "目前没有待处理事项"
                            }))
                            .description(EmptyDescription::new().child(if empty {
                                "从本地导入命令包或插件。导入后可检查功能，再独立信任和启用。"
                            } else if clear {
                                "试试名称、用途、入口或关键词，或清除筛选。"
                            } else {
                                "缺设置、信任或恢复操作的扩展会出现在这里。"
                            })),
                    )
                    .child(extensions_settings::view_button(
                        "extension-empty-action",
                        if empty {
                            "导入第一个扩展"
                        } else if clear {
                            "清除筛选"
                        } else {
                            "返回已安装"
                        },
                        busy,
                        view.clone(),
                        move |view, window, cx| {
                            if empty {
                                view.start_workspace_import(window, cx);
                            } else if clear {
                                view.clear_catalog_filters(window, cx);
                            } else {
                                view.select_workspace_tab(WorkspaceTab::Installed, cx);
                            }
                        },
                    )),
            );
        }
        workspace.child(body).into_any_element()
    }

    fn select_workspace_tab(&mut self, tab: WorkspaceTab, cx: &mut Context<Self>) {
        if self.workspace_tab != tab {
            self.workspace_tab = tab;
            self.workspace_scroll
                .set_offset(gpui_kit::point(px(0.0), px(0.0)));
            cx.notify();
        }
    }

    fn clear_catalog_filters(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.search
            .update(cx, |input, cx| input.set_value("", window, cx));
        self.location.update(cx, |select, cx| {
            select.set_selected_index(Some(IndexPath::default().row(0)), window, cx)
        });
    }

    fn start_workspace_import(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.import_open = true;
        let owner = self.application.downgrade();
        let hwnd = crate::ui::native_window_handle(window).unwrap_or_default();
        cx.defer(move |cx| {
            let _ = owner.update(cx, |app, cx| app.pick_manifest(hwnd, cx));
        });
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
