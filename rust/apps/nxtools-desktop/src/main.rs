mod runtime;
mod theme;
mod ui;

use anyhow::{Context as _, Result};
use gpui::{App, AppContext, Application};
use gpui_component::{Theme, ThemeMode};
use runtime::DesktopRuntime;
use tracing::info;
use ui::{ClipboardController, ClipboardControllerGlobal};

fn run() -> Result<()> {
    let mut runtime = DesktopRuntime::initialize()?;
    let platform_bridge = runtime
        .platform_bridge
        .take()
        .context("desktop runtime did not provide a platform bridge")?;
    let clipboard_history = runtime
        .clipboard_history
        .take()
        .context("desktop runtime did not provide clipboard history")?;
    info!(pipe_name = %runtime.config.platform_pipe_name, "starting GPUI application");

    Application::new().run(move |cx: &mut App| {
        gpui_component::init(cx);
        Theme::change(ThemeMode::Dark, None, cx);

        let controller =
            cx.new(|cx| ClipboardController::new(clipboard_history, platform_bridge, cx));
        cx.set_global(ClipboardControllerGlobal::new(controller));
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
