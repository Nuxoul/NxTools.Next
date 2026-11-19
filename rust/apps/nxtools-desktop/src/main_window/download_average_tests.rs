//! R4 real list-row regressions; no network, native Shell or user downloads.
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

use gpui_kit::{AppContext, TestAppContext, px, size};
use nxtools_core::SystemSettings;
use nxtools_download::{DownloadEngineTask, DownloadProgress, DownloadTaskId};
use serde_json::json;

use super::*;

thread_local! {
    // Armed only by these tests. Observes the exact string passed to the real
    // row's speed child; does not recompute the presenter or change its output.
    static ROW_RATES: RefCell<Option<HashMap<DownloadTaskId, String>>> = const { RefCell::new(None) };
}

pub(super) fn observe_row_rate(task_id: DownloadTaskId, rate: &str) {
    ROW_RATES.with(|rates| {
        if let Some(rates) = rates.borrow_mut().as_mut() {
            rates.insert(task_id, rate.to_owned());
        }
    });
}

struct RowRateObservation;

impl RowRateObservation {
    fn start() -> Self {
        ROW_RATES.with(|rates| *rates.borrow_mut() = Some(HashMap::new()));
        Self
    }

    fn rate(&self, task_id: DownloadTaskId) -> String {
        ROW_RATES.with(|rates| {
            rates
                .borrow()
                .as_ref()
                .and_then(|rates| rates.get(&task_id))
                .cloned()
                .expect("real production row reached speed-cell composition")
        })
    }
}

impl Drop for RowRateObservation {
    fn drop(&mut self) {
        ROW_RATES.with(|rates| *rates.borrow_mut() = None);
    }
}

fn completed_fixture(statistics: Option<serde_json::Value>) -> DownloadEngineTask {
    let task_id = DownloadTaskId::new();
    let mut wire = json!({
        "task": {
            "id": task_id,
            "request": {
                "source_url": "http://127.0.0.1:9/not-requested",
                "save_directory": ".",
                "save_directory_explicit": true,
                "file_name": "average-fixture.bin",
                "headers": [],
                "cookie": null,
                "referrer": null,
                "connection_limit": 1,
                "speed_limit_bytes_per_second": null,
                "expected_total_bytes": 1048576,
                "scheduled_at_unix_ms": null,
                "media": null,
                "category_id": null,
                "category_name": null
            },
            "origin": "internal",
            "status": "completed",
            "created_at_unix_ms": 1000,
            "updated_at_unix_ms": 172805000,
            "completed_at_unix_ms": 172805000
        },
        "probe": null,
        "downloaded_bytes": 1048576,
        "total_bytes": 1048576,
        "error": null,
        "final_path": null,
        "temporary_path": null,
        "queue_eligible": false,
        "has_final_artifact": false,
        "scheduled": false
    });
    if let Some(statistics) = statistics {
        wire["task"]["transfer_statistics"] = statistics;
    }
    serde_json::from_value(wire).expect("existing task wire accepts optional statistics")
}

struct AverageRowView {
    main_window: gpui_kit::Entity<MainWindow>,
    task: DownloadEngineTask,
    live_rate: u64,
}

impl gpui_kit::Render for AverageRowView {
    fn render(
        &mut self,
        _: &mut gpui_kit::Window,
        _: &mut gpui_kit::Context<Self>,
    ) -> impl gpui_kit::IntoElement {
        let task_id = self.task.task.id();
        download_task_row(
            crate::theme::Theme::zed_dark(),
            gpui_kit::WeakEntity::<crate::downloads::DownloadApplication>::new_invalid(),
            self.main_window.clone(),
            DownloadRowSelection {
                selected: true,
                visible_task_ids: Arc::new(vec![task_id]),
            },
            &self.task,
            Some(&DownloadProgress {
                task_id,
                downloaded_bytes: 1,
                total_bytes: Some(9),
                bytes_per_second: self.live_rate,
                active_connections: 1,
                segments: Vec::new(),
                finishing: None,
            }),
            None,
        )
    }
}

fn render_rate(cx: &mut TestAppContext, task: DownloadEngineTask, live_rate: u64) -> String {
    cx.update(gpui_kit::init);
    let observation = RowRateObservation::start();
    let task_id = task.task.id();
    let notifications =
        cx.update(|cx| cx.new(|_| crate::notifications::NotificationCenter::default()));
    let (main_window, _) = cx.add_window_view(move |window, cx| {
        MainWindow::new(
            gpui_kit::WeakEntity::<crate::clipboard::ClipboardApplication>::new_invalid(),
            gpui_kit::WeakEntity::<crate::quick_launch::QuickLaunchApplication>::new_invalid(),
            gpui_kit::WeakEntity::<crate::downloads::DownloadApplication>::new_invalid(),
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
    let (_, window) = cx.add_window_view(move |_, _| AverageRowView {
        main_window,
        task,
        live_rate,
    });
    window.simulate_resize(size(px(920.0), px(480.0)));
    window
        .refresh()
        .expect("paint actual production download row");
    assert!(
        window
            .debug_bounds("download-task-speed-selected")
            .is_some(),
        "assert the actual selected production speed cell was painted"
    );
    observation.rate(task_id)
}

#[gpui_kit::test]
fn average_completed_list_row_uses_durable_pair_not_stale_tick_or_created_at(
    cx: &mut TestAppContext,
) {
    let task = completed_fixture(Some(json!({
        "transferred_bytes": 1048576,
        "active_download_ms": 2000
    })));
    // Two days of wall time, two seconds of measured transfer, deliberately
    // conflicting stale progress. Only durable bytes / active time is valid.
    assert_eq!(render_rate(cx, task, 99 * 1024 * 1024), "512 KB/s");
}

#[gpui_kit::test]
fn average_legacy_completed_list_row_keeps_dash_even_with_stale_live_tick(cx: &mut TestAppContext) {
    assert_eq!(
        render_rate(cx, completed_fixture(None), 99 * 1024 * 1024),
        "—"
    );
}

#[gpui_kit::test]
fn average_zero_duration_completed_list_row_does_not_fabricate_rate(cx: &mut TestAppContext) {
    let task = completed_fixture(Some(json!({
        "transferred_bytes": 1048576,
        "active_download_ms": 0
    })));
    assert_eq!(render_rate(cx, task, 99 * 1024 * 1024), "—");
}
