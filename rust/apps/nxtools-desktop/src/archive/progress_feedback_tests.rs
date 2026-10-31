//! Actual original progress surface; fake monotonic elapsed observation, never product launch.
use super::*;
use gpui_kit::{Entity, TestAppContext, VisualTestContext, px, size};
fn mount(cx: &mut TestAppContext) -> (Entity<JobProgressView>, VisualTestContext) {
    cx.update(gpui_kit::init);
    cx.executor().allow_parking();
    let capture = std::rc::Rc::new(std::cell::RefCell::new(None));
    let slot = capture.clone();
    let (_, surface) = cx.add_window_view(move |window, cx| {
        let view =
            cx.new(|cx| JobProgressView::new(Uuid::new_v4(), WeakEntity::new_invalid(), true, cx));
        *slot.borrow_mut() = Some(view.clone());
        gpui_kit::base::Root::new(view, window, cx)
    });
    let handle = surface.update(|window, _| window.window_handle());
    let surface = VisualTestContext::from_window(handle, cx);
    surface.simulate_resize(size(px(560.), px(300.)));
    (capture.borrow().clone().expect("progress"), surface)
}
fn running(elapsed_ms: u64, done: u64) -> ProgressSnapshot {
    ProgressSnapshot {
        operation: Uuid::nil(),
        generation: 1,
        continuation: None,
        state: "运行中".into(),
        kind: ArchiveJobKind::Extract,
        elapsed_ms,
        reclamation: None,
        progress: Some(ArchiveJobProgress {
            phase: "running".into(),
            bytes_done: done,
            bytes_total: Some(10_000_000),
            files_done: 0,
            files_total: None,
            current_member: None,
            current_bytes_done: None,
            current_bytes_total: None,
        }),
        progress_elapsed_ms: elapsed_ms,
        terminal: None,
        prompt: None,
        paused: false,
        location_status: None,
        pause_requested: None,
        cancel_requested: false,
        control_status: None,
        batch: None,
    }
}
fn labels(surface: &mut VisualTestContext) -> Vec<String> {
    surface.run_until_parked();
    surface.update(|window, cx| {
        window.activate_window();
        window.draw(cx).clear(cx);
        gpui_kit::base::test_support::snapshots(window)
            .iter()
            .filter_map(|node| node.label().map(str::to_owned))
            .collect()
    })
}
#[gpui_kit::test]
fn hz2_original_progress_shows_sampled_rate_and_not_a_constant_unavailable_label(
    cx: &mut TestAppContext,
) {
    let (view, mut surface) = mount(cx);
    for (elapsed, done) in [(0, 0), (1000, 1_000_000), (2000, 2_000_000)] {
        surface.update(|window, cx| {
            view.update(cx, |view, cx| {
                view.apply(running(elapsed, done), window, cx)
            })
        });
    }
    let labels = labels(&mut surface);
    assert!(
        labels.iter().any(|label| label.contains("MB/s")),
        "original progress must show measured decimal processed-byte speed after enough history: {labels:?}"
    );
}
#[test]
fn feedback_archive_decimal_units_convert_numbers_not_only_labels() {
    for (bytes, expected) in [
        (0, "0 B"),
        (999, "999 B"),
        (1000, "1.0 KB"),
        (1_000_000, "1.0 MB"),
        (1_000_000_000, "1.0 GB"),
        (1_000_000_000_000, "1.0 TB"),
    ] {
        assert_eq!(format_bytes(bytes), expected, "decimal bytes are1000-based");
    }
}

