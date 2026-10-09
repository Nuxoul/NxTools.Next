//! Debug-only global settings rendering with synthetic rules and no platform owner.
use super::*;
use gpui::{App, WindowBounds, WindowDecorations, WindowOptions};

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
