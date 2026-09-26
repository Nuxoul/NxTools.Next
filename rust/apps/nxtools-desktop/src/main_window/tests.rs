//! Tests for the parent module. Extracted from the parent file so the
//! runtime code stays readable.

use std::cell::{Cell, RefCell};
use std::collections::HashSet;
use std::rc::Rc;
use std::time::{Duration, Instant};

use gpui::{
    AppContext, KeyDownEvent, Keystroke, Modifiers, MouseButton, MouseDownEvent, MouseUpEvent,
    TestAppContext, VisualTestContext, px, size,
};
use gpui_component::PixelsExt;
use nxtools_core::{
    AppConfig, ClipboardHotKeyModifier, CustomCommandSettings,
    DownloadRowDoubleClickActionSettings, DownloadSettings, LinkPanelPosition, SystemSettings,
};
use nxtools_download::{
    DownloadEngine, DownloadEngineTask, DownloadFault, DownloadFaultCode, DownloadProgress,
    DownloadRequest, DownloadStatus, DownloadTask, DownloadTaskId, RequestOrigin,
};
use nxtools_finder::FinderCatalog;
use nxtools_network::NetworkPolicy;

use nxtools_ipc::{PlatformNotification, platform_bridge};
use nxtools_protocol::{
    ActivateWindowCommand, IndexDriveStatus, IndexPhase, IndexStatus, IndexerCommand, IndexerEvent,
    Message, PlatformCommand, PlatformCommandResult,
};

use crate::clipboard::ClipboardApplication;
use crate::downloads::{DownloadApplication, DownloadRuntimeContext};
use crate::network_settings::NetworkSettingsApplication;
use crate::notifications::{NotificationAction, NotificationCenter, NotificationDetail};
use crate::quick_launch::QuickLaunchApplication;
use crate::runtime::IndexerNotification;
use crate::shell::MainModule;

fn assert_single_index_settings_command(
    endpoint: &crate::runtime::IndexerProcessEndpoint,
    expected_volumes: &[&str],
    expected_exclusions: &[&str],
) {
    let envelope = endpoint
        .try_recv_command_for_test()
        .expect("one index settings command");
    let Message::IndexerCommand(IndexerCommand::UpdateSettings(settings)) = envelope.message else {
        panic!("volume click must emit one typed index settings command");
    };
    assert_eq!(
        settings.indexed_volumes,
        expected_volumes
            .iter()
            .map(|mount| (*mount).to_owned())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        settings.excluded_paths,
        expected_exclusions
            .iter()
            .map(|path| (*path).to_owned())
            .collect::<Vec<_>>()
    );
    assert!(
        endpoint.try_recv_command_for_test().is_none(),
        "one pointer gesture must not persist and synchronize twice"
    );
}

#[test]
fn download_failure_summary_localizes_typed_errors_without_exposing_engine_ids() {
    let resource_changed = DownloadFault {
            code: DownloadFaultCode::Protocol,
            message: "download resource changed while task DownloadTaskId(d3660bd8-3e06-480e-b738-d783e303165e) was active".into(),
            retryable: false,
        };
    assert_eq!(
        super::download_failure_summary(&resource_changed),
        "远程文件已变化，请重新下载"
    );

    let ignored_range = DownloadFault {
        code: DownloadFaultCode::Protocol,
        message: "server ignored the requested byte range and returned HTTP 200".into(),
        retryable: false,
    };
    assert_eq!(
        super::download_failure_summary(&ignored_range),
        "服务器不支持断点续传，请重新开始"
    );
    assert!(!super::download_failure_summary(&resource_changed).contains("DownloadTaskId"));
}

#[test]
fn download_failure_summary_covers_every_typed_category() {
    let expected = [
        (
            DownloadFaultCode::InvalidRequest,
            "下载请求无效，请检查地址",
        ),
        (DownloadFaultCode::NotFound, "下载任务或文件不存在"),
        (DownloadFaultCode::InvalidState, "当前状态无法执行此操作"),
        (DownloadFaultCode::Network, "网络连接失败，请检查网络后重试"),
        (DownloadFaultCode::Protocol, "服务器响应异常，请重试"),
        (
            DownloadFaultCode::Storage,
            "无法写入下载文件，请检查保存位置",
        ),
        (DownloadFaultCode::Permission, "没有权限写入保存位置"),
        (DownloadFaultCode::DiskFull, "磁盘空间不足"),
        (DownloadFaultCode::FileInUse, "目标文件正在被其他程序使用"),
        (DownloadFaultCode::Cancelled, "下载已取消"),
        (
            DownloadFaultCode::Unavailable,
            "下载服务暂时不可用，请稍后重试",
        ),
        (DownloadFaultCode::Internal, "下载发生内部错误，请重试"),
    ];
    for (code, summary) in expected {
        let error = DownloadFault {
            code,
            message: "internal engine detail".into(),
            retryable: false,
        };
        assert_eq!(super::download_failure_summary(&error), summary);
    }
}

#[test]
fn download_row_transfer_uses_live_values_only_for_active_tasks() {
    let task_id = DownloadTaskId::new();
    let progress = DownloadProgress {
        task_id,
        downloaded_bytes: 25 * 1024 * 1024,
        total_bytes: Some(100 * 1024 * 1024),
        bytes_per_second: 5 * 1024 * 1024,
        active_connections: 4,
        segments: Vec::new(),
    };
    let active = super::download_row_transfer_presentation(
        DownloadStatus::Downloading,
        20 * 1024 * 1024,
        Some(100 * 1024 * 1024),
        Some(&progress),
    );
    assert_eq!(active.fraction, 0.25);
    assert_eq!(active.transferred, "25/100 MB");
    assert_eq!(active.speed, "5 MB/s");
    assert_eq!(active.remaining, "00:15");

    let completed = super::download_row_transfer_presentation(
        DownloadStatus::Completed,
        100 * 1024 * 1024,
        Some(100 * 1024 * 1024),
        Some(&progress),
    );
    assert_eq!(completed.fraction, 1.0);
    assert_eq!(completed.transferred, "100/100 MB");
    assert_eq!(completed.speed, "—");
    assert_eq!(completed.remaining, "—");

    assert_eq!(super::format_download_rate(1023 * 1024 * 1024), "1023 MB/s");
    assert_eq!(super::format_download_rate(1024 * 1024 * 1024), "1 GB/s");
    assert_eq!(super::format_download_remaining(3_600), "1h00m");
    assert_eq!(super::format_download_remaining(359_999), "99h59m");
    assert_eq!(super::format_download_remaining(360_000), "99h+");
}

#[gpui::test]
fn titlebar_global_settings_click_keeps_its_controller_alive(cx: &mut TestAppContext) {
    cx.update(gpui_component::init);
    let notifications = cx.update(|cx| cx.new(|_| NotificationCenter::default()));
    let config = std::rc::Rc::new(std::cell::RefCell::new(AppConfig::default()));
    let config_path = std::env::temp_dir().join(format!(
        "nxtools-global-settings-{}.json",
        uuid::Uuid::new_v4()
    ));
    let network_settings = cx.update(|cx| {
        cx.new(|_| {
            NetworkSettingsApplication::new(
                NetworkPolicy::new(config.borrow().network.clone()).expect("create network policy"),
                config.borrow().clone(),
                config_path,
                notifications.clone(),
                None,
                Rc::new(RefCell::new(SystemSettings::default())),
                false,
            )
        })
    });
    let settings_observer = network_settings.downgrade();
    let settings_for_window = Some(network_settings.clone());
    let (_main_window, window) = cx.add_window_view(move |window, cx| {
        MainWindow::new(
            gpui::WeakEntity::<ClipboardApplication>::new_invalid(),
            gpui::WeakEntity::<QuickLaunchApplication>::new_invalid(),
            gpui::WeakEntity::<DownloadApplication>::new_invalid(),
            MainWindowShellContext {
                settings: settings_for_window,
                platform: None,
                system_settings: Rc::new(RefCell::new(SystemSettings::default())),
            },
            notifications,
            window,
            cx,
        )
    });
    drop(network_settings);

    window.refresh().expect("render main title bar");
    assert!(
        window.debug_bounds("main-titlebar").is_some(),
        "main title bar must render"
    );
    let settings = window
        .debug_bounds("global-network-settings")
        .expect("global settings trigger");
    window.simulate_click(settings.center(), Modifiers::default());
    window.run_until_parked();

    assert!(
        settings_observer
            .update(cx, |application, _| application.window_is_open())
            .expect("main window must strongly own global settings")
    );
}

#[gpui::test]
fn module_switching_keeps_one_compact_header_and_sidebar_rhythm(cx: &mut TestAppContext) {
    cx.update(gpui_component::init);
    let notifications = cx.update(|cx| cx.new(|_| NotificationCenter::default()));
    let (_main_window, window) = cx.add_window_view(move |window, cx| {
        window.resize(gpui::size(px(1060.0), px(500.0)));
        MainWindow::new(
            gpui::WeakEntity::<ClipboardApplication>::new_invalid(),
            gpui::WeakEntity::<QuickLaunchApplication>::new_invalid(),
            gpui::WeakEntity::<DownloadApplication>::new_invalid(),
            MainWindowShellContext {
                settings: None,
                platform: None,
                system_settings: Rc::new(RefCell::new(SystemSettings::default())),
            },
            notifications,
            window,
            cx,
        )
    });
    window.refresh().expect("render download workspace");

    let sidebar = window
        .debug_bounds("main-sidebar")
        .expect("rendered production sidebar");
    assert_eq!(sidebar.size.width, px(SIDEBAR_WIDTH));
    for selector in ["main-module-下载", "main-module-速启", "main-module-剪贴板"] {
        let row = window
            .debug_bounds(selector)
            .unwrap_or_else(|| panic!("rendered sidebar row {selector}"));
        assert_eq!(row.size.height, px(SIDEBAR_ROW_HEIGHT));
    }
    for module in ["下载", "速启", "剪贴板"] {
        let title_selector: &'static str =
            Box::leak(format!("main-module-{module}-title").into_boxed_str());
        let description_selector: &'static str =
            Box::leak(format!("main-module-{module}-description").into_boxed_str());
        assert_eq!(
            window
                .debug_bounds(title_selector)
                .expect("sidebar module title")
                .size
                .height,
            px(LINE_HEIGHT_BODY)
        );
        assert_eq!(
            window
                .debug_bounds(description_selector)
                .expect("sidebar module description")
                .size
                .height,
            px(LINE_HEIGHT_LABEL)
        );
    }

    let download_header = window
        .debug_bounds("download-module-header")
        .expect("download module header");
    assert_eq!(download_header.size.height, px(48.0));
    assert_eq!(
        window
            .debug_bounds("download-module-header-title")
            .expect("download module title")
            .size
            .height,
        px(20.0)
    );
    assert_eq!(
        window
            .debug_bounds("download-module-header-metadata")
            .expect("download module metadata")
            .size
            .height,
        px(LINE_HEIGHT_LABEL)
    );
    let search = window
        .debug_bounds("download-search")
        .expect("download header search");
    let settings = window
        .debug_bounds("download-settings-action")
        .expect("download header settings");
    assert_eq!(settings.left() - search.right(), px(SPACING_4));
    let download_empty = window
        .debug_bounds("download-empty-state")
        .expect("download work-surface empty state");
    assert_eq!(
        download_empty.size.height,
        px(crate::ui::components::state::WORK_STATE_MIN_HEIGHT)
    );
    let download_empty_icon = window
        .debug_bounds("download-empty-state-icon")
        .expect("download empty icon slot");
    let download_empty_title = window
        .debug_bounds("download-empty-state-title")
        .expect("download empty title");
    let download_empty_description = window
        .debug_bounds("download-empty-state-description")
        .expect("download empty description");
    assert_eq!(download_empty_icon.size, size(px(18.0), px(18.0)));
    assert_eq!(download_empty_title.size.height, px(LINE_HEIGHT_BODY));
    assert_eq!(
        download_empty_description.size.height,
        px(LINE_HEIGHT_LABEL)
    );
    assert_eq!(
        download_empty_title.left(),
        download_empty_description.left()
    );
    assert!(download_empty.left() >= sidebar.right());

    let quick_switch = window
        .debug_bounds("main-module-速启")
        .expect("quick-launch sidebar row");
    window.simulate_click(quick_switch.center(), Modifiers::default());
    window.refresh().expect("render quick-launch workspace");
    let quick_header = window
        .debug_bounds("quick-launch-module-header")
        .expect("quick-launch module header");
    assert_eq!(quick_header.size.height, download_header.size.height);
    assert_eq!(quick_header.origin.y, download_header.origin.y);
    assert_eq!(
        window
            .debug_bounds("quick-launch-module-header-title")
            .expect("quick-launch module title")
            .size
            .height,
        px(20.0)
    );
    assert_eq!(
        window
            .debug_bounds("quick-launch-module-header-metadata")
            .expect("quick-launch module metadata")
            .size
            .height,
        px(LINE_HEIGHT_LABEL)
    );

    let clipboard_switch = window
        .debug_bounds("main-module-剪贴板")
        .expect("clipboard sidebar row");
    window.simulate_click(clipboard_switch.center(), Modifiers::default());
    window.refresh().expect("render clipboard workspace");
    let clipboard_header = window
        .debug_bounds("clipboard-module-header")
        .expect("clipboard module header");
    assert_eq!(clipboard_header.size.height, download_header.size.height);
    assert_eq!(clipboard_header.origin.y, download_header.origin.y);
    assert_eq!(
        window
            .debug_bounds("clipboard-module-header-title")
            .expect("clipboard module title")
            .size
            .height,
        px(20.0)
    );
    assert_eq!(
        window
            .debug_bounds("clipboard-module-header-metadata")
            .expect("clipboard module metadata")
            .size
            .height,
        px(LINE_HEIGHT_LABEL)
    );
}

#[gpui::test]
fn clipboard_settings_keep_one_continuous_primary_secondary_surface(cx: &mut TestAppContext) {
    cx.update(gpui_component::init);
    let notifications = cx.update(|cx| cx.new(|_| NotificationCenter::default()));
    let (_main_window, window) = cx.add_window_view(move |window, cx| {
        window.resize(gpui::size(px(1060.0), px(500.0)));
        let mut main_window = MainWindow::new(
            gpui::WeakEntity::<ClipboardApplication>::new_invalid(),
            gpui::WeakEntity::<QuickLaunchApplication>::new_invalid(),
            gpui::WeakEntity::<DownloadApplication>::new_invalid(),
            MainWindowShellContext {
                settings: None,
                platform: None,
                system_settings: Rc::new(RefCell::new(SystemSettings::default())),
            },
            notifications,
            window,
            cx,
        );
        main_window.shell.activate(MainModule::Clipboard);
        main_window
    });

    window
        .refresh()
        .expect("render clipboard settings workspace");
    let workspace = window
        .debug_bounds("clipboard-settings-workspace")
        .expect("clipboard settings workspace");
    let surface = window
        .debug_bounds("clipboard-settings-split-surface")
        .expect("one continuous clipboard settings surface");
    let primary = window
        .debug_bounds("clipboard-settings-primary-column")
        .expect("primary clipboard settings column");
    let divider = window
        .debug_bounds("clipboard-settings-column-divider")
        .expect("single clipboard settings column divider");
    let secondary = window
        .debug_bounds("clipboard-settings-secondary-column")
        .expect("secondary clipboard settings column");

    assert_eq!(surface.left() - workspace.left(), px(12.0));
    assert_eq!(workspace.right() - surface.right(), px(12.0));
    assert_eq!(surface.top() - workspace.top(), px(12.0));
    assert_eq!(primary.left() - surface.left(), px(1.0));
    assert_eq!(primary.right(), divider.left());
    assert_eq!(divider.size.width, px(1.0));
    assert_eq!(divider.right(), secondary.left());
    assert_eq!(surface.right() - secondary.right(), px(1.0));
    assert_eq!(secondary.size.width, px(410.0));
    assert!(primary.size.width > secondary.size.width);
    assert_eq!(primary.top(), secondary.top());
    assert_eq!(primary.bottom(), secondary.bottom());
}

#[gpui::test]
fn quick_launch_settings_navigation_mounts_only_the_selected_page(cx: &mut TestAppContext) {
    cx.update(gpui_component::init);
    let executor = cx.executor();
    let notifications = cx.update(|cx| cx.new(|_| NotificationCenter::default()));
    let temp = tempfile::tempdir().expect("temporary Quick Launch settings fixture");
    let catalog = FinderCatalog::open(temp.path().join("history.db"), &[])
        .expect("open Quick Launch settings catalog");
    let config_path = temp.path().join("config.json");
    let log_directory = temp.path().join("logs");
    let mut application_config = AppConfig::default();
    application_config.quick_launch.excluded_paths = (0..20)
        .map(|index| format!(r"C:\Build\Generated\{index}"))
        .collect();
    let quick_launch = cx.update(|cx| {
        let (platform, _platform_endpoint) = platform_bridge(8);
        let (indexer, _indexer_endpoint) = crate::runtime::indexer_bridge(8);
        cx.new(|_| {
            QuickLaunchApplication::new(
                catalog,
                Rc::new(platform),
                Rc::new(indexer),
                application_config,
                config_path,
                log_directory,
                notifications.clone(),
            )
        })
    });
    let settings_before_navigation = cx.read(|cx| quick_launch.read(cx).settings());
    assert_eq!(settings_before_navigation.excluded_paths.len(), 20);
    let quick_launch_for_window = quick_launch.downgrade();
    let notifications_for_window = notifications.clone();
    let (main_window, window) = cx.add_window_view(move |window, cx| {
        window.resize(gpui::size(px(1060.0), px(500.0)));
        let mut main_window = MainWindow::new(
            gpui::WeakEntity::<ClipboardApplication>::new_invalid(),
            quick_launch_for_window,
            gpui::WeakEntity::<DownloadApplication>::new_invalid(),
            MainWindowShellContext {
                settings: None,
                platform: None,
                system_settings: Rc::new(RefCell::new(SystemSettings::default())),
            },
            notifications_for_window,
            window,
            cx,
        );
        main_window.shell.activate(MainModule::QuickFind);
        main_window
    });

    window.refresh().expect("render Quick Launch settings");
    let navigation = window
        .debug_bounds("quick-launch-settings-navigation")
        .expect("stable Quick Launch settings navigation");
    assert_eq!(navigation.size.width, px(160.0));
    for selector in [
        "quick-launch-settings-nav-general",
        "quick-launch-settings-nav-index",
        "quick-launch-settings-nav-context",
        "quick-launch-settings-nav-commands",
        "quick-launch-settings-nav-diagnostics",
    ] {
        assert_eq!(
            window
                .debug_bounds(selector)
                .unwrap_or_else(|| panic!("missing {selector}"))
                .size
                .height,
            px(40.0)
        );
    }
    assert!(
        window
            .debug_bounds("quick-launch-settings-page-general")
            .is_some()
    );
    assert!(
        window
            .debug_bounds("quick-launch-settings-page-index")
            .is_none()
    );
    assert_eq!(
        window
            .debug_bounds("quick-launch-settings-page-header")
            .expect("fixed Quick Launch settings page header")
            .size
            .height,
        px(48.0)
    );
    assert_eq!(
        window
            .debug_bounds("quick-launch-settings-page-title")
            .expect("Quick Launch settings page title")
            .size
            .height,
        px(LINE_HEIGHT_TITLE)
    );
    assert_eq!(
        window
            .debug_bounds("quick-launch-settings-page-description")
            .expect("Quick Launch settings page description")
            .size
            .height,
        px(LINE_HEIGHT_LABEL)
    );

    let index_navigation = window
        .debug_bounds("quick-launch-settings-nav-index")
        .expect("Index navigation row");
    window.simulate_click(index_navigation.center(), Modifiers::default());
    window.run_until_parked();
    assert_eq!(
        window.read(|cx| main_window.read(cx).quick_launch_settings_page),
        QuickLaunchSettingsPage::Index
    );
    window.refresh().expect("render Index settings page");
    assert!(
        window
            .debug_bounds("quick-launch-settings-page-index")
            .is_some()
    );
    let max_offset = window.read(|cx| main_window.read(cx).quick_find_scroll.max_offset().height);
    assert!(
        max_offset > px(0.0),
        "the Index settings page must be vertically scrollable"
    );
    window.update(|_, cx| {
        main_window.update(cx, |main_window, _| {
            main_window
                .quick_find_scroll
                .set_offset(gpui::point(px(0.0), -max_offset));
        });
    });
    window.run_until_parked();
    window
        .refresh()
        .expect("render Index settings at the bottom");
    let settings_scroll = window
        .debug_bounds("quick-launch-settings-scroll")
        .expect("Quick Launch settings scroll viewport");
    let last_exclusion = window
        .debug_bounds("quick-launch-exclusion-path-19")
        .expect("last exclusion path at scroll bottom");
    let last_delete = window
        .debug_bounds("quick-launch-exclusion-delete-19")
        .expect("last exclusion delete action at scroll bottom");
    assert!(last_exclusion.top() >= settings_scroll.top());
    assert!(last_exclusion.bottom() <= settings_scroll.bottom());
    assert!(last_delete.bottom() <= settings_scroll.bottom());

    window.update(|_, cx| {
        main_window.update(cx, |main_window, cx| {
            main_window.animate_quick_find_scroll_to(-38.0, cx);
        });
    });

    let scrolling_generation = window.read(|cx| {
        let main_window = main_window.read(cx);
        assert!(main_window.quick_find_scroll_target < 0.0);
        main_window.quick_find_scroll_animation
    });

    let context_navigation = window
        .debug_bounds("quick-launch-settings-nav-context")
        .expect("Context navigation row");
    window.simulate_click(context_navigation.center(), Modifiers::default());
    window.run_until_parked();
    assert_eq!(
        window.read(|cx| main_window.read(cx).quick_launch_settings_page),
        QuickLaunchSettingsPage::Context
    );
    assert!(
        window.read(|cx| main_window.read(cx).quick_find_scroll_animation) > scrolling_generation
    );
    executor.advance_clock(Duration::from_millis(250));
    executor.run_until_parked();
    window.read(|cx| {
        let main_window = main_window.read(cx);
        assert_eq!(main_window.quick_find_scroll_target, 0.0);
        assert_eq!(main_window.quick_find_scroll.offset().y, px(0.0));
    });
    window.refresh().expect("render Context settings page");
    assert!(
        window
            .debug_bounds("quick-launch-settings-page-context")
            .is_some()
    );

    for (navigation_selector, page, content_selector) in [
        (
            "quick-launch-settings-nav-commands",
            QuickLaunchSettingsPage::Commands,
            "quick-launch-settings-page-commands",
        ),
        (
            "quick-launch-settings-nav-diagnostics",
            QuickLaunchSettingsPage::Diagnostics,
            "quick-launch-settings-page-diagnostics",
        ),
        (
            "quick-launch-settings-nav-general",
            QuickLaunchSettingsPage::General,
            "quick-launch-settings-page-general",
        ),
        (
            "quick-launch-settings-nav-index",
            QuickLaunchSettingsPage::Index,
            "quick-launch-settings-page-index",
        ),
    ] {
        let navigation = window
            .debug_bounds(navigation_selector)
            .unwrap_or_else(|| panic!("missing {navigation_selector}"));
        window.simulate_click(navigation.center(), Modifiers::default());
        window.run_until_parked();
        assert_eq!(
            window.read(|cx| main_window.read(cx).quick_launch_settings_page),
            page
        );
        window.refresh().expect("render selected settings page");
        assert!(
            window.debug_bounds(content_selector).is_some(),
            "missing {content_selector}"
        );
    }
    assert_eq!(
        cx.read(|cx| quick_launch.read(cx).settings()),
        settings_before_navigation,
        "settings navigation must not mutate Quick Launch settings"
    );
}

