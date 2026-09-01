use super::*;

pub(crate) fn custom_command_selection_after_removal(
    remaining_item_count: usize,
    removed_index: usize,
) -> Option<usize> {
    (remaining_item_count > 0).then(|| removed_index.min(remaining_item_count - 1))
}

#[derive(Clone, Copy)]
pub(crate) struct PendingCustomCommandProgramPicker {
    pub(crate) correlation_id: uuid::Uuid,
    pub(crate) command_index: usize,
    pub(crate) draft_generation: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HotkeyRecordingTarget {
    Clipboard,
    QuickLaunch,
}

#[derive(Default)]
pub(crate) struct ClipboardOverview {
    pub(crate) total: usize,
    pub(crate) text: usize,
    pub(crate) images: usize,
    pub(crate) files: usize,
}

impl MainWindow {
    pub(crate) fn clipboard_overview(&self, cx: &App) -> ClipboardOverview {
        let Ok(application) = self.clipboard.upgrade().ok_or(()) else {
            return ClipboardOverview::default();
        };
        let Ok(ClipboardHistoryCounts {
            total,
            text,
            images,
            files,
        }) = application.read(cx).clipboard_counts()
        else {
            return ClipboardOverview::default();
        };
        ClipboardOverview {
            total,
            text,
            images,
            files,
        }
    }

    pub(crate) fn clipboard_settings(&self, cx: &App) -> ClipboardSettings {
        self.clipboard
            .upgrade()
            .map(|application| application.read(cx).clipboard_settings())
            .unwrap_or_default()
    }

    pub(crate) fn select_custom_command_draft(
        &mut self,
        index: usize,
        command: CustomCommandSettings,
        source_window: AnyWindowHandle,
        cx: &mut Context<Self>,
    ) {
        self.selected_custom_command = Some(index);
        self.custom_command_draft_generation = self.custom_command_draft_generation.wrapping_add(1);
        self.pending_custom_command_program_picker = None;
        self.custom_command_run_as_admin = command.run_as_admin;
        self.custom_command_run_silently = command.run_silently;
        let fields = [
            (&self.custom_command_keyword, command.keyword),
            (&self.custom_command_title, command.title),
            (&self.custom_command_program, command.program),
            (&self.custom_command_arguments, command.argument_template),
            (
                &self.custom_command_working_directory,
                command.working_directory,
            ),
        ]
        .map(|(input, value)| (input.clone(), value));
        cx.defer(move |cx| {
            let _ = source_window.update(cx, |_, window, cx| {
                for (input, value) in fields {
                    input.update(cx, |state, cx| state.set_value(value, window, cx));
                }
            });
        });
        cx.notify();
    }

    pub(crate) fn save_selected_custom_command(&mut self, cx: &mut Context<Self>) {
        let Some(index) = self.selected_custom_command else {
            return;
        };
        let enabled = self
            .quick_launch
            .upgrade()
            .and_then(|application| {
                application
                    .read(cx)
                    .settings()
                    .custom_commands
                    .get(index)
                    .map(|command| command.enabled)
            })
            .unwrap_or(false);
        let command = CustomCommandSettings {
            enabled,
            keyword: self.custom_command_keyword.read(cx).value().to_string(),
            title: self.custom_command_title.read(cx).value().to_string(),
            program: self.custom_command_program.read(cx).value().to_string(),
            argument_template: self.custom_command_arguments.read(cx).value().to_string(),
            working_directory: self
                .custom_command_working_directory
                .read(cx)
                .value()
                .to_string(),
            run_as_admin: self.custom_command_run_as_admin,
            run_silently: self.custom_command_run_silently,
        };
        let quick_launch = self.quick_launch.clone();
        cx.defer(move |cx| {
            let _ = quick_launch.update(cx, |application, cx| {
                application.update_custom_command(index, command, cx);
            });
        });
        cx.notify();
    }
}

impl MainWindow {
    pub(crate) fn render_modifier_action_selector(
        &self,
        modifier: ClipboardClickModifier,
        current: ClipboardClickAction,
    ) -> AnyElement {
        let theme = self.theme;
        let clipboard = self.clipboard.clone();
        Button::new(match modifier {
            ClipboardClickModifier::Control => "clipboard-modifier-control",
            ClipboardClickModifier::Alt => "clipboard-modifier-alt",
            ClipboardClickModifier::Shift => "clipboard-modifier-shift",
        })
        .ghost()
        .compact()
        .xsmall()
        .h(px(26.0))
        .w(px(154.0))
        .px(px(8.0))
        .rounded(px(RADIUS_CONTROL))
        .border_1()
        .border_color(theme.border_subtle)
        .bg(theme.surface_raised)
        .child(
            div()
                .text_size(px(TEXT_LABEL))
                .child(click_action_label(current)),
        )
        .text_color(theme.text_secondary)
        .dropdown_menu_with_anchor(Corner::TopRight, move |menu, _window, _cx| {
            ClipboardClickAction::ALL.into_iter().fold(
                menu.min_w(px(154.0)).max_w(px(154.0)),
                |menu, action| {
                    let clipboard = clipboard.clone();
                    let label = click_action_label(action);
                    menu.item(
                        PopupMenuItem::element(move |_window, _cx| {
                            div().text_size(px(TEXT_LABEL)).child(label)
                        })
                        .checked(action == current)
                        .on_click(move |_event, _window, cx| {
                            let _ = clipboard.update(cx, |application, _| {
                                application.set_clipboard_modifier_action(modifier, action);
                            });
                        }),
                    )
                },
            )
        })
        .into_any_element()
    }

