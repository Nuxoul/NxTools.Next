//! Tests for the parent module. Extracted from the parent file so the
//! runtime code stays readable.

use super::polish_tests::{
    official_download_group_bounds, official_download_navigation_bounds, reveal_download_setting,
    select_official_download_page,
};
use super::*;

#[test]
fn task_icon_requests_use_a_bounded_inflight_budget_and_retry_window() {
    assert_eq!(DOWNLOAD_TASK_ICON_MAX_IN_FLIGHT, 8);
    assert_eq!(DOWNLOAD_TASK_ICON_RETRY_DELAY, Duration::from_secs(5));
}

#[test]
fn category_search_matches_name_directory_and_extension_without_unbounded_metadata() {
    let category = DownloadCategorySettings {
        id: "archives".into(),
        name: "压缩包".into(),
        enabled: true,
        save_directory: r"Downloads\Archive".into(),
        extensions: vec!["zip".into(), "7z".into()],
        url_patterns: Vec::new(),
    };
    assert!(category_matches_query(&category, "压缩"));
    assert!(category_matches_query(&category, "archive"));
    assert!(category_matches_query(&category, "7z"));
    assert!(!category_matches_query(&category, "video"));
}

#[test]
fn category_keyboard_highlight_is_visually_distinct_from_committed_selection() {
    let source = include_str!("new_download_view.rs");
    assert!(
        source.contains(".selected(highlighted)")
            && source.contains(".secondary_selected(selected)"),
        "actual ListItem distinguishes keyboard highlight from committed selection"
    );
    assert!(
        !source.contains("category_choice_background("),
        "retired custom palette does not paint category selection"
    );
}

use gpui_kit::{AppContext, Modifiers, TestAppContext, VisualTestContext};

#[gpui_kit::test]
fn new_download_terminal_failure_keeps_confirmed_draft_without_inline_notice(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_kit::init);
    let (application, directory) = download_application_for_window_test(cx, "terminal-feedback");
    application.update(cx, |app, cx| app.open_new_download_window(cx));
    cx.run_until_parked();
    let (handle, view) = cx.read(|cx| {
        let app = application.read(cx);
        (
            app.new_download_window.expect("production window"),
            app.new_download_view.clone().expect("production view"),
        )
    });
    let mut surface = VisualTestContext::from_window(handle, cx);
    let (input, idempotency) = surface.read(|cx| {
        (
            view.read(cx).url.entity_id(),
            view.read(cx).submission_idempotency_key,
        )
    });
    view.update_in(&mut surface, |view, _, cx| {
        view.submitting = true;
        view.finish_submission_failure("合成引擎拒绝；没有启动网络任务".into(), cx);
        assert!(!view.submitting);
        assert!(
            view.error.is_none(),
            "terminal operation feedback is not form validation"
        );
        assert_eq!(view.url.entity_id(), input);
        assert_eq!(view.submission_idempotency_key, idempotency);
    });
    view.update_in(&mut surface, |view, _, cx| {
        view.probing = true;
        view.finish_probe_failure("合成探测通道失败".into(), cx);
        assert!(!view.probing);
        assert!(
            view.error.is_none(),
            "remote probe failure is not input validation"
        );
    });
    surface.run_until_parked();
    application.read_with(&surface, |app, cx| {
        assert!(
            app.new_download_window.is_some(),
            "retry retains source window"
        );
        assert!(
            app.notifications
                .read(cx)
                .items()
                .iter()
                .any(|notice| notice.level == crate::notifications::NotificationLevel::Error)
        );
    });
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn multiline_header_placeholder_stays_on_one_line_for_windows_directwrite() {
    assert!(!DEFAULT_REQUEST_HEADERS_PLACEHOLDER.contains(['\r', '\n']));
}