#[gpui_kit::test]
fn hz2_original_wait_pause_attempt_and_terminal_remove_stale_metrics(cx: &mut TestAppContext) {
    let (view, mut surface) = mount(cx);
    for (time, done) in [(0, 0), (1000, 1_000_000)] {
        surface
            .update(|window, cx| view.update(cx, |v, cx| v.apply(running(time, done), window, cx)));
    }
    assert!(
        labels(&mut surface)
            .iter()
            .any(|label| label.contains("MB/s"))
    );
    let mut paused = running(2000, 1_000_000);
    paused.paused = true;
    paused.state = "已暂停".into();
    surface.update(|window, cx| view.update(cx, |v, cx| v.apply(paused, window, cx)));
    assert!(
        !labels(&mut surface)
            .iter()
            .any(|label| label.contains("/s") || label.contains("预计剩余"))
    );
    let mut resumed = running(3000, 2_000_000);
    resumed.operation = Uuid::new_v4();
    surface.update(|window, cx| view.update(cx, |v, cx| v.apply(resumed, window, cx)));
    assert!(
        !labels(&mut surface)
            .iter()
            .any(|label| label.contains("/s"))
    );
    let mut ended = running(4000, 10_000_000);
    ended.terminal=Some(serde_json::from_value(serde_json::json!({"outcome":"cancelled","code":"cancelled","files_committed":1,"bytes_committed":3,"files_verified":0,"bytes_verified":0,"files_skipped":0,"cleanup_complete":false,"commit_state":"Committed","elapsed_ms":4000,"native_error_code":null,"journal":[]})).expect("partial cancelled terminal"));
    surface.update(|window, cx| view.update(cx, |v, cx| v.apply(ended, window, cx)));
    let labels = labels(&mut surface);
    assert!(!labels.iter().any(|label| label.contains("已处理")
        || label.contains("/s")
        || label.contains("预计剩余")));
    assert!(labels.iter().any(|label| label.contains("实际提交")));
}

#[gpui_kit::test]
fn hz2_minimum_default_light_dark_long_member_footer_reachable(cx: &mut TestAppContext) {
    let (view, mut surface) = mount(cx);
    for dark in [false, true] {
        surface.update(|window, cx| {
            gpui_kit::component::Theme::change(
                if dark {
                    gpui_kit::component::ThemeMode::Dark
                } else {
                    gpui_kit::component::ThemeMode::Light
                },
                Some(window),
                cx,
            )
        });
        for (width, height) in [(480., 260.), (560., 300.)] {
            surface.simulate_resize(size(px(width), px(height)));
            let mut snapshot = running(1000, 1_000_000);
            snapshot
                .progress
                .as_mut()
                .expect("processed observation")
                .current_member = Some("long-member-".repeat(300));
            surface.update(|window, cx| view.update(cx, |v, cx| v.apply(snapshot, window, cx)));
            let _ = labels(&mut surface);
            let footer = surface
                .debug_bounds("archive-progress-footer")
                .expect("rendered original footer");
            assert!(
                footer.bottom() <= px(height) && footer.top() >= px(34.),
                "footer inside actual viewport: {footer:?}"
            );
        }
    }
}

#[gpui_kit::test]
fn fb_ac01_no_worker_event_expires_visible_rate_and_eta(cx: &mut TestAppContext) {
    let (view, mut surface) = mount(cx);
    for (time, done) in [(0, 0), (1_000, 1_000_000)] {
        surface.update(|window, cx| {
            view.update(cx, |view, cx| view.apply(running(time, done), window, cx))
        });
    }
    assert!(
        labels(&mut surface)
            .iter()
            .any(|label| label.contains("MB/s"))
    );
    // Do not apply a new snapshot: the display itself must expire obsolete worker samples.
    surface
        .executor()
        .advance_clock(std::time::Duration::from_millis(6_100));
    let displayed = labels(&mut surface);
    assert!(
        !displayed
            .iter()
            .any(|label| label.contains("MB/s") || label.contains("预计剩余")),
        "no-event display retained obsolete speed/ETA: {displayed:?}"
    );
    assert!(
        surface.update(|_, cx| view.read(cx).elapsed_ms) >= 7_000,
        "elapsed must advance without inventing processed bytes"
    );
}