    pub(crate) fn render_retention_stepper(
        &mut self,
        id: &'static str,
        value: usize,
        step: usize,
        edits_days: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.theme;
        div()
            .id(id)
            .h(px(26.0))
            .rounded(px(RADIUS_CONTROL))
            .border_1()
            .border_color(theme.border)
            .bg(theme.canvas)
            .flex()
            .items_center()
            .child(
                div()
                    .id(gpui::SharedString::from(format!("{id}-minus")))
                    .w(px(28.0))
                    .h_full()
                    .cursor_pointer()
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_color(theme.text_muted)
                    .hover(move |style| style.bg(theme.surface_hover).text_color(theme.text))
                    .child("−")
                    .on_click(cx.listener(move |this, _event, _window, cx| {
                        cx.stop_propagation();
                        let _ = this.clipboard.update(cx, |application, _| {
                            let settings = application.clipboard_settings();
                            if edits_days {
                                application.set_clipboard_retention(
                                    settings.max_items,
                                    settings.retention_days.saturating_sub(step as u32),
                                );
                            } else {
                                application.set_clipboard_retention(
                                    settings.max_items.saturating_sub(step),
                                    settings.retention_days,
                                );
                            }
                        });
                        cx.notify();
                    })),
            )
            .child(
                div()
                    .w(px(72.0))
                    .h_full()
                    .border_l_1()
                    .border_r_1()
                    .border_color(theme.border_subtle)
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(px(TEXT_LABEL))
                    .font_family(UI_MONO_FONT_FAMILY)
                    .child(value.to_string()),
            )
            .child(
                div()
                    .id(gpui::SharedString::from(format!("{id}-plus")))
                    .w(px(28.0))
                    .h_full()
                    .cursor_pointer()
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_color(theme.text_muted)
                    .hover(move |style| style.bg(theme.surface_hover).text_color(theme.text))
                    .child("+")
                    .on_click(cx.listener(move |this, _event, _window, cx| {
                        cx.stop_propagation();
                        let _ = this.clipboard.update(cx, |application, _| {
                            let settings = application.clipboard_settings();
                            if edits_days {
                                application.set_clipboard_retention(
                                    settings.max_items,
                                    settings.retention_days.saturating_add(step as u32),
                                );
                            } else {
                                application.set_clipboard_retention(
                                    settings.max_items.saturating_add(step),
                                    settings.retention_days,
                                );
                            }
                        });
                        cx.notify();
                    })),
            )
            .into_any_element()
    }

