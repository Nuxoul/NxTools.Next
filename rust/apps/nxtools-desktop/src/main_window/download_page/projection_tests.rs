//! Deterministic audit #11 observations on the real production projection and row path.
use super::*;
use gpui_kit::{AppContext, TestAppContext, VisualTestContext, size};
use nxtools_download::{DownloadRequest, DownloadSegmentProgress, DownloadTask, RequestOrigin};
use std::cell::RefCell;
use std::ops::Range;

#[derive(Clone, Debug, Default)]
struct Work {
    progress_reads: usize,
    icon_reads: usize,
    ranges: Vec<Range<usize>>,
    rows: Vec<DownloadTaskId>,
    copied_payloads: usize,
    source_payloads: HashMap<DownloadTaskId, usize>,
    source_progress: HashMap<DownloadTaskId, usize>,
    copied_progress: usize,
}
thread_local! {
    static WORK: RefCell<Option<Work>> = const { RefCell::new(None) };
}
pub(crate) fn observe_progress_read() {
    WORK.with(|work| {
        if let Some(work) = work.borrow_mut().as_mut() {
            work.progress_reads += 1;
        }
    });
}
pub(crate) fn observe_icon_read() {
    WORK.with(|work| {
        if let Some(work) = work.borrow_mut().as_mut() {
            work.icon_reads += 1;
        }
    });
}
pub(super) fn observe_range(range: Range<usize>) {
    WORK.with(|work| {
        if let Some(work) = work.borrow_mut().as_mut() {
            work.ranges.push(range);
        }
    });
}
pub(super) fn observe_row(task: &DownloadEngineTask, progress: Option<&DownloadProgress>) {
    WORK.with(|work| {
        if let Some(work) = work.borrow_mut().as_mut() {
            work.rows.push(task.task.id());
            if let Some(progress) = progress
                && work
                    .source_progress
                    .get(&task.task.id())
                    .is_some_and(|source| *source != progress.segments.as_ptr() as usize)
            {
                work.copied_progress += 1;
            }
            if work
                .source_payloads
                .get(&task.task.id())
                .is_some_and(|source| *source != task.task.request().source_url.as_ptr() as usize)
            {
                work.copied_payloads += 1;
            }
        }
    });
}
struct Observation;
impl Observation {
    fn start(tasks: &[DownloadEngineTask]) -> Self {
        WORK.with(|work| {
            *work.borrow_mut() = Some(Work {
                source_payloads: tasks
                    .iter()
                    .map(|task| {
                        (
                            task.task.id(),
                            task.task.request().source_url.as_ptr() as usize,
                        )
                    })
                    .collect(),
                ..Default::default()
            })
        });
        Self
    }
    fn take(&self) -> Work {
        WORK.with(|work| {
            let mut work = work.borrow_mut();
            let observed = work.as_mut().expect("armed observation");
            let source_payloads = std::mem::take(&mut observed.source_payloads);
            let source_progress = std::mem::take(&mut observed.source_progress);
            let result = std::mem::take(observed);
            observed.source_payloads = source_payloads;
            observed.source_progress = source_progress;
            result
        })
    }
}
impl Drop for Observation {
    fn drop(&mut self) {
        WORK.with(|work| *work.borrow_mut() = None);
    }
}

fn synthetic_tasks(count: usize) -> Vec<DownloadEngineTask> {
    (0..count)
        .map(|index| DownloadEngineTask {
            task: DownloadTask::new(
                DownloadRequest {
                    source_url: format!(
                        "https://synthetic.invalid/{index}/{}",
                        "payload".repeat(256)
                    ),
                    save_directory: ".".into(),
                    save_directory_explicit: true,
                    file_name: Some(format!("fixture-{index:05}.bin")),
                    headers: Vec::new(),
                    cookie: Some("synthetic-only".repeat(128)),
                    referrer: None,
                    source_page: None,
                    certificate_policy: Default::default(),
                    connection_limit: Some(1),
                    speed_limit_bytes_per_second: None,
                    expected_total_bytes: Some(1024),
                    scheduled_at_unix_ms: None,
                    adaptive_segment_policy: None,
                    media: None,
                    category_id: Some("synthetic-category".into()),
                    category_name: Some("合成分类".repeat(4)),
                    duplicate_strategy: nxtools_download::DownloadDuplicateStrategy::UseDefault,
                },
                RequestOrigin::Internal,
                index as i64,
            ),
            probe: None,
            downloaded_bytes: index as u64,
            total_bytes: Some(1024),
            error: None,
            final_path: None,
            temporary_path: None,
            queue_eligible: false,
            has_final_artifact: false,
            scheduled: false,
        })
        .collect()
}

