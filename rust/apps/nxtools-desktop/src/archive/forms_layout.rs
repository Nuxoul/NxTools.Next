//! Task-ordered presentation on the original OperationForm draft and callbacks.
use super::*;
use gpui_kit::component::collapsible::Collapsible;

impl OperationForm {
    fn destination_field(&self, here: bool, cx: &Context<Self>) -> Field {
        Field::new().label("目标文件夹").required(true).child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .min_w_0()
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .debug_selector(|| "archive-destination-group".into())
                        .child(
                            InputGroup::new("archive-destination-group")
                                .w_full()
                                .input(Input::new(&self.destination).aria_label("目标文件夹"))
                                .readonly(here)
                                .disabled(self.busy()),
                        ),
                )
                .child(
                    self.picker_button(
                        "archive-destination-picker",
                        "浏览…",
                        ArchivePickerPurpose::Destination,
                        cx,
                    )
                    .flex_none()
                    .disabled(self.busy() || here),
                ),
        )
    }
    fn preferences(&self, reclamation: bool, cx: &Context<Self>) -> Field {
        Field::new()
            .label("完成后")
            .child(self.check(
                "archive-open-after",
                "打开目标文件夹",
                self.open_after,
                Flag::Open,
                false,
                cx,
            ))
            .child(self.check(
                "archive-keep-open",
                "保留进度窗口",
                self.keep_open,
                Flag::Keep,
                false,
                cx,
            ))
            .child(self.check(
                "archive-remember",
                "记住本次设置",
                self.remember,
                Flag::Remember,
                reclamation,
                cx,
            ))
    }
    fn disclosure(&self, content: impl IntoElement, cx: &Context<Self>) -> impl IntoElement {
        Collapsible::new()
            .open(self.space.options_open)
            .child(
                Button::new("archive-options-toggle")
                    .debug_selector(|| "archive-options-toggle".into())
                    .label(if self.space.options_open {
                        "收起更多选项"
                    } else {
                        "更多选项"
                    })
                    .outline()
                    .with_size(Size::Small)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.space.options_open = !this.space.options_open;
                        cx.notify();
                    })),
            )
            .content(div().pt_3().child(content))
    }
    pub(super) fn create_fields(&self, cx: &Context<Self>) -> gpui_kit::AnyElement {
        let Draft::Create(draft) = &self.draft else {
            return div().into_any_element();
        };
        let busy = self.busy();
        let stream = matches!(
            draft.format,
            ArchiveCreateFormat::Gz | ArchiveCreateFormat::Xz
        );
        let mut sources = div().flex().flex_col().gap_1().min_w_0();
        for (index, source) in draft.sources.iter().enumerate() {
            let full_path = source.path.to_string_lossy().into_owned();
            let name = source
                .path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| full_path.clone());
            let detail = if source.directory {
                "文件夹".into()
            } else {
                source
                    .bytes
                    .map(super::super::progress::format_bytes)
                    .unwrap_or_else(|| "大小未知".into())
            };
            sources = sources.child(
                div()
                    .id(("archive-create-source", index))
                    .h(px(32.))
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap_2()
                    .min_w_0()
                    .tooltip(move |window, cx| {
                        gpui_kit::component::tooltip::Tooltip::new(full_path.clone())
                            .build(window, cx)
                    })
                    .child(div().flex_1().min_w_0().truncate().text_sm().child(name))
                    .child(
                        div()
                            .flex_none()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(detail),
                    )
                    .child(
                        Button::new(("archive-remove-source", index))
                            .debug_selector(move || format!("archive-create-remove-source-{index}"))
                            .label("移除")
                            .outline()
                            .with_size(Size::Small)
                            .disabled(busy)
                            .on_click(cx.listener(move |this, _, _, cx| {
                                if this.busy() {
                                    return;
                                }
                                if let Draft::Create(draft) = &mut this.draft
                                    && index < draft.sources.len()
                                {
                                    draft.sources.remove(index);
                                }
                                this.revalidate(cx);
                            })),
                    ),
            );
        }
        let known_bytes = draft
            .sources
            .iter()
            .filter_map(|source| source.bytes)
            .fold(0u64, u64::saturating_add);
        let unknown = draft
            .sources
            .iter()
            .filter(|source| source.bytes.is_none())
            .count();
        let summary = format!(
            "{} 个来源 · 已知大小 {}{}",
            draft.sources.len(),
            super::super::progress::format_bytes(known_bytes),
            if unknown == 0 {
                String::new()
            } else {
                format!(" · {unknown} 项大小待扫描")
            }
        );
        let sources = div()
            .id("archive-create-source-viewport")
            .debug_selector(|| "archive-create-source-viewport".into())
            .w_full()
            .min_w_0()
            .min_h(px(32.))
            .max_h(px(144.))
            .flex_none()
            .overflow_y_scroll()
            .child(sources);
        let format = Form::vertical()
            .with_size(Size::Small)
            .columns(2)
            .child(
                Field::new().label("格式").child(
                    Select::new(&self.format)
                        .w_full()
                        .accessibility_label("压缩格式")
                        .disabled(busy),
                ),
            )
            .child(
                Field::new()
                    .label("压缩等级")
                    .visible(draft.format != ArchiveCreateFormat::Tar)
                    .child(
                        Select::new(&self.level)
                            .w_full()
                            .accessibility_label("语义压缩等级")
                            .disabled(busy),
                    ),
            );
        let password_open = self.space.password_open
            || !self.password.read(cx).value().is_empty()
            || !self.confirm.read(cx).value().is_empty();
        let header_enabled = !self.password.read(cx).value().is_empty()
            && self.password.read(cx).value() == self.confirm.read(cx).value();
        let password = Collapsible::new()
            .open(password_open)
            .child(
                Button::new("archive-password-toggle")
                    .label(if password_open {
                        "密码已展开"
                    } else {
                        "设置密码…"
                    })
                    .outline()
                    .with_size(Size::Small)
                    .disabled(busy)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.space.password_open = !this.space.password_open;
                        cx.notify();
                    })),
            )
            .content(
                div().pt_3().child(
                    Form::vertical()
                        .columns(2)
                        .child(
                            Field::new().label("密码").child(
                                Input::new(&self.password)
                                    .aria_label("创建密码")
                                    .mask_toggle()
                                    .disabled(busy),
                            ),
                        )
                        .child(
                            Field::new()
                                .label("确认密码")
                                .when(
                                    self.password.read(cx).value() != self.confirm.read(cx).value(),
                                    |field| field.description("两次密码不一致"),
                                )
                                .child(
                                    Input::new(&self.confirm)
                                        .aria_label("确认创建密码")
                                        .mask_toggle()
                                        .disabled(busy),
                                ),
                        ),
                ),
            );
        let advanced = Form::vertical()
            .gap_y_3()
            .child(
                Field::new()
                    .label("ZIP 方法")
                    .visible(draft.format == ArchiveCreateFormat::Zip)
                    .description("密码保护使用 AES-256。")
                    .child(
                        Select::new(&self.method)
                            .w_full()
                            .accessibility_label("ZIP 方法")
                            .disabled(busy),
                    ),
            )
            .child(
                Field::new()
                    .label("7Z 选项")
                    .visible(draft.format == ArchiveCreateFormat::SevenZ)
                    .when(!header_enabled, |field| {
                        field.description("文件名加密需要先输入并确认密码。")
                    })
                    .child(self.check(
                        "archive-solid",
                        "固实压缩",
                        draft.solid,
                        Flag::Solid,
                        false,
                        cx,
                    ))
                    .child(self.check(
                        "archive-header",
                        "加密文件名（需要密码）",
                        draft.encrypt_header,
                        Flag::Header,
                        !header_enabled,
                        cx,
                    )),
            )
            .child(
                Field::new()
                    .label("文件与校验")
                    .child(self.check(
                        "archive-separately",
                        "分别压缩每个来源",
                        draft.separately,
                        Flag::Separately,
                        false,
                        cx,
                    ))
                    .child(self.check(
                        "archive-strip",
                        "单个文件夹不保留根目录",
                        draft.strip_single_root,
                        Flag::Strip,
                        stream
                            || draft.sources.len() != 1
                            || !draft.sources.first().is_some_and(|source| source.directory),
                        cx,
                    ))
                    .child(self.check(
                        "archive-test",
                        "创建后测试压缩包",
                        draft.test_after_create,
                        Flag::Test,
                        false,
                        cx,
                    )),
            )
            .child(self.volume_field(busy))
            .child(
                Field::new()
                    .label("排除规则")
                    .visible(!stream)
                    .description("每项一条规则，支持 * 和 ?，最多 32 条。")
                    .child(self.exclusion_fields(cx)),
            )
            .child(self.preferences(false, cx));
        Form::vertical()
            .with_size(Size::Small)
            .gap_y_2()
            .child(
                Field::new()
                    .label(format!("来源（{}）", draft.sources.len()))
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .flex_wrap()
                            .child(self.paste_button(cx))
                            .child(self.picker_button(
                                "archive-add-files",
                                "添加文件…",
                                ArchivePickerPurpose::Files,
                                cx,
                            ))
                            .child(self.picker_button(
                                "archive-add-folders",
                                "添加文件夹…",
                                ArchivePickerPurpose::Folders,
                                cx,
                            )),
                    )
                    .child(sources)
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(summary),
                    ),
            )
            .child(
                Field::new().child(
                    Form::horizontal()
                        .with_size(Size::Small)
                        .label_width(px(96.))
                        .gap_y_2()
                        .child(self.destination_field(false, cx))
                        .child(
                            Field::new()
                                .label("压缩包名称")
                                .required(true)
                                .when(draft.separately, |field| {
                                    field.description("按每个来源名称分别生成压缩包。")
                                })
                                .when(
                                    !draft.separately
                                        && !valid_job_name(self.name.read(cx).value().as_ref()),
                                    |field| {
                                        field.description(
                                            "名称不能包含路径或 Windows 不允许的字符。",
                                        )
                                    },
                                )
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap_2()
                                        .min_w_0()
                                        .child(
                                            div().flex_1().min_w_0().child(
                                                Input::new(&self.name)
                                                    .aria_label("压缩包名称")
                                                    .disabled(busy || draft.separately),
                                            ),
                                        )
                                        .child(self.picker_button(
                                            "archive-save-picker",
                                            "另存为…",
                                            ArchivePickerPurpose::Save,
                                            cx,
                                        )),
                                ),
                        )
                        .child(Field::new().child(format)),
                ),
            )
            .child(
                Field::new()
                    .visible(draft::encrypted(draft.format))
                    .child(password),
            )
            .child(Field::new().child(self.disclosure(advanced, cx)))
            .into_any_element()
    }
    pub(super) fn extract_fields(&self, cx: &Context<Self>) -> gpui_kit::AnyElement {
        let Draft::Extract(draft) = &self.draft else {
            return div().into_any_element();
        };
        if draft.continue_reclamation {
            return self.continuation_fields(cx);
        }
        let has_source = !draft.source.as_os_str().is_empty();
        let selected = draft
            .selection
            .as_ref()
            .map_or(0, |selection| selection.ordinals.len());
        let here = draft.strategy == ArchiveDestination::Here;
        let mut source = Field::new().label("压缩包");
        if has_source {
            source = source.child(
                div()
                    .text_sm()
                    .child(draft.source.to_string_lossy().into_owned()),
            );
        } else {
            source = source.description("选择要解压的压缩包");
        }
        if !self.source_bound {
            source = source.child(self.picker_button(
                "archive-extract-source",
                "选择压缩包…",
                ArchivePickerPurpose::Files,
                cx,
            ));
        }
        let range = Field::new().label("文件范围").child(if selected == 0 {
            div().text_sm().child("全部文件").into_any_element()
        } else {
            RadioGroup::horizontal("archive-extract-range")
                .selected_index(Some(usize::from(draft.selected_only)))
                .disabled(self.busy())
                .child(Radio::new("all").label("全部文件"))
                .child(Radio::new("selected").label(format!("选定文件（{selected}）")))
                .on_change(cx.listener(|this, index, _, cx| {
                    if this.busy() {
                        return;
                    }
                    if let Draft::Extract(draft) = &mut this.draft {
                        draft.selected_only = *index == 1
                            && draft
                                .selection
                                .as_ref()
                                .is_some_and(|selection| !selection.ordinals.is_empty());
                    }
                    this.revalidate(cx);
                }))
                .into_any_element()
        });
        let destination_options = Form::vertical()
            .columns(2)
            .child(
                Field::new().label("文件夹安排").child(
                    Select::new(&self.strategy)
                        .w_full()
                        .accessibility_label("解压目标策略")
                        .disabled(self.busy()),
                ),
            )
            .child(
                Field::new().label("同名文件").child(
                    Select::new(&self.conflict)
                        .w_full()
                        .accessibility_label("同名文件处理")
                        .disabled(self.busy()),
                ),
            );
        let arrangement = match draft.strategy {
            ArchiveDestination::Name => Some(format!(
                "解压到“{}”子文件夹；重名时使用新名称。",
                draft::archive_stem(
                    &draft
                        .source
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                )
            )),
            ArchiveDestination::Smart => {
                Some("单一顶层项直接解压，多个顶层项放入压缩包同名文件夹。".into())
            }
            _ => None,
        };
        let advanced = Form::vertical()
            .gap_y_3()
            .child(
                Field::new()
                    .label("文件保护")
                    .child(self.check(
                        "archive-single-file-folder",
                        "智能解压时单个文件也创建文件夹",
                        draft.single_file_folder,
                        Flag::SingleFile,
                        draft.strategy != ArchiveDestination::Smart,
                        cx,
                    ))
                    .child(self.check(
                        "archive-motw",
                        "保留下载来源标记",
                        draft.copy_motw,
                        Flag::Motw,
                        false,
                        cx,
                    )),
            )
            .child(self.preferences(draft.reclamation, cx))
            .child(Field::new().child(self.check(
                "archive-remember-destination",
                "同时记住自定义目标文件夹",
                draft.remember_destination,
                Flag::RememberDestination,
                here || draft.reclamation,
                cx,
            )));
        let mut form = Form::vertical()
            .gap_y_3()
            .child(source)
            .child(range)
            .child(self.destination_field(here, cx))
            .child(
                Field::new()
                    .child(destination_options)
                    .when_some(arrangement, |field, hint| field.description(hint)),
            )
            .child(Field::new().label("磁盘空间").child(self.space_section(cx)));
        if has_source {
            form = form.child(Field::new().child(self.reclamation_section(cx)));
        }
        form.child(Field::new().child(self.disclosure(advanced, cx)))
            .into_any_element()
    }
    fn reclamation_section(&self, cx: &Context<Self>) -> impl IntoElement {
        let Draft::Extract(draft) = &self.draft else {
            return div();
        };
        let refusal = draft.reclamation_refusal();
        let zip = draft
            .source
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("zip"));
        let mut content = div().flex().flex_col().gap_2().min_w_0();
        content = content.child(self.check(
            "archive-reclamation",
            "边解压边释放原件空间",
            draft.reclamation,
            Flag::Reclamation,
            refusal.is_some(),
            cx,
        ));
        if draft.reclamation {
            content = content.child(div().debug_selector(|| "archive-reclamation-warning".into()).child(
                Alert::warning("archive-reclamation-warning-alert", "原包数据释放后不能撤销，原压缩包将不再完整可用。保留原包、恢复记录和已解压文件，才能继续剩余解压。")
                    .with_size(Size::Small)))
                .child(div().text_sm().text_color(cx.theme().muted_foreground)
                    .child("仅支持普通单卷、未加密、仅存储（Stored）的 ZIP，且来源与目标在同一本地 NTFS 卷。开始前还会检查；常见 Deflate ZIP、7z 和 RAR 不支持此模式。"));
        } else if let Some(reason) = refusal {
            content = content.child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(reason),
            );
            if zip {
                content = content.child(
                    Button::new("archive-reclamation-prepare")
                        .debug_selector(|| "archive-reclamation-prepare".into())
                        .label("使用全部文件和当前文件夹")
                        .outline()
                        .with_size(Size::Small)
                        .disabled(self.busy())
                        .on_click(cx.listener(|this, _, window, cx| {
                            if this.busy() {
                                return;
                            }
                            if let Draft::Extract(draft) = &mut this.draft {
                                draft.selected_only = false;
                                draft.single_file_folder = false;
                                draft.strategy = ArchiveDestination::Here;
                                draft.conflict = ArchiveConflictPolicy::Ask;
                                let destination = draft
                                    .source
                                    .parent()
                                    .map(|path| path.to_string_lossy().into_owned())
                                    .unwrap_or_default();
                                this.destination.update(cx, |input, cx| {
                                    input.set_value(destination, window, cx)
                                });
                                this.strategy.update(cx, |state, cx| {
                                    state.set_selected_value(&STRATEGIES[2], window, cx)
                                });
                                this.conflict.update(cx, |state, cx| {
                                    state.set_selected_value(&CONFLICTS[0], window, cx)
                                });
                            }
                            this.revalidate(cx);
                        })),
                );
            }
        } else {
            content = content.child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child("可选且不可撤销；仅存储 ZIP 才可能适用，开始前检查。普通解压保留原包。"),
            );
        }
        content
    }
}
