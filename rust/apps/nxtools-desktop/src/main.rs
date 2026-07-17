mod assets;
mod runtime;
mod theme;
mod ui;

use anyhow::{Context as _, Result};
use assets::Assets;
use gpui::{
    App, AppContext, Application, Bounds, WindowBackgroundAppearance, WindowBounds,
    WindowDecorations, WindowOptions, px, size,
};
use gpui_component::{Root, Theme, ThemeMode};
use runtime::DesktopRuntime;
use tracing::info;
use ui::DesktopShell;

fn run() -> Result<()> {
    let runtime = DesktopRuntime::initialize()?;
    info!(pipe_name = %runtime.config.platform_pipe_name, "starting GPUI application");

    Application::new().with_assets(Assets).run(|cx: &mut App| {
        gpui_component::init(cx);
        Theme::change(ThemeMode::Dark, None, cx);

        let bounds = Bounds::centered(None, size(px(1040.0), px(680.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                window_background: WindowBackgroundAppearance::Opaque,
                window_decorations: Some(WindowDecorations::Client),
                is_movable: true,
                ..Default::default()
            },
            |window, cx| {
                let shell = cx.new(|cx| DesktopShell::new(window, cx));
                cx.new(|cx| Root::new(shell, window, cx))
            },
        )
        .expect("failed to open GPUI window");
        cx.activate(true);
    });

    drop(runtime);
    Ok(())
}

fn main() {
    if let Err(error) = run().context("desktop startup failed") {
        eprintln!("NxTools could not start: {error:#}");
        std::process::exit(1);
    }
}
