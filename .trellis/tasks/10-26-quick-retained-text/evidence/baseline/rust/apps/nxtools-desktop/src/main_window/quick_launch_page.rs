use super::*;
#[cfg(test)]
use crate::ui::components::tooltip::bounded_text_tooltip;
#[cfg(test)]
use crate::ui::typography::{LINE_HEIGHT_BODY, TEXT_BODY};
use gpui_kit::base::TestSupportExt;
use gpui_kit::component::group_box::GroupBoxVariant;
use gpui_kit::component::setting::{SelectIndex, SettingGroup, SettingItem, SettingPage, Settings};

#[cfg(test)]
pub(crate) fn quick_find_scroll_target(
    current: f32,
    previous_target: f32,
    maximum: f32,
    delta: f32,
) -> f32 {
    if maximum <= 0.0 {
        return 0.0;
    }
    let previous_target = previous_target.clamp(-maximum, 0.0);
    let base = if (previous_target - current).abs() <= QUICK_FIND_SCROLL_TARGET_SNAP_DISTANCE {
        previous_target
    } else {
        current
    };
    (base + delta).clamp(-maximum, 0.0)
}

#[cfg(test)]
pub(crate) fn quick_find_scroll_step(current: f32, target: f32) -> (f32, bool) {
    let remaining = target - current;
    if remaining.abs() <= 0.5 {
        (target, true)
    } else {
        (current + remaining * QUICK_FIND_SCROLL_EASING, false)
    }
}

pub(crate) fn exclusion_selection_after_removal(
    item_count: usize,
    removed_index: usize,
) -> Option<usize> {
    (item_count > 1).then(|| removed_index.min(item_count - 2))
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum QuickLaunchSettingsPage {
    #[default]
    General,
    Index,
    Context,
    Commands,
    Diagnostics,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct QuickLaunchSettingsPageMetadata {
    pub(crate) label: &'static str,
    pub(crate) description: &'static str,
    pub(crate) content_id: &'static str,
}

impl QuickLaunchSettingsPage {
    pub(crate) const ALL: [Self; 5] = [
        Self::General,
        Self::Index,
        Self::Context,
        Self::Commands,
        Self::Diagnostics,
    ];

    pub(crate) const fn metadata(self) -> QuickLaunchSettingsPageMetadata {
        match self {
            Self::General => QuickLaunchSettingsPageMetadata {
                label: "常规",
                description: "呼出方式与固定键盘操作",
                content_id: "quick-launch-settings-page-general",
            },
            Self::Index => QuickLaunchSettingsPageMetadata {
                label: "索引",
                description: "应用目录、文件索引与排除范围",
                content_id: "quick-launch-settings-page-index",
            },
            Self::Context => QuickLaunchSettingsPageMetadata {
                label: "上下文",
                description: "资源管理器与文件对话框联动",
                content_id: "quick-launch-settings-page-context",
            },
            Self::Commands => QuickLaunchSettingsPageMetadata {
                label: "命令",
                description: "维护关键词驱动的自定义动作",
                content_id: "quick-launch-settings-page-commands",
            },
            Self::Diagnostics => QuickLaunchSettingsPageMetadata {
                label: "诊断",
                description: "历史学习、查询性能与运行日志",
                content_id: "quick-launch-settings-page-diagnostics",
            },
        }
    }
}

pub(crate) fn grouped_count(value: u64) -> String {
    let digits = value.to_string();
    let mut grouped = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, character) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            grouped.push(',');
        }
        grouped.push(character);
    }
    grouped
}

pub(crate) fn format_query_elapsed(elapsed_us: Option<u64>) -> String {
    match elapsed_us {
        None => "尚无样本".into(),
        Some(elapsed_us) if elapsed_us < 1_000 => format!("{elapsed_us} µs"),
        Some(elapsed_us) => format!("{:.2} ms", elapsed_us as f64 / 1_000.0),
    }
}

pub(crate) fn file_index_summary(status: Option<&IndexStatus>) -> (String, &'static str) {
    let Some(status) = status else {
        return ("等待 NxTools.Indexer 连接".into(), "OFFLINE");
    };
    if let Some(error) = status.last_error.as_deref() {
        return (
            if status.indexed_items == 0 {
                format!("MFT 索引不可用：{error}")
            } else {
                format!("已索引 {} 项；部分卷失败：{error}", status.indexed_items)
            },
            if status.indexed_items == 0 {
                "ERROR"
            } else {
                "PARTIAL"
            },
        );
    }
    if status.phase == IndexPhase::Ready {
        return (
            format!(
                "已建立 {} 个 NTFS 卷快照 · {} 项",
                status.drives.len(),
                grouped_count(status.indexed_items)
            ),
            "READY",
        );
    }
    if status.indexed_items > 0 {
        return (
            format!(
                "正在更新新快照 · 旧索引 {} 项仍可搜索",
                grouped_count(status.indexed_items)
            ),
            "UPDATING",
        );
    }
    let drives = status
        .drives
        .iter()
        .map(|drive| drive.mount_point.trim_end_matches('\\'))
        .collect::<Vec<_>>()
        .join(" · ");
    if drives.is_empty() {
        ("未发现可建立 MFT 索引的 NTFS 卷".into(), "EMPTY")
    } else {
        (
            format!("已发现 {} 个 NTFS 卷：{drives}", status.drives.len()),
            "VOLUMES",
        )
    }
}

pub(crate) fn same_volume(left: &str, right: &str) -> bool {
    left.trim_end_matches(['\\', '/'])
        .eq_ignore_ascii_case(right.trim_end_matches(['\\', '/']))
}

