//! Archive preview preferences on the original official Settings owner.
use super::*;
use gpui_kit::component::setting::NumberFieldOptions;

impl ArchiveSettingsView {
    fn preview_number(
        &self,
        owner: WeakEntity<Self>,
        title: &'static str,
        value: u32,
        range: std::ops::RangeInclusive<u32>,
        set: fn(&mut ArchivePreviewSettings, u32),
    ) -> SettingItem {
        let (minimum, maximum) = (*range.start(), *range.end());
        SettingItem::new(
            title,
            SettingField::number_input(
                NumberFieldOptions {
                    min: f64::from(minimum),
                    max: f64::from(maximum),
                    step: 1.,
                },
                move |_| f64::from(value),
                move |value, cx| {
                    if !value.is_finite()
                        || value.fract() != 0.
                        || !(f64::from(minimum)..=f64::from(maximum)).contains(&value)
                    {
                        return;
                    }
                    let owner = owner.clone();
                    cx.defer(move |cx| {
                        let _ = owner.update(cx, |view, cx| {
                            let mut settings = view.settings.clone();
                            set(&mut settings.preview, value as u32);
                            view.save(settings, cx);
                        });
                    });
                },
            )
            .w(crate::ui::metrics::ordinary_rem(200.)),
        )
    }
    pub(super) fn preview_page(&self, owner: WeakEntity<Self>) -> SettingPage {
        let preview = self.settings.preview;
        SettingPage::new("预览").resettable(false)
            .group(SettingGroup::new().title("文本").items([
                self.preview_number(owner.clone(), "文本大小上限（KiB）", preview.text_limit_kib,
                    1..=8192, |settings, value| settings.text_limit_kib = value),
                self.preview_number(owner.clone(), "文本最大行数", preview.text_max_lines,
                    1..=100_000, |settings, value| settings.text_max_lines = value),
                note("文本预览", "仅支持 UTF-8 文本、代码和 Markdown；文件大小与行数限制同时生效。"),
            ]))
            .group(SettingGroup::new().title("图片").items([
                self.preview_number(owner.clone(), "图片文件大小上限（MiB）", preview.image_limit_mib,
                    1..=128, |settings, value| settings.image_limit_mib = value),
                self.preview_number(owner.clone(), "图片最大像素（百万）", preview.image_max_megapixels,
                    1..=64, |settings, value| settings.image_max_megapixels = value),
                self.preview_number(owner.clone(), "图片解码预算（MiB）", preview.image_decode_mib,
                    32..=1024, |settings, value| settings.image_decode_mib = value),
                note("图片预览", "每边最多 8192 像素；百万像素按 1,000,000 计算。大小、像素与解码预算同时生效；解码预算不是进程总内存上限。"),
            ]))
            .group(SettingGroup::new().title("处理").items([
                self.preview_number(owner, "解压工作时限（秒）", preview.timeout_seconds,
                    5..=120, |settings, value| settings.timeout_seconds = value),
                note("生效范围", "自动保存，下次选择或刷新预览时应用；正在处理的预览保持原设置。KiB/MiB 使用二进制单位；压缩包读取、中间解包及临时空间额度独立计算。"),
            ]))
    }
}