#[gpui::test]
fn every_quick_launch_settings_frame_mounts_exactly_one_page(cx: &mut TestAppContext) {
    cx.update(gpui_component::init);
    let notifications = cx.update(|cx| cx.new(|_| NotificationCenter::default()));

    for selected_page in QuickLaunchSettingsPage::ALL {
        let notifications_for_window = notifications.clone();
        let (_main_window, window) = cx.add_window_view(move |window, cx| {
            window.resize(gpui::size(px(1060.0), px(500.0)));
            let mut main_window = MainWindow::new(
                gpui::WeakEntity::<ClipboardApplication>::new_invalid(),
                gpui::WeakEntity::<QuickLaunchApplication>::new_invalid(),
                gpui::WeakEntity::<DownloadApplication>::new_invalid(),
                MainWindowShellContext {
                    settings: None,
                    platform: None,
                    system_settings: Rc::new(RefCell::new(SystemSettings::default())),
                },
                notifications_for_window,
                window,
                cx,
            );
            main_window.shell.activate(MainModule::QuickFind);
            main_window.quick_launch_settings_page = selected_page;
            main_window
        });

        window.refresh().expect("render selected Quick Launch page");
        for candidate_page in QuickLaunchSettingsPage::ALL {
            assert_eq!(
                window
                    .debug_bounds(candidate_page.metadata().content_id)
                    .is_some(),
                candidate_page == selected_page,
                "only {} may mount in the selected-page viewport",
                selected_page.metadata().label
            );
        }
        assert_eq!(
            window
                .debug_bounds("quick-launch-diagnostics-surface")
                .is_some(),
            selected_page == QuickLaunchSettingsPage::Diagnostics,
            "the diagnostics surface may mount only on the Diagnostics page"
        );
        assert_eq!(
            window
                .debug_bounds("quick-launch-diagnostics-performance-metrics")
                .is_some(),
            selected_page == QuickLaunchSettingsPage::Diagnostics,
            "performance metrics may mount only on the Diagnostics page"
        );
    }
}

