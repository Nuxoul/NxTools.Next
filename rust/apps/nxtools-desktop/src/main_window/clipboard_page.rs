use super::*;
use gpui_kit::component::form::{Field, Form};
use gpui_kit::component::group_box::GroupBoxVariant;
use gpui_kit::component::setting::{SettingGroup, SettingItem, SettingPage, Settings};
use tracing::error;

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
#[derive(Debug, Clone, Copy)]
pub(crate) enum ClipboardCaptureLimitKind {
    TextBytes,
    ImageBytes,
    ImagePixels,
}

fn update_clipboard_capture_limit(
    application: &mut crate::clipboard::ClipboardApplication,
    kind: ClipboardCaptureLimitKind,
    value: u64,
) {
    let settings = application.clipboard_settings();
    let (text, image, pixels) = match kind {
        ClipboardCaptureLimitKind::TextBytes => {
            (value, settings.max_image_bytes, settings.max_image_pixels)
        }
        ClipboardCaptureLimitKind::ImageBytes => {
            (settings.max_text_bytes, value, settings.max_image_pixels)
        }
        ClipboardCaptureLimitKind::ImagePixels => {
            (settings.max_text_bytes, settings.max_image_bytes, value)
        }
    };
    application.set_clipboard_capture_limits(text, image, pixels);
}
#[derive(Default, Clone)]
pub(crate) struct ClipboardOverview {
    pub(crate) total: usize,
    pub(crate) text: usize,
    pub(crate) images: usize,
    pub(crate) files: usize,
    pub(crate) loading: bool,
    pub(crate) error: bool,
}

impl MainWindow {
    pub(crate) fn clipboard_overview(&self) -> ClipboardOverview {
        self.clipboard_overview.clone()
    }

