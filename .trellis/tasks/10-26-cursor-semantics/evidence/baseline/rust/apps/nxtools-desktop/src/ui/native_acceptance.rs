//! Debug-only native acceptance fixture. No runtime, user storage, IPC or Windows API owner.
use super::*;
use crate::quick_launch::preview::{
    QuickLaunchPreview, QuickLaunchPreviewBody, QuickLaunchPreviewPayload,
};
use gpui_kit::base::Root;
use gpui_kit::component::input::Input;
use gpui_kit::{WindowBounds, WindowDecorations, WindowKind, WindowOptions};

struct Primary {
    input: Entity<InputState>,
}
impl Render for Primary {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .bg(Theme::zed_dark().canvas)
            .text_color(Theme::zed_dark().text)
            .p(px(12.0))
            .flex()
            .flex_col()
            .gap(px(8.0))
            .child(Input::new(&self.input).aria_label("Synthetic search input"))
            .child(
                components::action::action_button("acceptance-action", "合成验收动作")
                    .accessibility_label("Synthetic acceptance action")
                    .on_click(|_, _, _| {}),
            )
    }
}
pub(crate) fn open(cx: &mut App) -> anyhow::Result<()> {
    let mode = std::env::var("NXTOOLS_PREVIEW_NATIVE_ACCEPTANCE")?;
    anyhow::ensure!(
        mode == "quick" || mode == "clipboard",
        "explicit synthetic preview mode required"
    );
    let primary_bounds = Bounds::new(point(px(80.0), px(100.0)), size(px(620.0), px(180.0)));
    cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(primary_bounds)),
            titlebar: None,
            window_decorations: Some(WindowDecorations::Client),
            app_id: Some("NxTools.NativeAcceptance.Primary".into()),
            ..Default::default()
        },
        |window, cx| {
            window.set_window_title("NxTools Native Acceptance — synthetic primary");
            let view = cx.new(|cx| {
                let input =
                    cx.new(|cx| InputState::new(window, cx).default_value("Synthetic query"));
                input.focus_handle(cx).focus(window, cx);
                Primary { input }
            });
            println!(
                "ui.acceptance.primary hwnd={} scale={}",
                native_window_handle(window).unwrap_or_default(),
                window.scale_factor()
            );
            cx.new(|cx| Root::new(view, window, cx))
        },
    )?;
    let text = "仅用于隔离原生验收的合成内容。\n".repeat(18);
    let item = ClipboardItem {
        id: 1,
        kind: ClipboardKind::Text,
        text,
        image_path: String::new(),
        image_width: 0,
        image_height: 0,
        file_paths: Vec::new(),
        source: nxtools_clipboard::CaptureSource {
            window_handle: 0,
            process_id: 0,
            process_name: String::new(),
            process_icon_path: String::new(),
            window_title: String::new(),
        },
        captured_at_unix_ms: 1,
        last_used_at_unix_ms: 1,
        copy_count: 1,
        is_favorite: false,
        is_pinned: false,
        deleted_at_unix_ms: None,
    };
    let layout = preview::preview_layout(&item, 1920.0, 1080.0);
    let child_size = if mode == "quick" {
        size(px(400.0), px(410.0))
    } else {
        size(px(layout.window_width), px(layout.window_height))
    };
    cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                point(px(750.0), px(100.0)),
                child_size,
            ))),
            titlebar: None,
            show: false,
            focus: false,
            kind: WindowKind::PopUp,
            is_movable: false,
            is_resizable: false,
            is_minimizable: false,
            window_decorations: Some(WindowDecorations::Client),
            app_id: Some("NxTools.NativeAcceptance.Passive".into()),
            ..Default::default()
        },
        move |window, cx| {
            window.set_window_title("NxTools Native Acceptance — synthetic passive");
            println!(
                "ui.acceptance.passive hwnd={} scale={} width={} height={} hidden=true",
                native_window_handle(window).unwrap_or_default(),
                window.scale_factor(),
                child_size.width.as_f32(),
                child_size.height.as_f32()
            );
            if mode == "quick" {
                let view = cx.new(|_| {
                    QuickLaunchPreview::new(
                        QuickLaunchPreviewPayload {
                            entry_id: "synthetic".into(),
                            name: "合成预览.txt".into(),
                            target: "Synthetic preview target".into(),
                            detail: "合成原生验收".into(),
                            created_at: None,
                            modified_at: None,
                            body: QuickLaunchPreviewBody::Text {
                                text: item.text,
                                truncated: false,
                            },
                        },
                        WeakEntity::new_invalid(),
                    )
                });
                cx.new(|cx| Root::new(view, window, cx))
            } else {
                let view = cx.new(|cx| {
                    preview::ClipboardPreview::new(
                        item,
                        layout,
                        WeakEntity::new_invalid(),
                        window,
                        cx,
                    )
                });
                cx.new(|cx| Root::new(view, window, cx))
            }
        },
    )?;
    Ok(())
}
