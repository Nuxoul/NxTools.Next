//! Debug-only production Extensions rendering with owned scratch configuration and no runtime.
use super::*;
use nxtools_core::AppConfig;
use std::path::PathBuf;

pub(super) fn open(cx: &mut App) -> anyhow::Result<()> {
    let root = PathBuf::from(std::env::var("PI_SCRATCH_DIR")?)
        .join(format!("extensions-ui-preview-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&root)?;
    // Synthetic presentation fixture only; no native preference reader is connected.
    cx.set_global(crate::ui::motion::UiMotionPreference::Known {
        reduced_motion: false,
    });
    cx.set_reduce_motion(false);
    for (mode, width, height, populated) in [
        ("extensions-empty", 1060.0, 650.0, false),
        ("extensions-list", 1060.0, 650.0, true),
        ("extensions-list-minimum", 920.0, 480.0, true),
        ("extensions-import-minimum", 920.0, 480.0, false),
        ("extensions-detail-minimum", 920.0, 480.0, true),
        ("extensions-environment", 1060.0, 650.0, false),
        ("extensions-environment-minimum", 920.0, 480.0, false),
        ("extensions-confirm", 1060.0, 650.0, false),
        ("extensions-confirm-minimum", 920.0, 480.0, false),
        ("extensions-confirm-replace", 920.0, 480.0, false),
        ("extensions-confirm-long", 920.0, 480.0, false),
        ("extensions-trust", 1060.0, 650.0, false),
        ("extensions-trust-minimum", 920.0, 480.0, false),
        ("extensions-trust-missing", 920.0, 480.0, false),
    ] {
        let mut config = AppConfig::default();
        if populated {
            for (id, name, keyword, url) in [
                (
                    "synthetic.docs",
                    "文档搜索",
                    "docs",
                    "https://example.invalid/search?q={query}",
                ),
                (
                    "synthetic.reference",
                    "参考资料",
                    "ref",
                    "https://reference.invalid/?q={query}",
                ),
            ] {
                let manifest = serde_json::json!({"schema_version":1,"id":id,"name":name,"version":"1.0.0",
                    "description":"使用关键词查找指定站点的文档，在快速启动中打开匹配结果。",
                    "handlers":[{"id":"search","title":"搜索文档","feature":"quick_launch","entry":"command",
                    "keywords":[keyword],"action":{"type":"website","url_template":url}}]});
                config.extensions.install(
                    nxtools_extensions::ImportPreview::from_json(&serde_json::to_vec(&manifest)?)?,
                    "合成预览包".into(),
                    false,
                )?;
            }
            config.extensions.set_enabled("synthetic.docs", true, &[])?;
            let bundle = root.join(format!("{mode}-catalog-cleaner"));
            std::fs::create_dir_all(&bundle)?;
            std::fs::write(bundle.join("main.py"), "# synthetic UI only; never run\n")?;
            std::fs::write(
                bundle.join("extension.json"),
                serde_json::to_vec(&serde_json::json!({
                    "schema_version":1,"id":"synthetic.cleaner","name":"剪贴板文本整理","version":"1.0.0",
                    "description":"去掉行尾空白、合并连续空行，生成整理结果并保留原文。",
                    "runtime":{"program":"python"},"files":["main.py"],
                    "handlers":[{"id":"clean","title":"整理文本并保留原文","feature":"clipboard","entry":"action","action":{"type":"script","entry_file":"main.py"}}]
                }))?,
            )?;
            config.extensions.install(
                nxtools_extensions::ImportPreview::from_directory(&bundle)?
                    .stage_into(&root.join("catalog-managed"))?,
                "合成剪贴板插件".into(),
                false,
            )?;
            let manifest = serde_json::json!({"schema_version":1,"id":"synthetic.resources","name":"常用资料","version":"1.0.0",
                "description":"打开常用文档和资料目录，不启动插件脚本。",
                "handlers":[{"id":"open","title":"打开资料站","feature":"tools","entry":"action","action":{"type":"website","url_template":"https://example.invalid/resources"}}]});
            config.extensions.install(
                nxtools_extensions::ImportPreview::from_json(&serde_json::to_vec(&manifest)?)?,
                "合成工具包".into(),
                false,
            )?;
            config
                .extensions
                .set_enabled("synthetic.resources", true, &[])?;
            config
                .extensions
                .packages
                .sort_by_key(|package| package.manifest.id != "synthetic.cleaner");
        }
        let confirmation = if mode.contains("confirm") || mode.contains("trust") {
            let bundle = root.join(format!("{mode}-bundle"));
            std::fs::create_dir_all(&bundle)?;
            std::fs::write(
                bundle.join("main.py"),
                "# synthetic preview; never executed\n",
            )?;
            let description = if mode.contains("long") {
                "整理所选剪贴板文本，合并重复空格并保留原始记录。".repeat(12)
            } else {
                "整理所选文本的空白与换行，生成新条目并保留原文。".into()
            };
            let manifest = serde_json::json!({"schema_version":1,"id":"synthetic.cleaner","name":"剪贴板文本整理","version":"1.0.0","description":description,"runtime":{"program":"python","timeout_ms":10000},"files":["main.py"],"permissions":[{"id":"clipboard.read","reason":"读取用户明确选中的文本历史条目"},{"id":"clipboard.write","reason":"创建新的整理结果，不覆盖原条目"}],"handlers":[{"id":"clean","title":"整理文本并保留原文","feature":"clipboard","entry":"action","action":{"type":"script","entry_file":"main.py"}}]});
            std::fs::write(
                bundle.join("extension.json"),
                serde_json::to_vec(&manifest)?,
            )?;
            let preview = nxtools_extensions::ImportPreview::from_directory(&bundle)?;
            if mode.contains("trust") {
                config.extensions.install(
                    preview.stage_into(&root.join("managed"))?,
                    "clipboard-cleaner.zip".into(),
                    false,
                )?;
            }
            if mode.contains("replace") {
                config.extensions.install(
                    preview.stage_into(&root.join("managed"))?,
                    "合成旧包".into(),
                    false,
                )?;
            }
            if mode.contains("trust") {
                None
            } else {
                Some((preview, "C:\\Synthetic\\clipboard-cleaner.zip".into()))
            }
        } else {
            None
        };
        let path = root.join(format!("{mode}.json"));
        config.save_to(&path)?;
        let notifications = cx.new(|_| NotificationCenter::default());
        let application = cx.new(|_| {
            let mut app = crate::extensions::ExtensionApplication::new(
                config,
                path,
                None,
                gpui_kit::WeakEntity::new_invalid(),
            );
            app.preview = confirmation;
            app
        });
        cx.open_window(final_gallery::options(width, height), move |window, cx| {
            let main = cx.new(|cx| {
                let mut main = MainWindow::new(
                    gpui_kit::WeakEntity::new_invalid(),
                    gpui_kit::WeakEntity::new_invalid(),
                    gpui_kit::WeakEntity::new_invalid(),
                    MainWindowShellContext {
                        settings: None,
                        platform: None,
                        system_settings: Rc::new(RefCell::new(SystemSettings::default())),
                    },
                    notifications,
                    window,
                    cx,
                );
                main.extension_application = Some(application);
                main.shell.activate(MainModule::Extensions);
                if mode.contains("import")
                    || mode.contains("detail")
                    || mode.contains("environment")
                    || mode.contains("confirm")
                    || mode.contains("trust")
                {
                    let application = main.extension_application.clone().expect("synthetic owner");
                    main.extension_manager = Some(cx.new(|cx| {
                        let mut view = super::super::extensions_page::ExtensionManagerView::new(
                            application,
                            window,
                            cx,
                        );
                        view.initialize_preview(
                            mode.contains("import"),
                            mode.contains("detail").then(|| "synthetic.docs".to_owned()),
                        );
                        if mode.contains("environment") {
                            view.open_environment(None, window, cx);
                        }
                        if mode.contains("trust") {
                            view.initialize_trust_preview(mode.contains("missing"), window, cx);
                        }
                        view
                    }));
                }
                main
            });
            final_gallery::ready(mode, width, height, window);
            cx.new(|cx| gpui_kit::base::Root::new(main, window, cx))
        })?;
    }
    for (mode, running) in [
        ("extensions-feedback-running", true),
        ("extensions-feedback-saved", false),
    ] {
        let application = cx.new(|_| {
            crate::extensions::ExtensionApplication::new(
                AppConfig::default(),
                root.join(format!("{mode}-unused.json")),
                None,
                gpui_kit::WeakEntity::new_invalid(),
            )
        });
        let run = application.update(cx, |app, _| app.seed_manual_feedback_preview(running));
        let history_path = root.join(format!("{mode}-history.db"));
        let mut history = nxtools_clipboard::ClipboardHistory::open(&history_path)?;
        for index in 0..5 {
            history.capture(nxtools_clipboard::TextCapture {
                text: format!("合成历史记录 {}：原面板内查看处理结果。", index + 1),
                native_formats: Vec::new(),
                source: nxtools_clipboard::CaptureSource {
                    window_handle: 0,
                    process_id: 0,
                    process_name: "预览数据".into(),
                    process_icon_path: String::new(),
                    window_title: String::new(),
                },
                captured_at_unix_ms: index + 1,
            })?;
        }
        let page = history.query_category_page(
            "",
            nxtools_clipboard::BUILTIN_ALL_CATEGORY_ID,
            nxtools_clipboard::ClipboardSearchScope::Content,
            0,
            80,
        )?;
        let categories = history.categories()?;
        drop(history);
        let worker = crate::clipboard_worker::ClipboardCaptureWorkerHandle::spawn(
            history_path,
            root.join(format!("{mode}-assets")),
            nxtools_clipboard::ClipboardRetentionPolicy {
                max_items: 100,
                retention_days: 30,
            },
            100,
        )?;
        let source_item_id = page.items.get(1).expect("second synthetic item").id;
        cx.open_window(final_gallery::options(300.0, 440.0), move |window, cx| {
            let session = uuid::Uuid::new_v4();
            let panel = cx.new(|cx| {
                crate::ui::ClipboardPanel::new(
                    page,
                    categories,
                    Default::default(),
                    Vec::new(),
                    Default::default(),
                    nxtools_protocol::ForegroundContext {
                        window_handle: 0,
                        focus_window_handle: 0,
                        process_id: 0,
                        process_name: String::new(),
                        window_title: String::new(),
                    },
                    worker,
                    gpui_kit::WeakEntity::new_invalid(),
                    session,
                    None,
                    window,
                    cx,
                )
            });
            let origin = crate::extensions::manual_actions::ClipboardOrigin {
                panel: panel.downgrade(),
                session_id: session,
                window: window.window_handle(),
                item_id: source_item_id,
            };
            panel.update(cx, |panel, cx| {
                panel.show_extension_feedback(application, run, origin, window, cx)
            });
            final_gallery::ready(mode, 300.0, 440.0, window);
            cx.new(|cx| gpui_kit::base::Root::new(panel, window, cx))
        })?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn extensions_preview_is_scratch_only_without_runtime_or_connected_bridges() {
        let all = include_str!("extensions.rs");
        let source = all.split("#[cfg(test)]").next().expect("preview source");
        assert!(source.contains("PI_SCRATCH_DIR") && source.contains("WeakEntity::new_invalid()"));
        assert!(!source.contains("ExtensionService::new") && !source.contains("DesktopRuntime"));
    }
}