#[gpui_kit::test]
fn fb_hz2_long_failed_terminal_keeps_summary_and_footer_reachable(cx: &mut TestAppContext) {
    let (view, mut surface) = mount(cx);
    for dark in [false, true] {
        surface.update(|window, cx| {
            gpui_kit::component::Theme::change(
                if dark {
                    gpui_kit::component::ThemeMode::Dark
                } else {
                    gpui_kit::component::ThemeMode::Light
                },
                Some(window),
                cx,
            )
        });
        for (width, height) in [(480., 260.), (560., 300.)] {
            surface.simulate_resize(size(px(width), px(height)));
            for outcome in ["failed", "cancelled"] {
                let mut snapshot = running(9_000, 9_000_000);
                snapshot.terminal = Some(serde_json::from_value(serde_json::json!({"outcome":outcome,"code":"long-error-".repeat(500),"files_committed":2,"bytes_committed":3,"files_verified":1,"bytes_verified":2,"files_skipped":4,"cleanup_complete":false,"commit_state":"CommitOutcomeUnknown","elapsed_ms":9000,"native_error_code":5,"journal":[]})).expect("actual terminal schema"));
                surface.update(|window, cx| {
                    view.update(cx, |view, cx| view.apply(snapshot, window, cx))
                });
                let shown = labels(&mut surface);
                assert!(shown.iter().any(|label| label.contains("实际提交")));
                assert!(surface.update(|_, cx| {
                    view.read(cx).terminal.as_ref().is_some_and(|terminal| {
                        !terminal.cleanup_complete && terminal.native_error_code == Some(5)
                    })
                }));
                assert!(!shown.iter().any(|label| label.contains("MB/s")
                    || label.contains("预计剩余")
                    || label.contains("已处理")));
                let footer = surface
                    .debug_bounds("archive-progress-footer")
                    .expect("fixed original footer");
                assert!(footer.bottom() <= px(height) && footer.top() >= px(34.));
            }
        }
    }
}

#[gpui_kit::test]
fn fb_ac01_display_refresh_stops_hidden_terminal_and_background(cx: &mut TestAppContext) {
    let (view, mut surface) = mount(cx);
    for (time, done) in [(0, 0), (1_000, 1_000_000)] {
        surface.update(|window, cx| {
            view.update(cx, |view, cx| view.apply(running(time, done), window, cx))
        });
    }
    surface.run_until_parked();
    assert!(surface.update(|_, cx| view.read(cx).refresh.is_some()));
    surface.simulate_visibility_change(gpui_kit::WindowVisibility::Hidden);
    surface.run_until_parked();
    assert!(
        surface.update(|_, cx| view.read(cx).refresh.is_none()),
        "occluded window must not retain display timer"
    );
    surface
        .executor()
        .advance_clock(std::time::Duration::from_millis(6_100));
    surface.simulate_visibility_change(gpui_kit::WindowVisibility::Visible);
    assert!(
        !labels(&mut surface)
            .iter()
            .any(|label| label.contains("MB/s") || label.contains("预计剩余"))
    );
    let mut ended = running(8_000, 1_000_000);
    ended.terminal = Some(serde_json::from_value(serde_json::json!({"outcome":"failed","files_committed":0,"bytes_committed":0,"files_verified":0,"bytes_verified":0,"files_skipped":0,"cleanup_complete":false,"elapsed_ms":8000,"journal":[]})).expect("terminal"));
    surface.update(|window, cx| view.update(cx, |view, cx| view.apply(ended, window, cx)));
    assert!(
        surface.update(|_, cx| view.read(cx).refresh.is_none()),
        "terminal stops timer even with cleanup unknown"
    );
    surface.update(|window, cx| {
        view.update(cx, |view, cx| {
            view.apply(running(9_000, 1_000_000), window, cx)
        })
    });
    surface.update(|window, cx| view.update(cx, |view, cx| view.background(window, cx)));
    assert!(
        surface
            .update(|_, cx| view.read(cx).refresh.is_none() && view.read(cx).presentation_retired)
    );
}

#[gpui_kit::test]
fn fb_hz2_ui_only_refresh_unknown_totals_and_stage_switch_keep_metrics_truthful(
    cx: &mut TestAppContext,
) {
    let (view, mut surface) = mount(cx);
    for (time, done) in [(0, 0), (1_000, 1_000_000)] {
        surface.update(|window, cx| {
            view.update(cx, |view, cx| view.apply(running(time, done), window, cx))
        });
    }
    assert!(
        labels(&mut surface)
            .iter()
            .any(|label| label.contains("MB/s"))
    );
    surface
        .executor()
        .advance_clock(std::time::Duration::from_millis(6_100));
    let mut ui_refresh = running(7_100, 1_000_000);
    ui_refresh.progress_elapsed_ms = 1_000;
    surface.update(|window, cx| view.update(cx, |view, cx| view.apply(ui_refresh, window, cx)));
    assert!(
        !labels(&mut surface)
            .iter()
            .any(|label| label.contains("MB/s") || label.contains("预计剩余"))
    );
    for total in [None, Some(0)] {
        for (time, done) in [(8_000, 2_000_000), (9_100, 3_100_000)] {
            let mut snapshot = running(time, done);
            snapshot.progress.as_mut().expect("progress").bytes_total = total;
            surface
                .update(|window, cx| view.update(cx, |view, cx| view.apply(snapshot, window, cx)));
        }
        assert!(
            !labels(&mut surface)
                .iter()
                .any(|label| label.contains("预计剩余"))
        );
        assert!(surface.update(|_, cx| {
            view.read(cx)
                .progress
                .as_ref()
                .and_then(progress_percent)
                .is_none()
        }));
    }
    let mut next = running(10_100, 4_100_000);
    next.progress.as_mut().expect("progress").phase = "verify".into();
    surface.update(|window, cx| view.update(cx, |view, cx| view.apply(next, window, cx)));
    assert!(
        !labels(&mut surface)
            .iter()
            .any(|label| label.contains("MB/s") || label.contains("预计剩余"))
    );
}

