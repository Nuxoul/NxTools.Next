//! Debug-only production Quick Find rendering with synthetic models and no runtime.
use super::*;
use gpui_kit::WindowDecorations;

fn result(id: &str, name: &str, target: &str, source: ApplicationSource) -> SearchResult {
    SearchResult {
        entry: nxtools_finder::ApplicationEntry {
            id: id.into(),
            name: name.into(),
            target: target.into(),
            source,
        },
        action: SearchResultAction::OpenTarget,
        subtitle: (source == ApplicationSource::CustomCommand).then(|| target.into()),
        score: 0,
        activation_count: 0,
        last_activated_at_unix_ms: None,
        query_activation_count: 0,
        last_query_activation_at_unix_ms: None,
    }
}

pub(crate) fn open(cx: &mut App) -> anyhow::Result<()> {
    for (mode, origin, expanded) in [
        ("collapsed", point(px(80.0), px(80.0)), false),
        ("results", point(px(80.0), px(180.0)), true),
        ("actions", point(px(740.0), px(180.0)), true),
    ] {
        let bounds = Bounds::new(
            origin,
            size(
                px(PANEL_WIDTH),
                px(if expanded {
                    PANEL_HEIGHT
                } else {
                    COLLAPSED_PANEL_HEIGHT
                }),
            ),
        );
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: None,
                window_decorations: Some(WindowDecorations::Client),
                is_resizable: false,
                app_id: Some(format!("NxTools.QuickFind.LayoutPreview.{mode}")),
                ..Default::default()
            },
            move |window, cx| {
                window.set_window_title(&format!("NxTools Quick Find Preview — synthetic {mode}"));
                let model = cx.new(|cx| {
                    let mut panel = QuickLaunchPanel::new(
                        vec![
                            result(
                                "app",
                                "Windows 终端",
                                "系统应用 · 合成内容",
                                ApplicationSource::System,
                            ),
                            result(
                                "file",
                                "项目说明与工作记录.txt",
                                r"C:\synthetic\documents\项目说明与工作记录.txt",
                                ApplicationSource::IndexedFile,
                            ),
                            result(
                                "folder",
                                "开发项目",
                                r"D:\synthetic\projects",
                                ApplicationSource::IndexedDirectory,
                            ),
                            result(
                                "long",
                                "一个较长的中文名称，用来检查文字层级、裁剪和快捷键位置",
                                r"D:\synthetic\projects\a-long-directory-name\notes.txt",
                                ApplicationSource::IndexedFile,
                            ),
                            result(
                                "command",
                                "合成命令（不会执行）",
                                "命令摘要 · 不连接真实应用",
                                ApplicationSource::CustomCommand,
                            ),
                        ],
                        nxtools_protocol::ForegroundContext {
                            window_handle: 0,
                            focus_window_handle: 0,
                            process_id: 0,
                            process_name: String::new(),
                            window_title: String::new(),
                        },
                        None,
                        Vec::new(),
                        WeakEntity::new_invalid(),
                        window,
                        cx,
                    );
                    panel.row_style = if std::env::var("NXTOOLS_PREVIEW_QUICK_CONTINUOUS")
                        .as_deref()
                        == Ok("1")
                    {
                        nxtools_core::QuickLaunchRowStyle::Continuous
                    } else {
                        nxtools_core::QuickLaunchRowStyle::Separated
                    };
                    panel.results_visible = expanded;
                    if mode == "actions" {
                        let mut actions = QuickLaunchActionMode::enter_in_context(
                            panel.results[1].clone(),
                            "记录".into(),
                            0,
                            None,
                        );
                        actions.current_level_mut().shell_pending = false;
                        panel.action_mode = Some(actions);
                    }
                    panel.sync_window_content_size(
                        if expanded { "synthetic" } else { "" },
                        window,
                        cx,
                    );
                    // This startup-isolated fixture has no application/native adapter.
                    window.resize(panel.desired_content_size(window, cx));
                    panel
                });
                println!(
                    "ui.quick_layout.ready mode={mode} hwnd={} scale={}",
                    native_window_handle(window).unwrap_or_default(),
                    window.scale_factor()
                );
                cx.new(|cx| Root::new(model, window, cx))
            },
        )?;
    }
    cx.open_window(WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds::new(point(px(1400.0), px(180.0)), size(px(QUICK_LAUNCH_PREVIEW_WIDTH), px(QUICK_LAUNCH_PREVIEW_HEIGHT))))),
        titlebar: None, is_resizable: false, window_decorations: Some(WindowDecorations::Client),
        app_id: Some("NxTools.QuickFind.LayoutPreview.Content".into()), ..Default::default()
    }, |window, cx| {
        window.set_window_title("NxTools Quick Find Preview — synthetic content");
        let model = cx.new(|_| QuickLaunchPreview::new(QuickLaunchPreviewPayload {
            entry_id: "file".into(), name: "项目说明与工作记录.md".into(),
            target: r"C:\synthetic\documents\项目说明与工作记录.md".into(),
            detail: "Markdown · 合成预览内容".into(), created_at: Some("2026-10-11 12:00".into()),
            modified_at: Some("2026-10-11 13:00".into()),
            body: QuickLaunchPreviewBody::Text { text: "# 项目记录\n\n这是**隔离渲染检查**，不读取任何用户文件。\n\n- 原始动作载荷保持独立\n- 被动预览不接管键盘焦点\n\n| 状态 | 结果 |\n| --- | --- |\n| 本地 fixture | 可用 |\n\n```rust\nlet preview = document.readonly();\n```\n\n最后一段合成内容。".into(), truncated: false },
        }, WeakEntity::new_invalid()));
        println!("ui.quick_layout.ready mode=content hwnd={} scale={}", native_window_handle(window).unwrap_or_default(), window.scale_factor());
        cx.new(|cx| Root::new(model, window, cx))
    })?;
    Ok(())
}