pub(crate) fn volume_is_selected(configured: &[String], mount_point: &str) -> bool {
    configured.is_empty()
        || configured
            .iter()
            .any(|candidate| same_volume(candidate, mount_point))
}

pub(crate) fn volume_scope_selector_key(mount_point: &str) -> String {
    let key = mount_point
        .trim_end_matches(['\\', '/'])
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else {
                '-'
            }
        })
        .collect::<String>();
    key.trim_matches('-').to_owned()
}

pub(crate) fn toggle_indexed_volume_selection(
    configured: &[String],
    available: &[String],
    mount_point: &str,
) -> Vec<String> {
    let mut selected = if configured.is_empty() {
        available.to_vec()
    } else {
        configured.to_vec()
    };
    if let Some(index) = selected
        .iter()
        .position(|candidate| same_volume(candidate, mount_point))
    {
        let selected_available_count = available
            .iter()
            .filter(|available_mount| {
                selected
                    .iter()
                    .any(|candidate| same_volume(candidate, available_mount))
            })
            .count();
        if selected_available_count == 1 {
            return configured.to_vec();
        }
        selected.remove(index);
    } else {
        selected.push(mount_point.to_owned());
    }
    let exactly_all_available = !available.is_empty()
        && available.iter().all(|mount| {
            selected
                .iter()
                .any(|candidate| same_volume(candidate, mount))
        });
    if exactly_all_available {
        Vec::new()
    } else {
        selected
    }
}

impl MainWindow {
    pub(crate) fn handle_index_exclusion_keyboard(
        &mut self,
        event: &KeyDownEvent,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.shell.active_module() != MainModule::QuickFind
            || self.recording_hotkey.is_some()
            || event.keystroke.modifiers.modified()
        {
            return false;
        }
        let Some(application) = self.quick_launch.upgrade() else {
            self.selected_index_exclusion = None;
            return false;
        };
        let paths = application.read(cx).settings().excluded_paths.clone();
        if paths.is_empty() {
            self.selected_index_exclusion = None;
            return false;
        }
        let key = event.keystroke.key.to_ascii_lowercase();
        match key.as_str() {
            "up" => {
                self.selected_index_exclusion = Some(
                    self.selected_index_exclusion
                        .unwrap_or_default()
                        .saturating_sub(1),
                );
            }
            "down" => {
                self.selected_index_exclusion = Some(
                    self.selected_index_exclusion
                        .map_or(0, |index| (index + 1).min(paths.len() - 1)),
                );
            }
            "escape" => {
                if self.selected_index_exclusion.take().is_none() {
                    return false;
                }
            }
            "delete" | "backspace" => {
                let Some(index) = self
                    .selected_index_exclusion
                    .filter(|index| *index < paths.len())
                else {
                    return false;
                };
                let path = paths[index].clone();
                self.selected_index_exclusion =
                    exclusion_selection_after_removal(paths.len(), index);
                let quick_launch = self.quick_launch.clone();
                cx.defer(move |cx| {
                    let _ = quick_launch.update(cx, |application, cx| {
                        application.remove_excluded_path(&path, cx);
                    });
                });
            }
            _ => return false,
        }
        cx.stop_propagation();
        cx.notify();
        true
    }
}

