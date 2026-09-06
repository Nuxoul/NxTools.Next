//! Tests for the parent module. Extracted from the parent file so the
//! runtime code stays readable.

use super::*;


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
        let theme = Theme::zed_dark();

        assert_eq!(
            category_choice_background(theme, true, false),
            theme.accent_soft
        );
        assert_eq!(
            category_choice_background(theme, false, true),
            theme.element_selected
        );
        assert_ne!(
            category_choice_background(theme, true, false),
            category_choice_background(theme, false, true),
            "the committed selection and the single keyboard highlight need distinct surfaces"
        );
    }

    use gpui::{AppContext, Modifiers, TestAppContext, VisualTestContext};

    #[test]
    fn multiline_header_placeholder_stays_on_one_line_for_windows_directwrite() {
        assert!(!DEFAULT_REQUEST_HEADERS_PLACEHOLDER.contains(['\r', '\n']));
    }

    #[gpui::test]
    fn download_task_icons_use_the_typed_shell_boundary_and_publish_by_task_id(
        cx: &mut TestAppContext,
    ) {
        cx.update(gpui_component::init);
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
                            duplicate_strategy:
                                nxtools_download::DownloadDuplicateStrategy::UseDefault,
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
                            duplicate_strategy:
                                nxtools_download::DownloadDuplicateStrategy::UseDefault,
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

    #[gpui::test]
    fn taskbar_overlay_window_opens_without_reentering_download_application(
        cx: &mut TestAppContext,
    ) {
        cx.update(gpui_component::init);
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

    #[gpui::test]
    fn stale_native_taskbar_overlay_close_cannot_cancel_the_current_edit(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);
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

    #[gpui::test]
    fn production_taskbar_overlay_keeps_information_geometry_stable_in_edit_mode(
        cx: &mut TestAppContext,
    ) {
        cx.update(gpui_component::init);
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

    #[gpui::test]
    fn new_download_window_opens_without_reentering_download_application(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);
        let (application, directory) =
            download_application_for_window_test(cx, "new-download-window");

        cx.update(|cx| {
            application.update(cx, |application, cx| {
                application.open_new_download_window(cx);
            });
        });

        assert!(cx.read(|cx| { application.read(cx).new_download_window.as_ref().is_some() }));
        let _ = std::fs::remove_dir_all(directory);
    }

    #[gpui::test]
    fn production_new_download_category_form_and_bounded_menu_remain_readable(
        cx: &mut TestAppContext,
    ) {
        cx.update(gpui_component::init);
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
                && (file_name_field.bottom().as_f32() - category_field.bottom().as_f32()).abs()
                    <= 1.0
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
            .debug_bounds("download-new-file-name-control-label")
            .expect("production file-name label");
        let file_name_control = window
            .debug_bounds("download-new-file-name-control")
            .expect("production file-name control");
        let category_label = window
            .debug_bounds("download-new-category-control-label")
            .expect("production category label");
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
                px(LINE_HEIGHT_LABEL),
                "{name} label must use the shared Label line height"
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
        assert_eq!(directory_picker.size, gpui::size(px(28.0), px(28.0)));
        let url_paste = window
            .debug_bounds("download-new-url-paste")
            .expect("production URL paste button");
        assert_eq!(url_paste.size, gpui::size(px(28.0), px(28.0)));
        assert!(
            (url_paste.right().as_f32() - (form_grid.right().as_f32() - 16.0)).abs() <= 1.0,
            "the URL paste button sits at the right edge of the form width"
        );
        assert!(
            (url_control.left().as_f32() - (form_grid.left().as_f32() + 16.0)).abs() <= 1.0
                && (url_control.right().as_f32() - (form_grid.right().as_f32() - 16.0)).abs()
                    <= 1.0,
            "URL control row must fill the padded form width"
        );
        assert!(
            (directory_row.left().as_f32() - file_name_field.left().as_f32()).abs() <= 1.0
                && (directory_row.right().as_f32() - category_field.right().as_f32()).abs() <= 1.0,
            "directory path row must fill the file/category form column"
        );
        assert!(
            (file_name_label.top().as_f32() - category_label.top().as_f32()).abs() <= 1.0
                && (file_name_control.top().as_f32() - category_trigger.top().as_f32()).abs()
                    <= 1.0,
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
                    .height
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
        let category_content_left = first_category.left().as_f32() + SPACING_8;
        let category_content_right = first_category.right().as_f32() - SPACING_8;
        assert!(
            first_title.top() >= first_category.top()
                && first_title.bottom() <= first_directory.top()
                && first_directory.bottom() <= first_category.bottom()
                && (first_title.left().as_f32() - category_content_left).abs() <= 1.0
                && (first_title.right().as_f32() - category_content_right).abs() <= 1.0
                && (first_directory.left().as_f32() - category_content_left).abs() <= 1.0
                && (first_directory.right().as_f32() - category_content_right).abs() <= 1.0,
            "category name and directory must remain two bounded rows"
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
        let outside_popover = gpui::point(titlebar.left() + px(24.0), titlebar.center().y);
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

    #[gpui::test]
    fn production_delayed_start_uses_shared_popover_and_preserves_draft_lifecycle(
        cx: &mut TestAppContext,
    ) {
        cx.update(gpui_component::init);
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
                (28.0..=30.0).contains(&action.size.height.as_f32()),
                "schedule {name} must use the shared 28px tier, got {}px",
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
        let draft_date = committed.date + ChronoDuration::days(1);
        let draft_day = window
            .debug_bounds("download-new-schedule-next-day")
            .expect("next production schedule day");
        window.simulate_click(draft_day.center(), Modifiers::default());
        window.run_until_parked();
        window
            .refresh()
            .expect("render modified production schedule draft");
        assert_eq!(window.read(|cx| view.read(cx).scheduled_date), draft_date);
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
        let draft_day = window
            .debug_bounds("download-new-schedule-next-day")
            .expect("next production schedule day after reopening");
        window.simulate_click(draft_day.center(), Modifiers::default());
        window.run_until_parked();
        window
            .refresh()
            .expect("modify the reopened schedule draft");
        assert_eq!(window.read(|cx| view.read(cx).scheduled_date), draft_date);
        let titlebar = window
            .debug_bounds("download-new-titlebar")
            .expect("production new-download titlebar");
        let outside_popover = gpui::point(titlebar.right() - px(80.0), titlebar.center().y);
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

    #[gpui::test]
    fn browser_probe_window_options_allow_the_compact_native_minimum(cx: &mut TestAppContext) {
        let compact = new_download_window_size(false);
        let options =
            cx.update(|cx| new_download_window_options(WindowBounds::centered(compact, cx)));
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

    #[gpui::test]
    fn browser_download_window_shrinks_to_compact_height_while_probe_is_unconfirmed(
        cx: &mut TestAppContext,
    ) {
        cx.update(gpui_component::init);
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
        // The auto-probe of the browser handoff URL fails fast (nothing listens
        // on port 9), which surfaces the inline error strip. The dialog must
        // grow by exactly the error reserve instead of squeezing the footer.
        let compact = new_download_window_size(false).height.as_f32();
        assert_eq!(
            height,
            compact + f32::from(NEW_DOWNLOAD_ERROR_RESERVE_HEIGHT),
            "a failed browser probe must reserve the inline error row, not the confirmed form"
        );

        let _ = std::fs::remove_dir_all(directory);
    }

    #[gpui::test]
    fn production_probe_summary_uses_narrow_inspector_and_bounded_stage_geometry(
        cx: &mut TestAppContext,
    ) {
        cx.update(gpui_component::init);
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
            .update(cx, move |_, native_window, cx| {
                view.update(cx, |view, cx| {
                    view.probe = Some(probe);
                    cx.notify();
                });
                native_window.resize(confirmed_probe_size);
            })
            .expect("confirm production probe stage");
        window.run_until_parked();
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
        assert_eq!(
            confirmed_size.height.as_f32() - compact_size.height.as_f32(),
            102.0,
            "probe confirmation must add only the destination form height"
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
            ("download-new-probe-status", LINE_HEIGHT_LABEL),
            ("download-new-probe-size", LINE_HEIGHT_BODY),
            ("download-new-probe-capability", LINE_HEIGHT_META),
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

        let _ = std::fs::remove_dir_all(directory);
    }

    #[gpui::test]
    fn probe_error_grows_the_unconfirmed_dialog_and_keeps_the_footer_position(
        cx: &mut TestAppContext,
    ) {
        cx.update(gpui_component::init);
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
                // An invalid URL is the same begin_probe failure path the user
                // hits; it sets the error and reserves the row.
                view.update(cx, |view, cx| {
                    view.url.update(cx, |input, cx| {
                        input.set_value("not a valid url", window, cx);
                    });
                    view.begin_probe(window, cx);
                });
            })
            .expect("run failing probe");
        window.run_until_parked();
        window.refresh().expect("render error reserve");

        let base = new_download_window_size(false);
        let error_size = new_download_window
            .update(cx, |_, native_window, _| native_window.bounds().size)
            .expect("read error reserve bounds");
        assert_eq!(
            error_size,
            size(base.width, base.height + NEW_DOWNLOAD_ERROR_RESERVE_HEIGHT),
            "a probe error must grow the fixed-height dialog instead of squeezing the footer"
        );
        let footer_after = window
            .debug_bounds("download-new-footer")
            .expect("footer with error");
        assert_eq!(
            footer_after.size, footer_before.size,
            "footer geometry must not change when an error appears"
        );
        assert!(
            window.debug_bounds("download-inline-error").is_some(),
            "the inline error must render inside the grown dialog"
        );

        let _ = std::fs::remove_dir_all(directory);
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

    #[gpui::test]
    fn confirmed_new_download_schedule_and_actions_stay_inside_the_footer(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);
        let (application, directory) =
            download_application_for_window_test(cx, "new-download-layout");
        let application_for_window = application.clone();
        let default_directory = directory.clone();
        let default_connection_limit =
            cx.read(|cx| application.read(cx).default_connection_limit());
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
                && (file_name_field.bottom().as_f32() - category_field.bottom().as_f32()).abs()
                    <= 1.0,
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
                px(CONTROL_HEIGHT_ACTION),
                "{name} footer action must use the exact shared action tier"
            );
            assert_eq!(role, action, "{name} role must identify the visible action");
        }
        let refresh = window
            .debug_bounds("download-new-refresh-action")
            .expect("refresh footer utility");
        assert_eq!(
            refresh.size,
            size(px(CONTROL_HEIGHT_ACTION), px(CONTROL_HEIGHT_ACTION)),
            "refresh must remain a square shared action"
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
            gpui::point(root.left() + px(3.0), root.top() + px(17.0)),
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

    #[gpui::test]
    fn confirmed_hls_master_submits_the_selected_quality_as_typed_media(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);
        let (application, directory) =
            download_application_for_window_test(cx, "new-download-hls-quality");
        let application_for_window = application.clone();
        let default_directory = directory.clone();
        let default_connection_limit =
            cx.read(|cx| application.read(cx).default_connection_limit());
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
        let first_radio = window
            .debug_bounds("download-hls-quality-0-radio")
            .expect("first HLS radio control");
        let first_title = window
            .debug_bounds("download-hls-quality-0-title")
            .expect("first HLS quality title");
        let first_detail = window
            .debug_bounds("download-hls-quality-0-detail")
            .expect("first HLS quality detail");
        assert!(first_row.contains(&first_radio.origin));
        assert!(first_row.contains(&first_radio.bottom_right()));
        assert!(first_row.contains(&first_title.origin));
        assert!(first_row.contains(&first_detail.bottom_right()));
        assert_eq!(first_row.size.height, px(SETTINGS_ROW_MIN_HEIGHT));
        assert_eq!(first_radio.size, size(px(14.0), px(14.0)));
        assert_eq!(first_title.size.height, px(LINE_HEIGHT_BODY));
        assert_eq!(first_detail.size.height, px(LINE_HEIGHT_LABEL));
        assert_eq!(first_title.left(), first_detail.left());
        assert!(first_title.bottom() <= first_detail.top());
        let first_selected_dot = window
            .debug_bounds("download-hls-quality-0-radio-selected")
            .expect("selected HLS radio dot");
        assert_eq!(first_selected_dot.size, size(px(SPACING_6), px(SPACING_6)));
        assert!(
            first_radio.contains(&first_selected_dot.origin)
                && first_radio.contains(&first_selected_dot.bottom_right()),
            "the selected concrete quality must show a filled radio marker"
        );
        assert!(
            window
                .debug_bounds("download-hls-quality-1-radio-selected")
                .is_none(),
            "an unselected concrete quality must remain visually distinct"
        );
        assert!(
            window
                .debug_bounds("download-hls-quality-1-radio-unselected")
                .is_some()
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
        assert!(
            window
                .debug_bounds("download-hls-quality-1-radio-selected")
                .is_some()
        );
        assert!(
            window
                .debug_bounds("download-hls-quality-0-radio-unselected")
                .is_some(),
            "the previous concrete quality must render its current unselected state"
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
            333.0,
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
            402.0,
            "five HLS choices should use the bounded three-row viewport without extra footer whitespace"
        );
    }

    #[gpui::test]
    fn dash_probe_renders_track_choices_and_builds_a_typed_selection(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);
        let (application, directory) =
            download_application_for_window_test(cx, "new-download-dash-tracks");
        let application_for_window = application.clone();
        let default_directory = directory.clone();
        let default_connection_limit =
            cx.read(|cx| application.read(cx).default_connection_limit());
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
        let first_video_radio = window
            .debug_bounds("download-dash-video-track-0-radio")
            .expect("first DASH video radio control");
        let first_video_title = window
            .debug_bounds("download-dash-video-track-0-title")
            .expect("first DASH video title");
        let first_video_detail = window
            .debug_bounds("download-dash-video-track-0-detail")
            .expect("first DASH video detail");
        assert!(first_video_row.contains(&first_video_radio.origin));
        assert!(first_video_row.contains(&first_video_detail.bottom_right()));
        assert_eq!(first_video_row.size.height, px(SETTINGS_ROW_MIN_HEIGHT));
        assert_eq!(first_video_radio.size, size(px(14.0), px(14.0)));
        assert_eq!(first_video_title.size.height, px(LINE_HEIGHT_BODY));
        assert_eq!(first_video_detail.size.height, px(LINE_HEIGHT_LABEL));
        assert_eq!(first_video_title.left(), first_video_detail.left());
        assert!(first_video_title.bottom() <= first_video_detail.top());
        let first_video_dot = window
            .debug_bounds("download-dash-video-track-0-radio-selected")
            .expect("selected DASH video radio dot");
        assert_eq!(first_video_dot.size, size(px(SPACING_6), px(SPACING_6)));
        assert!(
            window
                .debug_bounds("download-dash-video-track-1-radio-selected")
                .is_none()
        );
        assert!(
            window
                .debug_bounds("download-dash-video-track-1-radio-unselected")
                .is_some()
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
        assert!(
            window
                .debug_bounds("download-dash-video-track-1-radio-selected")
                .is_some()
        );
        assert!(
            window
                .debug_bounds("download-dash-video-track-0-radio-unselected")
                .is_some()
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
        assert!(
            window
                .debug_bounds("download-dash-audio-track-0-radio-unselected")
                .is_some(),
            "cancelling the optional audio track must render an empty radio"
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

    #[gpui::test]
    fn browser_prefilled_download_window_preserves_private_request_context(
        cx: &mut TestAppContext,
    ) {
        cx.update(gpui_component::init);
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

    #[gpui::test]
    fn browser_blob_window_is_prefilled_from_staged_metadata_without_network_probe(
        cx: &mut TestAppContext,
    ) {
        cx.update(gpui_component::init);
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

    fn download_application_for_window_test(
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
        assert_eq!(
            titlebar.size.height,
            px(crate::ui::metrics::TITLEBAR_HEIGHT)
        );
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

    #[gpui::test]
    fn schedule_dialog_uses_shared_shell_and_preserves_cancel_close_apply_lifecycle(
        cx: &mut TestAppContext,
    ) {
        cx.update(gpui_component::init);
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
        assert_eq!(cancel.size.height, px(CONTROL_HEIGHT_ACTION));
        assert_eq!(apply.size.height, px(CONTROL_HEIGHT_ACTION));
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

    #[gpui::test]
    fn delete_dialog_uses_shared_danger_semantics_and_closes_only_after_a_decision(
        cx: &mut TestAppContext,
    ) {
        cx.update(gpui_component::init);
        let (application, directory) =
            download_application_for_window_test(cx, "delete-dialog-shell");
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
        assert_eq!(cancel.size.height, px(CONTROL_HEIGHT_ACTION));
        assert_eq!(confirm.size.height, px(CONTROL_HEIGHT_ACTION));
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

    #[gpui::test]
    fn settings_general_page_mounts_the_double_click_action_control(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);
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
        let general_page = general_source
            .split("DownloadSettingsPage::General => div()")
            .nth(1)
            .and_then(|source| {
                source
                    .split("DownloadSettingsPage::Request => div()")
                    .next()
            })
            .expect("general page render source");
        assert!(
            general_page.contains("double_click_action,"),
            "the list double-click control must render on the general page"
        );
        assert!(
            general_source.contains("download-settings-double-click-action"),
            "the segmented control helper must expose stable debug ids"
        );
        let _ = std::fs::remove_dir_all(directory);
    }

    #[gpui::test]
    fn download_settings_window_opens_without_reentering_download_application(
        cx: &mut TestAppContext,
    ) {
        cx.update(gpui_component::init);
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

    #[gpui::test]
    fn completion_feedback_crosses_the_bounded_platform_transport_once_per_batch(
        cx: &mut TestAppContext,
    ) {
        cx.update(gpui_component::init);
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
            nxtools_protocol::Message::PlatformCommand(
                PlatformCommand::PlaySystemNotificationSound
            )
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

    #[gpui::test]
    fn production_task_window_reuses_one_skeleton_and_generation_across_phases(
        cx: &mut TestAppContext,
    ) {
        cx.update(gpui_component::init);
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
            nxtools_protocol::Message::PlatformCommand(
                PlatformCommand::SetWindowBoundsCentered(command),
            ) => Some(command),
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
        completed_snapshot.final_path = Some(directory.join("shared-skeleton.bin"));
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
            assert_eq!(action.size.height, px(CONTROL_HEIGHT_ACTION));
        }
        assert_eq!(drag_action.size.width, px(CONTROL_HEIGHT_ACTION));
        assert!(drag_action.right() <= reveal_action.left());
        assert!(reveal_action.right() <= open_action.left());
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
        assert_eq!(
            platform_endpoint
                .try_receive_command_for_contract_test()
                .expect("completed reveal platform command")
                .message,
            nxtools_protocol::Message::PlatformCommand(PlatformCommand::OpenShellTarget(
                OpenShellTargetCommand {
                    target: directory.join("shared-skeleton.bin").display().to_string(),
                    action: ShellOpenAction::SelectInExplorer,
                }
            ))
        );
        assert!(
            !cx.read(|cx| application.read(cx).task_windows.contains_key(&task_id)),
            "opening the artifact from the completed window must close the terminal window"
        );
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
        let reopened_handle = cx.read(|cx| {
            application
                .read(cx)
                .task_windows
                .get(&task_id)
                .map(|entry| {
                    assert_ne!(
                        entry.generation, generation,
                        "the artifact-open close must release the keyed registration so a reopen creates a fresh window"
                    );
                    entry.window
                })
                .expect("reopened completed task window")
        });
        let mut window = VisualTestContext::from_window(reopened_handle, cx);
        window.refresh().expect("render reopened completed window");
        while platform_endpoint
            .try_receive_command_for_contract_test()
            .is_ok()
        {}
        window.simulate_click(open_action.center(), Modifiers::default());
        window.run_until_parked();
        assert_eq!(
            platform_endpoint
                .try_receive_command_for_contract_test()
                .expect("completed open platform command")
                .message,
            nxtools_protocol::Message::PlatformCommand(PlatformCommand::OpenShellTarget(
                OpenShellTargetCommand {
                    target: directory.join("shared-skeleton.bin").display().to_string(),
                    action: ShellOpenAction::Open,
                }
            ))
        );
        assert!(
            !cx.read(|cx| application.read(cx).task_windows.contains_key(&task_id)),
            "the Open action must also close the completed window"
        );

        let _ = task_window_handle.update(cx, |_, window, _| window.remove_window());
        let _ = std::fs::remove_dir_all(directory);
    }

    #[gpui::test]
    fn task_window_arms_and_cancels_the_real_shutdown_countdown(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);
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
        // Every task-window footer button shares one height; a drift here is
        // the "buttons don't match" regression the user reported.
        assert_eq!(arm.size.height, px(CONTROL_HEIGHT_ACTION));
        task_window.simulate_click(arm.center(), Modifiers::default());
        task_window.run_until_parked();
        assert!(cx.read(|cx| application.read(cx).is_shutdown_armed(task_id)));

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
        assert_eq!(cancel.size.height, px(CONTROL_HEIGHT_ACTION));
        assert_eq!(shutdown_now.size.height, px(CONTROL_HEIGHT_ACTION));
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

    #[gpui::test]
    fn a_native_task_window_close_releases_the_keyed_lifecycle_registration(
        cx: &mut TestAppContext,
    ) {
        cx.update(gpui_component::init);
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

    #[gpui::test]
    fn native_shutdown_window_close_cancels_the_countdown(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);
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

    #[gpui::test]
    fn stale_shutdown_arm_click_cannot_arm_a_task_that_already_terminated(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);
        let (application, directory) =
            download_application_for_window_test(cx, "shutdown-stale-arm");
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

    #[gpui::test]
    fn shutdown_now_crosses_the_typed_platform_boundary_and_closes_on_success(
        cx: &mut TestAppContext,
    ) {
        cx.update(gpui_component::init);
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

    #[gpui::test]
    fn closing_download_settings_always_leaves_taskbar_overlay_edit_mode(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);
        let (application, directory) =
            download_application_for_window_test(cx, "settings-edit-close");
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

    #[gpui::test]
    fn native_move_size_end_closes_only_the_matching_taskbar_edit_gesture(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);
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

    #[gpui::test]
    fn ffmpeg_platform_timeout_releases_picker_and_page_operations(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);
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

    #[gpui::test]
    fn settings_auto_download_commits_managed_ffmpeg_without_manual_save(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);
        let (application, directory) =
            download_application_for_window_test(cx, "ffmpeg-auto-install");
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
                application.ffmpeg_install_operation_override =
                    Some(FfmpegInstallOperationOverride {
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
        let transfer_navigation = window
            .debug_bounds("download-settings-nav-transfer")
            .expect("transfer settings navigation");
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

    #[gpui::test]
    fn settings_media_tools_gives_the_ffmpeg_path_a_full_width_row(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);
        let (application, directory) =
            download_application_for_window_test(cx, "ffmpeg-settings-layout");
        let application_for_window = application.clone();
        let initial = cx.read(|cx| application.read(cx).settings_view_initial());
        let (_settings, window) = cx.add_window_view(move |window, cx| {
            window.resize(size(px(984.0), px(650.0)));
            DownloadSettingsView::new(application_for_window, initial, window, cx)
        });
        window.refresh().expect("render FFmpeg settings layout");
        let transfer_navigation = window
            .debug_bounds("download-settings-nav-transfer")
            .expect("transfer settings navigation");
        window.simulate_click(transfer_navigation.center(), Modifiers::default());
        window.run_until_parked();
        window.refresh().expect("render transfer settings layout");

        let section = window
            .debug_bounds("download-settings-media-tools-section")
            .expect("media tools section");
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

    #[gpui::test]
    fn settings_sections_share_one_content_width_and_diagnostics_are_not_half_width(
        cx: &mut TestAppContext,
    ) {
        cx.update(gpui_component::init);
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
        assert_eq!(
            titlebar.size.height,
            px(crate::ui::metrics::TITLEBAR_HEIGHT)
        );
        assert_eq!(shell_footer.size.height, px(48.0));
        assert_eq!(shell_content.bottom(), shell_footer.top());
        assert_eq!(shell_footer.bottom(), shell.bottom());

        let navigation_title = window
            .debug_bounds("download-settings-nav-general-title")
            .expect("general navigation title");
        let navigation_description = window
            .debug_bounds("download-settings-nav-general-description")
            .expect("general navigation description");
        assert_eq!(navigation_title.size.height, px(LINE_HEIGHT_BODY));
        assert_eq!(navigation_description.size.height, px(LINE_HEIGHT_LABEL));

        let row_title = window
            .debug_bounds("download-settings-row-默认保存目录-title")
            .expect("default directory row title");
        let row_description = window
            .debug_bounds("download-settings-row-默认保存目录-description")
            .expect("default directory row description");
        assert_eq!(row_title.size.height, px(LINE_HEIGHT_BODY));
        assert_eq!(row_description.size.height, px(LINE_HEIGHT_LABEL));

        for (name, selector) in [
            ("cancel", "download-settings-cancel"),
            ("apply", "download-settings-apply"),
        ] {
            let action = window
                .debug_bounds(selector)
                .unwrap_or_else(|| panic!("missing {name} settings action"));
            assert_eq!(
                action.size.height,
                px(28.0),
                "{name} must use the shared 28px action tier"
            );
        }

        let save = window
            .debug_bounds("download-settings-section-保存")
            .expect("save section");
        let expected_left = save.origin.x.as_f32();
        let expected_width = save.size.width.as_f32();
        macro_rules! assert_section {
            ($name:literal, $selector:literal) => {{
                let bounds = window
                    .debug_bounds($selector)
                    .unwrap_or_else(|| panic!("missing {} section", $name));
                assert!(
                    (bounds.origin.x.as_f32() - expected_left).abs() <= 1.0,
                    "{} starts at {}px instead of {expected_left}px",
                    $name,
                    bounds.origin.x.as_f32()
                );
                assert!(
                    (bounds.size.width.as_f32() - expected_width).abs() <= 1.0,
                    "{} is {}px wide instead of the shared {expected_width}px content width",
                    $name,
                    bounds.size.width.as_f32()
                );
            }};
        }
        macro_rules! open_page {
            ($selector:literal) => {{
                let navigation = window
                    .debug_bounds($selector)
                    .expect("settings page navigation");
                window.simulate_click(navigation.center(), Modifiers::default());
                window.run_until_parked();
                window.refresh().expect("render selected settings page");
                assert_eq!(
                    window
                        .debug_bounds("download-settings-shell-footer")
                        .expect("stable settings footer"),
                    shell_footer,
                    "switching settings pages must not move the shared footer"
                );
            }};
        }
        assert_section!("分类与重名", "download-settings-section-分类与重名");
        assert!(
            window
                .debug_bounds("download-settings-section-传输")
                .is_none(),
            "inactive pages must not remain in one oversized scrolling column"
        );
        assert!(
            expected_width >= 760.0,
            "settings content remains unexpectedly narrow at {expected_width}px"
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
        assert_section!("请求上下文", "download-settings-section-请求上下文");
        assert_section!("证书校验", "download-settings-section-证书校验");
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
        let certificate_section = window
            .debug_bounds("download-settings-section-证书校验")
            .expect("certificate policy section");
        let certificate_toggle = window
            .debug_bounds("download-settings-accept-invalid-certificates")
            .expect("certificate policy toggle");
        assert_eq!(header_field.size.height, px(120.0));
        assert_eq!(cookie_field.size.height, px(CONTROL_HEIGHT_INPUT));
        assert_eq!(header_field.origin.x, cookie_field.origin.x);
        assert_eq!(header_field.size.width, cookie_field.size.width);
        assert_eq!(header_label.origin.x, cookie_label.origin.x);
        assert!(request_form.contains(&header_field.origin));
        assert!(request_form.contains(&header_field.bottom_right()));
        assert!(request_form.contains(&cookie_field.origin));
        assert!(request_form.contains(&cookie_field.bottom_right()));
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
                input.set_value(&long_header, window, cx);
            });
            cookie_input.update(cx, |input, cx| {
                input.set_value("session=bounded", window, cx);
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
            window
                .debug_bounds("download-settings-section-证书校验")
                .expect("stable certificate section after long Header input"),
            certificate_section,
            "long Header content must not move the separate certificate policy section"
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
        assert_section!("传输", "download-settings-section-传输");
        assert_section!("媒体工具", "download-settings-section-媒体工具");
        let media_rows = [
            "download-settings-stacked-row-FFmpeg 可执行文件",
            "download-settings-stacked-row-自动安装",
            "download-settings-stacked-row-公开下载来源",
        ]
        .map(|selector| {
            window
                .debug_bounds(selector)
                .unwrap_or_else(|| panic!("missing FFmpeg sequence row {selector}"))
        });
        assert!(
            media_rows.windows(2).all(|rows| {
                (rows[0].origin.x.as_f32() - rows[1].origin.x.as_f32()).abs() <= 1.0
                    && (rows[0].size.width.as_f32() - rows[1].size.width.as_f32()).abs() <= 1.0
                    && rows[1].origin.y.as_f32() > rows[0].origin.y.as_f32()
            }),
            "FFmpeg setup must render as three ordered full-width rows"
        );
        assert!(
            media_rows
                .iter()
                .all(|row| row.size.height.as_f32() <= 72.0),
            "FFmpeg setup rows must stay on the dense 72 px sequence tier"
        );
        let ffmpeg_path_control = window
            .debug_bounds("download-settings-ffmpeg-control")
            .expect("full-width FFmpeg path control");
        let ffmpeg_install_actions = window
            .debug_bounds("download-settings-ffmpeg-install-actions")
            .expect("full-width FFmpeg install action rail");
        let ffmpeg_source_actions = window
            .debug_bounds("download-settings-ffmpeg-source-actions")
            .expect("full-width FFmpeg source action rail");
        assert!(
            [
                ffmpeg_path_control,
                ffmpeg_install_actions,
                ffmpeg_source_actions
            ]
            .windows(2)
            .all(|controls| {
                (controls[0].origin.x.as_f32() - controls[1].origin.x.as_f32()).abs() <= 1.0
                    && (controls[0].size.width.as_f32() - controls[1].size.width.as_f32()).abs()
                        <= 1.0
            }),
            "path, managed install and public sources must share one full-width left control rail"
        );
        assert!(
            ffmpeg_path_control.size.width.as_f32()
                >= media_rows[0].size.width.as_f32() - SPACING_8 * 2.0 - 2.0,
            "the executable path must consume the full sequence-row content width"
        );
        let install_button = window
            .debug_bounds("download-settings-ffmpeg-install")
            .expect("primary managed-install action");
        let gyan_button = window
            .debug_bounds("download-settings-ffmpeg-gyan")
            .expect("quiet Gyan fallback action");
        assert!(
            (install_button.origin.x.as_f32() - ffmpeg_install_actions.origin.x.as_f32()).abs()
                <= 1.0
                && (gyan_button.origin.x.as_f32() - ffmpeg_source_actions.origin.x.as_f32()).abs()
                    <= 1.0,
            "managed install and public fallbacks must begin at the shared left rail"
        );
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
                );
            });
        });
        window.run_until_parked();
        window
            .refresh()
            .expect("render active FFmpeg installation in production settings");
        let installing_media_rows = [
            "download-settings-stacked-row-FFmpeg 可执行文件",
            "download-settings-stacked-row-自动安装",
            "download-settings-stacked-row-公开下载来源",
        ]
        .map(|selector| {
            window
                .debug_bounds(selector)
                .unwrap_or_else(|| panic!("missing active FFmpeg sequence row {selector}"))
        });
        assert_eq!(
            installing_media_rows, media_rows,
            "install progress and Cancel must not move or resize the FFmpeg sequence rows"
        );
        assert_eq!(
            window
                .debug_bounds("download-settings-shell-footer")
                .expect("fixed footer during FFmpeg installation"),
            shell_footer,
            "install progress and Cancel must not move the fixed settings footer"
        );
        let active_cancel = window
            .debug_bounds("download-settings-ffmpeg-cancel")
            .expect("quiet Cancel action during FFmpeg installation");
        assert!(
            installing_media_rows[1].contains(&active_cancel.origin)
                && installing_media_rows[1].contains(&active_cancel.bottom_right()),
            "the active Cancel action must remain inside the automatic-install row"
        );
        window.update(|_, cx| {
            application.update(cx, |application, cx| {
                application.set_ffmpeg_install_state(FfmpegInstallViewState::Idle, cx);
            });
        });
        window.run_until_parked();
        let first_transfer_cell = window
            .debug_bounds("download-settings-grid-传输-0")
            .expect("first transfer grid cell");
        let second_transfer_cell = window
            .debug_bounds("download-settings-grid-传输-1")
            .expect("second transfer grid cell");
        assert!(
            (first_transfer_cell.origin.y.as_f32() - second_transfer_cell.origin.y.as_f32()).abs()
                <= 1.0,
            "the first two transfer settings are not in the same grid row"
        );
        assert!(
            (first_transfer_cell.size.width.as_f32() - second_transfer_cell.size.width.as_f32())
                .abs()
                <= 1.0
                && (first_transfer_cell.size.width.as_f32()
                    + second_transfer_cell.size.width.as_f32()
                    + 3.0
                    - expected_width)
                    .abs()
                    <= 2.0,
            "transfer grid cells must equally fill the {expected_width}px section after its two 1px borders and 1px center seam; actual widths were {}px and {}px",
            first_transfer_cell.size.width.as_f32(),
            second_transfer_cell.size.width.as_f32()
        );
        let transfer_section = window
            .debug_bounds("download-settings-section-传输")
            .expect("continuous transfer settings section");
        let transfer_matrix_selectors = [
            (
                "download-settings-matrix-传输-row-0",
                "download-settings-matrix-传输-cell-0-left",
                "download-settings-matrix-传输-seam-0",
                "download-settings-matrix-传输-cell-0-right",
            ),
            (
                "download-settings-matrix-传输-row-1",
                "download-settings-matrix-传输-cell-1-left",
                "download-settings-matrix-传输-seam-1",
                "download-settings-matrix-传输-cell-1-right",
            ),
            (
                "download-settings-matrix-传输-row-2",
                "download-settings-matrix-传输-cell-2-left",
                "download-settings-matrix-传输-seam-2",
                "download-settings-matrix-传输-cell-2-right",
            ),
        ];
        let mut transfer_rows = Vec::new();
        for (row_index, (row_selector, left_selector, seam_selector, right_selector)) in
            transfer_matrix_selectors.into_iter().enumerate()
        {
            let row = window
                .debug_bounds(row_selector)
                .unwrap_or_else(|| panic!("missing transfer matrix row {row_index}"));
            let left = window
                .debug_bounds(left_selector)
                .unwrap_or_else(|| panic!("missing left transfer cell {row_index}"));
            let seam = window
                .debug_bounds(seam_selector)
                .unwrap_or_else(|| panic!("missing transfer seam {row_index}"));
            let right = window
                .debug_bounds(right_selector)
                .unwrap_or_else(|| panic!("missing right transfer cell {row_index}"));
            assert!(
                (left.origin.y.as_f32() - right.origin.y.as_f32()).abs() <= 1.0
                    && (left.size.height.as_f32() - right.size.height.as_f32()).abs() <= 1.0,
                "transfer matrix row {row_index} must share one vertical baseline"
            );
            assert!(
                (seam.size.width.as_f32() - 1.0).abs() <= 0.5
                    && (left.origin.x.as_f32() + left.size.width.as_f32() - seam.origin.x.as_f32())
                        .abs()
                        <= 1.0
                    && (seam.origin.x.as_f32() + seam.size.width.as_f32()
                        - right.origin.x.as_f32())
                    .abs()
                        <= 1.0,
                "transfer matrix row {row_index} must use one 1 px center seam without card gaps"
            );
            assert!(
                (left.size.width.as_f32() - right.size.width.as_f32()).abs() <= 2.0,
                "transfer matrix row {row_index} must split the parent into equal cells"
            );
            assert!(
                left.origin.x.as_f32() >= transfer_section.origin.x.as_f32()
                    && right.origin.x.as_f32() + right.size.width.as_f32()
                        <= transfer_section.origin.x.as_f32()
                            + transfer_section.size.width.as_f32()
                            + 1.0,
                "transfer matrix row {row_index} must stay inside the section width"
            );
            transfer_rows.push(row);
        }
        assert!(
            transfer_rows
                .windows(2)
                .all(
                    |rows| (rows[0].origin.x.as_f32() - rows[1].origin.x.as_f32()).abs() <= 1.0
                        && (rows[0].size.width.as_f32() - rows[1].size.width.as_f32()).abs() <= 1.0
                ),
            "all transfer matrix rows must share one parent-width geometry"
        );
        let left_control_edges = [
            "download-settings-connection-limit-field",
            "download-settings-adaptive-segments",
            "download-settings-adaptive-min-field",
        ]
        .map(|selector| {
            let bounds = window
                .debug_bounds(selector)
                .unwrap_or_else(|| panic!("missing transfer control {selector}"));
            bounds.origin.x.as_f32() + bounds.size.width.as_f32()
        });
        let right_control_edges = [
            "download-settings-concurrent-tasks-field",
            "download-settings-adaptive-max-field",
            "download-settings-speed-limit-field",
        ]
        .map(|selector| {
            let bounds = window
                .debug_bounds(selector)
                .unwrap_or_else(|| panic!("missing transfer control {selector}"));
            bounds.origin.x.as_f32() + bounds.size.width.as_f32()
        });
        assert!(
            left_control_edges
                .windows(2)
                .all(|edges| (edges[0] - edges[1]).abs() <= 1.0)
                && right_control_edges
                    .windows(2)
                    .all(|edges| (edges[0] - edges[1]).abs() <= 1.0),
            "transfer controls must align to stable left and right trailing rails"
        );
        open_page!("download-settings-nav-window");
        assert_section!("任务窗口", "download-settings-section-任务窗口");
        assert_section!("任务栏进度", "download-settings-section-任务栏进度");
        let window_matrix_sections = [
            ("任务窗口", "download-settings-section-任务窗口", 2_usize),
            (
                "任务栏进度",
                "download-settings-section-任务栏进度",
                7_usize,
            ),
        ];
        for (title, section_selector, row_count) in window_matrix_sections {
            let section = window
                .debug_bounds(section_selector)
                .unwrap_or_else(|| panic!("missing {title} matrix section"));
            for row_index in 0..row_count {
                let (row_selector, left_selector, seam_selector, right_selector) = match title {
                    "任务窗口" => [
                        (
                            "download-settings-matrix-任务窗口-row-0",
                            "download-settings-matrix-任务窗口-cell-0-left",
                            "download-settings-matrix-任务窗口-seam-0",
                            "download-settings-matrix-任务窗口-cell-0-right",
                        ),
                        (
                            "download-settings-matrix-任务窗口-row-1",
                            "download-settings-matrix-任务窗口-cell-1-left",
                            "download-settings-matrix-任务窗口-seam-1",
                            "download-settings-matrix-任务窗口-cell-1-right",
                        ),
                    ][row_index],
                    "任务栏进度" => [
                        (
                            "download-settings-matrix-任务栏进度-row-0",
                            "download-settings-matrix-任务栏进度-cell-0-left",
                            "download-settings-matrix-任务栏进度-seam-0",
                            "download-settings-matrix-任务栏进度-cell-0-right",
                        ),
                        (
                            "download-settings-matrix-任务栏进度-row-1",
                            "download-settings-matrix-任务栏进度-cell-1-left",
                            "download-settings-matrix-任务栏进度-seam-1",
                            "download-settings-matrix-任务栏进度-cell-1-right",
                        ),
                        (
                            "download-settings-matrix-任务栏进度-row-2",
                            "download-settings-matrix-任务栏进度-cell-2-left",
                            "download-settings-matrix-任务栏进度-seam-2",
                            "download-settings-matrix-任务栏进度-cell-2-right",
                        ),
                        (
                            "download-settings-matrix-任务栏进度-row-3",
                            "download-settings-matrix-任务栏进度-cell-3-left",
                            "download-settings-matrix-任务栏进度-seam-3",
                            "download-settings-matrix-任务栏进度-cell-3-right",
                        ),
                        (
                            "download-settings-matrix-任务栏进度-row-4",
                            "download-settings-matrix-任务栏进度-cell-4-left",
                            "download-settings-matrix-任务栏进度-seam-4",
                            "download-settings-matrix-任务栏进度-cell-4-right",
                        ),
                        (
                            "download-settings-matrix-任务栏进度-row-5",
                            "download-settings-matrix-任务栏进度-cell-5-left",
                            "download-settings-matrix-任务栏进度-seam-5",
                            "download-settings-matrix-任务栏进度-cell-5-right",
                        ),
                        (
                            "download-settings-matrix-任务栏进度-row-6",
                            "download-settings-matrix-任务栏进度-cell-6-left",
                            "download-settings-matrix-任务栏进度-seam-6",
                            "download-settings-matrix-任务栏进度-cell-6-right",
                        ),
                    ][row_index],
                    _ => unreachable!(),
                };
                let row = window
                    .debug_bounds(row_selector)
                    .expect("window matrix row");
                let left = window
                    .debug_bounds(left_selector)
                    .expect("window matrix left cell");
                let seam = window
                    .debug_bounds(seam_selector)
                    .expect("window matrix seam");
                let right = window
                    .debug_bounds(right_selector)
                    .expect("window matrix right cell");
                assert_eq!(left.origin.y, right.origin.y);
                assert_eq!(left.size.height, right.size.height);
                assert!((left.size.width.as_f32() - right.size.width.as_f32()).abs() <= 2.0);
                assert!((seam.size.width.as_f32() - 1.0).abs() <= 0.5);
                assert!(section.contains(&row.origin));
                assert!(section.contains(&row.bottom_right()));
            }
        }
        let task_window_left_edges = [
            "download-settings-progress-window",
            "download-settings-completion-sound",
        ]
        .map(|selector| {
            let bounds = window.debug_bounds(selector).expect("task-window control");
            bounds.origin.x.as_f32() + bounds.size.width.as_f32()
        });
        let task_window_right_edges = [
            "download-settings-completion-window",
            "download-settings-completion-directory",
        ]
        .map(|selector| {
            let bounds = window.debug_bounds(selector).expect("task-window control");
            bounds.origin.x.as_f32() + bounds.size.width.as_f32()
        });
        let taskbar_left_edges = [
            "download-settings-taskbar-overlay",
            "download-settings-taskbar-anchor",
            "download-settings-taskbar-x-field",
            "download-settings-taskbar-width-field",
            "download-settings-taskbar-show-progress",
            "download-settings-taskbar-show-remaining",
            "download-settings-taskbar-show-tasks",
        ]
        .map(|selector| {
            let bounds = window.debug_bounds(selector).expect("taskbar left control");
            bounds.origin.x.as_f32() + bounds.size.width.as_f32()
        });
        let taskbar_right_edges = [
            "download-settings-taskbar-edit",
            "download-settings-taskbar-progress-mode",
            "download-settings-taskbar-y-field",
            "download-settings-taskbar-height-field",
            "download-settings-taskbar-show-speed",
            "download-settings-taskbar-show-bytes",
            "download-settings-taskbar-show-connections",
        ]
        .map(|selector| {
            let bounds = window
                .debug_bounds(selector)
                .expect("taskbar right control");
            bounds.origin.x.as_f32() + bounds.size.width.as_f32()
        });
        for edges in [
            task_window_left_edges.as_slice(),
            task_window_right_edges.as_slice(),
            taskbar_left_edges.as_slice(),
            taskbar_right_edges.as_slice(),
        ] {
            assert!(
                edges
                    .windows(2)
                    .all(|pair| (pair[0] - pair[1]).abs() <= 1.0),
                "window and taskbar controls must align to stable trailing rails"
            );
        }
        open_page!("download-settings-nav-diagnostics");
        assert_section!("诊断", "download-settings-section-诊断");

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
                    actions.contains(&action.origin) && actions.contains(&action.bottom_right())
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
                                error: (!success)
                                    .then(|| "sensitive path must not be logged".into()),
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
        assert!(cx.read(|cx| {
            notifications
                .read(cx)
                .items()
                .iter()
                .all(|item| item.title.as_ref() != "路径已复制")
        }));

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

    #[gpui::test]
    fn category_settings_use_a_readable_list_and_a_labeled_full_width_directory_editor(
        cx: &mut TestAppContext,
    ) {
        cx.update(gpui_component::init);
        let executor = cx.executor();
        let (application, directory) =
            download_application_for_window_test(cx, "category-settings-layout");
        let config_path = directory.join("config.json");
        let persisted_before_actions = AppConfig::default();
        persisted_before_actions
            .save_to(&config_path)
            .expect("save category action persistence fixture");
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
        let first_marker = window
            .debug_bounds("download-settings-category-0-marker")
            .expect("selected category edge");
        assert_eq!(first_marker.size.width, px(2.0));
        assert!(category_master.contains(&category_item.origin));
        assert!(category_master.contains(&category_item.bottom_right()));
        assert!(category_item.contains(&first_marker.origin));
        assert!(category_item.contains(&first_marker.bottom_right()));

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
            .debug_bounds("download-settings-category-name-cell")
            .expect("category name cell");
        let directory_cell = window
            .debug_bounds("download-settings-category-directory-cell")
            .expect("category directory cell");
        let extensions_cell = window
            .debug_bounds("download-settings-category-extensions-cell")
            .expect("category extensions cell");
        let url_cell = window
            .debug_bounds("download-settings-category-url-cell")
            .expect("category URL cell");
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
        for (cell, description_selector) in [
            (
                directory_cell,
                "download-settings-category-directory-field-description",
            ),
            (
                extensions_cell,
                "download-settings-category-extensions-field-description",
            ),
            (url_cell, "download-settings-category-url-field-description"),
        ] {
            let description = window
                .debug_bounds(description_selector)
                .unwrap_or_else(|| panic!("missing {description_selector}"));
            assert!(cell.contains(&description.origin));
            assert!(cell.contains(&description.bottom_right()));
        }

        let initial_presentation_revision =
            cx.read(|cx| settings_view.read(cx).category_presentation_revision);
        let presentation_root = r"C:\Users\Administrator\Downloads\NxTools-Categories";
        let long_relative_directory =
            r"Compressed\A-Long-Readable-Category-Destination-For-Tooltip";
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
                input.set_value(presentation_root, window, cx);
            });
            category_name_input.update(cx, |input, cx| {
                input.set_value("压缩与归档文件", window, cx);
            });
            category_directory_input.update(cx, |input, cx| {
                input.set_value(long_relative_directory, window, cx);
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
        window.simulate_mouse_move(edited_category_item.center(), None, Modifiers::default());
        executor.advance_clock(Duration::from_millis(600));
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
        assert!(category_heading.contains(&enabled_toggle.bottom_right()));

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
                px(crate::ui::metrics::CONTROL_HEIGHT_ACTION),
                "category tools must use the shared 28px action tier"
            );
            assert!(action_toolbar.contains(&action.origin));
            assert!(action_toolbar.contains(&action.bottom_right()));
        }
        assert_eq!(move_up_button.origin.y, move_down_button.origin.y);
        assert!(order_group.contains(&move_up_button.origin));
        assert!(order_group.contains(&move_up_button.bottom_right()));
        assert!(order_group.contains(&move_down_button.origin));
        assert!(order_group.contains(&move_down_button.bottom_right()));
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
        assert!(disabled_row.contains(&disabled_title.origin));
        assert!(disabled_row.contains(&disabled_title.bottom_right()));
        assert!(disabled_row.contains(&disabled_detail.origin));
        assert!(disabled_row.contains(&disabled_detail.bottom_right()));

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
        assert!(
            window
                .debug_bounds("download-settings-category-1-marker")
                .is_some(),
            "the clicked master row must own the selected edge"
        );
        assert!(
            window
                .debug_bounds("download-settings-category-directory-field")
                .is_some(),
            "switching categories must keep the selected category's directory editor visible"
        );

        let move_down = window
            .debug_bounds("download-settings-category-down-control")
            .expect("category move-down action");
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
                px(crate::ui::metrics::CONTROL_HEIGHT_ACTION),
                "{name} must use the shared action tier"
            );
        }
        assert_eq!(
            window
                .debug_bounds("download-settings-category-enabled-button")
                .expect("category enabled toggle")
                .size
                .height,
            px(crate::ui::metrics::CONTROL_HEIGHT_DEFAULT),
            "settings toggles keep their compact shared tier"
        );

        let directory_picker = window
            .debug_bounds("download-settings-pick-directory")
            .expect("directory picker button");
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

        let transfer_navigation = window
            .debug_bounds("download-settings-nav-transfer")
            .expect("transfer navigation");
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
                px(crate::ui::metrics::CONTROL_HEIGHT_ACTION),
                "{name} must use the shared action tier"
            );
        }
        let ffmpeg_picker = window
            .debug_bounds("download-settings-pick-ffmpeg-button")
            .expect("FFmpeg picker button");
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

    #[gpui::test]
    fn settings_auto_download_cancel_click_reaches_the_operation_control(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);
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
                application.ffmpeg_install_operation_override =
                    Some(FfmpegInstallOperationOverride {
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
        let transfer_navigation = window
            .debug_bounds("download-settings-nav-transfer")
            .expect("transfer settings navigation");
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

    #[gpui::test]
    fn settings_window_routes_picker_applies_and_cancels_without_cross_module_loss(
        cx: &mut TestAppContext,
    ) {
        cx.update(gpui_component::init);
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
        assert!(window.debug_bounds("download-settings-scroll").is_some());
        window.update(|window, cx| {
            settings.update(cx, |settings, cx| {
                settings.default_request_headers.update(cx, |input, cx| {
                    input.set_value(
                        "X-NxTools-Default: desktop\nReferer: https://example.test/",
                        window,
                        cx,
                    );
                });
                settings.default_cookie.update(cx, |input, cx| {
                    input.set_value("session=desktop", window, cx);
                });
            });
        });
        let request_navigation = window
            .debug_bounds("download-settings-nav-request")
            .expect("request settings navigation");
        window.simulate_click(request_navigation.center(), Modifiers::default());
        window.run_until_parked();
        let certificate_toggle = window
            .debug_bounds("download-settings-accept-invalid-certificates")
            .expect("certificate policy toggle");
        window.simulate_click(certificate_toggle.center(), Modifiers::default());
        let general_navigation = window
            .debug_bounds("download-settings-nav-general")
            .expect("general settings navigation");
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
                    .update(cx, |input, cx| input.set_value("镜像包", window, cx));
                category
                    .save_directory
                    .update(cx, |input, cx| input.set_value("Mirrors", window, cx));
                category
                    .extensions
                    .update(cx, |input, cx| input.set_value("img, iso", window, cx));
                category.url_patterns.update(cx, |input, cx| {
                    input.set_value("https://mirror.example/*", window, cx)
                });
            });
        });
        let original_directory =
            window.read(|cx| settings.read(cx).directory.read(cx).value().to_string());
        window.update(|_, cx| {
            application.update(cx, |_, cx| {
                cx.emit(DownloadApplicationEvent::DirectoryPicked {
                    target: DownloadDirectoryTarget::NewTask,
                    path: directory.join("wrong-target"),
                });
            });
        });
        window.run_until_parked();
        assert_eq!(
            window.read(|cx| settings.read(cx).directory.read(cx).value().to_string()),
            original_directory,
            "a new-task picker result must not mutate the settings draft"
        );
        window.update(|_, cx| {
            application.update(cx, |_, cx| {
                cx.emit(DownloadApplicationEvent::DirectoryPicked {
                    target: DownloadDirectoryTarget::Settings,
                    path: selected_directory.clone(),
                });
            });
        });
        window.run_until_parked();
        assert_eq!(
            window.read(|cx| settings.read(cx).directory.read(cx).value().to_string()),
            selected_directory.display().to_string()
        );
        let transfer_navigation = window
            .debug_bounds("download-settings-nav-transfer")
            .expect("transfer settings navigation");
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
        let window_navigation = window
            .debug_bounds("download-settings-nav-window")
            .expect("window settings navigation");
        window.simulate_click(window_navigation.center(), Modifiers::default());
        window.run_until_parked();
        window.update(|_, cx| {
            settings.update(cx, |settings, cx| {
                settings.scroll.scroll_to_bottom();
                cx.notify();
            });
        });
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
        let window_navigation = cancel_window
            .debug_bounds("download-settings-nav-window")
            .expect("window settings navigation");
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

    #[gpui::test]
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
            .split("    pub(crate) fn submit(")
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
        let deferred_open = view_submit[close..]
            .find("cx.defer(move |cx|")
            .expect("task window opening is scheduled after source close");
        assert!(deferred_open > 0);
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
        assert!(close.is_some(), "dialog submission closes the source window");
        assert!(
            stash < close,
            "the dialog position must be stashed before the dialog is destroyed"
        );
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
            (440.0, 180.0)
        );
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
        // The user report: the second-level window had no way to inspect the
        // dynamic split state. The progress view must carry an expand toggle,
        // a details panel and old-NxTools-style per-segment dots, and the
        // thread metric must follow the live segment count instead of the
        // static connection budget.
        let source = include_str!("../downloads/task_window.rs");
        for marker in [
            "download-window-details-toggle",
            "download-window-details",
            "download-window-segment-dots",
            "TASK_WINDOW_PROGRESS_EXPANDED_HEIGHT",
            "\"分段\"",
        ] {
            assert!(
                source.contains(marker),
                "progress window render must contain {marker}"
            );
        }
    }

    #[test]
    fn segment_dot_ratio_bounds_each_segment_progress() {
        let segment = |downloaded: u64, total: Option<u64>| {
            nxtools_download::DownloadSegmentProgress {
                segment_index: 0,
                downloaded_bytes: downloaded,
                total_bytes: total,
            }
        };
        assert_eq!(segment_dot_ratio(&segment(0, Some(100))), 0.0);
        assert_eq!(segment_dot_ratio(&segment(50, Some(100))), 0.5);
        assert_eq!(segment_dot_ratio(&segment(150, Some(100))), 1.0);
        assert_eq!(segment_dot_ratio(&segment(10, None)), 1.0);
        assert_eq!(segment_dot_ratio(&segment(0, None)), 0.0);
        assert_eq!(segment_dot_ratio(&segment(0, Some(0))), 0.0);
        assert_eq!(
            segment_progress_summary(&[]),
            "尚未分段",
            "an unsegmented transfer explains its single-connection mode"
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
        let mut replacements =
            HashMap::from([(stale_task, stale_window), (other_task, other_window)]);

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
            (560.0, 278.0)
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
        let helper_marker = ["fn download_window_", "close_button"].concat();
        let close_button = source
            .split(&helper_marker)
            .nth(1)
            .and_then(|body| body.split("\npub(crate) fn download_task_status").next())
            .expect("shared download close button");

        assert!(
            close_button.contains(".on_click(|_, window, _| window.remove_window())"),
            "custom client titlebars must close explicitly instead of relying only on native hit testing"
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