#[gpui::test]
fn quick_launch_volume_scope_uses_one_checkbox_list_and_preserves_automatic_semantics(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_component::init);
    let notifications = cx.update(|cx| cx.new(|_| NotificationCenter::default()));
    let temp = tempfile::tempdir().expect("temporary Quick Launch volume fixture");
    let catalog = FinderCatalog::open(temp.path().join("history.db"), &[])
        .expect("open Quick Launch volume catalog");
    let (platform, _platform_endpoint) = platform_bridge(8);
    let (indexer, indexer_endpoint) = crate::runtime::indexer_bridge(8);
    indexer_endpoint.connect_for_test();
    let quick_launch = cx.update(|cx| {
        cx.new(|_| {
            QuickLaunchApplication::new(
                catalog,
                Rc::new(platform),
                Rc::new(indexer),
                AppConfig::default(),
                temp.path().join("config.json"),
                temp.path().join("logs"),
                notifications.clone(),
            )
        })
    });
    quick_launch.update(cx, |application, cx| {
        application.handle_indexer_notification(
            IndexerNotification::Event(IndexerEvent::Status(IndexStatus {
                phase: IndexPhase::Ready,
                indexed_items: 3_750_000,
                drives: vec![
                    IndexDriveStatus {
                        mount_point: r"C:\".into(),
                        file_system: "NTFS".into(),
                        indexed_items: 2_500_000,
                        journal_id: Some(1),
                        next_usn: Some(10),
                    },
                    IndexDriveStatus {
                        mount_point: r"D:\".into(),
                        file_system: "NTFS".into(),
                        indexed_items: 1_250_000,
                        journal_id: Some(2),
                        next_usn: Some(20),
                    },
                    IndexDriveStatus {
                        mount_point: r"E:\".into(),
                        file_system: "NTFS".into(),
                        indexed_items: 0,
                        journal_id: None,
                        next_usn: None,
                    },
                ],
                last_error: None,
            })),
            cx,
        );
    });
    assert_single_index_settings_command(&indexer_endpoint, &[], &[]);
    let quick_launch_for_window = quick_launch.downgrade();
    let notifications_for_window = notifications.clone();
    let (_main_window, window) = cx.add_window_view(move |window, cx| {
        window.resize(size(px(1060.0), px(500.0)));
        let mut main_window = MainWindow::new(
            gpui::WeakEntity::<ClipboardApplication>::new_invalid(),
            quick_launch_for_window,
            gpui::WeakEntity::<DownloadApplication>::new_invalid(),
            MainWindowShellContext {
                settings: None,
                platform: None,
                system_settings: Rc::new(RefCell::new(SystemSettings::default())),
            },
            notifications_for_window,
            window,
            cx,
        );
        main_window.shell.activate(MainModule::QuickFind);
        main_window.quick_launch_settings_page = QuickLaunchSettingsPage::Index;
        main_window
    });

    window.refresh().expect("render automatic volume scope");
    let scope = window
        .debug_bounds("quick-launch-volume-scope")
        .expect("one volume checkbox list");
    let automatic = window
        .debug_bounds("quick-launch-volume-scope-automatic")
        .expect("automatic volume mode row");
    assert_eq!(automatic.size.height, px(48.0));
    assert!(
        window
            .debug_bounds("quick-launch-volume-scope-automatic-checked")
            .is_some()
    );
    for (drive, row_selector, checked_selector) in [
        (
            "C",
            "quick-launch-index-volume-C",
            "quick-launch-index-volume-C-checked",
        ),
        (
            "D",
            "quick-launch-index-volume-D",
            "quick-launch-index-volume-D-checked",
        ),
        (
            "E",
            "quick-launch-index-volume-E",
            "quick-launch-index-volume-E-checked",
        ),
    ] {
        let row = window
            .debug_bounds(row_selector)
            .unwrap_or_else(|| panic!("missing {drive}: volume row"));
        assert_eq!(row.size.height, px(40.0));
        assert_eq!(row.left(), automatic.left());
        assert_eq!(row.size.width, automatic.size.width);
        assert!(
            window.debug_bounds(checked_selector).is_some(),
            "automatic mode must visibly include {drive}:"
        );
    }
    assert_eq!(automatic.left() - scope.left(), px(1.0));
    assert_eq!(scope.size.width - automatic.size.width, px(2.0));
    assert_eq!(
        window
            .debug_bounds("quick-launch-index-volume-C-title")
            .expect("volume title")
            .size
            .height,
        px(LINE_HEIGHT_BODY)
    );
    assert_eq!(
        window
            .debug_bounds("quick-launch-index-volume-C-metadata")
            .expect("volume metadata")
            .size
            .height,
        px(LINE_HEIGHT_LABEL)
    );

    let c_volume = window
        .debug_bounds("quick-launch-index-volume-C")
        .expect("C: volume row");
    window.simulate_click(
        gpui::point(c_volume.left() + px(19.0), c_volume.center().y),
        Modifiers::default(),
    );
    window.run_until_parked();
    assert_eq!(
        window.update(|_, cx| quick_launch.read(cx).settings().indexed_volumes),
        vec![r"D:\".to_owned(), r"E:\".to_owned()],
        "disabling one volume must materialize an explicit selected-volume set"
    );
    assert_single_index_settings_command(&indexer_endpoint, &[r"D:\", r"E:\"], &[]);
    window.refresh().expect("render specified volume scope");
    assert!(
        window
            .debug_bounds("quick-launch-volume-scope-automatic-unchecked")
            .is_some()
    );
    assert!(
        window
            .debug_bounds("quick-launch-index-volume-C-unchecked")
            .is_some()
    );

    let d_volume = window
        .debug_bounds("quick-launch-index-volume-D")
        .expect("D: volume row");
    window.simulate_click(
        gpui::point(d_volume.right() - px(20.0), d_volume.center().y),
        Modifiers::default(),
    );
    window.run_until_parked();
    assert_eq!(
        window.update(|_, cx| quick_launch.read(cx).settings().indexed_volumes),
        vec![r"E:\".to_owned()],
        "the non-indicator row area must share the same single toggle path"
    );
    assert_single_index_settings_command(&indexer_endpoint, &[r"E:\"], &[]);
    window.refresh().expect("render final explicit volume");

    let e_volume = window
        .debug_bounds("quick-launch-index-volume-E")
        .expect("E: volume row");
    window.simulate_click(
        gpui::point(e_volume.left() + px(19.0), e_volume.center().y),
        Modifiers::default(),
    );
    window.run_until_parked();
    assert_eq!(
        window.update(|_, cx| quick_launch.read(cx).settings().indexed_volumes),
        vec![r"E:\".to_owned()],
        "the last available selected volume cannot encode an empty explicit scope"
    );
    assert!(
        indexer_endpoint.try_recv_command_for_test().is_none(),
        "a rejected last-volume toggle must not persist or synchronize"
    );

    let automatic = window
        .debug_bounds("quick-launch-volume-scope-automatic")
        .expect("automatic volume mode row");
    window.simulate_click(automatic.center(), Modifiers::default());
    window.run_until_parked();
    assert!(
        window
            .update(|_, cx| quick_launch.read(cx).settings().indexed_volumes)
            .is_empty(),
        "automatic mode must retain the empty-vector persistence contract"
    );
    assert_single_index_settings_command(&indexer_endpoint, &[], &[]);
}

#[gpui::test]
fn quick_launch_exclusion_paths_use_one_keyboard_operable_list_with_bounded_actions(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_component::init);
    let executor = cx.executor();
    let notifications = cx.update(|cx| cx.new(|_| NotificationCenter::default()));
    let temp = tempfile::tempdir().expect("temporary Quick Launch exclusion fixture");
    let catalog = FinderCatalog::open(temp.path().join("history.db"), &[])
        .expect("open Quick Launch exclusion catalog");
    let (platform, platform_endpoint) = platform_bridge(8);
    platform_endpoint.set_connected_for_contract_test(true);
    let (indexer, indexer_endpoint) = crate::runtime::indexer_bridge(8);
    indexer_endpoint.connect_for_test();
    let long_path = r"C:\Users\Administrator\Documents\NxTools\Projects\Archived\Search Corpus";
    let build_path = r"D:\Build\Cache";
    let temp_path = r"E:\Temp";
    let mut config = AppConfig::default();
    config.quick_launch.excluded_paths = vec![
        long_path.to_owned(),
        build_path.to_owned(),
        temp_path.to_owned(),
    ];
    let quick_launch = cx.update(|cx| {
        cx.new(|_| {
            QuickLaunchApplication::new(
                catalog,
                Rc::new(platform),
                Rc::new(indexer),
                config,
                temp.path().join("config.json"),
                temp.path().join("logs"),
                notifications.clone(),
            )
        })
    });
    let quick_launch_for_window = quick_launch.downgrade();
    let notifications_for_window = notifications.clone();
    let (main_window, window) = cx.add_window_view(move |window, cx| {
        window.resize(size(px(1060.0), px(720.0)));
        let mut main_window = MainWindow::new(
            gpui::WeakEntity::<ClipboardApplication>::new_invalid(),
            quick_launch_for_window,
            gpui::WeakEntity::<DownloadApplication>::new_invalid(),
            MainWindowShellContext {
                settings: None,
                platform: None,
                system_settings: Rc::new(RefCell::new(SystemSettings::default())),
            },
            notifications_for_window,
            window,
            cx,
        );
        main_window.shell.activate(MainModule::QuickFind);
        main_window.quick_launch_settings_page = QuickLaunchSettingsPage::Index;
        main_window
    });

    window.refresh().expect("render exclusion path list");
    let list = window
        .debug_bounds("quick-launch-exclusion-list")
        .expect("one continuous exclusion list");
    let first_row = window
        .debug_bounds("quick-launch-exclusion-row-0")
        .expect("first exclusion row");
    let second_row = window
        .debug_bounds("quick-launch-exclusion-row-1")
        .expect("second exclusion row");
    let third_row = window
        .debug_bounds("quick-launch-exclusion-row-2")
        .expect("third exclusion row");
    for row in [first_row, second_row, third_row] {
        assert_eq!(row.size.height, px(40.0));
        assert_eq!(row.left(), list.left() + px(1.0));
        assert_eq!(row.size.width, list.size.width - px(2.0));
    }
    let first_path = window
        .debug_bounds("quick-launch-exclusion-path-0")
        .expect("truncated path rail");
    let first_delete = window
        .debug_bounds("quick-launch-exclusion-delete-0")
        .expect("bounded delete hit target");
    assert_eq!(first_path.size.height, px(LINE_HEIGHT_BODY));
    assert_eq!(first_delete.size, size(px(40.0), px(40.0)));
    assert!(first_path.right() <= first_delete.left());
    assert_eq!(
        crate::ui::components::path_list::exact_path_tooltip(long_path).to_string(),
        long_path,
        "the production tooltip seam must preserve the exact original path"
    );

    window.simulate_mouse_move(first_path.center(), None, Modifiers::default());
    executor.advance_clock(Duration::from_millis(500));
    window.run_until_parked();
    assert!(
        window
            .debug_bounds("quick-launch-exclusion-path-tooltip-0")
            .is_some(),
        "the exact long path must remain available through a tooltip"
    );

    window.simulate_click(second_row.center(), Modifiers::default());
    assert_eq!(
        window.read(|cx| main_window.read(cx).selected_index_exclusion),
        Some(1)
    );
    window.simulate_keystrokes("up");
    assert_eq!(
        window.read(|cx| main_window.read(cx).selected_index_exclusion),
        Some(0)
    );
    window.simulate_keystrokes("down");
    assert_eq!(
        window.read(|cx| main_window.read(cx).selected_index_exclusion),
        Some(1)
    );
    window.simulate_keystrokes("escape");
    assert_eq!(
        window.read(|cx| main_window.read(cx).selected_index_exclusion),
        None
    );
    window.simulate_click(second_row.center(), Modifiers::default());
    window.simulate_keystrokes("backspace");
    window.run_until_parked();
    assert_eq!(
        window.update(|_, cx| quick_launch.read(cx).settings().excluded_paths),
        vec![long_path.to_owned(), temp_path.to_owned()]
    );
    assert_single_index_settings_command(&indexer_endpoint, &[], &[long_path, temp_path]);

    window
        .refresh()
        .expect("render list after keyboard removal");
    let first_delete = window
        .debug_bounds("quick-launch-exclusion-delete-0")
        .expect("first delete target after keyboard removal");
    window.simulate_click(first_delete.center(), Modifiers::default());
    window.run_until_parked();
    assert_eq!(
        window.update(|_, cx| quick_launch.read(cx).settings().excluded_paths),
        vec![temp_path.to_owned()]
    );
    assert_single_index_settings_command(&indexer_endpoint, &[], &[temp_path]);

    window.refresh().expect("render add directory action");
    let add = window
        .debug_bounds("quick-launch-add-index-exclusion")
        .expect("standard add-directory action");
    assert_eq!(add.size.height, px(28.0));
    window.simulate_click(add.center(), Modifiers::default());
    window.run_until_parked();
    let picker = platform_endpoint
        .try_receive_command_for_contract_test()
        .expect("one typed directory picker command");
    assert!(matches!(
        &picker.message,
        Message::PlatformCommand(PlatformCommand::PickDirectory(_))
    ));
    assert!(
        platform_endpoint
            .try_receive_command_for_contract_test()
            .is_err(),
        "one add gesture must open only one directory picker"
    );

    window.update(|_, cx| {
        quick_launch.update(cx, |application, cx| {
            application.handle_platform_notification(
                PlatformNotification::CommandResult {
                    correlation_id: uuid::Uuid::new_v4(),
                    result: PlatformCommandResult {
                        success: true,
                        error: None,
                        native_error_code: None,
                        data: Some(r"F:\Archive".into()),
                    },
                },
                cx,
            );
        });
    });
    assert_eq!(
        window.update(|_, cx| quick_launch.read(cx).settings().excluded_paths),
        vec![temp_path.to_owned()],
        "a stale picker result must not mutate the current exclusion list"
    );
    window.update(|_, cx| {
        quick_launch.update(cx, |application, cx| {
            application.handle_platform_notification(
                PlatformNotification::CommandResult {
                    correlation_id: picker.id,
                    result: PlatformCommandResult {
                        success: true,
                        error: None,
                        native_error_code: None,
                        data: Some(r"e:\TEMP".into()),
                    },
                },
                cx,
            );
        });
    });
    assert!(
        indexer_endpoint.try_recv_command_for_test().is_none(),
        "a case-insensitive duplicate picker result must not synchronize"
    );

    window.refresh().expect("render add action after duplicate");
    let add = window
        .debug_bounds("quick-launch-add-index-exclusion")
        .expect("add action after duplicate");
    window.simulate_click(add.center(), Modifiers::default());
    window.run_until_parked();
    let selected_picker = platform_endpoint
        .try_receive_command_for_contract_test()
        .expect("second typed directory picker command");
    window.update(|_, cx| {
        quick_launch.update(cx, |application, cx| {
            application.handle_platform_notification(
                PlatformNotification::CommandResult {
                    correlation_id: selected_picker.id,
                    result: PlatformCommandResult {
                        success: true,
                        error: None,
                        native_error_code: None,
                        data: Some(r"F:\Archive".into()),
                    },
                },
                cx,
            );
        });
    });
    window.run_until_parked();
    assert_eq!(
        window.update(|_, cx| quick_launch.read(cx).settings().excluded_paths),
        vec![temp_path.to_owned(), r"F:\Archive".to_owned()]
    );
    assert_single_index_settings_command(&indexer_endpoint, &[], &[temp_path, r"F:\Archive"]);

    window.refresh().expect("render add action after selection");
    let add = window
        .debug_bounds("quick-launch-add-index-exclusion")
        .expect("add action after selection");
    window.simulate_click(add.center(), Modifiers::default());
    window.run_until_parked();
    let cancelled_picker = platform_endpoint
        .try_receive_command_for_contract_test()
        .expect("third typed directory picker command");
    window.update(|_, cx| {
        quick_launch.update(cx, |application, cx| {
            application.handle_platform_notification(
                PlatformNotification::CommandResult {
                    correlation_id: cancelled_picker.id,
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
        indexer_endpoint.try_recv_command_for_test().is_none(),
        "picker cancellation must not persist or synchronize"
    );

    window
        .refresh()
        .expect("render add action after cancellation");
    let add = window
        .debug_bounds("quick-launch-add-index-exclusion")
        .expect("add action after cancellation");
    window.simulate_click(add.center(), Modifiers::default());
    window.run_until_parked();
    let failed_picker = platform_endpoint
        .try_receive_command_for_contract_test()
        .expect("fourth typed directory picker command");
    window.update(|_, cx| {
        quick_launch.update(cx, |application, cx| {
            application.handle_platform_notification(
                PlatformNotification::CommandResult {
                    correlation_id: failed_picker.id,
                    result: PlatformCommandResult {
                        success: false,
                        error: Some("directory picker unavailable".into()),
                        native_error_code: Some(5),
                        data: None,
                    },
                },
                cx,
            );
        });
    });
    assert!(
        indexer_endpoint.try_recv_command_for_test().is_none(),
        "picker failure must not persist or synchronize"
    );
    assert_eq!(
        window.update(|_, cx| quick_launch.read(cx).settings().excluded_paths),
        vec![temp_path.to_owned(), r"F:\Archive".to_owned()]
    );

    window.refresh().expect("render final keyboard removal");
    let second_row = window
        .debug_bounds("quick-launch-exclusion-row-1")
        .expect("second exclusion row before Delete");
    window.simulate_click(second_row.center(), Modifiers::default());
    window.simulate_keystrokes("delete");
    window.run_until_parked();
    assert_eq!(
        window.update(|_, cx| quick_launch.read(cx).settings().excluded_paths),
        vec![temp_path.to_owned()]
    );
    assert_single_index_settings_command(&indexer_endpoint, &[], &[temp_path]);

    window.refresh().expect("render final exclusion row");
    let final_row = window
        .debug_bounds("quick-launch-exclusion-row-0")
        .expect("final exclusion row");
    window.simulate_click(final_row.center(), Modifiers::default());
    window.simulate_keystrokes("delete");
    window.run_until_parked();
    assert!(
        window
            .update(|_, cx| quick_launch.read(cx).settings().excluded_paths)
            .is_empty(),
        "removing the last exclusion must leave a valid empty list"
    );
    assert_single_index_settings_command(&indexer_endpoint, &[], &[]);
    window.refresh().expect("render empty exclusion list");
    assert_eq!(
        window
            .debug_bounds("quick-launch-exclusion-empty")
            .expect("neutral empty row")
            .size
            .height,
        px(40.0)
    );
}

#[gpui::test]
fn quick_launch_custom_commands_use_one_master_detail_editor_with_separated_actions(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_component::init);
    let notifications = cx.update(|cx| cx.new(|_| NotificationCenter::default()));
    let temp = tempfile::tempdir().expect("temporary custom-command fixture");
    let catalog = FinderCatalog::open(temp.path().join("history.db"), &[])
        .expect("open custom-command catalog");
    let (platform, platform_endpoint) = platform_bridge(8);
    platform_endpoint.set_connected_for_contract_test(true);
    let (indexer, _indexer_endpoint) = crate::runtime::indexer_bridge(8);
    let mut config = AppConfig::default();
    config.quick_launch.custom_commands = vec![
        CustomCommandSettings {
            enabled: true,
            keyword: "docs".into(),
            title: "Documentation".into(),
            program: r"C:\Tools\docs.exe".into(),
            argument_template: "%s".into(),
            working_directory: r"C:\Docs".into(),
            run_as_admin: false,
            run_silently: false,
        },
        CustomCommandSettings {
            enabled: false,
            keyword: "build".into(),
            title: "Build workspace".into(),
            program: r"D:\Tools\build.exe".into(),
            argument_template: "--target %s1".into(),
            working_directory: r"D:\Workspace".into(),
            run_as_admin: true,
            run_silently: true,
        },
    ];
    let quick_launch = cx.update(|cx| {
        cx.new(|_| {
            QuickLaunchApplication::new(
                catalog,
                Rc::new(platform),
                Rc::new(indexer),
                config,
                temp.path().join("config.json"),
                temp.path().join("logs"),
                notifications.clone(),
            )
        })
    });
    let quick_launch_for_window = quick_launch.downgrade();
    let notifications_for_window = notifications.clone();
    let (main_window, window) = cx.add_window_view(move |window, cx| {
        window.resize(size(px(1060.0), px(720.0)));
        let mut main_window = MainWindow::new(
            gpui::WeakEntity::<ClipboardApplication>::new_invalid(),
            quick_launch_for_window,
            gpui::WeakEntity::<DownloadApplication>::new_invalid(),
            MainWindowShellContext {
                settings: None,
                platform: None,
                system_settings: Rc::new(RefCell::new(SystemSettings::default())),
            },
            notifications_for_window,
            window,
            cx,
        );
        main_window.shell.activate(MainModule::QuickFind);
        main_window.quick_launch_settings_page = QuickLaunchSettingsPage::Commands;
        main_window
    });

    window
        .refresh()
        .expect("render custom-command master-detail");
    let shell = window
        .debug_bounds("quick-launch-custom-command-master-detail")
        .expect("one continuous custom-command shell");
    let master = window
        .debug_bounds("quick-launch-custom-command-master")
        .expect("stable master rail");
    let detail = window
        .debug_bounds("quick-launch-custom-command-detail")
        .expect("one detail editor");
    assert_eq!(master.right(), detail.left());
    assert_eq!(master.top(), shell.top() + px(1.0));
    assert_eq!(detail.bottom(), shell.bottom() - px(1.0));

    for index in 0..2 {
        let row_selector: &'static str =
            Box::leak(format!("quick-launch-command-{index}").into_boxed_str());
        let title_selector: &'static str =
            Box::leak(format!("quick-launch-command-title-{index}").into_boxed_str());
        let context_selector: &'static str =
            Box::leak(format!("quick-launch-command-context-{index}").into_boxed_str());
        assert_eq!(
            window
                .debug_bounds(row_selector)
                .expect("master row")
                .size
                .height,
            px(42.0)
        );
        assert_eq!(
            window
                .debug_bounds(title_selector)
                .expect("Body row title")
                .size
                .height,
            px(LINE_HEIGHT_BODY)
        );
        assert_eq!(
            window
                .debug_bounds(context_selector)
                .expect("Label row context")
                .size
                .height,
            px(LINE_HEIGHT_LABEL)
        );
        assert!(
            window
                .debug_bounds(Box::leak(
                    format!("quick-launch-command-enabled-{index}").into_boxed_str()
                ))
                .is_none(),
            "enable must belong to the detail heading, not each master row"
        );
        assert!(
            window
                .debug_bounds(Box::leak(
                    format!("quick-launch-command-delete-{index}").into_boxed_str()
                ))
                .is_none(),
            "delete must not crowd every master row"
        );
    }
    let selected_marker = window
        .debug_bounds("quick-launch-command-selection-0")
        .expect("non-shifting selected-command marker");
    assert_eq!(selected_marker.size.width, px(2.0));
    assert_eq!(
        selected_marker.size.height,
        window
            .debug_bounds("quick-launch-command-0")
            .expect("selected master row")
            .size
            .height
    );
    assert_eq!(
        window
            .debug_bounds("quick-launch-command-title-0")
            .expect("selected command title")
            .left(),
        window
            .debug_bounds("quick-launch-command-title-1")
            .expect("unselected command title")
            .left(),
        "the selected edge must not shift the command copy rail"
    );

    for selector in [
        "quick-launch-add-custom-command",
        "quick-launch-command-enabled",
        "quick-launch-delete-custom-command",
        "quick-launch-pick-command-program",
        "quick-launch-save-custom-command",
    ] {
        assert_eq!(
            window
                .debug_bounds(selector)
                .unwrap_or_else(|| panic!("missing bounded action {selector}"))
                .size
                .height,
            px(28.0)
        );
    }
    for selector in [
        "quick-launch-command-field-keyword",
        "quick-launch-command-field-title",
        "quick-launch-command-field-program",
        "quick-launch-command-field-arguments",
        "quick-launch-command-field-working-directory",
    ] {
        assert_eq!(
            window
                .debug_bounds(selector)
                .unwrap_or_else(|| panic!("missing shared field shell {selector}"))
                .size
                .height,
            px(32.0)
        );
    }

    window.update(|window, cx| {
        main_window.update(cx, |main_window, cx| {
            main_window
                .custom_command_keyword
                .update(cx, |input, cx| input.set_value("unsaved-docs", window, cx));
        });
    });
    let second_row = window
        .debug_bounds("quick-launch-command-1")
        .expect("second command row");
    window.simulate_click(second_row.center(), Modifiers::default());
    assert_eq!(
        window.read(|cx| main_window.read(cx).custom_command_keyword.read(cx).value()),
        "build"
    );
    assert_eq!(
        window.update(|_, cx| {
            quick_launch.read(cx).settings().custom_commands[0]
                .keyword
                .clone()
        }),
        "docs",
        "changing master selection must not implicitly save the previous draft"
    );

    window.refresh().expect("render selected command editor");
    let enabled = window
        .debug_bounds("quick-launch-command-enabled")
        .expect("detail enabled checkbox");
    window.simulate_click(enabled.center(), Modifiers::default());
    window.run_until_parked();
    assert!(
        window.update(|_, cx| quick_launch.read(cx).settings().custom_commands[1].enabled),
        "the detail checkbox must preserve explicit immediate persistence"
    );

    window.refresh().expect("render enabled command editor");
    let picker = window
        .debug_bounds("quick-launch-pick-command-program")
        .expect("program picker action");
    window.simulate_click(picker.center(), Modifiers::default());
    window.run_until_parked();
    let picker_command = platform_endpoint
        .try_receive_command_for_contract_test()
        .expect("one typed program picker command");
    assert!(matches!(
        picker_command.message,
        Message::PlatformCommand(PlatformCommand::PickProgram(_))
    ));
    assert!(
        platform_endpoint
            .try_receive_command_for_contract_test()
            .is_err(),
        "one picker gesture must emit one typed command"
    );
    window.update(|_, cx| {
        quick_launch.update(cx, |application, cx| {
            application.handle_platform_notification(
                PlatformNotification::CommandResult {
                    correlation_id: uuid::Uuid::new_v4(),
                    result: PlatformCommandResult {
                        success: true,
                        error: None,
                        native_error_code: None,
                        data: Some(r"E:\Picked\wrong-correlation.exe".into()),
                    },
                },
                cx,
            );
        });
    });
    window.run_until_parked();
    assert_eq!(
        window.read(|cx| main_window.read(cx).custom_command_program.read(cx).value()),
        r"D:\Tools\build.exe",
        "a non-matching platform correlation must not reach the editor draft"
    );
    let first_row = window
        .debug_bounds("quick-launch-command-0")
        .expect("first command row");
    window.simulate_click(first_row.center(), Modifiers::default());
    window.run_until_parked();
    window.refresh().expect("render first command draft");
    let second_row = window
        .debug_bounds("quick-launch-command-1")
        .expect("second command row after leaving it");
    window.simulate_click(second_row.center(), Modifiers::default());
    window.run_until_parked();
    window.update(|_, cx| {
        quick_launch.update(cx, |application, cx| {
            application.handle_platform_notification(
                PlatformNotification::CommandResult {
                    correlation_id: picker_command.id,
                    result: PlatformCommandResult {
                        success: true,
                        error: None,
                        native_error_code: None,
                        data: Some(r"E:\Picked\replacement.exe".into()),
                    },
                },
                cx,
            );
        });
    });
    window.run_until_parked();
    assert_eq!(
        window.read(|cx| main_window.read(cx).custom_command_program.read(cx).value()),
        r"D:\Tools\build.exe",
        "leaving and returning to the same index must still reject the stale picker identity"
    );

    window.refresh().expect("render cancellation picker");
    let picker = window
        .debug_bounds("quick-launch-pick-command-program")
        .expect("cancellation picker action");
    window.simulate_click(picker.center(), Modifiers::default());
    window.run_until_parked();
    let cancelled_picker = platform_endpoint
        .try_receive_command_for_contract_test()
        .expect("one picker to cancel");
    window.update(|_, cx| {
        quick_launch.update(cx, |application, cx| {
            application.handle_platform_notification(
                PlatformNotification::CommandResult {
                    correlation_id: cancelled_picker.id,
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
    window.run_until_parked();
    assert_eq!(
        window.read(|cx| main_window.read(cx).custom_command_program.read(cx).value()),
        r"D:\Tools\build.exe",
        "picker cancellation must not modify the current draft"
    );

    window.refresh().expect("render first command selection");
    let first_row = window
        .debug_bounds("quick-launch-command-0")
        .expect("first command row after picker cancellation");
    window.simulate_click(first_row.center(), Modifiers::default());
    window.run_until_parked();
    window.refresh().expect("render current command picker");
    let picker = window
        .debug_bounds("quick-launch-pick-command-program")
        .expect("current command picker action");
    window.simulate_click(picker.center(), Modifiers::default());
    window.run_until_parked();
    let current_picker = platform_endpoint
        .try_receive_command_for_contract_test()
        .expect("one picker for the current draft");
    window.update(|_, cx| {
        quick_launch.update(cx, |application, cx| {
            application.handle_platform_notification(
                PlatformNotification::CommandResult {
                    correlation_id: current_picker.id,
                    result: PlatformCommandResult {
                        success: true,
                        error: None,
                        native_error_code: None,
                        data: Some(r"E:\Picked\docs.exe".into()),
                    },
                },
                cx,
            );
        });
    });
    window.run_until_parked();
    assert_eq!(
        window.read(|cx| main_window.read(cx).custom_command_program.read(cx).value()),
        r"E:\Picked\docs.exe"
    );
    assert_eq!(
        window.update(|_, cx| {
            quick_launch.read(cx).settings().custom_commands[0]
                .program
                .clone()
        }),
        r"C:\Tools\docs.exe",
        "a matching picker result must remain an unsaved editor draft"
    );

    window.update(|window, cx| {
        main_window.update(cx, |main_window, cx| {
            main_window
                .custom_command_title
                .update(cx, |input, cx| input.set_value("Docs search", window, cx));
        });
    });
    window.refresh().expect("render edited draft");
    let save = window
        .debug_bounds("quick-launch-save-custom-command")
        .expect("save command action");
    window.simulate_click(save.center(), Modifiers::default());
    window.run_until_parked();
    assert_eq!(
        window.update(|_, cx| {
            quick_launch.read(cx).settings().custom_commands[0]
                .title
                .clone()
        }),
        "Docs search"
    );
    assert_eq!(
        window.update(|_, cx| {
            quick_launch.read(cx).settings().custom_commands[0]
                .program
                .clone()
        }),
        r"E:\Picked\docs.exe"
    );

    window.refresh().expect("render command before delete");
    let delete = window
        .debug_bounds("quick-launch-delete-custom-command")
        .expect("detail delete action");
    window.simulate_click(delete.center(), Modifiers::default());
    window.run_until_parked();
    assert_eq!(
        window.update(|_, cx| quick_launch.read(cx).settings().custom_commands.len()),
        1
    );
    assert_eq!(
        window.read(|cx| main_window.read(cx).selected_custom_command),
        Some(0)
    );

    window.refresh().expect("render add command action");
    let add = window
        .debug_bounds("quick-launch-add-custom-command")
        .expect("master add action");
    window.simulate_click(add.center(), Modifiers::default());
    window.run_until_parked();
    assert_eq!(
        window.update(|_, cx| quick_launch.read(cx).settings().custom_commands.len()),
        2
    );
    assert_eq!(
        window.read(|cx| main_window.read(cx).selected_custom_command),
        Some(1)
    );
}

#[gpui::test]
fn quick_launch_diagnostics_keep_metrics_logs_and_history_actions_in_one_surface(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_component::init);
    let executor = cx.executor();
    let notifications = cx.update(|cx| cx.new(|_| NotificationCenter::default()));
    let temp = tempfile::tempdir().expect("temporary Quick Launch diagnostics fixture");
    let history_path = temp.path().join("history.db");
    let mut catalog =
        FinderCatalog::open(&history_path, &[]).expect("open Quick Launch diagnostics catalog");
    catalog
        .record_activation("editor", "edit", 1_700_000_000_000)
        .expect("record first ranking activation");
    catalog
        .record_activation("editor", "edit", 1_700_000_001_000)
        .expect("record second ranking activation");
    let log_directory = temp.path().join("logs");
    let (platform, platform_endpoint) = platform_bridge(8);
    platform_endpoint.set_connected_for_contract_test(true);
    let quick_launch = cx.update(|cx| {
        let (indexer, _indexer_endpoint) = crate::runtime::indexer_bridge(8);
        cx.new(|_| {
            QuickLaunchApplication::new(
                catalog,
                Rc::new(platform),
                Rc::new(indexer),
                AppConfig::default(),
                temp.path().join("config.json"),
                log_directory.clone(),
                notifications.clone(),
            )
        })
    });
    cx.update(|cx| {
        quick_launch.update(cx, |application, _| {
            application.set_performance_snapshot_for_test(Some(240), Some(12_345));
        });
    });
    let quick_launch_for_window = quick_launch.downgrade();
    let notifications_for_window = notifications.clone();
    let (main_window, window) = cx.add_window_view(move |window, cx| {
        window.resize(gpui::size(px(1060.0), px(720.0)));
        let mut main_window = MainWindow::new(
            gpui::WeakEntity::<ClipboardApplication>::new_invalid(),
            quick_launch_for_window,
            gpui::WeakEntity::<DownloadApplication>::new_invalid(),
            MainWindowShellContext {
                settings: None,
                platform: None,
                system_settings: Rc::new(RefCell::new(SystemSettings::default())),
            },
            notifications_for_window,
            window,
            cx,
        );
        main_window.shell.activate(MainModule::QuickFind);
        main_window.quick_launch_settings_page = QuickLaunchSettingsPage::Diagnostics;
        main_window
    });

    window
        .refresh()
        .expect("render Quick Launch diagnostics page");
    let surface = window
        .debug_bounds("quick-launch-diagnostics-surface")
        .expect("one continuous diagnostics surface");
    let history_header = window
        .debug_bounds("quick-launch-diagnostics-history-header")
        .expect("history section header");
    let history_metrics = window
        .debug_bounds("quick-launch-diagnostics-history-metrics")
        .expect("history metric strip");
    let performance_header = window
        .debug_bounds("quick-launch-diagnostics-performance-header")
        .expect("performance section header");
    let performance_metrics = window
        .debug_bounds("quick-launch-diagnostics-performance-metrics")
        .expect("performance metric strip");
    let log_row = window
        .debug_bounds("quick-launch-diagnostics-log-row")
        .expect("log directory row");
    assert_eq!(history_header.left() - surface.left(), px(1.0));
    assert_eq!(surface.right() - history_header.right(), px(1.0));
    assert_eq!(history_header.bottom(), history_metrics.top());
    assert_eq!(history_metrics.bottom(), performance_header.top());
    assert_eq!(performance_header.bottom(), performance_metrics.top());
    assert_eq!(performance_metrics.bottom(), log_row.top());
    assert_eq!(surface.bottom() - log_row.bottom(), px(1.0));
    assert_eq!(history_header.size.height, px(48.0));
    assert_eq!(history_metrics.size.height, px(48.0));
    assert_eq!(performance_header.size.height, px(48.0));
    assert_eq!(performance_metrics.size.height, px(48.0));
    assert_eq!(log_row.size.height, px(48.0));

    let history_cells = [
        "quick-launch-diagnostics-ranked-items",
        "quick-launch-diagnostics-activations",
        "quick-launch-diagnostics-query-associations",
        "quick-launch-diagnostics-recent-directories",
    ]
    .map(|selector| {
        window
            .debug_bounds(selector)
            .unwrap_or_else(|| panic!("missing {selector}"))
    });
    for pair in history_cells.windows(2) {
        assert_eq!(pair[0].right(), pair[1].left());
        assert!((pair[0].size.width - pair[1].size.width).abs() <= px(1.0));
    }
    for selector in [
        "quick-launch-diagnostics-ranked-items-value",
        "quick-launch-diagnostics-activations-value",
        "quick-launch-diagnostics-query-associations-value",
        "quick-launch-diagnostics-recent-directories-value",
        "quick-launch-diagnostics-local-search-value",
        "quick-launch-diagnostics-index-search-value",
    ] {
        assert_eq!(
            window
                .debug_bounds(selector)
                .unwrap_or_else(|| panic!("missing {selector}"))
                .size
                .height,
            px(LINE_HEIGHT_BODY)
        );
    }
    let local_search = window
        .debug_bounds("quick-launch-diagnostics-local-search")
        .expect("local merge metric");
    let index_search = window
        .debug_bounds("quick-launch-diagnostics-index-search")
        .expect("index batch metric");
    assert_eq!(local_search.right(), index_search.left());
    assert!((local_search.size.width - index_search.size.width).abs() <= px(1.0));
    assert_eq!(
        window
            .debug_bounds("quick-launch-clear-history")
            .expect("clear history action")
            .size
            .height,
        px(28.0)
    );
    assert_eq!(
        window
            .debug_bounds("quick-launch-open-logs")
            .expect("open logs action")
            .size
            .height,
        px(28.0)
    );
    assert_eq!(
        crate::ui::components::quick_launch_diagnostics::exact_log_path_tooltip(
            &log_directory.to_string_lossy()
        )
        .to_string(),
        log_directory.to_string_lossy(),
        "the diagnostics tooltip must preserve the exact log directory"
    );
    let log_path = window
        .debug_bounds("quick-launch-diagnostics-log-path")
        .expect("truncated log path rail");
    window.simulate_mouse_move(log_path.center(), None, Modifiers::default());
    executor.advance_clock(Duration::from_millis(500));
    window.run_until_parked();
    assert!(
        window
            .debug_bounds("quick-launch-diagnostics-log-tooltip")
            .is_some(),
        "the exact log path must remain available through a tooltip"
    );

    let open_logs = window
        .debug_bounds("quick-launch-open-logs")
        .expect("open logs action");
    window.simulate_click(open_logs.center(), Modifiers::default());
    window.run_until_parked();
    let open_command = platform_endpoint
        .try_receive_command_for_contract_test()
        .expect("one typed log-directory command");
    let Message::PlatformCommand(PlatformCommand::OpenShellTarget(command)) = open_command.message
    else {
        panic!("opening logs must use the typed Windows shell boundary");
    };
    assert_eq!(command.target, log_directory.to_string_lossy());
    assert_eq!(command.action, nxtools_protocol::ShellOpenAction::Open);
    assert!(
        platform_endpoint
            .try_receive_command_for_contract_test()
            .is_err(),
        "one click must emit one shell command"
    );

    let clear_history = window
        .debug_bounds("quick-launch-clear-history")
        .expect("clear history action");
    assert_eq!(
        window.update(|_, cx| quick_launch.read(cx).history_summary().total_activations),
        2
    );

    for (navigation_selector, content_selector, expected_page) in [
        (
            "quick-launch-settings-nav-general",
            "quick-launch-settings-page-general",
            QuickLaunchSettingsPage::General,
        ),
        (
            "quick-launch-settings-nav-index",
            "quick-launch-settings-page-index",
            QuickLaunchSettingsPage::Index,
        ),
        (
            "quick-launch-settings-nav-context",
            "quick-launch-settings-page-context",
            QuickLaunchSettingsPage::Context,
        ),
        (
            "quick-launch-settings-nav-commands",
            "quick-launch-settings-page-commands",
            QuickLaunchSettingsPage::Commands,
        ),
    ] {
        let navigation = window
            .debug_bounds(navigation_selector)
            .unwrap_or_else(|| panic!("missing {navigation_selector}"));
        window.simulate_click(navigation.center(), Modifiers::default());
        window.run_until_parked();
        window.refresh().expect("render ordinary Quick Launch page");
        window.run_until_parked();
        assert_eq!(
            window.read(|cx| main_window.read(cx).quick_launch_settings_page),
            expected_page
        );
        assert!(
            window.debug_bounds(content_selector).is_some(),
            "the selected ordinary page must replace Diagnostics: {expected_page:?}"
        );

        window.simulate_click(open_logs.center(), Modifiers::default());
        window.run_until_parked();
        assert!(
            platform_endpoint
                .try_receive_command_for_contract_test()
                .is_err(),
            "the detached log action must not remain interactive on {expected_page:?}"
        );
        window.simulate_click(clear_history.center(), Modifiers::default());
        window.run_until_parked();
        assert_eq!(
            window.update(|_, cx| quick_launch.read(cx).history_summary().total_activations),
            2,
            "the detached history action must not remain interactive on {expected_page:?}"
        );
    }
    let diagnostics_navigation = window
        .debug_bounds("quick-launch-settings-nav-diagnostics")
        .expect("diagnostics navigation row");
    window.simulate_click(diagnostics_navigation.center(), Modifiers::default());
    window.run_until_parked();
    window
        .refresh()
        .expect("return to Quick Launch diagnostics");
    window.run_until_parked();
    assert!(
        window
            .debug_bounds("quick-launch-diagnostics-surface")
            .is_some()
            && window
                .debug_bounds("quick-launch-diagnostics-performance-metrics")
                .is_some(),
        "diagnostics metrics must remount only after returning to Diagnostics"
    );
    let clear_history = window
        .debug_bounds("quick-launch-clear-history")
        .expect("remounted clear history action");
    window.simulate_click(clear_history.center(), Modifiers::default());
    window.run_until_parked();
    assert_eq!(
        window.update(|_, cx| quick_launch.read(cx).history_summary()),
        nxtools_finder::QuickLaunchHistorySummary::default()
    );
    assert_eq!(
        window.read(|cx| main_window.read(cx).quick_launch_settings_page),
        QuickLaunchSettingsPage::Diagnostics
    );
}

#[gpui::test]
fn quick_launch_hotkey_field_exposes_the_native_registration_lifecycle(cx: &mut TestAppContext) {
    cx.update(gpui_component::init);
    let notifications = cx.update(|cx| cx.new(|_| NotificationCenter::default()));
    let temp = tempfile::tempdir().expect("temporary Quick Launch hotkey fixture");
    let catalog = FinderCatalog::open(temp.path().join("history.db"), &[])
        .expect("open Quick Launch hotkey catalog");
    let (platform, platform_endpoint) = platform_bridge(16);
    platform_endpoint.set_connected_for_contract_test(true);
    let (indexer, _indexer_endpoint) = crate::runtime::indexer_bridge(8);
    let quick_launch = cx.update(|cx| {
        cx.new(|_| {
            QuickLaunchApplication::new(
                catalog,
                Rc::new(platform),
                Rc::new(indexer),
                AppConfig::default(),
                temp.path().join("config.json"),
                temp.path().join("logs"),
                notifications.clone(),
            )
        })
    });
    quick_launch.update(cx, |application, cx| {
        application.handle_platform_notification(PlatformNotification::Connected, cx);
    });
    let initial_configuration = platform_endpoint
        .try_receive_command_for_contract_test()
        .expect("apply the initial Quick Launch configuration");
    assert!(matches!(
        initial_configuration.message,
        Message::PlatformCommand(PlatformCommand::ConfigureQuickLaunch(_))
    ));
    let original_hotkey = cx.read(|cx| quick_launch.read(cx).settings().hotkey);
    let quick_launch_for_window = quick_launch.downgrade();
    let notifications_for_window = notifications.clone();
    let (_main_window, window) = cx.add_window_view(move |window, cx| {
        window.resize(gpui::size(px(1060.0), px(500.0)));
        let mut main_window = MainWindow::new(
            gpui::WeakEntity::<ClipboardApplication>::new_invalid(),
            quick_launch_for_window,
            gpui::WeakEntity::<DownloadApplication>::new_invalid(),
            MainWindowShellContext {
                settings: None,
                platform: None,
                system_settings: Rc::new(RefCell::new(SystemSettings::default())),
            },
            notifications_for_window,
            window,
            cx,
        );
        main_window.shell.activate(MainModule::QuickFind);
        main_window
    });

    window
        .refresh()
        .expect("render initial applying Quick Launch hotkey");
    assert!(
        window
            .debug_bounds("quick-launch-hotkey-field-state-applying")
            .is_some()
    );
    window.update(|_, cx| {
        quick_launch.update(cx, |application, cx| {
            application.handle_platform_notification(
                PlatformNotification::CommandResult {
                    correlation_id: initial_configuration.id,
                    result: PlatformCommandResult {
                        success: false,
                        error: Some("initial hotkey registration was rejected".into()),
                        native_error_code: Some(1409),
                        data: None,
                    },
                },
                cx,
            );
        });
    });
    window.run_until_parked();
    window
        .refresh()
        .expect("render failed initial Quick Launch hotkey");
    assert!(
        window
            .debug_bounds("quick-launch-hotkey-field-state-failed")
            .is_some()
    );
    assert_eq!(
        window.update(|_, cx| quick_launch.read(cx).settings().hotkey),
        original_hotkey,
        "a rejected initial registration must keep the configured hotkey"
    );

    window.update(|_, cx| {
        quick_launch.update(cx, |application, cx| {
            application.handle_platform_notification(PlatformNotification::Connected, cx);
        });
    });
    let retry_configuration = (0..8)
        .filter_map(|_| {
            platform_endpoint
                .try_receive_command_for_contract_test()
                .ok()
        })
        .find(|envelope| {
            matches!(
                envelope.message,
                Message::PlatformCommand(PlatformCommand::ConfigureQuickLaunch(_))
            )
        })
        .expect("retry the initial Quick Launch configuration");
    window.update(|_, cx| {
        quick_launch.update(cx, |application, cx| {
            application.handle_platform_notification(
                PlatformNotification::CommandResult {
                    correlation_id: retry_configuration.id,
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
    window.run_until_parked();
    window
        .refresh()
        .expect("render applied Quick Launch hotkey");
    let field = window
        .debug_bounds("quick-launch-hotkey-field-control")
        .expect("shared Quick Launch HotkeyField");
    assert_eq!(field.size.height, px(32.0));
    assert_eq!(
        window
            .debug_bounds("quick-launch-hotkey-field-value")
            .expect("HotkeyField value")
            .size
            .height,
        px(LINE_HEIGHT_BODY)
    );
    assert_eq!(
        window
            .debug_bounds("quick-launch-hotkey-field-status")
            .expect("HotkeyField status")
            .size
            .height,
        px(LINE_HEIGHT_LABEL)
    );
    assert_eq!(
        window
            .debug_bounds("quick-launch-hotkey-setting-title")
            .expect("HotkeyField setting title")
            .size
            .height,
        px(LINE_HEIGHT_BODY)
    );
    assert_eq!(
        window
            .debug_bounds("quick-launch-hotkey-setting-description")
            .expect("HotkeyField setting description")
            .size
            .height,
        px(LINE_HEIGHT_LABEL)
    );
    assert!(
        window
            .debug_bounds("quick-launch-hotkey-field-state-applied")
            .is_some()
    );

    window.simulate_click(field.center(), Modifiers::default());
    window.run_until_parked();
    window.refresh().expect("render waiting HotkeyField");
    assert!(
        window
            .debug_bounds("quick-launch-hotkey-field-state-waiting")
            .is_some()
    );
    let recording_envelope = (0..8)
        .filter_map(|_| {
            platform_endpoint
                .try_receive_command_for_contract_test()
                .ok()
        })
        .find(|envelope| {
            matches!(
                &envelope.message,
                Message::PlatformCommand(PlatformCommand::SetQuickLaunchHotKeyRecording(
                    command
                )) if command.recording
            )
        })
        .expect("pause the native Quick Launch trigger before recording");
    window.update(|_, cx| {
        quick_launch.update(cx, |application, cx| {
            application.handle_platform_notification(
                PlatformNotification::CommandResult {
                    correlation_id: recording_envelope.id,
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
    window.run_until_parked();
    window.refresh().expect("render recording HotkeyField");
    assert!(
        window
            .debug_bounds("quick-launch-hotkey-field-state-recording")
            .is_some()
    );
    while platform_endpoint
        .try_receive_command_for_contract_test()
        .is_ok()
    {}
    window.simulate_click(field.center(), Modifiers::default());
    window.run_until_parked();
    window
        .refresh()
        .expect("keep the recording HotkeyField stable after pointer input");
    assert!(
        window
            .debug_bounds("quick-launch-hotkey-field-state-recording")
            .is_some(),
        "recording may be cancelled only through Escape or focus loss"
    );
    assert!(
        platform_endpoint
            .try_receive_command_for_contract_test()
            .is_err(),
        "clicking the recording field must not send a second native command"
    );

    window.simulate_keystrokes("ctrl-alt-v");
    window.run_until_parked();
    let apply_envelope = (0..8)
        .filter_map(|_| {
            platform_endpoint
                .try_receive_command_for_contract_test()
                .ok()
        })
        .find(|envelope| {
            matches!(
                &envelope.message,
                Message::PlatformCommand(PlatformCommand::ConfigureQuickLaunch(command))
                    if command.hotkey_display == "Ctrl+Alt+V"
            )
        })
        .expect("apply the recorded Quick Launch hotkey");
    window.refresh().expect("render applying HotkeyField");
    assert!(
        window
            .debug_bounds("quick-launch-hotkey-field-state-applying")
            .is_some()
    );
    window.update(|_, cx| {
        quick_launch.update(cx, |application, cx| {
            application.handle_platform_notification(
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
    });
    window.run_until_parked();
    window
        .refresh()
        .expect("ignore a stale successful hotkey result");
    assert!(
        window
            .debug_bounds("quick-launch-hotkey-field-state-applying")
            .is_some(),
        "an unrelated success cannot complete the pending hotkey"
    );
    assert_eq!(
        window.update(|_, cx| quick_launch.read(cx).settings().hotkey),
        original_hotkey,
        "an unrelated success cannot replace the applied hotkey"
    );

    window.update(|_, cx| {
        quick_launch.update(cx, |application, cx| {
            application.handle_platform_notification(
                PlatformNotification::CommandResult {
                    correlation_id: apply_envelope.id,
                    result: PlatformCommandResult {
                        success: false,
                        error: Some("hotkey is already registered".into()),
                        native_error_code: Some(1409),
                        data: None,
                    },
                },
                cx,
            );
        });
    });
    window.run_until_parked();
    window.refresh().expect("render failed HotkeyField");
    assert!(
        window
            .debug_bounds("quick-launch-hotkey-field-state-failed")
            .is_some()
    );
    assert_eq!(
        cx.read(|cx| quick_launch.read(cx).settings().hotkey),
        original_hotkey,
        "a rejected native registration must keep the applied hotkey"
    );
}

#[gpui::test]
fn notifications_use_a_bounded_bottom_layer_without_covering_shell_actions(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_component::init);
    let notifications = cx.update(|cx| cx.new(|_| NotificationCenter::default()));
    let action_invoked = Rc::new(Cell::new(false));
    let action_invoked_for_handler = Rc::clone(&action_invoked);
    let notifications_for_action = notifications.clone();
    notifications.update(cx, |center, cx| {
        center.show_progress("transfer", "正在下载", "已完成 42%", cx);
        center.show_error("failure", "下载失败", "服务器暂时不可用", cx);
        center.show_success_with_details(
            "backup",
            "备份完成",
            "已创建可恢复备份",
            vec![NotificationDetail::new(
                "保存位置",
                r"C:\Users\Administrator\Downloads\NxTools Backup",
            )],
            Some(NotificationAction::new(
                "打开目录",
                move |_window, cx| {
                    action_invoked_for_handler.set(true);
                    let notifications_for_action = notifications_for_action.clone();
                    cx.defer(move |cx| {
                        notifications_for_action.update(cx, |center, cx| {
                            center.show_success("backup", "新的备份通知", "替换旧 generation", cx);
                        });
                    });
                },
            )),
            cx,
        );
    });
    let notifications_for_window = notifications.clone();
    let (_main_window, window) = cx.add_window_view(move |window, cx| {
        window.resize(size(px(920.0), px(480.0)));
        MainWindow::new(
            gpui::WeakEntity::<ClipboardApplication>::new_invalid(),
            gpui::WeakEntity::<QuickLaunchApplication>::new_invalid(),
            gpui::WeakEntity::<DownloadApplication>::new_invalid(),
            MainWindowShellContext {
                settings: None,
                platform: None,
                system_settings: Rc::new(RefCell::new(SystemSettings::default())),
            },
            notifications_for_window,
            window,
            cx,
        )
    });
    window
        .refresh()
        .expect("render production notification layer");
    window.update(|window, _| {
        assert_eq!(window.bounds().size, size(px(920.0), px(480.0)));
    });

    let titlebar = window.debug_bounds("main-titlebar").expect("main titlebar");
    let window_controls = window
        .debug_bounds("main-window-controls")
        .expect("window control group");
    let header_actions = window
        .debug_bounds("download-module-header-actions")
        .expect("module primary actions");
    let stack = window
        .debug_bounds("notification-stack")
        .expect("notification stack");
    let main_surface = window
        .debug_bounds("nxtools-main-window")
        .expect("main window surface");
    assert!(stack.top() >= header_actions.bottom());
    assert!(stack.top() >= titlebar.bottom());
    assert!(stack.top() >= window_controls.bottom());
    assert!(
        stack.right() <= main_surface.right() - px(SPACING_12) + px(1.0),
        "notification stack right edge is {}px",
        stack.right().as_f32()
    );
    assert!(
        stack.bottom() <= main_surface.bottom() - px(SPACING_12) + px(1.0),
        "notification stack bottom edge is {}px",
        stack.bottom().as_f32()
    );

    for id in ["backup", "failure", "transfer"] {
        let card = window
            .debug_bounds(Box::leak(format!("notification-{id}").into_boxed_str()))
            .unwrap_or_else(|| panic!("notification card {id}"));
        let marker = window
            .debug_bounds(Box::leak(
                format!("notification-marker-{id}").into_boxed_str(),
            ))
            .unwrap_or_else(|| panic!("semantic marker {id}"));
        let title = window
            .debug_bounds(Box::leak(
                format!("notification-title-{id}").into_boxed_str(),
            ))
            .unwrap_or_else(|| panic!("notification title {id}"));
        let message = window
            .debug_bounds(Box::leak(
                format!("notification-message-{id}").into_boxed_str(),
            ))
            .unwrap_or_else(|| panic!("notification message {id}"));
        let dismiss = window
            .debug_bounds(Box::leak(
                format!("dismiss-notification-{id}").into_boxed_str(),
            ))
            .unwrap_or_else(|| panic!("notification dismiss {id}"));
        assert_eq!(card.size.width, px(336.0));
        assert!(card.size.height <= px(112.0));
        assert_eq!(marker.size.width, px(3.0));
        assert_eq!(marker.top(), card.top() + px(1.0));
        assert_eq!(marker.bottom(), card.bottom() - px(1.0));
        assert_eq!(title.size.height, px(LINE_HEIGHT_BODY));
        assert_eq!(message.size.height, px(LINE_HEIGHT_LABEL));
        assert_eq!(dismiss.size, size(px(28.0), px(28.0)));
    }
    let backup = window
        .debug_bounds("notification-backup")
        .expect("detailed notification");
    let action = window
        .debug_bounds("notification-action-backup")
        .expect("notification action");
    assert_eq!(action.size.height, px(28.0));
    assert!(backup.contains(&action.origin));
    assert!(backup.contains(&action.bottom_right()));

    window.simulate_click(action.center(), Modifiers::default());
    window.run_until_parked();
    assert!(action_invoked.get());
    assert!(
        window.read(|cx| notifications.read(cx).items().iter().any(|item| {
            item.id.as_ref() == "backup" && item.title.as_ref() == "新的备份通知" && !item.exiting
        }))
    );

    let dismiss = window
        .debug_bounds("dismiss-notification-transfer")
        .expect("notification dismiss hit target");
    window.simulate_click(dismiss.center(), Modifiers::default());
    window.update(|_, cx| {
        notifications.update(cx, |center, cx| {
            center.show_progress("transfer", "新的下载通知", "替换旧 generation", cx);
        });
    });
    window.run_until_parked();
    assert!(window.read(|cx| {
        notifications.read(cx).items().iter().any(|item| {
            item.id.as_ref() == "transfer" && item.title.as_ref() == "新的下载通知" && !item.exiting
        })
    }));
}

#[gpui::test]
fn titlebar_move_and_close_reach_transport_without_destroying_the_main_entity(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_component::init);
    let executor = cx.executor();
    let notifications = cx.update(|cx| cx.new(|_| NotificationCenter::default()));
    let (platform, endpoint) = platform_bridge(8);
    endpoint.set_connected_for_contract_test(true);
    let platform = Rc::new(platform);
    let platform_for_window = Rc::clone(&platform);
    let system_settings = Rc::new(RefCell::new(SystemSettings {
        start_on_login_silently: false,
        close_to_tray: true,
    }));
    let (main_window, window) = cx.add_window_view(move |window, cx| {
        window.resize(size(px(920.0), px(480.0)));
        MainWindow::new(
            gpui::WeakEntity::<ClipboardApplication>::new_invalid(),
            gpui::WeakEntity::<QuickLaunchApplication>::new_invalid(),
            gpui::WeakEntity::<DownloadApplication>::new_invalid(),
            MainWindowShellContext {
                settings: None,
                platform: Some(platform_for_window),
                system_settings,
            },
            notifications,
            window,
            cx,
        )
    });
    window.update(|_, cx| {
        main_window.update(cx, |main_window, _| {
            // TestAppContext has no Windows HWND; inject the opaque value that the real
            // native boundary supplies, then verify it survives the typed transport.
            main_window.native_window_handle = 42;
        });
    });
    window.refresh().expect("render real main titlebar");

    let titlebar_shell = window
        .debug_bounds("main-titlebar")
        .expect("main titlebar shell");
    assert_eq!(titlebar_shell.size.height, px(36.0));
    let brand_icon = window
        .debug_bounds("main-brand-icon")
        .expect("real NxTools brand icon");
    assert_eq!(brand_icon.size, gpui::size(px(18.0), px(18.0)));
    assert_eq!(
        window
            .debug_bounds("main-brand-title")
            .expect("brand title")
            .size
            .height,
        px(LINE_HEIGHT_BODY)
    );
    assert_eq!(
        window
            .debug_bounds("main-active-module")
            .expect("active module metadata")
            .size
            .height,
        px(LINE_HEIGHT_LABEL)
    );
    let global_settings = window
        .debug_bounds("global-network-settings")
        .expect("global settings action");
    assert_eq!(global_settings.size.height, px(28.0));
    let utilities = window
        .debug_bounds("main-titlebar-utilities")
        .expect("right-side titlebar utility group");
    let control_group = window
        .debug_bounds("main-window-controls")
        .expect("window control group");
    for bounds in [global_settings, control_group] {
        assert!(utilities.contains(&bounds.origin));
        assert!(utilities.contains(&bounds.bottom_right()));
        assert!(titlebar_shell.contains(&bounds.origin));
        assert!(titlebar_shell.contains(&bounds.bottom_right()));
    }

    let titlebar = window
        .debug_bounds("main-titlebar-drag-region")
        .expect("titlebar drag bounds");
    assert!(titlebar.right() <= utilities.left());
    window.simulate_mouse_down(titlebar.center(), MouseButton::Left, Modifiers::default());
    let move_envelope = endpoint
        .try_receive_command_for_contract_test()
        .expect("typed move command");
    assert!(matches!(
        move_envelope.message,
        Message::PlatformCommand(PlatformCommand::BeginWindowMove(_))
    ));

    let minimize = window
        .debug_bounds("window-control-minimize")
        .expect("minimize control");
    let maximize = window
        .debug_bounds("window-control-maximize")
        .expect("maximize control");
    let close = window
        .debug_bounds("window-control-close")
        .expect("close control");
    for control in [minimize, maximize, close] {
        assert_eq!(control.size.width, px(TITLEBAR_HEIGHT));
        assert_eq!(control.size.height, titlebar_shell.size.height - px(1.0));
    }
    assert_eq!(minimize.right(), maximize.left());
    assert_eq!(maximize.right(), close.left());
    assert_eq!(minimize.left() - global_settings.right(), px(SPACING_4));
    for selector in [
        "window-control-minimize-icon",
        "window-control-maximize-icon",
        "window-control-close-icon",
    ] {
        assert_eq!(
            window
                .debug_bounds(selector)
                .unwrap_or_else(|| panic!("rendered caption asset {selector}"))
                .size,
            size(px(12.0), px(12.0))
        );
    }
    window.simulate_mouse_move(minimize.center(), None, Modifiers::default());
    executor.advance_clock(Duration::from_millis(600));
    executor.run_until_parked();
    window.refresh().expect("render caption tooltip");
    assert!(
        window
            .debug_bounds("window-control-minimize-tooltip")
            .is_some(),
        "caption control exposes its visible tooltip"
    );

    window.simulate_click(minimize.center(), Modifiers::default());
    let minimize_envelope = endpoint
        .try_receive_command_for_contract_test()
        .expect("typed minimize command");
    assert!(matches!(
        minimize_envelope.message,
        Message::PlatformCommand(PlatformCommand::SetWindowMinimized(_))
    ));

    window.simulate_click(maximize.center(), Modifiers::default());
    let maximize_envelope = endpoint
        .try_receive_command_for_contract_test()
        .expect("typed maximize command");
    assert!(matches!(
        maximize_envelope.message,
        Message::PlatformCommand(PlatformCommand::ToggleWindowMaximized(_))
    ));
    let second_toggle = window
        .debug_bounds("window-control-maximize")
        .expect("maximize toggle control remains until native state changes");
    window.simulate_click(second_toggle.center(), Modifiers::default());
    let second_toggle_envelope = endpoint
        .try_receive_command_for_contract_test()
        .expect("typed second toggle envelope");
    assert!(matches!(
        second_toggle_envelope.message,
        Message::PlatformCommand(PlatformCommand::ToggleWindowMaximized(_))
    ));

    window.simulate_click(close.center(), Modifiers::default());
    let close_envelope = endpoint
        .try_receive_command_for_contract_test()
        .expect("typed close-to-tray command");
    assert!(matches!(
        close_envelope.message,
        Message::PlatformCommand(PlatformCommand::SetWindowVisibility(ref command))
            if !command.visible
    ));
    window.update(|window, cx| {
        main_window.update(cx, |_, _| ());
        window.activate_window();
        main_window.update(cx, |_, _| ());
    });
}

#[test]
fn download_multi_selection_matches_windows_ctrl_and_shift_conventions() {
    let tasks = (0..4).map(|_| DownloadTaskId::new()).collect::<Vec<_>>();
    let mut selected = HashSet::new();
    let mut primary = None;
    let mut anchor = None;

    update_download_selection(
        &mut selected,
        &mut primary,
        &mut anchor,
        &tasks,
        tasks[1],
        false,
        false,
    );
    update_download_selection(
        &mut selected,
        &mut primary,
        &mut anchor,
        &tasks,
        tasks[3],
        false,
        true,
    );
    assert_eq!(selected, HashSet::from([tasks[1], tasks[2], tasks[3]]));

    update_download_selection(
        &mut selected,
        &mut primary,
        &mut anchor,
        &tasks,
        tasks[2],
        true,
        false,
    );
    assert!(!selected.contains(&tasks[2]));
    assert_eq!(primary, Some(tasks[1]));
    assert_eq!(anchor, Some(tasks[2]));

    selected.clear();
    selected.insert(tasks[0]);
    primary = Some(tasks[0]);
    anchor = Some(tasks[0]);
    update_download_selection(
        &mut selected,
        &mut primary,
        &mut anchor,
        &tasks,
        tasks[0],
        true,
        false,
    );
    assert!(selected.is_empty());
    assert_eq!(primary, None);
}

use super::notifications::notification_transition;
use super::shell::{MainWindowControl, maximize_control};
use super::{
    DOWNLOAD_BATCH_TOOLBAR_HEIGHT, DOWNLOAD_DETAIL_HEIGHT, DOWNLOAD_FILTER_STRIP_HEIGHT,
    DOWNLOAD_TABLE_HEADER_HEIGHT, DOWNLOAD_TASK_ROW_HEIGHT, DownloadContextCapabilities,
    DownloadContextMenuPlan, DownloadDetailTab, DownloadSortKey, DownloadSortState,
    DownloadTaskFilter, MainWindow, MainWindowShellContext, QuickLaunchSettingsPage,
    download_context_capabilities, download_context_menu_plan, download_filter_count_label,
    download_filter_counts, download_queue_placement_capabilities, download_tasks_for_view,
    file_index_summary, format_query_elapsed, hotkey_from_key_down, masked_request_header_value,
    prepare_download_context_selection, quick_find_scroll_step, quick_find_scroll_target,
    reconcile_download_selection, set_download_sort, toggle_indexed_volume_selection,
    update_download_selection,
};
use crate::ui::metrics::{
    MODULE_HEADER_HEIGHT, SIDEBAR_ROW_HEIGHT, SIDEBAR_WIDTH, SPACING_4, SPACING_12, TITLEBAR_HEIGHT,
};
use crate::ui::typography::{LINE_HEIGHT_BODY, LINE_HEIGHT_LABEL, LINE_HEIGHT_TITLE};

#[test]
fn maximize_state_selects_a_distinct_restore_caption_asset() {
    let maximize = maximize_control(false);
    let restore = maximize_control(true);

    assert_eq!(maximize, MainWindowControl::Maximize);
    assert_eq!(restore, MainWindowControl::Restore);
    assert_ne!(maximize.icon(), restore.icon());
    assert_ne!(maximize.id(), restore.id());
    assert_eq!(MainWindowControl::Minimize.tooltip(), "最小化");
    assert_eq!(maximize.tooltip(), "最大化");
    assert_eq!(restore.tooltip(), "还原");
    assert_eq!(MainWindowControl::Close.tooltip(), "关闭");
}

#[test]
fn download_workspace_uses_the_compact_editor_density_scale() {
    assert_eq!(MODULE_HEADER_HEIGHT, 48.0);
    assert_eq!(DOWNLOAD_FILTER_STRIP_HEIGHT, 38.0);
    assert_eq!(DOWNLOAD_TABLE_HEADER_HEIGHT, 28.0);
    assert_eq!(DOWNLOAD_TASK_ROW_HEIGHT, 46.0);
    assert_eq!(DOWNLOAD_BATCH_TOOLBAR_HEIGHT, 36.0);
    assert_eq!(DOWNLOAD_DETAIL_HEIGHT, 176.0);
}

#[test]
fn download_detail_masks_credential_headers_but_keeps_ordinary_values() {
    assert_eq!(
        masked_request_header_value("Authorization", "Bearer private"),
        "已设置（内容已隐藏）"
    );
    assert_eq!(
        masked_request_header_value("X-API-Key", "private"),
        "已设置（内容已隐藏）"
    );
    assert_eq!(
        masked_request_header_value("X-Auth-Token", "private"),
        "已设置（内容已隐藏）"
    );
    assert_eq!(
        masked_request_header_value("X-ApiKey", "private"),
        "已设置（内容已隐藏）"
    );
    assert_eq!(
        masked_request_header_value("X_API_KEY", "private"),
        "已设置（内容已隐藏）"
    );
    assert_eq!(
        masked_request_header_value("User-Agent", "NxTools-Test"),
        "NxTools-Test"
    );
}

#[test]
fn selected_download_detail_query_is_deferred_across_the_gpui_entity_boundary() {
    let source = include_str!("../main_window/download_page.rs");
    let downloads = source
        .split("fn render_downloads")
        .nth(1)
        .and_then(|source| source.split("fn render_quick_find").next())
        .expect("download render source");
    let defer = downloads
        .find("cx.defer(move |cx|")
        .expect("deferred detail query");
    let request = downloads
        .find("application.request_task_detail(task_id, cx)")
        .expect("typed detail request");

    assert!(defer < request);
    let detail_renderer = downloads
        .split("pub(crate) fn download_task_detail")
        .nth(1)
        .expect("download task detail renderer");
    assert!(
        detail_renderer.contains("let full_task = detail.map_or(task, |detail| &detail.task)"),
        "selected task detail must supply detail-only fields without replacing summary state"
    );
    assert!(
        detail_renderer.contains("download_overview_detail(theme, task, detail"),
        "overview must combine selected detail fields with the compact summary"
    );
    assert!(
        detail_renderer.contains("download_header_detail(theme, task, detail"),
        "headers must use the selected task detail"
    );
    assert!(
        !downloads[..defer].contains("application.request_task_detail(task_id, cx)"),
        "render must not synchronously re-enter DownloadApplication"
    );
}

#[test]
fn download_workspace_is_one_filter_strip_flat_table_and_docked_inspector() {
    let source = include_str!("../main_window/download_page.rs");
    let production = source
        .split("#[cfg(test)]")
        .next()
        .expect("production main-window source");
    let downloads = source
        .split("fn render_downloads")
        .nth(1)
        .and_then(|source| source.split("fn render_quick_find").next())
        .expect("download workspace source");
    let row = source
        .split("fn download_task_row")
        .nth(1)
        .and_then(|source| source.split("fn download_task_detail").next())
        .expect("download task row source");
    let detail = source
        .split("fn download_task_detail")
        .nth(1)
        .and_then(|source| source.split("fn download_detail_line").next())
        .expect("download detail source");

    assert!(downloads.contains("download-filter-strip"));
    assert!(downloads.contains("download_table_header("));
    assert!(production.contains("download-table-header"));
    assert!(!downloads.contains("download-filter-sidebar"));
    assert!(row.contains(".border_b_1()"));
    assert!(row.contains(".border_l_2()"));
    assert!(!row.contains(".rounded(px(RADIUS_FLOATING))"));
    assert!(!row.contains("Button::new"));
    assert!(!row.contains("DOWNLOAD_ACTIONS_COLUMN_WIDTH"));
    assert!(
        !row.contains(".truncate()"),
        "nested percentage-width text must not collapse to an ellipsis-only layout"
    );
    assert!(row.contains(".whitespace_nowrap()"));
    assert!(!production.contains(".child(\"操作\")"));
    assert!(detail.contains(".border_t_1()"));
    assert!(!detail.contains(".rounded(px(RADIUS_FLOATING))"));
    assert!(!detail.contains("\"无错误\""));
}

#[test]
fn download_table_is_virtualized_and_keeps_one_scroll_handle() {
    let source = include_str!("../main_window.rs");
    let production = source
        .split("#[cfg(test)]")
        .next()
        .expect("production main-window source");
    let downloads = include_str!("../main_window/download_page.rs")
        .split("fn render_downloads")
        .nth(1)
        .and_then(|source| source.split("fn render_quick_find").next())
        .expect("download workspace source");

    assert!(production.contains("UniformListScrollHandle"));
    assert!(downloads.contains("uniform_list("));
    assert!(downloads.contains("track_scroll(self.download_list_scroll.clone())"));
    assert!(!downloads.contains(".children(task_rows)"));
}

#[test]
fn download_search_sort_and_context_selection_follow_reference_semantics() {
    let engine = DownloadEngine::open_in_memory().expect("open in-memory download engine");
    let first = engine
        .enqueue(
            DownloadRequest {
                source_url: "https://example.com/releases/Zeta.bin".into(),
                save_directory: r"C:\Downloads".into(),
                save_directory_explicit: false,
                file_name: Some("Zeta.bin".into()),
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
        .expect("enqueue first");
    let second = engine
        .enqueue(
            DownloadRequest {
                source_url: "https://mirror.invalid/alpha.bin".into(),
                save_directory: r"C:\Downloads".into(),
                save_directory_explicit: false,
                file_name: Some("Alpha.bin".into()),
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
        .expect("enqueue second");
    let tasks = engine.tasks().expect("read tasks");

    let queue_order = download_tasks_for_view(
        &tasks,
        DownloadTaskFilter::All,
        "",
        DownloadSortState::default(),
    );
    assert_eq!(
        queue_order
            .iter()
            .map(|task| task.task.id())
            .collect::<Vec<_>>(),
        vec![first, second]
    );
    let search = download_tasks_for_view(
        &tasks,
        DownloadTaskFilter::All,
        "EXAMPLE.COM/RELEASES",
        DownloadSortState::default(),
    );
    assert_eq!(search.len(), 1);
    assert_eq!(search[0].task.id(), first);
    let name_order = download_tasks_for_view(
        &tasks,
        DownloadTaskFilter::All,
        "",
        DownloadSortState {
            key: DownloadSortKey::Name,
            descending: false,
        },
    );
    assert_eq!(
        name_order
            .iter()
            .map(|task| task.task.id())
            .collect::<Vec<_>>(),
        vec![second, first]
    );
    let mut sort = DownloadSortState::default();
    assert!(set_download_sort(&mut sort, DownloadSortKey::Name, true));
    assert_eq!(
        sort,
        DownloadSortState {
            key: DownloadSortKey::Name,
            descending: false,
        }
    );
    assert!(set_download_sort(&mut sort, DownloadSortKey::Name, true));
    assert!(sort.descending);

    let visible = vec![first, second];
    let mut selected = HashSet::from([first]);
    let mut primary = Some(first);
    let mut anchor = Some(first);
    assert_eq!(
        prepare_download_context_selection(
            &mut selected,
            &mut primary,
            &mut anchor,
            &visible,
            second,
        ),
        vec![second]
    );
    selected.insert(first);
    assert_eq!(
        prepare_download_context_selection(
            &mut selected,
            &mut primary,
            &mut anchor,
            &visible,
            second,
        ),
        vec![first, second]
    );

    let mut unfinished_with_target = tasks[0].clone();
    unfinished_with_target.final_path = Some(r"C:\Downloads\Zeta.bin".into());
    let capabilities = download_context_capabilities(&[unfinished_with_target], &[first]);
    assert_eq!(
        capabilities.single_artifact, None,
        "a resolved destination is not yet a committed final artifact"
    );

    let mut paused = tasks[0].clone();
    paused
        .task
        .transition_to(DownloadStatus::Paused, 1)
        .expect("advance fixture to paused");
    paused.queue_eligible = true;
    let paused_capabilities = download_context_capabilities(&[paused], &[first]);
    assert_eq!(paused_capabilities.resume_targets, vec![first]);
    assert!(paused_capabilities.pause_targets.is_empty());
    assert_eq!(paused_capabilities.cancel_targets, vec![first]);
    assert!(paused_capabilities.can_move_queue);
    assert!(paused_capabilities.can_remove);

    let mut failed = tasks[0].clone();
    failed
        .task
        .transition_to(DownloadStatus::Probing, 1)
        .expect("advance failed fixture to probing");
    failed
        .task
        .transition_to(DownloadStatus::Failed, 2)
        .expect("advance fixture to failed");
    failed.error = Some(DownloadFault {
        code: DownloadFaultCode::Network,
        message: "temporary network failure".into(),
        retryable: true,
    });
    failed.queue_eligible = true;
    let failed_capabilities =
        download_context_capabilities(std::slice::from_ref(&failed), &[first]);
    assert_eq!(failed_capabilities.retry_targets, vec![first]);
    assert!(failed_capabilities.pause_targets.is_empty());
    assert!(failed_capabilities.cancel_targets.is_empty());
    assert!(!failed_capabilities.can_move_queue);
    assert!(failed_capabilities.can_remove);

    let mut downloading = tasks[1].clone();
    for status in [
        DownloadStatus::Probing,
        DownloadStatus::Waiting,
        DownloadStatus::Downloading,
    ] {
        downloading
            .task
            .transition_to(status, 1)
            .expect("advance fixture to downloading");
    }
    downloading.queue_eligible = true;
    let downloading_capabilities =
        download_context_capabilities(std::slice::from_ref(&downloading), &[second]);
    assert!(downloading_capabilities.resume_targets.is_empty());
    assert_eq!(downloading_capabilities.pause_targets, vec![second]);
    assert_eq!(downloading_capabilities.cancel_targets, vec![second]);
    assert!(downloading_capabilities.can_move_queue);
    assert!(!downloading_capabilities.can_remove);

    let mixed_capabilities =
        download_context_capabilities(&[failed, downloading], &[first, second]);
    assert!(mixed_capabilities.resume_targets.is_empty());
    assert_eq!(mixed_capabilities.retry_targets, vec![first]);
    assert_eq!(mixed_capabilities.pause_targets, vec![second]);
    assert_eq!(mixed_capabilities.cancel_targets, vec![second]);
    assert!(!mixed_capabilities.can_move_queue);
    assert!(!mixed_capabilities.can_remove);

    let mut completed = tasks[0].clone();
    for status in [
        DownloadStatus::Probing,
        DownloadStatus::Waiting,
        DownloadStatus::Downloading,
        DownloadStatus::Finalizing,
        DownloadStatus::Completed,
    ] {
        completed
            .task
            .transition_to(status, 1)
            .expect("advance fixture to completed");
    }
    completed.final_path = Some(r"C:\Downloads\Zeta.bin".into());
    completed.has_final_artifact = true;
    completed.queue_eligible = true;
    let completed_capabilities = download_context_capabilities(&[completed], &[first]);
    assert_eq!(
        download_context_menu_plan(&completed_capabilities, 1),
        DownloadContextMenuPlan {
            artifact: true,
            copy_url: true,
            task_window: true,
            resume: false,
            redownload: true,
            pause: false,
            cancel: false,
            remove: true,
            ..Default::default()
        },
        "completed tasks must not retain lifecycle or queue actions in the menu"
    );
}

#[test]
fn download_context_menu_omits_actions_that_cannot_run_for_the_selection() {
    let active_id = DownloadTaskId::new();
    let active = DownloadContextCapabilities {
        pause_targets: vec![active_id],
        cancel_targets: vec![active_id],
        can_move_queue: true,
        queue_top: true,
        queue_up: true,
        queue_down: true,
        queue_bottom: true,
        ..Default::default()
    };
    assert_eq!(
        download_context_menu_plan(&active, 1),
        DownloadContextMenuPlan {
            copy_url: true,
            task_window: true,
            pause: true,
            cancel: true,
            queue_top: true,
            queue_up: true,
            queue_down: true,
            queue_bottom: true,
            ..Default::default()
        }
    );

    let failed_id = DownloadTaskId::new();
    let failed = DownloadContextCapabilities {
        retry_targets: vec![failed_id],
        can_remove: true,
        ..Default::default()
    };
    assert_eq!(
        download_context_menu_plan(&failed, 1),
        DownloadContextMenuPlan {
            copy_url: true,
            task_window: true,
            retry: true,
            remove: true,
            ..Default::default()
        }
    );

    let paused_id = DownloadTaskId::new();
    let mixed_selection = DownloadContextCapabilities {
        retry_targets: vec![failed_id],
        resume_targets: vec![paused_id],
        pause_targets: vec![active_id],
        cancel_targets: vec![active_id, paused_id],
        ..Default::default()
    };
    assert_eq!(
        download_context_menu_plan(&mixed_selection, 2),
        DownloadContextMenuPlan {
            copy_url: true,
            resume: true,
            retry: true,
            pause: true,
            cancel: true,
            ..Default::default()
        },
        "multi-selection has no single-task or queue-only actions unless every task supports them"
    );

    let source = include_str!("../main_window/download_page.rs");
    let menu = source
        .split("fn download_task_context_menu(")
        .nth(1)
        .and_then(|body| body.split("fn download_task_detail(").next())
        .expect("download task context menu");
    assert!(
        !menu.contains(".disabled("),
        "unavailable download actions must be omitted instead of rendered disabled"
    );
}

#[test]
fn queue_position_actions_are_omitted_at_the_real_edges() {
    let first = DownloadTaskId::new();
    let middle = DownloadTaskId::new();
    let last = DownloadTaskId::new();
    let order = [first, middle, last];

    assert_eq!(
        download_queue_placement_capabilities(&order, &[first]),
        (false, false, true, true)
    );
    assert_eq!(
        download_queue_placement_capabilities(&order, &[middle]),
        (true, true, true, true)
    );
    assert_eq!(
        download_queue_placement_capabilities(&order, &[last]),
        (true, true, false, false)
    );
    assert_eq!(
        download_queue_placement_capabilities(&order, &[first, last]),
        (false, false, false, false)
    );
}

#[gpui::test]
fn completed_download_context_menu_renders_only_terminal_actions(cx: &mut TestAppContext) {
    cx.update(gpui_component::init);
    let engine = DownloadEngine::open_in_memory().expect("open in-memory download engine");
    let task_id = engine
        .enqueue(
            DownloadRequest {
                source_url: "https://example.com/completed.bin".into(),
                save_directory: r"C:\Downloads".into(),
                save_directory_explicit: false,
                file_name: Some("completed.bin".into()),
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
        .expect("enqueue completed-menu fixture");
    let mut completed = engine
        .tasks()
        .expect("load completed-menu fixture")
        .into_iter()
        .next()
        .expect("completed-menu task");
    for status in [
        DownloadStatus::Probing,
        DownloadStatus::Waiting,
        DownloadStatus::Downloading,
        DownloadStatus::Finalizing,
        DownloadStatus::Completed,
    ] {
        completed
            .task
            .transition_to(status, 1)
            .expect("advance completed-menu fixture");
    }
    completed.final_path = Some(r"C:\Downloads\completed.bin".into());
    completed.has_final_artifact = true;
    completed.queue_eligible = true;

    let (platform, platform_endpoint) = nxtools_ipc::platform_bridge(16);
    let notifications = cx.update(|cx| cx.new(|_| NotificationCenter::default()));
    let downloads = cx.update(|cx| {
        cx.new(|cx| {
            DownloadApplication::new(
                engine,
                std::rc::Rc::new(platform),
                notifications.clone(),
                DownloadRuntimeContext {
                    fallback_default_directory: r"C:\Downloads".into(),
                    config: AppConfig::default(),
                    config_path: std::env::temp_dir().join("nxtools-test-config.json"),
                    log_directory: std::env::temp_dir(),
                    database_path: std::env::temp_dir().join("downloads.sqlite3"),
                    ffmpeg_install_directory: std::env::temp_dir()
                        .join("nxtools-test-tools")
                        .join("ffmpeg"),
                },
                cx,
            )
        })
    });
    let downloads_for_window = downloads.downgrade();
    let (main_window, window) = cx.add_window_view(move |window, cx| {
        let mut main_window = MainWindow::new(
            gpui::WeakEntity::<ClipboardApplication>::new_invalid(),
            gpui::WeakEntity::<QuickLaunchApplication>::new_invalid(),
            downloads_for_window,
            MainWindowShellContext {
                settings: None,
                platform: None,
                system_settings: Rc::new(RefCell::new(SystemSettings::default())),
            },
            notifications,
            window,
            cx,
        );
        main_window.shell.activate(MainModule::Downloads);
        main_window
    });
    window.update(|_, cx| {
        downloads.update(cx, |application, _| {
            application.replace_tasks_for_test(vec![completed]);
        });
    });
    window.refresh().expect("render completed download fixture");
    window.run_until_parked();
    let element_id = task_id.as_uuid().to_string();
    let row_selector: &'static str =
        Box::leak(format!("download-task-row-{element_id}").into_boxed_str());
    let row = window
        .debug_bounds(row_selector)
        .expect("completed production task row bounds");

    window.simulate_mouse_down(row.center(), MouseButton::Right, Modifiers::default());
    window
        .refresh()
        .expect("refresh completed task context menu");
    window.run_until_parked();

    let visible_selectors = [
        "download-menu-open-file",
        "download-menu-reveal-file",
        "download-menu-task-window",
        "download-menu-delete-file",
        "download-menu-remove",
    ];
    let mut item_width = None;
    let mut open_file_bounds = None;
    for selector in visible_selectors {
        let bounds = window
            .debug_bounds(selector)
            .unwrap_or_else(|| panic!("completed task menu must render {selector}"));
        assert_eq!(
            bounds.size.height,
            px(28.0),
            "shared menu actions must keep the 28 px action rhythm: {selector}"
        );
        assert!(
            (px(160.0)..=px(216.0)).contains(&bounds.size.width),
            "the bounded 192–240 px menu shell must leave a stable content width: {selector}={bounds:?}"
        );
        if let Some(expected) = item_width {
            assert_eq!(
                bounds.size.width, expected,
                "all shared menu actions must use one icon/text grid"
            );
        } else {
            item_width = Some(bounds.size.width);
        }
        if selector == "download-menu-open-file" {
            open_file_bounds = Some(bounds);
        }
    }
    for selector in [
        "download-menu-resume",
        "download-menu-pause",
        "download-menu-cancel",
        "download-menu-prioritize",
        "download-menu-move-up",
        "download-menu-move-down",
        "download-menu-move-bottom",
    ] {
        assert!(
            window.debug_bounds(selector).is_none(),
            "completed task menu must omit {selector}"
        );
    }

    window.simulate_keystrokes("escape");
    window.run_until_parked();
    window
        .refresh()
        .expect("refresh dismissed completed task context menu");
    window.run_until_parked();
    window
        .refresh()
        .expect("settle dismissed completed task context menu");
    window.update(|window, cx| {
        assert!(
            main_window.read(cx).hotkey_focus.is_focused(window),
            "dismissal must return keyboard focus to the invoking workspace"
        );
    });
    while platform_endpoint
        .try_receive_command_for_contract_test()
        .is_ok()
    {}
    window.simulate_click(
        open_file_bounds
            .expect("open-file action bounds before dismissal")
            .center(),
        Modifiers::default(),
    );
    window.run_until_parked();
    assert!(
        platform_endpoint
            .try_receive_command_for_contract_test()
            .is_err(),
        "the old menu hit target must not dispatch after Escape dismissal"
    );
}

#[test]
fn download_status_filters_match_the_phase_three_contract() {
    assert!(DownloadTaskFilter::All.matches(DownloadStatus::Cancelled));
    for status in [
        DownloadStatus::Probing,
        DownloadStatus::Downloading,
        DownloadStatus::Finalizing,
        DownloadStatus::Verifying,
    ] {
        assert!(DownloadTaskFilter::Downloading.matches(status));
    }
    for status in [DownloadStatus::Created, DownloadStatus::Waiting] {
        assert!(DownloadTaskFilter::Waiting.matches(status));
    }
    assert!(DownloadTaskFilter::Paused.matches(DownloadStatus::Paused));
    assert!(DownloadTaskFilter::Completed.matches(DownloadStatus::Completed));
    assert!(DownloadTaskFilter::Failed.matches(DownloadStatus::Failed));

    for filter in [
        DownloadTaskFilter::Downloading,
        DownloadTaskFilter::Waiting,
        DownloadTaskFilter::Paused,
        DownloadTaskFilter::Completed,
        DownloadTaskFilter::Failed,
    ] {
        assert!(!filter.matches(DownloadStatus::Cancelled));
    }
}

#[test]
fn download_filter_counts_and_visible_selection_share_the_filtered_snapshot() {
    let counts = download_filter_counts([
        DownloadStatus::Created,
        DownloadStatus::Waiting,
        DownloadStatus::Downloading,
        DownloadStatus::Paused,
        DownloadStatus::Completed,
        DownloadStatus::Failed,
        DownloadStatus::Cancelled,
    ]);
    assert_eq!(counts.all, 7);
    assert_eq!(counts.downloading, 1);
    assert_eq!(counts.waiting, 2);
    assert_eq!(counts.paused, 1);
    assert_eq!(counts.completed, 1);
    assert_eq!(counts.failed, 1);

    let mut selected = HashSet::from([2, 3]);
    assert_eq!(
        reconcile_download_selection(Some(3), &mut selected, &[1, 2, 3], |id| *id),
        Some(3)
    );
    assert_eq!(selected, HashSet::from([2, 3]));

    assert_eq!(
        reconcile_download_selection(Some(3), &mut selected, &[1, 2], |id| *id),
        Some(2),
        "a still-visible member of the multi-selection becomes primary"
    );
    assert_eq!(selected, HashSet::from([2]));

    assert_eq!(
        reconcile_download_selection::<u64, u64>(Some(2), &mut selected, &[], |id| *id),
        None
    );
    assert!(selected.is_empty());

    assert_eq!(
        reconcile_download_selection(None, &mut selected, &[1, 2, 3], |id| *id),
        None,
        "visible rows must not create a selection without an explicit user action"
    );
    assert!(selected.is_empty());
}

#[test]
fn filtered_empty_states_name_the_active_status() {
    assert_eq!(
        DownloadTaskFilter::Downloading.empty_title(),
        "暂无正在下载的任务"
    );
    assert_eq!(DownloadTaskFilter::Waiting.empty_title(), "暂无等待任务");
    assert_eq!(DownloadTaskFilter::Paused.empty_title(), "暂无暂停任务");
    assert_eq!(
        DownloadTaskFilter::Completed.empty_title(),
        "暂无已完成任务"
    );
    assert_eq!(DownloadTaskFilter::Failed.empty_title(), "暂无失败任务");
}

#[test]
fn download_filter_count_labels_stay_bounded_for_fixed_segments() {
    assert_eq!(download_filter_count_label(0), "0");
    assert_eq!(download_filter_count_label(999), "999");
    assert_eq!(download_filter_count_label(1_000), "1.0k");
    assert_eq!(download_filter_count_label(12_345), "12.3k");
    assert_eq!(download_filter_count_label(1_234_567), "1.2M");
    assert!(download_filter_count_label(usize::MAX).chars().count() <= 6);
}

#[gpui::test]
fn clicking_a_download_filter_row_changes_the_real_main_window_state(cx: &mut TestAppContext) {
    cx.update(gpui_component::init);
    let notifications = cx.update(|cx| cx.new(|_| NotificationCenter::default()));
    let (main_window, window) = cx.add_window_view(move |window, cx| {
        let mut main_window = MainWindow::new(
            gpui::WeakEntity::<ClipboardApplication>::new_invalid(),
            gpui::WeakEntity::<QuickLaunchApplication>::new_invalid(),
            gpui::WeakEntity::<DownloadApplication>::new_invalid(),
            MainWindowShellContext {
                settings: None,
                platform: None,
                system_settings: Rc::new(RefCell::new(SystemSettings::default())),
            },
            notifications,
            window,
            cx,
        );
        main_window.shell.activate(MainModule::Downloads);
        main_window
    });
    let bounds = window
        .debug_bounds("download-filter-waiting")
        .expect("rendered waiting filter bounds");
    let filter_strip_bounds = window
        .debug_bounds("download-filter-strip")
        .expect("rendered horizontal filter strip bounds");
    let filter_segments = window
        .debug_bounds("download-filter-segments")
        .expect("status filters must share one segmented-control boundary");
    let table_header_bounds = window
        .debug_bounds("download-table-header")
        .expect("rendered download table header bounds");
    assert_eq!(
        filter_strip_bounds.size.height.as_f32(),
        DOWNLOAD_FILTER_STRIP_HEIGHT
    );
    assert_eq!(
        table_header_bounds.size.height.as_f32(),
        DOWNLOAD_TABLE_HEADER_HEIGHT
    );
    assert_eq!(
        filter_strip_bounds.size.width,
        table_header_bounds.size.width
    );
    let segment_bounds = DownloadTaskFilter::ALL.map(|filter| {
        window
            .debug_bounds(filter.id())
            .unwrap_or_else(|| panic!("rendered status segment {}", filter.id()))
    });
    assert_eq!(filter_segments.size.height.as_f32(), 28.0);
    for (index, segment) in segment_bounds.iter().enumerate() {
        assert_eq!(segment.size.height, segment_bounds[0].size.height);
        assert!(
            (segment.size.width.as_f32() - segment_bounds[0].size.width.as_f32()).abs() <= 0.5,
            "pixel rounding must be the only width difference between status segments"
        );
        assert!(filter_segments.contains(&segment.origin));
        assert!(filter_segments.contains(&segment.bottom_right()));
        if index > 0 {
            assert_eq!(segment.left(), segment_bounds[index - 1].right());
        }
    }
    assert_eq!(
        segment_bounds[0].left().as_f32(),
        filter_segments.left().as_f32() + 1.0
    );
    assert_eq!(
        segment_bounds[5].right().as_f32(),
        filter_segments.right().as_f32() - 1.0
    );
    window.simulate_click(bounds.center(), Modifiers::default());

    assert_eq!(
        cx.read(|cx| main_window.read(cx).download_filter),
        DownloadTaskFilter::Waiting
    );
    assert_eq!(
        cx.read(|cx| main_window.read(cx).selected_download_task),
        None
    );
}

#[test]
fn download_table_header_clicks_persist_the_sort_preference() {
    // The user report: header-click sorts were not restored after a
    // restart because only the context-menu path persisted the choice.
    let source = include_str!("../main_window/download_page.rs");
    let header_body = source
        .split("fn download_sort_header_cell")
        .nth(1)
        .and_then(|body| body.split("\npub(crate) fn persist_download_sort").next())
        .expect("download_sort_header_cell body");
    assert!(
        header_body.contains("persist_download_sort(window, cx)"),
        "header clicks must persist the sort like the context menu does"
    );
}

#[gpui::test]
fn clicking_download_table_headers_changes_only_the_view_sort(cx: &mut TestAppContext) {
    cx.update(gpui_component::init);
    let notifications = cx.update(|cx| cx.new(|_| NotificationCenter::default()));
    let (main_window, window) = cx.add_window_view(move |window, cx| {
        let mut main_window = MainWindow::new(
            gpui::WeakEntity::<ClipboardApplication>::new_invalid(),
            gpui::WeakEntity::<QuickLaunchApplication>::new_invalid(),
            gpui::WeakEntity::<DownloadApplication>::new_invalid(),
            MainWindowShellContext {
                settings: None,
                platform: None,
                system_settings: Rc::new(RefCell::new(SystemSettings::default())),
            },
            notifications,
            window,
            cx,
        );
        main_window.shell.activate(MainModule::Downloads);
        main_window
    });
    window.simulate_resize(size(px(920.0), px(480.0)));
    window
        .refresh()
        .expect("render minimum-width download table");
    let name_header = window
        .debug_bounds("download-sort-header-name")
        .expect("rendered sortable name header");
    let created_header = window
        .debug_bounds("download-sort-header-created")
        .expect("minimum-width download table exposes the sortable created header");
    assert_eq!(
        created_header.size.width,
        px(super::DOWNLOAD_CREATED_COLUMN_WIDTH)
    );

    window.simulate_click(name_header.center(), Modifiers::default());
    window.simulate_click(name_header.center(), Modifiers::default());
    assert_eq!(
        window.read(|cx| main_window.read(cx).download_sort),
        DownloadSortState {
            key: DownloadSortKey::Name,
            descending: true,
        }
    );
    window.simulate_click(created_header.center(), Modifiers::default());
    assert_eq!(
        window.read(|cx| main_window.read(cx).download_sort),
        DownloadSortState {
            key: DownloadSortKey::Created,
            descending: true,
        }
    );
}

#[gpui::test]
fn production_download_row_renders_resolved_shell_icon_without_generic_tile(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_component::init);
    let engine = DownloadEngine::open_in_memory().expect("open download icon test engine");
    let task_id = engine
        .enqueue(
            DownloadRequest {
                source_url: "https://example.test/archive.zip".into(),
                save_directory: r"C:\Downloads".into(),
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
        .expect("enqueue download icon task");
    let (platform, _platform_endpoint) = nxtools_ipc::platform_bridge(16);
    let notifications = cx.update(|cx| cx.new(|_| NotificationCenter::default()));
    let downloads = cx.update(|cx| {
        cx.new(|cx| {
            DownloadApplication::new(
                engine,
                Rc::new(platform),
                notifications.clone(),
                DownloadRuntimeContext {
                    fallback_default_directory: r"C:\Downloads".into(),
                    config: AppConfig::default(),
                    config_path: std::env::temp_dir().join("nxtools-test-config.json"),
                    log_directory: std::env::temp_dir(),
                    database_path: std::env::temp_dir().join("downloads.sqlite3"),
                    ffmpeg_install_directory: std::env::temp_dir()
                        .join("nxtools-test-tools")
                        .join("ffmpeg"),
                },
                cx,
            )
        })
    });
    let icon_fixture = tempfile::tempdir().expect("download row icon fixture directory");
    let icon_path = icon_fixture.path().join("archive-zip.png");
    std::fs::write(&icon_path, crate::icons::ONE_PIXEL_PNG)
        .expect("write production row icon fixture");
    cx.update(|cx| {
        downloads.update(cx, |application, cx| {
            application.replace_task_icon_for_test(task_id, icon_path, cx);
        });
    });
    let downloads_for_window = downloads.downgrade();
    let (_main_window, window) = cx.add_window_view(move |window, cx| {
        let mut main_window = MainWindow::new(
            gpui::WeakEntity::<ClipboardApplication>::new_invalid(),
            gpui::WeakEntity::<QuickLaunchApplication>::new_invalid(),
            downloads_for_window,
            MainWindowShellContext {
                settings: None,
                platform: None,
                system_settings: Rc::new(RefCell::new(SystemSettings::default())),
            },
            notifications,
            window,
            cx,
        );
        main_window.shell.activate(MainModule::Downloads);
        main_window
    });
    window.simulate_resize(size(px(1060.0), px(500.0)));
    window.refresh().expect("render wide download icon row");
    let row_selector: &'static str =
        Box::leak(format!("download-task-row-{}", task_id.as_uuid()).into_boxed_str());
    let row = window
        .debug_bounds(row_selector)
        .expect("production download icon row");
    window.simulate_click(row.center(), Modifiers::default());

    let shell_icon = window
        .debug_bounds("download-task-shell-icon-selected")
        .expect("selected download row must render its resolved Shell icon");
    assert_eq!(shell_icon.size, size(px(20.0), px(20.0)));
    let created_header = window
        .debug_bounds("download-sort-header-created")
        .expect("wide task list created header");
    let created_value = window
        .debug_bounds("download-task-created-selected")
        .expect("selected task created value");
    assert_eq!(
        created_value.size.width,
        px(super::DOWNLOAD_CREATED_COLUMN_WIDTH)
    );
    assert_eq!(created_header.left(), created_value.left());
    assert_eq!(created_header.right(), created_value.right());
    assert!(
        window
            .debug_bounds("download-task-icon-fallback-selected")
            .is_none(),
        "a resolved Shell icon must replace the generic fallback"
    );
    let row_source = include_str!("../main_window/download_page.rs")
        .split("fn download_task_row(")
        .nth(1)
        .and_then(|source| source.split("fn download_popup_item(").next())
        .expect("download task row source");
    let icon_source = row_source
        .split(".when_some(icon_path, |slot, path| {")
        .nth(1)
        .and_then(|source| source.split(".when(!has_icon_path, |slot| {").next())
        .expect("resolved Shell icon slot source");
    assert!(icon_source.contains("img(path)"));
    assert!(
        !icon_source.contains(".bg("),
        "Shell icons must stay on a transparent slot"
    );
}

#[gpui::test]
fn production_download_rows_and_detail_remain_coherent_at_compact_width(cx: &mut TestAppContext) {
    cx.update(gpui_component::init);
    let executor = cx.executor();
    let engine = DownloadEngine::open_in_memory().expect("open in-memory download engine");
    let first_task = engine
        .enqueue(
            DownloadRequest {
                source_url: "https://example.com/first.bin".into(),
                save_directory: r"C:\Downloads".into(),
                save_directory_explicit: false,
                file_name: Some("first.bin".into()),
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
            },
            RequestOrigin::Internal,
        )
        .expect("enqueue first task");
    let second_task = engine
        .enqueue(
            DownloadRequest {
                source_url: "https://example.com/second.bin".into(),
                save_directory: r"C:\Downloads".into(),
                save_directory_explicit: false,
                file_name: Some("second.bin".into()),
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
            },
            RequestOrigin::Internal,
        )
        .expect("enqueue second task");
    let failure_message =
        "server returned a bounded but deliberately long failure message ".repeat(12);
    let mut task_snapshots = engine.tasks().expect("read task snapshots");
    task_snapshots
        .iter_mut()
        .find(|task| task.task.id() == first_task)
        .expect("first task snapshot")
        .error = Some(DownloadFault {
        code: DownloadFaultCode::Protocol,
        message: failure_message.clone(),
        retryable: false,
    });
    let long_success_path = std::path::PathBuf::from(
        r"C:\Users\Administrator\Downloads\Very-Long-Archive-Name\Release\2026\second-success-file-with-a-long-name.bin",
    );
    task_snapshots
        .iter_mut()
        .find(|task| task.task.id() == second_task)
        .expect("second task snapshot")
        .final_path = Some(long_success_path.clone());
    task_snapshots
        .iter_mut()
        .find(|task| task.task.id() == second_task)
        .expect("second task snapshot")
        .has_final_artifact = true;
    let (platform, _platform_endpoint) = nxtools_ipc::platform_bridge(16);
    let notifications = cx.update(|cx| cx.new(|_| NotificationCenter::default()));
    let downloads = cx.update(|cx| {
        cx.new(|cx| {
            DownloadApplication::new(
                engine,
                std::rc::Rc::new(platform),
                notifications.clone(),
                DownloadRuntimeContext {
                    fallback_default_directory: r"C:\Downloads".into(),
                    config: AppConfig::default(),
                    config_path: std::env::temp_dir().join("nxtools-test-config.json"),
                    log_directory: std::env::temp_dir(),
                    database_path: std::env::temp_dir().join("downloads.sqlite3"),
                    ffmpeg_install_directory: std::env::temp_dir()
                        .join("nxtools-test-tools")
                        .join("ffmpeg"),
                },
                cx,
            )
        })
    });
    let downloads_for_window = downloads.downgrade();
    let failed_task_snapshots = task_snapshots.clone();
    downloads.update(cx, |application, _| {
        application.replace_tasks_for_test(task_snapshots);
    });
    let (main_window, window) = cx.add_window_view(move |window, cx| {
        let mut main_window = MainWindow::new(
            gpui::WeakEntity::<ClipboardApplication>::new_invalid(),
            gpui::WeakEntity::<QuickLaunchApplication>::new_invalid(),
            downloads_for_window,
            MainWindowShellContext {
                settings: None,
                platform: None,
                system_settings: Rc::new(RefCell::new(SystemSettings::default())),
            },
            notifications,
            window,
            cx,
        );
        main_window.shell.activate(MainModule::Downloads);
        main_window
    });
    window.simulate_resize(size(px(920.0), px(480.0)));

    let authoritative_task_ids = window.read(|cx| {
        downloads
            .read(cx)
            .tasks()
            .iter()
            .map(|task| task.task.id())
            .collect::<Vec<_>>()
    });
    let filter_segments = window
        .debug_bounds("download-filter-segments")
        .expect("compact production workspace status group bounds");
    let segment_bounds = DownloadTaskFilter::ALL.map(|filter| {
        window
            .debug_bounds(filter.id())
            .unwrap_or_else(|| panic!("compact production status segment {}", filter.id()))
    });
    assert_eq!(filter_segments.size.height.as_f32(), 28.0);
    for (index, segment) in segment_bounds.iter().enumerate() {
        assert_eq!(segment.size.height, segment_bounds[0].size.height);
        assert!((segment.size.width.as_f32() - segment_bounds[0].size.width.as_f32()).abs() <= 0.5);
        if index > 0 {
            assert_eq!(segment.left(), segment_bounds[index - 1].right());
        }
    }
    window.simulate_click(segment_bounds[2].center(), Modifiers::default());
    assert_eq!(
        window.read(|cx| main_window.read(cx).download_filter),
        DownloadTaskFilter::Waiting
    );
    window.simulate_click(segment_bounds[0].center(), Modifiers::default());
    assert_eq!(
        window.read(|cx| main_window.read(cx).download_filter),
        DownloadTaskFilter::All
    );

    let page_header = window
        .debug_bounds("download-module-header")
        .expect("download page header bounds");
    let search = window
        .debug_bounds("download-search")
        .expect("download search bounds");
    let settings_action = window
        .debug_bounds("download-settings-action")
        .expect("download settings action bounds");
    let schedule_action = window
        .debug_bounds("download-schedule-action")
        .expect("download schedule action bounds");
    let new_task_action = window
        .debug_bounds("download-new-task-action")
        .expect("download new-task action bounds");
    let sort = window
        .debug_bounds("download-sort")
        .expect("download sort action bounds");
    assert!(page_header.contains(&search.origin));
    assert!(page_header.contains(&search.bottom_right()));
    assert_eq!(search.size.height.as_f32(), 28.0);
    for action in [settings_action, schedule_action, new_task_action] {
        assert_eq!(action.size.height.as_f32(), 28.0);
        assert!(page_header.contains(&action.origin));
        assert!(page_header.contains(&action.bottom_right()));
    }
    assert_eq!(sort.size.height.as_f32(), 28.0);
    assert!(
        (sort.left() - filter_segments.right()) <= px(8.0),
        "sort must remain grouped with the status filter instead of floating at the far edge"
    );

    let first_element_id = first_task.as_uuid().to_string();
    let second_element_id = second_task.as_uuid().to_string();
    let first_row_selector: &'static str =
        Box::leak(format!("download-task-row-{first_element_id}").into_boxed_str());
    let second_row_selector: &'static str =
        Box::leak(format!("download-task-row-{second_element_id}").into_boxed_str());
    let search_input = window.read(|cx| main_window.read(cx).download_search_input.clone());
    window.update(|window, cx| {
        search_input.update(cx, |input, cx| input.set_value("first", window, cx));
        main_window.update(cx, |_, cx| cx.notify());
    });
    window.run_until_parked();
    window
        .refresh()
        .expect("render filtered production task list");
    assert_eq!(
        window.read(|cx| search_input.read(cx).value().to_string()),
        "first"
    );
    assert!(window.debug_bounds(first_row_selector).is_some());
    let clear_search = window
        .debug_bounds("download-search-clear")
        .expect("non-empty search exposes the bounded clear action");
    window.simulate_click(clear_search.center(), Modifiers::default());
    window.run_until_parked();
    window.update(|_, cx| main_window.update(cx, |_, cx| cx.notify()));
    window.refresh().expect("render cleared production search");
    assert!(window.read(|cx| search_input.read(cx).value().is_empty()));
    assert!(window.debug_bounds(second_row_selector).is_some());

    window.simulate_click(sort.center(), Modifiers::default());
    window.run_until_parked();
    window.refresh().expect("open compact sort menu");
    window.simulate_keystrokes("down down enter");
    window.run_until_parked();
    assert_eq!(
        window.read(|cx| main_window.read(cx).download_sort),
        DownloadSortState {
            key: DownloadSortKey::Name,
            descending: false,
        },
        "the compact sort menu must feed the same local view sort as table headers"
    );
    assert_eq!(
        window.read(|cx| {
            downloads
                .read(cx)
                .tasks()
                .iter()
                .map(|task| task.task.id())
                .collect::<Vec<_>>()
        }),
        authoritative_task_ids,
        "filter, search and sort must not mutate the authoritative task snapshot"
    );
    assert_eq!(
        window.read(|cx| main_window.read(cx).download_detail_tab),
        DownloadDetailTab::Overview
    );
    assert_eq!(
        window.read(|cx| main_window.read(cx).selected_download_task),
        None,
        "opening the download page must not invent a task selection"
    );
    assert!(
        window.debug_bounds("download-task-detail").is_none(),
        "the detail dock must stay hidden until the user selects a task"
    );
    assert!(
        window.debug_bounds("download-batch-toolbar").is_none(),
        "empty selection must not reserve a batch toolbar row"
    );

    let selected_task = first_task;
    let selected_row_selector = first_row_selector;
    let selected_row = window
        .debug_bounds(selected_row_selector)
        .expect("production task row bounds");
    assert_eq!(
        selected_row.size.height.as_f32(),
        DOWNLOAD_TASK_ROW_HEIGHT,
        "the main table must keep its compact row density"
    );
    window.update(|_, cx| {
        downloads.update(cx, |application, _| {
            application.replace_tasks_for_test(failed_task_snapshots);
        });
        main_window.update(cx, |window, _| {
            window.requested_download_detail = Some(selected_task);
        });
    });
    window.simulate_click(selected_row.center(), Modifiers::default());
    assert_eq!(
        window.read(|cx| main_window.read(cx).selected_download_task),
        Some(selected_task)
    );
    assert_eq!(
        window.read(|cx| main_window.read(cx).selected_download_tasks.clone()),
        HashSet::from([selected_task])
    );
    let detail_error = window
        .debug_bounds("download-task-detail-error")
        .expect("selected failure detail bounds");
    let detail = window
        .debug_bounds("download-task-detail")
        .expect("production detail dock bounds");

    let detail_tabs = window
        .debug_bounds("download-detail-tabs")
        .expect("production detail tab strip bounds");
    let detail_scroll = window
        .debug_bounds("download-detail-scroll")
        .expect("production detail scroll viewport bounds");
    let overview_primary = window
        .debug_bounds("download-detail-overview-primary")
        .expect("primary overview field column bounds");
    let overview_secondary = window
        .debug_bounds("download-detail-overview-secondary")
        .expect("secondary overview field column bounds");
    let status_label = window
        .debug_bounds("download-detail-field-status-label")
        .expect("fixed overview status label bounds");
    let downloaded_label = window
        .debug_bounds("download-detail-field-downloaded-label")
        .expect("fixed overview downloaded label bounds");
    let status_value = window
        .debug_bounds("download-detail-field-status-value")
        .expect("overview status value bounds");
    let downloaded_value = window
        .debug_bounds("download-detail-field-downloaded-value")
        .expect("overview downloaded value bounds");
    assert_eq!(detail_tabs.size.height.as_f32(), 30.0);
    let tab_bounds = DownloadDetailTab::ALL.map(|tab| {
        window
            .debug_bounds(Box::leak(
                format!("download-detail-tab-{}", tab.diagnostic_name()).into_boxed_str(),
            ))
            .unwrap_or_else(|| panic!("production {:?} detail tab bounds", tab))
    });
    for tab in &tab_bounds {
        assert_eq!(tab.top(), tab_bounds[0].top());
        assert_eq!(tab.size.height.as_f32(), 29.0);
        assert!(detail_tabs.contains(&tab.origin));
        assert!(detail_tabs.contains(&tab.bottom_right()));
    }
    assert_eq!(detail_scroll.top(), detail_tabs.bottom());
    assert_eq!(detail_scroll.bottom(), detail.bottom());
    assert!(
        (overview_primary.size.width.as_f32() - overview_secondary.size.width.as_f32()).abs()
            <= 1.0,
        "overview inspector columns must split the available content width evenly"
    );
    assert_eq!(status_label.size.width.as_f32(), 52.0);
    assert_eq!(downloaded_label.size.width, status_label.size.width);
    assert_eq!(status_value.left(), downloaded_value.left());

    let segment_tab = window
        .debug_bounds("download-detail-tab-segments")
        .expect("download segment detail tab bounds after selecting a task");
    window.simulate_click(segment_tab.center(), Modifiers::default());
    assert_eq!(
        window.read(|cx| main_window.read(cx).download_detail_tab),
        DownloadDetailTab::Segments
    );
    // The detail query settles deterministically in tests: the fixture task
    // has no transfer plan, so the dock first shows the empty segments
    // state.
    window.run_until_parked();
    window.refresh().expect("render settled segment detail");
    let segments_empty = window
        .debug_bounds("download-detail-segments-empty")
        .expect("empty segment detail state");
    assert!(segments_empty.size.height >= px(72.0));

    // Reset the detail cache so the loading progress state can be asserted
    // deterministically before the injected error state.
    window.update(|_, cx| {
        downloads.update(cx, |application, cx| {
            application.clear_task_detail_for_test(cx);
        });
    });
    window.run_until_parked();
    window.refresh().expect("render detail progress state");
    let detail_progress = window
        .debug_bounds("download-detail-segments-progress")
        .expect("production detail progress state");
    let detail_progress_icon = window
        .debug_bounds("download-detail-segments-progress-icon")
        .expect("detail progress icon");
    let detail_progress_title = window
        .debug_bounds("download-detail-segments-progress-title")
        .expect("detail progress title");
    let detail_progress_description = window
        .debug_bounds("download-detail-segments-progress-description")
        .expect("detail progress description");
    assert_eq!(detail_progress.size.height, px(72.0));

    window.update(|_, cx| {
        downloads.update(cx, |application, cx| {
            application.replace_task_detail_error_for_test(
                selected_task,
                "detail fixture failure",
                cx,
            );
        });
    });
    window.run_until_parked();
    window.refresh().expect("render detail error state");
    let detail_error_state = window
        .debug_bounds("download-detail-segments-error")
        .expect("production detail error state");
    let detail_error_marker = window
        .debug_bounds("download-detail-segments-error-marker")
        .expect("detail error semantic edge");
    let detail_error_icon = window
        .debug_bounds("download-detail-segments-error-icon")
        .expect("detail error icon");
    let detail_error_title = window
        .debug_bounds("download-detail-segments-error-title")
        .expect("detail error title");
    let detail_error_description = window
        .debug_bounds("download-detail-segments-error-description")
        .expect("detail error description");
    assert_eq!(detail_error_state.size, detail_progress.size);
    assert_eq!(detail_error_marker.size.width, px(2.0));
    assert_eq!(detail_error_icon.origin.x, detail_progress_icon.origin.x);
    assert_eq!(detail_error_title.origin.x, detail_progress_title.origin.x);
    assert_eq!(
        detail_error_description.origin.x,
        detail_progress_description.origin.x
    );

    let name = window
        .debug_bounds("download-task-name-selected")
        .expect("selected name column bounds");
    let name_text = window
        .debug_bounds("download-task-name-text-selected")
        .expect("selected file-name text bounds");
    let failure_text = window
        .debug_bounds("download-task-error-text")
        .expect("failed task must replace the auxiliary path line");
    assert!(
        window
            .debug_bounds("download-task-path-text-selected")
            .is_none(),
        "a failed task must not leave the ordinary path presentation active"
    );
    let progress = window
        .debug_bounds("download-task-progress-selected")
        .expect("selected progress column bounds");
    let transferred = window
        .debug_bounds("download-task-transferred-selected")
        .expect("selected downloaded/total subcolumn bounds");
    let speed = window
        .debug_bounds("download-task-speed-selected")
        .expect("selected speed subcolumn bounds");
    let remaining = window
        .debug_bounds("download-task-remaining-selected")
        .expect("selected remaining-time subcolumn bounds");
    let progress_track = window
        .debug_bounds("download-task-progress-track-selected")
        .expect("selected progress track bounds");
    let status = window
        .debug_bounds("download-task-status-selected")
        .expect("selected status column bounds");
    let transfer_header = window
        .debug_bounds("download-transfer-header")
        .expect("transfer header bounds");
    let speed_header = window
        .debug_bounds("download-speed-header")
        .expect("speed header bounds");
    let remaining_header = window
        .debug_bounds("download-remaining-header")
        .expect("remaining header bounds");
    let status_header = window
        .debug_bounds("download-sort-header-status")
        .expect("status header bounds");
    assert!(
        name.size.width.as_f32() >= 180.0,
        "the file-name column must retain enough width to render its content"
    );
    assert_eq!(name_text.size.width, name.size.width);
    assert_eq!(failure_text.size.width, name.size.width);
    assert!(failure_text.size.height.as_f32() >= 16.0);
    assert!(name.right() <= progress.left());
    assert_eq!(transferred.left(), progress.left());
    assert_eq!(transferred.right(), speed.left());
    assert_eq!(speed.right(), remaining.left());
    assert_eq!(remaining.right(), progress.right());
    assert_eq!(progress_track.left(), progress.left());
    assert_eq!(progress_track.right(), progress.right());
    assert_eq!(progress_track.size.height.as_f32(), 3.0);
    assert_eq!(transfer_header.left(), progress.left());
    assert_eq!(speed_header.left(), speed.left());
    assert_eq!(remaining_header.left(), remaining.left());
    assert_eq!(status_header.left(), status.left());
    assert!(progress.right() <= status.left());
    assert!(detail.size.width.as_f32() > 0.0);
    assert_eq!(detail.size.height.as_f32(), DOWNLOAD_DETAIL_HEIGHT);
    assert!(detail_error.size.width.as_f32() > 0.0);

    window.simulate_mouse_move(failure_text.center(), None, Modifiers::default());
    executor.advance_clock(Duration::from_millis(600));
    window.run_until_parked();
    let error_tooltip = window
        .debug_bounds("download-task-error-tooltip")
        .expect("bounded failure tooltip after hovering the auxiliary line");
    assert_eq!(
        error_tooltip.size.width.as_f32(),
        super::DOWNLOAD_ERROR_TOOLTIP_WIDTH
    );
    assert!(error_tooltip.size.height.as_f32() <= super::DOWNLOAD_ERROR_TOOLTIP_MAX_HEIGHT);
    let second_row = window
        .debug_bounds(second_row_selector)
        .expect("second long-path task row");
    window.simulate_click(second_row.center(), Modifiers::default());
    window.run_until_parked();
    window.refresh().expect("render selected success task");
    let success_path = window
        .debug_bounds("download-task-path-text-selected")
        .expect("selected success path text");
    window.simulate_mouse_move(success_path.center(), None, Modifiers::default());
    executor.advance_clock(Duration::from_millis(600));
    window.run_until_parked();
    assert!(
        window.debug_bounds("download-task-path-tooltip").is_some(),
        "a long successful path must remain available through a bounded tooltip"
    );
}

#[gpui::test]
fn production_download_batch_toolbar_appears_for_multi_select_and_dispatches_one_batch(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_component::init);
    let engine = DownloadEngine::open_in_memory().expect("open in-memory download engine");
    let request = |name: &str| DownloadRequest {
        source_url: format!("https://example.com/{name}"),
        save_directory: r"C:\Downloads".into(),
        save_directory_explicit: false,
        file_name: Some(name.into()),
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
    };
    let first = engine
        .enqueue(request("batch-first.bin"), RequestOrigin::Internal)
        .expect("enqueue first batch task");
    let second = engine
        .enqueue(request("batch-second.bin"), RequestOrigin::Internal)
        .expect("enqueue second batch task");
    let mut snapshots = engine.tasks().expect("read batch task snapshots");
    let failed_snapshot = snapshots
        .iter_mut()
        .find(|task| task.task.id() == first)
        .expect("first mixed-state task snapshot");
    failed_snapshot
        .task
        .transition_to(DownloadStatus::Probing, 1)
        .expect("advance first task to probing");
    failed_snapshot
        .task
        .transition_to(DownloadStatus::Failed, 2)
        .expect("advance first task to failed");
    failed_snapshot.error = Some(DownloadFault {
        code: DownloadFaultCode::Network,
        message: "temporary network failure".into(),
        retryable: true,
    });
    let downloading_snapshot = snapshots
        .iter_mut()
        .find(|task| task.task.id() == second)
        .expect("second mixed-state task snapshot");
    for (status, revision) in [
        (DownloadStatus::Probing, 1),
        (DownloadStatus::Waiting, 2),
        (DownloadStatus::Downloading, 3),
    ] {
        downloading_snapshot
            .task
            .transition_to(status, revision)
            .expect("advance second task to downloading");
    }
    let mixed_snapshots = snapshots.clone();
    let fixture_id = uuid::Uuid::new_v4();
    let (platform, _platform_endpoint) = nxtools_ipc::platform_bridge(16);
    let notifications = cx.update(|cx| cx.new(|_| NotificationCenter::default()));
    let downloads = cx.update(|cx| {
        cx.new(|cx| {
            DownloadApplication::new(
                engine,
                Rc::new(platform),
                notifications.clone(),
                DownloadRuntimeContext {
                    fallback_default_directory: r"C:\Downloads".into(),
                    config: AppConfig::default(),
                    config_path: std::env::temp_dir()
                        .join(format!("nxtools-batch-toolbar-config-{fixture_id}.json")),
                    log_directory: std::env::temp_dir(),
                    database_path: std::env::temp_dir()
                        .join(format!("nxtools-batch-toolbar-{fixture_id}.sqlite3")),
                    ffmpeg_install_directory: std::env::temp_dir()
                        .join("nxtools-test-tools")
                        .join("ffmpeg"),
                },
                cx,
            )
        })
    });
    downloads.update(cx, |application, _| {
        application.replace_tasks_for_test(snapshots);
    });
    let downloads_for_window = downloads.downgrade();
    let (main_window, window) = cx.add_window_view(move |window, cx| {
        let mut main_window = MainWindow::new(
            gpui::WeakEntity::<ClipboardApplication>::new_invalid(),
            gpui::WeakEntity::<QuickLaunchApplication>::new_invalid(),
            downloads_for_window,
            MainWindowShellContext {
                settings: None,
                platform: None,
                system_settings: Rc::new(RefCell::new(SystemSettings::default())),
            },
            notifications,
            window,
            cx,
        );
        main_window.shell.activate(MainModule::Downloads);
        main_window
    });
    window.simulate_resize(size(px(720.0), px(500.0)));
    window.update(|_, cx| {
        downloads.update(cx, |application, _| {
            application.replace_tasks_for_test(mixed_snapshots);
        });
        main_window.update(cx, |_, cx| cx.notify());
    });
    window.refresh().expect("render mixed-state task snapshots");

    assert!(window.debug_bounds("download-batch-toolbar").is_none());
    let first_selector: &'static str =
        Box::leak(format!("download-task-row-{}", first.as_uuid()).into_boxed_str());
    let second_selector: &'static str =
        Box::leak(format!("download-task-row-{}", second.as_uuid()).into_boxed_str());
    let first_row = window
        .debug_bounds(first_selector)
        .expect("first production task row");
    window.simulate_click(first_row.center(), Modifiers::default());
    assert!(window.debug_bounds("download-batch-toolbar").is_none());
    let second_row = window
        .debug_bounds(second_selector)
        .expect("second production task row");
    window.simulate_click(
        second_row.center(),
        Modifiers {
            control: true,
            ..Modifiers::default()
        },
    );
    window.run_until_parked();
    window.refresh().expect("render production batch toolbar");

    assert_eq!(
        window.read(|cx| main_window.read(cx).selected_download_tasks.clone()),
        HashSet::from([first, second])
    );
    let toolbar = window
        .debug_bounds("download-batch-toolbar")
        .expect("real multi-selection toolbar bounds");
    let cancel = window
        .debug_bounds("download-batch-cancel")
        .expect("mixed selection cancel action for the active subset");
    assert!(
        window.debug_bounds("download-batch-resume").is_some()
            || window.debug_bounds("download-batch-retry").is_some(),
        "mixed selection must expose the explicit continuation action"
    );
    assert!(window.debug_bounds("download-batch-pause").is_some());
    for unavailable in [
        "download-batch-queue-top",
        "download-batch-queue-up",
        "download-batch-queue-down",
        "download-batch-queue-bottom",
        "download-batch-remove",
    ] {
        assert!(
            window.debug_bounds(unavailable).is_none(),
            "mixed-state selection must omit unavailable toolbar action {unavailable}"
        );
    }
    assert_eq!(toolbar.size.height.as_f32(), DOWNLOAD_BATCH_TOOLBAR_HEIGHT);
    assert_eq!(cancel.size.height.as_f32(), 28.0);
    assert!(toolbar.contains(&cancel.origin));
    assert!(toolbar.contains(&cancel.bottom_right()));
    let header_with_toolbar = window
        .debug_bounds("download-table-header")
        .expect("table header below batch toolbar");

    window.simulate_click(cancel.center(), Modifiers::default());
    window.run_until_parked();
    assert!(window.read(|cx| {
        let tasks = downloads.read(cx).tasks();
        tasks
            .iter()
            .find(|task| task.task.id() == first)
            .is_some_and(|task| task.task.status() != DownloadStatus::Cancelled)
            && tasks
                .iter()
                .find(|task| task.task.id() == second)
                .is_some_and(|task| task.task.status() == DownloadStatus::Cancelled)
    }));
    window.update(|_, cx| main_window.update(cx, |_, cx| cx.notify()));
    window.refresh().expect("refresh terminal batch actions");
    let clear = window
        .debug_bounds("download-batch-clear")
        .expect("clear-selection action after batch completion");
    window.simulate_click(clear.center(), Modifiers::default());
    window.run_until_parked();
    window.update(|_, cx| main_window.update(cx, |_, cx| cx.notify()));
    window.refresh().expect("reclaim batch toolbar row");
    assert!(window.read(|cx| main_window.read(cx).selected_download_tasks.is_empty()));
    assert_eq!(
        window.read(|cx| main_window.read(cx).selected_download_task),
        None
    );
    let header_without_toolbar = window
        .debug_bounds("download-table-header")
        .expect("table header after clearing multi-selection");
    assert_eq!(
        header_without_toolbar.top() + px(DOWNLOAD_BATCH_TOOLBAR_HEIGHT),
        header_with_toolbar.top(),
        "single/empty selection must reclaim the complete batch toolbar height"
    );
}

#[gpui::test]
fn double_clicking_a_download_row_reuses_one_deferred_task_window(cx: &mut TestAppContext) {
    cx.update(gpui_component::init);
    let engine = DownloadEngine::open_in_memory().expect("open in-memory download engine");
    let task_id = engine
        .enqueue(
            DownloadRequest {
                source_url: "https://example.com/reopen.bin".into(),
                save_directory: r"C:\Downloads".into(),
                save_directory_explicit: false,
                file_name: Some("reopen.bin".into()),
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
        .expect("enqueue task");
    let (platform, _platform_endpoint) = nxtools_ipc::platform_bridge(16);
    let notifications = cx.update(|cx| cx.new(|_| NotificationCenter::default()));
    let downloads = cx.update(|cx| {
        cx.new(|cx| {
            DownloadApplication::new(
                engine,
                std::rc::Rc::new(platform),
                notifications.clone(),
                DownloadRuntimeContext {
                    fallback_default_directory: r"C:\Downloads".into(),
                    config: AppConfig {
                        download: DownloadSettings {
                            double_click_action:
                                DownloadRowDoubleClickActionSettings::OpenTaskWindow,
                            ..AppConfig::default().download
                        },
                        ..AppConfig::default()
                    },
                    config_path: std::env::temp_dir().join("nxtools-test-config.json"),
                    log_directory: std::env::temp_dir(),
                    database_path: std::env::temp_dir().join("downloads.sqlite3"),
                    ffmpeg_install_directory: std::env::temp_dir()
                        .join("nxtools-test-tools")
                        .join("ffmpeg"),
                },
                cx,
            )
        })
    });
    let activation_commands = cx.update(|cx| {
        downloads.update(cx, |application, _| {
            application.configure_task_window_activation_test(42)
        })
    });
    let downloads_for_window = downloads.downgrade();
    let (_main_window, window) = cx.add_window_view(move |window, cx| {
        let mut main_window = MainWindow::new(
            gpui::WeakEntity::<ClipboardApplication>::new_invalid(),
            gpui::WeakEntity::<QuickLaunchApplication>::new_invalid(),
            downloads_for_window,
            MainWindowShellContext {
                settings: None,
                platform: None,
                system_settings: Rc::new(RefCell::new(SystemSettings::default())),
            },
            notifications,
            window,
            cx,
        );
        main_window.shell.activate(MainModule::Downloads);
        main_window
    });
    let element_id = task_id.as_uuid().to_string();
    let row_selector: &'static str =
        Box::leak(format!("download-task-row-{element_id}").into_boxed_str());
    let row = window
        .debug_bounds(row_selector)
        .expect("production task row bounds");
    let double_click = |window: &mut gpui::VisualTestContext| {
        window.simulate_event(MouseDownEvent {
            button: MouseButton::Left,
            position: row.center(),
            modifiers: Modifiers::default(),
            click_count: 2,
            first_mouse: false,
        });
        window.simulate_event(MouseUpEvent {
            button: MouseButton::Left,
            position: row.center(),
            modifiers: Modifiers::default(),
            click_count: 2,
        });
        window.run_until_parked();
    };

    let initial_windows = window.windows();
    assert_eq!(initial_windows.len(), 1);
    double_click(window);
    let first_reopen_windows = window.windows();
    assert_eq!(first_reopen_windows.len(), 2);
    let task_window = *first_reopen_windows
        .iter()
        .find(|candidate| !initial_windows.contains(candidate))
        .expect("new keyed task window");
    double_click(window);
    let second_reopen_windows = window.windows();
    assert_eq!(
        second_reopen_windows.len(),
        2,
        "the second reopen must focus the keyed window instead of creating a duplicate"
    );
    assert!(
        second_reopen_windows.contains(&task_window),
        "the second reopen must retain the same keyed GPUI window"
    );
    assert_eq!(
        activation_commands.borrow().as_slice(),
        &[PlatformCommand::ActivateWindow(ActivateWindowCommand {
            window_handle: 42,
        })],
        "one in-flight native activation is reused instead of queueing duplicates"
    );
}

#[test]
fn download_task_window_entrypoints_defer_cross_entity_work() {
    let source = include_str!("../main_window/download_page.rs");
    let row = source
        .split("fn download_task_row(")
        .nth(1)
        .and_then(|body| body.split("fn download_task_context_menu(").next())
        .expect("download row renderer");
    let menu = source
        .split("fn download_task_context_menu(")
        .nth(1)
        .and_then(|body| body.split("fn download_task_detail(").next())
        .expect("download task context menu");

    for (name, body) in [("double click", row), ("context menu", menu)] {
        let request = body
            .find("application.request_task_window(task_id, cx)")
            .unwrap_or_else(|| panic!("{name} task-window request"));
        let deferred = body[..request]
            .rfind("cx.defer(move |cx|")
            .unwrap_or_else(|| panic!("{name} must defer its cross-entity update"));
        assert!(deferred < request);
    }
}

#[test]
fn download_filter_segments_are_wired_to_the_gpui_main_window() {
    let source = include_str!("../main_window/download_page.rs");
    let production = source
        .split("#[cfg(test)]")
        .next()
        .expect("production main-window source");
    let downloads = source
        .split("fn render_downloads")
        .nth(1)
        .and_then(|source| source.split("fn render_quick_find").next())
        .expect("download main-window source");

    assert!(production.contains("download-filter-waiting"));
    assert!(production.contains("download-filter-paused"));
    assert!(downloads.contains("segmented_control("));
    assert!(downloads.contains("SegmentedOption::new("));
    assert!(downloads.contains("cx.listener(move |this"));
    assert!(downloads.contains("select_download_filter("));
    assert!(downloads.contains("visible_tasks"));
}

#[test]
fn volume_toggles_materialize_all_then_collapse_back_to_automatic_all() {
    let available = vec![r"C:\".to_owned(), r"D:\".to_owned()];

    let without_c = toggle_indexed_volume_selection(&[], &available, r"C:\");
    assert_eq!(without_c, vec![r"D:\".to_owned()]);

    let all_again = toggle_indexed_volume_selection(&without_c, &available, r"C:\");
    assert!(
        all_again.is_empty(),
        "all selected is stored as automatic all"
    );

    let last_selected = vec![r"D:\".to_owned()];
    assert_eq!(
        toggle_indexed_volume_selection(&last_selected, &available, r"D:\"),
        last_selected,
        "the empty vector is reserved for automatic all, not an explicit empty scope"
    );

    let stale_and_current = vec![r"Z:\".to_owned(), r"C:\".to_owned()];
    assert!(
        toggle_indexed_volume_selection(&stale_and_current, &available, r"D:\").is_empty(),
        "all currently discovered volumes canonicalize to automatic all even with a stale mount"
    );
}

#[test]
fn index_scope_settings_use_a_checkbox_list_and_a_directory_picker_not_text_inputs() {
    let source = include_str!("../main_window.rs");
    let quick_launch = include_str!("../main_window/quick_launch_page.rs");
    let path_list = include_str!("../ui/components/path_list.rs");
    let production = source
        .split("#[cfg(test)]")
        .next()
        .expect("production main-window source");

    assert!(!production.contains("index_volumes_input"));
    assert!(!production.contains("index_exclusions_input"));
    assert!(!quick_launch.contains("index_volumes_input"));
    assert!(!quick_launch.contains("index_exclusions_input"));
    assert!(quick_launch.contains("volume_scope_setting("));
    assert!(quick_launch.contains("automatic_volume_scope_row("));
    assert!(quick_launch.contains("volume_scope_row("));
    assert!(!quick_launch.contains("quick-launch-index-all-volumes"));
    assert!(quick_launch.contains("path_exclusion_setting("));
    assert!(path_list.contains("quick-launch-add-index-exclusion"));
    assert!(quick_launch.contains("request_index_exclusion_directory"));
}

#[test]
fn quick_launch_settings_scrolls_only_the_selected_page_viewport() {
    let source = include_str!("../main_window/quick_launch_page.rs");
    let quick_find = source
        .split("fn render_quick_find")
        .nth(1)
        .and_then(|source| source.split("fn compact_setting_row(").next())
        .expect("quick-launch settings source");

    assert!(quick_find.contains("quick-launch-settings-scroll"));
    assert!(quick_find.contains("quick-launch-settings-navigation"));
    assert!(quick_find.contains("QuickLaunchSettingsPage::ALL"));
    for selector in [
        "quick-launch-settings-page-general",
        "quick-launch-settings-page-index",
        "quick-launch-settings-page-context",
        "quick-launch-settings-page-commands",
        "quick-launch-settings-page-diagnostics",
    ] {
        assert!(source.contains(selector), "missing {selector}");
    }
    assert!(quick_find.contains(".overflow_y_scroll()"));
    assert!(quick_find.contains(".on_scroll_wheel(cx.listener("));
    assert!(quick_find.contains("Scrollbar::vertical(&self.quick_find_scroll)"));
    assert!(!quick_find.contains(".vertical_scrollbar(&self.quick_find_scroll)"));
}

#[test]
fn quick_launch_wheel_scroll_accumulates_and_eases_to_a_clamped_target() {
    let first_target = quick_find_scroll_target(0.0, 0.0, 300.0, -38.0);
    let second_target = quick_find_scroll_target(-15.0, first_target, 300.0, -38.0);
    assert_eq!(first_target, -38.0);
    assert_eq!(second_target, -76.0);
    assert_eq!(
        quick_find_scroll_target(-290.0, -290.0, 300.0, -38.0),
        -300.0
    );

    let (next, done) = quick_find_scroll_step(-15.0, second_target);
    assert!(!done);
    assert!(next < -15.0 && next > second_target);
    assert_eq!(quick_find_scroll_step(-75.8, second_target), (-76.0, true));
}

#[test]
fn custom_command_program_uses_the_windows_picker() {
    let source = include_str!("../main_window/quick_launch_page.rs");
    let editor = include_str!("../ui/components/custom_command_editor.rs");
    let quick_find = source
        .split("fn render_quick_find")
        .nth(1)
        .and_then(|source| source.split("fn compact_setting_row(").next())
        .expect("quick-launch settings source");

    assert!(editor.contains("quick-launch-pick-command-program"));
    assert!(quick_find.contains("request_custom_command_program"));
}

#[test]
fn quick_launch_settings_expose_history_performance_and_diagnostics() {
    let source = include_str!("../main_window/quick_launch_page.rs");
    let diagnostics = include_str!("../ui/components/quick_launch_diagnostics.rs");
    let quick_find = source
        .split("fn render_quick_find")
        .nth(1)
        .and_then(|source| source.split("fn compact_setting_row(").next())
        .expect("quick-launch settings source");

    assert!(quick_find.contains("quick_launch_diagnostics("));
    assert!(quick_find.contains("clear_history"));
    assert!(quick_find.contains("open_log_directory"));
    assert!(diagnostics.contains("quick-launch-clear-history"));
    assert!(diagnostics.contains("quick-launch-open-logs"));
    assert!(diagnostics.contains("最近查询性能"));
    assert!(diagnostics.contains("排序学习"));
}

#[gpui::test]
fn compact_setting_descriptions_wrap_and_short_tooltips_fit_content(cx: &mut TestAppContext) {
    use gpui::prelude::*;

    struct SettingRowFixture {
        title: &'static str,
        description: &'static str,
    }

    impl gpui::Render for SettingRowFixture {
        fn render(
            &mut self,
            _window: &mut gpui::Window,
            _cx: &mut gpui::Context<Self>,
        ) -> impl IntoElement {
            let row = crate::main_window::quick_launch_page::compact_setting_row(
                crate::theme::Theme::zed_dark(),
                self.title,
                self.description,
                gpui::div().w(px(96.0)).h(px(26.0)).into_any_element(),
            );
            gpui::div().size_full().p(px(8.0)).child(row)
        }
    }

    cx.update(gpui_component::init);

    {
        let window = cx.add_empty_window();
        window.draw(
            gpui::point(px(0.0), px(0.0)),
            gpui::size(px(360.0), px(120.0)),
            |_, cx| {
                cx.new(|_| SettingRowFixture {
                    title: "short-setting",
                    description: "按住 Shift 点击历史条目时执行",
                })
            },
        );
        window.refresh().expect("render short compact setting row");

        let description = window
            .debug_bounds("short-setting")
            .expect("short setting description");
        assert!(description.size.width > px(300.0));
    }

    {
        let window = cx.add_empty_window();
        window.draw(
            gpui::point(px(0.0), px(0.0)),
            gpui::size(px(360.0), px(160.0)),
            |_, cx| {
                cx.new(|_| SettingRowFixture {
                    title: "long-setting",
                    description:
                        "收藏和置顶记录不会占用普通记录上限，过期记录会先进入回收状态，之后再按保留规则清理；当历史内容较多时，设置说明必须自动换行，不能把控件挤出设置列或截断成省略号。",
                })
            },
        );
        window.refresh().expect("render long compact setting row");

        let description = window
            .debug_bounds("long-setting")
            .expect("long setting description");
        assert!(description.size.width > px(300.0));
        assert!(description.size.height > px(20.0));
    }
    let quick_launch_source = include_str!("../main_window/quick_launch_page.rs");
    let tooltip_source = include_str!("../ui/components/tooltip.rs");
    assert!(
        quick_launch_source
            .contains("bounded_text_tooltip(\"compact-setting-description-tooltip\"")
    );
    assert!(tooltip_source.contains(".max_w(px(TOOLTIP_MAX_WIDTH))"));
    assert!(tooltip_source.contains(".max_h(px(TOOLTIP_MAX_HEIGHT))"));
    assert!(tooltip_source.contains(".whitespace_normal()"));
    assert!(!tooltip_source.contains(".w(px(360.0))"));
    assert!(!tooltip_source.contains(".h(px(170.0))"));

    let row_source = quick_launch_source
        .split("pub(crate) fn compact_setting_row(")
        .nth(1)
        .and_then(|source| source.split("pub(crate) fn compact_toggle(").next())
        .expect("compact setting row source");
    assert!(row_source.contains(".flex_col()"));
    assert!(row_source.contains(".w_full()"));
    assert!(row_source.contains(".whitespace_normal()"));
}

#[test]
fn file_index_summary_surfaces_privilege_failure_instead_of_claiming_only_volume_discovery() {
    let status = IndexStatus {
        phase: nxtools_protocol::IndexPhase::Ready,
        indexed_items: 0,
        drives: vec![nxtools_protocol::IndexDriveStatus {
            mount_point: r"C:\".into(),
            file_system: "NTFS".into(),
            indexed_items: 0,
            journal_id: None,
            next_usn: None,
        }],
        last_error: Some("Windows denied raw volume access".into()),
    };

    let (summary, badge) = file_index_summary(Some(&status));

    assert!(summary.contains("Windows denied raw volume access"));
    assert_eq!(badge, "ERROR");
}

#[test]
fn query_elapsed_summary_keeps_microseconds_readable_and_promotes_milliseconds() {
    assert_eq!(format_query_elapsed(None), "尚无样本");
    assert_eq!(format_query_elapsed(Some(240)), "240 µs");
    assert_eq!(format_query_elapsed(Some(12_345)), "12.35 ms");
}

#[test]
fn file_index_summary_keeps_a_last_good_snapshot_available_during_maintenance() {
    let status = IndexStatus {
        phase: IndexPhase::VolumesReady,
        indexed_items: 2_784_460,
        drives: Vec::new(),
        last_error: None,
    };

    let (summary, badge) = file_index_summary(Some(&status));

    assert_eq!(badge, "UPDATING");
    assert!(summary.contains("2,784,460"));
    assert!(summary.contains("仍可搜索"));
}

#[test]
fn notification_motion_has_distinct_enter_and_exit_endpoints() {
    assert_eq!(notification_transition(0.0, false), (0.0, -6.0));
    assert_eq!(notification_transition(1.0, false), (1.0, 0.0));
    assert_eq!(notification_transition(0.0, true), (1.0, 0.0));
    assert_eq!(notification_transition(1.0, true), (0.0, -4.0));
}

#[test]
fn hotkey_recording_requires_a_modifier_and_maps_windows_virtual_keys() {
    let recorded = hotkey_from_key_down(&KeyDownEvent {
        keystroke: Keystroke {
            modifiers: Modifiers {
                control: true,
                shift: true,
                ..Modifiers::default()
            },
            key: "v".into(),
            key_char: Some("V".into()),
        },
        is_held: false,
    })
    .expect("Ctrl+Shift+V must be recordable");
    assert_eq!(
        recorded.modifiers,
        vec![
            ClipboardHotKeyModifier::Control,
            ClipboardHotKeyModifier::Shift
        ]
    );
    assert_eq!(recorded.key, "V");
    assert_eq!(recorded.display, "Ctrl+Shift+V");

    assert!(
        hotkey_from_key_down(&KeyDownEvent {
            keystroke: Keystroke {
                modifiers: Modifiers::default(),
                key: "c".into(),
                key_char: Some("c".into()),
            },
            is_held: false,
        })
        .is_none()
    );
}
#[gpui::test]
fn tool_card_opens_feature_settings_and_persists_the_shared_toggle(cx: &mut TestAppContext) {
    cx.update(gpui_component::init);
    let directory = tempfile::tempdir().expect("create feature settings test directory");
    let worker = crate::clipboard_worker::ClipboardCaptureWorkerHandle::spawn(
        directory.path().join("clipboard.db"),
        directory.path().join("clipboard-images"),
        nxtools_clipboard::ClipboardRetentionPolicy {
            max_items: 100,
            retention_days: 30,
        },
        1_000,
    )
    .expect("start feature settings clipboard worker");
    let worker_shutdown = worker.clone();
    let (platform, _endpoint) = platform_bridge(8);
    let notifications = cx.update(|cx| cx.new(|_| NotificationCenter::default()));
    let mut config = AppConfig::default();
    config.clipboard.link_detector_enabled = false;
    let config_path = directory.path().join("config.json");
    let clipboard = cx.update(|cx| {
        cx.new(|cx| {
            ClipboardApplication::new(
                worker,
                Rc::new(platform),
                config,
                config_path.clone(),
                notifications.clone(),
                cx,
            )
        })
    });
    let clipboard_for_main = clipboard.downgrade();
    let notifications_for_main = notifications.clone();
    let (main_window, main_surface) = cx.add_window_view(move |window, cx| {
        let mut main_window = MainWindow::new(
            clipboard_for_main,
            gpui::WeakEntity::<QuickLaunchApplication>::new_invalid(),
            gpui::WeakEntity::<DownloadApplication>::new_invalid(),
            MainWindowShellContext {
                settings: None,
                platform: None,
                system_settings: Rc::new(RefCell::new(SystemSettings::default())),
            },
            notifications_for_main,
            window,
            cx,
        );
        main_window.shell.activate(MainModule::Tools);
        main_window
    });

    let card = main_surface
        .debug_bounds("tool-link-detector-card")
        .expect("link detector tool card");
    main_surface.simulate_click(card.center(), Modifiers::default());
    main_surface.run_until_parked();

    let first_handle = main_surface.read(|cx| {
        main_window
            .read(cx)
            .tool_settings_windows
            .get("tool-link-detector")
            .copied()
            .expect("settings window registered by tool id")
    });
    let mut settings_window = VisualTestContext::from_window(first_handle, cx);
    settings_window
        .refresh()
        .expect("render shared feature settings");
    let toggle = settings_window
        .debug_bounds("tool-settings-toggle")
        .expect("shared enabled toggle");
    assert!(
        settings_window
            .debug_bounds("tool-settings-enabled-status")
            .is_some()
    );
    assert!(
        settings_window
            .debug_bounds("tool-settings-panel-section")
            .is_some()
    );

    settings_window.simulate_click(toggle.center(), Modifiers::default());
    settings_window.run_until_parked();
    assert!(settings_window.read(|cx| clipboard.read(cx).link_detector_enabled()));
    let position = settings_window
        .debug_bounds("tool-settings-position")
        .expect("link panel position dropdown");
    settings_window.simulate_click(position.center(), Modifiers::default());
    settings_window.run_until_parked();
    let top_right = settings_window
        .debug_bounds("tool-settings-position-top-right")
        .expect("top-right position option");
    settings_window.simulate_click(top_right.center(), Modifiers::default());
    settings_window.run_until_parked();
    let saved = AppConfig::load_from(&config_path).expect("reload saved tool settings");
    assert_eq!(
        saved.clipboard.link_panel_position,
        LinkPanelPosition::TopRight
    );

    let close = settings_window
        .debug_bounds("tool-settings-close")
        .expect("shared close action");
    settings_window.simulate_click(close.center(), Modifiers::default());
    settings_window.run_until_parked();
    drop(settings_window);
    worker_shutdown
        .shutdown()
        .expect("shutdown feature settings clipboard worker");
}
#[test]
fn desktop_snapshot_projection_benchmark_uses_1000_and_10000_task_fixtures() {
    for task_count in [1_000, 10_000] {
        let mut tasks = Vec::with_capacity(task_count);
        for index in 0..task_count {
            let task = DownloadTask::new(
                DownloadRequest {
                    source_url: format!("https://snapshot.example.test/files/{index}.bin"),
                    save_directory: r"C:\Downloads".into(),
                    save_directory_explicit: false,
                    file_name: Some(format!("snapshot-{index:05}.bin")),
                    headers: Vec::new(),
                    cookie: None,
                    referrer: None,
                    certificate_policy: Default::default(),
                    connection_limit: Some(1),
                    speed_limit_bytes_per_second: None,
                    expected_total_bytes: Some(1024),
                    scheduled_at_unix_ms: None,
                    adaptive_segment_policy: None,
                    media: None,
                    category_id: None,
                    category_name: None,
                    duplicate_strategy: nxtools_download::DownloadDuplicateStrategy::UseDefault,
                },
                RequestOrigin::Internal,
                1_000 + index as i64,
            );
            tasks.push(DownloadEngineTask {
                task,
                probe: None,
                downloaded_bytes: index as u64,
                total_bytes: Some(1024),
                error: None,
                final_path: None,
                temporary_path: None,
                queue_eligible: false,
                has_final_artifact: false,
                scheduled: false,
            });
        }

        let mut samples = Vec::with_capacity(7);
        let mut visible_count = 0;
        for _ in 0..7 {
            let started = Instant::now();
            let visible = download_tasks_for_view(
                &tasks,
                DownloadTaskFilter::All,
                "",
                DownloadSortState {
                    key: DownloadSortKey::Name,
                    descending: false,
                },
            );
            let counts = download_filter_counts(tasks.iter().map(|task| task.task.status()));
            visible_count = visible.len();
            assert_eq!(counts.all, task_count);
            assert_eq!(visible_count, task_count);
            samples.push(started.elapsed().as_micros());
        }
        samples.sort_unstable();
        let median = samples[samples.len() / 2];
        let p95 = samples[((samples.len() * 95).div_ceil(100)).saturating_sub(1)];
        println!(
            "desktop_snapshot_projection_benchmark task_count={task_count} visible_count={visible_count} projection_us_median={median} projection_us_p95={p95}"
        );
    }
}
