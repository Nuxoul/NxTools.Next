//! Audit08 actual production labels and original gesture owners; no native/data operation.
use super::*;
use gpui_kit::base::{Root, test_support};
use gpui_kit::{AppContext, ElementId, Modifiers, Role, TestAppContext, VisualTestContext, size};
use nxtools_core::AppConfig;
use nxtools_download::{DownloadEngine, DownloadRequest};

fn draw(surface: &mut VisualTestContext) {
    surface.run_until_parked();
    surface.update(|window, cx| {
        window.refresh();
        window.draw(cx).clear(cx);
    });
}
fn node(surface: &mut VisualTestContext, id: &'static str) -> test_support::ElementSnapshot {
    surface.update(|window, _| {
        test_support::snapshots(window)
            .into_iter()
            .find(|node| node.path().last() == Some(&ElementId::from(id)))
            .unwrap_or_else(|| panic!("actual named production node {id}"))
    })
}
fn click(surface: &mut VisualTestContext, id: &'static str) {
    let hit = surface
        .debug_bounds(id)
        .unwrap_or_else(|| panic!("actual hit {id}"));
    surface.simulate_click(hit.center(), Modifiers::default());
    draw(surface);
}
fn main_fixture(
    cx: &mut TestAppContext,
    downloads: gpui_kit::WeakEntity<DownloadApplication>,
) -> (Entity<MainWindow>, VisualTestContext) {
    cx.update(gpui_kit::init);
    cx.update(crate::theme::apply_component_theme);
    let notices = cx.new(|_| NotificationCenter::default());
    let slot = Rc::new(RefCell::new(None));
    let retained = slot.clone();
    let (_, surface) = cx.add_window_view(move |window, cx| {
        let main = cx.new(|cx| {
            MainWindow::new(
                gpui_kit::WeakEntity::new_invalid(),
                gpui_kit::WeakEntity::new_invalid(),
                downloads,
                MainWindowShellContext {
                    settings: None,
                    platform: None,
                    system_settings: Rc::new(RefCell::new(SystemSettings::default())),
                },
                notices,
                window,
                cx,
            )
        });
        *retained.borrow_mut() = Some(main.clone());
        Root::new(main, window, cx)
    });
    let handle = surface.update(|window, _| window.window_handle());
    let main = slot.borrow().clone().expect("production MainWindow");
    (main, VisualTestContext::from_window(handle, cx))
}
#[gpui_kit::test]
fn audit08_download_header_labels_name_followup(cx: &mut TestAppContext) {
    let (_, mut surface) = main_fixture(cx, gpui_kit::WeakEntity::new_invalid());
    for dimensions in [(1060., 650.), (920., 480.)] {
        surface.simulate_resize(size(px(dimensions.0), px(dimensions.1)));
        draw(&mut surface);
        click(&mut surface, "main-module-下载");
        for (id, label) in [
            ("download-settings", "下载设置…"),
            ("download-schedule", "下载计划…"),
            ("download-new-task", "新建下载…"),
        ] {
            let control = node(&mut surface, id);
            assert_eq!(control.role(), Some(Role::Button));
            assert_eq!(control.label(), Some(label), "actual label/a11y {id}");
            assert!(control.bounds().right() <= px(dimensions.0));
        }
    }
}
#[gpui_kit::test]
fn audit08_tools_configure_labels_and_pointer_keep_keyed_window(cx: &mut TestAppContext) {
    let (main, mut surface) = main_fixture(cx, gpui_kit::WeakEntity::new_invalid());
    for dimensions in [(1060., 650.), (920., 480.)] {
        surface.simulate_resize(size(px(dimensions.0), px(dimensions.1)));
        draw(&mut surface);
        click(&mut surface, "main-module-其他工具");
        for id in [
            "tool-link-detector-configure",
            "tool-currency-configure",
            "tool-floating-assistant-configure",
        ] {
            let control = node(&mut surface, id);
            assert_eq!(control.label(), Some("配置…"));
            assert!(control.bounds().right() <= px(dimensions.0));
        }
        click(&mut surface, "tool-link-detector-configure");
        let handle = main.read_with(&surface, |main, _| {
            *main
                .tool_settings_windows
                .get("tool-link-detector")
                .expect("original keyed window")
        });
        click(&mut surface, "tool-link-detector-configure");
        assert_eq!(
            main.read_with(&surface, |main, _| main.tool_settings_windows.len()),
            1
        );
        assert_eq!(
            main.read_with(
                &surface,
                |main, _| main.tool_settings_windows["tool-link-detector"]
            ),
            handle
        );
        let mut tool = VisualTestContext::from_window(handle, &surface);
        draw(&mut tool);
        let close = tool
            .debug_bounds("tool-settings-close")
            .expect("actual close");
        tool.simulate_click(close.center(), Modifiers::default());
        surface.run_until_parked();
    }
}

