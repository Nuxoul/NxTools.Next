mod clipboard;
mod lifecycle;
mod runtime;
mod theme;
mod ui;

use std::{cell::RefCell, rc::Rc};

use anyhow::{Context as _, Result};
use clipboard::{ClipboardApplication, ClipboardApplicationGlobal};
use gpui::{App, AppContext, Application};
use gpui_component::{Theme, ThemeMode};
use runtime::DesktopRuntime;
use tracing::{error, info};

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

    let startup_error = Rc::new(RefCell::new(None));
    let startup_error_from_app = Rc::clone(&startup_error);
    Application::new().run(move |cx: &mut App| {
        gpui_component::init(cx);
        Theme::change(ThemeMode::Dark, None, cx);

        if let Err(error) = lifecycle::install_gpui_lifetime_window(cx) {
            error!(%error, "failed to create hidden background lifetime window");
            *startup_error_from_app.borrow_mut() = Some(error);
            cx.quit();
            return;
        }

        let clipboard =
            cx.new(|cx| ClipboardApplication::new(clipboard_history, platform_bridge, cx));
        cx.set_global(ClipboardApplicationGlobal::new(clipboard));
    });

    drop(runtime);
    let startup_error = startup_error.borrow_mut().take();
    if let Some(error) = startup_error {
        return Err(error).context("failed to create GPUI background lifetime window");
    }
    Ok(())
}

fn main() {
    if let Err(error) = run().context("desktop startup failed") {
        eprintln!("NxTools could not start: {error:#}");
        std::process::exit(1);
    }
}