impl MainWindow {
    pub(crate) fn render_quick_find(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let owner = cx.entity().downgrade();
        let item = |page: QuickLaunchSettingsPage,
                    slot: usize,
                    title: &'static str,
                    help: &'static str,
                    wide: bool| {
            let owner = owner.clone();
            let content = move |_: &gpui_kit::component::setting::RenderOptions,
                                window: &mut Window,
                                cx: &mut App| {
                let control = owner
                    .update(cx, |main, cx| {
                        main.render_quick_find_control(page, slot, cx)
                    })
                    .unwrap_or_else(|_| div().into_any_element());
                let mut body = div()
                    .id(gpui_kit::SharedString::from(format!(
                        "quick-settings-{}-{slot}",
                        page.metadata().label
                    )))
                    .test_support()
                    .role(gpui_kit::Role::Group)
                    .aria_label(page.metadata().label)
                    .debug_selector(move || page.metadata().content_id.into())
                    .w_full()
                    .min_w_0()
                    .when(!wide, |row| row.flex().items_center().justify_end())
                    .child(control);
                if page == QuickLaunchSettingsPage::Index
                    && slot == 4
                    && let Some(main) = owner.upgrade()
                {
                    body = body
                        .track_focus(&main.read(cx).exclusion_focus)
                        .on_key_down(move |event, _, cx| {
                            main.update(cx, |main, cx| {
                                main.handle_index_exclusion_keyboard(event, cx);
                            });
                        });
                }
                let _ = window;
                body
            };
            if wide {
                SettingItem::render(content).keywords([title, help])
            } else {
                SettingItem::render(move |options, window, cx| {
                    crate::ui::components::settings::aligned_setting_row(
                        format!("quick-setting-{title}"),
                        title,
                        help,
                        format!("quick-settings-{title}-help"),
                        crate::ui::metrics::ordinary_rem(280.0),
                        div()
                            .w_full()
                            .flex()
                            .items_center()
                            .justify_end()
                            .child(content(options, window, cx))
                            .into_any_element(),
                    )
                })
                .keywords([title, help])
            }
        };
        let page = |kind: QuickLaunchSettingsPage, group: SettingGroup| {
            SettingPage::new(kind.metadata().label)
                .resettable(false)
                .title_suffix(move |_, _| {
                    crate::ui::components::tooltip::description_help(
                        format!("quick-settings-{}-help", kind.metadata().label),
                        kind.metadata().description,
                    )
                })
                .group(group)
        };
        let row_owner = self.quick_launch.clone();
        let current_style = row_owner
            .upgrade()
            .map(|owner| owner.read(cx).settings().row_style)
            .unwrap_or_default();
        let row_setting = SettingItem::new(
            "结果列表样式",
            gpui_kit::component::setting::SettingField::scrollable_dropdown(
                nxtools_core::QuickLaunchRowStyle::ALL
                    .into_iter()
                    .map(|style| {
                        (
                            gpui_kit::SharedString::from(style.label()),
                            gpui_kit::SharedString::from(style.label()),
                        )
                    })
                    .collect(),
                move |_| gpui_kit::SharedString::from(current_style.label()),
                move |value: gpui_kit::SharedString, cx| {
                    if let Some(style) = nxtools_core::QuickLaunchRowStyle::ALL
                        .into_iter()
                        .find(|style| style.label() == value.as_ref())
                    {
                        let owner = row_owner.clone();
                        cx.defer(move |cx| {
                            let _ = owner.update(cx, |owner, cx| owner.set_row_style(style, cx));
                        });
                    }
                },
            ),
        )
        .description("连续列表没有条目间隙；分隔列表保留 4 DIP 小间隙，不改变内容行高。");
        let pages = vec![
            page(
                QuickLaunchSettingsPage::General,
                SettingGroup::new().title("呼出与执行").items([
                    item(
                        QuickLaunchSettingsPage::General,
                        0,
                        "全局呼出热键",
                        "点击字段开始录制；Esc 或失焦取消",
                        false,
                    ),
                    item(
                        QuickLaunchSettingsPage::General,
                        1,
                        "全屏游戏中暂不呼出",
                        "仅 Direct3D 独占全屏；无边框全屏仍可呼出速启",
                        false,
                    ),
                    item(QuickLaunchSettingsPage::General, 2, "键盘动作", "", false),
                    row_setting,
                ]),
            ),
            page(
                QuickLaunchSettingsPage::Index,
                SettingGroup::new().title("应用与文件索引").items([
                    item(QuickLaunchSettingsPage::Index, 0, "扫描状态", "", false),
                    item(QuickLaunchSettingsPage::Index, 1, "当前范围", "", false),
                    item(QuickLaunchSettingsPage::Index, 2, "本地文件索引", "", false),
                    item(
                        QuickLaunchSettingsPage::Index,
                        3,
                        "索引卷范围",
                        "NTFS 卷 自动 手动",
                        true,
                    ),
                    item(
                        QuickLaunchSettingsPage::Index,
                        4,
                        "排除目录",
                        "索引排除路径",
                        true,
                    ),
                ]),
            ),
            page(
                QuickLaunchSettingsPage::Context,
                SettingGroup::new().title("上下文联动").items([
                    item(
                        QuickLaunchSettingsPage::Context,
                        0,
                        "自动快速切换",
                        "从资源管理器返回已出现的打开/保存对话框时同步目录",
                        false,
                    ),
                    item(
                        QuickLaunchSettingsPage::Context,
                        1,
                        "文件对话框搜索栏",
                        "在受支持的打开/保存窗口底部显示当前目录搜索",
                        false,
                    ),
                ]),
            ),
            page(
                QuickLaunchSettingsPage::Commands,
                SettingGroup::new()
                    .variant(GroupBoxVariant::Normal)
                    .item(item(
                        QuickLaunchSettingsPage::Commands,
                        0,
                        "自定义命令",
                        "关键词 程序 参数 工作目录 管理员 静默",
                        true,
                    )),
            ),
            page(
                QuickLaunchSettingsPage::Diagnostics,
                SettingGroup::new()
                    .variant(GroupBoxVariant::Normal)
                    .item(item(
                        QuickLaunchSettingsPage::Diagnostics,
                        0,
                        "诊断",
                        "历史 排序 性能 日志",
                        true,
                    )),
            ),
        ];
        div()
            .size_full()
            .flex()
            .flex_col()
            .child(
                ModuleHeader::new(
                    "quick-launch-module-header",
                    "quick-launch-module-header-title",
                    "quick-launch-module-header-metadata",
                    "速启",
                    "应用、文件与系统工具",
                )
                .render(self.theme),
            )
            .child(
                div()
                    .debug_selector(|| "quick-launch-kit-settings".into())
                    .flex_1()
                    .min_h_0()
                    .child(
                        Settings::new("quick-launch-official-settings")
                            .sidebar_width(px(QUICK_LAUNCH_SETTINGS_NAV_WIDTH))
                            .with_group_variant(GroupBoxVariant::Fill)
                            .default_selected_index(SelectIndex {
                                page_ix: QuickLaunchSettingsPage::ALL
                                    .iter()
                                    .position(|page| *page == self.quick_launch_settings_page)
                                    .unwrap_or(0),
                                group_ix: None,
                            })
                            .pages(pages),
                    ),
            )
            .into_any_element()
    }