#[gpui_kit::test]
fn sevenz_edit_original_progress_distinguishes_reencode_verify_commit_without_fake_total(
    cx: &mut TestAppContext,
) {
    let (view, mut surface) = mount(cx);
    for (phase, expected) in [
        ("edit_planning", "准备 7z 编辑"),
        ("edit_reading", "读取并检查原归档"),
        ("edit_reencoding", "完整解码并重编码"),
        ("edit_verifying", "校验完整候选"),
        ("edit_committing", "等待原生提交"),
    ] {
        let mut snapshot = running(1000, 4096);
        snapshot.kind = ArchiveJobKind::Modify;
        let progress = snapshot.progress.as_mut().expect("original progress");
        progress.phase = phase.into();
        progress.bytes_total = None;
        surface.update(|window, cx| view.update(cx, |view, cx| view.apply(snapshot, window, cx)));
        assert!(
            labels(&mut surface)
                .iter()
                .any(|label| label.contains(expected)),
            "{phase}: real phase not generic modifying"
        );
        view.read_with(&surface, |view, _| {
            assert!(
                view.progress
                    .as_ref()
                    .expect("progress")
                    .bytes_total
                    .is_none()
            )
        });
    }
}

#[gpui_kit::test]
fn hz5_progress_distinguishes_destructive_start_and_retained_recovery_authority(
    cx: &mut TestAppContext,
) {
    let (view, mut surface) = mount(cx);
    let mut start = running(1_000, 4_096);
    start.reclamation = Some(ArchiveReclamationMode::Start);
    surface.update(|window, cx| view.update(cx, |view, cx| view.apply(start, window, cx)));
    assert!(
        labels(&mut surface)
            .iter()
            .any(|label| label.contains("完整文件核验并发布后才释放"))
    );

    let mut cancelled = running(2_000, 4_096);
    cancelled.reclamation = Some(ArchiveReclamationMode::Continue);
    cancelled.progress = None;
    cancelled.state = "归档操作已取消".into();
    cancelled.terminal = Some(
        serde_json::from_value(serde_json::json!({
            "outcome":"cancelled",
            "code":"cancelled",
            "files_committed":1,
            "bytes_committed":4096,
            "files_verified":1,
            "bytes_verified":4096,
            "files_skipped":0,
            "cleanup_complete":true,
            "elapsed_ms":2000,
            "journal":[]
        }))
        .expect("terminal"),
    );
    surface.update(|window, cx| view.update(cx, |view, cx| view.apply(cancelled, window, cx)));
    let labels = labels(&mut surface);
    assert!(
        labels
            .iter()
            .any(|label| label.contains("已释放的数据不会恢复"))
    );
    assert!(
        labels
            .iter()
            .any(|label| label.contains("继续空间回收解压"))
    );
    assert!(
        labels
            .iter()
            .any(|label| label.contains("不能恢复完整原压缩包"))
    );
}