#[test]
fn audit11_projection_preserves_payload_identity_for_large_fixtures() {
    for count in [10000, 1000] {
        let tasks = synthetic_tasks(count);
        let started = std::time::Instant::now();
        let projected = download_tasks_for_view(
            &tasks,
            DownloadTaskFilter::All,
            "",
            DownloadSortState::default(),
        );
        let copied = projected
            .iter()
            .zip(&tasks)
            .filter(|(index, source)| {
                tasks[**index].task.request().source_url.as_ptr()
                    != source.task.request().source_url.as_ptr()
            })
            .count();
        println!(
            "audit11 projection count={count} copied_task_payloads={copied} elapsed_us={}",
            started.elapsed().as_micros()
        );
        assert_eq!(projected.len(), count);
        assert_eq!(
            copied, 0,
            "the real projection must not deep-copy task payloads"
        );
    }
}

fn workspace(
    cx: &mut TestAppContext,
    tasks: Vec<DownloadEngineTask>,
) -> (
    Entity<MainWindow>,
    gpui_kit::AnyWindowHandle,
    Entity<DownloadApplication>,
    tempfile::TempDir,
) {
    workspace_with_engine(
        cx,
        tasks,
        nxtools_download::DownloadEngine::open_in_memory().expect("empty engine, no runner"),
    )
}

fn workspace_with_engine(
    cx: &mut TestAppContext,
    tasks: Vec<DownloadEngineTask>,
    engine: nxtools_download::DownloadEngine,
) -> (
    Entity<MainWindow>,
    gpui_kit::AnyWindowHandle,
    Entity<DownloadApplication>,
    tempfile::TempDir,
) {
    cx.update(gpui_kit::init);
    cx.update(crate::theme::apply_component_theme);
    let temp = tempfile::tempdir().expect("isolated synthetic workspace");
    let paths = temp.path().to_owned();
    let (platform, _) = nxtools_ipc::platform_bridge(8);
    let notifications = cx.update(|cx| cx.new(|_| NotificationCenter::default()));
    let downloads = cx.update(|cx| {
        cx.new(|cx| {
            let mut application = DownloadApplication::new(
                engine,
                Rc::new(platform),
                notifications.clone(),
                crate::downloads::DownloadRuntimeContext {
                    fallback_default_directory: paths.clone(),
                    config: nxtools_core::AppConfig::default(),
                    config_path: paths.join("config.json"),
                    log_directory: paths.clone(),
                    database_path: paths.join("unused.sqlite3"),
                    ffmpeg_install_directory: paths.join("ffmpeg"),
                },
                cx,
            );
            application.replace_tasks_for_test(tasks);
            application
        })
    });
    let owner = downloads.downgrade();
    let retained = Rc::new(RefCell::new(None));
    let output = retained.clone();
    let (_, surface) = cx.add_window_view(move |window, cx| {
        let main = cx.new(|cx| {
            let mut main = MainWindow::new(
                gpui_kit::WeakEntity::new_invalid(),
                gpui_kit::WeakEntity::new_invalid(),
                owner,
                MainWindowShellContext {
                    settings: None,
                    platform: None,
                    system_settings: Rc::new(RefCell::new(SystemSettings::default())),
                },
                notifications,
                window,
                cx,
            );
            main.shell.activate(MainModule::Downloads);
            main
        });
        *output.borrow_mut() = Some(main.clone());
        gpui_kit::base::Root::new(main, window, cx)
    });
    let handle = surface.update(|window, _| window.window_handle());
    let main = retained.borrow_mut().take().expect("original MainWindow");
    (main, handle, downloads, temp)
}

fn assert_bounded(work: &Work, count: usize) {
    let requested = work.ranges.iter().map(Range::len).sum::<usize>();
    println!(
        "audit11 render count={count} ranges={:?} requested={requested} rows={} copied_row_payloads={} progress_reads={} icon_reads={}",
        work.ranges,
        work.rows.len(),
        work.copied_payloads,
        work.progress_reads,
        work.icon_reads
    );
    assert!(
        !work.rows.is_empty(),
        "actual production rows must be composed"
    );
    assert_eq!(
        work.rows.len(),
        requested,
        "exactly requested rows, including GPUI measurement"
    );
    assert!(
        work.rows.len() < 64,
        "bounded visible work, not all {count} tasks"
    );
    assert_eq!(
        work.copied_payloads, 0,
        "row borrows the authoritative immutable snapshot"
    );
    assert_eq!(
        work.copied_progress, 0,
        "visible row borrows original segment payload"
    );
    assert_eq!(
        work.progress_reads, requested,
        "no eager whole-list progress lookup/copy"
    );
    assert_eq!(
        work.icon_reads, requested,
        "no eager whole-list icon lookup/copy"
    );
}

