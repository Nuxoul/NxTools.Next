//! Opt-in hidden Windows rendering of the production settings view.
use super::*;
use gpui_kit::{AppContext, Bounds, point};
use std::cell::RefCell;

#[test]
#[ignore = "explicit owned hidden Windows GPU fixture"]
fn default_solid_settings_navigation_has_native_text_pixels() {
    let scratch = PathBuf::from(std::env::var_os("PI_SCRATCH_DIR").expect("session scratch"))
        .join(format!("settings-native-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir(&scratch).expect("owned native fixture directory");
    let outcome = Rc::new(RefCell::new(None));
    let recorded = outcome.clone();
    gpui_kit::application()
        .with_assets(crate::assets::NxToolsAssets)
        .run(move |cx| {
            let result = (|| -> anyhow::Result<gpui_kit::AnyWindowHandle> {
                crate::assets::register_fonts(cx)?;
                gpui_kit::init(cx);
                crate::theme::apply_component_theme(cx);
                let notifications = cx.new(|_| NotificationCenter::default());
                let application = crate::downloads::visual_preview::gallery_owner(
                    &scratch,
                    notifications,
                    false,
                    cx,
                )?;
                let initial = application.read(cx).settings_view_initial();
                let handle = cx.open_window(
                    WindowOptions {
                        window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                            point(px(40.0), px(40.0)),
                            size(px(984.0), px(650.0)),
                        ))),
                        titlebar: None,
                        window_decorations: Some(WindowDecorations::Client),
                        show: false,
                        focus: false,
                        ..Default::default()
                    },
                    move |window, cx| {
                        let initial_size = window.viewport_size();
                        assert!(
                            initial_size.width >= px(0.0) && initial_size.height >= px(0.0),
                            "hidden first layout cannot expose CW_USEDEFAULT: {initial_size:?}"
                        );
                        let view = cx
                            .new(|cx| DownloadSettingsView::new(application, initial, window, cx));
                        cx.new(|cx| Root::new(view, window, cx))
                    },
                )?;
                Ok(handle.into())
            })();
            match result {
                Err(error) => {
                    *recorded.borrow_mut() = Some(Err(error));
                    cx.quit();
                }
                Ok(handle) => cx.defer(move |cx| {
                    let _ = handle.update(cx, |_, window, _| {
                        window.resize(size(px(984.0), px(650.0)));
                    });
                    // Windows dispatches the public resize asynchronously.
                    cx.spawn(async move |cx| {
                        cx.background_executor()
                            .timer(std::time::Duration::from_millis(50))
                            .await;
                        cx.update(|cx| {
                            let result = handle
                                .update(cx, |_, window, cx| capture_settings(window, cx, &scratch))
                                .and_then(|result| result);
                            *recorded.borrow_mut() = Some(result);
                            cx.quit();
                        });
                    })
                    .detach();
                }),
            }
        });
    outcome
        .borrow_mut()
        .take()
        .expect("native fixture completed")
        .expect("production settings native paint");
}

fn capture_settings(window: &mut Window, cx: &mut App, scratch: &Path) -> anyhow::Result<()> {
    for (name, preference) in [
        ("dark", nxtools_core::ThemePreference::Dark),
        ("light", nxtools_core::ThemePreference::Light),
    ] {
        crate::theme::set_theme_preference(preference, cx);
        window.refresh();
        window.draw(cx).clear(cx);
        window.refresh();
        window.draw(cx).clear(cx);
        let image = window.render_to_image()?;
        let capture = scratch.join(format!("settings-solid-{name}.png"));
        image.save(&capture)?;
        println!("settings.native.capture={}", capture.display());
        anyhow::ensure!(
            image.width() >= 984 && image.height() >= 650,
            "native resize did not reach the owned canvas"
        );
        anyhow::ensure!(
            matches!(
                window.window_decorations(),
                gpui_kit::Decorations::Client { .. }
            ),
            "ordinary custom window must expose its requested official frame"
        );
        let theme = gpui_kit::component::Theme::global(cx);
        let foreground = gpui_kit::Rgba::from(theme.sidebar_foreground);
        let target = [foreground.r, foreground.g, foreground.b].map(|v| v * 255.0);
        let frame = gpui_kit::Rgba::from(theme.window_border);
        let frame_target = [frame.r, frame.g, frame.b].map(|v| v * 255.0);
        for x in [0, image.width() - 1] {
            let pixel = image.get_pixel(x, image.height() / 2).0;
            anyhow::ensure!(
                (0..3).all(|channel| (pixel[channel] as f32 - frame_target[channel]).abs() <= 2.0),
                "{name} official frame pixel missing at x={x}: {pixel:?}, expected={frame_target:?}"
            );
        }
        let scale = window.scale_factor();
        for index in 0..5 {
            let id = gpui_kit::ElementId::from(gpui_kit::SharedString::from(format!("0-{index}")));
            let node = gpui_kit::base::test_support::snapshots(window)
                .into_iter()
                .find(|node| node.path().last() == Some(&id))
                .ok_or_else(|| anyhow::anyhow!("navigation row {index} missing"))?;
            let bounds = node.bounds();
            let x0 = ((bounds.left().as_f32() + 5.0) * scale) as u32;
            let x1 = ((bounds.left().as_f32() + bounds.size.width.as_f32() * 0.6) * scale) as u32;
            let y0 = (bounds.top().as_f32() * scale) as u32;
            let y1 = (bounds.bottom().as_f32() * scale) as u32;
            let mut ink = 0;
            for y in y0..y1.min(image.height()) {
                for x in x0..x1.min(image.width()) {
                    let pixel = image.get_pixel(x, y).0;
                    if (0..3).all(|channel| (pixel[channel] as f32 - target[channel]).abs() < 40.0)
                    {
                        ink += 1;
                    }
                }
            }
            println!("settings.native.theme={name} row={index} bounds={bounds:?} ink={ink}");
            anyhow::ensure!(
                ink >= 8,
                "{name} navigation row {index} has no rendered text pixels; capture={}",
                capture.display()
            );
        }
    }
    Ok(())
}