#[gpui_kit::test]
fn archive_progress_failure_keeps_recovery_visible_and_diagnostics_explicit(
    cx: &mut TestAppContext,
) {
    let (view, mut surface) = mount(cx);
    surface.simulate_resize(size(px(560.), px(640.)));
    let mut failed = running(2_000, 700);
    failed.state = "失败".into();
    failed.terminal = Some(
        serde_json::from_value(serde_json::json!({
            "outcome":"failed", "code":"output_io", "files_committed":1,
            "bytes_committed":700, "files_verified":0, "bytes_verified":0,
            "files_skipped":0, "cleanup_complete":false,
            "commit_state":"CommitOutcomeUnknown", "elapsed_ms":2000,
            "native_error_code":112, "journal":[]
        }))
        .expect("terminal"),
    );
    surface.update(|window, cx| view.update(cx, |view, cx| view.apply(failed, window, cx)));
    let shown = labels(&mut surface);
    assert!(
        shown
            .iter()
            .any(|label| label.contains("失败") && label.contains("检查")),
        "actionable failure must be visible: {shown:?}"
    );
    assert!(
        shown
            .iter()
            .any(|label| label.contains("已提交") && label.contains("保留"))
    );
    assert!(
        !shown
            .iter()
            .any(|label| label.contains("CommitOutcomeUnknown")
                || label.contains("output_io")
                || label.contains("112"))
    );
    let trigger = surface
        .debug_bounds("archive-progress-diagnostics-toggle")
        .expect("official disclosure");
    surface.simulate_click(trigger.center(), gpui_kit::Modifiers::default());
    let expanded = labels(&mut surface);
    assert!(
        expanded
            .iter()
            .any(|label| label.contains("CommitOutcomeUnknown"))
    );
    assert!(expanded.iter().any(|label| label.contains("112")));
    assert!(surface.debug_bounds("archive-progress-footer").is_some());
}

#[gpui_kit::test]
fn extract_progress_has_independent_current_file_and_total_bars(cx: &mut TestAppContext) {
    let (view, mut surface) = mount(cx);
    surface.simulate_resize(size(px(560.), px(260.)));
    let mut snapshot = running(1000, 4_000_000);
    let progress = snapshot.progress.as_mut().expect("running");
    progress.current_member = Some("folder/second.bin".into());
    progress.current_bytes_done = Some(500_000);
    progress.current_bytes_total = Some(2_000_000);
    surface.update(|window, cx| view.update(cx, |view, cx| view.apply(snapshot, window, cx)));
    let visible = labels(&mut surface);
    assert!(
        visible
            .iter()
            .any(|label| label.contains("当前文件") && label.contains("25%")),
        "{visible:?}"
    );
    assert!(
        visible
            .iter()
            .any(|label| label.contains("总体进度") && label.contains("40%")),
        "{visible:?}"
    );
    let current = surface
        .debug_bounds("archive-current-file-progress")
        .expect("current bar group");
    let overall = surface
        .debug_bounds("archive-overall-progress")
        .expect("total bar group");
    assert!(
        current.bottom() <= overall.top(),
        "two distinct stacked progress groups"
    );
    let footer = surface
        .debug_bounds("archive-progress-footer")
        .expect("fixed footer");
    assert!(
        overall.bottom() <= footer.top(),
        "both progress bars remain above the footer at the production default size"
    );
    let metrics = surface
        .debug_bounds("archive-progress-elapsed")
        .expect("file count and elapsed");
    assert!(
        metrics.bottom() <= footer.top(),
        "file count and elapsed must be fully visible at the default size"
    );
    let mut next = running(1100, 4_100_000);
    let progress = next.progress.as_mut().expect("running");
    progress.current_member = Some("folder/third.bin".into());
    progress.current_bytes_done = Some(100_000);
    progress.current_bytes_total = Some(1_000_000);
    surface.update(|window, cx| view.update(cx, |view, cx| view.apply(next, window, cx)));
    let visible = labels(&mut surface);
    assert!(
        visible
            .iter()
            .any(|label| label.contains("当前文件") && label.contains("10%"))
    );
    assert!(
        visible
            .iter()
            .any(|label| label.contains("总体进度") && label.contains("41%"))
    );
}

#[gpui_kit::test]
fn extract_unknown_member_progress_is_not_derived_from_overall_bytes(cx: &mut TestAppContext) {
    let (view, mut surface) = mount(cx);
    let mut snapshot = running(1000, 4_000_000);
    snapshot.progress.as_mut().expect("running").current_member = Some("unknown.bin".into());
    surface.update(|window, cx| view.update(cx, |view, cx| view.apply(snapshot, window, cx)));
    let visible = labels(&mut surface);
    let current = visible
        .iter()
        .find(|label| label.starts_with("当前文件 ·"))
        .expect("current state");
    assert!(
        current.contains("大小未知") && !current.contains('%'),
        "{current}"
    );
    assert!(
        visible
            .iter()
            .any(|label| label.contains("总体进度") && label.contains("40%"))
    );
}