#[gpui_kit::test]
fn audit11_real_render_bounds_payload_and_resource_work(cx: &mut TestAppContext) {
    for count in [10000, 1000] {
        let tasks = synthetic_tasks(count);
        let last_id = tasks.last().expect("last task").task.id();
        let (main, handle, downloads, temp) = workspace(cx, tasks);
        let mut surface = VisualTestContext::from_window(handle, cx);
        let icon = temp.path().join("synthetic.png");
        image::RgbaImage::from_pixel(20, 20, image::Rgba([12, 34, 56, 255]))
            .save(&icon)
            .expect("synthetic icon only");
        downloads.update(&mut surface, |app, cx| {
            let progress = app
                .tasks()
                .iter()
                .map(|task| DownloadProgress {
                    task_id: task.task.id(),
                    downloaded_bytes: 42,
                    total_bytes: Some(1024),
                    bytes_per_second: 7,
                    active_connections: 1,
                    segments: (0..128)
                        .map(|segment_index| DownloadSegmentProgress {
                            start_byte: None,
                            segment_index,
                            downloaded_bytes: 1,
                            total_bytes: Some(8),
                        })
                        .collect(),
                    finishing: None,
                })
                .collect();
            app.replace_progress_for_test(progress);
            let task_ids = app
                .tasks()
                .iter()
                .map(|task| task.task.id())
                .collect::<Vec<_>>();
            for id in task_ids {
                app.replace_task_icon_for_test(id, icon.clone(), cx);
            }
        });
        let observation = surface.read(|cx| Observation::start(downloads.read(cx).tasks()));
        surface.read(|cx| {
            let app = downloads.read(cx);
            let progress = app
                .tasks()
                .iter()
                .map(|task| {
                    (
                        task.task.id(),
                        app.progress(task.task.id())
                            .expect("synthetic progress")
                            .segments
                            .as_ptr() as usize,
                    )
                })
                .collect();
            WORK.with(|work| work.borrow_mut().as_mut().expect("armed").source_progress = progress);
        });
        surface.simulate_resize(size(px(1060.0), px(650.0)));
        observation.take();
        main.update(&mut surface, |_, cx| cx.notify());
        surface.update(|window, cx| window.draw(cx).clear(cx));
        let first = observation.take();
        // Print all measured dimensions even when the original implementation fails.
        println!("audit11 before-bound-assert count={count} work={first:?}");
        assert_bounded(&first, count);
        main.update(&mut surface, |main, cx| {
            main.download_list_scroll.scroll_to_bottom();
            cx.notify();
        });
        surface.update(|window, cx| window.draw(cx).clear(cx));
        let end = observation.take();
        assert_bounded(&end, count);
        assert!(
            end.rows.contains(&last_id),
            "actual end range uses stable last ID"
        );
        let selector: &'static str =
            Box::leak(format!("download-task-row-{}", last_id.as_uuid()).into_boxed_str());
        assert_eq!(
            surface
                .debug_bounds(selector)
                .expect("painted last row")
                .size
                .height,
            px(48.0)
        );
        surface.update(|window, _cx| window.remove_window());
    }
}

