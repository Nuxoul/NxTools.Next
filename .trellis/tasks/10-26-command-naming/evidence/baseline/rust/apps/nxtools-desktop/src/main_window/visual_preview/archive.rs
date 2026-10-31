//! Existing isolated preview entry, actual production MainWindow and settings.
use super::*;
use nxtools_core::AppConfig;

pub(super) fn open(cx: &mut App) -> anyhow::Result<()> {
    let scratch = PathBuf::from(std::env::var("PI_SCRATCH_DIR")?);
    let fixture = scratch.join(format!("archive-settings-preview-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&fixture)?;
    let path = fixture.join("config.json");
    let config = AppConfig::default();
    config.save_to(&path)?;
    let minimum = std::env::var_os("NXTOOLS_PREVIEW_ARCHIVE_MINIMUM").is_some();
    let dimensions = if minimum {
        (920.0, 480.0)
    } else {
        (1060.0, 650.0)
    };
    let notifications = cx.new(|_| NotificationCenter::default());
    println!("ui.archive_settings.fixture config={}", path.display());
    cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                None,
                size(px(dimensions.0), px(dimensions.1)),
                cx,
            ))),
            titlebar: None,
            window_decorations: Some(WindowDecorations::Client),
            window_min_size: Some(size(px(920.0), px(480.0))),
            app_id: Some("NxTools.ArchiveSettings.Preview".into()),
            ..Default::default()
        },
        move |window, cx| {
            window.set_window_title("NxTools Archive Settings — isolated");
            let view = cx.new(|cx| {
                super::super::archive_page::ArchiveSettingsView::new(
                    config.archive,
                    path,
                    window,
                    cx,
                )
            });
            let main = cx.new(|cx| {
                let mut main = MainWindow::new(
                    gpui_kit::WeakEntity::new_invalid(),
                    gpui_kit::WeakEntity::new_invalid(),
                    gpui_kit::WeakEntity::new_invalid(),
                    MainWindowShellContext {
                        settings: None,
                        platform: None,
                        system_settings: Rc::new(RefCell::new(SystemSettings::default())),
                    },
                    notifications,
                    window,
                    cx,
                );
                main.archive_settings = Some(view);
                main.shell.activate(MainModule::Archive);
                main
            });
            println!(
                "ui.archive_settings.ready hwnd={} scale={}",
                crate::ui::native_window_handle(window).unwrap_or_default(),
                window.scale_factor()
            );
            cx.new(|cx| gpui_kit::base::Root::new(main, window, cx))
        },
    )?;
    Ok(())
}