#[gpui_kit::test]
fn download_task_icons_use_the_typed_shell_boundary_and_publish_by_task_id(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_kit::init);
    let (application, directory, platform_endpoint) =
        download_application_for_window_test_with_platform(cx, "task-shell-icons");
    platform_endpoint.set_connected_for_contract_test(true);
    let (task_id, sibling_task_id) = cx.update(|cx| {
        application.update(cx, |application, cx| {
            let task_id = application
                .engine
                .enqueue(
                    DownloadRequest {
                        source_url: "https://example.test/archive.zip".into(),
                        save_directory: directory.clone(),
                        save_directory_explicit: false,
                        file_name: Some("archive.zip".into()),
                        headers: Vec::new(),
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
                .expect("enqueue icon fixture task");
            let sibling_task_id = application
                .engine
                .enqueue(
                    DownloadRequest {
                        source_url: "https://example.test/second.zip".into(),
                        save_directory: directory.clone(),
                        save_directory_explicit: false,
                        file_name: Some("second.zip".into()),
                        headers: Vec::new(),
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
                .expect("enqueue sibling icon fixture task");
            application.refresh_snapshot(false, cx);
            (task_id, sibling_task_id)
        })
    });

    let envelope = platform_endpoint
        .try_receive_command_for_contract_test()
        .expect("typed Shell icon command");
    match &envelope.message {
        nxtools_protocol::Message::PlatformCommand(PlatformCommand::ExtractShellIcon(
            ExtractShellIconCommand { target, size },
        )) => {
            assert_eq!(target, "archive.zip");
            assert_eq!(*size, DOWNLOAD_TASK_ICON_SIZE);
        }
        message => panic!("unexpected platform message: {message:?}"),
    }
    assert!(
        platform_endpoint
            .try_receive_command_for_contract_test()
            .is_err(),
        "tasks with the same extension must share one bounded Shell request"
    );
    let icon_path = directory.join("archive-icon.png");
    std::fs::write(&icon_path, crate::icons::ONE_PIXEL_PNG).expect("write Shell icon result");
    cx.update(|cx| {
        application.update(cx, |application, cx| {
            application.handle_platform_notification(
                PlatformNotification::CommandResult {
                    correlation_id: envelope.id,
                    result: PlatformCommandResult {
                        success: true,
                        error: None,
                        native_error_code: None,
                        data: Some(icon_path.to_string_lossy().into_owned()),
                    },
                },
                cx,
            );
        });
    });
    assert_eq!(
        cx.read(|cx| {
            application
                .read(cx)
                .task_icon_path(task_id)
                .map(Path::to_path_buf)
        }),
        Some(icon_path.clone())
    );
    assert_eq!(
        cx.read(|cx| {
            application
                .read(cx)
                .task_icon_path(sibling_task_id)
                .map(Path::to_path_buf)
        }),
        Some(icon_path)
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[gpui_kit::test]
fn stale_progress_envelopes_are_rejected_before_touching_cached_progress(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (application, _directory) = download_application_for_window_test(cx, "stale-progress");
    let task_id = DownloadTaskId::new();
    let cached = DownloadProgress {
        task_id,
        downloaded_bytes: 12,
        total_bytes: Some(100),
        bytes_per_second: 4,
        active_connections: 1,
        segments: Vec::new(),
    };
    let stale = DownloadProgress {
        downloaded_bytes: 4,
        ..cached.clone()
    };
    cx.update(|cx| {
        application.update(cx, |application, cx| {
            application.event_generation = 7;
            application.last_event_sequence = 10;
            application.latest_progress.insert(task_id, cached.clone());
            application.handle_engine_event(
                DownloadEventEnvelope {
                    generation: 7,
                    sequence: 9,
                    emitted_at_unix_ms: 0,
                    event: DownloadEvent::TaskProgress { progress: stale },
                },
                cx,
            );
            assert_eq!(application.progress(task_id), Some(&cached));
            assert_eq!(application.last_event_sequence, 10);
        });
    });
}

#[gpui_kit::test]
fn authoritative_snapshot_refresh_discards_live_progress_cache(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (application, _directory) = download_application_for_window_test(cx, "snapshot-progress");
    let task_id = DownloadTaskId::new();
    cx.update(|cx| {
        application.update(cx, |application, cx| {
            application.latest_progress.insert(
                task_id,
                DownloadProgress {
                    task_id,
                    downloaded_bytes: 8,
                    total_bytes: Some(10),
                    bytes_per_second: 2,
                    active_connections: 1,
                    segments: Vec::new(),
                },
            );
            application.refresh_snapshot(false, cx);
            assert!(application.latest_progress.is_empty());
        });
    });
}

#[test]
fn live_progress_updates_the_cached_selected_segment_without_requerying_sqlite() {
    let engine = DownloadEngine::open_in_memory().expect("open engine");
    let task_id = engine
        .enqueue(
            DownloadRequest {
                source_url: "https://example.test/live.bin".into(),
                save_directory: std::env::temp_dir(),
                save_directory_explicit: false,
                file_name: Some("live.bin".into()),
                headers: Vec::new(),
                cookie: None,
                referrer: None,
                certificate_policy: Default::default(),
                connection_limit: Some(2),
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
        .expect("enqueue task");
    let task = engine
        .task(task_id)
        .expect("read task")
        .expect("task exists");
    let mut detail = DownloadTaskDetail {
        task,
        segments: vec![
            DownloadSegment::new(0, 0, 9, 0, DownloadSegmentState::Waiting).expect("segment"),
        ],
        timeline: Vec::new(),
    };
    let progress = DownloadProgress {
        task_id,
        downloaded_bytes: 6,
        total_bytes: Some(10),
        bytes_per_second: 1_024,
        active_connections: 1,
        segments: vec![nxtools_download::DownloadSegmentProgress {
            segment_index: 0,
            downloaded_bytes: 6,
            total_bytes: Some(10),
        }],
    };

    apply_progress_to_task_detail(&mut detail, &progress);

    assert_eq!(detail.task.downloaded_bytes, 6);
    assert_eq!(detail.segments[0].downloaded_bytes(), 6);
    assert_eq!(detail.segments[0].state, DownloadSegmentState::Active);
}

#[test]
fn start_now_closes_the_add_window_before_scheduling_the_progress_window() {
    let task_id = DownloadTaskId::new();

    assert_eq!(
        submit_presentation_plan(DownloadSubmitMode::StartNow, task_id, false, true),
        DownloadSubmitPresentationPlan {
            close_source_window: true,
            deferred_task_window: Some(task_id),
        }
    );
}

#[test]
fn disabled_automatic_progress_window_still_closes_the_add_window() {
    let task_id = DownloadTaskId::new();

    assert_eq!(
        submit_presentation_plan(DownloadSubmitMode::StartNow, task_id, false, false),
        DownloadSubmitPresentationPlan {
            close_source_window: true,
            deferred_task_window: None,
        }
    );
}

#[test]
fn download_settings_validation_uses_real_engine_ranges() {
    let draft = parse_download_settings_draft(DownloadSettingsInput {
        default_directory: r"C:\Downloads",
        connection_limit: "8",
        adaptive_segment_scaling_enabled: true,
        adaptive_segment_max_segments: "32",
        adaptive_segment_min_mib: "3",
        max_concurrent_tasks: "3",
        speed_limit_kib: "2048",
        show_progress_window: true,
        show_completion_window: false,
        completion_sound_enabled: true,
        open_directory_on_completion: false,
        task_removal_deletes_file: true,
        taskbar_overlay_enabled: true,
        taskbar_overlay_anchor: DownloadTaskbarOverlayAnchor::Right,
        taskbar_overlay_offset_x: "12",
        taskbar_overlay_offset_y: "0",
        taskbar_overlay_width: "132",
        taskbar_overlay_height: "8",
        taskbar_overlay_progress_mode: DownloadTaskbarProgressMode::PerTask,
        taskbar_overlay_show_progress: true,
        taskbar_overlay_show_speed: false,
        taskbar_overlay_show_remaining_time: false,
        taskbar_overlay_show_active_tasks: false,
        taskbar_overlay_show_active_connections: false,
        taskbar_overlay_show_transferred_bytes: false,
    })
    .expect("valid download settings");

    assert_eq!(draft.default_connection_limit, 8);
    assert_eq!(draft.max_concurrent_tasks, 3);
    assert_eq!(
        draft.global_speed_limit_bytes_per_second,
        Some(2 * 1024 * 1024)
    );
    assert!(draft.show_progress_window);
    assert!(!draft.show_completion_window);
    assert!(draft.completion_sound_enabled);
    assert!(!draft.open_directory_on_completion);
    assert_eq!(draft.taskbar_progress_overlay.width, 132);
    assert_eq!(draft.taskbar_progress_overlay.height, 8);
    assert_eq!(
        draft.taskbar_progress_overlay.progress_mode,
        DownloadTaskbarProgressMode::PerTask
    );
    assert!(
        parse_download_settings_draft(DownloadSettingsInput {
            default_directory: r"C:\Downloads",
            connection_limit: "65",
            adaptive_segment_scaling_enabled: true,
            adaptive_segment_max_segments: "65",
            adaptive_segment_min_mib: "3",
            max_concurrent_tasks: "3",
            speed_limit_kib: "0",
            show_progress_window: true,
            show_completion_window: true,
            completion_sound_enabled: true,
            open_directory_on_completion: false,
            task_removal_deletes_file: true,
            taskbar_overlay_enabled: true,
            taskbar_overlay_anchor: DownloadTaskbarOverlayAnchor::Right,
            taskbar_overlay_offset_x: "12",
            taskbar_overlay_offset_y: "0",
            taskbar_overlay_width: "132",
            taskbar_overlay_height: "8",
            taskbar_overlay_progress_mode: DownloadTaskbarProgressMode::PerTask,
            taskbar_overlay_show_progress: true,
            taskbar_overlay_show_speed: false,
            taskbar_overlay_show_remaining_time: false,
            taskbar_overlay_show_active_tasks: false,
            taskbar_overlay_show_active_connections: false,
            taskbar_overlay_show_transferred_bytes: false,
        })
        .is_err()
    );
    assert!(
        parse_download_settings_draft(DownloadSettingsInput {
            default_directory: r"C:\Downloads",
            connection_limit: "8",
            adaptive_segment_scaling_enabled: true,
            adaptive_segment_max_segments: "32",
            adaptive_segment_min_mib: "3",
            max_concurrent_tasks: "0",
            speed_limit_kib: "0",
            show_progress_window: true,
            show_completion_window: true,
            completion_sound_enabled: true,
            open_directory_on_completion: false,
            task_removal_deletes_file: true,
            taskbar_overlay_enabled: true,
            taskbar_overlay_anchor: DownloadTaskbarOverlayAnchor::Right,
            taskbar_overlay_offset_x: "12",
            taskbar_overlay_offset_y: "0",
            taskbar_overlay_width: "132",
            taskbar_overlay_height: "8",
            taskbar_overlay_progress_mode: DownloadTaskbarProgressMode::PerTask,
            taskbar_overlay_show_progress: true,
            taskbar_overlay_show_speed: false,
            taskbar_overlay_show_remaining_time: false,
            taskbar_overlay_show_active_tasks: false,
            taskbar_overlay_show_active_connections: false,
            taskbar_overlay_show_transferred_bytes: false,
        })
        .is_err()
    );
    assert!(
        parse_download_settings_draft(DownloadSettingsInput {
            default_directory: r"C:\Downloads",
            connection_limit: "8",
            adaptive_segment_scaling_enabled: true,
            adaptive_segment_max_segments: "7",
            adaptive_segment_min_mib: "3",
            max_concurrent_tasks: "3",
            speed_limit_kib: "0",
            show_progress_window: true,
            show_completion_window: true,
            completion_sound_enabled: true,
            open_directory_on_completion: false,
            task_removal_deletes_file: true,
            taskbar_overlay_enabled: true,
            taskbar_overlay_anchor: DownloadTaskbarOverlayAnchor::Right,
            taskbar_overlay_offset_x: "12",
            taskbar_overlay_offset_y: "0",
            taskbar_overlay_width: "132",
            taskbar_overlay_height: "8",
            taskbar_overlay_progress_mode: DownloadTaskbarProgressMode::PerTask,
            taskbar_overlay_show_progress: true,
            taskbar_overlay_show_speed: false,
            taskbar_overlay_show_remaining_time: false,
            taskbar_overlay_show_active_tasks: false,
            taskbar_overlay_show_active_connections: false,
            taskbar_overlay_show_transferred_bytes: false,
        })
        .is_err(),
        "adaptive segment ceiling cannot be below the initial connection count"
    );
    assert!(
        parse_download_settings_draft(DownloadSettingsInput {
            default_directory: r"C:\Downloads",
            connection_limit: "8",
            adaptive_segment_scaling_enabled: true,
            adaptive_segment_max_segments: "32",
            adaptive_segment_min_mib: "0",
            max_concurrent_tasks: "3",
            speed_limit_kib: "0",
            show_progress_window: true,
            show_completion_window: true,
            completion_sound_enabled: true,
            open_directory_on_completion: false,
            task_removal_deletes_file: true,
            taskbar_overlay_enabled: true,
            taskbar_overlay_anchor: DownloadTaskbarOverlayAnchor::Right,
            taskbar_overlay_offset_x: "12",
            taskbar_overlay_offset_y: "0",
            taskbar_overlay_width: "132",
            taskbar_overlay_height: "8",
            taskbar_overlay_progress_mode: DownloadTaskbarProgressMode::PerTask,
            taskbar_overlay_show_progress: true,
            taskbar_overlay_show_speed: false,
            taskbar_overlay_show_remaining_time: false,
            taskbar_overlay_show_active_tasks: false,
            taskbar_overlay_show_active_connections: false,
            taskbar_overlay_show_transferred_bytes: false,
        })
        .is_err(),
        "adaptive split tail threshold cannot be zero"
    );
}

fn unique_download_settings_test_directory(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "nxtools-download-settings-{label}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .expect("system time")
            .as_nanos()
    ))
}

#[test]
fn completed_reveal_uses_existing_file_directory_or_save_directory_fallback() {
    let directory = unique_download_settings_test_directory("artifact-reveal-target");
    std::fs::create_dir_all(&directory).expect("create artifact test directory");
    let file = directory.join("final.bin");
    std::fs::write(&file, b"done").expect("create final artifact");
    let missing = directory.join("removed.bin");

    assert_eq!(
        resolve_download_artifact_target(
            Some(&file),
            &directory,
            ShellOpenAction::SelectInExplorer,
        ),
        Ok((file.clone(), false))
    );
    assert_eq!(
        resolve_download_artifact_target(
            Some(&directory),
            &directory,
            ShellOpenAction::SelectInExplorer,
        ),
        Ok((directory.clone(), false))
    );
    assert_eq!(
        resolve_download_artifact_target(
            Some(&missing),
            &directory,
            ShellOpenAction::SelectInExplorer,
        ),
        Ok((directory.clone(), true))
    );
    assert!(
        resolve_download_artifact_target(Some(&missing), &directory, ShellOpenAction::Open,)
            .is_err()
    );
    assert!(
        resolve_download_artifact_target(
            Some(&missing),
            &directory,
            ShellOpenAction::SelectInExplorer,
        )
        .expect("Reveal falls back only to the save directory")
        .0
        .is_dir()
    );

    assert!(
        resolve_download_artifact_target(Some(&directory), &directory, ShellOpenAction::OpenWith)
            .is_err()
    );
    assert_eq!(
        resolve_download_artifact_target(Some(&file), &directory, ShellOpenAction::OpenWith),
        Ok((file.clone(), false))
    );
    let missing_directory = directory.join("missing-directory");
    assert!(
        resolve_download_artifact_target(
            None,
            &missing_directory,
            ShellOpenAction::SelectInExplorer
        )
        .is_err()
    );
    assert!(
        !missing_directory.exists(),
        "opening a folder must not create it"
    );
    let _ = std::fs::remove_dir_all(directory);
}
#[gpui_kit::test]
fn taskbar_overlay_window_opens_without_reentering_download_application(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (application, directory) = download_application_for_window_test(cx, "taskbar-overlay");

    cx.update(|cx| {
        application.update(cx, |application, cx| {
            application.open_taskbar_overlay(cx);
        });
    });

    assert!(cx.read(|cx| {
        application
            .read(cx)
            .taskbar_overlay_window
            .as_ref()
            .is_some()
    }));
    let _ = std::fs::remove_dir_all(directory);
}

#[gpui_kit::test]
fn stale_native_taskbar_overlay_close_cannot_cancel_the_current_edit(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (application, directory) =
        download_application_for_window_test(cx, "taskbar-overlay-stale-close");
    cx.update(|cx| {
        application.update(cx, |application, cx| {
            application.open_taskbar_overlay(cx);
            application.open_schedule_window(cx);
        });
    });
    cx.run_until_parked();
    let (overlay_window, stale_window) = cx.read(|cx| {
        let application = application.read(cx);
        (
            application
                .taskbar_overlay_window
                .expect("current taskbar overlay window"),
            application
                .schedule_window
                .expect("stale comparison window"),
        )
    });
    let original_correlation_id = uuid::Uuid::new_v4();
    let replacement_correlation_id = uuid::Uuid::new_v4();
    cx.update(|cx| {
        application.update(cx, |application, cx| {
            application.taskbar_overlay_edit_draft =
                Some(DownloadTaskbarProgressOverlaySettings::default());
            application.taskbar_overlay_edit_correlation_id = Some(original_correlation_id);
            assert_eq!(
                application.taskbar_overlay_native_close_intent(stale_window),
                None
            );
            let stale_intent = application
                .taskbar_overlay_native_close_intent(overlay_window)
                .expect("current edit close intent");
            application.handle_native_taskbar_overlay_close(stale_window, stale_intent, cx);
            assert!(application.taskbar_overlay_edit_draft.is_some());

            application.taskbar_overlay_edit_correlation_id = Some(replacement_correlation_id);
            application.handle_native_taskbar_overlay_close(overlay_window, stale_intent, cx);
            assert!(application.taskbar_overlay_edit_draft.is_some());
            assert_eq!(
                application.taskbar_overlay_edit_correlation_id,
                Some(replacement_correlation_id)
            );

            let current_intent = application
                .taskbar_overlay_native_close_intent(overlay_window)
                .expect("replacement edit close intent");
            application.handle_native_taskbar_overlay_close(overlay_window, current_intent, cx);
            assert!(application.taskbar_overlay_edit_draft.is_none());
            assert!(application.taskbar_overlay_edit_correlation_id.is_none());
            assert!(application.taskbar_overlay_window == Some(overlay_window));
            application.close_schedule_window(cx);
        });
    });
    let _ = overlay_window.update(cx, |_, window, _| window.remove_window());
    let _ = std::fs::remove_dir_all(directory);
}

#[gpui_kit::test]
fn production_taskbar_overlay_keeps_information_geometry_stable_in_edit_mode(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_kit::init);
    let (application, directory) =
        download_application_for_window_test(cx, "taskbar-overlay-geometry");
    let settings = DownloadTaskbarProgressOverlaySettings {
        width: 320,
        height: 32,
        progress_mode: DownloadTaskbarProgressMode::Aggregate,
        show_speed: true,
        show_remaining_time: true,
        show_active_tasks: true,
        show_active_connections: true,
        show_transferred_bytes: true,
        ..Default::default()
    };
    let preview_samples = [
        DownloadTaskbarOverlaySample {
            status: DownloadStatus::Downloading,
            downloaded_bytes: 314 * 1024 * 1024,
            total_bytes: Some(1024 * 1024 * 1024),
            bytes_per_second: 10 * 1024 * 1024,
            active_connections: 4,
            segments: Vec::new(),
        },
        DownloadTaskbarOverlaySample {
            status: DownloadStatus::Downloading,
            downloaded_bytes: 420 * 1024 * 1024,
            total_bytes: Some(1024 * 1024 * 1024),
            bytes_per_second: 14 * 1024 * 1024,
            active_connections: 4,
            segments: Vec::new(),
        },
    ];
    let normal_model = aggregate_taskbar_progress(&settings, false, preview_samples);
    let platform = cx.read(|cx| Rc::clone(&application.read(cx).platform));
    let application_for_window = application.clone();
    let (_overlay, window) = cx.add_window_view(move |window, cx| {
        window.resize(size(px(320.0), px(32.0)));
        DownloadTaskbarOverlayView::new(
            application_for_window,
            platform,
            0,
            normal_model,
            window,
            cx,
        )
    });
    window.refresh().expect("render normal taskbar overlay");
    window.run_until_parked();
    let normal_content = window
        .debug_bounds("download-taskbar-overlay-content")
        .expect("normal overlay content frame");
    let normal_metrics = window
        .debug_bounds("download-taskbar-overlay-metrics")
        .expect("normal overlay metric row");
    let normal_progress = window
        .debug_bounds("download-taskbar-overlay-progress")
        .expect("normal overlay progress lanes");
    let normal_transferred = window
        .debug_bounds("download-taskbar-overlay-transferred")
        .expect("normal overlay transferred-byte metric");
    assert_eq!(normal_metrics.size.height, px(LINE_HEIGHT_LABEL));
    assert!(normal_transferred.right() <= normal_metrics.right());
    assert!(
        window
            .debug_bounds("download-taskbar-overlay-edit-border")
            .is_none()
    );
    assert!(
        window
            .debug_bounds("download-taskbar-overlay-resize-handle")
            .is_none()
    );

    window.update(|_, cx| {
        application.update(cx, |application, cx| {
            application.taskbar_overlay_edit_draft = Some(settings);
            cx.notify();
        });
    });
    window.run_until_parked();
    window.refresh().expect("render editable taskbar overlay");
    let edit_content = window
        .debug_bounds("download-taskbar-overlay-content")
        .expect("editable overlay content frame");
    let edit_metrics = window
        .debug_bounds("download-taskbar-overlay-metrics")
        .expect("editable overlay metric row");
    let edit_progress = window
        .debug_bounds("download-taskbar-overlay-progress")
        .expect("editable overlay progress lanes");
    let edit_transferred = window
        .debug_bounds("download-taskbar-overlay-transferred")
        .expect("editable overlay transferred-byte metric");
    assert_eq!(edit_content, normal_content);
    assert_eq!(edit_metrics, normal_metrics);
    assert_eq!(edit_progress, normal_progress);
    assert_eq!(edit_transferred, normal_transferred);
    assert!(
        window
            .debug_bounds("download-taskbar-overlay-edit-border")
            .is_some()
    );
    assert!(
        window
            .debug_bounds("download-taskbar-overlay-resize-handle")
            .is_some()
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[gpui_kit::test]
fn new_download_window_opens_without_reentering_download_application(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (application, directory) = download_application_for_window_test(cx, "new-download-window");

    cx.update(|cx| {
        application.update(cx, |application, cx| {
            application.open_new_download_window(cx);
        });
    });

    assert!(cx.read(|cx| { application.read(cx).new_download_window.as_ref().is_some() }));
    let _ = std::fs::remove_dir_all(directory);
}

#[gpui_kit::test]
fn production_new_download_category_form_and_bounded_menu_remain_readable(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (application, directory) =
        download_application_for_window_test(cx, "production-category-menu");
    let categories = (0..32)
        .map(|index| DownloadCategorySettings {
            id: format!("category-{index}"),
            name: format!("分类 {index:02}"),
            enabled: true,
            save_directory: format!(
                r"C:\Users\Administrator\Downloads\Category-{index:02}\A-Long-Readable-Directory"
            ),
            extensions: vec![format!("ext{index}")],
            url_patterns: Vec::new(),
        })
        .collect();
    let (completion, completed) = std::sync::mpsc::sync_channel(1);
    cx.update(|cx| {
        application.update(cx, |application, cx| {
            application.config.download.categories = categories;
            application.open_browser_download_dialog(
                BrowserDownloadDialogRequest {
                    correlation_id: uuid::Uuid::new_v4(),
                    handoff_id: uuid::Uuid::new_v4(),
                    source: BrowserDownloadDialogSource::Blob {
                        blob: BrowserBlob {
                            id: uuid::Uuid::new_v4(),
                            file_name: "captured.webm".into(),
                            media_type: Some("video/webm".into()),
                            size: 4096,
                        },
                        save_directory: directory.clone(),
                    },
                    completion,
                },
                cx,
            );
        });
    });
    assert_eq!(completed.recv().expect("browser dialog completion"), Ok(()));
    cx.run_until_parked();

    let new_download_window = cx
        .read(|cx| application.read(cx).new_download_window)
        .expect("production new-download window");
    let actual_size = new_download_window
        .update(cx, |_, window, _| window.bounds().size)
        .expect("read production new-download bounds");
    assert_eq!(actual_size, new_download_window_size(true));
    let mut window = VisualTestContext::from_window(new_download_window, cx);
    window.run_until_parked();
    window
        .refresh()
        .expect("render production new-download form");

    let file_name_field = window
        .debug_bounds("download-new-file-name-field")
        .expect("production file-name field");
    let category_field = window
        .debug_bounds("download-new-category-field")
        .expect("production category field");
    assert!(
        (file_name_field.top().as_f32() - category_field.top().as_f32()).abs() <= 1.0
            && (file_name_field.bottom().as_f32() - category_field.bottom().as_f32()).abs() <= 1.0
            && (file_name_field.size.width.as_f32() - category_field.size.width.as_f32()).abs()
                <= 2.0,
        "production file-name and category fields must share one equal-width form row"
    );
    let url_label = window
        .debug_bounds("download-new-url-field-label")
        .expect("production URL label");
    let url_control = window
        .debug_bounds("download-new-url-control")
        .expect("production URL control row");
    let form_grid = window
        .debug_bounds("download-new-form-grid")
        .expect("production new-download form grid");
    let directory_label = window
        .debug_bounds("download-new-directory-field-label")
        .expect("production directory label");
    let directory_row = window
        .debug_bounds("download-new-directory-control")
        .expect("production directory path row");
    let directory_control = window
        .debug_bounds("download-new-directory-input")
        .expect("production directory control");
    let directory_picker = window
        .debug_bounds("download-new-directory-picker")
        .expect("production directory picker");
    let file_name_label = window
        .debug_bounds("download-new-file-name-field-label")
        .expect("production Form file-name label");
    let file_name_control = window
        .debug_bounds("download-new-file-name-control")
        .expect("production file-name control");
    let category_label = window
        .debug_bounds("download-new-category-field-label")
        .expect("production Form category label");
    let category_trigger = window
        .debug_bounds("download-new-category-trigger")
        .expect("production category trigger");
    for (name, label) in [
        ("URL", url_label),
        ("directory", directory_label),
        ("file name", file_name_label),
        ("category", category_label),
    ] {
        assert_eq!(
            label.size.height,
            px(22.5),
            "{name} label uses released Form text_sm natural line height"
        );
    }
    for (name, control) in [
        ("URL", url_control),
        ("directory", directory_control),
        ("file name", file_name_control),
        ("category", category_trigger),
    ] {
        assert_eq!(
            control.size.height,
            px(CONTROL_HEIGHT_INPUT),
            "{name} control must use the shared 32 px input tier"
        );
    }
    assert_eq!(directory_picker.size, gpui_kit::size(px(32.0), px(32.0)));
    let url_paste = window
        .debug_bounds("download-new-url-paste")
        .expect("production URL paste button");
    assert_eq!(url_paste.size, gpui_kit::size(px(32.0), px(32.0)));
    assert!(
        (url_paste.right().as_f32() - (form_grid.right().as_f32() - 16.0)).abs() <= 1.0,
        "the URL paste button sits at the right edge of the form width"
    );
    assert!(
        (url_control.left().as_f32() - (form_grid.left().as_f32() + 16.0 + 76.0)).abs() <= 1.0
            && (url_control.right().as_f32() - (form_grid.right().as_f32() - 16.0)).abs() <= 1.0,
        "URL control row must fill the padded form width"
    );
    assert!(
        (directory_label.left().as_f32() - file_name_field.left().as_f32()).abs() <= 1.0
            && (directory_row.right().as_f32() - category_field.right().as_f32()).abs() <= 1.0,
        "directory path row must fill the file/category form column"
    );
    assert!(
        (file_name_label.top().as_f32() - category_label.top().as_f32()).abs() <= 1.0
            && (file_name_control.top().as_f32() - category_trigger.top().as_f32()).abs() <= 1.0,
        "file-name and category visible labels and controls must share baselines"
    );
    window.simulate_click(category_trigger.center(), Modifiers::default());
    window.run_until_parked();
    window.refresh().expect("render production category menu");
    let popover = window
        .debug_bounds("download-new-category-popover-content")
        .expect("bounded production category menu");
    assert!(
        popover.size.height.as_f32() <= 248.0,
        "32 categories must stay in a compact bounded menu, got {}px",
        popover.size.height.as_f32()
    );
    let search = window
        .debug_bounds("download-new-category-search")
        .expect("bounded category menu search field");
    let list = window
        .debug_bounds("download-new-category-list")
        .expect("shared bounded category ListBox");
    assert!(search.top() >= popover.top() && search.bottom() < list.top());
    assert!(
        list.bottom() <= popover.bottom(),
        "ListBox must stay inside the shared Popover: list={list:?}, popover={popover:?}"
    );
    assert!(
        window
            .debug_bounds("download-new-category-next-page")
            .is_none(),
        "the searchable ListBox must replace the seven-page control"
    );
    assert!(
        cx.read(|cx| {
            application
                .read(cx)
                .new_download_view
                .as_ref()
                .expect("production new-download view")
                .read(cx)
                .category_scroll
                .max_offset()
                .y
                > px(0.0)
        }),
        "all 32 categories must share one genuinely scrollable ListBox"
    );
    let first_directory = window
        .debug_bounds("download-new-category-option-directory-0")
        .expect("first category directory summary");
    let first_category = window
        .debug_bounds("download-new-category-option-0")
        .expect("first category option");
    let first_title = window
        .debug_bounds("download-new-category-option-title-0")
        .expect("first category name line");
    assert_eq!(first_title.size.height, px(LINE_HEIGHT_BODY));
    assert_eq!(first_directory.size.height, px(LINE_HEIGHT_LABEL));
    let category_content_left = first_category.left().as_f32() + 12.0;
    let category_content_right = first_category.right().as_f32() - 12.0;
    assert!(
        first_title.top() >= first_category.top()
            && first_title.bottom() <= first_directory.top()
            && first_directory.bottom() <= first_category.bottom()
            && (first_title.left().as_f32() - category_content_left).abs() <= 1.0
            && (first_title.right().as_f32() - category_content_right).abs() <= 1.0
            && (first_directory.left().as_f32() - category_content_left).abs() <= 1.0
            && (first_directory.right().as_f32() - category_content_right).abs() <= 1.0,
        "category bounded rows: option={first_category:?} title={first_title:?} directory={first_directory:?}"
    );
    let executor = cx.executor();
    window.simulate_mouse_move(first_category.center(), None, Modifiers::default());
    executor.advance_clock(Duration::from_millis(600));
    window.run_until_parked();
    window
        .refresh()
        .expect("render complete category-directory tooltip");
    assert!(
        window
            .debug_bounds("download-new-category-option-tooltip-0")
            .is_some(),
        "the complete category directory must remain readable from a stable tooltip"
    );
    let titlebar = window
        .debug_bounds("download-new-titlebar")
        .expect("production new-download titlebar");
    let outside_popover = gpui_kit::point(titlebar.left() + px(24.0), titlebar.center().y);
    assert!(
        !popover.contains(&outside_popover),
        "the outside-click fixture must remain outside the production Popover"
    );
    window.simulate_click(outside_popover, Modifiers::default());
    window.run_until_parked();
    window
        .refresh()
        .expect("dismiss category menu through overlay");
    assert!(
        !cx.read(|cx| {
            application
                .read(cx)
                .new_download_view
                .as_ref()
                .expect("production new-download view")
                .read(cx)
                .category_popover_open
        }),
        "clicking outside must close the controlled category Popover"
    );
    window.update(|window, cx| {
        let view = application
            .read(cx)
            .new_download_view
            .as_ref()
            .expect("production new-download view");
        assert!(view.read(cx).file_name.focus_handle(cx).is_focused(window));
    });
    window.simulate_click(category_trigger.center(), Modifiers::default());
    window.run_until_parked();
    window
        .refresh()
        .expect("reopen category menu after overlay close");
    window.simulate_keystrokes("escape");
    assert!(
        !cx.read(|cx| {
            application
                .read(cx)
                .new_download_view
                .as_ref()
                .expect("production new-download view")
                .read(cx)
                .category_popover_open
        }),
        "Escape must close the controlled category Popover"
    );
    window.update(|window, cx| {
        let view = application
            .read(cx)
            .new_download_view
            .as_ref()
            .expect("production new-download view");
        assert!(view.read(cx).file_name.focus_handle(cx).is_focused(window));
    });
    window.simulate_click(category_trigger.center(), Modifiers::default());
    window.run_until_parked();
    window.refresh().expect("reopen searchable category menu");
    window.simulate_input("分类 3");
    window.run_until_parked();
    window.refresh().expect("filter category ListBox");
    let last_category = window
        .debug_bounds("download-new-category-option-31")
        .expect("keyboard search reaches the final category");
    assert!(
        last_category.top() >= list.top() && last_category.bottom() <= list.bottom(),
        "the filtered final category must remain inside the ListBox: last={last_category:?}, list={list:?}"
    );
    window.simulate_keystrokes("down enter");
    window.run_until_parked();
    window
        .refresh()
        .expect("select searched production category");
    assert_eq!(
        cx.read(|cx| {
            application
                .read(cx)
                .new_download_view
                .as_ref()
                .expect("production new-download view")
                .read(cx)
                .selected_category_id
                .clone()
        }),
        Some("category-31".into()),
        "keyboard search and Enter must select the final production category"
    );
    assert!(
        !cx.read(|cx| {
            application
                .read(cx)
                .new_download_view
                .as_ref()
                .expect("production new-download view")
                .read(cx)
                .category_popover_open
        }),
        "selection must close the controlled Popover"
    );
    window.update(|window, cx| {
        let view = application
            .read(cx)
            .new_download_view
            .as_ref()
            .expect("production new-download view");
        assert!(view.read(cx).file_name.focus_handle(cx).is_focused(window));
    });

    let cancel = window
        .debug_bounds("download-new-cancel-action")
        .expect("production new-download cancel action");
    window.simulate_click(cancel.center(), Modifiers::default());
    window.run_until_parked();
    assert!(
        new_download_window.update(cx, |_, _, _| {}).is_err(),
        "clicking the production cancel action must close the native GPUI window"
    );

    let _ = std::fs::remove_dir_all(directory);
}

#[gpui_kit::test]
fn production_delayed_start_uses_shared_popover_and_preserves_draft_lifecycle(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_kit::init);
    let (application, directory) =
        download_application_for_window_test(cx, "production-delayed-start-popover");
    let (completion, completed) = std::sync::mpsc::sync_channel(1);
    cx.update(|cx| {
        application.update(cx, |application, cx| {
            application.open_browser_download_dialog(
                BrowserDownloadDialogRequest {
                    correlation_id: uuid::Uuid::new_v4(),
                    handoff_id: uuid::Uuid::new_v4(),
                    source: BrowserDownloadDialogSource::Blob {
                        blob: BrowserBlob {
                            id: uuid::Uuid::new_v4(),
                            file_name: "scheduled.webm".into(),
                            media_type: Some("video/webm".into()),
                            size: 4096,
                        },
                        save_directory: directory.clone(),
                    },
                    completion,
                },
                cx,
            );
        });
    });
    assert_eq!(completed.recv().expect("browser dialog completion"), Ok(()));
    cx.run_until_parked();

    let new_download_window = cx
        .read(|cx| application.read(cx).new_download_window)
        .expect("production new-download window");
    let view = cx
        .read(|cx| application.read(cx).new_download_view.clone())
        .expect("production new-download view");
    let mut window = VisualTestContext::from_window(new_download_window, cx);
    window.run_until_parked();
    window
        .refresh()
        .expect("render production delayed-start form");

    let schedule = window
        .debug_bounds("download-new-schedule-control")
        .expect("production delayed-start action");
    window.simulate_click(schedule.center(), Modifiers::default());
    window.run_until_parked();
    window.refresh().expect("render shared schedule Popover");

    let shell = window
        .debug_bounds("download-new-schedule-popover-shell")
        .expect("shared schedule Popover shell");
    let content = window
        .debug_bounds("download-new-schedule-popover-content")
        .expect("schedule Popover content");
    assert!(
        content.left() >= shell.left()
            && content.right() <= shell.right()
            && content.top() >= shell.top()
            && content.bottom() <= shell.bottom(),
        "schedule content must remain inside the shared Popover shell"
    );
    assert!(
        shell.size.width.as_f32() <= 304.0 && shell.size.height.as_f32() <= 240.0,
        "schedule Popover must stay bounded at 304x240, got {:?}",
        shell.size
    );
    let time_row = window
        .debug_bounds("download-new-schedule-time-row")
        .expect("production schedule time row");
    let action_row = window
        .debug_bounds("download-new-schedule-action-row")
        .expect("production schedule action row");
    let clear_button = window
        .debug_bounds("download-new-schedule-clear-button")
        .expect("visible clear button hit target");
    let confirm_button = window
        .debug_bounds("download-new-schedule-confirm-button")
        .expect("visible confirm button hit target");
    for (name, button) in [("clear", clear_button), ("confirm", confirm_button)] {
        assert_eq!(
            button.size.height,
            px(24.0),
            "schedule {name} uses official Small action tier"
        );
        assert!(button.left() >= action_row.left() && button.right() <= action_row.right());
        assert!(button.top() >= action_row.top() && button.bottom() <= action_row.bottom());
        assert!(button.left() >= shell.left() && button.right() <= shell.right());
        assert!(button.top() >= shell.top() && button.bottom() <= shell.bottom());
    }
    assert!(
        (clear_button.top().as_f32() - confirm_button.top().as_f32()).abs() <= 1.0
            && (clear_button.bottom().as_f32() - confirm_button.bottom().as_f32()).abs() <= 1.0,
        "schedule buttons must share a vertical baseline"
    );
    assert!(
        clear_button.right() < confirm_button.left(),
        "schedule button hit targets must not overlap"
    );
    assert!(clear_button.left() - action_row.left() >= px(SPACING_8));
    assert!(action_row.right() - confirm_button.right() >= px(SPACING_8));
    assert!(time_row.top() >= shell.top() && time_row.bottom() <= shell.bottom());
    assert!(action_row.top() >= time_row.bottom());
    for (name, selector) in [
        (
            "previous month",
            "download-new-schedule-previous-month-action",
        ),
        ("next month", "download-new-schedule-next-month-action"),
        ("clear", "download-new-schedule-clear-action"),
        ("confirm", "download-new-schedule-confirm"),
    ] {
        let action = window
            .debug_bounds(selector)
            .unwrap_or_else(|| panic!("missing shared schedule {name} action"));
        assert!(
            (24.0..=30.0).contains(&action.size.height.as_f32()),
            "schedule {name} must use its Kit official size tier, got {}px",
            action.size.height.as_f32()
        );
    }
    let initial_month = window.read(|cx| view.read(cx).scheduled_month);
    let next_month = window
        .debug_bounds("download-new-schedule-next-month-action")
        .expect("production next-month action");
    window.simulate_click(next_month.center(), Modifiers::default());
    window.run_until_parked();
    window
        .refresh()
        .expect("navigate to the next schedule month");
    assert_eq!(
        window.read(|cx| view.read(cx).scheduled_month),
        shift_calendar_month(initial_month, 1),
        "the production next-month action must update the local schedule draft"
    );
    let previous_month = window
        .debug_bounds("download-new-schedule-previous-month-action")
        .expect("production previous-month action");
    window.simulate_click(previous_month.center(), Modifiers::default());
    window.run_until_parked();
    window
        .refresh()
        .expect("navigate back to the original schedule month");
    assert_eq!(
        window.read(|cx| view.read(cx).scheduled_month),
        initial_month,
        "the production previous-month action must restore the local schedule month"
    );

    let confirm = window
        .debug_bounds("download-new-schedule-confirm")
        .expect("schedule confirm action");
    window.simulate_click(confirm.center(), Modifiers::default());
    window.run_until_parked();
    window.refresh().expect("commit production delayed start");
    let committed = window
        .read(|cx| view.read(cx).committed_schedule)
        .expect("confirm must commit the production delayed start");
    assert!(!window.read(|cx| view.read(cx).schedule_popover_open));

    window.simulate_click(schedule.center(), Modifiers::default());
    window.run_until_parked();
    window.refresh().expect("reopen production delayed start");
    assert_eq!(
        window.read(|cx| view.read(cx).scheduled_date),
        committed.date,
        "opening must snapshot the committed date back into the local draft"
    );
    // The 124px calendar viewport can clip later weeks (date-dependent).
    // A debug bound alone does not make a pointer target visible/hittable.
    fn visible_next_schedule_day(
        window: &mut VisualTestContext,
    ) -> gpui_kit::Bounds<gpui_kit::Pixels> {
        for _ in 0..8 {
            let day = window
                .debug_bounds("download-new-schedule-next-day")
                .expect("next schedule day");
            let shell = window
                .debug_bounds("download-new-schedule-popover-shell")
                .expect("calendar shell");
            let time = window
                .debug_bounds("download-new-schedule-time-row")
                .expect("time row");
            if day.top() >= shell.top() + px(4.0) && day.bottom() <= time.top() - px(4.0) {
                return day;
            }
            window.simulate_event(gpui_kit::ScrollWheelEvent {
                position: gpui_kit::point(shell.center().x, time.top() - px(40.0)),
                delta: gpui_kit::ScrollDelta::Lines(gpui_kit::point(0.0, -1.0)),
                modifiers: Modifiers::default(),
                touch_phase: gpui_kit::TouchPhase::Moved,
            });
            window.run_until_parked();
            window
                .refresh()
                .expect("reveal date inside actual calendar scroll viewport");
        }
        panic!("next schedule date must be visibly reachable before pointer activation");
    }
    let draft_date = committed.date + ChronoDuration::days(1);
    let draft_day = visible_next_schedule_day(&mut window);
    window.simulate_click(draft_day.center(), Modifiers::default());
    window.run_until_parked();
    window
        .refresh()
        .expect("render modified production schedule draft");
    assert_eq!(window.read(|cx| view.read(cx).scheduled_date), draft_date);
    let schedule = window
        .debug_bounds("download-new-footer-secondary-schedule-action")
        .expect("current actual schedule button");
    let clear = window
        .debug_bounds("download-new-schedule-clear-action")
        .expect("current Clear action");
    let popup = window
        .debug_bounds("download-new-schedule-popover-shell")
        .expect("schedule popup");
    assert!(
        !popup.contains(&schedule.center()),
        "schedule popup must leave trigger operable: trigger={schedule:?} popup={popup:?} clear={clear:?}"
    );
    assert!(
        !clear.contains(&schedule.center()),
        "popover Clear must not overlap trigger: trigger={schedule:?} clear={clear:?}"
    );
    window.simulate_click(schedule.center(), Modifiers::default());
    window.run_until_parked();
    window
        .refresh()
        .expect("dismiss schedule draft through trigger toggle");
    assert!(!window.read(|cx| view.read(cx).schedule_popover_open));
    assert_eq!(
        window.read(|cx| view.read(cx).committed_schedule),
        Some(committed),
        "trigger-toggle dismissal must not commit the modified draft"
    );

    window.simulate_click(schedule.center(), Modifiers::default());
    window.run_until_parked();
    window
        .refresh()
        .expect("reopen after trigger-toggle dismissal");
    assert_eq!(
        window.read(|cx| view.read(cx).scheduled_date),
        committed.date,
        "reopening after trigger dismissal must restore the committed draft snapshot"
    );
    let draft_day = visible_next_schedule_day(&mut window);
    window.simulate_click(draft_day.center(), Modifiers::default());
    window.run_until_parked();
    window
        .refresh()
        .expect("modify the reopened schedule draft");
    assert_eq!(window.read(|cx| view.read(cx).scheduled_date), draft_date);
    let titlebar = window
        .debug_bounds("download-new-titlebar")
        .expect("production new-download titlebar");
    let outside_popover = gpui_kit::point(titlebar.right() - px(80.0), titlebar.center().y);
    assert!(
        !shell.contains(&outside_popover),
        "the outside-click fixture must remain outside the production schedule Popover"
    );
    window.simulate_click(outside_popover, Modifiers::default());
    window.run_until_parked();
    window.refresh().expect("dismiss production schedule draft");
    assert!(!window.read(|cx| view.read(cx).schedule_popover_open));
    assert_eq!(
        window.read(|cx| view.read(cx).committed_schedule),
        Some(committed),
        "outside dismissal must preserve the last committed schedule"
    );

    window.simulate_click(schedule.center(), Modifiers::default());
    window.run_until_parked();
    window.refresh().expect("reopen schedule before clearing");
    let clear = window
        .debug_bounds("download-new-schedule-clear-action")
        .expect("schedule clear action");
    window.simulate_click(clear.center(), Modifiers::default());
    window.run_until_parked();
    window.refresh().expect("clear production delayed start");
    assert_eq!(window.read(|cx| view.read(cx).committed_schedule), None);
    assert!(!window.read(|cx| view.read(cx).schedule_popover_open));

    let cancel = window
        .debug_bounds("download-new-cancel-action")
        .expect("production new-download cancel action");
    window.simulate_click(cancel.center(), Modifiers::default());
    window.run_until_parked();
    assert!(new_download_window.update(cx, |_, _, _| {}).is_err());
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn browser_download_window_starts_at_compact_probe_height() {
    let request = DownloadRequest {
        source_url: "https://example.test/browser.bin".into(),
        save_directory: PathBuf::from(r"C:\Downloads"),
        save_directory_explicit: false,
        file_name: Some("browser.bin".into()),
        headers: Vec::new(),
        cookie: None,
        referrer: None,
        certificate_policy: Default::default(),
        connection_limit: Some(4),
        speed_limit_bytes_per_second: None,
        expected_total_bytes: None,
        scheduled_at_unix_ms: None,
        adaptive_segment_policy: None,
        media: None,
        category_id: None,
        category_name: None,
        duplicate_strategy: nxtools_download::DownloadDuplicateStrategy::UseDefault,
    };
    assert_eq!(
        initial_new_download_window_size(Some(&NewDownloadInitialSource::Download(request))),
        new_download_window_size(false),
        "an unconfirmed browser handoff must open at the compact probe height"
    );
}

#[gpui_kit::test]
fn browser_probe_window_options_allow_the_compact_native_minimum(cx: &mut TestAppContext) {
    let compact = new_download_window_size(false);
    let options = cx.update(|cx| new_download_window_options(WindowBounds::centered(compact, cx)));
    let initial = options
        .window_bounds
        .expect("initial native window bounds")
        .get_bounds()
        .size;
    let minimum = options.window_min_size.expect("native minimum window size");

    assert_eq!(initial.width.as_f32(), compact.width.as_f32());
    assert_eq!(initial.height.as_f32(), compact.height.as_f32());
    assert_eq!(minimum.width.as_f32(), compact.width.as_f32());
    assert_eq!(minimum.height.as_f32(), compact.height.as_f32());
}

#[gpui_kit::test]
fn browser_download_window_shrinks_to_compact_height_while_probe_is_unconfirmed(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_kit::init);
    let (application, directory) =
        download_application_for_window_test(cx, "browser-probe-window-height");
    let request = DownloadRequest {
        source_url: "http://127.0.0.1:9/browser.bin".into(),
        save_directory: directory.clone(),
        save_directory_explicit: false,
        file_name: Some("browser.bin".into()),
        headers: Vec::new(),
        cookie: None,
        referrer: None,
        certificate_policy: Default::default(),
        connection_limit: Some(4),
        speed_limit_bytes_per_second: None,
        expected_total_bytes: None,
        scheduled_at_unix_ms: None,
        adaptive_segment_policy: None,
        media: None,
        category_id: None,
        category_name: None,
        duplicate_strategy: nxtools_download::DownloadDuplicateStrategy::UseDefault,
    };

    cx.update(|cx| {
        application.update(cx, |application, cx| {
            application
                .open_new_download_window_with_request(
                    Some(NewDownloadInitialSource::Download(request)),
                    None,
                    cx,
                )
                .expect("open browser download window");
        });
    });
    cx.run_until_parked();

    let window = cx.read(|cx| {
        application
            .read(cx)
            .new_download_window
            .expect("browser download window")
    });
    let height = cx.update(|cx| {
        window
            .update(cx, |_, window, _| window.bounds().size.height.as_f32())
            .expect("read browser download window bounds")
    });
    // A failed remote probe is an operation notice, not an input-validation row.
    // Keep the compact form, source draft and retry path without reflowing its footer.
    let compact = new_download_window_size(false).height.as_f32();
    assert_eq!(height, compact, "probe failure must not add a page banner");
    cx.read(|cx| {
        let app = application.read(cx);
        let view = app
            .new_download_view
            .as_ref()
            .expect("retryable view")
            .read(cx);
        assert!(!view.probing);
        assert!(view.probe.is_none());
        assert!(view.error.is_none());
        assert_eq!(view.url.read(cx).value(), "http://127.0.0.1:9/browser.bin");
        assert!(
            app.notifications
                .read(cx)
                .items()
                .iter()
                .any(|notice| notice.id.starts_with("download-probe-")
                    && notice.level == crate::notifications::NotificationLevel::Error)
        );
    });

    let _ = std::fs::remove_dir_all(directory);
}

#[gpui_kit::test]
fn download_terminal_feedback_keeps_distinct_probe_and_submission_invocations(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_kit::init);
    let (application, directory) =
        download_application_for_window_test(cx, "notification-identities");
    application.update(cx, |app, cx| app.open_new_download_window(cx));
    cx.run_until_parked();
    let view = application.read_with(cx, |app, _| {
        app.new_download_view.clone().expect("production draft")
    });
    view.update(cx, |view, cx| {
        view.finish_probe_failure("first probe".into(), cx)
    });
    cx.run_until_parked();
    view.update(cx, |view, cx| {
        view.finish_probe_failure("second probe".into(), cx)
    });
    cx.run_until_parked();
    view.update(cx, |view, cx| {
        view.finish_submission_failure("submission".into(), cx)
    });
    cx.run_until_parked();
    application.read_with(cx, |app, cx| {
        let notices = app.notifications.read(cx).items();
        assert_eq!(notices.len(), 3);
        assert_eq!(
            notices
                .iter()
                .filter(|notice| notice.id.starts_with("download-probe-"))
                .count(),
            2
        );
        assert_eq!(
            notices
                .iter()
                .filter(|notice| notice.id.starts_with("download-submit-"))
                .count(),
            1
        );
        for (index, notice) in notices.iter().enumerate() {
            assert!(notices[..index].iter().all(|other| other.id != notice.id));
            assert_eq!(notice.level, crate::notifications::NotificationLevel::Error);
        }
    });
    let _ = std::fs::remove_dir_all(directory);
}

#[gpui_kit::test]
fn browser_export_terminal_routing_keeps_invocations_separate_and_ignores_duplicate_results(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_kit::init);
    let (application, directory, endpoint) =
        download_application_for_window_test_with_platform(cx, "export-notification-identity");
    endpoint.set_connected_for_contract_test(true);
    let mut keys = Vec::new();
    for format in ["chrome", "firefox", "chrome_store"] {
        let id = uuid::Uuid::new_v4();
        application.update(cx, |app, _| {
            app.request_browser_extension_export(format, id)
                .expect("accepted export")
        });
        let command = std::iter::from_fn(|| endpoint.try_receive_command_for_contract_test().ok())
            .find(|command| {
                matches!(
                    command.message,
                    nxtools_protocol::Message::PlatformCommand(
                        PlatformCommand::ExportBrowserExtension(_)
                    )
                )
            })
            .expect("typed export command");
        assert_eq!(
            command.id, id,
            "notification invocation ID crosses the native command boundary"
        );
        assert!(matches!(
            command.message,
            nxtools_protocol::Message::PlatformCommand(PlatformCommand::ExportBrowserExtension(_))
        ));
        let result = PlatformCommandResult {
            success: true,
            error: None,
            native_error_code: None,
            data: Some("synthetic-extension.zip".into()),
        };
        application.update(cx, |app, cx| {
            app.handle_platform_notification(
                PlatformNotification::CommandResult {
                    correlation_id: id,
                    result: result.clone(),
                },
                cx,
            );
            app.handle_platform_notification(
                PlatformNotification::CommandResult {
                    correlation_id: id,
                    result,
                },
                cx,
            );
        });
        cx.run_until_parked();
        keys.push(format!("download-browser-export-{id}"));
        application.read_with(cx, |app, cx| {
            assert!(app.pending_extension_export.is_none());
            let notices = app.notifications.read(cx).items();
            assert_eq!(notices.len(), keys.len());
            for key in &keys {
                let notice = notices
                    .iter()
                    .find(|notice| notice.id.as_ref() == key.as_str())
                    .expect("invocation terminal");
                assert_eq!(
                    notice.level,
                    crate::notifications::NotificationLevel::Success
                );
            }
        });
    }
    let _ = std::fs::remove_dir_all(directory);
}

#[gpui_kit::test]
fn production_probe_summary_uses_narrow_inspector_and_bounded_stage_geometry(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_kit::init);
    let (application, directory) =
        download_application_for_window_test(cx, "new-download-probe-summary");

    cx.update(|cx| {
        application.update(cx, |application, cx| {
            application.open_new_download_window(cx);
        });
    });
    cx.run_until_parked();

    let new_download_window = cx
        .read(|cx| application.read(cx).new_download_window)
        .expect("production new-download window");
    let compact_size = new_download_window
        .update(cx, |_, window, _| window.bounds().size)
        .expect("read compact new-download bounds");
    assert_eq!(compact_size, new_download_window_size(false));

    let mut window = VisualTestContext::from_window(new_download_window, cx);
    window
        .refresh()
        .expect("render compact new-download probe stage");
    let guidance = window
        .debug_bounds("download-new-probe-guidance")
        .expect("compact inline probe guidance");
    let footer = window
        .debug_bounds("download-new-footer")
        .expect("compact new-download footer");
    assert_eq!(guidance.size.height, px(LINE_HEIGHT_BODY));
    assert!(guidance.bottom() <= footer.top());
    assert!(
        window
            .debug_bounds("download-new-probe-inspector")
            .is_none(),
        "the narrow probe inspector must not reserve space before confirmation"
    );

    let view = cx
        .read(|cx| application.read(cx).new_download_view.clone())
        .expect("production new-download view");
    let probe = DownloadProbe {
        resolution: None,
        final_url: "https://example.test/archive.bin".into(),
        file_name: Some("archive.bin".into()),
        total_bytes: Some(64 * 1024 * 1024),
        supports_ranges: true,
        content_type: Some("application/octet-stream".into()),
        validator: nxtools_download::ResourceValidator::default(),
        media: None,
    };
    let confirmed_probe_size = new_download_window_size_for_probe(&probe);
    new_download_window
        .update(cx, move |_, _native_window, cx| {
            view.update(cx, |view, cx| {
                view.probe = Some(probe);
                view.confirmed_target_height = Some(confirmed_probe_size.height);
                // Even an expired gate cannot release a clipped form.
                view.confirmed_gate_expired = true;
                cx.notify();
            });
        })
        .expect("stage confirmed metadata while the native client remains compact");
    window.run_until_parked();
    window
        .refresh()
        .expect("render pending native confirmation");
    assert!(
        window
            .debug_bounds("download-new-probe-inspector")
            .is_none(),
        "confirmation metadata must not paint into the old compact client"
    );
    new_download_window
        .update(cx, |_, native_window, _| {
            native_window.resize(confirmed_probe_size)
        })
        .expect("native confirmation resize");
    window.run_until_parked();
    // The test surface keeps its initial size when only the cached window
    // bounds change; simulate the real confirmed surface so the geometry
    // below measures the layout the user actually sees instead of an
    // overflowing 176px draw (which used to push the footer off-window).
    window.simulate_resize(confirmed_probe_size);
    window.refresh().expect("render confirmed probe inspector");

    let confirmed_size = new_download_window
        .update(cx, |_, native_window, _| native_window.bounds().size)
        .expect("read confirmed new-download bounds");
    assert_eq!(confirmed_size, new_download_window_size(true));
    assert_eq!(
        confirmed_size.width.as_f32(),
        compact_size.width.as_f32(),
        "probe confirmation must not change the dialog width"
    );
    assert!(
        confirmed_size.height.as_f32() >= NEW_DOWNLOAD_CONFIRMED_HEIGHT,
        "confirmed probe must use the generous client-height floor"
    );
    assert_eq!(
        confirmed_size.height.as_f32() - compact_size.height.as_f32(),
        NEW_DOWNLOAD_CONFIRMED_HEIGHT - 176.0,
        "probe confirmation must switch to the full destination form height"
    );

    let form_grid = window
        .debug_bounds("download-new-form-grid")
        .expect("confirmed new-download form grid");
    let directory_label = window
        .debug_bounds("download-new-directory-field-label")
        .expect("confirmed directory label");
    let directory_row = window
        .debug_bounds("download-new-directory-control")
        .expect("confirmed directory path row");
    let file_name_field = window
        .debug_bounds("download-new-file-name-field")
        .expect("confirmed file-name field");
    let inspector = window
        .debug_bounds("download-new-probe-inspector")
        .expect("confirmed narrow probe inspector");
    assert!(
        inspector.size.width.as_f32() <= 120.0
            && inspector.size.width.as_f32() < form_grid.size.width.as_f32() * 0.25,
        "probe summary must remain a narrow inspector, got {}px inside {}px",
        inspector.size.width.as_f32(),
        form_grid.size.width.as_f32()
    );
    assert!(
        inspector.left() > directory_row.right()
            && inspector.top() <= directory_label.top()
            && inspector.bottom() >= file_name_field.bottom(),
        "probe inspector must occupy the right rail beside the destination fields"
    );
    for (selector, expected_height) in [
        ("download-new-probe-status", LINE_HEIGHT_BODY),
        ("download-new-probe-size", LINE_HEIGHT_BODY),
        ("download-new-probe-capability", LINE_HEIGHT_BODY),
    ] {
        let row = window
            .debug_bounds(selector)
            .unwrap_or_else(|| panic!("missing probe inspector row {selector}"));
        assert_eq!(row.size.height, px(expected_height));
        assert!(
            row.left() >= inspector.left()
                && row.right() <= inspector.right()
                && row.top() >= inspector.top()
                && row.bottom() <= inspector.bottom(),
            "{selector} must stay inside the narrow inspector"
        );
    }
    let confirmed_footer = window
        .debug_bounds("download-new-footer")
        .expect("confirmed new-download footer");
    assert!(inspector.bottom() <= confirmed_footer.top());
    let form_content = window
        .debug_bounds("download-new-form-grid")
        .expect("confirmed form content bounds");
    assert!(form_content.top().as_f32() >= 0.0);
    assert!(form_content.bottom().as_f32() <= confirmed_footer.top().as_f32());
    for field in [directory_row, file_name_field, inspector] {
        assert!(field.left().as_f32() >= form_content.left().as_f32());
        assert!(field.right().as_f32() <= form_content.right().as_f32());
        assert!(field.bottom().as_f32() <= confirmed_footer.top().as_f32());
    }

    window.simulate_resize(new_download_window_size(false));
    window.run_until_parked();
    let compact_viewport = new_download_window
        .update(cx, |_, window, _| window.viewport_size())
        .expect("read restored compact viewport");
    assert_eq!(compact_viewport, new_download_window_size(false));
    let restored_gate = new_download_window
        .update(cx, |_, native_window, cx| {
            let app = application.read(cx);
            let view = app
                .new_download_view
                .as_ref()
                .expect("new download view")
                .read(cx);
            (
                view.confirmed_target_height,
                confirmed_window_pending(
                    view.confirmed_target_height,
                    view.confirmed_gate_expired,
                    native_window.viewport_size().height,
                ),
            )
        })
        .expect("read compact confirmation gate");
    assert!(
        restored_gate.1,
        "compact gate must be pending: {:?}",
        restored_gate
    );
    window
        .refresh()
        .expect("render compact client after a confirmed frame");
    let compact_guidance = window
        .debug_bounds("download-new-probe-guidance")
        .expect("current compact confirmation-pending guidance");
    let compact_footer = window
        .debug_bounds("download-new-footer")
        .expect("compact recovery footer");
    let compact_root = window
        .debug_bounds("download-new-root")
        .expect("compact recovery root");
    let cancel_action = window
        .debug_bounds("download-new-cancel-action")
        .expect("visible compact cancel action");
    assert!(compact_footer.bottom() <= compact_root.bottom());
    assert!(
        cancel_action.top() >= compact_footer.top()
            && cancel_action.bottom() <= compact_footer.bottom()
    );
    assert!(compact_guidance.bottom() <= compact_footer.top());
    window.simulate_resize(confirmed_probe_size);
    window.run_until_parked();
    window
        .refresh()
        .expect("render settled client after compact restoration");
    let settled_footer = window
        .debug_bounds("download-new-footer")
        .expect("settled footer");
    let settled_inspector = window
        .debug_bounds("download-new-probe-inspector")
        .expect("restored confirmation inspector");
    let submit_action = window
        .debug_bounds("download-new-footer-primary-action")
        .expect("restored download action");
    assert!(settled_inspector.bottom() <= settled_footer.top());
    assert!(
        submit_action.top() >= settled_footer.top()
            && submit_action.bottom() <= settled_footer.bottom()
    );

    let _ = std::fs::remove_dir_all(directory);
}

#[gpui_kit::test]
fn probe_validation_notifies_without_resizing_and_keeps_footer(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (application, directory) =
        download_application_for_window_test(cx, "new-download-error-reserve");

    cx.update(|cx| {
        application.update(cx, |application, cx| {
            application.open_new_download_window(cx);
        });
    });
    cx.run_until_parked();

    let new_download_window = cx
        .read(|cx| application.read(cx).new_download_window)
        .expect("production new-download window");
    let mut window = VisualTestContext::from_window(new_download_window, cx);
    window.refresh().expect("render compact dialog");

    let footer_before = window
        .debug_bounds("download-new-footer")
        .expect("compact footer");
    let view = cx
        .read(|cx| application.read(cx).new_download_view.clone())
        .expect("production new-download view");
    new_download_window
        .update(cx, move |_, window, cx| {
            // User submission errors use the original notification ingress.
            view.update(cx, |view, cx| {
                view.url.update(cx, |input, cx| {
                    input.replace_all("not a valid url", window, cx);
                });
                view.begin_probe(window, cx);
            });
        })
        .expect("run failing probe");
    window.run_until_parked();
    window.simulate_resize(new_download_window_size(false));
    window.refresh().expect("render error reserve");

    let base = new_download_window_size(false);
    let error_size = new_download_window
        .update(cx, |_, native_window, _| native_window.bounds().size)
        .expect("read error reserve bounds");
    assert_eq!(
        error_size, base,
        "validation must not resize the draft window"
    );
    let footer_after = window
        .debug_bounds("download-new-footer")
        .expect("footer with error");
    assert_eq!(
        footer_after.size, footer_before.size,
        "footer geometry must not change when an error appears"
    );
    assert!(
        window.debug_bounds("download-inline-error").is_none(),
        "validation does not duplicate the shared notification"
    );
    application.read_with(&window, |app, cx| {
        assert!(
            app.notifications
                .read(cx)
                .items()
                .iter()
                .any(|notice| notice.level == crate::notifications::NotificationLevel::Error)
        )
    });

    let _ = std::fs::remove_dir_all(directory);
}

#[gpui_kit::test]
fn confirmed_new_download_submit_error_keeps_the_footer_inside_the_window(cx: &mut TestAppContext) {
    // The user report: after a probe was confirmed the dialog showed the
    // file inspector, and a submission failure squeezed the form — the
    // window stayed at the compact error height, so the file-name row and
    // the footer were pushed out of the fixed-size window. The error
    // reserve must stack onto the confirmed layout height.
    cx.update(gpui_kit::init);
    let (application, directory) =
        download_application_for_window_test(cx, "new-download-confirmed-error");
    let application_for_window = application.clone();
    let default_directory = directory.clone();
    let default_connection_limit = cx.read(|cx| application.read(cx).default_connection_limit());
    let default_adaptive_segment_policy =
        cx.read(|cx| application.read(cx).default_adaptive_segment_policy());
    let view_slot = Rc::new(RefCell::new(None));
    let opened_view_slot = Rc::clone(&view_slot);
    let categories = DownloadSettings::default().categories;
    let (_root, window) = cx.add_window_view(move |window, cx| {
        let view = cx.new(|cx| {
            NewDownloadView::new(
                NewDownloadViewInit {
                    application: application_for_window,
                    default_directory,
                    default_connection_limit,
                    default_adaptive_segment_policy,
                    categories,
                    initial_source: None,
                },
                window,
                cx,
            )
        });
        *opened_view_slot.borrow_mut() = Some(view.clone());
        Root::new(view, window, cx)
    });
    let view = view_slot
        .borrow()
        .clone()
        .expect("new download view mounted");
    window.refresh().expect("render compact dialog");

    let request = DownloadRequest {
        source_url: "https://example.test/file.bin".into(),
        save_directory: directory.clone(),
        save_directory_explicit: false,
        file_name: Some("file.bin".into()),
        headers: Vec::new(),
        cookie: None,
        referrer: None,
        certificate_policy: Default::default(),
        connection_limit: Some(default_connection_limit),
        speed_limit_bytes_per_second: None,
        expected_total_bytes: None,
        scheduled_at_unix_ms: None,
        adaptive_segment_policy: None,
        media: None,
        category_id: None,
        category_name: None,
        duplicate_strategy: nxtools_download::DownloadDuplicateStrategy::UseDefault,
    };
    window.update(|window, cx| {
        let url = view.read(cx).url.clone();
        url.update(cx, |input, cx| {
            input.set_value("https://example.test/file.bin", window, cx);
        });
        view.update(cx, |view, cx| {
            view.confirmed_request = Some(request);
            view.probe = Some(DownloadProbe {
                resolution: None,
                final_url: "https://example.test/file.bin".into(),
                file_name: Some("file.bin".into()),
                total_bytes: Some(1024),
                supports_ranges: true,
                content_type: Some("application/octet-stream".into()),
                validator: nxtools_download::ResourceValidator::default(),
                media: None,
            });
            cx.notify();
        });
        window.resize(new_download_window_size(true));
    });
    window
        .refresh()
        .expect("render confirmed dialog with probe inspector");
    let confirmed_window_size = window.update(|window, _| window.bounds().size);
    assert_eq!(
        confirmed_window_size,
        new_download_window_size(true),
        "the confirmed dialog opens at the confirmed fixed size"
    );

    // A synchronous submit validation failure (empty save directory) is
    // the same path a rejected submission takes: it keeps the probe
    // result visible and shows one inline error row.
    window.update(|window, cx| {
        let directory_input = view.read(cx).directory.clone();
        directory_input.update(cx, |input, cx| {
            input.replace_all("", window, cx);
        });
        view.update(cx, |view, cx| {
            view.submit(DownloadSubmitMode::StartNow, window, cx);
        });
    });
    window
        .refresh()
        .expect("render confirmed dialog with submit error");

    let error_window_size = window.update(|window, _| window.bounds().size);
    assert_eq!(
        error_window_size,
        new_download_window_size(true),
        "submit validation keeps the confirmed window size"
    );

    // The test surface draws at its default size, so reproduce the real
    // fixed-size dialog at the applied error bounds and verify the
    // confirmed form and footer still fit inside instead of being
    // squeezed out of the window.
    let error_bounds = new_download_window_size(true);
    window.simulate_resize(error_bounds);
    window
        .refresh()
        .expect("redraw at the confirmed error size");
    let drawn_height = error_bounds.height.as_f32();
    let footer = window
        .debug_bounds("download-new-footer")
        .expect("footer stays mounted after a confirmed submit error");
    assert!(
        footer.bottom().as_f32() <= drawn_height,
        "footer bottom {} must stay inside the {}px window instead of being squeezed out",
        footer.bottom().as_f32(),
        drawn_height
    );
    let file_name_field = window
        .debug_bounds("download-new-file-name-field")
        .expect("file-name field stays mounted after a confirmed submit error");
    assert!(
        file_name_field.bottom().as_f32() <= drawn_height,
        "the file-name row must stay visible instead of being clipped by the window"
    );
    assert!(
        window.debug_bounds("download-inline-error").is_none(),
        "submit validation uses shared notification only"
    );
    window.run_until_parked();
    application.read_with(window, |app, cx| {
        assert!(
            app.notifications
                .read(cx)
                .items()
                .iter()
                .any(|notice| notice.level == crate::notifications::NotificationLevel::Error)
        )
    });
}

#[test]
fn new_download_footer_maps_exactly_one_action_to_primary_emphasis() {
    let actions = [
        NewDownloadFooterAction::Schedule,
        NewDownloadFooterAction::QueueOnly,
        NewDownloadFooterAction::StartNow,
        NewDownloadFooterAction::Cancel,
    ];
    let primary = actions
        .iter()
        .copied()
        .filter(|action| action.emphasis() == NewDownloadFooterEmphasis::Primary)
        .collect::<Vec<_>>();

    assert_eq!(primary, vec![NewDownloadFooterAction::StartNow]);
    for action in actions {
        let selector = action.role_selector();
        assert_eq!(
            selector.contains("primary"),
            action.emphasis() == NewDownloadFooterEmphasis::Primary,
            "the observable footer role must be derived from the actual emphasis mapping"
        );
    }
}

#[gpui_kit::test]
fn confirmed_new_download_schedule_and_actions_stay_inside_the_footer(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (application, directory) = download_application_for_window_test(cx, "new-download-layout");
    let application_for_window = application.clone();
    let default_directory = directory.clone();
    let default_connection_limit = cx.read(|cx| application.read(cx).default_connection_limit());
    let default_adaptive_segment_policy =
        cx.read(|cx| application.read(cx).default_adaptive_segment_policy());
    let view_slot = Rc::new(RefCell::new(None));
    let opened_view_slot = Rc::clone(&view_slot);
    let mut categories = DownloadSettings::default().categories;
    categories[0].save_directory =
        r"C:\Users\Administrator\Downloads\Archives\Compressed\LongCategoryDirectory".into();
    let (_root, window) = cx.add_window_view(move |window, cx| {
        let view = cx.new(|cx| {
            NewDownloadView::new(
                NewDownloadViewInit {
                    application: application_for_window,
                    default_directory,
                    default_connection_limit,
                    default_adaptive_segment_policy,
                    categories,
                    initial_source: None,
                },
                window,
                cx,
            )
        });
        *opened_view_slot.borrow_mut() = Some(view.clone());
        Root::new(view, window, cx)
    });
    let view = view_slot
        .borrow()
        .clone()
        .expect("new download view mounted");
    window
        .refresh()
        .expect("render initial new download window");
    let request = DownloadRequest {
        source_url: "https://example.test/file.bin".into(),
        save_directory: directory.clone(),
        save_directory_explicit: false,
        file_name: Some("file.bin".into()),
        headers: Vec::new(),
        cookie: None,
        referrer: None,
        certificate_policy: Default::default(),
        connection_limit: Some(default_connection_limit),
        speed_limit_bytes_per_second: None,
        expected_total_bytes: None,
        scheduled_at_unix_ms: None,
        adaptive_segment_policy: None,
        media: None,
        category_id: None,
        category_name: None,
        duplicate_strategy: nxtools_download::DownloadDuplicateStrategy::UseDefault,
    };
    window.update(|window, cx| {
        let url = view.read(cx).url.clone();
        url.update(cx, |input, cx| {
            input.set_value("https://example.test/file.bin", window, cx);
        });
        view.update(cx, |view, cx| {
            view.confirmed_request = Some(request);
            view.probe = Some(DownloadProbe {
                resolution: None,
                final_url: "https://example.test/file.bin".into(),
                file_name: Some("file.bin".into()),
                total_bytes: Some(1024),
                supports_ranges: true,
                content_type: Some("application/octet-stream".into()),
                validator: nxtools_download::ResourceValidator::default(),
                media: None,
            });
            cx.notify();
        });
        window.resize(new_download_window_size(true));
    });
    window
        .refresh()
        .expect("render confirmed new download window");
    window.run_until_parked();
    window
        .refresh()
        .expect("render resized confirmed new download window");

    let file_name_field = window
        .debug_bounds("download-new-file-name-field")
        .expect("new download file-name field");
    let category_field = window
        .debug_bounds("download-new-category-field")
        .expect("new download category field");
    let category_trigger = window
        .debug_bounds("download-new-category-trigger")
        .expect("new download category trigger");
    assert!(
        (file_name_field.top().as_f32() - category_field.top().as_f32()).abs() <= 1.0
            && (file_name_field.bottom().as_f32() - category_field.bottom().as_f32()).abs() <= 1.0,
        "file name and category must share one form-row baseline"
    );
    let field_width_ratio =
        category_field.size.width.as_f32() / file_name_field.size.width.as_f32();
    assert!(
        (0.85..=1.15).contains(&field_width_ratio),
        "category is still disproportionately narrow: file={}px category={}px",
        file_name_field.size.width.as_f32(),
        category_field.size.width.as_f32()
    );
    assert!(
        category_trigger.size.width.as_f32() >= category_field.size.width.as_f32() - 2.0,
        "category trigger must fill its field instead of remaining a tiny button"
    );

    let category = window
        .debug_bounds("download-new-category-control")
        .expect("new download category control");
    window.simulate_click(category.center(), Modifiers::default());
    window.run_until_parked();
    window.refresh().expect("render category popover");
    let category_popover = window
        .debug_bounds("download-new-category-popover-content")
        .expect("category popover content");
    assert!(
        category_popover.size.width.as_f32() >= 240.0,
        "category popover remains too narrow at {}px",
        category_popover.size.width.as_f32()
    );
    let category_directory = window
        .debug_bounds("download-new-category-option-directory-0")
        .expect("category directory summary row");
    assert!(
        category_directory.size.height.as_f32() >= 9.0,
        "long category directories must retain a readable summary line"
    );
    let first_category = window
        .debug_bounds("download-new-category-option-0")
        .expect("first explicit category option");
    window.simulate_click(first_category.center(), Modifiers::default());
    window.run_until_parked();
    window
        .refresh()
        .expect("render explicit category selection");
    let selected_category_id = window.read(|cx| {
        view.read(cx)
            .categories
            .first()
            .expect("default category exists")
            .id
            .clone()
    });
    assert_eq!(
        window.read(|cx| view.read(cx).selected_category_id.clone()),
        Some(selected_category_id.clone())
    );

    let schedule = window
        .debug_bounds("download-new-schedule-control")
        .expect("schedule control");
    let footer = window
        .debug_bounds("download-new-footer")
        .expect("new download footer");
    let root = window
        .debug_bounds("download-new-root")
        .expect("new download root");
    assert!(
        schedule.top() >= footer.top() && schedule.bottom() <= footer.bottom(),
        "the schedule control must stay inside the fixed footer"
    );
    assert!(
        footer.bottom() <= root.bottom(),
        "the footer must stay inside the new download window"
    );
    let leading_actions = window
        .debug_bounds("download-new-footer-leading-actions")
        .expect("leading footer actions");
    let submission_actions = window
        .debug_bounds("download-new-footer-submission-actions")
        .expect("submission footer actions");
    assert!(
        leading_actions.left() >= footer.left()
            && leading_actions.right() < submission_actions.left()
            && submission_actions.right() <= footer.right(),
        "secondary controls must stay left while submit controls form one right-side group"
    );
    for (name, selector, role_selector) in [
        (
            "schedule",
            "download-new-schedule-control",
            "download-new-footer-secondary-schedule-action",
        ),
        (
            "queue",
            "download-add-to-queue",
            "download-new-footer-secondary-queue-action",
        ),
        (
            "download",
            "download-start-now",
            "download-new-footer-primary-action",
        ),
        (
            "cancel",
            "download-new-cancel-action",
            "download-new-footer-secondary-cancel-action",
        ),
    ] {
        let action = window
            .debug_bounds(selector)
            .unwrap_or_else(|| panic!("missing {name} footer action"));
        let role = window
            .debug_bounds(role_selector)
            .unwrap_or_else(|| panic!("missing {name} footer action role"));
        assert_eq!(
            action.size.height,
            px(24.0),
            "{name} footer action must use the exact shared action tier"
        );
        assert_eq!(role, action, "{name} role must identify the visible action");
    }
    let refresh = window
        .debug_bounds("download-new-refresh-action")
        .expect("refresh footer utility");
    assert_eq!(
        refresh.size,
        size(px(24.0), px(24.0)),
        "refresh must remain a square shared action"
    );
    let footer_actions = [
        schedule,
        refresh,
        window
            .debug_bounds("download-add-to-queue")
            .expect("queue-only footer action"),
        window
            .debug_bounds("download-start-now")
            .expect("start-now footer action"),
        window
            .debug_bounds("download-new-cancel-action")
            .expect("cancel footer action"),
    ];
    let footer_action_top = footer_actions[0].top().as_f32();
    let footer_action_bottom = footer_actions[0].bottom().as_f32();
    for (index, action) in footer_actions.iter().enumerate().skip(1) {
        assert!(
            (action.top().as_f32() - footer_action_top).abs() <= 1.0
                && (action.bottom().as_f32() - footer_action_bottom).abs() <= 1.0,
            "footer action {index} must share the schedule button's vertical baseline"
        );
    }
    assert!(
        (schedule.left().as_f32() - (footer.left().as_f32() + 16.0)).abs() <= 1.0,
        "the schedule action must align with the footer's leading inset"
    );
    assert!(
        (footer_actions[4].right().as_f32() - (footer.right().as_f32() - 16.0)).abs() <= 1.0,
        "the cancel action must align with the footer's trailing inset"
    );
    window.simulate_click(schedule.center(), Modifiers::default());
    window.run_until_parked();
    window.refresh().expect("render opened schedule popover");

    let popup = window
        .debug_bounds("download-new-schedule-popover-content")
        .expect("date and time popover");
    let root_after_open = window
        .debug_bounds("download-new-root")
        .expect("new download root after opening schedule");
    let footer_after_open = window
        .debug_bounds("download-new-footer")
        .expect("new download footer after opening schedule");
    assert_eq!(
        root_after_open, root,
        "opening the date-time picker must not resize the download window"
    );
    assert_eq!(
        footer_after_open, footer,
        "opening the date-time picker must not move or resize the footer"
    );
    assert!(
        popup.left() >= root.left()
            && popup.right() <= root.right()
            && popup.top() >= root.top()
            && popup.bottom() <= root.bottom(),
        "the date-time picker must snap inside the download window"
    );
    let confirm = window
        .debug_bounds("download-new-schedule-confirm")
        .expect("schedule confirm action");
    window.simulate_click(confirm.center(), Modifiers::default());
    window.run_until_parked();
    window.refresh().expect("render committed schedule");
    window.run_until_parked();
    window.refresh().expect("render closed schedule popover");
    let committed = window
        .read(|cx| view.read(cx).committed_schedule)
        .expect("confirming the date-time picker must commit the delayed start");
    assert!(
        !window.read(|cx| view.read(cx).schedule_popover_open),
        "confirming the date-time picker must close the popover"
    );

    let schedule = window
        .debug_bounds("download-new-schedule-control")
        .expect("committed schedule control");
    window.simulate_click(schedule.center(), Modifiers::default());
    window.run_until_parked();
    window.refresh().expect("reopen schedule popover");
    window.update(|_, cx| {
        view.update(cx, |view, cx| {
            view.scheduled_date = committed.date + ChronoDuration::days(1);
            cx.notify();
        });
    });
    window.refresh().expect("render modified schedule draft");
    window.simulate_click(
        gpui_kit::point(root.left() + px(3.0), root.top() + px(17.0)),
        Modifiers::default(),
    );
    window.run_until_parked();
    window.refresh().expect("dismiss modified schedule draft");
    assert!(
        !window.read(|cx| view.read(cx).schedule_popover_open),
        "clicking outside must close the date-time picker"
    );
    assert_eq!(
        window.read(|cx| view.read(cx).committed_schedule),
        Some(committed),
        "clicking outside must discard the modified draft"
    );
    let request = window
        .read(|cx| view.read(cx).request(cx))
        .expect("request after discarding a modified schedule draft");
    assert_eq!(
        request.scheduled_at_unix_ms,
        Some(
            committed
                .resolve_unix_ms(Local::now().timestamp_millis())
                .expect("committed schedule remains in the future")
        ),
        "submission must continue to use the last confirmed schedule"
    );
    assert_eq!(request.category_id, Some(selected_category_id));
    let _ = std::fs::remove_dir_all(directory);
}

#[gpui_kit::test]
fn confirmed_hls_master_submits_the_selected_quality_as_typed_media(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (application, directory) =
        download_application_for_window_test(cx, "new-download-hls-quality");
    let application_for_window = application.clone();
    let default_directory = directory.clone();
    let default_connection_limit = cx.read(|cx| application.read(cx).default_connection_limit());
    let default_adaptive_segment_policy =
        cx.read(|cx| application.read(cx).default_adaptive_segment_policy());
    let view_slot = Rc::new(RefCell::new(None));
    let opened_view_slot = Rc::clone(&view_slot);
    let (_root, window) = cx.add_window_view(move |window, cx| {
        let view = cx.new(|cx| {
            NewDownloadView::new(
                NewDownloadViewInit {
                    application: application_for_window,
                    default_directory,
                    default_connection_limit,
                    default_adaptive_segment_policy,
                    categories: DownloadSettings::default().categories,
                    initial_source: None,
                },
                window,
                cx,
            )
        });
        *opened_view_slot.borrow_mut() = Some(view.clone());
        Root::new(view, window, cx)
    });
    let view = view_slot
        .borrow()
        .clone()
        .expect("new download view mounted");
    let request = DownloadRequest {
        source_url: "https://example.test/master.m3u8".into(),
        save_directory: directory.clone(),
        save_directory_explicit: false,
        file_name: Some("master.m3u8".into()),
        headers: Vec::new(),
        cookie: None,
        referrer: None,
        certificate_policy: Default::default(),
        connection_limit: Some(default_connection_limit),
        speed_limit_bytes_per_second: None,
        expected_total_bytes: None,
        scheduled_at_unix_ms: None,
        adaptive_segment_policy: None,
        media: None,
        category_id: None,
        category_name: None,
        duplicate_strategy: nxtools_download::DownloadDuplicateStrategy::UseDefault,
    };
    window.update(|window, cx| {
        let url = view.read(cx).url.clone();
        url.update(cx, |input, cx| {
            input.set_value("https://example.test/master.m3u8", window, cx);
        });
        view.update(cx, |view, cx| {
            view.confirmed_request = Some(request);
            view.probe = Some(DownloadProbe {
                resolution: None,
                final_url: "https://example.test/master.m3u8".into(),
                file_name: Some("master.m3u8".into()),
                total_bytes: Some(512),
                supports_ranges: false,
                content_type: Some("application/vnd.apple.mpegurl".into()),
                validator: nxtools_download::ResourceValidator::default(),
                media: Some(nxtools_download::DownloadMediaProbe::Hls(
                    nxtools_download::HlsProbe {
                        is_master: true,
                        variants: vec![
                            nxtools_download::HlsVariantProbe {
                                playlist_url: "https://example.test/high.m3u8".into(),
                                bandwidth: 4_200_000,
                                average_bandwidth: Some(3_900_000),
                                resolution: Some((1920, 1080)),
                                frame_rate_milli: Some(60_000),
                                codecs: "avc1.640028,mp4a.40.2".into(),
                                audio_group: Some("stereo".into()),
                                subtitles_group: None,
                            },
                            nxtools_download::HlsVariantProbe {
                                playlist_url: "https://example.test/low.m3u8".into(),
                                bandwidth: 800_000,
                                average_bandwidth: Some(700_000),
                                resolution: Some((640, 360)),
                                frame_rate_milli: Some(29_970),
                                codecs: "avc1.4d401e,mp4a.40.2".into(),
                                audio_group: Some("stereo".into()),
                                subtitles_group: None,
                            },
                        ],
                    },
                )),
            });
            cx.notify();
        });
    });
    window.refresh().expect("render HLS quality choices");
    window.run_until_parked();
    window.refresh().expect("refresh HLS quality choices");

    assert!(window.debug_bounds("download-hls-quality-0").is_some());
    assert!(window.debug_bounds("download-hls-quality-1").is_some());
    let first_row = window
        .debug_bounds("download-hls-quality-0")
        .expect("first HLS quality row");
    let first_radio = kit_media_radio_snapshot(window, "download-hls-quality-group", 0);
    let first_title = window
        .debug_bounds("download-hls-quality-0-title")
        .expect("first HLS quality title");
    let first_detail = window
        .debug_bounds("download-hls-quality-0-detail")
        .expect("first HLS quality detail");
    assert_eq!(first_radio.role(), Some(gpui_kit::Role::RadioButton));
    assert_eq!(first_radio.checked(), Some(true));
    assert_eq!(
        first_radio.bounds().size.height,
        px(SETTINGS_ROW_MIN_HEIGHT)
    );
    assert_eq!(first_row.size.height, px(SETTINGS_ROW_MIN_HEIGHT));
    assert!(first_title.left() >= first_row.left() && first_detail.right() <= first_row.right());
    assert!(first_detail.bottom() <= first_row.bottom());
    assert_eq!(first_title.left(), first_detail.left());
    assert!(first_title.bottom() <= first_detail.top());
    assert_eq!(
        kit_media_radio_snapshot(window, "download-hls-quality-group", 1).checked(),
        Some(false)
    );
    let lower_quality = window
        .debug_bounds("download-hls-quality-1")
        .expect("lower HLS quality row");
    window.simulate_click(lower_quality.center(), Modifiers::default());
    window.run_until_parked();
    window.refresh().expect("render changed HLS selection");
    assert_eq!(
        window.read(|cx| view.read(cx).selected_hls_variant),
        Some(1)
    );
    assert_eq!(
        kit_media_radio_snapshot(window, "download-hls-quality-group", 1).checked(),
        Some(true)
    );
    assert_eq!(
        kit_media_radio_snapshot(window, "download-hls-quality-group", 0).checked(),
        Some(false),
        "previous Radio becomes unselected"
    );
    let add_to_queue = window
        .debug_bounds("download-add-to-queue")
        .expect("enabled HLS queue button");
    window.simulate_click(add_to_queue.center(), Modifiers::default());
    window.run_until_parked();
    cx.run_until_parked();

    let tasks = cx.read(|cx| {
        application
            .read(cx)
            .engine
            .tasks()
            .expect("read submitted tasks")
    });
    assert_eq!(tasks.len(), 1);
    assert_eq!(
        tasks[0].task.request().media,
        Some(nxtools_download::DownloadRequestMedia::Hls(
            nxtools_download::HlsDownloadRequest {
                playlist_url: "https://example.test/low.m3u8".into(),
            }
        ))
    );

    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn hls_variant_without_resolution_is_not_presented_as_automatic_quality() {
    let variant = nxtools_download::HlsVariantProbe {
        playlist_url: "https://example.test/gear4/prog_index.m3u8".into(),
        bandwidth: 1_927_833,
        average_bandwidth: None,
        resolution: None,
        frame_rate_milli: None,
        codecs: "mp4a.40.2, avc1.4d401f".into(),
        audio_group: None,
        subtitles_group: None,
    };

    assert_eq!(hls_variant_primary_label(&variant), "视频流");
    assert_eq!(
        hls_variant_detail_label(&variant),
        "mp4a.40.2, avc1.4d401f · 1.93 Mbps"
    );
}

#[test]
fn media_probe_window_height_tracks_the_visible_choice_rows() {
    let dash_probe = DownloadProbe {
        resolution: None,
        final_url: "https://example.test/manifest.mpd".into(),
        file_name: Some("manifest.mpd".into()),
        total_bytes: None,
        supports_ranges: false,
        content_type: Some("application/dash+xml".into()),
        validator: nxtools_download::ResourceValidator::default(),
        media: Some(nxtools_download::DownloadMediaProbe::Dash(
            nxtools_download::DashProbe {
                video_tracks: vec![nxtools_download::DashTrackProbe {
                    representation_id: "video".into(),
                    bandwidth: 5_100_000,
                    resolution: Some((1920, 1080)),
                    codecs: "avc1.64002a".into(),
                    mime_type: "video/mp4".into(),
                    language: None,
                    segment_count: 30,
                    has_initialization: true,
                }],
                audio_tracks: vec![nxtools_download::DashTrackProbe {
                    representation_id: "audio".into(),
                    bandwidth: 130_000,
                    resolution: None,
                    codecs: "mp4a.40.2".into(),
                    mime_type: "audio/mp4".into(),
                    language: None,
                    segment_count: 30,
                    has_initialization: true,
                }],
            },
        )),
    };

    assert_eq!(
        new_download_window_size_for_probe(&dash_probe)
            .height
            .as_f32(),
        353.0,
        "a one-video/one-audio DASH choice should use one visible row, not the maximum scroller height"
    );

    let hls_variant = nxtools_download::HlsVariantProbe {
        playlist_url: "https://example.test/video.m3u8".into(),
        bandwidth: 1_000_000,
        average_bandwidth: None,
        resolution: None,
        frame_rate_milli: None,
        codecs: "mp4a.40.2, avc1.4d401e".into(),
        audio_group: None,
        subtitles_group: None,
    };
    let hls_probe = DownloadProbe {
        resolution: None,
        final_url: "https://example.test/master.m3u8".into(),
        file_name: Some("master.m3u8".into()),
        total_bytes: None,
        supports_ranges: false,
        content_type: Some("application/vnd.apple.mpegurl".into()),
        validator: nxtools_download::ResourceValidator::default(),
        media: Some(nxtools_download::DownloadMediaProbe::Hls(
            nxtools_download::HlsProbe {
                is_master: true,
                variants: vec![hls_variant; 5],
            },
        )),
    };
    assert_eq!(
        new_download_window_size_for_probe(&hls_probe)
            .height
            .as_f32(),
        418.0,
        "five HLS choices should use the bounded three-row viewport without extra footer whitespace"
    );
}

#[gpui_kit::test]
fn dash_probe_renders_track_choices_and_builds_a_typed_selection(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (application, directory) =
        download_application_for_window_test(cx, "new-download-dash-tracks");
    let application_for_window = application.clone();
    let default_directory = directory.clone();
    let default_connection_limit = cx.read(|cx| application.read(cx).default_connection_limit());
    let default_adaptive_segment_policy =
        cx.read(|cx| application.read(cx).default_adaptive_segment_policy());
    let view_slot = Rc::new(RefCell::new(None));
    let opened_view_slot = Rc::clone(&view_slot);
    let (_root, window) = cx.add_window_view(move |window, cx| {
        let view = cx.new(|cx| {
            NewDownloadView::new(
                NewDownloadViewInit {
                    application: application_for_window,
                    default_directory,
                    default_connection_limit,
                    default_adaptive_segment_policy,
                    categories: DownloadSettings::default().categories,
                    initial_source: None,
                },
                window,
                cx,
            )
        });
        *opened_view_slot.borrow_mut() = Some(view.clone());
        Root::new(view, window, cx)
    });
    let view = view_slot
        .borrow()
        .clone()
        .expect("new download view mounted");
    let track = |representation_id: &str, resolution| nxtools_download::DashTrackProbe {
        representation_id: representation_id.into(),
        bandwidth: 2_000_000,
        resolution,
        codecs: "avc1.640028".into(),
        mime_type: "video/mp4".into(),
        language: None,
        segment_count: 2,
        has_initialization: true,
    };
    let dash = nxtools_download::DashProbe {
        video_tracks: vec![
            track("v1080", Some((1920, 1080))),
            track("v720", Some((1280, 720))),
        ],
        audio_tracks: vec![nxtools_download::DashTrackProbe {
            representation_id: "a128".into(),
            bandwidth: 128_000,
            resolution: None,
            codecs: "mp4a.40.2".into(),
            mime_type: "audio/mp4".into(),
            language: Some("zh".into()),
            segment_count: 2,
            has_initialization: true,
        }],
    };
    let request = DownloadRequest {
        source_url: "https://example.test/manifest.mpd".into(),
        save_directory: directory.clone(),
        save_directory_explicit: false,
        file_name: Some("manifest.mp4".into()),
        headers: Vec::new(),
        cookie: None,
        referrer: None,
        certificate_policy: Default::default(),
        connection_limit: Some(default_connection_limit),
        speed_limit_bytes_per_second: None,
        expected_total_bytes: None,
        scheduled_at_unix_ms: None,
        adaptive_segment_policy: None,
        media: None,
        category_id: None,
        category_name: None,
        duplicate_strategy: nxtools_download::DownloadDuplicateStrategy::UseDefault,
    };
    window.update(|window, cx| {
        let url = view.read(cx).url.clone();
        url.update(cx, |input, cx| {
            input.set_value("https://example.test/manifest.mpd", window, cx);
        });
        window.resize(size(px(500.0), px(430.0)));
        view.update(cx, |view, cx| {
            view.confirmed_request = Some(request);
            view.probe = Some(DownloadProbe {
                resolution: None,
                final_url: "https://example.test/manifest.mpd".into(),
                file_name: Some("manifest.mpd".into()),
                total_bytes: None,
                supports_ranges: false,
                content_type: Some("application/dash+xml".into()),
                validator: nxtools_download::ResourceValidator::default(),
                media: Some(nxtools_download::DownloadMediaProbe::Dash(dash)),
            });
            view.selected_dash_video_track = Some(0);
            view.selected_dash_audio_track = Some(0);
            cx.notify();
        });
    });
    window.refresh().expect("render DASH track choices");
    let first_video_row = window
        .debug_bounds("download-dash-video-track-0")
        .expect("first DASH video track row");
    let first_video_radio = kit_media_radio_snapshot(window, "视频", 0);
    let first_video_title = window
        .debug_bounds("download-dash-video-track-0-title")
        .expect("first DASH video title");
    let first_video_detail = window
        .debug_bounds("download-dash-video-track-0-detail")
        .expect("first DASH video detail");
    assert_eq!(first_video_radio.role(), Some(gpui_kit::Role::RadioButton));
    assert_eq!(first_video_radio.checked(), Some(true));
    assert_eq!(first_video_row.size.height, px(SETTINGS_ROW_MIN_HEIGHT));
    assert!(first_video_detail.bottom() <= first_video_row.bottom());
    assert_eq!(first_video_title.left(), first_video_detail.left());
    assert!(first_video_title.bottom() <= first_video_detail.top());
    assert_eq!(
        kit_media_radio_snapshot(window, "视频", 1).checked(),
        Some(false)
    );
    let lower_video = window
        .debug_bounds("download-dash-video-track-1")
        .expect("lower DASH video track row");
    assert!(window.debug_bounds("download-dash-audio-track-0").is_some());
    window.simulate_click(lower_video.center(), Modifiers::default());
    window.run_until_parked();
    window.refresh().expect("render changed DASH selection");
    assert_eq!(
        window.read(|cx| view.read(cx).selected_dash_video_track),
        Some(1)
    );
    assert_eq!(
        kit_media_radio_snapshot(window, "视频", 1).checked(),
        Some(true)
    );
    assert_eq!(
        kit_media_radio_snapshot(window, "视频", 0).checked(),
        Some(false)
    );
    let audio = window
        .debug_bounds("download-dash-audio-track-0")
        .expect("selected DASH audio track row");
    window.simulate_click(audio.center(), Modifiers::default());
    window.run_until_parked();
    window.refresh().expect("render deselected DASH audio");
    assert_eq!(
        window.read(|cx| view.read(cx).selected_dash_audio_track),
        None
    );
    assert_eq!(
        kit_media_radio_snapshot(window, "音频", 0).checked(),
        Some(false),
        "optional audio click-to-clear renders unselected Radio"
    );
    let media = window.read(|cx| {
        let view = view.read(cx);
        selected_download_request_media(
            view.probe.as_ref(),
            view.selected_hls_variant,
            view.selected_dash_video_track,
            view.selected_dash_audio_track,
        )
        .expect("build typed DASH request")
    });
    assert_eq!(
        media,
        Some(nxtools_download::DownloadRequestMedia::Dash(
            nxtools_download::DashDownloadRequest {
                manifest_url: "https://example.test/manifest.mpd".into(),
                video_representation_id: Some("v720".into()),
                audio_representation_id: None,
            }
        ))
    );
    let add_to_queue = window
        .debug_bounds("download-add-to-queue")
        .expect("enabled DASH queue button");
    window.simulate_click(add_to_queue.center(), Modifiers::default());
    window.run_until_parked();
    cx.run_until_parked();
    let tasks = cx.read(|cx| {
        application
            .read(cx)
            .engine
            .tasks()
            .expect("read submitted DASH tasks")
    });
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].task.request().media, media);
    let _ = std::fs::remove_dir_all(directory);
}

#[gpui_kit::test]
fn browser_prefilled_download_window_preserves_private_request_context(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (application, directory) =
        download_application_for_window_test(cx, "browser-prefilled-download");
    let application_for_window = application.clone();
    let initial = DownloadRequest {
        source_url: "https://example.test/private.bin".into(),
        save_directory: directory.clone(),
        save_directory_explicit: false,
        file_name: Some("private.bin".into()),
        headers: vec![nxtools_download::DownloadRequestHeader {
            name: "Authorization".into(),
            value: "Bearer secret".into(),
        }],
        cookie: Some("session=secret".into()),
        referrer: Some("https://example.test/page".into()),
        certificate_policy: Default::default(),
        connection_limit: Some(4),
        speed_limit_bytes_per_second: Some(512_000),
        expected_total_bytes: Some(8_192),
        scheduled_at_unix_ms: None,
        adaptive_segment_policy: None,
        media: None,
        category_id: None,
        category_name: None,
        duplicate_strategy: nxtools_download::DownloadDuplicateStrategy::UseDefault,
    };
    let expected = initial.clone();
    let default_directory = directory.clone();
    let view_slot = Rc::new(RefCell::new(None));
    let opened_view_slot = Rc::clone(&view_slot);
    let (_root, _window) = cx.add_window_view(move |window, cx| {
        let view = cx.new(|cx| {
            NewDownloadView::new(
                NewDownloadViewInit {
                    application: application_for_window,
                    default_directory,
                    default_connection_limit: 4,
                    default_adaptive_segment_policy:
                        nxtools_download::DownloadAdaptiveSegmentPolicy::default(),
                    categories: DownloadSettings::default().categories,
                    initial_source: Some(NewDownloadInitialSource::Download(initial)),
                },
                window,
                cx,
            )
        });
        *opened_view_slot.borrow_mut() = Some(view.clone());
        Root::new(view, window, cx)
    });
    let view = view_slot
        .borrow()
        .clone()
        .expect("browser-prefilled view mounted");
    let reconstructed = cx.read(|cx| view.read(cx).request(cx).expect("rebuild request"));

    assert_eq!(reconstructed.source_url, expected.source_url);
    assert_eq!(reconstructed.save_directory, expected.save_directory);
    assert_eq!(reconstructed.file_name, expected.file_name);
    assert_eq!(reconstructed.headers, expected.headers);
    assert_eq!(reconstructed.cookie, expected.cookie);
    assert_eq!(reconstructed.referrer, expected.referrer);
    assert_eq!(
        reconstructed.expected_total_bytes,
        expected.expected_total_bytes
    );
    assert_eq!(
        reconstructed.speed_limit_bytes_per_second,
        expected.speed_limit_bytes_per_second
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[gpui_kit::test]
fn browser_blob_window_is_prefilled_from_staged_metadata_without_network_probe(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_kit::init);
    let (application, directory) =
        download_application_for_window_test(cx, "browser-prefilled-blob");
    let application_for_window = application.clone();
    let default_directory = directory.clone();
    let blob_id = uuid::Uuid::new_v4();
    let view_slot = Rc::new(RefCell::new(None));
    let opened_view_slot = Rc::clone(&view_slot);
    let (_root, _window) = cx.add_window_view(move |window, cx| {
        let view = cx.new(|cx| {
            NewDownloadView::new(
                NewDownloadViewInit {
                    application: application_for_window,
                    default_directory: default_directory.clone(),
                    default_connection_limit: 4,
                    default_adaptive_segment_policy:
                        nxtools_download::DownloadAdaptiveSegmentPolicy::default(),
                    categories: DownloadSettings::default().categories,
                    initial_source: Some(NewDownloadInitialSource::Blob(
                        BrowserBlobDialogContext {
                            correlation_id: uuid::Uuid::new_v4(),
                            handoff_id: uuid::Uuid::new_v4(),
                            blob: BrowserBlob {
                                id: blob_id,
                                file_name: "captured.webm".into(),
                                media_type: Some("video/webm".into()),
                                size: 4096,
                            },
                            save_directory: default_directory,
                        },
                    )),
                },
                window,
                cx,
            )
        });
        *opened_view_slot.borrow_mut() = Some(view.clone());
        Root::new(view, window, cx)
    });
    let view = view_slot.borrow().clone().expect("Blob view mounted");
    cx.read(|cx| {
        let view = view.read(cx);
        assert_eq!(
            view.external_blob.as_ref().map(|value| value.blob.id),
            Some(blob_id)
        );
        assert_eq!(view.file_name.read(cx).value(), "captured.webm");
        assert_eq!(
            view.directory.read(cx).value(),
            directory.to_string_lossy().as_ref()
        );
        let probe = view.probe.as_ref().expect("Blob metadata is confirmed");
        assert_eq!(probe.total_bytes, Some(4096));
        assert_eq!(probe.content_type.as_deref(), Some("video/webm"));
        assert!(!view.probing);
    });
    let _ = std::fs::remove_dir_all(directory);
}

pub(super) fn download_application_for_window_test(
    cx: &mut TestAppContext,
    label: &str,
) -> (Entity<DownloadApplication>, PathBuf) {
    let (application, directory, _platform_endpoint) =
        download_application_for_window_test_with_platform(cx, label);
    (application, directory)
}

fn assert_shared_dialog_shell(
    window: &mut VisualTestContext,
    shell_selector: &'static str,
    titlebar_selector: &'static str,
    body_selector: &'static str,
    footer_selector: &'static str,
) {
    let shell = window
        .debug_bounds(shell_selector)
        .expect("shared dialog shell");
    let titlebar = window
        .debug_bounds(titlebar_selector)
        .expect("shared dialog titlebar");
    let body = window
        .debug_bounds(body_selector)
        .expect("shared dialog body");
    let footer = window
        .debug_bounds(footer_selector)
        .expect("shared dialog footer");
    assert_eq!(titlebar.size.height, px(34.));
    assert_eq!(
        footer.size.height,
        px(crate::ui::metrics::SETTINGS_FOOTER_HEIGHT)
    );
    assert_eq!(titlebar.bottom(), body.top());
    assert_eq!(body.bottom(), footer.top());
    assert_eq!(footer.bottom(), shell.bottom());
}

fn download_application_for_window_test_with_platform(
    cx: &mut TestAppContext,
    label: &str,
) -> (
    Entity<DownloadApplication>,
    PathBuf,
    nxtools_ipc::PlatformServerEndpoint,
) {
    let directory = unique_download_settings_test_directory(label);
    std::fs::create_dir_all(&directory).expect("create download window test directory");
    let engine = DownloadEngine::open_in_memory().expect("open download window test engine");
    let (platform, platform_endpoint) = nxtools_ipc::platform_bridge(16);
    let notifications = cx.update(|cx| cx.new(|_| NotificationCenter::default()));
    let application = cx.update(|cx| {
        cx.new(|cx| {
            DownloadApplication::new(
                engine,
                Rc::new(platform),
                notifications,
                DownloadRuntimeContext {
                    fallback_default_directory: directory.clone(),
                    config: AppConfig::default(),
                    config_path: directory.join("config.json"),
                    log_directory: directory.clone(),
                    database_path: directory.join("downloads.sqlite3"),
                    ffmpeg_install_directory: directory.join("tools").join("ffmpeg"),
                },
                cx,
            )
        })
    });
    (application, directory, platform_endpoint)
}

#[gpui_kit::test]
fn schedule_dialog_uses_shared_shell_and_preserves_cancel_close_apply_lifecycle(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_kit::init);
    let (application, directory) = download_application_for_window_test(cx, "schedule-window");

    cx.update(|cx| {
        application.update(cx, |application, cx| {
            application.open_schedule_window(cx);
        });
    });
    cx.run_until_parked();

    let window_handle = cx
        .read(|cx| application.read(cx).schedule_window)
        .expect("production schedule window");
    let mut window = VisualTestContext::from_window(window_handle, cx);
    window.refresh().expect("render production schedule dialog");
    assert_shared_dialog_shell(
        &mut window,
        "download-schedule-dialog-shell",
        "download-schedule-dialog-titlebar",
        "download-schedule-dialog-body",
        "download-schedule-dialog-footer",
    );
    let cancel = window
        .debug_bounds("download-schedule-secondary-cancel-action")
        .expect("schedule cancel secondary action");
    let apply = window
        .debug_bounds("download-schedule-primary-apply-action")
        .expect("schedule apply primary action");
    assert_eq!(cancel.size.height, px(24.0));
    assert_eq!(apply.size.height, px(24.0));
    assert!(cancel.right() <= apply.left());
    window.simulate_click(cancel.center(), Modifiers::default());
    window.run_until_parked();
    assert!(window_handle.update(cx, |_, _, _| ()).is_err());
    assert!(cx.read(|cx| application.read(cx).schedule_window.is_none()));

    cx.update(|cx| {
        application.update(cx, |application, cx| {
            application.open_schedule_window(cx);
        });
    });
    cx.run_until_parked();
    let native_close_handle = cx
        .read(|cx| application.read(cx).schedule_window)
        .expect("reopened schedule window for native close");
    let mut native_close_window = VisualTestContext::from_window(native_close_handle, cx);
    native_close_window
        .refresh()
        .expect("render schedule draft before native close");
    let start_toggle = native_close_window
        .debug_bounds("download-schedule-start-toggle")
        .expect("schedule start toggle");
    native_close_window.simulate_click(start_toggle.center(), Modifiers::default());
    native_close_window.run_until_parked();
    assert!(native_close_window.simulate_close());
    native_close_window.run_until_parked();
    assert!(cx.read(|cx| {
        let application = application.read(cx);
        application.schedule_window.is_none()
            && application
                .default_queue()
                .is_none_or(|queue| queue.schedule.is_none())
    }));

    cx.update(|cx| {
        application.update(cx, |application, cx| {
            application.open_schedule_window(cx);
        });
    });
    cx.run_until_parked();
    let apply_handle = cx
        .read(|cx| application.read(cx).schedule_window)
        .expect("reopened schedule window for apply");
    let mut apply_window = VisualTestContext::from_window(apply_handle, cx);
    apply_window
        .refresh()
        .expect("render schedule apply dialog");
    let start_toggle = apply_window
        .debug_bounds("download-schedule-start-toggle")
        .expect("schedule start toggle after discarded draft");
    apply_window.simulate_click(start_toggle.center(), Modifiers::default());
    apply_window.run_until_parked();
    let apply = apply_window
        .debug_bounds("download-schedule-primary-apply-action")
        .expect("schedule apply action after discarded draft");
    apply_window.simulate_click(apply.center(), Modifiers::default());
    apply_window.run_until_parked();
    assert!(apply_handle.update(cx, |_, _, _| ()).is_err());
    assert!(cx.read(|cx| {
        let application = application.read(cx);
        application.schedule_window.is_none()
            && application.default_queue().is_some_and(|queue| {
                queue
                    .schedule
                    .as_ref()
                    .is_some_and(|schedule| schedule.daily.enabled_start)
            })
    }));
    let _ = std::fs::remove_dir_all(directory);
}

#[gpui_kit::test]
fn delete_dialog_uses_shared_danger_semantics_and_closes_only_after_a_decision(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_kit::init);
    let (application, directory) = download_application_for_window_test(cx, "delete-dialog-shell");
    let task_id = cx.read(|cx| {
        application
            .read(cx)
            .engine
            .enqueue(
                DownloadRequest {
                    source_url: "https://example.test/delete-dialog.bin".into(),
                    save_directory: directory.clone(),
                    save_directory_explicit: false,
                    file_name: Some("delete-dialog.bin".into()),
                    headers: Vec::new(),
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
            .expect("enqueue delete-dialog fixture")
    });
    cx.read(|cx| {
        application
            .read(cx)
            .engine
            .cancel(task_id)
            .expect("cancel delete-dialog fixture")
    });
    cx.update(|cx| {
        application.update(cx, |application, cx| {
            application.refresh(cx);
            application.request_delete_confirmation(task_id, cx);
        });
    });
    cx.run_until_parked();

    let first_window_handle = cx
        .read(|cx| {
            application
                .read(cx)
                .delete_confirmation_windows
                .get(&task_id)
                .copied()
        })
        .expect("production delete dialog");
    let mut first_window = VisualTestContext::from_window(first_window_handle, cx);
    first_window
        .refresh()
        .expect("render production delete dialog");
    assert_shared_dialog_shell(
        &mut first_window,
        "download-delete-dialog-shell",
        "download-delete-dialog-titlebar",
        "download-delete-dialog-body",
        "download-delete-dialog-footer",
    );
    let cancel = first_window
        .debug_bounds("download-delete-secondary-cancel-action")
        .expect("delete cancel secondary action");
    let confirm = first_window
        .debug_bounds("download-delete-danger-confirm-action")
        .expect("delete danger confirmation action");
    assert_eq!(cancel.size.height, px(24.0));
    assert_eq!(confirm.size.height, px(24.0));
    assert!(cancel.right() <= confirm.left());
    first_window.simulate_click(cancel.center(), Modifiers::default());
    first_window.run_until_parked();
    assert!(first_window_handle.update(cx, |_, _, _| ()).is_err());
    assert!(cx.read(|cx| {
        let application = application.read(cx);
        !application
            .delete_confirmation_windows
            .contains_key(&task_id)
            && application.task(task_id).is_some()
    }));

    cx.update(|cx| {
        application.update(cx, |application, cx| {
            application.request_delete_confirmation(task_id, cx);
        });
    });
    cx.run_until_parked();
    let native_close_handle = cx
        .read(|cx| {
            application
                .read(cx)
                .delete_confirmation_windows
                .get(&task_id)
                .copied()
        })
        .expect("reopened delete dialog for native close");
    let mut native_close_window = VisualTestContext::from_window(native_close_handle, cx);
    native_close_window
        .refresh()
        .expect("render delete dialog before native close");
    assert!(native_close_window.simulate_close());
    native_close_window.run_until_parked();
    assert!(cx.read(|cx| {
        let application = application.read(cx);
        !application
            .delete_confirmation_windows
            .contains_key(&task_id)
            && application.task(task_id).is_some()
    }));

    cx.update(|cx| {
        application.update(cx, |application, cx| {
            application.request_delete_confirmation(task_id, cx);
        });
    });
    cx.run_until_parked();
    let confirm_window_handle = cx
        .read(|cx| {
            application
                .read(cx)
                .delete_confirmation_windows
                .get(&task_id)
                .copied()
        })
        .expect("reopened production delete dialog for confirmation");
    let mut confirm_window = VisualTestContext::from_window(confirm_window_handle, cx);
    confirm_window
        .refresh()
        .expect("render confirmed delete dialog");
    let confirm = confirm_window
        .debug_bounds("download-delete-danger-confirm-action")
        .expect("reopened delete confirmation action");
    confirm_window.simulate_click(confirm.center(), Modifiers::default());
    confirm_window.run_until_parked();
    assert!(confirm_window_handle.update(cx, |_, _, _| ()).is_err());
    assert!(cx.read(|cx| {
        let application = application.read(cx);
        !application
            .delete_confirmation_windows
            .contains_key(&task_id)
            && application.task(task_id).is_none()
    }));
    let _ = std::fs::remove_dir_all(directory);
}

#[gpui_kit::test]
fn settings_general_page_mounts_the_double_click_action_control(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (application, directory) =
        download_application_for_window_test(cx, "download-double-click-control");

    cx.update(|cx| {
        application.update(cx, |application, cx| {
            application.open_settings_window(cx);
        });
    });
    cx.run_until_parked();

    let view = cx
        .read(|cx| application.read(cx).settings_view.clone())
        .expect("settings view handle");
    let carries_state = view.update(cx, |view, _| {
        view.double_click_action == DownloadRowDoubleClickActionSettings::OpenFile
    });
    assert!(
        carries_state,
        "settings view must carry the double-click state"
    );
    let general_source = include_str!("../downloads/settings_view.rs");
    assert!(
        general_source.contains("(DownloadSettingsPage::General, 4) => double_click_action"),
        "the same double-click control belongs to official General item4"
    );
    assert!(
        general_source.contains("download-settings-double-click-action"),
        "the segmented control helper must expose stable debug ids"
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[gpui_kit::test]
fn download_settings_window_opens_without_reentering_download_application(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (application, directory) =
        download_application_for_window_test(cx, "download-settings-window");

    cx.update(|cx| {
        application.update(cx, |application, cx| {
            application.open_settings_window(cx);
        });
    });
    cx.run_until_parked();

    let settings_window = cx
        .read(|cx| application.read(cx).settings_window)
        .expect("production settings window");
    let actual_size = settings_window
        .update(cx, |_, window, _| window.bounds().size)
        .expect("read production settings window bounds");
    assert_eq!(actual_size.width, px(DOWNLOAD_SETTINGS_WINDOW_WIDTH));
    assert_eq!(actual_size.height, px(DOWNLOAD_SETTINGS_WINDOW_HEIGHT));
    cx.read(|cx| {
        let options = download_settings_window_options(cx);
        assert!(!options.is_resizable);
        assert_eq!(
            options.window_min_size,
            Some(size(
                px(DOWNLOAD_SETTINGS_WINDOW_WIDTH),
                px(DOWNLOAD_SETTINGS_WINDOW_HEIGHT)
            ))
        );
    });
    let _ = std::fs::remove_dir_all(directory);
}

#[gpui_kit::test]
fn completion_feedback_crosses_the_bounded_platform_transport_once_per_batch(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_kit::init);
    let directory = unique_download_settings_test_directory("completion-feedback");
    std::fs::create_dir_all(&directory).expect("create completion feedback directory");
    let mut config = AppConfig::default();
    config.download.completion_sound_enabled = true;
    config.download.open_directory_on_completion = true;
    let engine = DownloadEngine::open_in_memory().expect("open completion feedback engine");
    let (platform, platform_endpoint) = nxtools_ipc::platform_bridge(16);
    platform_endpoint.set_connected_for_contract_test(true);
    let notifications = cx.update(|cx| cx.new(|_| NotificationCenter::default()));
    let application = cx.update(|cx| {
        cx.new(|cx| {
            DownloadApplication::new(
                engine,
                Rc::new(platform),
                notifications,
                DownloadRuntimeContext {
                    fallback_default_directory: directory.clone(),
                    config,
                    config_path: directory.join("config.json"),
                    log_directory: directory.clone(),
                    database_path: directory.join("downloads.sqlite3"),
                    ffmpeg_install_directory: directory.join("tools").join("ffmpeg"),
                },
                cx,
            )
        })
    });
    cx.update(|cx| {
        application.update(cx, |application, cx| {
            for index in 0..5 {
                application
                    .completion_actions
                    .record_terminal(CompletionTerminal {
                        task_id: DownloadTaskId::new(),
                        status: DownloadStatus::Completed,
                        directory: Some(directory.join(format!("batch-{index}"))),
                        display_name: format!("batch-{index}.bin"),
                        error_message: None,
                    });
            }
            application.flush_completion_feedback(cx);
        });
    });

    let mut envelopes = Vec::new();
    for _ in 0..4 {
        envelopes.push(
            platform_endpoint
                .try_receive_command_for_contract_test()
                .expect("bounded completion command"),
        );
    }
    assert!(matches!(
        &envelopes[0].message,
        nxtools_protocol::Message::PlatformCommand(PlatformCommand::PlaySystemNotificationSound)
    ));
    assert_eq!(
        envelopes
            .iter()
            .filter(|envelope| matches!(
                &envelope.message,
                nxtools_protocol::Message::PlatformCommand(PlatformCommand::OpenShellTarget(_))
            ))
            .count(),
        3
    );
    let operation_id = envelopes[0]
        .correlation_id
        .expect("shared feedback correlation");
    assert!(
        envelopes
            .iter()
            .all(|envelope| envelope.correlation_id == Some(operation_id))
    );
    cx.update(|cx| {
        application.update(cx, |application, cx| {
            for envelope in envelopes {
                application.handle_platform_notification(
                    PlatformNotification::CommandResult {
                        correlation_id: envelope.id,
                        result: PlatformCommandResult {
                            success: true,
                            error: None,
                            native_error_code: None,
                            data: None,
                        },
                    },
                    cx,
                );
            }
            assert!(application.pending_completion_feedback.is_empty());
            assert!(application.completion_feedback_commands.is_empty());
        });
    });
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn completed_task_window_maps_exactly_one_action_to_primary_emphasis() {
    assert_eq!(CompletedTaskWindowAction::ALL.len(), 3);
    assert_eq!(
        CompletedTaskWindowAction::ALL
            .into_iter()
            .filter(|action| { action.spec().emphasis == CompletedTaskWindowEmphasis::Primary })
            .collect::<Vec<_>>(),
        vec![CompletedTaskWindowAction::Open]
    );
    assert!(
        CompletedTaskWindowAction::ALL
            .into_iter()
            .filter(|action| { action.spec().emphasis == CompletedTaskWindowEmphasis::Outline })
            .all(|action| matches!(
                action,
                CompletedTaskWindowAction::Drag | CompletedTaskWindowAction::Reveal
            ))
    );
}

#[gpui_kit::test]
fn production_task_window_reuses_one_skeleton_and_generation_across_phases(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_kit::init);
    let (application, directory, platform_endpoint) =
        download_application_for_window_test_with_platform(cx, "task-window-shared-skeleton");
    platform_endpoint.set_connected_for_contract_test(true);
    let task_id = cx.read(|cx| {
        application
            .read(cx)
            .engine
            .enqueue(
                DownloadRequest {
                    source_url: "https://example.test/shared-skeleton.bin".into(),
                    save_directory: directory.clone(),
                    save_directory_explicit: false,
                    file_name: Some("shared-skeleton.bin".into()),
                    headers: Vec::new(),
                    cookie: None,
                    referrer: None,
                    certificate_policy: Default::default(),
                    connection_limit: Some(1),
                    speed_limit_bytes_per_second: None,
                    expected_total_bytes: Some(4096),
                    scheduled_at_unix_ms: None,
                    adaptive_segment_policy: None,
                    media: None,
                    category_id: None,
                    category_name: None,
                    duplicate_strategy: nxtools_download::DownloadDuplicateStrategy::UseDefault,
                },
                RequestOrigin::Internal,
            )
            .expect("enqueue shared task-window fixture")
    });
    cx.update(|cx| {
        application.update(cx, |application, cx| {
            application.refresh(cx);
            let _ = application.configure_task_window_activation_test(42);
            application.request_task_window(task_id, cx);
        });
    });
    cx.run_until_parked();

    let (task_window_handle, generation, base_snapshot) = cx.read(|cx| {
        let application = application.read(cx);
        let entry = application
            .task_windows
            .get(&task_id)
            .expect("production keyed task window");
        assert_eq!(entry.native_window_handle, 42);
        (
            entry.window,
            entry.generation,
            application.task(task_id).cloned().expect("task snapshot"),
        )
    });
    let (_progress_bounds, progress_scale) = task_window_handle
        .update(cx, |_, window, _| (window.bounds(), window.scale_factor()))
        .expect("read progress window geometry");
    let mut window = VisualTestContext::from_window(task_window_handle, cx);
    window.refresh().expect("render progress task skeleton");

    let shell = window
        .debug_bounds("download-task-window-shell")
        .expect("shared task-window shell");
    let titlebar = window
        .debug_bounds("download-task-window-titlebar")
        .expect("shared task-window titlebar");
    let body = window
        .debug_bounds("download-task-window-body")
        .expect("shared task-window body");
    let footer = window
        .debug_bounds("download-task-window-footer")
        .expect("shared task-window footer");
    let progress_phase = window
        .debug_bounds("download-task-window-phase-progress")
        .expect("progress phase body");
    assert_eq!(titlebar.size.height, px(34.0));
    assert_eq!(footer.size.height, px(44.0));
    assert_eq!(progress_phase, body);
    assert_eq!(titlebar.bottom(), body.top());
    assert_eq!(body.bottom(), footer.top());
    assert_eq!(footer.bottom(), shell.bottom());
    let progress_identity = window
        .debug_bounds("download-window-progress-identity")
        .expect("download progress identity row");
    let progress_percent = window
        .debug_bounds("download-window-progress-percent")
        .expect("prominent progress value");
    let progress_track = window
        .debug_bounds("download-window-progress-track")
        .expect("dedicated progress track");
    let progress_metrics = window
        .debug_bounds("download-window-progress-metrics")
        .expect("aligned progress metrics");
    let capabilities = window
        .debug_bounds("download-window-progress-capabilities")
        .expect("resume/limit state");
    assert!(progress_percent.size.height >= px(16.0));
    assert!(progress_identity.bottom() <= progress_track.top());
    assert!(progress_track.bottom() <= progress_metrics.top());
    assert!(progress_metrics.bottom() <= capabilities.top());
    assert!(capabilities.bottom() <= footer.top());
    while platform_endpoint
        .try_receive_command_for_contract_test()
        .is_ok()
    {}

    let mut failed_snapshot = base_snapshot.clone();
    failed_snapshot
        .task
        .transition_to(DownloadStatus::Probing, 2)
        .expect("fixture enters probing");
    failed_snapshot
        .task
        .transition_to(DownloadStatus::Failed, 3)
        .expect("fixture enters failed");
    cx.update(|cx| {
        application.update(cx, |application, cx| {
            application.replace_tasks_for_test(vec![failed_snapshot]);
            let directive = application
                .task_window_lifecycle
                .task_changed(task_id, DownloadStatus::Failed);
            assert_eq!(
                directive,
                DownloadWindowDirective::TransformExisting(DownloadWindowPhase::Failed)
            );
            application.apply_task_window_directive(task_id, directive, cx);
        });
    });
    cx.run_until_parked();
    window.refresh().expect("render failed task skeleton");
    let failed_size = download_task_window_size(DownloadWindowPhase::Failed);
    let failed_resize = std::iter::from_fn(|| {
        platform_endpoint
            .try_receive_command_for_contract_test()
            .ok()
    })
    .find_map(|envelope| match envelope.message {
        nxtools_protocol::Message::PlatformCommand(PlatformCommand::SetWindowBoundsCentered(
            command,
        )) => Some(command),
        _ => None,
    })
    .expect("failed phase crosses the native resize boundary");
    assert_eq!(
        failed_resize,
        SetWindowBoundsCenteredCommand {
            window_handle: 42,
            width: (failed_size.width.as_f32() * progress_scale).round() as i32,
            height: (failed_size.height.as_f32() * progress_scale).round() as i32,
        },
        "the centered transform must delegate geometry to the native side"
    );
    assert_eq!(
        window
            .debug_bounds("download-task-window-phase-failed")
            .expect("failed phase body"),
        window
            .debug_bounds("download-task-window-body")
            .expect("failed shared body")
    );

    let failure_detail = window
        .debug_bounds("download-window-failure-detail")
        .expect("readable multi-line failure detail");
    assert!(
        window
            .debug_bounds("download-window-failure-kit-alert")
            .is_some(),
        "queue10 actual task failure Alert"
    );
    let failed_footer = window
        .debug_bounds("download-task-window-footer")
        .expect("failed footer");
    assert!(
        failure_detail.size.height >= px(30.0),
        "the failure message must wrap across readable lines instead of truncating to one"
    );
    assert!(failure_detail.bottom() <= failed_footer.top());
    let mut completed_snapshot = base_snapshot;
    for (status, timestamp) in [
        (DownloadStatus::Probing, 2),
        (DownloadStatus::Waiting, 3),
        (DownloadStatus::Downloading, 4),
        (DownloadStatus::Finalizing, 5),
        (DownloadStatus::Completed, 6),
    ] {
        completed_snapshot
            .task
            .transition_to(status, timestamp)
            .expect("fixture advances to completion");
    }
    completed_snapshot.total_bytes = Some(4096);
    completed_snapshot.downloaded_bytes = 4096;
    let final_artifact = directory.join("shared-skeleton.bin");
    std::fs::write(&final_artifact, b"fixture").expect("create completed artifact");
    completed_snapshot.final_path = Some(final_artifact);
    cx.update(|cx| {
        application.update(cx, |application, cx| {
            application.replace_tasks_for_test(vec![completed_snapshot]);
            let directive = application
                .task_window_lifecycle
                .task_changed(task_id, DownloadStatus::Completed);
            assert_eq!(
                directive,
                DownloadWindowDirective::TransformExisting(DownloadWindowPhase::Completed)
            );
            application.apply_task_window_directive(task_id, directive, cx);
        });
    });
    cx.run_until_parked();
    window.refresh().expect("render completed task skeleton");
    assert_eq!(
        window
            .debug_bounds("download-task-window-phase-completed")
            .expect("completed phase body"),
        window
            .debug_bounds("download-task-window-body")
            .expect("completed shared body")
    );
    let completed_summary = window
        .debug_bounds("download-complete-summary")
        .expect("completed file summary");
    let completed_name = window
        .debug_bounds("download-complete-name")
        .expect("completed file name");
    let completed_size = window
        .debug_bounds("download-complete-size")
        .expect("completed file size");
    let completed_path = window
        .debug_bounds("download-complete-path")
        .expect("completed destination");
    for item in [completed_name, completed_size, completed_path] {
        assert!(item.left() >= completed_summary.left());
        assert!(item.right() <= completed_summary.right());
        assert!(item.top() >= completed_summary.top());
        assert!(item.bottom() <= completed_summary.bottom());
    }
    assert!(completed_name.top() <= completed_size.top());
    assert!(completed_size.bottom() <= completed_path.top());
    assert!(
        completed_path.bottom()
            <= window
                .debug_bounds("download-task-window-footer")
                .expect("completed footer")
                .top()
    );
    let drag_action = window
        .debug_bounds("download-complete-secondary-drag-action")
        .expect("completed drag secondary action");
    let reveal_action = window
        .debug_bounds("download-complete-secondary-reveal-action")
        .expect("completed reveal secondary action");
    let open_action = window
        .debug_bounds("download-complete-primary-open-action")
        .expect("completed open primary action");
    for action in [drag_action, reveal_action, open_action] {
        assert_eq!(action.size.height, px(24.0));
    }
    assert_eq!(drag_action.size.width, px(24.0));
    assert!(drag_action.right() <= reveal_action.left());
    assert!(reveal_action.right() <= open_action.left());
    let open_with = window
        .debug_bounds("download-complete-open-with-action")
        .expect("Open With action");
    assert_eq!(
        open_with.size,
        size(px(24.0), px(24.0)),
        "Open With is icon-only, not a labelled button"
    );
    assert!(
        open_action.right() <= open_with.left(),
        "Open With belongs to the right of Open File"
    );
    assert!(
        window
            .update(|window, _| gpui_kit::base::test_support::snapshots(window))
            .iter()
            .any(|node| node.role() == Some(gpui_kit::Role::Button)
                && node.label() == Some("选择应用打开下载文件")),
        "icon-only chooser retains an accessible purpose"
    );
    assert!(
        window
            .debug_bounds("download-complete-delete-action")
            .is_none(),
        "delete must not enter the completed-window action path"
    );
    let executor = cx.executor();
    window.simulate_mouse_move(drag_action.center(), None, Modifiers::default());
    executor.advance_clock(Duration::from_millis(600));
    window.run_until_parked();
    window
        .refresh()
        .expect("render completed drag action tooltip");
    assert!(
        window
            .debug_bounds("download-complete-drag-tooltip")
            .is_some(),
        "the completed icon-only drag action must expose its Chinese tooltip"
    );
    while platform_endpoint
        .try_receive_command_for_contract_test()
        .is_ok()
    {}

    window.simulate_mouse_down(
        drag_action.center(),
        MouseButton::Left,
        Modifiers::default(),
    );
    window.run_until_parked();
    let drag_command = platform_endpoint
        .try_receive_command_for_contract_test()
        .expect("completed drag platform command");
    let nxtools_protocol::Message::PlatformCommand(PlatformCommand::BeginFileDrag(command)) =
        drag_command.message
    else {
        panic!("completed drag must cross the typed native file-drag boundary");
    };
    assert_eq!(
        command.path,
        directory.join("shared-skeleton.bin").display().to_string()
    );
    assert_eq!(command.source_window_handle, 42);
    assert!(
        cx.read(|cx| application.read(cx).task_windows.contains_key(&task_id)),
        "the native drag gesture must not close the completed window"
    );

    window.simulate_click(reveal_action.center(), Modifiers::default());
    window.run_until_parked();
    let reveal_envelope = platform_endpoint
        .try_receive_command_for_contract_test()
        .expect("completed reveal platform command");
    assert_eq!(
        reveal_envelope.message,
        nxtools_protocol::Message::PlatformCommand(PlatformCommand::OpenShellTarget(
            OpenShellTargetCommand {
                target: directory.join("shared-skeleton.bin").display().to_string(),
                action: ShellOpenAction::SelectInExplorer,
            }
        ))
    );
    assert!(cx.read(|cx| application.read(cx).task_windows.contains_key(&task_id)));
    window.simulate_click(reveal_action.center(), Modifiers::default());
    window.run_until_parked();
    assert!(
        platform_endpoint
            .try_receive_command_for_contract_test()
            .is_err(),
        "a repeated reveal while its correlated result is pending must not queue another Shell request"
    );
    cx.update(|cx| {
        application.update(cx, |application, cx| {
            application.handle_platform_notification(
                PlatformNotification::CommandResult {
                    correlation_id: reveal_envelope.id,
                    result: PlatformCommandResult {
                        success: false,
                        error: Some("Explorer activation failed".into()),
                        native_error_code: Some(5),
                        data: Some("explorer_window_not_found".into()),
                    },
                },
                cx,
            );
        });
    });
    assert!(
        cx.read(|cx| application.read(cx).task_windows.contains_key(&task_id)),
        "a failed Shell action must leave the completed window available for retry"
    );
    assert!(
        task_window_handle.update(cx, |_, _, _| ()).is_ok(),
        "real Explorer lookup failure keeps the presentation alive"
    );
    assert!(
        !cx.read(|cx| application
            .read(cx)
            .pending_artifact_actions
            .contains_key(&reveal_envelope.id)),
        "native failure retires the pending command so retry is possible"
    );

    window.simulate_click(reveal_action.center(), Modifiers::default());
    window.run_until_parked();
    let timed_out_reveal = platform_endpoint
        .try_receive_command_for_contract_test()
        .expect("reveal command that will time out");
    cx.update(|cx| {
        application.update(cx, |application, cx| {
            application.artifact_action_timed_out(timed_out_reveal.id, cx);
        });
    });
    assert!(
        cx.read(|cx| application.read(cx).task_windows.contains_key(&task_id)),
        "a timed-out Shell action must leave its completed window available for retry"
    );
    cx.update(|cx| {
        application.update(cx, |application, cx| {
            application.handle_platform_notification(
                PlatformNotification::CommandResult {
                    correlation_id: timed_out_reveal.id,
                    result: PlatformCommandResult {
                        success: true,
                        error: None,
                        native_error_code: None,
                        data: None,
                    },
                },
                cx,
            );
        });
    });
    assert!(
        cx.read(|cx| application.read(cx).task_windows.contains_key(&task_id)),
        "a late success after timeout must not close the completed window"
    );

    window.simulate_click(reveal_action.center(), Modifiers::default());
    window.run_until_parked();
    let disconnected_reveal = platform_endpoint
        .try_receive_command_for_contract_test()
        .expect("reveal command invalidated by PlatformHost disconnect");
    cx.update(|cx| {
        application.update(cx, |application, cx| {
            application.handle_platform_notification(
                PlatformNotification::Disconnected {
                    generation: 1,
                    pending_command_ids: vec![disconnected_reveal.id],
                },
                cx,
            );
        });
    });
    assert!(
        cx.read(|cx| application.read(cx).task_windows.contains_key(&task_id)),
        "a PlatformHost disconnect must leave the completed window available for retry"
    );
    platform_endpoint.set_connected_for_contract_test(true);

    window.simulate_click(reveal_action.center(), Modifiers::default());
    window.run_until_parked();
    let reveal_retry = platform_endpoint
        .try_receive_command_for_contract_test()
        .expect("reveal retry platform command");
    cx.update(|cx| {
        application.update(cx, |application, cx| {
            application.handle_platform_notification(
                PlatformNotification::CommandResult {
                    correlation_id: uuid::Uuid::new_v4(),
                    result: PlatformCommandResult {
                        success: true,
                        error: None,
                        native_error_code: None,
                        data: Some("foreground_confirmed".into()),
                    },
                },
                cx,
            );
        })
    });
    assert!(
        cx.read(|cx| application.read(cx).task_windows.contains_key(&task_id)),
        "unrelated native success cannot close a pending completion popup"
    );
    assert!(cx.read(|cx| {
        application
            .read(cx)
            .pending_artifact_actions
            .contains_key(&reveal_retry.id)
    }));
    cx.update(|cx| {
        application.update(cx, |application, cx| {
            application.handle_platform_notification(
                PlatformNotification::CommandResult {
                    correlation_id: reveal_retry.id,
                    result: PlatformCommandResult {
                        success: true,
                        error: None,
                        native_error_code: None,
                        data: Some("target_confirmed_foreground_not_confirmed".into()),
                    },
                },
                cx,
            );
        });
    });
    assert!(
        !cx.read(|cx| application.read(cx).task_windows.contains_key(&task_id)),
        "a confirmed reveal must close its originating completed window"
    );
    assert!(
        task_window_handle.update(cx, |_, _, _| ()).is_err(),
        "matching correlated native success removes the actual window, not only registration"
    );
    assert!(!cx.read(|cx| {
        application
            .read(cx)
            .pending_artifact_actions
            .contains_key(&reveal_retry.id)
    }));

    cx.update(|cx| {
        application.update(cx, |application, cx| {
            let directive = application
                .task_window_lifecycle
                .request_window(task_id, DownloadWindowPhase::Completed);
            assert_eq!(
                directive,
                DownloadWindowDirective::OpenAndFocus(DownloadWindowPhase::Completed),
                "after the completed window closes, a reopen request must open one fresh window"
            );
            application.apply_task_window_directive(task_id, directive, cx);
        });
    });
    cx.run_until_parked();
    let (reopened_handle, reopened_generation) = cx.read(|cx| {
        let application = application.read(cx);
        let entry = application
            .task_windows
            .get(&task_id)
            .expect("reopened completed task window");
        assert_ne!(
            entry.generation, generation,
            "the artifact-open close must release the keyed registration so a reopen creates a fresh window"
        );
        (entry.window, entry.generation)
    });
    assert_ne!(reopened_generation, generation);
    let mut window = VisualTestContext::from_window(reopened_handle, cx);
    window.refresh().expect("render reopened completed window");
    while platform_endpoint
        .try_receive_command_for_contract_test()
        .is_ok()
    {}
    // A successful result for a retired generation or the main list cannot close this replacement.
    for origin in [
        Some(TaskWindowIdentity {
            native_window_handle: 42,
            generation,
        }),
        None,
    ] {
        cx.update(|cx| {
            application.update(cx, |application, cx| {
                if let Some(identity) = origin {
                    application.open_task_artifact_from_task_window(
                        task_id,
                        ShellOpenAction::SelectInExplorer,
                        identity,
                        cx,
                    );
                } else {
                    application.open_task_artifact(task_id, ShellOpenAction::SelectInExplorer, cx);
                }
            })
        });
        let envelope = platform_endpoint
            .try_receive_command_for_contract_test()
            .expect("Shell command");
        cx.update(|cx| {
            application.update(cx, |application, cx| {
                application.handle_platform_notification(
                    PlatformNotification::CommandResult {
                        correlation_id: envelope.id,
                        result: PlatformCommandResult {
                            success: true,
                            error: None,
                            native_error_code: None,
                            data: Some("foreground_confirmed".into()),
                        },
                    },
                    cx,
                );
            })
        });
        assert!(
            reopened_handle.update(cx, |_, _, _| ()).is_ok(),
            "stale generation/main-list success must preserve replacement window"
        );
        assert_eq!(
            cx.read(|cx| application
                .read(cx)
                .task_windows
                .get(&task_id)
                .expect("replacement")
                .generation),
            reopened_generation
        );
    }
    let chooser_button = window
        .debug_bounds("download-complete-open-with-action")
        .expect("current icon-only chooser");
    window.simulate_click(chooser_button.center(), Modifiers::default());
    window.run_until_parked();
    let chooser = platform_endpoint
        .try_receive_command_for_contract_test()
        .expect("Open With command");
    assert!(
        matches!(chooser.message, nxtools_protocol::Message::PlatformCommand(PlatformCommand::OpenShellTarget(ref command)) if command.action == ShellOpenAction::OpenWith)
    );
    let notifications_before =
        cx.read(|cx| application.read(cx).notifications.read(cx).items().len());
    cx.update(|cx| {
        application.update(cx, |application, cx| {
            assert!(
                application
                    .pending_artifact_actions
                    .contains_key(&chooser.id),
                "pending identity installed before result"
            );
            application.handle_platform_notification(
                PlatformNotification::CommandResult {
                    correlation_id: chooser.id,
                    result: PlatformCommandResult {
                        success: false,
                        error: None,
                        native_error_code: None,
                        data: Some("cancelled".into()),
                    },
                },
                cx,
            );
            assert!(
                !application
                    .pending_artifact_actions
                    .contains_key(&chooser.id)
            );
            assert!(
                application.task_windows.contains_key(&task_id),
                "cancel retains originating task window"
            );
        })
    });
    assert_eq!(
        cx.read(|cx| application.read(cx).notifications.read(cx).items().len()),
        notifications_before,
        "cancel is not a failure toast"
    );
    cx.update(|cx| {
        application.update(cx, |application, cx| {
            application.handle_platform_notification(
                PlatformNotification::CommandResult {
                    correlation_id: chooser.id,
                    result: PlatformCommandResult {
                        success: true,
                        error: None,
                        native_error_code: None,
                        data: Some("open_with_completed".into()),
                    },
                },
                cx,
            );
        })
    });
    assert!(
        reopened_handle.update(cx, |_, _, _| ()).is_ok(),
        "late success after chooser cancellation cannot close its popup"
    );

    window.simulate_click(open_action.center(), Modifiers::default());
    window.run_until_parked();
    let open_envelope = platform_endpoint
        .try_receive_command_for_contract_test()
        .expect("completed open platform command");
    assert_eq!(
        open_envelope.message,
        nxtools_protocol::Message::PlatformCommand(PlatformCommand::OpenShellTarget(
            OpenShellTargetCommand {
                target: directory.join("shared-skeleton.bin").display().to_string(),
                action: ShellOpenAction::Open,
            }
        ))
    );
    assert!(cx.read(|cx| application.read(cx).task_windows.contains_key(&task_id)));
    cx.update(|cx| {
        application.update(cx, |application, cx| {
            application.handle_platform_notification(
                PlatformNotification::CommandResult {
                    correlation_id: open_envelope.id,
                    result: PlatformCommandResult {
                        success: true,
                        error: None,
                        native_error_code: None,
                        data: None,
                    },
                },
                cx,
            );
        });
    });
    assert!(
        !cx.read(|cx| application.read(cx).task_windows.contains_key(&task_id)),
        "a confirmed Open action must close its originating completed window"
    );

    let _ = task_window_handle.update(cx, |_, window, _| window.remove_window());
    let _ = std::fs::remove_dir_all(directory);
}

#[gpui_kit::test]
fn task_window_arms_and_cancels_the_real_shutdown_countdown(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (application, directory) =
        download_application_for_window_test(cx, "shutdown-countdown-window");
    let task_id = cx.read(|cx| {
        application
            .read(cx)
            .engine
            .enqueue(
                DownloadRequest {
                    source_url: "https://example.test/shutdown.bin".into(),
                    save_directory: directory.clone(),
                    save_directory_explicit: false,
                    file_name: Some("shutdown.bin".into()),
                    headers: Vec::new(),
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
            .expect("enqueue shutdown fixture")
    });
    cx.update(|cx| {
        application.update(cx, |application, cx| {
            application.refresh(cx);
            application.open_task_window(task_id, DownloadWindowPhase::Progress, None, cx);
        });
    });
    cx.run_until_parked();
    let task_window = cx
        .read(|cx| {
            application
                .read(cx)
                .task_windows
                .get(&task_id)
                .map(|entry| entry.window)
        })
        .expect("production task window");
    let mut task_window = VisualTestContext::from_window(task_window, cx);
    task_window
        .refresh()
        .expect("render production task window");
    let arm = task_window
        .debug_bounds("download-window-shutdown-arm-button")
        .expect("task-local shutdown arm");
    assert!(
        arm.size.height > px(0.0) && arm.size.height <= px(32.0),
        "official Small Checkbox inside original footer"
    );
    let footer = task_window
        .debug_bounds("download-task-window-footer")
        .expect("original footer");
    assert!(arm.top() >= footer.top() && arm.bottom() <= footer.bottom());
    task_window.simulate_click(arm.center(), Modifiers::default());
    task_window.run_until_parked();
    assert!(cx.read(|cx| application.read(cx).is_shutdown_armed(task_id)));
    task_window.update(|window, _| window.activate_window());
    task_window.run_until_parked();
    for _ in 0..16 {
        let focused = task_window.update(|window, _| {
            gpui_kit::base::test_support::snapshots(window)
                .into_iter()
                .any(|node| {
                    node.role() == Some(gpui_kit::Role::CheckBox)
                        && node.focused() == Some(true)
                        && node
                            .path()
                            .contains(&gpui_kit::ElementId::from("download-task-window-toolbar"))
                })
        });
        if focused {
            break;
        }
        task_window.update(|window, cx| window.focus_next(cx));
        task_window
            .refresh()
            .expect("actual caption-to-toolbar focus");
    }
    assert!(
        task_window.update(|window, _| gpui_kit::base::test_support::snapshots(window)
            .into_iter()
            .any(|node| node.role() == Some(gpui_kit::Role::CheckBox)
                && node.focused() == Some(true)
                && node
                    .path()
                    .contains(&gpui_kit::ElementId::from("download-task-window-toolbar")))),
        "shutdown Checkbox has actual focus before Space"
    );
    assert!(
        task_window
            .update(|window, cx| window.focused(cx))
            .is_some()
    );
    for expected in [false, true] {
        task_window.simulate_event(gpui_kit::KeyDownEvent {
            keystroke: gpui_kit::Keystroke::parse("space").expect("Checkbox Space"),
            is_held: false,
            prefer_character_input: false,
        });
        task_window.simulate_event(gpui_kit::KeyUpEvent {
            keystroke: gpui_kit::Keystroke::parse("space").expect("Checkbox release"),
        });
        task_window.run_until_parked();
        task_window.refresh().expect("controlled shutdown Checkbox");
        assert_eq!(
            cx.read(|cx| application.read(cx).is_shutdown_armed(task_id)),
            expected,
            "one real Checkbox release toggles task-local intent once"
        );
    }

    cx.update(|cx| {
        application.update(cx, |application, cx| {
            let transition = application
                .completion_actions
                .evaluate_shutdown([(task_id, DownloadStatus::Completed)], Instant::now());
            application.apply_shutdown_transition(transition, cx);
        });
    });
    cx.run_until_parked();
    let shutdown_window_handle = cx
        .read(|cx| {
            application
                .read(cx)
                .shutdown_window
                .as_ref()
                .map(|window| window.window)
        })
        .expect("production shutdown countdown window");
    let mut shutdown_window = VisualTestContext::from_window(shutdown_window_handle, cx);
    shutdown_window
        .refresh()
        .expect("render production shutdown countdown");
    assert_shared_dialog_shell(
        &mut shutdown_window,
        "download-shutdown-dialog-shell",
        "download-shutdown-dialog-titlebar",
        "download-shutdown-dialog-body",
        "download-shutdown-dialog-footer",
    );
    let cancel = shutdown_window
        .debug_bounds("download-shutdown-secondary-cancel-action")
        .expect("shutdown cancel secondary action");
    let shutdown_now = shutdown_window
        .debug_bounds("download-shutdown-danger-now-action")
        .expect("shutdown danger action");
    assert_eq!(cancel.size.height, px(24.0));
    assert_eq!(shutdown_now.size.height, px(24.0));
    assert!(shutdown_now.right() <= cancel.left());
    shutdown_window.simulate_click(cancel.center(), Modifiers::default());
    shutdown_window.run_until_parked();
    assert!(cx.read(|cx| {
        let application = application.read(cx);
        application.completion_actions.active_countdown().is_none()
            && application.shutdown_window.is_none()
            && application.pending_shutdown_command.is_none()
    }));
    assert!(shutdown_window_handle.update(cx, |_, _, _| ()).is_err());
    let _ = std::fs::remove_dir_all(directory);
}

#[gpui_kit::test]
fn a_native_task_window_close_releases_the_keyed_lifecycle_registration(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (application, directory) =
        download_application_for_window_test(cx, "native-close-lifecycle");
    let task_id = cx.read(|cx| {
        application
            .read(cx)
            .engine
            .enqueue(
                DownloadRequest {
                    source_url: "https://example.test/native-close.bin".into(),
                    save_directory: directory.clone(),
                    save_directory_explicit: false,
                    file_name: Some("native-close.bin".into()),
                    headers: Vec::new(),
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
            .expect("enqueue native-close fixture")
    });
    cx.update(|cx| {
        application.update(cx, |application, cx| {
            application.refresh(cx);
            application.open_task_window(task_id, DownloadWindowPhase::Progress, None, cx);
        });
    });
    cx.run_until_parked();
    let task_window = cx
        .read(|cx| {
            application
                .read(cx)
                .task_windows
                .get(&task_id)
                .map(|entry| entry.window)
        })
        .expect("production task window");
    assert!(
        task_window.update(cx, |_, _, _| ()).is_ok(),
        "the keyed task window is alive before the native close"
    );

    cx.update(|cx| {
        application.update(cx, |application, cx| {
            application.handle_task_window_native_close(task_id, task_window, cx);
        });
        // GPUI consumes the WM_CLOSE that triggered the handler and destroys
        // the platform window; mirror that in the test harness.
        let _ = task_window.update(cx, |_, window, _| window.remove_window());
    });
    cx.run_until_parked();
    assert!(
        !cx.read(|cx| application.read(cx).task_windows.contains_key(&task_id)),
        "the native close must release the keyed registration"
    );
    cx.update(|cx| {
        application.update(cx, |application, cx| {
            // The next progress event arrives after the user closed the window:
            // the lifecycle must open one fresh window instead of targeting the
            // destroyed one and replacing it (the visible flash).
            let directive = application
                .task_window_lifecycle
                .task_changed(task_id, DownloadStatus::Downloading);
            assert_eq!(
                directive,
                DownloadWindowDirective::None,
                "a task event after a native close must not resurrect the progress window"
            );
            application.apply_task_window_directive(task_id, directive, cx);
        });
    });
    assert!(
        task_window.update(cx, |_, _, _| ()).is_err(),
        "the natively closed window stays gone"
    );
    assert!(cx.read(|cx| application.read(cx).task_windows.is_empty()));

    let _ = std::fs::remove_dir_all(directory);
}

#[gpui_kit::test]
fn native_shutdown_window_close_cancels_the_countdown(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (application, directory) =
        download_application_for_window_test(cx, "shutdown-native-close");
    let task_id = DownloadTaskId::new();
    cx.update(|cx| {
        application.update(cx, |application, cx| {
            application
                .completion_actions
                .set_shutdown_armed(task_id, true);
            let transition = application
                .completion_actions
                .evaluate_shutdown([(task_id, DownloadStatus::Completed)], Instant::now());
            application.apply_shutdown_transition(transition, cx);
        });
    });
    cx.run_until_parked();
    let window_handle = cx
        .read(|cx| {
            application
                .read(cx)
                .shutdown_window
                .as_ref()
                .map(|window| window.window)
        })
        .expect("production shutdown countdown window");
    let mut window = VisualTestContext::from_window(window_handle, cx);
    assert!(
        window.simulate_close(),
        "native close is accepted after disarming"
    );
    window.run_until_parked();
    assert!(cx.read(|cx| {
        let application = application.read(cx);
        application.completion_actions.active_countdown().is_none()
            && application.shutdown_window.is_none()
            && application.pending_shutdown_command.is_none()
    }));
    let _ = std::fs::remove_dir_all(directory);
}

#[gpui_kit::test]
fn stale_shutdown_arm_click_cannot_arm_a_task_that_already_terminated(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (application, directory) = download_application_for_window_test(cx, "shutdown-stale-arm");
    let task_id = cx.read(|cx| {
        application
            .read(cx)
            .engine
            .enqueue(
                DownloadRequest {
                    source_url: "https://example.test/stale-arm.bin".into(),
                    save_directory: directory.clone(),
                    save_directory_explicit: false,
                    file_name: Some("stale-arm.bin".into()),
                    headers: Vec::new(),
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
            .expect("enqueue stale arm fixture")
    });
    cx.update(|cx| {
        application.update(cx, |application, cx| {
            application.refresh(cx);
            application.open_task_window(task_id, DownloadWindowPhase::Progress, None, cx);
        });
    });
    cx.run_until_parked();
    let task_window = cx
        .read(|cx| {
            application
                .read(cx)
                .task_windows
                .get(&task_id)
                .map(|entry| entry.window)
        })
        .expect("production task window");
    let mut task_window = VisualTestContext::from_window(task_window, cx);
    task_window
        .refresh()
        .expect("render task window before terminal");
    let arm = task_window
        .debug_bounds("download-window-shutdown-arm")
        .expect("stale shutdown arm hit target");

    cx.read(|cx| {
        application
            .read(cx)
            .engine
            .cancel(task_id)
            .expect("terminate task before deferred arm click");
    });
    task_window.simulate_click(arm.center(), Modifiers::default());
    task_window.run_until_parked();
    assert!(cx.read(|cx| {
        let application = application.read(cx);
        !application.is_shutdown_armed(task_id)
            && application.completion_actions.active_countdown().is_none()
            && application.shutdown_window.is_none()
    }));
    let _ = std::fs::remove_dir_all(directory);
}

#[gpui_kit::test]
fn shutdown_now_crosses_the_typed_platform_boundary_and_closes_on_success(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let directory = unique_download_settings_test_directory("shutdown-platform-boundary");
    std::fs::create_dir_all(&directory).expect("create shutdown boundary directory");
    let engine = DownloadEngine::open_in_memory().expect("open shutdown boundary engine");
    let (platform, platform_endpoint) = nxtools_ipc::platform_bridge(8);
    platform_endpoint.set_connected_for_contract_test(true);
    let notifications = cx.update(|cx| cx.new(|_| NotificationCenter::default()));
    let application = cx.update(|cx| {
        cx.new(|cx| {
            DownloadApplication::new(
                engine,
                Rc::new(platform),
                notifications,
                DownloadRuntimeContext {
                    fallback_default_directory: directory.clone(),
                    config: AppConfig::default(),
                    config_path: directory.join("config.json"),
                    log_directory: directory.clone(),
                    database_path: directory.join("downloads.sqlite3"),
                    ffmpeg_install_directory: directory.join("tools").join("ffmpeg"),
                },
                cx,
            )
        })
    });
    let task_id = DownloadTaskId::new();
    cx.update(|cx| {
        application.update(cx, |application, cx| {
            application
                .completion_actions
                .set_shutdown_armed(task_id, true);
            let transition = application
                .completion_actions
                .evaluate_shutdown([(task_id, DownloadStatus::Completed)], Instant::now());
            application.apply_shutdown_transition(transition, cx);
        });
    });
    cx.run_until_parked();
    let window_handle = cx
        .read(|cx| {
            application
                .read(cx)
                .shutdown_window
                .as_ref()
                .map(|window| window.window)
        })
        .expect("production shutdown countdown window");
    let mut window = VisualTestContext::from_window(window_handle, cx);
    window.refresh().expect("render shutdown countdown window");
    let shutdown_now = window
        .debug_bounds("download-shutdown-danger-now-action")
        .expect("immediate shutdown action");
    window.simulate_click(shutdown_now.center(), Modifiers::default());
    window.run_until_parked();

    let envelope = platform_endpoint
        .try_receive_command_for_contract_test()
        .expect("typed shutdown command");
    assert!(matches!(
        envelope.message,
        nxtools_protocol::Message::PlatformCommand(PlatformCommand::ShutdownSystem(
            ShutdownSystemCommand { delay_seconds: 1 }
        ))
    ));
    let operation_id = envelope
        .correlation_id
        .expect("shutdown operation correlation");
    assert_eq!(
        cx.read(|cx| {
            application
                .read(cx)
                .pending_shutdown_command
                .as_ref()
                .map(|pending| pending.countdown.correlation_id)
        }),
        Some(operation_id)
    );
    cx.update(|cx| {
        application.update(cx, |application, cx| {
            application.handle_platform_notification(
                PlatformNotification::CommandResult {
                    correlation_id: envelope.id,
                    result: PlatformCommandResult {
                        success: true,
                        error: None,
                        native_error_code: None,
                        data: None,
                    },
                },
                cx,
            );
        });
    });
    assert!(cx.read(|cx| {
        let application = application.read(cx);
        application.pending_shutdown_command.is_none()
            && application.shutdown_window.is_none()
            && application.completion_actions.active_countdown().is_none()
    }));
    assert!(window_handle.update(cx, |_, _, _| ()).is_err());
    let _ = std::fs::remove_dir_all(directory);
}

#[gpui_kit::test]
fn closing_download_settings_always_leaves_taskbar_overlay_edit_mode(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (application, directory) = download_application_for_window_test(cx, "settings-edit-close");
    let correlation_id = uuid::Uuid::new_v4();

    cx.update(|cx| {
        application.update(cx, |application, cx| {
            application.taskbar_overlay_edit_draft =
                Some(DownloadTaskbarProgressOverlaySettings::default());
            application.taskbar_overlay_edit_correlation_id = Some(correlation_id);
            application.handle_settings_window_closed(cx);
        });
    });

    assert!(cx.read(|cx| {
        let application = application.read(cx);
        application.settings_window.is_none()
            && application.taskbar_overlay_edit_draft.is_none()
            && application.taskbar_overlay_edit_correlation_id.is_none()
    }));
    let _ = std::fs::remove_dir_all(directory);
}

#[gpui_kit::test]
fn native_move_size_end_closes_only_the_matching_taskbar_edit_gesture(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (application, directory) =
        download_application_for_window_test(cx, "taskbar-edit-native-finish");
    cx.update(|cx| {
        application.update(cx, |application, cx| {
            application.taskbar_overlay_native_window_handle = 42;
            let failed_command_id = uuid::Uuid::new_v4();
            application.taskbar_overlay_edit_gesture_active = true;
            application.register_taskbar_overlay_edit_gesture(failed_command_id);
            application.handle_platform_notification(
                PlatformNotification::CommandResult {
                    correlation_id: failed_command_id,
                    result: PlatformCommandResult {
                        success: false,
                        error: Some("native move rejected".into()),
                        native_error_code: None,
                        data: None,
                    },
                },
                cx,
            );
            assert!(!application.taskbar_overlay_edit_gesture_active);

            application.taskbar_overlay_edit_gesture_active = true;
            application.handle_platform_notification(
                PlatformNotification::Event(PlatformEvent::TaskbarOverlayEditGestureFinished(
                    nxtools_protocol::TaskbarOverlayEditGesture { window_handle: 7 },
                )),
                cx,
            );
            assert!(application.taskbar_overlay_edit_gesture_active);
            application.handle_platform_notification(
                PlatformNotification::Event(PlatformEvent::TaskbarOverlayEditGestureFinished(
                    nxtools_protocol::TaskbarOverlayEditGesture { window_handle: 42 },
                )),
                cx,
            );
            assert!(!application.taskbar_overlay_edit_gesture_active);
        });
    });
    let _ = std::fs::remove_dir_all(directory);
}

#[gpui_kit::test]
fn ffmpeg_platform_timeout_releases_picker_and_page_operations(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (application, directory) =
        download_application_for_window_test(cx, "ffmpeg-platform-timeout");
    let (picker_id, page_id) = cx.update(|cx| {
        application.update(cx, |application, cx| {
            application.request_ffmpeg_executable(String::new(), cx);
            application.open_ffmpeg_download_page(FfmpegDownloadSource::Gyan, cx);
            let commands = application.ffmpeg_platform_commands.borrow();
            (commands[0].0, commands[1].0)
        })
    });

    cx.update(|cx| {
        application.update(cx, |application, cx| {
            application.ffmpeg_platform_operation_timed_out(picker_id, cx);
            application.ffmpeg_platform_operation_timed_out(page_id, cx);
            assert!(application.pending_ffmpeg_operations.is_empty());
            application.request_ffmpeg_executable(String::new(), cx);
            assert_eq!(application.ffmpeg_platform_commands.borrow().len(), 3);
        });
    });
    let _ = std::fs::remove_dir_all(directory);
}

#[gpui_kit::test]
fn settings_auto_download_commits_managed_ffmpeg_without_manual_save(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (application, directory) = download_application_for_window_test(cx, "ffmpeg-auto-install");
    let config_path = directory.join("config.json");
    AppConfig::default()
        .save_to(&config_path)
        .expect("save FFmpeg auto-install config fixture");
    let managed_directory = directory
        .join("tools")
        .join("ffmpeg")
        .join("managed-fixture");
    std::fs::create_dir_all(&managed_directory).expect("create managed FFmpeg fixture");
    let executable = managed_directory.join("ffmpeg.exe");
    std::fs::write(&executable, b"fixture").expect("write managed FFmpeg fixture");
    let (progress_sender, progress) = async_channel::bounded(4);
    let (completion_sender, completion) = async_channel::bounded(1);
    let correlation_id = uuid::Uuid::new_v4();
    cx.update(|cx| {
        application.update(cx, |application, _| {
            application.ffmpeg_install_operation_override = Some(FfmpegInstallOperationOverride {
                correlation_id,
                progress,
                completion,
                cancelled: Rc::new(Cell::new(false)),
            });
        });
    });
    let application_for_window = application.clone();
    let initial = cx.read(|cx| application.read(cx).settings_view_initial());
    let (settings, window) = cx.add_window_view(move |window, cx| {
        DownloadSettingsView::new(application_for_window, initial, window, cx)
    });
    window
        .refresh()
        .expect("render FFmpeg auto-install settings");
    window.run_until_parked();
    let transfer_navigation =
        official_download_navigation_bounds(window, DownloadSettingsPage::Transfer);
    window.simulate_click(transfer_navigation.center(), Modifiers::default());
    window.run_until_parked();
    window.refresh().expect("render FFmpeg install section");
    let install = window
        .debug_bounds("download-settings-ffmpeg-install")
        .expect("FFmpeg auto-download button");
    window.simulate_click(install.center(), Modifiers::default());
    window.run_until_parked();
    assert!(
        window
            .debug_bounds("download-settings-ffmpeg-cancel")
            .is_some()
    );
    progress_sender
        .try_send(FfmpegInstallProgress {
            phase: FfmpegInstallPhase::Downloading,
            percent: 48,
            downloaded_bytes: 48,
            total_bytes: Some(100),
        })
        .expect("send FFmpeg install progress");
    drop(progress_sender);
    completion_sender
        .try_send(Ok(FfmpegInstallation {
            executable: executable.clone(),
            source: FfmpegInstallSource::Gyan,
            downloaded_bytes: 100,
        }))
        .expect("complete FFmpeg install");
    drop(completion_sender);
    window.run_until_parked();

    let persisted = AppConfig::load_from(&config_path).expect("load automatic FFmpeg config");
    assert_eq!(
        persisted.download.ffmpeg_executable_path,
        executable.display().to_string()
    );
    assert_eq!(
        window.read(|cx| {
            settings
                .read(cx)
                .ffmpeg_executable
                .read(cx)
                .value()
                .to_string()
        }),
        executable.display().to_string()
    );
    cx.read(|cx| {
        assert_eq!(
            application
                .read(cx)
                .engine
                .ffmpeg_resolution()
                .expect("resolve managed FFmpeg")
                .expect("managed FFmpeg should be configured")
                .executable,
            executable
        );
    });
    let _ = std::fs::remove_dir_all(directory);
}

#[gpui_kit::test]
fn settings_media_tools_gives_the_ffmpeg_path_a_full_width_row(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (application, directory) =
        download_application_for_window_test(cx, "ffmpeg-settings-layout");
    let application_for_window = application.clone();
    let initial = cx.read(|cx| application.read(cx).settings_view_initial());
    let (_settings, window) = cx.add_window_view(move |window, cx| {
        window.resize(size(px(984.0), px(650.0)));
        DownloadSettingsView::new(application_for_window, initial, window, cx)
    });
    window.refresh().expect("render FFmpeg settings layout");
    window.simulate_resize(size(px(984.0), px(650.0)));
    let transfer_navigation =
        official_download_navigation_bounds(window, DownloadSettingsPage::Transfer);
    window.simulate_click(transfer_navigation.center(), Modifiers::default());
    window.run_until_parked();
    window.refresh().expect("render transfer settings layout");

    let _ = reveal_download_setting(window, "download-settings-ffmpeg-control");
    let section = official_download_group_bounds(window, DownloadSettingsPage::Transfer, 1);
    let control = window
        .debug_bounds("download-settings-ffmpeg-control")
        .expect("FFmpeg path control");
    let width_ratio = control.size.width.as_f32() / section.size.width.as_f32();
    assert!(
        width_ratio >= 0.9,
        "the FFmpeg path control is still cramped at {}px inside a {}px section ({width_ratio:.2})",
        control.size.width.as_f32(),
        section.size.width.as_f32()
    );

    let _ = std::fs::remove_dir_all(directory);
}

#[gpui_kit::test]
fn settings_sections_share_one_content_width_and_diagnostics_are_not_half_width(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_kit::init);
    let (application, directory, platform_endpoint) =
        download_application_for_window_test_with_platform(cx, "settings-section-widths");
    platform_endpoint.set_connected_for_contract_test(true);
    let diagnostic_root = directory
        .join("diagnostics-path-that-is-deliberately-long-for-the-production-settings-window")
        .join("runtime-owned-locations");
    let log_directory = diagnostic_root.join("logs");
    std::fs::create_dir_all(&log_directory).expect("create long diagnostics log directory");
    let config_path = diagnostic_root.join("nxtools-production-configuration.json");
    let database_path = diagnostic_root.join("downloads-production-database.sqlite3");
    AppConfig::default()
        .save_to(&config_path)
        .expect("save production settings layout config fixture");
    std::fs::write(&database_path, b"diagnostics database fixture")
        .expect("save production settings database fixture");
    cx.update(|cx| {
        application.update(cx, |application, cx| {
            application.config_path = config_path.clone();
            application.log_directory = log_directory.clone();
            application.database_path = database_path.clone();
            application.open_settings_window(cx);
        });
    });
    cx.run_until_parked();
    let settings_window = cx
        .read(|cx| application.read(cx).settings_window)
        .expect("production settings window");
    let mut window = VisualTestContext::from_window(settings_window, cx);
    window.run_until_parked();
    window.refresh().expect("render download settings widths");
    let settings_view = cx
        .read(|cx| application.read(cx).settings_view.clone())
        .expect("production settings view");

    let shell = window
        .debug_bounds("download-settings-shell")
        .expect("shared download settings shell");
    let shell_content = window
        .debug_bounds("download-settings-shell-content")
        .expect("shared download settings content");
    let shell_footer = window
        .debug_bounds("download-settings-shell-footer")
        .expect("shared download settings footer");
    let titlebar = window
        .debug_bounds("download-settings-titlebar")
        .expect("shared download settings titlebar");
    assert_eq!(shell.size.width, px(DOWNLOAD_SETTINGS_WINDOW_WIDTH));
    assert_eq!(shell.size.height, px(DOWNLOAD_SETTINGS_WINDOW_HEIGHT));
    assert_eq!(titlebar.size.height, px(34.));
    assert_eq!(shell_footer.size.height, px(48.0));
    assert_eq!(shell_content.bottom(), shell_footer.top());
    assert_eq!(shell_footer.bottom(), shell.bottom());

    let navigation =
        official_download_navigation_bounds(&mut window, DownloadSettingsPage::General);
    assert!(navigation.size.height >= px(28.0));
    let help = window
        .debug_bounds("download-settings-默认保存目录-help")
        .expect("full help Tooltip trigger");
    assert!(help.size.width <= px(24.0));

    for (name, selector) in [
        ("cancel", "download-settings-cancel"),
        ("apply", "download-settings-apply"),
    ] {
        let action = window
            .debug_bounds(selector)
            .unwrap_or_else(|| panic!("missing {name} settings action"));
        assert_eq!(
            action.size.height,
            px(24.0),
            "{name} must use the released Kit Small action tier"
        );
    }

    let save = official_download_group_bounds(&mut window, DownloadSettingsPage::General, 1);
    let expected_left = save.origin.x.as_f32();
    let expected_width = save.size.width.as_f32();
    assert!(
        expected_width >= 700.0,
        "official group padding retains a useful full content column"
    );
    macro_rules! open_page {
        ($selector:literal) => {{
            let page = match $selector {
                "download-settings-nav-general" => DownloadSettingsPage::General,
                "download-settings-nav-request" => DownloadSettingsPage::Request,
                "download-settings-nav-transfer" => DownloadSettingsPage::Transfer,
                "download-settings-nav-window" => DownloadSettingsPage::Window,
                _ => DownloadSettingsPage::Diagnostics,
            };
            select_official_download_page(&mut window, page);
            assert_eq!(
                window
                    .debug_bounds("download-settings-shell-footer")
                    .expect("footer"),
                shell_footer
            );
        }};
    }
    let active = window.update(|window, _| gpui_kit::base::test_support::snapshots(window));
    assert!(
        active
            .iter()
            .filter(|snapshot| snapshot.role() == Some(gpui_kit::Role::Group))
            .all(|snapshot| snapshot.label() != Some("传输")),
        "inactive content unmounts"
    );

    let initial_automatic_categories =
        cx.read(|cx| settings_view.read(cx).automatic_categories_enabled);
    let automatic_categories = window
        .debug_bounds("download-settings-automatic-categories")
        .expect("automatic categories draft control");
    window.simulate_click(automatic_categories.center(), Modifiers::default());
    window.run_until_parked();
    assert_eq!(
        cx.read(|cx| settings_view.read(cx).automatic_categories_enabled),
        !initial_automatic_categories,
        "the production settings draft must change before page navigation"
    );

    open_page!("download-settings-nav-request");
    let request_group =
        official_download_group_bounds(&mut window, DownloadSettingsPage::Request, 0);
    assert!((request_group.left().as_f32() - expected_left).abs() <= 1.0);
    assert!((request_group.size.width.as_f32() - expected_width).abs() <= 1.0);
    let request_form = window
        .debug_bounds("download-settings-request-context-form")
        .expect("request-context form");
    let header_field = window
        .debug_bounds("download-settings-default-headers-field")
        .expect("full-width Header editor");
    let header_label = window
        .debug_bounds("download-settings-default-headers-field-label")
        .expect("Header editor label");
    let cookie_field = window
        .debug_bounds("download-settings-default-cookie-field")
        .expect("full-width Cookie field");
    let cookie_label = window
        .debug_bounds("download-settings-default-cookie-field-label")
        .expect("Cookie field label");
    let certificate_section =
        official_download_group_bounds(&mut window, DownloadSettingsPage::Request, 1);
    let certificate_toggle = window
        .debug_bounds("download-settings-accept-invalid-certificates")
        .expect("certificate policy toggle");
    assert_eq!(header_field.size.height, px(120.0));
    assert_eq!(cookie_field.size.height, px(CONTROL_HEIGHT_INPUT));
    assert_eq!(header_field.origin.x, cookie_field.origin.x);
    assert_eq!(header_field.size.width, cookie_field.size.width);
    assert_eq!(header_label.origin.x, cookie_label.origin.x);
    for field in [header_field, cookie_field] {
        assert!(
            field.left() >= request_form.left()
                && field.right() <= request_form.right()
                && field.top() >= request_form.top()
                && field.bottom() <= request_form.bottom(),
            "form={request_form:?} field={field:?}"
        );
    }
    assert!(certificate_section.bottom() <= shell_footer.top());

    let long_header = (0..48)
        .map(|index| format!("X-NxTools-{index}: bounded-value-{index}"))
        .collect::<Vec<_>>()
        .join("\n");
    let (header_input, cookie_input) = cx.read(|cx| {
        let settings = settings_view.read(cx);
        (
            settings.default_request_headers.clone(),
            settings.default_cookie.clone(),
        )
    });
    window.update(|window, cx| {
        header_input.update(cx, |input, cx| {
            input.replace_all(long_header.clone(), window, cx);
        });
        cookie_input.update(cx, |input, cx| {
            input.replace_all("session=bounded", window, cx);
        });
    });
    window.run_until_parked();
    window.refresh().expect("render long request context");
    assert_eq!(
        window
            .debug_bounds("download-settings-default-headers-field")
            .expect("stable Header editor after long input"),
        header_field,
        "long Header content must scroll inside a fixed editor instead of growing the page"
    );
    assert_eq!(
        window
            .debug_bounds("download-settings-shell-footer")
            .expect("fixed footer after long Header input"),
        shell_footer,
        "long Header content must not move or resize the settings footer"
    );
    assert_eq!(
        official_download_group_bounds(&mut window, DownloadSettingsPage::Request, 1),
        certificate_section,
        "long Header never moves separate certificate group"
    );
    assert!(
        cx.read(|cx| {
            settings_view
                .read(cx)
                .default_request_headers
                .read(cx)
                .value()
                .to_string()
        }) == long_header,
        "the fixed editor must preserve the complete Header draft"
    );
    window.simulate_click(certificate_toggle.center(), Modifiers::default());
    window.run_until_parked();
    assert!(
        cx.read(|cx| settings_view.read(cx).accept_invalid_certificates),
        "the production certificate toggle must update the same request-settings draft"
    );
    open_page!("download-settings-nav-transfer");
    for selector in [
        "download-settings-connection-limit-field",
        "download-settings-concurrent-tasks-field",
        "download-settings-adaptive-segments",
        "download-settings-adaptive-max-field",
        "download-settings-adaptive-min-field",
        "download-settings-speed-limit-field",
    ] {
        let control = reveal_download_setting(&mut window, selector);
        assert!(
            control.left() >= px(expected_left)
                && control.right() <= px(expected_left + expected_width)
        );
    }
    let ffmpeg = reveal_download_setting(&mut window, "download-settings-ffmpeg-control");
    let media = official_download_group_bounds(&mut window, DownloadSettingsPage::Transfer, 1);
    assert!(ffmpeg.size.width >= media.size.width - px(32.0));
    let install = reveal_download_setting(&mut window, "download-settings-ffmpeg-install");
    let sources = reveal_download_setting(&mut window, "download-settings-ffmpeg-source-actions");
    assert_eq!(ffmpeg.left(), sources.left());
    assert!(install.left() >= media.left());
    window.update(|_, cx| {
        application.update(cx, |application, cx| {
            application.set_ffmpeg_install_state(
                FfmpegInstallViewState::Installing {
                    phase: FfmpegInstallPhase::Downloading,
                    percent: 48,
                    downloaded_bytes: 48,
                    total_bytes: Some(100),
                    cancelling: false,
                },
                cx,
            )
        })
    });
    window.run_until_parked();
    let cancel = reveal_download_setting(&mut window, "download-settings-ffmpeg-cancel");
    assert!(cancel.bottom() <= shell_footer.top());
    assert_eq!(
        window.debug_bounds("download-settings-shell-footer"),
        Some(shell_footer)
    );
    window.update(|_, cx| {
        application.update(cx, |application, cx| {
            application.set_ffmpeg_install_state(FfmpegInstallViewState::Idle, cx)
        })
    });
    window.run_until_parked();
    open_page!("download-settings-nav-window");
    for selector in [
        "download-settings-progress-window",
        "download-settings-completion-window",
        "download-settings-completion-sound",
        "download-settings-completion-directory",
        "download-settings-task-removal-delete-file",
        "download-settings-taskbar-overlay",
        "download-settings-taskbar-edit",
        "download-settings-taskbar-anchor",
        "download-settings-taskbar-progress-mode",
        "download-settings-taskbar-x-field",
        "download-settings-taskbar-y-field",
        "download-settings-taskbar-width-field",
        "download-settings-taskbar-height-field",
        "download-settings-taskbar-show-progress",
        "download-settings-taskbar-show-speed",
        "download-settings-taskbar-show-remaining",
        "download-settings-taskbar-show-bytes",
        "download-settings-taskbar-show-tasks",
        "download-settings-taskbar-show-connections",
    ] {
        let bounds = reveal_download_setting(&mut window, selector);
        assert!(bounds.right() <= px(expected_left + expected_width));
        assert!(bounds.bottom() <= shell_footer.top());
    }
    open_page!("download-settings-nav-diagnostics");
    let diagnostics_group =
        official_download_group_bounds(&mut window, DownloadSettingsPage::Diagnostics, 0);
    let expected_width = diagnostics_group.size.width.as_f32() - 32.0;
    for (label, row_selector, value_selector, actions_selector, copy_selector, open_selector) in [
        (
            "配置",
            "download-settings-path-配置",
            "download-settings-path-config-value",
            "download-settings-path-config-actions",
            "download-settings-path-config-copy",
            "download-settings-path-config-open",
        ),
        (
            "日志",
            "download-settings-path-日志",
            "download-settings-path-logs-value",
            "download-settings-path-logs-actions",
            "download-settings-path-logs-copy",
            "download-settings-path-logs-open",
        ),
        (
            "数据库",
            "download-settings-path-数据库",
            "download-settings-path-database-value",
            "download-settings-path-database-actions",
            "download-settings-path-database-copy",
            "download-settings-path-database-open",
        ),
    ] {
        let row = window
            .debug_bounds(row_selector)
            .unwrap_or_else(|| panic!("missing {label} diagnostic path"));
        assert!(
            row.size.width.as_f32() >= expected_width - 2.0,
            "{label} diagnostic row is only {}px inside a {expected_width}px section",
            row.size.width.as_f32()
        );
        let value = window
            .debug_bounds(value_selector)
            .unwrap_or_else(|| panic!("missing {label} diagnostic path value"));
        let actions = window
            .debug_bounds(actions_selector)
            .unwrap_or_else(|| panic!("missing {label} diagnostic action rail"));
        assert!(
            value.size.width.as_f32() >= expected_width * 0.7,
            "{label} long path is still constrained to a narrow column"
        );
        assert!(row.contains(&value.origin) && row.contains(&actions.bottom_right()));
        for (action_name, selector) in [("复制", copy_selector), ("打开", open_selector)] {
            let action = window
                .debug_bounds(selector)
                .unwrap_or_else(|| panic!("missing {label} {action_name} action"));
            assert_eq!(
                action.size,
                size(px(28.0), px(28.0)),
                "{label} {action_name} must use the shared square action tier"
            );
            assert!(
                action.left() >= actions.left()
                    && action.top() >= actions.top()
                    && action.right() <= actions.right()
                    && action.bottom() <= actions.bottom(),
                "rail={actions:?} action={action:?}"
            );
        }
    }

    let executor = cx.executor();
    let config_value = window
        .debug_bounds("download-settings-path-config-value")
        .expect("long config path value");
    window.simulate_mouse_move(config_value.center(), None, Modifiers::default());
    executor.advance_clock(Duration::from_millis(600));
    window.run_until_parked();
    window
        .refresh()
        .expect("render complete diagnostic path tooltip");
    assert!(
        window
            .debug_bounds("download-settings-path-config-value-tooltip")
            .is_some(),
        "the complete long path must remain readable from the production tooltip"
    );
    for (selector, tooltip_selector) in [
        (
            "download-settings-path-config-copy",
            "download-settings-path-config-copy-tooltip",
        ),
        (
            "download-settings-path-config-open",
            "download-settings-path-config-open-tooltip",
        ),
    ] {
        let action = window
            .debug_bounds(selector)
            .expect("diagnostic action with tooltip");
        window.simulate_mouse_move(action.center(), None, Modifiers::default());
        executor.advance_clock(Duration::from_millis(600));
        window.run_until_parked();
        window.refresh().expect("render diagnostic action tooltip");
        assert!(
            window.debug_bounds(tooltip_selector).is_some(),
            "diagnostic icon actions must expose bounded Chinese tooltips"
        );
    }

    for (selector, expected_path) in [
        (
            "download-settings-path-config-copy",
            config_path.to_string_lossy().into_owned(),
        ),
        (
            "download-settings-path-logs-copy",
            log_directory.to_string_lossy().into_owned(),
        ),
        (
            "download-settings-path-database-copy",
            database_path.to_string_lossy().into_owned(),
        ),
    ] {
        let action = window
            .debug_bounds(selector)
            .expect("diagnostic copy action");
        window.simulate_click(action.center(), Modifiers::default());
        window.run_until_parked();
        let envelope = platform_endpoint
            .try_receive_command_for_contract_test()
            .expect("copy-only platform command");
        assert!(
            envelope.correlation_id.is_some(),
            "diagnostic path actions must keep one operation correlation across PlatformHost"
        );
        assert_eq!(
            envelope.message,
            nxtools_protocol::Message::PlatformCommand(PlatformCommand::PasteText(
                nxtools_protocol::PasteTextCommand {
                    text: expected_path,
                    paste_after_write: false,
                    target_window_handle: 0,
                    target_focus_window_handle: 0,
                }
            ))
        );
        window.update(|_, cx| {
            application.update(cx, |application, cx| {
                application.handle_platform_notification(
                    PlatformNotification::CommandResult {
                        correlation_id: envelope.id,
                        result: PlatformCommandResult {
                            success: true,
                            error: None,
                            native_error_code: None,
                            data: None,
                        },
                    },
                    cx,
                );
            });
        });
    }
    let notifications = cx.read(|cx| application.read(cx).notifications.clone());
    assert!(cx.read(|cx| {
        notifications
            .read(cx)
            .items()
            .iter()
            .any(|item| item.title.as_ref() == "路径已复制")
    }));

    for (selector, target, action, success) in [
        (
            "download-settings-path-config-open",
            config_path.to_string_lossy().into_owned(),
            ShellOpenAction::SelectInExplorer,
            true,
        ),
        (
            "download-settings-path-logs-open",
            log_directory.to_string_lossy().into_owned(),
            ShellOpenAction::Open,
            true,
        ),
        (
            "download-settings-path-database-open",
            database_path.to_string_lossy().into_owned(),
            ShellOpenAction::SelectInExplorer,
            false,
        ),
    ] {
        let action_bounds = window
            .debug_bounds(selector)
            .expect("diagnostic open action");
        window.simulate_click(action_bounds.center(), Modifiers::default());
        window.run_until_parked();
        let envelope = platform_endpoint
            .try_receive_command_for_contract_test()
            .expect("shell path platform command");
        assert!(
            envelope.correlation_id.is_some(),
            "diagnostic path actions must keep one operation correlation across PlatformHost"
        );
        assert_eq!(
            envelope.message,
            nxtools_protocol::Message::PlatformCommand(PlatformCommand::OpenShellTarget(
                OpenShellTargetCommand { target, action }
            ))
        );
        window.update(|_, cx| {
            application.update(cx, |application, cx| {
                application.handle_platform_notification(
                    PlatformNotification::CommandResult {
                        correlation_id: envelope.id,
                        result: PlatformCommandResult {
                            success,
                            error: (!success).then(|| "sensitive path must not be logged".into()),
                            native_error_code: (!success).then_some(2),
                            data: None,
                        },
                    },
                    cx,
                );
            });
        });
    }
    assert!(cx.read(|cx| {
        notifications
            .read(cx)
            .items()
            .iter()
            .any(|item| item.title.as_ref() == "无法打开诊断路径")
    }));

    let timeout_action = window
        .debug_bounds("download-settings-path-config-copy")
        .expect("diagnostic copy timeout action");
    window.simulate_click(timeout_action.center(), Modifiers::default());
    window.run_until_parked();
    let timed_out_envelope = platform_endpoint
        .try_receive_command_for_contract_test()
        .expect("copy command that will time out");
    executor.advance_clock(DOWNLOAD_SETTINGS_PATH_ACTION_TIMEOUT);
    window.run_until_parked();
    assert!(cx.read(|cx| {
        notifications
            .read(cx)
            .items()
            .iter()
            .any(|item| item.title.as_ref() == "无法复制诊断路径")
    }));
    let before_late_reply = cx.read(|cx| {
        notifications
            .read(cx)
            .items()
            .iter()
            .map(|item| (item.id.clone(), item.generation()))
            .collect::<Vec<_>>()
    });
    window.update(|_, cx| {
        application.update(cx, |application, cx| {
            application.handle_platform_notification(
                PlatformNotification::CommandResult {
                    correlation_id: timed_out_envelope.id,
                    result: PlatformCommandResult {
                        success: true,
                        error: None,
                        native_error_code: None,
                        data: None,
                    },
                },
                cx,
            );
        });
    });
    assert_eq!(
        cx.read(|cx| notifications
            .read(cx)
            .items()
            .iter()
            .map(|item| (item.id.clone(), item.generation()))
            .collect::<Vec<_>>()),
        before_late_reply,
        "late reply cannot add or replace a notice; earlier successful independent copies remain"
    );

    open_page!("download-settings-nav-general");
    assert_eq!(
        cx.read(|cx| settings_view.read(cx).automatic_categories_enabled),
        !initial_automatic_categories,
        "switching through all five production pages must preserve the local draft"
    );

    let apply = window
        .debug_bounds("download-settings-apply")
        .expect("production settings Apply action");
    window.simulate_click(apply.center(), Modifiers::default());
    window.run_until_parked();
    let persisted =
        AppConfig::load_from(&config_path).expect("reload request-context production settings");
    assert!(
        persisted.download.default_request_headers == long_header,
        "Apply must preserve the complete Header draft without printing its value"
    );
    assert!(
        persisted.download.default_cookie == "session=bounded",
        "Apply must preserve the Cookie draft without printing its value"
    );
    assert!(persisted.download.accept_invalid_certificates);

    let _ = std::fs::remove_dir_all(directory);
}

#[gpui_kit::test]
fn category_settings_use_a_readable_list_and_a_labeled_full_width_directory_editor(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_kit::init);
    let executor = cx.executor();
    let (application, directory) =
        download_application_for_window_test(cx, "category-settings-layout");
    let config_path = directory.join("config.json");
    AppConfig::default()
        .save_to(&config_path)
        .expect("save category action persistence fixture");
    let persisted_before_actions =
        AppConfig::load_from(&config_path).expect("load normalized category persistence fixture");
    cx.update(|cx| {
        application.update(cx, |application, cx| {
            application.open_settings_window(cx);
        });
    });
    cx.run_until_parked();
    let settings_window = cx
        .read(|cx| application.read(cx).settings_window)
        .expect("production settings window");
    let mut window = VisualTestContext::from_window(settings_window, cx);
    window.run_until_parked();
    window.refresh().expect("render category settings");
    let settings_view = cx
        .read(|cx| application.read(cx).settings_view.clone())
        .expect("production settings view");
    let initial_category_ids = cx.read(|cx| {
        settings_view
            .read(cx)
            .categories
            .iter()
            .map(|category| category.id.clone())
            .collect::<Vec<_>>()
    });

    let category_item = window
        .debug_bounds("download-settings-category-0")
        .expect("first category item");
    assert_eq!(
        category_item.size.height,
        px(crate::ui::metrics::SETTINGS_ROW_MIN_HEIGHT),
        "category master rows must use the shared compact-list height"
    );
    let category_master = window
        .debug_bounds("download-settings-category-master")
        .expect("category master list");
    let category_nodes = window.update(|window, _| gpui_kit::base::test_support::snapshots(window));
    let selected_row = category_nodes
        .iter()
        .find(|node| {
            node.path().last() == Some(&gpui_kit::ElementId::from("download-settings-category-0"))
        })
        .expect("real selected ListItem");
    assert_eq!(selected_row.selected(), Some(true));
    assert!(
        category_item.left() >= category_master.left()
            && category_item.right() <= category_master.right()
            && category_item.top() >= category_master.top()
            && category_item.bottom() <= category_master.bottom()
    );

    let name_label = window
        .debug_bounds("download-settings-category-name-field-label")
        .expect("category name label");
    let directory_label = window
        .debug_bounds("download-settings-category-directory-field-label")
        .expect("category directory label");
    let extensions_label = window
        .debug_bounds("download-settings-category-extensions-field-label")
        .expect("category extensions label");
    let url_label = window
        .debug_bounds("download-settings-category-url-field-label")
        .expect("category URL label");
    assert_eq!(name_label.origin.y, directory_label.origin.y);
    assert_eq!(extensions_label.origin.y, url_label.origin.y);

    let name_cell = window
        .debug_bounds("download-settings-category-name-field")
        .expect("Form name control");
    let directory_cell = window
        .debug_bounds("download-settings-category-directory-field")
        .expect("Form directory control");
    let extensions_cell = window
        .debug_bounds("download-settings-category-extensions-field")
        .expect("Form extensions control");
    let url_cell = window
        .debug_bounds("download-settings-category-url-field")
        .expect("Form URL control");
    assert_eq!(name_cell.origin.y, directory_cell.origin.y);
    assert_eq!(extensions_cell.origin.y, url_cell.origin.y);
    assert!((name_cell.size.width.as_f32() - directory_cell.size.width.as_f32()).abs() <= 1.0);
    assert!((extensions_cell.size.width.as_f32() - url_cell.size.width.as_f32()).abs() <= 1.0);

    let name_field = window
        .debug_bounds("download-settings-category-name-field")
        .expect("labeled category name field");
    let directory_field = window
        .debug_bounds("download-settings-category-directory-field")
        .expect("category directory field");
    let extensions_field = window
        .debug_bounds("download-settings-category-extensions-field")
        .expect("category extensions field");
    let url_field = window
        .debug_bounds("download-settings-category-url-field")
        .expect("category URL field");
    for field in [name_field, directory_field, extensions_field, url_field] {
        assert_eq!(
            field.size.height,
            px(CONTROL_HEIGHT_INPUT),
            "all category editor fields must use the shared 32px input tier"
        );
    }
    let form = window
        .debug_bounds("download-settings-category-form")
        .expect("actual bounded Form");
    for selector in [
        "download-settings-category-directory-field-help",
        "download-settings-category-extensions-field-help",
        "download-settings-category-url-field-help",
    ] {
        let help = window
            .debug_bounds(selector)
            .expect("Form field Tooltip help");
        assert!(form.contains(&help.origin) && form.contains(&help.bottom_right()));
    }

    let initial_presentation_revision =
        cx.read(|cx| settings_view.read(cx).category_presentation_revision);
    let presentation_root = r"C:\Users\Administrator\Downloads\NxTools-Categories";
    let long_relative_directory = r"Compressed\A-Long-Readable-Category-Destination-For-Tooltip";
    let (directory_input, category_name_input, category_directory_input) = cx.read(|cx| {
        let settings = settings_view.read(cx);
        let category = settings.categories.first().expect("first category editor");
        (
            settings.directory.clone(),
            category.name.clone(),
            category.save_directory.clone(),
        )
    });
    window.update(|window, cx| {
        directory_input.update(cx, |input, cx| {
            input.replace_all(presentation_root, window, cx);
        });
        category_name_input.update(cx, |input, cx| {
            input.replace_all("压缩与归档文件", window, cx);
        });
        category_directory_input.update(cx, |input, cx| {
            input.replace_all(long_relative_directory, window, cx);
        });
    });
    window.run_until_parked();
    window
        .refresh()
        .expect("render edited category presentation");
    cx.read(|cx| {
            let settings = settings_view.read(cx);
            assert!(
                settings.category_presentation_revision >= initial_presentation_revision + 3,
                "default directory, category name and category directory changes must each invalidate the master presentation"
            );
            let presentation = settings.categories[0].presentation(presentation_root, cx);
            assert_eq!(presentation.name, "压缩与归档文件");
            assert_eq!(
                presentation.directory,
                PathBuf::from(presentation_root)
                    .join(long_relative_directory)
                    .display()
                    .to_string(),
                "relative category destinations must resolve against the current default directory"
            );
        });

    let edited_category_item = window
        .debug_bounds("download-settings-category-0")
        .expect("edited category master row");
    window.simulate_mouse_move(
        gpui_kit::point(px(1.0), px(1.0)),
        None,
        Modifiers::default(),
    );
    window.refresh().expect("outside hover frame");
    window.simulate_mouse_move(edited_category_item.center(), None, Modifiers::default());
    window.refresh().expect("inside hover frame");
    window.run_until_parked();
    executor.advance_clock(Duration::from_millis(700));
    window.run_until_parked();
    window.refresh().expect("render category directory tooltip");
    assert!(
        window
            .debug_bounds("download-settings-category-0-tooltip")
            .is_some(),
        "the production master row must expose the complete resolved destination through a stable tooltip"
    );

    let category_heading = window
        .debug_bounds("download-settings-category-heading")
        .expect("selected-category heading");
    let category_heading_title = window
        .debug_bounds("download-settings-category-heading-title")
        .expect("selected-category heading title");
    let enabled_toggle = window
        .debug_bounds("download-settings-category-enabled-button")
        .expect("category enabled toggle");
    assert!(category_heading.contains(&category_heading_title.origin));
    assert!(category_heading.contains(&category_heading_title.bottom_right()));
    assert!(category_heading.contains(&enabled_toggle.origin));
    assert!(
        enabled_toggle.right() <= category_heading.right()
            && enabled_toggle.bottom() <= category_heading.bottom()
    );

    let action_toolbar = window
        .debug_bounds("download-settings-category-action-toolbar")
        .expect("category action toolbar");
    let order_group = window
        .debug_bounds("download-settings-category-order-group")
        .expect("category order group");
    // The action buttons are gpui-component Buttons whose element ids are
    // not exposed as debug selectors; their wrapper controls own the
    // stable debug selectors and wrap each button exactly.
    let move_up_button = window
        .debug_bounds("download-settings-category-up")
        .expect("category move-up button");
    let move_down_button = window
        .debug_bounds("download-settings-category-down")
        .expect("category move-down button");
    let delete_button = window
        .debug_bounds("download-settings-category-delete")
        .expect("category delete button");
    for action in [move_up_button, move_down_button, delete_button] {
        assert_eq!(
            action.size.height,
            px(24.0),
            "category tools must use the released Kit Small action tier"
        );
        assert!(action_toolbar.contains(&action.origin));
        assert!(
            action.right() <= action_toolbar.right() && action.bottom() <= action_toolbar.bottom()
        );
    }
    assert_eq!(move_up_button.origin.y, move_down_button.origin.y);
    assert!(order_group.contains(&move_up_button.origin));
    assert!(
        move_up_button.right() <= order_group.right()
            && move_up_button.bottom() <= order_group.bottom()
    );
    assert!(order_group.contains(&move_down_button.origin));
    assert!(
        move_down_button.right() <= order_group.right()
            && move_down_button.bottom() <= order_group.bottom()
    );
    assert!(
        delete_button.origin.x >= order_group.bottom_right().x + px(SPACING_12),
        "destructive category action must be spatially separated from ordering tools"
    );

    let action_invocations_before_disabled_click =
        cx.read(|cx| settings_view.read(cx).category_action_invocations);
    window.simulate_click(move_up_button.center(), Modifiers::default());
    window.run_until_parked();
    assert_eq!(
        cx.read(|cx| settings_view.read(cx).category_action_invocations),
        action_invocations_before_disabled_click,
        "the edge-disabled move-up button must suppress its callback, not merely rely on a callback guard"
    );
    assert_eq!(
        cx.read(|cx| {
            settings_view
                .read(cx)
                .categories
                .iter()
                .map(|category| category.id.clone())
                .collect::<Vec<_>>()
        }),
        initial_category_ids,
        "the first category's disabled move-up action must not mutate draft order"
    );

    window.simulate_click(enabled_toggle.center(), Modifiers::default());
    window.run_until_parked();
    window.refresh().expect("render disabled category row");
    assert!(!cx.read(|cx| settings_view.read(cx).categories[0].enabled));
    let disabled_row = window
        .debug_bounds("download-settings-category-0")
        .expect("disabled category row remains rendered");
    let disabled_title = window
        .debug_bounds("download-settings-category-0-title")
        .expect("disabled category title remains readable");
    let disabled_detail = window
        .debug_bounds("download-settings-category-0-detail")
        .expect("disabled category directory remains readable");
    for content in [disabled_title, disabled_detail] {
        assert!(
            content.left() >= disabled_row.left()
                && content.right() <= disabled_row.right()
                && content.top() >= disabled_row.top()
                && content.bottom() <= disabled_row.bottom(),
            "disabled label must fit real ListItem: row={disabled_row:?} content={content:?}"
        );
    }

    let second_category = window
        .debug_bounds("download-settings-category-1")
        .expect("second category item");
    window.simulate_click(second_category.center(), Modifiers::default());
    window.run_until_parked();
    window.refresh().expect("render second category editor");
    // Selection moved to the clicked category (asserted through the state
    // below). Note: gpui 0.2.2 double-buffers debug_bounds across frames
    // (mem::swap), so absence assertions on removed elements are
    // unreliable; the selected edge is asserted on the clicked row only.
    assert_eq!(
        cx.read(|cx| settings_view.read(cx).selected_category),
        Some(1)
    );
    assert_eq!(
        window
            .update(|window, _| gpui_kit::base::test_support::snapshots(window))
            .iter()
            .find(|node| node.path().last()
                == Some(&gpui_kit::ElementId::from("download-settings-category-1")))
            .expect("clicked real ListItem")
            .selected(),
        Some(true),
        "clicked master row owns selected semantics"
    );
    assert!(
        window
            .debug_bounds("download-settings-category-directory-field")
            .is_some(),
        "switching categories must keep the selected category's directory editor visible"
    );

    let move_down = reveal_download_setting(&mut window, "download-settings-category-down-control");
    window.simulate_click(move_down.center(), Modifiers::default());
    window.run_until_parked();
    window.refresh().expect("render category after moving down");
    assert_eq!(
        cx.read(|cx| {
            settings_view
                .read(cx)
                .categories
                .iter()
                .map(|category| category.id.clone())
                .collect::<Vec<_>>()
        }),
        [
            initial_category_ids[0].clone(),
            initial_category_ids[2].clone(),
            initial_category_ids[1].clone(),
        ]
        .into_iter()
        .chain(initial_category_ids.iter().skip(3).cloned())
        .collect::<Vec<_>>(),
        "clicking move-down must update the production settings draft order"
    );

    let move_up = window
        .debug_bounds("download-settings-category-up-control")
        .expect("category move-up action");
    window.simulate_click(move_up.center(), Modifiers::default());
    window.run_until_parked();
    window.refresh().expect("render category after moving up");
    assert_eq!(
        cx.read(|cx| {
            settings_view
                .read(cx)
                .categories
                .iter()
                .map(|category| category.id.clone())
                .collect::<Vec<_>>()
        }),
        initial_category_ids,
        "clicking move-up must restore the production settings draft order"
    );

    let deleted_category_id = initial_category_ids[1].clone();
    let delete = window
        .debug_bounds("download-settings-category-delete-control")
        .expect("category delete action");
    let delete_button_before_hover = window
        .debug_bounds("download-settings-category-delete")
        .expect("category delete button before hover");
    window.simulate_mouse_move(
        delete_button_before_hover.center(),
        None,
        Modifiers::default(),
    );
    window.run_until_parked();
    window.refresh().expect("render category delete hover");
    assert_eq!(
        window
            .debug_bounds("download-settings-category-delete")
            .expect("category delete button after hover"),
        delete_button_before_hover,
        "danger hover must not move or resize the category delete action"
    );
    window.simulate_click(delete.center(), Modifiers::default());
    window.run_until_parked();
    window.refresh().expect("render category after delete");
    cx.read(|cx| {
        let settings = settings_view.read(cx);
        assert_eq!(settings.categories.len(), initial_category_ids.len() - 1);
        assert!(
            settings
                .categories
                .iter()
                .all(|category| category.id != deleted_category_id),
            "clicking delete must remove the selected category from the production draft"
        );
    });
    assert_eq!(
        AppConfig::load_from(&config_path)
            .expect("reload category settings before the Apply boundary"),
        persisted_before_actions,
        "category state, order and deletion actions must remain local draft changes until Apply"
    );
    let expected_categories_after_apply = cx.read(|cx| {
        settings_view
            .read(cx)
            .categories
            .iter()
            .map(|category| category.settings(cx))
            .collect::<Vec<_>>()
    });

    for (name, selector) in [
        ("directory picker", "download-settings-pick-directory"),
        ("category add", "download-settings-category-add-button"),
        ("category move up", "download-settings-category-up"),
        ("category move down", "download-settings-category-down"),
        ("category delete", "download-settings-category-delete"),
        ("settings cancel", "download-settings-cancel-button"),
        ("settings apply", "download-settings-apply-button"),
    ] {
        let bounds = window
            .debug_bounds(selector)
            .unwrap_or_else(|| panic!("missing {name} control"));
        assert_eq!(
            bounds.size.height,
            px(24.0),
            "{name} must use the shared action tier"
        );
    }
    assert_eq!(
        window
            .debug_bounds("download-settings-category-enabled-button")
            .expect("category enabled toggle")
            .size
            .height,
        px(20.0),
        "settings toggles use the same official Medium tier"
    );

    select_official_download_page(&mut window, DownloadSettingsPage::Request);
    select_official_download_page(&mut window, DownloadSettingsPage::General);
    let directory_picker = reveal_download_setting(&mut window, "download-settings-pick-directory");
    window.simulate_mouse_move(directory_picker.center(), None, Modifiers::default());
    executor.advance_clock(Duration::from_millis(600));
    window.run_until_parked();
    window.refresh().expect("render directory picker tooltip");
    assert!(
        window
            .debug_bounds("download-settings-pick-directory-tooltip")
            .is_some(),
        "the production directory picker exposes its Chinese tooltip"
    );

    let transfer_navigation =
        official_download_navigation_bounds(&mut window, DownloadSettingsPage::Transfer);
    window.simulate_click(transfer_navigation.center(), Modifiers::default());
    window.run_until_parked();
    window.refresh().expect("render transfer action sizes");
    for (name, selector) in [
        ("FFmpeg picker", "download-settings-pick-ffmpeg-button"),
        (
            "FFmpeg auto-download",
            "download-settings-ffmpeg-install-button",
        ),
        ("Gyan source", "download-settings-ffmpeg-gyan-button"),
        ("BtbN source", "download-settings-ffmpeg-btbn-button"),
    ] {
        let action = window
            .debug_bounds(selector)
            .unwrap_or_else(|| panic!("missing {name} action"));
        assert_eq!(
            action.size.height,
            px(24.0),
            "{name} must use the released Kit Small action tier"
        );
    }
    let ffmpeg_picker =
        reveal_download_setting(&mut window, "download-settings-pick-ffmpeg-button");
    window.simulate_mouse_move(ffmpeg_picker.center(), None, Modifiers::default());
    executor.advance_clock(Duration::from_millis(600));
    window.run_until_parked();
    window.refresh().expect("render FFmpeg picker tooltip");
    assert!(
        window
            .debug_bounds("download-settings-pick-ffmpeg-tooltip")
            .is_some(),
        "the production FFmpeg picker exposes its Chinese tooltip"
    );

    let apply = window
        .debug_bounds("download-settings-apply")
        .expect("settings Apply action after category draft edits");
    window.simulate_click(apply.center(), Modifiers::default());
    window.run_until_parked();
    assert_eq!(
        AppConfig::load_from(&config_path)
            .expect("reload category settings after the Apply boundary")
            .download
            .categories,
        expected_categories_after_apply,
        "Apply must persist the exact enabled/order/deletion state from the category draft"
    );

    let _ = std::fs::remove_dir_all(directory);
}

#[gpui_kit::test]
fn settings_auto_download_cancel_click_reaches_the_operation_control(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (application, directory) =
        download_application_for_window_test(cx, "ffmpeg-auto-install-cancel");
    AppConfig::default()
        .save_to(&directory.join("config.json"))
        .expect("save FFmpeg cancel config fixture");
    let (progress_sender, progress) = async_channel::bounded(1);
    let (completion_sender, completion) = async_channel::bounded(1);
    let cancelled = Rc::new(Cell::new(false));
    let cancellation_assertion = Rc::clone(&cancelled);
    cx.update(|cx| {
        application.update(cx, |application, _| {
            application.ffmpeg_install_operation_override = Some(FfmpegInstallOperationOverride {
                correlation_id: uuid::Uuid::new_v4(),
                progress,
                completion,
                cancelled,
            });
        });
    });
    let application_for_window = application.clone();
    let initial = cx.read(|cx| application.read(cx).settings_view_initial());
    let (_settings, window) = cx.add_window_view(move |window, cx| {
        DownloadSettingsView::new(application_for_window, initial, window, cx)
    });
    window.refresh().expect("render FFmpeg cancel settings");
    window.run_until_parked();
    let transfer_navigation =
        official_download_navigation_bounds(window, DownloadSettingsPage::Transfer);
    window.simulate_click(transfer_navigation.center(), Modifiers::default());
    window.run_until_parked();
    window.refresh().expect("render FFmpeg cancel section");
    let install = window
        .debug_bounds("download-settings-ffmpeg-install")
        .expect("FFmpeg auto-download button");
    window.simulate_click(install.center(), Modifiers::default());
    window.run_until_parked();
    let cancel = window
        .debug_bounds("download-settings-ffmpeg-cancel")
        .expect("FFmpeg auto-download cancel button");
    window.simulate_click(cancel.center(), Modifiers::default());
    window.run_until_parked();

    assert!(cancellation_assertion.get());
    cx.read(|cx| {
        assert!(matches!(
            application.read(cx).ffmpeg_install_state,
            FfmpegInstallViewState::Installing {
                cancelling: true,
                ..
            }
        ));
    });
    drop(progress_sender);
    drop(completion_sender);
    let _ = std::fs::remove_dir_all(directory);
}

#[gpui_kit::test]
fn settings_window_routes_picker_applies_and_cancels_without_cross_module_loss(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_kit::init);
    let directory = unique_download_settings_test_directory("window");
    let selected_directory = directory.join("selected");
    std::fs::create_dir_all(&selected_directory).expect("create selected directory");
    let ffmpeg_executable = directory.join("ffmpeg-user-selected.exe");
    std::fs::write(&ffmpeg_executable, b"fixture").expect("write FFmpeg picker fixture");
    let config_path = directory.join("config.json");
    let mut config = AppConfig::default();
    config.quick_launch.enabled = false;
    config.save_to(&config_path).expect("save settings fixture");
    let engine = DownloadEngine::open_in_memory().expect("open settings engine");
    let engine_for_assertion = engine.clone();
    let (platform, _platform_endpoint) = nxtools_ipc::platform_bridge(16);
    _platform_endpoint.set_connected_for_contract_test(true);
    let notifications = cx.update(|cx| cx.new(|_| NotificationCenter::default()));
    let application = cx.update(|cx| {
        cx.new(|cx| {
            DownloadApplication::new(
                engine,
                Rc::new(platform),
                notifications,
                DownloadRuntimeContext {
                    fallback_default_directory: directory.clone(),
                    config,
                    config_path: config_path.clone(),
                    log_directory: directory.clone(),
                    database_path: directory.join("downloads.sqlite3"),
                    ffmpeg_install_directory: directory.join("tools").join("ffmpeg"),
                },
                cx,
            )
        })
    });
    let application_for_window = application.clone();
    let settings_initial = cx.read(|cx| application.read(cx).settings_view_initial());
    let (settings, window) = cx.add_window_view(move |window, cx| {
        DownloadSettingsView::new(application_for_window, settings_initial, window, cx)
    });
    window.refresh().expect("render download settings window");
    window.run_until_parked();
    assert!(
        window
            .update(|window, _| gpui_kit::base::test_support::snapshots(window))
            .iter()
            .any(|snapshot| snapshot
                .path()
                .iter()
                .any(|id| id == &gpui_kit::ElementId::from("settings-sidebar")))
    );
    window.update(|window, cx| {
        settings.update(cx, |settings, cx| {
            settings.default_request_headers.update(cx, |input, cx| {
                input.replace_all(
                    "X-NxTools-Default: desktop\nReferer: https://example.test/",
                    window,
                    cx,
                );
            });
            settings.default_cookie.update(cx, |input, cx| {
                input.replace_all("session=desktop", window, cx);
            });
        });
    });
    let request_navigation =
        official_download_navigation_bounds(window, DownloadSettingsPage::Request);
    window.simulate_click(request_navigation.center(), Modifiers::default());
    window.run_until_parked();
    let certificate_toggle = window
        .debug_bounds("download-settings-accept-invalid-certificates")
        .expect("certificate policy toggle");
    window.simulate_click(certificate_toggle.center(), Modifiers::default());
    let general_navigation =
        official_download_navigation_bounds(window, DownloadSettingsPage::General);
    window.simulate_click(general_navigation.center(), Modifiers::default());
    window.run_until_parked();
    let automatic_categories = window
        .debug_bounds("download-settings-automatic-categories")
        .expect("automatic category toggle");
    window.simulate_click(automatic_categories.center(), Modifiers::default());
    let reject_duplicates = window
        .debug_bounds("download-settings-duplicate-reject")
        .expect("reject duplicate strategy");
    window.simulate_click(reject_duplicates.center(), Modifiers::default());
    let first_category_enabled = window
        .debug_bounds("download-settings-category-enabled-button")
        .expect("selected category enable control");
    window.simulate_click(first_category_enabled.center(), Modifiers::default());
    let initial_category_count = window.read(|cx| settings.read(cx).categories.len());
    let add_category = window
        .debug_bounds("download-settings-category-add")
        .expect("add category control");
    window.simulate_click(add_category.center(), Modifiers::default());
    window.run_until_parked();
    let custom_category_id = window.read(|cx| {
        let settings = settings.read(cx);
        assert_eq!(settings.categories.len(), initial_category_count + 1);
        settings.categories.last().expect("new category").id.clone()
    });
    window.update(|window, cx| {
        settings.update(cx, |settings, cx| {
            let category = settings.categories.last().expect("new category editor");
            category
                .name
                .update(cx, |input, cx| input.replace_all("镜像包", window, cx));
            category
                .save_directory
                .update(cx, |input, cx| input.replace_all("Mirrors", window, cx));
            category
                .extensions
                .update(cx, |input, cx| input.replace_all("img, iso", window, cx));
            category.url_patterns.update(cx, |input, cx| {
                input.replace_all("https://mirror.example/*", window, cx)
            });
        });
    });
    let original_directory =
        window.read(|cx| settings.read(cx).directory.read(cx).value().to_string());
    window.update(|_, cx| {
        application.update(cx, |app, cx| {
            app.handle_platform_notification(
                PlatformNotification::CommandResult {
                    correlation_id: uuid::Uuid::new_v4(),
                    result: PlatformCommandResult {
                        success: true,
                        error: None,
                        native_error_code: None,
                        data: Some(directory.join("wrong-target").display().to_string()),
                    },
                },
                cx,
            );
        });
    });
    window.run_until_parked();
    assert_eq!(
        window.read(|cx| settings.read(cx).directory.read(cx).value().to_string()),
        original_directory,
        "a new-task picker result must not mutate the settings draft"
    );
    settings.update_in(window, |view, window, cx| {
        view.request_directory(window, cx)
    });
    window.run_until_parked();
    let command_id = window.read(|cx| {
        application
            .read(cx)
            .pending_directory_picker
            .as_ref()
            .expect("registered original settings picker")
            .request
            .command_id
    });
    window.update(|_, cx| {
        application.update(cx, |app, cx| {
            app.handle_platform_notification(
                PlatformNotification::CommandResult {
                    correlation_id: command_id,
                    result: PlatformCommandResult {
                        success: true,
                        error: None,
                        native_error_code: None,
                        data: Some(selected_directory.display().to_string()),
                    },
                },
                cx,
            );
        });
    });
    window.run_until_parked();
    assert_eq!(
        window.read(|cx| settings.read(cx).directory.read(cx).value().to_string()),
        selected_directory.display().to_string()
    );
    let transfer_navigation =
        official_download_navigation_bounds(window, DownloadSettingsPage::Transfer);
    window.simulate_click(transfer_navigation.center(), Modifiers::default());
    window.run_until_parked();
    window.refresh().expect("render media tools section");
    let ffmpeg_picker = window
        .debug_bounds("download-settings-pick-ffmpeg")
        .expect("FFmpeg executable picker button");
    window.simulate_click(ffmpeg_picker.center(), Modifiers::default());
    window.run_until_parked();
    let picker_command_id = window.read(|cx| {
        let application = application.read(cx);
        let commands = application.ffmpeg_platform_commands.borrow();
        let (command_id, command) = commands.last().expect("FFmpeg picker command");
        assert!(matches!(command, PlatformCommand::PickProgram(_)));
        *command_id
    });
    window.update(|_, cx| {
        application.update(cx, |application, cx| {
            application.handle_platform_notification(
                PlatformNotification::CommandResult {
                    correlation_id: picker_command_id,
                    result: PlatformCommandResult {
                        success: true,
                        error: None,
                        native_error_code: None,
                        data: Some(ffmpeg_executable.display().to_string()),
                    },
                },
                cx,
            );
        });
    });
    window.run_until_parked();
    assert_eq!(
        window.read(|cx| {
            settings
                .read(cx)
                .ffmpeg_executable
                .read(cx)
                .value()
                .to_string()
        }),
        ffmpeg_executable.display().to_string()
    );
    for (button, expected_source, expected_url) in [
        (
            "download-settings-ffmpeg-gyan",
            FfmpegDownloadSource::Gyan,
            FFMPEG_GYAN_DOWNLOAD_URL,
        ),
        (
            "download-settings-ffmpeg-btbn",
            FfmpegDownloadSource::Btbn,
            FFMPEG_BTBN_DOWNLOAD_URL,
        ),
    ] {
        let bounds = window
            .debug_bounds(button)
            .expect("public FFmpeg download button");
        window.simulate_click(bounds.center(), Modifiers::default());
        window.run_until_parked();
        let command_id = window.read(|cx| {
            let application = application.read(cx);
            let commands = application.ffmpeg_platform_commands.borrow();
            let (command_id, command) = commands.last().expect("FFmpeg page command");
            assert_eq!(
                command,
                &PlatformCommand::OpenShellTarget(OpenShellTargetCommand {
                    target: expected_url.into(),
                    action: ShellOpenAction::Open,
                })
            );
            assert_eq!(
                application
                    .pending_ffmpeg_operations
                    .get(command_id)
                    .map(|pending| pending.kind),
                Some(FfmpegPlatformOperationKind::DownloadPage(expected_source))
            );
            *command_id
        });
        window.update(|_, cx| {
            application.update(cx, |application, cx| {
                application.handle_platform_notification(
                    PlatformNotification::CommandResult {
                        correlation_id: command_id,
                        result: PlatformCommandResult {
                            success: true,
                            error: None,
                            native_error_code: None,
                            data: None,
                        },
                    },
                    cx,
                );
            });
        });
    }
    let window_navigation =
        official_download_navigation_bounds(window, DownloadSettingsPage::Window);
    window.simulate_click(window_navigation.center(), Modifiers::default());
    window.run_until_parked();
    window.run_until_parked();
    let completion_toggle = window
        .debug_bounds("download-settings-completion-window")
        .expect("completion setting toggle");
    window.simulate_click(completion_toggle.center(), Modifiers::default());
    let sound_toggle = window
        .debug_bounds("download-settings-completion-sound")
        .expect("completion sound toggle");
    window.simulate_click(sound_toggle.center(), Modifiers::default());
    let directory_toggle = window
        .debug_bounds("download-settings-completion-directory")
        .expect("completion directory toggle");
    window.simulate_click(directory_toggle.center(), Modifiers::default());
    window.run_until_parked();
    let taskbar_overlay_toggle = window
        .debug_bounds("download-settings-taskbar-overlay")
        .expect("taskbar overlay setting toggle");
    window.simulate_click(taskbar_overlay_toggle.center(), Modifiers::default());
    let taskbar_left = window
        .debug_bounds("download-settings-taskbar-left")
        .expect("taskbar overlay left anchor");
    window.simulate_click(taskbar_left.center(), Modifiers::default());
    let apply = window
        .debug_bounds("download-settings-apply")
        .expect("settings apply button");
    window.simulate_click(apply.center(), Modifiers::default());
    window.run_until_parked();

    let persisted = AppConfig::load_from(&config_path).expect("reload applied settings");
    assert_eq!(
        persisted.download.default_directory,
        selected_directory.display().to_string()
    );
    assert_eq!(
        persisted.download.ffmpeg_executable_path,
        ffmpeg_executable.display().to_string()
    );
    assert!(
        persisted.download.default_request_headers
            == "X-NxTools-Default: desktop\nReferer: https://example.test/",
        "applied Header settings must round-trip without printing their value"
    );
    assert!(
        persisted.download.default_cookie == "session=desktop",
        "applied Cookie settings must round-trip without printing their value"
    );
    assert!(persisted.download.accept_invalid_certificates);
    assert!(persisted.download.automatic_categories_enabled);
    assert_eq!(
        persisted.download.duplicate_strategy,
        DownloadDuplicateStrategySettings::Reject
    );
    assert!(!persisted.download.categories[0].enabled);
    let custom_category = persisted
        .download
        .categories
        .iter()
        .find(|category| category.id == custom_category_id)
        .expect("custom category persisted");
    assert_eq!(custom_category.name, "镜像包");
    assert_eq!(custom_category.save_directory, "Mirrors");
    assert_eq!(custom_category.extensions, ["img", "iso"]);
    assert_eq!(custom_category.url_patterns, ["https://mirror.example/*"]);
    assert_eq!(
        engine_for_assertion
            .ffmpeg_resolution()
            .expect("resolve applied FFmpeg")
            .expect("applied FFmpeg should be available")
            .executable,
        ffmpeg_executable
    );
    assert!(!persisted.download.show_completion_window);
    assert!(!persisted.download.completion_sound_enabled);
    assert!(persisted.download.open_directory_on_completion);
    assert!(!persisted.download.taskbar_progress_overlay.enabled);
    assert_eq!(
        persisted.download.taskbar_progress_overlay.anchor,
        DownloadTaskbarOverlayAnchor::Left
    );
    assert!(
        !persisted.quick_launch.enabled,
        "applying download settings must preserve current quick-launch settings"
    );

    let application_for_cancel = application.clone();
    let cancel_initial = cx.read(|cx| application.read(cx).settings_view_initial());
    let (_cancel_view, cancel_window) = cx.add_window_view(move |window, cx| {
        DownloadSettingsView::new(application_for_cancel, cancel_initial, window, cx)
    });
    cancel_window.read(|cx| {
        let view = _cancel_view.read(cx);
        assert!(
            view.default_request_headers.read(cx).value()
                == "X-NxTools-Default: desktop\nReferer: https://example.test/",
            "reopened Header settings must match without printing their value"
        );
        assert!(
            view.default_cookie.read(cx).value() == "session=desktop",
            "reopened Cookie settings must match without printing their value"
        );
        assert!(view.accept_invalid_certificates);
        assert!(view.automatic_categories_enabled);
        assert_eq!(
            view.duplicate_strategy,
            DownloadDuplicateStrategySettings::Reject
        );
        assert!(!view.categories[0].enabled);
        assert!(!view.completion_sound_enabled);
        assert!(view.open_directory_on_completion);
    });
    cancel_window
        .refresh()
        .expect("render cancelled settings draft");
    let window_navigation =
        official_download_navigation_bounds(cancel_window, DownloadSettingsPage::Window);
    cancel_window.simulate_click(window_navigation.center(), Modifiers::default());
    cancel_window.run_until_parked();
    let progress_toggle = cancel_window
        .debug_bounds("download-settings-progress-window")
        .expect("progress setting toggle");
    cancel_window.simulate_click(progress_toggle.center(), Modifiers::default());
    let cancel = cancel_window
        .debug_bounds("download-settings-cancel")
        .expect("settings cancel button");
    cancel_window.simulate_click(cancel.center(), Modifiers::default());
    cancel_window.run_until_parked();
    assert!(
        AppConfig::load_from(&config_path)
            .expect("reload cancelled settings")
            .download
            .show_progress_window,
        "cancel must discard the local settings draft"
    );
    std::fs::remove_dir_all(directory).expect("remove settings fixture");
}

#[gpui_kit::test]
fn settings_persistence_failure_rolls_back_engine_values(cx: &mut TestAppContext) {
    let directory = unique_download_settings_test_directory("rollback");
    std::fs::create_dir_all(&directory).expect("create rollback directory");
    let missing_config = directory.join("missing").join("config.json");
    let engine = DownloadEngine::open_in_memory().expect("open rollback engine");
    let engine_for_assertion = engine.clone();
    let mut initial_config = AppConfig::default();
    initial_config.download.default_request_headers = "X-NxTools-Policy: persisted".into();
    initial_config.download.default_cookie = "persisted=1".into();
    let (platform, _platform_endpoint) = nxtools_ipc::platform_bridge(16);
    let notifications = cx.update(|cx| cx.new(|_| NotificationCenter::default()));
    let application = cx.update(|cx| {
        cx.new(|cx| {
            DownloadApplication::new(
                engine,
                Rc::new(platform),
                notifications,
                DownloadRuntimeContext {
                    fallback_default_directory: directory.clone(),
                    config: initial_config,
                    config_path: missing_config,
                    log_directory: directory.clone(),
                    database_path: directory.join("downloads.sqlite3"),
                    ffmpeg_install_directory: directory.join("tools").join("ffmpeg"),
                },
                cx,
            )
        })
    });
    let result = cx.update(|cx| {
        application.update(cx, |application, cx| {
            application.apply_settings(
                DownloadSettingsDraft {
                    default_directory: directory.clone(),
                    chrome_web_store_extension_id: String::new(),
                    automatic_categories_enabled: true,
                    categories: DownloadSettings::default().categories,
                    duplicate_strategy: DownloadDuplicateStrategySettings::AddNumbered,
                    double_click_action: DownloadRowDoubleClickActionSettings::OpenFile,
                    task_list_sort: None,
                    task_list_sort_descending: None,
                    ffmpeg_executable_path: String::new(),
                    default_request_headers: "X-NxTools-Policy: tentative".into(),
                    default_cookie: "tentative=1".into(),
                    accept_invalid_certificates: true,
                    default_connection_limit: 12,
                    adaptive_segment_scaling_enabled: true,
                    adaptive_segment_max_segments: 32,
                    adaptive_segment_min_mib: 3,
                    max_concurrent_tasks: 7,
                    global_speed_limit_bytes_per_second: Some(1024 * 1024),
                    background_speed_limit: Default::default(),
                    show_progress_window: false,
                    show_completion_window: false,
                    completion_sound_enabled: true,
                    open_directory_on_completion: false,
                    task_removal_deletes_file: true,
                    taskbar_progress_overlay: DownloadTaskbarProgressOverlaySettings::default(),
                },
                uuid::Uuid::new_v4(),
                cx,
            )
        })
    });
    assert!(result.is_err());
    assert_eq!(engine_for_assertion.global_speed_limit(), None);
    let task_id = engine_for_assertion
        .enqueue(
            DownloadRequest {
                source_url: "https://example.test/file.bin".into(),
                save_directory: directory.clone(),
                save_directory_explicit: false,
                file_name: Some("policy.bin".into()),
                headers: Vec::new(),
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
        .expect("accept request after persistence failure");
    let accepted = engine_for_assertion
        .task(task_id)
        .expect("query accepted policy task")
        .expect("policy task exists");
    assert!(
        accepted.task.request().cookie.as_deref() == Some("persisted=1"),
        "rollback must keep the prior Cookie without printing its value"
    );
    assert_eq!(
        accepted.task.request().certificate_policy,
        DownloadCertificatePolicy::Strict
    );
    assert!(accepted.task.request().headers.iter().any(|header| {
        header.name.eq_ignore_ascii_case("x-nxtools-policy") && header.value == "persisted"
    }));
    let snapshot = engine_for_assertion.handle_command(DownloadCommandEnvelope {
        protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
        request_id: uuid::Uuid::new_v4(),
        origin: RequestOrigin::Internal,
        command: DownloadCommand::Snapshot {
            query: DownloadQuery::default(),
        },
    });
    let DownloadCommandOutcome::Snapshot(snapshot) = snapshot.outcome else {
        panic!("expected rollback snapshot");
    };
    assert_eq!(
        snapshot
            .queues
            .iter()
            .find(|queue| queue.id == "default")
            .and_then(|queue| queue.max_concurrent_tasks),
        Some(3)
    );
    std::fs::remove_dir_all(directory).expect("remove rollback fixture");
}

#[test]
fn queue_only_closes_the_add_window_without_opening_a_progress_window() {
    let task_id = DownloadTaskId::new();

    assert_eq!(
        submit_presentation_plan(DownloadSubmitMode::QueueOnly, task_id, false, true),
        DownloadSubmitPresentationPlan {
            close_source_window: true,
            deferred_task_window: None,
        }
    );
}

#[test]
fn delayed_submission_closes_the_add_window_without_opening_progress_early() {
    let task_id = DownloadTaskId::new();

    assert_eq!(
        submit_presentation_plan(DownloadSubmitMode::StartNow, task_id, true, true),
        DownloadSubmitPresentationPlan {
            close_source_window: true,
            deferred_task_window: None,
        }
    );
}

#[test]
fn daily_schedule_draft_preserves_independent_triggers_and_weekday_order() {
    let days = [DownloadWeekday::Friday, DownloadWeekday::Monday]
        .into_iter()
        .collect();
    let schedule = build_download_schedule(true, false, &days, "22:30", "invalid")
        .expect("build start-only schedule")
        .expect("enabled schedule");
    assert!(schedule.daily.enabled_start);
    assert!(!schedule.daily.enabled_stop);
    assert_eq!(schedule.daily.start_minute, 22 * 60 + 30);
    assert_eq!(
        schedule.daily.days,
        vec![DownloadWeekday::Monday, DownloadWeekday::Friday]
    );
}

#[test]
fn daily_schedule_draft_rejects_missing_days_and_invalid_enabled_time() {
    let no_days = HashSet::new();
    assert_eq!(
        build_download_schedule(true, true, &no_days, "09:00", "17:00"),
        Err("至少选择一个生效星期".into())
    );
    let weekdays = [DownloadWeekday::Monday].into_iter().collect();
    assert_eq!(
        build_download_schedule(false, true, &weekdays, "ignored", "24:00"),
        Err("停止时间：时间必须在 00:00 到 23:59 之间".into())
    );
    assert_eq!(
        build_download_schedule(false, false, &no_days, "ignored", "ignored"),
        Ok(None)
    );
}

#[test]
fn optional_delayed_start_uses_selected_local_date_and_time() {
    let now = Local::now();
    assert_eq!(
        resolve_scheduled_start(None, now.timestamp_millis()),
        Ok(None),
        "no committed selection means an immediate download"
    );

    let future = now + ChronoDuration::days(2);
    let scheduled = resolve_scheduled_start(
        Some(DelayedStartSelection {
            date: future.date_naive(),
            hour: future.hour(),
            minute: future.minute(),
        }),
        now.timestamp_millis(),
    )
    .expect("future delayed start")
    .expect("selected date creates a delayed start");
    assert!(scheduled > now.timestamp_millis());

    assert_eq!(
        resolve_scheduled_start(
            Some(DelayedStartSelection {
                date: (now - ChronoDuration::days(1)).date_naive(),
                hour: 12,
                minute: 0,
            }),
            now.timestamp_millis(),
        ),
        Err("计划时间必须晚于当前时间".into())
    );
}

#[test]
fn delayed_start_calendar_uses_chinese_year_month_and_a_stable_sunday_grid() {
    let july = NaiveDate::from_ymd_opt(2026, 7, 1).expect("valid month");
    let days = delayed_start_calendar_days(july);

    assert_eq!(delayed_start_calendar_title(july), "2026年7月");
    assert_eq!(days.len(), 42);
    assert_eq!(days.first().copied(), NaiveDate::from_ymd_opt(2026, 6, 28));
    assert_eq!(days.last().copied(), NaiveDate::from_ymd_opt(2026, 8, 8));
    assert_eq!(
        shift_calendar_month(july, -7),
        NaiveDate::from_ymd_opt(2025, 12, 1).expect("valid previous month")
    );
    assert_eq!(
        shift_calendar_month(july, 6),
        NaiveDate::from_ymd_opt(2027, 1, 1).expect("valid next month")
    );
}

#[test]
fn a_new_active_cli_task_opens_one_desktop_progress_window() {
    for status in [
        DownloadStatus::Probing,
        DownloadStatus::Waiting,
        DownloadStatus::Downloading,
        DownloadStatus::Finalizing,
        DownloadStatus::Verifying,
    ] {
        assert!(should_open_admitted_progress_window(
            false,
            false,
            RequestOrigin::Cli,
            status
        ));
    }
    assert!(!should_open_admitted_progress_window(
        true,
        false,
        RequestOrigin::Cli,
        DownloadStatus::Downloading
    ));
    assert!(!should_open_admitted_progress_window(
        false,
        false,
        RequestOrigin::Cli,
        DownloadStatus::Created
    ));
    assert!(!should_open_admitted_progress_window(
        false,
        false,
        RequestOrigin::Ui,
        DownloadStatus::Downloading
    ));
    assert!(should_open_admitted_progress_window(
        true,
        true,
        RequestOrigin::Ui,
        DownloadStatus::Downloading
    ));
}

#[test]
fn removal_waits_for_the_single_task_removed_refresh() {
    assert!(
        !refresh_immediately_after_remove(),
        "the initiating action and TaskRemoved must not both rebuild the list"
    );
}

#[test]
fn add_window_submission_never_opens_a_gpui_window_while_the_view_is_leased() {
    let source = include_str!("../downloads.rs");
    let dialog_source = include_str!("../downloads/new_download_view.rs");
    let application_submit = source
        .split("    pub(crate) fn submit_with_idempotency(")
        .nth(1)
        .and_then(|body| body.split("    pub(crate) fn pause(").next())
        .expect("DownloadApplication::submit body");
    let view_submit = dialog_source
        .split("    pub(crate) fn submit(")
        .nth(1)
        .and_then(|body| body.split("\nimpl Render for NewDownloadView").next())
        .expect("NewDownloadView::submit body");

    assert!(
        !application_submit.contains("request_task_window("),
        "application submission must remain presentation-free"
    );
    assert!(
        view_submit.contains("cx.defer(move |cx|"),
        "cross-entity submission must leave the current GPUI entity callback"
    );
    let close = view_submit
        .find("source_window.update")
        .expect("source add window is closed after acceptance");
    let deferred_open = view_submit
        .find("application.request_task_window")
        .expect("task window opening is scheduled after acceptance");
    assert!(
        deferred_open < close,
        "the replacement task window must be requested before the source window is retired"
    );
}

#[test]
fn task_window_handoff_inherits_the_predecessor_position_natively() {
    // The user report: probe window → downloading → completed transitions
    // lose the window position because GPUI's cached origin never follows a
    // user drag. The replacement window must consume a native stash of the
    // predecessor's real rectangle, and every closing window must stash it.
    let source = include_str!("../downloads/task_windows_impl.rs");
    let open_body = source
        .split("    pub(crate) fn open_task_window(")
        .nth(1)
        .and_then(|body| body.split("\n    pub(crate) fn update_task_window").next())
        .expect("open_task_window body");
    assert!(
        open_body.contains("MoveWindowCenteredOverKey"),
        "a newly opened task window must inherit the stashed predecessor position"
    );

    let clear_body = source
        .split("    pub(crate) fn clear_task_window_registration(")
        .nth(1)
        .and_then(|body| body.split("\n    pub(crate) fn request_task_window").next())
        .expect("clear_task_window_registration body");
    assert!(
        clear_body.contains("stash_task_window_anchor"),
        "a closing task window must stash its real position for the next reopen"
    );

    let dialog_source = include_str!("../downloads/new_download_view.rs");
    let submit_body = dialog_source
        .split("    pub(crate) fn submit(")
        .nth(1)
        .and_then(|body| body.split("\nimpl Render for NewDownloadView").next())
        .expect("NewDownloadView::submit body");
    let stash = submit_body.find("stash_task_window_anchor");
    let close = submit_body.find("plan.close_source_window");
    assert!(stash.is_some(), "dialog submission must stash its position");
    assert!(
        close.is_some(),
        "dialog submission closes the source window"
    );
    assert!(
        stash < close,
        "the dialog position must be stashed before the dialog is destroyed"
    );
}

#[test]
fn task_window_handoff_places_before_revealing_the_replacement() {
    let source = include_str!("../downloads/task_windows_impl.rs");
    let open_body = source
        .split("    pub(crate) fn open_task_window(")
        .nth(1)
        .and_then(|body| body.split("\n    pub(crate) fn update_task_window").next())
        .expect("open_task_window body");
    let hidden = open_body
        .find("show: false")
        .expect("replacement starts hidden");
    let placement = open_body
        .find("MoveWindowCenteredOverKey")
        .expect("replacement is placed over the native anchor");
    let reveal = open_body
        .find("SetWindowVisibility")
        .expect("replacement is revealed through the native visibility boundary");
    assert!(hidden < placement && placement < reveal);
}
#[test]
fn task_window_render_never_reads_the_parent_download_application() {
    let source = include_str!("../downloads/task_window.rs");
    let task_window_render = source
        .split("impl Render for DownloadTaskWindowView")
        .nth(1)
        .and_then(|body| body.split("\nfn download_window_titlebar").next())
        .expect("DownloadTaskWindowView render body");

    assert!(
        !task_window_render.contains(".application.read(cx)"),
        "task windows opened or refreshed by DownloadApplication must render from pushed snapshots"
    );
}

#[test]
fn task_windows_use_compact_content_driven_phase_sizes() {
    let progress = download_task_window_size(DownloadWindowPhase::Progress);
    let failed = download_task_window_size(DownloadWindowPhase::Failed);
    let completed = download_task_window_size(DownloadWindowPhase::Completed);

    assert_eq!(
        (progress.width.as_f32(), progress.height.as_f32()),
        (440.0, 184.0)
    );
    assert_eq!(
        (failed.width.as_f32(), failed.height.as_f32()),
        (440.0, 196.0)
    );
    assert_eq!(
        (completed.width.as_f32(), completed.height.as_f32()),
        (440.0, 184.0)
    );
    let collapsed = download_task_window_size_for_details(DownloadWindowPhase::Progress, false);
    let expanded = download_task_window_size_for_details(DownloadWindowPhase::Progress, true);
    assert_eq!(collapsed, progress);
    assert_eq!(
        (expanded.width.as_f32(), expanded.height.as_f32()),
        (440.0, TASK_WINDOW_PROGRESS_EXPANDED_HEIGHT)
    );
    assert_eq!(
        download_task_window_size_for_details(DownloadWindowPhase::Failed, true),
        failed
    );
    assert_eq!(
        download_task_window_size_for_details(DownloadWindowPhase::Completed, true),
        completed
    );
}

#[test]
fn probe_resize_delegates_move_size_recovery_to_platform_host() {
    let source = include_str!("../downloads/new_download_view.rs");
    assert!(source.contains("fn apply_new_download_window_resize_with_correlation("));
    assert!(source.contains("PlatformCommand::SetWindowBoundsCentered"));
    assert!(source.contains("this.probe_generation != generation"));
    assert!(!source.contains("gpui_kit::Timer::after(retry_delay)"));
    assert!(!source.contains("Duration::from_millis(4_000)"));
    assert_eq!(
        source.matches("window.resize(window_size)").count(),
        1,
        "only HWND-less test/fallback windows use GPUI resize; production has one native owner"
    );
    assert!(source.contains("self.resize_window_with_correlation("));
    assert!(source.contains("let correlation_id = uuid::Uuid::new_v4();"));
    assert!(source.contains("correlation_id,"));
}

#[test]
fn task_window_phase_resize_delegates_geometry_to_the_native_side() {
    // GPUI's cached window origin never follows a user drag, so the phase
    // transform must not compute centered geometry from `window.bounds()`.
    // The C# side derives the real center and applies one atomic
    // SetWindowPos; the Rust boundary only carries the physical size.
    let source = include_str!("../downloads/task_windows_impl.rs");
    let update_body = source
        .split("fn update_task_window")
        .nth(1)
        .and_then(|body| body.split("\n    fn queue_task_window_activation").next())
        .expect("update_task_window body");

    assert!(
        update_body.contains("SetWindowBoundsCentered"),
        "phase resize must use the native centered-bounds command"
    );
    assert!(
        !update_body.contains("window.bounds()"),
        "phase resize must not trust GPUI's stale window origin"
    );
}

#[test]
fn progress_window_exposes_expandable_segment_details() {
    // Approved monitor retains disclosure/native budget and replaces minimal
    // dots with explicit number/status/downloaded/total columns.
    let source = include_str!("../downloads/task_window.rs");
    for marker in [
        "download-window-details-toggle",
        "download-window-details",
        "download-window-segment-table",
        "TASK_WINDOW_PROGRESS_EXPANDED_HEIGHT",
        "分段进度：序号、状态、已下载、总大小",
    ] {
        assert!(
            source.contains(marker),
            "progress window render must contain {marker}"
        );
    }
}

#[test]
fn segment_status_never_fabricates_completion_for_unknown_size() {
    let segment = |downloaded: u64, total: Option<u64>| nxtools_download::DownloadSegmentProgress {
        segment_index: 0,
        downloaded_bytes: downloaded,
        total_bytes: total,
    };
    assert_eq!(
        segment_display_status(&segment(0, Some(100)), DownloadStatus::Downloading),
        "等待"
    );
    assert_eq!(
        segment_display_status(&segment(50, Some(100)), DownloadStatus::Downloading),
        "传输中"
    );
    assert_eq!(
        segment_display_status(&segment(150, Some(100)), DownloadStatus::Downloading),
        "已完成"
    );
    assert_eq!(
        segment_display_status(&segment(10, None), DownloadStatus::Downloading),
        "传输中"
    );
    assert_eq!(
        segment_display_status(&segment(10, Some(0)), DownloadStatus::Paused),
        "已暂停"
    );
    assert_eq!(
        segment_display_status(&segment(10, None), DownloadStatus::Failed),
        "失败"
    );
    assert_eq!(
        segment_display_status(&segment(10, None), DownloadStatus::Cancelled),
        "已取消"
    );
}

#[test]
fn reopening_a_task_window_uses_the_native_restore_and_foreground_boundary() {
    let source = include_str!("../downloads/task_windows_impl.rs");
    let update_task_window = source
        .split("    pub(crate) fn update_task_window(")
        .nth(1)
        .and_then(|body| {
            body.split("    pub(crate) fn handle_platform_notification")
                .next()
        })
        .expect("DownloadApplication::update_task_window body");

    assert!(
        update_task_window.contains("queue_task_window_activation"),
        "native activation must be tracked for bounded terminal diagnostics"
    );
    assert_eq!(task_window_activation_command(0), None);
    assert!(matches!(
        task_window_activation_command(42),
        Some(PlatformCommand::ActivateWindow(ActivateWindowCommand {
            window_handle: 42
        }))
    ));
}

#[test]
fn a_late_activation_result_cannot_replace_a_new_window_generation() {
    let stale = TaskWindowIdentity {
        native_window_handle: 42,
        generation: uuid::Uuid::new_v4(),
    };
    let replacement = TaskWindowIdentity {
        native_window_handle: 42,
        generation: uuid::Uuid::new_v4(),
    };

    assert!(replacement_targets_current_window(Some(stale), stale));
    assert!(
        !replacement_targets_current_window(Some(replacement), stale),
        "even a recycled HWND must not let an old activation result remove the new window"
    );
    assert!(!replacement_targets_current_window(None, stale));
}

#[test]
fn stale_window_cleanup_discards_only_that_tasks_inflight_activation() {
    let stale_task = DownloadTaskId::new();
    let other_task = DownloadTaskId::new();
    let stale_window = TaskWindowIdentity {
        native_window_handle: 42,
        generation: uuid::Uuid::new_v4(),
    };
    let other_window = TaskWindowIdentity {
        native_window_handle: 84,
        generation: uuid::Uuid::new_v4(),
    };
    let stale_activation = uuid::Uuid::new_v4();
    let other_activation = uuid::Uuid::new_v4();
    let mut activations = HashMap::from([
        (
            stale_activation,
            PendingTaskWindowActivation {
                task_id: stale_task,
                window: stale_window,
                activation: TaskWindowActivation::Reused(DownloadWindowPhase::Progress),
                started_at: Instant::now(),
            },
        ),
        (
            other_activation,
            PendingTaskWindowActivation {
                task_id: other_task,
                window: other_window,
                activation: TaskWindowActivation::Reused(DownloadWindowPhase::Progress),
                started_at: Instant::now(),
            },
        ),
    ]);
    let mut replacements = HashMap::from([(stale_task, stale_window), (other_task, other_window)]);

    clear_pending_task_window_work(stale_task, &mut activations, &mut replacements);

    assert!(!activations.contains_key(&stale_activation));
    assert!(!replacements.contains_key(&stale_task));
    assert!(activations.contains_key(&other_activation));
    assert_eq!(replacements.get(&other_task), Some(&other_window));
}

#[test]
fn explicit_terminal_reopen_is_activating_while_automatic_fallback_is_passive() {
    let task_id = DownloadTaskId::new();
    let mut lifecycle = DownloadWindowLifecycle::default();

    assert_eq!(
        lifecycle.request_window(task_id, DownloadWindowPhase::Completed),
        DownloadWindowDirective::OpenAndFocus(DownloadWindowPhase::Completed)
    );
    lifecycle.window_closed(task_id);
    assert_eq!(
        lifecycle.reopen_terminal_fallback(task_id, DownloadWindowPhase::Completed),
        DownloadWindowDirective::Open(DownloadWindowPhase::Completed)
    );
    assert!(
        !TaskWindowActivation::Fresh(DownloadWindowPhase::Completed).replace_on_failure(),
        "a fresh window must not loop through replacements when native activation is rejected"
    );
    assert!(
        TaskWindowActivation::Reused(DownloadWindowPhase::Completed).replace_on_failure(),
        "a rejected reused-window activation must fall back to one guarded replacement"
    );
}

#[test]
fn new_download_window_expands_only_after_probe_confirmation() {
    let initial = new_download_window_size(false);
    let confirmed = new_download_window_size(true);

    assert_eq!(
        (initial.width.as_f32(), initial.height.as_f32()),
        (560.0, 176.0)
    );
    assert_eq!(
        (confirmed.width.as_f32(), confirmed.height.as_f32()),
        (560.0, NEW_DOWNLOAD_CONFIRMED_HEIGHT)
    );
}

#[test]
fn confirmed_form_is_withheld_until_the_native_viewport_reaches_its_height() {
    let target = Some(px(NEW_DOWNLOAD_CONFIRMED_HEIGHT));

    // A window dragged during the probe keeps the compact 176px viewport:
    // the confirmed form must not paint into it.
    assert!(confirmed_window_pending(target, false, px(176.0)));
    // Once WM_SIZE reports the confirmed height the form is released.
    assert!(!confirmed_window_pending(
        target,
        false,
        px(NEW_DOWNLOAD_CONFIRMED_HEIGHT)
    ));
    // The gate is derived from the live viewport, so a drag that rewrites the
    // compact size after the first release withholds the form again.
    assert!(confirmed_window_pending(target, false, px(176.0)));
    // Timeout never makes a too-short viewport safe to render into.
    assert!(confirmed_window_pending(target, true, px(176.0)));
    // Error layouts and non-native windows never set a target.
    assert!(!confirmed_window_pending(None, false, px(176.0)));
}

#[test]
fn successful_probe_uses_one_fixed_window_handoff_not_a_resize_watchdog() {
    let source = include_str!("../downloads/new_download_view.rs");
    let owner = include_str!("../downloads/browser_handoff_impl.rs");
    assert!(source.contains("create_fixed_confirmation"));
    assert!(!source.contains("NEW_DOWNLOAD_CONFIRMED_GATE_TIMEOUT"));
    assert!(!source.contains("download.confirmed_gate.expired"));
    assert!(owner.contains("PresentFixedWindowOverAnchor"));
    assert!(owner.contains("FIXED_CONFIRMATION_IPC_TIMEOUT"));
    assert!(owner.contains("self.pending_new_download_handoff = Some("));
    assert!(owner.contains("send_command_with_id(command, operation_id)"));
    assert!(owner.contains("view.probe_generation == generation"));
}

#[test]
fn withheld_confirmed_form_cannot_be_submitted_by_enter_or_any_button() {
    let source = include_str!("../downloads/new_download_view.rs");
    let submit = source
        .split("    pub(crate) fn submit(")
        .nth(1)
        .and_then(|body| body.split("\n    }\n").next())
        .expect("NewDownloadView::submit body");
    assert!(
        submit.contains("self.confirmed_window_pending(window)"),
        "submit is the single choke point for Enter, Download and Queue"
    );
    let enter = source
        .split("    fn default_enter_key_down(")
        .nth(1)
        .and_then(|body| body.split("\n    pub(crate) fn category_choice_ids").next())
        .expect("focused outer-input default Enter handler body");
    assert!(
        enter.contains("self.confirmed_window_pending(window)"),
        "Enter must neither submit nor start another probe while the form is withheld"
    );
}

#[test]
fn confirmed_probe_keeps_destination_and_filename_editable() {
    let confirmed = DownloadRequest {
        source_url: "https://example.com/original".into(),
        save_directory: PathBuf::from(r"C:\Downloads"),
        save_directory_explicit: false,
        file_name: Some("server-name.bin".into()),
        headers: Vec::new(),
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
    };
    let current = DownloadRequest {
        save_directory: PathBuf::from(r"D:\Archive"),
        file_name: Some("renamed.bin".into()),
        category_id: Some("documents".into()),
        category_name: None,
        duplicate_strategy: nxtools_download::DownloadDuplicateStrategy::Reject,
        ..confirmed.clone()
    };

    let merged = merge_confirmed_download_request(confirmed, current)
        .expect("destination-only edits do not invalidate a network probe");

    assert_eq!(merged.save_directory, PathBuf::from(r"D:\Archive"));
    assert_eq!(merged.file_name.as_deref(), Some("renamed.bin"));
    assert_eq!(merged.category_id.as_deref(), Some("documents"));
    assert_eq!(
        merged.duplicate_strategy,
        nxtools_download::DownloadDuplicateStrategy::Reject
    );
}

#[test]
fn download_titlebars_close_through_an_explicit_gpui_action() {
    let source = include_str!("../downloads/window_support.rs");
    assert!(source.contains("titlebar::controlled_titlebar("));
    assert!(
        source.contains("|_, window, _| window.remove_window()"),
        "shared caption preserves explicit original close owner"
    );
}

#[test]
fn probe_completion_hydrates_the_visible_file_name_input() {
    let source = include_str!("../downloads/new_download_view.rs");
    let begin_probe = source
        .split("    pub(crate) fn begin_probe(")
        .nth(1)
        .and_then(|body| body.split("\n    pub(crate) fn submit(").next())
        .expect("NewDownloadView::begin_probe body");

    assert!(
        begin_probe.contains("this.file_name.clone(),")
            && begin_probe.contains("resolved_file_name,")
            && begin_probe.contains("file_name.update"),
        "the resolved server filename must be written into the editable file-name field"
    );
    assert!(
        begin_probe.contains("input.set_value"),
        "the file-name input must visibly update when probing completes"
    );
}

#[test]
fn hls_probe_resolves_manifest_name_to_transport_stream_artifact() {
    let probe = DownloadProbe {
        resolution: None,
        final_url: "https://example.test/high/index.m3u8".into(),
        file_name: Some("index.m3u8".into()),
        total_bytes: None,
        supports_ranges: false,
        content_type: Some("application/vnd.apple.mpegurl".into()),
        validator: nxtools_download::ResourceValidator::default(),
        media: Some(nxtools_download::DownloadMediaProbe::Hls(
            nxtools_download::HlsProbe {
                is_master: false,
                variants: Vec::new(),
            },
        )),
    };

    assert_eq!(
        resolved_probe_file_name(&probe).as_deref(),
        Some("index.ts")
    );
    assert!(should_replace_media_manifest_name(
        Some("index.m3u8"),
        &probe
    ));
    assert!(
        !should_replace_media_manifest_name(Some("my-custom-name.m3u8"), &probe),
        "a user-edited manifest-looking name must remain user-owned"
    );
}

#[test]
fn settings_save_preserves_the_task_list_sort_preference() {
    // The user report: clicking a table header persisted the sort, but
    // saving ANY download setting later wiped it because the settings
    // draft hardcodes task_list_sort: None. current_draft must restore
    // the view-open-time preference into the saved config.
    let source = include_str!("settings_view.rs");
    let draft_body = source
        .split("pub(crate) fn current_draft")
        .nth(1)
        .and_then(|body| {
            body.split("pub(crate) fn toggle_taskbar_overlay_edit")
                .next()
        })
        .expect("current_draft body");
    assert!(
        draft_body.contains("draft.task_list_sort = self.task_list_sort.clone();"),
        "settings save must carry the task-list sort preference through"
    );
}

#[test]
fn progress_events_refresh_memory_and_open_windows_without_full_snapshot() {
    let source = include_str!("../downloads.rs");
    let progress_branch = source
        .split("fn handle_engine_event(")
        .nth(1)
        .and_then(|body| body.split("\n}\n\nmod window_support").next())
        .expect("download engine event handler");
    let progress_return = progress_branch
        .find("if !sequence_gap && let Some(task_id) = progress_task_id")
        .expect("progress-only early return");
    let snapshot_refresh = progress_branch
        .rfind("self.refresh_snapshot(true, cx)")
        .expect("non-progress snapshot refresh");
    assert!(
        progress_return < snapshot_refresh,
        "progress events must return before the full snapshot refresh"
    );
    assert!(progress_branch.contains("refresh_open_task_window_for_progress"));
}

#[test]
fn snapshot_refresh_error_is_bounded_and_keeps_a_user_visible_prefix() {
    let error = DownloadApplication::bounded_snapshot_error(&"x".repeat(400));
    assert!(error.starts_with("下载列表刷新失败："));
    assert!(error.ends_with('…'));
    assert!(error.chars().count() <= 190);
}

#[gpui_kit::test]
fn retrying_an_expanded_task_window_resets_details_and_native_progress_height(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_kit::init);
    let (application, directory, platform_endpoint) =
        download_application_for_window_test_with_platform(cx, "task-window-retry-details");
    platform_endpoint.set_connected_for_contract_test(true);
    let task_id = cx.read(|cx| {
        application
            .read(cx)
            .engine
            .enqueue(
                DownloadRequest {
                    source_url: "https://example.test/retry-details.bin".into(),
                    save_directory: directory.clone(),
                    save_directory_explicit: false,
                    file_name: Some("retry-details.bin".into()),
                    headers: Vec::new(),
                    cookie: None,
                    referrer: None,
                    certificate_policy: Default::default(),
                    connection_limit: Some(1),
                    speed_limit_bytes_per_second: None,
                    expected_total_bytes: Some(4096),
                    scheduled_at_unix_ms: None,
                    adaptive_segment_policy: None,
                    media: None,
                    category_id: None,
                    category_name: None,
                    duplicate_strategy: nxtools_download::DownloadDuplicateStrategy::UseDefault,
                },
                RequestOrigin::Internal,
            )
            .expect("enqueue retry-details fixture")
    });
    cx.update(|cx| {
        application.update(cx, |application, cx| {
            application.refresh(cx);
            let _ = application.configure_task_window_activation_test(42);
            application.request_task_window(task_id, cx);
        });
    });
    cx.run_until_parked();

    let (task_window, task_view, base_snapshot) = cx.read(|cx| {
        let application = application.read(cx);
        let entry = application
            .task_windows
            .get(&task_id)
            .expect("keyed retry-details task window");
        (
            entry.window,
            entry.view.clone(),
            application.task(task_id).cloned().expect("task snapshot"),
        )
    });
    let scale = task_window
        .update(cx, |_, window, _| window.scale_factor())
        .expect("read task-window scale");
    let generation = cx.read(|cx| {
        application
            .read(cx)
            .task_windows
            .get(&task_id)
            .expect("task window generation")
            .generation
    });
    let mut window = VisualTestContext::from_window(task_window, cx);
    window
        .refresh()
        .expect("render initial progress task window");
    let details_toggle = window
        .debug_bounds("download-window-details-toggle")
        .expect("progress details toggle");
    window.simulate_click(details_toggle.center(), Modifiers::default());
    window.run_until_parked();
    window.refresh().expect("render expanded progress details");
    assert!(
        window.debug_bounds("download-window-details").is_some(),
        "the progress window must render details before retry"
    );
    assert!(cx.read(|cx| task_view.read(cx).details_expanded));
    let mut native_details_requested = false;
    while let Ok(command) = platform_endpoint.try_receive_command_for_contract_test() {
        if let nxtools_protocol::Message::PlatformCommand(PlatformCommand::SetWindowBoundsCentered(
            size,
        )) = command.message
            && size.height == (TASK_WINDOW_PROGRESS_EXPANDED_HEIGHT * scale).round() as i32
        {
            assert_eq!(size.window_handle, 42);
            native_details_requested = true;
        }
    }
    assert!(
        native_details_requested,
        "details must supersede any collapsed native recovery request"
    );
    window.simulate_resize(download_task_window_size_for_details(
        DownloadWindowPhase::Progress,
        true,
    ));

    let mut failed_snapshot = base_snapshot.clone();
    failed_snapshot
        .task
        .transition_to(DownloadStatus::Probing, 2)
        .expect("fixture enters probing before failure");
    failed_snapshot
        .task
        .transition_to(DownloadStatus::Failed, 3)
        .expect("fixture enters failed");
    cx.update(|cx| {
        application.update(cx, |application, cx| {
            application.replace_tasks_for_test(vec![failed_snapshot]);
            let directive = application
                .task_window_lifecycle
                .task_changed(task_id, DownloadStatus::Failed);
            assert_eq!(
                directive,
                DownloadWindowDirective::TransformExisting(DownloadWindowPhase::Failed)
            );
            application.apply_task_window_directive(task_id, directive, cx);
        });
    });
    cx.run_until_parked();
    window.refresh().expect("render failed task window");
    assert!(
        cx.read(|cx| task_view.read(cx).details_expanded),
        "the terminal transform keeps the prior details selection until progress is restored"
    );
    let failed_resize = std::iter::from_fn(|| {
        platform_endpoint
            .try_receive_command_for_contract_test()
            .ok()
    })
    .find_map(|envelope| match envelope.message {
        nxtools_protocol::Message::PlatformCommand(PlatformCommand::SetWindowBoundsCentered(
            command,
        )) => Some(command),
        _ => None,
    })
    .expect("failed phase native resize");
    assert_eq!(
        failed_resize.height,
        (download_task_window_size(DownloadWindowPhase::Failed)
            .height
            .as_f32()
            * scale)
            .round() as i32
    );
    assert_eq!(failed_resize.window_handle, 42);

    let mut retry_snapshot = base_snapshot.clone();
    retry_snapshot
        .task
        .transition_to(DownloadStatus::Probing, 4)
        .expect("retry fixture enters probing");
    retry_snapshot
        .task
        .transition_to(DownloadStatus::Waiting, 5)
        .expect("retry fixture returns to waiting");
    cx.update(|cx| {
        application.update(cx, |application, cx| {
            application.replace_tasks_for_test(vec![retry_snapshot]);
            let directive = application
                .task_window_lifecycle
                .task_changed(task_id, DownloadStatus::Waiting);
            assert_eq!(
                directive,
                DownloadWindowDirective::TransformExisting(DownloadWindowPhase::Progress)
            );
            application.apply_task_window_directive(task_id, directive, cx);
        });
    });
    cx.run_until_parked();
    window
        .refresh()
        .expect("render retried progress task window");
    assert!(
        window
            .debug_bounds("download-window-progress-capabilities")
            .is_some(),
        "restored progress must render the collapsed capabilities"
    );
    assert!(!cx.read(|cx| task_view.read(cx).details_expanded));
    let retry_resize = std::iter::from_fn(|| {
        platform_endpoint
            .try_receive_command_for_contract_test()
            .ok()
    })
    .find_map(|envelope| match envelope.message {
        nxtools_protocol::Message::PlatformCommand(PlatformCommand::SetWindowBoundsCentered(
            command,
        )) => Some(command),
        _ => None,
    })
    .expect("retried progress native resize");
    assert_eq!(
        retry_resize.height,
        (download_task_window_size(DownloadWindowPhase::Progress)
            .height
            .as_f32()
            * scale)
            .round() as i32
    );
    assert_eq!(retry_resize.window_handle, 42);
    assert_eq!(
        cx.read(|cx| {
            application
                .read(cx)
                .task_windows
                .get(&task_id)
                .expect("same task window after failed retry")
                .generation
        }),
        generation,
        "retry must transform the keyed task window in place"
    );

    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn retrying_terminal_task_collapses_expanded_progress_details() {
    assert!(!details_expanded_after_phase_transition(
        Some(DownloadWindowPhase::Failed),
        DownloadWindowPhase::Progress,
        true,
    ));
    assert!(!details_expanded_after_phase_transition(
        Some(DownloadWindowPhase::Completed),
        DownloadWindowPhase::Progress,
        true,
    ));
    assert!(details_expanded_after_phase_transition(
        Some(DownloadWindowPhase::Progress),
        DownloadWindowPhase::Progress,
        true,
    ));
}

#[test]
fn download_rows_use_the_complete_task_uuid_for_identity() {
    let source = include_str!("../main_window/download_page.rs");
    assert!(source.contains("let element_id = task_id.as_uuid().to_string();"));
    assert!(!source.contains("task_id.as_uuid().as_u64_pair()"));
}
#[test]
fn confirmed_download_inspector_motion_is_generation_scoped_and_opacity_only() {
    let source = include_str!("../downloads/new_download_view.rs");
    let inspector = source
        .split("pub(crate) fn new_download_probe_inspector(")
        .nth(1)
        .and_then(|body| body.split("pub(crate) fn hls_quality_options(").next())
        .expect("production probe inspector");
    let animation = inspector
        .split(".with_animation(")
        .nth(1)
        .and_then(|body| body.split(".into_any_element()").next())
        .expect("probe inspector content animation");

    assert!(animation.contains("(\"download-new-probe-inspector\", generation)"));
    assert!(animation.contains("crate::ui::motion::ENTER_DURATION"));
    assert!(animation.contains("surface.opacity(0.94 + 0.06 * delta)"));
    assert!(
        !animation.contains(".top("),
        "the fixed form/footer row must not move"
    );
    assert!(!animation.contains("SetWindowBoundsCentered"));
}

#[gpui_kit::test]
fn invalid_probe_retires_confirmed_generation_and_notifies_without_inline_error(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_kit::init);
    let (application, directory) =
        download_application_for_window_test(cx, "invalid-probe-generation");
    application.update(cx, |application, cx| {
        application.open_new_download_window(cx)
    });
    cx.run_until_parked();
    let handle = cx
        .read(|cx| application.read(cx).new_download_window)
        .expect("add window");
    let view = cx
        .read(|cx| application.read(cx).new_download_view.clone())
        .expect("add view");
    let mut window = VisualTestContext::from_window(handle, cx);
    window.update(|native, cx| {
        view.update(cx, |view, cx| {
            view.probe_generation = 17;
            view.confirmed_target_height = Some(px(278.0));
            view.confirmed_gate_expired = false;
            view.begin_probe(native, cx); // empty URL: validation failure before dispatch
            assert_ne!(
                view.probe_generation, 17,
                "invalid request must retire old results/watchdog"
            );
            assert!(
                view.confirmed_target_height.is_none(),
                "no old layout gate may survive validation failure"
            );
            assert!(!view.probing);
        });
    });
    window.run_until_parked();
    window.simulate_resize(new_download_window_size(false));
    window.refresh().expect("compact error frame");
    let error = window
        .debug_bounds("download-new-probe-guidance")
        .expect("visible compact feedback");
    let footer = window
        .debug_bounds("download-new-footer")
        .expect("retry footer");
    assert!(
        error.bottom() <= footer.top(),
        "feedback fits before native error resize arrives"
    );
    window.update(|_, cx| assert!(view.read(cx).error.is_none()));
    application.read_with(&window, |app, cx| {
        assert!(
            app.notifications
                .read(cx)
                .items()
                .iter()
                .any(|item| item.level == crate::notifications::NotificationLevel::Error)
        )
    });
    let _ = std::fs::remove_dir_all(directory);
}

#[gpui_kit::test]
fn compact_probe_error_replaces_guidance_before_native_reserve_arrives(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (application, directory) =
        download_application_for_window_test(cx, "compact-error-feedback");
    application.update(cx, |application, cx| {
        application.open_new_download_window(cx)
    });
    cx.run_until_parked();
    let handle = cx
        .read(|cx| application.read(cx).new_download_window)
        .expect("add window");
    let view = cx
        .read(|cx| application.read(cx).new_download_view.clone())
        .expect("add view");
    let mut window = VisualTestContext::from_window(handle, cx);
    window.update(|_, cx| {
        view.update(cx, |view, cx| {
            view.error = Some("窗口尺寸未能更新，请松开鼠标后重试探测".into());
            cx.notify();
        })
    });
    window.simulate_resize(new_download_window_size(false));
    window.refresh().expect("compact failure frame");
    assert!(
        window.debug_bounds("download-inline-error").is_none(),
        "no extra clipped row before reserve exists"
    );
    let feedback = window
        .debug_bounds("download-new-probe-guidance")
        .expect("compact feedback");
    let footer = window
        .debug_bounds("download-new-footer")
        .expect("retry controls");
    assert!(feedback.bottom() <= footer.top());
    let _ = std::fs::remove_dir_all(directory);
}

#[gpui_kit::test]
fn media_submit_error_preserves_all_probe_rows_and_native_target(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (application, directory) = download_application_for_window_test(cx, "media-submit-layout");
    application.update(cx, |application, cx| {
        application.open_new_download_window(cx)
    });
    cx.run_until_parked();
    let handle = cx
        .read(|cx| application.read(cx).new_download_window)
        .expect("window");
    let view = cx
        .read(|cx| application.read(cx).new_download_view.clone())
        .expect("view");
    let mut surface = VisualTestContext::from_window(handle, cx);
    let variant = nxtools_download::HlsVariantProbe {
        playlist_url: "https://example.test/video.m3u8".into(),
        bandwidth: 1_000_000,
        average_bandwidth: None,
        resolution: None,
        frame_rate_milli: None,
        codecs: "avc1".into(),
        audio_group: None,
        subtitles_group: None,
    };
    surface.simulate_resize(size(px(560.0), px(424.0)));
    surface.update(|window, cx| {
        view.update(cx, |view, cx| {
            view.probe = Some(DownloadProbe {
                resolution: None,
                final_url: "https://example.test/master.m3u8".into(),
                file_name: Some("master.ts".into()),
                total_bytes: None,
                supports_ranges: false,
                content_type: None,
                validator: Default::default(),
                media: Some(nxtools_download::DownloadMediaProbe::Hls(
                    nxtools_download::HlsProbe {
                        is_master: true,
                        variants: vec![variant; 5],
                    },
                )),
            });
            view.confirmed_target_height = Some(px(418.0));
            // Empty URL causes the same validation error before queue dispatch.
            view.submit(DownloadSubmitMode::StartNow, window, cx);
            assert!(view.error.is_none());
        });
        assert_eq!(
            window.bounds().size.height,
            px(424.0),
            "media validation must not grow the existing window"
        );
    });
    surface.simulate_resize(size(px(560.0), px(424.0)));
    surface.refresh().expect("media error layout");
    assert!(
        surface
            .debug_bounds("download-new-probe-inspector")
            .is_some()
    );
    let footer = surface.debug_bounds("download-new-footer").expect("footer");
    assert!(surface.debug_bounds("download-inline-error").is_none());
    let options = surface
        .debug_bounds("download-hls-quality-options")
        .expect("all media options retained");
    assert!(
        options.bottom() <= footer.top(),
        "media options={options:?} footer={footer:?}"
    );
    surface.run_until_parked();
    application.read_with(&surface, |app, cx| {
        assert!(
            app.notifications
                .read(cx)
                .items()
                .iter()
                .any(|item| item.level == crate::notifications::NotificationLevel::Error)
        )
    });
    let _ = std::fs::remove_dir_all(directory);
}

fn kit_media_radio_snapshot(
    window: &mut VisualTestContext,
    group: &'static str,
    index: usize,
) -> gpui_kit::base::test_support::ElementSnapshot {
    window
        .update(|window, _| gpui_kit::base::test_support::snapshots(window))
        .into_iter()
        .find(|node| {
            node.role() == Some(gpui_kit::Role::RadioButton)
                && node.path().contains(&gpui_kit::ElementId::from(group))
                && node.path().last() == Some(&gpui_kit::ElementId::from(index))
        })
        .expect("actual current-frame media Radio")
}

#[gpui_kit::test]
fn foreground_background_cap_preserves_latest_manual_limit_and_restores_on_disconnect(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_kit::init);
    let (application, directory) = download_application_for_window_test(cx, "foreground-cap");
    let foreground = |process_id| {
        PlatformNotification::Event(PlatformEvent::ForegroundChanged(
            nxtools_protocol::ForegroundContext {
                window_handle: 42,
                focus_window_handle: 42,
                process_id,
                process_name: "synthetic.exe".into(),
                window_title: String::new(),
            },
        ))
    };
    cx.update(|cx| {
        application.update(cx, |application, cx| {
            application.config.download.background_speed_limit =
                nxtools_core::BackgroundDownloadLimitSettings {
                    enabled: true,
                    bytes_per_second: 2048,
                };
            application
                .set_global_speed_limit(Some(4096), uuid::Uuid::new_v4())
                .expect("manual limit");
            application
                .handle_platform_notification(foreground(std::process::id().saturating_add(1)), cx);
            assert_eq!(
                application.engine.effective_global_speed_limit(),
                Some(2048)
            );
            assert_eq!(application.engine.global_speed_limit(), Some(4096));
            application
                .set_global_speed_limit(Some(1024), uuid::Uuid::new_v4())
                .expect("lower manual limit");
            assert_eq!(
                application.engine.effective_global_speed_limit(),
                Some(1024)
            );
            application.handle_platform_notification(foreground(std::process::id()), cx);
            assert_eq!(
                application.engine.effective_global_speed_limit(),
                Some(1024)
            );
            application
                .set_global_speed_limit(None, uuid::Uuid::new_v4())
                .expect("manual unlimited");
            application
                .handle_platform_notification(foreground(std::process::id().saturating_add(1)), cx);
            assert_eq!(
                application.engine.effective_global_speed_limit(),
                Some(2048)
            );
            application.config.download.background_speed_limit.enabled = false;
            application
                .apply_background_speed_policy(uuid::Uuid::new_v4())
                .expect("disable temporary policy");
            assert_eq!(application.engine.effective_global_speed_limit(), None);
            application.config.download.background_speed_limit.enabled = true;
            application
                .apply_background_speed_policy(uuid::Uuid::new_v4())
                .expect("enable temporary policy");
            application.handle_platform_notification(
                PlatformNotification::Disconnected {
                    generation: 1,
                    pending_command_ids: Vec::new(),
                },
                cx,
            );
            assert_eq!(application.engine.effective_global_speed_limit(), None);
            assert_eq!(application.engine.global_speed_limit(), None);
        })
    });
    drop(application);
    let _ = std::fs::remove_dir_all(directory);
}

#[gpui_kit::test]
fn confirmed_extension_receipt_rejection_does_not_reparse_or_create_task(cx: &mut TestAppContext) {
    struct CountResolver(std::sync::atomic::AtomicUsize);
    impl nxtools_download::DownloadRequestResolver for CountResolver {
        fn identity(&self) -> nxtools_download::DownloadResolverIdentity {
            nxtools_download::DownloadResolverIdentity {
                content_version: "fixture".into(),
                settings_version: "settings".into(),
            }
        }
        fn resolve<'a>(
            &'a self,
            _: &'a DownloadRequest,
            _: &'a nxtools_download::DownloadResolutionOperation,
        ) -> nxtools_download::DownloadResolutionFuture<'a> {
            Box::pin(async move {
                self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Err(nxtools_download::DownloadResolutionError::Rejected)
            })
        }
    }
    cx.update(gpui_kit::init);
    let (application, directory) =
        download_application_for_window_test(cx, "extension-receipt-reject");
    let resolver = std::sync::Arc::new(CountResolver(std::sync::atomic::AtomicUsize::new(0)));
    cx.read(|cx| {
        application
            .read(cx)
            .engine
            .set_request_resolver(resolver.clone())
    })
    .expect("inject");
    let application_for_window = application.clone();
    let output = directory.clone();
    let slot = Rc::new(RefCell::new(None));
    let mount = slot.clone();
    let (_root, window) = cx.add_window_view(move |window, cx| {
        let view = cx.new(|cx| {
            NewDownloadView::new(
                NewDownloadViewInit {
                    application: application_for_window,
                    default_directory: output,
                    default_connection_limit: 1,
                    default_adaptive_segment_policy: Default::default(),
                    categories: DownloadSettings::default().categories,
                    initial_source: None,
                },
                window,
                cx,
            )
        });
        *mount.borrow_mut() = Some(view.clone());
        Root::new(view, window, cx)
    });
    let view = slot.borrow().clone().expect("view");
    window.update(|window, cx| {
        let url = view.read(cx).url.clone();
        url.update(cx, |input, cx| {
            input.set_value("https://fixture.test/file", window, cx)
        });
        let name = view.read(cx).file_name.clone();
        name.update(cx, |input, cx| input.set_value("chosen.bin", window, cx));
        let request = view.read(cx).request(cx).expect("request");
        view.update(cx, |view, cx| {
            view.confirmed_request = Some(request.clone());
            view.probe = Some(DownloadProbe {
                final_url: request.source_url.clone(),
                file_name: request.file_name.clone(),
                total_bytes: Some(1),
                supports_ranges: false,
                content_type: None,
                validator: Default::default(),
                media: None,
                resolution: Some(Box::new(nxtools_download::DownloadProbeResolution {
                    receipt: uuid::Uuid::new_v4(),
                    expires_at_unix_ms: 0,
                    effective_request: request.clone(),
                    provenance: nxtools_download::DownloadResolutionProvenance {
                        original_request: request,
                        resolver: nxtools_download::DownloadResolverIdentity {
                            content_version: "fixture".into(),
                            settings_version: "settings".into(),
                        },
                        stages: Vec::new(),
                        operation_id: uuid::Uuid::new_v4(),
                        correlation_id: uuid::Uuid::new_v4(),
                        session_generation: 1,
                        bypassed: false,
                        extension_causality: None,
                    },
                })),
            });
            view.submit(DownloadSubmitMode::QueueOnly, window, cx);
        });
    });
    window.run_until_parked();
    assert_eq!(
        resolver.0.load(std::sync::atomic::Ordering::SeqCst),
        0,
        "receipt failure must not silently resolve again"
    );
    cx.read(|cx| {
        assert!(
            application
                .read(cx)
                .engine
                .tasks()
                .expect("tasks")
                .is_empty()
        );
        assert!(!view.read(cx).submitting);
        assert!(
            view.read(cx).error.is_none(),
            "no duplicate inline operation notice"
        );
        assert_eq!(
            view.read(cx).url.read(cx).value(),
            "https://fixture.test/file"
        );
        assert_eq!(view.read(cx).file_name.read(cx).value(), "chosen.bin");
        assert!(
            application
                .read(cx)
                .notifications
                .read(cx)
                .items()
                .iter()
                .any(|notice| notice.id.starts_with("download-submit-")
                    && notice.level == crate::notifications::NotificationLevel::Error)
        );
        assert!(view.read(cx).confirmed_request.is_some());
    });
    let _ = std::fs::remove_dir_all(directory);
}

#[gpui_kit::test]
fn chrome_store_published_identity_registers_without_manual_configuration(cx: &mut TestAppContext) {
    let (application, directory, endpoint) =
        download_application_for_window_test_with_platform(cx, "chrome-store-default");
    endpoint.set_connected_for_contract_test(true);
    for legacy_value in [None, Some(""), Some("*")] {
        while endpoint.try_receive_command_for_contract_test().is_ok() {}
        application.update(cx, |app, cx| {
            let mut json = serde_json::to_value(&app.config).expect("synthetic config");
            if let Some(value) = legacy_value {
                json["download"]["chrome_web_store_extension_id"] = serde_json::json!(value);
            } else {
                json["download"]
                    .as_object_mut()
                    .expect("download section")
                    .remove("chrome_web_store_extension_id");
            }
            app.config = serde_json::from_value(json).expect("legacy config");
            app.config.download.normalize();
            app.handle_platform_notification(PlatformNotification::Connected, cx);
        });
        let registration =
            std::iter::from_fn(|| endpoint.try_receive_command_for_contract_test().ok())
                .find_map(|envelope| match envelope.message {
                    nxtools_protocol::Message::PlatformCommand(
                        PlatformCommand::EnsureBrowserNativeMessaging(command),
                    ) => Some(command),
                    _ => None,
                })
                .expect("automatic registration command");
        assert_eq!(
            registration.chromium_extension_id,
            "eldfplghejhopcmgfkacimanhaojakph"
        );
        assert_eq!(
            registration.firefox_extension_id,
            "nxtools-download-interceptor@nxtools.local"
        );
    }
    let _ = std::fs::remove_dir_all(directory);
}

#[gpui_kit::test]
fn chrome_store_configured_identity_reaches_reconnect_registration(cx: &mut TestAppContext) {
    let (application, directory, endpoint) =
        download_application_for_window_test_with_platform(cx, "chrome-store-registration");
    endpoint.set_connected_for_contract_test(true);
    while endpoint.try_receive_command_for_contract_test().is_ok() {}
    application.update(cx, |app, cx| {
        let mut json = serde_json::to_value(&app.config).expect("synthetic config");
        json["download"]["chrome_web_store_extension_id"] =
            serde_json::json!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        app.config = serde_json::from_value(json).expect("optional store identity config");
        app.handle_platform_notification(PlatformNotification::Connected, cx);
    });
    let registration = std::iter::from_fn(|| endpoint.try_receive_command_for_contract_test().ok())
        .find_map(|envelope| match envelope.message {
            nxtools_protocol::Message::PlatformCommand(
                PlatformCommand::EnsureBrowserNativeMessaging(command),
            ) => Some(command),
            _ => None,
        })
        .expect("original registration command");
    assert_eq!(
        registration.chromium_extension_id, "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "configured store ID must reach the exact native allowlist instead of the fixed local ID"
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[gpui_kit::test]
fn chrome_store_settings_save_registration_and_failure_preserve_original_owners(
    cx: &mut TestAppContext,
) {
    let (app, directory, endpoint) =
        download_application_for_window_test_with_platform(cx, "chrome-store-save");
    endpoint.set_connected_for_contract_test(true);
    let config_path = directory.join("config.json");
    let mut disk = AppConfig::default();
    disk.clipboard.max_items = 2468;
    disk.save_to(&config_path).expect("synthetic config");
    let mut draft = app.read_with(cx, |app, _| app.settings_draft());
    draft.chrome_web_store_extension_id = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into();
    app.update(cx, |app, cx| {
        app.apply_settings(draft.clone(), uuid::Uuid::new_v4(), cx)
    })
    .expect("save store identity");
    assert_eq!(
        AppConfig::load_from(&config_path)
            .expect("saved config")
            .download
            .chrome_web_store_extension_id,
        draft.chrome_web_store_extension_id
    );
    assert_eq!(
        AppConfig::load_from(&config_path)
            .expect("other section")
            .clipboard
            .max_items,
        2468
    );
    let first = app.read_with(cx, |app, _| {
        app.pending_browser_registration
            .expect("pending before result")
            .0
    });
    app.update(cx, |app, cx| {
        app.handle_platform_notification(
            PlatformNotification::CommandResult {
                correlation_id: first,
                result: PlatformCommandResult {
                    success: true,
                    error: None,
                    native_error_code: None,
                    data: None,
                },
            },
            cx,
        )
    });
    assert!(app.read_with(cx, |app, _| app.pending_browser_registration.is_none()));
    while endpoint.try_receive_command_for_contract_test().is_ok() {}
    let before = std::fs::read(&config_path).expect("saved bytes");
    let mut invalid = draft.clone();
    invalid.chrome_web_store_extension_id = "*".into();
    assert!(
        app.update(cx, |app, cx| app.apply_settings(
            invalid,
            uuid::Uuid::new_v4(),
            cx
        ))
        .is_err()
    );
    assert_eq!(
        std::fs::read(&config_path).expect("unchanged bytes"),
        before
    );
    assert!(endpoint.try_receive_command_for_contract_test().is_err());
    std::fs::remove_file(&config_path).expect("force persistence failure");
    draft.chrome_web_store_extension_id = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".into();
    assert!(
        app.update(cx, |app, cx| app.apply_settings(
            draft,
            uuid::Uuid::new_v4(),
            cx
        ))
        .is_err()
    );
    assert_eq!(
        app.read_with(cx, |app, _| app
            .config
            .download
            .chrome_web_store_extension_id
            .clone()),
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
    );
    assert!(
        endpoint.try_receive_command_for_contract_test().is_err(),
        "failed save must not register new identity"
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[gpui_kit::test]
fn chrome_store_registration_stale_failure_and_deadline_are_scoped(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (app, directory, endpoint) =
        download_application_for_window_test_with_platform(cx, "chrome-store-terminal");
    endpoint.set_connected_for_contract_test(true);
    app.update(cx, |app, cx| {
        app.request_browser_registration(uuid::Uuid::new_v4(), cx)
    });
    let first = app.read_with(cx, |app, _| {
        app.pending_browser_registration.expect("first").0
    });
    app.update(cx, |app, cx| {
        app.request_browser_registration(uuid::Uuid::new_v4(), cx)
    });
    let second = app.read_with(cx, |app, _| {
        app.pending_browser_registration.expect("replacement").0
    });
    app.update(cx, |app, cx| {
        app.handle_platform_notification(
            PlatformNotification::CommandResult {
                correlation_id: first,
                result: PlatformCommandResult {
                    success: false,
                    error: Some("synthetic".into()),
                    native_error_code: Some(5),
                    data: None,
                },
            },
            cx,
        )
    });
    assert_eq!(
        app.read_with(cx, |app, _| app
            .pending_browser_registration
            .expect("still current")
            .0),
        second
    );
    cx.run_until_parked();
    cx.executor().advance_clock(Duration::from_secs(16));
    cx.run_until_parked();
    assert!(app.read_with(cx, |app, _| app.pending_browser_registration.is_none()));
    app.read_with(cx, |app, cx| {
        assert!(
            app.notifications
                .read(cx)
                .items()
                .iter()
                .any(|notice| notice.id.as_ref() == "browser-native-registration")
        )
    });
    let _ = std::fs::remove_dir_all(directory);
}

#[gpui_kit::test]
fn chrome_store_settings_hide_identity_and_preserve_legacy_draft_cancel_and_save(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_kit::init);
    let (app, directory, endpoint) =
        download_application_for_window_test_with_platform(cx, "chrome-store-ui");
    endpoint.set_connected_for_contract_test(true);
    let config_path = directory.join("config.json");
    let legacy_id = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    let mut config = AppConfig::default();
    config.download.chrome_web_store_extension_id = legacy_id.into();
    config.save_to(&config_path).expect("synthetic config");
    app.update(cx, |app, _| app.config = config);
    app.update(cx, |app, cx| app.open_settings_window(cx));
    cx.run_until_parked();
    let handle = app.read_with(cx, |app, _| app.settings_window.expect("settings window"));
    let view = app.read_with(cx, |app, _| {
        app.settings_view.clone().expect("settings view")
    });
    let mut surface = VisualTestContext::from_window(handle, cx);
    surface.refresh().expect("render settings");
    assert!(
        surface
            .debug_bounds("download-settings-chrome-store-id")
            .is_none(),
        "ordinary settings must not expose an extension ID input"
    );
    let search = surface
        .update(|window, _| gpui_kit::base::test_support::snapshots(window))
        .into_iter()
        .find(|snapshot| {
            snapshot.role() == Some(gpui_kit::Role::TextInput)
                && snapshot
                    .path()
                    .iter()
                    .any(|id| id == &gpui_kit::ElementId::from("settings-sidebar"))
        })
        .expect("official settings search")
        .bounds();
    surface.simulate_click(search.center(), gpui_kit::Modifiers::default());
    surface.simulate_input("Chrome 商店");
    surface.run_until_parked();
    surface.refresh().expect("filtered store row");
    let snapshots = surface.update(|window, _| gpui_kit::base::test_support::snapshots(window));
    assert!(
        !snapshots
            .iter()
            .any(|snapshot| snapshot.label() == Some("Chrome 商店扩展 ID"))
    );
    assert!(
        snapshots
            .iter()
            .any(|snapshot| snapshot.label() == Some("打开 Chrome 扩展商店")),
        "published extension has a normal store action instead of an ID field"
    );
    let chrome_button = snapshots
        .iter()
        .find(|snapshot| snapshot.label() == Some("打开 Chrome 扩展商店"))
        .expect("visible Chrome store action")
        .bounds();
    while endpoint.try_receive_command_for_contract_test().is_ok() {}
    surface.simulate_click(chrome_button.center(), gpui_kit::Modifiers::default());
    surface.run_until_parked();
    let shell = std::iter::from_fn(|| endpoint.try_receive_command_for_contract_test().ok())
        .find(|envelope| {
            matches!(
                envelope.message,
                nxtools_protocol::Message::PlatformCommand(PlatformCommand::OpenShellTarget(_))
            )
        })
        .expect("original Button callback dispatches Shell command on the next turn");
    assert!(matches!(shell.message,
        nxtools_protocol::Message::PlatformCommand(PlatformCommand::OpenShellTarget(target))
            if target.target.ends_with("/eldfplghejhopcmgfkacimanhaojakph")));
    app.update(&mut surface, |app, cx| {
        app.handle_platform_notification(
            PlatformNotification::CommandResult {
                correlation_id: shell.id,
                result: PlatformCommandResult {
                    success: true,
                    error: None,
                    native_error_code: None,
                    data: None,
                },
            },
            cx,
        );
    });
    view.update_in(&mut surface, |view, _, cx| {
        view.show_progress_window = !view.show_progress_window;
        assert_eq!(
            view.current_draft(cx)
                .expect("valid draft")
                .chrome_web_store_extension_id,
            legacy_id
        );
    });
    assert_eq!(
        AppConfig::load_from(&config_path)
            .expect("not yet saved")
            .download
            .chrome_web_store_extension_id,
        legacy_id
    );
    surface.update(|window, _| window.remove_window());
    cx.run_until_parked();
    assert_eq!(
        AppConfig::load_from(&config_path)
            .expect("cancelled draft")
            .download
            .chrome_web_store_extension_id,
        legacy_id
    );
    assert!(app.read_with(cx, |app, _| app.pending_browser_registration.is_none()));
    let initial = app.read_with(cx, |app, _| app.settings_view_initial());
    let retained = app.clone();
    let (view, surface) = cx.add_window_view(move |window, cx| {
        DownloadSettingsView::new(retained, initial, window, cx)
    });
    view.update_in(surface, |view, window, cx| {
        assert_eq!(
            view.current_draft(cx)
                .expect("legacy draft")
                .chrome_web_store_extension_id,
            legacy_id
        );
        view.show_progress_window = !view.show_progress_window;
        view.save(window, cx);
    });
    surface.run_until_parked();
    assert_eq!(
        AppConfig::load_from(&config_path)
            .expect("durable save")
            .download
            .chrome_web_store_extension_id,
        legacy_id
    );
    assert_eq!(
        AppConfig::load_from(&config_path)
            .expect("changed preference")
            .download
            .show_progress_window,
        !AppConfig::default().download.show_progress_window
    );
    assert!(
        app.read_with(surface, |app, _| app.pending_browser_registration.is_some()),
        "saving a retained legacy identity keeps the original registration retry behavior"
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[gpui_kit::test]
fn chrome_store_navigation_shares_original_correlated_lifetime(cx: &mut TestAppContext) {
    cx.executor().allow_parking();
    let (app, directory, endpoint) =
        download_application_for_window_test_with_platform(cx, "chrome-store-navigation");
    endpoint.set_connected_for_contract_test(true);
    while endpoint.try_receive_command_for_contract_test().is_ok() {}
    app.update(cx, |app, cx| app.open_chrome_store(cx));
    let command = endpoint
        .try_receive_command_for_contract_test()
        .expect("Chrome store Shell command");
    let current = command.id;
    let nxtools_protocol::Message::PlatformCommand(PlatformCommand::OpenShellTarget(target)) =
        command.message
    else {
        panic!("expected original Shell owner");
    };
    assert_eq!(
        target.target,
        "https://chromewebstore.google.com/detail/nxtools-download-intercep/eldfplghejhopcmgfkacimanhaojakph"
    );
    assert_eq!(target.action, ShellOpenAction::Open);
    assert_eq!(
        app.read_with(cx, |app, _| app
            .pending_browser_store_page
            .expect("pending")
            .0),
        current
    );
    app.update(cx, |app, cx| {
        app.open_chrome_store(cx);
        app.open_firefox_store(cx);
        app.handle_platform_notification(
            PlatformNotification::CommandResult {
                correlation_id: uuid::Uuid::new_v4(),
                result: PlatformCommandResult {
                    success: true,
                    error: None,
                    native_error_code: None,
                    data: None,
                },
            },
            cx,
        );
    });
    assert!(endpoint.try_receive_command_for_contract_test().is_err());
    assert_eq!(
        app.read_with(cx, |app, _| app
            .pending_browser_store_page
            .expect("stale result ignored")
            .0),
        current
    );
    app.update(cx, |app, cx| {
        app.handle_platform_notification(
            PlatformNotification::CommandResult {
                correlation_id: current,
                result: PlatformCommandResult {
                    success: true,
                    error: None,
                    native_error_code: None,
                    data: None,
                },
            },
            cx,
        )
    });
    assert!(app.read_with(cx, |app, _| app.pending_browser_store_page.is_none()));
    app.update(cx, |app, cx| app.open_firefox_store(cx));
    let firefox = endpoint
        .try_receive_command_for_contract_test()
        .expect("Firefox remains available");
    assert!(matches!(firefox.message,
        nxtools_protocol::Message::PlatformCommand(PlatformCommand::OpenShellTarget(target))
            if target.target == browser_handoff_impl::FIREFOX_STORE_URL));
    app.update(cx, |app, cx| {
        app.handle_platform_notification(
            PlatformNotification::Disconnected {
                generation: 1,
                pending_command_ids: vec![firefox.id],
            },
            cx,
        )
    });
    assert!(app.read_with(cx, |app, _| app.pending_browser_store_page.is_none()));
    app.update(cx, |app, cx| app.open_chrome_store(cx));
    cx.run_until_parked();
    cx.executor().advance_clock(Duration::from_secs(11));
    cx.run_until_parked();
    assert!(app.read_with(cx, |app, _| app.pending_browser_store_page.is_none()));
    let _ = std::fs::remove_dir_all(directory);
}

#[gpui_kit::test]
fn ux_repair_invalid_new_download_notifies_once_without_banner_resize_or_probe(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_kit::init);
    let (application, directory) = download_application_for_window_test(cx, "ux-validation");
    application.update(cx, |app, cx| app.open_new_download_window(cx));
    cx.run_until_parked();
    let (handle, view) = cx.read(|cx| {
        let app = application.read(cx);
        (
            app.new_download_window.expect("window"),
            app.new_download_view.clone().expect("view"),
        )
    });
    let mut surface = VisualTestContext::from_window(handle, cx);
    let before = surface.update(|window, _| window.bounds().size);
    let count = application.read_with(&surface, |app, cx| app.notifications.read(cx).items().len());
    view.update_in(&mut surface, |view, window, cx| {
        view.url
            .update(cx, |input, cx| input.set_value("", window, cx));
        view.begin_probe(window, cx);
        assert!(!view.probing);
        assert!(
            view.error.is_none(),
            "operation feedback uses existing notification, not an inline duplicate"
        );
        assert!(view.confirmed_request.is_none());
    });
    surface.run_until_parked();
    surface.refresh().expect("unchanged draft surface");
    assert_eq!(surface.update(|window, _| window.bounds().size), before);
    assert!(surface.debug_bounds("download-inline-error").is_none());
    application.read_with(&surface, |app, cx| {
        let notices = app.notifications.read(cx).items();
        assert_eq!(notices.len(), count + 1);
        assert_eq!(
            notices.last().expect("validation notice").level,
            crate::notifications::NotificationLevel::Error
        );
    });
    assert_eq!(
        view.read_with(&surface, |view, cx| view.url.read(cx).value().to_string()),
        ""
    );
    let _ = std::fs::remove_dir_all(directory);
}