struct MenuFixture {
    application: Entity<DownloadApplication>,
    task: DownloadTaskId,
}
impl Render for MenuFixture {
    fn render(&mut self, _: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let application = self.application.downgrade();
        let task = self.task;
        div().size_full().child(
            Button::new("audit08-menu-trigger")
                .debug_selector(|| "audit08-menu-trigger".into())
                .label("任务操作")
                .dropdown_menu(move |menu, window, cx| {
                    let capabilities = application
                        .read_with(cx, |app, _| {
                            download_context_capabilities(app.tasks(), &[task])
                        })
                        .unwrap_or_default();
                    download_task_context_menu(
                        Theme::active(cx),
                        menu,
                        application.clone(),
                        vec![task],
                        capabilities,
                        window,
                        cx,
                    )
                }),
        )
    }
}
fn menu_fixture(
    cx: &mut TestAppContext,
    delete_file: bool,
) -> (
    Entity<DownloadApplication>,
    DownloadEngine,
    DownloadTaskId,
    tempfile::TempDir,
    VisualTestContext,
) {
    cx.update(gpui_kit::init);
    cx.update(crate::theme::apply_component_theme);
    let directory = tempfile::tempdir().expect("owned naming fixture");
    let engine = DownloadEngine::open_in_memory().expect("no transfer runner");
    let task = engine
        .enqueue(
            DownloadRequest {
                source_url: "https://synthetic.invalid/not-requested".into(),
                save_directory: directory.path().into(),
                save_directory_explicit: true,
                file_name: Some("很长的合成中文文件名-仅验证文案与取消.bin".into()),
                headers: vec![],
                cookie: None,
                referrer: None,
                certificate_policy: Default::default(),
                connection_limit: Some(1),
                speed_limit_bytes_per_second: None,
                expected_total_bytes: None,
                scheduled_at_unix_ms: None,
                adaptive_segment_policy: None,
                media: None,
                category_id: None,
                category_name: None,
                duplicate_strategy: nxtools_download::DownloadDuplicateStrategy::UseDefault,
            },
            RequestOrigin::Internal,
        )
        .expect("queue synthetic only");
    let mut snapshot = engine.tasks().expect("snapshot").remove(0);
    for status in [
        DownloadStatus::Probing,
        DownloadStatus::Waiting,
        DownloadStatus::Downloading,
        DownloadStatus::Finalizing,
        DownloadStatus::Completed,
    ] {
        snapshot
            .task
            .transition_to(status, 1)
            .expect("display-only completed snapshot");
    }
    let sentinel = directory.path().join("保留的合成文件.bin");
    std::fs::write(&sentinel, b"must remain").expect("owned sentinel");
    snapshot.final_path = Some(sentinel);
    snapshot.has_final_artifact = true;
    let mut config = AppConfig::default();
    config.download.task_removal_deletes_file = delete_file;
    let config_path = directory.path().join("config.json");
    config.save_to(&config_path).expect("owned config");
    let (platform, _endpoint) = nxtools_ipc::platform_bridge(16);
    let notices = cx.new(|_| NotificationCenter::default());
    let engine_for_app = engine.clone();
    let root = directory.path().to_path_buf();
    let application = cx.new(|cx| {
        DownloadApplication::new(
            engine_for_app,
            Rc::new(platform),
            notices,
            crate::downloads::DownloadRuntimeContext {
                fallback_default_directory: root.clone(),
                config,
                config_path,
                log_directory: root.clone(),
                database_path: root.join("unused.db"),
                ffmpeg_install_directory: root.join("ffmpeg"),
            },
            cx,
        )
    });
    application.update(cx, |app, _| app.replace_tasks_for_test(vec![snapshot]));
    let content = application.clone();
    let (_, surface) = cx.add_window_view(move |window, cx| {
        let view = cx.new(|_| MenuFixture {
            application: content,
            task,
        });
        Root::new(view, window, cx)
    });
    let handle = surface.update(|window, _| window.window_handle());
    let mut surface = VisualTestContext::from_window(handle, cx);
    surface.simulate_resize(size(px(620.), px(420.)));
    draw(&mut surface);
    (application, engine, task, directory, surface)
}
#[gpui_kit::test]
fn audit08_download_menu_labels_follow_existing_delete_policy(cx: &mut TestAppContext) {
    for delete_file in [true, false] {
        let (_application, _engine, _task, _directory, mut surface) = menu_fixture(cx, delete_file);
        click(&mut surface, "audit08-menu-trigger");
        let expected = if delete_file {
            "删除文件…"
        } else {
            "从列表移除"
        };
        assert_eq!(
            node(&mut surface, "download-menu-delete-file").label(),
            Some(expected)
        );
        assert_eq!(
            node(&mut surface, "download-menu-task-window").label(),
            Some("任务窗口…")
        );
        assert_eq!(
            node(&mut surface, "download-menu-remove").label(),
            Some("从列表移除")
        );
        surface.simulate_keystrokes("escape");
        draw(&mut surface);
    }
}

