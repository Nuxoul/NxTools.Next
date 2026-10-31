//! Synthetic production settings views; scratch-only data and disconnected bridges.
use super::*;
use nxtools_core::{AppConfig, CustomCommandSettings};
use nxtools_finder::FinderCatalog;

struct FixtureOwners {
    _applications: Vec<Entity<QuickLaunchApplication>>,
}
impl gpui_kit::Global for FixtureOwners {}

pub(super) fn open(cx: &mut App) -> anyhow::Result<()> {
    let scratch = std::path::PathBuf::from(std::env::var("PI_SCRATCH_DIR")?);
    let fixture = scratch.join(format!("quick-settings-preview-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&fixture)?;
    let mut applications = Vec::new();
    for (mode, page, dimensions) in [
        ("general", QuickLaunchSettingsPage::General, (1060.0, 650.0)),
        (
            "commands",
            QuickLaunchSettingsPage::Commands,
            (1060.0, 650.0),
        ),
        ("context", QuickLaunchSettingsPage::Context, (1060.0, 650.0)),
        ("minimum", QuickLaunchSettingsPage::Commands, (920.0, 480.0)),
        ("index", QuickLaunchSettingsPage::Index, (1060.0, 650.0)),
        (
            "diagnostics",
            QuickLaunchSettingsPage::Diagnostics,
            (1060.0, 650.0),
        ),
    ] {
        let catalog = FinderCatalog::open(fixture.join(format!("{mode}-history.db")), &[])?;
        let (platform, endpoint) = nxtools_ipc::platform_bridge(8);
        drop(endpoint);
        let (indexer, endpoint) = crate::runtime::indexer_bridge(8);
        drop(endpoint);
        let notifications = cx.new(|_| NotificationCenter::default());
        let mut config = AppConfig::default();
        config.quick_launch.excluded_paths = vec![
            r"C:\synthetic\excluded".into(),
            r"D:\synthetic\temporary".into(),
        ];
        config.quick_launch.custom_commands = vec![
            CustomCommandSettings {
                enabled: true,
                keyword: "docs".into(),
                title: "合成文档命令（不执行）".into(),
                program: r"C:\synthetic\tools\docs.exe".into(),
                argument_template: "--query %s".into(),
                working_directory: r"C:\synthetic\documents".into(),
                ..Default::default()
            },
            CustomCommandSettings {
                keyword: "build".into(),
                title: "较长的中文合成命令名称，用于检查主从编辑器的裁剪".into(),
                program: r"D:\synthetic\tools\build.exe".into(),
                ..Default::default()
            },
        ];
        let application = cx.new(|_| {
            QuickLaunchApplication::new(
                catalog,
                Rc::new(platform),
                Rc::new(indexer),
                config,
                fixture.join(format!("{mode}-config.json")),
                fixture.join("logs"),
                notifications.clone(),
            )
        });
        let owner = application.downgrade();
        applications.push(application);
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
                app_id: Some(format!("NxTools.QuickSettings.Preview.{mode}")),
                ..Default::default()
            },
            move |window, cx| {
                window.set_window_title(&format!(
                    "NxTools Quick Settings Preview — synthetic {mode}"
                ));
                let selected = owner
                    .upgrade()
                    .map(|application| application.read(cx).settings().custom_commands[0].clone());
                let main = cx.new(|cx| {
                    let mut main = MainWindow::new(
                        gpui_kit::WeakEntity::new_invalid(),
                        owner,
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
                    main.shell.activate(MainModule::QuickFind);
                    main.quick_launch_settings_page = page;
                    if page == QuickLaunchSettingsPage::Commands
                        && let Some(command) = selected
                    {
                        main.select_custom_command_draft(0, command, window.window_handle(), cx);
                    }
                    main
                });
                println!(
                    "ui.quick_settings.ready mode={mode} hwnd={} scale={}",
                    crate::ui::native_window_handle(window).unwrap_or_default(),
                    window.scale_factor()
                );
                cx.new(|cx| gpui_kit::base::Root::new(main, window, cx))
            },
        )?;
    }
    cx.set_global(FixtureOwners {
        _applications: applications,
    });
    Ok(())
}