    pub(crate) fn defer_hotkey_recording(
        &self,
        target: HotkeyRecordingTarget,
        recording: bool,
        cx: &mut Context<Self>,
    ) {
        let clipboard = self.clipboard.clone();
        let quick_launch = self.quick_launch.clone();
        cx.defer(move |cx| match target {
            HotkeyRecordingTarget::Clipboard => {
                let _ = clipboard.update(cx, |application, _| {
                    application.set_clipboard_hotkey_recording(recording);
                });
            }
            HotkeyRecordingTarget::QuickLaunch => {
                let _ = quick_launch.update(cx, |application, _| {
                    application.set_hotkey_recording(recording);
                });
            }
        });
    }

    pub(crate) fn defer_recorded_hotkey(
        &self,
        target: HotkeyRecordingTarget,
        hotkey: ClipboardHotKey,
        cx: &mut Context<Self>,
    ) {
        let clipboard = self.clipboard.clone();
        let quick_launch = self.quick_launch.clone();
        cx.defer(move |cx| match target {
            HotkeyRecordingTarget::Clipboard => {
                let _ = clipboard.update(cx, |application, _| {
                    application.set_clipboard_hotkey(hotkey);
                });
            }
            HotkeyRecordingTarget::QuickLaunch => {
                let _ = quick_launch.update(cx, |application, _| {
                    application.set_hotkey(hotkey);
                });
            }
        });
    }

    pub(crate) fn defer_all_hotkey_recording(&self, recording: bool, cx: &mut Context<Self>) {
        let clipboard = self.clipboard.clone();
        let quick_launch = self.quick_launch.clone();
        cx.defer(move |cx| {
            let _ = clipboard.update(cx, |application, _| {
                application.set_clipboard_hotkey_recording(recording);
            });
            let _ = quick_launch.update(cx, |application, _| {
                application.set_hotkey_recording(recording);
            });
        });
    }