    fn render_quick_find_control(
        &mut self,
        selected_page: QuickLaunchSettingsPage,
        slot: usize,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.theme;
        let (
            settings,
            status,
            index_status,
            hotkey_lifecycle,
            indexer_connected,
            history_summary,
            performance,
            log_directory,
        ) = self
            .quick_launch
            .upgrade()
            .map(|application| {
                let application = application.read(cx);
                (
                    application.settings(),
                    application.catalog_status(),
                    application.index_status(),
                    application.hotkey_lifecycle(),
                    application.indexer_connected(),
                    application.history_summary(),
                    application.performance_snapshot(),
                    application.log_directory().to_path_buf(),
                )
            })
            .unwrap_or_else(|| {
                (
                    nxtools_core::QuickLaunchSettings::default(),
                    nxtools_finder::CatalogStatus::default(),
                    None,
                    QuickLaunchHotkeyLifecycle::Failed {
                        error: "速启核心不可用".into(),
                    },
                    false,
                    nxtools_finder::QuickLaunchHistorySummary::default(),
                    QuickLaunchPerformanceSnapshot::default(),
                    std::path::PathBuf::new(),
                )
            });
        let recording_quick = self.recording_hotkey == Some(HotkeyRecordingTarget::QuickLaunch);
        if recording_quick && !hotkey_lifecycle.owns_recording_session() {
            self.recording_hotkey = None;
        }
        let (hotkey_state, hotkey_value, hotkey_status) = match hotkey_lifecycle {
            QuickLaunchHotkeyLifecycle::Applied => (
                HotkeyFieldState::Applied,
                settings.hotkey.display.clone(),
                "已应用到 Windows 全局热键".to_owned(),
            ),
            QuickLaunchHotkeyLifecycle::WaitingForAdapter { status } => (
                HotkeyFieldState::Waiting,
                settings.hotkey.display.clone(),
                status,
            ),
            QuickLaunchHotkeyLifecycle::WaitingForRecording { status } => (
                HotkeyFieldState::Waiting,
                settings.hotkey.display.clone(),
                status,
            ),
            QuickLaunchHotkeyLifecycle::Recording => (
                HotkeyFieldState::Recording,
                "按下组合键".to_owned(),
                "请按下包含修饰键的组合键 · Esc 取消".to_owned(),
            ),
            QuickLaunchHotkeyLifecycle::Applying { candidate } => (
                HotkeyFieldState::Applying,
                candidate.unwrap_or_else(|| settings.hotkey.display.clone()),
                "正在向 Windows 应用全局热键".to_owned(),
            ),
            QuickLaunchHotkeyLifecycle::Failed { error } => (
                HotkeyFieldState::Failed,
                settings.hotkey.display.clone(),
                error,
            ),
        };
        let hotkey_control = hotkey_field(
            theme,
            "quick-launch-hotkey-field",
            hotkey_value,
            hotkey_status,
            hotkey_state,
            cx.listener(move |this, _, window, cx| {
                let recording = this.recording_hotkey != Some(HotkeyRecordingTarget::QuickLaunch);
                if this.recording_hotkey.take().is_some() {
                    this.defer_all_hotkey_recording(false, cx);
                }
                this.recording_hotkey = recording.then_some(HotkeyRecordingTarget::QuickLaunch);
                this.defer_all_hotkey_recording(recording, cx);
                if recording {
                    this.hotkey_focus.focus(window, cx);
                }
                cx.notify();
            }),
        );
        let auto_quick_switch = self.quick_launch.clone();
        let auto_quick_switch_enabled = settings.auto_quick_switch;
        let auto_quick_switch_control = compact_toggle(
            theme,
            "quick-launch-auto-quick-switch",
            "自动快速切换",
            auto_quick_switch_enabled,
            move |requested, _window, cx| {
                let requested = *requested;
                let application = auto_quick_switch.clone();
                cx.defer(move |cx| {
                    let _ = application.update(cx, |application, cx| {
                        application.set_auto_quick_switch(requested, cx);
                    });
                });
            },
        );
        let file_dialog_search = self.quick_launch.clone();
        let file_dialog_search_enabled = settings.file_dialog_search;
        let file_dialog_search_control = compact_toggle(
            theme,
            "quick-launch-file-dialog-search",
            "文件对话框搜索栏",
            file_dialog_search_enabled,
            move |requested, _window, cx| {
                let requested = *requested;
                let application = file_dialog_search.clone();
                cx.defer(move |cx| {
                    let _ = application.update(cx, |application, cx| {
                        application.set_file_dialog_search(requested, cx);
                    });
                });
            },
        );
        let suppress_exclusive_fullscreen = self.quick_launch.clone();
        let suppress_exclusive_fullscreen_enabled = settings.suppress_in_exclusive_fullscreen;
        let suppress_exclusive_fullscreen_control = compact_toggle(
            theme,
            "quick-launch-exclusive-fullscreen-suppression",
            "全屏游戏中暂不呼出",
            suppress_exclusive_fullscreen_enabled,
            move |requested, _window, cx| {
                let requested = *requested;
                let quick_launch = suppress_exclusive_fullscreen.clone();
                cx.defer(move |cx| {
                    let _ = quick_launch.update(cx, |application, cx| {
                        application.set_exclusive_fullscreen_suppression(requested, cx);
                    });
                });
            },
        );

        let refresh_volumes = self.quick_launch.clone();
        let refresh_volumes_control = action_button(
            "quick-launch-refresh-volumes",
            if indexer_connected {
                "刷新"
            } else {
                "安装"
            },
        )
        .ghost()
        .on_click(move |_event, _window, cx| {
            let application = refresh_volumes.clone();
            cx.defer(move |cx| {
                let _ = application.update(cx, |application, cx| {
                    application.prepare_file_index(cx);
                });
            });
        })
        .into_any_element();
        let available_volumes = index_status
            .as_ref()
            .map(|status| status.drives.clone())
            .unwrap_or_default();
        let available_mounts = available_volumes
            .iter()
            .map(|drive| drive.mount_point.clone())
            .collect::<Vec<_>>();
        let configured_volumes = settings.indexed_volumes.clone();
        let set_all_volumes = self.quick_launch.clone();
        let mut volume_scope_rows = vec![automatic_volume_scope_row(
            theme,
            configured_volumes.is_empty(),
            move |checked, _window, cx| {
                if !checked {
                    return;
                }
                let set_all_volumes = set_all_volumes.clone();
                cx.defer(move |cx| {
                    let _ = set_all_volumes.update(cx, |application, cx| {
                        application.set_indexed_volumes(Vec::new(), cx);
                    });
                });
            },
        )];
        if available_volumes.is_empty() {
            volume_scope_rows.push(empty_volume_scope_row(
                theme,
                "等待发现可建立索引的 NTFS 卷",
            ));
        }
        for (index, drive) in available_volumes.iter().enumerate() {
            let selected = volume_is_selected(&configured_volumes, &drive.mount_point);
            let label = drive.mount_point.trim_end_matches(['\\', '/']).to_owned();
            let selector = format!(
                "quick-launch-index-volume-{}",
                volume_scope_selector_key(&drive.mount_point)
            );
            let metadata = if drive.indexed_items == 0 {
                format!("{} · 尚未建立索引", drive.file_system)
            } else {
                format!(
                    "{} · {} 项",
                    drive.file_system,
                    grouped_count(drive.indexed_items)
                )
            };
            let mount_for_click = drive.mount_point.clone();
            let configured_for_click = configured_volumes.clone();
            let available_for_click = available_mounts.clone();
            let quick_launch = self.quick_launch.clone();
            volume_scope_rows.push(volume_scope_row(
                theme,
                selector,
                label,
                metadata,
                selected,
                index + 1 < available_volumes.len(),
                move |_checked, _window, cx| {
                    let selected = toggle_indexed_volume_selection(
                        &configured_for_click,
                        &available_for_click,
                        &mount_for_click,
                    );
                    if selected == configured_for_click {
                        return;
                    }
                    let quick_launch = quick_launch.clone();
                    cx.defer(move |cx| {
                        let _ = quick_launch.update(cx, |application, cx| {
                            application.set_indexed_volumes(selected, cx);
                        });
                    });
                },
            ));
        }
        let index_volumes_control = volume_scope_setting(
            theme,
            "自动模式会持续包含新卷；取消任一卷后改为仅索引勾选项",
            volume_scope_rows,
        );
        if self
            .selected_index_exclusion
            .is_some_and(|index| index >= settings.excluded_paths.len())
        {
            self.selected_index_exclusion = settings.excluded_paths.len().checked_sub(1);
        }
        let main_window = cx.entity();
        let select_main_window = main_window.clone();
        let remove_main_window = main_window.clone();
        let remove_quick_launch = self.quick_launch.clone();
        let add_quick_launch = self.quick_launch.clone();
        let exclusion_count = settings.excluded_paths.len();
        let index_exclusions_control = path_exclusion_setting(
            theme,
            &settings.excluded_paths,
            self.selected_index_exclusion,
            move |index, window, cx| {
                select_main_window.update(cx, |this, cx| {
                    this.selected_index_exclusion = Some(index);
                    this.exclusion_focus.focus(window, cx);
                    cx.notify();
                });
            },
            move |index, path, window, cx| {
                remove_main_window.update(cx, |this, cx| {
                    this.selected_index_exclusion =
                        exclusion_selection_after_removal(exclusion_count, index);
                    this.exclusion_focus.focus(window, cx);
                    cx.notify();
                });
                let quick_launch = remove_quick_launch.clone();
                cx.defer(move |cx| {
                    let _ = quick_launch.update(cx, |application, cx| {
                        application.remove_excluded_path(&path, cx);
                    });
                });
            },
            move |_event, _window, cx| {
                let quick_launch = add_quick_launch.clone();
                cx.defer(move |cx| {
                    let _ = quick_launch.update(cx, |application, cx| {
                        application.request_index_exclusion_directory(cx);
                    });
                });
            },
            cx,
        );
        if self
            .selected_custom_command
            .is_some_and(|index| index >= settings.custom_commands.len())
        {
            self.selected_custom_command = None;
            self.custom_command_draft_generation =
                self.custom_command_draft_generation.wrapping_add(1);
            self.pending_custom_command_program_picker = None;
        }
        let selected_command = self
            .selected_custom_command
            .and_then(|index| settings.custom_commands.get(index).cloned());
        let custom_commands_control = custom_command_editor(
            theme,
            CustomCommandEditorState {
                commands: &settings.custom_commands,
                selected_index: self.selected_custom_command,
                inputs: CustomCommandEditorInputs {
                    keyword: self.custom_command_keyword.clone(),
                    title: self.custom_command_title.clone(),
                    program: self.custom_command_program.clone(),
                    arguments: self.custom_command_arguments.clone(),
                    working_directory: self.custom_command_working_directory.clone(),
                },
                run_as_admin: self.custom_command_run_as_admin,
                run_silently: self.custom_command_run_silently,
            },
            CustomCommandEditorActions {
                select: Rc::new(cx.listener(
                    move |this, event: &(usize, CustomCommandSettings), window, cx| {
                        let (index, command) = event;
                        this.select_custom_command_draft(
                            *index,
                            command.clone(),
                            window.window_handle(),
                            cx,
                        );
                    },
                )),
                add: Rc::new(cx.listener(move |this, _, window, cx| {
                    let quick_launch = this.quick_launch.clone();
                    let view = cx.entity().downgrade();
                    let source_window = window.window_handle();
                    cx.defer(move |cx| {
                        let added = quick_launch
                            .update(cx, |application, cx| {
                                let index = application.add_custom_command(cx);
                                let command =
                                    application.settings().custom_commands.get(index).cloned();
                                command.map(|command| (index, command))
                            })
                            .ok()
                            .flatten();
                        if let Some((index, command)) = added {
                            let _ = view.update(cx, |this, cx| {
                                this.select_custom_command_draft(index, command, source_window, cx);
                            });
                        }
                    });
                })),
                set_enabled: Rc::new(cx.listener({
                    let selected_command = selected_command.clone();
                    move |this, enabled, _window, cx| {
                        let Some(index) = this.selected_custom_command else {
                            return;
                        };
                        let Some(mut command) = selected_command.clone() else {
                            return;
                        };
                        command.enabled = *enabled;
                        let quick_launch = this.quick_launch.clone();
                        cx.defer(move |cx| {
                            let _ = quick_launch.update(cx, |application, cx| {
                                application.update_custom_command(index, command, cx);
                            });
                        });
                    }
                })),
                delete: Rc::new(cx.listener(move |this, _, window, cx| {
                    let Some(index) = this.selected_custom_command else {
                        return;
                    };
                    let quick_launch = this.quick_launch.clone();
                    let view = cx.entity().downgrade();
                    let source_window = window.window_handle();
                    cx.defer(move |cx| {
                        let removal = quick_launch
                            .update(cx, |application, cx| {
                                if !application.remove_custom_command(index, cx) {
                                    return (false, None);
                                }
                                let commands = application.settings().custom_commands;
                                let next_index =
                                    custom_command_selection_after_removal(commands.len(), index);
                                let next = next_index.and_then(|next_index| {
                                    commands
                                        .get(next_index)
                                        .cloned()
                                        .map(|command| (next_index, command))
                                });
                                (true, next)
                            })
                            .ok();
                        let Some((true, next)) = removal else {
                            return;
                        };
                        let _ = view.update(cx, |this, cx| {
                            if let Some((next_index, command)) = next {
                                this.select_custom_command_draft(
                                    next_index,
                                    command,
                                    source_window,
                                    cx,
                                );
                            } else {
                                this.selected_custom_command = None;
                                this.custom_command_draft_generation =
                                    this.custom_command_draft_generation.wrapping_add(1);
                                this.pending_custom_command_program_picker = None;
                                cx.notify();
                            }
                        });
                    });
                })),
                pick_program: Rc::new(cx.listener(move |this, _, _window, cx| {
                    let Some(index) = this.selected_custom_command else {
                        return;
                    };
                    let generation = this.custom_command_draft_generation;
                    let current_path = this.custom_command_program.read(cx).value().to_string();
                    let quick_launch = this.quick_launch.clone();
                    let view = cx.entity().downgrade();
                    cx.defer(move |cx| {
                        let correlation_id = quick_launch
                            .update(cx, |application, cx| {
                                application.request_custom_command_program(index, current_path, cx)
                            })
                            .ok()
                            .flatten();
                        if let Some(correlation_id) = correlation_id {
                            let _ = view.update(cx, |this, _| {
                                if this.selected_custom_command == Some(index)
                                    && this.custom_command_draft_generation == generation
                                {
                                    this.pending_custom_command_program_picker =
                                        Some(PendingCustomCommandProgramPicker {
                                            correlation_id,
                                            command_index: index,
                                            draft_generation: generation,
                                        });
                                }
                            });
                        }
                    });
                })),
                set_run_as_admin: Rc::new(cx.listener(move |this, enabled, _window, cx| {
                    this.custom_command_run_as_admin = *enabled;
                    cx.notify();
                })),
                set_run_silently: Rc::new(cx.listener(move |this, enabled, _window, cx| {
                    this.custom_command_run_silently = *enabled;
                    cx.notify();
                })),
                save: Rc::new(cx.listener(move |this, _, _window, cx| {
                    this.save_selected_custom_command(cx);
                })),
            },
            cx,
        );
        // Only the official Settings active item asks for this projection.
        let history_diagnostics_control = (selected_page == QuickLaunchSettingsPage::Diagnostics)
            .then(|| {
                let log_path = if log_directory.as_os_str().is_empty() {
                    "日志目录不可用".to_owned()
                } else {
                    log_directory.display().to_string()
                };
                quick_launch_diagnostics(
                    theme,
                    QuickLaunchDiagnosticsData {
                        ranked_items: history_summary.ranked_items,
                        total_activations: history_summary.total_activations,
                        query_associations: history_summary.query_associations,
                        recent_directories: history_summary.recent_directories,
                        local_search_elapsed: format_query_elapsed(
                            performance.last_local_search_us,
                        )
                        .into(),
                        index_search_elapsed: format_query_elapsed(
                            performance.last_index_search_us,
                        )
                        .into(),
                        log_path: log_path.into(),
                        log_available: !log_directory.as_os_str().is_empty(),
                    },
                    QuickLaunchDiagnosticsActions {
                        clear_history: Rc::new(cx.listener(move |this, _, _window, cx| {
                            let quick_launch = this.quick_launch.clone();
                            cx.defer(move |cx| {
                                let _ = quick_launch.update(cx, |application, cx| {
                                    application.clear_history(cx);
                                });
                            });
                        })),
                        open_logs: Rc::new(cx.listener(move |this, _, _window, cx| {
                            let quick_launch = this.quick_launch.clone();
                            cx.defer(move |cx| {
                                let _ = quick_launch.update(cx, |application, _| {
                                    application.open_log_directory();
                                });
                            });
                        })),
                    },
                    cx,
                )
            });
        let scan_state = if status.errors.is_empty() {
            format!(
                "已索引 {} 个入口 · {} 个目录",
                status.indexed_items, status.scanned_roots
            )
        } else {
            format!(
                "已索引 {} 个入口 · {} 个目录读取异常",
                status.indexed_items,
                status.errors.len()
            )
        };
        let (file_index_state, file_index_badge) = file_index_summary(index_status.as_ref());
        let file_index_badge_color = match file_index_badge {
            "ERROR" | "PARTIAL" => theme.danger,
            "OFFLINE" | "EMPTY" => theme.text_muted,
            "UPDATING" => theme.accent_text,
            _ => theme.accent_text,
        };
        match (selected_page, slot) {
            (QuickLaunchSettingsPage::General, 0) => hotkey_control,
            (QuickLaunchSettingsPage::General, 1) => suppress_exclusive_fullscreen_control,
            (QuickLaunchSettingsPage::General, _) => div()
                .debug_selector(|| "quick-find-shortcuts".into())
                .flex()
                .flex_wrap()
                .items_center()
                .gap_2()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_1()
                        .child(crate::ui::components::shortcut::key("up"))
                        .child(crate::ui::components::shortcut::key("down"))
                        .child(gpui_kit::component::label::Label::new("选择").text_xs()),
                )
                .child(crate::ui::components::shortcut::hint("enter", "打开"))
                .child(crate::ui::components::shortcut::hint("ctrl-enter", "定位"))
                .child(crate::ui::components::shortcut::hint(
                    "escape",
                    "清空或隐藏",
                ))
                .into_any_element(),
            (QuickLaunchSettingsPage::Context, 0) => auto_quick_switch_control,
            (QuickLaunchSettingsPage::Context, _) => file_dialog_search_control,
            (QuickLaunchSettingsPage::Index, 0) => {
                div().text_sm().child(scan_state).into_any_element()
            }
            (QuickLaunchSettingsPage::Index, 1) => div()
                .text_sm()
                .child(format!(
                    "开始菜单、桌面、系统工具 · {} 个入口",
                    status.indexed_items
                ))
                .into_any_element(),
            (QuickLaunchSettingsPage::Index, 2) => div()
                .flex()
                .items_center()
                .gap_2()
                .child(
                    div()
                        .text_sm()
                        .text_color(file_index_badge_color)
                        .child(format!("{file_index_state} · {file_index_badge}")),
                )
                .child(refresh_volumes_control)
                .into_any_element(),
            (QuickLaunchSettingsPage::Index, 3) => index_volumes_control,
            (QuickLaunchSettingsPage::Index, _) => index_exclusions_control,
            (QuickLaunchSettingsPage::Commands, _) => custom_commands_control,
            (QuickLaunchSettingsPage::Diagnostics, _) => {
                history_diagnostics_control.expect("active diagnostics item")
            }
        }
    }
}

