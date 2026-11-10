use super::*;
use gpui_kit::{Modifiers, TestAppContext, VisualTestContext};

struct PickerFixture {
    app: Entity<DownloadApplication>,
    endpoint: nxtools_ipc::PlatformServerEndpoint,
    root: PathBuf,
    config_path: PathBuf,
}

impl PickerFixture {
    fn new(cx: &mut TestAppContext) -> Self {
        cx.update(gpui_kit::init);
        let root = std::env::temp_dir().join(format!("download-picker-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).expect("picker temporary directory");
        let config_path = root.join("config.json");
        let mut config = AppConfig::default();
        config.download.default_directory = root.to_string_lossy().into_owned();
        config.quick_launch.enabled = false;
        config
            .save_to(&config_path)
            .expect("isolated picker config");
        let (platform, endpoint) = nxtools_ipc::platform_bridge(32);
        endpoint.set_connected_for_contract_test(true);
        let app = cx.update(|cx| {
            let notifications = cx.new(|_| NotificationCenter::default());
            cx.new(|cx| {
                DownloadApplication::new(
                    DownloadEngine::open_in_memory().expect("picker in-memory engine"),
                    Rc::new(platform),
                    notifications,
                    DownloadRuntimeContext {
                        fallback_default_directory: root.clone(),
                        config,
                        config_path: config_path.clone(),
                        log_directory: root.clone(),
                        database_path: root.join("downloads.sqlite3"),
                        ffmpeg_install_directory: root.join("tools"),
                    },
                    cx,
                )
            })
        });
        Self {
            app,
            endpoint,
            root,
            config_path,
        }
    }

    fn settings(
        &self,
        cx: &mut TestAppContext,
    ) -> (Entity<DownloadSettingsView>, VisualTestContext) {
        self.app.update(cx, |app, cx| app.open_settings_window(cx));
        cx.run_until_parked();
        let (view, handle) = self.app.read_with(cx, |app, _| {
            (
                app.settings_view.clone().expect("production settings view"),
                app.settings_window.expect("production settings window"),
            )
        });
        let mut surface = VisualTestContext::from_window(handle, cx);
        surface.refresh().expect("production settings frame");
        (view, surface)
    }

    fn click_picker(&self, surface: &mut VisualTestContext) -> uuid::Uuid {
        surface.refresh().expect("picker button frame");
        let button = surface
            .debug_bounds("download-settings-pick-directory")
            .expect("real settings picker Button");
        let viewport = surface.update(|window, _| window.viewport_size());
        assert!(
            button.left() >= px(0.0)
                && button.right() <= viewport.width
                && button.top() >= px(0.0)
                && button.bottom() <= viewport.height,
            "picker click must be visible: {button:?}, viewport={viewport:?}"
        );
        let snapshots = surface.update(|window, _| gpui_kit::base::test_support::snapshots(window));
        assert!(
            snapshots.iter().any(|node| node.path().last()
                == Some(&gpui_kit::ElementId::from(
                    "download-settings-pick-directory"
                ))
                && node.visible()),
            "mounted official picker Button: {snapshots:?}"
        );
        surface.simulate_click(button.center(), Modifiers::default());
        surface.run_until_parked();
        self.take_picker()
    }

    fn take_picker(&self) -> uuid::Uuid {
        while let Ok(envelope) = self.endpoint.try_receive_command_for_contract_test() {
            if matches!(
                envelope.message,
                nxtools_protocol::Message::PlatformCommand(PlatformCommand::PickDirectory(_))
            ) {
                assert_eq!(
                    envelope.correlation_id,
                    Some(envelope.id),
                    "one cross-process picker operation ID"
                );
                return envelope.id;
            }
        }
        panic!("real picker callback must enqueue typed PickDirectory");
    }

    fn reply(&self, cx: &mut TestAppContext, id: uuid::Uuid, result: PlatformCommandResult) {
        self.app.update(cx, |app, cx| {
            app.handle_platform_notification(
                PlatformNotification::CommandResult {
                    correlation_id: id,
                    result,
                },
                cx,
            )
        });
        cx.run_until_parked();
    }

    fn config_directory(&self) -> String {
        AppConfig::load_from(&self.config_path)
            .expect("read picker config")
            .download
            .default_directory
    }

    fn new_task(&self, cx: &mut TestAppContext) -> (Entity<NewDownloadView>, VisualTestContext) {
        self.app
            .update(cx, |app, cx| app.open_new_download_window(cx));
        cx.run_until_parked();
        let (view, handle) = self.app.read_with(cx, |app, _| {
            (
                app.new_download_view
                    .clone()
                    .expect("registered new-task view"),
                app.new_download_window.expect("registered new-task window"),
            )
        });
        let mut surface = VisualTestContext::from_window(handle, cx);
        view.update_in(&mut surface, |view, window, cx| {
            view.url.update(cx, |input, cx| {
                input.set_value("https://example.test/picker.bin", window, cx)
            });
            view.confirmed_request = Some(view.request(cx).expect("synthetic confirmed request"));
            view.probe = Some(DownloadProbe {
                resolution: None,
                server_observation: None,
                final_url: "https://example.test/picker.bin".into(),
                file_name: Some("picker.bin".into()),
                total_bytes: Some(1024),
                supports_ranges: true,
                content_type: None,
                validator: Default::default(),
                media: None,
            });
            view.confirmed_target_height = None;
            view.error = Some("existing validation".into());
            cx.notify();
        });
        surface.simulate_resize(new_download_window_size(true));
        surface.refresh().expect("real confirmed new-task form");
        (view, surface)
    }

    fn click_new_picker(&self, surface: &mut VisualTestContext) -> uuid::Uuid {
        surface.refresh().expect("new-task picker frame");
        let button = surface
            .update(|window, _| gpui_kit::base::test_support::snapshots(window))
            .into_iter()
            .find(|node| node.path().last() == Some(&"download-pick-directory".into()))
            .expect("real new-task picker Button")
            .bounds();
        surface.simulate_click(button.center(), Modifiers::default());
        surface.run_until_parked();
        self.take_picker()
    }

    fn pending(&self, cx: &TestAppContext) -> Option<uuid::Uuid> {
        self.app.read_with(cx, |app, _| {
            app.pending_directory_picker
                .as_ref()
                .map(|pending| pending.request.command_id)
        })
    }

    fn notices(&self, cx: &TestAppContext) -> Vec<String> {
        self.app.read_with(cx, |app, cx| {
            app.notifications
                .read(cx)
                .items()
                .iter()
                .map(|notice| notice.id.to_string())
                .collect()
        })
    }

    fn drain_commands(&self) {
        while self
            .endpoint
            .try_receive_command_for_contract_test()
            .is_ok()
        {}
    }
}

impl Drop for PickerFixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

fn picked(path: &std::path::Path) -> PlatformCommandResult {
    PlatformCommandResult {
        success: true,
        error: None,
        native_error_code: None,
        data: Some(path.to_string_lossy().into_owned()),
    }
}

fn directory(view: &Entity<DownloadSettingsView>, cx: &TestAppContext) -> String {
    view.read_with(cx, |view, cx| view.directory.read(cx).value().to_string())
}

#[gpui_kit::test]
fn directory_picker_normal_result_changes_only_original_draft_and_cancel_keeps_disk(
    cx: &mut TestAppContext,
) {
    let f = PickerFixture::new(cx);
    let (view, mut surface) = f.settings(cx);
    let initial = f.config_directory();
    let id = f.click_picker(&mut surface);
    let selected = f.root.join("selected");
    f.reply(cx, id, picked(&selected));
    assert_eq!(directory(&view, cx), selected.to_string_lossy());
    assert_eq!(f.config_directory(), initial, "picker is not Apply");
    let edits = view.read_with(cx, |view, _| view.category_presentation_revision);
    f.reply(cx, id, picked(&f.root.join("duplicate")));
    assert_eq!(directory(&view, cx), selected.to_string_lossy());
    assert_eq!(
        view.read_with(cx, |view, _| view.category_presentation_revision),
        edits,
        "same result applies once"
    );
    surface.refresh().expect("Cancel frame");
    let cancel = surface
        .debug_bounds("download-settings-cancel")
        .expect("real Cancel");
    surface.simulate_click(cancel.center(), Modifiers::default());
    surface.run_until_parked();
    assert_eq!(f.config_directory(), initial);
    assert!(
        f.app
            .read_with(cx, |app, _| app.pending_directory_picker.is_none())
    );
}

#[gpui_kit::test]
fn directory_picker_closed_settings_result_cannot_write_reopened_draft(cx: &mut TestAppContext) {
    let f = PickerFixture::new(cx);
    let (old, mut old_surface) = f.settings(cx);
    let id = f.click_picker(&mut old_surface);
    assert!(old_surface.simulate_close());
    old_surface.run_until_parked();
    let (replacement, _) = f.settings(cx);
    assert_ne!(old.entity_id(), replacement.entity_id());
    let initial = directory(&replacement, cx);
    f.reply(cx, id, picked(&f.root.join("stale-window")));
    assert_eq!(
        directory(&replacement, cx),
        initial,
        "old native success must not target replacement"
    );
    assert!(
        f.app
            .read_with(cx, |app, _| app.pending_directory_picker.is_none())
    );
}

#[gpui_kit::test]
fn directory_picker_same_type_views_do_not_share_a_result(cx: &mut TestAppContext) {
    let f = PickerFixture::new(cx);
    let (original, mut surface) = f.settings(cx);
    let initial = f.app.read_with(cx, |app, _| app.settings_view_initial());
    let app = f.app.clone();
    let slot = Rc::new(RefCell::new(None));
    let output = slot.clone();
    let (_, extra_surface) = cx.add_window_view(move |window, cx| {
        let view = cx.new(|cx| DownloadSettingsView::new(app, initial, window, cx));
        *output.borrow_mut() = Some(view.clone());
        Root::new(view, window, cx)
    });
    extra_surface
        .refresh()
        .expect("second actual settings view");
    let extra = slot.borrow().clone().expect("second settings view");
    let initial_extra = directory(&extra, cx);
    let id = f.click_picker(&mut surface);
    let selected = f.root.join("original-only");
    f.reply(cx, id, picked(&selected));
    assert_eq!(directory(&original, cx), selected.to_string_lossy());
    assert_eq!(
        directory(&extra, cx),
        initial_extra,
        "type is not view identity"
    );
}

#[gpui_kit::test]
fn directory_picker_same_turn_aba_user_edit_rejects_old_result(cx: &mut TestAppContext) {
    let f = PickerFixture::new(cx);
    let (view, mut surface) = f.settings(cx);
    let id = f.click_picker(&mut surface);
    let original = directory(&view, cx);
    surface.update(|window, cx| {
        view.update(cx, |view, cx| {
            view.directory.update(cx, |input, cx| {
                input.replace_all("B", window, cx);
                input.replace_all(original.clone(), window, cx);
            });
        });
        f.app.update(cx, |app, cx| {
            app.handle_platform_notification(
                PlatformNotification::CommandResult {
                    correlation_id: id,
                    result: picked(&f.root.join("old-picker")),
                },
                cx,
            )
        });
    });
    surface.run_until_parked();
    assert_eq!(
        directory(&view, cx),
        original,
        "A->B->A must invalidate by revision, not text equality"
    );
    assert!(
        f.app
            .read_with(cx, |app, _| app.pending_directory_picker.is_none())
    );
}

#[gpui_kit::test]
fn directory_picker_result_then_edit_before_deferred_apply_keeps_new_draft(
    cx: &mut TestAppContext,
) {
    let f = PickerFixture::new(cx);
    let (view, mut surface) = f.settings(cx);
    let id = f.click_picker(&mut surface);
    surface.update(|window, cx| {
        f.app.update(cx, |app, cx| {
            app.handle_platform_notification(
                PlatformNotification::CommandResult {
                    correlation_id: id,
                    result: picked(&f.root.join("old-result")),
                },
                cx,
            )
        });
        view.update(cx, |view, cx| {
            view.directory.update(cx, |input, cx| {
                input.replace_all("new user draft", window, cx);
            })
        });
    });
    surface.run_until_parked();
    assert_eq!(
        directory(&view, cx),
        "new user draft",
        "final application must revalidate after result claim"
    );
    assert!(
        f.app
            .read_with(cx, |app, _| app.pending_directory_picker.is_none())
    );
}

#[gpui_kit::test]
fn directory_picker_result_claim_then_close_reopen_cannot_apply_or_fail_replacement(
    cx: &mut TestAppContext,
) {
    let f = PickerFixture::new(cx);
    let (old, mut surface) = f.settings(cx);
    let id = f.click_picker(&mut surface);
    let original = directory(&old, cx);
    surface.update(|window, cx| {
        f.app.update(cx, |app, cx| {
            app.handle_platform_notification(
                PlatformNotification::CommandResult {
                    correlation_id: id,
                    result: picked(&f.root.join("retired")),
                },
                cx,
            )
        });
        window.remove_window();
        f.app.update(cx, |app, cx| app.open_settings_window(cx));
    });
    cx.run_until_parked();
    let (replacement, mut replacement_surface) = f.settings(cx);
    assert_ne!(old.entity_id(), replacement.entity_id());
    assert_eq!(
        directory(&old, cx),
        original,
        "closed retained view is not updated either"
    );
    assert_eq!(directory(&replacement, cx), original);
    let new_id = f.click_picker(&mut replacement_surface);
    let notices = f.notices(cx);
    f.reply(
        cx,
        id,
        PlatformCommandResult {
            success: false,
            error: Some("old failure".into()),
            native_error_code: Some(5),
            data: None,
        },
    );
    assert_eq!(
        f.pending(cx),
        Some(new_id),
        "old failure cannot clear replacement request"
    );
    assert_eq!(
        f.notices(cx),
        notices,
        "old failure is not reported as a current error"
    );
    f.reply(cx, new_id, picked(&f.root.join("new")));
    assert_eq!(
        directory(&replacement, cx),
        f.root.join("new").to_string_lossy()
    );
}

#[gpui_kit::test]
fn directory_picker_cancel_failure_disconnect_retry_and_old_ids_are_isolated(
    cx: &mut TestAppContext,
) {
    let f = PickerFixture::new(cx);
    let (view, mut surface) = f.settings(cx);
    let initial = directory(&view, cx);
    let first = f.click_picker(&mut surface);
    let button = surface
        .debug_bounds("download-settings-pick-directory")
        .expect("picker");
    surface.simulate_click(button.center(), Modifiers::default());
    surface.run_until_parked();
    assert_eq!(f.pending(cx), Some(first));
    assert!(
        f.endpoint.try_receive_command_for_contract_test().is_err(),
        "repeated pending click emits no command"
    );
    f.reply(
        cx,
        first,
        PlatformCommandResult {
            success: true,
            error: None,
            native_error_code: None,
            data: Some(String::new()),
        },
    );
    assert_eq!(f.pending(cx), None);
    assert_eq!(directory(&view, cx), initial);
    let failed = f.click_picker(&mut surface);
    f.reply(
        cx,
        failed,
        PlatformCommandResult {
            success: false,
            error: Some("synthetic Windows failure".into()),
            native_error_code: Some(5),
            data: None,
        },
    );
    assert_eq!(f.pending(cx), None);
    assert_eq!(directory(&view, cx), initial);
    assert!(
        f.notices(cx)
            .contains(&format!("download-directory-picker-{failed}"))
    );
    let disconnected = f.click_picker(&mut surface);
    f.endpoint.set_connected_for_contract_test(false);
    f.app.update(cx, |app, cx| {
        app.handle_platform_notification(
            PlatformNotification::Disconnected {
                generation: 1,
                pending_command_ids: vec![disconnected],
            },
            cx,
        )
    });
    cx.run_until_parked();
    assert_eq!(f.pending(cx), None);
    f.endpoint.set_connected_for_contract_test(true);
    let current = f.click_picker(&mut surface);
    let notices = f.notices(cx);
    for old in [first, failed, disconnected] {
        f.reply(cx, old, picked(&f.root.join("late")));
        assert_eq!(f.pending(cx), Some(current));
        assert_eq!(directory(&view, cx), initial);
    }
    assert_eq!(f.notices(cx), notices);
    f.reply(cx, current, picked(&f.root.join("current")));
    assert_eq!(f.pending(cx), None);
    assert_eq!(
        directory(&view, cx),
        f.root.join("current").to_string_lossy()
    );
}

#[gpui_kit::test]
fn directory_picker_dispatch_rejection_releases_original_token_and_can_retry(
    cx: &mut TestAppContext,
) {
    let f = PickerFixture::new(cx);
    let (view, mut surface) = f.settings(cx);
    let initial = directory(&view, cx);
    f.drain_commands();
    let platform = f.app.read_with(cx, |app, _| app.platform.clone());
    for _ in 0..32 {
        platform
            .send_command(PlatformCommand::PickDirectory(PickDirectoryCommand {
                current_directory: String::new(),
                title: "synthetic capacity occupant".into(),
                source_window_handle: 0,
                source_process_id: std::process::id(),
            }))
            .expect("fill bounded test queue");
    }
    let button = surface
        .debug_bounds("download-settings-pick-directory")
        .expect("picker");
    surface.simulate_click(button.center(), Modifiers::default());
    surface.run_until_parked();
    assert_eq!(f.pending(cx), None, "failed send retires pending");
    assert_eq!(directory(&view, cx), initial);
    f.drain_commands();
    let retry = f.click_picker(&mut surface);
    f.reply(cx, retry, picked(&f.root.join("retry")));
    assert_eq!(directory(&view, cx), f.root.join("retry").to_string_lossy());
    f.endpoint.set_connected_for_contract_test(false);
    surface.refresh().expect("disconnected click frame");
    surface.simulate_click(button.center(), Modifiers::default());
    surface.run_until_parked();
    assert_eq!(f.pending(cx), None);
    f.endpoint.set_connected_for_contract_test(true);
    let connected_retry = f.click_picker(&mut surface);
    f.reply(cx, connected_retry, picked(&f.root.join("connected-retry")));
    assert_eq!(
        directory(&view, cx),
        f.root.join("connected-retry").to_string_lossy()
    );
}

#[gpui_kit::test]
fn directory_picker_new_task_settings_isolation_explicit_draft_and_apply_policy(
    cx: &mut TestAppContext,
) {
    let f = PickerFixture::new(cx);
    let (settings, mut settings_surface) = f.settings(cx);
    let (task, mut task_surface) = f.new_task(cx);
    let original_settings = directory(&settings, cx);
    let original_task = task.read_with(cx, |view, cx| view.directory.read(cx).value().to_string());
    let settings_id = f.click_picker(&mut settings_surface);
    let task_button = task_surface
        .update(|window, _| gpui_kit::base::test_support::snapshots(window))
        .into_iter()
        .find(|node| node.path().last() == Some(&"download-pick-directory".into()))
        .expect("task picker")
        .bounds();
    task_surface.simulate_click(task_button.center(), Modifiers::default());
    task_surface.run_until_parked();
    assert_eq!(
        f.pending(cx),
        Some(settings_id),
        "one existing picker owner across target types"
    );
    assert!(f.endpoint.try_receive_command_for_contract_test().is_err());
    let selected_settings = f.root.join("settings-selected");
    f.reply(cx, settings_id, picked(&selected_settings));
    assert_eq!(
        directory(&settings, cx),
        selected_settings.to_string_lossy()
    );
    assert_eq!(
        task.read_with(cx, |view, cx| view.directory.read(cx).value().to_string()),
        original_task
    );
    let task_id = f.click_new_picker(&mut task_surface);
    let task_selected = f.root.join("task-selected");
    f.reply(cx, task_id, picked(&task_selected));
    task.read_with(cx, |view, cx| {
        assert_eq!(
            view.directory.read(cx).value().as_str(),
            task_selected.to_string_lossy()
        );
        assert!(view.save_directory_explicit);
        assert!(view.error.is_none());
    });
    assert_eq!(
        directory(&settings, cx),
        selected_settings.to_string_lossy()
    );
    assert_eq!(
        f.config_directory(),
        original_settings,
        "neither picker autosaves"
    );
    settings_surface.refresh().expect("Apply actual footer");
    let apply = settings_surface
        .debug_bounds("download-settings-apply")
        .expect("real Apply");
    settings_surface.simulate_click(apply.center(), Modifiers::default());
    settings_surface.run_until_parked();
    assert_eq!(f.config_directory(), selected_settings.to_string_lossy());
    assert!(
        !AppConfig::load_from(&f.config_path)
            .expect("merged config")
            .quick_launch
            .enabled
    );
    assert!(f.app.read_with(cx, |app, _| app.tasks().is_empty()));
}

#[gpui_kit::test]
fn directory_picker_new_task_aba_edit_close_reopen_and_silent_hydration(cx: &mut TestAppContext) {
    let f = PickerFixture::new(cx);
    let (view, mut surface) = f.new_task(cx);
    let first = f.click_new_picker(&mut surface);
    let original = view.read_with(cx, |view, cx| view.directory.read(cx).value().to_string());
    surface.update(|window, cx| {
        f.app.update(cx, |app, cx| {
            app.handle_platform_notification(
                PlatformNotification::CommandResult {
                    correlation_id: first,
                    result: picked(&f.root.join("old")),
                },
                cx,
            )
        });
        view.update(cx, |view, cx| {
            view.directory.update(cx, |input, cx| {
                input.replace_all("B", window, cx);
                input.replace_all(original.clone(), window, cx);
            })
        });
    });
    surface.run_until_parked();
    assert_eq!(
        view.read_with(cx, |view, cx| view.directory.read(cx).value().to_string()),
        original
    );
    let id = f.click_new_picker(&mut surface);
    surface.update(|window, cx| {
        view.update(cx, |view, cx| {
            view.directory.update(cx, |input, cx| {
                input.set_value(original.clone(), window, cx)
            });
        })
    });
    f.reply(cx, id, picked(&f.root.join("normal")));
    assert_eq!(
        view.read_with(cx, |view, cx| view.directory.read(cx).value().to_string()),
        f.root.join("normal").to_string_lossy()
    );
    let stale = f.click_new_picker(&mut surface);
    surface.update(|window, cx| {
        f.app.update(cx, |app, cx| {
            app.handle_platform_notification(
                PlatformNotification::CommandResult {
                    correlation_id: stale,
                    result: picked(&f.root.join("closed")),
                },
                cx,
            )
        });
        window.remove_window();
        f.app.update(cx, |app, cx| app.open_new_download_window(cx));
    });
    cx.run_until_parked();
    let (replacement, mut replacement_surface) = f.new_task(cx);
    assert_ne!(view.entity_id(), replacement.entity_id());
    assert_eq!(
        replacement.read_with(cx, |view, cx| view.directory.read(cx).value().to_string()),
        original
    );
    let current = f.click_new_picker(&mut replacement_surface);
    f.reply(cx, stale, picked(&f.root.join("late")));
    assert_eq!(f.pending(cx), Some(current));
    f.reply(cx, current, picked(&f.root.join("fresh")));
    assert_eq!(
        replacement.read_with(cx, |view, cx| view.directory.read(cx).value().to_string()),
        f.root.join("fresh").to_string_lossy()
    );
}

#[gpui_kit::test]
fn directory_picker_old_settings_close_callback_preserves_current_window_and_request(
    cx: &mut TestAppContext,
) {
    let f = PickerFixture::new(cx);
    let (_, mut old_surface) = f.settings(cx);
    let old_handle = old_surface.update(|window, _| window.window_handle());
    old_surface.update(|window, _| window.remove_window());
    cx.run_until_parked();
    let (view, mut surface) = f.settings(cx);
    let current_handle = surface.update(|window, _| window.window_handle());
    let current = f.click_picker(&mut surface);
    f.app.update(cx, |app, cx| {
        app.handle_native_settings_window_closed(old_handle, cx)
    });
    assert_eq!(f.pending(cx), Some(current));
    assert_eq!(
        f.app.read_with(cx, |app, _| app.settings_window),
        Some(current_handle)
    );
    f.reply(cx, current, picked(&f.root.join("still-current")));
    assert_eq!(
        directory(&view, cx),
        f.root.join("still-current").to_string_lossy()
    );
}

#[gpui_kit::test]
fn directory_picker_result_claim_then_same_turn_aba_is_not_applied(cx: &mut TestAppContext) {
    let f = PickerFixture::new(cx);
    let (view, mut surface) = f.settings(cx);
    let id = f.click_picker(&mut surface);
    let original = directory(&view, cx);
    surface.update(|window, cx| {
        f.app.update(cx, |app, cx| {
            app.handle_platform_notification(
                PlatformNotification::CommandResult {
                    correlation_id: id,
                    result: picked(&f.root.join("before-aba")),
                },
                cx,
            )
        });
        view.update(cx, |view, cx| {
            view.directory.update(cx, |input, cx| {
                input.replace_all("intermediate", window, cx);
                input.replace_all(original.clone(), window, cx);
            })
        });
    });
    surface.run_until_parked();
    assert_eq!(
        directory(&view, cx),
        original,
        "already-claimed result cannot precede queued Change revisions"
    );
    assert_eq!(f.pending(cx), None);
    let retry = f.click_picker(&mut surface);
    f.reply(cx, retry, picked(&f.root.join("retry-after-aba")));
    assert_eq!(
        directory(&view, cx),
        f.root.join("retry-after-aba").to_string_lossy()
    );
}

#[gpui_kit::test]
fn directory_picker_failure_claim_then_closed_origin_has_no_current_notice(
    cx: &mut TestAppContext,
) {
    let f = PickerFixture::new(cx);
    let (_, mut surface) = f.settings(cx);
    let id = f.click_picker(&mut surface);
    let notices = f.notices(cx);
    surface.update(|window, cx| {
        f.app.update(cx, |app, cx| {
            app.handle_platform_notification(
                PlatformNotification::CommandResult {
                    correlation_id: id,
                    result: PlatformCommandResult {
                        success: false,
                        error: Some("retired native error".into()),
                        native_error_code: Some(5),
                        data: None,
                    },
                },
                cx,
            )
        });
        window.remove_window();
        f.app.update(cx, |app, cx| app.open_settings_window(cx));
    });
    cx.run_until_parked();
    assert_eq!(
        f.notices(cx),
        notices,
        "failure needs final origin validation too"
    );
    assert_eq!(f.pending(cx), None);
}

#[gpui_kit::test]
fn directory_picker_fixed_confirmation_replacement_retires_original_view_request(
    cx: &mut TestAppContext,
) {
    let f = PickerFixture::new(cx);
    let (source_view, mut source) = f.new_task(cx);
    let id = f.click_new_picker(&mut source);
    let source_handle = source.update(|window, _| window.window_handle());
    let original =
        source_view.read_with(cx, |view, cx| view.directory.read(cx).value().to_string());
    source_view.update_in(&mut source, |view, _, _| {
        view.handoff_staged = true;
        view.handoff_pending = true;
        view.probe_generation = 7;
    });
    f.app.update(cx, |app, cx| {
        app.create_fixed_confirmation(
            source_handle,
            source_view.downgrade(),
            7,
            uuid::Uuid::new_v4(),
            cx,
        )
    });
    cx.run_until_parked();
    let (target, target_handle) = f.app.read_with(cx, |app, _| {
        (
            app.new_download_view
                .clone()
                .expect("actual handoff target view"),
            app.new_download_window
                .expect("actual handoff target window"),
        )
    });
    assert_ne!(target_handle, source_handle);
    assert_ne!(target.entity_id(), source_view.entity_id());
    assert_eq!(
        f.pending(cx),
        None,
        "source removal retires the pending picker"
    );
    f.reply(cx, id, picked(&f.root.join("source-only-late")));
    assert_eq!(
        target.read_with(cx, |view, cx| view.directory.read(cx).value().to_string()),
        original
    );
    assert_eq!(
        source_view.read_with(cx, |view, cx| view.directory.read(cx).value().to_string()),
        original
    );
}

#[gpui_kit::test]
fn browser_export_owner_close_retires_selection_before_a_reopened_draft(cx: &mut TestAppContext) {
    let f = PickerFixture::new(cx);
    let (view, mut surface) = f.settings(cx);
    let source = f.app.read_with(cx, |app, _| {
        app.settings_window.expect("source settings window")
    });
    let operation = uuid::Uuid::new_v4();
    f.app
        .update(cx, |app, _| {
            app.request_browser_extension_export("chrome", operation, source, view.downgrade(), 0)
        })
        .expect("selection accepted");
    f.drain_commands();
    f.app.update(cx, |app, cx| {
        app.handle_native_settings_window_closed(source, cx)
    });
    surface.update(|window, _| window.remove_window());
    let (_replacement, mut replacement_surface) = f.settings(cx);
    assert!(
        f.app
            .read_with(cx, |app, _| app.pending_extension_export.is_none())
    );
    f.reply(
        cx,
        operation,
        PlatformCommandResult {
            success: true,
            error: None,
            native_error_code: None,
            data: Some("C:\\must-not-export.zip".into()),
        },
    );
    assert!(
        !std::iter::from_fn(|| f.endpoint.try_receive_command_for_contract_test().ok()).any(
            |command| matches!(
                command.message,
                nxtools_protocol::Message::PlatformCommand(
                    PlatformCommand::ExportBrowserExtension(_)
                )
            )
        ),
        "retired selection cannot start machine work"
    );
    replacement_surface.update(|window, _| window.remove_window());
}

#[gpui_kit::test]
fn browser_export_selection_wait_has_no_feature_deadline_and_cancel_is_quiet(
    cx: &mut TestAppContext,
) {
    let f = PickerFixture::new(cx);
    let (view, mut surface) = f.settings(cx);
    let source = f.app.read_with(cx, |app, _| {
        app.settings_window.expect("source settings window")
    });
    let operation = uuid::Uuid::new_v4();
    f.app
        .update(cx, |app, _| {
            app.request_browser_extension_export("chrome", operation, source, view.downgrade(), 0)
        })
        .expect("selection accepted");
    f.drain_commands();
    cx.executor().advance_clock(Duration::from_secs(11 * 60));
    cx.run_until_parked();
    assert!(f.app.read_with(cx, |app, _| {
        app.pending_extension_export
            .as_ref()
            .is_some_and(|pending| pending.command_id == operation)
    }));
    f.reply(
        cx,
        operation,
        PlatformCommandResult {
            success: true,
            error: None,
            native_error_code: None,
            data: Some(String::new()),
        },
    );
    cx.run_until_parked();
    assert!(f.app.read_with(cx, |app, cx| {
        app.pending_extension_export.is_none()
            && app
                .notifications
                .read(cx)
                .items()
                .iter()
                .all(|notice| notice.level != crate::notifications::NotificationLevel::Error)
    }));
    assert!(
        !std::iter::from_fn(|| f.endpoint.try_receive_command_for_contract_test().ok()).any(
            |command| matches!(
                command.message,
                nxtools_protocol::Message::PlatformCommand(
                    PlatformCommand::ExportBrowserExtension(_)
                )
            )
        )
    );
    surface.update(|window, _| window.remove_window());
}

fn request_ffmpeg_picker(f: &PickerFixture, cx: &mut TestAppContext) -> uuid::Uuid {
    let view = f.app.read_with(cx, |app, _| {
        app.settings_view.clone().expect("settings draft")
    });
    let view_id = view.entity_id();
    let origin = view.read_with(cx, |view, _| FfmpegPickerOrigin {
        view_id,
        edit_generation: view.ffmpeg_discovery_generation,
    });
    f.app.update(cx, |app, cx| {
        app.request_ffmpeg_executable(String::new(), 0, Some(origin), cx);
        app.ffmpeg_platform_commands
            .borrow()
            .last()
            .expect("FFmpeg picker command")
            .0
    })
}

#[gpui_kit::test]
fn ffmpeg_picker_late_result_preserves_an_edited_draft(cx: &mut TestAppContext) {
    let f = PickerFixture::new(cx);
    let (view, mut surface) = f.settings(cx);
    let id = request_ffmpeg_picker(&f, cx);
    let input = view.read_with(cx, |view, _| view.ffmpeg_executable.clone());
    surface.update(|window, cx| {
        input.update(cx, |input, cx| {
            input.set_value("C:\\edited\\ffmpeg.exe", window, cx);
            cx.emit(gpui_kit::component::input::InputEvent::Change);
        });
    });
    cx.run_until_parked();
    f.reply(cx, id, picked(&f.root.join("late-ffmpeg.exe")));
    assert_eq!(
        input.read_with(cx, |input, _| input.value().to_string()),
        "C:\\edited\\ffmpeg.exe",
        "a selection cannot overwrite a newer field edit"
    );
    surface.update(|window, _| window.remove_window());
}

#[gpui_kit::test]
fn ffmpeg_picker_owner_close_retires_selection_before_a_reopened_draft(cx: &mut TestAppContext) {
    let f = PickerFixture::new(cx);
    let (_view, mut surface) = f.settings(cx);
    let id = request_ffmpeg_picker(&f, cx);
    let source = f
        .app
        .read_with(cx, |app, _| app.settings_window.expect("source settings"));
    f.app.update(cx, |app, cx| {
        app.handle_native_settings_window_closed(source, cx)
    });
    surface.update(|window, _| window.remove_window());
    let (replacement, mut replacement_surface) = f.settings(cx);
    assert!(f.app.read_with(cx, |app, _| {
        !app.pending_ffmpeg_operations
            .values()
            .any(|pending| pending.kind == FfmpegPlatformOperationKind::Picker)
    }));
    f.reply(cx, id, picked(&f.root.join("old-draft-ffmpeg.exe")));
    assert_eq!(
        replacement.read_with(cx, |view, cx| view
            .ffmpeg_executable
            .read(cx)
            .value()
            .to_string()),
        ""
    );
    let replacement_id = request_ffmpeg_picker(&f, cx);
    assert_ne!(
        replacement_id, id,
        "a replacement draft can begin its own picker"
    );
    let selected = f.root.join("current-ffmpeg.exe");
    f.reply(cx, replacement_id, picked(&selected));
    assert_eq!(
        replacement.read_with(cx, |view, cx| view
            .ffmpeg_executable
            .read(cx)
            .value()
            .to_string()),
        selected.to_string_lossy()
    );
    replacement_surface.update(|window, _| window.remove_window());
}
