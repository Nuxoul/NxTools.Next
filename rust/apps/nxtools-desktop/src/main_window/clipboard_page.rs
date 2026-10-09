use super::*;
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
        .h(px(28.0))
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
        let theme = self.theme;
        div()
            .id(id)
            .h(px(28.0))
            .rounded(px(RADIUS_CONTROL))
            .border_1()
            .border_color(theme.border)
            .bg(theme.canvas)
            .flex()
            .items_center()
            .child(
                Button::new(gpui::SharedString::from(format!("{id}-minus")))
                    .ghost()
                    .p_0()
                    .rounded(px(RADIUS_CONTROL))
                    .w(px(28.0))
                    .h_full()
                    .cursor_pointer()
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_color(theme.text_muted)
                    .child("−")
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
                Button::new(gpui::SharedString::from(format!("{id}-plus")))
                    .ghost()
                    .p_0()
                    .rounded(px(RADIUS_CONTROL))
                    .w(px(28.0))
                    .h_full()
                    .cursor_pointer()
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_color(theme.text_muted)
                    .child("+")
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
        let theme = self.theme;
        let clipboard = self.clipboard.clone();
        let value_label = format!("{display_value} {suffix}");
        let minus_clipboard = clipboard.clone();
        let plus_clipboard = clipboard;
        div()
            .id(id)
            .h(px(28.0))
            .rounded(px(RADIUS_CONTROL))
            .border_1()
            .border_color(theme.border)
            .bg(theme.canvas)
            .flex()
            .items_center()
            .child(
                Button::new(gpui::SharedString::from(format!("{id}-minus")))
                    .ghost()
                    .p_0()
                    .rounded(px(RADIUS_CONTROL))
                    .w(px(28.0))
                    .h_full()
                    .cursor_pointer()
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_color(theme.text_muted)
                    .child("−")
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
                    .w(px(86.0))
                    .h_full()
                    .border_l_1()
                    .border_r_1()
                    .border_color(theme.border_subtle)
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(px(TEXT_LABEL))
                    .font_family(UI_MONO_FONT_FAMILY)
                    .child(value_label),
            )
            .child(
                Button::new(gpui::SharedString::from(format!("{id}-plus")))
                    .ghost()
                    .p_0()
                    .rounded(px(RADIUS_CONTROL))
                    .w(px(28.0))
                    .h_full()
                    .cursor_pointer()
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_color(theme.text_muted)
                    .child("+")
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
        let overview_metadata = if overview.loading {
            "CLIPBOARD LOADING".to_owned()
        } else if overview.error {
            "CLIPBOARD UNAVAILABLE".to_owned()
        } else {
            format!(
                "ALL {:04}  TXT {:04}  IMG {:04}  FILE {:04}",
                overview.total, overview.text, overview.images, overview.files
            )
        };
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
        let hotkey_control = Button::new("record-clipboard-hotkey")
            .ghost()
            .p_0()
            .h(px(28.0))
            .min_w(px(116.0))
            .px(px(8.0))
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
                    .ghost()
                    .compact()
                    .xsmall()
                    .h(px(28.0))
                    .px(px(8.0))
                    .rounded(px(RADIUS_CONTROL))
                    .border_1()
                    .border_color(theme.border_subtle)
                    .bg(theme.surface_raised)
                    .disabled(management_busy)
                    .child(div().text_size(px(TEXT_LABEL)).child("导出"))
                    .on_click(move |_event, _window, cx| {
                        let clipboard = export_clipboard.clone();
                        cx.defer(move |cx| {
                            let _ = clipboard.update(cx, |application, cx| {
                                application.request_clipboard_export(cx);
                            });
                        });
                    }),
            )
            .child(
                Button::new("clipboard-import-archive")
                    .ghost()
                    .compact()
                    .xsmall()
                    .h(px(28.0))
                    .px(px(8.0))
                    .rounded(px(RADIUS_CONTROL))
                    .border_1()
                    .border_color(theme.border_subtle)
                    .bg(theme.surface_raised)
                    .disabled(management_busy)
                    .child(div().text_size(px(TEXT_LABEL)).child("导入"))
                    .on_click(move |_event, _window, cx| {
                        let clipboard = import_clipboard.clone();
                        cx.defer(move |cx| {
                            let _ = clipboard.update(cx, |application, cx| {
                                application.request_clipboard_import(cx);
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
                    .ghost()
                    .compact()
                    .xsmall()
                    .h(px(28.0))
                    .px(px(8.0))
                    .rounded(px(RADIUS_CONTROL))
                    .border_1()
                    .border_color(theme.border_subtle)
                    .bg(theme.surface_raised)
                    .disabled(management_busy)
                    .child(div().text_size(px(TEXT_LABEL)).child("备份"))
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
                    .ghost()
                    .compact()
                    .xsmall()
                    .h(px(28.0))
                    .px(px(8.0))
                    .rounded(px(RADIUS_CONTROL))
                    .border_1()
                    .border_color(theme.border_subtle)
                    .bg(theme.surface_raised)
                    .disabled(management_busy)
                    .child(div().text_size(px(TEXT_LABEL)).child("压缩"))
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
            .ghost()
            .compact()
            .xsmall()
            .h(px(28.0))
            .px(px(8.0))
            .rounded(px(RADIUS_CONTROL))
            .border_1()
            .border_color(theme.border_subtle)
            .bg(theme.surface_raised)
            .disabled(management_busy)
            .child(div().text_size(px(TEXT_LABEL)).child("迁移"))
            .on_click(move |_event, _window, cx| {
                let clipboard = migrate_clipboard.clone();
                cx.defer(move |cx| {
                    let _ = clipboard.update(cx, |application, cx| {
                        application.request_clipboard_storage_migration(cx);
                    });
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
                "文本大小上限",
                "按 UTF-8 字节计算；超限文本跳过，不截断保存",
                max_text_control,
            ),
            compact_setting_row(
                theme,
                "图片文件上限",
                "输出文件大小；超限图片跳过并保留其他格式",
                max_image_control,
            ),
            compact_setting_row(
                theme,
                "图片像素上限",
                "宽 × 高；超限图片跳过并保留其他格式",
                max_pixels_control,
            ),
            compact_setting_row(
                theme,
                "应用黑名单",
                "在全局设置中为每个应用分别禁用剪贴板或速启",
                action_button("clipboard-open-global-blacklist", "全局设置")
                    .ghost()
                    .disabled(self.network_settings.is_none())
                    .on_click(cx.listener(|this, _, _, cx| {
                        if let Some(application) = this.network_settings.clone() {
                            cx.defer(move |cx| {
                                application.update(cx, |owner, cx| owner.open_window(cx))
                            });
                        }
                    }))
                    .into_any_element(),
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
                    overview_metadata,
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
