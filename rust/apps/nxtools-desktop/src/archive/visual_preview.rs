//! Isolated native workflow preview: original views, synthetic paths, no application/runtime.
use super::{draft::DraftSource, forms::OperationForm};
use gpui_kit::{
    App, AppContext, Bounds, WeakEntity, WindowBounds, WindowDecorations, WindowOptions, px, size,
};
use nxtools_core::archive::ArchiveSettings;

pub(crate) fn open(cx: &mut App) -> anyhow::Result<()> {
    let scratch = std::path::PathBuf::from(std::env::var("PI_SCRATCH_DIR")?).canonicalize()?;
    anyhow::ensure!(scratch.is_dir(), "preview scratch required");
    let mode = std::env::var("NXTOOLS_PREVIEW_ARCHIVE_FLOW").unwrap_or_else(|_| "extract".into());
    let minimum = std::env::var_os("NXTOOLS_PREVIEW_ARCHIVE_MINIMUM").is_some();
    if mode == "progress" || mode == "progress-unknown" {
        return super::progress::open_visual_preview(mode == "progress-unknown", minimum, cx);
    }
    let (width, height) = if minimum {
        (480., 360.)
    } else if mode == "create" {
        (680., 560.)
    } else {
        (560., 600.)
    };
    cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                None,
                size(px(width), px(height)),
                cx,
            ))),
            titlebar: None,
            window_decorations: Some(WindowDecorations::Client),
            window_min_size: Some(size(px(480.), px(360.))),
            app_id: Some("NxTools.ArchiveFlow.Preview".into()),
            ..Default::default()
        },
        move |window, cx| {
            window.set_window_title("NxTools Archive Flow — isolated");
            let form = cx.new(|cx| {
                let settings = ArchiveSettings::default();
                match mode.as_str() {
                    "create" => OperationForm::create(
                        settings,
                        vec![
                            DraftSource {
                                path: r"C:\项目\设计资料\发布文件\界面设计说明和交付清单.txt"
                                    .into(),
                                directory: false,
                                bytes: Some(128_000),
                            },
                            DraftSource {
                                path: r"C:\项目\设计资料\图片与参考".into(),
                                directory: true,
                                bytes: None,
                            },
                        ],
                        WeakEntity::new_invalid(),
                        window,
                        cx,
                    ),
                    "empty" => OperationForm::extract(
                        settings,
                        Default::default(),
                        None,
                        WeakEntity::new_invalid(),
                        window,
                        cx,
                    ),
                    "continue" | "continue-found" | "continue-pending" | "continue-missing"
                    | "continue-completed" | "continue-error" => {
                        let destination =
                            (!matches!(mode.as_str(), "continue-pending" | "continue-missing"))
                                .then(|| {
                                    r"C:\项目\发布资料\经过多层目录的原始输出文件夹\最终交付目标"
                                        .into()
                                });
                        let mut form = OperationForm::continue_reclamation(
                            settings,
                            r"C:\项目\发布资料\需要继续处理的超长文件名原压缩包.zip".into(),
                            destination,
                            WeakEntity::new_invalid(),
                            window,
                            cx,
                        );
                        form.preview_recovery_state(&mode, window, cx);
                        form
                    }
                    _ => OperationForm::extract(
                        settings,
                        r"C:\项目\设计资料\发布文件\需要交付的完整项目资料.zip".into(),
                        None,
                        WeakEntity::new_invalid(),
                        window,
                        cx,
                    ),
                }
            });
            println!(
                "ui.archive_flow.ready mode={mode} hwnd={} scale={} runtime=false platform=false",
                crate::ui::native_window_handle(window).unwrap_or_default(),
                window.scale_factor()
            );
            cx.new(|cx| gpui_kit::base::Root::new(form, window, cx))
        },
    )?;
    Ok(())
}
