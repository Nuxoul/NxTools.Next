//! Debug-only queue12 production rendering. No ordinary runtime or connected transport.
use super::*;
use crate::clipboard_worker::ClipboardCaptureWorkerHandle;
use nxtools_core::AppConfig;
use std::path::PathBuf;

struct GalleryOwners {
    _downloads: Entity<DownloadApplication>,
    _clipboard: Entity<ClipboardApplication>,
    worker: ClipboardCaptureWorkerHandle,
}
impl gpui_kit::Global for GalleryOwners {}
impl Drop for GalleryOwners {
    fn drop(&mut self) {
        let _ = self.worker.clone().shutdown();
    }
}

pub(crate) fn options(width: f32, height: f32) -> WindowOptions {
    WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds::new(
            gpui_kit::point(px(60.0), px(60.0)),
            size(px(width), px(height)),
        ))),
        titlebar: None,
        window_decorations: Some(WindowDecorations::Client),
        ..Default::default()
    }
}
pub(crate) fn ready(mode: &str, width: f32, height: f32, window: &mut Window) {
    window.set_window_title(&format!("NxTools Final Gallery — synthetic {mode}"));
    println!(
        "ui.final.ready mode={mode} hwnd={} scale={} width={width} height={height}",
        crate::ui::native_window_handle(window).unwrap_or_default(),
        window.scale_factor()
    );
}
pub(super) fn open(cx: &mut App) -> anyhow::Result<()> {
    let group = std::env::var("NXTOOLS_PREVIEW_FINAL_GALLERY")?;
    anyhow::ensure!(
        ["downloads", "clipboard", "global", "tools"].contains(&group.as_str()),
        "explicit gallery group required"
    );
    let scratch = PathBuf::from(std::env::var("PI_SCRATCH_DIR")?)
        .join(format!("final-gallery-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&scratch)?;
    let notifications = cx.new(|_| NotificationCenter::default());
    let downloads = crate::downloads::visual_preview::gallery_owner(
        &scratch,
        notifications.clone(),
        group == "downloads",
        cx,
    )?;
    let worker = ClipboardCaptureWorkerHandle::spawn(
        scratch.join("clipboard.db"),
        scratch.join("images"),
        nxtools_clipboard::ClipboardRetentionPolicy {
            max_items: 1000,
            retention_days: 30,
        },
        1000,
    )?;
    let (bridge, endpoint) = nxtools_ipc::platform_bridge(8);
    drop(endpoint);
    let clipboard = cx.new(|cx| {
        ClipboardApplication::new(
            worker.clone(),
            Rc::new(bridge),
            AppConfig::default(),
            scratch.join("unused-config.json"),
            notifications.clone(),
            cx,
        )
    });
    if group == "global" {
        crate::network_settings::visual_preview::open_gallery(cx)?;
    } else if group == "clipboard" {
        crate::ui::final_gallery::open(worker.clone(), cx)?;
    } else if group == "downloads" {
        crate::downloads::visual_preview::open_gallery(downloads.clone(), cx)?;
    }
    if group != "global" {
        for (mode, width, height) in [("default", 1060.0, 650.0), ("minimum", 920.0, 480.0)] {
            let clipboard = clipboard.clone();
            let downloads = downloads.clone();
            let notices = notifications.clone();
            let group = group.clone();
            cx.open_window(options(width, height), move |window, cx| {
                let main = cx.new(|cx| {
                    let mut main = MainWindow::new(
                        clipboard.downgrade(),
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
                    main.shell.activate(match group.as_str() {
                        "clipboard" => MainModule::Clipboard,
                        "tools" => MainModule::Tools,
                        _ => MainModule::Downloads,
                    });
                    if group == "downloads" {
                        main.selected_download_task = downloads
                            .read(cx)
                            .tasks()
                            .first()
                            .map(|task| task.task.id());
                    }
                    main
                });
                let prefix = if group == "clipboard" {
                    "clipboard-settings"
                } else {
                    &group
                };
                ready(&format!("{prefix}-{mode}"), width, height, window);
                cx.new(|cx| gpui_kit::base::Root::new(main, window, cx))
            })?;
        }
    }
    if group == "tools" {
        notifications.update(cx, |center, cx| {
            center.show_progress(
                "gallery-loading",
                "合成加载中",
                "保持加载文案，不代表百分比",
                cx,
            );
            center.show_error(
                "gallery-error",
                "合成错误",
                "长错误说明，仅用于截图。".repeat(12),
                cx,
            );
            center.show_success_with_details(
                "gallery-success",
                "合成完成",
                "原通知管理器与操作仍保留",
                Vec::new(),
                Some(NotificationAction::new("合成操作", |_, _| {})),
                cx,
            );
        });
        for (mode, expanded) in [("link-collapsed", false), ("link-expanded", true)] {
            use crate::tools::link_panel::{LinkPanelView, link_panel_height};
            let height = link_panel_height(3, expanded);
            cx.open_window(options(420.0, height), move |window, cx| {
                let view = cx.new(|_| LinkPanelView {
                    links: (0..3)
                        .map(|index| crate::tools::link_detector::DetectedLink {
                            display: format!("合成链接{index} · 很长的标题用于检查裁剪"),
                            copy_value: format!("https://synthetic.invalid/{index}"),
                            kind: crate::tools::link_detector::DetectedLinkKind::Web,
                        })
                        .collect(),
                    application: gpui_kit::WeakEntity::new_invalid(),
                    expanded,
                    auto_hide_ms: 60000,
                    window_handle: None,
                    auto_hide_task: None,
                });
                ready(mode, 420.0, height, window);
                cx.new(|cx| gpui_kit::base::Root::new(view, window, cx))
            })?;
        }
    }
    cx.set_global(GalleryOwners {
        _downloads: downloads,
        _clipboard: clipboard,
        worker,
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn queue12_gallery_fixture_has_explicit_scratch_disconnected_boundary() {
        let source = include_str!("final_gallery.rs");
        assert!(
            source.contains("NXTOOLS_PREVIEW_FINAL_GALLERY") && source.contains("PI_SCRATCH_DIR")
        );
        assert!(
            source.contains("scratch.join(\"clipboard.db\")") && source.contains("drop(endpoint)")
        );
        let entry = include_str!("../visual_preview.rs");
        assert!(entry.contains("final_gallery::open(cx)") && entry.contains("return;"));
        let fixture = include_str!("../../downloads/visual_preview.rs");
        assert!(
            fixture.contains("DownloadEngine::open_in_memory()")
                && fixture.contains("RequestOrigin::Internal")
        );
    }
}
