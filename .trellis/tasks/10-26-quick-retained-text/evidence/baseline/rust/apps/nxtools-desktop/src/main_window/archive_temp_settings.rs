//! Temp controls on the original Settings view; no new configuration/lifetime owner.
use super::*;

impl ArchiveSettingsView {
    pub(super) fn save_temp_path(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let requested = self.temp_path.read(cx).value().to_string();
        let mut policy = self.settings.temp.clone();
        policy.base_directory = requested.clone();
        if let Err(message) = policy.validate() {
            self.error = Some(message);
            cx.notify();
            return;
        }
        let baseline = self.temp_baseline.clone();
        let mut conflict = false;
        let correlation_id = uuid::Uuid::new_v4();
        let started = Instant::now();
        tracing::info!(event="archive.temp_settings.save_started", %correlation_id, "temporary directory save accepted");
        let result = crate::archive::save_settings(&self.config_path, |settings| {
            if requested == baseline {
                return;
            }
            if settings.temp.base_directory != baseline && settings.temp.base_directory != requested
            {
                conflict = true;
            } else {
                settings.temp.base_directory = requested.clone();
            }
        });
        match result {
            Ok(_) if conflict => {
                self.error = Some(
                    "临时目录已被其他操作修改；未覆盖新设置，草稿已保留。请重新载入目录后编辑。"
                        .into(),
                );
            }
            Ok(saved) => {
                self.settings = saved.settings().clone();
                self.temp_baseline = self.settings.temp.base_directory.clone();
                self.temp_path.update(cx, |input, cx| {
                    input.set_value(self.temp_baseline.clone(), window, cx)
                });
                self.error = None;
                crate::archive::saved_settings_updated(saved, cx);
            }
            Err(_) => {
                self.error = Some("无法保存临时目录；当前草稿已保留，请检查配置后重试。".into())
            }
        }
        tracing::info!(event="archive.temp_settings.save_finished", %correlation_id, success=self.error.is_none(), elapsed_ms=started.elapsed().as_millis() as u64, "temporary directory save finished");
        cx.notify();
    }

    pub(super) fn pick_temp_path(&mut self, cx: &mut Context<Self>) {
        if self.temp_picker_pending {
            return;
        }
        self.temp_picker_pending = true;
        let baseline = self.temp_path.read(cx).value().to_string();
        let revision = self.temp_revision.get();
        let owner = cx.entity().downgrade();
        let source = self.window;
        crate::archive::pick_temp_directory(
            baseline.clone(),
            move |result, cx| {
                let owner = owner.clone();
                let baseline = baseline.clone();
                cx.defer(move |cx| {
                    let _ = source.update(cx, |_, window, cx| {
                        let _ = owner.update(cx, |view, cx| {
                            view.temp_picker_pending = false;
                            match result {
                                Ok(Some(path))
                                    if view.temp_revision.get() == revision
                                        && view.temp_path.read(cx).value().as_ref() == baseline =>
                                {
                                    view.temp_path
                                        .update(cx, |input, cx| input.set_value(path, window, cx));
                                    view.temp_revision
                                        .set(view.temp_revision.get().wrapping_add(1));
                                }
                                Err(_) => {
                                    view.error =
                                        Some("无法选择临时目录；当前草稿未改变，请重试。".into())
                                }
                                _ => {}
                            }
                            cx.notify();
                        });
                    });
                });
            },
            cx,
        );
        cx.notify();
    }

    pub(super) fn temp_items(&self, owner: WeakEntity<Self>) -> Vec<SettingItem> {
        let path = self.temp_path.clone();
        let pending = self.temp_picker_pending;
        let path_owner = owner.clone();
        let hours = self.settings.temp.retention_hours;
        let hours_owner = owner.clone();
        vec![
            SettingItem::new("临时目录", SettingField::render(move |_, _, _| {
                let browse_owner = path_owner.clone();
                let save_owner = path_owner.clone();
                let default_owner = path_owner.clone();
                let reload_owner = path_owner.clone();
                div().w_full().flex().flex_col().gap_2()
                    .child(Input::new(&path).aria_label("归档临时目录").w_full())
                    .child(div().flex().flex_wrap().gap_2()
                        .child(Button::new("archive-temp-browse").debug_selector(|| "archive-temp-browse".into()).label("浏览…").outline().with_size(Size::Small).loading(pending).disabled(pending).on_click(move |_, _, cx| {
                            let owner = browse_owner.clone();
                            cx.defer(move |cx| { let _ = owner.update(cx, |view, cx| view.pick_temp_path(cx)); });
                        }))
                        .child(Button::new("archive-temp-save").debug_selector(|| "archive-temp-save".into()).label("保存目录").outline().with_size(Size::Small).on_click(move |_, window, cx| {
                            let owner = save_owner.clone(); let source = window.window_handle();
                            cx.defer(move |cx| { let _ = source.update(cx, |_, window, cx| { let _ = owner.update(cx, |view, cx| view.save_temp_path(window, cx)); }); });
                        }))
                        .child(Button::new("archive-temp-default").debug_selector(|| "archive-temp-default".into()).label("使用默认目录").outline().with_size(Size::Small).on_click(move |_, window, cx| {
                            let owner = default_owner.clone(); let source = window.window_handle();
                            cx.defer(move |cx| { let _ = source.update(cx, |_, window, cx| { let _ = owner.update(cx, |view, cx| {
                                view.temp_path.update(cx, |input, cx| input.set_value("", window, cx));
                                view.save_temp_path(window, cx);
                            }); }); });
                        }))
                        .child(Button::new("archive-temp-reload").label("载入当前目录").outline().with_size(Size::Small).on_click(move |_, window, cx| {
                            let owner = reload_owner.clone(); let source = window.window_handle();
                            cx.defer(move |cx| { let _ = source.update(cx, |_, window, cx| { let _ = owner.update(cx, |view, cx| {
                                match AppConfig::load_from(&view.config_path) {
                                    Ok(config) => { view.temp_baseline = config.archive.temp.base_directory; view.temp_path.update(cx, |input, cx| input.set_value(view.temp_baseline.clone(), window, cx)); view.error = None; }
                                    Err(_) => view.error = Some("无法载入当前临时目录；草稿未改变。".into()),
                                }
                                cx.notify();
                            }); }); });
                        })))
            })).layout(gpui_kit::Axis::Vertical),
            SettingItem::new("失效临时文件保留期", SettingField::scrollable_dropdown(
                [(1, "1 小时"), (6, "6 小时"), (24, "24 小时"), (72, "3 天"), (168, "7 天")].into_iter().map(|(value, label)| (SharedString::from(value.to_string()), SharedString::from(label))).collect(),
                move |_| SharedString::from(hours.to_string()),
                move |value, cx| {
                    let Ok(hours) = value.parse::<u32>() else { return; };
                    let owner = hours_owner.clone();
                    cx.defer(move |cx| { let _ = owner.update(cx, |view, cx| { let mut settings = view.settings.clone(); settings.temp.retention_hours = hours; view.save(settings, cx); }); });
                },
            )),
            self.switch(owner, "archive-temp-child-exit", "可追踪子进程退出后尝试清理", self.settings.temp.cleanup_on_child_exit, |settings, value| settings.temp.cleanup_on_child_exit = value),
            note("清理范围", "只影响新打开条目的专属缓存。活动租约、被编辑或锁定文件不会强制删除；无可追踪进程时保留原关闭／到期清理。保留期不是删除期限。"),
        ]
    }
}
