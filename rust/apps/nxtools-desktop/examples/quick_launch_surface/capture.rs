//! One-shot GPU readback for the standalone visual fixture.
use gpui_kit::{AnyWindowHandle, App, Window};
use std::{cell::Cell, path::Path, path::PathBuf, rc::Rc};

pub(super) fn schedule(
    handle: AnyWindowHandle,
    directory: PathBuf,
    without_shadow: bool,
    successful: Rc<Cell<bool>>,
    correlation: uuid::Uuid,
    cx: &mut App,
) {
    cx.defer(move |cx| {
        let result = handle
            .update(cx, |_, window, cx| export(window, cx, &directory, without_shadow))
            .map_err(|_| "window")
            .and_then(|result| result);
        match result {
            Ok((width, height)) => {
                successful.set(true);
                println!("ui.surface_preview.capture.finished correlation={correlation} width={width} height={height}");
            }
            Err(stage) => {
                successful.set(false);
                eprintln!("ui.surface_preview.capture.failed correlation={correlation} stage={stage}");
            }
        }
        cx.quit();
    });
}

fn export(
    window: &mut Window,
    cx: &mut App,
    directory: &Path,
    without_shadow: bool,
) -> Result<(u32, u32), &'static str> {
    let clear = window.draw(cx);
    let readback = window.render_to_image();
    clear.clear(cx);
    let image = readback.map_err(|_| "readback")?;
    std::fs::create_dir_all(directory).map_err(|_| "directory")?;
    image
        .save(directory.join("frame-premultiplied.png"))
        .map_err(|_| "raw_save")?;
    // DirectComposition stores premultiplied RGB. Composite once onto white;
    // treating the raw bytes as a straight-alpha PNG would darken its edges.
    let mut white = image.clone();
    for pixel in white.pixels_mut() {
        let background = 255 - pixel[3];
        for channel in &mut pixel.0[..3] {
            *channel = channel.saturating_add(background);
        }
        pixel[3] = 255;
    }
    white
        .save(directory.join("frame-on-white.png"))
        .map_err(|_| "white_save")?;
    let metadata = serde_json::json!({
        "width": image.width(),
        "height": image.height(),
        "scale": window.scale_factor(),
        "shadow": !without_shadow,
    });
    std::fs::write(directory.join("frame.json"), metadata.to_string()).map_err(|_| "metadata")?;
    Ok(image.dimensions())
}