#[test]
fn audit11_projection_sort_filter_and_stable_ties_keep_original_results() {
    let names = ["Beta", "alpha", "ALPHA", "zeta", "gamma"];
    let statuses = ["waiting", "completed", "paused", "failed", "downloading"];
    let sizes = [Some(100), None, Some(50), Some(100), Some(0)];
    let bytes = [50, 0, 25, 100, 9];
    let tasks = synthetic_tasks(5)
        .into_iter()
        .enumerate()
        .map(|(index, task)| {
            let mut wire = serde_json::to_value(task).expect("synthetic wire");
            wire["task"]["request"]["file_name"] = serde_json::json!(names[index]);
            wire["task"]["status"] = serde_json::json!(statuses[index]);
            if statuses[index] == "completed" {
                wire["task"]["updated_at_unix_ms"] = serde_json::json!(100);
                wire["task"]["completed_at_unix_ms"] = serde_json::json!(100);
            }
            wire["total_bytes"] = serde_json::json!(sizes[index]);
            wire["downloaded_bytes"] = serde_json::json!(bytes[index]);
            serde_json::from_value::<DownloadEngineTask>(wire).expect("synthetic status projection")
        })
        .collect::<Vec<_>>();
    for (key, ascending, descending) in [
        (
            DownloadSortKey::Queue,
            vec![0, 1, 2, 3, 4],
            vec![0, 1, 2, 3, 4],
        ),
        (
            DownloadSortKey::Name,
            vec![1, 2, 0, 4, 3],
            vec![3, 4, 0, 1, 2],
        ),
        (
            DownloadSortKey::Size,
            vec![1, 4, 2, 0, 3],
            vec![0, 3, 2, 4, 1],
        ),
        (
            DownloadSortKey::Progress,
            vec![1, 4, 0, 2, 3],
            vec![3, 0, 2, 1, 4],
        ),
        (
            DownloadSortKey::Status,
            vec![4, 0, 2, 3, 1],
            vec![1, 3, 2, 0, 4],
        ),
        (
            DownloadSortKey::Created,
            vec![0, 1, 2, 3, 4],
            vec![4, 3, 2, 1, 0],
        ),
    ] {
        for (descending, expected) in [(false, ascending), (true, descending)] {
            assert_eq!(
                download_tasks_for_view(
                    &tasks,
                    DownloadTaskFilter::All,
                    "",
                    DownloadSortState { key, descending }
                ),
                expected,
                "stable presentation {key:?}/{descending}"
            );
        }
    }
    assert_eq!(
        download_tasks_for_view(
            &tasks,
            DownloadTaskFilter::All,
            "ALPHA",
            DownloadSortState::default()
        ),
        vec![1, 2]
    );
    for (filter, expected) in [
        (DownloadTaskFilter::Downloading, vec![4]),
        (DownloadTaskFilter::Waiting, vec![0]),
        (DownloadTaskFilter::Paused, vec![2]),
        (DownloadTaskFilter::Completed, vec![1]),
        (DownloadTaskFilter::Failed, vec![3]),
    ] {
        assert_eq!(
            download_tasks_for_view(&tasks, filter, "", DownloadSortState::default()),
            expected
        );
    }
    let counts = download_filter_counts(tasks.iter().map(|task| task.task.status()));
    assert_eq!(counts.all, 5);
    for filter in DownloadTaskFilter::ALL {
        assert_eq!(
            counts.get(filter),
            if filter == DownloadTaskFilter::All {
                5
            } else {
                1
            }
        );
    }
}

type AfterProjection = Box<dyn FnOnce(&mut App)>;

thread_local! {
    static AFTER_PROJECTION: RefCell<Option<AfterProjection>> = const { RefCell::new(None) };
}

pub(super) fn after_projection(cx: &mut App) {
    let hook = AFTER_PROJECTION.with(|hook| hook.borrow_mut().take());
    if let Some(hook) = hook {
        hook(cx);
    }
}

#[gpui_kit::test]
fn audit11_deferred_range_keeps_its_snapshot_when_owner_is_replaced(cx: &mut TestAppContext) {
    let tasks = synthetic_tasks(1000);
    let ids = tasks
        .iter()
        .map(|task| task.task.id())
        .collect::<HashSet<_>>();
    let (main, handle, downloads, _temp) = workspace(cx, tasks);
    let retained_snapshot = cx.read(|cx| downloads.read(cx).task_snapshot());
    let replacement = synthetic_tasks(1000);
    let new_ids = replacement
        .iter()
        .map(|task| task.task.id())
        .collect::<HashSet<_>>();
    let mut surface = VisualTestContext::from_window(handle, cx);
    surface.simulate_resize(size(px(920.0), px(480.0)));
    let application = downloads.clone();
    let replaced = Rc::new(std::cell::Cell::new(false));
    let fired = replaced.clone();
    AFTER_PROJECTION.with(|hook| {
        *hook.borrow_mut() = Some(Box::new(move |cx| {
            application.update(cx, |app, _| app.replace_tasks_for_test(replacement));
            fired.set(true);
        }))
    });
    let observation = Observation::start(&retained_snapshot);
    // Inspect this exact draw before another frame can project the replacement.
    let work = surface.update(|window, cx| {
        main.update(cx, |_, cx| cx.notify());
        window.refresh();
        window.draw(cx).clear(cx);
        observation.take()
    });
    assert!(
        replaced.get(),
        "owner replacement ran after production projection"
    );
    assert!(!work.rows.is_empty(), "real production deferred range ran");
    assert!(
        work.rows.iter().all(|id| ids.contains(id)),
        "old index cannot retarget to replacement tasks"
    );
    assert_eq!(
        work.copied_payloads, 0,
        "the captured old snapshot stayed owned"
    );
    assert!(surface.read(|cx| {
        downloads
            .read(cx)
            .tasks()
            .iter()
            .all(|task| new_ids.contains(&task.task.id()))
    }));
    assert!(
        retained_snapshot
            .iter()
            .all(|task| ids.contains(&task.task.id()))
    );
    surface.update(|window, _| window.remove_window());
}