#[cfg(test)]
fn compact_setting_tooltip(
    message: gpui_kit::SharedString,
) -> impl Fn(&mut Window, &mut App) -> gpui_kit::AnyView {
    bounded_text_tooltip("compact-setting-description-tooltip", message, false)
}

#[cfg(test)]
pub(crate) fn compact_setting_row(
    theme: Theme,
    title: impl Into<gpui_kit::SharedString>,
    description: impl Into<gpui_kit::SharedString>,
    control: AnyElement,
) -> AnyElement {
    let title = title.into();
    let description = description.into();
    let description_selector = title.clone();
    div()
        .min_h(px(48.0))
        .py(px(SPACING_8))
        .border_b_1()
        .border_color(theme.border_subtle)
        .flex()
        .flex_col()
        .gap(px(SPACING_4))
        .child(
            div()
                .w_full()
                .min_w_0()
                .flex()
                .items_center()
                .gap(px(SPACING_12))
                .child(
                    div()
                        .min_w_0()
                        .flex_1()
                        .truncate()
                        .text_size(px(TEXT_BODY))
                        .line_height(px(LINE_HEIGHT_BODY))
                        .text_color(theme.text)
                        .child(title.clone()),
                )
                .child(control),
        )
        .child(
            div()
                .id(title)
                .debug_selector(move || description_selector.to_string())
                .w_full()
                .min_w_0()
                .tooltip(compact_setting_tooltip(description.clone()))
                .text_size(px(TEXT_LABEL))
                .line_height(px(LINE_HEIGHT_LABEL))
                .whitespace_normal()
                .text_color(theme.text_secondary)
                .child(description),
        )
        .into_any_element()
}

