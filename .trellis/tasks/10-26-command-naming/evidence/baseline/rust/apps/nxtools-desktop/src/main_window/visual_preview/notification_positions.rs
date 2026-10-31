//! Isolated production notification rendering; no platform/runtime/user-data owners.
use super::*;
use crate::notifications::{
    AppNotification, NotificationAction, NotificationLevel, NotificationPosition,
};
use gpui_kit::{App, Context, IntoElement, Render, Window};

struct BatchPreviewOwner {
    _downloads: Entity<DownloadApplication>,
}
impl gpui_kit::Global for BatchPreviewOwner {}
struct Preview {
    center: Entity<NotificationCenter>,
}
impl Render for Preview {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        crate::theme::observe_theme(window, cx);
        div()
            .size_full()
            .relative()
            .bg(Theme::active(cx).canvas)
            .text_color(Theme::active(cx).text)
            .child(div().px_4().py_3().child("隔离预览 · 窗口内统一通知"))
            .child(crate::notifications::presentation::render(
                self.center.clone(),
                true,
                48.0,
                window,
                cx,
            ))
    }
}
pub(super) fn open(cx: &mut App) -> anyhow::Result<()> {
    let _scratch = std::env::var("PI_SCRATCH_DIR")?;
    crate::ui::components::styled_menu_surface::open_isolated_preview(cx)?;
    crate::network_settings::visual_preview::open_notification_setting(cx)?;
    let notices = cx.new(|_| NotificationCenter::default());
    let scratch = std::path::PathBuf::from(std::env::var("PI_SCRATCH_DIR")?)
        .join(format!("batch-preview-{}", uuid::Uuid::new_v4()));
    let downloads =
        crate::downloads::visual_preview::gallery_owner(&scratch, notices.clone(), true, cx)?;
    let ids = downloads
        .read(cx)
        .tasks()
        .iter()
        .map(|task| task.task.id())
        .collect::<Vec<_>>();
    cx.set_global(BatchPreviewOwner {
        _downloads: downloads.clone(),
    });
    cx.open_window(final_gallery::options(1060.0, 650.0), move |window, cx| {
        let main = cx.new(|cx| {
            let mut main = MainWindow::new(
                gpui_kit::WeakEntity::new_invalid(),
                gpui_kit::WeakEntity::new_invalid(),
                downloads.downgrade(),
                MainWindowShellContext {
                    settings: None,
                    platform: None,
                    system_settings: Rc::new(RefCell::new(SystemSettings::default())),
                },
                notices,
                window,
                cx,
            );
            main.selected_download_tasks = ids.iter().copied().collect();
            main.selected_download_task = ids.first().copied();
            main
        });
        final_gallery::ready("download-batch-centered", 1060.0, 650.0, window);
        cx.new(|cx| gpui_kit::base::Root::new(main, window, cx))
    })?;
    for (index, position) in NotificationPosition::ALL.into_iter().enumerate() {
        let center = cx.new(|_| NotificationCenter::default());
        center.update(cx, |center, cx| {
            center.set_position(position, cx);
            center.push(
                AppNotification::new(
                    format!("position-{index}"),
                    NotificationLevel::Success,
                    "扩展处理完成",
                    "已保存 1 个结果，原文保留",
                )
                .actions(vec![
                    NotificationAction::new("查看结果", |_, _| {}),
                    NotificationAction::new("详情", |_, _| {}),
                ])
                .duration(None),
                cx,
            );
        });
        cx.open_window(final_gallery::options(720.0, 480.0), move |window, cx| {
            let view = cx.new(|_| Preview { center });
            final_gallery::ready(&format!("notification-{index}"), 720.0, 480.0, window);
            cx.new(|cx| gpui_kit::base::Root::new(view, window, cx))
        })?;
    }
    let center = cx.new(|_| NotificationCenter::default());
    center.update(cx, |center, cx| {
        center.push(
            AppNotification::new(
                "narrow",
                NotificationLevel::Error,
                "扩展未能完成",
                "已保存的结果仍保留，请查看详情",
            )
            .actions(vec![
                NotificationAction::new("查看结果", |_, _| {}),
                NotificationAction::new("详情", |_, _| {}),
            ])
            .duration(None),
            cx,
        )
    });
    cx.open_window(final_gallery::options(300.0, 440.0), move |window, cx| {
        let view = cx.new(|_| Preview { center });
        final_gallery::ready("notification-narrow", 300.0, 440.0, window);
        cx.new(|cx| gpui_kit::base::Root::new(view, window, cx))
    })?;
    Ok(())
}