#[gpui_kit::test]
fn audit11_pointer_selection_survives_reorder_and_retires_removed_id(cx: &mut TestAppContext) {
    let queued_directory = tempfile::tempdir().expect("queue-only temporary directory");
    let engine = nxtools_download::DownloadEngine::open_in_memory().expect("queue-only engine");
    for task in synthetic_tasks(4) {
        let mut request = task.task.request().clone();
        request.save_directory = queued_directory.path().to_path_buf();
        request.category_id = None;
        request.category_name = None;
        engine
            .enqueue(request, RequestOrigin::Internal)
            .expect("queue only, never start runner");
    }
    let tasks = engine.tasks().expect("actual queued snapshots");
    let selected_id = tasks[2].task.id();
    let other_id = tasks[1].task.id();
    let (main, handle, downloads, _temp) = workspace_with_engine(cx, tasks, engine);
    let mut surface = VisualTestContext::from_window(handle, cx);
    surface.simulate_resize(size(px(920.0), px(480.0)));
    let selected_selector: &'static str =
        Box::leak(format!("download-task-row-{}", selected_id.as_uuid()).into_boxed_str());
    let other_selector: &'static str =
        Box::leak(format!("download-task-row-{}", other_id.as_uuid()).into_boxed_str());
    let selected_bounds = surface
        .debug_bounds(selected_selector)
        .expect("real selected row");
    surface.simulate_click(selected_bounds.center(), gpui_kit::Modifiers::default());
    surface.run_until_parked();
    assert_eq!(
        main.read_with(&surface, |main, _| main.selected_download_task),
        Some(selected_id)
    );
    let retained = surface.read(|cx| downloads.read(cx).task_snapshot());
    let mut replacement = retained.as_ref().clone();
    replacement.swap(1, 2);
    downloads.update(&mut surface, |app, cx| {
        app.replace_tasks_for_test(replacement);
        cx.notify();
    });
    main.update(&mut surface, |_, cx| cx.notify());
    surface.refresh().expect("reordered production page");
    assert_eq!(
        main.read_with(&surface, |main, _| main.selected_download_task),
        Some(selected_id)
    );
    assert!(
        surface
            .debug_bounds(selected_selector)
            .expect("same ID after reorder")
            .top()
            < surface
                .debug_bounds(other_selector)
                .expect("other ID after reorder")
                .top()
    );
    let other_bounds = surface
        .debug_bounds(other_selector)
        .expect("fresh actual target");
    surface.simulate_click(
        other_bounds.center(),
        gpui_kit::Modifiers {
            control: true,
            ..Default::default()
        },
    );
    surface.run_until_parked();
    assert_eq!(
        main.read_with(&surface, |main, _| main.selected_download_tasks.clone()),
        HashSet::from([selected_id, other_id])
    );
    let remaining = surface.read(|cx| {
        downloads
            .read(cx)
            .tasks()
            .iter()
            .filter(|task| task.task.id() != selected_id && task.task.id() != other_id)
            .cloned()
            .collect()
    });
    downloads.update(&mut surface, |app, cx| {
        app.replace_tasks_for_test(remaining);
        cx.notify();
    });
    main.update(&mut surface, |_, cx| cx.notify());
    surface
        .refresh()
        .expect("removed IDs reconcile at real render boundary");
    assert!(main.read_with(&surface, |main, _| main.selected_download_tasks.is_empty()));
    assert_eq!(
        main.read_with(&surface, |main, _| (
            main.selected_download_task,
            main.requested_download_detail
        )),
        (None, None)
    );
    assert_eq!(
        retained[2].task.id(),
        selected_id,
        "retained immutable snapshot has not been mutated"
    );
    surface.update(|window, _| window.remove_window());
}