    pub(crate) fn handle_hotkey_recording(&mut self, event: &KeyDownEvent, cx: &mut Context<Self>) {
        let Some(target) = self.recording_hotkey else {
            return;
        };
        if event.keystroke.key.eq_ignore_ascii_case("escape") {
            self.recording_hotkey = None;
            self.defer_all_hotkey_recording(false, cx);
            cx.stop_propagation();
            cx.notify();
            return;
        }
        let clipboard_ready = self
            .clipboard
            .upgrade()
            .is_none_or(|application| application.read(cx).clipboard_hotkey_recording_ready());
        let quick_launch_ready = self
            .quick_launch
            .upgrade()
            .is_none_or(|application| application.read(cx).hotkey_recording_ready());
        let recording_ready = clipboard_ready && quick_launch_ready;
        if !recording_ready {
            cx.stop_propagation();
            return;
        }
        let Some(hotkey) = hotkey_from_key_down(event) else {
            return;
        };
        self.defer_recorded_hotkey(target, hotkey, cx);
        let other = match target {
            HotkeyRecordingTarget::Clipboard => HotkeyRecordingTarget::QuickLaunch,
            HotkeyRecordingTarget::QuickLaunch => HotkeyRecordingTarget::Clipboard,
        };
        self.defer_hotkey_recording(other, false, cx);
        self.recording_hotkey = None;
        cx.stop_propagation();
        cx.notify();
    }
}

impl MainWindow {
    pub(crate) fn render_clipboard(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme;
        let overview = self.clipboard_overview(cx);
        let settings = self.clipboard_settings(cx);
        let configuration_status = self
            .clipboard
            .upgrade()
            .map(|application| {
                application
                    .read(cx)
                    .clipboard_configuration_status()
                    .to_owned()
            })
            .unwrap_or_else(|| "不可用".into());
        let management_busy = self
            .clipboard
            .upgrade()
            .is_some_and(|application| application.read(cx).clipboard_management_busy());
        let storage_directory = self
            .clipboard
            .upgrade()
            .map(|application| application.read(cx).clipboard_storage_directory())
            .unwrap_or_else(|| "不可用".into());
        let hotkey_update_pending = self
            .clipboard
            .upgrade()
            .is_some_and(|application| application.read(cx).clipboard_hotkey_update_pending());
        let recording_ready = self
            .clipboard
            .upgrade()
            .is_some_and(|application| application.read(cx).clipboard_hotkey_recording_ready());
        let recording_active = self
            .clipboard
            .upgrade()
            .is_some_and(|application| application.read(cx).clipboard_hotkey_recording_active());
        let recording_clipboard = self.recording_hotkey == Some(HotkeyRecordingTarget::Clipboard);
        if recording_clipboard && !recording_active {
            self.recording_hotkey = None;
        }
        let hotkey_label = if hotkey_update_pending {
            "正在应用…".to_owned()
        } else if recording_clipboard {
            if recording_ready {
                "按下组合键…".to_owned()
            } else {
                "正在暂停…".to_owned()
            }
        } else {
            settings.hotkey.display.clone()
        };
        let hotkey_control = div()
            .id("record-clipboard-hotkey")
            .h(px(26.0))
            .min_w(px(116.0))
            .px(px(9.0))
            .rounded(px(RADIUS_CONTROL))
            .border_1()
            .border_color(if recording_clipboard {
                theme.accent
            } else {
                theme.border
            })
            .bg(if recording_clipboard {
                theme.accent_soft
            } else {
                theme.canvas
            })
            .cursor_pointer()
            .flex()
            .items_center()
            .justify_center()
            .font_family(UI_MONO_FONT_FAMILY)
            .text_size(px(TEXT_META))
            .text_color(if recording_clipboard {
                theme.accent_text
            } else {
                theme.text_secondary
            })
            .child(hotkey_label)
            .on_click(cx.listener(move |this, _, window, cx| {
                if !hotkey_update_pending {
                    let recording = this.recording_hotkey != Some(HotkeyRecordingTarget::Clipboard);
                    if this.recording_hotkey.take().is_some() {
                        this.defer_all_hotkey_recording(false, cx);
                    }
                    this.recording_hotkey = recording.then_some(HotkeyRecordingTarget::Clipboard);
                    this.defer_all_hotkey_recording(recording, cx);
                    if recording {
                        this.hotkey_focus.focus(window);
                    }
                    cx.notify();
                }
            }))
            .into_any_element();
        let max_items_control = self.render_retention_stepper(
            "clipboard-max-items",
            settings.max_items,
            100,
            false,
            cx,
        );
        let retention_control = self.render_retention_stepper(
            "clipboard-retention-days",
            settings.retention_days as usize,
            1,
            true,
            cx,
        );
        let exclusions_control = div()
            .w(px(250.0))
            .h(px(28.0))
            .rounded(px(RADIUS_CONTROL))
            .border_1()
            .border_color(theme.border)
            .bg(theme.canvas)
            .px(px(7.0))
            .flex()
            .items_center()
            .child(
                Input::new(&self.exclusions_input)
                    .appearance(false)
                    .bordered(false)
                    .focus_bordered(false),
            )
            .into_any_element();
        let export_clipboard = self.clipboard.clone();
        let import_clipboard = self.clipboard.clone();
        let archive_controls = div()
            .flex()
            .items_center()
            .gap(px(5.0))
            .child(
                Button::new("clipboard-export-archive")
                    .ghost()
                    .compact()
                    .xsmall()
                    .h(px(26.0))
                    .px(px(9.0))
                    .rounded(px(RADIUS_CONTROL))
                    .border_1()
                    .border_color(theme.border_subtle)
                    .bg(theme.surface_raised)
                    .disabled(management_busy)
                    .child(div().text_size(px(TEXT_LABEL)).child("导出"))
                    .on_click(move |_event, _window, cx| {
                        let _ = export_clipboard.update(cx, |application, cx| {
                            application.request_clipboard_export(cx);
                        });
                    }),
            )
            .child(
                Button::new("clipboard-import-archive")
                    .ghost()
                    .compact()
                    .xsmall()
                    .h(px(26.0))
                    .px(px(9.0))
                    .rounded(px(RADIUS_CONTROL))
                    .border_1()
                    .border_color(theme.border_subtle)
                    .bg(theme.surface_raised)
                    .disabled(management_busy)
                    .child(div().text_size(px(TEXT_LABEL)).child("导入"))
                    .on_click(move |_event, _window, cx| {
                        let _ = import_clipboard.update(cx, |application, cx| {
                            application.request_clipboard_import(cx);
                        });
                    }),
            )
            .into_any_element();
        let backup_clipboard = self.clipboard.clone();
        let compact_clipboard = self.clipboard.clone();
        let database_controls = div()
            .flex()
            .items_center()
            .gap(px(5.0))
            .child(
                Button::new("clipboard-database-backup")
                    .ghost()
                    .compact()
                    .xsmall()
                    .h(px(26.0))
                    .px(px(9.0))
                    .rounded(px(RADIUS_CONTROL))
                    .border_1()
                    .border_color(theme.border_subtle)
                    .bg(theme.surface_raised)
                    .disabled(management_busy)
                    .child(div().text_size(px(TEXT_LABEL)).child("备份"))
                    .on_click(move |_event, _window, cx| {
                        let _ = backup_clipboard.update(cx, |application, cx| {
                            application.request_clipboard_database_backup(cx);
                        });
                    }),
            )
            .child(
                Button::new("clipboard-database-compact")
                    .ghost()
                    .compact()
                    .xsmall()
                    .h(px(26.0))
                    .px(px(9.0))
                    .rounded(px(RADIUS_CONTROL))
                    .border_1()
                    .border_color(theme.border_subtle)
                    .bg(theme.surface_raised)
                    .disabled(management_busy)
                    .child(div().text_size(px(TEXT_LABEL)).child("压缩"))
                    .on_click(move |_event, _window, cx| {
                        let _ = compact_clipboard.update(cx, |application, cx| {
                            application.request_clipboard_database_compaction(cx);
                        });
                    }),
            )
            .into_any_element();
        let migrate_clipboard = self.clipboard.clone();
        let storage_controls = Button::new("clipboard-storage-migrate")
            .ghost()
            .compact()
            .xsmall()
            .h(px(26.0))
            .px(px(9.0))
            .rounded(px(RADIUS_CONTROL))
            .border_1()
            .border_color(theme.border_subtle)
            .bg(theme.surface_raised)
            .disabled(management_busy)
            .child(div().text_size(px(TEXT_LABEL)).child("迁移"))
            .on_click(move |_event, _window, cx| {
                let _ = migrate_clipboard.update(cx, |application, cx| {
                    application.request_clipboard_storage_migration(cx);
                });
            })
            .into_any_element();
        let interaction_rows = vec![
            compact_setting_row(
                theme,
                "全局呼出热键",
                format!("Windows 注册状态 · {configuration_status}"),
                hotkey_control,
            ),
            compact_setting_row(
                theme,
                "Ctrl + 单击",
                "按住 Ctrl 点击历史条目时执行",
                self.render_modifier_action_selector(
                    ClipboardClickModifier::Control,
                    settings.modifier_actions.control,
                ),
            ),
            compact_setting_row(
                theme,
                "Alt + 单击",
                "按住 Alt 点击历史条目时执行",
                self.render_modifier_action_selector(
                    ClipboardClickModifier::Alt,
                    settings.modifier_actions.alt,
                ),
            ),
            compact_setting_row(
                theme,
                "Shift + 单击",
                "按住 Shift 点击历史条目时执行",
                self.render_modifier_action_selector(
                    ClipboardClickModifier::Shift,
                    settings.modifier_actions.shift,
                ),
            ),
        ];
        let policy_rows = vec![
            compact_setting_row(
                theme,
                "最大普通记录",
                "100–100000；收藏和置顶不计入淘汰队列",
                max_items_control,
            ),
            compact_setting_row(
                theme,
                "自动清理天数",
                "过期记录先进入回收状态，不立即物理删除",
                retention_control,
            ),
            compact_setting_row(
                theme,
                "排除来源进程",
                "分号或逗号分隔；失焦或回车后应用",
                exclusions_control,
            ),
            compact_setting_row(
                theme,
                "历史归档",
                "便携 .nxtclip；图片与原生格式随包迁移",
                archive_controls,
            ),
            compact_setting_row(
                theme,
                "数据库维护",
                "在线备份；压缩前自动备份",
                database_controls,
            ),
            compact_setting_row(theme, "存储位置", storage_directory, storage_controls),
        ];
        div()
            .size_full()
            .flex()
            .flex_col()
            .child(
                ModuleHeader::new(
                    "clipboard-module-header",
                    "clipboard-module-header-title",
                    "clipboard-module-header-metadata",
                    "剪贴板",
                    format!(
                        "ALL {:04}  TXT {:04}  IMG {:04}  FILE {:04}",
                        overview.total, overview.text, overview.images, overview.files
                    ),
                )
                .metadata_monospace()
                .trailing(
                    div()
                        .text_size(px(TEXT_LABEL))
                        .line_height(px(LINE_HEIGHT_LABEL))
                        .text_color(if settings.enabled {
                            theme.accent_text
                        } else {
                            theme.text_muted
                        })
                        .child(if settings.enabled {
                            "● CAPTURE ON"
                        } else {
                            "○ CAPTURE OFF"
                        })
                        .into_any_element(),
                )
                .render(theme),
            )
            .child(
                div()
                    .id("clipboard-settings-workspace")
                    .debug_selector(|| "clipboard-settings-workspace".into())
                    .flex_1()
                    .min_h_0()
                    .p(px(SPACING_12))
                    .child(settings_split_surface(
                        theme,
                        SettingsSplitSurfaceIds {
                            surface: "clipboard-settings-split-surface",
                            primary: "clipboard-settings-primary-column",
                            divider: "clipboard-settings-column-divider",
                            secondary: "clipboard-settings-secondary-column",
                        },
                        CLIPBOARD_SETTINGS_SECONDARY_WIDTH,
                        compact_group(theme, "呼出与快捷操作", interaction_rows),
                        compact_group(theme, "保留与隐私", policy_rows),
                    )),
            )
            .into_any_element()
    }
}

pub(crate) fn click_action_label(action: ClipboardClickAction) -> &'static str {
    match action {
        ClipboardClickAction::OriginalPaste => "原格式粘贴",
        ClipboardClickAction::CopyOnly => "仅复制",
        ClipboardClickAction::PlainText => "纯文本／路径粘贴",
        ClipboardClickAction::WithoutLineBreaks => "去换行粘贴",
        ClipboardClickAction::Disabled => "不执行",
    }
}