pub(crate) fn compact_toggle(
    _theme: Theme,
    id: &'static str,
    label: &'static str,
    enabled: bool,
    on_change: impl Fn(&bool, &mut Window, &mut App) + 'static,
) -> AnyElement {
    div()
        .debug_selector(move || id.into())
        .flex_none()
        .h(px(32.0))
        .flex()
        .items_center()
        .child(
            gpui_kit::component::switch::Switch::new(id)
                .checked(enabled)
                .accessibility_label(label)
                .on_change(on_change),
        )
        .into_any_element()
}

#[cfg(test)]
mod layout_tests {
    use super::*;
    use gpui_kit::{TestAppContext, WeakEntity, size};

    #[gpui_kit::test]
    fn quick_find_workbench_keeps_readable_copy_and_a_stable_control_rail(cx: &mut TestAppContext) {
        cx.update(gpui_kit::init);
        let (_main, window) = cx.add_window_view(|window, cx| {
            window.resize(size(px(920.0), px(480.0)));
            let notifications = cx.new(|_| NotificationCenter::default());
            let mut main = MainWindow::new(
                WeakEntity::new_invalid(),
                WeakEntity::new_invalid(),
                WeakEntity::new_invalid(),
                MainWindowShellContext {
                    settings: None,
                    platform: None,
                    system_settings: Rc::new(RefCell::new(SystemSettings::default())),
                },
                notifications,
                window,
                cx,
            );
            main.shell.activate(MainModule::QuickFind);
            main
        });
        window.simulate_resize(size(px(920.0), px(480.0)));
        window
            .refresh()
            .expect("render production Quick Find at minimum size");
        let host = window
            .debug_bounds("quick-launch-kit-settings")
            .expect("official settings viewport");
        let hotkey = window
            .debug_bounds("quick-launch-hotkey-field")
            .expect("original recorder");
        let toggle = window
            .debug_bounds("quick-launch-exclusive-fullscreen-suppression")
            .expect("real Switch");
        for bounds in [hotkey, toggle] {
            assert!(bounds.left() >= host.left() && bounds.right() <= host.right());
            assert!(bounds.top() >= host.top() && bounds.bottom() <= host.bottom());
        }
        assert_eq!(toggle.size.height, px(32.0));
        let help = window
            .debug_bounds("quick-settings-全局呼出热键-help")
            .expect("Kit help");
        assert!(help.size.width <= px(24.0));
        let inputs = window.update(|window, _| {
            gpui_kit::base::test_support::snapshots(window)
                .into_iter()
                .filter(|snapshot| snapshot.role() == Some(gpui_kit::Role::TextInput))
                .count()
        });
        assert_eq!(inputs, 1, "real Settings search at minimum window size");
    }
}

