use super::*;
use crate::ui::components::tooltip::bounded_text_tooltip;
use gpui_component::Selectable;

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
    pub(crate) navigation_id: &'static str,
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
                navigation_id: "quick-launch-settings-nav-general",
                content_id: "quick-launch-settings-page-general",
            },
            Self::Index => QuickLaunchSettingsPageMetadata {
                label: "索引",
                description: "应用目录、文件索引与排除范围",
                navigation_id: "quick-launch-settings-nav-index",
                content_id: "quick-launch-settings-page-index",
            },
            Self::Context => QuickLaunchSettingsPageMetadata {
                label: "上下文",
                description: "资源管理器与文件对话框联动",
                navigation_id: "quick-launch-settings-nav-context",
                content_id: "quick-launch-settings-page-context",
            },
            Self::Commands => QuickLaunchSettingsPageMetadata {
                label: "命令",
                description: "维护关键词驱动的自定义动作",
                navigation_id: "quick-launch-settings-nav-commands",
                content_id: "quick-launch-settings-page-commands",
            },
            Self::Diagnostics => QuickLaunchSettingsPageMetadata {
                label: "诊断",
                description: "历史学习、查询性能与运行日志",
                navigation_id: "quick-launch-settings-nav-diagnostics",
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
    pub(crate) fn smooth_scroll_quick_find(&mut self, delta: f32, cx: &mut Context<Self>) -> bool {
        let maximum = self.quick_find_scroll.max_offset().height.as_f32().max(0.0);
        if maximum <= 0.0 || delta == 0.0 {
            return false;
        }
        let current = self.quick_find_scroll.offset().y.as_f32();
        let target =
            quick_find_scroll_target(current, self.quick_find_scroll_target, maximum, delta);
        self.animate_quick_find_scroll_to(target, cx);
        true
    }

    pub(crate) fn animate_quick_find_scroll_to(&mut self, target: f32, cx: &mut Context<Self>) {
        self.quick_find_scroll_target = target;
        self.quick_find_scroll_animation = self.quick_find_scroll_animation.wrapping_add(1).max(1);
        if crate::ui::motion::reduced_motion(cx) {
            self.quick_find_scroll
                .set_offset(point(px(0.0), px(target)));
            cx.notify();
            return;
        }
        let generation = self.quick_find_scroll_animation;
        let scroll = self.quick_find_scroll.clone();
        let background = cx.background_executor().clone();
        cx.spawn(async move |this, cx| {
            for _ in 0..12 {
                background.timer(QUICK_FIND_SCROLL_FRAME_DURATION).await;
                let done = this
                    .update(cx, |this, cx| {
                        if this.quick_find_scroll_animation != generation {
                            return true;
                        }
                        if crate::ui::motion::reduced_motion(cx) {
                            scroll.set_offset(point(px(0.0), px(this.quick_find_scroll_target)));
                            cx.notify();
                            return true;
                        }
                        let current = scroll.offset().y.as_f32();
                        let (next, done) =
                            quick_find_scroll_step(current, this.quick_find_scroll_target);
                        scroll.set_offset(point(px(0.0), px(next)));
                        cx.notify();
                        done
                    })
                    .unwrap_or(true);
                if done {
                    break;
                }
            }
            let _ = this.update(cx, |this, cx| {
                if this.quick_find_scroll_animation == generation {
                    scroll.set_offset(point(px(0.0), px(this.quick_find_scroll_target)));
                    cx.notify();
                }
            });
        })
        .detach();
    }

    pub(crate) fn select_quick_launch_settings_page(
        &mut self,
        page: QuickLaunchSettingsPage,
        cx: &mut Context<Self>,
    ) {
        if self.quick_launch_settings_page == page {
            return;
        }
        self.quick_launch_settings_page = page;
        self.selected_index_exclusion = None;
        self.quick_find_scroll_animation = self.quick_find_scroll_animation.wrapping_add(1).max(1);
        self.quick_find_scroll_target = 0.0;
        self.quick_find_scroll.set_offset(point(px(0.0), px(0.0)));
        cx.notify();
    }

    pub(crate) fn handle_index_exclusion_keyboard(
        &mut self,
        event: &KeyDownEvent,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.shell.active_module() != MainModule::QuickFind
            || self.quick_launch_settings_page != QuickLaunchSettingsPage::Index
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
                    this.hotkey_focus.focus(window);
                }
                cx.notify();
            }),
        );
        let auto_quick_switch = self.quick_launch.clone();
        let auto_quick_switch_enabled = settings.auto_quick_switch;
        let auto_quick_switch_control = compact_toggle(
            theme,
            "quick-launch-auto-quick-switch",
            auto_quick_switch_enabled,
            move |_event, _window, cx| {
                let application = auto_quick_switch.clone();
                cx.defer(move |cx| {
                    let _ = application.update(cx, |application, cx| {
                        application.set_auto_quick_switch(!auto_quick_switch_enabled, cx);
                    });
                });
            },
        );
        let file_dialog_search = self.quick_launch.clone();
        let file_dialog_search_enabled = settings.file_dialog_search;
        let file_dialog_search_control = compact_toggle(
            theme,
            "quick-launch-file-dialog-search",
            file_dialog_search_enabled,
            move |_event, _window, cx| {
                let application = file_dialog_search.clone();
                cx.defer(move |cx| {
                    let _ = application.update(cx, |application, cx| {
                        application.set_file_dialog_search(!file_dialog_search_enabled, cx);
                    });
                });
            },
        );
        let suppress_exclusive_fullscreen = self.quick_launch.clone();
        let suppress_exclusive_fullscreen_enabled = settings.suppress_in_exclusive_fullscreen;
        let suppress_exclusive_fullscreen_control = compact_toggle(
            theme,
            "quick-launch-exclusive-fullscreen-suppression",
            suppress_exclusive_fullscreen_enabled,
            move |_event, _window, cx| {
                let quick_launch = suppress_exclusive_fullscreen.clone();
                cx.defer(move |cx| {
                    let _ = quick_launch.update(cx, |application, cx| {
                        application.set_exclusive_fullscreen_suppression(
                            !suppress_exclusive_fullscreen_enabled,
                            cx,
                        );
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
        .border_1()
        .border_color(theme.border_subtle)
        .bg(theme.surface_raised)
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
                    this.hotkey_focus.focus(window);
                    cx.notify();
                });
            },
            move |index, path, window, cx| {
                remove_main_window.update(cx, |this, cx| {
                    this.selected_index_exclusion =
                        exclusion_selection_after_removal(exclusion_count, index);
                    this.hotkey_focus.focus(window);
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
        let selected_page = self.quick_launch_settings_page;
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
        let general_page = quick_launch_settings_group_page(
            theme,
            QuickLaunchSettingsPage::General.metadata().content_id,
            compact_group(
                theme,
                "呼出与执行",
                vec![
                    hotkey_setting_row(
                        theme,
                        "quick-launch-hotkey-setting",
                        "全局呼出热键",
                        "点击字段开始录制；Esc 或失焦取消",
                        hotkey_control,
                    ),
                    compact_setting_row(
                        theme,
                        "全屏游戏中暂不呼出",
                        "仅 Direct3D 独占全屏；无边框全屏仍可呼出速启",
                        suppress_exclusive_fullscreen_control,
                    ),
                    compact_setting_row(
                        theme,
                        "键盘动作",
                        "↑↓ 选择；Enter 打开；Ctrl+Enter 定位；Esc 清空或隐藏",
                        div()
                            .text_size(px(TEXT_META))
                            .text_color(theme.text_muted)
                            .child("FIXED")
                            .into_any_element(),
                    ),
                ],
            ),
        );
        let context_page = quick_launch_settings_group_page(
            theme,
            QuickLaunchSettingsPage::Context.metadata().content_id,
            compact_group(
                theme,
                "上下文联动",
                vec![
                    compact_setting_row(
                        theme,
                        "自动快速切换",
                        "从资源管理器返回已出现的打开/保存对话框时同步目录",
                        auto_quick_switch_control,
                    ),
                    compact_setting_row(
                        theme,
                        "文件对话框搜索栏",
                        "在受支持的打开/保存窗口底部显示当前目录搜索",
                        file_dialog_search_control,
                    ),
                ],
            ),
        );
        let index_page = quick_launch_settings_group_page(
            theme,
            QuickLaunchSettingsPage::Index.metadata().content_id,
            compact_group(
                theme,
                "应用目录",
                vec![
                    compact_setting_row(
                        theme,
                        "扫描状态",
                        scan_state,
                        div()
                            .text_size(px(TEXT_META))
                            .text_color(if status.errors.is_empty() {
                                theme.success
                            } else {
                                theme.danger
                            })
                            .child(if status.errors.is_empty() {
                                "READY"
                            } else {
                                "PARTIAL"
                            })
                            .into_any_element(),
                    ),
                    compact_setting_row(
                        theme,
                        "当前范围",
                        "开始菜单、桌面、系统工具与 NTFS 文件索引",
                        div()
                            .text_size(px(TEXT_META))
                            .text_color(theme.text_muted)
                            .child(format!("{} ITEMS", status.indexed_items))
                            .into_any_element(),
                    ),
                    compact_setting_row(
                        theme,
                        "本地文件索引",
                        file_index_state,
                        div()
                            .flex()
                            .items_center()
                            .gap(px(SPACING_8))
                            .child(
                                div()
                                    .text_size(px(TEXT_META))
                                    .text_color(file_index_badge_color)
                                    .child(file_index_badge),
                            )
                            .child(refresh_volumes_control)
                            .into_any_element(),
                    ),
                    index_volumes_control,
                    index_exclusions_control,
                ],
            ),
        );
        let commands_page = div()
            .id(QuickLaunchSettingsPage::Commands.metadata().content_id)
            .debug_selector(|| {
                QuickLaunchSettingsPage::Commands
                    .metadata()
                    .content_id
                    .into()
            })
            .w_full()
            .child(custom_commands_control)
            .into_any_element();
        let selected_page_content =
            match selected_page {
                QuickLaunchSettingsPage::General => general_page,
                QuickLaunchSettingsPage::Index => index_page,
                QuickLaunchSettingsPage::Context => context_page,
                QuickLaunchSettingsPage::Commands => commands_page,
                QuickLaunchSettingsPage::Diagnostics => div()
                    .id(QuickLaunchSettingsPage::Diagnostics.metadata().content_id)
                    .debug_selector(|| {
                        QuickLaunchSettingsPage::Diagnostics
                            .metadata()
                            .content_id
                            .into()
                    })
                    .w_full()
                    .child(history_diagnostics_control.expect(
                        "Diagnostics content exists only for the selected Diagnostics page",
                    ))
                    .into_any_element(),
            };
        let mut settings_navigation = div()
            .id("quick-launch-settings-navigation")
            .debug_selector(|| "quick-launch-settings-navigation".into())
            .w(px(QUICK_LAUNCH_SETTINGS_NAV_WIDTH))
            .flex_none()
            .border_r_1()
            .border_color(theme.border_subtle)
            .bg(theme.canvas)
            .pt(px(SPACING_8))
            .flex()
            .flex_col();
        for page in QuickLaunchSettingsPage::ALL {
            let page_metadata = page.metadata();
            let selected = page == selected_page;
            settings_navigation = settings_navigation.child(
                div()
                    .id(page_metadata.navigation_id)
                    .debug_selector(move || page_metadata.navigation_id.into())
                    .h(px(QUICK_LAUNCH_SETTINGS_NAV_ROW_HEIGHT))
                    .w_full()
                    .child(
                        Button::new(page_metadata.navigation_id)
                            .ghost()
                            .selected(selected)
                            .w_full()
                            .h_full()
                            .rounded(px(RADIUS_CONTROL))
                            .px(px(SPACING_12))
                            .text_color(if selected {
                                theme.text
                            } else {
                                theme.text_secondary
                            })
                            .when(selected, |button| {
                                button
                                    .bg(theme.element_selected)
                                    .border_l_2()
                                    .border_color(theme.accent)
                            })
                            .label(page_metadata.label)
                            .on_click(cx.listener(move |this, _event, _window, cx| {
                                this.select_quick_launch_settings_page(page, cx);
                            })),
                    ),
            );
        }
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
                .render(theme),
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .child(settings_navigation)
                    .child(
                        div()
                            .min_w_0()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .id("quick-launch-settings-page-header")
                                    .debug_selector(|| "quick-launch-settings-page-header".into())
                                    .h(px(QUICK_LAUNCH_SETTINGS_PAGE_HEADER_HEIGHT))
                                    .flex_none()
                                    .px(px(SPACING_16))
                                    .border_b_1()
                                    .border_color(theme.border_subtle)
                                    .flex()
                                    .items_center()
                                    .gap(px(SPACING_12))
                                    .child(
                                        div()
                                            .debug_selector(|| {
                                                "quick-launch-settings-page-title".into()
                                            })
                                            .text_size(px(TEXT_TITLE))
                                            .line_height(px(LINE_HEIGHT_TITLE))
                                            .text_color(theme.text)
                                            .child(selected_page.metadata().label),
                                    )
                                    .child(
                                        div()
                                            .debug_selector(|| {
                                                "quick-launch-settings-page-description".into()
                                            })
                                            .min_w_0()
                                            .truncate()
                                            .text_size(px(TEXT_LABEL))
                                            .line_height(px(LINE_HEIGHT_LABEL))
                                            .text_color(theme.text_muted)
                                            .child(selected_page.metadata().description),
                                    ),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .min_h_0()
                                    .relative()
                                    .child(
                                        div()
                                            .id("quick-launch-settings-scroll")
                                            .debug_selector(|| {
                                                "quick-launch-settings-scroll".into()
                                            })
                                            .size_full()
                                            .track_scroll(&self.quick_find_scroll)
                                            .overflow_y_scroll()
                                            .on_scroll_wheel(cx.listener(
                                                |this, event: &ScrollWheelEvent, _window, cx| {
                                                    let delta = event.delta.pixel_delta(px(
                                                        QUICK_FIND_SCROLL_LINE_HEIGHT,
                                                    ));
                                                    let delta = if delta.y.as_f32() != 0.0 {
                                                        delta.y.as_f32()
                                                    } else {
                                                        delta.x.as_f32()
                                                    };
                                                    if this.smooth_scroll_quick_find(delta, cx) {
                                                        cx.stop_propagation();
                                                    }
                                                },
                                            ))
                                            .p(px(SPACING_16))
                                            .child(selected_page_content),
                                    )
                                    .child(
                                        Scrollbar::vertical(&self.quick_find_scroll)
                                            .id("quick-launch-settings-scrollbar"),
                                    ),
                            ),
                    ),
            )
            .into_any_element()
    }
}

fn compact_setting_tooltip(
    message: gpui::SharedString,
) -> impl Fn(&mut Window, &mut App) -> gpui::AnyView {
    bounded_text_tooltip("compact-setting-description-tooltip", message, false)
}

pub(crate) fn compact_setting_row(
    theme: Theme,
    title: impl Into<gpui::SharedString>,
    description: impl Into<gpui::SharedString>,
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
    theme: Theme,
    id: &'static str,
    enabled: bool,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
) -> AnyElement {
    Button::new(id)
        .debug_selector(move || id.into())
        .small()
        .h(px(BADGE_HEIGHT))
        .px(px(SPACING_8))
        .rounded(px(RADIUS_CONTROL))
        .border_1()
        .border_color(if enabled {
            theme.accent
        } else {
            theme.border_subtle
        })
        .bg(if enabled {
            theme.accent_soft
        } else {
            theme.canvas
        })
        .text_color(if enabled {
            theme.accent_text
        } else {
            theme.text_secondary
        })
        .selected(enabled)
        .label(if enabled { "✓ 开启" } else { "关闭" })
        .on_click(on_click)
        .into_any_element()
}

pub(crate) fn quick_launch_settings_group_page(
    theme: Theme,
    selector: &'static str,
    group: AnyElement,
) -> AnyElement {
    div()
        .id(selector)
        .debug_selector(move || selector.into())
        .w_full()
        .bg(theme.surface)
        .px(px(SPACING_12))
        .child(group)
        .into_any_element()
}

pub(crate) fn compact_group(
    theme: Theme,
    title: &'static str,
    rows: Vec<AnyElement>,
) -> AnyElement {
    div()
        .pb(px(SPACING_4))
        .child(
            div()
                .h(px(GROUP_HEADER_HEIGHT))
                .border_b_1()
                .border_color(theme.border_subtle)
                .flex()
                .items_center()
                .text_size(px(TEXT_LABEL))
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(theme.text_secondary)
                .child(title),
        )
        .children(rows)
        .into_any_element()
}