    pub(crate) fn request_clipboard_overview(&mut self, cx: &mut Context<Self>) {
        if self.clipboard_overview_loading {
            self.clipboard_overview_dirty = true;
            return;
        }
        let Some(worker) = self
            .clipboard
            .upgrade()
            .map(|application| application.read(cx).clipboard_worker_handle())
        else {
            self.clipboard_overview = ClipboardOverview {
                error: true,
                ..ClipboardOverview::default()
            };
            cx.notify();
            return;
        };
        self.clipboard_overview_loading = true;
        self.clipboard_overview_dirty = false;
        self.clipboard_overview_generation =
            self.clipboard_overview_generation.wrapping_add(1).max(1);
        let generation = self.clipboard_overview_generation;
        self.clipboard_overview.loading = true;
        self.clipboard_overview.error = false;
        let main_window = cx.entity().downgrade();
        cx.spawn(async move |_, cx| {
            let result = worker.counts_async().await;
            let _ = main_window.update(cx, |main_window, cx| {
                if main_window.clipboard_overview_generation != generation {
                    return;
                }
                main_window.clipboard_overview_loading = false;
                match result {
                    Ok(counts) => {
                        main_window.clipboard_overview = ClipboardOverview {
                            total: counts.total,
                            text: counts.text,
                            images: counts.images,
                            files: counts.files,
                            loading: false,
                            error: false,
                        };
                    }
                    Err(error) => {
                        error!(%error, "failed to load clipboard counts");
                        main_window.clipboard_overview.loading = false;
                        main_window.clipboard_overview.error = true;
                    }
                }
                if main_window.clipboard_overview_dirty {
                    main_window.request_clipboard_overview(cx);
                }
                cx.notify();
            });
        })
        .detach();
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
        let clipboard = self.clipboard.clone();
        Button::new(match modifier {
            ClipboardClickModifier::Control => "clipboard-modifier-control",
            ClipboardClickModifier::Alt => "clipboard-modifier-alt",
            ClipboardClickModifier::Shift => "clipboard-modifier-shift",
        })
        .debug_selector(move || match modifier {
            ClipboardClickModifier::Control => "clipboard-modifier-control".into(),
            ClipboardClickModifier::Alt => "clipboard-modifier-alt".into(),
            ClipboardClickModifier::Shift => "clipboard-modifier-shift".into(),
        })
        .outline()
        .label(click_action_label(current))
        .dropdown_caret(true)
        .dropdown_menu_with_anchor(Anchor::TopRight, move |menu, _window, _cx| {
            ClipboardClickAction::ALL.into_iter().fold(
                menu.min_w(px(154.0)).max_w(px(154.0)),
                |menu, action| {
                    let clipboard = clipboard.clone();
                    let label = click_action_label(action);
                    menu.item(
                        PopupMenuItem::new(label)
                            .checked(action == current)
                            .on_click(move |_event, _window, cx| {
                                let clipboard = clipboard.clone();
                                cx.defer(move |cx| {
                                    let _ = clipboard.update(cx, |application, _| {
                                        application.set_clipboard_modifier_action(modifier, action);
                                    });
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
        div()
            .id(id)
            .flex()
            .items_center()
            .gap_2()
            .child(
                Button::new(gpui_kit::SharedString::from(format!("{id}-minus")))
                    .small()
                    .outline()
                    .icon(gpui_kit::component::IconName::Minus)
                    .accessibility_label(format!("减少 {id}"))
                    .on_click(cx.listener(move |this, _event, _window, cx| {
                        cx.stop_propagation();
                        let clipboard = this.clipboard.clone();
                        cx.defer(move |cx| {
                            let _ = clipboard.update(cx, |application, _| {
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
                        });
                        cx.notify();
                    })),
            )
            .child(
                div()
                    .min_w(px(72.0))
                    .text_sm()
                    .font_family(UI_MONO_FONT_FAMILY)
                    .text_center()
                    .child(value.to_string()),
            )
            .child(
                Button::new(gpui_kit::SharedString::from(format!("{id}-plus")))
                    .small()
                    .outline()
                    .icon(MaterialIcon::Add)
                    .accessibility_label(format!("增加 {id}"))
                    .on_click(cx.listener(move |this, _event, _window, cx| {
                        cx.stop_propagation();
                        let clipboard = this.clipboard.clone();
                        cx.defer(move |cx| {
                            let _ = clipboard.update(cx, |application, _| {
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
                        });
                        cx.notify();
                    })),
            )
            .into_any_element()
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn render_capture_limit_stepper(
        &mut self,
        id: &'static str,
        value: u64,
        step: u64,
        kind: ClipboardCaptureLimitKind,
        display_value: String,
        suffix: &'static str,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let clipboard = self.clipboard.clone();
        let value_label = format!("{display_value} {suffix}");
        let minus_clipboard = clipboard.clone();
        let plus_clipboard = clipboard;
        div()
            .id(id)
            .flex()
            .items_center()
            .gap_2()
            .child(
                Button::new(gpui_kit::SharedString::from(format!("{id}-minus")))
                    .small()
                    .outline()
                    .icon(gpui_kit::component::IconName::Minus)
                    .accessibility_label(format!("减少 {id}"))
                    .on_click(cx.listener(move |_, _event, _window, cx| {
                        cx.stop_propagation();
                        let clipboard = minus_clipboard.clone();
                        cx.defer(move |cx| {
                            let _ = clipboard.update(cx, |application, _| {
                                update_clipboard_capture_limit(
                                    application,
                                    kind,
                                    value.saturating_sub(step),
                                );
                            });
                        });
                        cx.notify();
                    })),
            )
            .child(
                div()
                    .min_w(px(86.0))
                    .text_sm()
                    .font_family(UI_MONO_FONT_FAMILY)
                    .text_center()
                    .child(value_label),
            )
            .child(
                Button::new(gpui_kit::SharedString::from(format!("{id}-plus")))
                    .small()
                    .outline()
                    .icon(MaterialIcon::Add)
                    .accessibility_label(format!("增加 {id}"))
                    .on_click(cx.listener(move |_, _event, _window, cx| {
                        cx.stop_propagation();
                        let clipboard = plus_clipboard.clone();
                        cx.defer(move |cx| {
                            let _ = clipboard.update(cx, |application, _| {
                                update_clipboard_capture_limit(
                                    application,
                                    kind,
                                    value.saturating_add(step),
                                );
                            });
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
        let overview = self.clipboard_overview();
        let metadata = if overview.loading {
            "正在加载历史统计…".to_owned()
        } else if overview.error {
            "历史统计暂不可用".to_owned()
        } else {
            format!(
                "共 {} 条 · 文本 {} · 图片 {} · 文件 {}",
                overview.total, overview.text, overview.images, overview.files
            )
        };
        let settings = self.clipboard_settings(cx);
        let owner = cx.entity().downgrade();
        let item =
            |slot: usize, title: &'static str, help: &'static str, visible: bool, wide: bool| {
                let owner = owner.clone();
                let render = move |_: &gpui_kit::component::setting::RenderOptions,
                                   _: &mut Window,
                                   cx: &mut App| {
                    let control = owner
                        .update(cx, |main, cx| main.render_clipboard_control(slot, cx))
                        .unwrap_or_else(|_| div().into_any_element());
                    div()
                        .debug_selector(move || format!("clipboard-setting-{slot}"))
                        .w_full()
                        .min_w_0()
                        .when(!wide, |row| row.flex().items_center().justify_end())
                        .child(control)
                };
                if wide {
                    SettingItem::render(render).keywords([title, help])
                } else {
                    SettingItem::render(move |options, window, cx| {
                        div()
                            .w_full()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(crate::ui::components::settings::aligned_setting_row(
                                format!("clipboard-row-{slot}"),
                                title,
                                if visible { "" } else { help },
                                format!("clipboard-setting-{slot}-help"),
                                crate::ui::metrics::ordinary_rem(300.0),
                                div()
                                    .w_full()
                                    .flex()
                                    .items_center()
                                    .justify_end()
                                    .child(render(options, window, cx))
                                    .into_any_element(),
                            ))
                            .when(visible, |row| {
                                row.child(
                                    div().text_sm().text_color(theme.text_secondary).child(help),
                                )
                            })
                    })
                    .keywords([title, help])
                }
            };
        let height_owner = self.clipboard.clone();
        let height_reader = self.clipboard.clone();
        let height_setting = SettingItem::new(
            "文本卡最大高度",
            gpui_kit::component::setting::SettingField::number_input(
                gpui_kit::component::setting::NumberFieldOptions {
                    min: f64::from(nxtools_core::CLIPBOARD_TEXT_CARD_MIN_HEIGHT),
                    max: f64::from(nxtools_core::CLIPBOARD_TEXT_CARD_MAX_HEIGHT),
                    step: 1.0,
                },
                move |cx| {
                    height_reader.upgrade().map(|owner| f64::from(owner.read(cx).clipboard_settings().text_card_max_height))
                        .unwrap_or(f64::from(nxtools_core::default_clipboard_text_card_max_height()))
                },
                move |value, cx| {
                    if value.is_finite() && value.fract() == 0.0
                        && (f64::from(nxtools_core::CLIPBOARD_TEXT_CARD_MIN_HEIGHT)..=f64::from(nxtools_core::CLIPBOARD_TEXT_CARD_MAX_HEIGHT)).contains(&value)
                    {
                        let owner = height_owner.clone();
                        let height = value as u32;
                        cx.defer(move |cx| {
                            let _ = owner.update(cx, |owner, cx| owner.set_text_card_max_height(height, cx));
                        });
                    }
                },
            ),
        ).description("整张文本卡片的最大高度，120–320 DIP，默认 160；短文本按内容自然收缩，不影响完整原文。卡片间距另计。");
        let image_owner = self.clipboard.clone();
        let image_mode = settings.image_view_mode;
        let image_choices = nxtools_core::ClipboardImageViewMode::ALL
            .into_iter()
            .map(|mode| {
                (
                    gpui_kit::SharedString::from(mode.label()),
                    gpui_kit::SharedString::from(mode.label()),
                )
            })
            .collect();
        let image_setting = SettingItem::new(
            "视图模式",
            gpui_kit::component::setting::SettingField::scrollable_dropdown(
                image_choices,
                move |_| gpui_kit::SharedString::from(image_mode.label()),
                move |value: gpui_kit::SharedString, cx| {
                    if let Some(mode) = nxtools_core::ClipboardImageViewMode::ALL
                        .into_iter()
                        .find(|mode| mode.label() == value.as_ref())
                    {
                        let owner = image_owner.clone();
                        cx.defer(move |cx| {
                            let _ =
                                owner.update(cx, |owner, cx| owner.set_image_view_mode(mode, cx));
                        });
                    }
                },
            ),
        )
        .description(
            nxtools_core::ClipboardImageViewMode::ALL
                .into_iter()
                .map(|mode| format!("{}：{}", mode.label(), mode.description()))
                .collect::<Vec<_>>()
                .join("\n\n"),
        );
        let pages = [
            SettingPage::new("呼出与快捷操作").resettable(false).group(
                SettingGroup::new().title("热键与点击动作").items([
                    item(0, "全局呼出热键", "热键 Windows 注册", false, true),
                    item(
                        1,
                        "Ctrl + 单击",
                        "按住 Ctrl 点击历史条目时执行",
                        false,
                        false,
                    ),
                    item(2, "Alt + 单击", "按住 Alt 点击历史条目时执行", false, false),
                    item(
                        3,
                        "Shift + 单击",
                        "按住 Shift 点击历史条目时执行",
                        false,
                        false,
                    ),
                ]),
            ),
            SettingPage::new("保留与隐私").resettable(false).group(
                SettingGroup::new().title("保留规则与捕获上限").items([
                    item(
                        4,
                        "最大普通记录",
                        "100–100000；收藏和置顶不计入淘汰队列",
                        true,
                        false,
                    ),
                    item(
                        5,
                        "自动清理天数",
                        "1–3650 天；过期记录先进入回收状态，不立即物理删除",
                        true,
                        false,
                    ),
                    item(
                        6,
                        "文本大小上限",
                        "按 UTF-8 字节计算；超限文本跳过，不截断保存",
                        true,
                        false,
                    ),
                    item(
                        7,
                        "图片文件上限",
                        "输出文件大小；超限图片跳过并保留其他格式",
                        true,
                        false,
                    ),
                    item(
                        8,
                        "图片像素上限",
                        "宽 × 高；超限图片跳过并保留其他格式",
                        true,
                        false,
                    ),
                    item(
                        9,
                        "应用黑名单",
                        "在全局设置中为每个应用分别禁用剪贴板或速启",
                        false,
                        false,
                    ),
                ]),
            ),
            SettingPage::new("压缩包与存储").resettable(false).group(
                SettingGroup::new().title("数据管理").items([
                    item(
                        10,
                        "历史压缩包",
                        "便携 .nxtclip；图片与原生格式随包迁移",
                        false,
                        false,
                    ),
                    item(11, "数据库维护", "在线备份；压缩前自动备份", true, false),
                    item(12, "存储位置", "迁移", false, true),
                ]),
            ),
            SettingPage::new("历史显示")
                .resettable(false)
                .group(SettingGroup::new().title("文本密度").item(height_setting)),
            SettingPage::new("图片预览")
                .resettable(false)
                .group(SettingGroup::new().title("大图浮窗").item(image_setting)),
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
                    metadata,
                )
                .trailing(
                    div()
                        .text_size(px(TEXT_LABEL))
                        .line_height(px(LINE_HEIGHT_LABEL))
                        .text_color(if settings.enabled {
                            self.theme.accent_text
                        } else {
                            self.theme.text_muted
                        })
                        .child(if settings.enabled {
                            "捕获已启用"
                        } else {
                            "捕获已停用"
                        })
                        .into_any_element(),
                )
                .render(self.theme),
            )
            .child(
                div()
                    .debug_selector(|| "clipboard-settings-workspace".into())
                    .flex_1()
                    .min_h_0()
                    .child(
                        Settings::new("clipboard-official-settings")
                            .sidebar_width(px(160.0))
                            .with_group_variant(GroupBoxVariant::Fill)
                            .pages(pages),
                    ),
            )
            .into_any_element()
    }
    fn render_clipboard_control(&mut self, slot: usize, cx: &mut Context<Self>) -> AnyElement {
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
        let (custom_hotkey, reset_ready) = self
            .clipboard
            .upgrade()
            .map(|app| {
                let app = app.read(cx);
                (
                    app.clipboard_hotkey_is_custom(),
                    app.can_reset_clipboard_hotkey(),
                )
            })
            .unwrap_or_default();
        let recorder = Button::new("record-clipboard-hotkey")
            .debug_selector(|| "record-clipboard-hotkey".into())
            .disabled(hotkey_update_pending)
            .outline()
            .label(hotkey_label)
            .on_click(cx.listener(move |this, _, window, cx| {
                if !hotkey_update_pending {
                    let recording = this.recording_hotkey != Some(HotkeyRecordingTarget::Clipboard);
                    if this.recording_hotkey.take().is_some() {
                        this.defer_all_hotkey_recording(false, cx);
                    }
                    this.recording_hotkey = recording.then_some(HotkeyRecordingTarget::Clipboard);
                    this.defer_all_hotkey_recording(recording, cx);
                    if recording {
                        this.hotkey_focus.focus(window, cx);
                    }
                    cx.notify();
                }
            }));
        let hotkey_control = div()
            .flex()
            .items_center()
            .justify_end()
            .gap_2()
            .when(custom_hotkey, |row| {
                row.child(
                    Button::new("clipboard-hotkey-reset")
                        .label("重置")
                        .outline()
                        .tooltip("恢复默认快捷键 Alt+C")
                        .disabled(!reset_ready)
                        .on_click(cx.listener(|this, _, _, cx| {
                            let application = this.clipboard.clone();
                            cx.defer(move |cx| {
                                let _ = application.update(cx, |app, cx| {
                                    app.reset_clipboard_hotkey();
                                    cx.notify();
                                });
                            });
                        })),
                )
            })
            .child(recorder)
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
        let max_text_control = self.render_capture_limit_stepper(
            "clipboard-max-text-bytes",
            settings.max_text_bytes,
            64 * 1024,
            ClipboardCaptureLimitKind::TextBytes,
            format!("{:.2}", settings.max_text_bytes as f64 / (1024.0 * 1024.0)),
            "MB",
            cx,
        );
        let max_image_control = self.render_capture_limit_stepper(
            "clipboard-max-image-bytes",
            settings.max_image_bytes,
            1024 * 1024,
            ClipboardCaptureLimitKind::ImageBytes,
            (settings.max_image_bytes / (1024 * 1024)).to_string(),
            "MB",
            cx,
        );
        let max_pixels_control = self.render_capture_limit_stepper(
            "clipboard-max-image-pixels",
            settings.max_image_pixels,
            1_000_000,
            ClipboardCaptureLimitKind::ImagePixels,
            (settings.max_image_pixels / 1_000_000).to_string(),
            "MP",
            cx,
        );
        let export_clipboard = self.clipboard.clone();
        let import_clipboard = self.clipboard.clone();
        let archive_controls = div()
            .flex()
            .items_center()
            .gap(px(4.0))
            .child(
                Button::new("clipboard-export-archive")
                    .small()
                    .outline()
                    .label("导出…")
                    .debug_selector(|| "clipboard-export-archive".into())
                    .disabled(management_busy)
                    .on_click(move |_event, window, cx| {
                        let clipboard = export_clipboard.clone();
                        let source_window_handle =
                            crate::ui::native_window_handle(window).unwrap_or_default();
                        cx.defer(move |cx| {
                            let _ = clipboard.update(cx, |application, cx| {
                                application.request_clipboard_export(source_window_handle, cx);
                            });
                        });
                    }),
            )
            .child(
                Button::new("clipboard-import-archive")
                    .small()
                    .outline()
                    .label("导入…")
                    .debug_selector(|| "clipboard-import-archive".into())
                    .disabled(management_busy)
                    .on_click(move |_event, window, cx| {
                        let clipboard = import_clipboard.clone();
                        let source_window_handle =
                            crate::ui::native_window_handle(window).unwrap_or_default();
                        cx.defer(move |cx| {
                            let _ = clipboard.update(cx, |application, cx| {
                                application.request_clipboard_import(source_window_handle, cx);
                            });
                        });
                    }),
            )
            .into_any_element();
        let backup_clipboard = self.clipboard.clone();
        let compact_clipboard = self.clipboard.clone();
        let database_controls = div()
            .flex()
            .items_center()
            .gap(px(4.0))
            .child(
                Button::new("clipboard-database-backup")
                    .small()
                    .outline()
                    .label("备份")
                    .debug_selector(|| "clipboard-database-backup".into())
                    .disabled(management_busy)
                    .on_click(move |_event, _window, cx| {
                        let clipboard = backup_clipboard.clone();
                        cx.defer(move |cx| {
                            let _ = clipboard.update(cx, |application, cx| {
                                application.request_clipboard_database_backup(cx);
                            });
                        });
                    }),
            )
            .child(
                Button::new("clipboard-database-compact")
                    .small()
                    .outline()
                    .label("压缩")
                    .debug_selector(|| "clipboard-database-compact".into())
                    .disabled(management_busy)
                    .on_click(move |_event, _window, cx| {
                        let clipboard = compact_clipboard.clone();
                        cx.defer(move |cx| {
                            let _ = clipboard.update(cx, |application, cx| {
                                application.request_clipboard_database_compaction(cx);
                            });
                        });
                    }),
            )
            .into_any_element();
        let migrate_clipboard = self.clipboard.clone();
        let storage_controls = Button::new("clipboard-storage-migrate")
            .small()
            .outline()
            .label("迁移…")
            .debug_selector(|| "clipboard-storage-migrate".into())
            .disabled(management_busy)
            .on_click(move |_event, window, cx| {
                let clipboard = migrate_clipboard.clone();
                let source_window_handle =
                    crate::ui::native_window_handle(window).unwrap_or_default();
                cx.defer(move |cx| {
                    let _ = clipboard.update(cx, |application, cx| {
                        application.request_clipboard_storage_migration(source_window_handle, cx);
                    });
                });
            })
            .into_any_element();
        match slot {
            0 => div()
                .w_full()
                .flex()
                .flex_col()
                .gap_1()
                .child(crate::ui::components::settings::aligned_setting_row(
                    "clipboard-hotkey-row",
                    "全局呼出热键",
                    "点击字段开始录制；Esc 或失焦取消",
                    "clipboard-hotkey-help",
                    crate::ui::metrics::ordinary_rem(300.0),
                    hotkey_control,
                ))
                .child(
                    div()
                        .debug_selector(|| "clipboard-hotkey-registration-status".into())
                        .text_sm()
                        .text_color(self.theme.text_secondary)
                        .child(format!("Windows 注册状态 · {configuration_status}")),
                )
                .into_any_element(),
            1 => self.render_modifier_action_selector(
                ClipboardClickModifier::Control,
                settings.modifier_actions.control,
            ),
            2 => self.render_modifier_action_selector(
                ClipboardClickModifier::Alt,
                settings.modifier_actions.alt,
            ),
            3 => self.render_modifier_action_selector(
                ClipboardClickModifier::Shift,
                settings.modifier_actions.shift,
            ),
            4 => max_items_control,
            5 => retention_control,
            6 => max_text_control,
            7 => max_image_control,
            8 => max_pixels_control,
            9 => action_button("clipboard-open-global-blacklist", "全局设置")
                .outline()
                .disabled(self.network_settings.is_none())
                .on_click(cx.listener(|this, _, _, cx| {
                    if let Some(application) = this.network_settings.clone() {
                        cx.defer(move |cx| {
                            application.update(cx, |owner, cx| owner.open_window(cx))
                        });
                    }
                }))
                .into_any_element(),
            10 => archive_controls,
            11 => database_controls,
            12 => Form::vertical()
                .small()
                .child(
                    Field::new()
                        .label("存储位置")
                        .gap_0()
                        .child(
                            div()
                                .id("clipboard-storage-path")
                                .debug_selector(|| "clipboard-storage-path".into())
                                .w_full()
                                .min_w_0()
                                .text_sm()
                                .whitespace_normal()
                                .child(storage_directory),
                        )
                        .child(div().mt_2().child(storage_controls)),
                )
                .into_any_element(),
            _ => div().into_any_element(),
        }
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

#[cfg(test)]
mod kit_migration_tests;