#[cfg(test)]
mod component_adoption_tests {
    use super::*;
    struct ToggleFixture {
        checked: bool,
        requests: Rc<RefCell<Vec<bool>>>,
    }
    impl Render for ToggleFixture {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let owner = cx.entity().downgrade();
            let requests = self.requests.clone();
            div()
                .flex()
                .gap(px(16.0))
                .child(compact_toggle(
                    Theme::zed_dark(),
                    "adopted-quick-toggle",
                    "测试开关",
                    self.checked,
                    move |requested, _, cx| {
                        let requested = *requested;
                        requests.borrow_mut().push(requested);
                        let owner = owner.clone();
                        cx.defer(move |cx| {
                            let _ = owner.update(cx, |view, cx| {
                                view.checked = requested;
                                cx.notify();
                            });
                        });
                    },
                ))
                .child(
                    div()
                        .debug_selector(|| "reference-quick-switch".into())
                        .child(gpui_kit::component::switch::Switch::new(
                            "reference-quick-switch",
                        )),
                )
        }
    }
    #[gpui_kit::test]
    fn quick_settings_toggle_matches_real_released_switch(cx: &mut gpui_kit::TestAppContext) {
        cx.update(gpui_kit::init);
        cx.update(crate::theme::apply_component_theme);
        let requests = Rc::new(RefCell::new(Vec::new()));
        let observed = requests.clone();
        let (_, window) = cx.add_window_view(move |window, cx| {
            let view = cx.new(|_| ToggleFixture {
                checked: false,
                requests: observed,
            });
            gpui_kit::base::Root::new(view, window, cx)
        });
        window.refresh().expect("actual toggle and direct Switch");
        let bounds = window
            .debug_bounds("adopted-quick-toggle")
            .expect("production toggle");
        assert_eq!(
            bounds.size,
            window
                .debug_bounds("reference-quick-switch")
                .expect("released Switch")
                .size,
            "settings enable policy must be a Switch, not a styled Button"
        );
        let snapshot = window.update(|window, _| {
            gpui_kit::base::test_support::snapshots(window)
                .into_iter()
                .find(|snapshot| snapshot.label() == Some("测试开关"))
                .expect("actual semantic Switch snapshot")
        });
        assert_eq!(snapshot.role(), Some(gpui_kit::Role::Switch));
        assert_eq!(snapshot.checked(), Some(false));
        window.simulate_click(bounds.center(), gpui_kit::Modifiers::default());
        window.run_until_parked();
        window.refresh().expect("controlled checked state");
        assert_eq!(
            &*requests.borrow(),
            &[true],
            "one pointer gesture requests true once"
        );
        assert_eq!(
            window.update(|window, _| gpui_kit::base::test_support::snapshots(window)
                .into_iter()
                .find(|snapshot| snapshot.label() == Some("测试开关"))
                .expect("checked semantic Switch")
                .checked()),
            Some(true)
        );
        window.update(|window, cx| assert!(window.focused(cx).is_some(), "Kit owns pointer focus"));
        let keystroke = gpui_kit::Keystroke::parse("space").expect("Space toggle");
        window.simulate_event(gpui_kit::KeyDownEvent {
            keystroke: keystroke.clone(),
            is_held: false,
            prefer_character_input: false,
        });
        assert_eq!(&*requests.borrow(), &[true], "activation waits for release");
        window.simulate_event(gpui_kit::KeyUpEvent { keystroke });
        window.run_until_parked();
        window.refresh().expect("controlled unchecked state");
        assert_eq!(
            &*requests.borrow(),
            &[true, false],
            "one Space release uses the rendered controlled value"
        );
        assert_eq!(
            window.update(|window, _| gpui_kit::base::test_support::snapshots(window)
                .into_iter()
                .find(|snapshot| snapshot.label() == Some("测试开关"))
                .expect("unchecked semantic Switch")
                .checked()),
            Some(false)
        );
        assert_eq!(
            window
                .debug_bounds("adopted-quick-toggle")
                .expect("unchanged geometry"),
            bounds
        );
    }
}

#[cfg(test)]
mod settings_compatibility_tests;
