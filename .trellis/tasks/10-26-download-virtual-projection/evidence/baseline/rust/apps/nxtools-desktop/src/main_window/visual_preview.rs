//! Debug-only native preview of production views, with no product runtime.
use super::*;
use crate::notifications::NotificationAction;
use gpui_kit::{Bounds, WindowBounds, WindowDecorations, WindowOptions, size};

mod archive;
mod extensions;
pub(crate) mod final_gallery;
mod notification_positions;
mod quick_settings;

pub(crate) fn requested(arguments: &[String]) -> anyhow::Result<bool> {
    let Some(index) = arguments.iter().position(|value| value == "--ui-preview") else {
        return Ok(false);
    };
    anyhow::ensure!(
        index == 1 && arguments.len() == 3 && arguments[2] == "foundation",
        "UI preview requires exactly --ui-preview foundation"
    );
    Ok(true)
}

pub(crate) fn run() {
    if std::env::var_os("NXTOOLS_PREVIEW_FINAL_GALLERY").is_some()
        || std::env::var_os("NXTOOLS_PREVIEW_EXTENSIONS").is_some()
    {
        let fixture = std::thread::Builder::new()
            .name("isolated-final-gallery".into())
            .stack_size(32 * 1024 * 1024)
            .spawn(run_isolated);
        match fixture {
            Ok(fixture) => {
                let _ = fixture.join();
            }
            Err(_) => eprintln!("ui.preview.failed stage=fixture_thread"),
        }
    } else {
        run_isolated();
    }
}

