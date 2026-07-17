use gpui::{
    AnyWindowHandle, App, AppContext, Context, Global, Render, Window, WindowBackgroundAppearance,
    WindowDecorations, WindowKind, WindowOptions, div, prelude::*,
};

pub struct GpuiLifetimeWindowGlobal {
    _window: AnyWindowHandle,
}

impl Global for GpuiLifetimeWindowGlobal {}

pub fn install_gpui_lifetime_window(cx: &mut App) -> anyhow::Result<()> {
    let window = cx.open_window(gpui_lifetime_window_options(), |_window, cx| {
        cx.new(|_| BackgroundKeepAlive)
    })?;
    cx.set_global(GpuiLifetimeWindowGlobal {
        _window: window.into(),
    });
    Ok(())
}

fn gpui_lifetime_window_options() -> WindowOptions {
    WindowOptions {
        window_bounds: None,
        titlebar: None,
        focus: false,
        show: false,
        kind: WindowKind::PopUp,
        is_movable: false,
        is_resizable: false,
        is_minimizable: false,
        display_id: None,
        window_background: WindowBackgroundAppearance::Transparent,
        app_id: Some("NxTools.Background".into()),
        window_min_size: None,
        window_decorations: Some(WindowDecorations::Client),
        tabbing_identifier: None,
    }
}

struct BackgroundKeepAlive;

impl Render for BackgroundKeepAlive {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().size_full()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gpui_lifetime_window_is_hidden_and_non_interactive() {
        let options = gpui_lifetime_window_options();
        assert!(!options.show);
        assert!(!options.focus);
        assert!(!options.is_movable);
        assert!(!options.is_resizable);
        assert!(!options.is_minimizable);
        assert_eq!(options.kind, WindowKind::PopUp);
    }
}
