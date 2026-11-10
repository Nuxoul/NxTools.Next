use super::*;
use gpui_kit::component::{Icon, IconName, Size, tag::Tag};

fn extension_tokens(text: &str) -> Vec<String> {
    let mut values = Vec::new();
    for value in text.split([',', ';', '\n', '\r', '\t', ' ']) {
        let value = value.trim().trim_start_matches('.').to_ascii_lowercase();
        if !value.is_empty() && !values.contains(&value) {
            values.push(value);
        }
    }
    values
}

impl DownloadCategoryEditor {
    pub(super) fn extension_values(&self, cx: &App, include_pending: bool) -> Vec<String> {
        let mut text = self.extensions.read(cx).value().to_string();
        if include_pending && self.extension_editing {
            text.push(',');
            text.push_str(&self.extension_input.read(cx).value());
        }
        extension_tokens(&text)
    }
}

#[derive(Clone)]
enum ExtensionEdit {
    Begin,
    Confirm,
    Cancel,
    Remove(String),
}

impl DownloadSettingsView {
    pub(super) fn subscribe_category_extensions(
        category: &DownloadCategoryEditor,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> [Subscription; 2] {
        let changed = Self::subscribe_category_presentation_input(&category.extensions, window, cx);
        let id = category.id.clone();
        let entered = cx.subscribe_in(
            &category.extension_input,
            window,
            move |this, _, event: &InputEvent, window, cx| {
                if matches!(event, InputEvent::PressEnter { .. }) {
                    this.defer_extension_edit(id.clone(), ExtensionEdit::Confirm, window, cx);
                }
            },
        );
        [changed, entered]
    }

    fn defer_extension_edit(
        &self,
        id: String,
        edit: ExtensionEdit,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.saving {
            return;
        }
        let view = cx.entity().downgrade();
        window.defer(cx, move |window, cx| {
            // Read/write input entities outside the view lease, including input-emitted Enter.
            let Some((extensions, input, editing, return_focus)) = view
                .read_with(cx, |view, _| {
                    if view.saving {
                        return None;
                    }
                    view.selected_category
                        .and_then(|selected| view.categories.get(selected))
                        .filter(|category| category.id == id)
                        .map(|category| {
                            (
                                category.extensions.clone(),
                                category.extension_input.clone(),
                                category.extension_editing,
                                view.extension_add_focus.clone(),
                            )
                        })
                })
                .ok()
                .flatten()
            else {
                return;
            };
            let mut values = extension_tokens(&extensions.read(cx).value());
            match &edit {
                ExtensionEdit::Begin if !editing => {}
                ExtensionEdit::Confirm if editing => {
                    let additions = extension_tokens(&input.read(cx).value());
                    if additions.is_empty() {
                        return;
                    }
                    for value in additions {
                        if !values.contains(&value) {
                            values.push(value);
                        }
                    }
                    extensions.update(cx, |input, cx| {
                        input.set_value(values.join(", "), window, cx)
                    });
                }
                ExtensionEdit::Cancel if editing => {}
                ExtensionEdit::Remove(value) => {
                    values.retain(|existing| existing != value);
                    extensions.update(cx, |input, cx| {
                        input.set_value(values.join(", "), window, cx)
                    });
                }
                _ => return,
            }
            if !matches!(edit, ExtensionEdit::Remove(_)) {
                input.update(cx, |input, cx| input.set_value("", window, cx));
            }
            let _ = view.update(cx, |view, cx| {
                if let Some(category) = view
                    .categories
                    .iter_mut()
                    .find(|category| category.id == id)
                {
                    if !matches!(edit, ExtensionEdit::Remove(_)) {
                        category.extension_editing = matches!(edit, ExtensionEdit::Begin);
                    }
                    cx.notify();
                }
            });
            match edit {
                ExtensionEdit::Begin => input.focus_handle(cx).focus(window, cx),
                ExtensionEdit::Confirm | ExtensionEdit::Cancel => return_focus.focus(window, cx),
                ExtensionEdit::Remove(_) => {}
            }
        });
    }

    pub(super) fn category_extension_field(
        &self,
        category: &DownloadCategoryEditor,
        cx: &mut Context<Self>,
    ) -> Field {
        let values = category.extension_values(cx, false);
        let id = category.id.clone();
        let mut tags = div().flex().flex_wrap().items_center().gap_1().min_w_0();
        for (index, value) in values.into_iter().enumerate() {
            let remove_id = id.clone();
            let remove_value = value.clone();
            tags = tags.child(
                div()
                    .debug_selector(move || format!("download-category-extension-tag-{index}"))
                    .child(
                        Tag::secondary()
                            .outline()
                            .rounded_full()
                            .with_size(Size::Medium)
                            .py_0()
                            .px_2()
                            .gap_1()
                            .child(
                                div()
                                    .max_w(ordinary_rem(144.0))
                                    .truncate()
                                    .child(value.clone()),
                            )
                            .child(
                                Button::new(("download-category-extension-remove", index))
                                    .debug_selector(move || {
                                        format!("download-category-extension-remove-{index}")
                                    })
                                    .icon(IconName::Close)
                                    .ghost()
                                    .with_size(Size::XSmall)
                                    .accessibility_label(format!("删除扩展名 {value}"))
                                    .disabled(self.saving)
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        cx.stop_propagation();
                                        this.defer_extension_edit(
                                            remove_id.clone(),
                                            ExtensionEdit::Remove(remove_value.clone()),
                                            window,
                                            cx,
                                        );
                                    })),
                            ),
                    ),
            );
        }
        let mut body = div().flex_1().min_w_0().flex().flex_col().gap_1().child(
            div()
                .font_weight(FontWeight::MEDIUM)
                .child(category.name.read(cx).value().clone()),
        );
        body = body.child(
            div()
                .debug_selector(|| "download-category-extension-tags".into())
                .w_full()
                .child(tags),
        );
        if !category.extension_editing {
            let add_id = id.clone();
            body = body.child(
                div().flex().child(
                    Button::new("download-category-extension-add")
                        .debug_selector(|| "download-category-extension-add".into())
                        .label("+ 添加")
                        .outline()
                        .rounded(px(16.0))
                        .with_size(Size::Small)
                        .track_focus(&self.extension_add_focus)
                        .disabled(self.saving)
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.defer_extension_edit(
                                add_id.clone(),
                                ExtensionEdit::Begin,
                                window,
                                cx,
                            );
                        })),
                ),
            );
        }
        if category.extension_editing {
            let confirm_id = id.clone();
            let cancel_id = id.clone();
            let escape_id = id;
            body = body.child(
                div()
                    .w_full()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .on_action(cx.listener(
                        move |this, _: &gpui_kit::component::input::Escape, window, cx| {
                            cx.stop_propagation();
                            this.defer_extension_edit(
                                escape_id.clone(),
                                ExtensionEdit::Cancel,
                                window,
                                cx,
                            );
                        },
                    ))
                    .child(
                        div()
                            .w_full()
                            .debug_selector(|| "download-category-extension-input".into())
                            .child(
                                Input::new(&category.extension_input)
                                    .aria_label("新增扩展名")
                                    .disabled(self.saving),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(
                                Button::new("download-category-extension-confirm")
                                    .label("添加")
                                    .outline()
                                    .with_size(Size::Small)
                                    .disabled(self.saving)
                                    .debug_selector(|| "download-category-extension-confirm".into())
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        this.defer_extension_edit(
                                            confirm_id.clone(),
                                            ExtensionEdit::Confirm,
                                            window,
                                            cx,
                                        );
                                    })),
                            )
                            .child(
                                Button::new("download-category-extension-cancel")
                                    .label("取消")
                                    .ghost()
                                    .with_size(Size::Small)
                                    .disabled(self.saving)
                                    .debug_selector(|| "download-category-extension-cancel".into())
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        this.defer_extension_edit(
                                            cancel_id.clone(),
                                            ExtensionEdit::Cancel,
                                            window,
                                            cx,
                                        );
                                    })),
                            ),
                    ),
            );
        }
        download_form_field(
            "扩展名规则",
            "逐项添加；支持逗号分隔的多个扩展名",
            "download-settings-category-extensions-field",
            div()
                .debug_selector(|| "download-settings-category-extensions-field".into())
                .w_full()
                .min_w_0()
                .flex()
                .items_center()
                .gap_2()
                .child(
                    Icon::new(if category.id == "archives" {
                        MaterialIcon::FolderZip
                    } else {
                        MaterialIcon::Folder
                    })
                    .with_size(Size::Medium),
                )
                .child(body)
                .into_any_element(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::{Modifiers, TestAppContext, VisualTestContext};

    fn reveal_extension_control(
        window: &mut VisualTestContext,
        selector: &'static str,
    ) -> gpui_kit::Bounds<gpui_kit::Pixels> {
        for _ in 0..40 {
            window.refresh().expect("render actual rule viewport");
            let form = window
                .debug_bounds("download-settings-category-form")
                .expect("form viewport");
            let footer = window
                .debug_bounds("download-settings-cancel")
                .expect("fixed footer");
            let target = window.debug_bounds(selector);
            let visible_top = px(96.0);
            let visible_bottom = footer.top() - px(12.0);
            if let Some(target) = target
                && form.contains(&target.origin)
                && form.contains(&target.bottom_right())
                && target.top() >= visible_top
                && target.bottom() <= visible_bottom
            {
                return target;
            }
            let outer = target.is_some_and(|target| {
                form.contains(&target.origin) && form.contains(&target.bottom_right())
            });
            let direction = if target
                .is_some_and(|target| target.top() < if outer { visible_top } else { form.top() })
            {
                2.0
            } else {
                -2.0
            };
            let position = if outer {
                gpui_kit::point(form.right() + px(8.0), (visible_top + visible_bottom) / 2.0)
            } else {
                gpui_kit::point(
                    form.right() - px(20.0),
                    form.center().y.max(visible_top).min(visible_bottom),
                )
            };
            window.simulate_event(gpui_kit::ScrollWheelEvent {
                position,
                delta: gpui_kit::ScrollDelta::Lines(gpui_kit::point(0.0, direction)),
                ..Default::default()
            });
            window.run_until_parked();
        }
        panic!(
            "rule control must be reachable: {selector}, target={:?}, viewport={:?}",
            window.debug_bounds(selector),
            window.debug_bounds("download-settings-category-form")
        );
    }

    #[gpui_kit::test]
    fn category_extension_tags_remove_only_the_current_unsaved_rule(cx: &mut TestAppContext) {
        cx.update(gpui_kit::init);
        let (application, directory) =
            super::super::super::tests::download_application_for_window_test(cx, "extension-tags");
        let config = directory.join("config.json");
        AppConfig::default().save_to(&config).expect("owned config");
        let original = std::fs::read(&config).expect("initial config");
        application.update(cx, |application, cx| application.open_settings_window(cx));
        cx.run_until_parked();
        let handle =
            application.read_with(cx, |app, _| app.settings_window.expect("settings window"));
        let mut window = VisualTestContext::from_window(handle, cx);
        window.run_until_parked();
        let view = window.read(|cx| {
            application
                .read(cx)
                .settings_view
                .clone()
                .expect("settings view")
        });
        assert!(
            window
                .debug_bounds("download-category-extension-tags")
                .is_some()
        );
        let remove = reveal_extension_control(&mut window, "download-category-extension-remove-0");
        let before = view.read_with(&window, |view, cx| {
            view.categories[0].settings(cx).extensions
        });
        assert!(before.len() > 1);
        window.simulate_click(remove.center(), Modifiers::default());
        window.run_until_parked();
        let after = view.read_with(&window, |view, cx| {
            view.categories[0].settings(cx).extensions
        });
        assert_eq!(after, before[1..]);
        assert_eq!(std::fs::read(&config).expect("unchanged config"), original);
        assert!(
            window
                .debug_bounds("download-category-extension-add")
                .is_some()
        );
    }

    #[gpui_kit::test]
    fn category_extension_tags_add_enter_cancel_escape_and_saving_keep_original_owners(
        cx: &mut TestAppContext,
    ) {
        cx.update(gpui_kit::init);
        let (application, directory) =
            super::super::super::tests::download_application_for_window_test(
                cx,
                "extension-tag-input",
            );
        let config = directory.join("config.json");
        AppConfig::default().save_to(&config).expect("owned config");
        let original = std::fs::read(&config).expect("original config");
        application.update(cx, |app, cx| app.open_settings_window(cx));
        cx.run_until_parked();
        let handle = application.read_with(cx, |app, _| app.settings_window.expect("settings"));
        let mut window = VisualTestContext::from_window(handle, cx);
        window.run_until_parked();
        let view = window.read(|cx| application.read(cx).settings_view.clone().expect("view"));
        let add = reveal_extension_control(&mut window, "download-category-extension-add");
        window.simulate_click(add.center(), Modifiers::default());
        window.run_until_parked();
        view.read_with(&window, |view, _| {
            assert!(
                view.categories[0].extension_editing,
                "visible Add must open the original category draft; bounds={add:?}"
            )
        });
        assert!(
            window
                .debug_bounds("download-category-extension-input")
                .is_some()
        );
        reveal_extension_control(&mut window, "download-category-extension-input");
        window.simulate_input(".ZIP, WIM, wim, .tar.gz");
        window.simulate_keystrokes("enter");
        window.run_until_parked();
        view.read_with(&window, |view, cx| {
            let rules = view.categories[0].settings(cx).extensions;
            assert_eq!(
                rules.iter().filter(|value| value.as_str() == "zip").count(),
                1
            );
            assert_eq!(
                rules.iter().filter(|value| value.as_str() == "wim").count(),
                1
            );
            assert!(rules.iter().any(|value| value == "tar.gz"));
            assert!(!view.categories[0].extension_editing);
        });
        let add = reveal_extension_control(&mut window, "download-category-extension-add");
        window.simulate_click(add.center(), Modifiers::default());
        window.run_until_parked();
        reveal_extension_control(&mut window, "download-category-extension-input");
        window.simulate_input("pending");
        view.read_with(&window, |view, cx| {
            assert!(
                view.current_draft(cx)
                    .expect("draft includes visible pending rule")
                    .categories[0]
                    .extensions
                    .iter()
                    .any(|value| value == "pending")
            );
        });
        window.simulate_keystrokes("escape");
        window.run_until_parked();
        view.read_with(&window, |view, cx| {
            assert!(!view.categories[0].extension_editing);
            assert!(
                !view.categories[0]
                    .settings(cx)
                    .extensions
                    .iter()
                    .any(|value| value == "pending")
            );
        });
        assert_eq!(std::fs::read(&config).expect("no autosave"), original);
        let before = view.read_with(&window, |view, cx| {
            view.categories[0].settings(cx).extensions
        });
        view.update_in(&mut window, |view, _, cx| {
            view.saving = true;
            cx.notify();
        });
        window.run_until_parked();
        let remove = window
            .debug_bounds("download-category-extension-remove-0")
            .expect("disabled removal");
        window.simulate_click(remove.center(), Modifiers::default());
        window.run_until_parked();
        assert_eq!(
            view.read_with(&window, |view, cx| view.categories[0]
                .settings(cx)
                .extensions),
            before
        );
    }

    #[test]
    fn category_extension_tokens_preserve_compound_suffixes_and_normalize_bulk_input() {
        assert_eq!(
            extension_tokens(" .ZIP, zip; .tar.gz\nRAR\tiso  . "),
            ["zip", "tar.gz", "rar", "iso"]
        );
        assert!(extension_tokens(" , ; \n .").is_empty());
    }
}
