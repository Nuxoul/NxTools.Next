//! Debug-only synthetic Clipboard production-panel projections; scratch worker supplied by isolated entry.
use super::*;
use crate::main_window::visual_preview::final_gallery::{options, ready};
use gpui_kit::base::Root;

fn item(index: i64) -> ClipboardItem {
    ClipboardItem {
        id: index,
        kind: ClipboardKind::Text,
        text: format!("合成剪贴板记录{index}\n仅用于截图的中文纯文本 **literal**"),
        image_path: String::new(),
        image_width: 0,
        image_height: 0,
        file_paths: Vec::new(),
        source: nxtools_clipboard::CaptureSource {
            window_handle: 0,
            process_id: 0,
            process_name: "SyntheticEditor.exe".into(),
            process_icon_path: String::new(),
            window_title: String::new(),
        },
        captured_at_unix_ms: 1,
        last_used_at_unix_ms: 1,
        copy_count: 3,
        is_favorite: false,
        is_pinned: false,
        deleted_at_unix_ms: None,
    }
}
pub(crate) fn open(
    worker: crate::clipboard_worker::ClipboardCaptureWorkerHandle,
    cx: &mut App,
) -> anyhow::Result<()> {
    for (mode, width, height) in [
        ("clipboard-history", 300.0, 440.0),
        ("clipboard-minimum", 240.0, 380.0),
        ("clipboard-empty", 300.0, 440.0),
        ("clipboard-loading", 300.0, 440.0),
        ("clipboard-error", 300.0, 440.0),
        ("clipboard-text", 300.0, 440.0),
        ("clipboard-category", 300.0, 440.0),
    ] {
        let worker = worker.clone();
        cx.open_window(options(width, height), move |window, cx| {
            let panel = cx.new(|cx| {
                let items = if matches!(
                    mode,
                    "clipboard-empty" | "clipboard-loading" | "clipboard-error"
                ) {
                    Vec::new()
                } else {
                    (1..16).map(item).collect()
                };
                let mut panel = ClipboardPanel::new(
                    ClipboardHistoryPage {
                        total: items.len(),
                        has_more: false,
                        items,
                    },
                    vec![ClipboardCategory {
                        id: BUILTIN_ALL_CATEGORY_ID.into(),
                        name: "全部".into(),
                        kind: ClipboardCategoryKind::All,
                        sort_order: 0,
                    }],
                    HashMap::new(),
                    vec!["SyntheticEditor.exe".into(), "长名称合成来源.exe".repeat(5)],
                    ClipboardModifierActions::default(),
                    nxtools_protocol::ForegroundContext {
                        window_handle: 0,
                        focus_window_handle: 0,
                        process_id: 0,
                        process_name: String::new(),
                        window_title: String::new(),
                    },
                    worker,
                    WeakEntity::new_invalid(),
                    uuid::Uuid::new_v4(),
                    None,
                    window,
                    cx,
                );
                if mode == "clipboard-loading" {
                    panel.search_pending = true;
                }
                if mode == "clipboard-error" {
                    panel.query_error = Some("合成查询错误，长内容保持可滚动。".repeat(20));
                }
                if mode == "clipboard-text" {
                    let mut text = item(1);
                    text.text = "长中文合成编辑内容，仅供截图。\n".repeat(80);
                    panel.begin_edit(&text, window, cx);
                }
                if mode == "clipboard-category" {
                    panel.begin_category_editor(None, window, cx);
                }
                panel
            });
            ready(mode, width, height, window);
            cx.new(|cx| Root::new(panel, window, cx))
        })?;
    }
    Ok(())
}
