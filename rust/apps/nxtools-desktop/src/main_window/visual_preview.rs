//! Debug-only native preview of production views, with no product runtime.
use super::*;
use gpui::{Application, Bounds, WindowBounds, WindowDecorations, WindowOptions, size};

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
    println!("ui.preview.start runtime=false capture=false indexer=false platform=false");
    Application::new()
        .with_assets(crate::assets::NxToolsAssets)
        .run(|cx| {
            gpui_component::init(cx);
            crate::theme::apply_component_theme(cx);
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
                            gpui::WeakEntity::<ClipboardApplication>::new_invalid(),
                            gpui::WeakEntity::<QuickLaunchApplication>::new_invalid(),
                            gpui::WeakEntity::<DownloadApplication>::new_invalid(),
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
                    cx.new(|cx| gpui_component::Root::new(main, window, cx))
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