fn run_isolated() {
    println!("ui.preview.start runtime=false capture=false indexer=false platform=false");
    gpui_kit::application()
        .with_assets(crate::assets::NxToolsAssets)
        .run(|cx| {
            gpui_kit::init(cx);
            crate::theme::apply_component_theme(cx);
            match std::env::var("NXTOOLS_PREVIEW_THEME").as_deref() {
                Ok("light") => {
                    crate::theme::set_theme_preference(nxtools_core::ThemePreference::Light, cx)
                }
                Ok("dark") => {
                    crate::theme::set_theme_preference(nxtools_core::ThemePreference::Dark, cx)
                }
                Ok(_) => {
                    eprintln!("ui.preview.failed stage=theme_selection");
                    cx.quit();
                    return;
                }
                Err(_) => {}
            }
            if std::env::var_os("NXTOOLS_PREVIEW_ARCHIVE_SETTINGS").is_some() {
                if archive::open(cx).is_err() {
                    eprintln!("ui.preview.failed stage=archive_settings");
                    cx.quit();
                }
                return;
            }
            if std::env::var_os("NXTOOLS_PREVIEW_MENU_DIR").is_some() {
                if crate::ui::components::native_context_menu::open_isolated_preview(cx).is_err() {
                    eprintln!("ui.preview.failed stage=menu_lifecycle");
                    cx.quit();
                }
                return;
            }
            if std::env::var_os("NXTOOLS_PREVIEW_SCREEN_NOTIFICATION_DIR").is_some() {
                if crate::notifications::screen::preview::open(cx).is_err() {
                    eprintln!("ui.preview.failed stage=screen_notification");
                    cx.quit();
                }
                return;
            }
            if std::env::var_os("NXTOOLS_PREVIEW_NOTIFICATION_POSITIONS").is_some() {
                if notification_positions::open(cx).is_err() {
                    eprintln!("ui.preview.failed stage=notification_positions");
                    cx.quit();
                }
                return;
            }
            if std::env::var_os("NXTOOLS_PREVIEW_EXTENSIONS").is_some() {
                if extensions::open(cx).is_err() {
                    eprintln!("ui.preview.failed stage=extensions_fixture");
                    cx.quit();
                }
                return;
            }
            if std::env::var_os("NXTOOLS_PREVIEW_FINAL_GALLERY").is_some() {
                if final_gallery::open(cx).is_err() {
                    eprintln!("ui.preview.failed stage=final_gallery_fixture");
                    cx.quit();
                }
                return;
            }
            if std::env::var_os("NXTOOLS_PREVIEW_QUICK_SETTINGS").is_some() {
                if quick_settings::open(cx).is_err() {
                    eprintln!("ui.preview.failed stage=quick_settings_fixture");
                    cx.quit();
                }
                return;
            }
            if std::env::var_os("NXTOOLS_PREVIEW_QUICK_LAYOUT").is_some() {
                if crate::quick_launch::visual_preview::open(cx).is_err() {
                    cx.quit();
                }
                return;
            }
            if std::env::var_os("NXTOOLS_PREVIEW_NATIVE_ACCEPTANCE").is_some() {
                if crate::ui::native_acceptance::open(cx).is_err() {
                    eprintln!("ui.preview.failed stage=native_acceptance_fixture");
                    cx.quit();
                }
                return;
            }
            if std::env::var_os("NXTOOLS_PREVIEW_GLOBAL_BLACKLIST").is_some() {
                if crate::network_settings::visual_preview::open(cx).is_err() {
                    cx.quit();
                }
                return;
            }
            if std::env::var_os("NXTOOLS_PREVIEW_COMPLETION_CENTER").is_some() {
                if crate::downloads::completion_transition_preview::open(cx).is_err() {
                    cx.quit();
                }
                return;
            }
            if std::env::var_os("NXTOOLS_PREVIEW_PROBE_TRANSITION").is_some() {
                if crate::downloads::fixed_transition_preview::open(cx).is_err() {
                    cx.quit();
                }
                return;
            }
            let notifications = cx.new(|_| NotificationCenter::default());
            if std::env::var_os("NXTOOLS_PREVIEW_LONG_NOTIFICATION").is_some() {
                notifications.update(cx, |center, cx| {
                    center.show_success_with_details(
                        "layout-preview",
                        "合成通知：保留完整说明与操作",
                        "这是一段仅用于布局检查的长说明。".repeat(12),
                        Vec::new(),
                        Some(NotificationAction::new("合成操作", |_, _| {})),
                        cx,
                    );
                });
            }
            let opened = cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                        None,
                        size(px(1060.0), px(650.0)),
                        cx,
                    ))),
                    titlebar: None,
                    window_decorations: Some(WindowDecorations::Client),
                    window_min_size: Some(size(px(920.0), px(480.0))),
                    app_id: Some("NxTools.UI.Preview".into()),
                    ..Default::default()
                },
                move |window, cx| {
                    window.set_window_title("NxTools UI Preview — isolated");
                    let main = cx.new(|cx| {
                        let mut main = MainWindow::new(
                            gpui_kit::WeakEntity::<ClipboardApplication>::new_invalid(),
                            gpui_kit::WeakEntity::<QuickLaunchApplication>::new_invalid(),
                            gpui_kit::WeakEntity::<DownloadApplication>::new_invalid(),
                            MainWindowShellContext {
                                settings: None,
                                platform: None,
                                system_settings: Rc::new(RefCell::new(SystemSettings::default())),
                            },
                            notifications,
                            window,
                            cx,
                        );
                        main.shell.activate(MainModule::Tools);
                        main
                    });
                    let last_module = std::cell::Cell::new(MainModule::Tools);
                    cx.observe(&main, move |main, cx| {
                        let module = main.read(cx).shell.active_module();
                        if last_module.replace(module) != module {
                            println!("ui.preview.module selected={module:?}");
                        }
                    })
                    .detach();
                    println!(
                        "ui.preview.ready hwnd={} scale={}",
                        crate::ui::native_window_handle(window).unwrap_or_default(),
                        window.scale_factor()
                    );
                    cx.new(|cx| gpui_kit::base::Root::new(main, window, cx))
                },
            );
            if opened.is_err() {
                eprintln!("ui.preview.failed stage=open_window");
                cx.quit();
                return;
            }
            if crate::downloads::visual_preview::open_confirmation(cx).is_err() {
                eprintln!("ui.preview.failed stage=download_fixture");
                cx.quit();
            }
        });
    println!("ui.preview.stop");
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_ui_preview_is_explicit_and_rejects_mixed_runtime_arguments() {
        let args = |values: &[&str]| {
            values
                .iter()
                .map(|value| (*value).to_owned())
                .collect::<Vec<_>>()
        };
        assert!(!requested(&args(&["NxTools.exe"])).expect("ordinary startup"));
        assert!(
            requested(&args(&["NxTools.exe", "--ui-preview", "foundation"]))
                .expect("explicit preview")
        );
        for values in [
            vec!["NxTools.exe", "--ui-preview"],
            vec!["NxTools.exe", "--ui-preview", "unknown"],
            vec!["NxTools.exe", "--ui-preview", "foundation", "--background"],
        ] {
            assert!(
                requested(&args(&values)).is_err(),
                "invalid preview must not fall through to runtime"
            );
        }
    }
}
