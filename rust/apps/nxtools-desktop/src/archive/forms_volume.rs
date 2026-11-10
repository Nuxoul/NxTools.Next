//! Exact binary-unit sizing on the original creation form and input entities.
use super::*;
use nxtools_core::archive::{ArchiveVolumeSize, ArchiveVolumeUnit};

pub(super) const VOLUME_PRESETS: &[&str] =
    &["不分卷", "自定义", "100 MiB", "700 MiB", "2 GiB", "4 GiB"];
pub(super) const VOLUME_UNITS: &[&str] = &["KiB", "MiB", "GiB"];
fn preset_size(index: usize) -> Option<ArchiveVolumeSize> {
    match index {
        2 => Some(ArchiveVolumeSize {
            value: 100,
            unit: ArchiveVolumeUnit::MiB,
        }),
        3 => Some(ArchiveVolumeSize {
            value: 700,
            unit: ArchiveVolumeUnit::MiB,
        }),
        4 => Some(ArchiveVolumeSize {
            value: 2,
            unit: ArchiveVolumeUnit::GiB,
        }),
        5 => Some(ArchiveVolumeSize {
            value: 4,
            unit: ArchiveVolumeUnit::GiB,
        }),
        _ => None,
    }
}
pub(super) fn preset_index(value: &str, unit: ArchiveVolumeUnit) -> usize {
    if value.trim().is_empty() {
        return 0;
    }
    let bytes = value
        .trim()
        .parse::<u64>()
        .ok()
        .and_then(|value| value.checked_mul(unit.multiplier()));
    (2..VOLUME_PRESETS.len())
        .find(|index| preset_size(*index).and_then(|size| size.bytes().ok()) == bytes)
        .unwrap_or(1)
}
impl OperationForm {
    pub(super) fn select_volume_preset(
        &mut self,
        index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.busy() {
            return;
        }
        let Draft::Create(draft) = &mut self.draft else {
            return;
        };
        if index == 0 {
            self.volume
                .update(cx, |state, cx| state.set_value("", window, cx));
        } else if let Some(size) = preset_size(index) {
            draft.volume_unit = size.unit;
            self.volume.update(cx, |state, cx| {
                state.set_value(size.value.to_string(), window, cx)
            });
            self.volume_unit.update(cx, |state, cx| {
                state.set_selected_value(&size.unit.label(), window, cx)
            });
        }
        self.revalidate(cx);
    }
    pub(super) fn select_volume_unit(
        &mut self,
        unit: ArchiveVolumeUnit,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.busy() {
            return;
        }
        self.sync_inputs(cx);
        let Draft::Create(draft) = &mut self.draft else {
            return;
        };
        if unit == draft.volume_unit {
            return;
        }
        let result = draft
            .volume_size()
            .and_then(|size| size.map(|size| size.bytes()).transpose());
        match result {
            Ok(bytes) if bytes.is_none_or(|bytes| bytes % unit.multiplier() == 0) => {
                draft.volume_unit = unit;
                if let Some(bytes) = bytes {
                    self.volume.update(cx, |state, cx| {
                        state.set_value((bytes / unit.multiplier()).to_string(), window, cx)
                    });
                }
                self.volume_unit.update(cx, |state, cx| {
                    state.set_selected_value(&unit.label(), window, cx)
                });
                self.revalidate(cx);
            }
            rejected => {
                let old = draft.volume_unit.label();
                self.volume_unit
                    .update(cx, |state, cx| state.set_selected_value(&old, window, cx));
                self.error = Some(match rejected {
                    Err(reason) => reason,
                    Ok(_) => "当前大小不能用所选单位的正整数精确表示。已保留原值；请保留单位或明确修改大小。".into(),
                });
                cx.notify();
            }
        }
    }
    pub(super) fn step_volume(
        &mut self,
        action: gpui_kit::component::input::StepAction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.busy() {
            return;
        }
        let value = self.volume.read(cx).value();
        let parsed = if value.trim().is_empty() {
            Some(0)
        } else if value.trim().bytes().all(|byte| byte.is_ascii_digit()) {
            value.trim().parse::<u64>().ok()
        } else {
            None
        };
        let next = parsed.and_then(|value| match action {
            gpui_kit::component::input::StepAction::Increment => value.checked_add(1),
            gpui_kit::component::input::StepAction::Decrement => {
                value.checked_sub(1).filter(|value| *value > 0)
            }
        });
        if let Some(next) = next {
            self.volume.update(cx, |state, cx| {
                state.set_value(next.to_string(), window, cx)
            });
            self.revalidate(cx);
        }
    }
    pub(super) fn volume_field(&self, busy: bool) -> Field {
        Field::new()
            .label("分卷大小")
            .description("仅 ZIP / 7Z；正整数，至少 64 KiB。不覆盖已有分卷；切换单位不改变字节数。")
            .child(
                Form::vertical()
                    .columns(3)
                    .child(
                        Field::new().label("预设").child(
                            div()
                                .debug_selector(|| "archive-volume-preset".into())
                                .w_full()
                                .child(
                                    Select::new(&self.volume_preset)
                                        .w_full()
                                        .with_size(Size::Medium)
                                        .accessibility_label("分卷预设")
                                        .disabled(busy),
                                ),
                        ),
                    )
                    .child(
                        Field::new().label("数值").child(
                            div()
                                .debug_selector(|| "archive-volume-number".into())
                                .w_full()
                                .child(
                                    gpui_kit::component::input::NumberInput::new(&self.volume)
                                        .with_size(Size::Medium)
                                        .accessibility_label("分卷大小数值")
                                        .disabled(busy),
                                ),
                        ),
                    )
                    .child(
                        Field::new().label("单位").child(
                            div()
                                .debug_selector(|| "archive-volume-unit".into())
                                .w_full()
                                .child(
                                    Select::new(&self.volume_unit)
                                        .w_full()
                                        .with_size(Size::Medium)
                                        .accessibility_label("分卷大小单位")
                                        .disabled(busy),
                                ),
                        ),
                    ),
            )
    }
}