#[gpui_kit::test]
fn audit08_delete_confirmation_cancel_keeps_task_and_file(cx: &mut TestAppContext) {
    let (_application, engine, task, directory, mut surface) = menu_fixture(cx, true);
    let path = directory.path().join("保留的合成文件.bin");
    let before = std::fs::read(&path).expect("sentinel before");
    let windows = surface.read(|cx| cx.windows());
    click(&mut surface, "audit08-menu-trigger");
    click(&mut surface, "download-menu-delete-file");
    let opened = surface.read(|cx| {
        cx.windows()
            .into_iter()
            .filter(|h| !windows.contains(h))
            .collect::<Vec<_>>()
    });
    assert_eq!(opened.len(), 1, "one original confirmation");
    assert!(engine.task(task).expect("task read").is_some());
    assert_eq!(
        std::fs::read(&path).expect("not deleted before decision"),
        before
    );
    let handle = opened[0];
    let mut confirmation = VisualTestContext::from_window(handle, &surface);
    draw(&mut confirmation);
    assert_eq!(
        node(&mut confirmation, "download-delete-confirm").label(),
        Some("删除文件"),
        "final immediate commit does not add ellipsis"
    );
    assert_eq!(
        node(&mut confirmation, "download-delete-cancel").label(),
        Some("取消")
    );
    let cancel = node(&mut confirmation, "download-delete-cancel").bounds();
    confirmation.simulate_click(cancel.center(), Modifiers::default());
    surface.run_until_parked();
    assert!(!surface.read(|cx| cx.windows().contains(&handle)));
    assert!(
        engine
            .task(task)
            .expect("task remains after cancel")
            .is_some()
    );
    assert_eq!(
        std::fs::read(&path).expect("file remains after cancel"),
        before
    );
}

