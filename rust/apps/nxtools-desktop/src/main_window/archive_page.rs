//! Owned archive settings and real browser/create/extract entry points.
use super::*;
use gpui_kit::component::alert::Alert;
use gpui_kit::component::group_box::GroupBoxVariant;
use gpui_kit::component::setting::{
    SettingField, SettingGroup, SettingItem, SettingPage, Settings,
};
use gpui_kit::component::switch::Switch;
use gpui_kit::{SharedString, WeakEntity};
use nxtools_core::AppConfig;
use nxtools_core::archive::*;
use std::time::Instant;

pub(super) struct ArchiveSettingsView {
    pub(super) settings: ArchiveSettings,
    config_path: PathBuf,
    pub(super) exclusions: Entity<InputState>,
    pub(super) error: Option<String>,
}
impl ArchiveSettingsView {
    pub(super) fn new(
        settings: ArchiveSettings,
        config_path: PathBuf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let exclusions = cx.new(|cx| {
            InputState::new(window, cx).default_value(
                serde_json::to_string(&settings.exclude_patterns).expect("string list JSON"),
            )
        });
        Self {
            settings,
            config_path,
            exclusions,
            error: None,
        }
    }
    fn save(&mut self, settings: ArchiveSettings, cx: &mut Context<Self>) {
        if let Err(message) = settings.validate() {
            self.error = Some(message);
            cx.notify();
            return;
        }
        let correlation_id = uuid::Uuid::new_v4();
        let started = Instant::now();
        info!(event = "archive.settings.save_started", %correlation_id, "archive settings save accepted");
        match AppConfig::update_archive_settings(&self.config_path, settings) {
            Ok(merged) => {
                self.settings = merged.archive;
                crate::archive::settings_updated(self.settings.clone(), cx);
                self.error = None;
                info!(event = "archive.settings.save_finished", %correlation_id, outcome = "saved", elapsed_ms = started.elapsed().as_millis() as u64, "archive settings save finished");
            }
            Err(_) => {
                self.error = Some(
                    "设置未保存，仍保留上次生效值。请检查配置文件是否完整、可写，然后重试。".into(),
                );
                tracing::warn!(event = "archive.settings.save_finished", %correlation_id, outcome = "failed", reason = "config_update_failed", elapsed_ms = started.elapsed().as_millis() as u64, "archive settings save finished");
            }
        }
        cx.notify();
    }
    fn switch(
        &self,
        owner: WeakEntity<Self>,
        id: &'static str,
        label: &'static str,
        value: bool,
        apply: fn(&mut ArchiveSettings, bool),
    ) -> SettingItem {
        SettingItem::new(
            label,
            SettingField::render(move |_, _, _| {
                let owner = owner.clone();
                div().debug_selector(move || id.into()).child(
                    Switch::new(id)
                        .checked(value)
                        .accessibility_label(label)
                        .on_change(move |requested, _, cx| {
                            let requested = *requested;
                            let owner = owner.clone();
                            cx.defer(move |cx| {
                                let _ = owner.update(cx, |view, cx| {
                                    let mut settings = view.settings.clone();
                                    apply(&mut settings, requested);
                                    view.save(settings, cx);
                                });
                            });
                        }),
                )
            }),
        )
    }
    fn choice<T: Copy + PartialEq + 'static>(
        &self,
        owner: WeakEntity<Self>,
        label: &'static str,
        current: T,
        options: &'static [(T, &'static str)],
        apply: fn(&mut ArchiveSettings, T),
    ) -> SettingItem {
        let value: SharedString = options
            .iter()
            .find(|(value, _)| *value == current)
            .expect("released settings choice")
            .1
            .into();
        let choices = options
            .iter()
            .map(|(_, label)| (SharedString::from(*label), SharedString::from(*label)))
            .collect();
        SettingItem::new(
            label,
            SettingField::scrollable_dropdown(
                choices,
                move |_| value.clone(),
                move |requested, cx| {
                    let Some((requested, _)) = options
                        .iter()
                        .find(|(_, label)| *label == requested.as_ref())
                    else {
                        return;
                    };
                    let requested = *requested;
                    let owner = owner.clone();
                    cx.defer(move |cx| {
                        let _ = owner.update(cx, |view, cx| {
                            let mut settings = view.settings.clone();
                            apply(&mut settings, requested);
                            view.save(settings, cx);
                        });
                    });
                },
            ),
        )
    }
    fn exclusions_item(&self, owner: WeakEntity<Self>) -> SettingItem {
        let input = self.exclusions.clone();
        SettingItem::new(
            "排除规则",
            SettingField::render(move |_, _, _| {
                let owner = owner.clone();
                div()
                    .w_full()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(Input::new(&input).aria_label("压缩排除规则"))
                    .child("JSON 字符串数组，如 [\"Thumbs.db\",\"desktop.ini\"]；使用 [] 清空。保留文件名中的空格、分号及转义字符。")
                    .child(
                        Button::new("archive-exclusions-save")
                            .debug_selector(|| "archive-exclusions-save".into())
                            .label("保存排除规则")
                            .outline()
                            .on_click(move |_, _, cx| {
                                let owner = owner.clone();
                                cx.defer(move |cx| {
                                    let _ = owner.update(cx, |view, cx| {
                                        let mut settings = view.settings.clone();
                                        let Ok(patterns) = serde_json::from_str::<Vec<String>>(&view.exclusions.read(cx).value()) else {
                                            view.error = Some("排除规则需为 JSON 字符串数组，例如 [\"*.tmp\"]。未保存，请修正后重试。".into());
                                            cx.notify();
                                            return;
                                        };
                                        settings.exclude_patterns = patterns;
                                        view.save(settings, cx);
                                    });
                                });
                            }),
                    )
            }),
        )
        .layout(gpui_kit::Axis::Vertical)
        .keywords(["排除", "glob", "Thumbs.db", "desktop.ini"])
    }
}
fn note(label: &'static str, text: &'static str) -> SettingItem {
    SettingItem::new(
        label,
        SettingField::render(move |_, _, _| div().min_w_0().w_full().text_sm().child(text)),
    )
    .layout(gpui_kit::Axis::Vertical)
}
impl Render for ArchiveSettingsView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let owner = cx.entity().downgrade();
        let settings = &self.settings;
        let extract = vec![
            self.choice(
                owner.clone(),
                "默认目标策略",
                settings.default_destination,
                &[
                    (ArchiveDestination::Ask, "询问"),
                    (ArchiveDestination::Smart, "智能解压"),
                    (ArchiveDestination::Here, "当前文件夹"),
                    (ArchiveDestination::Name, "压缩包同名文件夹"),
                ],
                |settings, value| settings.default_destination = value,
            ),
            self.choice(
                owner.clone(),
                "同名文件处理",
                settings.conflict_policy,
                &[
                    (ArchiveConflictPolicy::Ask, "询问"),
                    (ArchiveConflictPolicy::Overwrite, "覆盖"),
                    (ArchiveConflictPolicy::Skip, "跳过"),
                    (ArchiveConflictPolicy::AutoRename, "自动重命名"),
                ],
                |settings, value| settings.conflict_policy = value,
            ),
            self.choice(
                owner.clone(),
                "默认代码页",
                settings.filename_encoding,
                &[
                    (ArchiveFilenameEncoding::Auto, "自动检测"),
                    (ArchiveFilenameEncoding::Utf8, "UTF-8"),
                    (ArchiveFilenameEncoding::Gbk, "简体中文 GBK (936)"),
                    (ArchiveFilenameEncoding::Big5, "繁体中文 Big5 (950)"),
                    (ArchiveFilenameEncoding::ShiftJis, "日文 Shift-JIS (932)"),
                    (ArchiveFilenameEncoding::Korean, "韩文 (949)"),
                    (ArchiveFilenameEncoding::Cp437, "CP437"),
                    (ArchiveFilenameEncoding::Windows1250, "Windows-1250"),
                    (ArchiveFilenameEncoding::Windows1251, "Windows-1251"),
                    (ArchiveFilenameEncoding::Windows1252, "Windows-1252"),
                ],
                |settings, value| settings.filename_encoding = value,
            ),
            note(
                "文件名识别",
                "自动检测可能不确定；归档中有效的 Unicode 文件名优先，不会被手动代码页覆盖。",
            ),
        ];
        let create = vec![
            self.choice(
                owner.clone(),
                "默认压缩格式",
                settings.create_format,
                &[
                    (ArchiveCreateFormat::Zip, "ZIP"),
                    (ArchiveCreateFormat::SevenZ, "7z"),
                    (ArchiveCreateFormat::Tar, "TAR"),
                    (ArchiveCreateFormat::Tgz, "TAR.GZ"),
                    (ArchiveCreateFormat::Txz, "TAR.XZ"),
                    (ArchiveCreateFormat::Gz, "Gzip（单文件）"),
                    (ArchiveCreateFormat::Xz, "XZ（单文件）"),
                ],
                |settings, value| settings.create_format = value,
            ),
            self.choice(
                owner.clone(),
                "默认压缩级别",
                settings.compression_level,
                if matches!(
                    settings.create_format,
                    ArchiveCreateFormat::Gz | ArchiveCreateFormat::Xz
                ) {
                    &[
                        (ArchiveCompressionLevel::Fast, "快速"),
                        (ArchiveCompressionLevel::Normal, "标准"),
                        (ArchiveCompressionLevel::Maximum, "最大"),
                    ]
                } else {
                    &[
                        (ArchiveCompressionLevel::Store, "仅存储"),
                        (ArchiveCompressionLevel::Fast, "快速"),
                        (ArchiveCompressionLevel::Normal, "标准"),
                        (ArchiveCompressionLevel::Maximum, "最大"),
                    ]
                },
                |settings, value| settings.compression_level = value,
            )
            .disabled(settings.create_format == ArchiveCreateFormat::Tar),
            note(
                "级别适用范围",
                "级别按格式能力映射，不是通用数字。TAR 不压缩；Gzip / XZ 不使用“仅存储”。此偏好保留，创建表单接入后按实际格式校验。",
            ),
            self.switch(
                owner.clone(),
                "archive-test-after-create",
                "压缩后测试",
                settings.test_after_create,
                |settings, value| settings.test_after_create = value,
            ),
            self.switch(
                owner.clone(),
                "archive-strip-single-root",
                "单个文件夹不创建根文件夹",
                settings.strip_single_root,
                |settings, value| settings.strip_single_root = value,
            ),
            self.exclusions_item(owner.clone()),
        ];
        let pages = [
            SettingPage::new("常规").resettable(false).group(SettingGroup::new().title("压缩模块").items([
                note("配置与操作", "默认参数用于新表单；浏览本页不会启动任务或创建输出。"),
                self.switch(owner.clone(),"archive-open-in-nxtools","使用 NxTools 打开压缩包",settings.open_in_nxtools,|settings,value|settings.open_in_nxtools=value),
                note("保存方式", "选项修改后立即保存；排除规则需点击“保存排除规则”。设置按独立 archive 配置节合并，不覆盖其他模块。"),
            ])),
            SettingPage::new("解压").resettable(false).group(SettingGroup::new().title("解压默认参数").items(extract)),
            SettingPage::new("压缩").resettable(false).group(SettingGroup::new().title("新建默认参数").items(create)),
            SettingPage::new("查看").resettable(false).group(SettingGroup::new().title("归档浏览器").item(note("浏览器状态", "目录树、列宽、排序和注释面板由独立归档窗口管理；窗口状态与配置分开保存。"))),
            SettingPage::new("集成").resettable(false).group(SettingGroup::new().title("Windows 集成").items([
                note("文件关联", "默认关闭，尚未接入。当前不会注册扩展名或修改 Windows 默认应用。"),
                note("资源管理器右键菜单", "默认关闭，尚未接入真实 Apply。此页面不会写入注册表，也不会显示虚假的应用成功。"),
            ])),
            SettingPage::new("高级").resettable(false).group(SettingGroup::new().title("安全与清理").items([
                self.switch(owner, "archive-copy-motw", "保留下载文件安全标记", settings.copy_motw, |settings, value| settings.copy_motw = value),
                note("安全标记", "保存 MOTW 处理偏好；原生执行阶段报告真实结果，不把配置保存当作已处理文件。"),
                note("源文件保护", "创建/解压不会删除或回收源文件；严格原对象回收能力尚未验收。临时会话只按既有租约和身份检查清理。"),
            ])),
        ];
        let mut page = div()
            .id("archive-settings-page")
            .debug_selector(|| "archive-settings-page".into())
            .size_full()
            .min_w_0()
            .min_h_0()
            .flex()
            .flex_col()
            .child(
                ModuleHeader::new(
                    "archive-module-header",
                    "archive-module-title",
                    "archive-module-status",
                    "压缩",
                    "归档浏览与默认参数",
                )
                .render(Theme::active(cx)),
            )
            .child(
                div()
                    .flex_none()
                    .px_4()
                    .py_2()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        Button::new("archive-open")
                            .debug_selector(|| "archive-open".into())
                            .label("打开压缩包…")
                            .outline()
                            .on_click(|_, window, cx| {
                                crate::archive::pick_from_window(window.window_handle(), cx)
                            }),
                    )
                    .child(
                        Button::new("archive-create")
                            .debug_selector(|| "archive-create".into())
                            .label("新建压缩包…")
                            .outline()
                            .on_click(|_, _, cx| crate::archive::new_from_window(cx)),
                    )
                    .child(
                        Button::new("archive-extract")
                            .debug_selector(|| "archive-extract".into())
                            .label("解压到…")
                            .outline()
                            .on_click(|_, _, cx| crate::archive::extract_from_window(cx)),
                    ),
            )
            .child(
                div()
                    .debug_selector(|| "archive-settings-viewport".into())
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .child(
                        Settings::new("archive-official-settings")
                            .sidebar_width(px(160.0))
                            .sidebar_size_range(px(160.0)..px(208.0))
                            .with_group_variant(GroupBoxVariant::Normal)
                            .pages(pages),
                    ),
            );
        if let Some(message) = &self.error {
            page = page.child(
                div()
                    .debug_selector(|| "archive-settings-error".into())
                    .flex_none()
                    .p_2()
                    .child(Alert::error("archive-save-error", message.clone()).small()),
            );
        }
        page
    }
}
impl MainWindow {
    pub(super) fn render_archive(&self) -> AnyElement {
        match &self.archive_settings {
            Some(view) => view.clone().into_any_element(),
            None => div()
                .p_4()
                .child("压缩设置当前不可用：配置所有者尚未初始化。")
                .into_any_element(),
        }
    }
}
