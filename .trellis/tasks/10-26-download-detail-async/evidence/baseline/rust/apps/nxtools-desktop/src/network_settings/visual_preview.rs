//! Debug-only global settings rendering with synthetic rules and no platform owner.
use super::*;
use gpui_kit::{App, WindowBounds, WindowDecorations, WindowOptions};

pub(crate) fn open(cx: &mut App) -> anyhow::Result<()> {
    let scratch = PathBuf::from(
        std::env::var_os("PI_SCRATCH_DIR").ok_or_else(|| anyhow::anyhow!("scratch required"))?,
    );
    let mut config = AppConfig::default();
    config.set_process_blacklist_settings(ProcessBlacklistSettings::from_entries(vec![
        ProcessBlacklistRule::new("Editor.exe", true, false).map_err(anyhow::Error::msg)?,
        ProcessBlacklistRule::new("Game.exe", false, true).map_err(anyhow::Error::msg)?,
        ProcessBlacklistRule::new("PasswordManager.exe", true, true).map_err(anyhow::Error::msg)?,
    ]));
    let notifications = cx.new(|_| NotificationCenter::default());
    let owner_config = config.clone();
    let application = cx.new(|_| {
        NetworkSettingsApplication::new(
            NetworkPolicy::new(owner_config.network.clone()).expect("synthetic policy"),
            owner_config,
            scratch.join("global-preview-unused-config.json"),
            notifications,
            None,
            Rc::new(RefCell::new(SystemSettings::default())),
            false,
        )
    });
    cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::centered(
                size(
                    px(GLOBAL_SETTINGS_WINDOW_WIDTH),
                    px(GLOBAL_SETTINGS_WINDOW_HEIGHT),
                ),
                cx,
            )),
            titlebar: None,
            window_decorations: Some(WindowDecorations::Client),
            is_resizable: false,
            ..Default::default()
        },
        move |window, cx| {
            let view = cx.new(|cx| {
                let mut view = NetworkSettingsView::new(
                    application,
                    config.network,
                    config.system,
                    config.process_blacklist,
                    Rc::new(std::cell::Cell::new(false)),
                    window,
                    cx,
                );
                view.tab = GlobalSettingsTab::Blacklist;
                view
            });
            println!(
                "ui.preview.blacklist_ready hwnd={} scale={}",
                crate::ui::native_window_handle(window).unwrap_or_default(),
                window.scale_factor()
            );
            cx.new(|cx| Root::new(view, window, cx))
        },
    )?;
    Ok(())
}

pub(crate) fn open_gallery(cx: &mut App) -> anyhow::Result<()> {
    use crate::main_window::visual_preview::final_gallery::{options, ready};
    let scratch = PathBuf::from(std::env::var("PI_SCRATCH_DIR")?);
    for (mode, tab, busy, error) in [
        ("global-general", GlobalSettingsTab::General, false, false),
        ("global-network", GlobalSettingsTab::Network, false, false),
        ("global-rules", GlobalSettingsTab::Blacklist, false, false),
        ("global-error", GlobalSettingsTab::Blacklist, false, true),
        ("global-busy", GlobalSettingsTab::Blacklist, true, false),
    ] {
        let mut config = AppConfig::default();
        config.set_process_blacklist_settings(ProcessBlacklistSettings::from_entries(
            (0..12)
                .map(|index| {
                    ProcessBlacklistRule::new(
                        &format!("合成长名称进程{index}.exe"),
                        true,
                        index % 2 == 0,
                    )
                    .map_err(anyhow::Error::msg)
                })
                .collect::<anyhow::Result<Vec<_>>>()?,
        ));
        let notifications = cx.new(|_| NotificationCenter::default());
        let saved = config.clone();
        let path = scratch.join("unused-gallery-global.json");
        let application = cx.new(|_| {
            NetworkSettingsApplication::new(
                NetworkPolicy::new(saved.network.clone()).expect("synthetic policy"),
                saved,
                path,
                notifications,
                None,
                Rc::new(RefCell::new(SystemSettings::default())),
                false,
            )
        });
        cx.open_window(options(720.0, 480.0), move |window, cx| {
            let view = cx.new(|cx| {
                let mut view = NetworkSettingsView::new(
                    application,
                    config.network,
                    config.system,
                    config.process_blacklist,
                    Rc::new(std::cell::Cell::new(busy)),
                    window,
                    cx,
                );
                view.tab = tab;
                view.saving = busy;
                if error {
                    view.error = Some("合成应用失败，保留草稿与重试。".repeat(30));
                }
                view
            });
            ready(mode, 720.0, 480.0, window);
            cx.new(|cx| Root::new(view, window, cx))
        })?;
    }
    Ok(())
}

pub(crate) fn open_notification_setting(cx: &mut App) -> anyhow::Result<()> {
    use crate::main_window::visual_preview::final_gallery::{options, ready};
    let scratch = PathBuf::from(std::env::var("PI_SCRATCH_DIR")?);
    let config = AppConfig::default();
    let saved = config.clone();
    let center = cx.new(|_| NotificationCenter::default());
    let application = cx.new(|_| {
        NetworkSettingsApplication::new(
            NetworkPolicy::new(saved.network.clone()).expect("synthetic policy"),
            saved,
            scratch.join("unused-notification-setting-config.json"),
            center,
            None,
            Rc::new(RefCell::new(SystemSettings::default())),
            false,
        )
    });
    cx.open_window(options(720.0, 480.0), move |window, cx| {
        let view = cx.new(|cx| {
            NetworkSettingsView::new(
                application,
                config.network,
                config.system,
                config.process_blacklist,
                Rc::new(std::cell::Cell::new(false)),
                window,
                cx,
            )
        });
        ready("global-notification-position", 720.0, 480.0, window);
        cx.new(|cx| Root::new(view, window, cx))
    })?;
    Ok(())
}