#[gpui_kit::test]
async fn audit08_list_removal_and_task_cancel_name_distinct_results(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (application, engine, task, directory, mut surface) = menu_fixture(cx, false);
    let path = directory.path().join("保留的合成文件.bin");
    let before = std::fs::read(&path).expect("owned file");
    click(&mut surface, "audit08-menu-trigger");
    assert_eq!(
        node(&mut surface, "download-menu-delete-file").label(),
        Some("从列表移除")
    );
    let windows = surface.read(|cx| cx.windows());
    click(&mut surface, "download-menu-delete-file");
    cx.condition(&application, |app, _| !app.batch_busy()).await;
    assert!(engine.task(task).expect("list removal result").is_none());
    assert_eq!(
        std::fs::read(&path).expect("list removal preserves final file"),
        before
    );
    assert_eq!(
        surface.read(|cx| cx.windows()),
        windows,
        "list-only branch opens no confirmation"
    );

    let (cancel_app, cancel_engine, cancel_task, cancel_directory, mut cancel_surface) =
        menu_fixture(cx, true);
    let tasks = cancel_engine.tasks().expect("original queued tasks");
    cancel_app.update(&mut cancel_surface, |app, _| {
        app.replace_tasks_for_test(tasks)
    });
    draw(&mut cancel_surface);
    click(&mut cancel_surface, "audit08-menu-trigger");
    assert_eq!(
        node(&mut cancel_surface, "download-menu-cancel").label(),
        Some("取消任务")
    );
    click(&mut cancel_surface, "download-menu-cancel");
    cx.condition(&cancel_app, |app, _| !app.batch_busy()).await;
    assert_eq!(
        cancel_engine
            .task(cancel_task)
            .expect("cancel result")
            .expect("cancel retains task")
            .task
            .status(),
        DownloadStatus::Cancelled
    );
    assert_eq!(
        std::fs::read(cancel_directory.path().join("保留的合成文件.bin"))
            .expect("cancel preserves unrelated final sentinel"),
        b"must remain"
    );
}

#[gpui_kit::test]
fn audit08_floating_image_picker_label_names_followup(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (_, surface) = cx.add_window_view(|window, cx| {
        let view = cx.new(|cx| {
            super::tools_page::floating_assistant_settings::FloatingAssistantSettingsBody::new(
                window, cx,
            )
        });
        Root::new(view, window, cx)
    });
    let handle = surface.update(|window, _| window.window_handle());
    let mut surface = VisualTestContext::from_window(handle, cx);
    surface.simulate_resize(size(px(460.), px(320.)));
    draw(&mut surface);
    assert_eq!(
        node(&mut surface, "floating-image-pick").label(),
        Some("选择图片…")
    );
    assert_eq!(
        node(&mut surface, "floating-image-reset").label(),
        Some("恢复默认")
    );
}

#[gpui_kit::test]
fn audit08_menu_keyboard_confirmation_cancel_never_dispatches_twice(cx: &mut TestAppContext) {
    let (_application, engine, task, directory, mut surface) = menu_fixture(cx, true);
    let before = surface.read(|cx| cx.windows());
    click(&mut surface, "audit08-menu-trigger");
    let label = node(&mut surface, "download-menu-delete-file");
    surface.simulate_mouse_move(label.bounds().center(), None, Modifiers::default());
    draw(&mut surface);
    surface.simulate_keystrokes("enter");
    surface.run_until_parked();
    surface.simulate_event(gpui_kit::KeyUpEvent {
        keystroke: gpui_kit::Keystroke::parse("enter").expect("Enter"),
    });
    surface.run_until_parked();
    let opened = surface.read(|cx| {
        cx.windows()
            .into_iter()
            .filter(|h| !before.contains(h))
            .collect::<Vec<_>>()
    });
    assert_eq!(
        opened.len(),
        1,
        "original menu Enter creates one confirmation"
    );
    let handle = opened[0];
    let mut confirmation = VisualTestContext::from_window(handle, &surface);
    draw(&mut confirmation);
    confirmation.update(|window, cx| window.focus_next(cx));
    for _ in 0..24 {
        if node(&mut confirmation, "download-delete-cancel").focused() == Some(true) {
            break;
        }
        confirmation.simulate_keystrokes("tab");
        draw(&mut confirmation);
    }
    assert_eq!(
        node(&mut confirmation, "download-delete-cancel").focused(),
        Some(true)
    );
    let key = gpui_kit::Keystroke::parse("space").expect("Space");
    confirmation.simulate_event(gpui_kit::KeyDownEvent {
        keystroke: key.clone(),
        is_held: false,
        prefer_character_input: false,
    });
    surface.run_until_parked();
    assert!(surface.read(|cx| cx.windows().contains(&handle)));
    confirmation.simulate_event(gpui_kit::KeyUpEvent { keystroke: key });
    surface.run_until_parked();
    assert!(!surface.read(|cx| cx.windows().contains(&handle)));
    assert!(engine.task(task).expect("cancel record").is_some());
    assert_eq!(
        std::fs::read(directory.path().join("保留的合成文件.bin")).expect("sentinel"),
        b"must remain"
    );
}