pub(crate) fn hotkey_from_key_down(event: &KeyDownEvent) -> Option<ClipboardHotKey> {
    let modifiers = event.keystroke.modifiers;
    let mut semantic_modifiers = Vec::new();
    let mut display = Vec::new();
    if modifiers.control {
        semantic_modifiers.push(ClipboardHotKeyModifier::Control);
        display.push("Ctrl".to_owned());
    }
    if modifiers.alt {
        semantic_modifiers.push(ClipboardHotKeyModifier::Alt);
        display.push("Alt".to_owned());
    }
    if modifiers.shift {
        semantic_modifiers.push(ClipboardHotKeyModifier::Shift);
        display.push("Shift".to_owned());
    }
    if modifiers.platform {
        semantic_modifiers.push(ClipboardHotKeyModifier::Win);
        display.push("Win".to_owned());
    }
    if semantic_modifiers.is_empty() {
        return None;
    }

    let key = event.keystroke.key.trim().to_ascii_lowercase();
    let key_display = if key.len() == 1 {
        let character = key.as_bytes()[0];
        if !character.is_ascii_alphanumeric() {
            return None;
        }
        char::from(character.to_ascii_uppercase()).to_string()
    } else if let Some(number) = key
        .strip_prefix('f')
        .and_then(|value| value.parse::<u32>().ok())
        && (1..=24).contains(&number)
    {
        format!("F{number}")
    } else {
        match key.as_str() {
            "space" => "Space".into(),
            "tab" => "Tab".into(),
            "insert" => "Insert".into(),
            "delete" => "Delete".into(),
            "home" => "Home".into(),
            "end" => "End".into(),
            "pageup" => "PageUp".into(),
            "pagedown" => "PageDown".into(),
            "up" => "Up".into(),
            "down" => "Down".into(),
            "left" => "Left".into(),
            "right" => "Right".into(),
            _ => return None,
        }
    };
    display.push(key_display);
    Some(ClipboardHotKey::new(
        semantic_modifiers,
        display.last().cloned().unwrap_or_default(),
        display.join("+"),
    ))
}
